use super::*;

#[test]
fn format_ws_symbol_prefers_display_when_already_slashed() {
    assert_eq!(
        format_ws_symbol("XXBTZUSD", "XBT/USD"),
        Some("XBT/USD".into())
    );
    assert_eq!(
        format_ws_symbol("ETHUSD", "eth/usd"),
        Some("ETH/USD".into())
    );
}

#[test]
fn format_ws_symbol_inserts_slash_from_legacy_pair_name() {
    // Display is the flat form; we must reconstruct base/quote.
    assert_eq!(
        format_ws_symbol("XXBTZUSD", "XBTUSD"),
        Some("BTC/USD".into())
    );
    assert_eq!(format_ws_symbol("ETHUSD", "ETHUSD"), Some("ETH/USD".into()));
    assert_eq!(format_ws_symbol("SOLUSD", "SOLUSD"), Some("SOL/USD".into()));
}

#[test]
fn format_ws_symbol_handles_stablecoin_quotes() {
    assert_eq!(
        format_ws_symbol("USDCUSD", "USDCUSD"),
        Some("USDC/USD".into())
    );
    assert_eq!(
        format_ws_symbol("BTCUSDT", "BTCUSDT"),
        Some("BTC/USDT".into())
    );
    // USDG (Paxos) — the longest stablecoin suffix.
    assert_eq!(
        format_ws_symbol("BTCUSDG", "BTCUSDG"),
        Some("BTC/USDG".into())
    );
}

#[test]
fn format_ws_symbol_returns_none_for_unrecognised_quote() {
    // No known quote suffix at all → can't construct a /-form.
    assert!(format_ws_symbol("XYZABC", "XYZABC").is_none());
}

#[test]
fn build_kraken_ws_subscribe_symbols_dedupes_and_sorts() {
    let pairs = vec![
        ("XXBTZUSD".to_string(), "XBT/USD".to_string()),
        ("ETHUSD".to_string(), "ETHUSD".to_string()),
        ("XXBTZUSD".to_string(), "XBTUSD".to_string()), // dup of first via fallback
        ("SOLUSD".to_string(), "SOLUSD".to_string()),
    ];
    let symbols = build_kraken_ws_subscribe_symbols(&pairs);
    // BTreeSet dedupes XBT/USD vs BTC/USD as distinct strings — both
    // get included. That's fine: Kraken accepts either form on
    // subscribe. Order is stable / sorted.
    assert!(symbols.contains(&"ETH/USD".to_string()));
    assert!(symbols.contains(&"SOL/USD".to_string()));
    let mut sorted = symbols.clone();
    sorted.sort();
    assert_eq!(symbols, sorted, "BTreeSet output should already be sorted");
}

#[test]
fn build_kraken_ws_subscribe_symbols_can_include_xstocks() {
    let pairs = vec![("XXBTZUSD".to_string(), "XBT/USD".to_string())];
    let xstocks = vec!["aapl".to_string(), "TSLA.EQ".to_string()];
    let symbols = build_kraken_ws_subscribe_symbols_for_app(&pairs, &xstocks, &xstocks, true);
    assert!(symbols.contains(&"AAPLx/USD".to_string()));
    assert!(symbols.contains(&"TSLAx/USD".to_string()));
    assert!(symbols.contains(&"XBT/USD".to_string()));
}

#[test]
fn build_kraken_ws_subscribe_symbols_streams_demand_xstocks_not_full_catalog() {
    let pairs = vec![("XXBTZUSD".to_string(), "XBT/USD".to_string())];
    let catalog_xstocks = vec!["AAPL".to_string(), "MSFT".to_string(), "WOK.EQ".to_string()];
    let demand_xstocks = vec!["wok".to_string()];

    let symbols =
        build_kraken_ws_subscribe_symbols_for_app(&pairs, &catalog_xstocks, &demand_xstocks, true);

    // Only the demand xStock (WOK) and spot pairs stream live. The broad
    // catalog (AAPL/MSFT) is intentionally excluded: streaming ~12k symbols
    // across 8 intervals is what caused the WS reset churn and UI stalls.
    assert!(symbols.contains(&"WOKx/USD".to_string()));
    assert!(symbols.contains(&"XBT/USD".to_string()));
    assert!(!symbols.contains(&"AAPLx/USD".to_string()));
    assert!(!symbols.contains(&"MSFTx/USD".to_string()));
}

#[test]
fn ws_bar_cache_target_routes_tokenized_stocks_to_equity_cache() {
    assert_eq!(
        kraken_ws_bar_cache_target("AAPLx/USD"),
        Some(("kraken-equities", "AAPL".to_string()))
    );
    assert_eq!(
        kraken_ws_bar_cache_target("BTC/USD"),
        Some(("kraken", "BTCUSD".to_string()))
    );
}

#[test]
fn build_kraken_ws_subscribe_symbols_filters_out_unmappable_pairs() {
    let pairs = vec![
        ("XXBTZUSD".to_string(), "XBT/USD".to_string()),
        ("GARBAGE".to_string(), "GARBAGE".to_string()),
    ];
    let symbols = build_kraken_ws_subscribe_symbols(&pairs);
    assert!(symbols.iter().all(|s| s.contains('/')));
}

#[test]
fn large_universe_disables_initial_ws_snapshots() {
    assert!(kraken_ws_should_request_initial_snapshot(
        WS_LARGE_UNIVERSE_PAIR_THRESHOLD - 1
    ));
    assert!(!kraken_ws_should_request_initial_snapshot(
        WS_LARGE_UNIVERSE_PAIR_THRESHOLD
    ));
    assert!(!kraken_ws_should_request_initial_snapshot(12_741));
}

#[test]
fn small_universe_plan_snapshots_every_interval_immediately() {
    let plan = plan_kraken_ws_streamers(
        WS_LARGE_UNIVERSE_PAIR_THRESHOLD - 1,
        KRAKEN_WS_OHLC_INTERVALS_MIN,
    );
    assert_eq!(plan.len(), KRAKEN_WS_OHLC_INTERVALS_MIN.len());
    assert!(
        plan.iter()
            .all(|&(_, snapshot, delay)| snapshot && delay.is_zero()),
        "small universe must snapshot every interval with no stagger: {plan:?}"
    );
    // Every served interval is represented exactly once.
    for &interval_min in KRAKEN_WS_OHLC_INTERVALS_MIN {
        assert_eq!(
            plan.iter().filter(|&&(m, _, _)| m == interval_min).count(),
            1
        );
    }
}

#[test]
fn large_universe_plan_snapshots_only_bounded_high_timeframes() {
    let plan = plan_kraken_ws_streamers(12_714, KRAKEN_WS_OHLC_INTERVALS_MIN);
    assert_eq!(plan.len(), KRAKEN_WS_OHLC_INTERVALS_MIN.len());
    for &(interval_min, snapshot, delay) in &plan {
        if interval_min >= 60 {
            assert!(snapshot, "high TF {interval_min} must snapshot");
        } else {
            assert!(!snapshot, "low TF {interval_min} must be live-only");
            assert!(delay.is_zero(), "live-only TF {interval_min} starts now");
        }
    }
    // Snapshot waves are ordered highest-TF-first and staggered so each
    // drains before the next: 1Week@0, 1Day@1×, 4Hour@2×, 1Hour@3×.
    let wave = |interval_min: u32| {
        plan.iter()
            .find(|&&(m, _, _)| m == interval_min)
            .map(|&(_, _, delay)| delay)
            .expect("interval present")
    };
    assert_eq!(wave(10080), Duration::ZERO);
    assert_eq!(wave(1440), WS_LARGE_UNIVERSE_INTERVAL_STAGGER);
    assert_eq!(wave(240), WS_LARGE_UNIVERSE_INTERVAL_STAGGER * 2);
    assert_eq!(wave(60), WS_LARGE_UNIVERSE_INTERVAL_STAGGER * 3);
}

#[test]
fn ws_ohlc_interval_plan_respects_enabled_sync_timeframe_controls() {
    let enabled = BTreeSet::from(["15Min".to_string(), "1Hour".to_string(), "1Day".to_string()]);

    assert_eq!(
        enabled_kraken_ws_ohlc_intervals(&enabled),
        vec![15, 60, 1440]
    );

    let plan = plan_kraken_ws_streamers(12_714, &enabled_kraken_ws_ohlc_intervals(&enabled));
    let intervals: Vec<u32> = plan
        .iter()
        .map(|(interval_min, _, _)| *interval_min)
        .collect();

    assert_eq!(intervals, vec![15, 1440, 60]);
    assert!(
        plan.iter()
            .all(|(interval_min, _, _)| *interval_min != 1 && *interval_min != 5)
    );
}

#[test]
fn snapshot_sweep_respects_enabled_sync_timeframe_controls() {
    // Disabled TFs are excluded from the interval list, so the highest ENABLED
    // interval with missing pairs is chosen.
    let enabled = BTreeSet::from(["1Day".to_string(), "15Min".to_string()]);
    let intervals = enabled_kraken_ws_ohlc_snapshot_sweep_intervals(&enabled);
    assert_eq!(intervals, vec![1440, 15]);
    let catalog = vec!["AAPL".to_string()];
    let fresh = std::collections::HashMap::new();
    let (interval_min, _pairs) = select_kraken_ws_snapshot_sweep_batch_high_first(
        &catalog, &intervals, &fresh, 0, 250,
    )
    .expect("highest enabled interval");
    assert_eq!(interval_min, 1440, "1Day chosen over 15Min");
}

#[test]
fn snapshot_sweep_picks_highest_timeframe_with_missing_pairs() {
    // Nothing fresh yet → every interval has missing pairs → the HIGHEST interval
    // wins (high-TF-first coverage), symbols normalized + sorted.
    let catalog = vec!["aapl".to_string(), "MSFT.EQ".to_string()];
    let fresh = std::collections::HashMap::new();
    let (interval_min, pairs) = select_kraken_ws_snapshot_sweep_batch_high_first(
        &catalog,
        &KRAKEN_WS_SNAPSHOT_SWEEP_INTERVALS_HIGH_FIRST,
        &fresh,
        10_000_000_000,
        250,
    )
    .expect("a batch when pairs are missing");
    assert_eq!(interval_min, 10080, "1Week (highest) swept first");
    assert_eq!(
        pairs,
        vec!["AAPLx/USD".to_string(), "MSFTx/USD".to_string()]
    );
}

#[test]
fn snapshot_sweep_skips_fresh_high_timeframes_to_the_lowest_gap() {
    // AAPL is WS-fresh for every interval except 1Min → the selector skips the
    // fresh high TFs and sweeps the one remaining gap instead of re-pulling
    // already-fresh high-TF bars.
    let now_ms = 10_000_000_000i64;
    let mut fresh = std::collections::HashMap::new();
    for &interval_min in &KRAKEN_WS_SNAPSHOT_SWEEP_INTERVALS_HIGH_FIRST {
        let tf = kraken_ws_interval_to_tf_label(interval_min).unwrap();
        if tf != "1Min" {
            fresh.insert(("AAPL".to_string(), tf.to_string()), now_ms);
        }
    }
    let catalog = vec!["AAPL".to_string()];
    let (interval_min, pairs) = select_kraken_ws_snapshot_sweep_batch_high_first(
        &catalog,
        &KRAKEN_WS_SNAPSHOT_SWEEP_INTERVALS_HIGH_FIRST,
        &fresh,
        now_ms,
        250,
    )
    .expect("the 1Min gap is swept");
    assert_eq!(interval_min, 1, "only 1Min still missing");
    assert_eq!(pairs, vec!["AAPLx/USD".to_string()]);
}

#[test]
fn snapshot_sweep_none_when_every_interval_is_fresh() {
    let now_ms = 10_000_000_000i64;
    let mut fresh = std::collections::HashMap::new();
    for &interval_min in &KRAKEN_WS_SNAPSHOT_SWEEP_INTERVALS_HIGH_FIRST {
        let tf = kraken_ws_interval_to_tf_label(interval_min).unwrap();
        fresh.insert(("AAPL".to_string(), tf.to_string()), now_ms);
    }
    let catalog = vec!["AAPL".to_string()];
    assert!(
        select_kraken_ws_snapshot_sweep_batch_high_first(
            &catalog,
            &KRAKEN_WS_SNAPSHOT_SWEEP_INTERVALS_HIGH_FIRST,
            &fresh,
            now_ms,
            250,
        )
        .is_none(),
        "fully-fresh catalog → no sweep"
    );
}

#[test]
fn snapshot_sweep_caps_batch_size_within_the_chosen_timeframe() {
    let catalog = vec!["aapl".to_string(), "MSFT".to_string(), "WOK".to_string()];
    let fresh = std::collections::HashMap::new();
    let (interval_min, pairs) = select_kraken_ws_snapshot_sweep_batch_high_first(
        &catalog,
        &KRAKEN_WS_SNAPSHOT_SWEEP_INTERVALS_HIGH_FIRST,
        &fresh,
        0,
        2,
    )
    .expect("a capped batch");
    assert_eq!(interval_min, 10080);
    assert_eq!(
        pairs,
        vec!["AAPLx/USD".to_string(), "MSFTx/USD".to_string()],
        "batch capped at 2, sorted"
    );
}

fn mk_bar(interval_min: u32, interval_begin_ms: i64) -> KrakenWsOhlcBar {
    KrakenWsOhlcBar {
        symbol: "BTC/USD".into(),
        interval_min,
        interval_begin_ms,
        open: 100.0,
        high: 101.0,
        low: 99.0,
        close: 100.5,
        volume: 1.0,
        vwap: None,
        trades: 1,
        is_snapshot: false,
    }
}

#[test]
fn partition_closed_bars_keeps_open_bars_in_remaining() {
    let now_ms = 1_700_000_300_000;
    let mut buffer = HashMap::new();
    // 1Min bar whose bucket runs to now+30s → still open.
    buffer.insert(
        ("kraken".into(), "BTCUSD".into(), 1, now_ms - 30_000),
        mk_bar(1, now_ms - 30_000),
    );
    let (to_flush, remaining) = partition_closed_bars(buffer, now_ms);
    assert!(to_flush.is_empty(), "open bar must not be flushed");
    assert_eq!(
        remaining.len(),
        1,
        "open bar must stay buffered for next tick"
    );
}

#[test]
fn partition_closed_bars_flushes_bars_past_bucket_end() {
    let now_ms = 1_700_000_300_000;
    let mut buffer = HashMap::new();
    // 1Min bar whose bucket ended one minute ago → closed.
    let begin = now_ms - 120_000;
    buffer.insert(
        ("kraken".into(), "BTCUSD".into(), 1, begin),
        mk_bar(1, begin),
    );
    let (to_flush, remaining) = partition_closed_bars(buffer, now_ms);
    assert_eq!(to_flush.len(), 1, "closed bar must flush");
    assert!(remaining.is_empty(), "closed bar must leave the buffer");
}

#[test]
fn partition_closed_bars_mixed_buffer_splits_by_close_state() {
    let now_ms = 1_700_000_300_000;
    let mut buffer = HashMap::new();
    // Closed 1Min from two minutes ago.
    let closed_begin = now_ms - 120_000;
    buffer.insert(
        ("kraken".into(), "BTCUSD".into(), 1, closed_begin),
        mk_bar(1, closed_begin),
    );
    // Open 5Min: bucket runs to now + 4 minutes.
    let open_begin = now_ms - 60_000;
    buffer.insert(
        ("kraken".into(), "ETHUSD".into(), 5, open_begin),
        mk_bar(5, open_begin),
    );
    // Open 1Day: bucket started 12 hours ago, 12 hours remaining.
    let open_day_begin = now_ms - 12 * 3_600_000;
    buffer.insert(
        ("kraken".into(), "SOLUSD".into(), 1440, open_day_begin),
        mk_bar(1440, open_day_begin),
    );
    let (to_flush, remaining) = partition_closed_bars(buffer, now_ms);
    assert_eq!(to_flush.len(), 1, "only the closed 1Min should flush");
    assert!(to_flush.contains_key(&("kraken".to_string(), "BTCUSD".to_string(), 1, closed_begin)));
    assert_eq!(remaining.len(), 2, "both open bars stay buffered");
    assert!(remaining.contains_key(&("kraken".to_string(), "ETHUSD".to_string(), 5, open_begin)));
    assert!(remaining.contains_key(&(
        "kraken".to_string(),
        "SOLUSD".to_string(),
        1440,
        open_day_begin
    )));
}

#[test]
fn partition_closed_bars_empty_buffer_returns_two_empty_maps() {
    let (to_flush, remaining) = partition_closed_bars(HashMap::new(), 1_700_000_000_000);
    assert!(to_flush.is_empty());
    assert!(remaining.is_empty());
}

#[test]
fn partition_closed_bars_snapshot_historical_bars_all_flush() {
    // Snapshot delivery from Kraken brings closed historical bars (the last
    // ~720 bars of the chosen interval). Every one should flush on the first
    // tick so the WS-fresh anchor is established and REST can stop scheduling
    // these (symbol, tf) pairs immediately.
    let now_ms = 1_700_000_000_000;
    let mut buffer = HashMap::new();
    for i in 1..=10 {
        // Ten consecutive closed 1Min bars ending 1..=10 minutes ago.
        let begin = now_ms - (i as i64) * 60_000;
        buffer.insert(
            ("kraken".into(), "BTCUSD".into(), 1, begin),
            mk_bar(1, begin),
        );
    }
    let (to_flush, remaining) = partition_closed_bars(buffer, now_ms);
    assert_eq!(to_flush.len(), 10);
    assert!(remaining.is_empty());
}

#[test]
fn grouped_ws_bar_entries_chunking_is_bounded_and_lossless() {
    let mut grouped: GroupedWsBars = HashMap::new();
    for i in 0..10 {
        grouped.insert(
            ("kraken".to_string(), format!("PAIR{i}USD"), "1Min"),
            (Vec::new(), i),
        );
    }
    let chunks = chunk_grouped_ws_bar_entries(grouped, 3);
    assert_eq!(chunks.len(), 4);
    assert!(chunks.iter().all(|chunk| chunk.len() <= 3));
    assert_eq!(chunks.iter().map(Vec::len).sum::<usize>(), 10);
}

#[test]
fn grouped_ws_bar_entries_chunking_never_uses_zero_chunk_size() {
    let mut grouped: GroupedWsBars = HashMap::new();
    grouped.insert(
        ("kraken".to_string(), "BTCUSD".to_string(), "1Min"),
        (Vec::new(), 1),
    );
    grouped.insert(
        ("kraken".to_string(), "ETHUSD".to_string(), "1Min"),
        (Vec::new(), 2),
    );
    let chunks = chunk_grouped_ws_bar_entries(grouped, 0);
    assert_eq!(chunks.len(), 2);
    assert!(chunks.iter().all(|chunk| chunk.len() == 1));
}
