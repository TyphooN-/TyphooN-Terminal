# ADR-072: Kraken as Full Broker (Data + Trading)

**Status:** Proposed | **Date:** 2026-04-05

## Context

Kraken is a US-friendly cryptocurrency exchange with comprehensive REST + WebSocket APIs. Currently TyphooN-Terminal uses Kraken as a **data-only** source (OHLCV bars via public OHLC endpoint). Full broker integration would enable trading directly from the terminal.

## Current Integration (Data Only)

- `engine/src/core/kraken.rs` — OHLCV bar fetching via public `GET /0/public/OHLC`
- 40+ crypto pairs mapped (BTC, ETH, SOL, XMR, ZEC, DASH, etc.)
- Monthly aggregation from daily bars
- No authentication required for data

## Proposed: Full Broker Integration

### Phase 1: Authentication + Account
- API key + secret storage in system keyring
- `GET /0/private/Balance` — account balances
- `GET /0/private/OpenPositions` — open positions
- `GET /0/private/OpenOrders` — pending orders
- HMAC-SHA512 request signing (Kraken's auth scheme)

### Phase 2: Order Placement *(Implemented)*
- [x] `POST /0/private/AddOrder` — market orders via `KrakenPlaceOrder` BrokerCmd
- [x] `POST /0/private/CancelOrder` — cancel by txid via `KrakenCancelOrder` BrokerCmd
- `POST /0/private/CancelAll` — not wired (use individual cancel)
- Support for leverage (Kraken margin trading) — not wired

### Phase 3: WebSocket Streaming
- `wss://ws.kraken.com` — real-time trades, orderbook, OHLC
- `wss://ws-auth.kraken.com` — authenticated own-trades, open-orders
- Feed into BarBuilder for real-time bar construction

### Implementation

New file: `engine/src/broker/kraken_broker.rs` (separate from existing `core/kraken.rs` data module)

```rust
pub struct KrakenBroker {
    client: reqwest::Client,
    api_key: String,
    api_secret: String, // base64-encoded
    rate_limiter: RateLimiter,
}
```

### Advantages
- **US-friendly** — Kraken operates legally in most US states
- **No 15-minute delay** — real-time data (vs Alpaca free tier)
- **Deep crypto liquidity** — BTC, ETH, SOL, XMR, ZEC, 200+ pairs
- **Margin trading** — up to 5x leverage on select pairs
- **Staking** — earn yield on holdings
- **Fiat on/off ramp** — USD deposits/withdrawals

### Data Coverage
- 200+ trading pairs against USD, EUR, BTC, ETH
- OHLCV data back to exchange inception (~2013 for BTC)
- Real-time orderbook depth
- Trade history

## Consequences

- **Pro:** Full trading capability for US crypto traders
- **Pro:** Real-time data without Alpaca's 15-min delay
- **Pro:** Deep history for 200+ crypto assets
- **Pro:** Complements Alpaca (stocks) + tastytrade (options) + MT5 (CFDs)
- **Con:** HMAC-SHA512 signing adds complexity vs Alpaca's simple header auth
- **Con:** Kraken rate limits are stricter than Alpaca (15 requests/second)
- **Con:** Kraken doesn't support US equities — crypto only
