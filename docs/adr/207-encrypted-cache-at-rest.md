# ADR-207: Password-Encrypted Cache at Rest

**Status:** Partially Implemented (Phase 1)
**Date:** 2026-05-02
**Updated:** 2026-05-06

## Context

`typhoon_cache.db` can contain broker-derived positions, imported DARWIN history, research packets, SEC/news/fundamental data, AI session history, and LAN sync state. A password-encrypted cache is a strong user-facing feature, especially for NAS/server deployments where the cache may sit on shared storage.

The constraint is performance: the cache is hot-path storage for chart bars, large zstd blobs, table sync, and background research. Encryption should protect data at rest without making chart loads, LAN sync, and backtests noticeably slower.

## Decision

Support encrypted cache as an opt-in mode, not the default mode for all users.

Phase 1 is now implemented as password-encrypted backup export/import:

- `SqliteCache::export_backup_encrypted(path, passphrase)` creates the same
  `VACUUM INTO` SQLite snapshot as plain backup export, zstd-compresses it at
  level 22, then writes a TyphooN AES-256-GCM envelope.
- `SqliteCache::import_backup_encrypted(path, passphrase)` decrypts the
  envelope, zstd-decompresses the snapshot, attaches it as a temporary SQLite
  database, and uses the same newer-wins `bar_cache`/`kv_cache` merge path as
  plain backup import.
- `typhoon-cli --export-cache backup.typhoon-backup --cache-backup-passphrase ...`
  writes an encrypted backup.
- `typhoon-cli --import-cache backup.typhoon-backup --cache-backup-passphrase ...`
  imports it. The CLI detects encrypted backup envelopes and refuses encrypted
  import without a passphrase.

Recommended implementation path:

1. **Phase 1: encrypted backup/export.** Add password-protected
   `.typhoon-backup` exports. Implemented with whole-snapshot authenticated
   encryption, matching the existing backup path's in-memory zstd snapshot
   behavior. Chunked AEAD can be added later if multi-GB encrypted exports need
   lower peak memory.
2. **Phase 2: encrypted live cache.** Add a dedicated encrypted cache mode behind a storage setting and migration flow.
3. **Phase 3: server unlock workflow.** Headless LAN server reads the unlock secret from OS keyring when available, or prompts/systemd-credential/Ansible-vault supplied secret at service start. The unlock password is never stored in `kv_cache`.

For live encryption, prefer page-level database encryption if a stable build path is acceptable:

- **SQLCipher/libsqlcipher**: encrypts the whole SQLite database, including research tables and WAL content. This is the cleanest security model, but adds native dependency/build complexity and must be benchmarked against multi-GB caches.

Fallback if SQLCipher creates too much packaging friction:

- **Application-level encrypted blobs**: encrypt `bar_cache.data` and `kv_cache.value` while keeping metadata columns plaintext. This is easier to ship and keeps metadata queries fast, but does not protect every research table until each table is migrated.

## Key Management

- Phase 1 encrypted backups use PBKDF2-HMAC-SHA256 with a random 16-byte
  per-backup salt, 210,000 iterations, and versioned envelope metadata.
- Future live-cache encryption may use Argon2id or PBKDF2-HMAC-SHA256 with
  per-cache random salt and versioned KDF parameters.
- Never store the raw password in SQLite.
- Store optional unlock material in OS keyring for desktop convenience.
- For headless deployment, support systemd credentials, Ansible Vault, Kubernetes Secret, or a one-shot CLI unlock command that writes to the target host keyring.
- LAN sync should continue to use TLS on the wire. Cache-at-rest encryption is local to each node; the encryption password is not synced.

## Performance

Encrypted backups have no runtime overhead outside backup export/import.

Live encrypted cache will add CPU work on page/blob reads and writes. The likely acceptable profile is:

- Small overhead for normal chart loads when SQLite page cache is warm.
- Higher overhead for cold full scans, compaction, backup, and initial LAN sync.
- Potentially noticeable cost on low-power NAS hardware.

Before making live encryption default, benchmark:

- 10K, 100K, and 1M bar loads from warm/cold cache.
- Full `detailed_stats()`/storage manager scan.
- LAN server initial sync from a 3-7 GB cache.
- Backtest and walk-forward reads.
- WAL checkpoint/compact.

## Consequences

- **Pro:** Protects sensitive local/NAS cache data at rest.
- **Pro:** Encrypted backup is low-risk and immediately useful; Phase 1 now
  protects migration/cloud/offline archives without changing live cache reads.
- **Pro:** Live encryption can be a premium-grade security feature for LAN servers.
- **Con:** SQLCipher adds native packaging complexity.
- **Con:** Application-level encryption has schema coverage gaps unless every table is migrated.
- **Con:** Headless unlock UX must be designed carefully to avoid storing the cache password in plaintext deployment manifests.

## Verification

- `cargo test --manifest-path engine/Cargo.toml encrypted_backup -- --nocapture`
- `cargo check --manifest-path cli/Cargo.toml`
