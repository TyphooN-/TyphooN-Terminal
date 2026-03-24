//! SQLite-backed cache for unlimited structured storage.
//!
//! Replaces IndexedDB's ~50MB limit with SQLite (no practical limit).
//! Bar data uses packed binary format (44 bytes/bar) + zstd compression.
//! KV data uses JSON + zstd compression.
//! Binary format: [u32 bar_count][per bar: i64 timestamp_ms, f64 OHLCV]

use rusqlite::{Connection, params};
use std::path::PathBuf;
use std::sync::Mutex;

/// Magic bytes to identify binary bar format (vs legacy JSON).
const BAR_BINARY_MAGIC: &[u8; 4] = b"TTBR"; // TyphooN Terminal Bar Record
/// Bytes per bar in binary format: i64 timestamp + 5×f64 (OHLCV) = 48 bytes
const BYTES_PER_BAR: usize = 8 + 5 * 8; // 48

/// Pack bars from JSON into binary format for efficient storage.
/// Format: [4-byte magic][u32 count][per bar: i64 ts_ms, f64 O, f64 H, f64 L, f64 C, f64 V]
fn pack_bars(json_data: &str) -> Result<Vec<u8>, String> {
    let bars: Vec<serde_json::Value> = serde_json::from_str(json_data)
        .map_err(|e| format!("JSON parse failed: {e}"))?;
    let count = bars.len() as u32;
    let mut buf = Vec::with_capacity(4 + 4 + bars.len() * BYTES_PER_BAR);
    buf.extend_from_slice(BAR_BINARY_MAGIC);
    buf.extend_from_slice(&count.to_le_bytes());
    for bar in &bars {
        // Parse timestamp string to epoch milliseconds
        let ts_str = bar["timestamp"].as_str().unwrap_or("");
        let ts_ms = chrono::DateTime::parse_from_rfc3339(ts_str)
            .map(|dt| dt.timestamp_millis())
            .unwrap_or(0i64);
        buf.extend_from_slice(&ts_ms.to_le_bytes());
        buf.extend_from_slice(&bar["open"].as_f64().unwrap_or(0.0).to_le_bytes());
        buf.extend_from_slice(&bar["high"].as_f64().unwrap_or(0.0).to_le_bytes());
        buf.extend_from_slice(&bar["low"].as_f64().unwrap_or(0.0).to_le_bytes());
        buf.extend_from_slice(&bar["close"].as_f64().unwrap_or(0.0).to_le_bytes());
        buf.extend_from_slice(&bar["volume"].as_f64().unwrap_or(0.0).to_le_bytes());
    }
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
    let expected = 8 + count * BYTES_PER_BAR;
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
    let expected = 8 + count * BYTES_PER_BAR;
    if data.len() < expected {
        return Err(format!("Binary data truncated: expected {expected}, got {}", data.len()));
    }
    let mut bars = Vec::with_capacity(count);
    for i in 0..count {
        let off = 8 + i * BYTES_PER_BAR;
        let ts = i64::from_le_bytes(data[off..off+8].try_into().unwrap());
        let o = f64::from_le_bytes(data[off+8..off+16].try_into().unwrap());
        let h = f64::from_le_bytes(data[off+16..off+24].try_into().unwrap());
        let l = f64::from_le_bytes(data[off+24..off+32].try_into().unwrap());
        let c = f64::from_le_bytes(data[off+32..off+40].try_into().unwrap());
        let v = f64::from_le_bytes(data[off+40..off+48].try_into().unwrap());
        bars.push((ts, o, h, l, c, v));
    }
    Ok(bars)
}

/// Extract last and second-to-last bar timestamps from binary data (for metadata columns).
/// Returns (second_last_ts_rfc3339, last_ts_rfc3339) or empty strings if not enough bars.
fn extract_tail_timestamps(binary: &[u8], count: usize) -> (Option<String>, Option<String>) {
    if count < 2 || binary.len() < 8 + count * BYTES_PER_BAR {
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
    let expected = 8 + count * BYTES_PER_BAR;
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
pub struct SqliteCache {
    conn: Mutex<Connection>,
}

impl SqliteCache {
    /// Open or create a SQLite database at the given path.
    pub fn open(path: &PathBuf) -> Result<Self, String> {
        let conn = Connection::open(path)
            .map_err(|e| format!("SQLite open failed: {e}"))?;

        // WAL mode for concurrent reads + single writer performance
        // mmap_size=256MB for memory-mapped I/O (faster reads, OS manages pages)
        conn.execute_batch("
            PRAGMA journal_mode=WAL;
            PRAGMA synchronous=NORMAL;
            PRAGMA cache_size=-64000;
            PRAGMA temp_store=MEMORY;
            PRAGMA mmap_size=268435456;
            PRAGMA auto_vacuum=INCREMENTAL;
        ").map_err(|e| format!("SQLite pragma failed: {e}"))?;

        // Create tables
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS bar_cache (
                key TEXT PRIMARY KEY,
                data BLOB NOT NULL,
                timestamp INTEGER NOT NULL,
                bar_count INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS kv_cache (
                key TEXT PRIMARY KEY,
                value BLOB NOT NULL,
                timestamp INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_bar_cache_ts ON bar_cache(timestamp);
            CREATE INDEX IF NOT EXISTS idx_bar_meta ON bar_cache(key, timestamp, bar_count);
            CREATE INDEX IF NOT EXISTS idx_kv_cache_ts ON kv_cache(timestamp);
        ").map_err(|e| format!("SQLite create tables failed: {e}"))?;

        // Schema migration: add last_ts column for fast incremental start lookup
        // (avoids decompressing full binary blob just to read 2 timestamps)
        let _ = conn.execute("ALTER TABLE bar_cache ADD COLUMN last_ts TEXT", []);
        let _ = conn.execute("ALTER TABLE bar_cache ADD COLUMN second_last_ts TEXT", []);

        Ok(Self { conn: Mutex::new(conn) })
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
        // Extract last and second-to-last timestamps for fast incremental start lookup
        let (second_last_ts, last_ts) = extract_tail_timestamps(&binary, bar_count as usize);
        let compressed = zstd::encode_all(binary.as_slice(), 3)
            .map_err(|e| format!("zstd compress failed: {e}"))?;
        let timestamp = chrono::Utc::now().timestamp();

        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        conn.execute(
            "INSERT OR REPLACE INTO bar_cache (key, data, timestamp, bar_count, last_ts, second_last_ts) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![key, compressed, timestamp, bar_count, last_ts, second_last_ts],
        ).map_err(|e| format!("SQLite insert failed: {e}"))?;
        Ok(())
    }

    /// Load bar data — handles both binary (new) and JSON (legacy) formats.
    pub fn get_bars(&self, key: &str) -> Result<Option<(String, i64)>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
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
                let decompressed = zstd::decode_all(compressed.as_slice())
                    .map_err(|e| format!("zstd decompress failed: {e}"))?;
                // Detect format: binary starts with TTBR magic, legacy is UTF-8 JSON
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
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        let mut stmt = conn.prepare_cached("SELECT data FROM bar_cache WHERE key = ?1")
            .map_err(|e| format!("Prepare failed: {e}"))?;
        let row: Option<Vec<u8>> = stmt.query_row(rusqlite::params![key], |r| r.get(0)).ok();
        match row {
            None => Ok(None),
            Some(compressed) => {
                let decompressed = zstd::decode_all(compressed.as_slice())
                    .map_err(|e| format!("Decompress failed: {e}"))?;
                if decompressed.len() >= 4 && &decompressed[0..4] == BAR_BINARY_MAGIC {
                    Ok(Some(unpack_bars_raw(&decompressed)?))
                } else {
                    let text = String::from_utf8_lossy(&decompressed);
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
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
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
                let decompressed = zstd::decode_all(compressed.as_slice())
                    .map_err(|e| format!("zstd decompress failed: {e}"))?;
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

    /// Load key-value data.
    pub fn get_kv(&self, key: &str) -> Result<Option<String>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
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
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        let bar_count: i64 = conn.query_row("SELECT COUNT(*) FROM bar_cache", [], |r| r.get(0))
            .unwrap_or(0);
        let kv_count: i64 = conn.query_row("SELECT COUNT(*) FROM kv_cache", [], |r| r.get(0))
            .unwrap_or(0);
        let total_size: i64 = conn.query_row(
            "SELECT COALESCE(SUM(LENGTH(data)),0) FROM bar_cache", [], |r| r.get(0)
        ).unwrap_or(0);
        Ok((bar_count, kv_count, total_size))
    }

    /// Get detailed per-key cache stats: returns JSON array of {key, compressed_bytes, timestamp}.
    /// Keys are "symbol:timeframe" format (e.g., "AAPL:1Hour").
    pub fn detailed_stats(&self) -> Result<Vec<(String, i64, i64)>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        let mut stmt = conn.prepare(
            "SELECT key, LENGTH(data) as size, timestamp FROM bar_cache ORDER BY key"
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
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;

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
                    let target_offset = 8 + (bc - 2) * BYTES_PER_BAR;
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

        // Merge and deduplicate by timestamp
        all_bars.extend(new_bars);
        all_bars.sort_by(|a, b| {
            let ta = a["timestamp"].as_str().unwrap_or("");
            let tb = b["timestamp"].as_str().unwrap_or("");
            ta.cmp(tb)
        });
        all_bars.dedup_by(|a, b| {
            a["timestamp"].as_str() == b["timestamp"].as_str()
        });

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
        let (second_last_ts, last_ts) = extract_tail_timestamps(&binary, bar_count as usize);
        let compressed = zstd::encode_all(binary.as_slice(), 3)
            .map_err(|e| format!("zstd compress failed: {e}"))?;
        let timestamp = chrono::Utc::now().timestamp();

        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        conn.execute(
            "INSERT OR REPLACE INTO bar_cache (key, data, timestamp, bar_count, last_ts, second_last_ts) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![key, compressed, timestamp, bar_count, last_ts, second_last_ts],
        ).map_err(|e| format!("SQLite insert failed: {e}"))?;
        Ok(())
    }

    /// Get cache timestamp (when bars were last stored) for a key.
    /// Returns None if key doesn't exist.
    pub fn get_cache_age_secs(&self, key: &str) -> Result<Option<i64>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
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
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
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
                "INSERT OR REPLACE INTO bar_cache (key, data, timestamp, bar_count) VALUES (?1, ?2, ?3, ?4)",
                params![key, compressed, timestamp, bar_count],
            ) {
                Ok(_) => count += 1,
                Err(e) => tracing::warn!("Batch write skip {}: {}", key, e),
            }
        }
        conn.execute_batch("COMMIT").map_err(|e| format!("COMMIT failed: {e}"))?;
        Ok(count)
    }

    /// Bulk-load cache metadata (age_secs, bar_count) for all entries.
    /// Returns HashMap<key, (age_secs, bar_count)> — one query instead of N individual lookups.
    pub fn get_all_cache_meta(&self) -> Result<std::collections::HashMap<String, (i64, i64)>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
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
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;

        // Use SQLite's VACUUM INTO to create a consistent snapshot
        let backup_path = format!("{}.tmp", path);
        conn.execute("VACUUM INTO ?1", [&backup_path])
            .map_err(|e| format!("VACUUM INTO failed: {e}"))?;

        // Read the temp file and compress with zstd level 9
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

        // Write to temp file
        let tmp_path = format!("{}.import.tmp", path);
        std::fs::write(&tmp_path, &data)
            .map_err(|e| format!("Write temp failed: {e}"))?;

        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;

        // Attach the backup DB
        conn.execute("ATTACH DATABASE ?1 AS backup_db", [&tmp_path])
            .map_err(|e| {
                let _ = std::fs::remove_file(&tmp_path);
                format!("Attach failed: {e}")
            })?;

        // Merge bar_cache: import entries where backup has newer timestamp or key doesn't exist
        let bar_count = conn.execute(
            "INSERT OR REPLACE INTO bar_cache (key, data, timestamp, bar_count)
             SELECT b.key, b.data, b.timestamp, b.bar_count
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

    /// Get a lock on the underlying connection for direct SQL operations.
    /// Used by darwin import to run table creation and batch inserts.
    pub fn connection(&self) -> Result<std::sync::MutexGuard<'_, Connection>, String> {
        self.conn.lock().map_err(|e| format!("Lock failed: {e}"))
    }

    /// List all kv_cache keys matching a prefix (e.g., "cred:" returns all credential keys).
    /// LIKE wildcards in the prefix are escaped to prevent overly broad matches.
    pub fn list_kv_keys(&self, prefix: &str) -> Result<Vec<String>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
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

    /// Get raw bar cache entry without decompression (for LAN sync transfer).
    /// Returns the compressed blob and its timestamp as stored in SQLite.
    pub fn get_raw_bar_entry(&self, key: &str) -> Result<Option<(Vec<u8>, i64)>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
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
            "INSERT OR REPLACE INTO bar_cache (key, data, timestamp, bar_count) VALUES (?1, ?2, ?3, ?4)",
            params![key, data, timestamp, bar_count],
        ).map_err(|e| format!("SQLite insert failed: {e}"))?;
        Ok(())
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
}
