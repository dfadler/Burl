#!/bin/bash
S=/private/tmp/claude-501/-Users-dfadler-Development-Burl/28b2cf9c-5b37-48fc-9223-8a7d96cdc267/scratchpad/gb
MA=/Users/dfadler/Development/Burl/.claude/worktrees/rust-rewrite-discovery/Makefile.am
cd $S
SRC="src/main.cpp $(sed -n '/^sources = /,/^$/p' $MA | grep -o 'src/[a-z0-9_/.-]*\.cpp') $(ls src/tinyxml/*.cpp)"
CF="-arch x86_64 -std=c++17 -O1 -w -I src -I src/core -I src/tinyxml -DSDLAPP_RESOURCE_DIR=\"$S/data\" -DHAVE_TIMEGM -DHAVE_UNSETENV $(pkg-config --cflags sdl2 SDL2_image glew freetype2 libpcre2-8 libpng) -I/usr/local/include -I/usr/local/opt/boost/include"
mkdir -p obj log
for f in $SRC; do
  n=$(echo $f | tr / _)
  ( clang++ $CF -c $f -o obj/$n.o > log/$n.txt 2>&1 || echo "FAIL $f" ) &
  while [ $(jobs -r | wc -l) -ge 8 ]; do sleep 0.5; done
done
wait
ls obj | wc -l
