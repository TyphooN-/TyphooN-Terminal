# ADR-177: Godel Parity Round 65 — MIDPRICE / APO / MOM / SAREXT / ADXR

**Status:** Accepted
**Date:** 2026-04-18
**Supersedes/extends:** ADR-108 through ADR-176
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Parity classification

| Feature | Godel Terminal documented | TA-Lib primitive | Research packet | egui popup | Chart overlay |
|---|---|---|---|---|---|
| MIDPRICE | No | Yes (`MIDPRICE`) | Yes | Yes | No (deferred — ADR-188) |
| APO | No | Yes (`APO`) | Yes | Yes | No (deferred — ADR-188) |
| MOM | No | Yes (`MOM`) | Yes | Yes | No (deferred — ADR-188) |
| SAREXT | No | Yes (`SAREXT`) | Yes | Yes | No (deferred — ADR-188) |
| ADXR | No | Yes (`ADXR`) | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** pure TA-Lib — five TA-Lib-only primitives covering raw range-midpoint (`MIDPRICE`), absolute price oscillator (`APO`), raw momentum (`MOM`), extended asymmetric parabolic SAR (`SAREXT`), and ADX Rating (`ADXR`).

## Context

Round 64 (ADR-176) shipped LINEARREG / LINEARREG_ANGLE / HT_DCPHASE /
HT_SINE / HT_PHASOR. Round 65 continues the additive indicator cadence
with five more TA-Lib canonical surfaces covering range-midpoint
calculation, price-oscillator differences, raw momentum, extended
parabolic SAR with asymmetric acceleration factors, and the ADX Rating
smoothing variant of ADX.

1. **No MIDPRICE snapshot.** TA-Lib's MIDPRICE function computes the
   midpoint of the highest-high and lowest-low over a 14-bar window:
   `midprice = (HHV(H, 14) + LLV(L, 14)) / 2`. Distinct from MIDPOINT
   (ADR-173 close-based HHV/LLV midpoint) because MIDPRICE uses raw
   high/low extremes for a true range-midpoint line that tracks the
   symmetric center of the 14-bar trading range. Header gives
   **midprice_label** (ABOVE_BAND / UPPER_HALF / NEAR_MID / LOWER_HALF
   / BELOW_BAND / INSUFFICIENT_DATA for n<15) from close position
   normalised to the HHV-LLV band (position >1 above_band, 0.5..1
   upper_half, 0.4..0.6 near_mid, 0..0.4 lower_half, <0 below_band).

2. **No APO snapshot.** TA-Lib's Absolute Price Oscillator is
   `APO = EMA(close, 12) − EMA(close, 26)`. Distinct from MACD (same
   math plus signal line and histogram components) and PPO (percent-
   based form dividing by the slow EMA) because APO reports the raw
   price-unit oscillator without additional smoothing — useful as a
   lightweight trend-momentum diagnostic alongside fuller MACD. Header
   gives **apo_label** (BULL_STRONG / BULL / NEUTRAL / BEAR /
   BEAR_STRONG / INSUFFICIENT_DATA for n<27) from apo magnitude as a
   percentage of close (>0.5% bull_strong, >0.1% bull, <-0.1% bear,
   <-0.5% bear_strong, else neutral).

3. **No MOM snapshot.** TA-Lib's raw momentum function computes
   `MOM = close − close[n − 10]` — the unscaled 10-bar price
   difference. Distinct from ROC (which normalises by the older value
   to report a ratio) and MOMENTUM_12_1 (prior 12-bar variant) because
   MOM reports the raw price-unit change without ratio conversion,
   preserving absolute magnitude for price-level-aware strategies.
   Header gives **mom_label** (BULL_STRONG / BULL / NEUTRAL / BEAR /
   BEAR_STRONG / INSUFFICIENT_DATA for n<12) from mom_pct magnitude
   (>3% bull_strong, >1% bull, <-1% bear, <-3% bear_strong).

4. **No SAREXT snapshot.** TA-Lib's Extended Parabolic SAR exposes
   asymmetric acceleration-factor controls — separate init/step/max
   AFs for long and short directions — so traders can tune trailing
   behaviour differently on rallies versus selloffs. Distinct from
   SAR because SAREXT allows `af_init_long ≠ af_init_short`,
   `af_step_long ≠ af_step_short`, and `af_max_long ≠ af_max_short`.
   Default values (0.02 / 0.02 / 0.20 for both directions) match
   standard PSAR, with a start_value override for seeding. Header
   gives **sarext_label** (BULL_STRONG / BULL / BEAR / BEAR_STRONG /
   INSUFFICIENT_DATA for n<4) from trend direction (up/down) and
   distance-to-price magnitude (>2% strong, else normal).

5. **No ADXR snapshot.** TA-Lib's ADX Rating function:
   `ADXR = (ADX[n] + ADX[n − 14]) / 2` — the midpoint of current ADX
   and ADX from 14 bars prior. Distinct from ADX because ADXR applies
   a 14-bar lag average that smooths momentum transitions and reduces
   whipsaws around the 20-25 trend-strength boundary. Header gives
   **adxr_label** (STRONG_TREND / TREND / WEAK_TREND / NO_TREND /
   INSUFFICIENT_DATA for n<3·period+1=43) from adxr magnitude (>30
   strong_trend, >20 trend, >15 weak_trend, else no_trend).

## Decision

Add five optional per-symbol snapshot blocks to the Godel-parity
pipeline, each reusing the existing `research_historical_price` HP
cache and the standard research-table LAN-sync path (no new API
dependencies):

1. `research::MidpriceSnapshot` + `compute_midprice_snapshot` +
   `upsert_midprice` + `get_midprice` — serialised to
   `research_midprice`.
2. `research::ApoSnapshot` + `compute_apo_snapshot` + `upsert_apo` +
   `get_apo` — serialised to `research_apo`.
3. `research::MomSnapshot` + `compute_mom_snapshot` + `upsert_mom` +
   `get_mom` — serialised to `research_mom`.
4. `research::SarextSnapshot` + `compute_sarext_snapshot` +
   `upsert_sarext` + `get_sarext` — serialised to `research_sarext`.
5. `research::AdxrSnapshot` + `compute_adxr_snapshot` + `upsert_adxr`
   + `get_adxr` — serialised to `research_adxr`.

Schema version bumps to v67 via `create_research_tables_v67` which
wraps v66 and adds five new CREATE TABLE stanzas (plus indexes on
`updated_at`). All five tables register in `SYNCABLE_TABLES`,
`create_table_sql`, and `table_timestamp_column` so LAN sync
incrementally propagates them to peer terminals.

Native wiring adds five `BrokerCmd::Compute*Snapshot` variants, five
`BrokerMsg::*SnapshotMsg` variants, five `show_*_win` / `*_symbol` /
`*_snapshot` / `*_loading` field tuples on `App`, five tokio-spawned
broker handlers (load HP cache → compute → upsert → emit msg), five
palette alias blocks, five packet-emitter blocks under section 2.313+
of the research packet, five egui windows with Use-Chart / Load-Cached
/ Compute controls plus a striped Grid summary, and five `BrokerMsg`
match arms. SAREXT window is sized wider (620×320) to display all six
acceleration-factor parameters alongside trend state.

Palette aliases were selected to avoid collision with earlier rounds
(verified at implementation time — `MOM` was already claimed by the
existing MOMENTUM_12_1 binding, so we use `MOMRAW` as the primary
alias for MOM):
`MIDPRICE | MID_PRICE | MIDBAR | MIDBARPRICE | HLMIDPRICE`;
`APO | ABS_PRICE_OSC | ABSPRICEOSC | ABSPO | APOWIN`;
`MOMRAW | MOMENTUM_RAW | MOM_TA | RAWMOM | TALIB_MOM`;
`SAREXT | SAR_EXT | EXTENDED_SAR | SAREXTENDED | PSAR_EXT`;
`ADXR | ADX_RATING | ADX_R | ADXRATING | ADX_RANK`. All 25 tokens
are fresh — zero collisions with earlier rounds after the MOMRAW
resolution.

The research packet emits fresh sub-blocks 2.313 MIDPRICE, 2.314 APO,
2.315 MOM, 2.316 SAREXT, 2.317 ADXR after the existing 2.312 HT_PHASOR
sub-block; INGESTED renumbers 2.313 → 2.318 and Sector peer 2.314 →
2.319. Envelope paragraph bumps "~90-173 KB" → "~91-175 KB" with a
description chain of the five new indicators prepended.

## Consequences

- Packet scope grows by 10 k/v rows per symbol when all five
  snapshots are populated — roughly +240 bytes for MIDPRICE (HHV/LLV
  + midprice + position), +260 bytes for APO (fast/slow EMA + apo +
  apo_prev), +220 bytes for MOM (mom + mom_prev + mom_pct), +320
  bytes for SAREXT (6 AF params + sar_value + extreme_point +
  distance_pct + trend state), +280 bytes for ADXR (adx_now +
  adx_prior + adxr + adxr_prev) — for a typical +1.45 KB per symbol.
- Schema is strictly additive; old peers running v66 continue to
  work (new tables are absent but none of the old tables change).
  LAN sync skips unknown tables via the whitelist.
- All five indicators share the HP cache with zero additional network
  dependencies. ADXR builds an internal DX and ADX series inline
  rather than calling `compute_adx_snapshot` because the existing ADX
  helper returns only the scalar snapshot, whereas ADXR needs the
  lagged `adx[n − period]` value from within the series.
- Like Round 64 + earlier rounds, the five roundtrip + five compute
  tests guard against regressions.

## Verification

1. **Engine tests:** `cargo test --package typhoon-engine --lib`
   passes 1386 tests (+10 from Round 64's 1376).
2. **Native build:** `cargo build --package typhoon-native` completes
   cleanly in 9m 17s with no warnings/errors.
3. **Unique palette tokens:** All 25 Round 65 palette tokens fresh —
   zero collisions with earlier rounds after the `MOM → MOMRAW`
   rename avoiding the existing MOMENTUM_12_1 binding.
4. **LAN sync whitelist:** All five `research_*` tables registered
   in `SYNCABLE_TABLES`, `create_table_sql`, and
   `table_timestamp_column`; incremental sync uses the `updated_at`
   column.

## Packet envelope delta

Before Round 65: packet emitted 106 k/v rows across Round 60 + Round
61 + Round 62 + Round 63 + Round 64 additions. After Round 65: 116
k/v rows when all thirty Round 60..65 additions populate, typical
+1.45 KB per symbol on top of the +1.45 KB Round 64 added, +1.45 KB
Round 63 added, +1.45 KB Round 62 added, +1.40 KB Round 61 added, and
+1.46 KB Round 60 added — bringing the observed per-symbol envelope
from ~90-173 KB to ~91-175 KB.
