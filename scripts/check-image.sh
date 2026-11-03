#!/usr/bin/env bash
# Docker headless export check (#14 exit criterion): run the image on a fixture log and verify
# the PPM stream is valid (P6 header, whole frames only, at least one frame).
#   scripts/check-image.sh [IMAGE]    (default burl:release; build it with docker/release.Dockerfile)
set -euo pipefail

case "${1:-}" in -h | --help)
	sed -n '2,4p' "$0" | sed 's/^# \{0,1\}//'
	exit 0
	;;
esac

ROOT=$(cd "$(dirname "$0")/.." && pwd)
IMG=${1:-burl:release}
W=320 H=240
out=$(mktemp)
trap 'rm -f "$out"' EXIT

docker run --rm --platform linux/amd64 -v "$ROOT/tests/logs":/logs:ro "$IMG" \
	--log-format custom --stop-at-time 1 "-${W}x${H}" -o - /logs/file-removal.log >"$out"

hdr="P6
$W $H
255
"
size=$(wc -c <"$out")
frame=$((${#hdr} + W * H * 3))
[ "$(head -c ${#hdr} "$out")" = "${hdr%$'\n'}" ] || { echo "FAIL: bad PPM header" >&2; exit 1; }
[ "$size" -ge "$frame" ] && [ $((size % frame)) -eq 0 ] ||
	{ echo "FAIL: $size bytes is not a whole number of $frame-byte frames" >&2; exit 1; }
echo "PPM OK: $((size / frame)) frames of ${W}x${H}"
