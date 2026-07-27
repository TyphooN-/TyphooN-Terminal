use super::*;
use std::collections::BTreeSet;

// ── Helpers ────────────────────────────────────────────────────────

fn provenance() -> DatasetProvenance {
    DatasetProvenance {
        source: "alpaca".to_string(),
        venue: "IEX".to_string(),
        pipeline: "cache-merge/v1".to_string(),
    }
}

fn input() -> DatasetManifestInput {
    DatasetManifestInput {
        symbol: "AAPL".to_string(),
        timeframe: "1Day".to_string(),
        provenance: provenance(),
        adjustment: AdjustmentPolicy::SplitAdjusted,
        calendar: CalendarPolicy::WeekdaysOnly,
    }
}

fn bar(ts: &str, open: f64, high: f64, low: f64, close: f64, volume: f64) -> Bar {
    Bar {
        timestamp: ts.to_string(),
        open,
        high,
        low,
        close,
        volume,
    }
}

/// Three well-formed consecutive weekday bars: Mon/Tue/Wed 2024-01-01..03.
fn weekday_bars() -> Vec<Bar> {
    vec![
        bar("2024-01-01T00:00:00Z", 10.0, 11.0, 9.5, 10.5, 1_000.0),
        bar("2024-01-02T00:00:00Z", 10.5, 12.0, 10.0, 11.0, 1_100.0),
        bar("2024-01-03T00:00:00Z", 11.0, 11.5, 10.5, 10.75, 900.0),
    ]
}

fn id_of(input: &DatasetManifestInput, bars: &[Bar]) -> String {
    DatasetManifest::build(input, bars)
        .expect("manifest builds")
        .dataset_id
}

// ── Identity: determinism & shape ──────────────────────────────────

#[test]
fn dataset_id_is_repeatable_and_lowercase_sha256() {
    let bars = weekday_bars();
    let first = DatasetManifest::build(&input(), &bars).expect("manifest builds");
    let second = DatasetManifest::build(&input(), &bars).expect("manifest builds");

    assert_eq!(first.dataset_id, second.dataset_id);
    assert_eq!(first, second);
    assert_eq!(first.dataset_id.len(), 64);
    assert!(
        first
            .dataset_id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "dataset_id must be lowercase hex: {}",
        first.dataset_id
    );

    assert_eq!(first.schema_version, DATASET_MANIFEST_SCHEMA_VERSION);
    assert_eq!(first.bar_count, 3);
    assert_eq!(
        first.first_timestamp.as_deref(),
        Some("2024-01-01T00:00:00Z")
    );
    assert_eq!(
        first.last_timestamp.as_deref(),
        Some("2024-01-03T00:00:00Z")
    );
    assert_eq!(
        first.calendar_policy_id,
        CalendarPolicy::WeekdaysOnly.policy_id()
    );
}

#[test]
fn empty_dataset_manifest_has_no_range_but_still_hashes() {
    let manifest = DatasetManifest::build(&input(), &[]).expect("manifest builds");
    assert_eq!(manifest.bar_count, 0);
    assert_eq!(manifest.first_timestamp, None);
    assert_eq!(manifest.last_timestamp, None);
    assert_eq!(manifest.dataset_id.len(), 64);
    // An empty dataset must not collide with a populated one.
    assert_ne!(manifest.dataset_id, id_of(&input(), &weekday_bars()));
}

// ── Identity: field sensitivity ────────────────────────────────────

#[test]
fn dataset_id_changes_when_any_single_metadata_field_changes() {
    let bars = weekday_bars();
    let mut ids = BTreeSet::new();
    ids.insert(id_of(&input(), &bars));

    let mut variants: Vec<(&str, DatasetManifestInput)> = Vec::new();
    let mut v = input();
    v.symbol = "MSFT".to_string();
    variants.push(("symbol", v));
    let mut v = input();
    v.timeframe = "1Hour".to_string();
    variants.push(("timeframe", v));
    let mut v = input();
    v.adjustment = AdjustmentPolicy::Raw;
    variants.push(("adjustment", v));
    let mut v = input();
    v.calendar = CalendarPolicy::Continuous24x7;
    variants.push(("calendar", v));

    for (field, variant) in &variants {
        assert!(
            ids.insert(id_of(variant, &bars)),
            "changing {field} did not change the dataset id"
        );
    }
    assert_eq!(ids.len(), variants.len() + 1);
}

#[test]
fn dataset_id_changes_when_any_provenance_field_changes() {
    let bars = weekday_bars();
    let mut ids = BTreeSet::new();
    ids.insert(id_of(&input(), &bars));

    let mut variants: Vec<(&str, DatasetManifestInput)> = Vec::new();
    let mut v = input();
    v.provenance.source = "kraken".to_string();
    variants.push(("source", v));
    let mut v = input();
    v.provenance.venue = "XNAS".to_string();
    variants.push(("venue", v));
    let mut v = input();
    v.provenance.pipeline = "cache-merge/v2".to_string();
    variants.push(("pipeline", v));

    for (field, variant) in &variants {
        assert!(
            ids.insert(id_of(variant, &bars)),
            "changing provenance.{field} did not change the dataset id"
        );
    }
    assert_eq!(ids.len(), variants.len() + 1);
}

#[test]
fn dataset_id_changes_when_bar_order_changes() {
    let bars = weekday_bars();
    let mut swapped = bars.clone();
    swapped.swap(0, 1);

    assert_ne!(id_of(&input(), &bars), id_of(&input(), &swapped));
    // Reversal is also a distinct dataset.
    let mut reversed = bars.clone();
    reversed.reverse();
    assert_ne!(id_of(&input(), &bars), id_of(&input(), &reversed));
}

#[test]
fn dataset_id_changes_when_any_single_bar_field_changes() {
    let bars = weekday_bars();
    let mut ids = BTreeSet::new();
    ids.insert(id_of(&input(), &bars));

    let mutations: Vec<(&str, fn(&mut Bar))> = vec![
        ("timestamp", |b: &mut Bar| {
            b.timestamp = "2024-01-04T00:00:00Z".to_string()
        }),
        ("open", |b: &mut Bar| b.open = 10.000_000_1),
        ("high", |b: &mut Bar| b.high = 12.000_000_1),
        ("low", |b: &mut Bar| b.low = 9.999_999_9),
        ("close", |b: &mut Bar| b.close = 11.000_000_1),
        ("volume", |b: &mut Bar| b.volume = 1_100.000_1),
    ];

    for (field, mutate) in mutations {
        let mut mutated = bars.clone();
        mutate(&mut mutated[1]);
        assert!(
            ids.insert(id_of(&input(), &mutated)),
            "changing bar.{field} did not change the dataset id"
        );
    }
}

#[test]
fn framing_prevents_adjacent_field_collisions() {
    let bars = weekday_bars();
    let mut left = input();
    left.symbol = "AB".to_string();
    left.timeframe = "C".to_string();
    let mut right = input();
    right.symbol = "A".to_string();
    right.timeframe = "BC".to_string();

    assert_ne!(id_of(&left, &bars), id_of(&right, &bars));
}

// ── Identity: numeric encoding decisions ───────────────────────────

#[test]
fn negative_zero_is_normalized_to_positive_zero_in_identity() {
    // Documented normalization: -0.0 and +0.0 are numerically equal, so they
    // must not produce two different dataset ids for the same data.
    let positive = vec![bar("2024-01-01T00:00:00Z", 10.0, 11.0, 9.5, 10.5, 0.0)];
    let negative = vec![bar("2024-01-01T00:00:00Z", 10.0, 11.0, 9.5, 10.5, -0.0)];
    assert_eq!(id_of(&input(), &positive), id_of(&input(), &negative));

    // ...and -0.0 volume is not a "negative volume" QA defect either.
    let report = run_dataset_qa("1Day", CalendarPolicy::Continuous24x7, &negative);
    assert!(
        !report
            .findings
            .iter()
            .any(|f| matches!(f.issue, DatasetQaIssue::NegativeVolume { .. })),
        "-0.0 volume must not be flagged negative: {:?}",
        report.findings
    );
}

#[test]
fn nonfinite_bar_values_are_rejected_not_hashed() {
    let cases: Vec<(BarField, Bar)> = vec![
        (
            BarField::Open,
            bar("2024-01-01T00:00:00Z", f64::NAN, 11.0, 9.5, 10.5, 1.0),
        ),
        (
            BarField::High,
            bar("2024-01-01T00:00:00Z", 10.0, f64::INFINITY, 9.5, 10.5, 1.0),
        ),
        (
            BarField::Low,
            bar(
                "2024-01-01T00:00:00Z",
                10.0,
                11.0,
                f64::NEG_INFINITY,
                10.5,
                1.0,
            ),
        ),
        (
            BarField::Close,
            bar("2024-01-01T00:00:00Z", 10.0, 11.0, 9.5, f64::NAN, 1.0),
        ),
        (
            BarField::Volume,
            bar("2024-01-01T00:00:00Z", 10.0, 11.0, 9.5, 10.5, f64::INFINITY),
        ),
    ];

    for (field, defective) in cases {
        let bars = vec![weekday_bars()[0].clone(), defective];
        let err = DatasetManifest::build(&input(), &bars).expect_err("must reject nonfinite");
        match err {
            DatasetError::NonFiniteBarValue {
                index, field: got, ..
            } => {
                assert_eq!(index, 1);
                assert_eq!(got, field);
            }
            other => panic!("expected NonFiniteBarValue for {field:?}, got {other:?}"),
        }
    }
}

#[test]
fn ambiguous_metadata_strings_are_rejected() {
    let mut empty_symbol = input();
    empty_symbol.symbol = "  ".to_string();
    assert!(matches!(
        DatasetManifest::build(&empty_symbol, &weekday_bars()),
        Err(DatasetError::InvalidMetadataField {
            field: "symbol",
            reason: InvalidTextReason::Empty
        })
    ));

    let mut padded_timeframe = input();
    padded_timeframe.timeframe = "1Day ".to_string();
    assert!(matches!(
        DatasetManifest::build(&padded_timeframe, &weekday_bars()),
        Err(DatasetError::InvalidMetadataField {
            field: "timeframe",
            reason: InvalidTextReason::SurroundingWhitespace
        })
    ));

    let mut control_source = input();
    control_source.provenance.source = "alp\naca".to_string();
    assert!(matches!(
        DatasetManifest::build(&control_source, &weekday_bars()),
        Err(DatasetError::InvalidMetadataField {
            field: "provenance.source",
            reason: InvalidTextReason::ControlCharacter
        })
    ));
}

#[test]
fn ambiguous_bar_timestamps_are_rejected() {
    let bars = vec![bar("", 10.0, 11.0, 9.5, 10.5, 1.0)];
    assert!(matches!(
        DatasetManifest::build(&input(), &bars),
        Err(DatasetError::InvalidBarTimestamp {
            index: 0,
            reason: InvalidTextReason::Empty
        })
    ));

    let bars = vec![bar(" 2024-01-01T00:00:00Z", 10.0, 11.0, 9.5, 10.5, 1.0)];
    assert!(matches!(
        DatasetManifest::build(&input(), &bars),
        Err(DatasetError::InvalidBarTimestamp {
            index: 0,
            reason: InvalidTextReason::SurroundingWhitespace
        })
    ));
}

// ── Manifest round-trip & verification ─────────────────────────────

#[test]
fn manifest_json_round_trips_and_verifies_against_bars() {
    let bars = weekday_bars();
    let manifest = DatasetManifest::build(&input(), &bars).expect("manifest builds");

    let json = serde_json::to_string(&manifest).expect("serialize");
    let loaded: DatasetManifest = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(loaded, manifest);
    loaded.verify(&bars).expect("loaded manifest verifies");
    assert_eq!(
        loaded.recompute_dataset_id(&bars).expect("recompute"),
        manifest.dataset_id
    );
}

#[test]
fn verify_detects_mutated_bars() {
    let bars = weekday_bars();
    let manifest = DatasetManifest::build(&input(), &bars).expect("manifest builds");

    let mut tampered = bars.clone();
    tampered[1].close = 11.000_000_1;
    match manifest.verify(&tampered) {
        Err(DatasetError::DatasetIdMismatch { expected, actual }) => {
            assert_eq!(expected, manifest.dataset_id);
            assert_ne!(actual, manifest.dataset_id);
        }
        other => panic!("expected DatasetIdMismatch, got {other:?}"),
    }
}

#[test]
fn verify_detects_truncated_bars_and_tampered_manifest_fields() {
    let bars = weekday_bars();
    let manifest = DatasetManifest::build(&input(), &bars).expect("manifest builds");

    match manifest.verify(&bars[..2]) {
        Err(DatasetError::ManifestFieldMismatch { field, .. }) => assert_eq!(field, "bar_count"),
        other => panic!("expected ManifestFieldMismatch on bar_count, got {other:?}"),
    }

    let mut tampered = manifest.clone();
    tampered.first_timestamp = Some("2023-12-31T00:00:00Z".to_string());
    match tampered.verify(&bars) {
        Err(DatasetError::ManifestFieldMismatch { field, .. }) => {
            assert_eq!(field, "first_timestamp")
        }
        other => panic!("expected ManifestFieldMismatch on first_timestamp, got {other:?}"),
    }

    let mut forged = manifest.clone();
    forged.dataset_id = "0".repeat(64);
    assert!(matches!(
        forged.verify(&bars),
        Err(DatasetError::DatasetIdMismatch { .. })
    ));
}

#[test]
fn verify_rejects_unsupported_schema_version() {
    let bars = weekday_bars();
    let mut manifest = DatasetManifest::build(&input(), &bars).expect("manifest builds");
    manifest.schema_version = DATASET_MANIFEST_SCHEMA_VERSION + 1;

    assert!(matches!(
        manifest.verify(&bars),
        Err(DatasetError::UnsupportedSchemaVersion { .. })
    ));
}

// ── QA: structural findings ────────────────────────────────────────

#[test]
fn qa_flags_empty_dataset() {
    let report = run_dataset_qa("1Day", CalendarPolicy::WeekdaysOnly, &[]);
    assert_eq!(report.bars_checked, 0);
    assert_eq!(report.schema_version, DATASET_QA_SCHEMA_VERSION);
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].issue, DatasetQaIssue::EmptyDataset);
    assert_eq!(report.findings[0].severity, DatasetQaSeverity::Error);
    assert_eq!(report.findings[0].bar_index, None);
    assert!(report.has_errors());
}

#[test]
fn qa_flags_duplicate_and_out_of_order_timestamps() {
    let bars = vec![
        bar("2024-01-02T00:00:00Z", 10.0, 11.0, 9.5, 10.5, 100.0),
        bar("2024-01-02T00:00:00Z", 10.0, 11.0, 9.5, 10.5, 100.0),
        bar("2024-01-01T00:00:00Z", 10.0, 11.0, 9.5, 10.5, 100.0),
    ];
    let report = run_dataset_qa("1Day", CalendarPolicy::Continuous24x7, &bars);

    let duplicate = report
        .findings
        .iter()
        .find(|f| matches!(f.issue, DatasetQaIssue::DuplicateTimestamp { .. }))
        .expect("duplicate timestamp finding");
    assert_eq!(duplicate.bar_index, Some(1));
    assert_eq!(duplicate.timestamp.as_deref(), Some("2024-01-02T00:00:00Z"));
    assert_eq!(
        duplicate.issue,
        DatasetQaIssue::DuplicateTimestamp {
            previous_index: 0,
            previous_timestamp: "2024-01-02T00:00:00Z".to_string(),
        }
    );

    let disorder = report
        .findings
        .iter()
        .find(|f| matches!(f.issue, DatasetQaIssue::TimestampOutOfOrder { .. }))
        .expect("out-of-order finding");
    assert_eq!(disorder.bar_index, Some(2));
    assert_eq!(disorder.severity, DatasetQaSeverity::Error);
}

#[test]
fn qa_flags_unparsable_timestamps() {
    let bars = vec![bar("not-a-timestamp", 10.0, 11.0, 9.5, 10.5, 100.0)];
    let report = run_dataset_qa("1Day", CalendarPolicy::Continuous24x7, &bars);

    let finding = report
        .findings
        .iter()
        .find(|f| matches!(f.issue, DatasetQaIssue::UnparsableTimestamp { .. }))
        .expect("unparsable timestamp finding");
    assert_eq!(finding.bar_index, Some(0));
    assert_eq!(finding.severity, DatasetQaSeverity::Error);
}

#[test]
fn qa_flags_ohlc_violations() {
    // high below low, below open and below close all at once.
    let bars = vec![bar("2024-01-01T00:00:00Z", 10.0, 9.0, 9.5, 9.8, 100.0)];
    let report = run_dataset_qa("1Day", CalendarPolicy::Continuous24x7, &bars);

    let kinds: Vec<OhlcViolationKind> = report
        .findings
        .iter()
        .filter_map(|f| match &f.issue {
            DatasetQaIssue::OhlcViolation { kind } => Some(*kind),
            _ => None,
        })
        .collect();

    assert!(
        kinds.contains(&OhlcViolationKind::HighBelowLow),
        "{kinds:?}"
    );
    assert!(
        kinds.contains(&OhlcViolationKind::HighBelowOpen),
        "{kinds:?}"
    );
    assert!(
        kinds.contains(&OhlcViolationKind::HighBelowClose),
        "{kinds:?}"
    );
    assert!(
        report
            .findings
            .iter()
            .all(|f| f.bar_index == Some(0) || f.bar_index.is_none())
    );

    // A well-formed bar produces no OHLC findings.
    let clean = run_dataset_qa("1Day", CalendarPolicy::Continuous24x7, &weekday_bars());
    assert!(
        !clean
            .findings
            .iter()
            .any(|f| matches!(f.issue, DatasetQaIssue::OhlcViolation { .. })),
        "{:?}",
        clean.findings
    );
}

#[test]
fn qa_flags_nonpositive_prices_and_invalid_volume() {
    let bars = vec![
        bar("2024-01-01T00:00:00Z", 10.0, 11.0, 9.0, 0.0, 100.0),
        bar("2024-01-02T00:00:00Z", 10.0, 11.0, -1.0, 10.0, 100.0),
        bar("2024-01-03T00:00:00Z", 10.0, 11.0, 9.0, 10.0, -5.0),
        bar("2024-01-04T00:00:00Z", f64::NAN, 11.0, 9.0, 10.0, f64::NAN),
    ];
    let report = run_dataset_qa("1Day", CalendarPolicy::Continuous24x7, &bars);

    let nonpositive: Vec<(usize, BarField)> = report
        .findings
        .iter()
        .filter_map(|f| match &f.issue {
            DatasetQaIssue::NonPositivePrice { field, .. } => Some((f.bar_index.unwrap(), *field)),
            _ => None,
        })
        .collect();
    assert!(
        nonpositive.contains(&(0, BarField::Close)),
        "{nonpositive:?}"
    );
    assert!(nonpositive.contains(&(1, BarField::Low)), "{nonpositive:?}");

    let negative_volume = report
        .findings
        .iter()
        .find(|f| matches!(f.issue, DatasetQaIssue::NegativeVolume { .. }))
        .expect("negative volume finding");
    assert_eq!(negative_volume.bar_index, Some(2));

    let nonfinite: Vec<(usize, BarField)> = report
        .findings
        .iter()
        .filter_map(|f| match &f.issue {
            DatasetQaIssue::NonFiniteValue { field, .. } => Some((f.bar_index.unwrap(), *field)),
            _ => None,
        })
        .collect();
    assert!(nonfinite.contains(&(3, BarField::Open)), "{nonfinite:?}");
    assert!(nonfinite.contains(&(3, BarField::Volume)), "{nonfinite:?}");
    assert!(report.has_errors());
}

// ── QA: calendar policy ────────────────────────────────────────────

#[test]
fn qa_flags_weekend_bars_under_weekdays_only_policy() {
    let bars = vec![
        bar("2024-01-05T00:00:00Z", 10.0, 11.0, 9.5, 10.5, 100.0), // Friday
        bar("2024-01-06T00:00:00Z", 10.0, 11.0, 9.5, 10.5, 100.0), // Saturday
        bar("2024-01-07T00:00:00Z", 10.0, 11.0, 9.5, 10.5, 100.0), // Sunday
        bar("2024-01-08T00:00:00Z", 10.0, 11.0, 9.5, 10.5, 100.0), // Monday
    ];
    let report = run_dataset_qa("1Day", CalendarPolicy::WeekdaysOnly, &bars);

    let weekend: Vec<usize> = report
        .findings
        .iter()
        .filter(|f| matches!(f.issue, DatasetQaIssue::UnexpectedWeekendBar { .. }))
        .map(|f| f.bar_index.unwrap())
        .collect();
    assert_eq!(weekend, vec![1, 2]);
    assert!(
        report
            .findings
            .iter()
            .filter(|f| matches!(f.issue, DatasetQaIssue::UnexpectedWeekendBar { .. }))
            .all(|f| f.severity == DatasetQaSeverity::Warning)
    );
    assert_eq!(
        report.calendar_policy_id,
        CalendarPolicy::WeekdaysOnly.policy_id()
    );
}

#[test]
fn qa_accepts_weekend_bars_under_continuous_policy() {
    let bars = vec![
        bar("2024-01-05T00:00:00Z", 10.0, 11.0, 9.5, 10.5, 100.0), // Friday
        bar("2024-01-06T00:00:00Z", 10.0, 11.0, 9.5, 10.5, 100.0), // Saturday
        bar("2024-01-07T00:00:00Z", 10.0, 11.0, 9.5, 10.5, 100.0), // Sunday
    ];
    let report = run_dataset_qa("1Day", CalendarPolicy::Continuous24x7, &bars);

    assert!(
        report.findings.is_empty(),
        "24x7 weekend bars are valid: {:?}",
        report.findings
    );
    assert!(!report.has_errors());
}

// ── QA: gap detection honesty ──────────────────────────────────────

#[test]
fn qa_gap_detection_skips_weekend_slots_under_weekdays_only() {
    let bars = vec![
        bar("2024-01-01T00:00:00Z", 10.0, 11.0, 9.5, 10.5, 100.0), // Monday
        bar("2024-01-08T00:00:00Z", 10.0, 11.0, 9.5, 10.5, 100.0), // next Monday
    ];

    let weekdays = run_dataset_qa("1Day", CalendarPolicy::WeekdaysOnly, &bars);
    assert_eq!(
        weekdays.gap_detection,
        GapDetectionStatus::Enabled {
            step_seconds: 86_400
        }
    );
    let gap = weekdays
        .findings
        .iter()
        .find(|f| matches!(f.issue, DatasetQaIssue::MissingBars { .. }))
        .expect("gap finding");
    assert_eq!(gap.bar_index, Some(1));
    assert_eq!(
        gap.issue,
        DatasetQaIssue::MissingBars {
            expected_next: "2024-01-02T00:00:00Z".to_string(),
            missing_slots: 4, // Tue..Fri; Sat/Sun are not expected
            scan_truncated: false,
        }
    );

    // The same bars under a 24x7 calendar are missing all six slots.
    let continuous = run_dataset_qa("1Day", CalendarPolicy::Continuous24x7, &bars);
    let gap = continuous
        .findings
        .iter()
        .find(|f| matches!(f.issue, DatasetQaIssue::MissingBars { .. }))
        .expect("gap finding");
    assert_eq!(
        gap.issue,
        DatasetQaIssue::MissingBars {
            expected_next: "2024-01-02T00:00:00Z".to_string(),
            missing_slots: 6,
            scan_truncated: false,
        }
    );
}

#[test]
fn qa_reports_unsupported_timeframe_instead_of_guessing_gaps() {
    let bars = weekday_bars();

    let unknown = run_dataset_qa("fortnightly", CalendarPolicy::Continuous24x7, &bars);
    assert_eq!(
        unknown.gap_detection,
        GapDetectionStatus::UnsupportedTimeframe {
            timeframe: "fortnightly".to_string()
        }
    );
    assert!(
        !unknown
            .findings
            .iter()
            .any(|f| matches!(f.issue, DatasetQaIssue::MissingBars { .. }))
    );

    let monthly = run_dataset_qa("1Month", CalendarPolicy::Continuous24x7, &bars);
    assert_eq!(
        monthly.gap_detection,
        GapDetectionStatus::VariableLengthTimeframe {
            timeframe: "1Month".to_string()
        }
    );

    // Intraday under a weekday-only calendar needs a session table we do not
    // have — say so instead of flagging every overnight close as a gap.
    let intraday = run_dataset_qa("15Min", CalendarPolicy::WeekdaysOnly, &bars);
    assert_eq!(
        intraday.gap_detection,
        GapDetectionStatus::UnsupportedForCalendar {
            timeframe: "15Min".to_string(),
            calendar_policy_id: CalendarPolicy::WeekdaysOnly.policy_id().to_string(),
        }
    );
    assert!(
        !intraday
            .findings
            .iter()
            .any(|f| matches!(f.issue, DatasetQaIssue::MissingBars { .. }))
    );

    assert_eq!(
        run_dataset_qa("15Min", CalendarPolicy::Continuous24x7, &bars).gap_detection,
        GapDetectionStatus::Enabled { step_seconds: 900 }
    );
}

// ── QA: determinism & serde ────────────────────────────────────────

#[test]
fn qa_report_is_deterministic_and_round_trips() {
    let bars = vec![
        bar("2024-01-06T00:00:00Z", 10.0, 9.0, 9.5, 0.0, -1.0), // Saturday, broken
        bar("2024-01-06T00:00:00Z", 10.0, 11.0, 9.5, 10.5, 100.0), // duplicate
    ];
    let first = run_dataset_qa("1Day", CalendarPolicy::WeekdaysOnly, &bars);
    let second = run_dataset_qa("1Day", CalendarPolicy::WeekdaysOnly, &bars);
    assert_eq!(first, second);

    let json = serde_json::to_string(&first).expect("serialize");
    let loaded: DatasetQaReport = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(loaded, first);

    // Findings are ordered by bar index.
    let indices: Vec<Option<usize>> = first.findings.iter().map(|f| f.bar_index).collect();
    let mut sorted = indices.clone();
    sorted.sort();
    assert_eq!(indices, sorted, "{:?}", first.findings);
}

#[test]
fn manifest_runs_qa_with_its_own_recorded_policy() {
    let bars = vec![bar("2024-01-06T00:00:00Z", 10.0, 11.0, 9.5, 10.5, 100.0)]; // Saturday
    let manifest = DatasetManifest::build(&input(), &bars).expect("manifest builds");

    let report = manifest.run_qa(&bars);
    assert_eq!(report.calendar_policy_id, manifest.calendar_policy_id);
    assert_eq!(report.timeframe, manifest.timeframe);
    assert!(
        report
            .findings
            .iter()
            .any(|f| matches!(f.issue, DatasetQaIssue::UnexpectedWeekendBar { .. }))
    );
}
