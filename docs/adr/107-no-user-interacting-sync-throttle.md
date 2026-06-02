# ADR-107: Do Not Use `user_interacting` as a Sync/Render Performance Throttle

## Status

Accepted.

## Context

A previous performance pass used a `user_interacting` flag as a broad bandaid: when chart drag/zoom was detected, the app reduced broker message drain, background market-data scheduling, cache rebuilds, sparkline reads, indicator recomputes, deferred chart loads, and repaint behavior.

That approach did not fix the real stalls. Worse, it created semantic regressions and hidden state coupling:

- chart responsiveness depended on a global flag instead of fixing render-thread hotpaths;
- background sync capacity changed based on mouse state rather than actual pending-work pressure;
- full-universe actions could silently shrink to active/watchlist-sized work;
- stale `user_interacting` state could freeze cache rebuilds and scheduler decisions;
- stall logs became misleading by reporting a flag that was usually false during the actual blocking work.

The root cause of the observed stalls was oversized work on the UI thread: per-frame universe expansion, SEC/news/fundamental render closures cloning large symbol sets, blocking broker-message handling, and cache/sync rebuild churn. Those must be fixed directly.

## Decision

Remove `user_interacting` from TyphooN Terminal performance control flow.

Do not reintroduce a global "the user is interacting" throttle for market-data sync, broker drains, cache rebuilds, chart loading, indicator recompute, or repaint scheduling.

The allowed pattern is:

1. keep UI/render-frame work bounded and mostly O(1);
2. defer large universe expansion until an explicit click/action requires it;
3. keep full-universe semantics intact — `ALL` means full universe, not active/watchlist/MTF Grid;
4. use explicit pending-work caps, provider rate limits, queue windows, and background worker budgets;
5. profile stalls by phase and remove the hotpath instead of hiding it behind an interaction flag.

## Consequences

- Chart pan/free-look must remain responsive because render paths are cheap, not because sync is paused.
- Background sync remains governed by queue windows, provider rate limits, and pending-fetch pressure.
- `Scrape Scope (ALL)` / `Fetch (All)` actions must preserve full-universe semantics.
- Any future responsiveness fix that depends on `user_interacting` should be rejected during review unless it is narrowly local to the widget interaction itself and does not change sync/cache/data semantics.
