# ADR-011: News, Earnings, and Dividend Data

**Status:** Implemented
**Date:** 2026-03-15
**Context:** Traders need to know upcoming earnings dates, dividend dates, and recent news for symbols they're watching. This data should be cached with the same three-tier strategy as bar data.

## Decision

Fetch news from Alpaca News API, earnings/dividends from Alpaca corporate actions. Store in the same zstd-compressed cold cache. Display on chart as markers and in a dedicated panel.

News/article syncing is not a broad exchange backfill lane. It is an active-context
lane: symbols from open positions, open orders, watchlist rows, focused/open
charts, and other explicit user scopes are fetched first. Broad catalog backlog
work is allowed only after active SEC/news/article work has drained or yielded its
budget. This keeps the right panel and chart research context relevant instead of
spending provider quota on inactive Kraken equities just because they exist in the
catalog.

## Data Sources

| Data | Alpaca Endpoint | Cache Strategy |
|---|---|---|
| News | `GET /v1beta1/news?symbols=LUMN&limit=50` | Warm + Cold, 15-min TTL |
| Earnings | Calendar API / third-party | Cold cache, daily refresh |
| Dividends | Calendar API / third-party | Cold cache, daily refresh |

## Storage

Same three-tier cache as bars:
- **Hot**: In-memory for current session
- **Warm**: SQLite cache (typhoon_cache.db events table)
- **Cold**: `~/.config/typhoon-terminal/cache/news_SYMBOL.zst`

## Display

- **Chart markers**: Earnings dates as triangle markers on the price chart
- **News panel**: Scrollable list below the indicator panel
- **Dividend indicators**: Price lines or markers at ex-dividend dates

## Scheduling priority

Priority order for event/research sync:

1. Active positions.
2. Open orders / order-entry symbols.
3. Watchlist symbols.
4. Focused/open chart symbols and MTF grid symbols.
5. Explicit user-triggered scope fetches.
6. Remaining backlog/catalog symbols.

SEC filing backfill follows the same active-context rule (see ADR-073). Bar
backfill/fallback can continue in the background, but it must not starve active
news or SEC filing work.
