//! Kraken WS v2 OHLC streaming.
//!
//! Endpoint: `wss://ws.kraken.com/v2`. Subscribe to the `ohlc` channel with a
//! batch of symbols and one interval (minutes); Kraken pushes bar snapshots
//! plus per-tick updates as each bar evolves. A bar with the same
//! `interval_begin` is sent repeatedly; the last one before the interval
//! rolls over is the close.
//!
//! Why this exists at all: the public REST OHLC endpoint serialises every
//! request through a ~1 req/sec global counter, so 13k pairs × 9 timeframes
//! is unreachable for the low timeframes via REST alone. The WS push path
//! provides forward streaming so the cache stays current on 1Min/5Min/etc.
//! REST keeps doing cold-start historical backfill where it still wins.
//!
//! This module is the protocol layer: subscribe-frame batching, message
//! parsing into typed [`KrakenWsOhlcBar`]. The connection driver lives in
//! `connection.rs` alongside the reconnect / heartbeat logic.

use std::sync::atomic::{AtomicU64, Ordering};

pub const KRAKEN_WS_V2_URL: &str = "wss://ws.kraken.com/v2";

/// Kraken WS v2 caps subscribe frames at a few hundred symbols. We chunk at
/// 250 to stay comfortably under that ceiling without paying the per-frame
/// connect overhead too many times for large universes.
pub(crate) const KRAKEN_WS_SUBSCRIBE_BATCH: usize = 250;

/// Valid OHLC intervals (minutes) Kraken WS v2 serves on the `ohlc` channel.
/// Note: Kraken does not serve `MN1` natively; monthly bars are aggregated
/// from `1Day` by the existing REST path and the same aggregator can be
/// reused for the WS-fed daily bars.
pub const KRAKEN_WS_OHLC_INTERVALS_MIN: &[u32] = &[1, 5, 15, 30, 60, 240, 1440, 10080];

/// One bar emitted by the Kraken WS OHLC channel. `interval_begin_ms` is the
/// epoch-ms timestamp of the bar's left edge — i.e. the natural cache key
/// for upserting into the existing `kraken:SYMBOL:TF` bar series.
#[derive(Debug, Clone, PartialEq)]
pub struct KrakenWsOhlcBar {
    pub symbol: String,
    pub interval_min: u32,
    pub interval_begin_ms: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub vwap: Option<f64>,
    pub trades: u64,
    /// `true` for the initial snapshot batch on first subscribe; `false` for
    /// live updates. Snapshot bars overwrite anything we had cached at the
    /// same `interval_begin_ms`; update bars upsert by the same key.
    pub is_snapshot: bool,
}

/// Monotonic request-id source for outgoing WS frames. Kraken uses `req_id`
/// to correlate subscribe ACKs / NACKs with the originating subscribe.
static REQ_ID: AtomicU64 = AtomicU64::new(1);

pub(crate) fn next_req_id() -> u64 {
    REQ_ID.fetch_add(1, Ordering::Relaxed)
}

/// Build subscribe frames for the given (interval, symbols), chunked to
/// stay under Kraken's per-frame symbol cap. Each returned string is one
/// JSON message ready to send on the WS.
pub fn build_subscribe_frames(interval_min: u32, symbols: &[String]) -> Vec<String> {
    if symbols.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(symbols.len().div_ceil(KRAKEN_WS_SUBSCRIBE_BATCH));
    for batch in symbols.chunks(KRAKEN_WS_SUBSCRIBE_BATCH) {
        let frame = serde_json::json!({
            "method": "subscribe",
            "params": {
                "channel": "ohlc",
                "symbol": batch,
                "interval": interval_min,
                "snapshot": true,
            },
            "req_id": next_req_id(),
        });
        out.push(frame.to_string());
    }
    out
}

/// Build the matching unsubscribe frame (used during planned shutdown so
/// Kraken stops pushing into a connection we're about to close).
pub fn build_unsubscribe_frame(interval_min: u32, symbols: &[String]) -> Option<String> {
    if symbols.is_empty() {
        return None;
    }
    let frame = serde_json::json!({
        "method": "unsubscribe",
        "params": {
            "channel": "ohlc",
            "symbol": symbols,
            "interval": interval_min,
        },
        "req_id": next_req_id(),
    });
    Some(frame.to_string())
}

/// Parse one incoming WS text frame into zero-or-more bars. Returns an empty
/// vec for non-OHLC frames (heartbeats, subscribe ACKs, system status). Only
/// the `ohlc` channel produces bar output.
pub fn parse_ohlc_message(text: &str) -> Vec<KrakenWsOhlcBar> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return Vec::new();
    };
    let Some(obj) = value.as_object() else {
        return Vec::new();
    };
    // Only ohlc channel frames carry bars; everything else (status,
    // subscribe ACK, pong) is silent here.
    if obj.get("channel").and_then(|v| v.as_str()) != Some("ohlc") {
        return Vec::new();
    }
    let is_snapshot = obj.get("type").and_then(|v| v.as_str()) == Some("snapshot");
    let Some(data) = obj.get("data").and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    let mut bars = Vec::with_capacity(data.len());
    for entry in data {
        let Some(entry_obj) = entry.as_object() else {
            continue;
        };
        let Some(symbol) = entry_obj.get("symbol").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(interval) = entry_obj
            .get("interval")
            .and_then(|v| v.as_u64())
            .map(|n| n as u32)
        else {
            continue;
        };
        let Some(interval_begin_ms) = entry_obj
            .get("interval_begin")
            .and_then(|v| v.as_str())
            .and_then(parse_rfc3339_to_ms)
        else {
            continue;
        };
        let Some(open) = entry_obj.get("open").and_then(|v| v.as_f64()) else {
            continue;
        };
        let Some(high) = entry_obj.get("high").and_then(|v| v.as_f64()) else {
            continue;
        };
        let Some(low) = entry_obj.get("low").and_then(|v| v.as_f64()) else {
            continue;
        };
        let Some(close) = entry_obj.get("close").and_then(|v| v.as_f64()) else {
            continue;
        };
        // Reject obviously bad bars instead of poisoning the cache with NaN
        // or inverted high/low pairs.
        if ![open, high, low, close].iter().all(|v| v.is_finite())
            || high < low
            || open <= 0.0
            || close <= 0.0
        {
            continue;
        }
        let volume = entry_obj
            .get("volume")
            .and_then(|v| v.as_f64())
            .filter(|v| v.is_finite() && *v >= 0.0)
            .unwrap_or(0.0);
        let vwap = entry_obj
            .get("vwap")
            .and_then(|v| v.as_f64())
            .filter(|v| v.is_finite() && *v > 0.0);
        let trades = entry_obj
            .get("trades")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        bars.push(KrakenWsOhlcBar {
            symbol: symbol.to_string(),
            interval_min: interval,
            interval_begin_ms,
            open,
            high,
            low,
            close,
            volume,
            vwap,
            trades,
            is_snapshot,
        });
    }
    bars
}

/// Parse an RFC-3339 timestamp (the format Kraken uses for `interval_begin`)
/// into epoch milliseconds. Returns `None` for unparseable strings rather
/// than panicking on malformed feed data.
fn parse_rfc3339_to_ms(s: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&chrono::Utc).timestamp_millis())
}

/// Lightweight predicate for "this looks like a subscribe ACK". Used by the
/// connection driver to log subscription failures without trying to mine
/// channel data out of them.
pub fn is_subscribe_ack(text: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(text)
        .ok()
        .as_ref()
        .and_then(|v| v.get("method"))
        .and_then(|m| m.as_str())
        == Some("subscribe")
}

/// Pong/heartbeat frame so the connection driver can route them away from
/// the bar parser without an allocation.
pub fn is_heartbeat_or_status(text: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return false;
    };
    let channel = value
        .get("channel")
        .and_then(|v| v.as_str());
    matches!(channel, Some("heartbeat") | Some("status") | Some("pong"))
}

#[cfg(test)]
mod tests {
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
}
