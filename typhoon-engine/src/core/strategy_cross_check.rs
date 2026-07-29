//! Canonical cross-market, cross-timeframe, cross-source, and cost-sensitivity retests (§7.5).
//!
//! Every observation is a fresh exact-lease `VerifiedRun`; labels and scores are never accepted as
//! evidence. The artifact seals the report, dataset/config identities, deterministic component seed,
//! objective direction, selection-universe N, and the documented retention formula.

use crate::broker::alpaca::Bar;
use crate::core::strategy_dataset::DatasetManifest;
use crate::core::strategy_ir::{StrategyExecutionConfig, StrategyIr};
use crate::core::strategy_metrics::{METRICS_SCHEMA_VERSION, MetricValue};
use crate::core::strategy_optimization::{
    MAX_ARTIFACT_BYTES, MAX_TRIAL_BUDGET, ObjectiveDirection, ObservationRole, RetestRequest,
    SearchDataLease, StageAccess,
};
use crate::core::strategy_perturbation::{has_scalable_cost, stressed_config};
use crate::core::strategy_report::StrategyReportArtifact;
use crate::core::strategy_retest::{
    RetestError, RetestExecutionRequest, execute_bound_observation, execution_request_id,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::ops::Range;

pub const CROSS_CHECK_STUDY_SCHEMA_VERSION: u32 = 1;
pub const MAX_CROSS_CHECK_DATASET_CASES: usize = 30;
pub const COST_MULTIPLIER_2X_BPS: u32 = 20_000;
pub const COST_MULTIPLIER_3X_BPS: u32 = 30_000;
const ARTIFACT_DOMAIN: &[u8] = b"typhoon.strategy_cross_check.study.v1";
const COMPONENT_SEED_DOMAIN: &[u8] = b"typhoon.strategy_cross_check.component_seed.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrossCheckKind {
    Baseline,
    OtherSymbol,
    AdjacentTimeframe,
    AlternativeSource,
    CostSensitivity { multiplier_bps: u32 },
}
impl CrossCheckKind {
    fn tag(self) -> &'static [u8] {
        match self {
            Self::Baseline => b"baseline",
            Self::OtherSymbol => b"other_symbol",
            Self::AdjacentTimeframe => b"adjacent_timeframe",
            Self::AlternativeSource => b"alternative_source",
            Self::CostSensitivity {
                multiplier_bps: 20_000,
            } => b"cost_2x",
            Self::CostSensitivity {
                multiplier_bps: 30_000,
            } => b"cost_3x",
            Self::CostSensitivity { .. } => b"invalid_cost",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossCheckStudySpec {
    pub metric_id: String,
    pub direction: ObjectiveDirection,
    pub minimum_retention_bps: u32,
    pub evaluations_n: usize,
    pub root_seed: u64,
}

/// One caller-selected exact dataset case. Cost sensitivity is generated internally so its 2×/3×
/// config identity cannot be mislabeled by a caller.
pub struct CrossCheckDatasetCase<'a> {
    pub kind: CrossCheckKind,
    pub label: String,
    pub config: &'a StrategyExecutionConfig,
    pub dataset: &'a DatasetManifest,
    pub bars: &'a [Bar],
    pub lease: SearchDataLease,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SerializableSpec {
    metric_id: String,
    direction: ObjectiveDirection,
    minimum_retention_bps: u32,
    evaluations_n: usize,
    root_seed: u64,
}
impl From<CrossCheckStudySpec> for SerializableSpec {
    fn from(value: CrossCheckStudySpec) -> Self {
        Self {
            metric_id: value.metric_id,
            direction: value.direction,
            minimum_retention_bps: value.minimum_retention_bps,
            evaluations_n: value.evaluations_n,
            root_seed: value.root_seed,
        }
    }
}
impl SerializableSpec {
    fn public(&self) -> CrossCheckStudySpec {
        CrossCheckStudySpec {
            metric_id: self.metric_id.clone(),
            direction: self.direction,
            minimum_retention_bps: self.minimum_retention_bps,
            evaluations_n: self.evaluations_n,
            root_seed: self.root_seed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrossCheckObservation {
    pub ordinal: usize,
    pub kind: CrossCheckKind,
    pub label: String,
    pub symbol: String,
    pub timeframe: String,
    pub source: String,
    pub venue: String,
    pub pipeline: String,
    pub dataset_id: String,
    pub manifest_id: String,
    pub range_start: usize,
    pub range_end: usize,
    pub config_id: String,
    pub component_seed: u64,
    pub request_id: String,
    pub run_id: String,
    pub report_id: String,
    pub value: f64,
    /// Direction-aware performance retained relative to baseline. For a positive maximize baseline
    /// (or negative minimize baseline), this is the ordinary ratio. Otherwise the formula is
    /// `10_000 + signed_delta / max(abs(baseline), 1) * 10_000`, clamped to `[0, 20_000]`.
    pub retention_bps: u32,
    config_json: Vec<u8>,
    report_json: Vec<u8>,
}
impl CrossCheckObservation {
    pub fn range(&self) -> Range<usize> {
        self.range_start..self.range_end
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrossCheckStudyArtifact {
    schema_version: u32,
    artifact_id: String,
    strategy_json: Vec<u8>,
    strategy_id: String,
    spec: SerializableSpec,
    baseline: CrossCheckObservation,
    checks: Vec<CrossCheckObservation>,
    worst_retention_bps: u32,
    passed: bool,
    verdict_reason: String,
}

impl CrossCheckStudyArtifact {
    pub fn artifact_id(&self) -> &str {
        &self.artifact_id
    }
    pub fn strategy_id(&self) -> &str {
        &self.strategy_id
    }
    pub fn source_dataset_id(&self) -> &str {
        &self.baseline.dataset_id
    }
    pub fn metric_id(&self) -> &str {
        &self.spec.metric_id
    }
    pub fn evaluations_n(&self) -> usize {
        self.spec.evaluations_n
    }
    pub fn baseline(&self) -> &CrossCheckObservation {
        &self.baseline
    }
    pub fn checks(&self) -> &[CrossCheckObservation] {
        &self.checks
    }
    pub fn worst_retention_bps(&self) -> u32 {
        self.worst_retention_bps
    }
    pub fn passed(&self) -> bool {
        self.passed
    }
    pub fn verdict_reason(&self) -> &str {
        &self.verdict_reason
    }
    pub fn to_json_vec(&self) -> Result<Vec<u8>, RetestError> {
        self.verify()?;
        let bytes = serde_json::to_vec(self).map_err(invalid)?;
        if bytes.len() > MAX_ARTIFACT_BYTES {
            return Err(RetestError::Invalid(
                "cross-check artifact is too large".into(),
            ));
        }
        Ok(bytes)
    }
    pub fn from_json_slice(bytes: &[u8]) -> Result<Self, RetestError> {
        if bytes.len() > MAX_ARTIFACT_BYTES {
            return Err(RetestError::Invalid(
                "cross-check artifact is too large".into(),
            ));
        }
        let artifact: Self = serde_json::from_slice(bytes).map_err(invalid)?;
        artifact.verify()?;
        Ok(artifact)
    }
    #[cfg(test)]
    pub(crate) fn resealed_from_json(bytes: &[u8]) -> Result<Self, RetestError> {
        let mut artifact: Self = serde_json::from_slice(bytes).map_err(invalid)?;
        artifact.artifact_id.clear();
        artifact.artifact_id = artifact.compute_id()?;
        artifact.verify()?;
        Ok(artifact)
    }
    pub fn verify(&self) -> Result<(), RetestError> {
        let strategy = StrategyIr::from_json_slice(&self.strategy_json).map_err(invalid)?;
        let spec = self.spec.public();
        validate_spec(&spec)?;
        if self.schema_version != CROSS_CHECK_STUDY_SCHEMA_VERSION
            || !is_id(&self.artifact_id)
            || strategy.strategy_id() != self.strategy_id
            || self.checks.len() < 5
            || self.checks.len() > MAX_CROSS_CHECK_DATASET_CASES + 2
        {
            return Err(RetestError::Invalid(
                "invalid cross-check artifact header".into(),
            ));
        }
        let baseline_config = StrategyExecutionConfig::from_json_slice(&self.baseline.config_json)
            .map_err(invalid)?;
        verify_observation(
            &strategy,
            &baseline_config,
            &self.baseline,
            ObservationRole::InSample,
            &spec,
        )?;
        if self.baseline.kind != CrossCheckKind::Baseline
            || self.baseline.ordinal != 0
            || self.baseline.retention_bps != 10_000
        {
            return Err(RetestError::Invalid("invalid cross-check baseline".into()));
        }
        let mut labels = BTreeSet::new();
        let mut requests = BTreeSet::new();
        let mut runs = BTreeSet::new();
        let mut reports = BTreeSet::new();
        let mut families = BTreeSet::new();
        let mut cost_multipliers = BTreeSet::new();
        for (index, check) in self.checks.iter().enumerate() {
            if check.ordinal != index + 1
                || check.kind == CrossCheckKind::Baseline
                || !labels.insert(check.label.as_str())
                || !requests.insert(check.request_id.as_str())
                || !runs.insert(check.run_id.as_str())
                || !reports.insert(check.report_id.as_str())
            {
                return Err(RetestError::Invalid(
                    "duplicate or misordered cross-check evidence".into(),
                ));
            }
            let config =
                StrategyExecutionConfig::from_json_slice(&check.config_json).map_err(invalid)?;
            validate_kind(&self.baseline, check)?;
            match check.kind {
                CrossCheckKind::OtherSymbol
                | CrossCheckKind::AdjacentTimeframe
                | CrossCheckKind::AlternativeSource => {
                    families.insert(check.kind);
                }
                CrossCheckKind::CostSensitivity { multiplier_bps } => {
                    if !cost_multipliers.insert(multiplier_bps) {
                        return Err(RetestError::Invalid(
                            "duplicate cost-sensitivity multiple".into(),
                        ));
                    }
                    let expected = stressed_config(&baseline_config, multiplier_bps - 10_000)?;
                    if config != expected {
                        return Err(RetestError::Invalid(
                            "cost-sensitivity config mismatch".into(),
                        ));
                    }
                }
                CrossCheckKind::Baseline => unreachable!(),
            }
            verify_observation(
                &strategy,
                &config,
                check,
                ObservationRole::CrossCheck,
                &spec,
            )?;
            let expected_retention =
                retention_bps(self.baseline.value, check.value, spec.direction)?;
            if check.retention_bps != expected_retention {
                return Err(RetestError::Invalid(
                    "cross-check retention mismatch".into(),
                ));
            }
        }
        let required_families = BTreeSet::from([
            CrossCheckKind::OtherSymbol,
            CrossCheckKind::AdjacentTimeframe,
            CrossCheckKind::AlternativeSource,
        ]);
        if families != required_families
            || cost_multipliers != BTreeSet::from([COST_MULTIPLIER_2X_BPS, COST_MULTIPLIER_3X_BPS])
        {
            return Err(RetestError::Invalid(
                "cross-check family coverage mismatch".into(),
            ));
        }
        let worst = self
            .checks
            .iter()
            .map(|check| check.retention_bps)
            .min()
            .ok_or_else(|| RetestError::Invalid("empty cross-check evidence".into()))?;
        let passed = worst >= spec.minimum_retention_bps;
        let reason = verdict_reason(worst, spec.minimum_retention_bps, self.checks.len());
        if self.worst_retention_bps != worst
            || self.passed != passed
            || self.verdict_reason != reason
        {
            return Err(RetestError::Invalid("cross-check verdict mismatch".into()));
        }
        if self.artifact_id != self.compute_id()? {
            return Err(RetestError::Invalid("cross-check identity mismatch".into()));
        }
        Ok(())
    }
    fn compute_id(&self) -> Result<String, RetestError> {
        let mut canonical = self.clone();
        canonical.artifact_id.clear();
        let bytes = serde_json::to_vec(&canonical).map_err(invalid)?;
        let mut hasher = Sha256::new();
        hasher.update(ARTIFACT_DOMAIN);
        frame(&mut hasher, &bytes);
        Ok(hex(hasher.finalize()))
    }
}

#[allow(clippy::too_many_arguments)]
pub fn execute_cross_check_study(
    strategy: &StrategyIr,
    baseline_config: &StrategyExecutionConfig,
    baseline_dataset: &DatasetManifest,
    baseline_bars: &[Bar],
    baseline_lease: SearchDataLease,
    mut cases: Vec<CrossCheckDatasetCase<'_>>,
    spec: CrossCheckStudySpec,
) -> Result<CrossCheckStudyArtifact, RetestError> {
    strategy.verify().map_err(invalid)?;
    baseline_config.verify().map_err(invalid)?;
    baseline_dataset.verify(baseline_bars).map_err(invalid)?;
    validate_spec(&spec)?;
    validate_lease(baseline_dataset, baseline_bars, &baseline_lease)?;
    if !has_scalable_cost(baseline_config) {
        return Err(RetestError::Invalid(
            "cross-check baseline has no scalable spread, commission, or slippage".into(),
        ));
    }
    if cases.len() < 3 || cases.len() > MAX_CROSS_CHECK_DATASET_CASES {
        return Err(RetestError::Invalid(
            "invalid cross-check dataset case count".into(),
        ));
    }
    cases.sort_by(|left, right| {
        left.kind
            .cmp(&right.kind)
            .then_with(|| left.label.cmp(&right.label))
    });
    let baseline = execute_observation(
        strategy,
        baseline_config,
        baseline_dataset,
        baseline_bars,
        baseline_lease,
        CrossCheckKind::Baseline,
        "baseline".into(),
        0,
        ObservationRole::InSample,
        &spec,
        None,
    )?;
    let mut checks = Vec::with_capacity(cases.len() + 2);
    let mut labels = BTreeSet::new();
    let mut families = BTreeSet::new();
    for case in cases {
        if case.kind == CrossCheckKind::Baseline
            || matches!(case.kind, CrossCheckKind::CostSensitivity { .. })
            || case.label.trim().is_empty()
            || !labels.insert(case.label.clone())
        {
            return Err(RetestError::Invalid(
                "invalid cross-check dataset case".into(),
            ));
        }
        validate_lease(case.dataset, case.bars, &case.lease)?;
        let ordinal = checks.len() + 1;
        let observation = execute_observation(
            strategy,
            case.config,
            case.dataset,
            case.bars,
            case.lease,
            case.kind,
            case.label,
            ordinal,
            ObservationRole::CrossCheck,
            &spec,
            Some(baseline.value),
        )?;
        validate_kind(&baseline, &observation)?;
        families.insert(observation.kind);
        checks.push(observation);
    }
    let required_families = BTreeSet::from([
        CrossCheckKind::OtherSymbol,
        CrossCheckKind::AdjacentTimeframe,
        CrossCheckKind::AlternativeSource,
    ]);
    if families != required_families {
        return Err(RetestError::Invalid(
            "cross-check requires symbol, timeframe, and source cases".into(),
        ));
    }
    for multiplier_bps in [COST_MULTIPLIER_2X_BPS, COST_MULTIPLIER_3X_BPS] {
        let config = stressed_config(baseline_config, multiplier_bps - 10_000)?;
        let lease = SearchDataLease::exact_partition(
            StageAccess::Robustness,
            baseline_dataset.dataset_id.clone(),
            baseline_lease_range(&baseline),
        )?;
        checks.push(execute_observation(
            strategy,
            &config,
            baseline_dataset,
            baseline_bars,
            lease,
            CrossCheckKind::CostSensitivity { multiplier_bps },
            format!("cost-{multiplier_bps}bps"),
            checks.len() + 1,
            ObservationRole::CrossCheck,
            &spec,
            Some(baseline.value),
        )?);
    }
    let worst_retention_bps = checks
        .iter()
        .map(|check| check.retention_bps)
        .min()
        .ok_or_else(|| RetestError::Invalid("empty cross-check evidence".into()))?;
    let passed = worst_retention_bps >= spec.minimum_retention_bps;
    let verdict_reason = verdict_reason(
        worst_retention_bps,
        spec.minimum_retention_bps,
        checks.len(),
    );
    let mut artifact = CrossCheckStudyArtifact {
        schema_version: CROSS_CHECK_STUDY_SCHEMA_VERSION,
        artifact_id: String::new(),
        strategy_json: serde_json::to_vec(strategy).map_err(invalid)?,
        strategy_id: strategy.strategy_id().to_string(),
        spec: spec.into(),
        baseline,
        checks,
        worst_retention_bps,
        passed,
        verdict_reason,
    };
    artifact.artifact_id = artifact.compute_id()?;
    artifact.verify()?;
    let _ = artifact.to_json_vec()?;
    Ok(artifact)
}

#[allow(clippy::too_many_arguments)]
pub fn replay_cross_check_study(
    strategy: &StrategyIr,
    baseline_config: &StrategyExecutionConfig,
    baseline_dataset: &DatasetManifest,
    baseline_bars: &[Bar],
    baseline_lease: SearchDataLease,
    cases: Vec<CrossCheckDatasetCase<'_>>,
    expected: &CrossCheckStudyArtifact,
) -> Result<CrossCheckStudyArtifact, RetestError> {
    expected.verify()?;
    let replay = execute_cross_check_study(
        strategy,
        baseline_config,
        baseline_dataset,
        baseline_bars,
        baseline_lease,
        cases,
        expected.spec.public(),
    )?;
    if &replay != expected {
        return Err(RetestError::Invalid("cross-check replay mismatch".into()));
    }
    Ok(replay)
}

#[allow(clippy::too_many_arguments)]
fn execute_observation(
    strategy: &StrategyIr,
    config: &StrategyExecutionConfig,
    dataset: &DatasetManifest,
    bars: &[Bar],
    lease: SearchDataLease,
    kind: CrossCheckKind,
    label: String,
    ordinal: usize,
    role: ObservationRole,
    spec: &CrossCheckStudySpec,
    baseline_value: Option<f64>,
) -> Result<CrossCheckObservation, RetestError> {
    config.verify().map_err(invalid)?;
    dataset.verify(bars).map_err(invalid)?;
    validate_lease(dataset, bars, &lease)?;
    let range = lease.range();
    let seed = component_seed(
        spec.root_seed,
        ordinal,
        kind,
        &label,
        &dataset.dataset_id,
        config.config_id(),
    );
    let execution = RetestExecutionRequest::seal(
        strategy,
        config,
        dataset,
        bars,
        lease,
        role,
        &spec.metric_id,
        seed,
    )?;
    let request_id = execution.request_id().to_string();
    let (report, observation, value) = execute_bound_observation(&execution)?;
    if observation.candidate_id() != strategy.strategy_id()
        || observation.report_id() != report.report_id()
        || !value.is_finite()
    {
        return Err(RetestError::Invalid(
            "canonical cross-check observation mismatch".into(),
        ));
    }
    let retention = match baseline_value {
        None => 10_000,
        Some(baseline) => retention_bps(baseline, value, spec.direction)?,
    };
    Ok(CrossCheckObservation {
        ordinal,
        kind,
        label,
        symbol: dataset.symbol.clone(),
        timeframe: dataset.timeframe.clone(),
        source: dataset.provenance.source.clone(),
        venue: dataset.provenance.venue.clone(),
        pipeline: dataset.provenance.pipeline.clone(),
        dataset_id: dataset.dataset_id.clone(),
        manifest_id: dataset.manifest_id.clone(),
        range_start: range.start,
        range_end: range.end,
        config_id: config.config_id().to_string(),
        component_seed: seed,
        request_id,
        run_id: report.run_id().to_string(),
        report_id: report.report_id().to_string(),
        value: canonical_zero(value),
        retention_bps: retention,
        config_json: serde_json::to_vec(config).map_err(invalid)?,
        report_json: report.to_json_vec().map_err(invalid)?,
    })
}

fn validate_spec(spec: &CrossCheckStudySpec) -> Result<(), RetestError> {
    if spec.metric_id.trim().is_empty()
        || spec.minimum_retention_bps > 10_000
        || spec.evaluations_n == 0
        || spec.evaluations_n > MAX_TRIAL_BUDGET
    {
        return Err(RetestError::Invalid(
            "invalid cross-check study specification".into(),
        ));
    }
    Ok(())
}

fn validate_lease(
    dataset: &DatasetManifest,
    bars: &[Bar],
    lease: &SearchDataLease,
) -> Result<(), RetestError> {
    if lease.stage() != StageAccess::Robustness
        || lease.dataset_id() != dataset.dataset_id
        || lease.range().len() != bars.len()
        || lease.range().start >= lease.range().end
    {
        return Err(RetestError::Invalid(
            "cross-check requires the exact robustness lease".into(),
        ));
    }
    Ok(())
}

fn validate_kind(
    baseline: &CrossCheckObservation,
    check: &CrossCheckObservation,
) -> Result<(), RetestError> {
    let valid = match check.kind {
        CrossCheckKind::Baseline => false,
        CrossCheckKind::OtherSymbol => {
            check.symbol != baseline.symbol && check.dataset_id != baseline.dataset_id
        }
        CrossCheckKind::AdjacentTimeframe => {
            check.symbol == baseline.symbol
                && check.timeframe != baseline.timeframe
                && check.dataset_id != baseline.dataset_id
        }
        CrossCheckKind::AlternativeSource => {
            check.symbol == baseline.symbol
                && check.timeframe == baseline.timeframe
                && check.dataset_id != baseline.dataset_id
                && (check.source != baseline.source
                    || check.venue != baseline.venue
                    || check.pipeline != baseline.pipeline)
        }
        CrossCheckKind::CostSensitivity { multiplier_bps } => {
            matches!(
                multiplier_bps,
                COST_MULTIPLIER_2X_BPS | COST_MULTIPLIER_3X_BPS
            ) && check.dataset_id == baseline.dataset_id
                && check.manifest_id == baseline.manifest_id
                && check.range() == baseline.range()
                && check.config_id != baseline.config_id
        }
    };
    if !valid {
        return Err(RetestError::Invalid(
            "cross-check kind does not match its inputs".into(),
        ));
    }
    Ok(())
}

fn verify_observation(
    strategy: &StrategyIr,
    config: &StrategyExecutionConfig,
    observation: &CrossCheckObservation,
    role: ObservationRole,
    spec: &CrossCheckStudySpec,
) -> Result<(), RetestError> {
    if observation.label.trim().is_empty()
        || !is_id(&observation.dataset_id)
        || !is_id(&observation.manifest_id)
        || observation.range_start >= observation.range_end
        || observation.config_id != config.config_id()
        || !observation.value.is_finite()
        || observation.retention_bps > 20_000
    {
        return Err(RetestError::Invalid(
            "invalid cross-check observation".into(),
        ));
    }
    let expected_seed = component_seed(
        spec.root_seed,
        observation.ordinal,
        observation.kind,
        &observation.label,
        &observation.dataset_id,
        &observation.config_id,
    );
    if observation.component_seed != expected_seed {
        return Err(RetestError::Invalid(
            "cross-check component seed mismatch".into(),
        ));
    }
    let lease = SearchDataLease::exact_partition(
        StageAccess::Robustness,
        observation.dataset_id.clone(),
        observation.range(),
    )?;
    let request = RetestRequest::seal(
        strategy,
        &lease,
        &observation.config_id,
        METRICS_SCHEMA_VERSION,
        expected_seed,
    )?;
    let expected_request =
        execution_request_id(&request, &observation.manifest_id, role, &spec.metric_id);
    if observation.request_id != expected_request {
        return Err(RetestError::Invalid(
            "cross-check request identity mismatch".into(),
        ));
    }
    let report =
        StrategyReportArtifact::from_json_slice(&observation.report_json).map_err(invalid)?;
    if observation.run_id != report.run_id() || observation.report_id != report.report_id() {
        return Err(RetestError::Invalid(
            "cross-check report identity mismatch".into(),
        ));
    }
    let manifest = report
        .run_manifest()
        .ok_or_else(|| RetestError::Invalid("cross-check report lacks run manifest".into()))?;
    let binding = manifest.binding();
    if binding.strategy_id != strategy.strategy_id()
        || binding.config_id != observation.config_id
        || binding.seed != expected_seed
        || binding.metrics_version != METRICS_SCHEMA_VERSION
        || !binding
            .datasets
            .iter()
            .any(|dataset| dataset.dataset_id == observation.dataset_id)
    {
        return Err(RetestError::Invalid("foreign cross-check report".into()));
    }
    match report.analysis().metric(&spec.metric_id) {
        Some(MetricValue::Defined { value })
            if value.is_finite()
                && canonical_zero(*value).to_bits() == observation.value.to_bits() =>
        {
            Ok(())
        }
        _ => Err(RetestError::Invalid(
            "undefined or altered cross-check metric".into(),
        )),
    }
}

fn retention_bps(
    baseline: f64,
    value: f64,
    direction: ObjectiveDirection,
) -> Result<u32, RetestError> {
    if !baseline.is_finite() || !value.is_finite() {
        return Err(RetestError::Invalid(
            "undefined cross-check retention".into(),
        ));
    }
    let raw = match direction {
        ObjectiveDirection::Maximize if baseline > 0.0 => value / baseline * 10_000.0,
        ObjectiveDirection::Minimize if baseline > 0.0 && value > 0.0 => {
            baseline / value * 10_000.0
        }
        ObjectiveDirection::Maximize => {
            10_000.0 + (value - baseline) / baseline.abs().max(1.0) * 10_000.0
        }
        ObjectiveDirection::Minimize => {
            10_000.0 + (baseline - value) / baseline.abs().max(1.0) * 10_000.0
        }
    };
    if !raw.is_finite() {
        return Err(RetestError::Invalid(
            "undefined cross-check retention".into(),
        ));
    }
    Ok(raw.round().clamp(0.0, 20_000.0) as u32)
}

fn baseline_lease_range(baseline: &CrossCheckObservation) -> Range<usize> {
    baseline.range()
}
fn verdict_reason(worst: u32, required: u32, checks: usize) -> String {
    format!("worst of N={checks} cross-checks retained {worst} bps; required {required} bps")
}
fn component_seed(
    root_seed: u64,
    ordinal: usize,
    kind: CrossCheckKind,
    label: &str,
    dataset_id: &str,
    config_id: &str,
) -> u64 {
    let mut hasher = Sha256::new();
    hasher.update(COMPONENT_SEED_DOMAIN);
    hasher.update(root_seed.to_be_bytes());
    hasher.update((ordinal as u64).to_be_bytes());
    frame(&mut hasher, kind.tag());
    if let CrossCheckKind::CostSensitivity { multiplier_bps } = kind {
        hasher.update(multiplier_bps.to_be_bytes());
    }
    frame(&mut hasher, label.as_bytes());
    frame(&mut hasher, dataset_id.as_bytes());
    frame(&mut hasher, config_id.as_bytes());
    let digest = hasher.finalize();
    let mut prefix = [0u8; 8];
    prefix.copy_from_slice(&digest[..8]);
    u64::from_be_bytes(prefix)
}
fn canonical_zero(value: f64) -> f64 {
    if value == 0.0 { 0.0 } else { value }
}
fn is_id(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
fn frame(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}
fn hex(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn invalid(error: impl std::fmt::Display) -> RetestError {
    RetestError::Invalid(error.to_string())
}

#[cfg(test)]
mod tests;
