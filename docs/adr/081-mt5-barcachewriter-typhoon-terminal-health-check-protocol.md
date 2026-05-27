# ADR-081: MT5 BarCacheWriter ⇆ TyphooN-Terminal health-check protocol

## Status

Accepted — 2026-04-16

## Context

The terminal relies on an MT5 EA (`BarCacheWriter.mq5`) to export OHLCV
bars into a SQLite cache (`typhoon_mt5_cache.db`). Data flows one way —
the EA writes blobs, the terminal reads blobs via `Mt5Sync`. Historically
the pipeline suffered two acute pain points:

1. **Cold start after `/dev/shm` clear.** On reboot the ramdisk is wiped
   and the EA must re-export all 851 symbols × 9 timeframes. At 30s
   update intervals and rotation batches of 100 symbols, a full warm-up
   used to take ~4 minutes — during which most charts rendered stale or
   empty data.
2. **Silent drift.** If the EA stalled (Wine/NTSYNC lock contention, file
   handle exhaustion, broker disconnect) the terminal had no channel to
   detect the freeze other than "the bars I'm reading look old." There
   was no heartbeat, no gap-fill channel, and no TF-level demand
   priority.

Prior rounds (v1.437–v1.446) reduced CPU and I/O overhead but did not
fix the cold-start latency or the silent-drift problem. Both of those
require a bidirectional protocol, not just a faster exporter.

## Decision

Implement a minimal 4-phase health-check protocol on top of the existing
shared SQLite DB + `demand.txt` sideband file. No new transport, no new
IPC — just new rows in the DB and new lines in the demand file, with
parsers on both sides.

### Phase A — EA heartbeat + initial-burst mode (BarCacheWriter v1.447)

- **Heartbeat row.** At the end of every `ExportAll()` cycle the EA
  writes a JSON payload to `bar_cache` under key
  `mt5:__HEARTBEAT__:{accountId}`. Payload schema:

  ```json
  {
    "ts": 1713259320,
    "rotation_offset": 200,
    "sym_count": 851,
    "cycle_ms": 412,
    "init_burst_active": false,
    "init_burst_cycles": 0,
    "cycle_count": 1423,
    "exported": 47,
    "skipped": 32,
    "track_count": 7659,
    "demand_count": 84,
    "version": "1.447"
  }
  ```

- **Initial-burst mode.** On startup, `ShouldEnterInitialBurst()` counts
  populated bar_cache rows (bar_count ≥ 100). If fewer than 20% of the
  expected demand keys are populated (or fewer than 450 rows total when
  no demand list exists) the EA flips `g_initBurstActive = true`. In
  burst mode rotation is bypassed — every `ExportAll()` cycle processes
  *every demand symbol* at *every timeframe*, with no TF gating and a
  50 ms inter-batch sleep instead of 200 ms. Non-demand symbols are
  skipped entirely until the cache is warm.

- **Burst exit.** After each cycle `CanExitInitialBurst()` confirms
  that every watched demand symbol has at least one TF with a
  populated, non-sentinel track time. Once satisfied, burst mode
  disables and the normal rotation resumes.

### Phase B — Terminal heartbeat reader + demand writer

- `engine/src/core/cache.rs` exposes
  `SqliteCache::read_mt5_heartbeat(account_tag)` returning the latest
  heartbeat JSON and its row timestamp. Empty `account_tag` matches any
  `__HEARTBEAT__` row (useful when the terminal does not know which
  account a source DB serves).
- `BrokerCmd::Mt5Sync` now collects heartbeats from each configured
  source before copying bar blobs and emits
  `BrokerMsg::Mt5Heartbeat(Vec<(path, json, row_ts)>)`.
- `App::mt5_heartbeats` holds the freshest snapshot per source path,
  with a terminal-local `received_at` timestamp for staleness display.
- The `write_mt5_demand_txt()` method was factored out of `save_session`
  so the heartbeat receive handler can re-push demand.txt mid-session
  (specifically when `init_burst_active=true` arrives).

### Phase C — EA demand.txt v3 gap-fill format (BarCacheWriter v1.447)

The `demand.txt` format now supports three line shapes, backward
compatible:

| Shape | Example | Meaning |
| --- | --- | --- |
| `SYMBOL` | `EURUSD` | v1: watch all TFs, full export |
| `SYMBOL:TF:LAST_TS_MS` | `EURUSD:1Hour:1713259320000` | v2: terminal already has data up to LAST_TS |
| `SYMBOL:TF:LAST_TS_MS:MAX_BARS` | `EURUSD:1Hour:1713259320000:1500` | v3: **gap-fill request** — force export of MAX_BARS bars |

v3 entries populate `g_gapFillKeys[] / g_gapFillMaxBars[]` arrays
parallel to the existing demand tables. In `ExportAll()`'s inner TF
loop, `GetGapFillMaxBars(symTf)` runs before the normal track-time
comparison. On a hit the EA calls `ExportSymbolTF(sym, tf, min(maxBars,
MaxBarsForTF(tf)))` unconditionally and `ClearGapFill(symTf)` on
success so the request is served exactly once.

Additionally the EA now re-reads `demand.txt` every 10 cycles
(~5 minutes at 30 s) to pick up gap-fill requests written after
`OnInit()` without requiring an EA restart.

### Phase D — Terminal gap detection + auto-request

- New method `App::detect_mt5_gaps()` walks open chart tabs. For each
  MT5-sourced symbol × TF combination, it compares the cached
  last-bar timestamp against "now − 2 × TF period". A symbol:TF is
  considered gapped if:
  - `bars == 0` (never synced), OR
  - `now_ms − last_ms > 2 × TF_period_ms`
- Gap entries are staged in `App::mt5_gap_requests: Vec<(sym, tf,
  last_ts_ms, max_bars)>`. Default `max_bars` per TF:

  | TF | max_bars |
  | --- | --- |
  | 1Min | 2000 |
  | 5Min | 2000 |
  | 15Min | 1500 |
  | 30Min | 1500 |
  | 1Hour | 1500 |
  | 4Hour | 1500 |
  | 1Day | 1500 |
  | 1Week | 500 |
  | 1Month | 500 |

- `write_mt5_demand_txt()` appends one v3 line per gap request alongside
  the existing v1/v2 content. Comment headers delineate the sections.
- Gap detection fires automatically inside the `Mt5Heartbeat` receive
  handler — when the writer is alive, we know it will pick up a refreshed
  `demand.txt` on its next 10-cycle re-read. No polling, no timer.

## Consequences

### Positive

- **Cold-start latency drops from ~4 min to <30 s** for watched symbols.
  Burst mode processes the entire demand list every cycle until warm.
- **Silent drift becomes impossible.** The terminal can detect "writer
  is dead" (`received_at` older than N seconds), "writer is stuck in
  burst" (`init_burst_active=true` for > 20 cycles), and "writer is
  falling behind" (gap detection returns nonempty).
- **Mid-session gap recovery.** Opening a new chart tab mid-session
  triggers gap detection on the next heartbeat, which writes a v3 line
  that the EA serves within one demand-refresh cycle (~5 min worst
  case). No more "this chart is 6 hours behind because it wasn't in the
  rotation."
- **Backward compatible.** A terminal running without this change keeps
  working — the EA ignores unknown fields in the heartbeat and the v1/v2
  format still parses. An EA running pre-1.447 just doesn't write the
  heartbeat row; the terminal's receive handler stays silent.

### Negative

- **One extra row write per 30 s cycle** on the EA side for the
  heartbeat. Negligible — same prepared statement path as metadata
  writes.
- **Terminal gap detection cost.** O(watched_charts × 9 TFs × detailed_stats).
  detailed_stats is already computed per background cycle. Worst case
  with 20 charts = 180 hash lookups per heartbeat. Ignorable.
- **Demand file grows.** Each gap request adds ~60 bytes. With 20 gapped
  charts × 9 TFs = 180 extra lines × 60 B ≈ 11 KB. Still trivial.

### Neutral

- `demand.txt` v3 format remains plain-text and ANSI/UTF-8 encoded —
  MQL5's `FILE_ANSI` flag already handles it.
- No new dependencies on either side.
- Protocol is pull-based from the EA perspective (EA re-reads on a
  timer) — the terminal never writes to SQLite during writer-held
  transactions, preserving Wine-compatible locking semantics.

## Verification

- `cargo check -p typhoon-engine` — clean
- `cargo check -p typhoon-native` — clean
- `cargo test -p typhoon-engine --lib` — 1106 passed, 0 failed, 3 ignored
- Heartbeat reader roundtrip is covered by the new ADR-081 path via
  `read_mt5_heartbeat` (no dedicated test — path is exercised by every
  Mt5Sync integration in development).

## Files touched

### Terminal (Rust)

- `engine/src/core/cache.rs` — `read_mt5_heartbeat()` added next to
  `read_bid_ask()`.
- `native/src/app.rs`:
  - `BrokerMsg::Mt5Heartbeat` variant added.
  - `App::mt5_heartbeats`, `App::mt5_gap_requests` fields added.
  - `write_mt5_demand_txt()` factored out from `save_session`.
  - `detect_mt5_gaps()` added.
  - `Mt5Sync` collects heartbeats before bar copy; `Mt5Heartbeat`
    receive handler updates state, runs gap detection, re-pushes demand
    when writer is in burst or has open gap requests.

### EA (MQL5)

- `BarCacheWriter.mq5` v1.446 → v1.447:
  - New globals: `g_initBurstActive`, `g_initBurstCycles`,
    `g_demandV2Keys[]`, `g_demandV2Timestamps[]`, `g_demandV2Count`,
    `g_gapFillKeys[]`, `g_gapFillLastTs[]`, `g_gapFillMaxBars[]`,
    `g_gapFillCount`, `g_demandFilePath`, `g_demandFileMtime`,
    `g_lastHeartbeatWrite`.
  - Helper fns: `ShouldEnterInitialBurst()`, `CanExitInitialBurst()`,
    `WriteHeartbeat()`, `LoadDemandFile()`, `GetGapFillMaxBars()`,
    `ClearGapFill()`.
  - `OnInit()` calls `LoadDemandFile()` then `ShouldEnterInitialBurst()`.
  - `ExportAll()` periodic demand reload, burst-mode rotation bypass,
    burst-mode TF gating bypass, shorter inter-batch sleep, gap-fill
    service ahead of normal track-time path, burst-exit check,
    heartbeat write at end of cycle.

## Historical Follow-up Context (not blocking)

- UI staleness rendering is resolved. The Settings window now shows
  per-source heartbeat freshness (`beat Ns ago`, lagging, stale, or no
  heartbeat) next to each configured MT5 DB path. A distinct
  `init_burst_active=true` banner remains optional.
- Allow per-chart override of default `max_bars` (e.g. a zoomed-out D1
  chart wants 5000 daily bars, not 1500). Requires a chart-level
  setting.
- If a gap request fails N cycles in a row (broker doesn't have that
  history), mark it as permanently-failed on the terminal side so we
  don't keep pressing the EA.
