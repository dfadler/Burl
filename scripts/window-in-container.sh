#!/usr/bin/env bash
# Runs inside docker/window.Dockerfile: start Xvfb and a window manager, open the real `burl`
# window on a fixture, drive it with xdotool and check what the interactive checklist asks of the
# Linux X11 window path. The session is recorded to /out/session.mp4 and every check writes a
# PNG to /out. Exit 1 if any check fails; all checks always run.
# `test && ok ... || bad ...` below: `ok` only echoes and cannot fail, so `bad` never runs after a pass.
# shellcheck disable=SC2015
set -uo pipefail

OUT=/out
LOG=/repo/tests/golden/sim/small-repo.log
SCREEN=1280x720
export DISPLAY=:99
mkdir -p "$OUT"
fail=0
ok() { echo "ok   $1"; }
bad() {
	echo "FAIL $1"
	fail=1
}

Xvfb "$DISPLAY" -screen 0 "${SCREEN}x24" -nolisten tcp &
for _ in $(seq 50); do xdpyinfo >/dev/null 2>&1 && break || sleep 0.1; done
openbox >/dev/null 2>&1 &
# A window mapped before the WM is ready is never managed: wait until it has announced itself.
for _ in $(seq 100); do xprop -root _NET_SUPPORTING_WM_CHECK 2>/dev/null | grep -q 'window id' && break || sleep 0.1; done
ffmpeg -loglevel error -y -f x11grab -draw_mouse 0 -framerate 10 -video_size "$SCREEN" \
	-i "$DISPLAY" "$OUT/session.mp4" &
REC=$!
# Stop the recorder so ffmpeg writes the MP4 trailer. As an EXIT trap this also covers the early
# `exit 1`s: this script is PID 1, so exiting would otherwise kill ffmpeg mid-file.
finish() {
	kill -INT "$REC" 2>/dev/null
	wait "$REC" 2>/dev/null
}
trap finish EXIT

shot() { ffmpeg -loglevel error -y -f x11grab -draw_mouse 0 -video_size "$SCREEN" -i "$DISPLAY" -frames:v 1 "$OUT/$1.png"; }
luma() { # IMAGE CROP -> mean luma 0-255
	ffmpeg -loglevel error -i "$OUT/$1.png" -vf "crop=$2,signalstats,metadata=print:key=lavfi.signalstats.YAVG:file=-" -f null - |
		sed -n 's/.*YAVG=//p' | head -1
}

# Orientation, in a launch of its own with only the date caption drawn: the caption sits near the
# top of the window, so a strip there must be brighter than the mirror strip near the bottom. A
# vertically flipped window (the windowed blit once mirrored it) swaps them.
burl "$LOG" --log-format custom -640x360 --seconds-per-day 1 --hash-seed 31 --prng glibc \
	--hide tree,files,users,bloom,usernames,filenames,dirnames,progress,mouse &
OBURL=$!
OWID=
for _ in $(seq 200); do
	OWID=$(xdotool search --onlyvisible --pid "$OBURL" 2>/dev/null | head -1)
	[ -n "$OWID" ] && break
	sleep 0.1
done
if [ -n "$OWID" ]; then
	sleep 2
	shot orientation
	# the client area's own screen position (xdotool reports the frame's)
	X=$(xwininfo -id "$OWID" | awk '/Absolute upper-left X/ {print $4}')
	Y=$(xwininfo -id "$OWID" | awk '/Absolute upper-left Y/ {print $4}')
	HEIGHT=$(xwininfo -id "$OWID" | awk '/Height:/ {print $2}')
	top=$(luma orientation "260:24:$((X + 190)):$((Y + 16))")
	bottom=$(luma orientation "260:24:$((X + 190)):$((Y + HEIGHT - 40))")
	if awk -v t="$top" -v b="$bottom" 'BEGIN { exit !(t > b + 2) }'; then
		ok "the scene is upright (date strip luma $top vs mirror strip $bottom)"
	else
		bad "the scene looks vertically flipped (top strip luma $top, bottom strip $bottom)"
	fi
else
	bad "orientation launch: window never appeared"
fi
kill "$OBURL" 2>/dev/null
wait "$OBURL" 2>/dev/null

# Same flags as the visual gate: fixed seeds, no auto-rotate, glibc PRNG.
# 10 s per day: the fixture spans a few days, so the clock is still running for the first ~30 s.
burl "$LOG" --log-format custom -640x360 --seconds-per-day 10 --hash-seed 31 \
	--disable-auto-skip --disable-auto-rotate --file-idle-time 0 --time-scale 1 \
	--hide mouse --prng glibc &
BURL=$!
# Poll for the window (any map state) instead of `xdotool search --sync`, which never returns
# if it does not appear; give up after 20 s and say what X has.
WID=
for _ in $(seq 200); do
	WID=$(xdotool search --pid "$BURL" 2>/dev/null | head -1)
	[ -n "$WID" ] && break
	sleep 0.1
done
if [ -z "$WID" ]; then
	bad "window did not open within 20 s"
	xwininfo -root -tree
	exit 1
fi
# `map-state` is its own check: a created-but-never-shown window is a bug a user would see.
for _ in $(seq 100); do
	xwininfo -id "$WID" | grep -q 'Map State: IsViewable' && break
	sleep 0.1
done
if xwininfo -id "$WID" | grep -q 'Map State: IsViewable'; then
	ok "window is shown (mapped) without help"
else
	bad "window was created but never mapped; mapping it by hand to run the other checks"
	xdotool windowmap --sync "$WID"
fi

key() { xdotool windowfocus --sync "$WID" key "$@"; }
size() { xdotool getwindowgeometry --shell "$WID" | awk -F= '/^(WIDTH|HEIGHT)=/ {printf "%s%s", sep, $2; sep="x"}'; }

[ "$(size)" = 640x360 ] && ok "window opens at the requested pixel size (G3)" || bad "window size $(size), want 640x360"

# Space (pause) and T (tree) are not judged here: the camera and tree keep moving while paused
# by design, so pixels cannot tell a paused scene from a running one. They are unit-tested
# (`key_action` in input.rs, `paused_freezes_the_clock_but_not_the_camera` in the sim).
sleep 4 # let the scene fill in
shot before-resize

xdotool windowsize --sync "$WID" 800 450
sleep 1
shot resized
[ "$(size)" = 800x450 ] && ok "resize reaches the window" || bad "size after resize $(size), want 800x450"
# The far corner of the 800x450 client area only exists if the window redrew at the new size: it
# must show the scene background (about 28 luma), not the black desktop (0) left by a stale frame.
rx=$(xwininfo -id "$WID" | awk '/Absolute upper-left X/ {print $4}')
ry=$(xwininfo -id "$WID" | awk '/Absolute upper-left Y/ {print $4}')
corner=$(luma resized "60:40:$((rx + 800 - 70)):$((ry + 450 - 50))")
if awk -v c="$corner" 'BEGIN { exit !(c > 15) }'; then
	ok "resized scene is redrawn to the new size (corner luma $corner)"
else
	bad "the resized window's far corner is not drawn (luma $corner)"
fi

key alt+Return
sleep 1
shot fullscreen
[ "$(size)" = 1280x720 ] && ok "Alt+Return goes fullscreen" || bad "fullscreen size $(size), want 1280x720"
key alt+Return
sleep 1
[ "$(size)" != 1280x720 ] && ok "Alt+Return leaves fullscreen" || bad "still fullscreen"

key Escape
for _ in $(seq 50); do kill -0 "$BURL" 2>/dev/null || break; sleep 0.1; done
if kill -0 "$BURL" 2>/dev/null; then
	bad "Esc did not quit"
	kill "$BURL"
else
	wait "$BURL" && ok "Esc quits cleanly" || bad "Esc quit with a non-zero status"
fi

finish
# ffmpeg exits non-zero after SIGINT, so judge the file, not the wait status.
ffprobe -v error "$OUT/session.mp4" >/dev/null 2>&1 && ok "session.mp4 is a valid recording" || bad "session.mp4 is missing or invalid"
exit $fail
