//! Kraken WS v2 OHLC streaming.
//!
//! Endpoint: `wss://ws.kraken.com/v2`. Subscribe to the `ohlc` channel with a
//! batch of symbols and one interval (minutes); Kraken pushes bar snapshots
//! plus per-tick updates as each bar evolves. A bar with the same
//! `interval_begin` is sent repeatedly; the last one before the interval
//! rolls over is the close.
//!
//! Why this exists at all: the public REST OHLC endpoint serialises every
//! request through a ~1 req/sec global counter, so 13k pairs × 9 timeframes
//! is unreachable for the low timeframes via REST alone. The WS push path
//! provides forward streaming so the cache stays current on 1Min/5Min/etc.
//! REST keeps doing cold-start historical backfill where it still wins.
//!
//! This module is the protocol layer: subscribe-frame batching, message
//! parsing into typed [`KrakenWsOhlcBar`]. The connection driver lives in
//! `connection.rs` alongside the reconnect / heartbeat logic.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

pub const KRAKEN_WS_V2_URL: &str = "wss://ws.kraken.com/v2";

/// Exponential backoff base; capped at [`KRAKEN_WS_RECONNECT_MAX`]. Used by
/// the streamer's reconnect loop after the WS drops.
const KRAKEN_WS_RECONNECT_INITIAL: Duration = Duration::from_secs(1);
const KRAKEN_WS_RECONNECT_MAX: Duration = Duration::from_secs(60);
/// How often the streamer sends an application-level ping. Kraken closes
/// idle connections after ~60s of silence on some pops, so 30s gives plenty
/// of headroom while still being cheap.
const KRAKEN_WS_PING_INTERVAL: Duration = Duration::from_secs(30);
/// Per-frame pause during the initial subscribe burst. Keep a small gap so a
/// single connection doesn't send all batches in one scheduler tick, but do
/// not drip-feed the full universe: low timeframe catch-up depends on the WS
/// snapshots landing immediately after AssetPairs discovery.
const KRAKEN_WS_SUBSCRIBE_FRAME_DELAY: Duration = Duration::from_millis(20);
/// During full-universe startup, Kraken begins sending snapshot payloads as
/// soon as each subscribe frame is accepted. Drain until the connection is idle
/// for this long before sending the next subscribe frame, so snapshots are
/// backpressured by the bounded writer instead of piling up behind a 51-frame
/// subscribe burst.
const KRAKEN_WS_SUBSCRIBE_DRAIN_IDLE: Duration = Duration::from_millis(50);
/// How long the subscribe-burst can take before we time it out and treat
/// the connection as broken. Sized so even the 13k/250 = 52 batches at
/// 1 frame/sec stay well under it.
const KRAKEN_WS_SUBSCRIBE_TIMEOUT: Duration = Duration::from_secs(120);

/// Kraken WS v2 caps subscribe frames at a few hundred symbols. We chunk at
/// 250 to stay comfortably under that ceiling without paying the per-frame
/// connect overhead too many times for large universes.
pub(crate) const KRAKEN_WS_SUBSCRIBE_BATCH: usize = 250;

/// Valid OHLC intervals (minutes) Kraken WS v2 serves on the `ohlc` channel.
/// Note: Kraken does not serve `MN1` natively; monthly bars are aggregated
/// from `1Day` by the existing REST path and the same aggregator can be
/// reused for the WS-fed daily bars.
pub const KRAKEN_WS_OHLC_INTERVALS_MIN: &[u32] = &[1, 5, 15, 30, 60, 240, 1440, 10080];

/// One bar emitted by the Kraken WS OHLC channel. `interval_begin_ms` is the
/// epoch-ms timestamp of the bar's left edge — i.e. the natural cache key
/// for upserting into the existing `kraken:SYMBOL:TF` bar series.
#[derive(Debug, Clone, PartialEq)]
pub struct KrakenWsOhlcBar {
    pub symbol: String,
    pub interval_min: u32,
    pub interval_begin_ms: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub vwap: Option<f64>,
    pub trades: u64,
    /// `true` for the initial snapshot batch on first subscribe; `false` for
    /// live updates. Snapshot bars overwrite anything we had cached at the
    /// same `interval_begin_ms`; update bars upsert by the same key.
    pub is_snapshot: bool,
}

/// Monotonic request-id source for outgoing WS frames. Kraken uses `req_id`
/// to correlate subscribe ACKs / NACKs with the originating subscribe.
static REQ_ID: AtomicU64 = AtomicU64::new(1);

pub(crate) fn next_req_id() -> u64 {
    REQ_ID.fetch_add(1, Ordering::Relaxed)
}

/// Build subscribe frames for the given (interval, symbols), chunked to
/// stay under Kraken's per-frame symbol cap. Each returned string is one
/// JSON message ready to send on the WS.
pub fn build_subscribe_frames(interval_min: u32, symbols: &[String]) -> Vec<String> {
    build_subscribe_frames_with_snapshot(interval_min, symbols, true)
}

/// Build subscribe frames with explicit snapshot control.
///
/// Kraken's default OHLC snapshot is useful for small/demand sets, but a full
/// 12k+ xStocks catalog can return hundreds of historical bars per symbol for
/// one interval. That is millions of bars before the app reaches live updates,
/// so large full-catalog startup must be able to subscribe live-only and leave
/// deep repair/backfill to the paced REST/assist lanes.
pub fn build_subscribe_frames_with_snapshot(
    interval_min: u32,
    symbols: &[String],
    snapshot: bool,
) -> Vec<String> {
    if symbols.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(symbols.len().div_ceil(KRAKEN_WS_SUBSCRIBE_BATCH));
    for batch in symbols.chunks(KRAKEN_WS_SUBSCRIBE_BATCH) {
        let frame = serde_json::json!({
            "method": "subscribe",
            "params": {
                "channel": "ohlc",
                "symbol": batch,
                "interval": interval_min,
                "snapshot": snapshot,
            },
            "req_id": next_req_id(),
        });
        out.push(frame.to_string());
    }
    out
}

/// Build the matching unsubscribe frame (used during planned shutdown so
/// Kraken stops pushing into a connection we're about to close).
pub fn build_unsubscribe_frame(interval_min: u32, symbols: &[String]) -> Option<String> {
    if symbols.is_empty() {
        return None;
    }
    let frame = serde_json::json!({
        "method": "unsubscribe",
        "params": {
            "channel": "ohlc",
            "symbol": symbols,
            "interval": interval_min,
        },
        "req_id": next_req_id(),
    });
    Some(frame.to_string())
}

/// Parse one incoming WS text frame into zero-or-more bars. Returns an empty
/// vec for non-OHLC frames (heartbeats, subscribe ACKs, system status). Only
/// the `ohlc` channel produces bar output.
pub fn parse_ohlc_message(text: &str) -> Vec<KrakenWsOhlcBar> {
    parse_ohlc_message_with_snapshot_policy(text, true)
}

/// Parse an OHLC frame while optionally dropping startup snapshots before
/// expanding their `data` array into per-bar structs. Large Kraken Securities
/// universes subscribe with `snapshot=false`, but this guard is deliberately
/// defensive: if Kraken, a proxy, or a reconnect path still delivers snapshot
/// frames, the app must not enqueue millions of historical buckets and OOM.
pub fn parse_ohlc_message_with_snapshot_policy(
    text: &str,
    accept_snapshots: bool,
) -> Vec<KrakenWsOhlcBar> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return Vec::new();
    };
    let Some(obj) = value.as_object() else {
        return Vec::new();
    };
    // Only ohlc channel frames carry bars; everything else (status,
    // subscribe ACK, pong) is silent here.
    if obj.get("channel").and_then(|v| v.as_str()) != Some("ohlc") {
        return Vec::new();
    }
    let is_snapshot = obj.get("type").and_then(|v| v.as_str()) == Some("snapshot");
    if is_snapshot && !accept_snapshots {
        return Vec::new();
    }
    let Some(data) = obj.get("data").and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    let mut bars = Vec::with_capacity(data.len());
    for entry in data {
        let Some(entry_obj) = entry.as_object() else {
            continue;
        };
        let Some(symbol) = entry_obj.get("symbol").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(interval) = entry_obj
            .get("interval")
            .and_then(|v| v.as_u64())
            .map(|n| n as u32)
        else {
            continue;
        };
        let Some(interval_begin_ms) = entry_obj
            .get("interval_begin")
            .and_then(|v| v.as_str())
            .and_then(parse_rfc3339_to_ms)
        else {
            continue;
        };
        let Some(open) = entry_obj.get("open").and_then(|v| v.as_f64()) else {
            continue;
        };
        let Some(high) = entry_obj.get("high").and_then(|v| v.as_f64()) else {
            continue;
        };
        let Some(low) = entry_obj.get("low").and_then(|v| v.as_f64()) else {
            continue;
        };
        let Some(close) = entry_obj.get("close").and_then(|v| v.as_f64()) else {
            continue;
        };
        // Reject obviously bad bars instead of poisoning the cache with NaN
        // or inverted high/low pairs.
        if ![open, high, low, close].iter().all(|v| v.is_finite())
            || high < low
            || open <= 0.0
            || close <= 0.0
        {
            continue;
        }
        let volume = entry_obj
            .get("volume")
            .and_then(|v| v.as_f64())
            .filter(|v| v.is_finite() && *v >= 0.0)
            .unwrap_or(0.0);
        let vwap = entry_obj
            .get("vwap")
            .and_then(|v| v.as_f64())
            .filter(|v| v.is_finite() && *v > 0.0);
        let trades = entry_obj
            .get("trades")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        bars.push(KrakenWsOhlcBar {
            symbol: symbol.to_string(),
            interval_min: interval,
            interval_begin_ms,
            open,
            high,
            low,
            close,
            volume,
            vwap,
            trades,
            is_snapshot,
        });
    }
    bars
}

/// Parse an RFC-3339 timestamp (the format Kraken uses for `interval_begin`)
/// into epoch milliseconds. Returns `None` for unparseable strings rather
/// than panicking on malformed feed data.
fn parse_rfc3339_to_ms(s: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&chrono::Utc).timestamp_millis())
}

/// `true` once the bar's bucket has ended in wall-clock terms — i.e.
/// `interval_begin_ms + interval_min * 60s <= now_ms`. The WS pipeline
/// uses this to gate cache writes: only closed bars are persisted, so the
/// open bar's per-tick rewrites of the same bucket no longer trigger a
/// full-history zstd-22 re-pack of the (symbol, tf) blob in
/// [`crate::core::cache::SqliteCache::merge_bars`]. With a 1Min snapshot
/// stream pushing dozens of in-progress updates per pair, this drops the
/// steady-state cache-write volume by ~60× and is the load-bearing fix
/// for the UI lag the WS feed introduced.
///
/// Saturating arithmetic so a clock-skewed or maliciously-old bar can't
/// overflow into the future and look "still open" forever.
pub fn ws_bar_is_closed(interval_min: u32, interval_begin_ms: i64, now_ms: i64) -> bool {
    let span_ms = (interval_min as i64).saturating_mul(60_000);
    let end_ms = interval_begin_ms.saturating_add(span_ms);
    end_ms <= now_ms
}

/// Map the Kraken WS interval (minutes) to TyphooN's cache timeframe
/// label. Returns `None` for any interval the cache doesn't store —
/// currently every WS-served interval has a matching cache key, so this
/// is essentially a total function over [`KRAKEN_WS_OHLC_INTERVALS_MIN`],
/// but a deliberate `None` for unknowns keeps the writer side honest if
/// Kraken ever ships new intervals.
pub fn kraken_ws_interval_to_tf_label(interval_min: u32) -> Option<&'static str> {
    match interval_min {
        1 => Some("1Min"),
        5 => Some("5Min"),
        15 => Some("15Min"),
        30 => Some("30Min"),
        60 => Some("1Hour"),
        240 => Some("4Hour"),
        1440 => Some("1Day"),
        10080 => Some("1Week"),
        _ => None,
    }
}

/// Convert a Kraken WS bar into the canonical JSON shape the cache stores
/// for every broker. The fields match what [`crate::core::cache::SqliteCache::merge_bars`]
/// looks for (`timestamp` as RFC-3339 + numeric OHLCV), so writes from the
/// WS path are indistinguishable from REST-fetched bars on read.
pub fn kraken_ws_bar_to_json(bar: &KrakenWsOhlcBar) -> serde_json::Value {
    let ts_rfc3339 = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(bar.interval_begin_ms)
        .map(|dt| dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
        .unwrap_or_default();
    serde_json::json!({
        "timestamp": ts_rfc3339,
        "open": bar.open,
        "high": bar.high,
        "low": bar.low,
        "close": bar.close,
        "volume": bar.volume,
    })
}

/// Normalise a Kraken WS pair like `"BTC/USD"` or `"XBT/USD"` to TyphooN's
/// flat cache form (`"BTCUSD"`). Strips the slash and runs the existing
/// pair-symbol normaliser so XBT → BTC and DOGE handling stay consistent
/// with the REST cache keys.
pub fn kraken_ws_symbol_to_cache_key(ws_symbol: &str) -> String {
    crate::core::kraken::normalize_pair_symbol(ws_symbol).replace('/', "")
}

/// Lightweight predicate for "this looks like a subscribe ACK". Used by the
/// connection driver to log subscription failures without trying to mine
/// channel data out of them.
pub fn is_subscribe_ack(text: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(text)
        .ok()
        .as_ref()
        .and_then(|v| v.get("method"))
        .and_then(|m| m.as_str())
        == Some("subscribe")
}

/// Pong/heartbeat frame so the connection driver can route them away from
/// the bar parser without an allocation.
pub fn is_heartbeat_or_status(text: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return false;
    };
    let channel = value.get("channel").and_then(|v| v.as_str());
    matches!(channel, Some("heartbeat") | Some("status") | Some("pong"))
}

/// Exponential backoff for the reconnect loop: 1s → 2s → 4s → 8s … capped at
/// 60s. Caller passes the consecutive-failure count (0-indexed); the first
/// retry waits 1s, the second 2s, and so on.
pub fn compute_reconnect_backoff(consecutive_failures: u32) -> Duration {
    let shift = consecutive_failures.min(8);
    let scaled = KRAKEN_WS_RECONNECT_INITIAL
        .as_secs()
        .checked_shl(shift)
        .unwrap_or(u64::MAX);
    Duration::from_secs(scaled).min(KRAKEN_WS_RECONNECT_MAX)
}

/// Status messages emitted by the streamer for the consuming app to log /
/// surface in the UI. Bars themselves flow on a separate channel so the
/// hot path doesn't allocate strings for status updates that the user
/// rarely sees.
#[derive(Debug, Clone)]
pub enum KrakenOhlcStreamerEvent {
    Connected { interval_min: u32 },
    Subscribed { interval_min: u32, batches: usize },
    Disconnected { interval_min: u32, reason: String },
    SubscribeFailed { interval_min: u32, reason: String },
}

/// Run a single Kraken WS v2 OHLC streamer for the given (interval, pairs).
/// Sends bars on `bar_tx` and lifecycle events on `event_tx`. Runs forever,
/// reconnecting with exponential backoff on any failure. Returns when
/// `bar_tx` is dropped by the consumer — that signals the app is shutting
/// down and there's no reader left to feed.
///
/// Designed to be spawned once per interval. The function does not hold any
/// references to the connecting `KrakenBroker` or REST client — it speaks
/// only to Kraken's public WS endpoint, which requires no authentication.
pub async fn run_ohlc_streamer(
    interval_min: u32,
    pairs: Vec<String>,
    bar_tx: mpsc::Sender<KrakenWsOhlcBar>,
    event_tx: mpsc::UnboundedSender<KrakenOhlcStreamerEvent>,
) {
    run_ohlc_streamer_with_snapshot(interval_min, pairs, true, bar_tx, event_tx).await;
}

/// Same as [`run_ohlc_streamer`], but lets callers disable the initial Kraken
/// OHLC snapshot for very large universes where startup history would exceed
/// memory budgets. Live updates still stream normally after subscribe.
pub async fn run_ohlc_streamer_with_snapshot(
    interval_min: u32,
    pairs: Vec<String>,
    snapshot: bool,
    bar_tx: mpsc::Sender<KrakenWsOhlcBar>,
    event_tx: mpsc::UnboundedSender<KrakenOhlcStreamerEvent>,
) {
    if pairs.is_empty() {
        return;
    }
    let mut consecutive_failures = 0u32;
    loop {
        if bar_tx.is_closed() {
            return;
        }
        let one_pass =
            run_ohlc_streamer_once(interval_min, &pairs, snapshot, &bar_tx, &event_tx).await;
        match one_pass {
            Ok(()) => {
                consecutive_failures = 0;
            }
            Err(e) => {
                consecutive_failures = consecutive_failures.saturating_add(1);
                let _ = event_tx.send(KrakenOhlcStreamerEvent::Disconnected {
                    interval_min,
                    reason: e,
                });
            }
        }
        let wait = compute_reconnect_backoff(consecutive_failures);
        if !wait.is_zero() {
            tokio::time::sleep(wait).await;
        }
    }
}

/// One connect / subscribe / read-until-error cycle. Returns `Ok(())` on a
/// clean close (the consumer dropped the bar channel) and `Err(reason)` on
/// any failure that should trigger a reconnect.
async fn run_ohlc_streamer_once(
    interval_min: u32,
    pairs: &[String],
    snapshot: bool,
    bar_tx: &mpsc::Sender<KrakenWsOhlcBar>,
    event_tx: &mpsc::UnboundedSender<KrakenOhlcStreamerEvent>,
) -> Result<(), String> {
    let (ws_stream, _) = connect_async(KRAKEN_WS_V2_URL)
        .await
        .map_err(|e| format!("ws connect failed: {e}"))?;
    let (mut sink, mut stream) = ws_stream.split();
    let _ = event_tx.send(KrakenOhlcStreamerEvent::Connected { interval_min });

    // Subscribe in batches. If Kraken NACKs one batch we keep going — better
    // to have partial coverage than to drop all subscriptions on this
    // connection. A bad-batch NACK is rare and usually indicates a single
    // delisted symbol; the per-batch ACK loop is the slow path so we just
    // log and move on.
    let frames = build_subscribe_frames_with_snapshot(interval_min, pairs, snapshot);
    let batches = frames.len();
    let subscribe_fut = async {
        for frame in &frames {
            sink.send(Message::Text(frame.clone().into()))
                .await
                .map_err(|e| format!("ws subscribe send failed: {e}"))?;

            // Kraken starts sending snapshot data immediately after each
            // subscribe frame. Do not send all 51 full-universe frames before
            // reading: that creates an unbounded startup burst. Drain until the
            // socket goes briefly idle; if the downstream writer is saturated,
            // `bar_tx.send(...).await` backpressures this loop and naturally
            // slows further subscriptions without shrinking universe coverage.
            loop {
                match tokio::time::timeout(KRAKEN_WS_SUBSCRIBE_DRAIN_IDLE, stream.next()).await {
                    Ok(Some(Ok(Message::Text(text)))) => {
                        if is_heartbeat_or_status(&text) || is_subscribe_ack(&text) {
                            continue;
                        }
                        for bar in parse_ohlc_message_with_snapshot_policy(&text, snapshot) {
                            if bar_tx.send(bar).await.is_err() {
                                return Ok::<(), String>(());
                            }
                        }
                    }
                    Ok(Some(Ok(Message::Ping(payload)))) => {
                        let _ = sink.send(Message::Pong(payload)).await;
                    }
                    Ok(Some(Ok(Message::Pong(_))))
                    | Ok(Some(Ok(Message::Binary(_))))
                    | Ok(Some(Ok(Message::Frame(_)))) => {}
                    Ok(Some(Ok(Message::Close(_)))) => {
                        return Err("ws closed by server during subscribe".to_string());
                    }
                    Ok(Some(Err(e))) => {
                        return Err(format!("ws read error during subscribe: {e}"));
                    }
                    Ok(None) => {
                        return Err("ws stream ended during subscribe".to_string());
                    }
                    Err(_) => break,
                }
            }

            // Brief pace so we don't trip Kraken's per-connection burst limit.
            tokio::time::sleep(KRAKEN_WS_SUBSCRIBE_FRAME_DELAY).await;
        }
        Ok::<(), String>(())
    };
    match tokio::time::timeout(KRAKEN_WS_SUBSCRIBE_TIMEOUT, subscribe_fut).await {
        Ok(Ok(())) => {
            let _ = event_tx.send(KrakenOhlcStreamerEvent::Subscribed {
                interval_min,
                batches,
            });
        }
        Ok(Err(e)) => {
            let _ = event_tx.send(KrakenOhlcStreamerEvent::SubscribeFailed {
                interval_min,
                reason: e.clone(),
            });
            return Err(e);
        }
        Err(_) => {
            let reason = "subscribe burst timed out".to_string();
            let _ = event_tx.send(KrakenOhlcStreamerEvent::SubscribeFailed {
                interval_min,
                reason: reason.clone(),
            });
            return Err(reason);
        }
    }

    // Heartbeat + read loop. We run a periodic ping on a tokio::time::interval
    // and `select!` between the WS read and the ping tick. Kraken's WS v2
    // accepts text "ping" frames; binary control pings work too but the
    // text form is documented and survives JSON-tolerant proxies.
    let mut ping_ticker = tokio::time::interval(KRAKEN_WS_PING_INTERVAL);
    ping_ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    // Skip the first tick — `interval` fires immediately on construction.
    ping_ticker.tick().await;

    loop {
        if bar_tx.is_closed() {
            return Ok(());
        }
        tokio::select! {
            msg = stream.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if is_heartbeat_or_status(&text) || is_subscribe_ack(&text) {
                            continue;
                        }
                        for bar in parse_ohlc_message_with_snapshot_policy(&text, snapshot) {
                            if bar_tx.send(bar).await.is_err() {
                                // Consumer is gone — clean shutdown.
                                return Ok(());
                            }
                        }
                    }
                    Some(Ok(Message::Ping(payload))) => {
                        let _ = sink.send(Message::Pong(payload)).await;
                    }
                    Some(Ok(Message::Pong(_))) | Some(Ok(Message::Binary(_)))
                    | Some(Ok(Message::Frame(_))) => {
                        // Ignore. Kraken doesn't use binary frames for OHLC.
                    }
                    Some(Ok(Message::Close(_))) => {
                        return Err("ws closed by server".to_string());
                    }
                    Some(Err(e)) => {
                        return Err(format!("ws read error: {e}"));
                    }
                    None => {
                        return Err("ws stream ended".to_string());
                    }
                }
            }
            _ = ping_ticker.tick() => {
                let ping = serde_json::json!({
                    "method": "ping",
                    "req_id": next_req_id(),
                }).to_string();
                if sink.send(Message::Text(ping.into())).await.is_err() {
                    return Err("ws ping send failed".to_string());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_subscribe_frames_batches_at_250() {
        let symbols: Vec<String> = (0..600).map(|i| format!("PAIR{i}/USD")).collect();
        let frames = build_subscribe_frames(1, &symbols);
        assert_eq!(frames.len(), 3); // 600 / 250 → ceil 3
        // First batch holds the first 250 pairs.
        let first: serde_json::Value = serde_json::from_str(&frames[0]).unwrap();
        let arr = first["params"]["symbol"].as_array().unwrap();
        assert_eq!(arr.len(), 250);
        assert_eq!(arr[0], "PAIR0/USD");
        assert_eq!(arr[249], "PAIR249/USD");
        // Last batch holds the leftover 100.
        let third: serde_json::Value = serde_json::from_str(&frames[2]).unwrap();
        let arr = third["params"]["symbol"].as_array().unwrap();
        assert_eq!(arr.len(), 100);
    }

    #[test]
    fn subscribe_burst_delay_keeps_full_universe_startup_fast() {
        let symbols: Vec<String> = (0..13_000).map(|i| format!("PAIR{i}/USD")).collect();
        let batches = build_subscribe_frames(1, &symbols).len() as u32;
        let expected_elapsed = KRAKEN_WS_SUBSCRIBE_FRAME_DELAY * batches;
        assert!(
            expected_elapsed <= Duration::from_secs(2),
            "full-universe subscribe snapshot should be online in <=2s per interval, got {expected_elapsed:?}"
        );
    }

    #[test]
    fn build_subscribe_frames_emits_canonical_v2_shape() {
        let frames = build_subscribe_frames(60, &["BTC/USD".to_string()]);
        let v: serde_json::Value = serde_json::from_str(&frames[0]).unwrap();
        assert_eq!(v["method"], "subscribe");
        assert_eq!(v["params"]["channel"], "ohlc");
        assert_eq!(v["params"]["interval"], 60);
        assert_eq!(v["params"]["snapshot"], true);
        assert!(v["req_id"].is_number());
    }

    #[test]
    fn build_subscribe_frames_can_disable_initial_snapshot() {
        let frames = build_subscribe_frames_with_snapshot(1, &["AAPLx/USD".to_string()], false);
        let v: serde_json::Value = serde_json::from_str(&frames[0]).unwrap();
        assert_eq!(v["params"]["snapshot"], false);
    }

    #[test]
    fn build_subscribe_frames_empty_input_returns_empty() {
        assert!(build_subscribe_frames(1, &[]).is_empty());
    }

    #[test]
    fn build_unsubscribe_frame_carries_method_and_pairs() {
        let frame = build_unsubscribe_frame(1, &["BTC/USD".into(), "ETH/USD".into()]).unwrap();
        let v: serde_json::Value = serde_json::from_str(&frame).unwrap();
        assert_eq!(v["method"], "unsubscribe");
        assert_eq!(v["params"]["channel"], "ohlc");
        assert_eq!(v["params"]["interval"], 1);
        assert_eq!(v["params"]["symbol"][0], "BTC/USD");
    }

    #[test]
    fn build_unsubscribe_frame_empty_input_returns_none() {
        assert!(build_unsubscribe_frame(1, &[]).is_none());
    }

    #[test]
    fn parse_ohlc_message_handles_snapshot_with_multiple_bars() {
        let text = r#"{
            "channel": "ohlc",
            "type": "snapshot",
            "data": [
                {
                    "symbol": "BTC/USD",
                    "open": 50000.0,
                    "high": 50100.0,
                    "low": 49900.0,
                    "close": 50050.0,
                    "volume": 1.5,
                    "vwap": 50025.0,
                    "trades": 25,
                    "interval_begin": "2026-05-23T19:00:00.000000Z",
                    "interval": 60,
                    "timestamp": "2026-05-23T19:00:30.123456Z"
                },
                {
                    "symbol": "ETH/USD",
                    "open": 2500.0,
                    "high": 2510.0,
                    "low": 2495.0,
                    "close": 2508.0,
                    "volume": 10.0,
                    "trades": 100,
                    "interval_begin": "2026-05-23T19:00:00.000000Z",
                    "interval": 60,
                    "timestamp": "2026-05-23T19:00:30.123456Z"
                }
            ]
        }"#;
        let bars = parse_ohlc_message(text);
        assert_eq!(bars.len(), 2);
        assert_eq!(bars[0].symbol, "BTC/USD");
        assert_eq!(bars[0].interval_min, 60);
        assert!(bars[0].is_snapshot);
        assert!((bars[0].open - 50000.0).abs() < f64::EPSILON);
        assert_eq!(bars[0].vwap, Some(50025.0));
        assert_eq!(bars[0].trades, 25);
        // 2026-05-23T19:00:00Z → ms since epoch (matches chrono parsing).
        let expected_ms = chrono::DateTime::parse_from_rfc3339("2026-05-23T19:00:00.000000Z")
            .unwrap()
            .timestamp_millis();
        assert_eq!(bars[0].interval_begin_ms, expected_ms);
        assert_eq!(bars[1].symbol, "ETH/USD");
        assert!(bars[1].vwap.is_none()); // missing vwap → None
    }

    #[test]
    fn parse_ohlc_message_can_drop_snapshot_frames_for_live_only_large_universe() {
        let text = r#"{
            "channel": "ohlc",
            "type": "snapshot",
            "data": [{
                "symbol": "AAPLx/USD",
                "open": 100.0, "high": 101.0, "low": 99.0, "close": 100.5,
                "volume": 10.0, "trades": 3,
                "interval_begin": "2026-05-23T19:00:00Z", "interval": 1
            }]
        }"#;
        assert!(parse_ohlc_message_with_snapshot_policy(text, false).is_empty());
        assert_eq!(parse_ohlc_message_with_snapshot_policy(text, true).len(), 1);
    }

    #[test]
    fn parse_ohlc_message_marks_update_frames_correctly() {
        let text = r#"{
            "channel": "ohlc",
            "type": "update",
            "data": [{
                "symbol": "BTC/USD",
                "open": 50000.0, "high": 50100.0, "low": 49900.0, "close": 50050.0,
                "volume": 1.5, "trades": 25,
                "interval_begin": "2026-05-23T19:00:00Z", "interval": 60
            }]
        }"#;
        let bars = parse_ohlc_message(text);
        assert_eq!(bars.len(), 1);
        assert!(!bars[0].is_snapshot);
    }

    #[test]
    fn parse_ohlc_message_rejects_non_ohlc_channel() {
        let text = r#"{
            "channel": "ticker",
            "type": "update",
            "data": [{"symbol": "BTC/USD"}]
        }"#;
        assert!(parse_ohlc_message(text).is_empty());
    }

    #[test]
    fn parse_ohlc_message_rejects_invalid_json() {
        assert!(parse_ohlc_message("not json").is_empty());
        assert!(parse_ohlc_message("").is_empty());
    }

    #[test]
    fn parse_ohlc_message_drops_bars_with_missing_fields() {
        let text = r#"{
            "channel": "ohlc",
            "type": "update",
            "data": [
                { "symbol": "BTC/USD", "open": 50000.0, "high": 50100.0, "low": 49900.0,
                  "close": 50050.0, "interval_begin": "2026-05-23T19:00:00Z", "interval": 60 },
                { "open": 1.0, "high": 1.0, "low": 1.0, "close": 1.0,
                  "interval_begin": "2026-05-23T19:00:00Z", "interval": 60 }
            ]
        }"#;
        let bars = parse_ohlc_message(text);
        // Second bar has no symbol → dropped.
        assert_eq!(bars.len(), 1);
        assert_eq!(bars[0].symbol, "BTC/USD");
    }

    #[test]
    fn parse_ohlc_message_drops_inverted_or_negative_bars() {
        let text = r#"{
            "channel": "ohlc",
            "type": "update",
            "data": [
                { "symbol": "BAD1", "open": 100.0, "high": 50.0, "low": 99.0, "close": 99.0,
                  "interval_begin": "2026-05-23T19:00:00Z", "interval": 60 },
                { "symbol": "BAD2", "open": 0.0, "high": 1.0, "low": 0.0, "close": 0.5,
                  "interval_begin": "2026-05-23T19:00:00Z", "interval": 60 }
            ]
        }"#;
        // Both rejected: BAD1 has high<low; BAD2 has open=0.
        assert!(parse_ohlc_message(text).is_empty());
    }

    #[test]
    fn parse_ohlc_message_drops_bars_with_unparseable_timestamp() {
        let text = r#"{
            "channel": "ohlc",
            "type": "update",
            "data": [{
                "symbol": "BTC/USD",
                "open": 50000.0, "high": 50100.0, "low": 49900.0, "close": 50050.0,
                "interval_begin": "not-an-rfc3339-stamp", "interval": 60
            }]
        }"#;
        assert!(parse_ohlc_message(text).is_empty());
    }

    #[test]
    fn is_subscribe_ack_matches_only_subscribe_method() {
        assert!(is_subscribe_ack(r#"{"method":"subscribe","success":true}"#));
        assert!(!is_subscribe_ack(r#"{"channel":"ohlc"}"#));
        assert!(!is_subscribe_ack("garbage"));
    }

    #[test]
    fn is_heartbeat_or_status_matches_known_channels() {
        assert!(is_heartbeat_or_status(r#"{"channel":"heartbeat"}"#));
        assert!(is_heartbeat_or_status(r#"{"channel":"status","data":[]}"#));
        assert!(is_heartbeat_or_status(r#"{"channel":"pong"}"#));
        assert!(!is_heartbeat_or_status(r#"{"channel":"ohlc"}"#));
    }

    #[test]
    fn next_req_id_returns_monotonic_ids() {
        let a = next_req_id();
        let b = next_req_id();
        let c = next_req_id();
        assert!(b > a);
        assert!(c > b);
    }

    #[test]
    fn compute_reconnect_backoff_doubles_each_failure() {
        assert_eq!(compute_reconnect_backoff(0), Duration::from_secs(1));
        assert_eq!(compute_reconnect_backoff(1), Duration::from_secs(2));
        assert_eq!(compute_reconnect_backoff(2), Duration::from_secs(4));
        assert_eq!(compute_reconnect_backoff(3), Duration::from_secs(8));
        assert_eq!(compute_reconnect_backoff(4), Duration::from_secs(16));
        assert_eq!(compute_reconnect_backoff(5), Duration::from_secs(32));
    }

    #[test]
    fn compute_reconnect_backoff_caps_at_60_seconds() {
        assert_eq!(compute_reconnect_backoff(6), KRAKEN_WS_RECONNECT_MAX);
        assert_eq!(compute_reconnect_backoff(20), KRAKEN_WS_RECONNECT_MAX);
        // Saturates without panicking on absurd inputs.
        assert_eq!(compute_reconnect_backoff(u32::MAX), KRAKEN_WS_RECONNECT_MAX);
    }

    #[tokio::test]
    async fn run_ohlc_streamer_returns_immediately_when_consumer_closes_channel() {
        // Drop the receiver before spawning so bar_tx.is_closed() returns
        // true on entry; the function must exit cleanly without trying
        // to connect to Kraken (this test runs offline).
        let (bar_tx, bar_rx) = mpsc::channel(1);
        drop(bar_rx);
        let (event_tx, _event_rx) = mpsc::unbounded_channel();
        let fut = run_ohlc_streamer(1, vec!["BTC/USD".to_string()], bar_tx, event_tx);
        // Must complete; a hung future would mean we tried to dial Kraken.
        tokio::time::timeout(Duration::from_secs(1), fut)
            .await
            .expect("streamer must exit when bar channel is closed");
    }

    #[tokio::test]
    async fn run_ohlc_streamer_no_op_for_empty_pair_list() {
        let (bar_tx, _bar_rx) = mpsc::channel(1);
        let (event_tx, _event_rx) = mpsc::unbounded_channel();
        let fut = run_ohlc_streamer(1, Vec::new(), bar_tx, event_tx);
        tokio::time::timeout(Duration::from_secs(1), fut)
            .await
            .expect("streamer must exit on empty pair list without dialing");
    }

    #[test]
    fn kraken_ws_interval_to_tf_label_covers_every_served_interval() {
        for &interval in KRAKEN_WS_OHLC_INTERVALS_MIN {
            assert!(
                kraken_ws_interval_to_tf_label(interval).is_some(),
                "interval {interval} from KRAKEN_WS_OHLC_INTERVALS_MIN must map"
            );
        }
        assert_eq!(kraken_ws_interval_to_tf_label(1), Some("1Min"));
        assert_eq!(kraken_ws_interval_to_tf_label(1440), Some("1Day"));
        assert_eq!(kraken_ws_interval_to_tf_label(10080), Some("1Week"));
        // Unknown interval (e.g. monthly via 21600) returns None — Kraken WS
        // doesn't serve it, so the writer must not invent a cache key.
        assert!(kraken_ws_interval_to_tf_label(21600).is_none());
        assert!(kraken_ws_interval_to_tf_label(0).is_none());
    }

    #[test]
    fn kraken_ws_bar_to_json_matches_cache_shape() {
        let bar = KrakenWsOhlcBar {
            symbol: "BTC/USD".into(),
            interval_min: 60,
            interval_begin_ms: 1_700_000_000_000,
            open: 50_000.0,
            high: 50_100.0,
            low: 49_900.0,
            close: 50_050.0,
            volume: 1.5,
            vwap: Some(50_025.0),
            trades: 25,
            is_snapshot: true,
        };
        let json = kraken_ws_bar_to_json(&bar);
        assert_eq!(json["open"], 50_000.0);
        assert_eq!(json["high"], 50_100.0);
        assert_eq!(json["low"], 49_900.0);
        assert_eq!(json["close"], 50_050.0);
        assert_eq!(json["volume"], 1.5);
        // Cache merger keys by RFC-3339 timestamp — must round-trip through
        // chrono parse without losing the bar.
        let ts = json["timestamp"].as_str().expect("timestamp string");
        let parsed = chrono::DateTime::parse_from_rfc3339(ts).expect("parseable");
        assert_eq!(parsed.timestamp_millis(), 1_700_000_000_000);
    }

    #[test]
    fn ws_bar_is_closed_returns_false_while_bucket_is_open() {
        // 1Min bar starting at 10:00:00; now is 10:00:30 → bucket runs to 10:01:00, still open.
        let begin = 1_700_000_000_000;
        let now = begin + 30_000;
        assert!(!ws_bar_is_closed(1, begin, now));
    }

    #[test]
    fn ws_bar_is_closed_returns_true_at_or_after_bucket_end() {
        let begin = 1_700_000_000_000;
        // Exactly at the closing instant: bucket end is inclusive of "closed" so a
        // bar whose interval has just rolled is flushable.
        assert!(ws_bar_is_closed(1, begin, begin + 60_000));
        assert!(ws_bar_is_closed(1, begin, begin + 60_000 + 1));
    }

    #[test]
    fn ws_bar_is_closed_scales_with_interval_minutes() {
        let begin = 1_700_000_000_000;
        // 5Min bar: still open at +4min, closed at +5min.
        assert!(!ws_bar_is_closed(5, begin, begin + 4 * 60_000));
        assert!(ws_bar_is_closed(5, begin, begin + 5 * 60_000));
        // 1Day bar: still open at +23h, closed at +24h.
        assert!(!ws_bar_is_closed(1440, begin, begin + 23 * 3_600_000));
        assert!(ws_bar_is_closed(1440, begin, begin + 24 * 3_600_000));
    }

    #[test]
    fn ws_bar_is_closed_snapshot_historical_bars_always_flushable() {
        // Snapshot delivery brings closed historical bars whose interval_begin is
        // far in the past relative to now. Every served interval should report
        // them as closed so the first flush persists the backfill.
        let now = 1_700_000_000_000;
        for &interval_min in KRAKEN_WS_OHLC_INTERVALS_MIN {
            // Place the bar two full periods in the past.
            let begin = now - (interval_min as i64) * 60_000 * 2;
            assert!(
                ws_bar_is_closed(interval_min, begin, now),
                "interval {interval_min} historical bar must be reported closed",
            );
        }
    }

    #[test]
    fn ws_bar_is_closed_handles_future_begin_without_overflow() {
        // Defensive: clock skew or a malformed feed could push interval_begin into
        // the future. Must not overflow and must report not-closed.
        let now = 1_700_000_000_000;
        assert!(!ws_bar_is_closed(1, now + 10 * 60_000, now));
        // Absurd extremes saturate to a sane "still open" answer rather than
        // wrapping around to a phantom past close.
        assert!(!ws_bar_is_closed(u32::MAX, i64::MAX, now));
    }

    #[test]
    fn kraken_ws_symbol_to_cache_key_drops_slash_and_normalises_xbt() {
        // XBT (Kraken's legacy BTC code) folds into BTC via the existing
        // pair normaliser so WS-written keys collide with REST-written keys.
        assert_eq!(kraken_ws_symbol_to_cache_key("XBT/USD"), "BTCUSD");
        assert_eq!(kraken_ws_symbol_to_cache_key("BTC/USD"), "BTCUSD");
        assert_eq!(kraken_ws_symbol_to_cache_key("ETH/USD"), "ETHUSD");
        // Already-flat form passes through.
        assert_eq!(kraken_ws_symbol_to_cache_key("ETHUSD"), "ETHUSD");
    }
}
