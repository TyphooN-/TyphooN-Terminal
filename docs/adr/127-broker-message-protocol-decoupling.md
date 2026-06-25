# ADR-127: Broker Message Protocol Decoupling (prerequisite for ADR-125 Target 3)

**Status:** IMPLEMENTED (Phases A–C done 2026-06-24; ADR-125 Target 3 completed
2026-06-25) | **Date:** 2026-06-24 |
**Related:** ADR-125 (native crate boundary plan — this unblocks its Target 3,
`typhoon-broker-runtime`), ADR-108 (research module compile-time modularization — the
`research_compute` engine split this depends on), ADR-126 (primary/assist broker selection —
`OrderBroker` lives in the protocol)

## Context

ADR-125 delivered its two prioritized native-crate boundaries — `typhoon-research-ui` and
`typhoon-chart-ui` — and evaluated **Target 3 (`typhoon-broker-runtime`)** in depth. Target 3
is **not** blocked by the deprecated-broker rip-out (that has landed on `master`; engine
`broker/` is Kraken + Alpaca only), nor by the processor task's shape (it is already a
well-decoupled async task: `spawn_broker_message_processor` takes explicit
channel/cache/runtime params, the 19 handler files have **0 `impl TyphooNApp`** and **0
`self.` access**).

The actual blocker is a **dependency cycle through the broker message protocol**. The
protocol — `BrokerCmd` / `BrokerMsg` / `OrderBroker` / `QuickTradePlan` /
`TradeAccountSnapshot`, then a native broker-message module and now
`typhoon_engine::broker::protocol` — is
the app-wide message bus (referenced in **220 / 97 files**). Moving it into a native-adjacent
crate as-is would pull native state across the boundary. Measured coupling:

- **The protocol is ~99% engine-typed.** It references **451 engine-defined types** and is
  written over `use super::*` (the native `state` module).
- **Exactly one genuine native payload type:** `BrokerMsg::WatchlistQuotes(Vec<WatchlistRow>)`,
  where `WatchlistRow` (`app/state/watchlist.rs`, 102 lines) is a **plain `serde` data struct**
  — `egui=0`, `engine=0`, `TyphooNApp=0` — used in only 8 files. (An earlier scan also flagged
  `Indicator`/`gpu_compute`; that was a false positive — the word only appears in doc comments.)
- **`research_compute/` is woven into the protocol, not a separable island.** It is 58 files /
  ~13.1k lines (70% of `app_broker_processor/`), is a *protocol sender* — it emits ~dozens of
  `BrokerMsg::*SnapshotMsg` research results (**926 `msg_tx` sends**, **1525 `BrokerMsg`/`BrokerCmd`
  refs**) — and per ADR-108 its eventual home is **engine**, not a native broker crate.

So the broker subsystem is one tightly-coupled **protocol ↔ compute ↔ native-state** fabric.
The good news from the measurement: the *protocol's* native entanglement is tiny (one
relocatable DTO + a glob import). The weight is the layering question for `research_compute`,
which is shared with ADR-108.

## Decision

Before extracting any broker crate, **make the broker message protocol depend only on
`typhoon-engine` + `std`, and give it a home that both the engine-side compute
(`research_compute`, bound for engine per ADR-108) and the native broker runtime can depend on
without a cycle.**

Concretely:

1. **Relocate `WatchlistRow` into `typhoon-engine`** (it is already a pure quote-row DTO).
   Native re-exports it from `app/state/watchlist` so the 8 call sites are unchanged. This
   removes the protocol's only native payload dependency.
2. **Replace `broker_messages.rs`'s `use super::*` with explicit `typhoon_engine` + `std`
   imports**, then assert (by `grep`/`cargo`) that the protocol references no `crate::app::*`
   type. At that point `broker_messages.rs` is engine/std-only.
3. **Move the protocol to `typhoon-engine`** (e.g. `typhoon_engine::broker::protocol`).
   Rationale: `research_compute` emits `BrokerMsg` and is bound for engine (ADR-108); engine
   cannot depend on a native crate, so the protocol must sit at or below engine's research
   layer. The protocol is already 99% engine-typed, so this is its natural home — not a native
   `typhoon-broker-runtime` crate. `typhoon-native` re-exports `BrokerCmd`/`BrokerMsg`/… from
   engine so the 220/97 call sites and every `BrokerCmd::Variant { … }` construction are
   unchanged (the same re-export pattern proven 4× in ADR-125 Targets 1–2).
4. **Then, and only then, the two downstream boundaries become clean cuts** (each its own
   effort, sequenced after this ADR):
   - `research_compute` → engine, per ADR-108 (now that the protocol it emits lives in engine).
   - the native broker *runtime* (the `spawn_broker_message_processor` task + the 19 handler
     families) → ADR-125 Target 3 (`typhoon-broker-runtime`), depending on the engine-resident
     protocol + engine clients + cache. Whether this becomes a crate or stays native is
     ADR-125's call once the protocol is no longer in the way.

This ADR's scope is **only step 1–3** (make the protocol state-independent and relocate it).
Steps 4 are downstream and tracked by ADR-108 / ADR-125.

## Plan (phased, one verified commit each)

**Phase A — relocate `WatchlistRow` to engine.**
Move the struct into `typhoon-engine` (a `core::watchlist` or `broker` DTO module), keep
`#[derive(Serialize, Deserialize, Clone, …)]`. Native `app/state/watchlist` re-exports it.
Verify: workspace builds, tests green, the 8 call sites unchanged.

**Phase B — make `broker_messages.rs` engine/std-only.**
Replace `use super::*` with explicit imports; resolve any remaining `crate::app` references
(expected: none beyond `WatchlistRow`, now engine). Add a guard test / CI grep asserting the
protocol module imports nothing from `crate::app`. Verify build + tests.

**Phase C — move the protocol into `typhoon-engine`.**
Create `typhoon_engine::broker::protocol` with `BrokerCmd`/`BrokerMsg`/`OrderBroker`/
`QuickTradePlan`/`TradeAccountSnapshot`. `typhoon-native` re-exports them from
`app/state/broker_messages.rs`, now a 4-line shim to the engine protocol. Verify the 220/97-file surface
compiles unchanged and the full workspace test suite is green. Confirm acyclic
(`engine` gains no dependency on `native`).

After Phase C, hand off: ADR-108 can continue tracking any future engine-side research
compute ownership questions; ADR-125 re-opened and then completed Target 3 for the native
broker runtime.

## Guardrails

- **Behavior-preserving only.** No protocol semantics change — this is type relocation +
  import hygiene, verified by the unchanged 220/97 call sites and the full test suite at each
  phase (the ADR-125 standard).
- **Do not move the broker *runtime* (handlers, async loop, reconciliation) into engine.**
  Only the message *protocol* (the data contract) and the `WatchlistRow` DTO move to engine.
  The runtime is native (or, later, a native `typhoon-broker-runtime` crate per ADR-125).
- **Do not let engine gain a dependency on `typhoon-native`** — verify with `cargo tree` after
  Phase C.
- **`OrderBroker` moves with the protocol** (it is the ADR-126 broker-identity enum and a
  `BrokerCmd`/`BrokerMsg` field). Note this also lets the chart-UI equity-merge (kept native
  in ADR-125 partly because `OrderBroker` was native) reconsider its boundary later — out of
  scope here.

## Consequences

Positive:
- Unblocks ADR-125 Target 3 by removing the only structural blocker (the protocol↔state cycle),
  reduced by measurement to a single relocatable DTO plus import hygiene.
- Unblocked the broker-runtime move that ADR-125 Target 3 needed. The protocol now lives in
  engine, while `research_compute` remains with the broker-runtime command handlers that emit
  protocol messages.
- Puts the broker data contract where its types already live (99% engine), which is more honest
  than the original ADR-125 sketch that implied the protocol would live in a native
  `typhoon-broker-runtime` crate.

Tradeoffs / open questions:
- **Protocol home: `typhoon-engine` vs a new thin `typhoon-broker-protocol` crate.** Engine is
  recommended (research_compute needs it and is engine-bound), but if the team prefers to keep
  engine free of UI-facing message enums, a minimal shared `typhoon-broker-protocol` crate that
  both engine and native depend on is the alternative. Decide at Phase C.
- Moving a ~3.7k-line, 220-file-referenced protocol is low-risk per-edit (re-export keeps call
  sites stable) but touches a lot of files; do it as its own ADR-tracked effort, not folded
  into an unrelated change.
- `research_compute`'s engine move (ADR-108) remains the larger downstream effort; this ADR
  only removes the protocol obstacle in its path.

## Implementation (2026-06-24)

All three phases landed, each its own commit, full workspace **2272 tests** green throughout.

- **Phase A** — `WatchlistRow` relocated to `typhoon_engine::core::watchlist` (pure `serde`
  DTO; fields widened `pub`). Native `app/state/watchlist` keeps the row builders + re-exports
  it; the 8 call sites unchanged.
- **Phase B** — `broker_messages.rs` made engine/std-only: `use super::*` replaced by exactly
  three bare-name imports (`std::path::PathBuf`, the alpaca `AccountInfo`/`OrderInfo`/`PositionInfo`
  trio, engine `WatchlistRow`). Everything else was *already* written with fully-qualified
  `typhoon_engine::…` paths (439 `research::` refs alone), so the file reached zero `crate::app`
  references with a 3-line import set — smaller than anticipated.
- **Phase C** — the protocol moved to **`typhoon_engine::broker::protocol`** (the recommended
  home; the open question resolved to engine, not a separate `typhoon-broker-protocol` crate).
  450 `typhoon_engine::`→`crate::` rewrites, `pub(crate)`→`pub` ×30, the `impl OrderBroker`
  block moved with it; no orphan-rule conflicts. `typhoon-native` keeps a 4-line re-export
  shim, so the ~220/97 call sites are byte-for-byte unchanged. **`cargo tree` confirms engine
  gained no `typhoon-native` dependency** (acyclic).

**Outcome:** the broker message protocol is engine-resident and state-independent. ADR-127 is
done. Its ADR-125 downstream dependency also landed: `research_compute` stayed with the broker
runtime command handlers, and the native broker runtime moved to `typhoon-broker-runtime` without
creating an engine→native cycle.
