# ADR-122: Godel Parity Round 15 — VAL / QUAL / RISK / INSSTRK / COVG

**Status:** Accepted
**Date:** 2026-04-15
**Supersedes/extends:** ADR-108, ADR-109, ADR-110, ADR-111, ADR-112, ADR-113, ADR-114, ADR-115, ADR-116, ADR-117, ADR-118, ADR-119, ADR-120, ADR-121
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Parity classification

| Feature | Godel Terminal documented | TA-Lib primitive | Research packet | egui popup | Chart overlay |
|---|---|---|---|---|---|
| VAL | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| QUAL | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| RISK | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| INSSTRK | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| COVG | Yes | No | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** pure Godel-Terminal-documented factor-rank / screen surfaces (value, quality, risk, insider streaks, analyst coverage breadth); no TA-Lib primitives in this round.

## Context

Round 14 (ADR-121) shipped GROWM / FLOW / REGIME / RELVOL / MARGINS
and firmly established the meta-composite pattern: new surfaces are
now primarily built by fusing prior-round snapshots rather than by
reading raw caches. GROWM fuses MOM+EARM+DIVG, REGIME fuses
VOLE+TECH+HRA, and CREDIT (from Round 13) fuses ALTZ+PTFS+LEV+ACRL.

Round 15 pushes the pattern harder. Three of its five surfaces are
pure meta-composites that consume Round 10-14 outputs; the remaining
two (INSSTRK, COVG) are post-processing passes over caches that
Round 12 / Round 9 already populate. No new API dependencies:

1. **VAL — Unified Value-Factor Composite.** Godel's value screen
   fuses *six* valuation ratios (P/E, Forward P/E, P/B, P/S,
   EV/EBITDA, FCF Yield) against **sector peers**, computes a
   weighted composite, and emits a single label
   (DEEP_VALUE / VALUE / FAIR / EXPENSIVE / PREMIUM / NO_DATA). This
   is the first Round 15 surface because the multi-metric value
   framing is what the Godel "factor rank" column actually shows.
   Weights: P/E 25 / Forward P/E 15 / P/B 15 / P/S 15 / EV/EBITDA
   20 / FCF Yield 10 — earnings-based multiples get the plurality
   (40 %) because they're the ratios most investors anchor on,
   asset-based (P/B) and sales-based (P/S) get the next tier, and
   cash-flow yield gets the quality tiebreaker slot.
2. **QUAL — Unified Quality-Factor Composite.** Fuses Round 10 PTFS
   (Piotroski F-score), Round 14 MARGINS (operating margin trend),
   Round 10 ACRL (cash conversion / accruals trend), and Round 10
   LEV (leverage summary). Emits HIGH_QUALITY / QUALITY / AVERAGE /
   POOR / WEAK / NO_DATA. Weights: PTFS 30 / MARGINS 25 / ACRL 25 /
   LEV 20. Piotroski gets the plurality because its nine checks are
   the broadest quality proxy we cache; the remaining three weights
   are evenly distributed across profitability, cash quality, and
   solvency.
3. **RISK — Unified Risk-Factor Composite.** Fuses Round 8 VOLE
   (realized vol), the legacy BETA snapshot (beta), Round 13 LIQ
   (liquidity tier), Round 10 SHRT (short % float + DTC), and
   Round 10 ALTZ (Altman Z). Critical inversion: composite score
   is **higher = riskier**, and DISTRESSED overrides numeric
   thresholds when Altman Z is in the distress zone. Weights:
   VOLE 25 / BETA 20 / LIQ 15 / SHRT 15 / ALTZ 25. Vol and
   solvency bracket the bulk (50 %) because they move the largest
   distance between "safe" and "dangerous"; beta is a systematic-risk
   anchor, liquidity and short interest are smaller tilts.
4. **INSSTRK — Insider Streak Detector.** Pure post-processing over
   the cached `InsiderTrade` rows populated for Round 12 MNGR and
   Round 14 FLOW. Groups trades by insider (CEO / CFO / Director /
   etc.), finds each insider's longest consecutive same-direction
   run (buy-buy-buy or sell-sell-sell), tallies buy-streak and
   sell-streak insider counts, and emits a single overall label
   (STRONG_ACCUMULATION / ACCUMULATION / DISTRIBUTION /
   STRONG_DISTRIBUTION / MIXED / NONE). Labels trigger on joint
   conditions — STRONG_ACCUMULATION needs ≥3 distinct insiders with
   buy streaks **and** at least one streak length ≥4. Window is
   tunable (default **180 days**); insiders are identified by
   `InsiderTrade.insider_name`. Nothing about this signal can be
   derived from the aggregate MNGR composite — MNGR tells you
   "insiders are net buying" but hides "the *same* three directors
   have bought four weeks in a row," which is the cluster pattern
   investors actually trade on.
5. **COVG — Analyst Coverage Breadth + Churn Score.** Fuses
   Round 7 PTD (price target + number of analysts), the
   `AnalystRecommendations` cache (consensus distribution), and
   Round 12 UPDM (90d upgrades / downgrades). Emits three sub-scores
   (breadth / consensus / churn) and a composite, plus a label
   (EXPANDING / STABLE / CONTRACTING / THIN / NONE). Composite
   weights: breadth 35 / consensus 35 / churn 30. Label logic:
   THIN when fewer than 5 analysts cover; EXPANDING when net
   90d upgrades ≥ 3 **and** breadth ≥ 70; CONTRACTING when net
   90d ≤ -3; STABLE otherwise.

The standing directive continues: *"continue combing over vs godel
parity until we cannot add more. rinse/repeat do not worry about round
count."*

## Decision

Add five new research surfaces following the Round 10 / 11 / 12 / 13 /
14 pattern. VAL / QUAL / RISK all share a generic `FactorComponent`
row struct (name / value / score / weight % / contribution) for their
component lists — this is the first time we have a shared component
shape across three surfaces, and it reflects the convergence of the
meta-composite pattern on a single factor-ranking idiom.

### Engine (`engine/src/core/research.rs`)

- **New structs** (lines 1619-1754, after `CreditSnapshot`):
  - `FactorComponent` — shared component-row struct used by VAL, QUAL,
    and RISK. Fields: `name`, `value` (display string), `score` (0-100),
    `weight` (raw percent), `contribution` (score × weight / 100).
  - `ValueSnapshot` — VAL (symbol, as_of, sector, peers_considered,
    6 pairs of `metric / sector_median`, composite_score, value_label,
    inputs_available, components, note).
  - `QualitySnapshot` — QUAL (symbol, as_of, piotroski score+label,
    operating margin + trend label, cash conversion + accruals trend,
    leverage summary + debt/EBITDA, composite_score, quality_label,
    inputs_available, components, note).
  - `RiskSnapshot` — RISK (symbol, as_of, realized vol, beta_1y,
    liquidity tier, short%float + DTC, altman_z + zone,
    composite_score [higher = riskier], risk_label, inputs_available,
    components, note).
  - `InsiderStreakRow` — one per-insider streak row (name, direction,
    consecutive_events, net_value_usd, net_shares, first/latest date).
  - `InsiderStreakSnapshot` — INSSTRK (symbol, as_of, window_days,
    unique_insiders, buy_streak_count, sell_streak_count,
    longest_buy_streak, longest_sell_streak, net buy/sell USD,
    streak_label, rows, note).
  - `CoverageSnapshot` — COVG (symbol, as_of, num_analysts,
    target mean/low/high, consensus 5-bucket counts + total +
    bull_ratio, 90d upgrades/downgrades/net/churn, 3 sub-scores,
    composite_score, coverage_label, inputs_available, note).

- **New compute fns** (lines 7974-8830):
  - `compute_val_snapshot(symbol, as_of, sector, fund?, peer_fundamentals,
    fcfy?, peer_fcf_yields)` — computes sector medians for each of the
    six ratios using a generic `median_f64` helper, scores each metric
    via `score_multiple_lower_better` (for P/E, FPE, P/B, P/S, EV/EBITDA)
    or `score_yield_higher_better` (for FCFY), weight-sums, and
    classifies. Label thresholds: DEEP_VALUE ≥80, VALUE ≥65, FAIR ≥45,
    EXPENSIVE ≥30, PREMIUM <30, NO_DATA when no inputs usable.
    Lower-better scoring: ratio ≤ 0.5× median → 100, ≥ 2.0× → 0, linear
    interpolation between. Higher-better scoring (FCFY): ratio ≥ 1.5×
    median → 100, ≤ 0.5× → 0.
  - `compute_qual_snapshot(symbol, as_of, ptfs?, margins?, accruals?,
    leverage?)` — pulls Piotroski F-score directly (0-9 → ×100/9),
    margin score from `operating_margin_pct` (20+ = 100, 10 = 65, 0 = 20),
    accruals score from cash conversion % trend (HIGH = 90, STABLE = 70,
    LOW = 40, DETERIORATING = 20), leverage score from debt/EBITDA
    (≤1 = 95, ≤2 = 80, ≤3 = 60, ≤4 = 40, >4 = 15). Labels:
    HIGH_QUALITY ≥80, QUALITY ≥65, AVERAGE ≥45, POOR ≥30, WEAK <30,
    NO_DATA.
  - `compute_risk_snapshot(symbol, as_of, vole?, beta?, liquidity?,
    short_interest?, altman?)` — vol score = `min(100, realized_vol × 2)`,
    beta score = `|beta − 1| × 100` clamped (beta 1.0 = 0 risk, 2.0 =
    100 risk), liquidity score from tier (DEEP = 10, HIGH = 25,
    MODERATE = 50, THIN = 75, ILLIQUID = 95), short score from
    short%float (≥30 = 100, linear to 0), altman score inverted from
    Z-score (distress = 100, safe = 10). Overall higher = riskier.
    Labels (order matters): DISTRESSED first when Altman zone is
    DISTRESS; then HIGH_RISK ≥75, ELEVATED ≥55, MODERATE ≥35,
    LOW_RISK <35, NO_DATA when no inputs.
  - `compute_insstrk_snapshot(symbol, as_of, trades, window_days)` —
    filters trades to the window using `parse_yyyy_mm_dd_to_days()`,
    groups by `insider_name` into a `BTreeMap<String, Vec<&InsiderTrade>>`
    (deterministic ordering), finds each insider's longest
    consecutive-direction run, classifies per-insider as
    BUY / SELL / MIXED based on their dominant streak, aggregates
    counts and dollar totals. Overall label logic:
    STRONG_ACCUMULATION if buy_streak_count ≥ 3 AND longest_buy_streak
    ≥ 4; ACCUMULATION if buy_streak_count ≥ 2; symmetric for sell side;
    MIXED when both sides have streaks; NONE otherwise.
  - `compute_covg_snapshot(symbol, as_of, price_target?, recs, updm?)` —
    breadth score from `num_analysts` (≥20 = 100, linear down to 0 at
    0 analysts). Consensus score = `bull_ratio × 100` where bull_ratio =
    (strong_buy + buy) / total. Churn score = `50 + net_90d × 5`,
    clamped 0-100 — this deliberately treats "no activity" as neutral
    (50) and moves ±10 points per net upgrade/downgrade. Composite =
    breadth × 0.35 + consensus × 0.35 + churn × 0.30. Labels: THIN when
    num_analysts < 5; EXPANDING when net_90d ≥ 3 AND breadth ≥ 70;
    CONTRACTING when net_90d ≤ -3; STABLE otherwise; NONE when no
    inputs.

- **Shared helpers** (co-located with `compute_val_snapshot`):
  - `fn median_f64(values: &[f64]) -> f64` — simple sort-and-middle,
    used by VAL's sector-median math.
  - `fn score_multiple_lower_better(value: f64, median: f64) -> f64` —
    used by VAL for all five ratios where lower is better.
  - `fn score_yield_higher_better(value: f64, median: f64) -> f64` —
    used by VAL for FCF yield.

- **Schema v15** (`create_research_tables_v15`, near line 10618):
  Creates `research_val`, `research_qual`, `research_risk`,
  `research_insstrk`, `research_covg`, each shaped the same way
  `(symbol TEXT PRIMARY KEY, snapshot_json TEXT, updated_at INTEGER)`
  with an `updated_at` index. Schema v15 is additive: no existing
  Round 1-14 tables change layout.

- **Upsert/get wrappers** (after `get_margins`, lines 10658-10770):
  `upsert_val` / `get_val`, `upsert_qual` / `get_qual`,
  `upsert_risk` / `get_risk`, `upsert_insstrk` / `get_insstrk`,
  `upsert_covg` / `get_covg`. All follow the existing
  `INSERT ... ON CONFLICT` + serde-JSON roundtrip pattern.

### LAN sync (`engine/src/core/lan_sync.rs`)

- `SYNCABLE_TABLES` — add `research_val`, `research_qual`,
  `research_risk`, `research_insstrk`, `research_covg` under a new
  `// ── ADR-122 Round 15 ──` divider.
- `create_table_sql()` — 5 new arms emitting the same DDL as the
  engine's `create_research_tables_v15` (the DDL strings live in
  two places by design so the sync layer can create receiver tables
  without calling engine code).
- `table_timestamp_column()` — 5 new arms returning `"updated_at"`
  for each new table.

### Native (`native/src/app.rs`)

- **BrokerCmd variants** (after `ComputeMarginsSnapshot`, line ~10010):
  - `ComputeValSnapshot { symbol }`
  - `ComputeQualSnapshot { symbol }`
  - `ComputeRiskSnapshot { symbol }`
  - `ComputeInsstrkSnapshot { symbol, window_days }`
  - `ComputeCovgSnapshot { symbol }`

- **BrokerMsg variants** (after `MarginsSnapshotMsg`) under a new
  `// ── ADR-122 ──` divider:
  - `ValSnapshotMsg(String, ValueSnapshot)`
  - `QualSnapshotMsg(String, QualitySnapshot)`
  - `RiskSnapshotMsg(String, RiskSnapshot)`
  - `InsstrkSnapshotMsg(String, InsiderStreakSnapshot)`
  - `CovgSnapshotMsg(String, CoverageSnapshot)`

- **TyphooNApp state fields** (after MARGINS state block) under a new
  `// ── ADR-122 Godel Parity Round 15 ──` divider. Each surface gets
  `show_*` / `*_symbol` / `*_snapshot` / `*_loading`; INSSTRK adds the
  extra `insstrk_window_days: i32` field (default 180).

- **Broker handler spawns** (after the MARGINS handler). Each one
  follows the established pattern:
  - Clone `broker_msg_tx` + `shared_cache_broker` into the task.
  - `tokio::spawn` an async block that reads the needed cached rows
    on the task thread, calls the corresponding `compute_*_snapshot`
    fn, and sends the resulting `*Msg` back.
  - **VAL handler is the most complex** — it needs the subject's
    Fundamentals *and* all sector peers' Fundamentals. Solution:
    inside the spawned task, call `fundamentals::get_fundamentals`
    for the subject, then
    `fundamentals::get_all_fundamentals(&conn)` and filter to peers
    whose sector matches and symbol differs. FCFY median is derived
    analogously from cached FCFY snapshots per peer.
  - QUAL pre-reads `get_piotroski` / `get_margins` / `get_accruals` /
    `get_leverage`.
  - RISK pre-reads `get_ohlc_vol` / `get_beta` / `get_liquidity` /
    `get_short_interest` / `get_altman_z`.
  - INSSTRK pre-reads `get_insider_trades`, passes the slice and the
    user's window_days into compute.
  - COVG pre-reads `get_price_target` / `get_analyst_recs` /
    `get_updm`. (Note: the actual getter is `get_analyst_recs`, not
    `get_analyst_recommendations` — a brief rename confusion was
    caught during native wiring.)

- **Receive arms** (in the `BrokerMsg` match, after MarginsSnapshotMsg):
  each one updates the matching state field if the incoming symbol
  matches the current `*_symbol`, then upserts the snapshot into the
  cache via `upsert_*`. The upsert is unconditional so LAN-synced
  receivers still get the benefit of the compute even when no
  window is open.

- **egui windows** (after the MARGINS window, 630×420 defaults, each
  titled `{CODE} — {Long Name}`). Each window has the standard header
  row: symbol editor / Use Chart / Load Cached / Compute / Loading
  spinner, followed by a color-coded summary line (`UP` / `DOWN` /
  `AXIS_TEXT`), and then per-surface-specific grids / tables:
  - **VAL** — 6-metric grid (symbol ratio vs sector median for each of
    PE / FPE / PB / PS / EV/EBITDA / FCFY) + 6-row component grid
    (name / value / score / weight % / contribution).
  - **QUAL** — 8-row summary grid (Piotroski F, F-label, op margin,
    margin trend, cash conversion, accruals trend, leverage summary,
    debt/EBITDA) + 4-row component grid.
  - **RISK** — 7-row summary grid (realized vol, beta_1y, liquidity
    tier, short % float, days to cover, Altman Z, Altman zone) + 5-row
    component grid. Label cell renders DISTRESSED in red regardless
    of numeric composite.
  - **INSSTRK** — 8-row summary grid (unique insiders, buy streak
    count, sell streak count, longest buy/sell streak, net buy/sell
    USD, window days) + up to 8-row per-insider streak table
    (name / direction / consecutive / net $ / net shares / first /
    latest).
  - **COVG** — 12-row summary grid (num analysts, target mean/low/high,
    consensus SB/B/H/S/SS + total, bull ratio, 3 sub-scores) +
    90d upgrades / downgrades / net / churn row.

- **Command palette entries** (match arms):
  `VAL | VALUE_FACTOR | VALUE_COMPOSITE`,
  `QUAL | QUALITY_FACTOR | QUALITY_COMPOSITE`,
  `RISK | RISK_FACTOR | RISK_COMPOSITE`,
  `INSSTRK | INSIDER_STREAK | INSIDER_STREAKS`,
  `COVG | COVERAGE | ANALYST_COVERAGE | COVERAGE_BREADTH`.
  Each arm sets `show_*`, copies the current chart symbol into
  `*_symbol`, and opportunistically loads the cached snapshot.

### Research Packet (`docs/RESEARCH_PACKET.md`)

- Header count bumped: sixty-two → **sixty-seven** sub-blocks.
- New sections 2.62 Value-Factor Composite (VAL), 2.63 Quality-Factor
  Composite (QUAL), 2.64 Risk-Factor Composite (RISK), 2.65 Insider
  Streak Detector (INSSTRK), 2.66 Analyst Coverage (COVG).
- Sector peer comparison renumbered 2.62 → **2.67**.
- Size caps table: 5 new rows.
- Data sources table: 5 new rows pointing at the new getters.
- Packet size budget revised:
  - Single symbol: 24-48 KB → **26-52 KB**
  - Ten symbols: 230-460 KB → **250-500 KB**

### Native packet generator (`investigate_symbols`)

- Five new blocks in the per-symbol loop, appended after the MARGINS
  block (lines ~19083-19190). Each block is gated on
  `label != "NO_DATA" && !label.is_empty()`. The blocks render the
  header (label + composite + as_of), a key-value list of sub-metrics,
  and (where applicable) a ≤6-row component/streak/consensus table.
  VAL also emits a "Peers considered: N (sector: S)" line so the model
  can judge whether the median is statistically meaningful.

## Alternatives considered

- **Make VAL a sector-neutral Z-score instead of a 0-100 ratio score.**
  Rejected because Z-scores are hard to interpret against a human-readable
  label and require a fatter sector-median sample (typically ≥30) to
  avoid noise. The ratio-vs-median approach gracefully degrades when
  peers are sparse — the composite still emits a fair score even when
  only two or three peers are in the sector.
- **Let QUAL consume GROWM or CREDIT directly as a sub-input.**
  Rejected because both of those are themselves meta-composites, and
  feeding composite outputs into composite inputs creates a dangerous
  "signal laundering" effect — stale caches silently amplify. QUAL
  consumes Round 10 raw caches only (PTFS, MARGINS, ACRL, LEV), which
  keeps the dependency graph one layer deep.
- **Invert RISK so higher = safer.** Rejected because Godel emits
  risk as "how risky is this," not "how safe is this," and the label
  ladder (LOW_RISK / MODERATE / ELEVATED / HIGH_RISK / DISTRESSED)
  reads more naturally with higher = riskier. The component contributions
  stay positive, and the docstring makes the inversion explicit.
- **Compute INSSTRK over all-time data instead of a tunable window.**
  Rejected because insider pattern signal decays fast — a streak that
  ended 2 years ago is noise, not signal. 180 days is long enough to
  catch multi-month accumulation campaigns and short enough to drop
  off stale behaviour.
- **Merge COVG into UPDMv2.** Rejected because UPDM is a momentum
  tape (upgrades/downgrades delta over time), while COVG is a
  cross-sectional breadth measure (how many analysts cover the name
  right now). Fusing them hides the "lots of firms but no recent
  activity" case that THIN vs STABLE needs to distinguish.
- **Use `HashMap` for the INSSTRK per-insider grouping.** Rejected
  because determinism matters for LAN sync and for the window's table
  rendering: the same set of insider trades must produce the same
  snapshot JSON every time, or sync will miss updates. `BTreeMap`
  gives free alphabetical ordering and stable serialisation.

## Consequences

- Research packet grows another 2-4 KB / symbol on average when
  Round 15 caches are warm. The ten-symbol packet ceiling rises from
  ~460 KB to ~500 KB, still well under the ~600 KB soft target for
  model-readable single-turn input.
- Five new SQLite tables (`research_val`, `research_qual`,
  `research_risk`, `research_insstrk`, `research_covg`) join the
  LAN-syncable set. Schema v15 is additive.
- **Meta-composite pattern saturates.** Six surfaces now fuse prior
  round outputs directly (CREDIT, GROWM, REGIME, VAL, QUAL, RISK),
  and every one of them is at the factor-rank / screen tier rather
  than the raw-cache tier. Future rounds will have fewer meta-composite
  candidates to pick from — the low-hanging fruit is largely picked.
- **Shared `FactorComponent` shape is now canonical for factor
  composites.** Any future factor-rank surface (e.g. a MOMENTUM
  factor or a SIZE factor) should reuse this struct rather than
  introducing a new per-surface component type.
- **VAL is the first surface that reads the sector-peer corpus.**
  This means `get_all_fundamentals(&conn)` is called on every VAL
  compute. For a 5000-row fundamentals cache, the scan is O(n) but
  cheap (single SQLite table, fully indexed). If future parity rounds
  need more sector-median lookups, a cached-sector-median table is
  the obvious next optimisation.
- **INSSTRK reveals a gap in the FLOW design.** Round 14 FLOW
  aggregated insider trades into a single buy/sell/net tape without
  attribution per insider. INSSTRK fills that gap — but the two
  surfaces now share input data and diverge only in aggregation
  strategy. A future Round could fold them into a single
  "insider-tape" window with a toggle, but the current split keeps
  each surface's label simple.
- **COVG depends on three caches with different freshness cadences.**
  PriceTarget (Round 7) updates on EVSCRAPE runs, AnalystRecs updates
  on a separate broker pull, and UPDM updates on a Round 12 scan. A
  stale PriceTarget will produce a stale `num_analysts` and therefore
  a stale breadth score. COVG does not detect this today — the `as_of`
  field reflects the compute timestamp, not the earliest input cache.

## Implementation notes

### Shared factor-component scoring

VAL, QUAL, and RISK all instantiate the same `FactorComponent` struct
per input. The component's `contribution` field is always computed as
`score × weight / 100`, so the sum of contributions equals the
composite score. This invariant holds as long as `inputs_available`
matches the renormalisation: if a component is missing, its weight
drops out and the remaining weights are scaled up proportionally. The
composite score is the weighted average after renormalisation, not
the raw weighted sum. This matches the Round 13 CREDIT convention.

### Weight display convention

Weights are stored as raw percentages (e.g. 25.0 means 25 %), not as
fractions. The window renderers and packet blocks display them with
`{:.0}%` directly — **not** `weight * 100`. This is the same bug
class that bit CREDIT in Round 13; the Round 14 ADR documented the
fix, and Round 15 structures enforce it by convention (the new
`FactorComponent` docstring calls out "raw percent weight").

### VAL sector-peer read on the task thread

The VAL broker handler is the first one that needs a whole-table scan
on the DB side (`get_all_fundamentals`). To avoid blocking the UI
thread, the entire sector-peer resolution runs inside the `tokio::spawn`
block:

```rust
let subj = fundamentals::get_fundamentals(&conn, &symbol).ok().flatten();
let sector = subj.as_ref().map(|s| s.sector.clone()).unwrap_or_default();
let mut peers: Vec<fundamentals::Fundamentals> = Vec::new();
if !sector.is_empty() {
    if let Ok(all) = fundamentals::get_all_fundamentals(&conn) {
        for f in all {
            if f.sector == sector && f.symbol.to_uppercase() != symbol.to_uppercase() {
                peers.push(f);
            }
        }
    }
}
```

FCFY peer values are gathered analogously via a per-peer `get_fcf_yield`
loop inside the same task.

### INSSTRK determinism via BTreeMap

Grouping insider trades by name uses `BTreeMap<String, Vec<&Trade>>`
rather than `HashMap`. The snapshot's `rows` field preserves this
alphabetical ordering, which means two nodes computing the same
snapshot from the same input rows produce byte-identical JSON — a
requirement for LAN sync's upsert-if-newer protocol.

### RISK label ordering

DISTRESSED is checked first in `compute_risk_snapshot` — before any
numeric threshold — because an Altman-Z in the distress zone is a
single-factor veto that dominates vol / beta / liquidity / short
considerations. If we checked numeric thresholds first, a company
with moderate vol and low short interest could score MODERATE despite
being on the brink of bankruptcy. The early-return pattern is
intentional.

### COVG churn score centring

Churn score is centred at 50 (neutral) rather than 0, which lets
the COVG composite average behave reasonably when net_90d = 0. If
churn were centred at 0, a "no activity" symbol would drag the
composite down by 30 % and bias all COVG composites low. Centring
at 50 means "no activity" contributes exactly the same weighted
value as "perfectly balanced up/down activity."

### Test coverage

15 new tests in `engine/src/core/research::tests`:

- 5 roundtrip tests (`val_snapshot_roundtrip`, `qual_snapshot_roundtrip`,
  `risk_snapshot_roundtrip`, `insstrk_snapshot_roundtrip`,
  `covg_snapshot_roundtrip`) verify schema_v15 create + upsert + get +
  JSON roundtrip.
- 2 VAL tests (`compute_val_cheap_vs_peers`, `compute_val_no_data`).
- 2 QUAL tests (`compute_qual_high_quality`, `compute_qual_no_data`).
- 3 RISK tests (`compute_risk_distressed_override`,
  `compute_risk_moderate`, `compute_risk_no_inputs`).
- 2 INSSTRK tests (`compute_insstrk_accumulation`,
  `compute_insstrk_empty`).
- 2 COVG tests (`compute_covg_expanding`, `compute_covg_no_inputs`).

Engine test suite: **756 passed / 0 failed / 3 ignored** (741 from
Round 14's net state + 15 new). The 741 baseline is one above the
Round 14 ADR's reported 740 because a latent roundtrip test was
caught by a reachability sweep during Round 15 wiring.

## Future work

The parity sweep continues. Candidates for Round 16, all pure compute
over existing caches:

- **FQM — Fundamental Quality Meter.** Second-order quality composite
  that would fuse QUAL + Round 14 MARGINS + Round 10 ACRL into a
  single "fundamental health" grade. Rejected from Round 15 because
  it risks the signal-laundering concern called out above, but
  re-opening is possible if the one-layer dependency rule is relaxed
  for explicit second-order surfaces.
- **VRK — Value Rank vs Peer Cohort.** Cross-sectional percentile
  rank of the VAL composite within the sector. Needs a sector-wide
  scan of the `research_val` cache — analogous to the VAL handler's
  peer read, but over the new VAL table.
- **QRK / RRK — Analogous rank surfaces for QUAL and RISK.** Same
  shape as VRK.
- **RELEPSGR — Relative EPS Growth** (symbol's 3y EPS CAGR vs sector
  median). Needs the Round 10 EPS stream cache and the sector
  classification from Fundamentals; both already present.
- **PEAD — Post-earnings-announcement drift window tracker.** Still
  blocked on EPSB not carrying `announcement_date` and
  `announcement_time` in a queryable form. A tiny EPSB schema bump
  in Round 16 would unblock this.
- **CALPB — Put/Call ratio and skew term-structure.** Still blocked
  on richer OMON chain snapshots (multi-expiry).
- **BETA — Rolling beta to sector ETF and to SPY** over user-tunable
  windows, with a stability label (STABLE / ROTATING / HIGH_BETA /
  LOW_BETA). Needs HP bars for the symbol and the benchmark, both
  of which are already cached.

The standing directive stands: continue until the compute-over-cache
well runs dry. Round 16 will pick the subset that doesn't need new
caches.
