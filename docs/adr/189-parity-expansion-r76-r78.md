# ADR-189: Parity Expansion R76-R78 — Quant Stats + TA-Lib Research Surfaces

**Status:** Accepted  
**Date:** 2026-04-20  
**Supersedes/extends:** ADR-108 through ADR-188  
**Consolidates:** former ADR-189 / ADR-190 / ADR-191  
**Related:** `engine/src/core/research.rs`, `engine/src/core/lan_sync.rs`, `native/src/app.rs`, ADR-150 (Quant Stats Round 41), ADR-188 (chart-drawing parity deferred)

## Context

Recent parity work kept using the same implementation envelope while the
docs kept being split into one ADR per five features. That was too much
ADR churn for too little architectural change.

Across these rounds, the implementation pattern never changed:

- Compute from the existing per-symbol historical-price cache.
- Persist one JSON snapshot per symbol into additive `research_*` tables.
- Register those tables in LAN sync.
- Surface the result in the research packet and an egui popup.
- Keep chart overlay deferred per ADR-188.

This ADR replaces the recent per-round files with one consolidated
record and extends the same parity track through the latest TA-Lib
candlestick bundle.

## Scope

### Quant stats packs

- R76 pack: `MODSHARPE`, `HSIEHTEST`, `CHOWBREAK`, `DRIFTBURST`, `HLVCLUST`
- R77 pack: `YANGZHANG`, `KUIPER`, `DAGOSTINO`, `BAIPERRON`, `KUPIECPOF`

These are not Godel-documented features and not TA-Lib primitives. They
are pure econometric/statistical research surfaces.

### TA-Lib candlestick packs

- R76 pack: `CDLDOJISTAR`, `CDLMORNINGDOJISTAR`, `CDLEVENINGDOJISTAR`, `CDLABANDONEDBABY`, `CDL3INSIDE`
- R77 pack: `CDLBELTHOLD`, `CDLCLOSINGMARUBOZU`, `CDLHIGHWAVE`, `CDLLONGLINE`, `CDLSHORTLINE`
- R78 pack: `CDLCOUNTERATTACK`, `CDLHOMINGPIGEON`, `CDLINNECK`, `CDLONNECK`, `CDLTHRUSTING`

These are TA-Lib `CDL*` parity surfaces, all using the existing
`candle_metrics` and `cdl_scan` helper pattern already established by
the earlier candlestick rounds.

## Decision

Ship the full R76-R78 parity expansion as one consolidated ADR instead
of keeping separate ADRs for each five-feature slice.

The code remains grouped by additive schema layers:

- `v78`: `research_modsharpe`, `research_hsiehtest`, `research_chowbreak`, `research_driftburst`, `research_hlvclust`
- `v79`: `research_yangzhang`, `research_kuiper`, `research_dagostino`, `research_baiperron`, `research_kupiecpof`
- `v80`: `research_cdl_doji_star`, `research_cdl_morning_doji_star`, `research_cdl_evening_doji_star`, `research_cdl_abandoned_baby`, `research_cdl_three_inside`
- `v81`: `research_cdl_belt_hold`, `research_cdl_closing_marubozu`, `research_cdl_high_wave`, `research_cdl_long_line`, `research_cdl_short_line`
- `v82`: `research_cdl_counterattack`, `research_cdl_homing_pigeon`, `research_cdl_in_neck`, `research_cdl_on_neck`, `research_cdl_thrusting`

For every feature in this ADR:

- Research packet: yes
- egui popup: yes
- LAN sync: yes
- Chart overlay: no, deferred by ADR-188

Going forward, related parity work does not need to be constrained to
five features per ADR when the architecture is unchanged.

## Consequences

### Positive

- The docs are materially simpler: one consolidated ADR replaces three
  recent near-duplicate round files.
- The implementation pattern stays consistent across quant stats and
  TA-Lib parity work, which makes future additions cheaper.
- No new fetchers, no cross-symbol scans, and no new external API
  dependencies were required for any feature in this consolidated set.

### Negative / risks

- The schema chain is longer (`v78` through `v82`), so future additions
  still need to preserve the additive migration pattern carefully.
- Packet size continues to grow incrementally as more optional research
  blocks are added.
- Chart overlay remains intentionally absent until ADR-188 is reopened.

### Neutral

- Older round-specific ADR files are removed because this document now
  carries the decision record for those slices.
- Code comments may still reference the historical round names near the
  implementation sites; the canonical documentation record is now this
  consolidated ADR.

## Verification

- `cargo test --manifest-path engine/Cargo.toml cdl_ -- --nocapture`
  70 CDL tests passing after the R77 + R78 candlestick additions.
- `cargo check --manifest-path native/Cargo.toml`
  clean via isolated target-dir build while wiring the new popup and
  packet surfaces.
- `engine/src/core/lan_sync.rs`
  includes all new `research_*` tables from `v78` through `v82` in
  `SYNCABLE_TABLES`, `create_table_sql`, and `table_timestamp_column`.
