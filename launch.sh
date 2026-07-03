#!/bin/bash
# TyphooN Terminal — Native GPU renderer (egui + wgpu)
# Pure Rust, zero WebKit, zero JS.
#
# Usage:
#   ./launch.sh          — release thin-LTO build + run (fast normal path)
#   ./launch.sh dev      — debug build + run (faster compile)
#   ./launch.sh build    — faster thin-LTO release build only
#   ./launch.sh max      — release-max full-LTO build + run (slow final artifact)

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
    max)
        echo "TyphooN Terminal (release-max/full LTO)..."
        cargo run -p typhoon-native --profile release-max
        ;;
    *)
        echo "TyphooN Terminal (release)..."
        cargo run -p typhoon-native --release
        ;;
esac
