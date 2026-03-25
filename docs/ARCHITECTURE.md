# TyphooN Terminal — Architecture

## Overview

Pure Rust native GPU application. No JavaScript, no WebKit, no IPC serialization.

```
┌─────────────────────────────────────────────────┐
│  Native Window (egui + wgpu)                    │
│  ┌─────────────────────────────────────────────┐│
│  │ Chart Renderer (egui Painter)               ││
│  │ - Candle/HeikinAshi/Line/Bars/Renko         ││
│  │ - 32+ indicators (pure Rust computation)    ││
│  │ - 7 drawing tools + 7 harmonic patterns     ││
│  │ - Sub-panes (Fisher, RSI, MACD, ADX, etc.)  ││
│  ├─────────────────────────────────────────────┤│
│  │ egui Panels                                 ││
│  │ - Console (~) with 104 commands             ││
│  │ - Positions / Orders / TradingView Watchlist││
│  │ - Risk calculator, VaR, Margin monitor      ││
│  │ - DARWIN analytics (69/69 engine functions) ││
│  │ - SEC Filing Scanner + Insider Trades       ││
│  │ - Finnhub News + Market Data APIs           ││
│  │ - Backtest engine + optimizer               ││
│  │ - MTF Grid (up to 16 chart viewports)       ││
│  │ - 32 floating windows                       ││
│  └─────────────────────────────────────────────┘│
├─────────────────────────────────────────────────┤
│  Engine Library (typhoon-engine crate)          │
│  - AlpacaBroker (REST + WebSocket)              │
│  - SqliteCache (TTBR binary, zstd compression)  │
│  - DarwinDB (69 analytics functions, 100% wired)│
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
| 2 | Kraken | 24/7 crypto, fills weekend gaps, history from 2013 |
| 3 | Alpaca | Live trading execution, US equities + crypto |

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
| Analytics | darwin.rs (5,400+ lines) | 70+ functions, 47 unit tests |
| Risk | risk.rs + margin.rs + var.rs | Full port of TyphooN EA v1.420 |

## Project Structure

```
TyphooN-Terminal/
├── native/                 # Native GPU application
│   ├── src/
│   │   ├── main.rs         # eframe init, wgpu renderer selection
│   │   └── app.rs          # All UI (5,147 lines)
│   └── Cargo.toml
├── engine/                 # Shared engine library
│   ├── src/
│   │   ├── lib.rs          # Crate root
│   │   ├── core/
│   │   │   ├── cache.rs    # SQLite + zstd bar cache
│   │   │   ├── darwin.rs   # DARWIN analytics (70+ functions)
│   │   │   ├── risk.rs     # Lot sizing (4 order modes)
│   │   │   ├── margin.rs   # TRIM, PROTECT, margin math
│   │   │   ├── var.rs      # VaR, CVaR, portfolio risk
│   │   │   ├── backtest.rs # Bar-by-bar engine, strategies
│   │   │   ├── screener.rs # Symbol filtering
│   │   │   └── ...
│   │   └── broker/
│   │       └── alpaca.rs   # REST + WebSocket client
│   └── Cargo.toml
├── cli/                    # Standalone TUI (6.5MB, SSH-ready)
├── mql5-compiler/          # MT5 XML → SQLite import pipeline
└── docs/
    ├── adr/                # 15 Architecture Decision Records
    ├── API_KEYS.md
    └── KEYBOARD_SHORTCUTS.md
```

## Performance Characteristics

| Metric | Native GPU |
|--------|-----------|
| Startup to interactive | < 2s |
| 10K bar chart render | < 5ms |
| Indicator computation (21 indicators) | < 10ms |
| Memory (single chart) | ~50-80MB |
| Memory (MTF 4-cell grid) | ~100-150MB |
| Binary size (release) | ~25MB |
