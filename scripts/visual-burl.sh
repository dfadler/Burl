#!/usr/bin/env bash
# Visual-equivalence gate, Burl side (ticket #12): render the structural fixtures with Burl in the
# pinned headless image (docker/release.Dockerfile) and SSIM each frame against the committed
# oracle references in tests/golden/visual/. Same flags and frame indices as scripts/visual.sh.
#   scripts/visual-burl.sh [MIN]    MIN defaults to 0.959 (docs/rust-rewrite/visual-gate.md)
# Exit 1 if any frame is below MIN. All frames are always reported.
set -euo pipefail

case "${1:-}" in -h | --help)
	sed -n '2,6p' "$0" | sed 's/^# \{0,1\}//'
	exit 0
	;;
esac

ROOT=$(cd "$(dirname "$0")/.." && pwd)
MIN=${1:-0.959}
OUT="$ROOT/target/visual-burl"
REFS="$ROOT/tests/golden/visual"
IMG=burl:release
# fixture:frame indices; keep in sync with scripts/visual.sh
FIXTURES="small-repo:30,70,110,170 file-removal:30,70,110,170 file-to-dir:30,70,110,170 real-burl-git-250:100,300,500,800"

mkdir -p "$OUT"
docker build -q --platform linux/amd64 -t "$IMG" -f "$ROOT/docker/release.Dockerfile" "$ROOT" >/dev/null
xtask() { (cd "$ROOT" && cargo run -q --locked -p xtask -- "$@"); }

render() { # LOG(relative to repo root) STOP FORMAT -> PPM stream on stdout
	docker run --rm --platform linux/amd64 -v "$ROOT":/repo:ro "$IMG" "/repo/$1" --log-format "$3" \
		-640x360 -r 25 --seconds-per-day 1 --hash-seed 31 --disable-auto-skip --disable-auto-rotate \
		--file-idle-time 0 --time-scale 1 --hide mouse,date,dirnames,filenames,usernames,progress,bloom \
		--stop-at-time "$2" -o -
}

fail=0
for f in $FIXTURES; do
	name=${f%%:*} idx=${f##*:}
	case $name in
	real-burl-git-250) log=tests/golden/real/burl-git-250.log stop=30 fmt=git ;;
	*) log=tests/golden/sim/$name.log stop=10 fmt=custom ;;
	esac
	render "$log" "$stop" "$fmt" >"$OUT/$name.ppm"
	xtask ssim-frames "$OUT/$name.ppm" "$OUT/$name" "$idx"
	for i in ${idx//,/ }; do
		if v=$(xtask ssim "$REFS/$name-$i.png" "$OUT/$name-$i.png" --min "$MIN"); then s=ok; else s=BELOW; fail=1; fi
		echo "$name frame=$i $v min=$MIN $s"
	done
done
exit $fail
