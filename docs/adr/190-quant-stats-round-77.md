# ADR-190: Quant Stats Round 77 — YANGZHANG / KUIPER / DAGOSTINO / BAIPERRON / KUPIECPOF

**Status:** Accepted
**Date:** 2026-04-19
**Supersedes/extends:** ADR-108 through ADR-189
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`, ADR-150 (Quant Stats Round 41), ADR-188 (chart-drawing parity deferred), ADR-189 (Quant Stats Round 76)

## Parity classification

| Feature | Godel Terminal documented | TA-Lib primitive | Research packet | egui popup | Chart overlay |
|---|---|---|---|---|---|
| YANGZHANG | No | No | Yes | Yes | No (deferred — ADR-188) |
| KUIPER | No | No | Yes | Yes | No (deferred — ADR-188) |
| DAGOSTINO | No | No | Yes | Yes | No (deferred — ADR-188) |
| BAIPERRON | No | No | Yes | Yes | No (deferred — ADR-188) |
| KUPIECPOF | No | No | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** pure quant/statistical econometric primitives (Yang-Zhang drift-independent range-vol, Kuiper two-sided CDF goodness-of-fit, D'Agostino-Pearson K² omnibus normality, Bai-Perron sup-F structural break with interior search, Kupiec Proportion-of-Failures VaR backtest) — not documented Godel Terminal features and not TA-Lib catalog entries. Classical quant-literature stats, orthogonal to R41 (ADR-150) and R76 (ADR-189).

## Context

Round 76 (ADR-189) shipped five econometric surfaces (MODSHARPE, HSIEHTEST, CHOWBREAK, DRIFTBURST, HLVCLUST) through the research-layer pipeline, holding chart-drawing parity deferred per ADR-188. Five additional orthogonal surfaces remain:

1. **No drift-independent range-volatility estimator.** R26 shipped Parkinson (high-low), R27 shipped Garman-Klass (OHLC), R28 shipped Rogers-Satchell (drift-independent OHLC). Yang-Zhang (2000) is the three-component minimum-variance combination σ²_YZ = σ²_O + k·σ²_C + (1−k)·σ²_RS that is simultaneously drift-independent *and* handles opening-gap jumps by including an overnight variance term. k = 0.34 / (1.34 + (n+1)/(n−1)) is the closed-form weight that minimises estimator variance. For bar data with overnight gaps this is the textbook-optimal range-vol estimator; Parkinson/Garman-Klass are strictly-dominated special cases. Reports σ²_O, σ²_C, σ²_RS, k, σ_YZ/bar, σ_YZ annualised, σ_CC annualised, and the efficiency ratio σ_CC/σ_YZ (higher ⇒ YZ is more efficient than close-to-close).

2. **No two-sided CDF goodness-of-fit test.** R39 shipped Kolmogorov-Smirnov one-sample vs normal using D = max |F_n(x) − F(x)|. KS is one-sided in the sense that it captures the single largest gap. Kuiper (1960) V = D⁺ + D⁻ sums the two extreme deviations separately, which gives substantially more power in the tails (the regions where stock-return distributions typically fail normality). Stephens (1970) finite-n modification V* = V · (√n + 0.155 + 0.24/√n) lets us compare directly to a finite-n critical value (≈1.747 at 95%). Orthogonal to JARQUEBERA (moment-based) and the existing KS surface (max-one-sided).

3. **No individual-moment normality decomposition.** JARQUEBERA reports one combined statistic; the user cannot see whether skewness or kurtosis is driving the rejection. D'Agostino-Pearson (1973) K² decomposes the omnibus normality test into z_skew (D'Agostino 1970 skewness transform) and z_kurt (Anscombe-Glynn 1983 kurtosis transform), then combines as K² = z_skew² + z_kurt² ~ χ²(2). We label the outcome as `SKEW_DOMINANT` / `KURT_DOMINANT` / `BOTH_DEPART` / `NORMAL` based on which component individually exceeds the 1.96 two-tail cutoff — a diagnostic dimension JB does not supply.

4. **Chow test assumes known break location.** R76 shipped CHOWBREAK with the break fixed at n/2. When the break location is *unknown*, sup-F over an interior interval [π₀·n, (1−π₀)·n] with π₀ = 0.15 (Andrews 1993 trimming) is the canonical solution. Bai-Perron (1998) extends this to multi-break search; the single-break variant we ship reports the argmax break index, sup-F, pre/post means, RSS at the break and under H₀, and the Andrews (1993) critical value ≈ 8.58. Orthogonal to CHOWBREAK because the sup over location is a strictly-different test.

5. **No VaR backtest.** JARQUEBERA / DAGOSTINO / KUIPER test distribution shape; they do not evaluate whether a *VaR threshold computed from past data* is accurate going forward. Kupiec (1995) Proportion-of-Failures test is the regulatory-standard VaR-coverage backtest: build a rolling historical-VaR_{α=0.95} on the first `rolling_window` bars (we use 60), count exceedances in the remaining test window, and compare the realised exceedance rate to the nominal α via the likelihood ratio LR_POF ~ χ²(1). We label the outcome `GOOD_FIT` / `OVER_ESTIMATED` (too few exceedances ⇒ VaR is too conservative) / `UNDER_ESTIMATED` (too many exceedances ⇒ VaR under-states risk). Orthogonal to every other R41-R76 surface because none of them evaluate predicted-vs-realised threshold breaches.

Round 77 ships these five surfaces as ADR-190. Same additive envelope as R41/R76: no new fetchers, no cross-symbol scans, no new external API dependencies. All five compute from the trailing 253-session window on the existing HP cache.

## Decision

Ship Round 77 as a five-surface additive bundle using schema v79 layered on v78:

| Surface     | Table                  | Purpose                                                                         |
|-------------|------------------------|---------------------------------------------------------------------------------|
| YANGZHANG   | `research_yangzhang`   | Yang-Zhang drift-independent 3-component range-volatility estimator             |
| KUIPER      | `research_kuiper`      | Kuiper two-sided CDF goodness-of-fit statistic vs standard normal               |
| DAGOSTINO   | `research_dagostino`   | D'Agostino-Pearson K² omnibus normality with skew/kurt decomposition            |
| BAIPERRON   | `research_baiperron`   | Bai-Perron sup-F structural break search over Andrews-trimmed interior          |
| KUPIECPOF   | `research_kupiecpof`   | Kupiec (1995) Proportion-of-Failures VaR backtest (rolling 60-bar window)       |

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

- **YANGZHANG**: `VERY_LOW` (σ_YZ(ann) < 10%) / `LOW` (< 20%) / `MODERATE` (< 40%) / `HIGH` (< 60%) / `VERY_HIGH` (≥ 60%).
- **KUIPER**: `NORMAL` (V* ≤ 1.620, 90% crit) / `MILD_DEPART` (≤ 1.747) / `STRONG_DEPART` (> 1.747).
- **DAGOSTINO**: `NORMAL` (|z_skew| ≤ 1.96 ∧ |z_kurt| ≤ 1.96) / `SKEW_DOMINANT` (skew exceeds, kurt does not) / `KURT_DOMINANT` (kurt exceeds, skew does not) / `BOTH_DEPART` (both exceed).
- **BAIPERRON**: `NO_BREAK` (sup-F ≤ 8.58) / `MILD_BREAK` (< 2×crit) / `STRONG_BREAK` (otherwise).
- **KUPIECPOF**: `GOOD_FIT` (LR_POF ≤ 3.841) / `OVER_ESTIMATED` (reject, realised rate < α) / `UNDER_ESTIMATED` (reject, realised rate > α).

## Consequences

### Positive

- **Closes the drift-independent + gap-robust range-vol gap.** YANGZHANG subsumes R26/R27/R28 as the minimum-variance combination; where those surfaces each drop one estimator leg (overnight / opening gap / drift), YZ carries all three and reports the efficiency ratio σ_CC/σ_YZ that quantifies how much statistical precision the range-based estimator buys over close-to-close.
- **First two-sided CDF goodness-of-fit in the packet.** KUIPER complements the one-sided KS surface and gives meaningfully higher power when tail departures are symmetric — which is the typical stock-return failure mode.
- **JB-complementary normality diagnostic.** DAGOSTINO decomposes the omnibus K² into skew and kurtosis legs, so the reader can see which moment is driving normality rejection — a diagnostic dimension JB does not supply.
- **First unknown-location structural-break test.** BAIPERRON answers the "is there *any* break over the interior?" question that CHOWBREAK (fixed-location) cannot answer. The argmax break index is directly actionable.
- **First VaR backtest.** KUPIECPOF is the canonical regulatory-standard VaR-coverage test and gives the packet a predicted-vs-realised evaluation dimension orthogonal to every other surface.
- **No new external dependencies, no fetcher expansion.** Pure econometric compute on the HP cache; same additive envelope as R41/R76.

### Negative / Risks

- **Schema migration.** `create_research_tables_v79` is additive over v78, so peers on v78 who receive v79 rows via LAN sync will create the 5 new tables via the existing create-before-insert path. No back-compat break.
- **YZ assumes bars are homogeneous.** The k-weight derivation requires returns/ranges across bars to be i.i.d. Violations under volatility clustering bias the estimator upward slightly; the label thresholds are wide enough that this rarely flips the bucket.
- **Kuiper p-value is approximate.** We use the Stephens (1970) large-n asymptotic; for n < 30 the approximation breaks down and the `note` flags it. Label buckets are stable against this because they read V* against the fixed critical value, not the p-value directly.
- **D'Agostino transforms require n ≥ 20.** Both the D'Agostino (1970) skew transform and the Anscombe-Glynn (1983) kurtosis transform are derived under asymptotic-normality arguments that degrade below n=20. We enforce a 20-bar minimum and emit `INSUFFICIENT_DATA` otherwise.
- **Bai-Perron is single-break.** We ship only the k=1 sup-F search, not the full multi-break dynamic-programming variant. For known single-break screening this is sufficient; for compound-break detection the user should cross-check against an explicit multi-break implementation.
- **Kupiec tests coverage only, not independence.** The POF test is unconditional — it does not check whether exceedances cluster in time. Christoffersen (1998) adds a conditional-coverage extension (joint POF + independence) which we do not ship in R77; users wanting clustering diagnostics should cross-check HLVCLUST (R76) on the same window.
- **Packet weight.** YANGZHANG adds ~240 bytes, KUIPER ~220, DAGOSTINO ~230, BAIPERRON ~260, KUPIECPOF ~280. Total Round 77 addition: ~1.23 KB/symbol. Updated envelope numbers appear in the RESEARCH_PACKET.md header.

### Neutral

- **Label-based color scheme continues** the convention established in R24–R76 (UP=green for "favorable" label, DOWN=red for "adverse", AXIS_TEXT=neutral).
- **Palette aliases** avoid prior bindings. Verified no collisions on `YANGZHANG`/`YZ_VOL`/`YZVOL`/`YZ_RANGEVOL`/`YANGZHANGWIN` (the pre-existing VolEstimator bind at app.rs:35344 matches the narrower alias `YANG_ZHANG`, so we picked `YZ_RANGEVOL` instead to avoid the unreachable-pattern warning), `KUIPER`/`KUIPERV`/`KUIPER_GOF`, `DAGOSTINO`/`DAGOSTINO_K2`/`K2TEST`/`K2OMNIBUS`, `BAIPERRON`/`BAIPERRON_SUPF`/`SUPF`/`SUP_F_BREAK`, `KUPIECPOF`/`KUPIEC_POF`/`POFTEST`/`VAR_BACKTEST`.
- **All five surfaces use the same broker handler shape** that has been stable since R22.
- **Chart overlay remains deferred per ADR-188.** The classification table above explicitly marks chart overlay `No (deferred — ADR-188)` for all five surfaces. No `native/src/chart/` changes land with R77.

### Paid-API gap (for later revisit)

Same as ADR-150 / ADR-187 / ADR-189. The gaps remain data-access-gated (intraday bars, order-book depth, options IV surfaces, corporate actions feeds, realised-variance matrices). No Round 77 surface needed any of these; all compute from the daily HP cache.

## Verification

- `cargo test -p typhoon-engine --lib` — 919 research tests passing (up from 909 in R76, +10 new: 5 roundtrip + 5 compute_oscillating).
- `cargo check -p typhoon-engine` — clean.
- `cargo check -p typhoon-native` — clean; no palette-alias collisions (YANG_ZHANG collision with VolEstimator resolved by picking YZ_RANGEVOL alias).
- YANGZHANG/KUIPER/DAGOSTINO/BAIPERRON/KUPIECPOF compute_oscillating use the ±0.5% oscillating fixture (150 bars, 149 log-returns). Each asserts the returned label belongs to its regime set, scalars are finite when label is not INSUFFICIENT_DATA, and axis-specific invariants:
  - **YANGZHANG**: overnight_var ≥ 0, open_to_close_var ≥ 0, rs_component ≥ 0, 0 < k_weight < 1, yz_vol_annualised_pct = yz_vol_bar·√252·100, efficiency_vs_close = cc_vol_annualised_pct / yz_vol_annualised_pct.
  - **KUIPER**: v_stat = d_plus + d_minus, v_stat_adj = v_stat·(√n + 0.155 + 0.24/√n), critical_95 = 1.747, reject_null iff v_stat_adj > critical_95.
  - **DAGOSTINO**: k2_stat = z_skew² + z_kurt², critical_95 = 5.991, reject_null iff k2_stat > critical_95; label consistent with which of |z_skew|, |z_kurt| exceed 1.96.
  - **BAIPERRON**: trim_fraction = 0.15, search_lo = ⌈0.15n⌉, search_hi = ⌊0.85n⌋, best_break_idx ∈ [search_lo, search_hi], rss_at_best ≤ rss_no_break, critical_95 ≈ 8.58, reject_null iff sup_f_stat > critical_95.
  - **KUPIECPOF**: confidence_level = 0.95, nominal_exceedance_rate = 0.05, rolling_window = 60, test_window = bars_used − rolling_window, critical_95 = 3.841, n_exceedances ≤ test_window, expected_exceedances = test_window·α, reject_null iff lr_pof_stat > critical_95.
- `engine/src/core/lan_sync.rs` — 5 new tables added to `SYNCABLE_TABLES`, 5 `create_table_sql` branches, 5 `table_timestamp_column` branches (all `updated_at`).

## Packet envelope

After Round 77, single-symbol packet target envelope is **~76-149 KB** (up from ~75-148 in Round 76). Basket (10 symbols via BASKET) is **~760-1493 KB** (up from ~750-1480). Sub-block count grows 247 → 252.

Total HP-local research snapshot count after Round 77: **227** (222 + 5). Total cross-symbol rank snapshots unchanged.
