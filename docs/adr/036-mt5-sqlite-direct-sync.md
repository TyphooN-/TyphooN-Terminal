# ADR-036: MT5 SQLite Direct Sync — Multi-Source Data Hierarchy

**Status:** Implemented
**Date:** 2026-03-21

> **Relates to:** [ADR-003](003-bar-data-caching.md) (cache architecture), [ADR-020](020-cache-optimization.md) (SQLite cache), [ADR-027](027-binary-storage-wasm-gpu.md) (binary bar storage).

## Context

TyphooN-Terminal originally relied solely on Alpaca's REST API for bar data. Alpaca's free tier has a 15-minute delay on US equities (IEX feed) and progressive throttling on sustained fetches. Meanwhile, Darwinex MetaTrader 5 provides real-time data for ~895 symbols (CFDs, forex, crypto, commodities, indices, stocks) across multiple account types.

Running MT5 under Wine on Linux, the BarCacheWriter EA exports OHLCV bars directly to SQLite databases — the same binary TTBR format used by TyphooN-Terminal's cache. This creates a zero-latency data pipeline from MT5 to the charting terminal.

## Decision

Implement a multi-source data hierarchy where MT5 is the primary real-time data source and Alpaca fills gaps for symbols not available on Darwinex.

### Data Hierarchy

```
Priority 1: Darwinex MT5 (real-time via BarCacheWriter → SQLite)
  └─ ~895 symbols: CFDs, forex, crypto, commodities, indices, stocks
  └─ All timeframes: M1, M5, M15, M30, H1, H4, D1, W1, MN1
  └─ Zero API calls — direct SQLite read from Wine filesystem

Priority 2: Alpaca Markets (15-min delayed free tier)
  └─ 11,000+ US equities, ETFs, options, crypto
  └─ Fills gaps for symbols not on Darwinex
  └─ REST API with adaptive rate limiting (see ADR-035)
```

MT5 provides the eyes (real-time data), Alpaca provides the hands (order execution for US equities). For symbols available on both, MT5 data takes precedence due to zero latency.

## Architecture

### BarCacheWriter (MQL5 EA — Source)

```
MT5 Instance (Wine) → BarCacheWriter.mq5
  ├─ Writes TTBR binary bars to SQLite: MQL5/Files/typhoon_mt5_cache.db
  ├─ Key format: "mt5:SYMBOL:TIMEFRAME"
  ├─ Metadata keys: "mt5:__SYMBOLS__:ACCOUNT_ID", "mt5:__SPECS__:ACCOUNT_ID"
  ├─ Specs CSV: Symbol, Sector, Industry, TradeMode, SwapLong, SwapShort, Spread
  ├─ Smart account detection: skips forex on specialized accounts (crypto/futures)
  └─ Runs on multiple MT5 instances simultaneously (3 Darwinex accounts)
```

### Sync Engine (Rust — Consumer)

```
sync_mt5_sqlite() — 3-phase pipeline
  ├─ Phase 1: Metadata scan (~40-96ms)
  │   └─ Bulk-load cache metadata, query all MT5 DBs via covering index
  │   └─ Filter: only import entries newer than cache (timestamp comparison)
  │   └─ Dedup: latest timestamp wins across instances
  │
  ├─ Phase 2: Parallel data read (~1-173ms)
  │   └─ std::thread::scope — one thread per DB
  │   └─ Read BLOB data only for entries that passed metadata filter
  │   └─ Emit progress events per symbol:TF for frontend UI
  │
  └─ Phase 3: Parallel compress + batch write (~25-956ms)
      ├─ rayon: Parallel zstd compression (level 3)
      └─ Single-transaction batch write to terminal SQLite cache
```

### Database Discovery

```rust
fn find_all_mt5_sqlite_dbs() -> Vec<PathBuf>
  └─ Scans ~/.wine/drive_c/Program Files/Darwinex MetaTrader 5/MQL5/Files/
  └─ Also checks mt5-instance-{1..20} Wine prefixes
  └─ Returns all typhoon_mt5_cache.db files found
```

### Sync Frequency

- **Continuous polling**: Every 30 seconds during runtime
- **On-demand**: Frontend triggers sync via `sync_mt5_sqlite` Tauri command
- **Incremental**: Only imports entries with newer timestamps than cache

## Performance

### Measured Sync Times (3 databases, ~4400 entries, ~895 symbols)

| Phase | Typical | Notes |
|---|---|---|
| Metadata scan | 40-96ms | Covering index: `idx_bar_meta(key, timestamp, bar_count)` |
| Data read | 1-173ms | Parallel across DBs, only changed entries |
| Compress + write | 25-956ms | Proportional to entries imported |
| **Total cycle** | **<2 seconds** | For incremental sync (few changes) |
| **Full initial sync** | **~66 seconds** | 4870 entries, all symbols, all timeframes |

### SQLite Optimization

**MT5 source databases** (read-only):
- `SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_NO_MUTEX` — no write contention
- `busy_timeout(30s)` — handles Wine/MT5 concurrent writes
- Covering index scan for metadata — no BLOB reads in Phase 1

**Terminal cache** (see ADR-020):
- WAL mode, `synchronous=NORMAL`, 64MB page cache, 256MB mmap
- Batch writes in single transaction (thousands of entries atomically)

## Frontend Integration

### Sync Status UI

The frontend displays a comprehensive sync dashboard:
- Per-sector progress bars with completion counts
- Symbol-level status (complete / partial / pending)
- Timeframe coverage per symbol
- Sync cycle counter and timing

### Progress Events

```javascript
window.__TAURI__.event.listen("mt5-sync-progress", (event) => {
    // Per-entry progress during Phase 2 parallel read
    const { symbol, timeframe, instance, bar_count } = event.payload;
});
```

## Multi-Account Support

BarCacheWriter tags metadata with account IDs, enabling:
- **Main CFD account**: Exports everything (forex, commodities, stocks, ETFs)
- **Crypto account**: Exports crypto pairs only, skips forex (avoids redundant writes)
- **Futures account**: Exports futures only, skips forex

Detection is automatic: accounts with <100 symbols are classified as specialized. The main account (hundreds of symbols) exports the full set. Deduplication across accounts uses latest-timestamp-wins.

## Specs Integration

`get_mt5_specs()` reads the `__SPECS__` CSV key to provide:
- **Sector/Industry classification** — used in frontend for sector-grouped progress display
- **TradeMode** — identifies which symbols are tradeable vs view-only
- **Swap rates** — long/short swap costs for carry trade analysis
- **Spread** — typical spread for cost estimation

## Consequences

- **Pro**: Real-time data for ~895 symbols — no API delay, no rate limits
- **Pro**: Zero API calls for MT5 symbols — SQLite reads are local I/O
- **Pro**: Same TTBR binary format — no conversion between MT5 and cache
- **Pro**: Multi-account support — 3 Darwinex instances syncing simultaneously
- **Pro**: Incremental sync — only changed entries imported (<2s typical)
- **Pro**: Sector/industry metadata — rich classification from MT5 specs
- **Pro**: Alpaca fills gaps — 11K+ US equities not on Darwinex
- **Con**: Requires Wine + MT5 running with BarCacheWriter EA loaded
- **Con**: MT5 data is Darwinex-specific — CFD prices, not exchange prices
- **Con**: Wine prefix discovery is hardcoded to Darwinex paths
