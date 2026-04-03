# TyphooN Terminal — Architecture

## Overview

Pure Rust native GPU application. No JavaScript, no WebKit, no IPC serialization.

```
┌─────────────────────────────────────────────────┐
│  Native Window (egui + wgpu)                    │
│  ┌─────────────────────────────────────────────┐│
│  │ Chart Renderer (egui Painter)               ││
│  │ - Candle/HeikinAshi/Line/Bars/Renko         ││
│  │ - 32+ indicators (GPU + CPU fallback)        ││
│  │ - 71 drawing tools + 10 harmonic patterns   ││
│  │ - DARWIN trade arrows + position lines       ││
│  │ - Sub-panes (Fisher, RSI, MACD, ADX, etc.)  ││
│  ├─────────────────────────────────────────────┤│
│  │ egui Panels                                 ││
│  │ - Console (~) with 121 commands               ││
│  │ - Positions / Orders / TradingView Watchlist││
│  │ - Risk calculator, VaR, Margin monitor      ││
│  │ - DARWIN analytics (80 engine functions)    ││
│  │ - SEC Filing Scanner + Insider Trades       ││
│  │ - Finnhub News + Market Data APIs           ││
│  │ - Backtest engine + optimizer               ││
│  │ - MTF Grid (up to 16 chart viewports)       ││
│  │ - 29 floating windows                       ││
│  └─────────────────────────────────────────────┘│
├─────────────────────────────────────────────────┤
│  Engine Library (typhoon-engine crate)          │
│  - AlpacaBroker (REST + WebSocket)              │
│  - SqliteCache (TTBR binary, zstd compression)  │
│  - DarwinDB (80 analytics functions, 100% wired) │
│  - RiskEngine (VaR, TRIM, martingale)           │
│  - BacktestEngine (bar-by-bar, optimization)    │
│  - BarBuilder (WebSocket → OHLCV)               │
│  - Notifications (Discord, Pushover, ntfy)      │
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
| 2 | Alpaca | Live trading execution, US equities + crypto. Auto-connects on startup. |
| 3 | tastytrade | Options/futures, DXLink WebSocket historical bars, IV rank/percentile, option chains. |
| 4 | CryptoCompare | Deep crypto history (BTC from 2010), 2000 bars/request, hourly+ TFs |
| 5 | Kraken | Sub-hourly gap-fill (720 bars, weekend coverage, no rate limit) |

MT5 is a **view-only data source** — bar data flows in via the BarCacheWriter EA to SQLite cache. Trade management stays in MT5 directly. DARWIN account analytics are imported via XLSX trade history exports. Alpaca auto-connects on startup if credentials are saved in the system keyring. tastytrade fully integrated: REST API (auth, positions, orders, quotes, market metrics, option chains) + DXLink WebSocket (historical bars). See ADR-022.

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
│   │   └── app.rs          # All UI (17,033 lines)
│   └── Cargo.toml
├── engine/                 # Shared engine library
│   ├── src/
│   │   ├── lib.rs          # Crate root
│   │   ├── core/
│   │   │   ├── cache.rs    # SQLite + zstd bar cache
│   │   │   ├── darwin.rs   # DARWIN analytics (80 functions)
│   │   │   ├── risk.rs     # Lot sizing (4 order modes)
│   │   │   ├── margin.rs   # TRIM, PROTECT, margin math
│   │   │   ├── var.rs      # VaR, CVaR, portfolio risk
│   │   │   ├── backtest.rs # Bar-by-bar engine, strategies
│   │   │   ├── screener.rs # Symbol filtering
│   │   │   ├── fred.rs     # FRED economic data (yield curve, CPI, GDP, VIX, M2)
│   │   │   ├── cryptocompare.rs # CryptoCompare deep history backfill
│   │   │   └── ...
│   │   └── broker/
│   │       ├── alpaca.rs   # REST + WebSocket client
│   │       ├── tastytrade.rs # REST API (auth, positions, orders, quotes, options)
│   │       └── dxlink.rs   # DXLink WebSocket (historical bars, streaming)
│   └── Cargo.toml
├── cli/                    # Standalone TUI (6.5MB, SSH-ready)
├── mql5-compiler/          # MQL5 + PineScript → WASM/WGSL compiler (82 tests)
└── docs/
    ├── adr/                # 47 Architecture Decision Records
    ├── API_KEYS.md
    └── KEYBOARD_SHORTCUTS.md
```

## Performance Characteristics

| Metric | Native GPU |
|--------|-----------|
| Startup to interactive | < 2s |
| 10K bar chart render | < 5ms |
| Indicator computation (32+ indicators) | < 15ms |
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

28 indicators run on GPU (wgpu compute shaders) with CPU fallback for compatibility. GPU/CPU parity is mandatory — GPU shaders must produce identical output to CPU implementations. BetterVolume GPU uses the full Emini-Watch algorithm with OHLCV interleaved input (5 floats/bar). VWAP GPU uses per-day dispatch via anchored compute calls per trading day. Supply/Demand Zones: GPU does fractal detection (parallel), CPU does zone testing/merging/break detection.

### DARWIN Trade Overlay

Chart renders buy/sell arrows at DARWIN deal entry/exit points with timestamp-to-bar mapping. Open position entry prices shown as dashed lines. Same-price entries are aggregated (combined lot size, single marker). Positions panel filters to current chart symbol.

### TradingView-Style Watchlist

Right-aligned numeric columns (Last, Chg, Chg%, Vol) with painter-based rendering. Works offline via SQLite cache fallback — no broker connection required for cached price data. Sortable by any column.
