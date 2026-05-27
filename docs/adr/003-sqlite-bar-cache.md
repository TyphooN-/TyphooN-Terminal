# ADR-003: SQLite Bar Cache

**Status:** Implemented
**Date:** 2026-03-24

## Context

Fetching historical bars from broker APIs on every startup is slow and rate-limited. The terminal needs a local cache that survives restarts, supports fast range queries by symbol and timeframe, and minimizes memory copies on the hot path.

## Decision

Use SQLite as the local bar cache with zstd-22-compressed TTBR (TyphooN Terminal Binary Record) format for durable bar storage. The `get_bars_raw()` function returns bar data as raw OHLCV tuples decoded directly from TTBR for the chart engine without a JSON roundtrip. Session state (open tabs, indicators, drawing tools) is persisted to a JSON file alongside the SQLite database, loaded at startup and saved on graceful shutdown.

## Consequences

- SQLite provides ACID guarantees, WAL mode for concurrent read/write, and zero-config deployment
- zstd-22 compression minimizes durable OHLCV storage while decompressing at chart-friendly speed
- Direct TTBR raw reads avoid JSON serialization for large date ranges; data goes straight to GPU-ready chart buffers
- JSON session file is human-readable and easy to debug; schema changes are backward-compatible with serde defaults
- Trade-off: SQLite single-writer lock means bar cache writes are serialized; acceptable since writes are infrequent batch inserts from broker fetches
- Trade-off: JSON session file is not crash-safe; a mid-write crash could lose the last session snapshot

### Multi-Connection Architecture (2026-04-02)

`SqliteCache` uses multiple independent connections under WAL mode:

| Connection | Owner | Purpose |
|------------|-------|---------|
| `conn` (Mutex) | Write operations | put_bars, put_kv, delete, compact, create_tables, import |
| `read_conn` (Mutex) | UI thread only | get_bars_raw, try_get_bars_raw, try_connection |
| BG thread conn | Background thread | DARWIN queries, stats, detailed_stats, crypto timestamps |
| Phase 5 conns | Per-account scoped threads | DARWIN per-account analytics |
| Mt5Sync conn | Mt5Sync thread | Separate `SqliteCache::open()` for writing |

WAL mode allows unlimited concurrent readers + one writer. Each connection has its own Mutex (or no Mutex for owned connections). The BG thread reopens its connection every 3-second cycle for WAL freshness — ensures it always sees the latest committed writes from `import_darwin_data`, Mt5Sync, etc.

All connections use `busy_timeout` (5-10s) via `conn.busy_timeout(Duration)` set before any PRAGMAs. Non-critical PRAGMAs (cache_size, temp_store) are best-effort (errors ignored).

**Mt5Sync** opens its own `SqliteCache::open()` for target writes. Source MT5 databases use `SqliteCache::open_readonly()` with 10s busy_timeout (DELETE journal mode — WAL doesn't work across the Wine/Linux boundary).

**Data format detection:** `maybe_decompress()` checks the first 4 bytes — "TTBR" magic = raw binary (from BarCacheWriter), otherwise assumes zstd-compressed (from Rust `put_bars`). Applied to all read paths.

### BarCacheWriter (v1.432, 2026-04-02)

The MQL5 BarCacheWriter EA uses **in-memory merge** for incremental sync:

- **Initial sync:** Full export of 100K bars per symbol/TF (one-time, captures complete history)
- **Subsequent syncs:** Read existing blob from DB (pre-prepared `g_stmtBarRead`), find merge point by timestamp, update last bar in-place, append new bars via `ArrayCopy`, write merged blob back.
- **Skip write when no new bars:** If `appendCount == 0`, skip the blob write entirely. The forming bar's close is cosmetic — captured when the next bar opens. This eliminates ~95% of blob writes in steady state.
- **TF gating:** Per-TF timer tracks last export time. If less than 80% of the TF period has elapsed, skip entirely — no CopyRates, no binary search, no string concat. Only M1 is checked every cycle; M5 every ~4 min, H1 every ~48 min, D1 every ~19 hours.
- **Bar count cap:** `MAX_BARS_PER_KEY = 100,000`. Falls back to full re-export when exceeded.
- **Batch size:** 5 symbols per transaction (shorter exclusive lock).
- **Batch sleep:** 200ms between commits — gives TyphooN-Terminal time to read between exclusive locks.
- **Periodic vacuum:** `PRAGMA incremental_vacuum(100)` every ~30 minutes.

Note: SQL BLOB manipulation via `SUBSTR`/`||` was attempted in v1.427-v1.429 but reverted — MQL5's `DatabaseBindArray` binds `uchar[]` as TEXT type, causing SUBSTR to use character offsets instead of byte offsets, producing truncated/corrupt output.

### Sync State Table

A `sync_state` table tracks incremental sync progress for LAN sync and data refresh:

```sql
CREATE TABLE IF NOT EXISTS sync_state (
    key TEXT PRIMARY KEY,
    last_sync_ts INTEGER NOT NULL DEFAULT 0
);
```

Keys follow the pattern `kv_cache`, `table:<table_name>` (e.g. `table:sec_filings`). All data tables (`bar_cache`, `kv_cache`, `sec_filings`, `fundamentals`, etc.) carry `updated_at` / `timestamp` columns, enabling incremental sync: the client sends its last known `since_ts` and the server returns only rows newer than that timestamp. Full re-sync is triggered automatically when an incremental response returns 0 rows but the local table is empty.
