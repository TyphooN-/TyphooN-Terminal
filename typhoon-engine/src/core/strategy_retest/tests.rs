use super::*;
use crate::broker::alpaca::Bar;
use crate::core::strategy_builder::GeneralStrategyBuilder;
use crate::core::strategy_dataset::{
    AdjustmentPolicy, CalendarPolicy, DatasetManifest, DatasetManifestInput, DatasetProvenance,
    DatasetQaPolicy,
};
use crate::core::strategy_ir::{
    CompareOp, Condition, ExecutionSettings, Operand, ParamRange, ParamValue,
    StrategyExecutionConfig, StrategyParameter,
};
use crate::core::strategy_metrics::METRICS_SCHEMA_VERSION;
use crate::core::strategy_optimization::{
    HoldoutQuarantine, MAX_CALENDAR_WINDOW_SECONDS, MAX_MONTE_CARLO_TRIALS, ObjectiveDirection,
    ObservationRole, OosScheme, ParameterDomain, Percentile, RobustnessPipeline,
    RobustnessStageSpec, SampleRole, SearchBatch, SearchMethod, SearchSpace, StageAccess,
    StageVerdict, Threshold, WalkForwardConfig, generate_candidates,
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

fn trading_fixture() -> (
    crate::core::strategy_ir::StrategyIr,
    StrategyExecutionConfig,
    Vec<Bar>,
    DatasetManifest,
) {
    let mut definition = GeneralStrategyBuilder::new("monte-carlo", "test")
        .definition()
        .clone();
    let always = Condition::Compare {
        left: Operand::Price {
            field: crate::core::strategy_ir::PriceField::Close,
            bars_ago: 0,
        },
        op: CompareOp::Greater,
        right: Operand::Constant(0.0),
    };
    definition.long.enabled = true;
    definition.long.entry = always.clone();
    definition.long.exit = always;
    let strategy = crate::core::strategy_ir::StrategyIr::build(&definition).unwrap();
    let config =
        StrategyExecutionConfig::build(&ExecutionSettings::conservative_defaults()).unwrap();
    let bars = bars();
    let manifest = manifest(&bars);
    (strategy, config, bars, manifest)
}

fn monte_carlo_request(
    strategy: &crate::core::strategy_ir::StrategyIr,
    config: &StrategyExecutionConfig,
    bars: &[Bar],
    manifest: &DatasetManifest,
    root_seed: u64,
) -> RetestExecutionRequest {
    let quarantine = HoldoutQuarantine::new(&manifest.dataset_id, "f".repeat(64), 10, 2).unwrap();
    RetestExecutionRequest::seal(
        strategy,
        config,
        manifest,
        bars,
        quarantine.lease(StageAccess::Robustness).unwrap(),
        ObservationRole::OutOfSample,
        "net_profit",
        root_seed,
    )
    .unwrap()
}

#[test]
fn canonical_trade_monte_carlo_executes_all_three_families_with_replayable_distributions() {
    let (strategy, config, bars, manifest) = trading_fixture();
    let settings = TradeMonteCarloConfig {
        seed: 44,
        trials: 32,
        trade_skip_bps: 2_500,
    };
    let artifact = execute_trade_monte_carlo(
        monte_carlo_request(&strategy, &config, &bars, &manifest, 9),
        settings,
        17,
    )
    .unwrap();

    artifact.verify().unwrap();
    assert_eq!(artifact.artifact_id().len(), 64);
    assert_eq!(artifact.candidate_id(), strategy.strategy_id());
    assert_eq!(artifact.run_id().len(), 64);
    assert_eq!(artifact.report_id().len(), 64);
    assert_eq!(artifact.dataset_id(), manifest.dataset_id);
    assert_eq!(artifact.config_id(), config.config_id());
    assert_eq!(artifact.root_seed(), 9);
    assert_eq!(artifact.seed(), 44);
    assert_eq!(artifact.evaluations_n(), 17);
    assert_eq!(artifact.families().len(), 3);
    assert!(artifact.trade_count() >= 2);
    for family in artifact.families() {
        assert_eq!(family.samples().len(), 32);
        assert!(family.samples()[0].net_profit().is_finite());
        assert!(family.samples()[0].max_drawdown().is_finite());
        assert_eq!(family.net_profit().confidence_level_bps(), 9_000);
        assert_eq!(family.max_drawdown().confidence_level_bps(), 9_000);
        assert!(family.net_profit().p05().is_finite());
        assert!(family.net_profit().median().is_finite());
        assert!(family.net_profit().p95().is_finite());
    }
    assert_ne!(
        artifact.families()[0].component_seed(),
        artifact.families()[1].component_seed()
    );
    assert_eq!(
        artifact,
        replay_trade_monte_carlo(
            monte_carlo_request(&strategy, &config, &bars, &manifest, 9),
            &artifact,
        )
        .unwrap()
    );
}

#[test]
fn trade_monte_carlo_rejects_unbounded_undefined_foreign_and_tampered_evidence() {
    let (strategy, config, bars, manifest) = trading_fixture();
    let request = || monte_carlo_request(&strategy, &config, &bars, &manifest, 9);
    for settings in [
        TradeMonteCarloConfig {
            seed: 1,
            trials: 0,
            trade_skip_bps: 100,
        },
        TradeMonteCarloConfig {
            seed: 1,
            trials: MAX_MONTE_CARLO_TRIALS + 1,
            trade_skip_bps: 100,
        },
        TradeMonteCarloConfig {
            seed: 1,
            trials: 8,
            trade_skip_bps: 10_000,
        },
    ] {
        assert!(execute_trade_monte_carlo(request(), settings, 1).is_err());
    }

    let artifact = execute_trade_monte_carlo(
        request(),
        TradeMonteCarloConfig {
            seed: 7,
            trials: 8,
            trade_skip_bps: 2_000,
        },
        2,
    )
    .unwrap();
    let mut tampered: serde_json::Value =
        serde_json::from_slice(&artifact.to_json_vec().unwrap()).unwrap();
    tampered["seed"] = 8.into();
    assert!(
        TradeMonteCarloArtifact::from_json_slice(&serde_json::to_vec(&tampered).unwrap()).is_err()
    );

    let mut other_settings = config.to_input();
    other_settings.warmup_bars = 1;
    let other_config = StrategyExecutionConfig::build(&other_settings).unwrap();
    assert!(
        replay_trade_monte_carlo(
            monte_carlo_request(&strategy, &other_config, &bars, &manifest, 9),
            &artifact,
        )
        .is_err()
    );

    let (empty_strategy, empty_config, empty_bars, empty_manifest) = fixture();
    assert!(
        execute_trade_monte_carlo(
            monte_carlo_request(
                &empty_strategy,
                &empty_config,
                &empty_bars,
                &empty_manifest,
                9,
            ),
            TradeMonteCarloConfig {
                seed: 1,
                trials: 8,
                trade_skip_bps: 100,
            },
            1,
        )
        .is_err()
    );
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

fn study_fixture(
    count: usize,
) -> (
    crate::core::strategy_ir::StrategyIr,
    StrategyExecutionConfig,
    Vec<Bar>,
    DatasetManifest,
) {
    let mut definition = GeneralStrategyBuilder::new("study", "test")
        .definition()
        .clone();
    definition.parameters = vec![StrategyParameter {
        id: "lookback".into(),
        value: ParamValue::Int(2),
        range: Some(ParamRange::Int { min: 2, max: 4 }),
    }];
    let strategy = crate::core::strategy_ir::StrategyIr::build(&definition).unwrap();
    let config =
        StrategyExecutionConfig::build(&ExecutionSettings::conservative_defaults()).unwrap();
    let bars = (0..count)
        .map(|index| {
            let open = 100.0 + index as f64;
            Bar {
                timestamp: format!("2026-03-{:02}T00:00:00Z", index + 1),
                open,
                high: open + 2.0,
                low: open - 1.0,
                close: open + 1.0,
                volume: 100.0,
            }
        })
        .collect::<Vec<_>>();
    let manifest = manifest(&bars);
    (strategy, config, bars, manifest)
}

fn study_lease(manifest: &DatasetManifest, bars: usize) -> SearchDataLease {
    HoldoutQuarantine::new(&manifest.dataset_id, "f".repeat(64), bars + 4, 4)
        .unwrap()
        .lease(StageAccess::Robustness)
        .unwrap()
}

fn candidate_batch(strategy: &crate::core::strategy_ir::StrategyIr) -> SearchBatch {
    let space = SearchSpace::new(
        strategy.clone(),
        vec![
            ParameterDomain::new(
                "lookback",
                vec![ParamValue::Int(2), ParamValue::Int(3), ParamValue::Int(4)],
            )
            .unwrap(),
        ],
    )
    .unwrap();
    generate_candidates(&space, SearchMethod::Grid, 3).unwrap()
}

#[test]
fn every_written_oos_scheme_executes_exact_content_and_proves_membership_seams() {
    let (strategy, config, bars, manifest) = study_fixture(24);
    let schemes = [
        OosScheme::Leading { oos_bars: 4 },
        OosScheme::Trailing { oos_bars: 4 },
        OosScheme::Interleaved {
            in_sample_bars: 4,
            oos_bars: 2,
        },
        OosScheme::Disjoint {
            windows: vec![4..6, 14..17],
        },
    ];
    for scheme in schemes {
        let result = execute_oos_scheme(
            &strategy,
            &config,
            &manifest,
            &bars,
            study_lease(&manifest, bars.len()),
            OosExecutionSpec {
                scheme,
                purge_bars: 1,
                embargo_bars: 1,
                metric_id: "total_return".into(),
                root_seed: 91,
            },
        )
        .unwrap();
        assert_eq!(result.source_dataset_id(), manifest.dataset_id);
        assert_eq!(result.membership().len(), bars.len());
        assert!(
            result
                .executed_partitions()
                .iter()
                .all(|partition| !partition.run_id.is_empty()
                    && !partition.report_id.is_empty()
                    && partition.score.is_finite()
                    && matches!(
                        partition.role,
                        SampleRole::InSample | SampleRole::OutOfSample
                    ))
        );
        let mut seen = std::collections::BTreeSet::new();
        for member in result.membership() {
            assert!(seen.insert(member.source_index));
            assert_eq!(member.timestamp, bars[member.source_index].timestamp);
        }
        assert_eq!(seen.len(), bars.len());
        for seam in result.seams() {
            assert!(seam.purged.end <= seam.oos.start || seam.purged.is_empty());
            assert!(seam.oos.end <= seam.embargoed.start || seam.embargoed.is_empty());
        }
    }
}

#[test]
fn rolling_and_anchored_walk_forward_reoptimize_on_exact_is_then_execute_oos() {
    let (strategy, config, bars, manifest) = study_fixture(30);
    let candidates = candidate_batch(&strategy);
    for anchored in [false, true] {
        let result = execute_walk_forward_optimization(
            &config,
            &manifest,
            &bars,
            study_lease(&manifest, bars.len()),
            &candidates,
            WalkForwardOptimizationSpec {
                config: WalkForwardConfig {
                    train_bars: 10,
                    test_bars: 4,
                    step_bars: 4,
                    purge_bars: 1,
                    embargo_bars: 1,
                    anchored,
                },
                minimum_windows: 2,
                metric_id: "total_return".into(),
                direction: ObjectiveDirection::Maximize,
                root_seed: 17,
            },
        )
        .unwrap();
        assert!(result.windows().len() >= 2);
        assert_eq!(
            result.degradation_distribution().len(),
            result.windows().len()
        );
        assert_eq!(
            result.concatenated_oos().report_ids.len(),
            result.windows().len()
        );
        for window in result.windows() {
            assert_eq!(window.evaluations_n, candidates.evaluations_n);
            assert!(!window.selected_candidate_id.is_empty());
            assert_eq!(window.in_sample.role, SampleRole::InSample);
            assert_eq!(window.out_of_sample.role, SampleRole::OutOfSample);
            assert!(window.in_sample.indices.iter().all(|index| {
                !window.out_of_sample.indices.contains(index)
                    && !window.purged.indices.contains(index)
                    && !window.embargoed.indices.contains(index)
            }));
            assert!(window.in_sample_report_id.len() == 64 && window.oos_report_id.len() == 64);
        }
    }
}

#[test]
fn calendar_walk_forward_executes_exact_irregular_membership_without_leakage_and_replays() {
    let (strategy, config, mut bars, _) = study_fixture(11);
    let timestamps = [
        "2026-01-01T14:30:00Z",
        "2026-01-02T14:30:00Z",
        "2026-01-05T14:30:00Z",
        "2026-01-06T14:30:00Z",
        "2026-01-10T14:30:00Z",
        "2026-01-11T14:30:00Z",
        "2026-01-15T14:30:00Z",
        "2026-01-16T14:30:00Z",
        "2026-01-20T14:30:00Z",
        "2026-01-21T14:30:00Z",
        "2026-01-25T14:30:00Z",
    ];
    for (bar, timestamp) in bars.iter_mut().zip(timestamps) {
        bar.timestamp = timestamp.into();
    }
    let manifest = manifest(&bars);
    let candidates = candidate_batch(&strategy);
    let execute = |anchored| {
        execute_calendar_walk_forward_optimization(
            &config,
            &manifest,
            &bars,
            study_lease(&manifest, bars.len()),
            &candidates,
            CalendarWalkForwardOptimizationSpec {
                config: CalendarWalkForwardConfig {
                    train_seconds: 4 * 86_400,
                    test_seconds: 4 * 86_400,
                    step_seconds: 5 * 86_400,
                    purge_seconds: 86_400,
                    embargo_seconds: 86_400,
                    anchored,
                },
                minimum_windows: 2,
                metric_id: "total_return".into(),
                direction: ObjectiveDirection::Maximize,
                root_seed: 117,
            },
        )
    };
    let rolling = execute(false).unwrap();
    let anchored = execute(true).unwrap();
    assert_eq!(rolling, execute(false).unwrap());
    assert_ne!(rolling.artifact_id(), anchored.artifact_id());
    assert_eq!(rolling.windows()[0].in_sample.indices, vec![0, 1]);
    assert_eq!(rolling.windows()[0].purged.indices, vec![2]);
    assert_eq!(rolling.windows()[0].embargoed.indices, vec![3]);
    assert_eq!(rolling.windows()[0].out_of_sample.indices, vec![4]);
    assert_eq!(rolling.windows()[1].in_sample.indices, vec![3]);
    assert_eq!(anchored.windows()[1].in_sample.indices, vec![0, 1, 2, 3]);
    assert_eq!(
        rolling,
        ExecutedCalendarWalkForward::from_json_slice(&rolling.to_json_vec().unwrap()).unwrap()
    );
    for window in rolling.windows() {
        let mut exact = std::collections::BTreeSet::new();
        for membership in [
            &window.in_sample,
            &window.purged,
            &window.embargoed,
            &window.out_of_sample,
        ] {
            for index in &membership.indices {
                assert!(exact.insert(*index));
            }
        }
        assert_eq!(window.in_sample_run_id.len(), 64);
        assert_eq!(window.oos_run_id.len(), 64);
    }
    assert_eq!(
        rolling.concatenated_oos().scores.len(),
        rolling.degradation_distribution().len()
    );
    let store = RetestEvidenceStore::open_in_memory().unwrap();
    store.persist_calendar_walk_forward(&rolling, 1).unwrap();
    let page = store
        .query_studies(&StudyArtifactQuery {
            source_dataset_id: manifest.dataset_id.clone(),
            kind: Some(StudyArtifactKind::CalendarWalkForward),
            after_sequence: None,
            limit: 1,
        })
        .unwrap();
    assert_eq!(page.records.len(), 1);
    assert!(matches!(
        &page.records[0].artifact,
        StudyArtifact::CalendarWalkForward(value) if value == &rolling
    ));
}

#[test]
fn calendar_walk_forward_rejects_timestamp_and_window_failures_before_execution() {
    let (strategy, config, bars, _) = study_fixture(8);
    let candidates = candidate_batch(&strategy);
    let execute = |bars: &[Bar], calendar: CalendarWalkForwardConfig| {
        let manifest = manifest(bars);
        execute_calendar_walk_forward_optimization(
            &config,
            &manifest,
            bars,
            study_lease(&manifest, bars.len()),
            &candidates,
            CalendarWalkForwardOptimizationSpec {
                config: calendar,
                minimum_windows: 2,
                metric_id: "total_return".into(),
                direction: ObjectiveDirection::Maximize,
                root_seed: 1,
            },
        )
    };
    let calendar = CalendarWalkForwardConfig {
        train_seconds: 2 * 86_400,
        test_seconds: 86_400,
        step_seconds: 86_400,
        purge_seconds: 0,
        embargo_seconds: 0,
        anchored: false,
    };
    for replacement in ["bad", "2026-03-31T00:00:00Z", "2026-03-01T00:00:00Z"] {
        let mut invalid = bars.clone();
        invalid[1].timestamp = replacement.into();
        assert!(execute(&invalid, calendar).is_err());
    }
    assert!(
        execute(
            &bars,
            CalendarWalkForwardConfig {
                test_seconds: MAX_CALENDAR_WINDOW_SECONDS + 1,
                ..calendar
            },
        )
        .is_err()
    );
}

#[test]
fn walk_forward_matrix_is_bounded_deterministic_and_refuses_insufficient_or_invalid_evidence() {
    let (strategy, config, bars, manifest) = study_fixture(30);
    let candidates = candidate_batch(&strategy);
    let dimensions = vec![(8, 3), (10, 4), (12, 3)];
    let execute = |dimensions: Vec<(usize, usize)>| {
        execute_walk_forward_matrix(
            &config,
            &manifest,
            &bars,
            study_lease(&manifest, bars.len()),
            &candidates,
            WalkForwardMatrixSpec {
                dimensions,
                step_bars: 4,
                purge_bars: 1,
                embargo_bars: 1,
                anchored: false,
                minimum_windows: 2,
                metric_id: "total_return".into(),
                direction: ObjectiveDirection::Maximize,
                root_seed: 27,
            },
        )
    };
    let matrix = execute(dimensions.clone()).unwrap();
    assert_eq!(matrix.cells().len(), dimensions.len());
    assert_eq!(matrix, execute(dimensions).unwrap());
    assert!(matrix.cells().windows(2).all(|pair| {
        (pair[0].train_bars, pair[0].test_bars) < (pair[1].train_bars, pair[1].test_bars)
    }));

    assert!(execute(vec![(28, 4)]).is_err());
    assert!(execute(vec![(8, 3); MAX_WALK_FORWARD_MATRIX_CELLS + 1]).is_err());

    let wrong_partition = HoldoutQuarantine::new("a".repeat(64), "f".repeat(64), 34, 4)
        .unwrap()
        .lease(StageAccess::Robustness)
        .unwrap();
    assert!(
        execute_walk_forward_optimization(
            &config,
            &manifest,
            &bars,
            wrong_partition,
            &candidates,
            WalkForwardOptimizationSpec {
                config: WalkForwardConfig {
                    train_bars: 10,
                    test_bars: 4,
                    step_bars: 4,
                    purge_bars: 1,
                    embargo_bars: 1,
                    anchored: false,
                },
                minimum_windows: 2,
                metric_id: "total_return".into(),
                direction: ObjectiveDirection::Maximize,
                root_seed: 17,
            },
        )
        .is_err()
    );

    let undefined = execute_walk_forward_optimization(
        &config,
        &manifest,
        &bars,
        study_lease(&manifest, bars.len()),
        &candidates,
        WalkForwardOptimizationSpec {
            config: WalkForwardConfig {
                train_bars: 10,
                test_bars: 4,
                step_bars: 4,
                purge_bars: 1,
                embargo_bars: 1,
                anchored: false,
            },
            minimum_windows: 2,
            metric_id: "profit_factor".into(),
            direction: ObjectiveDirection::Maximize,
            root_seed: 17,
        },
    );
    assert!(undefined.is_err());
}

fn durable_study_artifacts() -> (
    ExecutedOosScheme,
    ExecutedWalkForward,
    ExecutedWalkForwardMatrix,
    TradeMonteCarloArtifact,
) {
    let (strategy, config, bars, manifest) = study_fixture(30);
    let candidates = candidate_batch(&strategy);
    let oos = execute_oos_scheme(
        &strategy,
        &config,
        &manifest,
        &bars,
        study_lease(&manifest, bars.len()),
        OosExecutionSpec {
            scheme: OosScheme::Trailing { oos_bars: 5 },
            purge_bars: 1,
            embargo_bars: 1,
            metric_id: "total_return".into(),
            root_seed: 31,
        },
    )
    .unwrap();
    let walk_forward = execute_walk_forward_optimization(
        &config,
        &manifest,
        &bars,
        study_lease(&manifest, bars.len()),
        &candidates,
        WalkForwardOptimizationSpec {
            config: WalkForwardConfig {
                train_bars: 10,
                test_bars: 4,
                step_bars: 4,
                purge_bars: 1,
                embargo_bars: 1,
                anchored: false,
            },
            minimum_windows: 2,
            metric_id: "total_return".into(),
            direction: ObjectiveDirection::Maximize,
            root_seed: 32,
        },
    )
    .unwrap();
    let matrix = execute_walk_forward_matrix(
        &config,
        &manifest,
        &bars,
        study_lease(&manifest, bars.len()),
        &candidates,
        WalkForwardMatrixSpec {
            dimensions: vec![(8, 3), (10, 4)],
            step_bars: 4,
            purge_bars: 1,
            embargo_bars: 1,
            anchored: false,
            minimum_windows: 2,
            metric_id: "total_return".into(),
            direction: ObjectiveDirection::Maximize,
            root_seed: 33,
        },
    )
    .unwrap();
    let (mc_strategy, mc_config, mc_bars, mc_manifest) = trading_fixture();
    let monte_carlo = execute_trade_monte_carlo(
        monte_carlo_request(&mc_strategy, &mc_config, &mc_bars, &mc_manifest, 34),
        TradeMonteCarloConfig {
            seed: 35,
            trials: 8,
            trade_skip_bps: 2_000,
        },
        3,
    )
    .unwrap();
    (oos, walk_forward, matrix, monte_carlo)
}

#[test]
fn study_artifacts_are_content_addressed_bounded_round_trippable_and_tamper_evident() {
    let (oos, walk_forward, matrix, _) = durable_study_artifacts();
    assert_eq!(oos.artifact_id().len(), 64);
    assert_eq!(walk_forward.artifact_id().len(), 64);
    assert_eq!(matrix.artifact_id().len(), 64);
    oos.verify().unwrap();
    walk_forward.verify().unwrap();
    matrix.verify().unwrap();
    assert_eq!(
        oos,
        ExecutedOosScheme::from_json_slice(&oos.to_json_vec().unwrap()).unwrap()
    );
    assert_eq!(
        walk_forward,
        ExecutedWalkForward::from_json_slice(&walk_forward.to_json_vec().unwrap()).unwrap()
    );
    assert_eq!(
        matrix,
        ExecutedWalkForwardMatrix::from_json_slice(&matrix.to_json_vec().unwrap()).unwrap()
    );

    let mut tampered: serde_json::Value =
        serde_json::from_slice(&walk_forward.to_json_vec().unwrap()).unwrap();
    tampered["source_dataset_id"] = "0".repeat(64).into();
    assert!(ExecutedWalkForward::from_json_slice(&serde_json::to_vec(&tampered).unwrap()).is_err());
    assert!(ExecutedOosScheme::from_json_slice(&vec![b'x'; MAX_ARTIFACT_BYTES + 1]).is_err());
}

#[test]
fn durable_study_store_survives_restart_rejects_duplicates_tamper_and_unbounded_queries() {
    let (oos, walk_forward, matrix, monte_carlo) = durable_study_artifacts();
    let path = std::env::temp_dir().join(format!(
        "typhoon-study-evidence-{}-{}.sqlite",
        std::process::id(),
        oos.artifact_id()
    ));
    {
        let store = RetestEvidenceStore::open(&path).unwrap();
        store.persist_oos(&oos, 1).unwrap();
        store.persist_walk_forward(&walk_forward, 2).unwrap();
        store.persist_walk_forward_matrix(&matrix, 3).unwrap();
        store.persist_trade_monte_carlo(&monte_carlo, 4).unwrap();
        assert!(matches!(
            store.persist_oos(&oos, 5),
            Err(RetestError::DuplicateLineage)
        ));
        assert!(store.test_only_update_study(oos.artifact_id()).is_err());
        assert!(store.test_only_delete_study(oos.artifact_id()).is_err());
    }
    {
        let store = RetestEvidenceStore::open(&path).unwrap();
        let first = store
            .query_studies(&StudyArtifactQuery {
                source_dataset_id: oos.source_dataset_id().to_string(),
                kind: None,
                after_sequence: None,
                limit: 1,
            })
            .unwrap();
        assert_eq!(first.records.len(), 1);
        assert!(first.has_more);
        assert_eq!(first.records[0].artifact_id, oos.artifact_id());
        assert!(matches!(first.records[0].artifact, StudyArtifact::Oos(_)));
        assert!(
            store
                .query_studies(&StudyArtifactQuery {
                    source_dataset_id: oos.source_dataset_id().to_string(),
                    kind: None,
                    after_sequence: None,
                    limit: MAX_RETEST_QUERY_LIMIT + 1,
                })
                .is_err()
        );
        assert!(
            store
                .explain_study_query(&StudyArtifactQuery {
                    source_dataset_id: oos.source_dataset_id().to_string(),
                    kind: None,
                    after_sequence: None,
                    limit: 1,
                })
                .unwrap()
                .iter()
                .any(|line| line.contains("idx_study_dataset_sequence"))
        );
        assert!(
            store
                .explain_study_query(&StudyArtifactQuery {
                    source_dataset_id: oos.source_dataset_id().to_string(),
                    kind: Some(StudyArtifactKind::Oos),
                    after_sequence: None,
                    limit: 1,
                })
                .unwrap()
                .iter()
                .any(|line| line.contains("idx_study_dataset_kind_sequence"))
        );
        store
            .test_only_tamper_study_json(oos.artifact_id())
            .unwrap();
        assert!(
            store
                .query_studies(&StudyArtifactQuery {
                    source_dataset_id: oos.source_dataset_id().to_string(),
                    kind: None,
                    after_sequence: None,
                    limit: 1,
                })
                .is_err()
        );
    }
    let _ = std::fs::remove_file(path);
}

#[test]
fn disk_backed_worker_persists_completed_retest_across_restart() {
    let path = std::env::temp_dir().join(format!(
        "typhoon-retest-worker-{}-{}.sqlite",
        std::process::id(),
        execution_request("net_profit").request_id()
    ));
    let worker = RetestWorker::spawn(&path, 1, 8).unwrap();
    worker
        .try_submit(RetestWorkerJob {
            request_id: 77,
            execution: execution_request("net_profit"),
            pipeline: pipeline(),
            evaluations_n: 3,
            parent_run_id: None,
            created_sequence: 77,
        })
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut candidate_id = None;
    while std::time::Instant::now() < deadline && candidate_id.is_none() {
        for event in worker.poll() {
            if let RetestWorkerEvent::Completed { result, .. } = event {
                candidate_id = Some(result.observation().candidate_id().to_string());
            }
        }
        std::thread::yield_now();
    }
    drop(worker);
    let store = RetestEvidenceStore::open(&path).unwrap();
    let page = store
        .query(&RetestEvidenceQuery {
            candidate_id: candidate_id.unwrap(),
            after_sequence: None,
            limit: 1,
        })
        .unwrap();
    assert_eq!(page.records.len(), 1);
    let _ = std::fs::remove_file(path);
}
