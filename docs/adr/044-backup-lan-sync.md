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

WebSocket messages over TLS-encrypted TCP (wss://), authenticated with PBKDF2-derived shared secret + HMAC-SHA256 challenge-response. See [ADR-065](065-lan-sync-tls-encryption.md) for TLS implementation details (ephemeral self-signed certs, no pinning).

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

- Binds to `0.0.0.0:9847` (configurable), TLS encrypted (wss://)
- Accepts multiple concurrent WebSocket clients
- Connected client IPs tracked and displayed in UI (stored in KV `lan:server:clients`)
- Serves bar cache metadata and data on request
- Auto-starts on startup if `lan_server_enabled` is saved in session
- Stores broker positions/account/orders to KV cache for LAN client read-only view

#### Client Mode (`LanSyncClient`)

- Connects to server IP:port (wss://, TLS encrypted)
- Authenticates via HMAC challenge-response (PBKDF2 100K iterations)
- Requests metadata, compares timestamps, fetches only newer entries
- **15-minute periodic re-sync**: pulls updated bars, KV, DARWIN, research tables automatically (force-reconnect triggers incremental sync; picks up weekend crypto backfill bars)
- Auto-connects on startup if `lan_client_enabled` is saved in session
- Read-only view of server's Alpaca positions/orders/account (from KV cache `broker:*`)
- **23 DARWIN analytics fields** from server KV — zero local deal queries. Positions, portfolio, exposure, correlations, VaR, Monte Carlo, optimal allocation, rebalance, stress tests, drawdown, signal decay, etc. All identical to server.
- BG thread checks `lan_client_flag` — never calls `get_portfolio_open_positions()` or any deal-dependent computation locally.
- Resync buttons: Resync Bars, Resync DARWIN Analytics, Resync Positions
- SEC filing content fetched directly (public EDGAR URLs, not forwarded to server)

#### Headless CLI Mode

The CLI links the same `typhoon_engine::core::lan_sync::{LanSyncServer, LanSyncClient}` types as the GUI. `typhoon-cli --lan-server` starts the same WSS listener and `typhoon-cli --lan-client <host>` runs the same sync loop. Cache resolution is shared with the GUI cache-location contract: explicit `--cache-dir` / `TYPHOON_CACHE_DIR`, then `~/.config/typhoon-terminal/cache_location.txt`, then `~/.config/typhoon-terminal/cache`. Passphrase resolution also matches GUI mode: OS keyring key `lan_sync_passphrase`, then cache KV key `cred:lan_sync_passphrase`. If no saved passphrase exists, `--lan-passphrase` / `TYPHOON_LAN_PASSPHRASE` bootstraps and persists it to the same keyring/KV locations.

This keeps LAN server/client compatibility at the protocol and database level. Docker, Kubernetes, and Terraform deployments mount a user-provided local or NAS path to `/cache` and run the CLI with `--cache-dir /cache`.

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

#### KV Exclusion List

Machine-local configuration keys are **never synced** to other nodes:
- `lan:server_enabled`, `lan:client_enabled`, `lan:server_ip`, `lan:sync_port`
- `cred:*` (credentials — never leave the machine)

Previously, syncing `lan:server_enabled = "true"` from server to clients caused clients to auto-start a LAN server on next launch. The server-side `RequestKvData` handler now filters these keys before transmission. Client-side startup also reads `lan:client_enabled` first and sanitizes any stale `lan:server_enabled` in local DB.

## Security

- All traffic over TLS-encrypted WebSocket (wss://) with ephemeral self-signed certificates
- PBKDF2-HMAC-SHA256 challenge-response (100K iterations) prevents unauthorized sync
- No certificate pinning (ephemeral certs regenerated on every server restart; see ADR-065)
- 300-second read timeout during initial sync (DARWIN export of 45K+ deals takes >60s)
- PBKDF2 key derivation makes brute-force impractical
- LAN-only — no internet exposure by default (bind to 0.0.0.0, but router NAT provides isolation)
- Machine-local KV keys excluded from sync to prevent topology poisoning

## Consequences

- **Pro**: Real-time cache sync between machines — no manual export/import
- **Pro**: Desktop MT5 sync feeds laptop automatically via LAN
- **Pro**: Authenticated — only machines with the passphrase can sync
- **Pro**: Incremental — only changed entries transferred
- **Pro**: Works alongside portable backup (complementary, not competing)
- **Con**: Requires both machines on same LAN
- **Con**: No TLS by default (acceptable for trusted home/office network)
- **Con**: Static salt in key derivation (acceptable for LAN-only threat model)
