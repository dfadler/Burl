#!/bin/bash
# Link the objects produced by build.sh into $BUILD/gource.
# Usage: BUILD=/path/to/build-dir ./link.sh
set -uo pipefail
BUILD=${BUILD:?set BUILD to the directory build.sh printed}
ARCH=${ARCH:--arch x86_64}
BREW=${BREW:-/usr/local}
cd "$BUILD"
clang++ $ARCH obj/*.o $(pkg-config --libs sdl2 SDL2_image glew freetype2 libpcre2-8 libpng) -L"$BREW/opt/boost/lib" -lboost_filesystem -lboost_system -framework OpenGL -framework CoreFoundation -o gource 2>&1 | head -20
ls -la gource
