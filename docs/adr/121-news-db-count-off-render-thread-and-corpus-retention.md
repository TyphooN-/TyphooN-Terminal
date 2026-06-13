# ADR-121: News DB Count Off the Render Thread; News Corpus Retention Bounds

**Status:** Accepted | **Date:** 2026-06-13

Companion to **ADR-033** (Background Data Channels — Zero DB Queries on UI Thread)
and **ADR-100** (News Article Rendering). Extends the ADR-033 invariant to the
News window and adds storage bounds to the `research_news` corpus.

## Context

With the News window open, the UI froze for **10–17 seconds roughly once per
Kraken OHLC sweep cycle (~90 s)**. The frame-stall instrumentation pinned it
entirely on the News window:

```
floating_windows_ms=14746  ≈  "news took 14514ms"   (heavy_sync=false, news_loading=false)
```

Root cause: the header's "· N in DB" refresh ran, on the egui update path, every
5 s:

```rust
if let Ok(conn) = cache.connection() {                       // WRITE mutex
    if let Ok(n) = news::count_all_articles(&conn) { ... }    // SELECT COUNT(*) + create_news_tables (DDL)
}
```

Two faults compounded:

1. **`cache.connection()` is the write mutex** (`SqliteCache::conn`), the same
   lock the bulk bar-sync writers hold for the duration of a batch insert
   transaction. The render thread blocked until the in-flight OHLC-sweep batch
   committed. The recurrence period matched the sweep cycle exactly — the count
   poll and a large batch write collided once per cycle. (The COUNT itself is
   sub-millisecond; the 14 s was pure mutex wait.) This is the precise failure
   mode ADR-033 exists to prevent — a render-thread DB hit on the contended
   write path — that the News header had slipped past.
2. **Unbounded corpus.** A full-universe News "Fetch (All)" (12,598 symbols)
   grows `research_news` and its FTS5 mirror without limit, making even a correct
   `COUNT(*)`/FTS search progressively heavier and inflating the on-disk
   footprint.

## Decision

### 1. Push the count from the broker; zero render-thread DB work

A new `BrokerMsg::NewsDbTotal(i64)` is emitted broker-side — in the
`spawn_blocking`/connection scopes that already load or scrape news (cached load,
fresh multi-source fetch, scope scrape) — and handled in
`handle_news_ingest_msg` to set `news_db_total`. The render thread no longer
touches SQLite for the header at all. The auto-load `LoadCachedNews` on first
window open carries the count back, so the header still populates promptly.

The old render-thread poll, its `heavy_sync_in_progress` gate (now moot), and the
`news_db_total_last_refresh` anchor field are removed.

### 2. Bound the corpus in background maintenance

`SqliteCache::enforce_news_retention(cutoff_ts, max_rows)` runs on the existing
6-hour BG maintenance cadence (alongside `incremental_vacuum`/`evict_lru`),
applying two limits via `news::purge_older_than` + the new `news::enforce_max_rows`
(both keep the FTS mirror in sync):

- **Age:** drop articles older than **45 days**.
- **Hard cap:** keep at most the newest **250,000** rows.

Age alone can't bound a burst of full-universe scraping that lands inside the
window, so the row cap is the ceiling; together they keep `COUNT(*)`/FTS and disk
bounded regardless of how often "Fetch (All)" runs.

## Consequences

- News-window frame stalls disappear; the render thread holds the ADR-033
  zero-DB invariant. Worst case the header count is a few seconds stale.
- `research_news` stays bounded; retention runs off the render thread on the
  write connection.
- Constants (`NEWS_RETENTION_DAYS`, `NEWS_MAX_ROWS`) live at the BG call site for
  easy tuning. Engine helpers `count_all_articles_readonly` (DDL-free read-path
  count) and `enforce_max_rows` are unit-tested.
