# syntax=docker/dockerfile:1
# TyphooN Terminal CLI — multi-stage Docker build
# Builds only the CLI binary (no GPU dependencies).

# ── Builder stage ────────────────────────────────────────────────
FROM rust:1.86-bookworm AS builder

WORKDIR /build

# Install build dependencies for OpenSSL and SQLite (rusqlite bundled builds from source)
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace manifest and lockfile first (cache layer)
COPY Cargo.toml Cargo.lock ./

# Copy real CLI source
COPY cli/ cli/

# Create stub workspace members so Cargo resolves the workspace without
# pulling in GPU (native) or parser (mql5-compiler) dependencies.
RUN mkdir -p engine/src native/src mql5-compiler/src \
    && echo '[package]\nname = "typhoon-engine"\nversion = "0.1.0"\nedition = "2024"\n\n[lib]\npath = "src/lib.rs"' > engine/Cargo.toml \
    && echo 'pub fn stub() {}' > engine/src/lib.rs \
    && echo '[package]\nname = "typhoon-native"\nversion = "0.1.0"\nedition = "2024"\n\n[lib]\npath = "src/lib.rs"' > native/Cargo.toml \
    && echo 'pub fn stub() {}' > native/src/lib.rs \
    && echo '[package]\nname = "mql5-compiler"\nversion = "0.1.0"\nedition = "2024"\n\n[lib]\npath = "src/lib.rs"' > mql5-compiler/Cargo.toml \
    && echo 'pub fn stub() {}' > mql5-compiler/src/lib.rs

# Build only the CLI binary in release mode
RUN cargo build --release --package typhoon-cli \
    && strip /build/target/release/typhoon-cli

# ── Runtime stage ────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

LABEL org.opencontainers.image.title="TyphooN Terminal CLI" \
      org.opencontainers.image.description="TUI trading terminal — Alpaca Markets, multi-account portfolio, risk management" \
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

# Create data directory for SQLite cache and account registry
RUN mkdir -p /data && chown typhoon:typhoon /data

# Copy binary from builder
COPY --from=builder /build/target/release/typhoon-cli /usr/local/bin/typhoon-cli

USER typhoon
WORKDIR /home/typhoon

# XDG_DATA_HOME so dirs::data_dir() resolves to /data inside container
ENV XDG_DATA_HOME=/data

# Future Prometheus metrics endpoint
EXPOSE 9090

ENTRYPOINT ["typhoon-cli"]
