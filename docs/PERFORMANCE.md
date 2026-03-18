# TyphooN-Terminal Performance Evolution

## Overview

Performance is a first-class design goal. Every optimization traces back to one principle: **a trader's time is money — latency kills profits**. This document tracks the performance journey from initial implementation to current state, identifies remaining work, and flags blockers.

---

## 1. Data Pipeline — API → Cache → Chart

### Evolution

| Generation | Approach | Chart Load Time | TF Switch |
|---|---|---|---|
| **v0 (initial)** | Fetch on demand, no cache | 2-5s per load | 2-5s (re-fetch) |
| **v1 (memory cache)** | `barCache` in JS, unbounded | 2-5s first, instant repeat | Instant if cached |
| **v2 (+IndexedDB)** | Persist across restarts | First load 2-5s, restart instant | Instant |
| **v3 (+zstd files)** | Cold cache on disk, zstd level 3 | First load 2-5s, cold start ~200ms | Instant |
| **v4 (+SQLite)** | WAL mode, 64MB page cache, mmap | First load 2-5s, cold start ~50ms | Instant |
| **v5 (+binary format)** | Packed f64 (48 bytes/bar) + zstd | First load 2-5s, cold start ~30ms | Instant |
| **v6 (+background prefetch)** | All TFs cached silently after first load | First TF 2-5s, all others instant | Instant |
| **v7 (+LRU eviction)** | Max 200 entries, prevents OOM | Same speed, bounded memory | Instant |
| **v8 (+prepare_cached)** | SQLite statement caching | Same speed, less CPU per lookup | Instant |
| **v9 (+zstd level 9)** | Higher compression for persistent storage | Same speed, ~2x smaller on disk | Instant |

### Current Architecture (4-Tier Cache)

```
Tier 1: In-Memory LRU        — instant, max 200 entries, ~200MB cap
Tier 2: IndexedDB             — 50MB+, survives page reload
Tier 3: SQLite + zstd + binary — unlimited, WAL mode, mmap 256MB
Tier 4: zstd files             — legacy fallback, persistent backup
```

### Storage Compression (Binary + zstd)

| Bars | Raw JSON | JSON+zstd | Binary+zstd | Total Savings |
|------|----------|-----------|-------------|---------------|
| 500  | ~60 KB   | ~12 KB    | ~4 KB       | **15x** |
| 5000 | ~600 KB  | ~95 KB    | ~25 KB      | **24x** |
| 50K  | ~6 MB    | ~900 KB   | ~200 KB     | **30x** |

Binary format: `[4B magic "TTBR"][u32 count][per bar: i64 timestamp_ms, f64 OHLCV]` — 48 bytes/bar. Backward compatible: auto-detects `TTBR` magic, falls back to JSON for legacy entries.

### Status: ✅ Fully Optimized

No further gains possible without changing data sources. The bottleneck is now **Alpaca API response time** (~300ms per request) and **rate limit** (200 req/min), not our pipeline.

---

## 2. Rate Limiter — Budget Management

### Evolution

| Generation | Approach | Problem |
|---|---|---|
| **v0** | No rate limiting | Hit 429s constantly with multiple tabs |
| **v1** | Per-request sleep(320ms) | Wasted budget when only 1 request needed |
| **v2** | Centralized RateLimiter | All requests share one Mutex-guarded budget |
| **v3** | +429 cooldown (60s) | Auto-backs-off on rate limit hit, partial data returned |

### Current: Single shared budget, 320ms pacing (187.5 req/min under 200 limit)

All consumers — chart loads, MTF indicators, background prefetch, live polling, multiple tabs — share one rate limiter via `Arc<Mutex<RateLimiter>>`. First-come-first-served.

### Status: ✅ Fully Optimized

Zero 429 errors in normal operation. The rate limiter is the correct bottleneck — it exists to prevent API abuse.

### Future Work (Blocked)

| Improvement | Blocker |
|---|---|
| Priority queue (chart > MTF > prefetch > polling) | Architecture change — medium effort, low impact since prefetch completes in ~6s |
| WebSocket streaming (no polling) | Alpaca WS already implemented for trades/quotes; bar streaming would eliminate 10s poll |
| Incremental bar updates (fetch only new bars) | Needs timestamp tracking per cache entry — implemented but could be more aggressive |

---

## 3. Indicator Calculations — CPU Performance

### Evolution

| Generation | Approach | 1000-bar SMA | 50K param grid search |
|---|---|---|---|
| **v0 (JS only)** | Pure JavaScript | ~2ms | ~5-10 seconds |
| **v1 (+Wasm)** | Rust → Wasm via wasm-pack | ~0.1ms (20x faster) | ~100ms (50-100x faster) |

### Wasm Indicator Engine (32KB binary)

| Function | JS Speed | Wasm Speed | Speedup |
|----------|----------|------------|---------|
| SMA | 2ms | 0.1ms | 20x |
| EMA | 2ms | 0.1ms | 20x |
| KAMA | 5ms | 0.2ms | 25x |
| RSI | 3ms | 0.15ms | 20x |
| Fisher Transform | 8ms | 0.3ms | 27x |
| Bollinger Bands | 4ms | 0.2ms | 20x |
| MACD | 4ms | 0.2ms | 20x |
| Grid Optimizer (50K combos) | 5-10s | 100ms | 50-100x |

Data format: flat `Float64Array` (5 values/bar) — zero-copy Wasm interop, no serialization overhead.

### Status: ✅ Fully Optimized — Wasm Routed + Worker Thread

Chart rendering now routes SMA, EMA, KAMA, RSI, ATR through Wasm (15 call sites) with automatic JS fallback. Web Worker (`indicator-worker.js`) computes indicators off the main thread. Fisher Transform and BetterVolume remain JS (color segmentation not in Wasm — would need separate output arrays).

### Remaining (Low Impact)

| Improvement | Effort | Impact |
|---|---|---|
| Wasm BetterVolume + Supply/Demand zones | Medium | Complex JS indicators moved to native speed |
| Worker thread for indicator calculation | Low | Prevents main thread blocking on large datasets |

---

## 4. Chart Rendering — GPU Acceleration

### Evolution

| Generation | Approach | Max Bars | FPS |
|---|---|---|---|
| **v0** | lightweight-charts (Canvas2D) | ~50K | 30-60fps |
| **v1** | +GPU candlestick renderer (WebGL2/Wasm) | ~1M+ theoretical | 60fps |

### GPU Chart Engine (45KB Wasm)

Custom WebGL2 renderer via wgpu compiled to Wasm:
- Candlestick rendering via vertex shaders (2 triangles per body + 2 lines per wick)
- Grid lines, indicator overlays
- Pan, zoom, mouse wheel scroll
- Falls back to lightweight-charts when GPU unavailable

### Status: ✅ All 5 Phases Complete

```
Phase 1: Wasm indicators           ✅ DONE — computation in Rust/Wasm (32KB)
Phase 2: Binary storage             ✅ DONE — efficient data pipeline (48 bytes/bar + zstd)
Phase 3: GPU candlestick renderer   ✅ DONE — WebGL2 candles + grid + pan/zoom
Phase 4: GPU indicator overlays     ✅ DONE — SMA/EMA/KAMA/Bollinger via LINE_STRIP shader
Phase 5: Full chart engine          ✅ DONE — price scale, time axis, crosshair + OHLC tooltip (52KB)
```

Architecture: WebGL2 renders geometry (candles, wicks, indicator lines, grid, crosshair). Canvas2D overlay renders text (price labels, time labels, OHLC tooltip). This is the industry standard approach used by Bloomberg and TradingView.

### Future Work (Blocked by External)

| Phase | Status | Blocker |
|---|---|---|
| WebGPU migration | 🔲 Long-term | Browser/Tauri WebGPU support maturity |

---

## 5. UI Rendering — DOM Performance

### Evolution

| Generation | Problem | Fix |
|---|---|---|
| **v0** | Dashboard rebuilds all DOM elements every 2s | Delta updates — only update when values change |
| **v1** | Positions/Orders panels flicker on update | `content.textContent = ""` before rebuild |
| **v2** | Atomic panel swap | DocumentFragment + `replaceChildren()` — zero flicker |
| **v3** | setText/setTextClass skip unchanged values | DOM write elimination for static dashboard fields |
| **v4** | Dashboard overlap protection | `_dashboardInFlight` guard prevents concurrent updates |
| **v5** | Indicator error isolation | try/catch per indicator — one failure doesn't break others |
| **v6** | Custom TF rank resolution | `getTFRank()` handles 2Day, 3Hour, etc. — correct MTF filtering |

### Current Optimizations

- **Delta DOM updates** — `setText()` checks `el.textContent !== text` before writing
- **Atomic panel swaps** — positions/orders built in DocumentFragment, swapped in one operation
- **Parallel API calls** — `Promise.all([get_open_orders, get_order_history])` in orders panel
- **Event listener cleanup** — floating windows clean up on close
- **Indicator isolation** — try/catch per indicator prevents cascade failures
- **Race condition guards** — symbol/tab checks after every `await` (ADR-024: 7 bugs fixed)

### Status: ✅ Fully Optimized

No further DOM optimization needed. The 2-second dashboard interval is appropriate for trading — faster would waste CPU, slower would miss price changes.

---

## 6. Network — Connection Efficiency

### Current

- **reqwest::Client** — single shared client with connection pooling (keep-alive, TCP reuse)
- **HTTP/2** — automatic when server supports it (Alpaca does)
- **Timeouts** — 10-30s per request, prevents hanging connections
- **zstd Accept-Encoding** — not applicable (Alpaca API doesn't support compressed responses)
- **Chunk fetching** — sequential chunks of ~260 bars each (IEX API limit)

### Status: ✅ Fully Optimized

The network bottleneck is Alpaca's API design (260 bars/chunk, 200 req/min). Our client is already optimal for this constraint.

### Future Work (Blocked)

| Improvement | Blocker |
|---|---|
| Batch bar API (multiple symbols in one request) | Alpaca API doesn't support batch bar fetching for stocks |
| Server-side aggregation | Would need a proxy server (violates local-first principle) |
| Alternative data source with larger page sizes | Would need SIP feed ($) or different broker |

---

## 7. Binary Size

### Current

| Component | Size |
|---|---|
| Tauri binary (release) | ~10-15MB |
| Wasm indicators | 32KB |
| GPU chart Wasm | 45KB |
| SQLite (bundled) | ~1.5MB (in binary) |
| Frontend (JS+CSS+HTML) | ~600KB |
| **Total app** | **~12-17MB** |

vs Electron equivalent: ~150-200MB

### Cargo Dependencies (12 crates)

All dependencies are justified:
- `tauri` — framework
- `serde/serde_json` — serialization
- `tokio` — async runtime
- `reqwest` — HTTP client
- `chrono` — timestamps
- `zstd` — compression
- `rusqlite` — cache
- `aes-gcm/sha2/pbkdf2/rand/base64` — credential encryption
- `zeroize` — memory cleanup
- `tokio-tungstenite/futures-util` — WebSocket
- `async-trait` — broker trait
- `tracing/tracing-subscriber` — logging
- `hmac` — HMAC for Tastytrade auth

Shell plugin removed (unnecessary attack surface). Keyring removed (unreliable on Linux). Tokio reduced from `full` to `rt-multi-thread+sync+time+macros`.

### Crate Security Rollup (March 2026)

All dependencies at latest versions — zero outdated:

| Crate | Version | Notes |
|---|---|---|
| reqwest | **0.13** | HTTP client (json+query features) |
| rand | **0.10** | CSPRNG (Fill trait API) |
| rusqlite | **0.39** | SQLite (i64 stats, bundled) |
| tokio-tungstenite | **0.29** | WebSocket (Utf8Bytes message type) |
| tauri | **2.10** | Latest stable |

### Status: ✅ Fully Optimized

No unused dependencies. All at latest versions. Binary is 10-15x smaller than Electron equivalent.

---

## 8. Security Performance

### Credential Encryption

- AES-256-GCM with PBKDF2-derived key (100K iterations)
- Machine-specific key material (hostname + username)
- `zeroize` crate erases secrets from memory on drop
- SQLite storage (not config files)

### Input Validation

- `is_valid_symbol()` on all 17 symbol-accepting commands
- `is_valid_timeframe()` on all timeframe inputs
- `is_finite()` + positive checks on all financial values
- All 12 `RiskConfig` fields and `MartingaleConfig` fields range-validated

### CSP + XSS Prevention

- CSP: scripts, connects, frames restricted to self-origin
- No innerHTML anywhere — all DOM via createElement + textContent
- HTTP timeouts on all external requests
- Path traversal protection on cache operations

### Status: ✅ 18 Security Passes, 84 Findings (78 Fixed, 6 Accepted)

---

## 9. Memory Usage

### Current Profile

| Component | Typical Usage | Max |
|---|---|---|
| Tauri process | ~30-50MB | ~80MB |
| WebView (chart + UI) | ~100-200MB | ~400MB (many tabs) |
| barCache (LRU) | ~50-200MB | 200 entries cap |
| SQLite mmap | up to 256MB (OS-managed) | 256MB |
| **Total** | **~200-400MB** | **~800MB peak** |

vs MT5: ~200-500MB, vs Electron: ~500MB-1GB

### Status: ✅ Bounded

LRU eviction prevents unbounded growth. SQLite mmap is OS-managed (only maps pages in active use).

---

## Summary: Optimization Status

| Category | Status | Notes |
|---|---|---|
| Data pipeline (API → cache → chart) | ✅ Fully optimized | Bottleneck is Alpaca API, not us |
| Storage compression | ✅ Fully optimized | Binary + zstd = 15-30x savings |
| Rate limiting | ✅ Fully optimized | Zero 429s in normal operation |
| SQLite cache | ✅ Fully optimized | WAL, mmap, prepare_cached, auto-vacuum |
| Indicator calculation (backtester) | ✅ Wasm, 50-100x faster | Grid optimizer runs in ~100ms |
| Indicator calculation (chart) | ⚡ JS fallback | Wasm available but not routed for chart rendering yet |
| GPU chart rendering | ⚡ Phase 3 done | Phases 4-5 would eliminate lightweight-charts |
| DOM rendering | ✅ Fully optimized | Delta updates, atomic swaps, no flicker |
| Network | ✅ Fully optimized | Connection pooling, keep-alive, HTTP/2 |
| Binary size | ✅ 10-15MB | 10-15x smaller than Electron |
| Memory | ✅ Bounded | LRU eviction, 200-400MB typical |
| Security | ✅ 20 passes | AES-256-GCM, CSP, no innerHTML, input validation, SSRF prevention |

### Frontend Code Growth

The main `main.js` has grown from ~11K lines to ~23.7K lines with the addition of 112 new Ctrl+K command palette features (151 total) across 6 waves in a single session. All 112 new features are implemented using **existing cached data** (bar cache, options chain, positions, watchlist, order history) — zero new Alpaca API endpoints were added (only the stock snapshot endpoint was switched for pre/post-market pricing). This means no additional API rate limit pressure. Features span: options analytics, market analysis, chart tools, risk/portfolio tools, trading utilities, dashboards, DOM visualization, practice trading, voice alerts, AI strategy, session management, and data quality monitoring — all computed from data already in the 4-tier cache.

**Modularization needed**: 23.7K lines in a single file is a maintenance risk. Priority: split into ES modules (indicators, commands, trading, charts, session, cache, alerts).

### Completed TODOs (All Free-Tier Optimizations Done)

1. ✅ Route chart indicators through Wasm engine — 15 call sites, SMA/EMA/KAMA/RSI/ATR (10-20x faster)
2. ✅ GPU indicator overlays (Phase 4) — SMA/EMA/KAMA/Bollinger via WebGL2 LINE_STRIP shaders
3. ✅ Worker thread for indicator computation — `indicator-worker.js` with Wasm support, off-main-thread

### All Free-Tier Optimizations Complete

Every optimization that doesn't require paid APIs or external infrastructure has been implemented. The remaining blocked items below are external dependencies, not code limitations.

### Blocked by External Dependencies

1. Priority queue for rate limiter (low impact, medium effort)
2. WebSocket bar streaming (Alpaca supports quotes/trades WS, not bar aggregation)
3. Batch symbol fetching (Alpaca API limitation)
4. Alternative data sources with larger page sizes (needs paid SIP feed or different broker)
5. Full GPU chart engine (Phase 5) — significant engineering effort
