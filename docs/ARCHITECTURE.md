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
│  │ - Console (~) with 205+ commands             ││
│  │ - Positions / Orders / TradingView Watchlist││
│  │ - Risk calculator, VaR, Margin monitor      ││
│  │ - DARWIN analytics (80+ engine functions)   ││
│  │ - SEC Filing Scanner + Insider Trades       ││
│  │ - Finnhub News + Market Data APIs           ││
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
│  - KrakenBroker (full Spot REST orders + OHLCV) │
│  - SqliteCache (TTBR binary, zstd compression)  │
│  - DarwinDB (80+ analytics, 100% wired)         │
│  - RiskEngine (VaR, TRIM, martingale)           │
│  - BacktestEngine (bar-by-bar, optimization)    │
│  - BarBuilder (WebSocket → OHLCV)               │
│  - LanSync (TLS, PBKDF2, 14 remote commands)    │
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
| 2 | tastytrade DXLink | Real-time bars + quotes for funded accounts. |
| 3 | Alpaca | US equities + crypto, free IEX or paid SIP. Tier-autotuned sync (ADR-203). |
| 4 | Kraken | Crypto trading (full Spot REST order surface) + public OHLCV gap-fill (720 bars, weekend coverage). |
| 5 | CryptoCompare | Deep crypto history (BTC from 2010), 2000 bars/request, hourly+ TFs. |

MT5 is a **view-only data source** — bar data flows in via the BarCacheWriter EA to SQLite cache. Trade execution flows through Alpaca / tastytrade / Kraken with MT5 EA semantics (partial close, close-all, cancel-exits-before-close — see ADR-201). DARWIN account analytics are imported via XLSX trade history exports. Alpaca auto-connects on startup if credentials are saved in the system keyring. tastytrade fully integrated: REST API (auth, positions, orders, quotes, market metrics, option chains) + DXLink WebSocket (historical bars). See ADR-022. Kraken supports public-OHLCV-only mode and authenticated Spot REST trading with full AddOrder parameters, batch orders, amend/edit, dead-man cancel, cancel-all, balances, orders, trades, ledgers, and positions (ADR-072). See ADR-037 for cross-source priority hierarchy.

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
| Analytics | darwin.rs (7,000+ lines) | 80 functions, 59 unit tests |
| Risk | risk.rs + margin.rs + var.rs | Full port of TyphooN EA v1.420 |

## Project Structure

```
TyphooN-Terminal/
├── native/                 # Native GPU application
│   ├── src/
│   │   ├── main.rs         # eframe init, wgpu renderer selection
│   │   ├── app.rs          # TyphooNApp, chart pane, palette, dispatch
│   │   ├── app/            # Window renderers (ADR-202 split)
│   │   │   ├── ai.rs               # AI Chat / Claude / Gemini /
│   │   │   │                       # Codex / Sessions / Response Cache
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
│   │   │   ├── ai_sessions.rs # ADR-157 chat persistence
│   │   │   ├── ai_response_cache.rs # ADR-162 cross-client cache
│   │   │   └── lan_sync.rs    # TLS + PBKDF2 LAN sync (ADR-065)
│   │   └── broker/
│   │       ├── alpaca.rs       # REST + WebSocket (ADR-203 autotune)
│   │       ├── tastytrade.rs   # REST + DXLink (ADR-022)
│   │       ├── kraken_broker.rs # Kraken Spot REST trading (ADR-072)
│   │       └── dxlink.rs       # DXLink WebSocket
│   └── Cargo.toml
├── cli/                    # TUI plus headless LAN server/client
│   ├── src/main.rs         # --lan-server / --lan-client, shared cache dir
│   └── Cargo.toml
├── deploy/                 # Docker, Kubernetes, Terraform LAN server assets
│   ├── kubernetes/
│   └── terraform/
├── mql5-compiler/          # MQL5 + PineScript + 8 transpiler backends
├── web/                    # WASM LAN client (ADR-073)
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

TLS-encrypted (wss://) WebSocket cache synchronization between TyphooN Terminal instances. Ephemeral self-signed certificates (no pinning — PBKDF2 passphrase handles auth). 15-second periodic re-sync. Server and client auto-start on startup. Full data sync: bars + DARWIN tables + KV cache + 34 pre-computed analytics fields. LAN clients are read-only viewers — zero local deal computation, all analytics from server KV. Broker positions/orders/account synced via KV for read-only display. 14 remote commands wired (SEC_SCRAPE, DARWIN_IMPORT, FETCH_BARS, etc.). Connected client IPs shown in server UI. Trading buttons disabled on LAN client. Implemented in `engine/src/core/lan_sync.rs`.

### Storage Manager

The `STORAGE` command opens a cache storage manager with:
- View and delete data by symbol/source (color-coded by prefix: MT5/Kraken/Alpaca)
- **Compact (zstd-22):** Recompress all bar_cache entries at maximum compression (decompression speed unaffected)
- **Purge All Bar Data:** Delete all bar_cache + bar_track entries (with red confirmation prompt)
- **Purge All DARWIN Data:** Delete all DARWIN accounts, deals, positions, equity snapshots (with red confirmation prompt)

### SQLite Multi-Connection Architecture

`SqliteCache` uses 5 connection types under WAL mode: `conn` (write Mutex), `read_conn` (UI-exclusive Mutex), BG thread (own connection, reopened each cycle), Phase 5 scoped threads (each opens own connection), Mt5Sync (own `SqliteCache::open()`). WAL allows unlimited concurrent readers. BG thread reopens its connection every 3s for WAL freshness. `maybe_decompress()` transparently handles both raw TTBR (from BarCacheWriter) and zstd-compressed (from Rust) data. Source MT5 databases use `open_readonly()` with 10s `busy_timeout`.

### Multi-Window Support

The `NEW_WINDOW` / `POPOUT` command spawns a new terminal process, enabling multi-monitor setups. Each window is an independent process with its own state.

### Collapsible Right Panel

The right panel sections (Trade, Positions, Orders, Watchlist, Risk) are individually collapsible/expandable for flexible layout management.

### GPU Indicator Compute

40+ indicators run on GPU (wgpu compute shaders) with CPU fallback for compatibility. GPU/CPU parity is mandatory — GPU shaders must produce identical output to CPU implementations. BetterVolume GPU uses the full Emini-Watch algorithm with OHLCV interleaved input (5 floats/bar). VWAP GPU uses per-day dispatch via anchored compute calls per trading day. Supply/Demand Zones: GPU does fractal detection (parallel), CPU does zone testing/merging/break detection. Recent chart-parity rounds (CMO, QStick, Disparity, BOP, StdDev — see ADR-200) follow the same GPU-first / CPU-fallback pattern.

### DARWIN Trade Overlay

Chart renders buy/sell arrows at DARWIN deal entry/exit points with timestamp-to-bar mapping. Open position entry prices shown as dashed lines. Same-price entries are aggregated (combined lot size, single marker). Positions panel filters to current chart symbol.

### TradingView-Style Watchlist

Right-aligned numeric columns (Last, Chg, Chg%, Vol) with painter-based rendering. Works offline via SQLite cache fallback — no broker connection required for cached price data. Sortable by any column.

### AI Sessions

Four AI surfaces with persistent, resumable sessions (ADR-157): Claude Code (`claude --resume <uuid>`), Gemini CLI, Codex CLI, and a generic AI Chat (Claude / OpenAI / Gemini / Grok / Mistral / Perplexity / Local). Sessions auto-save to the SqliteCache `kv_cache` (zstd-9 compressed) on every reply. Cross-client AI response cache (ADR-162) deduplicates identical prompts across LAN clients so the same prompt issued from server + phone hits the cache once. Slash commands (`RESUMECLAUDE`, `RESUMEGEMINI`, `RESUMECODEX`, `RESUMEAI`) re-enter prior sessions; the AI Sessions browser window shows history with subject lines and last-touched timestamps.

### Research Packet (TA-Lib + Godel Parity)

The research packet is an AI-agent-readable markdown bundle emitted on demand via `RESEARCH_PACKET`. It carries every cached signal: ~375 TA-Lib primitives (indicators + candlestick patterns), Godel-Terminal-documented features (options chain, expirations calendar, earnings whispers, institutional ownership, insider transactions, etc.), and the user's open positions per symbol. Each surface flows through the same pipeline (snapshot struct → SQLite table → LAN-sync whitelist → BrokerCmd/Msg → packet emitter → egui popup) — see ADR-188. Chart-drawing parity for these signals is deferred (ADR-188); the agent reads the markdown directly.

### Web LAN Client (WASM)

A standalone WASM client built with `eframe`/`glow` (ADR-073) connects to the native `web-server` over HTTPS + WebSocket (PBKDF2 passphrase). Read-only chart, watchlist, positions/orders display — trading and analytics computation stay on the server. Built separately via `trunk`. See `web/`, `web-protocol/`, `web-server/` workspace members.
