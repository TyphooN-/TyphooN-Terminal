# TyphooN-Terminal Competitive Analysis

## Platform Comparison Matrix

### Legend
- **TT** = TyphooN-Terminal
- **MT5** = MetaTrader 5
- **GT** = Godel Terminal ($80-118/mo)
- **OBB** = OpenBB Terminal (free, CLI)
- **BBG** = Bloomberg Terminal ($24K/yr)

## Core Trading Features

| Feature | TT | MT5 | GT | OBB | BBG |
|---|---|---|---|---|---|
| Candlestick charts | ✅ | ✅ | ✅ | ❌ | ✅ |
| Line/Bar charts | ✅ | ✅ | ✅ | ❌ | ✅ |
| Multi-timeframe grid | ✅ | ✅ | ❌ | ❌ | ✅ |
| 30 indicators | ✅ | ✅ (38+) | ❌ | ✅ (50+) | ✅ |
| Drawing tools (6 types) | ✅ | ✅ (46) | ❌ | ❌ | ✅ |
| Auto Fibonacci | ✅ | ❌ | ❌ | ❌ | ❌ |
| Supply/Demand zones | ✅ | ✅* | ❌ | ❌ | ❌ |
| Draggable SL/TP lines | ✅ | ✅ | ❌ | ❌ | ❌ |
| 6 order types | ✅ | ✅ | ❌ | ❌ | ✅ |
| Bracket orders | ✅ | ✅ | ❌ | ❌ | ✅ |
| Pending order visualization | ✅ | ✅ | ❌ | ❌ | ✅ |
| Context menu on chart | ✅ | ✅ | ❌ | ❌ | ✅ |
| Keyboard shortcuts (15) | ✅ | ✅ | ✅ | ✅ | ✅ |

## Risk Management

| Feature | TT | MT5 | GT | OBB | BBG |
|---|---|---|---|---|---|
| 4 risk modes (Std/Fix/Dyn/VaR) | ✅ | ❌* | ❌ | ❌ | ❌ |
| VaR per-position | ✅ | ❌ | ❌ | ✅ | ✅ |
| Hedged martingale (TRIM/PROTECT) | ✅ | ❌* | ❌ | ❌ | ❌ |
| Break-even detection | ✅ | ❌ | ❌ | ❌ | ❌ |
| Equity TP/SL protection | ✅ | ❌ | ❌ | ❌ | ❌ |
| Margin level monitoring | ✅ | ✅ | ❌ | ❌ | ✅ |
| Spread tolerance | ✅ | ❌ | ❌ | ❌ | ❌ |
| Monte Carlo risk of ruin | ✅ | ❌ | ❌ | ✅ | ✅ |
| Correlation matrix | ✅ | ❌ | ❌ | ✅ | ✅ |

*MT5 has these via TyphooN EA — our own MQL5 code, not native MT5

## Research & Data

| Feature | TT | MT5 | GT | OBB | BBG |
|---|---|---|---|---|---|
| Command palette | ✅ | ❌ | ✅ | ✅ | ✅ |
| SEC fundamentals (EDGAR) | ✅ | ❌ | ✅ | ✅ | ✅ |
| SEC filings search | ✅ | ❌ | ✅ | ✅ | ✅ |
| Financial analysis (IS/BS/CF) | ✅ | ❌ | ✅ | ✅ | ✅ |
| Institutional holders (13F) | ✅ | ❌ | ✅ | ✅ | ✅ |
| Insider trading (Form 4) | ✅ | ❌ | ✅ | ✅ | ✅ |
| Options chain with Greeks | ✅ | ❌ | ✅ | ✅ | ✅ |
| Stock screener | ✅ | ❌ | ✅ | ✅ | ✅ |
| Most active / top movers | ✅ | ❌ | ✅ | ✅ | ✅ |
| News feed (in-app reader) | ✅ | ❌ | ✅ | ✅ | ✅ |
| Earnings calendar | ✅ | ❌ | ✅ | ✅ | ✅ |
| Economic calendar (FRED) | ✅ | ❌ | ❌ | ✅ | ✅ |
| Watchlist / quote monitor | ✅ | ✅ | ✅ | ❌ | ✅ |
| Bid/Ask spread display | ✅ | ✅ | ✅ | ❌ | ✅ |
| Time & Sales | ✅ | ✅ | ❌ | ❌ | ✅ |
| DOM / Level 2 (crypto) | ✅ | ✅ | ❌ | ❌ | ✅ |
| Analyst recommendations | ❌ | ❌ | ✅ | ✅ | ✅ |
| Short interest | ❌ | ❌ | ✅ | ✅ | ✅ |
| Dark pool / options flow | ❌ | ❌ | ❌ | ❌ | ✅ |

## Strategy Testing

| Feature | TT | MT5 | GT | OBB | BBG |
|---|---|---|---|---|---|
| Backtester (Strategy trait) | ✅ | ✅ | ❌ | ✅ | ❌ |
| Visual backtester (equity curve) | ✅ | ✅ | ❌ | ❌ | ❌ |
| Walk-forward testing | ✅ | ✅ | ❌ | ❌ | ❌ |
| Grid optimization | ✅ | ✅ | ❌ | ❌ | ❌ |
| Monte Carlo simulation | ✅ | ❌ | ❌ | ✅ | ❌ |
| Custom indicator plugins | ✅ | ✅ | ❌ | ✅ | ❌ |
| CSV trade export | ✅ | ✅ | ❌ | ✅ | ✅ |

## Platform & Infrastructure

| Feature | TT | MT5 | GT | OBB | BBG |
|---|---|---|---|---|---|
| Native desktop (local render) | ✅ | ✅ | ❌ (web) | ❌ (CLI) | ✅ |
| Linux native | ✅ | ❌ (Wine) | ✅ (web) | ✅ | ❌ |
| Open source | ✅ | ❌ | ❌ | ✅ | ❌ |
| Multi-broker support | ✅ (2) | ❌ (1) | ❌ | ✅ | ✅ |
| OS keychain storage | ✅ | ❌ | ❌ | ❌ | ✅ |
| SQLite + zstd cache | ✅ | ❌ | ❌ | ❌ | ❌ |
| WebSocket streaming | ✅ | ✅ | ✅ | ❌ | ✅ |
| AI assistant | ✅ | ❌ | ❌ | ❌ | ✅ |
| Push notifications | ✅ | ✅ | ❌ | ❌ | ✅ |
| Cost | Free | Free* | $80-118/mo | Free | $24K/yr |

*MT5 is free but broker-locked

## Where TyphooN-Terminal LEADS

1. **Risk management depth** — 4 modes + VaR + hedged martingale + break-even + equity protection. No other platform has this built-in.
2. **Auto Fibonacci** — Automatic fractal-based swing detection with 13 levels including extensions. Unique feature.
3. **Supply/Demand zones** — Fractal-based detection with strength tiers. Only available as paid MT5 indicators.
4. **Integrated NNFX system** — Full No Nonsense Forex system ported from MQL5 with exact visual parity.
5. **Multi-broker + multi-timeframe** — Alpaca + Tastytrade with 2-5 TF grid view per symbol.
6. **Local-first with cloud data** — Runs locally (no SaaS), caches everything in SQLite, works offline with cached data.

## Where TyphooN-Terminal TRAILS

### vs Bloomberg ($24K/yr)
- **Real-time news speed** — Bloomberg has fastest news delivery (milliseconds)
- **Historical depth** — Bloomberg has 30+ years of tick data
- **Fixed income / credit** — Bond trading, yield curves, credit analysis
- **IM/Chat** — Bloomberg messaging is an industry social network
- **Portfolio analytics** — Multi-asset attribution, factor analysis

### vs Godel Terminal ($80-118/mo)
- **Analyst consensus** — Godel aggregates sell-side analyst ratings
- **Short interest** — Real-time short interest data
- **Speed** — Godel optimizes for sub-second research queries

### vs MT5
- **Drawing tools** — MT5 has 46 drawing objects vs our 6
- **Indicator count** — MT5 has 38+ built-in vs our 30
- **EA/Expert Advisor system** — MT5 has a full algorithmic trading framework (MQL5)
- **Strategy tester** — MT5's visual tester is more polished (drag speed, visual replay)

### vs OpenBB
- **Quantitative analysis** — OpenBB has normality tests, CAPM, factor models
- **Crypto on-chain** — Blockchain analytics (Etherscan, etc.)
- **Government data** — Fed speakers, treasury auctions
- **Jupyter integration** — OpenBB runs in notebooks for research workflows

## Future Feature Priorities (Ranked by Impact)

### Tier A — High Impact, Achievable Now
1. **Conditional order placement** (OCO, OTO bracket management UI)
2. **Portfolio heat map** (sector/position size visualization)
3. ~~**Risk/reward overlay**~~ ✅ DONE — visual P&L zones on chart
4. **Multi-symbol alert dashboard** (check alerts across all watchlist symbols)
5. ~~**Trade journal**~~ ✅ DONE — Ctrl+K → JOURNAL

### Tier B — Medium Impact, Moderate Effort
6. ~~**Heikin-Ashi candlesticks**~~ ✅ DONE — chart type selector
7. **Renko/Range bars** (non-time-based charting)
8. **Custom timeframes** (e.g., 2H, 3D, 6H)
9. ~~**Chart annotations**~~ ✅ DONE — Ctrl+K → ANNOTATE, markers on chart
10. ~~**Position sizing calculator**~~ ✅ DONE — Ctrl+K → CALC

### Tier C — Differentiators (Unique Features)
11. **AI-powered trade review** ("analyze my last 10 trades" using AI chat context)
12. **Pattern recognition** (auto-detect head & shoulders, double top/bottom, wedges)
13. **Sentiment analysis** (aggregate news sentiment per symbol)
14. **Volatility surface** (options IV surface visualization)
15. ~~**Regime detection**~~ ✅ DONE — ADX-based trending/ranging/choppy shown in timer

### Tier D — Blocked by External Dependencies
16. Analyst consensus (needs paid data)
17. Short interest (needs paid data)
18. Dark pool (needs paid data)
19. World indices (needs non-US data)
20. Community chat (needs server)
