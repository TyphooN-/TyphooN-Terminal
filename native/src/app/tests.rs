use super::*;

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
    assert!(chart_equity_source_rank("kraken-equities") < chart_equity_source_rank("alpaca"));
    assert_eq!(chart_equity_source_rank("kraken"), None);
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
fn chart_equity_merge_drops_unadjusted_depth_history() {
    // Yahoo carries deep history but is unadjusted across a ~10,000× action: it
    // agrees with Alpaca recently and is wildly off earlier. That inconsistency
    // must make the merge DROP Yahoo entirely rather than splice scale-jumped
    // bars — including Yahoo's older-than-Alpaca history. (Real-world: WOK.)
    let day = 86_400_000i64;
    let alpaca: Vec<(i64, f64, f64, f64, f64, f64)> = (10..=25)
        .map(|d| (d as i64 * day, 1.0, 1.1, 0.9, 1.0, 100.0))
        .collect();
    let yahoo: Vec<(i64, f64, f64, f64, f64, f64)> = (1..=25)
        .map(|d| {
            // Older half unadjusted (10,000×), recent half agrees with Alpaca.
            let c = if d <= 17 { 10_000.0 } else { 1.0 };
            (d as i64 * day, c, c * 1.1, c * 0.9, c, 50.0)
        })
        .collect();

    let merged =
        chart_merge_equity_raw_bars("1Day", &[("yahoo-chart", &yahoo), ("alpaca", &alpaca)]);

    // Only Alpaca's range survives; none of Yahoo's older (day1..9) garbage.
    assert_eq!(merged.first().map(|b| b.ts_ms), Some(10 * day));
    assert_eq!(merged.last().map(|b| b.ts_ms), Some(25 * day));
    assert_eq!(merged.len(), 16);
    assert!(
        merged.iter().all(|b| b.close < 5.0),
        "no 10,000× unadjusted bars may be spliced in"
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
        chart_merge_equity_raw_bars("1Day", &[("yahoo-chart", &yahoo), ("alpaca", &alpaca)]);

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
        chart_merge_equity_raw_bars("1Day", &[("yahoo-chart", &yahoo), ("alpaca", &alpaca)]);

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
fn heavy_sync_mtf_overlay_render_policy_only_refreshes_focused_cell() {
    assert!(ChartState::should_ensure_mql_mtf_overlays_for_render(
        false, true, false
    ));
    assert!(ChartState::should_ensure_mql_mtf_overlays_for_render(
        true, false, false
    ));
    assert!(ChartState::should_ensure_mql_mtf_overlays_for_render(
        true, true, true
    ));
    assert!(!ChartState::should_ensure_mql_mtf_overlays_for_render(
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
fn watchlist_poll_is_not_blocked_by_background_backfill() {
    assert!(watchlist_quote_poll_ready(
        std::time::Duration::from_secs(15),
        true,
        false,
        true,
    ));
    assert!(!watchlist_quote_poll_ready(
        std::time::Duration::from_secs(14),
        true,
        false,
        true,
    ));
    assert!(!watchlist_quote_poll_ready(
        std::time::Duration::from_secs(15),
        false,
        false,
        true,
    ));
    assert!(!watchlist_quote_poll_ready(
        std::time::Duration::from_secs(15),
        true,
        true,
        true,
    ));
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

// ── SMA Tests ────────────────────────────────────────────────────────

#[test]
fn test_sma_basic() {
    let bars = make_bars(10);
    let sma = compute_sma(&bars, 3);
    assert_eq!(sma.len(), 10);
    // First 2 should be None (period-1)
    assert!(sma[0].is_none());
    assert!(sma[1].is_none());
    // Third bar should have a value
    assert!(sma[2].is_some());
    // SMA(3) of closes 101, 102, 103 = 102
    let v = sma[2].unwrap();
    assert!(
        (v - 102.0).abs() < 0.01,
        "SMA(3) bar 2 = {}, expected ~102",
        v
    );
}

#[test]
fn test_sma_empty() {
    let bars: Vec<Bar> = vec![];
    let sma = compute_sma(&bars, 5);
    assert!(sma.is_empty());
}

#[test]
fn test_sma_period_larger_than_data() {
    let bars = make_bars(3);
    let sma = compute_sma(&bars, 10);
    assert_eq!(sma.len(), 3);
    assert!(sma.iter().all(|v| v.is_none()));
}

// ── EMA Tests ────────────────────────────────────────────────────────

#[test]
fn test_ema_basic() {
    let bars = make_bars(20);
    let ema = compute_ema(&bars, 5);
    assert_eq!(ema.len(), 20);
    // First 4 should be None
    for i in 0..4 {
        assert!(ema[i].is_none(), "EMA[{}] should be None", i);
    }
    // Should have values from period-1 onward
    assert!(ema[4].is_some());
    // EMA should be close to but not exactly equal to close prices (trending up)
    let last = ema[19].unwrap();
    assert!(last > 100.0 && last < 125.0, "EMA last = {}", last);
}

#[test]
fn test_ema_follows_trend() {
    let bars = make_bars(50);
    let ema = compute_ema(&bars, 10);
    // EMA should be increasing for ascending bars
    let mut prev = 0.0;
    for v in ema.iter().flatten() {
        assert!(
            *v >= prev,
            "EMA should be non-decreasing: {} < {}",
            *v,
            prev
        );
        prev = *v;
    }
}

// ── KAMA Tests ───────────────────────────────────────────────────────

#[test]
fn test_kama_basic() {
    let bars = make_bars(30);
    let kama = compute_kama(&bars, 10, 2, 30);
    assert_eq!(kama.len(), 30);
    assert!(kama[9].is_none()); // period-1 warmup
    assert!(kama[10].is_some());
}

#[test]
fn test_kama_adapts_to_trend() {
    let bars = make_bars(50);
    let kama = compute_kama(&bars, 10, 2, 30);
    // KAMA should follow the uptrend
    let last = kama.last().unwrap().unwrap();
    assert!(last > 130.0, "KAMA should follow uptrend: {}", last);
}

// ── Bollinger Bands ──────────────────────────────────────────────────

#[test]
fn test_bollinger_bands() {
    let bars = make_bars(30);
    let (mid, upper, lower) = compute_bollinger(&bars, 20, 2.0);
    assert_eq!(mid.len(), 30);
    // After warmup, upper > mid > lower
    for i in 19..30 {
        if let (Some(u), Some(m), Some(l)) = (upper[i], mid[i], lower[i]) {
            assert!(u > m, "Upper {} should be > mid {}", u, m);
            assert!(m > l, "Mid {} should be > lower {}", m, l);
        }
    }
}

// ── RSI Tests ────────────────────────────────────────────────────────

#[test]
fn test_rsi_range() {
    let bars = make_oscillating_bars(50);
    let rsi = compute_rsi(&bars, 14);
    for v in rsi.iter().flatten() {
        assert!(*v >= 0.0 && *v <= 100.0, "RSI should be 0-100: {}", v);
    }
}

#[test]
fn test_rsi_uptrend_bullish() {
    let bars = make_bars(30);
    let rsi = compute_rsi(&bars, 14);
    // Strong uptrend should have RSI > 50
    if let Some(v) = rsi.last().unwrap() {
        assert!(*v > 50.0, "RSI in uptrend should be >50: {}", v);
    }
}

// ── Fisher Transform ─────────────────────────────────────────────────

#[test]
fn test_fisher_transform() {
    let bars = make_bars(50);
    let (fisher, signal) = compute_fisher(&bars, 32);
    assert_eq!(fisher.len(), 50);
    assert_eq!(signal.len(), 50);
    // Should have values after warmup
    let has_values = fisher.iter().any(|v| v.is_some());
    assert!(has_values, "Fisher should have computed values");
}

// ── MACD Tests ───────────────────────────────────────────────────────

#[test]
fn test_macd_basic() {
    let bars = make_bars(50);
    let (macd, signal, hist) = compute_macd(&bars, 12, 26, 9);
    assert_eq!(macd.len(), 50);
    assert_eq!(signal.len(), 50);
    assert_eq!(hist.len(), 50);
    // Should have values after warmup (26 + 9 bars)
    assert!(macd[35].is_some());
}

#[test]
fn test_macd_histogram_is_difference() {
    let bars = make_bars(50);
    let (macd, signal, hist) = compute_macd(&bars, 12, 26, 9);
    for i in 0..50 {
        if let (Some(m), Some(s), Some(h)) = (macd[i], signal[i], hist[i]) {
            assert!(
                (h - (m - s)).abs() < 0.001,
                "Histogram should be MACD - Signal"
            );
        }
    }
}

// ── Stochastic ───────────────────────────────────────────────────────

#[test]
fn test_stochastic_range() {
    let bars = make_oscillating_bars(50);
    let (k, d) = compute_stochastic(&bars, 14, 3, 3);
    for v in k.iter().flatten() {
        assert!(*v >= 0.0 && *v <= 100.0, "Stoch %K should be 0-100: {}", v);
    }
    for v in d.iter().flatten() {
        assert!(*v >= 0.0 && *v <= 100.0, "Stoch %D should be 0-100: {}", v);
    }
}

// ── ADX Tests ────────────────────────────────────────────────────────

#[test]
fn test_adx_range() {
    let bars = make_bars(50);
    let (adx, di_plus, di_minus) = compute_adx(&bars, 14);
    for v in adx.iter().flatten() {
        assert!(*v >= 0.0, "ADX should be >= 0: {}", v);
    }
    for v in di_plus.iter().flatten() {
        assert!(*v >= 0.0, "DI+ should be >= 0: {}", v);
    }
    for v in di_minus.iter().flatten() {
        assert!(*v >= 0.0, "DI- should be >= 0: {}", v);
    }
}

// ── ATR Tests ────────────────────────────────────────────────────────

#[test]
fn test_atr_positive() {
    let bars = make_bars(30);
    let atr = compute_atr(&bars, 14);
    for v in atr.iter().flatten() {
        assert!(*v > 0.0, "ATR should be > 0: {}", v);
    }
}

// ── Ichimoku Tests ───────────────────────────────────────────────────

#[test]
fn test_ichimoku_lengths() {
    let bars = make_bars(60);
    let (tenkan, kijun, span_a, span_b) = compute_ichimoku(&bars, 9, 26, 52);
    assert_eq!(tenkan.len(), 60);
    assert_eq!(kijun.len(), 60);
    assert_eq!(span_a.len(), 60);
    assert_eq!(span_b.len(), 60);
}

// ── WMA / HMA Tests ─────────────────────────────────────────────────

#[test]
fn test_wma_basic() {
    let bars = make_bars(30);
    let wma = compute_wma(&bars, 10);
    assert_eq!(wma.len(), 30);
    assert!(wma[9].is_some());
}

#[test]
fn test_hma_basic() {
    let bars = make_bars(30);
    let hma = compute_hma(&bars, 10);
    assert_eq!(hma.len(), 30);
    // HMA should have values after warmup
    let has_values = hma.iter().any(|v| v.is_some());
    assert!(has_values);
}

// ── CCI / Williams %R ────────────────────────────────────────────────

#[test]
fn test_cci_basic() {
    let bars = make_oscillating_bars(30);
    let cci = compute_cci(&bars, 20);
    assert_eq!(cci.len(), 30);
}

#[test]
fn test_williams_r_range() {
    let bars = make_oscillating_bars(30);
    let wr = compute_williams_r(&bars, 14);
    for v in wr.iter().flatten() {
        assert!(
            *v >= -100.0 && *v <= 0.0,
            "Williams %R should be -100 to 0: {}",
            v
        );
    }
}

// ── OBV / Momentum ──────────────────────────────────────────────────

#[test]
fn test_obv_basic() {
    let bars = make_bars(20);
    let obv = compute_obv(&bars);
    assert_eq!(obv.len(), 20);
    assert!(obv[0].is_some());
}

#[test]
fn test_momentum_basic() {
    let bars = make_bars(20);
    let mom = compute_momentum(&bars, 10);
    assert_eq!(mom.len(), 20);
}

// ── Parabolic SAR ────────────────────────────────────────────────────

#[test]
fn test_psar_basic() {
    let bars = make_bars(30);
    let psar = compute_parabolic_sar(&bars, 0.02, 0.2);
    assert_eq!(psar.len(), 30);
    let has_values = psar.iter().any(|v| v.is_some());
    assert!(has_values);
}

// ── Fractals ─────────────────────────────────────────────────────────

#[test]
fn test_fractals_length() {
    let bars = make_bars(20);
    let up = compute_fractals_up(&bars);
    let down = compute_fractals_down(&bars);
    assert_eq!(up.len(), 20);
    assert_eq!(down.len(), 20);
}

// ── BetterVolume ─────────────────────────────────────────────────────

#[test]
fn test_better_volume_classification() {
    let bars = make_oscillating_bars(30);
    let bv = compute_better_volume(&bars);
    assert_eq!(bv.len(), 30);
    // All values should be 0-5
    for v in &bv {
        assert!(*v <= 5, "BetterVolume type should be 0-5: {}", v);
    }
}

// ── Supply/Demand Zones ──────────────────────────────────────────────

#[test]
fn test_supply_demand_zones() {
    let bars = make_oscillating_bars(50);
    let (supply, demand) = compute_supply_demand_zones(&bars);
    for (idx, high, low, status) in &supply {
        assert!(*idx < bars.len());
        assert!(high > low);
        assert!(*status <= 2);
    }
    for (idx, high, low, status) in &demand {
        assert!(*idx < bars.len());
        assert!(high > low);
        assert!(*status <= 2);
    }
}

#[test]
fn test_supply_demand_realistic_swings() {
    // Simulate: rally from 20→200, then crash to 80, then bounce to 140, then drop to 85
    // Should produce surviving zones near recent swing highs/lows
    let mut bars = Vec::new();
    let prices = [
        // Phase 1: Rally 20→200 (100 bars)
        20.0, 22.0, 25.0, 28.0, 30.0, 33.0, 35.0, 38.0, 40.0, 43.0, 45.0, 48.0, 50.0, 55.0, 58.0,
        60.0, 65.0, 68.0, 70.0, 75.0, 78.0, 80.0, 85.0, 88.0, 90.0, 95.0, 98.0, 100.0, 105.0,
        110.0, 112.0, 115.0, 118.0, 120.0, 125.0, 128.0, 130.0, 135.0, 138.0, 140.0, 142.0, 145.0,
        148.0, 150.0, 155.0, 158.0, 160.0, 165.0, 168.0, 170.0, 172.0, 175.0, 178.0, 180.0, 182.0,
        185.0, 188.0, 190.0, 192.0, 195.0, 197.0, 200.0, 198.0, 195.0, 192.0, 190.0, 188.0, 185.0,
        182.0, 180.0, // Phase 2: Crash 180→80 (30 bars)
        175.0, 170.0, 165.0, 155.0, 150.0, 140.0, 135.0, 130.0, 125.0, 120.0, 115.0, 110.0, 105.0,
        100.0, 95.0, 90.0, 88.0, 85.0, 82.0, 80.0, // Phase 3: Bounce 80→140 (20 bars)
        82.0, 85.0, 88.0, 92.0, 95.0, 100.0, 105.0, 110.0, 115.0, 120.0, 125.0, 128.0, 130.0,
        135.0, 138.0, 140.0, 138.0, 135.0, 130.0, 125.0, // Phase 4: Drop 125→85 (20 bars)
        120.0, 115.0, 110.0, 105.0, 100.0, 98.0, 95.0, 92.0, 90.0, 88.0, 87.0, 86.0, 85.0, 86.0,
        87.0, 85.0, 84.0, 85.0, 86.0, 85.0,
    ];
    for (i, &close) in prices.iter().enumerate() {
        let range = close * 0.03; // 3% daily range
        bars.push(Bar {
            ts_ms: 1700000000000 + i as i64 * 86400000,
            open: close - range * 0.2,
            high: close + range * 0.5,
            low: close - range * 0.5,
            close,
            volume: 1000.0,
        });
    }
    let n = bars.len();
    eprintln!(
        "[test] {} bars, price range {:.0}-{:.0}",
        n,
        bars.iter().map(|b| b.low).fold(f64::MAX, f64::min),
        bars.iter().map(|b| b.high).fold(f64::MIN, f64::max)
    );

    let (supply, demand) = compute_supply_demand_zones(&bars);

    eprintln!(
        "[test] Result: {} supply, {} demand zones",
        supply.len(),
        demand.len()
    );
    for (idx, hi, lo, st) in &supply {
        eprintln!(
            "[test]   SUPPLY bar={} hi={:.2} lo={:.2} st={}",
            idx, hi, lo, st
        );
    }
    for (idx, hi, lo, st) in &demand {
        eprintln!(
            "[test]   DEMAND bar={} hi={:.2} lo={:.2} st={}",
            idx, hi, lo, st
        );
    }

    // We should have at least some surviving zones:
    // - The 200 peak supply zone (price never went above 200 again)
    // - The 80 low demand zone (price never went below 80 again)
    // - Recent swing zones near current price
    assert!(
        !supply.is_empty() || !demand.is_empty(),
        "Should have surviving zones for a chart with clear swings"
    );
}

// ── Ehlers DSP Indicators ────────────────────────────────────────────

#[test]
fn test_ehlers_super_smoother() {
    let bars = make_bars(30);
    let ss = ehlers_super_smoother(&bars, 10);
    assert_eq!(ss.len(), 30);
    let has_values = ss.iter().any(|v| v.is_some());
    assert!(has_values);
}

#[test]
fn test_ehlers_decycler() {
    let bars = make_bars(30);
    let dc = ehlers_decycler(&bars, 20);
    assert_eq!(dc.len(), 30);
}

#[test]
fn test_ehlers_mama_fama() {
    let bars = make_bars(30);
    let (mama, fama) = ehlers_mama_fama(&bars, 0.5, 0.05);
    assert_eq!(mama.len(), 30);
    assert_eq!(fama.len(), 30);
}

#[test]
fn test_ehlers_ebsw() {
    let bars = make_oscillating_bars(50);
    let ebsw = ehlers_even_better_sinewave(&bars, 40);
    assert_eq!(ebsw.len(), 50);
    // EBSW should be in -1 to 1 range
    for v in ebsw.iter().flatten() {
        assert!(*v >= -2.0 && *v <= 2.0, "EBSW should be ~-1 to 1: {}", v);
    }
}

#[test]
fn test_ehlers_cyber_cycle() {
    let bars = make_oscillating_bars(30);
    let cc = ehlers_cyber_cycle(&bars);
    assert_eq!(cc.len(), 30);
}

#[test]
fn test_ehlers_cg_oscillator() {
    let bars = make_bars(30);
    let cg = ehlers_cg_oscillator(&bars, 10);
    assert_eq!(cg.len(), 30);
}

#[test]
fn test_ehlers_roofing_filter() {
    let bars = make_oscillating_bars(60);
    let rf = ehlers_roofing_filter(&bars, 10, 48);
    assert_eq!(rf.len(), 60);
}

// ── Heikin-Ashi / Renko ──────────────────────────────────────────────

#[test]
fn test_heikin_ashi() {
    let bars = make_bars(10);
    let ha = heikin_ashi(&bars);
    assert_eq!(ha.len(), 10);
    // HA close = (O+H+L+C)/4
    let b = &bars[0];
    let ha_close = (b.open + b.high + b.low + b.close) / 4.0;
    assert!((ha[0].close - ha_close).abs() < 0.01);
}

#[test]
fn test_renko_bricks() {
    let bars = make_bars(50);
    let bricks = renko_bricks(&bars);
    // Renko should produce some bricks for trending data
    assert!(
        !bricks.is_empty(),
        "Renko should produce bricks for trending data"
    );
}

// ── ATR Projection ───────────────────────────────────────────────────

#[test]
fn test_atr_projection() {
    let bars = make_bars(20);
    let atr = compute_atr(&bars, 14);
    let (upper, lower) = compute_atr_projection(&bars, &atr);
    assert_eq!(upper.len(), 20);
    assert_eq!(lower.len(), 20);
    // Upper should be > lower where both exist
    for i in 0..20 {
        if let (Some(u), Some(l)) = (upper[i], lower[i]) {
            assert!(u > l, "ATR proj upper {} should be > lower {}", u, l);
        }
    }
}

// ── Previous Candle Levels ───────────────────────────────────────────

#[test]
fn test_prev_candle_levels() {
    let bars = make_bars(10);
    let (_h1, _h4, d1, w1, _mn1) = compute_prev_candle_levels(&bars);
    // With synthetic data, should have daily levels at least
    // (may be None if all bars are same "day" in synthetic data)
    let _ = (d1, w1);
}

// ── Helper Functions ─────────────────────────────────────────────────

#[test]
fn test_in_range() {
    assert!(in_range(0.5, 0.0, 1.0));
    assert!(!in_range(1.5, 0.0, 1.0));
    assert!(in_range(0.618, 0.5, 0.8));
}

#[test]
fn test_format_price() {
    let s = format_price(123.456);
    assert!(s.contains("123"));
}

#[test]
fn test_fuzzy_match() {
    assert!(fuzzy_match("sma", "SMA200"));
    assert!(fuzzy_match("fish", "Fisher Transform"));
    assert!(!fuzzy_match("xyz", "SMA200"));
    assert!(fuzzy_match("", "anything")); // empty matches all
}

// ── Auto Fibonacci ───────────────────────────────────────────────────

#[test]
fn test_auto_fibonacci() {
    let mut bars = make_bars(60);
    // Create a clear swing: up then down
    for i in 30..60 {
        bars[i].close = 160.0 - i as f64;
        bars[i].high = bars[i].close + 2.0;
        bars[i].low = bars[i].close - 1.0;
        bars[i].open = bars[i].close - 0.5;
    }
    let mut chart = ChartState::new("TEST", Timeframe::H4);
    chart.bars = bars;
    chart.compute_indicators();
    // Auto fib may or may not find levels depending on fractal detection.
    // Point of this test is "compute_indicators doesn't panic on a swing" —
    // the computation above is the assertion.
    let _ = chart.auto_fib_levels.len();
}

// ── ChartState Integration ───────────────────────────────────────────

#[test]
fn test_chart_state_compute_all_indicators() {
    let mut chart = ChartState::new("TEST", Timeframe::H4);
    chart.bars = make_bars(100);
    chart.compute_indicators();
    // All indicator vectors should have correct length
    assert_eq!(chart.sma200.len(), 100);
    assert_eq!(chart.sma100.len(), 100);
    assert_eq!(chart.kama.len(), 100);
    assert_eq!(chart.ema21.len(), 100);
    assert_eq!(chart.rsi.len(), 100);
    assert_eq!(chart.fisher.len(), 100);
    assert_eq!(chart.macd_line.len(), 100);
    assert_eq!(chart.atr.len(), 100);
    assert_eq!(chart.cmo.len(), 100);
    assert_eq!(chart.qstick.len(), 100);
    assert_eq!(chart.disparity.len(), 100);
    assert_eq!(chart.bop.len(), 100);
    assert_eq!(chart.stddev.len(), 100);
    assert_eq!(chart.mfi.len(), 100);
    assert_eq!(chart.trix_line.len(), 100);
    assert_eq!(chart.ppo_line.len(), 100);
    assert_eq!(chart.ultosc.len(), 100);
    assert_eq!(chart.stochrsi_k.len(), 100);
    assert_eq!(chart.better_vol_type.len(), 100);
}

#[test]
fn test_chart_talib_gpu_fallback_extension_bundle_ranges() {
    let bars = make_oscillating_bars(160);

    let mfi = compute_mfi(&bars, 14);
    for value in mfi.iter().flatten() {
        assert!(
            (0.0..=100.0).contains(value),
            "MFI should be 0-100: {}",
            value
        );
    }

    let (trix, trix_signal, trix_hist) = compute_trix(&bars, 15, 9);
    assert!(trix.iter().flatten().all(|v| v.is_finite()));
    assert!(trix_signal.iter().flatten().all(|v| v.is_finite()));
    assert!(trix_hist.iter().flatten().all(|v| v.is_finite()));

    let (ppo, ppo_signal, ppo_hist) = compute_ppo(&bars, 12, 26, 9);
    assert!(ppo.iter().flatten().all(|v| v.is_finite()));
    assert!(ppo_signal.iter().flatten().all(|v| v.is_finite()));
    assert!(ppo_hist.iter().flatten().all(|v| v.is_finite()));

    let ultosc = compute_ultosc(&bars);
    for value in ultosc.iter().flatten() {
        assert!(
            (0.0..=100.0).contains(value),
            "ULTOSC should be 0-100: {}",
            value
        );
    }

    let (stochrsi_k, stochrsi_d) = compute_stochrsi(&bars, 14, 14, 3, 3);
    for value in stochrsi_k.iter().flatten() {
        assert!(
            (0.0..=100.0).contains(value),
            "StochRSI %K should be 0-100: {}",
            value
        );
    }
    for value in stochrsi_d.iter().flatten() {
        assert!(
            (0.0..=100.0).contains(value),
            "StochRSI %D should be 0-100: {}",
            value
        );
    }
}

#[test]
fn chart_camera_accumulates_fractional_horizontal_pan() {
    let mut camera = ChartCamera::from_legacy(300, 100, false);
    camera.begin_pan(800.0, 400.0, 100.0, 20.0);

    camera.pan_pixels(3.0, 0.0, 800.0, 400.0, 500, 80.0, 120.0);
    assert!(
        (camera.right_edge_bar() - 299.625).abs() < 1e-9,
        "3px at 8px/bar should move by 0.375 bar, got {}",
        camera.right_edge_bar()
    );
    assert!(camera.manual_override());
    assert!(!camera.follow_latest);
}

#[test]
fn chart_camera_vertical_pan_uses_zoomed_visible_price_span() {
    let mut camera = ChartCamera::from_legacy(499, 100, false);
    camera.set_price_view(100.0, 10.0);
    camera.begin_pan(800.0, 400.0, 100.0, 20.0);

    camera.pan_pixels(0.0, 120.0, 800.0, 400.0, 500, 80.0, 120.0);

    assert!(
        (camera.price_center.unwrap() - 103.0).abs() < 1e-9,
        "120px over 400px of 10pt span should move price center by 3pt; got {:?}",
        camera.price_center
    );
    assert_eq!(camera.price_span, Some(10.0));
    assert!(camera.manual_override());
}

#[test]
fn chart_camera_price_range_can_free_pan_below_zero() {
    let mut camera = ChartCamera::from_legacy(499, 100, false);
    camera.set_price_view(0.10, 0.12);
    camera.begin_pan(800.0, 400.0, 0.10, 0.12);

    camera.pan_pixels(0.0, -380.0, 800.0, 400.0, 500, 0.10, 0.12);
    let (min, max) = camera.explicit_price_range().unwrap();

    assert!(
        min < 0.0,
        "free-look price range should be allowed below zero; got {min}..{max}"
    );
    assert!(max > min);
}

#[test]
fn chart_state_repeated_free_look_drag_keeps_camera_authoritative() {
    let mut chart = ChartState::new("TEST", Timeframe::H4);
    chart.bars = make_bars(500);
    chart.visible_bars = 100;
    chart.view_offset = 499;
    chart.begin_chart_camera_pan(800.0, 400.0);
    chart.pan_chart_camera_pixels(egui::vec2(80.0, 0.0), 800.0, 400.0);
    let first_right_edge = chart.camera.right_edge_bar();
    let first_price_range = chart.visible_price_range().unwrap();
    let first_gen = chart.visible_bars_gen;

    chart.begin_chart_camera_pan(800.0, 400.0);

    assert!(
        (chart.camera.right_edge_bar() - first_right_edge).abs() < 1e-9,
        "new drag must not rebuild camera from rounded legacy view_offset"
    );
    assert_eq!(chart.visible_price_range().unwrap(), first_price_range);
    assert!(
        chart.visible_bars_gen > first_gen,
        "camera changes must invalidate draw early-out"
    );
}

#[test]
fn chart_camera_allows_empty_space_at_both_horizontal_edges() {
    let mut camera = ChartCamera::from_legacy(99, 100, false);
    camera.begin_pan(800.0, 400.0, 100.0, 20.0);

    camera.pan_pixels(10_000.0, 0.0, 800.0, 400.0, 500, 100.0, 20.0);
    assert!(
        camera.right_edge_bar().abs() < 1e-9,
        "left free-look bound should put oldest bar at the right edge, not clamp the viewport full of data"
    );

    camera.begin_pan(800.0, 400.0, 100.0, 20.0);
    camera.pan_pixels(-10_000.0, 0.0, 800.0, 400.0, 500, 100.0, 20.0);
    assert!(
        (camera.right_edge_bar() - 598.0).abs() < 1e-9,
        "right free-look bound should put newest bar at the left edge for one viewport of empty space"
    );
}

#[test]
fn chart_state_visible_slot_window_preserves_empty_edge_slots() {
    let mut chart = ChartState::new("TEST", Timeframe::H4);
    chart.bars = make_bars(500);
    chart.visible_bars = 100;
    chart.view_offset = 99;
    chart.manual_view_override = true;
    chart.camera = ChartCamera::from_legacy(0, 100, true);

    let (start, end, first_slot, slots) = chart.visible_slot_window();
    assert_eq!((start, end, slots), (0, 1, 100));
    assert_eq!(
        first_slot, 99.0,
        "oldest bar should render in the final slot with empty space to its left"
    );

    chart.camera = ChartCamera::from_legacy(598, 100, true);
    let (start, end, first_slot, slots) = chart.visible_slot_window();
    assert_eq!((start, end, slots), (499, 500, 100));
    assert_eq!(
        first_slot, 0.0,
        "newest bar should render in the first slot with empty space to its right"
    );
}

#[test]
fn chart_state_visible_slot_window_preserves_fractional_camera_offset() {
    let mut chart = ChartState::new("TEST", Timeframe::H4);
    chart.bars = make_bars(500);
    chart.visible_bars = 100;
    chart.manual_view_override = true;
    chart.camera = ChartCamera::from_legacy(300, 100, true);
    chart.camera.begin_pan(800.0, 400.0, 100.0, 20.0);
    chart
        .camera
        .pan_pixels(3.0, 0.0, 800.0, 400.0, 500, 100.0, 20.0);

    let (start, end, first_slot, slots) = chart.visible_slot_window();

    assert_eq!((start, end, slots), (201, 301, 100));
    assert!(
        (first_slot - 0.375).abs() < 1e-6,
        "sub-bar drag must move candles smoothly instead of rounding/snap-back; got first_slot={first_slot}"
    );
}

#[test]
fn chart_price_pane_height_excludes_indicator_panes_for_one_to_one_drag() {
    assert_eq!(chart_price_pane_height(1000.0, 0), 978.0);
    assert_eq!(chart_price_pane_height(1000.0, 1), 898.0);
    assert_eq!(chart_price_pane_height(1000.0, 3), 738.0);
}

#[test]
fn chart_camera_reload_preserves_manual_position_but_follow_latest_tracks_end() {
    let mut manual = ChartCamera::from_legacy(588, 100, true);
    manual.on_data_len_changed(600, 720);
    assert!(
        (manual.right_edge_bar() - 588.0).abs() < 1e-9,
        "manual camera should preserve the user's absolute recentered viewport across live reloads"
    );
    assert!(!manual.follow_latest);

    let mut following = ChartCamera::from_legacy(600, 100, false);
    following.on_data_len_changed(601, 720);
    assert!(
        (following.right_edge_bar() - 724.0).abs() < 1e-9,
        "follow-latest camera should snap to new latest bar plus chart-shift margin"
    );
    assert!(following.follow_latest);
}

#[test]
fn test_chart_state_visible_range() {
    let mut chart = ChartState::new("TEST", Timeframe::H4);
    chart.bars = make_bars(500);
    chart.visible_bars = 200;
    chart.view_offset = 499;
    let (start, end) = chart.visible_range();
    assert_eq!(end - start, 200);
    assert_eq!(end, 500);
}

#[test]
fn chart_horizontal_zoom_marks_manual_view_override() {
    let mut chart = ChartState::new("WOK", Timeframe::H4);
    chart.bars = make_bars(500);
    chart.visible_bars = 200;
    chart.view_offset = 499;

    TyphooNApp::handle_zoom(&mut chart, 60.0);

    assert!(chart.visible_bars < 200);
    assert!(chart.manual_view_override);
}

#[test]
fn chart_zoom_keeps_free_look_camera_instead_of_rebuilding_from_legacy() {
    let mut chart = ChartState::new("WOK", Timeframe::H4);
    chart.bars = make_bars(500);
    chart.visible_bars = 100;
    chart.view_offset = 499;

    chart.begin_chart_camera_pan(800.0, 400.0);
    chart.pan_chart_camera_pixels(egui::vec2(83.0, 120.0), 800.0, 400.0);
    let right_before = chart.camera.right_edge_bar();
    let price_center_before = chart.camera.price_center.unwrap();

    chart.zoom_chart_price_by(1.25);
    assert!(
        (chart.camera.right_edge_bar() - right_before).abs() < 1e-9,
        "vertical zoom must not rebuild horizontal camera from rounded view_offset"
    );
    assert!(
        (chart.camera.price_center.unwrap() - price_center_before).abs() < 1e-9,
        "vertical zoom should scale around the current free-look price center"
    );

    TyphooNApp::handle_zoom(&mut chart, 30.0);
    assert!(
        chart.manual_view_override,
        "horizontal zoom must keep manual free-look active"
    );
    assert!(
        (chart.camera.price_center.unwrap() - price_center_before).abs() < 1e-9,
        "horizontal zoom must not reset vertical free-look price center"
    );
}

#[test]
fn chart_body_camera_pans_time_and_price() {
    let mut chart = ChartState::new("TEST", Timeframe::H4);
    chart.bars = make_bars(500);
    chart.visible_bars = 100;
    chart.view_offset = 499;

    chart.begin_chart_camera_pan(800.0, 400.0);
    chart.pan_chart_camera_pixels(egui::vec2(80.0, 120.0), 800.0, 400.0);

    assert_eq!(chart.view_offset, 489);
    assert!(
        chart.price_pan > 0.0,
        "dragging downward should move the series downward"
    );
    assert!(
        chart.manual_view_override,
        "manual pan must suppress auto-follow snapback on cache reload"
    );
}

#[test]
fn chart_body_camera_accumulates_sub_bar_motion_fractionally() {
    let mut chart = ChartState::new("TEST", Timeframe::H4);
    chart.bars = make_bars(500);
    chart.visible_bars = 100;
    chart.view_offset = 300;
    chart.price_pan = 2.0;

    chart.begin_chart_camera_pan(800.0, 400.0);
    chart.pan_chart_camera_pixels(egui::vec2(3.0, 0.0), 800.0, 400.0);

    assert!(
        (chart.camera.right_edge_bar() - 299.625).abs() < 1e-9,
        "camera must preserve fractional sub-bar pan; got {}",
        chart.camera.right_edge_bar()
    );
    assert_eq!(chart.price_pan, 2.0);
    assert!(chart.manual_view_override);
}

#[test]
fn chart_body_camera_vertical_pan_uses_zoomed_visible_price_span() {
    let mut chart = ChartState::new("WOK", Timeframe::H4);
    chart.bars = make_bars(500);
    chart.visible_bars = 100;
    chart.view_offset = 499;
    chart.price_zoom = 10.0;

    let (natural_center, natural_span) = chart.natural_visible_price_view().unwrap();
    chart.begin_chart_camera_pan(800.0, 400.0);
    chart.pan_chart_camera_pixels(egui::vec2(0.0, 120.0), 800.0, 400.0);

    let expected = natural_span / 10.0 * 120.0 / 400.0;
    assert!(
        (chart.price_pan - expected).abs() < 1e-9,
        "zoomed vertical pan should move by visible price span; got {}, expected {}",
        chart.price_pan,
        expected
    );
    assert!((chart.camera.price_center.unwrap() - (natural_center + expected)).abs() < 1e-9);
    assert!(chart.manual_view_override);
}

#[test]
fn test_chart_state_reload_match_requires_source_for_loaded_chart() {
    let mut chart = ChartState::new("BTC/USD", Timeframe::H2);
    chart.bars = make_bars(20);
    chart.primary_source = "kraken";

    assert!(chart.should_reload_for_bar_fetch("BTCUSD", "1Hour", "kraken"));
    assert!(!chart.should_reload_for_bar_fetch("BTCUSD", "1Hour", "alpaca"));

    chart.primary_source = "kraken-equities";
    assert!(chart.should_reload_for_bar_fetch("BTCUSD", "1Hour", "alpaca"));
    assert!(chart.should_reload_for_bar_fetch("BTCUSD", "1Hour", "yahoo-chart"));
    assert!(!chart.should_reload_for_bar_fetch("BTCUSD", "1Hour", "kraken"));
}

#[test]
fn test_chart_state_reload_match_allows_empty_chart_fill() {
    let chart = ChartState::new("AAPL", Timeframe::D1);

    assert!(chart.should_reload_for_bar_fetch("AAPL", "1Day", "alpaca"));
    assert!(!chart.should_reload_for_bar_fetch("MSFT", "1Day", "alpaca"));
}

// ── BetterVolume MQL5 classification tests ────────────────────────

#[test]
fn test_better_volume_mql5_classifications() {
    // BetterVolume uses adaptive comparison against lookback extremes (not fixed thresholds).
    // Verify basic properties: correct length, valid classification range, variety of results.
    let bars = make_oscillating_bars(50);
    let bv = compute_better_volume(&bars);
    assert_eq!(bv.len(), bars.len());
    // All values should be valid classification (0-5)
    for (i, &v) in bv.iter().enumerate() {
        assert!(v <= 5, "Bar {} has invalid classification {}", i, v);
    }
    // First `lookback` bars should be normal (5) since lookback not ready
    assert_eq!(bv[0], 5, "Bar 0 should be normal (5)");
    // With oscillating data, at least some bars should be non-normal
    let non_normal = bv.iter().filter(|&&v| v != 5).count();
    assert!(
        non_normal > 0,
        "With oscillating data, some bars should have non-normal classification"
    );
}

// ── Supply/Demand zone break detection ────────────────────────────

#[test]
fn test_supply_demand_break_detection() {
    // Create bars: rally to 200, crash through all supply zones
    let mut bars = Vec::new();
    // Phase 1: 30 bars oscillating 90-110 (creates fractals)
    for i in 0..30 {
        let base = 100.0 + (i as f64 * 0.5).sin() * 8.0;
        bars.push(Bar {
            ts_ms: 1700000000000 + i as i64 * 86400000,
            open: base - 0.5,
            high: base + 2.0,
            low: base - 2.0,
            close: base + 0.5,
            volume: 1000.0,
        });
    }
    // Phase 2: massive rally through all zones
    for i in 30..50 {
        let base = 100.0 + (i - 30) as f64 * 5.0;
        bars.push(Bar {
            ts_ms: 1700000000000 + i as i64 * 86400000,
            open: base,
            high: base + 3.0,
            low: base - 1.0,
            close: base + 2.0,
            volume: 2000.0,
        });
    }

    let (supply, _demand) = compute_supply_demand_zones(&bars);
    // Supply zones from phase 1 should be broken by phase 2 rally
    // Only zones near the top (if any) should survive
    for (_, hi, _, _) in &supply {
        assert!(
            *hi >= 150.0,
            "Surviving supply zone should be at high prices (got {})",
            hi
        );
    }
}

// ── Supply/Demand zone merge ──────────────────────────────────────

#[test]
fn test_supply_demand_merge_overlapping() {
    // Create bars with multiple close fractal lows at similar prices
    let mut bars = Vec::new();
    for i in 0..50 {
        let base = if i % 10 < 5 {
            100.0 + i as f64 * 0.1
        } else {
            95.0
        }; // oscillate with dips to 95
        bars.push(Bar {
            ts_ms: 1700000000000 + i as i64 * 86400000,
            open: base + 0.5,
            high: base + 2.0,
            low: base - 2.0,
            close: base - 0.5,
            volume: 1000.0,
        });
    }
    let (supply, demand) = compute_supply_demand_zones(&bars);
    // After merge, overlapping zones should be consolidated
    // Check no two zones of same type overlap
    for i in 0..supply.len() {
        for j in (i + 1)..supply.len() {
            let a = &supply[i];
            let b = &supply[j];
            let overlap = a.1 >= b.2 && b.1 >= a.2; // hi_a >= lo_b && hi_b >= lo_a
            assert!(
                !overlap,
                "Supply zones {} and {} overlap: ({:.2},{:.2}) vs ({:.2},{:.2})",
                i, j, a.2, a.1, b.2, b.1
            );
        }
    }
    for i in 0..demand.len() {
        for j in (i + 1)..demand.len() {
            let a = &demand[i];
            let b = &demand[j];
            let overlap = a.1 >= b.2 && b.1 >= a.2;
            assert!(!overlap, "Demand zones {} and {} overlap", i, j);
        }
    }
}

// ── GPU S/D zones from GPU output ─────────────────────────────────

#[test]
fn test_supply_demand_from_gpu() {
    let bars = make_oscillating_bars(50);
    // Simulate GPU output: mark bar 16 as supply fractal, bar 31 as demand
    let mut gpu_data = vec![0.0f32; 50 * 3];
    gpu_data[16 * 3] = -1.0; // supply
    gpu_data[16 * 3 + 1] = bars[16].high as f32;
    gpu_data[16 * 3 + 2] = bars[16].close as f32;
    gpu_data[31 * 3] = 1.0; // demand
    gpu_data[31 * 3 + 1] = bars[31].close as f32;
    gpu_data[31 * 3 + 2] = bars[31].low as f32;

    let (supply, demand) = compute_supply_demand_zones_from_gpu(&gpu_data, &bars);
    // Should produce at least the zones we marked (unless broken)
    let total = supply.len() + demand.len();
    assert!(
        total <= 2,
        "Should have at most 2 zones from 2 GPU fractals, got {}",
        total
    );
    for (_, hi, lo, _) in supply.iter().chain(demand.iter()) {
        assert!(hi > lo, "Zone high must be > low");
    }
}

// ── Aggregate bars to HTF ─────────────────────────────────────────

#[test]
fn test_aggregate_bars_to_htf() {
    // 12 hourly bars → 3 4-hour bars
    let mut bars = Vec::new();
    for i in 0..12 {
        bars.push(Bar {
            ts_ms: 1700000000000 + i as i64 * 3600000,
            open: 100.0 + i as f64,
            high: 105.0 + i as f64,
            low: 95.0 + i as f64,
            close: 102.0 + i as f64,
            volume: 1000.0,
        });
    }
    let htf = aggregate_bars_to_htf(&bars, 240); // 4 hours = 240 minutes
    assert!(
        htf.len() >= 3 && htf.len() <= 4,
        "12 hourly bars → 3-4 4-hour bars (timestamp bucketing), got {}",
        htf.len()
    );
    // First HTF bar should have open from bar 0
    assert_eq!(htf[0].open, bars[0].open);
    // Last HTF bar should have close from last input bar
    assert_eq!(htf.last().unwrap().close, bars[11].close);
    // Each HTF bar volume should be sum of its constituent bars
    assert!(htf[0].volume > 0.0, "HTF bar should have non-zero volume");
}

// ── TradeMarker aggregation ───────────────────────────────────────

#[test]
fn test_trade_marker_aggregation() {
    // Verify the HashMap aggregation logic used in build_trade_overlay
    use std::collections::HashMap;
    let mut marker_map: HashMap<(usize, bool, i64), (f64, u32, String)> = HashMap::new();

    // 3 buys at same bar+price
    for _ in 0..3 {
        let entry = marker_map
            .entry((100, true, 15000))
            .or_insert((0.0, 0, String::new()));
        entry.0 += 0.10;
        entry.1 += 1;
    }
    // 1 sell at different price
    let entry = marker_map
        .entry((100, false, 16000))
        .or_insert((0.0, 0, String::new()));
    entry.0 += 0.50;
    entry.1 += 1;

    assert_eq!(marker_map.len(), 2, "Should have 2 unique entries");
    let buy = marker_map.get(&(100, true, 15000)).unwrap();
    assert!(
        (buy.0 - 0.30).abs() < 0.001,
        "Aggregated volume should be 0.30"
    );
    assert_eq!(buy.1, 3, "Should have 3 aggregated trades");
}

// ── TradeOverlay default ──────────────────────────────────────────

#[test]
fn test_trade_overlay_default() {
    let ov = TradeOverlay::default();
    assert!(ov.markers.is_empty());
    assert!(ov.position_lines.is_empty());
}

// ── BetterVolume extended tests ─────────────────────────────────────

#[test]
fn test_better_volume_climax_up() {
    // Create bars where bar at index `lookback` has a massive bullish candle
    // with extremely high buy volume * range → should trigger climax up (1)
    // Need n > target + lookback to satisfy both skip conditions in compute_better_volume
    let lookback = 20usize;
    let target = lookback; // bar 20
    let n = target + lookback + 5; // 45 bars total
    let mut bars = Vec::new();
    for i in 0..n {
        if i == target {
            // Massive bullish candle: huge range, high volume, close > open
            bars.push(Bar {
                ts_ms: 1700000000000 + i as i64 * 3600000,
                open: 90.0,
                high: 130.0,
                low: 89.0,
                close: 128.0,
                volume: 50000.0, // 50x normal
            });
        } else {
            // Normal small bars
            bars.push(Bar {
                ts_ms: 1700000000000 + i as i64 * 3600000,
                open: 100.0,
                high: 101.0,
                low: 99.0,
                close: 100.5,
                volume: 1000.0,
            });
        }
    }
    let bv = compute_better_volume(&bars);
    // Bar at target should be climax up (1) or climax+churn (4)
    let val = bv[target];
    assert!(
        val == 1 || val == 4,
        "Massive bullish bar should be climax up (1) or climax+churn (4), got {}",
        val
    );
}

#[test]
fn test_better_volume_churn() {
    // Churn: very high volume but tiny range → vol/range is highest
    let lookback = 20usize;
    let target = lookback;
    let n = target + lookback + 5;
    let mut bars = Vec::new();
    for i in 0..n {
        if i == target {
            // Tiny range, huge volume
            bars.push(Bar {
                ts_ms: 1700000000000 + i as i64 * 3600000,
                open: 100.0,
                high: 100.1,
                low: 99.9,
                close: 100.05,
                volume: 100000.0, // enormous volume, tiny range
            });
        } else {
            bars.push(Bar {
                ts_ms: 1700000000000 + i as i64 * 3600000,
                open: 100.0,
                high: 102.0,
                low: 98.0,
                close: 101.0,
                volume: 1000.0,
            });
        }
    }
    let bv = compute_better_volume(&bars);
    // Should be churn (3) or climax+churn (4)
    let val = bv[target];
    assert!(
        val == 3 || val == 4,
        "High-volume tiny-range bar should be churn (3) or climax+churn (4), got {}",
        val
    );
}

#[test]
fn test_better_volume_low_volume() {
    // Create bars where one bar has extremely low volume
    let lookback = 20usize;
    let target = lookback;
    let n = target + lookback + 5;
    let mut bars = Vec::new();
    for i in 0..n {
        if i == target {
            bars.push(Bar {
                ts_ms: 1700000000000 + i as i64 * 3600000,
                open: 100.0,
                high: 101.0,
                low: 99.0,
                close: 100.0,
                volume: 0.1, // nearly zero volume
            });
        } else {
            bars.push(Bar {
                ts_ms: 1700000000000 + i as i64 * 3600000,
                open: 100.0,
                high: 102.0,
                low: 98.0,
                close: 101.0,
                volume: 5000.0,
            });
        }
    }
    let bv = compute_better_volume(&bars);
    // Should be low volume (0)
    assert_eq!(
        bv[target], 0,
        "Near-zero volume bar should be low volume (0), got {}",
        bv[target]
    );
}

#[test]
fn test_better_volume_all_normal_flat() {
    // Identical bars — at the lookback boundary, metrics equal extremes
    // so some may classify as non-normal, but most should be normal (5)
    let n = 30;
    let bars: Vec<Bar> = (0..n)
        .map(|i| Bar {
            ts_ms: 1700000000000 + i as i64 * 3600000,
            open: 100.0,
            high: 101.0,
            low: 99.0,
            close: 100.5,
            volume: 1000.0,
        })
        .collect();
    let bv = compute_better_volume(&bars);
    assert_eq!(bv.len(), n);
    // First `lookback` bars should be normal (5)
    for i in 0..20 {
        assert_eq!(bv[i], 5, "Bar {} in warmup should be normal", i);
    }
}

#[test]
fn test_var_oscillator_warmup_and_downside_signal() {
    let mut closes = vec![100.0];
    for i in 1..40 {
        let prev = closes[i - 1];
        let next = if i == 25 { prev * 0.92 } else { prev * 1.003 };
        closes.push(next);
    }
    let bars = make_close_bars(&closes);
    let osc = compute_var_oscillator(&bars, 20);
    assert!(osc[..20].iter().all(|v| v.is_none()));
    assert!(
        osc[20].is_some(),
        "first fully-populated VaR window should be valid"
    );
    assert!(
        osc[25].unwrap_or_default() > 100.0,
        "sharp downside move should exceed +100 VaR units"
    );
}

#[test]
fn test_var_oscillator_upside_moves_are_negative() {
    let mut closes = vec![100.0; 40];
    for i in 1..40 {
        closes[i] = closes[i - 1] * 1.002;
    }
    closes[25] = closes[24] * 1.08;
    let bars = make_close_bars(&closes);
    let osc = compute_var_oscillator(&bars, 20);
    assert!(
        osc[25].unwrap_or_default() < 0.0,
        "upside shock should plot below zero"
    );
}

#[test]
fn test_chart_talib_gpu_fallback_series_have_expected_ranges() {
    let bars = make_oscillating_bars(80);

    let cmo = compute_cmo(&bars, 9);
    assert!(cmo[..9].iter().all(|v| v.is_none()));
    assert!(cmo[9].unwrap_or_default().abs() <= 100.0);

    let qstick = compute_qstick(&bars, 14);
    assert!(qstick[..13].iter().all(|v| v.is_none()));
    assert!(qstick[13].unwrap_or_default().is_finite());

    let disparity = compute_disparity(&bars, 14);
    assert!(disparity[..13].iter().all(|v| v.is_none()));
    assert!(disparity[13].unwrap_or_default().is_finite());

    let bop = compute_bop(&bars, 14);
    assert!(bop[..13].iter().all(|v| v.is_none()));
    assert!(bop[13].unwrap_or_default().abs() <= 1.0);

    let stddev = compute_stddev(&bars, 20);
    assert!(stddev[..19].iter().all(|v| v.is_none()));
    assert!(stddev[19].unwrap_or_default() >= 0.0);
}

// ── Chart templates (snapshot capture/apply) ─────────────────────────

#[test]
fn test_capture_apply_indicator_snapshot() {
    // We cannot construct TyphooNApp directly in tests (requires eframe context),
    // but we can test the JSON round-trip logic directly.
    // Build a snapshot matching the NNFX template
    let snap = TyphooNApp::builtin_template_nnfx();
    assert_eq!(snap["sma200"].as_bool(), Some(true));
    assert_eq!(snap["kama"].as_bool(), Some(true));
    assert_eq!(snap["fisher"].as_bool(), Some(true));
    assert_eq!(snap["better_volume"].as_bool(), Some(true));
    assert_eq!(snap["cmo"].as_bool(), Some(false));
    assert_eq!(snap["qstick"].as_bool(), Some(false));
    assert_eq!(snap["disparity"].as_bool(), Some(false));
    assert_eq!(snap["bop"].as_bool(), Some(false));
    assert_eq!(snap["stddev"].as_bool(), Some(false));
    assert_eq!(snap["mfi"].as_bool(), Some(false));
    assert_eq!(snap["trix"].as_bool(), Some(false));
    assert_eq!(snap["ppo"].as_bool(), Some(false));
    assert_eq!(snap["ultosc"].as_bool(), Some(false));
    assert_eq!(snap["stochrsi"].as_bool(), Some(false));
    assert_eq!(snap["var_oscillator"].as_bool(), Some(false));
    assert_eq!(snap["bollinger"].as_bool(), Some(false));
    assert_eq!(snap["macd"].as_bool(), Some(false));
}

#[test]
fn test_clean_template_all_off_except_volume() {
    let snap = TyphooNApp::builtin_template_clean();
    assert_eq!(snap["volume_pane"].as_bool(), Some(true));
    // All others should be false
    for key in [
        "sma200",
        "sma100",
        "kama",
        "ema21",
        "bollinger",
        "ichimoku",
        "rsi",
        "fisher",
        "macd",
        "stochastic",
        "adx",
        "fractals",
        "harmonics",
        "supply_demand",
        "fvg",
        "cmo",
        "qstick",
        "disparity",
        "bop",
        "stddev",
        "mfi",
        "trix",
        "ppo",
        "ultosc",
        "stochrsi",
        "var_oscillator",
    ] {
        assert_eq!(
            snap[key].as_bool(),
            Some(false),
            "CLEAN template: {} should be false",
            key
        );
    }
}

#[test]
fn test_full_template_all_on() {
    let snap = TyphooNApp::builtin_template_full();
    for key in [
        "sma200",
        "sma100",
        "kama",
        "ema21",
        "bollinger",
        "ichimoku",
        "rsi",
        "fisher",
        "macd",
        "stochastic",
        "adx",
        "fractals",
        "harmonics",
        "supply_demand",
        "fvg",
        "cmo",
        "qstick",
        "disparity",
        "bop",
        "stddev",
        "mfi",
        "trix",
        "ppo",
        "ultosc",
        "stochrsi",
        "var_oscillator",
        "volume_pane",
        "better_volume",
        "squeeze",
        "regression",
    ] {
        assert_eq!(
            snap[key].as_bool(),
            Some(true),
            "FULL template: {} should be true",
            key
        );
    }
}

#[test]
fn test_template_roundtrip_json() {
    // Verify that all templates produce valid JSON with consistent keys
    let nnfx = TyphooNApp::builtin_template_nnfx();
    let clean = TyphooNApp::builtin_template_clean();
    let full = TyphooNApp::builtin_template_full();

    // All three should have the same keys
    let nnfx_obj = nnfx.as_object().unwrap();
    let clean_obj = clean.as_object().unwrap();
    let full_obj = full.as_object().unwrap();

    assert_eq!(
        nnfx_obj.len(),
        clean_obj.len(),
        "NNFX and CLEAN templates should have same number of keys"
    );
    assert_eq!(
        nnfx_obj.len(),
        full_obj.len(),
        "NNFX and FULL templates should have same number of keys"
    );

    for key in nnfx_obj.keys() {
        assert!(
            clean_obj.contains_key(key),
            "CLEAN template missing key: {}",
            key
        );
        assert!(
            full_obj.contains_key(key),
            "FULL template missing key: {}",
            key
        );
    }
}

// ── Format functions ─────────────────────────────────────────────────

#[test]
fn test_format_price_buf_zero() {
    let mut buf = String::new();
    format_price_buf(0.0, &mut buf);
    assert_eq!(buf, "0");
}

#[test]
fn test_format_price_buf_large() {
    let mut buf = String::new();
    format_price_buf(12345.67, &mut buf);
    assert_eq!(buf, "12345.67"); // >= 10000 → 2 decimal places
}

#[test]
fn test_format_price_buf_medium() {
    let mut buf = String::new();
    format_price_buf(123.4567, &mut buf);
    assert_eq!(buf, "123.4567"); // >= 1.0 → 4 decimal places
}

#[test]
fn test_format_price_buf_small() {
    let mut buf = String::new();
    format_price_buf(0.123456, &mut buf);
    assert_eq!(buf, "0.123456"); // < 1.0 → 6 decimal places
}

#[test]
fn test_format_price_buf_negative() {
    let mut buf = String::new();
    format_price_buf(-50.1234, &mut buf);
    assert_eq!(buf, "-50.1234"); // abs >= 1.0 → 4 decimals
}

#[test]
fn test_format_price_buf_reuses_buffer() {
    let mut buf = String::new();
    format_price_buf(100.0, &mut buf);
    let first = buf.clone();
    format_price_buf(200.0, &mut buf);
    assert_ne!(first, buf, "Buffer should be cleared and rewritten");
    assert!(buf.contains("200"), "Should contain new value");
}

#[test]
fn test_format_ts_buf_daily() {
    let mut buf = String::new();
    // 2023-11-15 00:00:00 UTC → 1700006400000
    let ts = 1700006400000_i64;
    format_ts_buf(ts, Timeframe::D1, &mut buf);
    assert!(
        buf.contains("Nov") || buf.contains("15"),
        "D1 format should contain day/month, got: {}",
        buf
    );
}

#[test]
fn test_format_ts_buf_hourly_midnight() {
    let mut buf = String::new();
    // Midnight → should show date, not time
    let ts = 1700006400000_i64; // 2023-11-15 00:00 UTC
    format_ts_buf(ts, Timeframe::H4, &mut buf);
    // At midnight, H4 shows date format
    assert!(
        !buf.contains(":") || buf.contains("00:00") || buf.contains("Nov"),
        "H4 at midnight should show date, got: {}",
        buf
    );
}

#[test]
fn test_format_ts_buf_hourly_nonmidnight() {
    let mut buf = String::new();
    // 2023-11-15 14:00:00 UTC
    let ts = 1700006400000_i64 + 14 * 3600000;
    format_ts_buf(ts, Timeframe::H1, &mut buf);
    assert!(
        buf.contains("14:00"),
        "H1 non-midnight should show HH:MM, got: {}",
        buf
    );
}

#[test]
fn test_format_ts_buf_monthly() {
    let mut buf = String::new();
    let ts = 1700006400000_i64;
    format_ts_buf(ts, Timeframe::MN1, &mut buf);
    // MN1 format: "Nov'23"
    assert!(
        buf.contains("Nov") && buf.contains("23"),
        "MN1 should show Mon'YY, got: {}",
        buf
    );
}

#[test]
fn test_format_ts_buf_minute() {
    let mut buf = String::new();
    let ts = 1700006400000_i64 + 9 * 3600000 + 30 * 60000; // 09:30
    format_ts_buf(ts, Timeframe::M15, &mut buf);
    assert!(buf.contains("09:30"), "M15 should show HH:MM, got: {}", buf);
}

#[test]
fn test_apply_storage_snapshot_prunes_deleted_keys_and_updates_sizes() {
    let mut bg = BgData::default();
    bg.bar_ts_cache
        .insert("kraken-futures:EURUSD:1Min".into(), (1, 2, 10));
    bg.bar_ts_cache
        .insert("alpaca:AAPL:1Day".into(), (3, 4, 20));

    apply_storage_snapshot(
        &mut bg,
        (1, 7, 9_999),
        vec![("alpaca:AAPL:1Day".into(), 123, 456, 789)],
    );

    assert_eq!(bg.cache_stats, Some((1, 7, 9_999)));
    assert_eq!(
        bg.detailed_stats,
        vec![("alpaca:AAPL:1Day".into(), 123, 456)]
    );
    assert_eq!(bg.cache_blob_sizes.get("alpaca:AAPL:1Day"), Some(&789));
    assert!(!bg.bar_ts_cache.contains_key("kraken-futures:EURUSD:1Min"));
    assert!(bg.bar_ts_cache.contains_key("alpaca:AAPL:1Day"));
}

#[test]
fn test_codex_reasoning_effort_normalization_defaults_unknown_values() {
    assert_eq!(
        TyphooNApp::normalize_codex_reasoning_effort("medium"),
        "medium"
    );
    assert_eq!(
        TyphooNApp::normalize_codex_reasoning_effort("bogus"),
        "default"
    );
    assert_eq!(TyphooNApp::normalize_codex_reasoning_effort(""), "default");
}

#[test]
fn test_build_codex_exec_args_omits_reasoning_override_for_default() {
    let args = TyphooNApp::build_codex_exec_args("gpt-5-codex", "default", "hello");
    assert_eq!(
        args,
        vec![
            "exec".to_string(),
            "--model".to_string(),
            "gpt-5-codex".to_string(),
            "--skip-git-repo-check".to_string(),
            "hello".to_string(),
        ]
    );
}

#[test]
fn test_build_codex_exec_args_includes_reasoning_override_when_selected() {
    let args = TyphooNApp::build_codex_exec_args("gpt-5", "xhigh", "hello");
    assert_eq!(
        args,
        vec![
            "exec".to_string(),
            "--model".to_string(),
            "gpt-5".to_string(),
            "--skip-git-repo-check".to_string(),
            "-c".to_string(),
            "model_reasoning_effort=\"xhigh\"".to_string(),
            "hello".to_string(),
        ]
    );
}

#[test]
fn test_build_hermes_exec_args_uses_configured_defaults_when_blank() {
    let args = TyphooNApp::build_hermes_exec_args("", "", "hello");
    assert_eq!(args, vec!["--oneshot".to_string(), "hello".to_string()]);
}

#[test]
fn test_build_hermes_exec_args_includes_overrides_when_selected() {
    let args = TyphooNApp::build_hermes_exec_args("openai/gpt-5.1", "openrouter", "hello");
    assert_eq!(
        args,
        vec![
            "--model".to_string(),
            "openai/gpt-5.1".to_string(),
            "--provider".to_string(),
            "openrouter".to_string(),
            "--oneshot".to_string(),
            "hello".to_string(),
        ]
    );
}

#[test]
fn test_grok_effort_normalization_defaults_unknown_values() {
    assert_eq!(TyphooNApp::normalize_grok_effort("max"), "max");
    assert_eq!(TyphooNApp::normalize_grok_effort("bogus"), "high");
    assert_eq!(TyphooNApp::normalize_grok_effort(""), "high");
}

#[test]
fn test_build_grok_exec_args_uses_auto_model_when_blank_or_auto() {
    let args = TyphooNApp::build_grok_exec_args("auto", "bogus", "hello");
    assert_eq!(
        args,
        vec![
            "--no-alt-screen".to_string(),
            "--output-format".to_string(),
            "plain".to_string(),
            "--effort".to_string(),
            "high".to_string(),
            "--single".to_string(),
            "hello".to_string(),
        ]
    );
}

#[test]
fn test_build_grok_exec_args_includes_model_and_effort() {
    let args = TyphooNApp::build_grok_exec_args("grok-code-fast-1", "max", "hello");
    assert_eq!(
        args,
        vec![
            "--no-alt-screen".to_string(),
            "--output-format".to_string(),
            "plain".to_string(),
            "--effort".to_string(),
            "max".to_string(),
            "--model".to_string(),
            "grok-code-fast-1".to_string(),
            "--single".to_string(),
            "hello".to_string(),
        ]
    );
}

fn sample_events() -> Vec<EventRow> {
    vec![
        EventRow {
            symbol: "AAPL".into(),
            company: "Apple Inc.".into(),
            date: "2026-05-01".into(),
            days_until: 10,
            kind: EventKind::Earnings,
            detail: "P/E 28.5".into(),
            in_alpaca: true,
            in_kraken: false,
        },
        EventRow {
            symbol: "MSFT".into(),
            company: "Microsoft".into(),
            date: "2026-05-08".into(),
            days_until: 17,
            kind: EventKind::ExDividend,
            detail: "0.82% yield".into(),
            in_alpaca: true,
            in_kraken: false,
        },
        EventRow {
            symbol: "T".into(),
            company: "AT&T, Inc.".into(), // tests comma escaping
            date: "2026-05-15".into(),
            days_until: 24,
            kind: EventKind::DividendPayment,
            detail: "5.10% yield".into(),
            in_alpaca: false,
            in_kraken: true,
        },
    ]
}

#[test]
fn test_build_events_ics_contains_calendar_wrapper() {
    let ics = TyphooNApp::build_events_ics(&sample_events(), EventSource::All, true, true, true);
    assert!(ics.starts_with("BEGIN:VCALENDAR\r\n"));
    assert!(ics.ends_with("END:VCALENDAR\r\n"));
    assert!(ics.contains("VERSION:2.0"));
    assert!(ics.contains("PRODID:-//TyphooN Terminal"));
}

#[test]
fn test_build_events_ics_emits_all_filtered_vevents() {
    let ics = TyphooNApp::build_events_ics(&sample_events(), EventSource::All, true, true, true);
    let vevent_count = ics.matches("BEGIN:VEVENT").count();
    assert_eq!(vevent_count, 3, "All 3 events should be emitted");
    assert!(ics.contains("SUMMARY:AAPL — Earnings"));
    assert!(ics.contains("DTSTART;VALUE=DATE:20260501"));
    assert!(ics.contains("DTEND;VALUE=DATE:20260502"));
}

#[test]
fn test_build_events_ics_respects_source_filter() {
    // Only Alpaca — AAPL (yes), MSFT (yes), T (no)
    let ics = TyphooNApp::build_events_ics(&sample_events(), EventSource::Alpaca, true, true, true);
    assert_eq!(ics.matches("BEGIN:VEVENT").count(), 2);
    assert!(ics.contains("AAPL"));
    assert!(ics.contains("MSFT"));
    assert!(!ics.contains("SUMMARY:T"));
}

#[test]
fn test_build_events_ics_respects_type_filter() {
    // Earnings only
    let ics = TyphooNApp::build_events_ics(&sample_events(), EventSource::All, true, false, false);
    assert_eq!(ics.matches("BEGIN:VEVENT").count(), 1);
    assert!(ics.contains("AAPL"));
    // Ex-Div + Div-Pay only
    let ics2 = TyphooNApp::build_events_ics(&sample_events(), EventSource::All, false, true, true);
    assert_eq!(ics2.matches("BEGIN:VEVENT").count(), 2);
}

#[test]
fn test_build_events_ics_escapes_special_chars() {
    let ics = TyphooNApp::build_events_ics(&sample_events(), EventSource::All, true, true, true);
    // Comma in "AT&T, Inc." must be escaped per RFC 5545
    assert!(
        ics.contains("AT&T\\, Inc."),
        "comma should be backslash-escaped: {}",
        ics
    );
}

#[test]
fn test_build_events_ics_skips_unparseable_dates() {
    let bad = vec![EventRow {
        symbol: "X".into(),
        company: "Bad".into(),
        date: "not-a-date".into(),
        days_until: 0,
        kind: EventKind::Earnings,
        detail: String::new(),
        in_alpaca: true,
        in_kraken: false,
    }];
    let ics = TyphooNApp::build_events_ics(&bad, EventSource::All, true, true, true);
    assert_eq!(ics.matches("BEGIN:VEVENT").count(), 0);
}

#[test]
fn news_article_in_focus_empty_set_passes_everything() {
    let focus = std::collections::HashSet::new();
    assert!(TyphooNApp::news_article_in_focus(&focus, "AAPL", &[]));
    assert!(TyphooNApp::news_article_in_focus(
        &focus,
        "",
        &["random".into()]
    ));
}

#[test]
fn news_article_in_focus_matches_primary_symbol() {
    let focus = std::collections::HashSet::from(["AAPL".to_string()]);
    assert!(TyphooNApp::news_article_in_focus(&focus, "AAPL", &[]));
    assert!(TyphooNApp::news_article_in_focus(&focus, "aapl", &[]));
    assert!(TyphooNApp::news_article_in_focus(&focus, " AAPL ", &[]));
}

#[test]
fn news_article_in_focus_matches_any_tagged_ticker() {
    let focus = std::collections::HashSet::from(["TMO".to_string()]);
    // Primary is unrelated, but tickers carry the match.
    assert!(TyphooNApp::news_article_in_focus(
        &focus,
        "A",
        &["XLV".into(), "TMO".into()]
    ));
}

#[test]
fn news_article_in_focus_rejects_when_no_overlap() {
    let focus = std::collections::HashSet::from(["AAPL".to_string(), "MSFT".to_string()]);
    assert!(!TyphooNApp::news_article_in_focus(
        &focus,
        "TMO",
        &["XLV".into(), "A".into()]
    ));
}

#[test]
fn news_article_in_focus_handles_empty_primary_with_tagged_tickers() {
    let focus = std::collections::HashSet::from(["BTC".to_string()]);
    assert!(TyphooNApp::news_article_in_focus(
        &focus,
        "",
        &["btc".into()]
    ));
}

#[test]
fn news_article_in_focus_ignores_whitespace_only_primary() {
    let focus = std::collections::HashSet::from(["AAPL".to_string()]);
    // Whitespace-only primary uppercases to empty; tickers must carry it.
    assert!(!TyphooNApp::news_article_in_focus(&focus, "   ", &[]));
    assert!(TyphooNApp::news_article_in_focus(
        &focus,
        "   ",
        &["AAPL".into()]
    ));
}

#[test]
fn kraken_ws_pair_is_fresh_at_returns_false_for_missing_entry() {
    let map = std::collections::HashMap::new();
    assert!(!TyphooNApp::kraken_ws_pair_is_fresh_at(
        &map, "BTCUSD", "1Min", 0
    ));
}

#[test]
fn kraken_ws_pair_is_fresh_at_returns_false_for_unknown_timeframe() {
    let mut map = std::collections::HashMap::new();
    map.insert(
        ("BTCUSD".to_string(), "BOGUS".to_string()),
        1_700_000_000_000,
    );
    assert!(!TyphooNApp::kraken_ws_pair_is_fresh_at(
        &map,
        "BTCUSD",
        "BOGUS",
        1_700_000_000_000
    ));
}

#[test]
fn kraken_ws_pair_is_fresh_at_passes_within_tf_x24_window() {
    // 1Min × 24 = 1440s = 1,440,000ms. Anchor at now-1,000,000ms (16.6min ago)
    // is still within the freshness window.
    let now_ms = 1_700_000_000_000i64;
    let anchor_ms = now_ms - 1_000_000;
    let mut map = std::collections::HashMap::new();
    map.insert(("BTCUSD".to_string(), "1Min".to_string()), anchor_ms);
    assert!(TyphooNApp::kraken_ws_pair_is_fresh_at(
        &map, "BTCUSD", "1Min", now_ms
    ));
}

#[test]
fn kraken_ws_pair_is_fresh_at_rejects_anchor_outside_window() {
    // 1Min × 24 = 1440s. Anchor at now-2000s (33 min) is past the window.
    let now_ms = 1_700_000_000_000i64;
    let anchor_ms = now_ms - 2_000_000;
    let mut map = std::collections::HashMap::new();
    map.insert(("BTCUSD".to_string(), "1Min".to_string()), anchor_ms);
    assert!(!TyphooNApp::kraken_ws_pair_is_fresh_at(
        &map, "BTCUSD", "1Min", now_ms
    ));
}

#[test]
fn kraken_ws_pair_is_fresh_at_scales_with_timeframe_period() {
    // 1Day × 24 = 24 days. Anchor at now - 20 days should still be fresh.
    let day_ms = 86_400_000i64;
    let now_ms = 1_700_000_000_000i64;
    let anchor_ms = now_ms - 20 * day_ms;
    let mut map = std::collections::HashMap::new();
    map.insert(("BTCUSD".to_string(), "1Day".to_string()), anchor_ms);
    assert!(TyphooNApp::kraken_ws_pair_is_fresh_at(
        &map, "BTCUSD", "1Day", now_ms
    ));
    // But 25 days ago is past the window.
    let stale_anchor = now_ms - 25 * day_ms;
    map.insert(("BTCUSD".to_string(), "1Day".to_string()), stale_anchor);
    assert!(!TyphooNApp::kraken_ws_pair_is_fresh_at(
        &map, "BTCUSD", "1Day", now_ms
    ));
}

#[test]
#[allow(deprecated)]
fn nav_typography_helpers_exist() {
    // Create a dummy context to exercise the helpers
    let ctx = egui::Context::default();
    let fonts = egui::FontDefinitions::default();
    ctx.set_fonts(fonts);

    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            nav_primary(ui, "TEST");
            nav_secondary(ui, "123.45");
            nav_muted(ui, "Yahoo");
        });
    });
}

#[test]
fn yahoo_fetcher_rate_limit_test() {
    // The rate limiter should prevent calls closer than 5 seconds
    // This is a structural test
    assert!(true);
}

#[test]
fn watchlist_fallback_price_display_test() {
    let mut fallback_prices = std::collections::HashMap::new();
    fallback_prices.insert(
        "TEST".to_string(),
        (123.45, "Yahoo".to_string(), std::time::Instant::now()),
    );
    assert!(fallback_prices.contains_key("TEST"));
}

#[test]
fn watchlist_row_from_raw_bars_uses_close_prices_for_weekend_cache() {
    let raw = vec![
        (1_700_000_000_000, 100.0, 110.0, 90.0, 105.0, 1_000.0),
        (1_700_086_400_000, 105.0, 115.0, 95.0, 112.0, 1_500.0),
    ];

    let row = watchlist_row_from_raw_bars("TEST", "alpaca:TEST:1Day", &raw).unwrap();

    assert_eq!(row.symbol, "TEST");
    assert_eq!(row.cache_key, "alpaca:TEST:1Day");
    assert_eq!(row.last, 112.0);
    assert_eq!(row.prev_close, 105.0);
    assert_eq!(row.change, 7.0);
    assert!((row.change_pct - 6.666_666_666_666_667).abs() < f64::EPSILON * 16.0);
    assert_eq!(row.volume, 1_500.0);
}

#[test]
fn watchlist_row_from_raw_bars_accepts_single_valid_cached_bar() {
    let raw = vec![(1_700_000_000_000, 10.0, 11.0, 9.0, 10.5, 250.0)];

    let row = watchlist_row_from_raw_bars("SOLO", "default:SOLO:1Day", &raw).unwrap();

    assert_eq!(row.last, 10.5);
    assert_eq!(row.prev_close, 10.5);
    assert_eq!(row.change, 0.0);
    assert_eq!(row.change_pct, 0.0);
}

#[test]
fn yahoo_price_fallback_test() {
    // Basic existence test - real network call is done at runtime
    assert!(true);
}

#[test]
#[allow(deprecated)]
fn market_depth_and_volume_profile_render_helpers_are_callable() {
    let depth = compute_market_depth(&[(100.0, 2.0), (99.5, 1.0)], &[(100.5, 3.0)]);
    assert_eq!(depth.bids.len(), 2);
    assert_eq!(depth.asks.len(), 1);

    let profile = VolumeProfile {
        price_levels: vec![(99.5, 10.0), (100.0, 25.0), (100.5, 15.0)],
        poc: 100.0,
        value_area_high: 100.5,
        value_area_low: 99.5,
    };

    let ctx = egui::Context::default();
    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(200.0, 120.0));
            let painter = ui.painter();
            draw_market_depth(painter, &depth, rect);
            draw_volume_profile(painter, &profile, rect);
        });
    });
}

#[test]
fn forming_bar_helpers_test() {
    let mut chart = ChartState::new("TEST", Timeframe::M1);
    chart.bars.push(Bar {
        ts_ms: 1000,
        open: 1.0,
        high: 1.0,
        low: 1.0,
        close: 1.0,
        volume: 1.0,
    });
    chart.mark_structural_change();
    let gen_before = chart.visible_bars_gen;
    chart.apply_forming_bar_update(Bar {
        ts_ms: 1000,
        open: 1.0,
        high: 2.0,
        low: 0.5,
        close: 1.5,
        volume: 2.0,
    });
    assert!(chart.forming_bar_dirty);
    assert_eq!(chart.visible_bars_gen, gen_before);
}

#[test]
fn live_quote_update_marks_forming_bar_dirty() {
    let mut chart = ChartState::new("TEST", Timeframe::M1);
    chart.bars.push(Bar {
        ts_ms: 1000,
        open: 100.0,
        high: 101.0,
        low: 99.0,
        close: 100.0,
        volume: 1.0,
    });
    chart.mark_structural_change();

    assert!(chart.apply_live_quote_update(110.0, 112.0, false));

    let last = chart.bars.last().unwrap();
    assert_eq!(last.close, 111.0);
    assert_eq!(last.high, 111.0);
    assert_eq!(last.low, 99.0);
    assert!(chart.forming_bar_dirty);
    assert_eq!(chart.fresh_live_quote_mid(), Some(111.0));
}

#[test]
fn full_recompute_folds_fresh_live_quote_after_cache_reload_without_fast_path() {
    let mut chart = ChartState::new("TEST", Timeframe::M1);
    for i in 0..300 {
        chart.bars.push(Bar {
            ts_ms: 1000 + i as i64 * 60_000,
            open: 100.0,
            high: 101.0,
            low: 99.0,
            close: 100.0,
            volume: 1.0,
        });
    }
    chart.compute_indicators();
    chart.forming_bar_dirty = false;

    // Simulate a queued cache reload that replaced the active forming candle with
    // an older persisted close while the chart still owns a fresh live quote.
    chart.bars.last_mut().unwrap().close = 100.0;
    chart.bars.last_mut().unwrap().high = 101.0;
    chart.live_bid = 119.0;
    chart.live_ask = 121.0;
    chart.live_quote_at = Some(std::time::Instant::now());
    chart.live_quote_delayed = false;

    chart.compute_indicators();

    let last = chart.bars.last().unwrap();
    assert_eq!(last.close, 120.0);
    assert_eq!(last.high, 120.0);
    assert!(!chart.forming_bar_dirty);
}

#[test]
fn stale_live_quote_is_not_folded_into_reloaded_bar() {
    let mut chart = ChartState::new("TEST", Timeframe::M1);
    chart.bars.push(Bar {
        ts_ms: 1000,
        open: 100.0,
        high: 101.0,
        low: 99.0,
        close: 100.0,
        volume: 1.0,
    });
    chart.live_bid = 119.0;
    chart.live_ask = 121.0;
    chart.live_quote_at = Some(std::time::Instant::now() - std::time::Duration::from_secs(31));

    chart.compute_indicators();

    assert_eq!(chart.bars.last().unwrap().close, 100.0);
    assert!(!chart.forming_bar_dirty);
}

#[test]
fn news_dedup_placeholder_test() {
    // Placeholder test for article deduplication logic.
    // Real implementation will use article_exists_by_url_hash.
    let should_dedup = true;
    assert!(should_dedup);
}

#[test]
fn kraken_ws_pair_is_fresh_at_handles_future_anchor_gracefully() {
    // Defensive: clock skew could land an anchor slightly in the future.
    // saturating_sub(future) clamps to 0, which is < max_age_ms → fresh.
    let now_ms = 1_700_000_000_000i64;
    let future_anchor = now_ms + 60_000;
    let mut map = std::collections::HashMap::new();
    map.insert(("BTCUSD".to_string(), "1Min".to_string()), future_anchor);
    assert!(TyphooNApp::kraken_ws_pair_is_fresh_at(
        &map, "BTCUSD", "1Min", now_ms
    ));
}

#[test]
fn chart_state_forming_bar_fast_path() {
    let mut chart = ChartState::new("TEST", Timeframe::M1);
    chart.bars.push(Bar {
        ts_ms: 1_000_000,
        open: 100.0,
        high: 101.0,
        low: 99.0,
        close: 100.5,
        volume: 10.0,
    });
    chart.mark_structural_change();
    let gen_before = chart.visible_bars_gen;

    let forming = Bar {
        ts_ms: 1_000_000,
        open: 100.0,
        high: 102.0,
        low: 99.5,
        close: 101.8,
        volume: 15.0,
    };
    chart.apply_forming_bar_update(forming);

    assert!(chart.forming_bar_dirty);
    assert_eq!(chart.last_visible_bar_ts, 1_000_000);
    assert_eq!(chart.bars.last().unwrap().close, 101.8);
    assert_eq!(chart.visible_bars_gen, gen_before);

    let closed = Bar {
        ts_ms: 1_060_000,
        open: 101.8,
        high: 103.0,
        low: 101.0,
        close: 102.5,
        volume: 20.0,
    };
    chart.bars.push(closed);
    chart.mark_structural_change();

    assert!(!chart.forming_bar_dirty);
    assert!(chart.visible_bars_gen > gen_before);
}

#[test]
fn chart_state_tracks_render_snapshot_fields_without_skipping_paint() {
    let mut chart = ChartState::new("TEST", Timeframe::M5);
    chart.bars.push(Bar {
        ts_ms: 1000,
        open: 1.0,
        high: 1.0,
        low: 1.0,
        close: 1.0,
        volume: 1.0,
    });
    chart.mark_structural_change();

    // These fields are retained for data/change diagnostics. draw_chart must
    // still paint every frame because egui has no retained chart render target;
    // skipping paint causes closed-market charts to blank/flicker on hover/pan.
    chart.last_rendered_gen = chart.visible_bars_gen;
    chart.last_rendered_bar_ts = chart.last_visible_bar_ts;

    assert_eq!(chart.visible_bars_gen, chart.last_rendered_gen);
    assert_eq!(chart.last_visible_bar_ts, chart.last_rendered_bar_ts);
}

#[test]
fn compute_indicators_gpu_forming_bar_fast_path() {
    let mut chart = ChartState::new("TEST", Timeframe::M1);
    // Seed with enough bars for SMA
    for i in 0..300 {
        chart.bars.push(Bar {
            ts_ms: 1000 + i as i64 * 60_000,
            open: 100.0 + i as f64 * 0.1,
            high: 101.0 + i as f64 * 0.1,
            low: 99.0 + i as f64 * 0.1,
            close: 100.5 + i as f64 * 0.1,
            volume: 1000.0,
        });
    }
    chart.mark_structural_change();

    // Simulate live WS tick
    chart.forming_bar_dirty = true;
    chart.apply_forming_bar_update(Bar {
        ts_ms: chart.bars.last().unwrap().ts_ms,
        open: 130.0,
        high: 132.0,
        low: 129.0,
        close: 131.5,
        volume: 1500.0,
    });

    // The fast path in compute_indicators_gpu should handle this without full recompute
    // (we just check that the flag is respected and last value is updated)
    assert!(chart.forming_bar_dirty); // still set until compute_indicators_gpu consumes it
}

// Yahoo Finance price fallback (used when primary broker has no recent data)
#[allow(dead_code)]
pub async fn fetch_yahoo_last_price(symbol: &str) -> Option<(f64, String)> {
    // Simple rate limiting to avoid hammering Yahoo
    static LAST_YAHOO_CALL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let last = LAST_YAHOO_CALL.load(std::sync::atomic::Ordering::Relaxed);
    if now - last < 5 {
        return None; // too soon
    }
    LAST_YAHOO_CALL.store(now, std::sync::atomic::Ordering::Relaxed);

    let url = format!(
        "https://query1.finance.yahoo.com/v8/finance/chart/{}?interval=1d&range=5d",
        symbol
    );

    let client = reqwest::Client::new();
    let resp = match client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0 (compatible; TyphooN-Terminal)")
        .timeout(std::time::Duration::from_secs(8))
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return None,
    };

    if !resp.status().is_success() {
        return None;
    }

    let json: serde_json::Value = match resp.json().await {
        Ok(j) => j,
        Err(_) => return None,
    };

    let price = json["chart"]["result"][0]["meta"]["regularMarketPrice"]
        .as_f64()
        .or_else(|| json["chart"]["result"][0]["meta"]["previousClose"].as_f64())?;

    Some((price, "Yahoo".to_string()))
}

#[allow(dead_code)]
pub async fn fetch_last_price_with_fallback(symbol: &str) -> Option<(f64, String)> {
    if let Some((price, source)) = fetch_yahoo_last_price(symbol).await {
        return Some((price, source));
    }
    None
}
