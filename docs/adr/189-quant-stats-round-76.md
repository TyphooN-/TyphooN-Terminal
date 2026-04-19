# ADR-189: Quant Stats Round 76 — MODSHARPE / HSIEHTEST / CHOWBREAK / DRIFTBURST / HLVCLUST

**Status:** Accepted
**Date:** 2026-04-19
**Supersedes/extends:** ADR-108 through ADR-187
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`, ADR-150 (Quant Stats Round 41), ADR-188 (chart-drawing parity deferred)

## Parity classification

| Feature | Godel Terminal documented | TA-Lib primitive | Research packet | egui popup | Chart overlay |
|---|---|---|---|---|---|
| MODSHARPE | No | No | Yes | Yes | No (deferred — ADR-188) |
| HSIEHTEST | No | No | Yes | Yes | No (deferred — ADR-188) |
| CHOWBREAK | No | No | Yes | Yes | No (deferred — ADR-188) |
| DRIFTBURST | No | No | Yes | Yes | No (deferred — ADR-188) |
| HLVCLUST | No | No | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** pure quant/statistical econometric primitives (Pezier-White adjusted Sharpe, Hsieh third-moment nonlinearity test, Chow structural-break F-test, Christensen-Oomen-Renò drift-burst kernel statistic, Parkinson high-low volatility clustering via Ljung-Box on log-range) — not documented Godel Terminal features and not TA-Lib catalog entries. Classical quant-literature stats, orthogonal to R41 (ADR-150) and R75 (ADR-187).

## Context

Round 75 (ADR-187) shipped five TA-Lib candlestick primitives through the research-layer pipeline, holding chart-drawing parity deferred per ADR-188. The Quant Stats track last advanced in R41 (ADR-150). Five additional econometric surfaces remain orthogonal to the existing suite:

1. **No skew/kurtosis-adjusted risk-return ratio.** The headline Sharpe ratio assumes returns are normal; for fat-tailed / skewed distributions it systematically over- or under-states risk-adjusted performance. The Pezier-White (2006) adjusted Sharpe — ASR = SR · [1 + (S/6)·SR − (EK/24)·SR²] — is the classical closed-form correction. Reports skewness, excess kurtosis, classical SR, ASR, adjustment factor, and a five-level label. Complements R41 classical-SR context.

2. **No explicit nonlinearity screen.** BDSTEST (ADR-149) detects general dependence but does not isolate nonlinearity from linear AR structure. Hsieh (1989) runs BDS / third-moment tests *on AR(1) residuals*, which under true linearity should be white noise. Third cross-moments T(i,j) = E[ε_{t−i} ε_{t−j} ε_t]/σ³ with |z|>1.96 at lag (1,1) or (2,2) indicate nonlinear dynamics the AR fit missed. Orthogonal to BDSTEST (general dependence) and Brock-Dechert-Scheinkman Σ counts.

3. **No structural-break F-test.** CUSUM-type tests detect when a break occurred; the Chow (1960) F-test directly asks whether a break *at* a specified point (we use n/2) is statistically significant. F = [(RSS_p − RSS_u)/k] / [RSS_u/(n−2k)]; large F ⇒ reject "no break at n/2". Fast, parametric, and the canonical first-line structural-change screen in econometrics. Orthogonal to recursive-residual and partial-sum methods.

4. **No kernel-based drift-burst test.** A "drift burst" is a local episode where the signed drift dominates the realised volatility scale — the precursor to flash-crash / momentum-ignition events. The Christensen-Oomen-Renò (2018) drift-burst hypothesis test normalises a kernel-smoothed local drift by a kernel-smoothed local standard deviation: T(t) = √(Σw) · μ̂(t)/σ̂(t). |T(t)|>3 is approximately the 99% pointwise critical value. Reports the max |T(t)| over the window, its signed value, bars-before-end offset, and excursion count. New risk-regime axis the research packet did not previously carry.

5. **No range-based volatility clustering test.** HLVCLUST applies the Parkinson (1980) high-low estimator — σ̂_P(t) = (1/(4 ln 2))·ln(H/L)² — and then runs Ljung-Box directly on the log-range series lr_t = ln(H_t/L_t) at lag h=10. Rejecting white noise on lr_t confirms volatility clustering *without* a returns-based GARCH fit and *without* requiring close-to-close return data. Orthogonal to MCLEODLI (returns-squared LB) and ARCHLM (LM regression on squared residuals) because the range carries intra-bar volatility information that close-to-close squared returns miss.

Round 76 ships these five surfaces as ADR-189. Same additive envelope as R41: no new fetchers, no cross-symbol scans, no new external API dependencies. All five compute from the trailing 253-session window on the existing HP cache.

## Decision

Ship Round 76 as a five-surface additive bundle using schema v78 layered on v77:

| Surface     | Table                  | Purpose                                                          |
|-------------|------------------------|------------------------------------------------------------------|
| MODSHARPE   | `research_modsharpe`   | Pezier-White adjusted Sharpe (skew/kurtosis-corrected)           |
| HSIEHTEST   | `research_hsiehtest`   | Hsieh third-moment nonlinearity test on AR(1) residuals          |
| CHOWBREAK   | `research_chowbreak`   | Chow mean-shift structural break F-test at n/2                   |
| DRIFTBURST  | `research_driftburst`  | Christensen-Oomen-Renò drift-burst kernel statistic              |
| HLVCLUST    | `research_hlvclust`    | Parkinson high-low volatility clustering via Ljung-Box on lr_t   |

Each table follows the established JSON-blob-per-symbol shape:

```sql
CREATE TABLE research_<name> (
    symbol TEXT PRIMARY KEY,
    snapshot_json TEXT NOT NULL DEFAULT '{}',
    updated_at INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_research_<name>_updated ON research_<name>(updated_at);
```

Each snapshot carries a regime `label` field (3–5 active buckets + `INSUFFICIENT_DATA` sentinel). Label strings:

- **MODSHARPE**: `STRONG_POS` (ASR_ann ≥ 1.0) / `MODERATE_POS` (≥ 0.5) / `WEAK` (|ASR| < 0.5) / `MODERATE_NEG` (≤ −0.5) / `STRONG_NEG` (≤ −1.0).
- **HSIEHTEST**: `LINEAR` (max|z| ≤ 1.96) / `MILD_NONLIN` (≤ 2.58) / `STRONG_NONLIN` (> 2.58).
- **CHOWBREAK**: `NO_BREAK` (F ≤ 3.84) / `MILD_BREAK` (< 6.63) / `STRONG_BREAK` (≥ 6.63).
- **DRIFTBURST**: `NO_BURST` (max|T| ≤ 3) / `MILD_BURST` (≤ 4) / `STRONG_BURST` (> 4).
- **HLVCLUST**: `NO_CLUST` (Q ≤ χ²_95) / `MILD_CLUST` (< 2·crit) / `STRONG_CLUST` (otherwise).

## Consequences

### Positive

- **Closes the skew/kurtosis-adjusted Sharpe gap.** MODSHARPE is the canonical Pezier-White correction that matches how quant funds benchmark risk-adjusted performance when returns are non-Gaussian. Complements the classical SR already reported elsewhere.
- **Adds a nonlinearity filter that BDSTEST lacks.** HSIEHTEST isolates nonlinearity from linear dependence by fitting AR(1) first — a property the BDS test by itself does not supply.
- **First explicit structural-break F-test.** CHOWBREAK is the canonical parametric break-detection stat; complements change-point and CUSUM-type approaches by answering the "at *this* point?" question directly.
- **New drift-burst risk-regime axis.** DRIFTBURST covers an episode-detection dimension (flash-crash / momentum-ignition precursors) that no prior research surface carried.
- **Range-based clustering estimator without GARCH machinery.** HLVCLUST uses only H and L per bar — widely available even in cached daily data — and produces a clustering Q-stat without the parametric overhead of fitting GARCH(1,1).
- **No new external dependencies, no fetcher expansion.** Pure econometric compute on the HP cache; same additive envelope as R41/R75.

### Negative / Risks

- **Schema migration.** `create_research_tables_v78` is additive over v77, so peers on v77 who receive v78 rows via LAN sync will create the 5 new tables via the existing create-before-insert path. No back-compat break.
- **ASR approximation is third-order.** The Pezier-White formula uses a Cornish-Fisher expansion truncated at the fourth moment. For returns with very heavy tails (|EK| > 5) the approximation under-states tail risk; the label thresholds are wide enough that this rarely flips the bucket, but the documented `note` flags it when |EK| > 5.
- **Hsieh test is fixed-AR-order.** We set AR(1) regardless of BIC/AIC. Higher-order linear dependence can leak into the residuals and inflate T(i,j). Documented; for sharp inference the user should cross-check against an explicit VAR-order-selection package.
- **Chow split point fixed at n/2.** The F-test assumes the break location is known *a priori*; choosing n/2 gives maximum statistical power when the break is central, lower power when it is near an edge. The label is a *central-break screen*; for unknown-location tests the user should cross-check against Andrews (1993) sup-F or Bai-Perron multiple-break tests.
- **Drift-burst bandwidth is fixed.** We use bw=10 bars for the Gaussian kernel. Smaller bandwidths increase detection sensitivity at the cost of false positives; larger bandwidths smooth real bursts. The label thresholds are at |T|>3 (99% pointwise) / >4 (≈99.97%) so occasional single-bar spurious excursions rarely flip the bucket.
- **Parkinson assumes log-normal price inside the bar.** Gaps and overnight returns violate this. The HLVCLUST test uses the log-range as the clustering observable, which is robust to the Parkinson σ-estimation bias — the clustering structure in lr_t is preserved even when the point estimate of σ is biased.
- **Packet weight.** MODSHARPE adds ~250 bytes, HSIEHTEST ~220, CHOWBREAK ~260, DRIFTBURST ~200, HLVCLUST ~260. Total Round 76 addition: ~1.2 KB/symbol. Updated envelope numbers appear in the RESEARCH_PACKET.md header.

### Neutral

- **Label-based color scheme continues** the convention established in R24–R75 (UP=green for "favorable" label, DOWN=red for "adverse", AXIS_TEXT=neutral).
- **Palette aliases** avoid prior bindings. Verified no collisions on `MODSHARPE`/`ADJSHARPE`/`PEZIER_WHITE`, `HSIEHTEST`/`HSIEH`/`HSIEH_NONLIN`/`NONLIN_3RDMOM`, `CHOWBREAK`/`CHOW`/`CHOW_TEST`/`STRUCT_BREAK`, `DRIFTBURST`/`DRIFT_BURST`/`COR18`/`KERNEL_DRIFT`, or `HLVCLUST`/`PARKINSON_CLUST`/`HL_CLUSTER`/`HL_VOLCLUST`.
- **All five surfaces use the same broker handler shape** that has been stable since R22.
- **Chart overlay remains deferred per ADR-188.** The classification table above explicitly marks chart overlay `No (deferred — ADR-188)` for all five surfaces. No `native/src/chart/` changes land with R76.

### Paid-API gap (for later revisit)

Same as ADR-150 / ADR-187. The gaps remain data-access-gated (intraday bars, order-book depth, options IV surfaces, corporate actions feeds, realised-variance matrices). No Round 76 surface needed any of these; all compute from the daily HP cache.

## Verification

- `cargo test -p typhoon-engine --lib` — 909 research tests passing (up from 899 in R75, +10 new: 5 roundtrip + 5 compute_oscillating).
- `cargo check -p typhoon-engine` — clean.
- `cargo check -p typhoon-native` — clean; no palette-alias collisions.
- MODSHARPE/HSIEHTEST/CHOWBREAK/DRIFTBURST/HLVCLUST compute_oscillating use the ±0.5% oscillating fixture (150 bars, 149 log-returns). Each asserts the returned label belongs to its regime set, scalars are finite when label is not INSUFFICIENT_DATA, and axis-specific invariants:
  - **MODSHARPE**: adjustment_factor finite, sharpe_ratio and adjusted_sharpe finite; when |EK|>5 the `note` flags the Cornish-Fisher approximation caveat.
  - **HSIEHTEST**: ar_order=1, critical_95=1.96, max_abs_z=max(|z_11|,|z_22|); reject_null iff max_abs_z>1.96.
  - **CHOWBREAK**: break_point_idx=n/2, k_regressors=1, df_num=1, df_den=n−2, rss_pooled ≥ rss_unrestricted, f_stat ≥ 0.
  - **DRIFTBURST**: kernel_bandwidth_bars=10, critical_99_approx=3.0, max_at_offset<bars_used, excursions_gt_3 ≤ bars_used.
  - **HLVCLUST**: lag_h=10, parkinson_vol_bar ≥ 0, parkinson_vol_annualised=parkinson_vol_bar·√252, critical_95=χ²_95(10)≈18.307, reject_null iff lb_q_stat>critical_95.
- `engine/src/core/lan_sync.rs` — 5 new tables added to `SYNCABLE_TABLES`, 5 `create_table_sql` branches, 5 `table_timestamp_column` branches (all `updated_at`).

## Packet envelope

After Round 76, single-symbol packet target envelope is **~75-148 KB** (up from ~74-147 in Round 75). Basket (10 symbols via BASKET) is **~750-1480 KB** (up from ~740-1470). Sub-block count grows 242 → 247.

Total HP-local research snapshot count after Round 76: **222** (217 + 5). Total cross-symbol rank snapshots unchanged.
