//! Kraken WebSocket v2 book (Level 2) parser and state helpers.

use super::ws_v2::{
    build_ws_v2_subscribe_frame, build_ws_v2_unsubscribe_frame, ws_v2_frame_is_channel,
    ws_v2_json_f64, ws_v2_json_u64, ws_v2_timestamp_ms,
};

pub const KRAKEN_WS_V2_BOOK_CHANNEL: &str = "book";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KrakenWsBookLevel {
    pub price: f64,
    pub qty: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KrakenWsBookDelta {
    pub symbol: String,
    pub bids: Vec<KrakenWsBookLevel>,
    pub asks: Vec<KrakenWsBookLevel>,
    pub checksum: Option<u64>,
    pub ts_ms: Option<i64>,
    pub is_snapshot: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct KrakenWsBookState {
    pub symbol: String,
    pub bids: Vec<KrakenWsBookLevel>,
    pub asks: Vec<KrakenWsBookLevel>,
    pub depth: usize,
    pub last_checksum: Option<u64>,
    pub last_ts_ms: Option<i64>,
}

impl KrakenWsBookState {
    pub fn new(symbol: impl Into<String>, depth: usize) -> Self {
        Self {
            symbol: symbol.into(),
            depth,
            ..Self::default()
        }
    }

    pub fn apply_delta(&mut self, delta: &KrakenWsBookDelta) {
        self.symbol = delta.symbol.clone();
        if delta.is_snapshot {
            self.bids.clear();
            self.asks.clear();
        }
        apply_levels(&mut self.bids, &delta.bids, true);
        apply_levels(&mut self.asks, &delta.asks, false);
        if self.depth > 0 {
            self.bids.truncate(self.depth);
            self.asks.truncate(self.depth);
        }
        self.last_checksum = delta.checksum;
        self.last_ts_ms = delta.ts_ms;
    }
}

pub fn build_book_subscribe_frame(symbols: &[String], depth: usize, snapshot: bool) -> String {
    let mut params = serde_json::Map::new();
    params.insert("depth".into(), serde_json::json!(depth));
    params.insert("snapshot".into(), serde_json::Value::Bool(snapshot));
    build_ws_v2_subscribe_frame(KRAKEN_WS_V2_BOOK_CHANNEL, symbols, params)
}

pub fn build_book_unsubscribe_frame(symbols: &[String]) -> String {
    build_ws_v2_unsubscribe_frame(KRAKEN_WS_V2_BOOK_CHANNEL, symbols)
}

pub fn parse_book_message(text: &str) -> Vec<KrakenWsBookDelta> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return Vec::new();
    };
    let Some(is_snapshot) = ws_v2_frame_is_channel(&value, KRAKEN_WS_V2_BOOK_CHANNEL) else {
        return Vec::new();
    };
    if !is_snapshot
        && value.get("channel").and_then(|v| v.as_str()) != Some(KRAKEN_WS_V2_BOOK_CHANNEL)
    {
        return Vec::new();
    }
    let Some(data) = value.get("data").and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    data.iter()
        .filter_map(|entry| parse_book_entry(entry, is_snapshot))
        .collect()
}

fn parse_book_entry(entry: &serde_json::Value, is_snapshot: bool) -> Option<KrakenWsBookDelta> {
    let obj = entry.as_object()?;
    let symbol = obj.get("symbol")?.as_str()?.to_string();
    let asks = obj
        .get("asks")
        .and_then(|v| v.as_array())
        .map(|levels| parse_levels(levels))
        .unwrap_or_default();
    let bids = obj
        .get("bids")
        .and_then(|v| v.as_array())
        .map(|levels| parse_levels(levels))
        .unwrap_or_default();
    Some(KrakenWsBookDelta {
        symbol,
        bids,
        asks,
        checksum: obj.get("checksum").and_then(ws_v2_json_u64),
        ts_ms: obj
            .get("timestamp")
            .or_else(|| obj.get("time"))
            .and_then(ws_v2_timestamp_ms),
        is_snapshot,
    })
}

fn parse_levels(levels: &[serde_json::Value]) -> Vec<KrakenWsBookLevel> {
    levels.iter().filter_map(parse_level).collect()
}

fn parse_level(level: &serde_json::Value) -> Option<KrakenWsBookLevel> {
    if let Some(obj) = level.as_object() {
        return Some(KrakenWsBookLevel {
            price: obj.get("price").and_then(ws_v2_json_f64)?,
            qty: obj
                .get("qty")
                .or_else(|| obj.get("quantity"))
                .and_then(ws_v2_json_f64)?,
        });
    }
    let arr = level.as_array()?;
    Some(KrakenWsBookLevel {
        price: arr.first().and_then(ws_v2_json_f64)?,
        qty: arr.get(1).and_then(ws_v2_json_f64)?,
    })
}

fn apply_levels(side: &mut Vec<KrakenWsBookLevel>, levels: &[KrakenWsBookLevel], is_bid: bool) {
    for level in levels {
        if let Some(existing_idx) = side
            .iter()
            .position(|existing| (existing.price - level.price).abs() <= f64::EPSILON)
        {
            if level.qty <= 0.0 {
                side.remove(existing_idx);
            } else {
                side[existing_idx] = *level;
            }
        } else if level.qty > 0.0 {
            side.push(*level);
        }
    }
    if is_bid {
        side.sort_by(|a, b| {
            b.price
                .partial_cmp(&a.price)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    } else {
        side.sort_by(|a, b| {
            a.price
                .partial_cmp(&b.price)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn book_subscribe_frame_uses_depth_and_snapshot() {
        let frame = build_book_subscribe_frame(&["BTC/USD".into()], 10, true);
        let value: serde_json::Value = serde_json::from_str(&frame).unwrap();
        assert_eq!(value["method"], "subscribe");
        assert_eq!(value["params"]["channel"], "book");
        assert_eq!(value["params"]["symbol"][0], "BTC/USD");
        assert_eq!(value["params"]["depth"], 10);
        assert_eq!(value["params"]["snapshot"], true);
    }

    #[test]
    fn book_unsubscribe_frame_uses_v2_channel() {
        let frame = build_book_unsubscribe_frame(&["BTC/USD".into()]);
        let value: serde_json::Value = serde_json::from_str(&frame).unwrap();
        assert_eq!(value["method"], "unsubscribe");
        assert_eq!(value["params"]["channel"], "book");
        assert_eq!(value["params"]["symbol"][0], "BTC/USD");
    }

    #[test]
    fn parse_book_snapshot_accepts_object_levels() {
        let msg = r#"{
            "channel":"book",
            "type":"snapshot",
            "data":[{
                "symbol":"BTC/USD",
                "asks":[{"price":"67101.0","qty":"0.75"}],
                "bids":[{"price":67100.0,"qty":1.25}],
                "checksum":123456,
                "timestamp":"2026-06-06T11:00:00.000000Z"
            }]
        }"#;
        let rows = parse_book_message(msg);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].symbol, "BTC/USD");
        assert_eq!(rows[0].asks[0].price, 67101.0);
        assert_eq!(rows[0].asks[0].qty, 0.75);
        assert_eq!(rows[0].bids[0].price, 67100.0);
        assert_eq!(rows[0].checksum, Some(123456));
        assert!(rows[0].is_snapshot);
    }

    #[test]
    fn parse_book_update_accepts_array_levels() {
        let msg = r#"{
            "channel":"book",
            "type":"update",
            "data":[{
                "symbol":"BTC/USD",
                "asks":[["67102.0","1.5"]],
                "bids":[["67099.0","0"]],
                "checksum":"777"
            }]
        }"#;
        let rows = parse_book_message(msg);
        assert_eq!(rows.len(), 1);
        assert!(!rows[0].is_snapshot);
        assert_eq!(rows[0].asks[0].price, 67102.0);
        assert_eq!(rows[0].bids[0].qty, 0.0);
        assert_eq!(rows[0].checksum, Some(777));
    }

    #[test]
    fn book_state_applies_snapshot_then_deltas() {
        let mut state = KrakenWsBookState::new("BTC/USD", 2);
        state.apply_delta(&KrakenWsBookDelta {
            symbol: "BTC/USD".into(),
            bids: vec![
                KrakenWsBookLevel {
                    price: 100.0,
                    qty: 1.0,
                },
                KrakenWsBookLevel {
                    price: 99.0,
                    qty: 1.0,
                },
                KrakenWsBookLevel {
                    price: 98.0,
                    qty: 1.0,
                },
            ],
            asks: vec![KrakenWsBookLevel {
                price: 101.0,
                qty: 1.0,
            }],
            checksum: Some(1),
            ts_ms: None,
            is_snapshot: true,
        });
        assert_eq!(state.bids.len(), 2);
        assert_eq!(state.bids[0].price, 100.0);

        state.apply_delta(&KrakenWsBookDelta {
            symbol: "BTC/USD".into(),
            bids: vec![KrakenWsBookLevel {
                price: 100.0,
                qty: 0.0,
            }],
            asks: vec![KrakenWsBookLevel {
                price: 100.5,
                qty: 2.0,
            }],
            checksum: Some(2),
            ts_ms: None,
            is_snapshot: false,
        });
        assert_eq!(state.bids[0].price, 99.0);
        assert_eq!(state.asks[0].price, 100.5);
        assert_eq!(state.last_checksum, Some(2));
    }
}
