# ADR-039: Portable Backup / Export System

**Status:** Implemented
**Date:** 2026-03-21

> **Note:** Extends [ADR-036](036-mt5-sqlite-direct-sync.md) (MT5 SQLite Direct Sync) and the SQLite cache system. See also [ADR-044](044-backup-lan-sync.md) for the combined Backup + LAN Sync system.

## Context

TyphooN-Terminal stores all bar data and key-value cache in a local SQLite database. When moving to a second PC (e.g., laptop for travel), the user must re-sync all symbols from MT5 or Alpaca, which can take significant time for deep history across many symbols and timeframes.

A portable backup system lets the user export the entire SQLite cache to a single compressed file, transfer it (USB, network share, cloud), and import it on the target machine. This avoids redundant API calls and hours of re-syncing.

## Decision

### Export (`export_backup`)

1. Acquire the SQLite connection lock.
2. Use `VACUUM INTO` to create a consistent snapshot of the database to a temp file (avoids WAL/lock issues).
3. Read the temp file and compress with zstd level 9 (maximum compression for archival transfer).
4. Write the compressed data to the user-specified path with `.typhoon-backup` extension.
5. Clean up the temp file.
6. Return JSON with `size_bytes` and `size_mb`.

### Import (`import_backup`)

1. Read and decompress the `.typhoon-backup` file (zstd).
2. Write decompressed SQLite DB to a temp file.
3. `ATTACH DATABASE` the temp file as `backup_db`.
4. Merge `bar_cache`: `INSERT OR REPLACE` where the backup has newer timestamps or the key does not exist locally.
5. Merge `kv_cache`: same newer-wins strategy.
6. `DETACH DATABASE` and clean up the temp file.
7. Return JSON with `bars_imported` and `kv_imported` counts.

### File Format

- Extension: `.typhoon-backup`
- Contents: zstd-compressed raw SQLite database file (output of `VACUUM INTO`)
- No tar/archive layer needed — single file is sufficient

### Frontend Integration

- Command palette: `CACHE-BACKUP` (export) and `CACHE-RESTORE` (import)
- Uses `prompt()` for file path input (no Tauri dialog plugin dependency)
- Tauri commands: `export_backup` and `import_backup`

## Consequences

- **Positive:** Users can transfer months of cached bar data in seconds instead of re-syncing.
- **Positive:** zstd level 9 gives excellent compression on already-compressed binary bar data (the outer DB structure compresses well even though bar blobs are pre-compressed).
- **Positive:** Newer-wins merge means importing never overwrites fresher local data.
- **Negative:** Large caches (hundreds of symbols, all timeframes) may produce multi-GB backup files. Acceptable for USB/network transfer.
- **Negative:** No incremental backup — always full export. Could add delta sync later if needed.
