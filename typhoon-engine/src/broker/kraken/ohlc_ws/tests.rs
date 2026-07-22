use super::*;

#[test]
fn build_subscribe_frames_batches_at_250() {
    let symbols: Vec<String> = (0..600).map(|i| format!("PAIR{i}/USD")).collect();
    let frames = build_subscribe_frames(1, &symbols);
    assert_eq!(frames.len(), 3); // 600 / 250 → ceil 3
    // First batch holds the first 250 pairs.
    let first: serde_json::Value = serde_json::from_str(&frames[0]).unwrap();
    let arr = first["params"]["symbol"].as_array().unwrap();
    assert_eq!(arr.len(), 250);
    assert_eq!(arr[0], "PAIR0/USD");
    assert_eq!(arr[249], "PAIR249/USD");
    // Last batch holds the leftover 100.
    let third: serde_json::Value = serde_json::from_str(&frames[2]).unwrap();
    let arr = third["params"]["symbol"].as_array().unwrap();
    assert_eq!(arr.len(), 100);
}

#[test]
fn subscribe_burst_delay_keeps_full_universe_startup_fast() {
    let symbols: Vec<String> = (0..13_000).map(|i| format!("PAIR{i}/USD")).collect();
    let batches = build_subscribe_frames(1, &symbols).len() as u32;
    let expected_elapsed = KRAKEN_WS_SUBSCRIBE_FRAME_DELAY * batches;
    assert!(
        expected_elapsed <= Duration::from_secs(2),
        "full-universe subscribe snapshot should be online in <=2s per interval, got {expected_elapsed:?}"
    );
}

#[test]
fn build_subscribe_frames_emits_canonical_v2_shape() {
    let frames = build_subscribe_frames(60, &["BTC/USD".to_string()]);
    let v: serde_json::Value = serde_json::from_str(&frames[0]).unwrap();
    assert_eq!(v["method"], "subscribe");
    assert_eq!(v["params"]["channel"], "ohlc");
    assert_eq!(v["params"]["interval"], 60);
    assert_eq!(v["params"]["snapshot"], true);
    assert!(v["req_id"].is_number());
}

#[test]
fn build_subscribe_frames_can_disable_initial_snapshot() {
    let frames = build_subscribe_frames_with_snapshot(1, &["AAPLx/USD".to_string()], false);
    let v: serde_json::Value = serde_json::from_str(&frames[0]).unwrap();
    assert_eq!(v["params"]["snapshot"], false);
}

#[test]
fn build_subscribe_frames_empty_input_returns_empty() {
    assert!(build_subscribe_frames(1, &[]).is_empty());
}

#[test]
fn build_unsubscribe_frame_carries_method_and_pairs() {
    let frame = build_unsubscribe_frame(1, &["BTC/USD".into(), "ETH/USD".into()]).unwrap();
    let v: serde_json::Value = serde_json::from_str(&frame).unwrap();
    assert_eq!(v["method"], "unsubscribe");
    assert_eq!(v["params"]["channel"], "ohlc");
    assert_eq!(v["params"]["interval"], 1);
    assert_eq!(v["params"]["symbol"][0], "BTC/USD");
}

#[test]
fn build_unsubscribe_frame_empty_input_returns_none() {
    assert!(build_unsubscribe_frame(1, &[]).is_none());
}

#[test]
fn parse_ohlc_message_handles_snapshot_with_multiple_bars() {
    let text = r#"{
        "channel": "ohlc",
        "type": "snapshot",
        "data": [
            {
                "symbol": "BTC/USD",
                "open": 50000.0,
                "high": 50100.0,
                "low": 49900.0,
                "close": 50050.0,
                "volume": 1.5,
                "vwap": 50025.0,
                "trades": 25,
                "interval_begin": "2026-05-23T19:00:00.000000Z",
                "interval": 60,
                "timestamp": "2026-05-23T19:00:30.123456Z"
            },
            {
                "symbol": "ETH/USD",
                "open": 2500.0,
                "high": 2510.0,
                "low": 2495.0,
                "close": 2508.0,
                "volume": 10.0,
                "trades": 100,
                "interval_begin": "2026-05-23T19:00:00.000000Z",
                "interval": 60,
                "timestamp": "2026-05-23T19:00:30.123456Z"
            }
        ]
    }"#;
    let bars = parse_ohlc_message(text);
    assert_eq!(bars.len(), 2);
    assert_eq!(bars[0].symbol, "BTC/USD");
    assert_eq!(bars[0].interval_min, 60);
    assert!(bars[0].is_snapshot);
    assert!((bars[0].open - 50000.0).abs() < f64::EPSILON);
    assert_eq!(bars[0].vwap, Some(50025.0));
    assert_eq!(bars[0].trades, 25);
    // 2026-05-23T19:00:00Z → ms since epoch (matches chrono parsing).
    let expected_ms = chrono::DateTime::parse_from_rfc3339("2026-05-23T19:00:00.000000Z")
        .unwrap()
        .timestamp_millis();
    assert_eq!(bars[0].interval_begin_ms, expected_ms);
    assert_eq!(bars[1].symbol, "ETH/USD");
    assert!(bars[1].vwap.is_none()); // missing vwap → None
}

#[test]
fn parse_ohlc_message_can_drop_snapshot_frames_for_live_only_large_universe() {
    let text = r#"{
        "channel": "ohlc",
        "type": "snapshot",
        "data": [{
            "symbol": "AAPLx/USD",
            "open": 100.0, "high": 101.0, "low": 99.0, "close": 100.5,
            "volume": 10.0, "trades": 3,
            "interval_begin": "2026-05-23T19:00:00Z", "interval": 1
        }]
    }"#;
    assert!(parse_ohlc_message_with_snapshot_policy(text, false).is_empty());
    assert_eq!(parse_ohlc_message_with_snapshot_policy(text, true).len(), 1);
}

#[test]
fn parse_ohlc_message_marks_update_frames_correctly() {
    let text = r#"{
        "channel": "ohlc",
        "type": "update",
        "data": [{
            "symbol": "BTC/USD",
            "open": 50000.0, "high": 50100.0, "low": 49900.0, "close": 50050.0,
            "volume": 1.5, "trades": 25,
            "interval_begin": "2026-05-23T19:00:00Z", "interval": 60
        }]
    }"#;
    let bars = parse_ohlc_message(text);
    assert_eq!(bars.len(), 1);
    assert!(!bars[0].is_snapshot);
}

#[test]
fn parse_ohlc_message_rejects_non_ohlc_channel() {
    let text = r#"{
        "channel": "ticker",
        "type": "update",
        "data": [{"symbol": "BTC/USD"}]
    }"#;
    assert!(parse_ohlc_message(text).is_empty());
}

#[test]
fn parse_ohlc_message_rejects_invalid_json() {
    assert!(parse_ohlc_message("not json").is_empty());
    assert!(parse_ohlc_message("").is_empty());
}

#[test]
fn parse_ohlc_message_drops_bars_with_missing_fields() {
    let text = r#"{
        "channel": "ohlc",
        "type": "update",
        "data": [
            { "symbol": "BTC/USD", "open": 50000.0, "high": 50100.0, "low": 49900.0,
              "close": 50050.0, "interval_begin": "2026-05-23T19:00:00Z", "interval": 60 },
            { "open": 1.0, "high": 1.0, "low": 1.0, "close": 1.0,
              "interval_begin": "2026-05-23T19:00:00Z", "interval": 60 }
        ]
    }"#;
    let bars = parse_ohlc_message(text);
    // Second bar has no symbol → dropped.
    assert_eq!(bars.len(), 1);
    assert_eq!(bars[0].symbol, "BTC/USD");
}

#[test]
fn parse_ohlc_message_drops_inverted_or_negative_bars() {
    let text = r#"{
        "channel": "ohlc",
        "type": "update",
        "data": [
            { "symbol": "BAD1", "open": 100.0, "high": 50.0, "low": 99.0, "close": 99.0,
              "interval_begin": "2026-05-23T19:00:00Z", "interval": 60 },
            { "symbol": "BAD2", "open": 0.0, "high": 1.0, "low": 0.0, "close": 0.5,
              "interval_begin": "2026-05-23T19:00:00Z", "interval": 60 }
        ]
    }"#;
    // Both rejected: BAD1 has high<low; BAD2 has open=0.
    assert!(parse_ohlc_message(text).is_empty());
}

#[test]
fn parse_ohlc_message_drops_bars_with_unparseable_timestamp() {
    let text = r#"{
        "channel": "ohlc",
        "type": "update",
        "data": [{
            "symbol": "BTC/USD",
            "open": 50000.0, "high": 50100.0, "low": 49900.0, "close": 50050.0,
            "interval_begin": "not-an-rfc3339-stamp", "interval": 60
        }]
    }"#;
    assert!(parse_ohlc_message(text).is_empty());
}

#[test]
fn is_subscribe_ack_matches_only_subscribe_method() {
    assert!(is_subscribe_ack(r#"{"method":"subscribe","success":true}"#));
    assert!(!is_subscribe_ack(r#"{"channel":"ohlc"}"#));
    assert!(!is_subscribe_ack("garbage"));
}

#[test]
fn is_heartbeat_or_status_matches_known_channels() {
    assert!(is_heartbeat_or_status(r#"{"channel":"heartbeat"}"#));
    assert!(is_heartbeat_or_status(r#"{"channel":"status","data":[]}"#));
    assert!(is_heartbeat_or_status(r#"{"channel":"pong"}"#));
    assert!(!is_heartbeat_or_status(r#"{"channel":"ohlc"}"#));
}

#[test]
fn next_req_id_returns_monotonic_ids() {
    let a = next_req_id();
    let b = next_req_id();
    let c = next_req_id();
    assert!(b > a);
    assert!(c > b);
}

#[test]
fn compute_reconnect_backoff_doubles_each_failure() {
    assert_eq!(compute_reconnect_backoff(0), Duration::from_secs(1));
    assert_eq!(compute_reconnect_backoff(1), Duration::from_secs(2));
    assert_eq!(compute_reconnect_backoff(2), Duration::from_secs(4));
    assert_eq!(compute_reconnect_backoff(3), Duration::from_secs(8));
    assert_eq!(compute_reconnect_backoff(4), Duration::from_secs(16));
    assert_eq!(compute_reconnect_backoff(5), Duration::from_secs(32));
}

#[test]
fn compute_reconnect_backoff_caps_at_60_seconds() {
    assert_eq!(compute_reconnect_backoff(6), KRAKEN_WS_RECONNECT_MAX);
    assert_eq!(compute_reconnect_backoff(20), KRAKEN_WS_RECONNECT_MAX);
    // Saturates without panicking on absurd inputs.
    assert_eq!(compute_reconnect_backoff(u32::MAX), KRAKEN_WS_RECONNECT_MAX);
}

#[tokio::test]
async fn run_ohlc_streamer_returns_immediately_when_consumer_closes_channel() {
    // Drop the receiver before spawning so bar_tx.is_closed() returns
    // true on entry; the function must exit cleanly without trying
    // to connect to Kraken (this test runs offline).
    let (bar_tx, bar_rx) = mpsc::channel(1);
    drop(bar_rx);
    let (event_tx, _event_rx) = mpsc::unbounded_channel();
    let fut = run_ohlc_streamer(1, vec!["BTC/USD".to_string()], bar_tx, event_tx);
    // Must complete; a hung future would mean we tried to dial Kraken.
    tokio::time::timeout(Duration::from_secs(1), fut)
        .await
        .expect("streamer must exit when bar channel is closed");
}

#[tokio::test]
async fn run_ohlc_streamer_no_op_for_empty_pair_list() {
    let (bar_tx, _bar_rx) = mpsc::channel(1);
    let (event_tx, _event_rx) = mpsc::unbounded_channel();
    let fut = run_ohlc_streamer(1, Vec::new(), bar_tx, event_tx);
    tokio::time::timeout(Duration::from_secs(1), fut)
        .await
        .expect("streamer must exit on empty pair list without dialing");
}

#[test]
fn kraken_ws_interval_to_tf_label_covers_every_served_interval() {
    for &interval in KRAKEN_WS_OHLC_INTERVALS_MIN {
        assert!(
            kraken_ws_interval_to_tf_label(interval).is_some(),
            "interval {interval} from KRAKEN_WS_OHLC_INTERVALS_MIN must map"
        );
    }
    assert_eq!(kraken_ws_interval_to_tf_label(1), Some("1Min"));
    assert_eq!(kraken_ws_interval_to_tf_label(1440), Some("1Day"));
    assert_eq!(kraken_ws_interval_to_tf_label(10080), Some("1Week"));
    // Unknown interval (e.g. monthly via 21600) returns None — Kraken WS
    // doesn't serve it, so the writer must not invent a cache key.
    assert!(kraken_ws_interval_to_tf_label(21600).is_none());
    assert!(kraken_ws_interval_to_tf_label(0).is_none());
}

#[test]
fn kraken_ws_bar_to_json_matches_cache_shape() {
    let bar = KrakenWsOhlcBar {
        symbol: "BTC/USD".into(),
        interval_min: 60,
        interval_begin_ms: 1_700_000_000_000,
        open: 50_000.0,
        high: 50_100.0,
        low: 49_900.0,
        close: 50_050.0,
        volume: 1.5,
        vwap: Some(50_025.0),
        trades: 25,
        is_snapshot: true,
    };
    let json = kraken_ws_bar_to_json(&bar);
    assert_eq!(json["open"], 50_000.0);
    assert_eq!(json["high"], 50_100.0);
    assert_eq!(json["low"], 49_900.0);
    assert_eq!(json["close"], 50_050.0);
    assert_eq!(json["volume"], 1.5);
    // Cache merger keys by RFC-3339 timestamp — must round-trip through
    // chrono parse without losing the bar.
    let ts = json["timestamp"].as_str().expect("timestamp string");
    let parsed = chrono::DateTime::parse_from_rfc3339(ts).expect("parseable");
    assert_eq!(parsed.timestamp_millis(), 1_700_000_000_000);
}

#[test]
fn ws_bar_is_closed_returns_false_while_bucket_is_open() {
    // 1Min bar starting at 10:00:00; now is 10:00:30 → bucket runs to 10:01:00, still open.
    let begin = 1_700_000_000_000;
    let now = begin + 30_000;
    assert!(!ws_bar_is_closed(1, begin, now));
}

#[test]
fn ws_bar_is_closed_returns_true_at_or_after_bucket_end() {
    let begin = 1_700_000_000_000;
    // Exactly at the closing instant: bucket end is inclusive of "closed" so a
    // bar whose interval has just rolled is flushable.
    assert!(ws_bar_is_closed(1, begin, begin + 60_000));
    assert!(ws_bar_is_closed(1, begin, begin + 60_000 + 1));
}

#[test]
fn ws_bar_is_closed_scales_with_interval_minutes() {
    let begin = 1_700_000_000_000;
    // 5Min bar: still open at +4min, closed at +5min.
    assert!(!ws_bar_is_closed(5, begin, begin + 4 * 60_000));
    assert!(ws_bar_is_closed(5, begin, begin + 5 * 60_000));
    // 1Day bar: still open at +23h, closed at +24h.
    assert!(!ws_bar_is_closed(1440, begin, begin + 23 * 3_600_000));
    assert!(ws_bar_is_closed(1440, begin, begin + 24 * 3_600_000));
}

#[test]
fn ws_bar_is_closed_snapshot_historical_bars_always_flushable() {
    // Snapshot delivery brings closed historical bars whose interval_begin is
    // far in the past relative to now. Every served interval should report
    // them as closed so the first flush persists the backfill.
    let now = 1_700_000_000_000;
    for &interval_min in KRAKEN_WS_OHLC_INTERVALS_MIN {
        // Place the bar two full periods in the past.
        let begin = now - (interval_min as i64) * 60_000 * 2;
        assert!(
            ws_bar_is_closed(interval_min, begin, now),
            "interval {interval_min} historical bar must be reported closed",
        );
    }
}

#[test]
fn ws_bar_is_closed_handles_future_begin_without_overflow() {
    // Defensive: clock skew or a malformed feed could push interval_begin into
    // the future. Must not overflow and must report not-closed.
    let now = 1_700_000_000_000;
    assert!(!ws_bar_is_closed(1, now + 10 * 60_000, now));
    // Absurd extremes saturate to a sane "still open" answer rather than
    // wrapping around to a phantom past close.
    assert!(!ws_bar_is_closed(u32::MAX, i64::MAX, now));
}

#[test]
fn kraken_ws_symbol_to_cache_key_drops_slash_and_normalises_xbt() {
    // XBT (Kraken's legacy BTC code) folds into BTC via the existing
    // pair normaliser so WS-written keys collide with REST-written keys.
    assert_eq!(kraken_ws_symbol_to_cache_key("XBT/USD"), "BTCUSD");
    assert_eq!(kraken_ws_symbol_to_cache_key("BTC/USD"), "BTCUSD");
    assert_eq!(kraken_ws_symbol_to_cache_key("ETH/USD"), "ETHUSD");
    // Already-flat form passes through.
    assert_eq!(kraken_ws_symbol_to_cache_key("ETHUSD"), "ETHUSD");
}
