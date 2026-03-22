# ADR-021: MT5 + Godel Terminal Feature Parity Roadmap

**Status:** Mostly Complete
**Date:** 2026-03-16

## Context

TyphooN-Terminal aims to replace both MetaTrader 5 and Godel Terminal. A comprehensive feature gap analysis was performed comparing all three platforms, plus OpenBB Terminal.

## Features Implemented (This Round)

| Feature | Source | Implementation |
|---|---|---|
| Line/Bar chart types | MT5 | Chart type selector: Candles/Line/Bars |
| Bid/Ask spread display | MT5 Market Watch | Latest quote via Alpaca REST, shown in dashboard |
| Time & Sales panel | MT5 Market Watch | WebSocket trade stream in scrolling floating window |
| Account activities | MT5 History tab | Alpaca account activities API (deposits, dividends, fills) |
| Insider trading (Form 4) | Godel Terminal | SEC EDGAR Form 4 parsing, command palette "INSIDER" |
| Right-click context menu | MT5 | Custom context menu on chart: draw, alerts, copy price |
| Pending order visualization | MT5 | Open orders rendered as colored price lines on chart |

## Remaining Implementable Features (Future Work)

| Feature | Effort | Notes |
|---|---|---|
| Data Window (all indicator values at cursor) | 4h | Wire crosshairMove to aggregate indicator values |
| Drawing object properties panel | 8h | Select + edit color/style/text of drawn objects |
| Portfolio breakdown by sector | 6h | Group positions by sector, pie/bar chart |
| Multi-condition alerts (RSI > 70, KAMA cross) | 8h | Extend alert system with indicator conditions |
| Alert history panel | 2h | Log triggered alerts in floating window |
| Walk-forward testing | 12h | Split backtest into in-sample/out-of-sample |
| Monte Carlo risk of ruin | 6h | Simulate P&L distribution from backtest |
| Earnings calendar with estimates | 4h | Yahoo Finance unofficial API or Alpaca |
| Dividend/corporate action alerts | 3h | Alpaca corporate_action_signals |
| Congress trading data | 8h | Congress API free tier |
| Correlation matrix | 6h | Rolling correlation heatmap from cached bars |
| Account statement export (PDF) | 4h | Build summary, export via Tauri |

## Blocked Features & Why

### Blocked by External Data Sources

| Feature | Blocker | Alternative |
|---|---|---|
| **Analyst recommendations (ANR)** | ~~No free API~~ **Resolved**: Finnhub free tier. | ✅ Implemented |
| **Short interest (SI)** | ~~No free real-time API~~ **Resolved**: Finnhub bi-weekly. | ✅ Implemented |
| **Dark pool / options flow** | No free unusual activity data. | ✅ Implemented synthetically from options chain volume/OI analysis |
| **World equity indices (WEI)** | ~~Alpaca is US-only~~ **Resolved**: Yahoo Finance world indices. | ✅ Implemented |
| **Forex currency matrix (FX)** | ~~Alpaca has no forex~~ **Resolved**: ECB rates + Darwinex MT5 real-time forex via SQLite Direct Sync ([ADR-036](036-mt5-sqlite-direct-sync.md)). | ✅ Implemented |
| **Historical market stats (HMS)** | ~~Needs user API key~~ **Resolved**: FRED API with user-provided key. | ✅ Implemented |

### Blocked by Infrastructure

| Feature | Blocker | Path Forward |
|---|---|---|
| **AI chat** | ~~Needs LLM API~~ **Resolved**: Claude/GPT with user's API key. | ✅ Implemented |
| **Community chat** | ~~Needs server~~ **Resolved**: Matrix protocol, no server needed. | ✅ Implemented |
| **Plugin marketplace** | Needs distribution server, versioning, review system. | Start with local plugin loading (already done), add GitHub-based sharing later |
| **Pure Rust GUI** | Architectural migration from Tauri webview to egui/iced. | Long-term goal — current webview works well |

### Blocked by API Limitations

| Feature | Blocker | Workaround |
|---|---|---|
| **Level 2 for equities** | Alpaca free tier doesn't include full orderbook for stocks. | Show NBBO (national best bid/offer) from quote data |
| **Tick charts** | Alpaca doesn't provide raw tick data in REST. | Aggregate WebSocket trades into tick-count bars client-side |
| **Email alerts** | Need SMTP server or email API (AWS SES, SendGrid). | User provides SMTP config in settings |

## Comparison: OpenBB Terminal Features

OpenBB is a Python CLI terminal. Features they have that we should consider:

| OpenBB Feature | Our Status | Notes |
|---|---|---|
| Fundamental analysis (DCF, ratios) | Partial (SEC EDGAR) | Could add ratio calculations from existing data |
| Technical analysis (50+ indicators) | 30 indicators | Could port more niche oscillators |
| Quantitative analysis (normality tests, CAPM) | Backtester covers some | Monte Carlo would add more |
| Portfolio optimization (Markowitz, HRP) | Not implemented | Complex, requires covariance matrix |
| Government data (Fed speakers, treasury) | Not implemented | FRED API when user provides key |
| Crypto on-chain analysis | Not implemented | Needs blockchain API (Etherscan, etc.) |
| Forex (OANDA integration) | Not implemented | Needs OANDA API key |
| Jupyter notebook integration | Not applicable | We're a desktop app, not CLI |

## Architecture Notes

All new features follow the established pattern:
- **Backend**: Method on `AlpacaBroker` → Tauri command in `main.rs` → input validation
- **Frontend**: Calculation/rendering in `main.js` → floating window via `createWindow()` → command palette entry
- **Data**: Four-tier cache (memory LRU → IndexedDB → SQLite → zstd files)
- **Security**: All inputs validated, no innerHTML, HTTPS-only external fetches
