# ADR-134: Godel Parity Round 26 — CALMAR / ULCER / VARRATIO / AMIHUD / JBNORM

**Status:** Accepted
**Date:** 2026-04-15
**Supersedes/extends:** ADR-108 through ADR-133
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Parity classification

| Feature | Godel Terminal documented | TA-Lib primitive | Research packet | egui popup | Chart overlay |
|---|---|---|---|---|---|
| CALMAR | No | No | Yes | Yes | No (deferred — ADR-188) |
| ULCER | No | No | Yes | Yes | No (deferred — ADR-188) |
| VARRATIO | No | No | Yes | Yes | No (deferred — ADR-188) |
| AMIHUD | No | No | Yes | Yes | No (deferred — ADR-188) |
| JBNORM | No | No | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** pure quant/statistical risk-return + econometric primitives (Calmar ratio, Ulcer index + Martin, Lo-MacKinlay variance ratio, Amihud illiquidity, Jarque-Bera normality test) — not documented Godel Terminal features and not TA-Lib catalog entries; classical quant-literature stats.

## Context

Round 25 (ADR-133) shipped the last cluster of "single-series
return-distribution risk metrics" — downside deviation / Sortino
(DOWNVOL), Sharpe ratio (SHARPR), Kaufman efficiency ratio (EFFRATIO),
wick bias (WICKBIAS), and vol-of-vol (VOLOFVOL). With Rounds 22–25
the packet now carries all the standard moment-based, autocorrelation,
persistence, bar-anatomy, and risk/return ratio surfaces the
literature names.

Five textbook surfaces remain that use the existing HP cache but
haven't been built:

1. **No drawdown-adjusted return metric.** The packet reports max
   drawdown (DDHIST) and annualized return (computable from HP) but
   nowhere combines them into a ratio. Calmar is the canonical
   "return per unit of drawdown pain" number and is the standard
   comparison metric in CTA / trend-following / managed-futures
   contexts.
2. **No continuous drawdown risk measure.** DDHIST reports the max
   drawdown event (deepest trough). But max drawdown is a single
   extreme and doesn't capture the *average* drawdown experience.
   The Ulcer index — `sqrt(mean(dd²))` — is the canonical continuous
   drawdown-weighted risk metric, and the Martin ratio (UPI =
   return / ulcer) is the drawdown-analogue of Sharpe.
3. **No formal random-walk hypothesis test.** HURST, AUTOCOR, and
   MRHL are *descriptive* statistics — they measure persistence,
   lag-k ACF, and shock decay. None is a formal *test* with a
   z-statistic and implicit p-value. The Lo-MacKinlay variance
   ratio is exactly that test: VR(q) = 1 under H₀ (random walk),
   with a known asymptotic z-distribution.
4. **No microstructure liquidity scalar.** VOLRATIO compares
   up-day vs down-day volume; the Liquidity snapshot from Round 13
   uses fundamentals-derived fields. But neither computes
   `|return| / dollar_volume` — the canonical Amihud (2002) price
   impact measure that captures how much the price moves per dollar
   traded. This is the foundational single-number liquidity metric
   in market microstructure.
5. **No combined normality test.** RETSKEW and RETKURT report
   skewness and excess kurtosis separately. But the reader who
   wants to know "are these returns normally distributed?" has to
   mentally combine both numbers. The Jarque-Bera test folds them
   into a single `JB = (n/6)(S² + K²/4)` statistic with an exact
   p-value for χ²(2), giving a clear actionable answer.

Round 26 ships these five surfaces as ADR-134. Same additive envelope
as Rounds 5–25: no new fetchers, no cross-symbol scans, no new
external API dependencies. All five compute from the trailing
253-session window on the existing `research_historical_price` cache.

## Decision

Ship Round 26 as a five-surface additive bundle using schema v27
(v26 was ADR-133's five-surface bundle):

1. **CALMAR — Calmar Ratio.** `annualized_return / max_drawdown`.
   Trims to last 253 bars, computes total return `(last/first - 1)`,
   annualizes by `× (252/bars)`, walks the series tracking running
   peak and max drawdown. Reports total_return_pct, annualized_return_pct,
   max_drawdown_pct, calmar_ratio. Labels: VERY_POOR (<0.5) / POOR (<1)
   / NEUTRAL (<2) / GOOD (<3) / EXCELLENT (≥3 or zero-drawdown with
   positive return) / INSUFFICIENT_DATA.
2. **ULCER — Ulcer Index + Martin Ratio (UPI).** Walks the same
   trailing 253-bar window tracking running peak; at each bar
   computes `dd_pct = (close - peak) / peak × 100` (always ≤ 0).
   `ulcer_index = sqrt(mean(dd_pct²))`. Reports ulcer_index,
   mean_drawdown_pct, max_drawdown_pct, pct_in_drawdown (share of
   bars strictly below peak), annualized_return_pct, and
   `martin_ratio = annualized_return / ulcer_index`. Labels from
   ulcer_index: LOW_PAIN (<2) / MILD (<5) / MODERATE (<10) /
   HIGH (<20) / SEVERE (≥20) / INSUFFICIENT_DATA.
3. **VARRATIO — Lo-MacKinlay Variance Ratio.** Uses demeaned log
   returns. `VR(q) = Var(q-period overlapping returns) /
   (q × Var(1-period returns))`. Computes at horizons 2/5/10/20
   plus asymptotic z-statistics for horizons 2 and 5. Labels from
   VR(5): STRONG_REVERT (<0.7) / MEAN_REVERT (<0.9) /
   RANDOM_WALK (0.9–1.1) / TRENDING (<1.3) / STRONG_TREND (≥1.3) /
   INSUFFICIENT_DATA. Requires ≥40 log returns.
4. **AMIHUD — Amihud Illiquidity Ratio.** Iterates bar pairs:
   `illiq_t = |log_return_t| / (close_t × volume_t)`, scaled by
   1e6. Skips bars with zero dollar volume. Reports mean, median,
   90th percentile of the daily ILLIQ series, plus average daily
   dollar volume. Labels from mean_illiq: VERY_LIQUID (<0.01) /
   LIQUID (<0.1) / MODERATE (<1) / ILLIQUID (<10) /
   VERY_ILLIQUID (≥10) / INSUFFICIENT_DATA. Requires ≥20 valid
   bar pairs.
5. **JBNORM — Jarque-Bera Normality Test.** Computes sample
   skewness S and excess kurtosis K over the trailing 253-session
   log returns. `JB = (n/6)(S² + K²/4)`. Under H₀ (normality),
   JB ~ χ²(2), so `p = exp(-JB/2)` (exact). Labels from p-value:
   NORMAL (>0.10) / MILD_DEPARTURE (>0.05) / MODERATE_DEPARTURE
   (>0.01) / NON_NORMAL (>0.001) / STRONGLY_NON_NORMAL (≤0.001)
   / INSUFFICIENT_DATA.

## Engine changes (`engine/src/core/research.rs`)

1. **5 new snapshot structs** under the `// ── ADR-134 Round 26 —
   HP calmar / ulcer / variance-ratio / amihud / jarque-bera ──`
   divider:
   - `CalmarRatioSnapshot`
   - `UlcerIndexSnapshot`
   - `VarianceRatioSnapshot`
   - `AmihudIlliqSnapshot`
   - `JarqueBeraSnapshot`

2. **5 new compute functions**:
   - `compute_calmar_snapshot` — trims to 253, walks peak tracker,
     computes max dd, ratio.
   - `compute_ulcer_snapshot` — walks peak tracker, accumulates
     dd² for ulcer, computes Martin ratio.
   - `compute_varratio_snapshot` — demeaned returns, overlapping
     q-period variance, z-stats.
   - `compute_amihud_snapshot` — iterates bar pairs, |r|/dvol,
     sorts for median/90th.
   - `compute_jbnorm_snapshot` — central moments m2/m3/m4,
     skewness/kurtosis, JB stat, exact χ²(2) p-value.

3. **Schema v27** — `create_research_tables_v27` (layered on v26)
   adds `research_calmar`, `research_ulcer`, `research_varratio`,
   `research_amihud`, `research_jbnorm`.

4. **5 upsert/get wrapper pairs**.

## LAN sync changes (`engine/src/core/lan_sync.rs`)

- Added 5 new entries to `SYNCABLE_TABLES` under
  `// ── ADR-134 Round 26 ────────────────────────────`.
- Added 5 new arms to `create_table_sql()`.
- Added 5 new arms to `table_timestamp_column()` → `updated_at`.

## Native changes (`native/src/app.rs`)

- **5 BrokerCmd variants**: `ComputeCalmarSnapshot`,
  `ComputeUlcerSnapshot`, `ComputeVarratioSnapshot`,
  `ComputeAmihudSnapshot`, `ComputeJbnormSnapshot`.
- **5 BrokerMsg variants**: `CalmarSnapshotMsg` …
  `JbnormSnapshotMsg`.
- **5 state field blocks** + default initializers.
- **5 broker handlers** (HP-pure).
- **5 BrokerMsg receive arms** with upsert + UI state update.
- **5 egui windows** with color schemes:
  - CALMAR: GOOD/EXCELLENT → UP, VERY_POOR/POOR → DOWN.
  - ULCER: LOW_PAIN → UP, HIGH/SEVERE → DOWN.
  - VARRATIO: TRENDING/STRONG_TREND → UP, STRONG_REVERT → DOWN.
  - AMIHUD: VERY_LIQUID/LIQUID → UP, ILLIQUID/VERY_ILLIQUID → DOWN.
  - JBNORM: NORMAL → UP, NON_NORMAL/STRONGLY_NON_NORMAL → DOWN.
- **5 command palette entries** with aliases:
  - `CALMAR | CALMAR_RATIO | CALMARRATIO`
  - `ULCER | ULCER_INDEX | ULCERINDEX | MARTIN | UPI`
  - `VARRATIO | VAR_RATIO | VARIANCE_RATIO | LO_MACKINLAY`
  - `AMIHUD | AMIHUD_ILLIQ | ILLIQ | ILLIQUIDITY`
  - `JBNORM | JB | JARQUE_BERA | NORMALITY`
- Removed stale `AMIHUD` alias from the existing LIQ palette entry
  (Round 13) to avoid unreachable pattern warning — the AMIHUD
  command now routes to the proper Amihud illiquidity surface.
- **5 packet generator blocks** inside `investigate_symbols()`.

## Research packet changes (`docs/RESEARCH_PACKET.md`)

- Header sub-block count: 118 → 123.
- New sections 2.117 CALMAR / 2.118 ULCER / 2.119 VARRATIO /
  2.120 AMIHUD / 2.121 JBNORM.
- Renumbered INGESTED 2.117 → 2.122.
- Renumbered Sector peer comparison 2.118 → 2.123.
- 5 new size-caps rows and 5 new data source rows.
- Updated packet size envelope: 46-90 KB → 48-94 KB single-symbol,
  450-900 KB → 470-940 KB basket.
- Added ADR-134 to the Related list.

## Alternatives considered

1. **Sterling ratio instead of Calmar** — Uses average of top N
   drawdowns instead of just the max. More robust but less standard.
   Rejected: Calmar is the canonical number and ULCER already
   provides the "average drawdown" view via mean_drawdown_pct.
2. **Calmar using geometric annualization** — `(1 + total)^(252/n) - 1`
   instead of linear `total × 252/n`. Rejected: arithmetic
   annualization matches the convention in most Calmar discussions
   and avoids complexity with negative returns.
3. **Calmar infinity for zero-drawdown positive returns** — Considered
   emitting `f64::INFINITY`. Rejected: calmar_ratio stays at 0.0
   but label emits EXCELLENT — keeps the JSON serialization clean
   and readers get the right message from the label.
4. **Ulcer index on log returns instead of prices** — Rejected.
   The canonical Ulcer index (Peter Martin, 1987) is defined on
   price drawdowns, not return drawdowns.
5. **Ulcer mean_drawdown_pct as absolute value** — Rejected: the
   drawdown is definitionally non-positive (close ≤ peak); the
   sign carries information and matches DDHIST's convention.
6. **Variance ratio using non-overlapping returns only** — Rejected.
   Lo-MacKinlay (1988) uses overlapping returns for efficiency;
   non-overlapping wastes most of the sample. The z-statistic
   formula accounts for the overlap.
7. **Variance ratio labelling from VR(2) instead of VR(5)** — VR(2)
   is noisier and over-sensitive to bid-ask bounce. VR(5) is the
   more commonly cited horizon in the literature.
8. **Amihud using close-to-close returns instead of log returns** —
   Rejected: log returns are the standard in the academic literature
   for Amihud's ILLIQ and are consistent with every other HP
   surface.
9. **Amihud without the 1e6 scaling** — Raw Amihud values are on
   the order of 1e-10 for liquid names, making them hard to read.
   The ×1e6 scaling gives "basis points of price impact per
   million dollars traded" which is the conventional presentation.
10. **Amihud adding a trend slope** — Considered adding linear
    regression slope of daily ILLIQ over the window to capture
    "is liquidity improving or deteriorating?" Rejected: adds
    complexity and is better as a separate surface if needed.
11. **Jarque-Bera using finite-sample correction** — The standard
    JB test uses `n/6`; a small-sample correction divides by
    `(n-1)(n-2)` etc. Rejected: at n=252 the difference is
    negligible and the standard form is what readers expect.
12. **Jarque-Bera reporting a formal "reject at α" decision** —
    Rejected: the p-value is strictly more informative. The label
    bands are effectively the decision at α = 0.10 / 0.05 / 0.01 /
    0.001 anyway.
13. **Combining CALMAR + ULCER into one snapshot** — Both use the
    drawdown series. Rejected for the same reason as every other
    round: separate snapshots, separate LAN sync, separate windows.
14. **Combining JBNORM with RETSKEW/RETKURT** — The JB test is a
    function of skewness and kurtosis, so the inputs are the same.
    Rejected: RETSKEW/RETKURT are descriptive views of the
    distribution shape; JBNORM is a hypothesis test. Different
    surfaces serve different reading modes.

## Consequences

- **Coverage**: After Round 26, the research packet has ≥123
  per-symbol sub-blocks. Round 26 closes out drawdown-adjusted
  returns, continuous drawdown risk, the formal random-walk test,
  microstructure liquidity, and normality testing.
- **Database growth**: ~5 KB per symbol added (5 tables × ~1 KB).
- **LAN sync**: 5 new rows per symbol per sync window.
- **Packet size**: +2 KB typical, +4 KB worst case per symbol.
- **No new external data sources**.
- **First formal hypothesis test**: JBNORM is the first surface in
  the packet that reports a p-value. VARRATIO's z-statistics also
  carry implicit p-values for the random-walk null.
- **Palette collision fixed**: The stale "AMIHUD" alias on the
  LIQ palette entry (Round 13) has been removed. AMIHUD now
  routes to the proper Amihud illiquidity surface from this round.

## Implementation notes

- **CALMAR zero-drawdown handling**: A monotonically rising series
  has max_drawdown_pct ≈ 0. In that case calmar_ratio = 0.0 (not
  infinity) but the label emits EXCELLENT — this is the best
  possible Calmar outcome and shouldn't be penalized.
- **CALMAR annualization**: Linear annualization `total × 252 / n`
  is used. For negative total returns this produces a negative
  annualized return, which divides by positive max_drawdown to
  give a negative Calmar — correctly capturing "you lost money
  AND had drawdowns."
- **ULCER drawdown convention**: `dd_pct = (close - peak) / peak × 100`
  is always ≤ 0. The ulcer index squares this, so the sign doesn't
  matter for the index itself. `mean_drawdown_pct` preserves the
  sign for interpretation. `max_drawdown_pct` is the most negative
  value in the series.
- **ULCER pct_in_drawdown**: Counts bars strictly below the running
  peak (dd_pct < -ε). A series that hits new highs every day has
  0% in drawdown.
- **VARRATIO overlapping returns**: For horizon q, computes
  `sum_{i..i+q}(demeaned_returns)` for all valid start positions
  i, then takes the variance. This is the Lo-MacKinlay overlapping
  estimator with maximum efficiency.
- **VARRATIO z-statistic formula**: Under IID returns,
  `se(VR(q)) = sqrt(2(q-1) / (3qn))`. The z-stat is
  `(VR(q) - 1) / se`. This is the homoscedastic Lo-MacKinlay
  z-statistic. The heteroscedastic variant (which adjusts for
  conditional heteroscedasticity) is not implemented — VOLCLUSTER
  already handles that view.
- **VARRATIO minimum bars**: Requires ≥40 log returns. At q=20
  this gives 21 overlapping windows, which is the minimum for
  a stable variance estimate.
- **AMIHUD dollar volume**: Uses `close × volume` as the dollar
  volume proxy. This is the standard approximation when VWAP is
  not available.
- **AMIHUD bar pairs**: Iterates `windows(2)` to get
  `log(close_t / close_{t-1})` for the return and uses
  `close_t × volume_t` for dollar volume at time t.
- **AMIHUD label thresholds**: 0.01 / 0.1 / 1.0 / 10.0 on the
  1e6-scaled ILLIQ. These correspond roughly to large-cap /
  mid-cap / small-cap / micro-cap liquidity tiers.
- **JBNORM exact p-value**: For χ²(2), the CDF is
  `1 - exp(-x/2)`, so `p-value = exp(-JB/2)`. This is exact,
  not an approximation.
- **JBNORM moment computation**: Uses biased central moments
  `m_k = Σ(r - mean)^k / n` (not n-1). This matches the
  standard JB formulation in the econometrics literature.

## Test coverage

- 5 roundtrip tests (one per new surface).
- 10 compute tests: calmar_insufficient, calmar_positive,
  ulcer_insufficient, ulcer_rising, varratio_insufficient,
  varratio_random, amihud_insufficient, amihud_liquid,
  jbnorm_insufficient, jbnorm_pvalue_chi2.
- Engine test suite: 352 → 367 research tests passing
  (+15 = 5 roundtrip + 10 compute).

## Future work

Continue the Godel-parity arc with additional surfaces:

- **CORRSTK** — still deferred; benchmark cache availability.
- **TLRANK** — still deferred; 30-day ADV$ peer scan.
- **DFA (detrended fluctuation analysis)** — alternate Hurst
  estimator more robust to non-stationarity.
- **GARCH(1,1) fit** — parametric vol model; natural refinement
  of VOLCLUSTER + VOLOFVOL.
- **Volume-weighted return stats** — vw-mean / vw-vol using HP
  close × volume.
- **Dollar-volume turnover rank** — liquidity tier using
  close × volume ranked across sector.
- **Monthly-seasonality hit-rate** — share of positive months
  per calendar month.
- **Omega ratio** — upside/downside area ratio of the return
  distribution; extends DOWNVOL.
- **Burke ratio** — return / sqrt(sum of squared drawdowns);
  natural companion to CALMAR + ULCER.
- **Roll's spread estimator** — estimated bid-ask spread from
  serial covariance of returns.
- **Kyle's lambda** — price impact coefficient from regressing
  returns on signed volume; extends AMIHUD.
- **Robust Jarque-Bera** — D'Agostino-Pearson omnibus test as
  an alternative normality test more robust to small samples.
