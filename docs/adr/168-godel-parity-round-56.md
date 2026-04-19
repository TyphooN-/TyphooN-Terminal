# ADR-168: Godel Parity Round 56 — GMMA / MAENV / ADL / VHF / VROC

**Status:** Accepted
**Date:** 2026-04-17
**Supersedes/extends:** ADR-108 through ADR-167
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Parity classification

| Feature | Godel Terminal documented | TA-Lib primitive | Research packet | egui popup | Chart overlay |
|---|---|---|---|---|---|
| GMMA | Canonical (all terminals) | No | Yes | Yes | No (deferred — ADR-188) |
| MAENV | Canonical (all terminals) | No | Yes | Yes | No (deferred — ADR-188) |
| ADL | Canonical (all terminals) | Yes (`AD`) | Yes | Yes | No (deferred — ADR-188) |
| VHF | Canonical (all terminals) | No | Yes | Yes | No (deferred — ADR-188) |
| VROC | Canonical (all terminals) | No | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** canonical technical indicators (Guppy Multiple MA fan, MA envelopes, Chaikin Accumulation/Distribution line via TA-Lib `AD`, Vertical Horizontal Filter, Volume ROC).

## Context

Round 55 (ADR-167) shipped SMMA / ALLIGATOR / CRSI / SEB / IMI. Round
56 continues the additive indicator cadence with five more canonical
surfaces filling distinct coverage holes in the multi-MA-fan,
percentage-channel, cumulative-money-flow, trend-vs-range, and
volume-momentum axes.

1. **No Guppy Multiple MA (GMMA) snapshot.** Daryl Guppy's MMA is a
   fan of twelve EMAs split into a **short-term trader group**
   (periods 3, 5, 8, 10, 12, 15) and a **long-term investor group**
   (30, 35, 40, 45, 50, 60). When the short group is above and fanned
   wide with the long group below and parallel, a strong uptrend is
   confirmed; compression in both groups signals an imminent move.
   Distinct from ALLIGATOR (ADR-167, 3-line SMMA system), from every
   single-MA surface (SMA/EMA/DEMA/TEMA/KAMA/SMMA/HMA etc.), and from
   any dual-MA crossover. Header gives **gmma_label**
   (STRONG_UPTREND / UPTREND / COMPRESSION / DOWNTREND /
   STRONG_DOWNTREND / NEUTRAL / INSUFFICIENT_DATA for n<62) derived
   from group ordering, fan state, and group-gap percentage.

2. **No Moving Average Envelope (MAENV) snapshot.** Classical
   technician's channel built from an SMA ± fixed percentage bands:
   `upper = MA·(1+k)`, `lower = MA·(1−k)`. Distinct from Bollinger
   (stdev-based, ADR-108), Keltner (ATR-based, ADR-135), Donchian
   (rolling high/low, ADR-149), SEB (regression-residual, ADR-167),
   and STARC (SMA ± k·ATR). MAENV is the only "fixed pct, no vol
   input" channel and is still the default envelope on every
   technician's charting platform (StockCharts, MetaStock, Yahoo
   Finance, Godel terminal). Header gives **maenv_label**
   (ABOVE_BAND / UPPER_HALF / NEUTRAL / LOWER_HALF / BELOW_BAND /
   INSUFFICIENT_DATA for n<21) derived from close position within
   the band.

3. **No Chaikin Accumulation/Distribution Line (ADL) snapshot.**
   Marc Chaikin's ADL is a cumulative running total of
   `money_flow_multiplier · volume`, where
   `MFM = ((close − low) − (high − close)) / (high − low)`. Bars
   that close in the upper half of their range contribute positive
   money flow (accumulation); bars closing in the lower half
   contribute negative money flow (distribution). Divergences
   between ADL slope and price are classic Chaikin signals — rising
   ADL with flat/down price = bullish divergence; falling ADL with
   flat/up price = bearish divergence. Distinct from OBV (raw signed
   volume, ADR-108) which is range-agnostic, from CMF (ADR-140) which
   is a ranged ratio over N bars rather than a cumulative running
   total, from KLINGER (ADR-152) which uses a dual-EMA transformation
   of volume force, from PVT (ADR-164) which uses ROC·volume rather
   than MFM·volume, and from ADOSC (Chaikin Oscillator, difference of
   EMAs on ADL — not shipped). Header gives **adl_label**
   (STRONG_ACCUMULATION / ACCUMULATION / NEUTRAL / DISTRIBUTION /
   STRONG_DISTRIBUTION / INSUFFICIENT_DATA for n<22) derived from
   the 20-bar OLS slope of ADL normalised by last close.

4. **No Vertical Horizontal Filter (VHF) snapshot.** Adam White's
   1991 VHF measures **trendiness vs ranging**:
   `VHF = (HHV_N − LLV_N) / Σ|Δclose|` over N=28 bars. High VHF
   (>0.5) means price is grinding in one direction with little
   back-and-forth (trending); low VHF (<0.3) means price is chopping
   around the same range (ranging). Distinct from ADX (ADR-108, trend
   strength from +DI/-DI differences), from CHOP (ADR-141, log10 of
   range/sum-of-TR), from AROON (ADR-140, positional HHV/LLV timing),
   and from VI (Vortex, ADR-150). VHF is the canonical "am I in a
   trend right now, or a range?" filter used to gate trend-following
   strategies. Header gives **vhf_label** (STRONG_TREND / TREND /
   NEUTRAL / RANGING / STRONG_RANGING / INSUFFICIENT_DATA for n<30).

5. **No Volume Rate of Change (VROC) snapshot.** Strict two-point
   volume delta: `VROC = (V_now − V_{now−N}) / V_{now−N} · 100`
   with N=14. Spikes mark unusual participation (news, earnings,
   breakouts); persistent positive VROC with rising price confirms
   trend. Distinct from RelVol (ADR-139, current-vs-long-horizon
   average), from NVol (ADR-148, current-vs-20-day-median), and from
   the price-based ROC (ADR-113). VROC is the "has volume
   accelerated?" gauge used in volume-first breakout systems. Header
   gives **vroc_label** (SURGE ≥+100% / ELEVATED ≥+30% / NEUTRAL /
   QUIET ≤−20% / COLLAPSE ≤−50% / INSUFFICIENT_DATA for n<16).

## Decision

Ship Round 56 as additive-only — no breaking changes to any existing
surface, schema, or LAN sync protocol.

### Engine (`engine/src/core/research.rs`)

Add five snapshot structs after `ImiSnapshot`:

- `GmmaSnapshot { symbol, as_of, bars_used, short_ema_avg,
  long_ema_avg, short_min, short_max, long_min, long_max,
  short_compression_pct, long_compression_pct, group_gap_pct,
  last_close, gmma_label, note }`
- `MaenvSnapshot { symbol, as_of, bars_used, length, pct_band,
  upper, middle, lower, bandwidth_pct, position_pct, last_close,
  maenv_label, note }`
- `AdlSnapshot { symbol, as_of, bars_used, adl_value, adl_prev,
  adl_sma_length, adl_sma, slope_per_bar, last_close,
  price_delta_pct, adl_label, note }`
- `VhfSnapshot { symbol, as_of, bars_used, length, highest_high,
  lowest_low, sum_abs_delta, vhf_value, vhf_prev, last_close,
  vhf_label, note }`
- `VrocSnapshot { symbol, as_of, bars_used, length, volume_now,
  volume_then, vroc_value, vroc_prev, last_close, vroc_label,
  note }`

Five compute functions:

- `compute_gmma_snapshot` — EMA recursion for each of the 12
  periods; aggregate group stats; label by fan/compression/gap.
- `compute_maenv_snapshot` — SMA(20) ± 2.5% bands.
- `compute_adl_snapshot` — cumulative MFM·V, 20-bar SMA of ADL,
  20-bar OLS slope, label by slope/close.
- `compute_vhf_snapshot` — (HHV−LLV)/Σ|Δclose| over 28 bars.
- `compute_vroc_snapshot` — (V_now−V_then)/V_then · 100 over 14
  bars.

Schema v58 wraps v57 with five new tables
(`research_gmma / research_maenv / research_adl / research_vhf /
research_vroc`) + timestamped indexes. Ten upsert/get helpers follow
the standard pattern.

### LAN sync (`engine/src/core/lan_sync.rs`)

Five entries under "// ── ADR-168 Round 56 ────" in `SYNCABLE_TABLES`;
five CREATE TABLE stanzas in `create_table_sql()`; five
`Some("updated_at")` entries in `table_timestamp_column()`.

### Native (`native/src/app.rs`)

Nine-section additive pattern as per prior rounds:

1. 5 new `BrokerCmd` variants
2. 5 new `BrokerMsg` variants
3. 20 struct fields (`show_*_win`, `*_win_symbol`, `*_win_snapshot`,
   `*_win_loading` × 5)
4. 20 default initialisers
5. 5 tokio compute handlers via `shared_cache_broker`
6. 5 palette alias blocks — GMMA/GUPPY/GUPPY_MMA/GUPPY_MULTIPLE_MA,
   MAENV/MA_ENVELOPE/MOVING_AVG_ENVELOPE/MA_ENV,
   ADL/ACCUM_DIST/ACCUMULATION_DISTRIBUTION/CHAIKIN_ADL/AD_LINE,
   VHF/VERTHORZ/VERT_HORZ_FILTER/VERTICAL_HORIZONTAL_FILTER,
   VROC/VOLUME_ROC/VOL_ROC/VOLUME_RATE_OF_CHANGE
7. 5 packet emitters (2.268–2.272 sub-blocks) in packet builder
8. 5 egui windows with Use-Chart / Load-Cached / Compute controls
   and striped summary grids
9. 5 BrokerMsg result handlers

### Documentation

- This ADR
- `docs/RESEARCH_PACKET.md` adds five new sub-blocks 2.268–2.272
  (GMMA, MAENV, ADL, VHF, VROC); INGESTED renumbers 2.268 → 2.273
  and Sector peer 2.269 → 2.274; envelope paragraph updated from
  "~81–155 KB" to "~82–157 KB"

### Alternatives considered

- **Ship Williams %R as WPR.** Rejected — the existing `WILLR`
  surface (ADR-134) already covers this with palette aliases
  `WILLIAMS_R / WILLIAMS_PCT_R / PERCENT_R`. Adding WPR would be
  pure duplication.
- **Ship Accumulation/Distribution Oscillator (ADOSC / Chaikin
  Osc) alongside ADL.** Deferred to a later round — ADOSC is a
  derivative of ADL (fast-EMA − slow-EMA on the cumulative line)
  and pairs naturally with its base. Shipping ADL alone in Round 56
  keeps the round focused on primary surfaces.
- **Ship Bill Williams Fractals in this round.** Deferred — Fractals
  are peak/trough markers (sequence of 5 bars with a local extreme
  at the middle) rather than a scalar-label indicator, and want
  different packet/window ergonomics than the rest of Round 56.

## Consequences

### Positive

- **Multi-MA-fan surface added** (GMMA) — the one canonical
  multi-EMA trend-gauge shipped by every charting platform in
  the Guppy ecosystem. Complements single-MA surfaces without
  duplicating them.
- **Fixed-pct channel added** (MAENV) — fills the "no vol input"
  gap in the envelope taxonomy, alongside stdev-based (Bollinger),
  ATR-based (Keltner), and regression-residual (SEB, ADR-167).
- **Cumulative money-flow line added** (ADL) — the classic
  Chaikin primary surface, complementing ranged-ratio CMF
  (ADR-140), dual-EMA Klinger (ADR-152), and ROC-based PVT
  (ADR-164).
- **Trend-vs-range filter added** (VHF) — the canonical gating
  signal for "is it worth running a trend-following strategy
  right now?" distinct from the ADX, CHOP, AROON, and VI
  families.
- **Volume-delta surface added** (VROC) — two-point volume ROC,
  complementary to RelVol (long-horizon average) and NVol
  (20-day median).
- +10 engine tests (5 roundtrip + 5 compute_oscillating)
  maintaining the property that every new surface has both
  persistence and compute-determinism coverage.

### Negative / Risks

- **GMMA warm-up is 62 bars.** The longest EMA period is 60 plus
  one bar for initialisation and one for prior value. First-run
  HP caches below this threshold produce INSUFFICIENT_DATA, which
  is surfaced via the note field.
- **ADL label thresholds are heuristic.** We normalise slope by
  last close and classify at magnitudes 100 k / 1 M times the
  close. This works well for mid-cap equities (typical daily
  volume 1 M–100 M); very-thin or very-liquid instruments may
  get systematically under/over-labelled. Documented in the help
  text.
- **MAENV pct is fixed at 2.5%.** No per-symbol volatility
  adjustment. Works well for mid-cap equities in normal regimes;
  high-vol instruments (crypto, small caps in news events) may
  need user-tunable k. Deferred to a future round.
- **VROC is noisy at the 14-bar horizon.** Zero-volume bars or
  synthetic volumes (e.g. FX aggregated feeds) will produce
  spurious surges/collapses. Users should verify volume reliability
  before trusting the label.
- **VHF rising-edge crossing from ranging to trending doesn't
  disambiguate direction.** The user must pair with a trend
  direction indicator (e.g. SMMA slope, GMMA group-gap). By
  design — VHF answers "is there a trend," not "which direction."

### Neutral

- No new API dependencies. All five surfaces reuse the existing
  `research_historical_price` HP cache.
- GMMA landed here rather than as a standalone ADR because it is
  mechanically a product of EMA recursion (already shipped
  universally) and contributes to the multi-MA-fan axis. Bundling
  it into Round 56 with its MAENV/ADL/VHF/VROC siblings avoided a
  single-surface ADR.

### Paid-API gap

None introduced in this round. All five surfaces are HP-derived
and work entirely from the existing free-data cache.

## Verification

- `cargo test -p typhoon-engine --lib`: 1296 tests pass (+10 from
  1286).
- `cargo build -p typhoon-native`: clean build in 3m 15s, no new
  warnings.
- `docs/RESEARCH_PACKET.md`: five new sub-blocks 2.268–2.272 added
  (INGESTED and Sector peer renumbered); envelope updated.

## Packet envelope delta

| Surface | Field count | Approx bytes when populated | Free / Paid |
|---|---|---|---|
| GMMA | 13 | ~320 | Free (HP cache) |
| MAENV | 12 | ~280 | Free (HP cache) |
| ADL | 11 | ~280 | Free (HP cache) |
| VHF | 11 | ~270 | Free (HP cache) |
| VROC | 10 | ~240 | Free (HP cache) |
| **Round 56 total** | **57 fields** | **≈1.39 KB** | **Free** |

Envelope: 81–155 KB → 82–157 KB single-symbol; 780–1510 KB →
790–1540 KB for the canonical 10-symbol basket.
