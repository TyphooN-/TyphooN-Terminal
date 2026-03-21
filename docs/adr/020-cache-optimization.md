# ADR-020: Cache Optimization — SQLite + LRU Eviction

**Status:** Implemented
**Date:** 2026-03-16

> **Note:** Builds on [ADR-003](003-bar-data-caching.md). Binary storage optimization in [ADR-027](027-binary-storage-wasm-gpu.md). MT5 SQLite Direct Sync in [ADR-036](036-mt5-sqlite-direct-sync.md).

## Context

The original three-tier cache (memory → IndexedDB → zstd files) had limitations:
- **IndexedDB**: ~50MB-2GB browser limit, unstructured, no query capability
- **Memory**: No eviction — `barCache` grew unbounded with every symbol/timeframe loaded
- **Cold cache**: zstd files are unlimited but unsearchable, no metadata

## Decision

Add SQLite as the primary persistent cache (Tier 3), add LRU eviction to the in-memory cache, and keep IndexedDB + zstd files as fallbacks.

## Four-Tier Cache Architecture

```
Tier 1: In-Memory LRU (instant, max 200 entries)
  ↓ overflow evicts least-recently-accessed
Tier 2: IndexedDB (50MB+, survives restarts)
  ↓ also written on save
Tier 3: SQLite (unlimited, zstd-compressed, WAL mode)
  ↓ also written on save
Tier 4: zstd files (legacy, persistent backup)
```

### Tier 1 — In-Memory LRU Cache
- Max 200 entries (configurable via `CACHE_MAX_ENTRIES`)
- Each entry tracks `lastAccess` timestamp
- `evictLRU()` runs on every write — sorts by lastAccess, removes oldest
- Prevents unbounded memory growth when many symbols/timeframes loaded

### Tier 3 — SQLite Cache (NEW)
- **Engine**: `rusqlite` v0.32 with bundled SQLite
- **Storage**: `~/.config/typhoon-terminal/cache/typhoon_cache.db`
- **Tables**:
  - `bar_cache(key, data BLOB, timestamp, bar_count)` — compressed bar data
  - `kv_cache(key, value BLOB, timestamp)` — fundamentals, news, etc.
- **Compression**: zstd level 3 before storage (~10x savings)
- **Performance**:
  - WAL journal mode (concurrent reads + writes)
  - `synchronous=NORMAL` (2x faster than FULL, safe with WAL)
  - `cache_size=-64000` (64MB page cache)
  - `temp_store=MEMORY` (temp tables in RAM)
- **Eviction**: `db_cache_evict` command removes entries older than N days
- **No size limit**: SQLite handles terabytes; practical limit is disk space

### Tauri Commands
- `db_cache_put(key, data, kind)` — store bars or KV data
- `db_cache_get(key, kind)` — retrieve with decompression
- `db_cache_stats()` — entry counts + total compressed size
- `db_cache_evict(max_age_days)` — remove old entries (default 30 days)

## Consequences

- **Pro**: No IndexedDB size limits — cache entire market history
- **Pro**: LRU prevents OOM on large sessions (200 entries × ~1MB avg = ~200MB max)
- **Pro**: SQLite WAL mode supports concurrent dashboard reads + background writes
- **Pro**: zstd compression gives ~10x storage efficiency
- **Pro**: Per-broker isolation via key prefixing
- **Con**: SQLite bundled binary adds ~1.5MB to app size
- **Con**: Four tiers adds complexity (mitigated by fire-and-forget writes)
