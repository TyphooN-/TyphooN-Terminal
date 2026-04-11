# ADR-038: Data Source Indicator & Pluggable Broker Hierarchy

**Status:** Implemented (Phase 1 + Phase 2)
**Date:** 2026-03-21

> **Note:** Extends [ADR-037](037-data-source-hierarchy.md) (Data Source Hierarchy) and [ADR-010](010-multi-account.md) (Multi-Account).

## Context

Users need to know where their chart data is coming from — MT5 (server-name), Alpaca, or another broker. When MT5 is disconnected (BarCacheWriter stopped, Wine crashed, etc.), the terminal should detect this and transparently fall back to the next data source in the hierarchy. The current system silently serves MT5 data without indicating provenance, and has no disconnect detection.

Long-term, TyphooN-Terminal aims to support **any number of data sources and brokers** in a user-configurable priority queue. A trader may have:
- MT5 (Darwinex) for CFDs/Forex with deep history
- Alpaca for US equities with live execution
- Interactive Brokers for options
- A crypto exchange for digital assets

Each source has strengths (depth of history, real-time freshness, asset coverage) and the user should control which source is authoritative per symbol or asset class.

## Decision

### Phase 1: Data Source Badge (This PR)

**Backend (`get_bars_incremental`):**
- Return a JSON wrapper: `{"bars": [...], "source": "mt5", "source_label": "MT5 (server-name)"}` instead of raw bar array
- Source is determined by which code path served the data (MT5 cache hit vs Alpaca API)
- Backward-compatible: if frontend receives a raw array, treat as legacy

**Frontend:**
- Show a subtle data source badge in the status bar: `CC — 2704 bars · MT5 (server-name)`
- Color-coded: green for MT5 (real-time), amber for Alpaca (delayed), red for disconnected
- Track `lastMt5SyncSuccess` timestamp from background sync
- If MT5 sync hasn't succeeded in 15 minutes → show "MT5 Disconnected" warning, mark Alpaca as active source

**MT5 Disconnect Detection:**
- `startMt5BackgroundSync()` already runs every 30 seconds
- Track last successful sync time (`databases_read > 0`)
- After 15 minutes of no successful reads → set `mt5Disconnected = true`
- When disconnected: invalidate MT5 cache keys, force Alpaca fallback
- When reconnected: re-sync and restore MT5 as primary

### Phase 2: Pluggable Data Source Queue (Implemented)

**Architecture:**
```
DataSourceManager {
  sources: [
    { id: "mt5-darwinex", type: "mt5", label: "MT5 (server-name)", priority: 1 },
    { id: "alpaca-paper", type: "alpaca", label: "Alpaca (Paper)", priority: 2 },
    { id: "ibkr-live",    type: "ibkr",   label: "IBKR (Live)", priority: 3 },
  ]

  // Per-symbol overrides
  symbolOverrides: {
    "BTC/USD": ["binance-spot", "mt5-darwinex"],  // crypto exchange first
    "SPX":     ["ibkr-live", "alpaca-paper"],       // IBKR for options
  }
}
```

**User Configuration:**
- Settings panel for ordering data sources (drag-and-drop priority)
- Per-symbol override rules (regex or asset-class based)
- Health monitoring dashboard showing each source's status
- Manual failover button per source

**Backend Contract:**
- Each data source implements a `DataSource` trait:
  ```rust
  trait DataSource: Send + Sync {
      fn id(&self) -> &str;
      fn label(&self) -> &str;
      fn get_bars(&self, symbol: &str, tf: &str, limit: u32) -> Result<Vec<Bar>>;
      fn is_healthy(&self) -> bool;
      fn last_sync(&self) -> Option<Instant>;
      fn supports_symbol(&self, symbol: &str) -> bool;
  }
  ```
- `DataSourceManager` iterates through priority-ordered sources until one returns data
- Health checks run on a background timer; unhealthy sources are temporarily skipped

**Implementation** (`engine/src/core/data_source.rs`):
- `DataSourceEntry` struct: id, cache_prefix, label, priority, healthy, last_success_ts, asset_classes
- `SymbolOverride` struct: pattern (supports `*` wildcard), ordered source IDs
- `DataSourceManager`: resolve_candidates(), mark_success/failure(), update_health(), add_override()
- Default 5 sources: MT5 (prio 1), Alpaca (2), tastytrade (3), CryptoCompare (4), Kraken (5)
- `SOURCES` console command shows health dashboard with result card
- `find_cache_key()` uses `DataSourceManager::resolve_candidates()` for priority-ordered lookup
- 11 unit tests (roundtrip, health, overrides, deny_unknown_fields)
- Vec<DataSourceEntry> (not HashMap) — 5 entries fits L1 cache, no hash overhead

## Consequences

### Phase 1
- **Pro**: User always knows where their data comes from
- **Pro**: MT5 disconnection is detected and communicated clearly
- **Pro**: Graceful degradation — Alpaca fills in when MT5 is down
- **Con**: Slight API change (wrapped response) — handled with backward compat

### Phase 2
- **Pro**: Fully extensible — any broker/exchange can be added as a plugin
- **Pro**: Per-symbol routing enables best-of-breed data per asset class
- **Pro**: Multi-account trading from a single terminal
- **Con**: Significant implementation effort
- **Con**: Complexity of managing conflicting data from multiple sources
