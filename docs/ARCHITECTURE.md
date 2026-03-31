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
│  │ - 56 drawing tools + 10 harmonic patterns     ││
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
| 1 | MT5 via BarCacheWriter | 895 symbols x 9 TFs, weekday authority (Darwinex) |
| 2 | Alpaca/tastytrade | Live trading execution, US equities + crypto |
| 3 | CryptoCompare | Deep crypto history (BTC from 2010), 2000 bars/request, all 9 TFs |
| 4 | Kraken | Weekend live gap-fill (720 bars, adaptive polling, no rate limit) |

MT5 is a **view-only data source** — bar data flows in via the BarCacheWriter EA to SQLite cache. Trade management stays in MT5 directly. DARWIN account analytics are imported via XLSX trade history exports.

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
| Analytics | darwin.rs (6,800+ lines) | 80 functions, 59 unit tests |
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
│   │       └── alpaca.rs   # REST + WebSocket client
│   └── Cargo.toml
├── cli/                    # Standalone TUI (6.5MB, SSH-ready)
├── mql5-compiler/          # MT5 XML → SQLite import pipeline
└── docs/
    ├── adr/                # 45 Architecture Decision Records
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

TLS-encrypted (wss://) WebSocket cache synchronization between TyphooN Terminal instances. Ephemeral self-signed certificates generated on server start. PBKDF2-HMAC-SHA256 challenge-response authentication. Full data sync: bars + DARWIN tables + KV cache (fundamentals, news, SEC, FRED). LAN clients forward data requests to server (no outbound API calls). Multi-client support. Implemented in `engine/src/core/lan_sync.rs`.

### Storage Manager

The `STORAGE` command opens a cache storage manager that allows viewing and deleting data by symbol/source. Includes a compact function that recompresses all bar_cache entries at zstd level 22 for maximum on-disk compression (decompression speed is unaffected).

### Multi-Window Support

The `NEW_WINDOW` / `POPOUT` command spawns a new terminal process, enabling multi-monitor setups. Each window is an independent process with its own state.

### Collapsible Right Panel

The right panel sections (Trade, Positions, Orders, Watchlist, Risk) are individually collapsible/expandable for flexible layout management.

### GPU Indicator Compute

28 indicators run on GPU (wgpu compute shaders) with CPU fallback for compatibility. Three indicators use CPU-only due to algorithmic requirements:
- **Supply/Demand Zones**: GPU does fractal detection (parallel), CPU does zone testing/merging/break detection
- **BetterVolume**: Requires buy/sell pressure estimation from OHLC (inherently sequential)
- **OBV**: Uses real volume data via interleaved [close, volume] GPU buffer

### DARWIN Trade Overlay

Chart renders buy/sell arrows at DARWIN deal entry/exit points with timestamp-to-bar mapping. Open position entry prices shown as dashed lines. Same-price entries are aggregated (combined lot size, single marker). Positions panel filters to current chart symbol.

### TradingView-Style Watchlist

Right-aligned numeric columns (Last, Chg, Chg%, Vol) with painter-based rendering. Works offline via SQLite cache fallback — no broker connection required for cached price data. Sortable by any column.
