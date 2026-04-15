# ADR-131: Godel Parity Round 23 — AUTOCOR / HURST / HITRATE / GLASYM / VOLRATIO

**Status:** Accepted
**Date:** 2026-04-15
**Supersedes/extends:** ADR-108 through ADR-130
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Context

Round 22 (ADR-129) shipped a five-surface bundle of pure symbol-local
HP distributional statistics (RETSKEW / RETKURT / TAILR / RUNLEN /
DAYRANGE) on top of the existing `research_historical_price` cache.
ADR-130 then took an orthogonal step — wiring in web-research ingest
from AI agents via a Return Path footer + a SQLite article bag. The
Godel parity arc has now saturated the moment-based and first-order
behavioral views of the HP return series, but several classical
time-series properties remain uncovered:

1. **Serial dependence / autocorrelation** — no surface measures
   whether today's return predicts tomorrow's.
2. **Long memory / persistence** — the Hurst exponent is the
   standard test for scale-invariant memory in a return series
   and is absent from the packet.
3. **Frequency-based bias** — all existing return surfaces report
   return *level* or distribution *shape*, not plain win-rate.
   "Hit rate" (fraction of positive bars) is one of the most
   interpretable stats and has been missing.
4. **Magnitude asymmetry** — RETSKEW covers the third-moment view,
   but the magnitude-ratio view (avg |up| vs avg |down|) is a
   more robust, more readable cousin that belongs alongside it.
5. **Volume flow** — every existing HP surface is price-only. HP
   bars do carry a volume field; a volume-derived accumulation/
   distribution ratio has been sitting in the cache unused.

Round 23 ships these five surfaces as ADR-131, keeping the additive
envelope consistent with Rounds 5–22: no new fetchers, no cross-
symbol scans, no new external API dependencies. All five compute
from the same trailing 253-session log-return window (with VOLRATIO
also reading the HP bar volume field).

## Decision

Ship Round 23 as a five-surface additive bundle using schema v24
(v23 was ADR-130's `research_web_articles`), following the same
struct / compute / schema / LAN sync / native / packet / ADR / test
pattern established by Rounds 8 through 22:

1. **AUTOCOR — Return Autocorrelation.** Sample ACF of log returns
   at lags 1 / 5 / 10 / 20 over the trailing 253-session window.
   Labels: MEAN_REVERTING (lag1 ≤ -0.15) / NEUTRAL / MOMENTUM
   (lag1 ≥ 0.15) / INSUFFICIENT_DATA. Short-horizon daily-return
   ACF is usually tiny (|ρ| < 0.2 on liquid US equities), so the
   ±0.05 / ±0.15 threshold ladder is intentionally tight.
2. **HURST — Hurst Exponent via R/S Analysis.** Classical
   rescaled-range persistence statistic. R/S is computed at a
   candidate scale family `[8, 12, 16, 24, 32, 48, 64, 96, 128]`
   (filtered so each scale has ≥2 non-overlapping chunks; with 252
   bars this gives scales 8–128), then H is the OLS slope of
   `log(R/S_avg)` vs `log(scale)`. H ∈ [0, 1]: H<0.5 anti-
   persistent / mean-reverting, H≈0.5 random walk, H>0.5 persistent
   / trending. Labels: STRONG_MEAN_REVERT (≤0.35) / MEAN_REVERT
   (≤0.45) / RANDOM_WALK / PERSISTENT (≥0.55) / STRONG_PERSISTENT
   (≥0.65) / INSUFFICIENT_DATA. Complements AUTOCOR: one measures
   short-lag dependence, the other measures multi-scale memory.
3. **HITRATE — Multi-Horizon Hit Rate.** Fraction of positive-
   return bars over the last 5 / 20 / 60 / 252 bars plus all-window
   up/down/flat counts. Labels: BEARISH / WEAK_BEARISH / NEUTRAL /
   WEAK_BULLISH / BULLISH / INSUFFICIENT_DATA, computed from the
   h20+h60 blend. A hit-rate surface is needed because existing
   RSTATS / RETURNS report *level* — a name can have a positive
   mean with a 40% win rate (one big up day + many small downs) or
   a 60% win rate with a slightly negative mean.
4. **GLASYM — Gain/Loss Asymmetry.** Compares the typical
   magnitude of up-days vs down-days. `magnitude_ratio` =
   `avg_up_pct / avg_down_pct`. Labels: DOWNSIDE_HEAVY (≤0.7) /
   SLIGHT_DOWNSIDE (≤0.85) / BALANCED / SLIGHT_UPSIDE (≥1.15) /
   UPSIDE_HEAVY (≥1.3) / INSUFFICIENT_DATA. Complements RETSKEW
   (moment-based third-central-moment view) with an average-
   magnitude view that is more robust to outliers and often
   easier to read on fat-tailed names.
5. **VOLRATIO — Up/Down Volume Ratio.** Ratio of average up-day
   volume to average down-day volume over the window. Ratio > 1 →
   heavier volume on up-days → accumulation; < 1 → heavier on
   down-days → distribution. Labels: DISTRIBUTION (≤0.75) /
   SLIGHT_DISTRIBUTION (≤0.9) / NEUTRAL / SLIGHT_ACCUMULATION
   (≥1.1) / ACCUMULATION (≥1.35) / INSUFFICIENT_DATA. First
   HP-derived surface to read the volume column — gracefully
   emits INSUFFICIENT_DATA when volume is all zero.

## Engine changes (`engine/src/core/research.rs`)

1. **5 new snapshot structs** under the `// ── ADR-131 Round 23 —
   HP serial/persistence/behavior stats ──` divider:
   - `AutocorrelationSnapshot`
   - `HurstSnapshot`
   - `HitRateSnapshot`
   - `GainLossAsymmetrySnapshot`
   - `VolumeRatioSnapshot`

2. **5 new compute functions + 1 helper**:
   - `acf_at_lag(returns, lag) -> f64` — shared ACF helper.
   - `compute_autocor_snapshot(symbol, as_of, bars)` — reuses
     `trailing_log_returns` helper from Round 22.
   - `compute_hurst_snapshot(symbol, as_of, bars)` — R/S at candidate
     scales `[8, 12, 16, 24, 32, 48, 64, 96, 128]` filtered to
     `s <= n / 2`; OLS slope of `log(R/S_avg)` vs `log(scale)`.
   - `compute_hitrate_snapshot(symbol, as_of, bars)` — inner helper
     `hit_over(rets, take)` computes the sliding-window hit rate;
     four calls land h5 / h20 / h60 / h252.
   - `compute_glasym_snapshot(symbol, as_of, bars)` — splits returns
     by sign into up-mag / down-mag lists, computes means / medians
     / ratio.
   - `compute_volratio_snapshot(symbol, as_of, bars)` — filters to
     `vol > 0` first (MT5 bars sometimes have zero volume); splits
     by sign of log return; computes mean / median / max per side.

3. **Schema v24** — `create_research_tables_v24` (layered on v23)
   adds `research_autocor`, `research_hurst`, `research_hitrate`,
   `research_glasym`, `research_volratio` — each `(symbol TEXT
   PRIMARY KEY, snapshot_json TEXT, updated_at INTEGER)` with
   `idx_<table>_updated` index.

4. **5 upsert/get wrapper pairs** following the JSON-blob-per-symbol
   pattern used since Round 5.

## LAN sync changes (`engine/src/core/lan_sync.rs`)

- Added 5 new entries to `SYNCABLE_TABLES` under
  `// ── ADR-131 Round 23 ────────────────────────────`.
- Added 5 new arms to `create_table_sql()` with identical DDL shape.
- Added 5 new arms to `table_timestamp_column()` mapping to
  `updated_at` for incremental sync.

## Native changes (`native/src/app.rs`)

- **5 BrokerCmd variants**: `ComputeAutocorSnapshot`,
  `ComputeHurstSnapshot`, `ComputeHitrateSnapshot`,
  `ComputeGlasymSnapshot`, `ComputeVolratioSnapshot`.
- **5 BrokerMsg variants**: `AutocorSnapshotMsg` …
  `VolratioSnapshotMsg`.
- **5 state field blocks** with `show_*` / `*_symbol` / `*_snapshot`
  / `*_loading` plus matching default initializers.
- **5 broker handlers**: all HP-pure — read
  `research::get_historical_price` and call the corresponding
  compute fn. The broker handler only *computes* and sends the
  snapshot — upserting into the cache happens in the receive arm so
  the LAN fan-out works identically to Round 22.
- **5 BrokerMsg receive arms** with unconditional upsert into the
  cache and UI state update.
- **5 egui windows** with Symbol / Use Chart / Load Cached / Compute
  buttons, summary row, and a Grid of details. Color schemes:
  - AUTOCOR: MOMENTUM → UP green, MEAN_REVERTING → DOWN red.
  - HURST: PERSISTENT/STRONG_PERSISTENT → UP green,
    MEAN_REVERT/STRONG_MEAN_REVERT → DOWN red.
  - HITRATE: BULLISH/WEAK_BULLISH → UP green,
    BEARISH/WEAK_BEARISH → DOWN red.
  - GLASYM: UPSIDE_HEAVY/SLIGHT_UPSIDE → UP green,
    DOWNSIDE_HEAVY/SLIGHT_DOWNSIDE → DOWN red.
  - VOLRATIO: ACCUMULATION/SLIGHT_ACCUMULATION → UP green,
    DISTRIBUTION/SLIGHT_DISTRIBUTION → DOWN red.
- **5 command palette entries** with aliases:
  - `AUTOCOR | AUTO_COR | ACF`
  - `HURST | HURST_EXPONENT | RESCALED_RANGE`
  - `HITRATE | HIT_RATE | WIN_RATE | WINRATE`
  - `GLASYM | GL_ASYM | GAIN_LOSS_ASYM | GAINLOSSASYM`
  - `VOLRATIO | VOL_RATIO | VOLUMERATIO | VOLUME_RATIO`
- **5 packet generator blocks** inside `investigate_symbols()` after
  the Round 22 DAYRANGE block and before the ADR-130 INGESTED
  block, each gated on the surface's label field
  `!= "INSUFFICIENT_DATA"` so clean fallbacks stay silent.

## Research packet changes (`docs/RESEARCH_PACKET.md`)

- Header sub-block count: 103 → 108.
- New sections 2.102 AUTOCOR / 2.103 HURST / 2.104 HITRATE /
  2.105 GLASYM / 2.106 VOLRATIO.
- Renumbered INGESTED section from 2.102 → 2.107.
- Renumbered Sector peer comparison from 2.103 → 2.108.
- 5 new size-caps rows and 5 new data source rows
  (`research::get_autocor`, etc.).
- Updated packet size envelope: 40-78 KB → 42-82 KB single-symbol,
  390-795 KB → 410-820 KB basket.
- Added ADR-131 to the Related list Godel-parity range.

## Alternatives considered

1. **CORRSTK (benchmark correlation)** — Still deferred. Would
   require a reference-index bar cache (SPY or equivalent) warmed
   and symbol-aligned. ADR-128's original deferral reason still
   holds: no benchmark cache infrastructure yet. Would be the
   natural next step if we stand up a benchmark cache table.
2. **REALIZED_VS_IMPLIED_VOL_RATIO** — Would pair RVCONE with the
   IVOL surface. Requires both caches warm for the subject symbol,
   and IVOL depends on optional Finnhub / Polygon option-chain
   fetches that many symbols don't have. Deferred until we have
   a "has both" gate.
3. **Welch's periodogram / spectral density** — Rejected as
   overkill for a 252-bar window. ACF at 4 fixed lags is the
   pragmatic middle ground; spectral analysis on 252 samples
   is dominated by noise at most frequencies.
4. **Hurst via DFA (detrended fluctuation analysis)** —
   Considered. DFA is more robust to non-stationarity than R/S
   but the R/S method is the canonical, name-brand version and
   produces interpretable results on 252 samples. If we ever
   extend the window to 5+ years we should add DFA as a second
   Hurst estimator and compare.
5. **HITRATE with direction-sign instead of return-sign** — A
   "hit" could be defined as `close > prior_close` rather than
   `log_return > 0`. The two are equivalent except on the first
   bar; the log-return definition is consistent with every other
   Round 22/23 return-based surface. Rejected for consistency.
6. **GLASYM using standard deviation per side** — Would compute
   `up_std / down_std` instead of `avg_up / avg_down`. Rejected:
   we already have RETSKEW + RETKURT for moment-based asymmetry
   and tail-heaviness; GLASYM's value is that it's a pure
   magnitude ratio that's independent of sample size per side.
7. **VOLRATIO with dollar volume instead of share volume** —
   Considered. Dollar volume is more comparable across price
   levels, but HP bars only carry share volume (MT5 convention).
   Converting to dollar-volume would require `vol * close` per
   bar, which is fine for the ratio but doesn't add information.
   Rejected as superfluous.
8. **VOLRATIO with `on_balance_volume` cumulative curve** — OBV is
   a popular technical indicator but is a running sum — not a
   single-value snapshot — and fits the chart overlay pattern
   better than the research-surface pattern. Rejected.
9. **Computing all 5 surfaces in a single snapshot** — Rejected.
   Same reasoning as Round 22: separate snapshots keep LAN sync
   granular and each window has its own state.
10. **R/S scales tied to N** — Considered using
    `[N/32, N/16, N/8, N/4]` instead of a fixed `[8..128]` ladder.
    Rejected: fixed scales give stable, comparable H values
    across symbols with different bar counts. The filter
    `s <= n / 2` handles short-history symbols.
11. **HITRATE with a "neutral" bucket for bars with
    |return| < some threshold** — Rejected. Flat-bar handling is
    already in RUNLEN and would make HITRATE less interpretable.
    A day is up or down; flat days are counted separately in the
    body but don't count as either hit or miss.

## Consequences

- **Coverage**: After Round 23, the research packet has ≥108
  per-symbol sub-blocks. Round 23 adds serial-dependence,
  persistence / long-memory, hit-rate, magnitude-asymmetry, and
  volume-flow views — closing out the classical HP return/volume
  time-series toolkit.
- **Database growth**: ~1 KB per symbol per snapshot × 5 new
  tables × N symbols. Measured: ~5 KB per symbol added. Smaller
  than Round 22 (which had more fields per struct).
- **LAN sync**: 5 new rows per symbol per sync window. Negligible.
- **Packet size**: +2 KB typical, +4 KB worst case per symbol.
- **No new external data sources**. All five surfaces compute from
  the existing `research_historical_price` cache.
- **Native compilation**: ~900 lines of wiring code (state,
  handlers, windows, palette, packet). Build time unchanged.

## Implementation notes

- **Shared `trailing_log_returns` helper from Round 22** is reused
  verbatim by AUTOCOR / HITRATE / GLASYM. HURST uses the same
  helper then operates on the log-returns slice in its own R/S
  loop. VOLRATIO *also* uses the helper but needs the `&bars`
  alongside `returns` so it can read the volume column — since the
  helper already returns `(trimmed_bars, returns)` this is a free
  alignment.
- **AUTOCOR denominator**: Uses the N denominator for the sample
  autocovariance (consistent with RETSKEW / RETKURT's convention
  in Round 22 and with the standard "biased but consistent"
  estimator used in most references). Bias at N=252 is negligible.
- **AUTOCOR label thresholds ±0.05 / ±0.15**: Deliberately tight.
  Daily-return autocorrelation at lag 1 is typically small
  (|ρ| < 0.2 on almost every liquid US equity). A ±0.3 ladder
  would make every name look neutral; ±0.15 captures the real
  signal range.
- **HURST R/S calculation**: For each scale `s`, we split the
  return series into `floor(n / s)` non-overlapping chunks, and
  for each chunk compute `range(cumsum_centered) / std(chunk)`;
  the scale's `R/S_avg` is the mean across chunks. H is the OLS
  slope of `log(R/S_avg)` vs `log(scale)` across all scales that
  survived the `s <= n / 2` filter. Edge cases:
  - `chunk_std < f64::EPSILON` → skip the chunk (can't divide).
  - `chunk_range == 0.0` → skip the chunk (nothing to measure).
  - Fewer than 3 valid `(x, y)` points → emit
    `hurst_exponent = 0.5, memory_label = INSUFFICIENT_DATA`.
  - OLS denominator `< f64::EPSILON` → same fallback.
- **HITRATE window cap**: `take = min(rets.len(), window_size)`
  so h252 collapses to the full window on short histories and
  h5 / h20 / h60 degrade gracefully. Also reports
  `up_days` / `down_days` / `flat_days` counted over the full
  window for context.
- **GLASYM ratio fallback**: If `avg_down_pct < f64::EPSILON`
  (essentially no down-days in the window — a one-way trend name),
  the ratio returns 0.0 and the label is INSUFFICIENT_DATA. If
  `up_days == 0`, same fallback.
- **GLASYM median computation**: Uses the standard `mut v; sort;
  v[v.len() / 2]` idiom. The lists are small (≤252) and we don't
  need the exact "average of the two middle elements" median for
  even-count lists — the single-element median is the widely-used
  convention.
- **VOLRATIO zero-volume gate**: Many MT5 symbols (especially
  spot FX and some CFDs) ship HP bars with `volume == 0.0`. Rather
  than divide-by-zero or emit nonsense, VOLRATIO filters to
  `vol > 0.0` *before* splitting by sign, and if either side ends
  up empty it emits `flow_label = INSUFFICIENT_DATA`. This also
  means a LAN peer that *does* populate volume (e.g. an equity
  broker vs a CFD broker) backfills the whole network via LAN
  sync — the equity snapshot wins on last-updated.
- **VOLRATIO max up/down volume**: Reports the single largest
  up-day and down-day volume in the window, which is often more
  informative than the average for reading "which big bar was
  that?" — a small number of high-volume bars dominate most
  equities' volume histogram.
- **Label thresholds**: All thresholds were calibrated against a
  sample of S&P 500 names over the last 252 sessions. AUTOCOR ±
  0.05 / ±0.15 reflects the tight typical range of daily-return
  ACF. HURST 0.35 / 0.45 / 0.55 / 0.65 matches the classical
  literature (Mandelbrot, Peters) but compressed slightly to
  account for finite-sample noise at N=252. HITRATE ±5% around
  50% for WEAK_* and ±10% for the strong bands tracks the
  transitions visible on real data. GLASYM ±15% / ±30% mirrors
  the TAILR ±15% band from Round 22 for consistency. VOLRATIO
  ±10% / ±35% is asymmetric because "accumulation" and
  "distribution" on real data are rarely symmetric — distribution
  is usually sharper.

## Test coverage

- 5 roundtrip tests (one per new surface).
- 9 compute tests: autocor_insufficient, autocor_mean_revert,
  hurst_insufficient, hurst_trending, hitrate_bullish,
  glasym_insufficient, glasym_upside_heavy, volratio_no_volume,
  volratio_with_volume.
- Engine test suite: 310 → 324 research tests passing
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
- **GARCH-style vol forecast** — orthogonal to the current rank /
  moment style of surfaces; would need its own design and testing
  envelope.
- **Volume-weighted return stats** — with VOLRATIO in place, a
  natural extension is volume-weighted mean / volatility, which
  gives "where the money actually moved" instead of unweighted.
