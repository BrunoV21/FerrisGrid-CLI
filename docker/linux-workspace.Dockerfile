FROM rust:1-bookworm AS builder

WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo build --release -p ferrisgrid-cli

FROM debian:bookworm-slim

ENV DISPLAY=:99 \
    FERRISGRID_BACKEND=native-linux-x11 \
    FERRISGRID_OUTPUT_DIR=/workspace/.ferrisgrid \
    FERRISGRID_MAX_IMAGE_EDGE=1280 \
    XVFB_SCREEN=1280x800x24

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        chromium \
        dbus-x11 \
        fonts-dejavu \
        imagemagick \
        novnc \
        openbox \
        websockify \
        x11-utils \
        x11-xserver-utils \
        x11vnc \
        xdotool \
        xterm \
        xvfb \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /src/target/release/ferrisgrid /usr/local/bin/ferrisgrid
COPY docker/linux-workspace-entrypoint.sh /usr/local/bin/ferrisgrid-linux-workspace

WORKDIR /workspace
EXPOSE 6080

ENTRYPOINT ["/usr/local/bin/ferrisgrid-linux-workspace"]
