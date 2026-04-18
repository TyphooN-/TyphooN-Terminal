# ADR-169: Godel Parity Round 57 — KDJ / QQE / PMO / CFO / TMF

**Status:** Accepted
**Date:** 2026-04-17
**Supersedes/extends:** ADR-108 through ADR-168
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Context

Round 56 (ADR-168) shipped GMMA / MAENV / ADL / VHF / VROC. Round 57
continues the additive indicator cadence with five more canonical
surfaces along the Chinese-stochastic, smoothed-RSI, double-smoothed
momentum, regression-forecast oscillator, and true-range money-flow
axes. ADOSC (Chaikin A/D Oscillator) was initially slated for this
round but was dropped on verification — see Alternatives below.

1. **No KDJ (Chinese Stoch variant) snapshot.** KDJ is the
   default-bundled oscillator on nearly every Chinese-market
   terminal (Tonghuashun, Eastmoney, Futubull). Built on the same
   RSV = 100·(close − LLV_N)/(HHV_N − LLV_N) base as Stochastic
   (ADR-108) with the canonical N=9 window, but with EMA smoothing
   via `α = 1/3` for both %K and %D (equivalent to Wilder 3-period
   smoothing): `K = EMA₁/₃(RSV)`, `D = EMA₁/₃(K)`,
   `J = 3·K − 2·D`. The J line is the characteristic "3× leverage"
   difference and can exceed 100 or drop below 0 — exactly the
   extreme J readings produce the early overbought/oversold signal
   that the bounded %K/%D pair cannot. Distinct from STOCH (simple
   MA smoothing, ADR-108), STOCHF (no smoothing), and STOCHRSI
   (stochastic of the RSI, ADR-137). Header gives **kdj_label**
   (OVERBOUGHT / BULL / NEUTRAL / BEAR / OVERSOLD /
   INSUFFICIENT_DATA for n<12) derived from K/D crossover, J
   magnitude, and the 80/50/20 threshold ladder.

2. **No Quantitative Qualitative Estimation (QQE) snapshot.** Igor
   Livshin's QQE applies 5-bar EMA smoothing to the RSI (default
   RSI₁₄) to produce `rsi_smoothed`, then a Wilder smoothed average
   of `|Δrsi_smoothed|` scaled by 4.236 gives an adaptive trailing
   band: `upper_band = rsi_smoothed + 4.236·fast_atr_rsi_avg`,
   symmetric lower. Used as both an early-trend filter and an
   overbought/oversold gauge. Distinct from RSI (raw, ADR-108),
   STOCHRSI (Stoch of RSI, ADR-137), CRSI (Connors composite,
   ADR-167), and RVI (ADR-114). QQE is the "smoothed RSI with an
   adaptive adx-like trailing system" surface. Header gives
   **qqe_label** (STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR
   / INSUFFICIENT_DATA for n<40) derived from the smoothed RSI
   crossing the 50 line and the relationship to its prior bar.

3. **No Price Momentum Oscillator (PMO) snapshot.** Martin Pring's
   PMO is a double-smoothed ROC: `PMO = EMA(EMA(ROC(close,1)·10,
   35), 20)` with a 10-bar EMA signal line. The heavy
   triple-smoothing produces a reactive-but-noise-filtered momentum
   line well suited to multi-month swing trading. Distinct from MACD
   (EMA₁₂ − EMA₂₆ of close, ADR-108), TRIX (triple-smoothed EMA of
   close, ADR-141), and PPO (percentage price oscillator, ADR-132);
   PMO is the only canonical smoothed-ROC-plus-signal pair. Header
   gives **pmo_label** (STRONG_BULL / BULL / NEUTRAL / BEAR /
   STRONG_BEAR / INSUFFICIENT_DATA for n<70) derived from the
   PMO/signal relationship and the histogram sign.

4. **No Chande Forecast Oscillator (CFO) snapshot.** Tushar
   Chande's CFO compares the current close to the one-bar-ahead
   forecast from a linear regression fit over N bars:
   `CFO = 100·(close − forecast)/close`. Positive means price is
   ahead of trend (bullish deviation); negative means behind
   (bearish deviation); zero-crossings are trend-reversal signals
   in Chande's systems. Distinct from LINREG (fitted value,
   ADR-145), TSF (projected future value, ADR-146), DPO
   (detrended price, ADR-131), and PPO (non-regression momentum).
   CFO is the one oscillator built as close-minus-regression-forecast
   as a percentage. Header gives **cfo_label** (STRONG_ABOVE_TREND
   / ABOVE_TREND / NEUTRAL / BELOW_TREND / STRONG_BELOW_TREND /
   INSUFFICIENT_DATA for n<15) derived from the CFO magnitude and
   sign.

5. **No Twiggs Money Flow (TMF) snapshot.** Colin Twiggs's
   smoothed, volume-weighted variant of Chaikin Money Flow
   (ADR-140). Replaces the bar's full high/low range with a
   **true range** (max(high, prev_close) − min(low, prev_close))
   to correctly handle gap bars, then smooths with an exponential
   MA rather than a simple N-bar sum: TMF tracks cumulative net
   volume more smoothly than raw CMF and is less jittery on
   gap-heavy instruments. Default is 21-bar EMA smoothing on both
   numerator (money flow volume) and denominator (volume).
   Distinct from CMF (range-based, simple sum, ADR-140), ADL
   (cumulative total, not ratio, ADR-168), KLINGER (dual-EMA
   volume force, ADR-152), PVT (ROC·volume, ADR-164), and
   CHAIKOSC (EMA diff of ADL, ADR-156). Header gives **tmf_label**
   (STRONG_INFLOW / INFLOW / NEUTRAL / OUTFLOW / STRONG_OUTFLOW
   / INSUFFICIENT_DATA for n<22) derived from the TMF value and
   its prior bar.

## Decision

Ship Round 57 as additive-only — no breaking changes to any existing
surface, schema, or LAN sync protocol.

### Engine (`engine/src/core/research.rs`)

Add five snapshot structs after `VrocSnapshot`:

- `KdjSnapshot { symbol, as_of, bars_used, stoch_length, k_smooth,
  rsv, k_value, d_value, j_value, j_prev, last_close, kdj_label,
  note }`
- `QqeSnapshot { symbol, as_of, bars_used, rsi_length,
  smooth_length, qqe_factor, rsi_value, rsi_smoothed,
  fast_atr_rsi_avg, upper_band, lower_band, qqe_prev, last_close,
  qqe_label, note }`
- `PmoSnapshot { symbol, as_of, bars_used, smooth1_length,
  smooth2_length, signal_length, pmo_value, pmo_signal, pmo_prev,
  histogram, last_close, pmo_label, note }`
- `CfoSnapshot { symbol, as_of, bars_used, length, slope,
  intercept, forecast, cfo_value, cfo_prev, last_close, cfo_label,
  note }`
- `TmfSnapshot { symbol, as_of, bars_used, length, ema_money_flow,
  ema_volume, tmf_value, tmf_prev, last_close, tmf_label, note }`

Five compute functions:

- `compute_kdj_snapshot` — RSV over 9-bar HHV/LLV, K = EMA₁/₃(RSV),
  D = EMA₁/₃(K), J = 3K − 2D, label on K/D cross + 80/50/20 ladder
  + J extreme.
- `compute_qqe_snapshot` — RSI₁₄, 5-bar EMA smoothing, Wilder MA
  of |ΔRSI_smoothed|, adaptive ±4.236·σ bands, label on smoothed
  RSI vs 50 + crossover direction.
- `compute_pmo_snapshot` — ROC(close,1)·10, EMA 35 → EMA 20 →
  signal EMA 10, label on PMO/signal cross and histogram sign.
- `compute_cfo_snapshot` — OLS fit over N=14 bars, forecast = m·N
  + b, CFO = 100·(close − forecast)/close, label on sign + magnitude.
- `compute_tmf_snapshot` — true-range MFM, EMA₂₁ of
  MFM·volume and of volume, TMF = ema_money_flow / ema_volume,
  label on sign + magnitude.

Schema v59 wraps v58 with five new tables
(`research_kdj / research_qqe / research_pmo / research_cfo /
research_tmf`) + timestamped indexes. Ten upsert/get helpers follow
the standard pattern.

### LAN sync (`engine/src/core/lan_sync.rs`)

Five entries under "// ── ADR-169 Round 57 ────" in `SYNCABLE_TABLES`;
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
6. 5 palette alias blocks — KDJ/KDJFIT/KDJ_WIN/K_D_J/KDJ_STOCH/STOCH_KDJ,
   QQE/QQE_MOD/QUANT_QUAL_EST/QUANTITATIVE_QUALITATIVE,
   PMO/PRING_PMO/PRICE_MOMENTUM_OSC/PRICE_MOMENTUM_OSCILLATOR,
   CFO/FORECAST_OSC/CHANDE_FORECAST/FORECAST_OSCILLATOR,
   TMF/TWIGGS_MF/TWIGGS_MONEY_FLOW/TWIGGSMONEYFLOW
7. 5 packet emitters (2.273–2.277 sub-blocks) in packet builder
8. 5 egui windows with Use-Chart / Load-Cached / Compute controls
   and striped summary grids
9. 5 BrokerMsg result handlers

### Documentation

- This ADR
- `docs/RESEARCH_PACKET.md` adds five new sub-blocks 2.273–2.277
  (KDJ, QQE, PMO, CFO, TMF); INGESTED renumbers 2.273 → 2.278
  and Sector peer 2.274 → 2.279; envelope paragraph updated from
  "~82–157 KB" to "~83–159 KB"

### Alternatives considered

- **Ship Chaikin A/D Oscillator (ADOSC).** Originally the first
  indicator slated for this round as a natural pair with ADL
  (ADR-168). **Rejected on verification** — `compute_chaikosc_snapshot`
  (shipped in Round 40 / ADR-156) is already the Chaikin A/D
  Oscillator: `EMA₃(ADL) − EMA₁₀(ADL)`. The common technician's
  trap is that "CHAIKIN_OSC" is sometimes used in older texts for
  the Chaikin *Volatility* oscillator (H−L range smoothed), and
  both ADR-156 and ADR-168's alternatives paragraph incorrectly
  described it as such. In practice Round 40's implementation is
  the A/D oscillator. KDJ swapped in as the round's oscillator slot.
- **Ship Bill Williams Fractals in this round.** Deferred again —
  Fractals are peak/trough markers with sequence-of-5-bars local
  extrema semantics and want different packet/window ergonomics
  than the rest of Round 57. Better bundled with MAMA, DIDI, or
  another chart-marker surface in a future round.
- **Ship MAMA (MESA Adaptive MA) as a 6th indicator.** Rejected —
  MAMA's Hilbert-transform dominant cycle extraction is
  algorithmically distinct from the rest of the round and deserves
  its own ADR.

## Consequences

### Positive

- **Chinese-market stochastic variant added** (KDJ) — fills the
  regional-variant gap alongside STOCH/STOCHF/STOCHRSI. J line's
  3·K − 2·D "leverage" construction surfaces extreme readings
  (J>100 or J<0) that the bounded K/D pair cannot, giving earlier
  reversal alerts.
- **Smoothed RSI with trailing bands added** (QQE) — fills the
  "adaptive-band oscillator" gap alongside raw RSI, STOCHRSI,
  CRSI. The 4.236-factor trailing bands are unique to QQE and
  give early trend-reversal warning.
- **Double-smoothed ROC with signal added** (PMO) — the
  Pring-specific smoothed-ROC-plus-signal surface, complementary
  to MACD, TRIX, and PPO on the momentum-with-signal axis.
- **Regression-forecast oscillator added** (CFO) — the one
  canonical "close vs regression forecast as pct" oscillator,
  complementing LINREG, TSF, and DPO on the regression-family axis.
- **True-range smoothed money flow added** (TMF) — fills the
  "gap-safe EMA-smoothed money flow" gap that CMF (range-based,
  simple sum) doesn't cover. The true-range construction is the
  key differentiator on gap-heavy instruments.
- +10 engine tests (5 roundtrip + 5 compute_oscillating)
  maintaining the property that every new surface has both
  persistence and compute-determinism coverage.

### Negative / Risks

- **KDJ's J line is unbounded.** By design — J = 3K − 2D can
  exceed 100 or drop below 0 on strong trends. Users used to
  bounded 0–100 oscillators may be surprised. The label ladder
  already accounts for J extremes (OVERBOUGHT when K>80 and J>90;
  OVERSOLD when K<20 and J<10) but callers inspecting raw J
  should be aware.
- **PMO warm-up is 70 bars.** Triple-EMA smoothing with 35/20/10
  window requires ≥65 bars for full lock-in; we use 70 as the
  conservative INSUFFICIENT_DATA threshold. First-run HP caches
  below this produce the empty label and surface the shortage
  via the note field.
- **QQE factor is fixed at 4.236.** Livshin's canonical value,
  but some traders run variants at 2.618 (wider early signals)
  or 4.618 (tighter). No per-symbol tunability in this round;
  deferred to a config pass if user demand materialises.
- **CFO is sensitive to the regression window.** We default to
  N=14 bars per Chande's original specification. Longer windows
  (21, 30) produce smoother oscillators but slower signals;
  shorter (7, 10) produce more noise. Fixed in this round.
- **TMF requires volume.** Zero-volume bars (FX aggregated feeds,
  some crypto pairs) degrade the label to INSUFFICIENT_DATA or
  produce noisy flat readings. Users should verify volume
  reliability before trusting the signal.

### Neutral

- No new API dependencies. All five surfaces reuse the existing
  `research_historical_price` HP cache.
- KDJ shares the 9-bar HHV/LLV base with STOCH (ADR-108) but uses
  different smoothing (EMA₁/₃ vs simple MA) and adds the J line;
  the two surfaces coexist without duplication.

### Paid-API gap

None introduced in this round. All five surfaces are HP-derived
and work entirely from the existing free-data cache.

## Verification

- `cargo test -p typhoon-engine --lib`: 1306 tests pass (+10 from
  1296).
- `cargo build -p typhoon-native`: clean build, no new warnings.
- `docs/RESEARCH_PACKET.md`: five new sub-blocks 2.273–2.277 added
  (INGESTED and Sector peer renumbered); envelope updated.

## Packet envelope delta

| Surface | Field count | Approx bytes when populated | Free / Paid |
|---|---|---|---|
| KDJ | 11 | ~280 | Free (HP cache) |
| QQE | 13 | ~320 | Free (HP cache) |
| PMO | 11 | ~290 | Free (HP cache) |
| CFO | 11 | ~290 | Free (HP cache) |
| TMF | 10 | ~260 | Free (HP cache) |
| **Round 57 total** | **56 fields** | **≈1.44 KB** | **Free** |

Envelope: 82–157 KB → 83–159 KB single-symbol; 790–1540 KB →
810–1570 KB for the canonical 10-symbol basket.
