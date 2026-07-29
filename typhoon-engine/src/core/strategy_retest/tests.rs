use super::*;
use crate::broker::alpaca::Bar;
use crate::core::strategy_builder::GeneralStrategyBuilder;
use crate::core::strategy_dataset::{
    AdjustmentPolicy, CalendarPolicy, DatasetManifest, DatasetManifestInput, DatasetProvenance,
    DatasetQaPolicy,
};
use crate::core::strategy_ir::{ExecutionSettings, StrategyExecutionConfig};
use crate::core::strategy_metrics::METRICS_SCHEMA_VERSION;
use crate::core::strategy_optimization::{
    HoldoutQuarantine, ObservationRole, Percentile, RobustnessPipeline, RobustnessStageSpec,
    StageAccess, StageVerdict, Threshold,
};

fn bars() -> Vec<Bar> {
    (0..8)
        .map(|index| {
            let open = 100.0 + index as f64;
            Bar {
                timestamp: format!("2026-01-{:02}T00:00:00Z", index + 1),
                open,
                high: open + 2.0,
                low: open - 1.0,
                close: open + 1.0,
                volume: 100.0,
            }
        })
        .collect()
}

fn manifest(bars: &[Bar]) -> DatasetManifest {
    DatasetManifest::build(
        &DatasetManifestInput {
            symbol: "BTC/USD".into(),
            timeframe: "1Day".into(),
            provenance: DatasetProvenance {
                source: "fixture".into(),
                venue: "test".into(),
                pipeline: "strategy-retest-test/v1".into(),
            },
            adjustment: AdjustmentPolicy::Raw,
            calendar: CalendarPolicy::Continuous24x7,
            qa_policy: DatasetQaPolicy::default(),
        },
        bars,
    )
    .unwrap()
}

fn fixture() -> (
    crate::core::strategy_ir::StrategyIr,
    StrategyExecutionConfig,
    Vec<Bar>,
    DatasetManifest,
) {
    let strategy = crate::core::strategy_ir::StrategyIr::build(
        GeneralStrategyBuilder::new("retest", "test").definition(),
    )
    .unwrap();
    let config =
        StrategyExecutionConfig::build(&ExecutionSettings::conservative_defaults()).unwrap();
    let bars = bars();
    let manifest = manifest(&bars);
    (strategy, config, bars, manifest)
}

fn pipeline() -> RobustnessPipeline {
    RobustnessPipeline::new(vec![RobustnessStageSpec::metric_percentile(
        1,
        "finite-return",
        ObservationRole::SearchEvaluation,
        "net_profit",
        Percentile::Median,
        Threshold::AtLeast(0.0),
    )])
    .unwrap()
}

#[test]
fn retest_executes_the_exact_leased_bars_and_seals_report_observation_and_verdict() {
    let (strategy, config, bars, manifest) = fixture();
    let quarantine = HoldoutQuarantine::new(&manifest.dataset_id, "f".repeat(64), 10, 2).unwrap();
    let lease = quarantine.lease(StageAccess::Search).unwrap();
    let request = RetestExecutionRequest::seal(
        &strategy,
        &config,
        &manifest,
        &bars,
        lease,
        ObservationRole::SearchEvaluation,
        "net_profit",
        9,
    )
    .unwrap();
    let completed = execute_retest(request, &pipeline(), 17).unwrap();

    completed.report().verify().unwrap();
    assert_eq!(completed.report().run_id(), completed.run_id());
    assert_eq!(completed.observation().metric("net_profit"), Some(0.0));
    assert_eq!(completed.robustness().verdict(), StageVerdict::Pass);
    assert_eq!(completed.evaluations_n(), 17);
    assert!(completed.best_label().contains("best of N=17"));
}

#[test]
fn retest_refuses_foreign_tampered_or_wrong_range_content_and_undefined_metrics() {
    let (strategy, config, bars, manifest) = fixture();
    let make = |manifest: &DatasetManifest, bars: &[Bar], total: usize, metric: &str| {
        let quarantine =
            HoldoutQuarantine::new(&manifest.dataset_id, "f".repeat(64), total, 2).unwrap();
        RetestExecutionRequest::seal(
            &strategy,
            &config,
            manifest,
            bars,
            quarantine.lease(StageAccess::Robustness).unwrap(),
            ObservationRole::OutOfSample,
            metric,
            9,
        )
    };

    let mut foreign = bars.clone();
    foreign[3].close += 5.0;
    assert!(make(&manifest, &foreign, 10, "net_profit").is_err());
    assert!(make(&manifest, &bars, 11, "net_profit").is_err());

    let mut tampered_manifest = manifest.clone();
    tampered_manifest.bar_count += 1;
    assert!(make(&tampered_manifest, &bars, 10, "net_profit").is_err());

    let request = make(&manifest, &bars, 10, "profit_factor").unwrap();
    assert!(execute_retest(request, &pipeline(), 1).is_err());
}

#[test]
fn immutable_evidence_store_persists_lineage_n_robustness_and_bounded_indexed_queries() {
    let (strategy, config, bars, manifest) = fixture();
    let quarantine = HoldoutQuarantine::new(&manifest.dataset_id, "f".repeat(64), 10, 2).unwrap();
    let request = RetestExecutionRequest::seal(
        &strategy,
        &config,
        &manifest,
        &bars,
        quarantine.lease(StageAccess::Search).unwrap(),
        ObservationRole::SearchEvaluation,
        "net_profit",
        9,
    )
    .unwrap();
    let completed = execute_retest(request, &pipeline(), 17).unwrap();
    let store = RetestEvidenceStore::open_in_memory().unwrap();
    store.persist(&completed, Some("parent-run"), 1).unwrap();

    let page = store
        .query(&RetestEvidenceQuery {
            candidate_id: strategy.strategy_id().to_string(),
            after_sequence: None,
            limit: 1,
        })
        .unwrap();
    assert_eq!(page.records.len(), 1);
    assert!(!page.has_more);
    let record = &page.records[0];
    assert_eq!(record.parent_run_id.as_deref(), Some("parent-run"));
    assert_eq!(record.evaluations_n, 17);
    assert_eq!(record.robustness_verdict, StageVerdict::Pass);
    assert_eq!(record.metric_id, "net_profit");
    assert_eq!(record.metric_value, 0.0);
    assert_eq!(record.range, 0..8);

    let plan = store
        .explain_query(&RetestEvidenceQuery {
            candidate_id: strategy.strategy_id().to_string(),
            after_sequence: None,
            limit: 1,
        })
        .unwrap();
    assert!(
        plan.iter()
            .any(|detail| detail.contains("idx_retest_candidate_sequence"))
    );
    assert!(store.persist(&completed, Some("parent-run"), 2).is_err());
    assert!(store.test_only_update(completed.run_id()).is_err());
    assert!(
        store
            .query(&RetestEvidenceQuery {
                candidate_id: strategy.strategy_id().to_string(),
                after_sequence: None,
                limit: MAX_RETEST_QUERY_LIMIT + 1,
            })
            .is_err()
    );
}

#[test]
fn holdout_consumption_is_one_way_immutable_and_index_queryable() {
    let store = RetestEvidenceStore::open_in_memory().unwrap();
    let quarantine = HoldoutQuarantine::new("a".repeat(64), "f".repeat(64), 10, 2).unwrap();
    assert!(quarantine.lease(StageAccess::FinalReview).is_err());
    let burned = quarantine.burn("promotion review").unwrap();
    store.record_holdout_consumption(&burned, 4).unwrap();
    let records = store.query_holdout(&"f".repeat(64), 4).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].range, 8..10);
    assert_eq!(records[0].reason, "promotion review");
    assert!(store.record_holdout_consumption(&burned, 5).is_err());
    assert!(store.test_only_update_holdout(&"f".repeat(64)).is_err());
}

#[test]
fn request_identity_changes_with_content_range_role_metric_and_run_inputs() {
    let (strategy, config, bars, manifest) = fixture();
    let seal = |config: &StrategyExecutionConfig,
                manifest: &DatasetManifest,
                bars: &[Bar],
                role: ObservationRole,
                metric: &str,
                seed: u64| {
        let quarantine =
            HoldoutQuarantine::new(&manifest.dataset_id, "f".repeat(64), 10, 2).unwrap();
        RetestExecutionRequest::seal(
            &strategy,
            config,
            manifest,
            bars,
            quarantine.lease(StageAccess::Search).unwrap(),
            role,
            metric,
            seed,
        )
        .unwrap()
    };
    let baseline = seal(
        &config,
        &manifest,
        &bars,
        ObservationRole::SearchEvaluation,
        "net_profit",
        9,
    );
    let other_config = {
        let mut settings = config.to_input();
        settings.warmup_bars = 1;
        StrategyExecutionConfig::build(&settings).unwrap()
    };
    let variants = [
        seal(
            &other_config,
            &manifest,
            &bars,
            ObservationRole::SearchEvaluation,
            "net_profit",
            9,
        ),
        seal(
            &config,
            &manifest,
            &bars,
            ObservationRole::OutOfSample,
            "net_profit",
            9,
        ),
        seal(
            &config,
            &manifest,
            &bars,
            ObservationRole::SearchEvaluation,
            "total_return",
            9,
        ),
        seal(
            &config,
            &manifest,
            &bars,
            ObservationRole::SearchEvaluation,
            "net_profit",
            10,
        ),
    ];
    assert!(
        variants
            .iter()
            .all(|variant| variant.request_id() != baseline.request_id())
    );
    assert_eq!(baseline.metrics_version(), METRICS_SCHEMA_VERSION);
}

fn execution_request(metric_id: &str) -> RetestExecutionRequest {
    let (strategy, config, bars, manifest) = fixture();
    let quarantine = HoldoutQuarantine::new(&manifest.dataset_id, "f".repeat(64), 10, 2).unwrap();
    RetestExecutionRequest::seal(
        &strategy,
        &config,
        &manifest,
        &bars,
        quarantine.lease(StageAccess::Search).unwrap(),
        ObservationRole::SearchEvaluation,
        metric_id,
        9,
    )
    .unwrap()
}

#[test]
fn bounded_worker_runs_off_thread_and_reports_cancel_stale_and_failure_without_installing() {
    let submitter = std::thread::current().id();
    let worker = RetestWorker::spawn_in_memory(1, 8).unwrap();
    worker.cancel(2);
    worker
        .try_submit(RetestWorkerJob {
            request_id: 2,
            execution: execution_request("net_profit"),
            pipeline: pipeline(),
            evaluations_n: 3,
            parent_run_id: None,
            created_sequence: 2,
        })
        .unwrap();
    worker
        .try_submit(RetestWorkerJob {
            request_id: 3,
            execution: execution_request("profit_factor"),
            pipeline: pipeline(),
            evaluations_n: 3,
            parent_run_id: None,
            created_sequence: 3,
        })
        .unwrap_or_else(|error| match error {
            RetestSubmitError::Backpressure(job) => {
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
                let mut pending = job;
                while std::time::Instant::now() < deadline {
                    match worker.try_submit(pending) {
                        Ok(()) => return,
                        Err(RetestSubmitError::Backpressure(job)) => pending = job,
                        Err(RetestSubmitError::Stopped(_)) => panic!("worker stopped"),
                    }
                    std::thread::yield_now();
                }
                panic!("worker queue never admitted failure job")
            }
            RetestSubmitError::Stopped(_) => panic!("worker stopped"),
        });
    let mut saw_backpressure = false;
    for request_id in 4..64 {
        if matches!(
            worker.try_submit(RetestWorkerJob {
                request_id,
                execution: execution_request("net_profit"),
                pipeline: pipeline(),
                evaluations_n: 3,
                parent_run_id: None,
                created_sequence: request_id as i64,
            }),
            Err(RetestSubmitError::Backpressure(_))
        ) {
            saw_backpressure = true;
            break;
        }
    }
    assert!(saw_backpressure);

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut cancelled = false;
    let mut failed = false;
    let mut worker_thread = None;
    while std::time::Instant::now() < deadline && !(cancelled && failed) {
        for event in worker.poll() {
            match event {
                RetestWorkerEvent::Started { thread_id, .. } => worker_thread = Some(thread_id),
                RetestWorkerEvent::Cancelled { request_id } if request_id == 2 => cancelled = true,
                RetestWorkerEvent::Failed { request_id, .. } if request_id == 3 => failed = true,
                // A consumer whose active id is newer ignores this completion by id; the worker
                // never owns or mutates UI state.
                RetestWorkerEvent::Completed { request_id, .. } => assert_ne!(request_id, 999),
                _ => {}
            }
        }
        std::thread::yield_now();
    }
    assert!(cancelled && failed);
    assert_ne!(worker_thread.unwrap(), submitter);
}
