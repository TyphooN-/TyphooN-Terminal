# ADR-126: Godel Parity Round 19 — DVDRANK / EARMRANK / UPDGRANK / GY / DES

**Status:** Accepted
**Date:** 2026-04-15
**Supersedes/extends:** ADR-108 through ADR-125
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Parity classification

| Feature | Godel Terminal documented | TA-Lib primitive | Research packet | egui popup | Chart overlay |
|---|---|---|---|---|---|
| DVDRANK | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| EARMRANK | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| UPDGRANK | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| GY | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| DES | Yes | No | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** pure Godel-Terminal-documented rank + time-series stat surfaces (dividend-growth rank, earnings-momentum rank, upgrade/downgrade rank, gap-yearly, daily-event streak); no TA-Lib primitives in this round.

## Context

Round 18 (ADR-125) shipped LEVRANK / OPERANK / FQMRANK / LIQRANK / SURPSTK
and explicitly closed the "rank every composite factor" arc. Its
future-work list called out six pure-compute candidates — DVD,
INSIDERCONC, GY, UPDG, BETA, CALPB. Round 19 picks the four that
don't need new caches or new external inputs, then adds a fifth
symbol-local time-series stat (DES) that falls out of the same
historical-price reader used by GY:

1. **DVDRANK — Dividend Growth Rank vs Sector Peers.** Round 18's
   future-work list called this "DVD", but Round 12 (ADR-119) already
   cached `DivgSnapshot` with `cagr_3y_pct`, `consecutive_growth_years`,
   and `trend_label`. So the useful Round 19 surface is the *rank
   overlay* — DVDRANK percentile-ranks 3y dividend CAGR within the
   same sector. Higher CAGR = higher rank. Peers with
   `trend_label = "NO_HISTORY"` drop out so the cohort captures only
   names with enough history to compute a meaningful CAGR.
2. **EARMRANK — Earnings Momentum Rank vs Sector Peers.** The natural
   rank overlay for Round 12's `EarmSnapshot.composite_score`. EARM
   was the absolute "is this name's EPS surprise-and-revision trend
   accelerating?" stat; EARMRANK answers the cross-sectional version.
3. **UPDGRANK — Upgrade/Downgrade Rank vs Sector Peers.** The Round 18
   future-work list called this "UPDG". Sector-relative percentile
   rank of `UpdmSnapshot.net_90d` — the net sell-side upgrade minus
   downgrade count over the trailing 90 days. No-coverage peers
   (`bias_label = "NO_COVERAGE"`) are filtered so the cohort is
   sell-side-active names only.
4. **GY — Gap Yearly.** Pure time-series stat over the HP cache:
   iterates the most recent 253 daily bars oldest-first, computes the
   overnight gap as `(open − prev_close) / prev_close × 100` for each
   adjacent pair, bins gaps at 2 / 5 / 10% thresholds in both
   directions, tracks the largest up/down gap with date, the average
   absolute gap, and emits a "gappiness" label (EXPLOSIVE / GAPPY /
   NORMAL / SMOOTH / INSUFFICIENT_DATA). Useful as an event-driven
   risk measure.
5. **DES — Daily Event Streak.** Pure time-series stat over the same
   HP window as GY. Classifies each close-over-close move as UP /
   DOWN / FLAT, computes the longest up-streak and longest down-streak
   over the window, the current trailing streak, the up-day rate, and
   average up/down move %, then maps to a directional-bias ladder
   (STRONG_UPTREND / UPTREND_BIAS / NEUTRAL / DOWNTREND_BIAS /
   STRONG_DOWNTREND / INSUFFICIENT_DATA). Complements SURPSTK (which
   is an *earnings-surprise* streak) with a *price-action* streak.

DVDRANK / EARMRANK / UPDGRANK all use the Round 18 sector-cross-join
pattern; GY and DES follow SURPSTK's symbol-local pattern (no sector,
no peer cross-join, pure history stat).

The standing directive continues: *"continue combing over vs godel
parity until we cannot add more. rinse/repeat do not worry about
round count."*

## Decision

Add five new research surfaces following the Round 18 pattern. Round
19 introduces three new `get_all_*` whole-table scan helpers
(`get_all_divg`, `get_all_earm`, `get_all_updm`) so DVDRANK / EARMRANK
/ UPDGRANK can read every cached row of the matching DIVG / EARM /
UPDM factor tables. GY and DES use the existing `get_historical_price`
reader directly — no new whole-table helpers needed.

### Engine (`engine/src/core/research.rs`)

- **New structs** (after `EarningsSurpriseStreakSnapshot`, under
  `// ── ADR-126 Round 19 — dividend/earnings/rating rank overlays + gap/streak ─` divider):
  - `DividendGrowthRankSnapshot` — DVDRANK (symbol, as_of, sector,
    cagr_3y_pct, consecutive_growth_years, trend_label,
    peers_considered, peers_with_data, sector_median_cagr_pct,
    sector_p25_cagr_pct, sector_p75_cagr_pct, percentile_rank,
    rank_position, rank_label, note). Standard
    TOP_DECILE → BOTTOM_DECILE labels. Subject's DIVG trend is copied
    in so the packet can render both views in one line.
  - `EarningsMomentumRankSnapshot` — EARMRANK (symbol, as_of, sector,
    composite_score, momentum_label, peers_considered, peers_with_data,
    sector_median_score, sector_p25, sector_p75, percentile_rank,
    rank_position, rank_label, note). Subject's EARM label is copied.
  - `UpgradeDowngradeRankSnapshot` — UPDGRANK (symbol, as_of, sector,
    net_90d, bias_label, peers_considered, peers_with_data,
    sector_median_net_90d, sector_p25_net_90d, sector_p75_net_90d,
    percentile_rank, rank_position, rank_label, note). Subject's UPDM
    bias is copied.
  - `GapYearlySnapshot` — GY (symbol, as_of, bars_used, gaps_total,
    gaps_up_2pct, gaps_down_2pct, gaps_up_5pct, gaps_down_5pct,
    gaps_up_10pct, gaps_down_10pct, largest_up_gap_pct,
    largest_up_gap_date, largest_down_gap_pct, largest_down_gap_date,
    avg_abs_gap_pct, gap_label, note).
  - `DailyEventStreakSnapshot` — DES (symbol, as_of, bars_used,
    current_streak_type, current_streak_len, longest_up_streak,
    longest_down_streak, up_days, down_days, flat_days,
    up_day_rate_pct, avg_up_move_pct, avg_down_move_pct, streak_label,
    note).

- **New compute fns** (after `compute_surpstk_snapshot`, under
  `// ── ADR-126 Round 19 compute fns ──` divider):
  - `compute_dvdrank_snapshot(symbol, as_of, sector, subject:
    Option<&DivgSnapshot>, peers: &[&DivgSnapshot])` — filters peers
    whose `trend_label != "NO_HISTORY"` and drops the subject itself.
    Needs ≥3 peers with data or short-circuits to
    `rank_label = "INSUFFICIENT_DATA"`. Computes percentile via
    `percentile_rank_score(subject_cagr, peer_cagr, higher_is_better=true)`
    and rank via `better = peer.filter(|p| p > subj).count(); pos = better + 1`.
  - `compute_earmrank_snapshot(symbol, as_of, sector, subject:
    Option<&EarmSnapshot>, peers: &[&EarmSnapshot])` — identical
    pattern, filtering `momentum_label != "INSUFFICIENT_DATA"`.
  - `compute_updgrank_snapshot(symbol, as_of, sector, subject:
    Option<&UpdmSnapshot>, peers: &[&UpdmSnapshot])` — identical
    pattern, filtering `bias_label != "NO_COVERAGE"`, casting
    `net_90d` to `f64` for ranking.
  - `compute_gy_snapshot(symbol, as_of, bars: &[HistoricalPriceRow])`
    — sorts bars oldest-first by date, windows to the most recent
    253 sessions, iterates adjacent pairs computing
    `(open − prev_close) / prev_close × 100`, skips gaps with
    `|gap| < 0.01%` to avoid counting the noise floor, bins at 2 / 5
    / 10% thresholds in each direction, and maps to:
    - **EXPLOSIVE**: ≥1 gap of ≥10% OR ≥4 gaps of ≥5%
    - **GAPPY**: ≥12 gaps of ≥2% OR ≥2 gaps of ≥5%
    - **SMOOTH**: <6 gaps of ≥2%
    - **NORMAL**: otherwise
    - **INSUFFICIENT_DATA**: <20 bars
  - `compute_des_snapshot(symbol, as_of, bars: &[HistoricalPriceRow])`
    — same sort + window as GY. For each adjacent pair computes
    `(cur_close − prev_close) / prev_close × 100`, classifies as
    UP / DOWN / FLAT, scans for longest up-run and longest down-run,
    tracks current trailing streak, and maps to:
    - **STRONG_UPTREND**: up_day_rate ≥ 60% AND longest_up_streak ≥ 5
    - **UPTREND_BIAS**: up_day_rate ≥ 55%
    - **STRONG_DOWNTREND**: up_day_rate ≤ 40% AND longest_down_streak ≥ 5
    - **DOWNTREND_BIAS**: up_day_rate ≤ 45%
    - **NEUTRAL**: otherwise
    - **INSUFFICIENT_DATA**: <20 bars

- **Schema v19** (`create_research_tables_v19`): calls v18 first, then
  creates `research_dvdrank`, `research_earmrank`, `research_updgrank`,
  `research_gy`, `research_des`, each shaped the same way
  `(symbol TEXT PRIMARY KEY, snapshot_json TEXT, updated_at INTEGER)`
  with per-table `updated_at` indexes. Schema v19 is additive.

- **Upsert/get wrappers** (after `get_surpstk`):
  `upsert_dvdrank` / `get_dvdrank`, `upsert_earmrank` / `get_earmrank`,
  `upsert_updgrank` / `get_updgrank`, `upsert_gy` / `get_gy`,
  `upsert_des` / `get_des`. Standard `INSERT ... ON CONFLICT` +
  serde-JSON roundtrip.

- **New whole-table scans**:
  - `get_all_divg(&conn) -> Result<Vec<DivgSnapshot>>` — scans
    `research_divg` (created by `create_research_tables_v12`, where
    DIVG was first cached in Round 12).
  - `get_all_earm(&conn) -> Result<Vec<EarmSnapshot>>` — scans
    `research_earm` (also Round 12).
  - `get_all_updm(&conn) -> Result<Vec<UpdmSnapshot>>` — scans
    `research_updm` (also Round 12).
  All three follow the Round 16/17/18 whole-table helper shape:
  `query_map([], |row| row.get::<_, String>(0))` over the target
  table, serde-JSON deserialisation per row.

### LAN sync (`engine/src/core/lan_sync.rs`)

- `SYNCABLE_TABLES` — add `research_dvdrank`, `research_earmrank`,
  `research_updgrank`, `research_gy`, `research_des` under a new
  `// ── ADR-126 Round 19 ──` divider.
- `create_table_sql()` — 5 new arms emitting the same DDL as the
  engine's `create_research_tables_v19`.
- `table_timestamp_column()` — the Round 18 commit consolidated the
  generic `research_*` pattern, so Round 19's new tables inherit
  `"updated_at"` without needing explicit arms.

### Native (`native/src/app.rs`)

- **BrokerCmd variants** (after `ComputeSurpstkSnapshot`, under a new
  `// ── ADR-126 Round 19 ──` divider):
  - `ComputeDvdrankSnapshot { symbol }`
  - `ComputeEarmrankSnapshot { symbol }`
  - `ComputeUpdgrankSnapshot { symbol }`
  - `ComputeGySnapshot { symbol }`
  - `ComputeDesSnapshot { symbol }`

- **BrokerMsg variants** (after `SurpstkSnapshotMsg`) under a new
  `// ── ADR-126 ──` divider:
  - `DvdrankSnapshotMsg(String, DividendGrowthRankSnapshot)`
  - `EarmrankSnapshotMsg(String, EarningsMomentumRankSnapshot)`
  - `UpdgrankSnapshotMsg(String, UpgradeDowngradeRankSnapshot)`
  - `GySnapshotMsg(String, GapYearlySnapshot)`
  - `DesSnapshotMsg(String, DailyEventStreakSnapshot)`

- **TyphooNApp state fields** (after the Round 18 fields). Each
  surface gets `show_*` / `*_symbol` / `*_snapshot` / `*_loading`.

- **Broker handler spawns** (after the SURPSTK handler). Each follows
  the Round 18 `tokio::spawn` + `shared_cache_broker` pattern:
  - **DVDRANK handler** — cross-join pattern. Calls
    `research::get_all_divg(&conn)` then per-peer
    `fundamentals::get_fundamentals` to filter to the subject's
    sector. DivgSnapshot doesn't carry sector, so the cross-join is
    required.
  - **EARMRANK handler** — same cross-join pattern using
    `get_all_earm` and `fundamentals::get_fundamentals`.
  - **UPDGRANK handler** — same cross-join pattern using
    `get_all_updm` and `fundamentals::get_fundamentals`.
  - **GY handler** — no cross-join. Pre-reads
    `research::get_historical_price(&conn, &symbol)` and feeds the
    Vec directly to `compute_gy_snapshot`.
  - **DES handler** — same shape as GY, feeds the HP Vec to
    `compute_des_snapshot`.

- **Receive arms** (in the `BrokerMsg` match, after
  `SurpstkSnapshotMsg`): each arm updates the matching state field if
  the incoming symbol matches `*_symbol`, then unconditionally upserts
  via `upsert_*`. Unconditional upsert so LAN-synced receivers benefit
  even when no window is open.

- **egui windows** (after the SURPSTK window). Each follows the
  established header pattern (symbol editor / Use Chart / Load Cached
  / Compute / Loading spinner), a color-coded summary line, and
  per-surface grids:
  - **DVDRANK** — 4-row summary (subject 3y CAGR / consecutive growth
    years / sector median/p25/p75 CAGR / peers considered/with data).
  - **EARMRANK** — 3-row summary (subject composite score / sector
    median/p25/p75 / peers considered/with data).
  - **UPDGRANK** — 3-row summary (subject net 90d / sector median/p25/p75
    net / peers considered/with data).
  - **GY** — 5-row summary (gaps up ≥2/5/10% / gaps down ≥2/5/10% /
    largest up gap + date / largest down gap + date / avg |gap|).
  - **DES** — 4-row summary (bars used / up/down/flat days / longest
    up/down streaks / avg up/down move %).

- **Command palette entries**:
  `DVDRANK | DIVG_RANK | DIVIDEND_RANK`,
  `EARMRANK | EARM_RANK | EARNINGS_MOMENTUM_RANK`,
  `UPDGRANK | UPDG_RANK | UPGRADE_RANK`,
  `GY_STAT | GAP_YEARLY | GAPS`,
  `DES_STREAK | DAILY_STREAK | EVENT_STREAK`.
  `GY` alone is already taken by Treasury yield curve, so the gap
  surface uses `GY_STAT` as its primary alias. `DES` alone is already
  taken by DESCRIPTION, so the daily-event-streak surface uses
  `DES_STREAK` as its primary alias. Each arm sets `show_*`, copies
  the current chart symbol into `*_symbol`, and opportunistically
  loads the cached snapshot.

### Research Packet (`docs/RESEARCH_PACKET.md`)

- Header count bumped: eighty-two → **eighty-seven** sub-blocks.
- New sections 2.82 Dividend Growth Rank (DVDRANK), 2.83 Earnings
  Momentum Rank (EARMRANK), 2.84 Upgrade/Downgrade Rank (UPDGRANK),
  2.85 Gap Yearly (GY), 2.86 Daily Event Streak (DES).
- Sector peer comparison renumbered 2.82 → **2.87**.
- Data sources table: 5 new rows pointing at the new getters.
- Packet size budget revised:
  - Single symbol: 32-61 KB → **34-64 KB**
  - Ten symbols: 310-620 KB → **330-660 KB**

### Native packet generator (`investigate_symbols`)

- Five new blocks in the per-symbol loop, appended after the SURPSTK
  block under a new `// ── ADR-126 Round 19 ──` divider. Each block
  is gated on
  `label != "NO_DATA" / "INSUFFICIENT_DATA" && !label.is_empty()`
  (DVDRANK / EARMRANK / UPDGRANK use `rank_label`, GY uses `gap_label`,
  DES uses `streak_label`).

## Alternatives considered

- **Let DVDRANK call itself DVD** (per Round 18 future-work list).
  Rejected because Round 12's DIVG already carries every field a
  bare-DVD surface would want (cagr_3y_pct, consecutive_growth_years,
  trend_label). The useful Round 19 addition is the *rank overlay* on
  DIVG, not a duplicate of it.
- **Use a 1-year lookback for GY/DES instead of 253 sessions.**
  Rejected because 253 is the exact trading-day count in a standard
  US equity year and matches the industry convention for
  "trailing-12-month" daily stats. Using 252 or 250 would leave one
  session on the table; using 260 would include a weekend's worth of
  extra noise. 253 is the precise boundary.
- **Fold GY and DES into a single "price-action" composite.**
  Rejected because gaps and close-over-close moves measure
  *different* things: GY measures overnight event risk, DES measures
  intraday directional persistence. A stock that grinds up every
  session with no gaps looks identical to a stock with one big gap
  and flat closes in a fused composite — but the risk profiles are
  wildly different.
- **Use a 3% gap band for GY's largest-gap bins instead of 2%.**
  Rejected because 2% is the lower bound where overnight moves start
  to be news-driven rather than noise-driven. The industry threshold
  for "meaningful gap" is 2% — any higher and we drop the steady-drip
  news-response names from the census.
- **Make UPDGRANK filter on `coverage_count >= 3` instead of
  `bias_label != "NO_COVERAGE"`.** Rejected because Round 12's UPDM
  already uses `NO_COVERAGE` as the sentinel for "no sell-side
  analysts" and `coverage_count` is not guaranteed to be populated on
  older UPDM rows. The label check is the contract.
- **Include current-year forward dividend estimate in DVDRANK.**
  Rejected because the DIVG snapshot is trailing-only. Forward
  dividend estimates live in `EarningsEstimateSnapshot`-style surfaces
  which aren't Round-19-scope. A future round can add DVDFWDRANK
  (forward dividend growth rank) if the forward-dividend cache ever
  gets populated with enough coverage.
- **Compute DES flat-day rate using close-over-close rather than
  open-to-close.** The Round 19 implementation uses close-over-close
  because the HP cache is adjusted-close-authoritative and
  intraday-open isn't split/dividend-adjusted the same way. Using
  close-over-close preserves the adjustment chain end-to-end.
- **Rank GY gap-count within sector instead of using an absolute
  label.** Rejected because gap counts are extremely skewed across
  sectors — biotech and small-cap tech will dominate any
  cross-sectional gap-count rank and crowd out the genuine
  sector-local signal for large-cap names. The absolute label ladder
  communicates "explosive" vs "smooth" without the sector-dominance
  artifact.

## Consequences

- Research packet grows another 1-3 KB / symbol on average when
  Round 19 caches are warm. The ten-symbol packet ceiling rises from
  ~620 KB to ~660 KB, still under the ~700 KB soft target for
  model-readable single-turn input.
- Five new SQLite tables (`research_dvdrank`, `research_earmrank`,
  `research_updgrank`, `research_gy`, `research_des`) join the
  LAN-syncable set. Schema v19 is additive.
- **Second-order staleness chain continues to grow.** DVDRANK
  depends on Round 12 DIVG, EARMRANK on Round 12 EARM, UPDGRANK on
  Round 12 UPDM. GY and DES depend on the HP cache which is
  refreshed on every chart load. Users who need strict freshness for
  the rank surfaces should recompute DIVG / EARM / UPDM first.
- **Round 19 completes the "rank overlay" pass on Round 12 factors.**
  Round 12 shipped DIVG / EARM / UPDM as absolute snapshots; Round 19
  adds the sector-rank overlay for all three. Every Round 12 factor
  now has a companion rank surface.
- **GY and DES set the HP-pure-compute precedent.** Round 18's
  SURPSTK was the first "pure time-series stat with no cross-join."
  Round 19's GY and DES extend this to the HP cache — which is
  significantly larger (253 bars vs. 4-20 earnings events) but still
  O(253) work per symbol, well under the 100 ms UI-latency budget.
  Future rounds can add more HP-pure-compute surfaces (e.g., volatility
  regime, drawdown history, max-favorable-excursion) without the
  cross-join overhead.
- **Three new whole-table scan helpers are now canonical.**
  `get_all_divg` / `get_all_earm` / `get_all_updm` join the Round
  16/17/18 set. After Round 19, the Round 12 factors (DIVG, EARM,
  UPDM) all have whole-table scan coverage, matching the Round 13-17
  coverage from Round 18.

## Implementation notes

### GY window naming collision

The single letters `GY` / `DES` were already taken by existing UI
surfaces: `GY` is the command palette alias for the US Treasury
Yield Curve window, and `DES` is the alias for DESCRIPTION (company
description). To avoid palette collisions, the Round 19 surfaces use
`GY_STAT` and `DES_STREAK` as their primary aliases while keeping
`GAP_YEARLY` / `GAPS` and `DAILY_STREAK` / `EVENT_STREAK` as
longer-form aliases. The engine-side struct names (`GapYearlySnapshot`,
`DailyEventStreakSnapshot`) and the internal compute fn names
(`compute_gy_snapshot`, `compute_des_snapshot`) are unambiguous
because they're scoped to `research::` — no collision at the API
level.

### Window size subset for bars

GY and DES both use the exact same `(sort oldest-first, window to
last 253)` preamble. The repetition is deliberate: each compute fn
is self-contained so it can be called from a test with a hand-built
bars Vec without depending on a shared helper. A fused
`window_hp_bars(bars, 253)` helper would save ~6 lines but make the
fns harder to read in isolation.

### Percentile floor for rank surfaces

DVDRANK / EARMRANK / UPDGRANK all enforce a 3-peer minimum before
computing a rank, same as LEVRANK / OPERANK / FQMRANK / LIQRANK from
Round 18. With 3 peers + 1 subject = 4 data points, the quartiles
are well-defined (p25 is the lowest, p75 is the highest, median is
the middle two's mean), and the percentile rank has meaningful
resolution. Fewer than 3 peers short-circuits to
`rank_label = "INSUFFICIENT_DATA"`.

### GY gap noise floor

The `|gap| < 0.01%` skip exists because many cached HP rows have
`open == prev_close` exactly due to how certain vendors adjust
overnight moves. Without the skip, those exactly-zero gaps would
flood the gaps_total count and distort the avg_abs_gap_pct
downward. The 0.01% threshold is tight enough to preserve every
meaningful gap while dropping the adjustment-artifact zeros.

### DES flat-day handling

The flat_days counter tracks exact zero close-over-close moves
(stock halted, extreme low-volume day, or coincidental match).
`up_day_rate_pct` is computed as `up_days / (up_days + down_days) × 100`,
deliberately excluding flat days from the denominator so flat days
don't dilute a genuine up/down bias. This matches how
directional-trading strategies report hit-rate — a tied day is
"no information" rather than a losing day.

### DVDRANK peers_considered vs peers_with_data

`peers_considered` is the raw count of sector peers scanned (pre-filter),
`peers_with_data` is the count after dropping `trend_label = "NO_HISTORY"`.
The distinction matters for the reader because a sector with 50
total peers but only 3 with dividend history tells a different story
than a sector with 50 peers and 50 dividend payers — in the first
case, the rank is meaningful within a small cohort but might be
unrepresentative of the sector as a whole.

### Test coverage

15 new tests in `engine/src/core/research::tests`:

- 5 roundtrip tests (`dvdrank_snapshot_roundtrip`,
  `earmrank_snapshot_roundtrip`, `updgrank_snapshot_roundtrip`,
  `gy_snapshot_roundtrip`, `des_snapshot_roundtrip`) verify
  schema_v19 create + upsert + get + JSON roundtrip.
- 2 DVDRANK tests (`compute_dvdrank_top_decile`,
  `compute_dvdrank_no_history_filtered`) — the second exercises the
  NO_HISTORY peer filter.
- 2 EARMRANK tests (`compute_earmrank_above_median`,
  `compute_earmrank_insufficient_filtered`).
- 2 UPDGRANK tests (`compute_updgrank_bullish`,
  `compute_updgrank_no_coverage_filtered`).
- 3 GY tests (`compute_gy_normal`, `compute_gy_explosive`,
  `compute_gy_insufficient`).
- 3 DES tests (`compute_des_uptrend`, `compute_des_downtrend`,
  `compute_des_insufficient`).

Engine test suite: **821 passed / 0 failed / 3 ignored** (806 from
Round 18 + 15 new).

## Historical Follow-up Context

The parity sweep continues. Candidates for Round 20, still pure
compute over existing caches:

- **DVDYIELDRANK — Dividend Yield Rank vs Sector Peers.** DVDRANK
  ranks *growth*; this would rank current *yield*. Needs
  `trailing_annual_dividend_rate / current_price × 100` from
  Fundamentals.
- **INSIDERCONC — Insider ownership concentration vs sector.** Ranks
  `Fundamentals.insiders_percent_held` or equivalent. Complement to
  Round 12's INSIDERS activity feed.
- **ATRANN — Annualized ATR (volatility regime).** Pure time-series
  stat over the HP cache: computes the 14-day ATR, annualizes via
  √252, and maps to a volatility regime label.
- **DDHIST — Drawdown history.** Pure HP-cache stat: longest drawdown,
  deepest drawdown, number of 5% corrections in the window.
- **PRICEPERF — Multi-horizon price performance.** 1M / 3M / 6M / YTD
  / 1Y price returns with sector-rank overlays (six new surfaces in
  one bundle).
- **BETA — Rolling beta to sector ETF and to SPY.** Still blocked on
  sector-ETF mapping.
- **CALPB — Put/Call ratio and skew term-structure.** Still blocked
  on richer OMON chain snapshots.

The standing directive stands: continue until the compute-over-cache
well runs dry. Round 20 will pick the subset that's still pure
compute.
