# ADR-057: Yahoo Finance Extended Hours Watchlist

**Status:** Implemented | **Date:** 2026-04-08

## Context

Alpaca's IEX/SIP feed snapshot API doesn't return real-time pre/post-market prices. TradingView shows $20.29 pre-market while the terminal showed $18.73 (regular close). Users need extended hours prices for watchlist and chart visualization.

## Decision

Use Yahoo Finance v7 quote API (`/v7/finance/quote`) for extended hours enrichment:
- Returns explicit `preMarketPrice` and `postMarketPrice` fields
- Batch query: all equity symbols in one HTTP call (`?symbols=CC,NCLH,SLV`)
- Authenticated session (cookie jar + crumb token) required — reuses `YahooSession` with 30-minute TTL
- Poll interval: 15 seconds (server only, LAN client uses KV-synced data)

### Extended Hours Candle
- Magenta candlestick drawn after last regular session bar
- Open = regular session close, High/Low/Close tracked from Yahoo ext price updates
- Magenta dashed price line at ext close price with labeled tag on Y-axis
- During regular hours: falls back to ghost candle placeholder

### Display Logic
When `ext_change_pct != 0`: Last/Chg/Chg% columns show the extended hours price (matching TradingView behavior). Ext% shows change from regular close to ext price.

## Consequences

- Pre/post market prices now match TradingView
- Single batch HTTP call per poll (no per-symbol rate limiting needed)
- Yahoo session cached 30 minutes, ~4 requests/min total
- LAN client gets extended hours data via `broker:watchlist` KV sync (no direct Yahoo calls)

See also: ADR-034 (Fundamentals Engine — shared YahooSession)
