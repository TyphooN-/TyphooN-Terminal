# ADR-029: Broker market-data sync scheduler lifecycle

> **Note (2026-06-10):** References to tastytrade in this document are historical.
> After ADR-111 the active brokers are Kraken + Alpaca only.

Status: Accepted
Date: 2026-05-20

## Context

TyphooN Terminal can sync large broker universes: Alpaca equities, Kraken Spot, Kraken Futures, and tastytrade. A naïve scheduler pass that scans every symbol/timeframe or releases queue slots on the first cache-write notification does not scale well:

- Kraken Spot's public AssetPairs universe is intentionally complete, including fiat-quoted crypto, crypto crosses, xStocks, and spot FX.
- Each logical broker fetch can emit multiple runtime messages: a cache-write `BarsFetched`, optional classification messages such as backfill-complete/no-data, then a terminal `FetchSettled` message.
- If a pending key is cleared on `BarsFetched`, the scheduler can refill the same symbol/timeframe before the terminal settlement and classification messages have landed.
- Scanning the full broker universe on every refill wastes UI-thread time while background network workers are the real bottleneck.

## Decision

The broker sync scheduler is cursor-limited and high-timeframe-first for all broad automated broker rotations, including Alpaca, Kraken Spot sectors, Kraken Futures sectors, and tastytrade.

1. Build candidate work from a bounded flattened ring of `(timeframe, symbol)` slots:
   - `1Month` across all symbols,
   - then `1Week`,
   - then `1Day`,
   - continuing down to `1Min`.
2. Keep the scan budget fixed per refill, independent of total universe size.
3. Select work by bucket in this order: `Missing`, `Stale`, `Backfill`.
4. Skip any `(symbol, timeframe)` whose normalized pending key already exists.
5. Alpaca retry-queue entries are also scheduler exclusions. Retry dispatch owns
   those keys until their backoff expires, preventing rate-limited partial
   results from being immediately requeued by the broad scheduler.
6. Use persisted no-data and backfill-complete markers to avoid repeating known unproductive backfills.
   Alpaca has app-side persisted no-data tombstones; Kraken, Kraken Futures,
   and tastytrade primarily use generic broker-unresolvable markers plus
   persisted backfill-complete markers. Backfill-complete markers are stored
   under `{broker}:backfill_complete_pairs` and suppress repeat `Backfill`
   scheduling while still allowing Missing/Stale freshness sync.
   Persisted broker-unresolvable markers are kept in per-broker `HashSet`
   indexes (`broker -> SYMBOL:TF`) so each candidate suppression check is an
   O(1) membership lookup and the scheduler no longer rebuilds filtered
   tombstone sets on every refill.
7. Brokers with explicit terminal settlement messages keep their pending slot until settlement:
   - Alpaca: `AlpacaFetchSettled`
   - Kraken Spot: `KrakenFetchSettled`
   - Kraken Futures: `KrakenFuturesFetchSettled`
   - tastytrade: `TastytradeFetchSettled`
8. `BarsFetched` remains an intermediate UI/cache refresh signal for those brokers; it must not own scheduler release.
9. Settlement handlers release provider-specific pending keys and refill sync
   slots. Alpaca failure/rate-limit settlements do not immediately refill; the
   retry queue or the next normal scheduler tick controls the next attempt.

## Consequences

- Large universes do not require an O(symbols * timeframes) pass per refill; every broad broker path has an explicit scan budget.
- tastytrade uses the same rotating bounded selector as Alpaca/Kraken instead of the legacy whole-symbol workset scan.
- Sync budgets and tastytrade timeframe-window helpers now live in `native/src/app/sync_config.rs`, keeping policy constants out of `app.rs`.
- Async broker fetch workers now live in `native/src/app/broker_fetch.rs`, keeping network-response parsing and task dispatch out of the parent `app.rs` integration file.
- High timeframes obtain broad initial coverage before low-timeframe backfills consume the queue.
- Manual/foreground and background fetches are deduplicated through the same normalized pending-key sets.
- Thin-history or provider-exhausted instruments converge instead of being requeued every few seconds after successful bounded responses.
- Recent successful writes are merged back into rebuilt sync-state maps so a `bg_rev` bump does not immediately make a just-settled pair look stale again. This applies both in scheduler entry points and in the app-frame cache refresh path.

## Verification

Use targeted tests before wider checks:

```bash
cargo test -p typhoon-native app::alpaca_sync::tests::select_alpaca_sync_workset_rotating_walks_all_symbols_mn1_before_lower_timeframes
cargo test -p typhoon-native app::alpaca_sync::tests::select_alpaca_sync_workset_rotating_skips_pending_without_advancing_priority
cargo test -p typhoon-native app::alpaca_sync::tests::merge_recent_sync_overrides_preserves_settled_fetch_across_bg_rev_rebuild
```

Then run:

```bash
cargo fmt --all
cargo check -p typhoon-native
```


## Tiered Priority Model (2026-06-10 update)

To balance full-universe coverage with UI responsiveness, the scheduler now classifies symbols into three tiers:

- **Tier 1 (MTF Grid / Foreground)**: Open or focused symbols in the MTF Grid + the currently visible single-chart symbol.
- **Tier 2 (Active)**: Watchlist symbols + current positions/holdings.
- **Tier 3 (Background)**: Everything else in the Kraken + Alpaca universe.

Within each tier, timeframes are still processed high-to-low (`1Month → 1Week → 1Day → ... → 1Min`).

This ensures that research packets, outlier scans, and MTF Grid charts get fresh data first while the broad universe continues to converge in the background.


## Tiered Priority Model (2026-06-10)

To balance full-universe coverage with UI responsiveness, the scheduler classifies symbols into three priority tiers before applying the existing high-timeframe-first and bucket ordering:

### Tiers
- **Tier 1 – MTF Grid / Foreground**: Open or focused symbols in the MTF Grid + the currently visible single-chart symbol. These receive the highest priority.
- **Tier 2 – Active**: Watchlist symbols + current positions/holdings.
- **Tier 3 – Background**: Everything else in the Kraken + Alpaca universe.

### Ordering within tiers
Timeframes are processed high-to-low:
`1Month → 1Week → 1Day → 4Hour → 1Hour → 30Min → 15Min → 5Min → 1Min`

### Bounded concurrency
When Tier 1 or Tier 2 work is available, the effective batch size for pure Tier 3 (background) work is reduced to prevent foreground starvation.

### Rationale
This model ensures that research packets, outlier detection, and MTF Grid charts get fresh data first, while the broad universe continues to converge in the background without starving the UI.
