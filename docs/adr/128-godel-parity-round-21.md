# ADR-128: Godel Parity Round 21 — BETARANK / PEGRANK / FHIGHLOW / RVCONE / CALPB

**Status:** Accepted
**Date:** 2026-04-15
**Supersedes/extends:** ADR-108 through ADR-127
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Parity classification

| Feature | Godel Terminal documented | TA-Lib primitive | Research packet | egui popup | Chart overlay |
|---|---|---|---|---|---|
| BETARANK | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| PEGRANK | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| FHIGHLOW | Canonical (all terminals) | No | Yes | Yes | No (deferred — ADR-188) |
| RVCONE | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| CALPB | Yes | No | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** Godel-Terminal-documented rank + price-history surfaces (beta rank, PEG rank, realized-vol cone, calendar-period breakdowns); FHIGHLOW is a canonical 52-week high/low distance measure common across all terminals. No TA-Lib primitives in this round.

## Context

Round 20 (ADR-127) shipped DVDYIELDRANK / SHRANK / ATRANN / DDHIST /
PRICEPERF and its future-work list flagged BETA, CALPB, MOMRANK_MULTI,
REALIZED_VOL_CONE, CORRSTK, TLRANK. Round 21 picks the five that land
cleanly against the existing caches — Fundamentals-based peer ranks and
HP-pure symbol-local time-series stats — plus a new 52-week high/low
distance surface that was missing despite how fundamental the measure
is to equity research:

1. **BETARANK — Beta Rank vs Sector Peers.** Direct companion to
   Round 20's SHRANK. Risk-inverted percentile rank of
   `Fundamentals.beta` within the same sector. *Lower* beta earns a
   *higher* (safer) rank. Uses `risk_rank_label_for_percentile` →
   SAFEST_DECILE / SAFEST_QUARTILE / ABOVE_MEDIAN_SAFE /
   BELOW_MEDIAN_RISKY / BOTTOM_QUARTILE_RISKY / RISKIEST_DECILE.
   Needs ≥3 sector peers with a non-None beta. (The beta field on
   Fundamentals was already populated by existing fetchers — Round 20
   had deferred BETARANK because it wasn't obvious whether sector-
   relative beta was informative; it is, because most sectors cluster
   tightly around characteristic beta levels and the relative
   positioning matters more than the absolute value for single-name
   risk assessment.)
2. **PEGRANK — PEG Ratio Rank vs Sector Peers.** Value-inverted
   percentile rank of `Fundamentals.peg_ratio` — *lower* PEG (cheaper
   growth) earns a *higher* (better-value) rank. VAL (Round 15) fuses
   P/E, Forward P/E, P/B, P/S, EV/EBITDA, FCF yield — but *not* PEG.
   PEGRANK fills that gap as a standalone ranked surface. Uses the
   standard `rank_label_for_percentile` ladder (TOP_DECILE …
   BOTTOM_DECILE). Non-positive / non-finite PEG is filtered on both
   subject and peer sides.
3. **FHIGHLOW — 52-Week High/Low Distance.** Pure symbol-local HP
   stat over the trailing 253-session window. Tracks max/min close,
   high/low dates, days since each, percent-from-high, percent-from-low,
   and range position (0 = at low, 100 = at high). Emits a proximity
   label: AT_HIGH / NEAR_HIGH / MID_RANGE / NEAR_LOW / AT_LOW. This
   is a foundational equity research measure that was surprisingly
   absent from the parity arc — Round 20's DDHIST covers peak-to-trough
   drawdown but not the symmetric high/low distance on the current
   bar.
4. **RVCONE — Realized Volatility Cone.** Pure symbol-local HP stat
   that computes 20d / 60d / 120d / 252d annualized realized
   volatility (stdev of log returns × √252) and overlays the latest
   20d RV percentile against the rolling distribution of 20d RVs
   across the full window. Cone-position label: COMPRESSED / BELOW_AVG
   / TYPICAL / ELEVATED / EXTREME. Complements Round 20's ATRANN
   (Wilder ATR annualized) and Round 8's IVOL (implied vol) with a
   multi-horizon realized-vol view that shows where current volatility
   sits in the historical cone.
5. **CALPB — Calendar Period Breakdowns.** Pure symbol-local HP stat
   that aligns to calendar boundaries rather than rolling-session
   offsets. Computes MTD, QTD, current-year YTD, prior-quarter full
   return, and prior-year full return. PRICEPERF (Round 20) covers
   rolling 1M/3M/6M/YTD/1Y lookbacks; CALPB is complementary because
   portfolio reporting and reviews are calendar-aligned, not
   rolling-window-aligned. Emits a momentum label comparing QTD to
   prior-quarter: ACCELERATING / STEADY / DECELERATING / REVERSING.

All five surfaces are additive: the three HP-pure-compute surfaces
(FHIGHLOW / RVCONE / CALPB) need only the existing
`research_historical_price` cache; the two rank surfaces (BETARANK /
PEGRANK) read directly from the existing Fundamentals table without
cross-join scans — same as Round 20's DVDYIELDRANK/SHRANK pattern.

## Decision

Ship Round 21 as a five-surface additive bundle using schema v21,
following the same struct / compute / schema / LAN sync / native /
packet / ADR / test pattern established by Rounds 8 through 20.

## Engine changes (`engine/src/core/research.rs`)

1. **5 new snapshot structs** under the `// ── ADR-128 Round 21 —
   beta/peg rank + HP 52wk/rvcone/calendar ──` divider:
   - `BetaRankSnapshot`
   - `PegRankSnapshot`
   - `FiftyTwoWeekHighLowSnapshot`
   - `RealizedVolConeSnapshot`
   - `CalendarPeriodBreakdownSnapshot`

2. **5 new compute functions** under `// ── ADR-128 Round 21 compute
   fns ──`:
   - `compute_betarank_snapshot(symbol, as_of, sector, subject_beta, peers: &[(String, Option<f64>)])`
   - `compute_pegrank_snapshot(symbol, as_of, sector, subject_peg, peers: &[(String, Option<f64>)])`
   - `compute_fhighlow_snapshot(symbol, as_of, bars: &[HistoricalPriceRow])`
   - `compute_rvcone_snapshot(symbol, as_of, bars: &[HistoricalPriceRow])`
   - `compute_calpb_snapshot(symbol, as_of, bars: &[HistoricalPriceRow])`

3. **Schema v21** — `create_research_tables_v21` (layered on v20) adds
   `research_betarank`, `research_pegrank`, `research_fhighlow`,
   `research_rvcone`, `research_calpb` — each `(symbol TEXT PRIMARY
   KEY, snapshot_json TEXT, updated_at INTEGER)` with
   `idx_<table>_updated` index.

4. **5 upsert/get wrapper pairs** following the JSON-blob-per-symbol
   pattern used since Round 5.

## LAN sync changes (`engine/src/core/lan_sync.rs`)

- Added 5 new entries to `SYNCABLE_TABLES` under
  `// ── ADR-128 Round 21 ────────────────────────────`.
- Added 5 new arms to `create_table_sql()` with identical DDL shape.
- Added 5 new arms to `table_timestamp_column()` mapping to `updated_at`
  for incremental sync.

## Native changes (`native/src/app.rs`)

- **5 BrokerCmd variants**: `ComputeBetarankSnapshot`,
  `ComputePegrankSnapshot`, `ComputeFhighlowSnapshot`,
  `ComputeRvconeSnapshot`, `ComputeCalpbSnapshot`.
- **5 BrokerMsg variants**: `BetarankSnapshotMsg` …
  `CalpbSnapshotMsg`.
- **5 state field blocks** with `show_*` / `*_symbol` / `*_snapshot` /
  `*_loading` plus matching default initializers.
- **5 broker handlers**:
  - BETARANK / PEGRANK iterate `get_all_fundamentals` and filter by
    matching sector — same pattern as Round 20's DVDYIELDRANK/SHRANK.
  - FHIGHLOW / RVCONE / CALPB read `get_historical_price` directly.
- **5 BrokerMsg receive arms** with unconditional upsert into the
  cache (so LAN peers pick up the snapshot even if the window isn't
  open for the subject symbol).
- **5 egui windows** (BETARANK / PEGRANK / FHIGHLOW / RVCONE / CALPB)
  with Load Cached + Compute buttons, summary row, and a Grid of
  details. BETARANK uses risk-inverted color (SAFEST green, RISKIEST
  red); PEGRANK uses standard green/red; FHIGHLOW colors AT_HIGH/
  NEAR_HIGH green and AT_LOW/NEAR_LOW red; RVCONE colors COMPRESSED/
  BELOW_AVG green and ELEVATED/EXTREME red; CALPB colors ACCELERATING
  green and DECELERATING/REVERSING red.
- **5 command palette entries** with distinct aliases chosen to avoid
  collision with existing commands:
  - `BETARANK | BETA_RANK | BRK` — avoids existing `BETA` (ROLLING_BETA).
  - `PEGRANK | PEG_RANK | PEG_SCORE` — `PEG` itself is only a label,
    not a command.
  - `FHIGHLOW | FHL | 52_WEEK`
  - `RVCONE | RV_CONE | REAL_VOL_CONE` — avoids existing `RV`
    (RELATIVE_VALUATION) and `VOL_CONE` which is already an alias for
    the Round 17 RVOL command.
  - `CALPB | CAL_PB | CAL_BREAK`
- **5 packet generator blocks** inside `investigate_symbols()` after
  the Round 20 PRICEPERF block, each gated on `rank_label !=
  "INSUFFICIENT_DATA"` / `proximity_label != "INSUFFICIENT_DATA"` /
  `cone_label != "INSUFFICIENT_DATA"` / `momentum_label !=
  "INSUFFICIENT_DATA"` so clean fallbacks stay silent in the packet.

## Research packet changes (`docs/RESEARCH_PACKET.md`)

- Header sub-block count: 92 → 97.
- New sections 2.92 BETARANK / 2.93 PEGRANK / 2.94 FHIGHLOW /
  2.95 RVCONE / 2.96 CALPB.
- Renumbered Sector peer comparison section from 2.92 → 2.97.
- 5 new size-caps rows and 5 new data source rows
  (`research::get_betarank`, etc.).
- Updated packet size envelope.
- Added ADR-128 to the Related list.

## Alternatives considered

1. **MOMRANK_MULTI (sector-relative PRICEPERF)** — Deferred from
   Round 20. Would percentile-rank each PRICEPERF horizon return
   against sector peers. Rejected for Round 21 because it would
   require cross-joining with `get_historical_price` for every peer
   symbol, which is expensive per-compute and doesn't fit the
   "additive-only, no new cache scans" envelope. Deferred again.
2. **CORRSTK (rolling correlation with SPY)** — Would need SPY's HP
   cache to be populated, and then join subject + benchmark bars on
   date. Technically doable but the cache-availability dependency
   makes the compute path brittle if the user hasn't cached SPY.
   Deferred until the benchmark cache is better-guaranteed.
3. **TLRANK (30-day liquidity rank)** — Rejected for Round 21 because
   computing 30-day ADV$ per peer requires scanning HP bars for each
   peer symbol, same objection as MOMRANK_MULTI. LIQRANK (Round 18)
   uses the LIQ snapshot cache to avoid this scan; a 30-day variant
   would need a parallel "LIQ30" cache first.
4. **REALIZED_VOL_CONE with separate caches per lookback** — Rejected.
   A single RVCONE snapshot with 4 horizons + a rolling 20d
   distribution is more compact than 4 separate RV-rank surfaces
   plus a cone histogram.
5. **INSIDERCONC** — Still blocked on a missing
   `insiders_percent_held` field on Fundamentals. No change from
   Round 20.
6. **Realized correlation matrix** — Computing pairwise correlations
   across a symbol universe is quadratic and doesn't fit the
   per-symbol snapshot pattern. Not a candidate for this arc.
7. **FHIGHLOW with a "recent-high-breakout" event detector** — The
   TECH surface (Round 15) already handles breakout events. FHIGHLOW
   is deliberately a snapshot stat, not an event stream.
8. **CALPB with "since IPO" bucket** — Rejected. Calendar breakdowns
   only make sense on boundaries that repeat (month, quarter, year);
   "since IPO" is a fixed-point return already covered by other
   fields and varies wildly across symbols.

## Consequences

- **Coverage**: After Round 21, the research packet has ≥97 per-symbol
  sub-blocks covering fundamentals, valuation, quality, risk, momentum,
  coverage, ranks, yield/short/vol/drawdown/performance, and now
  beta/peg/high-low/vol-cone/calendar.
- **Database growth**: ~5 KB per symbol per snapshot × 5 new tables ×
  N symbols. Measured: ~25 KB per symbol added.
- **LAN sync**: 5 new rows per symbol per sync window. Negligible.
- **Packet size**: +2 KB typical, +4 KB worst case per symbol.
- **No new external data sources**. All five surfaces compute from data
  already present in the cache.
- **Native compilation**: ~1000 lines of wiring code (state, handlers,
  windows, palette, packet). Build time unchanged.

## Implementation notes

- **BETARANK rank floor**: Beta can legitimately be negative
  (inverse-correlation stocks — gold miners vs tech, some utilities).
  The rank comparison treats lower as safer regardless of sign, which
  means a -0.3 beta subject in a sector of +1.1 peers will land at
  SAFEST_DECILE. That's the desired behavior — a truly low-beta
  position (including negative beta) should be flagged as safer than
  a conventional high-beta peer.
- **PEGRANK non-positive filter** happens twice: once on the subject
  (return NO_DATA if the subject PEG is ≤0 — negative PEG typically
  reflects negative earnings and the ratio has no ranking meaning),
  once on each peer. The 3-peer minimum is measured *after* the peer
  filter.
- **FHIGHLOW window cutoff**: Uses the trailing 253 sessions from the
  oldest-first-sorted bar list, selecting via `rev().take(253)`. If
  the cache only has 180 bars, FHIGHLOW reports those 180 bars as a
  "180-session high/low" and bars_used reflects that — it doesn't
  pad, it doesn't error. Only <2 bars returns INSUFFICIENT_DATA.
- **FHIGHLOW range_position fallback**: If `high == low` (perfectly
  flat series), the range is 0 and range_position would divide by
  zero. We fall back to 50.0 (mid-range) for that degenerate case,
  which correctly maps to MID_RANGE.
- **RVCONE rolling distribution count**: With N log returns, we have
  N-19 rolling 20d windows (end indices 20..=N). The latest window's
  RV is excluded from the `others` slice passed to
  `percentile_rank_score` so the latest RV is ranked against its
  *history*, not against itself. If there's only 1 window total, the
  distribution is empty and we default to 50.0 percentile.
- **RVCONE uses sample-mean stdev with N denominator**, not N-1. This
  matches the RVOL surface convention (Round 17) and keeps the two
  realized-vol views consistent.
- **CALPB date parsing** assumes ISO-8601 YYYY-MM-DD — same shortcut
  as PRICEPERF's YTD and GY's year comparison. Non-ISO bars fall
  through to INSUFFICIENT_DATA via the year/month parse fallback.
- **CALPB momentum label** is intentionally crude: a 5pp gap threshold
  between QTD and prior-quarter decides ACCELERATING vs DECELERATING
  vs STEADY, with REVERSING reserved for sign flips. The thresholds
  match the user's mental model for "is this meaningfully
  different" — a 3% quarter vs a 4% quarter is not a regime change.
- **CALPB prior-quarter rollover**: When the latest bar is in Q1,
  prior-quarter is Q4 of the previous year. The year-rollover
  arithmetic is explicit (`if quarter == 1 { (year - 1, 4) } else
  { (year, quarter - 1) }`) so there's no off-by-one risk.

## Test coverage

- 5 roundtrip tests (one per new surface).
- 14 compute tests: BETARANK (safest-decile, riskiest-decile,
  insufficient), PEGRANK (top-decile, negative-peer filter,
  negative-subject NO_DATA), FHIGHLOW (at-high, at-low, insufficient),
  RVCONE (compressed, extreme, insufficient), CALPB (accelerating,
  insufficient).
- Engine test suite: 840 (Round 20) → 859 passing (+19 = 5 roundtrip
  + 14 compute).

## Future work

Continue the Godel-parity arc with additional surfaces the future-work
list has flagged:

- **MOMRANK_MULTI** — still deferred; see alternative #1.
- **CORRSTK** — still deferred; see #2.
- **TLRANK** — still deferred; see #3.
- **INSIDERCONC** — still blocked on a new Fundamentals field.
- **SHORTRANK_DELTA** — trend in short interest (cached
  `short_percent_of_float` over time) rather than a point rank.
- **EPSACC** — EPS acceleration (y/y growth rate of y/y growth).
- **DIVACC** — dividend growth acceleration.
- **OPERANK_DELTA** — operating margin trend rank, not just current.
