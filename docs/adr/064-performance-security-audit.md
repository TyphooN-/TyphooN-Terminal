# ADR-064: Performance & Security Audit

**Status:** Complete | **Date:** 2026-03-28 | **Updated:** 2026-03-30

## Context

Charting performance degraded when floating windows (crypto backfill, storage manager, etc.) were open. A comprehensive audit of the render loop identified expensive per-frame operations and verified security posture.

## Performance Issues Found & Fixed

### Critical: Crypto Backfill Window Per-Frame DB Queries
- **Root cause:** `cache.get_bars_raw(key)` called for every crypto entry every frame to render first/last timestamps in the backfill table. With 50+ entries (some 137K bars), this decompressed megabytes of zstd data 60x/sec.
- **Fix:** Cached first/last timestamps in `crypto_ts_cache` HashMap, populated once in background thread. Zero DB queries per frame.

### Critical: Watchlist Cache Population Loop
- **Root cause:** When watchlist symbols weren't found in initial lookup, `cache.detailed_stats()` (full 7GB DB scan) ran every frame.
- **Fix:** `watchlist_cache_tried` flag ensures one-time lookup. Incremental lookups only for newly added symbols.

### Critical: DARWIN Background Refresh (25s every 5min)
- **Root cause:** Full DARWIN analytics recomputation (6 accounts × ~4s each) every 300 seconds, blocking the background thread.
- **Fix:** Changed to once-per-startup. DARWIN deal data is static (imported from XLSX), no need for periodic refresh.

## Operations Verified as Acceptable

| Operation | Location | Frequency | Assessment |
|-----------|----------|-----------|------------|
| Storage filter | String contains on cached stats | Per-frame when window open | Lightweight, paginated (200/page) |
| Portfolio ranking sort | 6-element Vec sort | Per-frame when window open | Negligible (~6 items) |
| Unusual volume scan | DB queries per symbol | On-command only | Blocks briefly, results cached |
| Screener load | Single symbol decompression | On-click | Acceptable per-interaction |
| Session chart load | 4-8 decompressions | Startup only | Acceptable |

## Repaint Scheduling

```
Idle repaint: 250ms (4 fps idle)
egui internal: repaints on hover/click/scroll (60fps during interaction)
No per-frame throttling by window state — expensive operations eliminated instead
```

## Security Audit Results

| Category | Status | Details |
|----------|--------|---------|
| SQL Injection | **SAFE** | All DB ops use parameterized queries via engine methods |
| Hardcoded Secrets | **SAFE** | Credentials in system keyring (AES-256-GCM), not source |
| Unsafe Code | **SAFE** | Zero `unsafe` blocks in app.rs |
| Path Traversal | **SAFE** | MT5 paths from config, DARWIN dirs validated with is_dir() |
| Session Deserialization | **SAFE** | Defensive parsing with type checks (.as_str(), .as_bool(), .as_u64()) |
| LAN Sync Auth | **SAFE** | PBKDF2-HMAC-SHA256 (100K iterations) challenge-response |
| LAN Sync Transport | **SAFE** | TLS encrypted (wss://) with ephemeral self-signed certificate (rcgen + native-tls). PBKDF2 passphrase authenticates server identity |
| External Data | **SAFE** | All API responses parsed through typed serde structs |

### Additional Fixes (Late Session)

- **Alpaca 401 error spam**: Added HTTP status check before JSON parse in `get_positions()`. Auto-disconnect on auth failure with single clear log message instead of repeated errors.
- **Credential persistence**: Credentials now saved to keyring on Settings window close AND on application quit (was only saved on "Connect Alpaca" click).
- **Trade overlay caching**: `build_trade_overlay()` result cached per chart, rebuilt every 120 frames (~30s) instead of every frame. Eliminates 2 HashMap + chrono parsing per chart per frame.
- **MTF SMA color lookup**: Replaced O(6) linear array search with O(1) match statement.
- **CryptoCompare rate limiting**: Exponential backoff retry (2s→16s), increased inter-page delay to 2500ms.
- **Weekend crypto polling**: Switched from CryptoCompare (rate limited) to Kraken (no limits, 720 recent bars).

## Consequences

- **Pro:** Chart rendering no longer degrades with floating windows open
- **Pro:** DARWIN refresh saves 25s CPU every 5 minutes
- **Pro:** Zero SQL injection, zero hardcoded secrets, zero unsafe code
- **Pro:** All per-frame expensive operations eliminated or cached
- **Pro:** LAN sync encrypted with TLS (wss://) — ephemeral self-signed certs, no plaintext transmission
- **Pro:** Broker auth failures handled gracefully (auto-disconnect, no log spam)
- **Pro:** Credentials persist across sessions (keyring save on settings close + quit)

## 2026-03-30 Follow-Up Audit

### UI Thread Unblocking
- **Compaction flag**: DB compaction moved off UI thread, driven by background flag
- **Unusual volume scanner**: Moved to background thread, no longer blocks UI during scan
- **Watchlist stats caching**: Stats computed once and cached, not recomputed per-frame

### Infrastructure
- **Prometheus metrics endpoint**: Exposed on port 9090 for external monitoring
- **Docker containerization**: Dockerfile added for reproducible builds and deployment

### Code Quality
- **480 tests** (75 compiler + 319 engine + 86 native) — comprehensive coverage across engine, GPU shaders, integration, and MQL5 compiler
- **bytemuck migration**: All `unsafe` transmute/pointer-cast blocks replaced with `bytemuck` Pod/Zeroable derives — zero `unsafe` blocks in entire codebase

### 2026-03-31 Session: UI Freeze Elimination

- **UI thread fully unblocked**: All remaining blocking operations moved off the UI thread
  - DB compaction: driven by background flag, no longer stalls render loop
  - Unusual volume scanner: runs in background thread, results cached
  - Watchlist stats: computed once and cached, not recomputed per-frame
- **Zero unsafe blocks** confirmed across entire codebase (engine + native + compiler)
- **480 tests** passing (75 compiler + 319 engine + 86 native): cache, darwin, fundamentals, SEC, crypto, var, risk, margin, backtest, GPU shaders, app integration, parser, WASM codegen, WGSL codegen
- **MQL5 compiler parser bug fix**: postfix_op unwrapping now correctly distinguishes `++`/`--` from wrapped call_args/index_access/member_access

### 2026-03-31 → 2026-04-02: SQLite Concurrency, LAN Sync, BarCacheWriter

**SQLite Concurrency:**
- Multi-connection architecture: `conn` (write), `read_conn` (UI-exclusive reads), BG thread owns its own connection (reopened each cycle for WAL freshness), Phase 5 threads open independent connections.
- `maybe_decompress()` handles both raw TTBR (from BarCacheWriter) and zstd-compressed (from Rust) data transparently.
- `busy_timeout` set via `conn.busy_timeout(Duration)` before any PRAGMAs — prevents cascade failure if DB is locked during PRAGMA execution.
- Mt5Sync: own `SqliteCache::open()` for writes + `open_readonly()` for source reads (10s timeout).
- DB maintenance: `stats()` reports actual file size (not data size). VACUUM on purge. `incremental_vacuum(500)` every 6 hours.
- Storage Manager: Purge All Bar Data + Purge All DARWIN Data buttons with red confirmation.

**LAN Sync:**
- TLS encrypted (wss://), PBKDF2-HMAC-SHA256 auth. No TOFU cert pinning (ephemeral certs).
- 15-second periodic re-sync (bars, KV, DARWIN, research tables) — near-real-time.
- Server auto-start on startup (`lan_server_enabled` persisted). Client auto-connect (`lan_client_enabled`).
- Connected client IPs tracked and displayed in server UI (stored in KV `lan:server:clients`).
- Broker positions/account/orders stored to KV cache → LAN clients read-only view.
- **23 DARWIN analytics fields** stored to KV by server, read by LAN client — zero local deal queries on client. Covers: open_positions, portfolio, exposure, correlations, daily_returns, var_stats, monte_carlo, rolling_var, var_forecast, conditional_var, market_regime, tail_risk, seasonal_analysis, optimal_allocation, rebalance, stress_tests, margin_call_sim, drawdown_dashboard, drawdown_attribution, risk_budget, signal_decay, var_multipliers, floating_equity, per_darwin_var.
- LAN client BG thread checks `lan_client_flag` — never computes from local deals (broken 45K-deal import produces wrong results). All analytics identical between server and client.
- 300-second read timeout (DARWIN export of 45K+ deals takes >60s).
- `import_darwin_data`: FK-safe delete order (equity_snapshots → deals → positions → accounts), safety ROLLBACK before `unchecked_transaction()`.
- SEC filing content fetched directly by client (public EDGAR URLs, not forwarded to server).
- Resync buttons: Bars, DARWIN Analytics, Positions.

**BarCacheWriter v1.432:**
- In-memory merge (SQL BLOB approach reverted — MQL5 TEXT/BLOB corruption).
- Skip blob write when no new bars (95% I/O reduction).
- TF gating: only check TFs that could have new bars (90% fewer CopyRates calls).
- Batch 5, sleep 200ms, incremental_vacuum every 30min, 100K bar cap.

**Other:**
- Alpaca auto-connect on startup. tastytrade button disabled (ADR-022).
- Crypto backfill: CryptoCompare + Kraken for sub-hourly TFs (complementary, both stored).
- Status bar: LIVE when any data source connected (removed market hours logic).
- MTF indicators (SMA/KAMA) computed in `try_load()` (was missing — invisible on chart).
- TF buttons and command palette TF shortcuts use active chart's symbol (not text box).
