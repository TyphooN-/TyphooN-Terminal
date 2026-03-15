# ADR-009: Centralized Rate Limiter

**Status:** Implemented
**Date:** 2026-03-15
**Context:** Alpaca's free plan allows 200 data API requests/minute. Multiple concurrent operations (chart loading, MTF indicators, pre-fetch, live polling, multiple tabs) can easily exceed this.

## Decision

Single `RateLimiter` struct in Rust, shared via `Arc<Mutex>` across all data API requests. Paces at 320ms intervals (187.5 req/min — under the 200 limit with headroom).

## Architecture

```rust
pub struct RateLimiter {
    last_request: Arc<Mutex<Instant>>,
    cooldown_until: Arc<Mutex<Option<Instant>>>,
}
```

All `get_bars()` calls pass through `self.rate_limiter.wait()` before making HTTP requests.

## 429 Cooldown

On HTTP 429 (Too Many Requests):
1. Trigger 60-second cooldown on the rate limiter
2. Return whatever bars were collected so far (partial success)
3. All subsequent requests from any caller wait for cooldown to expire
4. Log warning to terminal

No retry loops. No manual sleeps. Callers get partial data gracefully.

## Budget Allocation

All consumers share one budget. No priority queue — first-come-first-served via mutex. In practice:

| Consumer | Requests | When |
|---|---|---|
| Primary chart load | ~4 chunks | On Load button click |
| MTF indicator data | ~6 TFs × 1-2 chunks | After primary chart |
| Background pre-fetch | ~7 TFs × 1-4 chunks | After MTF indicators |
| Live bar polling | 1 every 10s | Continuously |
| Other tabs loading | Same pattern | Concurrent |

## Consequences

- **Pro**: Zero 429 errors in normal operation
- **Pro**: Multiple tabs can't overwhelm the API
- **Pro**: Single point of control for rate policy
- **Pro**: Graceful degradation (partial data on 429, not total failure)
- **Con**: Serial pacing means parallel fetching is sequential in practice
- **Con**: Heavy multi-tab usage slows down each individual tab's load time
