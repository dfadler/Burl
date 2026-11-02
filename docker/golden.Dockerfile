# Pinned image for generating goldens (#13, #17): debian:trixie-slim index digest, amd64 only.
# Used with --platform linux/amd64.
FROM debian@sha256:f6b08b09a05725f9431502d3b5714f5c4576e82c1a84fa1debe0c24a7e0c3ec4
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
 build-essential clang pkg-config autoconf automake libtool python3 xvfb xauth mesa-utils libgl1-mesa-dri \
 libsdl2-dev libsdl2-image-dev libpcre2-dev libfreetype-dev libglew-dev libglm-dev libboost-filesystem-dev libpng-dev libtinyxml-dev ca-certificates git \
 && rm -rf /var/lib/apt/lists/*
