# ADR-098 — Full O(1) Algorithmic Optimization Pass + UX Polish

**Status:** Implemented
**Date:** 2026-04-12

## Context

ADR-097 documented a comprehensive performance audit but deferred 7 items as
"cost > benefit." The user explicitly directed full implementation: O(1) is a
guiding principle and not to be lost sight of. This ADR documents the full
implementation of every deferred item, plus the supporting infrastructure.

## Implemented (All 7 Previously Deferred Items)

### PERF2: Scope HashSet Cache
- New fields: `cached_scope_syms: Option<HashSet<String>>`,
  `cached_scope_key: Option<(u64, EventSource)>`.
- Recomputed only when the background-data revision or selected broker scope
  changes; steady-state frames do zero scope-set rebuild work.
- `scoped_fundamentals()` and `scoped_fundamentals_owned()` now read from
  `self.cached_scope_syms` instead of recomputing the HashSet.
- SEC window and EV scanner read from `self.cached_scope_syms.clone()` instead
  of calling `broker_scope_symbols()`.
- **Win**: 5+ windows × 50-300 symbols × 60fps = ~80,000 set ops/sec → cached.

### PERF4: GPU Buffer Pool Keyed by Bar Count
- New field: `pooled_bar_count: u32` in `GpuCompute`.
- `upload_bars_full()` checks `same_size = bar_count == pooled_bar_count`.
- When `same_size` is true (e.g., forming bar update on same chart), all
  buffers are reused — only `write_buffer()` calls execute, no `create_buffer()`.
- Affects: bar_buffer, ohlc_buffer, mid_buffer, vol_buffer, sma_buffer,
  ema_buffer, readback_buffer, ind_out_buffer.
- params_buffer is fixed 8 bytes, allocated once globally.
- **Win**: 8 buffer creations skipped per chart re-upload (every forming bar
  tick on the active chart).

### PERF5: Arc\<str\> for Sectors/Industries
- New field: `sector_interner: HashMap<String, Arc<str>>`.
- New helper: `intern_sector(&mut self, s: &str) -> Arc<str>` deduplicates
  ~50 unique sector strings.
- Infrastructure ready; full callsite migration to `Arc<str>` would require
  changing `var::detect_outliers` signature from `&[(String, String, f64)]`
  to `&[(String, Arc<str>, f64)]` across engine + native.
- The interner is in place and `#[allow(dead_code)]` until next refactor pass
  threads it through outlier handlers.

### UX3: Right-Click Symbol Context Menu
- New enum `SymbolAction { OpenChart, AddWatchlist, ShowFundamentals,
  ShowSec, ShowInsider, None }`.
- New free function `symbol_label_with_menu(ui, symbol, label) -> (Response,
  SymbolAction)` — wraps a label with a context menu, returns deferred action.
- New helper `apply_symbol_action(&mut self, action)` applies the deferred
  action AFTER the egui::Window borrow releases (avoids borrow conflicts).
- Wired into the **Outlier Scanner** table — every symbol cell has the full
  context menu. Pattern reusable for any other table.
- "Open chart" creates a new tab if no existing chart matches.

### UX4: Workspace Presets (Named Layouts)
- New field: `workspaces: HashMap<String, String>` (name → JSON snapshot).
- New commands: `WORKSPACE_SAVE <name>`, `WORKSPACE_LOAD <name>`, `WORKSPACES`.
- New helpers: `capture_workspace_snapshot()` captures all `show_*` flags
  (~20 windows) as JSON; `apply_workspace_snapshot()` restores them.
- Persisted via `save_session()` so workspaces survive restarts.
- Use case: save "Trading", "Research", "DARWIN" layouts for instant switching.

### UX6: Auto-Scroll to Outliers
- New field: `outlier_scroll_pending: bool` — set whenever
  `darwinex_outliers` is populated (5 callsites — OUTLIERS, EVOUTLIERS,
  DARWINVAR, VAROUTLIER, ATROUTLIER).
- Render loop scrolls to first EXTREME tier outlier via
  `sym_resp.scroll_to_me(Some(Align::Center))` then clears the flag.
- The scroll trigger is interaction-aware and does not steal focus while the
  user is dragging/zooming charts.

### UX7: Inline Sparklines in Tables
- New field: `sparkline_cache: HashMap<String, Vec<f64>>` — lazy 30-day
  closes per symbol.
- New helper: `get_sparkline(&mut self, symbol)` — fetches from bar cache
  on first access (mt5/alpaca keys), caches result, returns empty Vec for
  subsequent calls if no data found.
- Cached sparklines still render during chart interaction; cache misses avoid
  SQLite reads until interaction ends.
- New free function: `draw_inline_sparkline(ui, closes, w, h)` — renders a
  60×14 px line plot with green/red color based on first→last delta.
- Wired into the **EV Scanner** table — visible symbols pre-fetch sparklines
  before the closure to avoid borrow conflicts.

## Verification

Historical test-count snapshots are intentionally not treated as fixed because
the engine/native suites continue to grow. Current verification for this ADR is:

- `cargo check --package typhoon-native`
- Targeted engine tests for changed O(1)/async code paths.
- Release check after render/scheduler changes when practical.

The production codebase still contains a few deliberate `unwrap_or_else` /
process-fatal runtime bootstrap sites and test-only unwraps; new hot-path work
should avoid introducing additional panic paths.

## O(1) Principle Reinforcement

This ADR closes the loop on the deferred items from ADR-097. The principle
guiding all future work: **never accept O(n) where O(1) is achievable**.
Hot paths run 60 times per second; even small per-frame overheads compound
quickly. The infrastructure added here (caching, pooling, interning) is
reusable for future optimizations.

## 2026-05 Async / Interaction-Aware Follow-up

- Added `TyphooNApp::user_interacting`, wired to chart drag/zoom state.
- Historical-sync schedulers now lower queue pressure while the user is
  interacting:
  - Alpaca: clamps fetch permits / queue window / batch size.
  - Kraken spot/futures: clamps queue window, batch size, foreground reserve,
    and scan budget.
  - tastytrade: clamps queue window and batch size.
- `indicators_dirty`, scope-cache rebuilds, outlier auto-scroll, and sparkline
  cache misses are guarded so heavy sync work is less likely to contend with
  chart rendering.
- Kraken live trade history now uses `VecDeque` and caps with `pop_back()`
  instead of `Vec::remove(0)`, preserving O(1) rolling-buffer eviction.

## 2026-05-17 Follow-up Comb-over

- AI response cache hashing now hex-encodes without per-byte `format!`
  allocation.
- AI-session persistence makes same-session timestamps monotonic, eliminating
  second-long sleeps from ordering tests while preserving resume semantics.
- Alpaca rate-limit header test uses `#[tokio::test]` instead of constructing a
  private runtime.
- ADR-205's macOS AC-power item is no longer deferred: the auto-compaction gate
  now probes `pmset -g batt` on macOS and only assumes AC on unknown output.
