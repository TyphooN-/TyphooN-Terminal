//! Crypto exchange data — Kraken public API for OHLCV klines.
//!
//! Kraken is used instead of Binance (geo-blocked in US/Canada).
//! No API key needed. No geo-restrictions.
//! History: BTC from 2013, ETH from 2016, most alts from 2017+.

/// Map TyphooN timeframes to Kraken interval (minutes).
fn to_kraken_interval(tf: &str) -> Option<u32> {
    match tf {
        "1Min" => Some(1),
        "5Min" => Some(5),
        "15Min" => Some(15),
        "30Min" => Some(30),
        "1Hour" => Some(60),
        "4Hour" => Some(240),
        "1Day" => Some(1440),
        "1Week" => Some(10080),
        // Kraken doesn't support 1Month — we'll aggregate from daily
        "1Month" => None,
        _ => None,
    }
}

/// Map TyphooN crypto symbols to Kraken trading pairs.
fn to_kraken_pair(sym: &str) -> Option<&'static str> {
    let clean = sym.replace("/", "").to_uppercase();
    match clean.as_str() {
        "BTCUSD" => Some("XBTUSD"),
        "ETHUSD" => Some("ETHUSD"),
        "SOLUSD" => Some("SOLUSD"),
        "DOGEUSD" => Some("XDGUSD"),
        "ADAUSD" => Some("ADAUSD"),
        "XRPUSD" => Some("XRPUSD"),
        "DOTUSD" => Some("DOTUSD"),
        "LINKUSD" => Some("LINKUSD"),
        "AVAXUSD" => Some("AVAXUSD"),
        "MATICUSD" | "POLUSD" => Some("MATICUSD"),
        "UNIUSD" => Some("UNIUSD"),
        "LTCUSD" => Some("LTCUSD"),
        "BCHUSD" => Some("BCHUSD"),
        "XLMUSD" => Some("XLMUSD"),
        "ATOMUSD" => Some("ATOMUSD"),
        "NEARUSD" => Some("NEARUSD"),
        "FILUSD" => Some("FILUSD"),
        "AAVEUSD" => Some("AAVEUSD"),
        "ALGOUSD" => Some("ALGOUSD"),
        "MANAUSD" => Some("MANAUSD"),
        "SANDUSD" => Some("SANDUSD"),
        "GRTUSD" => Some("GRTUSD"),
        "ICPUSD" => Some("ICPUSD"),
        "TRXUSD" => Some("TRXUSD"),
        "ETCUSD" => Some("ETCUSD"),
        "EOSUSD" => Some("EOSUSD"),
        "XTZUSD" => Some("XTZUSD"),
        "KAVAUSD" => Some("KAVAUSD"),
        "COMPUSD" => Some("COMPUSD"),
        "MKRUSD" => Some("MKRUSD"),
        "SNXUSD" => Some("SNXUSD"),
        "CRVUSD" => Some("CRVUSD"),
        "SUSHIUSD" => Some("SUSHIUSD"),
        "YFIUSD" => Some("YFIUSD"),
        "BATUSD" => Some("BATUSD"),
        "ZECUSD" => Some("ZECUSD"),
        "DASHUSD" => Some("DASHUSD"),
        "ENJUSD" => Some("ENJUSD"),
        "FTMUSD" => Some("FTMUSD"),
        "BNBUSD" => Some("BNBUSD"), // Kraken may not have BNB
        _ => None,
    }
}

/// Fetch OHLCV klines from Kraken public API.
/// Kraken returns max 720 bars per request. We paginate with `since` parameter.
pub async fn fetch_binance_klines(
    client: &reqwest::Client,
    symbol: &str,
    timeframe: &str,
    start_ms: i64,
    end_ms: i64,
) -> Result<Vec<serde_json::Value>, String> {
    // Handle 1Month by fetching daily and aggregating
    if timeframe == "1Month" {
        let daily = Box::pin(fetch_binance_klines(client, symbol, "1Day", start_ms, end_ms)).await?;
        return Ok(aggregate_to_monthly(&daily));
    }

    let interval = to_kraken_interval(timeframe)
        .ok_or_else(|| format!("Unsupported timeframe for Kraken: {}", timeframe))?;
    let kraken_pair = to_kraken_pair(symbol)
        .ok_or_else(|| format!("Unsupported symbol for Kraken: {}", symbol))?;

    let mut all_bars = Vec::new();
    let mut since = start_ms / 1000; // Kraken uses seconds

    loop {
        let url = format!(
            "https://api.kraken.com/0/public/OHLC?pair={}&interval={}&since={}",
            kraken_pair, interval, since
        );

        let resp = client.get(&url)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| format!("Kraken request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Kraken API error {}: {}", status, body));
        }

        let body: serde_json::Value = resp.json().await
            .map_err(|e| format!("Kraken JSON parse failed: {e}"))?;

        // Check for errors
        if let Some(errors) = body["error"].as_array() {
            if !errors.is_empty() {
                let err_msg = errors.iter()
                    .filter_map(|e| e.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                if !err_msg.is_empty() {
                    return Err(format!("Kraken error: {}", err_msg));
                }
            }
        }

        // Parse result — Kraken returns { "result": { "PAIR": [[...], ...], "last": N } }
        let result = &body["result"];
        let last = result["last"].as_i64().unwrap_or(0);

        // Find the data array (key varies by pair)
        let mut bars_in_page = Vec::new();
        for (key, val) in result.as_object().unwrap_or(&serde_json::Map::new()) {
            if key == "last" { continue; }
            if let Some(arr) = val.as_array() {
                for kline in arr {
                    if let Some(k) = kline.as_array() {
                        if k.len() < 7 { continue; }
                        let ts = k[0].as_i64().unwrap_or(0);
                        if ts == 0 { continue; }
                        let ts_ms = ts * 1000;
                        if ts_ms < start_ms || ts_ms > end_ms { continue; }

                        let dt = chrono::DateTime::from_timestamp(ts, 0).unwrap_or_default();
                        let open = k[1].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                        let high = k[2].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                        let low = k[3].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                        let close = k[4].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                        let volume = k[6].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);

                        if open > 0.0 {
                            bars_in_page.push(serde_json::json!({
                                "timestamp": dt.to_rfc3339(),
                                "open": open, "high": high, "low": low, "close": close, "volume": volume,
                            }));
                        }
                    }
                }
            }
        }

        let page_count = bars_in_page.len();
        all_bars.extend(bars_in_page);

        // Kraken: if `last` didn't advance or we got < 720 bars, we're done
        if last <= since || page_count < 700 {
            break;
        }
        since = last;

        // Rate limit: Kraken allows ~15 calls per minute for public endpoints
        // Use 4s between paginated calls to stay well under limit
        tokio::time::sleep(std::time::Duration::from_secs(4)).await;
    }

    // Sort by timestamp and deduplicate
    all_bars.sort_by(|a, b| {
        let ta = a["timestamp"].as_str().unwrap_or("");
        let tb = b["timestamp"].as_str().unwrap_or("");
        ta.cmp(tb)
    });
    all_bars.dedup_by(|a, b| a["timestamp"] == b["timestamp"]);

    Ok(all_bars)
}

/// Aggregate daily bars into monthly OHLCV.
fn aggregate_to_monthly(daily: &[serde_json::Value]) -> Vec<serde_json::Value> {
    let mut monthly: std::collections::BTreeMap<String, (f64, f64, f64, f64, f64, String)> = std::collections::BTreeMap::new();
    // key = "YYYY-MM", value = (open, high, low, close, volume, first_timestamp)

    for bar in daily {
        let ts = bar["timestamp"].as_str().unwrap_or("");
        if ts.len() < 7 { continue; }
        let month_key = ts[..7].to_string(); // "2024-06"
        let o = bar["open"].as_f64().unwrap_or(0.0);
        let h = bar["high"].as_f64().unwrap_or(0.0);
        let l = bar["low"].as_f64().unwrap_or(0.0);
        let c = bar["close"].as_f64().unwrap_or(0.0);
        let v = bar["volume"].as_f64().unwrap_or(0.0);

        let entry = monthly.entry(month_key).or_insert((o, h, l, c, 0.0, ts.to_string()));
        if h > entry.1 { entry.1 = h; }
        if l < entry.2 || entry.2 == 0.0 { entry.2 = l; }
        entry.3 = c; // close = last day's close
        entry.4 += v;
    }

    monthly.into_iter().map(|(_, (o, h, l, c, v, ts))| {
        serde_json::json!({
            "timestamp": ts,
            "open": o, "high": h, "low": l, "close": c, "volume": v,
        })
    }).collect()
}

/// Check if a symbol is supported by Kraken.
pub fn is_binance_supported(symbol: &str) -> bool {
    to_kraken_pair(symbol).is_some()
}

/// Get all supported crypto symbols from a list.
pub fn get_binance_crypto_symbols(symbols: &[String]) -> Vec<String> {
    symbols.iter()
        .filter(|s| is_binance_supported(s))
        .cloned()
        .collect()
}
