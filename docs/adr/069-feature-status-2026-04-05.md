# ADR-069: Feature Status & Roadmap (2026-04-05)

**Status:** Historical snapshot (superseded by ADR-088/089/090/092/093/201) | **Date:** 2026-04-05 | **Accuracy pass:** 2026-05-05

## Implemented (Production Ready)

### Core Terminal
- 89 drawing tools (100% TradingView parity + 7 bonus), all with move/drag, selection, control points, eraser
- 46 GPU-accelerated indicators with adjustable parameters (10 DragValue sliders)
- 5 chart types: Candlestick, Heikin-Ashi, Line, OHLC Bars, Renko
- Logarithmic + linear price scale, auto-fit, follow-latest toggle
- Pre-placement color picker, per-drawing right-click property editor
- Multi-symbol overlay (COMPARE command, % change line)
- 54 floating analytical windows, all with charts/gauges/heatmaps
- 205 command palette entries at the time of the snapshot; current registry is larger after later parity rounds
- Session persistence, chart templates (SAVE_TEMPLATE/LOAD_TEMPLATE)
- Screenshot export (PNG)

### Broker Integration
- **Alpaca Markets**: Full REST API — connect, account, positions, orders (market/limit/stop/bracket), close position, cancel order, portfolio history, quotes, streaming (WebSocket), watchlists, options chain, corporate actions, most active, top movers
- **tastytrade**: REST API — login, accounts, positions, balances, option chains, equity order placement + DXLink WebSocket streaming (historical bars + real-time quotes)
- **MT5 (Darwinex)**: LAN sync from BarCacheWriter databases, bid/ask live quotes, 34 KV-synced analytics fields, 15 remote commands, TLS encryption

### Data Sources (21 integrated, 2 deferred)
- **Integrated:** Alpaca Markets, tastytrade, Kraken, SEC EDGAR, FRED (10 series), Finnhub, FMP, Alpha Vantage, CryptoCompare, CoinGecko, House Stock Watcher, Yahoo Finance, Treasury.gov, alternative.me, FINRA RegSHO, Pushover, ntfy.sh, Anthropic (AI chat), OpenAI (AI chat), Matrix (chat), Discord (webhooks)
- **Deferred (paid/API-gated):** whale-alert.io (needs free API key), QuiverQuant (paid API)

### Analytics
- DARWIN analytics: 80+ functions, performance attribution, D-Score components, investment velocity, tax lots, rolling correlation, diversification candidates
- Risk engine: VaR, Monte Carlo, stress test, risk-of-ruin, margin monitor
- MQL5 compiler + 8 transpiler backends (216 tests, WASM + WGSL + PineScript + ThinkScript + EasyLanguage + AFL + ACSIL + cAlgo + NinjaScript + ProBuilder)
- GPU backtester + optimizer (SMA cross + NNFX strategies)
- Compound interest calculator with growth chart

### Infrastructure
- 854 tests (216 compiler + 511 engine + 78 native + 49 web-protocol)
  *(Updated 2026-04-10 — was 575 at time of writing)*
- Zero warnings, zero unsafe blocks, zero production unwrap/expect
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
| OCO Orders | IMPLEMENTED | `AlpacaBroker::oco_order()` — order_class "oco" with TP/SL legs. `OCO` console command. |
| Draggable SL/TP | IMPLEMENTED | Draggable lines + Set SL/TP places stop/limit orders on broker (full MT5 EA parity) |
| 7 Order Types | IMPLEMENTED | market/limit/stop/stop-limit/bracket/trailing/OCO + Set SL/TP orders. OCO is implemented through Alpaca order_class "oco" and the `OCO` console command. |

## Remaining Gaps (Prioritized)

### HIGH (Functional gaps)
- ~~Draggable SL/TP lines~~ DONE
- ~~Trailing stop order type~~ DONE
- ~~Fear & Greed Index~~ DONE
- ~~World indices dashboard~~ DONE
- ~~Crypto top 50~~ DONE
- ~~tastytrade position close (no mechanism exists)~~ DONE in ADR-088/201
- ~~Periodic MT5 sync loop (currently manual command only)~~ DONE in ADR-088
- ~~EasyLanguage compiler (third frontend for MQL5 IR pipeline)~~ DONE in ADR-089/090
- ~~thinkScript compiler (fourth frontend)~~ DONE in ADR-089/090
- ~~Options Greeks display in option chain windows~~ DONE in ADR-083/088

### LOW (Nice-to-have)
- ~~Forex cross-rate matrix~~ DONE
- Dark pool/block trade volume — still provider/data-feed gated
- ~~OCO order type~~ DONE
- ~~Stop-limit order type~~ DONE
- ~~Watchlist update/delete (only create/read exist)~~ DONE in ADR-088
- ~~Drawing control point drag-to-resize (handles render but aren't interactive)~~ DONE in ADR-088
- ~~Account-history-based compound interest projection~~ DONE in ADR-088

## Consequences
- Terminal exceeds TradingView/MT5 in drawing tools, indicators, and analytical windows
- Order placement now functional for Alpaca + tastytrade (was regression from Tauri migration)
- Most originally listed gaps were closed by ADR-088 through ADR-093; dark-pool/block-trade parity remains data-provider gated.
- All security vulnerabilities addressed (constant-time HMAC, webhook hardening, log bounds, zero unwrap)
