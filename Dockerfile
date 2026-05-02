# syntax=docker/dockerfile:1
# TyphooN Terminal CLI/LAN server — multi-stage Docker build
# Builds only the CLI binary plus the shared engine LAN-sync code (no GPU dependencies).

# ── Builder stage ────────────────────────────────────────────────
FROM rust:1.86-bookworm AS builder

WORKDIR /build

# Install build dependencies for OpenSSL and SQLite (rusqlite bundled builds from source)
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace manifest and lockfile first (cache layer)
COPY Cargo.toml Cargo.lock ./

# Copy real CLI + shared engine sources. The CLI LAN server/client uses the same
# typhoon_engine::core::lan_sync implementation as the native GUI.
COPY cli/ cli/
COPY engine/ engine/
COPY mql5-compiler/ mql5-compiler/
COPY vendor/thirtyfour/ vendor/thirtyfour/

# Create stub workspace members that are not needed for the CLI image so Cargo
# can resolve the workspace without pulling GUI/web-server dependencies.
RUN mkdir -p native/src web-protocol/src web-server/src \
    && echo '[package]\nname = "typhoon-native"\nversion = "0.1.0"\nedition = "2024"\n\n[lib]\npath = "src/lib.rs"' > native/Cargo.toml \
    && echo 'pub fn stub() {}' > native/src/lib.rs \
    && echo '[package]\nname = "typhoon-web-protocol"\nversion = "0.1.0"\nedition = "2021"\n\n[lib]\npath = "src/lib.rs"' > web-protocol/Cargo.toml \
    && echo 'pub fn stub() {}' > web-protocol/src/lib.rs \
    && echo '[package]\nname = "typhoon-web-server"\nversion = "0.1.0"\nedition = "2021"\n\n[lib]\npath = "src/lib.rs"' > web-server/Cargo.toml \
    && echo 'pub fn stub() {}' > web-server/src/lib.rs

# Build only the CLI binary in release mode
RUN cargo build --release --package typhoon-cli \
    && strip /build/target/release/typhoon-cli

# ── Runtime stage ────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

LABEL org.opencontainers.image.title="TyphooN Terminal CLI/LAN Server" \
      org.opencontainers.image.description="TUI trading terminal and LAN cache sync server/client" \
      org.opencontainers.image.vendor="TyphooN" \
      org.opencontainers.image.licenses="BSL-1.1" \
      org.opencontainers.image.source="https://github.com/TyphooN-/TyphooN-Terminal"

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    libsqlite3-0 \
    && rm -rf /var/lib/apt/lists/*

# Non-root user for security
RUN groupadd --gid 1000 typhoon \
    && useradd --uid 1000 --gid typhoon --create-home typhoon

# Create data/cache directories for account registry and LAN SQLite cache.
RUN mkdir -p /data /cache && chown typhoon:typhoon /data /cache

# Copy binary from builder
COPY --from=builder /build/target/release/typhoon-cli /usr/local/bin/typhoon-cli

USER typhoon
WORKDIR /home/typhoon

# XDG_DATA_HOME so dirs::data_dir() resolves to /data inside container
ENV XDG_DATA_HOME=/data
ENV TYPHOON_CACHE_DIR=/cache

# LAN sync (wss://) and future Prometheus metrics endpoint.
EXPOSE 9847 9090

ENTRYPOINT ["typhoon-cli"]
