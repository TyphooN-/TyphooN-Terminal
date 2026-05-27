# ADR-024: DARWIN Import Pipeline & Analytics Engine

**Status:** Implemented
**Date:** 2026-03-22

> **See also:** [ADR-026](026-darwin-analytics-expansion.md) for VaR multipliers, drawdown dashboard, floating equity, rebalancer, and symbol overlap.

## Context

TyphooN-Terminal needed the ability to import MT5 trade history from 6 live DARWIN accounts and provide comprehensive risk analytics — going beyond what Darwinex and myfxbook offer.

## Decision

### Architecture

**Import Pipeline:**
1. MT5 "Trade History Report" exported as XLSX (one per DARWIN)
2. `calamine` crate parses XLSX → SQLite tables (`darwin_accounts`, `darwin_deals`, `darwin_positions`)
3. Open positions reconstructed from deal volume balance (sum "in" - sum "out" per symbol+side)
4. Dedicated SQLite connections (not shared cache Mutex) prevent contention with MT5 bar sync

**Analytics Engine (`core/darwin.rs` — 6,800+ lines):**
- 80 public functions covering per-DARWIN and portfolio-level analytics
- 47 unit tests with in-memory SQLite test database
- All functions are pure computation on SQLite data — no external API calls

### Data Sources (current hierarchy, ADR-021 / ADR-023)

```
Priority 1: MT5 (Darwinex)      — authority where BarCacheWriter has data
Priority 2: Alpaca              — broker bars for non-MT5 symbols
Priority 3: tastytrade          — DXLink bars for funded accounts
Priority 4: CryptoCompare       — deep crypto history backfill
Priority 5: Kraken Spot/xStocks — recent public OHLCV + gap-fill
Priority 6: Kraken Futures      — public futures chart candles
```

### Frontend Commands

| Command | Views | Purpose |
|---------|-------|---------|
| DARWIN | 21 | Per-account analytics |
| DARWINS | 33 | Combined portfolio |
| DARWIN-RADAR | 1 | FTP screener (50K+ DARWINs) |
| CRYPTO-BACKFILL | 1 | CryptoCompare deep history + Kraken recent/gap-fill |
| SOURCES | 5 | Data provider manager |

### Analytics Categories

**Risk Metrics:** VaR (95/99), CVaR, Monte Carlo, stress testing, conditional VaR, margin call simulation, VaR forecast, tail risk (skewness, kurtosis, ulcer index, omega ratio)

**Performance:** Sharpe, Sortino, Calmar, Kelly criterion, profit factor, drawdown, recovery factor, gain-to-pain

**Trade Analysis:** Streaks, hourly P/L, day-of-week, hold time, MAE/MFE, slippage, pyramiding, bursts, autocorrelation, sizing efficiency

**Portfolio:** Correlation matrix, sector exposure, trade overlaps, timing divergence, optimal allocation (inverse-vol), what-if simulator, exposure treemap

**DARWIN-Specific:** D-Score components, investor flow (FTP), DARWIN price charting (FTP RETURN), low-correlation finder, regime performance

**Reporting:** Daily risk report, tax lot tracking (FIFO), cost analysis, seasonal patterns, monthly heatmaps

### Security

- XLSX parsing uses `calamine` (pure Rust, no shell execution)
- No user input reaches SQL without parameterized queries (`params![]`)
- Dedicated connections use WAL mode + busy timeout
- FTP path validated as directory before reading
- Symbol validation reuses existing `is_valid_symbol()`

## Consequences

- **Pro**: Complete trade analytics without leaving the terminal
- **Pro**: Independent VaR/risk computation — verify Darwinex's numbers
- **Pro**: 24/7 crypto charting via CryptoCompare deep history plus Kraken recent/gap-fill
- **Pro**: Portfolio-level risk aggregation across 6 accounts
- **Con**: XLSX re-export required for trade history updates (until investor mode)
- **Con**: Kraken Spot public pacing (ADR-095) intentionally slows large
  recent/gap-fill backfills so OHLC requests stay under Kraken's documented
  public limit.
- **Con**: darwin.rs is large (7,000+ lines) — self-contained for now, module split if complexity grows further
