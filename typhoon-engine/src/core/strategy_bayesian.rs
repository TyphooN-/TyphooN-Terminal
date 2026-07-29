//! Deterministic adaptive optimization over the exact ADR-135 retest boundary.
//!
//! The surrogate is a bounded mixed-domain k-nearest-neighbour model over canonical discrete
//! parameter ordinals. It is intentionally small and auditable: seeded design points are followed
//! by deterministic acquisition over a bounded unseen pool, and the model is updated only after a
//! finite metric has been projected from a verified canonical report.

use crate::broker::alpaca::Bar;
use crate::core::strategy_dataset::DatasetManifest;
use crate::core::strategy_ir::{ParamValue, StrategyExecutionConfig, StrategyIr};
use crate::core::strategy_metrics::{METRICS_SCHEMA_VERSION, MetricValue};
use crate::core::strategy_optimization::{
    Candidate, MAX_ARTIFACT_BYTES, ObjectiveDirection, ObservationRole, OptimizationError,
    ParameterDomain, RetestRequest, SearchDataLease, SearchMethod, SearchSpace, SplitMix64,
    StageAccess, generate_candidates, instantiate, ordinal_indices,
};
use crate::core::strategy_report::StrategyReportArtifact;
use crate::core::strategy_retest::{
    RetestError, RetestExecutionRequest, execute_bound_observation, execution_request_id,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const BAYESIAN_STUDY_SCHEMA_VERSION: u32 = 1;
pub const MAX_BAYESIAN_EVALUATIONS: usize = 64;
pub const MAX_BAYESIAN_ACQUISITION_POOL: usize = 4_096;
pub const MAX_BAYESIAN_NEIGHBOURS: usize = 16;
pub const MAX_BAYESIAN_RECORDED_ACQUISITIONS: usize = 4_096;
const ARTIFACT_DOMAIN: &[u8] = b"typhoon.strategy_bayesian.study.v1";
const COMPONENT_SEED_DOMAIN: &[u8] = b"typhoon.strategy_bayesian.component_seed.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BayesianOptimizationSpec {
    pub budget: usize,
    pub initial_design_size: usize,
    pub acquisition_pool_limit: usize,
    pub nearest_neighbors: usize,
    pub exploration_bps: u32,
    pub metric_id: String,
    pub direction: ObjectiveDirection,
    pub root_seed: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BayesianProposalKind {
    SeededDesign,
    Acquisition,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BayesianAcquisitionScore {
    pub ordinal: usize,
    pub candidate_id: String,
    pub assignments: Vec<(String, ParamValue)>,
    pub predicted_value: f64,
    pub uncertainty: f64,
    pub acquisition_value: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BayesianProposalDecision {
    pub evaluation_n: usize,
    pub kind: BayesianProposalKind,
    pub ordinal: usize,
    pub candidate_id: String,
    pub assignments: Vec<(String, ParamValue)>,
    /// Empty for seeded design points. Acquisition decisions retain the complete bounded pool so
    /// the selected argmax and every tie can be replayed without hidden model state.
    pub acquisition_pool: Vec<BayesianAcquisitionScore>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BayesianVerifiedObservation {
    pub evaluation_n: usize,
    pub candidate_id: String,
    pub request_id: String,
    pub run_id: String,
    pub report_id: String,
    pub component_seed: u64,
    pub value: f64,
    report_json: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SerializableSpec {
    budget: usize,
    initial_design_size: usize,
    acquisition_pool_limit: usize,
    nearest_neighbors: usize,
    exploration_bps: u32,
    metric_id: String,
    direction: ObjectiveDirection,
    root_seed: u64,
}

impl From<BayesianOptimizationSpec> for SerializableSpec {
    fn from(value: BayesianOptimizationSpec) -> Self {
        Self {
            budget: value.budget,
            initial_design_size: value.initial_design_size,
            acquisition_pool_limit: value.acquisition_pool_limit,
            nearest_neighbors: value.nearest_neighbors,
            exploration_bps: value.exploration_bps,
            metric_id: value.metric_id,
            direction: value.direction,
            root_seed: value.root_seed,
        }
    }
}
impl SerializableSpec {
    fn public(&self) -> BayesianOptimizationSpec {
        BayesianOptimizationSpec {
            budget: self.budget,
            initial_design_size: self.initial_design_size,
            acquisition_pool_limit: self.acquisition_pool_limit,
            nearest_neighbors: self.nearest_neighbors,
            exploration_bps: self.exploration_bps,
            metric_id: self.metric_id.clone(),
            direction: self.direction,
            root_seed: self.root_seed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BayesianDomain {
    id: String,
    values: Vec<ParamValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BayesianStudyArtifact {
    schema_version: u32,
    artifact_id: String,
    source_dataset_id: String,
    source_manifest_id: String,
    source_stage: StageAccess,
    config_id: String,
    range_start: usize,
    range_end: usize,
    base_strategy_json: Vec<u8>,
    domains: Vec<BayesianDomain>,
    spec: SerializableSpec,
    evaluations_n: usize,
    decisions: Vec<BayesianProposalDecision>,
    observations: Vec<BayesianVerifiedObservation>,
}

impl BayesianStudyArtifact {
    pub fn artifact_id(&self) -> &str {
        &self.artifact_id
    }
    pub fn source_dataset_id(&self) -> &str {
        &self.source_dataset_id
    }
    pub fn evaluations_n(&self) -> usize {
        self.evaluations_n
    }
    pub fn decisions(&self) -> &[BayesianProposalDecision] {
        &self.decisions
    }
    pub fn observations(&self) -> &[BayesianVerifiedObservation] {
        &self.observations
    }
    pub fn to_json_vec(&self) -> Result<Vec<u8>, RetestError> {
        self.verify()?;
        let bytes = serde_json::to_vec(self).map_err(invalid)?;
        if bytes.len() > MAX_ARTIFACT_BYTES {
            return Err(RetestError::Invalid(
                "Bayesian artifact is too large".into(),
            ));
        }
        Ok(bytes)
    }
    pub fn from_json_slice(bytes: &[u8]) -> Result<Self, RetestError> {
        if bytes.len() > MAX_ARTIFACT_BYTES {
            return Err(RetestError::Invalid(
                "Bayesian artifact is too large".into(),
            ));
        }
        let artifact: Self = serde_json::from_slice(bytes).map_err(invalid)?;
        artifact.verify()?;
        Ok(artifact)
    }
    pub fn verify(&self) -> Result<(), RetestError> {
        if self.schema_version != BAYESIAN_STUDY_SCHEMA_VERSION
            || !is_id(&self.source_dataset_id)
            || !is_id(&self.source_manifest_id)
            || !is_id(&self.config_id)
            || !matches!(
                self.source_stage,
                StageAccess::Search | StageAccess::Robustness
            )
            || self.range_start >= self.range_end
            || self.evaluations_n != self.spec.budget
            || self.decisions.len() != self.evaluations_n
            || self.observations.len() != self.evaluations_n
        {
            return Err(RetestError::Invalid(
                "invalid Bayesian artifact header".into(),
            ));
        }
        let base_strategy =
            StrategyIr::from_json_slice(&self.base_strategy_json).map_err(invalid)?;
        let domains = self
            .domains
            .iter()
            .map(|domain| ParameterDomain::new(&domain.id, domain.values.clone()))
            .collect::<Result<Vec<_>, _>>()?;
        let space = SearchSpace::new(base_strategy, domains)?;
        let spec = self.spec.public();
        validate_spec(&space, &spec)?;
        let lease = SearchDataLease::exact_partition(
            self.source_stage,
            self.source_dataset_id.clone(),
            self.range_start..self.range_end,
        )?;
        let seeded = seeded_design(&space, &spec)?;
        let mut seen_ordinals = BTreeSet::new();
        let mut seen_reports = BTreeSet::new();
        for index in 0..self.evaluations_n {
            let expected = proposal(
                &space,
                &spec,
                index,
                &seeded,
                &seen_ordinals,
                &self.decisions[..index],
                &self.observations[..index],
            )?;
            if self.decisions[index] != expected {
                return Err(RetestError::Invalid(
                    "Bayesian proposal/acquisition replay mismatch".into(),
                ));
            }
            let decision = &self.decisions[index];
            let observation = &self.observations[index];
            if !seen_ordinals.insert(decision.ordinal)
                || !seen_reports.insert(observation.report_id.as_str())
                || observation.evaluation_n != index + 1
                || observation.candidate_id != decision.candidate_id
                || !observation.value.is_finite()
            {
                return Err(RetestError::Invalid(
                    "duplicate or inconsistent Bayesian observation".into(),
                ));
            }
            let report = StrategyReportArtifact::from_json_slice(&observation.report_json)
                .map_err(invalid)?;
            if observation.run_id != report.run_id() || observation.report_id != report.report_id()
            {
                return Err(RetestError::Invalid(
                    "stored report identity mismatch".into(),
                ));
            }
            let candidate = instantiate(&space, decision.ordinal)?;
            let expected_seed = component_seed(spec.root_seed, index + 1, &candidate.candidate_id);
            if observation.component_seed != expected_seed {
                return Err(RetestError::Invalid("component seed mismatch".into()));
            }
            let request = RetestRequest::seal(
                &candidate.strategy,
                &lease,
                &self.config_id,
                METRICS_SCHEMA_VERSION,
                expected_seed,
            )?;
            if observation.request_id
                != execution_request_id(
                    &request,
                    &self.source_manifest_id,
                    ObservationRole::SearchEvaluation,
                    &spec.metric_id,
                )
            {
                return Err(RetestError::Invalid("retest request mismatch".into()));
            }
            verify_report_observation(
                observation,
                &report,
                &candidate,
                &self.source_dataset_id,
                &self.config_id,
                &spec.metric_id,
            )?;
        }
        if self.artifact_id != self.compute_id()? {
            return Err(RetestError::Invalid(
                "Bayesian artifact identity mismatch".into(),
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

pub fn execute_bayesian_optimization(
    config: &StrategyExecutionConfig,
    dataset: &DatasetManifest,
    bars: &[Bar],
    lease: SearchDataLease,
    space: &SearchSpace,
    spec: BayesianOptimizationSpec,
) -> Result<BayesianStudyArtifact, RetestError> {
    validate_spec(space, &spec)?;
    config.verify().map_err(invalid)?;
    dataset.verify(bars).map_err(invalid)?;
    if lease.dataset_id() != dataset.dataset_id
        || lease.range().len() != bars.len()
        || !matches!(lease.stage(), StageAccess::Search | StageAccess::Robustness)
    {
        return Err(RetestError::Invalid(
            "Bayesian study requires the exact non-holdout source lease".into(),
        ));
    }
    let seeded = seeded_design(space, &spec)?;
    let mut artifact = BayesianStudyArtifact {
        schema_version: BAYESIAN_STUDY_SCHEMA_VERSION,
        artifact_id: String::new(),
        source_dataset_id: dataset.dataset_id.clone(),
        source_manifest_id: dataset.manifest_id.clone(),
        source_stage: lease.stage(),
        config_id: config.config_id().to_string(),
        range_start: lease.range().start,
        range_end: lease.range().end,
        base_strategy_json: serde_json::to_vec(space.base()).map_err(invalid)?,
        domains: space
            .domains()
            .iter()
            .map(|domain| BayesianDomain {
                id: domain.id().to_string(),
                values: domain.values().to_vec(),
            })
            .collect(),
        spec: spec.clone().into(),
        evaluations_n: spec.budget,
        decisions: Vec::with_capacity(spec.budget),
        observations: Vec::with_capacity(spec.budget),
    };
    let mut seen = BTreeSet::new();
    for index in 0..spec.budget {
        let decision = proposal(
            space,
            &spec,
            index,
            &seeded,
            &seen,
            &artifact.decisions,
            &artifact.observations,
        )?;
        if !seen.insert(decision.ordinal) {
            return Err(RetestError::Invalid("duplicate Bayesian proposal".into()));
        }
        let candidate = instantiate(space, decision.ordinal)?;
        let seed = component_seed(spec.root_seed, index + 1, &candidate.candidate_id);
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
                "canonical observation disagrees with proposal".into(),
            ));
        }
        artifact.decisions.push(decision);
        artifact.observations.push(BayesianVerifiedObservation {
            evaluation_n: index + 1,
            candidate_id: candidate.candidate_id,
            request_id,
            run_id: report.run_id().to_string(),
            report_id: report.report_id().to_string(),
            component_seed: seed,
            value: if value == 0.0 { 0.0 } else { value },
            report_json: report.to_json_vec().map_err(invalid)?,
        });
    }
    artifact.artifact_id = artifact.compute_id()?;
    artifact.verify()?;
    let _ = artifact.to_json_vec()?;
    Ok(artifact)
}

fn validate_spec(space: &SearchSpace, spec: &BayesianOptimizationSpec) -> Result<(), RetestError> {
    let recorded_acquisitions = spec
        .budget
        .saturating_sub(spec.initial_design_size)
        .checked_mul(spec.acquisition_pool_limit.min(space.combinations()))
        .unwrap_or(usize::MAX);
    if spec.budget == 0
        || spec.budget > MAX_BAYESIAN_EVALUATIONS
        || spec.budget > space.combinations()
        || spec.initial_design_size < 2
        || spec.initial_design_size > spec.budget
        || spec.acquisition_pool_limit == 0
        || spec.acquisition_pool_limit > MAX_BAYESIAN_ACQUISITION_POOL
        || spec.nearest_neighbors == 0
        || spec.nearest_neighbors > MAX_BAYESIAN_NEIGHBOURS
        || recorded_acquisitions > MAX_BAYESIAN_RECORDED_ACQUISITIONS
        || spec.exploration_bps > 10_000
        || spec.metric_id.trim().is_empty()
    {
        return Err(RetestError::Optimization(
            OptimizationError::InvalidBudget { found: spec.budget },
        ));
    }
    Ok(())
}

fn seeded_design(
    space: &SearchSpace,
    spec: &BayesianOptimizationSpec,
) -> Result<Vec<Candidate>, RetestError> {
    Ok(generate_candidates(
        space,
        SearchMethod::LatinHypercube {
            seed: spec.root_seed,
        },
        spec.initial_design_size,
    )?
    .candidates)
}

fn proposal(
    space: &SearchSpace,
    spec: &BayesianOptimizationSpec,
    index: usize,
    seeded: &[Candidate],
    seen: &BTreeSet<usize>,
    decisions: &[BayesianProposalDecision],
    observations: &[BayesianVerifiedObservation],
) -> Result<BayesianProposalDecision, RetestError> {
    if index < seeded.len() {
        let candidate = &seeded[index];
        let ordinal = assignment_ordinal(space, &candidate.assignments)?;
        if seen.contains(&ordinal) {
            return Err(RetestError::Invalid("duplicate seeded design point".into()));
        }
        return Ok(BayesianProposalDecision {
            evaluation_n: index + 1,
            kind: BayesianProposalKind::SeededDesign,
            ordinal,
            candidate_id: candidate.candidate_id.clone(),
            assignments: candidate.assignments.clone(),
            acquisition_pool: vec![],
        });
    }
    if decisions.len() != observations.len() || observations.is_empty() {
        return Err(RetestError::Invalid(
            "undefined Bayesian model state".into(),
        ));
    }
    let ordinals = acquisition_pool_ordinals(space, spec, index + 1, seen)?;
    if ordinals.is_empty() {
        return Err(RetestError::Invalid(
            "Bayesian search space exhausted".into(),
        ));
    }
    let mut pool = ordinals
        .into_iter()
        .map(|ordinal| acquisition_score(space, spec, ordinal, decisions, observations))
        .collect::<Result<Vec<_>, _>>()?;
    pool.sort_by(|left, right| left.ordinal.cmp(&right.ordinal));
    let selected = pool
        .iter()
        .max_by(|left, right| {
            left.acquisition_value
                .total_cmp(&right.acquisition_value)
                .then_with(|| right.ordinal.cmp(&left.ordinal))
        })
        .ok_or_else(|| RetestError::Invalid("empty acquisition pool".into()))?;
    Ok(BayesianProposalDecision {
        evaluation_n: index + 1,
        kind: BayesianProposalKind::Acquisition,
        ordinal: selected.ordinal,
        candidate_id: selected.candidate_id.clone(),
        assignments: selected.assignments.clone(),
        acquisition_pool: pool,
    })
}

fn acquisition_pool_ordinals(
    space: &SearchSpace,
    spec: &BayesianOptimizationSpec,
    evaluation_n: usize,
    seen: &BTreeSet<usize>,
) -> Result<Vec<usize>, RetestError> {
    let remaining = space.combinations().saturating_sub(seen.len());
    if remaining == 0 {
        return Err(RetestError::Invalid(
            "Bayesian search space exhausted".into(),
        ));
    }
    let target = remaining.min(spec.acquisition_pool_limit);
    if remaining <= spec.acquisition_pool_limit {
        return Ok((0..space.combinations())
            .filter(|ordinal| !seen.contains(ordinal))
            .collect());
    }
    let mut rng =
        SplitMix64(spec.root_seed ^ (evaluation_n as u64).wrapping_mul(0xd6e8_feb8_6659_fd93));
    let mut selected = BTreeSet::new();
    for _ in 0..target.saturating_mul(16) {
        let ordinal = (rng.next() as usize) % space.combinations();
        if !seen.contains(&ordinal) {
            selected.insert(ordinal);
        }
        if selected.len() == target {
            break;
        }
    }
    if selected.len() < target {
        for ordinal in 0..space.combinations() {
            if !seen.contains(&ordinal) {
                selected.insert(ordinal);
            }
            if selected.len() == target {
                break;
            }
        }
    }
    Ok(selected.into_iter().collect())
}

fn acquisition_score(
    space: &SearchSpace,
    spec: &BayesianOptimizationSpec,
    ordinal: usize,
    decisions: &[BayesianProposalDecision],
    observations: &[BayesianVerifiedObservation],
) -> Result<BayesianAcquisitionScore, RetestError> {
    let candidate = instantiate(space, ordinal)?;
    let target_indices = ordinal_indices(space, ordinal);
    let mut neighbours = decisions
        .iter()
        .zip(observations)
        .map(|(decision, observation)| {
            (
                mixed_distance(space, &target_indices, decision.ordinal),
                decision.ordinal,
                observation.value,
            )
        })
        .collect::<Vec<_>>();
    neighbours.sort_by(|left, right| {
        left.0
            .total_cmp(&right.0)
            .then_with(|| left.1.cmp(&right.1))
    });
    neighbours.truncate(spec.nearest_neighbors.min(neighbours.len()));
    if neighbours.is_empty() || neighbours.iter().any(|entry| !entry.2.is_finite()) {
        return Err(RetestError::Invalid(
            "undefined surrogate observation".into(),
        ));
    }
    let mut weighted_sum = 0.0;
    let mut total_weight = 0.0;
    for (distance, _, value) in &neighbours {
        let weight = 1.0 / (distance + 1.0e-9);
        weighted_sum += weight * value;
        total_weight += weight;
    }
    let predicted = weighted_sum / total_weight;
    let nearest_distance = neighbours[0].0;
    let minimum = observations
        .iter()
        .map(|observation| observation.value)
        .min_by(f64::total_cmp)
        .ok_or_else(|| RetestError::Invalid("missing surrogate minimum".into()))?;
    let maximum = observations
        .iter()
        .map(|observation| observation.value)
        .max_by(f64::total_cmp)
        .ok_or_else(|| RetestError::Invalid("missing surrogate maximum".into()))?;
    let scale = (maximum - minimum).abs().max(1.0);
    let uncertainty = nearest_distance.sqrt() * scale;
    let exploitation = match spec.direction {
        ObjectiveDirection::Maximize => predicted,
        ObjectiveDirection::Minimize => -predicted,
    };
    let acquisition = exploitation + uncertainty * f64::from(spec.exploration_bps) / 10_000.0;
    if !predicted.is_finite() || !uncertainty.is_finite() || !acquisition.is_finite() {
        return Err(RetestError::Invalid("non-finite acquisition".into()));
    }
    Ok(BayesianAcquisitionScore {
        ordinal,
        candidate_id: candidate.candidate_id,
        assignments: candidate.assignments,
        predicted_value: canonical_zero(predicted),
        uncertainty: canonical_zero(uncertainty),
        acquisition_value: canonical_zero(acquisition),
    })
}

fn mixed_distance(space: &SearchSpace, target: &[usize], observed_ordinal: usize) -> f64 {
    let observed = ordinal_indices(space, observed_ordinal);
    target
        .iter()
        .zip(observed)
        .zip(space.domains())
        .map(|((left, right), domain)| {
            if domain.values().len() <= 1 {
                0.0
            } else {
                let delta = left.abs_diff(right) as f64 / (domain.values().len() - 1) as f64;
                delta * delta
            }
        })
        .sum::<f64>()
        / target.len() as f64
}

fn assignment_ordinal(
    space: &SearchSpace,
    assignments: &[(String, ParamValue)],
) -> Result<usize, RetestError> {
    if assignments.len() != space.domains().len() {
        return Err(RetestError::Invalid("assignment dimension mismatch".into()));
    }
    let mut ordinal = 0usize;
    for (domain, (id, value)) in space.domains().iter().zip(assignments) {
        if domain.id() != id {
            return Err(RetestError::Invalid("assignment ordering mismatch".into()));
        }
        let index = domain
            .values()
            .iter()
            .position(|candidate| candidate == value)
            .ok_or_else(|| RetestError::Invalid("assignment outside domain".into()))?;
        ordinal = ordinal
            .checked_mul(domain.values().len())
            .and_then(|value| value.checked_add(index))
            .ok_or_else(|| RetestError::Invalid("assignment ordinal overflow".into()))?;
    }
    Ok(ordinal)
}

fn verify_report_observation(
    observation: &BayesianVerifiedObservation,
    report: &StrategyReportArtifact,
    candidate: &Candidate,
    dataset_id: &str,
    config_id: &str,
    metric_id: &str,
) -> Result<(), RetestError> {
    let manifest = report
        .run_manifest()
        .ok_or_else(|| RetestError::Invalid("Bayesian report lacks run manifest".into()))?;
    let binding = manifest.binding();
    if binding.strategy_id != candidate.candidate_id
        || binding.config_id != config_id
        || binding.seed != observation.component_seed
        || binding.metrics_version != METRICS_SCHEMA_VERSION
        || !binding
            .datasets
            .iter()
            .any(|dataset| dataset.dataset_id == dataset_id)
    {
        return Err(RetestError::Invalid("foreign Bayesian report".into()));
    }
    match report.analysis().metric(metric_id) {
        Some(MetricValue::Defined { value })
            if value.is_finite()
                && canonical_zero(*value).to_bits() == observation.value.to_bits() =>
        {
            Ok(())
        }
        _ => Err(RetestError::Invalid(
            "undefined or altered Bayesian report metric".into(),
        )),
    }
}

fn component_seed(root_seed: u64, evaluation_n: usize, candidate_id: &str) -> u64 {
    let mut hasher = Sha256::new();
    hasher.update(COMPONENT_SEED_DOMAIN);
    hasher.update(root_seed.to_be_bytes());
    hasher.update((evaluation_n as u64).to_be_bytes());
    frame(&mut hasher, candidate_id.as_bytes());
    let digest = hasher.finalize();
    let mut prefix = [0_u8; 8];
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
