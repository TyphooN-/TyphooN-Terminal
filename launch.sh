#!/bin/bash
# TyphooN Terminal — Native GPU renderer (egui + wgpu)
# Pure Rust, zero WebKit, zero JS.
#
# Usage:
#   ./launch.sh          — release build + run
#   ./launch.sh dev      — debug build + run (faster compile)
#   ./launch.sh build    — release build only

set -euo pipefail
cd "$(dirname "$0")"

case "${1:-}" in
    dev)
        echo "TyphooN Terminal (debug)..."
        cargo run -p typhoon-native
        ;;
    build)
        echo "Building TyphooN Terminal (release)..."
        cargo build -p typhoon-native --release
        echo "Binary: target/release/typhoon"
        ;;
    *)
        echo "TyphooN Terminal (release)..."
        cargo run -p typhoon-native --release
        ;;
esac
