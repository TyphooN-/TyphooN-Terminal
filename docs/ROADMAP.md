# TyphooN-Terminal Roadmap

## Vision

Replace both MetaTrader 5 and Godel Terminal with a single open-source desktop trading terminal. Combine MT5's modular indicator/EA architecture with Godel's data-rich command-line interface and Bloomberg-style research capabilities.

---

## Broker Support

- **Alpaca Markets** — Stocks, ETFs, Options, Crypto (free paper trading, IEX data) ✅ Done

### Architecture
Each broker implements a Rust `BrokerTrait`. Adding a new broker means one new file — no changes to risk engine, indicators, or UI.

---

## Feature Tiers

### Tier 1 — Core Trading Terminal (Current + Near-Term)

| Feature | Status | Notes |
|---|---|---|
| Candlestick charts (M1-MN) | ✅ Done | Synthetic MN from weekly bars |
| Multi-tab charts | ✅ Done | Ctrl+T/W, tab state persistence |
| 10-button panel + keyboard shortcuts | ✅ Done | Exact MT5 port |
| 4 risk modes (Standard/Fixed/Dynamic/VaR) | ✅ Done | |
| Hedged martingale (TRIM/PROTECT) | ✅ Done | Forward-looking v1.420 |
| NNFX indicators (KAMA, Fisher, ATR_Proj, etc.) | ✅ Done | Exact MQL5 ports |
| Standard indicators (RSI, MACD, Bollinger, etc.) | ✅ Done | |
| Supply/Demand zones (fractal-based) | ✅ Done | Exact SupplyDemand.mqh port |
| BetterVolume (buy/sell estimation) | ✅ Done | Exact BetterVolume.mqh port |
| Three-tier cache (memory/IndexedDB/zstd) | ✅ Done | |
| Session persistence | ✅ Done | Tabs, indicators, pane sizes |
| Multi-account management | ✅ Done | Paper + Live |
| News feed (Alpaca) | ✅ Done | In-app reading with floating windows |
| SEC fundamentals (EDGAR) | ✅ Done | Revenue, EPS, shares, etc. |
| SEC filings search | ✅ Done | Hardened: parameterized queries |
| Auto-load on timeframe change | ✅ Done | No "Load" button needed |
| Security hardening (21 passes) | ✅ Done | 97 findings: input validation, timeouts, path traversal, CSP, config bounds, resource limits, event listener cleanup |
| MTF MA grid | ✅ Done | SMA200/KAMA/Fisher across TFs |
| Symbol autocomplete | ✅ Done | 11K+ symbols |
| Rate limiter (v4 adaptive) | ✅ Done | Adaptive pacing, native page_token, crypto lookback caps, early termination — crypto cold loads: seconds (was hours) |
| Background bar pre-fetch | ✅ Done | All TFs cached silently |
| Limit orders | ✅ Done | Limit, stop, stop-limit via order type selector |
| Trailing stops | ✅ Done | Trail price/percent via order type selector |
| Bracket orders | ✅ Done | Market entry + TP/SL legs (default with SL/TP lines) |
| Modify/cancel pending orders | ✅ Done | Orders panel with cancel buttons, PATCH API |
| Close Partial (smart) | ✅ Done | Floating window with 25/50/75/100% quick buttons |
| Draggable SL/TP lines | ✅ Done | Double-click to grab, drag to adjust |
| Drawing tools (trend lines, Fibonacci) | ✅ Done | Canvas overlay, keyboard L/F/X, persistent |
| Price alerts | ✅ Done | Keyboard `a`, browser notifications, persistent |
| Trade history panel | ✅ Done | Orders panel with open + recent fills |
| Open positions panel | ✅ Done | Live P/L, one-click close, click to switch chart |
| Zeroize API keys | ✅ Done | `zeroize` crate for memory cleanup on drop |
| Button debounce | ✅ Done | All trading buttons guarded against double-fire |
| Equity TP/SL protection | ✅ Done | Port of MQL5 EnableEquityTP/SL — auto-close all at target |
| Encrypted credentials | ✅ Done | AES-256-GCM encrypted, machine-specific key, SQLite-backed |
| MTF grid view | ✅ Done | 2-5 TF grid with full indicators, double-click fullscreen |
| Auto Fibonacci | ✅ Done | Fractal-based swing detection, 13 levels (retrace + extension) |
| Per-broker data isolation | ✅ Done | Separate IndexedDB + cold cache per account |
| Article images | ✅ Done | HTTPS images in news reader, click to enlarge |
| Security hardening (21 passes) | ✅ Done | 97 findings, 91 fixed, 6 accepted |

### Tier 2 — Competitive with MT5

| Feature | Status | Notes |
|---|---|---|
| Strategy backtester | ✅ Done | Strategy trait, SMA Cross example, BacktestResult with equity curve |
| Chart templates | ✅ Done | Save/load indicator configs + order mode via dropdown |
| Workspace profiles | ✅ Done | Save/load entire layout (tabs, indicators, panes) |
| Economic calendar | ✅ Done | Alpaca market calendar in floating window |
| Extended drawing tools | ✅ Done | Horizontal line (n), rectangle (r), channels, plus trend/fib |
| Detailed trade reports | ✅ Done | Profit factor, Sharpe, drawdown, consecutive W/L in backtester |
| Trade history export (CSV) | ✅ Done | Export closed orders as CSV via Tauri command |
| WebSocket streaming | ✅ Done | Real-time trades/quotes via Alpaca WS, poll_stream command |
| Time & Sales | ✅ Done | Via WebSocket trade stream subscription |
| 37 indicators + 22 Wasm | ✅ Done | NNFX (9) + Standard (19) + MT5 Parity (9: Alligator, AO, MFI, Force Index, Envelopes, StdDev, Chaikin, DeMarker, Fractals) |
| Custom indicator plugin system | ✅ Done | Load/save/list JS plugins from ~/.config/typhoon-terminal/indicators/ |
| NNFX backtesting strategy | ✅ Done | KAMA + Fisher Transform entry logic, D1/W1/MN1 optimized |
| Options P&L calculator | ✅ Done | Multi-leg payoff diagram with canvas rendering (Ctrl+K → OPTCALC) |
| Sector rotation heatmap | ✅ Done | S&P 500 sector ETFs colored by daily/weekly performance (Ctrl+K → SECTORS) |
| Options strategy builder | ✅ Done | Live chain viewer, strategy presets, aggregate Greeks (Ctrl+K → OPTSTRAT) |
| Strategy auto-trading framework | ✅ Done | JS plugin → live orders, paper-only safety (Ctrl+K → AUTOTRADE) |
| Watchlist SMA200 cross alerts | ✅ Done | Batch monitoring with browser notifications |
| Economic calendar + countdown | ✅ Done | FOMC, CPI, NFP, GDP with live countdown timers (Ctrl+K → ECON) |
| Options put/call ratio | ✅ Done | Ctrl+K → PCRATIO, weighted P/C from options chain |
| Unusual options activity | ✅ Done | Ctrl+K → UNUSUAL, volume vs OI spike detection |
| IV rank / IV percentile | ✅ Done | Ctrl+K → IVRANK, 52-week IV context |
| Options Greeks dashboard | ✅ Done | Ctrl+K → GREEKS, aggregate portfolio Greeks |
| Options profit scenario | ✅ Done | Ctrl+K → OPTPROFIT, theoretical P&L at target price/date |
| Multi-symbol comparison | ✅ Done | Ctrl+K → COMPARE, normalized overlay of up to 5 symbols |
| Spread chart | ✅ Done | Ctrl+K → SPREAD, price ratio/difference between two symbols |
| Support/Resistance levels | ✅ Done | Ctrl+K → SRLEVEL, automatic pivot/fractal-based S/R |
| Divergence scanner | ✅ Done | Ctrl+K → DIVERGENCE, RSI/MACD vs price divergence |
| Volume profile | ✅ Done | Ctrl+K → VOLUME, price-at-volume distribution (POC/VA) |
| Pivot points | ✅ Done | Ctrl+K → PIVOTS, classic/Fibonacci/Woodie pivot levels |
| Relative performance | ✅ Done | Ctrl+K → PERF, symbol vs benchmark % performance |
| Enhanced VWAP | ✅ Done | Ctrl+K → VWAP+, anchored VWAP with standard deviation bands |
| Market replay / practice | ✅ Done | Ctrl+K → REPLAY, historical bar-by-bar replay with simulated trading |
| Trade statistics | ✅ Done | Ctrl+K → TRADESTATS, win rate/expectancy/R-multiple analysis |
| Pairs trading | ✅ Done | Ctrl+K → PAIRS, cointegration test + z-score signals |

### Tier 3 — Godel Terminal Features

| Feature | Status | Notes |
|---|---|---|
| Command palette | ✅ Done | Ctrl+K, DES/NEWS/FA/OPT/SCAN/HDS/HIST/QM commands |
| DES — Company description | ✅ Done | SEC fundamentals via EDGAR |
| OPT — Options chain | ✅ Done | Alpaca options API with Greeks, strike, bid/ask |
| SCAN — Stock screener | ✅ Done | Filter by price, volume, sector, change%, tradable/shortable |
| QM — Quote monitor (watchlist) | ✅ Done | Multi-symbol dashboard with live prices, Ctrl+K → QM |
| Interactive news/filing panels | ✅ Done | In-app article reading in floating windows |
| FA — Financial analysis | ✅ Done | Income stmt, balance sheet, cash flow from SEC EDGAR us-gaap |
| **ANR — Analyst recommendations** | ✅ Done | Finnhub recommendations + price targets (Ctrl+K → ANR) |
| **SI — Short interest** | ✅ Done | Finnhub bi-weekly short interest + trend chart (Ctrl+K → SI) |
| HDS — Institutional holders | ✅ Done | 13F filing history from SEC EDGAR submissions |
| MOST — Most active | ✅ Done | Alpaca screener most-actives + top movers endpoints |
| **WEI — World equity indices** | ✅ Done | Yahoo Finance world indices, auto-refresh (Ctrl+K → WEI) |
| **FX — Currency matrix** | ✅ Done | ECB rates + cross rate matrix (Ctrl+K → FX) |
| HMS — Historical market stats (FRED) | ✅ Done | Ctrl+K → FRED (user provides free API key) |
| AI chat | ✅ Done | Ctrl+K → AI (Claude/GPT with market context) |
| **Community chat** | ✅ Done | Matrix protocol — Ctrl+K → CHAT, no server needed |

### Tier 4 — Advanced / Long-Term

| Feature | Status | Notes |
|---|---|---|
| Alpaca broker | ✅ Done | Full Alpaca API integration (stocks, ETFs, options, crypto) |
| Push notifications (mobile) | ✅ Done | Pushover + ntfy.sh integration |
| Multi-chart layouts | ✅ Done | MTF Grid view (2-5 TFs with full indicators per cell) |
| Chart screenshot export | ✅ Done | Ctrl+Shift+S, copy to clipboard + toast |
| Visual backtester | ✅ Done | Bar-by-bar replay with equity curve in floating window |
| Genetic optimization | ✅ Done | Grid search over SMA periods, sortable results table |
| DOM / Level 2 | ✅ Done | Crypto orderbook from Alpaca, bid/ask depth display |
| Line/Bar chart types | ✅ Done | Candles/Line/Bars selector |
| Bid/Ask spread display | ✅ Done | Latest quote in dashboard |
| Time & Sales panel | ✅ Done | WebSocket trade stream in floating window |
| Account activities history | ✅ Done | Deposits, dividends, fills from Alpaca |
| Insider trading (Form 4) | ✅ Done | SEC EDGAR Form 4 via command palette |
| Right-click context menu | ✅ Done | Draw, alert, copy price from chart |
| Pending order visualization | ✅ Done | Open orders as colored lines on chart |
| **Options flow analysis** | ✅ Done | Synthetic flow from options chain volume/OI analysis |
| **Wasm indicator engine** | ✅ Done | 32KB Wasm binary — SMA/EMA/KAMA/RSI/Fisher/ATR/MACD/Bollinger + grid optimizer |
| **Binary bar storage** | ✅ Done | Packed f64 format (48 bytes/bar) + zstd — 3-5x smaller than JSON |
| **Headless CLI backtest** | ✅ Done | `--backtest` flag — run strategies from command line, no GUI |
| **CLI / TUI Terminal** | ✅ Done | 6.5MB standalone binary — ratatui TUI with full trading parity, ASCII charts, risk dashboard, shared credentials |
| **Multi-Account Tabulation** | ✅ Done | MT5 CSV import + Alpaca, aggregate portfolio view, combined VaR, account weights |
| **Plugin marketplace** | 🔲 Blocked | Needs distribution infrastructure |
| **GPU chart rendering (WebGL2)** | ✅ Done | 45KB Wasm — opt-in via "GPU Candles" selector. Candlesticks, indicator lines, grid, pan/zoom/scroll |
| **Pure Rust GUI migration** | 🔲 Blocked | Egui/Iced — long-term architectural goal |

| Data Window | ✅ Done | Fixed panel: OHLCV + all indicator values at cursor |
| Drawing object properties | ✅ Done | Right-click drawing: color picker, line width, delete |
| Portfolio breakdown by sector | ✅ Done | Ctrl+K → PORTFOLIO, grouped by asset class |
| Multi-condition alerts | ✅ Done | Ctrl+K → ALERTS: RSI/KAMA/Fisher conditions |
| Walk-forward testing | ✅ Done | 70/30 in-sample/out-of-sample split, auto-optimize |
| Monte Carlo risk of ruin | ✅ Done | Ctrl+K → MONTECARLO, 100K simulations |
| Earnings calendar | ✅ Done | Ctrl+K → EARNINGS, corporate actions table |
| Dividend alerts | ✅ Done | Auto-notify 5 days before ex-dividend |
| Correlation matrix | ✅ Done | Ctrl+K → CORR, pairwise heatmap from cached bars |

| Heikin-Ashi candlesticks | ✅ Done | Additional chart type with smoothed OHLC |
| Risk/reward overlay | ✅ Done | Visual P&L zones when SL/TP set (green profit, red loss) |
| Trade journal | ✅ Done | Ctrl+K → JOURNAL, log trades with notes, persistent |
| Position sizing calculator | ✅ Done | Ctrl+K → CALC, risk/entry/SL → lot size + R:R |
| Chart annotations | ✅ Done | Ctrl+K → ANNOTATE, text markers on chart |
| Regime detection | ✅ Done | ADX-based trending/ranging/choppy in timer bar |
| Multi-symbol alert dashboard | ✅ Done | Ctrl+K → ALERTBOARD, cross-watchlist alert check |
| AI trade review | ✅ Done | "Review My Trades" button in AI chat |
| Pattern recognition | ✅ Done | Ctrl+K → PATTERNS, double top/bottom + H&S detection |
| Sentiment analysis | ✅ Done | Ctrl+K → SENTIMENT, keyword-based bullish/bearish score |
| Volatility surface | ✅ Done | Ctrl+K → VOLSURF, strike×expiry IV heatmap |
| Bracket order UI | ✅ Done | Ctrl+K → BRACKET, visual OCO/bracket placement |
| Portfolio heat map | ✅ Done | Ctrl+K → HEATMAP, finviz-style colored boxes |
| Custom timeframes | ✅ Done | 2H, 3H, 6H, 2D, 3D via bar aggregation |
| Renko bars | ✅ Done | ATR-based Renko chart type |
| GUI menu bar | ✅ Done | File/View/Trading/Tools/Research/Analysis (50+ entries) |
| Draggable tab reordering | ✅ Done | Drag-and-drop, unique feature vs all competitors |
| Ray + Ruler drawing tools | ✅ Done | TradingView-style ray and measurement |
| Improved SL/TP drag | ✅ Done | Single-click grab, live risk tooltip, MTF grid support |
| MTF grid trading | ✅ Done | Click cell to select, SL/TP on active cell |
| Live price sync (MTF) | ✅ Done | All grid cells show same current price |
| FRED economic data | ✅ Done | Fed Funds, CPI, GDP, Treasury yields, VIX, M2 (user provides free API key) |
| AI trading assistant | ✅ Done | Claude (Anthropic) or GPT (OpenAI) chat with market context |
| Settings panel | ✅ Done | Ctrl+K → SETTINGS for API key management |
| Market breadth dashboard | ✅ Done | Ctrl+K → BREADTH, advance/decline, new highs/lows, McClellan |
| Institutional flows | ✅ Done | Ctrl+K → FLOWS, sector ETF volume/price divergence |
| Gap analysis | ✅ Done | Ctrl+K → GAPS, unfilled gap detection with fill probability |
| Relative strength ranking | ✅ Done | Ctrl+K → RELSTRENGTH, Mansfield RS vs benchmark |
| Seasonality analysis | ✅ Done | Ctrl+K → SEASONALITY, monthly return patterns from historical data |
| Correlation watchdog | ✅ Done | Ctrl+K → CORRWATCH, alert on correlation regime changes |
| Flow map visualization | ✅ Done | Ctrl+K → FLOWMAP, sector money flow Sankey-style diagram |
| Risk heat map | ✅ Done | Ctrl+K → RISKMAP, portfolio VaR contribution by position |
| Risk scenario simulator | ✅ Done | Ctrl+K → RISKSIM, stress test portfolio against historical events |
| Equity curve tracker | ✅ Done | Ctrl+K → EQUITY, live equity curve with drawdown overlay |
| Smart alerts | ✅ Done | Ctrl+K → SMARTALERT, statistical anomaly detection alerts |
| Enhanced regime detection | ✅ Done | Ctrl+K → REGIME+, volatility regime + mean reversion signals |
| MTF divergence scanner | ✅ Done | Ctrl+K → MTFDIV, cross-timeframe indicator divergence |
| Multi-leg order builder | ✅ Done | Ctrl+K → MULTILEG, options/stock combo orders |
| Enhanced backtester | ✅ Done | Ctrl+K → BACKTEST+, no-code visual strategy builder |
| Enhanced scanner | ✅ Done | Ctrl+K → SCANNER+, multi-factor custom screening |
| Market profile / TPO | ✅ Done | Ctrl+K → MARKETPROFILE, time-price-opportunity distribution |
| Calendar heat map | ✅ Done | Ctrl+K → HEATCAL, daily returns calendar visualization |
| Enhanced economic calendar | ✅ Done | Ctrl+K → ECALENDAR, ForexFactory-style with impact filters |
| Order flow analysis | ✅ Done | Ctrl+K → ORDERFLOW, trade tape aggregation + delta |
| Portfolio snapshot | ✅ Done | Ctrl+K → SNAPSHOT, copy portfolio table to clipboard |
| Real-time top movers | ✅ Done | Ctrl+K → HOTLIST, auto-refresh gainers/losers/active |
| Per-symbol notes | ✅ Done | Ctrl+K → NOTES, persistent trading notes per symbol |
| Custom timers | ✅ Done | Ctrl+K → TIMER, countdown timers with London/NY presets |
| Chart data export | ✅ Done | Ctrl+K → EXPORT, OHLCV + indicators to CSV |
| Theme switcher | ✅ Done | Ctrl+K → DARKMODE, dark/pitch black/light themes |
| Fibonacci time zones | ✅ Done | Ctrl+K → FIBO+, time-based Fibonacci from fractal anchor |
| Composite signal | ✅ Done | Ctrl+K → SIGNAL, 0-100 score from 6 indicators |
| Trading profile | ✅ Done | Ctrl+K → PROFILE, analytics by symbol/day/side/hold time |
| Price ladder / DOM | ✅ Done | Ctrl+K → LADDER, vertical bid-ask depth visualization |
| Options chain visualizer | ✅ Done | Ctrl+K → CHAIN+, vol smile + OI profile + volume heatmap |
| Live spread monitor | ✅ Done | Ctrl+K → SPREAD+, bid-ask spread tracking with 2σ alert |
| Webhook alerts | ✅ Done | Ctrl+K → WEBHOOK, custom endpoint POST on events |
| Heatmap order book | ✅ Done | Ctrl+K → BOOKMAP, order book depth over time (canvas) |
| Custom dashboard | ✅ Done | Ctrl+K → DASHBOARD, 8-widget configurable grid |
| Real-time scanner | ✅ Done | Ctrl+K → SCANNER-RT, 7 conditions with 60s polling |
| Algorithm monitor | ✅ Done | Ctrl+K → ALGO, live auto-trade strategy status |
| Enhanced journal | ✅ Done | Ctrl+K → JOURNAL+, tags/ratings/monthly P&L calendar |
| Correlation network | ✅ Done | Ctrl+K → CORRELATION3D, force-directed graph (canvas) |
| Import trade history | ✅ Done | Ctrl+K → IMPORTTRADES, CSV from MT5/IB/generic |
| MT5 SQLite Direct Sync | ✅ Done | Real-time bar data from 3 Darwinex MT5 instances via BarCacheWriter — ~895 symbols, <2s incremental sync |

| Portable backup + LAN sync | ✅ Done | Export/import `.typhoon-backup` + WebSocket LAN sync with HMAC auth |
| DARWIN analytics expansion | ✅ Done | VaR multipliers, drawdown dashboard, floating equity, rebalancer, symbol overlap |

### Blocked / Deferred

| Feature | Status | Notes |
|---|---|---|
| **Plugin marketplace** | 🔲 Blocked | Needs distribution infrastructure |
| **Pure Rust GUI migration** | 🔲 Blocked | Egui/Iced — long-term architectural goal |

---

## Data Sources (Free)

| Source | Data | Auth | Rate Limit |
|---|---|---|---|
| **Darwinex MT5** | Real-time bars (~895 symbols: CFDs, forex, crypto, commodities, indices, stocks) | BarCacheWriter EA | None (local SQLite) |
| **Alpaca** | Bars (15-min delayed free), quotes, news, corporate actions, order execution | API key | 200/min |
| **SEC EDGAR** | Filings, company facts, 13F holders, insider trading | None (User-Agent) | 10/sec |
| **FRED** | Economic data, interest rates, GDP | API key (free) | 120/min |
| **ForexFactory** | Economic calendar | None (scrape) | — |
| **Congress API** | Legislative data, lobbying | None | — |
| **Yahoo Finance** | Backup quotes, earnings calendar, world indices | None (unofficial) | — |
| **Finnhub** | Analyst recommendations, price targets, short interest, insider sentiment | API key (free) | 60/min |
| **ECB** | Forex exchange rates (daily reference) | None (XML feed) | — |
| **House Stock Watcher** | Congressional stock trades | None | — |
| **CoinGecko** | Crypto market data, trending, sparklines | None | — |
| **Treasury.gov** | Daily treasury yield rates | None | — |
| **alternative.me** | Crypto Fear & Greed Index | None | — |
| **whale-alert.io** | Large crypto transactions | Free key | 10/min |
| **Reddit JSON** | WSB/investing post search | None | — |
| **FINRA RegSHO** | Daily short sale volume | None | — |
| **FMP** | Analyst estimates, financial ratios, DCF | API key (free) | 250/day |
| **Alpha Vantage** | Earnings surprises, company overview | API key (free) | 5/min |

---

## Architecture Principles

See [DESIGN_PHILOSOPHY.md](DESIGN_PHILOSOPHY.md) for the 8 core principles.

### Broker Abstraction

```rust
trait Broker {
    async fn get_account(&self) -> Result<AccountInfo>;
    async fn get_positions(&self) -> Result<Vec<Position>>;
    async fn market_order(&self, symbol: &str, qty: f64, side: &str) -> Result<Order>;
    async fn limit_order(&self, symbol: &str, qty: f64, side: &str, price: f64) -> Result<Order>;
    async fn close_position(&self, symbol: &str, qty: Option<f64>) -> Result<Order>;
    async fn get_bars(&self, symbol: &str, tf: &str, limit: u32) -> Result<Vec<Bar>>;
    async fn get_news(&self, symbol: &str, limit: u32) -> Result<Vec<News>>;
}
```

### Indicator Plugin System

```javascript
// User writes a JS file, drops in indicators/ directory
export default {
  name: "My Custom RSI",
  params: { period: 14 },
  pane: "separate", // or "overlay"
  calculate(data, params) {
    // return array of { time, value } or { time, value, color }
  },
  style: { color: "#FF0000", lineWidth: 2 },
};
```

### Strategy Backtester

```rust
trait Strategy {
    fn on_bar(&mut self, bar: &Bar, indicators: &HashMap<String, Vec<f64>>) -> Option<Signal>;
    fn on_fill(&mut self, fill: &Fill);
}
```

Run strategy against cached bar data. No broker connection needed. Results: equity curve, trade list, performance metrics.

---

## Contributing

TyphooN-Terminal is Apache-2.0. Contributions welcome:

1. Fork → branch → PR
2. Follow [DESIGN_PHILOSOPHY.md](DESIGN_PHILOSOPHY.md) principles
3. Add ADR for architectural decisions
4. Add tests for risk engine changes
5. No AI tool attribution in commits
