# ADR-117: Godel Parity Round 10 — LEV / ACRL / RVOL / FCFY / SHRT

**Status:** Accepted
**Date:** 2026-04-14
**Supersedes/extends:** ADR-108, ADR-109, ADR-110, ADR-111, ADR-112, ADR-113, ADR-114, ADR-115, ADR-116
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Parity classification

| Feature | Godel documented? | TA-Lib primitive? | Research packet | egui popup | Chart overlay |
| --- | --- | --- | --- | --- | --- |
| LEV (debt leverage & coverage) | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| ACRL (earnings quality / accruals) | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| RVOL (realized volatility cone) | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| FCFY (FCF yield / dividend sustainability) | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| SHRT (short interest / days-to-cover) | Yes | No | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** pure Godel-Terminal-documented research surfaces (credit leverage, earnings quality, realized vol cone, FCF yield, short interest); no TA-Lib primitives in this round.

## Context

Round 9 (ADR-116) closed the "time-series pattern analytics" gap with
SEAG / COR / TRA / TECH / SKEW — seasonality, cross-asset correlation,
dividend-aware total return, technical indicator snapshots, and the
implied-volatility surface. With those surfaces in place, the next
visible gap versus Godel Terminal was the **capital-structure and
cash-flow quality** layer: how much debt the issuer carries, whether
reported earnings translate into cash, how volatile the stock has been
historically vs what options are currently pricing, whether the dividend
is actually funded by free cash flow, and how crowded the short side is.

Round 10 picks up the five surfaces that fit TyphooN's research-packet
pattern and eliminate that gap. Crucially, **all five are pure compute
over existing caches** (`FA` FinancialStatements, `HP` bars, `IVOL`
snapshot, `FLOAT` shares-float, Fundamentals):

1. **LEV — Debt Leverage & Coverage.** Godel's credit-metrics panel
   reports Debt/EBITDA, Net Debt/EBITDA, Debt/Equity, interest coverage,
   and current/quick ratios with an investment-grade classification.
   TyphooN already caches `FA` quarterly income/balance/cashflow rows
   via ADR-108; LEV rolls 4 quarters into a TTM EBITDA + interest
   expense, picks the latest balance sheet, and classifies each ratio
   against the standard solvency cones.
2. **ACRL — Earnings Quality / Accruals.** Godel's quality view contrasts
   reported net income with free cash flow over the last 8 quarters to
   flag accrual build-up. TyphooN already has the quarterly statements;
   ACRL pairs income and cashflow rows by period date, computes
   FCF/NI cash conversion, and trends the latest two periods vs the
   older two to derive IMPROVING / STABLE / DETERIORATING.
3. **RVOL — Realized Volatility Cone.** Godel's vol surface shows a
   realized-vol cone across 20d / 60d / 120d / 252d windows with a
   percentile rank vs the rolling history of the same window, and
   compares current ATM IV to realized to classify CHEAP_IV / FAIR_IV /
   RICH_IV. TyphooN already caches `HP` bars (ADR-112) and optionally
   `IVOL` (ADR-115); RVOL builds log returns, rolls stdev × √252, and
   uses the cached IVOL ATM reading when present.
4. **FCFY — FCF Yield & Dividend Sustainability.** Godel's dividend
   quality view reports FCF yield (FCF / market cap), payout-from-FCF,
   payout-from-NI, 5Y FCF CAGR, and a SAFE / STRETCHED / UNSUSTAINABLE
   classification for the current dividend. TyphooN already has
   cashflow/income statements plus market cap from Fundamentals; FCFY
   pulls TTM FCF, TTM dividends paid, and computes the ratios.
5. **SHRT — Short Interest & Days-to-Cover.** Godel's short-interest
   panel reports shares short, short % of float, days-to-cover, and a
   squeeze-risk classification (LOW / ELEVATED / HIGH / EXTREME).
   TyphooN already has `short_percent_of_float` and `short_ratio` in
   Fundamentals plus `float_shares` cached via `FLOAT` (ADR-112) and
   bar volume via `HP`; SHRT computes DTC = short_shares /
   avg_daily_volume_20d and classifies the squeeze risk.

The standing directive applies: *"continue combing over vs godel parity
until we cannot add more. rinse/repeat do not worry about round count."*

## Decision

Add five new research surfaces following the Round 7/8/9 pattern verbatim:

### Engine (`engine/src/core/research.rs`)

- **New structs** (ADR-117 section near line 970):
  - `LeverageRatio` + `LeverageSnapshot` — one ratio row (name, value,
    peer median, signal, note) and the per-symbol wrapper (total debt,
    net debt, TTM EBITDA, TTM interest expense, total equity, ratios
    vector, solvency summary, note).
  - `AccrualPeriod` + `AccrualsSnapshot` — one quarter row (period label,
    date, NI, FCF, FCF/NI ratio, cash conversion %, accruals, quality
    label) and the per-symbol wrapper (TTM NI, TTM FCF, TTM cash
    conversion %, average cash conversion, periods vector, trend label,
    note).
  - `RealizedVolWindow` + `RealizedVolSnapshot` — one window row (label,
    trading days, realized vol %, percentile, observation count) and the
    per-symbol wrapper (last close, current ATM IV, IV/RV gap, IV/RV
    ratio, windows vector, regime label, note).
  - `FcfYieldPeriod` + `FcfYieldSnapshot` — one period row (period, date,
    FCF, dividends paid, payout-from-FCF %, payout-from-NI %, FCF yield
    %) and the per-symbol wrapper (market cap, TTM FCF, TTM dividends,
    TTM FCF yield, TTM dividend yield, TTM payout-from-FCF, TTM
    payout-from-NI, 5Y FCF CAGR, periods vector, sustainability label,
    note).
  - `ShortInterestSnapshot` — a single flat snapshot: shares outstanding,
    shares float, short shares, short % of float, avg daily vol 20d,
    days to cover, reported short ratio, utilization proxy, squeeze
    risk label, note.
- **New compute fns:**
  - `compute_leverage_snapshot(symbol, as_of, statements, total_debt_fund,
    cash_fund)` — pure compute. Picks the latest annual balance sheet
    (falls back to quarterly, then to Fundamentals) for book debt and
    cash. Sums 4 most-recent quarterly EBITDA and interest expense for
    TTM roll-up. Computes Debt/EBITDA, Net Debt/EBITDA, Debt/Equity,
    Interest Coverage (operating income / interest expense), Current
    Ratio, Quick Ratio. Thresholds: Debt/EBITDA <2.5 HEALTHY, <4.0
    ELEVATED, else STRETCHED; Interest Coverage ≥5 HEALTHY, ≥2 ELEVATED,
    else STRETCHED; Current Ratio ≥1.5 HEALTHY, ≥1.0 ELEVATED; Quick
    Ratio ≥1.0 HEALTHY, ≥0.7 ELEVATED.
  - `compute_accruals_snapshot(symbol, as_of, statements)` — pure
    compute. Iterates up to 8 most-recent quarterly income rows and
    matches each to its cashflow row by date. Builds per-quarter cash
    conversion (FCF / NI × 100) with labels HIGH (≥90%), MEDIUM (≥60%),
    LOW (<60%), or NEGATIVE_NI when NI ≤0. TTM is the sum of the last 4
    periods. Trend label compares the recent-2 average to the older-2
    average: ±5% band = STABLE, above = IMPROVING, below = DETERIORATING,
    MIXED when signs diverge, INSUFFICIENT when <2 periods exist.
  - `compute_realized_vol_snapshot(symbol, as_of, bars_oldest_first,
    current_atm_iv_pct)` — pure compute. Requires ≥25 bars. Builds log
    returns, then for each window in {20d, 60d, 120d, 252d} rolls an
    annualized stdev × √252. Percentile = fraction of rolling-window
    observations strictly below the latest × 100. IV/RV gap and ratio
    use the 20d window. Regime label: `ratio < 0.95` → CHEAP_IV,
    `ratio > 1.15` → RICH_IV, else FAIR_IV, and NO_IV_REFERENCE when
    `current_atm_iv_pct == 0`.
  - `compute_fcf_yield_snapshot(symbol, as_of, statements, market_cap,
    stock_price)` — pure compute. Builds up to 5 annual rows + a TTM
    row from the last 4 quarters. Per period: payout-from-FCF = div /
    FCF × 100, payout-from-NI = div / NI × 100. TTM FCF yield = TTM
    FCF / market_cap × 100. 5Y CAGR computed only when ≥5 annual rows
    exist. Labels: NO_DIVIDEND when TTM dividends ≤0, UNSUSTAINABLE
    when TTM FCF ≤0 or TTM payout-from-FCF >100%, STRETCHED when >75%,
    SAFE otherwise.
  - `compute_short_interest_snapshot(symbol, as_of, shares_out, float,
    short_pct, short_ratio_reported, bars)` — pure compute. Derives
    `short_shares = float × (short_pct / 100)`. Average daily volume is
    the mean of the last 20 bars' volume. `days_to_cover = short_shares
    / avg_daily_volume_20d`. Squeeze risk: ≥30% short OR DTC ≥10 →
    EXTREME; ≥20% OR DTC ≥7 → HIGH; ≥10% OR DTC ≥4 → ELEVATED; else LOW;
    INSUFFICIENT_DATA when short_pct ≤0 or float ≤0.
- **Schema v10:** `create_research_tables_v10` adds `research_leverage`,
  `research_accruals`, `research_realized_vol`, `research_fcf_yield`,
  and `research_short_interest` (all per-symbol, JSON-blob column).
  Each table has an `updated_at` index for LAN-sync delta selection.
- **Upsert / get pairs:** five new `upsert_*` / `get_*` functions
  matching the Round 9 style (JSON blob column, unconditional upsert
  on conflict).
- **Tests:** 14 new tests (5 roundtrip + 9 compute):
  - `leverage_snapshot_roundtrip`, `accruals_snapshot_roundtrip`,
    `realized_vol_snapshot_roundtrip`, `fcf_yield_snapshot_roundtrip`,
    `short_interest_snapshot_roundtrip` — cache write/read symmetry.
  - `compute_leverage_on_healthy_statements` — synthetic statements
    with Debt/EBITDA = 700/1880 ≈ 0.372 produce the HEALTHY signal.
  - `compute_leverage_empty_statements_produces_note` — empty input
    produces the no-data note.
  - `compute_accruals_high_conversion_labels_high` — Q4 latest labeled
    HIGH (280/300 = 93.3%).
  - `compute_accruals_insufficient_periods_labels_insufficient` — 1
    period produces `trend_label == "INSUFFICIENT"`.
  - `compute_realized_vol_with_drift_produces_rich_regime` — 260 bars
    with 0.001 drift + IV 40% → RICH_IV or NO_IV_REFERENCE.
  - `compute_realized_vol_insufficient_bars_returns_note` — <25 bars
    → INSUFFICIENT_DATA.
  - `compute_fcf_yield_with_market_cap` — TTM FCF=1000, mcap=100K →
    yield=1.0%; payout 80/1000 = 8% → SAFE.
  - `compute_fcf_yield_no_market_cap_emits_note` — missing market cap
    populates the note.
  - `compute_short_interest_high_risk_squeeze` — 200M float × 25% = 50M
    short; 1K avg daily vol → DTC 50000 → EXTREME.
  - `compute_short_interest_no_shorts_insufficient` — short_pct=0 →
    INSUFFICIENT_DATA.

### LAN sync (`engine/src/core/lan_sync.rs`)

Added five entries to `SYNCABLE_TABLES`, five `CREATE TABLE` branches in
`create_table_sql()`, and five `updated_at` mappings in
`table_timestamp_column()`. Schema v10 tables replicate across TyphooN
nodes using the same delta protocol as Round 6/7/8/9.

### Native app (`native/src/app.rs`)

Following the Round 8/9 surface-addition protocol verbatim:

- **5 new `BrokerCmd` variants:** `ComputeLeverageSnapshot`,
  `ComputeAccrualsSnapshot`, `ComputeRealizedVolSnapshot`,
  `ComputeFcfYieldSnapshot`, `ComputeShortInterestSnapshot`. Every
  compute runs on the broker thread; the RVOL and SHRT variants carry
  `bars_json: String` because `Vec<HistoricalPriceRow>` is already
  JSON-serializable and keeps the handler free of `&Connection` holds
  across `.await`. LEV/FCFY carry primitive `f64` values (total_debt,
  cash, market_cap, stock_price) pulled from Fundamentals on the main
  thread; the handler only reads cached `FA` statements on its own.
- **5 new `BrokerMsg` variants:** `LeverageSnapshotMsg`,
  `AccrualsSnapshotMsg`, `RealizedVolSnapshotMsg`, `FcfYieldSnapshotMsg`,
  `ShortInterestSnapshotMsg`.
- **5 new `TyphooNApp` state fields** (`show_*`, `*_symbol`,
  `*_snapshot`, `*_loading`) plus Round 10 default initializers.
- **5 new broker handlers** on `tokio::spawn`:
  - `ComputeLeverageSnapshot` — reads cached `FA` via `get_financials`,
    calls `compute_leverage_snapshot` with the Fundamentals-sourced
    debt/cash fallbacks.
  - `ComputeAccrualsSnapshot` — reads `FA`, calls
    `compute_accruals_snapshot`. No external dependencies.
  - `ComputeRealizedVolSnapshot` — deserializes `bars_json` and calls
    `compute_realized_vol_snapshot`. Handler unwraps
    `Option<f64>` ATM IV to 0.0 (the compute treats 0.0 as
    NO_IV_REFERENCE).
  - `ComputeFcfYieldSnapshot` — reads `FA`, calls
    `compute_fcf_yield_snapshot` with market cap / stock price pulled
    from the main thread.
  - `ComputeShortInterestSnapshot` — deserializes `bars_json` and calls
    `compute_short_interest_snapshot` with main-thread-sourced
    shares_out / float / short_pct / short_ratio.
- **5 new receive arms** pattern-matching each new `BrokerMsg`, guarding
  UI state by symbol match and upserting unconditionally to SQLite so
  LAN replication catches every compute.
- **5 new egui windows** (Round 8-style grids / scroll areas), each with
  Symbol / Use Chart / Load Cached / Compute controls. The LEV window
  renders a ratio grid with signal coloring (HEALTHY=UP, STRETCHED=DOWN).
  The ACRL window shows a per-quarter quality table. The RVOL window
  shows the 4-window cone. The FCFY window emits a key-value header
  plus per-period table. The SHRT window emits a key-value block with
  squeeze-risk coloring.
- **5 new palette entries:** `LEV / LEVERAGE / DEBT_LEVERAGE / SOLVENCY`,
  `ACRL / ACCRUALS / EARNINGS_QUALITY / FCF_QUALITY`,
  `RVOL / REALIZED_VOL / VOL_CONE / HV`,
  `FCFY / FCF_YIELD / PAYOUT / DIV_SUSTAINABILITY`,
  `SHRT / DTC / DAYS_TO_COVER / SHORT_FLOAT`. `SHORT_INTEREST` and
  `SQUEEZE` are intentionally **omitted** from the SHRT alias list
  because legacy commands already own those tokens (Finnhub legacy
  short-interest fetch + chart squeeze indicator).

### Research packet (`investigate_symbols`)

- **Per-symbol section:** adds five new sub-blocks after the SKEW
  snapshot:
  - LEV leverage summary (header + ratios grid).
  - ACRL earnings-quality summary (header + period grid capped at 8
    rows).
  - RVOL realized-vol cone (header + 4-row window grid).
  - FCFY dividend sustainability (header + period grid capped at 6
    rows).
  - SHRT short-interest key-value block.
- **Section counts updated:** "thirty-seven sub-blocks" → "forty-two".
  Size cap table gained five new rows (LEV ratios, ACRL periods, RVOL
  windows, FCFY periods, SHRT fields). Packet size estimate updated to
  16-32 KB single / 150-300 KB 10-symbol.

## Alternatives Considered

- **Shipping LEV / FCFY as live provider fetches.** Rejected: the
  underlying FinancialStatements bundle is already cached locally via
  `FA`, and recomputing locally lets the user see their own cut rather
  than whatever a vendor cached at a different refresh moment. Pure
  compute also avoids new API dependencies (same reasoning as Round 9).
- **Computing LEV against industry-specific thresholds.** Rejected for
  Round 10: the current cones are sector-agnostic and deliberately
  conservative. A future schema-additive round could overlay a
  sector-specific peer median column when we have enough sector-grouped
  fundamentals to compute robust medians.
- **Adding Altman Z-score and Piotroski F-score as LEV companions.**
  Held for a future round: Z-score needs working capital / total
  assets and retained earnings — values TyphooN doesn't yet parse from
  the FinancialStatements bundle. Additive.
- **Parkinson / Garman-Klass / Yang-Zhang estimators for RVOL.**
  Held for a future round: our HP bars already carry OHLC, so these
  estimators are pure-compute extensions. Round 10 ships the classical
  close-to-close realized vol to keep the window contract simple.
- **Proper exponentially-weighted rolling vol for RVOL.** Rejected:
  the equal-weight cone matches the Godel convention. EWMA can layer
  on as an additional window label later.
- **Fee-rate / borrow-cost column for SHRT.** Held for a future round:
  borrow costs aren't in the Fundamentals or SharesFloat caches.
  Additive when we add a borrow-rate source.

## Consequences

### Positive

- Five new research surfaces with **zero new API dependencies** — every
  compute reads from caches TyphooN already populates (FA, HP, IVOL,
  FLOAT, Fundamentals).
- The research packet now includes leverage and coverage ratios,
  earnings quality via NI-vs-FCF, a realized-vol cone with IV/RV gap,
  FCF yield with dividend sustainability, and short interest with
  days-to-cover — closing the capital-structure and cash-flow-quality
  layer in the Godel parity push.
- LAN-sync coverage is still 100% — any node that computes a Round 10
  surface replicates it to every peer via the standard delta protocol.
- Schema v10 migration is purely additive: existing `typhoon_cache.db`
  files create the new tables on first Round 10 invocation via
  `CREATE TABLE IF NOT EXISTS`. No data migration required.

### Neutral

- The FCFY window reads `market_cap` and `stock_price` from
  Fundamentals on the main thread before dispatching the compute.
  Missing Fundamentals produces a note in the snapshot rather than an
  error, and the packet sub-block is silently skipped when
  `periods.is_empty() && ttm_free_cash_flow == 0.0`.
- RVOL needs ≥25 cached HP bars for a meaningful 20d cone and ≥252 bars
  for the full cone. Missing-bar fallbacks emit an explicit note; the
  packet sub-block is silently skipped when `windows.is_empty()`.

### Negative

- The LEV compute uses book-value debt from the balance sheet. Market
  value of debt would be preferable for ratio analysis, but market debt
  is not exposed in our Fundamentals or FinancialStatements cache.
  Documented as a consequence, not a defect.
- The ACRL trend classification is a two-vs-two average comparison.
  On small samples (<4 quarters) it falls back to `INSUFFICIENT`. Not
  suited for tiny-history symbols (new IPOs, recent listings) — the
  packet sub-block is silently skipped in that case.
- The SHRT compute treats Fundamentals' `short_percent_of_float` as
  authoritative. When the Fundamentals row is stale (vendor refresh
  cadence is typically every ~2 weeks for short interest), the snapshot
  will be stale too. The packet sub-block shows the `as_of` date so
  downstream readers can judge freshness.

## Implementation Notes

- **Palette alias collisions.** The initial draft of the SHRT palette
  aliases included `"SHORT_INTEREST"` and `"SQUEEZE"`. Both tokens were
  already owned by earlier features (a Finnhub legacy short-interest
  BrokerCmd and a chart Squeeze indicator), so the compiler flagged
  both as unreachable patterns. Fixed by trimming the alias list to
  `"SHRT" | "DTC" | "DAYS_TO_COVER" | "SHORT_FLOAT"`, matching the
  Round 6/7/8/9 precedent of deferring to legacy tokens.
- **RVOL `Option<f64>` plumbing.** `compute_realized_vol_snapshot` takes
  `current_atm_iv_pct: f64` and treats 0.0 as NO_IV_REFERENCE. The
  BrokerCmd variant carries `Option<f64>` so the main thread can read
  the cached IVOL snapshot and pass `None` when IVOL has not been run.
  The handler unwraps with `.unwrap_or(0.0)` before calling compute —
  keeps the engine API scalar-only while still letting the UI
  distinguish between "IVOL never run" and "IVOL ran but was 0%".
- **IvolSnapshot field name.** The IVOL snapshot's current ATM IV is
  `current_atm_iv_pct`, not `atm_iv_now_pct`. The initial RVOL window
  referenced the wrong name and the compiler caught it — fixed in a
  follow-up edit.
- **FA fallback chain for LEV.** The LEV compute tries in order:
  `statements.balance_annual.first()`, then `balance_quarterly.first()`,
  then the Fundamentals-sourced `total_debt_fund` / `cash_fund`. This
  matches the DCF helper's fallback order from Round 8 and avoids a
  no-data path when only quarterly or only Fundamentals is populated.
- **Main-thread reads vs broker-thread reads.** For LEV / ACRL / FCFY,
  the handler reads `FA` on the broker thread (FinancialStatements
  bundle is a single SQLite row). For RVOL / SHRT, the main thread
  pre-serializes HP bars to JSON before dispatch (because the UI
  already did a reverse-oldest-first pass, so it's cheaper to reuse
  that than to re-read on the broker). Both patterns are Send-safe.

## Tests

All existing tests still pass. Round 10 adds 14 new tests, bringing the
engine library suite to **670 tests, 0 failures, 3 ignored**. (Reported
run: 671 passing — the extra one is a pre-existing Round 9 test that
was added after the Round 9 commit.)

Key new tests:

- `leverage_snapshot_roundtrip` — cache write/read symmetry.
- `compute_leverage_on_healthy_statements` — Debt/EBITDA 700/1880 ≈
  0.372 → HEALTHY signal.
- `compute_accruals_high_conversion_labels_high` — Q4 cash conversion
  93.3% → HIGH quality label.
- `compute_realized_vol_with_drift_produces_rich_regime` — 260-bar
  series with 0.001 drift + IV 40% exercises the CHEAP_IV / RICH_IV /
  NO_IV_REFERENCE branch path.
- `compute_fcf_yield_with_market_cap` — TTM FCF 1000, market cap 100K →
  FCF yield 1.0%, payout 8% → SAFE label.
- `compute_short_interest_high_risk_squeeze` — 50M short + 1K ADV →
  DTC 50000 → EXTREME label.

## Future Work

- **Altman Z-score / Piotroski F-score companion metrics for LEV.**
  Needs retained earnings, working capital, and a few additional
  balance-sheet fields parsed into the FinancialStatements bundle.
  Additive.
- **Parkinson / Garman-Klass / Yang-Zhang estimators for RVOL.**
  All pure-compute extensions over the cached OHLC bars; adds rows to
  the window table without touching the schema.
- **Sector-specific LEV thresholds.** Overlay a peer-median column on
  each ratio row once we have sector-grouped Fundamentals with enough
  density to compute robust medians. Additive.
- **Borrow-rate / fee column for SHRT.** Would need a borrow-rate
  source (IBKR, Interactive Data, or similar). Additive when the
  source lands.
- **Round 11 candidates.** Remaining gaps versus Godel: ESG score
  deltas over time, analyst-driven price target dispersion histogram,
  options dark-pool / unusual-activity flow, insider trade aggregate
  dollar-value over rolling windows. Most are additive over existing
  caches — follows the Round 10 pure-compute pattern.
