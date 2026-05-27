# TyphooN-Terminal

A native desktop trading terminal + TUI CLI with full risk management, multi-timeframe charting, and hedged martingale support — built in pure Rust with native GPU rendering (egui + wgpu) for MT5/Darwinex, Alpaca, tastytrade, Kraken, and CryptoCompare-backed market data.

**License:** [BUSL 1.1](LICENSE) ([Commercial](COMMERCIAL.md))

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
| **Cost** | Free for personal use ([commercial licensing](COMMERCIAL.md) available) |

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
| [001](docs/adr/001-native-gpu-architecture.md) | Native GPU Architecture |
| [002](docs/adr/002-chart-engine.md) | Chart Engine |
| [003](docs/adr/003-sqlite-bar-cache.md) | SQLite Bar Cache |
| [004](docs/adr/004-multi-timeframe-indicator-support.md) | Multi-Timeframe Indicator Support |
| [005](docs/adr/005-indicator-visual-parity-with-mt5.md) | Indicator Visual Parity with MT5 |
| [006](docs/adr/006-security-model.md) | Security Model |
| [007](docs/adr/007-multi-tab-charts.md) | Multi-Tab Charts |
| [008](docs/adr/008-centralized-rate-limiter.md) | Centralized Rate Limiter |
| [009](docs/adr/009-multi-broker-architecture.md) | Multi-Broker Architecture |
| [010](docs/adr/010-indicator-system-32-indicators.md) | Indicator System (32+ Indicators) |
| [011](docs/adr/011-news-earnings-and-dividend-data.md) | News, Earnings, and Dividend Data |
| [012](docs/adr/012-auto-load-on-timeframe-bar-count-change.md) | Auto-Load on Timeframe/Bar Count Change |
| [013](docs/adr/013-sl-tp-planning-lines.md) | SL/TP Planning Lines |
| [014](docs/adr/014-order-management.md) | Order Management |
| [015](docs/adr/015-price-alerts.md) | Price Alerts |
| [016](docs/adr/016-drawing-tools.md) | Drawing Tools |
| [017](docs/adr/017-multi-timeframe-grid.md) | Multi-Timeframe Grid |
| [018](docs/adr/018-tastytrade-broker-integration.md) | Tastytrade Broker Integration |
| [019](docs/adr/019-ehlers-dsp-indicators.md) | Ehlers DSP Indicators |
| [020](docs/adr/020-free-api-expansion-data-sources-research.md) | Free API Expansion — Data Sources Research |
| [021](docs/adr/021-data-source-hierarchy-mt5-master-broker-fallback.md) | Data Source Hierarchy — MT5 Master, Broker Fallback |
| [022](docs/adr/022-data-source-indicator-and-pluggable-broker-hierarchy.md) | Data Source Indicator & Pluggable Broker Hierarchy |
| [023](docs/adr/023-crypto-data-source-hierarchy-cryptocompare-kraken.md) | Crypto Data Source Hierarchy — CryptoCompare + Kraken |
| [024](docs/adr/024-darwin-import-pipeline-and-analytics-engine.md) | DARWIN Import Pipeline & Analytics Engine |
| [025](docs/adr/025-backup-lan-sync-system.md) | Backup + LAN Sync System |
| [026](docs/adr/026-darwin-analytics-expansion.md) | DARWIN Analytics Expansion |
| [027](docs/adr/027-bookmap-style-depth-heatmap.md) | Bookmap-Style Depth Heatmap |
| [028](docs/adr/028-scott-carney-harmonic-pattern-detection.md) | Scott Carney Harmonic Pattern Detection |
| [029](docs/adr/029-broker-market-data-sync-scheduler-lifecycle.md) | Broker market-data sync scheduler lifecycle |
| [030](docs/adr/030-gpu-compute-architecture-wgpu-compute-shaders-for-all-numerical-work.md) | GPU Compute Architecture — wgpu Compute Shaders for All Numerical Work |
| [031](docs/adr/031-dependency-version-alignment.md) | Dependency Version Alignment |
| [032](docs/adr/032-performance-architecture-background-data-render-decoupling.md) | Performance Architecture — Background Data + Render Decoupling |
| [033](docs/adr/033-background-data-channels-zero-db-queries-on-ui-thread.md) | Background Data Channels (Zero DB Queries on UI Thread) |
| [034](docs/adr/034-fundamentals-engine-enterprise-value-earnings-dividends.md) | Fundamentals Engine (Enterprise Value, Earnings, Dividends) |
| [035](docs/adr/035-gpu-accelerated-darwin-analytics.md) | GPU-Accelerated DARWIN Analytics |
| [036](docs/adr/036-data-pipelines-and-rendering-architecture.md) | Data Pipelines & Rendering Architecture |
| [037](docs/adr/037-symbol-specs-tracking-and-radar-export.md) | Symbol Specs Tracking & Radar Export |
| [038](docs/adr/038-gpu-strategy-optimizer-and-mql5-export-pipeline.md) | GPU Strategy Optimizer & MQL5 Export Pipeline |
| [039](docs/adr/039-security-by-design-credential-and-data-protection.md) | Security by Design — Credential & Data Protection |
| [040](docs/adr/040-mql5-compiler-pipeline-source-to-gpu-cpu-execution.md) | MQL5 Compiler Pipeline — Source to GPU/CPU Execution |
| [041](docs/adr/041-gpu-cpu-indicator-audit-parity-verification.md) | GPU/CPU Indicator Audit — Parity Verification |
| [042](docs/adr/042-darwin-trade-overlay-chart-arrows-position-lines.md) | DARWIN Trade Overlay — Chart Arrows + Position Lines |
| [043](docs/adr/043-bettervolume-full-mql5-port-emini-watch-algorithm.md) | BetterVolume — Full MQL5 Port (Emini-Watch Algorithm) |
| [044](docs/adr/044-performance-and-security-audit.md) | Performance & Security Audit |
| [045](docs/adr/045-lan-sync-tls-encryption.md) | LAN Sync TLS Encryption |
| [046](docs/adr/046-lan-sync-remote-request-forwarding.md) | LAN Sync Remote Request Forwarding |
| [047](docs/adr/047-feature-audit-and-gap-closure-record.md) | Feature Audit and Gap-Closure Record |
| [048](docs/adr/048-drawing-tools-and-ux-parity-with-tradingview.md) | Drawing Tools & UX Parity with TradingView |
| [049](docs/adr/049-lan-lite-client-mode-on-demand-bar-sync.md) | LAN-Lite Client Mode (On-Demand Bar Sync) |
| [050](docs/adr/050-broker-bar-data-provider-maximum-history.md) | Broker Bar Data — Provider-Maximum History |
| [051](docs/adr/051-kraken-as-full-broker-data-trading.md) | Kraken as Full Broker (Data + Trading) |
| [052](docs/adr/052-wasm-web-client-for-phone-access.md) | WASM Web Client for Phone Access |
| [053](docs/adr/053-notification-system-discord-pushover-ntfy-sh.md) | Notification System — Discord, Pushover, ntfy.sh |
| [054](docs/adr/054-swapharvester-financing-cost-screening-for-carry-strategies.md) | SwapHarvester — Financing Cost Screening for Carry Strategies |
| [055](docs/adr/055-darwinexradar-symbol-spec-change-tracking-and-export.md) | DarwinexRadar — Symbol Spec Change Tracking & Export |
| [056](docs/adr/056-screener-framework-ev-fundamentals-and-signal-scanning.md) | Screener Framework — EV, Fundamentals, and Signal Scanning |
| [057](docs/adr/057-yahoo-finance-extended-hours-watchlist.md) | Yahoo Finance Extended Hours Watchlist |
| [058](docs/adr/058-lan-sync-bandwidth-optimization.md) | LAN Sync Bandwidth Optimization |
| [059](docs/adr/059-ssd-write-reduction-strategy.md) | SSD Write Reduction Strategy |
| [060](docs/adr/060-optimization-roadmap-2026-04-08.md) | Optimization Roadmap (2026-04-08) |
| [061](docs/adr/061-no-unwrap-policy-production-error-handling.md) | No Unwrap Policy — Production Error Handling |
| [062](docs/adr/062-analytics-and-screening-expansion.md) | Analytics & Screening Expansion |
| [063](docs/adr/063-event-calendar-and-targeted-outlier-scanners.md) | Event Calendar & Targeted Outlier Scanners |
| [064](docs/adr/064-broker-scope-filter-forexfactory-calendar-perf-pass-unwrap-cleanup.md) | Broker Scope Filter, ForexFactory Calendar, Perf Pass, Unwrap Cleanup |
| [065](docs/adr/065-ux-pass-calendar-ui-staleness-alerts-help-order-entry-function-renames.md) | UX Pass: Calendar UI, Staleness, Alerts, Help, Order Entry, Function Renames |
| [066](docs/adr/066-easylanguage-thinkscript-compilers-phone-order-entry.md) | EasyLanguage + thinkScript Compilers, Phone Order Entry |
| [067](docs/adr/067-multi-frontend-expansion-cross-language-transpiler.md) | Multi-Frontend Expansion + Cross-Language Transpiler |
| [068](docs/adr/068-transpiler-phase-2-full-cross-language-matrix.md) | Transpiler Phase 2: Full Cross-Language Matrix |
| [069](docs/adr/069-ux-improvements-gpu-compute-expansion-and-client-parity.md) | UX Improvements, GPU Compute Expansion, and Client Parity |
| [070](docs/adr/070-darwinex-zero-live-web-scraping-via-selenium.md) | Darwinex Zero Live Web Scraping via Selenium |
| [071](docs/adr/071-gpu-parity-for-all-indicators-analytics-ux-overhaul.md) | GPU Parity for All Indicators + Analytics UX Overhaul |
| [072](docs/adr/072-o-1-hot-path-optimizations-scope-regression-fix.md) | O(1) Hot-Path Optimizations + Scope Regression Fix |
| [073](docs/adr/073-sec-filing-database-expansion.md) | SEC Filing Database Expansion |
| [074](docs/adr/074-comprehensive-performance-ux-memory-pass.md) | Comprehensive Performance / UX / Memory Pass |
| [075](docs/adr/075-full-o-1-algorithmic-optimization-pass-ux-polish.md) | Full O(1) Algorithmic Optimization Pass + UX Polish |
| [076](docs/adr/076-table-wiring-and-o-1-optimization-passes.md) | Table Wiring and O(1) Optimization Passes |
| [077](docs/adr/077-mimalloc-custom-allocator-optimal-release-profile.md) | mimalloc Custom Allocator + Optimal Release Profile |
| [078](docs/adr/078-multi-source-news-ingest-pipeline.md) | Multi-source News Ingest Pipeline |
| [079](docs/adr/079-godel-ta-lib-parity-program.md) | Godel / TA-Lib Parity Program |
| [080](docs/adr/080-web-research-ingestion-from-ai-agents-research-packet-viewer.md) | Web Research Ingestion from AI Agents + RESEARCH_PACKET Viewer |
| [081](docs/adr/081-mt5-barcachewriter-typhoon-terminal-health-check-protocol.md) | MT5 BarCacheWriter ⇆ TyphooN-Terminal health-check protocol |
| [082](docs/adr/082-ai-chat-session-persistence-resume-slash-commands.md) | AI chat session persistence + resume slash commands |
| [083](docs/adr/083-cross-client-ai-response-cache.md) | Cross-Client AI Response Cache |
| [084](docs/adr/084-options-expiration-calendar-tier-1-market-tier-2-per-symbol.md) | Options Expiration Calendar — Tier 1 Market + Tier 2 Per-Symbol |
| [085](docs/adr/085-mt5-ea-trading-flow-alignment-across-brokers.md) | MT5 EA Trading-Flow Alignment Across Brokers |
| [086](docs/adr/086-native-app-rs-module-decomposition-for-compile-speed.md) | native/app.rs Module Decomposition for Compile Speed |
| [087](docs/adr/087-alpaca-sync-autotuning-by-data-tier.md) | Alpaca Sync Autotuning by Data Tier |
| [088](docs/adr/088-dependency-audit-and-rustsec-advisory-closure.md) | Dependency Audit and RustSec Advisory Closure |
| [089](docs/adr/089-zstd-compression-level-policy-and-auto-compaction.md) | ZSTD Compression Level Policy and Auto-Compaction |
| [090](docs/adr/090-headless-lan-server-deployment.md) | Headless LAN Server Deployment |
| [091](docs/adr/091-password-encrypted-cache-at-rest.md) | Password-Encrypted Cache at Rest |
| [092](docs/adr/092-xynth-feature-parity-target.md) | Xynth Feature Parity Target |
| [093](docs/adr/093-lan-observability-and-kafka-deployment.md) | LAN Observability and Kafka Deployment |
| [094](docs/adr/094-kraken-async-bar-sync-acceleration.md) | Kraken Async Bar Sync Acceleration |
| [095](docs/adr/095-kraken-rate-limit-pacing-and-cooldown.md) | Kraken Rate-Limit Pacing and Cooldown |
| [096](docs/adr/096-ai-return-path-auto-ingest.md) | AI Return Path Auto-Ingest |
| [097](docs/adr/097-darwinex-zero-usa-equity-universe.md) | Darwinex Zero USA Equity Universe |
| [098](docs/adr/098-per-frame-o-1-discipline-in-chart-and-sync-paths.md) | Per-Frame O(1) Discipline in Chart and Sync Paths |
| [099](docs/adr/099-kraken-ws-full-universe-streaming-under-egui-responsiveness-budget.md) | Kraken WS Full-Universe Streaming Under egui Responsiveness Budget |
| [100](docs/adr/100-news-article-rendering-dom-aware-extractor-hero-images-commonmark-viewer.md) | News Article Rendering: DOM-aware extractor, hero images, CommonMark viewer; NO HTML/JS renderer |
| [101](docs/adr/101-kraken-iapi-aimd-rate-discovery-and-persistence.md) | Kraken iapi AIMD Rate Discovery and Persistence |

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
| **Open Source** | BUSL 1.1 | AGPL | No | No | No |

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
cargo run -q -p typhoon-cli -- --mcp-server  # MCP stdio server for research packets
```

The CLI shares encrypted credentials with the GUI — no need to re-enter API keys. 6.5MB standalone binary, works over SSH on any VPS.

CLI LAN server/client mode uses the same encrypted LAN sync protocol, saved LAN passphrase, and `typhoon_cache.db` cache as the GUI. Headless mode also exposes Prometheus metrics with `--metrics-port`. For Docker, Kubernetes, Terraform, Ansible, Grafana, Prometheus, and Kafka examples with a user-provided local or NAS cache path, see [LAN server deployment](docs/deployment/lan-server.md).

MCP clients can run `typhoon-cli --mcp-server` (or `cargo run -q -p typhoon-cli -- --mcp-server` from the workspace) to expose read-only TyphooN tools. The main tool is `research_packet`, which builds the same AI-facing markdown packet from the shared SQLite cache for requested symbols.

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
| MCP research packets | `--mcp-server` |

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
