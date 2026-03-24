# ADR-015: Order Management

**Status:** Implemented | **Date:** 2026-03-24

## Context
Full order lifecycle needed: entry, modification, close.

## Decision
Order Entry panel with Market/Limit/Stop/Bracket types. Side (Buy/Sell), quantity, limit/stop/TP prices. Risk preview shows notional value + ATR(14). Async execution via BrokerCmd::Connect → broker task. Live positions and orders displayed in right panel with P&L. Close All sends BrokerCmd::CloseAll.

## Consequences
- Pro: Full order types in native Rust implementation
- Pro: Async non-blocking execution
- Pro: Risk preview before submission
