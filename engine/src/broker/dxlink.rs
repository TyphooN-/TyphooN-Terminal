//! DXLink WebSocket client for tastytrade market data streaming.
//!
//! Protocol: JSON messages over WebSocket (wss://)
//! Handshake: SETUP → AUTH → CHANNEL_REQUEST → FEED_SETUP → FEED_SUBSCRIPTION
//! Data: COMPACT format (flat arrays with field ordering from FEED_SETUP)
//!
//! Reference: https://developer.tastytrade.com/streaming-market-data/

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio_tungstenite::{connect_async, tungstenite::Message};

const DXLINK_VERSION: &str = "0.1-DXF-JS/0.3.0";
const USER_AGENT: &str = "TyphooN-Terminal/1.0";

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
    pub time: i64, // epoch ms
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DxSnapshotStatus {
    Complete,
    Snipped,
    TimedOut,
}

#[derive(Debug, Clone)]
pub struct DxCandleFetch {
    pub candles: Vec<DxCandle>,
    pub status: DxSnapshotStatus,
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
pub async fn get_streaming_token(
    base_url: &str,
    session_token: &str,
) -> Result<DxLinkToken, String> {
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .unwrap_or_default();
    let url = format!("{}/api-quote-tokens", base_url);

    let resp = client
        .get(&url)
        .header("Accept", "application/json")
        .header("Authorization", session_token)
        .send()
        .await
        .map_err(|e| format!("Get streaming token failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        let clean = clean_http_error_body(&text);
        return Err(if clean.is_empty() {
            format!("Streaming token request failed at {url}: {status}")
        } else {
            format!("Streaming token request failed at {url}: {status} — {clean}")
        });
    }

    let data: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Parse streaming token failed: {e}"))?;

    parse_streaming_token_response(&data)
}

fn parse_streaming_token_response(data: &serde_json::Value) -> Result<DxLinkToken, String> {
    let token = data["data"]["token"]
        .as_str()
        .ok_or("No token in response")?
        .to_string();
    let ws_url = data["data"]["dxlink-url"]
        .as_str()
        .or_else(|| data["data"]["websocket-url"].as_str())
        .ok_or("No dxlink-url in response")?
        .to_string();

    Ok(DxLinkToken { token, url: ws_url })
}

fn clean_http_error_body(text: &str) -> String {
    let text = text.trim();
    if text.is_empty() {
        return String::new();
    }
    let clean = if text.contains('<') {
        text.lines()
            .map(str::trim)
            .filter(|line| !line.starts_with('<') && !line.ends_with('>'))
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        text.to_string()
    };
    clean.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Fetch historical candles via DXLink WebSocket.
/// Connects, authenticates, requests candles, collects results, disconnects.
pub async fn fetch_candles(
    dx_token: &DxLinkToken,
    symbol: &str,
    interval: &str, // "1d", "1h", "5m", etc.
    from_time_ms: i64,
) -> Result<Vec<DxCandle>, String> {
    Ok(
        fetch_candles_with_status(dx_token, symbol, interval, from_time_ms)
            .await?
            .candles,
    )
}

/// Fetch historical candles via DXLink WebSocket and return whether the server
/// exhausted the requested snapshot or snipped it because more history exists.
pub async fn fetch_candles_with_status(
    dx_token: &DxLinkToken,
    symbol: &str,
    interval: &str, // "1d", "1h", "5m", etc.
    from_time_ms: i64,
) -> Result<DxCandleFetch, String> {
    // Connect
    let (ws_stream, _) = connect_async(&dx_token.url)
        .await
        .map_err(|e| format!("DXLink connect failed: {e}"))?;
    let (mut sink, mut stream) = ws_stream.split();

    // Inline helpers (closures with async borrow don't work)
    macro_rules! dx_send {
        ($sink:expr, $msg:expr) => {
            $sink
                .send(Message::Text($msg.to_string().into()))
                .await
                .map_err(|e| format!("DXLink send failed: {e}"))?
        };
    }
    macro_rules! dx_recv {
        ($stream:expr) => {{
            loop {
                match $stream.next().await {
                    Some(Ok(Message::Text(txt))) => {
                        break serde_json::from_str::<serde_json::Value>(&txt)
                            .map_err(|e| format!("DXLink parse failed: {e}"));
                    }
                    Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) => continue,
                    Some(Ok(Message::Close(_))) => {
                        break Err("DXLink closed".into());
                    }
                    Some(Err(e)) => {
                        break Err(format!("DXLink error: {e}"));
                    }
                    None => {
                        break Err("DXLink ended".into());
                    }
                    _ => continue,
                }
            }
        }};
    }

    // Step 1: Send SETUP
    dx_send!(
        sink,
        serde_json::json!({
            "type": "SETUP",
            "channel": 0,
            "version": DXLINK_VERSION,
            "keepaliveTimeout": 60,
            "acceptKeepaliveTimeout": 60
        })
    );

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
    dx_send!(
        sink,
        serde_json::json!({
            "type": "AUTH",
            "channel": 0,
            "token": dx_token.token
        })
    );

    // Step 5: Wait for AUTH_STATE AUTHORIZED
    let msg = dx_recv!(stream)?;
    if msg["type"] != "AUTH_STATE" || msg["state"] != "AUTHORIZED" {
        return Err(format!("DXLink auth failed: {:?}", msg));
    }

    // Step 6: Open channel 1 for Candle data
    dx_send!(
        sink,
        serde_json::json!({
            "type": "CHANNEL_REQUEST",
            "channel": 1,
            "service": "FEED",
            "parameters": { "contract": "AUTO" }
        })
    );

    // Wait for CHANNEL_OPENED
    loop {
        let msg = dx_recv!(stream)?;
        if msg["type"] == "CHANNEL_OPENED" && msg["channel"] == 1 {
            break;
        }
        if msg["type"] == "ERROR" {
            return Err(format!("DXLink channel error: {}", msg["message"]));
        }
    }

    // Step 7: FEED_SETUP — request Candle fields
    let candle_fields = vec![
        "eventSymbol",
        "eventTime",
        "eventFlags",
        "index",
        "time",
        "sequence",
        "count",
        "volume",
        "vwap",
        "bidVolume",
        "askVolume",
        "impVolatility",
        "openInterest",
        "open",
        "high",
        "low",
        "close",
    ];
    dx_send!(
        sink,
        serde_json::json!({
            "type": "FEED_SETUP",
            "channel": 1,
            "acceptAggregationPeriod": 0,
            "acceptDataFormat": "COMPACT",
            "acceptEventFields": {
                "Candle": candle_fields
            }
        })
    );

    // Wait for FEED_CONFIG
    loop {
        let msg = dx_recv!(stream)?;
        if msg["type"] == "FEED_CONFIG" && msg["channel"] == 1 {
            break;
        }
        if msg["type"] == "KEEPALIVE" {
            continue;
        }
    }

    // Step 8: Subscribe to candles
    let candle_symbol = format!("{}{{={}}}", symbol, interval);
    dx_send!(
        sink,
        serde_json::json!({
            "type": "FEED_SUBSCRIPTION",
            "channel": 1,
            "add": [{
                "symbol": candle_symbol,
                "type": "Candle",
                "fromTime": from_time_ms
            }]
        })
    );

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

        if msg["type"] == "KEEPALIVE" {
            continue;
        }
        if msg["type"] != "FEED_DATA" || msg["channel"] != 1 {
            continue;
        }

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
                                    open,
                                    high,
                                    low,
                                    close,
                                    volume,
                                });
                            }

                            // Check for SNAPSHOT_END (0x8) or SNAPSHOT_SNIP (0x10)
                            if flags & 0x8 != 0 || flags & 0x10 != 0 {
                                // Snapshot complete/snipped — close and return
                                let _ = sink.send(Message::Close(None)).await;
                                candles.sort_by_key(|c| c.time);
                                candles.dedup_by_key(|c| c.time);
                                let status = if flags & 0x10 != 0 {
                                    DxSnapshotStatus::Snipped
                                } else {
                                    DxSnapshotStatus::Complete
                                };
                                return Ok(DxCandleFetch { candles, status });
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
    Ok(DxCandleFetch {
        candles,
        status: DxSnapshotStatus::TimedOut,
    })
}

/// Subscribe to real-time Quote events via a persistent DXLink WebSocket.
///
/// Returns a channel receiver that yields `DxQuote` structs as they arrive.
/// The connection stays open indefinitely; drop the receiver to stop.
pub async fn subscribe_quotes(
    dx_token: &DxLinkToken,
    symbols: Vec<String>,
) -> Result<tokio::sync::mpsc::Receiver<DxQuote>, String> {
    // Connect
    let (ws_stream, _) = connect_async(&dx_token.url)
        .await
        .map_err(|e| format!("DXLink connect failed: {e}"))?;
    let (mut sink, mut stream) = ws_stream.split();

    macro_rules! dx_send {
        ($sink:expr, $msg:expr) => {
            $sink
                .send(Message::Text($msg.to_string().into()))
                .await
                .map_err(|e| format!("DXLink send failed: {e}"))?
        };
    }
    macro_rules! dx_recv {
        ($stream:expr) => {{
            loop {
                match $stream.next().await {
                    Some(Ok(Message::Text(txt))) => {
                        break serde_json::from_str::<serde_json::Value>(&txt)
                            .map_err(|e| format!("DXLink parse failed: {e}"));
                    }
                    Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) => continue,
                    Some(Ok(Message::Close(_))) => {
                        break Err("DXLink closed".into());
                    }
                    Some(Err(e)) => {
                        break Err(format!("DXLink error: {e}"));
                    }
                    None => {
                        break Err("DXLink ended".into());
                    }
                    _ => continue,
                }
            }
        }};
    }

    // Step 1: SETUP
    dx_send!(
        sink,
        serde_json::json!({
            "type": "SETUP",
            "channel": 0,
            "version": DXLINK_VERSION,
            "keepaliveTimeout": 60,
            "acceptKeepaliveTimeout": 60
        })
    );

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

    // Step 4: AUTH
    dx_send!(
        sink,
        serde_json::json!({
            "type": "AUTH",
            "channel": 0,
            "token": dx_token.token
        })
    );

    // Step 5: Wait for AUTH_STATE AUTHORIZED
    let msg = dx_recv!(stream)?;
    if msg["type"] != "AUTH_STATE" || msg["state"] != "AUTHORIZED" {
        return Err(format!("DXLink auth failed: {:?}", msg));
    }

    // Step 6: Open channel 1 for Quote data
    dx_send!(
        sink,
        serde_json::json!({
            "type": "CHANNEL_REQUEST",
            "channel": 1,
            "service": "FEED",
            "parameters": { "contract": "AUTO" }
        })
    );

    loop {
        let msg = dx_recv!(stream)?;
        if msg["type"] == "CHANNEL_OPENED" && msg["channel"] == 1 {
            break;
        }
        if msg["type"] == "ERROR" {
            return Err(format!("DXLink channel error: {}", msg["message"]));
        }
    }

    // Step 7: FEED_SETUP — request Quote fields
    let quote_fields = vec!["eventSymbol", "bidPrice", "askPrice", "bidSize", "askSize"];
    let field_count = quote_fields.len();
    dx_send!(
        sink,
        serde_json::json!({
            "type": "FEED_SETUP",
            "channel": 1,
            "acceptAggregationPeriod": 0,
            "acceptDataFormat": "COMPACT",
            "acceptEventFields": {
                "Quote": quote_fields
            }
        })
    );

    // Wait for FEED_CONFIG
    loop {
        let msg = dx_recv!(stream)?;
        if msg["type"] == "FEED_CONFIG" && msg["channel"] == 1 {
            break;
        }
        if msg["type"] == "KEEPALIVE" {
            continue;
        }
    }

    // Step 8: Subscribe to quotes for all symbols
    let add_list: Vec<serde_json::Value> = symbols
        .iter()
        .map(|s| serde_json::json!({ "symbol": s, "type": "Quote" }))
        .collect();
    dx_send!(
        sink,
        serde_json::json!({
            "type": "FEED_SUBSCRIPTION",
            "channel": 1,
            "add": add_list
        })
    );

    // Step 9: Spawn reader task that streams quotes through channel
    let (tx, rx) = tokio::sync::mpsc::channel::<DxQuote>(256);

    tokio::spawn(async move {
        let _sink = sink; // keep sink alive so the connection stays open
        loop {
            let msg_val = match stream.next().await {
                Some(Ok(Message::Text(txt))) => {
                    match serde_json::from_str::<serde_json::Value>(&txt) {
                        Ok(v) => v,
                        Err(_) => continue,
                    }
                }
                Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) => continue,
                Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break,
                _ => continue,
            };

            if msg_val["type"] == "KEEPALIVE" {
                continue;
            }
            if msg_val["type"] != "FEED_DATA" || msg_val["channel"] != 1 {
                continue;
            }

            if let Some(data) = msg_val["data"].as_array() {
                let mut i = 0;
                while i < data.len() {
                    if data[i].as_str() == Some("Quote") {
                        i += 1;
                        if let Some(values) = data.get(i).and_then(|v| v.as_array()) {
                            let chunks = values.len() / field_count;
                            for c in 0..chunks {
                                let off = c * field_count;
                                let symbol = values[off].as_str().unwrap_or("").to_string();
                                let bid = parse_f64(&values[off + 1]);
                                let ask = parse_f64(&values[off + 2]);
                                let bid_size = parse_f64(&values[off + 3]);
                                let ask_size = parse_f64(&values[off + 4]);

                                if !symbol.is_empty() && !bid.is_nan() && !ask.is_nan() {
                                    let quote = DxQuote {
                                        symbol,
                                        bid,
                                        ask,
                                        bid_size,
                                        ask_size,
                                    };
                                    if tx.send(quote).await.is_err() {
                                        return; // receiver dropped
                                    }
                                }
                            }
                        }
                    }
                    i += 1;
                }
            }
        }
    });

    Ok(rx)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dxlink_token_default_construction() {
        let tok = DxLinkToken {
            token: String::new(),
            url: String::new(),
        };
        assert!(tok.token.is_empty());
        assert!(tok.url.is_empty());
    }

    #[test]
    fn dxlink_token_with_values() {
        let tok = DxLinkToken {
            token: "abc123".to_string(),
            url: "wss://example.com/feed".to_string(),
        };
        assert_eq!(tok.token, "abc123");
        assert_eq!(tok.url, "wss://example.com/feed");
    }

    #[test]
    fn parse_streaming_token_accepts_dxlink_url() {
        let json = serde_json::json!({
            "data": {
                "token": "tok",
                "dxlink-url": "wss://dxlink.example/realtime"
            }
        });

        let tok = parse_streaming_token_response(&json).unwrap();
        assert_eq!(tok.token, "tok");
        assert_eq!(tok.url, "wss://dxlink.example/realtime");
    }

    #[test]
    fn parse_streaming_token_accepts_legacy_websocket_url() {
        let json = serde_json::json!({
            "data": {
                "token": "tok",
                "websocket-url": "wss://dxlink.example/demo"
            }
        });

        let tok = parse_streaming_token_response(&json).unwrap();
        assert_eq!(tok.token, "tok");
        assert_eq!(tok.url, "wss://dxlink.example/demo");
    }

    #[test]
    fn clean_http_error_body_strips_html_noise() {
        let body = "<html>\n<body>\nquote token unavailable\n</body>\n</html>";
        assert_eq!(clean_http_error_body(body), "quote token unavailable");
    }

    #[test]
    fn dx_candle_fields_accessible() {
        let candle = DxCandle {
            symbol: "AAPL".to_string(),
            time: 1700000000000,
            open: 150.0,
            high: 155.0,
            low: 149.0,
            close: 153.0,
            volume: 1_000_000.0,
        };
        assert_eq!(candle.symbol, "AAPL");
        assert_eq!(candle.time, 1700000000000);
        assert!((candle.open - 150.0).abs() < f64::EPSILON);
        assert!((candle.high - 155.0).abs() < f64::EPSILON);
        assert!((candle.low - 149.0).abs() < f64::EPSILON);
        assert!((candle.close - 153.0).abs() < f64::EPSILON);
        assert!((candle.volume - 1_000_000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn dx_candle_clone() {
        let candle = DxCandle {
            symbol: "SPY".to_string(),
            time: 100,
            open: 1.0,
            high: 2.0,
            low: 0.5,
            close: 1.5,
            volume: 500.0,
        };
        let cloned = candle.clone();
        assert_eq!(cloned.symbol, "SPY");
        assert_eq!(cloned.time, 100);
    }

    #[test]
    fn dx_snapshot_status_distinguishes_complete_from_snipped() {
        assert_eq!(DxSnapshotStatus::Complete, DxSnapshotStatus::Complete);
        assert_ne!(DxSnapshotStatus::Complete, DxSnapshotStatus::Snipped);
        assert_ne!(DxSnapshotStatus::Snipped, DxSnapshotStatus::TimedOut);
    }

    #[test]
    fn dx_candle_fetch_carries_snapshot_status() {
        let fetch = DxCandleFetch {
            candles: vec![DxCandle {
                symbol: "SPY".to_string(),
                time: 100,
                open: 1.0,
                high: 2.0,
                low: 0.5,
                close: 1.5,
                volume: 500.0,
            }],
            status: DxSnapshotStatus::Snipped,
        };
        assert_eq!(fetch.candles.len(), 1);
        assert_eq!(fetch.status, DxSnapshotStatus::Snipped);
    }

    #[test]
    fn dx_quote_fields_accessible() {
        let quote = DxQuote {
            symbol: "TSLA".to_string(),
            bid: 240.50,
            ask: 240.75,
            bid_size: 100.0,
            ask_size: 200.0,
        };
        assert_eq!(quote.symbol, "TSLA");
        assert!((quote.bid - 240.50).abs() < f64::EPSILON);
        assert!((quote.ask - 240.75).abs() < f64::EPSILON);
        assert!((quote.bid_size - 100.0).abs() < f64::EPSILON);
        assert!((quote.ask_size - 200.0).abs() < f64::EPSILON);
    }

    #[test]
    fn dx_quote_clone() {
        let quote = DxQuote {
            symbol: "MSFT".to_string(),
            bid: 300.0,
            ask: 301.0,
            bid_size: 50.0,
            ask_size: 75.0,
        };
        let cloned = quote.clone();
        assert_eq!(cloned.symbol, "MSFT");
    }

    #[test]
    fn parse_f64_normal_number() {
        let v = serde_json::json!(42.5);
        assert!((parse_f64(&v) - 42.5).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_f64_integer() {
        let v = serde_json::json!(100);
        assert!((parse_f64(&v) - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_f64_negative() {
        let v = serde_json::json!(-3.14);
        assert!((parse_f64(&v) - (-3.14)).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_f64_nan_string() {
        let v = serde_json::json!("NaN");
        assert!(parse_f64(&v).is_nan());
    }

    #[test]
    fn parse_f64_infinity_string() {
        let v = serde_json::json!("Infinity");
        assert!(parse_f64(&v).is_infinite());
        assert!(parse_f64(&v).is_sign_positive());
    }

    #[test]
    fn parse_f64_neg_infinity_string() {
        let v = serde_json::json!("-Infinity");
        assert!(parse_f64(&v).is_infinite());
        assert!(parse_f64(&v).is_sign_negative());
    }

    #[test]
    fn parse_f64_numeric_string() {
        let v = serde_json::json!("99.9");
        assert!((parse_f64(&v) - 99.9).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_f64_invalid_string() {
        let v = serde_json::json!("not_a_number");
        assert!(parse_f64(&v).is_nan());
    }

    #[test]
    fn parse_f64_null() {
        let v = serde_json::json!(null);
        assert!(parse_f64(&v).is_nan());
    }

    #[test]
    fn parse_f64_bool() {
        let v = serde_json::json!(true);
        assert!(parse_f64(&v).is_nan());
    }
}
