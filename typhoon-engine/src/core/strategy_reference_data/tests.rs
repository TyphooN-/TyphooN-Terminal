use super::*;
use crate::core::strategy_calendar::{
    ClosedReason, EarlyCloseRule, ExchangeTimeZone, LocalSessionWindow, SessionStatus,
    TradingCalendarSpec,
};
use crate::core::strategy_dataset::AdjustmentPolicy;
use crate::core::strategy_ir::{
    ExecutionSettings, STRATEGY_EXECUTION_CONFIG_SCHEMA_VERSION, StrategyExecutionConfig,
};

fn date(value: &str) -> chrono::NaiveDate {
    chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").unwrap()
}

fn calendar_source(system: SourceSystem, authority: SourceAuthorityClass) -> SourceBatch {
    SourceBatch {
        source: system,
        authority,
        coverage: SourceCoverage::ExchangeDateRange {
            start: date("2024-01-01"),
            end_inclusive: date("2025-12-31"),
        },
        complete: true,
        as_of_ns: 1_799_000_000_000_000_000,
        retrieved_at_ns: 1_800_000_000_000_000_000,
        identity_metadata_policy: IdentityMetadataPolicy::AsOfIncludedRetrievalExcluded,
    }
}

fn calendar_request(records: Vec<CalendarSourceRecord>) -> CalendarMaterializationRequest {
    CalendarMaterializationRequest {
        venue: "XNYS".into(),
        time_zone: ExchangeTimeZone::UsEastern,
        range_start: date("2024-01-01"),
        range_end_inclusive: date("2025-12-31"),
        require_authoritative: true,
        source: calendar_source(
            SourceSystem::ExchangePublication {
                exchange: "NYSE".into(),
            },
            SourceAuthorityClass::ExchangeOfficial,
        ),
        base: TradingCalendarSpec::us_equity_regular(),
        records,
    }
}

fn calendar_record(
    id: &str,
    local_date: &str,
    kind: CalendarExceptionSourceKind,
) -> CalendarSourceRecord {
    let raw = format!("{{\"id\":\"{id}\"}}");
    CalendarSourceRecord {
        source_record_id: id.into(),
        raw_record_sha256: raw_source_sha256(&raw),
        venue: "XNYS".into(),
        time_zone: ExchangeTimeZone::UsEastern,
        local_date: date(local_date),
        kind,
        label: id.into(),
        raw_source: raw,
    }
}

fn action_source(system: SourceSystem, authority: SourceAuthorityClass) -> SourceBatch {
    SourceBatch {
        source: system,
        authority,
        coverage: SourceCoverage::UtcRange {
            start_ns: parse_utc_ns("2024-01-01T00:00:00Z").unwrap(),
            end_ns: parse_utc_ns("2026-01-01T00:00:00Z").unwrap(),
        },
        complete: true,
        as_of_ns: 1_799_000_000_000_000_000,
        retrieved_at_ns: 1_800_000_000_000_000_000,
        identity_metadata_policy: IdentityMetadataPolicy::AsOfIncludedRetrievalExcluded,
    }
}

fn corporate_request(
    records: Vec<CorporateActionSourceRecord>,
) -> CorporateActionMaterializationRequest {
    CorporateActionMaterializationRequest {
        venue: "XNYS".into(),
        symbol: "AAA".into(),
        time_zone: ExchangeTimeZone::UsEastern,
        currency: "USD".into(),
        range_start_ns: parse_utc_ns("2024-01-01T00:00:00Z").unwrap(),
        range_end_ns: parse_utc_ns("2026-01-01T00:00:00Z").unwrap(),
        require_authoritative: false,
        adjustment: AdjustmentPolicy::Raw,
        source: action_source(
            SourceSystem::ResearchDatabaseCache {
                upstream: Box::new(SourceSystem::YahooChartKeyless),
            },
            SourceAuthorityClass::UnverifiedPublic,
        ),
        records,
    }
}

fn action(id: &str, at: &str, kind: CorporateActionSourceKind) -> CorporateActionSourceRecord {
    let raw = format!("{{\"id\":\"{id}\"}}");
    CorporateActionSourceRecord {
        source_record_id: id.into(),
        raw_record_sha256: raw_source_sha256(&raw),
        venue: "XNYS".into(),
        symbol: "AAA".into(),
        time_zone: ExchangeTimeZone::UsEastern,
        currency: "USD".into(),
        effective_utc: at.into(),
        kind,
        raw_source: raw,
    }
}

fn split(id: &str, numerator: &str, denominator: &str) -> CorporateActionSourceRecord {
    action(
        id,
        "2024-06-10T13:30:00Z",
        CorporateActionSourceKind::Split {
            numerator: numerator.into(),
            denominator: denominator.into(),
        },
    )
}

#[test]
fn holiday_early_close_open_override_and_dst_use_exchange_local_dates() {
    let artifact = materialize_calendar(&calendar_request(vec![
        calendar_record(
            "christmas",
            "2024-12-25",
            CalendarExceptionSourceKind::Closed,
        ),
        calendar_record(
            "early-close",
            "2024-11-29",
            CalendarExceptionSourceKind::EarlyClose {
                close_minute: 13 * 60,
            },
        ),
        calendar_record(
            "sunday-open",
            "2024-03-10",
            CalendarExceptionSourceKind::OpenOverride {
                windows: vec![LocalSessionWindow::new(9 * 60 + 30, 10 * 60)],
            },
        ),
    ]))
    .unwrap();

    assert!(
        !artifact
            .calendar()
            .is_open_at_ns(parse_utc_ns("2024-12-25T15:00:00Z").unwrap())
    );
    assert!(
        artifact
            .calendar()
            .is_open_at_ns(parse_utc_ns("2024-11-29T17:59:00Z").unwrap())
    );
    assert!(
        !artifact
            .calendar()
            .is_open_at_ns(parse_utc_ns("2024-11-29T18:01:00Z").unwrap())
    );
    // DST begins on this date: 09:30 exchange-local is 13:30Z, not 14:30Z.
    assert!(
        artifact
            .calendar()
            .is_open_at_ns(parse_utc_ns("2024-03-10T13:45:00Z").unwrap())
    );
    assert!(
        !artifact
            .calendar()
            .is_open_at_ns(parse_utc_ns("2024-03-10T14:15:00Z").unwrap())
    );

    // A published closure is reported as an exchange statement, not as the
    // rule-derived holiday it would be mistaken for otherwise.
    assert_eq!(
        artifact
            .calendar()
            .status_at_ns(parse_utc_ns("2024-12-25T15:00:00Z").unwrap())
            .closed_reason(),
        Some(ClosedReason::PublishedClosure)
    );
}

/// The published minute is exchange-local, so the same 13:00 close lands on two
/// different UTC instants either side of a DST transition. A fixed offset would
/// silently move one of these half days by an hour.
#[test]
fn one_published_close_minute_resolves_to_two_utc_instants_across_dst() {
    let half_day = |id: &str, date: &str| {
        calendar_record(
            id,
            date,
            CalendarExceptionSourceKind::EarlyClose {
                close_minute: 13 * 60,
            },
        )
    };
    let artifact = materialize_calendar(&calendar_request(vec![
        // EDT (UTC−4): 13:00 local is 17:00Z.
        half_day("independence-eve", "2024-07-03"),
        // EST (UTC−5): the same 13:00 local is 18:00Z.
        half_day("thanksgiving-friday", "2024-11-29"),
    ]))
    .unwrap();
    let calendar = artifact.calendar();

    assert!(calendar.is_open_at_ns(parse_utc_ns("2024-07-03T16:59:00Z").unwrap()));
    assert!(!calendar.is_open_at_ns(parse_utc_ns("2024-07-03T17:01:00Z").unwrap()));
    assert!(calendar.is_open_at_ns(parse_utc_ns("2024-11-29T17:59:00Z").unwrap()));
    assert!(!calendar.is_open_at_ns(parse_utc_ns("2024-11-29T18:01:00Z").unwrap()));

    // The fall-back repeat hour is two UTC instants on one exchange-local date,
    // so an exception keyed by that date covers both without ambiguity.
    let closed = materialize_calendar(&calendar_request(vec![calendar_record(
        "fall-back-sunday",
        "2024-11-03",
        CalendarExceptionSourceKind::Closed,
    )]))
    .unwrap();
    for repeat in ["2024-11-03T05:30:00Z", "2024-11-03T06:30:00Z"] {
        assert_eq!(
            closed
                .calendar()
                .status_at_ns(parse_utc_ns(repeat).unwrap())
                .closed_reason(),
            Some(ClosedReason::PublishedClosure),
            "{repeat} is 01:30 exchange-local on the same published date"
        );
    }
}

/// Bounds are checked before a batch is cloned, sorted or hashed, so an
/// oversized snapshot is refused rather than processed and then rejected.
#[test]
fn oversize_batches_and_raw_records_are_refused() {
    let too_many: Vec<_> = (0..=MAX_REFERENCE_RECORDS)
        .map(|index| {
            calendar_record(
                &format!("holiday-{index}"),
                "2024-12-25",
                CalendarExceptionSourceKind::Closed,
            )
        })
        .collect();
    assert_eq!(
        materialize_calendar(&calendar_request(too_many)).unwrap_err(),
        ReferenceDataError::TooManyRecords {
            found: MAX_REFERENCE_RECORDS + 1
        }
    );

    let too_many_actions: Vec<_> = (0..=MAX_REFERENCE_RECORDS)
        .map(|index| split(&format!("split-{index}"), "2", "1"))
        .collect();
    assert_eq!(
        materialize_corporate_actions(&corporate_request(too_many_actions)).unwrap_err(),
        ReferenceDataError::TooManyRecords {
            found: MAX_REFERENCE_RECORDS + 1
        }
    );

    // An unbounded raw record would let one artifact carry an arbitrary payload
    // under a sealed id. Empty is refused for the same reason: nothing to hash.
    for raw in [String::new(), "x".repeat(MAX_RAW_SOURCE_BYTES + 1)] {
        let mut record =
            calendar_record("holiday", "2024-12-25", CalendarExceptionSourceKind::Closed);
        record.raw_record_sha256 = raw_source_sha256(&raw);
        record.raw_source = raw;
        assert!(matches!(
            materialize_calendar(&calendar_request(vec![record])),
            Err(ReferenceDataError::OversizeRawSource { .. })
        ));
    }
}

/// The shortened day is the *base calendar's* own session cut at the published
/// minute. Nothing here may assume one venue's opening bell.
#[test]
fn an_early_close_shortens_the_venues_own_windows_and_never_invents_an_open() {
    let base = TradingCalendarSpec {
        session: SessionRule::LocalWindows {
            windows: vec![
                LocalSessionWindow::new(8 * 60, 11 * 60),
                LocalSessionWindow::new(12 * 60, 17 * 60),
            ],
            early_close: EarlyCloseRule::None,
        },
        ..TradingCalendarSpec::us_equity_regular()
    };
    let mut request = calendar_request(vec![calendar_record(
        "half-day",
        "2024-12-24",
        CalendarExceptionSourceKind::EarlyClose {
            close_minute: 13 * 60,
        },
    )]);
    request.base = base;
    let artifact = materialize_calendar(&request).unwrap();

    // The 08:00 open is preserved and both windows are truncated at 13:00 — a
    // hardcoded 09:30 open would have silently deleted the first hour.
    assert_eq!(
        artifact.exceptions()[0].kind,
        CalendarExceptionKind::SessionOverride {
            windows: vec![
                LocalSessionWindow::new(8 * 60, 11 * 60),
                LocalSessionWindow::new(12 * 60, 13 * 60),
            ],
        }
    );
    let calendar = artifact.calendar();
    // 08:30 ET on a December date is 13:30Z (standard time).
    assert!(calendar.is_open_at_ns(parse_utc_ns("2024-12-24T13:30:00Z").unwrap()));
    assert!(!calendar.is_open_at_ns(parse_utc_ns("2024-12-24T18:30:00Z").unwrap()));

    // A policy-only calendar declares no windows, so there is nothing to
    // shorten and the record is refused rather than given an invented session.
    let mut policy_only = request.clone();
    policy_only.base = TradingCalendarSpec {
        time_zone: ExchangeTimeZone::UsEastern,
        ..TradingCalendarSpec::xstock_24x5()
    };
    assert!(matches!(
        materialize_calendar(&policy_only),
        Err(ReferenceDataError::MalformedRecord { .. })
    ));

    // An "early close" at or before the first open is a closure, not a session.
    let mut before_open = request.clone();
    before_open.records[0].kind = CalendarExceptionSourceKind::EarlyClose { close_minute: 60 };
    before_open.records[0].raw_record_sha256 =
        raw_source_sha256(&before_open.records[0].raw_source);
    assert!(matches!(
        materialize_calendar(&before_open),
        Err(ReferenceDataError::MalformedRecord { .. })
    ));
}

/// `early_close` says the published day is shorter than a regular one. An
/// override that runs to the usual bell is a normal day and must not claim
/// otherwise — a session-relative rule reads that flag.
#[test]
fn a_full_length_override_is_not_reported_as_an_early_close() {
    let artifact = materialize_calendar(&calendar_request(vec![
        calendar_record(
            "full-day-open",
            "2024-03-10",
            CalendarExceptionSourceKind::OpenOverride {
                windows: vec![LocalSessionWindow::new(9 * 60 + 30, 16 * 60)],
            },
        ),
        calendar_record(
            "half-day-open",
            "2024-03-17",
            CalendarExceptionSourceKind::OpenOverride {
                windows: vec![LocalSessionWindow::new(9 * 60 + 30, 13 * 60)],
            },
        ),
    ]))
    .unwrap();

    let flag = |at: &str| match artifact.calendar().status_at_ns(parse_utc_ns(at).unwrap()) {
        SessionStatus::Open { early_close, .. } => early_close,
        other => panic!("expected an open session, got {other:?}"),
    };
    assert!(!flag("2024-03-10T14:00:00Z"));
    assert!(flag("2024-03-17T14:00:00Z"));
}

#[test]
fn declaration_order_is_canonical_and_retrieval_time_is_audit_only() {
    let a = calendar_record("a", "2024-12-25", CalendarExceptionSourceKind::Closed);
    let b = calendar_record(
        "b",
        "2024-11-29",
        CalendarExceptionSourceKind::EarlyClose {
            close_minute: 13 * 60,
        },
    );
    let first = materialize_calendar(&calendar_request(vec![a.clone(), b.clone()])).unwrap();
    let mut reordered = calendar_request(vec![b, a]);
    reordered.source.retrieved_at_ns += 123;
    let second = materialize_calendar(&reordered).unwrap();
    assert_eq!(first.artifact_id(), second.artifact_id());

    reordered.source.as_of_ns += 1;
    let changed = materialize_calendar(&reordered).unwrap();
    assert_ne!(first.artifact_id(), changed.artifact_id());
}

#[test]
fn incomplete_outage_rule_only_yahoo_and_dishonest_authority_fail_closed() {
    let mut request = calendar_request(Vec::new());
    request.source.complete = false;
    assert_eq!(
        materialize_calendar(&request).unwrap_err(),
        ReferenceDataError::IncompleteRange
    );

    request.source.complete = true;
    request.source.coverage = SourceCoverage::ExchangeDateRange {
        start: request.range_start,
        end_inclusive: date("2025-12-30"),
    };
    assert_eq!(
        materialize_calendar(&request).unwrap_err(),
        ReferenceDataError::IncompleteRange
    );

    request.source = calendar_source(
        SourceSystem::RuleDerived {
            ruleset: "built-in NYSE rules".into(),
        },
        SourceAuthorityClass::DerivedRule,
    );
    assert_eq!(
        materialize_calendar(&request).unwrap_err(),
        ReferenceDataError::NotAuthoritative
    );

    request.source = calendar_source(
        SourceSystem::YahooChartKeyless,
        SourceAuthorityClass::ExchangeOfficial,
    );
    assert_eq!(
        materialize_calendar(&request).unwrap_err(),
        ReferenceDataError::DishonestAuthority
    );

    request.source = calendar_source(
        SourceSystem::Unavailable {
            intended_source: "NYSE".into(),
        },
        SourceAuthorityClass::Unavailable,
    );
    assert_eq!(
        materialize_calendar(&request).unwrap_err(),
        ReferenceDataError::SourceUnavailable
    );
}

/// One exchange-local date carries one published verdict. A repeat of the same
/// verdict and a contradictory one are named apart, because the operator fixes
/// them differently — one record is redundant, the other is wrong — and neither
/// may be collapsed into a single exception.
#[test]
fn a_repeated_calendar_date_is_a_duplicate_and_a_contradictory_one_is_a_conflict() {
    let closed = |id: &str| calendar_record(id, "2024-12-25", CalendarExceptionSourceKind::Closed);
    assert_eq!(
        materialize_calendar(&calendar_request(vec![closed("a"), closed("b")])).unwrap_err(),
        ReferenceDataError::DuplicateEvent {
            source_record_id: "b".into()
        }
    );
    assert_eq!(
        materialize_calendar(&calendar_request(vec![
            closed("a"),
            calendar_record(
                "b",
                "2024-12-25",
                CalendarExceptionSourceKind::EarlyClose {
                    close_minute: 13 * 60,
                },
            ),
        ]))
        .unwrap_err(),
        ReferenceDataError::ConflictingEvent {
            source_record_id: "b".into()
        }
    );
    // The same source id twice is a different fault: the batch itself is
    // malformed, before any question of what the two records say.
    let repeated = closed("a");
    assert_eq!(
        materialize_calendar(&calendar_request(vec![repeated.clone(), repeated])).unwrap_err(),
        ReferenceDataError::DuplicateSourceRecord {
            source_record_id: "a".into()
        }
    );
}

#[test]
fn split_and_dividend_convert_safely_and_have_deterministic_order() {
    let artifact = materialize_corporate_actions(&corporate_request(vec![
        action(
            "dividend",
            "2024-06-10T13:30:00Z",
            CorporateActionSourceKind::CashDividend {
                amount_per_unit: "0.25".into(),
            },
        ),
        split("split", "2", "1"),
    ]))
    .unwrap();
    assert_eq!(artifact.schedule().actions()[0].kind.wire_id(), "split");
    assert_eq!(
        artifact.schedule().actions()[1].kind.wire_id(),
        "cash_dividend"
    );

    assert!(
        materialize_corporate_actions(&corporate_request(vec![split("bad", "2.0", "1")])).is_err()
    );
    let bad_decimal = action(
        "bad-dividend",
        "2024-06-10T13:30:00Z",
        CorporateActionSourceKind::CashDividend {
            amount_per_unit: "0.250".into(),
        },
    );
    assert!(materialize_corporate_actions(&corporate_request(vec![bad_decimal])).is_err());
}

/// The source-record rank table mirrors the schedule's. If they drift, records
/// and sealed actions sort differently and the artifact id pairs the wrong
/// record with the wrong event — silently, and deterministically.
#[test]
fn source_ranks_match_the_schedule() {
    let cases = [
        (
            CorporateActionSourceKind::Split {
                numerator: "2".into(),
                denominator: "1".into(),
            },
            CorporateActionKind::Split {
                numerator: 2,
                denominator: 1,
            },
        ),
        (
            CorporateActionSourceKind::CashDividend {
                amount_per_unit: "0.25".into(),
            },
            CorporateActionKind::CashDividend {
                amount_per_unit: 0.25,
            },
        ),
        (
            CorporateActionSourceKind::SymbolChange {
                new_symbol: "BBB".into(),
            },
            CorporateActionKind::SymbolChange {
                new_symbol: "BBB".into(),
            },
        ),
        (
            CorporateActionSourceKind::Delisting,
            CorporateActionKind::Delisting,
        ),
    ];
    for (source, action) in cases {
        assert_eq!(
            source.order_rank(),
            action.order_rank(),
            "{source:?} ranks differently from {action:?}"
        );
    }
}

/// Records are sealed in canonical order, so a decoded artifact that presents
/// the same set in any other order is refused. Accepting it would give one set
/// of records as many artifact ids as it has permutations.
#[test]
fn a_decoded_artifact_whose_records_are_reordered_is_refused() {
    let artifact = materialize_corporate_actions(&corporate_request(vec![
        split("split", "2", "1"),
        action(
            "dividend",
            "2024-06-10T13:30:00Z",
            CorporateActionSourceKind::CashDividend {
                amount_per_unit: "0.25".into(),
            },
        ),
    ]))
    .unwrap();
    let bytes = encode_corporate_action_artifact(&artifact).unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let records = value["source_records"].as_array_mut().unwrap();
    records.reverse();
    assert_eq!(
        decode_corporate_action_artifact(&serde_json::to_vec(&value).unwrap()).unwrap_err(),
        ReferenceDataError::NonCanonicalArtifact
    );

    let calendar = materialize_calendar(&calendar_request(vec![
        calendar_record("a", "2024-11-29", CalendarExceptionSourceKind::Closed),
        calendar_record("b", "2024-12-25", CalendarExceptionSourceKind::Closed),
    ]))
    .unwrap();
    let bytes = encode_calendar_artifact(&calendar).unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    value["source_records"].as_array_mut().unwrap().reverse();
    value["exceptions"].as_array_mut().unwrap().reverse();
    assert!(decode_calendar_artifact(&serde_json::to_vec(&value).unwrap()).is_err());
}

/// A `+00:00` timestamp is a second spelling of one instant and a bare local
/// time is genuinely ambiguous across a DST boundary. Both are refused.
#[test]
fn ambiguous_and_non_canonical_timestamps_are_refused() {
    for stamp in [
        "2024-06-10T13:30:00+00:00",
        "2024-06-10T09:30:00-04:00",
        "2024-06-10T13:30:00",
        "2024-06-10",
    ] {
        assert!(
            matches!(
                parse_utc_ns(stamp),
                Err(ReferenceDataError::TimezoneAmbiguous { .. })
            ),
            "`{stamp}` must not resolve to an instant"
        );
        let mut record = split("split", "2", "1");
        record.effective_utc = stamp.into();
        assert!(materialize_corporate_actions(&corporate_request(vec![record])).is_err());
    }
    // `Z` with a fractional second is still exact, and still one instant.
    assert_eq!(
        parse_utc_ns("2024-06-10T13:30:00.000000001Z").unwrap(),
        parse_utc_ns("2024-06-10T13:30:00Z").unwrap() + 1
    );
}

#[test]
fn duplicate_conflict_unsupported_out_of_range_and_raw_tampering_are_rejected() {
    let duplicate = split("same", "2", "1");
    assert!(matches!(
        materialize_corporate_actions(&corporate_request(vec![duplicate.clone(), duplicate])),
        Err(ReferenceDataError::DuplicateSourceRecord { .. })
    ));
    // Same instant, same class, same economics under two source ids: redundant,
    // and refused rather than applied twice.
    assert_eq!(
        materialize_corporate_actions(&corporate_request(vec![
            split("a", "2", "1"),
            split("b", "2", "1"),
        ]))
        .unwrap_err(),
        ReferenceDataError::DuplicateEvent {
            source_record_id: "b".into()
        }
    );
    // Same instant, same class, different economics: the sources disagree.
    assert_eq!(
        materialize_corporate_actions(&corporate_request(vec![
            split("a", "2", "1"),
            split("b", "3", "1"),
        ]))
        .unwrap_err(),
        ReferenceDataError::ConflictingEvent {
            source_record_id: "b".into()
        }
    );
    assert!(matches!(
        materialize_corporate_actions(&corporate_request(vec![action(
            "spin",
            "2024-06-10T13:30:00Z",
            CorporateActionSourceKind::Unsupported {
                action_type: "spin_off".into()
            },
        )])),
        Err(ReferenceDataError::UnsupportedActionType { .. })
    ));
    let outside = action(
        "outside",
        "2026-01-01T00:00:00Z",
        CorporateActionSourceKind::Delisting,
    );
    assert!(matches!(
        materialize_corporate_actions(&corporate_request(vec![outside])),
        Err(ReferenceDataError::MalformedRecord { .. })
    ));
    let mut tampered = split("tampered", "2", "1");
    tampered.raw_source.push(' ');
    assert!(matches!(
        materialize_corporate_actions(&corporate_request(vec![tampered])),
        Err(ReferenceDataError::RawRecordIdentityMismatch { .. })
    ));
}

#[test]
fn adjusted_price_double_count_guard_rejects_splits_and_total_return_dividends() {
    let mut split_adjusted = corporate_request(vec![split("split", "2", "1")]);
    split_adjusted.adjustment = AdjustmentPolicy::SplitAdjusted;
    assert!(matches!(
        materialize_corporate_actions(&split_adjusted),
        Err(ReferenceDataError::AdjustedPriceDoubleCounting { .. })
    ));

    let mut total_return = corporate_request(vec![action(
        "dividend",
        "2024-06-10T13:30:00Z",
        CorporateActionSourceKind::CashDividend {
            amount_per_unit: "0.25".into(),
        },
    )]);
    total_return.adjustment = AdjustmentPolicy::TotalReturn;
    assert!(matches!(
        materialize_corporate_actions(&total_return),
        Err(ReferenceDataError::AdjustedPriceDoubleCounting { .. })
    ));
}

#[test]
fn bounded_codec_rejects_oversize_unknown_noncanonical_and_tampered_artifacts() {
    let artifact = materialize_calendar(&calendar_request(vec![calendar_record(
        "holiday",
        "2024-12-25",
        CalendarExceptionSourceKind::Closed,
    )]))
    .unwrap();
    let bytes = encode_calendar_artifact(&artifact).unwrap();
    assert_eq!(
        decode_calendar_artifact(&bytes).unwrap().artifact_id(),
        artifact.artifact_id()
    );
    assert_eq!(
        decode_calendar_artifact(&vec![b' '; MAX_REFERENCE_ARTIFACT_BYTES + 1]).unwrap_err(),
        ReferenceDataError::ArtifactTooLarge
    );
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    value["unknown"] = serde_json::json!(true);
    assert!(matches!(
        decode_calendar_artifact(&serde_json::to_vec(&value).unwrap()),
        Err(ReferenceDataError::Decode(_))
    ));
    let pretty = serde_json::to_vec_pretty(&artifact).unwrap();
    assert_eq!(
        decode_calendar_artifact(&pretty).unwrap_err(),
        ReferenceDataError::NonCanonicalArtifact
    );
    let mut tampered: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    tampered["artifact_id"] = serde_json::Value::String("0".repeat(64));
    assert_eq!(
        decode_calendar_artifact(&serde_json::to_vec(&tampered).unwrap()).unwrap_err(),
        ReferenceDataError::ArtifactIdentityMismatch
    );
}

#[test]
fn restart_identity_and_current_execution_binding_round_trip() {
    let calendar = materialize_calendar(&calendar_request(vec![calendar_record(
        "holiday",
        "2024-12-25",
        CalendarExceptionSourceKind::Closed,
    )]))
    .unwrap();
    let actions =
        materialize_corporate_actions(&corporate_request(vec![split("split", "2", "1")])).unwrap();
    let root = std::env::temp_dir().join(format!(
        "typhoon-reference-artifacts-{}-{:x}",
        std::process::id(),
        parse_utc_ns("2024-01-01T00:00:00Z").unwrap()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let store = ReferenceArtifactStore::open(&root).unwrap();
    store.put_calendar(&calendar).unwrap();
    store.put_corporate_actions(&actions).unwrap();
    // A restart knows only the ids: the store is addressed by content, so the
    // artifacts must come back byte-identical from the id alone.
    let reloaded_calendar = store.load_calendar(calendar.artifact_id()).unwrap();
    let reloaded_actions = store.load_corporate_actions(actions.artifact_id()).unwrap();
    assert_eq!(calendar, reloaded_calendar);
    assert_eq!(actions, reloaded_actions);
    assert_eq!(
        store.list_ids(ReferenceArtifactKind::Calendar).unwrap(),
        vec![calendar.artifact_id().to_string()]
    );
    assert_eq!(
        store
            .list_ids(ReferenceArtifactKind::CorporateActions)
            .unwrap(),
        vec![actions.artifact_id().to_string()]
    );
    // A path is never a key: only 64-hex ids resolve, so no caller can point
    // the store at a file outside its own root.
    assert_eq!(
        store.load_calendar("../../etc/passwd").unwrap_err(),
        ReferenceDataError::ArtifactIdentityMismatch
    );

    let bound = bind_reference_artifacts(
        &ExecutionSettings::conservative_defaults(),
        "AAA",
        "USD",
        &reloaded_calendar,
        &reloaded_actions,
    )
    .unwrap();
    let config = StrategyExecutionConfig::build(&bound).unwrap();
    assert_eq!(
        config.schema_version(),
        STRATEGY_EXECUTION_CONFIG_SCHEMA_VERSION
    );
    assert_eq!(
        config.settings().reference_data.calendar_artifact_ids,
        vec![calendar.artifact_id()]
    );
    assert_eq!(
        config
            .settings()
            .reference_data
            .corporate_action_artifact_ids,
        vec![actions.artifact_id()]
    );
    assert_eq!(config.recompute_config_id().unwrap(), config.config_id());
    std::fs::remove_dir_all(root).unwrap();
}
