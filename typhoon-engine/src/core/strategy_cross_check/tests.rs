use super::*;
use crate::broker::alpaca::Bar;
use crate::core::strategy_builder::GeneralStrategyBuilder;
use crate::core::strategy_dataset::{
    AdjustmentPolicy, CalendarPolicy, DatasetManifestInput, DatasetProvenance, DatasetQaPolicy,
};
use crate::core::strategy_ir::{
    CommissionModel, ExecutionSettings, SlippageModel, SpreadModel, StrategyExecutionConfig,
};
use crate::core::strategy_optimization::HoldoutQuarantine;
use crate::core::strategy_retest::{
    RetestEvidenceStore, StudyArtifact, StudyArtifactKind, StudyArtifactQuery,
};

fn bars() -> Vec<Bar> {
    (0..12)
        .map(|index| {
            let open = 100.0 + index as f64;
            Bar {
                timestamp: format!("2026-02-{:02}T00:00:00Z", index + 1),
                open,
                high: open + 2.0,
                low: open - 1.0,
                close: open + 1.0,
                volume: 100.0,
            }
        })
        .collect()
}

fn manifest(bars: &[Bar], symbol: &str, timeframe: &str, source: &str) -> DatasetManifest {
    DatasetManifest::build(
        &DatasetManifestInput {
            symbol: symbol.into(),
            timeframe: timeframe.into(),
            provenance: DatasetProvenance {
                source: source.into(),
                venue: "fixture".into(),
                pipeline: format!("cross-check-{source}/v1"),
            },
            adjustment: AdjustmentPolicy::Raw,
            calendar: CalendarPolicy::Continuous24x7,
            qa_policy: DatasetQaPolicy::default(),
        },
        bars,
    )
    .unwrap()
}

fn config() -> StrategyExecutionConfig {
    let mut settings = ExecutionSettings::conservative_defaults();
    settings.spread = SpreadModel::Constant { price_units: 0.02 };
    settings.slippage = SlippageModel::FixedPriceDistance { distance: 0.01 };
    settings.commission = CommissionModel::PerOrder { amount: 0.25 };
    StrategyExecutionConfig::build(&settings).unwrap()
}

fn lease(manifest: &DatasetManifest, len: usize) -> SearchDataLease {
    HoldoutQuarantine::new(&manifest.dataset_id, "f".repeat(64), len + 4, 4)
        .unwrap()
        .lease(StageAccess::Robustness)
        .unwrap()
}

fn spec() -> CrossCheckStudySpec {
    CrossCheckStudySpec {
        metric_id: "net_profit".into(),
        direction: ObjectiveDirection::Maximize,
        minimum_retention_bps: 8_000,
        evaluations_n: 41,
        root_seed: 77,
    }
}

struct Fixture {
    strategy: StrategyIr,
    config: StrategyExecutionConfig,
    bars: Vec<Bar>,
    baseline: DatasetManifest,
    symbol: DatasetManifest,
    timeframe: DatasetManifest,
    source: DatasetManifest,
}
impl Fixture {
    fn new() -> Self {
        let bars = bars();
        Self {
            strategy: StrategyIr::build(
                GeneralStrategyBuilder::new("cross-check", "test").definition(),
            )
            .unwrap(),
            config: config(),
            baseline: manifest(&bars, "BTC/USD", "1Day", "primary"),
            symbol: manifest(&bars, "ETH/USD", "1Day", "primary"),
            timeframe: manifest(&bars, "BTC/USD", "4Hour", "primary"),
            source: manifest(&bars, "BTC/USD", "1Day", "alternate"),
            bars,
        }
    }
    fn cases(&self) -> Vec<CrossCheckDatasetCase<'_>> {
        vec![
            CrossCheckDatasetCase {
                kind: CrossCheckKind::OtherSymbol,
                label: "eth-usd".into(),
                config: &self.config,
                dataset: &self.symbol,
                bars: &self.bars,
                lease: lease(&self.symbol, self.bars.len()),
            },
            CrossCheckDatasetCase {
                kind: CrossCheckKind::AdjacentTimeframe,
                label: "btc-4h".into(),
                config: &self.config,
                dataset: &self.timeframe,
                bars: &self.bars,
                lease: lease(&self.timeframe, self.bars.len()),
            },
            CrossCheckDatasetCase {
                kind: CrossCheckKind::AlternativeSource,
                label: "btc-alternate".into(),
                config: &self.config,
                dataset: &self.source,
                bars: &self.bars,
                lease: lease(&self.source, self.bars.len()),
            },
        ]
    }
    fn execute(
        &self,
        study_spec: CrossCheckStudySpec,
    ) -> Result<CrossCheckStudyArtifact, RetestError> {
        execute_cross_check_study(
            &self.strategy,
            &self.config,
            &self.baseline,
            &self.bars,
            lease(&self.baseline, self.bars.len()),
            self.cases(),
            study_spec,
        )
    }
}

#[test]
fn cross_check_executes_every_required_family_replays_and_persists() {
    let fixture = Fixture::new();
    let artifact = fixture.execute(spec()).unwrap();
    artifact.verify().unwrap();
    assert_eq!(artifact.artifact_id().len(), 64);
    assert_eq!(artifact.strategy_id(), fixture.strategy.strategy_id());
    assert_eq!(artifact.source_dataset_id(), fixture.baseline.dataset_id);
    assert_eq!(artifact.metric_id(), "net_profit");
    assert_eq!(artifact.evaluations_n(), 41);
    assert_eq!(artifact.baseline().kind, CrossCheckKind::Baseline);
    assert_eq!(artifact.baseline().retention_bps, 10_000);
    assert_eq!(artifact.checks().len(), 5);
    assert!(artifact.passed());
    assert!(artifact.verdict_reason().contains("N=5"));
    let kinds = artifact
        .checks()
        .iter()
        .map(|check| check.kind)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        kinds,
        BTreeSet::from([
            CrossCheckKind::OtherSymbol,
            CrossCheckKind::AdjacentTimeframe,
            CrossCheckKind::AlternativeSource,
            CrossCheckKind::CostSensitivity {
                multiplier_bps: COST_MULTIPLIER_2X_BPS,
            },
            CrossCheckKind::CostSensitivity {
                multiplier_bps: COST_MULTIPLIER_3X_BPS,
            },
        ])
    );
    assert!(artifact.checks().iter().enumerate().all(|(index, check)| {
        check.ordinal == index + 1
            && check.run_id.len() == 64
            && check.report_id.len() == 64
            && check.request_id.len() == 64
            && check.value.is_finite()
            && check.retention_bps <= 20_000
    }));
    let cost_configs = artifact
        .checks()
        .iter()
        .filter(|check| matches!(check.kind, CrossCheckKind::CostSensitivity { .. }))
        .map(|check| check.config_id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(cost_configs.len(), 2);
    assert!(!cost_configs.contains(fixture.config.config_id()));

    assert_eq!(
        replay_cross_check_study(
            &fixture.strategy,
            &fixture.config,
            &fixture.baseline,
            &fixture.bars,
            lease(&fixture.baseline, fixture.bars.len()),
            fixture.cases(),
            &artifact,
        )
        .unwrap(),
        artifact
    );

    let store = RetestEvidenceStore::open_in_memory().unwrap();
    store.persist_cross_check_study(&artifact, 5).unwrap();
    assert!(store.persist_cross_check_study(&artifact, 6).is_err());
    let page = store
        .query_studies(&StudyArtifactQuery {
            source_dataset_id: fixture.baseline.dataset_id.clone(),
            kind: Some(StudyArtifactKind::CrossCheck),
            after_sequence: None,
            limit: 2,
        })
        .unwrap();
    assert!(matches!(
        &page.records[0].artifact,
        StudyArtifact::CrossCheck(value) if value == &artifact
    ));
}

#[test]
fn cross_check_fails_closed_on_scope_bounds_mislabeling_tampering_and_foreign_replay() {
    let fixture = Fixture::new();
    let artifact = fixture.execute(spec()).unwrap();

    let mut invalid_spec = spec();
    invalid_spec.minimum_retention_bps = 10_001;
    assert!(fixture.execute(invalid_spec).is_err());
    let mut invalid_spec = spec();
    invalid_spec.evaluations_n = 0;
    assert!(fixture.execute(invalid_spec).is_err());

    let mut missing = fixture.cases();
    missing.pop();
    assert!(
        execute_cross_check_study(
            &fixture.strategy,
            &fixture.config,
            &fixture.baseline,
            &fixture.bars,
            lease(&fixture.baseline, fixture.bars.len()),
            missing,
            spec(),
        )
        .is_err()
    );
    let mislabeled = vec![
        CrossCheckDatasetCase {
            kind: CrossCheckKind::OtherSymbol,
            label: "wrong".into(),
            config: &fixture.config,
            dataset: &fixture.timeframe,
            bars: &fixture.bars,
            lease: lease(&fixture.timeframe, fixture.bars.len()),
        },
        CrossCheckDatasetCase {
            kind: CrossCheckKind::AdjacentTimeframe,
            label: "timeframe".into(),
            config: &fixture.config,
            dataset: &fixture.timeframe,
            bars: &fixture.bars,
            lease: lease(&fixture.timeframe, fixture.bars.len()),
        },
        CrossCheckDatasetCase {
            kind: CrossCheckKind::AlternativeSource,
            label: "source".into(),
            config: &fixture.config,
            dataset: &fixture.source,
            bars: &fixture.bars,
            lease: lease(&fixture.source, fixture.bars.len()),
        },
    ];
    assert!(
        execute_cross_check_study(
            &fixture.strategy,
            &fixture.config,
            &fixture.baseline,
            &fixture.bars,
            lease(&fixture.baseline, fixture.bars.len()),
            mislabeled,
            spec(),
        )
        .is_err()
    );
    let search_lease = HoldoutQuarantine::new(
        &fixture.baseline.dataset_id,
        "f".repeat(64),
        fixture.bars.len() + 4,
        4,
    )
    .unwrap()
    .lease(StageAccess::Search)
    .unwrap();
    assert!(
        execute_cross_check_study(
            &fixture.strategy,
            &fixture.config,
            &fixture.baseline,
            &fixture.bars,
            search_lease,
            fixture.cases(),
            spec(),
        )
        .is_err()
    );
    let no_cost =
        StrategyExecutionConfig::build(&ExecutionSettings::conservative_defaults()).unwrap();
    assert!(
        execute_cross_check_study(
            &fixture.strategy,
            &no_cost,
            &fixture.baseline,
            &fixture.bars,
            lease(&fixture.baseline, fixture.bars.len()),
            fixture.cases(),
            spec(),
        )
        .is_err()
    );

    let bytes = artifact.to_json_vec().unwrap();
    let tamper = |pointer: &str, replacement: serde_json::Value, reseal: bool| {
        let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        *value.pointer_mut(pointer).unwrap() = replacement;
        let altered = serde_json::to_vec(&value).unwrap();
        if reseal {
            CrossCheckStudyArtifact::resealed_from_json(&altered).is_err()
        } else {
            CrossCheckStudyArtifact::from_json_slice(&altered).is_err()
        }
    };
    assert!(tamper("/checks/0/value", serde_json::json!(12345.0), true));
    assert!(tamper(
        "/checks/0/retention_bps",
        serde_json::json!(1),
        true
    ));
    assert!(tamper("/checks/0/ordinal", serde_json::json!(9), true));
    assert!(tamper(
        "/checks/3/kind/cost_sensitivity/multiplier_bps",
        serde_json::json!(25_000),
        true
    ));
    assert!(tamper("/worst_retention_bps", serde_json::json!(1), true));
    assert!(tamper(
        "/passed",
        serde_json::json!(!artifact.passed()),
        true
    ));
    assert!(tamper("/spec/evaluations_n", serde_json::json!(42), false));

    let mut foreign_bars = fixture.bars.clone();
    foreign_bars[3].close += 0.25;
    assert!(
        replay_cross_check_study(
            &fixture.strategy,
            &fixture.config,
            &fixture.baseline,
            &foreign_bars,
            lease(&fixture.baseline, fixture.bars.len()),
            fixture.cases(),
            &artifact,
        )
        .is_err()
    );
}

#[test]
fn direction_aware_retention_handles_positive_negative_and_zero_baselines() {
    assert_eq!(
        retention_bps(100.0, 80.0, ObjectiveDirection::Maximize).unwrap(),
        8_000
    );
    assert_eq!(
        retention_bps(100.0, 125.0, ObjectiveDirection::Minimize).unwrap(),
        8_000
    );
    assert_eq!(
        retention_bps(-10.0, -12.0, ObjectiveDirection::Maximize).unwrap(),
        8_000
    );
    assert_eq!(
        retention_bps(-10.0, -8.0, ObjectiveDirection::Minimize).unwrap(),
        8_000
    );
    assert_eq!(
        retention_bps(0.0, 0.0, ObjectiveDirection::Maximize).unwrap(),
        10_000
    );

    let baseline = config();
    let doubled = stressed_config(&baseline, 10_000).unwrap().to_input();
    assert_eq!(
        doubled.commission,
        CommissionModel::PerOrder { amount: 0.5 }
    );
    assert_eq!(doubled.spread, SpreadModel::Constant { price_units: 0.04 });
    assert_eq!(
        doubled.slippage,
        SlippageModel::FixedPriceDistance { distance: 0.02 }
    );
}
