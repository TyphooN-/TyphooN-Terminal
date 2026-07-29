use super::*;
use crate::core::strategy_ir::{ParamRange, ParamValue, StrategyIr, StrategyParameter};

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
fn search_api_refuses_holdout_and_holdout_is_one_way_burned() {
    let quarantine = HoldoutQuarantine::new(100, 20).unwrap();
    assert_eq!(quarantine.search_range().unwrap(), 0..80);
    assert!(matches!(
        quarantine.range_for(StageAccess::Search, DataRegion::FinalHoldout),
        Err(OptimizationError::HoldoutForbidden)
    ));
    let burned = quarantine.consume_holdout("final-review").unwrap();
    assert_eq!(burned.range, 80..100);
    assert!(matches!(
        quarantine.search_range(),
        Err(OptimizationError::HoldoutAlreadyConsumed)
    ));
    assert!(matches!(
        quarantine.consume_holdout("again"),
        Err(OptimizationError::HoldoutAlreadyConsumed)
    ));
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
