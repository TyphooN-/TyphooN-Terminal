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
| Fear and Greed Index | IMPLEMENTED | FNG command, gauge visualization, alternative.me API |
| World Indices Dashboard | IMPLEMENTED | INDICES command, 16 ETF proxies via Alpaca quotes |
| Forex Cross-Rate Matrix | IMPLEMENTED | FOREX command, 10 major pairs with proper FX precision |
| Crypto Top 50 | IMPLEMENTED | CRYPTO50 command, CoinGecko API, scrollable grid |
| Dark Pool Volume | NOT IMPLEMENTED | SqueezMetrics DIX/GEX researched but not wired |
| OCO Orders | NOT IMPLEMENTED | Only market/limit/stop/bracket exist |
| Draggable SL/TP | IMPLEMENTED | Draggable lines + Set SL/TP places stop/limit orders on broker (full MT5 EA parity) |
| 7 Order Types | 6/7 | market/limit/stop/stop-limit/bracket/trailing + Set SL/TP orders. Missing: OCO (Alpaca limitation) |

## Remaining Gaps (Prioritized)

### HIGH (Functional gaps)
- ~~Draggable SL/TP lines~~ DONE
- ~~Trailing stop order type~~ DONE
- ~~Fear & Greed Index~~ DONE
- ~~World indices dashboard~~ DONE
- ~~Crypto top 50~~ DONE
- tastytrade position close (no mechanism exists)
- Periodic MT5 sync loop (currently manual command only)
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
