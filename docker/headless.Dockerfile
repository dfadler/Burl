# Headless render image (#15, #19): builds the burl-gl smoke example and runs it on Mesa llvmpipe in
# the pinned debian:trixie-slim (same index digest as golden.Dockerfile). amd64 only, like the goldens.
# Build context is the repo root; build stage uses Rust's image because Debian's cargo is too old.
FROM rust:1-slim-trixie@sha256:b8e062b1c3a80183df83d9ba187cadd2b5ab84bc78d453dbfeb2fccd5f982247 AS build
WORKDIR /w
COPY Cargo.toml Cargo.lock ./
COPY crates crates
COPY xtask xtask
RUN cargo build --release --locked -p burl-gl --example smoke

FROM debian@sha256:f6b08b09a05725f9431502d3b5714f5c4576e82c1a84fa1debe0c24a7e0c3ec4
RUN apt-get update && apt-get install -y --no-install-recommends \
    libegl1 libegl-mesa0 libgl1-mesa-dri \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /w/target/release/examples/smoke /usr/local/bin/smoke
# Software GL, one rasteriser thread: the capture-determinism settings from #12.
ENV LIBGL_ALWAYS_SOFTWARE=1 LP_NUM_THREADS=1
ENTRYPOINT ["smoke"]
