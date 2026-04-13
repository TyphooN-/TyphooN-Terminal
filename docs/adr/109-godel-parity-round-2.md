# ADR-109 — Godel Parity Round 2: Dividends, Forward Earnings, Ratings, Treasury Curve

**Status:** Implemented
**Date:** 2026-04-13

## Context

ADR-108 wired the first wave of Godel-Terminal research windows (DES, PEERS,
ERN, PRESS, SENTIMENT, TRANSCRIPTS, GLCO, IPO, TAS). The scoping pass for this
round discovered that the cache/UI layer already handled analyst ratings and
insider activity — `self.bg.insider_trades` feeds the INSIDER window and
`BrokerCmd::GetAnalyst` + `self.analyst_result` backs the ANALYST window —
so the initial "wire analyst + insider" line items were **false positives**
and were removed from the task list.

The genuine gaps were four Godel surfaces with no TyphooN equivalent:

1. **DVD — Dividend History.** TyphooN had an upcoming dividend *calendar*
   (`DIVIDENDS`) but no per-symbol historical payment ladder. A trader
   evaluating CC or NCLH can't see whether they cut, paused, or grew the
   payout over the last 5 years.
2. **EEB — Forward Earnings Estimates.** The existing ERN window shows
   *historical* actuals vs estimates (ADR-108). There was no analyst
   consensus view of the *next* 4 quarters, which is the standard PM
   question "does the street think this quarter's number will land."
3. **UPDG — Upgrades / Downgrades.** The `ANALYST` window shows current
   consensus ratings but not the stream of *changes*. Godel's UPDG has
   "Morgan Stanley upgraded from Equal-Weight to Overweight, PT $52" —
   TyphooN had no equivalent.
4. **GY — Treasury Yield Curve.** The macro dashboard had no at-a-glance
   US Treasury curve snapshot. GLCO (ADR-108) covers commodities via a
   Yahoo batch quote call; treasuries needed the same treatment.

The user's bar remains "rival TradingView; TradingView was inferior to Godel"
— these four surfaces complete the research window matrix that a Godel user
would expect to find when they log in.

## Decision

Add four new Godel-parity windows following the exact ADR-107/108 pattern:
typed data → research module fetcher → `BrokerCmd`/`BrokerMsg` pair → SQLite
cache + LAN sync whitelist → egui window render → command palette entries →
bulk-scrape integration.

### 1. New types in `engine/src/core/research.rs`

```rust
pub struct DividendRecord {
    pub ex_date: String,
    pub pay_date: String,
    pub record_date: String,
    pub declaration_date: String,
    pub amount: f64,
    pub adjusted_amount: f64,
    pub label: String,               // "Regular Cash", "Special", …
}

pub struct EarningsEstimate {
    pub date: String,                // period end YYYY-MM-DD
    pub eps_avg: f64, pub eps_high: f64, pub eps_low: f64,
    pub revenue_avg: f64, pub revenue_high: f64, pub revenue_low: f64,
    pub num_analysts_eps: i32, pub num_analysts_rev: i32,
}

pub struct RatingChange {
    pub date: String,
    pub symbol: String, pub company: String, pub firm: String,
    pub action: String,              // upgrade | downgrade | initiation | maintain
    pub from_grade: String, pub to_grade: String,
    pub price_target: f64,
}

pub struct TreasuryYield {
    pub tenor: String,               // "13W" | "5Y" | "10Y" | "30Y"
    pub ticker: String,              // Yahoo tickers ^IRX / ^FVX / ^TNX / ^TYX
    pub yield_pct: f64,
    pub change: f64,
    pub change_pct: f64,
}

pub const TREASURY_TENORS: &[(&str, &str)] = &[
    ("^IRX", "13W"), ("^FVX", "5Y"), ("^TNX", "10Y"), ("^TYX", "30Y"),
];
```

All types are `#[derive(Default, Serialize, Deserialize)]` so they roundtrip
to JSON for the SQLite blob schema, the LAN sync payload, and the egui row
state identically.

### 2. New fetchers

| Fn | Endpoint | Free-tier | Notes |
|---|---|---|---|
| `fetch_fmp_dividend_history` | `/api/v3/historical-price-full/stock_dividend/{sym}` | 250/day | Parses `historical[]` array |
| `fetch_fmp_earnings_estimates` | `/api/v3/analyst-estimates/{sym}` | 250/day | Annual and quarterly |
| `fetch_fmp_rating_changes` | `/api/v4/upgrades-downgrades` (params `symbol={sym}`) | 250/day | v4 endpoint — auth via `apikey` query |
| `fetch_treasury_yields` | `/v7/finance/quote?symbols=^IRX,^FVX,^TNX,^TYX` | Unlimited | Reuses existing `fetch_yahoo_quotes` path |

The treasury fetcher deliberately has no API key requirement — it routes
through the same Yahoo batch quote call the GLCO commodities window already
uses. Each Yahoo row is mapped back to its tenor label via `TREASURY_TENORS`
and sorted in curve order (13W → 5Y → 10Y → 30Y) before emission.

### 3. SQLite schema (`create_research_tables_v2`)

```sql
CREATE TABLE IF NOT EXISTS research_dividends (
    symbol TEXT PRIMARY KEY,
    rows_json TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS research_earnings_estimates (
    symbol TEXT PRIMARY KEY,
    rows_json TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS research_rating_changes (
    symbol TEXT PRIMARY KEY,
    rows_json TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);
```

Treasury yields are **not** persisted — the full curve is 4 rows, refreshes
in ~200 ms from Yahoo, and caching would just add staleness to a market
indicator that loses meaning the moment it goes cold. The UI shows a "last
fetched" timestamp next to the Fetch button instead.

Schema follows the same JSON-blob-per-symbol shape as `research_profiles` /
`research_peers`: one row per symbol, full payload in `rows_json`, touch
`updated_at` on upsert. The `get_*` helpers return `Option<Vec<T>>` so the
caller can distinguish "no row" from "row with empty array."

### 4. `BrokerCmd` / `BrokerMsg`

```rust
// cmd
FetchDividendHistory    { symbol: String, fmp_key: String },
FetchEarningsEstimates  { symbol: String, fmp_key: String },
FetchRatingChanges      { symbol: String, fmp_key: String },
FetchTreasuryYields,

// msg
DividendHistory(String, Vec<DividendRecord>),
EarningsEstimates(String, Vec<EarningsEstimate>),
RatingChanges(String, Vec<RatingChange>),
TreasuryYields(Vec<TreasuryYield>),
```

Handlers follow the ADR-107 async/sync split: `tokio::spawn` the fetch,
send the result back via `BrokerMsg`, the `update()` message loop both
updates in-memory state and calls the sync upsert helper under
`cache.try_connection()`. No `&Connection` crosses an await.

### 5. UI — four new egui windows

Each window mirrors the ADR-108 top-bar layout: **Symbol input / Use Chart
/ Load Cached / Fetch / Status**. Data density differs per window:

| Window | Layout | Source |
|---|---|---|
| **DVD** | Single grid: Ex Date, Pay Date, Record, Declared, Amount, Adjusted, Label | `dividend_history[]` |
| **EEB** | Two sub-grids: next-4 quarters + next-4 years, each with EPS avg/low/high + Rev avg/low/high + #analysts | `earnings_estimates[]` |
| **UPDG** | Single grid: Date, Firm, Action, From → To, Price Target, Company | `rating_changes[]` |
| **GY** | 4-row card: Tenor / Ticker / Yield % / Δ / Δ % + "Last fetched" timestamp | `treasury_yields[]` |

GY's Symbol / Use Chart controls are omitted — the curve is global, not
per-symbol. A single `Fetch` button replaces the per-symbol controls.

### 6. Command palette entries

Added to the string-match dispatcher in `update()` after the existing TAS
arm:

```
DVD  | DIV_HISTORY | DIVIDEND_HISTORY → open + fetch dividend history
EEB  | ESTIMATES   | FORWARD_EARNINGS → open + fetch forward estimates
UPDG | UPGRADES    | DOWNGRADES | RATING_CHANGES → open + fetch rating changes
GY   | TREASURY    | YIELD_CURVE | YIELDS → open + fetch treasury curve
```

`DIVIDENDS` remains bound to the existing upcoming-dividend *calendar*
window, distinct from `DVD`'s per-symbol history.

### 7. Bulk scrape integration

`scrape_and_cache_symbol` gets three new FMP calls tacked onto the end of
its existing FMP block (transcripts → dividends → estimates → ratings),
each sleeping 400 ms between calls to respect the 250-call/day FMP free
tier. Treasury yields are deliberately **not** part of the per-symbol
sweep — they're a one-shot call that `RESEARCH_SCRAPE` can fire once at
the start of a run if needed (not wired yet; low priority).

Additional per-symbol cost: **3 × 400 ms = 1.2 s**. A 500-ticker sweep
previously took ~50 minutes for research; the new floor is ~60 minutes.
That's within the tolerance set in ADR-108.

### 8. LAN sync whitelist

`engine/src/core/lan_sync.rs::SYNCABLE_TABLES` extended with the three new
table names. `create_table_sql` gets matching `CREATE TABLE IF NOT EXISTS`
clauses so a fresh client can materialize the schema before its first
sync pull. `table_timestamp_column` maps all three to `updated_at`. Once
the server runs `RESEARCH_SCRAPE`, clients pull deduplicated deltas on the
next `RequestTableSync` round.

Treasury yields table is omitted from the whitelist by design — see
section 3 ("not persisted").

## Alternatives considered

- **Persist Treasury yields.** Rejected — the curve is 4 rows that re-fetch
  in ~200 ms, and cached stale yields are worse than "no yields yet." The
  UI's "Last fetched" timestamp gives the user the staleness signal they'd
  otherwise need a DB write for.
- **Merge DVD into the upcoming DIVIDENDS calendar window.** Rejected — the
  calendar is future-looking and global (Finnhub `/calendar/dividend`),
  while history is per-symbol and sourced from FMP. Different shapes,
  different refresh cadences.
- **Use Finnhub's `/stock/upgrade-downgrade` for UPDG.** Rejected — Finnhub's
  upgrade feed free-tier coverage is US-large-cap only and returns nothing
  for most MT5/Darwinex tickers. FMP v4 covers a broader universe on the
  same free-tier budget we already spend on transcripts.
- **Merge EEB into the existing ERN window.** Rejected — ADR-108's ERN
  window is historical actuals-vs-estimates for already-reported quarters.
  Forward estimates have a different primary axis (fiscal period) and a
  different row count convention (4 quarters + 4 years). Collapsing them
  into one grid made the window harder to read for both use cases.
- **Add analyst/insider windows in this ADR.** Rejected after the scoping
  agent's false-positive was caught — those windows already work.
  Re-confirmation that the existing code paths (`BrokerCmd::GetAnalyst`,
  `self.bg.insider_trades`) remain the canonical routes.

## Consequences

**Positive:**

- Four Godel-parity surfaces land in one pass with the same data-flow
  pattern as ADR-107/108, keeping the engine module footprint consistent
  (one `core/research.rs` handles all 11 research types).
- Three new SQLite tables replicate over LAN sync automatically — a
  standalone client hitting a running cache server sees DVD/EEB/UPDG data
  without ever making its own API calls.
- `RESEARCH_SCRAPE` now warms 11 research caches per symbol in a single
  sweep, up from 8.
- Treasury curve snapshot gives the macro dashboard a live read on the US
  risk-free rate without a new provider, new key, or new SQL schema.
- The scoping pass's false-positive on analyst/insider is now permanently
  documented in "Alternatives considered" — future parity passes won't
  re-suggest wiring work that already exists.

**Trade-offs:**

- FMP v4 endpoint (`upgrades-downgrades`) authenticates via `apikey=` query
  parameter instead of `Authorization` header. Means the key appears in
  the URL, and any request logging will capture it. Acceptable because
  (1) the client is single-user desktop, (2) the v3 `Authorization`
  header variant doesn't exist for this endpoint.
- Treasury yield refresh is manual (button click). Godel auto-refreshes
  the curve on a timer. Not implemented because auto-refresh needs a
  broker-channel tick that isn't wired yet, and a stale button is a fine
  v1 — users only check the curve intermittently anyway.
- EEB two-grid layout (quarters + years) is more vertical space than a
  single grid, but forward estimates genuinely have two axes and
  flattening them made the "is the Q4 number crashing" question harder
  to answer at a glance.
- The field-of-view gap between "DVD shows the last 10 years of payments"
  and "DIVIDENDS shows the next ex-dividend calendar" is real but
  deliberate — splitting backward and forward views mirrors how
  Bloomberg / Godel distinguish them, and merging them would have
  required a paginated split inside a single window.

## Tests

6 new unit tests in `core::research::tests`:

- `dividend_record_default` + upsert/get roundtrip
- `earnings_estimate_default` + upsert/get roundtrip
- `rating_change_default` + upsert/get roundtrip
- `treasury_tenors_cover_curve` — ensures all 4 tenors are present
- `treasury_yield_default` — ensures struct defaulting works
- `dividend_upsert_replaces` — second upsert with different rows overwrites,
  does not append

Engine test count for the research module: **11 passing** (was 5).

## Related

- ADR-107 — Multi-source news ingest (async/sync split pattern)
- ADR-108 — Godel parity round 1: research windows + bulk scrape
- `engine/src/core/research.rs` — fetchers, types, and SQLite helpers
- `engine/src/core/lan_sync.rs::SYNCABLE_TABLES` — LAN sync whitelist
- `native/src/app.rs::update()` — command palette dispatch and window render
