# ADR-095: Kraken Rate-Limit Pacing and Cooldown

**Status:** Accepted | **Date:** 2026-05-04

## Context

ADR-094 made Kraken public bar sync queue-friendly by letting Spot and
Futures fetches run on async tasks behind a shared semaphore. That improved
chart catch-up, but Spot OHLC requests for one pair could still burst across
timeframes. Kraken's current Spot public-market-data guidance says OHLC and
Trades are limited by IP address and currency pair, and that public calls at
about one request per second or slower remain within limits.

Kraken also documents separate authenticated Spot REST account counters:
history-style calls cost more than ordinary account calls, trading calls are
handled by matching-engine limits, and throttling may return `EAPI:Rate limit
exceeded`, `EGeneral:Too many requests`, HTTP 429, or `EService: Throttled:
<unix timestamp>`.

Kraken Futures public REST endpoints used by TyphooN have no request cost in
the published Futures REST budget. Private Futures endpoints are not part of
the current TyphooN integration.

## Decision

Kraken Spot public OHLC requests are paced at the HTTP boundary:

- one Spot public request is reserved about every 1.1 seconds process-wide;
- each OHLC pair also has its own 1.1 second reservation, so pair-local bursts
  cannot outrun Kraken's pair/IP limiter;
- rate-limit responses arm a process-wide Spot public cooldown, starting at
  five seconds and doubling to sixty seconds on repeated hits;
- `EService: Throttled: <unix timestamp>` is honored when Kraken supplies an
  explicit retry timestamp;
- Spot public OHLC retries are limited to three attempts.

Authenticated Spot account/history REST requests now use a conservative local
counter that matches Kraken's default verified-account guidance:

- max counter: 20;
- decay: 0.5 counter units per second;
- ordinary private REST calls cost 1;
- `Ledgers`, `QueryLedgers`, `TradesHistory`, `QueryTrades`, and
  `ClosedOrders` cost 4;
- order-placement/cancel/amend/edit endpoints do not consume this REST counter
  because Kraken routes them through matching-engine trading limits.

TyphooN does not automatically retry trading-limit order rejections. A rejected
order is reported to the user so the terminal does not duplicate trading intent
after an ambiguous broker response.

## Consequences

- **Pro:** Direct Kraken backfill and CryptoCompare+Kraken union backfill share
  the same Spot public pacing and cooldown.
- **Pro:** The async task model remains useful for queueing, Futures requests,
  and non-blocking cache writes, while Spot REST itself is paced to Kraken's
  documented public level.
- **Pro:** Account polling and history pulls no longer burst through Kraken's
  private REST counter.
- **Con:** Large Spot backfills take longer because the limiter intentionally
  favors staying under Kraken's public threshold over saturating HTTP
  concurrency.
- **Con:** Trading-rate limits are modeled conservatively by surfacing
  rejections rather than retrying orders.

## References

- Kraken support, "What are the API rate limits?":
  https://support.kraken.com/articles/206548367-what-are-the-api-rate-limits-
- Kraken API Center, Spot REST Rate Limits:
  https://docs.kraken.com/api/docs/guides/spot-rest-ratelimits/
- Kraken API Center, Spot Trading Limits:
  https://docs.kraken.com/api/docs/guides/spot-ratelimits/
- Kraken API Center, Futures Rate Limits:
  https://docs.kraken.com/api/docs/guides/futures-rate-limits/
