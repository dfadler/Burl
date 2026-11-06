#!/bin/bash
set -u
cd "$(dirname "$0")"
echo "== os"; sw_vers; uname -m
echo "== session"; launchctl managername 2>&1; id -un; who 2>&1 | head -3
clang -fobjc-arc -framework Cocoa -framework ApplicationServices pf.m -o pf && clang -fobjc-arc -framework Cocoa -framework CoreGraphics vd.m -o vd || { echo BUILD FAILED; exit 1; }
echo "== before virtual display"; ./pf
echo "-- screencapture (no virtual display)"; screencapture -x /tmp/before.png 2>&1; ls -l /tmp/before.png 2>&1; sips -g pixelWidth -g pixelHeight /tmp/before.png 2>&1 | tail -2
echo "== virtual displays (E1/E2 probe)"
./vd 25 > vd.log 2>&1 & VD=$!
sleep 12
cat vd.log | sed -n '/after HiDPI/,$p'
./pf
echo "-- screencapture (with virtual display)"; screencapture -x /tmp/after.png 2>&1; ls -l /tmp/after.png 2>&1; sips -g pixelWidth -g pixelHeight /tmp/after.png 2>&1 | tail -2
kill $VD 2>/dev/null; wait $VD 2>/dev/null
echo "== done"
