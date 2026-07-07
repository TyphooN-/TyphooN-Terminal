# ADR-098: Performance & O(1) Optimization Program

> Formerly "Per-Frame O(1) Discipline in Chart and Sync Paths." Retitled 2026-06 when this became the durable home for the terminal's performance program — see *Consolidated execution-log ADRs* at the end.

**Date:** 2026-05-22
**Status:** Accepted
**Last updated:** 2026-07-07 (iterative O(1) sweeps)
**Related:** ADR-032 (background data + render decoupling),
`typhoon-native/src/app/technical_analysis.rs`,
`typhoon-native/src/app/app_runtime.rs`,
`typhoon-native/src/app/app_runtime_errors.rs`,
`typhoon-native/src/app/chart_ops.rs`,
`typhoon-native/src/app/market_data_sync.rs`,
`typhoon-native/src/app/session_persistence.rs`,
`typhoon-native/src/app/trade_ops.rs`

## Context

egui is immediate-mode. Every panel, every label, and every chart redraws
every frame. ADR-032 already moved expensive DB queries off the render
thread, but in-process linear scans, repeated symbol normalization, and
redundant `Clone` calls had crept back into hot per-frame and per-message
paths as the surface grew (broker quote handlers, MTF grid render,
indicator overlays, sync schedulers).

Symptoms read as "feels heavier than it should":

- `price_to_y` ran `price_max.ln()` and `price_min.ln()` on every call.
  That closure fires once per visible bar per overlaid indicator —
  hundreds of calls per frame, each repeating the same two `.ln()` evals.
- Fair Value Gap rendering scanned `bars[i+2..]` for every gap candidate
  to decide "filled or not" — O(n²) per FVG-enabled chart render.
- `kraken_spot_symbol_in_loaded_pairs` did a linear scan that re-ran
  `normalize_pair_symbol` (allocating) twice per element, multiplied by
  every caller that audits the universe each sync tick.
- `refresh_kraken_position_costs` built `updates` from `kr_positions` in
  order, then used `updates.iter().find(symbol == pos.symbol)` to re-pair
  them — O(n²) for data that was already aligned by index.
- `normalized_right_panel_order` deduped 8 section IDs with chained
  `out.contains(&section)` — O(n²) every frame the right panel rendered.
- MTF grid + single-chart draw paths cloned `cached_trade_overlay` per
  cell per frame just to side-step the borrow checker.
- `Mt5LiveQuotes` and `WatchlistQuotes` allocated a fresh
  `sym.to_uppercase()` String per row instead of reusing one buffer.
- `build_trade_overlay` re-computed `pos.symbol.replace('/', "").to_uppercase()`
  inside the broker-position loop even though `bare_upper` was already
  available at function scope.

Individually these are small. Collectively, at 60 Hz with N charts open
and busy quote feeds, they translate into measurable allocation pressure
and CPU drag during pan/zoom.

## Decision

Per-frame and per-message paths must be **O(1) per element after fixed
setup**, and they must avoid heap allocations that scale with iteration
count. Specifically:

1. **Hoist invariant work out of inner closures.** Anything that depends
   only on viewport / range / chart geometry is computed once per render,
   not per visible bar. `price_to_y` now reads pre-computed `log_max`,
   `log_min`, `log_range`, `linear_range`, `chart_top`, `chart_h`.

2. **Suffix / prefix arrays beat per-candidate scans.** FVG fill checks
   use `future_min_low[k] = min(bars[k..].low)` and `future_max_high[k] =
   max(bars[k..].high)` — one O(n) sweep, O(1) lookup per candidate.

3. **Pre-normalized HashSets beat linear `.iter().any()` over
   normalize_then_compare.** Kraken pairs build a
   `kraken_pairs_normalized: HashSet<String>` once when
   `BrokerMsg::KrakenPairs` arrives and `kraken_spot_symbol_in_loaded_pairs`
   becomes a `contains()`.

4. **Zip instead of join-by-key when data is already index-aligned.**
   `refresh_kraken_position_costs` builds `updates` from `kr_positions`
   in order, so the pairing step is `iter_mut().zip(updates.into_iter())`,
   not a symbol search.

5. **Bitsets replace `Vec::contains` for fixed-cardinality enums.**
   `normalized_right_panel_order` tracks membership of the 8
   `RightPanelSectionId` variants in a `u64` bitset; both passes (preserve
   user order, then append missing defaults) are O(n).

6. **`std::mem::take` + restore replaces clone for per-frame borrows.**
   The MTF grid and single-chart render move
   `cached_trade_overlay` out of the chart, pass it to `draw_chart` as
   `&TradeOverlay`, then put it back — avoiding `Vec<TradeMarker>`
   (with `String` tickers) clones per cell per frame.

7. **Reusable String buffers across batch loops.** `Mt5LiveQuotes` and
   `WatchlistQuotes` declare one `String::with_capacity(32)` outside
   their for-loop and reuse it for each row. `HashMap<String, _>::get`
   accepts `&str` via `Borrow`, so the lookup keeps the same shape.

8. **Allocation-free comparison helpers for `replace+uppercase`
   equality.** `chart_ops::symbol_matches_no_alloc` walks both strings
   byte-by-byte instead of allocating
   `pos.symbol.replace('/', "").to_uppercase()` per inner-loop check.

## Quantitative Notes

| Path                                     | Before                | After                |
|------------------------------------------|-----------------------|----------------------|
| `price_to_y` (log scale)                 | 2× `.ln()` per call   | 0 (hoisted)          |
| FVG fill check                           | O(n) per candidate    | O(1) per candidate   |
| `kraken_spot_symbol_in_loaded_pairs`     | O(n) + 2 alloc/elem   | O(1) + 1 alloc       |
| `refresh_kraken_position_costs` pairing  | O(n²)                 | O(n)                 |
| `normalized_right_panel_order`           | O(n²) over 8 entries  | O(n)                 |
| MTF cell `trade_overlay`                 | Clone per cell/frame  | `mem::take`/restore  |
| Single chart `trade_overlay`             | Clone per frame       | `mem::take`/restore  |
| `Mt5LiveQuotes` per row uppercase        | 1 alloc per row       | 1 alloc total        |
| `WatchlistQuotes` per row normalize      | 1 alloc per row       | 1 alloc total        |
| `build_trade_overlay` broker dedup       | 1 alloc per pos       | 0 (byte comparison)  |

None of these changes alter chart output or sync semantics; they only
remove redundant work.

## Invariants Going Forward

- **Per-frame paths (`update()`, `draw_chart()`, panel render closures)
  must not allocate proportional to dataset size.** Pre-computed caches,
  reusable buffers, and index-aligned iteration are the tools.
- **Per-message paths (`BrokerMsg::*` handlers) treat the message batch
  as the unit of work.** If row N triggers an allocation that row N+1
  could share, hoist it.
- **When you reach for `Vec::contains` on a hot path, ask whether the
  collection is bounded.** If yes, a bitset or fixed-size array. If no,
  a HashSet seeded once during invalidation.
- **When you reach for `.clone()` to satisfy the borrow checker,
  consider `std::mem::take` + restore first.** Default values are free.

## Consequences

- **Pro:** Chart pan/zoom and MTF grid keep their 60 FPS floor under
  busier broker feeds and deeper indicator stacks.
- **Pro:** Sync scheduling spends fewer allocator cycles per tick when
  the broker universe is large.
- **Con:** Adds a few small per-frame caches that must be invalidated
  on chart changes (already covered by `cached_*_key` patterns in
  `app_runtime.rs`).
- **Neutral:** Bitset and `mem::take` patterns are slightly less
  immediately readable than `Vec::contains` / `Clone`. Comments cite
  the cost they're cutting so the intent survives.

## Consolidated execution-log ADRs (2026-06)

This ADR is the durable home for the terminal's performance & O(1) program. The
following execution-pass ADRs were logs and are now stubs pointing here;
their durable outcomes are the discipline above, and pass-by-pass detail lives
in git history:

- **ADR-060** — Optimization Roadmap (2026-04-08): the initial 10-item audit (GPU
  utilization, UX responsiveness, hot-path perf) that seeded the program.
- **ADR-072** — O(1) hot-path pass + scope-regression fix: eliminated
  O(quotes × charts) nested loops in the broker drain; fixed a scope-filter regression.
- **ADR-074** — Comprehensive performance / UX / memory pass (18-item audit).
- **ADR-075** — Full O(1) algorithmic pass + UX polish: completed ADR-074's deferred items.
- **ADR-076** — Table wiring + O(1) passes (the cleanup that itself noted seven
  execution records were journaling, not durable decisions).
- **ADR-105** — Performance optimization plan & focus areas.

> Note (2026-06): the overnight log still shows multi-second `chrome_panels_ms` /
> `render_after_broker_ms` stalls during full-universe Kraken WS snapshot sweeps —
> this program is **not closed**; the render-thread cost of large sweeps is an open
> item (see the sync-performance investigation tracking this).

### Update (2026-06-21): three render-thread stall sources closed

A fresh log review (multi-second `update_ms` spikes plus a chronic ~300ms-every-60s
tick) traced to three distinct render-thread costs, now fixed:

- **Chart symbol-switch (the 4–5s `render_residual_ms` freezes).** Tab switch,
  new-tab creation, Alt+timeframe hotkeys, and OpenChart/OpenChartTf loaded full
  multi-source history + recomputed every MTF overlay synchronously. They now route
  through the existing paced deferred loader (`queue_chart_reload`) like the MTF grid
  already did. (commit `00194cbb`)
- **Watchlist cold-start (`chrome_panels_ms` ~500ms, pattern A).** The cache-populate
  fallback re-lowercased the whole `detailed_stats` key set (~tens of thousands) per
  missing symbol × timeframe; now lowercased once, lazily. (commit `20855888`)
- **60s scheduler tick (~300ms at idle, pattern B).** `kraken_equity_catalog_symbols`
  (12k normalize+sort) and `alpaca_equity_rotation_symbols` (11k uppercase+dedup+sort)
  were re-materialized every minute; now length-signature-memoized. (commit `182d5cd6`)

Still open on the render thread: the `build_bar_sync_inputs` snapshot
(`detailed_stats.clone()`, O(rows)) at the sync-status cadence — dominated by the
clone, would need Arc-sharing to remove — and the mandatory 60s session-JSON
serialize + `sync_preferences_save` `put_kv` (small at idle, can spike under SQLite
write contention). The full-universe WS snapshot-sweep cost noted above is now also
mitigated separately by the sweep failure-backoff.


## Update (2026-07): Iterative broker/runtime/research/UI O(1) membership and index sweeps

The O(1) program continued with targeted passes replacing remaining linear scans over broker catalogs, account rosters, positions, alerts, watchlists, charts, and sorted research data. All followed the established discipline: retain `Vec` order for UI/serde/rendering where required; build companion `HashMap`/`HashSet` or indices during the natural load/invalidate pass; use binary search where data is sorted; prefer one-pass selection or explicit flags over re-scans.

Key patterns added in recent sweeps (see git history for per-sweep commits such as "perf: cache Kraken balance ownership lookups", "perf: use indexed Kraken depth pair membership", "perf: fold broker primary account lookup scans", "perf: fold roster primary selection scans", "perf: avoid regulatory alert re-scans", "perf: index MTF grid open tabs", "perf: track MQL4 rewrite warnings by flag", "perf: index trading-tools chart lookup", "perf: binary-search Finviz perf windows", "perf: index right-panel section reorder", "perf: cache primary roster entries", "perf: O(1) Kraken position asset tails for quote updates", "perf: O(1) watchlist equity price and Kraken equity pair resolve"):

- **Companion HashSets for repeated membership**:
  - `kraken_balance_assets_by_display`: positive non-cash balance asset tails for "chart or owned" decisions (replaced `kraken_balances.iter().any`).
  - `kraken_position_asset_tails`: asset_id + symbol tails/aliases for live quote position checks (replaced `.values().any(|pos| pos.asset_id.ends_with(...))`).
  - `kraken_pairs_normalized` (extended): used for depth/bookmap stream gating.
  - `kraken_equity_pair_by_base`: base ticker → catalog wsname/name for order resolution (replaced `kraken_pairs.iter().find_map` over full catalog).

- **One-pass map + fallback construction** (Alpaca/Kraken):
  - Account handlers (positions/orders/fills/trades): build id→account map + primary in single pass.
  - Broker roster updates: build `roster_by_id` + connected-primary while iterating.
  - Recent-fills fallback rows now use cached primary roster entries.

- **One-pass selection while building lists**:
  - Regulatory (Reg SHO / trade halt) windows: matching alert row chosen during list construction; render path receives it directly instead of `alerts.iter().find/any` per symbol.
  - Pre-scan removal for Kraken order/position groups in right-panel render (render loop is sufficient).

- **Index / precompute tables**:
  - Watchlist reorder and `latest_watchlist_equity_price_for_symbol`: leverage existing `watchlist_by_bare: HashMap<String, usize>` (O(1) lookup + price check instead of `find_map` over rows).
  - Trading-tools / Bookmap depth/heatmap: chart resolution via `chart_by_bare` (no `charts.iter().find`).
  - Right-panel section drag reorder: section→index table built once (replaces two `.iter().position`).
  - MTF grid open tabs: precompute set of (symbol, tf) cells before fill selection (eliminates repeated `charts.iter().any` inside loops).
  - Command palette recent ordering and cAlgo output-name assignment: name→index maps.

- **Binary search on sorted data**:
  - Finviz performance windows (`window_return`, `ytd_return`): `partition_point` over newest-first date rows instead of repeated `rows.iter().find` prefix scans.

- **Explicit flags instead of repeated scans**:
  - MQL4 transpiler rewrite: `has_ordersend_warning` / `has_orderselect_warning` bools replace `warnings.iter().any(contains("OrderSend"))` etc. during processing and tests.

- **Catalog / pair handling**:
  - Kraken depth stream support helpers now accept normalized pair `HashSet` + empty-catalog fallback (no per-call linear scan).

All changes were accompanied by:
- rustfmt + `cargo check -p typhoon-native` (and engine/transpiler where touched).
- Targeted grep for removed patterns (now zero matches in production paths).
- `git diff --check`, clean worktree hygiene.
- Preservation of ordering, fallback (e.g. empty catalog), and serde behavior.

These build directly on the per-frame invariants in this ADR (hoisting, pre-normalized sets, indices over searches, one-pass where possible). No semantics or output changed. The program remains open; next focus areas include further bounded backpressure for full-universe sync and MTF grid data freshness.
