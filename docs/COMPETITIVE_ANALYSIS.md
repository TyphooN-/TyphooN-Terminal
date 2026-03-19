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
| 37 indicators + 22 Wasm | ✅ | ✅ (38+) | ❌ | ✅ (50+) | ✅ |
| Drawing tools (6 types) | ✅ | ✅ (46) | ❌ | ❌ | ✅ |
| Auto Fibonacci | ✅ | ❌ | ❌ | ❌ | ❌ |
| Supply/Demand zones | ✅ | ✅* | ❌ | ❌ | ❌ |
| Draggable SL/TP lines | ✅ | ✅ | ❌ | ❌ | ❌ |
| 6 order types | ✅ | ✅ | ❌ | ❌ | ✅ |
| Bracket orders | ✅ | ✅ | ❌ | ❌ | ✅ |
| Pending order visualization | ✅ | ✅ | ❌ | ❌ | ✅ |
| Context menu on chart | ✅ | ✅ | ❌ | ❌ | ✅ |
| Keyboard shortcuts (15+) | ✅ | ✅ | ✅ | ✅ | ✅ |
| Harmonic pattern detection | ✅ | ❌ | ❌ | ❌ | ❌ |
| GPU-accelerated charts (WebGL2) | ✅ | ❌ | ❌ | ❌ | ❌ |
| Wasm-accelerated indicators | ✅ | ❌ | ❌ | ❌ | ❌ |
| Multi-leg order builder | ✅ | ❌ | ❌ | ❌ | ✅ |
| Pre/post-market pricing | ✅ | ❌ | ✅ | ❌ | ✅ |

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
| Risk heat map (VaR contrib) | ✅ | ❌ | ❌ | ❌ | ✅ |
| Scenario stress testing | ✅ | ❌ | ❌ | ❌ | ✅ |
| Statistical anomaly alerts | ✅ | ❌ | ❌ | ❌ | ❌ |
| Equity curve tracker | ✅ | ❌ | ❌ | ✅ | ✅ |

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
| Volume profile | ✅ | ❌ | ❌ | ❌ | ✅ |
| Market profile / TPO | ✅ | ❌ | ❌ | ❌ | ✅ |
| Market breadth | ✅ | ❌ | ❌ | ✅ | ✅ |
| Pairs trading | ✅ | ❌ | ❌ | ✅ | ✅ |
| Seasonality analysis | ✅ | ❌ | ❌ | ✅ | ✅ |
| Analyst recommendations | ❌ | ❌ | ✅ | ✅ | ✅ |
| Short interest | ❌ | ❌ | ✅ | ✅ | ✅ |
| Options flow / unusual activity | ✅ | ❌ | ❌ | ❌ | ✅ |

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
| Market replay / practice | ✅ | ❌ | ❌ | ❌ | ❌ |
| No-code strategy builder | ✅ | ❌ | ❌ | ❌ | ❌ |
| Genetic algorithm optimizer | ✅ | ❌ | ❌ | ❌ | ❌ |
| Scenario stress testing | ✅ | ❌ | ❌ | ❌ | ✅ |
| Historical pattern matching | ✅ | ❌ | ❌ | ❌ | ❌ |
| Fourier / frequency analysis | ✅ | ❌ | ❌ | ❌ | ❌ |
| Fractal dimension analysis | ✅ | ❌ | ❌ | ❌ | ❌ |
| Cointegration testing | ✅ | ❌ | ❌ | ✅ | ✅ |
| Shadow paper portfolio | ✅ | ❌ | ❌ | ❌ | ❌ |
| Prediction accuracy tracking | ✅ | ❌ | ❌ | ❌ | ❌ |

## Platform & Infrastructure

| Feature | TT | MT5 | GT | OBB | BBG |
|---|---|---|---|---|---|
| Native desktop (local render) | ✅ | ✅ | ❌ (web) | ❌ (CLI) | ✅ |
| Linux native | ✅ | ❌ (Wine) | ✅ (web) | ✅ | ❌ |
| Open source | ✅ | ❌ | ❌ | ✅ | ❌ |
| Broker support | ✅ (Alpaca) | ❌ (1) | ❌ | ✅ | ✅ |
| OS keychain storage | ✅ | ❌ | ❌ | ❌ | ✅ |
| SQLite + zstd cache | ✅ | ❌ | ❌ | ❌ | ❌ |
| WebSocket streaming | ✅ | ✅ | ✅ | ❌ | ✅ |
| AI assistant | ✅ | ❌ | ❌ | ❌ | ✅ |
| Push notifications | ✅ | ✅ | ❌ | ❌ | ✅ |
| GPU-accelerated charts (WebGL2) | ✅ | ❌ | ❌ | ❌ | ❌ |
| Wasm indicator engine (22 funcs) | ✅ | ❌ | ❌ | ❌ | ❌ |
| Web Worker computation | ✅ | ❌ | ❌ | ❌ | ❌ |
| Voice + tonal audio alerts | ✅ | ❌ | ❌ | ❌ | ❌ |
| Price sonification (audio) | ✅ | ❌ | ❌ | ❌ | ❌ |
| Focus mode (distraction-free) | ✅ | ❌ | ❌ | ❌ | ❌ |
| PDT rule monitor | ✅ | ❌ | ❌ | ❌ | ❌ |
| Tax lot tracker (FIFO + wash sale) | ✅ | ❌ | ❌ | ❌ | ✅ |
| 228 command palette entries | ✅ | ❌ | ❌ | ❌ | ❌ |
| 602 static analysis assertions | ✅ | ❌ | ❌ | ❌ | ❌ |
| Cost | Free | Free* | $80-118/mo | Free | $24K/yr |

*MT5 is free but broker-locked

## Where TyphooN-Terminal LEADS

1. **Risk management depth** — 4 modes + VaR + hedged martingale + break-even + equity protection. No other platform has this built-in.
2. **Auto Fibonacci** — Automatic fractal-based swing detection with 13 levels including extensions. Unique feature.
3. **Supply/Demand zones** — Fractal-based detection with strength tiers. Only available as paid MT5 indicators.
4. **Integrated NNFX system** — Full No Nonsense Forex system ported from MQL5 with exact visual parity.
5. **Multi-timeframe** — Alpaca with 2-5 TF grid view per symbol.
6. **Local-first with cloud data** — Runs locally (no SaaS), caches everything in SQLite, works offline with cached data.
7. **Draggable tab reordering** — Drag tabs to rearrange. Godel Terminal doesn't support this (tabs are fixed order, very annoying). MT5 doesn't support it either (chart windows, not tabs).
8. **Volume Profile** — Price-at-volume distribution with POC and value area. TradingView charges for this feature (Premium plan, $24.95/mo). Free in TyphooN-Terminal.
9. **Market Profile / TPO** — Time-Price-Opportunity charts matching Bloomberg/CQG institutional tools. No other free platform offers this.
10. **Options flow / unusual activity** — Synthetic flow analysis from options chain volume vs open interest. Replaces FlowAlgo ($100/mo) and Unusual Whales ($40/mo) for basic flow detection.
11. **Market replay with simulated trading** — Bar-by-bar historical replay with paper trading. Matches TradingView Replay (Premium feature). Free in TyphooN-Terminal.
12. **Multi-leg order builder** — Complex options/stock combo orders. Matches Interactive Brokers ComboTrader.
13. **Scenario stress testing** — Portfolio stress test against historical events (2008 crash, COVID, etc.). Matches Bloomberg PORT risk analytics.
14. **Statistical anomaly detection** — Smart alerts that detect unusual price/volume/volatility patterns. Unique feature — no competitor offers this.
15. **No-code strategy builder** — Visual strategy builder without writing code. Comparable to TradingView Pine Script but visual/drag-and-drop.
16. **Pairs trading with cointegration** — Statistical pairs analysis with z-score signals. Professional quant feature usually found only in institutional platforms.
17. **Heatmap order book (Bookmap)** — Order book depth over time as a 2D heatmap. Comparable to Bookmap ($40/mo). Canvas rendering.
18. **Customizable dashboard** — 8-widget configurable grid with auto-refresh. Comparable to Bloomberg LAUNCHPAD.
19. **Real-time scanner** — Multi-condition scanner with 7 preset conditions, 60-second polling, browser notifications.
20. **Composite trading signal** — 0-100 score aggregating 6 indicators (Fisher, RSI, KAMA, SMA, volume, ATR). Unique feature.
21. **Price ladder / DOM** — Vertical bid-ask depth with volume bars. Standard on CQG and TT.
22. **Theme switcher** — Dark, pitch black (OLED), light themes. Accessibility feature competitors lack.
23. **Webhook alert automation** — Custom webhook endpoints for integrating with Discord bots, Zapier, etc.
24. **228 Ctrl+K commands** — More command palette entries than any trading terminal, open or proprietary.
25. **AI-powered strategy suggestions** — Contextual NNFX analysis via Claude/GPT with Fisher/RSI/KAMA/SMA200/volume context.
26. **Voice alerts** — Web Speech API reads alerts aloud. No competitor has this.
27. **Data quality monitoring** — Automatic detection of missing bars, OHLC violations, suspicious spikes.
28. **Performance profiler** — Built-in latency/memory/cache monitoring. No trading terminal offers this.
29. **Risk control center** — Unified margin/VaR/concentration/PDT status in one dashboard.
30. **Pre/post-market pricing** — Snapshot endpoint for extended hours trades on IEX (free tier).
31. **Session persistence** — Full state restore including MTF grid, chart zoom, panel states, news articles.
32. **Fourier / FFT frequency analysis** — Detect dominant price cycles. No trading terminal offers this natively.
33. **Shannon entropy of returns** — Measure market predictability. Academic quant tool made accessible.
34. **Fractal dimension (Higuchi)** — More robust than Hurst exponent for regime classification. Unique.
35. **Wavelet decomposition** — Haar wavelets decompose price into 5 frequency bands. Academic-grade.
36. **Engle-Granger cointegration test** — Statistical validation for pairs trading. Institutional quant feature.
37. **Price sonification** — Hear price action via Web Audio API. No trading platform has this.
38. **Harmonic pattern detection** — Gartley, Butterfly, Bat, Crab auto-detected from fractal swings.
39. **Genetic algorithm optimizer** — Evolve strategy parameters via tournament/crossover/mutation. Beyond grid search.
40. **Historical pattern matching** — DTW-based "find similar patterns" with forward return prediction.
41. **Radar chart (8 indicators)** — Spider chart for at-a-glance multi-indicator assessment.
42. **Liquidity heatmap** — Volume-weighted price heatmap showing institutional support/resistance.
43. **Shadow trading** — Parallel paper portfolio to compare alternative sizing/SL strategies.
44. **Tonal audio alerts** — Distinct frequency tones for Fisher/RSI/multi-signal events. Non-verbal.
45. **Prediction accuracy tracking** — Track your own win rate by setup type. Self-improvement tool.
46. **Focus mode** — Distraction-free trading with F12 toggle. Hides all panels except chart + trade buttons.
47. **Macro recording** — Record and replay command sequences + keyboard actions.
48. **Workspace save/restore** — Full app state persistence including all settings and window layouts.
49. **PDT monitor** — Pattern Day Trader rule tracking with countdown and warning banners.
50. **Tax lot tracker** — FIFO cost basis with wash sale detection and estimated tax impact.
51. **602 static analysis assertions** — Static analysis smoke test covering every command, function, and security invariant.

## UX Advantages Over Competitors

| Feature | TT | MT5 | GT | cTrader | NinjaTrader |
|---|---|---|---|---|---|
| Drag-reorder tabs | ✅ | ❌ | ❌ | ❌ | ❌ |
| Ctrl+K command palette | ✅ | ❌ | ✅ | ❌ | ❌ |
| Right-click context menu | ✅ | ✅ | ❌ | ✅ | ✅ |
| Keyboard shortcuts (15+) | ✅ | ✅ | ✅ | ✅ | ✅ |
| Custom timeframes (2H, 3H, 6H) | ✅ | ❌ | ❌ | ✅ | ✅ |
| Heikin-Ashi + Renko | ✅ | ✅ | ❌ | ✅ | ✅ |
| AI assistant | ✅ | ❌ | ❌ | ❌ | ❌ |
| Trade journal (enhanced) | ✅ | ❌ | ❌ | ❌ | ❌ |
| Risk/reward overlay | ✅ | ❌ | ❌ | ❌ | ❌ |
| Radar chart (8 indicators) | ✅ | ❌ | ❌ | ❌ | ❌ |
| Settings backup/restore | ✅ | ❌ | ❌ | ❌ | ✅ |
| Workspace save/restore | ✅ | ❌ | ❌ | ❌ | ✅ |
| Macro recording | ✅ | ❌ | ❌ | ❌ | ✅ |
| Dark/Light/OLED themes | ✅ | ❌ | ❌ | ❌ | ❌ |
| Webhook automation | ✅ | ❌ | ❌ | ❌ | ❌ |
| Chart template sharing | ✅ | ❌ | ❌ | ❌ | ❌ |
| Minimap (chart overview) | ✅ | ❌ | ❌ | ❌ | ❌ |
| Hotkey panel (customizable) | ✅ | ❌ | ❌ | ❌ | ❌ |
| Focus mode (F12) | ✅ | ❌ | ❌ | ❌ | ❌ |
| Risk/reward overlay | ✅ | ❌ | ❌ | ❌ | ❌ |

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
- **Drawing tools** — 22 types implemented (trend, fib, h-line, rectangle, channel, ray, ruler, etc.). MT5 has 46 total; remaining ~24 are niche objects (Elliott Wave labels, Gann Grid, etc.)
- ~~**Indicator count**~~ **RESOLVED** — MT5 has 38+ built-in, we now have **37 unique indicators** + 22 Wasm implementations. Parity achieved with Alligator, AO, MFI, Force Index, Envelopes, StdDev, Chaikin, DeMarker, Fractals.
- **EA/Expert Advisor system** — MT5 has a full algorithmic trading framework (MQL5). We have auto-trade + genetic optimizer but no custom language.
- **Strategy tester** — MT5's visual tester has better drag speed. Our replay mode + visual backtester are functionally equivalent.

### vs OpenBB
- ~~**Quantitative analysis**~~ **RESOLVED** — Fourier analysis, Shannon entropy, fractal dimension, wavelet decomposition, cointegration testing, genetic optimization, AND Jarque-Bera normality test. Only CAPM factor model remains (needs multi-factor return data).
- **Crypto on-chain** — Blockchain analytics (Etherscan, etc.) — blocked by external APIs
- ~~**Government data**~~ **RESOLVED** — FRED integration provides Fed Funds, CPI, GDP, Treasury yields, VIX, M2
- **Jupyter integration** — OpenBB runs in notebooks. We export data via CSV/clipboard for external analysis.

### vs TradingView ($0-60/mo)
TradingView is the most popular retail charting platform. Feature comparison:

| Feature | TyphooN-Terminal | TradingView Free | TradingView Premium ($60/mo) |
|---|---|---|---|
| Candlestick charts | ✅ | ✅ | ✅ |
| 37 indicators | ✅ | ✅ (limited) | ✅ (100+) |
| 22 drawing tools | ✅ | ✅ (limited) | ✅ (50+) |
| Custom timeframes | ✅ | ❌ | ✅ |
| Volume Profile | ✅ (free) | ❌ | ✅ ($24.95/mo+) |
| Market Replay | ✅ (free) | ❌ | ✅ ($24.95/mo+) |
| Pine Script (custom lang) | ❌ (JS plugins) | ✅ | ✅ |
| Server-side alerts | ❌ (local only) | ✅ (limited) | ✅ (unlimited) |
| Multi-chart layout | ✅ (MTF grid) | ❌ (1 chart) | ✅ (8 charts) |
| Paper trading | ✅ (real broker) | ✅ (simulated) | ✅ |
| Real order execution | ✅ (Alpaca) | ❌ | ✅ (limited brokers) |
| Open source | ✅ | ❌ | ❌ |
| No ads | ✅ | ❌ (heavy ads) | ✅ |
| Local-first (no cloud) | ✅ | ❌ (cloud-only) | ❌ |
| GPU-accelerated charts | ✅ | ❌ | ❌ |
| Wasm indicator engine | ✅ | ❌ | ❌ |
| Risk management (4 modes) | ✅ | ❌ | ❌ |
| AI strategy suggestions | ✅ | ❌ | ❌ |
| Voice/audio alerts | ✅ | ❌ | ❌ |
| Fourier/wavelet/entropy | ✅ | ❌ | ❌ |
| 228 Ctrl+K commands | ✅ | ❌ | ❌ |
| Cost | **Free** | Free (limited) | **$60/mo ($720/yr)** |

**TyphooN-Terminal advantages**: Volume Profile and Market Replay are free (TradingView charges $24.95+/mo). Local-first with no ads. Real order execution via broker API. GPU charts, Wasm indicators, 4 risk modes, AI strategy — none available on TradingView at any price.

**TradingView advantages**: Pine Script ecosystem, server-side alerts (work when PC is off), mobile app, social features (ideas, chat), 100+ indicators, 50+ drawing tools, professional data feeds.

### vs cTrader / NinjaTrader / Thinkorswim

| Feature | TyphooN-Terminal | cTrader | NinjaTrader | Thinkorswim |
|---|---|---|---|---|
| Open source | ✅ | ❌ | ❌ | ❌ |
| Linux native | ✅ | ❌ | ❌ | ❌ |
| Cost | Free | Free | $720/yr or $1,099 | Free (with TD) |
| Risk management | 4 modes + VaR | Basic | Basic | Basic |
| AI assistant | ✅ | ❌ | ❌ | ❌ |
| GPU charts | ✅ | ❌ | ❌ | ❌ |
| Quant analysis | ✅ (FFT, entropy, wavelets) | ❌ | ❌ | ❌ |
| DLL-free (no binary deps) | ✅ | ❌ (.NET) | ❌ (.NET) | ❌ (Java) |
| Custom indicators | JS (readable) | C# (compiled) | C# (compiled) | thinkScript |
| Options analytics | ✅ | Limited | ✅ | ✅ |
| Harmonic patterns | ✅ | ❌ | Add-on ($) | ❌ |

## Future Feature Priorities (Ranked by Impact)

### Tier A — High Impact, Achievable Now
1. ~~**Conditional order placement**~~ ✅ DONE — Ctrl+K → BRACKET (OCO + bracket)
2. ~~**Portfolio heat map**~~ ✅ DONE — Ctrl+K → HEATMAP (finviz-style)
3. ~~**Risk/reward overlay**~~ ✅ DONE — visual P&L zones on chart
4. ~~**Multi-symbol alert dashboard**~~ ✅ DONE — Ctrl+K → ALERTBOARD
5. ~~**Trade journal**~~ ✅ DONE — Ctrl+K → JOURNAL

### Tier B — Medium Impact, Moderate Effort
6. ~~**Heikin-Ashi candlesticks**~~ ✅ DONE — chart type selector
7. ~~**Renko/Range bars**~~ ✅ DONE — ATR-based Renko in chart type selector
8. ~~**Custom timeframes**~~ ✅ DONE — 2H, 3H, 6H, 2D, 3D via aggregation
9. ~~**Chart annotations**~~ ✅ DONE — Ctrl+K → ANNOTATE, markers on chart
10. ~~**Position sizing calculator**~~ ✅ DONE — Ctrl+K → CALC

### Tier C — Differentiators (Unique Features)
11. ~~**AI-powered trade review**~~ ✅ DONE — "Review My Trades" in AI chat
12. ~~**Pattern recognition**~~ ✅ DONE — Ctrl+K → PATTERNS (double top/bottom, H&S)
13. ~~**Sentiment analysis**~~ ✅ DONE — Ctrl+K → SENTIMENT (keyword scoring)
14. ~~**Volatility surface**~~ ✅ DONE — Ctrl+K → VOLSURF (strike×expiry IV grid)
15. ~~**Regime detection**~~ ✅ DONE — ADX-based trending/ranging/choppy in timer

**ALL 15 Tier A/B/C features are now implemented.**

## Why Open Source Matters for Trading Software

> "How can you trust your wealth if you cannot audit the code?"

### The Problem with Closed-Source Trading Platforms

| Platform | Source | Binary | DLL Risk | Audit |
|---|---|---|---|---|
| **TyphooN-Terminal** | Open (Apache-2.0) | Rust/Tauri | No DLLs | Full audit possible |
| **MetaTrader 5** | Closed | Proprietary .exe | Requires DLLs for indicators | Cannot audit |
| **cTrader** | Closed | .NET binary | Requires DLLs for cBots | Cannot audit |
| **NinjaTrader** | Closed | .NET binary | Requires DLLs for strategies | Cannot audit |
| **Godel Terminal** | Closed | Web app (SaaS) | N/A (server-side) | Cannot audit |
| **Bloomberg** | Closed | Proprietary | N/A | Cannot audit |
| **OpenBB** | Open (Apache-2.0) | Python | No DLLs | Full audit possible |

### DLL Hell in Trading

MT5, cTrader, and NinjaTrader all rely on **third-party DLLs** for indicators and strategies:
- **MT5**: Custom indicators compiled as `.ex5` (obfuscated MQL5 bytecode) — you cannot read the code. Third-party EAs often require `#import` of opaque `.dll` files that could contain anything: keyloggers, credential theft, order manipulation.
- **cTrader**: cBots and indicators are .NET assemblies (`.algo` files). While .NET is decompilable, most vendors obfuscate. The platform itself is closed-source .NET.
- **NinjaTrader**: Strategies are .NET DLLs. The platform requires admin access and installs deeply into Windows. Third-party indicators are compiled binaries you can't inspect.

**The risk**: You're trusting your brokerage credentials and trading capital to software you cannot verify. A malicious indicator could:
- Exfiltrate your API keys
- Place unauthorized orders
- Modify your stop losses
- Send your account data to a remote server

### TyphooN-Terminal's Approach

- **100% open source** — every line auditable (30,029 JS + 7,009 Rust lines, 21 security passes)
- **No DLLs** — pure Rust backend + JavaScript frontend, no binary dependencies
- **Custom indicator plugins** are plain JavaScript files you can read
- **API keys AES-256-GCM encrypted** (PBKDF2 100K iterations), stored in SQLite — not in config files
- **CSP prevents** external script injection even if the app is compromised
- **zeroize** crate erases credentials from memory on drop

### cTrader vs TyphooN-Terminal

| Feature | cTrader | TyphooN-Terminal |
|---|---|---|
| Open source | No | Yes (Apache-2.0) |
| Language | C# (.NET) | Rust + JavaScript |
| Linux native | No (Windows only) | Yes |
| DLL required | Yes (cBots) | No |
| Broker lock-in | Yes (per broker) | No (Alpaca) |
| Risk management | Basic | 4 modes + VaR + martingale |
| Strategy testing | Yes (cAlgo) | Yes (Strategy trait) |
| Custom indicators | C# (compiled) | JavaScript (readable) |

### NinjaTrader vs TyphooN-Terminal

| Feature | NinjaTrader | TyphooN-Terminal |
|---|---|---|
| Open source | No | Yes (Apache-2.0) |
| Cost | $1,099 lifetime or $720/yr | Free |
| Language | C# (.NET) | Rust + JavaScript |
| Linux native | No (Windows only) | Yes |
| DLL required | Yes (strategies) | No |
| Admin access | Required | Not required |
| Data fees | $99-299/mo for CME/CBOT | Free (Alpaca IEX) |
| Risk management | Basic | 4 modes + VaR + martingale |

### Tier D — Blocked by External Dependencies
16. Analyst consensus (needs paid data)
17. Short interest (needs paid data)
18. ~~Dark pool~~ — Done (synthetic flow from options chain data)
19. World indices (needs non-US data)
20. ~~Community chat~~ — Done (Matrix protocol)
