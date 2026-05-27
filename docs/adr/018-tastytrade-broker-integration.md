# ADR-018: Tastytrade Broker Integration

**Status:** Fully Implemented
**Date:** 2026-03-16

## Context

Alpaca's free tier has limited historical data (IEX feed, ~260 bars per chunk). Tastytrade offers free paper trading with broader market data access, plus superior options and futures support. Adding a second broker validates the `BrokerTrait` abstraction.

## Decision

Implement `TastytradeBroker` in `src/broker/tastytrade.rs` with session-based auth via the Tastytrade REST API.

## API Details

| Endpoint | Method | Path |
|---|---|---|
| Login | POST | `/sessions` |
| Accounts | GET | `/customers/me/accounts` |
| Balances | GET | `/accounts/{id}/balances` |
| Positions | GET | `/accounts/{id}/positions` |
| Place Order | POST | `/accounts/{id}/orders` |

**Base URLs:**
- Production: `https://api.tastyworks.com`
- Sandbox: `https://api.cert.tastyworks.com` (note: previously `api.cert.tastytrade.com`, changed upstream)

**Auth:** Username + password → session token (stored in `Zeroizing<String>`)

## Security

- Password passed via HTTPS POST body, never logged
- Session token stored in `Arc<Mutex<Zeroizing<String>>>` — zeroed on drop
- Input validation: username ≤100 chars, password ≤200 chars
- All HTTP responses: body consumed with `let _` on error (no leak)
- Client has 30s timeout via `expect()` (no silent fallback)

## Native UI Integration

- Broker selector in Connect to Broker window (Alpaca / Tastytrade)
- Tastytrade uses username/password instead of API key/secret
- Engine tracks `active_broker` for routing

## Current State (2026-04-02)

Fully implemented. Connect button active. All REST + DXLink WebSocket endpoints wired:

**REST API:**
- Auth (login/session), accounts, balances (NLV/BP/cash)
- Positions, orders (list/place equity), option chains (nested)
- Quote snapshots (`/market-data`), market metrics (`/market-metrics` — IV rank/percentile/beta)
- Persistent `tt_broker` stored for later use

**DXLink WebSocket (engine/src/broker/dxlink.rs):**
- `get_streaming_token()` — REST call for WebSocket auth token + URL
- `fetch_candles()` — full protocol: SETUP→AUTH→CHANNEL_REQUEST→FEED_SETUP→FEED_SUBSCRIPTION
- Historical bars for all intervals (1m, 5m, 15m, 30m, 1h, 4h, 1d, 1w, 1mo)
- `BrokerCmd::TastytradeFetchBars` wired to broker loop, stores as `tastytrade:SYM:TF`
- `try_load()` includes `tastytrade:` in the 6-source priority lookup (after Alpaca, before CryptoCompare; Kraken Futures remains last)

**Note:** tastytrade has no REST endpoint for historical OHLCV bars — DXLink WebSocket is the only way.
Alpaca free tier provides 15-min delayed bars; tastytrade DXLink provides real-time bars for funded accounts.

## Consequences

- **Pro**: Second broker validates multi-broker architecture
- **Pro**: Better options/futures support than Alpaca (IV rank, option chains)
- **Pro**: DXLink provides real-time historical bars (no 15-min delay)
- **Pro**: Market metrics (IV rank/percentile) unique to tastytrade
- **Con**: Different auth model (session vs API key) adds complexity
- **Con**: DXLink requires WebSocket handshake for bars (not simple REST)
