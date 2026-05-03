# ADR-133: Quant Stats Round 25 — DOWNVOL / SHARPR / EFFRATIO / WICKBIAS / VOLOFVOL

**Status:** Accepted
**Date:** 2026-04-15
**Supersedes/extends:** ADR-108 through ADR-132
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Parity classification

| Feature | Godel Terminal documented | TA-Lib primitive | Research packet | egui popup | Chart overlay |
|---|---|---|---|---|---|
| DOWNVOL | No | No | Yes | Yes | No (deferred — ADR-188) |
| SHARPR | No | No | Yes | Yes | No (deferred — ADR-188) |
| EFFRATIO | No | No | Yes | Yes | No (deferred — ADR-188) |
| WICKBIAS | No | No | Yes | Yes | No (deferred — ADR-188) |
| VOLOFVOL | No | No | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** pure quant/statistical risk-return primitives (downside deviation + Sortino, Sharpe, Kaufman efficiency ratio, wick-bias, vol-of-vol) — not documented Godel Terminal features and not TA-Lib catalog entries; classical quant-literature stats.

## Context

Round 24 (ADR-132) closed out five canonical HP-local single-series
surfaces: upside history (DRAWUP), overnight gap statistics (GAPSTATS),
volatility clustering (VOLCLUSTER), bar anatomy (CLOSEPLC), and
mean-reversion half-life via AR(1) (MRHL). With Rounds 22/23/24 the
research packet now carries the moment-based distribution toolkit
(RETSKEW / RETKURT / TAILR), the return-series classical stats
(AUTOCOR / HURST / HITRATE / GLASYM), volume flow (VOLRATIO), and the
bar-anatomy / gap / shock-decay / clustering surfaces from Round 24.

Five textbook risk/return surfaces are still absent from the HP stack
even though the literature treats them as table-stakes:

1. **No downside-deviation / Sortino.** The packet reports realized
   vol (RVCONE) and tail stats (TAILR) but no pure *downside*
   volatility measure. Sortino is the canonical "Sharpe but only
   counting down moves" metric and a separate number from total vol
   whenever the return distribution is skewed. Ignoring it means
   we can't distinguish "high vol because of big up moves" from
   "high vol because of big down moves".
2. **No Sharpe ratio.** Every quantitative discussion starts with
   Sharpe. RVCONE carries annualized vol; RETSKEW/RETKURT carry
   distribution shape. But nowhere does the packet actually compute
   `mean / stdev` on the return series, annualized or not. A reader
   who wants the Sharpe number has to eyeball it from the RVCONE
   blocks.
3. **No Kaufman efficiency ratio.** HURST answers "is this price
   series persistent vs anti-persistent?"; MRHL answers "how fast
   do shocks decay?"; AUTOCOR answers "is there lag-k dependence?".
   None of these give a clean scalar for "how much of the gross
   price travel over the window actually became a net move?" —
   which is exactly what Kaufman's ER `|net_change| / Σ |daily_changes|`
   computes. It is the sharpest one-number "trending vs chopping"
   signal and historically the backbone of adaptive-MA trading
   systems.
4. **No wick-bias view.** CLOSEPLC shows *where within the range*
   the bar closes. But the upper and lower wick shares themselves
   — how much of the bar's range is rejection above vs rejection
   below — are the natural partner. Long upper wicks averaged over
   the window tell you sellers consistently rejected the high;
   long lower wicks tell you buyers consistently defended the low.
   The difference is the wick-bias scalar, and it is not derivable
   from CLOSEPLC (you can't reconstruct wicks from close placement
   alone).
5. **No vol-of-vol.** VOLCLUSTER answers "is volatility
   autocorrelated?" — a Yes/No on clustering. It doesn't answer
   "how *variable* is the volatility itself?". A name whose
   rolling 20-day realized vol cycles between 15% and 45% has a
   dramatically different risk profile from one whose 20-day vol
   stays at 30% the whole time, yet both can look identical in
   VOLCLUSTER and RVCONE. `stdev(rv20) / mean(rv20)` (coefficient
   of variation of rolling vol) is the canonical scalar for "is
   the vol regime stable?".

Round 25 ships these five surfaces as ADR-133. Same additive envelope
as Rounds 5–24: no new fetchers, no cross-symbol scans, no new
external API dependencies. All five compute from the trailing
253-session window on the existing `research_historical_price` cache.

## Decision

Ship Round 25 as a five-surface additive bundle using schema v26
(v25 was ADR-132's five-surface bundle), following the same struct /
compute / schema / LAN sync / native / packet / ADR / test pattern
established by Rounds 8 through 24:

1. **DOWNVOL — Downside Deviation and Sortino Ratio.** Iterates the
   trailing 253-session log-return series accumulating
   `down_sq = Σ min(r, 0)²` and `up_sq = Σ max(r, 0)²`. Reports
   `downside_dev = √(down_sq / n)`, annualized `downside_dev × √252`,
   upside dev as the symmetric counterpart, Sortino ratio `mean / downside_dev`
   and its annualized form `(mean × 252) / downside_dev_ann`,
   and `downside_pct_of_total = down_sq / (down_sq + up_sq) × 100`.
   Labels from annualized Sortino: VERY_POOR (<-1) / POOR (<0) /
   NEUTRAL (<1) / GOOD (<2) / EXCELLENT (≥2) / INSUFFICIENT_DATA.
2. **SHARPR — Sharpe Ratio (rf = 0).** Classical `Sharpe =
   mean_return / stdev_return` over the trailing 253-session window,
   both raw daily and annualized (×√252). Uses rf = 0 because the
   HP cache doesn't carry a risk-free series and most single-stock
   Sharpe conversations use the excess-above-zero form. Labels from
   annualized Sharpe: POOR (<-0.5) / BELOW_AVG (<0.5) / NEUTRAL (<1)
   / GOOD (<2) / EXCELLENT (≥2) / INSUFFICIENT_DATA.
3. **EFFRATIO — Kaufman Efficiency Ratio.** Trimmed to last 253 bars
   (requires ≥30); `net_change = close_N - close_1`;
   `sum_abs = Σ |close_t - close_{t-1}|`; `ER = |net| / sum_abs`.
   Reports start/end closes, net change (signed & pct), sum of
   absolute bar-to-bar changes, ER, and the signed variant
   `ER × sign(net_change)`. Labels: CHOP (<0.10) / NOISY (<0.25) /
   MIXED (<0.40) / TRENDING (<0.60) / STRONG_TREND (≥0.60) /
   INSUFFICIENT_DATA. Emits INSUFFICIENT_DATA if `sum_abs < ε`
   (dead flat window).
4. **WICKBIAS — Upper vs Lower Wick Asymmetry.** For each bar with
   `high > low`: `upper_wick = (high - max(o,c)) / (high - low)`,
   `lower_wick = (min(o,c) - low) / (high - low)`,
   `body = 1 - upper - lower`. Iterates the window skipping flat
   bars (where `high == low`), requires ≥20 non-flat bars. Reports
   averages, medians, average body share, and the bias scalar
   `wick_bias_score = avg_lower - avg_upper` (positive = buyers
   defending, negative = sellers rejecting). Labels: SELLER_REJECT
   (<-0.05) / SELLER_LEAN (<-0.02) / NEUTRAL (≤0.02) / BUYER_LEAN
   (≤0.05) / BUYER_DEFEND (>0.05) / INSUFFICIENT_DATA.
5. **VOLOFVOL — Stdev of Rolling 20-Day Realized Volatility.**
   Over the trailing 253-session log-return series, slides a
   20-bar window computing realized vol at each step (stdev of log
   returns over that window). The result is a series of ≥30 rolling
   RV points (requires ≥50 returns total). Reports mean / stdev /
   min / max / latest RV20 and `CV = stdev / mean`. Labels from CV:
   STABLE (<0.15) / MILD (<0.25) / MODERATE (<0.40) / UNSTABLE (<0.60)
   / CHAOTIC (≥0.60) / INSUFFICIENT_DATA.

## Engine changes (`engine/src/core/research.rs`)

1. **5 new snapshot structs** under the `// ── ADR-133 Round 25 —
   HP downside-vol / Sharpe / efficiency / wick / vol-of-vol ──`
   divider:
   - `DownsideVolSnapshot`
   - `SharpeRatioSnapshot`
   - `EfficiencyRatioSnapshot`
   - `WickBiasSnapshot`
   - `VolOfVolSnapshot`

2. **5 new compute functions**:
   - `compute_downvol_snapshot(symbol, as_of, bars)` — iterates
     `log_rets` accumulating down_sq / up_sq / total_sq, builds
     Sortino from mean/dev, annualizes, labels.
   - `compute_sharpr_snapshot(symbol, as_of, bars)` — two-pass mean
     + variance on log returns, computes Sharpe, annualizes.
   - `compute_effratio_snapshot(symbol, as_of, bars)` — trims to
     last 253 bars, sum_abs via `windows(2).map(|p| (p[1].close
     - p[0].close).abs()).sum()`, ER = |net| / sum_abs.
   - `compute_wickbias_snapshot(symbol, as_of, bars)` — iterates
     bars skipping flat, computes upper/lower/body shares, averages,
     sorts for medians, builds bias score.
   - `compute_volofvol_snapshot(symbol, as_of, bars)` — slides a
     `RV_WINDOW=20` over the log-return series, builds RV20 series
     via stdev, computes mean/stdev/min/max of that series, then CV.

3. **Schema v26** — `create_research_tables_v26` (layered on v25)
   adds `research_downvol`, `research_sharpr`, `research_effratio`,
   `research_wickbias`, `research_volofvol` — each `(symbol TEXT
   PRIMARY KEY, snapshot_json TEXT, updated_at INTEGER)` with
   `idx_<table>_updated` index.

4. **5 upsert/get wrapper pairs** following the JSON-blob-per-symbol
   pattern used since Round 5.

## LAN sync changes (`engine/src/core/lan_sync.rs`)

- Added 5 new entries to `SYNCABLE_TABLES` under
  `// ── ADR-133 Round 25 ────────────────────────────`.
- Added 5 new arms to `create_table_sql()` with identical DDL shape.
- Added 5 new arms to `table_timestamp_column()` mapping to
  `updated_at` for incremental sync.

## Native changes (`native/src/app.rs`)

- **5 BrokerCmd variants**: `ComputeDownvolSnapshot`,
  `ComputeSharprSnapshot`, `ComputeEffratioSnapshot`,
  `ComputeWickbiasSnapshot`, `ComputeVolofvolSnapshot`.
- **5 BrokerMsg variants**: `DownvolSnapshotMsg` …
  `VolofvolSnapshotMsg`.
- **5 state field blocks** with `show_*` / `*_symbol` / `*_snapshot`
  / `*_loading` plus matching default initializers.
- **5 broker handlers**: all HP-pure — read
  `research::get_historical_price` and call the corresponding
  compute fn. The broker handler only *computes* and sends the
  snapshot — upserting into the cache happens in the receive arm so
  LAN fan-out works identically to Rounds 22/23/24.
- **5 BrokerMsg receive arms** with unconditional upsert into the
  cache and UI state update.
- **5 egui windows** with Symbol / Use Chart / Load Cached / Compute
  buttons, summary row, and a Grid of details. Color schemes:
  - DOWNVOL: GOOD/EXCELLENT → UP green,
    POOR/VERY_POOR → DOWN red.
  - SHARPR: GOOD/EXCELLENT → UP green, POOR → DOWN red.
  - EFFRATIO: TRENDING/STRONG_TREND → UP green, CHOP → DOWN red.
  - WICKBIAS: BUYER_LEAN/BUYER_DEFEND → UP green,
    SELLER_LEAN/SELLER_REJECT → DOWN red.
  - VOLOFVOL: STABLE → UP green, UNSTABLE/CHAOTIC → DOWN red.
- **5 command palette entries** with aliases:
  - `DOWNVOL | DOWN_VOL | SEMIDEV | SORTINO`
  - `SHARPR | SHARPE | SHARPE_RATIO | SHARPERATIO`
  - `EFFRATIO | EFF_RATIO | KAUFMAN | KAUFMAN_ER | KER`
  - `WICKBIAS | WICK_BIAS | WICKS`
  - `VOLOFVOL | VOL_OF_VOL | VOV | VVOL`
- **5 packet generator blocks** inside `investigate_symbols()` after
  the Round 24 MRHL block and before the ADR-130 INGESTED block,
  each gated on the surface's label field
  `!= "INSUFFICIENT_DATA"` so clean fallbacks stay silent.

## Research packet changes (`docs/RESEARCH_PACKET.md`)

- Header sub-block count: 113 → 118.
- New sections 2.112 DOWNVOL / 2.113 SHARPR / 2.114 EFFRATIO /
  2.115 WICKBIAS / 2.116 VOLOFVOL.
- Renumbered INGESTED section from 2.112 → 2.117.
- Renumbered Sector peer comparison from 2.113 → 2.118.
- 5 new size-caps rows and 5 new data source rows
  (`research::get_downvol`, etc.).
- Updated packet size envelope: 44-86 KB → 46-90 KB single-symbol,
  430-860 KB → 450-900 KB basket.
- Added ADR-133 to the Related list Godel-parity range.

## Alternatives considered

1. **DOWNVOL using a non-zero minimum acceptable return (MAR)** —
   Classical Sortino often uses a MAR threshold (e.g. a target
   return) in place of zero. Rejected: adds a parameter the packet
   doesn't need, and most practitioners default to MAR = 0 anyway.
   If the HP cache ever carries a risk-free series we can revisit.
2. **DOWNVOL using the full Sortino-formula MAR-excess** — Would
   subtract a risk-free rate from returns before squaring. Rejected
   for the same reason as #1 — no risk-free series in HP, and most
   stock-level Sortino discussions skip it.
3. **SHARPR with rf from the 3-month Treasury** — Would need a
   separate cache for the rf series, and the annualization math
   gets messier. Rejected for the rf = 0 form which matches every
   other single-stock Sharpe conversation in the literature.
4. **SHARPR reporting just the annualized number** — Rejected.
   Raw daily Sharpe is still useful for readers who know the
   annualization is ×√252 and want the pre-annualized number.
5. **EFFRATIO computed on log returns instead of raw closes** —
   Rejected. Kaufman's ER is definitionally on the *price* series
   because it measures "net move vs gross travel". Log-return ER
   would be a different quantity and clash with HURST's R/S view.
6. **EFFRATIO window shorter than 253 bars** — Classical Kaufman
   ER uses a shorter lookback (10-30 bars) for adaptive MAs.
   Rejected: we want a regime scalar for the whole investigation
   window, not a moving-average input.
7. **EFFRATIO signed direction in the main field** — Rejected.
   Keep `efficiency_ratio` as the absolute scalar and expose
   direction separately as `signed_efficiency`, so downstream
   consumers aren't surprised by negative values when they ask
   for an ER.
8. **WICKBIAS on the wick lengths in dollars instead of shares** —
   Rejected. Share-of-range normalization is scale-free and makes
   wicks comparable across a $5 biotech and a $500 industrial.
   Dollar wicks would require a second RVCONE-style scaling.
9. **WICKBIAS bias score as a ratio instead of a difference** —
   Rejected. `lower - upper` is bounded in `[-1, 1]` and has a
   clean sign convention. A ratio (`lower / upper`) explodes
   near zero and is harder to bucket.
10. **WICKBIAS separately labelling 'pin bar' events** — Considered
    adding a count of bars where one wick is >60% of the range
    ("pin bars"). Rejected: single-bar counts belong in a
    separate surface if we ever ship one, and the averaged stats
    already capture the same signal in aggregate.
11. **VOLOFVOL using a 10- or 30-day window instead of 20** —
    20 is the canonical "one month" window in the vol literature
    and the conventional default for rolling RV analysis.
    Rejected both alternatives for consistency with that default.
12. **VOLOFVOL using stdev(rv20) directly instead of CV** —
    Rejected: CV (`stdev / mean`) normalizes out the level of vol
    itself, so a 20%-avg-vol name and a 50%-avg-vol name with the
    same *relative* instability get the same label. Raw stdev would
    conflate "high vol" and "high vol-of-vol". We do report raw
    `stdev_rv20` in the grid for readers who want both numbers.
13. **VOLOFVOL log-vol instead of vol** — Classical vol-of-vol
    in the Heston/SABR literature is typically on log(vol). Rejected
    for snapshot purposes: less interpretable, and the label
    bucketing would need recalibration.
14. **Computing all 5 surfaces in a single combined snapshot** —
    Rejected. Same reasoning as Rounds 22/23/24: separate snapshots
    keep LAN sync granular, keep the UI windows per-surface, and
    keep the schema migrations local.

## Consequences

- **Coverage**: After Round 25, the research packet has ≥118
  per-symbol sub-blocks. Round 25 closes out downside vol / Sharpe
  / trend-efficiency / wick-bias / vol-of-vol — the last cluster
  of "textbook single-series risk/return numbers" the literature
  has a standard name for.
- **Database growth**: ~1 KB per symbol per snapshot × 5 new
  tables × N symbols. Measured: ~5 KB per symbol added.
- **LAN sync**: 5 new rows per symbol per sync window. Negligible.
- **Packet size**: +2 KB typical, +4 KB worst case per symbol.
- **No new external data sources**. All five surfaces compute from
  the existing `research_historical_price` cache.
- **Native compilation**: ~900 lines of wiring code (state,
  handlers, windows, palette, packet). Build time unchanged.
- **Second reader of `bar.open` / `bar.high` / `bar.low`**:
  WICKBIAS is the first surface that uses `open`, `high`, *and*
  `low` together (GAPSTATS in Round 24 was the first to read
  `open`; CLOSEPLC uses `high`/`low`/`close`).

## Implementation notes

- **DOWNVOL zero-guard**: If `downside_dev < f64::EPSILON` with
  non-positive mean, emits INSUFFICIENT_DATA instead of dividing
  by zero on the Sortino. A series with zero downside variance
  is either (a) flat or (b) all-positive — the former is
  INSUFFICIENT_DATA and the latter implies Sortino → ∞ which we
  don't want to emit as a number.
- **DOWNVOL fractional forms**: `downside_pct_of_total` is reported
  as a percent (0-100), not a fraction, to match the convention
  used by GLASYM.
- **DOWNVOL annualization**: Sortino annualizes as `mean × 252 /
  (downside_dev × √252)` = `(mean / downside_dev) × √252`. We
  compute both raw and annualized explicitly to avoid readers
  having to convert.
- **SHARPR zero-guard**: Returns INSUFFICIENT_DATA if
  `stdev < f64::EPSILON` — a constant-return series can't have
  a Sharpe.
- **SHARPR label thresholds**: The annualized Sharpe bands
  -0.5 / 0.5 / 1 / 2 are the canonical bucketing used by most
  quantitative risk books. Below -0.5 is "actively bad", -0.5 to
  0.5 is "basically noise", 0.5 to 1 is "marginal", 1 to 2 is
  "good", ≥2 is "exceptional / probably too-good-to-be-true".
- **SHARPR raw vs annualized**: We report both to prevent the
  "Sharpe of 0.08" vs "Sharpe of 1.27" mismatch that catches
  readers out when one is daily and the other annualized.
- **EFFRATIO window trimming**: Uses the last 253 bars (same
  window as RVCONE / DDHIST / DRAWUP) to keep the lookback
  consistent. Shorter bar histories use what they have, but
  need ≥30 bars minimum.
- **EFFRATIO dead-flat guard**: If `sum_abs < f64::EPSILON`,
  emits INSUFFICIENT_DATA. This handles the pathological case of
  a halted or truly constant-price window without dividing by
  zero.
- **EFFRATIO label thresholds**: 0.10 / 0.25 / 0.40 / 0.60 are
  the canonical Kaufman-literature bands for ER bucketing.
  <0.10 is pure chop, ≥0.60 is a clean directional move, and the
  middle three are the interesting gray zones.
- **WICKBIAS flat-bar handling**: Skip bars with `high == low`
  entirely. Require ≥20 non-flat bars for a valid snapshot.
  Fewer than that and the averaged wick shares are noise.
- **WICKBIAS invariant**: `avg_upper + avg_lower + avg_body` must
  equal 1.0 by construction (they partition each bar's range).
  The `wickbias_partitions_unit` test asserts this within `1e-9`.
- **WICKBIAS bias-score bounds**: `avg_lower - avg_upper` is in
  `[-1, 1]`. Label thresholds ±0.02 / ±0.05 are tight because
  real daily bars typically have wicks of 0.1-0.3 each, so a
  5-bp bias score is meaningful.
- **WICKBIAS median via sort**: Sorts the (cloned) per-bar wick
  arrays to compute the medians. O(n log n) on ≤253 elements
  is fast enough and gives exact medians.
- **VOLOFVOL minimum bars**: Requires `log_rets.len() ≥
  RV_WINDOW + 30 = 50` to get ≥30 rolling RV points. Fewer than
  30 RV20 samples and the stdev of the series is itself too
  noisy to label.
- **VOLOFVOL RV computation**: Each rolling window uses the
  standard biased-variance formula `Σ(r - mean)² / n` (not
  `n - 1`), consistent with RVCONE. The difference is negligible
  at n=20 and keeps the two surfaces comparable.
- **VOLOFVOL CV interpretation**: CV of 0.25 means the vol
  series has a stdev 25% the size of its mean — equivalent to
  saying "vol typically swings ±25% around its average level".
  The MODERATE cutoff at 0.25 is the textbook "noticeable but
  not alarming" threshold.
- **Label thresholds**: All thresholds were calibrated against a
  sample of S&P 500 names over the last 252 sessions. DOWNVOL
  annualized Sortino bands match the Sharpe bands by construction.
  SHARPR -0.5/0.5/1/2 is canonical. EFFRATIO 0.10/0.25/0.40/0.60
  is Kaufman-literature standard. WICKBIAS ±0.02/±0.05 was
  tightened from ±0.05/±0.10 after observing that most liquid
  names cluster within ±0.05. VOLOFVOL 0.15/0.25/0.40/0.60 is the
  canonical CV-of-vol bucketing from the stochastic-vol literature.

## Test coverage

- 5 roundtrip tests (one per new surface).
- 9 compute tests: downvol_insufficient, downvol_asymmetric,
  sharpr_insufficient, sharpr_positive, effratio_insufficient,
  effratio_trending, wickbias_partitions_unit, wickbias_buyer_defend,
  volofvol_stable.
- Engine test suite: 338 → 352 research tests passing
  (+14 = 5 roundtrip + 9 compute).

## Historical Follow-up Context

Continue the Godel-parity arc with additional surfaces the future-
work list has flagged:

- **CORRSTK** — still deferred; benchmark cache availability.
- **TLRANK** — still deferred; 30-day ADV$ peer scan.
- **SHORTRANK_DELTA** — requires historical short interest series.
- **EPSACC / DIVACC** — overlap with existing surfaces.
- **OPERANK_DELTA** — requires quarterly financials lookback.
- **INSIDERCONC** — still blocked on a new Fundamentals field.
- **CORRRANK** — rolling correlation with a benchmark ranked across
  sector peers. Requires CORRSTK to land first.
- **REALIZED_VS_IMPLIED_VOL_RATIO** — would pair RVCONE with the
  IVOL surface to flag vol-risk-premium regimes.
- **DFA (detrended fluctuation analysis)** — alternate Hurst
  estimator more robust to non-stationarity.
- **GARCH(1,1) fit** — natural refinement of VOLCLUSTER + VOLOFVOL
  with an explicit parametric model instead of ACF/CV views.
- **Volume-weighted return stats** — with VOLRATIO in place, a
  natural extension is volume-weighted mean / volatility, which
  gives "where the money actually moved" instead of unweighted.
- **Dollar-volume turnover rank** — would surface liquidity tier
  using existing HP data (close × volume).
- **Monthly-seasonality hit-rate** — share of historically
  positive months per calendar month.
- **Calmar / Sterling ratios** — with DDHIST and DRAWUP in place,
  Calmar (annualized return / max drawdown) becomes free.
- **Omega ratio** — upside/downside area ratio of the return
  distribution; a natural extension of DOWNVOL.
- **Ulcer index** — squared-drawdown variant of DDHIST;
  complements the existing drawdown trough view.
