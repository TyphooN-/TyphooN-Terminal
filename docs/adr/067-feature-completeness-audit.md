# ADR-067: Feature Completeness Audit

**Status:** Complete
**Date:** 2026-04-02 | **Updated:** 2026-04-04

## Audit Results

### Fully Implemented (Production Ready)
- **41 BrokerCmd variants** — all handled
- **21 BrokerMsg variants** — all sent/received
- **73 drawing tools** with live preview, OHLC snap, undo/redo, color picker, line width/style, selection hit-test (8px), Delete-to-remove-selected
- **LAN sync** — 34 KV-synced analytics fields, 14 remote commands wired, 15s incremental resync, TLS encryption
- **Multi-source bar loading** — MT5 → Alpaca → tastytrade → CryptoCompare → Kraken (5-source priority) with timezone-aware dedup
- **Supply/demand zones** — 1:1 MT5 parity (GPU + CPU paths, BACK_LIMIT=1000)
- **Crypto backfill** — CryptoCompare deep history + Kraken sub-hourly, skip-if-cached
- **BarCacheWriter v1.435** — TF gating, 16MB cache, /dev/shm ramdisk support
- **Prometheus metrics** — bar counts, positions, equity, cache stats
- **Security** — zero unsafe blocks, parameterized SQL, keyring credentials, TLS LAN sync
- **tastytrade** — Full REST API (auth, balances, positions, orders, quotes, market metrics, option chains) + DXLink WebSocket (historical bars via SETUP→AUTH→FEED protocol). Connect button active. See ADR-022.
- **MQL5 compiler** — All TODOs fixed (switch, do-while, assignment, local resolution, continue). 82 tests passing across parser, WASM codegen, WGSL codegen. See ADR-060.
- **PineScript v5 parser** — Compiles PineScript indicators to WASM via same IR pipeline. Supports indicator(), input.*, ta.*, plot(), math.*, built-in series. 7 dedicated tests.
- **Position visibility toggles** — Per-broker hide/show (DARWIN, Alpaca, tastytrade) including orders
- **Session persistence** — Auto-save on window close (on_exit), restore on startup
- **Backfill coloring** — Magenta candles for non-primary data source bars
- **POSITION_CHARTS** — Command to open W1 tabs for all open positions

### Known Limitations
- DARWIN deal import on LAN client produces wrong positions → fixed by KV-sourced analytics (34 fields)
- Alpaca free tier: 15-min delayed quotes, 200 req/min rate limit → position polling throttled
- /dev/shm ramdisk: data doesn't survive reboot (BarCacheWriter re-exports ~5-10 min)
- PineScript parser covers a subset of PineScript v5 (common indicators, not full language)

## Post-Audit Fixes (2026-04-04)
- **Replay mode**: `ChartState.replay_bar_cap` now actually caps visible_range so bars after replay_bar_idx are hidden
- **Tab drag-drop**: Fixed insert_at logic (right-half drop now inserts after, not at same position); drag_src saved before clearing
- **Drawing undo/redo**: drawing_styles Vec kept in sync across Delete, Ctrl+Z, Ctrl+Shift+Z
- **Drawing selection**: Click-to-select with 8px hit threshold for 6 common types; ESC/empty-click to deselect
- **Drawing render**: All 73 drawing types now use draw_line()/effective_width/sel_tint — width/style/selection finally active
- **Weekend bar coloring**: gap_fill_timestamps HashSet (explicit source tracking) replaces unreliable UTC day-of-week check
- **LAN KV sync poisoning**: server-side exclusion filter for lan:server_enabled/client_enabled/server_ip/sync_port/cred:*
- **Crypto backfill signals**: Mt5SyncDone sent after KrakenBackfill + CryptoCompareBackfill so charts reload automatically
- **LAN 15-min resync**: timeout forces reconnect to pick up new crypto/backfill bars without user intervention

## Consequences
- All user-facing features are production ready
- Future: hot-reload indicator files, indicator import UI, chart pattern recognition
