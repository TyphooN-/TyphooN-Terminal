//! Market-session status helpers shared by broker runtime and UI surfaces.

fn format_session_countdown(duration: chrono::Duration) -> String {
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

fn nth_sunday(year: i32, month: u32, nth: u32) -> Option<chrono::NaiveDate> {
    use chrono::Datelike;
    let first = chrono::NaiveDate::from_ymd_opt(year, month, 1)?;
    let days_to_sunday = (7 - first.weekday().num_days_from_sunday()) % 7;
    first.checked_add_signed(chrono::Duration::days(
        (days_to_sunday + (nth.saturating_sub(1)) * 7) as i64,
    ))
}

fn us_eastern_offset_seconds(now_utc: chrono::DateTime<chrono::Utc>) -> i64 {
    use chrono::Datelike;
    let year = now_utc.date_naive().year();
    // US Eastern daylight time starts at 02:00 local / 07:00 UTC on the second
    // Sunday in March and ends at 02:00 local / 06:00 UTC on the first Sunday
    // in November.
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

/// Coarse, clock-free check for whether the US-equities *extended* session
/// (pre-market 04:00 ET through after-hours 20:00 ET, Mon–Fri) could be live.
/// Used to back off transient-data refreshers (trading halts / LULD) on weekends
/// and overnight, where no new halt can post. Holiday-blind by design — a holiday
/// just costs one extra coarse refresh — so it needs no broker clock and is safe
/// to call from background threads.
pub fn us_equities_extended_session_possible(now_utc: chrono::DateTime<chrono::Utc>) -> bool {
    use chrono::{Datelike, Timelike, Weekday};
    let et = now_utc.naive_utc() + chrono::Duration::seconds(us_eastern_offset_seconds(now_utc));
    if matches!(et.weekday(), Weekday::Sat | Weekday::Sun) {
        return false;
    }
    let minute_of_day = et.hour() as i64 * 60 + et.minute() as i64;
    (4 * 60..20 * 60).contains(&minute_of_day)
}

/// Session-aware status for the regular US-equities market clock (Alpaca
/// `/v2/clock`). Unlike Kraken xStocks (24/5 with an overnight session), the
/// regular US market has four states: pre-market (4:00–9:30 ET), core/regular
/// (9:30–16:00, Alpaca `is_open`), after-hours (16:00–20:00), and CLOSED
/// (20:00–4:00 ET, weekends, holidays — there is no regular-market overnight
/// session). Alpaca's `is_open`/`next_open` give holiday and half-day accuracy;
/// the pre-market and after-hours overlays come from the fixed ET boundaries.
pub fn us_equities_session_status_at(
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
    let core_opens_today = next_open.is_some_and(|o| et_date_of(o) == now_et.date());

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

    let target = match next_open {
        Some(o) => {
            // Closed state opens at 04:00 ET pre-market on the next valid trading
            // day, not at Alpaca's 09:30 core open.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn at(ts: &str) -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339(ts)
            .unwrap()
            .with_timezone(&chrono::Utc)
    }

    #[test]
    fn extended_session_possible_is_weekday_0400_to_2000_et() {
        // 2026-06-08 Mon (EDT, UTC-4). Window is 04:00–20:00 ET.
        assert!(!us_equities_extended_session_possible(at("2026-06-08T07:59:00Z"))); // 03:59 ET
        assert!(us_equities_extended_session_possible(at("2026-06-08T08:00:00Z"))); // 04:00 ET
        assert!(us_equities_extended_session_possible(at("2026-06-08T17:00:00Z"))); // 13:00 ET
        assert!(!us_equities_extended_session_possible(at("2026-06-09T00:00:00Z"))); // 20:00 ET
        // Saturday/Sunday are always closed regardless of hour.
        assert!(!us_equities_extended_session_possible(at("2026-06-06T17:00:00Z"))); // Sat 13:00 ET
        assert!(!us_equities_extended_session_possible(at("2026-06-07T17:00:00Z"))); // Sun 13:00 ET
    }

    #[test]
    fn us_equities_session_status_labels_pre_core_after_and_closed() {
        // June 2026 is EDT (UTC-4); 2026-06-08 is a Monday. Alpaca next_open/
        // next_close are RFC3339 with the ET offset.
        let open_today = Some(at("2026-06-08T13:30:00Z")); // 09:30 ET Mon
        let close_today = Some(at("2026-06-08T20:00:00Z")); // 16:00 ET Mon
        let open_tomorrow = Some(at("2026-06-09T13:30:00Z")); // 09:30 ET Tue

        // 08:35 ET Monday — pre-market, core opens in 55m.
        assert_eq!(
            us_equities_session_status_at(
                at("2026-06-08T12:35:00Z"),
                false,
                open_today,
                close_today
            ),
            "US equities PRE-MARKET · Core in 55m"
        );
        // 11:00 ET Monday — core open, closes at 16:00 (5h).
        assert_eq!(
            us_equities_session_status_at(
                at("2026-06-08T15:00:00Z"),
                true,
                open_tomorrow,
                close_today
            ),
            "US equities OPEN · closes in 5h 0m"
        );
        // 17:00 ET Monday — after-hours, closes (8 PM) in 3h.
        assert_eq!(
            us_equities_session_status_at(at("2026-06-08T21:00:00Z"), false, open_tomorrow, None),
            "US equities AFTER-HOURS · closes in 3h 0m"
        );
        // 22:00 ET Monday — overnight = CLOSED for the regular market, reopens
        // pre-market 04:00 Tue (6h).
        assert!(
            us_equities_session_status_at(at("2026-06-09T02:00:00Z"), false, open_tomorrow, None)
                .starts_with("US equities CLOSED · opens in 6h")
        );
    }

    #[test]
    fn us_equities_session_status_closed_on_weekend_and_holiday() {
        // Saturday noon ET — closed until Monday's pre-market; next_open is Monday.
        let saturday = us_equities_session_status_at(
            at("2026-06-06T16:00:00Z"),
            false,
            Some(at("2026-06-08T13:30:00Z")),
            None,
        );
        assert!(
            saturday.starts_with("US equities CLOSED · opens in 1d"),
            "got {saturday}"
        );

        // Holiday at noon ET (is_open=false, next_open is a *later* day) must read
        // CLOSED, not PRE-MARKET or AFTER-HOURS — the trading-day gate comes from
        // Alpaca's next_open, not a local clock.
        let holiday_noon = us_equities_session_status_at(
            at("2026-06-08T16:00:00Z"),
            false,
            Some(at("2026-06-09T13:30:00Z")),
            None,
        );
        assert!(
            holiday_noon.starts_with("US equities CLOSED"),
            "got {holiday_noon}"
        );
    }
}
