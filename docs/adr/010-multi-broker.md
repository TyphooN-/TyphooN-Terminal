# ADR-010: Multi-Broker Architecture

**Status:** Implemented | **Date:** 2026-03-24

## Context
Multiple brokers needed for different asset classes and market data coverage.

## Decision
BrokerCmd/BrokerMsg enum-based async channel architecture. tokio runtime in background thread, mpsc channels bridge UI ↔ broker task.

**Supported:**
- **Alpaca** — US equities + crypto, paper + live. Wired via AlpacaBroker + async mpsc.
- **tastytrade** — Options + futures (ADR-022). Session-based auth. Broker module pending.
- **MT5** — View-only data source via BarCacheWriter EA → SQLite cache. Trade management stays in MT5.

## Consequences
- Pro: Multi-broker validates BrokerTrait abstraction
- Pro: MT5 data without managing MT5 instances from terminal
- Con: Each broker needs its own async client implementation
