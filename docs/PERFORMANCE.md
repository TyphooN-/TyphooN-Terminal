# TyphooN Terminal — Performance

## Native GPU Renderer

The terminal uses egui + wgpu for direct GPU rendering. No WebView, no JavaScript, no IPC overhead. Native builds request continuous repaint and let wgpu/eframe VSync/adaptive sync cap presentation at the display refresh rate; `TYPHOON_IDLE_FPS` is an opt-in profiling/problem-display cap, not the default. Periodic broker/sync/metrics maintenance is wall-clock gated rather than `frame_count` gated, so moving from idle repaint to 60/144/240Hz native refresh does not accidentally multiply backend/UI maintenance work. Tokio broker/feed workers scale to available CPU on AC/desktop while reserving one logical core for egui/wgpu; `TYPHOON_TOKIO_WORKERS` remains the explicit override.

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
2. **Immediate mode UI** — no DOM or retained widget tree; performance comes from bounded per-frame work, caching, and repaint throttling
3. **Pre-computed indicators** — computed once on load, cached in ChartState
4. **Vulkan backend** — wgpu selects Vulkan on Linux (NVIDIA), Metal on macOS
5. **No garbage collection** — Rust ownership model, no GC pauses
6. **Per-frame O(1) discipline** — per-frame paths (`update()`, `draw_chart()`,
   panel render closures) avoid allocations proportional to dataset size:
   invariant work is hoisted out of inner closures, suffix arrays replace
   per-candidate scans (e.g. FVG fill check), pre-normalized HashSets
   replace linear `iter().any` audits (Kraken pairs lookup, balance ownership,
   position asset tails), index-aligned data is paired by `zip()` not by key
   search, fixed-cardinality enums use bitset membership, and per-frame
   `trade_overlay` borrows use `std::mem::take` + restore instead of `Clone`.
   Companion maps/sets (`watchlist_by_bare`, `chart_by_bare`, `roster_by_id`,
   `kraken_equity_pair_by_base`, primary roster caches) and one-pass selection
   (regulatory alerts, rosters, MTF open tabs) eliminate repeated lookups and
   re-scans. Binary search (`partition_point`) for sorted research windows.
   See ADR-098 for the full ongoing O(1) optimization program.

### Storage Compact (zstd-22)

Rust bar-cache writes store packed TTBR blobs at a configurable base zstd level (default 3; range 1-22) selected in Storage Manager. Lower levels keep broad sync/import writes CPU-cheap; higher levels shrink new blobs immediately. The Storage Manager (`STORAGE` command) compact path remains the archival promotion path for bar_cache entries whose `zstd_level` metadata is below 22, including configured-base writes, legacy/raw/imported rows, and Kraken WS hot writes. Decompression speed is effectively unchanged by source compression level — only on-disk storage and encode time change. Progress is reported per 200 entries.

Auto-compact uses the same compaction path for leftovers, but only runs when the configured cadence/window, AC-power, idle, and min-row gates pass. Defaults are daily 04:00-05:00 local and at least 100 uncompacted rows; the Storage Manager exposes those knobs plus last-run, next-window, skip-reason, and running-state readouts.

### Kraken Public Bar Sync

Kraken has three separate public bar lanes with different performance contracts:

- **Spot REST + WS:** Spot uses the full public `AssetPairs` catalog subject to the global crypto/fiat quote filters. New sessions default to USD and USD stablecoin quotes (`USD`, `USDT`, `USDC`, `USDG`) instead of scraping every fiat-quoted crypto market. REST OHLC remains the cold-start/recent-window source and is paced at the engine boundary to Kraken's documented public level: about one request per second process-wide and per pair, with 5s -> 60s cooldown on rate-limit responses. The optional/recommended Spot OHLC WebSocket lane streams the full WS-mappable Spot catalog on every WS-served interval; it is not narrowed to open charts/watchlist/positions. WS writes are coalesced until bar close, flushed off the egui frame thread, written through the fast zstd-3 merge path, and later promoted by normal zstd-22 compaction. See ADR-099.
- **Kraken Securities / xStocks iapi:** Securities bars use the separate `kraken-equities:*` namespace and the Kraken iapi AIMD limiter. Native high timeframes (`1Day`, `1Week`, `1Month`) target the loaded Kraken equities catalog. Native intraday remains demand/focus scoped unless iapi throughput proves broad native intraday safe. Provider-assist rows (`alpaca:*`, `yahoo-chart:*`) are separate cache namespaces and can supply broad `15Min`+ chart-usable fallback coverage when enabled. Open-position quotes are not broad sync: they are a foreground safety lane with a hard sub-minute target, preferably updated by an enabled quote WebSocket and otherwise by the freshest bounded REST/iapi fallback available. See ADR-101, ADR-102, and ADR-103.
- **Kraken Futures:** Futures uses public instrument discovery and explicit `from`/`to` chart ranges under `kraken-futures:*`; first sync starts at the Futures historical floor, chunks forward until current, then marks backfill complete.

Direct Kraken requests spawn per-timeframe tasks where applicable, while the broad schedulers keep queue windows bounded with normalized pending/unresolvable/backfill-complete keys. SQLite/zstd cache merge and write work is offloaded with `spawn_blocking`, so network tasks stay responsive and active charts can reload on `BarsFetched` before the terminal `FetchSettled` releases scheduler slots.

### Broker Full-History Sync

Alpaca and Kraken Futures no longer use arbitrary local target depths such as 10k, 50k, 7.5k, or 3.5k bars. If the provider supports full historical traversal, first sync and incomplete-cache backfill continue until provider exhaustion and then persist a backfill-complete marker with the actual stored count. Kraken Spot remains recent-window-only by API design; deeper equity history is supplied by the Yahoo corroborator where available (ADR-113).

### Cache Format

Bar data stored in TTBR (TyphooN Terminal Binary Record) format:
- 6 x f64 per bar (timestamp_ms, open, high, low, close, volume) = 48 bytes/bar
- configurable zstd compression for Rust bar-cache writes (default 3, Storage Manager range 1-22); zstd-22 compaction is the archival target; decode speed remains chart-friendly
- 10,000 bars = ~100KB compressed on disk

### GPU Backend Selection

wgpu auto-selects the best available backend:
1. Vulkan (Linux/Windows, preferred)
2. Metal (macOS)
3. DX12 (Windows fallback)
4. OpenGL (legacy fallback)

The `Unrecognized present mode` warnings on Wayland are harmless — it's a Vulkan extension for adaptive VSync that wgpu doesn't yet handle. Filtered to error level in the log subscriber.
