# ADR-120: Godel Parity Round 13 — MOM / LIQ / BREAK / CCRL / CREDIT

**Status:** Accepted
**Date:** 2026-04-14
**Supersedes/extends:** ADR-108, ADR-109, ADR-110, ADR-111, ADR-112, ADR-113, ADR-114, ADR-115, ADR-116, ADR-117, ADR-118, ADR-119
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Context

Round 12 (ADR-119) shipped the "insider sentiment / dividend growth /
earnings momentum / sector rotation / analyst rotation" bundle
(MNGR / DIVG / EARM / SECTR / UPDM). With those in place, the remaining
visible gaps versus Godel Terminal were the **price-action regime
surfaces** — 12-minus-1-month momentum, liquidity profile, breakout
proximity — plus the **working-capital cycle** and a **fused credit
grade**. Godel surfaces each of these as a standalone screen. TyphooN
already had the data (historical price bars, FA statements, and the
Round 10/11 ALTZ/PTFS/LEV/ACRL snapshots) but never folded it into the
five single-label outputs that investors reach for first.

Round 13 picks up those five surfaces. As with Rounds 10 / 11 / 12,
**all five are pure compute over existing caches** (`HP` historical
prices, `FA` statements, Fundamentals, and the cached ALTZ / PTFS / LEV
/ ACRL snapshots) — no new API dependencies:

1. **MOM — 12-1 Month Momentum Score.** Godel's momentum panel is the
   Jegadeesh-Titman classic: the 12-month return excluding the most
   recent month, vol-adjusted and folded into a 0-100 composite with
   a regime label (STRONG / NEUTRAL / WEAK / CRASH) and a trend label
   (ACCELERATING / STABLE / DECELERATING). Needs ≥252 daily bars.
2. **LIQ — Liquidity Profile.** Godel's liquidity panel rolls up
   average share and dollar volume, median counterparts, daily
   turnover % (against shares outstanding), Amihud illiquidity
   (|return| / dollar volume × 1e6), ATR %, and a Corwin-Schultz
   high-low spread proxy, then labels a liquidity tier (DEEP / LIQUID
   / MODERATE / THIN / ILLIQUID). Needs ≥20 daily bars and the
   symbol's shares outstanding pre-read from Fundamentals.
3. **BREAK — Breakout Proximity.** Godel's breakout panel tracks the
   symbol's position inside its 20d / 60d / 52w range, distance from
   each high, consolidation tightness (20d range / mean close), and
   labels a regime (NEW_HIGH / NEAR_HIGH / MID_RANGE / NEAR_LOW /
   NEW_LOW) plus a setup hint (BREAKOUT_IMMINENT / CONSOLIDATING /
   TRENDING_UP / TRENDING_DOWN / NEUTRAL). Pure compute over cached
   HP bars; needs ≥20 bars.
4. **CCRL — Cash Conversion Cycle.** Godel's working-capital panel
   computes DSO + DIO − DPO for the latest period plus per-period
   rows, labels efficiency (EFFICIENT / NEUTRAL / INEFFICIENT) and
   trend (IMPROVING / STABLE / DETERIORATING) from the change vs the
   prior period. Prefers annual statements (days factor 365), falls
   back to quarterly (days factor 91.25) when annuals are missing.
5. **CREDIT — Unified Credit Score.** Fuses the four composite
   surfaces already cached by Round 10 / 11 (ALTZ, PTFS, LEV, ACRL)
   into one 0-100 weighted score (35 / 25 / 25 / 15), graded with a
   letter (AAA ≥90 / AA ≥80 / A ≥70 / BBB ≥60 / BB ≥50 / B ≥35 /
   CCC) and a category label (INVESTMENT_GRADE / BORDERLINE /
   SPECULATIVE / DISTRESSED). First Round 13 surface that consumes
   prior-round outputs directly — the rest stay at the raw-cache
   layer.

The standing directive applies: *"continue combing over vs godel parity
until we cannot add more. rinse/repeat do not worry about round count."*

## Decision

Add five new research surfaces following the Round 10 / 11 / 12 pattern
verbatim:

### Engine (`engine/src/core/research.rs`)

- **New structs** (near line 1359, after `UpdmSnapshot`):
  - `MomentumSnapshot` — flat per-symbol snapshot (symbol, as_of,
    bars_used, 1m / 3m / 6m / 12m / 12-1 returns %, annualised vol %,
    vol-adjusted score, composite 0-100, regime label, trend label,
    note).
  - `LiquiditySnapshot` — flat per-symbol snapshot (symbol, as_of,
    window_days, avg / median share volume, avg / median dollar
    volume, shares_outstanding, daily turnover %, Amihud ×1e6, ATR %,
    Corwin-Schultz spread proxy %, liquidity tier, note).
  - `BreakoutSnapshot` — flat per-symbol snapshot (symbol, as_of,
    current_price, 20d / 60d / 52w highs and lows, distance from
    52w high / low / 20d high / 60d high, position in 52w / 20d
    range %, consolidation %, breakout label, setup label, note).
  - `CashCycleRow` + `CashCycleSnapshot` — one per-period row
    (period, DSO, DIO, DPO, CCC in days) and the per-symbol wrapper
    (symbol, as_of, latest_period, latest DSO/DIO/DPO/CCC,
    prior_ccc_days, ccc_change_days, ccc_3y_avg_days, periods_used,
    efficiency label, trend label, per-period rows, note).
  - `CreditComponent` + `CreditSnapshot` — one per-component row
    (name, value, score, weight, contribution) and the per-symbol
    wrapper (symbol, as_of, altman_z, altman_zone, piotroski_score,
    piotroski_label, leverage_summary, leverage_score, accruals_trend,
    accruals_ttm_cash_conversion_pct, composite_score, letter_grade,
    credit_label, inputs_available, components vec, note).

- **New compute fns** (block near line 6486):
  - `compute_momentum_snapshot(symbol, as_of, bars_newest_first)` —
    requires ≥252 daily bars (one trading year). Picks closes at
    offsets 21 / 63 / 126 / 252 for 1m / 3m / 6m / 12m returns. The
    12-1 return is `pct(c_12m → c_1m)` — the Jegadeesh-Titman
    convention of skipping the most recent month to drop reversal
    bias. Annualised vol is the daily log-return stdev × √252 × 100.
    `vol_adjusted_score = return_12_1 / vol_ann_pct`. Composite =
    `50 + vol_adj·20 + 6m·0.3`, clamped to [0, 100]. Regime STRONG
    ≥75, NEUTRAL ≥40, WEAK ≥20, else CRASH. Trend ACCELERATING when
    1m > 3m/3 AND 3m > 6m/2; DECELERATING when both are reversed;
    STABLE otherwise.
  - `compute_liquidity_snapshot(symbol, as_of, bars_newest_first, shares_outstanding, window_days)`
    — requires ≥20 bars; slices the first `window_days` bars
    (default 60, min 20). Accumulates share volume, dollar volume
    (`volume × close`), true-range %, Amihud terms
    (`|daily return| / dollar volume`), and Corwin-Schultz beta
    (`ln²(H/L)`). Avg / median are computed across the populated
    slice. Daily turnover % = `avg_share / shares_outstanding × 100`.
    Amihud is mean-scaled × 1e6. Spread proxy uses the simplified
    Corwin-Schultz formula
    `α = (√(2β) − √β) / (3 − 2√2); spread ≈ 2(e^α − 1) / (e^α + 1)`
    clamped to ≥0. Tier thresholds on avg_daily_dollar_volume:
    DEEP ≥$500M, LIQUID ≥$50M, MODERATE ≥$5M, THIN ≥$500K,
    ILLIQUID below.
  - `compute_breakout_snapshot(symbol, as_of, bars_newest_first)` —
    requires ≥20 bars. Sweeps the first 20 / 60 / 252 bars for
    high/low ranges using explicit `hi/lo = MIN/MAX` init to avoid
    zero-clamping when gaps present. Position in 52w range =
    `(current - low_52w) / (high_52w - low_52w) × 100` (clamped to
    50 when width is 0). Consolidation = 20d range / mean close × 100.
    Breakout label: NEW_HIGH when pos_52w ≥ 99 AND current ≥ 52w high;
    NEAR_HIGH ≥85; MID_RANGE ≥15; NEAR_LOW ≥1; NEW_LOW otherwise.
    Setup: BREAKOUT_IMMINENT when consolidation <8 AND pos_20d ≥70;
    CONSOLIDATING when <6; TRENDING_UP when |60d-high dist| <3 AND
    pos_52w ≥60; TRENDING_DOWN when pos_52w ≤35 AND dist 52w low <10;
    NEUTRAL otherwise.
  - `compute_cash_cycle_snapshot(symbol, as_of, statements)` — picks
    annual I/S + B/S when both exist; falls back to quarterly when
    annuals are missing. `days_factor = 365` for annual basis,
    `91.25` for quarterly. Per-period: DSO = `net_receivables /
    revenue × days`, DIO = `inventory / cost_of_revenue × days`,
    DPO = `accounts_payable / cost_of_revenue × days`. CCC = DSO +
    DIO − DPO. Returns INSUFFICIENT_DATA when revenue or COGS is
    zero or missing. `prior_ccc_days` from row index 1; change =
    latest − prior. 3y avg takes the first 3 rows (annual) or first
    3 quarters. Efficiency EFFICIENT <30, NEUTRAL <90, else
    INEFFICIENT. Trend IMPROVING when change ≤ -5 days,
    DETERIORATING when ≥ +5 days, STABLE otherwise.
  - `compute_credit_snapshot(symbol, as_of, altman, piotroski, leverage, accruals)`
    — accepts `Option<&>` refs to the four upstream snapshots so the
    broker handler can pass whichever are cached. ALTZ weight 35,
    PTFS weight 25, LEV weight 25, ACRL weight 15. ALTZ mapping:
    Z ≥2.99 → 70..100 linear (capped at Z=5.99); 1.81..2.99 →
    30..70 linear; <1.81 → 0..30 linear. PTFS mapping: F/9 × 100.
    LEV mapping by `solvency_summary` label: HEALTHY=85, MODERATE /
    NEUTRAL=60, ELEVATED=40, STRETCHED / DISTRESSED=15, else 50.
    ACRL mapping by `trend_label`: IMPROVING=80, STABLE=60, MIXED=50,
    DETERIORATING=30, else 50 — then ±10 bonus/penalty when TTM cash
    conversion ≥100% or <50%. Composite = weighted mean over
    populated components. Letter grade: AAA ≥90, AA ≥80, A ≥70,
    BBB ≥60, BB ≥50, B ≥35, CCC below. Credit label:
    INVESTMENT_GRADE ≥70, BORDERLINE ≥55, SPECULATIVE ≥35, else
    DISTRESSED. Returns INSUFFICIENT_DATA when no inputs populated.

- **Schema v13** (`create_research_tables_v13` near line 8624):
  Creates `research_momentum`, `research_liquidity`, `research_breakout`,
  `research_cash_cycle`, and `research_credit` — all follow the
  Round 9 / 10 / 11 / 12 JSON-blob pattern:
  `(symbol TEXT PRIMARY KEY, snapshot_json TEXT, updated_at INTEGER)`
  with per-table `updated_at` indexes for incremental LAN sync.

- **Upsert/get wrappers**: 10 functions total —
  `upsert_momentum` / `get_momentum`, `upsert_liquidity` /
  `get_liquidity`, `upsert_breakout` / `get_breakout`,
  `upsert_cash_cycle` / `get_cash_cycle`, `upsert_credit` /
  `get_credit`. All uppercase the symbol on write and normalise it
  on read.

- **Helper** `pick_close_offset(bars_newest_first, offset)` — safe
  indexed close accessor that returns `Option<f64>` and filters
  non-positive closes so momentum math never divides by junk.
  Reused by `compute_momentum_snapshot` for the four lookback
  offsets (21 / 63 / 126 / 252).

### LAN sync (`engine/src/core/lan_sync.rs`)

- Whitelist the 5 new tables in `SYNCABLE_TABLES` under a Round 13
  marker block after the Round 12 block.
- Add 5 `CREATE TABLE IF NOT EXISTS …` branches in `create_table_sql()`
  so a fresh-peer handshake can materialise empty tables before the
  first bulk sync.
- Add 5 `"table" => Some("updated_at")` mappings in
  `table_timestamp_column()` so incremental sync filters rows by
  timestamp instead of falling back to full sync.

### Native (`native/src/app.rs`)

- **5 new `BrokerCmd` variants** (after the Round 12 block):
  - `ComputeMomentumSnapshot { symbol }` — historical prices loaded
    inside the handler via `get_historical_price` + `get_bars_raw`
    fallback.
  - `ComputeLiquiditySnapshot { symbol, window_days, shares_outstanding }`
    — `shares_outstanding` is pre-read from Fundamentals on the
    main thread so the tokio worker doesn't need to juggle the
    fundamentals read lock alongside the research read lock. Window
    default 60 days, user-tunable in the LIQ window via a DragValue
    (20..=252).
  - `ComputeBreakoutSnapshot { symbol }` — historical prices loaded
    inside the handler.
  - `ComputeCashCycleSnapshot { symbol }` — FA statements loaded
    inside the handler via `get_financials`.
  - `ComputeCreditSnapshot { symbol }` — the four upstream snapshots
    (ALTZ / PTFS / LEV / ACRL) loaded inside the handler via their
    existing getters, then passed as `Option<&>` refs to the compute
    fn. When zero are cached the snapshot carries the
    INSUFFICIENT_DATA marker and a "need at least one of ALTZ / PTFS
    / LEV / ACRL cached" note.

- **5 new `BrokerMsg` variants**: `MomentumSnapshotMsg`,
  `LiquiditySnapshotMsg`, `BreakoutSnapshotMsg`, `CashCycleSnapshotMsg`,
  `CreditSnapshotMsg` — each carries the uppercase symbol + the typed
  snapshot.

- **5 new state sets** on `TyphooNApp` (19 fields total) —
  `show_*`, `*_symbol`, `*_snapshot`, `*_loading` for mom / liq /
  break / ccrl / credit. LIQ has an extra `liq_window_days: i32`
  field (default 60). Note: `show_mom` (not `show_momentum`) was
  chosen because `show_momentum` already exists as the chart
  momentum-indicator toggle — a name collision caught during native
  wiring and resolved by prefixing the window field with the short
  command name.

- **5 tokio::spawn broker handlers** following the Round 12 pattern:
  MOM / BREAK / CCRL / CREDIT load their inputs inside the handler;
  LIQ receives `shares_outstanding` pre-read from Fundamentals on
  the main thread.

- **5 receive arms** with upsert-on-receive (main thread): each arm
  matches the current symbol into the window's loading slot and
  persists the snapshot to SQLite via the matching upsert helper.

- **5 egui windows** — MOM / LIQ / BREAK / CCRL / CREDIT — each with
  the standard header (Symbol input + Use Chart + Load Cached +
  Compute buttons), a Loading indicator, a symbol/status header
  line, and a key-value grid or table. LIQ also exposes a
  window-days DragValue so users can scrub 20–252 days. CCRL emits
  a per-period history table (up to 8 rows). CREDIT emits a
  component contribution table. Colour coding:
  - MOM regime: STRONG = UP, NEUTRAL = AXIS_TEXT, WEAK / CRASH = DOWN.
  - LIQ tier: DEEP / LIQUID = UP, MODERATE = AXIS_TEXT, THIN /
    ILLIQUID = DOWN.
  - BREAK label: NEW_HIGH / NEAR_HIGH = UP, MID_RANGE = AXIS_TEXT,
    NEAR_LOW / NEW_LOW = DOWN.
  - CCRL efficiency: EFFICIENT = UP, NEUTRAL = AXIS_TEXT,
    INEFFICIENT = DOWN.
  - CREDIT letter: AAA / AA / A / BBB = UP, BB = AXIS_TEXT,
    B / CCC = DOWN.

- **5 command-palette entries** — all clean, no collisions:
  - `MOM | MOMENTUM | MOM_SCORE | MOMENTUM_12_1` — chosen over
    `MOMENTUM` alone to avoid shadowing the chart-indicator field
    `show_momentum` that already resolves to the momentum(10)
    oscillator.
  - `LIQ | LIQUIDITY | LIQUIDITY_PROFILE | AMIHUD`
  - `BREAK | BREAKOUT | BREAKOUT_PROXIMITY | BRK_PROX`
  - `CCRL | CASH_CYCLE | CCC | WORKING_CAPITAL_CYCLE`
  - `CREDIT | CREDIT_SCORE | LETTER_GRADE | COMPOSITE_CREDIT`

  Each palette branch reads the active chart symbol, opens its
  window, and loads any cached snapshot into view.

### Research packet (`docs/RESEARCH_PACKET.md`)

- Header count bumped "fifty-two" → **"fifty-seven sub-blocks"**.
- 5 new sub-block sections (2.52–2.56); prior Sector peer comparison
  block renumbered to 2.57.
- 5 new rows in the size-caps table (MOM 5 k/v rows, LIQ 10 k/v rows,
  BREAK 10 k/v rows, CCRL up to 8 per-period rows, CREDIT up to 6
  component rows).
- 5 new rows in the data-source table.
- Packet size estimate: 20–40 KB → **22–44 KB** single symbol;
  190–380 KB → **210–420 KB** 10-symbol basket.
- `investigate_symbols()` in `native/src/app.rs` emits one new
  markdown block per cached Round 13 snapshot (inserted after the
  Round 12 UPDM block near line 18534), silently skipped when the
  data isn't populated.

## Alternatives considered

- **Using the existing TECH (Round 9) panel for MOM**. Rejected —
  TECH is a per-indicator table (RSI / MACD / ADX / etc.) with a
  trend-summary rollup. MOM is the single composite score that
  traders reach for first, with its own regime label and a 0-100
  composite that rolls up volatility-adjusted 12-1 return into a
  one-glance view. The two screens don't overlap in output even
  though both read HP bars.
- **Amihud-only liquidity tier for LIQ**. Rejected — Amihud is a
  single number that conflates price impact with dollar volume.
  Bundling it with avg daily dollar volume (the tier threshold),
  ATR %, and Corwin-Schultz spread lets the model see *three*
  orthogonal liquidity dimensions at once. Godel Terminal's
  liquidity panel does the same bundling.
- **52-week-high-only breakout label**. Rejected — traders care
  about the 20d / 60d context too, because a symbol ±2% from its
  52w high but 15% into its 60d range is a very different setup
  from one ±2% from *both*. The three ranges compose into a single
  label while still carrying the raw distances so the model can
  reason about either.
- **DSO/DIO/DPO only (no CCC rollup)**. Rejected — CCC is the
  single number that working-capital-focused investors track, and
  trend on CCC is what signals working-capital stress before it
  shows up in FCF. Per-period rows are kept in the snapshot for
  the packet-level history table.
- **Equal-weight credit composite (25 / 25 / 25 / 25)**. Rejected —
  Altman Z is the most load-bearing input (it already rolls five
  balance-sheet ratios into a single bankruptcy score) and
  deserves the largest weight. Piotroski and LEV are direct proxies
  for quality and solvency; ACRL is a tie-breaker. 35 / 25 / 25 / 15
  matches the weighting used by most sell-side composite credit
  scores.

## Consequences

### Positive

- **Five more pure-compute surfaces** materialise from data we
  already cache — no new API quotas, no rate limits, no per-symbol
  latencies. All five windows hydrate in microseconds once the
  feeder caches exist.
- **LAN sync carries the new tables** — same rusqlite backend, same
  HMAC sig, same JSON-blob shape. New peers self-materialise the
  `research_momentum` / `research_liquidity` / `research_breakout` /
  `research_cash_cycle` / `research_credit` tables via the whitelist
  handshake.
- **Research packet gains a price-action regime + liquidity profile
  + breakout context + cash conversion cycle + unified credit
  grade** — five of the first questions any fund manager asks about
  a new name are now in the AI prompt at the cost of ~2–4 KB per
  symbol.
- **CREDIT is the first Round-13 surface that composes prior-round
  outputs directly** — it takes the Round 10 / 11 ALTZ / PTFS /
  LEV / ACRL snapshots as inputs and surfaces a single letter
  grade. This establishes the pattern for future fused surfaces
  (e.g. a Round 14 FLOW that fuses INS + HDS deltas, or a REGIME
  label that fuses VOLE + TECH + HRA).
- **ADR-120 is strictly additive** — no schema changes to
  Round 1–12 tables, no broker protocol renames. Round 12
  regression surface is empty.

### Neutral / Trade-offs

- MOM requires ≥252 daily bars. Recent IPOs or symbols with fewer
  than one trading year of HP data will silently emit
  INSUFFICIENT_DATA — correct behaviour, but users running MOM on
  a three-month-old IPO will need to understand why.
- LIQ's spread proxy is Corwin-Schultz, not the actual bid-ask
  spread. The estimator is known to under-shoot when the
  symbol has strong overnight drift (which biases the
  high-low range). It's close enough to rank tiers and matches
  the estimator most liquidity researchers cite.
- BREAK's setup labels (BREAKOUT_IMMINENT / CONSOLIDATING /
  TRENDING_UP etc.) are heuristics tuned on US equities. Thinly
  traded names with erratic 20d ranges may land in NEUTRAL more
  often than a human would.
- CCRL prefers annual over quarterly when both are cached. For
  names with rapidly changing working capital (e.g. retailers
  with strong seasonality), the annual CCC can lag the
  quarter-end snapshot. The window label surfaces the latest
  period so users can tell which basis is in use.
- CREDIT's weights (35 / 25 / 25 / 15) are fixed at this layer.
  Future rounds can surface them as settings if the default
  drifts, but that's deferred until we see a case where it
  matters.

### Negative

- **~880 more lines in `native/src/app.rs`** — the file continues
  its linear growth with each round. The command palette, window
  render block, and packet builder all gain 5 more branches.
  Refactoring the window block into its own module remains a
  future task; this round is consciously deferred.
- **`research.rs` passes 11,600 lines.** Each round adds structs +
  compute + schema + helpers + tests; splitting compute / tables /
  tests into their own files is deferred past the current parity
  sweep.

## Implementation notes

### `show_momentum` name collision

The obvious field name `show_momentum` was already taken by the chart
Momentum(10) oscillator toggle (at `native/src/app.rs:4864`). Using
`show_mom` / `mom_symbol` / `mom_snapshot` / `mom_loading` for the
Round 13 window keeps the chart field intact and makes the distinction
visible at call-sites. The command-palette aliases use the longer
`MOMENTUM` / `MOM_SCORE` / `MOMENTUM_12_1` form so users typing
"MOMENTUM" get the Round 13 score (which is what they almost always
mean) rather than the chart oscillator.

### Main-thread vs broker-thread reads

LIQ needs `shares_outstanding` from Fundamentals — that's pre-read on
the main thread *before* dispatching the broker command so the tokio
handler doesn't need to juggle the `fundamentals` read lock alongside
the `research` read lock. MOM, BREAK, CCRL, CREDIT all read their own
caches inside the handler: `get_historical_price` for MOM and BREAK,
`get_financials` for CCRL, and `get_altman_z` / `get_piotroski` /
`get_leverage` / `get_accruals` for CREDIT.

### 12-minus-1 convention

MOM's 12-1 return skips the most recent 21 trading days (≈1 month) on
purpose — the Jegadeesh-Titman 1993 result showed that the 12-month
signal has reversal bias in the most recent month that wipes out the
momentum edge when holding periods are short. The lookback offsets
(21 / 63 / 126 / 252) are trading days, not calendar days, so the
math lines up with a 5-day week.

### Corwin-Schultz simplification

The full Corwin-Schultz estimator uses two-day high-low pairs to
decompose the spread from the overnight jump. LIQ's compute uses the
single-day version — `β = mean(ln²(H/L))` over the window — which
over-estimates spread by a constant factor (the overnight variance
term) but ranks tiers correctly and is sufficient for the packet. The
window header surfaces the spread proxy so the model can see what's
in its hand.

### CREDIT weighting

The 35 / 25 / 25 / 15 split was chosen so that ALTZ (the most
load-bearing single input — it already combines 5 balance-sheet
ratios into a single bankruptcy score) dominates, but no single
input can swing more than ~35 points on its own. With all four
inputs cached, the composite spans the full 0-100 range; with one
input missing the remaining weights re-normalise, so a symbol
with only ALTZ + PTFS can still receive a grade (it just has
`inputs_available = 2` in the header). Hitting INSUFFICIENT_DATA
requires all four inputs to be missing or zero.

### Test coverage

18 new tests in `engine/src/core/research::tests`:

- 5 roundtrip tests (`momentum_snapshot_roundtrip`,
  `liquidity_snapshot_roundtrip`, `breakout_snapshot_roundtrip`,
  `cash_cycle_snapshot_roundtrip`, `credit_snapshot_roundtrip`)
  verify schema_v13 create + upsert + get + JSON roundtrip.
- 2 MOM tests (`compute_momentum_strong`,
  `compute_momentum_insufficient`).
- 3 LIQ tests (`compute_liquidity_deep`,
  `compute_liquidity_thin`, `compute_liquidity_insufficient`).
- 3 BREAK tests (`compute_breakout_new_high`,
  `compute_breakout_near_low`, `compute_breakout_insufficient`).
- 2 CCRL tests (`compute_cash_cycle_efficient`,
  `compute_cash_cycle_insufficient`).
- 3 CREDIT tests (`compute_credit_investment_grade`,
  `compute_credit_distressed`, `compute_credit_no_inputs`).

Engine test suite: **724 passed / 0 failed / 3 ignored**
(706 from Round 12 + 18 new).

## Future work

The parity sweep is not done. Candidates for Round 14, all
pure-compute over existing caches:

- **GROWM** — a combined growth-at-reasonable-price ranking that
  fuses MOM, EARM, and DIVG into a single screen. Was on the
  Round 13 candidate list but deferred because MOM / EARM / DIVG
  all had to land first.
- **FLOW** — combined insider + institutional flow tape from
  cached `InsiderTrade` + `InstitutionalHolder` deltas, weighted
  by net share change over a user-tunable window.
- **REGIME** — combined VOLE + TECH + HRA regime classifier
  (trending / mean-reverting / volatile / quiet) fused into one
  label per symbol.
- **CALPB** — option put-to-call ratio and skew term-structure
  from cached OMON chains — still blocked on richer OMON
  snapshots (multi-expiry).
- **RELVOL** — relative volume over user-tunable windows (5d /
  20d / 60d) compared against the symbol's own history, with a
  label for unusual activity.

The standing directive stands: continue until the compute-over-cache
well runs dry.
