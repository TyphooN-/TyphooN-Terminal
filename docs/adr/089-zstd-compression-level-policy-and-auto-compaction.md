# ADR-089: ZSTD Compression Level Policy and Auto-Compaction

**Status:** Updated / Implemented
**Date:** 2026-04-28 (original decision), 2026-04-29 (auto-compact wired), 2026-05-03 (Storage Manager schedule controls), 2026-05-20 (bar-cache writes moved to zstd-22), 2026-05-24 (WS hot-path carve-out at zstd-3, see ADR-099), 2026-06-03 (user-selectable base write level)
**Related:** ADR-003 (SQLite + zstd cache), ADR-032 (performance architecture), ADR-058 (LAN sync bandwidth), ADR-099 (Kraken WS full-universe responsiveness), `engine/src/core/cache.rs`, `native/src/app/auto_compact.rs`, `native/src/app.rs::BrokerCmd::CompactStorage`

## Context

TyphooN now targets maximum historical depth across supported brokers. That changes the storage tradeoff: the bar cache is no longer a small rolling cache; it is the long-lived local market-data store. Recompressing later via a scheduled job or manual Storage Manager button is wasted lifecycle complexity when the write path can store final bar blobs at the archival level immediately.

The important distinction is bar data vs hot mutable KV data:

| Class | Level | Examples |
|---|---:|---|
| Bar cache writes (REST / imports / normal merges) | User-selected base level, default 3 (1-22) | `put_bars`, `merge_bars`, broker backfill, new bar-cache rows; persisted in sync preferences and exposed in Storage Manager |
| **Bar cache hot writes (WS bar-close)** | **3** | `merge_bars_fast` from Kraken WS OHLC writer (`native/src/app/kraken_ohlc_ws.rs`) keeps the latency carve-out regardless of the base setting |
| KV / metadata hot writes | 3 | AI sessions, broker queues, fundamentals metadata, LAN metadata |
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

### 2. Keep hot mutable KV at zstd-3

Do not blindly use zstd-22 for every blob.

KV/session/metadata paths are small, frequently rewritten, and closer to user-visible interaction loops. They remain level 3 unless a specific path is proven cold and worth promoting.

This is not a retreat from max compression for market data; it separates foreground write throughput, hot mutable metadata, and idle archival compaction.

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
- **Kraken WS OHLC writes (`merge_bars_fast`) intentionally store at zstd-3** so the snapshot storm on first subscribe (≈12k keys × ≈720 closed bars) does not saturate CPU and stall egui. Compactor promotes those rows back to zstd-22 on its next pass.

The compactor keeps the `zstd_level < target` filter at `engine/src/core/cache.rs::compact_storage`, so anything below 22 — configured-base writes, legacy rows, WS hot writes, raw LAN rows — is naturally promoted on the next scheduled or manual run. The auto-compact gate + `Compact (zstd-22)` button in Storage Manager close the loop without making every foreground write pay max-compression CPU.

### 5. Carve-out: WS bar-close writer uses zstd-3

The Kraken WS OHLC writer is the only `put_bars_*` path that intentionally bypasses level 22 on first write. Justification:

- The WS pipeline writes one merge per (symbol, timeframe) per bar-close (~25 closes/sec at steady state with the full Spot universe), but the load-bearing event is the initial **snapshot storm**: every freshly subscribed (pair, interval) hands back the last ≈720 closed bars in one batch. With ≈1500 pairs × 8 intervals that is ~12k cache entries to re-pack inside the first flush window. At zstd-22 (encoder ~5–10 MB/s) that work pegs every core for tens of seconds and is exactly the workload that visibly stalls the UI.
- zstd-3 (encoder ~150–200 MB/s) cuts that to a few seconds of background work behind `spawn_blocking`, keeping the egui thread idle.
- Storage cost of the carve-out is ≤20% per affected blob until the next compaction. With the bar cache typically a few hundred MB total and compaction running automatically, that is a transient overhead the user does not see.
- Read path is unchanged: `get_bars` decompresses whatever level the row was written at; chart loads pay the same TTBR unpack regardless.

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

- Bar compression controls: `engine/src/core/cache.rs` (`DEFAULT_BAR_ZSTD_LEVEL = 3`, `set_bar_zstd_level`, `bar_zstd_level`, `BACKUP_ZSTD_LEVEL = 22`, `KV_ZSTD_LEVEL = 3`).
- Bar writes (default): `put_bars`, `merge_bars`, and bar batch metadata use the configured base level and record it in `bar_cache.zstd_level`.
- Bar writes (WS hot path): `merge_bars_fast` → `put_bars_with_level(.., 3)` (`engine/src/core/cache.rs`).
- Merge O(1)-discipline cleanup: `merge_bars_with_level` parses RFC3339 timestamps once into keyed bars before sort/dedup.
- Compact gate / promotion: `compact_storage` at `engine/src/core/cache.rs:2390` filters `WHERE zstd_level < target` so WS-written rows are picked up automatically.
- Compact scheduling: `native/src/app/auto_compact.rs` runs the scheduled job; the manual `Compact (zstd-22)` button in Storage Manager invokes the same path. Storage Manager also owns the base-level slider/presets and persists the selected base level.

## Non-goals

- Recompressing hot KV/session writes at level 22.
- Blocking egui rendering while compression runs.
- Pretending the entire codebase can literally become O(1); full-depth sync and chart rendering still require O(n) work where n is visible bars, changed rows, or provider payload size. The goal is no avoidable O(n) work in per-frame/control paths and no repeated O(n) scans where maintained indexes/metadata suffice.
