//! Binance public API — free OHLCV klines for crypto weekend gap-fill.
//!
//! No API key needed. 1200 req/min rate limit.
//! Fetches klines and returns bars in TyphooN-Terminal's standard JSON format.

use serde::{Deserialize, Serialize};

/// Map TyphooN timeframes to Binance interval strings.
fn to_binance_interval(tf: &str) -> Option<&'static str> {
    match tf {
        "1Min" => Some("1m"),
        "5Min" => Some("5m"),
        "15Min" => Some("15m"),
        "30Min" => Some("30m"),
        "1Hour" => Some("1h"),
        "4Hour" => Some("4h"),
        "1Day" => Some("1d"),
        "1Week" => Some("1w"),
        "1Month" => Some("1M"),
        _ => None,
    }
}

/// Map TyphooN crypto symbols to Binance trading pairs.
/// TyphooN uses "BTC/USD" or "BTCUSD", Binance uses "BTCUSDT".
fn to_binance_symbol(sym: &str) -> String {
    let clean = sym.replace("/", "");
    // Map USD -> USDT for Binance (most liquid pairs)
    if clean.ends_with("USD") && !clean.ends_with("USDT") {
        format!("{}T", clean)
    } else {
        clean
    }
}

/// Binance kline response: array of arrays.
/// [open_time, open, high, low, close, volume, close_time, ...]
#[derive(Debug, Deserialize)]
struct BinanceKline(
    i64,    // 0: open time (ms)
    String, // 1: open
    String, // 2: high
    String, // 3: low
    String, // 4: close
    String, // 5: volume
    i64,    // 6: close time (ms)
    String, // 7: quote asset volume
    i64,    // 8: number of trades
    String, // 9: taker buy base
    String, // 10: taker buy quote
    String, // 11: ignore
);

/// Fetch klines from Binance public API.
/// Returns bars in TyphooN JSON format: [{timestamp, open, high, low, close, volume}, ...]
pub async fn fetch_binance_klines(
    client: &reqwest::Client,
    symbol: &str,
    timeframe: &str,
    start_ms: i64,
    end_ms: i64,
) -> Result<Vec<serde_json::Value>, String> {
    let interval = to_binance_interval(timeframe)
        .ok_or_else(|| format!("Unsupported timeframe for Binance: {}", timeframe))?;
    let binance_sym = to_binance_symbol(symbol);

    let mut all_bars = Vec::new();
    let mut cursor = start_ms;
    let limit = 1000; // Binance max per request

    loop {
        if cursor >= end_ms { break; }

        let url = format!(
            "https://api.binance.com/api/v3/klines?symbol={}&interval={}&startTime={}&endTime={}&limit={}",
            binance_sym, interval, cursor, end_ms, limit
        );

        let resp = client.get(&url)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| format!("Binance request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Binance API error {}: {}", status, body));
        }

        let klines: Vec<Vec<serde_json::Value>> = resp.json().await
            .map_err(|e| format!("Binance JSON parse failed: {e}"))?;

        if klines.is_empty() { break; }

        for k in &klines {
            if k.len() < 7 { continue; }
            let open_time_ms = k[0].as_i64().unwrap_or(0);
            let dt = chrono::DateTime::from_timestamp_millis(open_time_ms).unwrap_or_default();

            all_bars.push(serde_json::json!({
                "timestamp": dt.to_rfc3339(),
                "open": k[1].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0),
                "high": k[2].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0),
                "low": k[3].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0),
                "close": k[4].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0),
                "volume": k[5].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0),
            }));
        }

        // Advance cursor past the last kline's close time
        let last_close = klines.last()
            .and_then(|k| k.get(6))
            .and_then(|v| v.as_i64())
            .unwrap_or(end_ms);
        cursor = last_close + 1;

        // Rate limiting: brief pause between pages
        if klines.len() == limit as usize {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        } else {
            break; // got fewer than limit = last page
        }
    }

    Ok(all_bars)
}

/// Check if a timestamp falls on a weekend (Saturday or Sunday UTC).
pub fn is_weekend(ts_ms: i64) -> bool {
    if let Some(dt) = chrono::DateTime::from_timestamp_millis(ts_ms) {
        let weekday = dt.format("%u").to_string().parse::<u32>().unwrap_or(0);
        weekday >= 6 // 6=Saturday, 7=Sunday
    } else {
        false
    }
}

/// Filter bars to only include weekend bars (Saturday + Sunday).
pub fn filter_weekend_bars(bars: &[serde_json::Value]) -> Vec<serde_json::Value> {
    bars.iter().filter(|bar| {
        let ts = bar["timestamp"].as_str().unwrap_or("");
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts) {
            let weekday = dt.format("%u").to_string().parse::<u32>().unwrap_or(0);
            weekday >= 6
        } else {
            false
        }
    }).cloned().collect()
}

/// Supported Binance crypto pairs (maps TyphooN symbol -> Binance has data).
pub fn is_binance_supported(symbol: &str) -> bool {
    let clean = symbol.replace("/", "").to_uppercase();
    matches!(clean.as_str(),
        "BTCUSD" | "ETHUSD" | "SOLUSD" | "DOGEUSD" | "ADAUSD" | "XRPUSD" |
        "BNBUSD" | "AVAXUSD" | "DOTUSD" | "LINKUSD" | "MATICUSD" | "UNIUSD" |
        "LTCUSD" | "BCHUSD" | "XLMUSD" | "ATOMUSD" | "NEARUSD" | "FILUSD" |
        "AAVEUSD" | "ALGOUSD" | "MANAUSD" | "SANDUSD" | "AXSUSD" | "GALAUSD" |
        "FTMUSD" | "RUNEUSD" | "ENJUSD" | "BATUSD" | "ZECUSD" | "DASHUSD" |
        "COMPUSD" | "MKRUSD" | "SNXUSD" | "CRVUSD" | "SUSHIUSD" | "YFIUSD" |
        "GRTUSD" | "ICPUSD" | "THETAUSD" | "VETUSD" | "EGLDUSD" | "HBARUSD" |
        "TRXUSD" | "ETCUSD" | "EOSUSD" | "XTZUSD" | "NEOUSD" | "KAVAUSD" |
        // Darwinex-specific crypto tickers (no slash)
        "BTCUSD" | "ETHUSD" | "BNBUSD" | "SOLUSD"
    )
}

/// Get all supported crypto symbols from a list.
pub fn get_binance_crypto_symbols(symbols: &[String]) -> Vec<String> {
    symbols.iter()
        .filter(|s| is_binance_supported(s))
        .cloned()
        .collect()
}
