# ADR-058: LAN Sync Bandwidth Optimization

**Status:** Implemented | **Date:** 2026-04-08

## Context

LAN sync was consuming 5-9 GB/day due to: full bar metadata (460KB) sent every 15s, 851 individual quote KV entries per sync, DARWIN analytics KV rewritten every 3s, KV values decompressed on server then sent as plaintext (double compress/decompress), and forced WebSocket reconnect every 15 minutes triggering full initial sync.

## Decision

### Delta Metadata
New `RequestMetaSince { since_ts }` protocol message — server queries `WHERE timestamp > ?1` instead of returning all 7,700 bar entries. Client tracks `bar_cache` sync timestamp.

### KV Skip List
Server skips syncing: `cred:*`, `quote:*` (851 bid/ask entries), `darwin:daily_returns`, `darwin:correlations`, `darwin:exposure`, `darwin:insider_trades`, `client:demand`, and all `lan:*` config. These are secret, machine-local, high-churn, or computed locally on each machine.

### Compressed KV Transport
Server sends the already-compressed KV blobs directly instead of decompressing first. Client uses `put_kv_compressed()` to store pre-compressed blobs at the writer's stored zstd level. Eliminates double compress/decompress cycle.

### Write Throttling
`put_kv_dedup()` hashes JSON content AND throttles to max once per 30s per key. `broker:account` (equity changes every tick) now writes at most 2/min instead of 60/min.

### Timing
- Re-sync interval: 60s (was 15s)
- Table sync: every 5th cycle (~5 min)
- Forced reconnect: every 2 hours (was 15 min)
- WAL autocheckpoint: 2000 pages (was 1000)

## Consequences

| Metric | Before | After |
|--------|--------|-------|
| Idle traffic/day | 5-9 GB | ~1-2 MB |
| Active traffic/day | 2-3 GB | ~10-20 MB |
| SSD writes/min | ~100+ | ~2-4 |
| CPU (compress/decompress) | Double cycle | Single pass |

See also: ADR-045 (LAN Sync TLS), ADR-046 (Remote Request Forwarding)
