use super::*;

#[test]
fn yahoo_chart_429_backoff_escalates_then_caps() {
    // Isolated 429 (consecutive == 1) recovers fast; sustained limiting escalates
    // to a protective ceiling. Zero is treated as the first event.
    assert_eq!(yahoo_chart_429_backoff_secs(0), 45);
    assert_eq!(yahoo_chart_429_backoff_secs(1), 45);
    assert_eq!(yahoo_chart_429_backoff_secs(2), 90);
    assert_eq!(yahoo_chart_429_backoff_secs(3), 180);
    assert_eq!(yahoo_chart_429_backoff_secs(4), 360);
    assert_eq!(yahoo_chart_429_backoff_secs(5), 600);
    assert_eq!(yahoo_chart_429_backoff_secs(6), 600);
    assert_eq!(yahoo_chart_429_backoff_secs(100), 600);
}

#[test]
fn routine_market_data_status_filters_alpaca_progress_noise() {
    assert!(is_routine_market_data_status(
        "Alpaca GOOGL 1Week: fetching full server history (first sync)..."
    ));
    assert!(is_routine_market_data_status(
        "Alpaca TNDM 1Hour delta since 2022-09-16T17:00:00 (limit 1000)..."
    ));
    assert!(is_routine_market_data_status(
        "Alpaca AAPL 1Day: cache has 10 bars — syncing full server history..."
    ));
}

#[test]
fn routine_market_data_status_keeps_actionable_alpaca_messages_visible() {
    assert!(!is_routine_market_data_status(
        "Alpaca fetched 554 bars for WOK 4Hour — queued active chart reload"
    ));
    assert!(!is_routine_market_data_status(
        "Alpaca retry: re-dispatched 205 symbol(s) (205 in queue)"
    ));
}

#[test]
fn json_result_card_formats_array_json_without_log_dump() {
    let raw = r#"[
        {"symbol":"AAPL","change":1.25,"nested":{"ignored":true}},
        {"symbol":"MSFT","change":-0.5,"nested":{"ignored":true}}
    ]"#;
    let Some((
        ResultCard::Table {
            title,
            headers,
            rows,
            ..
        },
        summary,
    )) = json_result_card_from_text("Top Movers", raw)
    else {
        panic!("expected table result card");
    };
    assert_eq!(title, "Top Movers");
    assert!(headers.contains(&"change".to_string()));
    assert!(headers.contains(&"symbol".to_string()));
    assert_eq!(rows.len(), 2);
    assert!(summary.contains("raw JSON"));
}

#[test]
fn json_result_card_formats_object_json_as_summary() {
    let raw = r#"{"equity":12345.67,"status":"ok","positions":[{"symbol":"AAPL"}]}"#;
    let Some((ResultCard::Summary { title, metrics }, summary)) =
        json_result_card_from_text("Portfolio History", raw)
    else {
        panic!("expected summary result card");
    };
    assert_eq!(title, "Portfolio History");
    assert!(
        metrics
            .iter()
            .any(|(k, v, _)| k == "equity" && v == "12345.67")
    );
    assert!(metrics.iter().any(|(k, v, _)| k == "status" && v == "ok"));
    assert!(summary.contains("field"));
}

#[test]
fn alpaca_retry_queue_log_is_milestoned() {
    assert!(!should_emit_alpaca_retry_queue_log(0));
    assert!(!should_emit_alpaca_retry_queue_log(1));
    assert!(!should_emit_alpaca_retry_queue_log(99));
    assert!(should_emit_alpaca_retry_queue_log(100));
    assert!(should_emit_alpaca_retry_queue_log(200));
}

#[test]
fn alpaca_retry_dispatch_log_is_milestoned_not_every_tick() {
    assert!(!should_emit_alpaca_retry_dispatch_log(0));
    assert!(!should_emit_alpaca_retry_dispatch_log(24));
    assert!(!should_emit_alpaca_retry_dispatch_log(263));
    assert!(should_emit_alpaca_retry_dispatch_log(300));
}

#[test]
fn alpaca_retry_reason_detects_rate_limit_variants() {
    assert!(alpaca_retry_reason_is_rate_limited("rate_limited_empty"));
    assert!(alpaca_retry_reason_is_rate_limited("batch_rate_limited_partial"));
    assert!(alpaca_retry_reason_is_rate_limited("err:HTTP 429 (feed=Some(\"iex\"))"));
    assert!(alpaca_retry_reason_is_rate_limited("Rate limit exceeded"));
    assert!(!alpaca_retry_reason_is_rate_limited("provider returned no bars"));
}

#[test]
fn alpaca_sync_429_pause_escalates_and_caps() {
    assert_eq!(alpaca_sync_429_pause_secs(0), 60);
    assert_eq!(alpaca_sync_429_pause_secs(1), 60);
    assert_eq!(alpaca_sync_429_pause_secs(2), 120);
    assert_eq!(alpaca_sync_429_pause_secs(3), 300);
    assert_eq!(alpaca_sync_429_pause_secs(4), 600);
    assert_eq!(alpaca_sync_429_pause_secs(99), 600);
}

#[test]
fn broad_kraken_fundamentals_auto_scrape_is_bounded() {
    assert!(!should_auto_start_kraken_fundamentals_scrape(0));
    assert!(should_auto_start_kraken_fundamentals_scrape(512));
    assert!(!should_auto_start_kraken_fundamentals_scrape(513));
    assert!(!should_auto_start_kraken_fundamentals_scrape(12_268));
}

#[test]
fn kraken_xstocks_session_status_tracks_all_24_5_sessions() {
    let at = |ts: &str| {
        chrono::DateTime::parse_from_rfc3339(ts)
            .unwrap()
            .with_timezone(&chrono::Utc)
    };

    assert!(
        kraken_xstocks_session_status_at(at("2026-06-01T07:30:00Z"), true)
            .starts_with("Kraken xStocks OVERNIGHT · next pre-market")
    );
    assert!(
        kraken_xstocks_session_status_at(at("2026-06-01T12:00:00Z"), true)
            .starts_with("Kraken xStocks PRE · next core")
    );
    assert!(
        kraken_xstocks_session_status_at(at("2026-06-01T15:00:00Z"), true)
            .starts_with("Kraken xStocks CORE · next after-hours")
    );
    assert!(
        kraken_xstocks_session_status_at(at("2026-06-01T21:00:00Z"), true)
            .starts_with("Kraken xStocks AFTER · next overnight")
    );
    assert!(
        kraken_xstocks_session_status_at(at("2026-06-02T01:00:00Z"), true)
            .starts_with("Kraken xStocks OVERNIGHT · next pre-market")
    );
}

#[test]
fn kraken_xstocks_session_status_closes_overnight_window_without_overnight_support() {
    let at = |ts: &str| {
        chrono::DateTime::parse_from_rfc3339(ts)
            .unwrap()
            .with_timezone(&chrono::Utc)
    };
    // 21:00 UTC = 17:00 ET (after-hours): a no-overnight symbol counts down to
    // the 8 PM close, not to an overnight session.
    assert!(
        kraken_xstocks_session_status_at(at("2026-06-01T21:00:00Z"), false)
            .starts_with("Kraken xStocks AFTER · closes"),
        "no-overnight after-hours should close at 8 PM"
    );
    // 01:00 UTC Tue = 21:00 ET Mon (overnight window) and 07:30 UTC = 03:30 ET
    // (overnight window): a no-overnight symbol is CLOSED until pre-market.
    assert!(
        kraken_xstocks_session_status_at(at("2026-06-02T01:00:00Z"), false)
            .starts_with("Kraken xStocks CLOSED · opens pre-market")
    );
    assert!(
        kraken_xstocks_session_status_at(at("2026-06-01T07:30:00Z"), false)
            .starts_with("Kraken xStocks CLOSED · opens pre-market")
    );
    // Core hours are unaffected by overnight support.
    assert!(
        kraken_xstocks_session_status_at(at("2026-06-01T15:00:00Z"), false)
            .starts_with("Kraken xStocks CORE · next after-hours")
    );
}

#[test]
fn kraken_xstocks_session_status_is_holiday_aware() {
    let at = |ts: &str| {
        chrono::DateTime::parse_from_rfc3339(ts)
            .unwrap()
            .with_timezone(&chrono::Utc)
    };
    // 2026-07-03 (Fri) is observed Independence Day: noon ET reads CLOSED with
    // the holiday name, not a normal CORE session (ADR-110).
    let holiday_noon = kraken_xstocks_session_status_at(at("2026-07-03T16:00:00Z"), true);
    assert!(
        holiday_noon.starts_with("Kraken xStocks CLOSED · US market holiday (Independence Day)"),
        "got {holiday_noon}"
    );
    // Labor Day 2026-09-07 (Mon): the weekend Sunday-20:00-ET open must not
    // fire; the market opens Tuesday pre-market instead.
    let sunday_before_labor_day =
        kraken_xstocks_session_status_at(at("2026-09-06T16:00:00Z"), true);
    assert!(
        sunday_before_labor_day.starts_with("Kraken xStocks CLOSED · US market holiday Monday"),
        "got {sunday_before_labor_day}"
    );
    // Thursday 21:00 ET before Good Friday 2026-04-03: no overnight session
    // into a holiday — closed until the next trading day's pre-market (Monday).
    let overnight_into_good_friday =
        kraken_xstocks_session_status_at(at("2026-04-03T01:00:00Z"), true);
    assert!(
        overnight_into_good_friday.starts_with("Kraken xStocks CLOSED · US market holiday next"),
        "got {overnight_into_good_friday}"
    );
    // Thursday afternoon before Good Friday: after-hours counts down to the
    // 8 PM close instead of promising an overnight session.
    let after_before_good_friday =
        kraken_xstocks_session_status_at(at("2026-04-02T21:00:00Z"), true);
    assert!(
        after_before_good_friday.starts_with("Kraken xStocks AFTER · closes"),
        "got {after_before_good_friday}"
    );
}

#[test]
fn kraken_xstocks_session_status_closes_only_for_weekend_window() {
    let friday_after = chrono::DateTime::parse_from_rfc3339("2026-06-05T23:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    let saturday = chrono::DateTime::parse_from_rfc3339("2026-06-06T16:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    let sunday_open = chrono::DateTime::parse_from_rfc3339("2026-06-08T00:30:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);

    assert!(
        kraken_xstocks_session_status_at(friday_after, true)
            .starts_with("Kraken xStocks AFTER · closes")
    );
    assert!(kraken_xstocks_session_status_at(saturday, true).starts_with("Kraken xStocks CLOSED"));
    assert!(
        kraken_xstocks_session_status_at(sunday_open, true).starts_with("Kraken xStocks OVERNIGHT")
    );
}

#[test]
fn routine_news_progress_filters_scope_scrape_noise() {
    assert!(is_routine_news_progress(
        "News ETH/USD: base asset ETH already fetched — skipped network (2/42)"
    ));
    assert!(is_routine_news_progress(
        "News AAPL: cached/fresh — skipped network (7/42)"
    ));
    assert!(is_routine_news_progress("News MSFT: 12 cached (8/42)"));
    assert!(is_routine_news_progress("news/yahoo_rss AAPL: 20 articles"));
    assert!(is_routine_news_progress("news/AAPL: 20 articles fetched"));
    assert!(!is_routine_news_progress(
        "News scrape complete: 41 OK, 1 failed of 42 symbol(s)"
    ));
}

#[test]
fn news_scope_scrape_start_log_summarizes_large_symbol_sets() {
    let tickers: Vec<String> = (0..200).map(|i| format!("SYM{i}")).collect();
    let msg = format_news_scope_scrape_start(&tickers);

    assert!(msg.contains("200 symbol(s)"));
    assert!(msg.contains("SYM0, SYM1, SYM2"));
    assert!(!msg.contains("SYM199"));
    assert!(msg.len() < 240);
}

#[test]
fn auto_background_scope_scrape_skips_large_all_universe_after_symbols_load() {
    assert!(should_auto_start_background_scope_scrape(
        EventSource::All,
        12
    ));
    assert!(!should_auto_start_background_scope_scrape(
        EventSource::All,
        12_000
    ));
    assert!(!should_auto_start_background_scope_scrape(
        EventSource::Kraken,
        12_000
    ));
    assert!(should_auto_start_background_scope_scrape(
        EventSource::Kraken,
        512
    ));
    assert!(!should_auto_start_background_scope_scrape(
        EventSource::All,
        0
    ));
}

#[test]
fn manual_background_scope_scrape_blocks_large_all_during_heavy_sync() {
    assert!(should_start_manual_background_scope_scrape(
        EventSource::All,
        12_000,
        false
    ));
    assert!(!should_start_manual_background_scope_scrape(
        EventSource::All,
        12_000,
        true
    ));
    assert!(should_start_manual_background_scope_scrape(
        EventSource::All,
        12,
        true
    ));
    assert!(should_start_manual_background_scope_scrape(
        EventSource::Kraken,
        12_000,
        true
    ));
    assert!(!should_start_manual_background_scope_scrape(
        EventSource::All,
        0,
        false
    ));
}

#[test]
fn heavy_sync_gate_tracks_bulk_work_not_light_idle() {
    assert!(!ui_heavy_sync_active(0, 0, false, false, false, false));
    assert!(!ui_heavy_sync_active(
        HEAVY_SYNC_PENDING_FETCH_THRESHOLD - 1,
        HEAVY_SYNC_DEFERRED_CHART_THRESHOLD - 1,
        false,
        false,
        false,
        false
    ));
    assert!(ui_heavy_sync_active(
        HEAVY_SYNC_PENDING_FETCH_THRESHOLD,
        0,
        false,
        false,
        false,
        false
    ));
    assert!(ui_heavy_sync_active(
        0,
        HEAVY_SYNC_DEFERRED_CHART_THRESHOLD,
        false,
        false,
        false,
        false
    ));
    assert!(ui_heavy_sync_active(0, 0, true, false, false, false));
    assert!(ui_heavy_sync_active(0, 0, false, true, false, false));
    assert!(ui_heavy_sync_active(0, 0, false, false, true, false));
    assert!(ui_heavy_sync_active(0, 0, false, false, false, true));
}

#[test]
fn ui_task_watchdog_marks_stale_and_clears_when_idle() {
    let now = std::time::Instant::now();
    let mut started = Some(now - std::time::Duration::from_secs(10));

    assert!(!ui_task_is_stale(
        true,
        &mut started,
        now,
        std::time::Duration::from_secs(30)
    ));
    assert!(started.is_some());
    assert!(ui_task_is_stale(
        true,
        &mut started,
        now,
        std::time::Duration::from_secs(5)
    ));
    assert!(!ui_task_is_stale(
        false,
        &mut started,
        now,
        std::time::Duration::from_secs(5)
    ));
    assert!(started.is_none());
}

#[test]
fn xstocks_weekend_closed_spans_friday_8pm_to_sunday_8pm_et() {
    fn at(ts: &str) -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339(ts)
            .unwrap()
            .with_timezone(&chrono::Utc)
    }
    // June ⇒ EDT (UTC−4). 2026-06-12 is a Friday.
    assert!(!kraken_xstocks_weekend_closed_at(at(
        "2026-06-12T18:00:00Z"
    ))); // Fri 14:00 ET — open
    assert!(kraken_xstocks_weekend_closed_at(at("2026-06-13T01:00:00Z"))); // Fri 21:00 ET — closed
    assert!(kraken_xstocks_weekend_closed_at(at("2026-06-13T12:00:00Z"))); // Sat 08:00 ET — closed
    assert!(kraken_xstocks_weekend_closed_at(at("2026-06-14T20:00:00Z"))); // Sun 16:00 ET — closed
    assert!(!kraken_xstocks_weekend_closed_at(at(
        "2026-06-15T01:00:00Z"
    ))); // Sun 21:00 ET — reopened
    assert!(!kraken_xstocks_weekend_closed_at(at(
        "2026-06-10T15:00:00Z"
    ))); // Wed 11:00 ET — open
}
