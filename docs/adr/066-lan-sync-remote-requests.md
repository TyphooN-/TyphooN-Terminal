# ADR-066: LAN Sync Remote Request Forwarding

**Status:** Implemented | **Date:** 2026-03-29

## Context

LAN clients need data from external sources (SEC EDGAR, Finnhub, CryptoCompare, etc.) but should not make outbound API calls directly. All data should flow through the LAN server, which has the API keys and network access.

## Decision

### Remote Request Protocol
When a LAN client triggers a data-fetching action (e.g. "Scrape Now" button), the request is forwarded to the server via the existing WebSocket connection:

1. Client broker task detects `lan_client_enabled` flag
2. Matches the command against known data-fetching commands
3. Forwards as `RemoteRequest { cmd, args }` via the LAN sync WebSocket channel
4. Server receives, logs the request, executes it, and responds with `RemoteRequestDone`
5. Client triggers a KV re-sync to pull the new data

### Commands Forwarded
- SEC_SCRAPE, FUNDAMENTALS, FUNDAMENTALS_ONE
- KRAKEN_BACKFILL, CRYPTOCOMPARE
- MT5_SYNC, DARWIN_IMPORT
- FINNHUB_NEWS, ECON_CALENDAR, CONGRESS_TRADES, FRED_DATA
- SEC_FILING (content fetch)

### Commands NOT Forwarded (Client-Local)
- Broker connections (Alpaca, tastytrade) — client has its own API keys
- Order placement — goes directly to broker
- LAN sync control commands
- Chart/UI interactions

### Full Data Sync Flow
```
Phase 1: Bar cache (binary batch, all symbols × timeframes)
Phase 2: DARWIN tables (accounts, deals, positions as JSON+zstd)
Phase 3: KV cache (fundamentals, news, SEC, FRED, etc.)
Phase 4: Incremental updates (server pushes new data as it arrives)
Phase 5: Remote requests (client → server → execute → re-sync)
```

### Multi-Client Support
The server accepts multiple concurrent WebSocket connections. Each client gets:
- Full initial sync on connect
- Incremental updates as data changes
- Independent remote request handling

### API Keys
- Server holds all API keys (Finnhub, FRED, SEC EDGAR user-agent)
- LAN clients do NOT need API keys for data access
- Broker API keys (Alpaca, tastytrade) remain per-machine

## Consequences

- **Pro:** LAN client works behind firewall — only needs wss:// to server
- **Pro:** Single point of API key management (server only)
- **Pro:** Data cached on server benefits all clients
- **Pro:** Multiple clients supported simultaneously
- **Con:** Remote request latency (client → server → API → cache → re-sync)
- **Con:** Server must be running for clients to get fresh data
