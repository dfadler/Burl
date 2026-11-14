#!/usr/bin/env bash
# X11 cursor grab with a REAL evdev device (#212): a kernel uinput mouse read by Xorg's libinput
# driver on the dummy video driver, instead of xdotool/XTest under Xvfb. Runs gnarl, presses the right
# button on the device, pushes relative motion past every window edge and checks with the server's
# pointer position (XQueryPointer, via xdotool getmouselocation) that the pointer stays inside; then
# releases and checks it is free. The cursor image (XFixesGetCursorImage) must be hidden while held.
#   scripts/window-uinput.sh [OUT_DIR]     needs sudo, /dev/uinput, Xorg + xserver-xorg-video-dummy +
#   xserver-xorg-input-libinput, openbox, xdotool, x11-utils, libx11-dev, libxfixes-dev, a C compiler
#   and a built target/release/gnarl (GNARL=path to override). Linux only; see window-uinput.yml.
# Exit 1 if any check fails; all checks always run.
# `test && ok ... || bad ...` below: `ok` only echoes and cannot fail, so `bad` never runs after a pass.
# SC2329: cleanup is called by the EXIT trap.
# shellcheck disable=SC2015,SC2329
set -uo pipefail

case "${1:-}" in -h | --help)
	sed -n '2,10p' "$0" | sed 's/^# \{0,1\}//'
	exit 0
	;;
esac

ROOT=$(cd "$(dirname "$0")/.." && pwd)
OUT=${1:-"$ROOT/target/window-uinput"}
mkdir -p "$OUT/bin"
OUT=$(cd "$OUT" && pwd)
GNARL=${GNARL:-"$ROOT/target/release/gnarl"}
LOG=$ROOT/tests/golden/sim/small-repo.log
export DISPLAY=:98 LIBGL_ALWAYS_SOFTWARE=1 LP_NUM_THREADS=1
fail=0
ok() { echo "ok   $1"; }
bad() {
	echo "FAIL $1"
	fail=1
}

cc -O1 -o "$OUT/bin/cursor-alpha" "$ROOT/scripts/cursor-alpha.c" -lX11 -lXfixes || exit 1
cc -O1 -o "$OUT/bin/uinput-mouse" "$ROOT/scripts/uinput-mouse.c" || exit 1

cat >"$OUT/xorg.conf" <<'EOF'
Section "Device"
  Identifier "dummy0"
  Driver "dummy"
  VideoRam 256000
EndSection
Section "Monitor"
  Identifier "monitor0"
  HorizSync 5.0-1000.0
  VertRefresh 5.0-200.0
EndSection
Section "Screen"
  Identifier "screen0"
  Device "dummy0"
  Monitor "monitor0"
  DefaultDepth 24
  SubSection "Display"
    Depth 24
    Modes "1280x720"
  EndSubSection
EndSection
# Flat acceleration: a relative count moves the pointer about one pixel.
Section "InputClass"
  Identifier "gnarl uinput mouse"
  MatchProduct "gnarl-uinput-mouse"
  MatchIsPointer "on"
  MatchDevicePath "/dev/input/event*"
  Driver "libinput"
  Option "AccelProfile" "flat"
EndSection
EOF

# Xorg first, then the device: the mouse is hot-plugged through udev like a USB mouse would be.
sudo Xorg "$DISPLAY" -config "$OUT/xorg.conf" -logfile "$OUT/Xorg.log" -ac -noreset -nolisten tcp &
XORG=$!
cleanup() {
	exec 3>&- 2>/dev/null
	kill "$GNARLPID" 2>/dev/null
	kill "$OPENBOX" 2>/dev/null
	sudo kill "$XORG" 2>/dev/null
	sudo chmod a+r "$OUT/Xorg.log" 2>/dev/null
}
GNARLPID='' OPENBOX=''
trap cleanup EXIT
for _ in $(seq 100); do xdpyinfo >/dev/null 2>&1 && break || sleep 0.1; done
xdpyinfo >/dev/null 2>&1 || {
	echo "FAIL Xorg did not start"
	sudo cat "$OUT/Xorg.log"
	exit 1
}

mkfifo "$OUT/ctl"
"$OUT/bin/uinput-mouse" <"$OUT/ctl" >"$OUT/uinput.out" 2>"$OUT/uinput.err" &
exec 3>"$OUT/ctl"
for _ in $(seq 50); do grep -q '^ready' "$OUT/uinput.out" && break || sleep 0.1; done
if ! grep -q '^ready' "$OUT/uinput.out"; then
	echo "FAIL the uinput device was not created: $(cat "$OUT/uinput.err")"
	exit 1
fi
# Xorg must have taken it up as an evdev device through libinput: that is what makes it a real device.
for _ in $(seq 50); do sudo grep -q "Using input driver 'libinput' for 'gnarl-uinput-mouse'" "$OUT/Xorg.log" && break || sleep 0.1; done
sudo grep -E "gnarl-uinput-mouse" "$OUT/Xorg.log"
sudo grep -q "Using input driver 'libinput' for 'gnarl-uinput-mouse'" "$OUT/Xorg.log" &&
	ok "the uinput mouse is an evdev device Xorg reads through libinput" ||
	bad "Xorg did not attach the uinput mouse through libinput"

openbox >"$OUT/openbox.log" 2>&1 &
OPENBOX=$!
for _ in $(seq 100); do xprop -root _NET_CLIENT_LIST 2>/dev/null | grep -q 'window id' && break || sleep 0.1; done

"$GNARL" "$LOG" --log-format custom -640x360 --seconds-per-day 10 --hash-seed 31 --prng glibc \
	--disable-auto-skip --disable-auto-rotate --file-idle-time 0 >"$OUT/gnarl.log" 2>&1 &
GNARLPID=$!
CWID=
for _ in $(seq 200); do
	CWID=$(xdotool search --onlyvisible --pid "$GNARLPID" 2>/dev/null | head -1)
	[ -n "$CWID" ] && break
	sleep 0.1
done
if [ -z "$CWID" ]; then
	bad "uinput launch: window never appeared"
	cat "$OUT/gnarl.log"
	xwininfo -root -tree
	exit 1
fi
sleep 2
CX=$(xwininfo -id "$CWID" | awk '/Absolute upper-left X/ {print $4}')
CY=$(xwininfo -id "$CWID" | awk '/Absolute upper-left Y/ {print $4}')
CW=$(xwininfo -id "$CWID" | awk '/Width:/ {print $2}')
CH=$(xwininfo -id "$CWID" | awk '/Height:/ {print $2}')
echo "window $CWID at $CX,$CY ${CW}x${CH}"

dev() { echo "$@" >&3; }
# sets PX, PY from the server (XQueryPointer)
where() {
	local l
	l=$(xdotool getmouselocation --shell)
	PX=$(echo "$l" | awk -F= '/^X=/ {print $2}')
	PY=$(echo "$l" | awk -F= '/^Y=/ {print $2}')
}
pointer_inside() {
	where
	[ "$PX" -ge "$CX" ] && [ "$PX" -lt $((CX + CW)) ] && [ "$PY" -ge "$CY" ] && [ "$PY" -lt $((CY + CH)) ]
}
# push DX DY: 60 relative events of (DX,DY) from the device, 5 ms apart
push() {
	for _ in $(seq 60); do
		dev rel "$1" "$2"
		sleep 0.005
	done
	sleep 0.2
}
# goto X Y: closed loop with relative motion only (the pointer starts wherever the server left it)
goto() {
	local dx dy
	for _ in $(seq 80); do
		where
		dx=$(($1 - PX))
		dy=$(($2 - PY))
		[ "${dx#-}" -le 2 ] && [ "${dy#-}" -le 2 ] && return 0
		[ "$dx" -gt 40 ] && dx=40
		[ "$dx" -lt -40 ] && dx=-40
		[ "$dy" -gt 40 ] && dy=40
		[ "$dy" -lt -40 ] && dy=-40
		dev rel "$dx" "$dy"
		sleep 0.02
	done
	return 1
}
# cursor_is visible|hidden: polled for 5 s (the grab and the hide land a frame after the press)
cursor_is() {
	local a=
	for _ in $(seq 50); do
		a=$("$OUT/bin/cursor-alpha" 2>&1)
		if [ "$1" = hidden ]; then
			[ "$a" = 0 ] && return 0
		else
			case "$a" in '' | *[!0-9]* | 0) ;; *) return 0 ;; esac
		fi
		sleep 0.1
	done
	CURSOR_ALPHA=$a
	return 1
}

goto $((CX + CW / 2)) $((CY + CH / 2)) && ok "the device moves the server's pointer to the window centre" ||
	bad "the device could not move the pointer to the window centre (at $PX,$PY)"
push 60 60
pointer_inside && bad "control: the pointer cannot leave the window without a grab (at $PX,$PY)" ||
	ok "control: an ungrabbed pointer can leave the window"
goto $((CX + CW / 2)) $((CY + CH / 2))
cursor_is visible && ok "cursor is visible over the window before a button is pressed" ||
	bad "cursor is not visible over the window before a press (max alpha: ${CURSOR_ALPHA:-?})"

dev down
cursor_is hidden && ok "cursor is hidden while the real button is held" ||
	bad "cursor is not hidden while the right button is held (max alpha: ${CURSOR_ALPHA:-?})"
# every edge and corner; the pointer is recentred by the device between pushes
for d in "60 0 right" "-60 0 left" "0 60 bottom" "0 -60 top" "60 60 bottom-right" "-60 -60 top-left"; do
	read -r -a a <<<"$d"
	set -- "${a[@]}"
	push "$1" "$2"
	pointer_inside && ok "real motion past the $3 edge stays inside the window while the button is held (at $PX,$PY)" ||
		bad "pointer left the window past the $3 edge while the right button is held (at $PX,$PY)"
	goto $((CX + CW / 2)) $((CY + CH / 2))
done
dev up
cursor_is visible && ok "cursor is visible again after the real button is released" ||
	bad "cursor is still hidden after the button went up (max alpha: ${CURSOR_ALPHA:-?})"
push 60 60
pointer_inside && bad "pointer is still confined after the button went up (at $PX,$PY)" ||
	ok "releasing the real button releases the confinement"

echo "--- gnarl.log"
cat "$OUT/gnarl.log"
exit "$fail"
