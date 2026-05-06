# TyphooN-Terminal

A native desktop trading terminal + TUI CLI with full risk management, multi-timeframe charting, and hedged martingale support — built in pure Rust with native GPU rendering (egui + wgpu) for MT5/Darwinex, Alpaca, tastytrade, Kraken, and CryptoCompare-backed market data.

**Website:** [MarketWizardry.org](https://www.marketwizardry.org/) | **License:** [BSL 1.1](LICENSE) ([Commercial](LICENSE-COMMERCIAL))

## At a Glance

| Metric | Value |
|---|---|
| **GUI Binary** | ~25MB native (egui + wgpu) |
| **CLI Binary** | 6.5MB standalone TUI (SSH/VPS ready) |
| **Memory Usage** | ~50-100MB (vs thinkorswim ~2GB+) |
| **Startup Time** | < 2 seconds |
| **Lines of Code** | 170K+ native GUI + 135K+ engine/research (pure Rust) |
| **Indicators** | 46+ chart indicators plus ~375 TA-Lib/Godel research surfaces |
| **Commands** | 260+ Quake-console style (~) |
| **Drawing Tools** | 89 drawing and annotation types |
| **Harmonic Patterns** | 10 (Gartley, Butterfly, Bat, Crab, Shark, Cypher, 5-0, Alt Bat, Deep Crab, Three Drives) |
| **Chart Types** | 5 (Candle, Heikin-Ashi, Line, OHLC Bars, Renko) |
| **Data Sources** | MT5 (Darwinex), Alpaca, tastytrade, Kraken Spot/xStocks, Kraken Futures, CryptoCompare |
| **DARWIN Analytics** | 88 functions wired (VaR, correlation, equity, streaks, Monte Carlo, stress tests, rebalance, floating equity, D-Score, tax lots, CAGR, recovery factor, divergence index, risk budget, replication quality, performance attribution) |
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
| **Backtester** | 5 strategies (SMA Cross, NNFX, KAMA Cross, Fisher Cross, RSI Mean-Rev), equity curve, trade reports (Sharpe, drawdown, profit factor) |
| **WebSocket Streaming** | Real-time trades/quotes via Alpaca WebSocket, Time & Sales |
| **Options Chain** | Full Greeks, strike/expiry/bid/ask via Alpaca options API |
| **Stock Screener** | Filter by price, volume, sector, change%, tradable/shortable flags |
| **Command Palette** | ~ (tilde) Quake-console: DARWIN, BACKTEST, RISK_CALC, SCREENER, EXPORT_CSV |
| **Watchlist** | Multi-symbol quote monitor with live prices and daily change |
| **LAN Sync** | TLS/PBKDF2 LAN sync plus headless server/client deployment |
| **Storage Manager** | View, delete, compact (zstd-22), and schedule idle auto-compaction by symbol/source |
| **Multi-Window** | Open additional terminal windows (NEW_WINDOW/POPOUT) for multi-monitor setups |
| **Chart Templates** | Save/load indicator configs and order mode |
| **Workspace Profiles** | Save/load entire layout (tabs, indicators, pane sizes) |
| **Drawing Tools** | 89 types: trend, fib, ray, ruler, rectangle, channel, pitchfork, Elliott, Gann, regression, arrows, labels + GPU rendering |
| **Multi-Chart Layouts** | MTF grid (2-5 timeframes with full NNFX indicators per cell) |
| **Screenshot Export** | Ctrl+Shift+S to clipboard with toast notification |
| **Push Notifications** | Pushover + ntfy.sh for mobile alerts |
| **CSV Export** | Export trade history as CSV |
| **Chart Types** | Candlestick, Heikin-Ashi, Line, OHLC Bar, and Renko chart rendering |
| **Risk/Reward Overlay** | Visual profit/loss zones on chart when SL/TP lines set |
| **Trade Journal** | Log trades with notes, review history (~ →JOURNAL) |
| **Position Calculator** | Risk-based sizing with R:R ratio (~ →CALC) |
| **Chart Annotations** | Text markers on chart bars (~ →ANNOTATE) |
| **Regime Detection** | ADX-based trending/ranging/choppy state in dashboard |
| **Pattern Recognition** | Harmonic pattern auto-detection plus manual H&S/triangle/pattern annotation tools; classic double-top/H&S auto-detection is deferred |
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
| **Headless LAN** | `--lan-server` / `--lan-client` reuse the GUI cache/passphrase and expose Prometheus metrics for VPS/NAS deployment |
| **CLI/TUI** | Interactive SSH-ready TUI plus positions/account/import/LAN commands; strategy backtests run in the native GUI |
| **Community Chat** | Matrix protocol chat via ~ (tilde) → CHAT, no server needed |
| **Broker Abstraction** | BrokerTrait — extensible to any broker via single Rust file |
| **Multi-Account** | Save/load multiple Alpaca accounts (paper + live), OS-native keyring credential storage |
| **Indicators** | 46+ chart indicators: NNFX system, standard overlays/oscillators, Ehlers DSP, MTF overlays, volume/supply-demand, and GPU/CPU parity paths |
| **Security** | 21-pass audit (97 findings): OS-native keyring credentials, input validation, HTTP timeouts, path traversal, CSP, config bounds, zeroize, async lock optimization |
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
| **Economic Calendar** | Finnhub economic events: FOMC, NFP, CPI, PMI with impact ratings (~ →ECON) |
| **Kraken Primary Market Data** | Public Spot/xStocks universe sync with no API key; async OHLCV queueing paced to Kraken's public limits with cooldown |
| **Kraken Futures Market Data** | Public futures instrument discovery + async OHLCV sync under `kraken-futures:SYMBOL:TF`; no API key needed |
| **CryptoCompare Deep History** | BTC from 2010, ETH from 2015, 2000 bars/request — extends history before exchange listings where available |
| **Weekend Crypto Live** | Adaptive polling: 60s (M1), 2.5min (M15), 5min (H1+) — magenta-colored weekend candles |
| **Chart Right Margin** | 5-bar right margin (MT5 chart shift style) for price action breathing room |
| **Unusual Volume Scanner** | Detect abnormal volume spikes across symbols (~ →UNUSUAL_VOLUME) |
| **Multi-Signal Anomaly Scanner** | 4-dimensional scan: VaR + EV + ATR + SEC with tradability indicators (~ →ANOMALY) |
| **MTF Grid Visibility** | Per-tab checkboxes to show/hide individual timeframes in multi-timeframe grid |
| **Storage Pagination** | Paginated storage manager for large cache databases |
| **260+ Commands** | Quake-console command palette with fuzzy search |

---

## NNFX Indicator System

Ported from the [MQL5-NNFX-Risk_Management_System](https://github.com/TyphooN-/MQL5-NNFX-Risk_Management_System) with exact visual parity.

### Main Chart (enabled by default)

| Indicator | Source | Description |
|---|---|---|
| **MultiKAMA** (10/2/30) | KAMA.mqh / MultiKAMA.mqh | Kaufman Adaptive MA from multiple timeframes, white, width 2 |
| **200 SMA** | Standard | Yellow, width 1 |
| **MTF SMA** | Standard | H1/H4/D1/W1 200SMA + W1/MN1 100SMA — Tomato (H1), Magenta (H4+) |
| **Previous Candle Levels** | PreviousCandleLevels.mqh | MTF previous bar high/low — H1/H4/D1/W1/MN1 |
| **ATR Projection MTF** (14) | ATR_Projection.mqh | MTF open ± ATR bands (M15/H1/H4/D1/W1/MN1) — solid yellow, width 2 |
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

28 unit tests covering margin math, lot sizing, and VaR calculations.

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
| [INDICATORS.md](docs/INDICATORS.md) | All 46+ chart indicators with parameters and colors |
| [KEYBOARD_SHORTCUTS.md](docs/KEYBOARD_SHORTCUTS.md) | Keybindings, commands, menu reference |
| [PERFORMANCE.md](docs/PERFORMANCE.md) | Benchmarks, data pipeline timing, cache format |
| [ROADMAP.md](docs/ROADMAP.md) | Current status and future plans |
| [DESIGN_PHILOSOPHY.md](docs/DESIGN_PHILOSOPHY.md) | Core design principles |
| [API_KEYS.md](docs/API_KEYS.md) | Data source API key setup |
| [deployment/lan-server.md](docs/deployment/lan-server.md) | Docker, Kubernetes, Terraform, Ansible, Prometheus, Grafana, and Kafka LAN server deployment |
| [docs/adr/](docs/adr/) | Architecture Decision Records |

### ADR Index

| ADR | Topic |
|---|---|
| [001](docs/adr/001-native-gpu-architecture.md) | Native GPU architecture (egui + wgpu, tokio async broker) |
| [002](docs/adr/002-chart-engine.md) | Chart engine (5 types, egui Painter, egui_plot) |
| [003](docs/adr/003-sqlite-cache.md) | SQLite + zstd TTBR binary cache |
| [004](docs/adr/004-mtf-indicators.md) | Multi-timeframe indicator support |
| [005](docs/adr/005-indicator-visual-parity.md) | Indicator visual parity with MT5 |
| [006](docs/adr/006-security.md) | Security (no WebView, parameterized SQL, OS keyring) |
| [008](docs/adr/008-multi-tab-charts.md) | Multi-tab charts (Ctrl+N/W/Tab) |
| [009](docs/adr/009-rate-limiter.md) | Centralized rate limiter |
| [010](docs/adr/010-multi-broker.md) | Multi-broker (Alpaca + tastytrade + MT5 view-only) |
| [011](docs/adr/011-indicator-system.md) | 32+ indicators (NNFX + Ehlers + standard + harmonics) |
| [012](docs/adr/012-news-earnings-dividends.md) | News, earnings, and dividend data |
| [013](docs/adr/013-auto-load-timeframe.md) | Auto-load on timeframe change |
| [014](docs/adr/014-sl-tp-lines.md) | SL/TP planning lines on chart |
| [015](docs/adr/015-order-management.md) | Order management (Market/Limit/Stop/Bracket, async) |
| [016](docs/adr/016-price-alerts.md) | Price alerts (session persistent) |
| [017](docs/adr/017-drawing-tools.md) | 7 drawing tools with color picker |
| [019](docs/adr/019-mtf-grid.md) | Multi-timeframe 4-cell grid |
| [022](docs/adr/022-tastytrade-broker.md) | tastytrade broker integration |
| [032](docs/adr/032-ehlers-dsp-indicators.md) | 8 Ehlers DSP indicators |
| [033](docs/adr/033-free-api-expansion.md) | Free API expansion — 30+ data sources |
| [037](docs/adr/037-data-source-hierarchy.md) | Data source hierarchy (MT5 → Broker → CryptoCompare → Kraken → Kraken Futures) |
| [038](docs/adr/038-data-source-indicator.md) | Data source indicator UI |
| [040](docs/adr/040-crypto-data-source.md) | Crypto data sources (CryptoCompare + Kraken gap-fill) |
| [041](docs/adr/041-darwin-import-analytics.md) | DARWIN import pipeline & analytics engine |
| [044](docs/adr/044-backup-lan-sync.md) | Backup & LAN sync |
| [045](docs/adr/045-darwin-analytics-expansion.md) | DARWIN analytics expansion |
| [048](docs/adr/ADR-048-bookmap-depth-heatmap.md) | Bookmap depth heatmap |
| [049](docs/adr/049-harmonic-pattern-detection.md) | Scott Carney harmonic pattern detection |
| [072](docs/adr/072-kraken-broker.md) | Kraken full broker integration (Spot REST trading + public Spot/xStocks/Futures market data) |
| [050](docs/adr/050-gpu-compute-architecture.md) | GPU compute architecture (28 wgpu compute shaders) |
| [051](docs/adr/051-dependency-alignment.md) | Dependency version alignment |
| [052](docs/adr/052-performance-architecture.md) | Performance architecture |
| [053](docs/adr/053-background-data-channels.md) | Background data channels |
| [054](docs/adr/054-fundamentals-engine.md) | Fundamentals engine |
| [055](docs/adr/055-gpu-darwin-analytics.md) | GPU DARWIN analytics |
| [056](docs/adr/056-data-pipelines.md) | Data pipelines |
| [057](docs/adr/057-symbol-specs-tracking.md) | Symbol specs tracking |
| [058](docs/adr/058-gpu-strategy-optimizer.md) | GPU strategy optimizer |
| [059](docs/adr/059-security-by-design.md) | Security by design (credential & data protection) |
| [060](docs/adr/060-mql5-compiler-pipeline.md) | MQL5 compiler pipeline |
| [206](docs/adr/206-headless-lan-server-deployment.md) | Headless LAN server deployment |
| [207](docs/adr/207-encrypted-cache-at-rest.md) | Password-encrypted cache at rest |
| [208](docs/adr/208-xynth-feature-parity.md) | Xynth feature parity target |
| [209](docs/adr/209-lan-observability-kafka.md) | LAN observability and Kafka deployment |
| [210](docs/adr/210-kraken-async-bar-sync.md) | Kraken async bar sync acceleration |
| [211](docs/adr/211-kraken-rate-limit-cooldown.md) | Kraken rate-limit pacing and cooldown |
| [212](docs/adr/212-ai-return-path-auto-ingest.md) | AI Return Path auto-ingest |

---

## Competitive Comparison

| Feature | TyphooN Terminal | OpenBB | Godel | UnusualWhales | TradingView |
|---------|-----------------|--------|-------|---------------|-------------|
| **Native GPU Rendering** | Yes (wgpu) | No (Python) | No (Web) | No (Web) | No (Web) |
| **Trading Execution** | 3 brokers | No | No | No | 1 broker |
| **DARWIN Analytics** | 88 functions | No | No | No | No |
| **MQL5 Compiler** | Yes | No | No | No | PineScript |
| **FRED Economic Data** | Yes | Yes | No | No | No |
| **SEC Filings** | Yes | Yes | Yes | No | No |
| **Congressional Trades** | Yes | Yes | No | Yes | No |
| **Harmonic Patterns** | 10 Carney | No | No | No | Community |
| **Walk-Forward Optimizer** | Yes (5 strategies) | No | No | No | No |
| **Deep Crypto History** | 2010+ (CryptoCompare) | Yes | No | No | Yes |
| **Weekend Crypto Live** | Yes (60s polling) | No | No | No | Yes |
| **Anomaly Scanner** | 4-dim (VaR+EV+ATR+SEC) | No | No | Options only | No |
| **Storage Cost** | Free (local SQLite) | Free | $80-118/mo | $30-60/mo | $0-60/mo |
| **Open Source** | BSL 1.1 | AGPL | No | No | No |

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
./typhoon.sh --export-cache backup.typhoon-backup --cache-backup-passphrase "$PASS"
./typhoon.sh --import-cache backup.typhoon-backup --cache-backup-passphrase "$PASS"
./typhoon.sh --lan-server --cache-dir /mnt/nas/typhoon-cache
./typhoon.sh --lan-client 192.168.1.20
```

The CLI shares encrypted credentials with the GUI — no need to re-enter API keys. 6.5MB standalone binary, works over SSH on any VPS.

CLI LAN server/client mode uses the same encrypted LAN sync protocol, saved LAN passphrase, and `typhoon_cache.db` cache as the GUI. Headless mode also exposes Prometheus metrics with `--metrics-port`. For Docker, Kubernetes, Terraform, Ansible, Grafana, Prometheus, and Kafka examples with a user-provided local or NAS cache path, see [LAN server deployment](docs/deployment/lan-server.md).

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
| Cache backup | `--export-cache PATH` / `--import-cache PATH` |

---

## Brokers

**Alpaca Markets** — stocks, ETFs, options, and crypto via REST + WebSocket streaming. IEX (free) or SIP (paid) market data.

**tastytrade** — account, positions, orders, option chains, quote snapshots, market metrics, and DXLink historical bars.

**Kraken** — public Spot/xStocks and Futures market data without keys, plus authenticated Spot REST trading for crypto/xStocks accounts.

---

## License

[Business Source License 1.1](LICENSE)

---

## Disclaimer

This software is provided for educational and research purposes. Trading involves risk. Past performance does not guarantee future results. Always test with paper trading before using real funds.
