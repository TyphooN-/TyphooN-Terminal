# ADR-170: Godel Parity Round 58 — FRACTALS / IFT_RSI / MAMA / COG / DIDI

**Status:** Accepted
**Date:** 2026-04-17
**Supersedes/extends:** ADR-108 through ADR-169
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Context

Round 57 (ADR-169) shipped KDJ / QQE / PMO / CFO / TMF. Round 58
continues the additive indicator cadence with five more canonical
surfaces along the structural-pivot-marker, bounded-oscillator,
cycle-adaptive-MA, zero-lag-oscillator, and multi-MA-crossover axes.
Two of these (MAMA, IFT_RSI) are Ehlers contributions from the
quantitative-DSP school; one (FRACTALS) is the Bill Williams
Alligator-system building block; one (DIDI) is a Brazilian-market
canonical that has never been shipped; COG rounds out the
zero-lag-oscillator axis.

1. **No Bill Williams Fractals snapshot.** A 5-bar peak/trough
   structural pivot: a bullish (up) fractal forms when a bar's high
   is strictly greater than both the two preceding bars' highs AND
   the two following bars' highs; a bearish (down) fractal is the
   symmetric construction on lows. Used as structural S/R pivots
   and as the entry building block for Williams's Alligator system.
   Distinct from ZigZag (percent-move threshold, not 5-bar geometry),
   Pivot Points (floor-trader formula over prior OHLC), and
   swing-high/swing-low variants that use different window sizes.
   Header gives **fractals_label** (UP_RECENT / DOWN_RECENT /
   BOTH_RECENT / NONE_RECENT / INSUFFICIENT_DATA) plus the most
   recent up fractal's high, down fractal's low, and their
   `bars_ago` displacements from the last bar; the `up_fractal_count`
   and `down_fractal_count` fields count all fractals in the scanned
   window.

2. **No Ehlers Inverse Fisher RSI snapshot.** Rescales RSI (ADR-108)
   to [-5, 5] via `v = 0.1·(RSI − 50)`, smooths with a 9-bar WMA,
   then applies the inverse Fisher transform `ift = (e^{2v} − 1) /
   (e^{2v} + 1)` to produce a bounded [-1, 1] oscillator. Ehlers's
   construction compresses mid-range values toward zero and expands
   extremes toward ±1, sharpening reversal signals relative to raw
   RSI. Crossings of ±0.5 are strong trend-change alerts. Distinct
   from raw RSI (ADR-108), from STOCHRSI (stochastic of RSI,
   ADR-137), from QQE (smoothed RSI with adaptive bands, ADR-169),
   and from CRSI (Connors composite, ADR-167). Header gives
   **ift_rsi_label** (STRONG_BULL / BULL / NEUTRAL / BEAR /
   STRONG_BEAR / INSUFFICIENT_DATA for n<25) derived from the IFT
   magnitude.

3. **No MESA Adaptive Moving Average (MAMA) snapshot.** Ehlers's
   phase-adaptive MA that estimates the dominant cycle period via
   a simplified Hilbert transform (in-phase and quadrature
   discriminator) and then sets α adaptively: `α = fast_limit /
   (period / 2)`, clamped to `[slow_limit, fast_limit]`. The
   companion FAMA (Following Adaptive MA) is MAMA smoothed with
   half its α. Defaults: fast_limit=0.5, slow_limit=0.05. Distinct
   from KAMA (Kaufman efficiency-ratio-based adaptive, ADR-117),
   from T3 (Tillson triple-DEMA, ADR-142), from VIDYA (Chande
   volatility-index DMA, ADR-148), and from every fixed-α EMA.
   Header gives **mama_label** (STRONG_BULL / BULL / NEUTRAL /
   BEAR / STRONG_BEAR / INSUFFICIENT_DATA for n<32) derived from
   the MAMA vs FAMA relationship and divergence magnitude.

4. **No Ehlers Center of Gravity (COG) snapshot.** A zero-lag
   oscillator built as the negative weighted centroid of the last
   N closes: `COG = -Σ_{i=0..N-1}((i+1)·close_{N-1-i}) /
   Σ_{i=0..N-1}(close_{N-1-i})` with canonical N=10. Signal line
   is a 3-bar lagged copy. Ehlers argued the sign flip plus
   weighting by recency produces an oscillator that leads
   traditional momentum by roughly one bar on average. Distinct
   from every EMA-based oscillator (MACD, TRIX, PMO), from
   LINREG-based (LINREG/CFO), and from simple ROC. Header gives
   **cog_label** (STRONG_BULL / BULL / NEUTRAL / BEAR /
   STRONG_BEAR / INSUFFICIENT_DATA for n<14) derived from COG
   minus signal.

5. **No Didi Aguiar Didi Index snapshot.** A Brazilian 3-SMA
   crossover system where three SMAs (short 3, medium 8, long 20)
   are normalized by dividing by the medium: `short_ratio =
   short_sma/medium_sma − 1`, `long_ratio = long_sma/medium_sma −
   1`. The characteristic "didi needles" pattern fires when short
   and long cross the zero line from opposite sides — BULL_NEEDLES
   when short crosses up through zero while long crosses down
   through zero, and symmetric BEAR_NEEDLES. Between needle events,
   the ordering of short, medium, and long drives the trend
   classification. Distinct from every 2-line MA crossover
   (golden/death cross), from Guppy (GMMA, 12-line fan, ADR-168),
   and from ALLIGATOR (3-line SMMA, ADR-167). Header gives
   **didi_label** (BULL_NEEDLES / BULL / NEUTRAL / BEAR /
   BEAR_NEEDLES / INSUFFICIENT_DATA for n<22) derived from the
   ratio signs and cross events.

## Decision

Ship Round 58 as additive-only — no breaking changes to any existing
surface, schema, or LAN sync protocol.

### Engine (`engine/src/core/research.rs`)

Add five snapshot structs after `TmfSnapshot`:

- `FractalsSnapshot { symbol, as_of, bars_used, window,
  last_up_high, last_up_bars_ago, last_down_low,
  last_down_bars_ago, up_fractal_count, down_fractal_count,
  last_close, fractals_label, note }`
- `IftRsiSnapshot { symbol, as_of, bars_used, rsi_length,
  wma_length, rsi_value, v_value, ift_value, ift_prev,
  last_close, ift_rsi_label, note }`
- `MamaSnapshot { symbol, as_of, bars_used, fast_limit,
  slow_limit, mama_value, fama_value, mama_prev, fama_prev,
  alpha, period, last_close, mama_label, note }`
- `CogSnapshot { symbol, as_of, bars_used, length, cog_value,
  cog_signal, cog_prev, last_close, cog_label, note }`
- `DidiSnapshot { symbol, as_of, bars_used, short_length,
  medium_length, long_length, short_ratio, long_ratio,
  short_prev, long_prev, last_close, didi_label, note }`

Five compute functions:

- `compute_fractals_snapshot` — 5-bar window scan for strict
  local maxima (high) and minima (low); label from recency
  threshold ≤10 bars.
- `compute_ift_rsi_snapshot` — RSI₁₄, 0.1·(RSI−50), WMA₉ smoothing,
  inverse Fisher transform; label on sign + magnitude.
- `compute_mama_snapshot` — Hilbert-transform Ehlers MAMA/FAMA with
  fast_limit=0.5, slow_limit=0.05; label on MAMA vs FAMA cross and
  divergence percentage.
- `compute_cog_snapshot` — Ehlers centroid of 10-bar closes, 3-bar
  lagged signal; label on COG minus signal.
- `compute_didi_snapshot` — SMA(3)/SMA(8)/SMA(20) normalized
  ratios, detect needle crossovers; label on sign ordering +
  crossing event.

Schema v60 wraps v59 with five new tables
(`research_fractals / research_ift_rsi / research_mama /
research_cog / research_didi`) + timestamped indexes. Ten upsert/get
helpers follow the standard pattern.

### LAN sync (`engine/src/core/lan_sync.rs`)

Five entries under "// ── ADR-170 Round 58 ────" in `SYNCABLE_TABLES`;
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
6. 5 palette alias blocks — FRACTALS_WIN/FRACTAL_WIN/FRACTALS_RESEARCH/
   BILL_WILLIAMS_FRACTALS/BW_FRACTALS (note: plain "FRACTALS" is
   already bound to the pre-existing chart-overlay toggle, so the
   research window uses suffixed aliases),
   IFT_RSI/IFTRSI/INVERSE_FISHER_RSI/EHLERS_IFT_RSI/INVFISHER_RSI,
   MAMA/MAMA_WIN/MESA_ADAPTIVE_MA/MESA_AMA/EHLERS_MAMA,
   COG/COG_WIN/CENTER_OF_GRAVITY/EHLERS_COG/COG_OSC,
   DIDI/DIDI_INDEX/DIDI_NEEDLES/AGUIAR_DIDI/DIDI_WIN
7. 5 packet emitters (2.278–2.282 sub-blocks) in packet builder
8. 5 egui windows with Use-Chart / Load-Cached / Compute controls
   and striped summary grids
9. 5 BrokerMsg result handlers

### Documentation

- This ADR
- `docs/RESEARCH_PACKET.md` adds five new sub-blocks 2.278–2.282
  (FRACTALS, IFT_RSI, MAMA, COG, DIDI); INGESTED renumbers 2.278
  → 2.283 and Sector peer 2.279 → 2.284; envelope paragraph
  updated from "~83–159 KB" to "~84–161 KB"

### Alternatives considered

- **Ship COPPOCK (Coppock Curve) in this round.** Deferred — the
  Coppock Curve is `WMA₁₀(ROC₁₄ + ROC₁₁)` and is primarily a
  long-term (monthly-bar) indicator. Bundling it with daily-bar
  oscillators would confuse the operator about the appropriate
  timescale. Better shipped alongside other multi-timeframe
  surfaces.
- **Ship KELTNER_SQUEEZE (Bollinger/Keltner squeeze detector).**
  Deferred — KELTNER (ADR-116) and BOLLINGER (ADR-108) already
  ship separately; the squeeze detector is a derived surface that
  doesn't need its own struct. Could be emitted as a combined
  metadata field in a future round.
- **Ship HEIKEN-ASHI OHLC transformation.** Rejected — Heiken
  Ashi is a per-bar OHLC transform, not a reducing snapshot, and
  would want a dedicated bar-series emission pathway rather than
  the single-row snapshot pattern used throughout ADR-108+ rounds.

## Consequences

### Positive

- **Structural pivot markers added** (FRACTALS) — fills the
  Bill-Williams-system gap alongside ALLIGATOR (ADR-167) and
  AWESOME (ADR-117/134). Makes the Alligator-entry rule complete.
- **Bounded RSI-transform oscillator added** (IFT_RSI) — fills the
  "Ehlers-school compressed oscillator" gap alongside raw RSI,
  STOCHRSI, QQE, CRSI. The inverse Fisher transform is uniquely
  sharp at extremes compared to linear-rescale alternatives.
- **Phase-adaptive moving average added** (MAMA) — adds a
  second adaptive MA surface alongside KAMA (efficiency-ratio
  adaptive, ADR-117) and VIDYA (volatility-index adaptive,
  ADR-148). Together they cover the three dominant families of
  adaptive smoothing.
- **Zero-lag oscillator added** (COG) — Ehlers's characteristic
  "recency-weighted centroid" construction. Distinct from every
  EMA- and LINREG-based oscillator on the shipped list.
- **Regional indicator added** (DIDI) — Brazilian-market canonical
  that has been entirely absent from the Godel parity checklist
  until now. The "needles" pattern is unique to DIDI and not
  derivable from any general 3-SMA combination.
- +10 engine tests (5 roundtrip + 5 compute_oscillating)
  maintaining the property that every new surface has both
  persistence and compute-determinism coverage.

### Negative / Risks

- **MAMA's Hilbert-transform discriminator is sensitive to short
  warm-up.** We use n<32 as the INSUFFICIENT_DATA threshold.
  First-run HP caches below this produce the empty label via the
  note field. The dominant-cycle period estimate can drift on
  strong-trend regimes where the Hilbert transform is not well-defined.
- **Fractals need future bars to confirm.** Bill Williams's rule
  requires the middle bar's high/low to exceed the *two bars on
  each side*. The snapshot therefore reports the most recent
  **confirmed** fractal — bars that might still become fractals
  but haven't completed their confirmation window will not appear.
  This is an intrinsic property of the indicator, not a limitation
  of the implementation.
- **IFT_RSI's output is theoretically unbounded.** Although
  `(e^{2v} − 1)/(e^{2v} + 1)` is mathematically bounded to [-1, 1],
  the smoothed-RSI input `v` can reach extreme values on trending
  symbols where it approaches ±5. At those extremes the oscillator
  saturates near ±1 and loses resolution — expected behavior, but
  operators should know the fade-to-flat pattern at extreme trends
  is a feature not a bug.
- **DIDI's defaults (3/8/20) are the Brazilian standard.** Some
  US/EU traders running the same indicator use different periods
  (5/10/20 is common in Europe). No per-symbol tunability this
  round; deferred to a config pass.
- **"FRACTALS" palette token clash.** The plain "FRACTALS" palette
  alias was already bound to a pre-existing Bill-Williams chart
  overlay toggle (not a research window). Rather than rename the
  existing binding and break muscle memory, the new research
  window uses suffixed aliases: FRACTALS_WIN, FRACTAL_WIN,
  FRACTALS_RESEARCH, BILL_WILLIAMS_FRACTALS, BW_FRACTALS.

### Neutral

- No new API dependencies. All five surfaces reuse the existing
  `research_historical_price` HP cache.
- MAMA pairs naturally with the KAMA / VIDYA adaptive-MA family
  and its presence rounds out the "adaptive smoothing" axis.

### Paid-API gap

None introduced in this round. All five surfaces are HP-derived
and work entirely from the existing free-data cache.

## Verification

- `cargo test -p typhoon-engine --lib`: 1316 tests pass (+10 from
  1306).
- `cargo build -p typhoon-native`: clean build in 1m 25s, no new
  warnings (after adjusting the FRACTALS palette aliases to avoid
  a name clash with the existing chart-overlay binding).
- `docs/RESEARCH_PACKET.md`: five new sub-blocks 2.278–2.282 added
  (INGESTED and Sector peer renumbered); envelope updated.

## Packet envelope delta

| Surface | Field count | Approx bytes when populated | Free / Paid |
|---|---|---|---|
| FRACTALS | 11 | ~320 | Free (HP cache) |
| IFT_RSI | 10 | ~270 | Free (HP cache) |
| MAMA | 12 | ~340 | Free (HP cache) |
| COG | 9 | ~240 | Free (HP cache) |
| DIDI | 11 | ~300 | Free (HP cache) |
| **Round 58 total** | **53 fields** | **≈1.47 KB** | **Free** |

Envelope: 83–159 KB → 84–161 KB single-symbol; 810–1570 KB →
830–1600 KB for the canonical 10-symbol basket.
