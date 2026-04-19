# ADR-179: Godel Parity Round 67 — PLUS_DI / MINUS_DI / PLUS_DM / MINUS_DM / DX

**Status:** Accepted
**Date:** 2026-04-18
**Supersedes/extends:** ADR-108 through ADR-178
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Context

Round 66 (ADR-178) shipped the TA-Lib price-transform primitives
(AVGPRICE / MEDPRICE / TYPPRICE / WCLPRICE / VARIANCE). Round 67 now
completes the **Wilder Directional Movement System**: the five TA-Lib
primitives that ADX (ADR-136 / Round 31) and ADXR (ADR-177 / Round 65)
are built on top of. ADX / ADXR consumers have been consulting the
downstream-smoothed `adx`/`adxr` scalars since those rounds shipped,
but the intermediate surfaces (+DI, −DI, +DM, −DM, DX) that reveal
*why* trend strength is rising/falling were not individually
addressable from the research packet or palette. Godel parity agents
asking "is this a crossover?" or "is +DI's rate-of-change accelerating
ahead of ADX?" have no data to answer those questions without
recomputing the series themselves.

1. **No PLUS_DI snapshot.** Wilder's `+DI_t = 100 · (Wilder-smoothed
   +DM) / ATR` over 14-bar default. The bull-pressure scalar whose
   crossover with −DI is the classic DMI trading signal (long on
   +DI > −DI, short on +DI < −DI) — distinct from ADX (trend strength
   regardless of direction) and from MOM / ROC (raw return, no
   range-normalisation). Header gives **plus_di_label** (BULL_DOMINANT
   / BULL_LEAN / NEUTRAL / BEAR_LEAN / INSUFFICIENT_DATA for n<16)
   from the `+DI − −DI` spread magnitude (>10 bull_dominant,
   >2 bull_lean, <-2 bear_lean, else neutral).

2. **No MINUS_DI snapshot.** Wilder's `−DI_t = 100 · (Wilder-smoothed
   −DM) / ATR`. Mirror of +DI for downward directional movement;
   separating the two lets agents detect asymmetric regimes (e.g. +DI
   declining while −DI is flat — trend weakening ahead of ADX
   confirmation). Header gives **minus_di_label** (BEAR_DOMINANT /
   BEAR_LEAN / NEUTRAL / BULL_LEAN / INSUFFICIENT_DATA) using the same
   spread-magnitude bands with the opposite sign convention.

3. **No PLUS_DM snapshot.** The *raw* per-bar upward directional
   movement primitive: `+DM_t = max(0, H_t − H_{t−1})` only when that
   up-move exceeds `L_{t−1} − L_t`. Distinct from +DI because +DM is
   pre-Wilder-smoothing and pre-ATR-normalisation — it exposes the
   actual tick-level directional footprint that DI smooths over.
   Useful for agents diagnosing whether a low DI reflects flat
   price-action (raw +DM ~ 0) versus offsetting +DM/−DM churn (both
   non-zero, cancelling in DX). Header gives **plus_dm_label**
   (BULL_PRESSURE / BULL_SOFT / NEUTRAL / BEAR_PRESSURE /
   INSUFFICIENT_DATA) from the raw `+DM` / `−DM` ratio (>2× bull_pressure,
   >1× bull_soft, else neutral or bear-flipped).

4. **No MINUS_DM snapshot.** Mirror of +DM for downward motion:
   `−DM_t = max(0, L_{t−1} − L_t)` only when that down-move exceeds
   `H_t − H_{t−1}`. Exposes raw bear-pressure footprint; useful for
   the same asymmetric-regime diagnostics as −DI but at the pre-ATR
   primitive level. Header gives **minus_dm_label** (BEAR_PRESSURE /
   BEAR_SOFT / NEUTRAL / BULL_PRESSURE / INSUFFICIENT_DATA) with mirror
   semantics.

5. **No DX snapshot.** Wilder's unsmoothed `DX = 100 · |+DI − −DI| /
   (+DI + −DI)` — the raw directional-dispersion primitive that ADX
   is built from via a second Wilder smoothing. Distinct from ADX:
   DX is the per-bar dispersion (can swing wildly), whereas ADX is
   its 14-bar Wilder smoothing. Useful for agents asking "is today's
   DX outpacing ADX?" (a leading-indicator setup) or for detecting
   pre-smoothing directional shifts that the smoothed ADX would lag.
   Header gives **dx_label** (STRONG_DIR / DIR / WEAK_DIR / NO_DIR /
   INSUFFICIENT_DATA) mirroring the ADX band cutoffs (40/25/15).

## Decision

Add five optional per-symbol snapshot blocks to the Godel-parity
pipeline, each reusing the existing `research_historical_price` HP
cache and the standard research-table LAN-sync path (no new API
dependencies):

1. `research::PlusDiSnapshot` + `compute_plus_di_snapshot` +
   `upsert_plus_di` + `get_plus_di` — serialised to
   `research_plus_di`.
2. `research::MinusDiSnapshot` + `compute_minus_di_snapshot` +
   `upsert_minus_di` + `get_minus_di` — serialised to
   `research_minus_di`.
3. `research::PlusDmSnapshot` + `compute_plus_dm_snapshot` +
   `upsert_plus_dm` + `get_plus_dm` — serialised to
   `research_plus_dm`.
4. `research::MinusDmSnapshot` + `compute_minus_dm_snapshot` +
   `upsert_minus_dm` + `get_minus_dm` — serialised to
   `research_minus_dm`.
5. `research::DxSnapshot` + `compute_dx_snapshot` + `upsert_dx` +
   `get_dx` — serialised to `research_dx`.

All five compute functions share a private helper `compute_dmi_series`
that walks the bars once to build the `(+DI, −DI, ATR, +DM smoothed,
−DM smoothed, TR smoothed, +DM raw, −DM raw)` series via one Wilder
pass; each compute_* then selects the bar-wise scalars relevant to
its snapshot. This keeps per-compute cost bounded to O(n) and
eliminates the drift risk of five ad-hoc Wilder reimplementations.

Schema version bumps to v69 via `create_research_tables_v69` which
wraps v68 and adds five new CREATE TABLE stanzas (plus indexes on
`updated_at`). All five tables register in `SYNCABLE_TABLES`,
`create_table_sql`, and `table_timestamp_column` so LAN sync
incrementally propagates them to peer terminals.

Native wiring adds five `BrokerCmd::Compute*Snapshot` variants, five
`BrokerMsg::*SnapshotMsg` variants, five `show_*_win` / `*_symbol` /
`*_snapshot` / `*_loading` field tuples on `App`, five tokio-spawned
broker handlers (load HP cache → compute → upsert → emit msg), five
palette alias blocks, five packet-emitter blocks under sections
2.323-2.327 of the research packet, five egui windows with
Use-Chart / Load-Cached / Compute controls plus a striped Grid
summary, and five `BrokerMsg` match arms.

Palette aliases were selected to avoid collision with earlier rounds
(verified at implementation time against R60..R66 token sets):
`PLUS_DI | PDI | DI_PLUS | DIPOS | WILDER_PDI`;
`MINUS_DI | MDI | DI_MINUS | DINEG | WILDER_MDI`;
`PLUS_DM | PDM | DM_PLUS | DMPOS | WILDER_PDM`;
`MINUS_DM | MDM | DM_MINUS | DMNEG | WILDER_MDM`;
`DX | DX_WILDER | DXWIN | DIRIDX | WILDER_DX`. All 25 tokens are
fresh — zero collisions with earlier rounds. `DX` alone was available
because ADX (Round 31) shipped as `ADX` / `ADXFIT` / `ADX_WIN` /
`ADXREG` / `DIRECTIONAL_INDEX` / `WILDERADX` and never claimed the
bare `DX` token.

The research packet emits fresh sub-blocks 2.323 PLUS_DI, 2.324
MINUS_DI, 2.325 PLUS_DM, 2.326 MINUS_DM, 2.327 DX after the existing
2.318-2.322 Round 66 blocks; INGESTED shifts 2.323 → 2.328 and Sector
peer 2.324 → 2.329. Envelope paragraph bumps "~92-176 KB" → "~93-177
KB" with a description chain of the five new primitives prepended.

## Consequences

- Packet scope grows by up to 10 k/v rows per symbol when all five
  snapshots are populated — roughly +230 bytes for PLUS_DI (+DI,
  +DI prev, −DI ref, ATR, close), +230 bytes for MINUS_DI (mirror),
  +260 bytes for PLUS_DM (+DM raw, +DM smoothed, +DM smoothed prev,
  up, down, close), +260 bytes for MINUS_DM (mirror), +240 bytes for
  DX (+DI, −DI, DX, DX prev, close) — for a typical +1.22 KB per
  symbol.
- Schema is strictly additive; old peers running v68 continue to
  work (new tables are absent but none of the old tables change).
  LAN sync skips unknown tables via the whitelist.
- All five indicators share the HP cache with zero additional network
  dependencies. All five require n≥16 bars (period + 2 for Wilder
  seed + prev-bar lookback). The shared `compute_dmi_series` helper
  makes five compute_* calls cost 5·O(n) memory but only 5·O(n) time
  instead of what a naive implementation would cost — and guarantees
  DX[n-1] equals `100·|+DI−−DI|/(+DI+−DI)` computed from the same
  series by construction.
- Like Round 66 + earlier rounds, the five roundtrip + five compute
  tests guard against regressions.

## Verification

1. **Engine tests:** `cargo test --package typhoon-engine --lib`
   passes with +10 Round 67 tests over Round 66's count (1405 total
   including Round 67 additions).
2. **Native build:** `cargo build --package typhoon-native` completes
   cleanly with no warnings/errors.
3. **Unique palette tokens:** All 25 Round 67 palette tokens fresh —
   zero collisions with earlier rounds (verified against the 25 Round
   66 tokens and the cumulative R60..R65 set).
4. **LAN sync whitelist:** All five `research_*` tables registered
   in `SYNCABLE_TABLES`, `create_table_sql`, and
   `table_timestamp_column`; incremental sync uses the `updated_at`
   column.
5. **DX ≡ f(+DI, −DI) identity:** the dx_compute_oscillating test
   asserts `snap.dx == 100·|+DI − −DI|/(+DI + −DI)` when the inputs
   are non-degenerate — guarding against the shared-helper drift
   risk.

## Packet envelope delta

Before Round 67: packet emitted 126 k/v rows across Round 60 + Round
61 + Round 62 + Round 63 + Round 64 + Round 65 + Round 66 additions.
After Round 67: 136 k/v rows when all forty Round 60..67 additions
populate, typical +1.22 KB per symbol on top of the +1.05 KB Round
66 added, +1.45 KB Round 65 added, +1.45 KB Round 64 added, +1.45 KB
Round 63 added, +1.45 KB Round 62 added, +1.40 KB Round 61 added,
and +1.46 KB Round 60 added — bringing the observed per-symbol
envelope from ~92-176 KB to ~93-177 KB.
