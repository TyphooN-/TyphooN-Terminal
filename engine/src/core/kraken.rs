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
pub fn to_kraken_pair(sym: &str) -> Option<&'static str> {
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
        "XMRUSD" => Some("XXMRZUSD"),
        "ZECUSD" => Some("XZECZUSD"),
        "DASHUSD" => Some("DASHUSD"),
        "ENJUSD" => Some("ENJUSD"),
        "FTMUSD" => Some("FTMUSD"),
        "BNBUSD" => Some("BNBUSD"),
        "SHIBUSD" => Some("SHIBUSD"),
        "APEUSD" => Some("APEUSD"),
        "ARBUSD" => Some("ARBUSD"),
        "OPUSD" => Some("OPUSD"),
        "HBARUSD" => Some("HBARUSD"),
        "VETUSD" => Some("VETUSD"),
        "THETAUSD" => Some("THETAUSD"),
        "AXSUSD" => Some("AXSUSD"),
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
        let daily = Box::pin(fetch_binance_klines(
            client, symbol, "1Day", start_ms, end_ms,
        ))
        .await?;
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

        let resp = client
            .get(&url)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| format!("Kraken request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Kraken API error {}: {}", status, body));
        }

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Kraken JSON parse failed: {e}"))?;

        // Check for errors
        if let Some(errors) = body["error"].as_array() {
            if !errors.is_empty() {
                let err_msg = errors
                    .iter()
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
            if key == "last" {
                continue;
            }
            if let Some(arr) = val.as_array() {
                for kline in arr {
                    if let Some(k) = kline.as_array() {
                        // Kraken OHLCVT: [timestamp, open, high, low, close, vwap, volume, count]
                        if k.len() < 7 {
                            continue;
                        }
                        let ts = k[0].as_i64().unwrap_or(0);
                        if ts == 0 {
                            continue;
                        }
                        let ts_ms = ts * 1000;
                        if ts_ms < start_ms || ts_ms > end_ms {
                            continue;
                        }

                        let dt = chrono::DateTime::from_timestamp(ts, 0).unwrap_or_default();
                        let open = k[1]
                            .as_str()
                            .and_then(|s| s.parse::<f64>().ok())
                            .unwrap_or(0.0);
                        let high = k[2]
                            .as_str()
                            .and_then(|s| s.parse::<f64>().ok())
                            .unwrap_or(0.0);
                        let low = k[3]
                            .as_str()
                            .and_then(|s| s.parse::<f64>().ok())
                            .unwrap_or(0.0);
                        let close = k[4]
                            .as_str()
                            .and_then(|s| s.parse::<f64>().ok())
                            .unwrap_or(0.0);
                        let volume = k[6]
                            .as_str()
                            .and_then(|s| s.parse::<f64>().ok())
                            .unwrap_or(0.0);

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
    let mut monthly: std::collections::BTreeMap<String, (f64, f64, f64, f64, f64, String)> =
        std::collections::BTreeMap::new();
    // key = "YYYY-MM", value = (open, high, low, close, volume, first_timestamp)

    for bar in daily {
        let ts = bar["timestamp"].as_str().unwrap_or("");
        if ts.len() < 7 {
            continue;
        }
        let o = bar["open"].as_f64().unwrap_or(0.0);
        let h = bar["high"].as_f64().unwrap_or(0.0);
        let l = bar["low"].as_f64().unwrap_or(0.0);
        let c = bar["close"].as_f64().unwrap_or(0.0);
        let v = bar["volume"].as_f64().unwrap_or(0.0);
        // Skip bars with invalid prices
        if o <= 0.0 || h <= 0.0 || l <= 0.0 || c <= 0.0 || h < l {
            continue;
        }
        let month_key = ts[..7].to_string(); // "2024-06"

        let entry = monthly
            .entry(month_key)
            .or_insert((o, h, l, c, 0.0, ts.to_string()));
        if h > entry.1 {
            entry.1 = h;
        }
        if l < entry.2 {
            entry.2 = l;
        }
        entry.3 = c; // close = last day's close
        entry.4 += v;
    }

    monthly
        .into_iter()
        .map(|(_, (o, h, l, c, v, ts))| {
            serde_json::json!({
                "timestamp": ts,
                "open": o, "high": h, "low": l, "close": c, "volume": v,
            })
        })
        .collect()
}

/// Check if a symbol is supported by Kraken.
pub fn is_binance_supported(symbol: &str) -> bool {
    to_kraken_pair(symbol).is_some()
}

/// Get all supported crypto symbols from a list.
pub fn get_binance_crypto_symbols(symbols: &[String]) -> Vec<String> {
    symbols
        .iter()
        .filter(|s| is_binance_supported(s))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── to_kraken_interval ─────────────────────────────────

    #[test]
    fn kraken_interval_mapping() {
        assert_eq!(to_kraken_interval("1Min"), Some(1));
        assert_eq!(to_kraken_interval("5Min"), Some(5));
        assert_eq!(to_kraken_interval("15Min"), Some(15));
        assert_eq!(to_kraken_interval("30Min"), Some(30));
        assert_eq!(to_kraken_interval("1Hour"), Some(60));
        assert_eq!(to_kraken_interval("4Hour"), Some(240));
        assert_eq!(to_kraken_interval("1Day"), Some(1440));
        assert_eq!(to_kraken_interval("1Week"), Some(10080));
    }

    #[test]
    fn kraken_interval_unsupported() {
        assert_eq!(to_kraken_interval("1Month"), None);
        assert_eq!(to_kraken_interval(""), None);
        assert_eq!(to_kraken_interval("2Hour"), None);
        assert_eq!(to_kraken_interval("garbage"), None);
    }

    // ── to_kraken_pair ─────────────────────────────────────

    #[test]
    fn kraken_pair_major_cryptos() {
        assert_eq!(to_kraken_pair("BTCUSD"), Some("XBTUSD"));
        assert_eq!(to_kraken_pair("ETHUSD"), Some("ETHUSD"));
        assert_eq!(to_kraken_pair("SOLUSD"), Some("SOLUSD"));
        assert_eq!(to_kraken_pair("DOGEUSD"), Some("XDGUSD"));
        assert_eq!(to_kraken_pair("XRPUSD"), Some("XRPUSD"));
    }

    #[test]
    fn kraken_pair_with_slash() {
        assert_eq!(to_kraken_pair("BTC/USD"), Some("XBTUSD"));
        assert_eq!(to_kraken_pair("ETH/USD"), Some("ETHUSD"));
        assert_eq!(to_kraken_pair("DOGE/USD"), Some("XDGUSD"));
    }

    #[test]
    fn kraken_pair_case_insensitive() {
        // to_kraken_pair normalizes to uppercase internally
        assert_eq!(to_kraken_pair("btcusd"), Some("XBTUSD"));
        assert_eq!(to_kraken_pair("Ethusd"), Some("ETHUSD"));
    }

    #[test]
    fn kraken_pair_unsupported() {
        assert_eq!(to_kraken_pair("FAKEUSD"), None);
        assert_eq!(to_kraken_pair(""), None);
        assert_eq!(to_kraken_pair("BTCEUR"), None);
    }

    #[test]
    fn kraken_pair_matic_pol_alias() {
        assert_eq!(to_kraken_pair("MATICUSD"), Some("MATICUSD"));
        assert_eq!(to_kraken_pair("POLUSD"), Some("MATICUSD"));
    }

    // ── is_binance_supported / get_binance_crypto_symbols ──

    #[test]
    fn is_supported_known_pairs() {
        assert!(is_binance_supported("BTCUSD"));
        assert!(is_binance_supported("ETHUSD"));
        assert!(is_binance_supported("SOLUSD"));
        assert!(is_binance_supported("BTC/USD"));
    }

    #[test]
    fn is_not_supported_unknown() {
        assert!(!is_binance_supported("FAKEUSD"));
        assert!(!is_binance_supported(""));
    }

    #[test]
    fn get_supported_symbols_filters() {
        let syms = vec![
            "BTCUSD".to_string(),
            "ETHUSD".to_string(),
            "FAKEUSD".to_string(),
            "AAPL".to_string(),
        ];
        let result = get_binance_crypto_symbols(&syms);
        assert_eq!(result.len(), 2);
        assert!(result.contains(&"BTCUSD".to_string()));
        assert!(result.contains(&"ETHUSD".to_string()));
    }

    #[test]
    fn get_supported_symbols_empty() {
        let result = get_binance_crypto_symbols(&[]);
        assert!(result.is_empty());
    }

    // ── aggregate_to_monthly ───────────────────────────────

    fn make_bar(ts: &str, o: f64, h: f64, l: f64, c: f64, v: f64) -> serde_json::Value {
        json!({
            "timestamp": ts,
            "open": o,
            "high": h,
            "low": l,
            "close": c,
            "volume": v,
        })
    }

    #[test]
    fn monthly_aggregation_basic() {
        let daily = vec![
            make_bar(
                "2024-03-01T00:00:00Z",
                60000.0,
                62000.0,
                58000.0,
                61000.0,
                100.0,
            ),
            make_bar(
                "2024-03-15T00:00:00Z",
                61000.0,
                65000.0,
                59000.0,
                64000.0,
                200.0,
            ),
            make_bar(
                "2024-04-01T00:00:00Z",
                64000.0,
                70000.0,
                63000.0,
                68000.0,
                300.0,
            ),
        ];
        let result = aggregate_to_monthly(&daily);
        assert_eq!(result.len(), 2);

        // March
        let mar = &result[0];
        assert_eq!(mar["open"].as_f64().unwrap(), 60000.0);
        assert_eq!(mar["high"].as_f64().unwrap(), 65000.0);
        assert_eq!(mar["low"].as_f64().unwrap(), 58000.0);
        assert_eq!(mar["close"].as_f64().unwrap(), 64000.0);
        assert!((mar["volume"].as_f64().unwrap() - 300.0).abs() < 1e-10);

        // April
        let apr = &result[1];
        assert_eq!(apr["open"].as_f64().unwrap(), 64000.0);
    }

    #[test]
    fn monthly_aggregation_empty() {
        let result = aggregate_to_monthly(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn monthly_aggregation_short_timestamp_skipped() {
        let daily = vec![
            json!({"timestamp": "bad", "open": 1.0, "high": 2.0, "low": 0.5, "close": 1.5, "volume": 10.0}),
        ];
        let result = aggregate_to_monthly(&daily);
        assert!(result.is_empty());
    }

    #[test]
    fn monthly_aggregation_single_day() {
        let daily = vec![make_bar(
            "2024-06-15T00:00:00Z",
            50.0,
            55.0,
            45.0,
            52.0,
            100.0,
        )];
        let result = aggregate_to_monthly(&daily);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["open"].as_f64().unwrap(), 50.0);
        assert_eq!(result[0]["close"].as_f64().unwrap(), 52.0);
    }

    // ── OHLCV parsing from mock Kraken JSON ────────────────

    #[test]
    fn parse_mock_kraken_response() {
        // Simulate the structure Kraken returns
        let mock_response = json!({
            "error": [],
            "result": {
                "XBTUSD": [
                    [1704067200, "42000.0", "42500.0", "41500.0", "42200.0", "42100.0", "150.5", 1000],
                    [1704070800, "42200.0", "43000.0", "42000.0", "42800.0", "42500.0", "200.3", 1200],
                ],
                "last": 1704070800
            }
        });

        // Verify error array is empty
        let errors = mock_response["error"].as_array().unwrap();
        assert!(errors.is_empty());

        // Parse the OHLCV data the same way fetch_binance_klines does
        let result = &mock_response["result"];
        let mut parsed_bars = Vec::new();

        for (key, val) in result.as_object().unwrap() {
            if key == "last" {
                continue;
            }
            if let Some(arr) = val.as_array() {
                for kline in arr {
                    if let Some(k) = kline.as_array() {
                        if k.len() < 7 {
                            continue;
                        }
                        let ts = k[0].as_i64().unwrap_or(0);
                        let open = k[1]
                            .as_str()
                            .and_then(|s| s.parse::<f64>().ok())
                            .unwrap_or(0.0);
                        let high = k[2]
                            .as_str()
                            .and_then(|s| s.parse::<f64>().ok())
                            .unwrap_or(0.0);
                        let low = k[3]
                            .as_str()
                            .and_then(|s| s.parse::<f64>().ok())
                            .unwrap_or(0.0);
                        let close = k[4]
                            .as_str()
                            .and_then(|s| s.parse::<f64>().ok())
                            .unwrap_or(0.0);
                        let volume = k[6]
                            .as_str()
                            .and_then(|s| s.parse::<f64>().ok())
                            .unwrap_or(0.0);

                        if open > 0.0 {
                            parsed_bars.push(json!({
                                "timestamp": ts,
                                "open": open, "high": high, "low": low, "close": close, "volume": volume,
                            }));
                        }
                    }
                }
            }
        }

        assert_eq!(parsed_bars.len(), 2);
        assert_eq!(parsed_bars[0]["open"].as_f64().unwrap(), 42000.0);
        assert_eq!(parsed_bars[0]["high"].as_f64().unwrap(), 42500.0);
        assert_eq!(parsed_bars[0]["low"].as_f64().unwrap(), 41500.0);
        assert_eq!(parsed_bars[0]["close"].as_f64().unwrap(), 42200.0);
        assert_eq!(parsed_bars[0]["volume"].as_f64().unwrap(), 150.5);

        assert_eq!(parsed_bars[1]["open"].as_f64().unwrap(), 42200.0);
        assert_eq!(parsed_bars[1]["close"].as_f64().unwrap(), 42800.0);
    }

    #[test]
    fn parse_kraken_response_with_errors() {
        let mock_response = json!({
            "error": ["EGeneral:Too many requests"],
            "result": {}
        });
        let errors = mock_response["error"].as_array().unwrap();
        assert!(!errors.is_empty());
        let err_msg: Vec<&str> = errors.iter().filter_map(|e| e.as_str()).collect();
        assert_eq!(err_msg[0], "EGeneral:Too many requests");
    }

    #[test]
    fn parse_kraken_short_kline_skipped() {
        // Klines with fewer than 7 elements should be skipped
        let mock_response = json!({
            "error": [],
            "result": {
                "XBTUSD": [
                    [1704067200, "42000.0", "42500.0"], // too short
                    [1704070800, "42200.0", "43000.0", "42000.0", "42800.0", "42500.0", "200.3", 1200], // valid
                ],
                "last": 1704070800
            }
        });

        let result = &mock_response["result"];
        let mut count = 0;
        for (key, val) in result.as_object().unwrap() {
            if key == "last" {
                continue;
            }
            if let Some(arr) = val.as_array() {
                for kline in arr {
                    if let Some(k) = kline.as_array() {
                        if k.len() < 7 {
                            continue;
                        }
                        count += 1;
                    }
                }
            }
        }
        assert_eq!(count, 1, "should skip short klines");
    }

    #[test]
    fn parse_kraken_zero_open_skipped() {
        // Bars with open=0 should be skipped
        let mock_response = json!({
            "error": [],
            "result": {
                "XBTUSD": [
                    [1704067200, "0.0", "42500.0", "41500.0", "42200.0", "42100.0", "150.5", 1000],
                ],
                "last": 1704067200
            }
        });

        let result = &mock_response["result"];
        let mut parsed = Vec::new();
        for (key, val) in result.as_object().unwrap() {
            if key == "last" {
                continue;
            }
            if let Some(arr) = val.as_array() {
                for kline in arr {
                    if let Some(k) = kline.as_array() {
                        if k.len() < 7 {
                            continue;
                        }
                        let open = k[1]
                            .as_str()
                            .and_then(|s| s.parse::<f64>().ok())
                            .unwrap_or(0.0);
                        if open > 0.0 {
                            parsed.push(open);
                        }
                    }
                }
            }
        }
        assert!(parsed.is_empty(), "zero-open bars should be skipped");
    }

    #[test]
    fn parse_kraken_empty_result() {
        let mock_response = json!({
            "error": [],
            "result": {
                "XBTUSD": [],
                "last": 0
            }
        });
        let result = &mock_response["result"];
        let mut count = 0;
        for (key, val) in result.as_object().unwrap() {
            if key == "last" {
                continue;
            }
            if let Some(arr) = val.as_array() {
                count += arr.len();
            }
        }
        assert_eq!(count, 0);
    }
}
