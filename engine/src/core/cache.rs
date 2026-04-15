//! SQLite-backed cache for unlimited structured storage.
//!
//! Replaces IndexedDB's ~50MB limit with SQLite (no practical limit).
//! Bar data uses packed binary format (44 bytes/bar) + zstd compression.
//! KV data uses JSON + zstd compression.
//! Binary format: [u32 bar_count][per bar: i64 timestamp_ms, f64 OHLCV]

use rusqlite::{Connection, OpenFlags, params};
use std::path::PathBuf;
use std::sync::Mutex;

/// Re-export rusqlite::Connection so callers can use BG connections without depending on rusqlite directly.
pub type BgConnection = Connection;

/// Magic bytes to identify binary bar format (vs legacy JSON).
const BAR_BINARY_MAGIC: &[u8; 4] = b"TTBR"; // TyphooN Terminal Bar Record
/// Bytes per bar in binary format: i64 timestamp + 5×f64 (OHLCV) = 48 bytes
const BYTES_PER_BAR: usize = 8 + 5 * 8; // 48

/// Decompress bar data if needed. BarCacheWriter stores raw TTBR (magic "TTBR" at byte 0).
/// Rust put_bars() stores zstd-compressed (magic 0x28B52FFD). This function handles both.
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
fn pack_bars(json_data: &str) -> Result<Vec<u8>, String> {
    let bars: Vec<serde_json::Value> = serde_json::from_str(json_data)
        .map_err(|e| format!("JSON parse failed: {e}"))?;
    let mut buf = Vec::with_capacity(4 + 4 + bars.len() * BYTES_PER_BAR);
    buf.extend_from_slice(BAR_BINARY_MAGIC);
    // Reserve the count slot; overwrite once we know how many bars survived.
    buf.extend_from_slice(&0u32.to_le_bytes());
    let mut kept: u32 = 0;
    for bar in &bars {
        let ts_str = bar["timestamp"].as_str().unwrap_or("");
        let ts_ms = match chrono::DateTime::parse_from_rfc3339(ts_str) {
            Ok(dt) => dt.timestamp_millis(),
            Err(_) => continue,
        };
        if ts_ms <= 0 { continue; }
        let o = bar["open"].as_f64().unwrap_or(0.0);
        let h = bar["high"].as_f64().unwrap_or(0.0);
        let l = bar["low"].as_f64().unwrap_or(0.0);
        let c = bar["close"].as_f64().unwrap_or(0.0);
        let v = bar["volume"].as_f64().unwrap_or(0.0);
        if !(o > 0.0 && h > 0.0 && l > 0.0 && c > 0.0) { continue; }
        if !(o.is_finite() && h.is_finite() && l.is_finite() && c.is_finite() && v.is_finite()) { continue; }
        if h < l { continue; }
        buf.extend_from_slice(&ts_ms.to_le_bytes());
        buf.extend_from_slice(&o.to_le_bytes());
        buf.extend_from_slice(&h.to_le_bytes());
        buf.extend_from_slice(&l.to_le_bytes());
        buf.extend_from_slice(&c.to_le_bytes());
        buf.extend_from_slice(&v.to_le_bytes());
        kept += 1;
    }
    buf[4..8].copy_from_slice(&kept.to_le_bytes());
    Ok(buf)
}

/// Unpack binary bars back to JSON string for frontend consumption.
fn unpack_bars(data: &[u8]) -> Result<String, String> {
    if data.len() < 8 || &data[0..4] != BAR_BINARY_MAGIC {
        return Err("Not binary bar format".into());
    }
    let count = u32::from_le_bytes(
        data[4..8].try_into().map_err(|_| "Failed to read bar_count from binary header")?
    ) as usize;
    let expected = count.checked_mul(BYTES_PER_BAR)
        .and_then(|n| n.checked_add(8))
        .ok_or("Integer overflow computing bar data size")?;
    if data.len() < expected {
        return Err(format!("Binary data truncated: expected {expected}, got {}", data.len()));
    }

    let mut bars = Vec::with_capacity(count);
    for i in 0..count {
        let offset = 8 + i * BYTES_PER_BAR;
        let ts_ms = i64::from_le_bytes(
            data[offset..offset+8].try_into().map_err(|_| format!("Bad timestamp at bar {i}"))?
        );
        let open = f64::from_le_bytes(
            data[offset+8..offset+16].try_into().map_err(|_| format!("Bad open at bar {i}"))?
        );
        let high = f64::from_le_bytes(
            data[offset+16..offset+24].try_into().map_err(|_| format!("Bad high at bar {i}"))?
        );
        let low = f64::from_le_bytes(
            data[offset+24..offset+32].try_into().map_err(|_| format!("Bad low at bar {i}"))?
        );
        let close = f64::from_le_bytes(
            data[offset+32..offset+40].try_into().map_err(|_| format!("Bad close at bar {i}"))?
        );
        let volume = f64::from_le_bytes(
            data[offset+40..offset+48].try_into().map_err(|_| format!("Bad volume at bar {i}"))?
        );

        // Convert epoch ms back to RFC3339 timestamp
        let dt = chrono::DateTime::from_timestamp_millis(ts_ms)
            .unwrap_or_default();
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
        data[4..8].try_into().map_err(|_| "Failed to read bar_count")?
    ) as usize;
    let expected = count.checked_mul(BYTES_PER_BAR)
        .and_then(|n| n.checked_add(8))
        .ok_or("Integer overflow computing bar data size")?;
    if data.len() < expected {
        return Err(format!("Binary data truncated: expected {expected}, got {}", data.len()));
    }
    let mut bars = Vec::with_capacity(count);
    for i in 0..count {
        let off = 8 + i * BYTES_PER_BAR;
        // Bounds already validated above (data.len() >= expected), but use get() for defense in depth.
        let sl = data.get(off..off + BYTES_PER_BAR).ok_or("Bar data slice out of bounds")?;
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
    let required = match count.checked_mul(BYTES_PER_BAR).and_then(|n| n.checked_add(8)) {
        Some(n) => n,
        None => return (None, None),
    };
    if count < 2 || binary.len() < required {
        return (None, None);
    }
    let last_offset = 8 + (count - 1) * BYTES_PER_BAR;
    let second_offset = 8 + (count - 2) * BYTES_PER_BAR;
    let last_ts = i64::from_le_bytes(binary[last_offset..last_offset+8].try_into().unwrap_or([0;8]));
    let second_ts = i64::from_le_bytes(binary[second_offset..second_offset+8].try_into().unwrap_or([0;8]));
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
        data[4..8].try_into().map_err(|_| "Failed to read bar_count from binary header")?
    ) as usize;
    let expected = count.checked_mul(BYTES_PER_BAR)
        .and_then(|n| n.checked_add(8))
        .ok_or("Integer overflow computing bar data size")?;
    if data.len() < expected {
        return Err(format!("Binary data truncated: expected {expected}, got {}", data.len()));
    }
    if tail == 0 || tail >= count {
        return unpack_bars(data); // no trimming needed
    }

    let start_bar = count - tail;
    let mut bars = Vec::with_capacity(tail);
    for i in start_bar..count {
        let offset = 8 + i * BYTES_PER_BAR;
        let ts_ms = i64::from_le_bytes(
            data[offset..offset+8].try_into().map_err(|_| format!("Bad timestamp at bar {i}"))?
        );
        let open = f64::from_le_bytes(data[offset+8..offset+16].try_into().map_err(|_| format!("Bad open at bar {i}"))?);
        let high = f64::from_le_bytes(data[offset+16..offset+24].try_into().map_err(|_| format!("Bad high at bar {i}"))?);
        let low = f64::from_le_bytes(data[offset+24..offset+32].try_into().map_err(|_| format!("Bad low at bar {i}"))?);
        let close = f64::from_le_bytes(data[offset+32..offset+40].try_into().map_err(|_| format!("Bad close at bar {i}"))?);
        let volume = f64::from_le_bytes(data[offset+40..offset+48].try_into().map_err(|_| format!("Bad volume at bar {i}"))?);
        let dt = chrono::DateTime::from_timestamp_millis(ts_ms).unwrap_or_default();
        bars.push(serde_json::json!({
            "timestamp": dt.to_rfc3339(),
            "open": open, "high": high, "low": low, "close": close, "volume": volume,
        }));
    }
    serde_json::to_string(&bars).map_err(|e| format!("JSON serialize failed: {e}"))
}

/// Thread-safe SQLite cache manager.
///
/// Uses two connections for concurrency under WAL mode:
/// - `conn` (Mutex): exclusive write path — put_bars, put_kv, delete, compact, etc.
///   Also used by the bg thread for DARWIN queries (which do CREATE TABLE IF NOT EXISTS).
/// - `read_conn` (Mutex): dedicated read path — get_bars_raw, detailed_stats, stats, etc.
///   Never blocked by writes. The bg thread and UI thread can read simultaneously with
///   the write connection held by Mt5Sync or compaction.
///
/// SQLite WAL mode allows unlimited concurrent readers + one writer. The two Mutexes
/// are independent — a write lock on `conn` does NOT block reads on `read_conn`.
pub struct SqliteCache {
    conn: Mutex<Connection>,
    read_conn: Mutex<Connection>,
    db_path: PathBuf,
}

impl SqliteCache {
    /// Open or create a SQLite database at the given path.
    pub fn open(path: &PathBuf) -> Result<Self, String> {
        let conn = Connection::open(path)
            .map_err(|e| format!("SQLite open failed: {e}"))?;

        // WAL mode for concurrent reads + single writer performance.
        // This is used for the main typhoon_cache.db which is accessed only by
        // TyphooN-Terminal (Linux native). WAL shared memory works fine here.
        // busy_timeout=5000ms: retry for 5s on SQLITE_BUSY instead of failing
        // immediately. Critical when compact_storage() holds the write lock in
        // batches and other threads (e.g. Mt5Sync) need to write concurrently.
        conn.execute_batch("
            PRAGMA journal_mode=WAL;
            PRAGMA synchronous=NORMAL;
            PRAGMA cache_size=-64000;
            PRAGMA temp_store=MEMORY;
            PRAGMA mmap_size=268435456;
            PRAGMA auto_vacuum=INCREMENTAL;
            PRAGMA wal_autocheckpoint=2000;
            PRAGMA busy_timeout=5000;
        ").map_err(|e| format!("SQLite pragma failed: {e}"))?;

        // Create tables
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS bar_cache (
                key TEXT PRIMARY KEY,
                data BLOB NOT NULL,
                timestamp INTEGER NOT NULL,
                bar_count INTEGER NOT NULL DEFAULT 0,
                zstd_level INTEGER NOT NULL DEFAULT 3
            );
            CREATE TABLE IF NOT EXISTS kv_cache (
                key TEXT PRIMARY KEY,
                value BLOB NOT NULL,
                timestamp INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_bar_cache_ts ON bar_cache(timestamp);
            CREATE INDEX IF NOT EXISTS idx_bar_meta ON bar_cache(key, timestamp, bar_count);
            CREATE INDEX IF NOT EXISTS idx_kv_cache_ts ON kv_cache(timestamp);
            CREATE TABLE IF NOT EXISTS sync_state (
                key TEXT PRIMARY KEY,
                last_sync_ts INTEGER NOT NULL DEFAULT 0
            );
        ").map_err(|e| format!("SQLite create tables failed: {e}"))?;

        // Schema migration: add last_ts column for fast incremental start lookup
        // (avoids decompressing full binary blob just to read 2 timestamps)
        let _ = conn.execute("ALTER TABLE bar_cache ADD COLUMN last_ts TEXT", []);
        let _ = conn.execute("ALTER TABLE bar_cache ADD COLUMN second_last_ts TEXT", []);
        // Schema migration: track zstd compression level per entry (compact skips already-compacted)
        let _ = conn.execute("ALTER TABLE bar_cache ADD COLUMN zstd_level INTEGER NOT NULL DEFAULT 3", []);

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
            let purged = conn.execute(
                "DELETE FROM bar_cache WHERE key LIKE 'alpaca:%' AND key NOT LIKE 'alpaca:%/%'",
                [],
            ).unwrap_or(0);
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

        // Open a second read-only connection for the read path.
        // WAL mode allows this to read concurrently while conn writes.
        let read_conn = Connection::open_with_flags(path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)
            .map_err(|e| format!("SQLite read conn open failed: {e}"))?;
        read_conn.busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|e| format!("SQLite read conn busy_timeout failed: {e}"))?;
        // Align read_conn cache_size with write conn (-64000 = 64MB) so the
        // shared page cache is effective on hot reads. Previously -32000 (32MB)
        // which undersized the buffer pool for mixed read/write workloads.
        let _ = read_conn.execute_batch("
            PRAGMA cache_size=-64000;
            PRAGMA temp_store=MEMORY;
            PRAGMA mmap_size=268435456;
        ");

        Ok(Self { conn: Mutex::new(conn), read_conn: Mutex::new(read_conn), db_path: path.clone() })
    }

    /// Open an existing database read-only — for reading source MT5 cache files.
    ///
    /// Does NOT change journal_mode (avoids conflicting with BarCacheWriter which
    /// uses DELETE mode on the same file). Read-only mode means SQLite never needs
    /// a write lock, so BarCacheWriter can continue writing concurrently without
    /// any "database is locked" errors.
    pub fn open_readonly(path: &PathBuf) -> Result<Self, String> {
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)
            .map_err(|e| format!("SQLite read-only open failed: {e}"))?;
        // busy_timeout MUST be set first — BarCacheWriter uses DELETE journal mode
        // (WAL doesn't work across Wine/Linux boundary) which takes an exclusive lock
        // during write transactions. Without busy_timeout, all reads fail instantly
        // with SQLITE_BUSY when BarCacheWriter is mid-transaction.
        // Use rusqlite's built-in method (doesn't require a DB lock to execute).
        conn.busy_timeout(std::time::Duration::from_secs(10))
            .map_err(|e| format!("SQLite busy_timeout failed: {e}"))?;
        // Non-critical optimizations — ignore failures (DB may be locked briefly)
        let _ = conn.execute_batch("
            PRAGMA cache_size=-16000;
            PRAGMA temp_store=MEMORY;
        ");
        // Read-only source: use the same connection for both read and write paths
        // (open_readonly is only used for reading MT5 source files, no concurrent access)
        let read_conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)
            .map_err(|e| format!("SQLite read conn open failed: {e}"))?;
        read_conn.execute_batch("
            PRAGMA cache_size=-16000;
            PRAGMA temp_store=MEMORY;
            PRAGMA busy_timeout=5000;
        ").map_err(|e| format!("SQLite read conn pragma failed: {e}"))?;
        Ok(Self { conn: Mutex::new(conn), read_conn: Mutex::new(read_conn), db_path: path.clone() })
    }

    /// Store bar data in packed binary format + zstd compression.
    /// Binary format is ~3-5x smaller than JSON before compression.
    /// Uses zstd level 3 — same as put_bars_fast. Level 9 was wasteful since
    /// merge_bars recompresses anyway, and backup export uses level 9 for archival.
    pub fn put_bars(&self, key: &str, json_data: &str) -> Result<(), String> {
        let binary = pack_bars(json_data)?;
        let bar_count = u32::from_le_bytes(
            binary[4..8].try_into().map_err(|_| "bar_count header slice failed")?
        ) as i64;
        let (second_last_ts, last_ts) = get_last_two_bar_timestamps(&binary, bar_count as usize);
        let zstd_level = 3i32;
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

    /// Load bar data — handles both binary (new) and JSON (legacy) formats.
    pub fn get_bars(&self, key: &str) -> Result<Option<(String, i64)>, String> {
        let conn = self.read_conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        let mut stmt = conn.prepare_cached(
            "SELECT data, timestamp FROM bar_cache WHERE key = ?1"
        ).map_err(|e| format!("SQLite prepare failed: {e}"))?;

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
    pub fn get_bars_raw(&self, key: &str) -> Result<Option<Vec<(i64, f64, f64, f64, f64, f64)>>, String> {
        let conn = self.read_conn.lock().map_err(|e| format!("Read lock failed: {e}"))?;
        let mut stmt = conn.prepare_cached("SELECT data FROM bar_cache WHERE key = ?1")
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
                    let result = bars.iter().filter_map(|b| {
                        Some((
                            chrono::DateTime::parse_from_rfc3339(b["timestamp"].as_str()?).ok()?.timestamp_millis(),
                            b["open"].as_f64()?, b["high"].as_f64()?, b["low"].as_f64()?,
                            b["close"].as_f64()?, b["volume"].as_f64().unwrap_or(0.0),
                        ))
                    }).collect();
                    Ok(Some(result))
                }
            }
        }
    }

    /// Get the last `tail` bars from cache — much faster than get_bars() when tail << total.
    /// For 500 bars from a 50K-bar cache: converts only 500 bars to JSON instead of 50K.
    /// Decompression overhead is unchanged (zstd doesn't support seeking).
    pub fn get_bars_tail(&self, key: &str, tail: usize) -> Result<Option<(String, i64)>, String> {
        let conn = self.read_conn.lock().map_err(|e| format!("Read lock failed: {e}"))?;
        let mut stmt = conn.prepare_cached(
            "SELECT data, timestamp FROM bar_cache WHERE key = ?1"
        ).map_err(|e| format!("SQLite prepare failed: {e}"))?;

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
                    let all: Vec<serde_json::Value> = serde_json::from_str(&text).unwrap_or_default();
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
        let compressed = zstd::encode_all(json_data.as_bytes(), 9)
            .map_err(|e| format!("zstd compress failed: {e}"))?;
        let timestamp = chrono::Utc::now().timestamp();

        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        conn.execute(
            "INSERT OR REPLACE INTO kv_cache (key, value, timestamp) VALUES (?1, ?2, ?3)",
            params![key, compressed, timestamp],
        ).map_err(|e| format!("SQLite insert failed: {e}"))?;
        Ok(())
    }

    /// Store a pre-compressed KV blob directly (skip re-compression).
    /// Used by LAN sync to avoid decompress-on-server + recompress-on-client overhead.
    pub fn put_kv_compressed(&self, key: &str, compressed: &[u8]) -> Result<(), String> {
        let timestamp = chrono::Utc::now().timestamp();
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        conn.execute(
            "INSERT OR REPLACE INTO kv_cache (key, value, timestamp) VALUES (?1, ?2, ?3)",
            params![key, compressed, timestamp],
        ).map_err(|e| format!("SQLite insert failed: {e}"))?;
        Ok(())
    }

    /// Load key-value data.
    pub fn get_kv(&self, key: &str) -> Result<Option<String>, String> {
        let conn = self.read_conn.lock().map_err(|e| format!("Read lock failed: {e}"))?;
        let mut stmt = conn.prepare_cached(
            "SELECT value FROM kv_cache WHERE key = ?1"
        ).map_err(|e| format!("SQLite prepare failed: {e}"))?;

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
        let mut stmt = conn.prepare_cached(
            "SELECT key, value FROM kv_cache WHERE key LIKE ?1 ORDER BY key"
        ).map_err(|e| format!("Prepare failed: {e}"))?;
        let rows = stmt.query_map(params![like], |row| {
            let key: String = row.get(0)?;
            let data: Vec<u8> = row.get(1)?;
            Ok((key, data))
        }).map_err(|e| format!("Query failed: {e}"))?;

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
            let tx = conn.unchecked_transaction()
                .map_err(|e| format!("Transaction begin failed: {e}"))?;
            for chunk in result.chunks(CHUNK) {
                let placeholders = std::iter::repeat("?").take(chunk.len()).collect::<Vec<_>>().join(",");
                let sql = format!("DELETE FROM kv_cache WHERE key IN ({placeholders})");
                let params_refs: Vec<&dyn rusqlite::types::ToSql> = chunk.iter()
                    .map(|(k, _)| k as &dyn rusqlite::types::ToSql)
                    .collect();
                let _ = tx.execute(&sql, params_refs.as_slice());
            }
            tx.commit().map_err(|e| format!("Transaction commit failed: {e}"))?;
        }

        Ok(result.into_iter().map(|(_, v)| v).collect())
    }

    /// Load raw compressed KV blob (skip zstd decompression + UTF-8 decode).
    /// Used by the LAN sync pass-through path: the client decompresses on its end,
    /// so the server should not pay the decompression cost. Also useful for
    /// "is this key present?" probes without the decode overhead.
    pub fn get_kv_raw(&self, key: &str) -> Result<Option<(Vec<u8>, i64)>, String> {
        let conn = self.read_conn.lock().map_err(|e| format!("Read lock failed: {e}"))?;
        let mut stmt = conn.prepare_cached(
            "SELECT value, timestamp FROM kv_cache WHERE key = ?1"
        ).map_err(|e| format!("SQLite prepare failed: {e}"))?;
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
        let bars_deleted = conn.execute(
            "DELETE FROM bar_cache WHERE timestamp < ?1", params![cutoff]
        ).map_err(|e| format!("SQLite delete failed: {e}"))? as u64;
        let kv_deleted = conn.execute(
            "DELETE FROM kv_cache WHERE timestamp < ?1", params![cutoff]
        ).map_err(|e| format!("SQLite delete failed: {e}"))? as u64;
        Ok(bars_deleted + kv_deleted)
    }

    /// Get cache stats.
    pub fn stats(&self) -> Result<(i64, i64, i64), String> {
        let conn = self.read_conn.lock().map_err(|e| format!("Read lock failed: {e}"))?;
        let bar_count: i64 = conn.query_row("SELECT COUNT(*) FROM bar_cache", [], |r| r.get(0))
            .unwrap_or(0);
        // Internal migration markers (keys wrapped in "__") are not user-facing cache data.
        let kv_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM kv_cache WHERE key NOT LIKE '\\_\\_migration\\_\\_%' ESCAPE '\\'",
            [],
            |r| r.get(0),
        ).unwrap_or(0);
        // Report actual file size on disk (includes freed pages from DELETEs).
        // SUM(LENGTH(data)) only counts live data — misleading after purge operations
        // where the file stays large until VACUUM reclaims freed pages.
        let file_size = std::fs::metadata(&self.db_path)
            .map(|m| m.len() as i64)
            .unwrap_or(0);
        Ok((bar_count, kv_count, file_size))
    }

    /// Get detailed per-key cache stats: returns JSON array of {key, compressed_bytes, timestamp}.
    /// Keys are "symbol:timeframe" format (e.g., "AAPL:1Hour").
    pub fn detailed_stats(&self) -> Result<Vec<(String, i64, i64)>, String> {
        let conn = self.read_conn.lock().map_err(|e| format!("Read lock failed: {e}"))?;
        // Use bar_count instead of LENGTH(data) — avoids reading blob headers on 3.9GB DB
        let mut stmt = conn.prepare_cached(
            "SELECT key, bar_count, timestamp FROM bar_cache ORDER BY key"
        ).map_err(|e| format!("SQLite prepare failed: {e}"))?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
            ))
        }).map_err(|e| format!("SQLite query failed: {e}"))?;
        let mut result = Vec::new();
        for row in rows {
            if let Ok(r) = row { result.push(r); }
        }
        Ok(result)
    }

    /// Search cache keys by substring pattern. Uses SQL LIKE — avoids pulling the
    /// full bar_cache table into memory for partial-match fallbacks.
    ///
    /// `pattern` is matched case-insensitively against the key. Returns at most `limit`
    /// keys ordered by last-modified timestamp (most recent first).
    pub fn search_keys(&self, pattern: &str, limit: usize) -> Result<Vec<String>, String> {
        let conn = self.read_conn.lock().map_err(|e| format!("Read lock failed: {e}"))?;
        let like_pattern = format!("%{}%", pattern);
        let mut stmt = conn.prepare_cached(
            "SELECT key FROM bar_cache WHERE LOWER(key) LIKE LOWER(?1) ORDER BY timestamp DESC LIMIT ?2"
        ).map_err(|e| format!("SQLite prepare failed: {e}"))?;
        let rows = stmt.query_map(params![like_pattern, limit as i64], |row| row.get::<_, String>(0))
            .map_err(|e| format!("SQLite query failed: {e}"))?;
        let mut result = Vec::new();
        for row in rows {
            if let Ok(k) = row { result.push(k); }
        }
        Ok(result)
    }

    /// Delete a specific cache entry by key.
    pub fn delete_key(&self, key: &str) -> Result<bool, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        let deleted = conn.execute(
            "DELETE FROM bar_cache WHERE key = ?1", params![key]
        ).map_err(|e| format!("SQLite delete failed: {e}"))?;
        Ok(deleted > 0)
    }

    /// Get the second-to-last bar's RFC3339 timestamp from a cached entry.
    /// Returns second-to-last (not last) because the last candle is still forming —
    /// its high/low/close/volume update until the period closes. We must always
    /// re-fetch it from the API to get the live values.
    /// Also returns the total bar count for logging.
    /// Returns None if key doesn't exist or has fewer than 2 bars.
    pub fn get_incremental_start(&self, key: &str) -> Result<Option<(String, usize)>, String> {
        let conn = self.read_conn.lock().map_err(|e| format!("Lock failed: {e}"))?;

        // Fast path: read from metadata columns (no decompression needed)
        let mut stmt = conn.prepare_cached(
            "SELECT bar_count, second_last_ts FROM bar_cache WHERE key = ?1"
        ).map_err(|e| format!("SQLite prepare failed: {e}"))?;

        let result = stmt.query_row(rusqlite::params![key], |row| {
            let count: i64 = row.get(0)?;
            let second_last: Option<String> = row.get(1)?;
            Ok((count, second_last))
        });

        match result {
            Ok((count, second_last_ts)) => {
                if count < 2 { return Ok(None); }
                // If metadata columns are populated, use them directly (zero decompression)
                if let Some(ts) = second_last_ts {
                    if !ts.is_empty() {
                        return Ok(Some((ts, count as usize)));
                    }
                }
                // Fallback: decompress for legacy entries without metadata columns
                let mut stmt2 = conn.prepare_cached(
                    "SELECT data FROM bar_cache WHERE key = ?1"
                ).map_err(|e| format!("SQLite prepare failed: {e}"))?;
                let data: Vec<u8> = stmt2.query_row(rusqlite::params![key], |row| row.get(0))
                    .map_err(|e| format!("SQLite query failed: {e}"))?;
                let decompressed = zstd::decode_all(data.as_slice())
                    .map_err(|e| format!("zstd decompress failed: {e}"))?;
                if decompressed.len() >= 8 && &decompressed[0..4] == BAR_BINARY_MAGIC {
                    let bc = u32::from_le_bytes(
                        decompressed[4..8].try_into().unwrap_or([0;4])
                    ) as usize;
                    if bc < 2 { return Ok(None); }
                    let target_offset = match (bc - 2).checked_mul(BYTES_PER_BAR).and_then(|n| n.checked_add(8)) {
                        Some(n) => n,
                        None => return Ok(None),
                    };
                    if decompressed.len() < target_offset + 8 { return Ok(None); }
                    let ts_ms = i64::from_le_bytes(
                        decompressed[target_offset..target_offset+8].try_into().unwrap_or([0;8])
                    );
                    let dt = chrono::DateTime::from_timestamp_millis(ts_ms).unwrap_or_default();
                    Ok(Some((dt.to_rfc3339(), bc)))
                } else {
                    let json_str = String::from_utf8(decompressed)
                        .map_err(|e| format!("UTF-8 decode failed: {e}"))?;
                    let bars: Vec<serde_json::Value> = serde_json::from_str(&json_str)
                        .map_err(|e| format!("JSON parse failed: {e}"))?;
                    if bars.len() < 2 { return Ok(None); }
                    let ts = bars[bars.len() - 2]["timestamp"].as_str().map(|s| s.to_string());
                    Ok(ts.map(|t| (t, bars.len())))
                }
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(format!("SQLite query failed: {e}")),
        }
    }

    /// Merge new bars into existing cached entry. Deduplicates by timestamp, sorts, re-stores.
    /// Trims to max_bars (keeps most recent) to prevent unbounded cache growth.
    /// Uses zstd level 3 for merge writes (faster than level 9; archival can re-compress).
    /// Returns the full merged dataset as JSON.
    pub fn merge_bars(&self, key: &str, new_json: &str, max_bars: usize) -> Result<String, String> {
        // Parse new bars
        let new_bars: Vec<serde_json::Value> = serde_json::from_str(new_json)
            .map_err(|e| format!("JSON parse failed: {e}"))?;
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

        // Merge and deduplicate by numeric epoch-ms. String compare on RFC3339 silently
        // leaks duplicates when format drifts across sources (Z vs +00:00, millis vs no-millis)
        // and buckets all missing/empty timestamps together at the start of the series.
        all_bars.extend(new_bars);
        let ts_ms_of = |v: &serde_json::Value| -> Option<i64> {
            v["timestamp"].as_str()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.timestamp_millis())
                .filter(|ms| *ms > 0)
        };
        all_bars.retain(|b| ts_ms_of(b).is_some());
        all_bars.sort_by_key(|b| ts_ms_of(b).unwrap_or(0));
        all_bars.dedup_by(|a, b| ts_ms_of(a) == ts_ms_of(b));

        // Trim to max_bars (keep most recent) — prevents unbounded cache growth
        if max_bars > 0 && all_bars.len() > max_bars {
            let skip = all_bars.len() - max_bars;
            all_bars.drain(..skip);
        }

        // Store merged result (zstd level 3 for merge writes — faster than level 9)
        let merged_json = serde_json::to_string(&all_bars)
            .map_err(|e| format!("JSON serialize failed: {e}"))?;
        self.put_bars_fast(key, &merged_json)?;

        Ok(merged_json)
    }

    /// Store bar data with zstd level 3 (fast writes for frequent merge operations).
    /// Level 3 is ~3× faster than level 9 with only ~15% larger output.
    fn put_bars_fast(&self, key: &str, json_data: &str) -> Result<(), String> {
        let binary = pack_bars(json_data)?;
        let bar_count = u32::from_le_bytes(
            binary[4..8].try_into().map_err(|_| "bar_count header slice failed in put_bars_fast")?
        ) as i64;
        let (second_last_ts, last_ts) = get_last_two_bar_timestamps(&binary, bar_count as usize);
        let zstd_level = 3i32;
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
        let conn = self.read_conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        let mut stmt = conn.prepare_cached(
            "SELECT timestamp FROM bar_cache WHERE key = ?1"
        ).map_err(|e| format!("SQLite prepare failed: {e}"))?;

        match stmt.query_row(rusqlite::params![key], |row| row.get::<_, i64>(0)) {
            Ok(ts) => Ok(Some(chrono::Utc::now().timestamp() - ts)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(format!("SQLite query failed: {e}")),
        }
    }

    /// Get bar count for a cache entry. Returns None if key doesn't exist.
    pub fn get_bar_count(&self, key: &str) -> Result<Option<i64>, String> {
        let conn = self.read_conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        let mut stmt = conn.prepare_cached(
            "SELECT bar_count FROM bar_cache WHERE key = ?1"
        ).map_err(|e| format!("SQLite prepare failed: {e}"))?;

        match stmt.query_row(rusqlite::params![key], |row| row.get::<_, i64>(0)) {
            Ok(count) => Ok(Some(count)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(format!("SQLite query failed: {e}")),
        }
    }

    /// Batch-write pre-compressed bar entries in a single transaction.
    /// Takes (key, compressed_data, bar_count) tuples — compression done by caller.
    pub fn put_compressed_batch(&self, entries: &[(String, Vec<u8>, i64)]) -> Result<usize, String> {
        if entries.is_empty() { return Ok(0); }
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        conn.execute_batch("BEGIN").map_err(|e| format!("BEGIN failed: {e}"))?;
        let timestamp = chrono::Utc::now().timestamp();
        let mut count = 0;
        for (key, compressed, bar_count) in entries {
            match conn.execute(
                "INSERT OR REPLACE INTO bar_cache (key, data, timestamp, bar_count, zstd_level) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![key, compressed, timestamp, bar_count, 3],
            ) {
                Ok(_) => count += 1,
                Err(e) => tracing::warn!("Batch write skip {}: {}", key, e),
            }
        }
        conn.execute_batch("COMMIT").map_err(|e| format!("COMMIT failed: {e}"))?;
        Ok(count)
    }

    /// Bulk-load cache metadata for entries updated since `since_ts`.
    /// Returns Vec<(key, timestamp, bar_count)> — only changed entries.
    pub fn get_cache_meta_since(&self, since_ts: i64) -> Result<Vec<(String, i64, i64)>, String> {
        let conn = self.read_conn.lock().map_err(|e| format!("Read lock failed: {e}"))?;
        let mut stmt = conn.prepare(
            "SELECT key, timestamp, bar_count FROM bar_cache WHERE timestamp > ?1"
        ).map_err(|e| format!("SQLite prepare failed: {e}"))?;
        let mut result = Vec::new();
        let rows = stmt.query_map(rusqlite::params![since_ts], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?))
        }).map_err(|e| format!("SQLite query failed: {e}"))?;
        for row in rows {
            if let Ok(entry) = row { result.push(entry); }
        }
        Ok(result)
    }

    /// Bulk-load cache metadata (age_secs, bar_count) for all entries.
    /// Returns HashMap<key, (age_secs, bar_count)> — one query instead of N individual lookups.
    pub fn get_all_cache_meta(&self) -> Result<std::collections::HashMap<String, (i64, i64)>, String> {
        let conn = self.read_conn.lock().map_err(|e| format!("Read lock failed: {e}"))?;
        let mut stmt = conn.prepare(
            "SELECT key, timestamp, bar_count FROM bar_cache"
        ).map_err(|e| format!("SQLite prepare failed: {e}"))?;
        let now = chrono::Utc::now().timestamp();
        let mut map = std::collections::HashMap::new();
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?))
        }).map_err(|e| format!("SQLite query failed: {e}"))?;
        for row in rows {
            if let Ok((key, ts, bc)) = row {
                map.insert(key, (now - ts, bc));
            }
        }
        Ok(map)
    }

    /// Export entire cache to a compressed backup file.
    /// Format: zstd-compressed copy of the SQLite database file (via VACUUM INTO).
    pub fn export_backup(&self, path: &str) -> Result<String, String> {
        // Use SQLite's VACUUM INTO to create a consistent snapshot.
        // Use a unique temp file name to avoid TOCTOU races with concurrent exports.
        let backup_path = format!("{}.tmp.{}", path, std::process::id());
        // Remove any stale leftover from a previous crash
        let _ = std::fs::remove_file(&backup_path);

        // Hold write lock ONLY for VACUUM INTO — release before file I/O + compression
        {
            let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
            conn.execute("VACUUM INTO ?1", [&backup_path])
                .map_err(|e| format!("VACUUM INTO failed: {e}"))?;
        } // lock released here

        // File I/O + level-9 compression without holding any lock
        let data = std::fs::read(&backup_path)
            .map_err(|e| format!("Read backup failed: {e}"))?;
        let compressed = zstd::encode_all(data.as_slice(), 9)
            .map_err(|e| format!("Compress failed: {e}"))?;
        std::fs::write(path, &compressed)
            .map_err(|e| format!("Write backup failed: {e}"))?;
        let _ = std::fs::remove_file(&backup_path);

        let size_mb = compressed.len() as f64 / 1_048_576.0;
        Ok(format!("{{\"size_bytes\":{},\"size_mb\":{:.1}}}", compressed.len(), size_mb))
    }

    /// Import cache from a compressed backup file. Merges with existing data (newer wins).
    pub fn import_backup(&self, path: &str) -> Result<String, String> {
        let compressed = std::fs::read(path)
            .map_err(|e| format!("Read backup failed: {e}"))?;
        let data = zstd::decode_all(compressed.as_slice())
            .map_err(|e| format!("Decompress failed: {e}"))?;

        // Write to temp file with exclusive creation to avoid TOCTOU races
        let tmp_path = format!("{}.import.tmp.{}", path, std::process::id());
        {
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&tmp_path)
                .map_err(|e| format!("Create temp file failed (may already exist): {e}"))?;
            f.write_all(&data)
                .map_err(|e| {
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
        let bar_count = conn.execute(
            "INSERT OR REPLACE INTO bar_cache (key, data, timestamp, bar_count, zstd_level)
             SELECT b.key, b.data, b.timestamp, b.bar_count, COALESCE(b.zstd_level, 3)
             FROM backup_db.bar_cache b
             LEFT JOIN main.bar_cache c ON c.key = b.key
             WHERE c.key IS NULL OR b.timestamp > c.timestamp",
            [],
        ).map_err(|e| {
            let _ = conn.execute("DETACH DATABASE backup_db", []);
            let _ = std::fs::remove_file(&tmp_path);
            format!("Merge bar_cache failed: {e}")
        })?;

        // Merge kv_cache: same newer-wins strategy
        let kv_count = conn.execute(
            "INSERT OR REPLACE INTO kv_cache (key, value, timestamp)
             SELECT b.key, b.value, b.timestamp
             FROM backup_db.kv_cache b
             LEFT JOIN main.kv_cache c ON c.key = b.key
             WHERE c.key IS NULL OR b.timestamp > c.timestamp",
            [],
        ).map_err(|e| {
            let _ = conn.execute("DETACH DATABASE backup_db", []);
            let _ = std::fs::remove_file(&tmp_path);
            format!("Merge kv_cache failed: {e}")
        })?;

        conn.execute("DETACH DATABASE backup_db", [])
            .map_err(|e| format!("Detach failed: {e}"))?;

        let _ = std::fs::remove_file(&tmp_path);

        Ok(format!("{{\"bars_imported\":{},\"kv_imported\":{}}}", bar_count, kv_count))
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
        let mut stmt = conn.prepare_cached(
            "SELECT data FROM bar_cache WHERE key = ?1"
        ).ok()?;
        let blob: Vec<u8> = stmt.query_row(params![key], |r| r.get(0)).ok()?;
        let decompressed = maybe_decompress(blob).ok()?;
        if decompressed.len() < 8 || &decompressed[0..4] != BAR_BINARY_MAGIC { return None; }
        let count = u32::from_le_bytes(decompressed[4..8].try_into().ok()?) as usize;
        if count == 0 || decompressed.len() < 8 + count * 48 { return None; }
        let first_ts = i64::from_le_bytes(decompressed[8..16].try_into().ok()?);
        let last_off = 8 + (count - 1) * 48;
        let last_ts = i64::from_le_bytes(decompressed[last_off..last_off + 8].try_into().ok()?);
        Some((first_ts, last_ts))
    }

    /// Get a lock on the underlying connection for direct SQL operations.
    /// Used by darwin import to run table creation and batch inserts.
    pub fn connection(&self) -> Result<std::sync::MutexGuard<'_, Connection>, String> {
        self.conn.lock().map_err(|e| format!("Lock failed: {e}"))
    }

    /// Get a read-only connection for queries that don't mutate.
    /// Uses the dedicated read connection — never blocked by write operations.
    pub fn read_connection(&self) -> Result<std::sync::MutexGuard<'_, Connection>, String> {
        self.read_conn.lock().map_err(|e| format!("Read lock failed: {e}"))
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
        let conn = Connection::open_with_flags(&self.db_path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)
            .map_err(|e| format!("BG read conn open failed: {e}"))?;
        conn.busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|e| format!("BG read conn busy_timeout failed: {e}"))?;
        let _ = conn.execute_batch("
            PRAGMA cache_size=-32000;
            PRAGMA temp_store=MEMORY;
            PRAGMA mmap_size=268435456;
        ");
        Ok(conn)
    }

    /// Non-blocking version of get_bars_raw. Returns Ok(None) if lock is contended.
    pub fn try_get_bars_raw(&self, key: &str) -> Result<Option<Vec<(i64, f64, f64, f64, f64, f64)>>, String> {
        let conn = match self.read_conn.try_lock() {
            Ok(c) => c,
            Err(_) => return Ok(None), // lock contended — skip this frame
        };
        let mut stmt = conn.prepare_cached("SELECT data FROM bar_cache WHERE key = ?1")
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
                    let result = bars.iter().filter_map(|b| {
                        Some((
                            chrono::DateTime::parse_from_rfc3339(b["timestamp"].as_str()?).ok()?.timestamp_millis(),
                            b["open"].as_f64()?, b["high"].as_f64()?, b["low"].as_f64()?,
                            b["close"].as_f64()?, b["volume"].as_f64().unwrap_or(0.0),
                        ))
                    }).collect();
                    Ok(Some(result))
                }
            }
        }
    }

    /// List all kv_cache keys matching a prefix (e.g., "cred:" returns all credential keys).
    /// LIKE wildcards in the prefix are escaped to prevent overly broad matches.
    pub fn list_kv_keys(&self, prefix: &str) -> Result<Vec<String>, String> {
        let conn = self.read_conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        let escaped = prefix.replace('%', "\\%").replace('_', "\\_");
        let pattern = format!("{}%", escaped);
        let mut stmt = conn.prepare(
            "SELECT key FROM kv_cache WHERE key LIKE ?1 ESCAPE '\\'"
        ).map_err(|e| format!("SQLite prepare failed: {e}"))?;
        let rows = stmt.query_map(params![pattern], |row| {
            row.get::<_, String>(0)
        }).map_err(|e| format!("SQLite query failed: {e}"))?;
        let mut keys = Vec::new();
        for row in rows {
            if let Ok(k) = row { keys.push(k); }
        }
        Ok(keys)
    }

    // ── Sync State (LAN incremental sync tracking) ──────────────────

    /// Get the last sync timestamp for a given sync key.
    /// Returns 0 if no sync has been recorded (triggers full sync).
    pub fn get_sync_ts(&self, key: &str) -> i64 {
        let conn = match self.read_conn.lock() {
            Ok(c) => c,
            Err(_) => return 0,
        };
        conn.query_row(
            "SELECT last_sync_ts FROM sync_state WHERE key = ?1",
            params![key],
            |row| row.get::<_, i64>(0),
        ).unwrap_or(0)
    }

    /// Set the last sync timestamp for a given sync key.
    pub fn set_sync_ts(&self, key: &str, ts: i64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        conn.execute(
            "INSERT OR REPLACE INTO sync_state (key, last_sync_ts) VALUES (?1, ?2)",
            params![key, ts],
        ).map_err(|e| format!("SQLite insert sync_state failed: {e}"))?;
        Ok(())
    }

    // ── KV cache incremental queries (LAN sync) ─────────────────────

    /// List KV entries updated since a given timestamp.
    /// Returns (key, compressed_value) pairs for entries with timestamp > since_ts.
    /// Used by LAN sync server to send only new/updated KV entries.
    pub fn list_kv_entries_since(&self, since_ts: i64) -> Result<Vec<(String, Vec<u8>, i64)>, String> {
        let conn = self.read_conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        // prepare_cached avoids reparse on the hot LAN-sync polling loop.
        let mut stmt = conn.prepare_cached(
            "SELECT key, value, timestamp FROM kv_cache WHERE timestamp > ?1 ORDER BY timestamp ASC"
        ).map_err(|e| format!("SQLite prepare failed: {e}"))?;
        let rows = stmt.query_map(params![since_ts], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Vec<u8>>(1)?,
                row.get::<_, i64>(2)?,
            ))
        }).map_err(|e| format!("SQLite query failed: {e}"))?;
        let mut entries = Vec::new();
        for row in rows {
            if let Ok(entry) = row { entries.push(entry); }
        }
        Ok(entries)
    }

    /// Get the max timestamp in kv_cache (for sync state tracking).
    pub fn kv_max_timestamp(&self) -> i64 {
        let conn = match self.read_conn.lock() {
            Ok(c) => c,
            Err(_) => return 0,
        };
        conn.query_row("SELECT COALESCE(MAX(timestamp), 0) FROM kv_cache", [], |r| r.get::<_, i64>(0))
            .unwrap_or(0)
    }

    /// Count rows in kv_cache.
    pub fn kv_count(&self) -> i64 {
        let conn = match self.read_conn.lock() {
            Ok(c) => c,
            Err(_) => return 0,
        };
        conn.query_row("SELECT COUNT(*) FROM kv_cache", [], |r| r.get::<_, i64>(0))
            .unwrap_or(0)
    }

    /// Get raw bar cache entry without decompression (for LAN sync transfer).
    /// Returns the compressed blob and its timestamp as stored in SQLite.
    pub fn get_raw_bar_entry(&self, key: &str) -> Result<Option<(Vec<u8>, i64)>, String> {
        let conn = self.read_conn.lock().map_err(|e| format!("Read lock failed: {e}"))?;
        let mut stmt = conn.prepare_cached(
            "SELECT data, timestamp FROM bar_cache WHERE key = ?1"
        ).map_err(|e| format!("SQLite prepare failed: {e}"))?;

        match stmt.query_row(params![key], |row| {
            let data: Vec<u8> = row.get(0)?;
            let timestamp: i64 = row.get(1)?;
            Ok((data, timestamp))
        }) {
            Ok(result) => Ok(Some(result)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(format!("SQLite query failed: {e}")),
        }
    }

    /// Write raw bar cache entry (from LAN sync, no compression needed — already compressed).
    pub fn put_raw_bar_entry(&self, key: &str, data: &[u8], timestamp: i64, bar_count: i64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        conn.execute(
            "INSERT OR REPLACE INTO bar_cache (key, data, timestamp, bar_count, zstd_level) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![key, data, timestamp, bar_count, 3],
        ).map_err(|e| format!("SQLite insert failed: {e}"))?;
        Ok(())
    }

    /// List all keys in bar_cache.
    pub fn all_keys(&self) -> Result<Vec<String>, String> {
        let conn = self.read_conn.lock().map_err(|e| format!("Read lock failed: {e}"))?;
        let mut stmt = conn.prepare_cached("SELECT key FROM bar_cache")
            .map_err(|e| format!("Prepare failed: {e}"))?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| format!("Query failed: {e}"))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Collect failed: {e}"))
    }

    /// Get the raw compressed blob, timestamp, and bar_count for a key.
    /// Used for zero-copy sync between databases.
    pub fn get_raw_blob(&self, key: &str) -> Result<Option<(Vec<u8>, i64, i64)>, String> {
        let conn = self.read_conn.lock().map_err(|e| format!("Read lock failed: {e}"))?;
        let mut stmt = conn.prepare_cached("SELECT data, timestamp, bar_count FROM bar_cache WHERE key = ?1")
            .map_err(|e| format!("Prepare failed: {e}"))?;
        let result = stmt.query_row(params![key], |row| {
            // Use get_ref to accept both BLOB and TEXT without UTF-8 validation.
            // MQL5's DatabaseBindArray can bind uchar[] as TEXT type, making SQLite
            // store the result of BLOB || TEXT concatenation as TEXT. A String fallback
            // fails with "invalid utf-8 sequence" because binary bar data is not UTF-8.
            // get_ref returns the raw SQLite value regardless of type.
            let data: Vec<u8> = match row.get_ref(0)? {
                rusqlite::types::ValueRef::Blob(b) => b.to_vec(),
                rusqlite::types::ValueRef::Text(t) => t.to_vec(),
                _ => return Err(rusqlite::Error::InvalidColumnType(0, "data".into(), rusqlite::types::Type::Blob)),
            };
            Ok((data, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?))
        });
        match result {
            Ok(r) => Ok(Some(r)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(format!("Query failed: {e}")),
        }
    }

    /// Put a raw compressed blob (zero-copy from another database).
    /// Overwrites if the source timestamp is newer.
    pub fn put_raw_blob(&self, key: &str, blob: &[u8], ts: i64, bar_count: i64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        conn.execute(
            "INSERT INTO bar_cache (key, data, timestamp, bar_count)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(key) DO UPDATE SET data=excluded.data, timestamp=excluded.timestamp, bar_count=excluded.bar_count
             WHERE excluded.timestamp > bar_cache.timestamp",
            params![key, blob, ts, bar_count],
        ).map_err(|e| format!("Insert failed: {e}"))?;
        Ok(())
    }

    /// Read all bid/ask quotes from the bid_ask table (BarCacheWriter live prices).
    /// Returns Vec of (symbol, bid, ask, spread).
    pub fn read_bid_ask(&self) -> Result<Vec<(String, f64, f64, f64)>, String> {
        let conn = self.read_conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        let mut stmt = conn.prepare_cached("SELECT symbol, bid, ask, spread FROM bid_ask WHERE bid > 0 OR ask > 0")
            .map_err(|e| format!("Prepare failed (bid_ask table may not exist): {e}"))?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, f64>(1)?,
                row.get::<_, f64>(2)?,
                row.get::<_, f64>(3)?,
            ))
        }).map_err(|e| format!("Query failed: {e}"))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Delete all cache entries matching a symbol prefix (e.g., "AAPL:" deletes all TFs for AAPL).
    pub fn delete_symbol(&self, symbol_prefix: &str) -> Result<u64, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        // Escape LIKE wildcards in prefix to prevent overly broad deletion
        let escaped = symbol_prefix.replace('%', "\\%").replace('_', "\\_");
        let pattern = format!("{}:%", escaped);
        let deleted = conn.execute(
            "DELETE FROM bar_cache WHERE key LIKE ?1 ESCAPE '\\'", params![pattern]
        ).map_err(|e| format!("SQLite delete failed: {e}"))? as u64;
        Ok(deleted)
    }

    /// Delete ALL bar data from the cache. Returns the number of rows deleted.
    /// Runs VACUUM to reclaim freed pages and shrink the DB file on disk.
    pub fn delete_all_bars(&self) -> Result<u64, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        let deleted = conn.execute("DELETE FROM bar_cache", [])
            .map_err(|e| format!("SQLite delete failed: {e}"))? as u64;
        let _ = conn.execute("DELETE FROM bar_track", []);
        // VACUUM reclaims freed pages — without this, the DB file stays the same
        // size after DELETE (e.g., 56 GB file with only 14 GB of live data).
        let _ = conn.execute_batch("VACUUM");
        Ok(deleted)
    }

    /// Delete ALL DARWIN data (accounts, deals, positions, equity snapshots).
    /// Returns the total number of rows deleted across all tables.
    /// Runs VACUUM to reclaim freed pages and shrink the DB file on disk.
    pub fn delete_all_darwin(&self) -> Result<u64, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        let mut total = 0u64;
        // Table names are compile-time constants, not user input — safe for format!()
        for table in &["darwin_deals", "darwin_positions", "darwin_equity_snapshots", "darwin_accounts"] {
            let deleted = conn.execute(&format!("DELETE FROM {}", table), []).unwrap_or(0) as u64;
            total += deleted;
        }
        let _ = conn.execute_batch("VACUUM");
        Ok(total)
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

    /// Scan bar_cache for entries with bar_count=0 and repair from TTBR header.
    /// BarCacheWriter stores bar_count correctly, but LAN sync and earlier versions
    /// may have left stale 0 values. Returns number of entries repaired.
    pub fn repair_bar_counts(&self) -> Result<usize, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        let mut stmt = conn.prepare("SELECT key, data FROM bar_cache WHERE bar_count = 0 OR bar_count IS NULL")
            .map_err(|e| format!("Prepare failed: {e}"))?;
        let mut updates: Vec<(String, i64)> = Vec::new();
        let rows = stmt.query_map([], |row| {
            let key: String = row.get(0)?;
            let data: Vec<u8> = row.get(1)?;
            Ok((key, data))
        }).map_err(|e| format!("Query failed: {e}"))?;
        for row in rows {
            if let Ok((key, data)) = row {
                // Skip MT5 metadata entries (raw text, not bar data)
                if key.contains("__SPECS__") || key.contains("__SYMBOLS__") || key.contains("__SERVER__") {
                    continue;
                }
                let bytes = match maybe_decompress(data) {
                    Ok(b) => b,
                    Err(e) => { tracing::warn!("repair_bar_counts: decompress failed for {key}: {e}"); continue; }
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
        let total: i64 = conn.query_row(
            "SELECT COALESCE(SUM(LENGTH(data)), 0) FROM bar_cache",
            [],
            |r| r.get(0),
        ).unwrap_or(0);
        if total <= max_bytes { return Ok((0, 0)); }
        let cutoff_ts = chrono::Utc::now().timestamp() - 7 * 86400; // 7 days
        // Select oldest entries (excluding hot ones)
        let mut stmt = conn.prepare(
            "SELECT key, LENGTH(data) FROM bar_cache WHERE timestamp < ?1 ORDER BY timestamp ASC"
        ).map_err(|e| format!("Prepare evict failed: {e}"))?;
        let rows: Vec<(String, i64)> = stmt.query_map([cutoff_ts], |r| Ok((r.get(0)?, r.get(1)?)))
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
            if freed >= target_free { break; }
            keys_to_delete.push(key);
            freed += size;
        }
        let evicted = keys_to_delete.len();
        if !keys_to_delete.is_empty() {
            // Chunked to stay within SQLITE_MAX_VARIABLE_NUMBER (32766 in modern sqlite)
            const CHUNK: usize = 512;
            for chunk in keys_to_delete.chunks(CHUNK) {
                let placeholders = std::iter::repeat("?").take(chunk.len()).collect::<Vec<_>>().join(",");
                let sql = format!("DELETE FROM bar_cache WHERE key IN ({placeholders})");
                let params_refs: Vec<&dyn rusqlite::types::ToSql> = chunk.iter().map(|k| k as &dyn rusqlite::types::ToSql).collect();
                conn.execute(&sql, params_refs.as_slice()).map_err(|e| format!("Bulk evict delete failed: {e}"))?;
            }
        }
        Ok((evicted, freed))
    }

    /// Recompress all bar_cache entries at target zstd level (e.g. 19 for max compression).
    /// Decompression speed is identical regardless of compression level — only storage shrinks.
    /// Returns (entries_processed, bytes_saved).
    /// Progress callback: (processed, total, key, old_size, new_size)
    pub fn compact_storage(&self, level: i32, progress: Option<&dyn Fn(usize, usize, &str, usize, usize)>) -> Result<(usize, i64), String> {
        // Phase 1: Read entries that need compaction (skip already at target level)
        let entries: Vec<(String, Vec<u8>)>;
        let kv_entries: Vec<(String, Vec<u8>)>;
        let _skipped_count: usize;
        {
            let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
            // Only select entries where zstd_level < target — skip already-compacted
            let mut stmt = conn.prepare("SELECT key, data FROM bar_cache WHERE zstd_level < ?1")
                .map_err(|e| format!("Prepare failed: {e}"))?;
            entries = stmt.query_map(params![level], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
            }).map_err(|e| format!("Query failed: {e}"))?
              .filter_map(|r| r.ok())
              .collect();
            // Count how many were skipped (already at target level)
            _skipped_count = conn.query_row(
                "SELECT COUNT(*) FROM bar_cache WHERE zstd_level >= ?1",
                params![level], |r| r.get::<_, i64>(0),
            ).unwrap_or(0) as usize;
            drop(stmt);
            kv_entries = match conn.prepare("SELECT key, data FROM kv_store") {
                Ok(mut kv_stmt) => {
                    kv_stmt.query_map([], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
                    }).ok().map(|rows| rows.filter_map(|r| r.ok()).collect()).unwrap_or_default()
                }
                Err(_) => Vec::new(),
            };
        } // Lock released — UI thread can read cache freely during recompression

        // Phase 2: Recompress on CPU (no lock held — this is the slow part)
        let total = entries.len() + kv_entries.len();
        let mut processed = 0usize;
        let mut bytes_saved = 0i64;
        let mut bar_updates: Vec<(String, Vec<u8>)> = Vec::new();
        let mut kv_updates: Vec<(String, Vec<u8>)> = Vec::new();

        for (key, compressed) in &entries {
            let decompressed = match zstd::decode_all(compressed.as_slice()) {
                Ok(d) => d,
                Err(_) => { processed += 1; continue; }
            };
            let recompressed = match zstd::encode_all(decompressed.as_slice(), level) {
                Ok(r) => r,
                Err(_) => { processed += 1; continue; }
            };
            let saved = compressed.len() as i64 - recompressed.len() as i64;
            if saved > 0 {
                bar_updates.push((key.clone(), recompressed.clone()));
                bytes_saved += saved;
            }
            processed += 1;
            if let Some(cb) = progress {
                cb(processed, total, key, compressed.len(), if saved > 0 { recompressed.len() } else { compressed.len() });
            }
        }

        for (key, compressed) in &kv_entries {
            if let Ok(decompressed) = zstd::decode_all(compressed.as_slice()) {
                if let Ok(recompressed) = zstd::encode_all(decompressed.as_slice(), level) {
                    let saved = compressed.len() as i64 - recompressed.len() as i64;
                    if saved > 0 {
                        kv_updates.push((key.clone(), recompressed));
                        bytes_saved += saved;
                    }
                }
            }
            processed += 1;
            if let Some(cb) = progress {
                cb(processed, total, key, compressed.len(), compressed.len());
            }
        }

        // Phase 3: Write updates in batches (brief lock per batch, UI stays responsive)
        // Also updates zstd_level so subsequent compacts skip already-processed entries
        {
            let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
            for chunk in bar_updates.chunks(50) {
                let _ = conn.execute_batch("BEGIN;");
                for (key, data) in chunk {
                    let _ = conn.execute("UPDATE bar_cache SET data = ?1, zstd_level = ?2 WHERE key = ?3", params![data, level, key]);
                }
                let _ = conn.execute_batch("COMMIT;");
            }
            for chunk in kv_updates.chunks(50) {
                let _ = conn.execute_batch("BEGIN;");
                for (key, data) in chunk {
                    let _ = conn.execute("UPDATE kv_store SET data = ?1 WHERE key = ?2", params![data, key]);
                }
                let _ = conn.execute_batch("COMMIT;");
            }
        }

        // Phase 4: VACUUM (brief lock, reclaims space)
        {
            let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
            let _ = conn.execute_batch("VACUUM;");
        }

        Ok((processed, bytes_saved))
    }

    /// Export selected cache keys to a portable binary bundle for LAN sync.
    /// Bundle format: [u32 entry_count][per entry: u32 key_len, key_bytes, u32 data_len, data_bytes, i64 timestamp, i64 bar_count]
    /// Returns the serialized bundle bytes.
    pub fn export_keys(&self, key_patterns: &[&str]) -> Result<Vec<u8>, String> {
        let conn = self.read_conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        let all_keys = {
            let mut stmt = conn.prepare("SELECT key FROM bar_cache")
                .map_err(|e| format!("Prepare failed: {e}"))?;
            let rows = stmt.query_map([], |row| row.get::<_, String>(0))
                .map_err(|e| format!("Query failed: {e}"))?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("Collect failed: {e}"))?
        };

        // Filter keys matching any pattern (prefix match or substring match)
        let matched: Vec<&String> = all_keys.iter()
            .filter(|k| key_patterns.iter().any(|p| k.contains(p) || k.starts_with(p)))
            .collect();

        let mut buf = Vec::new();
        buf.extend_from_slice(&(matched.len() as u32).to_le_bytes());

        for key in &matched {
            // Read raw compressed data directly (no decompression needed)
            let mut stmt = conn.prepare_cached(
                "SELECT data, timestamp, bar_count FROM bar_cache WHERE key = ?1"
            ).map_err(|e| format!("Prepare failed: {e}"))?;

            if let Ok(row) = stmt.query_row(params![key], |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            }) {
                let (data, timestamp, bar_count) = row;
                let key_bytes = key.as_bytes();
                buf.extend_from_slice(&(key_bytes.len() as u32).to_le_bytes());
                buf.extend_from_slice(key_bytes);
                buf.extend_from_slice(&(data.len() as u32).to_le_bytes());
                buf.extend_from_slice(&data);
                buf.extend_from_slice(&timestamp.to_le_bytes());
                buf.extend_from_slice(&bar_count.to_le_bytes());
            }
        }

        Ok(buf)
    }

    /// Import a portable bundle into this cache (from LAN sync).
    /// Returns number of entries imported.
    pub fn import_keys(&self, bundle: &[u8]) -> Result<usize, String> {
        if bundle.len() < 4 { return Err("Bundle too small".into()); }
        let count = u32::from_le_bytes(bundle[0..4].try_into().map_err(|_| "Bad header")?) as usize;
        let mut offset = 4;
        let mut imported = 0;

        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;

        for _ in 0..count {
            if offset + 4 > bundle.len() { break; }
            let key_len = u32::from_le_bytes(bundle[offset..offset+4].try_into().map_err(|_| "Bad key_len")?) as usize;
            offset += 4;
            if offset + key_len > bundle.len() { break; }
            let key = std::str::from_utf8(&bundle[offset..offset+key_len]).map_err(|_| "Bad key UTF-8")?.to_string();
            offset += key_len;

            if offset + 4 > bundle.len() { break; }
            let data_len = u32::from_le_bytes(bundle[offset..offset+4].try_into().map_err(|_| "Bad data_len")?) as usize;
            offset += 4;
            if offset + data_len > bundle.len() { break; }
            let data = &bundle[offset..offset+data_len];
            offset += data_len;

            if offset + 16 > bundle.len() { break; }
            let timestamp = i64::from_le_bytes(bundle[offset..offset+8].try_into().map_err(|_| "Bad timestamp")?);
            offset += 8;
            let bar_count = i64::from_le_bytes(bundle[offset..offset+8].try_into().map_err(|_| "Bad bar_count")?);
            offset += 8;

            conn.execute(
                "INSERT OR REPLACE INTO bar_cache (key, data, timestamp, bar_count, zstd_level) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![key, data, timestamp, bar_count, 3],
            ).map_err(|e| format!("Insert failed: {e}"))?;
            imported += 1;
        }

        Ok(imported)
    }

    /// List keys matching patterns with their sizes (for sync preview).
    /// Returns Vec of (key, bar_count, compressed_size_bytes).
    pub fn list_matching_keys(&self, patterns: &[&str]) -> Result<Vec<(String, i64, usize)>, String> {
        let conn = self.read_conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        let mut stmt = conn.prepare("SELECT key, bar_count, length(data) FROM bar_cache")
            .map_err(|e| format!("Prepare failed: {e}"))?;
        let rows: Vec<(String, i64, usize)> = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)? as usize))
        }).map_err(|e| format!("Query failed: {e}"))?
          .filter_map(|r| r.ok())
          .filter(|(k, _, _)| patterns.iter().any(|p| k.contains(p) || k.starts_with(p)))
          .collect();
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// Monotonic counter for unique temp DB paths across parallel tests.
    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    /// Helper: unique temp DB path per test invocation (no external crate needed).
    fn temp_db_path() -> PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let pid = std::process::id();
        std::env::temp_dir().join(format!("typhoon_cache_test_{}_{}.db", pid, id))
    }

    /// Helper: build a valid TTBR binary blob with N bars.
    fn make_binary_bars(bars: &[(i64, f64, f64, f64, f64, f64)]) -> Vec<u8> {
        let count = bars.len() as u32;
        let mut buf = Vec::with_capacity(4 + 4 + bars.len() * BYTES_PER_BAR);
        buf.extend_from_slice(BAR_BINARY_MAGIC);
        buf.extend_from_slice(&count.to_le_bytes());
        for &(ts, o, h, l, c, v) in bars {
            buf.extend_from_slice(&ts.to_le_bytes());
            buf.extend_from_slice(&o.to_le_bytes());
            buf.extend_from_slice(&h.to_le_bytes());
            buf.extend_from_slice(&l.to_le_bytes());
            buf.extend_from_slice(&c.to_le_bytes());
            buf.extend_from_slice(&v.to_le_bytes());
        }
        buf
    }

    // ---- unpack_bars_raw tests ----

    #[test]
    fn unpack_bars_raw_single_bar() {
        let ts: i64 = 1_700_000_000_000; // 2023-11-14T22:13:20Z
        let bars = vec![(ts, 100.0, 105.0, 99.0, 103.0, 5000.0)];
        let binary = make_binary_bars(&bars);
        let result = unpack_bars_raw(&binary).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], (ts, 100.0, 105.0, 99.0, 103.0, 5000.0));
    }

    #[test]
    fn unpack_bars_raw_multiple_bars() {
        let bars = vec![
            (1_700_000_000_000, 100.0, 105.0, 99.0, 103.0, 5000.0),
            (1_700_000_060_000, 103.0, 107.0, 102.0, 106.0, 6000.0),
            (1_700_000_120_000, 106.0, 108.0, 104.0, 105.0, 4500.0),
        ];
        let binary = make_binary_bars(&bars);
        let result = unpack_bars_raw(&binary).unwrap();
        assert_eq!(result.len(), 3);
        for (i, bar) in bars.iter().enumerate() {
            assert_eq!(result[i], *bar);
        }
    }

    #[test]
    fn unpack_bars_raw_zero_bars() {
        let binary = make_binary_bars(&[]);
        let result = unpack_bars_raw(&binary).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn unpack_bars_raw_empty_data() {
        let result = unpack_bars_raw(&[]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Not binary bar format");
    }

    #[test]
    fn unpack_bars_raw_too_short_for_header() {
        let result = unpack_bars_raw(&[b'T', b'T', b'B']);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Not binary bar format");
    }

    #[test]
    fn unpack_bars_raw_wrong_magic() {
        let mut binary = make_binary_bars(&[(0, 1.0, 2.0, 3.0, 4.0, 5.0)]);
        binary[0] = b'X'; // corrupt magic
        let result = unpack_bars_raw(&binary);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Not binary bar format");
    }

    #[test]
    fn unpack_bars_raw_truncated_data() {
        let bars = vec![(1_700_000_000_000, 100.0, 105.0, 99.0, 103.0, 5000.0)];
        let mut binary = make_binary_bars(&bars);
        binary.truncate(binary.len() - 10); // chop off last 10 bytes
        let result = unpack_bars_raw(&binary);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Binary data truncated"));
    }

    #[test]
    fn unpack_bars_raw_count_claims_more_than_available() {
        // Header says 5 bars but only 1 bar of data follows
        let mut buf = Vec::new();
        buf.extend_from_slice(BAR_BINARY_MAGIC);
        buf.extend_from_slice(&5u32.to_le_bytes()); // claim 5 bars
        // Only write 1 bar worth of data
        buf.extend_from_slice(&0i64.to_le_bytes());
        for _ in 0..5 {
            buf.extend_from_slice(&1.0f64.to_le_bytes());
        }
        let result = unpack_bars_raw(&buf);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Binary data truncated"));
    }

    #[test]
    fn unpack_bars_raw_preserves_negative_values() {
        let bars = vec![(0, -10.5, -5.0, -20.0, -15.0, 0.0)];
        let binary = make_binary_bars(&bars);
        let result = unpack_bars_raw(&binary).unwrap();
        assert_eq!(result[0], (0, -10.5, -5.0, -20.0, -15.0, 0.0));
    }

    #[test]
    fn unpack_bars_raw_preserves_zero_volume() {
        let bars = vec![(1_000, 1.0, 2.0, 0.5, 1.5, 0.0)];
        let binary = make_binary_bars(&bars);
        let result = unpack_bars_raw(&binary).unwrap();
        assert_eq!(result[0].5, 0.0);
    }

    // ---- pack_bars / unpack_bars roundtrip tests ----

    #[test]
    fn pack_unpack_roundtrip() {
        let json = r#"[
            {"timestamp":"2024-01-15T12:00:00+00:00","open":100.0,"high":105.0,"low":99.0,"close":103.0,"volume":5000.0},
            {"timestamp":"2024-01-15T13:00:00+00:00","open":103.0,"high":107.0,"low":102.0,"close":106.0,"volume":6000.0}
        ]"#;
        let binary = pack_bars(json).unwrap();
        // Verify magic + count header
        assert_eq!(&binary[0..4], BAR_BINARY_MAGIC);
        assert_eq!(u32::from_le_bytes(binary[4..8].try_into().unwrap()), 2);
        // Roundtrip through unpack_bars
        let result_json = unpack_bars(&binary).unwrap();
        let result: Vec<serde_json::Value> = serde_json::from_str(&result_json).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0]["open"].as_f64().unwrap(), 100.0);
        assert_eq!(result[1]["close"].as_f64().unwrap(), 106.0);
    }

    #[test]
    fn pack_unpack_raw_roundtrip() {
        let json = r#"[
            {"timestamp":"2024-01-15T12:00:00+00:00","open":1.2345,"high":1.2400,"low":1.2300,"close":1.2380,"volume":12345.0}
        ]"#;
        let binary = pack_bars(json).unwrap();
        let raw = unpack_bars_raw(&binary).unwrap();
        assert_eq!(raw.len(), 1);
        assert_eq!(raw[0].1, 1.2345); // open
        assert_eq!(raw[0].2, 1.2400); // high
        assert_eq!(raw[0].3, 1.2300); // low
        assert_eq!(raw[0].4, 1.2380); // close
        assert_eq!(raw[0].5, 12345.0); // volume
    }

    #[test]
    fn pack_bars_empty_array() {
        let binary = pack_bars("[]").unwrap();
        assert_eq!(&binary[0..4], BAR_BINARY_MAGIC);
        assert_eq!(u32::from_le_bytes(binary[4..8].try_into().unwrap()), 0);
        assert_eq!(binary.len(), 8); // just header, no bar data
    }

    #[test]
    fn pack_bars_invalid_json() {
        let result = pack_bars("not json");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("JSON parse failed"));
    }

    // ---- unpack_bars tests ----

    #[test]
    fn unpack_bars_wrong_magic() {
        let result = unpack_bars(&[0, 0, 0, 0, 0, 0, 0, 0]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Not binary bar format");
    }

    #[test]
    fn unpack_bars_truncated() {
        let bars = vec![(1_700_000_000_000, 50.0, 55.0, 49.0, 53.0, 1000.0)];
        let mut binary = make_binary_bars(&bars);
        binary.truncate(20); // corrupt: not enough data for 1 bar
        let result = unpack_bars(&binary);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Binary data truncated"));
    }

    // ---- unpack_bars_tail tests ----

    #[test]
    fn unpack_bars_tail_returns_last_n() {
        let json = r#"[
            {"timestamp":"2024-01-01T00:00:00+00:00","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":100.0},
            {"timestamp":"2024-01-02T00:00:00+00:00","open":2.0,"high":3.0,"low":1.5,"close":2.5,"volume":200.0},
            {"timestamp":"2024-01-03T00:00:00+00:00","open":3.0,"high":4.0,"low":2.5,"close":3.5,"volume":300.0}
        ]"#;
        let binary = pack_bars(json).unwrap();
        let tail_json = unpack_bars_tail(&binary, 2).unwrap();
        let tail: Vec<serde_json::Value> = serde_json::from_str(&tail_json).unwrap();
        assert_eq!(tail.len(), 2);
        assert_eq!(tail[0]["open"].as_f64().unwrap(), 2.0);
        assert_eq!(tail[1]["open"].as_f64().unwrap(), 3.0);
    }

    #[test]
    fn unpack_bars_tail_zero_returns_all() {
        let json = r#"[
            {"timestamp":"2024-01-01T00:00:00+00:00","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":100.0}
        ]"#;
        let binary = pack_bars(json).unwrap();
        let tail_json = unpack_bars_tail(&binary, 0).unwrap();
        let tail: Vec<serde_json::Value> = serde_json::from_str(&tail_json).unwrap();
        assert_eq!(tail.len(), 1);
    }

    #[test]
    fn unpack_bars_tail_exceeding_count_returns_all() {
        let json = r#"[
            {"timestamp":"2024-01-01T00:00:00+00:00","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":100.0}
        ]"#;
        let binary = pack_bars(json).unwrap();
        let tail_json = unpack_bars_tail(&binary, 999).unwrap();
        let tail: Vec<serde_json::Value> = serde_json::from_str(&tail_json).unwrap();
        assert_eq!(tail.len(), 1);
    }

    // ---- get_last_two_bar_timestamps tests ----

    #[test]
    fn extract_tail_timestamps_two_bars() {
        let bars = vec![
            (1_705_000_000_000i64, 1.0, 2.0, 0.5, 1.5, 100.0),
            (1_705_100_000_000i64, 2.0, 3.0, 1.5, 2.5, 200.0),
        ];
        let binary = make_binary_bars(&bars);
        let (second, last) = get_last_two_bar_timestamps(&binary, 2);
        assert!(second.is_some());
        assert!(last.is_some());
        // second_last should correspond to first bar's timestamp
        let second_dt = chrono::DateTime::parse_from_rfc3339(&second.unwrap()).unwrap();
        assert_eq!(second_dt.timestamp_millis(), 1_705_000_000_000);
        let last_dt = chrono::DateTime::parse_from_rfc3339(&last.unwrap()).unwrap();
        assert_eq!(last_dt.timestamp_millis(), 1_705_100_000_000);
    }

    #[test]
    fn extract_tail_timestamps_single_bar_returns_none() {
        let bars = vec![(1_705_000_000_000i64, 1.0, 2.0, 0.5, 1.5, 100.0)];
        let binary = make_binary_bars(&bars);
        let (second, last) = get_last_two_bar_timestamps(&binary, 1);
        assert!(second.is_none());
        assert!(last.is_none());
    }

    #[test]
    fn extract_tail_timestamps_empty_returns_none() {
        let binary = make_binary_bars(&[]);
        let (second, last) = get_last_two_bar_timestamps(&binary, 0);
        assert!(second.is_none());
        assert!(last.is_none());
    }

    // ---- binary format size tests ----

    #[test]
    fn binary_format_size_is_correct() {
        assert_eq!(BYTES_PER_BAR, 48); // i64 + 5*f64
        let bars = vec![
            (0, 1.0, 2.0, 3.0, 4.0, 5.0),
            (1, 6.0, 7.0, 8.0, 9.0, 10.0),
        ];
        let binary = make_binary_bars(&bars);
        // 4 (magic) + 4 (count) + 2 * 48 (bars) = 104
        assert_eq!(binary.len(), 4 + 4 + 2 * 48);
    }

    // ---- SqliteCache integration tests ----

    #[test]
    fn sqlite_cache_put_get_bars() {
        let db_path = temp_db_path();
        let cache = SqliteCache::open(&db_path).unwrap();

        let json = r#"[{"timestamp":"2024-06-01T00:00:00+00:00","open":50.0,"high":55.0,"low":49.0,"close":53.0,"volume":1000.0}]"#;
        cache.put_bars("TEST:1Hour", json).unwrap();

        let result = cache.get_bars("TEST:1Hour").unwrap();
        assert!(result.is_some());
        let (returned_json, _ts) = result.unwrap();
        let bars: Vec<serde_json::Value> = serde_json::from_str(&returned_json).unwrap();
        assert_eq!(bars.len(), 1);
        assert_eq!(bars[0]["open"].as_f64().unwrap(), 50.0);
    }

    #[test]
    fn sqlite_cache_get_bars_raw_roundtrip() {
        let db_path = temp_db_path();
        let cache = SqliteCache::open(&db_path).unwrap();

        let json = r#"[
            {"timestamp":"2024-06-01T00:00:00+00:00","open":1.1,"high":1.2,"low":1.0,"close":1.15,"volume":500.0},
            {"timestamp":"2024-06-01T01:00:00+00:00","open":1.15,"high":1.3,"low":1.1,"close":1.25,"volume":600.0}
        ]"#;
        cache.put_bars("EURUSD:1Hour", json).unwrap();

        let raw = cache.get_bars_raw("EURUSD:1Hour").unwrap().unwrap();
        assert_eq!(raw.len(), 2);
        assert_eq!(raw[0].1, 1.1);  // open
        assert_eq!(raw[1].4, 1.25); // close
    }

    #[test]
    fn sqlite_cache_missing_key_returns_none() {
        let db_path = temp_db_path();
        let cache = SqliteCache::open(&db_path).unwrap();
        assert!(cache.get_bars("NONEXISTENT").unwrap().is_none());
        assert!(cache.get_bars_raw("NONEXISTENT").unwrap().is_none());
    }

    #[test]
    fn sqlite_cache_kv_roundtrip() {
        let db_path = temp_db_path();
        let cache = SqliteCache::open(&db_path).unwrap();

        cache.put_kv("fundamentals:AAPL", r#"{"pe":25.0}"#).unwrap();
        let result = cache.get_kv("fundamentals:AAPL").unwrap();
        assert_eq!(result.unwrap(), r#"{"pe":25.0}"#);
    }

    #[test]
    fn sqlite_cache_kv_missing_returns_none() {
        let db_path = temp_db_path();
        let cache = SqliteCache::open(&db_path).unwrap();
        assert!(cache.get_kv("missing").unwrap().is_none());
    }

    #[test]
    fn sqlite_cache_stats() {
        let db_path = temp_db_path();
        let cache = SqliteCache::open(&db_path).unwrap();

        let json = r#"[{"timestamp":"2024-01-01T00:00:00+00:00","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":100.0}]"#;
        cache.put_bars("A:1D", json).unwrap();
        cache.put_kv("k1", "v1").unwrap();

        let (bar_count, kv_count, _size) = cache.stats().unwrap();
        assert_eq!(bar_count, 1);
        assert_eq!(kv_count, 1);
    }

    #[test]
    fn sqlite_cache_delete_key() {
        let db_path = temp_db_path();
        let cache = SqliteCache::open(&db_path).unwrap();

        let json = r#"[{"timestamp":"2024-01-01T00:00:00+00:00","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":100.0}]"#;
        cache.put_bars("DEL:1D", json).unwrap();
        assert!(cache.get_bars("DEL:1D").unwrap().is_some());

        let deleted = cache.delete_key("DEL:1D").unwrap();
        assert!(deleted);
        assert!(cache.get_bars("DEL:1D").unwrap().is_none());
    }

    #[test]
    fn sqlite_cache_delete_nonexistent_key() {
        let db_path = temp_db_path();
        let cache = SqliteCache::open(&db_path).unwrap();
        let deleted = cache.delete_key("NOPE").unwrap();
        assert!(!deleted);
    }

    #[test]
    fn sqlite_cache_bar_count() {
        let db_path = temp_db_path();
        let cache = SqliteCache::open(&db_path).unwrap();

        let json = r#"[
            {"timestamp":"2024-01-01T00:00:00+00:00","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":100.0},
            {"timestamp":"2024-01-02T00:00:00+00:00","open":2.0,"high":3.0,"low":1.5,"close":2.5,"volume":200.0}
        ]"#;
        cache.put_bars("CNT:1D", json).unwrap();
        assert_eq!(cache.get_bar_count("CNT:1D").unwrap(), Some(2));
        assert_eq!(cache.get_bar_count("MISSING").unwrap(), None);
    }

    #[test]
    fn sqlite_cache_merge_bars_dedup() {
        let db_path = temp_db_path();
        let cache = SqliteCache::open(&db_path).unwrap();

        let json1 = r#"[
            {"timestamp":"2024-01-01T00:00:00+00:00","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":100.0},
            {"timestamp":"2024-01-02T00:00:00+00:00","open":2.0,"high":3.0,"low":1.5,"close":2.5,"volume":200.0}
        ]"#;
        cache.put_bars("MRG:1D", json1).unwrap();

        // Merge with overlapping + new bar
        let json2 = r#"[
            {"timestamp":"2024-01-02T00:00:00+00:00","open":2.1,"high":3.1,"low":1.6,"close":2.6,"volume":210.0},
            {"timestamp":"2024-01-03T00:00:00+00:00","open":3.0,"high":4.0,"low":2.5,"close":3.5,"volume":300.0}
        ]"#;
        let merged_json = cache.merge_bars("MRG:1D", json2, 10000).unwrap();
        let merged: Vec<serde_json::Value> = serde_json::from_str(&merged_json).unwrap();
        // Should have 3 bars (deduped on timestamp, newer wins via dedup_by which keeps first)
        assert_eq!(merged.len(), 3);
    }

    #[test]
    fn sqlite_cache_get_bars_tail() {
        let db_path = temp_db_path();
        let cache = SqliteCache::open(&db_path).unwrap();

        let json = r#"[
            {"timestamp":"2024-01-01T00:00:00+00:00","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":100.0},
            {"timestamp":"2024-01-02T00:00:00+00:00","open":2.0,"high":3.0,"low":1.5,"close":2.5,"volume":200.0},
            {"timestamp":"2024-01-03T00:00:00+00:00","open":3.0,"high":4.0,"low":2.5,"close":3.5,"volume":300.0}
        ]"#;
        cache.put_bars("TAIL:1D", json).unwrap();

        let result = cache.get_bars_tail("TAIL:1D", 1).unwrap().unwrap();
        let bars: Vec<serde_json::Value> = serde_json::from_str(&result.0).unwrap();
        assert_eq!(bars.len(), 1);
        assert_eq!(bars[0]["open"].as_f64().unwrap(), 3.0);
    }

    #[test]
    fn sqlite_cache_incremental_start() {
        let db_path = temp_db_path();
        let cache = SqliteCache::open(&db_path).unwrap();

        let json = r#"[
            {"timestamp":"2024-01-01T00:00:00+00:00","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":100.0},
            {"timestamp":"2024-01-02T00:00:00+00:00","open":2.0,"high":3.0,"low":1.5,"close":2.5,"volume":200.0},
            {"timestamp":"2024-01-03T00:00:00+00:00","open":3.0,"high":4.0,"low":2.5,"close":3.5,"volume":300.0}
        ]"#;
        cache.put_bars("INC:1D", json).unwrap();

        let result = cache.get_incremental_start("INC:1D").unwrap();
        assert!(result.is_some());
        let (ts, count) = result.unwrap();
        assert_eq!(count, 3);
        // Should be the second-to-last bar's timestamp
        let dt = chrono::DateTime::parse_from_rfc3339(&ts).unwrap();
        assert_eq!(dt.format("%Y-%m-%d").to_string(), "2024-01-02");
    }

    #[test]
    fn sqlite_cache_list_kv_keys() {
        let db_path = temp_db_path();
        let cache = SqliteCache::open(&db_path).unwrap();

        cache.put_kv("cred:alpaca", "{}").unwrap();
        cache.put_kv("cred:darwinex", "{}").unwrap();
        cache.put_kv("other:thing", "{}").unwrap();

        let keys = cache.list_kv_keys("cred:").unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"cred:alpaca".to_string()));
        assert!(keys.contains(&"cred:darwinex".to_string()));
    }

    #[test]
    fn sqlite_cache_delete_symbol() {
        let db_path = temp_db_path();
        let cache = SqliteCache::open(&db_path).unwrap();

        let json = r#"[{"timestamp":"2024-01-01T00:00:00+00:00","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":100.0}]"#;
        cache.put_bars("AAPL:1Hour", json).unwrap();
        cache.put_bars("AAPL:1Day", json).unwrap();
        cache.put_bars("MSFT:1Hour", json).unwrap();

        let deleted = cache.delete_symbol("AAPL").unwrap();
        assert_eq!(deleted, 2);
        assert!(cache.get_bars("AAPL:1Hour").unwrap().is_none());
        assert!(cache.get_bars("MSFT:1Hour").unwrap().is_some());
    }

    #[test]
    fn search_keys_finds_partial_matches() {
        let db_path = temp_db_path();
        let cache = SqliteCache::open(&db_path).unwrap();
        let json = r#"[{"timestamp":"2024-01-01T00:00:00+00:00","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":100.0}]"#;
        cache.put_bars("mt5:EURUSD:1Hour", json).unwrap();
        cache.put_bars("alpaca:AAPL:1Day", json).unwrap();
        cache.put_bars("kraken:BTCUSD:5Min", json).unwrap();

        let eur = cache.search_keys("EURUSD", 10).unwrap();
        assert_eq!(eur.len(), 1);
        assert_eq!(eur[0], "mt5:EURUSD:1Hour");

        // Case-insensitive
        let eur_lower = cache.search_keys("eurusd", 10).unwrap();
        assert_eq!(eur_lower.len(), 1);

        // Limit respected
        let all = cache.search_keys(":", 2).unwrap();
        assert!(all.len() <= 2);
    }

    #[test]
    fn search_keys_returns_empty_on_no_match() {
        let db_path = temp_db_path();
        let cache = SqliteCache::open(&db_path).unwrap();
        let result = cache.search_keys("DOESNOTEXIST", 10).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn queue_append_and_drain_in_order() {
        let db_path = temp_db_path();
        let cache = SqliteCache::open(&db_path).unwrap();

        cache.append_to_queue("lan:test_queue", r#"{"cmd":"A"}"#).unwrap();
        cache.append_to_queue("lan:test_queue", r#"{"cmd":"B"}"#).unwrap();
        cache.append_to_queue("lan:test_queue", r#"{"cmd":"C"}"#).unwrap();

        let drained = cache.drain_queue("lan:test_queue").unwrap();
        assert_eq!(drained.len(), 3);
        // Order by timestamp/seq — monotonic
        assert_eq!(drained[0], r#"{"cmd":"A"}"#);
        assert_eq!(drained[1], r#"{"cmd":"B"}"#);
        assert_eq!(drained[2], r#"{"cmd":"C"}"#);

        // Second drain returns empty — drain deletes
        let drained2 = cache.drain_queue("lan:test_queue").unwrap();
        assert!(drained2.is_empty());
    }

    #[test]
    fn queue_isolates_by_prefix() {
        let db_path = temp_db_path();
        let cache = SqliteCache::open(&db_path).unwrap();
        cache.append_to_queue("q1", "one").unwrap();
        cache.append_to_queue("q2", "two").unwrap();
        cache.append_to_queue("q1", "three").unwrap();

        let q1 = cache.drain_queue("q1").unwrap();
        assert_eq!(q1.len(), 2);
        assert!(q1.contains(&"one".to_string()));
        assert!(q1.contains(&"three".to_string()));

        let q2 = cache.drain_queue("q2").unwrap();
        assert_eq!(q2, vec!["two".to_string()]);
    }

    #[test]
    fn get_kv_raw_returns_compressed_blob() {
        let db_path = temp_db_path();
        let cache = SqliteCache::open(&db_path).unwrap();
        let payload = r#"{"hello":"world"}"#;
        cache.put_kv("test:kv", payload).unwrap();

        let raw = cache.get_kv_raw("test:kv").unwrap().unwrap();
        // Blob is zstd-compressed — decompress should roundtrip
        let decompressed = zstd::decode_all(raw.0.as_slice()).unwrap();
        assert_eq!(String::from_utf8(decompressed).unwrap(), payload);
        assert!(raw.1 > 0, "timestamp should be populated");

        let missing = cache.get_kv_raw("missing:key").unwrap();
        assert!(missing.is_none());
    }
}
