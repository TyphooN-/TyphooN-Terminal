# ADR-147: Godel Parity Round 39 — GARCH11 / SADF / CORDIM / SKSPEC / AUTOMI

**Status:** Accepted
**Date:** 2026-04-16
**Supersedes/extends:** ADR-108 through ADR-146
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Context

Round 38 (ADR-146) shipped BNSJUMP/PPROOT/MFDFA/HILLKS/TSI, pushing
HP-local research surfaces to 142 and per-symbol sub-blocks to 183 spanning
28+ orthogonal analytical dimensions. Five canonical surfaces remain, each
on an axis still missing.

1. **No parametric volatility-persistence model.** EWMAVOL (ADR-143) gives
   a single-parameter RiskMetrics estimate; RENYIENT/VOLOFVOL cover
   distributional / vol-of-vol axes. Nothing yet fits the Bollerslev 1986
   GARCH(1,1) model — the industry-standard 2-parameter persistence
   decomposition σ²_t = ω + α·r²_{t-1} + β·σ²_{t-1}. Key diagnostics:
   α (shock weight), β (persistence weight), α+β (total persistence),
   unconditional variance ω/(1−α−β), and half-life ln(0.5)/ln(α+β).
   GARCH11 ships these via coordinate-descent grid-MLE over (α, β) with
   ω implied by the unconditional-variance constraint — simpler than
   full-gradient BFGS, robust for n=253 daily bars.

2. **No bubble / explosive-root test.** ADF (ADR-126), KPSS (ADR-144),
   and now PPROOT (ADR-146) all test stationarity. What they cannot
   detect is the asymmetric case: a series that is stationary for most
   of the sample but exhibits an explosive (root > 1) region near the
   end. Phillips-Wu-Yu (2011) Sup-ADF expands a window from r0 forward,
   computing an ADF t-stat at each step; the sup over the window is
   compared to a simulated critical value. Exceeding the critical value
   is evidence that a rational bubble is active in the most recent
   segment. SADF ships sup ADF-t, argmax window end, a tabulated 5%
   critical value for n ∈ {100, 200, 500}, and a four-level label.

3. **No nonlinear-dynamics dimension estimate.** HURST, DFA, HIGUCHI,
   and MFDFA all measure scaling. None measures the *effective
   dimensionality* of the return dynamics. Grassberger-Procaccia (1983)
   correlation dimension D2 answers this: embed the return series in m
   dimensions, compute the correlation integral C(ε) = fraction of
   m-vector pairs within ε, and D2 = d log C(ε) / d log ε over the
   scaling region. For pure white noise, D2 ≈ m; for low-dimensional
   chaos, D2 saturates at a finite non-integer value. CORDIM ships D2
   at m=3, fit R², and a four-level label (LOW_DIM / MODERATE_DIM /
   HIGH_DIM / STOCHASTIC).

4. **No skewness-stability diagnostic.** We have RETQUANT (ADR-135)
   reporting the distributional moments over the full window, but no
   measure of how *stable* the skew is over time. A trend-following
   strategy facing persistent positive skew behaves very differently
   from one facing skew that flips sign month-to-month. SKSPEC rolls a
   30-bar window, computes the skew at each endpoint, and reports
   mean/std/min/max/range of the skew series — plus a four-level label
   (STABLE_POSITIVE / STABLE_NEGATIVE / DRIFTING / UNSTABLE) based on
   |mean| vs std.

5. **No information-theoretic autocorrelation.** Standard ACF only
   catches linear dependence. Financial returns routinely have zero
   linear ACF but strong nonlinear dependence (volatility clustering is
   the canonical example). Auto-mutual-information I(X_t; X_{t-k})
   catches *any* statistical dependence at lag k — the
   information-theoretic generalisation of ACF. AUTOMI ships MI at
   lags 1/5/10 using equiprobable histogram binning with k=8 bins, the
   marginal entropy H(X), and a normalised ratio MI(1)/H(X) ∈ [0, 1]
   expressing the fraction of marginal information shared between
   consecutive bars.

Round 39 ships these five surfaces as ADR-147. Same additive envelope
as Rounds 5–38: no new fetchers, no cross-symbol scans, no new external
API dependencies. All five compute from the trailing 253-session window
on the existing HP cache.

## Decision

Ship Round 39 as a five-surface additive bundle using schema v40
layered on v39:

| Surface   | Table                  | Purpose                                                             |
|-----------|------------------------|---------------------------------------------------------------------|
| GARCH11   | `research_garch11`     | Bollerslev 1986 GARCH(1,1) persistence / unconditional-var fit       |
| SADF      | `research_sadf`        | Phillips-Wu-Yu 2011 Sup-ADF bubble / explosive-root test            |
| CORDIM    | `research_cordim`      | Grassberger-Procaccia 1983 correlation dimension D2                 |
| SKSPEC    | `research_skspec`      | Rolling-window skewness spectrum / stability                        |
| AUTOMI    | `research_automi`      | Lag-1/5/10 auto-mutual-information (info-theoretic ACF)             |

Each table follows the established JSON-blob-per-symbol shape:

```sql
CREATE TABLE research_<name> (
    symbol TEXT PRIMARY KEY,
    snapshot_json TEXT NOT NULL DEFAULT '{}',
    updated_at INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_research_<name>_updated ON research_<name>(updated_at);
```

Each snapshot carries a regime `label` field (4–5 active buckets +
`INSUFFICIENT_DATA` sentinel). Label strings:

- **GARCH11**: `NEAR_INTEGRATED` (α+β≥0.99) / `HIGH_PERSISTENCE`
  (>0.95) / `MODERATE_PERSISTENCE` (>0.85) / `LOW_PERSISTENCE`
  (otherwise).
- **SADF**: `EXPLOSIVE_CONFIRMED` (SADF>1.5·crit) / `EXPLOSIVE_LIKELY`
  (>crit) / `BORDERLINE` (>0.8·crit) / `STABLE` (otherwise).
- **CORDIM**: `LOW_DIM` (D2<1.5) / `MODERATE_DIM` (<2.5) / `HIGH_DIM`
  (<3.5) / `STOCHASTIC` (otherwise).
- **SKSPEC**: `STABLE_POSITIVE` (|mean|>2·std, mean>0) /
  `STABLE_NEGATIVE` (|mean|>2·std, mean<0) / `DRIFTING` (|mean|>std) /
  `UNSTABLE` (otherwise).
- **AUTOMI**: `STRONG` (MI(1)/H(X)>0.20) / `MODERATE` (>0.10) / `WEAK`
  (>0.03) / `INDEPENDENT` (otherwise).

## Consequences

### Positive

- **First parametric volatility persistence fit.** EWMAVOL's single-λ
  decay gives a RiskMetrics-style short estimate, but GARCH11 is the
  industry-standard 2-parameter decomposition — α (shock response), β
  (persistence), α+β (half-life). Regimes with α+β→1 (near-integrated
  GARCH / IGARCH) are a well-documented tail-risk flag.
- **First explosive-root / bubble detector.** ADF/KPSS/PPROOT test
  stationarity *over the full window*; SADF tests whether a recent
  sub-window is in an explosive regime. The Phillips-Wu-Yu paper
  explicitly positioned this as a bubble-detection statistic for
  equities, and it has seen wide academic adoption. Complements the
  three existing stationarity tests by asking the *asymmetric* question.
- **First nonlinear-dynamics dimension surface.** Hurst, DFA, and
  Higuchi all reduce to scaling exponents that assume self-similarity.
  Correlation dimension D2 is a different beast — it quantifies the
  effective dimensionality of the return dynamics in the m-dimensional
  embedding space. Low D2 indicates the system is close to a
  low-dimensional attractor (possibly chaotic); high or saturating D2
  indicates near-stochastic behaviour.
- **First skewness-stability diagnostic.** RETQUANT ships total-window
  skew; SKSPEC answers "is that skew *reliably* positive, or is it an
  artefact of a few windows?". Valuable for strategies whose P&L
  depends on skew persistence — e.g. put-selling strategies care
  deeply about the sign AND the stability of skew over time.
- **First information-theoretic ACF.** Classical ACF only sees linear
  dependence. AUTOMI catches *any* dependence — this is the signature
  of volatility clustering, which contributes ~zero to the linear ACF
  of returns but dominates the MI of |returns|. A symbol with strong
  AUTOMI but near-zero linear ACF is highlighted as a candidate for
  volatility-based modelling even when simple linear filters say
  "no signal".

### Negative / Risks

- **Schema migration.** `create_research_tables_v40` is additive over
  v39, so peers on v39 who receive v40 rows via LAN sync will create
  the 5 new tables via the existing create-before-insert path. No
  back-compat break.
- **GARCH11 uses grid MLE, not gradient descent.** Full BFGS with
  analytic gradients is the conventional approach, but for n≈253 daily
  returns the 21×21 grid over (α, β) ∈ [0, 0.4] × [0.4, 0.99] with
  0.02 resolution is robust and within the ms-per-symbol envelope.
  Gradient fitting would refine the estimate but has been shown to be
  unstable at this sample size. Honest trade-off — shipping robust
  estimates over precise ones.
- **SADF critical values are tabulated, not simulated.** Phillips
  et al. (2011) report Monte-Carlo critical values for the finite-sample
  null distribution. We use values at n ∈ {100, 200, 500} and interpolate
  linearly in n. For n outside this range the nearest-endpoint value is
  used. This is conservative for large n; for small n it may slightly
  over-reject. Noted for future revisit if a finer null-distribution
  table is wanted.
- **CORDIM is sensitive to series length and scaling region.** At n≈253
  the Grassberger-Procaccia estimate is known to be biased upward for
  true low-dim attractors. We fit 10 log-spaced radii from 0.1 to ~1.0
  of the standardised-return range — the two-decade middle range where
  the estimate is least biased. Still, treat D2 as an ordinal signal
  (LOW vs HIGH) rather than a precise dimension estimate.
- **SKSPEC uses skewness of raw returns, not log-returns.** Both are
  defensible; raw returns are slightly more intuitive for trader
  interpretation. The label thresholds |mean|/std are unit-less so the
  choice doesn't affect the regime classification.
- **AUTOMI histogram binning biases the estimate.** Equiprobable k=8
  bins give a low-variance MI estimate but under-resolve the
  distribution tails. Alternative estimators (KSG, kernel-density) are
  more accurate but ~10× slower. For the terminal's "is this bar
  related to the last one beyond what linear ACF shows" question, k=8
  histograms are adequate. Cross-checks for independence should ideally
  use the KSG estimator — a future refinement.
- **Packet weight.** Each surface adds ~200–500 bytes per symbol.
  CORDIM and AUTOMI are lightest (5–6 scalars); GARCH11 is heaviest
  (8 scalars). Updated envelope numbers appear in the
  RESEARCH_PACKET.md header.

### Neutral

- **Label-based color scheme continues** the convention established in
  Rounds 24–38 (UP=green for "favorable" label, DOWN=red for
  "adverse", AXIS_TEXT=neutral).
- **Palette aliases** avoid prior bindings. Verified no collisions on
  `GARCH11`, `SADF`, `CORDIM`, `SKSPEC`, `AUTOMI`, or their aliases.
- **All five surfaces use the same broker handler shape** that has been
  stable since Round 22.

### Paid-API gap (for later revisit)

The user's new directive (2026-04-16) is explicit: "continue until we
have reached godel terminal feature parity (as long as we can achieve
without paid APIs). if we need Paid APIs for some features to complete,
note this in ADR so we can revisit later."

After Round 39, the remaining godel-equivalent surfaces that would
*require* paid / auth-gated APIs:

- **Intraday 1m/5m bars** for true high-frequency realised measures
  (BNSJUMP at daily frequency is conservative — the Barndorff-Nielsen
  theory was developed for intraday data). Would require a live market
  data feed (Polygon.io, IEX Cloud, or similar).
- **Order-book depth snapshots** for Kyle-style market-impact estimation
  at the top-of-book. KYLELAM (ADR-136) uses the Amihud-illiquidity
  approximation on daily OHLC which is a coarse proxy. True Kyle λ
  would need L2 order book (Polygon.io stream, Databento, or direct
  exchange feed).
- **Options-chain IV surfaces** for model-free variance swap pricing
  and risk-neutral moments. Would need per-strike IV (CBOE DataShop,
  or OptionMetrics for historical surfaces).
- **Corporate actions feeds** (splits, dividends, M&A) with higher
  fidelity than Yahoo's RSS — for proper total-return series
  reconstruction on thinly-traded tickers.
- **Realised-volatility / realised-correlation matrices** derived from
  intraday ticks across a basket — requires the same intraday feed as
  the first bullet.

These are all *data-access-gated*, not compute-gated. The research
compute layer in `research.rs` can consume higher-frequency inputs
trivially — the gap is purely in fetchers. Revisit when a paid-feed
integration is in scope.

## Verification

- `cargo test -p typhoon-engine --lib core::research::` — 534
  passing (up from 524 in Round 38, +10 new: 5 roundtrip + 5
  compute_oscillating).
- `cargo check -p typhoon-engine` — clean.
- `cargo check -p typhoon-native` — clean; no palette-alias
  collisions.
- GARCH11/SADF/CORDIM/SKSPEC/AUTOMI compute_oscillating use the ±0.5%
  oscillating fixture. Each asserts the returned label belongs to its
  regime set, scalars are finite when label is not INSUFFICIENT_DATA,
  and axis-specific invariants: GARCH11 α∈[0,1], β∈[0,1]; SADF
  critical>0; CORDIM embed_dim=3; SKSPEC window_size=30; AUTOMI
  num_bins=8, H(X)≥0.

## Packet envelope

After Round 39, single-symbol packet target envelope is **~65-131 KB**
(up from 64-129 in Round 38). Basket (10 symbols via BASKET) is
**~650-1310 KB** (up from 640-1290). Sub-block count grows 183 → 188.

Total HP-local research snapshot count after Round 39: **147**
(142 + 5). Total cross-symbol rank snapshots unchanged.
