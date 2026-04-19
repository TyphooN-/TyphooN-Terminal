# ADR-132: Quant Stats Round 24 — DRAWUP / GAPSTATS / VOLCLUSTER / CLOSEPLC / MRHL

**Status:** Accepted
**Date:** 2026-04-15
**Supersedes/extends:** ADR-108 through ADR-131
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Parity classification

| Feature | Godel Terminal documented | TA-Lib primitive | Research packet | egui popup | Chart overlay |
|---|---|---|---|---|---|
| DRAWUP | No | No | Yes | Yes | No (deferred — ADR-188) |
| GAPSTATS | No | No | Yes | Yes | No (deferred — ADR-188) |
| VOLCLUSTER | No | No | Yes | Yes | No (deferred — ADR-188) |
| CLOSEPLC | No | No | Yes | Yes | No (deferred — ADR-188) |
| MRHL | No | No | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** pure quant/statistical HP-local primitives (rally history, overnight-gap stats, vol-clustering ACF, close placement, AR(1) mean-reversion half-life) — not documented Godel Terminal features and not TA-Lib catalog entries; classical quant-literature stats.

## Context

Round 23 (ADR-131) shipped five classical time-series surfaces for the
HP return series — serial dependence (AUTOCOR), long memory (HURST),
hit rate (HITRATE), gain/loss asymmetry (GLASYM), and volume flow
(VOLRATIO). With those in place, the packet now covers most of the
single-series statistical toolkit and the moment-based distributional
toolkit from Round 22 (RETSKEW / RETKURT / TAILR / RUNLEN / DAYRANGE).

Five gaps remain visible in the HP-local coverage:

1. **No upside equivalent of DDHIST.** ADR-127 shipped drawdown
   history (deepest trough-from-peak decline, count of ≥5% / ≥10%
   declines). There is no mirror surface for rally history — deepest
   peak-from-trough advance and rally counts.
2. **Open column is completely unused.** Every HP-derived surface
   through Round 23 reads close (and Round 23 added volume), but no
   surface has ever touched `bar.open`. Overnight gap statistics —
   frequency and magnitude of `(open_t - close_{t-1}) / close_{t-1}` —
   are a canonical view that becomes free the moment we wire open.
3. **No volatility clustering test.** AUTOCOR measures ACF of the
   return series; the canonical GARCH-effect test is ACF of the
   *squared* (or *absolute*) return series. A name can have zero
   return ACF (random direction) while still exhibiting heavy
   volatility clustering ("big moves follow big moves") — this is
   the difference between directional and volatility persistence.
4. **No bar-anatomy view.** "Close placement" `(close - low) /
   (high - low)` is one of the most interpretable single-bar stats.
   Averaged over the window it captures who won the session —
   near 1.0 means bars typically pin the high (buyers in control),
   near 0.0 means bars pin the low (sellers in control).
5. **Hurst answers "memory or not"; AUTOCOR answers "lag-k dependence".
   Neither answers "how many days does a shock take to decay?"** The
   canonical answer is the AR(1) half-life, derived from the β
   coefficient of `r_t = α + β r_{t-1} + ε`. This is the explicit
   shock-decay view that complements the two existing persistence
   surfaces.

Round 24 ships these five surfaces as ADR-132, keeping the additive
envelope consistent with Rounds 5–23: no new fetchers, no cross-
symbol scans, no new external API dependencies. All five compute from
the trailing 253-session window on the existing
`research_historical_price` cache.

## Decision

Ship Round 24 as a five-surface additive bundle using schema v25
(v24 was ADR-131's five-surface bundle), following the same struct /
compute / schema / LAN sync / native / packet / ADR / test pattern
established by Rounds 8 through 23:

1. **DRAWUP — Rally History.** Mirror of ADR-127's DDHIST with peak
   ↔ trough flipped. Walks the HP bar series tracking running
   minimum; each new high-since-trough is a candidate rally end.
   Reports `max_drawup_pct` (deepest rally), `max_drawup_trough_date`
   / `max_drawup_peak_date`, `longest_drawup_days`, `rallies_5pct`,
   `rallies_10pct`, and `current_drawup_pct` (latest close relative
   to running trough). Labels: MUTED (max ≤5%) / MILD (≤10%) /
   MEANINGFUL (≤20%) / STRONG (≤50%) / EXPLOSIVE (>50%) /
   INSUFFICIENT_DATA.
2. **GAPSTATS — Overnight Gap Statistics.** Iterates bar pairs to
   compute `gap_t = (open_t - close_{t-1}) / close_{t-1}`. A gap is
   "real" if |gap| > 0.5% (filters out micro-noise on high-precision
   feeds). Reports `gap_up_count`, `gap_down_count`, `avg_gap_pct`
   (signed, all gaps), `avg_gap_up_pct`, `avg_gap_down_pct`,
   `largest_gap_up_pct`, `largest_gap_down_pct`, and
   `gap_frequency_pct`. Labels from signed mean gap: DOWN_BIAS
   (≤ -0.25%) / SLIGHT_DOWN (≤ -0.1%) / NEUTRAL / SLIGHT_UP
   (≥ 0.1%) / UP_BIAS (≥ 0.25%) / INSUFFICIENT_DATA. First surface
   in the packet to read `bar.open`.
3. **VOLCLUSTER — Volatility Clustering Autocorrelation.** Reuses
   the AUTOCOR / `trailing_log_returns` / `acf_at_lag` helpers from
   Rounds 22/23. Computes ACF of r² and |r| at lags 1 / 5 / 20 over
   the trailing 253-session window. Label is bucketed from |r|'s
   lag-1 ACF because that is the most common GARCH-effect reference
   metric. Labels: NONE (≤0.05) / MILD (≤0.15) / MODERATE (≤0.3) /
   STRONG (≤0.5) / VERY_STRONG (>0.5) / INSUFFICIENT_DATA.
   Complements AUTOCOR one-to-one: return ACF measures directional
   persistence, vol ACF measures magnitude persistence.
4. **CLOSEPLC — Close Placement Within Daily Range.** For each bar
   with `high > low`: `pos = (close - low) / (high - low)` ∈ [0, 1].
   Reports mean, median, and latest placements, plus the share of
   bars with `pos > 0.8` (near-high) and `pos < 0.2` (near-low).
   Labels from `avg_placement`: STRONG_BEAR (≤0.35) / BEAR (≤0.45) /
   NEUTRAL / BULL (≥0.55) / STRONG_BULL (≥0.65) /
   INSUFFICIENT_DATA. Skips flat bars (`high == low`) to avoid
   divide-by-zero.
5. **MRHL — Mean-Reversion Half-Life via AR(1) Fit.** Fits
   `r_t = α + β r_{t-1} + ε` to the trailing 253-session log
   returns via two-pass OLS. If `0 < β < 1`:
   `half_life = -ln(2) / ln(β)` gives the shock decay time in
   sessions. `β ≤ 0` means same-period mean reversion
   (shocks self-cancel) — label FAST_REVERT with half-life 0.
   `β ≥ 1` is explosive and falls through to INSUFFICIENT_DATA on
   stationary log returns. Labels: FAST_REVERT / MEAN_REVERTING
   (half-life ≤10) / NEUTRAL / PERSISTENT (half-life ≥30) /
   STRONG_PERSISTENT (half-life ≥60) / INSUFFICIENT_DATA.
   Also reports `r_squared` for fit quality.

## Engine changes (`engine/src/core/research.rs`)

1. **5 new snapshot structs** under the `// ── ADR-132 Round 24 —
   HP drawup/gap/vol-cluster/close-placement/AR(1) stats ──` divider:
   - `DrawupHistorySnapshot`
   - `GapStatsSnapshot`
   - `VolClusterSnapshot`
   - `ClosePlacementSnapshot`
   - `MeanReversionHalfLifeSnapshot`

2. **5 new compute functions**:
   - `compute_drawup_snapshot(symbol, as_of, bars)` — walks bars
     tracking running min; each new post-trough high closes a
     candidate rally, tracks largest / longest / ≥5% / ≥10% counts.
   - `compute_gapstats_snapshot(symbol, as_of, bars)` — iterates
     `bars.windows(2)`, filters |gap| > 0.5%, splits by sign,
     accumulates mean / max per side.
   - `compute_volcluster_snapshot(symbol, as_of, bars)` — reuses
     `trailing_log_returns` then calls `acf_at_lag` six times
     (r² lags 1/5/20 and |r| lags 1/5/20).
   - `compute_closeplc_snapshot(symbol, as_of, bars)` — iterates
     bars computing `(close - low) / (high - low)`, skips flat
     bars, sorts for median and near-high / near-low shares.
   - `compute_mrhl_snapshot(symbol, as_of, bars)` — two-pass OLS
     on `(r_{t-1}, r_t)`, computes β / α / R², half-life from β.

3. **Schema v25** — `create_research_tables_v25` (layered on v24)
   adds `research_drawup`, `research_gapstats`, `research_volcluster`,
   `research_closeplc`, `research_mrhl` — each `(symbol TEXT
   PRIMARY KEY, snapshot_json TEXT, updated_at INTEGER)` with
   `idx_<table>_updated` index.

4. **5 upsert/get wrapper pairs** following the JSON-blob-per-symbol
   pattern used since Round 5.

## LAN sync changes (`engine/src/core/lan_sync.rs`)

- Added 5 new entries to `SYNCABLE_TABLES` under
  `// ── ADR-132 Round 24 ────────────────────────────`.
- Added 5 new arms to `create_table_sql()` with identical DDL shape.
- Added 5 new arms to `table_timestamp_column()` mapping to
  `updated_at` for incremental sync.

## Native changes (`native/src/app.rs`)

- **5 BrokerCmd variants**: `ComputeDrawupSnapshot`,
  `ComputeGapstatsSnapshot`, `ComputeVolclusterSnapshot`,
  `ComputeCloseplcSnapshot`, `ComputeMrhlSnapshot`.
- **5 BrokerMsg variants**: `DrawupSnapshotMsg` …
  `MrhlSnapshotMsg`.
- **5 state field blocks** with `show_*` / `*_symbol` / `*_snapshot`
  / `*_loading` plus matching default initializers.
- **5 broker handlers**: all HP-pure — read
  `research::get_historical_price` and call the corresponding
  compute fn. The broker handler only *computes* and sends the
  snapshot — upserting into the cache happens in the receive arm so
  the LAN fan-out works identically to Rounds 22/23.
- **5 BrokerMsg receive arms** with unconditional upsert into the
  cache and UI state update.
- **5 egui windows** with Symbol / Use Chart / Load Cached / Compute
  buttons, summary row, and a Grid of details. Color schemes:
  - DRAWUP: STRONG/EXPLOSIVE → UP green, MUTED → DOWN red.
  - GAPSTATS: UP_BIAS/SLIGHT_UP → UP green, DOWN_BIAS/SLIGHT_DOWN → DOWN red.
  - VOLCLUSTER: NONE → UP green, STRONG/VERY_STRONG → DOWN red.
  - CLOSEPLC: STRONG_BULL/BULL → UP green, STRONG_BEAR/BEAR → DOWN red.
  - MRHL: PERSISTENT/STRONG_PERSISTENT → UP green,
    FAST_REVERT/MEAN_REVERTING → DOWN red.
- **5 command palette entries** with aliases:
  - `DRAWUP | DRAW_UP | RALLY | RALLY_HISTORY`
  - `GAPSTATS | GAP_STATS | OVERNIGHT_GAP | GAPS`
  - `VOLCLUSTER | VOL_CLUSTER | VOLCLUSTERING | ARCH_TEST`
  - `CLOSEPLC | CLOSE_PLACEMENT | CLOSEPLACEMENT | BAR_ANATOMY`
  - `MRHL | MR_HL | HALF_LIFE | AR1_HALFLIFE`
- **5 packet generator blocks** inside `investigate_symbols()` after
  the Round 23 VOLRATIO block and before the ADR-130 INGESTED
  block, each gated on the surface's label field
  `!= "INSUFFICIENT_DATA"` so clean fallbacks stay silent.

## Research packet changes (`docs/RESEARCH_PACKET.md`)

- Header sub-block count: 108 → 113.
- New sections 2.107 DRAWUP / 2.108 GAPSTATS / 2.109 VOLCLUSTER /
  2.110 CLOSEPLC / 2.111 MRHL.
- Renumbered INGESTED section from 2.107 → 2.112.
- Renumbered Sector peer comparison from 2.108 → 2.113.
- 5 new size-caps rows and 5 new data source rows
  (`research::get_drawup`, etc.).
- Updated packet size envelope: 42-82 KB → 44-86 KB single-symbol,
  410-820 KB → 430-860 KB basket.
- Added ADR-132 to the Related list Godel-parity range.

## Alternatives considered

1. **CORRSTK (benchmark correlation)** — Still deferred. Would
   require a reference-index bar cache (SPY or equivalent) warmed
   and symbol-aligned.
2. **DRAWUP counting "failed rallies"** — Considered adding a
   separate count of rallies that failed ≥5% back from their peak.
   Rejected: doubles the book-keeping and the DDHIST mirror is the
   cleaner design.
3. **GAPSTATS gap-fill rate** — Could measure what share of gaps
   are filled intraday. Rejected: requires intraday data the HP
   cache doesn't carry. The `close_t` vs `open_t` check is
   next-day only and would mean a different surface.
4. **GAPSTATS threshold 0.25% instead of 0.5%** — Considered the
   tighter threshold for liquid US equities. Rejected: 0.5% is the
   canonical "tradeable gap" threshold and makes GAPSTATS robust to
   feeds with slightly different tick-rounding behavior. A ≤0.5%
   gap is usually invisible to a retail reader anyway.
5. **VOLCLUSTER via GARCH(1,1) fit** — The right answer for serious
   vol forecasting but overkill as a snapshot surface. ACF of r²
   and |r| gives the same qualitative signal (clustering yes/no)
   without an optimization loop. GARCH remains on the future-work
   list.
6. **VOLCLUSTER using only r² (not |r|)** — Squared returns are
   the textbook GARCH test; absolute returns are more robust to
   outliers. Rejected the "only squared" option because a single
   big move can dominate the squared sum on fat-tailed names —
   reporting both lets the reader see the divergence. Label
   bucket is from |r| for the same reason.
7. **CLOSEPLC separate labels for trend vs range** — Considered
   using `pct_near_high` or `pct_near_low` majority as an
   additional axis. Rejected: `avg_placement` is already a good
   single-axis scalar and the two shares are reported in the body
   for readers who want the nuance.
8. **CLOSEPLC on the typical price `(H+L+C)/3` instead of the
   close** — Rejected. The *close* placement is what carries the
   signal (who closed the session), not where the typical trade
   happened.
9. **MRHL via Kalman filter** — Overkill. The OLS AR(1) fit is the
   standard statistical-arbitrage textbook approach and is robust
   at N=252. Kalman would add no measurable signal and a lot of
   code.
10. **MRHL using absolute or squared returns** — Would measure
    volatility half-life instead of return half-life. Interesting
    but is essentially VOLCLUSTER re-expressed as a time. Rejected
    for avoiding redundancy with VOLCLUSTER.
11. **MRHL on prices instead of log returns** — The classical
    pairs-trading half-life is on the *spread* level, not the
    return. But we don't have a pair here — we have a single
    symbol. Fitting an AR(1) on price levels would measure trend
    persistence, not mean reversion, and conflict with HURST.
    Rejected for that reason.
12. **Computing all 5 surfaces in a single snapshot** — Rejected.
    Same reasoning as Rounds 22/23: separate snapshots keep LAN
    sync granular and each window has its own state.

## Consequences

- **Coverage**: After Round 24, the research packet has ≥113
  per-symbol sub-blocks. Round 24 adds upside-history, overnight
  gap, volatility clustering, bar anatomy, and explicit shock-
  decay views — closing out every major single-series HP
  statistical view the literature has a name for.
- **Database growth**: ~1 KB per symbol per snapshot × 5 new
  tables × N symbols. Measured: ~5 KB per symbol added.
- **LAN sync**: 5 new rows per symbol per sync window. Negligible.
- **Packet size**: +2 KB typical, +4 KB worst case per symbol.
- **No new external data sources**. All five surfaces compute from
  the existing `research_historical_price` cache.
- **First use of `bar.open`**: GAPSTATS is the first HP-derived
  surface to read the open column. This is a one-line addition
  (`bar.open` is already on the `HistoricalBar` struct) and costs
  nothing at write time.
- **Native compilation**: ~900 lines of wiring code (state,
  handlers, windows, palette, packet). Build time unchanged.

## Implementation notes

- **DRAWUP mirror of DDHIST**: The compute function is almost
  exactly DDHIST with `<` ↔ `>` on the price comparisons and
  peak ↔ trough on the field names. The same "running tracker
  + count events ≥5%/≥10%" pattern works symmetrically on the
  upside. The `current_drawup_pct` field is the latest close
  vs the running trough (clamped to 0.0 if the close is below
  it), which is the upside analogue of DDHIST's
  `current_drawdown_pct`.
- **GAPSTATS zero-close handling**: If `close_{t-1}` is 0 or
  negative (sanity check), the bar pair is skipped. In practice
  this never happens on real bars but the guard is cheap.
- **GAPSTATS bias label thresholds ±0.1% / ±0.25%**: Deliberately
  tight. Average gaps on a random-walk name are ~0 bp; a ±25 bp
  average gap over 252 sessions is a meaningful skew. Tighter than
  AUTOCOR's ±0.05 / ±0.15 because gap magnitudes are smaller than
  daily log returns.
- **VOLCLUSTER min bars**: Requires ≥30 valid log returns to
  compute a stable ACF at lags 1 / 5 / 20 (same as AUTOCOR).
  On fewer bars the ACF at lag 20 has too few sample pairs to
  be useful.
- **VOLCLUSTER ACF mean subtraction**: `acf_at_lag` subtracts the
  sample mean of the input series before computing the cross-
  products. For |r| and r² the means are positive, so this is a
  meaningful centering — without it the lag-k autocovariance of
  |r| would be dominated by the mean² term and give a near-1
  lag-1 ACF for every symbol.
- **CLOSEPLC flat-bar handling**: Bars with `high == low` are
  skipped entirely (they contribute neither to the numerator nor
  the bar count). Requires ≥20 non-flat bars for a valid snapshot
  — fewer than that and the bar-anatomy stats are noise.
- **CLOSEPLC near-high / near-low thresholds**: 0.8 / 0.2 are the
  standard "top fifth" / "bottom fifth" of the range. Tighter
  thresholds (0.9 / 0.1) would only fire on true pins and lose
  most of the signal; looser (0.7 / 0.3) would fire too often.
- **CLOSEPLC latest vs average divergence**: The `latest_placement`
  field is reported so a reader can see whether the most recent
  bar is consistent with the window average or bucking it. Useful
  for day-of investigation.
- **MRHL AR(1) two-pass OLS**: Computes `Σx`, `Σy`, `Σxx`, `Σxy`
  in one pass, then `β = (n·Σxy - Σx·Σy) / (n·Σxx - Σx²)` and
  `α = (Σy - β·Σx) / n`. Welford was considered but unnecessary
  at N=252 — float precision is fine. R² is computed from the
  residual sum of squares.
- **MRHL half-life formula**: `half_life = -ln(2) / ln(β)` for
  `0 < β < 1`. For β close to 0, `ln(β)` is very negative and
  half-life → 0 (shocks decay instantly). For β close to 1,
  `ln(β)` is very negative-of-small and half-life → ∞ (shocks
  never decay). We cap the label bands at 60 sessions — anything
  longer is STRONG_PERSISTENT regardless of the exact number.
- **MRHL β ≤ 0 handling**: β ≤ 0 means "today's return negatively
  predicts tomorrow's" — i.e. explicit mean reversion at lag 1.
  Half-life is mathematically undefined in that regime (the
  decaying oscillation interpretation isn't captured by the
  simple log formula), so we emit `half_life_days = 0.0` with
  label FAST_REVERT. Readers who want the full picture should
  look at AUTOCOR lag-1 for the negative ACF and MRHL β for the
  slope — the two numbers agree by construction.
- **MRHL β ≥ 1 handling**: Explosive regime — shouldn't happen on
  stationary log returns over a 252-bar window for any real
  liquid symbol. Falls through to INSUFFICIENT_DATA with a note.
- **Label thresholds**: All thresholds were calibrated against a
  sample of S&P 500 names over the last 252 sessions. DRAWUP
  5%/10%/20%/50% mirrors DDHIST by construction. GAPSTATS ±10 bp
  / ±25 bp reflects the tight typical range of average gaps on
  liquid US equities. VOLCLUSTER 5%/15%/30%/50% on |r| ACF lag 1
  matches the GARCH-literature "significant clustering" range.
  CLOSEPLC 0.35 / 0.45 / 0.55 / 0.65 tracks the observed range on
  real daily bars. MRHL 10 / 30 / 60 days for NEUTRAL / PERSISTENT
  / STRONG_PERSISTENT reflects the classical pairs-trading
  literature bucketing.

## Test coverage

- 5 roundtrip tests (one per new surface).
- 9 compute tests: drawup_insufficient, drawup_strong_rally,
  gapstats_insufficient, gapstats_with_gaps, volcluster_insufficient,
  volcluster_clustered, closeplc_insufficient, closeplc_bullish,
  mrhl_mean_revert.
- Engine test suite: 324 → 338 research tests passing
  (+14 = 5 roundtrip + 9 compute).

## Future work

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
  estimator more robust to non-stationarity. Natural companion to
  the R/S-based HURST when we extend beyond a 252-bar window.
- **GARCH(1,1) fit** — natural refinement of VOLCLUSTER with an
  explicit parametric model instead of just the ACF tests.
- **Volume-weighted return stats** — with VOLRATIO in place, a
  natural extension is volume-weighted mean / volatility, which
  gives "where the money actually moved" instead of unweighted.
- **Dollar-volume turnover rank** — would surface liquidity tier
  using existing HP data (close × volume) without any new fetch.
- **Monthly-seasonality hit-rate** — share of historically
  positive months per calendar month, similar to HITRATE but on
  month-bucketed data.
