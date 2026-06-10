# ADR-097: Darwinex Zero USA Equity Universe

## Status
Superseded (removed with Darwinex in ADR-111, 2026-06-10)

## Context
We want to support viewing and analyzing Darwinex Zero tradable symbols inside TyphooN Terminal without relying on MT5 data sync.

Darwinex Zero offers ~800 USA Stocks and ETFs. We obtained the complete list via a Market Watch export from the platform.

## Decision
- Created `native/src/app/darwin_universe.rs`
- Hardcoded the **full** deduplicated list of 822 USA Stocks + ETFs (after filtering EURGBP/EURUSD/GBPUSD)
- Source: `Market Watch 20260515 235457.csv` export from https://www.darwinexzero.com/assets
- Exposed via `DARWINEX_USA_EQUITY_SYMBOLS`, `darwinex_usa_equity_symbols()`, and `darwinex_usa_equity_set()`

## Consequences
- Users can now search/chart Darwinex symbols using existing Kraken/Alpaca data
- List refresh is a manual maintenance task tied to the Market Watch export; the checked-in constant is deduplicated and covered by the `darwinex_usa_equity_set()` lookup path.
- No MT5 sync dependency

## References
- https://www.darwinexzero.com/assets
- Related: `cached_darwin_symbols` in `TyphooNApp`