# TyphooN-Terminal Roadmap

## Vision

Replace both MetaTrader 5 and Godel Terminal with a single open-source desktop trading terminal. Combine MT5's modular indicator/EA architecture with Godel's data-rich command-line interface and Bloomberg-style research capabilities.

---

## Broker Support

### Current
- **Alpaca** — Stocks, ETFs, Options, Crypto (free paper trading, IEX data)
- **Tastytrade** — Stocks, Options, Futures, Crypto (free paper trading, session auth)

### Architecture
Each broker implements a Rust trait. Adding a new broker means one new file — no changes to risk engine, indicators, or UI.

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
| Security hardening (6 passes) | ✅ Done | 50 findings: input validation, timeouts, path traversal, CSP, config bounds, resource limits, event listener cleanup |
| MTF MA grid | ✅ Done | SMA200/KAMA/Fisher across TFs |
| Symbol autocomplete | ✅ Done | 11K+ symbols |
| Rate limiter with 429 cooldown | ✅ Done | |
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
| OS keychain credentials | ✅ Done | gnome-keyring/KWallet via `keyring` crate |
| MTF grid view | ✅ Done | 2-5 TF grid with full indicators, double-click fullscreen |
| Auto Fibonacci | ✅ Done | Fractal-based swing detection, 13 levels (retrace + extension) |
| Per-broker data isolation | ✅ Done | Separate IndexedDB + cold cache per account |
| Article images | ✅ Done | HTTPS images in news reader, click to enlarge |
| Security hardening (18 passes) | ✅ Done | 84 findings, 78 fixed, 6 accepted |

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
| 30 indicators | ✅ Done | NNFX (9) + Standard (11) + Extended (10: Stochastic, CCI, ADX, Williams %R, Ichimoku Cloud, Parabolic SAR, OBV, Momentum, WMA, HMA) |
| Custom indicator plugin system | ✅ Done | Load/save/list JS plugins from ~/.config/typhoon-terminal/indicators/ |
| NNFX backtesting strategy | ✅ Done | KAMA + Fisher Transform entry logic, D1/W1/MN1 optimized |
| Options P&L calculator | ✅ Done | Multi-leg payoff diagram with canvas rendering (Ctrl+K → OPTCALC) |
| Sector rotation heatmap | ✅ Done | S&P 500 sector ETFs colored by daily/weekly performance (Ctrl+K → SECTORS) |
| Options strategy builder | ✅ Done | Live chain viewer, strategy presets, aggregate Greeks (Ctrl+K → OPTSTRAT) |
| Strategy auto-trading framework | ✅ Done | JS plugin → live orders, paper-only safety (Ctrl+K → AUTOTRADE) |
| Watchlist SMA200 cross alerts | ✅ Done | Batch monitoring with browser notifications |
| Economic calendar + countdown | ✅ Done | FOMC, CPI, NFP, GDP with live countdown timers (Ctrl+K → ECON) |

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
| **ANR — Analyst recommendations** | 🔲 Deferred | No free consensus data API |
| **SI — Short interest** | 🔲 Deferred | No free reliable API |
| HDS — Institutional holders | ✅ Done | 13F filing history from SEC EDGAR submissions |
| MOST — Most active | ✅ Done | Alpaca screener most-actives + top movers endpoints |
| **WEI — World equity indices** | 🔲 Deferred | Alpaca is US-only |
| **FX — Currency matrix** | 🔲 Deferred | Needs forex data source |
| HMS — Historical market stats (FRED) | ✅ Done | Ctrl+K → FRED (user provides free API key) |
| AI chat | ✅ Done | Ctrl+K → AI (Claude/GPT with market context) |
| **Community chat** | 🔲 Deferred | Needs server infrastructure |

### Tier 4 — Advanced / Long-Term

| Feature | Status | Notes |
|---|---|---|
| Multi-broker support | ✅ Done | BrokerTrait with async methods, AlpacaBroker impl |
| Push notifications (mobile) | ✅ Done | Pushover + ntfy.sh integration |
| Multi-chart layouts | ✅ Done | Split view with independent symbols |
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
| **Options flow analysis** | 🔲 Blocked | Needs paid data source (FlowAlgo, Unusual Whales) |
| **Plugin marketplace** | 🔲 Blocked | Needs distribution infrastructure |
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

### Blocked — Needs External Resources

| Feature | Blocker |
|---|---|
| Analyst recommendations (ANR) | No free consensus API |
| Short interest (SI) | No free real-time API |
| Dark pool / options flow | No free data source |
| World equity indices | Alpaca is US-only |
| Forex currency matrix | Alpaca has crypto not forex |
| Congress trading | Free APIs locked down; QuiverQuant requires paid tier |
| Community chat | Needs WebSocket server infrastructure |

---

## Data Sources (Free)

| Source | Data | Auth | Rate Limit |
|---|---|---|---|
| **Alpaca** | Bars, quotes, news, corporate actions | API key | 200/min |
| **SEC EDGAR** | Filings, company facts, 13F holders, insider trading | None (User-Agent) | 10/sec |
| **FRED** | Economic data, interest rates, GDP | API key (free) | 120/min |
| **ForexFactory** | Economic calendar | None (scrape) | — |
| **Congress API** | Legislative data, lobbying | None | — |
| **Yahoo Finance** | Backup quotes, earnings calendar | None (unofficial) | — |

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
