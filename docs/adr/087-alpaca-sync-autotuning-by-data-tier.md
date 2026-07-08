# ADR-087: Alpaca Sync Autotuning by Data Tier

**Date:** 2026-04-25
**Status:** Accepted
**Related:** `typhoon-engine/src/broker/alpaca.rs`, `typhoon-native/src/app.rs`, ADR-009 (multi-broker), ADR-059 (SSD write reduction)

## Context

The Alpaca sync loop fetches historical bars across the full tradable
universe (US equities + crypto) so chart panes, screener, and AI research
packets always have a recent cached series. Two rolling pain points
accumulated as the universe grew and as user accounts moved between
data tiers:

1. **Pacing was fixed**, set conservatively for the free IEX tier
   (~200 requests / min). Subscribed accounts have headroom that the
   sync loop never used; free-tier accounts occasionally still hit
   429 because pacing didn't react to actual rate-limit headers.
2. **Window sizing was fixed**, ~250 bars per request regardless of
   timeframe or cache depth. For 1-day bars on a symbol the user wants
   5 years cached, that meant ~5 round-trips per symbol; for 1-min
   bars on a symbol the user wants 5 days cached, the same
   default over-fetched and wasted requests.
3. **No-data symbols re-checked every cycle.** A symbol Alpaca cannot
   serve (delisted, never had IEX coverage, asset-class mismatch) was
   re-requested on every sweep and counted against rate limit even
   though every prior attempt had returned empty.
4. **Close-position rejects were silent.** A `close_position` rejection
   surfaced as `Ok` with an empty body, so the UI confirmed a close
   that didn't happen.

## Decision

Make Alpaca sync adaptive across four axes: pacing, window sizing,
symbol skipping, and reject visibility.

### 1. Header-driven pacing

`AlpacaBroker` carries a `requests_per_minute` field plus a separate
`bar_requests_per_minute` for `/v2/stocks/bars` and friends (Alpaca
gates these endpoints differently from non-bar endpoints).

- `with_requests_per_minute(rpm)` and `set_requests_per_minute` set
  the floor at construction or post-connect.
- `observe_rate_limit_headers(headers)` reads
  `X-Ratelimit-Limit` / `X-Ratelimit-Remaining` after every response
  and walks the effective rpm toward the observed limit.
- `apply_requests_per_minute_hint(rpm)` lets the UI feed a tier hint
  (free / IEX / SIP / live algo) so the loop starts in the right
  ballpark instead of probing up from 200.
- `set_bar_requests_per_minute_hint(rpm)` does the same for the bar
  endpoint, which has its own quota.
- `rpm_to_interval_ms(rpm)` converts a target RPM to a per-request
  sleep, used by the throttler in the sync loop.

### 2. Cache-depth-aware window sizing

`BarsLookbackMode` picks the window per request based on configured
cache depth, not a fixed bar count:

- `bars_per_day(is_crypto, timeframe)` — bars-per-trading-day for the
  TF (390 for 1m equities, 1440 for 1m crypto, 7-26 for 1h, etc.).
- `max_lookback_days(is_crypto, timeframe, mode)` — caps the lookback
  by a per-mode policy (`Default`, `Deep`, `Shallow`) and by what
  Alpaca's tier actually allows.
- `lookback_days_for_request(timeframe, target_bars, ...)` —
  inverse: given a target cache depth in bars, return the days to
  request, clamped by the tier limit. Picks the largest window that
  fits in one request, so a 5-year 1d backfill is one request, not
  five.

The sync loop now reads the user's per-symbol cache-depth target and
sizes each request to hit it without overshooting. The 250-bar fixed
window is gone.

### 3. No-data tombstones and retry ownership

Symbols that Alpaca definitively returns no data for are tracked in app-side
persisted KV tombstones under `alpaca:no_data_pairs`. The sync scheduler reads
those normalized `SYMBOL:Timeframe` keys before queuing and bypasses pairs the
broker cannot serve. A successful later bar write clears the no-data mark, and
users can clear markers explicitly after coverage/subscription changes.

Transient 429/partial/empty outcomes are owned by the persisted Alpaca retry
queue instead. Retry entries are scheduler exclusions until their backoff
expires, so a rate-limited partial write cannot be immediately requeued by the
broad Missing/Stale/Backfill scheduler.

### 4. Close-reject surfacing

`close_position` and `close_all_positions` now read
`response.status()` and parse `json["message"]`; any non-2xx with a
`"message"` body returns `Err` instead of pretending the close
succeeded. This pairs with the cancel-exits-then-close flow:
the flow can now distinguish a hard reject (asset not closeable)
from a soft `insufficient qty` race (which the retry handles).

### 5. O(1) scheduler hot path and settled-fetch refill

The automated scheduler now keeps the per-cycle candidate walk allocation-light:

- Pending work is keyed by normalized `SYMBOL:Timeframe` in `HashSet`s, so duplicate dispatch checks are O(1).
- No-data and backfill-complete markers are consulted as hash maps/sets before any broker command is queued.
- Candidate selection rotates through a bounded background slice and uses borrowed symbol iterators; it no longer clones the scanned background universe just to choose the next batch.
- Alpaca scheduling only rebuilds Alpaca's own cache-state map on `bg_rev` changes; it no longer warms Kraken/Futures/tastytrade maps from the Alpaca tick.
- Timeframe de-duplication uses a side `HashSet` while preserving high-to-low ordering (`1Month` → `1Min`).
- Coverage-first scheduling is the top priority: if a scanned symbol/timeframe has no cached bars at all, the scheduler fills those missing pairs highest timeframe → lowest before spending any slot on stale refresh or shallow-cache backfill. Foreground/focus symbols only sort within the same bucket; they no longer preempt never-cached coverage.
- `BarsFetched` updates cache state and UI, but Alpaca pending slots are only released by `AlpacaFetchSettled`. Successful settlements drain retry entries and refill scheduler slots; failure/rate-limit settlements leave refill to the retry queue or the next normal scheduler tick. This prevents the `BarsFetched`/`FetchSettled` race that repeatedly requeued shallow or rate-limited symbols before retry/backfill-complete bookkeeping arrived.
- Automated Alpaca writes do not synchronously rescan SQLite storage statistics unless the Storage/Cache windows are visible. Scheduler freshness is updated through `note_cached_sync_success`, keeping chart interaction responsive while bulk sync runs.
- Sync Health uses the last broker check/write time as a recent-health signal when the provider's latest available market bar is intrinsically old. A successfully checked symbol with no newer Alpaca bars should not make the broker look less healthy immediately after sync.



## Tiered Symbol Priority (2026-06-10)

In addition to the `Missing` / `Stale` / `Backfill` bucket ordering, symbols are now classified into three priority tiers (MTF Grid → Active → Background) before bucket selection. This ensures that charts the user is actively looking at (especially MTF Grid) receive bar data before background universe work.

## Update 2026-07: WS Market-Data Feed Awareness (extension of tier autotuning)

Alpaca market-data WebSocket (quotes + trades) now auto-detects feed ("sip" vs "iex") on connect and emits `BrokerMsg::AlpacaMarketDataFeed`.

- Native subscription logic (`alpaca_quote_subscription_symbols`) is feed-aware: SIP cap ~100, IEX/unknown cap 30 (conservative per Alpaca docs).
- On 406 / "limit" / "subscription" errors: surface as OrderResult, tighten cap temporarily (to 20), extend throttle (to 10s), 5-min backoff window; auto-clear on reconnect or subscribed ack.
- Diff subscribe/unsubscribe (add/remove only) + reconnect hygiene + stale detection + exp backoff already present; control frames (success subscribed/auth, errors) now also surfaced for UI visibility.
- Complements bar-tier autotuning (ADR-087) and Kraken WS v2 robustness (atomic CRC etc.).
- O(1) hot paths for applying WS ticks (bare-symbol HashMaps + rebuild indices).

This keeps WS sub limits respected without user intervention and provides symmetric robustness for Alpaca/Kraken brokers.

## Update 2026-07-08: broad-sync pause ownership and low-memory scaling

The bar-sync side now treats provider rate limits and machine headroom as first-class scheduler inputs:

- Single-symbol and batch Alpaca fetches emit `BrokerMsg::AlpacaRateLimitObserved { historical_rpm }` after observing headers, so native capacity can follow the actual historical-data RPM rather than only the configured hint.
- Broad/background Alpaca scheduling checks `alpaca_sync_pause_until_ts` before queueing fallback, retry, or background work. Focus/foreground paths keep their reserve, but the full background universe stops feeding new work until the time-bounded pause expires.
- Successful Alpaca bar writes clear no-data tombstones for that symbol/timeframe and reset consecutive 429 state; rate-limited failures become retry/backoff state instead of repeated visible errors.
- Installed-RAM scaling trims Alpaca full-tilt fetch permits, queue windows, and batch sizes before RSS pressure spikes. Floors preserve progress; the policy reduces in-flight memory on 16–64 GB machines without reducing the covered universe.

This extends the original tier autotuning decision: Alpaca capacity is bounded by account tier, live observed headers, retry state, and local machine headroom.

## Consequences

- **Sync throughput scales with the user's tier** without manual
  configuration. Free / IEX accounts pace at ~200 RPM, subscribed
  accounts walk up to whatever Alpaca returns in headers.
- **Backfill of deep history is materially faster.** A 5-year 1d
  backfill that previously needed ~5 paged round-trips per symbol
  now lands in 1; a 60-day 1h backfill that needed ~3 lands in 1.
- **The full tradable universe is feasible to keep current.**
  Without the skip set, the scheduler kept requeueing dead symbols;
  with it, every sync cycle spends rate-limit budget only on symbols
  that have ever returned data.
- **Close UI is honest.** A user clicking Close sees the actual
  broker response — success, hard reject with reason, or transient
  reject that auto-retries.
- **Tier upgrades require an explicit hint, not a restart.** The
  Settings window can call `apply_requests_per_minute_hint` after
  the user changes their Alpaca subscription; the loop reacts on
  the next request without dropping queued work.
- **Tier downgrades are observed automatically** through
  `observe_rate_limit_headers` — Alpaca returns a lower limit, the
  broker pacer steps down within one request.

## Validation

- `cargo build --workspace` clean.
- `cargo test --workspace --lib` — 1932 tests pass, 3 ignored.
- Unit test: `observe_rate_limit_headers_updates_bar_rpm` confirms
  the bar-endpoint RPM tracks the observed header independently of
  the general-endpoint RPM.
- Manual: live sync against a free-tier Alpaca account stays under
  the 200 RPM ceiling and never hits 429.
- Manual: live sync against a subscribed account with `2000` rpm
  headroom runs the loop at ~5x prior throughput.
