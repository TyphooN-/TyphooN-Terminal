# TyphooN Terminal — Architecture

## Overview

Pure Rust native GPU application. No JavaScript, no WebKit, no IPC serialization.

```
┌─────────────────────────────────────────────────┐
│  Native Window (egui + wgpu)                    │
│  ┌─────────────────────────────────────────────┐│
│  │ Chart Renderer (egui Painter)               ││
│  │ - Candle/HeikinAshi/Line/Bars/Renko         ││
│  │ - 46+ indicators (GPU + CPU fallback)        ││
│  │ - 89 drawing tools + harmonic patterns       ││
│  │ - DARWIN trade arrows + position lines       ││
│  │ - Sub-panes (Fisher, RSI, MACD, ADX, etc.)  ││
│  ├─────────────────────────────────────────────┤│
│  │ egui Panels                                 ││
│  │ - Console (~) with 260+ commands             ││
│  │ - Positions / Orders / TradingView Watchlist││
│  │ - Risk calculator, VaR, Margin monitor      ││
│  │ - DARWIN analytics (80+ engine functions)   ││
│  │ - SEC Filing Scanner + Insider Trades       ││
│  │ - Multi-source News & Research cache        ││
│  │ - Fundamentals (21 data sources)             ││
│  │ - Research packet (TA-Lib + Godel parity)   ││
│  │ - AI sessions (Claude / Gemini / Codex)     ││
│  │ - Backtest engine + optimizer (GPU)         ││
│  │ - MTF Grid (up to 16 chart viewports)       ││
│  │ - 54+ floating windows                      ││
│  └─────────────────────────────────────────────┘│
├─────────────────────────────────────────────────┤
│  Engine Library (typhoon-engine crate)          │
│  - AlpacaBroker (REST + WebSocket)              │
│  - TastytradeBroker (REST + DXLink WebSocket)   │
│  - KrakenBroker (Spot REST orders + private WS) │
│  - SqliteCache (TTBR binary, zstd compression)  │
│  - DarwinDB (80+ analytics, 100% wired)         │
│  - RiskEngine (VaR, TRIM, martingale)           │
│  - BacktestEngine (bar-by-bar, optimization)    │
│  - BarBuilder (WebSocket → OHLCV)               │
│  - LanSync (TLS, PBKDF2, 15 remote commands)    │
│  - Notifications (Discord, Pushover, ntfy,      │
│    Matrix)                                      │
└─────────────────────────────────────────────────┘
```

## Data Flow (Zero Serialization)

```
SQLite cache → zstd decompress → Vec<(i64,f64,f64,f64,f64,f64)>
  → Bar structs (zero-copy reinterpret)
    → Indicator computation (pure Rust on &[f64] slices)
      → egui Painter draws directly to wgpu surface
```

No JSON. No IPC. No garbage collection. Direct memory access from cache to GPU.

## Data Sources

| Priority | Source | Coverage |
|----------|--------|----------|
| 1 | MT5 via BarCacheWriter v1.435 | 895 symbols × 9 TFs, weekday authority (Darwinex). TF gating, 16MB cache, /dev/shm ramdisk. |
| 2 | Kraken Spot | Crypto trading (full Spot REST order surface + private WS ownTrades/openOrders), recent REST OHLCV, and full-catalog public OHLC WebSocket forward freshness under `kraken:SYMBOL:TF`. Async queueing with Spot public pacing/cooldown per ADR-095 and WS write-path controls per ADR-099. |
| 3 | Kraken Securities / xStocks | Tokenized-equity bars under `kraken-equities:SYMBOL:TF` via iapi AIMD. Native high timeframes are full-catalog; native intraday stays demand/focus scoped; enabled provider-assist rows provide separate chart-usable fallback coverage. Open-position quotes are foreground safety data: they must refresh ahead of broad history/research work, target sub-minute freshness, and use enabled quote WebSockets where available; Kraken Spot public WS is not xStocks realtime coverage. |
| 4 | Kraken Futures | Public futures instrument discovery and full chart-candle sync under `kraken-futures:SYMBOL:TF` using explicit from/to ranges. |
| 5 | CryptoCompare | Deep crypto history (BTC from 2010), 2000 bars/request, hourly+ TFs. |
| 6 | tastytrade DXLink | Real-time historical bars + quotes for funded accounts. |
| 7 | Alpaca | US equities + crypto, free IEX or paid SIP. Tier-autotuned sync (ADR-087) plus opt-in Kraken-equities assist-only mode. |

MT5 is a **view-only data source** — bar data flows in via the BarCacheWriter EA to SQLite cache. Trade execution flows through Alpaca / tastytrade / Kraken with MT5 EA semantics (partial close, close-all, cancel-exits-before-close — see ADR-085). DARWIN account analytics are imported via XLSX trade history exports. Alpaca auto-connects on startup if credentials are saved in the system keyring. tastytrade fully integrated: REST API (auth, positions, orders, quotes, market metrics, option chains) + DXLink WebSocket (historical bars). See ADR-018. Kraken supports public-OHLCV-only mode and authenticated Spot REST trading with full AddOrder parameters, batch orders, amend/edit, dead-man cancel, cancel-all, balances, orders, trades, ledgers, positions, and private WebSocket `ownTrades`/`openOrders` with reconnect/resubscribe (ADR-051). Kraken public bar sync is fully async at the task level: direct Spot/Futures fetches spawn per-timeframe tasks, Kraken Spot can opportunistically prepend CryptoCompare deep crypto backfill for enabled USD crypto symbols, and all Kraken HTTP work runs under bounded queue/semaphore control (ADR-094). Spot OHLC HTTP calls are paced to Kraken's documented public limit with cooldown on throttles (ADR-095), while the Spot OHLC WebSocket lane streams the full WS-mappable Spot catalog and persists only closed/coalesced bars through the fast off-thread merge path (ADR-099). Kraken Securities/iapi uses a separate persisted AIMD limiter that starts conservatively, ramps on clean traffic, halves on congestion/rate-limit responses, and defaults to a 5 req/s ceiling; power users can raise only that ceiling with `TYPHOON_KRAKEN_IAPI_AIMD_MAX_RATE`, but the discovered-ceiling/backoff logic remains active. Securities/xStocks fallback providers (`Alpaca`, `Yahoo Chart`) remain separate source namespaces and are merged only for chart/research usability when enabled. See ADR-021 for cross-source priority hierarchy; chart source order includes the implementation-specific `kraken-equities` and `default` fallbacks around the table above.

### Cross-broker history assist

Kraken equities fallback is the first implementation of a broader rule: enabled
brokers/sources may assist each other through a normalized instrument-identity
layer. A new broker should not force a full cold historical sync when existing
compatible bars already cover the same economic instrument. The selected broker
remains authoritative for execution and native health, while compatible history
from other sources is merged at read/render/research time with provenance. Native
cache namespaces stay separate, and visible symbol equality is only a candidate
match — wrappers, CFDs, ADRs, quote currencies, suffixes, data delays, and session
calendars require explicit mapping before histories can be combined.

News is a separate research cache, not a market-data universe mirror. Sync Status totals count `(symbol, timeframe)` bar-cache entries, while news scrapes count deduped symbols, so the numbers will not match one-for-one. Kraken equities Sync Status uses an explicit denominator: native `1Day`/`1Week`/`1Month` rows are full-catalog (`kraken_equity_universe_symbols`), native intraday rows remain demand-scoped unless iapi throughput proves broader native intraday safe, and fallback/merged rows can use full-catalog `15Min`+ coverage when an enabled provider supports that timeframe. Fallback rows (`Alpaca`, `Yahoo`) report provider assist coverage, not Kraken coverage; Yahoo 404/empty-result responses are normal no-data tombstones for symbols Yahoo cannot resolve, especially SPAC/unit-style Kraken Securities symbols. The News & Research window fetches the active symbol, currently visible MTF Grid symbols, or the configured source bulk news universe; Kraken bulk news candidates are derived from cached `kraken:*`, `kraken-equities:*`, and `kraken-futures:*` market-data keys when Kraken is enabled. `news/SYM: 0 articles fetched` means the enabled news providers returned no rows for that selected symbol, not that Kraken equities bar sync skipped the rest of the catalog. See ADR-078 and ADR-102.

The `SYM` / `SYMBOLS` command opens Symbol Explorer. It is the catalog-facing way to inspect large universes instead of inferring coverage from runtime logs: cached rows are grouped by source/timeframe, broker-universe rows are grouped by rough asset category, the filter matches symbol/name text, and rows can load a chart, add to watchlist, or request LAN-client sync. It currently shows catalog/cache presence and fundamentals-derived names/sectors when available; it is not yet a full sortable market scanner with last price, extended-hours change, or provider-health columns.

## Technology Stack

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Window | winit (via eframe) | Cross-platform, Wayland + X11 |
| GPU | wgpu | Vulkan/Metal/DX12/OpenGL abstraction |
| UI | egui (immediate mode) | No retained state, 60fps, minimal allocations |
| Charts | egui Painter (custom) | Direct GPU rendering, no chart library dependency |
| Plots | egui_plot | Analytics charts (equity curves, histograms) |
| Async | tokio | Shared with engine for broker WebSocket |
| Cache | SQLite + zstd | TTBR binary format, ~3-5x compression |
| Analytics | darwin.rs (~9,600 lines) | 88 pub fns, 51 unit tests |
| Risk | risk.rs + margin.rs + var.rs | Full port of TyphooN EA v1.420 |

## Project Structure

```
TyphooN-Terminal/
├── native/                 # Native GPU application
│   ├── src/
│   │   ├── main.rs         # eframe init, wgpu renderer selection
│   │   ├── app.rs          # TyphooNApp, chart pane, palette, dispatch
│   │   ├── app/            # Window renderers (ADR-086 split)
│   │   │   ├── ai.rs               # AI Chat / Claude / Gemini /
│   │   │   │                       # Codex / Sessions / Response Cache
│   │   │   ├── alpaca_sync.rs      # Broker sync capacities, TF filters, no-data marks
│   │   │   ├── auto_compact.rs     # Legacy/raw-row zstd-22 compact gate (ADR-089)
│   │   │   ├── bar_sync.rs         # Bar sync health rows for Sync Status / Storage
│   │   │   ├── settings.rs         # Settings window
│   │   │   ├── storage.rs          # Storage Manager
│   │   │   ├── sync_status.rs      # Sync Status (per-broker %)
│   │   │   ├── tool_windows.rs     # Indicator + analytical windows
│   │   │   └── strategy_windows.rs # Strategy / backtest / optimizer
│   │   └── gpu_compute.rs  # WGSL indicator shaders
│   └── Cargo.toml
├── engine/                 # Shared engine library
│   ├── src/
│   │   ├── lib.rs          # Crate root
│   │   ├── core/
│   │   │   ├── cache.rs       # SQLite + zstd bar cache
│   │   │   ├── darwin.rs      # DARWIN analytics (80+ functions)
│   │   │   ├── research.rs    # TA-Lib + Godel parity surfaces
│   │   │   ├── fundamentals.rs # 21 data-source fundamentals
│   │   │   ├── risk.rs        # Lot sizing (4 order modes)
│   │   │   ├── margin.rs      # TRIM, PROTECT, margin math
│   │   │   ├── var.rs         # VaR, CVaR, portfolio risk
│   │   │   ├── backtest.rs    # Bar-by-bar engine, strategies
│   │   │   ├── screener.rs    # Symbol filtering
│   │   │   ├── fred.rs        # FRED economic data
│   │   │   ├── cryptocompare.rs # CryptoCompare deep history
│   │   │   ├── kraken.rs        # Kraken Spot public OHLCV
│   │   │   ├── kraken_futures.rs # Kraken Futures public candles
│   │   │   ├── ai_sessions.rs # ADR-082 chat persistence
│   │   │   ├── ai_response_cache.rs # ADR-083 cross-client cache
│   │   │   └── lan_sync.rs    # TLS + PBKDF2 LAN sync (ADR-045)
│   │   └── broker/
│   │       ├── alpaca.rs       # REST + WebSocket (ADR-087 autotune)
│   │       ├── tastytrade.rs   # REST + DXLink (ADR-018)
│   │       ├── kraken_broker.rs # Kraken Spot REST trading (ADR-051)
│   │       └── dxlink.rs       # DXLink WebSocket
│   └── Cargo.toml
├── cli/                    # TUI plus headless LAN server/client
│   ├── src/main.rs         # --lan-server / --lan-client, shared cache dir
│   └── Cargo.toml
├── deploy/                 # Docker, Kubernetes, Terraform, Ansible, Grafana/Prometheus/Kafka assets
│   ├── ansible/            # LAN host role with optional observability + Kafka (ADR-093)
│   ├── grafana/            # Provisioned datasource + LAN server dashboard
│   ├── kubernetes/         # lan-server + observability-kafka manifests
│   ├── prometheus/         # Prometheus scrape config for /metrics
│   └── terraform/          # Kubernetes module incl. observability + Kafka resources
├── mql5-compiler/          # MQL5 compiler plus full 10-language transpiler matrix
├── web/                    # WASM LAN client (ADR-052)
├── web-protocol/           # Shared web ↔ server message types
├── web-server/             # axum HTTPS + WebSocket relay
└── docs/
    ├── adr/                # 200+ Architecture Decision Records
    ├── API_KEYS.md
    ├── INDICATORS.md
    ├── PERFORMANCE.md
    ├── ROADMAP.md
    └── KEYBOARD_SHORTCUTS.md
```

## Performance Characteristics

| Metric | Native GPU |
|--------|-----------|
| Startup to interactive | < 2s |
| 10K bar chart render | < 5ms |
| Indicator computation (46+ indicators) | < 15ms |
| Memory (single chart) | ~50-80MB |
| Memory (MTF 4-cell grid) | ~100-150MB |
| Binary size (release) | ~25MB |

## Additional Features

### LAN Sync

TLS-encrypted (wss://) WebSocket cache synchronization between TyphooN Terminal instances. Ephemeral self-signed certificates (no pinning — PBKDF2 passphrase handles auth). 60-second periodic re-sync. Server and client auto-start on startup. Full data sync: bars + DARWIN tables + filtered KV cache + 34 pre-computed analytics fields + the whitelisted research snapshot tables. LAN clients are read-only viewers — zero local deal computation, all analytics from server KV. Broker positions/orders/account synced via KV for read-only display. 15 remote commands wired (SEC_SCRAPE, DARWIN_IMPORT, FETCH_BARS, INGEST_RESEARCH, etc.). Connected client IPs shown in server UI. Trading buttons disabled on LAN client. CLI/headless mode exposes Prometheus metrics for cache size, cache rows, per-series bar counts, liveness, and uptime. Implemented in `engine/src/core/lan_sync.rs` with CLI deployment in `cli/src/main.rs`.

### Storage Manager

The `STORAGE` command opens a cache storage manager with:
- View and delete data by symbol/source (color-coded by prefix: MT5, Alpaca, tastytrade, CryptoCompare, Kraken, Kraken Futures)
- **Compact (zstd-22):** Cleanup path for legacy/raw/imported bar_cache entries that are not already stored at maximum compression. New Rust bar-cache writes are zstd-22 immediately.
- **Auto-compact:** Configurable cadence, weekday/hour window, min-row threshold, last-run, next-window, skip-reason, and running-state readout
- **Purge All Bar Data:** Delete all bar_cache + bar_track entries (with red confirmation prompt)
- **Purge All DARWIN Data:** Delete all DARWIN accounts, deals, positions, equity snapshots (with red confirmation prompt)

### SQLite Multi-Connection Architecture

`SqliteCache` uses 5 connection types under WAL mode: `conn` (write Mutex), `read_conn` (UI-exclusive Mutex), BG thread (own connection, reopened each cycle), Phase 5 scoped threads (each opens own connection), Mt5Sync (own `SqliteCache::open()`). WAL allows unlimited concurrent readers. BG thread reopens its connection every 3s for WAL freshness. `maybe_decompress()` transparently handles both raw TTBR (from BarCacheWriter) and zstd-compressed (from Rust) data. Source MT5 databases use `open_readonly()` with 10s `busy_timeout`.

### Multi-Window Support

The `NEW_WINDOW` / `POPOUT` command spawns a new terminal process, enabling multi-monitor setups. Each window is an independent process with its own state.

### Collapsible Right Panel

The right panel sections (Trade, Positions, Orders, Watchlist, Risk) are individually collapsible/expandable for flexible layout management.

### GPU Indicator Compute

40+ indicators run on GPU (wgpu compute shaders) with CPU fallback for compatibility. GPU/CPU parity is mandatory — GPU shaders must produce identical output to CPU implementations. BetterVolume GPU uses the full Emini-Watch algorithm with OHLCV interleaved input (5 floats/bar). VWAP GPU uses per-day dispatch via anchored compute calls per trading day. Supply/Demand Zones: GPU does fractal detection (parallel), CPU does zone testing/merging/break detection. Recent chart-parity rounds (CMO, QStick, Disparity, BOP, StdDev — see ADR-079) follow the same GPU-first / CPU-fallback pattern.

### DARWIN Trade Overlay

Chart renders buy/sell arrows at DARWIN deal entry/exit points with timestamp-to-bar mapping. Open position entry prices shown as dashed lines. Same-price entries are aggregated (combined lot size, single marker). Positions panel filters to current chart symbol.

### TradingView-Style Watchlist

Right-aligned numeric columns (Last, Chg, Chg%, Vol) with painter-based rendering. Works offline via SQLite cache fallback — no broker connection required for cached price data. Sortable by any column.

### AI Sessions

Four AI surfaces with persistent, resumable sessions (ADR-082): Claude Code (`claude --resume <uuid>`), Gemini CLI, Codex CLI, and a generic AI Chat (Claude / OpenAI / Gemini / Grok / Mistral / Perplexity / Local). Sessions auto-save to the SqliteCache `kv_cache` (zstd-compressed, level 3 on hot mutable KV writes) on every reply. Cross-client AI response cache (ADR-083) deduplicates identical hosted-AI prompts across LAN clients so the same prompt issued from server + phone hits the cache once. Slash commands (`RESUMECLAUDE`, `RESUMEGEMINI`, `RESUMECODEX`, `RESUMEAI`) re-enter prior sessions; the AI Sessions browser window shows history with subject lines and last-touched timestamps. If a built-in AI reply includes an ADR-080 `===TYPHOON_INGEST===` Return Path block, ADR-212 queues the existing research-ingest broker path automatically.

### Research Packet (TA-Lib + Godel Parity)

The research packet is an AI-agent-readable markdown bundle emitted on demand via `RESEARCH_PACKET`. It carries every cached signal: ~375 TA-Lib primitives (indicators + candlestick patterns), Godel-Terminal-documented features (options chain, expirations calendar, earnings whispers, institutional ownership, insider transactions, etc.), and the user's open positions per symbol. Each surface flows through the same pipeline (snapshot struct → SQLite table → LAN-sync whitelist → BrokerCmd/Msg → packet emitter → egui popup) — see ADR-079. Chart-drawing parity for these signals is deferred (ADR-079); the agent reads the markdown directly.

### Web LAN Client (WASM)

A standalone WASM client built with `eframe`/`glow` (ADR-052) connects to the native `web-server` over HTTPS + WebSocket (PBKDF2 passphrase). Read-only chart, watchlist, positions/orders display — trading and analytics computation stay on the server. Built separately via `trunk`. See `web/`, `web-protocol/`, `web-server/` workspace members.
