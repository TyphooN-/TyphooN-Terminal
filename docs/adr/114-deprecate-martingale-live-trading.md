# ADR-114: Deprecate Martingale Live-Trading Support

**Status:** Accepted | **Date:** 2026-06-11

## Context

TyphooN Terminal carried a hedged martingale implementation ported from the older
TyphooN EA flow: mode cycling, Open MG sizing, TRIM/PROTECT decisions, equity TP,
and unwind behavior. That code was useful as historical context, but it is not a
sane default strategy for live trading. It encourages position-size escalation
into adverse moves, hides tail risk behind margin mechanics, and creates the kind
of operational risk this terminal should prevent rather than normalize.

## Decision

Martingale support is deprecated from `master` and removed from the built product.
The historical implementation is preserved on the branch:

- `archive/martingale-deprecated`

That branch is a restore/reference point only. It is not part of the supported
live-trading surface, not tested, and not maintained unless explicitly revived
for research in an isolated branch.

## Consequences

- Remove the `typhoon-engine` martingale strategy module and its tests from the
  active build.
- Remove Open MG / martingale command-palette and menu affordances.
- Remove documentation that advertised martingale as an active feature.
- Keep normal risk tooling: TRIM/margin math, VaR sizing, risk calculator, SL/TP,
  position tracking, and order-management controls.
- Do not reintroduce martingale into live trading without a new ADR and explicit
  safety case.
