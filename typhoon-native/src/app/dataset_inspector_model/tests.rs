use super::*;
use typhoon_engine::broker::alpaca::Bar as EngineBar;
use typhoon_engine::core::strategy_dataset::{
    AdjustmentPolicy, BarField, DatasetQaFinding, DatasetQaIssue, DatasetQaSeverity,
    OhlcViolationKind,
};
use typhoon_engine::core::strategy_dataset_store::DatasetPage;

// ── Helpers ────────────────────────────────────────────────────────

fn bar(index: u64) -> EngineBar {
    EngineBar {
        timestamp: format!("2024-01-{:02}T00:00:00Z", (index % 28) + 1),
        open: 100.0 + index as f64,
        high: 101.0 + index as f64,
        low: 99.0 + index as f64,
        close: 100.5 + index as f64,
        volume: 1_000.0 + index as f64,
    }
}

fn finding(bar_index: usize, issue: DatasetQaIssue) -> DatasetQaFinding {
    DatasetQaFinding {
        bar_index: Some(bar_index),
        timestamp: Some(bar(bar_index as u64).timestamp),
        severity: issue.severity(),
        issue,
    }
}

fn page(offset: u64, count: usize, total: u64, findings: Vec<DatasetQaFinding>) -> DatasetPage {
    DatasetPage {
        offset,
        total_bars: total,
        bars: (0..count).map(|i| bar(offset + i as u64)).collect(),
        findings,
    }
}

fn summary(dataset_id: &str, bar_count: u64) -> DatasetRecordSummary {
    DatasetRecordSummary {
        dataset_id: dataset_id.to_string(),
        manifest_id: "m".repeat(64),
        symbol: "BTC/USD".to_string(),
        timeframe: "1Day".to_string(),
        source: "kraken".to_string(),
        venue: "kraken-spot".to_string(),
        pipeline: "cache-merge/v1".to_string(),
        adjustment: AdjustmentPolicy::Raw,
        calendar_policy_id: "continuous-24x7.v1".to_string(),
        qa_policy_id: "qa.v1:abcdef0123456789".to_string(),
        bar_count,
        first_timestamp: Some("2024-01-01T00:00:00Z".to_string()),
        last_timestamp: Some("2024-02-01T00:00:00Z".to_string()),
        qa_error_count: 1,
        qa_warning_count: 2,
        qa_findings_truncated: false,
    }
}

fn qa_summary() -> DatasetQaSummary {
    DatasetQaSummary {
        error_count: 1,
        warning_count: 2,
        info_count: 0,
        findings_truncated: false,
        findings_omitted: 0,
        gap_detection: "enabled (86400s step)".to_string(),
        spike_detection: "band 0.1000 from 99 moves".to_string(),
    }
}

// ── Page arithmetic ────────────────────────────────────────────────

#[test]
fn page_size_is_clamped_into_the_engines_bounds() {
    assert_eq!(clamp_page_size(0), 1);
    assert_eq!(clamp_page_size(1), 1);
    assert_eq!(
        clamp_page_size(DEFAULT_DATASET_PAGE_SIZE),
        DEFAULT_DATASET_PAGE_SIZE
    );
    assert_eq!(clamp_page_size(usize::MAX), MAX_PAGE_BARS);
    assert_eq!(clamp_page_size(MAX_PAGE_BARS + 1), MAX_PAGE_BARS);

    // Every offered page size is a legal engine request.
    for size in DATASET_INSPECTOR_PAGE_SIZES {
        assert_eq!(clamp_page_size(size), size);
        assert!(size >= 1 && size <= MAX_PAGE_BARS);
    }
    assert!(DATASET_INSPECTOR_PAGE_SIZES.contains(&DEFAULT_DATASET_PAGE_SIZE));
}

#[test]
fn paging_never_walks_off_either_end() {
    assert_eq!(previous_page_offset(0, 100), None);
    assert_eq!(previous_page_offset(50, 100), Some(0));
    assert_eq!(previous_page_offset(250, 100), Some(150));

    assert_eq!(next_page_offset(0, 100, 250), Some(100));
    assert_eq!(next_page_offset(100, 100, 250), Some(200));
    assert_eq!(next_page_offset(200, 100, 250), None);
    assert_eq!(next_page_offset(0, 100, 100), None);
    assert_eq!(next_page_offset(0, 100, 0), None);

    assert_eq!(last_page_offset(100, 250), 200);
    assert_eq!(last_page_offset(100, 200), 100);
    assert_eq!(last_page_offset(100, 0), 0);
    assert_eq!(last_page_offset(100, 1), 0);

    // Nothing overflows near the top of the range.
    assert_eq!(next_page_offset(u64::MAX - 1, 100, u64::MAX), None);
    assert_eq!(previous_page_offset(u64::MAX, 100), Some(u64::MAX - 100));
    assert!(last_page_offset(100, u64::MAX) < u64::MAX);
}

// ── Row projection ─────────────────────────────────────────────────

#[test]
fn rows_are_bounded_and_carry_their_own_flags() {
    let findings = vec![
        finding(
            3,
            DatasetQaIssue::OhlcViolation {
                kind: OhlcViolationKind::HighBelowLow,
            },
        ),
        finding(
            3,
            DatasetQaIssue::NonPositivePrice {
                field: BarField::Close,
                value: 0.0,
            },
        ),
        finding(
            7,
            DatasetQaIssue::CarryForwardBar {
                previous_index: 6,
                zero_volume: true,
            },
        ),
    ];
    let rows = build_rows(&page(0, 10, 10, findings));

    assert_eq!(rows.len(), 10);
    assert!(rows.len() <= MAX_PAGE_BARS);
    assert_eq!(rows[0].index, 0);
    assert!(rows[0].severity.is_none());
    assert!(rows[0].flags.is_empty());

    // The worst severity on a bar wins, and every flag is named.
    assert_eq!(rows[3].severity, Some(DatasetQaSeverity::Error));
    assert!(rows[3].flags.contains("OHLC"), "{}", rows[3].flags);
    assert!(rows[3].flags.contains("price"), "{}", rows[3].flags);

    assert_eq!(rows[7].severity, Some(DatasetQaSeverity::Warning));
    assert!(rows[7].flags.contains("carry"), "{}", rows[7].flags);

    // Offsets shift the absolute bar index, not the window size.
    let rows = build_rows(&page(500, 25, 1_000, Vec::new()));
    assert_eq!(rows.len(), 25);
    assert_eq!(rows[0].index, 500);
    assert_eq!(rows[24].index, 524);
}

#[test]
fn a_finding_outside_the_window_is_ignored_rather_than_mislabelled() {
    // A page whose findings vector was built for a different window must not
    // paint a flag onto an unrelated row.
    let rows = build_rows(&page(
        100,
        10,
        1_000,
        vec![finding(
            5,
            DatasetQaIssue::OhlcViolation {
                kind: OhlcViolationKind::HighBelowLow,
            },
        )],
    ));
    assert!(rows.iter().all(|row| row.severity.is_none()), "{rows:?}");
}

#[test]
fn every_issue_kind_has_a_flag_label() {
    let issues = [
        DatasetQaIssue::EmptyDataset,
        DatasetQaIssue::UnparsableTimestamp {
            raw: "x".to_string(),
        },
        DatasetQaIssue::DuplicateTimestamp {
            previous_index: 0,
            previous_timestamp: "t".to_string(),
        },
        DatasetQaIssue::TimestampOutOfOrder {
            previous_index: 0,
            previous_timestamp: "t".to_string(),
        },
        DatasetQaIssue::NonFiniteValue {
            field: BarField::Open,
            kind: typhoon_engine::core::strategy_dataset::NonFiniteKind::Nan,
        },
        DatasetQaIssue::NonPositivePrice {
            field: BarField::Close,
            value: 0.0,
        },
        DatasetQaIssue::NegativeVolume { value: -1.0 },
        DatasetQaIssue::OhlcViolation {
            kind: OhlcViolationKind::HighBelowLow,
        },
        DatasetQaIssue::UnexpectedWeekendBar {
            weekday: "Sat".to_string(),
        },
        DatasetQaIssue::UnexpectedHolidayBar {
            holiday: "Christmas Day".to_string(),
        },
        DatasetQaIssue::UnexpectedSessionBar {
            window: "w".to_string(),
        },
        DatasetQaIssue::PriceSpike {
            relative_move: 1.0,
            band: 0.1,
        },
        DatasetQaIssue::SuspiciousLevelShift {
            ratio_numerator: 1,
            ratio_denominator: 2,
            previous_close: 2.0,
            close: 1.0,
        },
        DatasetQaIssue::CarryForwardBar {
            previous_index: 0,
            zero_volume: true,
        },
        DatasetQaIssue::MissingBars {
            expected_next: "t".to_string(),
            missing_slots: 1,
            scan_truncated: false,
        },
    ];
    for issue in issues {
        let label = issue_flag_label(&issue);
        assert!(!label.is_empty(), "{issue:?} has no label");
    }
}

// ── Event ingest ───────────────────────────────────────────────────

#[test]
fn a_page_event_replaces_the_window_and_stays_bounded() {
    let mut state = DatasetInspectorState::default();
    state.selected = Some("a".repeat(64));
    let request_id = state.begin_request();

    state.apply_event(DatasetWorkerEvent::Page {
        request_id,
        summary: summary(&"a".repeat(64), 1_000),
        qa_summary: qa_summary(),
        page: Box::new(page(0, 100, 1_000, Vec::new())),
    });

    assert_eq!(state.rows.len(), 100);
    assert_eq!(state.total_bars, 1_000);
    assert_eq!(state.page_offset, 0);
    assert!(state.pending.is_none());
    assert_eq!(state.qa.as_ref().map(|qa| qa.error_count), Some(1));
    assert_eq!(
        state.summary.as_ref().map(|s| s.symbol.as_str()),
        Some("BTC/USD")
    );

    // A second window replaces the first rather than accumulating.
    let request_id = state.begin_request();
    state.apply_event(DatasetWorkerEvent::Page {
        request_id,
        summary: summary(&"a".repeat(64), 1_000),
        qa_summary: qa_summary(),
        page: Box::new(page(100, 100, 1_000, Vec::new())),
    });
    assert_eq!(state.rows.len(), 100);
    assert_eq!(state.page_offset, 100);
    assert_eq!(state.rows[0].index, 100);
}

#[test]
fn a_stale_reply_cannot_overwrite_the_current_window() {
    let mut state = DatasetInspectorState::default();
    state.selected = Some("a".repeat(64));
    let stale = state.begin_request();
    let current = state.begin_request();
    assert_ne!(stale, current);

    state.apply_event(DatasetWorkerEvent::Page {
        request_id: current,
        summary: summary(&"a".repeat(64), 500),
        qa_summary: qa_summary(),
        page: Box::new(page(200, 50, 500, Vec::new())),
    });
    assert_eq!(state.page_offset, 200);

    // The superseded request answers late; it must be dropped.
    state.apply_event(DatasetWorkerEvent::Page {
        request_id: stale,
        summary: summary(&"a".repeat(64), 500),
        qa_summary: qa_summary(),
        page: Box::new(page(0, 50, 500, Vec::new())),
    });
    assert_eq!(state.page_offset, 200, "a stale page overwrote the window");
    assert_eq!(state.rows[0].index, 200);
}

#[test]
fn the_record_list_is_capped_and_failures_surface_as_status() {
    let mut state = DatasetInspectorState::default();
    let request_id = state.begin_request();
    let records: Vec<DatasetRecordSummary> = (0..DATASET_LIST_LIMIT + 50)
        .map(|index| summary(&format!("{index:064x}"), 10))
        .collect();
    state.apply_event(DatasetWorkerEvent::Listed {
        request_id,
        records,
    });
    assert_eq!(state.records.len(), DATASET_LIST_LIMIT);

    let request_id = state.begin_request();
    state.apply_event(DatasetWorkerEvent::Failed {
        request_id,
        message: "payload digest mismatch".to_string(),
    });
    assert!(
        state.status.contains("payload digest mismatch"),
        "{}",
        state.status
    );
    assert!(state.pending.is_none());

    let request_id = state.begin_request();
    state.apply_event(DatasetWorkerEvent::Cancelled { request_id });
    assert!(state.pending.is_none());
    assert!(
        state.status.to_lowercase().contains("cancel"),
        "{}",
        state.status
    );
}

#[test]
fn started_events_do_not_clear_the_pending_request() {
    let mut state = DatasetInspectorState::default();
    let request_id = state.begin_request();
    state.apply_event(DatasetWorkerEvent::Started {
        request_id,
        worker_thread: std::thread::current().id(),
    });
    assert_eq!(state.pending, Some(request_id));
}

#[test]
fn backpressure_is_reported_and_does_not_strand_the_pending_slot() {
    let mut state = DatasetInspectorState::default();
    let request_id = state.begin_request();
    assert_eq!(state.pending, Some(request_id));

    state.note_submit_failure(DatasetSubmitError::QueueFull);
    assert!(state.pending.is_none(), "a refused job must free the slot");
    assert!(
        state.status.to_lowercase().contains("busy"),
        "{}",
        state.status
    );

    let request_id = state.begin_request();
    state.note_submit_failure(DatasetSubmitError::WorkerStopped);
    assert!(state.pending.is_none());
    assert_ne!(state.status, String::new());
    let _ = request_id;
}

// ── Bounded chart snapshot production ─────────────────────────────

fn materialization_identity(len: usize) -> MaterializationIdentity {
    MaterializationIdentity {
        chart_index: 2,
        symbol: "BTC/USD".to_string(),
        timeframe: "1Day".to_string(),
        source: "kraken".to_string(),
        bars_generation: 7,
        len,
        first_ts_ms: Some(1_000),
        last_ts_ms: Some(1_000 + len.saturating_sub(1) as i64),
    }
}

fn materialization_input() -> typhoon_engine::core::strategy_dataset::DatasetManifestInput {
    use typhoon_engine::core::strategy_dataset::{
        CalendarPolicy, DatasetManifestInput, DatasetProvenance, DatasetQaPolicy,
    };
    DatasetManifestInput {
        symbol: "BTC/USD".to_string(),
        timeframe: "1Day".to_string(),
        provenance: DatasetProvenance {
            source: "kraken".to_string(),
            venue: "kraken".to_string(),
            pipeline: "chart-window/v1".to_string(),
        },
        adjustment: AdjustmentPolicy::Raw,
        calendar: CalendarPolicy::Continuous24x7,
        qa_policy: DatasetQaPolicy::default(),
    }
}

#[test]
fn starting_materialization_does_not_visit_chart_bars() {
    let draft =
        MaterializationDraft::start(materialization_identity(10_000), materialization_input());
    assert_eq!(draft.cursor(), 0);
    assert_eq!(draft.produced_len(), 0);
}

#[test]
fn each_materialization_pump_visits_at_most_the_fixed_frame_budget() {
    for len in [1usize, MATERIALIZE_BARS_PER_FRAME, 100_000] {
        let identity = materialization_identity(len);
        let mut draft = MaterializationDraft::start(identity.clone(), materialization_input());
        let mut visits = 0usize;
        let result = draft.pump(&identity, |index| {
            visits += 1;
            bar(index as u64)
        });
        assert!(visits <= MATERIALIZE_BARS_PER_FRAME);
        assert_eq!(visits, len.min(MATERIALIZE_BARS_PER_FRAME));
        assert!(draft.max_chunk_len() <= MATERIALIZE_BARS_PER_FRAME);
        assert!(draft.allocation_high_water() <= MATERIALIZE_BARS_PER_FRAME);
        assert_eq!(
            matches!(result, MaterializationPump::Complete { .. }),
            len <= MATERIALIZE_BARS_PER_FRAME
        );
    }
}

#[test]
fn changed_chart_identity_cancels_a_partial_snapshot_without_visiting_more_bars() {
    let identity = materialization_identity(MATERIALIZE_BARS_PER_FRAME * 2);
    let mut draft = MaterializationDraft::start(identity.clone(), materialization_input());
    assert!(matches!(
        draft.pump(&identity, |index| bar(index as u64)),
        MaterializationPump::Pending
    ));
    for changed in [
        MaterializationIdentity {
            symbol: "ETH/USD".to_string(),
            ..identity.clone()
        },
        MaterializationIdentity {
            timeframe: "1Hour".to_string(),
            ..identity.clone()
        },
        MaterializationIdentity {
            len: identity.len + 1,
            ..identity.clone()
        },
        MaterializationIdentity {
            last_ts_ms: Some(99),
            ..identity.clone()
        },
        MaterializationIdentity {
            bars_generation: identity.bars_generation.wrapping_add(1),
            ..identity.clone()
        },
    ] {
        let mut visits = 0;
        assert!(matches!(
            draft.pump(&changed, |_| {
                visits += 1;
                bar(0)
            }),
            MaterializationPump::Changed
        ));
        assert_eq!(visits, 0);
    }
}

#[test]
fn completed_materialization_hands_bounded_chunks_to_the_worker() {
    let identity = materialization_identity(3);
    let mut draft = MaterializationDraft::start(identity.clone(), materialization_input());
    let MaterializationPump::Complete { bars, .. } =
        draft.pump(&identity, |index| bar(index as u64))
    else {
        panic!("short snapshot should complete");
    };
    assert_eq!(bars.len(), 3);
    assert_eq!(bars.chunk_count(), 1);
    assert!(bars.max_chunk_len() <= MATERIALIZE_BARS_PER_FRAME);
}

#[test]
fn unchanged_length_and_timestamps_with_new_ohlcv_generation_cancels_snapshot() {
    let identity = materialization_identity(MATERIALIZE_BARS_PER_FRAME * 2);
    let mut draft = MaterializationDraft::start(identity.clone(), materialization_input());
    assert!(matches!(
        draft.pump(&identity, |index| bar(index as u64)),
        MaterializationPump::Pending
    ));

    let changed = MaterializationIdentity {
        bars_generation: identity.bars_generation.wrapping_add(1),
        ..identity
    };
    assert!(matches!(
        draft.pump(&changed, |_| panic!(
            "changed generation must cancel before reading"
        )),
        MaterializationPump::Changed
    ));
}
