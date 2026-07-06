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

/// The US-Eastern calendar date for a UTC instant (DST-aware). Shared by the
/// session calculators and the SSR state machine so "today" always means the
/// same trading date.
pub fn us_eastern_date(now_utc: chrono::DateTime<chrono::Utc>) -> chrono::NaiveDate {
    (now_utc.naive_utc() + chrono::Duration::seconds(us_eastern_offset_seconds(now_utc))).date()
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

/// Easter Sunday (Gregorian) via the anonymous computus algorithm. Needed for
/// Good Friday, the one US market holiday that is not a fixed date or an
/// nth-weekday rule.
fn easter_sunday(year: i32) -> Option<chrono::NaiveDate> {
    let a = year % 19;
    let b = year / 100;
    let c = year % 100;
    let d = b / 4;
    let e = b % 4;
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let i = c / 4;
    let k = c % 4;
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    let month = (h + l - 7 * m + 114) / 31;
    let day = ((h + l - 7 * m + 114) % 31) + 1;
    chrono::NaiveDate::from_ymd_opt(year, month as u32, day as u32)
}

fn nth_weekday_of_month(
    year: i32,
    month: u32,
    weekday: chrono::Weekday,
    nth: u32,
) -> Option<chrono::NaiveDate> {
    use chrono::Datelike;
    let first = chrono::NaiveDate::from_ymd_opt(year, month, 1)?;
    let offset = (7 + weekday.num_days_from_monday() as i64
        - first.weekday().num_days_from_monday() as i64)
        % 7;
    first.checked_add_signed(chrono::Duration::days(offset + (nth as i64 - 1) * 7))
}

fn last_weekday_of_month(
    year: i32,
    month: u32,
    weekday: chrono::Weekday,
) -> Option<chrono::NaiveDate> {
    use chrono::Datelike;
    let last_day = if month == 12 {
        chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1)?
    } else {
        chrono::NaiveDate::from_ymd_opt(year, month + 1, 1)?
    }
    .pred_opt()?;
    let offset = (7 + last_day.weekday().num_days_from_monday() as i64
        - weekday.num_days_from_monday() as i64)
        % 7;
    last_day.checked_sub_signed(chrono::Duration::days(offset))
}

/// NYSE/NASDAQ full-day market holiday for the given ET calendar date, with the
/// exchange observation rule for fixed-date holidays (Saturday → the Friday
/// before, Sunday → the Monday after). Rule-based rather than a year table so
/// it needs no annual maintenance (ADR-110 interim holiday awareness for the
/// xStocks session calculator; the US-equities calculator stays driven by
/// Alpaca's authoritative clock). Early-close half days (Jul 3, day after
/// Thanksgiving, Christmas Eve) are deliberately not modeled here.
pub fn us_market_holiday(date: chrono::NaiveDate) -> Option<&'static str> {
    use chrono::{Datelike, Weekday};
    let year = date.year();
    // Observed date for a fixed-date holiday: Sat → Fri before, Sun → Mon after.
    let observed = |m: u32, d: u32| -> Option<chrono::NaiveDate> {
        let actual = chrono::NaiveDate::from_ymd_opt(year, m, d)?;
        Some(match actual.weekday() {
            Weekday::Sat => actual.pred_opt()?,
            Weekday::Sun => actual.succ_opt()?,
            _ => actual,
        })
    };
    // A Jan 1 that falls on Saturday is observed the prior Dec 31 — check the
    // *next* year's New Year against this date too.
    if let Some(ny) = chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1) {
        if ny.weekday() == Weekday::Sat && ny.pred_opt() == Some(date) {
            return Some("New Year's Day (observed)");
        }
    }
    if observed(1, 1) == Some(date) {
        return Some("New Year's Day");
    }
    if nth_weekday_of_month(year, 1, Weekday::Mon, 3) == Some(date) {
        return Some("Martin Luther King Jr. Day");
    }
    if nth_weekday_of_month(year, 2, Weekday::Mon, 3) == Some(date) {
        return Some("Washington's Birthday");
    }
    if easter_sunday(year).and_then(|e| e.checked_sub_signed(chrono::Duration::days(2)))
        == Some(date)
    {
        return Some("Good Friday");
    }
    if last_weekday_of_month(year, 5, Weekday::Mon) == Some(date) {
        return Some("Memorial Day");
    }
    if year >= 2022 && observed(6, 19) == Some(date) {
        return Some("Juneteenth");
    }
    if observed(7, 4) == Some(date) {
        return Some("Independence Day");
    }
    if nth_weekday_of_month(year, 9, Weekday::Mon, 1) == Some(date) {
        return Some("Labor Day");
    }
    if nth_weekday_of_month(year, 11, Weekday::Thu, 4) == Some(date) {
        return Some("Thanksgiving Day");
    }
    if observed(12, 25) == Some(date) {
        return Some("Christmas Day");
    }
    None
}

/// True when the given ET calendar date is a regular US equities trading day
/// (weekday and not a full-day market holiday).
pub fn is_us_market_trading_day(date: chrono::NaiveDate) -> bool {
    use chrono::{Datelike, Weekday};
    !matches!(date.weekday(), Weekday::Sat | Weekday::Sun) && us_market_holiday(date).is_none()
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
/// `/v2/clock`). Five states: pre-market (4:00–9:30 ET), core/regular
/// (9:30–16:00, Alpaca `is_open`), after-hours (16:00–20:00), OVERNIGHT
/// (20:00–4:00 ET — the Blue Ocean ATS 24/5 session Alpaca shares with the
/// Kraken xStocks overnight session, which runs only on the nights preceding a
/// trading day, Sun–Thu → Mon–Fri), and CLOSED (weekends, holidays, and the
/// overnight gap before a non-trading day). Alpaca's `is_open`/`next_open` give
/// holiday and half-day accuracy for core hours; the pre-market, after-hours,
/// and overnight overlays come from fixed ET boundaries plus the local
/// trading-day calendar (Alpaca's clock can't distinguish overnight from
/// fully-closed — both read `is_open=false`).
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

    // Overnight (20:00–04:00 ET): Alpaca runs a 24/5 overnight session on the
    // Blue Ocean ATS — the same venue and hours the Kraken xStocks overnight
    // session tracks — on the nights preceding a trading day (Sun–Thu → Mon–Fri).
    // The evening block (20:00–24:00) leads into tomorrow; the early block
    // (00:00–04:00) leads into today. A weekend or holiday gap (the target day
    // doesn't trade) stays CLOSED, matching the Kraken path: the Blue Ocean
    // session pauses with the US venues. Synthesized from ET boundaries and the
    // local trading-day calendar because `is_open`/`next_open` can't tell
    // overnight from truly-closed (both read `is_open=false`).
    let today = now_et.date();
    let overnight_into_today = minute_of_day < PRE && is_us_market_trading_day(today);
    let overnight_into_tomorrow =
        minute_of_day >= CLOSE && today.succ_opt().is_some_and(is_us_market_trading_day);
    if overnight_into_today || overnight_into_tomorrow {
        let next_pre_market = if overnight_into_today {
            day_start + chrono::Duration::minutes(PRE)
        } else {
            day_start + chrono::Duration::minutes(PRE) + chrono::Duration::days(1)
        };
        return format!(
            "US equities OVERNIGHT · next pre-market in {}",
            format_session_countdown(next_pre_market - now_et)
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

    fn d(s: &str) -> chrono::NaiveDate {
        chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn us_market_holidays_cover_fixed_observed_and_rule_based_dates() {
        // 2026: Jul 4 is a Saturday → observed Friday Jul 3.
        assert_eq!(us_market_holiday(d("2026-07-03")), Some("Independence Day"));
        assert_eq!(us_market_holiday(d("2026-07-04")), None); // Saturday itself
        assert_eq!(
            us_market_holiday(d("2026-01-19")),
            Some("Martin Luther King Jr. Day")
        );
        assert_eq!(us_market_holiday(d("2026-02-16")), Some("Washington's Birthday"));
        assert_eq!(us_market_holiday(d("2026-04-03")), Some("Good Friday"));
        assert_eq!(us_market_holiday(d("2026-05-25")), Some("Memorial Day"));
        assert_eq!(us_market_holiday(d("2026-06-19")), Some("Juneteenth"));
        assert_eq!(us_market_holiday(d("2026-09-07")), Some("Labor Day"));
        assert_eq!(us_market_holiday(d("2026-11-26")), Some("Thanksgiving Day"));
        assert_eq!(us_market_holiday(d("2026-12-25")), Some("Christmas Day"));
        assert_eq!(us_market_holiday(d("2026-01-01")), Some("New Year's Day"));
        // Jan 1 2028 is a Saturday → observed Friday 2027-12-31.
        assert_eq!(
            us_market_holiday(d("2027-12-31")),
            Some("New Year's Day (observed)")
        );
        // Juneteenth predates observance before 2022.
        assert_eq!(us_market_holiday(d("2021-06-18")), None);
        // An ordinary Wednesday.
        assert_eq!(us_market_holiday(d("2026-06-10")), None);
    }

    #[test]
    fn trading_day_check_excludes_weekends_and_holidays() {
        assert!(is_us_market_trading_day(d("2026-06-10"))); // Wed
        assert!(!is_us_market_trading_day(d("2026-06-06"))); // Sat
        assert!(!is_us_market_trading_day(d("2026-09-07"))); // Labor Day
        assert!(!is_us_market_trading_day(d("2026-07-03"))); // observed Jul 4
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
        // 22:00 ET Monday — overnight (Blue Ocean) session, leads into Tuesday's
        // pre-market at 04:00 ET (6h out).
        assert_eq!(
            us_equities_session_status_at(at("2026-06-09T02:00:00Z"), false, open_tomorrow, None),
            "US equities OVERNIGHT · next pre-market in 6h 0m"
        );
    }

    #[test]
    fn us_equities_session_status_overnight_session_and_weekend_holiday_gaps() {
        // 2026-06-08 is Monday (EDT, UTC-4): 06-07 Sun, 06-12 Fri, 06-13 Sat.
        // The overnight branch reads the local trading-day calendar, not
        // next_open, so None is fine here.

        // Sunday 20:00 ET — the weekly reopen; overnight into Monday's 04:00
        // pre-market (8h). 06-07 20:00 EDT = 06-08 00:00 UTC.
        assert_eq!(
            us_equities_session_status_at(at("2026-06-08T00:00:00Z"), false, None, None),
            "US equities OVERNIGHT · next pre-market in 8h 0m"
        );
        // Tuesday 02:00 ET — early overnight into today's 04:00 pre-market (2h).
        assert_eq!(
            us_equities_session_status_at(at("2026-06-09T06:00:00Z"), false, None, None),
            "US equities OVERNIGHT · next pre-market in 2h 0m"
        );
        // Friday 22:00 ET — no Friday-night session (Saturday doesn't trade) ⇒
        // CLOSED, not OVERNIGHT. 06-12 22:00 EDT = 06-13 02:00 UTC.
        assert!(
            us_equities_session_status_at(at("2026-06-13T02:00:00Z"), false, None, None)
                .starts_with("US equities CLOSED")
        );
        // Saturday 02:00 ET — no Friday→Saturday session ⇒ CLOSED.
        assert!(
            us_equities_session_status_at(at("2026-06-13T06:00:00Z"), false, None, None)
                .starts_with("US equities CLOSED")
        );
        // Holiday eve: Thursday 22:00 ET before observed Independence Day
        // (Fri 2026-07-03) — the overnight session pauses with the US venues ⇒
        // CLOSED. 07-02 22:00 EDT = 07-03 02:00 UTC.
        assert!(
            us_equities_session_status_at(at("2026-07-03T02:00:00Z"), false, None, None)
                .starts_with("US equities CLOSED")
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
