#!/bin/bash
# Deterministically writes the edge-case inputs and their .case files (custom log format,
# `TS|user|A|path`). Run by gen-cases.sh; outputs are committed.
set -euo pipefail
D=tests/golden/edge
mkdir -p $D
w() { printf '%b' "$2" > "$D/$1.log"; printf '%s\n' "$3" > "$D/$1.case"; }
C="--log-format custom"
w empty '' "$C"
w crlf '1000000000|alice|A|a.txt\r\n1000000100|bob|M|b.txt\r\n' "$C"
w blank-lines '\n1000000000|alice|A|a.txt\n\n\n1000000100|bob|M|b.txt\n\n' "$C"
w out-of-order '1000000200|alice|A|a.txt\n1000000100|bob|A|b.txt\n1000000150|carol|M|a.txt\n' "$C"
w pipe-in-name '1000000000|al|ice|A|a.txt\n1000000100|bob|A|dir/b|c.txt\n' "$C"
w bom '\xef\xbb\xbf1000000000|alice|A|a.txt\n1000000100|bob|M|b.txt\n' "$C"
w identical-timestamps '1000000000|alice|A|a.txt\n1000000000|bob|A|b.txt\n1000000000|alice|M|b.txt\n' "$C"
w invalid-utf8 '1000000000|al\xffice|A|a\xc3.txt\n1000000100|bob|A|b.txt\n' "$C"
w file-to-dir '1000000000|alice|A|x\n1000000100|alice|A|x/y.txt\n' "$C"
w dir-to-file '1000000000|alice|A|x/y.txt\n1000000100|alice|A|x\n' "$C"
long=$(printf 'd%.0s' $(seq 1 200))
w long-path "1000000000|alice|A|$long/$long/f.txt\n" "$C"
deep=""; for i in $(seq 1 60); do deep="$deep/d$i"; done
w deep-nesting "1000000000|alice|A|${deep#/}/f.txt\n1000000100|bob|M|${deep#/}/f.txt\n" "$C"
span='1577836800|alice|A|a.txt\n1577923200|bob|A|b.txt\n1578009600|carol|M|a.txt\n1609459200|alice|M|b.txt\n'
w start-stop-utc "$span" "TZ=UTC $C --start-date 2020-01-02 --stop-date 2020-06-01"
w start-stop-auckland "$span" "TZ=Pacific/Auckland $C --start-date 2020-01-02 --stop-date 2020-06-01"
w cvs-exp '000001:\n(date: 2004-01-02 03:04:05;  author: bob;  state: Exp;  lines: +1 -0)\n| src/a.c,v:1.1,\n\nfirst\n\n==========\n' "TZ=UTC"
