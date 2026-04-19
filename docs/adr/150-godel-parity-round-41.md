# ADR-150: Godel Parity Round 41 — MCLEODLI / OUFIT / GPH / BURGSPEC / KENDALLTAU

**Status:** Accepted
**Date:** 2026-04-17
**Supersedes/extends:** ADR-108 through ADR-149
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Parity classification

| Feature | Godel Terminal documented | TA-Lib primitive | Research packet | egui popup | Chart overlay |
|---|---|---|---|---|---|
| MCLEODLI | No | No | Yes | Yes | No (deferred — ADR-188) |
| OUFIT | No | No | Yes | Yes | No (deferred — ADR-188) |
| GPH | No | No | Yes | Yes | No (deferred — ADR-188) |
| BURGSPEC | No | No | Yes | Yes | No (deferred — ADR-188) |
| KENDALLTAU | No | No | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** pure quant/statistical econometric primitives (McLeod-Li squared-returns portmanteau, Ornstein-Uhlenbeck mean-reversion fit, Geweke-Porter-Hudak long-memory d, Burg maximum-entropy AR spectrum, Kendall's tau rank autocorrelation) — not documented Godel Terminal features and not TA-Lib catalog entries; classical quant-literature stats.

## Context

Round 40 (ADR-149) shipped DURBINWATSON/BDSTEST/BREUSCHPAGAN/TURNPTS/PERIODOGRAM,
taking HP-local research surfaces to 152 and per-symbol sub-blocks to 193. Five
canonical econometric surfaces remain orthogonal to the existing suite:

1. **No ARCH-on-squared-returns diagnostic.** ARCHLM (ADR-139) runs an LM
   regression of squared residuals on lagged squared residuals. The
   McLeod-Li (1983) test is the complementary *portmanteau* view: it
   applies Ljung-Box directly to squared returns, summing Q = n(n+2) Σ
   ρ̂²(k)/(n-k) on r_t² out to lag h and comparing to χ²(h). Widely-cited
   first-line ARCH screen in time-series econometrics; orthogonal to
   both ARCHLM (single-lag LM) and LJUNGB (portmanteau on levels, not
   squares).

2. **No explicit mean-reversion fit.** MRHL (ADR-??) reports the
   AR(1)-implied half-life but not the full Ornstein-Uhlenbeck
   parametrization. OUFIT ships θ (speed), μ (long-run log-price), σ
   (diffusion), residual sd, R², and a four-level label
   (TRENDING/SLOW_REVERT/MODERATE_REVERT/FAST_REVERT). Direct translation
   of the Vasicek / OU SDE that underlies pairs-trading, statistical
   arbitrage, and fixed-income mean-reversion models.

3. **No direct long-memory d estimator.** HURST (ADR-112), DFA (ADR-128),
   HIGUCHI (ADR-145), and MFDFA (ADR-146) all estimate fractal /
   long-memory properties, but the classical Geweke-Porter-Hudak (1983)
   semiparametric log-periodogram regression for the fractional
   integration order d is absent. GPH uses m = floor(n^0.5) low-frequency
   bins, regresses ln I(λ_j) on −2 ln|2 sin(λ_j/2)|, and returns d̂, its
   π²/(24m)-stderr, t-stat against H0: d=0, and a four-level label
   (ANTIPERSISTENT / SHORT_MEMORY / LONG_MEMORY / NONSTATIONARY). The
   most-cited semiparametric long-memory estimator.

4. **No parametric spectral estimator.** PERIODOGRAM (ADR-149) is the
   non-parametric direct-DFT. The Burg maximum-entropy AR estimator is
   the classic parametric alternative — it fits an AR(p) via the Burg
   recursion (Marple 1987) and evaluates the resulting spectral density
   on a frequency grid. Better peak resolution on short series than the
   raw periodogram at the cost of model-order sensitivity. Reports
   dominant frequency/period, peak power, mean power across the grid,
   peak-to-mean ratio, and a four-level cycle-strength label.

5. **No rank-based autocorrelation test.** RANKAC (ADR-??) uses
   Spearman's ρ. Kendall's τ is the complementary rank correlation
   defined on concordant-vs-discordant pair counts and has narrower
   asymptotic confidence bands for heavy-tailed distributions. We
   compute the lag-1 τ on daily log-returns: S = #concordant −
   #discordant, τ = S/[n(n−1)/2], and the asymptotic z-statistic
   τ/sqrt(2(2n+5)/(9n(n−1))). Complements DURBINWATSON (linear AR(1))
   by being non-parametric and tail-robust.

Round 41 ships these five surfaces as ADR-150. Same additive envelope
as Rounds 5–40: no new fetchers, no cross-symbol scans, no new external
API dependencies. All five compute from the trailing 253-session
window on the existing HP cache.

## Decision

Ship Round 41 as a five-surface additive bundle using schema v42
layered on v41:

| Surface      | Table                   | Purpose                                                                |
|--------------|-------------------------|------------------------------------------------------------------------|
| MCLEODLI     | `research_mcleodli`     | McLeod-Li portmanteau on squared returns (ARCH detection)              |
| OUFIT        | `research_oufit`        | Ornstein-Uhlenbeck mean-reversion fit (θ, μ, σ, half-life)             |
| GPH          | `research_gph`          | Geweke-Porter-Hudak log-periodogram long-memory d estimator            |
| BURGSPEC     | `research_burgspec`     | Burg maximum-entropy AR spectrum / parametric dominant-cycle detection |
| KENDALLTAU   | `research_kendalltau`   | Kendall's tau lag-1 rank autocorrelation                               |

Each table follows the established JSON-blob-per-symbol shape:

```sql
CREATE TABLE research_<name> (
    symbol TEXT PRIMARY KEY,
    snapshot_json TEXT NOT NULL DEFAULT '{}',
    updated_at INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_research_<name>_updated ON research_<name>(updated_at);
```

Each snapshot carries a regime `label` field (3–5 active buckets +
`INSUFFICIENT_DATA` sentinel). Label strings:

- **MCLEODLI**: `NO_ARCH` (Q ≤ χ²_95) / `MILD_ARCH` (Q < 2·crit) /
  `STRONG_ARCH` (otherwise).
- **OUFIT**: `TRENDING` (θ ≤ 0) / `SLOW_REVERT` (half-life > n/3) /
  `MODERATE_REVERT` (> n/10) / `FAST_REVERT` (otherwise).
- **GPH**: `ANTIPERSISTENT` (d < −0.1) / `SHORT_MEMORY` (|d| ≤ 0.1) /
  `LONG_MEMORY` (0.1 < d < 0.5) / `NONSTATIONARY` (d ≥ 0.5).
- **BURGSPEC**: `NO_AR_CYCLE` (peak/mean ≤ 2) / `WEAK_AR_CYCLE` (≤ 4) /
  `MODERATE_AR_CYCLE` (≤ 8) / `STRONG_AR_CYCLE` (otherwise).
- **KENDALLTAU**: `STRONG_POS` (τ > 0.1) / `WEAK_POS` (> 0.03) /
  `NO_RANK_AUTO` (|τ| ≤ 0.03) / `WEAK_NEG` (< −0.03) / `STRONG_NEG`
  (< −0.1).

## Consequences

### Positive

- **Covers the "complementary classical diagnostics" axis.** McLeod-Li
  is the direct portmanteau complement to ARCHLM's LM regression;
  Kendall's τ is the complement to Spearman's ρ (RANKAC) and to
  Durbin-Watson's linear AR(1).
- **Closes the long-memory gap.** HURST/DFA/HIGUCHI/MFDFA approach the
  same construct from different fractal angles; GPH is the canonical
  semi-parametric *regression-based* d estimator that completes the set.
- **Completes the spectral-domain family.** PERIODOGRAM covers the
  non-parametric DFT; BURGSPEC covers the parametric AR-spectrum
  alternative. Together they let the user triangulate dominant-cycle
  claims across estimators with different leakage/resolution tradeoffs.
- **First explicit SDE fit.** OUFIT makes the Vasicek / OU drift
  structure explicit and reports θ, μ, σ, half-life directly — useful
  for option pricing intuition and pairs-trading screens that were
  previously only accessible as an implied MRHL half-life.
- **No new external dependencies, no fetcher expansion.** Pure
  econometric compute on the HP cache — the same additive envelope as
  Rounds 26–40.

### Negative / Risks

- **Schema migration.** `create_research_tables_v42` is additive over
  v41, so peers on v41 who receive v42 rows via LAN sync will create
  the 5 new tables via the existing create-before-insert path. No
  back-compat break.
- **GPH estimator bias.** The GPH regression uses m = floor(n^0.5) low
  frequencies as a default bandwidth — the Shimotsu-Phillips exact local
  Whittle (ELW) is asymptotically more efficient, but requires
  Whittle-likelihood machinery we do not currently carry. For the label
  question ("is d in the long-memory range?") the simple GPH regression
  is adequate; for sharp d estimates the user should cross-check against
  a dedicated econometrics package. Documented in the `note` field.
- **Burg AR-order sensitivity.** We use p = min(20, n/4). Higher orders
  can introduce spurious peaks; lower orders can smooth over real ones.
  The AR(p) spectral density is model-dependent in a way the direct
  periodogram is not. Labels are at peak-to-mean-ratio thresholds
  (>2 / >4 / >8) that require clear spectral dominance, dampening the
  sensitivity. Documented tradeoff.
- **OUFIT assumes a linear SDE.** Real price processes have jumps,
  regime-switches, and volatility clustering that violate the OU
  assumptions. The label is a *regime indicator* at OU-fit level — if a
  security is strongly trending or has a structural break, θ estimated
  via OLS AR(1) will not stabilise. Half-life is ∞ when θ ≤ 0 (by
  convention, TRENDING label).
- **Kendall τ is O(m²) on the lag-1 pair count.** For the default
  n = 253-bar window that is ~64k ops — negligible in absolute terms
  but worth noting. Merge-sort-based O(m log m) exists but is more code
  and the gain is moot at this scale.
- **Packet weight.** MCLEODLI adds ~220 bytes, OUFIT ~280, GPH ~200,
  BURGSPEC ~260, KENDALLTAU ~240. Total Round 41 addition: ~1.2 KB/symbol.
  Updated envelope numbers appear in the RESEARCH_PACKET.md header.

### Neutral

- **Label-based color scheme continues** the convention established in
  Rounds 24–40 (UP=green for "favorable" label, DOWN=red for "adverse",
  AXIS_TEXT=neutral).
- **Palette aliases** avoid prior bindings. Verified no collisions on
  `MCLEODLI`, `OUFIT`, `GPH`, `BURGSPEC`, `KENDALLTAU`, or their
  aliases (`MLTEST`, `OU`, `LONGMEMORY`, `ARSPECTRUM`, `KTAU`, etc.).
  HALFLIFE deliberately omitted from OUFIT aliases — already bound to
  the MRHL window (ADR-??) for backwards compatibility.
- **All five surfaces use the same broker handler shape** that has been
  stable since Round 22.

### Paid-API gap (for later revisit)

Same as ADR-149. The gaps remain data-access-gated (intraday bars,
order-book depth, options IV surfaces, corporate actions feeds,
realised-variance matrices). No Round 41 surface needed any of these;
all compute from the daily HP cache.

## Verification

- `cargo test -p typhoon-engine --lib` — target 1126 passing (up from
  1116 in Round 40, +10 new: 5 roundtrip + 5 compute_oscillating).
- `cargo check -p typhoon-engine` — clean.
- `cargo check -p typhoon-native` — clean; no palette-alias collisions
  (HALFLIFE kept bound to MRHL; OUFIT uses OU_FIT instead).
- MCLEODLI/OUFIT/GPH/BURGSPEC/KENDALLTAU compute_oscillating use the
  ±0.5% oscillating fixture (150 bars, 149 log-returns). Each asserts
  the returned label belongs to its regime set, scalars are finite
  when label is not INSUFFICIENT_DATA, and axis-specific invariants:
  MCLEODLI Q≥0, h≥5, critical>0; OUFIT theta finite, residual_sd≥0,
  R²∈[0,1]; GPH d_stderr>0, m_freqs≥4; BURGSPEC ar_order≥2, peak_power≥0,
  mean_power>0; KENDALLTAU τ∈[-1,1], pair_count=m(m-1)/2.

## Packet envelope

After Round 41, single-symbol packet target envelope is **~67-134 KB**
(up from 66-132 in Round 40). Basket (10 symbols via BASKET) is
**~670-1340 KB** (up from 660-1320). Sub-block count grows 193 → 198.

Total HP-local research snapshot count after Round 41: **157**
(152 + 5). Total cross-symbol rank snapshots unchanged.
