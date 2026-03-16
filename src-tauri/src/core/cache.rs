//! SQLite-backed cache for unlimited structured storage.
//!
//! Replaces IndexedDB's ~50MB limit with SQLite (no practical limit).
//! Stores bar data, fundamentals, news, and any JSON-serializable data.
//! Compressed with zstd before storage for ~10x space savings.

use rusqlite::{Connection, params};
use std::path::PathBuf;
use std::sync::Mutex;

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
        conn.execute_batch("
            PRAGMA journal_mode=WAL;
            PRAGMA synchronous=NORMAL;
            PRAGMA cache_size=-64000;
            PRAGMA temp_store=MEMORY;
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
            CREATE INDEX IF NOT EXISTS idx_kv_cache_ts ON kv_cache(timestamp);
        ").map_err(|e| format!("SQLite create tables failed: {e}"))?;

        Ok(Self { conn: Mutex::new(conn) })
    }

    /// Store compressed bar data.
    pub fn put_bars(&self, key: &str, json_data: &str) -> Result<(), String> {
        let compressed = zstd::encode_all(json_data.as_bytes(), 3)
            .map_err(|e| format!("zstd compress failed: {e}"))?;
        let timestamp = chrono::Utc::now().timestamp();
        let bar_count = json_data.matches("\"timestamp\"").count() as i64;

        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        conn.execute(
            "INSERT OR REPLACE INTO bar_cache (key, data, timestamp, bar_count) VALUES (?1, ?2, ?3, ?4)",
            params![key, compressed, timestamp, bar_count],
        ).map_err(|e| format!("SQLite insert failed: {e}"))?;
        Ok(())
    }

    /// Load compressed bar data.
    pub fn get_bars(&self, key: &str) -> Result<Option<(String, i64)>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        let mut stmt = conn.prepare(
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
                let json = String::from_utf8(decompressed)
                    .map_err(|e| format!("UTF-8 decode failed: {e}"))?;
                Ok(Some((json, timestamp)))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(format!("SQLite query failed: {e}")),
        }
    }

    /// Store key-value data (fundamentals, news, etc.).
    pub fn put_kv(&self, key: &str, json_data: &str) -> Result<(), String> {
        let compressed = zstd::encode_all(json_data.as_bytes(), 3)
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
        let mut stmt = conn.prepare(
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
    pub fn stats(&self) -> Result<(u64, u64, u64), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        let bar_count: u64 = conn.query_row("SELECT COUNT(*) FROM bar_cache", [], |r| r.get(0))
            .unwrap_or(0);
        let kv_count: u64 = conn.query_row("SELECT COUNT(*) FROM kv_cache", [], |r| r.get(0))
            .unwrap_or(0);
        let total_size: u64 = conn.query_row(
            "SELECT COALESCE(SUM(LENGTH(data)),0) FROM bar_cache", [], |r| r.get(0)
        ).unwrap_or(0);
        Ok((bar_count, kv_count, total_size))
    }
}
