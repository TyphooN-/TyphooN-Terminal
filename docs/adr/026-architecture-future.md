# ADR-026: Future Architecture — Headless Mode, WebWorker, Wasm, Pine Script

**Status:** Implemented
**Date:** 2026-03-17

## Headless CLI Backtest Mode (IMPLEMENTED)

### Usage
```bash
# Set credentials
export ALPACA_API_KEY=your_key
export ALPACA_SECRET_KEY=your_secret

# Run NNFX strategy on SMCI daily bars
./typhoon-terminal --backtest --symbol SMCI --timeframe 1Day --strategy nnfx

# Run SMA cross with custom parameters
./typhoon-terminal --backtest --symbol SPY --timeframe 4Hour --strategy sma_cross \
    --fast 20 --slow 50 --equity 50000 --limit 10000

# Use live account (default is paper)
./typhoon-terminal --backtest --symbol AAPL --timeframe 1Week --strategy nnfx --live
```

### Architecture
- CLI argument parsing in `main()` — checks for `--backtest` flag before Tauri init
- Creates a standalone tokio runtime (no GUI event loop)
- Connects to Alpaca, fetches bars, runs strategy, prints results to stdout
- Available strategies: `sma_cross` (SMA crossover), `nnfx` (KAMA + Fisher)
- Exit code 0 on success, 1 on error
- Can be scripted, piped, or run on a VPS via SSH

### Parameters
| Flag | Default | Description |
|---|---|---|
| `--symbol` | SPY | Trading symbol |
| `--timeframe` | 1Day | Bar timeframe |
| `--strategy` | nnfx | Strategy name |
| `--fast` | 10 | First period parameter |
| `--slow` | 32 | Second period parameter |
| `--equity` | 100000 | Initial equity |
| `--limit` | 5000 | Max bars to fetch |
| `--live` | (paper) | Use live account |

## Matrix Community Chat (IMPLEMENTED)

### Architecture
- No server infrastructure needed — uses Matrix Client-Server API over HTTPS
- 4 Tauri commands: `matrix_login`, `matrix_join`, `matrix_send`, `matrix_poll`
- Long-polling via `/sync` endpoint (5-second timeout)
- Session persisted in localStorage (access token + room ID)
- Default room: `#typhoon-terminal:matrix.org` (configurable)
- Supports any Matrix homeserver (matrix.org, Element, self-hosted)

## WebWorker for Indicators (IMPLEMENTED)

### Implementation (2026-03-18)
`indicator-worker.js` computes SMA, EMA, KAMA, RSI off the main thread using Wasm when available, JS fallback otherwise. Data sent as compact OHLCV arrays, results returned as flat value arrays. 5-second timeout falls back to main-thread computation. Worker initialized on app startup alongside Wasm engine.

### Architecture (if needed later)
```
Main Thread                    Worker Thread
     │                              │
     ├── postMessage(chartData) ──→ │
     │                              ├── calcSMA()
     │                              ├── calcKAMA()
     │                              ├── calcFisher()
     │                              ├── calcSupplyDemand()
     │   ←── postMessage(results) ──┤
     ├── apply results to chart     │
```

**Key challenge:** lightweight-charts only accepts data on the main thread. The worker can compute, but rendering must happen on the main thread. The data transfer overhead (structured clone of 10K-bar arrays) may negate the threading benefit.

**Alternative:** Use `OffscreenCanvas` in the worker for chart rendering. Not supported by lightweight-charts.

## Wasm Indicator Engine (IMPLEMENTED)

Implemented as a separate `wasm-indicators` crate (32KB Wasm binary). See [ADR-027](027-binary-storage-wasm-gpu.md) for full details.

### Architecture
```
wasm-indicators/src/lib.rs  →  wasm-pack  →  pkg/typhoon_indicators_bg.wasm (32KB)
                                                      ↓
Frontend: import init, { wasm_sma, ... } from './pkg/typhoon_indicators.js'
```

**Key decisions:**
- Separate `wasm-indicators` crate (not shared with backend backtester — different I/O contracts)
- `wasm-bindgen` for JS interop with flat `Float64Array` (5 values/bar, zero-copy)
- 15+ call sites in chart rendering route through Wasm with JS fallback
- Grid optimizer runs 50K combinations in ~100ms (50-100x faster than JS)

## MQL5 & PineScript Compatibility Layer (PLANNED — ADR-047)

**Status changed 2026-03-24:** Previously "NOT PLANNED". Now the core strategic direction.

The vision: compile MQL5 indicators/EAs and PineScript indicators to WASM, run them natively in TyphooN Terminal. This inherits both MT5 and TradingView ecosystems instantly.

### Architecture
- MQL5/PineScript → Rust parser (pest grammar) → TyphooN IR → WASM bytecode
- Compiled indicators run in Web Workers (async, sandboxed)
- Output buffers route directly to GPU `add_line()`/`add_histogram()`/`add_fill()`
- EA trading functions route through Tauri commands to broker APIs

### Key Benefits
- Users compile their existing MQL5 indicators directly (99.99% compatibility)
- PineScript indicators also supported via same IR pipeline
- WASM execution: near-native speed, 2-10KB per compiled indicator
- GPU rendering: all draw types mapped to WebGL2 primitives
- No ecosystem lock-in — users bring their own code

See **[ADR-047](047-mql5-pinescript-compatibility-layer.md)** for full specification including API surface, compilation pipeline, and implementation plan.
