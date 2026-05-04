# ADR-205: ZSTD Compression Level Policy and Auto-Compaction

**Status:** Implemented
**Date:** 2026-04-28 (decision), 2026-04-29 (auto-compact wired), 2026-05-03 (Storage Manager schedule controls)
**Related:** ADR-003 (SQLite + zstd cache), ADR-052 (performance architecture), ADR-079 (LAN sync bandwidth), `engine/src/core/cache.rs`, `native/src/app/auto_compact.rs`, `native/src/app.rs::BrokerCmd::CompactStorage`

## Context

The cache uses zstd at three different levels depending on the access pattern:

| Class | Level | Examples |
|---|---|---|
| Hot synchronous puts | 3 | `put_bars` (`cache.rs:603, :1224`), `put_kv` (`cache.rs:743`), `store_filing_content` (`sec_filing.rs:1247`), `darwin` symbol-specs (`darwin.rs:2880`), LAN-sync server responses (`lan_sync.rs:5015, :5198`) |
| Snapshot / export | 9 | `export_backup` (`cache.rs:1378`), pre-compressed bulk blobs per ADR-079 |
| User-invoked compact | 22 | `compact_storage` (`cache.rs:2168`), `BrokerCmd::CompactStorage` (`app.rs:35280`) |

A schema column `bar_cache.zstd_level` (default 3) tracks the level each entry is currently stored at, so compact passes skip already-compacted rows (`cache.rs:381, :2181`).

A recurring temptation — including in conversation on 2026-04-28 — is to make zstd-22 the default everywhere, on the reasoning that modern CPUs have spare cores, that compression can run async, and that data lives in memory while compressing so there is no real latency penalty.

This ADR exists to record why that move is rejected, and what the periodic auto-compact policy should look like instead.

## Decision

### 1. Keep the tiered model

Compression levels stay as in the table above:

- **Level 3** for all hot synchronous puts (bar/KV/filing/AI-session/LAN-sync responses).
- **Level 9** for snapshot/transfer paths (backups, pre-compressed bulk blobs).
- **Level 22** for the user-invoked `STORAGE` compact pass and for periodic auto-compact (§3).

### 2. Reject "zstd-22 by default everywhere"

The argument *for* default-22 is: ratio improves, async hides latency, cores are cheap. The reasons against, recorded so they survive contributor rotation:

**Encode cost vs. ingest are 200x apart, not 2x.** On OHLCV-class data:

| Level | Encode | Decode | OHLCV ratio | Per-1MB encode |
|---|---|---|---|---|
| 3 | ~500 MB/s | ~1500 MB/s | ~3.5x | ~2 ms |
| 9 | ~50 MB/s | ~1500 MB/s | ~4.0x | ~20 ms |
| 22 | ~1–3 MB/s | ~1500 MB/s | ~4.5x | ~333–1000 ms |

Decompression is identical at every level — only encode cost changes. The hot put path (MT5/tastytrade sync threads, AI-session writes, KV puts) can produce data faster than 1–3 MB/s drains.

**Async hides caller latency, not CPU cost.** If hot puts queue into an async compress worker:
- Without backpressure, RAM grows unbounded under bursty ingest (uncompressed buffers held while the worker drains at 1–3 MB/s).
- With backpressure, the caller blocks anyway when the queue is full.

Either way the encode cost is paid. "In memory while compressing" is not free — it just shifts where the wait happens.

**Ratio gain is ~20% on already-compressed data.** Going from 3.5x to 4.5x means roughly 1 MB saved per symbol-year of M1 bars at the cost of hundreds of times the CPU per write. SSDs are cheap; CPU time on a hot path is not.

**LAN sync is request/response.** The server compresses per client request (`lan_sync.rs:5015, :5198`). zstd-22 turns a sub-100ms RPC into a multi-second hang from the *client's* perspective. Async on the server does not help — the client is on the wire either way.

**Battery and thermal matter.** This is a desktop trading app that runs on laptops and trading boxes already under thermal pressure during market hours. Sustained zstd-22 work spins fans and drains batteries for a marginal win.

**The read path does not benefit.** Chart render, AI-session resume, and LAN-sync delivery decompress at the same speed regardless of level. Higher levels only optimize cold-storage size, not runtime performance.

The current tiered model is the standard zstd-recommended pattern: fast levels for hot writes, snapshot levels for transfers, max level only for cold-storage compaction. The `bar_cache.zstd_level` column makes compaction one-time per entry — pay zstd-22 once on cold data that won't be rewritten, never again.

### 3. Auto-compaction policy

Periodic auto-compact is allowed and recommended, but conservative:

- **Cadence:** configurable from Storage Manager; defaults to weekly, Sunday 04:00-05:00 local.
- **Gating, all required** (`auto_compact::evaluate_gate`):
  - AC power detected (skips on battery, Linux only — non-Linux assumes AC).
  - User idle ≥ 5 minutes (no input events seen).
  - `COUNT(*) FROM bar_cache WHERE zstd_level < 22` exceeds the configured threshold (default 100). Small deltas are not worth waking up for.
  - Local time inside the configured weekday + hour window.
  - Cadence: at least the configured number of days since the last run.
- **Dispatch:** the gate sends `BrokerCmd::CompactStorage { level: 22 }` — the same command the manual button uses. The existing handler (`app.rs::BrokerCmd::CompactStorage`) already sets `importing_flag` so the background stats worker yields, runs `compact_storage` on a worker thread, and calls `incremental_vacuum(10000)` on success. No duplicated logic.
- **Incrementality:** `compact_storage` already skips entries with `zstd_level >= target` (`cache.rs:2181`). Steady-state work is bounded by the fresh-data delta since the last run, not the full cache.
- **User opt-out/config:** "Auto-compact" checkbox plus cadence, weekday, start/end hour, and min-row controls in Storage Manager. The manual `Compact (zstd-22)` button always works regardless.
- **Stale-flag recovery:** the dispatch timestamp is tracked separately; if the in-progress flag has been set for > 8 hours and no completion log arrived, the next tick resets it so the gate can recover from a lost completion message.

This makes the "spare cycles" model work without ever surprising the user mid-trade.

### Implementation pointers

- Gate logic + tests: `native/src/app/auto_compact.rs` (14 unit tests).
- Tick: `TyphooNApp::tick_auto_compact` in `native/src/app.rs`, called once per `update()` and self-throttled to one evaluation per minute.
- User-input timestamp updated each frame from `ctx.input(|i| !i.events.is_empty())`.
- Persistence: `auto_compact_enabled`, `auto_compact_last_run_ms`, and the schedule/threshold fields ride in `app:sync_preferences` KV (alongside `crypto_backfill_enabled`).
- Completion / failure: `BrokerMsg::OrderResult` matches the `Compact complete:` prefix to clear in-progress and stamp `last_run_ms`; `BrokerMsg::Error` matching `Compact failed:` clears in-progress without bumping the timestamp so the cadence keeps trying.
- New cache helper: `SqliteCache::count_uncompacted_bars(target)` (`engine/src/core/cache.rs`).
- UI: Storage Manager checkbox, schedule controls, `last: <duration>` / `next: <time>` / `(skip: <reason>)` / `running…` readout, immediately under the manual `Compact (zstd-22)` button.

## Consequences

**Pro:**
- Hot-path latency stays bounded — sub-millisecond compresses on typical bar batches.
- Storage trends toward zstd-22 without user effort, but only on cold data.
- LAN-sync RPCs stay fast for clients.
- Battery/thermal cost bounded to opt-in idle windows.
- The rationale for rejecting "always 22" is recorded so the discussion does not have to be relitigated.

**Con:**
- Three compression levels to remember when adding a new write site (mitigated: pick the level by access pattern, not by site).
- Auto-compact requires power-state and idle-detection plumbing (small).

## Non-Goals

- Changing the decompression path (already optimal — single decode speed for all levels).
- Changing the backup level from 9 (appropriate for one-shot exports).
- Changing the manual `STORAGE` compact level from 22.

## Remaining Tuning Knob

- ~~Wire the auto-compact scheduler with the gating described in §3.~~ (Shipped 2026-04-29.)
- ~~Expose the on/off toggle and last-run readout in the Storage Manager UI.~~ (Shipped 2026-04-29.)
- ~~Make the cadence, weekday, hour window, and uncompacted-row threshold user-configurable from the Storage Manager.~~ (Shipped 2026-05-03.)
- ~~Expose `next_auto_compact_at` (next time the gate will be re-evaluated against the schedule) so users can see what is scheduled rather than only when it last ran.~~ (Shipped 2026-05-03 as `next: <local time>`.)
- AC-power detection on Windows / macOS — currently those platforms always pass the AC gate. Low priority since the trading rigs in scope are Linux desktops.
