#!/bin/bash
# TyphooN Terminal — Launch script for Hyprland/NVIDIA/Wayland
#
# Required environment variables for WebKitGTK on Hyprland with NVIDIA:
#   WEBKIT_DISABLE_DMABUF_RENDERER=1  — prevents DMABUF crash on NVIDIA
#   WEBKIT_DISABLE_COMPOSITING_MODE=1 — disables GPU compositing (fixes blank window)
#   GDK_BACKEND=x11                   — forces X11 backend via XWayland (most stable)
#
# Usage:
#   ./launch.sh        — production build
#   ./launch.sh dev    — development mode with hot reload

set -euo pipefail

# Tauri v2 runs beforeDevCommand/beforeBuildCommand from src-tauri/,
# so cargo tauri must be invoked from src-tauri/ for 'cd ../frontend' to work.
cd "$(dirname "$0")/src-tauri"

export WEBKIT_DISABLE_DMABUF_RENDERER=1
export WEBKIT_DISABLE_COMPOSITING_MODE=1
export GDK_BACKEND=x11

if [ "${1:-}" = "dev" ]; then
    echo "Starting TyphooN Terminal (dev mode)..."
    cargo tauri dev
else
    echo "Starting TyphooN Terminal..."
    cargo tauri build
    echo "Build complete. Binary at target/release/typhoon-terminal"
fi
