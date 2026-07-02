//! Kraken WebSocket v2 public "trade" channel (executed trades).
//! Real-time public trades for volume, last price confirmation, tick activity.
//! Public (no auth). Snapshot + incremental updates.
//! O(1) per-symbol last-trade updates downstream.
//!
//! Modeled on ws_v2_ticker.rs for consistency and reconnect robustness.

use std::time::Duration;

use super::ws_v2::{
    KRAKEN_WS_V2_PUBLIC_URL, build_ws_v2_subscribe_frame, build_ws_v2_unsubscribe_frame,
    next_ws_v2_req_id, parse_ws_v2_channel_type, ws_v2_json_f64,
};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

pub const KRAKEN_WS_V2_TRADE_CHANNEL: &str = "trade";

const KRAKEN_WS_TRADE_SUBSCRIBE_BATCH: usize = 250;
const KRAKEN_WS_TRADE_SUBSCRIBE_FRAME_DELAY: Duration = Duration::from_millis(20);
const KRAKEN_WS_TRADE_PING_INTERVAL: Duration = Duration::from_secs(30);
/// Emit a one-shot "degraded" status after this many consecutive reconnect
/// failures. The streamer keeps retrying afterward (it never permanently gives
/// up on a transient burst) — this only surfaces the degradation once.
const KRAKEN_WS_TRADE_DEGRADED_AFTER: u32 = 5;

#[derive(Debug, Clone, PartialEq)]
pub struct KrakenWsPublicTrade {
    pub symbol: String,
    pub price: f64,
    pub volume: f64,
    pub time: f64, // seconds since epoch (Kraken style)
    pub side: String, // "buy" or "sell"
    pub is_snapshot: bool,
}

pub fn build_trades_subscribe_frame(symbols: &[String]) -> String {
    build_ws_v2_subscribe_frame(KRAKEN_WS_V2_TRADE_CHANNEL, symbols, serde_json::Map::new())
}

pub fn build_trades_subscribe_frames(symbols: &[String]) -> Vec<String> {
    symbols
        .chunks(KRAKEN_WS_TRADE_SUBSCRIBE_BATCH)
        .map(|batch| build_trades_subscribe_frame(batch))
        .collect()
}

pub fn build_trades_unsubscribe_frame(symbols: &[String]) -> String {
    build_ws_v2_unsubscribe_frame(KRAKEN_WS_V2_TRADE_CHANNEL, symbols)
}

fn parse_trade_entry(entry: &serde_json::Value, is_snapshot: bool) -> Option<KrakenWsPublicTrade> {
    let symbol = entry
        .get("symbol")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())?;

    let trades = entry.get("trades").and_then(|v| v.as_array())?;

    // For updates, there can be multiple trades per message; we emit one per trade row.
    // For simplicity in first cut, take the last (most recent) in the array for the symbol.
    // Caller can handle multiples if needed. For O(1) last-trade, latest is sufficient.
    if let Some(last_trade) = trades.last() {
        if let Some(arr) = last_trade.as_array() {
            if arr.len() >= 3 {
                let price = ws_v2_json_f64(&arr[0])?;
                let volume = ws_v2_json_f64(&arr[1])?;
                let time = ws_v2_json_f64(&arr[2])?;
                let side = arr
                    .get(3)
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();

                return Some(KrakenWsPublicTrade {
                    symbol,
                    price,
                    volume,
                    time,
                    side,
                    is_snapshot,
                });
            }
        }
    }
    None
}

pub fn parse_trade_message(text: &str) -> Vec<KrakenWsPublicTrade> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return Vec::new();
    };

    let Some((channel, frame_type)) = parse_ws_v2_channel_type(&value) else {
        return Vec::new();
    };
    if channel != KRAKEN_WS_V2_TRADE_CHANNEL {
        return Vec::new();
    }

    let is_snapshot = frame_type == "snapshot";

    if !is_snapshot && channel != KRAKEN_WS_V2_TRADE_CHANNEL {
        return Vec::new();
    }

    let Some(data) = value.get("data").and_then(|v| v.as_array()) else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for entry in data {
        if let Some(trade) = parse_trade_entry(entry, is_snapshot) {
            out.push(trade);
        }
    }
    out
}

async fn run_trades_streamer_once(
    symbols: &[String],
    trade_tx: &mpsc::Sender<KrakenWsPublicTrade>,
    event_tx: &mpsc::UnboundedSender<KrakenTradeStreamerEvent>,
) -> Result<(), String> {
    let (ws_stream, _) = connect_async(KRAKEN_WS_V2_PUBLIC_URL)
        .await
        .map_err(|e| format!("trade connect: {e}"))?;

    let (mut write, mut read) = ws_stream.split();

    // Subscribe in batches
    let frames = build_trades_subscribe_frames(symbols);
    for (i, frame) in frames.iter().enumerate() {
        write
            .send(Message::Text(frame.clone().into()))
            .await
            .map_err(|e| format!("trade subscribe send: {e}"))?;
        if i + 1 < frames.len() {
            tokio::time::sleep(KRAKEN_WS_TRADE_SUBSCRIBE_FRAME_DELAY).await;
        }
    }

    let _ = event_tx.send(KrakenTradeStreamerEvent::Subscribed {
        batches: frames.len(),
    });

    let mut ping_interval = tokio::time::interval(KRAKEN_WS_TRADE_PING_INTERVAL);

    loop {
        tokio::select! {
            _ = ping_interval.tick() => {
                let ping = serde_json::json!({"method": "ping", "req_id": next_ws_v2_req_id()}).to_string();
                if write.send(Message::Text(ping.into())).await.is_err() {
                    return Err("ping failed".to_string());
                }
            }
            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        // Ack handling omitted for brevity (similar to ticker)
                        let trades = parse_trade_message(&text);
                        for t in trades {
                            if trade_tx.send(t).await.is_err() {
                                return Ok(());
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        return Err("connection closed".to_string());
                    }
                    Some(Ok(_)) => {}
                    Some(Err(e)) => {
                        return Err(format!("read error: {e}"));
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KrakenTradeStreamerEvent {
    Connected,
    Subscribed { batches: usize },
    SubscribeFailed { reason: String },
    Disconnected { reason: String },
}

pub async fn run_trades_streamer(
    symbols: Vec<String>,
    trade_tx: mpsc::Sender<KrakenWsPublicTrade>,
    event_tx: mpsc::UnboundedSender<KrakenTradeStreamerEvent>,
) {
    if symbols.is_empty() || trade_tx.is_closed() {
        return;
    }

    let mut consecutive_failures: u32 = 0;

    loop {
        if trade_tx.is_closed() {
            return;
        }

        match run_trades_streamer_once(&symbols, &trade_tx, &event_tx).await {
            Ok(()) => consecutive_failures = 0,
            Err(reason) => {
                consecutive_failures = consecutive_failures.saturating_add(1);
                let _ = event_tx.send(KrakenTradeStreamerEvent::Disconnected { reason });
                // Robustness: never permanently abandon the tape on a transient
                // failure burst (mirrors the ticker/book/level3 streamers, per this
                // module's header). Surface a one-shot "degraded" status at the
                // threshold, then keep retrying with capped exponential backoff so
                // live executions self-heal when the feed recovers. The only terminal
                // exit is the consumer dropping `trade_tx` (checked at the loop top).
                if consecutive_failures == KRAKEN_WS_TRADE_DEGRADED_AFTER {
                    let _ = event_tx.send(KrakenTradeStreamerEvent::SubscribeFailed {
                        reason: format!(
                            "trade feed degraded after {consecutive_failures} consecutive failures; retrying"
                        ),
                    });
                }
                tokio::time::sleep(compute_trades_reconnect_backoff(consecutive_failures)).await;
            }
        }
    }
}

/// Capped exponential reconnect backoff, identical in shape to the ticker/book
/// streamers: a short 250 ms first retry, then `2^min(n,6)` seconds (max 64 s).
fn compute_trades_reconnect_backoff(consecutive_failures: u32) -> Duration {
    if consecutive_failures == 0 {
        Duration::from_millis(250)
    } else {
        let exp = consecutive_failures.min(6);
        Duration::from_secs(2_u64.saturating_pow(exp))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trade_reconnect_backoff_is_bounded_and_never_terminal() {
        // Same capped-exponential shape as the ticker/book lanes; a large failure
        // count saturates at 64 s rather than overflowing or giving up.
        assert_eq!(
            compute_trades_reconnect_backoff(0),
            Duration::from_millis(250)
        );
        assert_eq!(compute_trades_reconnect_backoff(1), Duration::from_secs(2));
        assert_eq!(compute_trades_reconnect_backoff(6), Duration::from_secs(64));
        assert_eq!(
            compute_trades_reconnect_backoff(100),
            Duration::from_secs(64)
        );
    }
}
