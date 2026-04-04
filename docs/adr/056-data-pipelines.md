# ADR-056: Data Pipelines & Rendering Architecture

**Status:** Implemented
**Date:** 2026-03-26 | **Updated:** 2026-04-05

## Overview

TyphooN Terminal has multiple data sources, processing pipelines, and rendering paths. This ADR documents how data flows from source to screen, which thread handles what, and how contention is avoided.

## Data Sources

### 1. MT5 BarCacheWriter (Primary bar data)
```
MT5 Terminal (Wine) → BarCacheWriter EA → typhoon_mt5_cache.db (MQL5/Files/)
                                              ↓ MT5SYNC command
                            typhoon_cache.db (main cache, ~/.config/typhoon-terminal/)
```
- **Format:** ZSTD-compressed binary OHLCV (6 × f64 per bar)
- **Schema:** `bar_cache(key TEXT PK, data BLOB, timestamp INT, bar_count INT)`
- **Key pattern:** `mt5:{broker}:{symbol}:{timeframe}` e.g. `mt5:CC:SLV:4Hour`
- **Sync:** User-configurable up to 4 MT5 database paths (Settings → MT5 BarCacheWriter Sources)
- **Sync mechanism:** Zero-copy blob transfer via `put_raw_blob()` — only overwrites if source timestamp is newer
- **Trigger:** Manual via `MT5SYNC` command or "Sync MT5 Data Now" button

### 2. Darwinex XLSX (DARWIN trade history)
```
MT5 → Export Trade History → .xlsx files → DarwinImportAll command
                                              ↓ (broker thread)
                            darwin_accounts, darwin_deals, darwin_positions tables
```
- **Format:** XLSX (UTF-16 LE XML inside ZIP, auto-converted)
- **Tables:** `darwin_accounts`, `darwin_deals` (62K+ rows), `darwin_positions` (30K+ rows)
- **Ticker derivation:** Filename stem → uppercase (e.g. `THA.xlsx` → "THA")
- **Trigger:** Auto on startup if no accounts exist, or manual via "Import All XLSX Now"
- **Directory:** User-configurable (Settings → DARWIN XLSX Import)

### 3. Darwinex FTP (50K DARWIN universe)
```
Darwinex FTP Server → NAS (ZFS raidz2) → darwin_ftp.rs parser
                      /mnt/bigraidz2/Darwinex_FTP/
                      50,317 DARWIN directories
```
- **Format:** Flat files per DARWIN with daily D-Score components + gzipped tick quotes
- **Components:** RETURN, TRADES, POSITIONS, EXPERIENCE, RISK_STABILITY, PERFORMANCE, etc. (23 files)
- **Quotes:** `quotes/YYYY-MM/*.csv.gz` with `timestamp,quote` tick data
- **Directory:** User-configurable (Settings → Darwinex FTP Dir)
- **Parser:** `engine/src/core/darwin_ftp.rs`
- **Access pattern:** Direct path construction (`{ftp_dir}/{ticker}/RETURN`), no recursive find

### 4. SEC EDGAR (Filings & insider trades)
```
SEC EDGAR API → reqwest HTTP → sec_filing.rs parser → SQLite tables
  data.sec.gov/submissions/
  data.sec.gov/api/xbrl/companyfacts/
```
- **Tables:** `sec_filings`, `sec_filing_alerts`, `sec_insider_trades`
- **Rate limit:** 200ms between requests (5 req/sec)
- **Trigger:** `SEC` command or "Scrape Now" button (async via broker thread)

### 5. Yahoo Finance (Fundamentals)
```
Yahoo Finance v10 API → reqwest HTTP → fundamentals.rs parser → SQLite tables
  query2.finance.yahoo.com/v10/finance/quoteSummary/
```
- **Tables:** `fundamentals`, `quarterly_financials`, `institutional_holders`
- **Data:** EV, MCap, P/E, earnings dates, dividends, quarterly financials, holders
- **Rate limit:** 300ms between requests
- **Trigger:** `EVSCRAPE` command (async via broker thread)

### 6. Alpaca Markets (Live trading)
```
Alpaca REST API → reqwest HTTP → AlpacaBroker → BrokerMsg channel
Alpaca WebSocket → tokio-tungstenite → streaming quotes/trades
```
- **Data:** Account info, positions, orders, quotes, market clock
- **Communication:** tokio mpsc unbounded channel (BrokerCmd → BrokerMsg)

### 7. Kraken (Crypto backfill)
```
Kraken Public API → reqwest HTTP → cache.put_bars()
  api.kraken.com/0/public/OHLC
```
- **Trigger:** `CRYPTO_BACKFILL` command or dedicated window

## Thread Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│ UI Thread (egui render loop)                                    │
│                                                                 │
│ update() called 2-10× per second                               │
│ ├─ Drain bg_rx channel → self.bg (BgDarwinData)               │
│ ├─ Drain broker_rx channel → positions, orders, log            │
│ ├─ Render charts (egui Painter, zero DB calls)                 │
│ ├─ Render floating windows (read from self.bg, zero DB calls)  │
│ └─ Handle button clicks (try_connection → non-blocking)        │
│                                                                 │
│ RULE: UI thread NEVER calls cache.connection() (blocking).      │
│       Uses cache.try_connection() for button actions.           │
│       Uses cache.try_get_bars_raw() for chart loading.          │
│       All rendering reads from self.bg.* cached data.           │
└─────────────────────────────────────────────────────────────────┘
        ↑ bg_rx (mpsc::channel, unbounded)
        ↑ broker_rx (tokio mpsc::unbounded_channel)
┌─────────────────────────────────────────────────────────────────┐
│ Background Data Thread (std::thread)                            │
│                                                                 │
│ Loops every 3-5 seconds:                                        │
│ ├─ Phase 1a: list_darwin_accounts (0ms) → send snapshot        │
│ ├─ Phase 1b: SEC filings + table creation (1.3s) → send       │
│ ├─ Phase 1b: portfolio_summary (300ms) + daily_returns → send  │
│ ├─ Phase 1c: detailed_stats + equity curves (400ms) → send    │
│ ├─ Phase 2: VaR/Monte Carlo/regime (pure CPU, no DB)          │
│ ├─ Phase 3: optimal allocation, rebalance, stress tests        │
│ ├─ Phase 4: per-DARWIN VaR                                     │
│ ├─ Phase 5: per-account details (6 accounts, ~20s total)       │
│ │   └─ Sends after each account (incremental updates)          │
│ ├─ Phase 6: tax lots                                           │
│ ├─ Phase 7: risk alerts                                        │
│ └─ Phase 8: SEC insider trades                                 │
│                                                                 │
│ Uses blocking cache.connection() (OK — separate thread).        │
│ Releases conn between query groups to reduce Mutex contention.  │
│ Pauses during DARWIN XLSX import (importing_flag AtomicBool).   │
└─────────────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────────────┐
│ Broker Task (tokio::spawn, async)                               │
│                                                                 │
│ Handles BrokerCmd variants:                                     │
│ ├─ Connect, GetAccount, GetPositions, GetOrders                │
│ ├─ CloseAll, ClosePosition, PlaceOrder                         │
│ ├─ SecScrape → spawns std::thread                              │
│ ├─ FinnhubNews → async reqwest                                 │
│ ├─ DarwinImportAll → spawns std::thread (importing_flag)       │
│ ├─ Mt5Sync → spawns std::thread                                │
│ ├─ FundamentalsScrape → spawns std::thread with tokio runtime  │
│ ├─ DarwinFtpScan → spawns std::thread                          │
│ ├─ DarwinGpuScan → spawns std::thread (reads FTP, sends to UI)│
│ └─ KrakenBackfill → async reqwest                              │
│                                                                 │
│ Heavy I/O commands spawn dedicated std::threads to avoid        │
│ blocking the tokio command recv loop.                           │
└─────────────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────────────┐
│ Cache-Open Thread (std::thread, one-shot)                       │
│                                                                 │
│ Opens SqliteCache from 3.9GB database file (async, ~10ms).     │
│ Publishes to Arc<RwLock> for bg thread + mpsc channel for UI.  │
│ Window appears immediately, cache arrives on first frame.       │
└─────────────────────────────────────────────────────────────────┘
```

## Rendering Pipeline

### Chart Rendering (egui Painter)
```
self.charts[active_tab].bars → draw_chart()
  ├─ Background fill (BG color)
  ├─ Grid lines (dotted, MT5 style)
  ├─ Candlestick/Bar/Line/HeikinAshi/Renko geometry
  ├─ Indicator overlays (SMA, EMA, KAMA, Bollinger, Ichimoku, etc.)
  ├─ Sub-panes (Fisher, RSI, MACD, Volume, Stochastic, ADX, etc.)
  ├─ Drawing tools (HLine, TrendLine, FibRetracement, etc.)
  ├─ Supply/Demand zones, ATR projection, Fibonacci
  ├─ Crosshair + price/time labels
  └─ SL/TP lines
```
- All data already in memory (`Vec<Bar>`, pre-computed indicator arrays)
- Zero DB calls during rendering
- Crosshair suppressed when pointer is over floating windows

### Floating Windows (egui Window)
```
self.bg.* → read-only access during rendering
  ├─ DARWIN Accounts: bg.accounts + bg.account_details
  ├─ DARWIN Portfolio: bg.portfolio + bg.var_stats + bg.correlations + ...
  ├─ SEC Filing Scanner: bg.sec_filings + bg.sec_alerts
  ├─ VaR Multiplier: bg.per_darwin_var + bg.var_multipliers
  ├─ Fundamentals: bg.all_fundamentals
  ├─ Earnings Calendar: bg.upcoming_earnings
  ├─ Dividend Calendar: bg.upcoming_dividends
  ├─ DARWIN Browser: self.ftp_scan_results + self.ftp_detail_*
  ├─ Cache Stats: bg.cache_stats + bg.detailed_stats
  └─ Risk/Positions/Orders: bg.open_positions + live_positions + live_orders
```
- All reads from `self.bg` (background-computed data)
- No DB queries in render code
- Button actions use `try_connection()` (non-blocking)

### GPU Compute Pipeline
```
CPU: Parse FTP RETURN files → Vec<Vec<f32>> daily returns
  ↓
GPU: upload_returns() → VRAM storage buffer (~100MB for 50K DARWINs)
  ↓
GPU: compute_stats() → dispatch batch_stats shader (50K threads)
  Output: 10 floats per DARWIN (mean, var, sharpe, sortino, maxdd, best, worst, skew, kurt, total_ret)
  ↓
GPU: compute_correlation_tile() → dispatch correlation shader (1024×1024 tiles)
  Output: Pearson correlation matrix tiles
  ↓
CPU: readback_stats() → staging buffer → map_async → Vec<GpuDarwinStats>
  ↓
UI: Display in DARWIN Browser table
```

## SQLite Concurrency Model

```
SqliteCache {
    conn: Mutex<Connection>   // Single connection, WAL mode
}
```

- **WAL mode:** Allows concurrent readers, single writer
- **Mutex contention:** bg thread holds lock for 200ms-2s per query batch
- **UI mitigation:** `try_lock()` / `try_connection()` — never blocks
- **Bg mitigation:** Releases conn between query groups (every 5-7 queries)
- **Import mitigation:** `importing_flag` AtomicBool pauses bg thread during XLSX import

### Why not multiple connections?
SQLite WAL mode supports multiple reader connections, but rusqlite `Connection` is not `Send`. Opening multiple connections would require separate `SqliteCache` instances per thread, each with their own PRAGMA setup and cache warming. The single-connection + `try_lock` approach is simpler and sufficient — bg queries complete in <2s per batch, UI skips frames during contention.

## Session Persistence

```json
// ~/.config/typhoon-terminal/session.json
{
  "symbol": "SLV",
  "tabs": [{"symbol": "CC", "timeframe": "H4", "chart_type": "Candle"}, ...],
  "indicators": {"sma200": true, "kama": true, ...},
  "mtf_enabled": true,
  "darwin_view": 0,
  "darwin_xlsx_dir": "/home/typhoon/mt5xml",
  "darwin_ftp_dir": "/mnt/bigraidz2/Darwinex_FTP/",
  "mt5_db_paths": ["/home/typhoon/.mt5_7/.../typhoon_mt5_cache.db", ...],
  "broker_api_key": "...",
  "windows": {"settings": false, "darwin_accounts": true, ...},
  "drawings": [{"type": "hline", "price": 18.45}],
  "alerts": [{"price": 20.0, "label": "resistance"}]
}
```

## Startup Sequence

```
T+0ms    fn new(): spawn cache-open thread, return empty app
T+0ms    Window appears (empty charts, "Loading..." states)
T+10ms   Cache-open thread: SqliteCache::open() (3.9GB, ~10ms)
T+10ms   Cache published to RwLock + channel
T+100ms  UI: cache_rx delivers → load_session() → load active chart
T+3s     BG thread: Phase 1a (list_accounts, 0ms) → UI shows 6 accounts
T+4.3s   BG thread: Phase 1b (SEC, portfolio summary) → UI shows portfolio
T+4.7s   BG thread: Phase 1c (detailed stats, fundamentals) → UI shows cache info
T+6.5s   BG thread: Phase 5 account ATPK (1.7s) → P&L appears
T+13s    BG thread: Phase 5 account HAKR (6.5s) → P&L appears
T+22s    BG thread: Phase 5 account WBYE (9s) → P&L appears
T+24s    BG thread: All phases complete → full data available
```

## Command Palette Commands (Data-Related)

| Command | Data Source | Thread | Notes |
|---------|-----------|--------|-------|
| MT5SYNC | MT5 DBs → main cache | Broker (std::thread) | Zero-copy blob transfer |
| DARWIN | DARWIN Accounts window | UI (reads bg.*) | |
| DARWINEX | DARWIN Portfolio window | UI (reads bg.*) | |
| REBALANCE | VaR/correlation analysis | UI (reads bg.rebalance) | |
| SEC | SEC Filing Scanner | UI (reads bg.sec_filings) | |
| EVSCRAPE | Yahoo Finance + SEC EDGAR | Broker (std::thread + tokio) | 300ms rate limit |
| FUNDAMENTALS | Fundamentals viewer | UI (reads bg.all_fundamentals) | |
| EV | EV Scanner table | UI (reads bg.all_fundamentals) | |
| EARNINGS | Earnings calendar | UI (reads bg.upcoming_earnings) | |
| DIVIDENDS | Dividend calendar | UI (reads bg.upcoming_dividends) | |
| DARWIN_BROWSER | FTP universe browser | UI + broker thread | |
| DARWIN_SCAN | CPU FTP scan | Broker (std::thread) | ~10s for 50K DARWINs |
| GPU_SCAN | GPU FTP scan | Broker (std::thread) → GPU | ~50ms compute after I/O |
