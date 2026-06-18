# ADR-078: Multi-source News Ingest Pipeline

**Status:** Implemented
**Date:** 2026-04-13

## Context

The prior news flow bound the "News & Events" window to a single provider,
Finnhub's `/stock/company-news` endpoint. Finnhub's free tier is sparse for any
ticker outside the US large-cap universe and returns nothing at all for the
Darwinex/MT5 symbols the terminal actually trades. The user's bar for this
feature is "rival TradingView, which itself was inferior to external terminal" —
a single-source, US-only pane does not clear it.

The requirement is:

1. Aggregate news from multiple free sources so the union of their coverage
   reaches every MT5/Darwinex symbol the terminal tracks.
2. Cache the result in SQLite so repeated opens of the window are instant and
   so the LAN sync server can replicate it to clients.
3. Replace the flat headline list with a two-pane reader (list → body) that
   mirrors the existing SEC filing viewer pattern.
4. Make it practical to bulk-scrape the configured source news universe in a
   single background operation, including Kraken market-data symbols when that
   source is enabled.

## Decision

Introduce `engine/src/core/news.rs` as a standalone module analogous to
`core/research.rs`: typed articles, pure async fetchers, sync SQLite upsert
helpers, and a fetch orchestrator for bulk scraping.

### Sources

All free tier, selected for non-overlapping coverage:

| Source             | Key? | Limit     | Coverage notes                           |
|--------------------|------|-----------|------------------------------------------|
| GDELT 2.0 Doc API  | No   | Unkeyed / cooldown-gated | Global web/news search by ticker or crypto asset name |
| Yahoo Finance RSS  | No   | Unkeyed   | Per-symbol feed, US + major international; returns zero for many wrappers/thin names |
| Marketaux          | Yes  | 100/day   | Finance-focused, API-supplied sentiment  |
| Alpha Vantage      | Yes  | 25/day    | Ticker-resolved sentiment + topic tags   |
| FMP stock_news     | Yes  | 250/day   | Clean normalized shape, images, summary  |
| Finnhub            | Yes  | 60/min free tier | Legacy single-symbol equity fetch plus crypto news when explicitly configured |

GDELT + Yahoo give the zero-key baseline. The paid-but-free-tier sources stack
on top when the user configures them, without altering the window behavior.
SEC EDGAR is intentionally not part of the general news pane anymore: filings
belong in the SEC/filings/research surfaces, not mixed into market headlines.

### Data Model

```rust
pub struct NewsArticle {
    pub url_hash: String,        // SHA-256 of lowercased URL (PK — dedups cross-source)
    pub symbol: String,          // primary ticker this row is associated with
    pub source: String,          // "GDELT" | "YahooRSS" | "Marketaux" | "AlphaVantage" | …
    pub provider: String,        // original publisher ("Reuters", "Bloomberg")
    pub headline: String,
    pub summary: String,
    pub url: String,
    pub published_at: i64,       // unix seconds
    pub image_url: String,
    pub sentiment: String,       // "bullish" | "bearish" | "neutral" | ""
    pub sentiment_score: f64,
    pub tickers: Vec<String>,
    pub categories: Vec<String>,
}
```

Dedup is by `url_hash`. An article syndicated across Yahoo + GDELT + FMP
collapses to one row; the upsert merges non-empty fields so the first source
to land the summary, image, or sentiment keeps them even if a later source
returns a sparser row.

### SQLite Schema

```sql
CREATE TABLE research_news (
    url_hash TEXT PRIMARY KEY,
    symbol TEXT NOT NULL DEFAULT '',
    source TEXT NOT NULL DEFAULT '',
    provider TEXT NOT NULL DEFAULT '',
    headline TEXT NOT NULL DEFAULT '',
    summary TEXT NOT NULL DEFAULT '',
    url TEXT NOT NULL DEFAULT '',
    published_at INTEGER NOT NULL DEFAULT 0,
    image_url TEXT NOT NULL DEFAULT '',
    sentiment TEXT NOT NULL DEFAULT '',
    sentiment_score REAL NOT NULL DEFAULT 0.0,
    tickers_json TEXT NOT NULL DEFAULT '[]',
    categories_json TEXT NOT NULL DEFAULT '[]',
    updated_at INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_research_news_sym_ts ON research_news(symbol, published_at DESC);
CREATE INDEX idx_research_news_updated ON research_news(updated_at);

CREATE VIRTUAL TABLE research_news_fts USING fts5(
    url_hash UNINDEXED, headline, summary, tokenize='porter unicode61'
);
```

The FTS5 virtual table mirrors `headline` + `summary` so the search box in the
reader can run `research_news_fts MATCH ?` joins across the entire cache in
O(log n) without scanning every row.

### Async / sync split

The first pass held `&rusqlite::Connection` across `await` points. That fails
the `tokio::spawn` `Send` bound — `Connection`'s internal `RefCell` statement
cache is `!Sync`. Split the orchestrator accordingly:

- `fetch_all_sources_for_symbol()` — async, no DB, returns `Vec<NewsArticle>`
- `upsert_news_batch()` — sync, takes `&Connection`

Callers on the tokio side do:

```rust
let articles = news::fetch_all_sources_for_symbol(&client, sym, …, cb).await?;
tokio::task::spawn_blocking(move || {
    let conn = cache.connection()?;
    news::upsert_news_batch(&conn, &articles)?;
    news::get_news_by_symbol(&conn, &sym, 200)
}).await??;
```

The existing `FundamentalsScrape` path does the whole thing on a dedicated
`std::thread` with a current-thread tokio runtime, which sidesteps the Send
bound entirely — the bulk `NewsScrapeAll` handler reuses that pattern.

### LAN Sync Whitelist

`research_news` is added to `engine/src/core/lan_sync.rs::SYNCABLE_TABLES`
and gets a `CREATE TABLE` clause in `create_table_sql()` plus an entry in
`table_timestamp_column()` (`updated_at`). The generic `RequestTableSync` path
replicates it the same way it already replicates `sec_filings` and
`fundamentals` — incremental by `updated_at` — so standalone clients pull a
fresh news mirror whenever the server runs a fresh scrape.

### Broker channel

Implemented `BrokerCmd` variants:

- `FetchNewsMulti { symbol, marketaux_key, alpha_vantage_key, fmp_key, finnhub_key, cryptopanic_key }` —
  on-demand single-symbol fetch from the reader window
- `LoadCachedNews { symbol, limit }` — cache-only read, no network; `symbol`
  may be empty for latest global rows, a single ticker, or symbol CSV such as
  `WOK,TNDM` for direct per-symbol reload after restart
- `SearchNews { query, limit }` — FTS5 search across the full cache
- `NewsScrapeAll { use_mt5, use_alpaca, use_tastytrade, use_kraken, …keys }` —
  bulk loop over the configured source news universe. Kraken symbols are
  derived from `kraken:*`, `kraken-equities:*`, and `kraken-futures:*` bar-cache
  keys and deduped before network fetch.
- `NewsScrapeSymbols { symbols, …keys }` — explicit deduped symbol list from
  MTF Grid / focused chart contexts

One new `BrokerMsg` variant: `NewsArticlesLoaded { symbol, articles }`.

### UI — two-pane reader

`show_news` replaces the old flat-list window with an SEC-viewer-style two-pane
layout:

- **Left pane** — scrollable list of cached articles for the current symbol
  (or for the FTS query). Each row shows source + provider + sentiment chip +
  headline + timestamp. Clicking selects the row.
- **Right pane** — full article body: headline, metadata line, sentiment
  badge, ticker tags, topic tags, summary text, "Open Source" button.

Controls above the panes:

- Symbol input (defaults to the active chart's ticker via a "Use Chart" button)
- "Load Cached" — pure cache read. With an empty Search field this loads the
  latest global rows; with symbol CSV in Search (`WOK,TNDM`) it loads those
  symbols directly from SQLite so older symbol-specific articles do not depend
  on being present in the latest global page.
- "Fetch All Sources" — single-symbol on-demand fetch
- "Scrape Scope" / "Scrape All" — bulk scrape for the configured/current
  source universe
- Right-panel "Fetch News (N MTF)" — deduped fetch over currently visible MTF
  Grid symbols only
- Search box — FTS5 query across all cached news
- Collapsible "API Keys (free tier)" section for Marketaux/Alpha Vantage/FMP

### Kraken equities / xStocks scope

Kraken equities/xStocks market-data coverage and news coverage are separate
measurements. The Sync Status total is a sum of `(symbol, timeframe)` cache
entries, not a unique-symbol count. For example, ~49k Kraken sync entries can be
only ~13k unique symbols once `M1`, `M5`, `M15`, `M30`, `H1`, `H4`, `D1`, `W1`,
and `MN1` rows are deduped.

The news bulk path now includes Kraken when `use_kraken` is enabled. It derives
candidate symbols from cached market-data keys under `kraken:*`,
`kraken-equities:*`, and `kraken-futures:*`, strips Kraken equity suffixes such
as `.EQ`, then dedupes with the MT5/Alpaca/tastytrade sets before network fetch.
This makes the scrape denominator a unique news-symbol count, not the bar-sync
entry count.

Current network news entry points are:

1. **Single active symbol** — the News & Research window / right panel fetches
   the current symbol.
2. **MTF Grid symbols** — when MTF Grid is active, the right-panel button calls
   `NewsScrapeSymbols` for the unique visible chart tickers. In the screenshot
   case that means names like `TNDM` and `WOK`; it does not mean all Kraken
   equities.
3. **Bulk scrape** — `NewsScrapeAll` gathers MT5 tickers, Alpaca tradable
   equities, tastytrade equity positions, and Kraken market-data cache symbols
   when those source checkboxes are enabled.

So log lines like `news/ACSV: 0 articles fetched` mean "the selected/focused
news symbol was queried and the enabled providers returned no headline rows."
They do **not** mean Kraken equities bar sync is only covering that subset.
The Sync Status window measures bar-cache coverage; the News pane measures
cached article rows for the active/focused symbols.

This partial coverage is expected with free providers. Many Kraken equities are
xStock wrappers, foreign wrappers, very thin tickers, or symbols with sparse
Yahoo/GDELT/Marketaux/AlphaVantage/FMP coverage. `research_news_scrape_index`
records the last scrape and article count per ticker; the UI skips a fresh
symbol only when the recent scrape produced at least one non-SEC article, so a
zero-result ticker can be retried manually without being hidden behind a stale
"fresh" marker.

Even with Kraken included, "all Kraken equities have article rows" is not a
provider guarantee. It only means every deduped Kraken candidate symbol is
attempted subject to freshness/rate gates. Full article coverage still depends
on the external providers resolving the ticker/wrapper name and returning rows;
that may require a paid/provider-specific policy later.

## Alternatives considered

- **Paid Bloomberg/Refinitiv/Benzinga feed.** Out of scope for a free terminal.
- **Single "best" source.** No free provider covers MT5/Darwinex symbols well.
  The union of GDELT + Yahoo plus optional keyed providers is the minimum that works.
- **Hold `&Connection` across await with `LocalSet`.** Workable but more
  complex than splitting fetch/persist, and it couples the module to a
  specific runtime topology.
- **Per-source SQLite tables.** Rejected — dedup by URL across sources is the
  whole point of having one table keyed by `url_hash`.
- **Polling loop.** Rejected — the bulk scrape is a manual button for now.
  An automatic hook alongside `FundamentalsScrape` is the natural next step
  once users confirm the scrape cadence they want.

## Consequences

**Positive:**

- Every MT5/Darwinex symbol can now attempt real news via GDELT + Yahoo RSS
  with optional keyed providers, breaking the Finnhub-only US-equity ceiling.
- Dedup-by-url means syndicated stories collapse to one row in the viewer.
- FTS5 search makes the whole cache queryable — the reader becomes a research
  tool, not just a latest-N feed.
- LAN sync replication means the cache server does the scraping work once and
  every client gets the deduped result.
- The fetch/persist split is a reusable pattern — next time we add an API that
  needs to cache to SQLite inside a tokio task, the template is here.

**Trade-offs:**

- Free tiers impose daily quotas (100/25/250) that a full universe scrape can
  exhaust on larger portfolios. The orchestrator sleeps between calls and
  skips missing keys, but a user running "Scrape All" on 500+ symbols will
  saturate Alpha Vantage within minutes. The UI surfaces this clearly by
  labeling each key with its free-tier cap.
- Atom/RSS parsing is regex-based (no XML-parser dep added). The feeds are
  fixed-format so this is fine in practice, but adding a real parser would be
  a first step if we extend to more providers with messier feeds.
- GDELT can return raw-only headlines with no summary — the UI shows "No
  summary — click Open Source" rather than guessing. The body hydrator
  (`news_ingest::hydrate_missing_bodies`) closes the gap by fetching and
  extracting the publisher's article text on a background loop; see
  ADR-100 for the DOM-aware extractor (scraper), hero-image rendering,
  and the explicit rejection of an embedded HTML/JS renderer.
- The existing Finnhub `news_articles` tuple state (and its web-protocol
  mirror) is retained for backward compatibility with the compact news side
  pane. Full-fat reader state lives in `news_full_articles`.
- Sentiment fields from different sources are not normalized to a common
  scale — Marketaux uses -1..1, Alpha Vantage uses categorical labels. The
  reader just shows whichever the source supplied; no cross-source averaging.

## Tests

15 new unit tests in `core::news::tests` covering:

- URL hash stability and case-insensitive normalization
- Upsert + get roundtrip
- Dedup-by-url_hash merge semantics
- FTS5 search across headline + summary
- Purge by cutoff timestamp
- Timestamp parsing (GDELT yyyymmddTHHMMSS, Alpha Vantage, RFC3339, RFC2822)
- HTML stripping + entity decoding
- RSS item and Atom entry extraction
- Symbol/news cache freshness gating
- Batch upsert counts

Engine test suite now at **577 passing** (was 565).

## Related

- ADR-073 — SEC filing expansion (pattern for in-terminal content viewer)
- ADR-076-105 — Deeper wiring passes (LAN sync whitelist precedent)
- `engine/src/core/research.rs` — Module layout template for this module
