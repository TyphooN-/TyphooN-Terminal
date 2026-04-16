# ADR-137: Godel Parity Round 29 — STERLING / KELLYF / LJUNGB / RUNSTEST / ZERORET

**Status:** Accepted
**Date:** 2026-04-15
**Supersedes/extends:** ADR-108 through ADR-136
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Context

Round 28 (ADR-136) shipped PARKINSON/GKVOL/RSVOL/CVAR/DOWEFFECT —
completing the OHLC-vol family (range-only, full-OHLC-zero-drift,
drift-independent), the coherent tail-risk measure (Expected
Shortfall), and the weekday calendar axis to pair with MONTHSEAS.
133 HP-local research sub-blocks now cover the return-, drawdown-,
distribution-, persistence-, liquidity-, monthly-calendar-, weekday-,
OHLC-vol- and tail-expectation axes.

Five canonical surfaces remain, each on an axis still missing from
the existing 133 sub-blocks:

1. **No drawdown-adjusted return scalar using the mean of the worst
   drawdowns.** CALMAR (Round 26) uses the *single* worst
   drawdown; BURKE (Round 27) uses the sum-of-squared drawdowns
   (quadratic penalty). The textbook *Sterling* ratio — annualized
   return divided by the arithmetic mean of the N worst distinct
   drawdown events (canonical N=5) — is the average-based middle
   ground. It smooths out the "lucky-escape" fragility of CALMAR
   (one bad event dominates) without over-penalizing clusters the
   way BURKE does. With CALMAR/BURKE/STERLING present, an agent
   can cross-check: CALMAR ≫ STERLING ⇒ worst drawdown was a
   single tail outlier; CALMAR ≈ STERLING ⇒ the top-5 drawdowns
   are of similar magnitude.

2. **No forward-looking position-sizing scalar.** SHARPR, SORTINO,
   CALMAR, BURKE, STERLING, OMEGA all measure *realized*
   risk-adjusted performance — backwards-looking ratios of return
   to various risk denominators. The *Kelly fraction*
   `f* = (b·p − q) / b` (Kelly 1956) is forward-looking: given
   the return distribution, what stake size maximizes expected
   log-wealth? It is the canonical answer to "how much of the
   portfolio should this asset get?" and pairs with CALMAR/BURKE
   the way an optimization target pairs with performance diagnostics.

3. **No joint autocorrelation test.** AUTOCOR reports the
   individual-lag ACF at k=1, 5, 10, 20 — four separate numbers.
   The Ljung-Box Q-statistic `Q = n(n+2)·Σ(ρ_k²/(n−k))` for
   k=1..h is the *joint* test: is the combined autocorrelation
   across lags 1..h significantly different from zero? It returns
   a single p-value against the "returns are white noise" null —
   the canonical econometrics test for model adequacy. With
   AUTOCOR + LJUNGB an agent sees both the per-lag shape and the
   joint significance.

4. **No formal randomness test for sign sequences.** RUNLEN
   (Round 23) is descriptive: longest/mean streak of consecutive
   up-or-down days. The *Wald-Wolfowitz runs test* converts this
   to an inferential statistic: given n₁ positive and n₂ negative
   days, under the null of random order the number of runs has
   mean `2n₁n₂/n + 1` and variance `2n₁n₂(2n₁n₂−n)/(n²(n−1))`.
   A z-statistic and two-sided p-value lets an agent say "sign
   sequence is clustered / random / anti-clustered" with formal
   significance — RUNLEN alone does not.

5. **No third microstructure liquidity proxy.** AMIHUD (Round 26)
   reports price-impact per $ of volume; ROLLSPRD (Round 27)
   reports implicit bid-ask spread from first-lag covariance. The
   *Lesmond-Ogden-Trzcinka (1999)* zero-return-day fraction is the
   third foundational scalar: the proportion of bars with
   `|log_return| < ε` (default 1e-6). Illiquid securities show
   more zero-return days (dealers don't update the close because
   nobody traded). Different mechanism from AMIHUD (impact) and
   ROLLSPRD (spread), and particularly useful for small-cap /
   emerging / fixed-income assets where volume data is thin.

Round 29 ships these five surfaces as ADR-137. Same additive
envelope as Rounds 5–28: no new fetchers, no cross-symbol scans,
no new external API dependencies. All five compute from the
trailing 253-session window on the existing HP cache.

## Decision

Ship Round 29 as a five-surface additive bundle using schema v30
layered on v29:

| Surface   | Table              | Purpose                                                            |
|-----------|--------------------|--------------------------------------------------------------------|
| STERLING  | `research_sterling`| Sterling ratio — return / mean of N worst distinct dd events       |
| KELLYF    | `research_kellyf`  | Kelly fraction `f* = (b·p − q) / b` for optimal leverage            |
| LJUNGB    | `research_ljungb`  | Ljung-Box Q at lag 10 — joint autocorrelation / white-noise test   |
| RUNSTEST  | `research_runstest`| Wald-Wolfowitz runs test — formal randomness test on sign sequence |
| ZERORET   | `research_zeroret` | Lesmond-Ogden-Trzcinka zero-return-day fraction (liquidity proxy)  |

Each table follows the established JSON-blob-per-symbol shape:

```sql
CREATE TABLE research_<name> (
    symbol TEXT PRIMARY KEY,
    snapshot_json TEXT NOT NULL DEFAULT '{}',
    updated_at INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_research_<name>_updated ON research_<name>(updated_at);
```

Each snapshot carries a regime `label` field (4–5 buckets +
`INSUFFICIENT_DATA` sentinel). Label strings:

- **STERLING**: `VERY_POOR` (ratio<−0.5) / `POOR` (<0) /
  `NEUTRAL` (<0.5) / `GOOD` (<1.5) / `EXCELLENT` (≥1.5). Ratios
  are signed (the "worst-dd" denominator is positive magnitude; a
  negative annualized return produces a negative ratio).
- **KELLYF**: `SKIP` (f*≤0) / `MARGINAL` (<0.10) / `MODERATE`
  (<0.25) / `AGGRESSIVE` (<0.50) / `ALL_IN` (≥0.50). The
  ALL_IN / SKIP extremes also signal noise — practitioners
  typically use `half_kelly` and rarely go above 0.25.
- **LJUNGB**: `WHITE_NOISE` (p≥0.10) / `WEAK_DEP` (p≥0.05) /
  `MODERATE_DEP` (p≥0.01) / `STRONG_DEP` (p<0.01).
  `reject_white_noise = p < 0.05`.
- **RUNSTEST**: `RANDOM` (|z| does not reject at α=0.05);
  `ANTI_CLUST` (z>0 and reject — runs *more* than expected,
  i.e. alternating sign series); `SLIGHT_CLUST` (z<0, p≥0.01) /
  `MOD_CLUST` (p≥0.001) / `STRONG_CLUST` (p<0.001).
- **ZERORET**: `HIGHLY_LIQUID` (<1%) / `LIQUID` (<5%) /
  `MODERATE` (<15%) / `ILLIQUID` (<30%) / `VERY_ILLIQUID`
  (≥30%).

LJUNGB p-values are computed via a Wilson-Hilferty cube-root
approximation to χ²(h) — accurate at the label-bucket granularity
needed here, and avoids adding a gamma-function dependency.
RUNSTEST uses the Abramowitz & Stegun 7.1.26 rational approximation
to the standard normal CDF. Both approximations are standard in
applied econometrics implementations where an exact statrs/gsl
dependency is avoided.

## Consequences

### Positive

- **Drawdown-ratio family complete.** CALMAR (single worst) →
  BURKE (sum-of-squares) → STERLING (mean of N worst). Three
  canonical drawdown-adjusted return scalars with complementary
  denominators; cross-comparison exposes whether performance is
  driven by one-off tail events (CALMAR dominates STERLING) vs
  a cluster of similar-magnitude drawdowns.
- **Forward-looking sizing scalar.** KELLYF gives the first
  packet surface that is explicitly an *optimization target*
  rather than a realized-performance ratio. Useful for agents
  asked "what weight should this asset get?" as distinct from
  "how did it perform?"
- **Formal inferential battery.** LJUNGB and RUNSTEST add real
  statistical tests (p-values, reject/no-reject) on top of the
  descriptive stats (AUTOCOR, RUNLEN, VARRATIO) — agents can
  now reason from null-hypothesis rejection instead of just
  reading magnitudes.
- **Microstructure liquidity trio complete.** AMIHUD (impact) +
  ROLLSPRD (spread) + ZERORET (trade frequency) covers the three
  canonical Lesmond-Ogden-Trzcinka / Amihud / Roll liquidity
  proxies. Especially useful on thinly-traded assets where
  AMIHUD's denominator (volume) is noisy.

### Negative / Risks

- **Schema migration.** `create_research_tables_v30` is additive
  over v29, so peers on v29 who receive v30 rows via LAN sync
  will create the 5 new tables via the existing
  create-before-insert path. No back-compat break.
- **LJUNGB uses an approximation.** The Wilson-Hilferty
  cube-root transform `z = ((Q/h)^(1/3) − (1 − 2/(9h))) /
  √(2/(9h))` is accurate to ~1% in the p∈[0.001, 0.10] tail range
  used for labels; agents reading `p_value` for quantitative
  work (not just label bucketing) should understand this is not
  exact. Documented in the struct doc-comment.
- **RUNSTEST needs ≥20 signed returns** after zero-return
  filtering. A very young ticker (or one with dense zero-return
  days) may pass the 30-return gate but fail the 20-signed gate
  and get `INSUFFICIENT_DATA`.
- **ZERORET `epsilon` is fixed at 1e-6.** Assets with natural
  tick-size truncation beyond this threshold may underreport
  zero days. This is the Lesmond-Ogden-Trzcinka original
  convention; making ε a configurable later round is possible.
- **Packet weight.** Each surface adds ~300-700 bytes per symbol.
  Updated envelope numbers appear in the RESEARCH_PACKET.md header.

### Neutral

- **Label-based color scheme continues** the convention
  established in Rounds 24–28 (UP=green for "favorable" label,
  DOWN=red for "adverse", AXIS_TEXT=neutral).
- **Palette aliases** avoid prior bindings. `KELLY_FRACTION` was
  not previously owned. `RUNS` is claimed here by RUNSTEST (not
  RUNLEN which owns `RUN_LEN` / `RUN_LENGTH`). `LOT` is claimed
  by ZERORET as the academic shorthand for Lesmond-Ogden-Trzcinka.
- **All five surfaces use the same broker handler shape** that
  has been stable since Round 22: `BrokerCmd::Compute*Snapshot →
  tokio::spawn → compute → msg_tx.send(BrokerMsg::*SnapshotMsg)`.
  The receive arm in `update()` both updates UI state and upserts
  to SQLite. LAN sync fans out on the next window.

## Verification

- `cargo test -p typhoon-engine --lib core::research::` — 413
  passing (up from 398 in Round 28, +15 new: 5 roundtrip + 10
  compute tests).
- `cargo check -p typhoon-engine` — clean.
- `cargo check -p typhoon-native` — clean; no palette-alias
  collisions.
- Each new compute surface carries at least one
  deterministic-fixture compute test beyond `INSUFFICIENT_DATA`:
  STERLING uses a periodic-drops fixture to produce real drawdown
  events; KELLYF/LJUNGB/RUNSTEST share an oscillating ±0.5%
  fixture that generates both positive and negative returns;
  ZERORET uses the monotonically-rising `synthetic_ohlc_bars_150`
  fixture and asserts `HIGHLY_LIQUID` with zero zero-return days.

## Packet envelope

After Round 29, single-symbol packet target envelope is **~54-106 KB**
(up from 52-102 in Round 28). Basket (10 symbols via BASKET) is
**~530-1060 KB** (up from 510-1020). Sub-block count grows 133 → 138.

Total HP-local research snapshot count after Round 29: **97**
(92 + 5). Total cross-symbol rank snapshots unchanged.
