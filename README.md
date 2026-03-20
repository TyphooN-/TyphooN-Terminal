# TyphooN-Terminal

A native desktop trading terminal + TUI CLI with full risk management, multi-timeframe charting, and hedged martingale support — built in Rust/Tauri for Alpaca Markets.

**Website:** [MarketWizardry.org](https://www.marketwizardry.org/) | **License:** [BSL 1.1](LICENSE) ([Commercial](LICENSE-COMMERCIAL))

## At a Glance

| Metric | Value |
|---|---|
| **GUI Binary** | ~12-15MB Tauri (vs Electron ~200MB) |
| **CLI Binary** | 6.5MB standalone TUI (SSH/VPS ready) |
| **Memory Usage** | ~50-100MB (vs thinkorswim ~2GB+) |
| **Startup Time** | <1 second |
| **Lines of Code** | ~45,500 (Rust + JS + Wasm) |
| **Indicators** | 39 (9 NNFX + 21 standard + 9 MT5 parity) |
| **Commands** | 288 Bloomberg-style (Ctrl+K) |
| **Drawing Tools** | 44 GPU-rendered (Fibonacci, channels, pitchforks, Elliott, Gann) |
| **Data Sources** | 21 free APIs (Alpaca, SEC, FRED, Finnhub, CoinGecko, ECB, ...) |
| **Security Audit** | 21 passes, 97 findings, 91 fixed |
| **Cost** | Free for personal use ([commercial licensing](LICENSE-COMMERCIAL) available) |

---

## Features

| Feature | Description |
|---|---|
| **Charting** | Candlestick charts with 10K+ bar support, auto-load on timeframe change, multi-timeframe indicator overlays, separate indicator panes |
| **Risk Management** | 4 order modes: Standard (% risk), Fixed lots, Dynamic (min-balance scaling), VaR (percent/notional) |
| **Hedged Martingale** | Forward-looking TRIM, dynamic PROTECT, Open MG one-click setup, equity TP, unwind — full port of TyphooN EA v1.420 |
| **Order Placement** | Draggable SL/TP lines, 6 order types (market/bracket/limit/stop/stop-limit/trailing), auto lot calculation |
| **Order Management** | Open positions panel with live P/L, trade history, cancel pending orders, smart partial close |
| **Price Alerts** | Set alerts at any price, browser notifications, persistent across sessions |
| **Backtester** | Strategy trait with SMA Cross example, equity curve, trade reports (Sharpe, drawdown, profit factor) |
| **WebSocket Streaming** | Real-time trades/quotes via Alpaca WebSocket, Time & Sales |
| **Options Chain** | Full Greeks, strike/expiry/bid/ask via Alpaca options API |
| **Stock Screener** | Filter by price, volume, sector, change%, tradable/shortable flags |
| **Command Palette** | Ctrl+K Bloomberg-style: DES, NEWS, FA, OPT, SCAN, HDS, HIST, QM |
| **Watchlist** | Multi-symbol quote monitor with live prices and daily change |
| **Chart Templates** | Save/load indicator configs and order mode |
| **Workspace Profiles** | Save/load entire layout (tabs, indicators, pane sizes) |
| **Drawing Tools** | 44 types: trend, fib, ray, ruler, rectangle, channel, pitchfork, Elliott, Gann, regression, arrows, labels + GPU rendering |
| **Multi-Chart Layouts** | MTF grid (2-5 timeframes with full NNFX indicators per cell) |
| **Screenshot Export** | Ctrl+Shift+S to clipboard with toast notification |
| **Push Notifications** | Pushover + ntfy.sh for mobile alerts |
| **CSV Export** | Export trade history as CSV |
| **Chart Types** | Candlestick, Heikin-Ashi, Line, and Bar chart rendering |
| **Risk/Reward Overlay** | Visual profit/loss zones on chart when SL/TP lines set |
| **Trade Journal** | Log trades with notes, review history (Ctrl+K → JOURNAL) |
| **Position Calculator** | Risk-based sizing with R:R ratio (Ctrl+K → CALC) |
| **Chart Annotations** | Text markers on chart bars (Ctrl+K → ANNOTATE) |
| **Regime Detection** | ADX-based trending/ranging/choppy state in dashboard |
| **Pattern Recognition** | Auto-detect double top/bottom, head & shoulders (Ctrl+K → PATTERNS) |
| **Sentiment Analysis** | Keyword-based bullish/bearish scoring from news (Ctrl+K → SENTIMENT) |
| **Volatility Surface** | Options IV heatmap by strike×expiry (Ctrl+K → VOLSURF) |
| **Portfolio Heat Map** | Finviz-style colored boxes by P&L (Ctrl+K → HEATMAP) |
| **Bracket Order UI** | Visual OCO/bracket order placement (Ctrl+K → BRACKET) |
| **Alert Dashboard** | Cross-watchlist alert monitoring (Ctrl+K → ALERTBOARD) |
| **Custom Timeframes** | 2H, 3H, 6H, 2D, 3D via bar aggregation |
| **Renko Charts** | ATR-based Renko brick charting |
| **GUI Menu Bar** | File/View/Trading/Tools/Research/Analysis dropdown menus |
| **Draggable Tabs** | Drag-and-drop tab reordering (unique — no competitor has this) |
| **Ray / Ruler** | TradingView-style ray and measurement tools |
| **Bid/Ask Spread** | Real-time bid/ask/spread display from Alpaca quotes |
| **Time & Sales** | WebSocket trade stream in scrolling panel |
| **Account Activities** | Deposits, withdrawals, dividends, fills history |
| **Insider Trading** | SEC Form 4 filings via EDGAR (command palette: INSIDER) |
| **Context Menu** | Right-click chart for drawing tools, alerts, copy price |
| **Pending Orders on Chart** | Open orders visualized as colored price lines |
| **Data Window** | Fixed OHLCV + all indicator values at cursor position |
| **Multi-Condition Alerts** | RSI > 70, KAMA crosses SMA200, Fisher > 0, dividend alerts |
| **Walk-Forward Testing** | 70/30 in-sample/out-of-sample with auto-optimization |
| **Monte Carlo** | 100K simulations for risk-of-ruin at 25/50/75% drawdown |
| **Correlation Matrix** | Pairwise Pearson correlation heatmap from cached bars |
| **Portfolio Breakdown** | Positions grouped by asset class with $ value/P&L |
| **Drawing Properties** | Right-click drawings: color picker, line width, delete |
| **Earnings Calendar** | Corporate actions table via command palette |
| **FRED Economic Data** | Fed Funds, CPI, GDP, Treasury yields, VIX, M2 Supply (free API key) |
| **AI Trading Assistant** | Claude (Anthropic) or GPT (OpenAI) with market context |
| **NNFX Strategy** | KAMA + Fisher Transform backtesting strategy — optimized for D1/W1/MN1 |
| **Options P&L Calc** | Multi-leg payoff diagram with canvas rendering (Ctrl+K → OPTCALC) |
| **Sector Rotation** | S&P 500 sector ETF heatmap with daily/weekly % (Ctrl+K → SECTORS) |
| **Options Strategy** | Live chain viewer, presets (spreads, condors), aggregate Greeks (Ctrl+K → OPTSTRAT) |
| **Auto-Trading** | JS plugin → live order execution, paper-only safety (Ctrl+K → AUTOTRADE) |
| **Headless CLI** | `--backtest` mode: run strategies from command line, no GUI needed (VPS/SSH) |
| **Community Chat** | Matrix protocol chat via Ctrl+K → CHAT, no server needed |
| **Broker Abstraction** | BrokerTrait — extensible to any broker via single Rust file |
| **Multi-Account** | Save/load multiple Alpaca accounts (paper + live), AES-256-GCM encrypted credential storage |
| **Indicators** | 39 indicators: NNFX system (9) + standard (21) + MT5 parity (9: Alligator, AO, MFI, Force Index, Envelopes, StdDev, Chaikin, DeMarker, Fractals) |
| **Security** | 21-pass audit (97 findings): AES-256-GCM credential encryption, input validation, HTTP timeouts, path traversal, CSP, config bounds, zeroize, async lock optimization |
| **Analyst Ratings** | Finnhub consensus: stacked buy/hold/sell chart + price targets (Ctrl+K → ANR) |
| **Fear & Greed** | Market sentiment gauge (0-100) + 30-day sparkline (Ctrl+K → FEAR) |
| **Dark Pool** | FINRA RegSHO daily short volume with gauge visualization (Ctrl+K → DARKPOOL) |
| **Congress Trading** | House Stock Watcher: congressional trades filterable by symbol/rep/party (Ctrl+K → CONGRESS) |
| **Earnings Overlay** | Toggle E/D/S markers on chart for earnings, dividends, splits (Ctrl+K → EARNINGS-OVERLAY) |
| **World Indices** | 14 major indices across Americas/Europe/Asia-Pacific, auto-refresh (Ctrl+K → WEI) |
| **Forex Dashboard** | ECB rates + 6×6 cross rate matrix (Ctrl+K → FX) |
| **Crypto Market** | CoinGecko top 50 + trending + 7-day sparklines (Ctrl+K → CRYPTO) |
| **Yield Curve** | Treasury rates with 2Y-10Y inversion detection (Ctrl+K → YIELD) |
| **GPU Chart Engine** | WebGL2 candlesticks, 44 drawing tools, sub-panes, price lines, histograms, fills — all on GPU |
| **286 Commands** | Most command palette entries of any trading terminal, open or proprietary |

---

## NNFX Indicator System

Ported from the [MQL5-NNFX-Risk_Management_System](https://github.com/TyphooN-/MQL5-NNFX-Risk_Management_System) with exact visual parity.

### Main Chart (enabled by default)

| Indicator | Source | Description |
|---|---|---|
| **MultiKAMA** (10/2/30) | KAMA.mqh / MultiKAMA.mqh | Kaufman Adaptive MA from multiple timeframes, white, width 2 |
| **200 SMA** | Standard | Yellow, width 1 |
| **Previous Candle Levels** | PreviousCandleLevels.mqh | MTF previous bar high/low — white (H1/H4), magenta (D1/W1) |
| **ATR Projection** (14) | ATR_Projection.mqh | MTF open ± ATR bands — solid yellow, width 2 |
| **Supply/Demand Zones** | Custom | Filled rectangles at impulse move origins — green (demand), red (supply) |

### Separate Panes (enabled by default)

| Indicator | Source | Description |
|---|---|---|
| **Ehlers Fisher Transform** (32) | EhlersFisherTransform.mqh | Color-changing line (green bullish / red bearish) + gray signal line |
| **BetterVolume** | Custom | Volume histogram colored by price action (climax/churn/high/low) |

### MTF MA Grid

| Row | Description |
|---|---|
| SMA200 | Price above/below 200 SMA on H1/H4/D1/W1 — green/red dots |
| KAMA | Price above/below KAMA on each timeframe |
| Fisher | Fisher Transform bullish/bearish state per timeframe |

### Standard Indicators (disabled by default)

EMA (50/200), SMA (50), DEMA (21), RSI (14), MACD (12/26/9), Bollinger (20), ATR (14), VWAP, RVOL (10), Volume, Stochastic (14/3/3), CCI (20), ADX (14), Williams %R (14), Ichimoku Cloud, Parabolic SAR, OBV, Momentum (10), WMA (20), HMA (20)

---

## Risk Engine

Full port of TyphooN EA v1.420 risk management from MQL5 to Rust:

| Module | Description |
|---|---|
| **margin.rs** | Forward-looking TRIM, PROTECT urgency, spread tolerance, usable margin with buffer |
| **risk.rs** | All 4 order modes (Standard/Fixed/Dynamic/VaR), RiskLots calculation, lot normalization |
| **var.rs** | VaR calculation with inline StdDev, inverse cumulative normal, configurable confidence |
| **position.rs** | Hedge/bias tracking, break-even detection, SL/TP P/L, risk/reward ratio |
| **martingale.rs** | State machine (OFF/LONG/SHORT/UNWIND), TRIM/PROTECT decisions, Open MG sizing, equity TP |

12 unit tests covering margin math, lot sizing, and VaR calculations.

---

## Keyboard Shortcuts

| Key | Action |
|---|---|
| `b` | Buy Lines (SL = low, TP = high) |
| `s` | Sell Lines (SL = high, TP = low) |
| `d` | Destroy Lines |
| `t` | Open Trade |
| `m` | Martingale mode toggle |
| `o` | Open MG |
| `c` | Close All |
| `p` | Close Partial |
| `l` | Draw trend line (click 2 points) |
| `f` | Draw Fibonacci retracement (click high/low) |
| `x` | Delete last drawing |
| `g` | Toggle MTF grid view (Alt+G to tile floating windows) |
| `a` | Set price alert at current price |
| `h` | Refresh trade history |
| `Esc` | Clear SL/TP lines |

---

## Architecture

**Rust backend (Tauri 2.0)** — risk engine, Alpaca REST API, margin math, VaR, martingale state machine.

**JavaScript frontend** — TradingView lightweight-charts (MIT, 170KB), HTML/CSS UI with 10-button panel, 11-label dashboard, indicator config panel.

### Documentation

| Document | Purpose |
|---|---|
| [ARCHITECTURE.md](docs/ARCHITECTURE.md) | Why Rust/Tauri vs Python, Electron, Qt/C++, pure Rust GUI |
| [DESIGN_PHILOSOPHY.md](docs/DESIGN_PHILOSOPHY.md) | Core design principles (API efficiency, visual accuracy, security) |
| [INDICATOR_PORTING.md](docs/INDICATOR_PORTING.md) | Lessons learned porting MQL5 indicators to JavaScript |
| [docs/adr/](docs/adr/) | 31 Architecture Decision Records |

### ADR Index

| ADR | Topic |
|---|---|
| [001](docs/adr/001-rust-tauri-architecture.md) | Rust + Tauri architecture decision |
| [002](docs/adr/002-lightweight-charts.md) | TradingView lightweight-charts rationale |
| [003](docs/adr/003-bar-data-caching.md) | Three-tier cache (memory + IndexedDB + zstd) |
| [004](docs/adr/004-mtf-indicators.md) | Multi-timeframe indicator support |
| [005](docs/adr/005-indicator-visual-parity.md) | Indicator visual parity with MT5 |
| [006](docs/adr/006-security-hardening.md) | Security hardening (21 passes, 97 findings) |
| [007](docs/adr/007-bar-prefetch-strategy.md) | Background bar pre-fetch strategy |
| [008](docs/adr/008-multi-tab-charts.md) | Multi-tab chart support |
| [009](docs/adr/009-rate-limiter.md) | Centralized rate limiter |
| [010](docs/adr/010-multi-account.md) | Multi-account credential management |
| [011](docs/adr/011-resizable-panes.md) | Resizable chart panes |
| [012](docs/adr/012-news-earnings-dividends.md) | News, earnings, and dividend data |
| [013](docs/adr/013-auto-load-timeframe.md) | Auto-load on timeframe/bar count change |
| [014](docs/adr/014-draggable-sl-tp-lines.md) | Draggable SL/TP lines (MT5-style) |
| [015](docs/adr/015-order-management.md) | Full order management (6 types, history, cancel) |
| [016](docs/adr/016-price-alerts.md) | Price alerts with browser notifications |
| [017](docs/adr/017-drawing-tools.md) | Drawing tools (trend lines + Fibonacci) |
| [018](docs/adr/018-mql5-feature-parity.md) | MQL5 feature parity audit |
| [019](docs/adr/019-mtf-grid-view.md) | Multi-timeframe grid view (MT5-style) |
| [020](docs/adr/020-cache-optimization.md) | SQLite cache + LRU eviction |
| [021](docs/adr/021-mt5-godel-parity-roadmap.md) | MT5 + Godel parity roadmap + blockers |
| [023](docs/adr/023-ux-features-batch.md) | UX batch: GUI menu, tabs, drawing tools, trading |
| [024](docs/adr/024-charting-engine-race-conditions.md) | Charting engine race conditions — cross-symbol contamination |
| [025](docs/adr/025-new-features-batch-2.md) | Feature batch 2: NNFX strategy, options tools, sectors, auto-trading |
| [026](docs/adr/026-architecture-future.md) | Architecture: headless CLI, WebWorker/Wasm plans, Pine Script analysis |
| [027](docs/adr/027-binary-storage-wasm-gpu.md) | Binary bar storage, Wasm indicator engine, GPU chart architecture |
| [028](docs/adr/028-performance-optimization-audit.md) | Performance optimization audit |
| [029](docs/adr/029-feature-expansion-analytics-risk.md) | Feature expansion: analytics + risk |
| [030](docs/adr/030-session-persistence-hardening.md) | Session persistence hardening |
| [031](docs/adr/031-testing-framework.md) | Testing framework (602 assertions) |
| [032](docs/adr/032-gpu-drawing-tools-roadmap.md) | GPU chart completion + drawing tools parity roadmap |
| [033](docs/adr/033-free-api-expansion.md) | Free API expansion research — 30+ data sources catalogued |
| [034](docs/adr/034-cli-tui.md) | CLI / TUI terminal interface — 6.5MB standalone binary |

---

## Building

### GUI Prerequisites

- Rust (latest stable)
- Node.js 18+
- Tauri CLI: `cargo install tauri-cli`
- Linux: `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`

### GUI Quick Start (Hyprland/NVIDIA)

```bash
./launch.sh dev    # development with hot reload
./launch.sh        # production build + run
```

### CLI / TUI (no GUI required)

```bash
cd cli && ./typhoon.sh              # Interactive TUI
./typhoon.sh --positions            # Print positions and exit
./typhoon.sh --account              # Print account and exit
./typhoon.sh --accounts             # All accounts (Alpaca + MT5 imports)
./typhoon.sh -s BTC/USD             # Start with specific symbol
./typhoon.sh --import-mt5 DARWIN_EUR:/path/to/statement.csv
```

The CLI shares encrypted credentials with the GUI — no need to re-enter API keys. 6.5MB standalone binary, works over SSH on any VPS.

| CLI Feature | Command |
|---|---|
| Market buy/sell | `:buy SMCI 100` / `:sell SLV 50` |
| Limit order | `:limit buy AAPL 10 150.00` |
| Stop order | `:stop sell SMCI 100 25.00` |
| Bracket order | `:bracket buy CC 500 15.00 25.00` |
| Close position | `:close CC` or `x` on selected |
| Partial close | `p` on selected (50%) |
| Close all | `:closeall` |
| Cancel all orders | `:cancelall` |
| Order history | `:history 20` |
| Chart symbol | `:chart BTC/USD H4` |
| Import MT5 | `:import DARWIN_EUR /path.csv` |

The launch script sets the required environment variables for WebKitGTK on Hyprland with NVIDIA:
- `WEBKIT_DISABLE_DMABUF_RENDERER=1` — prevents DMABUF crash
- `WEBKIT_DISABLE_COMPOSITING_MODE=1` — fixes blank window
- `GDK_BACKEND=x11` — forces XWayland backend (most stable)

### Manual Development

```bash
cd frontend && npm install
cd ../src-tauri && cargo tauri dev
```

### Production Build

```bash
cargo tauri build
```

---

## Broker

**Alpaca Markets** — stocks, ETFs, options, crypto. Paper and live trading via REST API + WebSocket streaming. IEX (free) or SIP (paid) market data.

---

## License

[Business Source License 1.1](LICENSE)

---

## Disclaimer

This software is provided for educational and research purposes. Trading involves risk. Past performance does not guarantee future results. Always test with paper trading before using real funds.
