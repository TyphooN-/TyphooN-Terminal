use super::*;
use crate::broker::alpaca::Bar;
use crate::core::strategy_builder::GeneralStrategyBuilder;
use crate::core::strategy_cross_check::{
    COST_MULTIPLIER_2X_BPS, CrossCheckDatasetCase, CrossCheckKind, CrossCheckStudyArtifact,
    CrossCheckStudySpec, execute_cross_check_study,
};
use crate::core::strategy_dataset::{
    AdjustmentPolicy, CalendarPolicy, DatasetManifest, DatasetManifestInput, DatasetProvenance,
    DatasetQaPolicy,
};
use crate::core::strategy_ir::{
    CompareOp, Condition, ExecutionSettings, Operand, ParamRange, ParamValue, SlippageModel,
    SpreadModel, StrategyExecutionConfig, StrategyParameter,
};
use crate::core::strategy_metrics::{METRICS_SCHEMA_VERSION, MetricValue};
use crate::core::strategy_optimization::{
    HoldoutQuarantine, MAX_CALENDAR_WINDOW_SECONDS, MAX_MONTE_CARLO_TRIALS, ObjectiveDirection,
    ObservationRole, OosScheme, ParameterDomain, Percentile, RobustnessPipeline,
    RobustnessStageSpec, SampleRole, SearchBatch, SearchMethod, SearchSpace, StageAccess,
    StageVerdict, Threshold, WalkForwardConfig, generate_candidates,
};
use crate::core::strategy_parameter_field::{
    MAX_PARAMETER_FIELD_NEIGHBOURHOOD, MAX_PARAMETER_FIELD_RADIUS, MAX_PARAMETER_FIELD_SAMPLE,
    ParameterFieldPhase, ParameterFieldStudyArtifact, ParameterFieldStudySpec, PlateauVerdict,
    execute_parameter_field_study, replay_parameter_field_study,
};
use crate::core::strategy_perturbation::{
    MAX_PERTURBATION_COST_SCALE_BPS, MAX_PERTURBATION_JITTER_STEPS, MAX_PERTURBATION_NOISE_BPS,
    MAX_PERTURBATION_START_OFFSET, MAX_PERTURBATION_TRIALS_PER_FAMILY, PerturbationDetail,
    PerturbationFamily, PerturbationStudyArtifact, PerturbationStudySpec,
    execute_perturbation_study, replay_perturbation_study,
};
use crate::core::strategy_problem_recognition::{
    ProblemRecognitionArtifact, ProblemRecognitionPolicy, execute_problem_recognition,
    replay_problem_recognition,
};
use crate::core::strategy_significance::{
    SignificancePolicy, SignificanceStudyArtifact, execute_significance_study,
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

fn final_holdout_fixture(metric_id: &str) -> FinalHoldoutExecutionRequest {
    let (strategy, config, search_bars, _) = fixture();
    let holdout_bars = vec![
        Bar {
            timestamp: "2026-01-09T00:00:00Z".into(),
            open: 108.0,
            high: 110.0,
            low: 107.0,
            close: 109.0,
            volume: 100.0,
        },
        Bar {
            timestamp: "2026-01-10T00:00:00Z".into(),
            open: 109.0,
            high: 111.0,
            low: 108.0,
            close: 110.0,
            volume: 100.0,
        },
    ];
    let search_len = search_bars.len();
    let holdout_len = holdout_bars.len();
    let mut parent_bars = search_bars;
    parent_bars.extend(holdout_bars);
    let parent_manifest = manifest(&parent_bars);
    let search_manifest =
        DatasetManifest::build(&parent_manifest.to_input(), &parent_bars[..8]).unwrap();
    let holdout_manifest =
        DatasetManifest::build(&parent_manifest.to_input(), &parent_bars[8..]).unwrap();
    let quarantine = HoldoutQuarantine::new(
        &search_manifest.dataset_id,
        &holdout_manifest.dataset_id,
        search_len + holdout_len,
        holdout_len,
    )
    .unwrap();
    FinalHoldoutExecutionRequest::seal(
        &strategy,
        &config,
        &parent_manifest,
        &parent_bars,
        quarantine.burn("promotion decision").unwrap(),
        metric_id,
        41,
        17,
    )
    .unwrap()
}

#[test]
fn final_holdout_executes_exact_bars_and_persists_all_identity_evidence() {
    let store = RetestEvidenceStore::open_in_memory().unwrap();
    let request = final_holdout_fixture("net_profit");
    let search_dataset_id = request.search_dataset_id().to_string();
    let holdout_dataset_id = request.holdout_dataset_id().to_string();
    let completed = store.execute_final_holdout(request, 10).unwrap();

    completed.report().verify().unwrap();
    assert_eq!(completed.request_id().len(), 64);
    assert_eq!(completed.report().run_id(), completed.run_id());
    assert_eq!(completed.evaluations_n(), 17);
    assert_eq!(completed.metric_id(), "net_profit");
    assert!(completed.metric_value().is_finite());
    let page = store
        .query_final_holdouts(&FinalHoldoutQuery {
            search_dataset_id,
            after_sequence: None,
            limit: 1,
        })
        .unwrap();
    assert_eq!(page.records.len(), 1);
    assert!(!page.has_more);
    let record = &page.records[0];
    assert_eq!(record.holdout_dataset_id, holdout_dataset_id);
    assert_eq!(record.range, 8..10);
    assert_eq!(record.reason, "promotion decision");
    assert_eq!(record.strategy_id, completed.strategy_id());
    assert_eq!(record.config_id, completed.config_id());
    assert_eq!(record.seed, 41);
    assert_eq!(record.evaluations_n, 17);
    assert_eq!(record.run_id.as_deref(), Some(completed.run_id()));
    assert_eq!(
        record.report_id.as_deref(),
        Some(completed.report().report_id())
    );
    assert_eq!(record.outcome, FinalHoldoutOutcome::Succeeded);
    assert!(record.failure.is_none());

    let store = RetestEvidenceStore::open_in_memory().unwrap();
    let mut maximum_seed = final_holdout_fixture("net_profit");
    maximum_seed.seed = u64::MAX;
    maximum_seed.request_id = maximum_seed.compute_id();
    let search_dataset_id = maximum_seed.search_dataset_id().to_string();
    store.execute_final_holdout(maximum_seed, 11).unwrap();
    let page = store
        .query_final_holdouts(&FinalHoldoutQuery {
            search_dataset_id,
            after_sequence: None,
            limit: 1,
        })
        .unwrap();
    assert_eq!(page.records[0].seed, u64::MAX);
}

#[test]
fn final_holdout_rejects_wrong_range_content_foreign_inputs_and_tampering() {
    let mut request = final_holdout_fixture("net_profit");
    request.range.end += 1;
    assert!(
        RetestEvidenceStore::open_in_memory()
            .unwrap()
            .execute_final_holdout(request, 1)
            .is_err()
    );

    let mut request = final_holdout_fixture("net_profit");
    request.holdout_bars[0].close += 1.0;
    assert!(
        RetestEvidenceStore::open_in_memory()
            .unwrap()
            .execute_final_holdout(request, 1)
            .is_err()
    );

    let mut request = final_holdout_fixture("net_profit");
    request.strategy = trading_fixture().0;
    assert!(
        RetestEvidenceStore::open_in_memory()
            .unwrap()
            .execute_final_holdout(request, 1)
            .is_err()
    );

    let mut request = final_holdout_fixture("net_profit");
    let mut settings = request.config.to_input();
    settings.warmup_bars = 1;
    request.config = StrategyExecutionConfig::build(&settings).unwrap();
    assert!(
        RetestEvidenceStore::open_in_memory()
            .unwrap()
            .execute_final_holdout(request, 1)
            .is_err()
    );

    let mut request = final_holdout_fixture("net_profit");
    let mut foreign_search_bars = bars();
    foreign_search_bars[0].close += 0.5;
    request.search_manifest = manifest(&foreign_search_bars);
    assert!(
        RetestEvidenceStore::open_in_memory()
            .unwrap()
            .execute_final_holdout(request, 1)
            .is_err()
    );

    let mut request = final_holdout_fixture("net_profit");
    request.request_id.replace_range(0..1, "0");
    assert!(
        RetestEvidenceStore::open_in_memory()
            .unwrap()
            .execute_final_holdout(request, 1)
            .is_err()
    );
}

#[test]
fn duplicate_concurrent_final_holdout_attempts_cannot_both_execute() {
    let path = std::env::temp_dir().join(format!(
        "typhoon-final-holdout-race-{}-{}.sqlite",
        std::process::id(),
        final_holdout_fixture("net_profit").request_id()
    ));
    let barrier = Arc::new(std::sync::Barrier::new(2));
    let mut handles = Vec::new();
    for sequence in [1, 2] {
        let path = path.clone();
        let barrier = barrier.clone();
        handles.push(std::thread::spawn(move || {
            let store = RetestEvidenceStore::open(&path).unwrap();
            let request = final_holdout_fixture("net_profit");
            barrier.wait();
            store.execute_final_holdout(request, sequence)
        }));
    }
    let outcomes = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(outcomes.iter().filter(|outcome| outcome.is_ok()).count(), 1);
    assert_eq!(
        outcomes.iter().filter(|outcome| outcome.is_err()).count(),
        1
    );
    let store = RetestEvidenceStore::open(&path).unwrap();
    let query = FinalHoldoutQuery {
        search_dataset_id: final_holdout_fixture("net_profit")
            .search_dataset_id()
            .to_string(),
        after_sequence: None,
        limit: 2,
    };
    assert_eq!(store.query_final_holdouts(&query).unwrap().records.len(), 1);
    let _ = std::fs::remove_file(path);
}

#[test]
fn failed_or_crashed_final_holdout_attempt_remains_burned_across_restart() {
    let path = std::env::temp_dir().join(format!(
        "typhoon-final-holdout-restart-{}-{}.sqlite",
        std::process::id(),
        final_holdout_fixture("net_profit").request_id()
    ));
    {
        let store = RetestEvidenceStore::open(&path).unwrap();
        store
            .test_only_reserve_final_holdout(&final_holdout_fixture("net_profit"), 1)
            .unwrap();
    }
    let store = RetestEvidenceStore::open(&path).unwrap();
    assert!(
        store
            .execute_final_holdout(final_holdout_fixture("net_profit"), 2)
            .is_err()
    );

    let failed_path = std::env::temp_dir().join(format!(
        "typhoon-final-holdout-failed-{}-{}.sqlite",
        std::process::id(),
        final_holdout_fixture("profit_factor").request_id()
    ));
    let failed_store = RetestEvidenceStore::open(&failed_path).unwrap();
    assert!(
        failed_store
            .execute_final_holdout(final_holdout_fixture("profit_factor"), 3)
            .is_err()
    );
    let query = FinalHoldoutQuery {
        search_dataset_id: final_holdout_fixture("profit_factor")
            .search_dataset_id()
            .to_string(),
        after_sequence: None,
        limit: 1,
    };
    let failed = failed_store.query_final_holdouts(&query).unwrap();
    assert_eq!(failed.records[0].outcome, FinalHoldoutOutcome::Failed);
    assert!(
        failed.records[0]
            .failure
            .as_deref()
            .is_some_and(|value| !value.is_empty())
    );
    assert!(
        failed_store
            .execute_final_holdout(final_holdout_fixture("profit_factor"), 4)
            .is_err()
    );
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(failed_path);
}

#[test]
fn final_holdout_query_is_bounded_deterministic_immutable_and_indexed() {
    let store = RetestEvidenceStore::open_in_memory().unwrap();
    let request = final_holdout_fixture("net_profit");
    let search_dataset_id = request.search_dataset_id().to_string();
    store.execute_final_holdout(request, 5).unwrap();
    let query = FinalHoldoutQuery {
        search_dataset_id,
        after_sequence: None,
        limit: 1,
    };
    assert!(
        store
            .explain_final_holdout_query(&query)
            .unwrap()
            .iter()
            .any(|line| line.contains("idx_final_holdout_search_sequence"))
    );
    assert!(store.test_only_update_final_holdout().is_err());
    let request_id = store.query_final_holdouts(&query).unwrap().records[0]
        .request_id
        .clone();
    store
        .test_only_tamper_final_holdout_report(&request_id)
        .unwrap();
    assert!(store.query_final_holdouts(&query).is_err());
    assert!(
        store
            .query_final_holdouts(&FinalHoldoutQuery {
                limit: MAX_RETEST_QUERY_LIMIT + 1,
                ..query
            })
            .is_err()
    );
}

#[test]
fn final_holdout_persisted_identity_is_content_addressed_after_storage_tamper() {
    let store = RetestEvidenceStore::open_in_memory().unwrap();
    let request = final_holdout_fixture("net_profit");
    let search_dataset_id = request.search_dataset_id().to_string();
    let request_id = request.request_id().to_string();
    store.execute_final_holdout(request, 5).unwrap();
    store
        .test_only_tamper_final_holdout_identity(&request_id)
        .unwrap();
    assert!(
        store
            .query_final_holdouts(&FinalHoldoutQuery {
                search_dataset_id,
                after_sequence: None,
                limit: 1,
            })
            .is_err()
    );
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

fn bayesian_fixture(
    descending: bool,
) -> (
    crate::core::strategy_ir::StrategyIr,
    StrategyExecutionConfig,
    Vec<Bar>,
    DatasetManifest,
    SearchSpace,
) {
    let mut definition = GeneralStrategyBuilder::new("bayesian", "test")
        .definition()
        .clone();
    definition.parameters = vec![StrategyParameter {
        id: "threshold".into(),
        value: ParamValue::Float(100.0),
        range: Some(ParamRange::Float {
            min: 95.0,
            max: 115.0,
        }),
    }];
    definition.long.enabled = true;
    definition.long.entry = Condition::Compare {
        left: Operand::Price {
            field: crate::core::strategy_ir::PriceField::Close,
            bars_ago: 0,
        },
        op: CompareOp::Greater,
        right: Operand::Parameter("threshold".into()),
    };
    definition.long.exit = Condition::Compare {
        left: Operand::Price {
            field: crate::core::strategy_ir::PriceField::Close,
            bars_ago: 0,
        },
        op: CompareOp::Less,
        right: Operand::Parameter("threshold".into()),
    };
    let strategy = crate::core::strategy_ir::StrategyIr::build(&definition).unwrap();
    let config =
        StrategyExecutionConfig::build(&ExecutionSettings::conservative_defaults()).unwrap();
    let closes = if descending {
        vec![114.0, 111.0, 108.0, 105.0, 102.0, 99.0, 96.0, 93.0]
    } else {
        vec![96.0, 99.0, 102.0, 105.0, 108.0, 111.0, 114.0, 117.0]
    };
    let bars = closes
        .into_iter()
        .enumerate()
        .map(|(index, close)| Bar {
            timestamp: format!("2026-04-{:02}T00:00:00Z", index + 1),
            open: close - 1.0,
            high: close + 2.0,
            low: close - 2.0,
            close,
            volume: 100.0,
        })
        .collect::<Vec<_>>();
    let manifest = manifest(&bars);
    let space = SearchSpace::new(
        strategy.clone(),
        vec![
            ParameterDomain::new(
                "threshold",
                vec![95.0, 99.0, 103.0, 107.0, 111.0, 115.0]
                    .into_iter()
                    .map(ParamValue::Float)
                    .collect(),
            )
            .unwrap(),
        ],
    )
    .unwrap();
    (strategy, config, bars, manifest, space)
}

fn bayesian_spec() -> crate::core::strategy_bayesian::BayesianOptimizationSpec {
    crate::core::strategy_bayesian::BayesianOptimizationSpec {
        budget: 5,
        initial_design_size: 2,
        acquisition_pool_limit: 64,
        nearest_neighbors: 2,
        exploration_bps: 2_500,
        metric_id: "net_profit".into(),
        direction: ObjectiveDirection::Maximize,
        root_seed: 0x135,
    }
}

#[test]
fn adaptive_bayesian_study_replays_verified_observations_and_persists_immutably() {
    use crate::core::strategy_bayesian::{
        BayesianProposalKind, BayesianStudyArtifact, execute_bayesian_optimization,
    };

    let (_, config, bars, manifest, space) = bayesian_fixture(false);
    let execute = || {
        execute_bayesian_optimization(
            &config,
            &manifest,
            &bars,
            study_lease(&manifest, bars.len()),
            &space,
            bayesian_spec(),
        )
    };
    let artifact = execute().unwrap();
    artifact.verify().unwrap();
    assert_eq!(artifact, execute().unwrap());
    assert_eq!(artifact.evaluations_n(), 5);
    assert_eq!(artifact.decisions().len(), 5);
    assert_eq!(artifact.observations().len(), 5);
    assert!(
        artifact.decisions()[..2]
            .iter()
            .all(|decision| decision.kind == BayesianProposalKind::SeededDesign)
    );
    assert!(
        artifact.decisions()[2..]
            .iter()
            .all(|decision| decision.kind == BayesianProposalKind::Acquisition)
    );
    assert!(
        artifact
            .observations()
            .iter()
            .enumerate()
            .all(|(index, observation)| {
                observation.evaluation_n == index + 1
                    && observation.run_id.len() == 64
                    && observation.report_id.len() == 64
                    && observation.value.is_finite()
            })
    );
    let ids = artifact
        .decisions()
        .iter()
        .map(|decision| decision.candidate_id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(ids.len(), 5);
    assert_eq!(
        artifact,
        BayesianStudyArtifact::from_json_slice(&artifact.to_json_vec().unwrap()).unwrap()
    );

    let store = RetestEvidenceStore::open_in_memory().unwrap();
    store.persist_bayesian_study(&artifact, 9).unwrap();
    assert!(matches!(
        store.persist_bayesian_study(&artifact, 10),
        Err(RetestError::DuplicateLineage)
    ));
    let page = store
        .query_studies(&StudyArtifactQuery {
            source_dataset_id: manifest.dataset_id.clone(),
            kind: Some(StudyArtifactKind::BayesianOptimization),
            after_sequence: None,
            limit: 1,
        })
        .unwrap();
    assert!(matches!(
        &page.records[0].artifact,
        StudyArtifact::BayesianOptimization(value) if value == &artifact
    ));
}

#[test]
fn adaptive_bayesian_later_proposals_depend_on_canonical_report_values() {
    use crate::core::strategy_bayesian::execute_bayesian_optimization;

    let (_, rising_config, rising_bars, rising_manifest, rising_space) = bayesian_fixture(false);
    let (_, falling_config, falling_bars, falling_manifest, falling_space) = bayesian_fixture(true);
    let rising = execute_bayesian_optimization(
        &rising_config,
        &rising_manifest,
        &rising_bars,
        study_lease(&rising_manifest, rising_bars.len()),
        &rising_space,
        bayesian_spec(),
    )
    .unwrap();
    let falling = execute_bayesian_optimization(
        &falling_config,
        &falling_manifest,
        &falling_bars,
        study_lease(&falling_manifest, falling_bars.len()),
        &falling_space,
        bayesian_spec(),
    )
    .unwrap();

    let rising_seeded = rising.decisions()[..2]
        .iter()
        .map(|decision| &decision.assignments)
        .collect::<Vec<_>>();
    let falling_seeded = falling.decisions()[..2]
        .iter()
        .map(|decision| &decision.assignments)
        .collect::<Vec<_>>();
    assert_eq!(rising_seeded, falling_seeded);
    assert_ne!(
        rising.observations()[..2]
            .iter()
            .map(|observation| observation.value.to_bits())
            .collect::<Vec<_>>(),
        falling.observations()[..2]
            .iter()
            .map(|observation| observation.value.to_bits())
            .collect::<Vec<_>>()
    );
    assert_ne!(
        rising.decisions()[2].assignments,
        falling.decisions()[2].assignments
    );
}

#[test]
fn adaptive_bayesian_fails_closed_on_bounds_holdout_undefined_and_tampering() {
    use crate::core::strategy_bayesian::{BayesianStudyArtifact, execute_bayesian_optimization};

    let (_, config, bars, manifest, space) = bayesian_fixture(false);
    let execute = |spec| {
        execute_bayesian_optimization(
            &config,
            &manifest,
            &bars,
            study_lease(&manifest, bars.len()),
            &space,
            spec,
        )
    };
    for budget in [0, space.combinations() + 1, MAX_TRIAL_BUDGET + 1] {
        let mut spec = bayesian_spec();
        spec.budget = budget;
        assert!(execute(spec).is_err());
    }
    let mut undefined = bayesian_spec();
    undefined.metric_id = "profit_factor".into();
    assert!(execute(undefined).is_err());

    let foreign = HoldoutQuarantine::new("a".repeat(64), "f".repeat(64), 12, 4)
        .unwrap()
        .lease(StageAccess::Search)
        .unwrap();
    assert!(
        execute_bayesian_optimization(&config, &manifest, &bars, foreign, &space, bayesian_spec(),)
            .is_err()
    );
    assert!(
        HoldoutQuarantine::new(&manifest.dataset_id, "f".repeat(64), 12, 4)
            .unwrap()
            .lease(StageAccess::FinalReview)
            .is_err()
    );

    let artifact = execute(bayesian_spec()).unwrap();
    let mut tampered: serde_json::Value =
        serde_json::from_slice(&artifact.to_json_vec().unwrap()).unwrap();
    tampered["observations"][0]["value"] = serde_json::json!(999999.0);
    assert!(
        BayesianStudyArtifact::from_json_slice(&serde_json::to_vec(&tampered).unwrap()).is_err()
    );
    let mut duplicate: serde_json::Value =
        serde_json::from_slice(&artifact.to_json_vec().unwrap()).unwrap();
    duplicate["decisions"][1]["candidate_id"] = duplicate["decisions"][0]["candidate_id"].clone();
    assert!(
        BayesianStudyArtifact::from_json_slice(&serde_json::to_vec(&duplicate).unwrap()).is_err()
    );
}

/// An oscillating series, so a threshold is crossed repeatedly from the first bars onward: every
/// perturbation family — jittered threshold, scaled cost, repriced bar, later start — can move the
/// realized trade set instead of only the fill prices.
fn perturbation_bars() -> Vec<Bar> {
    [
        96.0, 104.0, 112.0, 108.0, 100.0, 116.0, 106.0, 98.0, 114.0, 102.0, 110.0, 118.0, 94.0,
        120.0,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, close)| Bar {
        timestamp: format!("2026-05-{:02}T00:00:00Z", index + 1),
        open: close - 1.0,
        high: close + 2.0,
        low: close - 2.0,
        close,
        volume: 100.0,
    })
    .collect()
}

/// A parameterised long-only fixture: entry is gated by the jittered threshold and the exit is
/// unconditional, so trade count, fill prices and costs all move with a perturbation.
fn perturbation_fixture(
    base_threshold: f64,
    priced_costs: bool,
) -> (
    StrategyExecutionConfig,
    Vec<Bar>,
    DatasetManifest,
    SearchSpace,
) {
    let mut definition = GeneralStrategyBuilder::new("perturbation", "test")
        .definition()
        .clone();
    definition.parameters = vec![StrategyParameter {
        id: "threshold".into(),
        value: ParamValue::Float(base_threshold),
        range: Some(ParamRange::Float {
            min: 95.0,
            max: 115.0,
        }),
    }];
    definition.long.enabled = true;
    definition.long.entry = Condition::Compare {
        left: Operand::Price {
            field: crate::core::strategy_ir::PriceField::Close,
            bars_ago: 0,
        },
        op: CompareOp::Greater,
        right: Operand::Parameter("threshold".into()),
    };
    definition.long.exit = Condition::Compare {
        left: Operand::Price {
            field: crate::core::strategy_ir::PriceField::Close,
            bars_ago: 0,
        },
        op: CompareOp::Greater,
        right: Operand::Constant(0.0),
    };
    let strategy = crate::core::strategy_ir::StrategyIr::build(&definition).unwrap();
    let mut settings = ExecutionSettings::conservative_defaults();
    if priced_costs {
        settings.slippage = SlippageModel::FixedPriceDistance { distance: 0.05 };
        settings.spread = SpreadModel::Constant { price_units: 0.02 };
    }
    let config = StrategyExecutionConfig::build(&settings).unwrap();
    let bars = perturbation_bars();
    let manifest = manifest(&bars);
    let space = SearchSpace::new(
        strategy,
        vec![
            ParameterDomain::new(
                "threshold",
                vec![95.0, 99.0, 103.0, 107.0, 111.0, 115.0]
                    .into_iter()
                    .map(ParamValue::Float)
                    .collect(),
            )
            .unwrap(),
        ],
    )
    .unwrap();
    (config, bars, manifest, space)
}

fn perturbation_spec() -> PerturbationStudySpec {
    PerturbationStudySpec {
        trials_per_family: 4,
        jitter_steps: 2,
        cost_scale_bps: 20_000,
        data_noise_bps: 50,
        maximum_start_offset: 4,
        metric_id: "net_profit".into(),
        evaluations_n: 12,
        root_seed: 0x4d34,
    }
}

#[test]
fn perturbation_study_bounds_every_supported_family_and_persists_immutably() {
    let (config, bars, manifest, space) = perturbation_fixture(103.0, true);
    let source_manifest = manifest.clone();
    let execute = || {
        execute_perturbation_study(
            &config,
            &manifest,
            &bars,
            study_lease(&manifest, bars.len()),
            &space,
            perturbation_spec(),
        )
    };
    let artifact = execute().unwrap();
    artifact.verify().unwrap();
    assert_eq!(artifact, execute().unwrap());
    assert_eq!(
        artifact,
        replay_perturbation_study(
            &config,
            &manifest,
            &bars,
            study_lease(&manifest, bars.len()),
            &space,
            &artifact,
        )
        .unwrap()
    );
    assert_eq!(
        artifact,
        PerturbationStudyArtifact::from_json_slice(&artifact.to_json_vec().unwrap()).unwrap()
    );
    // Positive control for the re-sealing negative tests below: untouched evidence survives a
    // recomputed identity, so those tests fail on the invariant and not on the decode.
    assert_eq!(
        artifact,
        PerturbationStudyArtifact::resealed_from_json(&artifact.to_json_vec().unwrap()).unwrap()
    );

    // Immutable baseline evidence, bound to the exact sealed run it came from.
    assert_eq!(artifact.baseline_candidate_id(), space.base().strategy_id());
    assert_eq!(artifact.baseline_run_id().len(), 64);
    assert_eq!(artifact.baseline_report_id().len(), 64);
    assert_eq!(artifact.source_dataset_id(), manifest.dataset_id);
    assert_eq!(artifact.config_id(), config.config_id());
    assert_eq!(artifact.range(), 0..bars.len());
    assert_eq!(artifact.metric_id(), "net_profit");
    assert_eq!(artifact.evaluations_n(), 12);
    assert!(artifact.baseline_value().is_finite());
    // The source evidence a perturbation was derived from is still exactly what it was.
    assert_eq!(manifest, source_manifest);
    source_manifest.verify(&bars).unwrap();

    assert_eq!(artifact.families().len(), 4);
    let mut identities = std::collections::BTreeSet::new();
    let mut dispersed = 0;
    for family in artifact.families() {
        assert!(family.unsupported_reason().is_none());
        assert_eq!(family.trials().len(), 4);
        assert_ne!(family.component_seed(), artifact.root_seed());
        let percentiles = family.percentiles().unwrap();
        assert_eq!(percentiles.confidence_level_bps(), 9_000);
        assert!(percentiles.p05() <= percentiles.median());
        assert!(percentiles.median() <= percentiles.p95());
        if percentiles.p05() < percentiles.p95() {
            dispersed += 1;
        }
        for (index, trial) in family.trials().iter().enumerate() {
            assert_eq!(trial.trial_n, index + 1);
            assert_eq!(trial.request_id.len(), 64);
            assert_eq!(trial.run_id.len(), 64);
            assert_eq!(trial.report_id.len(), 64);
            assert!(trial.value.is_finite());
            assert!(identities.insert(trial.report_id.clone()));
            assert!(identities.insert(trial.run_id.clone()));
        }
    }
    // Every family is a distribution, not one score: each one actually moved the metric.
    assert_eq!(dispersed, 4);
    assert!(identities.insert(artifact.baseline_report_id().to_string()));

    let store = RetestEvidenceStore::open_in_memory().unwrap();
    store.persist_perturbation_study(&artifact, 4).unwrap();
    assert!(matches!(
        store.persist_perturbation_study(&artifact, 5),
        Err(RetestError::DuplicateLineage)
    ));
    let page = store
        .query_studies(&StudyArtifactQuery {
            source_dataset_id: manifest.dataset_id.clone(),
            kind: Some(StudyArtifactKind::Perturbation),
            after_sequence: None,
            limit: 4,
        })
        .unwrap();
    assert!(matches!(
        &page.records[0].artifact,
        StudyArtifact::Perturbation(value) if value == &artifact
    ));
}

#[test]
fn perturbation_study_declares_execution_cost_unsupported_for_a_zero_cost_config() {
    let (config, bars, manifest, space) = perturbation_fixture(103.0, false);
    let artifact = execute_perturbation_study(
        &config,
        &manifest,
        &bars,
        study_lease(&manifest, bars.len()),
        &space,
        perturbation_spec(),
    )
    .unwrap();
    artifact.verify().unwrap();

    for family in artifact.families() {
        if family.family() == PerturbationFamily::ExecutionCost {
            assert!(family.unsupported_reason().is_some());
            assert!(family.trials().is_empty());
            assert!(family.percentiles().is_none());
        } else {
            assert!(family.unsupported_reason().is_none());
            assert_eq!(family.trials().len(), 4);
            assert!(family.percentiles().is_some());
        }
    }

    // A recorded support decision is re-derived, never trusted: forging it fails closed even when
    // the artifact identity is recomputed over the forged content.
    let mut forged: serde_json::Value =
        serde_json::from_slice(&artifact.to_json_vec().unwrap()).unwrap();
    forged["families"][1]["unsupported_reason"] = serde_json::Value::Null;
    let forged = serde_json::to_vec(&forged).unwrap();
    assert!(PerturbationStudyArtifact::from_json_slice(&forged).is_err());
    assert!(PerturbationStudyArtifact::resealed_from_json(&forged).is_err());
}

#[test]
fn perturbation_trials_preserve_source_chronology_and_never_reach_future_bars() {
    let (config, bars, manifest, space) = perturbation_fixture(103.0, true);
    let artifact = execute_perturbation_study(
        &config,
        &manifest,
        &bars,
        study_lease(&manifest, bars.len()),
        &space,
        perturbation_spec(),
    )
    .unwrap();

    let source_first = bars.first().unwrap().timestamp.clone();
    let source_last = bars.last().unwrap().timestamp.clone();
    let mut offsets = std::collections::BTreeSet::new();
    let mut perturbed_datasets = std::collections::BTreeSet::new();
    for family in artifact.families() {
        for trial in family.trials() {
            assert_eq!(trial.last_timestamp, source_last);
            match &trial.detail {
                PerturbationDetail::StartOffset { offset } => {
                    assert!(*offset >= 1 && *offset <= 4);
                    assert!(offsets.insert(*offset));
                    assert_eq!(trial.range_start, *offset);
                    assert_eq!(trial.range_end, bars.len());
                    assert!(trial.first_timestamp > source_first);
                    assert_ne!(trial.dataset_id, manifest.dataset_id);
                    assert!(perturbed_datasets.insert(trial.dataset_id.clone()));
                }
                PerturbationDetail::DataNoise => {
                    assert_eq!(trial.range_start, 0);
                    assert_eq!(trial.range_end, bars.len());
                    assert_eq!(trial.first_timestamp, source_first);
                    assert_ne!(trial.dataset_id, manifest.dataset_id);
                    assert!(perturbed_datasets.insert(trial.dataset_id.clone()));
                }
                PerturbationDetail::ParameterJitter { ordinal, .. } => {
                    assert!(*ordinal < space.combinations());
                    assert_eq!(trial.dataset_id, manifest.dataset_id);
                    assert_eq!(trial.config_id, config.config_id());
                    assert_ne!(trial.strategy_id, artifact.baseline_candidate_id());
                }
                PerturbationDetail::ExecutionCost { scale_bps } => {
                    assert!(*scale_bps >= 1 && *scale_bps <= 20_000);
                    assert_eq!(trial.dataset_id, manifest.dataset_id);
                    assert_eq!(trial.strategy_id, artifact.baseline_candidate_id());
                    assert_ne!(trial.config_id, config.config_id());
                }
            }
        }
    }
    assert_eq!(offsets.len(), 4);
    assert_eq!(perturbed_datasets.len(), 8);

    // A trial that reached past the leased range, or moved its chronology, is refused on the
    // structural evidence alone — not merely because the recorded digest changed.
    for (field, value) in [
        ("range_end", serde_json::json!(bars.len() + 1)),
        ("range_start", serde_json::json!(0)),
        ("last_timestamp", serde_json::json!("2026-06-01T00:00:00Z")),
        ("first_timestamp", serde_json::json!("2026-04-01T00:00:00Z")),
    ] {
        let mut leaked: serde_json::Value =
            serde_json::from_slice(&artifact.to_json_vec().unwrap()).unwrap();
        leaked["families"][3]["trials"][0][field] = value;
        let leaked = serde_json::to_vec(&leaked).unwrap();
        assert!(PerturbationStudyArtifact::from_json_slice(&leaked).is_err());
        assert!(
            PerturbationStudyArtifact::resealed_from_json(&leaked).is_err(),
            "resealed leak through {field} verified"
        );
    }
}

#[test]
fn perturbation_study_fails_closed_on_bounds_leases_undefined_and_tampering() {
    let (config, bars, manifest, space) = perturbation_fixture(103.0, true);
    let execute = |spec| {
        execute_perturbation_study(
            &config,
            &manifest,
            &bars,
            study_lease(&manifest, bars.len()),
            &space,
            spec,
        )
    };
    for mutate in [
        (|spec: &mut PerturbationStudySpec| spec.trials_per_family = 0)
            as fn(&mut PerturbationStudySpec),
        |spec| spec.trials_per_family = MAX_PERTURBATION_TRIALS_PER_FAMILY + 1,
        |spec| spec.jitter_steps = 0,
        |spec| spec.jitter_steps = MAX_PERTURBATION_JITTER_STEPS + 1,
        |spec| spec.cost_scale_bps = 0,
        |spec| spec.cost_scale_bps = MAX_PERTURBATION_COST_SCALE_BPS + 1,
        |spec| spec.data_noise_bps = 0,
        |spec| spec.data_noise_bps = MAX_PERTURBATION_NOISE_BPS + 1,
        |spec| spec.maximum_start_offset = 0,
        |spec| spec.maximum_start_offset = MAX_PERTURBATION_START_OFFSET + 1,
        |spec| spec.evaluations_n = 0,
        |spec| spec.evaluations_n = MAX_TRIAL_BUDGET + 1,
        |spec| spec.metric_id = "  ".into(),
        // A metric the sealed report does not define.
        |spec| spec.metric_id = "unknown_metric".into(),
        // Neighbourhood smaller than the requested trial budget.
        |spec| spec.jitter_steps = 1,
        // Start offsets cannot be drawn without replacement.
        |spec| spec.maximum_start_offset = 3,
        // A start offset that would leave no leased bars behind.
        |spec| spec.maximum_start_offset = 13,
    ] {
        let mut spec = perturbation_spec();
        mutate(&mut spec);
        assert!(execute(spec).is_err());
    }

    // Foreign lease, wrong stage, and holdout access.
    let foreign = HoldoutQuarantine::new("a".repeat(64), "f".repeat(64), bars.len() + 4, 4)
        .unwrap()
        .lease(StageAccess::Robustness)
        .unwrap();
    assert!(
        execute_perturbation_study(
            &config,
            &manifest,
            &bars,
            foreign,
            &space,
            perturbation_spec()
        )
        .is_err()
    );
    let search = HoldoutQuarantine::new(&manifest.dataset_id, "f".repeat(64), bars.len() + 4, 4)
        .unwrap()
        .lease(StageAccess::Search)
        .unwrap();
    assert!(
        execute_perturbation_study(
            &config,
            &manifest,
            &bars,
            search,
            &space,
            perturbation_spec()
        )
        .is_err()
    );
    assert!(
        HoldoutQuarantine::new(&manifest.dataset_id, "f".repeat(64), bars.len() + 4, 4)
            .unwrap()
            .lease(StageAccess::FinalReview)
            .is_err()
    );

    // A baseline whose declared value has no place in its own domain has no neighbourhood.
    let (off_config, off_bars, off_manifest, off_space) = perturbation_fixture(101.0, true);
    assert!(
        execute_perturbation_study(
            &off_config,
            &off_manifest,
            &off_bars,
            study_lease(&off_manifest, off_bars.len()),
            &off_space,
            perturbation_spec(),
        )
        .is_err()
    );

    // Non-finite bars have no manifest at all, so they can only arrive as a payload that does not
    // belong to the sealed source evidence; non-positive prices have one and are refused directly.
    let mut non_finite = bars.clone();
    non_finite[3].close = f64::NAN;
    assert!(
        execute_perturbation_study(
            &config,
            &manifest,
            &non_finite,
            study_lease(&manifest, non_finite.len()),
            &space,
            perturbation_spec(),
        )
        .is_err()
    );
    let mut unpriced = bars.clone();
    unpriced[3].low = 0.0;
    let unpriced_manifest = self::manifest(&unpriced);
    assert!(
        execute_perturbation_study(
            &config,
            &unpriced_manifest,
            &unpriced,
            study_lease(&unpriced_manifest, unpriced.len()),
            &space,
            perturbation_spec(),
        )
        .is_err()
    );

    let artifact = execute(perturbation_spec()).unwrap();
    let tamper = |pointer: &str, value: serde_json::Value| {
        let mut tampered: serde_json::Value =
            serde_json::from_slice(&artifact.to_json_vec().unwrap()).unwrap();
        *tampered.pointer_mut(pointer).unwrap() = value;
        serde_json::to_vec(&tampered).unwrap()
    };
    // Evidence that is re-derived during verification: editing it fails closed even after the
    // artifact identity is recomputed over the edit.
    for (pointer, value) in [
        ("/baseline_value", serde_json::json!(1.0e9)),
        ("/baseline_run_id", serde_json::json!("0".repeat(64))),
        ("/families/0/trials/0/value", serde_json::json!(1.0e9)),
        ("/families/0/trials/0/component_seed", serde_json::json!(1)),
        (
            "/families/0/trials/1/strategy_id",
            serde_json::json!("0".repeat(64)),
        ),
        (
            "/families/1/trials/0/config_id",
            serde_json::json!("0".repeat(64)),
        ),
        (
            "/families/2/trials/0/dataset_id",
            serde_json::json!(&artifact.source_dataset_id()),
        ),
        ("/spec/root_seed", serde_json::json!(1)),
        ("/range_end", serde_json::json!(13)),
    ] {
        let bytes = tamper(pointer, value);
        assert!(
            PerturbationStudyArtifact::from_json_slice(&bytes).is_err(),
            "tampered {pointer} verified"
        );
        assert!(
            PerturbationStudyArtifact::resealed_from_json(&bytes).is_err(),
            "resealed {pointer} verified"
        );
    }
    // Evidence the artifact only seals: the recorded digest is what refuses the edit.
    assert!(
        PerturbationStudyArtifact::from_json_slice(&tamper(
            "/spec/evaluations_n",
            serde_json::json!(13)
        ))
        .is_err()
    );

    // Duplicate trial evidence inside one family.
    let mut duplicate: serde_json::Value =
        serde_json::from_slice(&artifact.to_json_vec().unwrap()).unwrap();
    duplicate["families"][0]["trials"][1] = duplicate["families"][0]["trials"][0].clone();
    let duplicate = serde_json::to_vec(&duplicate).unwrap();
    assert!(PerturbationStudyArtifact::from_json_slice(&duplicate).is_err());
    assert!(PerturbationStudyArtifact::resealed_from_json(&duplicate).is_err());

    // Replay under a different execution config is foreign evidence.
    let mut other = config.to_input();
    other.warmup_bars = 1;
    let other_config = StrategyExecutionConfig::build(&other).unwrap();
    assert!(
        replay_perturbation_study(
            &other_config,
            &manifest,
            &bars,
            study_lease(&manifest, bars.len()),
            &space,
            &artifact,
        )
        .is_err()
    );
}

/// A two-axis long-only fixture over the oscillating series: entry and exit are both gated by a
/// declared search axis, so the declared field is a genuine surface — moving either coordinate
/// changes the realized trade set, not only the fill prices.
fn parameter_field_fixture() -> (
    StrategyExecutionConfig,
    Vec<Bar>,
    DatasetManifest,
    SearchSpace,
) {
    let mut definition = GeneralStrategyBuilder::new("parameter-field", "test")
        .definition()
        .clone();
    definition.parameters = vec![
        StrategyParameter {
            id: "entry_level".into(),
            value: ParamValue::Float(103.0),
            range: Some(ParamRange::Float {
                min: 95.0,
                max: 115.0,
            }),
        },
        StrategyParameter {
            id: "exit_level".into(),
            value: ParamValue::Float(110.0),
            range: Some(ParamRange::Float {
                min: 100.0,
                max: 120.0,
            }),
        },
    ];
    definition.long.enabled = true;
    definition.long.entry = Condition::Compare {
        left: Operand::Price {
            field: crate::core::strategy_ir::PriceField::Close,
            bars_ago: 0,
        },
        op: CompareOp::Greater,
        right: Operand::Parameter("entry_level".into()),
    };
    definition.long.exit = Condition::Compare {
        left: Operand::Price {
            field: crate::core::strategy_ir::PriceField::Close,
            bars_ago: 0,
        },
        op: CompareOp::Greater,
        right: Operand::Parameter("exit_level".into()),
    };
    let strategy = crate::core::strategy_ir::StrategyIr::build(&definition).unwrap();
    let mut settings = ExecutionSettings::conservative_defaults();
    settings.slippage = SlippageModel::FixedPriceDistance { distance: 0.05 };
    settings.spread = SpreadModel::Constant { price_units: 0.02 };
    let config = StrategyExecutionConfig::build(&settings).unwrap();
    let bars = perturbation_bars();
    let manifest = manifest(&bars);
    let space = SearchSpace::new(
        strategy,
        vec![
            ParameterDomain::new(
                "entry_level",
                vec![99.0, 103.0, 107.0]
                    .into_iter()
                    .map(ParamValue::Float)
                    .collect(),
            )
            .unwrap(),
            ParameterDomain::new(
                "exit_level",
                vec![104.0, 110.0, 116.0]
                    .into_iter()
                    .map(ParamValue::Float)
                    .collect(),
            )
            .unwrap(),
        ],
    )
    .unwrap();
    (config, bars, manifest, space)
}

/// The whole declared 3x3 field, so the SPP sample is the field itself.
fn parameter_field_spec() -> ParameterFieldStudySpec {
    ParameterFieldStudySpec {
        field_sample_size: 9,
        neighbour_radius: 1,
        plateau_tolerance_bps: 1_500,
        minimum_plateau_neighbours: 3,
        metric_id: "net_profit".into(),
        direction: ObjectiveDirection::Maximize,
        root_seed: 0x5f1e,
    }
}

/// Every ordinal within one step of `centre` on each axis of the 3x3 fixture field, centre excluded.
fn expected_neighbourhood(centre: usize) -> std::collections::BTreeSet<usize> {
    let (row, column) = (centre / 3, centre % 3);
    let mut expected = std::collections::BTreeSet::new();
    for entry in row.saturating_sub(1)..=(row + 1).min(2) {
        for exit in column.saturating_sub(1)..=(column + 1).min(2) {
            if entry * 3 + exit != centre {
                expected.insert(entry * 3 + exit);
            }
        }
    }
    expected
}

#[test]
fn parameter_field_study_executes_the_declared_field_and_persists_immutably() {
    let (config, bars, manifest, space) = parameter_field_fixture();
    let source_manifest = manifest.clone();
    let execute = || {
        execute_parameter_field_study(
            &config,
            &manifest,
            &bars,
            study_lease(&manifest, bars.len()),
            &space,
            parameter_field_spec(),
        )
    };
    let artifact = execute().unwrap();
    artifact.verify().unwrap();
    assert_eq!(artifact, execute().unwrap());
    assert_eq!(
        artifact,
        replay_parameter_field_study(
            &config,
            &manifest,
            &bars,
            study_lease(&manifest, bars.len()),
            &space,
            &artifact,
        )
        .unwrap()
    );
    assert_eq!(
        artifact,
        ParameterFieldStudyArtifact::from_json_slice(&artifact.to_json_vec().unwrap()).unwrap()
    );
    // Positive control for the re-sealing negative tests below: untouched evidence survives a
    // recomputed identity, so those tests fail on the invariant and not on the decode.
    assert_eq!(
        artifact,
        ParameterFieldStudyArtifact::resealed_from_json(&artifact.to_json_vec().unwrap()).unwrap()
    );

    assert_eq!(artifact.source_dataset_id(), manifest.dataset_id);
    assert_eq!(artifact.source_manifest_id(), manifest.manifest_id);
    assert_eq!(artifact.config_id(), config.config_id());
    assert_eq!(artifact.range(), 0..bars.len());
    assert_eq!(artifact.metric_id(), "net_profit");
    assert_eq!(artifact.root_seed(), 0x5f1e);
    // The source evidence the field was measured on is still exactly what it was.
    assert_eq!(manifest, source_manifest);
    source_manifest.verify(&bars).unwrap();

    // Every coordinate is a distinct canonical execution with its own report identity.
    let mut identities = std::collections::BTreeSet::new();
    let mut ordinals = std::collections::BTreeSet::new();
    let mut ranks = std::collections::BTreeSet::new();
    for (index, point) in artifact.points().iter().enumerate() {
        assert_eq!(point.evaluation_n, index + 1);
        assert_eq!(point.request_id.len(), 64);
        assert_eq!(point.run_id.len(), 64);
        assert_eq!(point.report_id.len(), 64);
        assert!(point.value.is_finite());
        assert!(ordinals.insert(point.ordinal));
        assert!(ranks.insert(point.rank));
        assert!(identities.insert(point.request_id.clone()));
        assert!(identities.insert(point.run_id.clone()));
        assert!(identities.insert(point.report_id.clone()));
        // Bounded projection data: one axis index and one assignment per declared axis.
        assert_eq!(point.axis_indices.len(), artifact.axes().len());
        assert_eq!(point.assignments.len(), artifact.axes().len());
        for (axis, index) in artifact.axes().iter().zip(&point.axis_indices) {
            assert!(*index < axis.values().len());
        }
    }
    assert_eq!(ranks, (1..=artifact.points().len()).collect());
    assert_eq!(artifact.axes().len(), 2);
    assert_eq!(artifact.axes()[0].id(), "entry_level");
    assert_eq!(artifact.axes()[1].id(), "exit_level");

    // SPP: the whole declared field, its own distribution, and its evaluation N.
    let spp = artifact.spp();
    assert_eq!(spp.sample_size(), 9);
    assert_eq!(spp.field_combinations(), space.combinations());
    assert!(spp.exhaustive());
    assert_eq!(artifact.evaluations_n(), spp.sample_size());
    assert_eq!(spp.sorted_values().len(), 9);
    assert_eq!(spp.percentiles().confidence_level_bps(), 9_000);
    assert!(spp.percentiles().p05() <= spp.percentiles().median());
    assert!(spp.percentiles().median() <= spp.percentiles().p95());
    assert_eq!(spp.estimate(), spp.percentiles().median());
    // The declared field actually disperses, so the estimate is evidence and not a constant.
    assert!(spp.field_minimum() < spp.field_maximum());

    // Plateau: an explicit deterministic neighbourhood of the selected point, bound to reports.
    let plateau = artifact.plateau();
    assert_eq!(
        plateau.centre_ordinal(),
        artifact.profile().selected_ordinal()
    );
    assert_eq!(plateau.radius(), 1);
    let members = plateau
        .members()
        .iter()
        .map(|member| member.ordinal)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(members, expected_neighbourhood(plateau.centre_ordinal()));
    assert!(!members.contains(&plateau.centre_ordinal()));
    for member in plateau.members() {
        let point = artifact
            .points()
            .iter()
            .find(|point| point.ordinal == member.ordinal)
            .expect("plateau member is an executed point");
        assert_eq!(member.report_id, point.report_id);
        assert_eq!(member.candidate_id, point.candidate_id);
        assert_eq!(member.value, point.value);
    }
    assert_eq!(
        plateau.holding_members(),
        plateau.members().iter().filter(|m| m.holds).count()
    );

    // Optimization Profile: report-bound coordinates, ranks and the selection universe.
    let profile = artifact.profile();
    assert_eq!(profile.observations_n(), artifact.points().len());
    assert_eq!(profile.evaluations_n(), 9);
    assert!(profile.selected_rank() >= 1);
    assert_eq!(
        profile.selection_label(),
        format!("best of N=9: {}", profile.selected_candidate_id())
    );
    assert!(profile.stability_bps() <= 10_000);
    assert!(profile.within_tolerance() >= 1);

    let store = RetestEvidenceStore::open_in_memory().unwrap();
    store.persist_parameter_field_study(&artifact, 6).unwrap();
    assert!(matches!(
        store.persist_parameter_field_study(&artifact, 7),
        Err(RetestError::DuplicateLineage)
    ));
    let page = store
        .query_studies(&StudyArtifactQuery {
            source_dataset_id: manifest.dataset_id.clone(),
            kind: Some(StudyArtifactKind::ParameterField),
            after_sequence: None,
            limit: 4,
        })
        .unwrap();
    assert!(matches!(
        &page.records[0].artifact,
        StudyArtifact::ParameterField(value) if value == &artifact
    ));
    assert!(
        store
            .explain_study_query(&StudyArtifactQuery {
                source_dataset_id: manifest.dataset_id.clone(),
                kind: Some(StudyArtifactKind::ParameterField),
                after_sequence: None,
                limit: 4,
            })
            .unwrap()
            .iter()
            .any(|plan| plan.contains("idx_study_dataset_kind_sequence"))
    );
}

#[test]
fn parameter_field_plateau_separates_sharp_optima_from_broad_stable_regions() {
    let (config, bars, manifest, space) = parameter_field_fixture();
    let execute = |spec| {
        execute_parameter_field_study(
            &config,
            &manifest,
            &bars,
            study_lease(&manifest, bars.len()),
            &space,
            spec,
        )
        .unwrap()
    };

    // Zero tolerance with a full quorum: only a neighbourhood that is no worse than the optimum
    // anywhere can hold, so a unique optimum is reported as a sharp isolated spike.
    let mut sharp_spec = parameter_field_spec();
    sharp_spec.plateau_tolerance_bps = 0;
    sharp_spec.minimum_plateau_neighbours = 8;
    let sharp = execute(sharp_spec);
    assert_eq!(sharp.plateau().tolerance_bps(), 0);
    assert_eq!(sharp.plateau().threshold(), sharp.plateau().centre_value());
    assert!(sharp.plateau().holding_members() < sharp.plateau().members().len());
    assert_eq!(
        sharp.plateau().verdict(),
        PlateauVerdict::SharpIsolatedOptimum
    );

    // The whole field range as tolerance: the exhaustive sample bounds every neighbour from below,
    // so the same optimum is reported as a broad stable region.
    let mut broad_spec = parameter_field_spec();
    broad_spec.plateau_tolerance_bps = 10_000;
    broad_spec.minimum_plateau_neighbours = 1;
    let broad = execute(broad_spec);
    assert_eq!(
        broad.plateau().centre_ordinal(),
        sharp.plateau().centre_ordinal()
    );
    assert_eq!(
        broad.plateau().scale(),
        broad.spp().field_maximum() - broad.spp().field_minimum()
    );
    assert!(broad.plateau().scale() > 0.0);
    assert_eq!(
        broad.plateau().holding_members(),
        broad.plateau().members().len()
    );
    assert_eq!(broad.plateau().stability_bps(), 10_000);
    assert_eq!(broad.plateau().verdict(), PlateauVerdict::BroadStableRegion);
    assert_eq!(broad.profile().stability_bps(), 10_000);

    // A quorum the declared neighbourhood cannot supply is refused, not silently clamped.
    let mut impossible = parameter_field_spec();
    impossible.minimum_plateau_neighbours = MAX_PARAMETER_FIELD_NEIGHBOURHOOD;
    assert!(
        execute_parameter_field_study(
            &config,
            &manifest,
            &bars,
            study_lease(&manifest, bars.len()),
            &space,
            impossible,
        )
        .is_err()
    );
}

#[test]
fn parameter_field_spp_estimates_the_field_and_stays_distinct_from_the_optimized_point() {
    let (config, bars, manifest, space) = parameter_field_fixture();
    let execute = |spec| {
        execute_parameter_field_study(
            &config,
            &manifest,
            &bars,
            study_lease(&manifest, bars.len()),
            &space,
            spec,
        )
    };
    let artifact = execute(parameter_field_spec()).unwrap();

    // The field distribution is measured over the field sample alone — a neighbourhood evaluation
    // is local evidence about the optimum, never a member of the field's own distribution.
    let sampled = artifact
        .points()
        .iter()
        .filter(|point| point.phase == ParameterFieldPhase::FieldSample)
        .map(|point| point.value)
        .collect::<Vec<_>>();
    assert_eq!(sampled.len(), artifact.spp().sample_size());
    let mut sorted = sampled.clone();
    sorted.sort_by(f64::total_cmp);
    assert_eq!(artifact.spp().sorted_values(), sorted.as_slice());
    assert_eq!(artifact.spp().field_minimum(), sorted[0]);
    assert_eq!(artifact.spp().field_maximum(), sorted[sorted.len() - 1]);

    // The headline estimate is the field's median, not the optimized point, and the gap between
    // them is the recorded optimization bias.
    let selected = artifact.spp().selected_value();
    assert_eq!(selected, artifact.spp().field_maximum());
    assert!(artifact.spp().estimate() < selected);
    assert_eq!(
        artifact.spp().optimization_bias(),
        selected - artifact.spp().estimate()
    );
    assert!(artifact.spp().optimization_bias_bps() > 0);
    assert!(artifact.spp().optimization_bias_bps() <= 10_000);

    // A bounded sub-sample is a deterministic permutation prefix of the declared field: distinct
    // coordinates, seeded, and a different root seed selects a different sample.
    let mut partial_spec = parameter_field_spec();
    partial_spec.field_sample_size = 5;
    let partial = execute(partial_spec.clone()).unwrap();
    assert!(!partial.spp().exhaustive());
    assert_eq!(partial.spp().sample_size(), 5);
    assert_eq!(partial.profile().evaluations_n(), 5);
    let sample_ordinals = |artifact: &ParameterFieldStudyArtifact| {
        artifact
            .points()
            .iter()
            .filter(|point| point.phase == ParameterFieldPhase::FieldSample)
            .map(|point| point.ordinal)
            .collect::<Vec<_>>()
    };
    let drawn = sample_ordinals(&partial);
    assert_eq!(drawn.len(), 5);
    assert_eq!(
        drawn
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        5
    );
    assert!(drawn.iter().all(|ordinal| *ordinal < space.combinations()));
    let mut reseeded_spec = partial_spec.clone();
    reseeded_spec.root_seed = 0x991d;
    assert_ne!(sample_ordinals(&execute(reseeded_spec).unwrap()), drawn);

    // The neighbourhood of the selected point is executed even where it fell outside the sample,
    // and the selection universe stays exactly the field sample.
    let neighbourhood = expected_neighbourhood(partial.plateau().centre_ordinal());
    let executed = partial
        .points()
        .iter()
        .map(|point| point.ordinal)
        .collect::<std::collections::BTreeSet<_>>();
    assert!(neighbourhood.is_subset(&executed));
    assert_eq!(
        partial.profile().selection_label(),
        format!("best of N=5: {}", partial.profile().selected_candidate_id())
    );
    assert!(partial.points().len() > partial.spp().sample_size());
}

#[test]
fn parameter_field_study_fails_closed_on_bounds_leases_undefined_and_tampering() {
    let (config, bars, manifest, space) = parameter_field_fixture();
    let execute = |spec| {
        execute_parameter_field_study(
            &config,
            &manifest,
            &bars,
            study_lease(&manifest, bars.len()),
            &space,
            spec,
        )
    };
    for mutate in [
        (|spec: &mut ParameterFieldStudySpec| spec.field_sample_size = 0)
            as fn(&mut ParameterFieldStudySpec),
        |spec| spec.field_sample_size = 1,
        |spec| spec.field_sample_size = MAX_PARAMETER_FIELD_SAMPLE + 1,
        // Exhausted space: the declared field cannot supply that many distinct coordinates.
        |spec| spec.field_sample_size = 10,
        |spec| spec.neighbour_radius = 0,
        |spec| spec.neighbour_radius = MAX_PARAMETER_FIELD_RADIUS + 1,
        |spec| spec.plateau_tolerance_bps = 10_001,
        |spec| spec.minimum_plateau_neighbours = 0,
        |spec| spec.minimum_plateau_neighbours = MAX_PARAMETER_FIELD_NEIGHBOURHOOD + 1,
        |spec| spec.metric_id = "  ".into(),
        // A metric the sealed report does not define.
        |spec| spec.metric_id = "unknown_metric".into(),
    ] {
        let mut spec = parameter_field_spec();
        mutate(&mut spec);
        assert!(execute(spec).is_err());
    }

    // Foreign lease, and a lease that does not admit the supplied payload.
    let foreign = HoldoutQuarantine::new("a".repeat(64), "f".repeat(64), bars.len() + 4, 4)
        .unwrap()
        .lease(StageAccess::Robustness)
        .unwrap();
    assert!(
        execute_parameter_field_study(
            &config,
            &manifest,
            &bars,
            foreign,
            &space,
            parameter_field_spec()
        )
        .is_err()
    );
    let short = HoldoutQuarantine::new(&manifest.dataset_id, "f".repeat(64), bars.len() + 4, 6)
        .unwrap()
        .lease(StageAccess::Robustness)
        .unwrap();
    assert!(
        execute_parameter_field_study(
            &config,
            &manifest,
            &bars,
            short,
            &space,
            parameter_field_spec()
        )
        .is_err()
    );
    // The final holdout is not leasable to a field study at all.
    assert!(
        HoldoutQuarantine::new(&manifest.dataset_id, "f".repeat(64), bars.len() + 4, 4)
            .unwrap()
            .lease(StageAccess::FinalReview)
            .is_err()
    );

    let artifact = execute(parameter_field_spec()).unwrap();
    let tamper = |pointer: &str, value: serde_json::Value| {
        let mut tampered: serde_json::Value =
            serde_json::from_slice(&artifact.to_json_vec().unwrap()).unwrap();
        *tampered.pointer_mut(pointer).unwrap() = value;
        serde_json::to_vec(&tampered).unwrap()
    };
    // Evidence that is re-derived during verification: editing it fails closed even after the
    // artifact identity is recomputed over the edit.
    for (pointer, value) in [
        ("/points/0/value", serde_json::json!(1.0e9)),
        ("/points/0/ordinal", serde_json::json!(8)),
        ("/points/0/component_seed", serde_json::json!(1)),
        ("/points/0/rank", serde_json::json!(9)),
        ("/points/0/candidate_id", serde_json::json!("0".repeat(64))),
        ("/points/0/request_id", serde_json::json!("0".repeat(64))),
        ("/points/1/run_id", serde_json::json!("0".repeat(64))),
        ("/points/1/report_id", serde_json::json!("0".repeat(64))),
        ("/points/1/axis_indices/0", serde_json::json!(2)),
        ("/spp/estimate", serde_json::json!(1.0e9)),
        ("/spp/optimization_bias_bps", serde_json::json!(1)),
        ("/spp/sorted_values/0", serde_json::json!(-1.0e9)),
        ("/plateau/verdict", serde_json::json!("broad_stable_region")),
        ("/plateau/threshold", serde_json::json!(-1.0e9)),
        ("/plateau/members/0/holds", serde_json::json!(true)),
        ("/plateau/holding_members", serde_json::json!(0)),
        ("/profile/stability_bps", serde_json::json!(1)),
        ("/profile/selected_ordinal", serde_json::json!(0)),
        ("/profile/observations_n", serde_json::json!(3)),
        ("/spec/root_seed", serde_json::json!(1)),
        ("/spec/direction", serde_json::json!("minimize")),
        ("/range_end", serde_json::json!(13)),
    ] {
        let bytes = tamper(pointer, value);
        assert!(
            ParameterFieldStudyArtifact::from_json_slice(&bytes).is_err(),
            "tampered {pointer} verified"
        );
        assert!(
            ParameterFieldStudyArtifact::resealed_from_json(&bytes).is_err(),
            "resealed {pointer} verified"
        );
    }

    // A duplicated coordinate is duplicate evidence, not a second observation.
    let mut duplicate: serde_json::Value =
        serde_json::from_slice(&artifact.to_json_vec().unwrap()).unwrap();
    duplicate["points"][1] = duplicate["points"][0].clone();
    let duplicate = serde_json::to_vec(&duplicate).unwrap();
    assert!(ParameterFieldStudyArtifact::from_json_slice(&duplicate).is_err());
    assert!(ParameterFieldStudyArtifact::resealed_from_json(&duplicate).is_err());

    // A plateau member that names a coordinate the study never executed.
    let mut foreign_member: serde_json::Value =
        serde_json::from_slice(&artifact.to_json_vec().unwrap()).unwrap();
    foreign_member["plateau"]["members"][0]["report_id"] = serde_json::json!("0".repeat(64));
    let foreign_member = serde_json::to_vec(&foreign_member).unwrap();
    assert!(ParameterFieldStudyArtifact::from_json_slice(&foreign_member).is_err());
    assert!(ParameterFieldStudyArtifact::resealed_from_json(&foreign_member).is_err());

    // Replay under a different execution config is foreign evidence.
    let mut other = config.to_input();
    other.warmup_bars = 1;
    let other_config = StrategyExecutionConfig::build(&other).unwrap();
    assert!(
        replay_parameter_field_study(
            &other_config,
            &manifest,
            &bars,
            study_lease(&manifest, bars.len()),
            &space,
            &artifact,
        )
        .is_err()
    );
}

#[test]
fn adjusted_significance_is_derived_from_the_sealed_field_and_persists_immutably() {
    let (config, bars, manifest, space) = parameter_field_fixture();
    let field = execute_parameter_field_study(
        &config,
        &manifest,
        &bars,
        study_lease(&manifest, bars.len()),
        &space,
        parameter_field_spec(),
    )
    .unwrap();
    let policy = SignificancePolicy {
        null_value: -1_000_000.0,
        false_discovery_rate_bps: 500,
        minimum_observations: 9,
    };
    let significance = execute_significance_study(std::slice::from_ref(&field), policy).unwrap();
    significance.verify().unwrap();
    assert_eq!(significance.source_dataset_id(), manifest.dataset_id);
    assert_eq!(significance.metric_id(), "net_profit");
    assert_eq!(significance.direction(), ObjectiveDirection::Maximize);
    assert_eq!(significance.evaluations_n(), 9);
    assert_eq!(significance.candidates().len(), 1);
    let candidate = &significance.candidates()[0];
    assert_eq!(
        candidate.candidate_id(),
        field.profile().selected_candidate_id()
    );
    assert_eq!(candidate.field_artifact_id(), field.artifact_id());
    assert_eq!(candidate.observations_n(), 9);
    assert_eq!(candidate.favourable_observations(), 9);
    assert_eq!(candidate.headline_field_estimate(), field.spp().estimate());
    assert!((candidate.raw_p() - 1.0 / 512.0).abs() < 1e-12);
    assert!((candidate.bonferroni_p() - 9.0 / 512.0).abs() < 1e-12);
    assert_eq!(candidate.false_discovery_rate_q(), candidate.raw_p());
    assert!(candidate.significant());
    let bytes = significance.to_json_vec().unwrap();
    assert_eq!(
        significance,
        SignificanceStudyArtifact::from_json_slice(&bytes).unwrap()
    );
    assert_eq!(
        significance,
        SignificanceStudyArtifact::resealed_from_json(&bytes).unwrap()
    );

    let store = RetestEvidenceStore::open_in_memory().unwrap();
    store.persist_significance_study(&significance, 8).unwrap();
    assert!(matches!(
        store.persist_significance_study(&significance, 9),
        Err(RetestError::DuplicateLineage)
    ));
    let page = store
        .query_studies(&StudyArtifactQuery {
            source_dataset_id: manifest.dataset_id.clone(),
            kind: Some(StudyArtifactKind::Significance),
            after_sequence: None,
            limit: 1,
        })
        .unwrap();
    assert!(matches!(
        &page.records[0].artifact,
        StudyArtifact::Significance(value) if value == &significance
    ));

    let mut derived_tamper: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    derived_tamper["candidates"][0]["false_discovery_rate_q"] = serde_json::json!(0.9);
    let derived_tamper = serde_json::to_vec(&derived_tamper).unwrap();
    assert!(SignificanceStudyArtifact::from_json_slice(&derived_tamper).is_err());
    assert!(SignificanceStudyArtifact::resealed_from_json(&derived_tamper).is_err());

    let mut source_tamper: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    source_tamper["source_field_zstd"][0][0] = serde_json::json!(b'!');
    let source_tamper = serde_json::to_vec(&source_tamper).unwrap();
    assert!(SignificanceStudyArtifact::from_json_slice(&source_tamper).is_err());
    assert!(SignificanceStudyArtifact::resealed_from_json(&source_tamper).is_err());

    assert!(execute_significance_study(&[], policy).is_err());
    assert!(
        execute_significance_study(
            std::slice::from_ref(&field),
            SignificancePolicy {
                minimum_observations: 10,
                ..policy
            }
        )
        .is_err()
    );
}

/// An oscillating series with both winning and losing closed trades. The §7.6 concentration gate
/// needs real gross profit, while the permissive positive control must not trip the no-losing-trades
/// profit-factor sentinel.
fn problem_recognition_bars() -> Vec<Bar> {
    [
        96.0, 104.0, 112.0, 108.0, 100.0, 116.0, 106.0, 98.0, 114.0, 102.0, 110.0, 118.0, 119.0,
        105.0,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, close)| Bar {
        timestamp: format!("2026-06-{:02}T00:00:00Z", index + 1),
        open: close - 1.0,
        high: close + 1.0,
        low: close - 2.0,
        close,
        volume: 100.0,
    })
    .collect()
}

/// A priced-cost long-only field: entry and exit are both parameterised thresholds, so every
/// coordinate of the declared field is a distinct trade sequence over the same bars.
fn problem_recognition_field_fixture(
    bars: Vec<Bar>,
) -> (
    StrategyExecutionConfig,
    Vec<Bar>,
    DatasetManifest,
    SearchSpace,
) {
    let mut definition = GeneralStrategyBuilder::new("problem-recognition", "test")
        .definition()
        .clone();
    definition.parameters = vec![
        StrategyParameter {
            id: "entry_level".into(),
            value: ParamValue::Float(99.0),
            range: Some(ParamRange::Float {
                min: 95.0,
                max: 125.0,
            }),
        },
        StrategyParameter {
            id: "exit_level".into(),
            value: ParamValue::Float(104.0),
            range: Some(ParamRange::Float {
                min: 95.0,
                max: 125.0,
            }),
        },
    ];
    definition.long.enabled = true;
    definition.long.entry = Condition::Compare {
        left: Operand::Price {
            field: crate::core::strategy_ir::PriceField::Close,
            bars_ago: 0,
        },
        op: CompareOp::Greater,
        right: Operand::Parameter("entry_level".into()),
    };
    definition.long.exit = Condition::Compare {
        left: Operand::Price {
            field: crate::core::strategy_ir::PriceField::Close,
            bars_ago: 0,
        },
        op: CompareOp::Greater,
        right: Operand::Parameter("exit_level".into()),
    };
    let strategy = crate::core::strategy_ir::StrategyIr::build(&definition).unwrap();
    let mut settings = ExecutionSettings::conservative_defaults();
    settings.slippage = SlippageModel::FixedPriceDistance { distance: 0.05 };
    settings.spread = SpreadModel::Constant { price_units: 0.02 };
    let config = StrategyExecutionConfig::build(&settings).unwrap();
    let manifest = manifest(&bars);
    let space = SearchSpace::new(
        strategy,
        vec![
            ParameterDomain::new(
                "entry_level",
                vec![99.0, 103.0, 107.0]
                    .into_iter()
                    .map(ParamValue::Float)
                    .collect(),
            )
            .unwrap(),
            ParameterDomain::new(
                "exit_level",
                vec![104.0, 110.0, 116.0]
                    .into_iter()
                    .map(ParamValue::Float)
                    .collect(),
            )
            .unwrap(),
        ],
    )
    .unwrap();
    (config, bars, manifest, space)
}

/// A cross-check dataset variant of the shared fixture bars: same content, distinct identity.
fn cross_check_manifest(
    bars: &[Bar],
    symbol: &str,
    timeframe: &str,
    source: &str,
) -> DatasetManifest {
    DatasetManifest::build(
        &DatasetManifestInput {
            symbol: symbol.into(),
            timeframe: timeframe.into(),
            provenance: DatasetProvenance {
                source: source.into(),
                venue: "test".into(),
                pipeline: format!("strategy-retest-test-{source}/v1"),
            },
            adjustment: AdjustmentPolicy::Raw,
            calendar: CalendarPolicy::Continuous24x7,
            qa_policy: DatasetQaPolicy::default(),
        },
        bars,
    )
    .unwrap()
}

/// The exact immutable evidence a problem-recognition verdict may read: one sealed cross-check
/// study, the executed OOS scheme for the same candidate, and the adjusted-significance study of
/// the parameter field the candidate was selected from.
struct ProblemRecognitionFixture {
    config: StrategyExecutionConfig,
    bars: Vec<Bar>,
    manifest: DatasetManifest,
    space: SearchSpace,
    strategy: crate::core::strategy_ir::StrategyIr,
    field: ParameterFieldStudyArtifact,
    cross: CrossCheckStudyArtifact,
    oos: ExecutedOosScheme,
    significance: SignificanceStudyArtifact,
}

fn significance_policy(false_discovery_rate_bps: u32) -> SignificancePolicy {
    SignificancePolicy {
        null_value: -1_000_000.0,
        false_discovery_rate_bps,
        minimum_observations: 9,
    }
}

impl ProblemRecognitionFixture {
    fn new() -> Self {
        Self::from_bars(problem_recognition_bars())
    }

    fn from_bars(bars: Vec<Bar>) -> Self {
        let (config, bars, manifest, space) = problem_recognition_field_fixture(bars);
        let field = execute_parameter_field_study(
            &config,
            &manifest,
            &bars,
            study_lease(&manifest, bars.len()),
            &space,
            parameter_field_spec(),
        )
        .unwrap();
        let significance =
            execute_significance_study(std::slice::from_ref(&field), significance_policy(500))
                .unwrap();
        // The candidate the field actually selected — its strategy, not a caller's label.
        let strategy = generate_candidates(&space, SearchMethod::Grid, space.combinations())
            .unwrap()
            .candidates
            .into_iter()
            .find(|candidate| candidate.candidate_id == field.profile().selected_candidate_id())
            .expect("the selected candidate is a coordinate of the declared field")
            .strategy;
        let cross = Self::cross_check(&strategy, &config, &manifest, &bars);
        let oos = Self::oos(&strategy, &config, &manifest, &bars, "net_profit");
        Self {
            config,
            bars,
            manifest,
            space,
            strategy,
            field,
            cross,
            oos,
            significance,
        }
    }

    fn cross_check(
        strategy: &crate::core::strategy_ir::StrategyIr,
        config: &StrategyExecutionConfig,
        manifest: &DatasetManifest,
        bars: &[Bar],
    ) -> CrossCheckStudyArtifact {
        let other_symbol = cross_check_manifest(bars, "ETH/USD", "1Day", "fixture");
        let adjacent_timeframe = cross_check_manifest(bars, "BTC/USD", "4Hour", "fixture");
        let alternative_source = cross_check_manifest(bars, "BTC/USD", "1Day", "alternate");
        execute_cross_check_study(
            strategy,
            config,
            manifest,
            bars,
            study_lease(manifest, bars.len()),
            vec![
                CrossCheckDatasetCase {
                    kind: CrossCheckKind::OtherSymbol,
                    label: "eth-usd".into(),
                    config,
                    dataset: &other_symbol,
                    bars,
                    lease: study_lease(&other_symbol, bars.len()),
                },
                CrossCheckDatasetCase {
                    kind: CrossCheckKind::AdjacentTimeframe,
                    label: "btc-4h".into(),
                    config,
                    dataset: &adjacent_timeframe,
                    bars,
                    lease: study_lease(&adjacent_timeframe, bars.len()),
                },
                CrossCheckDatasetCase {
                    kind: CrossCheckKind::AlternativeSource,
                    label: "btc-alternate".into(),
                    config,
                    dataset: &alternative_source,
                    bars,
                    lease: study_lease(&alternative_source, bars.len()),
                },
            ],
            CrossCheckStudySpec {
                metric_id: "net_profit".into(),
                direction: ObjectiveDirection::Maximize,
                minimum_retention_bps: 5_000,
                evaluations_n: 9,
                root_seed: 0x9c05,
            },
        )
        .unwrap()
    }

    fn oos(
        strategy: &crate::core::strategy_ir::StrategyIr,
        config: &StrategyExecutionConfig,
        manifest: &DatasetManifest,
        bars: &[Bar],
        metric_id: &str,
    ) -> ExecutedOosScheme {
        execute_oos_scheme(
            strategy,
            config,
            manifest,
            bars,
            study_lease(manifest, bars.len()),
            OosExecutionSpec {
                scheme: OosScheme::Trailing { oos_bars: 4 },
                purge_bars: 1,
                embargo_bars: 0,
                metric_id: metric_id.into(),
                root_seed: 0x9c05,
            },
        )
        .unwrap()
    }

    fn execute(
        &self,
        policy: ProblemRecognitionPolicy,
    ) -> Result<ProblemRecognitionArtifact, RetestError> {
        execute_problem_recognition(&self.cross, &self.oos, &self.significance, policy)
    }
}

/// Permissive thresholds: every gate is satisfied, so the verdict tests the derivation and not a
/// threshold. The failing-gate tests below tighten exactly one bound at a time.
fn problem_recognition_policy() -> ProblemRecognitionPolicy {
    ProblemRecognitionPolicy {
        minimum_trades: 1,
        maximum_top_trade_share_bps: 10_000,
        maximum_time_in_market_bps: 10_000,
        boundary_width_bps: 1_000,
        maximum_boundary_trade_share_bps: 10_000,
        minimum_cost_2x_ratio_bps: 0,
        minimum_cost_3x_ratio_bps: 0,
        minimum_oos_is_ratio_bps: 0,
        maximum_edge_concentration_bps: 10_000,
        maximum_absolute_sharpe_bps: 1_000_000,
        minimum_max_drawdown_bps: 0,
        minimum_parameter_step_ratio_bps: 0,
    }
}

fn defined_metric(report: &StrategyReportArtifact, id: &str) -> f64 {
    match report.analysis().metric(id) {
        Some(MetricValue::Defined { value }) => *value,
        other => panic!("problem-recognition fixture needs a defined {id}: {other:?}"),
    }
}

fn failing_stage(artifact: &ProblemRecognitionArtifact) -> Option<&str> {
    artifact
        .stages()
        .iter()
        .find(|stage| stage.verdict == StageVerdict::Fail)
        .map(|stage| stage.stage.as_str())
}

#[test]
fn problem_recognition_derives_every_gate_from_canonical_report_evidence() {
    let fixture = ProblemRecognitionFixture::new();
    let artifact = fixture.execute(problem_recognition_policy()).unwrap();
    artifact.verify().unwrap();

    // Identity is bound to the evidence, not to the caller's request.
    assert_eq!(artifact.artifact_id().len(), 64);
    assert_eq!(artifact.source_dataset_id(), fixture.manifest.dataset_id);
    assert_eq!(artifact.strategy_id(), fixture.strategy.strategy_id());
    assert_eq!(artifact.metric_id(), "net_profit");
    assert_eq!(artifact.policy(), problem_recognition_policy());

    // Every observation is the canonical report/study value, never a caller's number.
    let report = fixture.cross.baseline_report().unwrap();
    let observations = artifact.observations();
    assert_eq!(observations.trade_count, report.analysis().trades.len());
    assert_eq!(
        observations.top_trade_share_bps,
        (defined_metric(&report, "top_decile_pnl_share") * 10_000.0).round() as u32
    );
    assert_eq!(
        observations.time_in_market_bps,
        (defined_metric(&report, "time_in_market") * 10_000.0).round() as u32
    );
    assert_eq!(
        observations.cost_2x_ratio_bps,
        fixture
            .cross
            .checks()
            .iter()
            .find(|check| check.kind
                == CrossCheckKind::CostSensitivity {
                    multiplier_bps: COST_MULTIPLIER_2X_BPS
                })
            .map(|check| check.retention_bps.min(10_000))
            .unwrap()
    );
    // Pin the original primitive observations and require the added report-derived families to
    // carry bounded, non-empty evidence rather than caller labels.
    assert_eq!(observations.trade_count, 2);
    assert_eq!(observations.top_trade_share_bps, 10_000);
    assert_eq!(observations.time_in_market_bps, 8_462);
    assert_eq!(observations.boundary_trade_share_bps, 5_000);
    assert_eq!(observations.cost_2x_ratio_bps, 9_259);
    assert_eq!(observations.cost_3x_ratio_bps, 8_519);
    assert_eq!(observations.oos_is_ratio_bps, 0);
    assert_eq!(observations.edge_concentration.calendar_periods, 2);
    assert_eq!(
        observations.edge_concentration.symbol_share_bps,
        Some(10_000)
    );
    assert_eq!(observations.edge_concentration.side_share_bps, Some(10_000));
    assert_eq!(observations.absurd_metrics.absolute_sharpe_bps, Some(2_693));
    assert_eq!(observations.absurd_metrics.max_drawdown_bps, Some(2));
    assert!(!observations.absurd_metrics.profit_factor_at_sentinel);
    assert_eq!(observations.parameter_step.steps_n, 2);
    assert_eq!(observations.parameter_step.worst_step_ratio_bps, 0);

    // The declared §7.6 gate set, in order, each carrying its own observation count.
    assert_eq!(
        artifact
            .stages()
            .iter()
            .map(|stage| stage.stage.as_str())
            .collect::<Vec<_>>(),
        vec![
            "minimum-trades",
            "trade-concentration",
            "time-in-market",
            "boundary-reliance",
            "cost-degradation",
            "oos-degradation",
            "cost-degradation-3x",
            "edge-concentration",
            "absurd-metrics",
            "parameter-step-cliff",
            "adjusted-significance",
        ]
    );
    assert!(
        artifact.stages()[..6]
            .iter()
            .all(|stage| stage.observations_n == observations.trade_count)
    );
    // §7.7: the significance gate displays the selection universe it was judged against.
    assert_eq!(
        artifact.stages()[10].observations_n,
        fixture.significance.evaluations_n()
    );
    assert!(artifact.stages()[10].reason.contains("bonferroni p="));
    assert_eq!(failing_stage(&artifact), None);
    assert!(artifact.passed());

    // Deterministic: the same sealed evidence always yields the same verdict artifact.
    assert_eq!(
        artifact,
        fixture.execute(problem_recognition_policy()).unwrap()
    );

    // Round trip, plus the positive control the tamper tests below need: untouched evidence
    // survives a recomputed identity, so those failures are the invariant and not the decode.
    let bytes = artifact.to_json_vec().unwrap();
    assert!(bytes.len() <= MAX_ARTIFACT_BYTES);
    assert_eq!(
        artifact,
        ProblemRecognitionArtifact::from_json_slice(&bytes).unwrap()
    );
    assert_eq!(
        artifact,
        ProblemRecognitionArtifact::resealed_from_json(&bytes).unwrap()
    );

    // The embedded evidence is exactly the sealed studies that were judged, and replaying from
    // those embedded bytes alone reproduces the verdict.
    assert_eq!(artifact.source_cross_check().unwrap(), fixture.cross);
    assert_eq!(artifact.source_oos().unwrap(), fixture.oos);
    assert_eq!(
        artifact.source_significance().unwrap(),
        fixture.significance
    );
    assert_eq!(replay_problem_recognition(&artifact).unwrap(), artifact);

    // The sources themselves are untouched by having been judged.
    fixture.cross.verify().unwrap();
    fixture.oos.verify().unwrap();
    fixture.significance.verify().unwrap();
}

#[test]
fn problem_recognition_persists_immutably_with_indexed_lookup() {
    let fixture = ProblemRecognitionFixture::new();
    let artifact = fixture.execute(problem_recognition_policy()).unwrap();
    let store = RetestEvidenceStore::open_in_memory().unwrap();
    store.persist_problem_recognition(&artifact, 12).unwrap();
    assert!(matches!(
        store.persist_problem_recognition(&artifact, 13),
        Err(RetestError::DuplicateLineage)
    ));
    let query = StudyArtifactQuery {
        source_dataset_id: fixture.manifest.dataset_id.clone(),
        kind: Some(StudyArtifactKind::ProblemRecognition),
        after_sequence: None,
        limit: 4,
    };
    let page = store.query_studies(&query).unwrap();
    assert_eq!(page.records.len(), 1);
    assert!(!page.has_more);
    assert_eq!(page.records[0].created_sequence, 12);
    let StudyArtifact::ProblemRecognition(stored) = &page.records[0].artifact else {
        panic!("stored kind is not problem recognition");
    };
    assert_eq!(stored, &artifact);
    // A reloaded verdict still replays from its own embedded evidence.
    assert_eq!(replay_problem_recognition(stored).unwrap(), artifact);
    assert!(
        store
            .explain_study_query(&query)
            .unwrap()
            .iter()
            .any(|plan| plan.contains("idx_study_dataset_kind_sequence"))
    );
    // The kind is its own indexed lane: a sibling kind does not answer this query.
    assert!(
        store
            .query_studies(&StudyArtifactQuery {
                kind: Some(StudyArtifactKind::CrossCheck),
                ..query
            })
            .unwrap()
            .records
            .is_empty()
    );
}

#[test]
fn problem_recognition_fails_the_exact_gate_its_evidence_misses() {
    let fixture = ProblemRecognitionFixture::new();
    let observations = fixture
        .execute(problem_recognition_policy())
        .unwrap()
        .observations();
    // Each policy tightens exactly one bound past what the sealed evidence actually shows.
    for (policy, expected) in [
        (
            ProblemRecognitionPolicy {
                minimum_trades: observations.trade_count + 1,
                ..problem_recognition_policy()
            },
            "minimum-trades",
        ),
        (
            ProblemRecognitionPolicy {
                maximum_top_trade_share_bps: observations.top_trade_share_bps - 1,
                ..problem_recognition_policy()
            },
            "trade-concentration",
        ),
        (
            ProblemRecognitionPolicy {
                maximum_time_in_market_bps: observations.time_in_market_bps - 1,
                ..problem_recognition_policy()
            },
            "time-in-market",
        ),
        (
            ProblemRecognitionPolicy {
                maximum_boundary_trade_share_bps: observations.boundary_trade_share_bps - 1,
                ..problem_recognition_policy()
            },
            "boundary-reliance",
        ),
        (
            ProblemRecognitionPolicy {
                minimum_cost_2x_ratio_bps: observations.cost_2x_ratio_bps + 1,
                ..problem_recognition_policy()
            },
            "cost-degradation",
        ),
        (
            ProblemRecognitionPolicy {
                minimum_oos_is_ratio_bps: observations.oos_is_ratio_bps + 1,
                ..problem_recognition_policy()
            },
            "oos-degradation",
        ),
        (
            ProblemRecognitionPolicy {
                minimum_cost_3x_ratio_bps: observations.cost_3x_ratio_bps + 1,
                ..problem_recognition_policy()
            },
            "cost-degradation-3x",
        ),
        (
            ProblemRecognitionPolicy {
                maximum_edge_concentration_bps: observations
                    .edge_concentration
                    .worst
                    .expect("fixture has attributable edge")
                    .1
                    - 1,
                ..problem_recognition_policy()
            },
            "edge-concentration",
        ),
        (
            ProblemRecognitionPolicy {
                maximum_absolute_sharpe_bps: observations
                    .absurd_metrics
                    .absolute_sharpe_bps
                    .expect("fixture has defined Sharpe")
                    - 1,
                ..problem_recognition_policy()
            },
            "absurd-metrics",
        ),
        (
            ProblemRecognitionPolicy {
                minimum_max_drawdown_bps: observations
                    .absurd_metrics
                    .max_drawdown_bps
                    .expect("fixture has defined drawdown")
                    + 1,
                ..problem_recognition_policy()
            },
            "absurd-metrics",
        ),
        (
            ProblemRecognitionPolicy {
                minimum_parameter_step_ratio_bps: observations.parameter_step.worst_step_ratio_bps
                    + 1,
                ..problem_recognition_policy()
            },
            "parameter-step-cliff",
        ),
    ] {
        let artifact = fixture.execute(policy).unwrap();
        assert_eq!(failing_stage(&artifact), Some(expected));
        assert!(!artifact.passed());
        // A failing verdict is evidence too: it seals, reloads and replays like any other.
        assert_eq!(
            artifact,
            ProblemRecognitionArtifact::from_json_slice(&artifact.to_json_vec().unwrap()).unwrap()
        );
        assert_eq!(replay_problem_recognition(&artifact).unwrap(), artifact);
    }

    // §7.7: the significance verdict is read from the sealed study, so a family that survives a
    // 5% discovery rate and fails a 1% one flips this gate without touching any threshold here.
    let strict = execute_significance_study(
        std::slice::from_ref(&fixture.field),
        significance_policy(100),
    )
    .unwrap();
    assert!(!strict.candidates()[0].significant());
    let artifact = execute_problem_recognition(
        &fixture.cross,
        &fixture.oos,
        &strict,
        problem_recognition_policy(),
    )
    .unwrap();
    assert_eq!(failing_stage(&artifact), Some("adjusted-significance"));
    assert!(!artifact.passed());

    // The edge-band width is a derivation input, not a verdict: widening it can only ever find
    // more boundary trades in the same sealed trade list.
    let wide = fixture
        .execute(ProblemRecognitionPolicy {
            boundary_width_bps: 5_000,
            ..problem_recognition_policy()
        })
        .unwrap();
    assert!(wide.observations().boundary_trade_share_bps >= observations.boundary_trade_share_bps);
}

#[test]
fn problem_recognition_flags_the_report_registry_profit_factor_sentinel() {
    let bars = (0..14)
        .map(|index| {
            let close = 100.0 + 2.0 * index as f64;
            Bar {
                timestamp: format!("2026-07-{:02}T00:00:00Z", index + 1),
                open: close - 1.0,
                high: close + 1.0,
                low: close - 2.0,
                close,
                volume: 100.0,
            }
        })
        .collect();
    let fixture = ProblemRecognitionFixture::from_bars(bars);
    let artifact = fixture.execute(problem_recognition_policy()).unwrap();

    assert!(
        artifact
            .observations()
            .absurd_metrics
            .profit_factor_at_sentinel
    );
    assert_eq!(failing_stage(&artifact), Some("absurd-metrics"));
    assert!(!artifact.passed());
    assert_eq!(replay_problem_recognition(&artifact).unwrap(), artifact);
}

#[test]
fn problem_recognition_refuses_foreign_tampered_and_mismatched_evidence() {
    let fixture = ProblemRecognitionFixture::new();
    let policy = problem_recognition_policy();
    let recognize = |oos: &ExecutedOosScheme| {
        execute_problem_recognition(&fixture.cross, oos, &fixture.significance, policy)
    };

    // A different candidate of the same field, executed on the same bars.
    let foreign_candidate = generate_candidates(
        &fixture.space,
        SearchMethod::Grid,
        fixture.space.combinations(),
    )
    .unwrap()
    .candidates
    .into_iter()
    .find(|candidate| candidate.candidate_id != fixture.strategy.strategy_id())
    .expect("the declared field has more than one coordinate")
    .strategy;
    assert!(
        recognize(&ProblemRecognitionFixture::oos(
            &foreign_candidate,
            &fixture.config,
            &fixture.manifest,
            &fixture.bars,
            "net_profit",
        ))
        .is_err()
    );

    // A different dataset, a different objective metric, and a different execution config all
    // make the degradation ratio incomparable with the cross-check baseline.
    let foreign_dataset = cross_check_manifest(&fixture.bars, "ETH/USD", "1Day", "fixture");
    assert!(
        recognize(&ProblemRecognitionFixture::oos(
            &fixture.strategy,
            &fixture.config,
            &foreign_dataset,
            &fixture.bars,
            "net_profit",
        ))
        .is_err()
    );
    assert!(
        recognize(&ProblemRecognitionFixture::oos(
            &fixture.strategy,
            &fixture.config,
            &fixture.manifest,
            &fixture.bars,
            "total_return",
        ))
        .is_err()
    );
    let zero_cost =
        StrategyExecutionConfig::build(&ExecutionSettings::conservative_defaults()).unwrap();
    assert_ne!(zero_cost.config_id(), fixture.config.config_id());
    assert!(
        recognize(&ProblemRecognitionFixture::oos(
            &fixture.strategy,
            &zero_cost,
            &fixture.manifest,
            &fixture.bars,
            "net_profit",
        ))
        .is_err()
    );

    // A cross-check whose candidate the significance family never judged.
    let foreign_cross = ProblemRecognitionFixture::cross_check(
        &foreign_candidate,
        &fixture.config,
        &fixture.manifest,
        &fixture.bars,
    );
    assert!(
        execute_problem_recognition(&foreign_cross, &fixture.oos, &fixture.significance, policy,)
            .is_err()
    );

    // Policies that cannot produce a bounded gate at all.
    for broken in [
        ProblemRecognitionPolicy {
            minimum_trades: 0,
            ..policy
        },
        ProblemRecognitionPolicy {
            boundary_width_bps: 0,
            ..policy
        },
        ProblemRecognitionPolicy {
            boundary_width_bps: 5_001,
            ..policy
        },
        ProblemRecognitionPolicy {
            maximum_time_in_market_bps: 10_001,
            ..policy
        },
    ] {
        assert!(fixture.execute(broken).is_err());
    }

    // Tampering: every mutation must fail both the sealed identity and, more importantly, the
    // re-derivation that a recomputed identity cannot rescue.
    let artifact = fixture.execute(policy).unwrap();
    let bytes = artifact.to_json_vec().unwrap();
    let strict = execute_significance_study(
        std::slice::from_ref(&fixture.field),
        significance_policy(100),
    )
    .unwrap();
    let swapped = zstd::bulk::compress(&strict.to_json_vec().unwrap(), 3).unwrap();
    for mutate in [
        (|value: &mut serde_json::Value| value["observations"]["trade_count"] = 3.into())
            as fn(&mut serde_json::Value),
        |value| value["observations"]["cost_2x_ratio_bps"] = 10_000.into(),
        |value| value["observations"]["cost_3x_ratio_bps"] = 10_000.into(),
        |value| value["observations"]["edge_concentration"]["side_share_bps"] = 5_000.into(),
        |value| value["observations"]["absurd_metrics"]["absolute_sharpe_bps"] = 1.into(),
        |value| value["observations"]["parameter_step"]["worst_step_ratio_bps"] = 10_000.into(),
        |value| value["stages"][0]["verdict"] = "Fail".into(),
        |value| value["stages"][8]["reason"] = "looks fine".into(),
        |value| value["passed"] = false.into(),
        |value| value["policy"]["minimum_trades"] = 2.into(),
        |value| value["metric_id"] = "total_return".into(),
        |value| value["strategy_id"] = "a".repeat(64).into(),
        |value| value["schema_version"] = 1.into(),
        |value| value["source_cross_check_zstd"][0] = 0.into(),
        |value| value["source_oos_zstd"][0] = 0.into(),
        |value| value["source_significance_zstd"][0] = 0.into(),
        |value| value["extra_field"] = 1.into(),
    ] {
        let mut tampered: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        mutate(&mut tampered);
        let tampered = serde_json::to_vec(&tampered).unwrap();
        assert!(ProblemRecognitionArtifact::from_json_slice(&tampered).is_err());
        assert!(ProblemRecognitionArtifact::resealed_from_json(&tampered).is_err());
    }

    // Swapping in a different but internally valid study still contradicts the sealed verdict.
    let mut mismatched: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    mismatched["source_significance_zstd"] = serde_json::to_value(&swapped).unwrap();
    let mismatched = serde_json::to_vec(&mismatched).unwrap();
    assert!(ProblemRecognitionArtifact::from_json_slice(&mismatched).is_err());
    assert!(ProblemRecognitionArtifact::resealed_from_json(&mismatched).is_err());

    // Bounded before decode.
    assert!(
        ProblemRecognitionArtifact::from_json_slice(&vec![b' '; MAX_ARTIFACT_BYTES + 1]).is_err()
    );
}
