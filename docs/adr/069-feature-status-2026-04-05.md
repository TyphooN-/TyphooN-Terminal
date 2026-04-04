# ADR-069: Feature Status & Roadmap (2026-04-05)

**Status:** Current | **Date:** 2026-04-05

## Implemented (Production Ready)

### Core Terminal
- 89 drawing tools (100% TradingView parity + 7 bonus), all with move/drag, selection, control points, eraser
- 46 GPU-accelerated indicators with adjustable parameters (10 DragValue sliders)
- 5 chart types: Candlestick, Heikin-Ashi, Line, OHLC Bars, Renko
- Logarithmic + linear price scale, auto-fit, follow-latest toggle
- Pre-placement color picker, per-drawing right-click property editor
- Multi-symbol overlay (COMPARE command, % change line)
- 54 floating analytical windows, all with charts/gauges/heatmaps
- 205 command palette entries, TradingView keyboard shortcuts (Alt+H/V/T/F/R/E/L/C)
- Session persistence, chart templates (SAVE_TEMPLATE/LOAD_TEMPLATE)
- Screenshot export (PNG)

### Broker Integration
- **Alpaca Markets**: Full REST API — connect, account, positions, orders (market/limit/stop/bracket), close position, cancel order, portfolio history, quotes, streaming (WebSocket), watchlists, options chain, corporate actions, most active, top movers
- **tastytrade**: REST API — login, accounts, positions, balances, option chains, equity order placement + DXLink WebSocket streaming (historical bars + real-time quotes)
- **MT5 (Darwinex)**: LAN sync from BarCacheWriter databases, bid/ask live quotes, 34 KV-synced analytics fields, 14 remote commands, TLS encryption

### Data Sources (22+)
- Alpaca Markets, SEC EDGAR, FRED (10 series), Finnhub, FMP, Alpha Vantage, CryptoCompare, CoinGecko, ECB, House Stock Watcher, Yahoo Finance, Treasury.gov, alternative.me, whale-alert.io, Reddit JSON, FINRA RegSHO, Pushover, ntfy.sh, Anthropic, OpenAI, QuiverQuant, Matrix

### Analytics
- DARWIN analytics: 80+ functions, performance attribution, D-Score components, investment velocity, tax lots, rolling correlation, diversification candidates
- Risk engine: VaR, Monte Carlo, stress test, risk-of-ruin, margin monitor
- MQL5 + PineScript compilers (82 tests, WASM + WGSL backends)
- GPU backtester + optimizer (SMA cross + NNFX strategies)
- Compound interest calculator with growth chart

### Infrastructure
- 575 tests (82 compiler + 421 engine + 72 native)
- Zero warnings, zero unsafe blocks, zero TODO/FIXME
- Prometheus metrics endpoint
- LAN sync with TLS + PBKDF2 auth + constant-time HMAC
- Notifications: Discord webhook, Pushover, ntfy.sh (fires on indicator alerts)

## Blog Post Claims — Status

| Claimed Feature | Status | Notes |
|----------------|--------|-------|
| Fear and Greed Index | NOT IMPLEMENTED | alternative.me API exists in ADR-033 research, not wired |
| World Indices Dashboard | NOT IMPLEMENTED | Yahoo Finance scraper exists but no dedicated dashboard |
| Forex Cross-Rate Matrix | NOT IMPLEMENTED | ECB rates exist but no matrix UI |
| Crypto Top 50 | NOT IMPLEMENTED | CoinGecko API researched but not wired |
| Dark Pool Volume | NOT IMPLEMENTED | SqueezMetrics DIX/GEX researched but not wired |
| OCO Orders | NOT IMPLEMENTED | Only market/limit/stop/bracket exist |
| Draggable SL/TP | PARTIAL | SL/TP lines render on chart but are not interactively draggable; must edit numerically |
| 7 Order Types | PARTIAL | 5 types functional (market/limit/stop/bracket/cancel). Missing: trailing stop, OCO, stop-limit |

## Remaining Gaps (Prioritized)

### HIGH (Functional gaps)
- Draggable SL/TP lines on chart (currently static visual only)
- Trailing stop order type (engine has `trailing_stop_order()`, not wired to UI)
- tastytrade position close (no mechanism exists)
- Periodic MT5 sync loop (currently manual command only)

### MEDIUM (Feature expansion)
- Fear & Greed Index window (alternative.me API)
- World indices dashboard (Yahoo Finance data)
- Crypto top 50 window (CoinGecko API)
- EasyLanguage compiler (third frontend for MQL5 IR pipeline)
- thinkScript compiler (fourth frontend)
- Options Greeks display in option chain windows

### LOW (Nice-to-have)
- Forex cross-rate matrix
- Dark pool volume (SqueezMetrics)
- OCO order type
- Stop-limit order type
- Watchlist update/delete (only create/read exist)
- Drawing control point drag-to-resize (handles render but aren't interactive)
- Account-history-based compound interest projection

## Consequences
- Terminal exceeds TradingView/MT5 in drawing tools, indicators, and analytical windows
- Order placement now functional for Alpaca + tastytrade (was regression from Tauri migration)
- Blog post claims 5 features that are researched but not implemented — documented here for transparency
- All security vulnerabilities addressed (constant-time HMAC, webhook hardening, log bounds, zero unwrap)
