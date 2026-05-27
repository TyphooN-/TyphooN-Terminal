# ADR-059: SSD Write Reduction Strategy

**Status:** Implemented | **Date:** 2026-04-08

## Context

SQLite KV cache writes were burning SSD at hundreds of writes/second during market hours. Primary sources: per-tick `quote:*` KV writes (851 symbols), `broker:account` on every price tick, BG thread DARWIN analytics rewritten every 3 seconds, and BarCacheWriter bid/ask table updates.

## Decision

### Eliminated Writes
- `quote:*` KV: removed entirely — live bid/ask stored in-memory only (`chart.live_bid/ask`)
- BG thread DARWIN KV: hash-based dedup macro (`put_kv_if_changed!`) — only writes when JSON content actually changes

### Throttled Writes
- `broker:account/positions/orders/watchlist`: `put_kv_dedup()` with 30s time throttle — max 2 writes/min per key even when content changes every tick
- BarCacheWriter bid/ask table: every 60s (aligns with M1 bar writes)

### SQLite Tuning
- `PRAGMA wal_autocheckpoint=2000` (both terminal and BarCacheWriter) — halves WAL→DB sync frequency
- Ramdisk deployment (`deploy_ramdisk.sh`) auto-detects tmpfs: `/dev/shm` → `/run/user/$UID` → `/tmp`

## Consequences

- SSD writes reduced ~99% during market hours
- Ramdisk portable across Linux distros (not just those with `/dev/shm`)
- Trade-off: live bid/ask no longer persisted across restarts (acceptable — refreshes immediately from streaming)
- WAL file grows slightly larger between checkpoints (~8MB vs ~4MB)

See also: ADR-003 (SQLite Cache), ADR-058 (LAN Sync Bandwidth)
