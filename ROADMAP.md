# TyphooN-Terminal Roadmap

## Vision

Replace both MetaTrader 5 and Godel Terminal with a single open-source desktop trading terminal. Combine MT5's modular indicator/EA architecture with Godel's data-rich command-line interface and Bloomberg-style research capabilities.

---

## Broker Support

### Current
- **Alpaca** — Stocks, ETFs, Options, Crypto (free paper trading, IEX data)

### Planned (Priority Order)

| Broker | Asset Classes | Paper | API Quality | Barrier | Priority |
|---|---|---|---|---|---|
| **Tastytrade** | Stocks, Options, Futures, Crypto | Yes | Good | None | HIGH |
| **Public.com** | Stocks, ETFs, Options, Crypto | Unclear | New | None | MEDIUM |
| **Webull** | Stocks, Options, Futures, Crypto | Yes | Good | 1-3 day approval | MEDIUM |
| **Tradier** | Stocks, Options | Yes | Good | None | LOW |
| **Schwab** | Stocks, Options | Yes | Decent | 7-day token refresh | LOW |
| **IBKR** | Everything (150+ exchanges) | Yes | Excellent | $10K deposit | LOW |

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
| **Drawing tools** (trend lines, Fibonacci) | 🔲 TODO | Requires lightweight-charts plugin API |
| Price alerts | ✅ Done | Keyboard `a`, browser notifications, persistent |
| Trade history panel | ✅ Done | Orders panel with open + recent fills |
| Open positions panel | ✅ Done | Live P/L, one-click close, click to switch chart |
| Zeroize API keys | ✅ Done | `zeroize` crate for memory cleanup on drop |
| Button debounce | ✅ Done | All trading buttons guarded against double-fire |

### Tier 2 — Competitive with MT5

| Feature | Status | Notes |
|---|---|---|
| **Strategy backtester** | 🔲 TODO | OHLC bar-based, run strategy logic against history |
| **Chart templates** | 🔲 TODO | Save/load indicator + color config |
| **Workspace profiles** | 🔲 TODO | Save/load entire layout |
| **Economic calendar** | 🔲 TODO | Third-party API (ForexFactory/TradingEconomics) |
| **All 38+ MT5 indicators** | 🔲 TODO | Port remaining oscillators/volume/Bill Williams |
| **All drawing tools** | 🔲 TODO | 46 MT5 objects: channels, Gann, Elliott, shapes |
| **Custom indicator plugin system** | 🔲 TODO | Load user-written JS indicators |
| **Detailed trade reports** | 🔲 TODO | Profit factor, Sharpe, drawdown, consecutive W/L |
| **Trade history export** (CSV/XLSX) | 🔲 TODO | |
| **WebSocket streaming** | 🔲 TODO | Replace polling with real-time updates |
| **Time & Sales** | 🔲 TODO | Alpaca trades websocket |

### Tier 3 — Godel Terminal Features

| Feature | Status | Notes |
|---|---|---|
| **Command palette** (DES, FOCUS, FA, etc.) | 🔲 TODO | Bloomberg/Godel-style command interface |
| **DES — Company description** | Partial | SEC fundamentals exist, need richer data |
| **FA — Financial analysis** | 🔲 TODO | Income statement, balance sheet, cash flow |
| **ANR — Analyst recommendations** | 🔲 TODO | Consensus ratings, price targets |
| **SI — Short interest** | 🔲 TODO | Short interest data |
| **HDS — Institutional holders** | 🔲 TODO | 13F filings from SEC EDGAR |
| **OPT — Options chain** | 🔲 TODO | Alpaca options data (Greeks) |
| **SCAN — Stock screener** | 🔲 TODO | Filter by price, volume, fundamentals |
| **MOST — Most active** | 🔲 TODO | Top movers, volume leaders |
| **QM — Quote monitor** (watchlist) | 🔲 TODO | Real-time multi-symbol dashboard |
| **WEI — World equity indices** | 🔲 TODO | Global index tracking |
| **FX — Currency matrix** | 🔲 TODO | 14-currency comparison |
| **HMS — Historical market stats** | 🔲 TODO | Long-term market statistics |
| **Interactive news/filing panels** | 🔲 TODO | Open articles in-app, not external browser |
| **AI chat** | 🔲 TODO | Natural language queries about market data |
| **Community chat** | 🔲 TODO | Symbol-based group chat |

### Tier 4 — Advanced / Long-Term

| Feature | Status | Notes |
|---|---|---|
| **Visual backtester** | 🔲 TODO | Chart replay with pause/speed control |
| **Genetic optimization** | 🔲 TODO | Parameter sweep with fitness function |
| **Multi-broker support** | 🔲 TODO | Trait-based broker abstraction |
| **DOM / Level 2** | 🔲 TODO | Crypto only via Alpaca |
| **Options flow analysis** | 🔲 TODO | Unusual activity, dark pool prints |
| **Push notifications** (mobile) | 🔲 TODO | Via Pushover/ntfy |
| **Plugin marketplace** | 🔲 TODO | Community indicators/strategies |
| **Multi-chart layouts** | 🔲 TODO | Tile/cascade/split views |
| **Chart screenshot export** | 🔲 TODO | Clipboard + file |
| **Pure Rust GUI migration** | 🔲 TODO | Egui/Iced — eliminate webview dependency |

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

TyphooN-Terminal is GPL-3.0. Contributions welcome:

1. Fork → branch → PR
2. Follow [DESIGN_PHILOSOPHY.md](DESIGN_PHILOSOPHY.md) principles
3. Add ADR for architectural decisions
4. Add tests for risk engine changes
5. No AI tool attribution in commits
