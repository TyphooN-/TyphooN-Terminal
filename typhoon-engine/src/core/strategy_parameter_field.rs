//! Deterministic report-bound parameter-field analysis for ADR-135 §7.4.
//!
//! Field samples and local plateau neighbours are always executed through the exact leased
//! `VerifiedRun` boundary. The resulting artifact carries a bounded SPP field estimate, explicit
//! plateau membership, and the projection/ranking data consumed by native analysis views.

use crate::broker::alpaca::Bar;
use crate::core::strategy_dataset::DatasetManifest;
use crate::core::strategy_ir::{ParamValue, StrategyExecutionConfig, StrategyIr};
use crate::core::strategy_metrics::{METRICS_SCHEMA_VERSION, MetricValue};
use crate::core::strategy_optimization::{
    MAX_ARTIFACT_BYTES, ObjectiveDirection, ObservationRole, ParameterDomain, RetestRequest,
    SearchDataLease, SearchSpace, SplitMix64, StageAccess, instantiate, ordinal_indices,
};
use crate::core::strategy_report::StrategyReportArtifact;
use crate::core::strategy_retest::{
    RetestError, RetestExecutionRequest, execute_bound_observation, execution_request_id,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::ops::Range;

pub const PARAMETER_FIELD_STUDY_SCHEMA_VERSION: u32 = 1;
pub const MAX_PARAMETER_FIELD_SAMPLE: usize = 64;
pub const MAX_PARAMETER_FIELD_RADIUS: usize = 4;
pub const MAX_PARAMETER_FIELD_NEIGHBOURHOOD: usize = 256;
const CONFIDENCE_LEVEL_BPS: u32 = 9_000;
const ARTIFACT_DOMAIN: &[u8] = b"typhoon.strategy_parameter_field.study.v1";
const COMPONENT_SEED_DOMAIN: &[u8] = b"typhoon.strategy_parameter_field.component_seed.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterFieldStudySpec {
    pub field_sample_size: usize,
    pub neighbour_radius: usize,
    pub plateau_tolerance_bps: u32,
    pub minimum_plateau_neighbours: usize,
    pub metric_id: String,
    pub direction: ObjectiveDirection,
    pub root_seed: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParameterFieldPhase {
    FieldSample,
    PlateauNeighbour,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlateauVerdict {
    SharpIsolatedOptimum,
    BroadStableRegion,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParameterFieldAxis {
    id: String,
    values: Vec<ParamValue>,
}
impl ParameterFieldAxis {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn values(&self) -> &[ParamValue] {
        &self.values
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParameterFieldPoint {
    pub evaluation_n: usize,
    pub phase: ParameterFieldPhase,
    pub ordinal: usize,
    pub candidate_id: String,
    pub assignments: Vec<(String, ParamValue)>,
    pub axis_indices: Vec<usize>,
    pub component_seed: u64,
    pub request_id: String,
    pub run_id: String,
    pub report_id: String,
    pub value: f64,
    pub rank: usize,
    report_json: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldPercentiles {
    confidence_level_bps: u32,
    p05: f64,
    median: f64,
    p95: f64,
}
impl FieldPercentiles {
    pub fn confidence_level_bps(&self) -> u32 {
        self.confidence_level_bps
    }
    pub fn p05(&self) -> f64 {
        self.p05
    }
    pub fn median(&self) -> f64 {
        self.median
    }
    pub fn p95(&self) -> f64 {
        self.p95
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SystemParameterPermutationEvidence {
    sample_size: usize,
    field_combinations: usize,
    exhaustive: bool,
    sorted_values: Vec<f64>,
    percentiles: FieldPercentiles,
    estimate: f64,
    field_minimum: f64,
    field_maximum: f64,
    selected_value: f64,
    optimization_bias: f64,
    optimization_bias_bps: u32,
}
impl SystemParameterPermutationEvidence {
    pub fn sample_size(&self) -> usize {
        self.sample_size
    }
    pub fn field_combinations(&self) -> usize {
        self.field_combinations
    }
    pub fn exhaustive(&self) -> bool {
        self.exhaustive
    }
    pub fn sorted_values(&self) -> &[f64] {
        &self.sorted_values
    }
    pub fn percentiles(&self) -> &FieldPercentiles {
        &self.percentiles
    }
    pub fn estimate(&self) -> f64 {
        self.estimate
    }
    pub fn field_minimum(&self) -> f64 {
        self.field_minimum
    }
    pub fn field_maximum(&self) -> f64 {
        self.field_maximum
    }
    pub fn selected_value(&self) -> f64 {
        self.selected_value
    }
    pub fn optimization_bias(&self) -> f64 {
        self.optimization_bias
    }
    pub fn optimization_bias_bps(&self) -> u32 {
        self.optimization_bias_bps
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlateauMember {
    pub ordinal: usize,
    pub candidate_id: String,
    pub report_id: String,
    pub value: f64,
    pub holds: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParameterPlateauEvidence {
    centre_ordinal: usize,
    centre_value: f64,
    radius: usize,
    tolerance_bps: u32,
    scale: f64,
    threshold: f64,
    members: Vec<PlateauMember>,
    holding_members: usize,
    stability_bps: u32,
    verdict: PlateauVerdict,
}
impl ParameterPlateauEvidence {
    pub fn centre_ordinal(&self) -> usize {
        self.centre_ordinal
    }
    pub fn centre_value(&self) -> f64 {
        self.centre_value
    }
    pub fn radius(&self) -> usize {
        self.radius
    }
    pub fn tolerance_bps(&self) -> u32 {
        self.tolerance_bps
    }
    pub fn scale(&self) -> f64 {
        self.scale
    }
    pub fn threshold(&self) -> f64 {
        self.threshold
    }
    pub fn members(&self) -> &[PlateauMember] {
        &self.members
    }
    pub fn holding_members(&self) -> usize {
        self.holding_members
    }
    pub fn stability_bps(&self) -> u32 {
        self.stability_bps
    }
    pub fn verdict(&self) -> PlateauVerdict {
        self.verdict
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OptimizationProfileEvidence {
    observations_n: usize,
    evaluations_n: usize,
    selected_ordinal: usize,
    selected_candidate_id: String,
    selected_rank: usize,
    selection_label: String,
    stability_bps: u32,
    within_tolerance: usize,
}
impl OptimizationProfileEvidence {
    pub fn observations_n(&self) -> usize {
        self.observations_n
    }
    pub fn evaluations_n(&self) -> usize {
        self.evaluations_n
    }
    pub fn selected_ordinal(&self) -> usize {
        self.selected_ordinal
    }
    pub fn selected_candidate_id(&self) -> &str {
        &self.selected_candidate_id
    }
    pub fn selected_rank(&self) -> usize {
        self.selected_rank
    }
    pub fn selection_label(&self) -> &str {
        &self.selection_label
    }
    pub fn stability_bps(&self) -> u32 {
        self.stability_bps
    }
    pub fn within_tolerance(&self) -> usize {
        self.within_tolerance
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SerializableSpec {
    field_sample_size: usize,
    neighbour_radius: usize,
    plateau_tolerance_bps: u32,
    minimum_plateau_neighbours: usize,
    metric_id: String,
    direction: ObjectiveDirection,
    root_seed: u64,
}
impl From<ParameterFieldStudySpec> for SerializableSpec {
    fn from(value: ParameterFieldStudySpec) -> Self {
        Self {
            field_sample_size: value.field_sample_size,
            neighbour_radius: value.neighbour_radius,
            plateau_tolerance_bps: value.plateau_tolerance_bps,
            minimum_plateau_neighbours: value.minimum_plateau_neighbours,
            metric_id: value.metric_id,
            direction: value.direction,
            root_seed: value.root_seed,
        }
    }
}
impl SerializableSpec {
    fn public(&self) -> ParameterFieldStudySpec {
        ParameterFieldStudySpec {
            field_sample_size: self.field_sample_size,
            neighbour_radius: self.neighbour_radius,
            plateau_tolerance_bps: self.plateau_tolerance_bps,
            minimum_plateau_neighbours: self.minimum_plateau_neighbours,
            metric_id: self.metric_id.clone(),
            direction: self.direction,
            root_seed: self.root_seed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParameterFieldStudyArtifact {
    schema_version: u32,
    artifact_id: String,
    source_dataset_id: String,
    source_manifest_id: String,
    source_stage: StageAccess,
    config_id: String,
    range_start: usize,
    range_end: usize,
    base_strategy_json: Vec<u8>,
    axes: Vec<ParameterFieldAxis>,
    spec: SerializableSpec,
    evaluations_n: usize,
    points: Vec<ParameterFieldPoint>,
    spp: SystemParameterPermutationEvidence,
    plateau: ParameterPlateauEvidence,
    profile: OptimizationProfileEvidence,
}

impl ParameterFieldStudyArtifact {
    pub fn artifact_id(&self) -> &str {
        &self.artifact_id
    }
    pub fn source_dataset_id(&self) -> &str {
        &self.source_dataset_id
    }
    pub fn source_manifest_id(&self) -> &str {
        &self.source_manifest_id
    }
    pub fn config_id(&self) -> &str {
        &self.config_id
    }
    pub fn range(&self) -> Range<usize> {
        self.range_start..self.range_end
    }
    pub fn metric_id(&self) -> &str {
        &self.spec.metric_id
    }
    pub fn direction(&self) -> ObjectiveDirection {
        self.spec.direction
    }
    pub fn root_seed(&self) -> u64 {
        self.spec.root_seed
    }
    pub fn evaluations_n(&self) -> usize {
        self.evaluations_n
    }
    pub fn axes(&self) -> &[ParameterFieldAxis] {
        &self.axes
    }
    pub fn points(&self) -> &[ParameterFieldPoint] {
        &self.points
    }
    pub fn spp(&self) -> &SystemParameterPermutationEvidence {
        &self.spp
    }
    pub fn plateau(&self) -> &ParameterPlateauEvidence {
        &self.plateau
    }
    pub fn profile(&self) -> &OptimizationProfileEvidence {
        &self.profile
    }
    pub fn to_json_vec(&self) -> Result<Vec<u8>, RetestError> {
        self.verify()?;
        let bytes = serde_json::to_vec(self).map_err(invalid)?;
        if bytes.len() > MAX_ARTIFACT_BYTES {
            return Err(RetestError::Invalid(
                "parameter-field artifact is too large".into(),
            ));
        }
        Ok(bytes)
    }
    pub fn from_json_slice(bytes: &[u8]) -> Result<Self, RetestError> {
        if bytes.len() > MAX_ARTIFACT_BYTES {
            return Err(RetestError::Invalid(
                "parameter-field artifact is too large".into(),
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
        if self.schema_version != PARAMETER_FIELD_STUDY_SCHEMA_VERSION
            || !is_id(&self.artifact_id)
            || !is_id(&self.source_dataset_id)
            || !is_id(&self.source_manifest_id)
            || !is_id(&self.config_id)
            || self.source_stage != StageAccess::Robustness
            || self.range_start >= self.range_end
            || self.evaluations_n != self.spec.field_sample_size
            || self.points.is_empty()
        {
            return Err(RetestError::Invalid(
                "invalid parameter-field artifact header".into(),
            ));
        }
        let base = StrategyIr::from_json_slice(&self.base_strategy_json).map_err(invalid)?;
        let domains = self
            .axes
            .iter()
            .map(|axis| ParameterDomain::new(&axis.id, axis.values.clone()))
            .collect::<Result<Vec<_>, _>>()?;
        let space = SearchSpace::new(base, domains)?;
        let spec = self.spec.public();
        validate_spec(&space, &spec)?;
        let expected_axes = axes(&space);
        if self.axes != expected_axes {
            return Err(RetestError::Invalid("parameter-field axes mismatch".into()));
        }
        let sample_ordinals = field_sample_ordinals(&space, &spec)?;
        let sample_len = sample_ordinals.len();
        if self.points.len() < sample_len {
            return Err(RetestError::Invalid("missing field sample evidence".into()));
        }
        let selected_ordinal = select_ordinal(&self.points[..sample_len], spec.direction)?;
        let neighbours = neighbourhood_ordinals(&space, selected_ordinal, spec.neighbour_radius)?;
        if maximum_neighbourhood_size(&space, spec.neighbour_radius)
            < spec.minimum_plateau_neighbours
        {
            return Err(RetestError::Invalid(
                "plateau neighbourhood cannot satisfy quorum".into(),
            ));
        }
        let sampled = sample_ordinals.iter().copied().collect::<BTreeSet<_>>();
        let mut expected_ordinals = sample_ordinals.clone();
        expected_ordinals.extend(
            neighbours
                .iter()
                .copied()
                .filter(|value| !sampled.contains(value)),
        );
        if self.points.len() != expected_ordinals.len() {
            return Err(RetestError::Invalid(
                "parameter-field execution set mismatch".into(),
            ));
        }
        let lease = SearchDataLease::exact_partition(
            self.source_stage,
            self.source_dataset_id.clone(),
            self.range(),
        )?;
        let mut seen_ordinals = BTreeSet::new();
        let mut seen_requests = BTreeSet::new();
        let mut seen_runs = BTreeSet::new();
        let mut seen_reports = BTreeSet::new();
        for (index, (point, ordinal)) in self.points.iter().zip(expected_ordinals).enumerate() {
            let expected_phase = if index < sample_len {
                ParameterFieldPhase::FieldSample
            } else {
                ParameterFieldPhase::PlateauNeighbour
            };
            let candidate = instantiate(&space, ordinal)?;
            let expected_seed = component_seed(
                spec.root_seed,
                index + 1,
                expected_phase,
                &candidate.candidate_id,
            );
            if point.evaluation_n != index + 1
                || point.phase != expected_phase
                || point.ordinal != ordinal
                || point.candidate_id != candidate.candidate_id
                || point.assignments != candidate.assignments
                || point.axis_indices != ordinal_indices(&space, ordinal)
                || point.component_seed != expected_seed
                || !point.value.is_finite()
                || !seen_ordinals.insert(point.ordinal)
                || !seen_requests.insert(point.request_id.as_str())
                || !seen_runs.insert(point.run_id.as_str())
                || !seen_reports.insert(point.report_id.as_str())
            {
                return Err(RetestError::Invalid("invalid parameter-field point".into()));
            }
            let report =
                StrategyReportArtifact::from_json_slice(&point.report_json).map_err(invalid)?;
            if point.run_id != report.run_id() || point.report_id != report.report_id() {
                return Err(RetestError::Invalid(
                    "stored parameter-field report identity mismatch".into(),
                ));
            }
            let retest = RetestRequest::seal(
                &candidate.strategy,
                &lease,
                &self.config_id,
                METRICS_SCHEMA_VERSION,
                expected_seed,
            )?;
            let expected_request = execution_request_id(
                &retest,
                &self.source_manifest_id,
                ObservationRole::SearchEvaluation,
                &spec.metric_id,
            );
            if point.request_id != expected_request {
                return Err(RetestError::Invalid(
                    "parameter-field request identity mismatch".into(),
                ));
            }
            verify_report_point(
                point,
                &report,
                &candidate.candidate_id,
                &self.source_dataset_id,
                &self.config_id,
                &spec.metric_id,
            )?;
        }
        let expected_ranks = ranks(&self.points, spec.direction);
        if self
            .points
            .iter()
            .zip(expected_ranks)
            .any(|(point, rank)| point.rank != rank)
        {
            return Err(RetestError::Invalid("parameter-field rank mismatch".into()));
        }
        let spp = derive_spp(
            &self.points[..sample_len],
            space.combinations(),
            selected_ordinal,
            spec.direction,
        )?;
        let plateau = derive_plateau(&self.points, &neighbours, selected_ordinal, &spp, &spec)?;
        let profile = derive_profile(&self.points, &plateau, sample_len, selected_ordinal)?;
        if self.spp != spp || self.plateau != plateau || self.profile != profile {
            return Err(RetestError::Invalid(
                "derived parameter-field evidence mismatch".into(),
            ));
        }
        if self.artifact_id != self.compute_id()? {
            return Err(RetestError::Invalid(
                "parameter-field identity mismatch".into(),
            ));
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

pub fn execute_parameter_field_study(
    config: &StrategyExecutionConfig,
    dataset: &DatasetManifest,
    bars: &[Bar],
    lease: SearchDataLease,
    space: &SearchSpace,
    spec: ParameterFieldStudySpec,
) -> Result<ParameterFieldStudyArtifact, RetestError> {
    validate_spec(space, &spec)?;
    config.verify().map_err(invalid)?;
    dataset.verify(bars).map_err(invalid)?;
    if lease.dataset_id() != dataset.dataset_id
        || lease.range().len() != bars.len()
        || lease.stage() != StageAccess::Robustness
    {
        return Err(RetestError::Invalid(
            "parameter-field study requires the exact robustness lease".into(),
        ));
    }
    let sample_ordinals = field_sample_ordinals(space, &spec)?;
    let mut points = Vec::with_capacity(spec.field_sample_size + MAX_PARAMETER_FIELD_NEIGHBOURHOOD);
    for ordinal in &sample_ordinals {
        points.push(execute_point(
            config,
            dataset,
            bars,
            &lease,
            space,
            &spec,
            *ordinal,
            ParameterFieldPhase::FieldSample,
            points.len() + 1,
        )?);
    }
    let selected_ordinal = select_ordinal(&points, spec.direction)?;
    let neighbours = neighbourhood_ordinals(space, selected_ordinal, spec.neighbour_radius)?;
    if maximum_neighbourhood_size(space, spec.neighbour_radius) < spec.minimum_plateau_neighbours {
        return Err(RetestError::Invalid(
            "plateau neighbourhood cannot satisfy quorum".into(),
        ));
    }
    let mut executed = sample_ordinals.into_iter().collect::<BTreeSet<_>>();
    for ordinal in &neighbours {
        if executed.insert(*ordinal) {
            points.push(execute_point(
                config,
                dataset,
                bars,
                &lease,
                space,
                &spec,
                *ordinal,
                ParameterFieldPhase::PlateauNeighbour,
                points.len() + 1,
            )?);
        }
    }
    assign_ranks(&mut points, spec.direction);
    let sample_len = spec.field_sample_size;
    let spp = derive_spp(
        &points[..sample_len],
        space.combinations(),
        selected_ordinal,
        spec.direction,
    )?;
    let plateau = derive_plateau(&points, &neighbours, selected_ordinal, &spp, &spec)?;
    let profile = derive_profile(&points, &plateau, sample_len, selected_ordinal)?;
    let mut artifact = ParameterFieldStudyArtifact {
        schema_version: PARAMETER_FIELD_STUDY_SCHEMA_VERSION,
        artifact_id: String::new(),
        source_dataset_id: dataset.dataset_id.clone(),
        source_manifest_id: dataset.manifest_id.clone(),
        source_stage: lease.stage(),
        config_id: config.config_id().to_string(),
        range_start: lease.range().start,
        range_end: lease.range().end,
        base_strategy_json: serde_json::to_vec(space.base()).map_err(invalid)?,
        axes: axes(space),
        spec: spec.into(),
        evaluations_n: sample_len,
        points,
        spp,
        plateau,
        profile,
    };
    artifact.artifact_id = artifact.compute_id()?;
    artifact.verify()?;
    let _ = artifact.to_json_vec()?;
    Ok(artifact)
}

pub fn replay_parameter_field_study(
    config: &StrategyExecutionConfig,
    dataset: &DatasetManifest,
    bars: &[Bar],
    lease: SearchDataLease,
    space: &SearchSpace,
    expected: &ParameterFieldStudyArtifact,
) -> Result<ParameterFieldStudyArtifact, RetestError> {
    expected.verify()?;
    let replay =
        execute_parameter_field_study(config, dataset, bars, lease, space, expected.spec.public())?;
    if &replay != expected {
        return Err(RetestError::Invalid(
            "parameter-field replay mismatch".into(),
        ));
    }
    Ok(replay)
}

#[allow(clippy::too_many_arguments)]
fn execute_point(
    config: &StrategyExecutionConfig,
    dataset: &DatasetManifest,
    bars: &[Bar],
    lease: &SearchDataLease,
    space: &SearchSpace,
    spec: &ParameterFieldStudySpec,
    ordinal: usize,
    phase: ParameterFieldPhase,
    evaluation_n: usize,
) -> Result<ParameterFieldPoint, RetestError> {
    let candidate = instantiate(space, ordinal)?;
    let seed = component_seed(spec.root_seed, evaluation_n, phase, &candidate.candidate_id);
    let execution = RetestExecutionRequest::seal(
        &candidate.strategy,
        config,
        dataset,
        bars,
        SearchDataLease::exact_partition(
            lease.stage(),
            lease.dataset_id().to_string(),
            lease.range(),
        )?,
        ObservationRole::SearchEvaluation,
        &spec.metric_id,
        seed,
    )?;
    let request_id = execution.request_id().to_string();
    let (report, observation, value) = execute_bound_observation(&execution)?;
    if observation.candidate_id() != candidate.candidate_id
        || observation.report_id() != report.report_id()
        || !value.is_finite()
    {
        return Err(RetestError::Invalid(
            "canonical field observation mismatch".into(),
        ));
    }
    Ok(ParameterFieldPoint {
        evaluation_n,
        phase,
        ordinal,
        candidate_id: candidate.candidate_id,
        assignments: candidate.assignments,
        axis_indices: ordinal_indices(space, ordinal),
        component_seed: seed,
        request_id,
        run_id: report.run_id().to_string(),
        report_id: report.report_id().to_string(),
        value: canonical_zero(value),
        rank: 0,
        report_json: report.to_json_vec().map_err(invalid)?,
    })
}

fn validate_spec(space: &SearchSpace, spec: &ParameterFieldStudySpec) -> Result<(), RetestError> {
    if spec.field_sample_size < 2
        || spec.field_sample_size > MAX_PARAMETER_FIELD_SAMPLE
        || spec.field_sample_size > space.combinations()
        || spec.neighbour_radius == 0
        || spec.neighbour_radius > MAX_PARAMETER_FIELD_RADIUS
        || spec.plateau_tolerance_bps > 10_000
        || spec.minimum_plateau_neighbours == 0
        || spec.minimum_plateau_neighbours > MAX_PARAMETER_FIELD_NEIGHBOURHOOD
        || spec.metric_id.trim().is_empty()
    {
        return Err(RetestError::Invalid(
            "invalid parameter-field study specification".into(),
        ));
    }
    Ok(())
}

fn axes(space: &SearchSpace) -> Vec<ParameterFieldAxis> {
    space
        .domains()
        .iter()
        .map(|domain| ParameterFieldAxis {
            id: domain.id().to_string(),
            values: domain.values().to_vec(),
        })
        .collect()
}

fn field_sample_ordinals(
    space: &SearchSpace,
    spec: &ParameterFieldStudySpec,
) -> Result<Vec<usize>, RetestError> {
    if spec.field_sample_size == space.combinations() {
        return Ok((0..space.combinations()).collect());
    }
    let mut rng = SplitMix64(spec.root_seed ^ 0x5f50_5046_4945_4c44);
    let mut seen = BTreeSet::new();
    let mut ordinals = Vec::with_capacity(spec.field_sample_size);
    let attempts = spec.field_sample_size.saturating_mul(32).max(64);
    for _ in 0..attempts {
        let ordinal = (rng.next() as usize) % space.combinations();
        if seen.insert(ordinal) {
            ordinals.push(ordinal);
            if ordinals.len() == spec.field_sample_size {
                return Ok(ordinals);
            }
        }
    }
    for ordinal in 0..space.combinations() {
        if seen.insert(ordinal) {
            ordinals.push(ordinal);
            if ordinals.len() == spec.field_sample_size {
                return Ok(ordinals);
            }
        }
    }
    Err(RetestError::Invalid(
        "parameter field exhausted before sample completed".into(),
    ))
}

fn neighbourhood_ordinals(
    space: &SearchSpace,
    centre_ordinal: usize,
    radius: usize,
) -> Result<Vec<usize>, RetestError> {
    let centre = ordinal_indices(space, centre_ordinal);
    let mut combinations = 1usize;
    let ranges = centre
        .iter()
        .zip(space.domains())
        .map(|(index, domain)| {
            let start = index.saturating_sub(radius);
            let end = index.saturating_add(radius).min(domain.values().len() - 1);
            combinations = combinations
                .checked_mul(end - start + 1)
                .unwrap_or(usize::MAX);
            start..=end
        })
        .collect::<Vec<_>>();
    if combinations.saturating_sub(1) > MAX_PARAMETER_FIELD_NEIGHBOURHOOD {
        return Err(RetestError::Invalid(
            "parameter-field neighbourhood exceeds bound".into(),
        ));
    }
    let mut coordinates = vec![0usize; ranges.len()];
    let mut output = Vec::with_capacity(combinations.saturating_sub(1));
    enumerate_neighbourhood(
        space,
        &ranges,
        0,
        &mut coordinates,
        centre_ordinal,
        &mut output,
    );
    output.sort_unstable();
    output.dedup();
    Ok(output)
}

fn maximum_neighbourhood_size(space: &SearchSpace, radius: usize) -> usize {
    space
        .domains()
        .iter()
        .try_fold(1usize, |total, domain| {
            total.checked_mul(domain.values().len().min(radius.saturating_mul(2) + 1))
        })
        .unwrap_or(usize::MAX)
        .saturating_sub(1)
}

fn enumerate_neighbourhood(
    space: &SearchSpace,
    ranges: &[std::ops::RangeInclusive<usize>],
    axis: usize,
    coordinates: &mut [usize],
    centre_ordinal: usize,
    output: &mut Vec<usize>,
) {
    if axis == ranges.len() {
        let ordinal = indices_ordinal(space, coordinates);
        if ordinal != centre_ordinal {
            output.push(ordinal);
        }
        return;
    }
    for value in ranges[axis].clone() {
        coordinates[axis] = value;
        enumerate_neighbourhood(space, ranges, axis + 1, coordinates, centre_ordinal, output);
    }
}

fn indices_ordinal(space: &SearchSpace, indices: &[usize]) -> usize {
    space
        .domains()
        .iter()
        .zip(indices)
        .fold(0usize, |ordinal, (domain, index)| {
            ordinal * domain.values().len() + index
        })
}

fn select_ordinal(
    points: &[ParameterFieldPoint],
    direction: ObjectiveDirection,
) -> Result<usize, RetestError> {
    points
        .iter()
        .min_by(|left, right| objective_order(left, right, direction))
        .map(|point| point.ordinal)
        .ok_or_else(|| RetestError::Invalid("empty parameter field".into()))
}

fn objective_order(
    left: &ParameterFieldPoint,
    right: &ParameterFieldPoint,
    direction: ObjectiveDirection,
) -> std::cmp::Ordering {
    let order = match direction {
        ObjectiveDirection::Maximize => right.value.total_cmp(&left.value),
        ObjectiveDirection::Minimize => left.value.total_cmp(&right.value),
    };
    order.then_with(|| left.ordinal.cmp(&right.ordinal))
}

fn ranks(points: &[ParameterFieldPoint], direction: ObjectiveDirection) -> Vec<usize> {
    let mut order = (0..points.len()).collect::<Vec<_>>();
    order.sort_by(|left, right| objective_order(&points[*left], &points[*right], direction));
    let mut ranks = vec![0usize; points.len()];
    for (rank, index) in order.into_iter().enumerate() {
        ranks[index] = rank + 1;
    }
    ranks
}

fn assign_ranks(points: &mut [ParameterFieldPoint], direction: ObjectiveDirection) {
    let derived = ranks(points, direction);
    for (point, rank) in points.iter_mut().zip(derived) {
        point.rank = rank;
    }
}

fn derive_spp(
    sample: &[ParameterFieldPoint],
    field_combinations: usize,
    selected_ordinal: usize,
    direction: ObjectiveDirection,
) -> Result<SystemParameterPermutationEvidence, RetestError> {
    let mut values = sample.iter().map(|point| point.value).collect::<Vec<_>>();
    if values.len() < 2 || values.iter().any(|value| !value.is_finite()) {
        return Err(RetestError::Invalid(
            "undefined SPP field distribution".into(),
        ));
    }
    values.sort_by(f64::total_cmp);
    let minimum = values[0];
    let maximum = values[values.len() - 1];
    let p05 = values[percentile_index(values.len(), 5)];
    let median = values[percentile_index(values.len(), 50)];
    let p95 = values[percentile_index(values.len(), 95)];
    let selected = sample
        .iter()
        .find(|point| point.ordinal == selected_ordinal)
        .ok_or_else(|| RetestError::Invalid("selected field point is absent".into()))?
        .value;
    let bias = match direction {
        ObjectiveDirection::Maximize => selected - median,
        ObjectiveDirection::Minimize => median - selected,
    }
    .max(0.0);
    let scale = maximum - minimum;
    let bias_bps = if scale > 0.0 {
        ((bias / scale) * 10_000.0).round().clamp(0.0, 10_000.0) as u32
    } else {
        0
    };
    Ok(SystemParameterPermutationEvidence {
        sample_size: sample.len(),
        field_combinations,
        exhaustive: sample.len() == field_combinations,
        sorted_values: values,
        percentiles: FieldPercentiles {
            confidence_level_bps: CONFIDENCE_LEVEL_BPS,
            p05: canonical_zero(p05),
            median: canonical_zero(median),
            p95: canonical_zero(p95),
        },
        estimate: canonical_zero(median),
        field_minimum: canonical_zero(minimum),
        field_maximum: canonical_zero(maximum),
        selected_value: canonical_zero(selected),
        optimization_bias: canonical_zero(bias),
        optimization_bias_bps: bias_bps,
    })
}

fn derive_plateau(
    points: &[ParameterFieldPoint],
    neighbour_ordinals: &[usize],
    selected_ordinal: usize,
    spp: &SystemParameterPermutationEvidence,
    spec: &ParameterFieldStudySpec,
) -> Result<ParameterPlateauEvidence, RetestError> {
    let centre = points
        .iter()
        .find(|point| point.ordinal == selected_ordinal)
        .ok_or_else(|| RetestError::Invalid("plateau centre is absent".into()))?;
    let scale = spp.field_maximum - spp.field_minimum;
    let tolerance = scale * f64::from(spec.plateau_tolerance_bps) / 10_000.0;
    let threshold = match spec.direction {
        ObjectiveDirection::Maximize => centre.value - tolerance,
        ObjectiveDirection::Minimize => centre.value + tolerance,
    };
    let mut members = neighbour_ordinals
        .iter()
        .map(|ordinal| {
            let point = points
                .iter()
                .find(|point| point.ordinal == *ordinal)
                .ok_or_else(|| RetestError::Invalid("plateau member was not executed".into()))?;
            let holds = match spec.direction {
                ObjectiveDirection::Maximize => point.value >= threshold,
                ObjectiveDirection::Minimize => point.value <= threshold,
            };
            Ok(PlateauMember {
                ordinal: *ordinal,
                candidate_id: point.candidate_id.clone(),
                report_id: point.report_id.clone(),
                value: point.value,
                holds,
            })
        })
        .collect::<Result<Vec<_>, RetestError>>()?;
    // Keep failed neighbours first so bounded UI consumers see cliffs before supporting evidence.
    members.sort_by_key(|member| (member.holds, member.ordinal));
    let holding_members = members.iter().filter(|member| member.holds).count();
    let stability_bps = if members.is_empty() {
        0
    } else {
        ((holding_members * 10_000) / members.len()) as u32
    };
    let required_members = spec.minimum_plateau_neighbours.min(members.len());
    let verdict = if holding_members >= required_members {
        PlateauVerdict::BroadStableRegion
    } else {
        PlateauVerdict::SharpIsolatedOptimum
    };
    Ok(ParameterPlateauEvidence {
        centre_ordinal: selected_ordinal,
        centre_value: canonical_zero(centre.value),
        radius: spec.neighbour_radius,
        tolerance_bps: spec.plateau_tolerance_bps,
        scale: canonical_zero(scale),
        threshold: canonical_zero(threshold),
        members,
        holding_members,
        stability_bps,
        verdict,
    })
}

fn derive_profile(
    points: &[ParameterFieldPoint],
    plateau: &ParameterPlateauEvidence,
    evaluations_n: usize,
    selected_ordinal: usize,
) -> Result<OptimizationProfileEvidence, RetestError> {
    let selected = points
        .iter()
        .find(|point| point.ordinal == selected_ordinal)
        .ok_or_else(|| RetestError::Invalid("profile selection is absent".into()))?;
    Ok(OptimizationProfileEvidence {
        observations_n: points.len(),
        evaluations_n,
        selected_ordinal,
        selected_candidate_id: selected.candidate_id.clone(),
        selected_rank: selected.rank,
        selection_label: format!("best of N={evaluations_n}: {}", selected.candidate_id),
        stability_bps: plateau.stability_bps,
        within_tolerance: plateau.holding_members + 1,
    })
}

fn verify_report_point(
    point: &ParameterFieldPoint,
    report: &StrategyReportArtifact,
    candidate_id: &str,
    dataset_id: &str,
    config_id: &str,
    metric_id: &str,
) -> Result<(), RetestError> {
    let manifest = report
        .run_manifest()
        .ok_or_else(|| RetestError::Invalid("parameter-field report lacks run manifest".into()))?;
    let binding = manifest.binding();
    if binding.strategy_id != candidate_id
        || binding.config_id != config_id
        || binding.seed != point.component_seed
        || binding.metrics_version != METRICS_SCHEMA_VERSION
        || !binding
            .datasets
            .iter()
            .any(|dataset| dataset.dataset_id == dataset_id)
    {
        return Err(RetestError::Invalid(
            "foreign parameter-field report".into(),
        ));
    }
    match report.analysis().metric(metric_id) {
        Some(MetricValue::Defined { value })
            if value.is_finite() && canonical_zero(*value).to_bits() == point.value.to_bits() =>
        {
            Ok(())
        }
        _ => Err(RetestError::Invalid(
            "undefined or altered parameter-field metric".into(),
        )),
    }
}

fn component_seed(
    root_seed: u64,
    evaluation_n: usize,
    phase: ParameterFieldPhase,
    candidate_id: &str,
) -> u64 {
    let mut hasher = Sha256::new();
    hasher.update(COMPONENT_SEED_DOMAIN);
    hasher.update(root_seed.to_be_bytes());
    hasher.update((evaluation_n as u64).to_be_bytes());
    hasher.update([match phase {
        ParameterFieldPhase::FieldSample => 0,
        ParameterFieldPhase::PlateauNeighbour => 1,
    }]);
    frame(&mut hasher, candidate_id.as_bytes());
    let digest = hasher.finalize();
    let mut prefix = [0u8; 8];
    prefix.copy_from_slice(&digest[..8]);
    u64::from_be_bytes(prefix)
}

fn percentile_index(len: usize, percentile: usize) -> usize {
    ((len - 1) * percentile + 50) / 100
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
