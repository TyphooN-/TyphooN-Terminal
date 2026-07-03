//! Kraken WebSocket v2 book (Level 2) parser, state helpers, and stream driver.

use std::time::Duration;

use super::ws_v2::{
    KRAKEN_WS_V2_PUBLIC_URL, KRAKEN_WS_V2_STALE_AFTER, build_ws_v2_subscribe_frame,
    build_ws_v2_unsubscribe_frame, next_ws_v2_req_id, ws_v2_connection_is_stale, ws_v2_json_u64,
    ws_v2_timestamp_ms,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::value::RawValue;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

pub const KRAKEN_WS_V2_BOOK_CHANNEL: &str = "book";

const KRAKEN_WS_BOOK_SUBSCRIBE_BATCH: usize = 250;
const KRAKEN_WS_BOOK_SUBSCRIBE_FRAME_DELAY: Duration = Duration::from_millis(20);
const KRAKEN_WS_BOOK_SUBSCRIBE_TIMEOUT: Duration = Duration::from_secs(120);
const KRAKEN_WS_BOOK_PING_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, PartialEq)]
pub struct KrakenWsBookLevel {
    pub price: f64,
    pub qty: f64,
    pub price_text: String,
    pub qty_text: String,
}

impl KrakenWsBookLevel {
    pub fn new(price: f64, qty: f64) -> Self {
        Self {
            price,
            qty,
            price_text: format_decimal_for_book_level(price),
            qty_text: format_decimal_for_book_level(qty),
        }
    }

    fn from_wire(price: f64, qty: f64, price_text: String, qty_text: String) -> Self {
        Self {
            price,
            qty,
            price_text,
            qty_text,
        }
    }
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KrakenBookStreamerEvent {
    Connected { depth: usize },
    Subscribed { depth: usize, batches: usize },
    SubscribeFailed { depth: usize, reason: String },
    Disconnected { depth: usize, reason: String },
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
        self.apply_delta_unchecked(delta);
    }

    pub fn apply_delta_with_checksum(
        &mut self,
        delta: &KrakenWsBookDelta,
    ) -> Result<Option<u32>, KrakenWsBookChecksumError> {
        let mut next = self.clone();
        next.apply_delta_unchecked(delta);
        let Some(expected) = delta.checksum else {
            *self = next;
            return Ok(None);
        };
        let actual = next.compute_checksum();
        if u64::from(actual) == expected {
            *self = next;
            Ok(Some(actual))
        } else {
            Err(KrakenWsBookChecksumError {
                symbol: delta.symbol.clone(),
                expected,
                actual,
            })
        }
    }

    fn apply_delta_unchecked(&mut self, delta: &KrakenWsBookDelta) {
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

    pub fn compute_checksum(&self) -> u32 {
        // The book levels retain Kraken's exact wire tokens (see `parse_raw_level`),
        // so the raw-text encoding reproduces Kraken's CRC for every pair.
        compute_book_checksum(&self.bids, &self.asks)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KrakenWsBookChecksumError {
    pub symbol: String,
    pub expected: u64,
    pub actual: u32,
}

pub fn build_book_subscribe_frame(symbols: &[String], depth: usize, snapshot: bool) -> String {
    let mut params = serde_json::Map::new();
    params.insert("depth".into(), serde_json::json!(depth));
    params.insert("snapshot".into(), serde_json::Value::Bool(snapshot));
    build_ws_v2_subscribe_frame(KRAKEN_WS_V2_BOOK_CHANNEL, symbols, params)
}

pub fn build_book_subscribe_frames(
    symbols: &[String],
    depth: usize,
    snapshot: bool,
) -> Vec<String> {
    symbols
        .chunks(KRAKEN_WS_BOOK_SUBSCRIBE_BATCH)
        .map(|batch| build_book_subscribe_frame(batch, depth, snapshot))
        .collect()
}

pub fn build_book_unsubscribe_frame(symbols: &[String]) -> String {
    build_ws_v2_unsubscribe_frame(KRAKEN_WS_V2_BOOK_CHANNEL, symbols)
}

/// Book frame with the price/qty leaves left as raw JSON tokens. Kraken
/// serializes book prices/quantities as JSON *numbers* carrying significant
/// trailing zeros (e.g. `"qty":2.296760`), and the v2 checksum is computed over
/// that exact digit string. Routing the value through `serde_json::Value`
/// collapses it to an `f64` and reserializes shortest-round-trip (`2.29676`),
/// dropping the trailing zero and breaking the CRC. Capturing the raw token text
/// (serde_json `raw_value`) preserves Kraken's exact serialization.
#[derive(serde::Deserialize)]
struct RawBookFrame<'a> {
    #[serde(default)]
    channel: Option<String>,
    #[serde(rename = "type", default)]
    msg_type: Option<String>,
    #[serde(borrow, default)]
    data: Vec<&'a RawValue>,
}

#[derive(serde::Deserialize)]
struct RawBookEntry<'a> {
    symbol: String,
    #[serde(borrow, default)]
    asks: Vec<&'a RawValue>,
    #[serde(borrow, default)]
    bids: Vec<&'a RawValue>,
    #[serde(default)]
    checksum: Option<serde_json::Value>,
    #[serde(default)]
    timestamp: Option<serde_json::Value>,
    #[serde(default)]
    time: Option<serde_json::Value>,
}

#[derive(serde::Deserialize)]
struct RawBookLevelObj<'a> {
    #[serde(borrow)]
    price: &'a RawValue,
    #[serde(borrow, default)]
    qty: Option<&'a RawValue>,
    #[serde(borrow, default)]
    quantity: Option<&'a RawValue>,
}

pub fn parse_book_message(text: &str) -> Vec<KrakenWsBookDelta> {
    let Ok(frame) = serde_json::from_str::<RawBookFrame>(text) else {
        return Vec::new();
    };
    if frame.channel.as_deref() != Some(KRAKEN_WS_V2_BOOK_CHANNEL) {
        return Vec::new();
    }
    let is_snapshot = frame.msg_type.as_deref() == Some("snapshot");
    frame
        .data
        .iter()
        .filter_map(|entry| parse_book_entry(entry, is_snapshot))
        .collect()
}

pub async fn run_book_streamer(
    symbols: Vec<String>,
    depth: usize,
    book_tx: mpsc::Sender<KrakenWsBookDelta>,
    event_tx: mpsc::UnboundedSender<KrakenBookStreamerEvent>,
) {
    if symbols.is_empty() || depth == 0 || book_tx.is_closed() {
        return;
    }
    let mut consecutive_failures: u32 = 0;
    loop {
        if book_tx.is_closed() {
            return;
        }
        match run_book_streamer_once(&symbols, depth, &book_tx, &event_tx).await {
            Ok(()) => consecutive_failures = 0,
            Err(reason) => {
                consecutive_failures = consecutive_failures.saturating_add(1);
                let _ = event_tx.send(KrakenBookStreamerEvent::Disconnected { depth, reason });
            }
        }
        tokio::time::sleep(compute_book_reconnect_backoff(consecutive_failures)).await;
    }
}

async fn run_book_streamer_once(
    symbols: &[String],
    depth: usize,
    book_tx: &mpsc::Sender<KrakenWsBookDelta>,
    event_tx: &mpsc::UnboundedSender<KrakenBookStreamerEvent>,
) -> Result<(), String> {
    let (ws_stream, _) = connect_async(KRAKEN_WS_V2_PUBLIC_URL)
        .await
        .map_err(|e| format!("book ws connect failed: {e}"))?;
    let (mut sink, mut stream) = ws_stream.split();
    let _ = event_tx.send(KrakenBookStreamerEvent::Connected { depth });

    let frames = build_book_subscribe_frames(symbols, depth, true);
    let batches = frames.len();
    let subscribe_fut = async {
        for frame in &frames {
            sink.send(Message::Text(frame.clone().into()))
                .await
                .map_err(|e| format!("book ws subscribe send failed: {e}"))?;
            tokio::time::sleep(KRAKEN_WS_BOOK_SUBSCRIBE_FRAME_DELAY).await;
        }
        Ok::<(), String>(())
    };

    match tokio::time::timeout(KRAKEN_WS_BOOK_SUBSCRIBE_TIMEOUT, subscribe_fut).await {
        Ok(Ok(())) => {
            let _ = event_tx.send(KrakenBookStreamerEvent::Subscribed { depth, batches });
        }
        Ok(Err(reason)) => {
            let _ = event_tx.send(KrakenBookStreamerEvent::SubscribeFailed {
                depth,
                reason: reason.clone(),
            });
            return Err(reason);
        }
        Err(_) => {
            let reason = "book subscribe burst timed out".to_string();
            let _ = event_tx.send(KrakenBookStreamerEvent::SubscribeFailed {
                depth,
                reason: reason.clone(),
            });
            return Err(reason);
        }
    }

    let mut ping_ticker = tokio::time::interval(KRAKEN_WS_BOOK_PING_INTERVAL);
    ping_ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    ping_ticker.tick().await;
    // Half-open watchdog: any received frame (data, heartbeat, ping, pong)
    // refreshes this; a lapse past KRAKEN_WS_V2_STALE_AFTER forces a reconnect.
    let mut last_frame = std::time::Instant::now();

    loop {
        if book_tx.is_closed() {
            return Ok(());
        }
        tokio::select! {
            msg = stream.next() => {
                if matches!(msg, Some(Ok(_))) {
                    last_frame = std::time::Instant::now();
                }
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        for delta in parse_book_message(&text) {
                            if book_tx.send(delta).await.is_err() {
                                return Ok(());
                            }
                        }
                    }
                    Some(Ok(Message::Ping(payload))) => {
                        let _ = sink.send(Message::Pong(payload)).await;
                    }
                    Some(Ok(Message::Pong(_))) | Some(Ok(Message::Binary(_)))
                    | Some(Ok(Message::Frame(_))) => {}
                    Some(Ok(Message::Close(_))) => return Err("book ws closed by server".into()),
                    Some(Err(e)) => return Err(format!("book ws read error: {e}")),
                    None => return Err("book ws stream ended".into()),
                }
            }
            _ = ping_ticker.tick() => {
                let ping = serde_json::json!({
                    "method": "ping",
                    "req_id": next_ws_v2_req_id(),
                }).to_string();
                if sink.send(Message::Text(ping.into())).await.is_err() {
                    return Err("book ws ping send failed".into());
                }
                if ws_v2_connection_is_stale(last_frame.elapsed(), KRAKEN_WS_V2_STALE_AFTER) {
                    return Err("book ws stale: no frame within window; reconnecting".into());
                }
            }
        }
    }
}

fn compute_book_reconnect_backoff(consecutive_failures: u32) -> Duration {
    if consecutive_failures == 0 {
        Duration::from_millis(250)
    } else {
        let exp = consecutive_failures.min(6);
        Duration::from_secs(2_u64.saturating_pow(exp))
    }
}

fn parse_book_entry(entry: &RawValue, is_snapshot: bool) -> Option<KrakenWsBookDelta> {
    let parsed: RawBookEntry = serde_json::from_str(entry.get()).ok()?;
    let asks = parsed
        .asks
        .iter()
        .filter_map(|level| parse_raw_level(level))
        .collect();
    let bids = parsed
        .bids
        .iter()
        .filter_map(|level| parse_raw_level(level))
        .collect();
    Some(KrakenWsBookDelta {
        symbol: parsed.symbol,
        bids,
        asks,
        checksum: parsed.checksum.as_ref().and_then(ws_v2_json_u64),
        ts_ms: parsed
            .timestamp
            .as_ref()
            .or(parsed.time.as_ref())
            .and_then(ws_v2_timestamp_ms),
        is_snapshot,
    })
}

/// Parse one book level — object `{"price":..,"qty":..}` or array `[price, qty]`
/// — keeping each price/qty as the exact wire token so the checksum reproduces
/// Kraken's digit string (trailing zeros and all).
fn parse_raw_level(level: &RawValue) -> Option<KrakenWsBookLevel> {
    let body = level.get();
    if body.trim_start().starts_with('{') {
        let obj: RawBookLevelObj = serde_json::from_str(body).ok()?;
        let qty_raw = obj.qty.or(obj.quantity)?;
        let (price, price_text) = raw_decimal_scalar(obj.price)?;
        let (qty, qty_text) = raw_decimal_scalar(qty_raw)?;
        Some(KrakenWsBookLevel::from_wire(price, qty, price_text, qty_text))
    } else {
        let arr: Vec<&RawValue> = serde_json::from_str(body).ok()?;
        let (price, price_text) = raw_decimal_scalar(arr.first()?)?;
        let (qty, qty_text) = raw_decimal_scalar(arr.get(1)?)?;
        Some(KrakenWsBookLevel::from_wire(price, qty, price_text, qty_text))
    }
}

/// Decode a numeric scalar token to `(f64, exact_text)`. A JSON number keeps its
/// literal text (trailing zeros intact — the whole point); a JSON string yields
/// its inner content, matching the prior `as_str()` behavior for string-encoded
/// levels. Non-finite values are rejected, as before.
fn raw_decimal_scalar(value: &RawValue) -> Option<(f64, String)> {
    let raw = value.get().trim();
    let text = if raw.starts_with('"') {
        serde_json::from_str::<String>(raw).ok()?
    } else {
        raw.to_string()
    };
    let parsed = text.trim().parse::<f64>().ok().filter(|v| v.is_finite())?;
    Some((parsed, text))
}

pub fn compute_book_checksum(bids: &[KrakenWsBookLevel], asks: &[KrakenWsBookLevel]) -> u32 {
    let mut payload = String::new();
    for level in asks.iter().take(10) {
        push_checksum_level(&mut payload, level);
    }
    for level in bids.iter().take(10) {
        push_checksum_level(&mut payload, level);
    }
    crc32fast::hash(payload.as_bytes())
}

fn push_checksum_level(payload: &mut String, level: &KrakenWsBookLevel) {
    super::helpers::push_book_checksum_component(payload, &level.price_text);
    super::helpers::push_book_checksum_component(payload, &level.qty_text);
}

fn format_decimal_for_book_level(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.1}")
    } else {
        value.to_string()
    }
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
                side[existing_idx] = level.clone();
            }
        } else if level.qty > 0.0 {
            side.push(level.clone());
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
    fn book_subscribe_frames_batch_at_250_symbols() {
        let symbols: Vec<String> = (0..501).map(|i| format!("PAIR{i}/USD")).collect();
        let frames = build_book_subscribe_frames(&symbols, 25, true);
        assert_eq!(frames.len(), 3);
        let first: serde_json::Value = serde_json::from_str(&frames[0]).unwrap();
        assert_eq!(first["params"]["symbol"].as_array().unwrap().len(), 250);
        assert_eq!(first["params"]["depth"], 25);
        let third: serde_json::Value = serde_json::from_str(&frames[2]).unwrap();
        assert_eq!(third["params"]["symbol"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn book_reconnect_backoff_is_bounded() {
        assert_eq!(
            compute_book_reconnect_backoff(0),
            Duration::from_millis(250)
        );
        assert_eq!(compute_book_reconnect_backoff(1), Duration::from_secs(2));
        assert_eq!(compute_book_reconnect_backoff(9), Duration::from_secs(64));
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
    fn checksum_decimal_component_preserves_wire_precision() {
        let component = |raw: &str| {
            let mut payload = String::new();
            super::super::helpers::push_book_checksum_component(&mut payload, raw);
            payload
        };
        assert_eq!(component("0.05000000"), "5000000");
        assert_eq!(component("67100.0"), "671000");
        assert_eq!(component("+001.2300"), "12300");
        assert_eq!(component("0.0"), "0");
        assert_eq!(component("1e-7"), "1");
        assert_eq!(component("2.5e3"), "25000");
    }

    #[test]
    fn book_checksum_uses_asks_then_bids_top_ten_payload() {
        let asks = vec![
            KrakenWsBookLevel::from_wire(100.5, 0.05000000, "100.5000".into(), "0.05000000".into()),
            KrakenWsBookLevel::from_wire(101.0, 1.0, "101.0".into(), "1.00000000".into()),
        ];
        let bids = vec![KrakenWsBookLevel::from_wire(
            100.0,
            2.5,
            "100.0".into(),
            "2.50000000".into(),
        )];
        let expected_payload = "1005000500000010101000000001000250000000";
        assert_eq!(
            compute_book_checksum(&bids, &asks),
            crc32fast::hash(expected_payload.as_bytes())
        );
    }

    #[test]
    fn book_state_validates_matching_checksum_and_reports_mismatch() {
        let mut state = KrakenWsBookState::new("BTC/USD", 10);
        let mut delta = KrakenWsBookDelta {
            symbol: "BTC/USD".into(),
            bids: vec![KrakenWsBookLevel::from_wire(
                100.0,
                2.5,
                "100.0".into(),
                "2.50000000".into(),
            )],
            asks: vec![KrakenWsBookLevel::from_wire(
                100.5,
                0.05,
                "100.5000".into(),
                "0.05000000".into(),
            )],
            checksum: None,
            ts_ms: None,
            is_snapshot: true,
        };
        let expected = compute_book_checksum(&delta.bids, &delta.asks);
        delta.checksum = Some(u64::from(expected));
        assert_eq!(state.apply_delta_with_checksum(&delta), Ok(Some(expected)));

        let mut bad = delta.clone();
        bad.checksum = Some(u64::from(expected).saturating_add(1));
        let err = state.apply_delta_with_checksum(&bad).unwrap_err();
        assert_eq!(err.symbol, "BTC/USD");
        assert_eq!(err.expected, u64::from(expected).saturating_add(1));
        assert_eq!(err.actual, expected);
        assert_eq!(state.last_checksum, Some(u64::from(expected)));
        assert_eq!(state.bids[0].price_text, "100.0");
        assert_eq!(state.asks[0].price_text, "100.5000");
    }

    #[test]
    fn book_state_applies_snapshot_then_deltas() {
        let mut state = KrakenWsBookState::new("BTC/USD", 2);
        state.apply_delta(&KrakenWsBookDelta {
            symbol: "BTC/USD".into(),
            bids: vec![
                KrakenWsBookLevel::new(100.0, 1.0),
                KrakenWsBookLevel::new(99.0, 1.0),
                KrakenWsBookLevel::new(98.0, 1.0),
            ],
            asks: vec![KrakenWsBookLevel::new(101.0, 1.0)],
            checksum: Some(1),
            ts_ms: None,
            is_snapshot: true,
        });
        assert_eq!(state.bids.len(), 2);
        assert_eq!(state.bids[0].price, 100.0);

        state.apply_delta(&KrakenWsBookDelta {
            symbol: "BTC/USD".into(),
            bids: vec![KrakenWsBookLevel::new(100.0, 0.0)],
            asks: vec![KrakenWsBookLevel::new(100.5, 2.0)],
            checksum: Some(2),
            ts_ms: None,
            is_snapshot: false,
        });
        assert_eq!(state.bids[0].price, 99.0);
        assert_eq!(state.asks[0].price, 100.5);
        assert_eq!(state.last_checksum, Some(2));
    }

    #[test]
    fn live_metax_snapshot_checksum_validates_with_raw_tokens() {
        // Real Kraken WS v2 book snapshot for METAx/USD captured 2026-06-30T11:15:58Z.
        // Kraken sends price/qty as JSON numbers with significant trailing zeros
        // (e.g. "qty":0.120000, "qty":2.296760); its checksum (1687683704) only
        // reproduces if those trailing zeros survive parsing. Collapsing the token
        // through an f64 would yield 0.12 / 2.29676 and a different CRC — the exact
        // xStock book-checksum loop this parser change fixes. Pins to live bytes.
        let raw = r#"{"channel":"book","type":"snapshot","data":[{"symbol":"METAx/USD","bids":[{"price":564.01374,"qty":0.120000},{"price":564.00376,"qty":2.296760},{"price":563.62464,"qty":8.836362},{"price":562.20789,"qty":17.672824},{"price":560.00296,"qty":0.060714},{"price":552.71970,"qty":0.087594},{"price":552.62990,"qty":0.087594},{"price":552.59997,"qty":0.569359},{"price":552.48025,"qty":3.591344},{"price":552.23082,"qty":0.043797}],"asks":[{"price":575.78668,"qty":1.425265},{"price":575.83657,"qty":0.164454},{"price":576.29551,"qty":3.289073},{"price":576.33542,"qty":5.481788},{"price":576.35538,"qty":0.548179},{"price":576.78439,"qty":0.164454},{"price":576.96398,"qty":3.014983},{"price":577.13359,"qty":0.548179},{"price":577.20343,"qty":5.481788},{"price":577.32315,"qty":7.619685}],"checksum":1687683704,"timestamp":"2026-06-30T11:15:58.397403Z"}]}"#;
        let deltas = parse_book_message(raw);
        assert_eq!(deltas.len(), 1);
        let delta = &deltas[0];
        assert_eq!(delta.symbol, "METAx/USD");
        assert!(delta.is_snapshot);
        assert_eq!(delta.bids.len(), 10);
        assert_eq!(delta.asks.len(), 10);
        assert_eq!(delta.checksum, Some(1_687_683_704));
        // Trailing zeros must be preserved verbatim in the checksum text.
        assert_eq!(delta.bids[0].qty_text, "0.120000");
        assert_eq!(delta.bids[1].qty_text, "2.296760");
        // Locally-computed CRC must match Kraken's embedded server checksum.
        let mut state = KrakenWsBookState::new("METAx/USD", 10);
        let validated = state
            .apply_delta_with_checksum(delta)
            .expect("computed checksum must match Kraken's server checksum");
        assert_eq!(validated, Some(1_687_683_704));
    }
}
