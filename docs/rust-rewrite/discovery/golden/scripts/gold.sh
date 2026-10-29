#!/bin/bash
W=/Users/dfadler/Development/Burl/.claude/worktrees/rust-rewrite-discovery
G=$W/docs/rust-rewrite/discovery/golden
B=/private/tmp/claude-501/-Users-dfadler-Development-Burl/28b2cf9c-5b37-48fc-9223-8a7d96cdc267/scratchpad/gb/gource
mkdir -p $G/custom-log
# run <out> <errfile> args... ; 20s kill timeout
run() {
  out=$1; err=$2; shift 2
  "$@" > "$out" 2> "$err" < /dev/null &
  p=$!
  ( sleep 20; kill -9 $p 2>/dev/null ) &
  k=$!
  wait $p; rc=$?
  kill $k 2>/dev/null
  echo "rc=$rc  $(basename $out)  ($*)" | sed "s|$B|gource|;s|$W/||g" >> $G/exit-codes.txt
}
: > $G/exit-codes.txt
run $G/help-short.txt $G/help-short.stderr.txt $B -h
run $G/help-long.txt $G/help-long.stderr.txt $B -H
run $G/version.txt $G/version.stderr.txt $B --version
for l in $W/tests/logs/*.log; do
  n=$(basename $l .log)
  fmt=custom; [ $n = svn-dir-delete ] && fmt=svn
  run $G/custom-log/$n.txt $G/custom-log/$n.stderr.txt $B --log-format $fmt --output-custom-log - $l
done
export SDL_VIDEODRIVER=dummy
run $G/probe-dummy-ppm.bin $G/probe-dummy.stderr.txt $B --stop-at-time 1 -o - $W/tests/logs/utf8.log
cat $G/exit-codes.txt
