# TyphooN-Terminal

A native desktop trading terminal with full risk management, multi-timeframe charting, and hedged martingale support — built in Rust/Tauri for Alpaca Markets.

**Website:** [MarketWizardry.org](https://www.marketwizardry.org/)

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
| **Drawing Tools** | Trend lines, Fibonacci, horizontal lines, rectangles, channels |
| **Multi-Chart Layouts** | Split view + MTF grid (2-5 timeframes with Fisher/BetterVolume per cell) |
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
| **Multi-Broker Trait** | BrokerTrait abstraction for future broker support |
| **Multi-Account** | Save/load multiple Alpaca accounts (paper + live), OS keychain credential storage (gnome-keyring/KWallet) |
| **Indicators** | 30 indicators: NNFX system (9) + standard (11) + extended (Stochastic, CCI, ADX, Williams %R, Ichimoku Cloud, Parabolic SAR, OBV, Momentum, WMA, HMA) |
| **Security** | 16-pass audit (72 findings): input validation, HTTP timeouts, path traversal, CSP, config bounds, resource limits, OS keychain, zeroize |

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
| [ARCHITECTURE.md](ARCHITECTURE.md) | Why Rust/Tauri vs Python, Electron, Qt/C++, pure Rust GUI |
| [DESIGN_PHILOSOPHY.md](DESIGN_PHILOSOPHY.md) | Core design principles (API efficiency, visual accuracy, security) |
| [INDICATOR_PORTING.md](INDICATOR_PORTING.md) | Lessons learned porting MQL5 indicators to JavaScript |
| [docs/adr/](docs/adr/) | 22 Architecture Decision Records |

### ADR Index

| ADR | Topic |
|---|---|
| [001](docs/adr/001-rust-tauri-architecture.md) | Rust + Tauri architecture decision |
| [002](docs/adr/002-lightweight-charts.md) | TradingView lightweight-charts rationale |
| [003](docs/adr/003-bar-data-caching.md) | Three-tier cache (memory + IndexedDB + zstd) |
| [004](docs/adr/004-mtf-indicators.md) | Multi-timeframe indicator support |
| [005](docs/adr/005-indicator-visual-parity.md) | Indicator visual parity with MT5 |
| [006](docs/adr/006-security-hardening.md) | Security hardening (6 passes, 50 findings) |
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
| [022](docs/adr/022-tastytrade-broker.md) | Tastytrade broker integration |

---

## Building

### Prerequisites

- Rust (latest stable)
- Node.js 18+
- Tauri CLI: `cargo install tauri-cli`
- Linux: `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`

### Quick Start (Hyprland/NVIDIA)

```bash
./launch.sh dev    # development with hot reload
./launch.sh        # production build + run
```

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

Supports two brokers:

### Alpaca Markets
- Paper and live trading accounts
- REST API for orders, positions, account info
- Historical bar data with IEX/SIP feed support
- WebSocket streaming for real-time trades/quotes
- Options chain with full Greeks

### Tastytrade
- Paper (sandbox) and live trading
- Stocks, options, futures, crypto
- Session-based auth (username/password)
- Account balances, positions, market orders
- Sign up: https://www.tastytrade.com/

---

## License

GNU General Public License v3.0

---

## Disclaimer

This software is provided for educational and research purposes. Trading involves risk. Past performance does not guarantee future results. Always test with paper trading before using real funds.
