# ADR-176: TA-Lib Parity Round 64 — LINEARREG / LINEARREG_ANGLE / HT_DCPHASE / HT_SINE / HT_PHASOR

**Status:** Accepted
**Date:** 2026-04-17
**Supersedes/extends:** ADR-108 through ADR-175
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Parity classification

| Feature | Godel Terminal documented | TA-Lib primitive | Research packet | egui popup | Chart overlay |
|---|---|---|---|---|---|
| LINEARREG | No | Yes (`LINEARREG`) | Yes | Yes | No (deferred — ADR-188) |
| LINEARREG_ANGLE | No | Yes (`LINEARREG_ANGLE`) | Yes | Yes | No (deferred — ADR-188) |
| HT_DCPHASE | No | Yes (`HT_DCPHASE`) | Yes | Yes | No (deferred — ADR-188) |
| HT_SINE | No | Yes (`HT_SINE`) | Yes | Yes | No (deferred — ADR-188) |
| HT_PHASOR | No | Yes (`HT_PHASOR`) | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** pure TA-Lib — five TA-Lib-only primitives completing the LINEARREG family (`LINEARREG` fitted value, `LINEARREG_ANGLE` slope-in-degrees) and filling out the Hilbert-transform cycle-analysis toolkit (`HT_DCPHASE`, `HT_SINE`, `HT_PHASOR`).

## Context

Round 63 (ADR-175) shipped LINEARREG_SLOPE / HT_DCPERIOD / HT_TRENDMODE
/ ACCBANDS / STOCHF. Round 64 continues the additive indicator cadence
with five more TA-Lib canonical surfaces, completing the LINEARREG
family and filling out the Hilbert-transform cycle-analysis toolkit.
LINEARREG reports the least-squares fitted endpoint value;
LINEARREG_ANGLE converts the raw slope into a scale-invariant angle;
HT_DCPHASE reports the dominant cycle phase; HT_SINE emits the
sine/leadsine sinusoidal waveforms for cycle-turn detection; HT_PHASOR
exposes the raw I/Q analytic-signal components. All three HT_*
indicators share a single Ehlers homodyne pipeline helper
(`ehlers_hilbert_pipeline`) mirroring TA-Lib's own shared code path.

1. **No LINEARREG snapshot.** TA-Lib's LINEARREG function: least-
   squares fit of close over 14 bars, reporting the fitted value at
   the endpoint of the regression line (y_hat at the current bar).
   Distinct from LINEARREG_SLOPE (ADR-175 raw slope coefficient) and
   LINEARREG_ANGLE (slope-to-angle conversion) because LINEARREG
   reports the fitted price level — a directly-comparable smoothed-
   price line useful for trend-following entries. Header gives
   **linearreg_label** (ABOVE_TREND / NEAR_TREND / BELOW_TREND /
   INSUFFICIENT_DATA for n<15) from residual_pct (close − fitted as %
   of close; >0.5% above_trend, <-0.5% below_trend, else near_trend).

2. **No LINEARREG_ANGLE snapshot.** TA-Lib's LINEARREG_ANGLE function:
   `atan(slope) × 180/π`, converting the least-squares slope of close
   over 14 bars from price-units-per-bar into degrees from horizontal.
   Distinct from LINEARREG_SLOPE (ADR-175 raw slope in price units) and
   LINEARREG (ADR-176 fitted value) because LINEARREG_ANGLE reports a
   scale-invariant angle bounded to (-90°, 90°) — comparable across
   symbols with different price scales. Header gives **angle_label**
   (STRONG_UP / UP / FLAT / DOWN / STRONG_DOWN / INSUFFICIENT_DATA for
   n<15) from angle_deg magnitude (>30° strong_up, >5° up, <-5° down,
   <-30° strong_down).

3. **No HT_DCPHASE snapshot.** TA-Lib's HT_DCPHASE function reuses the
   same Ehlers homodyne discriminator pipeline as HT_DCPERIOD
   (ADR-175) and reports the phase of the dominant cycle at the
   current bar in degrees (0°..360°) — where 0°/360° marks a cycle
   bottom, 180° marks a cycle top, 0°-180° is the rising half, and
   180°-360° is the falling half. Distinct from HT_DCPERIOD (cycle
   length in bars) and HT_SINE (ADR-176 sinusoidal projection) because
   HT_DCPHASE reports the raw phase angle — useful for custom phase-
   based entry/exit rules. Header gives **phase_label** (CYCLE_BOTTOM
   for phase<45° or >315°, RISING 45°-135°, CYCLE_TOP 135°-225°,
   FALLING 225°-315°, INSUFFICIENT_DATA for n<64) from phase_deg.

4. **No HT_SINE snapshot.** TA-Lib's HT_SINE emits two lines: `sine =
   sin(phase)` and `leadsine = sin(phase + 45°)`. The 45° lead makes
   leadsine cross above/below sine just before the underlying price
   cycle turns, producing leading-signal crossovers at cycle bottoms
   (leadsine up through sine) and tops (leadsine down through sine).
   Distinct from HT_DCPHASE (raw phase angle) and HT_DCPERIOD (cycle
   length) because HT_SINE produces sinusoidal waveforms suitable for
   direct crossover signals. Header gives **sine_label**
   (CYCLE_TURN_UP / BULL / NEUTRAL / BEAR / CYCLE_TURN_DOWN /
   INSUFFICIENT_DATA for n<64) from crossover bit + sine-leadsine
   relationship.

5. **No HT_PHASOR snapshot.** TA-Lib's HT_PHASOR emits the raw in-
   phase (I) and quadrature (Q) components of the Hilbert transform
   directly, along with derived magnitude `sqrt(I² + Q²)` and phase
   `atan2(Q, I) × 180/π`. Distinct from HT_DCPHASE (phase after
   homodyne discrimination) and HT_SINE (sinusoidal projection)
   because HT_PHASOR exposes the raw analytic signal — useful for
   custom cycle diagnostics and phase-amplitude hybrid strategies.
   Header gives **phasor_label** (STRONG_CYCLE / CYCLE / WEAK_CYCLE /
   INSUFFICIENT_DATA for n<64) from magnitude vs 50-bar mean (>1.5×
   strong_cycle, <0.5× weak_cycle).

## Decision

Add five optional per-symbol snapshot blocks to the Godel-parity
pipeline, each reusing the existing `research_historical_price` HP
cache and the standard research-table LAN-sync path (no new API
dependencies). All three HT_* indicators share a new
`ehlers_hilbert_pipeline(closes) -> (phase, period, i1, q1)` helper
so the Hilbert I/Q computation is not re-derived three times:

1. `research::LinearregSnapshot` + `compute_linearreg_snapshot` +
   `upsert_linearreg` + `get_linearreg` — serialised to
   `research_linearreg`.
2. `research::LinearregAngleSnapshot` +
   `compute_linearreg_angle_snapshot` + `upsert_linearreg_angle` +
   `get_linearreg_angle` — serialised to `research_linearreg_angle`.
3. `research::HtDcphaseSnapshot` + `compute_ht_dcphase_snapshot` +
   `upsert_ht_dcphase` + `get_ht_dcphase` — serialised to
   `research_ht_dcphase`.
4. `research::HtSineSnapshot` + `compute_ht_sine_snapshot` +
   `upsert_ht_sine` + `get_ht_sine` — serialised to `research_ht_sine`.
5. `research::HtPhasorSnapshot` + `compute_ht_phasor_snapshot` +
   `upsert_ht_phasor` + `get_ht_phasor` — serialised to
   `research_ht_phasor`.

Schema version bumps to v66 via `create_research_tables_v66` which
wraps v65 and adds five new CREATE TABLE stanzas (plus indexes on
`updated_at`). All five tables register in `SYNCABLE_TABLES`,
`create_table_sql`, and `table_timestamp_column` so LAN sync
incrementally propagates them to peer terminals.

Native wiring adds five `BrokerCmd::Compute*Snapshot` variants, five
`BrokerMsg::*SnapshotMsg` variants, five `show_*_win` / `*_symbol` /
`*_snapshot` / `*_loading` field tuples on `App`, five tokio-spawned
broker handlers (load HP cache → compute → upsert → emit msg), five
palette alias blocks, five packet-emitter blocks under section 2.308+
of the research packet, five egui windows with Use-Chart / Load-Cached
/ Compute controls plus a striped Grid summary, and five `BrokerMsg`
match arms.

Palette aliases were selected to avoid collision with earlier rounds
(verified at implementation time — "LINREG" was already claimed by the
existing LinregWinSnapshot binding, so we use "LINEARREG_FIT" as the
second alias for LINEARREG):
`LINEARREG | LINEARREG_FIT | LINEAR_REG | LINEARREG_WIN | LINREG_FITTED`;
`LINEARREG_ANGLE | LREGANGLE | LINEAR_REG_ANGLE | LINREGANGLE | LRANGLE`;
`HT_DCPHASE | DCPHASE | HILBERT_DCPHASE | HTDCPHASE | CYCLE_PHASE`;
`HT_SINE | HTSINE | HILBERT_SINE | SINEWAVE | LEADSINE`;
`HT_PHASOR | HTPHASOR | HILBERT_PHASOR | PHASOR | IQ_COMP`. All 25
tokens are fresh — zero collisions with earlier rounds.

The research packet emits fresh sub-blocks 2.308 LINEARREG, 2.309
LINEARREG_ANGLE, 2.310 HT_DCPHASE, 2.311 HT_SINE, 2.312 HT_PHASOR
after the existing 2.307 STOCHF sub-block; INGESTED renumbers 2.308 →
2.313 and Sector peer 2.309 → 2.314. Envelope paragraph bumps
"~89-171 KB" → "~90-173 KB" with a description chain of the five new
indicators prepended.

## Consequences

- Packet scope grows by 10 k/v rows per symbol when all five
  snapshots are populated — roughly +240 bytes for LINEARREG (fitted
  + residual pair), +240 bytes for LINEARREG_ANGLE (slope + angle
  pair), +300 bytes for HT_DCPHASE (phase + period + delta), +320
  bytes for HT_SINE (sine/leadsine pair + crossover + period), +280
  bytes for HT_PHASOR (I/Q pair + derived mag/phase) — for a typical
  +1.45 KB per symbol.
- Schema is strictly additive; old peers running v65 continue to
  work (new tables are absent but none of the old tables change).
  LAN sync skips unknown tables via the whitelist.
- All five indicators share the HP cache, and HT_DCPHASE / HT_SINE /
  HT_PHASOR additionally share a single `ehlers_hilbert_pipeline`
  helper so the Hilbert I/Q computation is computed once per
  invocation rather than three times.
- Like Round 63 + earlier rounds, the five tests + five
  roundtrip/compute tests guard against regressions.

## Verification

1. **Engine tests:** `cargo test --package typhoon-engine --lib`
   passes 1376 tests (+10 from Round 63's 1366).
2. **Native build:** `cargo build --package typhoon-native` completes
   in 1m 34s (after a prior 3m 36s full build) with no warnings/errors.
   Initial build surfaced a single unreachable-pattern warning when
   "LINREG" collided with the pre-existing LinregWinSnapshot binding;
   fixed by renaming the colliding alias to "LINEARREG_FIT".
3. **Unique palette tokens:** All 25 Round 64 palette tokens fresh —
   zero collisions with earlier rounds after the LINEARREG_FIT fix.
4. **LAN sync whitelist:** All five `research_*` tables registered
   in `SYNCABLE_TABLES`, `create_table_sql`, and
   `table_timestamp_column`; incremental sync uses the `updated_at`
   column.

## Packet envelope delta

Before Round 64: packet emitted 96 k/v rows across Round 60 + Round
61 + Round 62 + Round 63 additions. After Round 64: 106 k/v rows when
all twenty-five Round 60..64 additions populate, typical +1.45 KB
per symbol on top of the +1.45 KB Round 63 added, +1.45 KB Round 62
added, +1.40 KB Round 61 added, and +1.46 KB Round 60 added —
bringing the observed per-symbol envelope from ~89-171 KB to ~90-173
KB.
