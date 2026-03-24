# ADR-016: Price Alerts

**Status:** Implemented | **Date:** 2026-03-24

## Context
Traders need price-level notifications without watching charts continuously.

## Decision
Alert manager panel. Set alerts at any price with custom labels. Triggered when price within 0.1% proximity. Session-persistent (saved to session.json). Managed via Alerts floating window or ~ → ALERTS command.

## Consequences
- Pro: Works offline (from cached bar data)
- Pro: Session persistent across restarts
- Con: No push notifications yet (requires broker WebSocket for real-time)
