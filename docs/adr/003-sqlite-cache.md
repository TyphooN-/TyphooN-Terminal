# ADR-003: SQLite Bar Cache

**Status:** Implemented
**Date:** 2026-03-24

## Context

Fetching historical bars from broker APIs on every startup is slow and rate-limited. The terminal needs a local cache that survives restarts, supports fast range queries by symbol and timeframe, and minimizes memory copies on the hot path.

## Decision

Use SQLite as the local bar cache with zstd-compressed TTBR (TyphooN Terminal Binary Record) format for bulk storage. The `get_bars_raw()` function returns bar data as a zero-copy byte slice that is decoded directly into the chart engine's render structs without intermediate allocation. Session state (open tabs, indicators, drawing tools) is persisted to a JSON file alongside the SQLite database, loaded at startup and saved on graceful shutdown.

## Consequences

- SQLite provides ACID guarantees, WAL mode for concurrent read/write, and zero-config deployment
- zstd compression reduces on-disk size by ~70% for OHLCV data while decompressing at >1 GB/s
- Zero-copy `get_bars_raw()` avoids allocating Vec<Bar> for large date ranges; data goes straight to GPU
- JSON session file is human-readable and easy to debug; schema changes are backward-compatible with serde defaults
- Trade-off: SQLite single-writer lock means bar cache writes are serialized; acceptable since writes are infrequent batch inserts from broker fetches
- Trade-off: JSON session file is not crash-safe; a mid-write crash could lose the last session snapshot
