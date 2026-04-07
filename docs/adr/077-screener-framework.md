# ADR-077: Screener Framework — EV, Fundamentals, and Signal Scanning

**Status:** Implemented | **Date:** 2026-04-07

## Context

With 800+ Darwinex symbols and 12K+ Alpaca symbols cached, users need screening tools to filter the universe by fundamental metrics (Enterprise Value, P/E, dividend yield), technical signals (unusual volume, RSI extremes), and custom criteria.

## Decision

Screener module (`engine/src/core/screener.rs`) provides:
- **EV Scanner**: Ranks stocks by Enterprise Value composition (market cap, debt, cash) using Yahoo Finance + SEC EDGAR data stored in `fundamentals` table
- **Unusual Volume**: Compares current volume against rolling average from cached bar data
- **Fundamentals filters**: Query `fundamentals` table for dividend stocks, earnings dates, P/E ranges

All screeners run in background threads to avoid blocking UI. Results cached and displayed in dedicated windows.

## Consequences

- Screening 12K+ symbols requires efficient SQL queries with proper indices
- Yahoo fundamentals data must be kept current (24h cache, scrape_failures blocklist)
- SEC EDGAR data supplements Yahoo for accurate EV (debt/cash from XBRL filings)
- Extensible: new screeners added as functions that query the fundamentals/bar tables

See also: ADR-054 (Fundamentals Engine), ADR-003 (SQLite Cache)
