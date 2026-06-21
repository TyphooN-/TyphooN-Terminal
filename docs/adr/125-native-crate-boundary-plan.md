# ADR-125: Native Crate Boundary Plan

**Status:** Accepted as migration plan | **Date:** 2026-06-20 | **Last updated:**
2026-06-21 (Phase 0 inventory; Phase 1 step 1 — floating-windows research boundary;
Phase 1 step 2 — `symbol_investigation_packet` section formatters → ~25 free functions;
Phase 1 step 3 started — context threading (`SymbolResearchContext`, capital-valuation
converted) + a `read_conn` nesting bug fix — see
[Implementation Progress](#implementation-progress))

**Related:** ADR-086 (`typhoon-native` module decomposition), ADR-108
(research module compile-time modularization), ADR-118 (test module
convention)

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

### Next slice

Convert the remaining DB-driven sections to take `&SymbolResearchContext` — the
leaf-style dispatcher-direct ones first (ownership, market-behavior, fundamental-risk,
distribution-risk, fractal-tail, moving-average, momentum-volume), then the nesting
families (price-behavior / rank-drift / technical-indicator), whose holders then stop
re-acquiring too. Each conversion drops a `try_connection` re-acquire and a
`TyphooNApp` dependency. Once every section under the connection block takes the
context, the dispatcher's independent-connection workaround can be removed. That fully
decoupled state is the Phase 2 `typhoon-research-ui` gate.

## Verification Standard for Future Implementation

For every migration slice:

1. `cargo check -p typhoon-native`
2. relevant focused native tests
3. `cargo check --workspace`
4. `git diff --check`
5. timing comparison when a crate boundary is actually introduced
6. descriptive commit and push before starting the next boundary
