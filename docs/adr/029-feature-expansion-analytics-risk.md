# ADR-029: Feature Expansion — Analytics & Risk Tools (36 New Commands)

**Status:** Implemented
**Date:** 2026-03-17

## Context

After completing the core trading terminal, research platform, and strategy testing framework (ADRs 001-028), the terminal needed deeper analytical tools to match professional platforms like Bloomberg PORT, FlowAlgo, TradingView Premium, and CQG. Two batches of features were added in a single session — 16 commands in the first batch and 20 in the second — for a total of 36 new Ctrl+K commands.

**Key architectural decision:** All 36 features use existing cached data (bar cache, options chain, positions, watchlist). Zero new API endpoints were added. This preserves the rate limit budget and adds no new network latency.

## Features Added

### Batch 1 — Options Analytics & Chart Tools (16 Commands)

#### Options Analytics (5)

| Command | Ctrl+K | Description | Data Source |
|---|---|---|---|
| PCRATIO | `Ctrl+K → PCRATIO` | Weighted put/call ratio from options chain | Alpaca options chain (cached) |
| UNUSUAL | `Ctrl+K → UNUSUAL` | Unusual options activity — volume vs open interest spikes | Alpaca options chain (cached) |
| IVRANK | `Ctrl+K → IVRANK` | IV rank and IV percentile in 52-week context | Alpaca options chain + bar cache |
| GREEKS | `Ctrl+K → GREEKS` | Aggregate portfolio Greeks (delta, gamma, theta, vega) | Alpaca options positions |
| OPTPROFIT | `Ctrl+K → OPTPROFIT` | Theoretical P&L scenario at target price and date | Alpaca options chain (cached) |

#### Chart Tools (11)

| Command | Ctrl+K | Description | Data Source |
|---|---|---|---|
| COMPARE | `Ctrl+K → COMPARE` | Normalized overlay of up to 5 symbols | Bar cache (multiple symbols) |
| SPREAD | `Ctrl+K → SPREAD` | Price ratio or difference between two symbols | Bar cache (two symbols) |
| SRLEVEL | `Ctrl+K → SRLEVEL` | Automatic support/resistance from pivot and fractal analysis | Bar cache (current symbol) |
| DIVERGENCE | `Ctrl+K → DIVERGENCE` | RSI and MACD vs price divergence detection | Bar cache + indicator calculations |
| VOLUME | `Ctrl+K → VOLUME` | Volume profile — price-at-volume distribution with POC and value area | Bar cache (current symbol) |
| PIVOTS | `Ctrl+K → PIVOTS` | Classic, Fibonacci, and Woodie pivot point levels | Bar cache (daily bars) |
| PERF | `Ctrl+K → PERF` | Relative performance — symbol vs benchmark (SPY) as % | Bar cache (two symbols) |
| VWAP+ | `Ctrl+K → VWAP+` | Anchored VWAP with 1σ and 2σ standard deviation bands | Bar cache (intraday bars) |
| REPLAY | `Ctrl+K → REPLAY` | Market replay — bar-by-bar historical playback with simulated trading | Bar cache (any TF) |
| TRADESTATS | `Ctrl+K → TRADESTATS` | Trade statistics — win rate, expectancy, R-multiple, consecutive W/L | Trade history (cached) |
| PAIRS | `Ctrl+K → PAIRS` | Pairs trading — cointegration test, spread, z-score entry signals | Bar cache (two symbols) |

### Batch 2 — Market Analysis, Risk & Advanced Tools (20 Commands)

#### Market Analysis (7)

| Command | Ctrl+K | Description | Data Source |
|---|---|---|---|
| BREADTH | `Ctrl+K → BREADTH` | Market breadth dashboard — advance/decline, new highs/lows, McClellan oscillator | Alpaca screener + bar cache |
| FLOWS | `Ctrl+K → FLOWS` | Institutional money flows — sector ETF volume/price divergence analysis | Bar cache (sector ETFs) |
| GAPS | `Ctrl+K → GAPS` | Gap analysis — unfilled gap detection with fill probability estimates | Bar cache (daily bars) |
| RELSTRENGTH | `Ctrl+K → RELSTRENGTH` | Mansfield relative strength ranking vs benchmark | Bar cache (multiple symbols) |
| SEASONALITY | `Ctrl+K → SEASONALITY` | Monthly/weekly return patterns from multi-year historical data | Bar cache (monthly/weekly bars) |
| CORRWATCH | `Ctrl+K → CORRWATCH` | Correlation watchdog — alerts when pairwise correlations break regime | Bar cache (watchlist symbols) |
| FLOWMAP | `Ctrl+K → FLOWMAP` | Sector money flow visualization — Sankey-style flow diagram | Bar cache (sector ETFs) |

#### Risk & Portfolio (5)

| Command | Ctrl+K | Description | Data Source |
|---|---|---|---|
| RISKMAP | `Ctrl+K → RISKMAP` | Portfolio VaR contribution heat map by position | Positions + bar cache |
| RISKSIM | `Ctrl+K → RISKSIM` | Stress test portfolio against historical scenarios (2008, COVID, etc.) | Positions + bar cache |
| EQUITY | `Ctrl+K → EQUITY` | Live equity curve tracker with drawdown overlay | Account history + positions |
| SMARTALERT | `Ctrl+K → SMARTALERT` | Statistical anomaly detection — volume, volatility, price pattern alerts | Bar cache + indicator calculations |
| REGIME+ | `Ctrl+K → REGIME+` | Enhanced regime detection — volatility regimes + mean reversion signals | Bar cache + ADX/ATR calculations |

#### Trading Tools (5)

| Command | Ctrl+K | Description | Data Source |
|---|---|---|---|
| MTFDIV | `Ctrl+K → MTFDIV` | Multi-timeframe divergence scanner — cross-TF indicator divergence | Bar cache (multiple TFs) |
| MULTILEG | `Ctrl+K → MULTILEG` | Multi-leg order builder for complex options/stock combinations | Alpaca orders API |
| BACKTEST+ | `Ctrl+K → BACKTEST+` | No-code visual strategy builder with drag-and-drop conditions | Bar cache + indicator engine |
| SCANNER+ | `Ctrl+K → SCANNER+` | Enhanced scanner — multi-factor custom screening with saved filters | Alpaca screener + bar cache |
| ORDERFLOW | `Ctrl+K → ORDERFLOW` | Trade tape aggregation — buy/sell delta, cumulative delta chart | WebSocket trade stream |

#### Data Visualization (3)

| Command | Ctrl+K | Description | Data Source |
|---|---|---|---|
| MARKETPROFILE | `Ctrl+K → MARKETPROFILE` | Time-Price-Opportunity (TPO) distribution chart | Bar cache (intraday bars) |
| HEATCAL | `Ctrl+K → HEATCAL` | Calendar heat map — daily returns colored by magnitude | Bar cache (daily bars) |
| ECALENDAR | `Ctrl+K → ECALENDAR` | Enhanced economic calendar — ForexFactory-style with impact filters | ForexFactory + FRED data |

## Architectural Decisions

### 1. Zero New API Endpoints

All 36 features derive analytics from data already fetched and cached by the existing pipeline:
- **Bar cache** (4-tier: memory LRU → IndexedDB → SQLite+zstd → zstd files)
- **Options chain** (fetched via existing `get_options_chain` command)
- **Positions/orders** (fetched via existing `get_positions` / `get_open_orders`)
- **Watchlist** (stored in session persistence)
- **Trade history** (fetched via existing `get_order_history`)

This was a deliberate decision to avoid increasing API rate limit pressure (200 req/min Alpaca budget is shared across all features).

### 2. Frontend-Only Implementation

All 36 features are implemented in the frontend JavaScript. No new Rust/Tauri commands were added. This keeps the backend lean and avoids Tauri IPC overhead for derived calculations.

### 3. Floating Window Pattern

All new features use the established floating window pattern (`createFloatingWindow()`) for consistency. Each window is independently closeable and does not interfere with the chart or other panels.

### 4. Code Growth

The main `main.js` grew from approximately 11,000 lines to approximately 15,400 lines (+4,400 lines, ~40% increase). This is acceptable given:
- Each command is self-contained (function + floating window)
- No increase in cyclomatic complexity of existing code
- No new external dependencies
- All features are lazy-loaded (only instantiated when user invokes the command)

## Competitive Impact

These 36 features close significant gaps with paid platforms:

| Capability | Previously Matched | Now Matches |
|---|---|---|
| Volume Profile | No competitor | TradingView Premium ($24.95/mo) |
| Market Profile / TPO | No competitor | Bloomberg, CQG (institutional) |
| Options flow detection | No competitor | FlowAlgo ($100/mo), Unusual Whales ($40/mo) |
| Market replay | No competitor | TradingView Replay (Premium) |
| Multi-leg orders | No competitor | Interactive Brokers ComboTrader |
| Scenario stress testing | No competitor | Bloomberg PORT |
| Statistical anomaly alerts | No competitor | Unique — no competitor offers this |
| No-code strategy builder | No competitor | TradingView Pine Script (visual equivalent) |
| Pairs trading | No competitor | QuantConnect, institutional platforms |

## Consequences

- **Positive:** Terminal now covers options analytics, market structure, risk analysis, and advanced charting tools that previously required $100-24,000/mo subscriptions
- **Positive:** Zero new API calls means no performance regression for existing features
- **Positive:** All features use cached data, so they work offline with cached data
- **Negative:** Frontend JS file is now ~15.4K lines — may benefit from module splitting in future
- **Negative:** 36 additional command palette entries — may need categorized sub-menus for discoverability
