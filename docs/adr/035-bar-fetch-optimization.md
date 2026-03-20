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

## Measured Performance (After — Three Runs)

### Run 1: First cold load after optimization

| Symbol | Timeframe | Bars | Chunks | Time | Speedup vs Before |
|---|---|---|---|---|---|
| BTC/USD | 1Hour | 2,175 | 13 | **131s** | **~70×** |
| SOL/USD | 4Hour | 1,084 | 21 | **163s** | **~45×** |
| BTC/USD | 4Hour | 500 | 12 | **35s** | — |
| ADA/USD | 1Hour | 843 | 4 | **17s** | **~28×** |
| CC | 4Hour | 158 | 2 | **2s** | **~90×** |
| SLV | 1Hour | 302 | 2 | **4s** | **~30×** |
| LUMN | 4Hour | 116 | 2 | **4s** | — |
| SMCI | 4Hour | 142 | 2 | **5s** | — |

### Run 2: Second session (warm adaptive state)

| Symbol | Timeframe | Bars | Chunks | Time |
|---|---|---|---|---|
| BTC/USD | 1Hour | 2,175 | 13 | **33s** |
| SOL/USD | 4Hour | 1,084 | 21 | **57s** |
| BTC/USD | 4Hour | 500 | 12 | **30s** |
| ADA/USD | 1Hour | 843 | 4 | **17s** |
| CC | 4Hour | 158 | 2 | **2s** |

### Run 3: Third session (consistent results)

| Symbol | Timeframe | Bars | Chunks | Time |
|---|---|---|---|---|
| BTC/USD | 1Hour | 2,175 | 13 | **46s** |
| SOL/USD | 4Hour | 1,084 | 21 | **47s** |
| BTC/USD | 4Hour | 500 | 12 | **~30s** |
| ADA/USD | 1Hour | 843 | 4 | **17s** |
| Stocks (CC/SLV/LUMN/SMCI) | mixed | ~100-300 | 1-2 ea | **2-6s** |
| Prefetch (all TFs) | mixed | all | ~50 | **~60s** |

### Summary: Before vs After

| Scenario | Before | After (avg) | Speedup |
|---|---|---|---|
| BTC/USD 1Hour | 2.5+ hours | **33-131s** | **70-270×** |
| SOL/USD 4Hour | 2+ hours | **47-163s** | **45-150×** |
| Stocks (4H) | 1-3 min each | **2-6s** | **20-90×** |
| Full MTF grid + prefetch | **3-4 hours** | **~3 min** | **60-80×** |
| Subsequent loads (cached) | instant | instant | — |

### Chunk Latency Behavior

The adaptive pacer prevents progressive throttling. Observed chunk latency patterns:

| Phase | Chunk Latency | Adaptive Interval |
|---|---|---|
| First 5 chunks | 100-300ms | 320ms (base) |
| Chunks 5-10 | 300-600ms | 320-520ms |
| Chunks 10-15 | 400-1200ms | 520-720ms |
| Chunks 15-21 (SOL 4H) | 600-2100ms | 720-920ms |
| Recovery (next symbol) | 100-300ms | recovers to 320ms |

Without adaptive pacing, chunk latency grew exponentially to 7-10 minutes. With it, worst case stays under 2.5 seconds — the pacer absorbs throttle pressure by spacing requests wider.

### Progress Percentage Fix

Initial implementation showed `bars_fetched / raw_limit * 100`, which gave misleading 0-4% for prefetch calls (limit=50000 vs ~2000 available bars). Fixed to:

```rust
let expected_bars = (lookback_days as f64 * bars_per_day).ceil() as usize;
let bars_target = expected_bars.min(actual_limit as usize).max(1);
let pct = (total * 100) / bars_target;
```

Now BTC/USD 1Hour shows: 8% → 16% → 24% → 33% → 41% → 49% → 56% → 64% → 100%.

### Paid Tier Projection

Alpaca Algo Trader+ ($99/mo) provides consistent 200 req/min with no progressive throttling:

| Scenario | Free + Optimized | Paid + Optimized | Improvement |
|---|---|---|---|
| BTC/USD 1Hour (2,175 bars) | ~33-131s | **~10s** | 3-13× |
| SOL/USD 4Hour (1,084 bars) | ~47-163s | **~5s** | 9-32× |
| Full MTF grid cold load | ~3 min | **~45-90s** | 2-4× |
| Subsequent loads (cached) | instant | instant | — |

**Verdict:** Free-tier optimizations close most of the gap. Paid tier's remaining advantages:

1. **SIP feed** — real-time all-exchange data vs IEX 15-min delayed single-exchange
2. **Consistent latency** — no adaptive backoff needed, always 100-300ms/chunk
3. **Deep history** — can increase crypto lookback caps without throttle risk
4. **Larger page sizes** — crypto may return more bars per chunk on paid tier (currently ~45-65/chunk on free)

## Implemented Improvements (Phase 2)

### ✅ Incremental Cache-Aware Fetch

New `get_bars_incremental` Tauri command checks SQLite cache before hitting the API. If cached bars exist, fetches only the gap since the **second-to-last** cached bar (not the last — because the last candle is still forming and its OHLCV values are live/updating until the period closes).

**Architecture:**
```
Frontend: cachedGetBars() → invoke("get_bars_incremental")
Backend:  1. Check SQLite: get_incremental_start(key) → second-to-last bar timestamp
          2. If cache hit: broker.get_bars_after(symbol, tf, limit, after_ts) → fetch gap only
          3. Merge new bars into cache: cache.merge_bars(key, new_json) → dedup + sort + store
          4. Return full merged dataset
          5. If no cache: full fetch, store to SQLite for next time
```

**Impact:** 80-95% fewer API calls on warm start. BTC/USD 1Hour goes from 13 chunks to 1-2 chunks on second session. Applies to both GUI and CLI (shared SQLite cache).

**Key design decision:** Always re-fetch the live candle. Higher timeframe candles (H4, D1, W1) are "living" until the period closes — their high/low/close update continuously. The incremental start point is the second-to-last bar, ensuring the live candle is always refreshed from the API.

### ✅ WebSocket Live Bar Construction

New `BarBuilder` module (`core/bar_builder.rs`) constructs 1-minute OHLCV bars from the WebSocket trade stream in real-time:

**Architecture:**
```
WebSocket → StreamTrade(symbol, price, size, timestamp)
  → BarBuilder.ingest_trade(symbol, price, size, timestamp)
    → Accumulate into PartialBar (same minute)
    → When new minute starts → complete previous bar, start new one
  → poll_bars command:
    → BarBuilder.drain_completed() → Vec<CompletedBar> (finished 1-min candles)
    → BarBuilder.get_all_active_bars() → Vec<CompletedBar> (live candles still forming)
    → Returns { completed: [...], active: [...] }
  → Frontend renders completed bars + live candle updates
```

**API:**
- `ingest_trade(symbol, price, size, timestamp)` — feed WS trades into the builder
- `drain_completed()` — returns and clears all completed 1-min bars since last drain
- `get_active_bar(symbol)` — returns the currently-forming candle for a symbol
- `get_all_active_bars()` — returns all active candles across all symbols

**Impact:** Real-time candle updates (2s polling vs 10s API polling). Eliminates API calls for live bar updates when WebSocket is connected. Falls back to API polling when WS is down.

**Frontend integration:** `startWsBarPolling()` runs every 2s, processes completed bars (appends to chart, triggers indicator recalc) and active bars (updates live candle). Only affects 1Min chart directly; higher TFs still use API polling for now.

### ✅ Predictive Prefetch

On broker connect, immediately fetches open positions and prefetches all timeframes for each position symbol:

```javascript
// After successful connect:
const positions = await invoke("get_positions");
const symbols = [...new Set(positions.map(p => p.symbol))];
for (const sym of symbols) prefetchAllTimeframes(sym, null);
```

**Impact:** With incremental fetch, this is near-free for returning users (only the gap since last session). Position symbols are warm-cached before user clicks them.

### ✅ Cache Trim After Merge

`merge_bars(key, json, max_bars)` now accepts a `max_bars` limit. After merging and deduplicating, excess bars are trimmed (oldest removed) to prevent unbounded SQLite growth. The limit matches the original request (e.g., 2000 bars for prefetch, 500 for chart).

### ✅ Fast Compression for Merge Writes

Added `put_bars_fast()` — uses zstd level 3 instead of level 9 for frequent merge operations. Level 3 is ~3× faster with only ~15% larger output. Archival storage still uses level 9. This reduces CPU overhead on the hot merge path.

### ✅ Cache Freshness Gate

`get_bars_incremental` now checks `get_cache_age_secs()` before making any API call. If the cache was updated within the last period of the timeframe (e.g., <3600s for 1Hour), it returns cached data immediately. This eliminates the SLV 1Day polling loop (was re-fetching every 60s with no new data).

### ✅ Connection Pre-Warming

`warm_data_connection()` fires a HEAD request to `data.alpaca.markets` during the connect flow. Since `get_account()` only warms the trading endpoint (`api/paper-api.alpaca.markets`), bar fetches go to a different host. Pre-warming establishes TCP+TLS ~200ms before the first bar fetch needs it.

### ✅ Broker ID Key Fix

`get_bars_incremental` now accepts `broker_id` from the frontend (`activeBrokerId`). Tries the primary key first (e.g., `alpaca_Paper:BTC/USD:1Hour`), falls back to `default:` key. This fixed crypto symbols not finding cache from previous sessions (frontend saved under `alpaca_*:` prefix, backend looked for `default:`).

### ✅ Double-Write Elimination

Frontend `cachedGetBars` no longer calls `saveBarCacheToDisk` after receiving data from `get_bars_incremental` — the backend already persists to SQLite during merge. Only hot cache (`barCache`) is updated in the frontend. This eliminates duplicate SQLite writes and the associated zstd level 9 recompression.

### ✅ Arc Cache — Lock Contention Fix

`db_cache` changed from `Option<SqliteCache>` to `Option<Arc<SqliteCache>>`. The state lock (`state.lock().await`) is now dropped immediately after cloning the `Arc` reference. Heavy operations (API fetch, `merge_bars`, zstd compress) run outside the lock. Previously the state lock was held for the entire incremental fetch cycle (seconds to minutes for crypto), which blocked all other Tauri commands and froze the UI. Now the lock is held for ~microseconds.

### ✅ Bar Data Sanitization (Dual-Layer)

**Backend (Rust, alpaca.rs):** Bars from the API are sanitized at parse time:
- Reject bars with empty timestamps or zero/NaN prices (`o <= 0.0 || !o.is_finite()`)
- Fix OHLC consistency: `true_high = max(o,h,l,c)`, `true_low = min(o,h,l,c)`
- Reject bars where volume is negative

**Frontend (JS, `sanitizeBars()`):** Called before every `setData()` and on merge results:
- Remove bars with NaN/Infinity/null/zero prices or timestamps
- Fix OHLC inconsistency (same formula as backend — defense in depth)
- Remove duplicate timestamps (keeps latest occurrence)
- Sort by time ascending (lightweight-charts requires sorted data)
- Clamp negative volume to 0

The dual-layer approach catches API anomalies (Alpaca occasionally returns malformed crypto bars) at the source and at the render boundary. Zero chart artifacts from bad data.

### Deferred

| Improvement | Reason |
|---|---|
| **Priority queue** | First-come-first-served via Mutex is adequate now that incremental fetch reduces prefetch cost to 1-2 chunks per TF. Would add complexity for marginal UX improvement. |
| **Parallel symbol fetch** | Needs testing to determine if Alpaca rate-limits per-endpoint or globally. Risk of triggering harder throttling. |
| **Shared 1Min aggregation** | Dropped: 1Min crypto = 1440 bars/day, storing 90 days = 130K bars per symbol. Per-TF incremental fetch is more efficient. |
| **CLI cache reading** | CLI is a one-shot tool, not persistent. Adding SQLite cache to CLI requires duplicating cache module. Low impact vs effort. |

### Remaining (Blocked by External)

| Improvement | Blocker |
|---|---|
| Batch multi-symbol bar API | Alpaca doesn't support batch bar requests for stocks |
| Paid tier larger chunk sizes | Needs testing on Algo Trader+ |

### Measurement Needed

| Question | How to Test |
|---|---|
| Does Alpaca paid tier return larger crypto chunks? | Subscribe to Algo Trader+ for 1 month, compare chunk sizes |
| Is rate limiting per-endpoint or global? | Fire concurrent requests to `/crypto/bars` and `/stocks/bars`, check if they share a budget |
| What's the optimal adaptive backoff curve? | A/B test linear vs exponential backoff under sustained load |

## Files Changed

| File | Change |
|---|---|
| `src-tauri/src/broker/alpaca.rs` | `page_token` pagination, adaptive `RateLimiter`, `get_bars_after()` for incremental fetch, `warm_data_connection()` for pre-warming, tighter crypto lookback |
| `src-tauri/src/core/cache.rs` | `get_incremental_start()`, `merge_bars(key, json, max_bars)` with trim + zstd level 3, `put_bars_fast()`, `get_cache_age_secs()` |
| `src-tauri/src/core/bar_builder.rs` | **New**: `BarBuilder` constructs 1-min OHLCV from WebSocket trades |
| `src-tauri/src/main.rs` | `get_bars_incremental` (cache-aware, broker_id, freshness gate), `poll_bars`, `warm_data_connection()` in connect, `BarBuilder` in AppState |
| `frontend/src/main.js` | `cachedGetBars` → `get_bars_incremental` with `brokerId`, `startWsBarPolling()`, predictive prefetch on connect, removed double-write `saveBarCacheToDisk` from `cachedGetBars` |
| `cli/src/broker.rs` | Matching crypto lookback reduction |

## Consequences

- **Pro**: 60-270× faster cold loads for crypto — charts usable in seconds, not hours
- **Pro**: Incremental fetch: 80-95% fewer API calls on warm start (1-2 chunks vs 13+)
- **Pro**: Cache freshness gate eliminates redundant polling loops (SLV 1Day was re-fetching every 60s)
- **Pro**: Live candle always refreshed (second-to-last bar start point preserves forming candle accuracy)
- **Pro**: WebSocket bar builder: real-time 1Min candle updates without API polling
- **Pro**: Predictive prefetch: position symbols warm-cached before user clicks
- **Pro**: Cache trim prevents unbounded SQLite growth
- **Pro**: Fast compression (zstd 3) for merge writes, archival (zstd 9) for initial storage
- **Pro**: Connection pre-warming saves ~200ms on first bar fetch
- **Pro**: No double-writes: backend handles SQLite, frontend handles hot cache only
- **Pro**: Adaptive pacing prevents progressive throttling (benefits all tiers)
- **Pro**: `page_token` pagination eliminates overlap/gap bugs (simpler, more correct)
- **Con**: Crypto lookback capped at 90 days (1Hour) / 180 days (4Hour)
- **Con**: WebSocket bar builder only constructs 1Min bars directly; higher TFs still need API for period-closing updates
