# ADR-071: BARDATA — Full History Download Command

**Status:** Implemented | **Date:** 2026-04-05

## Context

MT5 syncs full bar history via BarCacheWriter. Alpaca and tastytrade data was previously limited to a proportional lookback window (e.g., 365 days for H1, 3650 days for D1). Users need the ability to download ALL available historical bars for complete analysis.

## Decision

### BARDATA Command

New command `BARDATA` (aliases: `FETCH_ALL`, `FULL_HISTORY`) downloads all available bars for the current chart's symbol and timeframe from Alpaca.

### Implementation

**Engine (`alpaca.rs`):**
- `get_all_bars(symbol, timeframe, progress)` — paginates from earliest available date to present
- Uses Alpaca's `page_token` pagination with 10,000 bars per chunk
- No limit cap — continues until API returns no more data or empty page
- Handles rate limiting gracefully (429 → cooldown → retry, accept partial on max retries)
- Progress callback via `UnboundedSender<String>` reports chunk count, total bars, latest date
- Monthly bars aggregated from weekly (Alpaca doesn't support 1Month natively)
- Crypto starts from 2015, stocks from 2000

**Native (`app.rs`):**
- `BrokerCmd::FetchAllBars { symbol, timeframe }` — spawns async task
- Progress messages displayed in log panel via `BrokerMsg::OrderResult`
- Completed bars stored in SQLite cache with `alpaca:` prefix
- Triggers `Mt5SyncDone` to reload charts after completion
- Auto-normalizes crypto symbols (SOLUSD → SOL/USD for API)

### Usage

```
~ BARDATA          → downloads all bars for active chart symbol+TF
~ FETCH_ALL        → alias
~ FULL_HISTORY     → alias
```

### Data Flow

```
User: BARDATA command
  → BrokerCmd::FetchAllBars { symbol: "SOL/USD", timeframe: "1Day" }
  → Spawns async task
    → get_all_bars() paginates from 2015-01-01
    → Chunk #1: +10000 bars (2015→2017)
    → Chunk #2: +10000 bars (2017→2019)
    → ... continues ...
    → Chunk #N: +3000 bars (2024→present)
    → Total: 53000 bars stored in cache
  → Mt5SyncDone → chart reloads with full history
```

## Consequences

- **Pro:** Complete historical data for backtesting, analysis, and indicator computation
- **Pro:** Progress reporting in log panel — user sees chunk-by-chunk progress
- **Pro:** Rate limit handling prevents API bans
- **Pro:** Stores in same SQLite cache as all other sources (automatic gap-fill)
- **Con:** Full download for M1 data can take several minutes (rate limiting)
- **Con:** Alpaca free tier has limited history depth for some timeframes
