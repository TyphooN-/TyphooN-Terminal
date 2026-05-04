# ADR-040: Crypto Data Source Hierarchy — CryptoCompare + Kraken

**Status:** Implemented
**Date:** 2026-03-22 | **Updated:** 2026-05-04

> 2026-05-01 update: CryptoCompare remains the deep-history extension for
> crypto before or beyond exchange-listed history. Kraken Spot/xStocks is now
> also synchronized as its own primary public market-data source under
> `kraken:SYMBOL:TF`; it is no longer only a weekend gap-fill source.
> Kraken Futures uses a separate public charts path and cache prefix:
> `kraken-futures:SYMBOL:TF`.

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

### Kraken Spot/xStocks (Primary Recent Public Sync + Complementary Gap-Fill)
Kraken provides recent public OHLCV without an API key. It is stored independently under `kraken:SYMBOL:TF` and participates in the normal six-source chart lookup after CryptoCompare. CryptoCompare remains the deep-history source; Kraken extends recent coverage, sub-hourly lookback, weekend continuity, and xStocks coverage.

### Crypto Backfill Flow (updated 2026-05-04)
The "Backfill ALL Crypto" button fetches 10 symbols × 8 TFs:
1. CryptoCompare: all 8 TFs (1Day through 5Min). Deep history for hourly+; 7-day limit for sub-hourly.
2. Kraken: launched immediately as the recent/gap-fill leg of the same background backfill task, under the shared Kraken public-request semaphore. It no longer waits behind CryptoCompare pagination.

### Data Hierarchy (6-tier)

```
Priority 1: MT5 (Darwinex)      — authority where BarCacheWriter has data
Priority 2: Alpaca              — broker bars for non-MT5 symbols
Priority 3: tastytrade          — DXLink bars for funded accounts
Priority 4: CryptoCompare       — deep crypto history backfill (2010+)
Priority 5: Kraken Spot/xStocks — public recent/gap-fill OHLCV
Priority 6: Kraken Futures      — public futures chart candles
```

### Cache Key Prefixes

```
mt5:SYMBOL:TF           — MT5 BarCacheWriter data (authoritative)
alpaca:SYMBOL:TF         — Live Alpaca bar fetch
cryptocompare:SYMBOL:TF  — CryptoCompare deep history
kraken:SYMBOL:TF         — Kraken Spot/xStocks public recent + gap-fill bars
kraken-futures:SYMBOL:TF — Kraken Futures public chart candles
```

### Independent Prefixes
CryptoCompare, Kraken Spot/xStocks, and Kraken Futures rows are kept under separate prefixes. Chart lookup and merge logic decide which rows to use; the CryptoCompare backfill path does not delete Kraken rows.

### Symbol Handling
CryptoCompare uses standard symbols: `fsym=BTC&tsym=USD`. The module normalizes TyphooN symbols: `BTCUSD` → `BTC`, `SOL/USD` → `SOL`.

### Aggregation
For timeframes not natively supported by CryptoCompare:
- 5Min/15Min/30Min: aggregated from 1Min
- 4Hour: aggregated from 1Hour
- 1Week: aggregated from 1Day
- 1Month: aggregated from 1Day (calendar month grouping)

### Backfill Coverage (Updated 2026-03-28)
All 9 timeframes are backfilled: 1Min, 5Min, 15Min, 30Min, 1Hour, 4Hour, 1Day, 1Week, 1Month.
Note: 1Min history limited to ~7 days at CryptoCompare. All sub-hourly TFs aggregate from 1Min.

### Two-Tier Refresh Strategy (Updated 2026-03-28)

| Layer | Source | When | Coverage |
|-------|--------|------|----------|
| Daily refresh | CryptoCompare | Once per session | 10 symbols × 3 TFs (1Day, 1Hour, 4Hour) |
| Recent public sync | Kraken | Continuous/adaptive polling + backfill union | Visible crypto/xStocks charts and requested TFs |
| Deep history | CryptoCompare | Manual button | 10 symbols × 8 TFs (excludes 1Min) |

CryptoCompare handles deep history (2000 bars/request, back to 2010). Kraken handles recent public OHLCV and gap-fill without requiring credentials. Rate limit retry with exponential backoff (2s→16s) for CryptoCompare.

## Consequences

- **Pro**: Full crypto history from 2010 (BTC) — deepest available
- **Pro**: No API key, no geo-blocking, no rate limit issues
- **Pro**: 2000 bars/request with proper backward pagination
- **Pro**: Independent prefixes preserve provenance while chart lookup merges useful recent/gap-fill bars
- **Pro**: Weekend gaps filled for all crypto symbols
- **Con**: CryptoCompare may have slightly different prices than Darwinex/Kraken
- **Con**: Lower timeframes (1Min) have limited history (~7 days at CryptoCompare)
