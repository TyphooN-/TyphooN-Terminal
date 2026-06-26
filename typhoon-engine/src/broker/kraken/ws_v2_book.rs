//! Kraken WebSocket v2 book (Level 2) parser, state helpers, and stream driver.

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};
use std::time::Duration;

use super::ws_v2::{
    KRAKEN_WS_V2_PUBLIC_URL, build_ws_v2_subscribe_frame, build_ws_v2_unsubscribe_frame,
    next_ws_v2_req_id, ws_v2_frame_is_channel, ws_v2_json_f64, ws_v2_json_u64, ws_v2_timestamp_ms,
};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

pub const KRAKEN_WS_V2_BOOK_CHANNEL: &str = "book";

const KRAKEN_WS_BOOK_SUBSCRIBE_BATCH: usize = 250;
const KRAKEN_WS_BOOK_SUBSCRIBE_FRAME_DELAY: Duration = Duration::from_millis(20);
const KRAKEN_WS_BOOK_SUBSCRIBE_TIMEOUT: Duration = Duration::from_secs(120);
const KRAKEN_WS_BOOK_PING_INTERVAL: Duration = Duration::from_secs(30);

/// Per-pair `(price_decimals, qty_decimals)` precision (Kraken AssetPairs
/// `pair_decimals` / `lot_decimals`), keyed by the exact WS v2 pair name
/// (e.g. `AAPLx/USD`). The v2 book checksum must format each level to the pair's
/// *fixed* precision before stripping the decimal point and leading zeros; the
/// raw wire text drops trailing zeros for round-numbered levels (notably the
/// tokenized xStocks), producing a deterministic checksum mismatch and an endless
/// resubscribe loop. Pairs absent here fall back to the wire-text formatting
/// (unchanged behavior — zero regression for pairs whose wire text is padded).
fn pair_book_precision_registry() -> &'static RwLock<HashMap<String, (u8, u8)>> {
    static REG: OnceLock<RwLock<HashMap<String, (u8, u8)>>> = OnceLock::new();
    REG.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Register a pair's `(price_decimals, qty_decimals)` for WS v2 book checksum
/// formatting, keyed by the WS pair name. Idempotent; call after loading the
/// Kraken AssetPairs metadata.
pub fn register_kraken_pair_book_precision(ws_pair: &str, price_decimals: u8, qty_decimals: u8) {
    if let Ok(mut reg) = pair_book_precision_registry().write() {
        reg.insert(ws_pair.to_string(), (price_decimals, qty_decimals));
    }
}

fn lookup_pair_book_precision(ws_pair: &str) -> Option<(u8, u8)> {
    pair_book_precision_registry()
        .read()
        .ok()
        .and_then(|reg| reg.get(ws_pair).copied())
}

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
        // Prefer the Kraken-correct fixed-precision encoding when this pair's
        // decimals are known; otherwise fall back to the raw wire-text encoding.
        match lookup_pair_book_precision(&self.symbol) {
            Some((price_decimals, qty_decimals)) => compute_book_checksum_with_precision(
                &self.bids,
                &self.asks,
                price_decimals,
                qty_decimals,
            ),
            None => compute_book_checksum(&self.bids, &self.asks),
        }
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

    loop {
        if book_tx.is_closed() {
            return Ok(());
        }
        tokio::select! {
            msg = stream.next() => {
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
        let price_value = obj.get("price")?;
        let qty_value = obj.get("qty").or_else(|| obj.get("quantity"))?;
        return Some(KrakenWsBookLevel::from_wire(
            ws_v2_json_f64(price_value)?,
            ws_v2_json_f64(qty_value)?,
            json_decimal_text(price_value)?,
            json_decimal_text(qty_value)?,
        ));
    }
    let arr = level.as_array()?;
    let price_value = arr.first()?;
    let qty_value = arr.get(1)?;
    Some(KrakenWsBookLevel::from_wire(
        ws_v2_json_f64(price_value)?,
        ws_v2_json_f64(qty_value)?,
        json_decimal_text(price_value)?,
        json_decimal_text(qty_value)?,
    ))
}

fn json_decimal_text(value: &serde_json::Value) -> Option<String> {
    if let Some(text) = value.as_str() {
        return Some(text.to_string());
    }
    if value.is_number() {
        return Some(value.to_string());
    }
    None
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

/// Checksum variant that formats each level to the pair's fixed price/qty
/// precision — the Kraken-correct algorithm — used when the precision is known.
/// Reconstructs the trailing zeros the wire text omits, so round-numbered
/// tokenized-xStock books match Kraken's CRC instead of looping on a mismatch.
pub fn compute_book_checksum_with_precision(
    bids: &[KrakenWsBookLevel],
    asks: &[KrakenWsBookLevel],
    price_decimals: u8,
    qty_decimals: u8,
) -> u32 {
    let mut payload = String::new();
    for level in asks.iter().take(10) {
        payload.push_str(&checksum_fixed_precision_component(
            level.price,
            price_decimals,
        ));
        payload.push_str(&checksum_fixed_precision_component(level.qty, qty_decimals));
    }
    for level in bids.iter().take(10) {
        payload.push_str(&checksum_fixed_precision_component(
            level.price,
            price_decimals,
        ));
        payload.push_str(&checksum_fixed_precision_component(level.qty, qty_decimals));
    }
    crc32fast::hash(payload.as_bytes())
}

/// Format a price/qty to a fixed number of decimals, then drop the decimal point
/// and leading zeros — the Kraken v2 book checksum digit encoding.
fn checksum_fixed_precision_component(value: f64, decimals: u8) -> String {
    let formatted = format!("{:.*}", decimals as usize, value);
    let mut compact = formatted.replace('.', "");
    while compact.starts_with('0') && compact.len() > 1 {
        compact.remove(0);
    }
    if compact.is_empty() {
        "0".to_string()
    } else {
        compact
    }
}

fn push_checksum_level(payload: &mut String, level: &KrakenWsBookLevel) {
    payload.push_str(&checksum_decimal_component(&level.price_text));
    payload.push_str(&checksum_decimal_component(&level.qty_text));
}

fn checksum_decimal_component(raw: &str) -> String {
    let normalized_source = if raw.contains(['e', 'E']) {
        raw.parse::<f64>()
            .ok()
            .map(format_decimal_for_book_level)
            .unwrap_or_else(|| raw.to_string())
    } else {
        raw.to_string()
    };
    let mut compact = normalized_source
        .trim()
        .trim_start_matches('+')
        .replace('.', "");
    while compact.starts_with('0') && compact.len() > 1 {
        compact.remove(0);
    }
    if compact.is_empty() {
        "0".to_string()
    } else {
        compact
    }
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
        assert_eq!(checksum_decimal_component("0.05000000"), "5000000");
        assert_eq!(checksum_decimal_component("67100.0"), "671000");
        assert_eq!(checksum_decimal_component("+001.2300"), "12300");
    }

    #[test]
    fn fixed_precision_component_reconstructs_trimmed_zeros() {
        // Round-numbered tokenized-xStock levels: the wire trims trailing zeros,
        // but the checksum must use the pair's fixed precision.
        assert_eq!(checksum_fixed_precision_component(190.0, 2), "19000");
        assert_eq!(checksum_fixed_precision_component(5.0, 8), "500000000");
        assert_eq!(checksum_fixed_precision_component(0.05, 8), "5000000");
        assert_eq!(checksum_fixed_precision_component(67100.0, 1), "671000");
        assert_eq!(checksum_fixed_precision_component(100.5, 4), "1005000");
    }

    #[test]
    fn fixed_precision_checksum_matches_wire_text_when_full_precision() {
        // When the wire already carries full precision, the precision-based
        // encoding reproduces the wire-text encoding exactly — so pairs whose wire
        // text is already padded (e.g. major crypto) are unaffected.
        let asks = vec![KrakenWsBookLevel::from_wire(
            100.5,
            0.05,
            "100.50".into(),
            "0.05000000".into(),
        )];
        let bids = vec![KrakenWsBookLevel::from_wire(
            100.0,
            2.5,
            "100.00".into(),
            "2.50000000".into(),
        )];
        assert_eq!(
            compute_book_checksum_with_precision(&bids, &asks, 2, 8),
            compute_book_checksum(&bids, &asks)
        );
    }

    #[test]
    fn registered_precision_fixes_trimmed_xstock_checksum() {
        // Tokenized xStock: the wire delivered trimmed text ("190" / "5"), so the
        // raw wire-text checksum diverges from Kraken's CRC. With the pair's
        // precision registered, the state recomputes against the fixed-precision
        // encoding — which is what stops the deterministic resubscribe loop.
        register_kraken_pair_book_precision("TESTx/USD", 2, 8);
        let state = KrakenWsBookState::new("TESTx/USD", 10);
        let delta = KrakenWsBookDelta {
            symbol: "TESTx/USD".into(),
            bids: vec![KrakenWsBookLevel::from_wire(
                189.0,
                3.0,
                "189".into(),
                "3".into(),
            )],
            asks: vec![KrakenWsBookLevel::from_wire(
                190.0,
                5.0,
                "190".into(),
                "5".into(),
            )],
            checksum: None,
            ts_ms: None,
            is_snapshot: true,
        };
        let mut applied = state.clone();
        applied.apply_delta_unchecked(&delta);
        let precision_checksum =
            compute_book_checksum_with_precision(&applied.bids, &applied.asks, 2, 8);
        let wire_checksum = compute_book_checksum(&applied.bids, &applied.asks);
        // The fix changes behavior for trimmed books, and the state's checksum now
        // uses the fixed-precision encoding (via the registry).
        assert_ne!(precision_checksum, wire_checksum);
        assert_eq!(applied.compute_checksum(), precision_checksum);

        // A delta carrying Kraken's (correct) checksum now validates instead of
        // looping forever on a mismatch.
        let mut good = state.clone();
        let mut accepted = good.clone();
        accepted.apply_delta_unchecked(&delta);
        let mut good_delta = delta.clone();
        good_delta.checksum = Some(u64::from(accepted.compute_checksum()));
        assert_eq!(
            good.apply_delta_with_checksum(&good_delta),
            Ok(Some(precision_checksum))
        );
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
}
