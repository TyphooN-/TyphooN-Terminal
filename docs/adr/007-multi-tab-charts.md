# ADR-007: Multi-Tab Charts

**Status:** Implemented
**Date:** 2026-03-24

## Context

Traders monitor multiple symbols and timeframes simultaneously. A single-chart view forces constant switching, which is slow and loses context. The terminal needs a tab system comparable to browser tabs but with per-tab trading state.

## Decision

Implement a tab bar at the top of the chart area using egui's built-in tab widget. Each tab holds independent state: symbol, timeframe, chart type, zoom level, scroll position, active indicators, and drawing objects. Keyboard shortcuts: Ctrl+N opens a new tab, Ctrl+W closes the current tab, Ctrl+Tab / Ctrl+Shift+Tab cycles tabs. Tab order is preserved across sessions via the JSON session file. Closing the last tab opens a default tab rather than leaving the chart area empty.

## Consequences

- Each tab is an independent chart viewport; changing symbol in one tab does not affect others
- Per-tab indicator and drawing state means a trader can have different analysis setups per symbol
- Ctrl+N/W/Tab shortcuts match universal tab conventions; no learning curve
- Session persistence restores the exact tab layout on restart, including scroll position and zoom
- Trade-off: each open tab holds its own bar data and indicator buffers in memory; 20+ tabs on low-memory systems may require lazy loading
- Trade-off: tab bar consumes vertical space; a future enhancement could support detachable/floating tabs
