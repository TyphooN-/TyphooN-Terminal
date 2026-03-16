# ADR-022: Tastytrade Broker Integration

**Status:** Implemented
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
- Sandbox: `https://api.cert.tastyworks.com`

**Auth:** Username + password → session token (stored in `Zeroizing<String>`)

## Security

- Password passed via HTTPS POST body, never logged
- Session token stored in `Arc<Mutex<Zeroizing<String>>>` — zeroed on drop
- Input validation: username ≤100 chars, password ≤200 chars
- All HTTP responses: body consumed with `let _` on error (no leak)
- Client has 30s timeout via `expect()` (no silent fallback)

## Frontend

- Broker selector dropdown in connect modal (Alpaca / Tastytrade)
- Tastytrade uses username/password instead of API key/secret
- AppState tracks `active_broker` field for routing

## Consequences

- **Pro**: Second broker validates multi-broker architecture
- **Pro**: Better options/futures support than Alpaca
- **Pro**: Free paper trading for testing
- **Con**: Different auth model (session vs API key) adds complexity
- **Con**: Market data via DXLink WebSocket (not implemented yet — future work)
