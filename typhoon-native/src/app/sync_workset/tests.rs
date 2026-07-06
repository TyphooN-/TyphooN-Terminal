use super::*;

fn alpaca_sync_target_bars(tf: &str) -> Option<u32> {
    normalize_sync_timeframe_key(tf).map(|_| u32::MAX)
}

fn provider_window_target_bars(tf: &str) -> Option<u32> {
    normalize_sync_timeframe_key(tf)?;
    None
}

#[test]
fn provider_window_native_lane_can_refresh_after_one_period() {
    let now_s = 1_700_000_000i64;
    let state = Some(SyncCacheState {
        last_bar_ts_s: now_s - 2 * 86_400,
        write_ts_s: now_s - 3600,
        bar_count: 40,
    });

    assert!(
        classify_alpaca_sync_candidate(
            now_s,
            "TNDM",
            "1Day",
            state,
            false,
            provider_window_target_bars,
        )
        .is_none()
    );

    let candidate = classify_alpaca_sync_candidate_with_stale_multiplier(
        now_s,
        "TNDM",
        "1Day",
        state,
        false,
        1,
        provider_window_target_bars,
    )
    .expect("one-period native provider-window refresh should be due");
    assert_eq!(candidate.bucket, AlpacaSyncBucket::Stale);
}

#[test]
fn normalize_sync_timeframe_key_accepts_short_and_cache_labels() {
    assert_eq!(normalize_sync_timeframe_key("M1"), Some("1Min"));
    assert_eq!(normalize_sync_timeframe_key("1Min"), Some("1Min"));
    assert_eq!(normalize_sync_timeframe_key("mn1"), Some("1Month"));
    assert_eq!(normalize_sync_timeframe_key("1Month"), Some("1Month"));
    assert_eq!(normalize_sync_timeframe_key("bogus"), None);
}

#[test]
fn sync_timeframe_short_label_maps_cache_suffixes() {
    assert_eq!(sync_timeframe_short_label("1Min"), "M1");
    assert_eq!(sync_timeframe_short_label("1Hour"), "H1");
    assert_eq!(sync_timeframe_short_label("1Month"), "MN1");
}

#[test]
fn ordered_sync_timeframes_high_first_dedupes_and_normalizes() {
    let ordered = ordered_sync_timeframes_high_first(&[
        "M1".to_string(),
        "1Day".to_string(),
        "MN1".to_string(),
        "1Min".to_string(),
        "1Day".to_string(),
        "bogus".to_string(),
    ]);
    assert_eq!(
        ordered,
        vec!["1Month".to_string(), "1Day".to_string(), "1Min".to_string(),]
    );
}

#[test]
fn sync_timeframe_period_secs_covers_standard_buckets() {
    assert_eq!(sync_timeframe_period_secs("M1"), Some(60));
    assert_eq!(sync_timeframe_period_secs("1Hour"), Some(3600));
    assert_eq!(sync_timeframe_period_secs("MN1"), Some(2_592_000));
    assert!(sync_timeframe_period_secs("bogus").is_none());
}

#[test]
fn alpaca_fetch_key_normalizes_and_strips_slashes() {
    assert_eq!(alpaca_fetch_key("BTC/USD", "H4"), "BTCUSD:4Hour");
    assert_eq!(alpaca_fetch_key("aapl", "1Day"), "AAPL:1Day");
}

#[test]
fn default_sync_timeframe_set_lists_all_nine_buckets() {
    let set = default_sync_timeframe_set();
    assert_eq!(set.len(), 9);
    assert!(set.contains("1Min"));
    assert!(set.contains("1Month"));
}

#[test]
fn select_alpaca_sync_candidates_prioritizes_missing_before_stale_or_backfill() {
    let now_s = 1_700_000_000i64;
    let symbols = vec!["AAPL".to_string(), "MSFT".to_string(), "TSLA".to_string()];
    let timeframes = vec!["1Day".to_string()];
    let mut state_map = HashMap::new();
    state_map.insert(
        ("MSFT".to_string(), "1Day".to_string()),
        SyncCacheState {
            last_bar_ts_s: now_s - 25 * 86_400,
            write_ts_s: now_s - 7 * 3600,
            bar_count: 10_000,
        },
    );
    state_map.insert(
        ("TSLA".to_string(), "1Day".to_string()),
        SyncCacheState {
            last_bar_ts_s: now_s - 60,
            write_ts_s: now_s - 60,
            bar_count: 100,
        },
    );

    let selected = select_alpaca_sync_candidates(
        &symbols,
        &timeframes,
        &state_map,
        &HashSet::new(),
        &HashSet::new(),
        &HashMap::new(),
        &HashSet::new(),
        3,
        now_s,
        alpaca_sync_target_bars,
    );

    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].symbol, "AAPL");
    assert_eq!(selected[0].bucket, AlpacaSyncBucket::Missing);
}

#[test]
fn select_alpaca_sync_candidates_prefers_focus_symbols_within_bucket() {
    let now_s = 1_700_000_000i64;
    let symbols = vec!["AAPL".to_string(), "MSFT".to_string()];
    let timeframes = vec!["1Day".to_string()];
    let focus = HashSet::from(["MSFT".to_string()]);

    let selected = select_alpaca_sync_candidates(
        &symbols,
        &timeframes,
        &HashMap::new(),
        &focus,
        &HashSet::new(),
        &HashMap::new(),
        &HashSet::new(),
        2,
        now_s,
        alpaca_sync_target_bars,
    );

    assert_eq!(selected.len(), 2);
    assert_eq!(selected[0].symbol, "MSFT");
    assert_eq!(selected[1].symbol, "AAPL");
    assert!(
        selected
            .iter()
            .all(|c| c.bucket == AlpacaSyncBucket::Missing)
    );
}

#[test]
fn select_alpaca_sync_candidates_skips_known_no_data_pairs() {
    let now_s = 1_700_000_000i64;
    let symbols = vec!["AAGIY".to_string(), "AAPL".to_string()];
    let timeframes = vec!["1Hour".to_string()];
    let no_data_keys = HashSet::from([alpaca_fetch_key("AAGIY", "1Hour")]);

    let selected = select_alpaca_sync_candidates(
        &symbols,
        &timeframes,
        &HashMap::new(),
        &HashSet::new(),
        &no_data_keys,
        &HashMap::new(),
        &HashSet::new(),
        2,
        now_s,
        alpaca_sync_target_bars,
    );

    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].symbol, "AAPL");
    assert_eq!(selected[0].timeframe, "1Hour");
}

#[test]
fn select_alpaca_sync_candidates_uses_normalized_pending_keys() {
    let now_s = 1_700_000_000i64;
    let symbols = vec!["BTC/USD".to_string(), "ETH/USD".to_string()];
    let timeframes = vec!["H4".to_string()];
    let pending = HashSet::from([alpaca_fetch_key("BTCUSD", "4Hour")]);

    let selected = select_alpaca_sync_candidates(
        &symbols,
        &timeframes,
        &HashMap::new(),
        &HashSet::new(),
        &HashSet::new(),
        &HashMap::new(),
        &pending,
        2,
        now_s,
        alpaca_sync_target_bars,
    );

    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].symbol, "ETHUSD");
    assert_eq!(selected[0].timeframe, "4Hour");
}

#[test]
fn low_timeframe_reserve_selects_low_tf_while_main_lane_stays_high_tf_first() {
    let now_s = 1_700_000_000i64;
    let symbols = vec!["AAA".to_string(), "BBB".to_string(), "CCC".to_string()];
    let timeframes = vec!["15Min".to_string(), "1Day".to_string()];
    let mut cursor = 0usize;
    let never_blocked = |_symbol: &str, _tf: &str| false;

    let main = select_alpaca_sync_workset_rotating(
        &symbols,
        &timeframes,
        &HashMap::new(),
        &HashSet::new(),
        &HashSet::new(),
        &HashMap::new(),
        &HashSet::new(),
        2,
        0,
        3,
        &mut cursor,
        now_s,
        alpaca_sync_target_bars,
        &never_blocked,
    );
    assert_eq!(main.len(), 2);
    assert!(main.iter().all(|candidate| candidate.timeframe == "1Day"));

    let mut staged_pending = HashSet::new();
    staged_pending.extend(
        main.iter()
            .map(|candidate| alpaca_fetch_key(&candidate.symbol, &candidate.timeframe)),
    );
    let reserve = select_low_timeframe_sync_reserve_rotating(
        &symbols,
        &timeframes,
        &HashMap::new(),
        &HashSet::new(),
        &HashSet::new(),
        &HashMap::new(),
        &staged_pending,
        2,
        3,
        &mut cursor,
        now_s,
        24,
        alpaca_sync_target_bars,
        &never_blocked,
    );

    assert_eq!(reserve.len(), 2);
    assert!(
        reserve
            .iter()
            .all(|candidate| candidate.timeframe == "15Min")
    );
}

#[test]
fn merge_recent_sync_overrides_preserves_settled_fetch_across_bg_rev_rebuild() {
    let now_s = 1_700_000_000i64;
    let mut rebuilt = HashMap::from([(
        ("GDC".to_string(), "4Hour".to_string()),
        SyncCacheState {
            last_bar_ts_s: now_s - 400 * 14_400,
            write_ts_s: now_s - 400 * 14_400,
            bar_count: 1_422,
        },
    )]);
    let previous = HashMap::from([(
        ("GDC".to_string(), "4Hour".to_string()),
        SyncCacheState {
            last_bar_ts_s: now_s - 30,
            write_ts_s: now_s - 30,
            bar_count: 1_422,
        },
    )]);

    merge_recent_sync_overrides(&mut rebuilt, &previous, now_s);

    let state = rebuilt
        .get(&("GDC".to_string(), "4Hour".to_string()))
        .expect("recent override should remain indexed");
    assert_eq!(state.write_ts_s, now_s - 30);
    assert_eq!(state.last_bar_ts_s, now_s - 30);
}

#[test]
fn select_alpaca_sync_candidates_backfill_marker_does_not_hide_stale_pair() {
    let now_s = 1_700_000_000i64;
    let symbols = vec!["GDC".to_string()];
    let timeframes = vec!["4Hour".to_string()];
    let state_map = HashMap::from([(
        ("GDC".to_string(), "4Hour".to_string()),
        SyncCacheState {
            last_bar_ts_s: now_s - 25 * 14_400,
            write_ts_s: now_s - 60,
            bar_count: 1_422,
        },
    )]);
    let backfill_complete = HashMap::from([(
        alpaca_fetch_key("GDC", "4Hour"),
        AlpacaBackfillCompletePair {
            symbol: "GDC".to_string(),
            timeframe: "4Hour".to_string(),
            marked_at: now_s - 30,
            bar_count: 1_422,
            target_bars: 14_000,
        },
    )]);

    let selected = select_alpaca_sync_candidates(
        &symbols,
        &timeframes,
        &state_map,
        &HashSet::new(),
        &HashSet::new(),
        &backfill_complete,
        &HashSet::new(),
        1,
        now_s,
        alpaca_sync_target_bars,
    );

    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].symbol, "GDC");
    assert_eq!(selected[0].bucket, AlpacaSyncBucket::Stale);
}

#[test]
fn select_alpaca_sync_candidates_prioritizes_higher_timeframes_across_symbols() {
    let now_s = 1_700_000_000i64;
    let symbols = vec!["AAPL".to_string(), "MSFT".to_string()];
    let timeframes = vec!["1Min".to_string(), "1Day".to_string(), "1Month".to_string()];

    let selected = select_alpaca_sync_candidates(
        &symbols,
        &timeframes,
        &HashMap::new(),
        &HashSet::new(),
        &HashSet::new(),
        &HashMap::new(),
        &HashSet::new(),
        4,
        now_s,
        alpaca_sync_target_bars,
    );

    assert_eq!(selected.len(), 4);
    assert!(
        selected
            .iter()
            .all(|c| c.bucket == AlpacaSyncBucket::Missing)
    );
    assert_eq!(selected[0].timeframe, "1Month");
    assert_eq!(selected[1].timeframe, "1Month");
    assert_eq!(selected[2].timeframe, "1Day");
    assert_eq!(selected[3].timeframe, "1Day");
}

#[test]
fn select_alpaca_sync_workset_prioritizes_missing_before_focus_refresh() {
    let now_s = 1_700_000_000i64;
    let symbols = vec!["AAPL".to_string(), "MSFT".to_string()];
    let timeframes = vec!["1Day".to_string()];
    let focus = HashSet::from(["MSFT".to_string()]);
    let state_map = HashMap::from([(
        ("MSFT".to_string(), "1Day".to_string()),
        SyncCacheState {
            last_bar_ts_s: now_s - 25 * 86_400,
            write_ts_s: now_s - 7 * 3600,
            bar_count: 10_000,
        },
    )]);

    let selected = select_alpaca_sync_workset(
        &symbols,
        &timeframes,
        &state_map,
        &focus,
        &HashSet::new(),
        &HashMap::new(),
        &HashSet::new(),
        2,
        1,
        now_s,
        alpaca_sync_target_bars,
    );

    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].symbol, "AAPL");
    assert_eq!(selected[0].bucket, AlpacaSyncBucket::Missing);
}

#[test]
fn select_alpaca_sync_workset_dedupes_focus_candidates_from_background() {
    let now_s = 1_700_000_000i64;
    let symbols = vec!["MSFT".to_string()];
    let timeframes = vec!["1Day".to_string(), "1Hour".to_string()];
    let focus = HashSet::from(["MSFT".to_string()]);

    let selected = select_alpaca_sync_workset(
        &symbols,
        &timeframes,
        &HashMap::new(),
        &focus,
        &HashSet::new(),
        &HashMap::new(),
        &HashSet::new(),
        2,
        1,
        now_s,
        alpaca_sync_target_bars,
    );

    assert_eq!(selected.len(), 2);
    assert_eq!(selected[0].symbol, "MSFT");
    assert_eq!(selected[1].symbol, "MSFT");
    assert_ne!(selected[0].timeframe, selected[1].timeframe);
}

#[test]
fn select_alpaca_sync_workset_rotating_bounds_background_scan() {
    let now_s = 1_700_000_000i64;
    let symbols = vec![
        "AAPL".to_string(),
        "MSFT".to_string(),
        "QQQ".to_string(),
        "TSLA".to_string(),
    ];
    let timeframes = vec!["1Day".to_string()];
    let mut cursor = 0usize;

    let first = select_alpaca_sync_workset_rotating(
        &symbols,
        &timeframes,
        &HashMap::new(),
        &HashSet::new(),
        &HashSet::new(),
        &HashMap::new(),
        &HashSet::new(),
        1,
        0,
        2,
        &mut cursor,
        now_s,
        alpaca_sync_target_bars,
        &|_, _| false,
    );

    assert_eq!(first.len(), 1);
    assert_eq!(first[0].symbol, "AAPL");
    assert_eq!(cursor, 2);

    let second = select_alpaca_sync_workset_rotating(
        &symbols,
        &timeframes,
        &HashMap::new(),
        &HashSet::new(),
        &HashSet::new(),
        &HashMap::new(),
        &HashSet::new(),
        1,
        0,
        2,
        &mut cursor,
        now_s,
        alpaca_sync_target_bars,
        &|_, _| false,
    );

    assert_eq!(second.len(), 1);
    assert_eq!(second[0].symbol, "QQQ");
    assert_eq!(cursor, 0);
}

#[test]
fn select_alpaca_sync_workset_rotating_prioritizes_focus_before_background_scan() {
    let now_s = 1_700_000_000i64;
    let symbols = vec!["AAPL".to_string(), "MSFT".to_string(), "QQQ".to_string()];
    let timeframes = vec!["1Day".to_string()];
    let focus = HashSet::from(["QQQ".to_string()]);
    let mut cursor = 0usize;

    let selected = select_alpaca_sync_workset_rotating(
        &symbols,
        &timeframes,
        &HashMap::new(),
        &focus,
        &HashSet::new(),
        &HashMap::new(),
        &HashSet::new(),
        1,
        1,
        1,
        &mut cursor,
        now_s,
        alpaca_sync_target_bars,
        &|_, _| false,
    );

    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].symbol, "QQQ");
    assert_eq!(selected[0].bucket, AlpacaSyncBucket::Missing);
    assert_eq!(cursor, 1);
}

#[test]
fn select_alpaca_sync_workset_rotating_walks_all_symbols_mn1_before_lower_timeframes() {
    let now_s = 1_700_000_000i64;
    let symbols = vec!["AAPL".to_string(), "MSFT".to_string(), "QQQ".to_string()];
    let timeframes = vec![
        "1Min".to_string(),
        "1Week".to_string(),
        "1Month".to_string(),
    ];
    let mut cursor = 0usize;

    let first = select_alpaca_sync_workset_rotating(
        &symbols,
        &timeframes,
        &HashMap::new(),
        &HashSet::new(),
        &HashSet::new(),
        &HashMap::new(),
        &HashSet::new(),
        2,
        0,
        2,
        &mut cursor,
        now_s,
        alpaca_sync_target_bars,
        &|_, _| false,
    );
    assert_eq!(
        first
            .iter()
            .map(|c| (&c.symbol, &c.timeframe))
            .collect::<Vec<_>>(),
        vec![
            (&"AAPL".to_string(), &"1Month".to_string()),
            (&"MSFT".to_string(), &"1Month".to_string()),
        ]
    );
    assert_eq!(cursor, 2);

    let pending: HashSet<String> = first
        .iter()
        .map(|c| alpaca_fetch_key(&c.symbol, &c.timeframe))
        .collect();
    let second = select_alpaca_sync_workset_rotating(
        &symbols,
        &timeframes,
        &HashMap::new(),
        &HashSet::new(),
        &HashSet::new(),
        &HashMap::new(),
        &pending,
        2,
        0,
        2,
        &mut cursor,
        now_s,
        alpaca_sync_target_bars,
        &|_, _| false,
    );
    assert_eq!(
        second
            .iter()
            .map(|c| (&c.symbol, &c.timeframe))
            .collect::<Vec<_>>(),
        vec![(&"QQQ".to_string(), &"1Month".to_string()),]
    );
    assert_eq!(cursor, 1);

    let pending: HashSet<String> = pending
        .into_iter()
        .chain(
            second
                .iter()
                .map(|c| alpaca_fetch_key(&c.symbol, &c.timeframe)),
        )
        .collect();
    let third = select_alpaca_sync_workset_rotating(
        &symbols,
        &timeframes,
        &HashMap::new(),
        &HashSet::new(),
        &HashSet::new(),
        &HashMap::new(),
        &pending,
        2,
        0,
        2,
        &mut cursor,
        now_s,
        alpaca_sync_target_bars,
        &|_, _| false,
    );
    assert_eq!(
        third
            .iter()
            .map(|c| (&c.symbol, &c.timeframe))
            .collect::<Vec<_>>(),
        vec![
            (&"MSFT".to_string(), &"1Week".to_string()),
            (&"QQQ".to_string(), &"1Week".to_string()),
        ]
    );
    assert_eq!(cursor, 0);
}

#[test]
fn select_alpaca_sync_workset_rotating_advances_cursor_by_actual_tail_window() {
    let now_s = 1_700_000_000i64;
    let symbols = vec!["AAPL", "MSFT", "QQQ", "SPY", "TSLA"]
        .into_iter()
        .map(String::from)
        .collect::<Vec<_>>();
    let timeframes = vec!["1Month".to_string()];
    let mut state = HashMap::new();
    for symbol in ["AAPL", "MSFT", "QQQ", "SPY"] {
        state.insert(
            (symbol.to_string(), "1Month".to_string()),
            SyncCacheState {
                last_bar_ts_s: now_s,
                write_ts_s: now_s,
                bar_count: i64::from(u32::MAX),
            },
        );
    }
    let mut cursor = 0usize;

    let selected = select_alpaca_sync_workset_rotating(
        &symbols,
        &timeframes,
        &state,
        &HashSet::new(),
        &HashSet::new(),
        &HashMap::new(),
        &HashSet::new(),
        1,
        0,
        4,
        &mut cursor,
        now_s,
        alpaca_sync_target_bars,
        &|_, _| false,
    );

    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].symbol, "TSLA");
    assert_eq!(selected[0].timeframe, "1Month");
    assert_eq!(
        cursor, 0,
        "tail windows smaller than scan_limit should not advance the cursor past already-scanned symbols"
    );
}

#[test]
fn select_alpaca_sync_workset_rotating_keeps_global_high_tf_priority_across_slices() {
    let now_s = 1_700_000_000i64;
    let symbols = vec![
        "AAPL".to_string(),
        "MSFT".to_string(),
        "QQQ".to_string(),
        "TSLA".to_string(),
    ];
    let timeframes = vec!["1Month".to_string(), "1Week".to_string()];
    let mut state = HashMap::new();
    for symbol in ["AAPL", "MSFT"] {
        state.insert(
            (symbol.to_string(), "1Month".to_string()),
            SyncCacheState {
                last_bar_ts_s: now_s,
                write_ts_s: now_s,
                bar_count: i64::from(u32::MAX),
            },
        );
    }
    let mut cursor = 0usize;

    let selected = select_alpaca_sync_workset_rotating(
        &symbols,
        &timeframes,
        &state,
        &HashSet::new(),
        &HashSet::new(),
        &HashMap::new(),
        &HashSet::new(),
        2,
        0,
        2,
        &mut cursor,
        now_s,
        alpaca_sync_target_bars,
        &|_, _| false,
    );

    assert_eq!(
        selected
            .iter()
            .map(|c| (&c.symbol, &c.timeframe))
            .collect::<Vec<_>>(),
        vec![
            (&"QQQ".to_string(), &"1Month".to_string()),
            (&"TSLA".to_string(), &"1Month".to_string()),
        ],
        "later missing 1Month symbols must be scheduled before lower-TF work in an already-complete cursor slice"
    );
    assert_eq!(cursor, 0);
}

#[test]
fn select_alpaca_sync_workset_rotating_gives_focus_the_full_batch() {
    let now_s = 1_700_000_000i64;
    let symbols = vec!["AAPL".to_string(), "MSFT".to_string(), "QQQ".to_string()];
    let timeframes = vec!["15Min".to_string()];
    let focus = HashSet::from(["MSFT".to_string(), "QQQ".to_string()]);
    let mut cursor = 0usize;

    let selected = select_alpaca_sync_workset_rotating(
        &symbols,
        &timeframes,
        &HashMap::new(),
        &focus,
        &HashSet::new(),
        &HashMap::new(),
        &HashSet::new(),
        3,
        1,
        3,
        &mut cursor,
        now_s,
        alpaca_sync_target_bars,
        &|_, _| false,
    );

    assert_eq!(selected.len(), 3);
    assert_eq!(selected[0].symbol, "MSFT");
    assert_eq!(selected[1].symbol, "QQQ");
    assert!(selected[0].focus);
    assert!(selected[1].focus);
}

#[test]
fn select_alpaca_sync_workset_rotating_does_not_foreground_m1_m5() {
    let now_s = 1_700_000_000i64;
    let symbols = vec!["AAPL".to_string(), "MSFT".to_string(), "QQQ".to_string()];
    let timeframes = vec!["1Min".to_string(), "5Min".to_string()];
    let focus = HashSet::from(["QQQ".to_string()]);
    let mut cursor = 0usize;

    let selected = select_alpaca_sync_workset_rotating(
        &symbols,
        &timeframes,
        &HashMap::new(),
        &focus,
        &HashSet::new(),
        &HashMap::new(),
        &HashSet::new(),
        1,
        1,
        1,
        &mut cursor,
        now_s,
        alpaca_sync_target_bars,
        &|_, _| false,
    );

    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].symbol, "AAPL");
    assert!(!selected[0].focus);
    assert_eq!(cursor, 1);
}

#[test]
fn foreground_stale_refresh_is_due_after_one_timeframe_period() {
    let now_s = 1_700_000_000i64;
    let symbols = vec!["AAPL".to_string(), "MSFT".to_string()];
    let timeframes = vec!["1Min".to_string()];
    let state_map = HashMap::from([
        (
            ("AAPL".to_string(), "1Min".to_string()),
            SyncCacheState {
                last_bar_ts_s: now_s - 65,
                write_ts_s: now_s - 60,
                bar_count: 50_000,
            },
        ),
        (
            ("MSFT".to_string(), "1Min".to_string()),
            SyncCacheState {
                last_bar_ts_s: now_s - 65,
                write_ts_s: now_s - 60,
                bar_count: 50_000,
            },
        ),
    ]);
    let focus = HashSet::from(["AAPL".to_string()]);

    let selected = select_alpaca_sync_candidates(
        &symbols,
        &timeframes,
        &state_map,
        &focus,
        &HashSet::new(),
        &HashMap::new(),
        &HashSet::new(),
        2,
        now_s,
        |_| None,
    );

    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].symbol, "AAPL");
    assert_eq!(selected[0].bucket, AlpacaSyncBucket::Stale);
    assert!(selected[0].focus);
}

#[test]
fn select_alpaca_sync_workset_rotating_skips_pending_without_advancing_priority() {
    let now_s = 1_700_000_000i64;
    let symbols = vec!["AAPL".to_string(), "MSFT".to_string(), "QQQ".to_string()];
    let timeframes = vec!["1Min".to_string(), "1Month".to_string()];
    let pending = HashSet::from([alpaca_fetch_key("AAPL", "1Month")]);
    let mut cursor = 0usize;

    let selected = select_alpaca_sync_workset_rotating(
        &symbols,
        &timeframes,
        &HashMap::new(),
        &HashSet::new(),
        &HashSet::new(),
        &HashMap::new(),
        &pending,
        2,
        0,
        3,
        &mut cursor,
        now_s,
        alpaca_sync_target_bars,
        &|_, _| false,
    );

    assert_eq!(
        selected
            .iter()
            .map(|c| (&c.symbol, &c.timeframe))
            .collect::<Vec<_>>(),
        vec![
            (&"MSFT".to_string(), &"1Month".to_string()),
            (&"QQQ".to_string(), &"1Month".to_string()),
        ]
    );
    assert_eq!(cursor, 0);
}

#[test]
fn high_timeframe_backfill_preempts_lower_timeframe_stale_refresh() {
    let now_s = 1_700_000_000i64;
    let symbols = vec!["AAPL".to_string()];
    let timeframes = vec!["1Hour".to_string(), "1Month".to_string()];
    let state_map = HashMap::from([
        (
            ("AAPL".to_string(), "1Month".to_string()),
            SyncCacheState {
                last_bar_ts_s: now_s - 60,
                write_ts_s: now_s - 60,
                bar_count: 10,
            },
        ),
        (
            ("AAPL".to_string(), "1Hour".to_string()),
            SyncCacheState {
                last_bar_ts_s: now_s - 25 * 3600,
                write_ts_s: now_s - 3600,
                bar_count: 50_000,
            },
        ),
    ]);
    let mut cursor = 0usize;

    let selected = select_alpaca_sync_workset_rotating(
        &symbols,
        &timeframes,
        &state_map,
        &HashSet::new(),
        &HashSet::new(),
        &HashMap::new(),
        &HashSet::new(),
        1,
        0,
        1,
        &mut cursor,
        now_s,
        |_| Some(100),
        &|_, _| false,
    );

    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].timeframe, "1Month");
    assert_eq!(selected[0].bucket, AlpacaSyncBucket::Backfill);
}

#[test]
fn stale_window_uses_timeframe_last_bar_age_not_write_age() {
    let now_s = 1_700_000_000i64;
    let symbols = vec!["AAPL".to_string()];
    let timeframes = vec!["1Min".to_string()];
    let state_map = HashMap::from([(
        ("AAPL".to_string(), "1Min".to_string()),
        SyncCacheState {
            last_bar_ts_s: now_s - 25 * 60,
            write_ts_s: now_s - 60,
            bar_count: 50_000,
        },
    )]);

    let selected = select_alpaca_sync_candidates(
        &symbols,
        &timeframes,
        &state_map,
        &HashSet::new(),
        &HashSet::new(),
        &HashMap::new(),
        &HashSet::new(),
        1,
        now_s,
        alpaca_sync_target_bars,
    );

    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].bucket, AlpacaSyncBucket::Stale);
}

#[test]
fn select_alpaca_sync_candidates_skips_backfill_complete_pairs() {
    let now_s = 1_700_000_000i64;
    let symbols = vec!["LUMN".to_string()];
    let timeframes = vec!["1Month".to_string()];
    let state_map = HashMap::from([(
        ("LUMN".to_string(), "1Month".to_string()),
        SyncCacheState {
            last_bar_ts_s: now_s - 60,
            write_ts_s: now_s - 60,
            bar_count: 70,
        },
    )]);
    let backfill_complete = HashMap::from([(
        alpaca_fetch_key("LUMN", "1Month"),
        AlpacaBackfillCompletePair {
            symbol: "LUMN".to_string(),
            timeframe: "1Month".to_string(),
            marked_at: now_s,
            bar_count: 70,
            target_bars: u32::MAX as i64,
        },
    )]);

    let selected = select_alpaca_sync_candidates(
        &symbols,
        &timeframes,
        &state_map,
        &HashSet::new(),
        &HashSet::new(),
        &backfill_complete,
        &HashSet::new(),
        1,
        now_s,
        alpaca_sync_target_bars,
    );

    assert!(selected.is_empty());
}

#[test]
fn rotating_selector_descends_past_timeframe_whose_candidates_are_dispatch_blocked() {
    // The overnight wedge: every (symbol, 1Month) classifies Backfill forever
    // (unbounded target, no completion marks) but sits on a multi-day fetch
    // cooldown. Without the dispatch-blocked probe the selector spends every
    // batch on those undispatchable 1Month candidates and 1Day never syncs.
    let now_s = 1_700_000_000i64;
    let symbols = vec!["AAPL".to_string(), "MSFT".to_string()];
    let timeframes = vec!["1Month".to_string(), "1Day".to_string()];
    let mut state_map = HashMap::new();
    for symbol in &symbols {
        state_map.insert(
            (symbol.clone(), "1Month".to_string()),
            SyncCacheState {
                last_bar_ts_s: now_s - 3_600,
                write_ts_s: now_s - 3_600,
                bar_count: 120,
            },
        );
    }

    let mut cursor = 0usize;
    let wedged = select_alpaca_sync_workset_rotating(
        &symbols,
        &timeframes,
        &state_map,
        &HashSet::new(),
        &HashSet::new(),
        &HashMap::new(),
        &HashSet::new(),
        2,
        0,
        2,
        &mut cursor,
        now_s,
        alpaca_sync_target_bars,
        &|_, _| false,
    );
    assert!(
        wedged.iter().all(|c| c.timeframe == "1Month"),
        "without blocking, the 1Month Backfill bucket owns the batch"
    );

    let mut cursor = 0usize;
    let descended = select_alpaca_sync_workset_rotating(
        &symbols,
        &timeframes,
        &state_map,
        &HashSet::new(),
        &HashSet::new(),
        &HashMap::new(),
        &HashSet::new(),
        2,
        0,
        2,
        &mut cursor,
        now_s,
        alpaca_sync_target_bars,
        &|_, tf| tf == "1Month",
    );
    assert_eq!(descended.len(), 2);
    assert!(
        descended.iter().all(|c| c.timeframe == "1Day"),
        "blocked 1Month candidates must not hold the TF descent: {descended:?}"
    );
    assert!(
        descended
            .iter()
            .all(|c| c.bucket == AlpacaSyncBucket::Missing)
    );
}

#[test]
fn rotating_selector_blocked_candidates_do_not_consume_batch_slots() {
    let now_s = 1_700_000_000i64;
    let symbols = vec!["AAPL".to_string(), "MSFT".to_string()];
    let timeframes = vec!["1Day".to_string()];
    let mut cursor = 0usize;

    let selected = select_alpaca_sync_workset_rotating(
        &symbols,
        &timeframes,
        &HashMap::new(),
        &HashSet::new(),
        &HashSet::new(),
        &HashMap::new(),
        &HashSet::new(),
        1,
        0,
        2,
        &mut cursor,
        now_s,
        alpaca_sync_target_bars,
        &|symbol, _| symbol == "AAPL",
    );

    assert_eq!(selected.len(), 1);
    assert_eq!(
        selected[0].symbol, "MSFT",
        "the blocked candidate must not eat the only batch slot"
    );
}
