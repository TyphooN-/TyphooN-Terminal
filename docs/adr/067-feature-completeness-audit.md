# ADR-067: Feature Completeness Audit

**Status:** Complete
**Date:** 2026-04-02 | **Updated:** 2026-05-05

## Audit Results

### Fully Implemented (Production Ready)
- **BrokerCmd/BrokerMsg dispatch** — all variants from the original audit were handled; the enums are now much larger after the TA-Lib + Godel parity rounds and remain routed through the broker task.
- **89 drawing tools** with live preview, OHLC snap, undo/redo, color picker, line width/style, selection hit-test (8px), Delete-to-remove-selected
- **LAN sync** — 34 KV-synced analytics fields, 15 remote commands wired, periodic incremental resync, TLS encryption
- **Multi-source bar loading** — MT5 → Alpaca → tastytrade → CryptoCompare → Kraken → Kraken Futures (6-source priority) with timezone-aware dedup
- **Supply/demand zones** — 1:1 MT5 parity (GPU + CPU paths, BACK_LIMIT=1000)
- **Crypto backfill** — CryptoCompare deep history + Kraken sub-hourly, skip-if-cached
- **BarCacheWriter v1.435** — TF gating, 16MB cache, /dev/shm ramdisk support
- **Prometheus metrics** — bar counts, positions, equity, cache stats
- **Security** — zero unsafe blocks, parameterized SQL, keyring credentials, TLS LAN sync
- **tastytrade** — Full REST API (auth, balances, positions, orders, quotes, market metrics, option chains) + DXLink WebSocket (historical bars via SETUP→AUTH→FEED protocol). Connect button active. See ADR-022.
- **MQL5 compiler** — All TODOs fixed (switch, do-while, assignment, local resolution, continue). 227 tests passing across parser, IR, WASM codegen, WGSL codegen, and the full 10-language transpiler matrix *(updated 2026-05-17 — was 82 at time of writing)*. See ADR-060, ADR-090, ADR-091, ADR-098.
- **PineScript v5 parser** — Compiles PineScript indicators to WASM via same IR pipeline. Supports indicator(), input.*, ta.*, plot(), math.*, built-in series. Part of the 216 compiler tests.
- **Position visibility toggles** — Per-broker hide/show (DARWIN, Alpaca, tastytrade) including orders
- **Session persistence** — Auto-save on window close (on_exit), restore on startup
- **Backfill coloring** — Magenta candles for non-primary data source bars
- **POSITION_CHARTS** — Command to open W1 tabs for all open positions

### Known Limitations
- DARWIN deal import on LAN client produces wrong positions → fixed by KV-sourced analytics (34 fields)
- Alpaca free tier: 15-min delayed quotes, 200 req/min rate limit → position polling throttled
- /dev/shm ramdisk: data doesn't survive reboot (BarCacheWriter re-exports ~5-10 min)
- PineScript parser covers a subset of PineScript v5 (common indicators, not full language)

## Post-Audit Fixes (2026-04-05)
- **Analyst Ratings window**: Now renders structured grid (Period/StrongBuy/Buy/Hold/Sell/StrongSell/Consensus) from cached Finnhub recommendations JSON, auto-opens on fetch completion
- **Institutional Holders window**: Now renders entity metadata (SIC, state, FY end) + 13F filing table from cached SEC EDGAR data, auto-opens on fetch completion
- **Orderbook DOM window**: New window (`show_orderbook_window`) renders real L2 bid/ask depth from `orderbook_result` — shows bid/ask ladders with volume bars, auto-opens when Fetch Depth or Fetch L2 returns data
- **BrokerMsg::JsonResult routing**: Labels prefixed "Analyst:", "Holders:", "Orderbook:" now route to respective result fields + auto-open windows instead of log-only
- **Drawing move/drag (Gap #2)**: Selected drawings can now be dragged to new positions — `is_drawing_drag` blocks chart pan when a drawing is selected; all 89 types covered
- **Option Chain window**: New `OPTION_CHAIN` command fetches tastytrade expirations and displays them in a collapsible-expiration grid with strike/call/put symbols
- **tastytrade OPTION_CHAIN command**: Added to command palette, loads from KV cache `tt:options:<symbol>`
- **Notifications**: `BrokerCmd::SendNotification` wired — Discord webhook, Pushover, ntfy push notifications fire on indicator alert trigger. Settings panel has config fields + test button. Credentials stored in system keyring.
- **ADR status fixes**: 4 ADRs promoted from "Accepted" to "Implemented" (053, 054, 055, 056). Drawing count corrected 73→70 across all ADRs.
- **MQL5/PineScript compiler UI**: New `COMPILE` command + Indicator Compiler window with source editor, Load File dialog, language selector (MQL5/PineScript), compile button, diagnostics display, and metadata summary (buffers, inputs, plots)
- **BarBuilder real-time streaming**: `BrokerCmd::StartStream` wired to `AlpacaBroker::start_stream()`. `StreamTick` feeds `BarBuilder` for 1-minute bar construction. Completed bars auto-append to matching chart tabs. `StreamQuoteTick` updates forming bar close price.
- **64 new tests**: bar_builder(7), martingale(14), notifications(13), mql5_export(7), lan_sync(9), fred(4), screener(10). Total: 537 tests.
- **3 darwin analytics wired**: performance_attribution (per-symbol P&L contribution), dscore_components (FTP 8-component grid), investment_velocity (investor growth rate chart) — all rendered in per-account detail section
- **Price target display**: Finnhub price target (high/median/low/mean + analyst count) appended to Analyst Ratings window

## Post-Audit Fixes (2026-04-04)
- **Replay mode**: `ChartState.replay_bar_cap` now actually caps visible_range so bars after replay_bar_idx are hidden
- **Tab drag-drop**: Fixed insert_at logic (right-half drop now inserts after, not at same position); drag_src saved before clearing
- **Drawing undo/redo**: drawing_styles Vec kept in sync across Delete, Ctrl+Z, Ctrl+Shift+Z
- **Drawing selection**: Click-to-select with 8px hit threshold for 6 common types; ESC/empty-click to deselect
- **Drawing render**: All 89 drawing types now use draw_line()/effective_width/sel_tint — width/style/selection finally active
- **Weekend bar coloring**: gap_fill_timestamps HashSet (explicit source tracking) replaces unreliable UTC day-of-week check
- **LAN KV sync poisoning**: server-side exclusion filter for lan:server_enabled/client_enabled/server_ip/sync_port/cred:*
- **Crypto backfill signals**: Mt5SyncDone sent after KrakenBackfill + CryptoCompareBackfill so charts reload automatically
- **LAN 15-min resync**: timeout forces reconnect to pick up new crypto/backfill bars without user intervention

## Consequences
- All user-facing features are production ready
- Deferred/data-gated items remain tracked outside this completeness audit: hot-reload indicator files, indicator import UI, and automatic classic chart-pattern recognition.
