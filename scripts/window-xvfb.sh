#!/usr/bin/env bash
# Linux X11 window checks (interactive checklist, automated part): build docker/window.Dockerfile,
# open the real `burl` window under Xvfb + openbox, drive it with xdotool (pause, T, resize,
# Alt+Return, Esc) and check the result. Writes PNGs and session.mp4 to OUT.
#   scripts/window-xvfb.sh [OUT_DIR]     PLATFORM=linux/arm64 for a native run on Apple silicon
# Exit 1 if any check fails.
set -euo pipefail

case "${1:-}" in -h | --help)
	sed -n '2,6p' "$0" | sed 's/^# \{0,1\}//'
	exit 0
	;;
esac

ROOT=$(cd "$(dirname "$0")/.." && pwd)
OUT=${1:-"$ROOT/target/window-xvfb"}
PLATFORM=${PLATFORM:-linux/amd64}
IMG=burl-window:${PLATFORM#linux/}

mkdir -p "$OUT"
OUT=$(cd "$OUT" && pwd)
docker build --platform "$PLATFORM" -t "$IMG" -f "$ROOT/docker/window.Dockerfile" "$ROOT"
# The checks poll with timeouts; the cap here (WINDOW_WATCHDOG_SECS, default 240 s) stops any other hang from running for hours.
name="burl-window-$$" # the watchdog kills this run only, not another run of the same image
docker run --rm --name "$name" --platform "$PLATFORM" -v "$ROOT":/repo:ro -v "$OUT":/out "$IMG" &
pid=$!
secs=${WINDOW_WATCHDOG_SECS:-240}
# The timer is a subshell whose `sleep` is its own child: on TERM the subshell kills that sleep too,
# so a finished run leaves nothing behind (an orphaned sleep would hold a pipe open for `secs`).
(
	trap 'kill "$timer" 2>/dev/null; exit 0' TERM
	sleep "$secs" &
	timer=$!
	wait "$timer" && {
		echo "FAIL window checks still running after $secs s: killing the container" >&2
		docker kill "$name" >/dev/null 2>&1
	}
) &
watchdog=$!
rc=0
wait "$pid" || rc=$?
kill "$watchdog" 2>/dev/null || true # best effort: after a timeout it has already exited, and set -e must not replace rc
wait "$watchdog" 2>/dev/null || true
exit "$rc"
