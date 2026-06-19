//! Kraken public order-book WebSocket plumbing.
//!
//! Kraken's public WS v1 (`wss://ws.kraken.com`) delivers the `book-N` channel
//! as a JSON array: `[channelId, payload, "book-10", "XBT/USD"]`. The payload
//! is either an initial snapshot (`as`/`bs` keys) or a delta (`a`/`b` keys);
//! both shapes funnel through `apply_kraken_public_book_message`. Price
//! levels arrive as `[price, size, timestamp, ...]` arrays — `size = 0` means
//! remove the level. Output JSON uses TyphooN's normalised `{price, size}`
//! representation so the rest of the app doesn't have to know the raw shape.

use futures_util::SinkExt;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

pub(super) async fn connect_kraken_public_book_once(
    ws_pair: &str,
    depth: usize,
) -> Result<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    String,
> {
    let (mut ws_stream, _) = connect_async("wss://ws.kraken.com")
        .await
        .map_err(|e| format!("Kraken public WS connect failed: {e}"))?;
    let sub = serde_json::json!({
        "event": "subscribe",
        "pair": [ws_pair],
        "subscription": { "name": "book", "depth": depth }
    });
    ws_stream
        .send(Message::Text(sub.to_string().into()))
        .await
        .map_err(|e| format!("Kraken public WS subscribe failed: {e}"))?;
    Ok(ws_stream)
}

pub(super) fn apply_kraken_public_book_message(
    text: &str,
    bids: &mut Vec<(f64, f64)>,
    asks: &mut Vec<(f64, f64)>,
    depth: usize,
) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return false;
    };
    let Some(arr) = value.as_array() else {
        return false;
    };
    if !arr
        .iter()
        .any(|v| v.as_str().is_some_and(|s| s.starts_with("book-")))
    {
        return false;
    }
    let mut changed = false;
    for payload in arr.iter().filter_map(|v| v.as_object()) {
        if let Some(levels) = payload.get("as").and_then(|v| v.as_array()) {
            asks.clear();
            apply_kraken_book_levels(asks, levels);
            changed = true;
        }
        if let Some(levels) = payload.get("bs").and_then(|v| v.as_array()) {
            bids.clear();
            apply_kraken_book_levels(bids, levels);
            changed = true;
        }
        if let Some(levels) = payload.get("a").and_then(|v| v.as_array()) {
            apply_kraken_book_levels(asks, levels);
            changed = true;
        }
        if let Some(levels) = payload.get("b").and_then(|v| v.as_array()) {
            apply_kraken_book_levels(bids, levels);
            changed = true;
        }
    }
    if changed {
        bids.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        asks.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        bids.truncate(depth);
        asks.truncate(depth);
    }
    changed
}

fn apply_kraken_book_levels(side: &mut Vec<(f64, f64)>, levels: &[serde_json::Value]) {
    for level in levels {
        let Some(arr) = level.as_array() else {
            continue;
        };
        let Some(price) = arr.first().and_then(kraken_json_f64) else {
            continue;
        };
        let Some(size) = arr.get(1).and_then(kraken_json_f64) else {
            continue;
        };
        if let Some(existing_idx) = side
            .iter()
            .position(|(existing_price, _)| (*existing_price - price).abs() <= f64::EPSILON)
        {
            if size <= 0.0 {
                side.remove(existing_idx);
            } else {
                side[existing_idx] = (price, size);
            }
        } else if size > 0.0 {
            side.push((price, size));
        }
    }
}

fn kraken_json_f64(value: &serde_json::Value) -> Option<f64> {
    match value {
        serde_json::Value::String(s) => s.parse::<f64>().ok(),
        serde_json::Value::Number(n) => n.as_f64(),
        _ => None,
    }
    .filter(|v| v.is_finite())
}

pub(super) fn kraken_public_book_snapshot_json(
    display_symbol: &str,
    ws_pair: &str,
    bids: &[(f64, f64)],
    asks: &[(f64, f64)],
) -> String {
    let side_json = |levels: &[(f64, f64)]| -> Vec<serde_json::Value> {
        levels
            .iter()
            .map(|(price, size)| serde_json::json!({ "price": price, "size": size }))
            .collect()
    };
    serde_json::json!({
        "source": "kraken_ws",
        "symbol": display_symbol,
        "ws_pair": ws_pair,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "bids": side_json(bids),
        "asks": side_json(asks),
    })
    .to_string()
}

pub(super) fn kraken_public_book_status_message(
    display_symbol: &str,
    ws_pair: &str,
    status: &str,
) -> String {
    serde_json::json!({
        "source": "kraken_ws",
        "symbol": display_symbol,
        "ws_pair": ws_pair,
        "status": status,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "bids": [],
        "asks": [],
    })
    .to_string()
}

pub(super) fn kraken_public_book_error_message(
    display_symbol: &str,
    ws_pair: &str,
    error: &str,
) -> String {
    serde_json::json!({
        "source": "kraken_ws",
        "symbol": display_symbol,
        "ws_pair": ws_pair,
        "status": "error",
        "error": error,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "bids": [],
        "asks": [],
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_kraken_public_book_message_seeds_snapshot_from_as_bs_keys() {
        let mut bids = Vec::new();
        let mut asks = Vec::new();
        let text = r#"[123, {
            "as": [["50001.0", "1.0", "1700000000.0"], ["50002.0", "2.0", "1700000001.0"]],
            "bs": [["49999.0", "3.0", "1700000000.0"], ["49998.0", "4.0", "1700000001.0"]]
        }, "book-10", "XBT/USD"]"#;
        assert!(apply_kraken_public_book_message(
            text, &mut bids, &mut asks, 10
        ));
        assert_eq!(asks, vec![(50001.0, 1.0), (50002.0, 2.0)]);
        assert_eq!(bids, vec![(49999.0, 3.0), (49998.0, 4.0)]);
    }

    #[test]
    fn apply_kraken_public_book_message_applies_a_b_deltas() {
        let mut bids = vec![(49999.0, 3.0)];
        let mut asks = vec![(50001.0, 1.0)];
        let text = r#"[123, {
            "a": [["50001.0", "5.0", "1700000010.0"]],
            "b": [["49999.0", "0.0", "1700000010.0"]]
        }, "book-10", "XBT/USD"]"#;
        assert!(apply_kraken_public_book_message(
            text, &mut bids, &mut asks, 10
        ));
        assert_eq!(asks, vec![(50001.0, 5.0)]);
        assert!(bids.is_empty(), "size=0 must remove the level");
    }

    #[test]
    fn apply_kraken_public_book_message_truncates_to_depth() {
        let mut bids = Vec::new();
        let mut asks = Vec::new();
        let text = r#"[123, {
            "as": [["50001.0","1","_"], ["50002.0","1","_"], ["50003.0","1","_"]],
            "bs": [["49999.0","1","_"], ["49998.0","1","_"], ["49997.0","1","_"]]
        }, "book-10", "XBT/USD"]"#;
        assert!(apply_kraken_public_book_message(
            text, &mut bids, &mut asks, 2
        ));
        assert_eq!(asks.len(), 2);
        assert_eq!(bids.len(), 2);
    }

    #[test]
    fn apply_kraken_public_book_message_ignores_non_book_frames() {
        let mut bids = Vec::new();
        let mut asks = Vec::new();
        let text = r#"{"event": "systemStatus", "status": "online"}"#;
        assert!(!apply_kraken_public_book_message(
            text, &mut bids, &mut asks, 10
        ));
    }

    #[test]
    fn snapshot_json_emits_canonical_shape() {
        let json =
            kraken_public_book_snapshot_json("BTCUSD", "XBT/USD", &[(99.0, 1.0)], &[(101.0, 2.0)]);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["source"], "kraken_ws");
        assert_eq!(parsed["symbol"], "BTCUSD");
        assert_eq!(parsed["ws_pair"], "XBT/USD");
        assert_eq!(parsed["bids"][0]["price"], 99.0);
        assert_eq!(parsed["asks"][0]["size"], 2.0);
    }

    #[test]
    fn status_and_error_messages_have_empty_levels() {
        let status = kraken_public_book_status_message("BTCUSD", "XBT/USD", "connecting");
        let parsed: serde_json::Value = serde_json::from_str(&status).unwrap();
        assert_eq!(parsed["status"], "connecting");
        assert_eq!(parsed["bids"].as_array().unwrap().len(), 0);

        let err = kraken_public_book_error_message("BTCUSD", "XBT/USD", "timeout");
        let parsed: serde_json::Value = serde_json::from_str(&err).unwrap();
        assert_eq!(parsed["status"], "error");
        assert_eq!(parsed["error"], "timeout");
    }
}
