# ADR-123: Godel Parity Round 16 — VRK / QRK / RRK / RELEPSGR / PEAD

**Status:** Accepted
**Date:** 2026-04-15
**Supersedes/extends:** ADR-108 through ADR-122
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Parity classification

| Feature | Godel Terminal documented | TA-Lib primitive | Research packet | egui popup | Chart overlay |
|---|---|---|---|---|---|
| VRK | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| QRK | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| RRK | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| RELEPSGR | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| PEAD | Yes | No | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** pure Godel-Terminal-documented factor-rank / event-drift surfaces (sector rank of VAL/QUAL/RISK composites, relative EPS growth, post-earnings drift); no TA-Lib primitives in this round.

## Context

Round 15 (ADR-122) shipped VAL / QUAL / RISK / INSSTRK / COVG and
closed out the "factor composite" tier of the parity sweep. Three of
those surfaces (VAL, QUAL, RISK) emit a 0-100 composite that scores
one symbol against absolute thresholds. What they don't do is tell the
reader **where that composite sits within its peer cohort** — a
symbol with a VAL composite of 65 looks solid on its own, but the
question investors actually ask is "is 65 above or below the median
for this sector?"

Round 16 fills that gap by adding three **rank surfaces** (VRK / QRK
/ RRK) that turn Round 15 composites into percentile ranks within a
sector cohort. Each rank surface reads the new `research_val` /
`research_qual` / `research_risk` tables end-to-end (the first parity
round to consume another parity round's output tables), computes
sector medians and quartiles, and emits a 1-based rank position plus a
decile/quartile label.

The remaining two surfaces (RELEPSGR, PEAD) were flagged as candidates
at the end of ADR-122 and are now unblocked:

1. **VRK — Value Rank vs Sector Peers.** Percentile rank of
   `ValueSnapshot.composite_score` within the same sector. Higher
   rank = better value. Emits TOP_DECILE / TOP_QUARTILE / ABOVE_MEDIAN
   / BELOW_MEDIAN / BOTTOM_QUARTILE / BOTTOM_DECILE / NO_DATA.
2. **QRK — Quality Rank vs Sector Peers.** Percentile rank of
   `QualitySnapshot.composite_score` within the same sector. Same
   label ladder as VRK (higher = higher quality).
3. **RRK — Risk Rank vs Sector Peers (inverted).** Percentile rank of
   `RiskSnapshot.composite_score` within the same sector. Critical
   inversion: RISK composite is higher = riskier, so the rank is
   inverted such that `percentile_rank = 100 - raw_percentile`, and
   the label ladder is SAFEST_DECILE / SAFEST_QUARTILE /
   ABOVE_MEDIAN_SAFE / BELOW_MEDIAN_RISKY / BOTTOM_QUARTILE_RISKY /
   RISKIEST_DECILE / NO_DATA. The `rank_position` is likewise
   1-based from safest.
4. **RELEPSGR — Relative 3y EPS CAGR vs Sector Median.** Computes the
   3-year compound annual growth rate of EPS from the cached
   `FinancialStatements.income_annual[].eps` series (requires ≥4
   annual rows), compares to the sector median CAGR, and emits
   FAR_ABOVE / ABOVE / INLINE / BELOW / FAR_BELOW / CAGR_NEGATIVE /
   NO_DATA. The CAGR_NEGATIVE label applies when either endpoint of
   the symbol's EPS series is non-positive (sign change or loss),
   falling back to a linear-growth proxy so the composite still emits
   a meaningful gap-to-median value.
5. **PEAD — Post-Earnings-Announcement Drift.** Joins cached
   `EarningsSurprise` rows with cached `HistoricalPriceRow` bars to
   measure average forward drift over 1 / 3 / 5 / 10 trading days
   after each announcement. Breaks out average drift by BEAT vs MISS
   classification at the 5d horizon, reports the latest event's
   surprise% and 5d drift separately, and emits DRIFT_UP / DRIFT_DOWN
   / MIXED / INSUFFICIENT_DATA based on the sign and magnitude of the
   avg 5d drift.

The standing directive continues: *"continue combing over vs godel
parity until we cannot add more. rinse/repeat do not worry about round
count."*

## Decision

Add five new research surfaces following the Round 10 through 15
pattern. Round 16 introduces one new infrastructure primitive: three
new `get_all_*` whole-table scan helpers in `research.rs`
(`get_all_val`, `get_all_qual`, `get_all_risk`) so the rank surfaces
can read every cached row of the matching factor table. These are the
analogues of `fundamentals::get_all_fundamentals` but for the Round 15
factor caches.

### Engine (`engine/src/core/research.rs`)

- **New structs** (lines 1755-1875, after `CoverageSnapshot`):
  - `ValueRankSnapshot` — VRK (symbol, as_of, sector, composite_score,
    peers_considered, peers_with_data, sector_median_score, sector_p25,
    sector_p75, percentile_rank, rank_position, rank_label, note).
  - `QualityRankSnapshot` — QRK (same shape as VRK but for QUAL).
    Sector is carried explicitly on the snapshot because
    `QualitySnapshot` itself does not carry sector — the handler has
    to resolve it via `fundamentals::get_fundamentals` for the subject.
  - `RiskRankSnapshot` — RRK (same shape; composite semantics are
    inverted — higher composite = riskier, higher percentile_rank =
    SAFER — and the label ladder reflects the inversion).
  - `RelativeEpsGrowthSnapshot` — RELEPSGR (symbol, as_of, sector,
    latest_eps, earliest_eps, years_used, symbol_cagr_pct,
    peers_considered, peers_with_data, sector_median_cagr_pct,
    sector_p25_cagr_pct, sector_p75_cagr_pct, gap_to_median_pp,
    relative_label, note).
  - `PeadEventRow` — one per earnings event (event_date, surprise_pct,
    classification, drift_1d_pct, drift_3d_pct, drift_5d_pct,
    drift_10d_pct).
  - `PeadSnapshot` — PEAD (symbol, as_of, num_events, events_used,
    avg_drift_1d/3d/5d/10d_pct, beat_event_drift_5d_pct,
    miss_event_drift_5d_pct, latest_event_date,
    latest_event_surprise_pct, latest_event_drift_5d_pct,
    drift_direction_label, `rows: Vec<PeadEventRow>`, note).

- **New compute fns** (after `compute_covg_snapshot`, lines 8894+):
  - `compute_vrk_snapshot(symbol, as_of, subject: Option<&ValueSnapshot>,
    peers: &[&ValueSnapshot])` — reads the subject's composite (and
    sector from the ValueSnapshot itself), builds an `others` vec of
    peer composites, calls `percentile_rank_score(value, others,
    higher_is_better=true)` to get the 0-100 percentile, computes p25
    / p50 / p75 via `quantile_f64`, and emits a decile-granularity
    label via `rank_label_for_percentile`.
  - `compute_qrk_snapshot(symbol, as_of, sector, subject, peers)` —
    same as VRK but sector is passed in (the QualitySnapshot doesn't
    carry it, so the broker handler resolves it from Fundamentals).
  - `compute_rrk_snapshot(symbol, as_of, sector, subject, peers)` —
    same shape but passes `higher_is_better=false` to
    `percentile_rank_score` and emits labels via
    `risk_rank_label_for_percentile`. `rank_position` is 1-based from
    safest (lowest composite).
  - `compute_relepsgr_snapshot(symbol, as_of, sector, subject:
    Option<&FinancialStatements>, peer_statements: &[(String,
    FinancialStatements)])` — calls `eps_cagr_3y_from_statements` for
    the subject and every peer with ≥4 annual rows, computes the
    sector median / p25 / p75 of peer CAGRs, and labels by
    `gap_to_median = symbol_cagr - sector_median`. Labels (in
    percentage points): FAR_ABOVE ≥ +10, ABOVE ≥ +3, INLINE within
    ±3, BELOW ≤ -3, FAR_BELOW ≤ -10. CAGR_NEGATIVE overrides when
    subject's CAGR is negative.
  - `compute_pead_snapshot(symbol, as_of, surprises:
    &[EarningsSurprise], bars_newest_first: &[HistoricalPriceRow])` —
    walks each surprise, calls `find_t0_index_newest_first` to locate
    the bar at (or just before) the announcement date, computes
    forward returns at 1 / 3 / 5 / 10 trading days by indexing
    `bars[t0_idx - N]` (newest-first ordering), classifies each event
    as BEAT / MISS / INLINE by surprise%, and averages the drifts.
    Requires ≥11 bars per event (t0 + 10 forward).

- **New helpers** (co-located with the rank compute fns):
  - `fn quantile_f64(sorted: &[f64], q: f64) -> f64` — linear
    interpolation between the two nearest sorted values.
  - `fn percentile_rank_score(value: f64, others: &[f64],
    higher_is_better: bool) -> f64` — midrank convention:
    `(below + 0.5 * equal) / total * 100`.
  - `fn rank_label_for_percentile(pct: f64) -> &'static str` — decile
    ladder (TOP_DECILE ≥90, TOP_QUARTILE ≥75, ABOVE_MEDIAN ≥50,
    BELOW_MEDIAN ≥25, BOTTOM_QUARTILE ≥10, BOTTOM_DECILE <10).
  - `fn risk_rank_label_for_percentile(pct: f64) -> &'static str` —
    inverted ladder (SAFEST_DECILE ≥90, ... RISKIEST_DECILE <10).
  - `fn eps_cagr_3y_from_statements(stmts: &FinancialStatements) ->
    (latest, earliest, years_used, cagr_pct)` — requires ≥4 annual
    income rows; falls back to linear growth when signs cross.
  - `fn find_t0_index_newest_first(bars: &[HistoricalPriceRow],
    target_date: &str) -> Option<usize>` — bars are newest-first,
    finds the first index whose date is ≤ target_date.

- **Schema v16** (`create_research_tables_v16`): calls v15 first,
  then creates `research_vrk`, `research_qrk`, `research_rrk`,
  `research_relepsgr`, `research_pead`, each shaped the same way
  `(symbol TEXT PRIMARY KEY, snapshot_json TEXT, updated_at INTEGER)`
  with an `updated_at` index. Schema v16 is additive: no existing
  Round 1-15 tables change layout.

- **Upsert/get wrappers** (after `get_covg`):
  `upsert_vrk` / `get_vrk`, `upsert_qrk` / `get_qrk`,
  `upsert_rrk` / `get_rrk`, `upsert_relepsgr` / `get_relepsgr`,
  `upsert_pead` / `get_pead`. Standard `INSERT ... ON CONFLICT` +
  serde-JSON roundtrip.

- **New whole-table scans** (for the rank surfaces):
  `get_all_val`, `get_all_qual`, `get_all_risk`. Each returns
  `Result<Vec<{Value,Quality,Risk}Snapshot>>` and is implemented as
  `query_map([], |row| row.get::<_, String>(0))` over the matching
  `research_*` table, with serde-JSON deserialisation per row. The
  scan is O(n) but cheap — expected to be called only when the user
  opens VRK/QRK/RRK, never on the UI's render loop.

### LAN sync (`engine/src/core/lan_sync.rs`)

- `SYNCABLE_TABLES` — add `research_vrk`, `research_qrk`,
  `research_rrk`, `research_relepsgr`, `research_pead` under a new
  `// ── ADR-123 Round 16 ──` divider.
- `create_table_sql()` — 5 new arms emitting the same DDL as the
  engine's `create_research_tables_v16`.
- `table_timestamp_column()` — 5 new arms returning `"updated_at"`
  for each new table.

### Native (`native/src/app.rs`)

- **BrokerCmd variants** (after `ComputeCovgSnapshot`):
  - `ComputeVrkSnapshot { symbol }`
  - `ComputeQrkSnapshot { symbol }`
  - `ComputeRrkSnapshot { symbol }`
  - `ComputeRelepsgrSnapshot { symbol }`
  - `ComputePeadSnapshot { symbol }`

- **BrokerMsg variants** (after `CovgSnapshotMsg`) under a new
  `// ── ADR-123 ──` divider:
  - `VrkSnapshotMsg(String, ValueRankSnapshot)`
  - `QrkSnapshotMsg(String, QualityRankSnapshot)`
  - `RrkSnapshotMsg(String, RiskRankSnapshot)`
  - `RelepsgrSnapshotMsg(String, RelativeEpsGrowthSnapshot)`
  - `PeadSnapshotMsg(String, PeadSnapshot)`

- **TyphooNApp state fields** (after `covg_loading`) under a new
  `// ── ADR-123 Godel Parity Round 16 ──` divider. Each surface gets
  `show_*` / `*_symbol` / `*_snapshot` / `*_loading`.

- **Broker handler spawns** (after the COVG handler). Each one follows
  the established `tokio::spawn` + `shared_cache_broker` pattern and
  pre-reads the caches needed on the task thread:
  - **VRK handler** calls `research::get_all_val(&conn)` and filters
    by sector directly (VAL carries sector in its struct, so no
    cross-join with fundamentals is needed). This is the simplest
    rank handler.
  - **QRK handler** is more complex: `QualitySnapshot` does not carry
    sector, so after `get_all_qual`, the handler does a per-peer
    lookup against `fundamentals::get_fundamentals` and filters to
    peers whose sector matches the subject's sector. Cost is
    O(peers × 1) extra fundamental reads, but typically fewer than 50
    peers per sector.
  - **RRK handler** — same cross-join pattern as QRK, using
    `get_all_risk`.
  - **RELEPSGR handler** — iterates `fundamentals::get_all_fundamentals`,
    filters to peer sector, and calls `get_financials` per peer for
    the EPS series. The two-step read keeps the SQL simple at the
    cost of one extra query per peer.
  - **PEAD handler** — simplest of the five: pre-reads
    `get_earnings_surprises(&conn, &symbol)` and
    `get_historical_price(&conn, &symbol)`, then passes both slices
    into `compute_pead_snapshot`.

- **Receive arms** (in the `BrokerMsg` match, after
  `CovgSnapshotMsg`): each arm updates the matching state field if
  the incoming symbol matches `*_symbol`, then unconditionally
  upserts the snapshot into the cache via `upsert_*`. The upsert is
  unconditional so LAN-synced receivers get the benefit of the
  compute even when no window is open.

- **egui windows** (after the COVG window, 640×360 for rank surfaces,
  640×380 for RELEPSGR, 720×480 for PEAD to accommodate the per-event
  detail table). Each window follows the established header pattern:
  symbol editor / Use Chart / Load Cached / Compute / Loading
  spinner, a color-coded summary line, and per-surface grids:
  - **VRK / QRK** — 3-row summary grid (subject composite / sector
    median/p25/p75 / peers considered/with data).
  - **RRK** — identical grid but labelled "higher = riskier" on the
    composite row and "safe pct" on the summary header.
  - **RELEPSGR** — 3-row grid (latest/earliest EPS with years_used /
    sector median/p25/p75 CAGR / peers considered/with data).
  - **PEAD** — 4-row summary grid (avg drift 1d/3d/5d/10d / beat 5d /
    miss 5d / latest event with surprise + 5d drift / events used vs
    in cache) plus a scrollable per-event table (date, classification,
    surprise%, 1d/3d/5d/10d drifts).

- **Command palette entries**:
  `VRK | VALUE_RANK | VAL_RANK`,
  `QRK | QUALITY_RANK | QUAL_RANK`,
  `RRK | RISK_RANK`,
  `RELEPSGR | REL_EPS_GROWTH | RELATIVE_EPS_GROWTH | EPSGR`,
  `PEAD | EARNINGS_DRIFT | POST_EARNINGS_DRIFT`.
  Each arm sets `show_*`, copies the current chart symbol into
  `*_symbol`, and opportunistically loads the cached snapshot.

### Research Packet (`docs/RESEARCH_PACKET.md`)

- Header count bumped: sixty-seven → **seventy-two** sub-blocks.
- New sections 2.67 Value Rank (VRK), 2.68 Quality Rank (QRK),
  2.69 Risk Rank (RRK), 2.70 Relative EPS Growth (RELEPSGR),
  2.71 Post-Earnings Drift (PEAD).
- Sector peer comparison renumbered 2.67 → **2.72**.
- Size caps table: 5 new rows.
- Data sources table: 5 new rows pointing at the new getters.
- Packet size budget revised:
  - Single symbol: 26-52 KB → **28-55 KB**
  - Ten symbols: 250-500 KB → **270-540 KB**

### Native packet generator (`investigate_symbols`)

- Five new blocks in the per-symbol loop, appended after the COVG
  block. Each block is gated on
  `label != "NO_DATA" / "INSUFFICIENT_DATA" && !label.is_empty()`.
  The blocks render the header (label + composite/CAGR/drift + as_of),
  a sector/percentile/rank summary line, and (for PEAD) the latest
  event summary. VRK/QRK/RRK each render the sector's p25/p50/p75
  triple so the model can judge where the subject sits inside the
  cohort distribution, not just against the median.

## Alternatives considered

- **Make VRK/QRK/RRK operate over the whole cache instead of just the
  sector.** Rejected because the Godel factor-rank column is
  sector-relative, and a universe-wide rank would bury sector rotation
  signals (e.g., a "premium" utility looks cheap next to an "average"
  software name). Sector-relative ranks are the standard factor-
  investing convention.
- **Compute the rank by z-score instead of percentile.** Rejected for
  the same reason VAL uses ratio-vs-median in ADR-122: z-scores need
  a larger sample for stable means and stdevs, and rank/percentile
  degrades more gracefully when only 3-5 peers are available.
- **Let RRK emit "higher = riskier" like the underlying RISK
  composite.** Rejected because the label ladder becomes harder to
  read side-by-side with VRK/QRK (which have higher = better). The
  inversion costs one boolean flag in the compute fn and adds
  ladder-semantic clarity: "SAFEST_DECILE" is immediately
  interpretable; "BOTTOM_DECILE for risk" is not.
- **Use 4-year CAGR for RELEPSGR instead of 3-year.** Rejected
  because 4-year windows require ≥5 annual rows which knocks out
  recently-IPO'd names; 3-year is the industry-standard "medium-term
  growth" horizon and only needs ≥4 rows.
- **Weight RELEPSGR's label by sector median CAGR magnitude.**
  Rejected because the gap-in-percentage-points is already
  scale-free (a 3pp gap is always meaningful), and weighting it
  introduces a second tunable that isn't obviously better.
- **Treat PEAD's CAGR_NEGATIVE as an error rather than a label.**
  Rejected because ~10-15 % of the cached names have a transient
  loss year in their EPS history, and dropping them loses coverage.
  The fallback (linear growth proxy + label) preserves the signal.
- **Compute PEAD over calendar days instead of trading days.**
  Rejected because forward drift over the first N calendar days is
  dominated by weekend/holiday noise and diverges from academic PEAD
  convention. Using `t0_idx - N` on a newest-first bar array gives
  trading-day semantics for free.
- **Cache the sector-peer median for each Round 15 composite as a
  standalone table.** Rejected as premature optimisation. The rank
  surfaces recompute medians per-call, which is O(n) over a few
  thousand VAL rows — cheap enough to run on demand. If a future
  sweep needs instant-lookup sector baselines, the table can be added
  without rewriting the rank surfaces.

## Consequences

- Research packet grows another 1-3 KB / symbol on average when
  Round 16 caches are warm. The ten-symbol packet ceiling rises from
  ~500 KB to ~540 KB, still well under the ~600 KB soft target for
  model-readable single-turn input.
- Five new SQLite tables (`research_vrk`, `research_qrk`,
  `research_rrk`, `research_relepsgr`, `research_pead`) join the
  LAN-syncable set. Schema v16 is additive.
- **First round to consume parity-round outputs as inputs.** VRK,
  QRK, and RRK read the `research_val` / `research_qual` /
  `research_risk` tables end-to-end — they are rank overlays on
  Round 15's absolute composites. This introduces a **two-stage
  staleness chain**: if a VAL cache is stale, the VRK rank built on
  top of it is also stale, and the VRK cache's `as_of` field does
  not propagate the VAL cache's earlier `as_of`. Users who need
  strict freshness should recompute VAL first, then VRK.
- **Three new whole-table scan helpers are now canonical primitives.**
  `get_all_val` / `get_all_qual` / `get_all_risk` follow the same
  shape as `get_all_fundamentals` and are the reference pattern for
  any future parity sweep that needs cross-sectional analytics over
  a research cache.
- **RELEPSGR closes the "growth relative to peers" gap.** Round 14
  GROWM measured growth composite for the symbol alone; RELEPSGR
  puts that growth in context. A company with a 20% CAGR looks
  great in isolation, but if every peer in its sector is growing
  25%, the relative signal is neutral-to-negative.
- **PEAD unblocks the "earnings-day edge" class of signals.** Before
  Round 16, the engine could tell the user "AAPL beat by 5%" but
  could not answer "how have AAPL's beats historically drifted over
  the next week?" PEAD answers that question from cached data with
  no new API dependency. This is the first signal that explicitly
  fuses the EarningsSurprise cache with the HP bar cache.
- **Round 15 factor rank + Round 16 peer rank is the factor-investing
  closure.** With VAL/QUAL/RISK absolute + VRK/QRK/RRK relative, the
  terminal now renders both "how does this symbol score" and "how
  does this symbol rank" — the two columns a factor screen needs.
  Subsequent rounds focused on factor work will hit diminishing
  returns.

## Implementation notes

### Percentile rank midrank convention

`percentile_rank_score` uses the midrank formula
`(strictly_below + 0.5 × equal) / total × 100`. This is the standard
definition used in rank-based statistics and handles ties gracefully
(two identical values both end up at the same midrank rather than one
getting an artificial edge). For `higher_is_better=false`, the formula
flips: `(strictly_above + 0.5 × equal) / total × 100`, which is
equivalent to `100 - raw_percentile` for non-tied cases.

### RRK rank position semantics

`rank_position` is 1-based and counts from the best end of the ladder
(highest percentile = rank 1). For VRK/QRK, rank 1 is the best-value
or highest-quality peer. For RRK, rank 1 is the **safest** peer
(lowest raw RISK composite). The subject is included in the rank
count, so `peers_considered + 1` is the total cohort size.

### RELEPSGR CAGR sign handling

`eps_cagr_3y_from_statements` only emits a valid compound-growth rate
when both the latest and earliest EPS values are strictly positive.
When either endpoint is ≤ 0, the function falls back to a linear
growth proxy: `((latest - earliest) / |earliest|) / years × 100`.
This keeps the snapshot emitting a meaningful gap-to-median even when
the symbol went through a loss year. The `relative_label` is
overridden to CAGR_NEGATIVE in that case so the packet can distinguish
"measured CAGR" from "proxied growth rate."

### PEAD newest-first bar indexing

The `HistoricalPriceRow` cache is ordered newest-first (bars[0] is the
most recent). To measure forward drift from an earnings announcement,
`compute_pead_snapshot` first locates `t0_idx` (the bar at or just
before the announcement date), then reads the forward bar as
`bars[t0_idx - N]` for N = 1, 3, 5, 10. This requires
`t0_idx >= 10`, which is why events near the end of the cached
window get dropped silently.

### PEAD BEAT/MISS classification thresholds

Events are classified as BEAT when surprise_pct ≥ +1.0%, MISS when
surprise_pct ≤ -1.0%, and INLINE otherwise. The ±1% band is narrow
enough to catch "slight beat" and "slight miss" separately but wide
enough to avoid classifying rounding noise as a directional event.
The breakdown matters because BEAT drift and MISS drift are
asymmetric in the empirical literature — BEAT drift is typically
2-3× larger in magnitude than MISS drift over the 5-10 day horizon.

### QRK / RRK cross-join with fundamentals

Because `QualitySnapshot` and `RiskSnapshot` don't carry sector as a
field (they were not designed with sector-peer rank in mind), the
QRK and RRK broker handlers must resolve each peer's sector through
a per-peer `fundamentals::get_fundamentals` call. A future refactor
could add `sector: String` to those two snapshots as part of their
compute pass, which would cut this extra DB hop. For now the cross-
join is acceptable because sector cohorts are small (typically ≤50
peers per sector in the cached universe).

### Test coverage

15 new tests in `engine/src/core/research::tests`:

- 5 roundtrip tests (`vrk_snapshot_roundtrip`, `qrk_snapshot_roundtrip`,
  `rrk_snapshot_roundtrip`, `relepsgr_snapshot_roundtrip`,
  `pead_snapshot_roundtrip`) verify schema_v16 create + upsert + get +
  JSON roundtrip.
- 2 VRK tests (`compute_vrk_top_decile`, `compute_vrk_no_data`).
- 2 QRK tests (`compute_qrk_above_median`, `compute_qrk_no_data`).
- 2 RRK tests (`compute_rrk_safest`, `compute_rrk_riskiest`).
- 2 RELEPSGR tests (`compute_relepsgr_above_median`,
  `compute_relepsgr_cagr_negative`).
- 2 PEAD tests (`compute_pead_drift_up`, `compute_pead_insufficient`).

Engine test suite: **771 passed / 0 failed / 3 ignored** (756 from
Round 15 + 15 new).

## Future work

The parity sweep continues. Candidates for Round 17, all still pure
compute over existing caches:

- **FQM — Fundamental Quality Meter.** Still parked from ADR-122 for
  the signal-laundering concern. Opening it would require either a
  one-layer-deep rule exception or a careful provenance check that
  the fused inputs come from distinct source caches.
- **SIZEF — Size Factor Rank.** Percentile rank of market cap within
  the symbol's sector, plus a size-tier label (MEGA / LARGE / MID /
  SMALL / MICRO). Needs only Fundamentals + sector.
- **MOMF — Momentum Factor Rank.** Percentile rank of Round 10 MOM
  composite within the sector, analogous to VRK but for the momentum
  cache. Would benefit from the same cross-sectional approach.
- **BETA — Rolling beta to sector ETF and to SPY** over user-tunable
  windows, with a stability label. Still blocked on sector-ETF
  mapping and an SPY HP cache persistence decision.
- **CALPB — Put/Call ratio and skew term-structure.** Still blocked
  on richer OMON chain snapshots (multi-expiry).
- **PEADRANK — Percentile rank of PEAD avg drift within sector.**
  The natural Round 16 follow-on — how does this name's historical
  post-earnings drift compare to its peers?

The standing directive stands: continue until the compute-over-cache
well runs dry. Round 17 will pick the subset that doesn't need new
caches.
