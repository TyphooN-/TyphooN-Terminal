# ADR-015: Price Alerts

**Status:** Implemented | **Date:** 2026-03-24

## Context
Traders need price-level notifications without watching charts continuously.

## Decision
Alert manager panel. Set alerts at any price with custom labels. Triggered when price within 0.1% proximity. Session-persistent (saved to session.json). Managed via Alerts floating window or ~ → ALERTS command.

## Consequences
- Pro: Works offline (from cached bar data)
- Pro: Session persistent across restarts
- Pro: Indicator-alert triggers request OS attention, add a toast/log entry, update the top-bar breach badge, and send Discord/Pushover/ntfy notifications when configured in Settings.
- Trade-off: simple price-level alerts remain cache/chart driven; provider-push latency depends on whichever broker/WebSocket/feed path updates the active chart data.
