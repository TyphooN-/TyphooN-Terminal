# ADR-128: Sync-Coverage Levers (market-hours gate, reachable %, WS ceiling)

**Status:** Accepted | **Date:** 2026-06-28

Builds on **ADR-112** (demand-depth vs catalog-breadth sync lanes), **ADR-113 /
ADR-124** (cross-source equity merge), and **ADR-126** (primary/assist broker).
Touches the sync scheduler (`app/market_data_sync.rs`), the Sync Status
computation/display (`app/sync_status.rs`, `app/bar_sync.rs`), the Yahoo fallback
error path (`app/app_runtime_errors.rs`, `app/app_runtime_support.rs`), and the
Kraken WS OHLC lane (`app/kraken_ohlc_ws.rs`).

## Context

Overnight bar-sync coverage is near-flat: across ~7.5h the per-broker healthy
counts moved by tens of cells against denominators of ~74k–87k, and Alpaca sat at
*exactly* its prior value. This reads as "sync is stuck," but most of it is
structural, not a bug:

- **The denominators are ~87k cells** (catalog × timeframes) and the provider
  rate walls are fixed — Alpaca ~200 req/min, Kraken iapi ~6 req/s (Cloudflare),
  Yahoo an unauthenticated throttle. A single full Alpaca pass of 87k cells is
  ~7h *if* every request succeeded and none needed refreshing, so high coverage of
  that denominator is mathematically unreachable.
- **US equities were closed overnight**, so the equity breadth lanes
  (Alpaca/Yahoo) had no new intraday bars to fetch and could not progress.
- **A large tail of the denominator is provider-no-data** — symbols/timeframes a
  source genuinely does not carry. These can never become healthy, yet they were
  counted in the coverage %, pinning it visually low.

Three levers were considered to make real, honest progress without pretending the
rate walls don't exist. (A separate change — ADR-less, same date — gave the Yahoo
lane an escalating/decaying 429 backoff so one rate-limit event no longer pins the
lane dark for a flat 5 minutes.)

## Decision

### Lever 1 — market-closed gate on intraday-equity incremental fetches

When the US equities session (including extended hours) is **fully CLOSED**, a
**backfill-complete intraday** equity cell cannot gain a newer bar, so re-probing
it only burns Alpaca RPM. `queue_alpaca_fetch` now skips such a fetch when **all**
hold:

- `us_equities_closed()` — the Alpaca clock string reports `CLOSED` (not OPEN /
  PRE-MARKET / AFTER-HOURS, which can still print extended-hours bars; an empty
  status fails open).
- `is_intraday_equity_sync_tf(tf)` — `5Min`/`15Min`/`30Min`/`1Hour`/`4Hour`.
  Daily-and-higher settle at the close and stay worth pulling.
- `backfill_complete` — the cell has no historical gap left to fill.
- `!focus` — the actively-viewed chart is never gated.

This is deliberately scoped to **Alpaca**. Crypto rides Kraken (24/7, never
gated); Kraken xStocks are 24/5 on their own clock; Yahoo is already
429-self-limited and its intraday is a full-history pull where closed-hours
backfill is still useful. Net: closed-market RPM is redirected to lanes that can
progress (historical backfill, daily settles, 24/7 crypto) instead of re-probing
cells that cannot move.

### Lever 2 — additive "reachable %" (no-data excluded), Merged-scoped

Sync Status now distinguishes *permanent provider-no-data* from *not-yet-fetched*.
A new `SyncStatsRow.unreachable` overlay counts cells where **every applicable
provider has tombstoned** the (symbol, tf) as no-data (`MergedSyncStatus::Unreachable`).
It is populated only on the **Merged** lane — the one whose denominator is the
full catalog, where the no-data tail actually lives (per-source rows only count
cells that returned data at least once, so they have no such tail).

The reporting is **additive**: the raw `healthy / total` and `pct_healthy` are
unchanged; the header chip shows a second figure —
`X% reachable (N no-data)` — where `reachable = healthy / (total − unreachable)`.
No existing number or test semantics changed. The tombstone snapshot
(`no_data_keys_by_source`) mirrors the scheduler's view: the per-broker
unresolvable index plus the persisted Alpaca no-data set folded into `alpaca`.

### Lever 3 — bounded full-universe WS waves replace permanent catalog subscriptions

Permanent full-catalog WS subscription was tried and rejected because it caused connection churn
("reset without closing handshake") plus multi-second snapshot-write stalls on the
render thread. Current native WS planning is bounded rather than demand-only:
persistent live subscriptions prioritize the demand set, while rotating snapshot
waves cover the WS-tokenized catalog across enabled native timeframes from `1Week`
through `1Min`, strict high-timeframe-first. `1Month` is derived from daily. Queue,
subscription, and writer bounds—not catalog exclusion—are the safety mechanism.
Non-tokenized equities still depend on iapi demand repair and assist/merged lanes.

## Consequences

- Coverage % is now **honest**: a permanently-no-data tail no longer reads as
  "pending work," and the closed-market gate stops the scheduler from spending RPM
  on cells that cannot move.
- The headline numbers still won't approach 100% — that is the rate-wall reality,
  not a defect. A fair throughput test is **during market hours**, when the equity
  lanes can actually advance.
- Lever 1 is Alpaca-only by design. If Alpaca crypto is ever enabled, the gate
  must grow a crypto exemption (crypto is 24/7); today crypto never reaches this
  path.
- Lever 2's overlay is best-effort: it depends on the unresolvable index being
  built (it is, early, by the scheduler). Before then the reachable % equals the
  raw %, which is a safe degradation.

## Non-goals

- Raising any provider's request rate. The limiters are the real ceiling; only
  request **reduction** (TF derivation) and **redirection** (Lever 1) help.
- Changing the raw coverage definition. Lever 2 is purely additive.
