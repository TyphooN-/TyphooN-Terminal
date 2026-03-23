# ADR-037: Data Source Hierarchy — MT5 Master, Broker Fallback

**Status:** Implemented
**Date:** 2026-03-21

> **Note:** Builds on [ADR-036](036-mt5-sqlite-direct-sync.md) (MT5 SQLite Direct Sync) and [ADR-020](020-cache-optimization.md) (Cache Optimization).

## Context

TyphooN Terminal supports multiple data sources:
- **MT5 via BarCacheWriter** — 9 standard timeframes (M1→MN1), deep history (up to 50K bars per TF), real-time from broker (Darwinex), synced every 30 seconds
- **Alpaca free tier** — 15-minute delayed data, rate-limited, shallow history (~69 monthly bars for some symbols)

Previously, the system treated the connected broker (Alpaca) as the primary data source with MT5 as supplementary. An enrichment layer attempted to merge deeper MT5 history into Alpaca results. This added complexity and still failed when frontend in-memory caching returned stale Alpaca data before the backend was reached.

## Decision

**MT5 is the master data source.** When MT5 data exists for a symbol:timeframe, use it exclusively. Alpaca is a fallback only for symbols not in MT5.

### Data Source Priority

```
1. MT5 (mt5: prefix) — authoritative, real-time, deepest history
2. Connected broker (Alpaca/etc.) — fallback for non-MT5 symbols only
```

### MT5 Coverage via BarCacheWriter

| Timeframe | Max Bars | Approximate History |
|-----------|----------|-------------------|
| M1        | 10,000   | ~7 days           |
| M5        | 20,000   | ~69 days          |
| M15       | 50,000   | ~520 days         |
| M30       | 50,000   | ~3 years          |
| H1        | 50,000   | ~6 years          |
| H4        | 20,000   | ~10 years         |
| D1        | 10,000   | ~40 years         |
| W1        | 2,000    | ~38 years         |
| MN1       | 1,000    | ~83 years         |

### Implementation

**Backend (`get_bars_incremental`):**
- `get_incremental_start` checks `mt5:SYMBOL:TF` key first
- If MT5 data exists → MT5 fast path returns it directly, Alpaca API never called
- If MT5 data doesn't exist → falls through to connected broker

**Frontend (`cachedGetBars`):**
- In-memory cache dedup reduced to 5 seconds (from per-TF staleness up to 7 days)
- Ensures every chart load reaches the backend where MT5-first logic runs
- Background MT5 sync invalidates in-memory cache and reloads chart when new data arrives

**Removed:**
- `enrich_with_deepest_history()` — no longer needed; MT5 wins outright
- `get_deepest_key()` — no longer needed; no cross-prefix merging
- Per-timeframe staleness map in frontend — replaced with 5s rapid dedup

### Crypto-Specific Hierarchy (ADR-040)

Crypto symbols have a 3-tier hierarchy due to MT5's weekend closure (Fri 23:00 → Sun 23:05 UTC):

```
Weekday: MT5 (Darwinex) — authoritative, real-time
Weekend: Alpaca (if connected) > Kraken (backfill data in cache)
Backfill: Kraken — fills ALL weekend gaps from 2013, stored in MT5 cache keys
```

Kraken is **never** a live data source — it only fills historical gaps. On weekends:
- Alpaca provides live crypto prices (if the Alpaca connection is active)
- Kraken data in cache provides continuous chart history (no weekend gaps)
- Weekend bars are tinted blue on charts to visually distinguish from MT5 weekday data

## Consequences

- **Pro**: Simpler architecture — no cross-prefix merging, no enrichment layer
- **Pro**: MT5 data is always authoritative for symbols it has
- **Pro**: Charts load with full history immediately (no waiting for Alpaca API)
- **Pro**: No Alpaca rate limit consumption for MT5 symbols
- **Con**: Symbols not in MT5 watchlists still depend on Alpaca's limited data
- **Con**: MT5 data is only as fresh as the last BarCacheWriter sync (30s interval)
