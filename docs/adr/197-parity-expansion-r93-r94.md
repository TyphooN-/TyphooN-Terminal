# ADR-197: Parity Expansion R93/R94 — OPERANK_DELTA / DIVACC / EPSACC / VRP

**Date:** 2026-04-21  
**Status:** Accepted  
**Related:** `engine/src/core/research.rs`, `engine/src/core/lan_sync.rs`, `native/src/app.rs`, ADR-129, ADR-131, ADR-133, ADR-195, ADR-196, ADR-188

## Context

After ADR-195 and ADR-196 landed, most of the long-deferred Godel parity
surface list was unblocked. The remaining practical gaps were the items that
could now be derived from caches TyphooN already persists:

- `OPERANK_DELTA` from cached `MARGINS`
- `DIVACC` from cached dividend history
- `EPSACC` from cached quarterly financial statements
- `REALIZED_VS_IMPLIED_VOL_RATIO`, reframed here as `VRP`, from cached `IVOL`
  plus `RVCONE`

The only named parity holdouts after this pass are the ones that still need
new upstream data rather than new compute:

- `SHORTRANK_DELTA` needs historical short-interest storage
- `INSIDERCONC` needs insider-ownership concentration in fundamentals

## Decision

Ship the four remaining cache-backed Godel parity surfaces as one bundled
pass:

1. `OPERANK_DELTA`
   Ranks operating-margin expansion/contraction vs same-sector peers using
   `MarginsSnapshot.operating_margin_change_pct`.
2. `DIVACC`
   Measures dividend-growth acceleration from annualized dividend buckets,
   surfacing latest y/y growth, prior y/y growth, and the acceleration delta.
3. `EPSACC`
   Measures EPS acceleration from quarterly financial statements by comparing
   the latest EPS y/y growth rate against the prior quarter's y/y growth rate.
4. `VRP`
   Pairs cached `IVOL` and `RVCONE` into a focused implied-vs-realized-vol
   premium view with cheap/fair/rich implied-vol labels.

Each surface ships end to end:

- engine compute logic
- SQLite persistence
- LAN sync registration
- research-packet output
- palette aliases
- broker plumbing
- egui popup windows

Chart overlays remain deferred by ADR-188.

## Consequences

- The deferred Godel parity list is now reduced to genuinely blocked items,
  not merely unimplemented compute-over-cache work.
- `OPERANK_DELTA` complements `OPERANK` instead of replacing it:
  current operating quality and operating-margin trend are now distinct views.
- `DIVACC` and `EPSACC` make the packet more explicit about second-derivative
  behavior instead of forcing agents to infer it from `DIVG` / `EARM`.
- `VRP` separates the focused vol-risk-premium question from the broader
  realized-vol surfaces, making the signal easier to discover in the UI and
  the packet.

## Notes

- `DIVACC` still degrades to `NO_HISTORY` for non-payers or short dividend
  histories; that is expected and matches the underlying data reality.
- `EPSACC` uses cached quarterly financial statements rather than introducing a
  new estimate-history cache.
- `VRP` requires both `IVOL` and `RVCONE`; when either side is missing it stays
  `INSUFFICIENT_DATA` instead of inventing placeholders.
