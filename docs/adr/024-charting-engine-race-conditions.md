# ADR-024: Charting Engine Race Conditions — Cross-Symbol Data Contamination

**Status:** Fixed
**Date:** 2026-03-17

## Context

When switching between tabs (symbols) with multiple charts open, indicator overlays (supply/demand zones, KAMA projections, ATR bands) from one symbol could appear on another symbol's chart. The contamination manifested as price lines at impossible levels (e.g., $868 lines on an SMCI chart trading at $31). The MTF grid view amplified the issue by running concurrent bar fetches for multiple symbols through the same rate limiter.

## Root Causes Found

7 bugs identified through systematic audit. All share the pattern: **async operation completes after a tab switch and writes stale data to global state**.

### HIGH Severity

| # | Function | Issue |
|---|----------|-------|
| 1 | `loadMTFData()` | Overwrites global `mtfData` without checking if symbol changed during async fetch. AAPL's MTF data could overwrite MSFT's if AAPL's fetch completed later. |
| 2 | `updateLatestBar()` | Only checked `symbol !== currentSymbol` at function entry, not after the `await invoke()`. Tab switch during the fetch could write wrong symbol's price to `lastPrice` and trigger `applyIndicators()` with stale `mtfData`. |
| 3 | `applyIndicators()` | Reads `mtfData[tf]` for HTF KAMA/ATR projections without validating that `mtfData` matches the current symbol. Called from `updateLatestBar()` which could fire with stale MTF data. |

### MEDIUM Severity

| # | Function | Issue |
|---|----------|-------|
| 4 | `loadChart()` background refresh | Checked `currentSymbol === symbol && currentTimeframe === timeframe` but not `activeTabId`. Two symbols with the same timeframe could pass the guard during rapid tab switches. |
| 5 | `liveBarInterval` closure | Captured `symbol`/`timeframe` in closure. Rapid `loadChart()` calls could create overlapping intervals if the first hadn't reached the `clearInterval` line yet. |

### LOW Severity

| # | Function | Issue |
|---|----------|-------|
| 6 | `syncMTFGridLivePrice()` | Already fixed (prior pass) — guard `mtfGridSymbol !== currentSymbol` prevents cross-symbol live price sync. |
| 7 | MTF grid reload | `closeMTFGrid()` + `requestAnimationFrame(() => openMTFGrid())` has a small window where both grids could exist. Low impact — DOM cleanup is synchronous. |

## Fixes Applied

### 1. Symbol guards after every `await`

Every async function that mutates global chart state now re-checks `currentSymbol` after the `await` returns:

```javascript
// loadMTFData: guard before writing to mtfData
if (currentSymbol !== symbol) { log("...discarded..."); return; }
mtfData = {};

// updateLatestBar: guard after fetch completes
if (symbol !== currentSymbol) return;

// updateLatestBar: guard before mutating currentChartData
if (symbol !== currentSymbol) return;
applyIndicators(currentChartData);

// loadMTFData .then() callback
if (currentSymbol !== symbol) return;
applyIndicators(chartData);
```

### 2. Tab ID guard on background refresh

```javascript
// Before: loose check
if (currentSymbol === symbol && currentTimeframe === timeframe)

// After: includes tab ID
if (currentSymbol === symbol && currentTimeframe === timeframe && activeTabId === loadTabId)
```

### 3. Generation counter for live interval

```javascript
const gen = ++chartLoadGeneration;
liveBarInterval = setInterval(() => {
  if (chartLoadGeneration !== gen) { clearInterval(liveBarInterval); return; }
  updateLatestBar(symbol, timeframe);
}, 10000);
```

### 4. Clear stale MTF data on tab switch

```javascript
// In switchTab()
mtfData = {}; // Clear stale MTF data from previous symbol
```

## Verified Safe (Not Bugs)

| Component | Why safe |
|-----------|----------|
| `barCache` keys | `${symbol}:${tf}` — unique per symbol, no collision |
| `indicatorSeries` | Cleared and rebuilt on every `applyIndicators()` call |
| `fisherSeries`/`volumeSeries` | Cleared on every tab switch |
| `orderPriceLines` | Cleared and rebuilt each cycle, filters by `currentSymbol` |
| `calcSupplyDemandZones()` | Takes `chartData` parameter, never reads global state |
| `prefetchAllTimeframes()` | Only writes to `barCache`, never touches chart state |

## Testing

- Switch tabs rapidly between 5+ symbols with MTF grid active
- Verify no supply/demand zones appear at wrong price levels
- Verify live price updates don't create false wicks across symbols
- Verify indicator overlays (KAMA, ATR) use correct symbol's HTF data
