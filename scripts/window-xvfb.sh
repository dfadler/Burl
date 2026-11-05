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
# The checks poll with timeouts; the 180 s cap here stops any other hang from running for hours.
name="burl-window-$$" # the watchdog kills this run only, not another run of the same image
docker run --rm --name "$name" --platform "$PLATFORM" -v "$ROOT":/repo:ro -v "$OUT":/out "$IMG" &
pid=$!
(sleep 180 && docker kill "$name" >/dev/null 2>&1) &
wait "$pid"
