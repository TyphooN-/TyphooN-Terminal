# ADR-010: Multi-Broker Architecture

**Status:** Implemented | **Date:** 2026-03-24

## Context
Multiple brokers needed for different asset classes and market data coverage.

## Decision
BrokerCmd/BrokerMsg enum-based async channel architecture. tokio runtime in background thread, mpsc channels bridge UI ↔ broker task.

**Supported:**
- **Alpaca** — US equities + crypto, paper + live. Auto-connects on startup if credentials saved in system keyring. Positions/orders/account stored to KV cache for LAN client read-only view.
- **tastytrade** — Options + futures (ADR-022). Fully integrated: REST API + DXLink WebSocket. Historical bars, quotes, market metrics (IV rank), option chains, positions, orders.
- **MT5** — View-only data source via BarCacheWriter v1.432 → SQLite cache. Trade management stays in MT5.

## Consequences
- Pro: Multi-broker validates BrokerTrait abstraction
- Pro: MT5 data without managing MT5 instances from terminal
- Pro: Alpaca auto-connect eliminates manual connection step
- Pro: LAN clients see server's broker positions read-only (no separate credentials needed)
- Con: Each broker needs its own async client implementation
- Con: tastytrade DXLink requires WebSocket handshake for historical bars (more complex than REST)
