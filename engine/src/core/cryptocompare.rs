//! CryptoCompare deep crypto history — free, no API key, no geo-blocking.
//! Provides OHLCV data back to BTC genesis (2010) with 2000 bars per request.
//! Replaces Kraken OHLC as primary crypto backfill source.

/// Map TyphooN timeframes to CryptoCompare endpoints.
fn endpoint_for_tf(tf: &str) -> Option<(&'static str, u32)> {
    match tf {
        "1Min" => Some(("histominute", 1)),
        "5Min" => None, // CryptoCompare doesn't have 5min; use 1min and aggregate
        "15Min" => None, // same
        "30Min" => None,
        "1Hour" => Some(("histohour", 1)),
        "4Hour" => None, // aggregate from hourly
        "1Day" => Some(("histoday", 1)),
        "1Week" => None, // aggregate from daily
        "1Month" => None, // aggregate from daily
        _ => None,
    }
}

/// Fetch OHLCV from CryptoCompare with backward pagination.
/// Returns ALL available history from `start_ms` to `end_ms`.
pub async fn fetch_ohlcv(
    client: &reqwest::Client,
    symbol: &str,
    timeframe: &str,
    start_ms: i64,
    end_ms: i64,
) -> Result<Vec<serde_json::Value>, String> {
    // For timeframes without direct support, fetch base TF and aggregate
    let (endpoint, _multiplier) = match endpoint_for_tf(timeframe) {
        Some(e) => e,
        None => {
            // Aggregate: 4Hour from hourly, 1Week from daily, etc.
            match timeframe {
                "5Min" => {
                    let minutely = Box::pin(fetch_ohlcv(client, symbol, "1Min", start_ms, end_ms)).await?;
                    return Ok(aggregate_bars(&minutely, 5));
                }
                "15Min" => {
                    let minutely = Box::pin(fetch_ohlcv(client, symbol, "1Min", start_ms, end_ms)).await?;
                    return Ok(aggregate_bars(&minutely, 15));
                }
                "30Min" => {
                    let minutely = Box::pin(fetch_ohlcv(client, symbol, "1Min", start_ms, end_ms)).await?;
                    return Ok(aggregate_bars(&minutely, 30));
                }
                "4Hour" => {
                    let hourly = Box::pin(fetch_ohlcv(client, symbol, "1Hour", start_ms, end_ms)).await?;
                    return Ok(aggregate_bars(&hourly, 4));
                }
                "1Week" => {
                    let daily = Box::pin(fetch_ohlcv(client, symbol, "1Day", start_ms, end_ms)).await?;
                    return Ok(aggregate_bars(&daily, 7));
                }
                "1Month" => {
                    let daily = Box::pin(fetch_ohlcv(client, symbol, "1Day", start_ms, end_ms)).await?;
                    return Ok(aggregate_to_monthly(&daily));
                }
                _ => return Err(format!("Unsupported timeframe: {}", timeframe)),
            }
        }
    };

    // Normalize symbol: BTCUSD → BTC, SOL/USD → SOL
    let fsym = symbol.replace("/USD", "").replace("USD", "").to_uppercase();

    let mut all_bars = Vec::new();
    let mut to_ts = end_ms / 1000;
    let start_ts = start_ms / 1000;

    loop {
        let url = format!(
            "https://min-api.cryptocompare.com/data/v2/{}?fsym={}&tsym=USD&limit=2000&toTs={}",
            endpoint, fsym, to_ts
        );

        // Single attempt — on rate limit, abort immediately (use Kraken instead)
        let resp = client.get(&url)
            .timeout(std::time::Duration::from_secs(30))
            .send().await
            .map_err(|e| format!("CryptoCompare request failed: {e}"))?;

        if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err("CryptoCompare rate limited — use Kraken for recent data, try CryptoCompare later".into());
        }

        if !resp.status().is_success() {
            return Err(format!("CryptoCompare HTTP {}", resp.status()));
        }

        let body: serde_json::Value = resp.json().await
            .map_err(|e| format!("CryptoCompare JSON parse failed: {e}"))?;

        if body["Response"].as_str() != Some("Success") {
            let msg = body["Message"].as_str().unwrap_or("unknown");
            if msg.contains("rate limit") || msg.contains("upgrade") {
                return Err("CryptoCompare rate limited — use Kraken for recent data, try CryptoCompare later".into());
            }
            return Err(format!("CryptoCompare error: {msg}"));
        }

        // (Success already verified above)

        let data = body["Data"]["Data"].as_array()
            .ok_or_else(|| "No data array in response".to_string())?;

        let mut page_bars = Vec::new();
        let mut earliest_ts = i64::MAX;
        for bar in data {
            let ts = bar["time"].as_i64().unwrap_or(0);
            if ts == 0 { continue; }
            let open = bar["open"].as_f64().unwrap_or(0.0);
            if open <= 0.0 { continue; } // skip zero-price bars (before trading started)

            let ts_ms = ts * 1000;
            if ts_ms < start_ms || ts_ms > end_ms { continue; }

            let dt = chrono::DateTime::from_timestamp(ts, 0).unwrap_or_default();
            page_bars.push(serde_json::json!({
                "timestamp": dt.to_rfc3339(),
                "open": open,
                "high": bar["high"].as_f64().unwrap_or(0.0),
                "low": bar["low"].as_f64().unwrap_or(0.0),
                "close": bar["close"].as_f64().unwrap_or(0.0),
                "volume": bar["volumefrom"].as_f64().unwrap_or(0.0),
            }));

            if ts < earliest_ts { earliest_ts = ts; }
        }

        let page_count = page_bars.len();
        all_bars.extend(page_bars);

        // Stop if: no bars, reached start, or didn't go further back
        if page_count == 0 || earliest_ts <= start_ts || earliest_ts >= to_ts {
            break;
        }
        to_ts = earliest_ts;

        // Rate limit: CryptoCompare free tier allows ~30 calls/min
        tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
    }

    // Sort by timestamp ascending and deduplicate
    all_bars.sort_by(|a, b| a["timestamp"].as_str().unwrap_or("").cmp(b["timestamp"].as_str().unwrap_or("")));
    all_bars.dedup_by(|a, b| a["timestamp"] == b["timestamp"]);

    Ok(all_bars)
}

/// Aggregate N bars into 1 (e.g., 4 hourly bars → 1 4-hour bar).
/// The last chunk may be incomplete (fewer than `period` bars); this is fine —
/// it produces a shorter-period bar for the most recent partial window.
fn aggregate_bars(bars: &[serde_json::Value], period: usize) -> Vec<serde_json::Value> {
    bars.chunks(period).filter_map(|chunk| {
        let first = &chunk[0];
        let mut high = f64::MIN;
        let mut low = f64::MAX;
        let mut vol = 0.0_f64;
        let mut valid = false;
        for b in chunk {
            let h = b["high"].as_f64()?;
            let l = b["low"].as_f64()?;
            let v = b["volume"].as_f64().unwrap_or(0.0);
            if h > high { high = h; }
            if l < low { low = l; }
            vol += v;
            valid = true;
        }
        if !valid { return None; }
        let last = chunk.last().unwrap_or(first);
        Some(serde_json::json!({
            "timestamp": first["timestamp"],
            "open": first["open"],
            "high": high,
            "low": low,
            "close": last["close"],
            "volume": vol,
        }))
    }).collect()
}

/// Aggregate daily bars into monthly OHLCV.
fn aggregate_to_monthly(daily: &[serde_json::Value]) -> Vec<serde_json::Value> {
    let mut monthly: std::collections::BTreeMap<String, (f64, f64, f64, f64, f64, String)> = std::collections::BTreeMap::new();
    for bar in daily {
        let ts = bar["timestamp"].as_str().unwrap_or("");
        if ts.len() < 7 { continue; }
        let month_key = ts[..7].to_string();
        let o = bar["open"].as_f64().unwrap_or(0.0);
        let h = bar["high"].as_f64().unwrap_or(0.0);
        let l = bar["low"].as_f64().unwrap_or(0.0);
        let c = bar["close"].as_f64().unwrap_or(0.0);
        let v = bar["volume"].as_f64().unwrap_or(0.0);
        let entry = monthly.entry(month_key).or_insert((o, h, l, c, 0.0, ts.to_string()));
        if h > entry.1 { entry.1 = h; }
        if l < entry.2 || entry.2 == 0.0 { entry.2 = l; }
        entry.3 = c;
        entry.4 += v;
    }
    monthly.into_iter().map(|(_, (o, h, l, c, v, ts))| {
        serde_json::json!({"timestamp": ts, "open": o, "high": h, "low": l, "close": c, "volume": v})
    }).collect()
}

/// Check if a symbol is supported (all crypto pairs with USD are supported).
pub fn is_supported(symbol: &str) -> bool {
    let s = symbol.to_uppercase().replace('/', "");
    s.ends_with("USD") && s.len() >= 6
}
