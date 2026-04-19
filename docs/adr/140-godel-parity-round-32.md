# ADR-140: Godel Parity Round 32 — ENTROPY / RACHEV / GPR / PACF / APEN

**Status:** Accepted
**Date:** 2026-04-16
**Supersedes/extends:** ADR-108 through ADR-139
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Parity classification

| Feature | Godel Terminal documented | TA-Lib primitive | Research packet | egui popup | Chart overlay |
|---|---|---|---|---|---|
| ENTROPY | No | No | Yes | Yes | No (deferred — ADR-188) |
| RACHEV | No | No | Yes | Yes | No (deferred — ADR-188) |
| GPR | No | No | Yes | Yes | No (deferred — ADR-188) |
| PACF | No | No | Yes | Yes | No (deferred — ADR-188) |
| APEN | No | No | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** pure quant/statistical econometric primitives (Shannon entropy, Rachev tail-asymmetry ratio, Gain-to-Pain + Profit Factor, partial autocorrelation PACF, approximate entropy ApEn) — not documented Godel Terminal features and not TA-Lib catalog entries; classical quant-literature stats.

## Context

Round 31 (ADR-139) shipped HILLTAIL/ARCHLM/PAINRATIO/CUSUM/CFVAR,
completing the nonparametric tail-index, conditional-heteroskedasticity,
L¹ drawdown norm, structural-break, and higher-moment-adjusted
parametric VaR axes. 148 HP-local research sub-blocks now cover
return-, drawdown-magnitude-, drawdown-duration-, distribution-,
persistence-, liquidity-, monthly- / weekday-seasonality-, OHLC-vol-,
tail-expectation-, sign-inference-, sizing-, random-walk-test-,
unit-root-test-, trend-presence-test-, jump-composition-,
tail-index-, conditional-heteroskedasticity-, structural-break-,
and parametric-VaR axes.

Five canonical surfaces remain, each on an axis still missing
from the existing 148 sub-blocks:

1. **No information-theoretic distributional measure.** KURT
   measures tail weight, SKEW measures asymmetry, JBNORM tests
   joint normality — none measures the *information content* or
   *unpredictability* of the return distribution. **Shannon
   entropy** `H = −Σ pᵢ log₂(pᵢ)` over a histogram of daily
   log-returns (bins = ceil(√n)) directly quantifies distributional
   spread in bits. Low H ⇒ concentrated/predictable; high H ⇒
   dispersed/unpredictable. Normalised `H/H_max` gives a [0,1]
   score independent of bin count.

2. **No asymmetric tail comparison ratio.** CVAR reports left-tail
   Expected Shortfall; CFVAR adjusts VaR for moments; TAILR reports
   the quantile ratio pct_95/|pct_05|. None compares left-tail
   *expected loss* to right-tail *expected gain* at matching
   confidence. The **Rachev ratio** = ES_α(+R) / ES_α(−R) gives a
   single number for tail asymmetry in reward-vs-risk terms.
   Rachev > 1 ⇒ upside tail outweighs downside tail. Reported at
   both 5% and 1% levels.

3. **No return-per-realized-loss metric.** Pain Ratio uses
   mean|drawdown|; Omega integrates above/below a threshold.
   Missing: the **Gain-to-Pain Ratio** (Schwager) = Σ rₜ / Σ |min(rₜ,0)|.
   GPR measures net return per unit of total realized dollar loss —
   a different axis from drawdown-based pain. Also reports **Profit
   Factor** = Σ max(rₜ,0) / Σ |min(rₜ,0)| = GPR + 1.

4. **No lag-specific dependence measure.** LJUNGB tests *joint*
   autocorrelation over h lags (Q-statistic). Missing: *which
   specific lag(s)* carry the dependence? **Partial autocorrelation
   (PACF) at lags 1–5** via the Durbin-Levinson recursion reports
   the net correlation at each lag after removing shorter-lag
   effects. Bartlett 95% critical band ±1.96/√n flags significant
   lags. Tells an agent whether lag-1 mean reversion, lag-2
   momentum, etc. are present.

5. **No nonlinear predictability measure.** HURST measures
   long-range dependence, DFA detects trends via detrended
   fluctuation, LJUNGB tests linear autocorrelation, RUNSTEST
   tests sign randomness — all are either linear or long-range.
   **Approximate entropy (ApEn)** (Pincus 1991) measures
   short-range nonlinear regularity: how predictable are the
   *sequential patterns* in returns? Parameters: m=2, r=0.2·σ.
   Low ApEn ⇒ regular, self-similar; high ApEn ⇒ irregular,
   complex dynamics. Captures structure invisible to linear tests.

Round 32 ships these five surfaces as ADR-140. Same additive
envelope as Rounds 5–31: no new fetchers, no cross-symbol scans,
no new external API dependencies. All five compute from the
trailing 253-session window on the existing HP cache.

## Decision

Ship Round 32 as a five-surface additive bundle using schema v33
layered on v32:

| Surface  | Table               | Purpose                                                       |
|----------|---------------------|---------------------------------------------------------------|
| ENTROPY  | `research_entropy`  | Shannon entropy of return distribution (information-theoretic)|
| RACHEV   | `research_rachev`   | Rachev ratio (conditional tail expectation ratio)             |
| GPR      | `research_gpr`      | Gain-to-Pain Ratio + Profit Factor (Schwager)                 |
| PACF     | `research_pacf`     | Partial autocorrelation at lags 1–5 (Durbin-Levinson)         |
| APEN     | `research_apen`     | Approximate entropy (Pincus 1991 nonlinear regularity)        |

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

- **ENTROPY**: `LOW_ENTROPY` (normalised H < 0.50) /
  `MODERATE_ENTROPY` (< 0.70) / `HIGH_ENTROPY` (< 0.85) /
  `VERY_HIGH_ENTROPY` (≥ 0.85). Normalised entropy = H / log₂(bins)
  ∈ [0,1].
- **RACHEV**: `STRONG_LEFT_TAIL` (R₅% < 0.5) / `LEFT_HEAVY`
  (< 0.8) / `SYMMETRIC` (0.8–1.2) / `RIGHT_HEAVY` (> 1.2) /
  `STRONG_RIGHT_TAIL` (> 2.0). Label driven by the 5% Rachev
  ratio.
- **GPR**: `DEEP_PAIN` (GPR < −0.5) / `NEGATIVE` (< 0) /
  `MODEST` (< 0.5) / `GOOD` (< 1.5) / `EXCELLENT` (≥ 1.5).
- **PACF**: `NO_STRUCTURE` (no lag significant at 95%) /
  `LAG1_DOMINANT` (only lag 1 significant) / `LAG_STRUCTURE`
  (multiple lags significant) / `STRONG_STRUCTURE` (max |PACF|
  > 2× critical value). Critical band ±1.96/√n.
- **APEN**: `REGULAR` (ApEn < 0.3) / `MODERATE` (< 0.7) /
  `COMPLEX` (< 1.2) / `HIGHLY_COMPLEX` (≥ 1.2). ApEn is
  clamped to max(0, φ^m − φ^{m+1}) to handle the
  self-match edge effect on periodic data.

## Consequences

### Positive

- **First information-theoretic distributional measure.** ENTROPY
  quantifies return-distribution complexity in bits, orthogonal
  to all existing moment-based (KURT, SKEW) and test-based
  (JBNORM) diagnostics. Normalised score enables cross-symbol
  comparison regardless of bin count.
- **First asymmetric tail comparison.** RACHEV directly answers
  "does the upside tail compensate for the downside tail?"
  at matching confidence levels. Complements TAILR (quantile
  ratio) and CVAR (left-tail only ES).
- **First return-per-realized-loss metric.** GPR fills the gap
  between Sharpe (total vol), Sortino/DOWNVOL (downside dev),
  Omega (threshold integration), and Pain Ratio (drawdown-based
  loss). Profit Factor = GPR + 1 gives the dual gross-gain/loss
  view.
- **First lag-specific dependence structure.** PACF decomposes the
  joint autocorrelation (LJUNGB) into individual lag contributions,
  revealing whether lag-1 mean-reversion, lag-2 momentum, or
  longer-lag calendar effects are present.
- **First nonlinear predictability measure.** APEN captures
  short-range pattern regularity invisible to HURST (long-range),
  DFA (trend), LJUNGB (linear), and RUNSTEST (sign-only).
  Low ApEn on a stock ⇒ returns exhibit repeating micro-patterns
  that a regime-aware agent might exploit.

### Negative / Risks

- **Schema migration.** `create_research_tables_v33` is additive
  over v32, so peers on v32 who receive v33 rows via LAN sync
  will create the 5 new tables via the existing
  create-before-insert path. No back-compat break.
- **Shannon entropy is histogram-dependent.** Bin count = ceil(√n)
  ≈ 16 for n=253. Normalised H/H_max mitigates this, but the
  raw bit count is not directly comparable across different n.
- **Rachev ratio is noisy at small n.** At 5% tail with n=253,
  each tail uses ~13 observations. The ratio of two small-sample
  means can be volatile. Agents should use Rachev alongside CVAR
  and TAILR for a more complete picture.
- **ApEn O(n²) complexity.** For n=253, m=2, this is
  ~63K × 2 = ~126K comparisons per compute — trivially fast
  (sub-millisecond). Would need attention only if the window grew
  to 10K+ bars.
- **ApEn self-match bias.** The Pincus (1991) formulation
  includes self-matches (i==j counted), which can produce
  slightly negative ApEn on perfectly periodic data. Clamped to
  max(0, result) in the implementation.
- **Packet weight.** Each surface adds ~200-600 bytes per
  symbol. Updated envelope numbers appear in the
  RESEARCH_PACKET.md header.

### Neutral

- **Label-based color scheme continues** the convention
  established in Rounds 24–31 (UP=green for "favorable" label,
  DOWN=red for "adverse", AXIS_TEXT=neutral).
- **Palette aliases** avoid prior bindings. Verified no
  collisions on `ENTROPY`, `RACHEV`, `GPR`, `PACF`, `APEN`,
  or their aliases.
- **All five surfaces use the same broker handler shape** that
  has been stable since Round 22.

## Verification

- `cargo test -p typhoon-engine --lib core::research::` — 458
  passing (up from 443 in Round 31, +15 new: 5 roundtrip + 10
  compute tests).
- `cargo check -p typhoon-engine` — clean.
- `cargo check -p typhoon-native` — clean; no palette-alias
  collisions.
- ENTROPY/RACHEV/GPR/PACF use the oscillating ±0.5% fixture;
  APEN uses the oscillating fixture and asserts REGULAR (low
  ApEn ⇒ perfectly alternating pattern is highly predictable).
  GPR oscillating asserts profit_factor > 0 and sum_losses > 0.
  PACF oscillating asserts negative lag-1 PACF (alternation).

## Packet envelope

After Round 32, single-symbol packet target envelope is **~59-116 KB**
(up from 58-114 in Round 31). Basket (10 symbols via BASKET) is
**~580-1160 KB** (up from 570-1140). Sub-block count grows 148 → 153.

Total HP-local research snapshot count after Round 32: **112**
(107 + 5). Total cross-symbol rank snapshots unchanged.
