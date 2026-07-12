# ADR-033: Background Data Channels (Zero DB Queries on UI Thread)

**Status:** Implemented
**Date:** 2026-03-25 | **Updated:** 2026-07-12

## Context

The native egui+wgpu terminal was freezing ("Application Not Responding") when DARWIN Portfolio, DARWIN Accounts, SEC Filing, VaR Multiplier, or other analytics windows were open. Root cause: **60+ synchronous SQLite queries executing on the egui UI thread** inside `draw_floating_windows()` on every repaint.

A partial mitigation existed — a background thread computed ~15 fields into `Arc<Mutex<BgDarwinData>>`, but the rendering code largely ignored it and queried the DB directly. A `db_ok` frame-gating hack (`self.cache.take()` on non-DB frames) reduced query frequency but caused UI flickering as windows rendered empty on skipped frames.

### Measured Impact (before fix)

| Window Open | DB Queries/Frame | Estimated Blocking Time |
|-------------|-----------------|------------------------|
| DARWIN Accounts (6 accounts expanded) | 30+ per-account queries | ~200-500ms |
| DARWIN Portfolio (any view) | 5-15 queries per view | ~50-150ms |
| VaR Multiplier | N × per-account VaR computation | ~100-300ms |
| SEC Filing Scanner | 3 queries (filings, alerts, importance) | ~20-50ms |
| Right Panel Risk tab | 3 queries (portfolio, daily returns, VaR) | ~30-80ms |
| **Combined (typical use)** | **50-80 queries** | **~400-1000ms** |

At 250ms repaint interval, the UI thread spent more time in SQLite than rendering.

## Decision

Replace `Arc<Mutex<BgDarwinData>>` with `mpsc::sync_channel(1)` — a bounded channel where the background thread sends complete data snapshots and the UI thread only reads cached values.

### Architecture

```
┌──────────────────────────┐     sync_channel(1)     ┌──────────────────────────┐
│   Background Thread      │ ───── BgData ─────────── │    UI Thread (egui)      │
│                          │                          │                          │
│  3s lightweight /        │                          │  Every frame:            │
│  5m full refresh:        │                          │                          │
│  1. Open SQLite conn     │                          │  1. Drain bg_rx channel  │
│  2. Run ALL queries      │                          │  2. Store as self.bg     │
│  3. Compute analytics    │                          │  3. Render from self.bg  │
│  4. Send snapshot        │                          │  4. Zero DB queries      │
│  5. Release conn         │                          │                          │
│  6. Sleep 5s             │                          │  Button actions only:    │
│                          │                          │  cache.connection() for  │
│  80+ fields computed:    │                          │  XLSX import, delete,    │
│  - Portfolio summary     │                          │  report, export, FTP,    │
│  - Per-account details   │                          │  dismiss alert, what-if  │
│  - VaR, Monte Carlo      │                          │                          │
│  - Correlations          │                          │                          │
│  - SEC filings           │                          │                          │
│  - Insider trades        │                          │                          │
│  - Stress tests          │                          │                          │
│  - All 20 portfolio views│                          │                          │
└──────────────────────────┘                          └──────────────────────────┘
```

### Channel Design

- **`sync_channel(1)`**: Bounded capacity of 1. If the UI has not consumed the queued snapshot, `try_send()` drops the publication. This invariant was restored on 2026-07-12 after the implementation had drifted to an unbounded channel; runtime evidence showed `VmHWM` above 45 GB while full `BgData` clones accumulated behind UI stalls.
- **UI drain pattern**: nonblocking `try_recv`; superseded snapshots are dropped on a blocking worker so destruction of large vectors does not stall egui.
- **Data freshness**: lightweight publication runs on a 3-second cycle and full refresh on a 5-minute cadence. Broker positions, orders, and quotes use the separate broker channel.

### What Moved Off the UI Thread

**Per-account analytics** (previously queried per-frame when DARWIN Accounts window was open):
- Summary, VaR stats, monthly returns, streak analysis, hourly P&L
- Equity curve, P&L by symbol, day-of-week, hold time, Kelly criterion
- Cost analysis, D-Score estimate, slippage, MAE/MFE, sizing efficiency
- Symbol rotation, open positions, pyramiding, trading bursts
- Autocorrelation, recent deals, closed positions, equity snapshots
- Benchmark comparison, sector classification

**Portfolio views** (DARWIN Portfolio window, 20 views):
- Rolling VaR, drawdown dashboard, exposure treemap
- Timing divergences, VaR forecast, conditional VaR
- Market regime, regime performance, tail risk, seasonal analysis
- Sector exposure, liquidity risk, floating equity, tax lots

**Standalone windows**:
- Symbol overlap, correlation matrix, VaR multiplier
- SEC filings, filing alerts, insider trades (per-symbol)
- Cache stats, detailed stats

**Right panel**:
- Portfolio open positions, portfolio summary, VaR stats

### What Stays on the UI Thread

User-initiated one-shot actions that need immediate DB access:
- XLSX import (`darwin::import_darwin_xlsx`)
- Delete account (`darwin::delete_darwin_account`)
- Daily risk report (`darwin::generate_daily_report`)
- Export radar (`darwin::export_radar_txt`)
- FTP scan (`darwin::find_low_correlation_darwins`, `darwin::scan_darwin_ftp`)
- Dismiss filing alert (`sec_filing::dismiss_alert`)
- What-if close symbol (`darwin::what_if_close_symbol`)
- Chart bar loading (`cache.get_bars_raw`) — on symbol/timeframe change only

## Consequences

### Positive
- UI thread does zero SQLite queries during rendering — no more "Application Not Responding"
- Eliminates the `db_ok` / `real_cache` frame-gating hack and its flickering side effect
- Data is always available (never `None` due to frame gating)
- Background thread releases SQLite connections between query phases, reducing lock contention
- Simple mental model: UI reads `self.bg.*`, background writes `self.bg` via channel

### Negative
- Background analytics can be stale by their configured lightweight/full-refresh cadence (acceptable for non-trading tables)
- Background thread does more work per cycle (80+ queries vs 15), taking ~2-5s per cycle
- `BgData` can own very large SEC/news/storage collections; only one unpublished clone may be retained
- Per-account details scale with number of DARWIN accounts (currently 6, each adds ~25 queries)

### Neutral
- Broker real-time data (positions, orders, quotes) continues using the separate tokio mpsc channel — unaffected
- Chart rendering (candlesticks, indicators) continues using direct `&[Bar]` slice access — unaffected
- Session persistence follows its own coalesced off-thread writer; credential/keyring maintenance is independent of `BgData` publication

## Pattern Reuse

This same `sync_channel(1)` pattern can be applied to any future heavy computation:

```rust
// 1. Define data struct
struct HeavyData { /* fields */ }

// 2. Create channel
let (tx, rx) = std::sync::mpsc::sync_channel::<HeavyData>(1);

// 3. Background thread
std::thread::spawn(move || loop {
    let data = compute_heavy_stuff();
    let _ = tx.try_send(data); // drop if UI hasn't consumed yet
    std::thread::sleep(Duration::from_secs(N));
});

// 4. UI thread (in update())
while let Ok(data) = rx.try_recv() {
    self.cached_data = data;
}
// Render from self.cached_data — never block
```
