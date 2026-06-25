# ADR-125: Native Crate Boundary Plan

**Status:** Targets 1 & 2 delivered (`typhoon-research-ui`, `typhoon-chart-ui`); ADR-127 cleared Target 3's protocol cycle. Target 3 still gated by a native-helper closure (~17 fetch/sync/watchlist helpers, compiler-verified) that needs relocating first — extraction attempt reverted, accurate closure documented | **Date:** 2026-06-20 |
**Last updated:** 2026-06-24 (**Target 1 complete** — `typhoon-research-ui` owns `render` +
`window_shell` + `format` + the 55-module `packet` section tree; `command_research_windows`
kept native as command dispatch. **Target 2 complete** — `typhoon-chart-ui` owns `types` +
`indicators` + `drawing` + `models` + `state` (`ChartState`) + the 10k-line `render` tree
(~15.5k lines), acyclic, 2272 workspace tests green; the broker/cache/gpu pipeline stays
native behind extension traits (`ChartDataLoad`/`ChartIndicatorCompute`/`ChartMtfOverlays`/
`ChartSymbolMatch`). **Target 3 (`typhoon-broker-runtime`) is next to evaluate** per Phase 4
— see [Implementation Progress](#implementation-progress))

**Related:** ADR-086 (`typhoon-native` module decomposition), ADR-108
(research module compile-time modularization), ADR-118 (test module
convention), ADR-127 (broker message protocol decoupling — the prerequisite
that unblocks Target 3)

## Context

`typhoon-native` has moved past the original `app.rs` monolith described in
ADR-086, but it is still one large Cargo package. The current workspace is:

- `typhoon-engine`
- `typhoon-native`
- `typhoon-transpiler`

The native package now has clear internal seams:

- `typhoon-native/src/app/floating_windows/`: 117 Rust files, ~61.7k lines.
- `typhoon-native/src/app/command_research_windows/`: 19 Rust files, ~14.3k
  lines.
- `typhoon-native/src/app/symbol_investigation_packet/`: 18 Rust files plus
  the parent `symbol_investigation_packet.rs`, ~13.0k lines combined.
- `typhoon-native/src/app/app_broker_processor/`: 77 Rust files, ~18.5k
  lines.
- Remaining native hotspots include `state.rs` (~3.4k lines),
  `technical_analysis.rs` (~2.1k lines), and several chart/runtime integration
  files.

> The per-tree counts above are the 2026-06-20 baseline. Continued semantic and
> test-module splitting (ADR-118) has since grown several of these trees; current
> measured values are recorded under [Implementation Progress](#implementation-progress).

ADR-108 explicitly deferred a full `typhoon-research` crate split because the
engine research module still had dependency entanglement with engine internals.
That warning still matters: a crate named `typhoon-research` should not become a
native UI dumping ground. Engine-side research compute, providers, storage, and
DTO ownership remain in `typhoon-engine` until their dependency graph is clean
enough for a real engine research crate.

The question now is whether the native application should also split into
multiple crates to improve compile-time locality and code ownership. The answer
is yes, but only if the crate boundaries follow dependency direction and change
cadence rather than file size alone.

## Decision

Split `typhoon-native` only after preparing semantic native UI/runtime boundaries
inside the existing package. The first target is a native research UI crate, not
an engine research crate.

Planned crate direction:

```text
typhoon-native
  ├── depends on typhoon-engine
  ├── depends on typhoon-transpiler
  ├── depends on typhoon-research-ui      # future
  ├── depends on typhoon-chart-ui         # future
  └── depends on typhoon-broker-runtime   # future, name may narrow
```

New crates must not depend on `typhoon-native`. `typhoon-native` remains the
binary/app shell and integration owner. If a proposed extraction requires the
child crate to import `TyphooNApp` or private native internals directly, the
boundary is not ready; first introduce a smaller context/action API inside the
native package.

### Target 1: `typhoon-research-ui`

This is the first crate candidate.

Owns, once prepared:

- research floating-window renderers from `floating_windows/research_*`;
- research command-window rendering and command metadata from
  `command_research_windows/*`;
- symbol investigation packet section formatting from
  `symbol_investigation_packet/*`;
- small research UI view models, labels, table/section formatters, and action
  enums.

Does not own:

- engine research compute;
- research storage/schema/provider fetchers;
- broker/cache hot paths;
- chart camera/rendering;
- the `TyphooNApp` state graph.

The public surface should trend toward functions/types like:

- `render_research_windows(...)`;
- `render_symbol_investigation_packet(...)`;
- `handle_research_command(...) -> ResearchUiAction`;
- compact read-only context structs for selected symbol, cached snapshots,
  visible flags, and command input.

The current `impl TyphooNApp` shape is acceptable for internal modules, but it
is not a good cross-crate API. The migration must first shrink those methods'
dependence on full app state.

### Target 2: `typhoon-chart-ui`

Second crate candidate after research UI.

Owns, once prepared:

- chart rendering helpers;
- chart camera/interaction behavior;
- overlays and drawing tools tightly coupled to egui rendering;
- chart-local state and action DTOs.

This crate should be named `typhoon-chart-ui` while it depends directly on egui.
Reserve `typhoon-chart` for a future renderer-agnostic chart/domain package, if
one ever exists.

### Target 3: `typhoon-broker-runtime` or `typhoon-broker-ui`

Third crate candidate, after the first two boundaries prove the pattern.

Owns, once prepared:

- broker command/result routing that is native-runtime-specific;
- order/account/position reconciliation into native display state;
- Kraken/Alpaca native runtime handlers that are not engine provider logic.

Name choice depends on the final boundary:

- use `typhoon-broker-runtime` if it owns async message loops and
  reconciliation;
- use `typhoon-broker-ui` if it owns only UI projection/render-adjacent broker
  state.

This split is deliberately later because broker handlers often touch channels,
cache state, provider types, runtime handles, and app-level coordination.

### Optional later: `typhoon-native-state`

Only create this if concrete crate cycles force a shared state/model package.
Do not create a broad `typhoon-common` or state junk drawer up front. Shared
crates should appear as a response to a proven dependency cycle, not as a guess.

## Migration Plan

### Phase 0 — Measure and inventory

Before any crate extraction:

1. Capture a current timing baseline with Cargo timings for the relevant edit
   loop, not just a clean build.
2. Record file/module counts for the candidate boundary.
3. Search for all `impl TyphooNApp` methods inside the candidate tree.
4. Inventory imports from `crate::app::*`, direct state-field access, cache
   access, channels, and engine DTOs.
5. Identify which dependencies are truly needed by the candidate crate.

Success means we know whether the boundary is presentation-only, runtime-heavy,
or still coupled to app internals.

### Phase 1 — Prepare the research UI boundary inside `typhoon-native`

Do not create the crate first. First make the boundary honest while everything
still compiles in one package:

1. Add or tighten a parent research-UI module boundary around:
   - `floating_windows/research_*`;
   - `command_research_windows/*`;
   - `symbol_investigation_packet/*`.
2. Move pure formatting helpers and table/section builders behind small free
   functions or narrow inherent methods.
3. Replace broad `TyphooNApp` reads with explicit view/context structs where the
   call site is already obvious.
4. Return actions for mutations where practical instead of mutating unrelated
   app fields deep inside research renderers.
5. Keep existing behavior and command names stable.
6. Verify each slice with `cargo check -p typhoon-native`, relevant native tests,
   `cargo check --workspace`, and `git diff --check`.

Success means research UI code has an identifiable API surface that can be moved
without making half of `TyphooNApp` public.

### Phase 2 — Promote `typhoon-research-ui` to a workspace crate

After Phase 1:

1. Add `typhoon-research-ui` as a workspace member.
2. Move the prepared research UI module tree into the new crate.
3. Add only the dependencies the crate actually uses: likely `egui`, shared
   serde/chrono helpers if needed, and `typhoon-engine` DTOs.
4. Expose the narrow research UI API back to `typhoon-native`.
5. Keep `typhoon-native` as the only binary owner and app-shell owner.
6. Re-run full verification and compare timings against Phase 0.

Success means edits in research UI no longer force the same native crate rebuild
blast radius as chart/broker/app-shell edits, and the dependency direction
remains acyclic.

### Phase 3 — Repeat for chart UI

Use the research UI extraction as the template, but do not start until the first
crate split is stable.

1. Prepare chart-local context/action types inside `typhoon-native`.
2. Move rendering/camera/overlay helpers behind a chart UI boundary.
3. Promote to `typhoon-chart-ui` only once the API does not require importing
   `TyphooNApp`.
4. Verify chart behavior explicitly: pan/zoom, price-axis scale drag,
   crosshair, drawing tools, MTF overlays, live forming bars, and provider source
   labels.

### Phase 4 — Evaluate broker runtime split

Only after research/chart patterns are proven:

1. Inventory async broker handler state and ownership.
2. Separate engine provider logic from native runtime reconciliation.
3. Extract handler families behind explicit command/message/action APIs.
4. Promote to `typhoon-broker-runtime` or `typhoon-broker-ui` only if the crate
   can avoid depending on `typhoon-native`.

## Guardrails

- Do not create `typhoon-research` for native UI code. That name is reserved for
  a future engine/domain research crate if ADR-108's dependency blockers are
  resolved.
- Do not split several crates in one commit. One boundary, one verified slice.
- Do not make broad native state public just to satisfy cross-crate access.
- Do not create `typhoon-common` without a concrete cycle that cannot be solved
  with a narrower API.
- Do not move engine research compute/storage/provider code into native-adjacent
  crates.
- Do not rename semantic modules back to implementation-batch or parity-round
  labels.
- Do not treat clean-build speed as the only metric. The goal is faster and
  safer edit loops for research/chart/broker work.

## Consequences

Positive:

- Gives future work a concrete crate migration path instead of ad hoc file moves.
- Preserves the current working product while reducing risk through internal
  boundary preparation before package extraction.
- Keeps engine research and native research UI separate, avoiding the misleading
  `typhoon-research` dumping-ground problem.
- Creates a repeatable pattern for chart and broker extraction once research UI
  proves the dependency direction.

Tradeoffs:

- The first useful work is not a Cargo.toml change; it is dependency-boundary
  cleanup inside `typhoon-native`.
- Some current `impl TyphooNApp` methods will need context/action refactors
  before they can move across crates cleanly.
- Cross-crate boundaries may improve incremental rebuild locality but can add
  clean-build overhead and API maintenance cost.
- The plan delays broker extraction because its state/runtime coupling is more
  likely to create accidental cycles.

## Implementation Progress

A living log of completed migration slices. Each entry met the verification
standard below.

### Phase 0 — Measure and inventory (2026-06-21)

Research-UI candidate region, as currently measured:

| Tree | Files | Lines | Parent boundary today |
| --- | ---: | ---: | --- |
| `floating_windows/research/` | 96 | ~50.2k | `render_research_ui_windows` (Phase 1, step 1) |
| `command_research_windows/` | 57 + parent | ~14.9k | `command_research_windows.rs` |
| `symbol_investigation_packet/` | 53 + ~2.6k-line parent | ~13.5k | `symbol_investigation_packet.rs` |

Coupling findings for the research floating-window tree (the slice transformed
in Phase 1, step 1), measured before the move:

- All 59 research floating-window renderers were a single `impl TyphooNApp`
  block exposing one `pub(super) fn render_research_*_windows(&mut self, ctx)`.
  They are presentation-shaped (`egui::Window` popups) but every one is an
  `&mut self` method over full app state — not yet a clean cross-crate API.
- They were dispatched from exactly one site (`draw_floating_windows`), nothing
  outside `floating_windows` referenced them by path, and they carried zero
  non-glob `super::` path coupling (only `use super::*`). The other two trees
  share this shape: each is already a single-parent module of `&mut self`
  renderers.
- Conclusion: the boundary is presentation-only in shape but state-coupled in
  fact. Promotion to a `typhoon-research-ui` crate (Phase 2) stays blocked until
  Phase 1 steps 3–4 replace `&mut self` / `self.<field>` access with explicit
  read-only context structs and returned action enums.

### Phase 1, step 1 — Parent boundary for the research floating-window tree (2026-06-21)

The 59 loose `research_*` modules (plus their 8 nested sub-trees) lived directly
under `floating_windows`, interleaved with unrelated windows. They now live under
one `floating_windows/research/` parent module that exposes a single entry point,
`TyphooNApp::render_research_ui_windows`, called once from `draw_floating_windows`.

- Visibility was *tightened*, not widened: each `render_research_*_windows`
  method went from `floating_windows`-scoped `pub(super)` to research-module-
  scoped `pub(super)`. They are now private to the research subtree and reachable
  only through the aggregator — the module's sole public surface.
- Pure module move: the 59 files and 8 sub-trees moved untouched (`git mv`). Only
  `floating_windows/mod.rs` (59 `mod` decls → `mod research;`; the 59 inline
  dispatch calls → one aggregator call) and the new `research/mod.rs` changed. No
  renderer body, behavior, command name, or call order changed.
- `command_research_windows` and `symbol_investigation_packet` already each sit
  behind a single parent-module file, so they were left untouched this slice —
  one boundary per commit, per the guardrails.

Verified: `cargo check -p typhoon-native` (clean), `cargo check --workspace`
(clean), `cargo test -p typhoon-native` (392 passed), `git diff --check` (clean).

### Phase 1, step 2 — Section formatters as free functions (2026-06-21)

First decoupling slice, on the `symbol_investigation_packet` tree. Unlike the egui
floating windows, the packet is already `&self` text-building
(`write_*_sections(&self, p: &mut String, …)`); the coupling that blocks a crate
move is that each section both *reads* app state and *formats* it in one method.

- New `symbol_investigation_packet/format.rs`: free functions over engine DTOs with
  no `TyphooNApp` access — the seed of the formatter layer the crate will own. It
  uses explicit `typhoon_engine` imports rather than the parent `use super::*` glob,
  so it carries no native-app dependency.
- `write_fundamentals_overview(p, &Fundamentals)` extracted from `overview.rs`. The
  section method now only gathers data (the user-position section, the
  `bg.all_fundamentals` lookup) and hands the resolved engine DTO to the pure
  formatter. Behavior-preserving — the formatter reproduces the markdown verbatim;
  two unit tests pin the header + valuation-table output.
- Pattern established: **method gathers from app state → pure free function formats a
  DTO.** This is the repeatable shape for the rest of the packet.
- `capital_valuation_sections` followed: its ten `rx::get_*` → format blocks (WACC,
  Beta, DDM, RelVal, FIGI, HRA, DCF, SVM, Options-chain, IVOL) are now
  `write_wacc(p, &WaccSnapshot)` … `write_ivol(p, &IvolSnapshot)` free functions, and
  the section method collapsed to a flat gather-and-delegate (`if let Ok(Some(x)) =
  rx::get_x(…) { format::write_x(p, &x) }`). The per-snapshot emit guards moved into
  the formatters. Behavior-preserving: all 36 markdown format-string literals are
  byte-identical to the pre-move section (verified by diff), and the compiler checked
  every DTO field access. The options-chain block (put/call ratios, ATM-IV, ATM-window
  table) is pure over the snapshot, so it moved whole.
- `peer_comparison` + the price-behavior / composite-signal / rank-drift section files
  followed in one batch: fourteen more formatters (`write_sharpr` … `write_momf`, plus
  `write_sector_peer_comparison(p, &Fundamentals, &[&Fundamentals])` whose method still
  gathers the sector peers from `all_fundamentals` and hands the slice to the pure
  table builder). Behavior-preserving: all 55 markdown literals across the four
  pre-move sections are present unchanged (verified by diff), with compiler-checked
  field access.

`format.rs` is now a substantial formatter layer (~25 free functions over engine DTOs)
with no `TyphooNApp` access. The remaining packet sections (`distribution_risk`,
`fractal_tail_*`, `momentum_volume_*`, `moving_average_*`, `price_transform_*`, and the
other `price_behavior_*` / `rank_drift_*` / `composite_signal_*` files) are the same
mechanical `rx::get_*` → format shape and migrate the same way.

Verified: `cargo check -p typhoon-native` (clean), `cargo check --workspace` (clean),
`cargo test -p typhoon-native` (395 passed), `git diff --check` (clean).

### Phase 1, step 3 — investigation surfaced a connection bug (2026-06-21)

Scoping the read-only context for the packet sections surfaced a latent correctness
bug, fixed separately (commit `e76c1c99`): the dispatcher held the shared `read_conn`
mutex (`SqliteCache::try_connection` = `read_conn.try_lock`) across its whole
per-symbol block, and the section aggregators it called each re-acquired
`try_connection` — the re-entrant `try_lock` returns `None`, so ~14 analytical section
groups (ownership, capital-valuation, market-behavior, fundamental-risk,
composite-signal, rank-drift, price-behavior, distribution-risk, fractal-tail,
technical-indicator, moving-average, momentum-volume, price-transform, talib) silently
emitted nothing. Only 4 files actually nest (the dispatcher +
`price_behavior_sections` / `rank_drift_sections` / `technical_indicator_sections`);
they now open an *independent* read connection (`open_bg_read_connection`) so
`read_conn` stays free for descendants.

This reframes step 3: the connection the sections need is *already* acquired up the
call stack. The clean end state is to thread that one connection (inside the read-only
context) down to the sections so they stop re-acquiring at all — which removes the
nesting structurally *and* completes the decoupling. The 4-holder fix restores
correctness now; the context threading is the remaining decoupling work, on a working
base.

### Phase 1, step 3 (started) — context threading (2026-06-21)

`SymbolResearchContext { conn: &Connection }` introduced (`context.rs`); the dispatcher
builds it once from its (independent) connection and passes `&ctx` to converted
sections. `capital_valuation_sections` is the first converted: a free function over
`&SymbolResearchContext` that uses `ctx.conn` instead of re-acquiring `read_conn` — no
`TyphooNApp`. `Connection` is re-exported from `typhoon_engine::core::cache` so native
can name it without a direct `rusqlite` dependency. The context is intentionally a
one-field seed that grows as more sections convert (the fundamentals-driven sections
add `all_fundamentals`; visible flags / command input later). Behavior-preserving:
same `rx::get_*` calls and formatters, just sourcing the connection from the context.

Then the 7 leaf-style dispatcher-direct sections followed (one batch): ownership,
market-behavior, fundamental-risk, distribution-risk, fractal-tail, moving-average,
momentum-volume. Each used only `self.cache`, so each is now a free function over
`&SymbolResearchContext` that uses `ctx.conn`, with `use super::*` dropped — no
`TyphooNApp` dependency and no `read_conn` re-acquire. The dispatcher passes `&ctx` to
all 8 converted sections. Behavior-preserving: every markdown literal is byte-identical
to the pre-conversion files (the large line delta is body dedent + rustfmt).

Then the 3 nesting families followed (one batch, 27 files): `price_behavior`,
`rank_drift`, `technical_indicator`. Each family converted atomically — all leaves to
ctx-functions, plus the aggregator, which now threads `ctx` to its leaves
(`super::<leaf>::write_…(ctx, …)`) and uses `ctx.conn` for its own inline rx. The 3
aggregators dropped their `open_bg_read_connection` workaround entirely — they no
longer touch a connection at all. ~35 sections are now free functions over the context;
behavior-preserving (every markdown literal byte-identical across the 27 files).

The 2 pass-through families followed (composite_signal + price_transform, 9 files): the
leaves convert like any other, and the aggregators — which hold no connection, just call
leaves — became trivial `ctx`-threading free functions. The `talib_price_momentum`
family (4 leaves + aggregator) converted the same way.

Finally the dispatcher's own inline rx code moved out: the options-expiration calendar
(EXPCAL) and the ~70 candlestick-pattern + statistical-test blocks — ~2,200 lines that
were inline in the per-symbol loop — are now `write_expiration_calendar` /
`write_candlestick_and_stats` in a new `dispatcher_inline_sections.rs`, free functions
over `&SymbolResearchContext` called in their exact positions (output order unchanged;
all 167 markdown literals preserved, all 73 `rx::get_*` calls relocated to `ctx.conn`).
The dispatcher's per-symbol DB block is now purely a list of `ctx` section calls — it
shrank from ~2,640 to ~410 lines and contains zero inline research code.

**End state reached for the connection block:** the research connection is acquired
exactly once (`open_bg_read_connection`, an independent connection that never contends
with the render thread's `read_conn`) and threaded to every section via the context. No
section re-acquires `read_conn`. The earlier per-aggregator independent-connection
workaround is gone.

The fundamentals-driven section methods followed: `overview` and `peer`. These are
called *outside* the connection block and need app-state slices, not the DB — so rather
than bloat the DB context (`SymbolResearchContext` stays `conn`-only for the DB
sections), they became free functions over **explicit engine slices**:
`write_symbol_investigation_overview_sections(p, sym, fund, &[PositionInfo],
&[PositionInfo])` and `write_symbol_sector_peer_comparison(p, sym, fund,
&[Fundamentals])`. `user_position_section` (only called by `overview`) moved out of
`style_scope.rs` into `overview.rs` as a pure free function over the position slices
(`PositionInfo` is an engine type). The dispatcher does the one `all_fundamentals`
lookup and passes the resolved record + slices. Behavior-preserving (literals
unchanged; `style_scope.rs` is a pure 74-line deletion).

Every named `write_symbol_*` packet section is now a free function over engine types —
no section method remains on `impl TyphooNApp`.

Finally the dispatcher's own inline glue moved out: quarterly-financials + holders,
SEC filings (`bg.sec_filings`), insider summary (`bg.insider_trades`), price/volatility
stats (D1 bar cache), recent news, and the cached-research surfaces. All read engine
types (`SecFiling`, `InsiderTrade`, `NewsArticle`, fundamentals/research DTOs) or the
cache, so each became a free function over `&SqliteCache` / engine slices / the context
in `dispatcher_inline_sections.rs` (or its own module for `recent_news` /
`cached_research`). `cached_research` merged into the main `open_bg` ctx block (it was
adjacent), so it now shares the one connection. Behavior-preserving: all 1,139 markdown
literals across the whole packet tree are unchanged (verified by diff), output order
identical.

**Step 3 is complete for the symbol-investigation packet.** Every section and inline
block is a free function over engine types / `&SqliteCache` / `&SymbolResearchContext` —
no `write_symbol_*` work remains on `impl TyphooNApp`. The dispatcher method
`write_symbol_investigation_sections` shrank from ~2,640 to ~180 lines and is now a pure
orchestrator: it loops symbols, emits the `## SYM` header, does the one `all_fundamentals`
lookup, and passes app-state slices down. Per ADR-125 that orchestrator legitimately
stays in `typhoon-native` (the app shell owns integration); the sections are the
crate-movable surface.

### Phase 1, step 3 — research egui renderer trees, started (2026-06-22)

Began the harder `floating_windows/research` tree (the `&mut self` egui renderers). The
per-window renderers each derived the active chart's research symbol with a byte-identical
~13-line inline block (`self.charts.get(self.active_tab).map(|c| c.symbol.split(':')…)`) —
58 copies. Extracted to one `research_chart_symbol(Option<&str>) -> String` free function
in `research/mod.rs`: pure over the symbol string (no `TyphooNApp`, no native types), so
it is crate-movable, and it is the first shared read-context helper for this tree. The 58
call sites now pass `self.charts.get(self.active_tab).map(|c| c.symbol.as_str())`.
Behavior-preserving (logic unchanged; 2 unit tests pin the `source:symbol:timeframe`
extraction + `AAPL` fallback).

Then the display-half extraction began (the egui analog of the packet's `format.rs`).
New `research/render.rs`: pure snapshot-display renderers, free functions over
`(&mut egui::Ui, &Snapshot)` with no `TyphooNApp` — crate-movable since
`typhoon-research-ui` may depend on egui. First file done as proof of concept:
`research_ohlc_price_transforms` (AVGPRICE / MEDPRICE / TYPPRICE / WCLPRICE / VARIANCE)
— each window's display body (the label + summary `egui::Grid`, ~90 lines) moved to a
`render::render_<x>_snapshot` free function; the renderer keeps the header/input/action
half and calls it. Behavior-preserving: all 56 string literals present unchanged
(verified by diff), all 5 display Grids relocated (renderer 5 → 0, `render.rs` 0 → 5).
Done via an indent-based guarded transform (format strings contain `{}`, so
brace-counting is unsafe).

The display-half extraction then ran to completion across the tree via two
self-discovering, guarded transforms into the same `render.rs`:

- **Snapshot pattern** (`render_<x>_snapshot(ui, &Snapshot)`): every window with a
  separator-anchored `let snap = &self.<x>_snapshot;` and a *pure* display body. 247
  functions extracted (the type lookup reads multi-line `state.rs` declarations; a
  collision guard falls back to a type-based name; impure bodies are skipped).
- **Data-table pattern** (`render_<field>(ui, &[Elem])`): the fundamental-data windows
  that render a `Vec` field (dividends, earnings, ratings, splits, holders, …). The
  body is extracted by passing the slice and substituting `self.<field>` → `rows`
  (join-then-substitute for multi-line method chains; `\b` keeps `self.<field>_symbol`
  safe). 12 functions.

`render.rs` now holds **259 pure display functions** over engine DTOs / slices, with the
common color constants auto-imported from actual (string-stripped) usage. Each batch was
verified by full-tree literal-preservation diff + `cargo check`/`test`. Crucially, the
external auto-formatter kept re-touching the previous commit's `research_chart_symbol`
call across ~45 files; each batch filtered the commit to only the files that actually
gained a `render::` call (per the lint caveat — no blanket-format churn).

What deliberately remains inline (≈19 grids in 7 files): **multi-field summary cards**
(a window's header block that reads several `self.<x>` fields into one fixed-layout grid,
not a single snapshot/slice) and **interactive filtered tables** (display bodies that read
a `self.<x>_filter`). Neither fits a mechanical pure-display transform — they need either
a small per-window context struct or belong to the input/action half. They are left for
that phase rather than forced through a transform that doesn't fit.

### Phase 1, step 3 — input/action half, started (2026-06-22)

Began the `&mut self` input/action decoupling with a **generic window shell**.
The "compute snapshot" windows share one interaction shell — symbol input + Use Chart /
Load Cached / Compute buttons + loading indicator, then the snapshot display.
`window_shell::render_compute_window<S, Cmd>` now owns it: generic over the snapshot
type `S` and the action type `Cmd`, so it has **no `TyphooNApp` or `BrokerCmd`
coupling** (depends only on egui + the engine cache types — crate-movable). The
per-window variation arrives as closures (`load_cached`, `make_cmd`, `render_snapshot`),
the window's own state is threaded as `&mut` field refs, and the Compute action is
**returned** (`Option<Cmd>`) instead of sent inline — the caller dispatches it.

Proof of concept: `research_ohlc_price_transforms` (AVGPRICE / MEDPRICE / TYPPRICE /
WCLPRICE / VARIANCE). The renderer dropped from 254 to 124 lines and now holds five
declarative window specs + closures with zero inline `egui::Window`; the borrow checker
accepts the call (disjoint `&mut self.<x>_*` field borrows + `self.cache.as_deref()`).
Behavior-preserving by construction (the shell replicates the exact button logic).

The shell then scaled across the tree via a strict template matcher, widened over several
batches to tolerate pure-formatting variation (method-chain wrapping, multi-line titles
with trailing commas, wrapped getter/render calls) while still rejecting any logic
difference: **219 compute windows now route through `window_shell::render_compute_window`**,
each a declarative spec + closures instead of inline `egui::Window`/buttons/`broker_tx`.
Only blocks whose normalized text matches the canonical template exactly are transformed;
every window title is preserved (verified by diff per batch). `render.rs` holds the 259
display functions; the shell owns the interaction and returns the action.

**Multi-field commands handled (10 windows).** The compute windows whose Compute body
runs a bespoke pre-read producing a multi-field `BrokerCmd` (FLOW, DDM, SECTR, RVOL,
SHRT, SVM, IVOL, VOLE, PTD, LIQ) route through the same shell with an *identity*
`make_cmd` (`|symbol| symbol`): the shell returns the symbol on Compute click, and the
pre-read + multi-field send move verbatim into the `if let Some(sym)` body — where `self`
is free (the shell's `&mut` borrows are released) and the pre-read runs only on click, so
behavior is exactly preserved. **229 compute windows now route through the shell**; all
BrokerCmd sends + window titles verified preserved.

**`.max_size(...)` handled (9 windows).** `ComputeWindow` gained an optional `max_size`
field (applied to the egui builder when `Some`); `max_size: None` was mechanically added
to the 229 existing call sites, and the transforms now capture `.max_size([w,h])`.
**238 compute windows route through the shell** — every single-field, multi-field, and
max-size canonical compute window.

**Extra-control windows handled (3 of 4).** `render_compute_window_ext` adds an
`extra_controls` closure rendered in the button row (between Use Chart and Load Cached);
`render_compute_window` is now a thin wrapper passing a no-op, so the 238 callers are
untouched. INSSTRK / MNGR (`window_days` DragValue) and COR (DragValue + peer-JSON
pre-read) route through it — **241 compute windows total**. DCF stays inline: it lays its
3 assumption DragValues + Compute in a *separate* second `ui.horizontal` row, which the
shell's single-row layout would visibly change — a genuine exception, not forced.

### Bespoke-tail analysis: the "Fetch" windows stay native (by ADR design)

The 27 "Fetch"-button windows pull fundamental data from an external API. Their *display*
is already crate-extracted (15 use the `render::` data-table functions from the display
pass). But their *header* is integration glue — API-key-gated buttons that send broker
fetches (`BrokerCmd::FetchX { symbol, fmp_key / finnhub_key }`). Per ADR Target 1 the
crate owns view models / table formatters, **not** broker/provider fetch logic, which
"`typhoon-native` remains the … integration owner." So the fetch headers correctly stay
in native (the same call as the `command_research_windows` handlers): routing them
through a "crate-movable" shell would be deduplication of integration code, not a crate
boundary. The crate-relevant extraction for these windows — the displays — is done.

**Net:** every research view (display) and the compute-window interaction layer are
decoupled into crate-movable free functions; what remains inline is integration glue the
ADR explicitly keeps in `typhoon-native`. The research-UI boundary is ready for the
Phase 2 crate-extraction decision.

### Phase 1, step 3 — `command_research_windows`, started (2026-06-23)

Began decoupling the *other* research tree — the command handlers that map command
strings (e.g. `"AVGPRICE"`) to actions (set `show_*` flags, send broker fetches). Every
handler arm derived the active-chart symbol with a byte-identical inline block — **429
copies across 52 files** — now one `command_chart_symbol(Option<&str>) -> String` free
function (empty default, matching the originals; pure over the symbol string, no
`TyphooNApp`). Behavior-preserving; the dispatch (`handle_research_window_command`) and
command names are unchanged.

### Phase 2, step 1 — `typhoon-research-ui` crate extracted (2026-06-23)

The first crate is real. New workspace member `typhoon-research-ui` now owns the
research-UI view + interaction layer:

- `render` — the 259 pure snapshot-display functions (`git mv` from
  `floating_windows/research/render.rs`).
- `window_shell` — the compute-window interaction shell (`render_compute_window[_ext]`,
  `ComputeWindow`).
- `theme` — the six color constants the modules used, mirrored from `app::common` so the
  crate needs no native import (a shared theme crate stays deferred per the guardrails).

Dependencies: `egui`, `chrono`, `typhoon-engine` only. `cargo tree` confirms the crate
does **not** depend on `typhoon-native` — the ADR's hard requirement; the direction is
acyclic. The moved functions became `pub` (were `pub(super)`).

Zero call-site churn: `typhoon-native` re-exports the modules from
`floating_windows/research/mod.rs` (`use typhoon_research_ui::{render, window_shell};`),
so every renderer call (`super::render::…`, `window_shell::…`) resolves unchanged.
`typhoon-native` stays the only binary/app shell — dispatchers, command handlers, window
state, and Fetch/integration logic remain native, calling into the crate.

Verified: `cargo check -p typhoon-research-ui` (clean), `cargo check --workspace` (clean,
0 warnings), `cargo test --workspace` (2272 passed: engine 1641 + native 400 +
transpiler 231), `git diff --check` (clean).

### Phase 2, step 2 — packet `format` module moved (2026-06-23)

`symbol_investigation_packet/format.rs` (the ~25 packet text formatters + their 3 unit
tests) moved into `typhoon-research-ui` as the `format` module — pure over
`typhoon_engine` DTOs (no egui, no `TyphooNApp`), the same clean move as
`render`/`window_shell`. Functions became `pub`; the crate stays acyclic. `typhoon-native`
re-exports it from `symbol_investigation_packet.rs` (`use typhoon_research_ui::format;`),
so the section call sites (`format::…`, `super::format::…`) resolve unchanged. Verified:
workspace clean (0 warnings), 2272 tests (the 3 format tests now run in the crate).

The crate now owns: `render` (259 display fns), `window_shell` (compute shell), `format`
(packet formatters), `theme`.

### Phase 2, step 3 — packet section tree moved (2026-06-23)

The **packet section tree** — all 55 `symbol_investigation_packet/*` section modules
(`capital_valuation_sections`, `composite_signal_*`, `distribution_risk_sections`,
`talib_*`, `dispatcher_inline_sections`, …, plus `context`) — moved into
`typhoon-research-ui` as the `packet` module. These are `write_symbol_*_sections(&SymbolResearchContext, …)`
free functions over `ctx.conn` (engine `Connection`) + `rx::get_*` (engine) + the crate's
`format` — no `TyphooNApp`, no egui — so they moved the same way as `render`/`format`.
`SymbolResearchContext` (the read-only `{ conn: &Connection }` thread) moved with them as
`packet::context`. Functions/structs became `pub`; intra-tree paths rewrote
`super::format` → `crate::format`; the crate stays acyclic (`cargo tree` shows no
`typhoon-native`).

What stays in native is exactly the **dispatcher**: `write_symbol_investigation_sections`
(`symbol_investigation_packet.rs`) gathers app state (`self.bg`, `self.live_positions`,
`self.cache`), opens the single `open_bg_read_connection()` (ADR-125 step 3), builds the
`SymbolResearchContext`, and calls the moved section functions — re-exported via
`use typhoon_research_ui::packet::*;`, so every call site (`capital_valuation_sections::write_…`)
resolves unchanged. Verified: crate standalone (0 err / 0 warn), native (0 err / 0 warn),
workspace 2272 tests pass / 0 fail (the 3 `format` tests run in the crate).

The crate now owns: `render` (259 display fns), `window_shell` (compute shell), `format`
(packet formatters), `packet` (55-module section tree + `SymbolResearchContext`), `theme`.

### Phase 2, step 4 — `command_research_windows` boundary assessed: stays native (2026-06-23)

With `render`/`window_shell`/`format`/`packet` extracted, the last item in Target 1's
"owns, once prepared" list is `command_research_windows/*` (57 files, ~10.6k lines). It was
inventoried against the crate boundary and **stays in `typhoon-native` as integration
glue** — it is not crate-movable as written:

- **No rendering lives there.** `grep` finds **0** `egui::` / `ui.` / `.show()` calls in
  the tree — the actual command-window rendering is the `render_*_snapshot` functions,
  which already moved to the crate's `render` module. What remains is pure command→state
  dispatch.
- **It mutates the `TyphooNApp` state graph directly** — **1295 distinct** `self.show_*_win`
  / `self.*_win_symbol` / `self.*_win_snapshot` field writes across ~429 command arms, plus
  `self.broker_tx` sends and `self.fmp_key`/`finnhub_key` reads. The ADR explicitly excludes
  the `TyphooNApp` state graph from the crate, and the guardrail says a boundary that needs
  the child to import `TyphooNApp` is "not ready."
- Moving it would require either a ~429-variant `ResearchUiAction` enum or relocating the
  per-window `{show, symbol, snapshot}` state off `TyphooNApp` into the crate — the latter
  is the "later optional step … not a prerequisite" already noted below, and would churn the
  entire `floating_windows` render dispatch (which reads those same fields). Neither is
  warranted: these handlers are the native app-shell's command dispatch, the same role as
  the `write_symbol_investigation_sections` dispatcher that also stays native.

**Target 1 (`typhoon-research-ui`) is therefore complete.** The crate owns the entire
movable research-UI presentation layer — `render` (259 display fns), `window_shell`,
`format`, `packet` (55-module section tree + `SymbolResearchContext`), `theme` — and native
retains only integration glue: the two dispatchers (`render_research_ui_windows`,
`write_symbol_investigation_sections`), the `command_research_windows` command handlers, and
the Fetch/API-key headers. Dependency direction is acyclic and verified.

### Target 2 — `typhoon-chart-ui`: Phase 0 inventory (2026-06-24)

The research-UI split is stable (compiles, 2272 tests green), so per Phase 3 the chart-UI
boundary is cleared to begin. Inventory of the candidate region:

| Tree / file | Lines | `impl TyphooNApp` | `&self` (other types) | `TyphooNApp` refs | Nature |
| --- | ---: | ---: | ---: | ---: | --- |
| `technical_analysis/` (23 files) | 7910 | 0 | 0 | 0 | render/overlays/drawing — free fns |
| `technical_analysis.rs` | 2112 | 0 | 0 | 0 | render entry + helpers — free fns |
| `technical_indicators.rs` | 3089 | 0 | 0 | 0 | pure indicator math (no egui/engine) |
| `chart.rs` | 852 | 0 | 9 | 1 (doc comment) | `ChartState` def + view/quote impls |
| `chart/` (8 files) | 5659 | 0 | 181 | 0 | chart-local types + behavior + data |
| `chart_ops.rs` | 1381 | **2** | 32 | — | **native glue** (TyphooNApp ops) |

**The rendering layer is already decoupled.** `technical_analysis/` references `TyphooNApp`
**zero** times; renderers are `pub(crate) fn draw_*(painter: &egui::Painter, chart:
&ChartState, bars: &[Bar], flags: &IndicatorFlags, …)` free functions over egui (1302 refs)
+ chart-local data types, read-only on `&ChartState` (18 reads, **0** `&mut`). Unlike
research-UI this needs almost no Phase-1 decoupling prep — the free-function form is already
there.

**The chart-local types are `TyphooNApp`-free and crate-shaped:** `ChartState` (chart.rs),
`ChartCamera` / `IndicatorFlags` / color consts (`chart/models.rs`), and `Bar` / `ChartType`
/ `Timeframe` (`types.rs`, alongside many native-only types that stay). These are exactly the
"chart-local state and action DTOs" the ADR assigns to Target 2 and should move to the crate;
`TyphooNApp.charts: Vec<ChartState>` then holds the crate type (native → crate, acyclic).

**The orphan-rule constraint that shapes the slice plan.** `ChartState` has inherent-`impl`
blocks in **7 files** — chart.rs (quote/forming-bar), `camera_controls`, `auto_fibonacci`,
`mtf_overlays` (chart-local behavior, movable) **and** `load_cache`, `indicator_compute`,
`equity_merge`, `market_data_helpers` (the data/merge pipeline). Rust requires every inherent
`impl ChartState` to live in the crate that defines `ChartState`. So moving `ChartState`
forces a decision per data-pipeline file:

- `indicator_compute.rs` (1613l, pure math via `technical_indicators`), `market_data_helpers.rs`
  (244l, engine only), `load_cache.rs` (1325l, engine cache + chrono) — engine-reachable, so
  their `impl ChartState` can move **with** `ChartState` into the crate (crate → engine is
  allowed).
- `equity_merge.rs` (1338l) **cannot move**: it uses `OrderBroker` (defined in
  `state/broker_messages.rs`, native) and owns the `MERGE_PRIMARY_BROKER` atomic +
  `chart_equity_source_rank_for` (ADR-126). A crate dep on it would be a `crate → native`
  cycle. Its `impl ChartState` methods must be **converted to native free functions** over
  `&mut ChartState` (the same "glue stays native" pattern as the research dispatchers), since
  they can't remain inherent impls once `ChartState` is in the crate.

`chart_ops.rs` (2 `impl TyphooNApp` blocks) is native dispatch glue and stays, calling into
the crate — the chart analogue of `command_research_windows`.

**Slice plan (dependency order, one verified commit each):**
1. Create the `typhoon-chart-ui` crate skeleton + move the leaf data types `Bar` / `ChartType`
   / `Timeframe` out of `types.rs`; native re-exports them so call sites are unchanged.
2. Move `technical_indicators.rs` (pure math) into the crate.
3. Convert `equity_merge.rs`'s `impl ChartState` methods to native free functions over
   `&mut ChartState` (un-block the orphan rule before `ChartState` moves).
4. Move `ChartState` + `ChartCamera` / `IndicatorFlags` / consts + the movable behavior/data
   impls (`chart.rs`, `models`, `camera_controls`, `auto_fibonacci`, `mtf_overlays`,
   `load_cache`, `indicator_compute`, `market_data_helpers`) into the crate.
5. Move the rendering layer (`technical_analysis/`, `technical_analysis.rs`) into the crate.
6. Leave `chart_ops.rs` + `equity_merge.rs` free fns in native as integration glue; verify
   pan/zoom, axis drag, crosshair, drawing tools, MTF overlays, live forming bars, source
   labels (Phase 3 acceptance checks).

### Target 2 — `typhoon-chart-ui` COMPLETE (2026-06-24)

Done in seven verified, individually-committed slices. The crate now owns the chart-UI
presentation + state layer (**~15.5k lines**): `types` (`Bar`/`ChartType`/`Timeframe` +
the `bare_symbol_from_key` cache-key primitive), `indicators` (3.1k-line indicator math),
`drawing` (egui drawing tools), `models` (`ChartCamera`/`IndicatorFlags` + the full color
palette), `state` (`ChartState` + camera/auto-fib behavior), and `render` (the 10k-line
egui rendering tree). Deps: `typhoon-engine` + `egui` + `chrono` — acyclic, never
`typhoon-native`; full workspace 2272 tests green at every slice.

What stays in `typhoon-native` (~6.9k lines of chart glue): `chart_ops.rs` (the
`impl TyphooNApp` chart dispatch — the chart analogue of `command_research_windows`),
`chart_sources.rs`, `chart.rs` (now a thin 166-line glue module: `mod` wiring + the
`ChartState` re-export), and the `chart/` data pipeline (`equity_merge`, `load_cache`,
`indicator_compute`, `mtf_overlays`, `market_data_helpers`).

**The orphan-rule pattern that made it work.** `ChartState` had to move (the renderers
read 130 distinct fields, so a borrow-view was not viable), but several of its inherent
`impl` blocks call native-only glue — the broker-coupled equity-merge (`OrderBroker`, the
ADR-126 `MERGE_PRIMARY_BROKER` atomic), the `gpu_compute` (wgpu, 6.3k lines) pipeline, and
the market-data cache-key normalizer. Since Rust forbids an inherent `impl ChartState` in
native once `ChartState` is foreign, those blocks became **native extension traits**
implemented for the (now-crate) type — `ChartDataLoad`, `ChartIndicatorCompute`,
`ChartMtfOverlays`, `ChartSymbolMatch` — with byte-identical bodies and unchanged
method-syntax call sites (each trait re-exported through the `chart` → app glob). This kept
the ADR-126-sensitive merge logic untouched and native, while the data type + pure behavior
moved. The plan's "convert `equity_merge` impls to free fns" step (3) turned out unnecessary
— `equity_merge.rs` was already free functions, so it simply stays native as-is.

Notable boundary calls: `gpu_compute` (wgpu) is too heavy/broad to pull into a UI crate, so
indicator compute stays native behind a trait rather than moving; the base chart palette was
split out of native `app::common` (UI-button colors + `nav_*` helpers stay native); the pure
`bare_symbol_from_key` parser moved to `types` since both renderers and native need it.

### Target 3 — `typhoon-broker-runtime`: evaluated in depth, DEFERRED (2026-06-24)

Phase 4 says *evaluate* the broker split "only after research/chart patterns are proven"
and promote "only if the crate can avoid depending on `typhoon-native`." Candidate region
`app/app_broker_processor/` (77 files, ~18.6k lines) + the protocol in
`app/state/broker_messages.rs` (3.7k lines). The evaluation was taken further than a surface
inventory; the findings below supersede an earlier draft that leaned on the broker rip-out as
the blocker.

**The rip-out is NOT the blocker — it has already landed on `master`.** `typhoon-engine/src/broker/`
is Kraken + Alpaca only; darwin/mt5/tastytrade are gone from the broker code (the `deprecated/*`
branches are restore points, not pending work). So the surface is stable today.

**The processor task itself is well-decoupled** — encouraging but not sufficient.
`spawn_broker_message_processor` is a self-contained async task taking explicit params
(`BrokerCmd`/`BrokerMsg` channels, an `Arc<RwLock<Option<Arc<SqliteCache>>>>`, a tokio
`Handle`, an importing flag); it holds **no `TyphooNApp`** (the only 2 refs are a trivial
static `TyphooNApp::default_gemini_cli_model()` call). The 19 broker-handler files have **0
`impl TyphooNApp`** and **0 `self.` field access**.

**The real, structural blocker is the message protocol's entanglement with the native state
graph — a cycle, not a sequencing issue:**

- `BrokerCmd`/`BrokerMsg` (the app's message bus) are referenced in **220 / 97 files** — fine
  on its own (a re-export keeps call sites stable, as proven 4× in Targets 1–2), **but**
  `BrokerMsg`'s payload variants carry native state-graph types — `WatchlistRow`
  (`state/watchlist`), the `gpu_compute` `Indicator`, and friends — and `broker_messages.rs`
  itself is written over `use super::*` (the native `state` module). Moving the protocol into
  a crate would drag native UI/gpu state across the boundary, i.e. a `crate → typhoon-native`
  cycle.
- `research_compute/` (58 files / **13.1k lines** — 70% of the region) is **not a separable
  engine-compute island**: it carries **1525 `BrokerMsg`/`BrokerCmd` references** and
  `use super::*` in all 59 files. It is woven into the broker message flow (compute-on-command,
  emit-results/progress-as-`BrokerMsg`) and also belongs in *engine* per ADR-108, not a
  broker-UI crate. It cannot simply be hoisted out as a prerequisite.

So the broker subsystem is a single tightly-coupled protocol ↔ compute ↔ native-state fabric
— exactly the "channels, cache state, provider types, runtime handles, app-level coordination"
the ADR flagged when it made broker extraction "deliberately later" with the highest
accidental-cycle risk.

**Verdict: defer Target 3.** The precondition is no longer "wait for the rip-out" (done) but a
genuine **protocol-decoupling project**: lift the `BrokerMsg` payloads off the native state
graph (so the protocol depends only on engine/std), and untangle `research_compute` from the
message flow toward its ADR-108 engine home. That is its own boundary-prep effort — larger
than, and upstream of, a clean crate cut — and is out of scope for ADR-125's "one clean
boundary at a time."

That prerequisite was scoped in **[ADR-127](127-broker-message-protocol-decoupling.md)** and is
now **IMPLEMENTED**. A deeper measurement done for ADR-127 shrank the protocol's native
entanglement to a single relocatable DTO (`WatchlistRow`), and the three-phase decoupling
landed (full workspace 2272 tests green throughout): `WatchlistRow` → engine; `broker_messages.rs`
made engine/std-only; and the protocol moved to **`typhoon_engine::broker::protocol`** with a
4-line native re-export shim keeping the ~220/97 call sites unchanged. `cargo tree` confirms the
engine gained no native dependency.

**So the protocol↔state cycle no longer exists** (the structural blocker is gone), and the
post-ADR-127 framing supersedes the earlier "split `research_compute` to engine first" idea —
`research_compute` references engine types as **bare names through `use super::*`** (431 engine
refs, 0 `TyphooNApp`, 0 egui) and is structurally one of the broker handlers (`BrokerCmd` →
compute → emit `BrokerMsg`), so it should **stay with its siblings**, not split to engine
(which also keeps engine free of command-orchestration concerns, the ADR-108 caution).

**Correction (an attempted extraction surfaced the real closure).** A first crate-extraction
attempt was made on the premise — from a grep-based scan — that the whole `app_broker_processor/`
had only *two* trivial native couplings (`ALPACA_DEFAULT_HISTORICAL_RPM`,
`TyphooNApp::default_gemini_cli_model()`; both removed in the Target-3-prep commit). **That scan
was incomplete.** Once those two were gone and the tree was moved to a crate, the compiler
found **~17 more native symbols** the processor calls that the scan's name-patterns missed — a
real dependency closure, not constants:

- **fetch-task runners** — `run_alpaca_fetch_task`, `run_alpaca_batch_fetch_task`,
  `run_kraken_fetch_task`, `run_kraken_futures_fetch_task` (`app/broker_fetch.rs`, ~786 l);
- **Yahoo fallback fetch** — `fetch_yahoo_chart_bars`, `store_fallback_bars`,
  `yahoo_chart_provider_no_data_error` (`app/fallback_bars.rs`, ~355 l);
- **sync permits** — `KRAKEN_PUBLIC_FETCH_PERMITS`, `KRAKEN_EQUITIES_FETCH_PERMITS`
  (`app/sync_config.rs`);
- **watchlist builders** — `watchlist_row_from_raw_bars`, `empty_watchlist_row`
  (`app/state/watchlist.rs`); watchlist/yahoo predicates in `app/state/models.rs`;
- **misc** — `chart_source_cache_keys` (`app/chart_sources.rs`),
  `normalize_kraken_equity_symbol_list` (`app/market_data_sync.rs`),
  `extract_news_symbols_from_market_data_cache` (`app/chart/equity_merge.rs`).

The extraction was reverted to a clean state (the prep commit stands). Most of the closure is
engine-adjacent and movable (`broker_fetch`/`fallback_bars`/`sync_config` are `TyphooNApp`-free),
but it has **placement problems**, not just mechanics: `chart_source_cache_keys` is cache/chart
logic (not broker-runtime), one helper lives in `market_data_sync.rs` which *has* `impl
TyphooNApp`, and several of these files are shared with the native sync subsystem. This is
exactly the "broker handlers touch fetch/cache/app-level coordination" entanglement the ADR
flagged as the reason broker extraction is highest-risk.

**Revised Target 3 plan: it needs a prerequisite decoupling effort, not a one-shot move.**
First relocate the helper closure to its right homes — the pure fetch/Yahoo/permit helpers to
a shared lower layer (engine, or the crate once it exists), the cache-key helper to engine,
and extract the one helper out of the `impl TyphooNApp` file — re-exporting from native so
callers are stable (the proven pattern). *Then* the `use super::*` → engine-prelude move of the
77-file tree is the clean mechanical cut, with a single `spawn_broker_message_processor` seam.
Scope the closure with the **compiler**, not a grep, before estimating.

With Targets 1 & 2 delivered and ADR-127 removing the protocol cycle, Target 3 is unblocked at
the protocol layer but still gated by this native-helper closure — a bounded but real
decoupling slice that must land first.

**Closure reduction started (2026-06-25).** The first bounded slice moved the two broker
runtime permit constants (`KRAKEN_PUBLIC_FETCH_PERMITS`, `KRAKEN_EQUITIES_FETCH_PERMITS`) to
`typhoon_engine::broker::sync_config` with a native compatibility re-export, and moved the
chart source cache-key generator to `typhoon_chart_ui::cache_keys` with a native shim. This
removes the broker processor's direct dependency on native `sync_config` and native
`chart_sources` while preserving call sites. The next slice moved the cache-backed
watchlist row builders (`watchlist_row_from_raw_bars`, `empty_watchlist_row`) next to
`WatchlistRow` in `typhoon_engine::core::watchlist`, again leaving a native re-export shim.
The next chart-cache slice moved `normalize_kraken_equity_symbol_list` to
`typhoon_chart_ui::cache_keys` and updated the broker processor's fundamentals command to call
the crate helper directly. The next watchlist slice moved the cache-fallback source selector
and Yahoo extended-quote freshness predicates to `typhoon_engine::core::watchlist`; the broker
watchlist handler now calls those engine helpers directly. The remaining closure is the
fetch/Yahoo task runners. Market-data cache news-symbol extraction moved to
`typhoon_engine::core::market_data_symbols`, with native chart wrappers retained for chart
tests/callers and the broker news handler calling the engine helper directly. The watchlist
broker handler now also calls engine/chart-ui cache helpers directly (`empty_watchlist_row`,
`watchlist_row_from_raw_bars`, `chart_source_cache_keys`) instead of reaching through native
shims. The final closure slice moved the fetch-task runners to
`typhoon_engine::broker::bar_fetch` and Yahoo fallback fetch/store helpers to
`typhoon_engine::core::fallback_bars`; native now calls those engine modules directly and the
old native `broker_fetch.rs` / `fallback_bars.rs` files are gone. This closes the helper list
that blocked a future standalone broker-runtime crate cut; the remaining Target 3 work is the
mechanical crate extraction / processor prelude seam, not more native helper migration. The next
Target-3 seam slice added `app_broker_processor/prelude.rs` and repointed every direct
broker-processor child module from `use super::*` to `use super::prelude::*`; nested research
compute children keep their local parent imports for now. This centralizes the native-facing
surface in one file (`pub(super) use crate::app::*`) instead of routing through the parent
module's glob import, so the future broker-runtime crate extraction can turn the prelude into an
explicit dependency boundary instead of auditing 19 top-level child modules independently. The
parent `app_broker_processor.rs` now also imports `crate::app::*` directly rather than `super::*`,
removing the last broker-processor dependency on relative native-parent glob routing.
The next nested seam added `app_broker_processor/research_compute/prelude.rs` and repointed the
first-level research compute handlers (`analytics`, `breakout`, `risk`, `squeeze`,
`technical_indicators`, `valuation`, `volatility`) from `use super::*` to
`use super::prelude::*`; the prelude forwards the app-surface prelude plus the local sibling
modules still referenced across handlers. Risk/technical grandchild modules remain on their
domain-parent imports until those subtrees get their own narrower seams.
The next subtree seam added `app_broker_processor/research_compute/technical_indicators/prelude.rs`
and repointed all direct technical-indicator compute handlers from `use super::*` to
`use super::prelude::*`; that prelude forwards the research-compute prelude and leaves the
technical router as the only local sibling-dispatch owner.
The next risk subtree seam added `app_broker_processor/research_compute/risk/prelude.rs` and
repointed all direct risk compute handlers from `use super::*` to `use super::prelude::*`; that
prelude forwards the research-compute prelude and leaves the risk router as the only local
sibling-dispatch owner.
The next crate-shell slice added the workspace member `typhoon-broker-runtime` with a deliberately
small lower-layer prelude (`typhoon_engine::{broker, core}` and `typhoon_chart_ui::cache_keys`).
No native processor files have been moved into it yet; the point of this slice is to make the
future physical move target compile independently before threading the native spawn seam through
the new crate.
The next runtime-resource seam moved broker-loop permit/client construction into
`typhoon_broker_runtime::resources::BrokerRuntimeResources` and wired native to depend on the new
crate for those lower-layer resources. Native still owns the app-shell spawn call and UI/cache
state, but Kraken/Yahoo/Alpaca runtime resource setup now compiles inside the crate that will own
the processor after the physical move.
The next helper-closure slice moved regular US-equities market-clock status formatting into
`typhoon_engine::core::market_session::us_equities_session_status_at` and routed the broker
`GetMarketClock` command through the engine helper. The native copy remains test-only while the
runtime path no longer reaches into `app_runtime_support`.
The next helper-closure slice moved Kraken crypto symbol-search suggestion construction into
`typhoon_engine::core::symbol_search::append_kraken_crypto_symbol_suggestions`, removing the
broker processor's local `KRAKEN_CRYPTO_BASES` table while preserving cross-source de-duplication
through the caller-owned seen set.
The first physical processor-child move relocated `misc_commands` into
`typhoon_broker_runtime::misc_commands`; native now imports that runtime module for
`MarkUnresolvable`, `GetQuote`, and `GetMarketClock` routing instead of compiling a local child
module for those stateless command arms.
The next physical processor-child move relocated `symbol_search` into
`typhoon_broker_runtime::symbol_search`; native now imports that runtime module for
`SearchSymbols` routing instead of compiling a local child module.
The next physical processor-child move relocated `external_feeds` into
`typhoon_broker_runtime::external_feeds`; native now imports that runtime module for FRED,
economic-calendar, Congress, Fear & Greed, Reddit WSB, and crypto-top-50 external feed routing.
The next physical processor-child move relocated `alpaca_account_data` into
`typhoon_broker_runtime::alpaca_account_data`; native now imports that runtime module for account,
position, order-list, activity, top-mover, and asset-list routing.
The next physical processor-child move relocated `alpaca_order_ops` into
`typhoon_broker_runtime::alpaca_order_ops`; native now imports that runtime module for Alpaca close,
market, limit, stop, bracket, OCO, modify, trailing-stop, and sync-exit order routing.
The next physical processor-child move relocated `connection_commands` into
`typhoon_broker_runtime::connection_commands`; native now imports that runtime module for Alpaca
connect/sync configuration and Kraken REST/WebSocket connection routing while still passing the
mutable broker handles from the native-owned spawn loop.
The next physical processor-child move relocated `storage` into
`typhoon_broker_runtime::storage`; native now imports that runtime module for compaction and
unusual-volume scan routing while still passing the shared cache and importing flag explicitly.
The next physical processor-child move relocated `bar_fetch_commands` into
`typhoon_broker_runtime::bar_fetch_commands`; native now imports that runtime module for Alpaca,
Kraken spot, and Kraken futures bar-fetch/backfill routing while passing broker handles, shared
cache, permits, and clients explicitly.
The next physical processor-child move relocated `kraken_order_ops` into
`typhoon_broker_runtime::kraken_order_ops`; native now imports that runtime module for Kraken exit
sync, trade/open-order refresh, order placement, cancellation, and REST resync routing while
passing Kraken broker handles explicitly.
The next physical processor-child move relocated `matrix_commands` into
`typhoon_broker_runtime::matrix_commands`; native now imports that runtime module for Matrix room
join, message fetch, image send, and text send routing while the runtime crate owns the needed
Tokio fs feature for screenshot reads.
The next physical processor-child move relocated `ai_chat` into
`typhoon_broker_runtime::ai_chat`; native now imports that runtime module for AI chat request
routing while passing the shared cache explicitly for cross-client AI response caching.
The next physical processor-child move relocated `kraken_market_commands` into
`typhoon_broker_runtime::kraken_market_commands`; native now imports that runtime module for Kraken
equity ticker/history/universe and Yahoo Chart fallback bar routing while passing broker handles,
shared cache, permits, and HTTP clients explicitly.
The next physical processor-child move relocated `market_data_commands` into
`typhoon_broker_runtime::market_data_commands`; native now imports that runtime module for
fundamentals/holders, orderbook, most-active, portfolio history, analyst/price-target/short-interest,
corporate-actions, watchlist, and options-chain routing while passing broker handles and shared cache explicitly.
The next physical processor-child move relocated `research_fetch` into
`typhoon_broker_runtime::research_fetch`; native now imports that runtime module for company profile,
peers, earnings, IPOs, press, sentiment, transcripts, commodities, ETF/crypto/rates calendars, and
other research fetch routing.
The next physical processor-child move relocated `fundamentals_commands` into
`typhoon_broker_runtime::fundamentals_commands`; native now imports that runtime module for batch and
single-symbol fundamentals scrape routing while keeping fundamentals helper re-exports test-only in
native state.

### Earlier notes — Phase 1 → Phase 2 readiness (superseded)

The Phase-1 decoupling work that preceded the crate (deciding the public surface,
confirming the dependency cut, converting the per-window renderers and packet sections to
`TyphooNApp`-free free functions, and the variant-shell / `command_research_windows`
slices) is recorded in the git history of this ADR. It is fully captured by the Phase 2
step 1–3 records above and no longer tracks open work; the crate now owns `render`,
`window_shell`, `format`, `packet`, and `theme`, with only dispatchers / command handlers
/ Fetch headers (app-shell integration glue) left in native.

The window state itself (`show_*` / `*_symbol` / `*_loading` / `*_snapshot`) still lives
on `TyphooNApp` and is threaded in as `&mut` field refs; bundling it into per-window
structs is a later optional step (only needed if the shell signatures become unwieldy or
a cycle forces it), not a prerequisite for the crate.

## Verification Standard for Future Implementation

For every migration slice:

1. `cargo check -p typhoon-native`
2. relevant focused native tests
3. `cargo check --workspace`
4. `git diff --check`
5. timing comparison when a crate boundary is actually introduced
6. descriptive commit and push before starting the next boundary
