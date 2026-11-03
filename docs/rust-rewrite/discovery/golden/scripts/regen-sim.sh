#!/bin/bash
# Regenerate the simulation goldens (tests/golden/sim/*.jsonl) with the state-dump fork, inside
# its pinned amd64 Debian image (tickets #11, #13). Run from anywhere on a host with Docker;
# commit the result only in a `golden-refresh` PR. Each fixture is NAME.args (extra fork arguments, e.g. --debug-dump-every N) plus NAME.log, or
# `@conf:PATH` as the first word of NAME.args for a multi-section run; with names as arguments only those are regenerated. The fixed inputs are in reference/gource-cpp-dump/scripts/dump-run.sh.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../../../../.." && pwd)
FX=$ROOT/tests/golden/sim
IMG=burl-gource-dump:amd64
docker build --platform linux/amd64 -t "$IMG" "$ROOT/reference/gource-cpp-dump"
names=("$@")
[ ${#names[@]} -gt 0 ] || { for a in "$FX"/*.args; do names+=("$(basename "$a" .args)"); done; }
for n in "${names[@]}"; do
  args=$(cat "$FX/$n.args")
  # A fixture is NAME.log + NAME.args, or (multi-section) NAME.args whose first word is
  # @conf:PATH, PATH relative to tests/ (mounted at /t, read-only).
  case $args in
    @conf:*) input=/t/${args%% *}; input=${input/@conf:/}; args=${args#"${args%% *}"} ;;
    *) input=/fx/$n.log ;;
  esac
  # shellcheck disable=SC2086 # $args is a list of words by design
  docker run --rm --platform linux/amd64 -v "$FX":/fx -v "$ROOT/tests":/t:ro "$IMG" \
    /src/scripts/dump-run.sh "$input" "/fx/$n.jsonl" $args
done
{
  echo "image: debian@sha256:f6b08b09a05725f9431502d3b5714f5c4576e82c1a84fa1debe0c24a7e0c3ec4 (trixie-slim), amd64"
  echo "built image id: $(docker image inspect "$IMG" -f '{{.Id}}')"
  echo "source sha: $(git -C "$ROOT" rev-parse HEAD) (reference/gource-cpp-dump/ must be unmodified at this commit)"
  echo "fork: clang++ with -O1 -fsigned-char -ffp-contract=off (reference/gource-cpp-dump/Dockerfile), Xvfb + llvmpipe, one warm-up run"
  echo "regenerate: docs/rust-rewrite/discovery/golden/scripts/regen-sim.sh"
} > "$FX/PROVENANCE.txt"
