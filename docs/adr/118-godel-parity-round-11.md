# ADR-118: TA-Lib + Godel Parity Round 11 — ALTZ / PTFS / VOLE / EPSB / PTD

**Status:** Accepted
**Date:** 2026-04-14
**Supersedes/extends:** ADR-108, ADR-109, ADR-110, ADR-111, ADR-112, ADR-113, ADR-114, ADR-115, ADR-116, ADR-117
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Parity classification

| Feature | Godel documented? | TA-Lib primitive? | Research packet | egui popup | Chart overlay |
| --- | --- | --- | --- | --- | --- |
| ALTZ (Altman Z-Score) | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| PTFS (Piotroski F-Score) | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| VOLE (OHLC volatility estimators: Parkinson / GK / RS / YZ) | No | No (literature formulas) | Yes | Yes | No (deferred — ADR-188) |
| EPSB (EPS beat streak) | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| PTD (price target dispersion) | Yes | No | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** mostly Godel-Terminal-documented (composite risk and consensus surfaces: Altman Z, Piotroski F, EPS streak, PT dispersion); VOLE bundles the OHLC volatility-estimator literature (Parkinson / Garman-Klass / Rogers-Satchell / Yang-Zhang) which are classical quant formulas, not TA-Lib functions.

## Context

Round 10 (ADR-117) closed the "capital structure and cash-flow quality"
gap with LEV / ACRL / RVOL / FCFY / SHRT — debt leverage ratios,
earnings-quality accruals, realized volatility cone, FCF yield /
dividend sustainability, and short interest / days-to-cover. With those
in place, the next visible gap versus Godel Terminal was the **composite
risk scoring and analyst-consensus** layer: a bankruptcy-risk single
number, a one-shot quality score, the full family of OHLC volatility
estimators (not just close-to-close), EPS beat streaks as a sentiment
proxy, and analyst price-target dispersion as a consensus signal.

Round 11 picks up the five surfaces that close that gap. As with
Round 10, **all five are pure compute over existing caches** (`FA`
FinancialStatements, `HP` bars, `ERN` EarningsSurprise history, `UPDG`
PriceTarget, Fundamentals) — no new API dependencies:

1. **ALTZ — Altman Z-Score.** Godel's credit panel shows the classic
   5-component Z for public manufacturers with DISTRESS / GRAY / SAFE
   zones. TyphooN already caches `FA` balance sheets + income statements
   via ADR-108; ALTZ picks the latest annual (quarterly fallback),
   computes WC/TA, RE/TA, EBIT/TA, MVE/TL, Sales/TA, multiplies by the
   standard coefficients (1.2, 1.4, 3.3, 0.6, 1.0), and sums. MVE comes
   from Fundamentals market cap on the main thread.
2. **PTFS — Piotroski F-Score.** Godel's quality panel scores a
   nine-point checklist across profitability (positive NI, positive
   OCF, ROA↑, OCF>NI), leverage/liquidity (LTDebt/TA↓, current ratio↑,
   no new shares), and operating efficiency (gross margin↑, asset
   turnover↑). Requires two consecutive annual periods; TyphooN's
   `FA.income_annual` / `balance_annual` / `cashflow_annual` already
   provide exactly that.
3. **VOLE — OHLC Volatility Estimators.** Godel's vol panel shows the
   full family of range-based volatility estimators: Close-to-Close,
   Parkinson, Garman-Klass, Rogers-Satchell, and Yang-Zhang — all
   annualized with √252. RVOL (Round 10) only used close-to-close;
   VOLE adds the four range-based estimators that are strictly more
   efficient and includes the opening-jump-aware Yang-Zhang as the
   preferred estimator. Pure compute over cached `HP` bars.
4. **EPSB — EPS Beat Streak & Surprise.** Godel's earnings panel
   tracks beat/miss counters, current streak, longest streaks, and a
   recent vs historical surprise trend. TyphooN already caches
   `EarningsSurprise` per report via `get_earnings_surprises`; EPSB
   walks that list oldest-first, builds the counts, and labels the bias
   (POSITIVE / NEUTRAL / NEGATIVE) and trend (ACCELERATING / STABLE /
   DECELERATING).
5. **PTD — Price Target Dispersion.** Godel's consensus panel shows
   analyst target high/low/mean/median, dispersion, spread vs current,
   implied return to median and mean, and a consensus label. TyphooN
   already caches `PriceTarget` aggregate from ADR-108 UPDG; PTD plugs
   it into the dispersion + implied-return math and classifies
   BULLISH / NEUTRAL / BEARISH.

The standing directive applies: *"continue combing over vs godel parity
until we cannot add more. rinse/repeat do not worry about round count."*

## Decision

Add five new research surfaces following the Round 10 pattern verbatim:

### Engine (`engine/src/core/research.rs`)

- **New structs** (ADR-118 section near line 1097):
  - `AltmanComponent` + `AltmanZSnapshot` — one component row (name,
    ratio, coefficient, contribution, note) and the per-symbol wrapper
    (working capital, retained earnings, EBIT, market value equity,
    sales, total assets, total liabilities, Z-score, zone, components
    vector, note).
  - `PiotroskiCheck` + `PiotroskiSnapshot` — one check row (category,
    name, passed bool, current value, prior value, note) and the
    per-symbol wrapper (current period label, prior period label,
    F-score 0..9, strength label, sub-scores by category, checks
    vector, note).
  - `VolEstimator` + `OhlcVolSnapshot` — one estimator row (name,
    annualized vol %, efficiency vs close-to-close, note) and the
    per-symbol wrapper (trading days, estimators vector, preferred
    estimate %, preferred label, note).
  - `EpsBeatSnapshot` — flat per-symbol snapshot (total reports, beats,
    misses, inlines, beat rate %, current streak signed, longest beat
    streak, longest miss streak, avg surprise %, median surprise %,
    recent-4 avg surprise %, bias label, trend label, latest date,
    latest surprise %, note).
  - `PriceTargetDispersion` — flat per-symbol snapshot (current price,
    target high/low/mean/median, num analysts, dispersion %, spread %,
    implied return median/mean %, upside to high %, downside to low %,
    consensus label, note).

- **New compute fns** (ADR-118 block near line 5041):
  - `compute_altman_z_snapshot(symbol, as_of, statements, market_value_equity)`
    — picks `balance_annual.first()` with `balance_quarterly.first()`
    fallback; same for income. Builds the five components, sums, and
    classifies by zone (DISTRESS <1.81, GRAY, SAFE ≥2.99). Returns
    INSUFFICIENT_DATA when MVE ≤ 0 or total_assets ≤ 0 or
    total_liabilities ≤ 0.
  - `compute_piotroski_snapshot(symbol, as_of, statements)` — requires
    `income_annual.len() ≥ 2 && balance_annual.len() ≥ 2 &&
    !cashflow_annual.is_empty()`. Builds 9 checks (4 profitability, 3
    leverage/liquidity, 2 efficiency) with `passed` bool + current/prior
    values. Labels STRONG ≥ 7, WEAK ≤ 3, else MIXED.
  - `compute_ohlc_vol_snapshot(symbol, as_of, bars_oldest_first, window_days)`
    — uses `window_days.max(20)`, takes tail, filters valid OHLC bars.
    Computes Close-to-Close (log return stdev × √252), Parkinson
    (range-based with 1/(4·ln2) coefficient), Garman-Klass (0.5·ln(H/L)²
    − (2ln2−1)·ln(C/O)²), Rogers-Satchell (drift-independent:
    hc·ho + lc·lo with hc = ln(H/C), ho = ln(H/O), lc = ln(L/C),
    lo = ln(L/O)), and Yang-Zhang (overnight + k·oc + (1−k)·rs with
    k = 0.34/(1.34 + (N+1)/(N−1))). Preferred = Yang-Zhang when all
    components are available; falls back to Parkinson, then CtC.
  - `compute_eps_beat_snapshot(symbol, as_of, reports)` — sorts
    oldest-first by date string. Counts beats / misses / inlines (± 0.5%
    tolerance for inline). Walks newest-back for current_streak (signed).
    Tracks longest_beat / longest_miss via running runs. Computes
    avg / median / recent-4 average surprise_pct. Bias POSITIVE if avg
    > 2.0, NEGATIVE if < -2.0, else NEUTRAL. Trend ACCELERATING if
    recent > avg + 1.0, DECELERATING if < avg - 1.0, else STABLE.
  - `compute_price_target_dispersion(symbol, as_of, current_price, target: Option<&PriceTarget>)`
    — returns NO_COVERAGE when target is None or num_analysts ≤ 0.
    Otherwise computes dispersion_pct = (high - low) / mean × 100,
    spread_pct = (high - low) / current × 100, implied returns vs
    median / mean, upside_to_high, downside_to_low. Consensus BULLISH
    when implied_median ≥ 10%, BEARISH when ≤ -5%, else NEUTRAL.

- **Schema v11** (`create_research_tables_v11` near line 6854):
  Creates `research_altman_z`, `research_piotroski`, `research_ohlc_vol`,
  `research_eps_beat`, and `research_price_target_dispersion` — all
  follow the Round 9/10 JSON-blob pattern:
  `(symbol TEXT PRIMARY KEY, snapshot_json TEXT, updated_at INTEGER)`
  with per-table `updated_at` indexes for incremental LAN sync.

- **Upsert/get wrappers**: 10 functions total —
  `upsert_altman_z` / `get_altman_z`, `upsert_piotroski` / `get_piotroski`,
  `upsert_ohlc_vol` / `get_ohlc_vol`, `upsert_eps_beat` / `get_eps_beat`,
  `upsert_price_target_dispersion` / `get_price_target_dispersion`. All
  uppercase the symbol on write and normalise it on read.

### LAN sync (`engine/src/core/lan_sync.rs`)

- Whitelist the 5 new tables in `SYNCABLE_TABLES` under a Round 11 marker
  block after the Round 10 block.
- Add 5 `CREATE TABLE IF NOT EXISTS …` branches in `create_table_sql()`
  so a fresh-peer handshake can materialise empty tables before the first
  bulk sync.
- Add 5 `"table" => Some("updated_at")` mappings in
  `table_timestamp_column()` so incremental sync filters rows by
  timestamp instead of falling back to full sync.

### Native (`native/src/app.rs`)

- **5 new `BrokerCmd` variants** (ADR-118 section after the Round 10
  block):
  - `ComputeAltmanZSnapshot { symbol, market_value_equity }` — MVE is
    pre-read from Fundamentals on the main thread so the broker handler
    stays Send-safe.
  - `ComputePiotroskiSnapshot { symbol }` — statements loaded inside the
    broker handler from the shared cache.
  - `ComputeOhlcVolSnapshot { symbol, window_days, bars_json }` — bars
    are pre-read and JSON-encoded on the main thread to keep the
    `&Connection` off the tokio worker. `window_days` defaults to 60 in
    the window UI.
  - `ComputeEpsBeatSnapshot { symbol }` — earnings surprise history
    loaded inside the broker handler.
  - `ComputePriceTargetDispersionSnapshot { symbol, current_price }` —
    current price is pre-read from Fundamentals on the main thread.

- **5 new `BrokerMsg` variants**: `AltmanZSnapshotMsg`,
  `PiotroskiSnapshotMsg`, `OhlcVolSnapshotMsg`, `EpsBeatSnapshotMsg`,
  `PriceTargetDispersionSnapshotMsg` — each carries the uppercase
  symbol + the typed snapshot.

- **5 new state sets** on `TyphooNApp` (20 fields total) — `show_*`,
  `*_symbol`, `*_snapshot`, `*_loading` for altz / ptfs / vole / epsb /
  ptd. Defaults wired in the struct literal after the Round 10 block.

- **5 tokio::spawn broker handlers** following the Round 10 pattern:
  ALTZ and PTFS load statements inside the handler; VOLE receives bars
  as JSON; EPSB loads earnings surprise history inside the handler; PTD
  loads the cached price target inside the handler.

- **5 receive arms** with upsert-on-receive (main thread): each arm
  matches the current symbol into the window's loading slot and
  persists the snapshot to SQLite via the matching upsert helper.

- **5 egui windows** — ALTZ / PTFS / VOLE / EPSB / PTD — each with the
  standard header (Symbol input + Use Chart + Load Cached + Compute
  buttons), a Loading indicator, a symbol/status header line, and a
  table or key-value grid. Colour coding:
  - ALTZ zone: SAFE = UP, GRAY = AXIS_TEXT, DISTRESS = DOWN.
  - PTFS strength: STRONG = UP, MIXED = AXIS_TEXT, WEAK = DOWN, and
    per-check PASS = UP / FAIL = DOWN.
  - EPSB bias: POSITIVE = UP, NEUTRAL = AXIS_TEXT, NEGATIVE = DOWN.
  - PTD consensus: BULLISH = UP, NEUTRAL = AXIS_TEXT, BEARISH = DOWN.

- **5 command-palette entries** (none shadow existing aliases):
  - `ALTZ | ALTMAN | Z_SCORE | BANKRUPTCY_RISK`
  - `PTFS | PIOTROSKI | F_SCORE | QUALITY_SCORE`
  - `VOLE | OHLC_VOL | VOL_ESTIMATORS | YANG_ZHANG`
  - `EPSB | EPS_BEAT | BEAT_STREAK | SURPRISE_HISTORY`
  - `PTD | TARGET_DISPERSION | IMPLIED_RETURN | CONSENSUS_TARGET`

  Each palette branch reads the active chart symbol, opens its window,
  and loads any cached snapshot into view.

### Research packet (`docs/RESEARCH_PACKET.md`)

- Header count bumped "forty-two" → **"forty-seven sub-blocks"**.
- 5 new sub-block sections (2.42–2.46); prior Sector peer comparison
  block renumbered to 2.47.
- 5 new rows in the size-caps table (ALTZ 5 rows, PTFS 9 rows, VOLE 5
  rows, EPSB 8 k/v rows, PTD 8 k/v rows).
- 5 new rows in the data-source table.
- Packet size estimate: 16–32 KB → **18–36 KB** single symbol; 150–300
  KB → **170–340 KB** 10-symbol basket.
- `investigate_symbols()` in `native/src/app.rs` emits one new markdown
  block per cached Round 11 snapshot, silently skipped when the data
  isn't populated.

## Alternatives considered

- **External API for Altman / Piotroski** (FMP / stockanalysis.com). The
  ratios ship on both vendors but cost an API key per symbol per day.
  Rejected — we already have every input cached from ADR-108 `FA`, so
  compute locally at zero marginal cost.
- **One unified "quality score" struct** combining Z + F. Rejected — the
  two scores have different zone logic, different required periods, and
  different audit tables. Keeping them separate mirrors the Round 10
  decomposition (LEV ≠ ACRL ≠ FCFY) and keeps each window readable.
- **Parkinson-only OHLC volatility**. Rejected — Yang-Zhang is strictly
  more efficient because it accounts for overnight gaps and is
  drift-independent when the stock trends, so it's the right default.
  Shipping all five side-by-side lets the packet reader see the
  efficiency spread at a glance.
- **EPS beat streak on top of `EarningRow`** (from `get_earnings`
  legacy). Rejected — that table stores period/quarter/year but not the
  per-report date / surprise_pct needed for streak math. `get_earnings_surprises`
  (ADR-112 ERN) already returns the per-report history in the required
  shape.
- **PTD showing the analyst distribution as a histogram**. Rejected for
  this round — the existing `PriceTarget` only stores aggregate
  high/low/mean/median (via FMP / Finnhub aggregate endpoints), so we
  have no per-analyst rows to bucket. Dispersion + spread are derivable
  from the aggregate and already cover the consensus story.

## Consequences

### Positive

- **Five more pure-compute surfaces** materialise from data we already
  fetch — no new API quotas, no rate limits, and no per-symbol
  latencies. All five windows hydrate in microseconds once the feeder
  caches exist.
- **LAN sync carries the new tables** — same rusqlite backend, same
  HMAC sig, same JSON-blob shape. New peers self-materialise the
  `research_altman_z` / `research_piotroski` / `research_ohlc_vol` /
  `research_eps_beat` / `research_price_target_dispersion` tables via
  the whitelist handshake.
- **Research packet gains bankruptcy risk + quality score + vol
  estimator family + beat streak + consensus target** — five of the
  most commonly-referenced Godel panels are now in the AI prompt at the
  cost of ~2–4 KB per symbol.
- **ADR-118 is strictly additive** — no schema changes to Round 1–10
  tables, no broker protocol renames, no command alias collisions. Round
  10 regression surface is empty.

### Neutral / Trade-offs

- Piotroski requires 2 consecutive annual periods. Names with <2 years
  of `FA` history silently emit INSUFFICIENT_DATA — correct behaviour,
  but users running PTFS on a recent IPO will need to understand why.
- Yang-Zhang is opening-jump-aware and therefore more sensitive to
  thin premarket prints in low-volume names than Parkinson. We surface
  all 5 estimators side-by-side so the packet reader can cross-check.
- EPS beat bias/trend thresholds (±2% bias, ±1% trend) are the same
  heuristic used across Godel-style quality panels but are not
  user-tunable at this layer. Future rounds can add a settings entry if
  the default drifts.

### Negative

- **~600 more lines in `native/src/app.rs`** — the file continues its
  linear growth with each round. The command palette, window render
  block, and packet builder all gain 5 more branches. Refactoring the
  window block into its own module remains a future task.
- **`research.rs` passes 8,700 lines.** Each round adds structs +
  compute + schema + helpers + tests; splitting compute/tables/tests
  into their own files is deferred past the current parity sweep.

## Implementation notes

### Palette alias collision check

Before adding palette branches, we grep for every candidate alias and
confirm none already resolve to a legacy handler. Round 11 clears:
`ALTZ`, `ALTMAN`, `Z_SCORE`, `BANKRUPTCY_RISK`, `PTFS`, `PIOTROSKI`,
`F_SCORE`, `QUALITY_SCORE`, `VOLE`, `OHLC_VOL`, `VOL_ESTIMATORS`,
`YANG_ZHANG`, `EPSB`, `EPS_BEAT`, `BEAT_STREAK`, `SURPRISE_HISTORY`,
`PTD`, `TARGET_DISPERSION`, `IMPLIED_RETURN`, `CONSENSUS_TARGET`.

### Main-thread vs broker-thread reads

ALTZ needs MVE and PTD needs current price — both come from the
Fundamentals table. These are read on the main thread *before*
dispatching the broker command so the tokio handler doesn't need to
juggle the shared cache lock for both FA and FUND in one task.
PTFS, VOLE, EPSB all read their own snapshots inside the handler: the
FA load for PTFS, the HP bar load (pre-encoded as JSON on the main
thread) for VOLE, and the `EarningsSurprise` history for EPSB.

### Yang-Zhang preference

The preferred estimator is Yang-Zhang whenever the full 60-day window
has valid overnight + intraday legs. Yang-Zhang is drift-independent
and overnight-gap-aware, so it's the right default for range-based
volatility reporting. The efficiency column (versus close-to-close)
surfaces the gain: values around 3–8x are typical for liquid US equities
over 60d.

### Test coverage

14 new tests in `engine/src/core/research::tests`:

- 5 roundtrip tests (`altman_z_snapshot_roundtrip`, `piotroski_snapshot_roundtrip`,
  `ohlc_vol_snapshot_roundtrip`, `eps_beat_snapshot_roundtrip`,
  `price_target_dispersion_roundtrip`) verify schema_v11 create +
  upsert + get + JSON roundtrip.
- 9 compute tests:
  - `compute_altman_z_on_healthy_statements` + `compute_altman_z_insufficient_data_returns_note`
  - `compute_piotroski_strong_score` + `compute_piotroski_insufficient_data`
  - `compute_ohlc_vol_five_estimators` + `compute_ohlc_vol_insufficient_bars`
  - `compute_eps_beat_six_beats_labels_positive` + `compute_eps_beat_empty_reports`
  - `compute_price_target_dispersion_bullish` + `compute_price_target_dispersion_no_coverage`

Engine test suite: **686 passed / 0 failed / 3 ignored**
(671 from Round 10 + 15 new — 14 Round 11 plus one previously-counted
delta from the Round 10 baseline).

## Future work

The parity sweep is not done. Candidates for Round 12, all pure-compute
over existing caches:

- **EARN_MOMENTUM** — revenue + EPS surprise trend across 8 quarters
  with an acceleration/deceleration classifier, building on EPSB.
- **MNGR** — insider-buying ratio and net insider flow from the cached
  `research::get_insider_trades` table.
- **DIVG** — dividend growth rate / dividend yield trajectory with
  5Y / 10Y CAGR, using the cached dividend history.
- **CALPB** — option put-to-call ratio and skew term-structure from
  cached OMON chains.
- **SECTR** — sector rotation strength via cached sector performance
  snapshots.

The standing directive stands: continue until the compute-over-cache
well runs dry.
