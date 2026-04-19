# ADR-119: Godel Parity Round 12 — MNGR / DIVG / EARM / SECTR / UPDM

**Status:** Accepted
**Date:** 2026-04-14
**Supersedes/extends:** ADR-108, ADR-109, ADR-110, ADR-111, ADR-112, ADR-113, ADR-114, ADR-115, ADR-116, ADR-117, ADR-118
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Parity classification

| Feature | Godel documented? | TA-Lib primitive? | Research packet | egui popup | Chart overlay |
| --- | --- | --- | --- | --- | --- |
| MNGR (insider activity bias) | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| DIVG (dividend growth analysis) | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| EARM (earnings momentum trend) | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| SECTR (sector rotation strength) | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| UPDM (upgrade/downgrade momentum) | Yes | No | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** pure Godel-Terminal-documented composite surfaces (insider bias, dividend growth, earnings momentum, sector rotation, analyst rotation); no TA-Lib primitives in this round.

## Context

Round 11 (ADR-118) closed the "composite risk and analyst consensus" gap
with ALTZ / PTFS / VOLE / EPSB / PTD — Altman Z bankruptcy score,
Piotroski quality score, the full OHLC volatility estimator family,
EPS beat streaks, and price-target dispersion. With those in place the
next visible gaps versus Godel Terminal were the **insider sentiment,
dividend growth trajectory, earnings momentum, sector rotation, and
analyst rotation** panels: Godel surfaces each of these as a standalone
screen, each derived from data TyphooN already caches but never
synthesised into a single label.

Round 12 picks up the five surfaces that close that gap. As with Round
10 and Round 11, **all five are pure compute over existing caches**
(`FA` FinancialStatements, `FA` dividends, `ERN` EarningsSurprise,
`DVD` DividendRecord, `INS` InsiderTrade, `UPDG` RatingChange history,
`INDU` SectorPerformance, Fundamentals) — no new API dependencies:

1. **MNGR — Insider Activity Bias.** Godel's insider panel rolls up
   Form-4 filings over a user-selectable window and labels the net bias
   (BULLISH / NEUTRAL / BEARISH) plus conviction (HIGH / MEDIUM / LOW)
   from unique-insider count and net dollar flow. TyphooN already caches
   `InsiderTrade` rows via ADR-112 INS; MNGR windows them (default 90d)
   and derives buy/sell/other counts, unique insiders, gross and net
   dollar flow, buy/sell ratio, and net shares.
2. **DIVG — Dividend Growth Analysis.** Godel's dividend panel buckets
   historical dividends into calendar-year rows, computes 1Y/3Y/5Y CAGRs,
   consecutive-growth-year counter, consistency %, and a trend label
   (GROWING / STABLE / CUTTING). TyphooN already caches `DividendRecord`
   via ADR-109 DVD; DIVG sorts them, excludes the incomplete current
   year, and runs the math.
3. **EARM — Earnings Momentum Trend.** Godel's earnings panel compares
   the most-recent-4-quarter revenue yoy growth against the prior
   4-quarter yoy growth, then layers the EPS surprise acceleration from
   `EarningsSurprise` on top, and rolls it into a 0-100 composite score
   labelled ACCELERATING / STABLE / DECELERATING. Needs ≥5 quarters of
   cached income statements and cached surprise history.
4. **SECTR — Sector Rotation Strength.** Godel's sector panel ranks the
   symbol's sector among all S&P sectors (cached from `INDU`) and
   derives a relative strength label (LEADER / NEUTRAL / LAGGARD). A
   symbol is a LEADER when its sector is in the top third of all
   sectors AND the sector's change % exceeds the median. TyphooN
   already caches `SectorPerformance` snapshots via ADR-113 INDU.
5. **UPDM — Upgrade/Downgrade Momentum.** Godel's analyst panel
   buckets cached rating changes into 30d / 90d / 180d windows, counts
   upgrades / downgrades / initiations / maintains per window, and
   labels bias (BULLISH / NEUTRAL / BEARISH) plus trend (IMPROVING /
   STABLE / DETERIORATING) from the net 30d vs 90d vs 180d deltas.
   TyphooN already caches `RatingChange` via ADR-109 UPDG.

The standing directive applies: *"continue combing over vs godel parity
until we cannot add more. rinse/repeat do not worry about round count."*

## Decision

Add five new research surfaces following the Round 10 / Round 11 pattern
verbatim:

### Engine (`engine/src/core/research.rs`)

- **New structs** (ADR-119 section near line 1220):
  - `InsiderActivitySnapshot` — flat per-symbol snapshot (window_days,
    total/buy/sell/other counts, unique insiders, gross buy/sell in USD,
    net in USD, buy/sell ratio, net shares, latest trade date,
    bias_label, conviction_label, note).
  - `DivgAnnualRow` + `DivgSnapshot` — one annual row (year, total
    amount, payment count, growth_pct) and the per-symbol wrapper
    (total payments, first/latest payment dates, latest amount,
    annualised dividend, years covered, 1Y/3Y/5Y CAGRs, consecutive
    growth years, consistency score %, annual rows vector, trend label,
    note).
  - `EarmQuarterRow` + `EarmSnapshot` — one quarter row (period,
    revenue, revenue yoy %, EPS actual/estimate/surprise) and the
    per-symbol wrapper (quarters used, recent vs prior revenue growth
    %, revenue acceleration %, recent vs prior EPS surprise %, EPS
    surprise acceleration %, composite score 0-100, momentum label,
    quarters vector, note).
  - `SectorRotationSnapshot` — flat per-symbol snapshot (symbol sector,
    symbol sector change %, sector rank, sectors total, avg / median
    sector change %, relative strength %, breadth %, strongest /
    weakest sector + pct, strength label, note).
  - `UpdmSnapshot` — flat per-symbol snapshot (total actions,
    upgrades/downgrades at 30d/90d/180d, initiations 90d, maintains
    90d, net 30d/90d/180d, latest date/action/firm/to_grade, bias
    label, trend label, note).

- **New compute fns** (ADR-119 block near line 5613):
  - `compute_insider_activity_snapshot(symbol, as_of, trades, window_days)`
    — filters cached `InsiderTrade` rows by `as_of - window_days` using
    a crude julian helper (`parse_yyyy_mm_dd_to_days`). Buys, sells and
    other transactions are bucketed. Unique insiders counted by name.
    Gross buy/sell dollar flow aggregated. Net = gross_buy − gross_sell.
    Bias BULLISH when net > +gross_total·0.1, BEARISH when net <
    −gross_total·0.1, else NEUTRAL. Conviction HIGH when unique_insiders
    ≥ 3 AND |net| > $500k, MEDIUM when either condition alone, LOW when
    neither.
  - `compute_divg_snapshot(symbol, as_of, dividends)` — sorts cached
    `DividendRecord` oldest-first, buckets into calendar years using
    the first-4-char year prefix, skips the incomplete current year
    (year ≥ as_of_year), computes per-year growth %, 1Y/3Y/5Y CAGR
    from the year totals, consecutive_growth_years from the newest-back
    run, consistency % from positive-growth-year count / total-growth-
    year count. Trend GROWING when 3Y CAGR ≥ 5% AND consistency ≥ 60%,
    CUTTING when 3Y CAGR < -5% OR latest < prior annualised × 0.9,
    STABLE otherwise. Annualised dividend = latest_amount × (payments
    in the most recent full year). Returns NO_HISTORY when <1 full year
    of data.
  - `compute_earm_snapshot(symbol, as_of, statements, surprises)` —
    requires `income_quarterly.len() ≥ 5`. Sorts quarters newest-first
    by date. For the most recent 4 quarters computes revenue yoy using
    the quarter 4 positions back. The prior 4 quarters (offset 4..8)
    get the same treatment. `revenue_acceleration_pct = recent - prior`.
    Walks the surprise history newest-first to get recent-4 avg and
    prior-4 avg surprise %. Composite score = 50 + 2·revenue_acc +
    1.5·surprise_acc, clamped to [0, 100]. Label ACCELERATING if score
    ≥ 65, DECELERATING if ≤ 35, STABLE otherwise.
  - `compute_sector_rotation_snapshot(symbol, as_of, symbol_sector, sectors)`
    — returns NO_DATA when symbol_sector is empty OR sectors list is
    empty. Fuzzy-matches `symbol_sector` against `SectorPerformance.sector`
    using a normalised lower-case comparison that ignores punctuation.
    Ranks sectors by change_pct descending. Relative strength =
    `symbol_change - avg_change`. Breadth = positive_sectors / total.
    LEADER when rank is in the top third (rank × 3 < sectors_total) AND
    relative strength > 0. LAGGARD when in the bottom third AND
    relative strength < 0. NEUTRAL otherwise.
  - `compute_updm_snapshot(symbol, as_of, actions)` — filters cached
    `RatingChange` by 30 / 90 / 180 day windows from `as_of`. Classifies
    each action by case-insensitive substring match on the action field:
    "upgrad" / "downgrad" / "initiat" / "maintain". Nets 30d = upgrades
    − downgrades (same for 90d / 180d). Bias BULLISH when net_90d ≥ 2,
    BEARISH when ≤ -2, else NEUTRAL. Trend IMPROVING when net_30d >
    net_90d/3, DETERIORATING when net_30d < -net_90d/3, else STABLE.
    Latest-action fields come from the newest row by date.

- **Schema v12** (`create_research_tables_v12` near line 6900): Creates
  `research_insider_activity`, `research_divg`, `research_earm`,
  `research_sector_rotation`, and `research_updm` — all follow the
  Round 9/10/11 JSON-blob pattern: `(symbol TEXT PRIMARY KEY, snapshot_json TEXT,
  updated_at INTEGER)` with per-table `updated_at` indexes for
  incremental LAN sync.

- **Upsert/get wrappers**: 10 functions total —
  `upsert_insider_activity` / `get_insider_activity`, `upsert_divg` /
  `get_divg`, `upsert_earm` / `get_earm`, `upsert_sector_rotation` /
  `get_sector_rotation`, `upsert_updm` / `get_updm`. All uppercase the
  symbol on write and normalise it on read.

- **Helper** `parse_yyyy_mm_dd_to_days(s)` — crude julian day helper
  `y*372 + m*31 + d` used for windowing and sorting without taking a
  chrono dependency on the pure compute path. Good enough for
  pairwise comparisons within the same year-range; not suitable for
  duration math.

### LAN sync (`engine/src/core/lan_sync.rs`)

- Whitelist the 5 new tables in `SYNCABLE_TABLES` under a Round 12
  marker block after the Round 11 block.
- Add 5 `CREATE TABLE IF NOT EXISTS …` branches in `create_table_sql()`
  so a fresh-peer handshake can materialise empty tables before the
  first bulk sync.
- Add 5 `"table" => Some("updated_at")` mappings in
  `table_timestamp_column()` so incremental sync filters rows by
  timestamp instead of falling back to full sync.

### Native (`native/src/app.rs`)

- **5 new `BrokerCmd` variants** (ADR-119 section after the Round 11
  block):
  - `ComputeInsiderActivitySnapshot { symbol, window_days }` — trades
    loaded inside the handler from `shared_cache_broker.read`. Default
    window 90 days, user-tunable in the window via a drag-value.
  - `ComputeDivgSnapshot { symbol }` — dividends loaded inside the
    handler.
  - `ComputeEarmSnapshot { symbol }` — statements + surprises loaded
    inside the handler.
  - `ComputeSectorRotationSnapshot { symbol, symbol_sector }` — the
    symbol's sector is pre-read from Fundamentals on the main thread;
    the cached sector performance list is loaded inside the handler.
    This keeps the tokio worker Send-safe without juggling two cache
    read locks.
  - `ComputeUpdmSnapshot { symbol }` — rating change history loaded
    inside the handler.

- **5 new `BrokerMsg` variants**: `InsiderActivitySnapshotMsg`,
  `DivgSnapshotMsg`, `EarmSnapshotMsg`, `SectorRotationSnapshotMsg`,
  `UpdmSnapshotMsg` — each carries the uppercase symbol + the typed
  snapshot.

- **5 new state sets** on `TyphooNApp` (19 fields total) — `show_*`,
  `*_symbol`, `*_snapshot`, `*_loading` for mngr / divg / earm / sectr /
  updm. MNGR has an extra `mngr_window_days: i32` field (default 90).
  Defaults wired in the struct literal after the Round 11 block.

- **5 tokio::spawn broker handlers** following the Round 11 pattern:
  MNGR / DIVG / EARM / UPDM load their inputs inside the handler;
  SECTR receives `symbol_sector` pre-read from Fundamentals on the
  main thread.

- **5 receive arms** with upsert-on-receive (main thread): each arm
  matches the current symbol into the window's loading slot and
  persists the snapshot to SQLite via the matching upsert helper.

- **5 egui windows** — MNGR / DIVG / EARM / SECTR / UPDM — each with
  the standard header (Symbol input + Use Chart + Load Cached +
  Compute buttons), a Loading indicator, a symbol/status header line,
  and a table or key-value grid. MNGR also exposes a window-days drag
  so users can scrub 30–365d. Colour coding:
  - MNGR bias: BULLISH = UP, NEUTRAL = AXIS_TEXT, BEARISH = DOWN.
  - DIVG trend: GROWING = UP, STABLE = AXIS_TEXT, CUTTING = DOWN.
  - EARM momentum: ACCELERATING = UP, STABLE = AXIS_TEXT,
    DECELERATING = DOWN.
  - SECTR strength: LEADER = UP, NEUTRAL = AXIS_TEXT, LAGGARD = DOWN.
  - UPDM bias: BULLISH = UP, NEUTRAL = AXIS_TEXT, BEARISH = DOWN.

- **5 command-palette entries** — one collision against a legacy alias
  was caught by `cargo check` and resolved:
  - `MNGR | INSIDER_BIAS | INSIDER_ACTIVITY | INSIDER_SCORE`
  - `DIVG | DIV_GROWTH | DIVIDEND_GROWTH | DIV_CAGR`
  - `EARM | EARN_MOMENTUM | EARNINGS_MOMENTUM | REV_MOMENTUM`
  - `SECTR | SECT_ROT | SECTOR_STRENGTH | RS_SECTOR` — note:
    `SECTOR_ROTATION` was rejected as an alias because it already
    resolves to the legacy `show_sector_rotation` window from an
    earlier round. `SECT_ROT` replaces it.
  - `UPDM | UPGRADE_MOMENTUM | RATING_MOMENTUM | ANALYST_MOMENTUM`

  Each palette branch reads the active chart symbol, opens its window,
  and loads any cached snapshot into view.

### Research packet (`docs/RESEARCH_PACKET.md`)

- Header count bumped "forty-seven" → **"fifty-two sub-blocks"**.
- 5 new sub-block sections (2.47–2.51); prior Sector peer comparison
  block renumbered to 2.52.
- 5 new rows in the size-caps table (MNGR 10 k/v rows, DIVG up to 10
  annual rows, EARM up to 8 quarter rows, SECTR 10 k/v rows, UPDM 12
  k/v rows).
- 5 new rows in the data-source table.
- Packet size estimate: 18–36 KB → **20–40 KB** single symbol; 170–340
  KB → **190–380 KB** 10-symbol basket.
- `investigate_symbols()` in `native/src/app.rs` emits one new markdown
  block per cached Round 12 snapshot, silently skipped when the data
  isn't populated.

## Alternatives considered

- **External API for insider bias** (WhaleWisdom, Openinsider, Insider
  Score). All three ship a pre-rolled score but cost either an API key
  or web-scraping fragility. Rejected — TyphooN already caches the
  underlying Form-4 rows via ADR-112 INS, so compute locally.
- **Using EPSB (Round 11) instead of a new EARM**. Rejected — EPSB is
  a pure EPS beat-rate tracker. EARM adds revenue yoy trajectory
  (which is the headline the market reacts to) and combines it with
  the EPS surprise acceleration into a single composite — a strictly
  richer view that deserves its own surface.
- **Monthly dividend bucketing for DIVG**. Rejected — most US names
  pay quarterly, so monthly buckets inflate row counts without
  adding signal. Calendar-year bucketing mirrors the way dividend
  growth is reported in the wild (1Y/3Y/5Y CAGRs) and keeps the row
  count bounded.
- **Fetching live sector performance inside the broker handler**.
  Rejected — `compute_sector_rotation_snapshot` must stay pure compute
  over cached `INDU` rows so that a packet built mid-session isn't
  blocked on a network round-trip. The user refreshes `INDU` manually
  and then fires `SECTR`.
- **UPDM showing the full rating distribution per action**. Rejected
  for this round — the cached `RatingChange` schema stores one row per
  action, not a full distribution snapshot. The 30/90/180 window
  approach is what Godel surfaces too, and the window totals give the
  model enough signal.

## Consequences

### Positive

- **Five more pure-compute surfaces** materialise from data we already
  fetch — no new API quotas, no rate limits, no per-symbol latencies.
  All five windows hydrate in microseconds once the feeder caches
  exist.
- **LAN sync carries the new tables** — same rusqlite backend, same
  HMAC sig, same JSON-blob shape. New peers self-materialise the
  `research_insider_activity` / `research_divg` / `research_earm` /
  `research_sector_rotation` / `research_updm` tables via the
  whitelist handshake.
- **Research packet gains insider sentiment + dividend growth arc +
  earnings momentum + sector rotation + analyst rotation** — five of
  the most commonly-referenced Godel panels are now in the AI prompt
  at the cost of ~2–4 KB per symbol.
- **ADR-119 is strictly additive** — no schema changes to Round 1–11
  tables, no broker protocol renames. One palette alias collision
  (`SECTOR_ROTATION` → legacy `show_sector_rotation`) was caught
  during compile and renamed to `SECT_ROT`. Round 11 regression
  surface is empty.

### Neutral / Trade-offs

- EARM requires ≥5 quarters of cached income statements. Recent IPOs
  or names that only fetch 4 quarters via `FA` will silently emit
  INSUFFICIENT_DATA — correct behaviour, but users running EARM on a
  recent IPO will need to understand why.
- DIVG excludes the incomplete current calendar year from CAGR and
  consistency math. A stock that just cut its Q4 dividend in December
  will still show GROWING until the calendar year ends. This is the
  same convention dividend trackers (Seeking Alpha, Simply Safe
  Dividends) use.
- SECTR uses the cached `INDU` snapshot — if the snapshot is stale
  (e.g. INDU hasn't been run for a week), the leader/laggard label
  reflects that stale regime. The window header surfaces the
  snapshot's as_of timestamp so the user can tell.
- MNGR's BULLISH / BEARISH thresholds (net > 10% of gross total, |net|
  > $500k for HIGH conviction) are heuristics lifted from the
  common-knowledge insider-score literature, not user-tunable at this
  layer. Future rounds can add a settings entry if the default drifts.

### Negative

- **~700 more lines in `native/src/app.rs`** — the file continues its
  linear growth with each round. The command palette, window render
  block, and packet builder all gain 5 more branches. Refactoring the
  window block into its own module remains a future task.
- **`research.rs` passes 10,900 lines.** Each round adds structs +
  compute + schema + helpers + tests; splitting compute/tables/tests
  into their own files is deferred past the current parity sweep.

## Implementation notes

### Palette alias collision check

Before adding palette branches, we grep for every candidate alias and
confirm none already resolve to a legacy handler. Round 12 cleared all
aliases except `SECTOR_ROTATION`, which already maps to the legacy
`show_sector_rotation` window. `cargo check` flagged it as
`unreachable pattern` and it was renamed to `SECT_ROT`. All other
aliases are clean: `MNGR`, `INSIDER_BIAS`, `INSIDER_ACTIVITY`,
`INSIDER_SCORE`, `DIVG`, `DIV_GROWTH`, `DIVIDEND_GROWTH`, `DIV_CAGR`,
`EARM`, `EARN_MOMENTUM`, `EARNINGS_MOMENTUM`, `REV_MOMENTUM`, `SECTR`,
`SECT_ROT`, `SECTOR_STRENGTH`, `RS_SECTOR`, `UPDM`, `UPGRADE_MOMENTUM`,
`RATING_MOMENTUM`, `ANALYST_MOMENTUM`.

### Main-thread vs broker-thread reads

SECTR needs the symbol's sector from Fundamentals — that's pre-read on
the main thread *before* dispatching the broker command so the tokio
handler doesn't need to juggle the `fundamentals` read lock alongside
the `research` read lock. MNGR, DIVG, EARM, UPDM all read their own
caches inside the handler: `get_insider_trades` for MNGR,
`get_dividends` for DIVG, `get_financials` + `get_earnings_surprises`
for EARM, and `get_rating_changes` for UPDM.

### `parse_yyyy_mm_dd_to_days` helper

A crude julian-day helper that converts `"YYYY-MM-DD"` into `y·372 +
m·31 + d`. Used only for pairwise comparisons and window filtering
within the same calendar-range; it is *not* suitable for absolute
duration math. Chosen over adding a `chrono` dependency to the pure
compute path because the compute fns must stay pure and
dependency-minimal for the LAN sync / broker-thread story to work.

### EARM composite score

`composite = 50 + 2·revenue_acc + 1.5·surprise_acc`, clamped to
[0, 100]. The 2:1.5 ratio biases the composite toward revenue
acceleration (which is the headline) while still giving surprise
acceleration weight. The ±15 threshold around the midline (50) maps
to ACCELERATING / DECELERATING; values that hit 65 or 35 require
either a sizable revenue trend change or a coordinated surprise
shift, matching Godel's own classification bounds within a point or
two.

### Test coverage

20 new tests in `engine/src/core/research::tests`:

- 5 roundtrip tests (`insider_activity_roundtrip`, `divg_snapshot_roundtrip`,
  `earm_snapshot_roundtrip`, `sector_rotation_snapshot_roundtrip`,
  `updm_snapshot_roundtrip`) verify schema_v12 create + upsert + get +
  JSON roundtrip.
- 4 MNGR tests (`compute_mngr_bullish`, `compute_mngr_bearish`,
  `compute_mngr_no_activity`, `compute_mngr_window_respected`).
- 3 DIVG tests (`compute_divg_growing`, `compute_divg_cutting`,
  `compute_divg_no_history`).
- 2 EARM tests (`compute_earm_accelerating`,
  `compute_earm_insufficient_data`).
- 3 SECTR tests (`compute_sectr_leader`, `compute_sectr_laggard`,
  `compute_sectr_no_data`).
- 3 UPDM tests (`compute_updm_bullish`, `compute_updm_bearish`,
  `compute_updm_no_coverage`).

Engine test suite: **706 passed / 0 failed / 3 ignored**
(686 from Round 11 + 20 new).

## Future work

The parity sweep is not done. Candidates for Round 13, all pure-compute
over existing caches:

- **CALPB** — option put-to-call ratio and skew term-structure from
  cached OMON chains — was on the Round 12 candidate list but deferred
  because it needs richer OMON snapshots (multi-expiry) than the
  current OMON window caches.
- **GROWM** — a combined growth-at-reasonable-price ranking that fuses
  EARM, DIVG, and PTFS into a single screen.
- **CREDIT** — fused Altman Z + Piotroski + leverage + accruals credit
  score, labelled AAA / AA / A / BBB / BB / B / CCC by score bucket.
- **FLOW** — combined insider + institutional flow tape from cached
  `InsiderTrade` + `InstitutionalHolder` deltas.
- **REGIME** — combined VOLE + TECH + HRA regime classifier
  (trending / mean-reverting / volatile / quiet) fused into one label
  per symbol.

The standing directive stands: continue until the compute-over-cache
well runs dry.
