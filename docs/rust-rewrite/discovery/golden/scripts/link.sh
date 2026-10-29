#!/bin/bash
cd /private/tmp/claude-501/-Users-dfadler-Development-Burl/28b2cf9c-5b37-48fc-9223-8a7d96cdc267/scratchpad/gb
clang++ -arch x86_64 obj/*.o $(pkg-config --libs sdl2 SDL2_image glew freetype2 libpcre2-8 libpng) -L/usr/local/opt/boost/lib -lboost_filesystem -lboost_system -framework OpenGL -framework CoreFoundation -o gource 2>&1 | head -20
ls -la gource
