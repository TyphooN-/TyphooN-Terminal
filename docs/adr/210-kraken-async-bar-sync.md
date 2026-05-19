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

- Public Kraken tasks are still queued behind a shared 16-permit semaphore, but
  Spot/xStocks OHLC HTTP calls are now additionally paced by ADR-211: one
  request about every 1.1 seconds process-wide and per pair, with cooldown on
  Kraken rate-limit responses.
- Spot and Futures queue windows are enlarged so refill scheduling can keep the
  public pipeline bounded. Futures public requests can still use the semaphore
  directly because Kraken's published Futures REST budget assigns no cost to
  public endpoints.
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

- The shared candidate selector is the same O(1)-membership path used by Alpaca:
  pending work, unresolvable symbols, and limited-history/backfill-complete markers are checked by normalized `SYMBOL:Timeframe` hash keys before dispatch.
- Spot/xStocks sectors use independent rotating cursors so a large USD-crypto sector cannot starve xStocks, fiat-quoted crypto, spot FX, or crypto crosses.
- The bounded background scan borrows symbol names from the source universe instead of cloning each scanned slice; queue pressure is controlled by sector-specific batch/window limits and interaction-aware clamps.
- `BarsFetched` remains the settlement point for Kraken because each public command writes exactly one cache result before settling; unlike Alpaca there is no separate retry/backfill-complete lifecycle message that can race the slot release.

## Consequences

- **Pro:** Opening or requesting a crypto chart can start Kraken OHLCV work
  immediately, even when a CryptoCompare deep-history pass is already running.
- **Pro:** Kraken results emit `BarsFetched` as each timeframe lands, so active
  charts can reload before the slower combined backfill is fully complete.
- **Pro:** Synchronous SQLite/zstd merge writes no longer occupy async runtime
  workers.
- **Pro:** Kraken Futures shares the same non-blocking cache-write behavior.
- **Con:** Spot Kraken fetches no longer saturate the semaphore at the HTTP
  boundary; large Spot backfills trade peak speed for Kraken-documented public
  pacing.

## Implementation

- `native/src/app.rs`
  - `KRAKEN_PUBLIC_FETCH_PERMITS = 16`
  - `KRAKEN_SPOT_QUEUE_WINDOW = 160`
  - `KRAKEN_FUTURES_QUEUE_WINDOW = 96`
  - `CRYPTOCOMPARE_BACKFILL_PERMITS = 2`
  - `store_json_bars_in_cache()` for blocking cache merge/write work
  - `run_crypto_compare_backfill_task()` for background CC + Kraken union work
- `engine/src/core/kraken.rs`
  - Spot public OHLC limiter and cooldown (ADR-211)

## References

- ADR-037: Data source hierarchy
- ADR-040: Crypto data source
- ADR-072: Kraken as full broker
- ADR-203: Alpaca sync autotuning
- ADR-211: Kraken rate-limit pacing and cooldown
