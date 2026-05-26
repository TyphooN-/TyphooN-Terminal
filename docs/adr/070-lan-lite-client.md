# ADR-070: LAN-Lite Client Mode (On-Demand Bar Sync)

**Status:** Closed by existing client-demand sync | **Date:** 2026-04-05 | **Updated:** 2026-05-26

## Context

The original concern was that LAN clients could be forced toward full-cache
sync behavior. The current code path no longer needs a separate LAN-Lite
protocol mode for the MT5/bar-demand workflow: client mode forwards active chart
demand through `client:demand`, and the server/standalone side unions that demand
before writing `demand.txt` for BarCacheWriter.

The old full-cache comparison is kept below as historical context only; it is
not an active implementation checklist.

MT5 uses a different model: bars are downloaded on-demand when the user opens a chart. The terminal starts fast with an empty cache and populates it as the user explores symbols.

## Decision

### LAN-Lite Mode

Do not add a separate LAN-Lite protocol mode right now. Use the existing
client-demand path instead.

**Full Sync (current):** Syncs all bar data, KV cache, analytics. Client has a complete local copy.

**Client-demand sync (current):** Active viewed/gap-fill symbols are rendered to
demand text. In LAN client mode that demand is stored as `client:demand`; the
server reads it, unions it with local demand, and writes the BarCacheWriter
`demand.txt` targets. Heavy analytics remain server-side and sync through the
existing KV/cache surfaces.

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

### Resolution

No new sync message, Settings toggle, or second LAN mode is needed for the
current architecture. The existing `RequestEntries { keys }` protocol remains
available for key-level cache requests, while the active MT5/bar workflow is
handled by `client:demand` forwarding.

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
