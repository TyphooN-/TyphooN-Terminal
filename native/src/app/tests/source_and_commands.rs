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
    // WOK-style failure: Alpaca/Kraken trusted history can be raw or only
    // partially adjusted while Yahoo/TradingView-style chart data is adjusted
    // across multiple reverse-split eras. Recent bars agree, but older eras are
    // stable 100x / 10,000x steps. The merge should adopt the adjusted depth OHLC
    // for those eras instead of dropping it as an inconsistent splice.
    let day = 86_400_000i64;
    let alpaca: Vec<(i64, f64, f64, f64, f64, f64)> = (1..=80)
        .map(|d| (d as i64 * day, 1.0, 1.1, 0.9, 1.0, 100.0))
        .collect();
    let yahoo: Vec<(i64, f64, f64, f64, f64, f64)> = (1..=80)
        .map(|d| {
            let c = if d <= 20 {
                10_000.0
            } else if d <= 40 {
                100.0
            } else {
                1.0
            };
            (d as i64 * day, c, c * 1.1, c * 0.9, c, 50.0)
        })
        .collect();

    let merged =
        chart_merge_equity_raw_bars("1Day", &[("yahoo-chart", &yahoo), ("alpaca", &alpaca)], &[]);
    let by_ts: std::collections::HashMap<i64, f64> =
        merged.iter().map(|b| (b.ts_ms, b.close)).collect();

    assert!((by_ts[&(5 * day)] - 10_000.0).abs() < 1e-6);
    assert!((by_ts[&(30 * day)] - 100.0).abs() < 1e-6);
    assert!((by_ts[&(60 * day)] - 1.0).abs() < 1e-6);
    assert_eq!(merged.len(), 80);
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

    let merged =
        chart_merge_equity_raw_bars("1Hour", &[("yahoo-chart", &yahoo), ("alpaca", &alpaca)], &[]);
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
    assert_eq!(splits[0].ex_ts_ms, expected_ts, "ex-date 2025-12-29 00:00 UTC");
    assert!(
        chart_curated_known_splits("AAPL").is_empty(),
        "curated table is opt-in per symbol"
    );

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

