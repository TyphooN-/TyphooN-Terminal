# 106: Remove Stooq Daily Fallback Provider

Date: 2026-05-30

## Status
Accepted

## Context
Stooq (`stooq.com`) daily CSV fallback (`fetch_stooq_daily_bars`) was introduced alongside Yahoo Chart as an unkeyed public source for long-history 1Day equity/xStock bars under the `stooq:` cache namespace (see ADR 102).

In operation:
- The endpoint is almost always in cooldown / rate-limited for our request patterns.
- Successful responses contain zero or N/D rows for the normalized symbols we request.
- It never contributes bars to any MTF Grid, gap-fill, or backfill flow for Kraken-equities.
- The only remaining references were the UI checkbox, pause banner, BrokerCmd::StooqDailyFetchBars variant, source lists, and the CSV parser in fallback_bars.rs.

Maintaining dead code that never activates increases cognitive load and binary size with no benefit.

## Decision
Completely excise Stooq support:
- Delete `fetch_stooq_daily_bars`, `stooq_supports_timeframe`, `stooq_symbol` from the fallback-bar helper layer (now `typhoon_engine::core::fallback_bars`; it was native when this ADR was accepted).
- Remove `BrokerCmd::StooqDailyFetchBars` variant and its handler.
- Remove `backfill_stooq_daily_enabled` and `stooq_sync_pause_*` fields and all related logic from App struct, settings UI, persistence, sync_status, bar_sync.
- Strip `"stooq"` from every source list, cache-key builder, and provider enumeration.
- Update PERFORMANCE.md and remove any test assertions mentioning Stooq.
- Add this ADR so the decision is never reversed without a new recorded rationale.

Yahoo Chart remains the sole unkeyed public fallback.

## Consequences
- Smaller code surface for fallback providers.
- One fewer checkbox and pause state machine in Settings.
- Cache keys of the form `stooq:SYM:1Day` will no longer be written (existing ones can be ignored or manually cleaned; they were never populated).
- Future data-source experiments must justify activation with observed production data, not "maybe useful someday".

## References
- ADR 102 (Kraken equities gap-fill via Alpaca and provider fallback)
- fallback-bar helper layer, now `typhoon_engine::core::fallback_bars` (removed the old Stooq branch)
- typhoon-native/src/app.rs (multiple sites)
