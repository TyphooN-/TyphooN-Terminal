use super::*;
use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::json;

// ── parse_f64_field ─────────────────────────────────────────────

#[test]
fn parse_f64_field_from_string() {
    let j = json!({"equity": "123456.78"});
    assert!((parse_f64_field(&j, "equity") - 123456.78).abs() < 1e-10);
}

#[test]
fn parse_f64_field_from_number() {
    let j = json!({"equity": 99999.50});
    assert!((parse_f64_field(&j, "equity") - 99999.50).abs() < 1e-10);
}

#[test]
fn parse_f64_field_null_returns_zero() {
    let j = json!({"equity": null});
    assert_eq!(parse_f64_field(&j, "equity"), 0.0);
}

#[test]
fn parse_f64_field_missing_returns_zero() {
    let j = json!({});
    assert_eq!(parse_f64_field(&j, "equity"), 0.0);
}

#[test]
fn parse_f64_field_bad_string_returns_zero() {
    let j = json!({"equity": "not_a_number"});
    assert_eq!(parse_f64_field(&j, "equity"), 0.0);
}

// ── format_order_price ─────────────────────────────────────────────────

#[test]
fn round_price_stock_above_one() {
    assert_eq!(format_order_price(15.6789), "15.68");
    assert_eq!(format_order_price(100.0), "100.00");
    assert_eq!(format_order_price(1.0), "1.00");
}

#[test]
fn round_price_penny_stock() {
    assert_eq!(format_order_price(0.1234), "0.1234");
    assert_eq!(format_order_price(0.01), "0.0100");
    assert_eq!(format_order_price(0.99), "0.9900");
}

#[test]
fn round_price_sub_penny_crypto() {
    assert_eq!(format_order_price(0.00123456), "0.00123456");
    assert_eq!(format_order_price(0.009), "0.00900000");
}

#[tokio::test]
async fn observe_rate_limit_headers_updates_bar_rpm() {
    let limiter = RateLimiter::new();
    let mut headers = HeaderMap::new();
    headers.insert("x-ratelimit-limit", HeaderValue::from_static("10000"));

    assert_eq!(
        limiter.observe_rate_limit_headers(&headers).await,
        Some(10000)
    );
    assert_eq!(limiter.requests_per_minute(), 10000);
}

// ── is_crypto detection (symbol.contains('/')) ──────────────────

#[test]
fn crypto_detection_by_slash() {
    assert!("BTC/USD".contains('/'));
    assert!("SOL/USD".contains('/'));
    assert!(!"AAPL".contains('/'));
    assert!(!"SPY".contains('/'));
}

// ── parse_option_symbol ─────────────────────────────────────────

#[test]
fn parse_option_symbol_call() {
    let (strike, opt_type, expiry) = AlpacaBroker::parse_option_symbol("AAPL240119C00150000");
    assert!((strike - 150.0).abs() < 1e-10);
    assert_eq!(opt_type, "call");
    assert_eq!(expiry, "2024-01-19");
}

#[test]
fn parse_option_symbol_put() {
    let (strike, opt_type, expiry) = AlpacaBroker::parse_option_symbol("TSLA250221P00200000");
    assert!((strike - 200.0).abs() < 1e-10);
    assert_eq!(opt_type, "put");
    assert_eq!(expiry, "2025-02-21");
}

#[test]
fn parse_option_symbol_fractional_strike() {
    // Strike 72.50 = 00072500
    let (strike, opt_type, _) = AlpacaBroker::parse_option_symbol("INTC240315C00072500");
    assert!((strike - 72.5).abs() < 1e-10);
    assert_eq!(opt_type, "call");
}

#[test]
fn parse_option_symbol_too_short() {
    let (strike, opt_type, expiry) = AlpacaBroker::parse_option_symbol("SHORT");
    assert_eq!(strike, 0.0);
    assert_eq!(opt_type, "unknown");
    assert!(expiry.is_empty());
}

#[test]
fn targeted_lookback_is_wider_than_incremental_for_equity_minute_sync() {
    let incremental =
        lookback_days_for_request(false, "1Min", 50_000, BarsLookbackMode::Incremental);
    let targeted = lookback_days_for_request(false, "1Min", 50_000, BarsLookbackMode::Targeted);
    assert_eq!(incremental, 7);
    assert!(targeted > incremental);
}

#[test]
fn targeted_lookback_scales_for_equity_hour_sync() {
    let targeted = lookback_days_for_request(false, "1Hour", 30_000, BarsLookbackMode::Targeted);
    assert!(targeted >= 6_000);
}

#[test]
fn detects_sip_bar_entitlement_failures() {
    assert!(AlpacaBroker::is_sip_bar_entitlement_failure(
        reqwest::StatusCode::FORBIDDEN,
        "subscription does not permit querying SIP data"
    ));
    assert!(AlpacaBroker::is_sip_bar_entitlement_failure(
        reqwest::StatusCode::UNPROCESSABLE_ENTITY,
        "SIP feed requires plan upgrade"
    ));
}

#[test]
fn ignores_non_entitlement_bar_failures() {
    assert!(!AlpacaBroker::is_sip_bar_entitlement_failure(
        reqwest::StatusCode::NOT_FOUND,
        "not found"
    ));
    assert!(!AlpacaBroker::is_sip_bar_entitlement_failure(
        reqwest::StatusCode::FORBIDDEN,
        "market data temporarily unavailable"
    ));
    assert!(!AlpacaBroker::is_sip_bar_entitlement_failure(
        reqwest::StatusCode::FORBIDDEN,
        "subscription does not permit querying IEX data"
    ));
}

// ── parse_bars (mock JSON) ──────────────────────────────────────

#[test]
fn parse_stock_bars_by_symbol_batch_valid() {
    let json = json!({
        "bars": {
            "AAPL": [{"t":"2024-01-02T00:00:00Z","o":100.0,"h":110.0,"l":99.0,"c":105.0,"v":1000.0}],
            "MSFT": [{"t":"2024-01-02T00:00:00Z","o":200.0,"h":220.0,"l":190.0,"c":210.0,"v":2000.0}]
        }
    });
    let symbols = vec![
        "AAPL".to_string(),
        "MSFT".to_string(),
        "MISSING".to_string(),
    ];
    let bars = AlpacaBroker::parse_stock_bars_by_symbol(&json, &symbols);
    assert_eq!(bars["AAPL"].len(), 1);
    assert_eq!(bars["AAPL"][0].close, 105.0);
    assert_eq!(bars["MSFT"].len(), 1);
    assert!(!bars.contains_key("MISSING"));
}

#[test]
fn parse_bars_stock_valid() {
    let json = json!({
        "bars": [
            {"t": "2024-01-02T05:00:00Z", "o": 100.0, "h": 105.0, "l": 99.0, "c": 103.0, "v": 50000.0},
            {"t": "2024-01-03T05:00:00Z", "o": 103.0, "h": 107.0, "l": 102.0, "c": 106.0, "v": 60000.0},
        ]
    });
    let bars = AlpacaBroker::parse_bars(&json, "AAPL", false);
    assert_eq!(bars.len(), 2);
    assert_eq!(bars[0].open, 100.0);
    assert_eq!(bars[0].high, 105.0);
    assert_eq!(bars[0].low, 99.0);
    assert_eq!(bars[0].close, 103.0);
    assert_eq!(bars[0].volume, 50000.0);
}

#[test]
fn parse_bars_crypto_nested_by_symbol() {
    let json = json!({
        "bars": {
            "BTC/USD": [
                {"t": "2024-01-02T00:00:00Z", "o": 42000.0, "h": 43000.0, "l": 41000.0, "c": 42500.0, "v": 100.0},
            ]
        }
    });
    let bars = AlpacaBroker::parse_bars(&json, "BTC/USD", true);
    assert_eq!(bars.len(), 1);
    assert_eq!(bars[0].open, 42000.0);
}

#[test]
fn parse_bars_rejects_zero_open() {
    let json = json!({
        "bars": [
            {"t": "2024-01-02T05:00:00Z", "o": 0.0, "h": 5.0, "l": 0.0, "c": 4.0, "v": 100.0},
        ]
    });
    let bars = AlpacaBroker::parse_bars(&json, "BAD", false);
    assert_eq!(bars.len(), 0);
}

#[test]
fn parse_bars_rejects_missing_timestamp() {
    let json = json!({
        "bars": [
            {"t": "", "o": 10.0, "h": 12.0, "l": 9.0, "c": 11.0, "v": 100.0},
        ]
    });
    let bars = AlpacaBroker::parse_bars(&json, "X", false);
    assert_eq!(bars.len(), 0);
}

#[test]
fn parse_bars_fixes_ohlc_consistency() {
    // h < o should be corrected: true_high = max(o,h,l,c)
    let json = json!({
        "bars": [
            {"t": "2024-01-02T05:00:00Z", "o": 110.0, "h": 105.0, "l": 99.0, "c": 108.0, "v": 100.0},
        ]
    });
    let bars = AlpacaBroker::parse_bars(&json, "FIX", false);
    assert_eq!(bars.len(), 1);
    assert_eq!(bars[0].high, 110.0); // corrected to max(110, 105, 99, 108)
    assert_eq!(bars[0].low, 99.0);
}

#[test]
fn parse_bars_empty_array() {
    let json = json!({"bars": []});
    let bars = AlpacaBroker::parse_bars(&json, "EMPTY", false);
    assert!(bars.is_empty());
}

// ── parse_order_info (mock JSON) ────────────────────────────────

#[test]
fn parse_order_info_full() {
    let j = json!({
        "id": "abc-123",
        "symbol": "AAPL",
        "qty": "10",
        "filled_qty": "10",
        "side": "buy",
        "type": "limit",
        "order_class": "bracket",
        "status": "filled",
        "limit_price": "150.00",
        "stop_price": null,
        "trail_price": null,
        "trail_percent": null,
        "created_at": "2024-01-02T10:00:00Z",
        "filled_at": "2024-01-02T10:00:05Z",
        "filled_avg_price": "149.98",
        "legs": null,
    });
    let order = AlpacaBroker::parse_order_info(&j);
    assert_eq!(order.id, "abc-123");
    assert_eq!(order.symbol, "AAPL");
    assert_eq!(order.qty, "10");
    assert_eq!(order.side, "buy");
    assert_eq!(order.order_type, "limit");
    assert_eq!(order.order_class, Some("bracket".to_string()));
    assert_eq!(order.status, "filled");
    assert_eq!(order.limit_price, Some("150.00".to_string()));
    assert_eq!(order.filled_avg_price, Some("149.98".to_string()));
}

#[test]
fn parse_order_info_with_bracket_legs() {
    let j = json!({
        "id": "parent-1",
        "symbol": "SPY",
        "qty": "5",
        "filled_qty": "5",
        "side": "buy",
        "type": "market",
        "order_class": "bracket",
        "status": "filled",
        "limit_price": null,
        "stop_price": null,
        "trail_price": null,
        "trail_percent": null,
        "created_at": "2024-01-02T10:00:00Z",
        "filled_at": "2024-01-02T10:00:01Z",
        "filled_avg_price": "470.00",
        "legs": [
            {
                "id": "tp-leg",
                "symbol": "SPY",
                "qty": "5",
                "filled_qty": "0",
                "side": "sell",
                "type": "limit",
                "status": "new",
                "limit_price": "480.00",
                "stop_price": null,
                "trail_price": null,
                "trail_percent": null,
                "created_at": "2024-01-02T10:00:00Z",
                "filled_at": null,
                "filled_avg_price": null,
            },
            {
                "id": "sl-leg",
                "symbol": "SPY",
                "qty": "5",
                "filled_qty": "0",
                "side": "sell",
                "type": "stop",
                "status": "held",
                "limit_price": null,
                "stop_price": "460.00",
                "trail_price": null,
                "trail_percent": null,
                "created_at": "2024-01-02T10:00:00Z",
                "filled_at": null,
                "filled_avg_price": null,
            },
        ],
    });
    let order = AlpacaBroker::parse_order_info(&j);
    assert_eq!(order.id, "parent-1");
    let legs = order.legs.expect("should have legs");
    assert_eq!(legs.len(), 2);
    assert_eq!(legs[0].id, "tp-leg");
    assert_eq!(legs[0].limit_price, Some("480.00".to_string()));
    assert_eq!(legs[1].id, "sl-leg");
    assert_eq!(legs[1].stop_price, Some("460.00".to_string()));
}

#[test]
fn collect_cancellable_order_ids_for_symbol_skips_filled_parent_and_keeps_open_legs() {
    let parent = OrderInfo {
        id: "parent-1".to_string(),
        symbol: "SPY".to_string(),
        qty: "5".to_string(),
        filled_qty: "5".to_string(),
        side: "buy".to_string(),
        order_type: "market".to_string(),
        order_class: Some("bracket".to_string()),
        status: "filled".to_string(),
        limit_price: None,
        stop_price: None,
        trail_price: None,
        trail_percent: None,
        created_at: "2024-01-02T10:00:00Z".to_string(),
        filled_at: Some("2024-01-02T10:00:01Z".to_string()),
        filled_avg_price: Some("470.00".to_string()),
        legs: Some(vec![
            OrderInfo {
                id: "tp-leg".to_string(),
                symbol: "SPY".to_string(),
                qty: "5".to_string(),
                filled_qty: "0".to_string(),
                side: "sell".to_string(),
                order_type: "limit".to_string(),
                order_class: None,
                status: "new".to_string(),
                limit_price: Some("480.00".to_string()),
                stop_price: None,
                trail_price: None,
                trail_percent: None,
                created_at: "2024-01-02T10:00:00Z".to_string(),
                filled_at: None,
                filled_avg_price: None,
                legs: None,
            },
            OrderInfo {
                id: "sl-leg".to_string(),
                symbol: "SPY".to_string(),
                qty: "5".to_string(),
                filled_qty: "0".to_string(),
                side: "sell".to_string(),
                order_type: "stop".to_string(),
                order_class: None,
                status: "held".to_string(),
                limit_price: None,
                stop_price: Some("460.00".to_string()),
                trail_price: None,
                trail_percent: None,
                created_at: "2024-01-02T10:00:00Z".to_string(),
                filled_at: None,
                filled_avg_price: None,
                legs: None,
            },
        ]),
    };

    let ids = AlpacaBroker::collect_cancellable_order_ids_for_symbol(&[parent], "SPY");
    assert_eq!(ids, vec!["tp-leg".to_string(), "sl-leg".to_string()]);
}

#[test]
fn collect_cancellable_order_ids_for_symbol_normalizes_crypto_symbol() {
    let order = OrderInfo {
        id: "crypto-exit".to_string(),
        symbol: "BTCUSD".to_string(),
        qty: "0.2".to_string(),
        filled_qty: "0".to_string(),
        side: "sell".to_string(),
        order_type: "limit".to_string(),
        order_class: Some("oco".to_string()),
        status: "new".to_string(),
        limit_price: Some("70000".to_string()),
        stop_price: None,
        trail_price: None,
        trail_percent: None,
        created_at: "2024-01-02T10:00:00Z".to_string(),
        filled_at: None,
        filled_avg_price: None,
        legs: None,
    };

    let ids = AlpacaBroker::collect_cancellable_order_ids_for_symbol(&[order], "BTC/USD");
    assert_eq!(ids, vec!["crypto-exit".to_string()]);
}

#[test]
fn collect_cancellable_exit_order_ids_for_symbol_filters_by_exit_side() {
    let sell_exit = OrderInfo {
        id: "sell-exit".to_string(),
        symbol: "SPY".to_string(),
        qty: "5".to_string(),
        filled_qty: "0".to_string(),
        side: "sell".to_string(),
        order_type: "limit".to_string(),
        order_class: None,
        status: "new".to_string(),
        limit_price: Some("500.00".to_string()),
        stop_price: None,
        trail_price: None,
        trail_percent: None,
        created_at: "2024-01-02T10:00:00Z".to_string(),
        filled_at: None,
        filled_avg_price: None,
        legs: None,
    };
    let buy_entry = OrderInfo {
        id: "buy-entry".to_string(),
        symbol: "SPY".to_string(),
        qty: "5".to_string(),
        filled_qty: "0".to_string(),
        side: "buy".to_string(),
        order_type: "limit".to_string(),
        order_class: None,
        status: "new".to_string(),
        limit_price: Some("470.00".to_string()),
        stop_price: None,
        trail_price: None,
        trail_percent: None,
        created_at: "2024-01-02T10:01:00Z".to_string(),
        filled_at: None,
        filled_avg_price: None,
        legs: None,
    };

    let ids = AlpacaBroker::collect_cancellable_exit_order_ids_for_symbol(
        &[sell_exit, buy_entry],
        "SPY",
        "sell",
    );
    assert_eq!(ids, vec!["sell-exit".to_string()]);
}

// ── AccountInfo parsing from mock JSON ──────────────────────────

#[test]
fn parse_account_json_string_fields() {
    // Alpaca returns most numeric fields as strings
    let j = json!({
        "equity": "100000.00",
        "cash": "50000.00",
        "buying_power": "200000.00",
        "portfolio_value": "100000.00",
        "initial_margin": "25000.00",
        "maintenance_margin": "12500.00",
        "currency": "USD",
        "pattern_day_trader": false,
        "trading_blocked": false,
        "last_equity": "99500.00",
    });
    let info = AccountInfo {
        equity: parse_f64_field(&j, "equity"),
        cash: parse_f64_field(&j, "cash"),
        buying_power: parse_f64_field(&j, "buying_power"),
        portfolio_value: parse_f64_field(&j, "portfolio_value"),
        initial_margin: parse_f64_field(&j, "initial_margin"),
        maintenance_margin: parse_f64_field(&j, "maintenance_margin"),
        currency: j["currency"].as_str().unwrap_or("USD").to_string(),
        pattern_day_trader: j["pattern_day_trader"].as_bool().unwrap_or(false),
        trading_blocked: j["trading_blocked"].as_bool().unwrap_or(false),
        last_equity: parse_f64_field(&j, "last_equity"),
        balance: parse_f64_field(&j, "last_equity"),
    };
    assert!((info.equity - 100_000.0).abs() < 1e-10);
    assert!((info.cash - 50_000.0).abs() < 1e-10);
    assert!((info.buying_power - 200_000.0).abs() < 1e-10);
    assert!((info.last_equity - 99_500.0).abs() < 1e-10);
    assert!((info.balance - 99_500.0).abs() < 1e-10);
    assert_eq!(info.currency, "USD");
    assert!(!info.pattern_day_trader);
}

// ── SnapshotData struct ─────────────────────────────────────────

#[test]
fn snapshot_data_construction() {
    let snap = SnapshotData {
        symbol: "AAPL".to_string(),
        last: 178.50,
        prev_close: 177.00,
        daily_volume: 45_000_000.0,
        regular_close: 178.25,
    };
    assert_eq!(snap.symbol, "AAPL");
    // Change % = (last - prev_close) / prev_close
    let change_pct = (snap.last - snap.prev_close) / snap.prev_close * 100.0;
    assert!((change_pct - 0.847457).abs() < 0.001);
}

// ── aggregate_weekly_to_monthly ─────────────────────────────────

#[test]
fn aggregate_weekly_to_monthly_basic() {
    let weekly = vec![
        Bar {
            timestamp: "2024-01-01T00:00:00Z".into(),
            open: 100.0,
            high: 110.0,
            low: 95.0,
            close: 105.0,
            volume: 1000.0,
        },
        Bar {
            timestamp: "2024-01-08T00:00:00Z".into(),
            open: 105.0,
            high: 112.0,
            low: 100.0,
            close: 108.0,
            volume: 1200.0,
        },
        Bar {
            timestamp: "2024-01-15T00:00:00Z".into(),
            open: 108.0,
            high: 115.0,
            low: 106.0,
            close: 113.0,
            volume: 900.0,
        },
        Bar {
            timestamp: "2024-01-22T00:00:00Z".into(),
            open: 113.0,
            high: 118.0,
            low: 110.0,
            close: 116.0,
            volume: 1100.0,
        },
        // February
        Bar {
            timestamp: "2024-02-05T00:00:00Z".into(),
            open: 116.0,
            high: 120.0,
            low: 114.0,
            close: 119.0,
            volume: 800.0,
        },
    ];
    let monthly = AlpacaBroker::aggregate_weekly_to_monthly(&weekly);
    assert_eq!(monthly.len(), 2);
    // January
    assert_eq!(monthly[0].open, 100.0);
    assert_eq!(monthly[0].high, 118.0);
    assert_eq!(monthly[0].low, 95.0);
    assert_eq!(monthly[0].close, 116.0);
    assert_eq!(monthly[0].volume, 4200.0);
    // February
    assert_eq!(monthly[1].open, 116.0);
    assert_eq!(monthly[1].close, 119.0);
}

#[test]
fn aggregate_weekly_to_monthly_empty() {
    let monthly = AlpacaBroker::aggregate_weekly_to_monthly(&[]);
    assert!(monthly.is_empty());
}

// ── OCO order body construction ──

#[test]
fn oco_order_body_has_correct_class() {
    // Verify the JSON body shape for OCO orders matches Alpaca's spec
    let body = serde_json::json!({
        "symbol": "SPY",
        "qty": "10",
        "side": "sell",
        "type": "limit",
        "time_in_force": "gtc",
        "order_class": "oco",
        "take_profit": { "limit_price": "500.00" },
        "stop_loss": { "stop_price": "450.00" },
    });
    assert_eq!(body["order_class"], "oco");
    assert_eq!(body["type"], "limit");
    assert_eq!(body["take_profit"]["limit_price"], "500.00");
    assert_eq!(body["stop_loss"]["stop_price"], "450.00");
}

#[test]
fn oco_order_body_with_stop_limit() {
    let body = serde_json::json!({
        "symbol": "AAPL",
        "qty": "5",
        "side": "sell",
        "type": "limit",
        "time_in_force": "gtc",
        "order_class": "oco",
        "take_profit": { "limit_price": "200.00" },
        "stop_loss": { "stop_price": "170.00", "limit_price": "169.50" },
    });
    assert_eq!(body["stop_loss"]["limit_price"], "169.50");
}

#[test]
fn format_order_price_rounds_correctly() {
    assert_eq!(format_order_price(100.123456), "100.12"); // $1+ → 2 decimals
    assert_eq!(format_order_price(0.05), "0.0500"); // $0.01-$0.99 → 4 decimals
    assert_eq!(format_order_price(0.00345), "0.00345000"); // sub-penny → 8 decimals
    assert_eq!(format_order_price(1500.0), "1500.00");
}
