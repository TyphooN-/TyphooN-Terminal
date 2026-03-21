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
    let count = u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize;
    let expected = 8 + count * BYTES_PER_BAR;
    if data.len() < expected {
        return Err(format!("Binary data truncated: expected {expected}, got {}", data.len()));
    }

    let mut bars = Vec::with_capacity(count);
    for i in 0..count {
        let offset = 8 + i * BYTES_PER_BAR;
        let ts_ms = i64::from_le_bytes(data[offset..offset+8].try_into().unwrap());
        let open = f64::from_le_bytes(data[offset+8..offset+16].try_into().unwrap());
        let high = f64::from_le_bytes(data[offset+16..offset+24].try_into().unwrap());
        let low = f64::from_le_bytes(data[offset+24..offset+32].try_into().unwrap());
        let close = f64::from_le_bytes(data[offset+32..offset+40].try_into().unwrap());
        let volume = f64::from_le_bytes(data[offset+40..offset+48].try_into().unwrap());

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

        Ok(Self { conn: Mutex::new(conn) })
    }

    /// Store bar data in packed binary format + zstd compression.
    /// Binary format is ~3-5x smaller than JSON before compression.
    /// Uses zstd level 9 for persistent storage (2-3x better ratio than level 3,
    /// ~10ms overhead per 10K bars — acceptable for background writes).
    pub fn put_bars(&self, key: &str, json_data: &str) -> Result<(), String> {
        let binary = pack_bars(json_data)?;
        let bar_count = u32::from_le_bytes(binary[4..8].try_into().unwrap()) as i64;
        let compressed = zstd::encode_all(binary.as_slice(), 9)
            .map_err(|e| format!("zstd compress failed: {e}"))?;
        let timestamp = chrono::Utc::now().timestamp();

        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        conn.execute(
            "INSERT OR REPLACE INTO bar_cache (key, data, timestamp, bar_count) VALUES (?1, ?2, ?3, ?4)",
            params![key, compressed, timestamp, bar_count],
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
        let mut stmt = conn.prepare_cached(
            "SELECT data FROM bar_cache WHERE key = ?1"
        ).map_err(|e| format!("SQLite prepare failed: {e}"))?;

        let result = stmt.query_row(rusqlite::params![key], |row| {
            let data: Vec<u8> = row.get(0)?;
            Ok(data)
        });

        match result {
            Ok(compressed) => {
                let decompressed = zstd::decode_all(compressed.as_slice())
                    .map_err(|e| format!("zstd decompress failed: {e}"))?;
                if decompressed.len() >= 8 && &decompressed[0..4] == BAR_BINARY_MAGIC {
                    let count = u32::from_le_bytes(decompressed[4..8].try_into().unwrap()) as usize;
                    if count < 2 { return Ok(None); }
                    // Second-to-last bar — so we re-fetch the live candle
                    let target_offset = 8 + (count - 2) * BYTES_PER_BAR;
                    if decompressed.len() < target_offset + 8 { return Ok(None); }
                    let ts_ms = i64::from_le_bytes(decompressed[target_offset..target_offset+8].try_into().unwrap());
                    let dt = chrono::DateTime::from_timestamp_millis(ts_ms).unwrap_or_default();
                    Ok(Some((dt.to_rfc3339(), count)))
                } else {
                    // Legacy JSON — parse second-to-last element
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
        let bar_count = u32::from_le_bytes(binary[4..8].try_into().unwrap()) as i64;
        let compressed = zstd::encode_all(binary.as_slice(), 3)
            .map_err(|e| format!("zstd compress failed: {e}"))?;
        let timestamp = chrono::Utc::now().timestamp();

        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        conn.execute(
            "INSERT OR REPLACE INTO bar_cache (key, data, timestamp, bar_count) VALUES (?1, ?2, ?3, ?4)",
            params![key, compressed, timestamp, bar_count],
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

    /// Delete all cache entries matching a symbol prefix (e.g., "AAPL:" deletes all TFs for AAPL).
    pub fn delete_symbol(&self, symbol_prefix: &str) -> Result<u64, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        let pattern = format!("{}:%", symbol_prefix);
        let deleted = conn.execute(
            "DELETE FROM bar_cache WHERE key LIKE ?1", params![pattern]
        ).map_err(|e| format!("SQLite delete failed: {e}"))? as u64;
        Ok(deleted)
    }
}
