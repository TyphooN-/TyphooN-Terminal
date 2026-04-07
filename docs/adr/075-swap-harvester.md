# ADR-075: SwapHarvester — Financing Cost Screening for Carry Strategies

**Status:** Implemented | **Date:** 2026-04-07

## Context

Darwinex CFD symbols have varying swap (financing) rates. Identifying symbols with positive swap enables passive carry income on held positions. Manual scanning of 800+ symbols is impractical.

## Decision

SwapHarvester scans all MT5 symbol specs (`__SPECS__` entries from BarCacheWriter) for symbols where `swap_long > 0` or `swap_short > 0`. Available as:
- **MQL5 script** (`MQL5-ExAmples/SwapHarvester.mq5`) — runs in MetaTrader, exports CSV
- **Terminal command** (`SWAPHARVEST`) — scans cached specs, shows rich UI grid with direction filters, search, color-coded swap values, CSV export

Data source: `__SPECS__` CSV stored in kv_cache, merged across all MT5 accounts via `load_all_specs()`.

## Consequences

- Instant identification of carry-positive symbols without manual inspection
- Direction filters (Long/Short/Both) help target specific strategies
- Depends on BarCacheWriter running on MT5 to keep specs current
- Swap rates change — results are point-in-time snapshots

See also: ADR-057 (Symbol Specs)
