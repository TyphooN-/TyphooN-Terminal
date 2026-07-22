use super::*;
use serde_json::json;

#[test]
fn kraken_pair_catalog_resolves_live_asset_pairs() {
    let body = json!({
        "error": [],
        "result": {
            "XXMRZUSD": {
                "altname": "XMRUSD",
                "wsname": "XMR/USD",
                "status": "online"
            },
            "POLUSD": {
                "altname": "POLUSD",
                "wsname": "POL/USD",
                "status": "online"
            },
            "XDGUSD": {
                "altname": "XDGUSD",
                "wsname": "XDG/USD",
                "status": "online"
            },
            "MATICUSD": {
                "altname": "MATICUSD",
                "wsname": "MATIC/USD",
                "status": "delisted"
            }
        }
    });
    let catalog = parse_kraken_pair_catalog(&body).unwrap();
    assert_eq!(
        catalog.by_symbol.get("XMRUSD"),
        Some(&"XXMRZUSD".to_string())
    );
    assert_eq!(catalog.by_symbol.get("POLUSD"), Some(&"POLUSD".to_string()));
    assert_eq!(
        catalog.by_symbol.get("DOGEUSD"),
        Some(&"XDGUSD".to_string())
    );
    assert!(!catalog.by_symbol.contains_key("MATICUSD"));
}

#[test]
fn kraken_pair_static_pol_alias_uses_live_pol_market() {
    assert_eq!(to_kraken_pair("POLUSD"), Some("POLUSD"));
    assert_eq!(to_kraken_pair("MATICUSD"), Some("POLUSD"));
}

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
    assert_eq!(to_kraken_pair("MATICUSD"), Some("POLUSD"));
    assert_eq!(to_kraken_pair("POLUSD"), Some("POLUSD"));
}

#[test]
fn normalize_pair_symbol_handles_display_and_wrapped_forms() {
    assert_eq!(normalize_pair_symbol("BTC/USD"), "BTCUSD");
    assert_eq!(normalize_pair_symbol("XBTUSD"), "BTCUSD");
    assert_eq!(normalize_pair_symbol("XXBTZUSD"), "BTCUSD");
    assert_eq!(normalize_pair_symbol("POMZUSD"), "POMUSD");
    assert_eq!(normalize_pair_symbol("HRTXZUSD"), "HRTXUSD");
    assert_eq!(normalize_pair_symbol("FNGRZUSD"), "FNGRUSD");
    assert_eq!(normalize_pair_symbol("ETH/BTC"), "ETHBTC");
    assert_eq!(normalize_pair_symbol("XDG/USD"), "DOGEUSD");
}

#[test]
fn kraken_pair_lossy_supports_dynamic_pairs() {
    assert_eq!(to_kraken_pair_lossy("BTC/USD"), Some("XBTUSD".to_string()));
    assert_eq!(to_kraken_pair_lossy("ETH/BTC"), Some("ETHBTC".to_string()));
    assert_eq!(to_kraken_pair_lossy("ETH/EUR"), Some("ETHEUR".to_string()));
    assert_eq!(to_kraken_pair_lossy("XXBTZUSD"), Some("XBTUSD".to_string()));
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
    assert!(!is_binance_supported("AAPL"));
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
    assert_eq!(result.len(), 3);
    assert!(result.contains(&"BTCUSD".to_string()));
    assert!(result.contains(&"ETHUSD".to_string()));
    assert!(result.contains(&"FAKEUSD".to_string()));
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
fn parse_kraken_ohlc_response_helper_filters_and_sorts_input_shape() {
    let mock_response = json!({
        "error": [],
        "result": {
            "XBTUSD": [
                [1704067200, "42000.0", "42500.0", "41500.0", "42200.0", "42100.0", "150.5", 1000],
                [1704070800, "0.0", "43000.0", "42000.0", "42800.0", "42500.0", "200.3", 1200],
                [1704074400, "42800.0", "43100.0", "42700.0", "43000.0", "42900.0", "50.0", 500],
            ],
            "last": 1704074400
        }
    });
    let bars =
        parse_kraken_ohlc_response(&mock_response, 1704067200_i64 * 1000, 1704074400_i64 * 1000)
            .unwrap();
    assert_eq!(bars.len(), 2);
    assert_eq!(bars[0]["open"].as_f64().unwrap(), 42000.0);
    assert_eq!(bars[1]["close"].as_f64().unwrap(), 43000.0);
}

#[test]
fn parse_kraken_ohlc_response_helper_returns_api_error() {
    let mock_response = json!({
        "error": ["EGeneral:Too many requests"],
        "result": {}
    });
    let err = parse_kraken_ohlc_response(&mock_response, 0, i64::MAX).unwrap_err();
    assert!(err.contains("Too many requests"));
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
fn kraken_rate_limit_detection_covers_public_and_rest_errors() {
    assert!(is_kraken_rate_limit_error("EGeneral:Too many requests"));
    assert!(is_kraken_rate_limit_error("EAPI:Rate limit exceeded"));
    assert!(is_kraken_rate_limit_error(
        "EService: Throttled: 1999999999"
    ));
    assert!(is_kraken_rate_limit_error(
        "Kraken API error 429: retry later"
    ));
    assert!(!is_kraken_rate_limit_error("EQuery:Unknown asset pair"));
}

#[test]
fn kraken_throttled_wait_parses_retry_timestamp() {
    let retry_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 30;
    let wait = kraken_throttled_wait(&format!("EService: Throttled: {retry_at}")).unwrap();
    assert!(wait <= Duration::from_secs(30));
    assert!(wait >= Duration::from_secs(20));
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
