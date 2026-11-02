#!/bin/bash
# Usage: gen-cases.sh <oracle-binary>  (cwd = repo root, normally inside the golden image)
# For every tests/golden/{edge,real}/NAME.case runs the oracle twice (as written, and with
# `--log-format X` removed = auto-detect) and records NAME.{out,err,rc} and NAME.auto.{out,err,rc}.
# A .case file is one line: optional NAME=VALUE env words, then oracle options; the harness
# appends `--output-custom-log - tests/golden/<dir>/NAME.log`.
set -uo pipefail
B=$1
cd /repo
bash docs/rust-rewrite/discovery/golden/scripts/make-edge-inputs.sh
bash docs/rust-rewrite/discovery/golden/scripts/make-vcs-logs.sh tests/golden/real
for c in tests/golden/edge/*.case tests/golden/real/*.case; do
  base=${c%.case}
  read -r -a words < "$c"
  env=(); opts=(); auto=(); skip=0
  for w in "${words[@]}"; do
    if [ $skip = 1 ]; then skip=0; opts+=("$w"); continue; fi
    case $w in
      *=*) env+=("$w") ;;
      --log-format) skip=1; opts+=("$w") ;;
      *) opts+=("$w"); auto+=("$w") ;;
    esac
  done
  # auto-detect variant: drop `--log-format X`
  auto=(); i=0
  while [ $i -lt ${#opts[@]} ]; do
    if [ "${opts[$i]}" = --log-format ]; then i=$((i+2)); continue; fi
    auto+=("${opts[$i]}"); i=$((i+1))
  done
  for v in "" auto; do
    args=("${opts[@]}"); suffix=""
    [ -n "$v" ] && { args=("${auto[@]}"); suffix=".auto"; }
    env "${env[@]}" timeout 20 "$B" "${args[@]}" --output-custom-log - "$base.log" \
      > "$base$suffix.out" 2> "$base$suffix.err" < /dev/null
    echo $? > "$base$suffix.rc"
  done
done
