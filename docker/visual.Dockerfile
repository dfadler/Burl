# Burl side of the visual gate (#12): the release `burl` binary in the pinned Debian trixie-slim
# runtime (Mesa llvmpipe, headless EGL), same digest as golden.Dockerfile. amd64 only, like the refs.
# Build context is the repo root.
FROM rust:1-slim-trixie@sha256:b8e062b1c3a80183df83d9ba187cadd2b5ab84bc78d453dbfeb2fccd5f982247 AS build
WORKDIR /w
COPY Cargo.toml Cargo.lock ./
COPY crates crates
COPY data data
COPY xtask xtask
RUN --mount=type=cache,target=/w/target --mount=type=cache,target=/usr/local/cargo/registry \
    cargo build --release --locked -p burl && cp target/release/burl /burl

FROM debian@sha256:f6b08b09a05725f9431502d3b5714f5c4576e82c1a84fa1debe0c24a7e0c3ec4
RUN apt-get update && apt-get install -y --no-install-recommends \
    libegl1 libegl-mesa0 libgl1-mesa-dri \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /burl /usr/local/bin/burl
# Software GL, one rasteriser thread: the capture-determinism settings from #12.
ENV LIBGL_ALWAYS_SOFTWARE=1 LP_NUM_THREADS=1
