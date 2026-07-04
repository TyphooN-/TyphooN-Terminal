use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

struct BarZstdLevelGuard(i32);

impl BarZstdLevelGuard {
    fn set(level: i32) -> Self {
        let previous = bar_zstd_level();
        set_bar_zstd_level(level);
        Self(previous)
    }
}

impl Drop for BarZstdLevelGuard {
    fn drop(&mut self) {
        set_bar_zstd_level(self.0);
    }
}

/// Monotonic counter for unique temp DB paths across parallel tests.
static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Helper: unique temp DB path per test invocation (no external crate needed).
fn temp_db_path() -> PathBuf {
    let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    std::env::temp_dir().join(format!("typhoon_cache_test_{}_{}.db", pid, id))
}

#[serial_test::serial]
#[test]
fn live_bar_writes_use_user_selected_zstd_level() {
    let _guard = BarZstdLevelGuard::set(22);
    let db_path = temp_db_path();
    let cache = SqliteCache::open(&db_path).unwrap();
    let bars = r#"[{"timestamp":"2024-01-01T00:00:00+00:00","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":10.0}]"#;

    cache.put_bars("alpaca:AAPL:1Day", bars).unwrap();
    let level: i32 = cache
        .conn
        .lock()
        .unwrap()
        .query_row(
            "SELECT zstd_level FROM bar_cache WHERE key = ?1",
            params!["alpaca:AAPL:1Day"],
            |row| row.get::<_, i32>(0),
        )
        .unwrap();

    assert_eq!(level, 22);
    let _ = std::fs::remove_file(db_path);
}

#[serial_test::serial]
#[test]
fn ws_fast_merge_writes_user_selected_zstd_level() {
    let _guard = BarZstdLevelGuard::set(22);
    let db_path = temp_db_path();
    let cache = SqliteCache::open(&db_path).unwrap();
    let bars = r#"[{"timestamp":"2024-01-01T00:00:00+00:00","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":10.0}]"#;

    cache.merge_bars_fast("kraken:BTCUSD:1Hour", bars, 0).unwrap();
    let level: i32 = cache
        .conn
        .lock()
        .unwrap()
        .query_row(
            "SELECT zstd_level FROM bar_cache WHERE key = ?1",
            params!["kraken:BTCUSD:1Hour"],
            |row| row.get::<_, i32>(0),
        )
        .unwrap();

    assert_eq!(level, 22);
    let _ = std::fs::remove_file(db_path);
}

#[test]
fn purge_bars_for_source_timeframes_only_removes_matching_source_and_tf() {
    let db_path = temp_db_path();
    let cache = SqliteCache::open(&db_path).unwrap();
    let bars = r#"[{"timestamp":"2024-01-01T00:00:00+00:00","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":10.0}]"#;
    cache.put_bars("yahoo-chart:AAPL:15Min", bars).unwrap();
    cache.put_bars("yahoo-chart:AAPL:1Hour", bars).unwrap();
    cache.put_bars("yahoo-chart:AAPL:1Day", bars).unwrap(); // keep — not an intraday tf
    cache.put_bars("alpaca:AAPL:15Min", bars).unwrap(); // keep — different source

    let n = cache
        .purge_bars_for_source_timeframes("yahoo-chart", &["15Min", "30Min", "1Hour"])
        .unwrap();
    assert_eq!(n, 2); // 15Min + 1Hour existed; 30Min did not

    assert!(cache.get_bars_raw("yahoo-chart:AAPL:15Min").unwrap().is_none());
    assert!(cache.get_bars_raw("yahoo-chart:AAPL:1Hour").unwrap().is_none());
    assert!(cache.get_bars_raw("yahoo-chart:AAPL:1Day").unwrap().is_some());
    assert!(cache.get_bars_raw("alpaca:AAPL:15Min").unwrap().is_some());
    let _ = std::fs::remove_file(db_path);
}

#[test]
fn delete_equity_bar_cache_for_symbol_clears_provider_and_merged_rows() {
    let db_path = temp_db_path();
    let cache = SqliteCache::open(&db_path).unwrap();
    let bars = r#"[{"timestamp":"2024-01-01T00:00:00+00:00","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":10.0}]"#;

    for key in [
        "merged:WOK:1Day",
        "kraken-equities:WOK:1Day",
        "alpaca:WOK.EQ:1Day",
        "yahoo-chart:wok:1Day",
    ] {
        cache.put_bars(key, bars).unwrap();
    }
    cache.put_bars("alpaca:AAPL:1Day", bars).unwrap();
    cache.put_kv("alpaca:WOK:meta", "keep").unwrap();

    let deleted = cache.delete_equity_bar_cache_for_symbol("WOK.EQ").unwrap();
    assert_eq!(deleted, 4);
    assert!(cache.get_bars("merged:WOK:1Day").unwrap().is_none());
    assert!(
        cache
            .get_bars("kraken-equities:WOK:1Day")
            .unwrap()
            .is_none()
    );
    assert!(cache.get_bars("alpaca:WOK.EQ:1Day").unwrap().is_none());
    assert!(cache.get_bars("yahoo-chart:wok:1Day").unwrap().is_none());
    assert!(cache.get_bars("alpaca:AAPL:1Day").unwrap().is_some());
    assert_eq!(
        cache.get_kv("alpaca:WOK:meta").unwrap().as_deref(),
        Some("keep")
    );

    let _ = std::fs::remove_file(db_path);
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
fn pack_bars_for_key_normalizes_daily_weekly_monthly_sessions() {
    let daily = r#"[
            {"timestamp":"2026-05-28T04:00:00+00:00","open":14.0,"high":15.0,"low":13.0,"close":14.5,"volume":100.0},
            {"timestamp":"2026-05-28T20:00:00+00:00","open":14.1,"high":15.5,"low":13.5,"close":15.0,"volume":200.0}
        ]"#;
    let raw = unpack_bars_raw(&pack_bars_for_key("alpaca:TNDM:1Day", daily).unwrap()).unwrap();
    assert_eq!(raw.len(), 1);
    assert_eq!(raw[0].0, 1_779_926_400_000);
    assert_eq!(raw[0].4, 15.0);

    let weekly = r#"[
            {"timestamp":"2026-05-25T04:00:00+00:00","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":100.0}
        ]"#;
    let raw = unpack_bars_raw(&pack_bars_for_key("alpaca:TNDM:1Week", weekly).unwrap()).unwrap();
    assert_eq!(raw[0].0, 1_779_667_200_000);

    let monthly = r#"[
            {"timestamp":"2026-05-04T04:00:00+00:00","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":100.0}
        ]"#;
    let raw = unpack_bars_raw(&pack_bars_for_key("alpaca:TNDM:1Month", monthly).unwrap()).unwrap();
    assert_eq!(raw[0].0, 1_777_593_600_000);
}

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
    let last_dt = chrono::DateTime::parse_from_rfc3339(&last.unwrap()).unwrap();
    assert_eq!(last_dt.timestamp_millis(), 1_705_000_000_000);
}

#[test]
fn sqlite_cache_single_bar_rows_persist_last_timestamp_metadata() {
    let db_path = temp_db_path();
    let cache = SqliteCache::open(&db_path).unwrap();
    let json = r#"[{"timestamp":"2024-06-01T00:00:00+00:00","open":50.0,"high":55.0,"low":49.0,"close":53.0,"volume":1000.0}]"#;

    cache.put_bars("yahoo-chart:ONE:1Month", json).unwrap();
    let (last_ts, second_last_ts): (Option<String>, Option<String>) = cache
        .conn
        .lock()
        .unwrap()
        .query_row(
            "SELECT last_ts, second_last_ts FROM bar_cache WHERE key = ?1",
            params!["yahoo-chart:ONE:1Month"],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();

    assert_eq!(last_ts.as_deref(), Some("2024-06-01T00:00:00+00:00"));
    assert!(second_last_ts.is_none());
    let _ = std::fs::remove_file(db_path);
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
    let bars = vec![(0, 1.0, 2.0, 3.0, 4.0, 5.0), (1, 6.0, 7.0, 8.0, 9.0, 10.0)];
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
    assert_eq!(raw[0].1, 1.1); // open
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
fn sqlite_cache_timestamp_range_binary_blob() {
    let db_path = temp_db_path();
    let cache = SqliteCache::open(&db_path).unwrap();
    let json = r#"[
            {"timestamp":"2024-01-01T00:00:00+00:00","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":100.0},
            {"timestamp":"2024-01-02T00:00:00+00:00","open":2.0,"high":3.0,"low":1.5,"close":2.5,"volume":200.0}
        ]"#;
    cache.put_bars("RANGE:1D", json).unwrap();

    let conn = cache.read_connection().unwrap();
    let range = SqliteCache::get_bar_timestamp_range_with_conn(&conn, "RANGE:1D");
    assert_eq!(range, Some((1_704_067_200_000, 1_704_153_600_000)));
}

#[test]
fn sqlite_cache_timestamp_range_text_typed_ttbr_blob() {
    let db_path = temp_db_path();
    let cache = SqliteCache::open(&db_path).unwrap();
    let bars = vec![
        (1_704_067_200_000, 1.0, 2.0, 0.5, 1.5, 100.0),
        (1_704_153_600_000, 2.0, 3.0, 1.5, 2.5, 200.0),
    ];
    let raw = make_binary_bars(&bars);

    let conn = cache.connection().unwrap();
    conn.execute(
        "INSERT INTO bar_cache (key, data, timestamp, bar_count, zstd_level)
             VALUES (?1, CAST(?2 AS TEXT), ?3, ?4, ?5)",
        params!["TEXTTTBR:1D", raw, 1_704_153_600i64, bars.len() as i64, 3],
    )
    .unwrap();
    let ty: String = conn
        .query_row(
            "SELECT typeof(data) FROM bar_cache WHERE key = ?1",
            params!["TEXTTTBR:1D"],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(ty, "text");

    let range = SqliteCache::get_bar_timestamp_range_with_conn(&conn, "TEXTTTBR:1D");
    assert_eq!(range, Some((1_704_067_200_000, 1_704_153_600_000)));
}

#[test]
fn sqlite_cache_timestamp_range_legacy_json_blob() {
    let db_path = temp_db_path();
    let cache = SqliteCache::open(&db_path).unwrap();
    let json = r#"[
            {"timestamp":"2024-01-01T00:00:00+00:00","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":100.0},
            {"timestamp":"2024-01-02T00:00:00+00:00","open":2.0,"high":3.0,"low":1.5,"close":2.5,"volume":200.0}
        ]"#;
    let compressed = zstd::encode_all(json.as_bytes(), 9).unwrap();

    let conn = cache.connection().unwrap();
    conn.execute(
        "INSERT INTO bar_cache (key, data, timestamp, bar_count, zstd_level)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        params!["LEGACY:1D", compressed, 1_704_153_600i64, 2i64, 3],
    )
    .unwrap();

    let range = SqliteCache::get_bar_timestamp_range_with_conn(&conn, "LEGACY:1D");
    assert_eq!(range, Some((1_704_067_200_000, 1_704_153_600_000)));
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
    cache.put_kv("cred:kraken", "{}").unwrap();
    cache.put_kv("other:thing", "{}").unwrap();

    let keys = cache.list_kv_keys("cred:").unwrap();
    assert_eq!(keys.len(), 2);
    assert!(keys.contains(&"cred:alpaca".to_string()));
    assert!(keys.contains(&"cred:kraken".to_string()));
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
fn sqlite_cache_delete_timeframe_across_brokers() {
    let db_path = temp_db_path();
    let cache = SqliteCache::open(&db_path).unwrap();

    let json = r#"[{"timestamp":"2024-01-01T00:00:00+00:00","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":100.0}]"#;
    cache.put_bars("kraken-futures:EURUSD:1Min", json).unwrap();
    cache.put_bars("alpaca:AAPL:1Min", json).unwrap();
    cache.put_bars("kraken:BTCUSD:1Min", json).unwrap();
    cache.put_bars("kraken-futures:EURUSD:1Hour", json).unwrap();

    let deleted = cache.delete_timeframe("M1").unwrap();
    assert_eq!(deleted, 3);
    assert!(
        cache
            .get_bars("kraken-futures:EURUSD:1Min")
            .unwrap()
            .is_none()
    );
    assert!(cache.get_bars("alpaca:AAPL:1Min").unwrap().is_none());
    assert!(cache.get_bars("kraken:BTCUSD:1Min").unwrap().is_none());
    assert!(
        cache
            .get_bars("kraken-futures:EURUSD:1Hour")
            .unwrap()
            .is_some()
    );
}

#[test]
fn sqlite_cache_reclaim_space_shrinks_after_prior_delete() {
    let db_path = temp_db_path();
    let cache = SqliteCache::open(&db_path).unwrap();
    {
        let conn = cache.connection().unwrap();
        conn.execute(
                "INSERT INTO bar_cache (key, data, timestamp, bar_count, zstd_level) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    "kraken-futures:EURUSD:1Min",
                    vec![0xABu8; 2_000_000],
                    1i64,
                    1000i64,
                    3i64
                ],
            )
            .unwrap();
        conn.execute(
                "INSERT INTO bar_cache (key, data, timestamp, bar_count, zstd_level) VALUES (?1, ?2, ?3, ?4, ?5)",
                params!["keep:1Day", vec![0xCDu8; 128_000], 1i64, 100i64, 3i64],
            )
            .unwrap();
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .unwrap();
    }

    let size_before_delete = cache.stats().unwrap().2;
    assert!(cache.delete_key("kraken-futures:EURUSD:1Min").unwrap());
    let size_after_delete = cache.stats().unwrap().2;
    assert!(size_after_delete >= size_before_delete);

    let (before_reclaim, after_reclaim) = cache.reclaim_space().unwrap();
    assert!(after_reclaim < before_reclaim);
    assert!(cache.stats().unwrap().2 < size_before_delete);
}

#[test]
fn sqlite_cache_delete_keys_batch_reclaims_space() {
    let db_path = temp_db_path();
    let cache = SqliteCache::open(&db_path).unwrap();
    {
        let conn = cache.connection().unwrap();
        for i in 0..3 {
            conn.execute(
                    "INSERT INTO bar_cache (key, data, timestamp, bar_count, zstd_level) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![format!("kraken-futures:PAIR{i}:1Min"), vec![i as u8; 900_000], 1i64, 1000i64, 3i64],
                )
                .unwrap();
        }
        conn.execute(
                "INSERT INTO bar_cache (key, data, timestamp, bar_count, zstd_level) VALUES (?1, ?2, ?3, ?4, ?5)",
                params!["keep:1Hour", vec![0xEFu8; 64_000], 1i64, 100i64, 3i64],
            )
            .unwrap();
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .unwrap();
    }

    let before = cache.stats().unwrap().2;
    let deleted = cache
        .delete_keys(&[
            "kraken-futures:PAIR0:1Min".to_string(),
            "kraken-futures:PAIR1:1Min".to_string(),
            "kraken-futures:PAIR2:1Min".to_string(),
        ])
        .unwrap();
    assert_eq!(deleted, 3);
    let after = cache.stats().unwrap().2;
    assert!(after < before);
}

#[test]
fn sqlite_cache_delete_broker_data_removes_bar_and_kv_rows() {
    let db_path = temp_db_path();
    let cache = SqliteCache::open(&db_path).unwrap();

    let json = r#"[{"timestamp":"2024-01-01T00:00:00+00:00","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":100.0}]"#;
    cache.put_bars("alpaca:AAPL:1Day", json).unwrap();
    cache.put_bars("kraken-futures:EURUSD:1Day", json).unwrap();
    cache.put_kv("alpaca:meta:test", "{\"ok\":true}").unwrap();

    let deleted = cache.delete_broker_data("alpaca").unwrap();
    assert_eq!(deleted, 2);
    assert!(cache.get_bars("alpaca:AAPL:1Day").unwrap().is_none());
    assert!(
        cache
            .get_bars("kraken-futures:EURUSD:1Day")
            .unwrap()
            .is_some()
    );
    assert!(cache.get_kv("alpaca:meta:test").unwrap().is_none());
}

#[test]
fn search_keys_finds_partial_matches() {
    let db_path = temp_db_path();
    let cache = SqliteCache::open(&db_path).unwrap();
    let json = r#"[{"timestamp":"2024-01-01T00:00:00+00:00","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":100.0}]"#;
    cache.put_bars("kraken-futures:EURUSD:1Hour", json).unwrap();
    cache.put_bars("alpaca:AAPL:1Day", json).unwrap();
    cache.put_bars("kraken:BTCUSD:5Min", json).unwrap();

    let eur = cache.search_keys("EURUSD", 10).unwrap();
    assert_eq!(eur.len(), 1);
    assert_eq!(eur[0], "kraken-futures:EURUSD:1Hour");

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

    cache
        .append_to_queue("lan:test_queue", r#"{"cmd":"A"}"#)
        .unwrap();
    cache
        .append_to_queue("lan:test_queue", r#"{"cmd":"B"}"#)
        .unwrap();
    cache
        .append_to_queue("lan:test_queue", r#"{"cmd":"C"}"#)
        .unwrap();

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
fn zstd_level_sanitizer_clamps_to_supported_range() {
    assert_eq!(sanitize_zstd_level(9), 9);
    assert_eq!(sanitize_zstd_level(99), MAX_ZSTD_LEVEL);
    assert_eq!(sanitize_zstd_level(-10), MIN_ZSTD_LEVEL);
}

#[test]
fn put_bars_with_level_records_selected_zstd_level_metadata() {
    let db_path = temp_db_path();
    let cache = SqliteCache::open(&db_path).unwrap();
    let json = r#"[{"timestamp":"2024-01-01T00:00:00+00:00","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":100.0}]"#;

    cache.put_bars_with_level("CFG:1Day", json, 7).unwrap();

    let conn = cache.connection().unwrap();
    let level: i32 = conn
        .query_row(
            "SELECT zstd_level FROM bar_cache WHERE key = ?1",
            params!["CFG:1Day"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(level, 7);
    assert_eq!(cache.get_bars("CFG:1Day").unwrap().unwrap().0, json);
}

#[test]
fn compact_storage_recompresses_bar_and_kv_cache_tables() {
    let db_path = temp_db_path();
    let cache = SqliteCache::open(&db_path).unwrap();
    let raw = "A".repeat(4096);
    let compressed = zstd::encode_all(raw.as_bytes(), 3).unwrap();
    {
        let conn = cache.connection().unwrap();
        conn.execute(
                "INSERT OR REPLACE INTO bar_cache (key, data, timestamp, bar_count, zstd_level) VALUES (?1, ?2, ?3, ?4, ?5)",
                params!["kraken:BTCUSD:1Min", compressed, 1i64, 1i64, 3i64],
            )
            .unwrap();
    }
    cache.put_kv("broker:test", &raw).unwrap();

    let (processed, _saved) = cache.compact_storage(22, None).unwrap();
    assert!(
        processed >= 3,
        "expected at least the inserted bar/blob rows to be recompressed, got {processed}"
    );
    assert_eq!(cache.count_uncompacted_bars(22).unwrap(), 0);
    assert_eq!(
        cache.get_kv("broker:test").unwrap().as_deref(),
        Some(raw.as_str())
    );
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

#[serial_test::serial]
#[test]
fn kv_writes_obey_configured_zstd_level() {
    let previous_level = bar_zstd_level();
    let db_path = temp_db_path();
    let cache = SqliteCache::open(&db_path).unwrap();
    let payload = "SEC filing / news payload ".repeat(512);

    set_bar_zstd_level(1);
    cache.put_kv("news:test", &payload).unwrap();
    let low_level_raw = cache.get_kv_raw("news:test").unwrap().unwrap().0;
    assert_eq!(
        low_level_raw,
        zstd::encode_all(payload.as_bytes(), 1).unwrap()
    );

    set_bar_zstd_level(22);
    cache.put_kv("sec:test", &payload).unwrap();
    let high_level_raw = cache.get_kv_raw("sec:test").unwrap().unwrap().0;
    assert_eq!(
        high_level_raw,
        zstd::encode_all(payload.as_bytes(), 22).unwrap()
    );
    assert_eq!(
        cache.get_kv("sec:test").unwrap().as_deref(),
        Some(payload.as_str())
    );

    set_bar_zstd_level(previous_level);
}

#[test]
fn encrypted_backup_roundtrips_bar_and_kv_rows() {
    let src_db_path = temp_db_path();
    let dst_db_path = temp_db_path();
    let backup_path = temp_db_path().with_extension("typhoon-backup");
    let backup_path_str = backup_path.to_string_lossy().to_string();
    let src = SqliteCache::open(&src_db_path).unwrap();
    let dst = SqliteCache::open(&dst_db_path).unwrap();

    let json = r#"[{"timestamp":"2024-01-01T00:00:00+00:00","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":100.0}]"#;
    src.put_bars("alpaca:AAPL:1Day", json).unwrap();
    src.put_kv("research:test", r#"{"ok":true}"#).unwrap();

    let export_meta = src
        .export_backup_encrypted(&backup_path_str, "correct horse battery staple")
        .unwrap();
    assert!(export_meta.contains(r#""encrypted":true"#));
    assert!(SqliteCache::backup_file_is_encrypted(&backup_path_str).unwrap());

    dst.import_backup_encrypted(&backup_path_str, "correct horse battery staple")
        .unwrap();
    assert!(dst.get_bars("alpaca:AAPL:1Day").unwrap().is_some());
    assert_eq!(
        dst.get_kv("research:test").unwrap(),
        Some(r#"{"ok":true}"#.to_string())
    );

    let _ = std::fs::remove_file(src_db_path);
    let _ = std::fs::remove_file(dst_db_path);
    let _ = std::fs::remove_file(backup_path);
}

#[test]
fn encrypted_backup_rejects_wrong_passphrase() {
    let src_db_path = temp_db_path();
    let dst_db_path = temp_db_path();
    let backup_path = temp_db_path().with_extension("typhoon-backup");
    let backup_path_str = backup_path.to_string_lossy().to_string();
    let src = SqliteCache::open(&src_db_path).unwrap();
    let dst = SqliteCache::open(&dst_db_path).unwrap();

    src.put_kv("test:key", "secret").unwrap();
    src.export_backup_encrypted(&backup_path_str, "right-pass")
        .unwrap();

    let err = dst
        .import_backup_encrypted(&backup_path_str, "wrong-pass")
        .unwrap_err();
    assert!(err.contains("Decrypt backup failed"));
    assert_eq!(dst.get_kv("test:key").unwrap(), None);

    let _ = std::fs::remove_file(src_db_path);
    let _ = std::fs::remove_file(dst_db_path);
    let _ = std::fs::remove_file(backup_path);
}

#[test]
fn obsolete_low_tf_provider_purge_keeps_native_kraken_and_higher_tf_rows() {
    let db_path = temp_db_path();
    let cache = SqliteCache::open(&db_path).unwrap();
    let json = r#"[{"timestamp":"2024-06-01T00:00:00+00:00","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":100.0}]"#;

    cache.put_bars("kraken-equities:AAPL:1Min", json).unwrap();
    cache.put_bars("kraken-equities:AAPL:5Min", json).unwrap();
    cache.put_bars("alpaca:AAPL:1Min", json).unwrap();
    cache.put_bars("alpaca:AAPL:5Min", json).unwrap();
    cache.put_bars("yahoo-chart:AAPL:1Min", json).unwrap();
    cache.put_bars("yahoo-chart:AAPL:5Min", json).unwrap();
    cache.put_bars("kraken-equities:AAPL:15Min", json).unwrap();
    cache.put_bars("alpaca:AAPL:15Min", json).unwrap();
    cache.put_bars("yahoo-chart:AAPL:15Min", json).unwrap();
    cache.put_bars("kraken:BTC/USD:1Min", json).unwrap();
    cache.put_kv("alpaca:AAPL:1Min", "stale-kv").unwrap();
    cache.put_kv("yahoo-chart:AAPL:5Min", "stale-kv").unwrap();
    cache.put_kv("kraken:BTC/USD:1Min", "spot-kv").unwrap();

    let conn = cache.connection().unwrap();
    let purged = SqliteCache::purge_obsolete_low_tf_provider_bars_locked(&conn).unwrap();
    drop(conn);

    assert_eq!(purged, 4);
    assert!(
        cache
            .get_bars("kraken-equities:AAPL:1Min")
            .unwrap()
            .is_some()
    );
    assert!(
        cache
            .get_bars("kraken-equities:AAPL:5Min")
            .unwrap()
            .is_some()
    );
    assert!(cache.get_bars("alpaca:AAPL:1Min").unwrap().is_none());
    assert!(cache.get_bars("alpaca:AAPL:5Min").unwrap().is_none());
    assert!(cache.get_bars("yahoo-chart:AAPL:1Min").unwrap().is_none());
    assert!(cache.get_bars("yahoo-chart:AAPL:5Min").unwrap().is_none());
    assert!(
        cache
            .get_bars("kraken-equities:AAPL:15Min")
            .unwrap()
            .is_some()
    );
    assert!(cache.get_bars("alpaca:AAPL:15Min").unwrap().is_some());
    assert!(cache.get_bars("yahoo-chart:AAPL:15Min").unwrap().is_some());
    assert!(cache.get_bars("kraken:BTC/USD:1Min").unwrap().is_some());
    assert!(cache.get_kv("alpaca:AAPL:1Min").unwrap().is_none());
    assert!(cache.get_kv("yahoo-chart:AAPL:5Min").unwrap().is_none());
    assert!(cache.get_kv("kraken:BTC/USD:1Min").unwrap().is_some());

    let _ = std::fs::remove_file(db_path);
}

#[test]
fn delete_kraken_equity_bars_by_tf_targets_only_matching_rows() {
    let db_path = temp_db_path();
    let cache = SqliteCache::open(&db_path).unwrap();
    let json = r#"[{"timestamp":"2024-06-01T00:00:00+00:00","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":100.0}]"#;
    // Should be deleted:
    cache.put_bars("kraken-equities:AAPL:1Min", json).unwrap();
    cache.put_bars("kraken-equities:MSFT:5Min", json).unwrap();
    cache.put_bars("kraken-equities:TSLA:1Min", json).unwrap();
    // Should survive (different TF, different broker, or symbol-name
    // patterns that don't end in the targeted suffix):
    cache.put_bars("kraken-equities:AAPL:1Hour", json).unwrap();
    cache.put_bars("kraken-equities:AAPL:1Day", json).unwrap();
    cache.put_bars("kraken:BTCUSD:1Min", json).unwrap();
    cache.put_bars("alpaca:AAPL:1Min", json).unwrap();
    cache
        .put_bars("kraken-equities:NOT1MinFoo:1Hour", json)
        .unwrap();

    let (deleted, _bytes) = cache
        .delete_kraken_equity_bars_by_tf(&["1Min", "5Min"])
        .unwrap();
    assert_eq!(deleted, 3, "expected 3 rows deleted, got {deleted}");

    // Survivors still queryable:
    assert!(
        cache
            .get_bars("kraken-equities:AAPL:1Hour")
            .unwrap()
            .is_some()
    );
    assert!(
        cache
            .get_bars("kraken-equities:AAPL:1Day")
            .unwrap()
            .is_some()
    );
    assert!(cache.get_bars("kraken:BTCUSD:1Min").unwrap().is_some());
    assert!(cache.get_bars("alpaca:AAPL:1Min").unwrap().is_some());
    assert!(
        cache
            .get_bars("kraken-equities:NOT1MinFoo:1Hour")
            .unwrap()
            .is_some()
    );
    // Targets gone:
    assert!(
        cache
            .get_bars("kraken-equities:AAPL:1Min")
            .unwrap()
            .is_none()
    );
    assert!(
        cache
            .get_bars("kraken-equities:MSFT:5Min")
            .unwrap()
            .is_none()
    );
    assert!(
        cache
            .get_bars("kraken-equities:TSLA:1Min")
            .unwrap()
            .is_none()
    );

    let _ = std::fs::remove_file(db_path);
}

#[test]
fn delete_kraken_equity_bars_by_tf_empty_list_is_noop() {
    let db_path = temp_db_path();
    let cache = SqliteCache::open(&db_path).unwrap();
    let json = r#"[{"timestamp":"2024-06-01T00:00:00+00:00","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":100.0}]"#;
    cache.put_bars("kraken-equities:AAPL:1Min", json).unwrap();
    let (deleted, bytes) = cache.delete_kraken_equity_bars_by_tf(&[]).unwrap();
    assert_eq!(deleted, 0);
    assert_eq!(bytes, 0);
    assert!(
        cache
            .get_bars("kraken-equities:AAPL:1Min")
            .unwrap()
            .is_some()
    );
    let _ = std::fs::remove_file(db_path);
}

#[test]
fn data_sanity_audit_flags_structural_metadata_and_price_errors() {
    let db_path = temp_db_path();
    let cache = SqliteCache::open(&db_path).unwrap();
    let good = r#"[
        {"timestamp":"2024-01-01T00:00:00+00:00","open":10.0,"high":11.0,"low":9.0,"close":10.5,"volume":100.0},
        {"timestamp":"2024-01-02T00:00:00+00:00","open":10.5,"high":12.0,"low":10.0,"close":11.0,"volume":120.0}
    ]"#;
    cache.put_bars("alpaca:AAPL:1Day", good).unwrap();
    {
        let mut bad_binary = Vec::new();
        bad_binary.extend_from_slice(BAR_BINARY_MAGIC);
        bad_binary.extend_from_slice(&1u32.to_le_bytes());
        bad_binary.extend_from_slice(&1_704_067_200_000i64.to_le_bytes());
        for value in [10.0f64, 8.0, 9.0, 10.5, 100.0] {
            bad_binary.extend_from_slice(&value.to_le_bytes());
        }
        let compressed = zstd::encode_all(bad_binary.as_slice(), DEFAULT_BAR_ZSTD_LEVEL).unwrap();
        let conn = cache.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO bar_cache (key, data, timestamp, bar_count, last_ts, second_last_ts, zstd_level) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params!["yahoo-chart:BAD:1Day", compressed, chrono::Utc::now().timestamp(), 1i64, "2024-01-01T00:00:00+00:00", Option::<String>::None, DEFAULT_BAR_ZSTD_LEVEL],
        )
        .unwrap();
        conn.execute(
            "UPDATE bar_cache SET bar_count = 99, last_ts = NULL, second_last_ts = NULL WHERE key = ?1",
            params!["alpaca:AAPL:1Day"],
        )
        .unwrap();
    }

    let report = cache.audit_bar_cache_sanity().unwrap();
    assert_eq!(report.rows_scanned, 2);
    assert!(report.error_count >= 2, "{report:#?}");
    assert!(report.has_code("bar_count_mismatch"), "{report:#?}");
    assert!(report.has_code("last_ts_missing"), "{report:#?}");
    assert!(report.has_code("invalid_ohlc"), "{report:#?}");

    let _ = std::fs::remove_file(db_path);
}

#[test]
fn data_sanity_audit_flags_cross_source_recent_overlap_mismatch() {
    let db_path = temp_db_path();
    let cache = SqliteCache::open(&db_path).unwrap();
    let trusted = r#"[
        {"timestamp":"2024-01-01T00:00:00+00:00","open":100.0,"high":101.0,"low":99.0,"close":100.0,"volume":100.0},
        {"timestamp":"2024-01-02T00:00:00+00:00","open":100.0,"high":102.0,"low":99.0,"close":101.0,"volume":100.0}
    ]"#;
    let depth = r#"[
        {"timestamp":"2024-01-01T00:00:00+00:00","open":100.0,"high":101.0,"low":99.0,"close":100.0,"volume":100.0},
        {"timestamp":"2024-01-02T00:00:00+00:00","open":190.0,"high":202.0,"low":188.0,"close":200.0,"volume":100.0}
    ]"#;
    cache.put_bars("alpaca:WOK:1Day", trusted).unwrap();
    cache.put_bars("yahoo-chart:WOK:1Day", depth).unwrap();

    let report = cache.audit_bar_cache_sanity().unwrap();
    assert!(report.has_code("cross_source_overlap_mismatch"), "{report:#?}");
    assert!(report.warn_count >= 1, "{report:#?}");

    let _ = std::fs::remove_file(db_path);
}

#[test]
fn data_sanity_audit_flags_merged_source_overlap_mismatch() {
    let db_path = temp_db_path();
    let cache = SqliteCache::open(&db_path).unwrap();
    let source = r#"[
        {"timestamp":"2024-01-01T00:00:00+00:00","open":50.0,"high":51.0,"low":49.0,"close":50.0,"volume":100.0},
        {"timestamp":"2024-01-02T00:00:00+00:00","open":51.0,"high":52.0,"low":50.0,"close":51.0,"volume":100.0}
    ]"#;
    let merged = r#"[
        {"timestamp":"2024-01-01T00:00:00+00:00","open":50.0,"high":51.0,"low":49.0,"close":50.0,"volume":100.0},
        {"timestamp":"2024-01-02T00:00:00+00:00","open":150.0,"high":155.0,"low":145.0,"close":150.0,"volume":100.0}
    ]"#;
    cache.put_bars("alpaca:WOK:1Day", source).unwrap();
    cache.put_bars("merged:WOK:1Day", merged).unwrap();

    let report = cache.audit_bar_cache_sanity().unwrap();
    assert!(report.has_code("merged_source_overlap_mismatch"), "{report:#?}");
    assert!(
        report.issue_code_count("merged_source_overlap_mismatch") >= 1,
        "{report:#?}"
    );

    let _ = std::fs::remove_file(db_path);
}

#[test]
fn data_sanity_audit_allows_stable_merged_source_scale_delta() {
    let db_path = temp_db_path();
    let cache = SqliteCache::open(&db_path).unwrap();
    let raw = r#"[
        {"timestamp":"2024-01-01T00:00:00+00:00","open":1.0,"high":1.0,"low":1.0,"close":1.0,"volume":100.0},
        {"timestamp":"2024-01-02T00:00:00+00:00","open":1.1,"high":1.1,"low":1.1,"close":1.1,"volume":100.0},
        {"timestamp":"2024-01-03T00:00:00+00:00","open":1.2,"high":1.2,"low":1.2,"close":1.2,"volume":100.0}
    ]"#;
    let merged = r#"[
        {"timestamp":"2024-01-01T00:00:00+00:00","open":10.0,"high":10.0,"low":10.0,"close":10.0,"volume":100.0},
        {"timestamp":"2024-01-02T00:00:00+00:00","open":11.0,"high":11.0,"low":11.0,"close":11.0,"volume":100.0},
        {"timestamp":"2024-01-03T00:00:00+00:00","open":12.0,"high":12.0,"low":12.0,"close":12.0,"volume":100.0}
    ]"#;
    cache.put_bars("alpaca:WOK:1Day", raw).unwrap();
    cache.put_bars("merged:WOK:1Day", merged).unwrap();

    let report = cache.audit_bar_cache_sanity().unwrap();
    assert_eq!(
        report.issue_code_count("merged_source_overlap_mismatch"),
        0,
        "stable split/corporate-action scale deltas should not be treated as corrupt merged drift: {report:#?}"
    );
    assert!(
        report.issue_code_count("merged_source_stable_scale_delta") >= 1,
        "stable scale delta should still be visible as informational audit context: {report:#?}"
    );

    let _ = std::fs::remove_file(db_path);
}

#[test]
fn data_sanity_audit_allows_historical_cross_source_scale_delta_when_recent_agrees() {
    let db_path = temp_db_path();
    let cache = SqliteCache::open(&db_path).unwrap();
    let compact = r#"[
        {"timestamp":"2024-01-01T00:00:00+00:00","open":1.0,"high":1.0,"low":1.0,"close":1.0,"volume":100.0},
        {"timestamp":"2024-01-02T00:00:00+00:00","open":1.0,"high":1.0,"low":1.0,"close":1.0,"volume":100.0},
        {"timestamp":"2024-01-03T00:00:00+00:00","open":1.0,"high":1.0,"low":1.0,"close":1.0,"volume":100.0},
        {"timestamp":"2024-01-04T00:00:00+00:00","open":1.0,"high":1.0,"low":1.0,"close":1.0,"volume":100.0}
    ]"#;
    let split_adjusted = r#"[
        {"timestamp":"2024-01-01T00:00:00+00:00","open":100.0,"high":100.0,"low":100.0,"close":100.0,"volume":100.0},
        {"timestamp":"2024-01-02T00:00:00+00:00","open":100.0,"high":100.0,"low":100.0,"close":100.0,"volume":100.0},
        {"timestamp":"2024-01-03T00:00:00+00:00","open":1.0,"high":1.0,"low":1.0,"close":1.0,"volume":100.0},
        {"timestamp":"2024-01-04T00:00:00+00:00","open":1.0,"high":1.0,"low":1.0,"close":1.0,"volume":100.0}
    ]"#;
    cache.put_bars("alpaca:WOK:1Day", compact).unwrap();
    cache.put_bars("yahoo-chart:WOK:1Day", split_adjusted).unwrap();

    let report = cache.audit_bar_cache_sanity().unwrap();
    assert_eq!(
        report.issue_code_count("cross_source_overlap_mismatch"),
        0,
        "historical split-era source differences with recent agreement should not be flagged as corrupt overlap drift: {report:#?}"
    );
    assert!(
        report.issue_code_count("cross_source_historical_scale_delta") >= 1,
        "historical scale delta should still be visible as informational audit context: {report:#?}"
    );

    let _ = std::fs::remove_file(db_path);
}

#[test]
fn data_sanity_audit_allows_historical_merged_source_scale_delta_when_recent_agrees() {
    let db_path = temp_db_path();
    let cache = SqliteCache::open(&db_path).unwrap();
    let merged = r#"[
        {"timestamp":"2024-01-01T00:00:00+00:00","open":1.0,"high":1.0,"low":1.0,"close":1.0,"volume":100.0},
        {"timestamp":"2024-01-02T00:00:00+00:00","open":1.0,"high":1.0,"low":1.0,"close":1.0,"volume":100.0},
        {"timestamp":"2024-01-03T00:00:00+00:00","open":1.0,"high":1.0,"low":1.0,"close":1.0,"volume":100.0},
        {"timestamp":"2024-01-04T00:00:00+00:00","open":1.0,"high":1.0,"low":1.0,"close":1.0,"volume":100.0}
    ]"#;
    let raw = r#"[
        {"timestamp":"2024-01-01T00:00:00+00:00","open":100.0,"high":100.0,"low":100.0,"close":100.0,"volume":100.0},
        {"timestamp":"2024-01-02T00:00:00+00:00","open":100.0,"high":100.0,"low":100.0,"close":100.0,"volume":100.0},
        {"timestamp":"2024-01-03T00:00:00+00:00","open":1.0,"high":1.0,"low":1.0,"close":1.0,"volume":100.0},
        {"timestamp":"2024-01-04T00:00:00+00:00","open":1.0,"high":1.0,"low":1.0,"close":1.0,"volume":100.0}
    ]"#;
    cache.put_bars("alpaca:WOK:1Day", raw).unwrap();
    cache.put_bars("merged:WOK:1Day", merged).unwrap();

    let report = cache.audit_bar_cache_sanity().unwrap();
    assert_eq!(
        report.issue_code_count("merged_source_overlap_mismatch"),
        0,
        "historical split-era raw differences with recent merged agreement should not be flagged as corrupt merged drift: {report:#?}"
    );
    assert!(
        report.issue_code_count("merged_source_historical_scale_delta") >= 1,
        "historical merged/source scale delta should still be visible as informational audit context: {report:#?}"
    );

    let _ = std::fs::remove_file(db_path);
}

/// Build a TTBR binary blob from raw bar tuples (ts_ms, o, h, l, c, v).
fn ttbr_binary(bars: &[(i64, f64, f64, f64, f64, f64)]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(8 + bars.len() * BYTES_PER_BAR);
    buf.extend_from_slice(BAR_BINARY_MAGIC);
    buf.extend_from_slice(&(bars.len() as u32).to_le_bytes());
    for (ts, o, h, l, c, v) in bars {
        buf.extend_from_slice(&ts.to_le_bytes());
        buf.extend_from_slice(&o.to_le_bytes());
        buf.extend_from_slice(&h.to_le_bytes());
        buf.extend_from_slice(&l.to_le_bytes());
        buf.extend_from_slice(&c.to_le_bytes());
        buf.extend_from_slice(&v.to_le_bytes());
    }
    buf
}

/// Fixed `timestamp` column value used by raw-inserted test rows so tests can
/// assert repairs preserve the last-fetch time.
const TEST_ROW_TIMESTAMP: i64 = 1_700_000_000;

/// Insert a raw blob row directly, bypassing the write path's normalization.
fn insert_raw_row(
    cache: &SqliteCache,
    key: &str,
    blob: &[u8],
    bar_count: Option<i64>,
    last_ts: Option<&str>,
    second_last_ts: Option<&str>,
) {
    let compressed = zstd::encode_all(blob, DEFAULT_BAR_ZSTD_LEVEL).unwrap();
    let conn = cache.conn.lock().unwrap();
    conn.execute(
        "INSERT OR REPLACE INTO bar_cache (key, data, timestamp, bar_count, last_ts, second_last_ts, zstd_level) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![key, compressed, TEST_ROW_TIMESTAMP, bar_count, last_ts, second_last_ts, DEFAULT_BAR_ZSTD_LEVEL],
    )
    .unwrap();
}

fn row_timestamp(cache: &SqliteCache, key: &str) -> i64 {
    let conn = cache.conn.lock().unwrap();
    conn.query_row(
        "SELECT timestamp FROM bar_cache WHERE key = ?1",
        params![key],
        |r| r.get(0),
    )
    .unwrap()
}

#[test]
fn data_sanity_repair_fixes_metadata_and_preserves_fetch_timestamp() {
    let db_path = temp_db_path();
    let cache = SqliteCache::open(&db_path).unwrap();
    let day = 86_400_000i64;
    let blob = ttbr_binary(&[
        (day, 1.0, 2.0, 0.5, 1.5, 10.0),
        (2 * day, 1.5, 2.5, 1.0, 2.0, 12.0),
    ]);
    insert_raw_row(&cache, "alpaca:META:1Day", &blob, Some(99), None, Some("wrong"));

    let report = cache.audit_bar_cache_sanity().unwrap();
    assert!(report.has_code("bar_count_mismatch"), "{report:#?}");
    assert!(report.has_code("last_ts_missing"), "{report:#?}");
    assert!(report.has_code("second_last_ts_mismatch"), "{report:#?}");
    assert_eq!(report.metadata_repairable_rows, 1, "{report:#?}");

    let outcome = cache
        .repair_bar_cache(
            BarCacheRepairOptions {
                fix_metadata: true,
                ..Default::default()
            },
            None,
            None,
        )
        .unwrap();
    assert_eq!(outcome.metadata_fixed, 1, "{outcome:#?}");
    assert_eq!(outcome.rows_rewritten, 0, "{outcome:#?}");
    assert_eq!(outcome.rows_deleted, 0, "{outcome:#?}");

    let report = cache.audit_bar_cache_sanity().unwrap();
    assert!(!report.has_code("bar_count_mismatch"), "{report:#?}");
    assert!(!report.has_code("last_ts_missing"), "{report:#?}");
    assert!(!report.has_code("second_last_ts_mismatch"), "{report:#?}");
    assert_eq!(report.metadata_repairable_rows, 0, "{report:#?}");
    assert_eq!(
        row_timestamp(&cache, "alpaca:META:1Day"),
        TEST_ROW_TIMESTAMP,
        "metadata repair must not touch the last-fetch timestamp"
    );

    let _ = std::fs::remove_file(db_path);
}

#[test]
fn data_sanity_repair_rewrites_invalid_duplicate_and_future_bars() {
    let db_path = temp_db_path();
    let cache = SqliteCache::open(&db_path).unwrap();
    let now_ms = chrono::Utc::now().timestamp_millis();
    let day = 86_400_000i64;
    let d1 = now_ms - 3 * day;
    let d2 = now_ms - 2 * day;
    let bars = [
        (d1, 1.0, 2.0, 0.5, 1.5, 10.0),          // valid
        (d2, 1.5, 2.5, 1.0, 2.0, 12.0),          // valid, superseded by dup below
        (d2 + 3_600_000, 9.0, 9.5, 8.5, 9.2, 5.0), // same 1Day bucket — later wins
        (d2 + 7_200_000, 5.0, 4.0, 6.0, 5.0, 5.0), // invalid: high < low
        (now_ms + 10 * day, 1.0, 2.0, 0.5, 1.5, 1.0), // future
    ];
    let blob = ttbr_binary(&bars);
    insert_raw_row(&cache, "alpaca:RW:1Day", &blob, Some(5), None, None);

    let report = cache.audit_bar_cache_sanity().unwrap();
    assert!(report.has_code("invalid_ohlc"), "{report:#?}");
    assert!(report.has_code("future_timestamp"), "{report:#?}");
    assert!(
        report.has_code("non_monotonic_or_duplicate_bucket"),
        "{report:#?}"
    );
    assert!(report.rewritable_rows >= 1, "{report:#?}");

    let outcome = cache
        .repair_bar_cache(
            BarCacheRepairOptions {
                rewrite_bad_rows: true,
                ..Default::default()
            },
            None,
            None,
        )
        .unwrap();
    assert_eq!(outcome.rows_rewritten, 1, "{outcome:#?}");
    assert_eq!(outcome.bars_dropped, 3, "{outcome:#?}");

    let report = cache.audit_bar_cache_sanity().unwrap();
    assert!(!report.has_code("invalid_ohlc"), "{report:#?}");
    assert!(!report.has_code("future_timestamp"), "{report:#?}");
    assert!(
        !report.has_code("non_monotonic_or_duplicate_bucket"),
        "{report:#?}"
    );
    assert!(!report.has_code("bar_count_mismatch"), "{report:#?}");
    assert!(!report.has_code("last_ts_missing"), "{report:#?}");

    let kept = cache.get_bars_raw("alpaca:RW:1Day").unwrap().unwrap();
    assert_eq!(kept.len(), 2, "{kept:#?}");
    assert!(
        (kept[1].4 - 9.2).abs() < 1e-9,
        "later duplicate-bucket bar must win: {kept:#?}"
    );
    assert_eq!(row_timestamp(&cache, "alpaca:RW:1Day"), TEST_ROW_TIMESTAMP);

    let _ = std::fs::remove_file(db_path);
}

#[test]
fn data_sanity_repair_converts_legacy_json_rows() {
    let db_path = temp_db_path();
    let cache = SqliteCache::open(&db_path).unwrap();
    let legacy = br#"[{"timestamp":"2024-01-02T00:00:00+00:00","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":10.0}]"#;
    insert_raw_row(&cache, "alpaca:LEG:1Day", legacy, Some(1), None, None);

    let report = cache.audit_bar_cache_sanity().unwrap();
    assert!(report.has_code("legacy_json_row"), "{report:#?}");
    assert!(!report.has_code("bad_binary_header"), "{report:#?}");
    assert_eq!(report.rewritable_rows, 1, "{report:#?}");
    assert_eq!(report.corrupt_rows, 0, "{report:#?}");

    let outcome = cache
        .repair_bar_cache(
            BarCacheRepairOptions {
                rewrite_bad_rows: true,
                ..Default::default()
            },
            None,
            None,
        )
        .unwrap();
    assert_eq!(outcome.legacy_rows_converted, 1, "{outcome:#?}");
    assert_eq!(outcome.rows_rewritten, 1, "{outcome:#?}");

    let report = cache.audit_bar_cache_sanity().unwrap();
    assert!(!report.has_code("legacy_json_row"), "{report:#?}");
    let kept = cache.get_bars_raw("alpaca:LEG:1Day").unwrap().unwrap();
    assert_eq!(kept.len(), 1, "{kept:#?}");

    let _ = std::fs::remove_file(db_path);
}

#[test]
fn data_sanity_repair_deletes_undecodable_rows() {
    let db_path = temp_db_path();
    let cache = SqliteCache::open(&db_path).unwrap();
    insert_raw_row(
        &cache,
        "alpaca:BADBLOB:1Day",
        b"garbage neither ttbr nor json",
        Some(1),
        None,
        None,
    );
    let good = r#"[{"timestamp":"2024-01-02T00:00:00+00:00","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":10.0}]"#;
    cache.put_bars("alpaca:GOOD:1Day", good).unwrap();

    let report = cache.audit_bar_cache_sanity().unwrap();
    assert!(report.has_code("bad_binary_header"), "{report:#?}");
    assert_eq!(report.corrupt_rows, 1, "{report:#?}");

    let outcome = cache
        .repair_bar_cache(
            BarCacheRepairOptions {
                delete_corrupt_rows: true,
                ..Default::default()
            },
            None,
            None,
        )
        .unwrap();
    assert_eq!(outcome.rows_deleted, 1, "{outcome:#?}");

    let report = cache.audit_bar_cache_sanity().unwrap();
    assert!(!report.has_code("bad_binary_header"), "{report:#?}");
    assert_eq!(report.rows_scanned, 1, "{report:#?}");
    assert!(
        cache.get_bars_raw("alpaca:GOOD:1Day").unwrap().is_some(),
        "healthy row must survive corrupt-row deletion"
    );

    let _ = std::fs::remove_file(db_path);
}

#[test]
fn data_sanity_audit_collects_merged_mismatch_keys() {
    let db_path = temp_db_path();
    let cache = SqliteCache::open(&db_path).unwrap();
    let source = r#"[
        {"timestamp":"2024-01-01T00:00:00+00:00","open":50.0,"high":51.0,"low":49.0,"close":50.0,"volume":100.0},
        {"timestamp":"2024-01-02T00:00:00+00:00","open":51.0,"high":52.0,"low":50.0,"close":51.0,"volume":100.0}
    ]"#;
    let merged = r#"[
        {"timestamp":"2024-01-01T00:00:00+00:00","open":50.0,"high":51.0,"low":49.0,"close":50.0,"volume":100.0},
        {"timestamp":"2024-01-02T00:00:00+00:00","open":150.0,"high":155.0,"low":145.0,"close":150.0,"volume":100.0}
    ]"#;
    cache.put_bars("alpaca:WOK:1Day", source).unwrap();
    cache.put_bars("merged:WOK:1Day", merged).unwrap();

    let report = cache.audit_bar_cache_sanity().unwrap();
    assert!(
        report.merged_mismatch_keys.contains("merged:WOK:1Day"),
        "{report:#?}"
    );

    let _ = std::fs::remove_file(db_path);
}

#[test]
fn data_sanity_audit_orders_issues_most_severe_first() {
    let db_path = temp_db_path();
    let cache = SqliteCache::open(&db_path).unwrap();
    // Info issue on a key that sorts first…
    cache.put_bars("alpaca:AAA:1Day", "[]").unwrap();
    // …and an Error issue on a key that sorts last.
    let day = 86_400_000i64;
    let blob = ttbr_binary(&[(day, 5.0, 4.0, 6.0, 5.0, 1.0)]); // high < low
    insert_raw_row(&cache, "alpaca:ZZZ:1Day", &blob, Some(1), None, None);

    let report = cache.audit_bar_cache_sanity().unwrap();
    let first = report.issues.first().expect("issues expected");
    assert_eq!(first.severity, BarCacheSanitySeverity::Error, "{report:#?}");
    assert!(first.key.contains("ZZZ"), "{report:#?}");

    let _ = std::fs::remove_file(db_path);
}

#[test]
fn data_sanity_audit_aggregates_per_bar_hits_into_one_issue_per_row() {
    let db_path = temp_db_path();
    let cache = SqliteCache::open(&db_path).unwrap();
    let day = 86_400_000i64;
    let blob = ttbr_binary(&[
        (day, 5.0, 4.0, 6.0, 5.0, 1.0),     // invalid: high < low
        (2 * day, 1.0, 2.0, 0.5, 1.5, 1.0), // valid
        (3 * day, -1.0, 2.0, 0.5, 1.5, 1.0), // invalid: open <= 0
        (4 * day, 1.0, 2.0, 0.5, 1.5, -5.0), // invalid: volume < 0
    ]);
    insert_raw_row(&cache, "alpaca:AGG:1Day", &blob, Some(4), None, None);

    let report = cache.audit_bar_cache_sanity().unwrap();
    assert_eq!(report.issue_code_count("invalid_ohlc"), 1, "{report:#?}");
    let issue = report
        .issues
        .iter()
        .find(|i| i.code == "invalid_ohlc")
        .expect("invalid_ohlc issue");
    assert_eq!(issue.occurrences, 3, "{report:#?}");

    let _ = std::fs::remove_file(db_path);
}

#[test]
fn data_sanity_audit_gap_details_have_dates_and_recent_intraday_gaps_warn() {
    let db_path = temp_db_path();
    let cache = SqliteCache::open(&db_path).unwrap();
    let now = chrono::Utc::now();
    let fmt = |dt: chrono::DateTime<chrono::Utc>| dt.to_rfc3339();
    // 15Min series with a 40-day hole that ends within the last 30 days.
    let recent_gap = format!(
        r#"[
        {{"timestamp":"{}","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":1.0}},
        {{"timestamp":"{}","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":1.0}},
        {{"timestamp":"{}","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":1.0}}
    ]"#,
        fmt(now - chrono::Duration::days(40)),
        fmt(now - chrono::Duration::minutes(60)),
        fmt(now - chrono::Duration::minutes(45)),
    );
    cache.put_bars("alpaca:GAPNEW:15Min", &recent_gap).unwrap();
    // Daily series whose hole ended years ago.
    let old_gap = r#"[
        {"timestamp":"2019-01-01T00:00:00+00:00","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":1.0},
        {"timestamp":"2019-01-02T00:00:00+00:00","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":1.0},
        {"timestamp":"2020-06-01T00:00:00+00:00","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":1.0}
    ]"#;
    cache.put_bars("alpaca:GAPOLD:1Day", old_gap).unwrap();

    let report = cache.audit_bar_cache_sanity().unwrap();
    let recent = report
        .issues
        .iter()
        .find(|i| i.code == "large_time_gap" && i.key.contains("GAPNEW"))
        .expect("recent intraday gap issue");
    assert_eq!(
        recent.severity,
        BarCacheSanitySeverity::Warn,
        "recent intraday hole should be an actionable warning: {report:#?}"
    );
    assert!(recent.detail.contains("from="), "{recent:?}");
    assert!(recent.detail.contains("to="), "{recent:?}");
    let old = report
        .issues
        .iter()
        .find(|i| i.code == "large_time_gap" && i.key.contains("GAPOLD"))
        .expect("old daily gap issue");
    assert_eq!(old.severity, BarCacheSanitySeverity::Info, "{report:#?}");

    let _ = std::fs::remove_file(db_path);
}

#[test]
fn data_sanity_audit_keeps_holiday_weekend_intraday_gaps_informational() {
    let db_path = temp_db_path();
    let cache = SqliteCache::open(&db_path).unwrap();
    let now = chrono::Utc::now();
    let fmt = |dt: chrono::DateTime<chrono::Utc>| dt.to_rfc3339();
    // A 4-day market closure (holiday long weekend) ending recently clears the
    // 15Min gap threshold on every healthy series — must stay Info, not Warn.
    let bars = format!(
        r#"[
        {{"timestamp":"{}","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":1.0}},
        {{"timestamp":"{}","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":1.0}},
        {{"timestamp":"{}","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":1.0}}
    ]"#,
        fmt(now - chrono::Duration::days(6)),
        fmt(now - chrono::Duration::days(2)),
        fmt(now - chrono::Duration::minutes(30)),
    );
    cache.put_bars("alpaca:HOLIDAY:15Min", &bars).unwrap();

    let report = cache.audit_bar_cache_sanity().unwrap();
    let gap = report
        .issues
        .iter()
        .find(|i| i.code == "large_time_gap" && i.key.contains("HOLIDAY"))
        .expect("holiday gap issue");
    assert_eq!(
        gap.severity,
        BarCacheSanitySeverity::Info,
        "a sub-week closure must not read as a stalled sync lane: {report:#?}"
    );

    let _ = std::fs::remove_file(db_path);
}

#[test]
fn data_sanity_history_roundtrip_and_delta_line() {
    let db_path = temp_db_path();
    let cache = SqliteCache::open(&db_path).unwrap();
    let good = r#"[{"timestamp":"2024-01-02T00:00:00+00:00","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":10.0}]"#;
    cache.put_bars("alpaca:HIST:1Day", good).unwrap();

    let first = cache.audit_bar_cache_sanity().unwrap();
    cache.record_bar_sanity_history(&first).unwrap();
    assert_eq!(cache.load_bar_sanity_history().len(), 1);

    // Introduce a metadata problem, re-audit, and diff against run 1.
    {
        let conn = cache.conn.lock().unwrap();
        conn.execute(
            "UPDATE bar_cache SET last_ts = NULL WHERE key = ?1",
            params!["alpaca:HIST:1Day"],
        )
        .unwrap();
    }
    let second = cache.audit_bar_cache_sanity().unwrap();
    let prev = cache.load_bar_sanity_history().pop().unwrap();
    let delta = second.delta_line(&prev).expect("delta expected");
    assert!(delta.contains("+1 last_ts_missing"), "{delta}");
    cache.record_bar_sanity_history(&second).unwrap();
    assert_eq!(cache.load_bar_sanity_history().len(), 2);

    // Identical runs produce no delta.
    let third = cache.audit_bar_cache_sanity().unwrap();
    let prev = cache.load_bar_sanity_history().pop().unwrap();
    assert!(third.delta_line(&prev).is_none());

    let _ = std::fs::remove_file(db_path);
}

#[test]
fn data_sanity_audit_cancel_skips_cross_source_checks() {
    let db_path = temp_db_path();
    let cache = SqliteCache::open(&db_path).unwrap();
    let bars = r#"[{"timestamp":"2024-01-02T00:00:00+00:00","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":10.0}]"#;
    cache.put_bars("alpaca:CXL:1Day", bars).unwrap();
    cache.put_bars("yahoo-chart:CXL:1Day", bars).unwrap();

    let cancel = std::sync::atomic::AtomicBool::new(true);
    let calls = std::cell::Cell::new(0usize);
    let progress = |_done: usize, _total: usize| calls.set(calls.get() + 1);
    let report = cache
        .audit_bar_cache_sanity_with(Some(&progress), Some(&cancel))
        .unwrap();
    assert!(report.cancelled, "{report:#?}");
    assert_eq!(report.source_pairs_checked, 0, "{report:#?}");
    assert!(calls.get() >= 1, "progress callback should have fired");
    assert!(report.summary_line().contains("CANCELLED"));

    let _ = std::fs::remove_file(db_path);
}

#[test]
fn data_sanity_audit_classifies_carried_open_as_info() {
    let db_path = temp_db_path();
    let cache = SqliteCache::open(&db_path).unwrap();
    // Sparse-market candle: open carried from previous close sits far above
    // the traded range, but the close is inside. Kraken OHLC semantics.
    let bars = r#"[
        {"timestamp":"2019-01-01T00:00:00+00:00","open":1.0,"high":1.1,"low":0.9,"close":1.0,"volume":1.0},
        {"timestamp":"2019-01-02T00:00:00+00:00","open":1.0,"high":0.7,"low":0.5,"close":0.6,"volume":1.0}
    ]"#;
    cache.put_bars("kraken:ACHUSD:1Day", bars).unwrap();

    let report = cache.audit_bar_cache_sanity().unwrap();
    assert!(report.has_code("carried_open_range"), "{report:#?}");
    assert!(!report.has_code("body_outside_range"), "{report:#?}");
    let issue = report
        .issues
        .iter()
        .find(|i| i.code == "carried_open_range")
        .unwrap();
    assert_eq!(issue.severity, BarCacheSanitySeverity::Info, "{report:#?}");

    let _ = std::fs::remove_file(db_path);
}

#[test]
fn data_sanity_audit_warns_on_settled_close_outside_range() {
    let db_path = temp_db_path();
    let cache = SqliteCache::open(&db_path).unwrap();
    // Historical bar whose close is outside its own high/low — malformed.
    let bars = r#"[
        {"timestamp":"2015-05-04T00:00:00+00:00","open":10.0,"high":10.2,"low":9.8,"close":10.0,"volume":1.0},
        {"timestamp":"2015-05-05T00:00:00+00:00","open":10.16,"high":10.16,"low":10.16,"close":10.69,"volume":1.0}
    ]"#;
    cache.put_bars("yahoo-chart:SEC:1Day", bars).unwrap();

    let report = cache.audit_bar_cache_sanity().unwrap();
    assert!(report.has_code("body_outside_range"), "{report:#?}");
    let issue = report
        .issues
        .iter()
        .find(|i| i.code == "body_outside_range")
        .unwrap();
    assert_eq!(issue.severity, BarCacheSanitySeverity::Warn, "{report:#?}");
    assert!(issue.detail.contains("settled_close_out=1"), "{issue:?}");

    let _ = std::fs::remove_file(db_path);
}

#[test]
fn data_sanity_audit_treats_forming_candle_close_drift_as_info() {
    let db_path = temp_db_path();
    let cache = SqliteCache::open(&db_path).unwrap();
    let now = chrono::Utc::now();
    // The current month's still-forming candle: live close has moved below a
    // lagging high/low (Yahoo coarse-feed behavior). Not settled damage.
    let month_start = chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc();
    let prev_month = month_start - chrono::Duration::days(15);
    let bars = format!(
        r#"[
        {{"timestamp":"{}","open":30.0,"high":31.0,"low":29.0,"close":30.0,"volume":1.0}},
        {{"timestamp":"{}","open":29.4,"high":30.6,"low":29.4,"close":25.5,"volume":1.0}}
    ]"#,
        prev_month.to_rfc3339(),
        month_start.to_rfc3339(),
    );
    cache.put_bars("yahoo-chart:ACLZ:1Month", &bars).unwrap();

    let report = cache.audit_bar_cache_sanity().unwrap();
    assert!(!report.has_code("body_outside_range"), "{report:#?}");
    assert!(report.has_code("carried_open_range"), "{report:#?}");
    let issue = report
        .issues
        .iter()
        .find(|i| i.code == "carried_open_range")
        .unwrap();
    assert!(issue.detail.contains("forming_close_out=1"), "{issue:?}");

    let _ = std::fs::remove_file(db_path);
}

#[test]
fn data_sanity_audit_ignores_single_field_range_noise_across_sources() {
    let db_path = temp_db_path();
    let cache = SqliteCache::open(&db_path).unwrap();
    // Closes agree on every bar; one provider has a degenerate low. The old
    // max-over-OHLC comparison reported a 90× "mismatch" here.
    let a = r#"[
        {"timestamp":"2024-01-01T00:00:00+00:00","open":13.9,"high":14.0,"low":13.8,"close":13.92,"volume":1.0},
        {"timestamp":"2024-01-02T00:00:00+00:00","open":13.9,"high":14.0,"low":13.8,"close":13.90,"volume":1.0}
    ]"#;
    let b = r#"[
        {"timestamp":"2024-01-01T00:00:00+00:00","open":13.9,"high":14.0,"low":0.154,"close":13.80,"volume":1.0},
        {"timestamp":"2024-01-02T00:00:00+00:00","open":13.9,"high":14.0,"low":13.8,"close":13.85,"volume":1.0}
    ]"#;
    cache.put_bars("alpaca:DCX:1Day", a).unwrap();
    cache.put_bars("yahoo-chart:DCX:1Day", b).unwrap();

    let report = cache.audit_bar_cache_sanity().unwrap();
    assert!(!report.has_code("cross_source_overlap_mismatch"), "{report:#?}");
    assert!(!report.has_code("cross_source_scale_blowout"), "{report:#?}");

    let _ = std::fs::remove_file(db_path);
}

#[test]
fn data_sanity_audit_classifies_runaway_scale_blowout_as_info() {
    let db_path = temp_db_path();
    let cache = SqliteCache::open(&db_path).unwrap();
    // Recent closes disagree by 1000× — runaway provider back-adjust, not a
    // plausible corporate action. Merge quarantines it; audit records context.
    let trusted = r#"[
        {"timestamp":"2024-01-01T00:00:00+00:00","open":0.8,"high":0.9,"low":0.7,"close":0.80,"volume":1.0},
        {"timestamp":"2024-01-02T00:00:00+00:00","open":0.8,"high":0.9,"low":0.7,"close":0.81,"volume":1.0}
    ]"#;
    let runaway = r#"[
        {"timestamp":"2024-01-01T00:00:00+00:00","open":800.0,"high":900.0,"low":700.0,"close":800.0,"volume":1.0},
        {"timestamp":"2024-01-02T00:00:00+00:00","open":800.0,"high":900.0,"low":700.0,"close":810.0,"volume":1.0}
    ]"#;
    cache.put_bars("alpaca:ADTX:1Day", trusted).unwrap();
    cache.put_bars("yahoo-chart:ADTX:1Day", runaway).unwrap();

    let report = cache.audit_bar_cache_sanity().unwrap();
    assert!(report.has_code("cross_source_scale_blowout"), "{report:#?}");
    assert!(!report.has_code("cross_source_overlap_mismatch"), "{report:#?}");
    let issue = report
        .issues
        .iter()
        .find(|i| i.code == "cross_source_scale_blowout")
        .unwrap();
    assert_eq!(issue.severity, BarCacheSanitySeverity::Info, "{report:#?}");

    let _ = std::fs::remove_file(db_path);
}

#[test]
fn delete_keys_light_removes_rows_without_reclaim() {
    let db_path = temp_db_path();
    let cache = SqliteCache::open(&db_path).unwrap();
    let bars = r#"[{"timestamp":"2024-01-02T00:00:00+00:00","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":10.0}]"#;
    cache.put_bars("merged:LIT:1Day", bars).unwrap();
    cache.put_bars("alpaca:LIT:1Day", bars).unwrap();

    let deleted = cache
        .delete_keys_light(&["merged:LIT:1Day".to_string()])
        .unwrap();
    assert_eq!(deleted, 1);
    assert!(cache.get_bars_raw("merged:LIT:1Day").unwrap().is_none());
    assert!(cache.get_bars_raw("alpaca:LIT:1Day").unwrap().is_some());

    let _ = std::fs::remove_file(db_path);
}
