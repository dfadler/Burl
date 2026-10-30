#!/bin/bash
# Recapture the goldens from a built binary into docs/rust-rewrite/discovery/golden/.
# Usage: BUILD=/path/to/build-dir ./gold.sh   (binary expected at $BUILD/gource)
set -uo pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
REPO=${REPO:-$(cd "$HERE/../../../../.." && pwd)}
BUILD=${BUILD:?set BUILD to the directory build.sh printed}
B="$BUILD/gource"
G="$HERE/.."
mkdir -p "$G/custom-log"
# run <out> <errfile> args... ; 20s kill timeout (macOS has no `timeout`)
run() {
  out=$1; err=$2; shift 2
  "$@" > "$out" 2> "$err" < /dev/null &
  p=$!
  ( sleep 20; kill -9 $p 2>/dev/null ) &
  k=$!
  wait $p; rc=$?
  kill $k 2>/dev/null
  echo "rc=$rc  $(basename "$out")  ($*)" | sed "s|$B|gource|;s|$REPO/||g" >> "$G/exit-codes.txt"
}
: > "$G/exit-codes.txt"
run "$G/help-short.txt" "$G/help-short.stderr.txt" "$B" -h
run "$G/help-long.txt" "$G/help-long.stderr.txt" "$B" -H
run "$G/version.txt" "$G/version.stderr.txt" "$B" --version
for l in "$REPO"/tests/logs/*.log; do
  n=$(basename "$l" .log)
  fmt=custom; [ "$n" = svn-dir-delete ] && fmt=svn
  run "$G/custom-log/$n.txt" "$G/custom-log/$n.stderr.txt" "$B" --log-format $fmt --output-custom-log - "$l"
done
# stdin variant of the utf8 fixture (must match custom-log/utf8.txt)
"$B" --log-format custom --output-custom-log - - < "$REPO/tests/logs/utf8.log" > "$G/custom-log/utf8.stdin.txt" 2> "$G/custom-log/utf8.stdin.stderr.txt"
echo "rc=$?  utf8.stdin.txt  (gource --log-format custom --output-custom-log - - < tests/logs/utf8.log)" >> "$G/exit-codes.txt"
export SDL_VIDEODRIVER=dummy
run "$G/probe-dummy-ppm.bin" "$G/probe-dummy.stderr.txt" "$B" --stop-at-time 1 -o - "$REPO/tests/logs/utf8.log"
cat "$G/exit-codes.txt"
