//! Shared Kraken WebSocket v2 protocol helpers.
//!
//! Keep this module intentionally small. Channel-specific payload parsing
//! belongs in `ws_v2_ticker.rs`, `ws_v2_book.rs`, `ws_v2_trade.rs`, etc.
//! The goal is to prevent another monolithic Kraken protocol file.

use std::sync::atomic::{AtomicU64, Ordering};

pub const KRAKEN_WS_V2_PUBLIC_URL: &str = "wss://ws.kraken.com/v2";
pub const KRAKEN_WS_V2_AUTH_URL: &str = "wss://ws-auth.kraken.com/v2";
pub const KRAKEN_WS_V2_LEVEL3_URL: &str = "wss://ws-l3.kraken.com/v2";

static WS_V2_REQ_ID: AtomicU64 = AtomicU64::new(10_000);

pub(crate) fn next_ws_v2_req_id() -> u64 {
    WS_V2_REQ_ID.fetch_add(1, Ordering::Relaxed)
}

pub fn build_ws_v2_subscribe_frame(
    channel: &str,
    symbols: &[String],
    mut extra_params: serde_json::Map<String, serde_json::Value>,
) -> String {
    extra_params.insert(
        "channel".to_string(),
        serde_json::Value::String(channel.to_string()),
    );
    if !symbols.is_empty() {
        extra_params.insert(
            "symbol".to_string(),
            serde_json::Value::Array(
                symbols
                    .iter()
                    .map(|s| serde_json::Value::String(s.clone()))
                    .collect(),
            ),
        );
    }
    serde_json::json!({
        "method": "subscribe",
        "params": extra_params,
        "req_id": next_ws_v2_req_id(),
    })
    .to_string()
}

pub fn build_ws_v2_unsubscribe_frame(channel: &str, symbols: &[String]) -> String {
    let mut params = serde_json::Map::new();
    params.insert(
        "channel".to_string(),
        serde_json::Value::String(channel.to_string()),
    );
    if !symbols.is_empty() {
        params.insert(
            "symbol".to_string(),
            serde_json::Value::Array(
                symbols
                    .iter()
                    .map(|s| serde_json::Value::String(s.clone()))
                    .collect(),
            ),
        );
    }
    serde_json::json!({
        "method": "unsubscribe",
        "params": params,
        "req_id": next_ws_v2_req_id(),
    })
    .to_string()
}

pub fn parse_ws_v2_channel_type(value: &serde_json::Value) -> Option<(&str, &str)> {
    let channel = value.get("channel").and_then(|v| v.as_str())?;
    let frame_type = value
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("update");
    Some((channel, frame_type))
}

pub fn ws_v2_frame_is_channel(value: &serde_json::Value, expected_channel: &str) -> Option<bool> {
    let (channel, frame_type) = parse_ws_v2_channel_type(value)?;
    if channel == expected_channel {
        Some(frame_type == "snapshot")
    } else {
        Some(false)
    }
}

pub fn ws_v2_json_f64(value: &serde_json::Value) -> Option<f64> {
    match value {
        serde_json::Value::Number(n) => n.as_f64(),
        serde_json::Value::String(s) => s.parse::<f64>().ok(),
        _ => None,
    }
    .filter(|v| v.is_finite())
}

pub fn ws_v2_json_u64(value: &serde_json::Value) -> Option<u64> {
    match value {
        serde_json::Value::Number(n) => n.as_u64(),
        serde_json::Value::String(s) => s.parse::<u64>().ok(),
        _ => None,
    }
}

pub fn ws_v2_json_i64(value: &serde_json::Value) -> Option<i64> {
    match value {
        serde_json::Value::Number(n) => n.as_i64(),
        serde_json::Value::String(s) => s.parse::<i64>().ok(),
        _ => None,
    }
}

pub fn ws_v2_timestamp_ms(value: &serde_json::Value) -> Option<i64> {
    if let Some(ms) = ws_v2_json_i64(value) {
        return Some(ms);
    }
    value
        .as_str()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc).timestamp_millis())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KrakenWsV2Ack {
    pub method: String,
    pub success: Option<bool>,
    pub channel: Option<String>,
    pub error: Option<String>,
    pub req_id: Option<u64>,
}

pub fn parse_ws_v2_ack(text: &str) -> Option<KrakenWsV2Ack> {
    let value = serde_json::from_str::<serde_json::Value>(text).ok()?;
    let method = value.get("method")?.as_str()?.to_string();
    if !matches!(method.as_str(), "subscribe" | "unsubscribe") {
        return None;
    }
    let params = value.get("params");
    Some(KrakenWsV2Ack {
        method,
        success: value.get("success").and_then(|v| v.as_bool()),
        channel: params
            .and_then(|p| p.get("channel"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        error: value
            .get("error")
            .or_else(|| value.get("error_message"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        req_id: value.get("req_id").and_then(ws_v2_json_u64),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscribe_frame_builds_canonical_v2_shape() {
        let mut extra = serde_json::Map::new();
        extra.insert("snapshot".into(), serde_json::Value::Bool(true));
        let frame = build_ws_v2_subscribe_frame("ticker", &["BTC/USD".into()], extra);
        let value: serde_json::Value = serde_json::from_str(&frame).unwrap();
        assert_eq!(value["method"], "subscribe");
        assert_eq!(value["params"]["channel"], "ticker");
        assert_eq!(value["params"]["symbol"][0], "BTC/USD");
        assert_eq!(value["params"]["snapshot"], true);
        assert!(value["req_id"].is_u64());
    }

    #[test]
    fn parse_ack_accepts_subscribe_response() {
        let ack = parse_ws_v2_ack(
            r#"{"method":"subscribe","success":true,"params":{"channel":"book"},"req_id":42}"#,
        )
        .unwrap();
        assert_eq!(ack.method, "subscribe");
        assert_eq!(ack.success, Some(true));
        assert_eq!(ack.channel.as_deref(), Some("book"));
        assert_eq!(ack.req_id, Some(42));
    }
}
