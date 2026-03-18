# ADR-007: Background Bar Pre-Fetch Strategy

**Status:** Implemented
**Date:** 2026-03-15
**Context:** When a user loads a symbol on any timeframe, switching to a different timeframe requires a fresh API fetch. This creates delays and wastes rate limit budget on symbols the user is actively watching.

## Decision

After the primary chart loads for a symbol, silently pre-fetch all other timeframes in the background and cache them. This makes timeframe switching instant for watched symbols.

## Pre-Fetch Behavior

1. User loads `LUMN @ H1` → chart renders immediately
2. Background task starts: fetch `LUMN` bars for all other timeframes (`M15, M30, H4, D1, W1, MN`)
3. Each fetch respects the centralized rate limiter (320ms pacing)
4. Results cached to both memory and localStorage
5. When user clicks `D1` tab → instant chart from cache, no API call

## Rate Budget Allocation

Alpaca free plan: 200 requests/minute (one request every 320ms).

With the centralized `RateLimiter`, all requests (primary chart chunks, MTF indicator data, background pre-fetch, live polling) share the same budget. Pre-fetch runs at lowest priority — after primary chart and MTF indicators complete.

**Example budget for loading one symbol:**
- Primary chart H1 (1000 bars): ~4 chunks × 320ms = 1.3s
- MTF indicator data (6 TFs × 1 chunk each): ~6 × 320ms = 1.9s
- Background pre-fetch (5 remaining TFs × ~2 chunks each): ~10 × 320ms = 3.2s
- **Total**: ~6.4s for full cache population

**With multiple tabs**: the rate limiter queues requests. 3 tabs loading simultaneously = 3× longer for each, but all data arrives eventually.

## Cache Architecture

```
Memory (barCache):     { "LUMN:1Hour": { data: [...], timestamp: Date }, ... }
Disk (localStorage):   { "typhoon_bars_LUMN:1Hour": { data: [...], timestamp: Date }, ... }

TTL: 1 minute (memory) — prevents stale data during active trading
Disk: Persistent — survives app restarts, only re-fetches on first load
```

## Stale Chunk Detection

The API sometimes returns 1-bar chunks with the same timestamp repeatedly when all historical data has been exhausted. Detection:

- If chunk returns ≤5 bars AND the last bar's timestamp matches the previous chunk's last timestamp → stop fetching
- Advance `start` by a full period duration (1 week for W1, 1 day for D1) instead of 1 second

## Synthetic MN1

Alpaca doesn't support monthly bars. Synthesized by:
1. Fetching weekly bars (max available, typically 294 on IEX)
2. Grouping by calendar month (YYYY-MM from timestamp)
3. Aggregating: first open, max high, min low, last close, sum volume
4. Result: ~69 monthly bars from 294 weekly bars (5.7 years)

## Loading UX

Global loading queue shows all symbols currently fetching:
```
LUMN (2021-04-05 → 2026-03-14 · 1000 bars) | SLV (loading...) | SMCI (loading...)
```

Each symbol shows its date range and bar count once primary fetch completes. Pre-fetch runs silently — no UI indication (data appears in cache for instant access).

## Consequences

- **Pro**: Instant timeframe switching for watched symbols
- **Pro**: Maximizes cache value per symbol
- **Pro**: Rate limiter prevents API abuse even with aggressive pre-fetch
- **Con**: Higher initial API usage per symbol (~20 requests vs ~4)
- **Con**: localStorage capacity (~5-10MB) limits how many symbols can be fully cached
- **Con**: Pre-fetch delays availability for other symbols' primary loads

## Future Improvements

- Priority queue: primary chart > MTF indicators > pre-fetch > live polling
- Incremental updates: only fetch bars newer than cached data
- IndexedDB instead of localStorage for larger cache capacity
- WebSocket streaming for real-time bar updates (no polling)
