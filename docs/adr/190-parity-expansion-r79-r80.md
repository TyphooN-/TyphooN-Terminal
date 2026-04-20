# ADR-190: Parity Expansion R79-R80 — Additional TA-Lib Candlestick Research Surfaces

**Status:** Accepted  
**Date:** 2026-04-20  
**Extends:** ADR-189  
**Related:** `engine/src/core/research.rs`, `engine/src/core/lan_sync.rs`, `native/src/app.rs`, ADR-188 (chart-drawing parity deferred)

## Context

After ADR-189, the remaining TA-Lib `CDL*` gap was no longer the easy
"add any five" set. The backlog had split into two groups:

- Low-ambiguity patterns that still fit the existing `candle_metrics` +
  `cdl_scan` snapshot model.
- More stateful or more subjective patterns such as `HIKKAKE`,
  `MATHOLD`, and `BREAKAWAY`, where false positives become more likely
  if we rush them into the same heuristic envelope.

The implementation architecture still did not change:

- Compute from cached per-symbol historical bars.
- Persist one JSON snapshot per symbol in additive `research_*` tables.
- Register the tables in LAN sync.
- Expose each surface in the research packet and a dedicated egui popup.
- Keep chart overlay work deferred by ADR-188.

## Scope

### Round 79

- `CDL2CROWS`
- `CDL3LINESTRIKE`
- `CDL3OUTSIDE`
- `CDLMATCHINGLOW`

### Round 80

- `CDLSEPARATINGLINES`
- `CDLSTICKSANDWICH`
- `CDLRICKSHAWMAN`
- `CDLTAKURI`

These are all TA-Lib candlestick primitives. None are Godel-documented
research windows.

## Decision

Ship R79-R80 as one consolidated ADR and deliberately pick the next
eight `CDL*` surfaces that can be implemented cleanly with the current
snapshot model.

The new additive schema layers are:

- `v83`: `research_cdl_two_crows`, `research_cdl_three_line_strike`, `research_cdl_three_outside`, `research_cdl_matching_low`
- `v84`: `research_cdl_separating_lines`, `research_cdl_stick_sandwich`, `research_cdl_rickshaw_man`, `research_cdl_takuri`

For every feature in this ADR:

- Research packet: yes
- egui popup: yes
- LAN sync: yes
- Chart overlay: no, deferred by ADR-188

The remaining harder `CDL*` surfaces stay deferred to later rounds so we
do not dilute parity quality with over-loose heuristics.

## Consequences

### Positive

- Eight more TA-Lib candlestick surfaces are available end-to-end with
  the same discoverability as the prior rounds.
- The bundle keeps momentum on parity while avoiding the riskier,
  stateful pattern detectors for now.
- The code stays consistent with the existing candlestick architecture,
  so future additions still compose predictably.

### Negative / risks

- The additive schema chain grows to `v84`.
- Some labels remain heuristic approximations of TA-Lib semantics rather
  than byte-for-byte reference implementations.
- The remaining backlog is now disproportionately the harder `CDL*`
  family members.

### Neutral

- This ADR continues the newer "bundle more than five when the
  architecture is unchanged" documentation policy from ADR-189.
- The research packet becomes incrementally larger as more candlestick
  blocks are added.

## Verification

- `cargo test --manifest-path engine/Cargo.toml cdl_ -- --nocapture`
  passed with the new R79-R80 tests included.
- `env CARGO_TARGET_DIR=/tmp/typhoon-native-parity-check cargo check --manifest-path native/Cargo.toml`
  passed after wiring the new popup, palette, and packet surfaces.
- `engine/src/core/lan_sync.rs`
  includes all `v83` and `v84` tables in `SYNCABLE_TABLES`,
  `create_table_sql`, and `table_timestamp_column`.
