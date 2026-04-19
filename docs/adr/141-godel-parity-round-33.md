# ADR-141: Godel Parity Round 33 — UPR / LEVEREFF / DRAWDAR / VARHALF / GINI

**Status:** Accepted
**Date:** 2026-04-16
**Supersedes/extends:** ADR-108 through ADR-140
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Parity classification

| Feature | Godel Terminal documented | TA-Lib primitive | Research packet | egui popup | Chart overlay |
|---|---|---|---|---|---|
| UPR | No | No | Yes | Yes | No (deferred — ADR-188) |
| LEVEREFF | No | No | Yes | Yes | No (deferred — ADR-188) |
| DRAWDAR | No | No | Yes | Yes | No (deferred — ADR-188) |
| VARHALF | No | No | Yes | Yes | No (deferred — ADR-188) |
| GINI | No | No | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** pure quant/statistical econometric primitives (Upside Potential Ratio, Black leverage effect, Drawdown-at-Risk / CDaR, volatility half-life, Gini concentration of move magnitudes) — not documented Godel Terminal features and not TA-Lib catalog entries; classical quant-literature stats.

## Context

Round 32 (ADR-140) shipped ENTROPY/RACHEV/GPR/PACF/APEN,
completing the information-theoretic, asymmetric-tail-comparison,
return-per-realized-loss, lag-specific-dependence, and nonlinear-
predictability axes. 153 per-symbol research sub-blocks now cover
return-, drawdown-magnitude-, drawdown-duration-, distribution-,
persistence-, liquidity-, monthly- / weekday-seasonality-, OHLC-vol-,
tail-expectation-, sign-inference-, sizing-, random-walk-test-,
unit-root-test-, trend-presence-test-, jump-composition-,
tail-index-, conditional-heteroskedasticity-, structural-break-,
parametric-VaR-, information-theoretic-, asymmetric-tail-comparison-,
return-per-loss-, lag-specific-dependence-, and nonlinear-
predictability axes.

Five canonical surfaces remain, each on an axis still missing
from the existing 153 sub-blocks:

1. **No asymmetric upside-capture-vs-downside-risk ratio.** Sharpe
   uses total volatility, Sortino uses downside deviation, Omega
   integrates above/below a threshold — none separates *upside
   potential* from *downside risk* as distinct moment orders. The
   **Upside Potential Ratio** (Sortino & van der Meer 1991) =
   UPM₁(MAR) / √LPM₂(MAR) with MAR=0. UPM₁ = mean of max(r,0)
   captures first-moment upside capture; √LPM₂ = sqrt(mean of
   min(r,0)²) measures second-moment downside risk. UPR > 1 ⇒
   upside potential exceeds downside risk on a risk-adjusted basis.

2. **No return→volatility feedback measure.** VOLCLUSTER measures
   temporal persistence of vol (ACF of |r| and r²); ARCHLM tests
   conditional heteroskedasticity; VOLOFVOL measures vol dispersion.
   None measures the **directional feedback** from returns to future
   vol. The **leverage effect** (Black 1976): corr(rₜ, rₜ₊₁²)
   measures whether negative returns amplify subsequent volatility.
   Also reports asymmetric vol ratio = σ(down-days) / σ(up-days).
   Strong negative correlation + ratio > 1 ⇒ classic leverage
   effect (equity risk premium compresses after losses).

3. **No quantile-based drawdown risk measure.** DDHIST reports
   descriptive drawdown statistics (max, longest, counts); CVAR
   measures return-tail Expected Shortfall; CALMAR/BURKE/STERLING
   are return-per-drawdown ratios. None reports the drawdown
   analog of VaR/CVaR. **Drawdown-at-Risk** (Chekhlov et al.
   2005): DaR(α) = quantile of running drawdown series at
   confidence α; CDaR(α) = mean of drawdowns exceeding DaR(α).
   Reported at both 5% and 1%. DaR tells an agent: "95% of trading
   days have drawdowns ≤ DaR(5%)."

4. **No volatility-regime persistence measure.** VOLCLUSTER measures
   temporal clustering (ACF); VOLOFVOL measures vol dispersion (CV);
   MRHL measures return mean-reversion speed. None measures how
   quickly **volatility shocks** dissipate. **Volatility half-life**:
   fit AR(1) on rolling 20-day realized vol → HL = −ln(2)/ln(β).
   Fast HL (< 5 days) ⇒ vol spikes revert quickly (short-lived
   event); slow HL (> 30 days) ⇒ persistent vol regime changes
   (structural shift). Complementary to MRHL (return-level) and
   VOLCLUSTER (presence of clustering).

5. **No return-magnitude concentration measure.** KURT measures
   tail weight (fourth moment); BIPOWER decomposes variance into
   continuous + jump; VOLCLUSTER tests temporal clustering. None
   measures the **distributional concentration** of move sizes.
   The **Gini coefficient** on |log returns|:
   G = (2·Σ(i·|r|_sorted)) / (n·Σ|r|) − (n+1)/n. High Gini ⇒
   a few outsized moves dominate total absolute return (fat-tail
   concentration); low Gini ⇒ moves are evenly distributed.
   Orthogonal to KURT (measures tail weight, not concentration)
   and BIPOWER (measures jump share, not size distribution).

Round 33 ships these five surfaces as ADR-141. Same additive
envelope as Rounds 5–32: no new fetchers, no cross-symbol scans,
no new external API dependencies. All five compute from the
trailing 253-session window on the existing HP cache.

## Decision

Ship Round 33 as a five-surface additive bundle using schema v34
layered on v33:

| Surface  | Table                | Purpose                                                       |
|----------|----------------------|---------------------------------------------------------------|
| UPR      | `research_upr`       | Upside Potential Ratio (Sortino & van der Meer 1991)          |
| LEVEREFF | `research_levereff`  | Leverage effect (Black 1976 return→vol feedback)              |
| DRAWDAR  | `research_drawdar`   | Drawdown-at-Risk + CDaR (Chekhlov et al. 2005)               |
| VARHALF  | `research_varhalf`   | Volatility half-life (AR(1) on rolling RV)                    |
| GINI     | `research_gini`      | Gini coefficient on |returns| (magnitude concentration)       |

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

- **UPR**: `POOR` (UPR < 0.3) / `BELOW_AVERAGE` (< 0.6) /
  `AVERAGE` (< 1.0) / `GOOD` (< 1.5) / `EXCELLENT` (≥ 1.5).
- **LEVEREFF**: `STRONG_INVERSE` (corr < −0.3) /
  `MODERATE_INVERSE` (< −0.1) / `WEAK_OR_NONE` (−0.1 to 0.1) /
  `POSITIVE_LEVERAGE` (≥ 0.1). Label driven by corr(rₜ, rₜ₊₁²).
- **DRAWDAR**: `LOW_DD_RISK` (DaR_5% < 3%) / `MODERATE` (< 8%) /
  `ELEVATED` (< 15%) / `HIGH` (≥ 15%).
- **VARHALF**: `FAST_REVERT` (HL < 5d) / `MODERATE` (< 15d) /
  `SLOW` (< 30d) / `PERSISTENT` (≥ 30d). HL = −ln(2)/ln(β).
- **GINI**: `LOW_CONCENTRATION` (Gini < 0.3) / `MODERATE` (< 0.5) /
  `HIGH` (< 0.7) / `VERY_HIGH` (≥ 0.7).

## Consequences

### Positive

- **First asymmetric capture-vs-risk ratio.** UPR separates
  first-moment upside potential from second-moment downside risk,
  filling the gap between Sharpe (total vol), Sortino (downside
  dev), and Omega (threshold integration).
- **First return→volatility feedback measure.** LEVEREFF quantifies
  the Black (1976) leverage effect — does bad news amplify future
  vol? — orthogonal to VOLCLUSTER (temporal persistence) and
  ARCHLM (conditional heteroskedasticity detection).
- **First quantile-based drawdown risk measure.** DRAWDAR provides
  the drawdown analog of VaR/CVaR, enabling probability-based
  drawdown budgeting distinct from descriptive DDHIST and
  ratio-based CALMAR/BURKE/STERLING.
- **First vol-regime persistence measure.** VARHALF tells agents
  whether a vol spike is likely to dissipate quickly or persist,
  complementing VOLCLUSTER (existence of clustering) and VOLOFVOL
  (vol dispersion) with a time-to-mean-revert scalar.
- **First return-concentration measure.** GINI captures whether
  total absolute return is driven by a few outsized moves (high
  concentration) or evenly distributed (low concentration),
  orthogonal to KURT (tail weight) and BIPOWER (jump share).

### Negative / Risks

- **Schema migration.** `create_research_tables_v34` is additive
  over v33, so peers on v33 who receive v34 rows via LAN sync
  will create the 5 new tables via the existing
  create-before-insert path. No back-compat break.
- **UPR denominator near-zero.** When all returns are non-negative
  (e.g. monotonic rise), LPM₂ = 0 and √LPM₂ = 0. Protected by
  the `INSUFFICIENT_DATA` sentinel when the denominator is too
  small. In practice, 253 trading days always contain enough
  down days.
- **LEVEREFF requires lag-1 pairs.** Uses n−1 pairs (rₜ, rₜ₊₁²),
  requiring ≥21 bars. With the 253-bar window, this yields ~252
  pairs — ample for Pearson correlation.
- **DRAWDAR quantile estimation.** At 1% tail with n=253, the 1%
  quantile uses the ~2nd-3rd worst drawdown observation. CDaR(1%)
  averages ~2 observations. Agents should use DRAWDAR alongside
  DDHIST for a fuller picture.
- **VARHALF AR(1) model risk.** Volatility dynamics are not
  strictly AR(1); GARCH(1,1) would be more realistic. However,
  AR(1) half-life is the standard practitioner approximation and
  requires no iterative MLE.
- **Gini on |returns| vs raw returns.** Using absolute returns
  avoids sign-cancellation issues but makes the coefficient
  always ∈ [0,1]. This is the standard finance application
  (measuring return-size concentration, not inequality in
  signed returns).
- **Packet weight.** Each surface adds ~200-600 bytes per
  symbol. Updated envelope numbers appear in the
  RESEARCH_PACKET.md header.

### Neutral

- **Label-based color scheme continues** the convention
  established in Rounds 24–31 (UP=green for "favorable" label,
  DOWN=red for "adverse", AXIS_TEXT=neutral).
- **Palette aliases** avoid prior bindings. Verified no
  collisions on `UPR`, `LEVEREFF`, `DRAWDAR`, `VARHALF`, `GINI`,
  or their aliases.
- **All five surfaces use the same broker handler shape** that
  has been stable since Round 22.

## Verification

- `cargo test -p typhoon-engine --lib core::research::` — 473
  passing (up from 458 in Round 32, +15 new: 5 roundtrip + 10
  compute tests).
- `cargo check -p typhoon-engine` — clean.
- `cargo check -p typhoon-native` — clean; no palette-alias
  collisions.
- UPR/LEVEREFF/VARHALF/GINI use the oscillating ±0.5% fixture;
  DRAWDAR uses monotonically rising synthetic bars (LOW_DD_RISK).
  UPR oscillating asserts upr > 0 and sqrt_lpm2 > 0.
  LEVEREFF oscillating asserts negative corr_r_vol (alternation
  creates return→vol anti-correlation). VARHALF oscillating asserts
  half_life_days > 0. GINI oscillating asserts gini ∈ [0,1].

## Packet envelope

After Round 33, single-symbol packet target envelope is **~60-118 KB**
(up from 59-116 in Round 32). Basket (10 symbols via BASKET) is
**~590-1180 KB** (up from 580-1160). Sub-block count grows 153 → 158.

Total HP-local research snapshot count after Round 33: **117**
(112 + 5). Total cross-symbol rank snapshots unchanged.
