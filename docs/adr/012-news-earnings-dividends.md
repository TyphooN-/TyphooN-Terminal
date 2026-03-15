# ADR-012: News, Earnings, and Dividend Data

**Status:** Implementing
**Date:** 2026-03-15
**Context:** Traders need to know upcoming earnings dates, dividend dates, and recent news for symbols they're watching. This data should be cached with the same three-tier strategy as bar data.

## Decision

Fetch news from Alpaca News API, earnings/dividends from Alpaca corporate actions. Store in the same zstd-compressed cold cache. Display on chart as markers and in a dedicated panel.

## Data Sources

| Data | Alpaca Endpoint | Cache Strategy |
|---|---|---|
| News | `GET /v1beta1/news?symbols=LUMN&limit=50` | Warm + Cold, 15-min TTL |
| Earnings | Calendar API / third-party | Cold cache, daily refresh |
| Dividends | Calendar API / third-party | Cold cache, daily refresh |

## Storage

Same three-tier cache as bars:
- **Hot**: In-memory for current session
- **Warm**: IndexedDB `typhoon_events` store
- **Cold**: `~/.config/typhoon-terminal/cache/news_SYMBOL.zst`

## Display

- **Chart markers**: Earnings dates as triangle markers on the price chart
- **News panel**: Scrollable list below the indicator panel
- **Dividend indicators**: Price lines or markers at ex-dividend dates
