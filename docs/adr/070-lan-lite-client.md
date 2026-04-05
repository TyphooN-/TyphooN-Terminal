# ADR-070: LAN-Lite Client Mode (On-Demand Bar Sync)

**Status:** Proposed | **Date:** 2026-04-05

## Context

The current LAN client mode syncs the FULL bar cache from the server (~3.9 GB for 851 symbols × 9 TFs). This requires significant storage on the client machine and takes time on first sync.

MT5 uses a different model: bars are downloaded on-demand when the user opens a chart. The terminal starts fast with an empty cache and populates it as the user explores symbols.

## Decision

### LAN-Lite Mode

Add an optional "LAN-Lite" client mode alongside the existing "Full Sync" mode.

**Full Sync (current):** Syncs all bar data, KV cache, analytics. Client has a complete local copy.

**LAN-Lite (new):** Only syncs bars for symbols the user actively views. All heavy analytics remain server-side, delivered via KV cache.

### Architecture

```
LAN-Lite Client                    LAN Server
──────────────                    ──────────
User opens SOLUSD H1 chart
  → RequestBars("SOLUSD", "H1")  ─────→  Server reads from cache
                                  ←─────  EntryData("mt5:SOLUSD:1Hour", bars)
  → Chart renders immediately

User switches to EURUSD D1
  → RequestBars("EURUSD", "D1")  ─────→  Server reads from cache
                                  ←─────  EntryData("mt5:EURUSD:1Day", bars)
  → Chart renders

Analytics (VaR, correlations, 
positions, account data)          ←─────  Synced via KV cache (same as Full mode)
```

### Implementation Plan

1. **New LAN sync message:** `RequestBars { keys: Vec<String> }` — client requests specific cache keys
2. **Server handler:** On receiving `RequestBars`, reads requested keys from cache and sends `EntryData` for each
3. **Client behavior:**
   - On chart load, if bars are missing from local cache, send `RequestBars` to server
   - Cache received bars locally for offline access
   - Don't sync bars for symbols not actively viewed
4. **Toggle in Settings:** "LAN Sync Mode: Full / Lite" radio buttons
5. **KV analytics still sync normally** — positions, account, DARWIN analytics, etc. (small data)
6. **Bar cache grows incrementally** — as user views more symbols, local cache grows. Never exceeds what the user actually uses.

### Storage Comparison

| Mode | Initial Sync | Steady State | Client Storage |
|------|-------------|--------------|----------------|
| Full | ~3.9 GB (all 851 symbols) | ~50 MB/day incremental | ~4 GB |
| Lite | ~0 bytes | ~5-50 MB per symbol opened | ~100-500 MB typical |

### Existing SyncMessage Variants (for reference)

The LAN sync protocol already has `RequestEntries { keys }` which requests specific cache keys. LAN-Lite can reuse this:
- Client sends `RequestEntries { keys: ["mt5:SOLUSD:1Hour", "mt5:SOLUSD:1Day"] }`
- Server responds with `EntryData` for each key

No new protocol messages needed — just change when the client sends `RequestEntries`.

## Consequences

- **Pro:** LAN-Lite clients start instantly with near-zero storage
- **Pro:** Bandwidth proportional to symbols actively viewed, not total universe
- **Pro:** Multiple light clients can connect without each needing 4 GB
- **Pro:** Reuses existing LAN sync protocol (RequestEntries/EntryData)
- **Con:** First chart load for a new symbol has network latency (~1-2s)
- **Con:** No offline access for unviewed symbols
- **Con:** If server is down, only previously-cached symbols available
