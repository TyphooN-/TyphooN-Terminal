# UI Responsiveness & Sync — Implementation Status

_Last updated: 2026-07-12_

> **Historical (2026-06):** the MT5 `BarCacheWriter` external writer referenced below was removed with the broker rip-out (ADR-111). The SQLite write-contention insight still applies — the contending writers are now the Rust bulk bar-sync tasks, not the MT5 EA.

Investigation + fixes for the reported symptoms: UI lag / stalls, `ask/bid/last`
decoupling, and "improve data sync." This doc records what shipped, the root
cause, and the pros/cons of pushing further.

## Historical root cause

The native UI's "UI frame stall detail" logs were dominated by `session_save_ms`
— regularly **4.4s**, spiking to **18–23s**, firing every ~6s. `session.json` is
only ~6.6 KB, so serialization was never the cost. The time was the **write
path** blocking on the cache's single write connection.

`SqliteCache` (`typhoon-engine/src/core/cache.rs`) has one `conn: Mutex<Connection>`
(WAL, `busy_timeout=5s`, `wal_autocheckpoint=2000`) shared by all Rust writers.
The Rust write path is already lock-lean — `put_bars` / `put_bars_with_level` /
`merge_bars` compress **outside** the lock and hold `conn` only for a single-row
INSERT. The real blocker is an **external process**: `BarCacheWriter` (the MT5 EA
under Wine) writes the same SQLite file with `journal_mode=DELETE` + **exclusive**
write locks. While it is mid-transaction, every Rust write blocks on
`SQLITE_BUSY` up to `busy_timeout` — **5s = the 4.4s stalls**. The 18–23s cases
stack a BCW hold + waiters queued on the single `conn` mutex.

The per-frame session autosave was doing its `put_kv` on the **render thread**,
so it inherited that 5s block. That one contention point also amplified the
other two symptoms: a frozen render thread can't drain quote ticks (bid/ask go
stale vs. last), and it steals the write lock from the sync pipeline.

## Shipped (branch: `master`)

| # | Change | Files | Effect |
|---|--------|-------|--------|
| **P0** | Single-owner off-thread session autosave | `session_persistence/`, `state.rs`, `app_runtime.rs` | Incremental autosave skips heavy sync, coalesces writes, and persists on a blocking worker. The duplicate 60-second path that rebuilt session JSON and synchronously wrote preferences was removed; that timer now checks credentials only. |
| **P2** | Floating-window render timing | `floating_windows/mod.rs` | `timed_window!` macro logs `slow floating window: <name> took <ms>` when any single window render >500ms. Diagnostic to attribute the rare 12–16s `floating_windows_ms` spikes (cause not yet identified statically). |
| **A** | Startup DB-walk off render thread | `sync_status.rs`, `app_runtime.rs` | Removed the synchronous `refresh_storage_snapshot_from_cache` fallback (full ~86k-row `bar_cache` scan) from the per-frame `refresh_bar_sync_rows_if_stale`. The BG thread populates `cache_stats`/`detailed_stats` from its own connection; `show_sync_status` added to the BG-snapshot allowlist so the window stays fed during heavy sync. |
| **C** | Stale bid/ask guard | `chart.rs`, `app_runtime.rs`, `technical_analysis.rs` | `ChartState.live_quote_at` stamps quote receipt; the spread lines are hidden once the quote is **>30s stale**, so a frozen bid/ask isn't drawn as "live" next to a moving candle. Addresses the decoupling display. |
| **D** | Saturated broker-refill backpressure | `app_runtime_broker_messages.rs` | Heavy-sync settlement batches do not immediately rerun broad catalog selection while pending work is above 200; the periodic scheduler owns refill until the queue drains. |
| **E** | Bounded background snapshots | `app_background.rs` | `BgData` publication is capacity-one `sync_channel` + `try_send`; UI stalls cannot accumulate multi-GB clones. Superseded snapshots are destroyed off the render thread. |

### How to verify
Run a release build and watch the logs / chart:
- `session_save_ms` should be ~0 every frame; the multi-second `session_save`
  stall lines gone.
- Stale bid/ask spread lines disappear instead of freezing far from `last`.
- Use phase attribution rather than assuming one cause: high `pre_broker_ms` means startup/snapshot/scheduler work; high `broker_drain_ms` means message handling or post-drain refill; high chrome/floating timings name a UI surface; minute-aligned `render_residual_ms` previously identified the duplicate autosave.

## Pushing further — pros/cons

### B — write-coalescing queue (DECLINED, low priority)
Batch the many small `put_bars` into grouped `put_compressed_batch` transactions
to cut how often Rust writers collide with BarCacheWriter's lock.

- **Cons (why declined):** The visible bar-write sites each write **one
  symbol×TF per call** and are **API-rate-limited** (`typhoon-broker-runtime/src/broker_processor.rs`;
  Alpaca `FetchAllBars` is explicitly sequential; kraken-equities is gated by the
  iapi ~6 req/s wall). Sparse, rate-limited writes have **no burst to coalesce**.
  A cross-task queue would also risk **bar-data correctness** (`merge_bars` is a
  read-modify-write that can't be naively batched). `put_compressed_batch` is
  currently dead code.
- **Pros (if revisited):** Only worth it if the Yahoo/Alpaca *breadth* path
  actually bursts — unverified. Gate the decision on a **bar-writes/sec counter**
  during a sweep before building anything.
- **Net:** P0 already removed the UI-thread impact; sync throughput is
  API-bound. Not worth the complexity/risk now.

### Startup DB-walk (DONE — was item A)
Shipped above. Pro: removes a real render-thread full-table scan. Con: the Sync
Status window may show stale/partial rows for up to one ~3s BG cycle at startup
(acceptable vs. a multi-second freeze).

### Secondary — since resolved (2026-06/07)
- ~~`pre_broker_ms` steady ~290ms tax~~ **Fixed:** the cause was the full
  12k-catalog bar-sync coverage matrix scan running as pure CPU on the render
  thread (~500ms spikes every ~120s under heavy sync). It now runs on a
  `spawn_blocking` worker with an mpsc result poll.
- ~~12–16s `floating_windows` spikes~~ **Fixed structurally:** the spikes were
  SQLite contention — render-thread `put_kv`/DB walks blocking on the single
  connection mutex held by bulk bar-sync writers. Cold per-chart loads moved
  off the render thread (deferred chart loader + result-cache restore,
  2026-06-30), `read_conn` became a read-connection pool (`ReadConnPool`),
  and cache compaction now streams by key cursor with `incremental_vacuum`
  under a heavy-sync gate instead of load-all + full VACUUM. The `timed_window!`
  diagnostic (P2) remains in place to attribute any recurrence.

## Related
- ADR-107 (no user-interacting sync throttle), `docs/PERFORMANCE.md`,
  `docs/floating-windows-perf-plan.md`.
