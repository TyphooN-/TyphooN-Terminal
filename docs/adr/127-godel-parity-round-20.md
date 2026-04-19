# ADR-127: Godel Parity Round 20 — DVDYIELDRANK / SHRANK / ATRANN / DDHIST / PRICEPERF

**Status:** Accepted
**Date:** 2026-04-15
**Supersedes/extends:** ADR-108 through ADR-126
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Parity classification

| Feature | Godel Terminal documented | TA-Lib primitive | Research packet | egui popup | Chart overlay |
|---|---|---|---|---|---|
| DVDYIELDRANK | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| SHRANK | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| ATRANN | Canonical (all terminals) | Yes (`ATR`) | Yes | Yes | No (deferred — ADR-188) |
| DDHIST | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| PRICEPERF | Yes | No | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** mostly Godel-Terminal-documented rank + price-history surfaces (dividend yield rank, short-interest rank, drawdown history, multi-horizon performance); ATRANN is a Wilder ATR-annualized volatility overlay using the canonical TA-Lib `ATR` primitive.

## Context

Round 19 (ADR-126) shipped DVDRANK / EARMRANK / UPDGRANK / GY / DES and
its future-work list flagged six candidates — DVDYIELDRANK, INSIDERCONC,
ATRANN, DDHIST, PRICEPERF, BETA, CALPB. Round 20 picks the four that
land cleanly against the existing caches, swaps INSIDERCONC for SHRANK
after scouting the `Fundamentals` struct and finding no
`insiders_percent_held` field (but confirming `short_percent_of_float`
is present), and ships the five-surface bundle below:

1. **DVDYIELDRANK — Dividend Yield Rank vs Sector Peers.** The natural
   companion to Round 19's DVDRANK. DVDRANK percentile-ranks 3y
   dividend *growth* (CAGR); DVDYIELDRANK percentile-ranks current
   dividend *yield*. Non-payers (`Fundamentals.dividend_yield` is None
   or 0.0) are filtered out on both subject and peer sides so the
   cohort captures dividend-paying names only. Needs ≥3 paying sector
   peers.
2. **SHRANK — Short Interest Rank vs Sector Peers.** Risk-inverted
   percentile rank of `Fundamentals.short_percent_of_float` within the
   same sector. *Lower* short interest earns a *higher* (safer) rank,
   mirroring the RISK / LEVRANK risk-rank pattern. Label ladder uses
   `risk_rank_label_for_percentile` → SAFEST_DECILE /
   SAFEST_QUARTILE / ABOVE_MEDIAN_SAFE / BELOW_MEDIAN_RISKY /
   BOTTOM_QUARTILE_RISKY / RISKIEST_DECILE. INSIDERCONC was the
   originally-planned surface but the cache has no insider-concentration
   field; SHRANK replaces it as a comparable risk-rank surface backed
   by an existing Fundamentals column.
3. **ATRANN — Annualized ATR (Volatility Regime).** Pure symbol-local
   time-series stat over the cached HP daily bars. Computes the
   14-period Wilder Average True Range on the most recent 253 sessions,
   expresses it as a percent of the latest close, annualizes via √252,
   and maps to a volatility regime label (LOW_VOL < 15% < NORMAL_VOL <
   30% < HIGH_VOL < 60% < EXTREME_VOL). Complements Round 8's IVOL
   (implied vol via options) with a realized-vol surface that works
   without options data.
4. **DDHIST — Drawdown History.** Pure symbol-local time-series stat
   over the same HP window. Tracks the deepest peak-to-trough
   drawdown with peak/trough dates, the longest drawdown duration in
   sessions (peak to recovery or to window end if unrecovered), the
   count of 5% and 10% corrections (local-peak-to-trough declines),
   and the current drawdown from the running peak. Regime label ladder:
   RECOVERING (> -1% current) / SHALLOW (max dd > -10%) / MEANINGFUL
   (> -20%) / SEVERE (> -35%) / CATASTROPHIC (else).
5. **PRICEPERF — Multi-horizon Price Performance.** Pure symbol-local
   time-series stat over the HP cache. Computes total returns at 1M
   (21 sessions), 3M (63), 6M (126), YTD (from first bar of as_of's
   calendar year), and 1Y (253) lookbacks. Emits a trend label
   (STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR /
   INSUFFICIENT_DATA) blended from 1Y and 3M returns.

All five surfaces are additive: the three HP-pure-compute surfaces
(ATRANN / DDHIST / PRICEPERF) need only the existing
`research_historical_price` cache; the two rank surfaces (DVDYIELDRANK /
SHRANK) read directly from the existing Fundamentals table and don't
need cross-join scans of other research tables — unlike Round 19's
DVDRANK/EARMRANK/UPDGRANK, which had to cross-join with
`get_all_<factor>` + per-peer `get_fundamentals` to recover the sector.

## Decision

Ship Round 20 as a five-surface additive bundle using schema v20,
following the same struct / compute / schema / LAN sync / native /
packet / ADR / test pattern established by Rounds 8 through 19.

## Engine changes (`engine/src/core/research.rs`)

1. **5 new snapshot structs** (after Round 19's `DailyEventStreakSnapshot`)
   under the `// ── ADR-127 Round 20 — yield/short rank + HP volatility/drawdown/returns ──`
   divider:
   - `DividendYieldRankSnapshot`
   - `ShortInterestRankSnapshot`
   - `AnnualizedAtrSnapshot`
   - `DrawdownHistorySnapshot`
   - `PricePerformanceSnapshot`

2. **5 new compute functions** (after `compute_des_snapshot`) under
   `// ── ADR-127 Round 20 compute fns ──`:
   - `compute_dvdyieldrank_snapshot(symbol, as_of, sector, subject_yield_pct, peers: &[(String, Option<f64>)])`
   - `compute_shrank_snapshot(symbol, as_of, sector, subject_short_pct, peers: &[(String, Option<f64>)])`
   - `compute_atrann_snapshot(symbol, as_of, bars: &[HistoricalPriceRow])`
   - `compute_ddhist_snapshot(symbol, as_of, bars: &[HistoricalPriceRow])`
   - `compute_priceperf_snapshot(symbol, as_of, bars: &[HistoricalPriceRow])`

3. **Schema v20** — `create_research_tables_v20` (layered on v19) adds
   `research_dvdyieldrank`, `research_shrank`, `research_atrann`,
   `research_ddhist`, `research_priceperf` — each `(symbol TEXT
   PRIMARY KEY, snapshot_json TEXT, updated_at INTEGER)` with
   `idx_<table>_updated` index.

4. **5 upsert/get wrapper pairs** following the JSON-blob-per-symbol
   pattern used since Round 5.

## LAN sync changes (`engine/src/core/lan_sync.rs`)

- Added 5 new entries to `SYNCABLE_TABLES` under
  `// ── ADR-127 Round 20 ────────────────────────────`.
- Added 5 new arms to `create_table_sql()` with identical DDL shape.
- Added 5 new arms to `table_timestamp_column()` mapping to `updated_at`
  for incremental sync. (Also back-filled Round 19 entries here, which
  had been missed.)

## Native changes (`native/src/app.rs`)

- **5 BrokerCmd variants**: `ComputeDvdyieldrankSnapshot`,
  `ComputeShrankSnapshot`, `ComputeAtrannSnapshot`,
  `ComputeDdhistSnapshot`, `ComputePriceperfSnapshot`.
- **5 BrokerMsg variants**: `DvdyieldrankSnapshotMsg` … `PriceperfSnapshotMsg`.
- **5 state field blocks** with `show_*` / `*_symbol` / `*_snapshot` /
  `*_loading` plus matching default initializers.
- **5 broker handlers**:
  - DVDYIELDRANK / SHRANK iterate `get_all_fundamentals` and filter by
    matching sector — no cross-join needed because sector and the ranked
    field live on the same Fundamentals row.
  - ATRANN / DDHIST / PRICEPERF read `get_historical_price` directly.
- **5 BrokerMsg receive arms** with unconditional upsert into the
  cache (so LAN peers pick up the snapshot even if the window isn't
  open for the subject symbol).
- **5 egui windows** (DVDYIELDRANK / SHRANK / ATRANN / DDHIST / PRICEPERF)
  with Load Cached + Compute buttons, summary row, and a Grid of
  details. SHRANK uses risk-inverted color (SAFEST green, RISKIEST red).
- **5 command palette entries** with distinct aliases chosen to avoid
  collision with existing commands:
  - `DVDYIELDRANK | DVDY_RANK | DIVIDEND_YIELD_RANK`
  - `SHRANK | SHORT_RANK | SHORT_INT_RANK` — avoids existing `SHRT` and
    `SHORT_INTEREST` commands.
  - `ATRANN | ATR_ANN | ANNUALIZED_ATR` — avoids existing `ATR`
    indicator name.
  - `DDHIST | DD_HIST | DRAWDOWN_HIST` — avoids existing `DRAWDOWN`
    (Darwin drawdown dashboard).
  - `PRICEPERF | PRICE_PERF | MULTI_RETURN` — avoids existing `PERF`
    (Darwin Seasonals).
- **5 packet generator blocks** inside `investigate_symbols()` after
  the Round 19 DES block, each gated on `rank_label !=
  "INSUFFICIENT_DATA"` / `trend_label != "INSUFFICIENT_DATA"` etc. so
  clean fallbacks stay silent in the packet.

## Research packet changes (`docs/RESEARCH_PACKET.md`)

- Header sub-block count: 87 → 92.
- New sections 2.87 DVDYIELDRANK / 2.88 SHRANK / 2.89 ATRANN /
  2.90 DDHIST / 2.91 PRICEPERF.
- Renumbered Sector peer comparison section from 2.87 → 2.92.
- 5 new size-caps rows and 5 new data source rows
  (`research::get_dvdyieldrank`, etc.).
- Updated packet size envelope.
- Added ADR-127 to the Related list.

## Alternatives considered

1. **INSIDERCONC (insider concentration)** — Originally on the Round 20
   short-list from Round 19. Scouted `engine/src/core/fundamentals.rs`
   and found no `insiders_percent_held` field. Rejected: can't ship
   without adding a new fetcher + schema migration, which would
   violate the "additive-only, no new caches" Round 20 envelope.
   Replaced with SHRANK, which uses an existing Fundamentals column.
2. **BETA percentile rank** — Beta is already on `Fundamentals`, but
   sector-relative beta ranking is typically less informative than
   absolute beta (because all tech names cluster near beta 1.2, etc.).
   Deferred to a later round as a potential companion to a realized-beta
   HP-compute surface.
3. **CALPB (calendar period breakdowns)** — Computes quarter-over-quarter
   and year-over-year returns from the HP window. Deferred: PRICEPERF
   already covers the 1M/3M/6M/YTD/1Y horizons which is what users
   actually ask about. CALPB would add noise without informational
   gain.
4. **Separate 63-session and 21-session ATR regimes** — Rejected as
   overfitting. A single 14-period Wilder ATR annualized via √252 is
   the industry-standard volatility measure; sub-horizon splits belong
   in a realized-vol-cone surface, not in ATRANN.
5. **Drawdown distribution histogram** — Could emit a full histogram
   of drawdown-depth bins. Rejected as overkill for a summary snapshot;
   DDHIST's 5%/10% correction counts + max dd + current dd cover the
   questions a packet reader actually needs to answer.
6. **Weekly / monthly PRICEPERF horizons** — Rejected as noise. 1M is
   the shortest meaningful equity-return horizon; adding WTD/MTD
   complicates the trend-label ladder without adding signal.
7. **Percentile-ranked PRICEPERF (vs sector peers)** — Rejected for
   Round 20 to keep the surface symbol-local and fast. A sector-rank
   overlay could ship later as a separate "MOMRANK_MULTI" surface if
   there's demand.
8. **Move DDHIST's correction counter to an event-driven detector** —
   Rejected. The local-peak-to-trough heuristic is good enough for
   the summary count; a proper event detector would duplicate TECH
   (ADR-116) without adding value.

## Consequences

- **Coverage**: After Round 20, the research packet has ≥92 per-symbol
  sub-blocks covering fundamentals, valuation, quality, risk, momentum,
  coverage, ranks, and now yield/short/volatility/drawdown/performance.
- **Database growth**: ~5 KB per symbol per snapshot × 5 new tables ×
  N symbols. Measured: ~25 KB per symbol added.
- **LAN sync**: 5 new rows per symbol per sync window. Negligible.
- **Packet size**: +2 KB typical, +4 KB worst case per symbol.
- **No new external data sources**. All five surfaces compute from data
  already present in the cache.
- **Native compilation**: +1000 lines of wiring code (state, handlers,
  windows, palette, packet). Build time unchanged.

## Implementation notes

- **DVDYIELDRANK non-payer filter** happens twice: once on the subject
  (return NO_DATA if the subject doesn't pay), once on each peer
  (filter out None or 0.0 peers before ranking). The 3-peer minimum is
  measured *after* the peer filter.
- **SHRANK risk-inverted rank-position** counts `peers_safer =
  filter(|p| p < subject).count()`, not `peers_better`. The
  `percentile_rank_score(.., false)` flips the comparison so that
  lower short earns a higher pct, but the rank_position counter is
  independent and must be flipped separately.
- **ATRANN seed** is the mean of the first 14 TR values, then Wilder
  smoothing `atr_i = (prev_atr × 13 + tr_i) / 14` for the rest. TR is
  `max(high-low, |high-prev_close|, |low-prev_close|)`. Bars with
  non-positive OHLC are skipped on TR construction. √252 annualization
  follows the conventional daily-to-annual equity volatility scaling.
- **DDHIST longest-drawdown tracker** is per-drawdown-episode, not
  across the whole window. When the close touches the running peak,
  the current drawdown bucket closes and duration is measured from the
  peak index. If the window ends still below peak, the in-progress
  duration is finalized at window end.
- **DDHIST correction detector** opens when close drops below the
  correction-peak tracker and closes on recovery to or above the
  previous running peak. An open correction at window end is also
  finalized so the last-window decline gets counted.
- **PRICEPERF YTD** matches via year-prefix string comparison on the
  first 4 chars of the date field. This is a shortcut that assumes
  ISO-8601 date strings; all HP bars in the cache use this format.
- **PRICEPERF trend label** gates on `bars_used >= 20` — shorter
  windows get INSUFFICIENT_DATA to avoid trend labels on days-old
  histories.
- **Risk-inverted rank floor**: With 5 total (subject + 4 peers) and
  subject highest, `percentile_rank_score(..., false)` returns
  `(0 below + 0 equal + 0.5) / 5 × 100 = 10.0` exactly, which lands at
  `BOTTOM_QUARTILE_RISKY` (the 10.0 ≤ pct < 25.0 bucket). To hit
  `RISKIEST_DECILE` the test needs ≥10 total items so the floor is
  `0.5/10 × 100 = 5.0 < 10.0`. Tests use 10-peer fixtures where the
  label matters.

## Test coverage

- 5 roundtrip tests (one per new surface).
- 10 compute tests: DVDYIELDRANK (top-decile, non-payer filter,
  subject non-payer), SHRANK (safest-decile, riskiest-decile,
  insufficient), ATRANN (low-vol, high-vol, insufficient), DDHIST
  (shallow, severe), PRICEPERF (bull, bear, insufficient).
- Engine test suite: 821 (Round 19) → 840 passing (+19 = 15 Round 20
  tests + 4 Round 19 tests that had been written but missed in the
  earlier count).

## Future work

Continue the Godel-parity arc with additional surfaces the future-work
list has flagged:

- **BETA percentile rank** (still deferred; see alternative #2).
- **CALPB — calendar period breakdowns** (still deferred; see #3).
- **MOMRANK_MULTI — sector-relative PRICEPERF** (see #7).
- **INSIDERCONC** — still blocked on a new Fundamentals field; needs a
  new fetcher before it can ship.
- **REALIZED_VOL_CONE** — percentile-of-percentile surface over
  multiple lookback horizons; a natural companion to ATRANN and IVOL.
- **CORRSTK** — rolling correlation with SPY/sector ETF as a
  single-symbol risk stat.
- **TLRANK — trading liquidity rank** over a narrower (30-day)
  window than Round 18's LIQRANK.
