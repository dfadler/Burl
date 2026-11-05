# Windowed-path test image: the `burl` binary plus Xvfb, a minimal window manager, xdotool and
# ffmpeg, so scripts/window-in-container.sh can drive the real window (keys, resize, fullscreen)
# and screenshot/record it. Same pinned Debian trixie-slim base as release.Dockerfile.
# amd64 only, like the goldens. Build context is the repo root.
FROM rust:1-slim-trixie@sha256:b8e062b1c3a80183df83d9ba187cadd2b5ab84bc78d453dbfeb2fccd5f982247 AS build
WORKDIR /w
COPY Cargo.toml Cargo.lock ./
COPY crates crates
COPY xtask xtask
# Resources are embedded with include_bytes! (#21), so the build needs them.
COPY data data
RUN cargo build --release --locked -p burl

FROM debian@sha256:f6b08b09a05725f9431502d3b5714f5c4576e82c1a84fa1debe0c24a7e0c3ec4
RUN apt-get update && apt-get install -y --no-install-recommends \
    libegl1 libegl-mesa0 libgl1-mesa-dri \
    xvfb openbox xdotool x11-utils ffmpeg \
    libx11-6 libxcursor1 libxi6 libxrandr2 libxkbcommon-x11-0 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /w/target/release/burl /usr/local/bin/burl
COPY scripts/window-in-container.sh /usr/local/bin/window-in-container.sh
# Software GL, one rasteriser thread: the capture-determinism settings from #12.
ENV LIBGL_ALWAYS_SOFTWARE=1 LP_NUM_THREADS=1
ENTRYPOINT ["bash", "/usr/local/bin/window-in-container.sh"]
