# ADR-139: Godel Parity Round 31 — HILLTAIL / ARCHLM / PAINRATIO / CUSUM / CFVAR

**Status:** Accepted
**Date:** 2026-04-15
**Supersedes/extends:** ADR-108 through ADR-138
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Context

Round 30 (ADR-138) shipped PSR/ADF/MNKENDALL/BIPOWER/DDDUR,
completing the formal-test / stationarity / trend-presence
battery alongside the first higher-moment-corrected Sharpe
and the jump-vs-continuous volatility decomposition, plus the
drawdown-duration axis (DDDUR). 143 HP-local research sub-blocks
now cover the return-, drawdown-magnitude-, drawdown-duration-,
distribution-, persistence-, liquidity-, monthly- / weekday-
seasonality-, OHLC-vol-, tail-expectation-, sign-inference-,
sizing-, random-walk-test-, unit-root-test-, trend-presence-test-,
and jump-composition axes.

Five canonical surfaces remain, each on an axis still missing
from the existing 143 sub-blocks:

1. **No nonparametric tail-index estimator.** JBNORM (joint
   normality test) and the raw KURT magnitude describe tail
   heaviness *moment-wise*, but if the fourth moment is
   infinite — which is precisely the case of heavy-tailed
   returns where power-law behaviour matters most — moment-based
   diagnostics become meaningless. The *Hill estimator*
   `α̂ = k / Σᵢ₌₁ᵏ log(X_(i) / X_(k+1))` directly estimates the
   Pareto-tail exponent `P(|R| > x) ≈ c·x^(−α)`. Small α
   (≤ 2) ⇒ infinite-variance tails; α > 4 ≈ Gaussian-like tails.
   Separate estimates on left-tail and right-tail expose tail
   asymmetry — visible here, invisible to KURT.

2. **No formal conditional-heteroskedasticity test.** VOLOFVOL
   (rolling-σ variability) and BIPOWER (jump share) describe
   volatility structure, but neither tests the textbook null
   hypothesis "returns are iid with constant variance". The
   *Engle (1982) ARCH-LM test* regresses squared mean-residuals
   ε²_t on intercept + ε²_{t-1}, …, ε²_{t-q} and reports
   `LM = n·R² ~ χ²(q)` under H₀. Reject ⇒ there is memory in
   ε² ⇒ volatility clusters. With LJUNGB (joint autocorrelation
   of r), RUNSTEST (randomness of signs of r), ADF
   (stationarity of log p), and CUSUM (mean stability of r),
   ARCHLM becomes the *fifth inferential* diagnostic and the
   first on *second-moment memory*.

3. **No mean-magnitude drawdown metric.** CALMAR (single
   worst dd), BURKE (sum-of-squares of worst dds), STERLING
   (mean of N worst), ULCER (RMS of all-bar dds), and DDDUR
   (duration) cover the extreme, the L² norm, the worst-N, the
   RMS, and the time axis. Missing: the L¹ norm — **arithmetic
   mean of all-bar drawdowns**. The *Pain Index*
   `Σ_t |dd_t| / n` treats every bar equally (unlike worst-N
   or max), and the *Pain Ratio = ann_return / pain_index*
   is its Sharpe-style companion (Zephyr, FIBA). Completes the
   magnitude-norm sextet {max, RSS, mean-worst-N, RMS, mean-L¹,
   duration}.

4. **No structural-break test.** ADF tests stationarity of
   levels, LJUNGB tests joint autocorrelation of returns,
   RUNSTEST tests sign-randomness — all assume the series
   generation mechanism is stable throughout the window.
   None detects whether the *mean changed partway through*.
   The *Brown-Durbin-Evans (1975) OLS CUSUM test* builds
   `S_t = Σ_{s=1..t} (r_s − r̄) / σ̂` and reports
   `D = max_t |S_t| / √n` whose null distribution has
   Kolmogorov-Smirnov critical values {10%=1.22, 5%=1.36,
   1%=1.63}. Rejection ⇒ mean shift somewhere in the window.
   First structural-break test in the packet.

5. **No skew/kurt-adjusted parametric VaR.** CVAR reports
   historical (nonparametric) Expected Shortfall at 5%/1%.
   DOWNVOL/TAILR describe loss-side magnitude/shape. Missing:
   a parametric VaR that *uses* γ₃ and γ₄ to correct the
   Gaussian quantile, useful when an agent wants a smooth
   analytical quantile rather than an empirical one. The
   *Cornish-Fisher (1938) expansion*
   `z* = z + (z²−1)·γ₃/6 + (z³−3z)·γ₄/24 − (2z³−5z)·γ₃²/36`
   adjusts the standard-normal quantile `z` for sample skew
   and excess-kurtosis; then CF-VaR = μ + z*·σ. Reporting
   both the Gauss and CF quantiles plus the skew-term vs
   kurt-term split lets an agent see *which moment* is
   driving any departure from a vanilla Gaussian VaR.

Round 31 ships these five surfaces as ADR-139. Same additive
envelope as Rounds 5–30: no new fetchers, no cross-symbol scans,
no new external API dependencies. All five compute from the
trailing 253-session window on the existing HP cache.

## Decision

Ship Round 31 as a five-surface additive bundle using schema v32
layered on v31:

| Surface   | Table                | Purpose                                                         |
|-----------|----------------------|-----------------------------------------------------------------|
| HILLTAIL  | `research_hilltail`  | Hill tail-index estimator (power-law tail exponent)             |
| ARCHLM    | `research_archlm`    | Engle ARCH Lagrange-multiplier test (conditional heteroskedasticity) |
| PAINRATIO | `research_painratio` | Pain Index + Pain Ratio (Zephyr/FIBA — mean-|dd| drawdown norm) |
| CUSUM     | `research_cusum`     | Brown-Durbin-Evans OLS CUSUM (mean-stability / structural break) |
| CFVAR     | `research_cfvar`     | Cornish-Fisher modified Value-at-Risk (skew/kurt-adjusted)      |

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

- **HILLTAIL**: `GAUSSIAN_LIKE` (α>4) / `LIGHT_TAIL` (α>3) /
  `MODERATE_TAIL` (α>2) / `HEAVY_TAIL` (α>1) /
  `VERY_HEAVY_TAIL` (α≤1). The `|r|` estimate drives the
  label; left/right alphas are carried alongside for
  asymmetry inspection.
- **ARCHLM**: `NO_ARCH` (LM<11.07) / `WEAK_ARCH` (<15.09) /
  `STRONG_ARCH` (≥15.09). Hardcoded χ²₀.₀₅(5)=11.0705 and
  χ²₀.₀₁(5)=15.0863. Singular design matrix (near-constant
  ε² — e.g. alternating deterministic returns) is treated
  as NO_ARCH with LM=0, not INSUFFICIENT_DATA.
- **PAINRATIO**: `LOW_PAIN` (pain<1%) / `MILD_PAIN` (<3%) /
  `MODERATE_PAIN` (<7%) / `HIGH_PAIN` (<15%) /
  `SEVERE_PAIN` (≥15%). Buckets calibrated to the trailing-
  year pain-index range observed across typical equities.
- **CUSUM**: `STABLE` (D<1.22) / `MARGINAL` (<1.36) /
  `BREAK_DETECTED` (<1.63) / `STRONG_BREAK` (≥1.63).
  Critical values are Kolmogorov-Smirnov quantiles of the
  Brownian-bridge `sup` statistic.
- **CFVAR**: `BENIGN` (|Δ/Gauss| < 10%) / `SKEW_DRIVEN` (|skew
  term| ≥ |kurt term|, and 10% ≤ |Δ/Gauss| ≤ 50%) /
  `KURT_DRIVEN` (kurt-term dominant, 10-50%) /
  `EXTREME_DEVIATION` (|Δ/Gauss| > 50%). Decomposes the CF
  adjustment into its two drivers so an agent sees *why*
  CF ≠ Gauss.

ARCHLM uses Wilson-Hilferty `χ² → Φ` transform for the p-value
display; label determination uses direct χ² critical-value
comparison, not the transform. CUSUM and HILLTAIL use no
iterative or tabulated helpers — both are closed-form. CFVAR
uses hardcoded `z(5%)=-1.6448536…` and `z(1%)=-2.3263478…`
standard-normal quantiles. PAINRATIO uses the existing
closed-close-drawdown walker (shared form with CALMAR / BURKE
/ ULCER / STERLING).

## Consequences

### Positive

- **First nonparametric tail-index estimator.** HILLTAIL
  reports a Pareto-like α that remains well-defined even when
  the underlying distribution has infinite fourth (or even
  second) moment — precisely the regime where JBNORM / KURT
  become unreliable. Left/right-tail split exposes asymmetry
  invisible to KURT.
- **First formal conditional-heteroskedasticity test.**
  ARCHLM joins the inferential battery
  (LJUNGB / RUNSTEST / ADF / MNKENDALL / CUSUM / ARCHLM) as
  the sixth formal test and the only one on *second-moment*
  memory. Enables agents to distinguish "volatility clusters"
  (reject ARCHLM) from "iid return magnitude" (accept).
- **Drawdown magnitude-norm sextet complete.** CALMAR (sup)
  + BURKE (L²) + STERLING (worst-N mean) + ULCER (RMS) +
  DDDUR (duration) + PAINRATIO (L¹ / mean-|dd|) now cover
  every canonical magnitude summary an agent might want.
- **First structural-break test.** CUSUM enables agents to
  ask "is the return mean stable through the window" before
  trusting ADF / HURST / any stationarity-dependent diagnostic.
  Pairs with ADF (stationarity of levels) and RUNSTEST
  (randomness of signs) as the three stability-of-generator
  tests.
- **First higher-moment-adjusted parametric VaR.** CFVAR
  complements historical CVAR (nonparametric tail) with a
  smooth analytical quantile that an agent can extrapolate
  beyond the sample's worst observed loss. Skew-term vs
  kurt-term attribution tells an agent *why* CF diverges
  from Gauss.

### Negative / Risks

- **Schema migration.** `create_research_tables_v32` is additive
  over v31, so peers on v31 who receive v32 rows via LAN sync
  will create the 5 new tables via the existing
  create-before-insert path. No back-compat break.
- **Hill estimator k-choice sensitivity.** We use a fixed
  k = max(10, floor(0.10·n)) ≈ 25 for the 253-session window.
  Smaller k ⇒ more-tail-only but noisier; larger k ⇒ smoother
  but contaminated by body. 10% is a common rule-of-thumb;
  left/right-tail estimates with this k can have k as low as
  ~12 if roughly half the returns are one-signed.
- **ARCH-LM is the DOF-5 lag choice.** Standard in
  applied trading literature (Engle 1982 uses q=1 or q=5).
  Longer q increases power against long-memory ARCH but
  shrinks sample (n−q regression rows). We fix q=5 as the
  canonical mid-range choice.
- **CUSUM is the OLS (recursive-residual) form, not the MOSUM
  (moving-window) variant.** Detects a *single* level shift
  well; multiple offsetting shifts may cancel in the running
  sum. Agents suspecting multiple breaks should layer MNKENDALL
  (trend presence across the whole window) with CUSUM.
- **Cornish-Fisher can be non-monotone for extreme skew/kurt.**
  The adjusted quantile `z*` is not guaranteed to be monotone
  in probability when γ₃ or γ₄ are very large; CF-VaR at 1%
  may occasionally be closer to zero than CF-VaR at 5%.
  Documented in the struct doc-comment — when
  `EXTREME_DEVIATION` fires, agents should prefer the
  empirical CVAR over CF-VaR.
- **Packet weight.** Each surface adds ~300-700 bytes per
  symbol. Updated envelope numbers appear in the
  RESEARCH_PACKET.md header.

### Neutral

- **Label-based color scheme continues** the convention
  established in Rounds 24–30 (UP=green for "favorable" label,
  DOWN=red for "adverse", AXIS_TEXT=neutral).
- **Palette aliases** avoid prior bindings. Verified no
  collisions on `HILLTAIL`, `ARCHLM`, `PAINRATIO`, `CUSUM`,
  `CFVAR`, or their aliases (HILL, TAIL_INDEX, PAIN, CORNISH_
  FISHER, etc.). `BREAK` stays reserved for the Breakout
  palette entry; CUSUM uses `BDE_CUSUM` / `STRUCTURAL_BREAK` /
  `MEAN_BREAK` / `CUSUM_TEST` / `STABILITY_TEST`.
- **All five surfaces use the same broker handler shape** that
  has been stable since Round 22: `BrokerCmd::Compute*Snapshot →
  tokio::spawn → compute → msg_tx.send(BrokerMsg::*SnapshotMsg)`.
  The receive arm in `update()` both updates UI state and upserts
  to SQLite. LAN sync fans out on the next window.

## Verification

- `cargo test -p typhoon-engine --lib core::research::` — 443
  passing (up from 428 in Round 30, +15 new: 5 roundtrip + 10
  compute tests).
- `cargo check -p typhoon-engine` — clean.
- `cargo check -p typhoon-native` — clean; no palette-alias
  collisions.
- Each new compute surface carries at least one
  deterministic-fixture compute test beyond `INSUFFICIENT_DATA`:
  HILLTAIL/CUSUM/CFVAR share the oscillating ±0.5% fixture
  (positive and negative returns, finite variance); ARCHLM uses
  the oscillating fixture to exercise the singular-design edge
  (all ε² equal ⇒ treated as NO_ARCH); PAINRATIO uses
  `synthetic_ohlc_bars_150` (monotonically rising) to assert
  LOW_PAIN with pain_index < 1%.

## Packet envelope

After Round 31, single-symbol packet target envelope is **~58-114 KB**
(up from 56-110 in Round 30). Basket (10 symbols via BASKET) is
**~570-1140 KB** (up from 550-1100). Sub-block count grows 143 → 148.

Total HP-local research snapshot count after Round 31: **107**
(102 + 5). Total cross-symbol rank snapshots unchanged.
