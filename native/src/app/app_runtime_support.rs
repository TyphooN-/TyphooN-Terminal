use super::*;

pub(super) fn is_routine_market_data_status(msg: &str) -> bool {
    let routine_progress = msg.contains(": fetching recent window...")
        || msg.contains(": provider-window cache ")
        || msg.contains(": fetching full server history")
        || msg.contains(": cache has ") && msg.contains(" — syncing full server history")
        || msg.contains(" delta since ")
        || msg.contains(" already up to date")
        || msg.contains(": no bars returned")
        || msg.contains(": MT5/Darwinex has this symbol — skipping")
        || msg.contains(": tastytrade has this symbol — skipping");

    (msg.starts_with("Kraken ") || msg.starts_with("Alpaca ")) && routine_progress
        || msg.starts_with("CryptoCompare backfill ")
        || msg.contains("Yahoo Chart HTTP 429")
        // Once the iapi back-off is armed, late-arriving dispatches that
        // race the gate come back with this prefix. The first arm already
        // produced a tracing::warn at the engine layer, so the user-log
        // path should treat repeats as routine status instead of stacking
        // a red error per balance tick.
        || msg.starts_with(typhoon_engine::broker::kraken::IAPI_RATE_LIMITED_ERR_PREFIX)
}

pub(super) fn should_emit_alpaca_retry_queue_log(queue_len: usize) -> bool {
    queue_len > 0 && queue_len.is_multiple_of(100)
}

pub(super) fn is_routine_news_progress(msg: &str) -> bool {
    (msg.starts_with("News ")
        && (msg.contains(": base asset ")
            || msg.contains(": cached/fresh — skipped network")
            || msg.contains(" cached (")
            || msg.contains(" failed:")))
        || (msg.starts_with("news/")
            && (msg.contains(" articles") || msg.contains(" cached") || msg.contains("failed:")))
}

const HEAVY_SYNC_PENDING_FETCH_THRESHOLD: usize = 32;
const HEAVY_SYNC_DEFERRED_CHART_THRESHOLD: usize = 4;
pub(super) fn should_auto_start_background_scope_scrape(
    scope: EventSource,
    symbol_count: usize,
) -> bool {
    // Broad Scope ALL is valid when the user explicitly asks for it, but
    // auto-starting a 12k-symbol SEC sweep on startup turns chart interaction
    // into molasses: the scrape pounds SQLite/EDGAR while egui is trying to
    // render and apply camera drags. Keep automatic startup scrapes bounded;
    // manual ALL still goes through the full universe at click time.
    symbol_count > 0 && (!matches!(scope, EventSource::All) || symbol_count <= 512)
}

pub(super) fn should_auto_start_kraken_fundamentals_scrape(symbol_count: usize) -> bool {
    // Same startup-safety rule as SEC/news broad scraping: Kraken xStocks ALL is
    // a valid manual universe scrape, but launching 12k fundamentals requests as
    // soon as the catalog lands piles onto cache/news/SEC sync and makes the UI
    // unusable. Keep automatic startup recovery bounded to focused/small scopes.
    symbol_count > 0 && symbol_count <= 512
}

pub(super) fn format_session_countdown(duration: chrono::Duration) -> String {
    let seconds = duration.num_seconds().max(0);
    let hours = seconds / 3_600;
    let minutes = (seconds % 3_600) / 60;
    if hours >= 24 {
        format!("{}d {}h", hours / 24, hours % 24)
    } else if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else {
        format!("{}m", minutes.max(1))
    }
}

pub(super) fn nth_sunday(year: i32, month: u32, nth: u32) -> Option<chrono::NaiveDate> {
    use chrono::Datelike;
    let first = chrono::NaiveDate::from_ymd_opt(year, month, 1)?;
    let days_to_sunday = (7 - first.weekday().num_days_from_sunday()) % 7;
    first.checked_add_signed(chrono::Duration::days(
        (days_to_sunday + (nth.saturating_sub(1)) * 7) as i64,
    ))
}

pub(super) fn us_eastern_offset_seconds(now_utc: chrono::DateTime<chrono::Utc>) -> i64 {
    use chrono::Datelike;
    let year = now_utc.date_naive().year();
    // US Eastern daylight time starts at 02:00 local / 07:00 UTC on the second
    // Sunday in March and ends at 02:00 local / 06:00 UTC on the first Sunday
    // in November. Kraken's published equity/xStock sessions are in ET.
    let Some(dst_start) = nth_sunday(year, 3, 2).and_then(|d| d.and_hms_opt(7, 0, 0)) else {
        return -5 * 3_600;
    };
    let Some(dst_end) = nth_sunday(year, 11, 1).and_then(|d| d.and_hms_opt(6, 0, 0)) else {
        return -5 * 3_600;
    };
    let now_naive = now_utc.naive_utc();
    if now_naive >= dst_start && now_naive < dst_end {
        -4 * 3_600
    } else {
        -5 * 3_600
    }
}

pub(super) fn kraken_xstocks_session_status_at(now_utc: chrono::DateTime<chrono::Utc>) -> String {
    use chrono::{Datelike, Timelike};

    let now_et =
        now_utc.naive_utc() + chrono::Duration::seconds(us_eastern_offset_seconds(now_utc));
    let weekday = now_et.weekday();
    let minute_of_day = now_et.hour() as i64 * 60 + now_et.minute() as i64;
    pub(crate) const PRE: i64 = 4 * 60;
    pub(crate) const REGULAR: i64 = 9 * 60 + 30;
    pub(crate) const AFTER: i64 = 16 * 60;
    pub(crate) const OVERNIGHT: i64 = 20 * 60;

    let day_start = now_et.date().and_hms_opt(0, 0, 0).unwrap_or(now_et);
    let boundary_today = |minutes: i64| day_start + chrono::Duration::minutes(minutes);
    let next_sunday_open = || {
        let days_until_sunday = (7 - weekday.num_days_from_sunday()) % 7;
        let mut target = day_start
            + chrono::Duration::days(days_until_sunday as i64)
            + chrono::Duration::minutes(OVERNIGHT);
        if target <= now_et {
            target += chrono::Duration::days(7);
        }
        target
    };

    if (weekday == chrono::Weekday::Fri && minute_of_day >= OVERNIGHT)
        || weekday == chrono::Weekday::Sat
        || (weekday == chrono::Weekday::Sun && minute_of_day < OVERNIGHT)
    {
        let target = next_sunday_open();
        return format!(
            "Kraken xStocks CLOSED · opens Sun 8:00 PM ET in {}",
            format_session_countdown(target - now_et)
        );
    }

    let (session, next_label, target) = if minute_of_day < PRE {
        ("OVERNIGHT", "pre-market", boundary_today(PRE))
    } else if minute_of_day < REGULAR {
        ("PRE", "regular", boundary_today(REGULAR))
    } else if minute_of_day < AFTER {
        ("REGULAR", "after-hours", boundary_today(AFTER))
    } else if minute_of_day < OVERNIGHT {
        let label = if weekday == chrono::Weekday::Fri {
            "close"
        } else {
            "overnight"
        };
        ("AFTER", label, boundary_today(OVERNIGHT))
    } else {
        (
            "OVERNIGHT",
            "pre-market",
            boundary_today(PRE) + chrono::Duration::days(1),
        )
    };

    if next_label == "close" {
        format!(
            "Kraken xStocks {session} · closes in {}",
            format_session_countdown(target - now_et)
        )
    } else {
        format!(
            "Kraken xStocks {session} · next {next_label} in {}",
            format_session_countdown(target - now_et)
        )
    }
}

pub(super) fn kraken_xstocks_session_status_now() -> String {
    kraken_xstocks_session_status_at(chrono::Utc::now())
}

pub(super) fn broker_msg_kind(msg: &BrokerMsg) -> &'static str {
    match msg {
        BrokerMsg::Connected(_) => "Connected",
        BrokerMsg::Error(_) => "Error",
        BrokerMsg::Account(_) => "Account",
        BrokerMsg::Positions(_) => "Positions",
        BrokerMsg::Orders(_) => "Orders",
        BrokerMsg::OrderResult(_) => "OrderResult",
        BrokerMsg::KrakenTrades(_) => "KrakenTrades",
        BrokerMsg::KrakenLiveTrade(_) => "KrakenLiveTrade",
        BrokerMsg::KrakenOpenOrders(_) => "KrakenOpenOrders",
        BrokerMsg::KrakenWsStatus { .. } => "KrakenWsStatus",
        BrokerMsg::KrakenOrderbookUpdate(_) => "KrakenOrderbookUpdate",
        BrokerMsg::KrakenWsBarsCommitted { .. } => "KrakenWsBarsCommitted",
        BrokerMsg::KrakenWsOhlcStatus { .. } => "KrakenWsOhlcStatus",
        BrokerMsg::KrakenEquityQuote(_) => "KrakenEquityQuote",
        BrokerMsg::KrakenEquityBars { .. } => "KrakenEquityBars",
        BrokerMsg::KrakenEquityHistoryError { .. } => "KrakenEquityHistoryError",
        BrokerMsg::KrakenEquityUniverse(_) => "KrakenEquityUniverse",
        BrokerMsg::SecScrapeResult(_) => "SecScrapeResult",
        BrokerMsg::FilingContent(_) => "FilingContent",
        BrokerMsg::FinnhubNewsResult(_) => "FinnhubNewsResult",
        BrokerMsg::Quote(_, _, _, _) => "Quote",
        BrokerMsg::MarketClock(_) => "MarketClock",
        BrokerMsg::StreamTick { .. } => "StreamTick",
        BrokerMsg::StreamQuoteTick { .. } => "StreamQuoteTick",
        BrokerMsg::JsonResult(_, _) => "JsonResult",
        BrokerMsg::FundamentalsProgress(_) => "FundamentalsProgress",
        BrokerMsg::DarwinFtpScanResult(_) => "DarwinFtpScanResult",
        BrokerMsg::DarwinFtpReturns(_) => "DarwinFtpReturns",
        BrokerMsg::BarsFetched { .. } => "BarsFetched",
        BrokerMsg::AlpacaRetryEnqueue { .. } => "AlpacaRetryEnqueue",
        BrokerMsg::AlpacaNoData { .. } => "AlpacaNoData",
        BrokerMsg::AlpacaBackfillComplete { .. } => "AlpacaBackfillComplete",
        BrokerMsg::AlpacaFetchSettled { .. } => "AlpacaFetchSettled",
        BrokerMsg::KrakenFetchSettled { .. } => "KrakenFetchSettled",
        BrokerMsg::Unresolvable { .. } => "Unresolvable",
        BrokerMsg::KrakenBackfillComplete { .. } => "KrakenBackfillComplete",
        BrokerMsg::KrakenFuturesFetchSettled { .. } => "KrakenFuturesFetchSettled",
        BrokerMsg::KrakenFuturesBackfillComplete { .. } => "KrakenFuturesBackfillComplete",
        BrokerMsg::TastytradeBackfillComplete { .. } => "TastytradeBackfillComplete",
        BrokerMsg::TastytradeFetchSettled { .. } => "TastytradeFetchSettled",
        BrokerMsg::AlpacaRateLimitObserved { .. } => "AlpacaRateLimitObserved",
        BrokerMsg::SymbolSuggestions(_) => "SymbolSuggestions",
        BrokerMsg::WatchlistQuotes(_) => "WatchlistQuotes",
        BrokerMsg::FredData(_, _) => "FredData",
        BrokerMsg::EconCalendarData(_) => "EconCalendarData",
        BrokerMsg::CongressData(_) => "CongressData",
        BrokerMsg::UnusualVolumeResults(_) => "UnusualVolumeResults",
        BrokerMsg::TastytradePositions(_) => "TastytradePositions",
        BrokerMsg::TastytradeBalances(_) => "TastytradeBalances",
        BrokerMsg::KrakenPositions(_) => "KrakenPositions",
        BrokerMsg::AllAssets(_) => "AllAssets",
        BrokerMsg::RecentFills(_) => "RecentFills",
        BrokerMsg::Mt5SyncDone(_) => "Mt5SyncDone",
        BrokerMsg::Mt5LiveQuotes(_) => "Mt5LiveQuotes",
        BrokerMsg::Mt5Heartbeat(_) => "Mt5Heartbeat",
        BrokerMsg::CryptoTop50(_) => "CryptoTop50",
        BrokerMsg::KrakenBalances(_) => "KrakenBalances",
        BrokerMsg::KrakenPairs(_) => "KrakenPairs",
        BrokerMsg::KrakenFuturesInstruments(_) => "KrakenFuturesInstruments",
        BrokerMsg::TastytradeUniverse(_) => "TastytradeUniverse",
        _ => "Other",
    }
}
const NEWS_LOADING_STALE_AFTER: std::time::Duration = std::time::Duration::from_secs(180);
const FUNDAMENTALS_SCRAPE_STALE_AFTER: std::time::Duration =
    std::time::Duration::from_secs(30 * 60);
const SEC_SCRAPE_STALE_AFTER: std::time::Duration = std::time::Duration::from_secs(30 * 60);

pub(super) fn ui_task_is_stale(
    running: bool,
    started_at: &mut Option<std::time::Instant>,
    now: std::time::Instant,
    stale_after: std::time::Duration,
) -> bool {
    if !running {
        *started_at = None;
        return false;
    }
    let start = *started_at.get_or_insert(now);
    now.saturating_duration_since(start) > stale_after
}

pub(super) fn ui_heavy_sync_active(
    pending_fetches: usize,
    deferred_chart_loads: usize,
    news_loading: bool,
    scrape_fund_running: bool,
    scrape_sec_running: bool,
    auto_compact_in_progress: bool,
) -> bool {
    pending_fetches >= HEAVY_SYNC_PENDING_FETCH_THRESHOLD
        || deferred_chart_loads >= HEAVY_SYNC_DEFERRED_CHART_THRESHOLD
        || news_loading
        || scrape_fund_running
        || scrape_sec_running
        || auto_compact_in_progress
}

pub(super) fn deferred_chart_load_interval(
    heavy_sync_in_progress: bool,
    mtf_enabled: bool,
) -> std::time::Duration {
    match (heavy_sync_in_progress, mtf_enabled) {
        (true, true) => std::time::Duration::from_millis(175),
        (true, false) => std::time::Duration::from_millis(90),
        (false, true) => std::time::Duration::from_millis(75),
        (false, false) => std::time::Duration::from_millis(35),
    }
}

impl TyphooNApp {
    #[inline]
    pub(super) fn drop_bg_snapshot_off_ui(&self, data: BgDarwinData) {
        // BgDarwinData can own hundreds of thousands of SEC/news/storage rows.
        // Dropping it on the egui thread was enough to create 300ms-15s stalls
        // even when we intentionally skipped applying the snapshot. Move the
        // destructor work to a blocking worker; the update hot path only moves
        // the Vec/HashMap headers.
        self.rt_handle.spawn_blocking(move || drop(data));
    }

    #[inline]
    pub(super) fn replace_bg_snapshot_off_ui_drop(&mut self, data: BgDarwinData) {
        let old = std::mem::replace(&mut self.bg, data);
        self.drop_bg_snapshot_off_ui(old);
        self.bg_rev = self.bg_rev.wrapping_add(1);
    }

    pub(super) fn clear_stale_ui_busy_flags(&mut self, now: std::time::Instant) {
        if ui_task_is_stale(
            self.news_loading,
            &mut self.news_loading_started_at,
            now,
            NEWS_LOADING_STALE_AFTER,
        ) {
            self.news_loading = false;
            self.news_loading_started_at = None;
            self.log.push_back(LogEntry::warn(
                "News loading watchdog cleared stale busy flag after 180s".to_string(),
            ));
        }
        if ui_task_is_stale(
            self.scrape_fund_running,
            &mut self.scrape_fund_started_at,
            now,
            FUNDAMENTALS_SCRAPE_STALE_AFTER,
        ) {
            self.scrape_fund_running = false;
            self.scrape_fund_started_at = None;
            self.scrape_fund_last_msg =
                "stale fundamentals scrape flag cleared by UI watchdog".to_string();
            self.log.push_back(LogEntry::warn(
                "Fundamentals scrape watchdog cleared stale busy flag after 30m".to_string(),
            ));
        }
        if ui_task_is_stale(
            self.scrape_sec_running,
            &mut self.scrape_sec_started_at,
            now,
            SEC_SCRAPE_STALE_AFTER,
        ) {
            self.scrape_sec_running = false;
            self.scrape_sec_started_at = None;
            self.scrape_sec_last_msg = "stale SEC scrape flag cleared by UI watchdog".to_string();
            self.log.push_back(LogEntry::warn(
                "SEC scrape watchdog cleared stale busy flag after 30m".to_string(),
            ));
        }
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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
    fn alpaca_retry_queue_log_is_milestoned() {
        assert!(!should_emit_alpaca_retry_queue_log(0));
        assert!(!should_emit_alpaca_retry_queue_log(1));
        assert!(!should_emit_alpaca_retry_queue_log(99));
        assert!(should_emit_alpaca_retry_queue_log(100));
        assert!(should_emit_alpaca_retry_queue_log(200));
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
            kraken_xstocks_session_status_at(at("2026-06-01T07:30:00Z"))
                .starts_with("Kraken xStocks OVERNIGHT · next pre-market")
        );
        assert!(
            kraken_xstocks_session_status_at(at("2026-06-01T12:00:00Z"))
                .starts_with("Kraken xStocks PRE · next regular")
        );
        assert!(
            kraken_xstocks_session_status_at(at("2026-06-01T15:00:00Z"))
                .starts_with("Kraken xStocks REGULAR · next after-hours")
        );
        assert!(
            kraken_xstocks_session_status_at(at("2026-06-01T21:00:00Z"))
                .starts_with("Kraken xStocks AFTER · next overnight")
        );
        assert!(
            kraken_xstocks_session_status_at(at("2026-06-02T01:00:00Z"))
                .starts_with("Kraken xStocks OVERNIGHT · next pre-market")
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
            kraken_xstocks_session_status_at(friday_after)
                .starts_with("Kraken xStocks AFTER · closes")
        );
        assert!(kraken_xstocks_session_status_at(saturday).starts_with("Kraken xStocks CLOSED"));
        assert!(
            kraken_xstocks_session_status_at(sunday_open).starts_with("Kraken xStocks OVERNIGHT")
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
        assert!(should_auto_start_background_scope_scrape(
            EventSource::Kraken,
            12_000
        ));
        assert!(!should_auto_start_background_scope_scrape(
            EventSource::All,
            0
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
    pub(super) fn deferred_chart_load_interval_paces_heavy_mtf_restores() {
        assert!(
            deferred_chart_load_interval(true, true) > deferred_chart_load_interval(true, false)
        );
        assert!(
            deferred_chart_load_interval(true, false) > deferred_chart_load_interval(false, false)
        );
        assert!(
            deferred_chart_load_interval(false, true) > deferred_chart_load_interval(false, false)
        );
    }
}
