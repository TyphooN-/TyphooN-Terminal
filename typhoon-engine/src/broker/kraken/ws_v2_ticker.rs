//! Kraken WebSocket v2 ticker (Level 1) parser, subscription helpers, and stream driver.

use std::time::Duration;

use super::ws_v2::{
    KRAKEN_WS_V2_PUBLIC_URL, KRAKEN_WS_V2_STALE_AFTER, build_ws_v2_subscribe_frame,
    build_ws_v2_unsubscribe_frame, next_ws_v2_req_id, ws_v2_connection_is_stale,
    ws_v2_frame_is_channel, ws_v2_json_f64, ws_v2_timestamp_ms,
};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

pub const KRAKEN_WS_V2_TICKER_CHANNEL: &str = "ticker";

const KRAKEN_WS_TICKER_SUBSCRIBE_BATCH: usize = 250;
const KRAKEN_WS_TICKER_SUBSCRIBE_FRAME_DELAY: Duration = Duration::from_millis(20);
const KRAKEN_WS_TICKER_SUBSCRIBE_TIMEOUT: Duration = Duration::from_secs(120);
const KRAKEN_WS_TICKER_PING_INTERVAL: Duration = Duration::from_secs(30);

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
    /// Last trade side from public trades (for richer live trade indicators / depth tint).
    /// None for regular ticker L1.
    pub last_trade_side: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KrakenTickerStreamerEvent {
    Connected,
    Subscribed { batches: usize },
    SubscribeFailed { reason: String },
    Disconnected { reason: String },
}

pub fn build_ticker_subscribe_frame(symbols: &[String], snapshot: bool) -> String {
    let mut params = serde_json::Map::new();
    params.insert("snapshot".into(), serde_json::Value::Bool(snapshot));
    build_ws_v2_subscribe_frame(KRAKEN_WS_V2_TICKER_CHANNEL, symbols, params)
}

pub fn build_ticker_subscribe_frames(symbols: &[String], snapshot: bool) -> Vec<String> {
    symbols
        .chunks(KRAKEN_WS_TICKER_SUBSCRIBE_BATCH)
        .map(|batch| build_ticker_subscribe_frame(batch, snapshot))
        .collect()
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

pub async fn run_ticker_streamer(
    symbols: Vec<String>,
    ticker_tx: mpsc::Sender<KrakenWsTicker>,
    event_tx: mpsc::UnboundedSender<KrakenTickerStreamerEvent>,
) {
    if symbols.is_empty() || ticker_tx.is_closed() {
        return;
    }
    let mut consecutive_failures: u32 = 0;
    loop {
        if ticker_tx.is_closed() {
            return;
        }
        match run_ticker_streamer_once(&symbols, &ticker_tx, &event_tx).await {
            Ok(()) => consecutive_failures = 0,
            Err(reason) => {
                consecutive_failures = consecutive_failures.saturating_add(1);
                let _ = event_tx.send(KrakenTickerStreamerEvent::Disconnected { reason });
            }
        }
        tokio::time::sleep(compute_ticker_reconnect_backoff(consecutive_failures)).await;
    }
}

async fn run_ticker_streamer_once(
    symbols: &[String],
    ticker_tx: &mpsc::Sender<KrakenWsTicker>,
    event_tx: &mpsc::UnboundedSender<KrakenTickerStreamerEvent>,
) -> Result<(), String> {
    let (ws_stream, _) = connect_async(KRAKEN_WS_V2_PUBLIC_URL)
        .await
        .map_err(|e| format!("ticker ws connect failed: {e}"))?;
    let (mut sink, mut stream) = ws_stream.split();
    let _ = event_tx.send(KrakenTickerStreamerEvent::Connected);

    let frames = build_ticker_subscribe_frames(symbols, true);
    let batches = frames.len();
    let subscribe_fut = async {
        for frame in &frames {
            sink.send(Message::Text(frame.clone().into()))
                .await
                .map_err(|e| format!("ticker ws subscribe send failed: {e}"))?;
            tokio::time::sleep(KRAKEN_WS_TICKER_SUBSCRIBE_FRAME_DELAY).await;
        }
        Ok::<(), String>(())
    };

    match tokio::time::timeout(KRAKEN_WS_TICKER_SUBSCRIBE_TIMEOUT, subscribe_fut).await {
        Ok(Ok(())) => {
            let _ = event_tx.send(KrakenTickerStreamerEvent::Subscribed { batches });
        }
        Ok(Err(reason)) => {
            let _ = event_tx.send(KrakenTickerStreamerEvent::SubscribeFailed {
                reason: reason.clone(),
            });
            return Err(reason);
        }
        Err(_) => {
            let reason = "ticker subscribe burst timed out".to_string();
            let _ = event_tx.send(KrakenTickerStreamerEvent::SubscribeFailed {
                reason: reason.clone(),
            });
            return Err(reason);
        }
    }

    let mut ping_ticker = tokio::time::interval(KRAKEN_WS_TICKER_PING_INTERVAL);
    ping_ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    ping_ticker.tick().await;
    // Half-open watchdog: any received frame (data, heartbeat, ping, pong)
    // refreshes this; a lapse past KRAKEN_WS_V2_STALE_AFTER forces a reconnect.
    let mut last_frame = std::time::Instant::now();

    loop {
        if ticker_tx.is_closed() {
            return Ok(());
        }
        tokio::select! {
            msg = stream.next() => {
                if matches!(msg, Some(Ok(_))) {
                    last_frame = std::time::Instant::now();
                }
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        for ticker in parse_ticker_message(&text) {
                            if ticker_tx.send(ticker).await.is_err() {
                                return Ok(());
                            }
                        }
                    }
                    Some(Ok(Message::Ping(payload))) => {
                        let _ = sink.send(Message::Pong(payload)).await;
                    }
                    Some(Ok(Message::Pong(_))) | Some(Ok(Message::Binary(_)))
                    | Some(Ok(Message::Frame(_))) => {}
                    Some(Ok(Message::Close(_))) => return Err("ticker ws closed by server".into()),
                    Some(Err(e)) => return Err(format!("ticker ws read error: {e}")),
                    None => return Err("ticker ws stream ended".into()),
                }
            }
            _ = ping_ticker.tick() => {
                let ping = serde_json::json!({
                    "method": "ping",
                    "req_id": next_ws_v2_req_id(),
                }).to_string();
                if sink.send(Message::Text(ping.into())).await.is_err() {
                    return Err("ticker ws ping send failed".into());
                }
                if ws_v2_connection_is_stale(last_frame.elapsed(), KRAKEN_WS_V2_STALE_AFTER) {
                    return Err("ticker ws stale: no frame within window; reconnecting".into());
                }
            }
        }
    }
}

fn compute_ticker_reconnect_backoff(consecutive_failures: u32) -> Duration {
    if consecutive_failures == 0 {
        Duration::from_millis(250)
    } else {
        let exp = consecutive_failures.min(6);
        Duration::from_secs(2_u64.saturating_pow(exp))
    }
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
        last_trade_side: None,
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
    fn ticker_subscribe_frames_batch_at_250_symbols() {
        let symbols: Vec<String> = (0..501).map(|i| format!("PAIR{i}/USD")).collect();
        let frames = build_ticker_subscribe_frames(&symbols, true);
        assert_eq!(frames.len(), 3);
        let first: serde_json::Value = serde_json::from_str(&frames[0]).unwrap();
        assert_eq!(first["params"]["symbol"].as_array().unwrap().len(), 250);
        let third: serde_json::Value = serde_json::from_str(&frames[2]).unwrap();
        assert_eq!(third["params"]["symbol"].as_array().unwrap().len(), 1);
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
    fn ticker_reconnect_backoff_is_bounded() {
        assert_eq!(
            compute_ticker_reconnect_backoff(0),
            Duration::from_millis(250)
        );
        assert_eq!(compute_ticker_reconnect_backoff(1), Duration::from_secs(2));
        assert_eq!(compute_ticker_reconnect_backoff(9), Duration::from_secs(64));
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
