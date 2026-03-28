# ADR-065: LAN Sync TLS Encryption

**Status:** Implemented | **Date:** 2026-03-28

## Context

LAN sync previously used unencrypted WebSocket (ws://). While the HMAC challenge-response authentication prevented unauthorized access, all data (bar cache, DARWIN analytics) was transmitted in plaintext over the local network. This was flagged as a security concern.

## Decision

Upgrade LAN sync transport from `ws://` to `wss://` (WebSocket over TLS).

### Implementation

**Server:**
1. On startup, generates an ephemeral self-signed TLS certificate using `rcgen` (SAN: `typhoon-lan-sync`, `localhost`)
2. Wraps TCP listener with `native-tls::TlsAcceptor` → `tokio-native-tls::TlsAcceptor`
3. TLS handshake occurs before WebSocket upgrade
4. Certificate is ephemeral — regenerated on each server restart (no persistent key material)

**Client:**
1. Connects to `wss://host:port` instead of `ws://host:port`
2. Uses `native-tls::TlsConnector` with `danger_accept_invalid_certs(true)` for self-signed LAN certs
3. `tokio-tungstenite::connect_async_tls_with_config` handles the TLS upgrade

### Security Layers (Defense in Depth)
```
Layer 1: TLS encryption (all data encrypted in transit)
Layer 2: HMAC-SHA256 challenge-response authentication (PBKDF2 100K iterations)
Layer 3: Application-level data validation (typed serde deserialization)
```

### Dependencies Added
- `rcgen = "0.13"` — self-signed certificate generation
- `native-tls = "0.2"` — TLS acceptor/connector
- `tokio-native-tls = "0.3"` — async TLS wrapper

## Consequences

- **Pro:** All LAN sync traffic encrypted — no plaintext bar data or DARWIN analytics
- **Pro:** Ephemeral certificates — no key management burden
- **Pro:** Backward compatible — authentication protocol unchanged
- **Con:** Client accepts any self-signed cert (`danger_accept_invalid_certs`) — suitable for LAN, not internet
- **Con:** Small latency overhead from TLS handshake (~5ms per connection)
