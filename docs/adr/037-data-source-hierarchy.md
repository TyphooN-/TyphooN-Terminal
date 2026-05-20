# ADR-037: Data Source Hierarchy — MT5 Master, Broker Fallback

**Status:** Implemented
**Date:** 2026-03-21

> **Note:** Builds on the MT5 BarCacheWriter and SQLite cache work that predated the retained ADR set. Later updates are tracked by [ADR-040](040-crypto-data-source.md), [ADR-071](071-bardata-full-history.md), and [ADR-203](203-alpaca-sync-autotuning.md).

## Context

TyphooN Terminal supports multiple data sources:
- **MT5 via BarCacheWriter** — 9 standard timeframes (M1→MN1), provider-maximum history requested from the broker/server, real-time from broker (Darwinex), synced every 30 seconds
- **Alpaca** — US equities broker feed; paginated history is traversed until the server reports exhaustion, subject to account/rate-limit constraints

Previously, the system treated the connected broker (Alpaca) as the primary data source with MT5 as supplementary. An enrichment layer attempted to merge deeper MT5 history into Alpaca results. This added complexity and still failed when frontend in-memory caching returned stale Alpaca data before the backend was reached.

## Decision

**MT5 is the master data source.** When MT5 data exists for a symbol:timeframe, use it first. Non-MT5 symbols fall through the configured source queue.

### Current Data Source Priority

```
1. MT5 (mt5: prefix) — authoritative Darwinex/BarCacheWriter data
2. Alpaca (alpaca: prefix) — US equities/crypto broker feed
3. tastytrade (tastytrade: prefix) — DXLink bars and options market context
4. CryptoCompare (cryptocompare: prefix) — deep crypto history
5. Kraken Spot/xStocks (kraken: prefix) — public recent/gap-fill bars + authenticated Spot trading
6. Kraken Futures (kraken-futures: prefix) — public futures chart candles
```

`engine/src/core/data_source.rs::DataSourceManager` formalizes this order, with per-symbol overrides and health-based fallback. Fast chart reloads in `native/src/app.rs::ChartState::try_load` use the same six-source order.

### MT5 Coverage via BarCacheWriter

The old terminal-side 10K/20K/50K per-timeframe table is obsolete. Current MT5 sync emits the largest safe MQL5 `MAX_BARS` sentinel (`i32::MAX`) for every enabled timeframe and lets the broker/server decide how much history exists. If a symbol/timeframe stops growing below that sentinel, saturation memory treats that count as the provider's natural ceiling and suppresses repeat full-depth churn while normal stale/current updates continue.

| Timeframe | Requested Depth |
|-----------|-----------------|
| M1        | provider maximum |
| M5        | provider maximum |
| M15       | provider maximum |
| M30       | provider maximum |
| H1        | provider maximum |
| H4        | provider maximum |
| D1        | provider maximum |
| W1        | provider maximum |
| MN1       | provider maximum |

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

Crypto symbols still prefer MT5 when Darwinex data exists, but non-primary crypto bars are no longer a weekend-only special case. CryptoCompare provides deep history, Kraken Spot/xStocks provides recent public OHLCV under `kraken:SYMBOL:TF`, and Kraken Futures syncs independently under `kraken-futures:SYMBOL:TF`. Non-primary merged bars are tracked as gap-fill timestamps and rendered magenta on charts.

## Consequences

- **Pro**: Simpler architecture — no cross-prefix merging, no enrichment layer
- **Pro**: MT5 data is always authoritative for symbols it has
- **Pro**: Charts load with full history immediately (no waiting for Alpaca API)
- **Pro**: No Alpaca rate limit consumption for MT5 symbols
- **Con**: Symbols not in MT5 watchlists still depend on Alpaca's limited data
- **Con**: MT5 data is only as fresh as the last BarCacheWriter sync (30s interval)
