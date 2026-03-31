# ADR-044: Backup + LAN Sync System

**Status:** Implemented
**Date:** 2026-03-23

> **Note:** Extends [ADR-039](039-portable-backup.md) (Portable Backup) and [ADR-020](020-cache-optimization.md) (SQLite Cache).

## Context

TyphooN-Terminal stores all bar data, DARWIN analytics, and key-value cache in a local SQLite database. Two complementary systems are needed:

1. **Portable Backup**: Export/import the entire cache for machine migration (USB, cloud transfer)
2. **LAN Sync**: Real-time cache synchronization between terminal instances on the same network (desktop ↔ laptop, multiple workstations)

## Decision

### 1. Portable Backup (ADR-039 — Implemented)

See [ADR-039](039-portable-backup.md) for full details. Summary:

- `export_backup`: `VACUUM INTO` → zstd level 9 → `.typhoon-backup` file
- `import_backup`: zstd decompress → `ATTACH DATABASE` → newer-wins merge
- Frontend: `CACHE-BACKUP` / `CACHE-RESTORE` in command palette

### 2. LAN Sync (Implemented)

WebSocket-based real-time cache synchronization between terminal instances on the local network.

#### Architecture

```
Desktop (Server)                    Laptop (Client)
┌────────────────────┐             ┌────────────────────┐
│ TyphooN Terminal   │   WebSocket │ TyphooN Terminal   │
│                    │◄────────────┤                    │
│ LanSyncServer      │────────────►│ LanSyncClient      │
│  :9847 (TCP)       │  Encrypted  │                    │
│                    │  + HMAC Auth │                    │
│ SQLite Cache       │             │ SQLite Cache       │
└────────────────────┘             └────────────────────┘
```

#### Protocol (`core/lan_sync.rs`)

WebSocket messages over TCP, authenticated with PBKDF2-derived shared secret + HMAC-SHA256 challenge-response:

1. **AuthChallenge**: Server sends random challenge bytes
2. **Auth**: Client responds with HMAC-SHA256(challenge, shared_secret)
3. **AuthOk / AuthFail**: Server verifies HMAC
4. **RequestMeta**: Client requests all cache entry metadata (keys + timestamps)
5. **Metadata**: Server sends `CacheMeta` list (key, timestamp, bar_count)
6. **RequestEntries**: Client requests entries where server timestamp > local timestamp
7. **EntryData**: Server sends compressed bar data per entry
8. **BatchComplete**: Server signals end of bulk sync
9. **IncrementalUpdate**: Server pushes new entries as they arrive (after initial sync)
10. **Ping / Pong**: Keepalive

#### Key Derivation

- PBKDF2-HMAC-SHA256 with 100,000 iterations
- Salt: `"typhoon-lan-sync"` (static — acceptable for LAN-only use)
- User provides passphrase; both instances must use the same one

#### Server Mode (`LanSyncServer`)

- Binds to `0.0.0.0:9847` (configurable)
- Accepts multiple concurrent WebSocket clients
- Serves bar cache metadata and data on request
- Pushes incremental updates as new data arrives from MT5 sync or API fetches

#### Client Mode (`LanSyncClient`)

- Connects to server IP:port
- Authenticates via HMAC challenge-response
- Requests metadata, compares timestamps, fetches only newer entries
- Receives incremental pushes for live sync
- Reconnects automatically on connection loss

#### Full Data Sync Protocol (13 tables)

The LAN sync transfers all SQLite tables in phases:

1. **Bar cache** (`bar_cache`): Binary batch, all symbols x timeframes
2. **DARWIN tables** (4): `darwin_accounts`, `darwin_deals`, `darwin_positions`, `darwin_equity_snapshots` — always full sync (deal data is static XLSX import)
3. **KV cache** (`kv_cache`): Fundamentals, news, SEC, FRED, etc.
4. **Research tables** (8 via `SYNCABLE_TABLES` whitelist): `darwin_equity_snapshots`, `sec_filings`, `sec_insider_trades`, `sec_filing_alerts`, `sec_scrape_index`, `fundamentals`, `quarterly_financials`, `institutional_holders`
5. **Sync state** (`sync_state`): Tracks `last_sync_ts` per table for incremental sync

#### Incremental Sync Protocol

All data sync uses timestamp-based incremental transfer via the `sync_state` table:

- Each table's last sync timestamp is stored as `sync_state[table:<name>]`
- Client sends `RequestKvData { since_ts }` and `RequestTableSync { tables: [(name, since_ts), ...] }`
- Server filters rows by `updated_at > since_ts` (or full export when `since_ts == 0`)
- Safety: if incremental returns 0 rows but local table is empty, client auto-triggers full re-sync
- DARWIN data always uses full sync (static deal data, no incremental benefit)

#### Auto Re-Sync After Remote Requests

When a `RemoteRequestDone` message arrives from the server (after SEC scrape, backfill, etc.), the client automatically triggers an incremental re-sync of all research tables, KV cache, and DARWIN data. This ensures freshly fetched data propagates to clients without manual intervention.

#### Frontend Integration

- Command palette: `LAN-SYNC` to configure server/client mode
- Status indicator in dashboard showing sync state (connected, syncing, disconnected)
- Passphrase prompt on first setup

## Security

- All traffic over WebSocket (can be upgraded to WSS with self-signed cert for future)
- HMAC-SHA256 challenge-response prevents unauthorized sync
- PBKDF2 key derivation makes brute-force impractical
- LAN-only — no internet exposure by default (bind to 0.0.0.0, but router NAT provides isolation)

## Consequences

- **Pro**: Real-time cache sync between machines — no manual export/import
- **Pro**: Desktop MT5 sync feeds laptop automatically via LAN
- **Pro**: Authenticated — only machines with the passphrase can sync
- **Pro**: Incremental — only changed entries transferred
- **Pro**: Works alongside portable backup (complementary, not competing)
- **Con**: Requires both machines on same LAN
- **Con**: No TLS by default (acceptable for trusted home/office network)
- **Con**: Static salt in key derivation (acceptable for LAN-only threat model)
