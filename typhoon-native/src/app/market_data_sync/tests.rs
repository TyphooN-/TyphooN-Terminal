use super::*;

#[test]
fn broad_dispatch_rotate_runs_one_lane_per_visible_pass() {
    // Visible budget is 1: only the first runnable lane runs; the cursor
    // lands just past it so the next pass starts at the next lane.
    let mut log = Vec::new();
    let (ran, cursor) = broad_dispatch_rotate(0, 3, 1, |lane| {
        log.push(lane);
        true
    });
    assert_eq!((ran, cursor), (1, 1));
    assert_eq!(log, vec![0]);

    // Next pass continues the rotation from lane 1.
    let (ran, cursor) = broad_dispatch_rotate(cursor, 3, 1, |lane| lane == 1);
    assert_eq!((ran, cursor), (1, 2));
}

#[test]
fn broad_dispatch_rotate_declined_lanes_do_not_consume_budget() {
    // Lane 0 declines (disabled / not due) — lane 1 still runs this pass.
    let mut offered = Vec::new();
    let (ran, cursor) = broad_dispatch_rotate(0, 3, 1, |lane| {
        offered.push(lane);
        lane == 1
    });
    assert_eq!((ran, cursor), (1, 2));
    assert_eq!(offered, vec![0, 1]);

    // Nothing runnable: no lane runs and the cursor stays put.
    let (ran, cursor) = broad_dispatch_rotate(2, 3, 1, |_| false);
    assert_eq!((ran, cursor), (0, 2));
}

#[test]
fn broad_dispatch_rotate_hidden_budget_runs_every_due_lane() {
    // Hidden passes have no frame pacing to protect: budget = lane count,
    // every lane is offered exactly once and all due lanes run.
    let mut log = Vec::new();
    let (ran, cursor) = broad_dispatch_rotate(1, 3, 3, |lane| {
        log.push(lane);
        lane != 2 // lane 2 not due
    });
    assert_eq!(ran, 2);
    assert_eq!(log, vec![1, 2, 0], "offered once each, starting at cursor");
    assert_eq!(cursor, 1, "cursor lands past the last lane that ran (0)");
}

#[test]
fn broad_dispatch_lane_due_periodic_and_refill_floor() {
    let interval = std::time::Duration::from_secs(1);
    let now = std::time::Instant::now();
    let due = |elapsed_ms: u64, refill: bool| {
        TyphooNApp::broad_dispatch_lane_due(
            now - std::time::Duration::from_millis(elapsed_ms),
            now,
            interval,
            refill,
        )
    };
    // Periodic cadence holds with no refill pending.
    assert!(due(1_000, false));
    assert!(!due(999, false));
    // A pending refill pulls a lane in early, but never below the 250ms
    // floor — a continuous settlement stream can't scan every frame.
    assert!(due(250, true));
    assert!(!due(249, true));
    assert!(!due(100, true));
}

#[test]
fn broad_scan_slot_floor_amortizes_full_tilt_but_leaves_balanced_unchanged() {
    // Balanced mode (small batch) collapses to floor 1 — identical to the old
    // `available > 0` gate, so nothing off full-tilt changes behaviour.
    assert_eq!(broad_scan_slot_floor(1), 1);
    assert_eq!(broad_scan_slot_floor(3), 1);
    assert_eq!(broad_scan_slot_floor(4), 1);
    // Full-tilt batches gate on a quarter-batch of free slots, so the catalog
    // scan runs ~batch/4x less often during steady-state catch-up.
    assert_eq!(broad_scan_slot_floor(128), 32);
    assert_eq!(broad_scan_slot_floor(256), 64);
    // The floor never exceeds the batch size, so `available` (capped at the
    // batch) can always reach it as in-flight fetches complete — no wedge.
    for batch in [1usize, 2, 4, 16, 64, 128, 256, 1024] {
        assert!(broad_scan_slot_floor(batch) <= batch);
    }
}

#[test]
fn market_status_is_idle_only_for_closed_and_overnight() {
    // Idle = no live regular session; adaptive backoff engages only here.
    assert!(market_status_is_idle("US equities CLOSED · opens in 6h"));
    assert!(market_status_is_idle(
        "US equities OVERNIGHT · next pre-market in 5h 44m"
    ));
    // Live regular sessions and an unfetched clock read not-idle (backoff
    // bypassed → fast resync).
    assert!(!market_status_is_idle("US equities OPEN · closes in 5h 0m"));
    assert!(!market_status_is_idle(
        "US equities PRE-MARKET · Core in 55m"
    ));
    assert!(!market_status_is_idle(
        "US equities AFTER-HOURS · closes in 3h 0m"
    ));
    assert!(!market_status_is_idle(""));
}

#[test]
fn refetch_backoff_secs_caps_at_one_timeframe_period() {
    // 15Min: base = 900/2 = 450s. streak 0 keeps the fast catch-up cadence.
    assert_eq!(refetch_backoff_secs(900, 0), 450);
    // Any empty streak caps at exactly one period — the bar-formation rate,
    // the most aggressive re-check that can still surface a new bar.
    assert_eq!(refetch_backoff_secs(900, 1), 900);
    assert_eq!(refetch_backoff_secs(900, 2), 900);
    assert_eq!(refetch_backoff_secs(900, 40), 900);
    // Scales per-timeframe, not a fixed ceiling: 5Min → 5min, 4Hour → 4h.
    assert_eq!(refetch_backoff_secs(300, 5), 300);
    assert_eq!(refetch_backoff_secs(14_400, 5), 14_400);
    // Base floor: even a tiny period never re-probes faster than 30s.
    assert_eq!(refetch_backoff_secs(40, 0), 30);
}

#[test]
fn intraday_equity_sync_tf_excludes_daily_and_higher() {
    for tf in ["5Min", "15Min", "30Min", "1Hour", "4Hour"] {
        assert!(is_intraday_equity_sync_tf(tf), "{tf} should be intraday");
    }
    for tf in ["1Min", "1Day", "1Week", "1Month"] {
        assert!(!is_intraday_equity_sync_tf(tf), "{tf} should not be gated");
    }
}

#[test]
fn build_source_sync_state_maps_buckets_by_source_and_keeps_newest() {
    let detailed = vec![
        ("alpaca:AAPL:1Day".to_string(), 100i64, 1_000i64),
        ("alpaca:AAPL:1Day".to_string(), 250, 2_000), // newer write wins
        ("kraken:ETHUSD:1Hour".to_string(), 50, 1_500),
        ("yahoo-chart:msft:1Week".to_string(), 7, 1_200), // lowercase symbol
        ("kraken-equities:TNDM.EQ:1Day".to_string(), 9, 1_100), // .EQ stripped
        ("kraken-futures:XBTUSD:4Hour".to_string(), 3, 1_050),
        ("merged:AAPL:1Day".to_string(), 999, 9_999), // untracked source
        ("default:AAPL:1Day".to_string(), 999, 9_999), // untracked source
        ("alpaca:__META__:1Day".to_string(), 1, 9_999), // meta key skipped
        ("alpaca:BADKEY".to_string(), 1, 9_999),      // no timeframe → skipped
        ("alpaca:AAPL:1Day:extra".to_string(), 1, 9_999), // extra segment → skipped
    ];
    let bar_ts: std::collections::HashMap<String, (i64, i64, i64)> =
        std::collections::HashMap::from([("alpaca:AAPL:1Day".to_string(), (0, 5_000_000, 0))]);
    let maps = build_source_sync_state_maps(&detailed, &bar_ts);

    let alpaca = &maps["alpaca:"];
    assert_eq!(
        alpaca.len(),
        1,
        "only the valid AAPL:1Day pair; meta/malformed keys skipped"
    );
    let aapl = alpaca[&("AAPL".to_string(), "1Day".to_string())];
    assert_eq!(aapl.bar_count, 250, "newest write_ts wins");
    assert_eq!(aapl.write_ts_s, 2_000);
    assert_eq!(
        aapl.last_bar_ts_s, 5_000,
        "last_bar_ts from bar_ts_cache, ms→s"
    );

    // Every tracked lane buckets under its own prefix; untracked sources don't.
    assert_eq!(maps["kraken:"].len(), 1);
    assert_eq!(maps["yahoo-chart:"].len(), 1);
    assert_eq!(maps["kraken-equities:"].len(), 1);
    assert_eq!(maps["kraken-futures:"].len(), 1);
    assert!(!maps.contains_key("merged:"), "merged is not a sync lane");
    assert!(!maps.contains_key("default:"), "default is not a sync lane");
}

#[test]
fn kraken_equity_native_symbols_for_timeframe_is_demand_scoped() {
    let catalog = vec!["TNDM.EQ".to_string(), "wok".to_string(), "TNDM".to_string()];
    let demand = vec!["POM.EQ".to_string(), "array".to_string()];

    // Native rows count only the demand set, regardless of catalog size, so
    // the "Kraken Equities" status row can converge to ~100%.
    for tf in ["15Min", "1Day", "1Week"] {
        assert_eq!(
            kraken_equity_native_symbols_for_timeframe(&catalog, &demand, tf),
            vec!["ARRAY".to_string(), "POM".to_string()],
            "{tf} native row should be demand-scoped"
        );
    }
    assert!(kraken_equity_native_symbols_for_timeframe(&catalog, &demand, "1Month").is_empty());
    assert_eq!(
        kraken_equity_native_symbols_for_timeframe(&[], &demand, "1Day"),
        vec!["ARRAY".to_string(), "POM".to_string()]
    );
}

#[test]
fn kraken_equity_symbols_for_timeframe_uses_catalog_for_all_supported_merged_timeframes() {
    let catalog = vec!["TNDM.EQ".to_string(), "wok".to_string(), "TNDM".to_string()];
    let demand = vec!["POM.EQ".to_string(), "array".to_string()];

    for tf in ["1Min", "5Min", "15Min", "1Day", "1Week", "1Month"] {
        assert_eq!(
            kraken_equity_symbols_for_timeframe(&catalog, &demand, tf),
            vec!["TNDM".to_string(), "WOK".to_string()],
            "{tf} should be catalog-scoped when the Kraken Equities universe is loaded"
        );
    }
    assert_eq!(
        kraken_equity_symbols_for_timeframe(&[], &demand, "1Day"),
        vec!["ARRAY".to_string(), "POM".to_string()]
    );
    assert_eq!(
        kraken_equity_symbols_for_timeframe(&[], &demand, "15Min"),
        vec!["ARRAY".to_string(), "POM".to_string()]
    );
}

#[test]
fn normalize_kraken_equity_symbol_list_strips_wrappers_and_dedupes() {
    let raw = vec![
        "tndm.eq".to_string(),
        "TNDM".to_string(),
        "".to_string(),
        "w/ok.EQ".to_string(),
    ];
    assert_eq!(
        normalize_kraken_equity_symbol_list(raw.iter()),
        vec!["TNDM".to_string(), "WOK".to_string()]
    );
}

fn symbols(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

#[test]
fn kraken_equity_native_history_is_demand_scoped_not_full_catalog() {
    let catalog = symbols(&["AAPL", "MSFT", "NVDA", "TSLA"]);
    let demand = symbols(&["WOK.EQ", "AAPLx/USD"]);

    let selected = kraken_equity_native_history_symbols(&catalog, &demand);

    // iapi (~6 req/s, 1015-banned on overshoot) owns only the demand depth
    // lane; the catalog's breadth is carried by Alpaca/Yahoo + the merge.
    assert_eq!(selected, symbols(&["AAPLXUSD", "WOK"]));
}

#[test]
fn kraken_equity_native_history_falls_back_to_demand_before_catalog_loads() {
    let catalog: Vec<String> = Vec::new();
    let demand = symbols(&["WOK.EQ", "AAPLx/USD"]);

    let selected = kraken_equity_native_history_symbols(&catalog, &demand);

    // Until the catalog has loaded, repair the active/held/watchlist set so
    // open charts still backfill instead of waiting on the universe fetch.
    assert_eq!(selected, symbols(&["AAPLXUSD", "WOK"]));
}

#[test]
fn kraken_equity_native_is_demand_scoped_while_assist_lanes_stay_catalog_broad() {
    let catalog = symbols(&["MSFT", "AAPL"]);
    let demand = symbols(&["WOK.EQ"]);

    assert_eq!(
        kraken_equity_native_symbols_for_timeframe(&catalog, &demand, "1Day"),
        symbols(&["WOK"]),
        "native iapi/WS rows are demand-scoped depth, not catalog breadth"
    );
    assert_eq!(
        kraken_equity_symbols_for_timeframe(&catalog, &demand, "1Month"),
        symbols(&["AAPL", "MSFT"]),
        "assist/merged broad lanes (Alpaca/Yahoo) still rotate over the catalog"
    );
}

#[test]
fn batch_topup_limit_covers_gap_with_headroom_and_stays_bounded() {
    // 3-day gap on 1Day bars: gap 3 + headroom 8 = 11, floored at 64 so a
    // batch is never pointlessly narrow.
    assert_eq!(
        TyphooNApp::alpaca_batch_topup_limit_bars("1Day", 3 * 86_400),
        64
    );
    // 200-day gap: 200 + 100 headroom = 300 — a bounded window instead of
    // the old full 10k-bar history re-pull.
    assert_eq!(
        TyphooNApp::alpaca_batch_topup_limit_bars("1Day", 200 * 86_400),
        300
    );
    // Pathological ages clamp at the deep-history ceiling.
    assert_eq!(
        TyphooNApp::alpaca_batch_topup_limit_bars("15Min", 400 * 86_400),
        TyphooNApp::ALPACA_BATCH_DEEP_HISTORY_BARS
    );
}

#[test]
fn memory_pressure_uses_system_relative_rss_and_available_thresholds() {
    assert_eq!(
        market_data_memory_pressure_at(11_500, 32_000, 20_000),
        MarketDataMemoryPressure::Normal
    );
    assert_eq!(
        market_data_memory_pressure_at(12_500, 32_000, 20_000),
        MarketDataMemoryPressure::Reduced
    );
    assert_eq!(
        market_data_memory_pressure_at(15_500, 32_000, 20_000),
        MarketDataMemoryPressure::PauseBackground
    );
    assert_eq!(
        market_data_memory_pressure_at(8_000, 32_000, 10_000),
        MarketDataMemoryPressure::PauseBackground
    );
    assert_eq!(
        market_data_memory_pressure_at(0, 32_000, 20_000),
        MarketDataMemoryPressure::Normal
    );
}

#[test]
fn memory_pressure_fallback_without_meminfo_is_still_bounded() {
    assert_eq!(
        market_data_memory_pressure_at(11_999, 0, 0),
        MarketDataMemoryPressure::Normal
    );
    assert_eq!(
        market_data_memory_pressure_at(12_000, 0, 0),
        MarketDataMemoryPressure::Reduced
    );
    assert_eq!(
        market_data_memory_pressure_at(16_000, 0, 0),
        MarketDataMemoryPressure::PauseBackground
    );
}

#[test]
fn low_memory_sync_budget_scales_full_tilt_work_before_pressure_spikes() {
    assert_eq!(low_memory_sync_budget_percent(16_384), 35);
    assert_eq!(low_memory_sync_budget_percent(32_000), 50);
    assert_eq!(low_memory_sync_budget_percent(49_152), 75);
    assert_eq!(low_memory_sync_budget_percent(98_304), 100);

    assert_eq!(memory_scaled_sync_budget(256, 32_000, 32), 128);
    assert_eq!(memory_scaled_sync_budget(24, 32_000, 6), 12);
    assert_eq!(memory_scaled_sync_budget(6, 16_384, 6), 6);
    assert_eq!(memory_scaled_sync_budget(256, 98_304, 32), 256);
}

#[test]
fn background_retry_dispatch_stops_when_pending_pressure_is_high() {
    assert!(background_retry_dispatch_allowed(0));
    assert!(background_retry_dispatch_allowed(
        BACKGROUND_RETRY_PENDING_FETCH_CAP - 1
    ));
    assert!(!background_retry_dispatch_allowed(
        BACKGROUND_RETRY_PENDING_FETCH_CAP
    ));
}

#[test]
fn alpaca_background_sync_pause_is_time_bounded() {
    assert!(!alpaca_background_sync_paused_until(999, 1000));
    assert!(!alpaca_background_sync_paused_until(1000, 1000));
    assert!(alpaca_background_sync_paused_until(1001, 1000));
}

#[test]
fn background_fetch_backpressure_preserves_focus_symbols() {
    assert!(!background_market_data_fetch_allowed(
        false,
        BACKGROUND_RETRY_PENDING_FETCH_CAP
    ));
    // RSS guard is environment-dependent; we only assert the pending_fetches path here.

    assert!(background_market_data_fetch_allowed(true, 0));
    assert!(background_market_data_fetch_allowed(
        true,
        BACKGROUND_RETRY_PENDING_FETCH_CAP * 10
    ));
}
