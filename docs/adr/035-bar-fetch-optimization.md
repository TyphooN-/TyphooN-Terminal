# ADR-035: Bar Fetch Optimization — Adaptive Pacing, Native Pagination, Crypto Lookback

**Status:** Implemented
**Date:** 2026-03-20

> **Supersedes:** Portions of [ADR-009](009-rate-limiter.md) (rate limiter v3 → v4) and [ADR-007](007-bar-prefetch-strategy.md) (chunk fetching strategy).

## Context

On the Alpaca free tier, crypto bar fetching exhibited **progressive throttling**: early chunks completed in ~200ms, but after sustained fetching, individual chunks took 7-10+ minutes. A full MTF grid cold load (BTC/USD 1Hour + SOL/USD 4Hour + stocks) took **3-4 hours** due to:

1. **Excessive lookback**: Crypto 1Hour fetched 365 days (8,760 bars, 30+ chunks). Crypto 4Hour fetched 730 days (4,380 bars, 35+ chunks).
2. **Manual date-based pagination**: Code advanced `start` date after each chunk — caused overlapping fetches, stale-chunk detection complexity, and empty-gap skip logic.
3. **Fixed-rate pacing (320ms)**: No awareness of progressive throttling. Kept hammering the API at max rate even as responses slowed to minutes.
4. **No early termination**: Would spend hours fetching historical data even when charts only needed ~500-1000 recent bars.

### Observed Performance (Before)

| Symbol | Timeframe | Bars Fetched | Chunks | Time |
|---|---|---|---|---|
| BTC/USD | 1Hour | 8,201 | 30+ | **2.5+ hours** |
| SOL/USD | 4Hour | 1,824 | 35+ | **2+ hours** |
| ADA/USD | 1Hour | 895 | 4 | ~8 min |
| CC | 4Hour | 1,177 | 5 | ~3 min |
| Full MTF grid | mixed | all | 100+ | **3-4 hours** |

## Decision

Four optimizations applied to `get_bars()` in `alpaca.rs`, affecting all tiers (free and paid):

### 1. Native `page_token` Pagination

**Before:** Manual date-advancing after each chunk — overlapping data, stale-chunk detection, empty-gap skip logic.

**After:** Alpaca returns `next_page_token` in each response. Pass it back for the next request. Server-maintained cursor with zero overlap, zero gaps, and no client-side date arithmetic.

```rust
// Extract next_page_token from response
let new_page_token = json.get("next_page_token")
    .and_then(|t| t.as_str())
    .map(|s| s.to_string());

// Use it for next request instead of manual start date
if let Some(ref token) = next_page_token {
    params.push(("page_token", token.clone()));
} else {
    params.push(("start", start_str)); // only first request
}
```

**Eliminated:** `consecutive_empty` counter, empty-gap skip logic, stale-chunk detection (comparing last-bar dates), period-appropriate jump table. ~50 lines of complexity removed.

### 2. Adaptive Rate Limiting

**Before:** Fixed 320ms between all requests. No response to progressive throttling.

**After:** `RateLimiter` tracks response latency and adjusts pacing:

```rust
pub struct RateLimiter {
    last_request: Arc<Mutex<Instant>>,
    cooldown_until: Arc<Mutex<Option<Instant>>>,
    adaptive_ms: Arc<Mutex<u64>>,  // NEW: dynamic interval
}
```

| Condition | Action |
|---|---|
| Response < 2s | Gradually recover toward 320ms base |
| Response > 10s | Increase interval by 200ms (cap 5s) |
| HTTP 429 | Double interval (cap 5s) + 60s cooldown |

This **prevents** progressive throttling rather than suffering through it. By backing off when the API slows down, we maintain consistent throughput instead of triggering harder throttling.

### 3. Tighter Crypto Lookback

Crypto trades 24/7 (6× more bars/day than stocks at same timeframe). The old lookback was sized for stocks.

| Timeframe | Before (days) | After (days) | Reduction |
|---|---|---|---|
| 1Min | 7 | 3 | 57% |
| 5Min-30Min | 30 | 14 | 53% |
| **1Hour** | **365** | **90** | **75%** |
| **4Hour** | **730** | **180** | **75%** |
| 1Day | 3650 | 1825 | 50% |
| 1Week | 7300 | 3650 | 50% |

Stock lookback unchanged. The proportional formula (`bars_needed / bars_per_day * 1.5`) still applies — these are just the *max* caps.

### 4. Early Termination on Progressive Throttle

If any single chunk takes >60 seconds and we already have >100 bars, accept the data and stop. This prevents hours-long fetches for historical data the user may not need.

```rust
if chunk_elapsed_ms > (SLOW_CHUNK_THRESHOLD_SECS * 1000) && all_bars.len() > 100 {
    tracing::warn!("...accepting {} bars to avoid progressive throttle", all_bars.len());
    break;
}
```

### 5. Enhanced Progress Logging

Each chunk now logs: chunk number, bars added, date range, total bars, completion percentage, elapsed time, and ms/chunk. This gives visibility into fetch progress and helps diagnose throttling.

```
BTC/USD @ 1Hour: chunk #4 +180 bars (2026-01-11 → 2026-01-18), total 718 (71%, 17s elapsed, 292ms/chunk)
```

## Performance (After)

| Symbol | Timeframe | Bars | Chunks | Time (Before) | Time (After) | Speedup |
|---|---|---|---|---|---|---|
| BTC/USD | 1Hour | ~2,160 | ~8 | 2.5+ hours | **~30s** | **300×** |
| SOL/USD | 4Hour | ~1,080 | ~5 | 2+ hours | **~15s** | **480×** |
| ADA/USD | 1Hour | ~843 | 4 | ~8 min | **~17s** | **28×** |
| CC | 4Hour | ~158 | 2 | ~3 min | **~2s** | **90×** |
| Full MTF grid | mixed | all | ~25 | 3-4 hours | **~3-5 min** | **50-60×** |

### Paid Tier Projection

Alpaca Algo Trader+ ($99/mo) provides consistent 200 req/min with no progressive throttling:

| Scenario | Free + Optimized | Paid + Optimized |
|---|---|---|
| BTC/USD 1Hour (2,160 bars) | ~30s | ~10s |
| SOL/USD 4Hour (1,080 bars) | ~15s | ~5s |
| Full MTF grid cold load | ~3-5 min | ~45-90s |
| Subsequent loads (cached) | instant | instant |

**Verdict:** Free-tier optimizations close most of the gap. Paid tier's main advantages are now: (1) SIP feed (real-time, all exchanges), (2) consistent latency with no adaptive backoff, (3) more headroom for deep historical data.

## Files Changed

| File | Change |
|---|---|
| `src-tauri/src/broker/alpaca.rs` | Rewrote `get_bars()` chunk loop, added `adaptive_ms` to `RateLimiter`, added `report_latency()`, added `SLOW_CHUNK_THRESHOLD_SECS`, tighter crypto lookback caps |
| `cli/src/broker.rs` | Matching crypto lookback reduction |

## Consequences

- **Pro**: 50-300× faster cold loads for crypto — charts usable in seconds, not hours
- **Pro**: Adaptive pacing prevents progressive throttling (benefits all tiers)
- **Pro**: `page_token` pagination eliminates overlap/gap bugs (simpler, more correct)
- **Pro**: Early termination prevents unbounded fetch times
- **Pro**: Enhanced logging gives visibility into fetch progress
- **Pro**: All improvements apply to paid tier too (no wasted budget)
- **Con**: Crypto lookback capped at 90 days (1Hour) / 180 days (4Hour) — users wanting deep crypto history would need to increase `max_lookback_days`
- **Con**: Early termination means some charts may have fewer historical bars than requested during heavy throttling
