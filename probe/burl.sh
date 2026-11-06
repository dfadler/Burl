#!/bin/bash
# Runs the real burl on a hosted runner with 2x + 1x virtual displays; captures its window.
set -u
cd "$(dirname "$0")"
ROOT=$(cd .. && pwd)
clang -fobjc-arc -framework Cocoa -framework CoreGraphics vd.m -o vd
clang -fobjc-arc -framework Cocoa wid2.m -o wid2
echo "== cargo build"; (cd "$ROOT" && cargo build --release -p burl 2>&1 | tail -3); BURL="$ROOT/target/release/burl"; ls -l "$BURL" || exit 1
./vd 80 > vd.log 2>&1 & VD=$!; sleep 12; sed -n '/after second/,$p' vd.log
LOG="$ROOT/tests/golden/sim/small-repo.log"
"$BURL" "$LOG" --log-format custom -640x360 --seconds-per-day 10 --hash-seed 31 --disable-auto-skip --disable-auto-rotate --file-idle-time 0 --time-scale 1 --hide mouse --prng glibc --log-level debug > burl.out 2>&1 &
B=$!; sleep 8
echo "== burl alive: $(kill -0 $B 2>&1 && echo yes || echo no)"; head -20 burl.out
echo "== windows"; ./wid2 burl
cap(){ ID=$(./wid2 burl | head -1 | cut -d' ' -f1); [ -z "$ID" ] && { echo "no window to capture"; return; }
  screencapture -x -o -l "$ID" "$1.png" 2>&1; sips -g pixelWidth -g pixelHeight "$1.png" | tail -2
  sips -s format bmp "$1.png" --out "$1.bmp" >/dev/null 2>&1
  python3 - "$1.bmp" <<'PY'
import sys,struct
d=open(sys.argv[1],'rb').read(); off=struct.unpack_from('<I',d,10)[0]; bpp=struct.unpack_from('<H',d,28)[0]//8
px=d[off:]; s=set(px[i:i+bpp] for i in range(0,len(px)-bpp,bpp*7)); print("distinct colours (sampled):",len(s))
PY
}
echo "== capture where burl opened"; cap first
echo "== move burl onto the 2x display (x>=1920) via System Events"
osascript -e 'tell application "System Events" to tell (first process whose name is "burl") to set position of window 1 to {2100, 100}' 2>&1
sleep 3; ./wid2 burl; echo "== capture after move"; cap moved
kill $B 2>/dev/null; kill $VD 2>/dev/null; echo "== done"
