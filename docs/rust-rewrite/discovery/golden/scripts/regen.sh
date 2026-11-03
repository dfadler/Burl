#!/bin/bash
# Regenerate every oracle golden inside the pinned amd64 Debian image (ticket #13/#17).
# Run from anywhere on a host with Docker; commit the result only in a `golden-refresh` PR.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../../../../.." && pwd)
IMG=burl-golden:amd64
docker build --platform linux/amd64 -t "$IMG" -f "$ROOT/docker/golden.Dockerfile" "$ROOT/docker"
docker run --rm --platform linux/amd64 -v "$ROOT":/repo \
  -e HOST_UID="$(id -u)" -e HOST_GID="$(id -g)" \
  -e SOURCE_SHA="$(git -C "$ROOT" rev-parse HEAD)" \
  -e IMAGE_ID="$(docker image inspect "$IMG" -f '{{.Id}}')" \
  "$IMG" bash /repo/docs/rust-rewrite/discovery/golden/scripts/regen-in-container.sh
