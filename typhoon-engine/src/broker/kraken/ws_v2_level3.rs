//! Kraken WebSocket v2 Level 3 (per-order / market-by-order) parser and basic streamer.
//!
//! L3 requires authenticated connection (token) and entitlements.
//! See ADR-109 and ADR-129.
//! This provides the wiring foundation. Real auth + full delta apply with checksums
//! can be added mirroring ws_v2_book + private_ws when keys are available.

use std::time::Duration;

use super::ws_v2::{
    KRAKEN_WS_V2_LEVEL3_URL, KRAKEN_WS_V2_STALE_AFTER, build_ws_v2_subscribe_frame,
    ws_v2_connection_is_stale,
};
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
    // Exact wire text for CRC (mirrors book price_text/qty_text)
    pub price_text: Option<String>,
    pub qty_text: Option<String>,
    // Runtime received time (millis since epoch) for age persistence/coloring even if wire ts absent
    pub received_at_ms: Option<u64>,
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
    token: Option<String>,  // pass Some(token) when entitled
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
        match run_level3_streamer_once(&symbols, &token, &l3_tx, &event_tx).await {
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
    token: &Option<String>,
    l3_tx: &mpsc::Sender<KrakenL3Delta>,
    event_tx: &mpsc::UnboundedSender<String>,
) -> Result<(), String> {
    let (ws_stream, _) = connect_async(KRAKEN_WS_V2_LEVEL3_URL)
        .await
        .map_err(|e| format!("L3 ws connect failed: {e}"))?;
    let (mut sink, mut stream) = ws_stream.split();

    let connected_msg = if token.is_some() { "L3 connected (auth path)" } else { "L3 connected (demo/sim - no token)" };
    let _ = event_tx.send(connected_msg.into());

    // Subscribe with token if provided (actual auth wiring)
    let subscribe_frame = build_ws_v2_subscribe_frame(
        "level3",
        symbols,
        {
            let mut p = serde_json::Map::new();
            p.insert("snapshot".to_string(), serde_json::Value::Bool(true));
            if let Some(t) = token {
                p.insert("token".to_string(), serde_json::Value::String(t.clone()));
            }
            p
        },
    );
    sink.send(Message::Text(subscribe_frame.into()))
        .await
        .map_err(|e| format!("L3 subscribe send failed: {e}"))?;

    let _ = event_tx.send(format!("L3 subscribed for {:?}", symbols));

    let mut state = KrakenL3State {
        symbol: symbols.get(0).cloned().unwrap_or_default(),
        ..Default::default()
    };

    // Real consume + state maintenance. L3 deltas feed same KrakenOrderbookUpdate / KrakenBookQuoteTick paths as L2
    // (provides implicit aggregated L2 projection for cross-check / downstream consumers, per ADR-109 Phase 5).
    // Fall back to sim if no token or empty.
    let mut tick = 0u64;
    // Half-open watchdog (auth path): refreshed by any received frame; a lapse
    // past KRAKEN_WS_V2_STALE_AFTER forces a reconnect via the outer loop.
    let mut last_frame = std::time::Instant::now();
    loop {
        if l3_tx.is_closed() {
            return Ok(());
        }

        // Real WS path + state maintenance
        let received = tokio::time::timeout(Duration::from_millis(1500), stream.next()).await;
        match received {
            Ok(Some(Ok(Message::Text(text)))) => {
                last_frame = std::time::Instant::now();
                for delta in parse_l3_message(&text) {
                    // Full real-feed CRC on live deltas (when checksum present; applies to real auth + sim test paths)
                    let validated = if delta.checksum.is_some() {
                        match state.apply_delta_with_checksum(&delta) {
                            Ok(Some(actual)) => {
                                let _ = event_tx.send(format!("L3 real-feed CRC OK {}: {}", delta.symbol, actual));
                                let mut d = delta.clone();
                                d.checksum = Some(actual as u64);
                                d
                            }
                            Err(e) => {
                                let _ = event_tx.send(format!("L3 real-feed CRC MISMATCH {} exp={} act={} (keeping delta; prod: resub on mismatch)", e.symbol, e.expected, e.actual));
                                delta
                            }
                            _ => delta,
                        }
                    } else {
                        state.apply_delta(&delta);
                        delta
                    };
                    if l3_tx.send(validated).await.is_err() {
                        return Ok(());
                    }
                }
            }
            // Auth path: any non-text frame (heartbeat/ping/pong) is liveness.
            Ok(Some(Ok(_))) if token.is_some() => {
                last_frame = std::time::Instant::now();
            }
            // Auth path: surface hard failures so the outer loop reconnects
            // (mirrors ticker/book/trade). The demo/sim branch below is untouched.
            Ok(Some(Err(e))) if token.is_some() => {
                return Err(format!("L3 ws read error: {e}"));
            }
            Ok(None) if token.is_some() => {
                return Err("L3 ws stream ended".into());
            }
            _ => {
                // Fallback/demo sim when no real data or no token -- route through CRC validation for full path test
                if token.is_none() {
                    let sim = simulate_l3_delta(symbols.get(0).cloned().unwrap_or("DEMO/USD".into()), tick);
                    let validated_sim = if sim.checksum.is_some() {
                        match state.apply_delta_with_checksum(&sim) {
                            Ok(Some(_)) => sim,
                            Err(_) => { state.apply_delta(&sim); sim },
                            _ => sim,
                        }
                    } else {
                        state.apply_delta(&sim);
                        sim
                    };
                    if l3_tx.send(validated_sim).await.is_err() {
                        return Ok(());
                    }
                    tick += 1;
                    tokio::time::sleep(Duration::from_millis(300)).await;
                    continue;
                }
                // Real auth path with no frame this interval (timeout). Kraken v2
                // heartbeats keep an alive feed non-silent, so a lapse past the
                // window means a half-open socket — reconnect.
                if ws_v2_connection_is_stale(last_frame.elapsed(), KRAKEN_WS_V2_STALE_AFTER) {
                    return Err("L3 ws stale: no frame within window; reconnecting".into());
                }
            }
        }

        tick += 1;
        // small sleep only on fallback path; real stream drives rate
        if token.is_none() {
            tokio::time::sleep(Duration::from_millis(800)).await;
        }
    }
}

fn simulate_l3_delta(symbol: String, tick: u64) -> KrakenL3Delta {
    // Enhanced for real-feed CRC path testing + age demo + variety for gated L3 sim (add/mod/delete mix, proper received_at_ms)
    let base = 100.0 + (tick % 7) as f64 * 0.12;
    let ts = Some(format!("{}", 1000000000 + tick));
    let now_ms = Some((1000000000u64 + tick * 100) as u64);
    let csum = if tick % 4 == 0 { Some(0xBEEF_CAFEu64 + tick) } else { None };

    let mut bids = vec![KrakenL3Level {
        order_id: format!("B{}", tick),
        limit_price: base - 0.05,
        order_qty: 1.2 + (tick % 3) as f64 * 0.15,
        timestamp: ts.clone(),
        price_text: Some(format!("{:.4}", base - 0.05)),
        qty_text: Some("1.2".into()),
        received_at_ms: now_ms,
    }];

    let asks = vec![KrakenL3Level {
        order_id: format!("A{}", tick),
        limit_price: base + 0.05,
        order_qty: 2.5,
        timestamp: ts.clone(),
        price_text: Some(format!("{:.4}", base + 0.05)),
        qty_text: Some("2.5".into()),
        received_at_ms: now_ms,
    }];

    if tick % 7 == 3 {
        bids.clear(); // delete sim
    }
    if tick % 5 == 2 {
        if let Some(b) = bids.first_mut() {
            b.order_qty *= 0.7; // mod sim
        }
    }

    KrakenL3Delta {
        symbol,
        bids,
        asks,
        checksum: csum,
        is_snapshot: tick % 6 == 0,
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
                let price_text = l.get("limit_price").map(|v| v.to_string());
                let qty_text = l.get("order_qty").map(|v| v.to_string());
                res.push(KrakenL3Level {
                    order_id: oid,
                    limit_price: price,
                    order_qty: qty,
                    timestamp: l.get("timestamp").and_then(|t| t.as_str()).map(|s| s.to_string()),
                    price_text,
                    qty_text,
                    received_at_ms: None,
                });
            }
        }
    }
    res
}

/// Deeper L3 state for per-order delta apply (add/mod/delete by order_id).
#[derive(Debug, Clone, Default)]
pub struct KrakenL3State {
    pub symbol: String,
    pub bids: Vec<KrakenL3Level>,
    pub asks: Vec<KrakenL3Level>,
    pub last_checksum: Option<u64>,
}

impl KrakenL3State {
    pub fn apply_delta(&mut self, delta: &KrakenL3Delta) {
        self.symbol = delta.symbol.clone();
        if delta.is_snapshot {
            self.bids.clear();
            self.asks.clear();
        }
        apply_l3_levels(&mut self.bids, &delta.bids);
        apply_l3_levels(&mut self.asks, &delta.asks);
        self.last_checksum = delta.checksum;
    }

    pub fn compute_checksum(&self) -> u32 {
        compute_l3_checksum(&self.bids, &self.asks)
    }

    pub fn apply_delta_with_checksum(
        &mut self,
        delta: &KrakenL3Delta,
    ) -> Result<Option<u32>, KrakenL3ChecksumError> {
        let mut next = self.clone();
        next.apply_delta(delta);
        let Some(expected) = delta.checksum else {
            *self = next;
            return Ok(None);
        };
        let actual = next.compute_checksum();
        if u64::from(actual) == expected {
            *self = next;
            Ok(Some(actual))
        } else {
            Err(KrakenL3ChecksumError {
                symbol: delta.symbol.clone(),
                expected,
                actual,
            })
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KrakenL3ChecksumError {
    pub symbol: String,
    pub expected: u64,
    pub actual: u32,
}

pub fn compute_l3_checksum(bids: &[KrakenL3Level], asks: &[KrakenL3Level]) -> u32 {
    let mut payload = String::new();
    for level in asks.iter().take(10) {
        push_l3_checksum_level(&mut payload, level);
    }
    for level in bids.iter().take(10) {
        push_l3_checksum_level(&mut payload, level);
    }
    crc32fast::hash(payload.as_bytes())
}

fn push_l3_checksum_level(payload: &mut String, level: &KrakenL3Level) {
    let p_owned = level.price_text.clone().unwrap_or_else(|| level.limit_price.to_string());
    let q_owned = level.qty_text.clone().unwrap_or_else(|| level.order_qty.to_string());
    let p = p_owned.as_str();
    let q = q_owned.as_str();
    payload.push_str(&checksum_decimal_component(p));
    payload.push_str(&checksum_decimal_component(q));
}

fn checksum_decimal_component(raw: &str) -> String {
    let normalized = if raw.contains(['e', 'E']) {
        raw.parse::<f64>().ok().map(|v| if v.fract() == 0.0 { format!("{v:.1}") } else { v.to_string() }).unwrap_or_else(|| raw.to_string())
    } else { raw.to_string() };
    let mut compact = normalized.trim().trim_start_matches('+').replace('.', "");
    while compact.starts_with('0') && compact.len() > 1 { compact.remove(0); }
    if compact.is_empty() { "0".to_string() } else { compact }
}

fn apply_l3_levels(levels: &mut Vec<KrakenL3Level>, updates: &[KrakenL3Level]) {
    for u in updates {
        if u.order_qty <= 0.0 {
            levels.retain(|l| l.order_id != u.order_id);
        } else if let Some(ex) = levels.iter_mut().find(|l| l.order_id == u.order_id) {
            ex.limit_price = u.limit_price;
            ex.order_qty = u.order_qty;
            ex.timestamp = u.timestamp.clone();
        } else {
            levels.push(u.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l3_state_apply_and_checksum_basic() {
        let mut state = KrakenL3State { symbol: "TEST/USD".into(), ..Default::default() };

        let snap = KrakenL3Delta {
            symbol: "TEST/USD".into(),
            bids: vec![
                KrakenL3Level { order_id: "b1".into(), limit_price: 100.0, order_qty: 1.0, timestamp: None, price_text: Some("100.0".into()), qty_text: Some("1.0".into()), received_at_ms: None },
                KrakenL3Level { order_id: "b2".into(), limit_price: 99.9, order_qty: 2.0, timestamp: None, price_text: Some("99.9".into()), qty_text: Some("2.0".into()), received_at_ms: None },
            ],
            asks: vec![
                KrakenL3Level { order_id: "a1".into(), limit_price: 100.1, order_qty: 0.5, timestamp: None, price_text: Some("100.1".into()), qty_text: Some("0.5".into()), received_at_ms: None },
            ],
            checksum: None,
            is_snapshot: true,
        };

        state.apply_delta(&snap);
        assert_eq!(state.bids.len(), 2);
        assert_eq!(state.asks.len(), 1);

        let cs = state.compute_checksum();
        assert!(cs > 0);

        // delta modify
        let mod_delta = KrakenL3Delta {
            symbol: "TEST/USD".into(),
            bids: vec![KrakenL3Level { order_id: "b1".into(), limit_price: 100.0, order_qty: 1.5, timestamp: None, price_text: Some("100.0".into()), qty_text: Some("1.5".into()), received_at_ms: None }],
            asks: vec![],
            checksum: None,
            is_snapshot: false,
        };
        state.apply_delta(&mod_delta);
        assert_eq!(state.bids[0].order_qty, 1.5);

        // delete
        let del = KrakenL3Delta {
            symbol: "TEST/USD".into(),
            bids: vec![KrakenL3Level { order_id: "b2".into(), limit_price: 0.0, order_qty: 0.0, timestamp: None, price_text: Some("0".into()), qty_text: Some("0".into()), received_at_ms: None }],
            asks: vec![],
            checksum: None,
            is_snapshot: false,
        };
        state.apply_delta(&del);
        assert_eq!(state.bids.len(), 1);
    }

    #[test]
    fn l3_checksum_apply_with_mismatch_and_age() {
        let mut state = KrakenL3State { symbol: "TEST2/USD".into(), ..Default::default() };

        let snap = KrakenL3Delta {
            symbol: "TEST2/USD".into(),
            bids: vec![KrakenL3Level {
                order_id: "b1".into(),
                limit_price: 50.0,
                order_qty: 1.0,
                timestamp: Some("2026-07-01T00:00:00Z".into()),
                price_text: Some("50.0".into()),
                qty_text: Some("1.0".into()),
                received_at_ms: None,
            }],
            asks: vec![],
            checksum: Some(12345678), // bogus for test
            is_snapshot: true,
        };

        // mismatch: state not committed (safe design like L2), but Err returned and timestamp logic exercised via apply_delta
        let res = state.apply_delta_with_checksum(&snap);
        assert!(res.is_err());
        assert_eq!(state.bids.len(), 0); // not applied on mismatch

        // use plain apply for the snapshot to set state + timestamp
        state.apply_delta(&snap);
        assert_eq!(state.bids.len(), 1);
        assert!(state.bids[0].timestamp.is_some());

        // now a no-checksum delta (should apply cleanly)
        let good = KrakenL3Delta {
            symbol: "TEST2/USD".into(),
            bids: vec![],
            asks: vec![KrakenL3Level {
                order_id: "a1".into(),
                limit_price: 50.1,
                order_qty: 0.5,
                timestamp: None,
                price_text: Some("50.1".into()),
                qty_text: Some("0.5".into()),
                received_at_ms: None,
            }],
            checksum: None,
            is_snapshot: false,
        };
        let res = state.apply_delta_with_checksum(&good);
        assert!(res.is_ok());
        assert_eq!(state.asks.len(), 1);
    }

    #[test]
    fn l3_parse_and_apply_fixture_style() {
        // inline fixture-like JSON (as desired in ADR-109) — object form matching parser
        let json = r#"{
            "channel":"level3",
            "type":"snapshot",
            "data":[{
                "symbol":"FIX/USD",
                "bids":[{"order_id":"b1","limit_price":99.9,"order_qty":2.0,"timestamp":"2026-07-01T12:00:00Z"}],
                "asks":[{"order_id":"a1","limit_price":100.1,"order_qty":1.0}]
            }]
        }"#;

        let deltas = parse_l3_message(json);
        assert!(!deltas.is_empty());
        let mut state = KrakenL3State { symbol: "FIX/USD".into(), ..Default::default() };
        for d in &deltas {
            let _ = state.apply_delta_with_checksum(d);
        }
        assert_eq!(state.bids.len(), 1);
        assert_eq!(state.asks.len(), 1);
    }
}