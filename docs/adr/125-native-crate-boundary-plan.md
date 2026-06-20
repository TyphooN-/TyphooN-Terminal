# ADR-125: Native Crate Boundary Plan

**Status:** Accepted as migration plan | **Date:** 2026-06-20

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

## Verification Standard for Future Implementation

For every migration slice:

1. `cargo check -p typhoon-native`
2. relevant focused native tests
3. `cargo check --workspace`
4. `git diff --check`
5. timing comparison when a crate boundary is actually introduced
6. descriptive commit and push before starting the next boundary
