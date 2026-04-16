# ADR-143: Godel Parity Round 35 — ROBVOL / RENYIENT / RETQUANT / MSENT / EWMAVOL

**Status:** Accepted
**Date:** 2026-04-16
**Supersedes/extends:** ADR-108 through ADR-142
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Context

Round 34 (ADR-142) shipped SAMPEN/PERMEN/RECFACT/KPSS/SPECENT,
completing the self-match-excluded-regularity, ordinal-pattern-entropy,
raw-recovery, stationarity-null-hypothesis, and frequency-domain-entropy
axes. 163 per-symbol research sub-blocks now span 26+ orthogonal
analytical dimensions.

Five canonical surfaces remain, each on an axis still missing:

1. **No outlier-resistant volatility.** Classical σ (used by REAL_VOL,
   VOLCLUSTER, PARKINSON, etc.) is dominated by a handful of large
   returns — an OPEC headline or earnings miss can double the realized
   σ. Robust estimators downweight outliers: **MAD** (Median Absolute
   Deviation, Hampel 1974) scaled by 1/0.6745 recovers the Gaussian σ
   on clean data but caps influence at the median; **IQR/1.349**
   (Tukey's hinges) uses only quartile spread. Both annualized ×√252.
   Shipping classical + MAD + IQR side-by-side lets the user see how
   much of "vol" is outliers.

2. **No quadratic-order entropy.** ENTROPY (ADR-140) is Shannon
   (α=1); APEN/SAMPEN/PERMEN/SPECENT are all Shannon-flavored. The
   **Rényi family** generalises to any α≥0; at **α=2** (collision
   entropy, Rényi 1961) H₂ = −log₂(Σ pᵢ²) weights probabilities
   quadratically. Σ pᵢ² = collision probability (chance two random
   samples share a bin) — a classical concentration measure. H₂ ≤ H₁
   with equality iff uniform; the gap measures non-uniformity that
   Shannon averages over.

3. **No dense quantile snapshot.** TAILR/CVAR report single-point tail
   statistics at one threshold; RETSKEW captures third-moment shape
   but not the specific percentile levels. **Return Quantile Profile**
   reports the full 9-point profile (P1, P5, P10, P25, P50, P75, P90,
   P95, P99), plus IQR and a tail asymmetry ratio (P99+P01)/(P99−P01).
   This is what a quant looks at on day one of a new symbol — a dense,
   non-parametric snapshot of the return distribution.

4. **No multi-scale complexity.** APEN/SAMPEN compute at a single time
   scale. Real financial series have **scale-dependent complexity** —
   daily noise is different from weekly structure. **Multiscale
   Entropy** (Costa, Goldberger, Peng 2005) computes SampEn on
   coarse-grained series at scales τ=1..5: the raw series at τ=1,
   2-period averages at τ=2, ..., 5-period averages at τ=5. A
   **decaying** MSE curve indicates short-scale noise; **sustained**
   or **increasing** curves indicate genuine long-range structure.
   The integral Σ SampEn(τ) is the Complexity Index.

5. **No adaptive-weighted volatility.** REAL_VOL uses equal weights;
   VARHALF (ADR-141) reports the AR(1) half-life but not a current
   volatility level. The **RiskMetrics EWMA** (J.P. Morgan 1996) with
   λ=0.94 is the industry-standard adaptive vol: σ²_t = λ·σ²_{t−1} +
   (1−λ)·r²_t. Recent returns dominate (effective lookback ≈ 1/(1−λ)
   ≈ 17 days). The ratio EWMA/classical is a regime flag: >1.2 ⇒
   recent vol elevated, <0.8 ⇒ recent vol suppressed.

Round 35 ships these five surfaces as ADR-143. Same additive envelope
as Rounds 5–34: no new fetchers, no cross-symbol scans, no new
external API dependencies. All five compute from the trailing
253-session window on the existing HP cache.

## Decision

Ship Round 35 as a five-surface additive bundle using schema v36
layered on v35:

| Surface   | Table                  | Purpose                                                        |
|-----------|------------------------|----------------------------------------------------------------|
| ROBVOL    | `research_robvol`      | Robust Volatility (MAD/0.6745 + IQR/1.349 + classical)         |
| RENYIENT  | `research_renyient`    | Rényi Entropy at α=2 (collision entropy)                        |
| RETQUANT  | `research_retquant`    | 9-point Return Quantile Profile                                |
| MSENT     | `research_msent`       | Multiscale Entropy (Costa-Goldberger-Peng 2005, τ=1..5)        |
| EWMAVOL   | `research_ewmavol`     | RiskMetrics EWMA Volatility (λ=0.94)                           |

Each table follows the established JSON-blob-per-symbol shape:

```sql
CREATE TABLE research_<name> (
    symbol TEXT PRIMARY KEY,
    snapshot_json TEXT NOT NULL DEFAULT '{}',
    updated_at INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_research_<name>_updated ON research_<name>(updated_at);
```

Each snapshot carries a regime `label` field (3–5 buckets +
`INSUFFICIENT_DATA` sentinel). Label strings:

- **ROBVOL**: `HEAVY_OUTLIERS` (avg ratio < 0.60) / `MODERATE_OUTLIERS`
  (< 0.80) / `CLEAN` (< 1.10) / `LIGHT_TAILS` (≥ 1.10). Ratio averages
  MAD/classical and IQR/classical; values well below 1 indicate that
  classical σ is inflated by outliers (robust estimator is smaller);
  values above 1 indicate a sub-Gaussian tail.
- **RENYIENT**: `CONCENTRATED` (H₂_norm < 0.50) / `MODERATE` (< 0.70)
  / `DISPERSED` (< 0.85) / `HIGHLY_DISPERSED` (≥ 0.85).
- **RETQUANT**: `LEFT_TAIL_HEAVY` (asymm < −0.30) / `RIGHT_TAIL_HEAVY`
  (> 0.30) / `WIDE_IQR` (IQR > 4% daily, symmetric) / `SYMMETRIC`.
- **MSENT**: `LONG_RANGE_REGULAR` (all τ SampEn < 0.3) / `DECAYING`
  (τ=5 < 0.7·τ=1) / `INCREASING` (τ=5 > 1.3·τ=1) / `SUSTAINED`
  (otherwise, flat-ish across scales).
- **EWMAVOL**: `ELEVATED` (ratio > 1.20) / `SUPPRESSED` (< 0.80) /
  `NORMAL` (in between).

## Consequences

### Positive

- **First outlier-resistant vol measure.** ROBVOL complements
  classical σ-based RV (ADR-108) and Parkinson OHLC σ. When MAD ratio
  is well below 1, the quoted RV is being driven by a small number of
  big days — a hidden fragility.
- **First quadratic-order entropy.** RENYIENT fills the α=2 slot in
  the Rényi family; collision probability is directly meaningful for
  concentration assessment and differs from Shannon by weighting high-
  probability outcomes more heavily.
- **First dense quantile snapshot.** RETQUANT is what a desk trader
  wants at glance — the entire empirical CDF boiled down to 9
  anchors + IQR + tail asymmetry, no distributional assumption.
- **First multi-scale complexity measure.** MSENT exposes
  scale-dependent structure invisible to single-scale APEN/SAMPEN.
  A decaying MSE curve flags a series that looks complex at τ=1 but
  reduces to near-random at τ=5 (short-scale artefact).
- **First adaptive-weighted vol.** EWMAVOL is the industry-standard
  RiskMetrics estimator. The ratio EWMA/classical is a compact
  regime flag distinct from the AR(1) persistence reported by VARHALF.

### Negative / Risks

- **Schema migration.** `create_research_tables_v36` is additive
  over v35, so peers on v35 who receive v36 rows via LAN sync will
  create the 5 new tables via the existing create-before-insert path.
  No back-compat break.
- **RENYIENT histogram bin choice.** Uses Sturges' rule (K = ⌈log₂(n)
  + 1⌉, min 4). Freedman-Diaconis (IQR-based) would be asymptotically
  better for heavy-tailed data, but Sturges is simpler and the
  normalization H₂/log₂(K) adjusts for bin-count differences.
- **RETQUANT linear interpolation.** Quantiles use linear
  interpolation between the two nearest sorted values (type-7 in R
  nomenclature). This matches numpy.quantile defaults.
- **MSENT τ≥3 sample sizes.** For n=253 at τ=5 we have 50 coarse
  samples — adequate for SampEn but the estimate has higher variance
  than τ=1. The required ≥100 bars (checked via early return) guards
  against pathological short-history cases.
- **MSENT tolerance fixed to raw-series σ.** Standard Costa et al.
  convention: r stays fixed across scales so cross-scale SampEn
  values are comparable on a common scale (rescaling r to each
  coarse-grained σ would hide the variance-reducing effect of
  averaging that is itself diagnostic).
- **EWMAVOL burn-in.** The first variance estimate is seeded at the
  classical sample variance then iterated forward — after 253 steps,
  λ^253 ≈ 10⁻⁷, so the seed has negligible residual influence. Pure
  cold-start (σ²₀ = 0) is avoided to prevent early-window
  instability.
- **Packet weight.** Each surface adds ~200-800 bytes per symbol.
  RETQUANT is the heaviest (14 percentile values) but still small in
  absolute terms. Updated envelope numbers appear in the
  RESEARCH_PACKET.md header.

### Neutral

- **Label-based color scheme continues** the convention established
  in Rounds 24–32 (UP=green for "favorable" label, DOWN=red for
  "adverse", AXIS_TEXT=neutral).
- **Palette aliases** avoid prior bindings. Verified no collisions on
  `ROBVOL`, `RENYIENT`, `RETQUANT`, `MSENT`, `EWMAVOL`, or their
  aliases.
- **All five surfaces use the same broker handler shape** that has
  been stable since Round 22.

## Verification

- `cargo test -p typhoon-engine --lib core::research::` — 494
  passing (up from 484 in Round 34, +10 new: 5 roundtrip + 5
  compute).
- `cargo check -p typhoon-engine` — clean.
- `cargo check -p typhoon-native` — clean; no palette-alias
  collisions.
- ROBVOL/RENYIENT/RETQUANT/MSENT/EWMAVOL use the oscillating ±0.5%
  fixture. ROBVOL oscillating asserts classical/MAD/IQR σ > 0.
  RENYIENT oscillating asserts H₂_norm ∈ [0,1] and collision_prob ∈
  (0,1]. RETQUANT oscillating asserts quantile ordering (P1 ≤ P25 ≤
  P50 ≤ P75 ≤ P99) and IQR ≥ 0. MSENT oscillating asserts max_scale=5
  and tolerance > 0. EWMAVOL oscillating asserts σ_annual > 0,
  classical_annual > 0, ratio > 0, λ=0.94.

## Packet envelope

After Round 35, single-symbol packet target envelope is **~62-123 KB**
(up from 61-120 in Round 34). Basket (10 symbols via BASKET) is
**~610-1230 KB** (up from 600-1200). Sub-block count grows 163 → 168.

Total HP-local research snapshot count after Round 35: **127**
(122 + 5). Total cross-symbol rank snapshots unchanged.
