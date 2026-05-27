# ADR-032: Performance Architecture — Background Data + Render Decoupling

**Status:** Implemented
**Date:** 2026-03-25

## Context

egui is immediate-mode: every widget must render every frame. SQLite queries running in the render loop caused chart performance to degrade from 60 FPS to <10 FPS when DARWIN/SEC floating windows were open.

The old WebKit/Tauri didn't have this problem because:
- React is retained-mode (DOM only updates on state change)
- Tauri IPC is async (invoke() returns Promise, doesn't block render)
- Browser compositor handles scrolling independently of JS

## Decision

Move expensive DB queries to a background thread. Render thread reads from cached data via `Arc<Mutex<BgDarwinData>>` with `try_lock()` (never blocks).

## Architecture

```
Background Thread (every 5s):
  → SQLite queries (get_portfolio_summary, compute_var, etc.)
  → Store results in BgDarwinData
  → lock() to write (blocks only bg thread)

Render Thread (every frame):
  → try_lock() to read (never blocks — skips if bg holds lock)
  → Render from cached data
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
| cache_stats | cache.stats() | 5s |
| sec_filings | get_recent_filings | 5s |
| sec_alerts | get_filing_alerts | 5s |

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
- BgDarwinData: ~50 KB (negligible)

## Dependency Policy

Stay on latest stable versions of all crates. Do not wait for upstream to mark releases as stable — adopt early, report issues upstream. Security patches applied immediately. Version alignment documented in ADR-031.

## Consequences

- **Pro:** Charts stay at 60 FPS regardless of open floating windows
- **Pro:** DB queries run once every 5 seconds, not 4x/second
- **Pro:** try_lock() never blocks render thread
- **Pro:** Background thread handles all expensive computation
- **Con:** DARWIN data may be up to 5 seconds stale (acceptable for analytics)
- **Con:** ~114 render-thread queries remain in less-used views (negligible impact)
