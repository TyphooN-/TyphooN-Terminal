# ADR-192: Parity Expansion R83-R84 — Additional Multi-Bar TA-Lib Candlestick Research Surfaces

**Status:** Accepted  
**Date:** 2026-04-20  
**Extends:** ADR-191  
**Related:** `engine/src/core/research.rs`, `engine/src/core/lan_sync.rs`, `native/src/app.rs`, ADR-188 (chart-drawing parity deferred)

## Context

After ADR-191, the remaining TA-Lib candlestick backlog was mostly the
multi-bar gap and continuation/reversal set. These still fit the
current research-snapshot architecture:

- compute from cached historical bars
- persist one JSON snapshot per symbol in additive `research_*` tables
- register each table in LAN sync
- expose the result in the research packet and an egui popup
- keep chart overlays deferred by ADR-188

This round keeps the same approach rather than splitting another batch
of tiny ADRs or pretending to implement byte-for-byte TA-Lib internals.

## Scope

### Round 83

- `CDLADVANCEBLOCK`
- `CDLBREAKAWAY`
- `CDLGAPSIDESIDEWHITE`

### Round 84

- `CDLUPSIDEGAP2CROWS`
- `CDLXSIDEGAP3METHODS`
- `CDLCONCEALBABYSWALL`

These are TA-Lib candlestick research surfaces only. They are not new
Godel-only quant-stat windows.

## Decision

Ship R83-R84 as one consolidated six-feature bundle.

The new additive schema layer is:

- `v86`: `research_cdl_advance_block`, `research_cdl_breakaway`, `research_cdl_gap_side_side_white`, `research_cdl_upside_gap_two_crows`, `research_cdl_xside_gap_three_methods`, `research_cdl_conceal_baby_swallow`

For every feature in this ADR:

- Research packet: yes
- egui popup: yes
- Palette aliases: yes
- LAN sync: yes
- Chart overlay: no, deferred by ADR-188

## Consequences

### Positive

- Six more TA-Lib candlestick surfaces are available end to end.
- The remaining candlestick parity gap is pushed further into the truly
  stateful or more interpretation-sensitive family.
- The new bundle covers more of the gap/continuation subfamily that was
  still absent from the research packet and terminal palette.

### Negative / risks

- The additive schema chain grows to `v86`.
- These remain heuristic research surfaces aligned to TA-Lib semantics,
  not a byte-identical TA-Lib port.
- Some patterns in this bundle, especially the rarer four- and five-bar
  formations, are sensitive to threshold choices and may still evolve in
  later parity passes.

### Neutral

- This ADR continues the bundled documentation policy used in ADR-189,
  ADR-190, and ADR-191.
- The research packet continues to grow as more candlestick blocks are
  added.

## Verification

- `env CARGO_TARGET_DIR=/tmp/typhoon-engine-parity-tests cargo test --manifest-path engine/Cargo.toml cdl_ -- --nocapture`
  passed with the new roundtrip and detector tests included.
- `env CARGO_TARGET_DIR=/tmp/typhoon-native-parity-check cargo check --manifest-path native/Cargo.toml`
  passed after wiring the broker path, aliases, packet output, and popup
  windows.
- `engine/src/core/lan_sync.rs`
  now includes all `v86` tables in `SYNCABLE_TABLES`,
  `create_table_sql`, and `table_timestamp_column`.
