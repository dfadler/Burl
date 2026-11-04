#!/bin/bash
# Runs inside the golden image (see visual.sh): builds the untouched oracle `src/` exactly as
# regen-in-container.sh does (clang, signed char, no FMA contraction) unless /out/gource exists,
# then renders the structural fixtures under Xvfb + llvmpipe (ticket #12).
#   visual-in-container.sh build
#   visual-in-container.sh stream FIXTURE OUT.ppm      one warm-up render discarded first
#   visual-in-container.sh burl FIXTURE OUT.ppm       the same fixture through Burl (burl-visual image, no X)
#   visual-in-container.sh cut STREAM.ppm PREFIX I,J,..   write PREFIX-I.png (RGB8) for those frames
set -euo pipefail
T=/repo/tests/golden/sim
# Structural set: text and overlays hidden, bloom off. Fixed size/rate/seed like the sim gate.
# BLOOM=1 (the bloom set, ticket #12): same, but bloom drawn.
HIDE=mouse,date,dirnames,filenames,usernames,progress
[ "${BLOOM:-}" = 1 ] || HIDE=$HIDE,bloom
COMMON="-640x360 -r 25 --seconds-per-day 1 --hash-seed 31 --disable-auto-skip --disable-auto-rotate \
 --file-idle-time 0 --time-scale 1 --hide $HIDE"
# fixture -> "LOG EXTRA-ARGS"
fixture() {
  case $1 in
    small-repo) echo "$T/small-repo.log --log-format custom --stop-at-time 10" ;;
    file-removal) echo "$T/file-removal.log --log-format custom --stop-at-time 10" ;;
    file-to-dir) echo "$T/file-to-dir.log --log-format custom --stop-at-time 10" ;;
    real-burl-git-250) echo "$T/../real/burl-git-250.log --log-format git --stop-at-time 30" ;;
    *) echo "unknown fixture $1" >&2; exit 2 ;;
  esac
}
build() {
  [ -x /out/gource ] && return
  mkdir -p /work/src /work/b
  tar -C /repo --exclude=.git --exclude=target --exclude=tests/golden -cf - . | tar -C /work/src -xf -
  (cd /work/src && ./autogen.sh) > /work/autogen.log 2>&1
  cd /work/b && /work/src/configure CXX=clang++ CXXFLAGS="-O1 -fsigned-char -ffp-contract=off" > /work/configure.log 2>&1
  make -j"$(nproc)" > /work/make.log 2>&1 || { tail -30 /work/make.log; exit 1; }
  cp /work/b/gource /out/gource
}
render() { # LOG ARGS... -> PPM stream on stdout. Resources: the oracle looks in SDLAPP_RESOURCE_DIR.
  local log=$1; shift
  # shellcheck disable=SC2086 # COMMON is a word list by design
  LIBGL_ALWAYS_SOFTWARE=1 LP_NUM_THREADS=1 xvfb-run -a -s "-screen 0 1280x720x24" \
    /out/gource "$log" $COMMON "$@" -o -
}
case ${1:-} in
  build) build ;;
  stream)
    ln -sfn /repo/data /usr/local/share/gource  # SDLAPP_RESOURCE_DIR of the unmodified oracle build
    # shellcheck disable=SC2046 # fixture() prints a word list
    set -- $(fixture "$2") "$3"
    log=$1 out=${!#}; args=("${@:2:$#-2}")
    render "$log" "${args[@]}" > /dev/null  # warm-up: the first llvmpipe run in a container differs (#16)
    render "$log" "${args[@]}" > "$out"
    ;;
  burl)
    # shellcheck disable=SC2046 # fixture() prints a word list
    set -- $(fixture "$2") "$3"
    log=$1 out=${!#}; args=("${@:2:$#-2}")
    # shellcheck disable=SC2086 # COMMON is a word list by design
    # Gource never calls srand, so its rand() is glibc srand(1); Burl's default PRNG is clock-seeded.
    burl "$log" $COMMON "${args[@]}" --prng glibc -o "$out"
    ;;
  cut)
    python3 -I - "$2" "$3" "$4" <<'PY'
import re, struct, sys, zlib
stream, prefix, idx = sys.argv[1], sys.argv[2], [int(i) for i in sys.argv[3].split(",")]
b = open(stream, "rb").read()
frames, pos = [], 0
while pos < len(b):
    m = re.match(rb"P6\s+(\d+)\s+(\d+)\s+255\s", b[pos:pos + 32])
    w, h = int(m[1]), int(m[2])
    start = pos + m.end()
    frames.append((w, h, b[start:start + w * h * 3]))
    pos = start + w * h * 3
def chunk(t, d):
    return struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d))
for i in idx:
    w, h, rgb = frames[i]
    raw = b"".join(b"\0" + rgb[y * w * 3:(y + 1) * w * 3] for y in range(h))
    png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0)) \
        + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b"")
    open(f"{prefix}-{i}.png", "wb").write(png)
print("frames=%d" % len(frames))
PY
    ;;
  *) echo "usage: $0 build | stream FIXTURE OUT.ppm | burl FIXTURE OUT.ppm | cut STREAM.ppm PREFIX I,J,.." >&2; exit 2 ;;
esac
