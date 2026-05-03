# ADR-113: Godel Parity Round 6 — WEI/MOV/INDU/CACS/WACC

## Status
Accepted — 2026-04-14

## Parity classification

| Feature | Godel documented? | TA-Lib primitive? | Research packet | egui popup | Chart overlay |
| --- | --- | --- | --- | --- | --- |
| WEI (world equity indices) | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| MOV (market movers) | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| INDU (sector performance) | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| CACS (corporate actions calendar) | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| WACC (cost of capital, CAPM) | Yes | No | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** pure Godel-Terminal-documented research surfaces (world indices, movers, sector perf, corporate actions calendar, cost of capital); no TA-Lib primitives in this round.

## Context

Round 5 (ADR-112) brought the per-symbol research surfaces to 25 commands
covering insider flow, institutional holders, float, daily-bar history, and
EPS surprise. After running the Godel gap audit again, a new cluster of
high-value surfaces surfaced — this time dominated by **market-wide context**
rather than per-symbol deep dives:

1. **WEI — World Equity Indices.** A cross-region dashboard of the major
   benchmarks (S&P 500, FTSE, DAX, Nikkei, Hang Seng, etc). The first
   question a PM asks on the morning call is "what did the world do
   overnight" — we did not have a single surface that answered it. The
   legacy "World Indices Dashboard" in the app uses ETF proxies (SPY, EFA,
   EEM) routed through the broker quote stream — useful, but it is not the
   same as a real index tape. WEI queries Yahoo's `/v7/finance/quote`
   endpoint directly against ^GSPC, ^N225, etc., so the numbers match what
   Bloomberg and Godel show.
2. **MOV — Market Movers.** Top gainers / losers / most actives for the US
   session. There is a legacy `MOVERS` command that hits the broker (Alpaca)
   for a single gainers list, but it is broker-dependent and coupled to the
   trading scope. MOV uses FMP's three dedicated endpoints to produce a full
   three-column view without needing a broker connection.
3. **INDU — Sector Performance.** The daily change of the eleven S&P
   sectors. Without this, the model has no way to answer "is energy weak
   today?" or "is the rotation into defensives?" except by re-computing it
   from individual tickers.
4. **CACS — Corporate Actions Calendar.** A single unified timeline of
   splits, dividends, earnings, and IPOs for a given ticker. The individual
   surfaces already exist (SPLT, DVD, ERN, IPO), but searching across them
   is tedious. CACS is purely a UI aggregator — no new fetch, no new cache
   — so its implementation cost is small but its UX payoff is outsized.
5. **WACC — Cost of Capital (CAPM).** The weighted-average cost of capital
   computed from live fundamentals + the current 10-year Treasury yield.
   Needed for DCF sanity checks, for judging whether a company's ROIC
   exceeds its cost of capital, and — most practically — as a plug figure
   when the model is asked to rough-estimate a fair value. The inputs
   (beta, market cap, total debt, interest expense, tax rate) all come from
   FMP's free-tier fundamentals endpoints; the risk-free rate is sourced
   from the in-memory treasury yields already populated by the existing
   `GY` command.

Three of the five (WEI/MOV/INDU) are **market-wide** surfaces rather than
per-symbol, which is a new pattern for the research module. They cache a
single latest-snapshot row using a `snapshot_key TEXT PRIMARY KEY` column
with a literal `'latest'` key, rather than the per-symbol keying used in
Rounds 1-5. CACS carries no cache at all. WACC is per-symbol and follows
the Round 5 keying.

## Decision

Ship the five surfaces as a single bundled round following the Round 5
playbook, with adjustments for the three global-snapshot tables:

- Typed structs in `engine/src/core/research.rs`:
  `WorldIndex`, `MarketMover`, `MarketMovers`, `SectorPerformance`,
  `WaccSnapshot`
- New constant `WORLD_INDICES_UNIVERSE` — 22 entries across
  Americas/EMEA/Asia-Pacific, used by the Yahoo fetcher and the
  `investigate_symbols()` packet extender
- New fetchers:
  - `fetch_world_indices` — wraps the existing `fetch_yahoo_quotes` helper
    against `WORLD_INDICES_UNIVERSE` (no API key required)
  - `fetch_fmp_market_movers` — hits FMP `/v3/stock_market/{gainers,
    losers,actives}` in sequence, bundles the three lists
  - `fetch_fmp_sector_performance` — parses FMP `/v3/sector-performance`
    (handles both f64 and `"1.23%"` string formats)
  - `compute_wacc_snapshot` — pure function implementing CAPM and the
    WACC formula, with special handling for zero-debt and tax-rate
    clamping
- SQLite schema v6 (`create_research_tables_v6`) with 4 new tables:
  `research_world_indices`, `research_market_movers`,
  `research_sector_performance`, `research_wacc`. The first three use
  `snapshot_key TEXT PRIMARY KEY` with literal `'latest'`, storing the
  full JSON payload in a `snapshot_json` column. `research_wacc` uses
  per-symbol keying. All four have `updated_at` indexes for LAN sync.
- `upsert_*` / `get_*` pairs for each of the four tables with round-trip
  tests
- `BrokerCmd::Fetch*` / `BrokerMsg::*Msg` pairs for the four fetch
  operations (CACS needs none — it reads from existing tables)
- One `tokio::spawn` handler per BrokerCmd in the broker loop
- Five new egui windows following the standard pattern; CACS is the first
  research window that is purely an aggregator over cached data from
  prior rounds
- Command palette entries for each surface, intentionally omitting
  `"INDICES"` and `"MOVERS"` aliases to preserve the legacy broker/ETF
  dashboard behavior (same precedent as Round 5 omitting `"HOLDERS"`)
- Research packet (`investigate_symbols()`) extended with:
  1. A new **Global Market Context** section at the top of the packet
     (one-time, not per-symbol) containing WEI / MOV / INDU
  2. A per-symbol **WACC Snapshot** sub-block (2.22 in the packet layout)
- LAN sync (`SYNCABLE_TABLES` + `create_table_sql()` +
  `table_timestamp_column()`) extended to cover all four new tables

## Alternatives Considered

**Replace the legacy World Indices Dashboard with WEI.** The existing
`show_world_indices` surface is an ETF-driven grid that routes SPY/QQQ/IWM/
EFA/EEM/TLT/etc. through the broker quote stream — useful because it gives
tradable proxies and integrates with position views. WEI, by contrast,
targets the real index tickers (^GSPC, ^N225, ^FTSE) for analytical purity.
Rather than rip out the legacy window, WEI uses its own state
(`show_wei` / `wei_indices` / `wei_region_filter`) and is bound to the
`WEI` / `GLOBAL_INDICES` aliases — the legacy `INDICES` / `WORLD_INDICES`
aliases still open the broker-driven grid.

**Replace the legacy MOVERS command with MOV.** Same rationale — the
legacy `MOVERS` command hits the broker and returns a single gainers list
scoped to whatever account is connected. MOV hits FMP for a full
three-list snapshot without requiring a broker connection and does not
depend on session state. Coexistence is cheap and avoids breaking
muscle memory.

**Integrate CACS as a new table with its own cache.** Considered and
rejected — all four event streams CACS aggregates (splits, dividends,
earnings, IPOs) are already cached by ADR-109 / ADR-111 / ADR-112. Adding
a CACS table would just duplicate that data, drift out of sync on refresh,
and increase the LAN sync surface area for no real benefit. CACS is
therefore a pure UI surface with zero incremental storage.

**Use a higher-tier FMP endpoint for WACC.** FMP has a `/v4/advanced_dcf`
endpoint that includes a pre-computed WACC. It is behind the paid plan,
and the inputs are a black box — we prefer the transparent CAPM pipeline
(user can see every input and reproduce the math). Our WACC window shows
each intermediate value so a careful user can trace exactly how the
number was built.

**Derive risk-free rate from the yield curve endpoint inside the broker
handler.** The broker handler is an async task that does not hold a
`Connection`, and treasury yields are stored in-memory only (never
persisted to SQLite). Rather than plumb the connection or persist yields
just to get Rf, the main thread reads the 10-year yield from
`self.treasury_yields` before dispatch and passes it as an argument on
`BrokerCmd::FetchWaccSnapshot`. Falls back to 4.5% if `GY` has not been
run yet.

## Consequences

**Positive**
- **30 total research windows** (up from 25). WEI/MOV/INDU give the model
  a regime-level view at the top of every research packet — one of the
  most common gaps in the prior packets was "what is the broader market
  doing today."
- CACS unifies the four corporate-event streams into one sorted timeline,
  so an analyst can see "next dividend + next earnings + last 2 splits"
  for any ticker in a single window without juggling four commands.
- WACC is the first **derived / computed** research surface — it does not
  have a direct Bloomberg-style single-endpoint backing; it synthesizes
  four FMP calls plus the yield curve cache into a single CAPM output.
  This sets the pattern for the future RV / SPLC / DDM surfaces called
  out in Round 5's deferred list.
- Global context is emitted **once** at the top of the packet regardless
  of the number of symbols requested. A 10-symbol packet pays the WEI/MOV/
  INDU overhead exactly once — roughly +2 KB total for huge analytical
  leverage.
- All four new tables are LAN-synced, so Claude/Gemini sessions running
  against a secondary node see the same cached market context.

**Negative**
- Incremental compile time bump (+~900 lines in `app.rs`, ~500 in
  `research.rs`, 10 new tests).
- The three snapshot tables are truly single-row — an over-eager user who
  runs WEI twice in a row will replace the first snapshot with the
  second. For current use this is fine (the UI always shows "last
  fetched"); a future round may want to keep a short history for
  intraday comparison.
- WACC is a point-in-time snapshot that silently goes stale as the market
  moves — the `as_of` label is the only signal to the user. We do not
  auto-refresh the cached WACC on quote updates. This is acceptable
  because the downstream use (plug figure into a DCF or sanity check) is
  already low-precision by nature, but it should be kept in mind.
- When GY has not been run, WACC falls back to a hard-coded 4.5% Rf. The
  window shows a tip telling the user to run GY first, but the first-run
  experience is still "numbers that look right but are slightly off."

## Implementation

### Engine (`engine/src/core/research.rs`)

- **New structs** (after `EarningsSurprise`, ~line 420):
  `WorldIndex`, `MarketMover`, `MarketMovers`, `SectorPerformance`,
  `WaccSnapshot`.
- **New constants**: `WORLD_INDICES_UNIVERSE` (22 entries),
  `DEFAULT_EQUITY_RISK_PREMIUM_PCT = 5.0`.
- **New fetchers**:
  - `fetch_world_indices(client)` — wraps `fetch_yahoo_quotes` and
    preserves the universe's declared order via a HashMap lookup
  - `parse_fmp_mover(e)` — helper that handles FMP's dual format
    (f64 or `"1.23%"` string) for `changesPercentage`
  - `fetch_fmp_market_movers(client, fmp_key)` — hits gainers / losers /
    actives in sequence, wraps them in `MarketMovers`
  - `fetch_fmp_sector_performance(client, fmp_key)` — parses
    string-formatted percentages
  - `compute_wacc_snapshot(symbol, as_of, beta, market_cap,
    risk_free_pct, total_debt, interest_expense, effective_tax_rate_pct)`
    — pure CAPM calculation, handles zero-debt edge case, clamps tax
    rate to [0, 60]%
- **Schema v6** (`create_research_tables_v6`) creates:
  - `research_world_indices` (snapshot_key, snapshot_json, updated_at)
  - `research_market_movers` (snapshot_key, snapshot_json, updated_at)
  - `research_sector_performance` (snapshot_key, snapshot_json, updated_at)
  - `research_wacc` (symbol PRIMARY KEY, snapshot_json, updated_at)
  Each has an `updated_at` index for incremental LAN sync.
- **10 new tests**:
  - `world_indices_universe_has_all_regions`
  - `world_indices_universe_has_sp500_and_nikkei`
  - `world_indices_roundtrip`
  - `world_indices_upsert_replaces`
  - `market_movers_roundtrip`
  - `sector_performance_roundtrip`
  - `wacc_compute_basic_calc` — validates CAPM for AAPL-like inputs
  - `wacc_handles_zero_debt` — debt_weight=0, wacc==Re
  - `wacc_roundtrip`
  - `fmp_mover_parses_string_percentage`

### LAN sync (`engine/src/core/lan_sync.rs`)

- `SYNCABLE_TABLES` extended with 4 new entries (guarded by
  `// ── ADR-113 Round 6 ──` comment).
- `create_table_sql()` gains 4 matching `CREATE TABLE IF NOT EXISTS`
  branches, three using `snapshot_key`, one using `symbol`.
- `table_timestamp_column()` maps all 4 to `"updated_at"`.

### Native (`native/src/app.rs`)

- **BrokerCmd**: `FetchWorldIndices`, `FetchMarketMovers { fmp_key }`,
  `FetchSectorPerformance { fmp_key }`, `FetchWaccSnapshot { symbol,
  fmp_key, risk_free_pct }`. (CACS needs none.)
- **BrokerMsg**: `WorldIndicesMsg`, `MarketMoversMsg`,
  `SectorPerformanceMsg`, `WaccSnapshotMsg(String, WaccSnapshot)`.
- **TyphooNApp state** (new fields — `show_wei` / `wei_*` to avoid
  collision with the legacy `show_world_indices`):
  - `show_wei`, `wei_indices`, `wei_loading`, `wei_region_filter`
  - `show_market_movers`, `market_movers`, `mov_loading`
  - `show_sector_perf`, `sector_perf`, `indu_loading`
  - `show_cacs`, `cacs_symbol` (no data field — UI-only aggregator)
  - `show_wacc`, `wacc_symbol`, `wacc_snapshot`, `wacc_loading`
- **Broker handlers**: 4 new `tokio::spawn` arms. The WACC handler
  sequentially hits 4 FMP endpoints (profile, key-metrics-ttm,
  income-statement, balance-sheet-statement) to source beta /
  market_cap / interest_expense / income_before_tax / income_tax /
  totalDebt, computes effective_tax_rate_pct with fallback to
  key-metrics-ttm or a 21% default, then calls
  `compute_wacc_snapshot()`. Uses `chrono::Utc::now().format("%Y-%m-%d")`
  for the as_of field.
- **Msg receive loop**: 4 new arms, each updating state and
  unconditionally upserting into SQLite for LAN replication.
- **Egui windows** (inserted after Round 5 EPS window):
  - `WEI — Global Equity Indices` (720×520) — Fetch + Load Cached +
    Region filter ComboBox (All / Americas / EMEA / Asia-Pacific),
    advancing / declining summary, 5-column grid (region / ticker /
    name / last / chg%).
  - `MOV — Market Movers` (860×540) — 3-column horizontal layout for
    Top Gainers / Losers / Most Active, each with a 4-column grid
    capped to 25 rows.
  - `INDU — Sector Performance` (520×420) — sorted high-to-low on
    change %, unicode bar chart using `█` scaled to the max absolute
    value, up / down / avg summary header.
  - `CACS — Corporate Actions Calendar` (760×520) — **zero-fetch**
    aggregator. Reads from the existing splits / dividends / earnings /
    IPO caches via `rx::get_stock_splits`, `rx::get_dividends`,
    `rx::get_earnings_surprises`, `rx::get_ipo_calendar`. Produces a
    unified Events timeline sorted by date desc with colored type tags
    (SPLIT / DIV / EARN / IPO).
  - `WACC — Cost of Capital (CAPM)` (560×480) — 12-field grid with a
    `fmt_money` helper (T/B/M scaling), prominent WACC % header in the
    top bar, CAPM formula footer, and a tip reminding the user to run
    `GY` first for accurate Rf sourcing.
- **Command palette** (new arms):
  - `WEI | GLOBAL_INDICES` — intentionally omits `INDICES` to preserve
    the legacy ETF dashboard
  - `MOV | GAINERS | LOSERS | ACTIVES` — intentionally omits `MOVERS`
    to preserve the legacy broker top-movers arm
  - `INDU | SECTOR | SECTORS | SECTOR_PERFORMANCE`
  - `CACS | CORP_ACTIONS | CORPORATE_ACTIONS | ACTIONS`
  - `WACC | COST_OF_CAPITAL | CAPM` — sources Rf from
    `self.treasury_yields.iter().find(|y| y.tenor == "10Y")` with a
    4.5% fallback when GY has not been run
- **`investigate_symbols()` research packet** extended:
  1. A new **Global Market Context** section emitted once at the top
     of the packet (before the per-symbol loop) containing
     `### World Equity Indices`, `### Market Movers (US)`, and
     `### Sector Performance` sub-blocks — each skipped silently when
     its cache is empty.
  2. A new per-symbol `### WACC Snapshot (CAPM, as of YYYY-MM-DD)`
     sub-block inside the per-symbol loop, added after the Round 5
     `### EPS Surprise History` block.

## Tests

- 10 new research-module tests added (34 → 44 total research tests).
- `cargo test -p typhoon-engine --lib` → 616 passed / 0 failed / 3
  ignored (was 606 — all prior tests still green).
- `cargo check -p typhoon-native` clean; no warnings.
- Hand-verified: all five new windows open from the command palette,
  pull cached data when available, fetch fresh data, and render without
  layout glitches. CACS correctly aggregates from the existing
  splits/dividends/earnings/IPO caches; WACC shows a plausible number
  for AAPL with GY populated.

## Related ADRs

- ADR-107 — news pipeline + initial research surfaces
- ADR-108 — Round 1 research windows
- ADR-109 — Round 2 (DVD, EEB, UPDG, GY)
- ADR-110 — Round 3 (FA, MGMT, COT)
- ADR-111 — Round 4 (SPLT, ETF, ANR, ESG, MEMB) + AI chat overhaul
- ADR-112 — Round 5 (INS, HDS, FLOAT, HP, EPS)

## Historical Follow-up Context

Explicitly deferred out of this round:

- **SHRT — short interest** (still blocked on a stable free data source;
  the FMP `/v4/stock-short-interest` endpoint coverage is spotty).
- **SECF — equity screener** (needs a filter-state DSL — too large for
  a bundled round).
- **RV / SPLC / DDM — derived analytics** that build on WACC. Now that
  the CAPM pipeline is in place, these become straightforward follow-ups
  (RV is comparable-company multiples against the subject's own metrics;
  SPLC is supply-chain graph; DDM is a Gordon Growth model that reuses
  the WACC cache as its discount rate).
- **WEI intraday refresh / history.** The current WEI implementation
  stores a single latest snapshot. A future pass could keep a rolling
  intraday series so the model can answer "how did the world look two
  hours ago vs. now."
