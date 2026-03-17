# ADR-025: Feature Batch 2 — NNFX Strategy, Options Tools, Sector Rotation, Auto-Trading

**Status:** Implemented
**Date:** 2026-03-17

## Context

After achieving MT5 + Godel feature parity (ADR-021), the terminal needed deeper analytical tools, a native NNFX backtesting strategy, and market-wide visualization capabilities. These features close the gap with professional platforms (TradingView, Bloomberg) while maintaining the local-first architecture.

## Features Added

### 1. NNFX Strategy (KAMA + Fisher Transform)

**Backend:** `src-tauri/src/core/backtest.rs` — `NNFXStrategy`

Port of the NNFX system's core entry logic:
- **Entry long:** Price crosses above KAMA + Fisher > 0 (bullish confirmation)
- **Entry short:** Price crosses below KAMA + Fisher < 0 (bearish confirmation)
- Uses the same KAMA (10/2/30) and Fisher Transform (32) as the chart indicators
- Optimized for D1/W1/MN1 timeframes (higher timeframe swing trading)
- Available in backtester, optimizer, walk-forward, and visual replay

**Parameters:**
- `fast_period` (default 10) — KAMA period
- `slow_period` (default 32) — Fisher Transform period

### 2. Options P&L Calculator (OPTCALC)

**Command:** Ctrl+K → OPTCALC

Canvas-based payoff diagram for multi-leg options strategies:
- Add/remove legs (buy/sell × call/put × strike × premium × qty)
- Auto-populates strike from current price
- Green/red profit/loss zone fills
- Strike price markers as dashed yellow lines
- Max profit/loss labels
- Click to calculate — instant rendering

### 3. Sector Rotation Heatmap (SECTORS)

**Command:** Ctrl+K → SECTORS

Finviz-style colored grid of 16 sector/index ETFs:
- XLK, XLF, XLE, XLV, XLI, XLP, XLY, XLU, XLB, XLRE, XLC (sectors)
- SPY, QQQ, IWM, DIA, GLD (indices/commodities)
- Color intensity proportional to daily % change
- Weekly % change shown below daily
- Click any box to load that ETF's chart

### 4. Economic Calendar with Countdown (ECON)

**Command:** Ctrl+K → ECON

Key economic events with live countdown timers:
- Market Open/Close (next trading day)
- FOMC Meeting (next Wednesday after 15th)
- CPI Release (13th of month)
- Non-Farm Payrolls (first Friday of month)
- GDP Release (28th of month)
- Color-coded impact levels (HIGH/MEDIUM/LOW)

### 5. Options Strategy Builder (OPTSTRAT)

**Command:** Ctrl+K → OPTSTRAT

Full options chain viewer with strategy presets:
- Loads live options chain from Alpaca API
- Call/Put columns with bid/ask/delta
- In-the-money strikes highlighted
- Strategy presets: Long Call, Bull Call Spread, Straddle, Iron Condor
- Aggregate Greeks summary (Delta, Gamma, Theta, Vega)

### 6. Strategy Auto-Trading Framework (AUTOTRADE)

**Command:** Ctrl+K → AUTOTRADE

Framework for automated strategy execution:
- Select from installed JS indicator plugins
- Configurable max position size and cooldown
- Paper-trading-only safety toggle (default on)
- Enable/disable toggle with visual state feedback
- Plugin interface: `onSignal()` returns `{ action, qty }`

### 7. Watchlist SMA200 Cross Alerts

Automatic batch monitoring of watchlist symbols:
- Checks all watchlist symbols for SMA200 crossovers
- 5-minute cache to avoid rate limit abuse
- Browser notification on cross above/below
- Runs in dashboard cycle (every 2s, throttled by cache)

## Architecture Notes

- All features use the existing `createWindow()` floating window system
- Options data comes from Alpaca's options API (already integrated)
- Sector data uses existing `get_bars` with sector ETF symbols
- NNFX strategy reuses the same KAMA/Fisher math as the chart indicators
- Auto-trading framework is plugin-based — no hardcoded strategies
