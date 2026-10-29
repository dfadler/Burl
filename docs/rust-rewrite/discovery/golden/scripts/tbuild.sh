#!/bin/bash
cd /private/tmp/claude-501/-Users-dfadler-Development-Burl/28b2cf9c-5b37-48fc-9223-8a7d96cdc267/scratchpad/gb || exit 1
S=$PWD
mkdir -p tobj
for f in main datetime_tests regex_tests; do
  clang++ -arch x86_64 -std=c++17 -O1 -w -I src -I src/core -I src/tinyxml -I src/test -DSDLAPP_RESOURCE_DIR=\"$S/data\" -DHAVE_TIMEGM -DHAVE_UNSETENV $(pkg-config --cflags sdl2 SDL2_image glew freetype2 libpcre2-8 libpng) -I/usr/local/include -I/usr/local/opt/boost/include -c src/test/$f.cpp -o tobj/$f.o || exit 1
done
OBJS=$(ls obj/*.o | grep -v 'src_main.cpp.o')
clang++ -arch x86_64 tobj/*.o $OBJS $(pkg-config --libs sdl2 SDL2_image glew freetype2 libpcre2-8 libpng) -L/usr/local/opt/boost/lib -lboost_filesystem -lboost_system -framework OpenGL -framework CoreFoundation -o gource_tests || exit 1
echo BUILD_OK
