#!/bin/bash
# TyphooN Terminal CLI — launch script
# Usage: ./typhoon.sh [args]
#   ./typhoon.sh                    # Interactive TUI
#   ./typhoon.sh --positions        # Print positions
#   ./typhoon.sh --account          # Print account
#   ./typhoon.sh --accounts         # All accounts (Alpaca + MT5 imports)
#   ./typhoon.sh -s BTC/USD         # Start with symbol
#   ./typhoon.sh --import-mt5 NAME:/path/to/statement.csv

set -euo pipefail
cd "$(dirname "$0")"

# Build if binary doesn't exist or source is newer
if [ ! -f target/release/typhoon ] || [ src/main.rs -nt target/release/typhoon ] || [ src/broker.rs -nt target/release/typhoon ]; then
    echo "Building TyphooN CLI..."
    cargo build --release 2>&1 | tail -3
fi

exec ./target/release/typhoon "$@"
