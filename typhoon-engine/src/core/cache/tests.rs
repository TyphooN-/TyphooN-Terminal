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

#[serial_test::serial]
#[test]
fn live_bar_writes_use_fast_zstd_level_not_idle_compaction_level() {
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

    assert_eq!(level, 3);
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
