# ADR-142: Godel Parity Round 34 — SAMPEN / PERMEN / RECFACT / KPSS / SPECENT

**Status:** Accepted
**Date:** 2026-04-16
**Supersedes/extends:** ADR-108 through ADR-141
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Parity classification

| Feature | Godel Terminal documented | TA-Lib primitive | Research packet | egui popup | Chart overlay |
|---|---|---|---|---|---|
| SAMPEN | No | No | Yes | Yes | No (deferred — ADR-188) |
| PERMEN | No | No | Yes | Yes | No (deferred — ADR-188) |
| RECFACT | No | No | Yes | Yes | No (deferred — ADR-188) |
| KPSS | No | No | Yes | Yes | No (deferred — ADR-188) |
| SPECENT | No | No | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** pure quant/statistical econometric primitives (sample entropy, permutation entropy, recovery factor, KPSS stationarity test, spectral entropy) — not documented Godel Terminal features and not TA-Lib catalog entries; classical quant-literature stats.

## Context

Round 33 (ADR-141) shipped UPR/LEVEREFF/DRAWDAR/VARHALF/GINI,
completing the asymmetric-capture-vs-risk, return→vol-feedback,
quantile-based-drawdown-risk, vol-regime-persistence, and
return-concentration axes. 158 per-symbol research sub-blocks now
span 25+ orthogonal analytical dimensions.

Five canonical surfaces remain, each on an axis still missing:

1. **No self-match-excluded regularity measure.** APEN (Pincus 1991)
   includes self-matches (i==j counted), producing slight bias on
   periodic data (clamped to max(0, result)). **Sample Entropy**
   (Richman & Moorman 2000) excludes self-matches:
   SampEn = −ln(A/B) where A = m+1-length template matches and
   B = m-length matches (i≠j only). More consistent, lower bias,
   modern standard. Parameters: m=2, r=0.2·σ. Same O(n²) complexity.

2. **No temporal ordering entropy.** ENTROPY measures value
   distribution (histogram); APEN/SAMPEN measure template
   regularity. Neither captures *ordinal pattern structure*.
   **Permutation Entropy** (Bandt & Pompe 2002) maps consecutive
   m-tuples to their rank permutation and computes Shannon entropy
   of the pattern distribution. m=3 gives 6 possible ordinal
   patterns. Normalised PE ∈ [0,1]. Low PE ⇒ returns follow a
   small set of ordinal patterns (predictable ordering).

3. **No raw-cumulative-return-per-drawdown metric.** CALMAR uses
   annualized return / max dd; BURKE uses annualized / √(Σdd²);
   STERLING uses annualized / mean-worst-N dd; PAINRATIO uses
   annualized / mean|dd|. All annualize. **Recovery Factor** =
   raw cumulative return / |max drawdown| answers: "has the asset
   fully recovered from its worst loss?" RF > 1 ⇒ yes. Simple,
   intuitive, distinct from all existing annualized ratios.

4. **No stationarity-null hypothesis test.** ADF (ADR-138) tests
   H₀: unit root (non-stationary). The **KPSS test**
   (Kwiatkowski-Phillips-Schmidt-Shin 1992) tests H₀: stationary.
   Standard practice reports both: agreement strengthens the
   conclusion; disagreement (both reject or both fail) signals
   fractional integration or ambiguity. KPSS uses the Newey-West
   long-run variance estimator with Bartlett kernel and
   ℓ = floor(4·(n/100)^(2/9)) lag truncation.

5. **No frequency-domain entropy.** ENTROPY measures value
   distribution; APEN/SAMPEN measure time-domain template
   regularity; PERMEN measures ordinal patterns. None examines the
   **frequency spectrum**. **Spectral Entropy** = Shannon entropy
   of normalised power spectral density (PSD) via DFT. Low
   SpecEnt ⇒ dominant frequency components (cyclical returns);
   high SpecEnt ⇒ broad spectrum (noise-like, unpredictable).
   Implemented via O(n²) DFT on mean-centred returns — trivially
   fast for n=253 (~32K multiplies).

Round 34 ships these five surfaces as ADR-142. Same additive
envelope as Rounds 5–33: no new fetchers, no cross-symbol scans,
no new external API dependencies. All five compute from the
trailing 253-session window on the existing HP cache.

## Decision

Ship Round 34 as a five-surface additive bundle using schema v35
layered on v34:

| Surface  | Table                | Purpose                                                       |
|----------|----------------------|---------------------------------------------------------------|
| SAMPEN   | `research_sampen`    | Sample Entropy (Richman & Moorman 2000)                       |
| PERMEN   | `research_permen`    | Permutation Entropy (Bandt & Pompe 2002)                      |
| RECFACT  | `research_recfact`   | Recovery Factor (cumulative return / max drawdown)             |
| KPSS     | `research_kpss`      | KPSS stationarity test (complement to ADF)                    |
| SPECENT  | `research_specent`   | Spectral Entropy via DFT (frequency-domain periodicity)       |

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

- **SAMPEN**: `REGULAR` (SampEn < 0.3) / `MODERATE` (< 0.7) /
  `COMPLEX` (< 1.2) / `HIGHLY_COMPLEX` (≥ 1.2) / `UNDEFINED`
  (B=0 — no template matches found).
- **PERMEN**: `REGULAR` (H_norm < 0.50) / `MODERATE` (< 0.70) /
  `COMPLEX` (< 0.85) / `HIGHLY_COMPLEX` (≥ 0.85).
- **RECFACT**: `DEEP_LOSS` (RF < −1) / `NEGATIVE` (< 0) /
  `RECOVERING` (< 1) / `GOOD` (< 3) / `EXCELLENT` (≥ 3).
- **KPSS**: `STATIONARY` (η_μ ≤ 0.347) / `WEAKLY_NONSTATIONARY`
  (≤ 0.463) / `NONSTATIONARY` (> 0.463). Critical values from
  Kwiatkowski et al. (1992) Table 1 (level stationarity, µ only).
- **SPECENT**: `PERIODIC` (H_norm < 0.50) /
  `MODERATE_PERIODICITY` (< 0.70) / `BROAD_SPECTRUM` (< 0.85) /
  `NOISE_LIKE` (≥ 0.85).

## Consequences

### Positive

- **First self-match-excluded regularity measure.** SAMPEN
  complements APEN as the modern standard entropy measure for
  time-series regularity. Having both provides a robustness check.
- **First ordinal-pattern entropy.** PERMEN captures temporal
  ordering structure invisible to ENTROPY (value distribution),
  APEN/SAMPEN (template matching), and SPECENT (frequency domain).
- **First raw-cumulative recovery metric.** RECFACT answers the
  intuitive question "has this asset recovered from its worst
  loss?" — distinct from all annualized drawdown ratios.
- **First stationarity-null test.** KPSS formally complements ADF:
  ADF tests H₀: non-stationary; KPSS tests H₀: stationary.
  Standard econometric practice reports both.
- **First frequency-domain entropy.** SPECENT reveals periodicity
  in returns (dominant cycles, seasonality harmonics) orthogonal to
  all time-domain complexity measures.

### Negative / Risks

- **Schema migration.** `create_research_tables_v35` is additive
  over v34, so peers on v34 who receive v35 rows via LAN sync
  will create the 5 new tables via the existing
  create-before-insert path. No back-compat break.
- **SampEn undefined when B=0.** If no m-length template matches
  exist (extremely rare for n=253), SampEn is mathematically
  undefined. Handled via `UNDEFINED` label sentinel.
- **SampEn O(n²) complexity.** Same as APEN. For n=253, ~31K
  pair comparisons — sub-millisecond.
- **Permutation entropy ties.** When two consecutive returns are
  exactly equal, their ordinal pattern assignment depends on index
  order (stable sort). This is the standard Bandt-Pompe convention.
- **KPSS critical values are asymptotic.** For n=253, the finite-
  sample distribution may differ slightly from the tabulated
  asymptotic values. The Newey-West Bartlett kernel HAC estimator
  mitigates this.
- **DFT O(n²) complexity.** For n=253, this is ~32K complex
  multiplies — trivially fast. A Cooley-Tukey FFT would be needed
  only if the window grew to 10K+ bars.
- **Packet weight.** Each surface adds ~200-600 bytes per
  symbol. Updated envelope numbers appear in the
  RESEARCH_PACKET.md header.

### Neutral

- **Label-based color scheme continues** the convention
  established in Rounds 24–31 (UP=green for "favorable" label,
  DOWN=red for "adverse", AXIS_TEXT=neutral).
- **Palette aliases** avoid prior bindings. Verified no
  collisions on `SAMPEN`, `PERMEN`, `RECFACT`, `KPSS`, `SPECENT`,
  or their aliases.
- **All five surfaces use the same broker handler shape** that
  has been stable since Round 22.

## Verification

- `cargo test -p typhoon-engine --lib core::research::` — 484
  passing (up from 473 in Round 33, +11 new: 5 roundtrip + 5
  compute + 1 recfact_compute_rising).
- `cargo check -p typhoon-engine` — clean.
- `cargo check -p typhoon-native` — clean; no palette-alias
  collisions.
- SAMPEN/PERMEN/KPSS/SPECENT use the oscillating ±0.5% fixture;
  RECFACT uses both oscillating and monotonically rising fixtures.
  SAMPEN oscillating asserts sampen ≥ 0 and b_count > 0.
  PERMEN oscillating asserts normalised PE ∈ [0,1] and
  patterns_observed ≤ 6. KPSS oscillating asserts STATIONARY
  (oscillating returns are mean-reverting). SPECENT oscillating
  asserts PERIODIC or MODERATE_PERIODICITY (dominant frequency at
  N/2 for alternating returns). RECFACT rising asserts positive
  cumulative return and recovery_factor > 0.

## Packet envelope

After Round 34, single-symbol packet target envelope is **~61-120 KB**
(up from 60-118 in Round 33). Basket (10 symbols via BASKET) is
**~600-1200 KB** (up from 590-1180). Sub-block count grows 158 → 163.

Total HP-local research snapshot count after Round 34: **122**
(117 + 5). Total cross-symbol rank snapshots unchanged.
