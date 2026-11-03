#!/usr/bin/env bash
# Headless render smoke test (#15, #19): build the burl-gl smoke example into the pinned amd64
# Debian trixie-slim image and run it on Mesa llvmpipe. The example asserts its own pixels (incl.
# the row flip) and exits non-zero on a failure. Heavy-tier compatible: needs only Docker.
#   scripts/render-smoke.sh [OUT_DIR] [smoke args...]   e.g. scripts/render-smoke.sh out --size 1920x1080 --frames 20
set -euo pipefail

case "${1:-}" in -h | --help)
	sed -n '2,5p' "$0" | sed 's/^# \{0,1\}//'
	exit 0
	;;
esac

ROOT=$(cd "$(dirname "$0")/.." && pwd)
OUT=${1:-"$ROOT/target/render-smoke"}
[ $# -gt 0 ] && shift
IMG=burl-headless:amd64

mkdir -p "$OUT"
OUT=$(cd "$OUT" && pwd)
docker build --platform linux/amd64 -t "$IMG" -f "$ROOT/docker/headless.Dockerfile" "$ROOT"
docker run --rm --platform linux/amd64 -v "$OUT":/out "$IMG" --out /out/smoke.png "$@"
echo "PNG: $OUT/smoke.png"
