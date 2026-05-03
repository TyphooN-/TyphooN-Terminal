# ADR-210: Kraken Async Bar Sync Acceleration

**Status:** Accepted | **Date:** 2026-05-03

## Context

Kraken Spot/xStocks OHLCV and Kraken Futures candles are public, no-key market
data paths. The terminal uses them for immediate crypto chart catch-up,
weekend/recent gap-fill, broad Kraken universe rotation, and fallback coverage
when CryptoCompare is rate-limited.

Direct `KrakenBackfill` commands already spawned one async task per timeframe,
but the combined `CryptoCompareBackfill` command ran inline in the broker
command loop. Several UI paths enqueue CryptoCompare before Kraken, which meant
a slow deep-history pass could delay public Kraken fetch commands from even
starting.

## Decision

Kraken public bar sync now follows a fully async, queue-friendly model:

- Public Kraken concurrency is raised to 16 shared permits.
- Spot and Futures queue windows are enlarged so refill scheduling can keep the
  public pipeline saturated while still bounded.
- Direct Spot and Futures fetches keep the one-task-per-timeframe model.
- Cache merge/write work for Kraken and Kraken Futures runs inside
  `spawn_blocking`, keeping tokio workers focused on network I/O.
- `CryptoCompareBackfill` now launches as a background task instead of blocking
  the broker command receiver.
- The CryptoCompare task immediately schedules its Kraken leg into a
  `JoinSet`, so recent Kraken bars can arrive and reload charts while
  CryptoCompare deep-history pagination continues.
- CryptoCompare pagination is separately bounded to two concurrent backfill
  tasks, so "backfill all" does not turn deep-history requests into an
  unbounded burst.
- CryptoCompare and Kraken continue to use independent 6-hour freshness checks
  under `cryptocompare:*` and `kraken:*` keys.

## Consequences

- **Pro:** Opening or requesting a crypto chart can start Kraken OHLCV work
  immediately, even when a CryptoCompare deep-history pass is already running.
- **Pro:** Kraken results emit `BarsFetched` as each timeframe lands, so active
  charts can reload before the slower combined backfill is fully complete.
- **Pro:** Synchronous SQLite/zstd merge writes no longer occupy async runtime
  workers.
- **Pro:** Kraken Futures shares the same non-blocking cache-write behavior.
- **Con:** Public Kraken fetches are more parallel. The semaphore remains the
  backpressure point and should be tuned down if Kraken starts returning public
  rate-limit responses on a target network.

## Implementation

- `native/src/app.rs`
  - `KRAKEN_PUBLIC_FETCH_PERMITS = 16`
  - `KRAKEN_SPOT_QUEUE_WINDOW = 160`
  - `KRAKEN_FUTURES_QUEUE_WINDOW = 96`
  - `CRYPTOCOMPARE_BACKFILL_PERMITS = 2`
  - `store_json_bars_in_cache()` for blocking cache merge/write work
  - `run_crypto_compare_backfill_task()` for background CC + Kraken union work

## References

- ADR-037: Data source hierarchy
- ADR-040: Crypto data source
- ADR-072: Kraken as full broker
- ADR-203: Alpaca sync autotuning
