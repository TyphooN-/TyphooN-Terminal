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

### Dual-Connection Architecture (2026-03-31)

`SqliteCache` opens **two connections** under WAL mode for concurrent access:

| Connection | Mutex | Purpose |
|------------|-------|---------|
| `conn` | Write lock | put_bars, put_kv, delete, compact, create_tables |
| `read_conn` | Read lock | get_bars_raw, get_kv, stats, detailed_stats, all_keys |

WAL mode allows unlimited concurrent readers + one writer. The two Mutexes are independent — write operations on `conn` (Mt5Sync, compaction) never block reads on `read_conn` (UI chart loading, background thread queries). All connections use `PRAGMA busy_timeout=5000` to retry on SQLite-level SQLITE_BUSY for 5 seconds.

**Mt5Sync** opens its own separate `SqliteCache::open()` connection for writing to the main cache. Source MT5 databases are opened via `SqliteCache::open_readonly()` (no journal_mode change, compatible with BarCacheWriter's DELETE mode on the Wine/Linux boundary).

### BarCacheWriter (v1.429, 2026-03-31)

The MQL5 BarCacheWriter EA uses **SQL BLOB manipulation** for incremental sync:

- **Initial sync:** Full export of 100K bars per symbol/TF (one-time, captures complete history)
- **Subsequent syncs:** Three pre-prepared SQL UPDATE statements manipulate BLOBs server-side. Only the delta (48 bytes × new bars + 4-byte count) crosses the MQL5/SQLite boundary — the full blob never enters MQL5 memory.
  - `UpdateLastBar`: replaces last 48 bytes (forming bar close update)
  - `ReplaceLastAndAppend`: SUBSTR splice — updates last bar + appends new bars
  - `AppendOnly`: SUBSTR splice — appends new bars past existing last timestamp
- **CAST AS BLOB:** All bound parameters wrapped in `CAST(?N AS BLOB)` — MQL5's `DatabaseBindArray` may bind `uchar[]` as TEXT type, which would corrupt binary concatenation.
- **Bar count cap:** `MAX_BARS_PER_KEY = 100,000`. When exceeded, falls back to full re-export capped at 100K. Only triggers once per key at the threshold.
- **Batch sleep:** 50ms between BatchSize commits — prevents Wine CPU thrashing.
- **Periodic vacuum:** `PRAGMA incremental_vacuum(100)` every ~30 minutes — reclaims freed pages from DELETE mode fragmentation.
- **Performance:** Only 48×N bytes cross MQL5/SQLite boundary vs 4.8MB full blob round-trip per key per cycle.

### Sync State Table

A `sync_state` table tracks incremental sync progress for LAN sync and data refresh:

```sql
CREATE TABLE IF NOT EXISTS sync_state (
    key TEXT PRIMARY KEY,
    last_sync_ts INTEGER NOT NULL DEFAULT 0
);
```

Keys follow the pattern `kv_cache`, `table:<table_name>` (e.g. `table:sec_filings`). All data tables (`bar_cache`, `kv_cache`, `sec_filings`, `fundamentals`, etc.) carry `updated_at` / `timestamp` columns, enabling incremental sync: the client sends its last known `since_ts` and the server returns only rows newer than that timestamp. Full re-sync is triggered automatically when an incremental response returns 0 rows but the local table is empty.
