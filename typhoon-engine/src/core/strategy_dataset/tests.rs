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
        qa_policy: DatasetQaPolicy::default(),
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

// ── QA: seeded-defect corpus (ADR-135 M0 gate) ─────────────────────

/// `count` consecutive UTC-midnight daily bars starting `2024-01-01`, drifting
/// +0.5 % a bar with a fixed intrabar range. Deliberately boring: every
/// relative move is identical, so the robust spike band collapses onto the
/// policy's absolute floor and any seeded defect stands out.
fn clean_daily_bars(count: usize) -> Vec<Bar> {
    let start = chrono::DateTime::from_timestamp(1_704_067_200, 0).expect("2024-01-01T00:00:00Z");
    let mut close = 100.0_f64;
    let mut bars = Vec::with_capacity(count);
    for index in 0..count {
        let open = close;
        close = open * 1.005;
        let timestamp = (start + chrono::Duration::days(index as i64))
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        bars.push(bar(
            &timestamp,
            open,
            close * 1.005,
            open * 0.995,
            close,
            1_000.0,
        ));
    }
    bars
}

fn issue_kinds(report: &DatasetQaReport) -> Vec<&DatasetQaIssue> {
    report.findings.iter().map(|f| &f.issue).collect()
}

fn has_issue(report: &DatasetQaReport, predicate: impl Fn(&DatasetQaIssue) -> bool) -> bool {
    report.findings.iter().any(|f| predicate(&f.issue))
}

fn crypto_input(timeframe: &str) -> DatasetManifestInput {
    DatasetManifestInput {
        symbol: "BTC/USD".to_string(),
        timeframe: timeframe.to_string(),
        provenance: DatasetProvenance {
            source: "kraken".to_string(),
            venue: "kraken-spot".to_string(),
            pipeline: "cache-merge/v1".to_string(),
        },
        adjustment: AdjustmentPolicy::Raw,
        calendar: CalendarPolicy::Continuous24x7,
        qa_policy: DatasetQaPolicy::default(),
    }
}

/// The M0 acceptance corpus: every seeded defect class must be detected, and
/// the undamaged control must stay clean.
#[test]
fn qa_detects_every_seeded_defect_class() {
    let policy = DatasetQaPolicy::default();
    let clean = clean_daily_bars(40);

    // Control: an undamaged 24x7 series produces no findings at all.
    let control =
        run_dataset_qa_with_policy("1Day", CalendarPolicy::Continuous24x7, &policy, &clean);
    assert!(
        control.findings.is_empty(),
        "clean corpus must be finding-free: {:?}",
        issue_kinds(&control)
    );

    // 1. Gap — a whole bar removed from the middle.
    let mut gapped = clean.clone();
    gapped.remove(20);
    let report =
        run_dataset_qa_with_policy("1Day", CalendarPolicy::Continuous24x7, &policy, &gapped);
    assert!(
        has_issue(&report, |i| matches!(i, DatasetQaIssue::MissingBars { .. })),
        "gap not detected: {:?}",
        issue_kinds(&report)
    );

    // 2. Spike — a single bad tick eight times the level, reverting after.
    let mut spiked = clean.clone();
    spiked[20].high = spiked[20].close * 8.0;
    spiked[20].close *= 8.0;
    let report =
        run_dataset_qa_with_policy("1Day", CalendarPolicy::Continuous24x7, &policy, &spiked);
    assert!(
        has_issue(&report, |i| matches!(i, DatasetQaIssue::PriceSpike { .. })),
        "spike not detected: {:?}",
        issue_kinds(&report)
    );

    // 3. Duplicate timestamp.
    let mut duplicated = clean.clone();
    duplicated[21].timestamp = duplicated[20].timestamp.clone();
    let report =
        run_dataset_qa_with_policy("1Day", CalendarPolicy::Continuous24x7, &policy, &duplicated);
    assert!(
        has_issue(&report, |i| matches!(
            i,
            DatasetQaIssue::DuplicateTimestamp { .. }
        )),
        "duplicate timestamp not detected: {:?}",
        issue_kinds(&report)
    );

    // 4. Out-of-order timestamp.
    let mut disordered = clean.clone();
    disordered.swap(20, 21);
    let report =
        run_dataset_qa_with_policy("1Day", CalendarPolicy::Continuous24x7, &policy, &disordered);
    assert!(
        has_issue(&report, |i| matches!(
            i,
            DatasetQaIssue::TimestampOutOfOrder { .. }
        )),
        "out-of-order timestamp not detected: {:?}",
        issue_kinds(&report)
    );

    // 5. Carry-forward bar — the Alpaca v=0 signature.
    let mut carried = clean.clone();
    let previous_close = carried[19].close;
    carried[20].open = previous_close;
    carried[20].high = previous_close;
    carried[20].low = previous_close;
    carried[20].close = previous_close;
    carried[20].volume = 0.0;
    let report =
        run_dataset_qa_with_policy("1Day", CalendarPolicy::Continuous24x7, &policy, &carried);
    assert!(
        has_issue(&report, |i| matches!(
            i,
            DatasetQaIssue::CarryForwardBar {
                zero_volume: true,
                ..
            }
        )),
        "carry-forward bar not detected: {:?}",
        issue_kinds(&report)
    );

    // 6. OHLC violation.
    let mut broken = clean.clone();
    broken[20].high = broken[20].low * 0.5;
    let report =
        run_dataset_qa_with_policy("1Day", CalendarPolicy::Continuous24x7, &policy, &broken);
    assert!(
        has_issue(&report, |i| matches!(
            i,
            DatasetQaIssue::OhlcViolation { .. }
        )),
        "OHLC violation not detected: {:?}",
        issue_kinds(&report)
    );

    // 7. Split-like level shift — every price halves from bar 20 onward.
    let mut split = clean.clone();
    for candidate in split.iter_mut().skip(20) {
        candidate.open /= 2.0;
        candidate.high /= 2.0;
        candidate.low /= 2.0;
        candidate.close /= 2.0;
    }
    let report =
        run_dataset_qa_with_policy("1Day", CalendarPolicy::Continuous24x7, &policy, &split);
    let shift = report
        .findings
        .iter()
        .find(|f| matches!(f.issue, DatasetQaIssue::SuspiciousLevelShift { .. }))
        .unwrap_or_else(|| panic!("level shift not detected: {:?}", issue_kinds(&report)));
    assert_eq!(shift.bar_index, Some(20));
    match &shift.issue {
        DatasetQaIssue::SuspiciousLevelShift {
            ratio_numerator,
            ratio_denominator,
            ..
        } => assert_eq!((*ratio_numerator, *ratio_denominator), (1, 2)),
        other => panic!("unexpected issue {other:?}"),
    }

    // 8. Unexpected weekend bar under a weekday-only calendar.
    let report = run_dataset_qa_with_policy(
        "1Day",
        CalendarPolicy::WeekdaysOnly,
        &policy,
        &[bar("2024-01-06T00:00:00Z", 10.0, 11.0, 9.5, 10.5, 100.0)],
    );
    assert!(
        has_issue(&report, |i| matches!(
            i,
            DatasetQaIssue::UnexpectedWeekendBar { .. }
        )),
        "weekend bar not detected: {:?}",
        issue_kinds(&report)
    );

    // 9. Unexpected US-market holiday bar.
    let report = run_dataset_qa_with_policy(
        "1Day",
        CalendarPolicy::UsEquityRegular,
        &policy,
        &[bar("2024-07-04T00:00:00Z", 10.0, 11.0, 9.5, 10.5, 100.0)],
    );
    assert!(
        has_issue(&report, |i| matches!(
            i,
            DatasetQaIssue::UnexpectedHolidayBar { .. }
        )),
        "holiday bar not detected: {:?}",
        issue_kinds(&report)
    );

    // 10. Unexpected out-of-session bar on a 24x5 xStock venue.
    let report = run_dataset_qa_with_policy(
        "1Hour",
        CalendarPolicy::XStock24x5,
        &policy,
        &[bar("2024-01-06T12:00:00Z", 10.0, 11.0, 9.5, 10.5, 100.0)],
    );
    assert!(
        has_issue(&report, |i| matches!(
            i,
            DatasetQaIssue::UnexpectedSessionBar { .. }
        )),
        "out-of-session xStock bar not detected: {:?}",
        issue_kinds(&report)
    );
}

#[test]
fn qa_does_not_flag_ordinary_volatility_as_a_spike() {
    // ±3 % alternating moves: real volatility, well under the policy floor.
    let start = chrono::DateTime::from_timestamp(1_704_067_200, 0).expect("epoch");
    let mut bars = Vec::new();
    let mut close = 100.0_f64;
    for index in 0..40 {
        let open = close;
        close = if index % 2 == 0 {
            open * 1.03
        } else {
            open / 1.03
        };
        let timestamp = (start + chrono::Duration::days(index))
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        bars.push(bar(
            &timestamp,
            open,
            open.max(close),
            open.min(close),
            close,
            1_000.0,
        ));
    }

    let report = run_dataset_qa("1Day", CalendarPolicy::Continuous24x7, &bars);
    assert!(
        !has_issue(&report, |i| matches!(i, DatasetQaIssue::PriceSpike { .. })),
        "ordinary volatility must not be a spike: {:?}",
        issue_kinds(&report)
    );
    assert!(
        !has_issue(&report, |i| matches!(
            i,
            DatasetQaIssue::SuspiciousLevelShift { .. }
        )),
        "ordinary volatility must not be a level shift: {:?}",
        issue_kinds(&report)
    );
}

#[test]
fn qa_separates_a_reverting_spike_from_a_sustained_level_shift() {
    let policy = DatasetQaPolicy::default();

    // A spike reverts on the next bar — it is not a level shift.
    let mut spiked = clean_daily_bars(40);
    spiked[20].high *= 8.0;
    spiked[20].close *= 8.0;
    let report =
        run_dataset_qa_with_policy("1Day", CalendarPolicy::Continuous24x7, &policy, &spiked);
    let spike_indices: Vec<usize> = report
        .findings
        .iter()
        .filter(|f| matches!(f.issue, DatasetQaIssue::PriceSpike { .. }))
        .filter_map(|f| f.bar_index)
        .collect();
    assert!(spike_indices.contains(&20), "{spike_indices:?}");
    assert!(
        !has_issue(&report, |i| matches!(
            i,
            DatasetQaIssue::SuspiciousLevelShift { .. }
        )),
        "a reverting spike is not a level shift: {:?}",
        issue_kinds(&report)
    );

    // A sustained halving is a level shift and is *not* double-reported as a spike.
    let mut split = clean_daily_bars(40);
    for candidate in split.iter_mut().skip(20) {
        candidate.open /= 2.0;
        candidate.high /= 2.0;
        candidate.low /= 2.0;
        candidate.close /= 2.0;
    }
    let report =
        run_dataset_qa_with_policy("1Day", CalendarPolicy::Continuous24x7, &policy, &split);
    let spikes_at_shift = report
        .findings
        .iter()
        .any(|f| f.bar_index == Some(20) && matches!(f.issue, DatasetQaIssue::PriceSpike { .. }));
    assert!(
        !spikes_at_shift,
        "level shift must not also report a spike: {:?}",
        issue_kinds(&report)
    );
}

#[test]
fn qa_reports_carry_forward_volume_evidence() {
    let mut bars = clean_daily_bars(10);
    let previous_close = bars[4].close;
    for index in [5usize, 6] {
        bars[index].open = previous_close;
        bars[index].high = previous_close;
        bars[index].low = previous_close;
        bars[index].close = previous_close;
    }
    bars[5].volume = 0.0;
    bars[6].volume = 42.0;

    let report = run_dataset_qa("1Day", CalendarPolicy::Continuous24x7, &bars);
    let carried: Vec<(usize, bool)> = report
        .findings
        .iter()
        .filter_map(|f| match &f.issue {
            DatasetQaIssue::CarryForwardBar { zero_volume, .. } => {
                Some((f.bar_index.expect("located"), *zero_volume))
            }
            _ => None,
        })
        .collect();
    assert_eq!(carried, vec![(5, true), (6, false)]);
}

#[test]
fn qa_spike_detection_reports_insufficient_samples_instead_of_guessing() {
    let report = run_dataset_qa("1Day", CalendarPolicy::Continuous24x7, &weekday_bars());
    assert!(
        matches!(
            report.spike_detection,
            SpikeDetectionStatus::InsufficientSamples { .. }
        ),
        "{:?}",
        report.spike_detection
    );
    assert!(!has_issue(&report, |i| matches!(
        i,
        DatasetQaIssue::PriceSpike { .. }
    )));

    let report = run_dataset_qa(
        "1Day",
        CalendarPolicy::Continuous24x7,
        &clean_daily_bars(40),
    );
    assert!(
        matches!(report.spike_detection, SpikeDetectionStatus::Enabled { .. }),
        "{:?}",
        report.spike_detection
    );
}

// ── QA: calendar policies ──────────────────────────────────────────

#[test]
fn calendar_policy_ids_are_unique_and_versioned() {
    let policies = [
        CalendarPolicy::Continuous24x7,
        CalendarPolicy::WeekdaysOnly,
        CalendarPolicy::UsEquityRegular,
        CalendarPolicy::XStock24x5,
    ];
    let ids: BTreeSet<&str> = policies.iter().map(|p| p.policy_id()).collect();
    assert_eq!(ids.len(), policies.len(), "{ids:?}");
    assert!(ids.iter().all(|id| id.ends_with(".v1")), "{ids:?}");
}

#[test]
fn crypto_weekend_bars_stay_valid_under_the_continuous_calendar() {
    let bars = vec![
        bar("2024-01-05T00:00:00Z", 10.0, 11.0, 9.5, 10.5, 100.0), // Friday
        bar("2024-01-06T00:00:00Z", 10.5, 11.0, 10.0, 10.6, 100.0), // Saturday
        bar("2024-01-07T00:00:00Z", 10.6, 11.0, 10.2, 10.7, 100.0), // Sunday
        bar("2024-01-08T00:00:00Z", 10.7, 11.0, 10.3, 10.8, 100.0), // Monday
    ];
    let manifest = DatasetManifest::build(&crypto_input("1Day"), &bars).expect("manifest builds");
    let report = manifest.run_qa(&bars);
    assert!(
        report.findings.is_empty(),
        "crypto weekends are valid: {:?}",
        issue_kinds(&report)
    );
}

#[test]
fn us_equity_calendar_separates_weekend_holiday_and_trading_days() {
    let bars = vec![
        bar("2024-07-03T00:00:00Z", 10.0, 11.0, 9.5, 10.5, 100.0), // Wed, trading
        bar("2024-07-04T00:00:00Z", 10.0, 11.0, 9.5, 10.5, 100.0), // Thu, Independence Day
        bar("2024-07-05T00:00:00Z", 10.0, 11.0, 9.5, 10.5, 100.0), // Fri, trading
        bar("2024-07-06T00:00:00Z", 10.0, 11.0, 9.5, 10.5, 100.0), // Sat
    ];
    let report = run_dataset_qa("1Day", CalendarPolicy::UsEquityRegular, &bars);

    let holidays: Vec<usize> = report
        .findings
        .iter()
        .filter(|f| matches!(f.issue, DatasetQaIssue::UnexpectedHolidayBar { .. }))
        .filter_map(|f| f.bar_index)
        .collect();
    assert_eq!(holidays, vec![1]);

    let weekends: Vec<usize> = report
        .findings
        .iter()
        .filter(|f| matches!(f.issue, DatasetQaIssue::UnexpectedWeekendBar { .. }))
        .filter_map(|f| f.bar_index)
        .collect();
    assert_eq!(weekends, vec![3]);

    // The holiday is not counted as a missing slot either — the calendar knows.
    assert!(
        !has_issue(&report, |i| matches!(i, DatasetQaIssue::MissingBars { .. })),
        "{:?}",
        issue_kinds(&report)
    );
}

#[test]
fn xstock_calendar_judges_intraday_bars_against_the_24x5_window() {
    // Friday 21:00 ET (after the 20:00 close) and Sunday 15:00 ET (before the
    // 20:00 open) are outside; Sunday 21:00 ET and Monday 00:00 ET are inside.
    let cases = [
        ("2024-01-06T02:00:00Z", false), // Fri 2024-01-05 21:00 ET
        ("2024-01-06T12:00:00Z", false), // Sat 07:00 ET
        ("2024-01-07T20:00:00Z", false), // Sun 15:00 ET
        ("2024-01-08T02:00:00Z", true),  // Sun 21:00 ET — session open
        ("2024-01-08T05:00:00Z", true),  // Mon 00:00 ET
        ("2024-07-04T14:00:00Z", false), // Independence Day
    ];
    for (timestamp, expected) in cases {
        let report = run_dataset_qa(
            "1Hour",
            CalendarPolicy::XStock24x5,
            &[bar(timestamp, 10.0, 11.0, 9.5, 10.5, 100.0)],
        );
        let flagged = has_issue(&report, |i| {
            matches!(
                i,
                DatasetQaIssue::UnexpectedSessionBar { .. }
                    | DatasetQaIssue::UnexpectedHolidayBar { .. }
                    | DatasetQaIssue::UnexpectedWeekendBar { .. }
            )
        });
        assert_eq!(
            !flagged,
            expected,
            "{timestamp} expected in-session={expected}: {:?}",
            issue_kinds(&report)
        );
    }
}

#[test]
fn xstock_daily_bars_are_judged_by_session_date_not_by_utc_clock_time() {
    // A UTC-midnight Monday daily bar is 19:00 ET Sunday — outside the intraday
    // window, but it is a normal daily bar for Monday's session.
    let daily = run_dataset_qa(
        "1Day",
        CalendarPolicy::XStock24x5,
        &[bar("2024-01-08T00:00:00Z", 10.0, 11.0, 9.5, 10.5, 100.0)],
    );
    assert!(
        !has_issue(&daily, |i| matches!(
            i,
            DatasetQaIssue::UnexpectedSessionBar { .. }
        )),
        "{:?}",
        issue_kinds(&daily)
    );

    // Saturday is still not a session date.
    let weekend = run_dataset_qa(
        "1Day",
        CalendarPolicy::XStock24x5,
        &[bar("2024-01-06T00:00:00Z", 10.0, 11.0, 9.5, 10.5, 100.0)],
    );
    assert!(
        has_issue(&weekend, |i| matches!(
            i,
            DatasetQaIssue::UnexpectedWeekendBar { .. }
        )),
        "{:?}",
        issue_kinds(&weekend)
    );
}

// ── QA: policy validation and bounds ───────────────────────────────

#[test]
fn qa_policy_rejects_non_finite_and_out_of_range_settings() {
    let cases: Vec<(&str, fn(&mut DatasetQaPolicy))> = vec![
        ("schema_version", |p| p.schema_version = 0),
        ("spike_band_multiple", |p| p.spike_band_multiple = f64::NAN),
        ("spike_band_multiple", |p| p.spike_band_multiple = 0.0),
        ("spike_min_relative_move", |p| {
            p.spike_min_relative_move = f64::INFINITY
        }),
        ("spike_min_relative_move", |p| {
            p.spike_min_relative_move = -1.0
        }),
        ("spike_min_samples", |p| p.spike_min_samples = 1),
        ("level_shift_tolerance", |p| p.level_shift_tolerance = 0.0),
        ("level_shift_tolerance", |p| p.level_shift_tolerance = 0.9),
        ("level_shift_max_ratio", |p| p.level_shift_max_ratio = 1),
        ("max_findings", |p| p.max_findings = 0),
    ];

    for (field, mutate) in cases {
        let mut policy = DatasetQaPolicy::default();
        mutate(&mut policy);
        match policy.validate() {
            Err(DatasetError::InvalidQaPolicy { field: got, .. }) => assert_eq!(got, field),
            other => panic!("expected InvalidQaPolicy for {field}, got {other:?}"),
        }
    }

    DatasetQaPolicy::default()
        .validate()
        .expect("the default policy is valid");
}

#[test]
fn manifest_build_rejects_an_invalid_qa_policy() {
    let mut broken = input();
    broken.qa_policy.max_findings = 0;
    assert!(matches!(
        DatasetManifest::build(&broken, &weekday_bars()),
        Err(DatasetError::InvalidQaPolicy { .. })
    ));
}

#[test]
fn qa_findings_are_capped_by_the_policy_and_report_the_omission() {
    let policy = DatasetQaPolicy {
        max_findings: 5,
        ..DatasetQaPolicy::default()
    };

    // 60 bars, every one of them an OHLC violation.
    let mut bars = clean_daily_bars(60);
    for candidate in bars.iter_mut() {
        candidate.high = candidate.low * 0.5;
    }

    let report = run_dataset_qa_with_policy("1Day", CalendarPolicy::Continuous24x7, &policy, &bars);
    assert_eq!(report.findings.len(), 5);
    assert!(report.findings_truncated);
    assert!(
        report.findings_omitted >= 55,
        "omitted {} findings",
        report.findings_omitted
    );
    // The cap must not hide the fact that the dataset is broken.
    assert!(report.has_errors());
}

// ── QA report hash & manifest seal ─────────────────────────────────

#[test]
fn qa_report_hash_is_deterministic_and_input_sensitive() {
    let bars = clean_daily_bars(40);
    let policy = DatasetQaPolicy::default();
    let base = run_dataset_qa_with_policy("1Day", CalendarPolicy::Continuous24x7, &policy, &bars);
    assert_eq!(base.report_hash(), base.report_hash());
    assert_eq!(base.report_hash().len(), 64);

    let rerun = run_dataset_qa_with_policy("1Day", CalendarPolicy::Continuous24x7, &policy, &bars);
    assert_eq!(base.report_hash(), rerun.report_hash());

    let mut hashes = BTreeSet::new();
    hashes.insert(base.report_hash());

    let mut mutated = bars.clone();
    mutated[7].high = mutated[7].low * 0.5;
    assert!(
        hashes.insert(
            run_dataset_qa_with_policy("1Day", CalendarPolicy::Continuous24x7, &policy, &mutated)
                .report_hash()
        )
    );

    assert!(
        hashes.insert(
            run_dataset_qa_with_policy("1Day", CalendarPolicy::WeekdaysOnly, &policy, &bars)
                .report_hash()
        )
    );

    let strict = DatasetQaPolicy {
        spike_band_multiple: policy.spike_band_multiple + 1.0,
        ..policy.clone()
    };
    assert!(
        hashes.insert(
            run_dataset_qa_with_policy("1Day", CalendarPolicy::Continuous24x7, &strict, &bars)
                .report_hash()
        )
    );
}

#[test]
fn dataset_id_addresses_data_while_manifest_id_seals_qa_state() {
    let bars = weekday_bars();
    let base = DatasetManifest::build(&input(), &bars).expect("manifest builds");

    let mut retuned = input();
    retuned.qa_policy.spike_band_multiple += 1.0;
    let other = DatasetManifest::build(&retuned, &bars).expect("manifest builds");

    // Same bytes, same data address...
    assert_eq!(base.dataset_id, other.dataset_id);
    // ...but the QA policy is identity-bearing for the sealed manifest.
    assert_ne!(base.manifest_id, other.manifest_id);
    assert_ne!(base.qa_policy_id, other.qa_policy_id);
    assert_eq!(base.manifest_id.len(), 64);

    // The manifest carries the QA report's hash and its headline counts.
    let report = base.run_qa(&bars);
    assert_eq!(base.qa_report_hash, report.report_hash());
    assert_eq!(base.qa_error_count, report.error_count() as u64);
    assert_eq!(base.qa_warning_count, report.warning_count() as u64);
    assert!(!base.qa_findings_truncated);
    base.verify_qa_report(&report).expect("report matches seal");
}

#[test]
fn verify_detects_tampered_qa_and_seal_fields() {
    let bars = weekday_bars();
    let manifest = DatasetManifest::build(&input(), &bars).expect("manifest builds");
    manifest.verify(&bars).expect("pristine manifest verifies");

    let mut forged = manifest.clone();
    forged.qa_error_count += 1;
    match forged.verify(&bars) {
        Err(DatasetError::ManifestFieldMismatch { field, .. }) => {
            assert_eq!(field, "qa_error_count")
        }
        other => panic!("expected qa_error_count mismatch, got {other:?}"),
    }

    let mut forged = manifest.clone();
    forged.qa_report_hash = "0".repeat(64);
    match forged.verify(&bars) {
        Err(DatasetError::ManifestFieldMismatch { field, .. }) => {
            assert_eq!(field, "qa_report_hash")
        }
        other => panic!("expected qa_report_hash mismatch, got {other:?}"),
    }

    let mut forged = manifest.clone();
    forged.manifest_id = "0".repeat(64);
    assert!(matches!(
        forged.verify(&bars),
        Err(DatasetError::ManifestIdMismatch { .. })
    ));

    // A QA report from a different dataset must not pass the seal check.
    let foreign = run_dataset_qa_with_policy(
        "1Day",
        CalendarPolicy::WeekdaysOnly,
        &DatasetQaPolicy::default(),
        &clean_daily_bars(10),
    );
    assert!(matches!(
        manifest.verify_qa_report(&foreign),
        Err(DatasetError::QaReportHashMismatch { .. })
    ));
}

#[test]
fn manifest_json_round_trip_preserves_the_seal() {
    let bars = clean_daily_bars(30);
    let manifest = DatasetManifest::build(&crypto_input("1Day"), &bars).expect("manifest builds");
    let json = serde_json::to_string(&manifest).expect("serialize");
    let loaded: DatasetManifest = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(loaded, manifest);
    loaded
        .verify(&bars)
        .expect("round-tripped manifest verifies");

    let report = manifest.run_qa(&bars);
    let qa_json = serde_json::to_string(&report).expect("serialize qa");
    let loaded_qa: DatasetQaReport = serde_json::from_str(&qa_json).expect("deserialize qa");
    assert_eq!(loaded_qa, report);
    assert_eq!(loaded_qa.report_hash(), report.report_hash());
}
