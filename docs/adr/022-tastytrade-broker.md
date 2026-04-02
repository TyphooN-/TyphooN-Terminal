# ADR-022: Tastytrade Broker Integration

**Status:** Phase 1 — Auth Only
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

## Current State (2026-04-01)

Phase 1 (auth) is complete: session login, account listing, balances, positions endpoints are wired. Market data via DXLink WebSocket is on the roadmap but not yet implemented — tastytrade is currently auth-only with no live quotes or streaming.

**Connect button is disabled in the UI** (greyed out with "coming soon" label) until DXLink market data and order execution are fully implemented. Credentials fields remain visible for future use. Alpaca auto-connects on startup if credentials are saved in the system keyring.

## Consequences

- **Pro**: Second broker validates multi-broker architecture
- **Pro**: Better options/futures support than Alpaca
- **Pro**: Free paper trading for testing
- **Con**: Different auth model (session vs API key) adds complexity
- **Con**: Market data via DXLink WebSocket — large scope, requires separate library
- **Con**: Connect button disabled until full implementation — users cannot accidentally connect to a partial integration
