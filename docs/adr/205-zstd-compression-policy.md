# ADR-205: ZSTD Compression Level Policy and Auto-Compaction

**Status:** Updated / Implemented
**Date:** 2026-04-28 (original decision), 2026-04-29 (auto-compact wired), 2026-05-03 (Storage Manager schedule controls), 2026-05-20 (bar-cache writes moved to zstd-22)
**Related:** ADR-003 (SQLite + zstd cache), ADR-052 (performance architecture), ADR-079 (LAN sync bandwidth), `engine/src/core/cache.rs`, `native/src/app/auto_compact.rs`, `native/src/app.rs::BrokerCmd::CompactStorage`

## Context

TyphooN now targets maximum historical depth across supported brokers. That changes the storage tradeoff: the bar cache is no longer a small rolling cache; it is the long-lived local market-data store. Recompressing later via a scheduled job or manual Storage Manager button is wasted lifecycle complexity when the write path can store final bar blobs at the archival level immediately.

The important distinction is bar data vs hot mutable KV data:

| Class | Level | Examples |
|---|---:|---|
| Bar cache writes | 22 | `put_bars`, `put_bars_fast`, merge/backfill writes, new bar-cache rows |
| KV / metadata hot writes | 3 | AI sessions, broker queues, fundamentals metadata, LAN metadata |
| Backup / export | 22 | compressed SQLite snapshot exports |
| Manual / auto compact | 22 | legacy rows, raw imported rows, or any entries still marked below 22 |

A schema column `bar_cache.zstd_level` tracks the level each entry is currently stored at. New bar rows default to 22. Compact remains useful only for legacy rows, LAN/raw rows with unknown provenance, or existing databases created before this policy.

## Decision

### 1. Store bar-cache blobs at zstd-22 immediately

All Rust bar-cache writes should store packed TTBR bar blobs with zstd level 22.

Rationale:
- Bar writes are not chart-render hot-path work. Broker sync/import/cache merge work happens outside the immediate painter path.
- zstd decompression speed is effectively independent of the original compression level for this use case; chart reads still pay one decode plus direct TTBR unpack.
- Maximum-depth sync means the cache is the product. Smaller persistent storage matters more than shaving encode time on background writes.
- Writing final-form blobs avoids future scheduled recompression churn and avoids relying on the user to click Compact.

### 2. Keep hot mutable KV at zstd-3

Do not blindly use zstd-22 for every blob.

KV/session/metadata paths are small, frequently rewritten, and closer to user-visible interaction loops. They remain level 3 unless a specific path is proven cold and worth promoting.

This is not a retreat from max compression for market data; it is separating durable bar storage from mutable app metadata.

### 3. Compression does not replace O(1) render discipline

zstd-22 optimizes disk size, not frame time. Full-FPS charting still depends on:
- Keeping compression and SQLite writes off the render/painter path.
- Using `get_bars_raw` / TTBR direct unpack for native chart loads rather than JSON serialization when possible.
- Avoiding repeated parsing in cache merge paths. `merge_bars` parses each timestamp once, sorts keyed bars, deduplicates by epoch-ms, then stores final zstd-22 output.
- Maintaining metadata columns (`bar_count`, `last_ts`, `second_last_ts`, `zstd_level`) so scheduler/UI checks do not decompress blobs just to answer cache-state questions.
- Letting SQLite WAL readers proceed independently of the write connection.

### 4. Auto-compaction becomes a compatibility/cleanup path

Auto-compact and manual Compact stay, but their role changes:
- Existing databases may contain zstd-3 rows from the old policy.
- MT5/BarCacheWriter/raw LAN rows may arrive without level-22 metadata.
- Restored backups may contain older rows.

The compactor should keep skipping entries with `zstd_level >= 22`. In steady state, newly-written Rust bar blobs should already be at target and require no scheduled recompression.

## Consequences

**Pros**
- Minimum persistent storage for full-depth market data.
- No dependency on scheduled events or manual compaction for new data.
- Chart decompression path remains unchanged.
- Metadata-driven cache checks stay cheap.
- Legacy compaction still exists for old rows.

**Tradeoffs**
- Background broker/import writes spend more CPU per stored bar blob.
- Very bursty imports can take longer to persist. That is acceptable as long as the work is not performed on the egui render path and scheduler backpressure remains bounded.
- zstd-22 is not a magic O(1) optimization. Any per-frame scans, repeated timestamp parsing, full JSON roundtrips, or synchronous stats refreshes still need to be eliminated separately.

## Implementation pointers

- Bar compression constants: `engine/src/core/cache.rs` (`BAR_ZSTD_LEVEL = 22`, `BACKUP_ZSTD_LEVEL = 22`, `KV_ZSTD_LEVEL = 3`).
- Bar writes: `put_bars`, `put_bars_fast`, and bar batch metadata should mark new rows as level 22.
- Merge O(1)-discipline cleanup: `merge_bars` parses RFC3339 timestamps once into keyed bars before sort/dedup.
- Compact gate/tests: `native/src/app/auto_compact.rs`.
- Storage UI: manual `Compact (zstd-22)` remains a cleanup button for older or externally imported rows.

## Non-goals

- Recompressing hot KV/session writes at level 22.
- Blocking egui rendering while compression runs.
- Pretending the entire codebase can literally become O(1); full-depth sync and chart rendering still require O(n) work where n is visible bars, changed rows, or provider payload size. The goal is no avoidable O(n) work in per-frame/control paths and no repeated O(n) scans where maintained indexes/metadata suffice.
