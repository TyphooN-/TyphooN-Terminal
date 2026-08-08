use super::*;

use crate::core::strategy_optimization::{DataRegion, OptimizationError};

fn store() -> (tempfile::TempDir, FileDatasetStore) {
    let root = tempfile::tempdir().unwrap();
    let store = FileDatasetStore::open(root.path()).unwrap();
    (root, store)
}

fn run(process: SyntheticProcess) -> (tempfile::TempDir, FileDatasetStore, AcceptanceOutcome) {
    let (root, datasets) = store();
    let outcome = execute_m4_acceptance(&datasets, &AcceptanceCorpusSpec::gate(process)).unwrap();
    (root, datasets, outcome)
}

#[test]
fn literal_pipeline_rejects_known_random_curve_fit_and_accepts_planted_edge() {
    let (_random_root, _random_store, random) = run(SyntheticProcess::KnownRandom);
    let (_edge_root, _edge_store, edge) = run(SyntheticProcess::PlantedEdge);

    assert!(
        !random.passed(),
        "known-random curve fit unexpectedly survived"
    );
    assert!(
        random.failed_gates().contains(&"adjusted-significance")
            && random.failed_gates().contains(&"oos-degradation")
            && random.failed_gates().contains(&"cost-degradation"),
        "known-random rejection was not backed by independent pipeline gates: {:?}",
        random.failed_gates()
    );
    assert!(
        edge.passed(),
        "planted edge failed: {:?}",
        edge.failed_gates()
    );

    for outcome in [&random, &edge] {
        assert_eq!(outcome.evaluations_n(), ACCEPTANCE_EVALUATIONS_N);
        assert_eq!(outcome.field().evaluations_n(), ACCEPTANCE_EVALUATIONS_N);
        assert_eq!(
            outcome.cross_check().evaluations_n(),
            ACCEPTANCE_EVALUATIONS_N
        );
        assert!(outcome.best_label().contains("N=9"));
        assert_ne!(outcome.search_dataset_id(), outcome.holdout_dataset_id());
        outcome.field().verify().unwrap();
        outcome.significance().verify().unwrap();
        outcome.cross_check().verify().unwrap();
        outcome.oos().verify().unwrap();
        outcome.problem_recognition().verify().unwrap();
    }
}

#[test]
fn holdout_is_unreachable_from_search_and_robustness() {
    let (_root, datasets) = store();
    let split = publish_split(
        &datasets,
        &SyntheticSeriesSpec {
            process: SyntheticProcess::PlantedEdge,
            symbol: "SYN-A/USD".into(),
            timeframe: "1Day".into(),
            source: "synthetic-primary".into(),
            seed: 7,
            bars: ACCEPTANCE_PARENT_BARS,
        },
        ACCEPTANCE_HOLDOUT_BARS,
    )
    .unwrap();
    let quarantine = split.quarantine();
    assert!(matches!(
        quarantine.lease(StageAccess::FinalReview),
        Err(OptimizationError::HoldoutForbidden)
    ));
    assert!(matches!(
        quarantine.range_for(StageAccess::Search, DataRegion::FinalHoldout),
        Err(OptimizationError::HoldoutForbidden)
    ));
    assert!(matches!(
        quarantine.range_for(StageAccess::Robustness, DataRegion::FinalHoldout),
        Err(OptimizationError::HoldoutForbidden)
    ));
    assert_eq!(
        split.search_bars().len(),
        ACCEPTANCE_PARENT_BARS - ACCEPTANCE_HOLDOUT_BARS
    );
}

#[test]
fn a_field_study_seals_over_the_whole_acceptance_search_partition() {
    // ADR-135 §7.4 is only worth gating on if it can be executed over a corpus large enough to
    // close a statistically meaningful number of trades. A field artifact that embedded one whole
    // *uncompressed* report per executed point reached `MAX_ARTIFACT_BYTES` at roughly seventy
    // bars — long before the trade count became meaningful — so this pins the partition the study
    // must actually be able to seal, round-trip and replay.
    let (_root, datasets) = store();
    let split = publish_split(
        &datasets,
        &SyntheticSeriesSpec {
            process: SyntheticProcess::PlantedEdge,
            symbol: "SYN-A/USD".into(),
            timeframe: "1Day".into(),
            source: "synthetic-primary".into(),
            seed: 11,
            bars: ACCEPTANCE_PARENT_BARS,
        },
        ACCEPTANCE_HOLDOUT_BARS,
    )
    .unwrap();
    let field = execute_parameter_field_study(
        &acceptance_execution_config().unwrap(),
        split.search_manifest(),
        split.search_bars(),
        split.quarantine().lease(StageAccess::Robustness).unwrap(),
        &acceptance_search_space().unwrap(),
        ParameterFieldStudySpec {
            field_sample_size: ACCEPTANCE_EVALUATIONS_N,
            neighbour_radius: 1,
            plateau_tolerance_bps: 1_500,
            minimum_plateau_neighbours: 3,
            metric_id: ACCEPTANCE_METRIC.into(),
            direction: ObjectiveDirection::Maximize,
            root_seed: 13,
        },
    )
    .unwrap();
    assert_eq!(field.points().len(), ACCEPTANCE_EVALUATIONS_N);
    assert_eq!(
        field.range().len(),
        ACCEPTANCE_PARENT_BARS - ACCEPTANCE_HOLDOUT_BARS
    );
    let bytes = field.to_json_vec().unwrap();
    assert_eq!(
        crate::core::strategy_parameter_field::ParameterFieldStudyArtifact::from_json_slice(&bytes)
            .unwrap(),
        field
    );
    let replayed = crate::core::strategy_parameter_field::replay_parameter_field_study(
        &acceptance_execution_config().unwrap(),
        split.search_manifest(),
        split.search_bars(),
        split.quarantine().lease(StageAccess::Robustness).unwrap(),
        &acceptance_search_space().unwrap(),
        &field,
    )
    .unwrap();
    assert_eq!(replayed, field);
}
