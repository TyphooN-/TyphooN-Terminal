# TyphooN-Terminal

A native desktop trading terminal + TUI CLI with full risk management, multi-timeframe charting, and hedged martingale support — built in pure Rust with native GPU rendering (egui + wgpu) for Alpaca Markets.

**Website:** [MarketWizardry.org](https://www.marketwizardry.org/) | **License:** [BSL 1.1](LICENSE) ([Commercial](LICENSE-COMMERCIAL))

## At a Glance

| Metric | Value |
|---|---|
| **GUI Binary** | ~25MB native (egui + wgpu) |
| **CLI Binary** | 6.5MB standalone TUI (SSH/VPS ready) |
| **Memory Usage** | ~50-100MB (vs thinkorswim ~2GB+) |
| **Startup Time** | < 2 seconds |
| **Lines of Code** | ~6,600 GUI + ~12,000 engine (pure Rust) |
| **Indicators** | 32+ (NNFX + Ehlers DSP + standard + harmonics) |
| **Commands** | 95 Quake-console style (~) |
| **Drawing Tools** | 7 types (HLine, Trendline, Fibonacci, VLine, Rectangle, Ray, Channel) |
| **Harmonic Patterns** | 7 (Gartley, Butterfly, Bat, Crab, Shark, Cypher, 5-0) |
| **Chart Types** | 5 (Candle, Heikin-Ashi, Line, OHLC Bars, Renko) |
| **Data Sources** | MT5 (Darwinex), Alpaca, Kraken, tastytrade |
| **DARWIN Analytics** | 70+ functions (VaR, correlation, equity curves, streaks) |
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
| **Command Palette** | ~ (tilde) Quake-console: DARWIN, BACKTEST, RISK_CALC, SCREENER, EXPORT_CSV |
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
| **Trade Journal** | Log trades with notes, review history (~ →JOURNAL) |
| **Position Calculator** | Risk-based sizing with R:R ratio (~ →CALC) |
| **Chart Annotations** | Text markers on chart bars (~ →ANNOTATE) |
| **Regime Detection** | ADX-based trending/ranging/choppy state in dashboard |
| **Pattern Recognition** | Auto-detect double top/bottom, head & shoulders (~ →PATTERNS) |
| **Sentiment Analysis** | Keyword-based bullish/bearish scoring from news (~ →SENTIMENT) |
| **Volatility Surface** | Options IV heatmap by strike×expiry (~ →VOLSURF) |
| **Portfolio Heat Map** | Finviz-style colored boxes by P&L (~ →HEATMAP) |
| **Bracket Order UI** | Visual OCO/bracket order placement (~ →BRACKET) |
| **Alert Dashboard** | Cross-watchlist alert monitoring (~ →ALERTBOARD) |
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
| **Options P&L Calc** | Multi-leg payoff diagram with canvas rendering (~ →OPTCALC) |
| **Sector Rotation** | S&P 500 sector ETF heatmap with daily/weekly % (~ →SECTORS) |
| **Options Strategy** | Live chain viewer, presets (spreads, condors), aggregate Greeks (~ →OPTSTRAT) |
| **Auto-Trading** | JS plugin → live order execution, paper-only safety (~ →AUTOTRADE) |
| **Headless CLI** | `--backtest` mode: run strategies from command line, no GUI needed (VPS/SSH) |
| **Community Chat** | Matrix protocol chat via ~ (tilde) → CHAT, no server needed |
| **Broker Abstraction** | BrokerTrait — extensible to any broker via single Rust file |
| **Multi-Account** | Save/load multiple Alpaca accounts (paper + live), AES-256-GCM encrypted credential storage |
| **Indicators** | 39 indicators: NNFX system (9) + standard (21) + MT5 parity (9: Alligator, AO, MFI, Force Index, Envelopes, StdDev, Chaikin, DeMarker, Fractals) |
| **Security** | 21-pass audit (97 findings): AES-256-GCM credential encryption, input validation, HTTP timeouts, path traversal, CSP, config bounds, zeroize, async lock optimization |
| **Analyst Ratings** | Finnhub consensus: stacked buy/hold/sell chart + price targets (~ →ANR) |
| **Fear & Greed** | Market sentiment gauge (0-100) + 30-day sparkline (~ →FEAR) |
| **Dark Pool** | FINRA RegSHO daily short volume with gauge visualization (~ →DARKPOOL) |
| **Congress Trading** | House Stock Watcher: congressional trades filterable by symbol/rep/party (~ →CONGRESS) |
| **Earnings Overlay** | Toggle E/D/S markers on chart for earnings, dividends, splits (~ →EARNINGS-OVERLAY) |
| **World Indices** | 14 major indices across Americas/Europe/Asia-Pacific, auto-refresh (~ →WEI) |
| **Forex Dashboard** | ECB rates + 6×6 cross rate matrix (~ →FX) |
| **Crypto Market** | CoinGecko top 50 + trending + 7-day sparklines (~ →CRYPTO) |
| **Yield Curve** | Treasury rates with 2Y-10Y inversion detection (~ →YIELD) |
| **GPU Chart Engine** | Native wgpu candlesticks, drawing tools, sub-panes, price lines, histograms, fills — all on GPU |
| **Draggable Panel Splitter** | Resize chart/sidebar panels by dragging the divider — layout persists across sessions |
| **50+ Commands** | Quake-console command palette with fuzzy search |

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

**Pure Rust native GPU renderer** — egui + wgpu (Vulkan/Metal/DX12).

Direct memory path: SQLite cache → zstd decompress → `&[f64]` OHLCV → wgpu vertex buffer → GPU renders candlesticks + indicators.

### Documentation

| Document | Purpose |
|---|---|
| [ARCHITECTURE.md](docs/ARCHITECTURE.md) | Native GPU architecture, data flow, project structure |
| [INDICATORS.md](docs/INDICATORS.md) | All 21 indicators with parameters and colors |
| [KEYBOARD_SHORTCUTS.md](docs/KEYBOARD_SHORTCUTS.md) | Keybindings, commands, menu reference |
| [PERFORMANCE.md](docs/PERFORMANCE.md) | Benchmarks, data pipeline timing, cache format |
| [ROADMAP.md](docs/ROADMAP.md) | Current status and future plans |
| [DESIGN_PHILOSOPHY.md](docs/DESIGN_PHILOSOPHY.md) | Core design principles |
| [API_KEYS.md](docs/API_KEYS.md) | Data source API key setup |
| [docs/adr/](docs/adr/) | Architecture Decision Records |

### ADR Index

| ADR | Topic |
|---|---|
| [004](docs/adr/004-mtf-indicators.md) | Multi-timeframe indicator support |
| [005](docs/adr/005-indicator-visual-parity.md) | Indicator visual parity with MT5 |
| [009](docs/adr/009-rate-limiter.md) | Centralized rate limiter |
| [012](docs/adr/012-news-earnings-dividends.md) | News, earnings, and dividend data |
| [013](docs/adr/013-auto-load-timeframe.md) | Auto-load on timeframe change |
| [022](docs/adr/022-tastytrade-broker.md) | tastytrade broker integration |
| [033](docs/adr/033-free-api-expansion.md) | Free API expansion — 30+ data sources |
| [037](docs/adr/037-data-source-hierarchy.md) | Data source hierarchy (MT5 → Kraken → Alpaca) |
| [038](docs/adr/038-data-source-indicator.md) | Data source indicator UI |
| [040](docs/adr/040-crypto-data-source.md) | Crypto data sources (Kraken gap-fill) |
| [041](docs/adr/041-darwin-import-analytics.md) | DARWIN import pipeline & analytics engine |
| [044](docs/adr/044-backup-lan-sync.md) | Backup & LAN sync |
| [045](docs/adr/045-darwin-analytics-expansion.md) | DARWIN analytics expansion (VaR, drawdown, rebalancer) |
| [048](docs/adr/ADR-048-bookmap-depth-heatmap.md) | Bookmap-style depth heatmap (future) |
| [049](docs/adr/049-harmonic-pattern-detection.md) | Scott Carney harmonic pattern detection |

---

## Building

### Prerequisites

- Rust (latest stable, edition 2024)
- Linux: Vulkan drivers (NVIDIA/AMD/Intel)

### Native GUI

```bash
cd native && cargo run          # development
cd native && cargo build --release  # production
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

---

## Broker

**Alpaca Markets** — stocks, ETFs, options, crypto. Paper and live trading via REST API + WebSocket streaming. IEX (free) or SIP (paid) market data.

---

## License

[Business Source License 1.1](LICENSE)

---

## Disclaimer

This software is provided for educational and research purposes. Trading involves risk. Past performance does not guarantee future results. Always test with paper trading before using real funds.
