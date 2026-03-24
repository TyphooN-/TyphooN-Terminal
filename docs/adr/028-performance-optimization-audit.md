# ADR-028: Performance Optimization Audit — March 2026

**Status:** Implemented
**Date:** 2026-03-18

## Context

Full-scope codebase audit across security, performance, storage, and rendering. This ADR records all optimizations implemented and remaining work identified.

## Optimizations Implemented (This Pass)

### 1. SQLite Statement Caching
- Changed `conn.prepare()` → `conn.prepare_cached()` in all read methods
- Avoids re-parsing SQL on repeated cache lookups (every 10s poll)
- File: `core/cache.rs`

### 2. SQLite Auto-Vacuum
- Added `PRAGMA auto_vacuum=INCREMENTAL`
- Prevents DB file from growing unbounded as entries are evicted
- File: `core/cache.rs`

### 3. HTTP Connection Pooling
- Added `pool_max_idle_per_host(5)` and `tcp_keepalive(30s)` to reqwest Client
- Reuses TCP connections across chunk fetches (17 chunks for BTC/USD 4H = 17 connections → 1)
- File: `broker/alpaca.rs`

### 4. Dashboard DOM Delta Updates
- `setText()` now checks `el.textContent !== text` before writing
- `setTextClass()` checks both text and className before DOM mutation
- Eliminates ~20 unnecessary DOM writes per 2-second dashboard cycle
- File: `frontend/src/main.js`

### 5. Dashboard Overlap Prevention
- Added `_dashboardInFlight` guard to prevent concurrent `updateDashboard()` calls
- Previous behavior: if an API call took >2s, two dashboard updates could run simultaneously
- File: `frontend/src/main.js`

### 6. Positions/Orders Panel Atomic Swap
- Replaced `content.textContent = ""` + element-by-element append with DocumentFragment + `replaceChildren()`
- Orders panel: parallelized `get_open_orders` + `get_order_history` with `Promise.all()`
- Eliminates visual flicker during 2-second update cycle
- File: `frontend/src/main.js`

### 7. Indicator Error Isolation
- Added try/catch per indicator in `applyIndicators()` loop
- One indicator failure no longer breaks all remaining indicators
- Failures logged to console for debugging
- File: `frontend/src/main.js`

### 8. Custom Timeframe Rank Resolution
- Added `getTFRank()` function that parses custom TFs (2Day → rank 6.5, 3Hour → rank 4.5)
- Previous bug: D2 chart defaulted to rank 3 (30Min), showing wrong HTF indicators
- Fixes indicator rendering on all custom timeframes
- File: `frontend/src/main.js`

### 9. ATR Projection MQL5 Parity
- Current TF: replaced per-bar moving line with fixed horizontal lines at lastOpen ± ATR
- HTF: extended line span to match MQL5 lookbacks (D1: 7 bars, W1: 4 bars, H4: 11 bars)
- Changed to dotted line style (STYLE_DOT) matching MQL5
- File: `frontend/src/main.js`

### 10. PreviousCandleLevels MQL5 Parity
- Removed incorrect current-TF per-bar stepped line
- Fixed TF filtering to match MQL5 (H1 chart shows D1/W1 only, not H4)
- File: `frontend/src/main.js`

### 11. Fisher Transform Last-Bar Fix
- Fixed last segment with 1 data point being skipped
- Ensures Fisher line renders to the current candle on all timeframes
- File: `frontend/src/main.js`

### 12. Crypto Feed Label
- Changed `feed=None` → `feed=crypto` in log messages
- Clarifies that crypto uses Alpaca's dedicated crypto bars endpoint
- File: `broker/alpaca.rs`

## Previously Implemented Optimizations (Historical)

| Optimization | When | Impact |
|---|---|---|
| Binary bar format (48 bytes/bar) | v0.1 | 3-5x smaller storage vs JSON |
| zstd compression (level 3) | v0.1 | Additional 3-5x compression |
| SQLite WAL mode | v0.1 | Concurrent reads + writes |
| SQLite mmap (256MB) | v0.1 | OS-managed page caching |
| In-memory LRU (200 entries) | v0.1 | Prevents OOM on large sessions |
| Background bar prefetch | v0.1 | All TFs cached after first load |
| Centralized rate limiter | v0.1 | Zero 429 errors |
| 429 cooldown (60s auto-backoff) | v0.1 | Graceful degradation |
| Wasm indicator engine (32KB) | v0.1 | 20-100x faster calculations |
| GPU chart renderer (45KB) | v0.1 | WebGL2 candlesticks at 60fps |
| Race condition guards (ADR-024) | v0.1 | 7 cross-symbol bugs fixed |
| AES-256-GCM credentials | v0.1 | Machine-specific encryption |
| CSP + no innerHTML | v0.1 | XSS prevention |

## Remaining Work

### All Previously Identified Items — Complete

1. ✅ **Route chart indicators through Wasm** — 15 call sites, SMA/EMA/KAMA/RSI/ATR (10-20x faster)
2. ✅ **GPU indicator overlays (Phase 4)** — SMA/EMA/KAMA/Bollinger via WebGL2 LINE_STRIP shaders
3. ✅ **Web Worker for indicators** — `indicator-worker.js` with Wasm support, off-main-thread
4. ✅ **Full GPU chart engine (Phase 5)** — price scale, time axis, crosshair + OHLC tooltip (52KB Wasm)

### Post-Audit Optimizations (Implemented Since)

5. ✅ **`get_bars_tail()`** — tail-only JSON conversion for MT5 deep caches (34x faster for 50K→500 bars). See [PERFORMANCE.md](../PERFORMANCE.md).
6. ✅ **Indicator bar cap (1000)** — indicator computation limited to last 1000 bars, prevents O(N) scaling with deep history
7. ✅ **MT5 sync no-reload** — background sync updates MTF data without full chart rebuild

### Phase 2 Optimizations (2026-03-24, see ADR-046)

8. ✅ **Async indicator pipeline** — `applyIndicators()` converted to async with yield points between expensive indicators, generation-based cancellation
9. ✅ **Indicator memoization** — cache keyed by (indicator, period, barCount, lastBarTime, lastClose), 200-entry cap
10. ✅ **Fast timestamp parser** — `fastParseTimestamp()` hand-rolled ISO parser, 10x faster than `new Date()`
11. ✅ **Optimized bar sanitization** — forward-pass Map dedup, arithmetic weekend detection, sort-check
12. ✅ **GPU histogram + fill rendering** — rebuilt WASM with `add_histogram()`, `add_fill()`, `add_pane_histogram()`
13. ✅ **Parallel grid data prefetch** — `Promise.all` pre-fetch, sequential render with memoized indicators
14. ✅ **Binary search data clipping** — `sliceFrom()` and `clip()` use O(log n) binary search
15. ✅ **Crosshair O(1) lookup** — `_timeToBarMap` Map replaces O(n) `.find()`, cached container dims
16. ✅ **Debounced ResizeObserver** — batched via `requestAnimationFrame`
17. ✅ **MT5 badge race condition** — `mt5SyncActive` flag set before first sync completes

### Blocked by External Dependencies

1. **WebSocket bar aggregation** — Alpaca WS provides raw trades/quotes but not aggregated bars. We'd need to aggregate ourselves, adding complexity.
2. **Batch symbol API** — Alpaca doesn't support fetching bars for multiple symbols in one request.
3. **Larger page sizes** — IEX feed caps at ~260 bars/chunk. SIP feed (paid) may allow larger pages.

## Consequences

- All free-tier optimizations are now implemented
- Remaining gains require paid APIs
- Performance bottleneck is Alpaca API response time (~300ms/request), not our code
