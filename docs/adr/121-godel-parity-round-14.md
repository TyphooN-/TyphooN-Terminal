# ADR-121: Godel Parity Round 14 — GROWM / FLOW / REGIME / RELVOL / MARGINS

**Status:** Accepted
**Date:** 2026-04-15
**Supersedes/extends:** ADR-108, ADR-109, ADR-110, ADR-111, ADR-112, ADR-113, ADR-114, ADR-115, ADR-116, ADR-117, ADR-118, ADR-119, ADR-120
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Parity classification

| Feature | Godel documented? | TA-Lib primitive? | Research packet | egui popup | Chart overlay |
| --- | --- | --- | --- | --- | --- |
| GROWM (GARP growth-at-reasonable-price composite) | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| FLOW (smart-money flow: insider + 13F) | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| REGIME (market regime classifier) | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| RELVOL (relative volume unusual activity) | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| MARGINS (margin trajectory) | Yes | No | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** pure Godel-Terminal-documented composite surfaces (GARP growth, smart-money flow, regime, unusual volume, margin trajectory); no TA-Lib primitives in this round.

## Context

Round 13 (ADR-120) shipped the price-action-regime bundle
(MOM / LIQ / BREAK / CCRL / CREDIT) and, critically, introduced the
first surface that fuses prior-round outputs instead of the raw caches
(CREDIT = ALTZ + PTFS + LEV + ACRL). That fusion layer unblocked a
class of Godel screens we had deferred: once enough composites are
cached, new surfaces can be built as meta-composites at effectively
zero marginal data cost.

Round 14 picks the next five surfaces off the ADR-120 future-work list
and ships them as pure compute over existing caches — no new API
dependencies:

1. **GROWM — Growth-at-Reasonable-Price Composite.** Godel's "growth
   screen" fuses momentum, earnings acceleration, and dividend
   consistency into a single label (GARP / GROWTH / VALUE /
   SPECULATIVE / NO_DATA) with a 0-100 composite score. Follows
   CREDIT's fusion pattern but consumes the Round 13 MOM composite,
   the Round 12 EARM composite, and the Round 12 DIVG snapshot.
   Weights: MOM 40 / EARM 40 / DIVG 20 (momentum and earnings are
   the primary axes, dividends are a quality tiebreaker).
2. **FLOW — Smart-Money Flow.** Godel's flow tape combines insider
   Form-4 net buying with institutional 13F holder deltas, windowed
   to the user's chosen horizon (default 90 days), and emits a
   0-100 composite with a STRONG_BUY / BUY / NEUTRAL / SELL /
   STRONG_SELL / NO_DATA label. Pure compute over cached
   `InsiderTrade` rows (SEC EDGAR Form 4 → cache) and
   `InstitutionalHolder` rows (13F → cache). Insider gets 60 %,
   institutional 40 % — insiders have tighter filing windows and
   harder regulatory costs, so the signal is cleaner; institutional
   is a slower but broader confirmation.
3. **REGIME — Market Regime Classifier.** Fuses the Round 8 VOLE
   realized-vol snapshot, the Round 7 TECH technicals ADX field,
   and the Round 7 HRA 1-year return / Sharpe into one regime
   label (TRENDING / MEAN_REVERTING / VOLATILE / QUIET /
   INSUFFICIENT_DATA). The classifier is rule-based rather than
   weighted: VOLATILE if realized vol ≥ 40 %, TRENDING if ADX ≥ 25
   and 1Y return positive, QUIET if vol < 20 % and ADX < 18,
   else MEAN_REVERTING. Sub-scores (trend / volatility / return)
   still feed a 0-100 composite for ranking.
4. **RELVOL — Relative Volume.** Godel's unusual-activity screen
   compares the latest bar's volume against 5-day / 20-day / 60-day
   trailing averages (excluding the current bar to prevent
   self-skew) and labels activity (EXTREME ≥3× / HIGH ≥2× /
   ELEVATED ≥1.5× / NORMAL / LOW <0.5×) plus direction
   (BULLISH / BEARISH / NEUTRAL based on current close vs prior).
   Also surfaces the 60-day percentile rank of the current bar's
   volume. Pure compute over cached HP bars; needs ≥20 bars.
5. **MARGINS — Margin Trajectory.** Godel surfaces gross / operating /
   net margin trends over the last several periods with a single
   quality-and-direction label. Pure compute over cached FA
   statements (annual preferred, quarterly fallback). Per-metric
   trend is EXPANDING if change ≥ +1 pp, CONTRACTING if change
   ≤ −1 pp, else STABLE; the overall label is the majority across
   gross / op / net. Quality bucket is HIGH ≥20 %, MEDIUM ≥8 %,
   LOW otherwise (latest operating margin).

With this round, TyphooN has **four surfaces that consume prior-round
outputs directly** (CREDIT, GROWM, REGIME, and — indirectly — FLOW
via the INS+HDS caches already maintained for MNGR). The meta-composite
pattern is now the default for Godel-style "top-level scorecards," and
further rounds will lean harder on it.

The standing directive continues: *"continue combing over vs godel
parity until we cannot add more. rinse/repeat do not worry about round
count."*

## Decision

Add five new research surfaces following the Round 10 / 11 / 12 / 13
pattern:

### Engine (`engine/src/core/research.rs`)

- **New structs** (near line 1484, after `CreditSnapshot`):
  - `GarpComponent` — one per-component row (name, value, score,
    weight, contribution). Used by `GrowmSnapshot.components`.
  - `GrowmSnapshot` — flat per-symbol snapshot (symbol, as_of,
    momentum_score, momentum_regime, earnings_momentum_score,
    earnings_label, dividend_cagr_3y_pct, dividend_trend,
    composite_score, garp_label, inputs_available, components vec,
    note).
  - `FlowSnapshot` — flat per-symbol snapshot (symbol, as_of,
    window_days, insider_buy_value_usd, insider_sell_value_usd,
    insider_net_value_usd, insider_trade_count, unique_insiders,
    institutional_share_delta, institutional_buyers,
    institutional_sellers, institutional_holders_tracked,
    institutional_net_ratio, insider_score, institutional_score,
    composite_score, flow_label, note).
  - `RegimeSnapshot` — flat per-symbol snapshot (symbol, as_of,
    realized_vol_pct, vol_source, adx_value, trend_summary,
    sharpe_ratio, return_1y_pct, trend_strength_score,
    volatility_score, return_score, composite_score, regime_label,
    inputs_available, note).
  - `RelVolSnapshot` — flat per-symbol snapshot (symbol, as_of,
    current_volume, avg_volume_5d / 20d / 60d, rel_volume_5d / 20d /
    60d, volume_trend_5d_pct, volume_percentile_60d, activity_label,
    direction_label, bars_used, note).
  - `MarginRow` + `MarginsSnapshot` — one per-period row (period,
    gross / operating / net margin %) and the per-symbol wrapper
    (symbol, as_of, basis, latest_period, latest / prior / avg
    gross+op+net margins, gross/op/net margin change pp, periods_used,
    per-metric trend labels, overall_trend_label, quality_label,
    per-period rows, note).

- **New compute fns** (block near line 7220):
  - `compute_growm_snapshot(symbol, as_of, momentum?, earm?, divg?)` —
    weight-sums the three inputs (40 / 40 / 20), renormalising over
    available inputs. MOM composite and EARM composite are already
    0-100 and drop in directly; DIVG is scored from `dividend_cagr_3y_pct`
    (≥15 % = 100, ≥10 % = 80, ≥5 % = 60, ≥2 % = 45, else 20) capped
    by `trend_label`. Label logic: GARP if composite ≥ 70 and both
    momentum and earnings are present; GROWTH if ≥ 65 with momentum;
    VALUE if ≥ 55 with dividends only; SPECULATIVE if ≥ 50 with
    momentum only; else VALUE (35+) or SPECULATIVE; NO_DATA if no
    inputs.
  - `compute_flow_snapshot(symbol, as_of, insider_trades, holders,
    window_days)` — windows insider trades by `transaction_date`
    using the crude `parse_yyyy_mm_dd_to_days()` helper and a window
    cut-off of `as_of - window_days×31/30` (loose so near-month
    boundaries don't clip signal). Insider score = `(net / gross) ×
    50 + 50`, clamped 0-100. Institutional score = `net_ratio × 50
    + 50`. Composite weights insider 60 / institutional 40 when
    both present; falls back to the single available side. Labels:
    STRONG_BUY ≥ 80, BUY ≥ 60, NEUTRAL ≥ 40, SELL ≥ 20, else
    STRONG_SELL; NO_DATA if both inputs empty.
  - `compute_regime_snapshot(symbol, as_of, vole?, tech?, hra?)` —
    pulls realized vol from `VOLE.preferred_estimate_pct`, ADX from
    the TECH indicators map (expects a key containing "ADX"), and
    1Y return + Sharpe from HRA. Trend strength score = `min(100,
    adx × 2)` (ADX 50 = full marks). Volatility score is inverse:
    `max(0, 100 − vol × 1.5)`. Return score = `50 + return × 1.5`,
    clamped. Labels (order matters): VOLATILE first if vol ≥ 40,
    then TRENDING if ADX ≥ 25 and return positive, then QUIET if
    vol < 20 and ADX < 18, else MEAN_REVERTING. INSUFFICIENT_DATA
    if no inputs available.
  - `compute_relvol_snapshot(symbol, as_of, bars_newest_first)` —
    needs ≥ 20 bars. Averages are computed over bars[1..] (excluding
    the current bar) to avoid self-skew. Trailing windows: 5d uses
    bars[1..6], 20d uses bars[1..21], 60d uses bars[1..61] (or fewer
    when the series is shorter). Volume trend = `(5d avg / 20d avg −
    1) × 100`. Percentile rank = `rank(current, bars[0..60].volume)
    / N × 100`. Activity label: EXTREME ≥ 3× r20, HIGH ≥ 2×,
    ELEVATED ≥ 1.5×, NORMAL, LOW < 0.5×. Direction: BULLISH if
    current close ≥ prior × 1.005, BEARISH if ≤ prior × 0.995,
    NEUTRAL otherwise.
  - `compute_margins_snapshot(symbol, as_of, statements)` — prefers
    `income_annual`, falls back to `income_quarterly`. Per-period
    rows compute `gross = (revenue − cogs) / revenue`, `op =
    operating_income / revenue`, `net = net_income / revenue`, each
    expressed as a percent. Trend per metric: EXPANDING if change ≥
    +1 pp, CONTRACTING if ≤ −1 pp, else STABLE. Overall: majority
    rule across the three. Quality: HIGH if latest op margin ≥ 20 %,
    MEDIUM ≥ 8 %, LOW otherwise. Periods are returned newest-first
    and capped to 6 rows for compact display.

- **Schema v14** (`create_research_tables_v14`, near line 9487):
  Creates `research_growm`, `research_flow`, `research_regime`,
  `research_relvol`, `research_margins`, each shaped the same way:
  `(symbol TEXT PRIMARY KEY, snapshot_json TEXT, updated_at
  INTEGER)` with an `updated_at` index. The five tables sit on the
  existing one-JSON-blob-per-symbol pattern (no new table layouts).

- **Upsert/get wrappers** (after `get_credit`, line ~9487):
  `upsert_growm` / `get_growm`, `upsert_flow` / `get_flow`,
  `upsert_regime` / `get_regime`, `upsert_relvol` / `get_relvol`,
  `upsert_margins` / `get_margins`. All follow the existing
  `INSERT ... ON CONFLICT` + JSON-serde pattern.

### LAN sync (`engine/src/core/lan_sync.rs`)

- `SYNCABLE_TABLES` — add `research_growm`, `research_flow`,
  `research_regime`, `research_relvol`, `research_margins` under a
  new `// ── ADR-121 Round 14 ──` divider.
- `create_table_sql()` — 5 new arms emitting the same DDL as the
  engine's `create_research_tables_v14` (the DDL strings live in
  two places by design so the sync layer can create receiver
  tables without calling engine code).
- `table_timestamp_column()` — 5 new arms returning `"updated_at"`
  for each new table.

With those three changes, Round 14 tables participate in the same
incremental LAN sync protocol as every prior round.

### Native (`native/src/app.rs`)

- **BrokerCmd variants** (after `ComputeCreditSnapshot`):
  - `ComputeGrowmSnapshot { symbol }`
  - `ComputeFlowSnapshot { symbol, window_days }`
  - `ComputeRegimeSnapshot { symbol }`
  - `ComputeRelvolSnapshot { symbol }`
  - `ComputeMarginsSnapshot { symbol }`

- **BrokerMsg variants** (after `CreditSnapshotMsg`) under a new
  `// ── ADR-121 ──` divider:
  - `GrowmSnapshotMsg(String, GrowmSnapshot)`
  - `FlowSnapshotMsg(String, FlowSnapshot)`
  - `RegimeSnapshotMsg(String, RegimeSnapshot)`
  - `RelvolSnapshotMsg(String, RelVolSnapshot)`
  - `MarginsSnapshotMsg(String, MarginsSnapshot)`

- **TyphooNApp state fields** (after CREDIT state block) under a
  new `// ── ADR-121 Godel Parity Round 14 ──` divider:
  `show_growm / growm_symbol / growm_snapshot / growm_loading`,
  `show_flow / flow_symbol / flow_window_days / flow_snapshot /
  flow_loading`, `show_regime / regime_symbol / regime_snapshot /
  regime_loading`, `show_relvol / relvol_symbol / relvol_snapshot /
  relvol_loading`, `show_margins / margins_symbol / margins_snapshot
  / margins_loading`.

- **Broker handler spawns** (after the CREDIT handler). Each one
  follows the established pattern:
  - Clone `broker_msg_tx` + `shared_cache_broker` into the task.
  - `tokio::spawn` an async block that reads the needed cached
    snapshots/rows on the task thread, calls the corresponding
    `compute_*_snapshot` fn, and sends the resulting `*Msg` back.
  - GROWM pre-reads `get_momentum` / `get_earm` / `get_divg` from
    the cache, passes references into compute.
  - FLOW pre-reads `get_insider_trades` and
    `get_institutional_holders`, passes slices into compute.
  - REGIME pre-reads `get_ohlc_vol` / `get_technicals` / `get_hra`,
    passes references into compute.
  - RELVOL pre-reads `get_historical_price`, passes the slice.
  - MARGINS pre-reads `get_financials`, passes the reference.

- **Receive arms** (in the `BrokerMsg` match, after CREDIT): each
  one updates the matching state field if the incoming symbol
  matches the current `*_symbol`, then upserts the snapshot into
  the cache via `upsert_*`. The upsert is unconditional so
  LAN-synced receivers still get the benefit of the compute even
  when no window is open.

- **egui windows** (after the CREDIT window, ~630x420 defaults,
  each titled `{CODE} — {Long Name}`). Each window has the
  standard header row: symbol editor / Use Chart / Load Cached /
  Compute / Loading spinner, followed by a color-coded summary
  line (`UP` for positive labels, `DOWN` for negative, `AXIS_TEXT`
  for neutral), and then per-surface-specific grids / tables:
  - **GROWM** — 4-row summary grid (momentum regime, earnings
    trend, dividend CAGR, inputs available) + 5-column component
    grid (name, value, score, weight, contribution).
  - **FLOW** — 10-row summary grid with insider / institutional
    sub-scores and the raw share / dollar deltas.
  - **REGIME** — 8-row summary grid (realized vol, ADX, 1Y
    return, Sharpe, 3 sub-scores, inputs available).
  - **RELVOL** — 6-row summary grid (current vol, trailing
    averages, relative volumes, vol trend, 60d percentile, bars
    used).
  - **MARGINS** — 4-column per-metric grid (metric, latest,
    prior, change/trend) + 4-row averages grid + per-period
    history table.

- **Command palette entries** (match arms, not the `COMMANDS`
  const since prior Research rounds also only route via the match
  arm): `GROWM | GARP | GROWTH`, `FLOW | SMART_MONEY |
  INSIDER_FLOW`, `REGIME | MARKET_REGIME | REGIME_CLASSIFIER`,
  `RELVOL | REL_VOLUME | RELATIVE_VOLUME | RELVOLUME`, `MARGINS |
  MARGIN_TRAJECTORY | MARGIN_TREND | MARGIN_HISTORY`. Each arm
  sets `show_*`, copies the current chart symbol into `*_symbol`,
  and opportunistically loads the cached snapshot.

### Research Packet (`docs/RESEARCH_PACKET.md`)

- Header count bumped: fifty-seven → **sixty-two** sub-blocks.
- New sections 2.57 GARP composite (GROWM), 2.58 Smart-money flow
  (FLOW), 2.59 Market regime (REGIME), 2.60 Relative volume
  (RELVOL), 2.61 Margin trajectory (MARGINS).
- Sector peer comparison renumbered 2.57 → **2.62**.
- Size caps table: 5 new rows (GROWM 5 k/v + 5 components, FLOW 10
  k/v, REGIME 8 k/v, RELVOL 6 k/v, MARGINS 4×3 grid + ≤6 history
  rows).
- Data sources table: 5 new rows pointing at the new getters and
  the compute fns.
- Packet size budget revised:
  - Single symbol: 22-44 KB → **24-48 KB**
  - Ten symbols: 210-420 KB → **230-460 KB**

### Native packet generator (`investigate_symbols`)

- Five new blocks in the per-symbol loop, appended after the
  CREDIT block, each gated on "at least one input present":
  GROWM needs `inputs_available > 0`, FLOW needs
  `insider_trade_count > 0 || institutional_holders_tracked > 0`,
  REGIME needs `inputs_available > 0`, RELVOL skips if
  `activity_label == "INSUFFICIENT_DATA"`, MARGINS needs
  `periods_used > 0`. The blocks render the summary line, the
  key sub-metrics, and (where applicable) a ≤6-row
  component/period list.

## Alternatives considered

- **Store GROWM as a plain 0-100 score with no label.** Rejected
  because Godel emits a category label as the headline and
  investors use the label first, the score second. Keeping both
  costs nothing extra in JSON.
- **Score FLOW as signed net dollars rather than a 0-100.**
  Rejected because absolute dollar magnitudes don't normalise
  across market caps — a $10M net buy means very different things
  for AAPL and a small-cap. The (net / gross) × 50 + 50 approach
  is scale-free.
- **Use an exponential decay instead of a hard-window cutoff in
  FLOW.** Rejected as over-engineered: the hard window is easy to
  explain, easy to tune, and survives the common case of
  "investors want to see the last 90d" without argument.
- **Run REGIME on a single input (ADX only) to avoid the
  "inputs_available" fallback complexity.** Rejected because
  we already pay for VOLE + TECH + HRA on every research run; not
  fusing them into the regime label would leave signal on the
  table.
- **Make RELVOL include the current bar in its trailing averages.**
  Rejected because the self-skew on a high-volume day can cut the
  apparent ratio in half and weaken the unusual-activity signal
  exactly when it matters most.
- **Skip MARGINS and use RATIOS + FA directly.** Rejected because
  margin *trajectory* is not in RATIOS today — RATIOS surfaces
  point-in-time margin levels, not the multi-period trend label.
  MARGINS adds ~300 bytes per symbol for the trend label + the
  per-metric pp change, which is a very cheap addition.

## Consequences

- Research packet grows another 2-4 KB / symbol on average when
  Round 14 caches are warm. The ten-symbol packet ceiling rises
  from ~420 KB to ~460 KB, still well under the ~600 KB soft
  target for model-readable single-turn input.
- Five new SQLite tables (`research_growm`, `research_flow`,
  `research_regime`, `research_relvol`, `research_margins`) join
  the LAN-syncable set. Schema v14 is additive: no existing
  Round 1-13 tables change layout, so the LAN-sync compatibility
  story remains "any v≥14 node can talk to any v≥13 node for
  Round 1-13 surfaces; only v14 nodes exchange Round 14 tables."
- **Meta-composite pattern is now dominant.** Four of Round 14's
  five surfaces (GROWM, REGIME, FLOW partially, CREDIT from
  Round 13) fuse upstream snapshots. This gives the parity sweep
  diminishing returns per "round of 5" surfaces — we're
  progressively converting raw-cache surfaces into composed
  screens — but each composition adds real signal.
- **GROWM surfaces latent upstream staleness.** Because GROWM
  reads MOM / EARM / DIVG, a stale momentum snapshot will silently
  make the GARP label stale too. Round 14 doesn't attempt to
  detect this — the `as_of` field on each component is whatever
  was cached when that surface was last computed. Investors who
  want the freshest GROWM need to re-run MOM / EARM / DIVG first,
  then GROWM.
- **FLOW depends on INS and HDS cache freshness.** If the cache
  last refreshed 6 months ago, FLOW's 90-day window will return
  NO_DATA even though the underlying rows exist. The cache
  refresh cadence is external to this ADR.
- The command palette now has 5 new head aliases (GROWM, FLOW,
  REGIME, RELVOL, MARGINS) plus a handful of readable synonyms.
  `MARGIN` (singular) is intentionally *not* a RELVOL/MARGINS
  alias — it already routes to the existing margin monitor.
  `UNUSUAL_VOLUME` is also intentionally not an alias — it
  already routes to the unusual-volume scanner.

## Implementation notes

### Meta-composite discipline

The Round 14 meta-composites (GROWM, REGIME) all use the same
"read what you can, weight what you got, renormalise" approach.
`inputs_available` is a numeric header on every such snapshot, and
the label logic treats "missing input" as a first-class signal
rather than an error. This is why GROWM can still emit a GARP
label with only MOM + EARM (the DIVG weight just gets absorbed
by the other two), and why REGIME can still classify a symbol
with only two of the three inputs.

### Option<i64> handling for date windows

`parse_yyyy_mm_dd_to_days()` returns `Option<i64>` — the crude
helper returns `None` for malformed dates. FLOW's window cutoff
therefore has to be `Option<i64>` too, and the row-level check
pattern-matches `(Some(cutoff), Some(transaction_day))` before
comparing. Any trade whose date parses fails gets *included*
rather than dropped — the assumption is that a SEC EDGAR-sourced
row with a bad date is still more likely to be in-window than
out, and dropping it silently would understate volume.

### MARGINS statement selection

MARGINS prefers annual statements for a stability reason: the
trend label is noise-sensitive, and quarterly margins (especially
for seasonal businesses) will emit EXPANDING / CONTRACTING labels
that reflect seasonality more than fundamental trajectory. The
`basis` field tells the reader which was used. A future extension
could require annuals and emit INSUFFICIENT_DATA when only
quarterlies are available, but that would cut coverage for newly
public names, so the fallback stays.

### Self-skew in RELVOL

Excluding the current bar from the 5d / 20d / 60d averages is not
cosmetic — on a 3× volume day, including the current bar would
pull the 5-day average up by ~40 %, dropping the apparent ratio
from 3× to ~2.1×. The test `compute_relvol_high` specifically
sets up this scenario (5M current, 1M average from the trailing
window) to verify the EXTREME label fires.

### CREDIT weight display

Round 13 left a lingering weight-display bug in the packet block
(`c.weight * 100.0` when `c.weight` was already a percentage). It
was fixed in the packet block during Round 13 commit but the same
bug lived on in the CREDIT window's component-grid cell
(`native/src/app.rs:~34470`). Round 14 fixes that window cell
alongside its own wiring — a quality-of-life fix that belongs
with this ADR because the same weight-format convention carries
over to GROWM, where the weights (40 / 40 / 20) are also raw
percentages.

### Test coverage

15 new tests in `engine/src/core/research::tests`:

- 5 roundtrip tests (`growm_snapshot_roundtrip`,
  `flow_snapshot_roundtrip`, `regime_snapshot_roundtrip`,
  `relvol_snapshot_roundtrip`, `margins_snapshot_roundtrip`)
  verify schema_v14 create + upsert + get + JSON roundtrip.
- 2 GROWM tests (`compute_growm_garp`, `compute_growm_no_inputs`).
- 2 FLOW tests (`compute_flow_buy`, `compute_flow_no_data`).
- 3 REGIME tests (`compute_regime_trending`,
  `compute_regime_volatile`, `compute_regime_no_inputs`).
- 2 RELVOL tests (`compute_relvol_high`,
  `compute_relvol_insufficient`).
- 2 MARGINS tests (`compute_margins_expanding`,
  `compute_margins_insufficient`).

Engine test suite: **740 passed / 0 failed / 3 ignored**
(725 from Round 13's net state + 15 new). Round 13 reported 724;
the extra baseline test came from an existing roundtrip that
runs under the new schema v14 path.

## Future work

The parity sweep continues. Candidates for Round 15, all pure
compute over existing caches:

- **CALPB — Put/Call ratio and skew term-structure.** Still
  blocked on richer OMON chain snapshots (multi-expiry). Round 14
  does not unblock this, but the compute shape is clear: groupby
  expiry, compute put-OI / call-OI per strike bucket, fit a skew
  curve, report ATM vol vs 25-delta wings.
- **BETA — Rolling beta to sector ETF and to SPY** over user-
  tunable windows, with a stability label (STABLE / ROTATING /
  HIGH_BETA / LOW_BETA). Needs HP bars for the symbol and for the
  benchmark, both of which are already cached for world-index and
  sector-performance surfaces.
- **PEAD — Post-earnings-announcement drift window tracker.**
  Needs an earnings-date cache (Round 11 has the beats/misses in
  EPSB but not the date column in a queryable form). Potentially
  unblock by enriching EPSB with `announcement_date` and
  `announcement_time`.
- **COVG — Analyst coverage breadth + churn score.** Fuses the
  number of analyst firms covering the name (from the existing
  price-target cache) with the Round 12 UPDM upgrade/downgrade
  tape. The structure is clear; the only question is whether to
  fold this into a UPDMv2 or keep it separate.
- **INSSTRK — Insider streak detector.** Flags symbols where the
  same insider cluster has been buying (or selling) for ≥ N
  consecutive weeks. Pure post-processing of the `InsiderTrade`
  cache.
- **RELEPSGR — Relative EPS growth** (this symbol's EPS CAGR vs
  its sector median). Needs the Round 9 / 10 EPS stream cache and
  the sector classification from Fundamentals; both already
  present.
- **FQM — Fundamental Quality Meter.** A fusion of ROIC trend,
  FCF margin trend, and leverage trend into one quality label.
  Reuses CREDIT's compute shape.

The standing directive stands: continue until the compute-over-cache
well runs dry. Round 15 will pick the subset that doesn't need new
caches.
