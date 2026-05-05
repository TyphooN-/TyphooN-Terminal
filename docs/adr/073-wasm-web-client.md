# ADR-073: WASM Web Client for Phone Access

**Status:** Implemented | **Date:** 2026-04-07

## Context

Users need read-only access to account data, positions, orders, and charts from a phone over LAN WiFi. The native app (egui 0.34 + wgpu) can't run in a browser — the engine depends on SQLite, native-TLS, keyring, and file I/O. A full port is infeasible (9400+ line app.rs, 66 BrokerCmd variants).

## Decision

Thin WASM client architecture: engine stays native on the PC, phone browser renders a minimal egui app that communicates via WebSocket.

### Architecture

```
Phone Browser ──HTTPS──→ axum (port 9848) ──serves──→ WASM bundle
Phone Browser ──WSS────→ axum /ws endpoint ←──relay──→ BrokerCmd/BrokerMsg
```

Three new crates:
- **web-protocol** — Shared `WebCmd`/`WebMsg` serde types (compiles to native + wasm32)
- **web-server** — axum HTTPS + WSS server with TLS + auth + rate limiting
- **web** — eframe 0.34 + glow (WebGL2) WASM app, built via trunk

### Security Model

- **TLS**: Self-signed ephemeral certificate (rcgen), same pattern as LAN sync
- **Authentication**: First WebSocket message must be `Auth { passphrase }` matching LAN sync passphrase. 10-second auth timeout.
- **Rate limiting**: 20 commands/second per client, sliding window
- **Connection limits**: 10 max clients, 3 per IP
- **Message size**: 64 KB max WebSocket message
- **Input validation**: Symbol/timeframe validated against allowlist (alphanumeric + dots + slashes, no path traversal, bounded length)
- **Protocol hardening**: `deny_unknown_fields` on all serde types, invalid type tags rejected

### Phase 1 Scope (Read-Only)

- Account summary, positions with P&L, orders
- Basic line chart via egui_plot
- Login screen with passphrase authentication
- Auto-reconnect, 5-second polling

### Phase 2 Status (updated 2026-05-05)

- **Order entry from phone:** implemented in ADR-089 with broker selection, two-step confirmation, close/cancel actions.
- **Indicators on phone:** implemented in ADR-092 through server-computed `GetIndicators` / `IndicatorData`.
- **DARWIN analytics on phone:** implemented in ADR-093 through `GetDarwinWeb` / `DarwinWebUpdate`.
- **Push-style updates:** implemented in ADR-092 via `BarUpdate`, `PositionUpdate`, and `AccountUpdate` messages.
- **Still deferred:** drawing tools and MTF grid on phone. They remain a larger mobile interaction project, not a protocol gap.

## Consequences

**Positive:**
- Phone access with zero app installation — just browse to `https://<IP>:9848/`
- Same passphrase as LAN sync — no new credentials to manage
- 3.7 MB WASM bundle — acceptable for LAN
- Engine stays native — no SQLite/TLS/keyring porting headaches
- Multiple concurrent phone clients supported via broadcast channel

**Negative:**
- Self-signed TLS requires accepting browser cert warning on first connect
- Phase 1 shipped read-only; later ADRs added limited phone trading and push updates.
- WASM bundle must be rebuilt separately (`trunk build --release`)
- Closure leaks possible in WASM on reconnect (mitigated by clearing callbacks before reconnect)
