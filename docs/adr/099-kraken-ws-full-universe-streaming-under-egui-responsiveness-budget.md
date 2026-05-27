# ADR-099: Kraken WS Full-Universe Streaming Under egui Responsiveness Budget

**Status:** Accepted
**Date:** 2026-05-24
**Related:** ADR-089 (zstd compression policy), ADR-032 (performance architecture), ADR-094 (Kraken async bar sync), ADR-098 (per-frame O(1) discipline), `native/src/app/kraken_ohlc_ws.rs`, `native/src/app/app_runtime.rs`, `engine/src/broker/kraken/ohlc_ws.rs`, `engine/src/core/cache.rs::merge_bars_fast`

## Context

ADR-094 covers REST-side Kraken sync pacing. This ADR covers the public
WebSocket OHLC channel turned on by default and pointed at the full Spot
catalog.

The motivation is straightforward: REST OHLC is serialised behind a
~1 req/sec global counter (see ADR-094/-211). At ≈1500 Spot pairs × 8
timeframes that is unreachable through REST alone for the low TFs. WS push
exists exactly to close that gap. ADR-094's REST-side pacing is unchanged;
WS is additive forward streaming on top of it.

The risk this ADR addresses is the inverse of REST: REST is slow because
it is rate-limited; WS is fast and the danger is that the resulting bar
firehose lands on the egui render thread and stalls the UI. On first
subscribe Kraken delivers a snapshot of up to ≈720 historical closed bars
for every (pair, interval). With the full universe enabled that is on the
order of 8 million bars arriving in a single flush window — exactly the
kind of work that, naively persisted, would saturate every core for tens
of seconds and translate into visible egui jank.

The earlier subscription-shrinking proposal (open charts + held positions
+ watchlist only) was rejected: the WS feed exists to keep the **whole
universe** current independent of REST, and narrowing subscriptions just
leaves the rest of the catalog on REST alone — the exact gap WS was
intended to close. The performance fix had to be in the write path, not
in the subscription set.

## Decision

Stream the full Spot universe on every Kraken-served interval, but
keep every per-bar processing step bounded and **off the egui frame
thread**. Specifically:

### 1. Defer cache writes to bar close

`native/src/app/kraken_ohlc_ws.rs::run_ws_bar_writer` buffers incoming
bars by `(symbol, interval, interval_begin)` with last-write-wins. Only
bars whose bucket has fully closed are flushed; open in-progress buckets
stay buffered until the interval rolls over and the final close value
supersedes every intermediate tick.

Without this, Kraken's per-tick updates to the same open 1Min bar would
trigger one `merge_bars` (with full zstd re-pack of the entire history
blob) for every WebSocket message — easily dozens per pair per minute.
The deferred-write gate drops the steady-state write rate by ~60× while
preserving freshness semantics: `kraken_ws_fresh_until` updates only on
close, which is exactly when the staleness check cares.

`WS_BAR_FLUSH_INTERVAL = 5s` is the flush cadence. `partition_closed_bars`
is unit-tested in the same module for the snapshot and steady-state
cases.

### 2. Lighter compression on the hot write path

Even with bar-close gating, the initial snapshot storm is the worst case:
≈12k cache entries to re-pack inside the first flush window. At the
default `BAR_ZSTD_LEVEL = 22` (encoder ~5–10 MB/s) that pegs cores for
tens of seconds.

`engine/src/core/cache.rs::merge_bars_fast` is the WS-only variant that
calls `put_bars_with_level(.., 3)`. zstd-3 cuts encode time ~10–20× at
the cost of ~15–20% larger blobs **until the next compaction pass**.

The promotion is automatic: `compact_storage` at
`engine/src/core/cache.rs:2390` filters `WHERE zstd_level < target`, so
the next scheduled `auto_compact` run (or the manual `Compact (zstd-22)`
button in Storage Manager) picks the WS-written rows up and rewrites
them at zstd-22. ADR-089 §5 captures the formal carve-out. Operators do
not need to do anything; users who want immediate reclamation can run
the manual button.

### 3. All cache work behind `spawn_blocking`

`flush_ws_bars` wraps the per-key `merge_bars_fast` work in
`tokio::task::spawn_blocking` so SQLite locking, zstd encoding, and JSON
serialisation never compete with tokio's async workers or the egui
thread. ADR-094 established the same pattern for REST cache writes; the
WS writer reuses it.

### 4. Pair-discovery retry for the spawn lifecycle

`maybe_start_kraken_ws_ohlc` is idempotent on `kraken_ws_ohlc_started`
but it also defers until Kraken AssetPairs have landed. The WS pipeline is
intentionally full-universe, not focus-scoped; an empty chart/watchlist/
position set is not a reason to skip subscriptions. `app_runtime.rs`
re-evaluates every 15s (`kraken_ws_ohlc_last_spawn_retry`) so settings
that are enabled before pair discovery still bring the streamers up once
the catalog is available. Once `started == true` the retry becomes a
no-op.

### 5. UI-thread `user_interacting` auto-reset

`user_interacting` was set true on drag-start / price-axis scale-start
and was never reset. After the user's first drag, every `if
self.user_interacting && !full_tilt` throttle in `market_data_sync.rs`
stayed in the "interacting" branch forever, holding background sync at
shrunken batch sizes for hours. Worse, the cache-rebuild guards in
`app_runtime.rs` (`if !self.user_interacting && ...` for active-symbol
sets, scoped fundamentals, MT5/tastytrade coverage maps, alpaca sync
state) were skipped every frame, so those caches were frozen at whatever
value they held the moment of the first drag.

The reset gate added to the top of `eframe::App::update` clears
`user_interacting` the moment the pointer is released **and** no scroll
delta is active. Cache rebuilds run on the next frame; sync returns to
full throttle. This is independent of the WS pipeline but is load-bearing
for the "always-on full-universe WS" decision because the constant
trickle of WS-fresh bars depends on the rebuild gates firing every frame
the user is not actively dragging.

### 6. Bar-close diagnostic on the broker thread

`KrakenLiveTrade` ownTrades handling is instrumented with a 2 ms
threshold log (`tracing::warn!` if `t0.elapsed() > 2ms`). This is not a
WS-bar concern directly but it shares the same broker-message hot path;
the log surfaces any regression where a sync side-effect (balance/positions/
open-orders refresh) drifts back onto the egui frame.

## Consequences

**Pros**
- Every Spot pair × every WS-served interval stays current at the bar
  close, without REST budget contention.
- Initial snapshot storm completes in a few seconds of background CPU
  rather than tens of seconds of foreground stall.
- Cache stays correct: every bar that lands is persisted; freshness
  anchor (`kraken_ws_fresh_until`) is updated on close so REST skips
  refetches that WS already covered.
- No new operator burden: zstd-3 rows are picked up by existing auto-
  compact, and the existing manual `Compact (zstd-22)` button is the
  same recovery handle.

**Cons / Tradeoffs**
- Until the next compaction, WS-written rows take ~15–20% more disk
  than equivalent REST rows. Auto-compact closes the gap.
- An open 1Min bar can lag the WS feed by up to `WS_BAR_FLUSH_INTERVAL`
  (5s) before the close-value reaches the cache. Live trade-prints and
  ownTrades are not affected — they go through a separate ticker path.
- The `user_interacting` reset uses `primary_down || secondary_down ||
  smooth_scroll_delta.y.abs() > 0.0` as the "still interacting" gate.
  Any tiny residual scroll-delta keeps the throttle on for that frame;
  in practice this is one frame, not perceptible. If a touchpad starts
  feeding sub-pixel scroll noise this gate may need a small threshold.

## Implementation pointers

- WS bar writer: `native/src/app/kraken_ohlc_ws.rs::run_ws_bar_writer`,
  `partition_closed_bars`, `flush_ws_bars` (uses `merge_bars_fast`).
- Hot-path merge: `engine/src/core/cache.rs::merge_bars_fast`,
  `put_bars_with_level`.
- Spawn lifecycle: `maybe_start_kraken_ws_ohlc` +
  `kraken_ws_ohlc_last_spawn_retry` in `app_runtime.rs`.
- User-interaction reset: top of `eframe::App::update` in
  `app_runtime.rs` (~line 40).
- Auto-compact promotion: `engine/src/core/cache.rs::compact_storage`
  filter on `zstd_level < target`; weekly schedule in
  `native/src/app/auto_compact.rs`.

## Non-goals

- Narrowing the WS subscription to focus symbols. WS exists to cover the
  whole catalog; narrowing just regresses to the REST gap WS was added
  to close.
- Persisting open-bar updates. Only closed bars hit the cache. Live
  intra-bar values stay in `bar_builder` for the chart that needs them.
- Replacing REST. WS is forward streaming; REST still handles cold-start
  deep history. ADR-094 covers REST; this ADR is strictly the WS side.
