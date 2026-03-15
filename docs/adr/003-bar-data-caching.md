# ADR-003: Three-Tier Bar Data Cache

**Status:** Implemented
**Date:** 2026-03-15
**Context:** Alpaca's free IEX data feed caps at ~260 bars per API request. Loading 1000+ bars requires multiple sequential API calls. Re-downloading the same data on every chart load wastes time and API quota.

## Decision

Three-tier cache architecture: hot (memory) + warm (IndexedDB) + cold (zstd-compressed files).

## Architecture

```
                    ┌─────────────────┐
  Chart renders ←── │   HOT (memory)  │ ← instant, 1-min TTL
                    └────────┬────────┘
                             │ miss
                    ┌────────▼────────┐
                    │  WARM (IndexedDB)│ ← 50MB+, structured, persistent
                    └────────┬────────┘
                             │ miss
                    ┌────────▼────────┐
                    │  COLD (zstd)    │ ← unlimited, disk, ~5-10x compression
                    └────────┬────────┘
                             │ miss
                    ┌────────▼────────┐
                    │  Alpaca API     │ ← 260 bars/request, 200 req/min
                    └─────────────────┘
```

## Tier Details

### Hot Cache (Memory)
- JavaScript `Map`: `barCache["SYMBOL:TF"]` → `{ data, timestamp }`
- 1-minute TTL for freshness checks
- Instant access (~0ms)
- Lost on page reload (populated from warm/cold on startup)

### Warm Cache (IndexedDB)
- Database: `typhoon_bars`, object store: `bars`, key: `"SYMBOL:TF"`
- 50MB+ quota (browser-managed, far exceeds localStorage's 5-10MB)
- Structured storage with key-based lookup
- Survives app restarts, page reloads
- Populated from cold cache on startup, updated on every save

### Cold Cache (zstd-compressed files)
- Location: `~/.config/typhoon-terminal/cache/SYMBOL_TF.zst`
- Compression: zstd level 3 via Rust `zstd` crate
- ~5-10x compression ratio on OHLCV bar data
- Unlimited capacity (disk-bound)
- Survives app reinstalls (user config directory)
- Managed via Tauri commands: `save_cold_cache`, `load_cold_cache`, `list_cold_cache`

## Data Flow

### Save (after API fetch)
```
API response → Hot (instant) → Warm (IndexedDB, async) → Cold (zstd file, async)
```
All three tiers updated in parallel, non-blocking. Chart renders from hot immediately.

### Load (on chart request)
```
1. Check hot cache (memory) — if fresh (< 1 min), use it
2. If stale but present: display immediately, refresh in background
3. If miss: check warm (IndexedDB) — load to hot, display
4. If miss: check cold (zstd) — decompress, promote to warm+hot, display
5. If miss: fetch from Alpaca API — save to all three tiers
```

### Startup
```
1. Open IndexedDB → load all entries to hot cache
2. List cold cache files → promote missing entries to warm+hot
3. Migrate old localStorage entries to IndexedDB (one-time)
```

## Cache-First Display Strategy

When cached data exists (any tier), display it IMMEDIATELY — even if stale. Then refresh from API in background:

1. User opens chart → cached data appears in ~5ms
2. Background fetch starts → API returns fresh bars in ~2-5s
3. Chart silently updates with new data
4. User never sees a loading spinner for previously-viewed symbols

## Sequential Chunk Loading

Alpaca's IEX feed returns max ~260 bars per request. To load 1000+ bars:

1. Start from earliest needed date
2. Fetch a chunk (server returns up to ~260 bars)
3. Advance `start` by one full period past the last bar's timestamp
4. Repeat until: enough bars, or stale chunk detected (same date as existing data)
5. Rate limiter paces requests at 320ms intervals
6. Sort by timestamp and deduplicate after collection

### Stale Chunk Detection
The API returns bars from the last trading day repeatedly when all history is exhausted. Detection: compare the last bar's DATE (YYYY-MM-DD) of the new chunk with existing data. If same → stop.

## Rate Limiting

Centralized `RateLimiter` struct (Rust, `Arc<Mutex>`):
- Paces all data API requests at 320ms intervals (200 req/min with headroom)
- On 429 response: triggers 60-second cooldown, returns bars collected so far
- Shared across: chart loading, MTF indicators, background pre-fetch, live polling
- Multiple tabs share the same budget — no double-spending

## Synthetic MN1

Alpaca doesn't support monthly bars. Synthesized by:
1. Fetching 1000+ weekly bars (max available, typically ~294)
2. Grouping by calendar month (YYYY-MM from timestamp)
3. Aggregating OHLCV: first open, max high, min low, last close, sum volume
4. Result: ~69 monthly bars from 294 weekly bars (5.7 years)

## Background Pre-Fetch

After primary chart loads, silently fetch all other timeframes (M1/M5/M15/M30/H1/H4/D1/W1) for the same symbol. This populates all three cache tiers, making timeframe switching instant.

Pre-fetch uses 1-hour TTL (60× normal) to avoid redundant re-fetching.

## API Call Efficiency Philosophy

Every API call is an investment. The caching strategy maximizes the useful lifespan of each call:

1. **Never re-fetch what we have**: Cold cache persists indefinitely — historical bars fetched once are never fetched again
2. **Cache-first display**: Show stale cached data immediately, refresh in background — user sees chart in milliseconds
3. **Background pre-fetch**: Silently cache all timeframes — future switches are instant
4. **Deduplication**: Sort + dedup after chunk collection prevents storing duplicates
5. **Stale detection**: Stop fetching when API returns bars we already have
6. **Rate budgeting**: Centralized limiter prevents wasted calls on 429 errors
7. **Compression**: zstd cold cache stores ~10x more data per byte of disk

## Migration

Old localStorage cache (`typhoon_bars_*` keys) is automatically migrated to IndexedDB on first run. localStorage entries are deleted after migration.

## Consequences

- **Pro**: Charts appear instantly on restart from warm/cold cache
- **Pro**: API calls only fetch NEW bars, not rebuild history
- **Pro**: 50MB+ warm cache vs 5-10MB localStorage
- **Pro**: Unlimited cold storage with ~5-10x compression
- **Pro**: Three redundant tiers — data survives any single tier failure
- **Con**: IndexedDB API is async (slightly more complex than localStorage)
- **Con**: Cold cache requires Tauri backend (not available in pure web mode)
- **Con**: Three tiers to debug when cache issues arise
