# ADR-009: Multi-Broker Architecture

> **Note (2026-06):** broker scope was reduced to **Kraken + Alpaca** — see [ADR-111](111-broker-scope-reduction-kraken-alpaca-only.md). The pluggable broker abstraction described here remains in force; the Tastytrade / MT5 / Darwin integrations were removed (code on `deprecated/*`).
>
> **Note (2026-06):** which enabled broker is **primary** (order-routing default + trusted equity-merge lane) vs **assist** is now a user-selectable, persisted choice — see [ADR-126](126-primary-assist-broker-selection.md). `OrderBroker` is the broker-identity enum that ADR-126 extends for N brokers.
>
> **Note (2026-07):** `BrokerTrait` (order/account/bars) has a market-data companion — the typed L1/L2/L3 **capability model** in `typhoon_engine::broker::capabilities` (`MarketDataSupport`, `DepthAssetScope`, `OrderBroker::market_data_capabilities`). UI surfaces read normalized capabilities instead of matching on broker identity, and adding a broker is a compile error until its L1/L2/L3 support is declared. See [ADR-129](129-l1-l2-l3-market-data-support.md) → "Broker Capability Model (code)". This keeps L1/L2/L3 robust regardless of the selected primary broker and is where restored tastytrade / future Binance modules plug in.

**Status:** Implemented | **Date:** 2026-03-24

## Context
Multiple brokers needed for different asset classes and market data coverage.

## Decision
BrokerCmd/BrokerMsg enum-based async channel architecture. tokio runtime in background thread, mpsc channels bridge UI ↔ broker task.

**Supported:**
- **Alpaca** — US equities + crypto, paper + live. Auto-connects on startup if credentials saved in system keyring. Positions/orders/account stored to KV cache for LAN client read-only view.
- **MT5** — View-only data source via BarCacheWriter v1.435 → SQLite cache (TF gating, 16MB cache, /dev/shm ramdisk). Trade management stays in MT5.

## Consequences
- Pro: Multi-broker validates BrokerTrait abstraction
- Pro: MT5 data without managing MT5 instances from terminal
- Pro: Alpaca auto-connect eliminates manual connection step
- Pro: LAN clients see server's broker positions read-only (no separate credentials needed)
- Con: Each broker needs its own async client implementation
- Con: tastytrade DXLink requires WebSocket handshake for historical bars (more complex than REST)
