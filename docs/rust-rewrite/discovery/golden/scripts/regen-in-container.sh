#!/bin/bash
# Runs inside the golden image (see regen.sh). Builds the untouched oracle `src/` with clang
# (signed char, no FMA contraction, ticket #11) and recaptures all goldens.
set -euo pipefail
G=/repo/docs/rust-rewrite/discovery/golden
mkdir -p /work/src /work/b
tar -C /repo --exclude=.git --exclude=target --exclude=tests/golden -cf - . | tar -C /work/src -xf -
cd /work/src && ./autogen.sh > /work/autogen.log 2>&1
cd /work/b && /work/src/configure CXX=clang++ CXXFLAGS="-O1 -fsigned-char -ffp-contract=off" > /work/configure.log 2>&1
make -j"$(nproc)" > /work/make.log 2>&1
mkdir -p /work/gb && cp /work/b/gource /work/gb/gource
cd /repo
BUILD=/work/gb REPO=/repo bash "$G/scripts/gold.sh" > /work/gold.out
bash "$G/scripts/gen-cases.sh" /work/gb/gource
rm -f "$G/probe-dummy-ppm.bin"  # empty stdout of the SDL probe, never committed
{
  echo "image: debian@sha256:f6b08b09a05725f9431502d3b5714f5c4576e82c1a84fa1debe0c24a7e0c3ec4 (trixie-slim), amd64"
  echo "built image id: $IMAGE_ID"
  echo "source sha: $SOURCE_SHA (src/ must be unmodified at this commit)"
  echo "oracle: clang++ $(clang++ --version | head -1), CXXFLAGS=-O1 -fsigned-char -ffp-contract=off"
  echo "regenerate: docs/rust-rewrite/discovery/golden/scripts/regen.sh"
} > /repo/tests/golden/PROVENANCE.txt
chown -R "$HOST_UID:$HOST_GID" /repo/tests /repo/docs 2>/dev/null || true
