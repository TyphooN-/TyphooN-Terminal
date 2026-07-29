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

fn local_date(value: &str) -> chrono::NaiveDate {
    chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").expect("valid date")
}

fn exception(date: &str) -> CalendarException {
    CalendarException {
        local_date: local_date(date),
        source_record_id: format!("record-{date}"),
        label: format!("closure {date}"),
        kind: CalendarExceptionKind::Closed,
    }
}

/// Exceptions must reach a built calendar unique, ascending and named by the
/// artifact that sealed them. `status_at` binary-searches them, so an unsorted
/// or duplicated set would not merely be untidy — it would silently miss a
/// published closure.
#[test]
fn exception_sets_must_be_ascending_unique_and_named_by_their_artifact() {
    let artifact_id = "a".repeat(64);
    let with = |exceptions: Vec<CalendarException>, id: Option<&str>| TradingCalendarSpec {
        exceptions,
        exception_artifact_id: id.map(str::to_string),
        ..TradingCalendarSpec::us_equity_regular()
    };

    let valid = with(
        vec![exception("2024-11-29"), exception("2024-12-25")],
        Some(&artifact_id),
    );
    let calendar = TradingCalendar::build(&valid).expect("an ascending unique set builds");
    assert!(!calendar.is_open_at_ns(utc_ns("2024-12-25T15:00:00Z")));

    assert!(matches!(
        TradingCalendar::build(&with(
            vec![exception("2024-12-25"), exception("2024-11-29")],
            Some(&artifact_id),
        )),
        Err(CalendarError::ExceptionsOutOfOrder { index: 1 })
    ));
    assert!(matches!(
        TradingCalendar::build(&with(
            vec![exception("2024-12-25"), exception("2024-12-25")],
            Some(&artifact_id),
        )),
        Err(CalendarError::DuplicateExceptionDate { index: 1 })
    ));
    // Exceptions and their artifact id are present together or not at all, and
    // the id must be a real digest — otherwise a calendar could claim published
    // closures with no provenance to check them against.
    for broken in [
        with(vec![exception("2024-12-25")], None),
        with(Vec::new(), Some(&artifact_id)),
        with(vec![exception("2024-12-25")], Some("not-a-digest")),
    ] {
        assert!(matches!(
            TradingCalendar::build(&broken),
            Err(CalendarError::MissingExceptionArtifactId)
        ));
    }

    // Untrimmed, empty or control-bearing operator text never reaches an id.
    let mut unbounded = valid.clone();
    unbounded.exceptions[0].label = " padded".to_string();
    assert!(matches!(
        TradingCalendar::build(&unbounded),
        Err(CalendarError::InvalidException { index: 0 })
    ));

    // The exception set is part of what the calendar *is*, so it is sealed into
    // the id rather than carried alongside it.
    let rule_only = TradingCalendar::build(&TradingCalendarSpec::us_equity_regular()).unwrap();
    assert_ne!(calendar.calendar_id(), rule_only.calendar_id());
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
