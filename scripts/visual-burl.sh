#!/usr/bin/env bash
# Visual-equivalence gate, Burl side (ticket #12): render the structural fixtures with Burl in the
# pinned headless image (docker/release.Dockerfile) and SSIM each frame against the committed
# oracle references in tests/golden/visual/. Same flags and frame indices as scripts/visual.sh.
#   scripts/visual-burl.sh [MIN]    MIN defaults to 0.959 (docs/rust-rewrite/visual-gate.md)
# Exit 1 if any frame is below MIN. All frames are always reported.
#   BLOOM=1 scripts/visual-burl.sh   the bloom set (bloom drawn, refs bloom-*.png): report-only, always exit 0,
#                                    prints SSIM and mean luma per frame (limit 5%, visual-gate.md; #89, #53)
set -euo pipefail

case "${1:-}" in -h | --help)
	sed -n '2,8p' "$0" | sed 's/^# \{0,1\}//'
	exit 0
	;;
esac

ROOT=$(cd "$(dirname "$0")/.." && pwd)
MIN=${1:-0.959}
BLOOM=${BLOOM:-}
PFX='' HIDE=mouse,date,dirnames,filenames,usernames,progress
if [ "$BLOOM" = 1 ]; then PFX=bloom-; else HIDE=$HIDE,bloom; fi
OUT="$ROOT/target/visual-burl"
REFS="$ROOT/tests/golden/visual"
IMG=burl:release
# fixture:frame indices; keep in sync with scripts/visual.sh
FIXTURES="small-repo:30,70,110,170 file-removal:30,70,110,170 file-to-dir:30,70,110,170 real-burl-git-250:100,300,500,800"

mkdir -p "$OUT"
docker build -q --platform linux/amd64 -t "$IMG" -f "$ROOT/docker/release.Dockerfile" "$ROOT" >/dev/null
xtask() { (cd "$ROOT" && cargo run -q --locked -p xtask -- "$@"); }

# --prng glibc: the oracle never calls srand, so its rand() is glibc srand(1); Burl's default PRNG is
# clock-seeded, which made every CI run score differently (visual-gate.md, PR #75).
render() { # LOG(relative to repo root) STOP FORMAT HIDE -> PPM stream on stdout
	docker run --rm --platform linux/amd64 -v "$ROOT":/repo:ro "$IMG" "/repo/$1" --log-format "$3" \
		-640x360 -r 25 --seconds-per-day 1 --hash-seed 31 --disable-auto-skip --disable-auto-rotate \
		--file-idle-time 0 --time-scale 1 --hide "$4" \
		--stop-at-time "$2" --prng glibc -o -
}

fail=0
for f in $FIXTURES; do
	name=${f%%:*} idx=${f##*:}
	case $name in
	real-burl-git-250) log=tests/golden/real/burl-git-250.log stop=30 fmt=git ;;
	*) log=tests/golden/sim/$name.log stop=10 fmt=custom ;;
	esac
	# structural mode: a render or extract failure aborts (exit 1); bloom mode reports it and moves on
	if ! { render "$log" "$stop" "$fmt" "$HIDE" >"$OUT/$name.ppm" && xtask ssim-frames "$OUT/$name.ppm" "$OUT/$name" "$idx"; }; then
		[ "$BLOOM" = 1 ] || exit 1
		echo "$name BLOOM-ERROR render or frame extraction failed"
		continue
	fi
	for i in ${idx//,/ }; do
		s=ok rc=0
		v=$(xtask ssim "$REFS/$PFX$name-$i.png" "$OUT/$name-$i.png" --min "$MIN") || rc=$?
		# xtask ssim: 1 = below MIN; anything else (missing ref, size mismatch) is an input error
		case $rc in
		0) ;;
		1)
			s=BELOW
			fail=1
			;;
		*)
			s=ERROR v=ssim=ERROR
			fail=1
			;;
		esac
		[ "$BLOOM" != 1 ] || v="$v $(xtask ssim-luma "$REFS/$PFX$name-$i.png" "$OUT/$name-$i.png" 2>&1 || echo 'luma=ERROR')"
		echo "$name frame=$i $v min=$MIN $s"
	done
done
[ "$BLOOM" = 1 ] || exit $fail
