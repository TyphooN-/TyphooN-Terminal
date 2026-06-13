# ADR-098: Performance & O(1) Optimization Program

> Formerly "Per-Frame O(1) Discipline in Chart and Sync Paths." Retitled 2026-06 when this became the durable home for the terminal's performance program — see *Consolidated execution-log ADRs* at the end.

**Date:** 2026-05-22
**Status:** Accepted
**Related:** ADR-032 (background data + render decoupling),
`native/src/app/technical_analysis.rs`,
`native/src/app/app_runtime.rs`,
`native/src/app/app_runtime_errors.rs`,
`native/src/app/chart_ops.rs`,
`native/src/app/market_data_sync.rs`,
`native/src/app/session_persistence.rs`,
`native/src/app/trade_ops.rs`

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
following round/pass ADRs were execution logs and are now stubs pointing here;
their durable outcomes are the discipline above, and round-by-round detail lives
in git history:

- **ADR-060** — Optimization Roadmap (2026-04-08): the initial 10-item audit (GPU
  utilization, UX responsiveness, hot-path perf) that seeded the program.
- **ADR-072** — O(1) hot-path pass + scope-regression fix: eliminated
  O(quotes × charts) nested loops in the broker drain; fixed a scope-filter regression.
- **ADR-074** — Comprehensive performance / UX / memory pass (18-item audit).
- **ADR-075** — Full O(1) algorithmic pass + UX polish: completed ADR-074's deferred items.
- **ADR-076** — Table wiring + O(1) passes (the cleanup that itself noted seven
  per-round records were execution journaling, not durable decisions).
- **ADR-105** — Performance optimization plan & focus areas.

> Note (2026-06): the overnight log still shows multi-second `chrome_panels_ms` /
> `render_after_broker_ms` stalls during full-universe Kraken WS snapshot sweeps —
> this program is **not closed**; the render-thread cost of large sweeps is an open
> item (see the sync-performance investigation tracking this).
