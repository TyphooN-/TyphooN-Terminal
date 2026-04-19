# ADR-110 — Godel Parity Round 3: Financials, Management, CFTC Positioning

**Status:** Implemented
**Date:** 2026-04-14

## Parity classification

| Feature | Godel documented? | TA-Lib primitive? | Research packet | egui popup | Chart overlay |
| --- | --- | --- | --- | --- | --- |
| FA (financial statements) | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| MGMT (executive officers) | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| COT (CFTC commitments of traders) | Yes | No | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** pure Godel-Terminal-documented research windows (fundamentals statements, management, CFTC positioning); no TA-Lib primitives in this round.

## Context

ADR-108 wired the first wave of Godel-parity research surfaces (DES, PEERS,
ERN, PRESS, SENTIMENT, TRANSCRIPTS, GLCO, IPO, TAS). ADR-109 closed the next
four gaps (DVD, EEB, UPDG, GY). What remained after Round 2 were three Godel
surfaces with no TyphooN equivalent — the ones a professional user reaches
for when the valuation ratios in DES or the consensus in EEB need to be
reconciled against the underlying statements:

1. **FA — Financial Statements.** ERN (ADR-108) shows historical EPS actuals
   vs estimates, and DES shows top-line ratios, but neither exposes the
   full income statement, balance sheet, or cash-flow statement. A PM
   asking "where is the gross margin compression coming from" or "how much
   of FCF is stock-based comp" had no TyphooN window to answer from.
2. **MGMT — Management / Executive Officers.** DES has the company
   description but no officer list. Godel's MGMT shows the named executive
   team with position, tenure, and compensation — the "who runs this
   place" surface.
3. **COT — CFTC Commitments of Traders.** The macro dashboard has GLCO
   commodity spots (ADR-108) and GY treasury curve (ADR-109) but no
   positioning data. COT is the canonical weekly read on non-commercial
   (speculator) vs commercial (hedger) net futures positioning — the
   question "is the market crowded long gold here" has no answer in
   TyphooN without this surface.

The user's bar remains "rival TradingView; TradingView was inferior to
Godel." Rounds 1–3 together now span the research window matrix a Godel
user would expect.

## Decision

Add three new Godel-parity windows following the exact ADR-107/108/109
pattern: typed data → research module fetcher → `BrokerCmd`/`BrokerMsg`
pair → SQLite cache + LAN sync whitelist → egui window render → command
palette entries → bulk-scrape integration.

### 1. New types in `engine/src/core/research.rs`

```rust
pub struct IncomeStatement {
    pub date: String, pub period: String, pub revenue: f64,
    pub cost_of_revenue: f64, pub gross_profit: f64, pub operating_income: f64,
    pub net_income: f64, pub eps: f64, pub eps_diluted: f64,
    pub ebitda: f64, pub rnd_expense: f64, pub sga_expense: f64,
}

pub struct BalanceSheet {
    pub date: String, pub period: String,
    pub cash_and_equivalents: f64, pub short_term_investments: f64,
    pub net_receivables: f64, pub inventory: f64, pub total_current_assets: f64,
    pub property_plant_equipment: f64, pub goodwill: f64, pub total_assets: f64,
    pub accounts_payable: f64, pub short_term_debt: f64,
    pub total_current_liabilities: f64, pub long_term_debt: f64,
    pub total_liabilities: f64, pub total_stockholders_equity: f64,
}

pub struct CashFlowStatement {
    pub date: String, pub period: String,
    pub net_income: f64, pub depreciation: f64, pub stock_based_comp: f64,
    pub change_in_working_capital: f64, pub operating_cash_flow: f64,
    pub capex: f64, pub free_cash_flow: f64,
    pub dividends_paid: f64, pub stock_repurchased: f64,
    pub net_change_in_cash: f64,
}

pub struct FinancialStatements {
    pub income_annual: Vec<IncomeStatement>,
    pub income_quarterly: Vec<IncomeStatement>,
    pub balance_annual: Vec<BalanceSheet>,
    pub balance_quarterly: Vec<BalanceSheet>,
    pub cashflow_annual: Vec<CashFlowStatement>,
    pub cashflow_quarterly: Vec<CashFlowStatement>,
}

pub struct Executive {
    pub name: String, pub position: String,
    pub age: i32, pub sex: String, pub since: String,
    pub compensation: f64, pub year: i32,
}

pub struct CotReport {
    pub market_name: String, pub market_code: String, pub report_date: String,
    pub open_interest: f64,
    pub noncomm_long: f64, pub noncomm_short: f64, pub noncomm_spreads: f64,
    pub comm_long: f64, pub comm_short: f64,
    pub nonrept_long: f64, pub nonrept_short: f64,
    pub noncomm_net: f64, pub noncomm_net_change: f64,
}
```

All `#[derive(Default, Serialize, Deserialize)]` — same roundtripping
story as ADR-108/109 types. `FinancialStatements` is the unified bundle
holding all six FMP pulls in a single JSON blob per symbol.

### 2. New fetchers

| Fn | Endpoint | Free-tier | Notes |
|---|---|---|---|
| `fetch_fmp_income_statement` | `/api/v3/income-statement/{sym}?period={annual\|quarter}` | 250/day | One call per period |
| `fetch_fmp_balance_sheet` | `/api/v3/balance-sheet-statement/{sym}?period=…` | 250/day | One call per period |
| `fetch_fmp_cash_flow` | `/api/v3/cash-flow-statement/{sym}?period=…` | 250/day | One call per period |
| `fetch_fmp_financial_bundle` | Calls all 3 × 2 periods | 6 FMP calls | Internal 400ms sleeps between calls |
| `fetch_finnhub_executives` | `/api/v1/stock/executive` | 60/min | Returns name/position/age/sex/since/comp |
| `fetch_cftc_cot` | `publicreporting.cftc.gov/resource/6dca-aqww.json` | Unlimited | Public Socrata endpoint, no key |

`fetch_fmp_financial_bundle` is the one the UI and bulk scrape actually
call — it assembles a full `FinancialStatements` struct from six sub-calls
with 400 ms cooldowns, mirroring ADR-108's approach to FMP rate limits.
Sub-calls use `unwrap_or_default()` so a partial fetch (e.g. one period
404s) doesn't collapse the whole bundle.

`fetch_cftc_cot` uses the CFTC's public Socrata JSON feed with
`$order=report_date_as_yyyy_mm_dd DESC` and `$limit=2000`. It finds the
latest report date in the payload, scans earlier rows for the same market
to compute the prior-week non-commercial net, and emits latest-week rows
with a derived `noncomm_net_change` WoW delta. The "noncomm_postions_spread_all"
column name keeps the original CFTC typo that Socrata echoes through
verbatim.

### 3. SQLite schema (`create_research_tables_v3`)

```sql
CREATE TABLE IF NOT EXISTS research_financials (
    symbol TEXT PRIMARY KEY,
    bundle_json TEXT NOT NULL DEFAULT '{}',
    updated_at INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS research_executives (
    symbol TEXT PRIMARY KEY,
    rows_json TEXT NOT NULL DEFAULT '[]',
    updated_at INTEGER NOT NULL DEFAULT 0
);
```

`research_financials` stores the full six-statement bundle in a single
`bundle_json` column, keeping one row per symbol. This intentionally
differs from splitting into three tables — the struct is atomic (the FA
window always loads the whole bundle at once) and a three-table join would
just add schema surface for no caller benefit. `research_executives`
follows the standard `rows_json` blob-per-symbol shape introduced in
ADR-109 for DVD/EEB/UPDG.

COT is **not** persisted. The payload is ~1000 market rows that re-fetch
in ~1.5 s from the free Socrata endpoint, refreshes weekly (Fridays), and
caching would just add staleness to a global indicator that loses meaning
the moment it goes cold — same reasoning as the GY treasury curve
(ADR-109 §3). The UI shows a "Last fetched" timestamp next to the Fetch
button instead.

### 4. `BrokerCmd` / `BrokerMsg`

```rust
// cmd
FetchFinancialStatements { symbol: String, fmp_key: String },
FetchExecutives          { symbol: String, finnhub_key: String },
FetchCotReports,

// msg
FinancialStatementsMsg(String, FinancialStatements),
Executives(String, Vec<Executive>),
CotReports(Vec<CotReport>),
```

Handlers use the ADR-107 async/sync split — `tokio::spawn` the fetch,
emit the `BrokerMsg`, the `update()` loop both updates in-memory state
and calls the sync upsert helper under `cache.connection()`. No
`&Connection` crosses an `.await` boundary. `FetchFinancialStatements`
is the single-cmd wrapper over `fetch_fmp_financial_bundle`, so a user
clicking the Fetch button pays one 6-call roundtrip (~2.5 s total) rather
than six separate commands.

### 5. UI — three new egui windows

Each mirrors the ADR-108/109 top-bar layout: **Symbol input / Use Chart /
Load Cached / Fetch / Status**. Data density differs per window:

| Window | Size | Layout |
|---|---|---|
| **FA** | 960×580 | Top bar + view tabs (Income / Balance / CashFlow) + period tabs (Annual / Quarterly). Body is a 9-column grid per statement type, first column is line-item label, next 8 are fiscal periods (most recent left). B/M/K money formatter. |
| **MGMT** | 720×440 | Top bar + aggregate total-comp header. 6-column grid: Name / Position / Age / Sex / Since / Compensation. Missing ages/compensation render as `—`. |
| **COT** | 920×560 | Top bar + filter textbox (`e.g. GOLD, CRUDE, S&P`) + latest-week header. 7-column grid: Market / Open Int / NC Long / NC Short / NC Net / Δ Net / Comm Net. Δ Net color-coded by sign. |

COT's `Symbol / Use Chart` controls are omitted — positioning data is
global, not per-symbol. A single `Fetch` button replaces the per-symbol
controls. FA's view+period tabs are 2×3 = 6 render combinations driven
by two enum fields (`FinancialsView`, `FinancialsPeriod`) stored on
`TyphooNApp`; a single match in the window body picks the right slice of
the bundle to render.

### 6. Command palette entries

Added to the string-match dispatcher after the existing GY arm:

```
FA   | FINANCIALS | INCOME | BALANCE | CASHFLOW → open FA, fetch bundle, set view
MGMT | MANAGEMENT | OFFICERS | EXECUTIVES      → open MGMT, fetch executives
COT  | COMMITMENTS | POSITIONING                → open COT, fetch CFTC report
```

The FA arm inspects `cmd_upper.as_str()` so `INCOME` opens the window on
the Income tab, `BALANCE` on the Balance tab, `CASHFLOW` on the Cash Flow
tab, and plain `FA` / `FINANCIALS` defaults to Income. Period defaults to
Annual on first open and persists between subsequent opens.

### 7. Bulk scrape integration

`scrape_and_cache_symbol` gets two new calls tacked onto its existing
ordering:

- FMP block: adds `fetch_fmp_financial_bundle` after the rating-changes
  call, with the standard 400 ms cooldown after. **Cost: +6 FMP calls
  per symbol = +2.4 s.**
- Finnhub block (already runs profile/peers/earnings/press/sentiment):
  adds `fetch_finnhub_executives` at the end with a 1100 ms sleep,
  matching the other Finnhub-rate-limited calls. **Cost: +1 Finnhub call
  per symbol = +1.1 s.**

COT is not part of the per-symbol sweep — it's a one-shot global fetch
that would run once at the start of a `RESEARCH_SCRAPE` run (not wired
yet; low priority since the manual `COT` command is cheap enough to
invoke directly).

Per-symbol sweep floor goes from ~60 min (ADR-109) to ~65 min for a
500-ticker run. Within tolerance.

### 8. LAN sync whitelist

`engine/src/core/lan_sync.rs::SYNCABLE_TABLES` gets `research_financials`
and `research_executives`. `create_table_sql` gains matching
`CREATE TABLE IF NOT EXISTS` clauses so a fresh client can materialize
the schema before its first sync pull. `table_timestamp_column` maps
both to `updated_at`. `research_cot` is **omitted** by design — see
section 3 ("not persisted").

## Alternatives considered

- **Split `FinancialStatements` into three SQLite tables** (income /
  balance / cashflow). Rejected — the struct is always loaded and
  rendered atomically, a three-table schema would add join surface with
  no caller using the separation. The blob approach also makes the LAN
  sync wire shape identical to DVD/EEB/UPDG.
- **Persist COT weekly snapshots.** Rejected — the full feed is ~1000
  rows and refreshes on a known weekly cadence (Fridays), re-fetch is
  under 2 s, and caching would let stale WoW deltas drift out of sync
  with the CFTC site. The "Last fetched" timestamp in the UI gives the
  staleness signal a DB write would otherwise provide.
- **Use Finnhub `/stock/financials-reported` instead of FMP** for FA.
  Rejected — Finnhub returns raw 10-K/10-Q XBRL nodes (hundreds of
  atomic line items per statement with inconsistent labels between
  issuers), while FMP normalizes to a canonical schema. Rendering raw
  XBRL would have required a separate taxonomy layer and still wouldn't
  match Godel's "Revenue / COGS / Gross Profit" presentation.
- **Include COT in `RESEARCH_SCRAPE`'s auto-fetch.** Deferred — the
  per-symbol sweep is already the long pole on bulk scrape wall time,
  and COT is a global single call that doesn't benefit from being
  folded into a per-symbol loop. A future `MACRO_SCRAPE` or similar
  one-shot command is a better home.
- **Extend ERN (ADR-108) with FA data.** Rejected — ERN is historical
  EPS vs estimates at the quarterly level. FA spans three statement
  types, two periods, and 8+ fiscal periods of history. Collapsing
  them would bury the FA content inside a narrower window.

## Consequences

**Positive:**

- Three more Godel-parity surfaces land with the same data-flow shape
  introduced in ADR-107/108/109 — the `core/research.rs` module now
  owns 13 research types with a single test suite and a single bulk
  scrape entry point.
- The COT window gives the macro dashboard its first positioning read
  — answers "is this move crowded" at a glance, which GLCO and GY
  deliberately don't.
- FA's bundle approach means a user hitting the FA window once pays
  six FMP calls upfront; switching view/period tabs afterwards is
  instant in-memory reads off the bundle.
- MGMT reuses the existing Finnhub key already wired for DES/PEERS/
  ERN/PRESS/SENTIMENT — zero new provider onboarding.
- Research test count for the module: **17 passing** (was 11).

**Trade-offs:**

- FA bulk scrape adds ~2.4 s per symbol (6 FMP calls × 400 ms). For a
  500-ticker sweep this pushes wall time from ~60 min to ~65 min.
  Acceptable; bulk scrape is overnight work.
- MGMT compensation rows rely on the "most recent year" filter that
  Finnhub's endpoint returns — there's no UI toggle to see historical
  comp trajectory. If a user wants "how much did the CEO make in
  2022", they need the 10-K directly. Out of scope for v1.
- COT's weekly refresh cadence means the "Last fetched" timestamp can
  lag the actual CFTC release. The window shows the report date
  explicitly in a header row, so the lag is visible — but there's no
  auto-refresh on Friday evenings. A user who wants fresh COT has to
  hit Fetch. Acceptable v1.
- FA's 8-period cap on the grid means very long histories are
  truncated at 8 fiscal periods. FMP returns up to 40 annual periods
  on the free tier; the extras are fetched and cached but not
  rendered. Could add pagination in v2.
- The Socrata "noncomm_postions_spread_all" typo is a real upstream
  wart — if the CFTC ever fixes it, the fetcher will silently return
  0.0 for that field. Low-risk because the column isn't load-bearing
  for the UI (the NC Net calc uses long − short, not spreads).

## Tests

6 new unit tests in `core::research::tests`:

- `financials_bundle_default_is_empty` — default struct has all six
  Vecs empty
- `financials_bundle_roundtrip` — AAPL FY24 record roundtrips through
  `upsert_financials` / `get_financials` preserving revenue/EPS/FCF
- `financials_upsert_replaces` — second upsert with different bundle
  overwrites, does not append
- `executive_roundtrip` — Tim Cook / Luca Maestri records roundtrip
  with name/position/compensation preserved
- `cot_report_default_is_empty` — default `CotReport` has 0.0 fields
- `cot_report_net_math` — `noncomm_long − noncomm_short` matches
  `noncomm_net`

Engine research module test count: **17 passing** (was 11).

## Related

- ADR-107 — Multi-source news ingest (async/sync split pattern)
- ADR-108 — Godel parity round 1: DES/PEERS/ERN/PRESS/SENTIMENT/TRANSCRIPTS/GLCO/IPO/TAS
- ADR-109 — Godel parity round 2: DVD/EEB/UPDG/GY
- `engine/src/core/research.rs` — fetchers, types, SQLite helpers
- `engine/src/core/lan_sync.rs::SYNCABLE_TABLES` — LAN sync whitelist
- `native/src/app.rs::update()` — command palette dispatch and window render
