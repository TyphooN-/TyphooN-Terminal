# TyphooN Terminal — Performance

## Native GPU Renderer

The terminal uses egui + wgpu for direct GPU rendering. No WebView or JavaScript is involved. Native builds request continuous repaint when normal and use a 16 ms repaint interval while heavy sync is active; `TYPHOON_IDLE_FPS` is an opt-in profiling/problem-display override. Periodic broker/sync/metrics maintenance is wall-clock gated rather than `frame_count` gated, so moving between repaint rates does not multiply maintenance work. Tokio broker/feed workers scale to available CPU on AC/desktop while reserving one logical core for egui/wgpu; `TYPHOON_TOKIO_WORKERS` remains the explicit override.

### Performance targets and telemetry

The repository does not currently enforce the old fixed `<2s`/`<5ms` benchmark table. Those numbers were misleading for a 31+ GB cache, full-catalog sync, large restored MTF sessions, and high-resolution multi-monitor rendering. Current performance is verified with phase-attributed runtime telemetry: `pre_broker_ms`, `broker_drain_ms`, `render_after_broker_ms`, chrome/floating-window subphases, `session_save_ms`, pending fetches, RSS, and system memory. A release workload is healthy when foreground frames remain responsive while bounded background queues continue to converge; absolute timings depend on cache depth, viewport size, enabled overlays, and provider payloads.

### Chart Rendering

Chart rendering keeps provider-depth history in memory/cache, but the draw path emits only the detail the current viewport can display. Dense visible ranges are decimated to roughly two samples per horizontal pixel before creating egui line/candle/OHLC/indicator/PSAR primitives, and grid/reference/AutoFib lines use fixed primitive counts instead of dotted per-pixel dash/circle spam. Overlay fills use the same sampling step and widen sampled spans, so clouds/ribbons/bands stay continuous without per-bar rectangle emission. Indicator sub-panes use the same render-only sampling for lines and histograms; histogram buckets preserve the strongest volume/MACD value, and calculations/trading/algo inputs still use the full underlying series. Dense market-structure/fractal overlays keep their context scan but enforce a minimum pixel gap between painted text labels, preventing overlapping HH/LH/HL/LL glyph storms. Fixed-size overlays avoid per-frame heap allocations where possible, e.g. harmonic XABCD screen points stay in arrays. This protects drag/zoom responsiveness when the user zooms out over very deep synced histories.

### Live Bar Builder

WebSocket trade streams build 1-minute OHLCV bars in-process. Completed bars use a bounded FIFO buffer (`VecDeque`) so live sessions cannot grow unbounded and old-bar eviction remains O(1) instead of draining/shifting a `Vec` under load.

### Data Pipeline

Cache-to-pixels timing is workload-dependent. Slow merged-cache loads are explicitly logged with symbol, timeframe, bar count, elapsed time, and RSS before/after; frame stalls are split by phase rather than hidden behind a single aspirational total.

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

- **Spot REST + WS:** Spot uses the full public `AssetPairs` catalog subject to the global crypto/fiat quote filters. New sessions default to USD and USD stablecoin quotes (`USD`, `USDT`, `USDC`, `USDG`) instead of scraping every fiat-quoted crypto market. REST OHLC remains the cold-start/recent-window source and is paced at the engine boundary to Kraken's documented public level: about one request per second process-wide and per pair, with 5s -> 60s cooldown on rate-limit responses. The optional/recommended Spot OHLC WebSocket lane streams the full WS-mappable Spot catalog on every WS-served interval; it is not narrowed to open charts/watchlist/positions. WS writes are coalesced until bar close, flushed off the egui frame thread, and persisted through `merge_bars_fast` at the configured zstd level. See ADR-089 and ADR-099.
- **Kraken Securities / xStocks iapi:** Securities bars use the separate `kraken-equities:*` namespace and the Kraken iapi AIMD limiter. iapi is a demand-depth repair lane for held, watched, and open-chart symbols across enabled timeframes; it does not sweep the ~13k catalog. Catalog breadth is supplied by bounded Kraken WS snapshot work plus Alpaca/Yahoo assist lanes and merged coverage. Provider-assist rows (`alpaca:*`, `yahoo-chart:*`) remain separate cache namespaces. Open-position quotes are a foreground safety lane, preferably updated by an enabled quote WebSocket and otherwise by the freshest bounded REST/iapi fallback. See ADR-101, ADR-102, ADR-103, and ADR-112.
- **Kraken Futures:** Futures uses public instrument discovery and explicit `from`/`to` chart ranges under `kraken-futures:*`; first sync starts at the Futures historical floor, chunks forward until current, then marks backfill complete.

Direct Kraken requests spawn per-timeframe tasks where applicable, while the broad schedulers keep queue windows bounded with normalized pending/unresolvable/backfill-complete keys. SQLite/zstd cache merge and write work is offloaded with `spawn_blocking`, so network tasks stay responsive and active charts can reload on `BarsFetched` before the terminal `FetchSettled` releases scheduler slots.

Broker-message draining is bounded by both message count and elapsed budget. Expensive broad refill work is coalesced after a drain, and saturated heavy-sync queues defer event-driven refill to the existing periodic scheduler until pending work drops below the high-water mark. The background analytics/cache snapshot channel is capacity one and published with `try_send`; a stalled UI cannot retain a new multi-GB `BgData` clone every refresh cycle. Session persistence has one owner: the incremental saver snapshots on the UI thread only when heavy sync is inactive, then writes session JSON and sync preferences on a blocking worker. The separate 60-second credential safety-net does not rebuild session state.

Alpaca multi-account sync scales at the dispatch boundary, not by duplicating
scheduler work. Each single-symbol request or complete batch is assigned
round-robin to one successfully connected account with its own limiter; all
results merge into the same canonical cache keys. Aggregate RPM and scheduler
capacity use the connected rotation count. Primary selection controls trading
and account state only. A failed request settles/retries normally; a later retry
re-enters round-robin and may use another account rather than migrating the
already-running request.

Broad sync is memory-aware without changing universe semantics. The runtime reads installed RAM from `/proc/meminfo` and scales broad queue windows, batch sizes, Alpaca full-tilt capacity, and Yahoo/Kraken HTTP semaphore permits on smaller machines (35% at <=24 GB, 50% at <=40 GB, 75% at <=64 GB, with foreground-safe floors). Process RSS and system available/total memory are included in UI-stall diagnostics. Memory pressure pauses background expansion before it starves the foreground, but it does not collapse Kraken Spot/Securities/xStocks from full-catalog coverage to active-only.

### Broker Full-History Sync

Alpaca and Kraken Futures no longer use arbitrary local target depths such as 10k, 50k, 7.5k, or 3.5k bars. If the provider supports full historical traversal, first sync and incomplete-cache backfill continue until provider exhaustion and then persist a backfill-complete marker with the actual stored count. Kraken Spot remains recent-window-only by API design; deeper equity history is supplied by the Yahoo corroborator where available (ADR-113).

Alpaca rate-limit and no-data outcomes are owned by scheduler state instead of noisy user-facing logs: provider no-data becomes a tombstone, 429/rate-limit responses enqueue retries and pause broad/background Alpaca scheduling until the backoff expires, and successful writes clear the consecutive-rate-limit state. Sync Status shows the active Alpaca pause when present.

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
