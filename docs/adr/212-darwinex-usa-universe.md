# ADR-212: Darwinex Zero USA Equity Universe

## Status
Accepted

## Context
We want to support viewing and analyzing Darwinex Zero tradable symbols inside TyphooN Terminal without relying on MT5 data sync.

Darwinex Zero offers ~800 USA Stocks and ETFs. We obtained the complete list via a Market Watch export from the platform.

## Decision
- Created `native/src/app/darwin_universe.rs`
- Hardcoded the **full** list of 793 USA Stocks + ETFs (after filtering EURGBP/EURUSD/GBPUSD)
- Source: `Market Watch 20260515 235457.csv` export from https://www.darwinexzero.com/assets
- Exposed via `DARWINEX_USA_EQUITY_SYMBOLS`, `darwinex_usa_equity_symbols()`, and `darwinex_usa_equity_set()`

## Consequences
- Users can now search/chart Darwinex symbols using existing Kraken/Alpaca data
- List should be refreshed periodically when Darwinex adds/removes symbols
- No MT5 sync dependency

## References
- https://www.darwinexzero.com/assets
- Related: `cached_darwin_symbols` in `TyphooNApp`