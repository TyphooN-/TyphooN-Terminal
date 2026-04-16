# ADR-138: Godel Parity Round 30 — PSR / ADF / MNKENDALL / BIPOWER / DDDUR

**Status:** Accepted
**Date:** 2026-04-15
**Supersedes/extends:** ADR-108 through ADR-137
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Context

Round 29 (ADR-137) shipped STERLING/KELLYF/LJUNGB/RUNSTEST/ZERORET,
completing the drawdown-ratio family (CALMAR / BURKE / STERLING),
introducing forward-looking sizing (KELLYF), adding a formal
inferential battery (LJUNGB joint autocorrelation + RUNSTEST
randomness), and the third microstructure liquidity proxy
(ZERORET). 138 HP-local research sub-blocks now cover the return-,
drawdown-magnitude-, distribution-, persistence-, liquidity-,
monthly- and weekday-seasonality-, OHLC-vol-, tail-expectation-,
sign-inference-, and sizing axes.

Five canonical surfaces remain, each on an axis still missing
from the existing 138 sub-blocks:

1. **No higher-moment-corrected Sharpe.** SHARPR assumes normally
   distributed returns; SORTINO/CALMAR/BURKE/STERLING all use
   alternative denominators but still report magnitudes without
   any statement about *confidence* that the ratio is real. The
   *Probabilistic Sharpe Ratio* (Lopez de Prado, 2012) answers
   "given sample size, observed skew and kurtosis, what's the
   probability the true Sharpe exceeds SR*?":
   `PSR = Φ((SR − SR*)·√(n−1) / √(1 − γ₃·SR + (γ₄−1)/4·SR²))`.
   First packet surface to correct a return-quality ratio for
   higher-order moments with a formal probability.

2. **No stationarity / unit-root test.** HURST (persistence),
   DFA (nonstationarity-robust persistence), AUTOCOR and LJUNGB
   (autocorrelation structure) each describe memory/dependence
   but none tests the textbook null hypothesis of a random walk.
   The *Dickey-Fuller* regression
   `Δlog(p)_t = α + β·log(p)_{t-1} + ε` with critical values
   {−3.43, −2.86, −2.57} at 1/5/10% is the canonical unit-root
   test. A negative-enough t-statistic rejects the unit root,
   i.e. the log-price series is mean-reverting / stationary.
   Complements HURST/DFA (continuous measure) with a binary
   reject/no-reject outcome.

3. **No nonparametric trend-presence test.** Mann-Kendall
   `S = Σᵢ<ⱼ sign(x_j − x_i)` over all pairs has a closed-form
   null distribution
   `Var(S) = n(n−1)(2n+5)/18` and reports a distribution-free
   z-statistic. Unlike linear regression of price on time,
   Mann-Kendall makes no assumption of linearity or normality
   — it tests whether the *order* of observations reflects
   a monotone trend. Pairs with ADF (stationarity) to separate
   trending-but-mean-reverting from trending-and-non-stationary.

4. **No jump-vs-continuous volatility decomposition.** Five
   volatility estimators (CLOSEVOL/PARKINSON/GKVOL/RSVOL/VOLOFVOL)
   measure *level* of variability but none separates it into
   continuous (diffusive) vs jump components. The Barndorff-
   Nielsen & Shephard (2004) bipower variation
   `BPV = (π/2)·Σ|r_t|·|r_{t-1}|` is a consistent estimator of
   the integrated *continuous* variance, so `1 − BPV/RV` is the
   share of realized variance attributable to jumps. Useful for
   classifying returns by regime (diffusive vs jump-driven).

5. **No drawdown-duration axis.** CALMAR/BURKE/STERLING measure
   drawdown *magnitude*; RUNLEN measures sign-streak length
   (days-in-a-row). Neither captures "how long from peak to
   recovery". Drawdown Duration (DDDUR) walks the close series
   with a running-max tracker and records, for each closed dd
   event, the peak-to-recovery bar count. Reports
   max/mean/median event durations, total bars underwater,
   % of time underwater, and (if a drawdown is still open)
   a `currently_underwater` flag.

Round 30 ships these five surfaces as ADR-138. Same additive
envelope as Rounds 5–29: no new fetchers, no cross-symbol scans,
no new external API dependencies. All five compute from the
trailing 253-session window on the existing HP cache.

## Decision

Ship Round 30 as a five-surface additive bundle using schema v31
layered on v30:

| Surface   | Table                | Purpose                                                         |
|-----------|----------------------|-----------------------------------------------------------------|
| PSR       | `research_psr`       | Probabilistic Sharpe Ratio (Lopez de Prado 2012)                |
| ADF       | `research_adf`       | Dickey-Fuller unit-root test on log-price                       |
| MNKENDALL | `research_mnkendall` | Mann-Kendall nonparametric trend test                           |
| BIPOWER   | `research_bipower`   | Bipower variation + realized-jump ratio (BN&S 2004)             |
| DDDUR     | `research_dddur`     | Drawdown duration statistics (max/mean/median + % underwater)   |

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

- **PSR**: `VERY_LOW` (<0.50) / `LOW` (<0.75) / `MODERATE`
  (<0.90) / `HIGH` (<0.95) / `VERY_HIGH` (≥0.95). Default
  benchmark `SR*=0`; higher PSR = stronger evidence the true SR
  is above the benchmark.
- **ADF**: `STATIONARY` (t < crit_5pct, reject unit root) /
  `BORDERLINE` (crit_5pct ≤ t < crit_10pct) / `NON_STATIONARY`
  (t ≥ crit_10pct). Critical values hardcoded for the
  constant-only model: crit_1pct=-3.43, crit_5pct=-2.86,
  crit_10pct=-2.57 (MacKinnon 1996).
- **MNKENDALL**: `STRONG_UP` (reject, z>0, p<0.001) /
  `UP` (reject, z>0) / `NO_TREND` (|z| does not reject at
  α=0.05) / `DOWN` (reject, z<0) / `STRONG_DOWN` (reject, z<0,
  p<0.001).
- **BIPOWER**: `NO_JUMPS` (ratio<0.05) / `MILD_JUMPS` (<0.20) /
  `NOTABLE_JUMPS` (<0.40) / `HEAVY_JUMPS` (≥0.40). Ratio
  clamped to [0, 1] (negative values from finite-sample noise
  are floored at 0).
- **DDDUR**: `MOSTLY_DRY` (<20% time underwater) /
  `FREQUENT_DD` (<40%) / `PERSISTENT_DD` (<60%) / `DEEP_WATER`
  (≥60%).

PSR uses the existing Abramowitz & Stegun 7.1.26 `std_normal_cdf`
helper. ADF uses hardcoded Dickey-Fuller critical values (no
Monte-Carlo or tabulated polynomial needed for the label-bucket
granularity). Mann-Kendall z-test also uses `std_normal_cdf` for
its two-sided p-value.

## Consequences

### Positive

- **First PSR / higher-moment-corrected Sharpe.** Agents can now
  reason about *confidence* in a positive Sharpe, not just its
  magnitude. A ticker with SR=1.2 and PSR=0.48 is statistically
  indistinguishable from noise; the same SR with PSR=0.96 is a
  strong signal.
- **Formal stationarity / trend battery complete.** HURST
  (continuous persistence) + DFA (noise-robust persistence) +
  ADF (unit-root test, binary) + MNKENDALL (trend presence,
  binary) now cover both continuous and inferential axes of
  mean-reversion / trend diagnosis.
- **Volatility composition available.** BIPOWER jump ratio
  separates jump-driven variance from diffusive variance, a
  qualitative axis the vol-level estimators
  (CLOSEVOL/PARKINSON/GKVOL/RSVOL/VOLOFVOL) do not provide.
- **Drawdown duration fills the last dd axis.** CALMAR (worst
  single) + BURKE (sum-of-squares) + STERLING (mean of N worst)
  + DDDUR (*time* underwater) forms the complete magnitude-and-
  duration drawdown quartet.

### Negative / Risks

- **Schema migration.** `create_research_tables_v31` is additive
  over v30, so peers on v30 who receive v31 rows via LAN sync
  will create the 5 new tables via the existing
  create-before-insert path. No back-compat break.
- **PSR SR scale subtlety.** The PSR formula uses per-period SR
  with per-period skew/kurtosis; the displayed `sharpe` is
  annualized. We carry both internally and report the annualized
  value in UI/packet for consistency with SHARPR, but the denom
  uses the per-period value to avoid scale mismatch. Documented
  in the struct doc-comment.
- **ADF is the lag-0 (classical DF) form, not the "augmented"
  form with k lagged-difference terms.** Standard in applied
  trading literature but worth noting that AR(p) serial
  correlation in Δlog(p) will bias the t-statistic. Agents who
  need strictly ADF(k) should layer LJUNGB on the residuals and
  interpret ADF with caution when `reject_white_noise = true`.
- **Mann-Kendall is O(n²).** For n=253, ~31,878 pairs — fast in
  practice, but noted for any future scaling to multi-year
  windows.
- **DDDUR walker uses closing price only.** Intrabar drawdowns
  (wicks) are ignored by design, consistent with CALMAR/BURKE/
  STERLING which all use close-to-close drawdowns.
- **Packet weight.** Each surface adds ~300-700 bytes per
  symbol. Updated envelope numbers appear in the
  RESEARCH_PACKET.md header.

### Neutral

- **Label-based color scheme continues** the convention
  established in Rounds 24–29 (UP=green for "favorable" label,
  DOWN=red for "adverse", AXIS_TEXT=neutral).
- **Palette aliases** avoid prior bindings. Verified no
  collisions on `PSR`, `ADF`, `MNKENDALL`, `BIPOWER`, `DDDUR`,
  or their aliases.
- **All five surfaces use the same broker handler shape** that
  has been stable since Round 22: `BrokerCmd::Compute*Snapshot →
  tokio::spawn → compute → msg_tx.send(BrokerMsg::*SnapshotMsg)`.
  The receive arm in `update()` both updates UI state and upserts
  to SQLite. LAN sync fans out on the next window.

## Verification

- `cargo test -p typhoon-engine --lib core::research::` — 428
  passing (up from 413 in Round 29, +15 new: 5 roundtrip + 10
  compute tests).
- `cargo check -p typhoon-engine` — clean.
- `cargo check -p typhoon-native` — clean; no palette-alias
  collisions.
- Each new compute surface carries at least one
  deterministic-fixture compute test beyond `INSUFFICIENT_DATA`:
  PSR/BIPOWER share the oscillating ±0.5% fixture (positive and
  negative returns, finite variance); ADF uses the oscillating
  fixture to exercise a mean-reverting series; MNKENDALL uses
  `synthetic_ohlc_bars_150` (monotonically rising close) to
  assert STRONG_UP; DDDUR uses the same monotone fixture to
  assert MOSTLY_DRY with `dd_event_count=0`.

## Packet envelope

After Round 30, single-symbol packet target envelope is **~56-110 KB**
(up from 54-106 in Round 29). Basket (10 symbols via BASKET) is
**~550-1100 KB** (up from 530-1060). Sub-block count grows 138 → 143.

Total HP-local research snapshot count after Round 30: **102**
(97 + 5). Total cross-symbol rank snapshots unchanged.
