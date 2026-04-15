# ADR-125: Godel Parity Round 18 — LEVRANK / OPERANK / FQMRANK / LIQRANK / SURPSTK

**Status:** Accepted
**Date:** 2026-04-15
**Supersedes/extends:** ADR-108 through ADR-124
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Context

Round 17 (ADR-124) shipped SIZEF / MOMF / PEADRANK / FQM / REVRANK,
closing the momentum-rank, PEAD-rank, and revenue-growth-rank parity
gaps and adding a three-input FQM operator composite deliberately
split from the Round 15 QUAL surface. Round 17's future-work list
called out four pure-compute candidates that don't need new caches:

1. **LEVRANK — Leverage Rank vs Sector Peers.** Sector-relative
   percentile rank of debt-to-equity. Completes the "rank overlay"
   family for the one factor dimension FQM deliberately excluded
   (leverage) so the reader can still view leverage on its own rank
   axis even though FQM doesn't fold it in.
2. **FQMRANK — FQM Rank vs Sector Peers.** The natural rank overlay
   for the Round 17 FQM composite. Depends on the new whole-table
   scan `get_all_fqm`.
3. **OPERANK — Operating Quality Rank vs Sector Peers.** Percentile
   rank of operating margin alone, isolating the "pricing power"
   signal from the fused FQM/QUAL composites. Distinct from QRK/FQM
   because it single-sources from `MarginsSnapshot.latest_operating_margin_pct`
   and ranks that one field directly.

Round 18 also adds two new surfaces that the Round 17 future-work
list didn't explicitly call out but that are natural continuations
of the "rank every cached factor" sweep:

4. **LIQRANK — Liquidity Rank vs Sector Peers.** Sector-relative
   percentile rank of `LiquiditySnapshot.avg_daily_dollar_volume`.
   Closes the parity gap where LIQ was absolute-tier-only (DEEP /
   LIQUID / THIN / ILLIQUID / INSUFFICIENT_DATA) and didn't surface
   "how does this name's tradeable depth compare to its sector
   cohort?" Higher ADV$ = deeper = higher rank.
5. **SURPSTK — Earnings Surprise Streak.** A pure time-series
   statistic over cached `EarningsSurprise` rows: classifies each
   historical event as BEAT / MISS / INLINE (±2% band around
   `surprise_pct`), counts consecutive and longest streaks, and emits
   a streak-strength label (HOT_STREAK / BEAT_TREND / MIXED /
   MISS_TREND / COLD_STREAK / INSUFFICIENT_DATA). Unlike the other
   four Round 18 surfaces, SURPSTK is **not a sector-rank** — it's
   a symbol-local history stat, so it needs no peer cross-join.

The standing directive continues: *"continue combing over vs godel
parity until we cannot add more. rinse/repeat do not worry about
round count."*

## Decision

Add five new research surfaces following the Round 15/16/17 pattern.
Round 18 introduces four new `get_all_*` whole-table scan helpers
(`get_all_leverage`, `get_all_margins`, `get_all_fqm`, `get_all_liquidity`)
so LEVRANK / OPERANK / FQMRANK / LIQRANK can read every cached row
of their matching factor tables. This extends the Round 16/17
pattern (`get_all_val` / `get_all_qual` / `get_all_risk` /
`get_all_momentum` / `get_all_pead`) to four more factor caches.

### Engine (`engine/src/core/research.rs`)

- **New structs** (after the Round 17 structs, under
  `// ── ADR-125 Round 18 ──` divider):
  - `LeverageRankSnapshot` — LEVRANK (symbol, as_of, sector,
    debt_to_equity, total_debt, total_equity, peers_considered,
    peers_with_data, sector_median_d2e, sector_p25_d2e,
    sector_p75_d2e, percentile_rank, rank_position, rank_label,
    note). Risk-inverted labels: SAFEST_DECILE (lowest D/E) ... to
    RISKIEST_DECILE (highest D/E), plus NEGATIVE_EQUITY for the
    `total_equity <= 0` edge case.
  - `OperatingQualityRankSnapshot` — OPERANK (symbol, as_of, sector,
    operating_margin_pct, margin_trend_label, peers_considered,
    peers_with_data, sector_median_margin_pct, sector_p25_margin_pct,
    sector_p75_margin_pct, percentile_rank, rank_position, rank_label,
    note). Standard TOP_DECILE...BOTTOM_DECILE labels.
  - `FqmRankSnapshot` — FQMRANK (symbol, as_of, sector,
    composite_score, operator_label, peers_considered, peers_with_data,
    sector_median_score, sector_p25, sector_p75, percentile_rank,
    rank_position, rank_label, note). The subject's operator label
    is copied verbatim from the upstream FQM row.
  - `LiquidityRankSnapshot` — LIQRANK (symbol, as_of, sector,
    avg_daily_dollar_volume, tier_label, peers_considered,
    peers_with_data, sector_median_dollar_volume,
    sector_p25_dollar_volume, sector_p75_dollar_volume,
    percentile_rank, rank_position, rank_label, note). The subject's
    absolute `liquidity_tier` is copied alongside the sector rank so
    the reader can distinguish "deep for this sector" from "deep
    absolutely."
  - `EarningsSurpriseStreakSnapshot` — SURPSTK (symbol, as_of,
    total_events, beats, misses, inlines, beat_rate_pct,
    current_streak_type, current_streak_len, longest_beat_streak,
    longest_miss_streak, avg_surprise_pct, latest_event_date,
    latest_event_surprise_pct, latest_event_label, streak_label,
    note).

- **New compute fns** (after `compute_revrank_snapshot`, under the
  same divider):
  - `compute_levrank_snapshot(symbol, as_of, sector, subject:
    Option<&LeverageSnapshot>, peers: &[&LeverageSnapshot])` — helper
    `debt_to_equity_for(lev)` returns `Some(total_debt / total_equity)`
    when `total_equity > 0`, otherwise `None`. Subject with
    `total_equity <= 0` short-circuits to `rank_label = "NEGATIVE_EQUITY"`
    (no percentile computed). Otherwise calls
    `percentile_rank_score(subject_d2e, others, higher_is_better=false)`
    so lower D/E earns a higher (safer) percentile, then maps the
    percentile to the risk-inverted ladder.
  - `compute_operank_snapshot(symbol, as_of, sector, subject:
    Option<&MarginsSnapshot>, peers: &[&MarginsSnapshot])` — ranks
    `latest_operating_margin_pct`, filters peers whose
    `periods_used > 0` (the MarginsSnapshot sentinel for "no data").
    Standard `higher_is_better=true` ranking.
  - `compute_fqmrank_snapshot(symbol, as_of, sector, subject:
    Option<&FundamentalQualityMeterSnapshot>, peers:
    &[&FundamentalQualityMeterSnapshot])` — ranks `composite_score`,
    filters peers whose `operator_label != "NO_DATA" && composite_score > 0`.
    Copies the subject's `operator_label` into the rank row so the
    packet can render both views in one line.
  - `compute_liqrank_snapshot(symbol, as_of, sector, subject:
    Option<&LiquiditySnapshot>, peers: &[&LiquiditySnapshot])` —
    ranks `avg_daily_dollar_volume`, filters peers whose
    `liquidity_tier != "INSUFFICIENT_DATA"`. Copies the subject's
    absolute tier label. `higher_is_better=true` since deeper = higher.
  - `compute_surpstk_snapshot(symbol, as_of, surprises:
    &[EarningsSurprise])` — sorts newest-first by event date,
    classifies each row via a ±2% band around `surprise_pct`
    (BEAT > +2%, MISS < -2%, INLINE in between), counts beats /
    misses / inlines / avg surprise, tracks current and longest
    streaks, and maps to the streak ladder:
    - HOT_STREAK: beat_rate ≥ 75% AND current = BEAT AND current_len ≥ 3
    - BEAT_TREND: beat_rate ≥ 60%
    - COLD_STREAK: beat_rate ≤ 25% AND current = MISS AND current_len ≥ 3
    - MISS_TREND: beat_rate ≤ 40%
    - MIXED: otherwise (when total_events ≥ 4)
    - INSUFFICIENT_DATA: total_events < 4

- **New helpers** (co-located with the compute fns):
  - `fn debt_to_equity_for(lev: &LeverageSnapshot) -> Option<f64>` —
    safe divide with the positive-equity guard.

- **Schema v18** (`create_research_tables_v18`): calls v17 first,
  then creates `research_levrank`, `research_operank`,
  `research_fqmrank`, `research_liqrank`, `research_surpstk`, each
  shaped the same way `(symbol TEXT PRIMARY KEY, snapshot_json TEXT,
  updated_at INTEGER)` with an `updated_at` index. Schema v18 is
  additive: no existing Round 1-17 tables change layout.

- **Upsert/get wrappers** (after `get_pead`):
  `upsert_levrank` / `get_levrank`, `upsert_operank` / `get_operank`,
  `upsert_fqmrank` / `get_fqmrank`, `upsert_liqrank` / `get_liqrank`,
  `upsert_surpstk` / `get_surpstk`. Standard
  `INSERT ... ON CONFLICT` + serde-JSON roundtrip.

- **New whole-table scans**:
  - `get_all_leverage(&conn) -> Result<Vec<LeverageSnapshot>>` —
    scans `research_leverage` (created by `create_research_tables_v10`,
    where leverage was first cached).
  - `get_all_margins(&conn) -> Result<Vec<MarginsSnapshot>>` — scans
    `research_margins` (created by `create_research_tables_v14`).
  - `get_all_fqm(&conn) -> Result<Vec<FundamentalQualityMeterSnapshot>>`
    — scans `research_fqm` (created by `create_research_tables_v17`).
  - `get_all_liquidity(&conn) -> Result<Vec<LiquiditySnapshot>>` —
    scans `research_liquidity` (created by `create_research_tables_v13`).
  All four follow the same shape as the Round 16/17 whole-table
  helpers: `query_map([], |row| row.get::<_, String>(0))` over the
  target table, serde-JSON deserialisation per row.

### LAN sync (`engine/src/core/lan_sync.rs`)

- `SYNCABLE_TABLES` — add `research_levrank`, `research_operank`,
  `research_fqmrank`, `research_liqrank`, `research_surpstk` under a
  new `// ── ADR-125 Round 18 ──` divider.
- `create_table_sql()` — 5 new arms emitting the same DDL as the
  engine's `create_research_tables_v18`.
- `table_timestamp_column()` — 5 new arms returning `"updated_at"`
  for each new table. (Also added the 5 Round 17 arms that were
  missing from the Round 17 commit — see "Round 17 backfill" below.)

### Native (`native/src/app.rs`)

- **BrokerCmd variants** (after `ComputeRevrankSnapshot`, under a new
  `// ── ADR-125 Round 18 ──` divider):
  - `ComputeLevrankSnapshot { symbol }`
  - `ComputeOperankSnapshot { symbol }`
  - `ComputeFqmrankSnapshot { symbol }`
  - `ComputeLiqrankSnapshot { symbol }`
  - `ComputeSurpstkSnapshot { symbol }`

- **BrokerMsg variants** (after `RevrankSnapshotMsg`) under a new
  `// ── ADR-125 ──` divider:
  - `LevrankSnapshotMsg(String, LeverageRankSnapshot)`
  - `OperankSnapshotMsg(String, OperatingQualityRankSnapshot)`
  - `FqmrankSnapshotMsg(String, FqmRankSnapshot)`
  - `LiqrankSnapshotMsg(String, LiquidityRankSnapshot)`
  - `SurpstkSnapshotMsg(String, EarningsSurpriseStreakSnapshot)`

- **TyphooNApp state fields** (after the Round 17 fields) under a new
  `// ── ADR-125 Godel Parity Round 18 ──` divider. Each surface gets
  `show_*` / `*_symbol` / `*_snapshot` / `*_loading`.

- **Broker handler spawns** (after the REVRANK handler). Each one
  follows the Round 17 `tokio::spawn` + `shared_cache_broker` pattern
  and pre-reads the caches needed on the task thread:
  - **LEVRANK handler** — cross-join pattern. Calls
    `research::get_all_leverage(&conn)` then per-peer
    `fundamentals::get_fundamentals` to filter to the subject's
    sector. LeverageSnapshot doesn't carry sector, so the cross-join
    is required.
  - **OPERANK handler** — same cross-join pattern using
    `get_all_margins` and `fundamentals::get_fundamentals`.
  - **FQMRANK handler** — same cross-join pattern using `get_all_fqm`
    and `fundamentals::get_fundamentals`.
  - **LIQRANK handler** — same cross-join pattern using
    `get_all_liquidity` and `fundamentals::get_fundamentals`.
  - **SURPSTK handler** — the odd one out: no peer iteration, no
    cross-join. Pre-reads
    `research::get_earnings_surprises(&conn, &symbol)` and feeds the
    Vec directly to `compute_surpstk_snapshot`.

- **Receive arms** (in the `BrokerMsg` match, after
  `RevrankSnapshotMsg`): each arm updates the matching state field if
  the incoming symbol matches `*_symbol`, then unconditionally
  upserts the snapshot via `upsert_*`. Unconditional upsert so
  LAN-synced receivers benefit even when no window is open.

- **egui windows** (after the REVRANK window). Each window follows
  the established header pattern (symbol editor / Use Chart / Load
  Cached / Compute / Loading spinner), a color-coded summary line,
  and per-surface grids:
  - **LEVRANK** — 3-row summary (subject D/E / sector median/p25/p75
    D/E / peers considered/with data), with a dedicated NEGATIVE_EQUITY
    branch that drops the percentile-rank view and shows total_debt /
    total_equity in $B instead.
  - **OPERANK** — 3-row summary (subject op margin % + trend / sector
    median/p25/p75 op margin % / peers considered/with data).
  - **FQMRANK** — 3-row summary (subject composite + operator label /
    sector median/p25/p75 composite / peers considered/with data).
  - **LIQRANK** — 3-row summary (subject ADV$ in $M + absolute tier /
    sector median/p25/p75 ADV$ in $M / peers considered/with data).
  - **SURPSTK** — 4-row summary (events breakdown beats/misses/inlines
    with beat rate / current + longest streaks / avg surprise % /
    latest event date + label + surprise %).

- **Command palette entries**:
  `LEVRANK | LEVERAGE_RANK | DE_RANK`,
  `OPERANK | OPERATING_RANK | OP_MARGIN_RANK`,
  `FQMRANK | FQM_RANK | QUALITY_METER_RANK`,
  `LIQRANK | LIQUIDITY_RANK | ADV_RANK`,
  `SURPSTK | SURPRISE_STREAK | EARNINGS_STREAK`.
  Each arm sets `show_*`, copies the current chart symbol into
  `*_symbol`, and opportunistically loads the cached snapshot.

### Research Packet (`docs/RESEARCH_PACKET.md`)

- Header count bumped: seventy-seven → **eighty-two** sub-blocks.
- New sections 2.77 Leverage Rank (LEVRANK), 2.78 Operating Quality
  Rank (OPERANK), 2.79 FQM Rank (FQMRANK), 2.80 Liquidity Rank
  (LIQRANK), 2.81 Earnings Surprise Streak (SURPSTK).
- Sector peer comparison renumbered 2.77 → **2.82**.
- Data sources table: 5 new rows pointing at the new getters.
- Packet size budget revised:
  - Single symbol: 30-58 KB → **32-61 KB**
  - Ten symbols: 290-580 KB → **310-620 KB**

### Native packet generator (`investigate_symbols`)

- Five new blocks in the per-symbol loop, appended after the REVRANK
  block under a new `// ── ADR-125 Round 18 ──` divider. Each block
  is gated on
  `label != "NO_DATA" / "INSUFFICIENT_DATA" && !label.is_empty()`
  (LEVRANK / OPERANK / FQMRANK / LIQRANK use `rank_label`, SURPSTK
  uses `streak_label`). LEVRANK has a special NEGATIVE_EQUITY branch
  that prints total_debt / total_equity instead of the D/E rank line.

## Alternatives considered

- **Let LEVRANK just be a boolean on the Fundamentals cross-join**
  (instead of reading the dedicated `research_leverage` cache).
  Rejected because `research_leverage` already carries a computed,
  smoothed `debt_to_equity` with period normalization and trend
  labels; reading Fundamentals raw would duplicate that logic and
  miss the period-of-record alignment.
- **Make OPERANK a four-component margin composite (gross / operating
  / net / EBITDA).** Rejected because the single-axis
  "operating margin percentile" is deliberately narrow — the whole
  point is to isolate the pricing-power signal from the fused
  FQM/QUAL composites. A multi-margin OPERANK would collapse right
  back into FQM's latitude.
- **Rank LIQRANK by share-count volume instead of dollar volume.**
  Rejected because share-count volume is nominal — a name trading
  10M shares at $2 isn't comparable to one trading 1M shares at
  $200. ADV$ is the only sector-comparable metric.
- **Use a 3-event MIXED floor for SURPSTK instead of 4.** Rejected
  because 3 events with 100% beats / 0% beats could be a
  two-quarters-plus-one-recent fluke, and the MIXED bucket is
  explicitly "we don't have a strong signal yet." 4 events matches
  the RELEPSGR / REVRANK CAGR floor.
- **Report `latest_revenue` / `earliest_revenue` for SURPSTK in
  addition to surprise %.** Rejected because SURPSTK is explicitly
  a *surprise* statistic, not an earnings level statistic. The packet
  already surfaces earnings levels via EARN / RELEPSGR / REVRANK;
  duplicating them in SURPSTK would clutter the line.
- **Cache SURPSTK against `research_earnings_surprises` as a
  dependent materialised view.** Rejected as premature optimisation:
  the surprise cache has O(20) rows per symbol, the sort + classify
  pass is O(n log n) on a 20-element list, and the whole thing runs
  in sub-millisecond time.
- **Dedicated "NEGATIVE_EQUITY_RANK" tier in LEVRANK's ladder.**
  Rejected because negative equity doesn't have a meaningful
  percentile — the D/E ratio is undefined and the name belongs in a
  categorical bucket, not on the 0-100 axis. The current approach
  (rank_label = "NEGATIVE_EQUITY", percentile_rank = 0.0) keeps the
  axis interpretation clean.

## Consequences

- Research packet grows another 1-3 KB / symbol on average when
  Round 18 caches are warm. The ten-symbol packet ceiling rises from
  ~580 KB to ~620 KB, still well under the ~650 KB soft target for
  model-readable single-turn input.
- Five new SQLite tables (`research_levrank`, `research_operank`,
  `research_fqmrank`, `research_liqrank`, `research_surpstk`) join
  the LAN-syncable set. Schema v18 is additive.
- **Second-order staleness chain now covers Round 15 and Round 17
  outputs as well.** LEVRANK depends on Round 15 LEV, OPERANK depends
  on Round 14 MARGINS, FQMRANK depends on Round 17 FQM, LIQRANK
  depends on Round 13 LIQ — their rank is only as fresh as the
  underlying factor cache. Users who need strict freshness should
  recompute LEV / MARGINS / FQM / LIQ first, then LEVRANK /
  OPERANK / FQMRANK / LIQRANK.
- **LEVRANK completes the "rank every composite factor" arc.** Round
  16 delivered VRK / QRK / RRK for VAL / QUAL / RISK, Round 17 added
  MOMF / PEADRANK for MOMENTUM / PEAD, Round 18 adds LEVRANK for
  LEVERAGE and the OPERANK / LIQRANK / FQMRANK rank overlays. Every
  factor surface in the terminal now has both an absolute and a
  peer-relative view.
- **SURPSTK is the first Round-18+ surface that is not a rank.** It's
  a symbol-local time-series statistic over the earnings surprise
  cache. This sets a useful precedent: not every new parity surface
  has to be a sector cross-join. Future rounds can add more
  symbol-local descriptors (e.g., "has this name ever cut the
  dividend," "what's the ex-dividend calendar for the next 4
  quarters") without the cross-join overhead.
- **Four new whole-table scan helpers are now canonical.**
  `get_all_leverage` / `get_all_margins` / `get_all_fqm` /
  `get_all_liquidity` join the Round 16/17 set
  (`get_all_val` / `get_all_qual` / `get_all_risk` /
  `get_all_momentum` / `get_all_pead`). After this round, the only
  Round-1-through-17 factor caches still missing a whole-table helper
  are the growth / solvency / efficiency / upgrade-momentum tables —
  and most of those are already covered by their own rank surfaces
  through the relevant `get_all_fundamentals` path.
- **Round 17 backfill.** During Round 18 implementation, discovered
  the Round 17 commit had merged the `SYNCABLE_TABLES` entries but
  missed adding the corresponding `table_timestamp_column()` arms for
  the five Round 17 tables. Round 18 adds all ten entries (Round 17 +
  Round 18) together, fixing the gap.

## Implementation notes

### LEVRANK risk-inversion

LEVRANK is the second risk-inverted rank surface (RRK from Round 16
was the first). The `percentile_rank_score` helper is called with
`higher_is_better=false` so a *lower* D/E translates to a *higher*
(safer) percentile. The label ladder mirrors RRK's: SAFEST_DECILE →
RISKIEST_DECILE instead of TOP_DECILE → BOTTOM_DECILE. Reader
expectation is that any *_DECILE label following the word "rank"
means the same thing: *"1-in-10 position, interpret direction from
the label prefix."*

### NEGATIVE_EQUITY short-circuit

When the subject's `total_equity <= 0`, the rank is undefined because
D/E diverges. Rather than propagating a sentinel `NaN` through
`percentile_rank_score`, the compute fn short-circuits to:

```rust
rank_label = "NEGATIVE_EQUITY"
percentile_rank = 0.0
rank_position = 0
sector_*_d2e = 0.0
```

and emits a note explaining the situation. This matches how other
rank surfaces handle "no data" cases: return a well-formed row with
a sentinel label rather than an error or a missing row. The native
window has a dedicated branch that renders this case specially (no
percentile grid, a "negative equity" badge, and the raw debt / equity
levels in $B for context).

### OPERANK vs FQM vs QRK

Three overlapping "quality" rank surfaces now exist:
- **QRK** (Round 16) ranks the Round 15 QUAL composite. QUAL fuses
  Piotroski + Margins + Accruals + Leverage.
- **FQMRANK** (Round 18) ranks the Round 17 FQM composite. FQM fuses
  Piotroski + Margins + Accruals **without Leverage**.
- **OPERANK** (Round 18) ranks **just** the operating margin — no
  fusion at all.

The three answer different questions:
- "How does this name rank on the full QUAL composite including
  balance sheet?" → QRK
- "How does this name rank on pure operator quality, independent
  of how levered the balance sheet is?" → FQMRANK
- "How does this name's pricing power (operating margin) compare
  to its sector?" → OPERANK

The packet emits all three when available because they disagree
*systematically*: a high-margin levered-up LBO refi vs a
low-margin unlevered growth name will show opposite patterns across
the three surfaces, and the differences are the signal.

### SURPSTK classification band

The ±2% band around `surprise_pct` for BEAT / INLINE / MISS
classification matches the industry convention used by earnings-reaction
studies. The cache already has `surprise_pct = (actual - estimate) / |estimate| × 100`,
so the band interpretation is unambiguous:

- surprise_pct > +2% → BEAT
- surprise_pct < -2% → MISS
- -2% ≤ surprise_pct ≤ +2% → INLINE

The INLINE band is deliberately narrow because "in-line" implies
a very tight match — a 1% miss is still a miss to most readers, not
an inline. A wider band would inflate the INLINE bucket at the
expense of BEAT and MISS, which would dilute the streak signal.

### SURPSTK streak ladder

The ladder intentionally has two thresholds per direction (HOT vs
BEAT_TREND, COLD vs MISS_TREND) so the difference between "quite
good" and "exceptionally consistent" can be expressed. The gating
conditions are:

- **HOT_STREAK**: beat_rate ≥ 75% AND current_streak_type = BEAT AND
  current_streak_len ≥ 3. Three consecutive beats in a row with a
  75%-plus overall rate.
- **BEAT_TREND**: beat_rate ≥ 60%. Two-thirds of all events are
  beats, but not necessarily consecutive.
- **COLD_STREAK**: beat_rate ≤ 25% AND current_streak_type = MISS
  AND current_streak_len ≥ 3. The inverse of HOT_STREAK.
- **MISS_TREND**: beat_rate ≤ 40%. Two-thirds of all events are
  non-beats.
- **MIXED**: between 40% and 60% beat rate.
- **INSUFFICIENT_DATA**: fewer than 4 events.

The 4-event floor matches the REVRANK / RELEPSGR CAGR floor and
means SURPSTK doesn't fire for names with fewer than ~1 year of
earnings history.

### Cross-join volume

Four of the five Round 18 surfaces use the cross-join pattern
(`get_all_<factor> + per-peer get_fundamentals` to recover sector).
On a cached universe of ~500 symbols per sector, each rank compute
does ~500 fundamentals lookups. SQLite primary-key-indexed lookups
are ~1 μs each, so the cross-join adds <1 ms per rank. Still well
under the 100 ms UI-latency budget.

### Test coverage

15 new tests in `engine/src/core/research::tests`:

- 5 roundtrip tests (`levrank_snapshot_roundtrip`,
  `operank_snapshot_roundtrip`, `fqmrank_snapshot_roundtrip`,
  `liqrank_snapshot_roundtrip`, `surpstk_snapshot_roundtrip`) verify
  schema_v18 create + upsert + get + JSON roundtrip.
- 2 LEVRANK tests (`compute_levrank_safest_decile`,
  `compute_levrank_negative_equity`) — the second exercises the
  NEGATIVE_EQUITY short-circuit.
- 1 OPERANK test (`compute_operank_top_decile`).
- 1 FQMRANK test (`compute_fqmrank_filters_no_data`) — verifies the
  peer filter drops `operator_label = "NO_DATA"` rows.
- 1 LIQRANK test (`compute_liqrank_filters_insufficient_data`) —
  verifies the peer filter drops `liquidity_tier = "INSUFFICIENT_DATA"`
  rows.
- 5 SURPSTK tests: `compute_surpstk_hot_streak`, `compute_surpstk_cold_streak`,
  `compute_surpstk_mixed`, `compute_surpstk_insufficient`, plus the
  roundtrip.

Engine test suite: **801 passed / 0 failed / 3 ignored** (786 from
Round 17 + 15 new).

## Future work

The parity sweep continues. Candidates for Round 19, still pure
compute over existing caches:

- **DVD — Dividend history snapshot.** Pure time-series stat over
  cached dividend events: streak (consecutive years of increases),
  CAGR of the payout, yield-on-cost vs current yield, payout-ratio
  trend. No sector needed. Sets up DVDRANK in a later round.
- **INSIDERCONC — Insider ownership concentration vs sector.** Ranks
  `Fundamentals.insiders_percent_held` or equivalent. Complement to
  Round 12's INSIDERS activity feed, which measures flow; this
  measures stock.
- **GY — Gap Yearly.** Pure time-series stat over the HP cache:
  counts the number of overnight gaps > X% in the last Y sessions.
  Useful risk measure for event-driven strategies.
- **UPDG — Upgrade/Downgrade momentum rank.** Sector-relative
  percentile rank of the Round X `UpgradeDowngradeSnapshot.net_score`.
- **BETA — Rolling beta to sector ETF and to SPY** over user-tunable
  windows, with a stability label. Still blocked on sector-ETF
  mapping and an SPY HP cache persistence decision.
- **CALPB — Put/Call ratio and skew term-structure.** Still blocked
  on richer OMON chain snapshots (multi-expiry).

The standing directive stands: continue until the compute-over-cache
well runs dry. Round 19 will pick the subset that doesn't need new
caches.
