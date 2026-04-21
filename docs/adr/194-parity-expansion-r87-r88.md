# ADR-194: Parity Expansion R87-R88 — Final TA-Lib Candlestick Research Surfaces

**Status:** Accepted  
**Date:** 2026-04-21  
**Extends:** ADR-193  
**Related:** `engine/src/core/research.rs`, `engine/src/core/lan_sync.rs`, `native/src/app.rs`, ADR-188 (chart-drawing parity deferred)

## Context

After ADR-193, the remaining TA-Lib candlestick backlog in the terminal
was down to a very small tail: a stalled three-candle exhaustion
pattern and a gap-continuation pattern that still needed to show up in
research packets, LAN sync, and popup inspection windows.

They still fit the established parity approach:

- compute from cached historical bars with local heuristics
- persist one JSON snapshot per symbol in additive `research_*` tables
- register those tables for LAN sync
- expose the results in the research packet and egui popups
- keep chart overlays deferred by ADR-188

## Scope

### Round 87

- `CDLSTALLEDPATTERN`

### Round 88

- `CDLTASUKIGAP`

This ADR covers research surfaces only. It does not add a chart-study
overlay or a separate TA-Lib runtime dependency.

## Decision

Ship R87-R88 as one final candlestick parity bundle.

The additive schema layer is:

- `v88`: `research_cdl_stalled_pattern`, `research_cdl_tasuki_gap`

For every feature in this ADR:

- Research packet: yes
- egui popup: yes
- Palette aliases: yes
- LAN sync: yes
- Chart overlay: no, deferred by ADR-188

## Consequences

### Positive

- The remaining candlestick parity backlog in this research-snapshot
  track is closed out.
- Terminal users can inspect stalled-pattern and tasuki-gap signals
  with the same workflow as the prior CDL bundles.
- LAN peers and stored snapshots stay additive and backward-compatible
  with the existing parity architecture.

### Negative / risks

- The additive schema chain grows to `v88`.
- These detectors are heuristic TA-Lib-aligned implementations, not a
  byte-for-byte TA-Lib port.
- Tasuki Gap and Stalled Pattern are both threshold-sensitive and may
  still need tuning against live datasets.

### Neutral

- This continues the bundled ADR approach from ADR-189 through ADR-193.
- Chart-drawing parity remains deferred under ADR-188 even though the
  research surfaces are now in place.

## Verification

- `cargo test --manifest-path engine/Cargo.toml stalled_pattern -- --nocapture`
  verifies the new roundtrip and detector coverage for
  `CDLSTALLEDPATTERN`.
- `cargo test --manifest-path engine/Cargo.toml tasuki_gap -- --nocapture`
  verifies the new roundtrip and detector coverage for `CDLTASUKIGAP`.
- `cargo check --manifest-path native/Cargo.toml`
  verifies the broker wiring, packet output, palette aliases, and popup
  windows for the new bundle.
