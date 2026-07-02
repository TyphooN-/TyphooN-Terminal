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
│  │ - Broker trade arrows + position lines       ││
│  │ - Sub-panes (Fisher, RSI, MACD, ADX, etc.)  ││
│  ├─────────────────────────────────────────────┤│
│  │ egui Panels                                 ││
│  │ - Console (~) with 260+ commands             ││
│  │ - Positions / Orders / TradingView Watchlist││
│  │ - Risk calculator, VaR, Margin monitor      ││
│  │ - Risk / VaR / margin + research analytics  ││
│  │ - SEC Filing Scanner + Insider Trades       ││
│  │ - Multi-source News & Research cache        ││
│  │ - Fundamentals (21 data sources)             ││
│  │ - Research packet (research + indicator parity)   ││
│  │ - AI sessions (Claude / Gemini / Codex)     ││
│  │ - Backtest engine + optimizer (GPU)         ││
│  │ - MTF Grid (up to 16 chart viewports)       ││
│  │ - 54+ floating windows                      ││
│  └─────────────────────────────────────────────┘│
├─────────────────────────────────────────────────┤
│  Engine Library (typhoon-engine crate)          │
│  - AlpacaBroker (REST market/account/trading)   │
│  - KrakenBroker (Spot REST orders + private WS) │
│  - SqliteCache (TTBR binary, zstd compression)  │
│  - RiskEngine (VaR, TRIM, order sizing)         │
│  - BacktestEngine (bar-by-bar, optimization)    │
│  - Research (research + indicator parity surfaces)    │
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

Current broker & data scope is **Kraken + Alpaca** (ADR-111), with Yahoo Chart as corroborator. The architecture remains broker-modular: L1/L2/L3 capability, entitlement, freshness, and snapshot-vs-stream behavior are modeled per broker via a typed capability model (`typhoon-engine::broker::capabilities` — `MarketDataSupport`/`DepthAssetScope`/`BrokerMarketDataCapabilities` with an exhaustive match over `OrderBroker`; ADR-129) so the selected primary broker does not hard-wire UI semantics. After the Alpaca/Kraken combover, tastytrade is the likely next restored broker module; Binance is a plausible later crypto venue. Equity bars merge a trusted tier against an independent corroborator (ADR-112/113), with known stock splits back-adjusted from a curated fallback when the FMP split feed is unavailable (ADR-122).

Recent market data work (ADR-129/109): Strong L1 (ticker/quotes/trades with sizes and freshness guards) for both brokers. Kraken L2 (v2 book with atomic CRC32, exact wire tokens, shared DOM depth preference across toolbar/DOM/Order Flow/Bookmap stream entrypoints). L3 foundation (per-order `ws_v2_level3.rs`, real/sim streamer + token/no-token entitlement status, CRC, KrakenL3State). Depth profile (live bins + overlay) and richer Bookmap (per-order bid/ask markers, selected-order persistence/highlight, age coloring, interactions) on focused symbols. M1/M5 are valid low-TF targets for Kraken Spot and Equities (assist rows like Alpaca/Yahoo remain non-target/stale for those TFs).

| Tier | Source | Coverage |
|------|--------|----------|
| Trusted | Kraken Spot | Crypto trading (full Spot REST order surface + private WS ownTrades/openOrders), recent REST OHLCV, and full-catalog public OHLC WebSocket forward freshness under `kraken:SYMBOL:TF`. Spot public pacing/cooldown per ADR-095 and WS write-path controls per ADR-099. |
| Trusted | Kraken Securities / xStocks | Tokenized-equity bars under `kraken-equities:SYMBOL:TF` via iapi AIMD (ADR-101). Native high timeframes full-catalog; native intraday demand/focus scoped (ADR-112). Kraken sources xStock bars from Alpaca's backend, so it is not self-corroborating (ADR-113). |
| Trusted | Kraken Futures | Public futures instrument discovery and full chart-candle sync under `kraken-futures:SYMBOL:TF` using explicit from/to ranges. |
| Trusted | Alpaca | US equities + crypto, free IEX or paid SIP. Tier-autotuned sync (ADR-087); catalog-breadth lane for equities (ADR-112). |
| Corroborator | Yahoo Chart | Independent equity history + freshness assist under `yahoo-chart:SYMBOL:TF` (15Min/30Min/1Hour/1Day/1Week/1Month — no native 4-hour). Back-adjusted + scale-validated before splicing; the only independent reference for the Kraken/Alpaca trusted tier (ADR-113). |

Trade execution currently flows through **Alpaca and Kraken** with shared net-position EA semantics (partial close, close-all, cancel-exits-before-close). Alpaca auto-connects on startup if credentials are saved in the system keyring. Kraken supports public-OHLCV-only mode and authenticated Spot REST trading with full AddOrder parameters, batch orders, amend/edit, dead-man cancel, cancel-all, balances, orders, trades, ledgers, positions, and private WebSocket `ownTrades`/`openOrders` with reconnect/resubscribe (ADR-051). Kraken public bar sync is fully async at the task level: direct Spot/Futures fetches spawn per-timeframe tasks, and all Kraken HTTP work runs under bounded queue/semaphore control (ADR-094). Spot OHLC HTTP calls are paced to Kraken's documented public limit with cooldown on throttles (ADR-095), while the Spot OHLC WebSocket lane streams the full WS-mappable Spot catalog and persists only closed/coalesced bars through the fast off-thread merge path (ADR-099). Kraken Securities/iapi uses a separate persisted AIMD limiter that starts conservatively, ramps on clean traffic, halves on congestion/rate-limit responses, and defaults to a 5 req/s ceiling; power users can raise only that ceiling with `TYPHOON_KRAKEN_IAPI_AIMD_MAX_RATE`, but the discovered-ceiling/backoff logic remains active. Securities/xStocks fallback providers (`Alpaca`, `Yahoo Chart`) remain separate source namespaces and are merged only for chart/research usability when enabled. See ADR-113 for the cross-source merge/priority hierarchy; chart source order includes the implementation-specific `kraken-equities` and `default` fallbacks around the table above.

### Cross-broker history assist and broker-module expansion

Kraken equities fallback is the first implementation of a broader rule: enabled
brokers/sources may assist each other through a normalized instrument-identity
layer. The same modularity rule applies to future tastytrade/Binance support: provider adapters advertise capabilities, and shared chart/watchlist/DOM/Bookmap surfaces consume normalized L1/L2/L3 data rather than broker-specific UI forks. A new broker should not force a full cold historical sync when existing
compatible bars already cover the same economic instrument. The selected broker
remains authoritative for execution and native health, while compatible history
from other sources is merged at read/render/research time with provenance. Native
cache namespaces stay separate, and visible symbol equality is only a candidate
match — wrappers, CFDs, ADRs, quote currencies, suffixes, data delays, and session
calendars require explicit mapping before histories can be combined.

News is a separate research cache, not a market-data universe mirror. Sync Status totals count `(symbol, timeframe)` bar-cache entries, while news scrapes count deduped symbols, so the numbers will not match one-for-one. Kraken equities Sync Status uses an explicit denominator: native `1Day`/`1Week`/`1Month` rows are full-catalog (`kraken_equity_universe_symbols`), native intraday rows remain demand-scoped unless iapi throughput proves broader native intraday safe, and fallback/merged rows can use full-catalog `15Min`+ coverage when an enabled provider supports that timeframe. Fallback rows (`Alpaca`, `Yahoo`) report provider assist coverage, not Kraken coverage; Yahoo 404/empty-result responses are normal no-data tombstones for symbols Yahoo cannot resolve, especially SPAC/unit-style Kraken Securities symbols. The News & Research window fetches the active symbol, currently visible MTF Grid symbols, or the configured source bulk news universe; Kraken bulk news candidates are derived from cached `kraken:*`, `kraken-equities:*`, and `kraken-futures:*` market-data keys when Kraken is enabled. `news/SYM: 0 articles fetched` means the enabled news providers returned no rows for that selected symbol, not that Kraken equities bar sync skipped the rest of the catalog. See ADR-078 and ADR-102.

The `SYM` / `SYMBOLS` command opens Symbol Explorer. It is the catalog-facing way to inspect large universes instead of inferring coverage from runtime logs: cached rows are grouped by source/timeframe, broker-universe rows are grouped by rough asset category, the filter matches symbol/name text, and rows can load a chart or add to watchlist. It currently shows catalog/cache presence and fundamentals-derived names/sectors when available; it is not yet a full sortable market scanner with last price, extended-hours change, or provider-health columns.

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
| Analytics | research.rs + screener.rs | research + indicator parity surfaces, EV/signal scanning |
| Risk | risk.rs + margin.rs + var.rs | Full port of TyphooN EA v1.420 |

## Project Structure

```
TyphooN-Terminal/
├── typhoon-native/                 # Native GPU application
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
├── typhoon-engine/                 # Shared engine library
│   ├── src/
│   │   ├── lib.rs          # Crate root
│   │   ├── core/
│   │   │   ├── cache.rs       # SQLite + zstd bar cache
│   │   │   ├── research.rs    # research + indicator parity surfaces
│   │   │   ├── fundamentals.rs # 21 data-source fundamentals
│   │   │   ├── risk.rs        # Lot sizing (4 order modes)
│   │   │   ├── margin.rs      # TRIM, PROTECT, margin math
│   │   │   ├── var.rs         # VaR, CVaR, portfolio risk
│   │   │   ├── backtest.rs    # Bar-by-bar engine, strategies
│   │   │   ├── screener.rs    # Symbol filtering
│   │   │   ├── fred.rs        # FRED economic data
│   │   │   ├── kraken.rs        # Kraken Spot public OHLCV
│   │   │   ├── kraken_futures.rs # Kraken Futures public candles
│   │   │   ├── ai_sessions.rs # ADR-082 chat persistence
│   │   │   └── ai_response_cache.rs # ADR-083 local AI response cache
│   │   └── broker/
│   │       ├── protocol.rs     # BrokerCmd / BrokerMsg / OrderBroker bus (ADR-127)
│   │       ├── capabilities.rs # Typed L1/L2/L3 capability + provenance model (ADR-129)
│   │       ├── alpaca.rs       # REST + WebSocket (ADR-087 autotune)
│   │       └── kraken/         # Kraken Spot REST trading + WS (ADR-051)
│   └── Cargo.toml
├── typhoon-broker-runtime/         # Broker command processor + handlers + research compute (ADR-125 Target 3, unblocked by ADR-127)
├── typhoon-chart-ui/               # Chart types, state, indicators, drawing, egui chart render (ADR-125 Target 2)
├── typhoon-research-ui/            # Research packet formatter + window shell + packet section tree (ADR-125 Target 1)
├── typhoon-transpiler/             # Multi-language indicator transpiler + WASM/WGSL codegen
└── docs/
    ├── adr/                # Architecture Decision Records (106)
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

### Storage Manager

The `STORAGE` command opens a cache storage manager with:
- View and delete data by symbol/source (color-coded by prefix: Alpaca, Kraken, Kraken Securities, Kraken Futures, Yahoo)
- **Base bar zstd:** Runtime/persisted Storage Manager control for normal Rust bar-cache write compression (default 3, range 1-22). Kraken WS hot writes stay at zstd-3 for responsiveness.
- **Compact / Auto-compact (zstd-22):** Manual and scheduled promotion path for configured-base, legacy/raw/imported, and WS-written bar_cache entries below the archival target. Auto-compact exposes cadence, weekday/hour window, min-row threshold, last-run, next-window, skip-reason, and running-state readout
- **Purge All Bar Data:** Delete all bar_cache + bar_track entries (with red confirmation prompt)

### SQLite Multi-Connection Architecture

`SqliteCache` uses multiple connection types under WAL mode: `conn` (write Mutex), `read_conn` (UI-exclusive Mutex), BG thread (own connection, reopened each cycle), and scoped sync threads (each opens its own connection). WAL allows unlimited concurrent readers. BG thread reopens its connection every 3s for WAL freshness. `maybe_decompress()` transparently handles both raw TTBR and zstd-compressed bar blobs.

### Multi-Window Support

The `NEW_WINDOW` / `POPOUT` command spawns a new terminal process, enabling multi-monitor setups. Each window is an independent process with its own state.

### Collapsible Right Panel

The right panel sections (Trade, Positions, Orders, Watchlist, Risk) are individually collapsible/expandable for flexible layout management.

### GPU Indicator Compute

40+ indicators run on GPU (wgpu compute shaders) with CPU fallback for compatibility. GPU/CPU equivalence is required — shaders must produce identical output to CPU implementations. BetterVolume GPU uses the full Emini-Watch algorithm with OHLCV interleaved input (5 floats/bar). VWAP GPU uses per-day dispatch via anchored compute calls per trading day. Supply/Demand Zones: GPU does fractal detection (parallel), CPU does zone testing/merging/break detection. Recent chart expansions (CMO, QStick, Disparity, BOP, StdDev — see ADR-079) follow the same GPU-first / CPU-fallback pattern.

### Broker Trade Overlay

Chart renders buy/sell arrows at broker fill entry/exit points (Alpaca + Kraken) with timestamp-to-bar mapping. Open position entry prices shown as dashed lines. Same-price entries are aggregated (combined size, single marker). Positions panel filters to current chart symbol.

### TradingView-Style Watchlist

Right-aligned numeric columns (Last, Chg, Chg%, Vol) with painter-based rendering. Works offline via SQLite cache fallback — no broker connection required for cached price data. Sortable by any column.

### AI Sessions

Four AI surfaces with persistent, resumable sessions (ADR-082): Claude Code (`claude --resume <uuid>`), Gemini CLI, Codex CLI, and a generic AI Chat (Claude / OpenAI / Gemini / Grok / Mistral / Perplexity / Local). Sessions auto-save to the SqliteCache `kv_cache` (zstd-compressed, level 3 on hot mutable KV writes) on every reply. Local AI response cache (ADR-083) deduplicates identical hosted-AI prompts so repeated prompts avoid duplicate hosted-model calls. Slash commands (`RESUMECLAUDE`, `RESUMEGEMINI`, `RESUMECODEX`, `RESUMEAI`) re-enter prior sessions; the AI Sessions browser window shows history with subject lines and last-touched timestamps. If a built-in AI reply includes an ADR-080 `===TYPHOON_INGEST===` Return Path block, ADR-096 queues the existing research-ingest broker path automatically.

### Research Packet (Research and indicator parity)

The research packet is an AI-agent-readable markdown bundle emitted on demand via `RESEARCH_PACKET`. It carries every cached signal: ~375 indicator/candlestick primitives plus external-terminal-style research features (options chain, expirations calendar, earnings whispers, institutional ownership, insider transactions, etc.), and the user's open positions per symbol. Each surface flows through the same pipeline (snapshot struct → SQLite table → BrokerCmd/Msg → packet emitter → egui popup) — see ADR-079. Chart-drawing parity for these signals is deferred (ADR-079); the agent reads the markdown directly.
