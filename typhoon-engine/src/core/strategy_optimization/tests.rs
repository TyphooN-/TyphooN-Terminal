use super::*;
use crate::core::strategy_ir::{
    DatasetBinding, ParamRange, ParamValue, RunBinding, StrategyIr, StrategyParameter,
    StrategyRunManifest,
};
use crate::core::strategy_metrics::METRICS_SCHEMA_VERSION;
use crate::core::strategy_report::StrategyReportArtifact;

fn base_strategy() -> StrategyIr {
    let builder = crate::core::strategy_builder::GeneralStrategyBuilder::new("optimizer", "test");
    let mut definition = builder.definition().clone();
    definition.parameters = vec![
        StrategyParameter {
            id: "fast".into(),
            value: ParamValue::Int(2),
            range: Some(ParamRange::Int { min: 2, max: 6 }),
        },
        StrategyParameter {
            id: "threshold".into(),
            value: ParamValue::Float(0.0),
            range: Some(ParamRange::Float {
                min: -1.0,
                max: 1.0,
            }),
        },
    ];
    StrategyIr::build(&definition).unwrap()
}

fn space() -> SearchSpace {
    SearchSpace::new(
        base_strategy(),
        vec![
            ParameterDomain::new(
                "fast",
                vec![ParamValue::Int(2), ParamValue::Int(4), ParamValue::Int(6)],
            )
            .unwrap(),
            ParameterDomain::new(
                "threshold",
                vec![
                    ParamValue::Float(-1.0),
                    ParamValue::Float(0.0),
                    ParamValue::Float(1.0),
                ],
            )
            .unwrap(),
        ],
    )
    .unwrap()
}

const RUN_SEED: u64 = 17;
const INITIAL_EQUITY: f64 = 1_000.0;

fn run_config_id() -> String {
    "c".repeat(64)
}

/// Seal a report through the ordinary manifest + metrics boundary. Observations are never
/// hand-built metric vectors: every value the optimizer sees came out of a verified run.
fn sealed_report(
    dataset_id: &str,
    candidate: &StrategyIr,
    ending_equity: f64,
) -> StrategyReportArtifact {
    let manifest = StrategyRunManifest::build(&RunBinding {
        datasets: vec![DatasetBinding {
            input_id: "primary".into(),
            dataset_id: dataset_id.to_string(),
        }],
        sub_bar_datasets: vec![],
        strategy_id: candidate.strategy_id().to_string(),
        config_id: run_config_id(),
        seed: RUN_SEED,
        engine_version: "typhoon-engine/optimization-test".into(),
        metrics_version: METRICS_SCHEMA_VERSION.into(),
        intervention_log_id: None,
        repaint_qa: vec![],
    })
    .unwrap();
    let simulator = crate::core::strategy_metrics::tests::report(
        vec![],
        &[(0, INITIAL_EQUITY), (86_400_000_000_001, ending_equity)],
    );
    StrategyReportArtifact::build(&manifest, &simulator, &[], INITIAL_EQUITY).unwrap()
}

fn sealed_request(lease: &SearchDataLease, candidate: &StrategyIr) -> RetestRequest {
    RetestRequest::seal(
        candidate,
        lease,
        run_config_id(),
        METRICS_SCHEMA_VERSION,
        RUN_SEED,
    )
    .unwrap()
}

fn sealed_observation(
    lease: &SearchDataLease,
    role: ObservationRole,
    candidate: &StrategyIr,
    ending_equity: f64,
) -> ReportObservation {
    let report = sealed_report(lease.dataset_id(), candidate, ending_equity);
    let request = sealed_request(lease, candidate);
    let result = RetestResult::seal(&request, report.report_id()).unwrap();
    ReportObservation::from_report(lease, role, &request, &result, &report, &["total_return"])
        .unwrap()
}

#[test]
fn grid_random_and_local_search_are_deterministic_deduplicated_and_budgeted() {
    let grid = generate_candidates(&space(), SearchMethod::Grid, 5).unwrap();
    assert_eq!(grid.candidates.len(), 5);
    assert_eq!(grid.evaluations_n, 5);
    assert!(grid.exhausted_budget);
    assert_eq!(
        grid,
        generate_candidates(&space(), SearchMethod::Grid, 5).unwrap()
    );

    let random = generate_candidates(&space(), SearchMethod::Random { seed: 44 }, 7).unwrap();
    assert_eq!(
        random,
        generate_candidates(&space(), SearchMethod::Random { seed: 44 }, 7).unwrap()
    );
    assert_eq!(random.candidates.len(), 7);

    let local = generate_candidates(&space(), SearchMethod::Local, 9).unwrap();
    assert_eq!(local.candidates.len(), 9);
    let ids: std::collections::BTreeSet<_> = local
        .candidates
        .iter()
        .map(|c| c.candidate_id.as_str())
        .collect();
    assert_eq!(ids.len(), local.candidates.len());
    assert!(
        local
            .candidates
            .iter()
            .all(|candidate| candidate.strategy.verify().is_ok())
    );
}

#[test]
fn candidate_identity_is_canonical_and_invalid_spaces_fail_closed() {
    let a = ParameterDomain::new(
        "fast",
        vec![ParamValue::Int(4), ParamValue::Int(2), ParamValue::Int(4)],
    )
    .unwrap();
    let b = ParameterDomain::new("fast", vec![ParamValue::Int(2), ParamValue::Int(4)]).unwrap();
    assert_eq!(a, b);
    let first = SearchSpace::new(
        base_strategy(),
        vec![
            a,
            ParameterDomain::new("threshold", vec![ParamValue::Float(0.0)]).unwrap(),
        ],
    )
    .unwrap();
    let second = SearchSpace::new(
        base_strategy(),
        vec![
            b,
            ParameterDomain::new("threshold", vec![ParamValue::Float(-0.0)]).unwrap(),
        ],
    )
    .unwrap();
    assert_eq!(
        generate_candidates(&first, SearchMethod::Grid, 2)
            .unwrap()
            .candidates[0]
            .candidate_id,
        generate_candidates(&second, SearchMethod::Grid, 2)
            .unwrap()
            .candidates[0]
            .candidate_id
    );
    assert!(matches!(
        ParameterDomain::new("threshold", vec![ParamValue::Float(f64::NAN)]),
        Err(OptimizationError::NonFiniteParameter { .. })
    ));
    assert!(matches!(
        generate_candidates(&space(), SearchMethod::Grid, 0),
        Err(OptimizationError::InvalidBudget { .. })
    ));
}

#[test]
fn purged_embargoed_holdout_and_walk_forward_folds_are_causal_and_deterministic() {
    let holdout = FoldPlan::trailing_holdout(100, 20, 3, 4).unwrap();
    assert_eq!(
        holdout.folds(),
        &[Fold {
            train: 0..73,
            test: 80..100
        }]
    );
    assert!(
        holdout
            .folds()
            .iter()
            .all(|fold| fold.train.end <= fold.test.start)
    );

    let rolling = FoldPlan::walk_forward(
        100,
        WalkForwardConfig {
            train_bars: 30,
            test_bars: 10,
            step_bars: 10,
            purge_bars: 2,
            embargo_bars: 3,
            anchored: false,
        },
    )
    .unwrap();
    let anchored = FoldPlan::walk_forward(
        100,
        WalkForwardConfig {
            anchored: true,
            ..WalkForwardConfig {
                train_bars: 30,
                test_bars: 10,
                step_bars: 10,
                purge_bars: 2,
                embargo_bars: 3,
                anchored: false,
            }
        },
    )
    .unwrap();
    assert_eq!(
        rolling,
        FoldPlan::walk_forward(
            100,
            WalkForwardConfig {
                train_bars: 30,
                test_bars: 10,
                step_bars: 10,
                purge_bars: 2,
                embargo_bars: 3,
                anchored: false
            }
        )
        .unwrap()
    );
    assert!(
        rolling
            .folds()
            .iter()
            .all(|fold| fold.train.end + 5 <= fold.test.start)
    );
    assert!(
        anchored
            .folds()
            .windows(2)
            .all(|pair| pair[0].train.start == 0 && pair[1].train.start == 0)
    );
}

#[test]
fn calendar_walk_forward_uses_irregular_time_not_bar_counts_and_distinguishes_rolling_from_anchored()
 {
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
    ]
    .map(str::to_string);
    let config = CalendarWalkForwardConfig {
        train_seconds: 4 * 86_400,
        test_seconds: 4 * 86_400,
        step_seconds: 5 * 86_400,
        purge_seconds: 86_400,
        embargo_seconds: 86_400,
        anchored: false,
    };
    let rolling = CalendarFoldPlan::walk_forward(&timestamps, config).unwrap();
    let anchored = CalendarFoldPlan::walk_forward(
        &timestamps,
        CalendarWalkForwardConfig {
            anchored: true,
            ..config
        },
    )
    .unwrap();

    assert_eq!(rolling.folds()[0].train, 0..2);
    assert_eq!(rolling.folds()[0].purged, 2..3);
    assert_eq!(rolling.folds()[0].embargoed, 3..4);
    assert_eq!(rolling.folds()[0].test, 4..5);
    assert_eq!(rolling.folds()[1].train, 3..4);
    assert_eq!(anchored.folds()[1].train, 0..4);
    assert_ne!(
        rolling.folds()[0].train,
        FoldPlan::walk_forward(
            timestamps.len(),
            WalkForwardConfig {
                train_bars: 4,
                test_bars: 4,
                step_bars: 5,
                purge_bars: 1,
                embargo_bars: 1,
                anchored: false,
            },
        )
        .unwrap()
        .folds()[0]
            .train
    );
    assert_eq!(
        rolling,
        CalendarFoldPlan::walk_forward(&timestamps, config).unwrap()
    );
}

#[test]
fn calendar_walk_forward_fails_closed_on_bad_or_unbounded_time_evidence() {
    let valid = [
        "2026-01-01T00:00:00Z".to_string(),
        "2026-02-01T00:00:00Z".to_string(),
    ];
    let config = CalendarWalkForwardConfig {
        train_seconds: 86_400,
        test_seconds: 86_400,
        step_seconds: 86_400,
        purge_seconds: 0,
        embargo_seconds: 0,
        anchored: false,
    };
    for timestamps in [
        vec!["not-iso-8601".into(), valid[1].clone()],
        vec![valid[1].clone(), valid[0].clone()],
        vec![valid[0].clone(), valid[0].clone()],
        vec![
            "2026-01-01T00:00:00Z".into(),
            "2025-12-31T19:00:00-05:00".into(),
        ],
    ] {
        assert!(CalendarFoldPlan::walk_forward(&timestamps, config).is_err());
    }
    assert!(
        CalendarFoldPlan::walk_forward(
            &valid,
            CalendarWalkForwardConfig {
                train_seconds: MAX_CALENDAR_WINDOW_SECONDS + 1,
                ..config
            },
        )
        .is_err()
    );
    assert!(
        CalendarFoldPlan::walk_forward(
            &["9999-12-30T00:00:00Z".into(), "9999-12-31T00:00:00Z".into()],
            config,
        )
        .is_err()
    );
    assert!(CalendarFoldPlan::walk_forward(&valid[..1], config).is_err());
}

#[test]
fn search_api_refuses_holdout_and_holdout_is_one_way_burned() {
    let quarantine = HoldoutQuarantine::new("a".repeat(64), "f".repeat(64), 100, 20).unwrap();
    assert_eq!(quarantine.search_range().unwrap(), 0..80);
    assert!(matches!(
        quarantine.range_for(StageAccess::Search, DataRegion::FinalHoldout),
        Err(OptimizationError::HoldoutForbidden)
    ));
    let burned = quarantine.burn("final-review").unwrap();
    assert_eq!(burned.range(), 80..100);
    assert_eq!(burned.reason(), "final-review");
}

#[test]
fn perturbation_monte_carlo_and_plateau_evidence_are_seeded_bounded_and_explicit() {
    let perturbations =
        ExecutionPerturbationGrid::new(vec![100, 200], vec![100, 150], vec![0, 5]).unwrap();
    assert_eq!(perturbations.cases().len(), 8);
    let delayed = perturbations.cases()[1]
        .apply_strategy(&base_strategy())
        .unwrap();
    assert_eq!(delayed.definition().timing.submit_delay_bars, 5);
    assert_ne!(delayed.strategy_id(), base_strategy().strategy_id());
    assert_eq!(
        perturbations,
        ExecutionPerturbationGrid::new(vec![200, 100], vec![150, 100], vec![5, 0]).unwrap()
    );

    let trades = [3.0, -1.0, 2.0, -2.0, 4.0];
    let shuffle = monte_carlo_trade_returns(&trades, MonteCarloMethod::TradeOrder, 9, 16).unwrap();
    assert_eq!(
        shuffle,
        monte_carlo_trade_returns(&trades, MonteCarloMethod::TradeOrder, 9, 16).unwrap()
    );
    assert_eq!(shuffle.samples.len(), 16);
    let bootstrap = monte_carlo_trade_returns(&trades, MonteCarloMethod::Bootstrap, 9, 16).unwrap();
    assert_ne!(shuffle.samples, bootstrap.samples);

    let evidence = parameter_plateau_evidence(
        10.0,
        &[9.8, 9.5, 10.1, 9.9],
        PlateauPolicy {
            minimum_neighbour_ratio_bps: 9000,
            minimum_passing_neighbours: 3,
        },
    )
    .unwrap();
    assert_eq!(evidence.verdict, StageVerdict::Pass);
    assert_eq!(evidence.observations_n, 4);
    let rejected = parameter_plateau_evidence(
        10.0,
        &[1.0, 1.1, 1.2],
        PlateauPolicy {
            minimum_neighbour_ratio_bps: 9000,
            minimum_passing_neighbours: 2,
        },
    )
    .unwrap();
    assert_eq!(rejected.verdict, StageVerdict::Fail);
    assert!(!rejected.reason.is_empty());
}

#[test]
fn robustness_artifact_is_deterministic_immutable_and_every_best_names_n() {
    let stages = vec![
        StageEvidence::pass("minimum-trades", 40, "40 >= 30"),
        StageEvidence::fail("cost-2x", 1, "net profit became non-positive"),
    ];
    let artifact = RobustnessArtifact::seal("candidate-a", 17, stages.clone()).unwrap();
    assert_eq!(
        artifact,
        RobustnessArtifact::seal("candidate-a", 17, stages).unwrap()
    );
    assert_eq!(artifact.verdict(), StageVerdict::Fail);
    assert!(artifact.best_label(12.5).contains("best of N=17"));
    let bytes = artifact.to_json_vec().unwrap();
    assert_eq!(
        artifact,
        RobustnessArtifact::from_json_slice(&bytes).unwrap()
    );
    let mut tampered: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    tampered["evaluations_n"] = 18.into();
    assert!(RobustnessArtifact::from_json_slice(&serde_json::to_vec(&tampered).unwrap()).is_err());
}

#[test]
fn worker_is_bounded_off_thread_and_reports_failure_without_installing_a_result() {
    let submitter = std::thread::current().id();
    let worker = OptimizationWorker::spawn(1, 2).unwrap();
    worker
        .try_submit(OptimizationJob::Generate {
            request_id: 7,
            space: space(),
            method: SearchMethod::Grid,
            budget: 3,
        })
        .unwrap();
    let mut saw_backpressure = false;
    for request_id in 8..64 {
        if matches!(
            worker.try_submit(OptimizationJob::Generate {
                request_id,
                space: space(),
                method: SearchMethod::Grid,
                budget: 3,
            }),
            Err(SubmitError::Backpressure(_))
        ) {
            saw_backpressure = true;
            break;
        }
    }
    assert!(
        saw_backpressure,
        "bounded queue must eventually reject work"
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    let mut completed = None;
    while std::time::Instant::now() < deadline {
        for event in worker.poll() {
            if let OptimizationWorkerEvent::Completed {
                worker_thread,
                batch,
                ..
            } = event
            {
                completed = Some((worker_thread, batch));
            }
        }
        if completed.is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let (worker_thread, batch) = completed.expect("worker completion");
    assert_ne!(worker_thread, submitter);
    assert_eq!(batch.candidates.len(), 3);
}

#[test]
fn retest_identity_binds_the_stored_strategy_and_every_changed_run_input() {
    let strategy = base_strategy();
    let primary = HoldoutQuarantine::new("a".repeat(64), "f".repeat(64), 100, 20).unwrap();
    let lease = primary.lease(StageAccess::Search).unwrap();
    let request =
        RetestRequest::seal(&strategy, &lease, "execution-cost-2x", "metrics-v2", 91).unwrap();
    assert_eq!(request.strategy_id(), strategy.strategy_id());
    assert_eq!(request.dataset_id(), "a".repeat(64));
    assert_eq!(
        request,
        RetestRequest::seal(&strategy, &lease, "execution-cost-2x", "metrics-v2", 91).unwrap()
    );

    // Every changed run input moves the identity: the dataset, the cost model, the metric
    // version, the seed, and the leased partition the evidence was produced on.
    let other_dataset = HoldoutQuarantine::new("b".repeat(64), "f".repeat(64), 100, 20).unwrap();
    let other_lease = other_dataset.lease(StageAccess::Search).unwrap();
    let narrower = HoldoutQuarantine::new("a".repeat(64), "f".repeat(64), 100, 40).unwrap();
    let narrower_lease = narrower.lease(StageAccess::Search).unwrap();
    let variants = [
        RetestRequest::seal(
            &strategy,
            &other_lease,
            "execution-cost-2x",
            "metrics-v2",
            91,
        )
        .unwrap(),
        RetestRequest::seal(
            &strategy,
            &narrower_lease,
            "execution-cost-2x",
            "metrics-v2",
            91,
        )
        .unwrap(),
        RetestRequest::seal(&strategy, &lease, "execution-cost-1x", "metrics-v2", 91).unwrap(),
        RetestRequest::seal(&strategy, &lease, "execution-cost-2x", "metrics-v3", 91).unwrap(),
        RetestRequest::seal(&strategy, &lease, "execution-cost-2x", "metrics-v2", 92).unwrap(),
        RetestRequest::seal(
            &strategy,
            &primary.lease(StageAccess::Robustness).unwrap(),
            "execution-cost-2x",
            "metrics-v2",
            91,
        )
        .unwrap(),
    ];
    for variant in &variants {
        assert_ne!(request.request_id(), variant.request_id());
    }
    let ids: std::collections::BTreeSet<_> = variants
        .iter()
        .map(|variant| variant.request_id())
        .collect();
    assert_eq!(ids.len(), variants.len(), "each changed input is distinct");

    let result = RetestResult::seal(&request, "sealed-report-id").unwrap();
    assert_eq!(result.request_id(), request.request_id());
    assert!(result.verify_against(&request).is_ok());
    assert!(result.verify_against(&variants[0]).is_err());
    assert!(RetestRequest::seal(&strategy, &lease, "", "metrics-v2", 91).is_err());
    assert!(RetestRequest::seal(&strategy, &lease, "config", "", 91).is_err());
    assert!(RetestResult::seal(&request, "").is_err());
}

#[test]
fn every_oos_scheme_records_exact_roles_with_purge_and_embargo() {
    let leading = OosPlan::new(30, OosScheme::Leading { oos_bars: 6 }, 2, 3).unwrap();
    assert_eq!(leading.role_ranges(SampleRole::OutOfSample), &[0..6]);
    assert_eq!(leading.role_ranges(SampleRole::Embargoed), &[6..9]);
    assert_eq!(leading.role_ranges(SampleRole::InSample), &[9..30]);

    let trailing = OosPlan::new(30, OosScheme::Trailing { oos_bars: 6 }, 2, 3).unwrap();
    assert_eq!(trailing.role_ranges(SampleRole::InSample), &[0..22]);
    assert_eq!(trailing.role_ranges(SampleRole::Purged), &[22..24]);
    assert_eq!(trailing.role_ranges(SampleRole::OutOfSample), &[24..30]);

    let striped = OosPlan::new(
        30,
        OosScheme::Interleaved {
            in_sample_bars: 6,
            oos_bars: 3,
        },
        1,
        1,
    )
    .unwrap();
    assert_eq!(
        striped.role_ranges(SampleRole::OutOfSample),
        &[6..9, 15..18, 24..27]
    );
    assert_eq!(striped.roles().len(), 30);

    let disjoint = OosPlan::new(
        40,
        OosScheme::Disjoint {
            windows: vec![5..10, 25..30],
        },
        2,
        2,
    )
    .unwrap();
    assert_eq!(
        disjoint.role_ranges(SampleRole::OutOfSample),
        &[5..10, 25..30]
    );
    assert!(
        OosPlan::new(
            20,
            OosScheme::Disjoint {
                windows: vec![3..8, 7..9]
            },
            1,
            1
        )
        .is_err()
    );
}

#[test]
fn walk_forward_optimization_and_matrix_preserve_per_window_oos_evidence() {
    let rolling = FoldPlan::walk_forward(
        60,
        WalkForwardConfig {
            train_bars: 20,
            test_bars: 5,
            step_bars: 10,
            purge_bars: 1,
            embargo_bars: 1,
            anchored: false,
        },
    )
    .unwrap();
    let windows: Vec<_> = rolling
        .folds()
        .iter()
        .enumerate()
        .map(|(index, fold)| WalkForwardWindowEvidence {
            fold: fold.clone(),
            selected_candidate_id: format!("candidate-{index}"),
            evaluations_n: 7,
            is_score: 10.0 - index as f64,
            oos_score: 8.0 - index as f64,
        })
        .collect();
    let evidence = WalkForwardEvidence::new(rolling.clone(), windows).unwrap();
    assert_eq!(
        evidence.concatenated_oos(),
        &[22..27, 32..37, 42..47, 52..57]
    );
    assert_eq!(evidence.degradation_bps(), &[-2000, -2222, -2500, -2857]);

    let anchored = FoldPlan::walk_forward(
        60,
        WalkForwardConfig {
            anchored: true,
            ..WalkForwardConfig {
                train_bars: 20,
                test_bars: 5,
                step_bars: 10,
                purge_bars: 1,
                embargo_bars: 1,
                anchored: false,
            }
        },
    )
    .unwrap();
    let matrix = WalkForwardMatrix::new(vec![
        WalkForwardMatrixCell::new(20, 5, evidence.clone()).unwrap(),
        WalkForwardMatrixCell::new(
            30,
            5,
            WalkForwardEvidence::new(
                anchored.clone(),
                anchored
                    .folds()
                    .iter()
                    .map(|fold| WalkForwardWindowEvidence {
                        fold: fold.clone(),
                        selected_candidate_id: "anchored".into(),
                        evaluations_n: 4,
                        is_score: 5.0,
                        oos_score: 4.0,
                    })
                    .collect(),
            )
            .unwrap(),
        )
        .unwrap(),
    ])
    .unwrap();
    assert_eq!(matrix.cells().len(), 2);
    assert!(WalkForwardEvidence::new(rolling, Vec::new()).is_err());
}

#[test]
fn deterministic_variation_profile_cross_checks_and_adjustment_are_bounded() {
    let plan = DeterministicVariationPlan::new(
        12,
        VariationConfig {
            trials: 8,
            trade_count: 20,
            trade_skip_bps: 1500,
            parameter_jitter_bps: 500,
            data_noise_bps: 25,
            maximum_start_offset: 4,
        },
    )
    .unwrap();
    assert_eq!(
        plan,
        DeterministicVariationPlan::new(12, plan.config()).unwrap()
    );
    assert_eq!(plan.cases().len(), 8);
    assert!(plan.cases().iter().all(|case| {
        case.kept_trade_indices.windows(2).all(|w| w[0] < w[1])
            && case.parameter_delta_bps.abs() <= 500
            && case.data_noise_bps.abs() <= 25
            && case.start_offset <= 4
    }));

    let profile = parameter_field_profile(&[9.0, 10.0, 11.0, 10.0, 10.0]).unwrap();
    assert_eq!(profile.spp_median, 10.0);
    assert_eq!(profile.optimization_profile_stability_bps, 9000);
    let cross = cross_check_gate(
        10.0,
        &[
            CrossCheckObservation::new("other-market", 8.0).unwrap(),
            CrossCheckObservation::new("cost-2x", 7.0).unwrap(),
        ],
        6500,
    )
    .unwrap();
    assert_eq!(cross.verdict, StageVerdict::Pass);
    let adjusted = bounded_bonferroni_adjustment(0.001, 100).unwrap();
    assert_eq!(adjusted.evaluations_n, 100);
    assert_eq!(adjusted.adjusted_p, 0.1);
    assert!(bounded_bonferroni_adjustment(0.2, MAX_TRIAL_BUDGET + 1).is_err());
}

#[test]
fn problem_recognition_and_degradation_reject_explicit_failure_paths() {
    let stages = problem_recognition_gates(
        ProblemObservations {
            trade_count: 8,
            top_trade_share_bps: 7000,
            time_in_market_bps: 9800,
            boundary_trade_share_bps: 4000,
            cost_2x_ratio_bps: 2000,
            oos_is_ratio_bps: 3000,
        },
        ProblemPolicy {
            minimum_trades: 30,
            maximum_top_trade_share_bps: 5000,
            maximum_time_in_market_bps: 9500,
            maximum_boundary_trade_share_bps: 2500,
            minimum_cost_2x_ratio_bps: 5000,
            minimum_oos_is_ratio_bps: 6000,
        },
    )
    .unwrap();
    assert_eq!(stages.len(), 6);
    assert!(
        stages
            .iter()
            .all(|stage| stage.verdict == StageVerdict::Fail)
    );
}

#[test]
fn final_holdout_is_linear_and_synthetic_curve_fit_gate_is_literal() {
    let quarantine = HoldoutQuarantine::new("a".repeat(64), "f".repeat(64), 24, 4).unwrap();
    let search = quarantine.lease(StageAccess::Search).unwrap();
    let robustness = quarantine.lease(StageAccess::Robustness).unwrap();
    assert_eq!(search.range(), 0..20);
    assert_eq!(robustness.range(), 0..20);
    assert!(quarantine.lease(StageAccess::FinalReview).is_err());
    assert!(matches!(
        quarantine.range_for(StageAccess::Search, DataRegion::FinalHoldout),
        Err(OptimizationError::HoldoutForbidden)
    ));

    let random_curve_fit = synthetic_edge_gate(
        &search,
        &[4.0, 4.0, 4.0, 4.0],
        &[1.0, -1.0, 1.0, -1.0],
        SyntheticGatePolicy {
            minimum_oos_mean: 0.25,
            minimum_oos_is_ratio_bps: 5000,
        },
    )
    .unwrap();
    assert_eq!(random_curve_fit.verdict, StageVerdict::Fail);
    let planted_edge = synthetic_edge_gate(
        &robustness,
        &[1.0, 1.2, 0.8, 1.1],
        &[0.9, 1.0, 0.8, 1.1],
        SyntheticGatePolicy {
            minimum_oos_mean: 0.25,
            minimum_oos_is_ratio_bps: 5000,
        },
    )
    .unwrap();
    assert_eq!(planted_edge.verdict, StageVerdict::Pass);

    drop(search);
    drop(robustness);
    let burned = quarantine.burn("single final review").unwrap();
    assert_eq!(burned.range(), 20..24);
    assert_eq!(burned.reason(), "single final review");
    // `burn` consumes the quarantine and the non-Clone token exposes no search lease.
}

#[test]
fn latin_hypercube_search_is_seeded_bounded_and_covers_each_discrete_axis() {
    let first =
        generate_candidates(&space(), SearchMethod::LatinHypercube { seed: 991 }, 3).unwrap();
    assert_eq!(
        first,
        generate_candidates(&space(), SearchMethod::LatinHypercube { seed: 991 }, 3).unwrap()
    );
    assert_eq!(first.evaluations_n, 3);
    for parameter in ["fast", "threshold"] {
        let values: std::collections::BTreeSet<_> = first
            .candidates
            .iter()
            .map(|candidate| {
                candidate
                    .assignments
                    .iter()
                    .find(|(id, _)| id == parameter)
                    .map(|(_, value)| format!("{value:?}"))
                    .unwrap()
            })
            .collect();
        assert_eq!(values.len(), 3, "every stratum on {parameter} is covered");
    }
}

#[test]
fn declarative_pipeline_consumes_only_identity_verified_report_observations() {
    let quarantine = HoldoutQuarantine::new("a".repeat(64), "f".repeat(64), 100, 20).unwrap();
    let lease = quarantine.lease(StageAccess::Robustness).unwrap();
    let candidate = base_strategy();
    let observations = vec![
        sealed_observation(&lease, ObservationRole::InSample, &candidate, 1_100.0),
        sealed_observation(&lease, ObservationRole::OutOfSample, &candidate, 1_080.0),
        sealed_observation(&lease, ObservationRole::OutOfSample, &candidate, 1_090.0),
    ];
    let pipeline = RobustnessPipeline::new(vec![
        RobustnessStageSpec::metric_percentile(
            20,
            "oos-floor",
            ObservationRole::OutOfSample,
            "total_return",
            Percentile::P05,
            Threshold::AtLeast(0.05),
        ),
        RobustnessStageSpec::degradation_ratio(
            30,
            "oos-vs-is",
            "total_return",
            Percentile::Median,
            5_000,
        ),
    ])
    .unwrap();
    let outcome = pipeline
        .execute(&lease, candidate.strategy_id(), 11, observations)
        .unwrap();
    assert_eq!(outcome.artifact().verdict(), StageVerdict::Pass);
    assert_eq!(outcome.executed_stages(), 2);
    assert_eq!(outcome.distributions().len(), 3);
    assert!(outcome.artifact().best_label(0.09).contains("best of N=11"));
}

#[test]
fn pipeline_propagates_first_failure_in_deterministic_stage_order() {
    let quarantine = HoldoutQuarantine::new("a".repeat(64), "f".repeat(64), 40, 8).unwrap();
    let lease = quarantine.lease(StageAccess::Robustness).unwrap();
    let candidate = base_strategy();
    let observations = vec![sealed_observation(
        &lease,
        ObservationRole::OutOfSample,
        &candidate,
        990.0,
    )];
    let pipeline = RobustnessPipeline::new(vec![
        RobustnessStageSpec::metric_percentile(
            99,
            "must-not-run",
            ObservationRole::OutOfSample,
            "total_return",
            Percentile::Median,
            Threshold::AtLeast(-1.0),
        ),
        RobustnessStageSpec::metric_percentile(
            1,
            "cheap-rejection",
            ObservationRole::OutOfSample,
            "total_return",
            Percentile::Median,
            Threshold::AtLeast(0.0),
        ),
    ])
    .unwrap();
    let outcome = pipeline
        .execute(&lease, candidate.strategy_id(), 1, observations)
        .unwrap();
    assert_eq!(outcome.artifact().verdict(), StageVerdict::Fail);
    assert_eq!(outcome.executed_stages(), 1);
    assert_eq!(outcome.failed_stage(), Some("cheap-rejection"));
    assert_eq!(outcome.artifact().stages()[0].stage, "cheap-rejection");
}

#[test]
fn exact_synthetic_report_pipeline_rejects_curve_fit_and_preserves_planted_edge() {
    let quarantine = HoldoutQuarantine::new("a".repeat(64), "f".repeat(64), 120, 20).unwrap();
    let lease = quarantine.lease(StageAccess::Robustness).unwrap();
    let pipeline = RobustnessPipeline::new(vec![
        RobustnessStageSpec::metric_percentile(
            1,
            "oos-positive",
            ObservationRole::OutOfSample,
            "total_return",
            Percentile::P05,
            Threshold::AtLeast(0.02),
        ),
        RobustnessStageSpec::degradation_ratio(
            2,
            "stable-oos-ratio",
            "total_return",
            Percentile::Median,
            5_000,
        ),
    ])
    .unwrap();

    let random_fit = base_strategy();
    let random_outcome = pipeline
        .execute(
            &lease,
            random_fit.strategy_id(),
            101,
            vec![
                sealed_observation(&lease, ObservationRole::InSample, &random_fit, 1_400.0),
                sealed_observation(&lease, ObservationRole::OutOfSample, &random_fit, 1_000.0),
                sealed_observation(&lease, ObservationRole::OutOfSample, &random_fit, 990.0),
            ],
        )
        .unwrap();
    assert_eq!(random_outcome.artifact().verdict(), StageVerdict::Fail);

    let planted = generate_candidates(&space(), SearchMethod::Grid, 2)
        .unwrap()
        .candidates
        .pop()
        .unwrap()
        .strategy;
    let planted_outcome = pipeline
        .execute(
            &lease,
            planted.strategy_id(),
            101,
            vec![
                sealed_observation(&lease, ObservationRole::InSample, &planted, 1_100.0),
                sealed_observation(&lease, ObservationRole::OutOfSample, &planted, 1_080.0),
                sealed_observation(&lease, ObservationRole::OutOfSample, &planted, 1_090.0),
            ],
        )
        .unwrap();
    assert_eq!(planted_outcome.artifact().verdict(), StageVerdict::Pass);
}

#[test]
fn search_and_robustness_cannot_name_or_obtain_the_final_holdout_partition() {
    let search_id = "a".repeat(64);
    let holdout_id = "f".repeat(64);
    let quarantine = HoldoutQuarantine::new(search_id.clone(), holdout_id.clone(), 50, 10).unwrap();
    for stage in [StageAccess::Search, StageAccess::Robustness] {
        let lease = quarantine.lease(stage).unwrap();
        assert_eq!(lease.dataset_id(), search_id);
        assert_ne!(lease.dataset_id(), holdout_id);
        let request = RetestRequest::seal(
            &base_strategy(),
            &lease,
            "c".repeat(64),
            METRICS_SCHEMA_VERSION,
            3,
        )
        .unwrap();
        assert_eq!(request.dataset_id(), search_id);
    }
    assert!(quarantine.lease(StageAccess::FinalReview).is_err());
    let burned = quarantine.burn("one-way final review").unwrap();
    assert_eq!(burned.dataset_id(), holdout_id);
}

#[test]
fn pareto_and_single_objective_best_results_are_report_derived_and_always_name_n() {
    let quarantine = HoldoutQuarantine::new("a".repeat(64), "f".repeat(64), 80, 10).unwrap();
    let lease = quarantine.lease(StageAccess::Search).unwrap();
    let candidates = generate_candidates(&space(), SearchMethod::Grid, 3)
        .unwrap()
        .candidates;
    let observations = candidates
        .iter()
        .zip([1_050.0, 1_100.0, 1_075.0])
        .map(|(candidate, equity)| {
            sealed_observation(
                &lease,
                ObservationRole::SearchEvaluation,
                &candidate.strategy,
                equity,
            )
        })
        .collect::<Vec<_>>();
    let objective = ObjectiveSpec::new("total_return", ObjectiveDirection::Maximize).unwrap();
    let best = select_best(&observations, &objective).unwrap();
    assert!(best.label().contains("best of N=3"));
    assert_eq!(best.candidate_id(), candidates[1].candidate_id);

    let front = pareto_front(&observations, &[objective]).unwrap();
    assert_eq!(front.members().len(), 1);
    assert!(front.label().contains("Pareto front of N=3"));
    assert!(front.members()[0].label().contains("best of N=3"));
}

#[test]
fn report_observations_refuse_foreign_tampered_and_undefined_metric_evidence() {
    let quarantine = HoldoutQuarantine::new("a".repeat(64), "f".repeat(64), 100, 20).unwrap();
    let lease = quarantine.lease(StageAccess::Robustness).unwrap();
    let candidate = base_strategy();
    let report = sealed_report(lease.dataset_id(), &candidate, 1_100.0);
    let request = sealed_request(&lease, &candidate);
    let result = RetestResult::seal(&request, report.report_id()).unwrap();
    let observe = |request: &RetestRequest,
                   result: &RetestResult,
                   report: &StrategyReportArtifact,
                   lease: &SearchDataLease,
                   metrics: &[&str]| {
        ReportObservation::from_report(
            lease,
            ObservationRole::OutOfSample,
            request,
            result,
            report,
            metrics,
        )
    };
    assert!(observe(&request, &result, &report, &lease, &["total_return"]).is_ok());

    // A report sealed for a different strategy is not this candidate's evidence.
    let foreign_strategy = generate_candidates(&space(), SearchMethod::Grid, 3)
        .unwrap()
        .candidates
        .pop()
        .unwrap()
        .strategy;
    let foreign_report = sealed_report(lease.dataset_id(), &foreign_strategy, 1_100.0);
    let foreign_result = RetestResult::seal(&request, foreign_report.report_id()).unwrap();
    assert!(
        observe(
            &request,
            &foreign_result,
            &foreign_report,
            &lease,
            &["total_return"]
        )
        .is_err()
    );

    // A report sealed on a different dataset is not this partition's evidence.
    let foreign_dataset_report = sealed_report(&"b".repeat(64), &candidate, 1_100.0);
    let foreign_dataset_result =
        RetestResult::seal(&request, foreign_dataset_report.report_id()).unwrap();
    assert!(
        observe(
            &request,
            &foreign_dataset_result,
            &foreign_dataset_report,
            &lease,
            &["total_return"],
        )
        .is_err()
    );

    // A result that names some other report cannot import this one's numbers.
    let other_report = sealed_report(lease.dataset_id(), &candidate, 1_200.0);
    assert!(observe(&request, &result, &other_report, &lease, &["total_return"]).is_err());

    // A request sealed under one lease cannot be replayed against another partition.
    let other_quarantine = HoldoutQuarantine::new("b".repeat(64), "f".repeat(64), 100, 20).unwrap();
    let other_lease = other_quarantine.lease(StageAccess::Robustness).unwrap();
    assert!(observe(&request, &result, &report, &other_lease, &["total_return"]).is_err());
    let narrower = HoldoutQuarantine::new("a".repeat(64), "f".repeat(64), 100, 40).unwrap();
    let narrower_lease = narrower.lease(StageAccess::Robustness).unwrap();
    assert!(
        observe(
            &request,
            &result,
            &report,
            &narrower_lease,
            &["total_return"]
        )
        .is_err()
    );

    // A tampered report cannot even be materialized, so it can never reach an observation.
    let bytes = report.to_json_vec().unwrap();
    let mut tampered: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    tampered["analysis"]["metrics"][0]["value"]["Defined"]["value"] = 4.0.into();
    assert!(
        StrategyReportArtifact::from_json_slice(&serde_json::to_vec(&tampered).unwrap()).is_err()
    );

    // Degenerate metrics stay typed. `profit_factor` has no losing trade here, so it is
    // `Undefined` rather than a sentinel, and an undefined value is not an observation.
    // A non-finite value is unrepresentable upstream: `MetricValue::defined` converts it to
    // `Undefined { ArithmeticOverflow }` at construction.
    assert!(observe(&request, &result, &report, &lease, &["profit_factor"]).is_err());
    assert!(observe(&request, &result, &report, &lease, &["no_such_metric"]).is_err());
    assert!(observe(&request, &result, &report, &lease, &[]).is_err());
    assert!(observe(&request, &result, &report, &lease, &["total_return"; 2]).is_err());
    assert!(observe(&request, &result, &report, &lease, &[" "]).is_err());
}

#[test]
fn pipeline_declarations_reject_duplicate_misordered_and_out_of_bounds_stages() {
    let stage = |order: u16, name: &str| {
        RobustnessStageSpec::metric_percentile(
            order,
            name,
            ObservationRole::OutOfSample,
            "total_return",
            Percentile::Median,
            Threshold::AtLeast(0.0),
        )
    };
    assert!(RobustnessPipeline::new(vec![]).is_err());
    assert!(RobustnessPipeline::new(vec![stage(1, "a"), stage(1, "b")]).is_err());
    assert!(RobustnessPipeline::new(vec![stage(1, "a"), stage(2, "a")]).is_err());
    assert!(RobustnessPipeline::new(vec![stage(1, "  ")]).is_err());
    assert!(
        RobustnessPipeline::new(vec![RobustnessStageSpec::metric_percentile(
            1,
            "blank-metric",
            ObservationRole::OutOfSample,
            "",
            Percentile::Median,
            Threshold::AtLeast(0.0),
        )])
        .is_err()
    );
    assert!(
        RobustnessPipeline::new(vec![RobustnessStageSpec::metric_percentile(
            1,
            "non-finite-threshold",
            ObservationRole::OutOfSample,
            "total_return",
            Percentile::Median,
            Threshold::AtLeast(f64::NAN),
        )])
        .is_err()
    );
    assert!(
        RobustnessPipeline::new(vec![RobustnessStageSpec::degradation_ratio(
            1,
            "impossible-ratio",
            "total_return",
            Percentile::Median,
            10_001,
        )])
        .is_err()
    );

    let bounded: Vec<_> = (0..MAX_ROBUSTNESS_STAGES)
        .map(|index| stage(index as u16, &format!("stage-{index}")))
        .collect();
    assert!(RobustnessPipeline::new(bounded.clone()).is_ok());
    let mut over_cap = bounded.clone();
    over_cap.push(stage(u16::MAX, "one-too-many"));
    assert!(RobustnessPipeline::new(over_cap).is_err());

    // Declaration order is irrelevant: the declared `order` is the execution order.
    let mut shuffled = bounded.clone();
    shuffled.reverse();
    assert_eq!(
        RobustnessPipeline::new(bounded).unwrap(),
        RobustnessPipeline::new(shuffled).unwrap()
    );
}

#[test]
fn pipeline_execution_fails_closed_on_foreign_candidate_lease_duplicate_and_bounds() {
    let quarantine = HoldoutQuarantine::new("a".repeat(64), "f".repeat(64), 100, 20).unwrap();
    let lease = quarantine.lease(StageAccess::Robustness).unwrap();
    let candidate = base_strategy();
    let pipeline = RobustnessPipeline::new(vec![RobustnessStageSpec::metric_percentile(
        1,
        "oos-floor",
        ObservationRole::OutOfSample,
        "total_return",
        Percentile::P05,
        Threshold::AtLeast(0.0),
    )])
    .unwrap();
    let observations = || {
        vec![sealed_observation(
            &lease,
            ObservationRole::OutOfSample,
            &candidate,
            1_080.0,
        )]
    };
    let candidate_id = candidate.strategy_id();
    assert!(
        pipeline
            .execute(&lease, candidate_id, 4, observations())
            .is_ok()
    );

    assert!(pipeline.execute(&lease, "", 4, observations()).is_err());
    assert!(
        pipeline
            .execute(&lease, "some-other-candidate", 4, observations())
            .is_err()
    );
    assert!(
        pipeline
            .execute(&lease, candidate_id, 0, observations())
            .is_err()
    );
    assert!(
        pipeline
            .execute(&lease, candidate_id, MAX_TRIAL_BUDGET + 1, observations())
            .is_err()
    );
    assert!(pipeline.execute(&lease, candidate_id, 4, vec![]).is_err());

    // The same sealed report counted twice would double its own evidence.
    let mut duplicated = observations();
    duplicated.push(duplicated[0].clone());
    assert!(
        pipeline
            .execute(&lease, candidate_id, 4, duplicated)
            .is_err()
    );

    // Evidence produced on another partition cannot be laundered through this lease.
    let other_quarantine = HoldoutQuarantine::new("b".repeat(64), "f".repeat(64), 100, 20).unwrap();
    let other_lease = other_quarantine.lease(StageAccess::Robustness).unwrap();
    let foreign = vec![sealed_observation(
        &other_lease,
        ObservationRole::OutOfSample,
        &candidate,
        1_080.0,
    )];
    assert!(pipeline.execute(&lease, candidate_id, 4, foreign).is_err());

    // A degradation stage with no in-sample baseline, or a zero baseline, refuses to rule
    // rather than silently reporting a verdict it has no evidence for.
    let degradation = RobustnessPipeline::new(vec![RobustnessStageSpec::degradation_ratio(
        1,
        "oos-vs-is",
        "total_return",
        Percentile::Median,
        5_000,
    )])
    .unwrap();
    assert!(
        degradation
            .execute(&lease, candidate_id, 4, observations())
            .is_err()
    );
    assert!(
        degradation
            .execute(
                &lease,
                candidate_id,
                4,
                vec![
                    sealed_observation(&lease, ObservationRole::InSample, &candidate, 1_000.0),
                    sealed_observation(&lease, ObservationRole::OutOfSample, &candidate, 1_080.0),
                ],
            )
            .is_err()
    );
}

#[test]
fn published_distributions_cover_the_upper_tail_of_small_samples() {
    let quarantine = HoldoutQuarantine::new("a".repeat(64), "f".repeat(64), 100, 20).unwrap();
    let lease = quarantine.lease(StageAccess::Robustness).unwrap();
    let candidate = base_strategy();
    let pipeline = RobustnessPipeline::new(vec![RobustnessStageSpec::metric_percentile(
        1,
        "oos-band",
        ObservationRole::OutOfSample,
        "total_return",
        Percentile::P95,
        Threshold::AtLeast(0.09),
    )])
    .unwrap();
    let outcome = pipeline
        .execute(
            &lease,
            candidate.strategy_id(),
            9,
            vec![
                sealed_observation(&lease, ObservationRole::OutOfSample, &candidate, 1_050.0),
                sealed_observation(&lease, ObservationRole::OutOfSample, &candidate, 1_090.0),
                sealed_observation(&lease, ObservationRole::OutOfSample, &candidate, 1_020.0),
            ],
        )
        .unwrap();
    assert_eq!(outcome.artifact().verdict(), StageVerdict::Pass);
    let distribution = &outcome.distributions()[0];
    assert_eq!(distribution.observations_n, 3);
    assert_eq!(distribution.sorted_samples, vec![0.02, 0.05, 0.09]);
    assert_eq!((distribution.p05, distribution.median), (0.02, 0.05));
    assert_eq!(distribution.p95, 0.09);

    // Two samples must still resolve an upper bound above the lower one; a truncating index
    // would report the worst observation as the 95th percentile.
    let pair = pipeline
        .execute(
            &lease,
            candidate.strategy_id(),
            9,
            vec![
                sealed_observation(&lease, ObservationRole::OutOfSample, &candidate, 1_090.0),
                sealed_observation(&lease, ObservationRole::OutOfSample, &candidate, 1_020.0),
            ],
        )
        .unwrap();
    let distribution = &pair.distributions()[0];
    assert_eq!((distribution.p05, distribution.p95), (0.02, 0.09));
}

#[test]
fn objective_selection_refuses_non_search_roles_unknown_metrics_and_duplicate_candidates() {
    let quarantine = HoldoutQuarantine::new("a".repeat(64), "f".repeat(64), 100, 20).unwrap();
    let lease = quarantine.lease(StageAccess::Search).unwrap();
    let candidates = generate_candidates(&space(), SearchMethod::Grid, 3)
        .unwrap()
        .candidates;
    let observations: Vec<_> = candidates
        .iter()
        .zip([1_050.0, 1_100.0, 1_075.0])
        .map(|(candidate, equity)| {
            sealed_observation(
                &lease,
                ObservationRole::SearchEvaluation,
                &candidate.strategy,
                equity,
            )
        })
        .collect();
    let maximize = ObjectiveSpec::new("total_return", ObjectiveDirection::Maximize).unwrap();
    let minimize = ObjectiveSpec::new("total_return", ObjectiveDirection::Minimize).unwrap();

    let worst = select_best(&observations, &minimize).unwrap();
    assert_eq!(worst.candidate_id(), candidates[0].candidate_id);
    assert!(worst.label().contains("best of N=3"));

    assert!(ObjectiveSpec::new("  ", ObjectiveDirection::Maximize).is_err());
    assert!(
        select_best(
            &observations,
            &ObjectiveSpec::new("no_such_metric", ObjectiveDirection::Maximize).unwrap()
        )
        .is_err()
    );
    assert!(pareto_front(&observations, &[]).is_err());
    assert!(select_best(&[], &maximize).is_err());

    // Robustness evidence is not a search evaluation and must not silently become a "best".
    let mut mixed = observations.clone();
    mixed.push(sealed_observation(
        &lease,
        ObservationRole::OutOfSample,
        &candidates[0].strategy,
        1_200.0,
    ));
    assert!(select_best(&mixed, &maximize).is_err());

    // One candidate evaluated twice would appear twice in the selection universe.
    let mut duplicated = observations.clone();
    duplicated.push(observations[0].clone());
    assert!(select_best(&duplicated, &maximize).is_err());
}
