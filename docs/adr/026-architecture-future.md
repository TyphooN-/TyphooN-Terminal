# ADR-026: Future Architecture — Headless Mode, WebWorker, Wasm, Pine Script

**Status:** Partially Implemented
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

## WebWorker for Indicators (DEFERRED)

### Why Deferred
The 35 indicator calculation functions (839 lines) are pure functions that could run in a WebWorker. However:
1. The functions are already fast enough for current bar counts (< 50ms for 10K bars)
2. The bottleneck is API fetching, not computation
3. Moving to a worker requires serializing/deserializing large data arrays across threads
4. The incremental update pattern (only recompute last bar) is more impactful

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

## Wasm Indicator Engine (DEFERRED)

### Why Deferred
- JavaScript is already fast enough for the current workload
- Wasm compilation adds build complexity (wasm-pack, wasm-bindgen)
- The indicator math would need to be ported from JS to Rust (duplication)
- Marginal benefit: 2-5x speedup on computation that takes < 50ms

### Architecture (if needed later)
```
Rust (src-tauri/src/indicators/)  →  wasm-pack  →  pkg/indicators_bg.wasm
                                                      ↓
Frontend: import { calcSMA, calcKAMA } from './pkg/indicators.js'
```

**Key decisions:**
- Share Rust indicator code between backend (backtester) and frontend (Wasm)
- Use `wasm-bindgen` for JS interop with typed arrays
- Bundle Wasm as a Vite asset (< 100KB gzipped for all indicators)

## Pine Script Compatibility Layer (NOT PLANNED)

### Why Not Planned
Pine Script is TradingView's proprietary language. Building a transpiler requires:
1. **Lexer + Parser** for Pine Script syntax (~5K lines of grammar)
2. **AST** representation of Pine constructs (series, security(), strategy.*)
3. **Code generator** to emit JS plugin format
4. **Runtime library** implementing Pine built-in functions (~200 functions)
5. **Testing** against hundreds of real Pine scripts

This is a standalone project (3-6 months) with limited value since:
- Users can manually port Pine logic to JS plugins
- The JS plugin system already supports the same capabilities
- Pine Script evolves frequently (v5, v6) requiring ongoing maintenance

### Alternative
Document a "Pine Script to JS Plugin" porting guide with examples:
```pine
// Pine Script
//@version=5
strategy("My Strategy")
fast = ta.sma(close, 10)
slow = ta.sma(close, 50)
if ta.crossover(fast, slow)
    strategy.entry("Long", strategy.long)
```
```javascript
// JS Plugin equivalent
export default {
  name: "My Strategy",
  params: { fast: 10, slow: 50 },
  pane: "overlay",
  calculate(data, params) {
    // Use built-in calcSMA from TyphooN-Terminal
    return calcSMA(data, params.fast);
  },
};
```
