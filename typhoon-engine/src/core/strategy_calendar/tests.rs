use super::*;

fn utc_ns(value: &str) -> i64 {
    chrono::DateTime::parse_from_rfc3339(value)
        .expect("valid timestamp")
        .timestamp_nanos_opt()
        .expect("timestamp fits")
}

#[test]
fn us_equity_sessions_are_dst_correct_and_half_open() {
    let calendar = TradingCalendar::build(&TradingCalendarSpec::us_equity_regular())
        .expect("calendar is valid");

    assert!(!calendar.is_open_at_ns(utc_ns("2026-03-09T13:29:59Z")));
    assert!(calendar.is_open_at_ns(utc_ns("2026-03-09T13:30:00Z")));
    assert!(!calendar.is_open_at_ns(utc_ns("2026-03-09T20:00:00Z")));
    assert!(!calendar.is_open_at_ns(utc_ns("2026-01-05T14:29:59Z")));
    assert!(calendar.is_open_at_ns(utc_ns("2026-01-05T14:30:00Z")));
}

#[test]
fn calendar_identity_is_content_addressed_and_tamper_evident() {
    let first = TradingCalendar::build(&TradingCalendarSpec::us_equity_regular()).unwrap();
    let second = TradingCalendar::build(&TradingCalendarSpec::us_equity_regular()).unwrap();
    assert_eq!(first.calendar_id(), second.calendar_id());

    let mut wire = serde_json::to_value(&first).unwrap();
    wire["calendar_id"] = serde_json::Value::String("cal.v1.local_windows:0000000000000000".into());
    assert!(serde_json::from_value::<TradingCalendar>(wire).is_err());
}
