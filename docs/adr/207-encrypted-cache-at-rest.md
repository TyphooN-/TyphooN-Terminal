# ADR-207: Password-Encrypted Cache at Rest

**Status:** Proposed
**Date:** 2026-05-02

## Context

`typhoon_cache.db` can contain broker-derived positions, imported DARWIN history, research packets, SEC/news/fundamental data, AI session history, and LAN sync state. A password-encrypted cache is a strong user-facing feature, especially for NAS/server deployments where the cache may sit on shared storage.

The constraint is performance: the cache is hot-path storage for chart bars, large zstd blobs, table sync, and background research. Encryption should protect data at rest without making chart loads, LAN sync, and backtests noticeably slower.

## Decision

Support encrypted cache as an opt-in mode, not the default mode for all users.

Recommended implementation path:

1. **Phase 1: encrypted backup/export.** Add password-protected `.typhoon-backup` exports using chunked authenticated encryption. This gives immediate protection for migration, cloud sync, and offline archives with no runtime cost.
2. **Phase 2: encrypted live cache.** Add a dedicated encrypted cache mode behind a storage setting and migration flow.
3. **Phase 3: server unlock workflow.** Headless LAN server reads the unlock secret from OS keyring when available, or prompts/systemd-credential/Ansible-vault supplied secret at service start. The unlock password is never stored in `kv_cache`.

For live encryption, prefer page-level database encryption if a stable build path is acceptable:

- **SQLCipher/libsqlcipher**: encrypts the whole SQLite database, including research tables and WAL content. This is the cleanest security model, but adds native dependency/build complexity and must be benchmarked against multi-GB caches.

Fallback if SQLCipher creates too much packaging friction:

- **Application-level encrypted blobs**: encrypt `bar_cache.data` and `kv_cache.value` while keeping metadata columns plaintext. This is easier to ship and keeps metadata queries fast, but does not protect every research table until each table is migrated.

## Key Management

- Use Argon2id or PBKDF2-HMAC-SHA256 with per-cache random salt and versioned KDF parameters.
- Never store the raw password in SQLite.
- Store optional unlock material in OS keyring for desktop convenience.
- For headless deployment, support systemd credentials, Ansible Vault, Kubernetes Secret, or a one-shot CLI unlock command that writes to the target host keyring.
- LAN sync should continue to use TLS on the wire. Cache-at-rest encryption is local to each node; the encryption password is not synced.

## Performance

Encrypted backups have no runtime overhead.

Live encrypted cache will add CPU work on page/blob reads and writes. The likely acceptable profile is:

- Small overhead for normal chart loads when SQLite page cache is warm.
- Higher overhead for cold full scans, compaction, backup, and initial LAN sync.
- Potentially noticeable cost on low-power NAS hardware.

Before making it default, benchmark:

- 10K, 100K, and 1M bar loads from warm/cold cache.
- Full `detailed_stats()`/storage manager scan.
- LAN server initial sync from a 3-7 GB cache.
- Backtest and walk-forward reads.
- WAL checkpoint/compact.

## Consequences

- **Pro:** Protects sensitive local/NAS cache data at rest.
- **Pro:** Encrypted backup is low-risk and immediately useful.
- **Pro:** Live encryption can be a premium-grade security feature for LAN servers.
- **Con:** SQLCipher adds native packaging complexity.
- **Con:** Application-level encryption has schema coverage gaps unless every table is migrated.
- **Con:** Headless unlock UX must be designed carefully to avoid storing the cache password in plaintext deployment manifests.
