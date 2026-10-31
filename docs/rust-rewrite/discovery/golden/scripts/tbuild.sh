#!/bin/bash
# Build and run the Boost unit tests (src/test) against the objects from build.sh.
# Usage: BUILD=/path/to/build-dir ./tbuild.sh
set -uo pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
REPO=${REPO:-$(cd "$HERE/../../../../.." && pwd)}
BUILD=${BUILD:?set BUILD to the directory build.sh printed}
ARCH=${ARCH:--arch x86_64}
BREW=${BREW:-/usr/local}
cd "$BUILD"
CF="$ARCH -std=c++17 -O1 -w -I src -I src/core -I src/tinyxml -I src/test -DSDLAPP_RESOURCE_DIR=\"$BUILD/data\" -DHAVE_TIMEGM -DHAVE_UNSETENV $(pkg-config --cflags sdl2 SDL2_image glew freetype2 libpcre2-8 libpng) -I$BREW/include -I$BREW/opt/boost/include"
mkdir -p tobj
for f in src/test/main.cpp src/test/datetime_tests.cpp src/test/regex_tests.cpp; do
  clang++ $CF -c "$f" -o "tobj/$(basename "$f" .cpp).o" || exit 1
done
# Boost.Test is used header-only (boost/test/included), so main.o is excluded and no -lboost_unit_test_framework is needed.
clang++ $ARCH tobj/*.o $(ls obj/*.o | grep -v src_main.cpp.o) $(pkg-config --libs sdl2 SDL2_image glew freetype2 libpcre2-8 libpng) -L"$BREW/opt/boost/lib" -lboost_filesystem -lboost_system -framework OpenGL -framework CoreFoundation -o gource_tests || exit 1
./gource_tests --log_level=all --report_level=detailed 2>&1 | sed 's/\x1b\[[0-9;]*m//g' > "$HERE/../unit-tests.txt"
echo "rc=${PIPESTATUS[0]}"; tail -5 "$HERE/../unit-tests.txt"
