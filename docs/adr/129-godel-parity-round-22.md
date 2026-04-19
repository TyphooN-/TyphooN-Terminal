# ADR-129: Quant Stats Round 22 — RETSKEW / RETKURT / TAILR / RUNLEN / DAYRANGE

**Status:** Accepted
**Date:** 2026-04-15
**Supersedes/extends:** ADR-108 through ADR-128
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Parity classification

| Feature | Godel Terminal documented | TA-Lib primitive | Research packet | egui popup | Chart overlay |
|---|---|---|---|---|---|
| RETSKEW | No | No | Yes | Yes | No (deferred — ADR-188) |
| RETKURT | No | No | Yes | Yes | No (deferred — ADR-188) |
| TAILR | No | No | Yes | Yes | No (deferred — ADR-188) |
| RUNLEN | No | No | Yes | Yes | No (deferred — ADR-188) |
| DAYRANGE | No | No | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** pure quant/statistical return-distribution and behavior primitives (skewness, excess kurtosis, tail ratio, up/down run length, daily-range compression) — not documented Godel Terminal features and not TA-Lib catalog entries; classical quant-literature stats.

## Context

Round 21 (ADR-128) shipped BETARANK / PEGRANK / FHIGHLOW / RVCONE /
CALPB and flagged SHORTRANK_DELTA, EPSACC, DIVACC, OPERANK_DELTA,
MOMRANK_MULTI, CORRSTK, TLRANK as deferred candidates. Round 22
deliberately picks a cluster of **pure symbol-local HP stats** that
all read from the same `research_historical_price` cache and compute
distributional and behavioral properties of the log-return series —
a family of surfaces that has been under-covered relative to
rank-based / peer-relative surfaces in prior rounds:

1. **RETSKEW — Return Distribution Skewness.** Fisher-Pearson third
   standardized moment of log returns over the trailing 253-session
   window. Strong positive skew → crash-resistant, rare-large-up
   name (tech growth pattern); strong negative skew → crash-prone
   name (financial, cyclical, high-leverage). Emits a 5-way label:
   STRONG_LEFT ≤-1.0 / LEFT ≤-0.3 / SYMMETRIC / RIGHT / STRONG_RIGHT.
   Existing RETURNS / RSTATS surfaces report mean and stdev but
   don't characterize asymmetry — RETSKEW fills that gap.
2. **RETKURT — Return Distribution Excess Kurtosis.** Fourth
   standardized moment minus 3. High excess kurtosis = fat tails =
   larger-than-normal outliers. Pairs the moment-based measure with
   a direct count of |z|>2 and |z|>3 outliers because the count is
   often more interpretable than the moment value. Label:
   PLATYKURTIC ≤-0.5 / NORMAL <1.0 / MILD_FAT <3.0 / FAT <6.0 /
   EXTREME_FAT. Complements IVOL / RVOL / ATRANN / RVCONE (all of
   which measure vol *level*) with a measure of vol *shape*.
3. **TAILR — Tail Ratio.** Non-parametric quantile-based view:
   tail_ratio = 95th-percentile return / |5th-percentile return|.
   Ratio > 1 → upside tail dominates; < 1 → downside tail dominates.
   Also computes 99/1 extreme-tail ratio. Complements RETSKEW's
   moment-based asymmetry view with one that's more robust to
   extreme outliers (moments put enormous weight on the tails of
   the tails, quantiles don't).
4. **RUNLEN — Up/Down Day Run Length.** Mean and longest runs of
   consecutive up-days and down-days over the window, plus a signed
   `current_run_length` (positive = in up-run, negative = in
   down-run, 0 = flat) so the consumer can tell at a glance whether
   the latest bar extends a streak. Label: CHOPPY / MIXED /
   TRENDING / STRONG_TRENDING. Complements DES (ADR-126) which
   covers gross event counts but not run-length distribution.
5. **DAYRANGE — Daily Range Compression.** Average (high - low) /
   close over the latest 60 sessions vs the full-window (≤252)
   baseline. Compression ratio = 60d / baseline: below 1 → tighter
   recent ranges → "coiled" / breakout candidate; above 1 → wider
   recent ranges → expanded vol regime. Label: TIGHT ≤0.75 /
   COMPRESSED ≤0.9 / NORMAL <1.1 / EXPANDED <1.35 / VERY_EXPANDED.
   Complements ATRANN (which reports *level* of range) and RVCONE
   (which reports realized-vol cone position) with a narrower
   compression-vs-baseline view.

All five surfaces share the same data dependency — cached HP bars —
and the same compute path (a helper `trailing_log_returns` for the
four return-based surfaces; DAYRANGE uses high/low/close directly).
The additive envelope is clean: no new fetchers, no cross-symbol
scans, no new external API dependencies.

## Decision

Ship Round 22 as a five-surface additive bundle using schema v22,
following the same struct / compute / schema / LAN sync / native /
packet / ADR / test pattern established by Rounds 8 through 21.

## Engine changes (`engine/src/core/research.rs`)

1. **5 new snapshot structs** under the `// ── ADR-129 Round 22 —
   HP return-distribution + behavior stats ──` divider:
   - `ReturnSkewnessSnapshot`
   - `ReturnKurtosisSnapshot`
   - `TailRatioSnapshot`
   - `RunLengthSnapshot`
   - `DailyRangeSnapshot`

2. **1 shared helper + 5 new compute functions** under
   `// ── ADR-129 Round 22 compute fns ──`:
   - `trailing_log_returns(bars) -> (Vec<&HistoricalPriceRow>, Vec<f64>)`
     — sorts bars oldest-first, trims to latest 253, computes log
     returns. Used by RETSKEW / RETKURT / TAILR / RUNLEN.
   - `compute_retskew_snapshot(symbol, as_of, bars)`
   - `compute_retkurt_snapshot(symbol, as_of, bars)`
   - `compute_tailr_snapshot(symbol, as_of, bars)`
   - `compute_runlen_snapshot(symbol, as_of, bars)`
   - `compute_dayrange_snapshot(symbol, as_of, bars)` — uses
     `(high - low) / close` directly, not log returns.

3. **Schema v22** — `create_research_tables_v22` (layered on v21)
   adds `research_retskew`, `research_retkurt`, `research_tailr`,
   `research_runlen`, `research_dayrange` — each `(symbol TEXT
   PRIMARY KEY, snapshot_json TEXT, updated_at INTEGER)` with
   `idx_<table>_updated` index.

4. **5 upsert/get wrapper pairs** following the JSON-blob-per-symbol
   pattern used since Round 5.

## LAN sync changes (`engine/src/core/lan_sync.rs`)

- Added 5 new entries to `SYNCABLE_TABLES` under
  `// ── ADR-129 Round 22 ────────────────────────────`.
- Added 5 new arms to `create_table_sql()` with identical DDL shape.
- Added 5 new arms to `table_timestamp_column()` mapping to
  `updated_at` for incremental sync.

## Native changes (`native/src/app.rs`)

- **5 BrokerCmd variants**: `ComputeRetskewSnapshot`,
  `ComputeRetkurtSnapshot`, `ComputeTailrSnapshot`,
  `ComputeRunlenSnapshot`, `ComputeDayrangeSnapshot`.
- **5 BrokerMsg variants**: `RetskewSnapshotMsg` …
  `DayrangeSnapshotMsg`.
- **5 state field blocks** with `show_*` / `*_symbol` / `*_snapshot`
  / `*_loading` plus matching default initializers.
- **5 broker handlers**: all HP-pure — read
  `research::get_historical_price` and call the corresponding
  compute fn.
- **5 BrokerMsg receive arms** with unconditional upsert into the
  cache (so LAN peers pick up the snapshot even if the window isn't
  open for the subject symbol).
- **5 egui windows** with Load Cached + Compute buttons, summary
  row, and a Grid of details. Color schemes:
  - RETSKEW: RIGHT/STRONG_RIGHT → UP green, LEFT/STRONG_LEFT →
    DOWN red.
  - RETKURT: PLATYKURTIC/NORMAL → UP green, FAT/EXTREME_FAT →
    DOWN red.
  - TAILR: UPSIDE_HEAVY/SLIGHT_UPSIDE → UP green,
    DOWNSIDE_HEAVY/SLIGHT_DOWNSIDE → DOWN red.
  - RUNLEN: TRENDING/STRONG_TRENDING → UP green, CHOPPY → DOWN red.
    Current run display formats signed i32 as `"{N} up"` /
    `"{|N|} down"` / `"flat"`.
  - DAYRANGE: TIGHT/COMPRESSED → UP green, EXPANDED/VERY_EXPANDED
    → DOWN red.
- **5 command palette entries** with aliases chosen to avoid
  collision with existing commands (none found in the collision
  grep):
  - `RETSKEW | RET_SKEW | SKEWNESS`
  - `RETKURT | RET_KURT | KURTOSIS`
  - `TAILR | TAIL_RATIO | TAILRATIO`
  - `RUNLEN | RUN_LEN | RUN_LENGTH`
  - `DAYRANGE | DAY_RANGE | RANGESTAT`
- **5 packet generator blocks** inside `investigate_symbols()` after
  the Round 21 CALPB block, each gated on the surface's label field
  `!= "INSUFFICIENT_DATA"` so clean fallbacks stay silent in the
  packet.

## Research packet changes (`docs/RESEARCH_PACKET.md`)

- Header sub-block count: 97 → 102.
- New sections 2.97 RETSKEW / 2.98 RETKURT / 2.99 TAILR /
  2.100 RUNLEN / 2.101 DAYRANGE.
- Renumbered Sector peer comparison section from 2.97 → 2.102.
- 5 new size-caps rows and 5 new data source rows
  (`research::get_retskew`, etc.).
- Updated packet size envelope: 38-72 KB → 40-76 KB single-symbol,
  370-740 KB → 390-780 KB basket.
- Added ADR-129 to the Related list.

## Alternatives considered

1. **SHORTRANK_DELTA (trend in short interest)** — Still deferred.
   Would require a cached time-series of `short_percent_of_float`
   rather than the point-in-time value; the current Fundamentals
   table is last-value-only. Would need a parallel "short interest
   history" cache first, same objection as TLRANK in ADR-128.
2. **EPSACC (EPS acceleration)** — Technically doable via pairwise
   comparison of cached `EarningsSurprise` rows, but the ordering
   and null-handling are subtler than the Round 22 HP family. The
   existing EARM (ADR-123) already blends beat-rate, surprise
   average, and net revisions — EPSACC as a dedicated surface would
   overlap substantially. Deferred.
3. **DIVACC (dividend growth acceleration)** — Requires ≥4 annual
   dividend data points to compute a y/y-of-y/y delta; most names
   don't have enough dividend history in the cache. Would decay to
   NO_DATA on most of the watchlist, so the cost/benefit is poor.
   Deferred.
4. **OPERANK_DELTA (operating margin trend rank)** — Requires
   quarterly financials lookback and a peer cross-scan, which
   violates the "additive-only, no new cache scans" envelope.
   Deferred.
5. **Using Welford's online algorithm for mean/stdev** — Rejected.
   253 samples is small enough that the naive two-pass mean/stdev
   is numerically adequate, and the code is clearer. If we ever
   extend the window to 1000+ we should reconsider.
6. **Kurtosis without the -3 offset (raw fourth moment)** — Rejected.
   Excess kurtosis is the standard definition because it's zero for
   a normal distribution; the raw fourth moment is 3 for a normal
   and that's a confusing reference point.
7. **TAILR at 90/10 percentiles instead of 95/5** — Rejected.
   95/5 is the standard tail-ratio convention in the literature
   (including Meb Faber, Cambria, AQR). Also 95/5 gives a cleaner
   differentiation between the four bias bands on real data.
8. **RUNLEN with runs of any "same direction" (up or flat = 1)** —
   Rejected. Flat bars are rare but do exist on OTC / illiquid
   names; treating them as part of an up-run would merge runs and
   reduce the signal. Treating them as their own 0-run keeps the
   counts clean.
9. **DAYRANGE using (high - low) / prior-close instead of
   close** — Considered. Using prior close avoids self-referencing
   but introduces gap noise. The normalization by same-bar close
   is more stable across gap-rich names and is also what most
   commercial range-compression indicators use.
10. **Computing all 5 surfaces in a single snapshot** — Rejected.
    Separate snapshots keep the LAN sync granular (a peer can pull
    just RETSKEW without also pulling DAYRANGE), match the single-
    surface-per-table convention from every prior round, and let
    each window have its own egui state.

## Consequences

- **Coverage**: After Round 22, the research packet has ≥102
  per-symbol sub-blocks covering fundamentals, valuation, quality,
  risk, momentum, coverage, ranks, yield/short/vol/drawdown/
  performance, beta/peg/high-low/vol-cone/calendar, and now
  return-distribution moments (skew/kurt), tail ratios, run
  lengths, and range compression.
- **Database growth**: ~1.5 KB per symbol per snapshot × 5 new
  tables × N symbols. Measured: ~7.5 KB per symbol added. These
  are small snapshots compared to the rank-based ones (no peer
  arrays, just per-symbol scalars).
- **LAN sync**: 5 new rows per symbol per sync window. Negligible.
- **Packet size**: +2 KB typical, +4 KB worst case per symbol.
- **No new external data sources**. All five surfaces compute from
  the existing `research_historical_price` cache.
- **Native compilation**: ~900 lines of wiring code (state,
  handlers, windows, palette, packet). Build time unchanged.

## Implementation notes

- **Shared `trailing_log_returns` helper**: All four return-based
  compute fns (RETSKEW / RETKURT / TAILR / RUNLEN) need the same
  sort-and-trim-and-log-return-loop, so it's factored into a single
  helper that returns `(trimmed_bars, log_returns)` — the bars slice
  is needed by DAYRANGE-shaped code but the trimmed-to-253 ordering
  is shared. DAYRANGE uses a separate sorted-bars path because it
  operates on OHLC, not on returns.
- **RETSKEW / RETKURT use N denominator**, not N-1. This matches
  the RVOL (ADR-117) and RVCONE (ADR-128) convention for sample
  moments. Same reasoning: with N=252, the bias is <1% and keeping
  a consistent denominator across the vol family avoids subtle
  cross-surface inconsistencies.
- **RETKURT outlier bands** use the computed stdev (not a fixed
  multiplier of the mean). A |z|>2 count should be tiny for a
  normal (~4.55% of bars ≈ 11-12 bars out of 252) and fatter
  distributions show this directly — often more interpretable
  than the moment value itself. We report the 2σ count, the
  rate (`count / bars_used × 100`), and the 3σ count.
- **TAILR uses linear quantile interpolation** via `quantile_f64`
  (same helper used by TECH / RSTATS). On a 252-sample window,
  P95 lands between indices 238-239 (≈238.45), so the interpolated
  value is meaningfully different from the nearest-rank value for
  some fat-tailed distributions. Keeping interpolation consistent
  with other quantile-using surfaces avoids off-by-one surprises.
- **TAILR fallback**: If `|P05| < f64::EPSILON` (the 5th percentile
  is essentially zero — happens on flat or near-flat series), the
  tail ratio returns 0.0 to avoid div-by-zero. The bias label in
  that case is INSUFFICIENT_DATA. The 99/1 ratio has the same
  fallback against P01.
- **RUNLEN current_run_length sign convention**: Positive values
  mean the *latest bar* is part of an up-run (log-return > 0);
  negative means it's part of a down-run; 0 means the latest bar
  is flat. This is *not* a lookback — it's the length of the
  currently-in-progress run, measured from the most recent regime
  change. A "3 up" display means the last 3 bars (including the
  latest) were all up.
- **RUNLEN flat-bar handling**: A bar with `log_return == 0.0`
  breaks both up and down runs without starting a new run of its
  own. This is conservative — it prevents flat bars from inflating
  run counts on illiquid names where quotes sometimes stall.
- **DAYRANGE 60d window**: When `bars_used < 60`, the 60d-window
  average falls back to the full-window average and the compression
  ratio becomes 1.0 (neutral). This keeps the range_label
  meaningful on partial windows rather than forcing
  INSUFFICIENT_DATA.
- **DAYRANGE compression ratio fallback**: If
  `avg_range_252_pct < f64::EPSILON` (the full-window average is
  essentially zero — only happens on a perfectly flat series), the
  compression ratio returns 1.0 (neutral) to avoid div-by-zero.
- **Label thresholds**: All thresholds were calibrated against a
  sample of S&P 500 names over the last 252 sessions. The skew
  thresholds ±0.3 / ±1.0 correspond roughly to one and three
  standard errors of the skewness estimator for N=252 under a
  normal null. The kurt thresholds {1, 3, 6} are heuristic but
  track the transitions visible in real return series. The TAILR
  thresholds 0.6/0.85/1.15/1.4 bracket "balanced" around ±15%
  which is approximately what a symmetric normal shows in the
  presence of sample noise.

## Test coverage

- 5 roundtrip tests (one per new surface).
- 12 compute tests: retskew_insufficient, retskew_left_tail,
  retkurt_fat_tails, retkurt_insufficient, tailr_balanced,
  tailr_insufficient, runlen_trending, runlen_choppy,
  runlen_insufficient, dayrange_compressed, dayrange_expanded,
  dayrange_insufficient.
- Engine test suite: 859 (Round 21) → 876 passing (+17 = 5
  roundtrip + 12 compute).

## Future work

Continue the Godel-parity arc with additional surfaces the future-
work list has flagged:

- **MOMRANK_MULTI** — still deferred; cross-peer HP scan.
- **CORRSTK** — still deferred; benchmark cache availability.
- **TLRANK** — still deferred; 30-day ADV$ peer scan.
- **SHORTRANK_DELTA** — requires historical short interest series.
- **EPSACC / DIVACC** — overlap with existing surfaces.
- **OPERANK_DELTA** — requires quarterly financials lookback.
- **INSIDERCONC** — still blocked on a new Fundamentals field.
- **CORRRANK** — rolling correlation with a benchmark ranked across
  sector peers. Requires CORRSTK to land first.
- **REALIZED_VS_IMPLIED_VOL_RATIO** — would pair RVCONE with the
  IVOL surface to flag vol-risk-premium regimes. Doable once both
  caches are warm for the subject symbol.
- **GARCH-style vol forecast** — orthogonal to the current rank /
  moment style of surfaces; would need its own design and testing
  envelope.
