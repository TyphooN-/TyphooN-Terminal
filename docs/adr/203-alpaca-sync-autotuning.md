# ADR-203: Alpaca Sync Autotuning by Data Tier

**Date:** 2026-04-25
**Status:** Accepted
**Related:** `engine/src/broker/alpaca.rs`, `native/src/app.rs`, ADR-010 (multi-broker), ADR-079 (LAN sync bandwidth), ADR-080 (SSD write reduction)

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

### 3. No-data symbol skip set

Symbols that Alpaca consistently returns empty for are tracked in a
broker-side skip set. The sync scheduler reads the set before
queuing a request and bypasses symbol-TF pairs that have failed
N consecutive times. The set is cleared on tier-change or on user
action (e.g. when a new subscription unlocks coverage). This is
effectively the dual of the persistent retry queue from earlier
work — the queue handles transient 429s, the skip set handles
permanent no-data.

### 4. Close-reject surfacing

`close_position` and `close_all_positions` now read
`response.status()` and parse `json["message"]`; any non-2xx with a
`"message"` body returns `Err` instead of pretending the close
succeeded. This pairs with ADR-201's cancel-exits-then-close flow:
the flow can now distinguish a hard reject (asset not closeable)
from a soft `insufficient qty` race (which the retry handles).

### 5. O(1) scheduler hot path and settled-fetch refill

The automated scheduler now keeps the per-cycle candidate walk allocation-light:

- Pending work is keyed by normalized `SYMBOL:Timeframe` in `HashSet`s, so duplicate dispatch checks are O(1).
- No-data and backfill-complete markers are consulted as hash maps/sets before any broker command is queued.
- Candidate selection rotates through a bounded background slice and uses borrowed symbol iterators; it no longer clones the scanned background universe just to choose the next batch.
- Timeframe de-duplication uses a side `HashSet` while preserving high-to-low ordering (`1Month` → `1Min`).
- `BarsFetched` updates cache state and UI, but Alpaca pending slots are only released by `AlpacaFetchSettled`. This prevents the `BarsFetched`/`FetchSettled` race that repeatedly requeued shallow symbols such as `GDC @ 4Hour` before backfill-complete bookkeeping arrived.
- Sync Health uses the last broker check/write time as a recent-health signal when the provider's latest available market bar is intrinsically old. A successfully checked symbol with no newer Alpaca bars should not make the broker look less healthy immediately after sync.

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
