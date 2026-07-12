# ADR-089: ZSTD Compression Level Policy and Auto-Compaction

**Status:** Updated / Implemented
**Date:** 2026-04-28 (original decision), updated through 2026-07-12 (one configured live-write policy)
**Related:** ADR-003 (SQLite + zstd cache), ADR-032 (performance architecture), ADR-099 (Kraken WS full-universe responsiveness), `typhoon-engine/src/core/cache.rs`, `typhoon-native/src/app/auto_compact.rs`

## Context

TyphooN now targets maximum historical depth across supported brokers. That changes the storage tradeoff: the bar cache is no longer a small rolling cache; it is the long-lived local market-data store. Recompressing later via a scheduled job or manual Storage Manager button is wasted lifecycle complexity when the write path can store final bar blobs at the archival level immediately.

The important distinction is bar data vs hot mutable KV data:

| Class | Level | Examples |
|---|---:|---|
| Bar cache writes (REST / imports / normal merges) | User-selected base level, default 3 (1-22) | `put_bars`, `merge_bars`, broker backfill, new bar-cache rows; persisted in sync preferences and exposed in Storage Manager |
| Bar cache fast/WS writes | User-selected base level | `merge_bars_fast` honors `bar_zstd_level()` like normal writes |
| KV / metadata writes | User-selected base level | Current `put_kv` uses the same configured compression policy |
| Backup / export | 22 | compressed SQLite snapshot exports |
| Manual / auto compact | 22 | legacy rows, raw imported rows, any entries still marked below 22 — promotes WS-written rows back to 22 once the streamer settles |

A schema column `bar_cache.zstd_level` tracks the level each entry is currently stored at. New bar rows default to zstd-3 for write throughput, but the operator can raise/lower the base level in Storage Manager without rebuilding. Compact remains useful for rows below the archival target, LAN/raw rows with unknown provenance, WS hot writes, or existing databases created before this policy.

## Decision

### 1. Make the normal bar-cache write level configurable

Rust bar-cache writes store packed TTBR bar blobs at a runtime-configurable base zstd level. The default is zstd-3 because full-universe sync/import bursts made zstd-22 too expensive for sustained foreground operation; Storage Manager exposes the knob (1-22) with Fast/Balanced/Max presets and persists it in `app:sync_preferences`.

Rationale:
- Bar writes are not painter-path work, but they can still saturate CPU during broad broker sync, snapshot catch-up, and large imports.
- zstd decompression speed is effectively independent of the original compression level for this use case; chart reads still pay one decode plus direct TTBR unpack.
- Operators should be able to choose the CPU-vs-disk tradeoff without recompiling: fast level during catch-up, higher level when disk pressure matters.
- The archival target is still zstd-22; manual/auto compaction promotes rows below that target when the machine is idle.

### 2. Apply the configured level consistently

Normal bar writes, `merge_bars_fast`, and current KV writes all honor `bar_zstd_level()`. The Storage Manager setting is policy, not a hint. A fast default remains appropriate during broad catch-up; operators who choose a higher level accept the corresponding encode cost. Idle/manual compaction promotes older rows below the archival target.

### 3. Compression does not replace O(1) render discipline

zstd-22 optimizes disk size, not frame time. Full-FPS charting still depends on:
- Keeping compression and SQLite writes off the render/painter path.
- Using `get_bars_raw` / TTBR direct unpack for native chart loads rather than JSON serialization when possible.
- Avoiding repeated parsing in cache merge paths. `merge_bars` parses each timestamp once, sorts keyed bars, deduplicates by epoch-ms, then stores output at the configured base level.
- Maintaining metadata columns (`bar_count`, `last_ts`, `second_last_ts`, `zstd_level`) so scheduler/UI checks do not decompress blobs just to answer cache-state questions.
- Letting SQLite WAL readers proceed independently of the write connection.

### 4. Auto-compaction becomes a compatibility/cleanup path

Auto-compact and manual Compact stay, but their role changes:
- Existing databases may contain zstd-3 rows from the old policy.
- MT5/BarCacheWriter/raw LAN rows may arrive without level-22 metadata.
- Restored backups may contain older rows.
- Rows written under an earlier/lower configured level remain eligible for archival compaction.

The compactor keeps the `zstd_level < target` filter at `typhoon-engine/src/core/cache.rs::compact_storage`, so anything below 22 — configured-base writes, legacy rows, WS hot writes, raw LAN rows — is naturally promoted on the next scheduled or manual run. The auto-compact gate + `Compact (zstd-22)` button in Storage Manager close the loop without making every foreground write pay max-compression CPU.

### 5. WS fast path preserves the configured policy

`merge_bars_fast` is fast because it uses nonblocking cache-lock behavior and the bounded/coalesced WS pipeline, not because it silently overrides the operator's compression setting. Regression tests assert that normal and fast writes record the selected level.

See ADR-099 for the broader UI-responsiveness work this carve-out enables.

## Consequences

**Pros**
- Operator-visible CPU/disk tradeoff for full-depth market data.
- Default write path is safe for broad sync and imports; compaction can still recover max storage density later.
- Chart decompression path remains unchanged.
- Metadata-driven cache checks stay cheap.
- Legacy compaction still exists for old rows.

**Tradeoffs**
- Lower base levels use more disk until compaction runs.
- Higher base levels can spend much more CPU per stored bar blob. That is acceptable only when the operator deliberately chooses it and the work is not performed on the egui render path.
- zstd-22 is not a magic O(1) optimization. Any per-frame scans, repeated timestamp parsing, full JSON roundtrips, or synchronous stats refreshes still need to be eliminated separately.

## Implementation pointers

- Compression controls: `typhoon-engine/src/core/cache.rs` (`DEFAULT_BAR_ZSTD_LEVEL = 3`, `set_bar_zstd_level`, `bar_zstd_level`, `BACKUP_ZSTD_LEVEL = 22`); `put_kv` reads `bar_zstd_level()` too.
- Bar writes (default): `put_bars`, `merge_bars`, and bar batch metadata use the configured base level and record it in `bar_cache.zstd_level`.
- Bar writes (WS fast path): `merge_bars_fast` uses `bar_zstd_level()` (`typhoon-engine/src/core/cache.rs`).
- Merge O(1)-discipline cleanup: `merge_bars_with_level` parses RFC3339 timestamps once into keyed bars before sort/dedup.
- Compact gate / promotion: `compact_storage` at `typhoon-engine/src/core/cache.rs:2390` filters `WHERE zstd_level < target` so WS-written rows are picked up automatically.
- Compact scheduling: `typhoon-native/src/app/auto_compact.rs` runs the scheduled job; the manual `Compact (zstd-22)` button in Storage Manager invokes the same path. Storage Manager also owns the base-level slider/presets and persists the selected base level.

## Non-goals

- Recompressing hot KV/session writes at level 22.
- Blocking egui rendering while compression runs.
- Pretending the entire codebase can literally become O(1); full-depth sync and chart rendering still require O(n) work where n is visible bars, changed rows, or provider payload size. The goal is no avoidable O(n) work in per-frame/control paths and no repeated O(n) scans where maintained indexes/metadata suffice.
