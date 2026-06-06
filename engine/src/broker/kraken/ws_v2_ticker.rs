//! Kraken WebSocket v2 ticker (Level 1) parser and subscription helpers.

use super::ws_v2::{
    build_ws_v2_subscribe_frame, build_ws_v2_unsubscribe_frame, ws_v2_frame_is_channel,
    ws_v2_json_f64, ws_v2_timestamp_ms,
};

pub const KRAKEN_WS_V2_TICKER_CHANNEL: &str = "ticker";

#[derive(Debug, Clone, PartialEq)]
pub struct KrakenWsTicker {
    pub symbol: String,
    pub bid: Option<f64>,
    pub bid_qty: Option<f64>,
    pub ask: Option<f64>,
    pub ask_qty: Option<f64>,
    pub last: Option<f64>,
    pub volume_24h: Option<f64>,
    pub vwap_24h: Option<f64>,
    pub low_24h: Option<f64>,
    pub high_24h: Option<f64>,
    pub change_24h: Option<f64>,
    pub change_pct_24h: Option<f64>,
    pub ts_ms: Option<i64>,
    pub is_snapshot: bool,
}

pub fn build_ticker_subscribe_frame(symbols: &[String], snapshot: bool) -> String {
    let mut params = serde_json::Map::new();
    params.insert("snapshot".into(), serde_json::Value::Bool(snapshot));
    build_ws_v2_subscribe_frame(KRAKEN_WS_V2_TICKER_CHANNEL, symbols, params)
}

pub fn build_ticker_unsubscribe_frame(symbols: &[String]) -> String {
    build_ws_v2_unsubscribe_frame(KRAKEN_WS_V2_TICKER_CHANNEL, symbols)
}

pub fn parse_ticker_message(text: &str) -> Vec<KrakenWsTicker> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return Vec::new();
    };
    let Some(is_snapshot) = ws_v2_frame_is_channel(&value, KRAKEN_WS_V2_TICKER_CHANNEL) else {
        return Vec::new();
    };
    if !is_snapshot
        && value.get("channel").and_then(|v| v.as_str()) != Some(KRAKEN_WS_V2_TICKER_CHANNEL)
    {
        return Vec::new();
    }
    let Some(data) = value.get("data").and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    data.iter()
        .filter_map(|entry| parse_ticker_entry(entry, is_snapshot))
        .collect()
}

fn parse_ticker_entry(entry: &serde_json::Value, is_snapshot: bool) -> Option<KrakenWsTicker> {
    let obj = entry.as_object()?;
    let symbol = obj.get("symbol")?.as_str()?.to_string();
    Some(KrakenWsTicker {
        symbol,
        bid: get_any_f64(obj, &["bid", "best_bid"]),
        bid_qty: get_any_f64(obj, &["bid_qty", "bid_quantity", "best_bid_qty"]),
        ask: get_any_f64(obj, &["ask", "best_ask"]),
        ask_qty: get_any_f64(obj, &["ask_qty", "ask_quantity", "best_ask_qty"]),
        last: get_any_f64(obj, &["last", "last_price", "price"]),
        volume_24h: get_any_f64(obj, &["volume", "volume_24h"]),
        vwap_24h: get_any_f64(obj, &["vwap", "vwap_24h"]),
        low_24h: get_any_f64(obj, &["low", "low_24h"]),
        high_24h: get_any_f64(obj, &["high", "high_24h"]),
        change_24h: get_any_f64(obj, &["change", "change_24h"]),
        change_pct_24h: get_any_f64(obj, &["change_pct", "change_pct_24h"]),
        ts_ms: obj
            .get("timestamp")
            .or_else(|| obj.get("time"))
            .and_then(ws_v2_timestamp_ms),
        is_snapshot,
    })
}

fn get_any_f64(obj: &serde_json::Map<String, serde_json::Value>, keys: &[&str]) -> Option<f64> {
    keys.iter()
        .filter_map(|key| obj.get(*key))
        .find_map(ws_v2_json_f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticker_subscribe_frame_uses_v2_channel() {
        let frame = build_ticker_subscribe_frame(&["BTC/USD".into()], true);
        let value: serde_json::Value = serde_json::from_str(&frame).unwrap();
        assert_eq!(value["method"], "subscribe");
        assert_eq!(value["params"]["channel"], "ticker");
        assert_eq!(value["params"]["symbol"][0], "BTC/USD");
        assert_eq!(value["params"]["snapshot"], true);
    }

    #[test]
    fn ticker_unsubscribe_frame_uses_v2_channel() {
        let frame = build_ticker_unsubscribe_frame(&["BTC/USD".into()]);
        let value: serde_json::Value = serde_json::from_str(&frame).unwrap();
        assert_eq!(value["method"], "unsubscribe");
        assert_eq!(value["params"]["channel"], "ticker");
        assert_eq!(value["params"]["symbol"][0], "BTC/USD");
    }

    #[test]
    fn parse_ticker_snapshot_accepts_doc_shape() {
        let msg = r#"{
            "channel":"ticker",
            "type":"snapshot",
            "data":[{
                "symbol":"BTC/USD",
                "bid":67100.1,
                "bid_qty":1.25,
                "ask":"67101.2",
                "ask_qty":"0.75",
                "last":67100.9,
                "volume":1234.5,
                "vwap":67000.0,
                "low":66000.0,
                "high":68000.0,
                "change":100.0,
                "change_pct":0.15,
                "timestamp":"2026-06-06T11:00:00.000000Z"
            }]
        }"#;
        let rows = parse_ticker_message(msg);
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row.symbol, "BTC/USD");
        assert_eq!(row.bid, Some(67100.1));
        assert_eq!(row.ask, Some(67101.2));
        assert_eq!(row.ask_qty, Some(0.75));
        assert_eq!(row.last, Some(67100.9));
        assert!(row.is_snapshot);
        assert!(row.ts_ms.is_some());
    }

    #[test]
    fn parse_ticker_ignores_non_ticker_frames() {
        assert!(parse_ticker_message(r#"{"channel":"heartbeat"}"#).is_empty());
    }
}
