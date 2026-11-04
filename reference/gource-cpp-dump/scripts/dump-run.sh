#!/bin/sh
# Run the state-dump fork once under Xvfb + llvmpipe (inside the image). Usage:
#   dump-run.sh LOG OUT.jsonl [extra gource args...]
#   dump-run.sh CONF OUT.jsonl [extra gource args...]     (CONF ends in .conf: a multi-section run)
# Fixed inputs follow parity-test-spec section 2.1 (Script-mode PRNG). A log run pins the
# per-repo options on the command line; a conf run pins only the display (the sections carry
# their own options, and command-line gource options would reach the first section only).
# The pseudo-argument @auto-rotate (log runs) leaves auto-rotate on; it is not passed to gource.
# @mouse (log runs) shows the mouse (hides only the progress slider), for --debug-pointer-script.
# The run happens in the directory of LOG or CONF so relative paths in the fixtures resolve.
# One warm-up run is discarded first because the first llvmpipe run in a fresh container differs (issue #16).
set -eu
log=$1; out=$2; shift 2
rot=--disable-auto-rotate
hide=mouse
n=$#
while [ "$n" -gt 0 ]; do
  a=$1; shift; n=$((n - 1))
  case $a in @auto-rotate) rot= ;; @mouse) hide=progress ;; *) set -- "$@" "$a" ;; esac
done
# Script mode unless the fixture picks a generator itself with --prng.
case " $* " in *" --prng "*) rs= ;; *) rs="--rand-script /src/tests/dump/rand-script.txt" ;; esac
cd "$(dirname "$log")"
run() {
  dst=$1; shift
  case $log in
    *.conf)
      xvfb-run -a -s "-screen 0 1280x720x24" gource-dump "$(basename "$log")" \
        -1280x720 -r 60 -o /dev/null \
        $rs \
        --debug-dump-state "$dst" --debug-dump-max-frames 4000 "$@" ;;
    *)
      # shellcheck disable=SC2086 # $rot and $rs are empty or a flag with its value, by design
      xvfb-run -a -s "-screen 0 1280x720x24" gource-dump "$log" --log-format custom \
        -1280x720 -r 60 -o /dev/null --seconds-per-day 1 --stop-at-time 10 --hash-seed 31 \
        --disable-auto-skip $rot --file-idle-time 0 \
        --time-scale 1 --hide $hide \
        $rs \
        --debug-dump-state "$dst" --debug-dump-max-frames 4000 "$@" ;;
  esac
}
tmp=$(mktemp); trap 'rm -f "$tmp"' EXIT
[ -n "${NO_WARMUP:-}" ] || run "$tmp" "$@" >/dev/null 2>&1 || true
run "$out" "$@"
