//! Kraken WebSocket v2 Level 3 (per-order / market-by-order) parser and basic streamer.
//!
//! L3 requires authenticated connection (token) and entitlements.
//! See ADR-109 and ADR-129.
//! This provides the wiring foundation. Real auth + full delta apply with checksums
//! can be added mirroring ws_v2_book + private_ws when keys are available.

use std::time::Duration;

use super::ws_v2::{KRAKEN_WS_V2_LEVEL3_URL, build_ws_v2_subscribe_frame, next_ws_v2_req_id};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

#[derive(Debug, Clone, PartialEq)]
pub struct KrakenL3Level {
    pub order_id: String,
    pub limit_price: f64,
    pub order_qty: f64,
    pub timestamp: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KrakenL3Delta {
    pub symbol: String,
    pub bids: Vec<KrakenL3Level>,
    pub asks: Vec<KrakenL3Level>,
    pub checksum: Option<u64>,
    pub is_snapshot: bool,
}

/// Basic run for L3 streamer.
/// For real use with entitlements:
/// - Connect to KRAKEN_WS_V2_LEVEL3_URL (or auth variant)
/// - Obtain token via get_websockets_token
/// - Subscribe with "token" in params
/// - Parse snapshot/update for per-order data (order_id, limit_price, order_qty)
/// - Emit deltas for downstream (charts depth bins, Bookmap per-order, DOM)
pub async fn run_level3_streamer(
    symbols: Vec<String>,
    l3_tx: mpsc::Sender<KrakenL3Delta>,
    event_tx: mpsc::UnboundedSender<String>,
) {
    if symbols.is_empty() || l3_tx.is_closed() {
        return;
    }

    let mut consecutive_failures: u32 = 0;
    loop {
        if l3_tx.is_closed() {
            return;
        }
        match run_level3_streamer_once(&symbols, &l3_tx, &event_tx).await {
            Ok(()) => consecutive_failures = 0,
            Err(reason) => {
                consecutive_failures = consecutive_failures.saturating_add(1);
                let _ = event_tx.send(format!("L3 disconnected: {reason}"));
            }
        }
        // Backoff
        let backoff = if consecutive_failures == 0 {
            Duration::from_millis(250)
        } else {
            Duration::from_secs(2u64.saturating_pow(consecutive_failures.min(6)))
        };
        tokio::time::sleep(backoff).await;
    }
}

async fn run_level3_streamer_once(
    symbols: &[String],
    l3_tx: &mpsc::Sender<KrakenL3Delta>,
    event_tx: &mpsc::UnboundedSender<String>,
) -> Result<(), String> {
    // Real L3 is authenticated. For wiring/demo we connect to LEVEL3_URL.
    // When entitled: pass token in subscribe params.
    let (ws_stream, _) = connect_async(KRAKEN_WS_V2_LEVEL3_URL)
        .await
        .map_err(|e| format!("L3 ws connect failed: {e}"))?;
    let (mut sink, mut stream) = ws_stream.split();

    let _ = event_tx.send("L3 connected (stub - token required for real)".into());

    // Subscribe (token omitted here; real path adds "token": <ws_token>)
    let subscribe_frame = build_ws_v2_subscribe_frame(
        "level3",
        symbols,
        {
            let mut p = serde_json::Map::new();
            p.insert("snapshot".to_string(), serde_json::Value::Bool(true));
            // p.insert("token".to_string(), serde_json::Value::String(token));
            p
        },
    );
    sink.send(Message::Text(subscribe_frame.into()))
        .await
        .map_err(|e| format!("L3 subscribe send failed: {e}"))?;

    let _ = event_tx.send(format!("L3 subscribed for {:?}", symbols));

    // Demo: emit simulated L3 data to exercise parse, depth binning, Bookmap per-order.
    // In real: loop on stream, parse, send real deltas.
    let mut tick = 0u64;
    loop {
        if l3_tx.is_closed() {
            return Ok(());
        }

        // Simulate L3 snapshot/update with per-order data (for demo when no entitlements)
        let sim = simulate_l3_delta(symbols.get(0).cloned().unwrap_or("DEMO/USD".into()), tick);
        if l3_tx.send(sim).await.is_err() {
            return Ok(());
        }

        // In real impl, read from stream here like book:
        // match stream.next().await { Some(Ok(Message::Text(text))) => { for delta in parse_l3_message(&text) { ... } } ... }

        tick += 1;
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

fn simulate_l3_delta(symbol: String, tick: u64) -> KrakenL3Delta {
    // Simple varying per-order data to demo
    let base = 100.0 + (tick % 5) as f64 * 0.1;
    KrakenL3Delta {
        symbol,
        bids: vec![
            KrakenL3Level { order_id: format!("B{}", tick), limit_price: base - 0.05, order_qty: 1.2 + (tick % 3) as f64 * 0.1, timestamp: Some("now".into()) },
            KrakenL3Level { order_id: format!("B2{}", tick), limit_price: base - 0.1, order_qty: 0.8, timestamp: Some("now".into()) },
        ],
        asks: vec![
            KrakenL3Level { order_id: format!("A{}", tick), limit_price: base + 0.05, order_qty: 2.5, timestamp: Some("now".into()) },
        ],
        checksum: Some(123456 + tick),
        is_snapshot: tick % 5 == 0,
    }
}

/// Parse L3 message (skeleton using known format).
/// Extend with full delta handling for add/mod/del per order_id when real stream is active.
pub fn parse_l3_message(text: &str) -> Vec<KrakenL3Delta> {
    if let Ok(v) = serde_json::from_str::<Value>(text) {
        if let Some(data) = v.get("data").and_then(|d| d.as_array()) {
            let mut out = vec![];
            for item in data {
                let sym = item.get("symbol").and_then(|s| s.as_str()).unwrap_or("?").to_string();
                let checksum = item.get("checksum").and_then(|c| c.as_u64());
                let bids = parse_l3_side(item.get("bids"));
                let asks = parse_l3_side(item.get("asks"));
                out.push(KrakenL3Delta {
                    symbol: sym,
                    bids,
                    asks,
                    checksum,
                    is_snapshot: v.get("type").map(|t| t == "snapshot").unwrap_or(true),
                });
            }
            return out;
        }
    }
    vec![]
}

fn parse_l3_side(side: Option<&Value>) -> Vec<KrakenL3Level> {
    let mut res = vec![];
    if let Some(arr) = side.and_then(|s| s.as_array()) {
        for l in arr.iter().take(25) {  // more levels for profile bins
            let oid = l.get("order_id").and_then(|o| o.as_str()).unwrap_or("").to_string();
            let price = l.get("limit_price").and_then(|p| p.as_f64()).unwrap_or(0.0);
            let qty = l.get("order_qty").and_then(|q| q.as_f64()).unwrap_or(0.0);
            if price > 0.0 && qty > 0.0 {
                res.push(KrakenL3Level {
                    order_id: oid,
                    limit_price: price,
                    order_qty: qty,
                    timestamp: l.get("timestamp").and_then(|t| t.as_str()).map(|s| s.to_string()),
                });
            }
        }
    }
    res
}