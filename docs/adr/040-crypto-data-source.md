# ADR-040: Crypto Data Source Hierarchy — Kraken over Binance

**Status:** Implemented
**Date:** 2026-03-22

## Context

Crypto symbols at Darwinex (MT5) have weekend gaps — markets close Friday and reopen Monday. For continuous charting and accurate risk analytics (VaR, drawdown), weekend price data must be backfilled from a 24/7 exchange.

## Decision

### Rejected: Binance Public API

Binance.com geo-blocks API access from restricted jurisdictions (US, Canada, and others) per their Terms of Service section "b. Eligibility". HTTP 451 "Unavailable For Legal Reasons" is returned. Binance.US exists but has limited pair coverage and separate API.

### Accepted: Kraken Public API

Kraken has no geo-restrictions, requires no API key for public market data, and provides deep history:

- **BTC/USD**: from 2013 (deepest of any exchange)
- **ETH/USD**: from 2016
- **Most alts**: from 2017-2018
- **Rate limit**: ~15 calls/minute (public), 720 bars per request
- **Timeframes**: 1m, 5m, 15m, 30m, 1h, 4h, 1d, 1w (monthly aggregated from daily)
- **24/7/365**: including weekends, holidays

### Data Hierarchy (3-tier)

```
Priority 1: MT5 (Darwinex) — weekday authority, has the spread/pricing DARWINs trade
Priority 2: Kraken         — fills weekend gaps + extends history pre-MT5
Priority 3: Alpaca         — live trading execution (NOT 24/7 for crypto)
```

### Merge Logic

1. Load existing MT5 bars (have Friday-close → Monday-open gaps)
2. Fetch Kraken bars for full date range (including pre-MT5 history)
3. Insert Kraken bars **only where MT5 has no data** (gap-fill, never overwrite)
4. For symbols not at Darwinex/Alpaca — Kraken is sole source, stored under `mt5:` key for unified charting

### Symbol Mapping

| TyphooN | Kraken | Notes |
|---------|--------|-------|
| BTC/USD | XBTUSD | Kraken uses XBT for Bitcoin |
| DOGE/USD | XDGUSD | Kraken uses XDG for Dogecoin |
| ETH/USD | ETHUSD | Direct mapping |
| SOL/USD | SOLUSD | Direct mapping |

### Weekend Auto-Sync
The frontend automatically polls Kraken every 30 seconds when markets are closed (Friday 22:00 UTC - Sunday 22:00 UTC) for the currently viewed crypto symbol. This provides near-real-time weekend price updates without manual intervention.

## Implementation Notes (2026-03-26)

- **Deep backfill**: Kraken daily data now goes back to 2013 for BTC/USD, providing the deepest continuous history available.
- **Live Alpaca fallback**: When cache misses occur, a live Alpaca bar fetch is used as fallback to fill gaps in real time.
- **Cache key prefix fix**: Kraken-sourced bars are stored under the `kraken:` cache key prefix (not `mt5:`), ensuring correct source attribution and avoiding collisions with MT5 data.

## Consequences

- **Pro**: Weekend gaps filled for all crypto symbols
- **Pro**: No API key or account needed
- **Pro**: Works in all jurisdictions (no geo-blocking)
- **Pro**: BTC history from 2013 (vs MT5's ~2011, but with gaps)
- **Pro**: Can chart Kraken-only symbols not available at Darwinex
- **Con**: 720 bars per request = multiple paginated calls for deep history
- **Con**: Kraken doesn't list every alt (some niche coins may be missing)
- **Con**: Rate limit lower than Binance (15/min vs 1200/min)
