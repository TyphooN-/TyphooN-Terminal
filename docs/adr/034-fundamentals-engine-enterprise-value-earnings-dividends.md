# ADR-034: Fundamentals Engine (Enterprise Value, Earnings, Dividends)

**Status:** Implemented
**Date:** 2026-03-26 | **Updated:** 2026-04-08

## Context

The terminal needs comprehensive fundamentals data for all Darwinex MT5 symbols — Enterprise Value, earnings dates, dividend dates, quarterly financials, institutional holders, and company summaries. Previously this was handled by external Python scripts (`SECScrape/secscrape.py` and `SECScrape/evscrape.py`) that had to be run manually from the command line.

## Decision

Build `engine/src/core/fundamentals.rs` — a Rust module that replicates and extends the Python SECScrape/EVScrape functionality, storing all data in SQLite for offline access.

### Data Sources

| Source | Data | Rate Limit |
|--------|------|------------|
| Yahoo Finance v10 quoteSummary | Market cap, P/E, EPS, margins, beta, short interest, earnings dates, dividends, quarterly financials, institutional holders, company profile | 300ms between requests |
| SEC EDGAR XBRL companyfacts | Total debt, cash (more accurate than Yahoo for EV calculation) | 200ms (5 req/sec) |
| SEC EDGAR company_tickers.json | CIK lookup by ticker | Cached |

### Enterprise Value Calculation

```
EV = Market Cap + Total Debt - Cash & Equivalents
```

- Market Cap from Yahoo Finance (`price.marketCap`)
- Total Debt: prefer SEC XBRL (`LongTermDebtAndCapitalLeaseObligations` + `DebtAndCapitalLeaseObligationsCurrent`), fallback to Yahoo (`financialData.totalDebt`)
- Cash: prefer SEC XBRL (`CashAndCashEquivalentsAtCarryingValue`), fallback to Yahoo (`financialData.totalCash`)

### SQLite Schema

Three tables in the existing `typhoon_cache.db`:

- **`fundamentals`** — one row per symbol: EV, MCap, debt, cash, ratios, dates, profile (34 columns)
- **`quarterly_financials`** — revenue, net income, FCF, EPS per quarter per symbol
- **`institutional_holders`** — holder name, shares, % held per symbol

### API

```rust
// Single ticker scrape (Yahoo + SEC XBRL)
fundamentals::scrape_ticker(client, conn, "SLV").await

// Batch scrape with skip-if-recent and progress channel
fundamentals::scrape_batch(client, conn, &tickers, 24, Some(&progress_tx)).await

// Extract stock tickers from MT5 cache keys (filters out forex/crypto/indices)
fundamentals::extract_stock_tickers_from_cache(conn)

// Query functions
fundamentals::get_fundamentals(conn, "SLV")           // Single symbol
fundamentals::get_all_fundamentals(conn)                // All (for EV scanner)
fundamentals::get_upcoming_earnings(conn, 50)           // Earnings calendar
fundamentals::get_upcoming_dividends(conn, 50)          // Dividend calendar
fundamentals::get_quarterly_financials(conn, "SLV")     // Quarterly data
fundamentals::get_institutional_holders(conn, "SLV")    // Top holders
```

### Ticker Extraction from Cache

MT5 cache keys are `"mt5:SLV:4Hour"` (3-part: `mt5:{SYM}:{TF}`; BarCacheWriter is the sole producer and has always emitted this shape). The module parses these and filters out:
- Forex pairs (6-char: EURUSD, GBPJPY, etc.)
- Crypto (BTCUSD, ETHUSD, SOLUSD, etc.)
- Indices (starting with # or .)

### Integration Points

1. **BrokerCmd::FundamentalsScrape** — UI button triggers batch scrape via broker channel (non-blocking)
2. **Background thread** — can periodically refresh stale data (>24h old)
3. **UI windows** — Fundamentals viewer, EV Scanner, Earnings Calendar, Dividend Calendar

## Consequences

### Positive
- No more external Python scripts — fundamentals are native Rust, integrated into the terminal
- Data persists in SQLite between sessions
- Batch scrape of all MT5 symbols with progress reporting
- SEC XBRL provides more accurate debt/cash than Yahoo alone
- Skip-if-recent avoids redundant API calls

### Negative
- Yahoo Finance API may change or rate-limit aggressively
- SEC XBRL only available for US-listed companies
- Non-US Darwinex symbols (forex CFDs) will have no fundamentals data
- Initial batch scrape of ~800+ symbols takes ~5 minutes at 300ms rate limit

## Updates (2026-04-08)

### Permanent Failure Blocklist
`scrape_failures` table stores symbols that return 404/Not Found from Yahoo. These are permanently skipped on future scrape runs, avoiding wasted API calls on delisted or non-existent symbols.

### Rate Limit Cooldown
When Yahoo returns HTTP 429 (Too Many Requests), the scraper automatically pauses for 60 seconds, then retries the failed ticker before continuing. Previously, 10 consecutive failures would abort the entire batch.

### Per-Broker Scrape Buttons
Scrape Status Dashboard now has individual buttons: "MT5 Only", "Alpaca Only", "TastyTrade Only", "All Sources". Each sends `BrokerCmd::FundamentalsScrape` with the appropriate `use_mt5/use_alpaca/use_tastytrade` flags.

### Symbol Sources
`FundamentalsScrape` carries three boolean flags: `use_mt5`, `use_alpaca`, `use_tastytrade`. MT5 symbols extracted from `bar_cache` keys (`mt5:SYMBOL:TF`, 3-part). Alpaca symbols from `broker.get_all_assets()`. TastyTrade symbols from `tt.get_positions()`.

See also: ADR-056 (Screener)
