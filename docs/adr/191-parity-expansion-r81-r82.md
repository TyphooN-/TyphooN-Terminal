# ADR-191: Parity Expansion R81-R82 — Harder TA-Lib Candlestick Research Surfaces

**Status:** Accepted  
**Date:** 2026-04-20  
**Extends:** ADR-190  
**Related:** `engine/src/core/research.rs`, `engine/src/core/lan_sync.rs`, `native/src/app.rs`, ADR-188 (chart-drawing parity deferred)

## Context

After ADR-190, the remaining TA-Lib `CDL*` backlog was mostly the set
that needed slightly tighter multi-bar heuristics, but still fit the
existing candlestick snapshot architecture:

- compute from cached historical bars
- persist one JSON snapshot per symbol in additive `research_*` tables
- include the table in LAN sync
- expose the result in the research packet and an egui popup
- keep chart overlays deferred by ADR-188

This round deliberately continues that model instead of jumping into the
more stateful `HIKKAKE`, `BREAKAWAY`, `MATHOLD`, or three-methods family
in the same pass.

## Scope

### Round 81

- `CDL3STARSINSOUTH`
- `CDLIDENTICAL3CROWS`
- `CDLKICKING`

### Round 82

- `CDLKICKINGBYLENGTH`
- `CDLLADDERBOTTOM`
- `CDLUNIQUE3RIVER`

These are TA-Lib candlestick research surfaces only. They are not new
Godel-documented quant-stat windows.

## Decision

Ship R81-R82 as one consolidated six-feature bundle.

The new additive schema layer is:

- `v85`: `research_cdl_three_stars_in_south`, `research_cdl_identical_three_crows`, `research_cdl_kicking`, `research_cdl_kicking_by_length`, `research_cdl_ladder_bottom`, `research_cdl_unique_three_river`

For every feature in this ADR:

- Research packet: yes
- egui popup: yes
- LAN sync: yes
- Palette aliases: yes
- Chart overlay: no, deferred by ADR-188

## Consequences

### Positive

- Six more TA-Lib candlestick surfaces are available end to end.
- The parity backlog shrinks without fragmenting into another cluster of
  tiny ADRs.
- `CDLKICKING` and `CDLKICKINGBYLENGTH` now expose both direct and
  longer-body interpretations with the same storage and UI path as the
  rest of the candlestick family.

### Negative / risks

- The additive schema chain grows to `v85`.
- These detectors remain heuristic approximations of TA-Lib semantics,
  not a byte-for-byte TA-Lib port.
- The remaining backlog is increasingly composed of the more stateful
  or more interpretation-sensitive candlestick patterns.

### Neutral

- This ADR continues the bundled documentation approach from ADR-189 and
  ADR-190.
- The research packet continues to grow as more candlestick sections are
  added.

## Verification

- `env CARGO_TARGET_DIR=/tmp/typhoon-engine-parity-tests cargo test --manifest-path engine/Cargo.toml cdl_ -- --nocapture`
  passed, including the new roundtrip and detector tests for all six
  surfaces.
- `env CARGO_TARGET_DIR=/tmp/typhoon-native-parity-check cargo check --manifest-path native/Cargo.toml`
  passed after wiring the broker commands, packet output, aliases, and
  popup windows.
- `engine/src/core/lan_sync.rs`
  now includes all `v85` tables in `SYNCABLE_TABLES`,
  `create_table_sql`, and `table_timestamp_column`.
