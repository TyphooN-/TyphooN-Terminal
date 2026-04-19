# ADR-112: Godel Parity Round 5 — INS/HDS/FLOAT/HP/EPS

## Status
Accepted — 2026-04-14

## Parity classification

| Feature | Godel documented? | TA-Lib primitive? | Research packet | egui popup | Chart overlay |
| --- | --- | --- | --- | --- | --- |
| INS (insider trades, Form 4) | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| HDS (13F institutional holders) | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| FLOAT (shares outstanding / free float) | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| HP (historical daily price table) | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| EPS (earnings surprise history) | Yes | No | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** pure Godel-Terminal-documented research windows (insider trades, 13F holders, float, historical OHLCV grid, EPS surprise); no TA-Lib primitives in this round.

## Context

Round 4 (ADR-111) brought the per-symbol research surfaces to 20 commands
covering corporate actions, holdings, ratings, ESG, and index membership.
Running a gap audit against the full Godel / Bloomberg command set surfaced
five high-value single-symbol features we did not yet have:

1. **INS** — SEC Form 4 insider trades. Who inside the company is buying or
   selling the stock, when, and at what size. A rolling picture of insider
   sentiment is one of the first questions an analyst asks when a name moves.
2. **HDS** — 13F-derived institutional holders. Top-N holders with QoQ share
   deltas. Essential for understanding passive/active ownership structure
   and detecting concentration risk.
3. **FLOAT** — Shares outstanding / free float snapshot. Needed to sanity
   check short interest ratios, compute true liquidity, and spot structural
   supply constraints (low-float runners).
4. **HP** — Historical daily price table (OHLCV). Not a chart — a raw grid
   with CSV export. Analysts consistently want this for audit trails,
   spreadsheet imports, and quick back-of-envelope calculations that don't
   warrant opening a charting surface.
5. **EPS** — Earnings surprise history. Quarterly actual-vs-estimate EPS
   with surprise percentage, beats/misses aggregate, and 8-quarter average.
   Complements the existing ERN (earnings history) and EEB (forward
   estimates) by focusing specifically on the delivery record.

All five are **FMP free-tier** data surfaces, so no new provider secrets,
rate-limit concerns, or licensing exposure. They all slot cleanly into the
existing per-symbol research pattern established across ADR-107→111.

## Decision

Ship the five surfaces as a single bundled round following the same playbook
as Round 4:

- One typed struct per surface in `engine/src/core/research.rs`
- One `pub async fn fetch_fmp_*` fetcher per surface (reqwest, JSON, typed
  parse, graceful empty-array handling)
- One new SQLite schema version (`create_research_tables_v5`) with 5 tables
- One `upsert_*` / `get_*` pair per surface with round-trip tests
- One `BrokerCmd::Fetch*` / `BrokerMsg::*Msg` pair per surface
- One spawned `tokio::spawn` handler per surface in the broker loop
- One egui window per surface with the standard pattern:
  Symbol field → Use Chart → Load Cached → Fetch button → table/grid render
- One command-palette match arm per surface with aliases
- Research packet (`investigate_symbols()`) extended with all 5 new
  sub-blocks so the AI assistants (ASKAI/ASKCLAUDE/ASKGEMINI) get the data
  automatically whenever a user references a symbol
- LAN sync (`SYNCABLE_TABLES` + `create_table_sql()` + `table_timestamp_column()`)
  extended to cover all 5 new tables

## Alternatives Considered

**Replace the legacy "HOLDERS" JSON-dump window with HDS.** The existing
`show_holders` surface is a debug-style textbox that just prints the raw
13F JSON from SEC EDGAR. Rather than rip it out (which touches unrelated
code paths), the new HDS surface uses its own state (`show_inst_holders` /
`inst_holders_*`) and is bound to the `HDS` / `INST` / `INSTITUTIONAL` /
`13F` aliases. The legacy `HOLDERS` command still opens the old viewer —
power users who prefer the raw JSON can keep using it, and the two
surfaces can coexist until a future cleanup round.

**Defer SHRT (short interest) to a later round.** FMP's free-tier short
interest coverage is spotty and requires combining multiple endpoints for
ratios, so rather than ship a half-working surface, we substituted HDS
(which uses FMP `/v3/institutional-holder/{symbol}` — a solidly supported
free endpoint) and left SHRT as a Round 6+ candidate once we pick a better
data source (likely FINRA via FMP `/v4/stock-short-interest` once the free
tier matures, or a Finnhub fallback).

**Integrate HP into the existing chart engine.** The charting surface is
optimized for pan/zoom/overlay — the user's ask here is specifically a
data-grid view with CSV export for spreadsheet import. Keeping it as a
separate `HP` window matches Godel's model and avoids contaminating the
chart state.

## Consequences

**Positive**
- 25 total research windows (up from 20), with a consistent UX shape.
- Analyst packet now includes insider flow, top holders, float, recent
  price history, and EPS surprise track record — a large step up in the
  raw material the AI assistants have to reason over.
- HP CSV export unblocks spreadsheet workflows without requiring CLI tools.
- All five new tables are LAN-synced, so Claude/Gemini sessions running
  against a secondary node see the same cached research.

**Negative**
- Incremental compile time bump (+~600 lines in `app.rs`, ~500 in
  `research.rs`). Still well inside the single-crate build envelope.
- The `insider_trades` packet block can be noisy for high-activity tickers
  (dozens of small awards per month). Capped to 8 rows in the packet and
  100 in the UI — acceptable first pass, may want filtering later.
- The `historical_price` SQLite row stores the full OHLCV JSON array for a
  symbol. A 1000-bar request for one ticker is ~100KB. Cache footprint is
  bounded by the number of distinct symbols a user investigates × the
  chosen bar limit. We are not sharding historical data across time ranges
  yet — the latest fetch just replaces the prior cached rows.

## Implementation

### Engine (`engine/src/core/research.rs`)

- **New structs** (lines ~366–420):
  `InsiderTrade`, `InstitutionalHolder`, `SharesFloat`,
  `HistoricalPriceRow`, `EarningsSurprise`.
- **New fetchers**:
  - `fetch_fmp_insider_trades` → `/v4/insider-trading?symbol={}&page=0`
  - `fetch_fmp_institutional_holders` → `/v3/institutional-holder/{}`
  - `fetch_fmp_shares_float` → `/v4/shares_float?symbol={}`
  - `fetch_fmp_historical_price` → `/v3/historical-price-full/{}` (with
    client-side `limit` applied to the `historical` array)
  - `fetch_fmp_earnings_surprises` → `/v3/earning_surprise/{}`
- **Schema v5** (`create_research_tables_v5`) creates:
  - `research_insider_trades`
  - `research_institutional_holders`
  - `research_shares_float` (stores snapshot as `snapshot_json`, not rows)
  - `research_historical_price`
  - `research_earnings_surprise`
  Each has an `updated_at` index for incremental LAN sync.
- **8 new tests**:
  - `insider_trade_default_is_empty`
  - `insider_trade_roundtrip`
  - `institutional_holder_roundtrip`
  - `shares_float_default_is_empty`
  - `shares_float_roundtrip`
  - `historical_price_roundtrip`
  - `earnings_surprise_roundtrip`
  - `earnings_surprise_upsert_replaces`

### LAN sync (`engine/src/core/lan_sync.rs`)

- `SYNCABLE_TABLES` extended with 5 new entries (guarded by
  `// ── ADR-112 Round 5 ──` comment).
- `create_table_sql()` gains 5 matching `CREATE TABLE IF NOT EXISTS`
  branches.
- `table_timestamp_column()` maps all 5 to `"updated_at"`.

### Native (`native/src/app.rs`)

- **BrokerCmd**: `FetchInsiderTrades`, `FetchInstitutionalHolders`,
  `FetchSharesFloat`, `FetchHistoricalPrice { limit }`,
  `FetchEarningsSurprises`.
- **BrokerMsg**: `InsiderTradesMsg`, `InstitutionalHoldersMsg`,
  `SharesFloatMsg`, `HistoricalPriceMsg`, `EarningsSurpriseMsg`.
- **TyphooNApp state** (new fields, wired in both the struct and the
  default constructor):
  - `show_insider_trades`, `insider_symbol`, `insider_trades`,
    `insider_loading`
  - `show_inst_holders`, `inst_holders_symbol`, `institutional_holders`,
    `inst_holders_loading`
  - `show_shares_float`, `float_symbol`, `shares_float`, `float_loading`
  - `show_hist_price`, `hp_symbol`, `hp_rows`, `hp_loading`, `hp_limit`
    (default 200)
  - `show_eps_surprise`, `eps_symbol`, `eps_surprises`, `eps_loading`
- **Broker handlers**: 5 new `tokio::spawn` arms, one per BrokerCmd,
  converting fetcher output into the matching BrokerMsg.
- **Msg receive loop**: 5 new arms, each updating state iff the
  symbol matches the currently-open window, and unconditionally upserting
  into SQLite for LAN replication.
- **Egui windows** (all inserted just after the Round 4 MEMB window):
  - `INS — Insider Trades` (820×480) — 7-column grid with net buy/sell
    summary header (Buys / Sells / Net in millions USD).
  - `HDS — Institutional Holders` (720×460) — 4-column grid with QoQ
    Δ coloring (green/red) and total shares header.
  - `FLOAT — Shares Outstanding` (460×260) — 2-column key/value grid
    showing symbol, as-of date, outstanding, float, free-float %, source.
  - `HP — Historical Price` (760×520) — 8-column OHLCV grid with
    Bars slider (30–1000), Copy CSV button that uses `ui.ctx().copy_text(...)`
    to place the full table into the clipboard in spreadsheet-ready format.
  - `EPS — Earnings Surprise` (640×420) — 5-column actual/estimate/
    surprise grid with beats/misses summary and rolling 8-quarter average
    surprise-% header.
- **Command palette** (new arms):
  - `INS | INSIDER_TRADES | FORM4`
  - `HDS | INST | INSTITUTIONAL | 13F` (intentionally omits `HOLDERS` to
    preserve the legacy JSON viewer)
  - `FLOAT | SHARES | OUTSTANDING`
  - `HP | HIST | HISTORICAL | PRICE_HISTORY`
  - `EPS | SURPRISE | EARNINGS_SURPRISE`
- **`investigate_symbols()` research packet** gains 5 new sub-blocks inside
  the per-symbol loop (after `### ESG Score`):
  1. `### Insider Flow (Form 4)` — aggregate buy/sell/net + last 8 filings
  2. `### Institutional Holders (13F)` — total + top 6 holders with QoQ Δ
  3. `### Shares Float` — outstanding/float/free-float% + source
  4. `### Recent Price History` — last 10 daily bars as a markdown table
  5. `### EPS Surprise History` — beats/misses + 8Q avg + last 8 quarters

## Tests

- 8 new research-module tests added (26 → 34 total research tests).
- `cargo test -p typhoon-engine --lib` → 606 passed / 0 failed / 3 ignored
  (was 598 passed — all prior tests still green).
- `cargo check --workspace` clean; no new warnings.
- Hand-verified: all five new windows open from the command palette, pull
  cached data, fetch fresh data, and render without layout glitches.

## Related ADRs

- ADR-107 — news pipeline + initial research surfaces
- ADR-108 — Round 1 research windows
- ADR-109 — Round 2 (DVD, EEB, UPDG, GY)
- ADR-110 — Round 3 (FA, MGMT, COT)
- ADR-111 — Round 4 (SPLT, ETF, ANR, ESG, MEMB) + AI chat overhaul

## Future Work

Explicitly deferred out of this round:

- **SHRT** — short interest / days-to-cover, blocked on picking a stable
  free data source.
- **WEI / MOV / INDU / CMOS** — multi-symbol dashboards (world equity
  indices, movers, sector performance, commodities). These share a common
  "grid of N quotes with refresh" UI pattern and will be grouped into
  their own round.
- **SECF** — equity screener (fundamental filters). Needs a filter-state
  DSL — too large for a bundled round.
- **WACC / RV / SPLC** — derived / multi-dataset analytics. Should wait
  until the raw data surfaces they depend on are all present.
