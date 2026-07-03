//! SQLite-backed cache for unlimited structured storage.
//!
//! Replaces IndexedDB's ~50MB limit with SQLite (no practical limit).
//! Bar data uses packed binary format (48 bytes/bar) + configurable zstd compression.
//! KV data uses JSON + zstd compression.
//! Binary format: [u32 bar_count][per bar: i64 timestamp_ms, f64 OHLCV]

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use chrono::Datelike;
pub use rusqlite::Connection;
use rusqlite::{OpenFlags, params};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicI32, Ordering};
use zeroize::Zeroize;

/// Re-export rusqlite::Connection so callers can use BG connections without depending on rusqlite directly.
pub type BgConnection = Connection;

/// Magic bytes to identify binary bar format (vs legacy JSON).
const BAR_BINARY_MAGIC: &[u8; 4] = b"TTBR"; // TyphooN Terminal Bar Record
/// Bytes per bar in binary format: i64 timestamp + 5×f64 (OHLCV) = 48 bytes
const BYTES_PER_BAR: usize = 8 + 5 * 8; // 48
pub const DEFAULT_BAR_ZSTD_LEVEL: i32 = 3;
pub const MIN_ZSTD_LEVEL: i32 = 1;
pub const MAX_ZSTD_LEVEL: i32 = 22;
static BAR_ZSTD_LEVEL: AtomicI32 = AtomicI32::new(DEFAULT_BAR_ZSTD_LEVEL);

pub fn sanitize_zstd_level(level: i32) -> i32 {
    level.clamp(MIN_ZSTD_LEVEL, MAX_ZSTD_LEVEL)
}

pub fn set_bar_zstd_level(level: i32) -> i32 {
    let level = sanitize_zstd_level(level);
    BAR_ZSTD_LEVEL.store(level, Ordering::Relaxed);
    level
}

pub fn bar_zstd_level() -> i32 {
    sanitize_zstd_level(BAR_ZSTD_LEVEL.load(Ordering::Relaxed))
}
const BACKUP_ZSTD_LEVEL: i32 = 22;
const ENCRYPTED_BACKUP_MAGIC: &[u8] = b"TYPHOON-BACKUP-AESGCM-V1\0";
const ENCRYPTED_BACKUP_ITERATIONS: u32 = 210_000;
const ENCRYPTED_BACKUP_SALT_LEN: usize = 16;
const ENCRYPTED_BACKUP_NONCE_LEN: usize = 12;

fn derive_backup_key(passphrase: &str, salt: &[u8], iterations: u32) -> [u8; 32] {
    let mut key = [0u8; 32];
    pbkdf2::pbkdf2_hmac::<sha2::Sha256>(passphrase.as_bytes(), salt, iterations, &mut key);
    key
}

fn encrypted_backup_header(iterations: u32, salt: &[u8], nonce: &[u8]) -> Result<Vec<u8>, String> {
    if salt.len() > u8::MAX as usize || nonce.len() > u8::MAX as usize {
        return Err("Encrypted backup salt/nonce too large".to_string());
    }
    let mut header =
        Vec::with_capacity(ENCRYPTED_BACKUP_MAGIC.len() + 6 + salt.len() + nonce.len());
    header.extend_from_slice(ENCRYPTED_BACKUP_MAGIC);
    header.extend_from_slice(&iterations.to_be_bytes());
    header.push(salt.len() as u8);
    header.push(nonce.len() as u8);
    header.extend_from_slice(salt);
    header.extend_from_slice(nonce);
    Ok(header)
}

fn parse_encrypted_backup_header(data: &[u8]) -> Result<(u32, &[u8], &[u8], usize), String> {
    if !data.starts_with(ENCRYPTED_BACKUP_MAGIC) {
        return Err("Not a TyphooN encrypted backup".to_string());
    }
    let fixed_len = ENCRYPTED_BACKUP_MAGIC.len() + 6;
    if data.len() < fixed_len {
        return Err("Encrypted backup header is truncated".to_string());
    }
    let iterations_offset = ENCRYPTED_BACKUP_MAGIC.len();
    let iterations = u32::from_be_bytes(
        data[iterations_offset..iterations_offset + 4]
            .try_into()
            .map_err(|_| "Encrypted backup iterations are malformed".to_string())?,
    );
    let salt_len = data[iterations_offset + 4] as usize;
    let nonce_len = data[iterations_offset + 5] as usize;
    if salt_len != ENCRYPTED_BACKUP_SALT_LEN || nonce_len != ENCRYPTED_BACKUP_NONCE_LEN {
        return Err("Unsupported encrypted backup salt/nonce length".to_string());
    }
    let header_len = fixed_len + salt_len + nonce_len;
    if data.len() <= header_len {
        return Err("Encrypted backup payload is missing".to_string());
    }
    let salt_start = fixed_len;
    let nonce_start = salt_start + salt_len;
    Ok((
        iterations,
        &data[salt_start..nonce_start],
        &data[nonce_start..header_len],
        header_len,
    ))
}

fn encrypt_backup_payload(compressed: &[u8], passphrase: &str) -> Result<Vec<u8>, String> {
    if passphrase.is_empty() {
        return Err("Encrypted backup passphrase cannot be empty".to_string());
    }
    let salt: [u8; ENCRYPTED_BACKUP_SALT_LEN] = rand::random();
    let nonce_bytes: [u8; ENCRYPTED_BACKUP_NONCE_LEN] = rand::random();
    let header = encrypted_backup_header(ENCRYPTED_BACKUP_ITERATIONS, &salt, &nonce_bytes)?;
    let mut key = derive_backup_key(passphrase, &salt, ENCRYPTED_BACKUP_ITERATIONS);
    let cipher = match Aes256Gcm::new_from_slice(&key) {
        Ok(cipher) => cipher,
        Err(_) => {
            key.zeroize();
            return Err("Create encrypted backup cipher failed".to_string());
        }
    };
    let encrypted = cipher
        .encrypt(
            &Nonce::from(nonce_bytes),
            Payload {
                msg: compressed,
                aad: &header,
            },
        )
        .map_err(|_| "Encrypt backup failed".to_string());
    key.zeroize();
    let ciphertext = encrypted?;

    let mut out = Vec::with_capacity(header.len() + ciphertext.len());
    out.extend_from_slice(&header);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

fn decrypt_backup_payload(encrypted: &[u8], passphrase: &str) -> Result<Vec<u8>, String> {
    if passphrase.is_empty() {
        return Err("Encrypted backup passphrase cannot be empty".to_string());
    }
    let (iterations, salt, nonce_bytes, header_len) = parse_encrypted_backup_header(encrypted)?;
    let header = &encrypted[..header_len];
    let ciphertext = &encrypted[header_len..];
    let mut key = derive_backup_key(passphrase, salt, iterations);
    let cipher = match Aes256Gcm::new_from_slice(&key) {
        Ok(cipher) => cipher,
        Err(_) => {
            key.zeroize();
            return Err("Create encrypted backup cipher failed".to_string());
        }
    };
    // Length is validated against ENCRYPTED_BACKUP_NONCE_LEN by the header parser.
    let nonce = Nonce::try_from(nonce_bytes).map_err(|_| {
        key.zeroize();
        "Unsupported encrypted backup salt/nonce length".to_string()
    });
    let nonce = match nonce {
        Ok(nonce) => nonce,
        Err(e) => return Err(e),
    };
    let decrypted = cipher
        .decrypt(
            &nonce,
            Payload {
                msg: ciphertext,
                aad: header,
            },
        )
        .map_err(|_| {
            "Decrypt backup failed; passphrase may be wrong or file is corrupt".to_string()
        });
    key.zeroize();
    let compressed = decrypted?;
    Ok(compressed)
}

/// Decompress bar data if needed. Some legacy cache blobs are stored as raw TTBR
/// (magic "TTBR" at byte 0); `put_bars()` stores zstd-compressed (magic
/// 0x28B52FFD). This function handles both so old caches still read correctly.
fn maybe_decompress(data: Vec<u8>) -> Result<Vec<u8>, String> {
    if data.len() >= 4 && &data[0..4] == BAR_BINARY_MAGIC {
        Ok(data) // Already raw TTBR — no decompression needed
    } else {
        zstd::decode_all(data.as_slice()).map_err(|e| format!("Decompress failed: {e}"))
    }
}

/// Pack bars from JSON into binary format for efficient storage.
/// Format: [4-byte magic][u32 count][per bar: i64 ts_ms, f64 O, f64 H, f64 L, f64 C, f64 V]
///
/// Bars with unparseable timestamps or invalid OHLC (non-positive, NaN, high<low) are
/// silently dropped — corrupt rows that previously defaulted to epoch 0 polluted charts
/// with a phantom flat line at the far left.
fn bar_timeframe_from_key(key: &str) -> Option<&str> {
    key.rsplit(':').next()
}

fn normalized_bar_timestamp_ms(key: &str, ts_ms: i64) -> Option<i64> {
    let dt = chrono::DateTime::from_timestamp_millis(ts_ms)?.with_timezone(&chrono::Utc);
    match bar_timeframe_from_key(key) {
        Some("1Day") => chrono::NaiveDate::from_ymd_opt(dt.year(), dt.month(), dt.day())?
            .and_hms_opt(0, 0, 0)
            .map(|ndt| ndt.and_utc().timestamp_millis()),
        Some("1Week") => {
            let monday = dt.date_naive().week(chrono::Weekday::Mon).first_day();
            monday
                .and_hms_opt(0, 0, 0)
                .map(|ndt| ndt.and_utc().timestamp_millis())
        }
        Some("1Month") => chrono::NaiveDate::from_ymd_opt(dt.year(), dt.month(), 1)?
            .and_hms_opt(0, 0, 0)
            .map(|ndt| ndt.and_utc().timestamp_millis()),
        _ => Some(ts_ms),
    }
}

#[cfg(test)]
fn pack_bars(json_data: &str) -> Result<Vec<u8>, String> {
    pack_bars_for_key("", json_data)
}

fn pack_bars_for_key(key: &str, json_data: &str) -> Result<Vec<u8>, String> {
    let bars: Vec<serde_json::Value> =
        serde_json::from_str(json_data).map_err(|e| format!("JSON parse failed: {e}"))?;
    let mut by_bucket: std::collections::BTreeMap<i64, (f64, f64, f64, f64, f64)> =
        std::collections::BTreeMap::new();
    for bar in &bars {
        let ts_str = bar["timestamp"].as_str().unwrap_or("");
        let ts_ms = match chrono::DateTime::parse_from_rfc3339(ts_str) {
            Ok(dt) => dt.timestamp_millis(),
            Err(_) => continue,
        };
        let Some(ts_ms) = normalized_bar_timestamp_ms(key, ts_ms) else {
            continue;
        };
        if ts_ms <= 0 {
            continue;
        }
        let o = bar["open"].as_f64().unwrap_or(0.0);
        let h = bar["high"].as_f64().unwrap_or(0.0);
        let l = bar["low"].as_f64().unwrap_or(0.0);
        let c = bar["close"].as_f64().unwrap_or(0.0);
        let v = bar["volume"].as_f64().unwrap_or(0.0);
        if !(o > 0.0 && h > 0.0 && l > 0.0 && c > 0.0) {
            continue;
        }
        if !(o.is_finite() && h.is_finite() && l.is_finite() && c.is_finite() && v.is_finite()) {
            continue;
        }
        if h < l {
            continue;
        }
        // Later bars for the same merge bucket win. This handles provider
        // refreshes that return an early partial D/W/M candle and then a
        // finalized candle with the same session key.
        by_bucket.insert(ts_ms, (o, h, l, c, v));
    }

    let mut buf = Vec::with_capacity(4 + 4 + by_bucket.len() * BYTES_PER_BAR);
    buf.extend_from_slice(BAR_BINARY_MAGIC);
    buf.extend_from_slice(&(by_bucket.len() as u32).to_le_bytes());
    for (ts_ms, (o, h, l, c, v)) in by_bucket {
        buf.extend_from_slice(&ts_ms.to_le_bytes());
        buf.extend_from_slice(&o.to_le_bytes());
        buf.extend_from_slice(&h.to_le_bytes());
        buf.extend_from_slice(&l.to_le_bytes());
        buf.extend_from_slice(&c.to_le_bytes());
        buf.extend_from_slice(&v.to_le_bytes());
    }
    Ok(buf)
}

/// Unpack binary bars back to JSON string for frontend consumption.
fn unpack_bars(data: &[u8]) -> Result<String, String> {
    if data.len() < 8 || &data[0..4] != BAR_BINARY_MAGIC {
        return Err("Not binary bar format".into());
    }
    let count = u32::from_le_bytes(
        data[4..8]
            .try_into()
            .map_err(|_| "Failed to read bar_count from binary header")?,
    ) as usize;
    let expected = count
        .checked_mul(BYTES_PER_BAR)
        .and_then(|n| n.checked_add(8))
        .ok_or("Integer overflow computing bar data size")?;
    if data.len() < expected {
        return Err(format!(
            "Binary data truncated: expected {expected}, got {}",
            data.len()
        ));
    }

    let mut bars = Vec::with_capacity(count);
    for i in 0..count {
        let offset = 8 + i * BYTES_PER_BAR;
        let ts_ms = i64::from_le_bytes(
            data[offset..offset + 8]
                .try_into()
                .map_err(|_| format!("Bad timestamp at bar {i}"))?,
        );
        let open = f64::from_le_bytes(
            data[offset + 8..offset + 16]
                .try_into()
                .map_err(|_| format!("Bad open at bar {i}"))?,
        );
        let high = f64::from_le_bytes(
            data[offset + 16..offset + 24]
                .try_into()
                .map_err(|_| format!("Bad high at bar {i}"))?,
        );
        let low = f64::from_le_bytes(
            data[offset + 24..offset + 32]
                .try_into()
                .map_err(|_| format!("Bad low at bar {i}"))?,
        );
        let close = f64::from_le_bytes(
            data[offset + 32..offset + 40]
                .try_into()
                .map_err(|_| format!("Bad close at bar {i}"))?,
        );
        let volume = f64::from_le_bytes(
            data[offset + 40..offset + 48]
                .try_into()
                .map_err(|_| format!("Bad volume at bar {i}"))?,
        );

        // Convert epoch ms back to RFC3339 timestamp
        let dt = chrono::DateTime::from_timestamp_millis(ts_ms).unwrap_or_default();
        bars.push(serde_json::json!({
            "timestamp": dt.to_rfc3339(),
            "open": open, "high": high, "low": low, "close": close, "volume": volume,
        }));
    }

    serde_json::to_string(&bars).map_err(|e| format!("JSON serialize failed: {e}"))
}

/// Unpack binary bars to Vec of (timestamp_ms, open, high, low, close, volume) tuples.
/// Zero-copy-friendly: returns raw f64 data, no JSON serialization.
/// Used by native GPU renderer to go directly from cache → GPU vertex buffer.
pub fn unpack_bars_raw(data: &[u8]) -> Result<Vec<(i64, f64, f64, f64, f64, f64)>, String> {
    if data.len() < 8 || &data[0..4] != BAR_BINARY_MAGIC {
        return Err("Not binary bar format".into());
    }
    let count = u32::from_le_bytes(
        data[4..8]
            .try_into()
            .map_err(|_| "Failed to read bar_count")?,
    ) as usize;
    let expected = count
        .checked_mul(BYTES_PER_BAR)
        .and_then(|n| n.checked_add(8))
        .ok_or("Integer overflow computing bar data size")?;
    if data.len() < expected {
        return Err(format!(
            "Binary data truncated: expected {expected}, got {}",
            data.len()
        ));
    }
    let mut bars = Vec::with_capacity(count);
    for i in 0..count {
        let off = 8 + i * BYTES_PER_BAR;
        // Bounds already validated above (data.len() >= expected), but use get() for defense in depth.
        let sl = data
            .get(off..off + BYTES_PER_BAR)
            .ok_or("Bar data slice out of bounds")?;
        let ts = i64::from_le_bytes(sl[0..8].try_into().map_err(|_| "Bad bar timestamp")?);
        let o = f64::from_le_bytes(sl[8..16].try_into().map_err(|_| "Bad bar open")?);
        let h = f64::from_le_bytes(sl[16..24].try_into().map_err(|_| "Bad bar high")?);
        let l = f64::from_le_bytes(sl[24..32].try_into().map_err(|_| "Bad bar low")?);
        let c = f64::from_le_bytes(sl[32..40].try_into().map_err(|_| "Bad bar close")?);
        let v = f64::from_le_bytes(sl[40..48].try_into().map_err(|_| "Bad bar volume")?);
        bars.push((ts, o, h, l, c, v));
    }
    Ok(bars)
}

/// Extract last and second-to-last bar timestamps from binary data (for metadata columns).
/// Returns (second_last_ts_rfc3339, last_ts_rfc3339) or empty strings if not enough bars.
fn get_last_two_bar_timestamps(binary: &[u8], count: usize) -> (Option<String>, Option<String>) {
    let required = match count
        .checked_mul(BYTES_PER_BAR)
        .and_then(|n| n.checked_add(8))
    {
        Some(n) => n,
        None => return (None, None),
    };
    if count < 2 || binary.len() < required {
        return (None, None);
    }
    let last_offset = 8 + (count - 1) * BYTES_PER_BAR;
    let second_offset = 8 + (count - 2) * BYTES_PER_BAR;
    let last_ts = i64::from_le_bytes(
        binary[last_offset..last_offset + 8]
            .try_into()
            .unwrap_or([0; 8]),
    );
    let second_ts = i64::from_le_bytes(
        binary[second_offset..second_offset + 8]
            .try_into()
            .unwrap_or([0; 8]),
    );
    let fmt = |ms: i64| -> Option<String> {
        chrono::DateTime::from_timestamp_millis(ms).map(|dt| dt.to_rfc3339())
    };
    (fmt(second_ts), fmt(last_ts))
}

/// Unpack only the last `tail` bars from binary format — avoids converting 50K bars when only 500 needed.
/// Decompression is still required (zstd doesn't support seeking), but JSON construction is O(tail) not O(n).
fn unpack_bars_tail(data: &[u8], tail: usize) -> Result<String, String> {
    if data.len() < 8 || &data[0..4] != BAR_BINARY_MAGIC {
        return Err("Not binary bar format".into());
    }
    let count = u32::from_le_bytes(
        data[4..8]
            .try_into()
            .map_err(|_| "Failed to read bar_count from binary header")?,
    ) as usize;
    let expected = count
        .checked_mul(BYTES_PER_BAR)
        .and_then(|n| n.checked_add(8))
        .ok_or("Integer overflow computing bar data size")?;
    if data.len() < expected {
        return Err(format!(
            "Binary data truncated: expected {expected}, got {}",
            data.len()
        ));
    }
    if tail == 0 || tail >= count {
        return unpack_bars(data); // no trimming needed
    }

    let start_bar = count - tail;
    let mut bars = Vec::with_capacity(tail);
    for i in start_bar..count {
        let offset = 8 + i * BYTES_PER_BAR;
        let ts_ms = i64::from_le_bytes(
            data[offset..offset + 8]
                .try_into()
                .map_err(|_| format!("Bad timestamp at bar {i}"))?,
        );
        let open = f64::from_le_bytes(
            data[offset + 8..offset + 16]
                .try_into()
                .map_err(|_| format!("Bad open at bar {i}"))?,
        );
        let high = f64::from_le_bytes(
            data[offset + 16..offset + 24]
                .try_into()
                .map_err(|_| format!("Bad high at bar {i}"))?,
        );
        let low = f64::from_le_bytes(
            data[offset + 24..offset + 32]
                .try_into()
                .map_err(|_| format!("Bad low at bar {i}"))?,
        );
        let close = f64::from_le_bytes(
            data[offset + 32..offset + 40]
                .try_into()
                .map_err(|_| format!("Bad close at bar {i}"))?,
        );
        let volume = f64::from_le_bytes(
            data[offset + 40..offset + 48]
                .try_into()
                .map_err(|_| format!("Bad volume at bar {i}"))?,
        );
        let dt = chrono::DateTime::from_timestamp_millis(ts_ms).unwrap_or_default();
        bars.push(serde_json::json!({
            "timestamp": dt.to_rfc3339(),
            "open": open, "high": high, "low": low, "close": close, "volume": volume,
        }));
    }
    serde_json::to_string(&bars).map_err(|e| format!("JSON serialize failed: {e}"))
}

/// Number of independent read-only connections in the read pool. WAL allows many
/// concurrent readers; a single shared read `Connection` (behind one `Mutex`) was
/// the bottleneck — a background worker's zstd decompress is held *under the lock*
/// inside `get_bars_raw`, so it parked the render thread's small reads (prev-candle
/// levels, watchlist quotes) for the whole decompress. 4 covers the common case:
/// the render thread + the 3 deferred-chart-load workers reading at once.
const READ_CONN_POOL_SIZE: usize = 4;

/// A small pool of independent read-only SQLite connections that fans readers out
/// instead of serializing them through one shared connection. `lock` / `try_lock`
/// mirror `std::sync::Mutex` exactly (same return types), so every existing call
/// site (`self.read_conn.lock()`, `.try_lock()`) is unchanged.
struct ReadConnPool {
    conns: Vec<Mutex<Connection>>,
    next: std::sync::atomic::AtomicUsize,
}

impl ReadConnPool {
    fn new(conns: Vec<Mutex<Connection>>) -> Self {
        debug_assert!(!conns.is_empty(), "read pool must have at least one conn");
        Self {
            conns,
            next: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Acquire any currently-free connection without blocking; if all are busy,
    /// block on a round-robin pick. Mirrors `Mutex::lock`'s return type.
    fn lock(&self) -> std::sync::LockResult<std::sync::MutexGuard<'_, Connection>> {
        let n = self.conns.len();
        let start = self.next.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % n;
        for k in 0..n {
            if let Ok(guard) = self.conns[(start + k) % n].try_lock() {
                return Ok(guard);
            }
        }
        // Everything busy — block on the round-robin pick so the wait is spread.
        self.conns[start].lock()
    }

    /// Acquire any currently-free connection, or `WouldBlock` if all are busy.
    /// Mirrors `Mutex::try_lock`.
    fn try_lock(&self) -> std::sync::TryLockResult<std::sync::MutexGuard<'_, Connection>> {
        let n = self.conns.len();
        let start = self.next.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % n;
        for k in 0..n {
            match self.conns[(start + k) % n].try_lock() {
                Ok(guard) => return Ok(guard),
                Err(std::sync::TryLockError::Poisoned(e)) => {
                    return Err(std::sync::TryLockError::Poisoned(e));
                }
                Err(std::sync::TryLockError::WouldBlock) => {}
            }
        }
        Err(std::sync::TryLockError::WouldBlock)
    }
}

/// Open `READ_CONN_POOL_SIZE` independent read-only connections for the read pool.
/// `cache_size` is the per-connection page-cache pragma (negative = KiB). `with_mmap`
/// enables the 256MB mmap, which the OS page-cache backs and shares across
/// connections (so it is NOT multiplied by the pool size).
fn open_read_conn_pool(
    path: &PathBuf,
    cache_size: i64,
    with_mmap: bool,
) -> Result<ReadConnPool, String> {
    let mut conns = Vec::with_capacity(READ_CONN_POOL_SIZE);
    for _ in 0..READ_CONN_POOL_SIZE {
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|e| format!("SQLite read conn open failed: {e}"))?;
        conn.busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|e| format!("SQLite read conn busy_timeout failed: {e}"))?;
        let mmap = if with_mmap {
            "PRAGMA mmap_size=268435456;"
        } else {
            ""
        };
        let _ = conn.execute_batch(&format!(
            "PRAGMA cache_size={cache_size}; PRAGMA temp_store=MEMORY; {mmap}"
        ));
        conns.push(Mutex::new(conn));
    }
    Ok(ReadConnPool::new(conns))
}

/// Thread-safe SQLite cache manager.
///
/// Uses separate connections for concurrency under WAL mode:
/// - `conn` (Mutex): exclusive write path — put_bars, put_kv, delete, compact, etc.
/// - `read_conn` (`ReadConnPool`): dedicated read path — get_bars_raw, detailed_stats,
///   stats, etc. Never blocked by writes. Several readers (render thread + the
///   deferred-chart-load workers) fan out across the pool's connections instead of
///   serializing through one, so a worker's in-lock zstd decompress no longer parks
///   the render thread's small reads.
///
/// SQLite WAL mode allows unlimited concurrent readers + one writer. The write Mutex
/// and the read pool are independent — a write lock on `conn` does NOT block reads.
pub struct SqliteCache {
    conn: Mutex<Connection>,
    read_conn: ReadConnPool,
    db_path: PathBuf,
}

impl SqliteCache {
    fn total_disk_usage_bytes(db_path: &Path) -> i64 {
        let mut total = std::fs::metadata(db_path)
            .map(|m| m.len() as i64)
            .unwrap_or(0);
        for suffix in ["-wal", "-shm"] {
            let sidecar = PathBuf::from(format!("{}{}", db_path.to_string_lossy(), suffix));
            total += std::fs::metadata(sidecar)
                .map(|m| m.len() as i64)
                .unwrap_or(0);
        }
        total
    }

    fn reclaim_space_locked(conn: &Connection, db_path: &Path) -> Result<(i64, i64), String> {
        let before = Self::total_disk_usage_bytes(db_path);
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .map_err(|e| format!("WAL checkpoint failed: {e}"))?;
        conn.execute_batch("VACUUM")
            .map_err(|e| format!("VACUUM failed: {e}"))?;
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .map_err(|e| format!("post-VACUUM checkpoint failed: {e}"))?;
        let after = Self::total_disk_usage_bytes(db_path);
        Ok((before, after))
    }

    fn purge_obsolete_low_tf_provider_bars_locked(conn: &Connection) -> Result<usize, String> {
        let deleted = conn
            .execute(
                "DELETE FROM bar_cache
             WHERE (key LIKE 'alpaca:%:1Min'
                 OR key LIKE 'alpaca:%:5Min'
                 OR key LIKE 'yahoo-chart:%:1Min'
                 OR key LIKE 'yahoo-chart:%:5Min')",
                [],
            )
            .map_err(|e| format!("obsolete low-TF provider bar purge failed: {e}"))?;
        let _ = conn.execute(
            "DELETE FROM bar_track
             WHERE (key LIKE 'alpaca:%:1Min'
                 OR key LIKE 'alpaca:%:5Min'
                 OR key LIKE 'yahoo-chart:%:1Min'
                 OR key LIKE 'yahoo-chart:%:5Min')",
            [],
        );
        let _ = conn.execute(
            "DELETE FROM kv_cache
             WHERE (key LIKE 'alpaca:%:1Min'
                 OR key LIKE 'alpaca:%:5Min'
                 OR key LIKE 'yahoo-chart:%:1Min'
                 OR key LIKE 'yahoo-chart:%:5Min')",
            [],
        );
        Ok(deleted)
    }

    /// Open or create a SQLite database at the given path.
    pub fn open(path: &PathBuf) -> Result<Self, String> {
        let conn = Connection::open(path).map_err(|e| format!("SQLite open failed: {e}"))?;

        // WAL mode for concurrent reads + single writer performance.
        // This is used for the main typhoon_cache.db which is accessed only by
        // TyphooN-Terminal (Linux native). WAL shared memory works fine here.
        // busy_timeout=5000ms: retry for 5s on SQLITE_BUSY instead of failing
        // immediately. Critical when compact_storage() holds the write lock in
        // batches and other threads (e.g. bar fetches, SEC scraping) need to write concurrently.
        conn.execute_batch(
            "
            PRAGMA journal_mode=WAL;
            PRAGMA synchronous=NORMAL;
            PRAGMA cache_size=-64000;
            PRAGMA temp_store=MEMORY;
            PRAGMA mmap_size=268435456;
            PRAGMA auto_vacuum=INCREMENTAL;
            PRAGMA wal_autocheckpoint=2000;
            PRAGMA busy_timeout=5000;
        ",
        )
        .map_err(|e| format!("SQLite pragma failed: {e}"))?;

        // Create tables
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS bar_cache (
                key TEXT PRIMARY KEY,
                data BLOB NOT NULL,
                timestamp INTEGER NOT NULL,
                bar_count INTEGER NOT NULL DEFAULT 0,
                zstd_level INTEGER NOT NULL DEFAULT 22
            );
            CREATE TABLE IF NOT EXISTS kv_cache (
                key TEXT PRIMARY KEY,
                value BLOB NOT NULL,
                timestamp INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_bar_cache_ts ON bar_cache(timestamp);
            CREATE INDEX IF NOT EXISTS idx_bar_meta ON bar_cache(key, timestamp, bar_count);
            CREATE INDEX IF NOT EXISTS idx_kv_cache_ts ON kv_cache(timestamp);
        ",
        )
        .map_err(|e| format!("SQLite create tables failed: {e}"))?;

        // Schema migration: add last_ts column for fast incremental start lookup
        // (avoids decompressing full binary blob just to read 2 timestamps)
        let _ = conn.execute("ALTER TABLE bar_cache ADD COLUMN last_ts TEXT", []);
        let _ = conn.execute("ALTER TABLE bar_cache ADD COLUMN second_last_ts TEXT", []);
        // Schema migration: track zstd compression level per entry (compact skips already-compacted)
        let _ = conn.execute(
            "ALTER TABLE bar_cache ADD COLUMN zstd_level INTEGER NOT NULL DEFAULT 22",
            [],
        );

        // One-shot migration: purge existing Alpaca stock bar entries. Prior builds never
        // requested adjustment=all, so every cached stock series is split-unadjusted and
        // renders as flat-line-then-spike for any symbol that had a reverse split. Crypto
        // keys (which contain a slash like "alpaca:BTC/USD:1Day") are left intact.
        let migration_marker = "__migration__alpaca_bar_adjust_2026_04__";
        let already_migrated: bool = conn
            .query_row(
                "SELECT 1 FROM kv_cache WHERE key = ?1",
                params![migration_marker],
                |_| Ok(true),
            )
            .unwrap_or(false);
        if !already_migrated {
            let purged = conn
                .execute(
                    "DELETE FROM bar_cache WHERE key LIKE 'alpaca:%' AND key NOT LIKE 'alpaca:%/%'",
                    [],
                )
                .unwrap_or(0);
            tracing::info!(
                "cache migration: purged {} alpaca stock bar entries (re-fetch with adjustment=all)",
                purged
            );
            let _ = conn.execute(
                "INSERT OR REPLACE INTO kv_cache (key, value, timestamp) VALUES (?1, ?2, ?3)",
                params![
                    migration_marker,
                    purged.to_string().as_bytes(),
                    chrono::Utc::now().timestamp()
                ],
            );
        }

        // One-shot migration: remove stale low-timeframe provider-assist bars.
        // M1/M5 remain valid for Kraken Spot and Kraken Equities/xStocks now.
        // Alpaca/Yahoo assist low-TF rows are still not broad merge targets;
        // they make equities look better-covered than they are and inflate
        // startup cache work. Keep all native Kraken rows and higher-TF assists.
        let migration_marker = "__migration__purge_nonspot_provider_1m5m_2026_06__";
        let already_migrated: bool = conn
            .query_row(
                "SELECT 1 FROM kv_cache WHERE key = ?1",
                params![migration_marker],
                |_| Ok(true),
            )
            .unwrap_or(false);
        if !already_migrated {
            let purged = Self::purge_obsolete_low_tf_provider_bars_locked(&conn)?;
            tracing::info!(
                "cache migration: purged {} obsolete provider-assist M1/M5 bar entries",
                purged
            );
            let _ = conn.execute(
                "INSERT OR REPLACE INTO kv_cache (key, value, timestamp) VALUES (?1, ?2, ?3)",
                params![
                    migration_marker,
                    purged.to_string().as_bytes(),
                    chrono::Utc::now().timestamp()
                ],
            );
        }

        // Follow-up for installs that already ran the first bar-only migration:
        // clear any matching metadata/sync KVs too. The helper is idempotent and
        // also keeps the bar tables clean for fresh installs.
        let metadata_migration_marker =
            "__migration__purge_nonspot_provider_1m5m_metadata_2026_06__";
        let metadata_already_migrated: bool = conn
            .query_row(
                "SELECT 1 FROM kv_cache WHERE key = ?1",
                params![metadata_migration_marker],
                |_| Ok(true),
            )
            .unwrap_or(false);
        if !metadata_already_migrated {
            let purged = Self::purge_obsolete_low_tf_provider_bars_locked(&conn)?;
            tracing::info!(
                "cache migration: verified obsolete provider-assist M1/M5 metadata purge ({} bar rows removed)",
                purged
            );
            let _ = conn.execute(
                "INSERT OR REPLACE INTO kv_cache (key, value, timestamp) VALUES (?1, ?2, ?3)",
                params![
                    metadata_migration_marker,
                    purged.to_string().as_bytes(),
                    chrono::Utc::now().timestamp()
                ],
            );
        }

        // Open the read-path connection POOL — read-only connections that read
        // concurrently with the write `conn` (WAL) AND with each other. 32MB page
        // cache per connection (128MB across the pool, vs the old single 64MB); the
        // 256MB mmap is OS-shared, so it isn't multiplied. The pool is what keeps a
        // worker's in-lock decompress from parking the render thread's reads.
        let read_conn = open_read_conn_pool(path, -32000, true)?;

        Ok(Self {
            conn: Mutex::new(conn),
            read_conn,
            db_path: path.clone(),
        })
    }

    /// Open an existing database read-only.
    ///
    /// Does NOT change journal_mode. Read-only mode means SQLite never needs a
    /// write lock, so another process can keep writing the same file concurrently
    /// without "database is locked" errors. Used by the CLI to inspect the main
    /// cache DB without taking a write lock.
    pub fn open_readonly(path: &PathBuf) -> Result<Self, String> {
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|e| format!("SQLite read-only open failed: {e}"))?;
        // busy_timeout MUST be set first so reads retry rather than failing
        // instantly with SQLITE_BUSY if another writer holds an exclusive lock.
        // Use rusqlite's built-in method (doesn't require a DB lock to execute).
        conn.busy_timeout(std::time::Duration::from_secs(10))
            .map_err(|e| format!("SQLite busy_timeout failed: {e}"))?;
        // Non-critical optimizations — ignore failures (DB may be locked briefly)
        let _ = conn.execute_batch(
            "
            PRAGMA cache_size=-16000;
            PRAGMA temp_store=MEMORY;
        ",
        );
        // Read-only: use a read-only connection pool for the read path too.
        let read_conn = open_read_conn_pool(path, -16000, false)?;
        Ok(Self {
            conn: Mutex::new(conn),
            read_conn,
            db_path: path.clone(),
        })
    }

    /// Store bar data in packed binary format + zstd compression.
    /// Binary format is ~3-5x smaller than JSON before compression.
    /// Uses live-ingest zstd level for bar blobs. Max compression belongs in idle
    /// compaction; using zstd-22 during broad sync can saturate CPU and starve egui.
    pub fn put_bars(&self, key: &str, json_data: &str) -> Result<(), String> {
        let binary = pack_bars_for_key(key, json_data)?;
        let bar_count = u32::from_le_bytes(
            binary[4..8]
                .try_into()
                .map_err(|_| "bar_count header slice failed")?,
        ) as i64;
        let (second_last_ts, last_ts) = get_last_two_bar_timestamps(&binary, bar_count as usize);
        let zstd_level = bar_zstd_level();
        let compressed = zstd::encode_all(binary.as_slice(), zstd_level)
            .map_err(|e| format!("zstd compress failed: {e}"))?;
        let timestamp = chrono::Utc::now().timestamp();

        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        conn.execute(
            "INSERT OR REPLACE INTO bar_cache (key, data, timestamp, bar_count, last_ts, second_last_ts, zstd_level) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![key, compressed, timestamp, bar_count, last_ts, second_last_ts, zstd_level],
        ).map_err(|e| format!("SQLite insert failed: {e}"))?;
        Ok(())
    }

    /// Best-effort variant of [`put_bars`] for hot render-thread callers (e.g.
    /// merged-equity cache warming). Does all the CPU prep (pack + compress) up
    /// front, then writes *only* if the writer connection is immediately
    /// available; returns `Ok(false)` (skipped) when it is busy — typically held
    /// by bulk bar-sync — so the render thread never stalls behind a long write
    /// transaction. The blob is a best-effort cache, so callers must already
    /// tolerate it being absent (it gets re-materialised off-thread).
    /// Mirrors the prep in [`put_bars`]; keep the two in sync.
    pub fn put_bars_if_uncontended(&self, key: &str, json_data: &str) -> Result<bool, String> {
        let binary = pack_bars_for_key(key, json_data)?;
        let bar_count = u32::from_le_bytes(
            binary[4..8]
                .try_into()
                .map_err(|_| "bar_count header slice failed")?,
        ) as i64;
        let (second_last_ts, last_ts) = get_last_two_bar_timestamps(&binary, bar_count as usize);
        let zstd_level = bar_zstd_level();
        let compressed = zstd::encode_all(binary.as_slice(), zstd_level)
            .map_err(|e| format!("zstd compress failed: {e}"))?;
        let timestamp = chrono::Utc::now().timestamp();

        let conn = match self.conn.try_lock() {
            Ok(conn) => conn,
            Err(std::sync::TryLockError::WouldBlock) => return Ok(false),
            Err(std::sync::TryLockError::Poisoned(e)) => return Err(format!("Lock poisoned: {e}")),
        };
        conn.execute(
            "INSERT OR REPLACE INTO bar_cache (key, data, timestamp, bar_count, last_ts, second_last_ts, zstd_level) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![key, compressed, timestamp, bar_count, last_ts, second_last_ts, zstd_level],
        ).map_err(|e| format!("SQLite insert failed: {e}"))?;
        Ok(true)
    }

    /// Load bar data — handles both binary (new) and JSON (legacy) formats.
    pub fn get_bars(&self, key: &str) -> Result<Option<(String, i64)>, String> {
        let conn = self
            .read_conn
            .lock()
            .map_err(|e| format!("Lock failed: {e}"))?;
        let mut stmt = conn
            .prepare_cached("SELECT data, timestamp FROM bar_cache WHERE key = ?1")
            .map_err(|e| format!("SQLite prepare failed: {e}"))?;

        let result = stmt.query_row(params![key], |row| {
            let data: Vec<u8> = row.get(0)?;
            let timestamp: i64 = row.get(1)?;
            Ok((data, timestamp))
        });

        match result {
            Ok((data, timestamp)) => {
                let decompressed = maybe_decompress(data)?;
                let json = if decompressed.len() >= 4 && &decompressed[0..4] == BAR_BINARY_MAGIC {
                    unpack_bars(&decompressed)?
                } else {
                    // Legacy JSON format — read as-is
                    String::from_utf8(decompressed)
                        .map_err(|e| format!("UTF-8 decode failed: {e}"))?
                };
                Ok(Some((json, timestamp)))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(format!("SQLite query failed: {e}")),
        }
    }

    /// Get bars as raw OHLCV tuples (no JSON serialization).
    /// Zero-serialization hot path for native GPU renderer: cache → f64 → GPU vertex buffer.
    pub fn get_bars_raw(
        &self,
        key: &str,
    ) -> Result<Option<Vec<(i64, f64, f64, f64, f64, f64)>>, String> {
        let conn = self
            .read_conn
            .lock()
            .map_err(|e| format!("Read lock failed: {e}"))?;
        let mut stmt = conn
            .prepare_cached("SELECT data FROM bar_cache WHERE key = ?1")
            .map_err(|e| format!("Prepare failed: {e}"))?;
        let row: Option<Vec<u8>> = stmt.query_row(rusqlite::params![key], |r| r.get(0)).ok();
        match row {
            None => Ok(None),
            Some(data) => {
                let bytes = maybe_decompress(data)?;
                if bytes.len() >= 4 && &bytes[0..4] == BAR_BINARY_MAGIC {
                    Ok(Some(unpack_bars_raw(&bytes)?))
                } else {
                    let text = String::from_utf8_lossy(&bytes);
                    let bars: Vec<serde_json::Value> = serde_json::from_str(&text)
                        .map_err(|e| format!("JSON parse failed: {e}"))?;
                    let result = bars
                        .iter()
                        .filter_map(|b| {
                            Some((
                                chrono::DateTime::parse_from_rfc3339(b["timestamp"].as_str()?)
                                    .ok()?
                                    .timestamp_millis(),
                                b["open"].as_f64()?,
                                b["high"].as_f64()?,
                                b["low"].as_f64()?,
                                b["close"].as_f64()?,
                                b["volume"].as_f64().unwrap_or(0.0),
                            ))
                        })
                        .collect();
                    Ok(Some(result))
                }
            }
        }
    }

    /// Get the last `tail` bars from cache — much faster than get_bars() when tail << total.
    /// For 500 bars from a 50K-bar cache: converts only 500 bars to JSON instead of 50K.
    /// Decompression overhead is unchanged (zstd doesn't support seeking).
    pub fn get_bars_tail(&self, key: &str, tail: usize) -> Result<Option<(String, i64)>, String> {
        let conn = self
            .read_conn
            .lock()
            .map_err(|e| format!("Read lock failed: {e}"))?;
        let mut stmt = conn
            .prepare_cached("SELECT data, timestamp FROM bar_cache WHERE key = ?1")
            .map_err(|e| format!("SQLite prepare failed: {e}"))?;

        let result = stmt.query_row(params![key], |row| {
            let data: Vec<u8> = row.get(0)?;
            let timestamp: i64 = row.get(1)?;
            Ok((data, timestamp))
        });

        match result {
            Ok((compressed, timestamp)) => {
                let decompressed = maybe_decompress(compressed)?;
                let json = if decompressed.len() >= 4 && &decompressed[0..4] == BAR_BINARY_MAGIC {
                    unpack_bars_tail(&decompressed, tail)?
                } else {
                    // Legacy JSON: parse, trim, reserialize
                    let text = String::from_utf8(decompressed)
                        .map_err(|e| format!("UTF-8 decode failed: {e}"))?;
                    let all: Vec<serde_json::Value> =
                        serde_json::from_str(&text).unwrap_or_default();
                    if tail > 0 && all.len() > tail {
                        serde_json::to_string(&all[all.len() - tail..])
                            .map_err(|e| format!("JSON error: {e}"))?
                    } else {
                        text
                    }
                };
                Ok(Some((json, timestamp)))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(format!("SQLite query failed: {e}")),
        }
    }

    /// Store key-value data (fundamentals, news, etc.).
    pub fn put_kv(&self, key: &str, json_data: &str) -> Result<(), String> {
        // Use the same live zstd setting as bar ingestion so every cache write
        // path — bars, news, SEC filings, fundamentals, broker queues, and
        // scraped metadata — obeys the UI zstd slider instead of silently
        // pinning KV data to a separate compression level.
        let zstd_level = bar_zstd_level();
        let compressed = zstd::encode_all(json_data.as_bytes(), zstd_level)
            .map_err(|e| format!("zstd compress failed: {e}"))?;
        let timestamp = chrono::Utc::now().timestamp();

        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        conn.execute(
            "INSERT OR REPLACE INTO kv_cache (key, value, timestamp) VALUES (?1, ?2, ?3)",
            params![key, compressed, timestamp],
        )
        .map_err(|e| format!("SQLite insert failed: {e}"))?;
        Ok(())
    }

    /// Load key-value data.
    pub fn get_kv(&self, key: &str) -> Result<Option<String>, String> {
        let conn = self
            .read_conn
            .lock()
            .map_err(|e| format!("Read lock failed: {e}"))?;
        let mut stmt = conn
            .prepare_cached("SELECT value FROM kv_cache WHERE key = ?1")
            .map_err(|e| format!("SQLite prepare failed: {e}"))?;

        let result = stmt.query_row(params![key], |row| {
            let data: Vec<u8> = row.get(0)?;
            Ok(data)
        });

        match result {
            Ok(compressed) => {
                let decompressed = zstd::decode_all(compressed.as_slice())
                    .map_err(|e| format!("zstd decompress failed: {e}"))?;
                let json = String::from_utf8(decompressed)
                    .map_err(|e| format!("UTF-8 decode failed: {e}"))?;
                Ok(Some(json))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(format!("SQLite query failed: {e}")),
        }
    }

    /// Append a single entry to a logical KV queue. Keys are generated as
    /// `{prefix}:{nanos}` — monotonic within a single process, unique across retries.
    /// O(1) append vs the previous read-modify-write pattern which was O(n^2) overall
    /// for rapid bursts (e.g. 9 FETCH_BARS requests arriving together).
    pub fn append_to_queue(&self, prefix: &str, entry_json: &str) -> Result<(), String> {
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let ts_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        let seq = SEQ.fetch_add(1, Ordering::Relaxed);
        let key = format!("{prefix}:{ts_ns:020}:{seq:08}");
        self.put_kv(&key, entry_json)
    }

    /// Drain all entries from a KV queue in key order (monotonic timestamp ascending).
    /// Returns the list of entry values and deletes them atomically within a single
    /// transaction. Consumers call this to process pending queue entries.
    pub fn drain_queue(&self, prefix: &str) -> Result<Vec<String>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        let like = format!("{prefix}:%");
        // Collect matching keys + values in order.
        let mut stmt = conn
            .prepare_cached("SELECT key, value FROM kv_cache WHERE key LIKE ?1 ORDER BY key")
            .map_err(|e| format!("Prepare failed: {e}"))?;
        let rows = stmt
            .query_map(params![like], |row| {
                let key: String = row.get(0)?;
                let data: Vec<u8> = row.get(1)?;
                Ok((key, data))
            })
            .map_err(|e| format!("Query failed: {e}"))?;

        let mut result: Vec<(String, String)> = Vec::new();
        for row in rows {
            if let Ok((k, compressed)) = row {
                if let Ok(decompressed) = zstd::decode_all(compressed.as_slice()) {
                    if let Ok(s) = String::from_utf8(decompressed) {
                        result.push((k, s));
                    }
                }
            }
        }
        drop(stmt);

        // Delete drained keys in a single chunked `DELETE ... WHERE key IN (...)`.
        // Was executing N per-row DELETEs inside one transaction — still O(N)
        // roundtrips to the SQLite engine. Bulk form cuts this to ~N/CHUNK calls.
        if !result.is_empty() {
            const CHUNK: usize = 512;
            let tx = conn
                .unchecked_transaction()
                .map_err(|e| format!("Transaction begin failed: {e}"))?;
            for chunk in result.chunks(CHUNK) {
                let placeholders = std::iter::repeat("?")
                    .take(chunk.len())
                    .collect::<Vec<_>>()
                    .join(",");
                let sql = format!("DELETE FROM kv_cache WHERE key IN ({placeholders})");
                let params_refs: Vec<&dyn rusqlite::types::ToSql> = chunk
                    .iter()
                    .map(|(k, _)| k as &dyn rusqlite::types::ToSql)
                    .collect();
                let _ = tx.execute(&sql, params_refs.as_slice());
            }
            tx.commit()
                .map_err(|e| format!("Transaction commit failed: {e}"))?;
        }

        Ok(result.into_iter().map(|(_, v)| v).collect())
    }

    /// Load the raw stored KV blob (skip zstd decompression + UTF-8 decode).
    /// Useful for inspecting the on-disk compressed form (e.g. compression-level
    /// checks) or cheap "is this key present?" probes without the decode overhead.
    pub fn get_kv_raw(&self, key: &str) -> Result<Option<(Vec<u8>, i64)>, String> {
        let conn = self
            .read_conn
            .lock()
            .map_err(|e| format!("Read lock failed: {e}"))?;
        let mut stmt = conn
            .prepare_cached("SELECT value, timestamp FROM kv_cache WHERE key = ?1")
            .map_err(|e| format!("SQLite prepare failed: {e}"))?;
        match stmt.query_row(params![key], |row| {
            let data: Vec<u8> = row.get(0)?;
            let ts: i64 = row.get(1)?;
            Ok((data, ts))
        }) {
            Ok(r) => Ok(Some(r)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(format!("SQLite query failed: {e}")),
        }
    }

    /// Delete entries older than max_age_secs.
    pub fn evict_old(&self, max_age_secs: i64) -> Result<u64, String> {
        let cutoff = chrono::Utc::now().timestamp() - max_age_secs;
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        let bars_deleted = conn
            .execute(
                "DELETE FROM bar_cache WHERE timestamp < ?1",
                params![cutoff],
            )
            .map_err(|e| format!("SQLite delete failed: {e}"))? as u64;
        let kv_deleted = conn
            .execute("DELETE FROM kv_cache WHERE timestamp < ?1", params![cutoff])
            .map_err(|e| format!("SQLite delete failed: {e}"))? as u64;
        Ok(bars_deleted + kv_deleted)
    }

    /// Get cache stats.
    pub fn stats(&self) -> Result<(i64, i64, i64), String> {
        let conn = self
            .read_conn
            .lock()
            .map_err(|e| format!("Read lock failed: {e}"))?;
        let bar_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM bar_cache", [], |r| r.get(0))
            .unwrap_or(0);
        // Internal migration markers (keys wrapped in "__") are not user-facing cache data.
        let kv_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM kv_cache WHERE key NOT LIKE '\\_\\_migration\\_\\_%' ESCAPE '\\'",
            [],
            |r| r.get(0),
        ).unwrap_or(0);
        // Report full on-disk footprint (main DB + WAL + SHM). Freed pages stay
        // allocated until VACUUM rebuilds the DB, so physical size is the user-visible metric.
        let file_size = Self::total_disk_usage_bytes(&self.db_path);
        Ok((bar_count, kv_count, file_size))
    }

    /// Get detailed per-key cache stats: returns JSON array of {key, compressed_bytes, timestamp}.
    /// Keys are "symbol:timeframe" format (e.g., "AAPL:1Hour").
    pub fn detailed_stats(&self) -> Result<Vec<(String, i64, i64)>, String> {
        let conn = self
            .read_conn
            .lock()
            .map_err(|e| format!("Read lock failed: {e}"))?;
        // Use bar_count instead of LENGTH(data) — avoids reading blob headers on 3.9GB DB
        let mut stmt = conn
            .prepare_cached("SELECT key, bar_count, timestamp FROM bar_cache ORDER BY key")
            .map_err(|e| format!("SQLite prepare failed: {e}"))?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })
            .map_err(|e| format!("SQLite query failed: {e}"))?;
        let mut result = Vec::new();
        for row in rows {
            if let Ok(r) = row {
                result.push(r);
            }
        }
        Ok(result)
    }

    /// Same as `detailed_stats` plus per-row blob byte size for the Storage
    /// Manager size column. `LENGTH(data)` on a BLOB is O(1) in SQLite — the
    /// payload length is recorded in the row header, so adding the column
    /// does not stream blob bodies off disk. Tuple order is (key, bar_count,
    /// timestamp, blob_bytes).
    pub fn detailed_stats_with_size(&self) -> Result<Vec<(String, i64, i64, i64)>, String> {
        let conn = self
            .read_conn
            .lock()
            .map_err(|e| format!("Read lock failed: {e}"))?;
        let mut stmt = conn
            .prepare_cached(
                "SELECT key, bar_count, timestamp, LENGTH(data) FROM bar_cache ORDER BY key",
            )
            .map_err(|e| format!("SQLite prepare failed: {e}"))?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })
            .map_err(|e| format!("SQLite query failed: {e}"))?;
        let mut result = Vec::new();
        for row in rows {
            if let Ok(r) = row {
                result.push(r);
            }
        }
        Ok(result)
    }

    /// Search cache keys by substring pattern. Uses SQL LIKE — avoids pulling the
    /// full bar_cache table into memory for partial-match fallbacks.
    ///
    /// `pattern` is matched case-insensitively against the key. Returns at most `limit`
    /// keys ordered by last-modified timestamp (most recent first).
    pub fn search_keys(&self, pattern: &str, limit: usize) -> Result<Vec<String>, String> {
        let conn = self
            .read_conn
            .lock()
            .map_err(|e| format!("Read lock failed: {e}"))?;
        let like_pattern = format!("%{}%", pattern);
        let mut stmt = conn.prepare_cached(
            "SELECT key FROM bar_cache WHERE LOWER(key) LIKE LOWER(?1) ORDER BY timestamp DESC LIMIT ?2"
        ).map_err(|e| format!("SQLite prepare failed: {e}"))?;
        let rows = stmt
            .query_map(params![like_pattern, limit as i64], |row| {
                row.get::<_, String>(0)
            })
            .map_err(|e| format!("SQLite query failed: {e}"))?;
        let mut result = Vec::new();
        for row in rows {
            if let Ok(k) = row {
                result.push(k);
            }
        }
        Ok(result)
    }

    /// Delete a specific cache entry by key.
    pub fn delete_key(&self, key: &str) -> Result<bool, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        let deleted = conn
            .execute("DELETE FROM bar_cache WHERE key = ?1", params![key])
            .map_err(|e| format!("SQLite delete failed: {e}"))?;
        Ok(deleted > 0)
    }

    /// Delete all bar-cache rows for one equity/xStock symbol across provider
    /// and merged prefixes, plus matching bar-track rows.
    ///
    /// Corporate actions are not append-only data. After a split/reverse split,
    /// providers often restate historical OHLC on a new adjusted scale; merging
    /// only the recent post-split candles leaves old pre-split cache rows intact
    /// forever. Use this when a new split is discovered so the next fetch/load is
    /// forced through a clean provider rebuild instead of timestamp-preserving
    /// incremental merge.
    pub fn delete_equity_bar_cache_for_symbol(&self, symbol: &str) -> Result<u64, String> {
        let trimmed = symbol.trim();
        if trimmed.is_empty() {
            return Ok(0);
        }
        let raw = trimmed.to_ascii_uppercase();
        let bare = raw
            .trim_end_matches(".EQ")
            .replace('/', "")
            .to_ascii_uppercase();
        if bare.is_empty() {
            return Ok(0);
        }

        let mut variants = Vec::new();
        for candidate in [raw.as_str(), bare.as_str()] {
            if !candidate.is_empty() && !variants.iter().any(|v: &String| v == candidate) {
                variants.push(candidate.to_string());
            }
        }
        let eq_variant = format!("{bare}.EQ");
        if !variants.iter().any(|v| v == &eq_variant) {
            variants.push(eq_variant);
        }

        let prefixes = [
            "merged",
            "kraken-equities",
            "alpaca",
            "yahoo-chart",
            "default",
        ];

        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        let mut deleted = 0u64;
        for prefix in prefixes {
            for variant in &variants {
                let pattern = format!("{prefix}:{variant}:%");
                deleted = deleted.saturating_add(
                    conn.execute(
                        "DELETE FROM bar_cache WHERE key LIKE ?1 COLLATE NOCASE",
                        params![pattern],
                    )
                    .map_err(|e| format!("delete bar_cache {pattern}: {e}"))?
                        as u64,
                );
                let _ = conn.execute(
                    "DELETE FROM bar_track WHERE key LIKE ?1 COLLATE NOCASE",
                    params![pattern],
                );
            }
        }
        Ok(deleted)
    }

    /// Delete a specific set of bar-cache keys in chunks, then reclaim freed pages.
    /// Intended for bulk filter deletes from the Storage Manager.
    pub fn delete_keys(&self, keys: &[String]) -> Result<u64, String> {
        if keys.is_empty() {
            return Ok(0);
        }
        let mut conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        let tx = conn
            .transaction()
            .map_err(|e| format!("SQLite transaction failed: {e}"))?;
        let mut deleted = 0u64;
        for chunk in keys.chunks(500) {
            let placeholders = std::iter::repeat("?")
                .take(chunk.len())
                .collect::<Vec<_>>()
                .join(",");
            let sql = format!("DELETE FROM bar_cache WHERE key IN ({placeholders})");
            deleted +=
                tx.execute(&sql, rusqlite::params_from_iter(chunk.iter()))
                    .map_err(|e| format!("SQLite bulk delete failed: {e}"))? as u64;
            let track_sql = format!("DELETE FROM bar_track WHERE key IN ({placeholders})");
            let _ = tx.execute(&track_sql, rusqlite::params_from_iter(chunk.iter()));
        }
        tx.commit()
            .map_err(|e| format!("SQLite commit failed: {e}"))?;
        match Self::reclaim_space_locked(&conn, &self.db_path) {
            Ok(_) => Ok(deleted),
            Err(e) => Err(format!(
                "Deleted {deleted} cache rows but reclaim failed: {e}"
            )),
        }
    }

    fn normalize_timeframe_suffix(tf: &str) -> Option<&'static str> {
        match tf {
            "M1" | "1Min" => Some("1Min"),
            "M5" | "5Min" => Some("5Min"),
            "M15" | "15Min" => Some("15Min"),
            "M30" | "30Min" => Some("30Min"),
            "H1" | "1Hour" => Some("1Hour"),
            "H4" | "4Hour" => Some("4Hour"),
            "D1" | "1Day" => Some("1Day"),
            "W1" | "1Week" => Some("1Week"),
            "MN1" | "1Month" => Some("1Month"),
            _ => None,
        }
    }

    /// Get the second-to-last bar's RFC3339 timestamp from a cached entry.
    /// Returns second-to-last (not last) because the last candle is still forming —
    /// its high/low/close/volume update until the period closes. We must always
    /// re-fetch it from the API to get the live values.
    /// Also returns the total bar count for logging.
    /// Returns None if key doesn't exist or has fewer than 2 bars.
    pub fn get_incremental_start(&self, key: &str) -> Result<Option<(String, usize)>, String> {
        let conn = self
            .read_conn
            .lock()
            .map_err(|e| format!("Lock failed: {e}"))?;

        // Fast path: read from metadata columns (no decompression needed)
        let mut stmt = conn
            .prepare_cached("SELECT bar_count, second_last_ts FROM bar_cache WHERE key = ?1")
            .map_err(|e| format!("SQLite prepare failed: {e}"))?;

        let result = stmt.query_row(rusqlite::params![key], |row| {
            let count: i64 = row.get(0)?;
            let second_last: Option<String> = row.get(1)?;
            Ok((count, second_last))
        });

        match result {
            Ok((count, second_last_ts)) => {
                if count < 2 {
                    return Ok(None);
                }
                // If metadata columns are populated, use them directly (zero decompression)
                if let Some(ts) = second_last_ts {
                    if !ts.is_empty() {
                        return Ok(Some((ts, count as usize)));
                    }
                }
                // Fallback: decompress for legacy entries without metadata columns
                let mut stmt2 = conn
                    .prepare_cached("SELECT data FROM bar_cache WHERE key = ?1")
                    .map_err(|e| format!("SQLite prepare failed: {e}"))?;
                let data: Vec<u8> = stmt2
                    .query_row(rusqlite::params![key], |row| row.get(0))
                    .map_err(|e| format!("SQLite query failed: {e}"))?;
                let decompressed = zstd::decode_all(data.as_slice())
                    .map_err(|e| format!("zstd decompress failed: {e}"))?;
                if decompressed.len() >= 8 && &decompressed[0..4] == BAR_BINARY_MAGIC {
                    let bc = u32::from_le_bytes(decompressed[4..8].try_into().unwrap_or([0; 4]))
                        as usize;
                    if bc < 2 {
                        return Ok(None);
                    }
                    let target_offset = match (bc - 2)
                        .checked_mul(BYTES_PER_BAR)
                        .and_then(|n| n.checked_add(8))
                    {
                        Some(n) => n,
                        None => return Ok(None),
                    };
                    if decompressed.len() < target_offset + 8 {
                        return Ok(None);
                    }
                    let ts_ms = i64::from_le_bytes(
                        decompressed[target_offset..target_offset + 8]
                            .try_into()
                            .unwrap_or([0; 8]),
                    );
                    let dt = chrono::DateTime::from_timestamp_millis(ts_ms).unwrap_or_default();
                    Ok(Some((dt.to_rfc3339(), bc)))
                } else {
                    let json_str = String::from_utf8(decompressed)
                        .map_err(|e| format!("UTF-8 decode failed: {e}"))?;
                    let bars: Vec<serde_json::Value> = serde_json::from_str(&json_str)
                        .map_err(|e| format!("JSON parse failed: {e}"))?;
                    if bars.len() < 2 {
                        return Ok(None);
                    }
                    let ts = bars[bars.len() - 2]["timestamp"]
                        .as_str()
                        .map(|s| s.to_string());
                    Ok(ts.map(|t| (t, bars.len())))
                }
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(format!("SQLite query failed: {e}")),
        }
    }

    /// Merge new bars into existing cached entry. Deduplicates by timestamp, sorts, re-stores.
    /// Trims to max_bars (keeps most recent) when a bounded caller explicitly asks for it.
    /// Uses the live-ingest zstd level for final bar storage. Idle compaction can
    /// recompress cold rows to zstd-22 without taxing foreground interaction.
    /// Returns the full merged dataset as JSON.
    pub fn merge_bars(&self, key: &str, new_json: &str, max_bars: usize) -> Result<String, String> {
        self.merge_bars_with_level(key, new_json, max_bars, bar_zstd_level())
    }

    /// Hot-path merge for high-frequency writers (Kraken WS bar close). Keeps
    /// the same zstd-3 level as live REST ingestion. Encoder is ~10–20× faster
    /// than zstd-22 which is the
    /// load-bearing fix for first-subscribe snapshot storms (~12k keys × ~700
    /// closed bars each landing in one flush): zstd-22 turns that into ~30s
    /// of CPU saturation, zstd-3 into ~3s. Compression ratio drops by ~15%
    /// (mid-50s KB → low-60s KB per blob) which is irrelevant on disk but
    /// massive on encode latency. The next REST refetch that touches this
    /// key can later be repacked at zstd-22 by idle compaction.
    pub fn merge_bars_fast(
        &self,
        key: &str,
        new_json: &str,
        max_bars: usize,
    ) -> Result<String, String> {
        self.merge_bars_with_level(key, new_json, max_bars, 3)
    }

    fn merge_bars_with_level(
        &self,
        key: &str,
        new_json: &str,
        max_bars: usize,
        zstd_level: i32,
    ) -> Result<String, String> {
        // Parse new bars
        let new_bars: Vec<serde_json::Value> =
            serde_json::from_str(new_json).map_err(|e| format!("JSON parse failed: {e}"))?;
        if new_bars.is_empty() {
            // Nothing to merge — return existing cache or empty
            return match self.get_bars(key)? {
                Some((json, _)) => Ok(json),
                None => Ok("[]".to_string()),
            };
        }

        // Load existing cache
        let mut all_bars: Vec<serde_json::Value> = match self.get_bars(key)? {
            Some((json, _)) => serde_json::from_str(&json).unwrap_or_default(),
            None => Vec::new(),
        };

        // Merge and deduplicate by timeframe-aware epoch bucket. D/W/M bars
        // from Yahoo/Alpaca/Kraken can represent the same candle at 00:00,
        // 04:00, 05:00, or a live-close timestamp; keep one canonical bucket
        // and let the newer incoming/refetched bar replace older cache content.
        all_bars.extend(new_bars);
        let mut keyed_bars: std::collections::BTreeMap<i64, serde_json::Value> =
            std::collections::BTreeMap::new();
        for mut bar in all_bars {
            let Some(ts_ms) = bar["timestamp"]
                .as_str()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .and_then(|dt| normalized_bar_timestamp_ms(key, dt.timestamp_millis()))
            else {
                continue;
            };
            if ts_ms <= 0 {
                continue;
            }
            if let Some(dt) = chrono::DateTime::from_timestamp_millis(ts_ms) {
                bar["timestamp"] = serde_json::Value::String(dt.to_rfc3339());
            }
            keyed_bars.insert(ts_ms, bar);
        }

        // Trim only when a bounded caller explicitly requests it. Full-depth sync passes 0.
        if max_bars > 0 && keyed_bars.len() > max_bars {
            let remove = keyed_bars.len() - max_bars;
            let stale_keys: Vec<i64> = keyed_bars.keys().copied().take(remove).collect();
            for stale_key in stale_keys {
                keyed_bars.remove(&stale_key);
            }
        }

        let all_bars: Vec<serde_json::Value> = keyed_bars.into_values().collect();
        let merged_json =
            serde_json::to_string(&all_bars).map_err(|e| format!("JSON serialize failed: {e}"))?;
        self.put_bars_with_level(key, &merged_json, zstd_level)?;

        Ok(merged_json)
    }

    /// Store bar data with caller-chosen zstd level. Encoder cost varies by
    /// 10–20× across zstd-3 → zstd-22; this lets WS hot writes pay the
    /// cheaper level while batch REST writes pay max compression.
    fn put_bars_with_level(
        &self,
        key: &str,
        json_data: &str,
        zstd_level: i32,
    ) -> Result<(), String> {
        let binary = pack_bars_for_key(key, json_data)?;
        let bar_count = u32::from_le_bytes(
            binary[4..8]
                .try_into()
                .map_err(|_| "bar_count header slice failed in put_bars_with_level")?,
        ) as i64;
        let (second_last_ts, last_ts) = get_last_two_bar_timestamps(&binary, bar_count as usize);
        let compressed = zstd::encode_all(binary.as_slice(), zstd_level)
            .map_err(|e| format!("zstd compress failed: {e}"))?;
        let timestamp = chrono::Utc::now().timestamp();

        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        conn.execute(
            "INSERT OR REPLACE INTO bar_cache (key, data, timestamp, bar_count, last_ts, second_last_ts, zstd_level) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![key, compressed, timestamp, bar_count, last_ts, second_last_ts, zstd_level],
        ).map_err(|e| format!("SQLite insert failed: {e}"))?;
        Ok(())
    }

    /// Get cache timestamp (when bars were last stored) for a key.
    /// Returns None if key doesn't exist.
    pub fn get_cache_age_secs(&self, key: &str) -> Result<Option<i64>, String> {
        let conn = self
            .read_conn
            .lock()
            .map_err(|e| format!("Lock failed: {e}"))?;
        let mut stmt = conn
            .prepare_cached("SELECT timestamp FROM bar_cache WHERE key = ?1")
            .map_err(|e| format!("SQLite prepare failed: {e}"))?;

        match stmt.query_row(rusqlite::params![key], |row| row.get::<_, i64>(0)) {
            Ok(ts) => Ok(Some(chrono::Utc::now().timestamp() - ts)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(format!("SQLite query failed: {e}")),
        }
    }

    /// Get bar count for a cache entry. Returns None if key doesn't exist.
    pub fn get_bar_count(&self, key: &str) -> Result<Option<i64>, String> {
        let conn = self
            .read_conn
            .lock()
            .map_err(|e| format!("Lock failed: {e}"))?;
        let mut stmt = conn
            .prepare_cached("SELECT bar_count FROM bar_cache WHERE key = ?1")
            .map_err(|e| format!("SQLite prepare failed: {e}"))?;

        match stmt.query_row(rusqlite::params![key], |row| row.get::<_, i64>(0)) {
            Ok(count) => Ok(Some(count)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(format!("SQLite query failed: {e}")),
        }
    }

    /// Batch-write pre-compressed bar entries in a single transaction.
    /// Takes (key, compressed_data, bar_count) tuples — compression done by caller.
    pub fn put_compressed_batch(
        &self,
        entries: &[(String, Vec<u8>, i64)],
    ) -> Result<usize, String> {
        if entries.is_empty() {
            return Ok(0);
        }
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        conn.execute_batch("BEGIN")
            .map_err(|e| format!("BEGIN failed: {e}"))?;
        let timestamp = chrono::Utc::now().timestamp();
        let mut count = 0;
        for (key, compressed, bar_count) in entries {
            match conn.execute(
                "INSERT OR REPLACE INTO bar_cache (key, data, timestamp, bar_count, zstd_level) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![key, compressed, timestamp, bar_count, bar_zstd_level()],
            ) {
                Ok(_) => count += 1,
                Err(e) => tracing::warn!("Batch write skip {}: {}", key, e),
            }
        }
        conn.execute_batch("COMMIT")
            .map_err(|e| format!("COMMIT failed: {e}"))?;
        Ok(count)
    }

    /// Bulk-load cache metadata for entries updated since `since_ts`.
    /// Returns Vec<(key, timestamp, bar_count)> — only changed entries.
    pub fn get_cache_meta_since(&self, since_ts: i64) -> Result<Vec<(String, i64, i64)>, String> {
        let conn = self
            .read_conn
            .lock()
            .map_err(|e| format!("Read lock failed: {e}"))?;
        let mut stmt = conn
            .prepare("SELECT key, timestamp, bar_count FROM bar_cache WHERE timestamp > ?1")
            .map_err(|e| format!("SQLite prepare failed: {e}"))?;
        let mut result = Vec::new();
        let rows = stmt
            .query_map(rusqlite::params![since_ts], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })
            .map_err(|e| format!("SQLite query failed: {e}"))?;
        for row in rows {
            if let Ok(entry) = row {
                result.push(entry);
            }
        }
        Ok(result)
    }

    /// Bulk-load cache metadata (age_secs, bar_count) for all entries.
    /// Returns HashMap<key, (age_secs, bar_count)> — one query instead of N individual lookups.
    pub fn get_all_cache_meta(
        &self,
    ) -> Result<std::collections::HashMap<String, (i64, i64)>, String> {
        let conn = self
            .read_conn
            .lock()
            .map_err(|e| format!("Read lock failed: {e}"))?;
        let mut stmt = conn
            .prepare("SELECT key, timestamp, bar_count FROM bar_cache")
            .map_err(|e| format!("SQLite prepare failed: {e}"))?;
        let now = chrono::Utc::now().timestamp();
        let mut map = std::collections::HashMap::new();
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })
            .map_err(|e| format!("SQLite query failed: {e}"))?;
        for row in rows {
            if let Ok((key, ts, bc)) = row {
                map.insert(key, (now - ts, bc));
            }
        }
        Ok(map)
    }

    fn compressed_backup_bytes(&self, path: &str) -> Result<Vec<u8>, String> {
        // Use SQLite's VACUUM INTO to create a consistent snapshot.
        // Use a unique temp file name to avoid TOCTOU races with concurrent exports.
        let backup_path = format!("{}.tmp.{}", path, std::process::id());
        // Remove any stale leftover from a previous crash
        let _ = std::fs::remove_file(&backup_path);

        // Hold write lock ONLY for VACUUM INTO — release before file I/O + compression
        {
            let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
            conn.execute("VACUUM INTO ?1", [&backup_path])
                .map_err(|e| {
                    let _ = std::fs::remove_file(&backup_path);
                    format!("VACUUM INTO failed: {e}")
                })?;
        } // lock released here

        // File I/O + maximum compression without holding any lock
        let data = std::fs::read(&backup_path).map_err(|e| {
            let _ = std::fs::remove_file(&backup_path);
            format!("Read backup failed: {e}")
        })?;
        let _ = std::fs::remove_file(&backup_path);
        zstd::encode_all(data.as_slice(), BACKUP_ZSTD_LEVEL)
            .map_err(|e| format!("Compress failed: {e}"))
    }

    /// Export entire cache to a compressed backup file.
    /// Format: zstd-compressed copy of the SQLite database file (via VACUUM INTO).
    pub fn export_backup(&self, path: &str) -> Result<String, String> {
        let compressed = self.compressed_backup_bytes(path)?;
        std::fs::write(path, &compressed).map_err(|e| format!("Write backup failed: {e}"))?;

        let size_mb = compressed.len() as f64 / 1_048_576.0;
        Ok(format!(
            "{{\"size_bytes\":{},\"size_mb\":{:.1}}}",
            compressed.len(),
            size_mb
        ))
    }

    /// Export entire cache to a password-encrypted backup file.
    /// Format: TyphooN AES-256-GCM envelope containing the zstd-compressed SQLite snapshot.
    pub fn export_backup_encrypted(&self, path: &str, passphrase: &str) -> Result<String, String> {
        let compressed = self.compressed_backup_bytes(path)?;
        let encrypted = encrypt_backup_payload(&compressed, passphrase)?;
        std::fs::write(path, &encrypted).map_err(|e| format!("Write backup failed: {e}"))?;

        let size_mb = encrypted.len() as f64 / 1_048_576.0;
        Ok(format!(
            "{{\"size_bytes\":{},\"size_mb\":{:.1},\"encrypted\":true}}",
            encrypted.len(),
            size_mb
        ))
    }

    fn import_compressed_backup_bytes(
        &self,
        path: &str,
        compressed: &[u8],
    ) -> Result<String, String> {
        let data = zstd::decode_all(compressed).map_err(|e| format!("Decompress failed: {e}"))?;

        // Write to temp file with exclusive creation to avoid TOCTOU races
        let tmp_path = format!("{}.import.tmp.{}", path, std::process::id());
        {
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&tmp_path)
                .map_err(|e| format!("Create temp file failed (may already exist): {e}"))?;
            f.write_all(&data).map_err(|e| {
                let _ = std::fs::remove_file(&tmp_path);
                format!("Write temp failed: {e}")
            })?;
        }

        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;

        // Attach the backup DB
        conn.execute("ATTACH DATABASE ?1 AS backup_db", [&tmp_path])
            .map_err(|e| {
                let _ = std::fs::remove_file(&tmp_path);
                format!("Attach failed: {e}")
            })?;

        // Merge bar_cache: import entries where backup has newer timestamp or key doesn't exist
        let bar_count = conn
            .execute(
                "INSERT OR REPLACE INTO bar_cache (key, data, timestamp, bar_count, zstd_level)
             SELECT b.key, b.data, b.timestamp, b.bar_count, COALESCE(b.zstd_level, 3)
             FROM backup_db.bar_cache b
             LEFT JOIN main.bar_cache c ON c.key = b.key
             WHERE c.key IS NULL OR b.timestamp > c.timestamp",
                [],
            )
            .map_err(|e| {
                let _ = conn.execute("DETACH DATABASE backup_db", []);
                let _ = std::fs::remove_file(&tmp_path);
                format!("Merge bar_cache failed: {e}")
            })?;

        // Merge kv_cache: same newer-wins strategy
        let kv_count = conn
            .execute(
                "INSERT OR REPLACE INTO kv_cache (key, value, timestamp)
             SELECT b.key, b.value, b.timestamp
             FROM backup_db.kv_cache b
             LEFT JOIN main.kv_cache c ON c.key = b.key
             WHERE c.key IS NULL OR b.timestamp > c.timestamp",
                [],
            )
            .map_err(|e| {
                let _ = conn.execute("DETACH DATABASE backup_db", []);
                let _ = std::fs::remove_file(&tmp_path);
                format!("Merge kv_cache failed: {e}")
            })?;

        conn.execute("DETACH DATABASE backup_db", [])
            .map_err(|e| format!("Detach failed: {e}"))?;

        let _ = std::fs::remove_file(&tmp_path);

        Ok(format!(
            "{{\"bars_imported\":{},\"kv_imported\":{}}}",
            bar_count, kv_count
        ))
    }

    /// Import cache from a compressed backup file. Merges with existing data (newer wins).
    pub fn import_backup(&self, path: &str) -> Result<String, String> {
        let compressed = std::fs::read(path).map_err(|e| format!("Read backup failed: {e}"))?;
        self.import_compressed_backup_bytes(path, &compressed)
    }

    /// Import cache from a password-encrypted backup file. Merges with existing data (newer wins).
    pub fn import_backup_encrypted(&self, path: &str, passphrase: &str) -> Result<String, String> {
        let encrypted = std::fs::read(path).map_err(|e| format!("Read backup failed: {e}"))?;
        let compressed = decrypt_backup_payload(&encrypted, passphrase)?;
        self.import_compressed_backup_bytes(path, &compressed)
    }

    /// Detect whether a backup file uses TyphooN's encrypted backup envelope.
    pub fn backup_file_is_encrypted(path: &str) -> Result<bool, String> {
        let data = std::fs::read(path).map_err(|e| format!("Read backup failed: {e}"))?;
        Ok(data.starts_with(ENCRYPTED_BACKUP_MAGIC))
    }

    /// Get the database file path.
    pub fn db_path(&self) -> &PathBuf {
        &self.db_path
    }

    /// Get first and last bar timestamps for a key using a caller-provided connection.
    /// Used by the BG thread with its own connection to avoid read_conn contention.
    pub fn get_bar_timestamp_range_with_conn(conn: &Connection, key: &str) -> Option<(i64, i64)> {
        // prepare_cached → repeated calls (BG thread iterates all crypto entries every cycle)
        // reuse the same parsed statement instead of reparsing on every call.
        let mut stmt = conn
            .prepare_cached("SELECT data FROM bar_cache WHERE key = ?1")
            .ok()?;
        let blob: Vec<u8> = stmt
            .query_row(params![key], |row| match row.get_ref(0)? {
                rusqlite::types::ValueRef::Blob(b) => Ok(b.to_vec()),
                rusqlite::types::ValueRef::Text(t) => Ok(t.to_vec()),
                _ => Err(rusqlite::Error::InvalidColumnType(
                    0,
                    "data".into(),
                    rusqlite::types::Type::Blob,
                )),
            })
            .ok()?;
        let decompressed = maybe_decompress(blob).ok()?;
        if decompressed.len() >= 8 && &decompressed[0..4] == BAR_BINARY_MAGIC {
            let count = u32::from_le_bytes(decompressed[4..8].try_into().ok()?) as usize;
            if count == 0 || decompressed.len() < 8 + count * BYTES_PER_BAR {
                return None;
            }
            let first_ts = i64::from_le_bytes(decompressed[8..16].try_into().ok()?);
            let last_off = 8 + (count - 1) * BYTES_PER_BAR;
            let last_ts = i64::from_le_bytes(decompressed[last_off..last_off + 8].try_into().ok()?);
            return Some((first_ts, last_ts));
        }

        // Legacy JSON rows can still exist in upgraded caches. Preserve first/last
        // bar visibility instead of treating them as timestamp-less blobs.
        let bars: Vec<serde_json::Value> = serde_json::from_slice(&decompressed).ok()?;
        let first_ts =
            chrono::DateTime::parse_from_rfc3339(bars.first()?.get("timestamp")?.as_str()?)
                .ok()?
                .timestamp_millis();
        let last_ts =
            chrono::DateTime::parse_from_rfc3339(bars.last()?.get("timestamp")?.as_str()?)
                .ok()?
                .timestamp_millis();
        Some((first_ts, last_ts))
    }

    /// Get a lock on the underlying connection for direct SQL operations.
    /// Used for direct table creation and batch inserts.
    pub fn connection(&self) -> Result<std::sync::MutexGuard<'_, Connection>, String> {
        self.conn.lock().map_err(|e| format!("Lock failed: {e}"))
    }

    /// Get a read-only connection for queries that don't mutate.
    /// Uses the dedicated read connection — never blocked by write operations.
    pub fn read_connection(&self) -> Result<std::sync::MutexGuard<'_, Connection>, String> {
        self.read_conn
            .lock()
            .map_err(|e| format!("Read lock failed: {e}"))
    }

    /// Try to get a read connection without blocking. Returns None if the read_conn lock is held.
    /// Used from UI thread to avoid any chance of freezing.
    pub fn try_connection(&self) -> Option<std::sync::MutexGuard<'_, Connection>> {
        self.read_conn.try_lock().ok()
    }

    /// Open an independent read-only Connection to the same database file.
    /// The caller owns this connection — it is NOT shared via any Mutex.
    /// Use this for long-running background threads that need to read without
    /// contending with the UI thread's read_conn or the write conn.
    pub fn open_bg_read_connection(&self) -> Result<Connection, String> {
        let conn = Connection::open_with_flags(
            &self.db_path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|e| format!("BG read conn open failed: {e}"))?;
        conn.busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|e| format!("BG read conn busy_timeout failed: {e}"))?;
        let _ = conn.execute_batch(
            "
            PRAGMA cache_size=-32000;
            PRAGMA temp_store=MEMORY;
            PRAGMA mmap_size=268435456;
        ",
        );
        Ok(conn)
    }

    /// Non-blocking version of get_bars_raw. Returns Ok(None) if lock is contended.
    pub fn try_get_bars_raw(
        &self,
        key: &str,
    ) -> Result<Option<Vec<(i64, f64, f64, f64, f64, f64)>>, String> {
        let conn = match self.read_conn.try_lock() {
            Ok(c) => c,
            Err(_) => return Ok(None), // lock contended — skip this frame
        };
        let mut stmt = conn
            .prepare_cached("SELECT data FROM bar_cache WHERE key = ?1")
            .map_err(|e| format!("Prepare failed: {e}"))?;
        let row: Option<Vec<u8>> = stmt.query_row(params![key], |r| r.get(0)).ok();
        match row {
            None => Ok(None),
            Some(data) => {
                let bytes = maybe_decompress(data)?;
                if bytes.len() >= 4 && &bytes[0..4] == BAR_BINARY_MAGIC {
                    Ok(Some(unpack_bars_raw(&bytes)?))
                } else {
                    let text = String::from_utf8_lossy(&bytes);
                    let bars: Vec<serde_json::Value> = serde_json::from_str(&text)
                        .map_err(|e| format!("JSON parse failed: {e}"))?;
                    let result = bars
                        .iter()
                        .filter_map(|b| {
                            Some((
                                chrono::DateTime::parse_from_rfc3339(b["timestamp"].as_str()?)
                                    .ok()?
                                    .timestamp_millis(),
                                b["open"].as_f64()?,
                                b["high"].as_f64()?,
                                b["low"].as_f64()?,
                                b["close"].as_f64()?,
                                b["volume"].as_f64().unwrap_or(0.0),
                            ))
                        })
                        .collect();
                    Ok(Some(result))
                }
            }
        }
    }

    /// List all kv_cache keys matching a prefix (e.g., "cred:" returns all credential keys).
    /// LIKE wildcards in the prefix are escaped to prevent overly broad matches.
    pub fn list_kv_keys(&self, prefix: &str) -> Result<Vec<String>, String> {
        let conn = self
            .read_conn
            .lock()
            .map_err(|e| format!("Lock failed: {e}"))?;
        let escaped = prefix.replace('%', "\\%").replace('_', "\\_");
        let pattern = format!("{}%", escaped);
        let mut stmt = conn
            .prepare("SELECT key FROM kv_cache WHERE key LIKE ?1 ESCAPE '\\'")
            .map_err(|e| format!("SQLite prepare failed: {e}"))?;
        let rows = stmt
            .query_map(params![pattern], |row| row.get::<_, String>(0))
            .map_err(|e| format!("SQLite query failed: {e}"))?;
        let mut keys = Vec::new();
        for row in rows {
            if let Ok(k) = row {
                keys.push(k);
            }
        }
        Ok(keys)
    }

    /// Delete all cache entries matching a symbol prefix (e.g., "AAPL:" deletes all TFs for AAPL).
    pub fn delete_symbol(&self, symbol_prefix: &str) -> Result<u64, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        // Escape LIKE wildcards in prefix to prevent overly broad deletion
        let escaped = symbol_prefix.replace('%', "\\%").replace('_', "\\_");
        let pattern = format!("{}:%", escaped);
        let deleted = conn
            .execute(
                "DELETE FROM bar_cache WHERE key LIKE ?1 ESCAPE '\\'",
                params![pattern],
            )
            .map_err(|e| format!("SQLite delete failed: {e}"))? as u64;
        Ok(deleted)
    }

    /// Delete all bar entries matching a timeframe suffix across every broker.
    /// Example: `1Min` removes `kraken:BTCUSD:1Min`, `alpaca:AAPL:1Min`, etc.
    pub fn delete_timeframe(&self, timeframe_suffix: &str) -> Result<u64, String> {
        let Some(tf) = Self::normalize_timeframe_suffix(timeframe_suffix) else {
            return Err(format!("Unknown timeframe: {}", timeframe_suffix));
        };
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        let pattern = format!("%:{}", tf);
        let deleted = conn
            .execute(
                "DELETE FROM bar_cache WHERE key LIKE ?1 ESCAPE '\\'",
                params![pattern],
            )
            .map_err(|e| format!("SQLite delete failed: {e}"))? as u64;
        let _ = conn.execute(
            "DELETE FROM bar_track WHERE key LIKE ?1 ESCAPE '\\'",
            params![pattern],
        );
        match Self::reclaim_space_locked(&conn, &self.db_path) {
            Ok(_) => Ok(deleted),
            Err(e) => Err(format!(
                "Deleted {deleted} timeframe rows but reclaim failed: {e}"
            )),
        }
    }

    /// Delete ALL bar data from the cache. Returns the number of rows deleted.
    /// Runs VACUUM to reclaim freed pages and shrink the DB file on disk.
    pub fn delete_all_bars(&self) -> Result<u64, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        let deleted = conn
            .execute("DELETE FROM bar_cache", [])
            .map_err(|e| format!("SQLite delete failed: {e}"))? as u64;
        let _ = conn.execute("DELETE FROM bar_track", []);
        match Self::reclaim_space_locked(&conn, &self.db_path) {
            Ok(_) => Ok(deleted),
            Err(e) => Err(format!(
                "Deleted {deleted} bar rows but reclaim failed: {e}"
            )),
        }
    }

    /// Delete all cache data for one supported broker prefix and reclaim freed pages.
    /// Applies to bar_cache, kv_cache, and bar_track keys with the broker prefix.
    pub fn delete_broker_data(&self, broker_prefix: &str) -> Result<u64, String> {
        let broker = match broker_prefix.to_ascii_lowercase().as_str() {
            "alpaca" => "alpaca",
            other => return Err(format!("Unsupported broker purge target: {other}")),
        };
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        let pattern = format!("{}:%", broker);
        let bars_deleted =
            conn.execute(
                "DELETE FROM bar_cache WHERE key LIKE ?1 ESCAPE '\\'",
                params![pattern],
            )
            .map_err(|e| format!("SQLite bar delete failed: {e}"))? as u64;
        let kv_deleted = conn
            .execute(
                "DELETE FROM kv_cache WHERE key LIKE ?1 ESCAPE '\\'",
                params![pattern],
            )
            .map_err(|e| format!("SQLite KV delete failed: {e}"))? as u64;
        let _ = conn.execute(
            "DELETE FROM bar_track WHERE key LIKE ?1 ESCAPE '\\'",
            params![pattern],
        );
        let total = bars_deleted + kv_deleted;
        match Self::reclaim_space_locked(&conn, &self.db_path) {
            Ok(_) => Ok(total),
            Err(e) => Err(format!(
                "Deleted {total} {broker} cache rows but reclaim failed: {e}"
            )),
        }
    }

    /// Force a WAL checkpoint + VACUUM cycle to reclaim free pages after prior deletes.
    pub fn reclaim_space(&self) -> Result<(i64, i64), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        Self::reclaim_space_locked(&conn, &self.db_path)
    }

    /// Delete all kraken-equities bar cache + track rows for the given
    /// timeframe suffixes (e.g. `&["1Min", "5Min"]`). Used by the one-shot
    /// startup migration that drops illusory M1/M5 bars from the iapi
    /// 15-min-delayed equity feed. Returns `(rows_deleted, bytes_freed)`.
    pub fn delete_kraken_equity_bars_by_tf(
        &self,
        timeframe_suffixes: &[&str],
    ) -> Result<(u64, i64), String> {
        if timeframe_suffixes.is_empty() {
            return Ok((0, 0));
        }
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        let mut deleted = 0u64;
        for tf in timeframe_suffixes {
            // Pattern: kraken-equities:<symbol>:<TF>. The middle %
            // matches the symbol; ESCAPE '\' keeps the ':' literal.
            let pattern = format!("kraken-equities:%:{}", tf);
            let bars = conn
                .execute(
                    "DELETE FROM bar_cache WHERE key LIKE ?1 ESCAPE '\\'",
                    params![pattern],
                )
                .map_err(|e| format!("delete kraken-equities bars failed for tf {tf}: {e}"))?
                as u64;
            let _ = conn.execute(
                "DELETE FROM bar_track WHERE key LIKE ?1 ESCAPE '\\'",
                params![pattern],
            );
            deleted = deleted.saturating_add(bars);
        }
        let (before, after) = Self::reclaim_space_locked(&conn, &self.db_path)
            .map_err(|e| format!("Deleted {deleted} rows but reclaim failed: {e}"))?;
        Ok((deleted, (before - after).max(0)))
    }

    /// Delete provider-assist M1/M5 rows that are not valid broad merge/cache
    /// targets. Kraken Spot and Kraken Equities/xStocks low-TF rows are preserved.
    /// Returns `(rows_deleted, bytes_freed)`.
    pub fn delete_non_spot_low_timeframe_bars(&self) -> Result<(u64, i64), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        let deleted = Self::purge_obsolete_low_tf_provider_bars_locked(&conn)? as u64;
        let (before, after) = Self::reclaim_space_locked(&conn, &self.db_path)
            .map_err(|e| format!("Deleted {deleted} rows but reclaim failed: {e}"))?;
        Ok((deleted, (before - after).max(0)))
    }

    /// Bound the news corpus: drop articles older than `cutoff_ts` and then cap
    /// the table at `max_rows`. Runs on the write connection from the background
    /// maintenance loop so `research_news` (and its FTS mirror) stays small no
    /// matter how many full-universe news scrapes the user runs — keeping the
    /// header COUNT and FTS search cheap and the on-disk footprint bounded.
    /// Returns `(purged_by_age, purged_by_cap)`.
    pub fn enforce_news_retention(
        &self,
        cutoff_ts: i64,
        max_rows: i64,
    ) -> Result<(usize, usize), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        let by_age = crate::core::news::purge_older_than(&conn, cutoff_ts)?;
        let by_cap = crate::core::news::enforce_max_rows(&conn, max_rows)?;
        Ok((by_age, by_cap))
    }

    /// Run PRAGMA incremental_vacuum to reclaim freed pages without full VACUUM.
    /// Lighter than VACUUM — doesn't rewrite the entire DB, just reclaims free pages.
    /// Safe to call periodically (e.g., on shutdown, after compaction).
    pub fn incremental_vacuum(&self, pages: i64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        conn.execute(&format!("PRAGMA incremental_vacuum({})", pages), [])
            .map_err(|e| format!("incremental_vacuum failed: {e}"))?;
        Ok(())
    }

    /// Delete every `source:SYMBOL:tf` bar entry for the given source and the
    /// listed timeframes. Used to reclaim orphaned KVs after a lane stops syncing
    /// a timeframe (e.g. the Yahoo chart lane dropping intraday — those rows are
    /// no longer fetched, merged, or counted, so they are dead weight). Keys are
    /// `source:SYMBOL:tf` and symbols never contain a colon, so the anchored
    /// `LIKE 'source:%:tf'` cannot match a different source or timeframe. Returns
    /// the number of rows deleted across all timeframes.
    pub fn purge_bars_for_source_timeframes(
        &self,
        source: &str,
        timeframes: &[&str],
    ) -> Result<usize, String> {
        // Delete in bounded rowid chunks, releasing the write lock between each, so
        // a large purge (tens of thousands of rows) never holds the single conn
        // mutex long enough to stall a render-thread cache read — the same hazard
        // the streaming compaction avoids. Symbols never contain a colon, so the
        // anchored `LIKE 'source:%:tf'` can't match a different source/timeframe.
        const CHUNK: i64 = 500;
        let mut deleted = 0usize;
        for tf in timeframes {
            let pattern = format!("{source}:%:{tf}");
            loop {
                let n = {
                    let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
                    conn.execute(
                        "DELETE FROM bar_cache WHERE rowid IN \
                         (SELECT rowid FROM bar_cache WHERE key LIKE ?1 LIMIT ?2)",
                        params![pattern, CHUNK],
                    )
                    .map_err(|e| format!("purge {source}:*:{tf} failed: {e}"))?
                }; // lock released each iteration so other readers/writers interleave
                deleted += n;
                if (n as i64) < CHUNK {
                    break;
                }
            }
        }
        Ok(deleted)
    }

    /// Count bar_cache rows below a target zstd level. Used by the auto-compact
    /// scheduler to decide whether a recompression pass is worth waking up for —
    /// already-compacted entries are skipped by `compact_storage`, so the work
    /// budget is bounded by this count.
    pub fn count_uncompacted_bars(&self, target: i32) -> Result<i64, String> {
        let conn = self
            .read_conn
            .lock()
            .map_err(|e| format!("Read lock failed: {e}"))?;
        conn.query_row(
            "SELECT COUNT(*) FROM bar_cache WHERE zstd_level < ?1",
            params![target],
            |r| r.get::<_, i64>(0),
        )
        .map_err(|e| format!("count_uncompacted_bars failed: {e}"))
    }

    /// Scan bar_cache for entries with bar_count=0 and repair from TTBR header.
    /// Earlier versions may have left stale 0 values. Returns
    /// number of entries repaired.
    pub fn repair_bar_counts(&self) -> Result<usize, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        let mut stmt = conn
            .prepare("SELECT key, data FROM bar_cache WHERE bar_count = 0 OR bar_count IS NULL")
            .map_err(|e| format!("Prepare failed: {e}"))?;
        let mut updates: Vec<(String, i64)> = Vec::new();
        let rows = stmt
            .query_map([], |row| {
                let key: String = row.get(0)?;
                let data: Vec<u8> = row.get(1)?;
                Ok((key, data))
            })
            .map_err(|e| format!("Query failed: {e}"))?;
        for row in rows {
            if let Ok((key, data)) = row {
                // Skip metadata rows — they follow `<prefix>:__<NAME>__[…]`
                // and aren't bar blobs.
                if key.contains(":__") {
                    continue;
                }
                let bytes = match maybe_decompress(data) {
                    Ok(b) => b,
                    Err(e) => {
                        tracing::warn!("repair_bar_counts: decompress failed for {key}: {e}");
                        continue;
                    }
                };
                if bytes.len() >= 8 && &bytes[0..4] == BAR_BINARY_MAGIC {
                    let count = u32::from_le_bytes(bytes[4..8].try_into().unwrap_or([0; 4])) as i64;
                    if count > 0 {
                        updates.push((key, count));
                    }
                }
            }
        }
        drop(stmt);
        let count = updates.len();
        for (key, bar_count) in &updates {
            let _ = conn.execute(
                "UPDATE bar_cache SET bar_count = ?1 WHERE key = ?2",
                params![bar_count, key],
            );
        }
        Ok(count)
    }

    /// LRU eviction: if total bar_cache size exceeds `max_bytes`, delete oldest entries
    /// (by timestamp ASC) until under the limit. Skips entries newer than 7 days to avoid
    /// evicting hot data. Returns (evicted_count, bytes_freed).
    pub fn evict_lru(&self, max_bytes: i64) -> Result<(usize, i64), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        let total: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(LENGTH(data)), 0) FROM bar_cache",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if total <= max_bytes {
            return Ok((0, 0));
        }
        let cutoff_ts = chrono::Utc::now().timestamp() - 7 * 86400; // 7 days
        // Select oldest entries (excluding hot ones)
        let mut stmt = conn.prepare(
            "SELECT key, LENGTH(data) FROM bar_cache WHERE timestamp < ?1 ORDER BY timestamp ASC"
        ).map_err(|e| format!("Prepare evict failed: {e}"))?;
        let rows: Vec<(String, i64)> = stmt
            .query_map([cutoff_ts], |r| Ok((r.get(0)?, r.get(1)?)))
            .map_err(|e| format!("Query evict failed: {e}"))?
            .filter_map(|r| r.ok())
            .collect();
        drop(stmt);
        // Collect keys to delete up to target_free, then issue a single bulk DELETE.
        // Single-statement bulk DELETE is ~100× faster than per-row roundtrips when
        // the eviction batch is large.
        let target_free = total - max_bytes;
        let mut freed: i64 = 0;
        let mut keys_to_delete: Vec<String> = Vec::new();
        for (key, size) in rows {
            if freed >= target_free {
                break;
            }
            keys_to_delete.push(key);
            freed += size;
        }
        let evicted = keys_to_delete.len();
        if !keys_to_delete.is_empty() {
            // Chunked to stay within SQLITE_MAX_VARIABLE_NUMBER (32766 in modern sqlite)
            const CHUNK: usize = 512;
            for chunk in keys_to_delete.chunks(CHUNK) {
                let placeholders = std::iter::repeat("?")
                    .take(chunk.len())
                    .collect::<Vec<_>>()
                    .join(",");
                let sql = format!("DELETE FROM bar_cache WHERE key IN ({placeholders})");
                let params_refs: Vec<&dyn rusqlite::types::ToSql> = chunk
                    .iter()
                    .map(|k| k as &dyn rusqlite::types::ToSql)
                    .collect();
                conn.execute(&sql, params_refs.as_slice())
                    .map_err(|e| format!("Bulk evict delete failed: {e}"))?;
            }
        }
        Ok((evicted, freed))
    }

    /// Recompress all bar_cache entries at target zstd level (e.g. 19 for max compression).
    /// Decompression speed is identical regardless of compression level — only storage shrinks.
    /// Returns (entries_processed, bytes_saved).
    /// Progress callback: (processed, total, key, old_size, new_size)
    pub fn compact_storage(
        &self,
        level: i32,
        progress: Option<&dyn Fn(usize, usize, &str, usize, usize)>,
    ) -> Result<(usize, i64), String> {
        // Streaming, memory-bounded compaction.
        //
        // The earlier design loaded *every* uncompacted blob into one in-memory
        // Vec (phase 1) and recompressed them all at once (phase 2). On a multi-GB
        // cache that produced gigabyte RSS swings, and the recompression — though
        // it held no lock — starved the egui thread via allocator/page pressure for
        // the whole run (200ms+ frame stalls). Process in small key-cursor chunks
        // instead: read a window on the read connection, recompress off-lock, write
        // it back under a brief write lock, then advance the cursor. Peak memory is
        // O(READ_CHUNK), and the write lock is released between every chunk so
        // foreground sync writes and chart reads interleave.
        const READ_CHUNK: i64 = 256;

        // Totals up front for the progress bar (cheap COUNTs on the read conn).
        let total = {
            let conn = self
                .read_conn
                .lock()
                .map_err(|e| format!("Read lock failed: {e}"))?;
            let bar_total: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM bar_cache WHERE zstd_level < ?1",
                    params![level],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            let kv_total: i64 = conn
                .query_row("SELECT COUNT(*) FROM kv_cache", [], |r| r.get(0))
                .unwrap_or(0);
            (bar_total + kv_total).max(0) as usize
        };
        let mut processed = 0usize;
        let mut bytes_saved = 0i64;

        // ---- Bars: cursor by key over rows still below the target level ----
        // Rows updated to `level` drop out of the `zstd_level < ?` filter; the
        // cursor advances by key regardless, so each key is examined at most once
        // (O(n) total index walk, never O(n²)).
        let mut cursor = String::new();
        loop {
            let batch: Vec<(String, Vec<u8>)> = {
                let conn = self
                    .read_conn
                    .lock()
                    .map_err(|e| format!("Read lock failed: {e}"))?;
                let mut stmt = conn
                    .prepare(
                        "SELECT key, data FROM bar_cache \
                         WHERE zstd_level < ?1 AND key > ?2 ORDER BY key LIMIT ?3",
                    )
                    .map_err(|e| format!("Prepare failed: {e}"))?;
                stmt.query_map(params![level, cursor, READ_CHUNK], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
                })
                .map_err(|e| format!("Query failed: {e}"))?
                .filter_map(|r| r.ok())
                .collect()
            };
            let Some((last_key, _)) = batch.last() else {
                break;
            };
            cursor = last_key.clone();

            // Recompress off-lock — the slow part stays out of the critical section.
            let mut updates: Vec<(String, Vec<u8>)> = Vec::new();
            for (key, compressed) in &batch {
                let Ok(decompressed) = zstd::decode_all(compressed.as_slice()) else {
                    processed += 1;
                    continue;
                };
                let Ok(recompressed) = zstd::encode_all(decompressed.as_slice(), level) else {
                    processed += 1;
                    continue;
                };
                let saved = compressed.len() as i64 - recompressed.len() as i64;
                let after_len = if saved > 0 {
                    recompressed.len()
                } else {
                    compressed.len()
                };
                if saved > 0 {
                    bytes_saved += saved;
                    updates.push((key.clone(), recompressed));
                }
                processed += 1;
                if let Some(cb) = progress {
                    cb(processed, total, key, compressed.len(), after_len);
                }
            }

            // Write this window back under a brief write lock.
            if !updates.is_empty() {
                let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
                let _ = conn.execute_batch("BEGIN;");
                for (key, data) in &updates {
                    let _ = conn.execute(
                        "UPDATE bar_cache SET data = ?1, zstd_level = ?2 WHERE key = ?3",
                        params![data, level, key],
                    );
                }
                let _ = conn.execute_batch("COMMIT;");
            }
        }

        // ---- KV: cursor by key over every row (kv_cache has no level column) ----
        let mut cursor = String::new();
        loop {
            let batch: Vec<(String, Vec<u8>)> = {
                let conn = self
                    .read_conn
                    .lock()
                    .map_err(|e| format!("Read lock failed: {e}"))?;
                let mut stmt = conn
                    .prepare("SELECT key, value FROM kv_cache WHERE key > ?1 ORDER BY key LIMIT ?2")
                    .map_err(|e| format!("Prepare failed: {e}"))?;
                stmt.query_map(params![cursor, READ_CHUNK], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
                })
                .map_err(|e| format!("Query failed: {e}"))?
                .filter_map(|r| r.ok())
                .collect()
            };
            let Some((last_key, _)) = batch.last() else {
                break;
            };
            cursor = last_key.clone();

            let mut updates: Vec<(String, Vec<u8>)> = Vec::new();
            for (key, compressed) in &batch {
                if let Ok(decompressed) = zstd::decode_all(compressed.as_slice()) {
                    if let Ok(recompressed) = zstd::encode_all(decompressed.as_slice(), level) {
                        let saved = compressed.len() as i64 - recompressed.len() as i64;
                        if saved > 0 {
                            bytes_saved += saved;
                            updates.push((key.clone(), recompressed));
                        }
                    }
                }
                processed += 1;
                if let Some(cb) = progress {
                    cb(processed, total, key, compressed.len(), compressed.len());
                }
            }

            if !updates.is_empty() {
                let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
                let _ = conn.execute_batch("BEGIN;");
                for (key, data) in &updates {
                    let _ = conn.execute(
                        "UPDATE kv_cache SET value = ?1 WHERE key = ?2",
                        params![data, key],
                    );
                }
                let _ = conn.execute_batch("COMMIT;");
            }
        }

        // Reclaim the pages freed by the rewrites. auto_vacuum=INCREMENTAL is set on
        // the connection, so a bounded incremental vacuum reclaims compaction's freed
        // pages without the multi-minute exclusive file rewrite a full VACUUM costs on
        // a large cache (and without needing a second full copy of the DB on disk).
        {
            let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
            let _ = conn.execute_batch("PRAGMA incremental_vacuum;");
        }

        Ok((processed, bytes_saved))
    }
}

#[cfg(test)]
mod tests;
