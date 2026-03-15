# ADR-003: Bar Data Caching Strategy

**Status:** Accepted
**Date:** 2026-03-15
**Context:** Alpaca's free IEX data feed caps at ~260 bars per API request. Loading 1000+ bars requires multiple sequential API calls. Re-downloading the same data on every chart load wastes time and API quota.

## Decision

Two-tier cache: in-memory (1-minute TTL) + localStorage (persistent across restarts).

## How It Works

1. **Memory cache**: `barCache["SYMBOL:TF"]` with 1-minute TTL for instant switching between timeframes
2. **Disk cache**: `localStorage` with `typhoon_bars_` prefix — persists across app restarts
3. **On chart load**: check memory → check disk → fetch from Alpaca (sequential 260-bar chunks)
4. **After fetch**: save to both memory and disk

## Sequential Chunk Loading

Alpaca's IEX feed returns max ~260 bars per request regardless of the `limit` parameter. To load 1000+ bars:

1. Start from earliest needed date
2. Fetch a chunk (up to 260 bars returned)
3. Advance `start` to just after the last bar's timestamp
4. Repeat until enough bars collected or no more data
5. 250ms delay between chunks to avoid 429 rate limits
6. Sort by timestamp and deduplicate (chunks may overlap)

## Synthetic MN1

Alpaca doesn't support monthly bars. Synthesized by:
1. Fetching 1000+ weekly bars
2. Grouping by calendar month (YYYY-MM from timestamp)
3. Aggregating OHLCV: first open, max high, min low, last close, sum volume

## Consequences

- First load for a new symbol may take several seconds (multiple API calls)
- Subsequent loads are instant from cache
- App restart loads from disk cache, only fetches latest bars
- localStorage has ~5-10MB limit per origin — sufficient for typical usage
- Cache cleared automatically on overflow

## API Call Efficiency Philosophy

Every API call is an investment. The caching strategy maximizes the useful lifespan of each call:

1. **Never re-fetch what we have**: Disk cache persists across restarts — historical bars fetched once are never fetched again
2. **Cache-first display**: Show stale cached data immediately, refresh in background — user sees chart in milliseconds, not seconds
3. **Background pre-fetch**: After primary chart loads, silently cache all other timeframes — future timeframe switches are instant
4. **Deduplication**: Sort + dedup after chunk collection prevents storing duplicate bars from overlapping requests
5. **Stale detection**: Stop fetching when API returns bars we already have (same date as last cached bar)

This philosophy mirrors how we respect CPU and memory cycles — API calls are a finite resource (200/min on free plan) and each one should produce lasting value in the cache.

## Future: Cache Compression

localStorage is limited to ~5-10MB. For heavy multi-symbol usage, consider:
- **zstd compression** via Rust backend: store compressed bar data on disk (Tauri file API), decompress on load. OHLCV data compresses ~5-10x due to repeating price patterns
- **IndexedDB**: larger storage quota than localStorage (50MB+), structured storage
- **Hybrid**: hot cache in memory, warm cache in IndexedDB, cold cache in compressed files
