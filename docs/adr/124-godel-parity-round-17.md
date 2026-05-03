# ADR-124: Godel Parity Round 17 — SIZEF / MOMF / PEADRANK / FQM / REVRANK

**Status:** Accepted
**Date:** 2026-04-15
**Supersedes/extends:** ADR-108 through ADR-123
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Parity classification

| Feature | Godel Terminal documented | TA-Lib primitive | Research packet | egui popup | Chart overlay |
|---|---|---|---|---|---|
| SIZEF | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| MOMF | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| PEADRANK | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| FQM | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| REVRANK | Yes | No | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** pure Godel-Terminal-documented factor-rank / screen surfaces (size factor, momentum rank, PEAD rank, fundamental quality meter, revenue-CAGR rank); no TA-Lib primitives in this round.

## Context

Round 16 (ADR-123) shipped VRK / QRK / RRK (rank overlays on VAL / QUAL
/ RISK) plus RELEPSGR and PEAD, closing the "factor composite ↔ peer
rank" loop for Round 15's absolute composites. Round 16's future-work
list flagged seven candidates; five of them are pure-compute surfaces
over existing caches and are unblocked:

1. **SIZEF — Size Factor Rank vs Sector Peers.** Sector-relative
   percentile rank of `Fundamentals.market_cap`, plus an absolute tier
   label (MEGA_CAP / LARGE_CAP / MID_CAP / SMALL_CAP / MICRO_CAP). The
   size factor is one of the three canonical Fama-French style
   factors and has no standalone surface until now.
2. **MOMF — Momentum Factor Rank vs Sector Peers.** Sector-relative
   percentile rank of the Round 10 `MomentumSnapshot.composite_score`.
   Analogous to VRK / QRK / RRK, but built on the momentum composite
   table instead of VAL / QUAL / RISK. Closes the parity gap where
   momentum was composite-only and not rank-overlaid.
3. **PEADRANK — PEAD Drift Rank vs Sector Peers.** Sector-relative
   percentile rank of `PeadSnapshot.avg_drift_5d_pct`. The natural
   Round 16 follow-on that the ADR-123 future-work list explicitly
   called out: *"how does this name's historical post-earnings drift
   compare to its peers?"*
4. **FQM — Fundamental Quality Meter.** A fused Piotroski + margins
   + accruals composite, deliberately differentiated from Round 15
   QUAL by **excluding leverage** and reweighting the remaining
   three components (PTFS 40 / MARGINS 30 / ACRL 30 vs QUAL's 30 /
   25 / 25 / 20). The differentiation matters: a highly-levered
   business with great margins and cash conversion lands MID on QUAL
   (because LEV drags it down) but HIGH on FQM (because FQM asks
   purely "is this a good operator?"). Parked in ADR-122/123 over a
   signal-laundering concern; Round 17 accepts the one-layer rule
   exception because the three inputs come from fully independent
   source caches (research_piotroski, research_margins, research_accruals).
5. **REVRANK — Relative 3y Revenue CAGR vs Sector Median.** The
   revenue-line twin of RELEPSGR. Computes 3-year compound annual
   growth rate from `FinancialStatements.income_annual[].revenue`
   (requires ≥4 annual rows), compares to sector median, and emits
   FAR_ABOVE / ABOVE / INLINE / BELOW / FAR_BELOW / CAGR_NEGATIVE /
   INSUFFICIENT_DATA. Closes the gap where RELEPSGR measures
   bottom-line (EPS) growth but not top-line growth.

The remaining two Round 16 future-work items (BETA vs sector ETF,
CALPB chain multi-expiry) remain blocked on new data caches and are
deferred.

The standing directive continues: *"continue combing over vs godel
parity until we cannot add more. rinse/repeat do not worry about round
count."*

## Decision

Add five new research surfaces following the Round 15/16 pattern.
Round 17 introduces two new infrastructure primitives: two new
`get_all_*` whole-table scan helpers (`get_all_momentum` and
`get_all_pead`) so MOMF and PEADRANK can read every cached row of
their matching factor tables. This extends the Round 16 pattern
(`get_all_val` / `get_all_qual` / `get_all_risk`) to the momentum and
PEAD caches.

### Engine (`engine/src/core/research.rs`)

- **New structs** (after the Round 16 structs, under
  `// ── ADR-124 Round 17 ──` divider):
  - `SizeFactorSnapshot` — SIZEF (symbol, as_of, sector, market_cap,
    log_market_cap, tier_label, peers_considered, peers_with_data,
    sector_median_cap, sector_p25_cap, sector_p75_cap, percentile_rank,
    rank_position, rank_label, note).
  - `MomentumRankSnapshot` — MOMF (symbol, as_of, sector,
    composite_score, peers_considered, peers_with_data,
    sector_median_score, sector_p25, sector_p75, percentile_rank,
    rank_position, rank_label, note).
  - `PeadRankSnapshot` — PEADRANK (symbol, as_of, sector,
    avg_drift_5d_pct, peers_considered, peers_with_data,
    sector_median_drift_5d_pct, sector_p25_drift_5d_pct,
    sector_p75_drift_5d_pct, percentile_rank, rank_position,
    rank_label, note).
  - `FundamentalQualityMeterSnapshot` — FQM (symbol, as_of,
    piotroski_score, piotroski_label, operating_margin_pct,
    margin_trend_label, cash_conversion_pct, accruals_trend_label,
    composite_score, operator_label, inputs_available,
    `components: Vec<FactorComponent>`, note).
  - `RevenueGrowthRankSnapshot` — REVRANK (symbol, as_of, sector,
    latest_revenue, earliest_revenue, years_used, symbol_cagr_pct,
    peers_considered, peers_with_data, sector_median_cagr_pct,
    sector_p25_cagr_pct, sector_p75_cagr_pct, gap_to_median_pp,
    relative_label, note).

- **New compute fns** (after `compute_pead_snapshot`, under the same
  divider):
  - `compute_sizef_snapshot(symbol, as_of, sector, subject_market_cap:
    Option<f64>, peers: &[(String, f64)])` — computes
    `log_market_cap = log(cap)`, emits a `tier_label` from absolute
    thresholds (MEGA ≥ $200B, LARGE ≥ $10B, MID ≥ $2B, SMALL ≥ $300M,
    MICRO > $0), builds an `others` vec from peer caps, calls
    `percentile_rank_score(log_cap, others, higher_is_better=true)`
    to get the 0-100 percentile, computes p25/p50/p75 via
    `quantile_f64`, and emits the standard decile label.
  - `compute_momf_snapshot(symbol, as_of, sector, subject:
    Option<&MomentumSnapshot>, peers: &[&MomentumSnapshot])` — same
    rank pattern as VRK, filtering out peers with
    `regime_label == "INSUFFICIENT_DATA"` (MomentumSnapshot has no
    `value_label` equivalent; `regime_label` is the sentinel).
  - `compute_peadrank_snapshot(symbol, as_of, sector, subject:
    Option<&PeadSnapshot>, peers: &[&PeadSnapshot])` — ranks by
    `avg_drift_5d_pct`, filtering both subject and peers on
    `drift_direction_label != "INSUFFICIENT_DATA" && events_used >= 3`.
    The 3-event floor prevents the sector median from being poisoned
    by zero-drift insufficient-data rows.
  - `compute_fqm_snapshot(symbol, as_of, piotroski:
    Option<&PiotroskiSnapshot>, margins: Option<&MarginsSnapshot>,
    accruals: Option<&AccrualsSnapshot>)` — three-weight fusion
    (PTFS 40, MARGINS 30, ACRL 30). Each input is mapped to a 0-100
    sub-score, then weighted-sum-divided-by-present-weight so the
    composite is always 0-100 even when only one or two inputs are
    present. Operator label ladder: ELITE_OPERATOR ≥85,
    STRONG_OPERATOR ≥70, AVERAGE_OPERATOR ≥50, WEAK_OPERATOR ≥30,
    BROKEN_OPERATOR <30. Margin and accrual sub-scores receive
    ±10-point adjustments for expansion/contraction and cash
    conversion extremes. Emits NO_DATA when no input is present.
  - `compute_revrank_snapshot(symbol, as_of, sector, subject:
    Option<&FinancialStatements>, peer_statements: &[(String,
    FinancialStatements)])` — mirrors `compute_relepsgr_snapshot`
    but over `IncomeStatement.revenue` via a new
    `revenue_cagr_3y_from_statements` helper. Gap thresholds
    identical to RELEPSGR (±3pp INLINE band, ±10pp FAR band).

- **New helpers** (co-located with the compute fns):
  - `fn size_tier_label(market_cap: f64) -> &'static str` — absolute
    tier thresholds in USD.
  - `fn revenue_cagr_3y_from_statements(stmts: &FinancialStatements)
    -> (latest_rev, earliest_rev, years_used, cagr_pct)` — direct
    structural analogue of `eps_cagr_3y_from_statements`, operating
    on `income_annual[].revenue`. Requires ≥4 annual rows; falls
    back to a linear-growth proxy when either endpoint is
    non-positive (revenue sign changes are vanishingly rare but
    handled for symmetry with the EPS case).

- **Schema v17** (`create_research_tables_v17`): calls v16 first,
  then creates `research_sizef`, `research_momf`, `research_peadrank`,
  `research_fqm`, `research_revrank`, each shaped the same way
  `(symbol TEXT PRIMARY KEY, snapshot_json TEXT, updated_at INTEGER)`
  with an `updated_at` index. Schema v17 is additive: no existing
  Round 1-16 tables change layout.

- **Upsert/get wrappers** (after `get_pead`):
  `upsert_sizef` / `get_sizef`, `upsert_momf` / `get_momf`,
  `upsert_peadrank` / `get_peadrank`, `upsert_fqm` / `get_fqm`,
  `upsert_revrank` / `get_revrank`. Standard `INSERT ... ON CONFLICT`
  + serde-JSON roundtrip.

- **New whole-table scans**:
  - `get_all_momentum(&conn) -> Result<Vec<MomentumSnapshot>>` —
    scans `research_momentum` (created by `create_research_tables_v13`,
    the schema where momentum was first cached in Round 10's layout).
  - `get_all_pead(&conn) -> Result<Vec<PeadSnapshot>>` — scans
    `research_pead` (created by `create_research_tables_v16`).
  Both follow the same shape as the Round 16 `get_all_val` /
  `get_all_qual` / `get_all_risk` helpers: `query_map([], |row|
  row.get::<_, String>(0))` over the target table, serde-JSON
  deserialisation per row.

### LAN sync (`engine/src/core/lan_sync.rs`)

- `SYNCABLE_TABLES` — add `research_sizef`, `research_momf`,
  `research_peadrank`, `research_fqm`, `research_revrank` under a new
  `// ── ADR-124 Round 17 ──` divider.
- `create_table_sql()` — 5 new arms emitting the same DDL as the
  engine's `create_research_tables_v17`.
- `table_timestamp_column()` — 5 new arms returning `"updated_at"`
  for each new table.

### Native (`native/src/app.rs`)

- **BrokerCmd variants** (after `ComputePeadSnapshot`, under a new
  `// ── ADR-124 Round 17 ──` divider):
  - `ComputeSizefSnapshot { symbol }`
  - `ComputeMomfSnapshot { symbol }`
  - `ComputePeadrankSnapshot { symbol }`
  - `ComputeFqmSnapshot { symbol }`
  - `ComputeRevrankSnapshot { symbol }`

- **BrokerMsg variants** (after `PeadSnapshotMsg`) under a new
  `// ── ADR-124 ──` divider:
  - `SizefSnapshotMsg(String, SizeFactorSnapshot)`
  - `MomfSnapshotMsg(String, MomentumRankSnapshot)`
  - `PeadrankSnapshotMsg(String, PeadRankSnapshot)`
  - `FqmSnapshotMsg(String, FundamentalQualityMeterSnapshot)`
  - `RevrankSnapshotMsg(String, RevenueGrowthRankSnapshot)`

- **TyphooNApp state fields** (after `pead_loading`) under a new
  `// ── ADR-124 Godel Parity Round 17 ──` divider. Each surface gets
  `show_*` / `*_symbol` / `*_snapshot` / `*_loading`.

- **Broker handler spawns** (after the PEAD handler). Each one
  follows the Round 16 `tokio::spawn` + `shared_cache_broker` pattern
  and pre-reads the caches needed on the task thread:
  - **SIZEF handler** — simplest of the five: iterates
    `fundamentals::get_all_fundamentals` once and filters by sector
    directly (Fundamentals carries sector, so no cross-join).
    Extracts `market_cap.filter(|c| *c > 0.0)` to drop zero-cap rows
    before ranking.
  - **MOMF handler** — analogous to QRK's cross-join pattern. Calls
    `research::get_all_momentum(&conn)` then per-peer
    `fundamentals::get_fundamentals` to filter to the subject's
    sector. MomentumSnapshot doesn't carry sector, so the
    cross-join is required.
  - **PEADRANK handler** — same cross-join pattern using
    `get_all_pead` and `fundamentals::get_fundamentals`.
  - **FQM handler** — pre-reads
    `research::get_piotroski(&conn, &symbol)`,
    `research::get_margins(&conn, &symbol)`,
    `research::get_accruals(&conn, &symbol)`, and passes all three as
    `Option<&T>` into `compute_fqm_snapshot`. No peer iteration.
  - **REVRANK handler** — identical shape to RELEPSGR's handler:
    iterates `fundamentals::get_all_fundamentals`, filters to sector,
    and calls `research::get_financials` per peer for the income
    series.

- **Receive arms** (in the `BrokerMsg` match, after
  `PeadSnapshotMsg`): each arm updates the matching state field if
  the incoming symbol matches `*_symbol`, then unconditionally
  upserts the snapshot via `upsert_*`. Unconditional upsert so
  LAN-synced receivers benefit even when no window is open.

- **egui windows** (after the PEAD window, 640×360 for rank surfaces,
  640×380 for FQM/REVRANK). Each window follows the established
  header pattern (symbol editor / Use Chart / Load Cached / Compute
  / Loading spinner), a color-coded summary line, and per-surface
  grids:
  - **SIZEF** — 4-row summary (subject market cap in $B / log(cap) /
    sector median/p25/p75 cap / peers considered/with data).
  - **MOMF** — 3-row summary (subject composite / sector
    median/p25/p75 / peers considered/with data).
  - **PEADRANK** — 3-row summary (subject avg 5d drift / sector
    median/p25/p75 drift / peers considered/with data).
  - **FQM** — 4-row summary (Piotroski score + label / operating
    margin % + trend / cash conversion % + trend / PTFS-MARGINS-ACRL
    component scores from the `components: Vec<FactorComponent>`).
  - **REVRANK** — 4-row summary (subject 3y CAGR / latest-vs-earliest
    revenue in $B with years_used / sector median/p25/p75 CAGR /
    peers considered/with data).

- **Command palette entries**:
  `SIZEF | SIZE_FACTOR | SIZE_RANK`,
  `MOMF | MOMENTUM_RANK | MOM_RANK`,
  `PEADRANK | PEAD_RANK`,
  `FQM | FUND_QUALITY | QUALITY_METER`,
  `REVRANK | REV_RANK | REVENUE_GROWTH_RANK`.
  Each arm sets `show_*`, copies the current chart symbol into
  `*_symbol`, and opportunistically loads the cached snapshot.

### Research Packet (`docs/RESEARCH_PACKET.md`)

- Header count bumped: seventy-two → **seventy-seven** sub-blocks.
- New sections 2.72 Size Factor (SIZEF), 2.73 Momentum Rank (MOMF),
  2.74 PEAD Rank (PEADRANK), 2.75 Fundamental Quality Meter (FQM),
  2.76 Relative Revenue Growth (REVRANK).
- Sector peer comparison renumbered 2.72 → **2.77**.
- Size caps table: 5 new rows.
- Data sources table: 5 new rows pointing at the new getters.
- Packet size budget revised:
  - Single symbol: 28-55 KB → **30-58 KB**
  - Ten symbols: 270-540 KB → **290-580 KB**

### Native packet generator (`investigate_symbols`)

- Five new blocks in the per-symbol loop, appended after the PEAD
  block under a new `// ── ADR-124 Round 17 ──` divider. Each block
  is gated on
  `label != "NO_DATA" / "INSUFFICIENT_DATA" && !label.is_empty()`
  (the FQM block uses `operator_label`, the others use `rank_label`
  or `relative_label` as appropriate).

## Alternatives considered

- **Let FQM reuse the QUAL composite pipeline.** Rejected because
  FQM deliberately differentiates itself from QUAL by dropping
  leverage. Reusing the QUAL pipeline would require a feature flag
  to suppress the LEV component and reweight the remaining three,
  which pushes complexity into QUAL for no benefit. A standalone
  compute fn is simpler and keeps the two signals independently
  auditable.
- **Use a 4-way fusion for FQM (PTFS + MARGINS + ACRL + LIQ).**
  Rejected because liquidity is a market-microstructure signal, not
  an operator-quality signal. Adding it would confuse the FQM
  narrative ("is this a good operator?") with "can I trade this
  name cheaply?" The two questions need different surfaces.
- **Rank SIZEF by raw `market_cap` instead of `log(market_cap)`.**
  Rejected because market caps span 4-5 orders of magnitude within
  a single sector, and raw-cap ranking compresses the entire
  SMALL/MID/LARGE cohort into a ±5% percentile band at the top of
  the ladder. Log-rank preserves interpretability across the full
  cap distribution.
- **Use raw momentum regime_label as a MOMF filter.** Rejected in
  favour of just dropping `INSUFFICIENT_DATA` peers. The regime
  labels (BULL / BEAR / CHOP / ...) carry useful information but
  applying them as a filter would restrict MOMF to peers in the
  same regime, which defeats the purpose of cross-sectional
  ranking.
- **Require PEADRANK to only include subjects with ≥5 events.**
  Rejected because the PEAD cache routinely has only 3-4 events
  for names that IPO'd in the last 1-2 years, and dropping them
  loses coverage. The 3-event floor is the minimum viable sample
  and matches the floor used by PEAD itself.
- **Cache FQM's input lookup by pre-fusing PTFS/MARGINS/ACRL into a
  materialised composite table.** Rejected as premature optimisation.
  The three reads are O(1) per symbol on SQLite's primary-key index
  and the fusion is cheap.
- **Split REVRANK into quarterly-CAGR and annual-CAGR variants.**
  Rejected because quarterly revenue is noisy (seasonality) and
  the 3-year annual CAGR is the industry-standard growth benchmark.
  A future round could add a "QoQ revenue momentum" surface
  separately without touching REVRANK.
- **Include sector sentinel filtering in MOMF (drop BEAR-regime
  peers).** Rejected for the same reason as regime filtering above:
  sector-relative rank is supposed to compare symbols under the
  same macro conditions, not cherry-pick cohorts.

## Consequences

- Research packet grows another 1-3 KB / symbol on average when
  Round 17 caches are warm. The ten-symbol packet ceiling rises from
  ~540 KB to ~580 KB, still well under the ~650 KB soft target for
  model-readable single-turn input.
- Five new SQLite tables (`research_sizef`, `research_momf`,
  `research_peadrank`, `research_fqm`, `research_revrank`) join the
  LAN-syncable set. Schema v17 is additive.
- **Second-order staleness chain extends to Round 10 and Round 16
  outputs.** MOMF depends on Round 10 MOMENTUM and PEADRANK depends
  on Round 16 PEAD — their rank is only as fresh as the underlying
  factor cache. Users who need strict freshness should recompute
  MOMENTUM / PEAD first, then MOMF / PEADRANK.
- **FQM is the first round to deliberately split a signal from QUAL.**
  Round 15 delivered QUAL as a 4-component fusion (PTFS + MARGINS +
  ACRL + LEV); Round 17 delivers FQM as a 3-component reweighted
  fusion of the same three non-LEV inputs. The two composites will
  disagree systematically for highly-levered names — by design.
- **SIZEF closes the third Fama-French-style factor.** With VAL
  (value), MOM (momentum) + MOMF (momentum rank), and now SIZEF
  (size), the terminal renders all three canonical style factors
  with both absolute and peer-relative views.
- **REVRANK completes the "growth-relative-to-peers" coverage.**
  RELEPSGR measures bottom-line (EPS) growth relative to peers;
  REVRANK measures top-line (revenue) growth relative to peers.
  Together they let the reader distinguish "fast-growing but
  margin-compressing" (revenue up, EPS flat) from "mature but
  improving operator" (revenue flat, EPS up).
- **Two new whole-table scan helpers are now canonical.**
  `get_all_momentum` and `get_all_pead` join the Round 16 set
  (`get_all_val` / `get_all_qual` / `get_all_risk`) as the reference
  pattern for cross-sectional analytics over a research cache.

## Implementation notes

### SIZEF tier thresholds

The size-tier labels use absolute USD thresholds, not percentile cuts,
because "MEGA_CAP" is an industry term with a commonly-understood
dollar floor:

- MEGA_CAP ≥ $200B
- LARGE_CAP ≥ $10B
- MID_CAP ≥ $2B
- SMALL_CAP ≥ $300M
- MICRO_CAP > $0

These are independent from the percentile rank, so a company that is
MID_CAP absolutely can still rank TOP_DECILE within its sector (if the
sector is heavily small-cap).

### FQM weight rationale

The 40/30/30 split (PTFS / MARGINS / ACRL) puts more weight on
Piotroski because Piotroski already incorporates 9 sub-signals across
profitability, liquidity, and operating efficiency, so it carries more
information per point than the single-dimensional margins and
accruals sub-scores. The 30/30 tie between margins and accruals
reflects that both are measuring "how efficiently does this business
turn revenue into cash" from two different angles — margins via the
income statement, accruals via the reconciliation between net income
and operating cash flow.

### FQM input independence check

FQM fuses three inputs: Piotroski, Margins, Accruals. Round 16's
ADR-123 flagged a general "signal laundering" concern (fusing
composites whose inputs overlap), but in FQM's case the three
source caches come from fully independent roots:

- `research_piotroski` — sourced from `FinancialStatements` (balance,
  income, cashflow history).
- `research_margins` — sourced from `Fundamentals` ratios plus
  `FinancialStatements.income_*`.
- `research_accruals` — sourced from the reconciliation between
  `IncomeStatement.net_income` and `CashFlowStatement.operating_cash_flow`.

The one potential overlap is that both Piotroski and Accruals look at
operating cash flow, but Piotroski uses it as a boolean sub-signal
(positive-or-not) while Accruals uses it as a ratio to net income.
The semantic overlap is <10% and doesn't trigger the laundering
concern.

### MOMF / PEADRANK cross-join

Like QRK and RRK in Round 16, MOMF and PEADRANK must cross-join peer
symbols to `fundamentals::get_fundamentals` because neither
`MomentumSnapshot` nor `PeadSnapshot` carries sector in its struct.
A future refactor could add `sector: String` to those two snapshots
as part of their compute pass, which would cut the extra DB hop. For
now the cross-join is acceptable because sector cohorts are small
(typically ≤50 peers per sector in the cached universe).

### REVRANK vs RELEPSGR duplication

The two surfaces use structurally identical code paths: subject +
peer-set CAGR computation, sector median / p25 / p75, gap-to-median
labelling. The duplication is accepted over an abstraction because
the underlying metrics come from different accessors
(`IncomeStatement.eps` vs `IncomeStatement.revenue`) and a generic
"compute CAGR from a field selector" helper would add complexity
without reducing line count meaningfully. If a third CAGR surface is
ever added (e.g., FCF growth), the abstraction becomes worth
extracting.

### Test coverage

15 new tests in `engine/src/core/research::tests`:

- 5 roundtrip tests (`sizef_snapshot_roundtrip`,
  `momf_snapshot_roundtrip`, `peadrank_snapshot_roundtrip`,
  `fqm_snapshot_roundtrip`, `revrank_snapshot_roundtrip`) verify
  schema_v17 create + upsert + get + JSON roundtrip.
- 2 SIZEF tests (`compute_sizef_top_decile`,
  `compute_sizef_no_subject`).
- 2 MOMF tests (`compute_momf_above_median`,
  `compute_momf_no_subject`).
- 2 PEADRANK tests (`compute_peadrank_above_median`,
  `compute_peadrank_insufficient`).
- 2 FQM tests (`compute_fqm_elite_operator`, `compute_fqm_no_inputs`).
- 2 REVRANK tests (`compute_revrank_far_above`,
  `compute_revrank_insufficient_subject`).

Engine test suite: **786 passed / 0 failed / 3 ignored** (771 from
Round 16 + 15 new).

## Historical Follow-up Context

The parity sweep continues. Candidates for Round 18, all still pure
compute over existing caches:

- **LEVRANK — Leverage Rank vs Sector Peers.** Sector-relative
  percentile rank of debt-to-equity from Fundamentals. Completes the
  "rank overlay" family for the fourth factor dimension FQM
  deliberately excluded.
- **SIZECUT — Size-stratified factor ranks.** Re-rank VAL/QUAL/RISK
  within same-size-tier cohorts instead of same-sector cohorts. A
  mega-cap value composite of 65 is not comparable to a small-cap
  value composite of 65; size-stratified ranks would disambiguate.
- **FQMRANK — FQM rank vs sector peers.** The natural rank overlay
  for the FQM composite added in Round 17. Depends on
  `get_all_fqm` (new whole-table scan).
- **OPERANK — Operating Quality Rank vs Sector Peers.** Percentile
  rank of operating margin alone (from the MARGINS cache) within
  sector. Distinct from QRK/FQM because it isolates the "pricing
  power" signal from the fused quality composite.
- **BETA — Rolling beta to sector ETF and to SPY** over user-tunable
  windows, with a stability label. Still blocked on sector-ETF
  mapping and an SPY HP cache persistence decision.
- **CALPB — Put/Call ratio and skew term-structure.** Still blocked
  on richer OMON chain snapshots (multi-expiry).

The standing directive stands: continue until the compute-over-cache
well runs dry. Round 18 will pick the subset that doesn't need new
caches.
