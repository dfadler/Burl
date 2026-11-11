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
openbox >"$OUT/openbox.log" 2>&1 &
OPENBOX=$!
# A window mapped before the WM is ready is never managed. _NET_SUPPORTING_WM_CHECK is set early
# in openbox's startup, well before it handles map requests (about 1 s later on a starved CPU), so
# a window opened in between stays unmapped for good (#163). _NET_CLIENT_LIST is set once startup
# is done. Fail loudly if that never happens rather than carry on with an unmanaged display.
wm_ready=
for _ in $(seq 100); do
	xprop -root _NET_CLIENT_LIST 2>/dev/null | grep -q 'window id' && {
		wm_ready=1
		break
	}
	sleep 0.1
done
if [ -z "$wm_ready" ]; then
	echo "FAIL openbox not ready within 10 s (still running: $(kill -0 "$OPENBOX" 2>/dev/null && echo yes || echo no))"
	cat "$OUT/openbox.log"
	exit 1
fi
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

# diag NAME PID: say why a check could not run (is burl alive, which windows it owns and in what
# map state, what X has, what the screen shows). Saved as OUT/NAME-fail.png.
diag() {
	if kill -0 "$2" 2>/dev/null; then echo "  diag: burl is still running"; else echo "  diag: burl has exited"; fi
	local w
	for w in $(xdotool search --pid "$2" 2>/dev/null); do
		echo "  diag: window $w: $(xwininfo -id "$w" 2>/dev/null | grep 'Map State')"
	done
	# PID-independent too: a window that exists but has not set _NET_WM_PID yet is missed above.
	for w in $(xwininfo -root -children 2>/dev/null | awk '/^ +0x/ {print $1}'); do
		echo "  diag: top-level $w: $(xwininfo -id "$w" 2>/dev/null | grep 'Map State')"
	done
	xwininfo -root -tree
	shot "$1-fail"
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
	diag orientation "$OBURL"
fi
kill "$OBURL" 2>/dev/null
wait "$OBURL" 2>/dev/null

# Selection and hover (#125), read from the `pointer:` debug line (`--log-level debug`) rather than
# pixels: Tab selects a user, a background click clears it, sweeping the pointer finds a pawn whose
# hover box would show, clicking it selects it, and a selected pawn gets no hover box. Where the box
# is drawn, its text and the edge flip are not judged here; the pick itself is real X11 input.
SLOG=$OUT/selection.log
burl "$LOG" --log-format custom -640x360 --seconds-per-day 10 --hash-seed 31 --prng glibc \
	--disable-auto-skip --disable-auto-rotate --file-idle-time 0 --log-level debug >"$SLOG" 2>&1 &
SBURL=$!
SWID=
for _ in $(seq 200); do
	SWID=$(xdotool search --onlyvisible --pid "$SBURL" 2>/dev/null | head -1)
	[ -n "$SWID" ] && break
	sleep 0.1
done
lastp() { grep -o 'pointer: .*' "$SLOG" | tail -1; }
if [ -n "$SWID" ]; then
	sleep 2
	SX=$(xwininfo -id "$SWID" | awk '/Absolute upper-left X/ {print $4}')
	SY=$(xwininfo -id "$SWID" | awk '/Absolute upper-left Y/ {print $4}')
	xdotool windowfocus --sync "$SWID"
	xdotool mousemove $((SX + 3)) $((SY + 3))
	sleep 0.5
	# Tab skips users that are not fully opaque, so press it until one is selected (6 tries)
	for _ in 1 2 3 4 5 6; do
		xdotool key Tab
		sleep 0.4
		lastp | grep -q 'selected=user ' && break
	done
	lastp | grep -q 'selected=user ' && ok "selection: Tab selects a user ($(lastp))" ||
		{ bad "selection: Tab selected no user ($(lastp))"; diag selection "$SBURL"; }
	# `selectBackground` needs the button still down when the frame draws, so hold it for a few frames
	xdotool mousemove $((SX + 3)) $((SY + 3)) mousedown 1
	sleep 0.3
	xdotool mouseup 1
	sleep 0.3
	lastp | grep -q 'selected=none' && ok "selection: a background click deselects" ||
		bad "selection: a background click did not deselect ($(lastp))"
	# Sweep the client area in 12 px steps until the pointer is over a pawn (hover box content).
	found=
	for y in $(seq 20 12 340); do
		for x in $(seq 12 12 628); do
			xdotool mousemove $((SX + x)) $((SY + y))
			sleep 0.05
			if ! lastp | grep -q 'hover=none'; then
				found="$x $y"
				break 2
			fi
		done
	done
	if [ -n "$found" ]; then
		hov=$(lastp)
		ok "selection: hovering a pawn shows its box content at $found ($hov)"
		xdotool click 1
		sleep 0.4
		if lastp | grep -q 'selected=\(user\|file\) '; then
			ok "selection: a click on the hovered pawn selects it ($(lastp))"
			lastp | grep -q 'hover=none' && ok "selection: no hover box on the selected pawn" ||
				bad "selection: the selected pawn still shows a hover box ($(lastp))"
		else
			bad "selection: a click on the hovered pawn selected nothing (hovered: $hov; now: $(lastp))"
		fi
	else
		bad "selection: the pointer swept the whole window without hovering a pawn"
	fi
else
	bad "selection launch: window never appeared"
	diag selection "$SBURL"
fi
kill "$SBURL" 2>/dev/null
wait "$SBURL" 2>/dev/null

# Cursor confinement (winit X11 `Confined` = core GrabPointer with confine_to the window): holding
# the right button grabs the mouse (`Live::set_grab`); `Locked` is unsupported on X11 so Burl falls
# back to `Confined`. A pointer pushed far outside the window must stay clamped inside it while the
# button is down and go free once it is up. No `--hide mouse`: that flag ignores buttons.
burl "$LOG" --log-format custom -640x360 --seconds-per-day 10 --hash-seed 31 --prng glibc \
	--disable-auto-skip --disable-auto-rotate --file-idle-time 0 &
CBURL=$!
CWID=
for _ in $(seq 200); do
	CWID=$(xdotool search --onlyvisible --pid "$CBURL" 2>/dev/null | head -1)
	[ -n "$CWID" ] && break
	sleep 0.1
done
if [ -n "$CWID" ]; then
	sleep 2
	CX=$(xwininfo -id "$CWID" | awk '/Absolute upper-left X/ {print $4}')
	CY=$(xwininfo -id "$CWID" | awk '/Absolute upper-left Y/ {print $4}')
	CW=$(xwininfo -id "$CWID" | awk '/Width:/ {print $2}')
	CH=$(xwininfo -id "$CWID" | awk '/Height:/ {print $2}')
	# sets PX, PY; succeeds if the pointer is inside the window's client rectangle
	pointer_inside() {
		PX=$(xdotool getmouselocation --shell | awk -F= '/^X=/ {print $2}')
		PY=$(xdotool getmouselocation --shell | awk -F= '/^Y=/ {print $2}')
		[ "$PX" -ge "$CX" ] && [ "$PX" -lt $((CX + CW)) ] && [ "$PY" -ge "$CY" ] && [ "$PY" -lt $((CY + CH)) ]
	}
	# Push the pointer to the screen's far corner until it is `inside` or `outside` the window
	# (polled for 5 s: the grab is requested a frame after the button goes down).
	push_until() {
		for _ in $(seq 50); do
			xdotool mousemove 1270 710
			if pointer_inside; then
				[ "$1" = inside ] && return 0
			else
				[ "$1" = outside ] && return 0
			fi
			sleep 0.1
		done
		return 1
	}
	xdotool mousemove $((CX + CW / 2)) $((CY + CH / 2))
	push_until outside && ok "control: an ungrabbed pointer can leave the window" ||
		bad "control: the pointer cannot leave the window even without a grab"
	xdotool mousemove $((CX + CW / 2)) $((CY + CH / 2)) mousedown 3
	push_until inside && ok "holding a button confines the pointer to the window (Linux X11 Confined grab)" ||
		bad "pointer left the window while the right button is held (at $PX,$PY)"
	xdotool mouseup 3
	push_until outside && ok "releasing the button releases the confinement" ||
		bad "pointer is still confined after the button went up (at $PX,$PY)"
else
	bad "cursor-confine launch: window never appeared"
	diag cursor "$CBURL"
fi
kill "$CBURL" 2>/dev/null
wait "$CBURL" 2>/dev/null

# Pointer pixels (#180, #125): what the state-based selection block above cannot judge: selected
# sprites drawn white, the hover box (present, left and right of the pointer, flipped at the right
# edge, absent over a selected pawn or file) and its 3 s idle. Selecting and deselecting by state is
# that block's; here Tab and the clicks are setup. The sim runs in real time, so nothing is hard-coded:
# Space stops the clock, the camera then eases to rest (two shots a second apart come out identical
# once it has), and every target is found in a screenshot. Tab selects one user, and what changes
# is that pawn's place; a narrow window crops the scene so files sit at both side edges.

# blob A B THR X Y W H: "COUNT MINX MAXX MINY MAXY CX CY" (screen pixels) of the pixels in the
# rectangle whose luma differs by more than THR between shots A and B; COUNT is 0 for none.
blob() {
	ffmpeg -loglevel error -i "$OUT/$1.png" -i "$OUT/$2.png" \
		-filter_complex "[0][1]blend=all_mode=difference,format=gray,crop=$6:$7:$4:$5" -f rawvideo -pix_fmt gray - |
		od -An -v -tu1 -w"$6" |
		awk -v thr="$3" -v ox="$4" -v oy="$5" '
			{
				for (i = 1; i <= NF; i++) {
					if ($i <= thr) continue
					x = ox + i - 1; y = oy + NR - 1
					if (n == 0) { x0 = x1 = x; y0 = y1 = y }
					if (x < x0) x0 = x
					if (x > x1) x1 = x
					if (y < y0) y0 = y
					if (y > y1) y1 = y
					sx += x; sy += y; n++
				}
			}
			END { if (n) printf "%d %d %d %d %d %d %d\n", n, x0, x1, y0, y1, sx / n, sy / n; else print "0 0 0 0 0 0 0" }'
}

# nonbg IMG X Y W H: the same numbers for the pixels in the rectangle that are not the scene
# background (taken from the window's top-left corner). With users hidden these are the files.
nonbg() {
	local bg
	bg=$(ffmpeg -loglevel error -i "$OUT/$1.png" -vf "crop=1:1:$((WX + 3)):$((WY + 3))" -f rawvideo -pix_fmt rgb24 - | od -An -v -tu1)
	ffmpeg -loglevel error -i "$OUT/$1.png" -vf "crop=$4:$5:$2:$3" -f rawvideo -pix_fmt rgb24 - |
		od -An -v -tu1 -w"$(($4 * 3))" |
		awk -v bg="$bg" -v ox="$2" -v oy="$3" '
			BEGIN { split(bg, c, " ") }
			{
				for (i = 1; i + 2 <= NF; i += 3) {
					d1 = $i - c[1]; d2 = $(i + 1) - c[2]; d3 = $(i + 2) - c[3]
					if ((d1 < 0 ? -d1 : d1) + (d2 < 0 ? -d2 : d2) + (d3 < 0 ? -d3 : d3) < 60) continue
					x = ox + (i - 1) / 3; y = oy + NR - 1
					if (n == 0) { x0 = x1 = x; y0 = y1 = y }
					if (x < x0) x0 = x
					if (x > x1) x1 = x
					if (y < y0) y0 = y
					if (y > y1) y1 = y
					sx += x; sy += y; n++
				}
			}
			END { if (n) printf "%d %d %d %d %d %d %d\n", n, x0, x1, y0, y1, sx / n, sy / n; else print "0 0 0 0 0 0 0" }'
}

# white IMG X Y W H: how many pixels in the rectangle are near white (every channel above 200).
# Selected sprites are drawn white; no unselected sprite or beam in the fixture gets that bright.
white() {
	ffmpeg -loglevel error -i "$OUT/$1.png" -vf "crop=$4:$5:$2:$3" -f rawvideo -pix_fmt rgb24 - |
		od -An -v -tu1 -w"$(($4 * 3))" |
		awk '{ for (i = 1; i + 2 <= NF; i += 3) if ($i > 200 && $(i + 1) > 200 && $(i + 2) > 200) n++ } END { print n + 0 }'
}

# inner_edge IMG X Y W H left|right: "X Y" (screen pixels) of the non-background pixel nearest
# the left or right side of the rectangle whose 3x3 neighbourhood is all non-background, i.e. a
# point inside a file, not on its rim; "0 0" if there is none. The rim of a disc cut by the
# window edge is too thin to aim at, so the point is taken from the inside.
inner_edge() {
	local bg
	bg=$(ffmpeg -loglevel error -i "$OUT/$1.png" -vf "crop=1:1:$((WX + 3)):$((WY + 3))" -f rawvideo -pix_fmt rgb24 - | od -An -v -tu1)
	ffmpeg -loglevel error -i "$OUT/$1.png" -vf "crop=$4:$5:$2:$3" -f rawvideo -pix_fmt rgb24 - |
		od -An -v -tu1 -w"$(($4 * 3))" |
		awk -v bg="$bg" -v ox="$2" -v oy="$3" -v w="$4" -v side="$6" '
			BEGIN { split(bg, c, " ") }
			{
				for (i = 1; i + 2 <= NF; i += 3) {
					d1 = $i - c[1]; d2 = $(i + 1) - c[2]; d3 = $(i + 2) - c[3]
					nb[NR - 1, (i - 1) / 3] = ((d1 < 0 ? -d1 : d1) + (d2 < 0 ? -d2 : d2) + (d3 < 0 ? -d3 : d3) >= 60)
				}
				h = NR
			}
			END {
				for (k = 1; k < w - 1; k++) {
					x = (side == "left") ? k : w - 1 - k
					for (y = 1; y < h - 1; y++) {
						ok = 1
						for (dx = -1; dx <= 1 && ok; dx++) for (dy = -1; dy <= 1 && ok; dy++) if (!nb[y + dy, x + dx]) ok = 0
						if (ok) { print ox + x, oy + y; exit }
					}
				}
				print "0 0"
			}'
}

# hover_at X Y NAME: put the pointer on X,Y and shoot NAME.png once a box shows (more than 150
# px differ from p-narrow). Only the delivery is retried: up to 4 times the pointer leaves for the
# corner and comes back. The shot of the last try stays for the caller to judge.
hover_at() {
	local n
	for _ in 1 2 3 4; do
		xdotool mousemove $((WX + 12)) $((WY + WH - 14))
		sleep 0.3
		xdotool mousemove "$1" "$2"
		sleep 1
		shot "$3"
		read -r n _ < <(blob p-narrow "$3" 20 "$WX" "$WY" "$WW" "$WH")
		[ "$n" -gt 150 ] && return 0
	done
	return 1
}

# click_until NAME X Y REF: click at X,Y and shoot NAME.png once it differs from REF.png (more than
# 20 px anywhere in the window). Only the delivery is retried, up to 3 times; the assertion about
# what changed is the caller's, on the last shot.
click_until() {
	local n
	for _ in 1 2 3; do
		click_at "$2" "$3"
		sleep 1
		shot "$1"
		read -r n _ < <(blob "$4" "$1" 20 "$WX" "$WY" "$WW" "$WH")
		[ "$n" -gt 20 ] && return 0
	done
	return 1
}

# scene_ready: wait until the window shows a filled-in scene (users and files drawn). The sim clock
# advances by at most one frame's worth per frame, so on a starved CPU it is far behind the wall
# clock and a fixed sleep can pause an empty scene. Fails after 40 s.
scene_ready() {
	local n
	for _ in $(seq 80); do
		shot p-wait
		read -r n _ < <(nonbg p-wait "$WX" "$WY" "$WW" "$WH")
		if [ "$n" -gt 4000 ]; then
			rm -f "$OUT/p-wait.png"
			return 0
		fi
		sleep 0.5
	done
	return 1
}

# settle NAME: shoot once a second until two shots in a row are identical; the last one is
# NAME.png. Fails after 40 s.
settle() {
	rm -f "$OUT/$1-prev.png"
	for _ in $(seq 40); do
		sleep 1
		shot "$1"
		if [ -f "$OUT/$1-prev.png" ] && cmp -s "$OUT/$1.png" "$OUT/$1-prev.png"; then
			rm -f "$OUT/$1-prev.png"
			return 0
		fi
		cp "$OUT/$1.png" "$OUT/$1-prev.png"
	done
	return 1
}

# win_rect WID: sets WX WY WW WH, the client area on screen
win_rect() {
	WX=$(xwininfo -id "$1" | awk '/Absolute upper-left X/ {print $4}')
	WY=$(xwininfo -id "$1" | awk '/Absolute upper-left Y/ {print $4}')
	WW=$(xwininfo -id "$1" | awk '/Width:/ {print $2}')
	WH=$(xwininfo -id "$1" | awk '/Height:/ {print $2}')
}

# click_at X Y: a left click that stays down for 0.3 s (a background click only counts if the
# button is still down when a frame is drawn)
click_at() {
	xdotool mousemove "$1" "$2"
	sleep 0.2
	xdotool mousedown 1
	sleep 0.3
	xdotool mouseup 1
}

# A user that has been idle for `user-idle-time` fades and cannot be picked or Tab-selected, so a
# pause at an arbitrary moment could catch one half-faded. `user-idle-time` is config-file only;
# a very long one keeps every user whole.
printf '[gource]\nuser-idle-time=1000\n' >"$OUT/pointer.conf"
burl "$LOG" --load-config "$OUT/pointer.conf" --log-format custom -640x360 --seconds-per-day 10 \
	--hash-seed 31 --prng glibc --disable-auto-skip --disable-auto-rotate --file-idle-time 0 \
	--hide bloom,tree,usernames,filenames,dirnames,progress,date &
PBURL=$!
PWID=
for _ in $(seq 200); do
	PWID=$(xdotool search --onlyvisible --pid "$PBURL" 2>/dev/null | head -1)
	[ -n "$PWID" ] && break
	sleep 0.1
done
if [ -z "$PWID" ]; then
	bad "pointer launch: window never appeared"
	diag pointer "$PBURL"
elif win_rect "$PWID" && scene_ready && xdotool windowfocus --sync "$PWID" key space && settle p-base; then
	# Tab walks the users in order and some may be out of view, so press it until a pawn in the
	# window changes: the pixels it changes are that pawn.
	n=0
	for _ in $(seq 8); do
		xdotool windowfocus --sync "$PWID" key Tab
		sleep 1
		shot p-tab
		read -r n x0 x1 y0 y1 cx cy < <(blob p-base p-tab 20 "$WX" "$WY" "$WW" "$WH")
		[ "$n" -gt 50 ] && break
	done
	if [ "$n" -lt 200 ] || [ $((x1 - x0)) -gt 80 ] || [ $((y1 - y0)) -gt 80 ]; then
		bad "window did not react to Tab within 8 tries, so no pawn to aim at: input not delivered or no selectable user (environment, not a pointer check; changed pixels: $n, box $x0-$x1 x $y0-$y1)"
		diag pointer-tab "$PBURL"
	else
		rx=$((x0 - 4)) ry=$((y0 - 4)) rw=$((x1 - x0 + 9)) rh=$((y1 - y0 + 9))
		bgx=$((WX + 12)) bgy=$((WY + WH - 14))

		click_until p-desel "$bgx" "$bgy" p-tab
		read -r back _ < <(blob p-base p-desel 20 "$rx" "$ry" "$rw" "$rh")
		read -r gone _ < <(blob p-tab p-desel 20 "$rx" "$ry" "$rw" "$rh")
		# Setup for the pixel checks below, not a check of its own (the selection block above judges
		# deselecting by state): the pawn must be back in its own colour before it is clicked.
		if [ "$gone" -le 200 ] || [ "$back" -ge 10 ]; then
			bad "setup: the background click did not put the pawn back in its colour ($back px differ from before, $gone from selected)"
		fi

		# control for the next check: an unselected pawn under the pointer does get a box
		xdotool mousemove "$cx" "$cy"
		sleep 1
		shot p-hov-u
		read -r box _ < <(blob p-desel p-hov-u 20 "$WX" "$WY" "$WW" "$WH")
		if [ "$box" -gt 150 ]; then
			ok "hovering an unselected user shows a hover box ($box px)"
		else
			bad "no hover box over an unselected user ($box px)"
		fi

		click_until p-sel "$cx" "$cy" p-desel
		read -r moved _ < <(blob p-desel p-sel 20 "$rx" "$ry" "$rw" "$rh")
		read -r same _ < <(blob p-tab p-sel 20 "$rx" "$ry" "$rw" "$rh")
		w0=$(white p-desel "$rx" "$ry" "$rw" "$rh")
		w1=$(white p-sel "$rx" "$ry" "$rw" "$rh")
		if [ "$moved" -gt 200 ] && [ "$same" -lt 10 ] && [ "$w0" -lt 10 ] && [ "$w1" -gt 10 ]; then
			ok "clicking a user selects it: the sprite turns white ($w0 -> $w1 white px, same pixels as the Tab selection)"
		else
			bad "click on the user at $cx,$cy did not select it ($moved px changed, $same px differ from the Tab selection, white px $w0 -> $w1)"
		fi

		# the pointer is still on the selected pawn: no hover box. Same state, pointer moved off.
		xdotool mousemove "$bgx" "$bgy"
		sleep 1
		shot p-sel-off
		read -r box _ < <(blob p-sel p-sel-off 20 "$WX" "$WY" "$WW" "$WH")
		if [ "$box" -lt 5 ]; then
			ok "no hover box over the selected pawn"
		else
			bad "something is drawn over the selected pawn that is not there with the pointer away ($box px)"
		fi

		# Narrow window, users hidden: the crop puts files at both side edges (a hover box that
		# stays right of the pointer would not fit at the right one).
		click_at "$bgx" "$bgy"
		xdotool windowfocus --sync "$PWID" key g
		xdotool windowsize --sync "$PWID" 260 600
		win_rect "$PWID"
		xdotool mousemove $((WX + 12)) $((WY + WH - 14))
		if settle p-narrow; then
			read -r n x0 x1 _ < <(nonbg p-narrow "$WX" "$WY" "$WW" "$WH")
			# a point inside the file nearest each side edge
			read -r lx ly < <(inner_edge p-narrow "$WX" "$WY" "$WW" "$WH" left)
			read -r hx ry1 < <(inner_edge p-narrow "$WX" "$WY" "$WW" "$WH" right)
			if [ "$n" -lt 100 ] || [ $((x1 - x0)) -lt $((WW - 60)) ] || [ "$lx" = 0 ] || [ "$hx" = 0 ]; then
				bad "narrow window: no file to aim at on both side edges ($n px, x $x0-$x1 of $WX-$((WX + WW)), left $lx,$ly right $hx,$ry1)"
				diag pointer-narrow "$PBURL"
			else
				hover_at "$lx" "$ly" p-hov-l
				read -r n _ _ _ _ bcx _ < <(blob p-narrow p-hov-l 20 "$WX" "$WY" "$WW" "$WH")
				if [ "$n" -gt 150 ] && [ "$bcx" -gt $((lx + 20)) ]; then
					ok "hovering a file near the left edge shows a box to the right of the pointer (box centre x $bcx, pointer $lx)"
				else
					bad "left hover: $n px changed, box centre x $bcx, pointer $lx,$ly"
				fi
				hover_at "$hx" "$ry1" p-hov-r
				read -r n _ _ _ _ bcx _ < <(blob p-narrow p-hov-r 20 "$WX" "$WY" "$WW" "$WH")
				shown=$n
				if [ "$n" -gt 150 ] && [ "$bcx" -lt $((hx - 20)) ]; then
					ok "hovering a file near the right edge flips the box to the left of the pointer (box centre x $bcx, pointer $hx)"
				else
					bad "right hover: $n px changed, box centre x $bcx, pointer $hx"
				fi
				sleep 3.6
				shot p-idle
				read -r n _ < <(blob p-narrow p-idle 20 "$WX" "$WY" "$WW" "$WH")
				# the box was up ($shown px) a moment ago: the idle clock took it down
				if [ "$shown" -gt 150 ] && [ "$n" -lt 5 ]; then
					ok "the hover box goes after 3 s without pointer movement (MouseCursor idle timeout)"
				else
					bad "hover box not gone after 3.6 s idle ($n px differ from the no-hover shot, $shown while it was up)"
				fi

				# a file is selected the same way and turns white too; hovered while selected it
				# gets no box. The click is the first input after the idle, so it also wakes the cursor.
				click_until p-fsel "$hx" "$ry1" p-narrow
				# the file may be cut by the window edge: take the part of the 28 px square inside it
				fx=$((hx - 14)) fy=$((ry1 - 14)) fw=$((WX + WW - hx + 14))
				[ "$fw" -gt 28 ] && fw=28
				w0=$(white p-narrow "$fx" "$fy" "$fw" 28)
				w1=$(white p-fsel "$fx" "$fy" "$fw" 28)
				if [ "$w0" -lt 3 ] && [ "$w1" -gt 5 ]; then
					ok "clicking a file selects it: the sprite turns white ($w0 -> $w1 white px)"
				else
					bad "click on the file at $hx,$ry1 did not select it (white px $w0 -> $w1)"
				fi
				xdotool mousemove $((WX + 12)) $((WY + WH - 14))
				sleep 1
				shot p-fsel-off
				read -r n _ < <(blob p-fsel p-fsel-off 20 "$WX" "$WY" "$WW" "$WH")
				if [ "$n" -lt 5 ]; then
					ok "no hover box over the selected file"
				else
					bad "something is drawn over the selected file that is not there with the pointer away ($n px)"
				fi
			fi
		else
			bad "narrow window never stopped changing"
			diag pointer-narrow "$PBURL"
		fi
	fi
else
	bad "pointer checks: the scene never filled in, or never stopped moving after pausing"
	diag pointer "$PBURL"
fi
kill "$PBURL" 2>/dev/null
wait "$PBURL" 2>/dev/null

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
if [ "$(size)" = 1280x720 ]; then
	ok "Alt+Return goes fullscreen"
else
	bad "fullscreen size $(size), want 1280x720"
	diag fullscreen "$BURL"
fi
key alt+Return
sleep 1
[ "$(size)" != 1280x720 ] && ok "Alt+Return leaves fullscreen" || bad "still fullscreen"

key Escape
for _ in $(seq 50); do
	kill -0 "$BURL" 2>/dev/null || break
	sleep 0.1
done
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
