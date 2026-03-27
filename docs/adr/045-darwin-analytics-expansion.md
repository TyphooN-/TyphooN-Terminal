# ADR-045: DARWIN Analytics Expansion

**Status:** Implemented
**Date:** 2026-03-23

> **Note:** Extends [ADR-041](041-darwin-import-analytics.md) (DARWIN Import Pipeline & Analytics Engine).

## Context

The initial DARWIN analytics engine (ADR-041) provided core import and per-account metrics. After live deployment with 6 DARWIN accounts, the analytics needed expansion to cover portfolio-level risk aggregation, live equity tracking, and Darwinex-specific scoring — features that Darwinex's own dashboard and myfxbook lack.

## New Analytics (Implemented in `core/darwin.rs`)

### 1. VaR Multipliers (`compute_var_multipliers`)

Darwinex uses a proprietary VaR model that differs from standard portfolio VaR. The VaR multiplier function computes:

- **45-day VaR**: Short-term risk using recent trade volatility
- **6-month VaR**: Medium-term risk for regime detection
- **Blended VaR**: Weighted combination matching Darwinex's methodology
- Per-DARWIN multipliers for position sizing decisions

This enables independent verification of Darwinex's risk scores and identification of DARWINs whose risk profile has changed.

### 2. Drawdown Dashboard (`get_combined_drawdown_dashboard`)

Combined portfolio drawdown tracking across all 6 accounts:

- Per-DARWIN current drawdown from equity peak
- Per-DARWIN max historical drawdown
- Combined portfolio drawdown (equity-weighted)
- Recovery time estimates based on historical recovery rates
- Top N worst drawdowns with dates and duration

### 3. Floating Equity (`compute_floating_equity`)

Live equity computation from open positions:

- Per-DARWIN: closed balance + unrealized P/L = floating equity
- Combined: aggregate across all accounts
- Equity snapshots stored in `darwin_equity_snapshots` table with timestamp, closed balance, unrealized P/L, floating equity, and open position count
- Historical equity curve from snapshot time series

### 4. Rebalancer (`compute_rebalance_suggestions`)

Portfolio rebalancing recommendations:

- Target allocation based on inverse-volatility weighting
- Current allocation from live equity
- Delta (over/underweight) per DARWIN
- Suggested capital movements to reach target
- Respects Darwinex minimum investment constraints

### 5. Symbol Overlap (`get_symbol_overlap`)

Cross-account position conflict detection:

- Identifies symbols held across multiple DARWINs
- Flags opposing directions (DARWIN A long EUR/USD, DARWIN B short EUR/USD)
- Computes net exposure per symbol across the portfolio
- Highlights concentration risk (multiple DARWINs in same instrument)

### 6. Equity Snapshots

Persistent equity tracking via `darwin_equity_snapshots` table:

```sql
CREATE TABLE darwin_equity_snapshots (
    timestamp TEXT NOT NULL,
    darwin_ticker TEXT NOT NULL,
    closed_balance REAL NOT NULL,
    unrealized_pnl REAL NOT NULL,
    floating_equity REAL NOT NULL DEFAULT 0,
    open_position_count INTEGER NOT NULL DEFAULT 0
);
```

Snapshots recorded on each analytics refresh, enabling:
- Historical equity curves per DARWIN
- Combined portfolio equity over time
- Drawdown duration analysis
- Performance attribution across time periods

## Frontend Commands

All analytics accessible via the existing DARWIN/DARWINS command palette entries:

| View | Command | Description |
|------|---------|-------------|
| VaR Multipliers | DARWINS → Risk | 45d/6m/blended VaR per DARWIN |
| Drawdown Dashboard | DARWINS → Drawdown | Combined + per-DARWIN drawdown tracking |
| Floating Equity | DARWINS → Equity | Live equity with unrealized P/L |
| Rebalancer | DARWINS → Rebalance | Inverse-vol target vs actual allocation |
| Symbol Overlap | DARWINS → Overlap | Cross-account position conflicts |
| Equity History | DARWINS → History | Equity curve from snapshots |

### 7. Advanced Performance Metrics

Additional per-DARWIN and portfolio-level metrics implemented in `core/darwin.rs`:

- **CAGR** (`compute_cagr`): Compound Annual Growth Rate from daily returns
- **Recovery Factor** (`compute_recovery_factor`): Net profit / max drawdown ratio
- **Drawdown Duration** (`compute_drawdown_duration`): Max drawdown duration in days, current drawdown duration, average recovery time
- **Divergence Index** (`compute_divergence_index`): Measures return divergence between signal (MT5 trades) and DARWIN quote price over time — identifies when investor-visible performance diverges from underlying strategy
- **Investment Velocity** (`compute_investment_velocity`): Rate of AuM change from investor flow data

### 8. Signal vs Quote Comparison

All DARWIN views now include signal vs quote comparison:

- Signal Sharpe vs Quote Sharpe per DARWIN
- Signal max drawdown vs Quote max drawdown
- Per-DARWIN equity curves rendered side-by-side (Signal vs Quote)
- Divergence Index plot showing cumulative return gap over time
- Per-DARWIN drawdown stats table with both Signal and Quote columns

### 9. Monthly Returns Heatmap

Darwinex-style monthly returns grid per DARWIN:

- Year × Month colored grid (green = positive, red = negative)
- Built from `get_monthly_returns()` data
- Displayed in the combined Drawdown dashboard view

## Consequences

- **Pro**: Portfolio-level risk visibility across 6 accounts in one view
- **Pro**: Independent VaR verification — catch Darwinex scoring discrepancies
- **Pro**: Symbol overlap detection prevents unintended hedging/concentration
- **Pro**: Rebalancer provides actionable allocation recommendations
- **Pro**: Equity snapshots create persistent performance history
- **Pro**: All computation is local (SQLite queries) — no API calls
- **Con**: Equity snapshots grow over time (mitigated by periodic pruning)
- **Con**: Rebalancer assumes inverse-vol is optimal (user should verify)
