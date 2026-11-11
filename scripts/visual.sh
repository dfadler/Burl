#!/usr/bin/env bash
# Visual-equivalence gate, oracle side (ticket #12): render the structural fixtures with the
# untouched oracle in the pinned amd64 Debian image (Xvfb + llvmpipe, LP_NUM_THREADS=1, one
# discarded warm-up run) and cut out the reference frames.
#   scripts/visual.sh refs [bloom]   rewrite tests/golden/visual/*.png (golden-refresh PR only); `bloom`: the
#                                    bloom set (bloom drawn, files bloom-*.png), else the structural set
#   scripts/visual.sh self [N]       N (default 10) further oracle renders, each frame SSIM'd against
#                                    the committed references; prints the worst and the proposed threshold
#   scripts/visual.sh burl [bloom]   render the fixtures with Burl (--prng glibc, like the oracle's unseeded
#                                    rand()) and print each frame's SSIM against the references; the
#                                    structural set exits 1 below 0.959 (ticket #12), bloom only reports
set -euo pipefail

case "${1:-}" in -h | --help | "")
	sed -n '2,11p' "$0" | sed 's/^# \{0,1\}//'
	exit 0
	;;
esac

ROOT=$(cd "$(dirname "$0")/.." && pwd)
OUT="$ROOT/target/visual"
REFS="$ROOT/tests/golden/visual"
IMG=burl-golden:amd64
# fixture:frame indices (-r 25: the 10 s fixtures run ~355 frames, the 30 s real-repo one ~1070)
FIXTURES=${VISUAL_FIXTURES:-"small-repo:30,70,110,170 file-removal:30,70,110,170 file-to-dir:30,70,110,170 real-burl-git-250:100,300,500,800"}

# Optional second word: `bloom` selects the bloom set (bloom drawn, refs named bloom-*).
export BLOOM='' PFX=''
if [ "$1" != self ] && [ "${2:-}" = bloom ]; then export BLOOM=1 PFX=bloom-; fi
THRESHOLD=0.959

mkdir -p "$OUT" "$REFS"
docker build -q --platform linux/amd64 -t "$IMG" -f "$ROOT/docker/golden.Dockerfile" "$ROOT/docker" > /dev/null
in_image() {
	docker run --rm --platform linux/amd64 -e BLOOM -v "$ROOT":/repo -v "$OUT":/out "$IMG" bash /repo/scripts/visual-in-container.sh "$@"
}
# SSIM maths needs the comparator (cargo xtask ssim, PR #56); cutting frames does not.
xtask() { (cd "$ROOT" && cargo run -q --locked -p xtask -- "$@"); }

# The burl mode compares against committed references and needs no oracle binary.
[ "$1" = burl ] || in_image build
case "$1" in
refs)
	for f in $FIXTURES; do
		in_image stream "${f%%:*}" /out/stream.ppm
		in_image cut /out/stream.ppm "/repo/tests/golden/visual/$PFX${f%%:*}" "${f##*:}"
	done
	{
		echo "image: debian@sha256:f6b08b09a05725f9431502d3b5714f5c4576e82c1a84fa1debe0c24a7e0c3ec4 (trixie-slim), amd64"
		echo "built image id: $(docker image inspect "$IMG" -f '{{.Id}}')"
		echo "source sha: $(git -C "$ROOT" rev-parse HEAD) (src/ must be unmodified at this commit)"
		echo "oracle: clang++ -O1 -fsigned-char -ffp-contract=off; Xvfb 1280x720x24, LIBGL_ALWAYS_SOFTWARE=1 LP_NUM_THREADS=1, warm-up render discarded"
		echo "frames: $FIXTURES${PFX:+ (bloom drawn; files bloom-*)}"
		echo "regenerate: scripts/visual.sh refs${PFX:+ bloom} (golden-refresh PR only)"
	} > "$REFS/PROVENANCE${PFX:+-bloom}.txt"
	;;
self)
	n=${2:-10}
	: > "$OUT/self.txt"
	for f in $FIXTURES; do
		for i in $(seq "$n"); do
			in_image stream "${f%%:*}" /out/stream.ppm
			in_image cut /out/stream.ppm /out/run "${f##*:}" > /dev/null
			for idx in $(echo "${f##*:}" | tr , ' '); do
				v=$(xtask ssim "$REFS/$PFX${f%%:*}-$idx.png" "$OUT/run-$idx.png")
				echo "${f%%:*} run=$i frame=$idx $v" | tee -a "$OUT/self.txt"
			done
		done
	done
	xtask ssim-threshold $(sed 's/.*ssim=//' "$OUT/self.txt")
	;;
burl)
	docker build -q --platform linux/amd64 -t burl-visual:amd64 -f "$ROOT/docker/visual.Dockerfile" "$ROOT" > /dev/null
	fail=0
	for f in $FIXTURES; do
		docker run --rm --platform linux/amd64 -e BLOOM -v "$ROOT":/repo -v "$OUT":/out burl-visual:amd64 \
			bash /repo/scripts/visual-in-container.sh burl "${f%%:*}" /out/burl.ppm
		in_image cut /out/burl.ppm /out/burl "${f##*:}" > /dev/null
		for idx in $(echo "${f##*:}" | tr , ' '); do
			v=$(xtask ssim "$REFS/$PFX${f%%:*}-$idx.png" "$OUT/burl-$idx.png")
			echo "${f%%:*} frame=$idx $v"
			awk -v v="${v#ssim=}" -v t="$THRESHOLD" 'BEGIN { exit !(v < t) }' && fail=1
		done
	done
	[ -n "$PFX" ] || exit $fail
	;;
*) exit 2 ;;
esac
