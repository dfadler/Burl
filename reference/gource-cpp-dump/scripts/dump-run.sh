#!/bin/sh
# Run the state-dump fork once under Xvfb + llvmpipe (inside the image). Usage:
#   dump-run.sh LOG OUT.jsonl [extra gource args...]
# Fixed inputs follow parity-test-spec section 2.1 (Script-mode PRNG).
# One warm-up run is discarded first because the first llvmpipe run in a fresh container differs (issue #16).
set -eu
log=$1; out=$2; shift 2
# Script mode unless the fixture picks a generator itself with --prng.
case " $* " in *" --prng "*) rs= ;; *) rs="--rand-script /src/tests/dump/rand-script.txt" ;; esac
run() {
  dst=$1; shift
  xvfb-run -a -s "-screen 0 1280x720x24" gource-dump "$log" --log-format custom \
    -1280x720 -r 60 -o /dev/null --seconds-per-day 1 --stop-at-time 10 --hash-seed 31 \
    --disable-auto-skip --disable-auto-rotate --file-idle-time 0 \
    --time-scale 1 --hide mouse \
    $rs \
    --debug-dump-state "$dst" --debug-dump-max-frames 4000 "$@"
}
tmp=$(mktemp); trap 'rm -f "$tmp"' EXIT
[ -n "${NO_WARMUP:-}" ] || run "$tmp" "$@" >/dev/null 2>&1 || true
run "$out" "$@"
