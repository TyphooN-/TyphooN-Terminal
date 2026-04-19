# ADR-144: Quant Stats Round 36 — KSNORM / ADTEST / LMOM / KYLELAM / PEAKOVER

**Status:** Accepted
**Date:** 2026-04-16
**Supersedes/extends:** ADR-108 through ADR-143
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Parity classification

| Feature | Godel Terminal documented | TA-Lib primitive | Research packet | egui popup | Chart overlay |
|---|---|---|---|---|---|
| KSNORM | No | No | Yes | Yes | No (deferred — ADR-188) |
| ADTEST | No | No | Yes | Yes | No (deferred — ADR-188) |
| LMOM | No | No | Yes | Yes | No (deferred — ADR-188) |
| KYLELAM | No | No | Yes | Yes | No (deferred — ADR-188) |
| PEAKOVER | No | No | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** pure quant/statistical econometric primitives (Kolmogorov-Smirnov normality, Anderson-Darling normality, Hosking L-moments, Kyle's lambda price-impact, EVT Peaks-Over-Threshold) — not documented Godel Terminal features and not TA-Lib catalog entries; classical quant-literature stats.

## Context

Round 35 (ADR-143) shipped ROBVOL/RENYIENT/RETQUANT/MSENT/EWMAVOL, pushing
HP-local research surfaces to 127 and per-symbol sub-blocks to 168 spanning
26+ orthogonal analytical dimensions. Five canonical surfaces remain, each
on an axis still missing.

1. **No Kolmogorov-Smirnov normality test.** RETSKEW/RETKURT report
   moments, RETQUANT reports percentiles, LMOM (this round) reports robust
   shape — but none answer the specific question "is this sample
   Gaussian?" KSNORM runs the **Kolmogorov-Smirnov one-sample test**
   (Kolmogorov 1933) against N(μ̂,σ̂²), computing the sup-norm distance
   between the empirical CDF and Φ. Critical values at 10%/5%/1%
   significance (1.22/√n, 1.36/√n, 1.63/√n) give a compact three-way
   rejection flag, and the raw D statistic is itself a distance metric.

2. **No tail-weighted normality test.** KS is omnibus but relatively weak
   in the tails where financial data most often deviates from Gaussian.
   The **Anderson-Darling test** (Anderson-Darling 1954) weights by
   1/(F(1−F)), putting the emphasis on tail regions. Stephens (1986)
   provides the small-sample correction A²·(1 + 0.75/n + 2.25/n²) and
   the p-value approximation for the N(μ̂,σ̂²) case; fixed critical
   values (0.631/0.752/1.035 at 10%/5%/1%) make it comparable across
   symbols.

3. **No L-moments.** Classical skew/kurt (RETSKEW) are notoriously
   sensitive to outliers because they weight observations by |x−μ|³ and
   |x−μ|⁴. Hosking's **L-moments** (Hosking 1990) are linear combinations
   of order statistics and are defined (finite) whenever the mean exists.
   L-ratios τ3 = L₃/L₂ (L-skew) and τ4 = L₄/L₂ (L-kurtosis) are bounded
   — τ3 ∈ [−1,1], τ4 ∈ [−¼, 1] for continuous distributions — which
   makes them directly comparable across symbols and regime-stable in
   the presence of heavy tails. The probability-weighted moments use the
   unbiased Hosking estimator b_r = (1/n) Σ_{i=1..n} C(i−1,r)/C(n−1,r) ·
   x_(i).

4. **No Kyle's lambda.** AMIHUD reports |r|/$-volume (a return-based
   illiquidity ratio). Kyle (1985) motivates a **price-impact
   coefficient** λ on dollar — or share — volume: the slope of |Δp| ~ V
   regression. λ = cov(|Δp|, V) / var(V) measures how many price units
   per share of order flow. KYLELAM ships λ + correlation ρ + R² for
   signal-quality assessment; the distinction from AMIHUD is that Kyle
   is a *linear regression coefficient* on share flow with physical
   units ($-per-share) whereas Amihud is a scale-free ratio.

5. **No EVT Peaks-Over-Threshold.** TAILR and CVAR report point
   estimates of left-tail risk; RETQUANT gives the P1/P99 anchors but
   not the tail structure above them. **POT** (Pickands-Balkema-de Haan
   1974/1975) examines exceedances above a high threshold u: the
   conditional distribution (X−u | X>u) approaches a Generalized Pareto
   as u increases, and the **mean-excess / threshold ratio** is a
   classical GPD-shape diagnostic. PEAKOVER reports P95 and P99
   thresholds, counts, mean excesses, and max excesses — the raw
   material for any GPD fit and a standalone extreme-tail diagnostic on
   its own.

Round 36 ships these five surfaces as ADR-144. Same additive envelope
as Rounds 5–35: no new fetchers, no cross-symbol scans, no new external
API dependencies. All five compute from the trailing 253-session window
on the existing HP cache.

## Decision

Ship Round 36 as a five-surface additive bundle using schema v37
layered on v36:

| Surface   | Table                  | Purpose                                                           |
|-----------|------------------------|-------------------------------------------------------------------|
| KSNORM    | `research_ksnorm`      | Kolmogorov-Smirnov normality test (omnibus)                       |
| ADTEST    | `research_adtest`      | Anderson-Darling normality test (tail-weighted)                   |
| LMOM      | `research_lmom`        | Hosking L-moments L1..L4 + τ3 τ4                                  |
| KYLELAM   | `research_kylelam`     | Kyle's daily price-impact λ (regression |Δp|~V)                   |
| PEAKOVER  | `research_peakover`    | Peaks-Over-Threshold (EVT/GPD foundation)                         |

Each table follows the established JSON-blob-per-symbol shape:

```sql
CREATE TABLE research_<name> (
    symbol TEXT PRIMARY KEY,
    snapshot_json TEXT NOT NULL DEFAULT '{}',
    updated_at INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_research_<name>_updated ON research_<name>(updated_at);
```

Each snapshot carries a regime `label` field (4 active buckets +
`INSUFFICIENT_DATA` sentinel). Label strings:

- **KSNORM**: `NORMAL` (fails to reject at 10%) / `MILD_DEVIATION`
  (rejects at 10% but not 5%) / `MODERATE_DEVIATION` (rejects at 5% but
  not 1%) / `STRONG_NON_NORMAL` (rejects at 1%).
- **ADTEST**: Same four labels with the same rejection progression on
  A²-adjusted vs 0.631/0.752/1.035.
- **LMOM**: `HEAVY_LEFT` (τ3 < −0.30) / `HEAVY_RIGHT` (τ3 > 0.30) /
  `HEAVY_TAILS` (τ4 > 0.30) / `LIGHT_TAILS` (τ4 < 0.05) /
  `NEAR_SYMMETRIC` (otherwise).
- **KYLELAM**: `HIGH_IMPACT` (R² > 0.20) / `MODERATE_IMPACT` (R² > 0.05)
  / `LOW_IMPACT` (otherwise) / `NO_SIGNAL` (R² < 0.02).
- **PEAKOVER**: `EXTREME_TAIL` (mean-excess / P95 > 0.80) /
  `HEAVY_TAIL` (> 0.40) / `MODERATE_TAIL` (> 0.20) / `LIGHT_TAIL`
  (otherwise). Ratio is Pickands' shape-parameter proxy — high values
  indicate slowly decaying tails above the P95 threshold.

## Consequences

### Positive

- **First KS-distance normality test.** KSNORM ships the specific
  omnibus goodness-of-fit question at three significance levels in one
  snapshot. The raw D statistic is a distance metric usable for
  cross-symbol comparison even when none reject.
- **First tail-weighted normality test.** ADTEST is strictly more
  sensitive than KS in the tails. Symbols for which KSNORM says NORMAL
  but ADTEST says STRONG_NON_NORMAL are those with tail-region
  deviations that KS failed to detect — a useful diagnostic axis.
- **First L-moment family.** LMOM is robust to heavy tails and is
  defined whenever the mean exists — particularly valuable since
  RETSKEW/RETKURT moment estimates explode for infinite-variance
  candidates. τ3/τ4 bounds [-1,1]/[-0.25,1] make LMOM directly
  regime-comparable across symbols.
- **First price-impact regression.** KYLELAM complements AMIHUD with a
  slope (λ with units $-per-share) rather than a ratio. λ and R²
  together give both signal strength and signal quality.
- **First POT / EVT surface.** PEAKOVER gives mean-excess statistics at
  two high thresholds without committing to a GPD fit, letting the user
  eyeball tail behavior. Foundation for any future GPD/Hill-estimator
  snapshot.

### Negative / Risks

- **Schema migration.** `create_research_tables_v37` is additive over
  v36, so peers on v36 who receive v37 rows via LAN sync will create
  the 5 new tables via the existing create-before-insert path. No
  back-compat break.
- **KSNORM uses sample-estimated μ̂,σ̂ in Φ evaluation.** The standard
  Kolmogorov critical values technically assume fully-specified
  parameters; with plug-in estimates the D statistic is
  slightly-downward-biased (Lilliefors correction would tighten the
  cutoffs). We keep the classical 1.22/1.36/1.63 cutoffs to match the
  most commonly cited reference table; label thresholds are broad
  enough that the bias is not operationally meaningful.
- **ADTEST Stephens p-value is an asymptotic approximation.** Works
  well for n ≥ 25; we enforce n ≥ 30. Extreme A²_adj values (> 10) map
  to p ≈ 0 via the high branch; this is correct in the limit but the
  exact p-value quickly goes below machine ε.
- **LMOM unbiased estimator is O(n) per moment.** Overall compute is
  O(n log n) dominated by the sort — same cost envelope as RETQUANT.
- **KYLELAM on constant-volume symbols.** When var(V) ≈ 0 (e.g.,
  illiquid microcaps with only round-lot quotes) the compute function
  returns `INSUFFICIENT_DATA` rather than a divide-by-zero; this is the
  honest answer. Also returns INSUFFICIENT_DATA when var(|Δp|) ≈ 0
  (the rarer case of zero price variation).
- **PEAKOVER threshold choice.** We report P95 and P99 rather than
  fitting a Pickands-optimal threshold. The mean-excess/threshold
  ratio-based label is a first-pass diagnostic; a dedicated GPD fit
  would belong in a later round.
- **Packet weight.** Each surface adds ~250–700 bytes per symbol.
  PEAKOVER is the heaviest (10 tail-related numbers plus counts).
  Updated envelope numbers appear in the RESEARCH_PACKET.md header.

### Neutral

- **Label-based color scheme continues** the convention established in
  Rounds 24–35 (UP=green for "favorable" label, DOWN=red for
  "adverse", AXIS_TEXT=neutral).
- **Palette aliases** avoid prior bindings. Verified no collisions on
  `KSNORM`, `ADTEST`, `LMOM`, `KYLELAM`, `PEAKOVER`, or their aliases.
- **All five surfaces use the same broker handler shape** that has been
  stable since Round 22.

## Verification

- `cargo test -p typhoon-engine --lib core::research::` — 504
  passing (up from 494 in Round 35, +10 new: 5 roundtrip + 5
  compute_oscillating).
- `cargo check -p typhoon-engine` — clean.
- `cargo check -p typhoon-native` — clean; no palette-alias
  collisions.
- KSNORM/ADTEST/LMOM/PEAKOVER compute_oscillating use the ±0.5%
  oscillating fixture. KSNORM asserts D ∈ [0,1] with monotone critical
  values; ADTEST asserts A²_adj ≥ 0 and p ∈ [0,1]; LMOM asserts L2 > 0
  and τ3 ∈ [−1,1] (τ4 is finite-only because bimodal oscillating lies
  outside the continuous-distribution τ4 envelope); PEAKOVER asserts
  P95 > 0, P99 ≥ P95, mean_excess ≥ 0, max_excess ≥ mean_excess.
  KYLELAM compute_oscillating asserts `INSUFFICIENT_DATA` because the
  fixture has constant volume (var(V) = 0).

## Packet envelope

After Round 36, single-symbol packet target envelope is **~63-125 KB**
(up from 62-123 in Round 35). Basket (10 symbols via BASKET) is
**~620-1250 KB** (up from 610-1230). Sub-block count grows 168 → 173.

Total HP-local research snapshot count after Round 36: **132**
(127 + 5). Total cross-symbol rank snapshots unchanged.
