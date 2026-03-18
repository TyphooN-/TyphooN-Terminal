# ADR-008: Multi-Tab Chart Support

**Status:** Implemented
**Date:** 2026-03-15
**Context:** Traders monitor multiple symbols simultaneously. MT5 achieves this via separate chart windows. TyphooN-Terminal uses a single window with tabs.

## Decision

Implement browser-style tabs within the Tauri window. Each tab stores: symbol, timeframe, bar count, last price.

## Behavior

- **Ctrl+T** or **+** button: new empty tab
- **Click tab**: switch (saves current state, restores target state, clears chart, loads data)
- **Ctrl+W** or **×**: close tab (minimum 1 always open)
- Tab label shows symbol name, updates on chart load

## State Management

On tab switch:
1. Save current tab: symbol, timeframe, barCount, lastPrice
2. Stop live bar polling from previous tab
3. Clear chart (candles + indicators + sub-panes)
4. Restore target tab's UI state (symbol input, timeframe, bar count)
5. Load chart from cache (instant) or API

## Async Load Guard

If user switches tabs while bars are loading, the late-arriving data is discarded:
```javascript
const loadTabId = activeTabId;
// ... async fetch ...
if (activeTabId !== loadTabId) { return; } // tab changed during load
```

## Global Loading Queue

Loading indicator shows ALL symbols loading across all tabs:
```
LUMN (2021-04-05 → 2026-03-14 · 1000 bars) | SLV (loading...) | SMCI (loading...)
```

## Consequences

- **Pro**: Monitor multiple symbols without multiple windows
- **Pro**: Cache shared across tabs — switching back to a cached symbol is instant
- **Pro**: Rate limiter shared — multiple tabs don't double-spend API budget
- **Con**: Single chart instance reused — tab switch requires full chart rebuild
- **Con**: No side-by-side comparison (use MTF Grid for multi-chart layout)
