use super::*;

#[test]
fn new_creates_broker() {
    let broker = KrakenBroker::new(String::new(), String::new());
    assert!(!broker.is_authenticated());
    assert!(broker.api_key.is_empty());
}

#[test]
fn is_authenticated_false_when_empty() {
    let broker = KrakenBroker::new(String::new(), String::new());
    assert!(!broker.is_authenticated());
}

#[test]
fn is_authenticated_true_when_set() {
    let broker = KrakenBroker::new("key".into(), "c2VjcmV0".into());
    assert!(broker.is_authenticated());
}

#[test]
fn sign_request_produces_nonempty_base64() {
    // Use a known base64-encoded secret
    let secret = BASE64.encode(b"test_secret_key_1234567890");
    let broker = KrakenBroker::new("test_key".into(), secret);
    let sig = broker.sign_request("/0/private/Balance", 1234567890, "nonce=1234567890");
    assert!(!sig.is_empty());
    // Verify it's valid base64
    assert!(BASE64.decode(&sig).is_ok());
}

#[test]
fn sign_request_deterministic() {
    let secret = BASE64.encode(b"deterministic_secret");
    let broker = KrakenBroker::new("key".into(), secret);
    let sig1 = broker.sign_request("/0/private/Balance", 100, "nonce=100");
    let sig2 = broker.sign_request("/0/private/Balance", 100, "nonce=100");
    assert_eq!(sig1, sig2);
}

#[test]
fn sign_request_matches_kraken_doc_vector() {
    let broker = KrakenBroker::new(
        "key".into(),
        "kQH5HW/8p1uGOVjbgWA7FunAmGO8lsSUXNsu3eow76sz84Q18fWxnyRzBHCd3pd5nE9qa99HAZtuZuj6F1huXg=="
            .into(),
    );
    let post_data =
        "nonce=1616492376594&ordertype=limit&pair=XBTUSD&price=37500&type=buy&volume=1.25";
    let sig = broker.sign_request("/0/private/AddOrder", 1616492376594, post_data);
    assert_eq!(
        sig,
        "4/dpxb3iT4tp/ZCVEwSnEsLxx0bqyhLpdfOpc6fn7OR8+UClSV5n9E6aSS8MPtnRfp32bAb0nmbRn6H8ndwLUQ=="
    );
}

#[test]
fn encode_form_params_percent_encodes_order_fields() {
    let params = vec![
        ("price".to_string(), "+2%".to_string()),
        (
            "close[ordertype]".to_string(),
            "stop-loss-limit".to_string(),
        ),
        ("close[price2]".to_string(), "28400".to_string()),
    ];
    assert_eq!(
        encode_form_params(&params),
        "price=%2B2%25&close%5Bordertype%5D=stop-loss-limit&close%5Bprice2%5D=28400"
    );
}

#[test]
fn kraken_private_rest_counter_cost_matches_rate_limit_docs() {
    assert_eq!(kraken_private_rest_counter_cost("/0/private/Balance"), 1.0);
    assert_eq!(
        kraken_private_rest_counter_cost("/0/private/OpenOrders"),
        1.0
    );
    assert_eq!(kraken_private_rest_counter_cost("/0/private/Ledgers"), 4.0);
    assert_eq!(
        kraken_private_rest_counter_cost("/0/private/QueryTrades"),
        4.0
    );
    assert_eq!(kraken_private_rest_counter_cost("/0/private/AddOrder"), 0.0);
    assert_eq!(
        kraken_private_rest_counter_cost("/0/private/CancelOrder"),
        0.0
    );
}

#[test]
fn trades_history_result_parses_unwrapped_result_and_count() {
    let payload = serde_json::json!({
        "count": 123,
        "trades": {
            "T-HRTX-1": {
                "ordertxid": "O-HRTX-1",
                "pair": "HRTX.EQUSD",
                "time": 1778841060.0,
                "type": "buy",
                "ordertype": "market",
                "price": "0.88",
                "cost": "230.56",
                "fee": "0.10",
                "vol": "262.0",
                "margin": "0"
            }
        }
    });

    let (trades, count) = parse_trades_history_result(&payload).unwrap();
    assert_eq!(count, Some(123));
    assert_eq!(trades.len(), 1);
    assert_eq!(trades[0].pair, "HRTX.EQUSD");
    assert_eq!(trades[0].side, "buy");
    assert_eq!(trades[0].price, 0.88);
    assert_eq!(trades[0].vol, 262.0);
}

#[test]
fn equity_position_summaries_convert_only_equity_balances() {
    let balances = vec![
        ("XXBT".to_string(), 0.25),
        ("BABY".to_string(), 123.0),
        ("XXMR".to_string(), 1.5),
        ("HRTX.EQ".to_string(), 7.0),
        ("ZUSD".to_string(), 1000.0),
        ("USDT".to_string(), 50.0),
    ];
    let positions = KrakenBroker::equity_position_summaries_from_balances(&balances);

    assert_eq!(positions.len(), 1);
    assert_eq!(positions[0].symbol, "HRTX");
    assert_eq!(positions[0].qty, 7.0);
    assert_eq!(positions[0].asset_class, "stock");
    assert_eq!(positions[0].side, "long");
}

#[test]
fn kraken_order_request_builds_full_add_order_params() {
    let mut order = KrakenOrderRequest::basic("XBTUSD", "BUY", "stop_loss_limit", 1.25)
        .with_price(37000.0)
        .with_price2(36950.0);
    order.leverage = Some("2".to_string());
    order.reduce_only = true;
    order.oflags = vec!["post".to_string(), "fciq".to_string()];
    order.time_in_force = Some("gtd".to_string());
    order.expire_time = Some("+60".to_string());
    order.client_order_id = Some("typhoon-kr-0001".to_string());
    order.close = Some(KrakenConditionalClose {
        order_type: "take_profit".to_string(),
        price: Some("39000".to_string()),
        price2: None,
    });

    order.validate().unwrap();
    let params = order.to_params();
    assert!(params.contains(&("pair".to_string(), "XBTUSD".to_string())));
    assert!(params.contains(&("type".to_string(), "buy".to_string())));
    assert!(params.contains(&("ordertype".to_string(), "stop-loss-limit".to_string())));
    assert!(params.contains(&("price".to_string(), "37000".to_string())));
    assert!(params.contains(&("price2".to_string(), "36950".to_string())));
    assert!(params.contains(&("reduce_only".to_string(), "true".to_string())));
    assert!(params.contains(&("oflags".to_string(), "post,fciq".to_string())));
    assert!(params.contains(&("timeinforce".to_string(), "GTD".to_string())));
    assert!(params.contains(&("close[ordertype]".to_string(), "take-profit".to_string())));
}

#[test]
fn kraken_order_request_rejects_missing_stop_limit_price2() {
    let order =
        KrakenOrderRequest::basic("XBTUSD", "sell", "stop-loss-limit", 1.0).with_price(37000.0);
    let err = order.validate().unwrap_err();
    assert!(err.contains("price2"));
}

#[test]
fn kraken_iceberg_uses_rest_displayvol() {
    let order = KrakenOrderRequest::basic("XBTUSD", "sell", "iceberg", 10.0)
        .with_price(50000.0)
        .with_display_volume(1.0);
    order.validate().unwrap();
    let params = order.to_params();
    assert!(params.contains(&("ordertype".to_string(), "limit".to_string())));
    assert!(params.contains(&("displayvol".to_string(), "1".to_string())));
}

#[test]
fn kraken_settle_position_allows_zero_volume() {
    let mut order = KrakenOrderRequest::basic("XBTUSD", "buy", "settle-position", 0.0);
    order.leverage = Some("5".to_string());
    order.validate().unwrap();
    let params = order.to_params();
    assert!(params.contains(&("ordertype".to_string(), "settle-position".to_string())));
    assert!(params.contains(&("volume".to_string(), "0".to_string())));
}

#[test]
fn nonce_increments() {
    let broker = KrakenBroker::new(String::new(), String::new());
    let n1 = broker.next_nonce();
    let n2 = broker.next_nonce();
    assert!(n2 > n1);
}

#[test]
fn new_broker_has_empty_api_key() {
    let broker = KrakenBroker::new(String::new(), String::new());
    assert!(broker.api_key.is_empty());
}

#[test]
fn new_broker_with_credentials() {
    let broker = KrakenBroker::new("my_key".to_string(), "my_secret".to_string());
    assert_eq!(broker.api_key.as_str(), "my_key");
    assert_eq!(broker.api_secret.as_str(), "my_secret");
}

#[test]
fn pair_matches_normalizes_btc_aliases() {
    assert!(KrakenBroker::pair_matches("XXBTZUSD", "BTCUSD"));
    assert!(KrakenBroker::pair_matches("POMZUSD", "POMUSD"));
    assert!(KrakenBroker::pair_matches("HRTXZUSD", "HRTX"));
    assert!(KrakenBroker::pair_matches("XBTUSD", "BTC/USD"));
    assert!(KrakenBroker::pair_matches("XDGUSD", "DOGEUSD"));
    assert!(!KrakenBroker::pair_matches("ETHUSD", "SOLUSD"));
}

#[test]
fn position_summaries_from_raw_normalizes_display_pairs() {
    let raw = vec![
        serde_json::json!({
            "pair": "XXBTZUSD",
            "type": "buy",
            "vol": "0.75",
            "vol_closed": "0.25",
            "cost": "30000",
            "value": "31000",
            "net": "1000",
            "posid": "abc123"
        }),
        serde_json::json!({
            "pair": "XDGUSD",
            "type": "sell",
            "vol": "1000",
            "vol_closed": "400",
            "cost": "60",
            "value": "55",
            "net": "-5",
            "posid": "doge456"
        }),
    ];
    let summaries = KrakenBroker::position_summaries_from_raw(&raw);
    assert_eq!(summaries.len(), 2);
    assert_eq!(summaries[0].symbol, "BTCUSD");
    assert_eq!(summaries[0].side, "long");
    assert!((summaries[0].qty - 0.5).abs() < f64::EPSILON);
    assert_eq!(summaries[1].symbol, "DOGEUSD");
    assert_eq!(summaries[1].side, "short");
    assert!((summaries[1].qty - 600.0).abs() < f64::EPSILON);
}

#[test]
fn net_position_volume_offsets_long_and_short() {
    let raw = vec![
        serde_json::json!({
            "pair": "XXBTZUSD",
            "type": "buy",
            "vol": "1.5",
            "vol_closed": "0.25"
        }),
        serde_json::json!({
            "pair": "XBTUSD",
            "type": "sell",
            "vol": "0.5",
            "vol_closed": "0.1"
        }),
    ];
    let net = KrakenBroker::net_position_volume(&raw, "BTCUSD");
    assert!((net - 0.85).abs() < 1e-9);
}

#[test]
fn ws_instrument_parser_extracts_tokenized_assets_as_equity_universe() {
    let snapshot = serde_json::json!({
        "assets": [
            {"id":"BTC", "class":"currency"},
            {"id":"AAPLx", "underlying_symbol":"AAPL", "class":"tokenized_asset"},
            {"id":"AAPLSPV", "underlying_symbol":"AAPL", "class":"tokenized_asset"},
            {"id":"BRK.Bx", "underlying_symbol":"BRK.B", "class":"tokenized_asset"}
        ],
        "pairs": [
            {"symbol":"BTC/USD", "base":"BTC", "quote":"USD", "status":"online"},
            {"symbol":"AAPLx/USD", "base":"AAPLx", "quote":"USD", "status":"online"},
            {"symbol":"AAPLSPV/USD", "base":"AAPLx", "quote":"USD", "status":"online"},
            {"symbol":"BRK.Bx/USD", "base":"BRK.Bx", "quote":"USD", "status":"post_only"},
            {"symbol":"AAPLx/EUR", "base":"AAPLx", "quote":"EUR", "status":"online"}
        ]
    });

    let markets = parse_tokenized_equity_markets(&snapshot).unwrap();
    let symbols: Vec<_> = markets.iter().map(|m| m.symbol.as_str()).collect();
    assert_eq!(symbols, vec!["AAPL", "BRK.B"]);
    assert!(markets.iter().all(|m| m.tradable));
    assert!(
        markets
            .iter()
            .all(|m| m.instrument_status.as_deref() == Some("enabled"))
    );
}

#[test]
fn equity_market_merge_preserves_full_iapi_catalog_and_adds_ws_tokenized() {
    let iapi = vec![
        KrakenEquityMarket {
            symbol: "A".into(),
            name: Some("Agilent Technologies Inc.".into()),
            tradable: true,
            status: Some("active".into()),
            instrument_status: Some("enabled".into()),
            overnight_trading: Some(true),
            tokenized: false,
        },
        KrakenEquityMarket {
            symbol: "AAPL".into(),
            name: Some("Apple Inc.".into()),
            tradable: true,
            status: Some("active".into()),
            instrument_status: Some("enabled".into()),
            overnight_trading: Some(false),
            tokenized: false,
        },
    ];
    let ws = vec![
        KrakenEquityMarket {
            symbol: "AAPL".into(),
            name: None,
            tradable: true,
            status: Some("online".into()),
            instrument_status: Some("enabled".into()),
            overnight_trading: None,
            tokenized: true,
        },
        KrakenEquityMarket {
            symbol: "WOK".into(),
            name: None,
            tradable: true,
            status: Some("online".into()),
            instrument_status: Some("enabled".into()),
            overnight_trading: None,
            tokenized: true,
        },
    ];

    let merged = merge_equity_markets(iapi, ws);
    let symbols: Vec<_> = merged.iter().map(|m| m.symbol.as_str()).collect();
    assert_eq!(symbols, vec!["A", "AAPL", "WOK"]);
    let aapl = merged.iter().find(|m| m.symbol == "AAPL").unwrap();
    assert_eq!(aapl.name.as_deref(), Some("Apple Inc."));
    assert!(aapl.tradable);
    // iapi's known overnight value survives the merge with the WS row's None.
    assert_eq!(aapl.overnight_trading, Some(false));
    assert_eq!(
        merged
            .iter()
            .find(|m| m.symbol == "A")
            .unwrap()
            .overnight_trading,
        Some(true)
    );
    // The WS-tokenized flag merges in: true for symbols present in the WS
    // tokenized snapshot, false for iapi-only Securities.
    assert!(aapl.tokenized, "AAPL is on WS → tokenized");
    assert!(
        merged.iter().find(|m| m.symbol == "WOK").unwrap().tokenized,
        "WOK is WS-only → tokenized"
    );
    assert!(
        !merged.iter().find(|m| m.symbol == "A").unwrap().tokenized,
        "A is iapi-only → not tokenized"
    );
}

#[tokio::test]
#[ignore] // Requires network access — run with `cargo test -- --ignored`
async fn get_equity_markets_public_catalog_preserves_full_securities_universe() {
    let broker = KrakenBroker::new(String::new(), String::new());
    let markets = broker.get_equity_markets().await.unwrap();
    assert!(
        markets.len() > 12_000,
        "expected full Kraken Securities universe, got {} markets",
        markets.len()
    );
    assert!(markets.iter().any(|market| market.symbol == "AAPL"));
    assert!(markets.iter().any(|market| market.symbol == "TSLA"));
}

#[tokio::test]
#[ignore] // Requires network access — run with `cargo test -- --ignored`
async fn get_tradeable_pairs_public() {
    let broker = KrakenBroker::new(String::new(), String::new());
    let pairs = broker.get_tradeable_pairs().await.unwrap();
    assert!(!pairs.is_empty());
    assert!(
        pairs
            .iter()
            .any(|(name, _)| name.contains("BTC") || name.contains("XBT"))
    );
}
