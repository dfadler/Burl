# State-dump fork build image (Burl). Debian trixie amd64, clang, signed char, no FP contraction.
# Build:  docker build --platform linux/amd64 -t burl-gource-dump reference/gource-cpp-dump
# Pinned to the debian:trixie-slim index digest used for the issue #16 measurements.
FROM debian@sha256:f6b08b09a05725f9431502d3b5714f5c4576e82c1a84fa1debe0c24a7e0c3ec4

RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
      build-essential clang pkg-config autoconf automake libtool python3 \
      xvfb xauth mesa-utils libgl1-mesa-dri \
      libsdl2-dev libsdl2-image-dev libpcre2-dev libfreetype-dev libglew-dev libglm-dev \
      libboost-filesystem-dev libpng-dev libtinyxml-dev ca-certificates \
 && rm -rf /var/lib/apt/lists/*

COPY . /src
WORKDIR /src

# -fsigned-char: string hashes depend on it (aarch64 Linux defaults to unsigned).
# -ffp-contract=off: no FMA contraction, so f32 results match the Rust port bit for bit.
ENV CC=clang CXX=clang++ \
    CXXFLAGS="-O1 -fsigned-char -ffp-contract=off"
RUN autoreconf -f -i && ./configure && make -j"$(nproc)" \
 && cp gource /usr/local/bin/gource-dump \
 && mkdir -p /usr/local/share/gource && cp -r data/. /usr/local/share/gource/

# llvmpipe software GL, one thread (issue #16: multi-threaded llvmpipe can wobble)
ENV LIBGL_ALWAYS_SOFTWARE=1 LP_NUM_THREADS=1 TZ=UTC
