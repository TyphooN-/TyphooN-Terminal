# ADR-032: Performance Architecture — Background Data + Render Decoupling

**Status:** Implemented
**Date:** 2026-03-25

## Context

egui is immediate-mode: every widget must render every frame. SQLite queries running in the render loop caused chart performance to degrade from 60 FPS to <10 FPS when DARWIN/SEC floating windows were open.

The old retained web UI didn't have this problem because:
- React is retained-mode (DOM only updates on state change)
- UI/backend IPC was async (invoke-style calls returned without blocking render)
- Browser compositor handles scrolling independently of JS

## Decision

Move expensive DB queries to a background thread. The current implementation publishes complete `BgData` snapshots through a capacity-one `sync_channel` using nonblocking `try_send`; the render thread drains the newest available snapshot and never waits for the producer.

## Architecture

```
Background Thread (3s lightweight cycle; 5m full refresh):
  → SQLite queries (get_portfolio_summary, compute_var, etc.)
  → Store results in BgData
  → try_send through sync_channel(1); drop publication if one is queued

Render Thread (every frame):
  → try_recv() (never blocks)
  → Replace self.bg; destroy superseded large snapshots off the UI thread
  → ZERO DB queries for portfolio views 0-5
  → Per-account details gated by db_ok (every 8th frame)
```

## Data Cached in Background

| Field | Function | Frequency |
|-------|----------|-----------|
| portfolio | get_portfolio_summary | 5s |
| accounts | list_darwin_accounts | 5s |
| daily_returns | get_portfolio_daily_returns | 5s |
| var_stats | compute_var | 5s |
| correlations | get_darwin_correlations | 5s |
| exposure | get_portfolio_exposure | 5s |
| equity_curve | get_portfolio_equity_curve | 5s |
| open_positions | get_portfolio_open_positions | 5s |
| cache/storage snapshot | background cache scan | lightweight/full cadence |
| sec_filings / insider trades | background refresh | full refresh cadence |
| regulatory alerts / halts | source-specific cadence | 30m / short-lived halt cadence |

## Compression

- Bar data: TTBR binary format (48 bytes/bar) + zstd level 22 on Rust bar-cache writes
- KV data: JSON + zstd level 3 for hot mutable metadata/session writes
- Backup/export: zstd level 22
- Decompression speed is independent enough of source level for charting; choose level by write-path heat
- Auto/manual compact remains for legacy/raw/imported rows whose `zstd_level` metadata is below 22

## Memory Footprint

- Bar struct: 56 bytes × N bars
- Indicator vectors: 30 × N × 16 bytes (Option<f64>)
- Per chart (10K bars): ~5.1 MB
- 9 charts (MTF): ~46 MB
- `BgData` can be large when SEC/news/storage tables are populated; channel capacity is therefore a correctness/performance invariant, not an optional optimization.

## Dependency Policy

Stay on latest stable versions of all crates. Do not wait for upstream to mark releases as stable — adopt early, report issues upstream. Security patches applied immediately. Version alignment documented in ADR-031.

## Consequences

- **Pro:** Floating windows consume cached snapshots instead of issuing repeated render-thread DB queries
- **Pro:** DB work follows explicit lightweight/full-refresh cadences rather than repaint cadence
- **Pro:** `try_recv()` never blocks the render thread and the capacity-one channel bounds retained snapshots
- **Pro:** Background thread handles all expensive computation
- **Con:** Background analytics may be stale by their lightweight/full-refresh cadence (acceptable for non-trading tables)
- **Historical con:** this audit originally left a set of less-used render-thread queries. Follow-up passes moved the critical chart/background-data surfaces to cached/background paths; see ADR-033, ADR-075, and ADR-098 for the current O(1)/zero-hot-path-query posture.
