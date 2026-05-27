# ADR-037: Symbol Specs Tracking & Radar Export

**Status:** Implemented (via BarCacheWriter v1.420 __SPECS__ export)
**Date:** 2026-03-26

## Context

The Darwinex radar system tracks symbol specification changes over time — TradeMode transitions, swap rate adjustments, spread changes, delistings, and close-only status. Currently the BarCacheWriter only exports OHLCV bar data but not the symbol metadata needed for this tracking.

The radar currently processes CSV exports from MT5 (`SymbolsExport-Darwinex-Live-*.csv`) with columns: Symbol, TradeMode, SwapLong, SwapShort, Spread. This runs externally via Python scripts.

## Decision

Extend the BarCacheWriter EA and the `typhoon_cache.db` schema to capture symbol specifications alongside bar data, enabling built-in radar functionality without external scripts.

### Data to Capture Per Symbol (Daily Snapshot)

| Field | Source | Purpose |
|-------|--------|---------|
| `symbol` | MT5 `Symbol()` | Identifier |
| `trade_mode` | `SymbolInfoInteger(SYMBOL_TRADE_MODE)` | Detect close-only/delisted transitions |
| `swap_long` | `SymbolInfoDouble(SYMBOL_SWAP_LONG)` | Track financing cost changes |
| `swap_short` | `SymbolInfoDouble(SYMBOL_SWAP_SHORT)` | Track financing cost changes |
| `spread` | `SymbolInfoInteger(SYMBOL_SPREAD)` | Liquidity/cost monitoring |
| `tick_size` | `SymbolInfoDouble(SYMBOL_TRADE_TICK_SIZE)` | Spec change detection |
| `tick_value` | `SymbolInfoDouble(SYMBOL_TRADE_TICK_VALUE)` | Position sizing |
| `contract_size` | `SymbolInfoDouble(SYMBOL_TRADE_CONTRACT_SIZE)` | Notional calculation |
| `margin_initial` | `SymbolInfoDouble(SYMBOL_MARGIN_INITIAL)` | Margin requirement changes |
| `volume_min` | `SymbolInfoDouble(SYMBOL_VOLUME_MIN)` | Lot size constraints |
| `volume_max` | `SymbolInfoDouble(SYMBOL_VOLUME_MAX)` | Lot size constraints |
| `volume_step` | `SymbolInfoDouble(SYMBOL_VOLUME_STEP)` | Lot size precision |
| `snapshot_date` | Current date | Time dimension |

### SQLite Schema

```sql
CREATE TABLE IF NOT EXISTS symbol_specs (
    symbol TEXT NOT NULL,
    snapshot_date TEXT NOT NULL,  -- YYYY-MM-DD
    trade_mode INTEGER,          -- 0=disabled, 1=long-only, 2=close-only, 4=full
    swap_long REAL,
    swap_short REAL,
    spread INTEGER,
    tick_size REAL,
    tick_value REAL,
    contract_size REAL,
    margin_initial REAL,
    volume_min REAL,
    volume_max REAL,
    volume_step REAL,
    PRIMARY KEY (symbol, snapshot_date)
);

CREATE INDEX idx_specs_symbol ON symbol_specs(symbol);
CREATE INDEX idx_specs_date ON symbol_specs(snapshot_date);
```

### Cache Key Pattern

`mt5:{broker}:specs:{date}` → ZSTD-compressed JSON array of symbol specs

### Built-in Radar Features

Once specs are in the cache, the terminal can:

1. **Detect delistings**: `trade_mode` transitions (4 → 2 → 0)
2. **Track swap changes**: Alert on significant swap rate adjustments
3. **Monitor spread widening**: Flag unusual spread increases
4. **Margin changes**: Alert on margin requirement changes
5. **Generate radar TXT**: Replace external Python scripts entirely
6. **Historical analysis**: Backtest with accurate swap/spread costs

### BarCacheWriter EA Changes

```mql5
// After writing bar data, snapshot symbol specs once daily
if (TimeToString(TimeCurrent(), TIME_DATE) != lastSpecsDate) {
    SnapshotSymbolSpecs();
    lastSpecsDate = TimeToString(TimeCurrent(), TIME_DATE);
}
```

### MT5SYNC Integration

When syncing MT5 databases to the main cache, also sync the `symbol_specs` table:
```
MT5 → BarCacheWriter → typhoon_mt5_cache.db (specs table)
  → MT5SYNC → typhoon_cache.db (specs table merged)
```

## Consequences

### Positive
- Self-contained radar — no external Python scripts needed
- Historical spec data enables accurate backtest cost modeling
- Delisting/close-only alerts built into the terminal
- Single source of truth for symbol metadata

### Negative
- Additional DB storage (~1KB per symbol per day × 900 symbols = ~900KB/day, ~330MB/year)
- BarCacheWriter EA needs update and redeployment to all MT5 instances
- Spec snapshot adds ~100ms to daily BarCacheWriter cycle

See also: ADR-054 (SwapHarvester — scans specs for positive swap), ADR-055 (DarwinexRadar — changelog from spec snapshots)
