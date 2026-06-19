use super::*;

pub(super) fn is_routine_market_data_status(msg: &str) -> bool {
    let routine_progress = msg.contains(": fetching recent window...")
        || msg.contains(": provider-window cache ")
        || msg.contains(": fetching full server history")
        || msg.contains(": cache has ") && msg.contains(" — syncing full server history")
        || msg.contains(" delta since ")
        || msg.contains(" already up to date")
        || msg.contains(": no bars returned");

    (msg.starts_with("Kraken ") || msg.starts_with("Alpaca ")) && routine_progress
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

pub(super) fn json_result_card_from_text(label: &str, text: &str) -> Option<(ResultCard, String)> {
    let trimmed = text.trim_start();
    if !(trimmed.starts_with('{') || trimmed.starts_with('[')) {
        return None;
    }

    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    match value {
        serde_json::Value::Array(items) => json_array_result_card(label, &items, text.len()),
        serde_json::Value::Object(map) => json_object_result_card(label, &map, text.len()),
        _ => None,
    }
}

fn json_array_result_card(
    label: &str,
    items: &[serde_json::Value],
    byte_len: usize,
) -> Option<(ResultCard, String)> {
    let first_obj = items.iter().find_map(|v| v.as_object())?;
    let headers: Vec<String> = first_obj
        .iter()
        .filter(|(_, v)| json_value_is_compact_scalar(v))
        .take(5)
        .map(|(k, _)| k.clone())
        .collect();
    if headers.is_empty() {
        return None;
    }

    let rows: Vec<Vec<String>> = items
        .iter()
        .filter_map(|v| v.as_object())
        .take(12)
        .map(|obj| {
            headers
                .iter()
                .map(|k| obj.get(k).map(format_json_scalar).unwrap_or_default())
                .collect()
        })
        .collect();
    if rows.is_empty() {
        return None;
    }

    let summary = format!(
        "{label}: {} row{} shown as a result card (raw JSON {} bytes hidden)",
        items.len(),
        if items.len() == 1 { "" } else { "s" },
        byte_len
    );
    Some((
        ResultCard::Table {
            title: label.to_string(),
            headers,
            rows,
            sort_col: 0,
            sort_asc: true,
        },
        summary,
    ))
}

fn json_object_result_card(
    label: &str,
    map: &serde_json::Map<String, serde_json::Value>,
    byte_len: usize,
) -> Option<(ResultCard, String)> {
    let metrics: Vec<(String, String, egui::Color32)> = map
        .iter()
        .filter(|(_, v)| json_value_is_compact_scalar(v))
        .take(10)
        .map(|(k, v)| (k.clone(), format_json_scalar(v), egui::Color32::LIGHT_GRAY))
        .collect();
    if metrics.is_empty() {
        return None;
    }

    let summary = format!(
        "{label}: {} field{} shown as a result card (raw JSON {} bytes hidden)",
        metrics.len(),
        if metrics.len() == 1 { "" } else { "s" },
        byte_len
    );
    Some((
        ResultCard::Summary {
            title: label.to_string(),
            metrics,
        },
        summary,
    ))
}

fn json_value_is_compact_scalar(value: &serde_json::Value) -> bool {
    matches!(
        value,
        serde_json::Value::Null
            | serde_json::Value::Bool(_)
            | serde_json::Value::Number(_)
            | serde_json::Value::String(_)
    )
}

fn format_json_scalar(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "—".to_string(),
        serde_json::Value::Bool(v) => v.to_string(),
        serde_json::Value::Number(v) => v.to_string(),
        serde_json::Value::String(v) => {
            let mut s: String = v.chars().take(64).collect();
            if v.chars().count() > 64 {
                s.push('…');
            }
            s
        }
        _ => String::new(),
    }
}

const HEAVY_SYNC_PENDING_FETCH_THRESHOLD: usize = 32;
const HEAVY_SYNC_DEFERRED_CHART_THRESHOLD: usize = 4;
pub(super) fn should_auto_start_background_scope_scrape(
    _scope: EventSource,
    symbol_count: usize,
) -> bool {
    // Broad scopes are valid when the user explicitly asks for them, but
    // auto-starting a 12k-symbol SEC sweep on startup turns chart interaction
    // into molasses: the scrape pounds SQLite/EDGAR while egui is trying to
    // render and apply camera drags. Keep automatic startup scrapes bounded
    // for every scope, including Scope KRAKEN after the xStocks catalog lands.
    // Manual ALL/KRAKEN remains a real full-universe scrape via the separate
    // manual gate below.
    symbol_count > 0 && symbol_count <= 512
}

pub(super) fn should_start_manual_background_scope_scrape(
    scope: EventSource,
    symbol_count: usize,
    heavy_sync_in_progress: bool,
) -> bool {
    // Manual ALL remains a real full-universe scrape when the app is idle, but
    // a 12k-symbol News/SEC sweep during bar catch-up steals SQLite/network/UI
    // budget from the sync path the user is watching. Focused/small scopes are
    // still allowed so active-symbol research isn't blocked.
    symbol_count > 0
        && (!heavy_sync_in_progress || !matches!(scope, EventSource::All) || symbol_count <= 512)
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

pub(super) fn kraken_xstocks_session_status_at(
    now_utc: chrono::DateTime<chrono::Utc>,
    overnight_enabled: bool,
) -> String {
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

    // Symbols without overnight (Blue Ocean ATS) support are CLOSED during the
    // 20:00–04:00 ET overnight window — they trade pre/core/after only. Driven by
    // the catalog `overnight_trading_support` flag.
    let in_overnight_window = minute_of_day >= OVERNIGHT || minute_of_day < PRE;
    if in_overnight_window && !overnight_enabled {
        let target = if minute_of_day < PRE {
            boundary_today(PRE)
        } else {
            boundary_today(PRE) + chrono::Duration::days(1)
        };
        return format!(
            "Kraken xStocks CLOSED · opens pre-market in {}",
            format_session_countdown(target - now_et)
        );
    }

    let (session, next_label, target) = if minute_of_day < PRE {
        ("OVERNIGHT", "pre-market", boundary_today(PRE))
    } else if minute_of_day < REGULAR {
        ("PRE", "core", boundary_today(REGULAR))
    } else if minute_of_day < AFTER {
        ("CORE", "after-hours", boundary_today(AFTER))
    } else if minute_of_day < OVERNIGHT {
        // No overnight session ⇒ the next boundary is the 8 PM close, not overnight.
        let label = if weekday == chrono::Weekday::Fri || !overnight_enabled {
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

pub(super) fn kraken_xstocks_session_status_now(overnight_enabled: bool) -> String {
    kraken_xstocks_session_status_at(chrono::Utc::now(), overnight_enabled)
}

/// True during the Kraken xStocks weekend close — Friday ≥ 20:00 ET through Sunday
/// < 20:00 ET — when no pre/core/after/overnight session of any kind exists. During
/// this window live bid/ask and extended-hours quotes are stale (the market is shut),
/// so callers suppress them and skip equity snapshot work. Symbol-independent: the
/// weekend closes every xStock regardless of its overnight-trading flag. Mirrors the
/// weekend CLOSED branch of `kraken_xstocks_session_status_at`.
pub(super) fn kraken_xstocks_weekend_closed_at(now_utc: chrono::DateTime<chrono::Utc>) -> bool {
    use chrono::{Datelike, Timelike};
    let now_et =
        now_utc.naive_utc() + chrono::Duration::seconds(us_eastern_offset_seconds(now_utc));
    let minute_of_day = now_et.hour() as i64 * 60 + now_et.minute() as i64;
    const OVERNIGHT: i64 = 20 * 60; // 8:00 PM ET — Friday close / Sunday open
    match now_et.weekday() {
        chrono::Weekday::Fri => minute_of_day >= OVERNIGHT,
        chrono::Weekday::Sat => true,
        chrono::Weekday::Sun => minute_of_day < OVERNIGHT,
        _ => false,
    }
}

pub(super) fn kraken_xstocks_weekend_closed_now() -> bool {
    kraken_xstocks_weekend_closed_at(chrono::Utc::now())
}

/// Session-aware status for the regular US-equities market clock (Alpaca
/// `/v2/clock`). Unlike Kraken xStocks (24/5 with an overnight session), the
/// regular US market has four states: pre-market (4:00–9:30 ET), core/regular
/// (9:30–16:00, Alpaca `is_open`), after-hours (16:00–20:00), and CLOSED
/// (20:00–4:00 ET, weekends, holidays — there is no regular-market overnight
/// session). Alpaca's `is_open`/`next_open` give holiday and half-day accuracy;
/// the pre-market and after-hours overlays come from the fixed ET boundaries.
/// Fixes the old binary label that read "US equities CLOSED" all through
/// pre-market.
pub(super) fn us_equities_session_status_at(
    now_utc: chrono::DateTime<chrono::Utc>,
    is_open: bool,
    next_open: Option<chrono::DateTime<chrono::Utc>>,
    next_close: Option<chrono::DateTime<chrono::Utc>>,
) -> String {
    use chrono::{Datelike, Timelike};

    let now_et =
        now_utc.naive_utc() + chrono::Duration::seconds(us_eastern_offset_seconds(now_utc));
    let weekday = now_et.weekday();
    let minute_of_day = now_et.hour() as i64 * 60 + now_et.minute() as i64;
    const PRE: i64 = 4 * 60;
    const CORE: i64 = 9 * 60 + 30;
    const AFTER: i64 = 16 * 60;
    const CLOSE: i64 = 20 * 60;
    let day_start = now_et.date().and_hms_opt(0, 0, 0).unwrap_or(now_et);
    let et_date_of = |dt: chrono::DateTime<chrono::Utc>| {
        (dt.naive_utc() + chrono::Duration::seconds(us_eastern_offset_seconds(dt))).date()
    };

    // Core hours are authoritative from Alpaca's clock (covers holidays and
    // early-close half-days that fixed ET boundaries would miss).
    if is_open {
        let target = next_close
            .map(|nc| nc - now_utc)
            .unwrap_or_else(|| (day_start + chrono::Duration::minutes(AFTER)) - now_et);
        return format!(
            "US equities OPEN · closes in {}",
            format_session_countdown(target)
        );
    }

    // A regular trading day still has its core open ahead ⇒ Alpaca's next_open is
    // on today's ET date. This separates a normal weekday from weekends/holidays
    // without shipping a local holiday table.
    let core_opens_today = next_open.map_or(false, |o| et_date_of(o) == now_et.date());

    if core_opens_today && (PRE..CORE).contains(&minute_of_day) {
        let target = next_open
            .map(|o| o - now_utc)
            .unwrap_or_else(|| (day_start + chrono::Duration::minutes(CORE)) - now_et);
        return format!(
            "US equities PRE-MARKET · Core in {}",
            format_session_countdown(target)
        );
    }

    let is_weekday = !matches!(weekday, chrono::Weekday::Sat | chrono::Weekday::Sun);
    if is_weekday && !core_opens_today && (AFTER..CLOSE).contains(&minute_of_day) {
        let target = (day_start + chrono::Duration::minutes(CLOSE)) - now_et;
        return format!(
            "US equities AFTER-HOURS · closes in {}",
            format_session_countdown(target)
        );
    }

    // Closed: overnight (20:00–04:00 ET), weekends, holidays. Count down to the
    // next regular session — pre-market (4:00 ET) on Alpaca's next trading day.
    let target = match next_open {
        Some(o) => {
            let o_et = o.naive_utc() + chrono::Duration::seconds(us_eastern_offset_seconds(o));
            let pre_et = o_et.date().and_hms_opt(4, 0, 0).unwrap_or(o_et);
            pre_et - now_et
        }
        None => (day_start + chrono::Duration::minutes(PRE) + chrono::Duration::days(1)) - now_et,
    };
    format!(
        "US equities CLOSED · opens in {}",
        format_session_countdown(target)
    )
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
        BrokerMsg::KrakenBookQuoteTick { .. } => "KrakenBookQuoteTick",
        BrokerMsg::KrakenWsBarsCommitted { .. } => "KrakenWsBarsCommitted",
        BrokerMsg::KrakenWsOhlcStatus { .. } => "KrakenWsOhlcStatus",
        BrokerMsg::KrakenEquityQuote(_) => "KrakenEquityQuote",
        BrokerMsg::KrakenEquityBars { .. } => "KrakenEquityBars",
        BrokerMsg::KrakenEquityHistoryError { .. } => "KrakenEquityHistoryError",
        BrokerMsg::KrakenEquityUniverse(_) => "KrakenEquityUniverse",
        BrokerMsg::SecScrapeResult(_) => "SecScrapeResult",
        BrokerMsg::FilingContent(_) => "FilingContent",
        BrokerMsg::FinnhubNewsResult(_) => "FinnhubNewsResult",
        BrokerMsg::NewsDbTotal(_) => "NewsDbTotal",
        BrokerMsg::Quote(_, _, _, _) => "Quote",
        BrokerMsg::MarketClock(_) => "MarketClock",
        BrokerMsg::JsonResult(_, _) => "JsonResult",
        BrokerMsg::FundamentalsProgress(_) => "FundamentalsProgress",
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
        BrokerMsg::AlpacaRateLimitObserved { .. } => "AlpacaRateLimitObserved",
        BrokerMsg::SymbolSuggestions(_) => "SymbolSuggestions",
        BrokerMsg::WatchlistQuotes(_) => "WatchlistQuotes",
        BrokerMsg::FredData(_, _) => "FredData",
        BrokerMsg::EconCalendarData(_) => "EconCalendarData",
        BrokerMsg::CongressData(_) => "CongressData",
        BrokerMsg::UnusualVolumeResults(_) => "UnusualVolumeResults",
        BrokerMsg::KrakenPositions(_) => "KrakenPositions",
        BrokerMsg::AllAssets(_) => "AllAssets",
        BrokerMsg::RecentFills(_) => "RecentFills",
        BrokerMsg::BarsSynced(_) => "BarsSynced",
        BrokerMsg::CryptoTop50(_) => "CryptoTop50",
        BrokerMsg::KrakenBalances(_) => "KrakenBalances",
        BrokerMsg::KrakenPairs(_) => "KrakenPairs",
        BrokerMsg::KrakenFuturesInstruments(_) => "KrakenFuturesInstruments",
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
    pub(super) fn drop_bg_snapshot_off_ui(&self, data: BgData) {
        // BgData can own hundreds of thousands of SEC/news/storage rows.
        // Dropping it on the egui thread was enough to create 300ms-15s stalls
        // even when we intentionally skipped applying the snapshot. Move the
        // destructor work to a blocking worker; the update hot path only moves
        // the Vec/HashMap headers.
        self.rt_handle.spawn_blocking(move || drop(data));
    }

    #[inline]
    pub(super) fn replace_bg_snapshot_off_ui_drop(&mut self, data: BgData) {
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
mod tests;
