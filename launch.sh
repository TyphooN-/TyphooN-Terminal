#!/bin/bash
# TyphooN Terminal — Native GPU renderer (egui + wgpu)
# Pure Rust, zero WebKit, zero JS.
#
# Usage:
#   ./launch.sh          — release-max full-LTO build + run
#   ./launch.sh dev      — debug build + run (faster compile)
#   ./launch.sh build    — faster thin-LTO release build only
#   ./launch.sh max      — release-max full-LTO build + run

set -euo pipefail
cd "$(dirname "$0")"

# Build WASM web client if trunk is available and web/ exists
build_wasm() {
    if command -v trunk &>/dev/null && [ -d web ]; then
        if [ ! -f target/web-dist/index.html ] || \
           [ web/src/app.rs -nt target/web-dist/index.html ] || \
           [ web/src/lib.rs -nt target/web-dist/index.html ] || \
           [ web-protocol/src/lib.rs -nt target/web-dist/index.html ]; then
            # Ensure wasm32 target is installed (survives rustup updates)
            rustup target add wasm32-unknown-unknown 2>/dev/null || true
            echo "Building WASM web client..."
            (cd web && trunk build --release 2>&1 | tail -3)
        fi
    fi
}

case "${1:-}" in
    dev)
        echo "TyphooN Terminal (debug)..."
        build_wasm
        cargo run -p typhoon-native
        ;;
    build)
        echo "Building TyphooN Terminal (release)..."
        build_wasm
        cargo build -p typhoon-native --release
        echo "Binary: target/release/typhoon"
        ;;
    max)
        echo "TyphooN Terminal (release-max/full LTO)..."
        build_wasm
        cargo run -p typhoon-native --profile release-max
        ;;
    web)
        echo "Building WASM web client (force)..."
        (cd web && trunk build --release)
        ;;
    *)
        echo "TyphooN Terminal (release-max/full LTO)..."
        build_wasm
        cargo run -p typhoon-native --profile release-max
        ;;
esac
