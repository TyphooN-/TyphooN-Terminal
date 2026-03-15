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
