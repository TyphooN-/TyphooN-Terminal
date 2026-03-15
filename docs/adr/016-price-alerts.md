# ADR-016: Price Alerts

**Status:** Implemented
**Date:** 2026-03-15

## Context

Traders need to be notified when price crosses a level of interest without staring at the chart. MT5 has built-in alerts; TyphooN-Terminal had none.

## Decision

Client-side price alerts checked every 2 seconds during the dashboard update cycle. Persistent across sessions via localStorage.

## Implementation

- **Storage**: `typhoon_alerts` in localStorage — array of `{ symbol, price, direction, triggered }`
- **Check cycle**: `checkAlerts()` called from `updateDashboard()` (runs every 2s when connected)
- **Trigger**: When `lastPrice` crosses alert threshold, fire notification + log
- **Notification**: Browser `Notification` API (works in Tauri webview)
- **Keyboard**: `a` key sets alert at current price (prompts for above/below)
- **Scope**: Current chart symbol only (multi-symbol alerts require price polling — deferred)

## Consequences

- **Pro**: Zero backend changes — purely frontend
- **Pro**: Persistent across app restarts
- **Pro**: 2-second latency (acceptable for manual trading)
- **Con**: Only alerts on currently displayed symbol
- **Con**: No sound (Notification API handles system sound)
