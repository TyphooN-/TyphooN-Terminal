# ADR-193: Parity Expansion R85-R86 — Stateful TA-Lib Candlestick Research Surfaces

**Status:** Accepted  
**Date:** 2026-04-21  
**Extends:** ADR-192  
**Related:** `engine/src/core/research.rs`, `engine/src/core/lan_sync.rs`, `native/src/app.rs`, ADR-188 (chart-drawing parity deferred)

## Context

After R83-R84, the remaining TA-Lib candlestick backlog leaned more
stateful. These patterns depend on multi-bar containment, false breaks,
and continuation structure rather than mostly local shadow/body ratios.

They still fit the current parity strategy:

- compute heuristically from cached historical bars
- persist one JSON snapshot per symbol in additive `research_*` tables
- register the tables for LAN sync
- expose them in the research packet and terminal popups
- defer on-chart TA-Lib overlays per ADR-188

## Scope

### Round 85

- `CDLHIKKAKE`
- `CDLHIKKAKEMOD`

### Round 86

- `CDLMATHOLD`
- `CDLRISEFALL3METHODS`

This ADR covers candlestick research surfaces only. It does not add a
new quant-stat family or chart study overlay.

## Decision

Ship R85-R86 as one consolidated four-feature bundle.

The additive schema layer is:

- `v87`: `research_cdl_hikkake`, `research_cdl_hikkake_mod`, `research_cdl_mat_hold`, `research_cdl_rise_fall_three_methods`

For every feature in this ADR:

- Research packet: yes
- egui popup: yes
- Palette aliases: yes
- LAN sync: yes
- Chart overlay: no, deferred by ADR-188

## Consequences

### Positive

- Four more of the stateful TA-Lib candlestick patterns are available
  end to end.
- The parity backlog moves beyond simpler single-bar and direct
  three-bar formations into the trap/continuation family.
- The engine and UI keep one consistent snapshot workflow instead of
  creating a special-case TA-Lib subsystem.

### Negative / risks

- The additive schema chain grows to `v87`.
- These detectors remain heuristic approximations of TA-Lib semantics,
  not a byte-identical implementation.
- Stateful patterns like Hikkake and Mat Hold are threshold-sensitive,
  so the heuristics may still need tuning after real-world use.

### Neutral

- This continues the bundled ADR approach established in ADR-189
  through ADR-192.
- The research packet keeps expanding as more parity surfaces land.

## Verification

- `env CARGO_TARGET_DIR=/tmp/typhoon-engine-parity-tests cargo test --manifest-path engine/Cargo.toml cdl_ -- --nocapture`
  covers the new roundtrip and detector tests for all four additions.
- `env CARGO_TARGET_DIR=/tmp/typhoon-native-parity-check cargo check --manifest-path native/Cargo.toml`
  verifies the broker wiring, packet output, aliases, and popup windows.
- `engine/src/core/lan_sync.rs`
  includes all `v87` tables in `SYNCABLE_TABLES`, `create_table_sql`,
  and `table_timestamp_column`.
