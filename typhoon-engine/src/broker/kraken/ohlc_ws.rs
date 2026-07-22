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
/// Hard upper bound on how long a single subscribe frame drains before we send
/// the next one. The idle-detection above breaks when the socket goes quiet for
/// `KRAKEN_WS_SUBSCRIBE_DRAIN_IDLE`, but an *active* low-timeframe interval
/// (15Min/30Min/4Hour) over a large pair universe streams bars continuously, so
/// the 50ms idle gap never appears — the drain never breaks, the outer loop
/// never reaches the next batch, and the whole burst hits
/// `KRAKEN_WS_SUBSCRIBE_TIMEOUT` and reconnects (the every-2-3-min resubscribe
/// storm in the live log). Capping each frame's drain lets the burst always
/// progress through every batch; the writer's `bar_tx.send().await`
/// backpressure still bounds in-flight snapshot memory. At the worst 13k/250≈52
/// batches this keeps the full burst (52 × 1.5s ≈ 78s) under the 120s timeout.
const KRAKEN_WS_SUBSCRIBE_DRAIN_MAX: Duration = Duration::from_millis(1_500);
/// How long the subscribe-burst can take before we time it out and treat
/// the connection as broken. Sized so even the 13k/250 = 52 batches at
/// 1 frame/sec stay well under it.
const KRAKEN_WS_SUBSCRIBE_TIMEOUT: Duration = Duration::from_secs(120);
/// Snapshot sweep connections are intentionally short-lived: subscribe one
/// bounded catalog batch, drain the initial history burst, unsubscribe, close.
/// This idle window decides when the snapshot burst is done.
const KRAKEN_WS_SNAPSHOT_SWEEP_DRAIN_IDLE: Duration = Duration::from_millis(750);
/// Wall-clock cap on a single snapshot-sweep drain, mirroring
/// `KRAKEN_WS_SUBSCRIBE_DRAIN_MAX`: the 750ms idle test never fires on an
/// actively-streaming universe, so an uncapped drain would hang the sweep
/// indefinitely. Sweeps are bounded high-TF batches, so a few seconds is ample
/// for the snapshot to land while still guaranteeing forward progress.
const KRAKEN_WS_SNAPSHOT_SWEEP_DRAIN_MAX: Duration = Duration::from_secs(5);

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

/// Subscribe one bounded OHLC batch with `snapshot=true`, drain its startup
/// history, unsubscribe, and return. This is the catalog-breadth lane for
/// Kraken xStocks/equities: it harvests recent Kraken-native history without
/// holding a permanent full-catalog live subscription.
pub async fn run_ohlc_snapshot_sweep_once(
    interval_min: u32,
    pairs: Vec<String>,
    bar_tx: mpsc::Sender<KrakenWsOhlcBar>,
    event_tx: mpsc::UnboundedSender<KrakenOhlcStreamerEvent>,
) -> Result<(), String> {
    if pairs.is_empty() {
        return Ok(());
    }
    let (ws_stream, _) = connect_async(KRAKEN_WS_V2_URL)
        .await
        .map_err(|e| format!("snapshot sweep ws connect failed: {e}"))?;
    let (mut sink, mut stream) = ws_stream.split();
    let _ = event_tx.send(KrakenOhlcStreamerEvent::Connected { interval_min });

    let frames = build_subscribe_frames_with_snapshot(interval_min, &pairs, true);
    let batches = frames.len();
    for frame in &frames {
        sink.send(Message::Text(frame.clone().into()))
            .await
            .map_err(|e| format!("snapshot sweep subscribe send failed: {e}"))?;
        drain_ohlc_ws_until_idle(
            &mut sink,
            &mut stream,
            &bar_tx,
            true,
            KRAKEN_WS_SNAPSHOT_SWEEP_DRAIN_IDLE,
        )
        .await?;
        tokio::time::sleep(KRAKEN_WS_SUBSCRIBE_FRAME_DELAY).await;
    }
    let _ = event_tx.send(KrakenOhlcStreamerEvent::Subscribed {
        interval_min,
        batches,
    });

    if let Some(frame) = build_unsubscribe_frame(interval_min, &pairs) {
        sink.send(Message::Text(frame.into()))
            .await
            .map_err(|e| format!("snapshot sweep unsubscribe send failed: {e}"))?;
        drain_ohlc_ws_until_idle(
            &mut sink,
            &mut stream,
            &bar_tx,
            true,
            KRAKEN_WS_SNAPSHOT_SWEEP_DRAIN_IDLE,
        )
        .await?;
    }
    let _ = sink.send(Message::Close(None)).await;
    Ok(())
}

async fn drain_ohlc_ws_until_idle<S>(
    sink: &mut futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<S>, Message>,
    stream: &mut futures_util::stream::SplitStream<tokio_tungstenite::WebSocketStream<S>>,
    bar_tx: &mpsc::Sender<KrakenWsOhlcBar>,
    accept_snapshots: bool,
    idle: Duration,
) -> Result<(), String>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    // Bound the total drain: on an active universe the `idle` gap never
    // appears, so idle-detection alone would spin here forever.
    let deadline = tokio::time::Instant::now() + KRAKEN_WS_SNAPSHOT_SWEEP_DRAIN_MAX;
    loop {
        if tokio::time::Instant::now() >= deadline {
            return Ok(());
        }
        match tokio::time::timeout(idle, stream.next()).await {
            Ok(Some(Ok(Message::Text(text)))) => {
                if is_heartbeat_or_status(&text) || is_subscribe_ack(&text) {
                    continue;
                }
                for bar in parse_ohlc_message_with_snapshot_policy(&text, accept_snapshots) {
                    if bar_tx.send(bar).await.is_err() {
                        return Ok(());
                    }
                }
            }
            Ok(Some(Ok(Message::Ping(payload)))) => {
                let _ = sink.send(Message::Pong(payload)).await;
            }
            Ok(Some(Ok(Message::Pong(_))))
            | Ok(Some(Ok(Message::Binary(_))))
            | Ok(Some(Ok(Message::Frame(_)))) => {}
            Ok(Some(Ok(Message::Close(_)))) => return Ok(()),
            Ok(Some(Err(e))) => return Err(format!("snapshot sweep ws read error: {e}")),
            Ok(None) => return Ok(()),
            Err(_) => return Ok(()),
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
            // A wall-clock cap guarantees the drain also ends when the socket
            // never goes idle (active low-TF universe), so the burst still
            // reaches every batch instead of hanging until the 120s timeout.
            let drain_deadline = tokio::time::Instant::now() + KRAKEN_WS_SUBSCRIBE_DRAIN_MAX;
            loop {
                if tokio::time::Instant::now() >= drain_deadline {
                    break;
                }
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
mod tests;
