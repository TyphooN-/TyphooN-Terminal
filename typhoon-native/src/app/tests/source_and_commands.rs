fn test_bar(ts_ms: i64) -> (i64, f64, f64, f64, f64, f64) {
    (ts_ms, 10.0, 11.0, 9.0, 10.5, 1000.0)
}

#[test]
fn alpaca_batch_fetch_supports_every_standard_timeframe_for_broad_equity_assist() {
    for timeframe in [
        "1Min", "M1", "5Min", "M5", "15Min", "30Min", "1Hour", "4Hour", "1Day", "1Week", "1Month",
        "MN1",
    ] {
        assert!(
            TyphooNApp::alpaca_batch_fetch_supported(timeframe),
            "{timeframe} should use Alpaca's multi-symbol bars endpoint"
        );
    }
    assert_eq!(TyphooNApp::alpaca_batch_fetch_chunk_symbols("1Min"), 8);
    assert_eq!(TyphooNApp::alpaca_batch_fetch_chunk_symbols("1Hour"), 16);
    assert_eq!(TyphooNApp::alpaca_batch_fetch_chunk_symbols("1Month"), 50);
}

#[test]
fn chart_source_cadence_rejects_monthly_bars_mislabeled_as_daily() {
    let day = 86_400_000i64;
    let monthly_as_daily: Vec<_> = (0..36).map(|i| test_bar(i * 30 * day)).collect();
    assert!(!chart_source_bars_match_timeframe(
        "yahoo-chart",
        "1Day",
        &monthly_as_daily
    ));
    assert!(!chart_source_bars_match_timeframe(
        "alpaca",
        "1Day",
        &monthly_as_daily
    ));
}

#[test]
fn chart_source_cadence_rejects_kraken_equity_rolling_htf_bars() {
    let day = 86_400_000i64;
    let rolling_month: Vec<_> = (0..36).map(|i| test_bar(i * 43 * day)).collect();
    for source in ["kraken", "kraken-equities", "kraken-futures"] {
        assert!(!chart_source_bars_match_timeframe(
            source,
            "1Month",
            &rolling_month
        ));
    }

    let rolling_week: Vec<_> = (0..36).map(|i| test_bar(i * 10 * day)).collect();
    assert!(!chart_source_bars_match_timeframe(
        "kraken-equities",
        "1Week",
        &rolling_week
    ));
}

#[test]
fn chart_source_cadence_accepts_calendar_monthly_bars() {
    let day = 86_400_000i64;
    let calendar_month: Vec<_> = (0..36).map(|i| test_bar(i * 31 * day)).collect();
    assert!(chart_source_bars_match_timeframe(
        "alpaca",
        "1Month",
        &calendar_month
    ));
}

#[test]
fn chart_constructs_calendar_monthly_from_daily_raw_bars() {
    let day = 86_400_000i64;
    let raw = vec![
        (1_704_067_200_000, 10.0, 12.0, 9.0, 11.0, 100.0),
        (1_704_153_600_000, 11.0, 13.0, 10.0, 12.0, 200.0),
        (1_706_745_600_000, 12.0, 15.0, 11.0, 14.0, 300.0),
        (1_706_832_000_000, 14.0, 16.0, 13.0, 15.0, 400.0),
    ];
    let monthly = ChartState::aggregate_daily_raw_to_monthly(raw);
    assert_eq!(monthly.len(), 2);
    assert_eq!(monthly[0].ts_ms, 1_704_067_200_000 / day * day);
    assert_eq!(monthly[0].open, 10.0);
    assert_eq!(monthly[0].high, 13.0);
    assert_eq!(monthly[0].low, 9.0);
    assert_eq!(monthly[0].close, 12.0);
    assert_eq!(monthly[0].volume, 300.0);
    assert_eq!(monthly[1].open, 12.0);
    assert_eq!(monthly[1].high, 16.0);
    assert_eq!(monthly[1].low, 11.0);
    assert_eq!(monthly[1].close, 15.0);
    assert_eq!(monthly[1].volume, 700.0);
}

#[test]
fn chart_source_cadence_accepts_market_daily_bars() {
    let day = 86_400_000i64;
    let mut ts = 0i64;
    let mut bars = Vec::new();
    for i in 0..60 {
        bars.push(test_bar(ts));
        ts += if i % 5 == 4 { 3 * day } else { day };
    }
    assert!(chart_source_bars_match_timeframe(
        "yahoo-chart",
        "1Day",
        &bars
    ));
}

#[test]
fn chart_gap_fill_rejects_equity_fallback_inside_primary_span() {
    assert!(!chart_gap_fill_bar_allowed(
        "kraken-equities",
        "yahoo-chart",
        5,
        Some(1),
        Some(10)
    ));
    assert!(chart_gap_fill_bar_allowed(
        "kraken-equities",
        "alpaca",
        11,
        Some(1),
        Some(10)
    ));
    assert!(chart_gap_fill_bar_allowed(
        "kraken",
        "yahoo-chart",
        5,
        Some(1),
        Some(10)
    ));
}

#[test]
fn chart_quote_overlay_rejects_stale_quote_for_newer_bar() {
    let day = 86_400_000i64;
    assert!(!chart_quote_overlay_allowed(
        10 * day + 13 * 3_600_000,
        11 * day
    ));
    assert!(chart_quote_overlay_allowed(
        11 * day + 13 * 3_600_000,
        11 * day
    ));
}

#[test]
fn kraken_depth_stream_support_uses_loaded_pair_universe() {
    let pairs = vec![("XBT/USD".to_string(), "BTC/USD".to_string())];
    assert!(kraken_depth_stream_supported("BTCUSD", &pairs));
    assert!(kraken_depth_stream_supported("BTC/USD", &pairs));
    assert!(kraken_depth_stream_supported("XBTUSD", &pairs));
    assert!(!kraken_depth_stream_supported("AAPL", &pairs));
    assert!(!kraken_depth_stream_supported("AAPL.EQ", &pairs));
}

#[test]
fn kraken_depth_stream_support_falls_back_only_without_pair_universe() {
    assert!(kraken_depth_stream_supported("BTCUSD", &[]));
    assert!(!kraken_depth_stream_supported("", &[]));
    assert!(!kraken_depth_stream_supported("AAPL.EQ", &[]));
}

#[test]
fn chart_fresh_equity_source_policy_targets_plain_equities_only() {
    assert!(chart_prefers_fresh_equity_source("WOK"));
    assert!(chart_prefers_fresh_equity_source("WOK.EQ"));
    assert!(!chart_prefers_fresh_equity_source("BTCUSD"));
    assert!(!chart_prefers_fresh_equity_source("BTC/USD"));
    assert!(!chart_prefers_fresh_equity_source("EURUSD"));
}

#[test]
fn chart_equity_source_selection_can_prefer_fresher_fallback() {
    let day = 86_400_000i64;
    let stale_native = vec![test_bar(1 * day), test_bar(2 * day)];
    let fresh_fallback = vec![test_bar(8 * day), test_bar(9 * day)];
    assert!(chart_bar_last_valid_ts(&fresh_fallback) > chart_bar_last_valid_ts(&stale_native));
    // Default orientation (Kraken primary): kraken-equities defines the scale.
    assert!(
        chart_equity_source_rank_for("kraken-equities", OrderBroker::Kraken)
            < chart_equity_source_rank_for("alpaca", OrderBroker::Kraken)
    );
    assert_eq!(chart_equity_source_rank("kraken"), None);
}

#[test]
fn chart_missing_equity_data_key_uses_native_equity_source_not_kraken_spot() {
    assert_eq!(
        chart_missing_data_cache_key("WEN", "1Min"),
        "kraken-equities:WEN:1Min"
    );
    assert_eq!(
        chart_missing_data_cache_key("BTC/USD", "1Min"),
        "kraken:BTC/USD:1Min"
    );
}

#[test]
fn restored_low_tf_equity_cache_key_does_not_probe_alpaca_assist() {
    let db_path = std::env::temp_dir().join(format!(
        "typhoon-low-tf-restored-cache-key-test-{}-{}.db",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    let cache = SqliteCache::open(&db_path).unwrap();
    let dsm = typhoon_engine::core::data_source::DataSourceManager::default();

    let mut chart = ChartState::new("alpaca:AVAT:1Min", Timeframe::M1);
    assert_eq!(
        chart.find_cache_key(&cache, &dsm),
        "kraken-equities:AVAT:1Min"
    );

    let mut log = std::collections::VecDeque::new();
    chart.load(&cache, &mut log, None, &dsm);
    let messages: Vec<String> = log.iter().map(|entry| entry.msg.clone()).collect();
    assert!(
        messages
            .iter()
            .any(|msg| msg.contains("No chart data found for key 'kraken-equities:AVAT:1Min'")),
        "logs should point at the native low-TF source: {messages:?}"
    );
    assert!(
        messages
            .iter()
            .all(|msg| !msg.contains("Merged cache load") && !msg.contains("alpaca:AVAT:1Min")),
        "restored M1 equity load must not spam merged/alpaca probes: {messages:?}"
    );

    let _ = std::fs::remove_file(db_path);
}

#[test]
fn alpaca_low_tf_assist_skips_kraken_equity_universe_with_o1_membership() {
    let kraken_equities: std::collections::HashSet<String> = ["AVAT".to_string()].into();
    assert!(TyphooNApp::alpaca_low_tf_assist_unsupported_for_symbol(
        &kraken_equities,
        "AVAT",
        "1Min"
    ));
    assert!(TyphooNApp::alpaca_low_tf_assist_unsupported_for_symbol(
        &kraken_equities,
        "AVAT",
        "5Min"
    ));
    assert!(!TyphooNApp::alpaca_low_tf_assist_unsupported_for_symbol(
        &kraken_equities,
        "AVAT",
        "15Min"
    ));
    assert!(!TyphooNApp::alpaca_low_tf_assist_unsupported_for_symbol(
        &kraken_equities,
        "AAPL",
        "1Min"
    ));
}

#[test]
fn chart_equity_source_rank_inverts_with_primary_broker() {
    // ADR-126: the primary broker's equity source is the trusted rank-0 scale; the
    // other tradeable broker drops to the rank-2 assist. Yahoo/default are unchanged
    // depth fallbacks in both orientations.
    assert_eq!(
        chart_equity_source_rank_for("kraken-equities", OrderBroker::Kraken),
        Some(0)
    );
    assert_eq!(
        chart_equity_source_rank_for("alpaca", OrderBroker::Kraken),
        Some(2)
    );
    assert_eq!(
        chart_equity_source_rank_for("alpaca", OrderBroker::Alpaca),
        Some(0)
    );
    assert_eq!(
        chart_equity_source_rank_for("kraken-equities", OrderBroker::Alpaca),
        Some(2)
    );
    for primary in [OrderBroker::Kraken, OrderBroker::Alpaca] {
        assert_eq!(
            chart_equity_source_rank_for("yahoo-chart", primary),
            Some(3)
        );
        assert_eq!(chart_equity_source_rank_for("default", primary), Some(4));
        assert_eq!(chart_equity_source_rank_for("kraken", primary), None);
    }
}

#[test]
fn order_broker_persistence_and_cycle_helpers() {
    // Persistence round-trip (string token <-> enum), case-insensitive parse.
    for broker in [OrderBroker::Alpaca, OrderBroker::Kraken] {
        assert_eq!(
            OrderBroker::from_persist_str(broker.as_persist_str()),
            Some(broker)
        );
    }
    assert_eq!(
        OrderBroker::from_persist_str("ALPACA"),
        Some(OrderBroker::Alpaca)
    );
    assert_eq!(OrderBroker::from_persist_str("nope"), None);
    // Identity -> equity-merge source tag bridge.
    assert_eq!(OrderBroker::Alpaca.equity_source_tag(), "alpaca");
    assert_eq!(OrderBroker::Kraken.equity_source_tag(), "kraken-equities");
    // The top-bar switch cycle lists only enabled brokers, in stable order, and is
    // empty/singleton when fewer than two are enabled (switch hidden in the UI).
    assert_eq!(
        OrderBroker::enabled_cycle(true, true),
        vec![OrderBroker::Alpaca, OrderBroker::Kraken]
    );
    assert_eq!(
        OrderBroker::enabled_cycle(false, true),
        vec![OrderBroker::Kraken]
    );
    assert_eq!(
        OrderBroker::enabled_cycle(true, false),
        vec![OrderBroker::Alpaca]
    );
    assert!(OrderBroker::enabled_cycle(false, false).is_empty());
}

#[test]
fn chart_equity_merge_trusted_scale_follows_primary_broker() {
    // Two tradeable sources on DIFFERENT scales over the same buckets. Whichever
    // broker is primary defines the merged price scale (rank 0); the other only
    // fills buckets the primary lacks. ADR-126.
    let day = 86_400_000i64;
    let alpaca = vec![
        (1 * day, 10.0, 10.0, 10.0, 10.0, 100.0),
        (2 * day, 11.0, 11.0, 11.0, 11.0, 100.0),
    ];
    let kraken = vec![
        (2 * day, 22.0, 22.0, 22.0, 22.0, 100.0),
        (3 * day, 33.0, 33.0, 33.0, 33.0, 100.0),
    ];
    let sources: [(&str, &[(i64, f64, f64, f64, f64, f64)]); 2] =
        [("alpaca", &alpaca), ("kraken-equities", &kraken)];

    // Alpaca primary: the shared bucket (day2) takes Alpaca's 11.0, not Kraken's 22.0.
    let merged_alpaca =
        chart_merge_equity_raw_bars_with_primary("1Day", &sources, &[], OrderBroker::Alpaca);
    let alpaca_by_ts: std::collections::HashMap<i64, f64> =
        merged_alpaca.iter().map(|b| (b.ts_ms, b.close)).collect();
    assert_eq!(alpaca_by_ts[&(2 * day)], 11.0);

    // Kraken primary: the same shared bucket now takes Kraken's 22.0.
    let merged_kraken =
        chart_merge_equity_raw_bars_with_primary("1Day", &sources, &[], OrderBroker::Kraken);
    let kraken_by_ts: std::collections::HashMap<i64, f64> =
        merged_kraken.iter().map(|b| (b.ts_ms, b.close)).collect();
    assert_eq!(kraken_by_ts[&(2 * day)], 22.0);
}

#[test]
fn chart_equity_merge_preserves_old_assist_history_and_prefers_kraken_overlap() {
    let day = 86_400_000i64;
    let yahoo_old = vec![
        (1 * day, 1.0, 2.0, 0.5, 1.5, 10.0),
        (2 * day, 2.0, 3.0, 1.5, 2.5, 20.0),
        (3 * day, 3.0, 4.0, 2.5, 3.5, 30.0),
    ];
    let alpaca_mid = vec![
        (3 * day, 30.0, 31.0, 29.0, 30.5, 300.0),
        (4 * day, 4.0, 5.0, 3.5, 4.5, 40.0),
    ];
    let kraken_fresh = vec![
        (4 * day, 400.0, 401.0, 399.0, 400.5, 4000.0),
        (5 * day, 5.0, 6.0, 4.5, 5.5, 50.0),
    ];

    let merged = chart_merge_equity_raw_bars(
        "1Day",
        &[
            ("yahoo-chart", &yahoo_old),
            ("alpaca", &alpaca_mid),
            ("kraken-equities", &kraken_fresh),
        ],
        &[],
    );

    // Trusted overlap still prefers kraken > alpaca (day3=alpaca, day4=kraken).
    // Yahoo's older day1/day2 are preserved but BACK-ADJUSTED to the trusted
    // scale by the day3 ratio (30.5 / 3.5) so the splice is continuous instead
    // of dropping two decades from $3.5 to $30.5.
    assert_eq!(
        merged.iter().map(|b| b.ts_ms).collect::<Vec<_>>(),
        vec![day, 2 * day, 3 * day, 4 * day, 5 * day]
    );
    let splice_factor = 30.5 / 3.5;
    assert!((merged[0].close - 1.5 * splice_factor).abs() < 1e-6);
    assert!((merged[1].close - 2.5 * splice_factor).abs() < 1e-6);
    assert_eq!(merged[2].close, 30.5);
    assert_eq!(merged[3].close, 400.5);
    assert_eq!(merged[4].close, 5.5);
}

#[test]
fn chart_equity_merge_uses_adjusted_yahoo_for_multi_split_history() {
    // Alpaca/Kraken trusted history can be raw or only partially adjusted while
    // Yahoo/TradingView is split-adjusted across multiple reverse-split eras. Recent
    // bars agree; older eras are stable, MODEST few-fold steps. The merge adopts the
    // adjusted depth OHLC for those eras instead of dropping it as an inconsistent
    // splice. (A depth source that instead explodes the scale by orders of magnitude
    // — WOK/Yahoo's runaway back-adjustment — is refused by
    // `chart_depth_promotion_keeps_trusted_scale`; see
    // `chart_equity_merge_keeps_compact_trusted_scale_over_exploded_depth`.)
    let day = 86_400_000i64;
    // Trusted is stuck near the recent price and never reflects the older eras.
    let alpaca: Vec<(i64, f64, f64, f64, f64, f64)> = (1..=80)
        .map(|d| (d as i64 * day, 3.0, 3.3, 2.7, 3.0, 100.0))
        .collect();
    // Yahoo carries the real, adjusted multi-era history on a compact scale.
    let yahoo: Vec<(i64, f64, f64, f64, f64, f64)> = (1..=80)
        .map(|d| {
            let c = if d <= 20 {
                30.0
            } else if d <= 40 {
                10.0
            } else {
                3.0
            };
            (d as i64 * day, c, c * 1.1, c * 0.9, c, 50.0)
        })
        .collect();

    let merged =
        chart_merge_equity_raw_bars("1Day", &[("yahoo-chart", &yahoo), ("alpaca", &alpaca)], &[]);
    let by_ts: std::collections::HashMap<i64, f64> =
        merged.iter().map(|b| (b.ts_ms, b.close)).collect();

    assert!((by_ts[&(5 * day)] - 30.0).abs() < 1e-6);
    assert!((by_ts[&(30 * day)] - 10.0).abs() < 1e-6);
    assert!((by_ts[&(60 * day)] - 3.0).abs() < 1e-6);
    assert_eq!(merged.len(), 80);
}

#[test]
fn chart_equity_merge_keeps_compact_trusted_scale_over_exploded_depth() {
    // WOK reality: two 1-for-100 reverse splits, no kraken-equities source. Alpaca is
    // raw — compact, traded prices with a ×100 step at each split — while Yahoo is
    // back-adjusted across BOTH splits and so runs away to ~10,000× the recent price
    // in deep history. Yahoo's per-split eras are individually stable, which used to
    // make the depth-era reconciliation paste Yahoo's tens-of-thousands bars over
    // Alpaca's compact ones: the H1/H4 spikes, and a divergence from the compact
    // D1/W1 views. The trusted tier defines the scale (ADR-113); an exploded-scale
    // depth source must NOT redefine it, so the merge keeps the compact trusted
    // bars — identically across every timeframe.
    let day = 86_400_000i64;
    let n = 120i64;
    // Era A (deep) and Era B (between splits) are both raw ~0.02 on Alpaca; recent
    // (post both splits) is ~2.0. Yahoo back-adjusts A by ×10,000 and B by ×100.
    let alpaca: Vec<(i64, f64, f64, f64, f64, f64)> = (1..=n)
        .map(|d| {
            let c = if d <= 80 { 0.02 } else { 2.0 };
            (d * day, c, c * 1.05, c * 0.95, c, 100.0)
        })
        .collect();
    let yahoo: Vec<(i64, f64, f64, f64, f64, f64)> = (1..=n)
        .map(|d| {
            let c = if d <= 40 {
                20_000.0
            } else if d <= 80 {
                200.0
            } else {
                2.0
            };
            (d * day, c, c * 1.05, c * 0.95, c, 50.0)
        })
        .collect();

    let merged =
        chart_merge_equity_raw_bars("1Day", &[("yahoo-chart", &yahoo), ("alpaca", &alpaca)], &[]);
    assert!(
        merged.iter().all(|b| b.close <= 3.0),
        "exploded back-adjusted depth must not overwrite the compact trusted scale"
    );
    let by_ts: std::collections::HashMap<i64, f64> =
        merged.iter().map(|b| (b.ts_ms, b.close)).collect();
    assert!(
        (by_ts[&(20 * day)] - 0.02).abs() < 1e-9,
        "deep raw era kept"
    );
    assert!(
        (by_ts[&(60 * day)] - 0.02).abs() < 1e-9,
        "between-split raw era kept"
    );
    assert!(
        (by_ts[&(100 * day)] - 2.0).abs() < 1e-9,
        "recent bars unchanged"
    );
}

#[test]
fn chart_equity_merge_backadjusts_consistent_deep_history() {
    // Yahoo is deeper than Alpaca and uniformly 2× (a clean, constant offset —
    // an unadjusted but consistent split). The merge keeps Alpaca's range as-is
    // and PREPENDS Yahoo's older history back-adjusted by the overlap ratio so
    // the splice is continuous.
    let day = 86_400_000i64;
    let alpaca: Vec<(i64, f64, f64, f64, f64, f64)> = (10..=25)
        .map(|d| (d as i64 * day, 5.0, 5.5, 4.5, 5.0, 100.0))
        .collect();
    let yahoo: Vec<(i64, f64, f64, f64, f64, f64)> = (1..=25)
        .map(|d| (d as i64 * day, 10.0, 11.0, 9.0, 10.0, 50.0)) // 2× alpaca
        .collect();

    let merged =
        chart_merge_equity_raw_bars("1Day", &[("yahoo-chart", &yahoo), ("alpaca", &alpaca)], &[]);

    assert_eq!(merged.first().map(|b| b.ts_ms), Some(day)); // depth extends back
    assert_eq!(merged.len(), 25);
    // Prepended Yahoo bars rescaled by 0.5 → continuous with Alpaca's 5.0.
    assert!((merged[0].close - 5.0).abs() < 1e-6); // day1, Yahoo back-adjusted
    assert!((merged[8].close - 5.0).abs() < 1e-6); // day9, still Yahoo
    assert!((merged[9].close - 5.0).abs() < 1e-6); // day10, Alpaca
}

#[test]
fn chart_equity_merge_corrects_trusted_outlier_print_against_recent_corroborator() {
    // WOK 2026-06: Alpaca (trusted) momentarily doubled to ~2× on the last two
    // days while Yahoo and TradingView stayed flat. The depth tier only fills
    // gaps, so without a guard the bad trusted print is charted unchallenged and
    // pins the autoscale. The merge must replace the 2× bars with the
    // corroborated value — while still ignoring Yahoo's deep unadjusted region.
    let day = 86_400_000i64;
    let alpaca: Vec<(i64, f64, f64, f64, f64, f64)> = (1..=50)
        .map(|d| {
            let c = if d >= 49 { 0.20 } else { 0.10 }; // last two days: bad 2× print
            (d as i64 * day, c, c, c, c, 100.0)
        })
        .collect();
    let yahoo: Vec<(i64, f64, f64, f64, f64, f64)> = (1..=50)
        .map(|d| {
            let c = if d <= 10 { 1_000.0 } else { 0.10 }; // deep region unadjusted
            (d as i64 * day, c, c, c, c, 50.0)
        })
        .collect();

    let merged =
        chart_merge_equity_raw_bars("1Day", &[("yahoo-chart", &yahoo), ("alpaca", &alpaca)], &[]);

    let by_ts: std::collections::HashMap<i64, f64> =
        merged.iter().map(|b| (b.ts_ms, b.close)).collect();
    // The two doubled prints are pulled back to the ~0.10 consensus.
    assert!((by_ts[&(49 * day)] - 0.10).abs() < 1e-6, "day49 corrected");
    assert!((by_ts[&(50 * day)] - 0.10).abs() < 1e-6, "day50 corrected");
    // A normal earlier day is untouched, and no 1000× unadjusted bar leaks in.
    assert!((by_ts[&(40 * day)] - 0.10).abs() < 1e-6, "day40 untouched");
    assert!(
        merged.iter().all(|b| b.close < 1.0),
        "no unadjusted deep region may be spliced or scaled in"
    );
}

#[test]
fn chart_equity_merge_corrects_days_old_intraday_spike() {
    // An intraday bad trusted print days old must still be corrected against the
    // Yahoo corroborator. The old fixed 40-bucket window was only ~10 hours on
    // M15/H1, so a spike ~10 days back was never reached — the WOK M15 artifact.
    let hour = 3_600_000i64;
    let n = 300i64; // ~12.5 days of hourly bars
    let alpaca: Vec<(i64, f64, f64, f64, f64, f64)> = (0..n)
        .map(|i| {
            // Bad 2x print at index 50 (~10 days before the latest bar).
            let v = if i == 50 { 0.20 } else { 0.10 };
            (i * hour, v, v, v, v, 100.0)
        })
        .collect();
    let yahoo: Vec<(i64, f64, f64, f64, f64, f64)> = (0..n)
        .map(|i| (i * hour, 0.10, 0.10, 0.10, 0.10, 50.0))
        .collect();

    let merged = chart_merge_equity_raw_bars(
        "1Hour",
        &[("yahoo-chart", &yahoo), ("alpaca", &alpaca)],
        &[],
    );
    let by_ts: std::collections::HashMap<i64, f64> =
        merged.iter().map(|b| (b.ts_ms, b.close)).collect();
    assert!(
        (by_ts[&(50 * hour)] - 0.10).abs() < 1e-6,
        "days-old intraday spike must be corrected against the corroborator; got {}",
        by_ts[&(50 * hour)]
    );
}

#[test]
fn chart_equity_merge_prefers_adjusted_alpaca_over_raw_kraken_across_split() {
    // kraken-equities (rank 0) returns RAW xStock bars; Alpaca (rank 2) returns
    // split-adjusted bars. Across a reverse split the raw source out-ranks Alpaca
    // and would paint unadjusted pre-split history (the WOK December discontinuity).
    // The era-wide reconciliation must adopt the adjusted Alpaca bars there.
    let day = 86_400_000i64;
    let n = 150i64;
    // Days 1..=90 PRE-split: kraken RAW ~10.0 (100x), Alpaca adjusted ~0.10.
    // Days 91..=150 POST-split: both ~0.10 (agree).
    let kraken: Vec<(i64, f64, f64, f64, f64, f64)> = (1..=n)
        .map(|d| {
            let c = if d <= 90 { 10.0 } else { 0.10 };
            (d * day, c, c, c, c, 100.0)
        })
        .collect();
    let alpaca: Vec<(i64, f64, f64, f64, f64, f64)> = (1..=n)
        .map(|d| (d * day, 0.10, 0.10, 0.10, 0.10, 80.0))
        .collect();

    let merged = chart_merge_equity_raw_bars(
        "1Day",
        &[("kraken-equities", &kraken), ("alpaca", &alpaca)],
        &[],
    );
    let by_ts: std::collections::HashMap<i64, f64> =
        merged.iter().map(|b| (b.ts_ms, b.close)).collect();
    // Pre-split era is now on the adjusted (0.10) scale, continuous with post-split.
    assert!(
        (by_ts[&(10 * day)] - 0.10).abs() < 1e-6,
        "raw pre-split bars must be replaced by adjusted Alpaca; got {}",
        by_ts[&(10 * day)]
    );
    assert!(
        (by_ts[&(50 * day)] - 0.10).abs() < 1e-6,
        "mid pre-split era corrected; got {}",
        by_ts[&(50 * day)]
    );
    assert!(
        (by_ts[&(130 * day)] - 0.10).abs() < 1e-6,
        "post-split bars unchanged; got {}",
        by_ts[&(130 * day)]
    );
}

#[test]
fn known_split_back_adjusts_raw_kraken_equities_even_without_alpaca() {
    let day = 86_400_000i64;
    // Raw kraken-equities: pre-split (days 1–10) trade at 0.50, post-split (11–20)
    // at 50.0 — a 1-for-100 reverse split on day 11 lifts the price ×100. With NO
    // adjusted reference (no Alpaca/Yahoo), the era-inference path cannot fire; the
    // KNOWN split must still back-adjust the pre-split era onto the post-split scale.
    let kraken: Vec<(i64, f64, f64, f64, f64, f64)> = (1..=20i64)
        .map(|d| {
            let c = if d < 11 { 0.50 } else { 50.0 };
            (d * day, c, c, c, c, 1000.0)
        })
        .collect();
    let splits = [crate::app::chart::ChartSplit {
        ex_ts_ms: 11 * day,
        pre_split_factor: 100.0, // denominator/numerator = 100/1
    }];
    let merged = chart_merge_equity_raw_bars("1Day", &[("kraken-equities", &kraken)], &splits);
    let by_ts: std::collections::HashMap<i64, f64> =
        merged.iter().map(|b| (b.ts_ms, b.close)).collect();
    assert!(
        (by_ts[&(5 * day)] - 50.0).abs() < 1e-6,
        "pre-split bars must be lifted onto the post-split scale; got {}",
        by_ts[&(5 * day)]
    );
    assert!(
        (by_ts[&(15 * day)] - 50.0).abs() < 1e-6,
        "post-split bars unchanged; got {}",
        by_ts[&(15 * day)]
    );
}

#[test]
fn curated_known_splits_supply_wok_reverse_split() {
    use crate::app::chart::chart_curated_known_splits;
    // research_stock_splits can be empty / unsynced (free-tier FMP omits microcap
    // reverse splits); the curated fallback must still supply WOK's 1-for-100 so
    // the exact back-adjust fires — otherwise raw pre-split Kraken bars paint the
    // December discontinuity TradingView never shows.
    let splits = chart_curated_known_splits("wok"); // case-insensitive
    assert_eq!(splits.len(), 1, "WOK has one curated reverse split");
    assert!(
        (splits[0].pre_split_factor - 100.0).abs() < 1e-9,
        "1-for-100 reverse split → pre_split_factor 100"
    );
    let expected_ts = chrono::NaiveDate::from_ymd_opt(2025, 12, 29)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
        .timestamp_millis();
    assert_eq!(
        splits[0].ex_ts_ms, expected_ts,
        "ex-date 2025-12-29 00:00 UTC"
    );
    assert!(
        chart_curated_known_splits("AAPL").is_empty(),
        "curated table is opt-in per symbol"
    );

    // HUB Cyber Security — 1-for-20 reverse split (issuer-verified: effective
    // 2026-06-05 ET, Nasdaq split-adjusted trading 2026-06-08).
    let hubc = chart_curated_known_splits("hubc");
    assert_eq!(hubc.len(), 1, "HUBC has one curated reverse split");
    assert!(
        (hubc[0].pre_split_factor - 20.0).abs() < 1e-9,
        "1-for-20 reverse split → pre_split_factor 20"
    );
    let hubc_ex = chrono::NaiveDate::from_ymd_opt(2026, 6, 8)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
        .timestamp_millis();
    assert_eq!(hubc[0].ex_ts_ms, hubc_ex, "ex-date 2026-06-08 00:00 UTC");

    // End-to-end: the curated split, run through the same merge path as cached
    // FMP splits, lifts raw pre-split kraken-equities bars onto the post-split
    // scale (the fix for WOK rendering vs TradingView).
    let day = 86_400_000i64;
    let ex = splits[0].ex_ts_ms;
    let kraken: Vec<(i64, f64, f64, f64, f64, f64)> = (-6..=5i64)
        .map(|k| {
            let ts = ex + k * day;
            let c = if ts < ex { 0.50 } else { 50.0 }; // ×100 reverse split at ex
            (ts, c, c, c, c, 1000.0)
        })
        .collect();
    let merged = chart_merge_equity_raw_bars(
        "1Day",
        &[("kraken-equities", &kraken)],
        &chart_curated_known_splits("WOK"),
    );
    let by_ts: std::collections::HashMap<i64, f64> =
        merged.iter().map(|b| (b.ts_ms, b.close)).collect();
    assert!(
        (by_ts[&(ex - 3 * day)] - 50.0).abs() < 1e-6,
        "pre-split bars lifted ×100 onto post-split scale; got {}",
        by_ts[&(ex - 3 * day)]
    );
    assert!(
        (by_ts[&(ex + 2 * day)] - 50.0).abs() < 1e-6,
        "post-split bars unchanged; got {}",
        by_ts[&(ex + 2 * day)]
    );
}

#[test]
fn known_split_back_adjusts_raw_alpaca_trusted_source_without_kraken() {
    // HUBC shape: Alpaca (trusted, rank 2) served RAW bars across a 1-for-20
    // reverse split — no kraken-equities source, and a lone split era — so the
    // known split must lift Alpaca's pre-split history itself. The split day
    // also crashed (~ -60%, step only 8× not 20×) and the actual step lands two
    // sessions BEFORE the published ex-date, so the boundary is detected from
    // the bars, not the date, and the lift uses the published factor.
    let day = 86_400_000i64;
    let ex = 100 * day; // published ex-date
    let step = 96 * day; // Alpaca's actual scale step (2 sessions early)
    let factor = 20.0;
    let mut alpaca: Vec<(i64, f64, f64, f64, f64, f64)> = Vec::new();
    for d in 70..96i64 {
        // Pre-split low scale ~0.50, with a volatile spike to 2.00 that must be
        // lifted along with the rest (not misclassified by price level).
        let c = if d == 80 { 2.00 } else { 0.50 };
        alpaca.push((d * day, c, c, c, c, 1000.0));
    }
    for d in 96..110i64 {
        // Post-split high scale ~4.00 (0.50×20 then a same-day ~60% drop).
        let c = 4.00;
        alpaca.push((d * day, c, c, c, c, 1000.0));
    }
    let splits = [crate::app::chart::ChartSplit {
        ex_ts_ms: ex,
        pre_split_factor: factor,
    }];
    let merged = chart_merge_equity_raw_bars("1Day", &[("alpaca", &alpaca)], &splits);
    let by_ts: std::collections::HashMap<i64, f64> =
        merged.iter().map(|b| (b.ts_ms, b.close)).collect();
    assert!(
        (by_ts[&(72 * day)] - 10.0).abs() < 1e-6,
        "pre-split bars lifted ×20 onto post scale; got {}",
        by_ts[&(72 * day)]
    );
    assert!(
        (by_ts[&(80 * day)] - 40.0).abs() < 1e-6,
        "volatile pre-split spike lifted ×20 (boundary is time-based, not price); got {}",
        by_ts[&(80 * day)]
    );
    assert!(
        (by_ts[&step] - 4.0).abs() < 1e-6,
        "the detected step bar (pre-ex-date) is post-split, unchanged; got {}",
        by_ts[&step]
    );
    assert!(
        (by_ts[&(105 * day)] - 4.0).abs() < 1e-6,
        "post-split bars unchanged; got {}",
        by_ts[&(105 * day)]
    );
}

#[test]
fn known_split_leaves_already_adjusted_trusted_source_untouched() {
    // When Alpaca already applied the split (continuous series, no scale step),
    // the known split must NOT fire — otherwise we'd double-adjust the history.
    let day = 86_400_000i64;
    let ex = 100 * day;
    let mut alpaca: Vec<(i64, f64, f64, f64, f64, f64)> = Vec::new();
    for d in 70..110i64 {
        // Already-adjusted, continuous ~10.0 with only ordinary <2× moves.
        let c = if d == 96 { 12.0 } else { 10.0 };
        alpaca.push((d * day, c, c, c, c, 1000.0));
    }
    let splits = [crate::app::chart::ChartSplit {
        ex_ts_ms: ex,
        pre_split_factor: 20.0,
    }];
    let merged = chart_merge_equity_raw_bars("1Day", &[("alpaca", &alpaca)], &splits);
    let by_ts: std::collections::HashMap<i64, f64> =
        merged.iter().map(|b| (b.ts_ms, b.close)).collect();
    assert!(
        (by_ts[&(72 * day)] - 10.0).abs() < 1e-6,
        "continuous (already-adjusted) bars must stay put, not ×20; got {}",
        by_ts[&(72 * day)]
    );
}

#[test]
fn chart_persists_merged_equity_bars_under_merged_cache_key() {
    let db_path = std::env::temp_dir().join(format!(
        "typhoon-merged-cache-test-{}-{}.db",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    let cache = SqliteCache::open(&db_path).unwrap();
    let day = 86_400_000i64;
    let bars = vec![
        Bar {
            ts_ms: day,
            open: 1.0,
            high: 2.0,
            low: 0.5,
            close: 1.5,
            volume: 10.0,
        },
        Bar {
            ts_ms: 2 * day,
            open: 2.0,
            high: 3.0,
            low: 1.5,
            close: 2.5,
            volume: 20.0,
        },
    ];

    chart_persist_merged_equity_bars_to_cache(&cache, "WOK.EQ", "1Day", &bars).unwrap();

    let raw = cache
        .get_bars_raw("merged:WOK:1Day")
        .unwrap()
        .expect("merged cache key should exist");
    assert_eq!(raw.len(), 2);
    assert_eq!(raw[0].0, day);
    assert_eq!(raw[1].4, 2.5);
    let _ = std::fs::remove_file(db_path);
}

#[test]
fn chart_merged_source_bar_counts_reports_available_input_sources() {
    let db_path = std::env::temp_dir().join(format!(
        "typhoon-merged-source-counts-test-{}-{}.db",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    let cache = SqliteCache::open(&db_path).unwrap();
    cache
        .put_bars(
            "yahoo-chart:WOK:1Day",
            r#"[
                {"timestamp":"2024-01-01T00:00:00+00:00","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":10.0},
                {"timestamp":"2024-01-02T00:00:00+00:00","open":2.0,"high":3.0,"low":1.5,"close":2.5,"volume":20.0},
                {"timestamp":"2024-01-03T00:00:00+00:00","open":3.0,"high":4.0,"low":2.5,"close":3.5,"volume":30.0}
            ]"#,
        )
        .unwrap();
    cache
        .put_bars(
            "alpaca:WOK:1Day",
            r#"[
                {"timestamp":"2024-01-02T00:00:00+00:00","open":2.1,"high":3.1,"low":1.6,"close":2.6,"volume":21.0},
                {"timestamp":"2024-01-03T00:00:00+00:00","open":3.1,"high":4.1,"low":2.6,"close":3.6,"volume":31.0}
            ]"#,
        )
        .unwrap();

    let counts = chart_merged_source_bar_counts(&cache, "WOK.EQ", "1Day");
    assert_eq!(counts, vec![("yahoo-chart", 3), ("alpaca", 2)]);
    let _ = std::fs::remove_file(db_path);
}

#[test]
fn chart_source_override_loads_requested_provider_rows() {
    let db_path = std::env::temp_dir().join(format!(
        "typhoon-source-override-test-{}-{}.db",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    let cache = SqliteCache::open(&db_path).unwrap();
    cache
        .put_bars(
            "alpaca:WOK:1Day",
            r#"[
                {"timestamp":"2024-01-01T00:00:00+00:00","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":10.0},
                {"timestamp":"2024-01-02T00:00:00+00:00","open":2.0,"high":3.0,"low":1.5,"close":2.5,"volume":20.0}
            ]"#,
        )
        .unwrap();
    cache
        .put_bars(
            "yahoo-chart:WOK:1Day",
            r#"[
                {"timestamp":"2024-01-01T00:00:00+00:00","open":10.0,"high":20.0,"low":5.0,"close":15.0,"volume":100.0},
                {"timestamp":"2024-01-02T00:00:00+00:00","open":20.0,"high":30.0,"low":15.0,"close":25.0,"volume":200.0}
            ]"#,
        )
        .unwrap();

    let mut chart = ChartState::new("WOK.EQ", Timeframe::D1);
    chart.source_override = "yahoo-chart";
    let mut log = std::collections::VecDeque::new();

    assert!(chart.try_load(&cache, &mut log, None));
    assert_eq!(chart.source_override, "yahoo-chart");
    assert_eq!(chart.primary_source, "yahoo-chart");
    assert_eq!(chart.bars.len(), 2);
    assert_eq!(chart.bars[0].close, 15.0);

    let _ = std::fs::remove_file(db_path);
}

#[test]
fn prev_candle_levels_use_native_higher_timeframe_candles() {
    // PreviousCandleLevels.mqh reads iHigh(_Symbol, PERIOD_D1, 1) / PERIOD_W1 from
    // each timeframe's *own* series. The native refinement must mirror that:
    // previous = second-to-last native HTF bar, current = last native HTF bar — not
    // a re-aggregation of the host H1 chart bars, which for a 24/7 merged-source
    // xStock can miss the true weekly/daily high (the reported bug).
    use chrono::{Duration, TimeZone, Utc};
    let db_path = std::env::temp_dir().join(format!(
        "typhoon-prev-levels-native-test-{}-{}.db",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    let cache = SqliteCache::open(&db_path).unwrap();
    let base = Utc.with_ymd_and_hms(2026, 6, 1, 0, 0, 0).unwrap();
    let day = Duration::days(1);
    let week = Duration::weeks(1);

    // Native 1Day candles: highs 0.12 / 0.13 / 0.14 (closes ~0.10 to match host scale).
    let daily = serde_json::json!([
        {"timestamp": base.to_rfc3339(),         "open":0.10,"high":0.12,"low":0.090,"close":0.10,"volume":1.0},
        {"timestamp": (base+day).to_rfc3339(),   "open":0.10,"high":0.13,"low":0.095,"close":0.10,"volume":1.0},
        {"timestamp": (base+day*2).to_rfc3339(), "open":0.10,"high":0.14,"low":0.100,"close":0.10,"volume":1.0},
    ]);
    cache
        .put_bars("alpaca:WOK:1Day", &serde_json::to_string(&daily).unwrap())
        .unwrap();

    // Native 1Week candles: highs 0.15 / 0.16 / 0.17.
    let weekly = serde_json::json!([
        {"timestamp": base.to_rfc3339(),          "open":0.10,"high":0.15,"low":0.080,"close":0.10,"volume":1.0},
        {"timestamp": (base+week).to_rfc3339(),   "open":0.10,"high":0.16,"low":0.085,"close":0.10,"volume":1.0},
        {"timestamp": (base+week*2).to_rfc3339(), "open":0.10,"high":0.17,"low":0.090,"close":0.10,"volume":1.0},
    ]);
    cache
        .put_bars("alpaca:WOK:1Week", &serde_json::to_string(&weekly).unwrap())
        .unwrap();

    let mut chart = ChartState::new("WOK", Timeframe::H1);
    chart.primary_source = "alpaca";
    // Host H1 bars on the same (~0.10) scale so the HTF scale guard accepts the
    // native series. Their own highs (0.105) are intentionally *below* the true
    // daily/weekly highs to prove the levels come from the native HTF candles, not
    // from re-aggregating these host bars.
    for i in 0..5i64 {
        chart.bars.push(Bar {
            ts_ms: (base + Duration::hours(i)).timestamp_millis(),
            open: 0.10,
            high: 0.105,
            low: 0.095,
            close: 0.10,
            volume: 1.0,
        });
    }

    chart.compute_prev_candle_levels_native(&cache);

    let approx = |a: Option<f64>, b: f64| (a.unwrap() - b).abs() < 1e-9;
    // Daily: previous = 2nd-to-last native day, current = last native day.
    assert!(
        approx(chart.prev_daily_high, 0.13),
        "{:?}",
        chart.prev_daily_high
    );
    assert!(approx(chart.prev_daily_low, 0.095));
    assert!(approx(chart.current_daily_high, 0.14));
    assert!(approx(chart.current_daily_low, 0.100));
    // Weekly: previous = 2nd-to-last native week, current = last native week.
    assert!(
        approx(chart.prev_weekly_high, 0.16),
        "{:?}",
        chart.prev_weekly_high
    );
    assert!(approx(chart.prev_weekly_low, 0.085));
    assert!(approx(chart.current_weekly_high, 0.17));
    assert!(approx(chart.current_weekly_low, 0.090));

    let _ = std::fs::remove_file(db_path);
}

#[test]
fn chart_materializes_merged_equity_cache_from_provider_rows() {
    let db_path = std::env::temp_dir().join(format!(
        "typhoon-merged-materialize-test-{}-{}.db",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    let cache = SqliteCache::open(&db_path).unwrap();
    cache
        .put_bars(
            "yahoo-chart:WOK:1Day",
            r#"[
                {"timestamp":"2024-01-01T00:00:00+00:00","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":10.0},
                {"timestamp":"2024-01-02T00:00:00+00:00","open":2.0,"high":3.0,"low":1.5,"close":2.5,"volume":20.0}
            ]"#,
        )
        .unwrap();
    cache
        .put_bars(
            "kraken-equities:WOK:1Day",
            r#"[
                {"timestamp":"2024-01-02T00:00:00+00:00","open":20.0,"high":30.0,"low":15.0,"close":25.0,"volume":200.0},
                {"timestamp":"2024-01-03T00:00:00+00:00","open":3.0,"high":4.0,"low":2.5,"close":3.5,"volume":30.0}
            ]"#,
        )
        .unwrap();

    assert_eq!(
        chart_materialize_merged_equity_cache(&cache, "WOK.EQ", "1Day").unwrap(),
        3
    );

    let raw = cache
        .get_bars_raw("merged:WOK:1Day")
        .unwrap()
        .expect("merged cache key should exist");
    assert_eq!(raw.len(), 3);
    // 2024-01-01 exists only in Yahoo (older than the kraken-equities range), so
    // it is back-adjusted to the trusted scale by the 01-02 overlap ratio
    // (kraken 25.0 / yahoo 2.5 = 10×) for a continuous splice: 1.5 * 10 = 15.0.
    assert_eq!(raw[0].4, 15.0);
    assert_eq!(raw[1].4, 25.0); // 01-02 overlap: kraken-equities wins
    assert_eq!(raw[2].4, 3.5); // 01-03: kraken-equities
    let _ = std::fs::remove_file(db_path);
}

#[test]
fn chart_equity_merge_does_not_pin_to_stale_trusted_intraday_rows() {
    let db_path = std::env::temp_dir().join(format!(
        "typhoon-merged-stale-trusted-test-{}-{}.db",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    let cache = SqliteCache::open(&db_path).unwrap();
    let hour = 3_600_000i64;
    let stale_base = 1_609_459_200_000i64; // 2021-01-01T00:00:00Z
    let fresh_base = 1_767_225_600_000i64; // 2026-01-01T00:00:00Z

    let stale_trusted: Vec<serde_json::Value> = (0..32i64)
        .map(|i| {
            serde_json::json!({
                "timestamp": chrono::DateTime::from_timestamp_millis(stale_base + i * hour).unwrap().to_rfc3339(),
                "open": 0.60, "high": 0.70, "low": 0.50, "close": 0.65, "volume": 100.0
            })
        })
        .collect();
    cache
        .put_bars(
            "alpaca:BYND:1Hour",
            &serde_json::to_string(&stale_trusted).unwrap(),
        )
        .unwrap();

    let fresh_depth: Vec<serde_json::Value> = (0..32i64)
        .map(|i| {
            serde_json::json!({
                "timestamp": chrono::DateTime::from_timestamp_millis(fresh_base + i * hour).unwrap().to_rfc3339(),
                "open": 0.70, "high": 0.80, "low": 0.60, "close": 0.75, "volume": 1000.0
            })
        })
        .collect();
    cache
        .put_bars(
            "yahoo-chart:BYND:1Hour",
            &serde_json::to_string(&fresh_depth).unwrap(),
        )
        .unwrap();

    let merged = chart_load_merged_equity_bars_from_cache(&cache, "BYND", "1Hour");
    assert_eq!(merged.len(), 32);
    assert!(
        merged.first().unwrap().ts_ms >= fresh_base,
        "fresh Yahoo/current bars must win when the trusted intraday feed is years stale"
    );
    assert_eq!(merged.last().unwrap().close, 0.75);
    let _ = std::fs::remove_file(db_path);
}

#[test]
fn chart_equity_merge_4hour_corrects_wick_spike_via_synthesized_yahoo_corroborator() {
    // Yahoo has no native 4-hour interval, so the H4 merge had no independent
    // corroborator and a bad Alpaca print sailed through (the WOK H4 artifact).
    // The merge now aggregates cached Yahoo 1h bars into a 4h corroborator, and
    // the outlier check compares the high (not just the close) — so a lone wick
    // spike on the trusted 4h feed is pulled back to the corroborated value.
    use chrono::{TimeZone, Utc};
    let db_path = std::env::temp_dir().join(format!(
        "typhoon-merged-4h-corroborate-test-{}-{}.db",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    let cache = SqliteCache::open(&db_path).unwrap();

    let base = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
    let four_h = chrono::Duration::hours(4);
    let one_h = chrono::Duration::hours(1);
    let n: i64 = 60;

    // Alpaca (trusted) 4h bars: flat ~0.10, except the final bar carries a bad
    // wick to 0.30 (high only — open/close stay 0.10).
    let alpaca: Vec<serde_json::Value> = (0..n)
        .map(|i| {
            let high = if i == n - 1 { 0.30 } else { 0.11 };
            serde_json::json!({
                "timestamp": (base + four_h * i as i32).to_rfc3339(),
                "open": 0.10, "high": high, "low": 0.09, "close": 0.10, "volume": 100.0
            })
        })
        .collect();
    cache
        .put_bars("alpaca:WOK:4Hour", &serde_json::to_string(&alpaca).unwrap())
        .unwrap();

    // Yahoo 1h bars over the same span: clean ~0.10, no spike.
    let yahoo_hourly: Vec<serde_json::Value> = (0..n * 4)
        .map(|j| {
            serde_json::json!({
                "timestamp": (base + one_h * j as i32).to_rfc3339(),
                "open": 0.10, "high": 0.11, "low": 0.09, "close": 0.10, "volume": 25.0
            })
        })
        .collect();
    cache
        .put_bars(
            "yahoo-chart:WOK:1Hour",
            &serde_json::to_string(&yahoo_hourly).unwrap(),
        )
        .unwrap();

    let merged = chart_load_merged_equity_bars_from_cache(&cache, "WOK", "4Hour");
    let last = merged.last().expect("merged 4h bars should exist");
    assert!(
        last.high < 0.20,
        "the 0.30 wick on the final 4h bar must be corrected against the synthesized \
         Yahoo corroborator; got high={}",
        last.high
    );
    let _ = std::fs::remove_file(db_path);
}

#[test]
fn chart_equity_merge_1hour_self_heals_stalled_native_from_15min() {
    // A native 1-hour feed can stall for years (Alpaca's META 1Hour stopped
    // 2024-01-25) while the denser 15Min keeps printing to the current bar. The
    // merge derives 1Hour from the still-current 15Min of the same source and
    // unions it in: native wins every overlap bucket, derived fills the hole — so
    // the merged 1Hour is gapless and the MTF overlay no longer paints a flat
    // segment then a vertical catch-up.
    use chrono::{TimeZone, Utc};
    let db_path = std::env::temp_dir().join(format!(
        "typhoon-merged-1h-selfheal-test-{}-{}.db",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    let cache = SqliteCache::open(&db_path).unwrap();
    let base = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
    let hour = chrono::Duration::hours(1);
    let min = chrono::Duration::minutes(1);

    // Native 1Hour: hours 0..=20 then STOPS (close 100 to tell it apart).
    let native: Vec<serde_json::Value> = (0..=20i64)
        .map(|i| {
            serde_json::json!({
                "timestamp": (base + hour * i as i32).to_rfc3339(),
                "open": 100.0, "high": 101.0, "low": 99.0, "close": 100.0, "volume": 40.0
            })
        })
        .collect();
    cache
        .put_bars(
            "alpaca:HEAL:1Hour",
            &serde_json::to_string(&native).unwrap(),
        )
        .unwrap();

    // 15Min: dense hours 0..40 (4 bars/hour), close 50 so derived hours are distinct.
    let m15: Vec<serde_json::Value> = (0..40i64)
        .flat_map(|h| {
            (0..4i64).map(move |q| {
                serde_json::json!({
                    "timestamp": (base + hour * h as i32 + min * (q * 15) as i32).to_rfc3339(),
                    "open": 50.0, "high": 51.0, "low": 49.0, "close": 50.0, "volume": 10.0
                })
            })
        })
        .collect();
    cache
        .put_bars("alpaca:HEAL:15Min", &serde_json::to_string(&m15).unwrap())
        .unwrap();

    let merged = chart_load_merged_equity_bars_from_cache(&cache, "HEAL", "1Hour");
    let by_hour: std::collections::HashMap<i64, &Bar> = merged
        .iter()
        .map(|b| ((b.ts_ms - base.timestamp_millis()) / 3_600_000, b))
        .collect();

    // Gapless: every hour 0..40 present (was a 19-hour hole at hours 21..39).
    assert_eq!(
        merged.len(),
        40,
        "merged 1Hour should be gapless across 0..40"
    );
    assert!(
        by_hour.contains_key(&30),
        "hole hour 30 must be filled from 15Min"
    );
    // Overlap buckets keep the native bar; derived fills only the hole.
    assert_eq!(
        by_hour.get(&10).map(|b| b.close),
        Some(100.0),
        "native bar must win the overlap bucket"
    );
    assert_eq!(
        by_hour.get(&30).map(|b| b.close),
        Some(50.0),
        "hole bucket must come from the 15Min-derived rollup"
    );
    let _ = std::fs::remove_file(db_path);
}

#[test]
fn chart_loads_merged_weekly_from_corrected_daily_bars() {
    // Native weekly provider blobs can preserve stale/mis-adjusted OHLC across
    // corporate-action weeks. The user-facing Merged W1 chart should be the
    // weekly aggregation of the already-adjudicated daily Merged bars, matching
    // TradingView-style HTF construction instead of trusting a separate bad W1
    // provider row.
    use chrono::{TimeZone, Utc};
    let db_path = std::env::temp_dir().join(format!(
        "typhoon-merged-weekly-from-daily-test-{}-{}.db",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    let cache = SqliteCache::open(&db_path).unwrap();

    let day = chrono::Duration::days(1);
    let week = chrono::Duration::weeks(1);
    let base = Utc.with_ymd_and_hms(2026, 6, 1, 0, 0, 0).unwrap();

    let alpaca_daily: Vec<serde_json::Value> = (0..10i64)
        .map(|i| {
            let c = if i == 6 || i == 7 { 0.20 } else { 0.10 };
            serde_json::json!({
                "timestamp": (base + day * i as i32).to_rfc3339(),
                "open": c, "high": c, "low": c, "close": c, "volume": 100.0
            })
        })
        .collect();
    cache
        .put_bars(
            "alpaca:WOK:1Day",
            &serde_json::to_string(&alpaca_daily).unwrap(),
        )
        .unwrap();

    let yahoo_daily: Vec<serde_json::Value> = (0..10i64)
        .map(|i| {
            serde_json::json!({
                "timestamp": (base + day * i as i32).to_rfc3339(),
                "open": 0.10, "high": 0.11, "low": 0.09, "close": 0.10, "volume": 50.0
            })
        })
        .collect();
    cache
        .put_bars(
            "yahoo-chart:WOK:1Day",
            &serde_json::to_string(&yahoo_daily).unwrap(),
        )
        .unwrap();

    let bad_weekly: Vec<serde_json::Value> = (0..2i64)
        .map(|i| {
            serde_json::json!({
                "timestamp": (base + week * i as i32).to_rfc3339(),
                "open": 0.20, "high": 0.40, "low": 0.18, "close": 0.20, "volume": 500.0
            })
        })
        .collect();
    cache
        .put_bars(
            "alpaca:WOK:1Week",
            &serde_json::to_string(&bad_weekly).unwrap(),
        )
        .unwrap();

    let weekly = chart_load_merged_equity_bars_from_cache(&cache, "WOK", "1Week");
    assert_eq!(weekly.len(), 2);
    assert!(
        weekly.iter().all(|bar| bar.close < 0.12 && bar.high < 0.12),
        "weekly Merged bars must come from corrected daily bars, not stale native W1 rows: {:?}",
        weekly.iter().map(|b| b.close).collect::<Vec<_>>()
    );
    let _ = std::fs::remove_file(db_path);
}

#[test]
fn chart_merged_equity_low_timeframes_ignore_provider_assist_rows() {
    use chrono::TimeZone;

    let db_path = std::env::temp_dir().join(format!(
        "typhoon-merged-lowtf-assist-test-{}-{}.db",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    let cache = SqliteCache::open(&db_path).unwrap();
    let base = chrono::Utc
        .with_ymd_and_hms(2026, 6, 12, 13, 30, 0)
        .unwrap();
    let minute = chrono::Duration::minutes(1);

    let assist_bars: Vec<serde_json::Value> = (0..20i64)
        .map(|i| {
            serde_json::json!({
                "timestamp": (base + minute * i as i32).to_rfc3339(),
                "open": 0.10, "high": 0.11, "low": 0.09, "close": 0.10, "volume": 100.0
            })
        })
        .collect();
    cache
        .put_bars(
            "alpaca:WOK:1Min",
            &serde_json::to_string(&assist_bars).unwrap(),
        )
        .unwrap();
    cache
        .put_bars(
            "yahoo-chart:WOK:1Min",
            &serde_json::to_string(&assist_bars).unwrap(),
        )
        .unwrap();

    let merged = chart_load_merged_equity_bars_from_cache(&cache, "WOK", "1Min");
    assert!(
        merged.is_empty(),
        "equity M1 merged charts must not resurrect stale Alpaca/Yahoo assist rows"
    );

    let kraken_bars: Vec<serde_json::Value> = (0..20i64)
        .map(|i| {
            serde_json::json!({
                "timestamp": (base + minute * i as i32).to_rfc3339(),
                "open": 0.20, "high": 0.21, "low": 0.19, "close": 0.20, "volume": 50.0
            })
        })
        .collect();
    cache
        .put_bars(
            "kraken-equities:WOK:1Min",
            &serde_json::to_string(&kraken_bars).unwrap(),
        )
        .unwrap();

    let merged = chart_load_merged_equity_bars_from_cache(&cache, "WOK", "1Min");
    assert_eq!(merged.len(), 20);
    assert!(merged.iter().all(|bar| (bar.close - 0.20).abs() < 1e-9));
    let _ = std::fs::remove_file(db_path);
}

#[test]
fn chart_mtf_overlays_load_from_merged_cache_rows() {
    let db_path = std::env::temp_dir().join(format!(
        "typhoon-merged-mtf-overlay-test-{}-{}.db",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    let cache = SqliteCache::open(&db_path).unwrap();
    let hour = 3_600_000i64;
    let mut raw = Vec::new();
    for i in 0..220i64 {
        let close = 10.0 + i as f64 * 0.01;
        raw.push(Bar {
            ts_ms: i * hour,
            open: close - 0.1,
            high: close + 0.2,
            low: close - 0.2,
            close,
            volume: 1000.0 + i as f64,
        });
    }
    chart_persist_merged_equity_bars_to_cache(&cache, "WOK.EQ", "1Hour", &raw).unwrap();

    let mut chart = ChartState::new("WOK.EQ", Timeframe::H4);
    chart.bars = raw
        .iter()
        .step_by(4)
        .map(|bar| Bar {
            ts_ms: bar.ts_ms,
            open: bar.open,
            high: bar.high,
            low: bar.low,
            close: bar.close,
            volume: bar.volume,
        })
        .collect();

    assert_eq!(cache_source_from_key("merged:WOK:1Hour"), "merged");
    chart.compute_mtf_sma(&cache);
    chart.compute_multi_kama(&cache);

    assert!(
        chart
            .mtf_sma
            .iter()
            .any(|(label, points)| label == "H1 200" && !points.is_empty()),
        "MTF 200SMA should load from merged:WOK:1Hour"
    );
    assert!(
        chart
            .multi_kama
            .iter()
            .any(|(label, points)| label == "H1" && !points.is_empty()),
        "MultiKAMA should load from merged:WOK:1Hour"
    );
    let _ = std::fs::remove_file(db_path);
}

#[test]
fn chart_ensures_mql_mtf_overlays_for_render_from_cache() {
    let db_path = std::env::temp_dir().join(format!(
        "typhoon-render-mtf-overlay-test-{}-{}.db",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    let cache = SqliteCache::open(&db_path).unwrap();
    let hour = 3_600_000i64;
    let mut raw = Vec::new();
    for i in 0..260i64 {
        let close = 10.0 + i as f64 * 0.01;
        raw.push(Bar {
            ts_ms: i * hour,
            open: close - 0.1,
            high: close + 0.2,
            low: close - 0.2,
            close,
            volume: 1000.0 + i as f64,
        });
    }
    chart_persist_merged_equity_bars_to_cache(&cache, "WOK.EQ", "1Hour", &raw).unwrap();

    let mut chart = ChartState::new("WOK", Timeframe::H1);
    chart.bars = raw;
    assert!(chart.mtf_sma.is_empty());
    assert!(chart.multi_kama.is_empty());

    chart.ensure_mql_mtf_overlays_for_render(&cache, true, true);

    assert!(
        chart
            .mtf_sma
            .iter()
            .any(|(label, points)| label == "H1 200" && !points.is_empty()),
        "render guard must populate MTF_MA before fallback SMA200 is drawn"
    );
    assert!(
        chart
            .multi_kama
            .iter()
            .any(|(label, points)| label == "H1" && !points.is_empty()),
        "render guard must populate MultiKAMA before fallback KAMA is drawn"
    );
    let _ = std::fs::remove_file(db_path);
}

#[test]
fn heavy_sync_mtf_overlay_render_policy_ensures_all_mtf_cells() {
    // Non-heavy MTF cell (any focus): always ensure
    assert!(ChartState::should_ensure_mql_mtf_overlays_for_render(
        false, true, false
    ));
    // Heavy, non-MTF: the test previously expected true for this combo (legacy coverage)
    assert!(ChartState::should_ensure_mql_mtf_overlays_for_render(
        true, false, false
    ));
    // Heavy MTF focused: ensure
    assert!(ChartState::should_ensure_mql_mtf_overlays_for_render(
        true, true, true
    ));
    // Heavy MTF *non-focused*: now true (the fix). Overlays must be populated for
    // background MTF grid cells at launch so users don't have to click each cell.
    assert!(ChartState::should_ensure_mql_mtf_overlays_for_render(
        true, true, false
    ));
}

#[test]
fn chart_forming_bar_requires_caught_up_previous_bucket() {
    let day = 86_400_000i64;
    let now = 20 * day + 23 * 3_600_000;
    assert!(chart_forming_bar_allowed(19 * day, now, day));
    assert!(!chart_forming_bar_allowed(14 * day, now, day));
}

#[test]
fn fundamentals_scrape_progress_log_is_milestoned() {
    assert!(should_emit_fundamentals_scrape_progress(1, 12_187));
    assert!(should_emit_fundamentals_scrape_progress(10, 12_187));
    assert!(!should_emit_fundamentals_scrape_progress(11, 12_187));
    assert!(!should_emit_fundamentals_scrape_progress(99, 12_187));
    assert!(should_emit_fundamentals_scrape_progress(100, 12_187));
    assert!(should_emit_fundamentals_scrape_progress(12_187, 12_187));
}

#[test]
fn fundamentals_provider_coverage_gaps_are_non_actionable() {
    assert!(is_fundamentals_provider_coverage_gap(
        "Yahoo returned 404 Not Found for TRAD.U"
    ));
    assert!(is_fundamentals_provider_coverage_gap(
        "Yahoo returned 400 Bad Request for BAD"
    ));
    assert!(is_fundamentals_provider_coverage_gap(
        "No Yahoo data for SPAC.U"
    ));
    assert!(!is_fundamentals_provider_coverage_gap("DB lock failed"));
}

#[test]
fn fundamentals_scrape_symbol_filter_rejects_crypto_before_provider_calls() {
    assert_eq!(
        normalize_fundamentals_scrape_symbol("AAPL"),
        Some("AAPL".to_string())
    );
    assert_eq!(
        normalize_fundamentals_scrape_symbol("WOK.EQ"),
        Some("WOK".to_string())
    );
    assert_eq!(normalize_fundamentals_scrape_symbol("BABY"), None);
    assert_eq!(normalize_fundamentals_scrape_symbol("BABY.EQ"), None);
    assert_eq!(normalize_fundamentals_scrape_symbol("BTC/USD"), None);
}

#[test]
fn yahoo_extended_quote_state_filter_blocks_regular_session() {
    assert!(yahoo_market_state_allows_extended_quote("PRE"));
    assert!(yahoo_market_state_allows_extended_quote("post"));
    assert!(!yahoo_market_state_allows_extended_quote("REGULAR"));
    assert!(!yahoo_market_state_allows_extended_quote("CLOSED"));
    assert!(!yahoo_market_state_allows_extended_quote(""));
}

#[test]
fn yahoo_extended_quote_time_filter_rejects_stale_ext_ticks() {
    assert!(yahoo_extended_quote_time_is_fresh(200, 100));
    assert!(yahoo_extended_quote_time_is_fresh(100, 100));
    assert!(!yahoo_extended_quote_time_is_fresh(99, 100));
    assert!(!yahoo_extended_quote_time_is_fresh(0, 100));
}

#[test]
fn watchlist_cache_fallback_prioritizes_kraken_equities_for_stocks() {
    assert_eq!(
        watchlist_cache_fallback_sources("WOK")[0],
        "kraken-equities"
    );
    assert_eq!(
        watchlist_cache_fallback_sources("TNDM")[0],
        "kraken-equities"
    );
    assert_eq!(watchlist_cache_fallback_sources("BTCUSD")[0], "kraken");
}

#[test]
fn kraken_pair_asset_class_identifies_spot_fx() {
    assert_eq!(kraken_pair_asset_class("ZEURZUSD", "EUR/USD"), "fx");
    assert_eq!(kraken_pair_asset_class("AUDJPY", "AUD/JPY"), "fx");
    assert_eq!(TyphooNApp::kraken_symbol_sector("EURUSD"), 2);
    assert_eq!(TyphooNApp::kraken_symbol_quote("EURUSD"), Some("USD"));
    assert!(TyphooNApp::kraken_spot_sector_scrape_enabled_from_flags(
        2, false, true, false, false, false, false, false, false, false, false, false, false,
    ));
    assert_eq!(kraken_pair_asset_class("XBTUSD", "BTC/USD"), "crypto");
    // A crypto/EUR pair reads as EUR-quoted, so the global quote filter (and the WS
    // firehose / cache prune that share kraken_pair_quote_disabled) exclude it when
    // EUR is disabled — even while USD stays enabled.
    assert_eq!(TyphooNApp::kraken_symbol_quote("ADXEUR"), Some("EUR"));
}

#[test]
fn kraken_xstock_detection_does_not_strip_crypto_x_suffixes() {
    assert_eq!(
        kraken_xstock_fundamental_symbol("AVAXUSD", "AVAX/USD"),
        None
    );
    assert_eq!(
        kraken_xstock_fundamental_symbol("FLUXUSD", "FLUX/USD"),
        None
    );
    assert_eq!(
        kraken_xstock_fundamental_symbol("HRTX.EQUSD", "HRTX.EQ/USD"),
        Some("HRTX".to_string())
    );
}

#[test]
fn kraken_equity_balances_use_bare_underlying_symbol() {
    // Market-data / price-cache key: bare underlying ticker.
    assert_eq!(
        TyphooNApp::kraken_spot_pair_for_balance_asset("WOK.EQ"),
        "WOK"
    );
    assert_eq!(
        TyphooNApp::kraken_spot_pair_for_balance_asset("HRTX.EQ"),
        "HRTX"
    );
    assert_eq!(
        TyphooNApp::kraken_spot_pair_for_balance_asset("XXBT"),
        "BTCUSD"
    );
}

#[test]
fn kraken_equity_order_pair_construction_uses_xstock_form_not_bare_or_equsd() {
    // Construction fallback (catalog miss). The bare ticker `WOK` is an unknown
    // Spot pair, and the earlier `WOK.EQUSD` (taken from a TradesHistory sample)
    // is ALSO rejected by AddOrder — so the fallback is the app's tradeable xStock
    // form `{TICKER}x/USD`, the same `{SYM}x/USD` the WS book/OHLC use. The live
    // path (`kraken_resolved_order_pair_for_balance_asset`) prefers the real
    // AssetPairs catalog wsname over this when one exists.
    assert_eq!(
        TyphooNApp::kraken_order_pair_for_balance_asset("WOK.EQ"),
        "WOKx/USD"
    );
    assert_eq!(
        TyphooNApp::kraken_order_pair_for_balance_asset("HRTX.EQ"),
        "HRTXx/USD"
    );
    // Crypto is unaffected: still `{DISPLAY}USD`.
    assert_eq!(
        TyphooNApp::kraken_order_pair_for_balance_asset("XXBT"),
        "BTCUSD"
    );
}

#[test]
fn kraken_pair_base_ticker_peels_tokenized_and_eq_markers() {
    // The catalog matcher must reduce every tradeable form to the bare ticker so a
    // balance like `ADTX.EQ` finds whatever Kraken actually lists for ADTX.
    assert_eq!(TyphooNApp::kraken_pair_base_ticker("ADTXx/USD"), "ADTX");
    assert_eq!(TyphooNApp::kraken_pair_base_ticker("WOK.EQ/USD"), "WOK");
    assert_eq!(TyphooNApp::kraken_pair_base_ticker("XBT/USD"), "XBT");
}

#[test]
fn alpaca_risk_balance_uses_current_equity_not_previous_equity() {
    let acct = AccountInfo {
        equity: 102_500.0,
        cash: 40_000.0,
        buying_power: 205_000.0,
        portfolio_value: 102_500.0,
        initial_margin: 25_000.0,
        maintenance_margin: 12_500.0,
        currency: "USD".to_string(),
        pattern_day_trader: false,
        trading_blocked: false,
        last_equity: 99_500.0,
        balance: 99_500.0,
    };

    assert_eq!(TyphooNApp::alpaca_current_risk_balance(&acct), 102_500.0);
}

#[test]
fn alpaca_account_pl_matches_sum_of_position_panel_pl() {
    let lumn = PositionInfo {
        symbol: "LUMN".to_string(),
        qty: 100.0,
        qty_available: 100.0,
        side: "long".to_string(),
        avg_entry_price: 5.00,
        market_value: 550.0,
        unrealized_pl: 50.0,
        asset_class: "stock".to_string(),
        asset_id: "lumn".to_string(),
    };
    let wen = PositionInfo {
        symbol: "WEN".to_string(),
        qty: 50.0,
        qty_available: 50.0,
        side: "long".to_string(),
        avg_entry_price: 10.00,
        market_value: 450.0,
        unrealized_pl: -50.0,
        asset_class: "stock".to_string(),
        asset_id: "wen".to_string(),
    };

    let lumn_pl = super::app_runtime_right_panel_positions::position_unrealized_pl_from_price(
        &lumn,
        Some(5.50),
    );
    let wen_pl = super::app_runtime_right_panel_positions::position_unrealized_pl_from_price(
        &wen,
        Some(9.00),
    );
    let total_basis = super::app_runtime_right_panel_positions::position_cost_basis(&lumn)
        + super::app_runtime_right_panel_positions::position_cost_basis(&wen);

    assert_eq!(lumn_pl, 50.0);
    assert_eq!(wen_pl, -50.0);
    assert_eq!(lumn_pl + wen_pl, 0.0);
    assert_eq!(total_basis, 1_000.0);
}

// ── parse_ask_args (ASKAI/ASKCLAUDE/ASKGEMINI argument parser) ───────────

// Note: handle_command() has already uppercased the input by the time
// parse_ask_args is called, so these tests pass uppercase input to match.

#[test]
fn parse_ask_args_single_symbol_only() {
    let (syms, q) = TyphooNApp::parse_ask_args("CC");
    assert_eq!(syms, vec!["CC".to_string()]);
    assert!(q.is_empty());
}

#[test]
fn parse_ask_args_comma_symbols_only() {
    let (syms, q) = TyphooNApp::parse_ask_args("CC,NCLH");
    assert_eq!(syms, vec!["CC".to_string(), "NCLH".to_string()]);
    assert!(q.is_empty());
}

#[test]
fn parse_ask_args_question_is_not_treated_as_tickers() {
    // Regression: the entire uppercased question used to end up as "tickers"
    // because handle_command upper-cases input before parse_ask_args sees it.
    let (syms, q) = TyphooNApp::parse_ask_args(
        "CC,NCLH WHAT IS YOUR OPINION FUNDAMENTALLY OF THESE SYMBOLS WITH ALL AVAILABLE",
    );
    assert_eq!(syms, vec!["CC".to_string(), "NCLH".to_string()]);
    assert_eq!(
        q,
        "WHAT IS YOUR OPINION FUNDAMENTALLY OF THESE SYMBOLS WITH ALL AVAILABLE"
    );
}

#[test]
fn parse_ask_args_single_symbol_with_question() {
    let (syms, q) = TyphooNApp::parse_ask_args("CC WHAT IS THE OUTLOOK");
    assert_eq!(syms, vec!["CC".to_string()]);
    assert_eq!(q, "WHAT IS THE OUTLOOK");
}

#[test]
fn parse_ask_args_dedupes_symbols() {
    let (syms, _) = TyphooNApp::parse_ask_args("CC,NCLH,CC");
    assert_eq!(syms, vec!["CC".to_string(), "NCLH".to_string()]);
}

#[test]
fn parse_ask_args_empty_input() {
    let (syms, q) = TyphooNApp::parse_ask_args("");
    assert!(syms.is_empty());
    assert!(q.is_empty());
}

#[test]
fn packet_export_stem_sanitizes_path_unsafe_chars() {
    // '/', '.', '+' are valid in tickers (is_tickerish) but unsafe in a
    // filename component — EXPORT_PACKET collapses them to '_'.
    assert_eq!(
        TyphooNApp::packet_export_stem(&["BTC/USD".to_string(), "BRK.B".to_string()]),
        "BTC_USD_BRK_B"
    );
    assert_eq!(
        TyphooNApp::packet_export_stem(&["CC".to_string(), "NCLH".to_string()]),
        "CC_NCLH"
    );
    assert_eq!(
        TyphooNApp::packet_export_stem(&["BHP.AX".to_string()]),
        "BHP_AX"
    );
}

#[test]
fn gemini_cli_default_prefers_3_1_pro_preview() {
    assert_eq!(
        TyphooNApp::default_gemini_cli_model(),
        "gemini-3.1-pro-preview"
    );
}

#[test]
fn gemini_cli_model_options_include_cli_valid_set() {
    let models: Vec<&str> = TyphooNApp::gemini_cli_model_options()
        .iter()
        .map(|(model, _label)| *model)
        .collect();
    for expected in [
        "auto",
        "pro",
        "flash",
        "flash-lite",
        "gemini-3.1-pro-preview",
        "gemini-3.1-pro-preview-customtools",
        "gemini-3.1-flash-lite-preview",
        "gemini-3-pro-preview",
        "gemini-3-flash-preview",
        "gemini-2.5-pro",
        "gemini-2.5-flash",
        "gemini-2.5-flash-lite",
        "gemma-4-31b-it",
        "gemma-4-26b-a4b-it",
    ] {
        assert!(
            models.contains(&expected),
            "missing Gemini CLI model {expected}"
        );
    }
}

#[test]
fn gemini_cli_json_response_appends_usage_stats() {
    let stdout = r#"{
          "response": "OK",
          "stats": {
            "models": {
              "gemini-2.5-flash": {
                "tokens": {
                  "prompt": 100,
                  "candidates": 7,
                  "total": 125,
                  "cached": 80,
                  "thoughts": 18
                }
              }
            }
          }
        }"#;
    let parsed = TyphooNApp::gemini_cli_json_response(stdout).unwrap();
    assert!(parsed.starts_with("OK"));
    assert!(parsed.contains("model=gemini-2.5-flash"));
    assert!(parsed.contains("total_tokens=125"));
    assert!(parsed.contains("Remaining quota is not exposed"));
}

#[test]
fn gemini_cli_json_response_preserves_error_message() {
    let stdout = r#"{"error":{"message":"Requested entity was not found."}}"#;
    assert_eq!(
        TyphooNApp::gemini_cli_json_response(stdout).unwrap(),
        "Error: Requested entity was not found."
    );
}

#[test]
fn parse_ask_args_preserves_special_chars_in_tickers() {
    // BRK.B, RDS-A, BTC-USD all need to survive the tickerish check.
    let (syms, _) = TyphooNApp::parse_ask_args("BRK.B,RDS-A,BTC-USD");
    assert_eq!(
        syms,
        vec![
            "BRK.B".to_string(),
            "RDS-A".to_string(),
            "BTC-USD".to_string()
        ]
    );
}

#[test]
fn normalize_market_data_symbol_strips_exchange_suffixes_only() {
    assert_eq!(normalize_market_data_symbol("AAPL.US"), "AAPL");
    assert_eq!(normalize_market_data_symbol("BMW.DE"), "BMW");
    assert_eq!(normalize_market_data_symbol("BRK.B"), "BRK.B");
    assert_eq!(normalize_market_data_symbol("BTC/USD"), "BTC/USD");
}

/// Create synthetic test bars (ascending prices).
fn make_bars(n: usize) -> Vec<Bar> {
    (0..n)
        .map(|i| {
            let base = 100.0 + i as f64;
            Bar {
                ts_ms: 1700000000000 + i as i64 * 3600000,
                open: base,
                high: base + 2.0,
                low: base - 1.0,
                close: base + 1.0,
                volume: 1000.0 + i as f64 * 10.0,
            }
        })
        .collect()
}

/// Create bars with known pattern for oscillator tests.
fn make_oscillating_bars(n: usize) -> Vec<Bar> {
    (0..n)
        .map(|i| {
            let base = 100.0 + (i as f64 * 0.1).sin() * 10.0;
            Bar {
                ts_ms: 1700000000000 + i as i64 * 3600000,
                open: base - 0.5,
                high: base + 1.0,
                low: base - 1.0,
                close: base + 0.5,
                volume: 500.0 + (i as f64 * 0.3).cos().abs() * 1000.0,
            }
        })
        .collect()
}

fn make_close_bars(closes: &[f64]) -> Vec<Bar> {
    closes
        .iter()
        .enumerate()
        .map(|(i, close)| Bar {
            ts_ms: 1700000000000 + i as i64 * 3600000,
            open: *close,
            high: *close * 1.01,
            low: *close * 0.99,
            close: *close,
            volume: 1000.0,
        })
        .collect()
}

#[test]
fn mtf_overlay_drops_misscaled_intraday_source_but_keeps_lagging_average() {
    // YI [W1]: the intraday HTF source was unadjusted across a reverse split (~10×
    // the adjusted daily/weekly in the pre-split era, with no intraday corroborator
    // to correct it), so the H1/H4 MTF_MA + MultiKAMA lines plateaued ~10× above
    // price. The bar-level guard drops such a source. A clean higher-TF whose
    // *lagging* average rides far above a crashed price must be KEPT — its bars are
    // on-scale; only the projected SMA lags (ADR-123).
    let day = 86_400_000i64;
    // Host weekly candles: a clean −90% decline 10 → 1.
    let host: Vec<Bar> = (0..120i64)
        .map(|i| {
            let c = 10.0 - (i as f64 / 119.0) * 9.0;
            Bar {
                ts_ms: i * day,
                open: c,
                high: c * 1.02,
                low: c * 0.98,
                close: c,
                volume: 100.0,
            }
        })
        .collect();

    // A clean source on the same scale is kept even though a 200-period SMA of it
    // would lag far above the recent low (the bars themselves are on-scale).
    assert!(ChartState::htf_source_matches_host_scale(&host, &host));

    // A uniformly offset-but-consistent source (within tolerance everywhere) is kept.
    let shifted: Vec<Bar> = host
        .iter()
        .map(|b| Bar {
            close: b.close * 1.5,
            ..*b
        })
        .collect();
    assert!(ChartState::htf_source_matches_host_scale(&host, &shifted));

    // A mis-scaled era — the first 40% of bars sit ~10× high (unadjusted), the rest
    // match — is dropped.
    let misscaled: Vec<Bar> = host
        .iter()
        .enumerate()
        .map(|(i, b)| {
            let f = if i < 48 { 10.0 } else { 1.0 };
            Bar {
                open: b.open * f,
                high: b.high * f,
                low: b.low * f,
                close: b.close * f,
                ..*b
            }
        })
        .collect();
    assert!(!ChartState::htf_source_matches_host_scale(
        &host, &misscaled
    ));
}

// ── SMA Intelligence (ADR-131) ───────────────────────────────────────────

#[test]
fn sma_intelligence_command_is_registered() {
    let cmd = crate::app::commands::COMMANDS
        .iter()
        .find(|c| c.name == "SMA_INTELLIGENCE")
        .expect("SMA_INTELLIGENCE must be in the palette");
    assert!(
        cmd.desc.to_lowercase().contains("outfit"),
        "description should name the SMA-outfit concept: {}",
        cmd.desc
    );
}

#[test]
fn sma_default_outfits_round_trip_through_the_spec_parser() {
    // The session-restore path re-validates persisted outfits through
    // parse_outfit_spec; the shipped defaults must always survive it.
    for outfit in typhoon_chart_ui::sma_outfits::default_sma_outfits() {
        let spec = typhoon_chart_ui::sma_outfits::outfit_label(&outfit);
        assert_eq!(
            typhoon_chart_ui::sma_outfits::parse_outfit_spec(&spec),
            Some(outfit),
            "default outfit {spec} failed re-validation"
        );
    }
}

#[test]
fn live_tick_anchor_clamps_only_gross_divergence_on_newest_bar() {
    use crate::app::chart::chart_live_tick_anchor_guard;
    let mk = |c: f64| Bar {
        ts_ms: 0,
        open: c,
        high: c * 1.01,
        low: c * 0.99,
        close: c,
        volume: 1.0,
    };
    // 2x bad print vs a live mid of 4.20 → clamped, wick geometry preserved.
    let mut bars = vec![mk(4.20), mk(8.40)];
    let div = chart_live_tick_anchor_guard(&mut bars, 4.20);
    assert!(div.is_some());
    assert!((bars[1].close - 4.20).abs() < 1e-9);
    assert!((bars[1].high / bars[1].close - 1.01).abs() < 1e-6);
    // Older bars untouched.
    assert!((bars[0].close - 4.20).abs() < 1e-9);

    // Ordinary intraday moves (<1.5x) never clamp.
    let mut ok_bars = vec![mk(4.20)];
    assert!(chart_live_tick_anchor_guard(&mut ok_bars, 4.90).is_none());
    assert!((ok_bars[0].close - 4.20).abs() < 1e-9);

    // Bad inputs never clamp.
    assert!(chart_live_tick_anchor_guard(&mut ok_bars, 0.0).is_none());
    assert!(chart_live_tick_anchor_guard(&mut [], 4.2).is_none());
}
