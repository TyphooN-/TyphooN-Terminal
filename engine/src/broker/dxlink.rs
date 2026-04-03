//! DXLink WebSocket client for tastytrade market data streaming.
//!
//! Protocol: JSON messages over WebSocket (wss://)
//! Handshake: SETUP → AUTH → CHANNEL_REQUEST → FEED_SETUP → FEED_SUBSCRIPTION
//! Data: COMPACT format (flat arrays with field ordering from FEED_SETUP)
//!
//! Reference: https://developer.tastytrade.com/streaming-market-data/

use serde::{Deserialize, Serialize};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};

const DXLINK_VERSION: &str = "0.1-DXF-JS/0.3.0";
const KEEPALIVE_SECS: u64 = 30;

/// DXLink streaming token + URL from tastytrade REST API.
#[derive(Debug, Clone)]
pub struct DxLinkToken {
    pub token: String,
    pub url: String,
}

/// A single OHLCV candle from DXLink.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DxCandle {
    pub symbol: String,
    pub time: i64,       // epoch ms
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

/// A live quote snapshot from DXLink.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DxQuote {
    pub symbol: String,
    pub bid: f64,
    pub ask: f64,
    pub bid_size: f64,
    pub ask_size: f64,
}

/// Get the DXLink streaming token from tastytrade REST API.
pub async fn get_streaming_token(base_url: &str, session_token: &str) -> Result<DxLinkToken, String> {
    let client = reqwest::Client::new();
    let url = format!("{}/api-quote-tokens", base_url);

    let resp = client.get(&url)
        .header("Authorization", session_token)
        .send().await
        .map_err(|e| format!("Get streaming token failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Streaming token request failed: {}", resp.status()));
    }

    let data: serde_json::Value = resp.json().await
        .map_err(|e| format!("Parse streaming token failed: {e}"))?;

    let token = data["data"]["token"].as_str()
        .ok_or("No token in response")?.to_string();
    let ws_url = data["data"]["dxlink-url"].as_str()
        .ok_or("No dxlink-url in response")?.to_string();

    Ok(DxLinkToken { token, url: ws_url })
}

/// Fetch historical candles via DXLink WebSocket.
/// Connects, authenticates, requests candles, collects results, disconnects.
pub async fn fetch_candles(
    dx_token: &DxLinkToken,
    symbol: &str,
    interval: &str, // "1d", "1h", "5m", etc.
    from_time_ms: i64,
) -> Result<Vec<DxCandle>, String> {
    // Connect
    let (ws_stream, _) = connect_async(&dx_token.url).await
        .map_err(|e| format!("DXLink connect failed: {e}"))?;
    let (mut sink, mut stream) = ws_stream.split();

    // Inline helpers (closures with async borrow don't work)
    macro_rules! dx_send {
        ($sink:expr, $msg:expr) => {
            $sink.send(Message::Text($msg.to_string().into())).await
                .map_err(|e| format!("DXLink send failed: {e}"))?
        };
    }
    macro_rules! dx_recv {
        ($stream:expr) => {{
            let mut result: Result<serde_json::Value, String> = Err("no message".into());
            loop {
                match $stream.next().await {
                    Some(Ok(Message::Text(txt))) => {
                        result = serde_json::from_str::<serde_json::Value>(&txt)
                            .map_err(|e| format!("DXLink parse failed: {e}"));
                        break;
                    }
                    Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) => continue,
                    Some(Ok(Message::Close(_))) => { result = Err("DXLink closed".into()); break; }
                    Some(Err(e)) => { result = Err(format!("DXLink error: {e}")); break; }
                    None => { result = Err("DXLink ended".into()); break; }
                    _ => continue,
                }
            }
            result
        }};
    }

    // Step 1: Send SETUP
    dx_send!(sink, serde_json::json!({
        "type": "SETUP",
        "channel": 0,
        "version": DXLINK_VERSION,
        "keepaliveTimeout": 60,
        "acceptKeepaliveTimeout": 60
    }));

    // Step 2: Wait for server SETUP
    let msg = dx_recv!(stream)?;
    if msg["type"] != "SETUP" {
        return Err(format!("Expected SETUP, got {:?}", msg["type"]));
    }

    // Step 3: Wait for AUTH_STATE UNAUTHORIZED
    let msg = dx_recv!(stream)?;
    if msg["type"] != "AUTH_STATE" || msg["state"] != "UNAUTHORIZED" {
        return Err(format!("Expected AUTH_STATE UNAUTHORIZED, got {:?}", msg));
    }

    // Step 4: Send AUTH
    dx_send!(sink, serde_json::json!({
        "type": "AUTH",
        "channel": 0,
        "token": dx_token.token
    }));

    // Step 5: Wait for AUTH_STATE AUTHORIZED
    let msg = dx_recv!(stream)?;
    if msg["type"] != "AUTH_STATE" || msg["state"] != "AUTHORIZED" {
        return Err(format!("DXLink auth failed: {:?}", msg));
    }

    // Step 6: Open channel 1 for Candle data
    dx_send!(sink, serde_json::json!({
        "type": "CHANNEL_REQUEST",
        "channel": 1,
        "service": "FEED",
        "parameters": { "contract": "AUTO" }
    }));

    // Wait for CHANNEL_OPENED
    loop {
        let msg = dx_recv!(stream)?;
        if msg["type"] == "CHANNEL_OPENED" && msg["channel"] == 1 { break; }
        if msg["type"] == "ERROR" {
            return Err(format!("DXLink channel error: {}", msg["message"]));
        }
    }

    // Step 7: FEED_SETUP — request Candle fields
    let candle_fields = vec![
        "eventSymbol", "eventTime", "eventFlags", "index", "time",
        "sequence", "count", "volume", "vwap", "bidVolume",
        "askVolume", "impVolatility", "openInterest",
        "open", "high", "low", "close",
    ];
    dx_send!(sink, serde_json::json!({
        "type": "FEED_SETUP",
        "channel": 1,
        "acceptAggregationPeriod": 0,
        "acceptDataFormat": "COMPACT",
        "acceptEventFields": {
            "Candle": candle_fields
        }
    }));

    // Wait for FEED_CONFIG
    loop {
        let msg = dx_recv!(stream)?;
        if msg["type"] == "FEED_CONFIG" && msg["channel"] == 1 { break; }
        if msg["type"] == "KEEPALIVE" { continue; }
    }

    // Step 8: Subscribe to candles
    let candle_symbol = format!("{}{{={}}}", symbol, interval);
    dx_send!(sink, serde_json::json!({
        "type": "FEED_SUBSCRIPTION",
        "channel": 1,
        "add": [{
            "symbol": candle_symbol,
            "type": "Candle",
            "fromTime": from_time_ms
        }]
    }));

    // Step 9: Collect candle data until snapshot complete
    let mut candles: Vec<DxCandle> = Vec::new();
    let field_count = candle_fields.len(); // 17 fields per candle
    let timeout = tokio::time::Duration::from_secs(30);
    let deadline = tokio::time::Instant::now() + timeout;

    loop {
        let msg_result = tokio::select! {
            next = stream.next() => {
                match next {
                    Some(Ok(Message::Text(txt))) => serde_json::from_str::<serde_json::Value>(&txt).map_err(|e| format!("Parse: {e}")),
                    Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) => continue,
                    Some(Ok(Message::Close(_))) => Err("DXLink closed".into()),
                    Some(Err(e)) => Err(format!("DXLink error: {e}")),
                    None => Err("DXLink ended".into()),
                    _ => continue,
                }
            }
            _ = tokio::time::sleep_until(deadline) => {
                break; // Timeout — return what we have
            }
        };
        let msg = msg_result?;

        if msg["type"] == "KEEPALIVE" { continue; }
        if msg["type"] != "FEED_DATA" || msg["channel"] != 1 { continue; }

        if let Some(data) = msg["data"].as_array() {
            let mut i = 0;
            while i < data.len() {
                if data[i].as_str() == Some("Candle") {
                    i += 1;
                    if let Some(values) = data.get(i).and_then(|v| v.as_array()) {
                        // Parse candles from flat array
                        let chunks = values.len() / field_count;
                        for c in 0..chunks {
                            let off = c * field_count;
                            let sym = values[off].as_str().unwrap_or("").to_string();
                            // Strip candle interval from symbol: "AAPL{=1d}" → "AAPL"
                            let clean_sym = sym.split('{').next().unwrap_or(&sym).to_string();
                            let time_ms = values[off + 4].as_i64().unwrap_or(0);
                            let open = parse_f64(&values[off + 13]);
                            let high = parse_f64(&values[off + 14]);
                            let low = parse_f64(&values[off + 15]);
                            let close = parse_f64(&values[off + 16]);
                            let volume = parse_f64(&values[off + 7]);
                            let flags = values[off + 2].as_i64().unwrap_or(0);

                            if !open.is_nan() && !close.is_nan() && time_ms > 0 {
                                candles.push(DxCandle {
                                    symbol: clean_sym,
                                    time: time_ms,
                                    open, high, low, close, volume,
                                });
                            }

                            // Check for SNAPSHOT_END (0x8) or SNAPSHOT_SNIP (0x10)
                            if flags & 0x8 != 0 || flags & 0x10 != 0 {
                                // Snapshot complete — close and return
                                let _ = sink.send(Message::Close(None)).await;
                                candles.sort_by_key(|c| c.time);
                                candles.dedup_by_key(|c| c.time);
                                return Ok(candles);
                            }
                        }
                    }
                }
                i += 1;
            }
        }
    }

    // Timed out — return what we collected
    let _ = sink.send(Message::Close(None)).await;
    candles.sort_by_key(|c| c.time);
    candles.dedup_by_key(|c| c.time);
    Ok(candles)
}

/// Parse f64 from JSON value (handles "NaN", "Infinity" strings).
fn parse_f64(v: &serde_json::Value) -> f64 {
    match v {
        serde_json::Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        serde_json::Value::String(s) => match s.as_str() {
            "NaN" => f64::NAN,
            "Infinity" => f64::INFINITY,
            "-Infinity" => f64::NEG_INFINITY,
            _ => s.parse().unwrap_or(f64::NAN),
        },
        _ => f64::NAN,
    }
}
