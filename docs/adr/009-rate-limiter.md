# ADR-009: Centralized Rate Limiter

**Status:** Implemented (v4 — Adaptive)
**Date:** 2026-03-15 (updated 2026-03-20)

> **See also:** [ADR-035](035-bar-fetch-optimization.md) — the authoritative reference for all bar fetch optimizations built on top of this rate limiter: adaptive pacing, page_token pagination, incremental cache-aware fetch, WebSocket bar builder, and data sanitization.

**Context:** Alpaca's free plan allows 200 data API requests/minute. Multiple concurrent operations (chart loading, MTF indicators, pre-fetch, live polling, multiple tabs) can easily exceed this. The free tier also applies **progressive throttling** — sustained requests are silently slowed, with individual responses taking 1-10+ minutes.

## Decision

Single `RateLimiter` struct in Rust, shared via `Arc<Mutex>` across all data API requests. Base pacing at 320ms intervals (187.5 req/min). **Adaptive**: monitors API response latency and backs off when progressive throttling is detected.

## Architecture

```rust
pub struct RateLimiter {
    last_request: Arc<Mutex<Instant>>,
    cooldown_until: Arc<Mutex<Option<Instant>>>,
    adaptive_ms: Arc<Mutex<u64>>,  // v4: dynamic interval
}
```

All `get_bars()` calls pass through `self.rate_limiter.wait()` before making HTTP requests.

## Evolution

| Version | Approach | Problem Solved |
|---|---|---|
| v1 | Per-request sleep(320ms) | Wasted budget when only 1 request needed |
| v2 | Centralized RateLimiter | All requests share one Mutex-guarded budget |
| v3 | +429 cooldown (60s) | Auto-backs-off on rate limit hit, partial data returned |
| **v4** | **+Adaptive pacing** | **Detects progressive throttling, backs off before 429** |

## Adaptive Pacing (v4)

The rate limiter adjusts its interval based on observed API response times:

| Condition | Action |
|---|---|
| Response < 2s | Recover: decrease interval by 50ms (min 320ms) |
| Response > 10s | Back off: increase interval by 200ms (max 5s) |
| HTTP 429 | Emergency: double interval (max 5s) + 60s cooldown |

After each API call, `report_latency(elapsed_ms)` feeds the response time back to the pacer, which adjusts the interval for the next request. This prevents the exponential slowdown seen on the free tier: instead of blindly hammering at 320ms while the API responds slower and slower, we match our pace to the API's capacity. On paid tier, responses stay fast so the interval stays at 320ms.

## 429 Cooldown

On HTTP 429 (Too Many Requests):
1. Trigger 60-second cooldown on the rate limiter
2. Double the adaptive interval (capped at 5s)
3. Retry up to 3 times (page_token preserved)
4. If retries exhausted, return collected bars (trimmed to most recent)
5. All subsequent requests from any caller wait for cooldown to expire

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
- **Pro**: Adaptive pacing prevents progressive throttling on free tier
- **Pro**: Multiple tabs can't overwhelm the API
- **Pro**: Single point of control for rate policy
- **Pro**: Graceful degradation (partial data on 429, not total failure)
- **Pro**: Paid tier benefits too (no unnecessary backoff when API is fast)
- **Con**: Serial pacing means parallel fetching is sequential in practice
- **Con**: Heavy multi-tab usage slows down each individual tab's load time
