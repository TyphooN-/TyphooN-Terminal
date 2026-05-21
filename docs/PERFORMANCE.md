# TyphooN Terminal — Performance

## Native GPU Renderer

The terminal uses egui + wgpu for direct GPU rendering. No WebView, no JavaScript, no IPC overhead. Native builds request continuous repaint and let wgpu/eframe VSync/adaptive sync cap presentation at the display refresh rate; `TYPHOON_IDLE_FPS` is an opt-in profiling/problem-display cap, not the default. Periodic broker/sync/metrics/MT5 maintenance is wall-clock gated rather than `frame_count` gated, so moving from idle repaint to 60/144/240Hz native refresh does not accidentally multiply backend/UI maintenance work. Tokio broker/feed workers scale to available CPU on AC/desktop while reserving one logical core for egui/wgpu; `TYPHOON_TOKIO_WORKERS` remains the explicit override.

### Benchmarks

| Metric | Value |
|--------|-------|
| Startup to interactive | < 2s (including SQLite cache load) |
| 10K bar chart render | < 5ms |
| 46+ indicators on 10K bars | < 15ms total |
| MTF grid (4 cells x 5K bars) | < 50ms |
| Chart zoom/pan | 60fps, zero frame drops |
| Memory (single chart + indicators) | ~50-80MB |
| Memory (MTF 4-cell grid) | ~100-150MB |
| Binary size (release) | ~25MB |

### Chart Rendering

Chart rendering keeps provider-depth history in memory/cache, but the draw path emits only the detail the current viewport can display. Dense visible ranges are decimated to roughly two samples per horizontal pixel before creating egui line/candle/OHLC/indicator/PSAR primitives, and grid/reference/AutoFib lines use fixed primitive counts instead of dotted per-pixel dash/circle spam. Overlay fills use the same sampling step and widen sampled spans, so clouds/ribbons/bands stay continuous without per-bar rectangle emission. Indicator sub-panes use the same render-only sampling for lines and histograms; histogram buckets preserve the strongest volume/MACD value, and calculations/trading/algo inputs still use the full underlying series. Dense market-structure/fractal overlays keep their context scan but enforce a minimum pixel gap between painted text labels, preventing overlapping HH/LH/HL/LL glyph storms. Fixed-size overlays avoid per-frame heap allocations where possible, e.g. harmonic XABCD screen points stay in arrays. This protects drag/zoom responsiveness when the user zooms out over very deep synced histories.

### Live Bar Builder

WebSocket trade streams build 1-minute OHLCV bars in-process. Completed bars use a bounded FIFO buffer (`VecDeque`) so live sessions cannot grow unbounded and old-bar eviction remains O(1) instead of draining/shifting a `Vec` under load.

### Data Pipeline

| Step | Time |
|------|------|
| SQLite read + zstd decompress | < 1ms |
| Bar struct construction | < 0.5ms |
| Indicator computation (all 46+) | < 15ms |
| egui Painter → wgpu surface | < 2ms |
| **Total: cache → pixels** | **< 20ms** |

### Why It's Fast

1. **Zero serialization** — data stays as Rust types from SQLite to GPU
2. **Immediate mode UI** — egui redraws only what changed, no DOM diffing
3. **Pre-computed indicators** — computed once on load, cached in ChartState
4. **Vulkan backend** — wgpu selects Vulkan on Linux (NVIDIA), Metal on macOS
5. **No garbage collection** — Rust ownership model, no GC pauses

### GPU DarwinIA Scan

Large DarwinIA datasets (>128MB) are processed via chunked batching in the GPU compute pipeline. The `compute_all_batches()` method splits return series into chunks that fit within wgpu buffer size limits, processes each chunk on the GPU, and merges the results. This enables scanning 50K+ DARWINs without exceeding VRAM constraints.

### Storage Compact (zstd-22)

Rust bar-cache writes now store packed TTBR blobs at zstd level 22 immediately, so new provider-depth sync data lands in final compressed form. The Storage Manager (`STORAGE` command) compact path remains for legacy/raw/imported bar_cache entries whose `zstd_level` metadata is below 22. Decompression speed is effectively unchanged by source compression level — only on-disk storage and encode time change. Progress is reported per 200 entries.

Auto-compact uses the same compaction path for leftovers, but only runs when the configured cadence/window, AC-power, idle, and min-row gates pass. Defaults are weekly Sunday 04:00-05:00 local and at least 100 uncompacted rows; the Storage Manager exposes those knobs plus last-run, next-window, skip-reason, and running-state readouts.

### Auto MT5 Sync

Bar data from MT5 (via BarCacheWriter EA) is automatically synced about once per minute when the cache is loaded and the terminal is not a LAN client. The sync is smart enough to skip unchanged keys.

### Kraken Public Bar Sync

Kraken Spot/xStocks and Futures public bars run on tokio tasks with a shared 16-permit public fetch semaphore. Direct Kraken requests spawn one task per timeframe, while the broad universe scheduler keeps Spot and Futures queue windows bounded with normalized pending/unresolvable/backfill-complete keys. Crypto/fiat quote filters are global broker settings; new sessions default to USD and USD stablecoin quotes (`USD`, `USDT`, `USDC`, `USDG`) rather than scraping every fiat-quoted crypto market, and future crypto brokers should reuse the same filter. Spot/xStocks OHLC HTTP calls are paced at the engine boundary to Kraken's documented public level: about one request per second process-wide and per pair, with 5s -> 60s cooldown on rate-limit responses. Spot/xStocks uses the full recent OHLC provider window because Kraken Spot's public endpoint is bounded; it is not a deep-history API. Kraken Futures uses explicit `from`/`to` chart ranges and first sync starts at the Futures historical floor, chunking forward until current before marking backfill complete. SQLite/zstd cache merge and write work is offloaded with `spawn_blocking`, so network tasks stay responsive and active charts can reload on `BarsFetched` before the terminal `FetchSettled` releases scheduler slots.

### Broker Full-History Sync

Alpaca, tastytrade, and Kraken Futures no longer use arbitrary local target depths such as 10k, 50k, 7.5k, or 3.5k bars. If the provider supports full historical traversal, first sync and incomplete-cache backfill continue until provider exhaustion and then persist a backfill-complete marker with the actual stored count. tastytrade uses DXLink Candle snapshot status: `SNAPSHOT_SNIP` pages forward from the last candle; only `SNAPSHOT_END` marks full-history complete. Kraken Spot remains recent-window-only by API design; deep crypto history belongs to CryptoCompare in the source hierarchy.

### Cache Format

Bar data stored in TTBR (TyphooN Terminal Binary Record) format:
- 6 x f64 per bar (timestamp_ms, open, high, low, close, volume) = 48 bytes/bar
- zstd level 22 compression for Rust bar-cache writes = minimum durable storage; decode speed remains chart-friendly
- 10,000 bars = ~100KB compressed on disk

### GPU Backend Selection

wgpu auto-selects the best available backend:
1. Vulkan (Linux/Windows, preferred)
2. Metal (macOS)
3. DX12 (Windows fallback)
4. OpenGL (legacy fallback)

The `Unrecognized present mode` warnings on Wayland are harmless — it's a Vulkan extension for adaptive VSync that wgpu doesn't yet handle. Filtered to error level in the log subscriber.
