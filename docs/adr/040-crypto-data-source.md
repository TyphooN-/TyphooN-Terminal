# ADR-040: Crypto Data Source Hierarchy — CryptoCompare + Kraken

**Status:** Implemented (Updated 2026-03-27)
**Date:** 2026-03-22 | **Updated:** 2026-03-27

## Context

Crypto symbols at Darwinex (MT5) have weekend gaps — markets close Friday and reopen Monday. For continuous charting and accurate risk analytics (VaR, drawdown), weekend price data must be backfilled from a 24/7 exchange.

## Decision

### Rejected: Binance Public API
Binance.com geo-blocks API access from US/Canada. HTTP 451 returned.

### Rejected: Kraken OHLC as Primary Backfill
Kraken's OHLC endpoint returns only the most recent ~720 bars regardless of the `since` parameter. The `since` parameter does NOT enable historical pagination for OHLC data (unlike their Trades endpoint). This makes Kraken unsuitable for deep historical backfill.

### Accepted: CryptoCompare (Primary Backfill)

CryptoCompare provides the deepest free crypto history with proper pagination:

- **No API key required** — works without authentication
- **No geo-restrictions** — available worldwide
- **2000 bars per request** — 3x more than Kraken
- **Proper backward pagination** — `toTs` parameter works correctly
- **Deep history**: BTC from 2010, ETH from 2015, most alts from 2017+
- **Endpoints**: `histoday`, `histohour`, `histominute`
- **OHLCV data**: open, high, low, close, volumefrom, volumeto

### Kraken (Retained for Weekend Gap-Fill)
Kraken is retained as a secondary source for near-real-time weekend data (720 most recent bars), but CryptoCompare handles all deep history backfill.

### Data Hierarchy (4-tier)

```
Priority 1: MT5 (Darwinex)      — weekday authority, signal account data
Priority 2: Alpaca/tastytrade   — where user actually trades
Priority 3: CryptoCompare       — deep history backfill (2010+)
Priority 4: Kraken              — weekend gap-fill (720 most recent bars)
```

### Cache Key Prefixes

```
mt5:SYMBOL:TF           — MT5 BarCacheWriter data (authoritative)
alpaca:SYMBOL:TF         — Live Alpaca bar fetch
cryptocompare:SYMBOL:TF  — CryptoCompare deep history
kraken:SYMBOL:TF         — Kraken weekend gap-fill (legacy, auto-deleted when CryptoCompare replaces)
```

### Auto-Cleanup
When CryptoCompare backfill completes for a symbol, old Kraken data for the same symbol/timeframes is automatically deleted from cache (superseded by deeper CryptoCompare data).

### Symbol Handling
CryptoCompare uses standard symbols: `fsym=BTC&tsym=USD`. The module normalizes TyphooN symbols: `BTCUSD` → `BTC`, `SOL/USD` → `SOL`.

### Aggregation
For timeframes not natively supported by CryptoCompare:
- 5Min/15Min/30Min: aggregated from 1Min
- 4Hour: aggregated from 1Hour
- 1Week: aggregated from 1Day
- 1Month: aggregated from 1Day (calendar month grouping)

## Consequences

- **Pro**: Full crypto history from 2010 (BTC) — deepest available
- **Pro**: No API key, no geo-blocking, no rate limit issues
- **Pro**: 2000 bars/request with proper backward pagination
- **Pro**: Automatically supersedes and cleans up limited Kraken data
- **Pro**: Weekend gaps filled for all crypto symbols
- **Con**: CryptoCompare may have slightly different prices than Darwinex/Kraken
- **Con**: Lower timeframes (1Min) have limited history (~7 days at CryptoCompare)
