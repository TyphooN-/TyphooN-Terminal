# ADR-094: Kraken Async Bar Sync Acceleration

**Status:** Accepted | **Date:** 2026-05-03

## Context

Kraken Spot OHLCV and Kraken Futures candles are public, no-key market
data paths. The terminal uses them for immediate crypto chart catch-up,
weekend/recent gap-fill, and broad Kraken universe rotation across the complete
Spot AssetPairs catalog.

## Decision

Kraken public bar sync now follows a fully async, queue-friendly model:

- Public Kraken tasks are still queued behind a shared 16-permit semaphore, but
  Spot OHLC HTTP calls are now additionally paced by ADR-095: one
  request about every 1.1 seconds process-wide and per pair, with cooldown on
  Kraken rate-limit responses.
- Spot and Futures queue windows are enlarged so refill scheduling can keep the
  public pipeline bounded. Futures public requests can still use the semaphore
  directly because Kraken's published Futures REST budget assigns no cost to
  public endpoints.
- Direct Spot and Futures fetches keep the one-task-per-timeframe model.
- Spot initial/provider-window fetches request Kraken's bounded public OHLC
  window instead of sending `since=0`; Spot backfill-complete markers mean the
  provider's recent OHLC window has been saturated, not that deep exchange
  history is available through Spot REST.
- Futures is deliberately decoupled from Spot's bounded OHLC policy. Futures
  chart candles use explicit from/to traversal from the configured historical
  floor and request provider-maximum history with the full-history sentinel.
- Cache merge/write work for Kraken and Kraken Futures runs inside
  `spawn_blocking`, keeping tokio workers focused on network I/O.
- CryptoCompare remains a targeted deep-history helper for crypto broker
  backfill. It is not scheduled as an independent full CryptoCompare universe;
  Kraken Spot can combine CryptoCompare deep history with Kraken's
  provider-window OHLC for enabled crypto symbols.

- The shared candidate selector is the same O(1)-membership path used by Alpaca:
  pending work, unresolvable symbols, and limited-history/backfill-complete markers are checked by normalized `SYMBOL:Timeframe` hash keys before dispatch.
- Spot sectors use independent rotating cursors so a large USD-crypto sector cannot starve fiat-quoted crypto, spot FX, or crypto crosses. Kraken Securities/xStocks scheduling is separate under the iapi/provider-assist lanes documented in ADR-101 through ADR-103.
- Spot fiat/crypto inclusion is controlled by global broker quote filters. New sessions default to USD and USD stablecoin quotes (`USD`, `USDT`, `USDC`, `USDG`) instead of assuming every fiat-quoted crypto pair is wanted. Existing session schema v2 settings are migrated into the new per-quote schema v3; future crypto brokers should reuse the same quote filter rather than adding broker-local defaults.
- The bounded background scan borrows symbol names from the source universe instead of cloning each scanned slice; queue pressure is controlled by sector-specific batch/window limits and interaction-aware clamps.
- Coverage-first priority is shared with Alpaca: never-cached symbol/timeframe pairs are scheduled before stale refresh or provider-history backfill, ordered from `1Month` down to `1Min` within the scanned sector/window.
- `BarsFetched` is an intermediate UI/cache freshness signal for Kraken Spot and Kraken Futures. Pending scheduler slots are released only by `KrakenFetchSettled` / `KrakenFuturesFetchSettled`, so zero-bar, failure, unresolvable, and backfill-complete paths cannot leak or prematurely recycle pending keys.
- Kraken Spot emits provider-window completion when its bounded public OHLC response returns less than the requested recent window. Kraken Futures emits full-history completion after a successful provider-maximum range traversal; its marker stores the actual cached count rather than an old fixed local target.

## Consequences

- **Pro:** Opening or requesting a crypto chart can start Kraken OHLCV work
  immediately without depending on any deep-history CryptoCompare path.
- **Pro:** Kraken results emit `BarsFetched` as each timeframe lands, so active
  charts can reload before final `FetchSettled` slot release.
- **Pro:** Synchronous SQLite/zstd merge writes no longer occupy async runtime
  workers.
- **Pro:** Kraken Futures shares the same non-blocking cache-write behavior.
- **Con:** Spot Kraken fetches no longer saturate the semaphore at the HTTP
  boundary; large Spot backfills trade peak speed for Kraken-documented public
  pacing.

## Implementation

- `typhoon-native/src/app/sync_config.rs`
  - `KRAKEN_PUBLIC_FETCH_PERMITS = 16`
  - `KRAKEN_SPOT_QUEUE_WINDOW = 160`
  - `KRAKEN_FUTURES_QUEUE_WINDOW = 96`
  - `KRAKEN_SPOT_BACKGROUND_SCAN_LIMIT = 384`
  - `KRAKEN_FUTURES_BACKGROUND_SCAN_LIMIT = 192`
- `typhoon-native/src/app/broker_fetch.rs`
  - `run_kraken_fetch_task()` and `run_kraken_futures_fetch_task()`
  - `store_json_bars_in_cache()` for blocking cache merge/write work
  - terminal `FetchSettled` messages and backfill-complete classification
- `typhoon-native/src/app/market_data_sync.rs`
  - `queue_kraken_fetch()`, `queue_kraken_futures_fetch()`
  - bounded sector scheduling with normalized pending/unresolvable/backfill keys
- `typhoon-native/src/app/app_runtime.rs`
  - `BarsFetched` UI/cache handling
  - `KrakenFetchSettled` / `KrakenFuturesFetchSettled` pending-slot release
- `typhoon-engine/src/core/kraken.rs`
  - Spot public OHLC limiter and cooldown (ADR-095)

## References

- ADR-051: Kraken as full broker
- ADR-087: Alpaca sync autotuning
- ADR-095: Kraken rate-limit pacing and cooldown
