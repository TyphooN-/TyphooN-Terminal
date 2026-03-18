#!/bin/bash
# TyphooN Terminal — Cross-compile Windows binary from Linux via MinGW
#
# Prerequisites:
#   sudo pacman -S mingw-w64-gcc         # MinGW cross-compiler
#   rustup target add x86_64-pc-windows-gnu  # Rust Windows target
#
# This builds a Windows .exe that bundles WebView2 (Edge-based) instead of WebKitGTK.
# Note: Tauri on Windows uses WebView2 (pre-installed on Windows 10/11).
#
# Usage:
#   ./build-windows.sh          # Release build
#   ./build-windows.sh debug    # Debug build (faster, larger)

set -e

echo "╔══════════════════════════════════════════════╗"
echo "║  TyphooN Terminal — Windows Cross-Compile    ║"
echo "╚══════════════════════════════════════════════╝"

# Ensure Rust Windows target is installed
if ! rustup target list --installed | grep -q "x86_64-pc-windows-gnu"; then
    echo "Installing Rust Windows target (x86_64-pc-windows-gnu)..."
    rustup target add x86_64-pc-windows-gnu
fi

# Ensure MinGW is available
if ! which x86_64-w64-mingw32-gcc > /dev/null 2>&1; then
    echo "ERROR: MinGW not found. Install with:"
    echo "  sudo pacman -S mingw-w64-gcc    # Arch"
    echo "  sudo apt install gcc-mingw-w64  # Debian/Ubuntu"
    exit 1
fi

# Build frontend first
echo ""
echo "Building frontend..."
cd frontend
npm run build
cd ..

# Configure linker for Windows target
export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc
export CC_x86_64_pc_windows_gnu=x86_64-w64-mingw32-gcc
export CXX_x86_64_pc_windows_gnu=x86_64-w64-mingw32-g++
export AR_x86_64_pc_windows_gnu=x86_64-w64-mingw32-ar

# Build mode
PROFILE="release"
CARGO_FLAGS="--release"
if [ "$1" = "debug" ]; then
    PROFILE="debug"
    CARGO_FLAGS=""
    echo "Building in DEBUG mode..."
else
    echo "Building in RELEASE mode..."
fi

# Build the Tauri app for Windows
echo ""
echo "Compiling Rust backend for Windows (x86_64-pc-windows-gnu)..."
cd src-tauri
cargo build $CARGO_FLAGS --target x86_64-pc-windows-gnu 2>&1

# Check result
BINARY="target/x86_64-pc-windows-gnu/${PROFILE}/typhoon-terminal.exe"
if [ -f "$BINARY" ]; then
    SIZE=$(du -h "$BINARY" | cut -f1)
    echo ""
    echo "╔══════════════════════════════════════════════╗"
    echo "║  BUILD SUCCESSFUL                            ║"
    echo "╠══════════════════════════════════════════════╣"
    echo "║  Binary: $BINARY"
    echo "║  Size:   $SIZE"
    echo "║                                              ║"
    echo "║  Copy to Windows and run.                    ║"
    echo "║  Requires WebView2 (pre-installed Win10/11). ║"
    echo "╚══════════════════════════════════════════════╝"
else
    echo ""
    echo "BUILD FAILED — check errors above."
    echo ""
    echo "Common issues:"
    echo "  1. Missing Windows system libraries — Tauri needs Windows SDK headers"
    echo "  2. SQLite bundled compilation may fail — try: SQLITE_SYSTEM_LIBS=1"
    echo "  3. WebView2 headers — may need to build on Windows with cargo tauri build"
    echo ""
    echo "Alternative: build natively on Windows with:"
    echo "  cargo install tauri-cli"
    echo "  cargo tauri build"
    exit 1
fi
