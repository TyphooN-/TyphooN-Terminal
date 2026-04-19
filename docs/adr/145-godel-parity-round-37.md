# ADR-145: Quant Stats Round 37 — HIGUCHI / PICKANDS / KAPPA3 / LYAPUNOV / RANKAC

**Status:** Accepted
**Date:** 2026-04-16
**Supersedes/extends:** ADR-108 through ADR-144
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Parity classification

| Feature | Godel Terminal documented | TA-Lib primitive | Research packet | egui popup | Chart overlay |
|---|---|---|---|---|---|
| HIGUCHI | No | No | Yes | Yes | No (deferred — ADR-188) |
| PICKANDS | No | No | Yes | Yes | No (deferred — ADR-188) |
| KAPPA3 | No | No | Yes | Yes | No (deferred — ADR-188) |
| LYAPUNOV | No | No | Yes | Yes | No (deferred — ADR-188) |
| RANKAC | No | No | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** pure quant/statistical econometric primitives (Higuchi fractal dimension, Pickands extreme-value index, Kaplan-Knowles kappa-3 ratio, Rosenstein Lyapunov exponent, Spearman rank autocorrelation) — not documented Godel Terminal features and not TA-Lib catalog entries; classical quant-literature stats.

## Context

Round 36 (ADR-144) shipped KSNORM/ADTEST/LMOM/KYLELAM/PEAKOVER, pushing
HP-local research surfaces to 132 and per-symbol sub-blocks to 173 spanning
27+ orthogonal analytical dimensions. Five canonical surfaces remain, each
on an axis still missing.

1. **No fractal-dimension measure.** RS (Hurst) is a long-memory exponent,
   BOX (box-count) is not wired, and MSENT is an entropy family — none
   reports the direct geometric fractal dimension of the price walk.
   Higuchi (1988) gives a FD estimator robust for finite series: for each
   sub-sampling interval k ∈ 1..k_max, compute the normalised path
   length L(k), then FD = −slope of log L(k) on log k. FD ∈ [1,2]
   distinguishes SMOOTH (<1.1), PERSISTENT (<1.4), RANDOM (~1.5), and
   ROUGH (>1.6) regimes. HIGUCHI ships the estimator on the cumulative
   log-return walk with k_max=10.

2. **No Pickands tail-index.** HILLTAIL ships the Hill α estimator for
   heavy-tailed distributions, but Hill assumes Fréchet-domain (γ>0) and
   is biased under Gumbel/Weibull. Pickands (1975) γ̂ = ln((x_k − x_2k)
   / (x_2k − x_4k)) / ln 2 is valid across the whole extreme-value
   domain: γ̂>0 Fréchet (power tails), γ̂≈0 Gumbel (exponential tails),
   γ̂<0 Weibull (bounded tails). PICKANDS ships γ̂, tail α=1/γ̂, and the
   three order-statistic inputs with domain-classification labels.

3. **No third-moment downside ratio.** SHARPR (Sharpe), SORTINO (LPM2
   root), and PSR (probabilistic Sharpe) cover the Sharpe family.
   Kaplan-Knowles (2004) generalises Sortino: κ_n = (μ−MAR) / LPM_n^(1/n).
   κ3 uses LPM3 which penalises tail losses more heavily than LPM2 —
   important for distributions with visible skew. KAPPA3 ships κ3 plus
   Sortino as a reference so regression between the two is diagnostic.

4. **No chaos / Lyapunov measure.** Up through Round 36 the dynamics
   axis includes runs (RUNS), autocorrelation (PACF/VRK), and memory
   (MSENT/HURST). None answers the specific question "is the system
   chaotic?". Rosenstein et al. (1993) estimate the largest Lyapunov
   exponent λ₁ on an embedded time series: for each reference point
   find its nearest non-Theiler neighbour and track how log-distance
   grows with step. Slope of the log-divergence curve is λ₁. λ₁>0
   indicates sensitive dependence on initial conditions (chaotic);
   λ₁≈0 indicates periodic / quasi-periodic; λ₁<0 indicates stable
   convergence. LYAPUNOV uses m=3 embedding, τ=1, Theiler window=10,
   max 20 divergence steps.

5. **No rank-based autocorrelation.** PACF (partial ACF) is the Pearson
   version: sensitive to outliers and non-Gaussian tails. Spearman
   rank-ACF is the nonparametric counterpart — rank-transformed values
   then Pearson ρ. Robust under fat tails and invariant under monotone
   transforms. RANKAC ships ρ at lags 1, 5, 10 plus mean|ρ| and max|ρ|
   as an aggregate dependence score.

Round 37 ships these five surfaces as ADR-145. Same additive envelope
as Rounds 5–36: no new fetchers, no cross-symbol scans, no new external
API dependencies. All five compute from the trailing 253-session window
on the existing HP cache.

## Decision

Ship Round 37 as a five-surface additive bundle using schema v38
layered on v37:

| Surface   | Table                  | Purpose                                                           |
|-----------|------------------------|-------------------------------------------------------------------|
| HIGUCHI   | `research_higuchi`     | Higuchi 1988 fractal dimension of the cumulative log-return walk  |
| PICKANDS  | `research_pickands`    | Pickands 1975 extreme-value γ̂ (all three EV domains)             |
| KAPPA3    | `research_kappa3`      | Kaplan-Knowles 2004 κ3 = (μ−MAR)/LPM3^(1/3) annualised            |
| LYAPUNOV  | `research_lyapunov`    | Rosenstein 1993 largest Lyapunov exponent λ₁                      |
| RANKAC    | `research_rankac`      | Spearman rank autocorrelation at lags 1/5/10                      |

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

- **HIGUCHI**: `SMOOTH` (FD<1.1) / `PERSISTENT` (FD<1.4) / `RANDOM`
  (FD<1.6) / `ROUGH` (otherwise).
- **PICKANDS**: `FRECHET_HEAVY` (γ̂>0.5) / `FRECHET_MODERATE` (γ̂>0.1)
  / `GUMBEL_EXPONENTIAL` (γ̂>−0.1) / `WEIBULL_BOUNDED` (otherwise).
- **KAPPA3**: `STRONG` (κ3>1) / `POSITIVE` (κ3>0) / `NEUTRAL` (κ3>−0.5)
  / `NEGATIVE` (otherwise).
- **LYAPUNOV**: `CHAOTIC` (λ>0.10) / `WEAKLY_CHAOTIC` (λ>0.02) /
  `PERIODIC` (λ>−0.02) / `STABLE` (otherwise).
- **RANKAC**: `STRONG_DEPENDENCE` (max|ρ|>0.30) /
  `MODERATE_DEPENDENCE` (>0.15) / `WEAK_DEPENDENCE` (>0.05) /
  `INDEPENDENT` (otherwise).

## Consequences

### Positive

- **First direct fractal-dimension surface.** HIGUCHI operates on the
  cumulative walk (Higuchi 1988 convention), giving FD ∈ [1,2] with
  SMOOTH/RANDOM/ROUGH regime labels. Complements the Hurst exponent
  (H = 2 − FD under Brownian assumptions) as an independent
  finite-series estimator.
- **First EV-domain-agnostic tail estimator.** PICKANDS γ̂ is valid
  across Fréchet/Gumbel/Weibull; HILLTAIL (Hill) is only well-behaved
  in the Fréchet domain and can badly over-estimate for Gumbel series.
  Having both lets the user detect Hill/Pickands disagreement as a
  diagnostic for misspecified tail assumptions.
- **First third-moment downside ratio.** KAPPA3 weights LPM3 more
  heavily than LPM2, making it more sensitive to rare extreme losses
  than Sortino. The Sortino reference in the snapshot lets the user
  regress the two to detect asymmetry in the downside.
- **First chaos / Lyapunov diagnostic.** LYAPUNOV gives a principled
  answer to "is this sample chaotic?" that complements the RUNS
  randomness test and the PACF linear dependence test — sensitive to
  nonlinear deterministic dynamics neither of those detect.
- **First rank-based autocorrelation surface.** RANKAC is robust to
  fat tails and monotone transforms; useful for heavy-tailed assets
  (e.g., crypto) where Pearson ACF over-weights tail observations.

### Negative / Risks

- **Schema migration.** `create_research_tables_v38` is additive over
  v37, so peers on v37 who receive v38 rows via LAN sync will create
  the 5 new tables via the existing create-before-insert path. No
  back-compat break.
- **HIGUCHI uses the cumulative-sum walk,** which is the standard
  Higuchi 1988 convention. A variant definition uses raw returns
  directly; we choose the walk convention because it is what the
  original paper evaluated and gives stable FD estimates for
  financial-return series.
- **PICKANDS is sample-order-statistic-sensitive.** The estimator uses
  the x_k, x_2k, x_4k triplet where k = n/16. Choice of k is a bias /
  variance tradeoff; we default to a conservative n/16 that ensures
  4k < n with reasonable tail depth for n=253. Degenerate cases
  (x_k = x_2k or x_2k = x_4k, happening only on highly-discretised
  data) return `INSUFFICIENT_DATA`.
- **KAPPA3 divides by a cube-root of LPM3.** When LPM3 is numerically
  tiny (all returns above MAR), the function returns
  `INSUFFICIENT_DATA`. This is the honest answer — the ratio is
  undefined when there is no downside.
- **LYAPUNOV is computationally quadratic.** For n=253 → 251 embedded
  vectors, the nearest-neighbour search is O(n²)≈63k pair comparisons
  per symbol. Well within the ~ms envelope of other per-symbol
  snapshots.
- **RANKAC is O(n log n)** dominated by the sort for rank assignment.
  Ties are handled via average-rank per the Spearman convention —
  important for quantised price data where ties are common.
- **Packet weight.** Each surface adds ~200–600 bytes per symbol.
  PICKANDS is the lightest (7 scalars), LYAPUNOV the heaviest (7
  scalars + regression diagnostics). Updated envelope numbers appear
  in the RESEARCH_PACKET.md header.

### Neutral

- **Label-based color scheme continues** the convention established in
  Rounds 24–36 (UP=green for "favorable" label, DOWN=red for
  "adverse", AXIS_TEXT=neutral).
- **Palette aliases** avoid prior bindings. Verified no collisions on
  `HIGUCHI`, `PICKANDS`, `KAPPA3`, `LYAPUNOV`, `RANKAC`, or their
  aliases.
- **All five surfaces use the same broker handler shape** that has been
  stable since Round 22.

## Verification

- `cargo test -p typhoon-engine --lib core::research::` — 514
  passing (up from 504 in Round 36, +10 new: 5 roundtrip + 5
  compute_oscillating).
- `cargo check -p typhoon-engine` — clean.
- `cargo check -p typhoon-native` — clean; no palette-alias
  collisions.
- HIGUCHI/KAPPA3/RANKAC compute_oscillating use the ±0.5% oscillating
  fixture. HIGUCHI asserts FD is finite and log-k points ≥ 3.
  KAPPA3 asserts LPM3>0 and κ3, Sortino finite. RANKAC asserts ρ ∈
  [−1,1] at each lag and max|ρ| ≥ mean|ρ|.
  PICKANDS compute_oscillating asserts `INSUFFICIENT_DATA` because the
  two-value fixture gives x_k = x_2k = x_4k (degenerate order-stat
  triplet). LYAPUNOV accepts any of the five labels (including
  `INSUFFICIENT_DATA` in the degenerate case) and enforces λ_max finite
  when not INSUFFICIENT_DATA.

## Packet envelope

After Round 37, single-symbol packet target envelope is **~63-127 KB**
(up from 63-125 in Round 36). Basket (10 symbols via BASKET) is
**~630-1270 KB** (up from 620-1250). Sub-block count grows 173 → 178.

Total HP-local research snapshot count after Round 37: **137**
(132 + 5). Total cross-symbol rank snapshots unchanged.
