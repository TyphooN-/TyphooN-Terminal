# ADR-046: Data Pipeline Optimization — GPU-First, Zero-Copy Architecture

## Status: Accepted (2026-03-24)

## Context

Profiling the bar data pipeline from backend SQLite cache through frontend rendering revealed multiple redundant allocations, double JSON parsing, and CPU-bound transforms that blocked the main thread for 3-20s during MTF grid loading (8 cells × 10K bars = 80K bars total). The GPU chart engine (WebGL2 WASM) renders fast once data reaches VRAM, but the JS-side pipeline created 3 copies of every bar before GPU upload.

## Previous Pipeline (Per Cell)

```
Backend SQLite → zstd decompress → binary unpack → JSON serialize (Rust)
  → Tauri IPC (JSON string)
    → JSON.parse #1 (cachedGetBars cache) → return JSON string
      → JSON.parse #2 (caller) → bars[]
        → .map(b => ({time, open, ...})) → chartBars[] (intermediate)
          → sanitizeBars(chartBars) → cleanBars[] (3rd copy)
            → packBarsForWasm(cleanBars) → Float64Array
              → GPU set_data(Float64Array)
```

**Cost per cell (10K bars):** ~500KB JSON parse + 3 × 10K object allocations + Float64Array construction = ~50-200ms main thread blocking.

## Optimized Pipeline

### 1. Single JSON.parse — `cachedGetBars` Returns Parsed Array

`cachedGetBars()` now returns the parsed bars array directly (not a JSON string). All callers use `typeof` guard for backward compatibility:

```javascript
// Before: const barsJson = await cachedGetBars(sym, tf, limit);
//         const bars = JSON.parse(barsJson);  // DOUBLE PARSE
// After:  const bars = await cachedGetBars(sym, tf, limit);  // Already parsed
```

### 2. Merged Parse + Sanitize — `parseSanitizeBars()`

Single-pass function replaces the `.map()` → `sanitizeBars()` chain. Eliminates the intermediate array allocation:

```javascript
// Before: sanitizeBars(bars.map(b => ({time: parse(b.timestamp), ...})))
//         = 2 × 10K object allocations
// After:  parseSanitizeBars(bars)
//         = 1 × 10K allocations (directly into dedup Map)
```

### 3. Adaptive MT5 Sync (Non-Blocking)

- **100 entries/cycle** (~2s compress+write)
- **10s interval** during catch-up, **30s** steady-state
- **10s startup delay** — grid bar requests get priority
- Heavy UI work (MTF reload) only when fully caught up
- `deferred` count in response JSON enables adaptive interval switching

### 4. Async Interval Gates

All 7 async `setInterval` handlers gated with `_running` flags to prevent queue buildup:
- WS bar poll, DARWIN watcher, MT5 sync, tape, anomaly, option flow, stream poll

### 5. Performance Fixes

- `Math.min.apply()` → loop-based min/max (6 locations, prevents stack overflow)
- SEC scan: sequential → 5-concurrent `Promise.allSettled` batches
- MT5 symbol filter: 3-pass `.filter()` → single-pass reduce
- `requestAnimationFrame` yield before heavy DARWIN renders

### 6. GPU Analytics (GpuMiniChart)

`GpuMiniChart` wrapper class for non-OHLC GPU rendering. `set_line_data()` in Rust avoids 5x JS packing overhead. Migrated 7 Canvas 2D renderers to GPU WebGL2.

## Data Flow (Optimized)

```
Backend SQLite → zstd decompress → binary unpack → JSON serialize (Rust)
  → Tauri IPC (JSON string)
    → JSON.parse (single, in cachedGetBars) → bars[]
      → parseSanitizeBars(bars) → cleanBars[] (single allocation)
        → packBarsForWasm(cleanBars) → Float64Array (cached by reference)
          → GPU set_data(Float64Array) → VRAM
```

**Savings:** Eliminated 1 JSON.parse + 1 full array allocation per data load. For 8 grid cells × 10K bars = ~80ms saved on main thread per grid open.

## Future: Binary Bar Transfer (Eliminates JSON Entirely)

The backend already stores bars in binary TTBR format (48 bytes/bar). A future `get_bars_binary` Tauri command could return `Vec<u8>` directly:

```
Backend SQLite → zstd decompress → TTBR binary
  → Tauri IPC (ArrayBuffer)
    → Float64Array view (zero-copy)
      → GPU set_data(Float64Array) → VRAM
```

This would eliminate JSON serialization (~20-50ms for 10K bars on Rust side) and JSON.parse (~10-30ms on JS side) entirely. Requires:
1. Rust: New command returning `Vec<u8>` with OHLCV layout matching GPU engine's `[O,H,L,C,V,...]`
2. JS: `new Float64Array(arrayBuffer)` directly to GPU, skip object allocation entirely
3. Timestamp handling: Separate channel or embedded in first 8 bytes per bar

## Metrics

| Metric | Before | After |
|--------|--------|-------|
| JSON.parse per bar load | 2 | 1 |
| Object allocations per load | 3 × N bars | 1 × N bars |
| Grid cell yield | 0ms | 200ms |
| MT5 sync initial delay | 0s | 10s (grid-first) |
| MT5 sync catch-up interval | 30s | 10s |
| SEC scan (50 symbols) | ~10s sequential | ~5s batched |
| Canvas 2D renderers → GPU | 0 | 7 |
| Async interval gates | 0 | 7 |

## Consequences

- `cachedGetBars` returns parsed array — callers that stored result as "Json" variable now hold arrays (backward-compatible via `typeof` guard)
- `parseSanitizeBars` duplicates sanitize logic — must keep in sync with `sanitizeBars` if that function changes
- MT5 sync 10s startup delay means grid data may be slightly stale on first render (offset by subsequent syncs)
- Binary transfer future work requires coordinated Rust + JS changes
