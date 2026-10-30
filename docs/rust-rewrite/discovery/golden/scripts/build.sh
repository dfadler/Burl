#!/bin/bash
# Build gource without autotools (macOS, Homebrew x86_64 libs under /usr/local, run under Rosetta).
# Usage: BUILD=/path/to/build-dir ./build.sh   (BUILD defaults to a temp dir; REPO defaults to this checkout)
set -uo pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
REPO=${REPO:-$(cd "$HERE/../../../../.." && pwd)}
BUILD=${BUILD:-$(mktemp -d "${TMPDIR:-/tmp}/gource-build.XXXXXX")}
ARCH=${ARCH:--arch x86_64}
BREW=${BREW:-/usr/local}
mkdir -p "$BUILD"
[ -d "$BUILD/src" ] || cp -R "$REPO/src" "$REPO/data" "$BUILD/"
cd "$BUILD"
SRC="src/main.cpp $(sed -n '/^sources = /,/^$/p' "$REPO/Makefile.am" | grep -o 'src/[a-z0-9_/.-]*\.cpp') $(ls src/tinyxml/*.cpp)"
CF="$ARCH -std=c++17 -O1 -w -I src -I src/core -I src/tinyxml -DSDLAPP_RESOURCE_DIR=\"$BUILD/data\" -DHAVE_TIMEGM -DHAVE_UNSETENV $(pkg-config --cflags sdl2 SDL2_image glew freetype2 libpcre2-8 libpng) -I$BREW/include -I$BREW/opt/boost/include"
mkdir -p obj log
for f in $SRC; do
  n=$(echo "$f" | tr / _)
  ( clang++ $CF -c "$f" -o "obj/$n.o" > "log/$n.txt" 2>&1 || echo "FAIL $f" ) &
  while [ "$(jobs -r | wc -l)" -ge 8 ]; do sleep 0.5; done
done
wait
echo "objects: $(ls obj | wc -l) in $BUILD"
echo "BUILD=$BUILD"
