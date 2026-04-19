# ADR-175: Godel Parity Round 63 — LINEARREG_SLOPE / HT_DCPERIOD / HT_TRENDMODE / ACCBANDS / STOCHF

**Status:** Accepted
**Date:** 2026-04-17
**Supersedes/extends:** ADR-108 through ADR-174
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Parity classification

| Feature | Godel Terminal documented | TA-Lib primitive | Research packet | egui popup | Chart overlay |
|---|---|---|---|---|---|
| LINEARREG_SLOPE | No | Yes (`LINEARREG_SLOPE`) | Yes | Yes | No (deferred — ADR-188) |
| HT_DCPERIOD | No | Yes (`HT_DCPERIOD`) | Yes | Yes | No (deferred — ADR-188) |
| HT_TRENDMODE | No | Yes (`HT_TRENDMODE`) | Yes | Yes | No (deferred — ADR-188) |
| ACCBANDS | Canonical (all terminals) | No | Yes | Yes | No (deferred — ADR-188) |
| STOCHF | No | Yes (`STOCHF`) | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** mixed — TA-Lib-only primitives (`LINEARREG_SLOPE` raw least-squares slope, `HT_DCPERIOD` Hilbert dominant-cycle period, `HT_TRENDMODE` trend/cycle regime classifier, `STOCHF` unsmoothed fast stochastic) plus Acceleration Bands (Price Headley's canonical range-adaptive volatility envelope).

## Context

Round 62 (ADR-174) shipped MASSINDEX / NATR / TTM_SQUEEZE / FORCE_INDEX /
TRANGE. Round 63 continues the additive indicator cadence with five
more TA-Lib canonical surfaces along the regression-slope, Hilbert-
cycle, regime-classifier, range-adaptive-band, and unsmoothed-
stochastic axes. LINEARREG_SLOPE brings TA-Lib's raw least-squares
slope; HT_DCPERIOD brings Ehlers's homodyne discriminator for the
dominant cycle period; HT_TRENDMODE reuses that pipeline to classify
bars as trending vs cycling; ACCBANDS adds Headley's Acceleration
Bands with range-normalized volatility envelopes; STOCHF rounds out
the fast-stochastic family with TA-Lib's unsmoothed %K variant.

1. **No LINEARREG_SLOPE snapshot.** TA-Lib's LINEARREG_SLOPE function:
   least-squares slope of close over 14 bars, reporting the price-
   units-per-bar drift rate. Distinct from LINEARREG (fitted value at
   current bar) and LINEARREG_ANGLE (slope-to-angle conversion)
   because LINEARREG_SLOPE reports the raw slope coefficient — the
   instantaneous trend magnitude. Header gives **slope_label**
   (STRONG_UP / UP / FLAT / DOWN / STRONG_DOWN / INSUFFICIENT_DATA for
   n<15) from slope_pct (slope as % of close; >0.5 strong_up, >0.1 up,
   <-0.1 down, <-0.5 strong_down).

2. **No HT_DCPERIOD snapshot.** TA-Lib's HT_DCPERIOD function uses the
   Ehlers homodyne discriminator to estimate the dominant market cycle
   length in bars. The homodyne discriminator computes the I/Q
   components of the Hilbert transform on price, then the analytic
   signal's phase derivative yields the instantaneous period; the
   result is clamped to [6, 50] bars. Distinct from Ehlers's Fisher
   Transform (price normalization), MESA adaptive indicators
   (parameter tuning), and from traditional periodogram FFTs because
   HT_DCPERIOD tracks a single dominant period adaptively bar-by-bar.
   Header gives **period_label** (VERY_SHORT / SHORT / MEDIUM / LONG /
   VERY_LONG / INSUFFICIENT_DATA for n<64) from period magnitude (<10
   very_short, <16 short, >25 long, >35 very_long).

3. **No HT_TRENDMODE snapshot.** TA-Lib's HT_TRENDMODE reuses the same
   Hilbert homodyne pipeline and classifies each bar as trending (1)
   or cycling (0) via a CV-based regime classifier — when the
   coefficient of variation of the estimated period exceeds 0.15 OR
   the period is longer than 35 bars, we call it TREND, otherwise
   CYCLE. The reference TA-Lib uses phase-accumulation + sinewave-
   amplitude heuristics; we use the simpler CV-based variant because
   it matches the intent (unstable cycle → trend regime) with less
   bookkeeping. Distinct from pure oscillators (RSI, STOCH) that
   presume a cycle regime, from raw trend indicators (SMA, linreg)
   that presume a trend regime, and from ADX (directional movement)
   because HT_TRENDMODE uses the cycle-period stability as the regime
   signal. Header gives **mode_label** (TREND / CYCLE /
   INSUFFICIENT_DATA for n<64) from trendmode bit + lock_in_bars for
   recent regime persistence.

4. **No ACCBANDS snapshot.** Headley's Acceleration Bands: upper band
   = SMA-20 of `H × (1 + 4·(H-L)/(H+L))`, lower band symmetric with
   `L × (1 - 4·(H-L)/(H+L))`, middle = SMA-20 of close. The key
   adaptation vs Bollinger/Keltner is that ACCBANDS widens with high
   H-L range relative to H+L (range-normalized volatility), making
   the bands tighter during range compression and wider during range
   expansion — a pure realized-volatility adaptive envelope. Distinct
   from Bollinger (σ-based), Keltner (ATR-based), and from Donchian
   (fixed high/low lookback). Header gives **accbands_label**
   (BREAKOUT_UP / UPPER / MID / LOWER / BREAKOUT_DOWN /
   INSUFFICIENT_DATA for n<21) from close position within the band
   (close>upper breakout_up, pos>0.7 upper, pos<0.3 lower, close<lower
   breakout_down).

5. **No STOCHF snapshot.** TA-Lib's STOCHF: unsmoothed fast stochastic
   — `fastK = 100 × (close − LLV_14) / (HHV_14 − LLV_14)`, `fastD =
   SMA-3 of fastK`. Distinct from STOCH (slow stochastic: applies
   SMA-3 smoothing to fastK before the %D pass) by emitting the raw
   fastK directly, giving a more responsive but noisier oscillator.
   Useful when you want the unfiltered stochastic reading or when you
   plan to build custom smoothing downstream. Header gives
   **stochf_label** (OVERBOUGHT / BULL / NEUTRAL / BEAR / OVERSOLD /
   INSUFFICIENT_DATA for n<17) from fastK level + fastD relationship
   (fastK>80 overbought, >55 bull, <45 bear, <20 oversold).

## Decision

Add five optional per-symbol snapshot blocks to the Godel-parity
pipeline, each reusing the existing `research_historical_price` HP
cache and the standard research-table LAN-sync path (no new API
dependencies):

1. `research::LinearregSlopeSnapshot` +
   `compute_linearreg_slope_snapshot` + `upsert_linearreg_slope` +
   `get_linearreg_slope` — serialised to `research_linearreg_slope`.
2. `research::HtDcperiodSnapshot` + `compute_ht_dcperiod_snapshot` +
   `upsert_ht_dcperiod` + `get_ht_dcperiod` — serialised to
   `research_ht_dcperiod`.
3. `research::HtTrendmodeSnapshot` + `compute_ht_trendmode_snapshot` +
   `upsert_ht_trendmode` + `get_ht_trendmode` — serialised to
   `research_ht_trendmode`.
4. `research::AccbandsSnapshot` + `compute_accbands_snapshot` +
   `upsert_accbands` + `get_accbands` — serialised to
   `research_accbands`.
5. `research::StochfSnapshot` + `compute_stochf_snapshot` +
   `upsert_stochf` + `get_stochf` — serialised to `research_stochf`.

Schema version bumps to v65 via `create_research_tables_v65` which
wraps v64 and adds five new CREATE TABLE stanzas (plus indexes on
`updated_at`). All five tables register in `SYNCABLE_TABLES`,
`create_table_sql`, and `table_timestamp_column` so LAN sync
incrementally propagates them to peer terminals.

Native wiring adds five `BrokerCmd::Compute*Snapshot` variants, five
`BrokerMsg::*SnapshotMsg` variants, five `show_*_win` / `*_symbol` /
`*_snapshot` / `*_loading` field tuples on `App`, five tokio-spawned
broker handlers (load HP cache → compute → upsert → emit msg), five
palette alias blocks, five packet-emitter blocks under section 2.303+
of the research packet, five egui windows with Use-Chart / Load-Cached
/ Compute controls plus a striped Grid summary, and five `BrokerMsg`
match arms.

Palette aliases were selected to avoid collision with earlier rounds:
`LINEARREG_SLOPE | LINREG_SLOPE | LINREGSLOPE | LRSLOPE | SLOPE`;
`HT_DCPERIOD | HTDCPERIOD | DCPERIOD | HILBERT_PERIOD | CYCLE_PERIOD`;
`HT_TRENDMODE | HTTRENDMODE | TRENDMODE | HILBERT_TRENDMODE |
CYCLE_TRENDMODE`; `ACCBANDS | ACCELERATION_BANDS | ACCBAND | HEADLEY
| ACC_BANDS`; `STOCHF | STOCHFAST | FAST_STOCH | FASTSTOCH |
STOCH_FAST`. All 25 tokens are fresh — no existing bindings collided.

The research packet emits fresh sub-blocks 2.303 LINEARREG_SLOPE,
2.304 HT_DCPERIOD, 2.305 HT_TRENDMODE, 2.306 ACCBANDS, 2.307 STOCHF
after the existing 2.302 TRANGE sub-block; INGESTED renumbers 2.303 →
2.308 and Sector peer 2.304 → 2.309. Envelope paragraph bumps
"~88–169 KB" → "~89–171 KB" with a description chain of the five new
indicators prepended.

## Consequences

- Packet scope grows by 10 k/v rows per symbol when all five
  snapshots are populated — roughly +240 bytes for LINEARREG_SLOPE
  (slope pair + pct), +260 bytes for HT_DCPERIOD (period + 64-bar
  range), +280 bytes for HT_TRENDMODE (mode bit + lock-in + period),
  +310 bytes for ACCBANDS (three bands + width + position), +280
  bytes for STOCHF (fastK/fastD pair) — for a typical +1.45 KB per
  symbol.
- Schema is strictly additive; old peers running v64 continue to
  work (new tables are absent but none of the old tables change).
  LAN sync skips unknown tables via the whitelist.
- All five indicators share the HP cache, so no additional API cost.
- Like Round 62 + earlier rounds, the five tests + five
  roundtrip/compute tests guard against regressions.

## Verification

1. **Engine tests:** `cargo test --package typhoon-engine --lib`
   passes 1366 tests (+10 from Round 62's 1356).
2. **Native build:** `cargo build --package typhoon-native` completes
   in 3m 36s with no warnings/errors.
3. **Unique palette tokens:** All 25 Round 63 palette tokens fresh —
   zero collisions with earlier rounds.
4. **LAN sync whitelist:** All five `research_*` tables registered
   in `SYNCABLE_TABLES`, `create_table_sql`, and
   `table_timestamp_column`; incremental sync uses the `updated_at`
   column.

## Packet envelope delta

Before Round 63: packet emitted 86 k/v rows across Round 60 + Round
61 + Round 62 additions. After Round 63: 96 k/v rows when all twenty
Round 60 + Round 61 + Round 62 + Round 63 additions populate, typical
+1.45 KB per symbol on top of the +1.45 KB Round 62 added, +1.40 KB
Round 61 added, and +1.46 KB Round 60 added — bringing the observed
per-symbol envelope from ~88–169 KB to ~89–171 KB.
