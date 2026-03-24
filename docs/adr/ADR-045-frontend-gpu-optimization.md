# ADR-045: Frontend GPU & Performance Optimization

## Status: Accepted (2026-03-24)

## Context

The TyphooN-Terminal frontend renders financial charts via a custom WebGL2 GPU engine (Rust/WASM) with 40K+ lines of JavaScript handling indicators, MTF grid, real-time data, and 100+ trading commands. Performance profiling revealed main-thread blocking from indicator computation, excessive polling, DOM churn, and GPU data alignment issues.

## Decisions

### GPU Engine (Rust WASM)

1. **Auto-offset indicator alignment**: `add_line`, `add_fill`, `add_histogram`, `add_pane_line`, `add_pane_histogram` auto-compute `offset = total_bars - values.len()` to right-align indicator data with bar data. Eliminates manual offset tracking in JS.

2. **Y-axis zoom**: `zoom_price(factor, center_y)`, `pan_price(delta_fraction)`, `reset_price_scale()` with `manual_price_scale` flag that guards auto-scaling in `set_visible_range` and `update_last_bar`. MT5/TradingView-style drag on price scale.

3. **Renko safety**: Added `data_bar_count` field separate from `total_bars` to prevent Renko brick count from corrupting geometry builder array bounds. All `build_*_geometry` and `update_last_bar` use `data_bar_count`.

4. **Price line style updates**: `update_price_line_style(index, r, g, b, a, width)` for dynamic SL/TP color changes without recreation.

### Web Worker Offloading

5. **Batch indicator computation**: SMA, EMA, KAMA, RSI, Fisher, BetterVolume, ATR offloaded to Web Worker via `workerComputeBatch()`. Main thread uses cached results instead of blocking on `reg.calc()`. Saves 300-600ms per indicator apply cycle.

6. **Debounced applyIndicators**: 100ms debounce coalesces rapid calls from WS polling (2s) + API updates (10s). Grid cells bypass debounce for immediate rendering.

### Polling & Visibility

7. **Dashboard polling**: Reduced from 2s to 5s (margin/equity/positions don't need sub-second updates).
8. **MTF Grid live sync**: Throttled to max 1Hz (was called from 3 sources at 2-10s intervals).
9. **Visibility API**: Pauses dashboard, bid/ask, and live bar polling when window is hidden. Immediate refresh on return.

### DOM & Rendering

10. **Zero innerHTML**: All 57 innerHTML assignments converted to safe DOM helpers (`el()`, `span()`, `td()`, `theadRow()`, `textContent`). Eliminates XSS vectors from template literal interpolation.
11. **DOM reuse**: Risk calc panel reuses existing child divs instead of recreating on every update. Session save caches indicator checkbox NodeList refs.
12. **O(n) Bollinger**: Rolling sum/sumSq (Welford's method) replaces O(n·period) nested loop.

### MTF Grid

13. **Multi-symbol support**: Default "All Tabs" mode, grid layouts for 1-16 cells with dynamic class selection.
14. **GPU canvas sizing**: Force reflow (`void gridContainer.offsetHeight`) before reading dimensions. Unique canvas IDs per symbol+tf.
15. **Memory cleanup**: `gpuChart.free()` in `closeMTFGrid()` releases WASM/WebGL resources.
16. **10K bars/cell**: Yield between cells (not inside cell load) keeps UI responsive during progressive population.
17. **Hex color parsing**: Fixed `|| 0.5` fallback that treated RGB=0 as falsy (black → gray).

### Data Pipeline

18. **O(1) bar updates**: `series.update()` uses GPU `update_last_bar()` for same-bar updates instead of O(n) full re-send.
19. **Cached packBarsForWasm**: Array reference caching avoids redundant O(n) flattening on re-renders.
20. **Price fallback**: Memory cache (`_pendingPriceWrites`) checked before localStorage `JSON.parse` on error paths.
21. **Async drawing load**: Large drawing sets (>10KB) parsed via `queueMicrotask` to avoid blocking chart init.

### Security

22. **Plugin sandbox**: `new Function()` wrapped with `"use strict"` and shadowed globals (`window=undefined, document=undefined, globalThis=undefined, self=undefined`).

### Code Quality

23. **Zero TODO/FIXME/HACK markers** remaining.
24. **Zero console.log** outside the log system (2 system console.log in the log() function itself).
25. **Zero var declarations** — all converted to const/let.
26. **getActiveSymbols()** utility: returns grid symbols in MTF mode, single currentSymbol in chart mode.
27. **timeToCoordinate/coordinateToTime** implemented with binary search.

## Metrics

| Metric | Before | After |
|--------|--------|-------|
| innerHTML assignments | 57 | 0 |
| console.log (non-system) | 26 | 0 |
| TODO/FIXME markers | 19 | 0 |
| var declarations | 2 | 0 |
| Indicator apply (main thread) | 2-5s freeze | <200ms (worker offloaded) |
| Dashboard poll interval | 2s | 5s |
| MTF Grid bar limit | 500 (flat) | 10K (with yields) |
| GPU bar update | O(n) re-send | O(1) update_last_bar |
| Bollinger computation | O(n·period) | O(n) |

## Consequences

- GPU engine requires `data_bar_count` field — Renko mode changes `total_bars` but geometry builders use `data_bar_count`
- Worker batch adds ~50ms latency for initial indicator computation (offset by 300-600ms main thread savings)
- 10K bars/cell × 12 cells = 120K bars total — grid populates progressively over 2-3s instead of instant
- Zero innerHTML policy requires all new UI code to use `el()`/`span()`/`td()` helpers
