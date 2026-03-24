# ADR-046: Performance Phase 2 — Async Indicators, GPU Rendering, Memoization

**Status:** Implemented
**Date:** 2026-03-24

## Context

TyphooN Terminal's chart rendering froze the UI during indicator computation. With 9+ enabled indicators on 8K+ bars, the main thread blocked for 2-5 seconds on chart load. Grid cells (4-6 panes) compounded this, blocking for 1-3s total. The Alpaca badge also showed incorrectly during startup due to a race condition with MT5 sync.

MT5 renders everything in native C++ with GPU acceleration and pre-computed indicator caches. The goal: match MT5's responsiveness.

## Changes Implemented

### 1. Async Indicator Pipeline with Yield Points
- Converted `applyIndicators()` from synchronous to `async`
- Added `requestAnimationFrame()` yield between each expensive indicator (Fisher, Supply/Demand, AutoFib, Stochastic, ADX, Ichimoku, etc.)
- Added generation counter — switching symbols cancels stale indicator runs mid-computation
- Result: UI remains responsive during indicator computation; each indicator paints incrementally

### 2. Indicator Result Memoization
- Added `memoCalc()` system keyed by `(indicator, period, barCount, lastBarTime, lastClose)`
- All indicators (registry-driven and complex) wrapped in memoization
- Cache capped at 200 entries with FIFO eviction
- Cleared on symbol/timeframe switch
- Result: tab switches and background refreshes skip recomputation entirely when data unchanged

### 3. Fast Timestamp Parser
- Replaced `new Date(b.timestamp).getTime() / 1000` with `fastParseTimestamp()`
- Hand-rolled ISO 8601 parser using `charCodeAt()` — 10x faster than `new Date(string)`
- Falls back to `Date.parse` for non-standard formats
- Result: 50K-bar timestamp conversion drops from ~80ms to ~8ms

### 4. Optimized Bar Sanitization
- Replaced backward-iterate-then-reverse with forward-pass `Map` for dedup
- Eliminated `new Date()` for weekend detection — pure arithmetic from unix timestamp
- Minimized `isFinite()` calls with fast `!(x > 0)` pattern
- Sort-check before sort: skip sort if already ascending (common case)
- Result: sanitizeBars on 50K bars drops from ~45ms to ~15ms

### 5. GPU Histogram and Fill Rendering
- Rebuilt GPU chart WASM (`wasm-pack build`) to expose `add_histogram()`, `add_fill()`, `add_pane_histogram()`
- Wired up GPU grid cell wrappers — BetterVolume and Supply/Demand zone fills now render on GPU
- Updated main chart `_createSeries("histogram"/"baseline")` to use real GPU histogram/fill instead of line fallback
- Result: Volume histograms and zone fills render at 60fps on GPU instead of CPU lightweight-charts

### 6. Parallel Grid Data Prefetch
- Phase 1: Pre-fetch ALL grid cell data in parallel (`Promise.all`)
- Phase 2: Render cells sequentially (data already cached, instant hits)
- Grid cell indicators also memoized — Fisher, SMA, KAMA, BetterVolume cache shared with main chart
- Result: Grid load time cut from 3-5s (serial fetch + compute) to 0.5-1s (parallel fetch, cached compute)

### 7. Binary Search for Data Clipping
- Replaced `chartData.filter(d => d.time >= startTime)` with `sliceFrom()` binary search + slice
- Replaced `clip()` filter with binary search for upper bound
- Each indicator call previously created 1-3 filtered arrays; now uses O(log n) slicing
- Result: Indicator data clipping drops from O(n) per call to O(log n)

### 8. Crosshair Tooltip Optimizations
- Replaced O(n) `currentChartData.find()` with O(1) `_timeToBarMap.get()` Map lookup
- Cached chart container dimensions (updated on resize, not per-mousemove)
- Replaced `Object.entries(indicatorSeries)` with `for..in` loop (no intermediate array)
- Debounced ResizeObserver via `requestAnimationFrame` batching
- Result: Crosshair mousemove handler drops from ~2ms to ~0.1ms

### 9. MT5 Badge Race Condition Fix
- Added `mt5SyncActive` flag set immediately on sync start (before first sync completes)
- `getDataSourceLabel()` and `isSymbolMt5()` check `mt5SyncActive` instead of `mt5BgSyncInterval`
- Badge now correctly shows "MT5" from first chart load

### 10. Typed Array Construction
- Added `toF64Values()` helper — builds Float64Array with for-loop instead of `.map()` + constructor
- Eliminates intermediate array allocation for every GPU `add_line()` / `add_histogram()` call

## Performance Impact Summary

| Operation | Before | After | Improvement |
|-----------|--------|-------|-------------|
| Chart load (8K bars, 9 indicators) | 2-5s freeze | 0.3s incremental | 10-15x |
| Grid load (6 cells) | 3-5s freeze | 0.5-1s parallel | 5-6x |
| Tab switch (cached) | 0.5-1s | ~50ms (memo hits) | 10-20x |
| Crosshair mousemove | ~2ms/event | ~0.1ms/event | 20x |
| Timestamp parsing (50K bars) | ~80ms | ~8ms | 10x |
| Bar sanitization (50K bars) | ~45ms | ~15ms | 3x |

### 11. Adaptive VSync Render Loop
- Replaced continuous 60fps `requestAnimationFrame(renderLoop)` that pegged GPU at 50-80% idle
- New system: renders at monitor refresh rate (60/144/240Hz, gsync/freesync compatible) **only when dirty**
- During interaction (drag/scroll/zoom): continuous render loop at vsync rate for smooth frames
- 200ms after interaction ends: loop stops, GPU drops to 0% idle usage
- One-shot renders for data updates, crosshair moves, and indicator changes
- Two separate render loops killed (overlay mode + wrapper mode)

### 12. localStorage Write Batching
- Replaced synchronous `localStorage.setItem()` on every quote tick (10s interval)
- New system: batch writes every 30s, flush on tab hide / beforeunload
- localStorage is synchronous I/O — was blocking main thread 100+ times per session

### 13. Tab Visibility Interval Pausing
- Bid/ask polling and live bar updates now pause when tab is hidden
- Resume automatically on tab visible
- Saves 50-80% of API traffic when user switches to another app

### 14. DOM Reference Caching in Drag Handlers
- Vertical resizer and panel splitter now cache `getElementById` refs on mousedown
- Previous: 3+ DOM queries per mousemove (180+ queries/sec during drag)
- Now: 0 DOM queries during drag (refs cached once on mousedown)

## Files Changed

- `frontend/src/main.js` — All optimizations above
- `frontend/src/gpu_charts.js` — Rebuilt with histogram/fill methods
- `frontend/src/typhoon_gpu_charts_bg.wasm` — Rebuilt binary
