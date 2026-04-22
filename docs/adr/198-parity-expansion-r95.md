# ADR-198: Parity Expansion R95 — SHORTRANK_DELTA + Short-Interest History

**Date:** 2026-04-21  
**Status:** Accepted  
**Related:** `engine/src/core/research.rs`, `engine/src/core/fundamentals.rs`, `engine/src/core/lan_sync.rs`, `native/src/app.rs`, ADR-127, ADR-197, ADR-188

## Context

ADR-197 reduced the deferred Godel parity list to two holdouts:

- `SHORTRANK_DELTA`, which had been blocked on historical short-interest storage
- `INSIDERCONC`, which still needs insider-ownership concentration in fundamentals

The current codebase now has enough infrastructure to unblock the first of
those without inventing a new upstream dependency:

- fundamentals scrapes already refresh `short_percent_of_float`
- the app already exposes a Finnhub short-interest fetch command
- the missing piece was compact per-symbol history persistence

That makes `SHORTRANK_DELTA` the next legitimate parity target.

## Decision

Ship one focused parity pass:

1. Add `research_short_interest_history`, a compact per-symbol JSON time
   series of short-interest observations.
2. Feed that history from existing fundamentals upserts and explicit
   `SHORT_INTEREST` fetches when vendor rows are available.
3. Add `SHORTRANK_DELTA`, a 180-day sector-relative rank of the change in
   `short_percent_of_float`, risk-inverted so short covering earns a higher
   and safer rank.

The new surface ships end to end:

- engine compute logic
- SQLite persistence
- LAN sync registration
- research-packet output
- palette aliases
- broker plumbing
- egui popup window

Chart overlays remain deferred by ADR-188.

## Consequences

- The formerly blocked `SHORTRANK_DELTA` surface is now live.
- Short-interest history grows automatically as the user continues normal
  fundamentals scrapes, instead of requiring a separate maintenance step.
- Duplicate-value compression keeps the new history cache compact even when
  fundamentals are refreshed more often than short-interest values change.
- The only named Godel parity holdout left is `INSIDERCONC`, which is still
  blocked on new upstream ownership data.

## Notes

- `SHORTRANK_DELTA` intentionally measures trend, not level. `SHRANK` remains
  the current short-interest rank, while `SHORTRANK_DELTA` answers whether the
  short thesis is building or being covered versus peers.
- The rank uses the same risk-inverted label ladder as `SHRANK` because a more
  negative delta is the safer outcome.
- Symbols need at least two history points in the trailing 180-day window, and
  the sector needs at least three peers with usable history, before the rank
  becomes meaningful.
