# ADR-003: SQLite Bar Cache

**Status:** Implemented
**Date:** 2026-03-24

## Context

Fetching historical bars from broker APIs on every startup is slow and rate-limited. The terminal needs a local cache that survives restarts, supports fast range queries by symbol and timeframe, and minimizes memory copies on the hot path.

## Decision

Use SQLite as the local bar cache with zstd-compressed TTBR (TyphooN Terminal Binary Record) format for bulk storage. The `get_bars_raw()` function returns bar data as a zero-copy byte slice that is decoded directly into the chart engine's render structs without intermediate allocation. Session state (open tabs, indicators, drawing tools) is persisted to a JSON file alongside the SQLite database, loaded at startup and saved on graceful shutdown.

## Consequences

- SQLite provides ACID guarantees, WAL mode for concurrent read/write, and zero-config deployment
- zstd compression reduces on-disk size by ~70% for OHLCV data while decompressing at >1 GB/s
- Zero-copy `get_bars_raw()` avoids allocating Vec<Bar> for large date ranges; data goes straight to GPU
- JSON session file is human-readable and easy to debug; schema changes are backward-compatible with serde defaults
- Trade-off: SQLite single-writer lock means bar cache writes are serialized; acceptable since writes are infrequent batch inserts from broker fetches
- Trade-off: JSON session file is not crash-safe; a mid-write crash could lose the last session snapshot

### BarCacheWriter Incremental Sync (v1.426, 2026-03-30)

The MQL5 BarCacheWriter EA now uses **incremental sync** instead of full re-export:

- **Initial sync:** Full export of 100K bars per symbol/TF (one-time, captures complete history)
- **Subsequent syncs:** Dynamic fetch — calculates `(elapsed_seconds / tf_period) + 2` to determine exactly how many bars to fetch (usually 3). Reads existing TTBR blob from DB, merges by timestamp, appends only truly new bars.
- **Merge logic:** Finds merge point by comparing timestamps. Updates last bar's close in-place (for live price updates). Appends only bars with timestamps newer than the last existing bar.
- **Fallback:** If existing blob is missing/corrupt, falls back to full export automatically.
- **Performance:** 10,000x reduction in steady-state data volume (480 bytes vs 4.8MB per symbol/TF per cycle). Memory usage drops from 36.7GB/cycle (851 sym × 9 TF × 4.8MB) to 3.6MB/cycle.
- **Cap:** Maximum 200 bars per incremental fetch (long offline periods catch up over multiple cycles)

### Sync State Table

A `sync_state` table tracks incremental sync progress for LAN sync and data refresh:

```sql
CREATE TABLE IF NOT EXISTS sync_state (
    key TEXT PRIMARY KEY,
    last_sync_ts INTEGER NOT NULL DEFAULT 0
);
```

Keys follow the pattern `kv_cache`, `table:<table_name>` (e.g. `table:sec_filings`). All data tables (`bar_cache`, `kv_cache`, `sec_filings`, `fundamentals`, etc.) carry `updated_at` / `timestamp` columns, enabling incremental sync: the client sends its last known `since_ts` and the server returns only rows newer than that timestamp. Full re-sync is triggered automatically when an incremental response returns 0 rows but the local table is empty.
