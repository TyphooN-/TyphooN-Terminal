# ADR-171: Godel Parity Round 59 — DEMARKER / GATOR / BW_MFI / VWMA / STDDEV

**Status:** Accepted
**Date:** 2026-04-17
**Supersedes/extends:** ADR-108 through ADR-170
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Context

Round 58 (ADR-170) shipped FRACTALS / IFT_RSI / MAMA / COG / DIDI.
Round 59 continues the additive indicator cadence with five more
canonical surfaces along the bounded-oscillator, Bill-Williams-system,
volume-weighted-smoother, and regime-classifier axes. Two of these
(GATOR, BW_MFI) are Bill Williams contributions that complete the
Williams/Profitunity toolkit alongside ALLIGATOR (ADR-167), FRACTALS
(ADR-170) and AWESOME (ADR-117/134); DEMARKER rounds out the classic
bounded-oscillator family; VWMA fills the volume-weighted-MA gap
distinct from VWAP (ADR-155); STDDEV provides a dedicated rolling
dispersion surface with a long-baseline regime classifier.

1. **No DeMarker (DeM) snapshot.** Tom DeMark's bounded [0,1]
   momentum oscillator. Over N=14, DeMax[i] = max(high[i]−high[i−1],
   0) and DeMin[i] = max(low[i−1]−low[i], 0); summing and taking
   `DeM = ΣDeMax / (ΣDeMax + ΣDeMin)` weights recent highs vs recent
   lows so sustained up-legs push DeM toward 1 and sustained
   down-legs toward 0. Canonical thresholds: ≥0.7 overbought, ≤0.3
   oversold. Distinct from RSI (Wilder smoothing of close-based
   gains/losses, ADR-108), from Williams %R (range-position of close,
   ADR-148), from STOCHRSI (stochastic of RSI, ADR-137), and from
   CCI (ADR-116). Header gives **demarker_label** (OVERBOUGHT / BULL
   / NEUTRAL / BEAR / OVERSOLD / INSUFFICIENT_DATA for n<16) derived
   from DeM magnitude plus bar-over-bar direction.

2. **No Bill Williams Gator Oscillator snapshot.** Williams's
   companion to the Alligator (ADR-167) that visualizes how the
   three shifted SMMAs diverge or converge. `upper = |jaws − teeth|`
   plotted above zero and `lower = −|teeth − lips|` plotted below
   zero, where jaws = SMMA₁₃ shifted 8 bars, teeth = SMMA₈ shifted
   5, lips = SMMA₅ shifted 3. Four life phases: SLEEPING (both bars
   smaller than 0.05% of price — alligator asleep, no trend),
   AWAKENING (one bar growing, one shrinking — initial phase of a
   new trend), EATING (both bars growing — trend feeding), and
   SATED (both bars shrinking — trend exhausting, exit zone).
   Distinct from ALLIGATOR (the raw MA triplet) and from every
   MA-spread oscillator on the shipped list. Header gives
   **gator_label** (SLEEPING / AWAKENING / EATING / SATED /
   INSUFFICIENT_DATA for n<23) derived from the sign of the
   bar-over-bar change in |upper| and |lower|.

3. **No Bill Williams Market Facilitation Index (BW_MFI)
   snapshot.** Measures how much price moved per unit of volume:
   `mfi = (high − low) / volume` tick-scaled, then classifies each
   bar by comparing current MFI and volume to the prior bar's values,
   producing four colored dots: GREEN (MFI up, volume up — genuine
   strong move), FADE (MFI down, volume down — interest fading),
   FAKE (MFI up, volume down — false breakout to be faded) and
   SQUAT (MFI down, volume up — indecision battle, often precedes
   reversal). Distinct from Chaikin's MFI (money-flow-volume-based,
   ADR-148), which uses a rolling 14-bar ratio, not the 2-bar
   color classification here. Header gives **bwmfi_label** (GREEN /
   FADE / FAKE / SQUAT / INSUFFICIENT_DATA for n<2) plus the
   **bwmfi_color** bit.

4. **No Volume Weighted Moving Average (VWMA) snapshot.** Simple
   moving average of close weighted by volume: `vwma = Σ(close·vol)
   / Σ(vol)` over N=20. High-volume closes dominate the average, so
   VWMA diverges from the plain SMA when recent volume spikes align
   (or don't) with the price direction, providing an
   institutional-footprint smoother. The core signal is the VWMA−SMA
   spread: positive when big volume aligns with higher prices,
   negative when big volume aligns with lower prices. Distinct from
   VWAP (session-anchored, ADR-155), from PVI/NVI (volume-trigger
   MA switches, ADR-141), and from every other fixed-length MA on
   the shipped list (SMA, EMA, HMA, DEMA, ALMA, KAMA, MAMA). Header
   gives **vwma_label** (BULL / WEAK_BULL / NEUTRAL / WEAK_BEAR /
   BEAR / INSUFFICIENT_DATA for n<21) derived from the close vs
   VWMA vs SMA ordering.

5. **No rolling sample Standard Deviation snapshot with regime
   classifier.** Returns the mean, variance, and sample stddev of
   close over N=20, plus the 252-day annualized stddev (note: this
   uses price-level stddev, not log-return — hence distinct from
   REALIZED_VOL and EWMAVOL). The `regime_label` compares current
   N=20 stddev against a trailing 60-bar stddev: HIGH_VOL when
   current >1.5× long, LOW_VOL when <0.67×, MID_VOL otherwise. The
   dual-window ratio is the novel contribution — the raw stddev is
   trivially derivable, but pre-classifying by the regime ratio
   saves operators a second lookup. Distinct from EWMAVOL
   (exponentially-weighted, ADR-158), from REALIZED_VOL
   (return-based), and from Parkinson / Garman-Klass / Rogers-Satchell
   (range-based). Header gives **regime_label** (HIGH_VOL / MID_VOL
   / LOW_VOL / INSUFFICIENT_DATA for n<60).

## Decision

Ship Round 59 as additive-only — no breaking changes to any existing
surface, schema, or LAN sync protocol.

### Engine (`engine/src/core/research.rs`)

Add five snapshot structs after `DidiSnapshot`:

- `DemarkerSnapshot { symbol, as_of, bars_used, length, demax_sum,
  demin_sum, demarker_value, demarker_prev, last_close,
  demarker_label, note }`
- `GatorSnapshot { symbol, as_of, bars_used, jaw_length, teeth_length,
  lips_length, upper_bar, lower_bar, upper_prev, lower_prev,
  last_close, gator_label, note }`
- `BwMfiSnapshot { symbol, as_of, bars_used, mfi_value, mfi_prev,
  volume, volume_prev, last_close, bwmfi_color, bwmfi_label, note }`
- `VwmaSnapshot { symbol, as_of, bars_used, length, vwma_value,
  sma_value, vwma_prev, spread, spread_ratio, last_close,
  vwma_label, note }`
- `StddevSnapshot { symbol, as_of, bars_used, length, long_length,
  mean, variance, stddev, stddev_long, cv, annualized, last_close,
  regime_label, note }`

Five compute functions:

- `compute_demarker_snapshot` — sum DeMax/DeMin over 14, ratio
  `ΣDeMax / (ΣDeMax + ΣDeMin)`; label on 0.3 / 0.5 / 0.7 thresholds
  plus bar-over-bar slope.
- `compute_gator_snapshot` — reuses the Alligator SMMA helper with
  jaw/teeth/lips lengths 13/8/5 and shifts 8/5/3; derives upper/lower
  bars and their prevs; label from growth-direction pairs and SLEEPING
  threshold at 0.05% of close.
- `compute_bw_mfi_snapshot` — single-bar `(high − low)/volume × 1e6`;
  classify against prior bar's MFI and volume for the 4-color map.
- `compute_vwma_snapshot` — rolling Σ(close·vol)/Σ(vol) and Σ(close)/N
  over N=20; spread and spread_ratio; label from close/VWMA/SMA
  ordering.
- `compute_stddev_snapshot` — sample stddev over N=20 and N=60;
  derive mean, variance, cv, annualized; classify regime by N20/N60
  ratio thresholds 0.67 / 1.5.

Schema v61 wraps v60 with five new tables
(`research_demarker / research_gator / research_bw_mfi /
research_vwma / research_stddev`) + timestamped indexes. Ten
upsert/get helpers follow the standard pattern.

### LAN sync (`engine/src/core/lan_sync.rs`)

Five entries under "// ── ADR-171 Round 59 ────" in `SYNCABLE_TABLES`;
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
6. 5 palette alias blocks — DEMARKER/DEM/DEMARK/DEMARKER_WIN/
   DEMARKER_RESEARCH, GATOR_OSC/GATOR_OSCILLATOR/GATOR_WIN/BW_GATOR/
   BILL_WILLIAMS_GATOR (note: plain "GATOR" is already bound to the
   ALLIGATOR research window, so this round uses suffixed aliases),
   BW_MFI/BWMFI/MARKET_FACILITATION_INDEX/BILL_WILLIAMS_MFI/BWMFI_WIN,
   VWMA/VWMA_WIN/VOL_WEIGHTED_MA/VOLUME_WEIGHTED_MA/VWMA_RESEARCH,
   STDDEV/STD_DEV/STANDARD_DEVIATION/ROLLING_STDDEV/STDDEV_WIN
7. 5 packet emitters (2.283–2.287 sub-blocks) in packet builder
8. 5 egui windows with Use-Chart / Load-Cached / Compute controls
   and striped summary grids
9. 5 BrokerMsg result handlers

### Documentation

- This ADR
- `docs/RESEARCH_PACKET.md` adds five new sub-blocks 2.283–2.287
  (DEMARKER, GATOR, BW_MFI, VWMA, STDDEV); INGESTED renumbers 2.283
  → 2.288 and Sector peer 2.284 → 2.289; envelope paragraph updated
  from "~84–161 KB" to "~85–163 KB"

### Alternatives considered

- **Ship COPPOCK (Coppock Curve) in this round.** Deferred for a
  second round — while Coppock is a canonical long-term signal,
  bundling it with Round 59's bar-level surfaces would again mix
  timescales. Best shipped with HEIKEN_ASHI and other multi-timeframe
  surfaces in a dedicated bundle.
- **Ship Keltner/Bollinger squeeze detector as a new struct.**
  Rejected — BBSQUEEZE (ADR-147) already covers this; adding a
  second near-duplicate would be churn.
- **Ship MESA Sine Wave (Ehlers).** Deferred — requires the same
  Hilbert-transform infrastructure used in MAMA (ADR-170), but its
  sine-wave output is better visualized as a 2-line overlay rather
  than a single-row snapshot, needing a dedicated emission path.

## Consequences

### Positive

- **Bill Williams toolkit completed** (GATOR + BW_MFI) — the
  Profitunity trade-classifier trio (FRACTALS + ALLIGATOR + GATOR +
  AWESOME + BW_MFI) is now fully represented. Entry/exit rules that
  cite Williams's original books can be evaluated end-to-end on the
  shipped data.
- **Bounded momentum oscillator family rounded out** (DEMARKER) —
  fills the DeMark niche alongside RSI, Williams %R, CCI, STOCHRSI,
  QQE, CRSI, IFT_RSI. Each uses a different normalization and
  threshold convention, so operators can now triangulate.
- **Volume-weighted MA shipped** (VWMA) — distinct from VWAP
  (session-anchored) and from volume-trigger switchers (PVI/NVI).
  The VWMA−SMA spread surface is a direct institutional-footprint
  read.
- **Dedicated rolling stddev + regime classifier** (STDDEV) —
  distinct from all range-based and return-based vol surfaces
  previously shipped. Useful as a normalization denominator for
  ad-hoc z-scores.
- +10 engine tests (5 roundtrip + 5 compute_oscillating) maintaining
  the property that every new surface has both persistence and
  compute-determinism coverage.

### Negative / Risks

- **GATOR's "SLEEPING" threshold is a heuristic.** We use `tiny =
  last_close · 0.0005` (0.05% of price) — above which at least one
  bar must rise to exit SLEEPING. On very-low-volatility small-caps
  or stablecoins this could keep the indicator stuck in SLEEPING
  despite clear MA-triplet ordering. The threshold is a reasonable
  default for S&P 500 equities at typical daily volatilities; symbols
  outside that regime may need tuning in a future config pass.
- **BW_MFI's classification depends on noisy volume data.** Yahoo
  HP cache occasionally reports zero or missing volume for holiday
  / half-day / pre-IPO bars. The implementation guards against
  division by zero but the resulting MFI will not match platforms
  that use intraday tick volume.
- **VWMA requires non-zero volume.** When volume data is missing
  (warrants, OTC penny stocks, some ETFs in thin sessions), VWMA
  falls back to the plain SMA — the label will correctly indicate
  NEUTRAL/WEAK in these cases, but the VWMA value will be
  indistinguishable from SMA. Operators should cross-reference the
  `spread` field; a spread of exactly zero indicates the fallback
  happened.
- **STDDEV is price-level, not log-return.** This means absolute
  dollar stddev, which scales with price. Useful for same-symbol
  regime comparison but NOT comparable across symbols at different
  price levels. For cross-symbol comparison use REALIZED_VOL or
  ANNVOL (ADR-114, ADR-141), which normalize by mean-price or use
  log returns.
- **"GATOR" palette token clash.** The plain "GATOR" alias was
  already bound to the ALLIGATOR research window (ADR-167). Rather
  than rename the existing binding, the new Gator-Oscillator window
  uses suffixed aliases: GATOR_OSC, GATOR_OSCILLATOR, GATOR_WIN,
  BW_GATOR, BILL_WILLIAMS_GATOR.

### Neutral

- No new API dependencies. All five surfaces reuse the existing
  `research_historical_price` HP cache.
- With Round 59 the Bill-Williams complete set is achieved; any
  future rounds targeting this school (market facilitation decile
  dots, wiseman fractals, etc.) are stylistic rather than structural.

### Paid-API gap

None introduced in this round. All five surfaces are HP-derived
and work entirely from the existing free-data cache.

## Verification

- `cargo test -p typhoon-engine --lib`: 1326 tests pass (+10 from
  1316).
- `cargo build -p typhoon-native`: clean build in 3m 19s, no new
  warnings (after adjusting the GATOR palette aliases to avoid a
  name clash with the existing ALLIGATOR binding).
- `docs/RESEARCH_PACKET.md`: five new sub-blocks 2.283–2.287 added
  (INGESTED and Sector peer renumbered); envelope updated.

## Packet envelope delta

| Surface | Field count | Approx bytes when populated | Free / Paid |
|---|---|---|---|
| DEMARKER | 10 | ~260 | Free (HP cache) |
| GATOR | 12 | ~320 | Free (HP cache) |
| BW_MFI | 10 | ~260 | Free (HP cache) |
| VWMA | 11 | ~290 | Free (HP cache) |
| STDDEV | 13 | ~360 | Free (HP cache) |
| **Round 59 total** | **56 fields** | **≈1.49 KB** | **Free** |

Envelope: 84–161 KB → 85–163 KB single-symbol; 830–1600 KB →
840–1620 KB for the canonical 10-symbol basket.
