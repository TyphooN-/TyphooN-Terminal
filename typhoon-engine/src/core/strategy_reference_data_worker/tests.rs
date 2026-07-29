use super::*;
use crate::core::strategy_calendar::{ExchangeTimeZone, TradingCalendarSpec};
use crate::core::strategy_dataset::AdjustmentPolicy;
use crate::core::strategy_reference_data::{
    CalendarExceptionSourceKind, CalendarSourceRecord, CorporateActionSourceKind,
    CorporateActionSourceRecord, IdentityMetadataPolicy, SourceAuthorityClass, SourceSystem,
    parse_utc_ns, raw_source_sha256,
};

/// A private scratch directory per test. Content-addressed stores share file
/// names, so two tests must never share a root.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "typhoon-reference-worker-{}-{name}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("scratch root");
        Self(root)
    }

    fn write(&self, name: &str, snapshot: &ReferenceSourceSnapshot) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, serde_json::to_vec(snapshot).expect("serializes")).expect("write");
        path
    }

    fn store_root(&self) -> PathBuf {
        self.0.join("store")
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn date(value: &str) -> chrono::NaiveDate {
    chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").expect("valid date")
}

fn batch(
    source: SourceSystem,
    authority: SourceAuthorityClass,
    coverage: SourceCoverage,
) -> SourceBatch {
    SourceBatch {
        source,
        authority,
        coverage,
        complete: true,
        as_of_ns: 1_799_000_000_000_000_000,
        retrieved_at_ns: 1_800_000_000_000_000_000,
        identity_metadata_policy: IdentityMetadataPolicy::AsOfIncludedRetrievalExcluded,
    }
}

fn exchange_batch() -> SourceBatch {
    batch(
        SourceSystem::ExchangePublication {
            exchange: "NYSE".into(),
        },
        SourceAuthorityClass::ExchangeOfficial,
        SourceCoverage::ExchangeDateRange {
            start: date("2024-01-01"),
            end_inclusive: date("2025-12-31"),
        },
    )
}

fn calendar_snapshot(source: SourceBatch) -> ReferenceSourceSnapshot {
    let raw = "{\"holiday\":\"christmas\"}".to_string();
    ReferenceSourceSnapshot::Calendar(Box::new(CalendarMaterializationRequest {
        venue: "XNYS".into(),
        time_zone: ExchangeTimeZone::UsEastern,
        range_start: date("2024-01-01"),
        range_end_inclusive: date("2025-12-31"),
        require_authoritative: true,
        source,
        base: TradingCalendarSpec::us_equity_regular(),
        records: vec![CalendarSourceRecord {
            source_record_id: "christmas-2024".into(),
            raw_record_sha256: raw_source_sha256(&raw),
            venue: "XNYS".into(),
            time_zone: ExchangeTimeZone::UsEastern,
            local_date: date("2024-12-25"),
            kind: CalendarExceptionSourceKind::Closed,
            label: "Christmas Day".into(),
            raw_source: raw,
        }],
    }))
}

fn corporate_snapshot() -> ReferenceSourceSnapshot {
    let raw = "{\"split\":\"2:1\"}".to_string();
    ReferenceSourceSnapshot::CorporateActions(Box::new(CorporateActionMaterializationRequest {
        venue: "XNYS".into(),
        symbol: "AAA".into(),
        time_zone: ExchangeTimeZone::UsEastern,
        currency: "USD".into(),
        range_start_ns: parse_utc_ns("2024-01-01T00:00:00Z").expect("start"),
        range_end_ns: parse_utc_ns("2026-01-01T00:00:00Z").expect("end"),
        require_authoritative: false,
        adjustment: AdjustmentPolicy::Raw,
        source: batch(
            SourceSystem::ResearchDatabaseCache {
                upstream: Box::new(SourceSystem::YahooChartKeyless),
            },
            SourceAuthorityClass::UnverifiedPublic,
            SourceCoverage::UtcRange {
                start_ns: parse_utc_ns("2024-01-01T00:00:00Z").expect("start"),
                end_ns: parse_utc_ns("2026-01-01T00:00:00Z").expect("end"),
            },
        ),
        records: vec![CorporateActionSourceRecord {
            source_record_id: "aaa-split-2024".into(),
            raw_record_sha256: raw_source_sha256(&raw),
            venue: "XNYS".into(),
            symbol: "AAA".into(),
            time_zone: ExchangeTimeZone::UsEastern,
            currency: "USD".into(),
            effective_utc: "2024-06-10T13:30:00Z".into(),
            kind: CorporateActionSourceKind::Split {
                numerator: "2".into(),
                denominator: "1".into(),
            },
            raw_source: raw,
        }],
    }))
}

/// Drain until the job's terminal event arrives, so a test never depends on how
/// many frames the worker took.
fn await_terminal(worker: &ReferenceDataWorker, request_id: u64) -> ReferenceDataWorkerEvent {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        for event in worker.poll() {
            match event {
                ReferenceDataWorkerEvent::Started {
                    request_id: id,
                    worker_thread,
                } => {
                    assert_eq!(id, request_id);
                    // The off-render-thread contract, asserted rather than
                    // assumed: no job may run on the caller's thread.
                    assert_ne!(worker_thread, std::thread::current().id());
                }
                terminal => return terminal,
            }
        }
        assert!(
            std::time::Instant::now() < deadline,
            "worker produced no terminal event for request {request_id}"
        );
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
}

#[test]
fn inspecting_a_snapshot_reports_authority_coverage_and_never_promotes() {
    let scratch = Scratch::new("inspect");
    let path = scratch.write("calendar.json", &calendar_snapshot(exchange_batch()));
    let worker = ReferenceDataWorker::spawn_at(scratch.store_root()).expect("spawns");

    worker
        .submit(ReferenceDataJob::InspectSnapshot {
            request_id: 1,
            path,
        })
        .expect("submits");
    let ReferenceDataWorkerEvent::SnapshotInspected { summary, .. } = await_terminal(&worker, 1)
    else {
        panic!("expected an inspection");
    };
    assert_eq!(summary.kind, "calendar exceptions");
    assert_eq!(summary.authority, "exchange_official");
    assert!(summary.authoritative);
    assert!(summary.complete);
    assert_eq!(summary.record_count, 1);
    assert_eq!(
        summary.covered_range,
        "2024-01-01 … 2025-12-31 (exchange dates)"
    );
    assert!(summary.is_promotable(), "blocked: {:?}", summary.blocked);

    // Inspection is read-only: nothing reached the store.
    worker
        .submit(ReferenceDataJob::ListArtifacts {
            request_id: 2,
            limit: MAX_LISTED_ARTIFACTS,
        })
        .expect("submits");
    let ReferenceDataWorkerEvent::ArtifactsListed { summaries, .. } = await_terminal(&worker, 2)
    else {
        panic!("expected a listing");
    };
    assert!(summaries.is_empty(), "inspection must not promote anything");
}

/// The three source classes that cannot back an authoritative calendar are
/// reported as blocked, with the reason, and promotion stays unavailable.
#[test]
fn rule_only_keyless_and_unreachable_sources_are_blocked_with_a_reason() {
    let scratch = Scratch::new("blocked");
    let worker = ReferenceDataWorker::spawn_at(scratch.store_root()).expect("spawns");
    let coverage = SourceCoverage::ExchangeDateRange {
        start: date("2024-01-01"),
        end_inclusive: date("2025-12-31"),
    };
    let cases = [
        (
            "rule",
            batch(
                SourceSystem::RuleDerived {
                    ruleset: "built-in NYSE rules".into(),
                },
                SourceAuthorityClass::DerivedRule,
                coverage,
            ),
        ),
        (
            "yahoo",
            batch(
                SourceSystem::YahooChartKeyless,
                SourceAuthorityClass::UnverifiedPublic,
                coverage,
            ),
        ),
        (
            "outage",
            batch(
                SourceSystem::Unavailable {
                    intended_source: "NYSE".into(),
                },
                SourceAuthorityClass::Unavailable,
                coverage,
            ),
        ),
    ];

    for (index, (name, source)) in cases.into_iter().enumerate() {
        let request_id = index as u64 + 1;
        let path = scratch.write(&format!("{name}.json"), &calendar_snapshot(source));
        worker
            .submit(ReferenceDataJob::InspectSnapshot {
                request_id,
                path: path.clone(),
            })
            .expect("submits");
        let ReferenceDataWorkerEvent::SnapshotInspected { summary, .. } =
            await_terminal(&worker, request_id)
        else {
            panic!("expected an inspection for {name}");
        };
        assert!(
            !summary.authoritative,
            "{name} must not read as authoritative"
        );
        assert!(
            !summary.is_promotable(),
            "{name} must not be promotable: {summary:?}"
        );

        // And the block is real: materializing fails rather than substituting.
        worker
            .submit(ReferenceDataJob::MaterializeSnapshot {
                request_id: request_id + 100,
                path,
            })
            .expect("submits");
        assert!(
            matches!(
                await_terminal(&worker, request_id + 100),
                ReferenceDataWorkerEvent::Failed { .. }
            ),
            "{name} must fail closed"
        );
    }
}

#[test]
fn materializing_then_selecting_seals_a_config_and_labels_its_authority() {
    let scratch = Scratch::new("select");
    let calendar_path = scratch.write("calendar.json", &calendar_snapshot(exchange_batch()));
    let actions_path = scratch.write("actions.json", &corporate_snapshot());
    let worker = ReferenceDataWorker::spawn_at(scratch.store_root()).expect("spawns");

    worker
        .submit(ReferenceDataJob::MaterializeSnapshot {
            request_id: 1,
            path: calendar_path,
        })
        .expect("submits");
    let ReferenceDataWorkerEvent::Materialized { summary, .. } = await_terminal(&worker, 1) else {
        panic!("expected a calendar artifact");
    };
    let calendar_id = summary.artifact_id.clone();
    assert!(summary.authoritative);
    assert_eq!(summary.event_count, 1);

    worker
        .submit(ReferenceDataJob::MaterializeSnapshot {
            request_id: 2,
            path: actions_path,
        })
        .expect("submits");
    let ReferenceDataWorkerEvent::Materialized { summary, .. } = await_terminal(&worker, 2) else {
        panic!("expected a corporate-action artifact");
    };
    let actions_id = summary.artifact_id.clone();
    // Honest: the research cache of a keyless feed is not authoritative, and
    // the summary says so even though the artifact sealed successfully.
    assert!(!summary.authoritative);
    assert_eq!(summary.adjustment, Some("raw"));

    worker
        .submit(ReferenceDataJob::ListArtifacts {
            request_id: 3,
            limit: MAX_LISTED_ARTIFACTS,
        })
        .expect("submits");
    let ReferenceDataWorkerEvent::ArtifactsListed {
        summaries, omitted, ..
    } = await_terminal(&worker, 3)
    else {
        panic!("expected a listing");
    };
    assert_eq!(summaries.len(), 2);
    assert_eq!(omitted, 0);

    worker
        .submit(ReferenceDataJob::SelectIntoConfig {
            request_id: 4,
            settings: Box::new(ExecutionSettings::conservative_defaults()),
            symbol: "AAA".into(),
            currency: "USD".into(),
            calendar_artifact_id: calendar_id.clone(),
            corporate_action_artifact_id: actions_id.clone(),
        })
        .expect("submits");
    let ReferenceDataWorkerEvent::Selected {
        config_id,
        settings,
        authoritative,
        ..
    } = await_terminal(&worker, 4)
    else {
        panic!("expected a selection");
    };
    // One artifact is unverified-public, so the selection is labelled
    // non-authoritative rather than presented as an exchange-backed run.
    assert!(!authoritative);
    assert_eq!(
        settings.reference_data.calendar_artifact_ids,
        vec![calendar_id]
    );
    assert_eq!(
        settings
            .reference_data
            .corporate_action_artifact_id
            .as_deref(),
        Some(actions_id.as_str())
    );
    let rebuilt = StrategyExecutionConfig::build(&settings).expect("rebuilds");
    assert_eq!(rebuilt.config_id(), config_id);
    assert_eq!(rebuilt.schema_version(), 4);
}

/// A listing bounded below the store's size says how many artifacts it is not
/// showing. A silent truncation would read as "this is the whole store".
#[test]
fn a_listing_bounded_below_the_store_reports_what_it_omitted() {
    let scratch = Scratch::new("omitted");
    let worker = ReferenceDataWorker::spawn_at(scratch.store_root()).expect("spawns");
    for (index, snapshot) in [calendar_snapshot(exchange_batch()), corporate_snapshot()]
        .into_iter()
        .enumerate()
    {
        let path = scratch.write(&format!("snapshot-{index}.json"), &snapshot);
        let request_id = index as u64 + 1;
        worker
            .submit(ReferenceDataJob::MaterializeSnapshot { request_id, path })
            .expect("submits");
        assert!(matches!(
            await_terminal(&worker, request_id),
            ReferenceDataWorkerEvent::Materialized { .. }
        ));
    }

    worker
        .submit(ReferenceDataJob::ListArtifacts {
            request_id: 3,
            limit: 1,
        })
        .expect("submits");
    let ReferenceDataWorkerEvent::ArtifactsListed {
        summaries, omitted, ..
    } = await_terminal(&worker, 3)
    else {
        panic!("expected a listing");
    };
    assert_eq!(summaries.len(), 1);
    assert_eq!(
        omitted, 1,
        "the unlisted artifact must be counted, not hidden"
    );
}

/// The job queue is bounded and reports backpressure instead of blocking the
/// caller. A frame callback submits into this queue, so a full queue must be an
/// error value rather than a wait.
#[test]
fn the_job_queue_is_bounded_and_reports_backpressure() {
    let scratch = Scratch::new("backpressure");
    // A snapshot that does not exist: every job still occupies a queue slot,
    // and none of them can seal anything into the store.
    let absent = scratch.0.join("absent.json");
    let worker = ReferenceDataWorker::spawn_at(scratch.store_root()).expect("spawns");

    // The worker drains as it goes, so filling the queue is racy by nature.
    // What is not racy: submitting far past capacity must eventually refuse,
    // and it must refuse with `QueueFull` rather than blocking or panicking.
    let mut refusals = 0;
    for request_id in 0..(REFERENCE_JOB_QUEUE_CAPACITY as u64 + 1) * 64 {
        match worker.submit(ReferenceDataJob::InspectSnapshot {
            request_id,
            path: absent.clone(),
        }) {
            Ok(()) => {}
            Err(ReferenceSubmitError::QueueFull) => refusals += 1,
            Err(other) => panic!("the worker must stay alive, got {other:?}"),
        }
    }
    assert!(
        refusals > 0,
        "a bounded queue submitted far past capacity must report backpressure"
    );

    // Every poll is bounded too, however far behind the caller has fallen.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let mut drained = 0;
    while drained == 0 && std::time::Instant::now() < deadline {
        let batch = worker.poll();
        assert!(
            batch.len() <= MAX_REFERENCE_EVENTS_PER_POLL,
            "one poll drained {} events, past the {MAX_REFERENCE_EVENTS_PER_POLL} ceiling",
            batch.len()
        );
        drained += batch.len();
    }
    assert!(drained > 0, "the worker produced no events at all");

    // The worker is now backlogged on both queues and nothing is draining it.
    // Dropping it must still terminate: it parks rather than growing the event
    // queue, so a drop that only joined would wait on a thread waiting on the
    // receiver the drop itself owns.
    let (finished, shutdown) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        drop(worker);
        let _ = finished.send(());
    });
    assert!(
        shutdown
            .recv_timeout(std::time::Duration::from_secs(30))
            .is_ok(),
        "dropping a backlogged worker must not hang"
    );
}

/// A missing, oversized, or unparseable snapshot fails with a message rather
/// than panicking the worker or wedging the queue.
#[test]
fn unreadable_snapshots_and_unknown_artifact_ids_fail_without_stopping_the_worker() {
    let scratch = Scratch::new("errors");
    let worker = ReferenceDataWorker::spawn_at(scratch.store_root()).expect("spawns");

    let garbage = scratch.0.join("garbage.json");
    std::fs::write(&garbage, b"{\"kind\":\"calendar\"}").expect("write");
    let cases = [
        (1_u64, scratch.0.join("absent.json")),
        (2, garbage),
        (3, scratch.0.clone()),
    ];
    for (request_id, path) in cases {
        worker
            .submit(ReferenceDataJob::InspectSnapshot { request_id, path })
            .expect("submits");
        assert!(matches!(
            await_terminal(&worker, request_id),
            ReferenceDataWorkerEvent::Failed { .. }
        ));
    }

    // A path is not an id, and an unknown id is not a file to open.
    worker
        .submit(ReferenceDataJob::SelectIntoConfig {
            request_id: 4,
            settings: Box::new(ExecutionSettings::conservative_defaults()),
            symbol: "AAA".into(),
            currency: "USD".into(),
            calendar_artifact_id: "../escape".into(),
            corporate_action_artifact_id: "0".repeat(64),
        })
        .expect("submits");
    assert!(matches!(
        await_terminal(&worker, 4),
        ReferenceDataWorkerEvent::Failed { .. }
    ));

    // Still alive afterwards.
    worker
        .submit(ReferenceDataJob::ListArtifacts {
            request_id: 5,
            limit: MAX_LISTED_ARTIFACTS,
        })
        .expect("submits");
    assert!(matches!(
        await_terminal(&worker, 5),
        ReferenceDataWorkerEvent::ArtifactsListed { .. }
    ));
}
