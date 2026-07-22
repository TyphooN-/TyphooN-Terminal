use super::*;

fn test_order(
    symbol: &str,
    side: &str,
    qty: &str,
    filled: &str,
    limit: Option<&str>,
    status: &str,
) -> OrderInfo {
    OrderInfo {
        id: format!("{symbol}-{side}"),
        symbol: symbol.to_string(),
        qty: qty.to_string(),
        filled_qty: filled.to_string(),
        side: side.to_string(),
        order_type: "limit".to_string(),
        order_class: None,
        status: status.to_string(),
        limit_price: limit.map(str::to_string),
        stop_price: None,
        trail_price: None,
        trail_percent: None,
        created_at: "2026-01-01T00:00:00Z".to_string(),
        filled_at: None,
        filled_avg_price: None,
        legs: None,
    }
}

#[test]
fn alpaca_order_lines_use_open_qty_signed_notional_pct_and_pips() {
    let orders = vec![test_order("AAPL", "buy", "10", "2", Some("99.50"), "new")];
    let mut lines = Vec::new();

    collect_alpaca_order_lines_for_symbol(&orders, "AAPL", 100.0, 0.01, Some(10_000.0), &mut lines);

    assert_eq!(lines.len(), 1);
    let line = &lines[0];
    assert!(line.is_buy);
    assert_eq!(line.qty, 8.0);
    assert_eq!(line.price, 99.50);
    assert!((line.notional_delta + 796.0).abs() < 1e-9);
    assert!((line.account_pct_delta.unwrap() + 7.96).abs() < 1e-9);
    assert!((line.pips_from_current.unwrap() + 50.0).abs() < 1e-9);
}

#[test]
fn alpaca_order_lines_merge_same_side_source_and_price() {
    let orders = vec![
        test_order("AAPL", "sell", "10", "0", Some("105.00"), "new"),
        test_order("AAPL", "sell", "15", "5", Some("105.00"), "new"),
    ];
    let mut lines = Vec::new();

    collect_alpaca_order_lines_for_symbol(&orders, "AAPL", 100.0, 0.01, Some(10_000.0), &mut lines);

    assert_eq!(lines.len(), 1);
    let line = &lines[0];
    assert!(!line.is_buy);
    assert_eq!(line.qty, 20.0);
    assert_eq!(line.price, 105.0);
    assert!((line.notional_delta - 2100.0).abs() < 1e-9);
    assert!((line.account_pct_delta.unwrap() - 21.0).abs() < 1e-9);
}

#[test]
fn alpaca_order_lines_flatten_nested_working_legs_and_skip_filled_parent() {
    let mut parent = test_order("SPY", "buy", "1", "1", Some("470"), "filled");
    parent.legs = Some(vec![test_order(
        "SPY",
        "sell",
        "1",
        "0",
        Some("480"),
        "new",
    )]);
    let mut lines = Vec::new();

    collect_alpaca_order_lines_for_symbol(
        &[parent],
        "SPY",
        475.0,
        0.01,
        Some(20_000.0),
        &mut lines,
    );

    assert_eq!(lines.len(), 1);
    assert!(!lines[0].is_buy);
    assert_eq!(lines[0].price, 480.0);
    assert!((lines[0].notional_delta - 480.0).abs() < 1e-9);
    assert!((lines[0].pips_from_current.unwrap() - 500.0).abs() < 1e-9);
}

#[test]
fn news_article_tickers_normalizes_and_deduplicates_symbols() {
    let tickers = vec![
        " aapl ".to_string(),
        "MSFT".to_string(),
        "msft".to_string(),
        "../bad".to_string(),
        "THIS-SYMBOL-IS-TOO-LONG".to_string(),
    ];

    assert_eq!(
        TyphooNApp::news_article_tickers("AAPL", &tickers),
        vec!["AAPL".to_string(), "MSFT".to_string()]
    );
}

#[test]
fn news_ticker_normalization_accepts_common_market_symbols() {
    assert_eq!(
        TyphooNApp::normalize_news_ticker_for_chart(" brk.b "),
        Some("BRK.B".to_string())
    );
    assert_eq!(
        TyphooNApp::normalize_news_ticker_for_chart("BTC/USD"),
        Some("BTC/USD".to_string())
    );
    assert_eq!(TyphooNApp::normalize_news_ticker_for_chart(""), None);
    assert_eq!(
        TyphooNApp::normalize_news_ticker_for_chart("THIS-SYMBOL-IS-TOO-LONG"),
        None
    );
}

#[test]
fn mtf_grid_timeframes_include_low_timeframes_for_native_kraken_pairs() {
    let labels: Vec<&str> = MTF_GRID_TIMEFRAMES
        .iter()
        .map(|(label, _)| *label)
        .collect();

    assert_eq!(
        labels,
        vec!["M1", "M5", "M15", "M30", "H1", "H4", "D1", "W1", "MN1"]
    );
}

#[test]
fn mtf_grid_groups_visible_charts_by_symbol_and_sorts_each_symbol_by_timeframe() {
    let charts = vec![
        ChartState::new("kraken:WOK.EQ:1Day", Timeframe::D1),
        ChartState::new("kraken:BABYUSD:4Hour", Timeframe::H4),
        ChartState::new("kraken:WOK.EQ:15Min", Timeframe::M15),
        ChartState::new("kraken:BABYUSD:1Hour", Timeframe::H1),
        ChartState::new("kraken:WOK.EQ:1Min", Timeframe::M1),
        ChartState::new("kraken:WOK.EQ:1Week", Timeframe::W1),
        ChartState::new("kraken:BABYUSD:5Min", Timeframe::M5),
    ];
    let visible = vec![true; charts.len()];

    let groups = mtf_visible_chart_groups(&charts, &visible);

    // Groups are sorted alphabetically by symbol (BABYUSD before WOK), and each
    // group's indices are ordered by ascending timeframe rank.
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].symbol, "BABYUSD");
    assert_eq!(groups[0].indices, vec![6, 3, 1]);
    assert_eq!(groups[1].symbol, "WOK");
    assert_eq!(groups[1].indices, vec![4, 2, 0, 5]);
}

#[test]
fn mtf_grid_omits_empty_low_timeframe_cells() {
    let mut m15 = ChartState::new("AMC", Timeframe::M15);
    m15.bars.push(Bar {
        ts_ms: 1,
        open: 1.0,
        high: 1.0,
        low: 1.0,
        close: 1.0,
        volume: 1.0,
    });
    let mut h1 = ChartState::new("AMC", Timeframe::H1);
    h1.bars.push(Bar {
        ts_ms: 1,
        open: 1.0,
        high: 1.0,
        low: 1.0,
        close: 1.0,
        volume: 1.0,
    });
    let mut m1 = ChartState::new("AMC", Timeframe::M1);
    m1.show_in_tab_bar = false;
    let mut m5 = ChartState::new("AMC", Timeframe::M5);
    m5.show_in_tab_bar = false;
    let charts = vec![m1, m5, m15, h1];
    let visible = vec![true; charts.len()];

    let groups = mtf_visible_chart_groups(&charts, &visible);

    assert_eq!(groups.len(), 1);
    // Empty M1/M5 backing charts (indices 0,1) are excluded; only the loaded
    // M15/H1 tabs (indices 2,3) remain.
    assert_eq!(groups[0].indices, vec![2, 3]);
}

#[test]
fn mtf_chart_canvas_uses_flat_two_column_flow() {
    let charts = vec![
        ChartState::new("AMC", Timeframe::M15),
        ChartState::new("AMC", Timeframe::M30),
        ChartState::new("AMC", Timeframe::H1),
        ChartState::new("AVAT", Timeframe::M15),
        ChartState::new("AVAT", Timeframe::M30),
    ];
    let visible = vec![true; charts.len()];
    let groups = mtf_visible_chart_groups(&charts, &visible);

    assert_eq!(mtf_flat_chart_indices(&groups), vec![0, 1, 2, 3, 4]);
    assert_eq!(mtf_canvas_grid_cols(1), 2);
    assert_eq!(mtf_canvas_grid_cols(5), 2);
    assert_eq!(mtf_canvas_grid_rows(5), 3);
}

#[test]
fn mtf_grid_suppresses_symbol_when_broker_has_no_m1_or_m5_bars() {
    let charts = vec![
        ChartState::new("CC", Timeframe::D1),
        ChartState::new("CC", Timeframe::H4),
        ChartState::new("WEN", Timeframe::D1),
    ];
    let visible = vec![true; charts.len()];
    let no_low_tf_symbols = ["CC".to_string()].into_iter().collect();

    let groups = mtf_visible_chart_groups_filtered(&charts, &visible, &no_low_tf_symbols);

    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].symbol, "WEN");
}

#[test]
fn low_timeframe_no_data_symbols_require_m1_and_m5_from_same_broker() {
    let mut pairs = std::collections::HashMap::new();
    pairs.insert(
        "m1".to_string(),
        UnresolvablePair {
            broker: "kraken-equities".to_string(),
            symbol: "CC".to_string(),
            timeframe: "1Min".to_string(),
            reason: "provider returned no bars".to_string(),
            ts: 1,
        },
    );
    pairs.insert(
        "m5".to_string(),
        UnresolvablePair {
            broker: "kraken-equities".to_string(),
            symbol: "CC.EQ".to_string(),
            timeframe: "5Min".to_string(),
            reason: "provider returned no data".to_string(),
            ts: 2,
        },
    );
    pairs.insert(
        "wen_m1".to_string(),
        UnresolvablePair {
            broker: "kraken-equities".to_string(),
            symbol: "WEN".to_string(),
            timeframe: "1Min".to_string(),
            reason: "provider returned no bars".to_string(),
            ts: 3,
        },
    );

    let suppressed = low_timeframe_no_data_symbols(&pairs);

    assert!(suppressed.contains("CC"));
    assert!(!suppressed.contains("WEN"));
}

#[test]
fn open_chart_preload_indices_include_inactive_empty_tabs() {
    let mut loaded = ChartState::new("CC", Timeframe::D1);
    loaded.bars.push(Bar {
        ts_ms: 1,
        open: 10.0,
        high: 10.0,
        low: 10.0,
        close: 10.0,
        volume: 1.0,
    });
    let mut hidden_low_tf_backing = ChartState::new("CC", Timeframe::M1);
    hidden_low_tf_backing.show_in_tab_bar = false;
    let charts = vec![
        loaded,
        ChartState::new("WEN", Timeframe::D1),
        ChartState::new("CC", Timeframe::H4),
        hidden_low_tf_backing,
    ];

    assert_eq!(open_chart_preload_indices(&charts), vec![1, 2]);
}

#[test]
fn company_name_catalog_prefers_primary_broker_names() {
    let alpaca_assets = vec![
        (
            "CC".to_string(),
            "The Chemours Company".to_string(),
            "us_equity".to_string(),
        ),
        (
            "BTCUSD".to_string(),
            "Bitcoin".to_string(),
            "crypto".to_string(),
        ),
    ];
    let mut kraken_names = std::collections::HashMap::new();
    kraken_names.insert("CC".to_string(), "Kraken CC Placeholder".to_string());

    let alpaca_primary =
        chart_company_name_catalog(&alpaca_assets, &kraken_names, OrderBroker::Alpaca);
    assert_eq!(
        alpaca_primary.get("CC").map(String::as_str),
        Some("The Chemours Company")
    );
    assert!(!alpaca_primary.contains_key("BTCUSD"));

    let kraken_primary =
        chart_company_name_catalog(&alpaca_assets, &kraken_names, OrderBroker::Kraken);
    assert_eq!(
        kraken_primary.get("CC").map(String::as_str),
        Some("Kraken CC Placeholder")
    );
}

#[test]
fn empty_chart_load_retry_is_backed_off_after_no_data_attempt() {
    let now = std::time::Instant::now();

    assert!(empty_chart_load_retry_due(None, now));
    assert!(!empty_chart_load_retry_due(
        Some(now - EMPTY_CHART_RELOAD_RETRY_AFTER / 2),
        now
    ));
    assert!(empty_chart_load_retry_due(
        Some(now - EMPTY_CHART_RELOAD_RETRY_AFTER),
        now
    ));
}

fn test_position(symbol: &str, qty: f64, side: &str) -> PositionInfo {
    PositionInfo {
        symbol: symbol.to_string(),
        qty,
        qty_available: qty,
        side: side.to_string(),
        avg_entry_price: 1.0,
        market_value: qty,
        unrealized_pl: 0.0,
        asset_class: "stock".to_string(),
        asset_id: "equity_balance:test".to_string(),
    }
}

#[test]
fn kraken_balance_overlay_skips_assets_already_reported_as_positions() {
    let positions = vec![test_position("WOK", 7142.0, "long")];

    assert!(kraken_position_covers_balance_asset(&positions, "WOK.EQ"));
    assert!(kraken_position_covers_balance_asset(&positions, "WOK"));
}

#[test]
fn kraken_balance_overlay_still_allows_inventory_without_position_row() {
    let positions = vec![test_position("GDC", 100.0, "long")];

    assert!(!kraken_position_covers_balance_asset(&positions, "WOK.EQ"));
    assert!(!kraken_position_covers_balance_asset(&positions, "WOK"));
}
