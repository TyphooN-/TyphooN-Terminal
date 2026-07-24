# ADR-059: SSD Write Reduction Strategy

**Status:** Implemented (policy current; several described components removed) | **Date:** 2026-04-08

> **Scope note (2026-07-24).** The write-reduction *policy* still holds and the
> load-bearing mechanisms are live: `put_kv_dedup` throttles the `broker:*` KV
> keys (`app_runtime_alpaca_account.rs`, `app_runtime_kraken_market.rs`) and the
> cache still opens with `PRAGMA wal_autocheckpoint=2000`
> (`typhoon-engine/src/core/cache.rs`). The components this ADR names as write
> sources are gone: DARWIN analytics and the MT5 `BarCacheWriter` were removed
> with the broker scope reduction (ADR-111), and the `put_kv_if_changed!` macro
> and `deploy_ramdisk.sh` are no longer in the tree. Read the numbers below as
> the 2026-04 measurement, not current topology. Current storage-write policy
> lives in ADR-089 (compression/compaction) and ADR-121 (retention).

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

See also: ADR-003 (SQLite Cache)
