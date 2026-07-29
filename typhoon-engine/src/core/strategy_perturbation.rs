//! Deterministic non-trade perturbation robustness over the exact ADR-135 retest boundary (§7.3).
//!
//! Every trial is a *new* run: the perturbation rebuilds one of the three sealed inputs — the
//! strategy (parameter jitter inside its declared domains), the execution config (cost stress), or
//! the dataset (price noise, later start) — and then executes the rebuilt triple through the same
//! `VerifiedRun` → canonical simulator → sealed report path a plain retest uses. No metric is ever
//! accepted from a caller, no perturbation is applied to an already-sealed report, and the source
//! manifest and bars are read-only throughout.
//!
//! Chronological market data is never reordered or resampled: a data perturbation keeps every bar
//! at its own timestamp, and a start-date perturbation drops leading bars only. Both properties are
//! recorded per trial and re-checked during verification, so a stored study cannot claim a
//! perturbation that would have leaked a future bar into a decision.

use crate::broker::alpaca::Bar;
use crate::core::strategy_dataset::DatasetManifest;
use crate::core::strategy_ir::{
    CommissionModel, ParamValue, SlippageModel, SpreadModel, StrategyExecutionConfig, StrategyIr,
};
use crate::core::strategy_metrics::{METRICS_SCHEMA_VERSION, MetricValue};
use crate::core::strategy_optimization::{
    Candidate, MAX_ARTIFACT_BYTES, MAX_SEARCH_COMBINATIONS, MAX_TRIAL_BUDGET, ObservationRole,
    ParameterDomain, RetestRequest, SearchDataLease, SearchSpace, SplitMix64, StageAccess,
    instantiate, percentile_index,
};
use crate::core::strategy_report::StrategyReportArtifact;
use crate::core::strategy_retest::{
    RetestError, RetestExecutionRequest, execute_bound_observation, execution_request_id,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::ops::Range;

pub const PERTURBATION_STUDY_SCHEMA_VERSION: u32 = 1;
pub const MAX_PERTURBATION_TRIALS_PER_FAMILY: usize = 32;
pub const MAX_PERTURBATION_TRIALS: usize = 128;
pub const MAX_PERTURBATION_JITTER_STEPS: usize = 8;
pub const MAX_PERTURBATION_NEIGHBOURHOOD: usize = 4_096;
pub const MAX_PERTURBATION_COST_SCALE_BPS: u32 = 40_000;
pub const MAX_PERTURBATION_NOISE_BPS: u32 = 1_000;
pub const MAX_PERTURBATION_START_OFFSET: usize = 512;
/// Bars a start-shifted trial must still be able to lease.
const MIN_PERTURBED_BARS: usize = 2;
const CONFIDENCE_LEVEL_BPS: u32 = 9_000;
const ARTIFACT_DOMAIN: &[u8] = b"typhoon.strategy_perturbation.study.v1";
const FAMILY_SEED_DOMAIN: &[u8] = b"typhoon.strategy_perturbation.family_seed.v1";
const TRIAL_SEED_DOMAIN: &[u8] = b"typhoon.strategy_perturbation.trial_seed.v1";
/// The unperturbed reference observation on the leased range.
const BASELINE_ROLE: ObservationRole = ObservationRole::InSample;
/// Every perturbed trial: the same candidate re-checked against deliberately altered inputs.
const TRIAL_ROLE: ObservationRole = ObservationRole::CrossCheck;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PerturbationFamily {
    ParameterJitter,
    ExecutionCost,
    DataNoise,
    StartOffset,
}
impl PerturbationFamily {
    const ALL: [Self; 4] = [
        Self::ParameterJitter,
        Self::ExecutionCost,
        Self::DataNoise,
        Self::StartOffset,
    ];
    fn tag(self) -> &'static [u8] {
        match self {
            Self::ParameterJitter => b"parameter_jitter",
            Self::ExecutionCost => b"execution_cost",
            Self::DataNoise => b"data_noise",
            Self::StartOffset => b"start_offset",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PerturbationStudySpec {
    /// Trials executed per supported family. Every family draws without replacement, so a family
    /// whose perturbation space cannot supply this many distinct trials fails closed.
    pub trials_per_family: usize,
    /// Ordinal steps a jittered parameter may move along each declared domain axis.
    pub jitter_steps: usize,
    /// Upper bound of the additional execution cost, in basis points of the sealed cost model.
    pub cost_scale_bps: u32,
    /// Upper bound of the per-bar relative price noise, in basis points.
    pub data_noise_bps: u32,
    /// Upper bound of the leading bars a start-shifted trial may drop.
    pub maximum_start_offset: usize,
    pub metric_id: String,
    /// Size of the selection universe the baseline candidate was chosen from (§7.7).
    pub evaluations_n: usize,
    pub root_seed: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PerturbationPercentiles {
    confidence_level_bps: u32,
    p05: f64,
    median: f64,
    p95: f64,
}
impl PerturbationPercentiles {
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

/// The exact draw that made one trial different from the baseline. It is the family's uniqueness
/// key: two trials of one family may never record the same detail.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum PerturbationDetail {
    ParameterJitter {
        ordinal: usize,
        assignments: Vec<(String, ParamValue)>,
    },
    ExecutionCost {
        scale_bps: u32,
    },
    /// Per-bar noise is a bounded function of the trial's component seed, which is what separates
    /// one noise trial from another.
    DataNoise,
    StartOffset {
        offset: usize,
    },
}
impl PerturbationDetail {
    fn family(&self) -> PerturbationFamily {
        match self {
            Self::ParameterJitter { .. } => PerturbationFamily::ParameterJitter,
            Self::ExecutionCost { .. } => PerturbationFamily::ExecutionCost,
            Self::DataNoise => PerturbationFamily::DataNoise,
            Self::StartOffset { .. } => PerturbationFamily::StartOffset,
        }
    }
}

/// The value that separates one trial of a family from its siblings. Noise is a bounded function of
/// the trial seed, so the dataset it actually produced is what has to be unique.
fn trial_key(trial: &PerturbationTrial) -> Vec<u8> {
    match &trial.detail {
        PerturbationDetail::ParameterJitter { ordinal, .. } => ordinal.to_be_bytes().to_vec(),
        PerturbationDetail::ExecutionCost { scale_bps } => scale_bps.to_be_bytes().to_vec(),
        PerturbationDetail::DataNoise => trial.dataset_id.as_bytes().to_vec(),
        PerturbationDetail::StartOffset { offset } => offset.to_be_bytes().to_vec(),
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PerturbationTrial {
    pub trial_n: usize,
    pub component_seed: u64,
    pub detail: PerturbationDetail,
    pub strategy_id: String,
    pub config_id: String,
    pub dataset_id: String,
    pub manifest_id: String,
    pub range_start: usize,
    pub range_end: usize,
    /// Chronology of the exact bars this trial executed, as sealed by its own manifest.
    pub first_timestamp: String,
    pub last_timestamp: String,
    pub request_id: String,
    pub run_id: String,
    pub report_id: String,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PerturbationFamilyEvidence {
    family: PerturbationFamily,
    component_seed: u64,
    /// Why this family could not be perturbed at all — a zero-cost model cannot be cost-stressed.
    /// Re-derived during verification, so a stored study cannot claim or hide support.
    unsupported_reason: Option<String>,
    trials: Vec<PerturbationTrial>,
    percentiles: Option<PerturbationPercentiles>,
}
impl PerturbationFamilyEvidence {
    pub fn family(&self) -> PerturbationFamily {
        self.family
    }
    pub fn component_seed(&self) -> u64 {
        self.component_seed
    }
    pub fn unsupported_reason(&self) -> Option<&str> {
        self.unsupported_reason.as_deref()
    }
    pub fn trials(&self) -> &[PerturbationTrial] {
        &self.trials
    }
    pub fn percentiles(&self) -> Option<&PerturbationPercentiles> {
        self.percentiles.as_ref()
    }
}

/// Content-addressed perturbation evidence. Its only production constructor is
/// [`execute_perturbation_study`]; the baseline report it was measured against travels with it, so
/// the artifact can prove its own reference point without the dataset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PerturbationStudyArtifact {
    schema_version: u32,
    artifact_id: String,
    baseline_candidate_id: String,
    baseline_request_id: String,
    baseline_run_id: String,
    baseline_report_id: String,
    baseline_value: f64,
    baseline_report_json: Vec<u8>,
    source_dataset_id: String,
    source_manifest_id: String,
    source_stage: StageAccess,
    source_first_timestamp: String,
    source_last_timestamp: String,
    range_start: usize,
    range_end: usize,
    config_id: String,
    config_json: Vec<u8>,
    base_strategy_json: Vec<u8>,
    domains: Vec<PerturbationDomain>,
    spec: PerturbationStudySpec,
    families: Vec<PerturbationFamilyEvidence>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PerturbationDomain {
    id: String,
    values: Vec<ParamValue>,
}

impl PerturbationStudyArtifact {
    pub fn artifact_id(&self) -> &str {
        &self.artifact_id
    }
    pub fn baseline_candidate_id(&self) -> &str {
        &self.baseline_candidate_id
    }
    pub fn baseline_run_id(&self) -> &str {
        &self.baseline_run_id
    }
    pub fn baseline_report_id(&self) -> &str {
        &self.baseline_report_id
    }
    pub fn baseline_value(&self) -> f64 {
        self.baseline_value
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
    pub fn evaluations_n(&self) -> usize {
        self.spec.evaluations_n
    }
    pub fn root_seed(&self) -> u64 {
        self.spec.root_seed
    }
    pub fn spec(&self) -> &PerturbationStudySpec {
        &self.spec
    }
    pub fn families(&self) -> &[PerturbationFamilyEvidence] {
        &self.families
    }
    pub fn to_json_vec(&self) -> Result<Vec<u8>, RetestError> {
        self.verify()?;
        let bytes = serde_json::to_vec(self).map_err(invalid)?;
        if bytes.len() > MAX_ARTIFACT_BYTES {
            return Err(RetestError::Invalid(
                "perturbation artifact is too large".into(),
            ));
        }
        Ok(bytes)
    }
    pub fn from_json_slice(bytes: &[u8]) -> Result<Self, RetestError> {
        let artifact = Self::decode(bytes)?;
        artifact.verify()?;
        Ok(artifact)
    }
    /// Decode, re-seal, and verify. Test-only: it proves the structural invariants refuse edited
    /// evidence on their own, instead of only because the recorded digest stopped matching.
    #[cfg(test)]
    pub(crate) fn resealed_from_json(bytes: &[u8]) -> Result<Self, RetestError> {
        let mut artifact = Self::decode(bytes)?;
        artifact.artifact_id = artifact.compute_id()?;
        artifact.verify()?;
        Ok(artifact)
    }
    fn decode(bytes: &[u8]) -> Result<Self, RetestError> {
        if bytes.len() > MAX_ARTIFACT_BYTES {
            return Err(RetestError::Invalid(
                "perturbation artifact is too large".into(),
            ));
        }
        serde_json::from_slice(bytes).map_err(invalid)
    }
    pub fn verify(&self) -> Result<(), RetestError> {
        let total_trials = self
            .families
            .iter()
            .try_fold(0usize, |total, family| {
                total.checked_add(family.trials.len())
            })
            .ok_or_else(|| RetestError::Invalid("perturbation trial overflow".into()))?;
        if self.schema_version != PERTURBATION_STUDY_SCHEMA_VERSION
            || !is_id(&self.baseline_candidate_id)
            || !is_id(&self.baseline_request_id)
            || !is_id(&self.baseline_run_id)
            || !is_id(&self.baseline_report_id)
            || !is_id(&self.source_dataset_id)
            || !is_id(&self.source_manifest_id)
            || !is_id(&self.config_id)
            || self.range_start >= self.range_end
            || !self.baseline_value.is_finite()
            || self.families.len() != PerturbationFamily::ALL.len()
            // A study in which nothing was perturbed is not evidence.
            || total_trials == 0
            || total_trials > MAX_PERTURBATION_TRIALS
        {
            return Err(RetestError::Invalid(
                "invalid perturbation artifact header".into(),
            ));
        }
        // A robustness lease is the only stage a perturbation study is ever granted, and it can
        // never name the final holdout.
        if self.source_stage != StageAccess::Robustness {
            return Err(RetestError::Invalid(
                "perturbation study requires a robustness lease".into(),
            ));
        }
        let source_first = parse_instant(&self.source_first_timestamp)?;
        let source_last = parse_instant(&self.source_last_timestamp)?;
        if source_first > source_last {
            return Err(RetestError::Invalid(
                "non-chronological source evidence".into(),
            ));
        }
        validate_spec(&self.spec)?;
        let space = self.space()?;
        let config = StrategyExecutionConfig::from_json_slice(&self.config_json)
            .map_err(invalid)
            .and_then(|config| {
                if config.config_id() == self.config_id {
                    Ok(config)
                } else {
                    Err(RetestError::Invalid(
                        "stored execution config identity mismatch".into(),
                    ))
                }
            })?;
        let baseline = self.baseline_candidate(&space)?;
        self.verify_baseline(&baseline)?;

        let bar_count = self.range_end - self.range_start;
        validate_start_budget(bar_count, &self.spec)?;
        let neighbourhood = jitter_neighbourhood(&space, &self.spec)?;
        let mut identities = BTreeSet::new();
        identities.insert(self.baseline_report_id.clone());
        identities.insert(self.baseline_run_id.clone());
        identities.insert(self.baseline_request_id.clone());
        for (expected_family, evidence) in PerturbationFamily::ALL.iter().zip(&self.families) {
            let component_seed = self.family_seed(*expected_family);
            let support = family_support(*expected_family, &config);
            if evidence.family != *expected_family
                || evidence.component_seed != component_seed
                || evidence.unsupported_reason.as_deref() != support
            {
                return Err(RetestError::Invalid(
                    "perturbation family evidence disagrees with its declared support".into(),
                ));
            }
            if support.is_some() {
                if !evidence.trials.is_empty() || evidence.percentiles.is_some() {
                    return Err(RetestError::Invalid(
                        "unsupported perturbation family carries trial evidence".into(),
                    ));
                }
                continue;
            }
            if evidence.trials.len() != self.spec.trials_per_family
                || evidence.percentiles != Some(summarize(&trial_values(&evidence.trials))?)
            {
                return Err(RetestError::Invalid(
                    "perturbation family distribution disagrees with its trials".into(),
                ));
            }
            let mut keys = BTreeSet::new();
            for (index, trial) in evidence.trials.iter().enumerate() {
                if trial.trial_n != index + 1
                    || trial.component_seed != trial_seed(component_seed, trial.trial_n)
                    || trial.detail.family() != *expected_family
                    || !trial.value.is_finite()
                    || !keys.insert(trial_key(trial))
                    || !identities.insert(trial.request_id.clone())
                    || !identities.insert(trial.run_id.clone())
                    || !identities.insert(trial.report_id.clone())
                {
                    return Err(RetestError::Invalid(
                        "duplicate or inconsistent perturbation trial".into(),
                    ));
                }
                self.verify_trial(
                    trial,
                    &space,
                    &config,
                    &baseline,
                    &neighbourhood,
                    (source_first, source_last),
                )?;
            }
        }
        if self.artifact_id != self.compute_id()? {
            return Err(RetestError::Invalid(
                "perturbation artifact identity mismatch".into(),
            ));
        }
        Ok(())
    }

    fn space(&self) -> Result<SearchSpace, RetestError> {
        let base = StrategyIr::from_json_slice(&self.base_strategy_json).map_err(invalid)?;
        let domains = self
            .domains
            .iter()
            .map(|domain| ParameterDomain::new(&domain.id, domain.values.clone()))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(SearchSpace::new(base, domains)?)
    }

    /// The unperturbed candidate: the declared strategy, re-instantiated at its own declared point
    /// inside the search space, so every jitter trial is a move away from a proven centre.
    fn baseline_candidate(&self, space: &SearchSpace) -> Result<Candidate, RetestError> {
        let candidate = instantiate(space, base_ordinal(space)?)?;
        if candidate.candidate_id != space.base().strategy_id()
            || candidate.candidate_id != self.baseline_candidate_id
        {
            return Err(RetestError::Invalid(
                "baseline candidate is not the declared strategy".into(),
            ));
        }
        Ok(candidate)
    }

    fn verify_baseline(&self, baseline: &Candidate) -> Result<(), RetestError> {
        let request = RetestRequest::seal(
            &baseline.strategy,
            &self.source_lease(self.source_dataset_id.clone(), self.range())?,
            &self.config_id,
            METRICS_SCHEMA_VERSION,
            self.spec.root_seed,
        )?;
        if self.baseline_request_id
            != execution_request_id(
                &request,
                &self.source_manifest_id,
                BASELINE_ROLE,
                &self.spec.metric_id,
            )
        {
            return Err(RetestError::Invalid(
                "baseline retest request mismatch".into(),
            ));
        }
        let report =
            StrategyReportArtifact::from_json_slice(&self.baseline_report_json).map_err(invalid)?;
        if report.run_id() != self.baseline_run_id || report.report_id() != self.baseline_report_id
        {
            return Err(RetestError::Invalid(
                "stored baseline report identity mismatch".into(),
            ));
        }
        let manifest = report
            .run_manifest()
            .ok_or_else(|| RetestError::Invalid("baseline report lacks run manifest".into()))?;
        let binding = manifest.binding();
        if binding.strategy_id != self.baseline_candidate_id
            || binding.config_id != self.config_id
            || binding.seed != self.spec.root_seed
            || binding.metrics_version != METRICS_SCHEMA_VERSION
            || !binding
                .datasets
                .iter()
                .any(|dataset| dataset.dataset_id == self.source_dataset_id)
        {
            return Err(RetestError::Invalid("foreign baseline report".into()));
        }
        match report.analysis().metric(&self.spec.metric_id) {
            Some(MetricValue::Defined { value })
                if value.is_finite()
                    && canonical_zero(*value).to_bits() == self.baseline_value.to_bits() =>
            {
                Ok(())
            }
            _ => Err(RetestError::Invalid(
                "undefined or altered baseline metric".into(),
            )),
        }
    }

    fn verify_trial(
        &self,
        trial: &PerturbationTrial,
        space: &SearchSpace,
        config: &StrategyExecutionConfig,
        baseline: &Candidate,
        neighbourhood: &[usize],
        source_bounds: (chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>),
    ) -> Result<(), RetestError> {
        if !is_id(&trial.strategy_id)
            || !is_id(&trial.config_id)
            || !is_id(&trial.dataset_id)
            || !is_id(&trial.manifest_id)
            || !is_id(&trial.request_id)
            || !is_id(&trial.run_id)
            || !is_id(&trial.report_id)
            || trial.range_start >= trial.range_end
            || trial.range_end - trial.range_start < MIN_PERTURBED_BARS
        {
            return Err(RetestError::Invalid("invalid perturbation trial".into()));
        }
        let (first, last) = (
            parse_instant(&trial.first_timestamp)?,
            parse_instant(&trial.last_timestamp)?,
        );
        // No perturbation may extend past the leased content, and none may end anywhere other than
        // where the source ended: a trial can only ever see less of the future, never more.
        if first < source_bounds.0
            || last != source_bounds.1
            || trial.range_end != self.range_end
            || trial.range_start < self.range_start
        {
            return Err(RetestError::Invalid(
                "perturbation trial reaches outside its leased chronology".into(),
            ));
        }
        let strategy = match &trial.detail {
            PerturbationDetail::ParameterJitter {
                ordinal,
                assignments,
            } => {
                if !neighbourhood.contains(ordinal) {
                    return Err(RetestError::Invalid(
                        "jittered parameters fall outside the declared neighbourhood".into(),
                    ));
                }
                let candidate = instantiate(space, *ordinal)?;
                if candidate.candidate_id != trial.strategy_id
                    || &candidate.assignments != assignments
                {
                    return Err(RetestError::Invalid(
                        "jittered candidate identity mismatch".into(),
                    ));
                }
                candidate.strategy
            }
            _ => {
                if trial.strategy_id != baseline.candidate_id {
                    return Err(RetestError::Invalid(
                        "non-parameter perturbation altered the candidate".into(),
                    ));
                }
                baseline.strategy.clone()
            }
        };
        match &trial.detail {
            PerturbationDetail::ExecutionCost { scale_bps } => {
                if *scale_bps == 0 || *scale_bps > self.spec.cost_scale_bps {
                    return Err(RetestError::Invalid(
                        "execution cost scale outside its declared bound".into(),
                    ));
                }
                if trial.config_id != stressed_config(config, *scale_bps)?.config_id() {
                    return Err(RetestError::Invalid(
                        "stressed execution config identity mismatch".into(),
                    ));
                }
            }
            _ if trial.config_id != self.config_id => {
                return Err(RetestError::Invalid(
                    "non-cost perturbation altered the execution config".into(),
                ));
            }
            _ => {}
        }
        match &trial.detail {
            PerturbationDetail::ParameterJitter { .. }
            | PerturbationDetail::ExecutionCost { .. } => {
                if trial.dataset_id != self.source_dataset_id
                    || trial.manifest_id != self.source_manifest_id
                    || trial.range_start != self.range_start
                    || trial.first_timestamp != self.source_first_timestamp
                    || trial.last_timestamp != self.source_last_timestamp
                {
                    return Err(RetestError::Invalid(
                        "non-data perturbation altered the leased dataset".into(),
                    ));
                }
            }
            PerturbationDetail::DataNoise => {
                // Noise re-prices bars in place: same count, same timestamps, new content.
                if trial.dataset_id == self.source_dataset_id
                    || trial.range_start != self.range_start
                    || trial.first_timestamp != self.source_first_timestamp
                    || trial.last_timestamp != self.source_last_timestamp
                {
                    return Err(RetestError::Invalid(
                        "price noise perturbation did not preserve the source chronology".into(),
                    ));
                }
            }
            PerturbationDetail::StartOffset { offset } => {
                if *offset == 0
                    || *offset > self.spec.maximum_start_offset
                    || trial.dataset_id == self.source_dataset_id
                    || trial.range_start != self.range_start + offset
                    || first <= source_bounds.0
                    || trial.last_timestamp != self.source_last_timestamp
                {
                    return Err(RetestError::Invalid(
                        "start offset perturbation did not drop leading bars only".into(),
                    ));
                }
            }
        }
        let request = RetestRequest::seal(
            &strategy,
            &self.source_lease(trial.dataset_id.clone(), trial.range_start..trial.range_end)?,
            &trial.config_id,
            METRICS_SCHEMA_VERSION,
            trial.component_seed,
        )?;
        if trial.request_id
            != execution_request_id(
                &request,
                &trial.manifest_id,
                TRIAL_ROLE,
                &self.spec.metric_id,
            )
        {
            return Err(RetestError::Invalid(
                "perturbation trial request mismatch".into(),
            ));
        }
        Ok(())
    }

    fn source_lease(
        &self,
        dataset_id: String,
        range: Range<usize>,
    ) -> Result<SearchDataLease, RetestError> {
        Ok(SearchDataLease::exact_partition(
            self.source_stage,
            dataset_id,
            range,
        )?)
    }

    fn family_seed(&self, family: PerturbationFamily) -> u64 {
        let mut hasher = Sha256::new();
        hasher.update(FAMILY_SEED_DOMAIN);
        hasher.update(self.spec.root_seed.to_be_bytes());
        for value in [
            self.baseline_candidate_id.as_bytes(),
            self.baseline_report_id.as_bytes(),
            self.source_dataset_id.as_bytes(),
            self.source_manifest_id.as_bytes(),
            self.config_id.as_bytes(),
            self.spec.metric_id.as_bytes(),
            family.tag(),
        ] {
            frame(&mut hasher, value);
        }
        digest_prefix(hasher)
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

/// Execute every supported perturbation family against the exact leased content.
///
/// `bars` must be the immutable payload `lease` admits and `dataset` seals; both are read-only, and
/// each perturbation rebuilds its own strategy, config and manifest before executing the canonical
/// bound observation path.
pub fn execute_perturbation_study(
    config: &StrategyExecutionConfig,
    dataset: &DatasetManifest,
    bars: &[Bar],
    lease: SearchDataLease,
    space: &SearchSpace,
    spec: PerturbationStudySpec,
) -> Result<PerturbationStudyArtifact, RetestError> {
    validate_spec(&spec)?;
    config.verify().map_err(invalid)?;
    dataset.verify(bars).map_err(invalid)?;
    if lease.stage() != StageAccess::Robustness
        || lease.dataset_id() != dataset.dataset_id
        || lease.range().len() != bars.len()
    {
        return Err(RetestError::Invalid(
            "perturbation study requires the exact leased robustness content".into(),
        ));
    }
    if bars.iter().any(|bar| {
        [bar.open, bar.high, bar.low, bar.close]
            .iter()
            .any(|price| !price.is_finite() || *price <= 0.0)
            || !bar.volume.is_finite()
    }) {
        return Err(RetestError::Invalid(
            "perturbation study requires finite positive source prices".into(),
        ));
    }
    validate_start_budget(bars.len(), &spec)?;
    let total_trials = spec
        .trials_per_family
        .checked_mul(PerturbationFamily::ALL.len())
        .filter(|total| *total <= MAX_PERTURBATION_TRIALS)
        .ok_or_else(|| RetestError::Invalid("perturbation trial budget overflow".into()))?;
    if total_trials
        .checked_mul(bars.len())
        .is_none_or(|work| work > MAX_SEARCH_COMBINATIONS)
    {
        return Err(RetestError::Invalid("perturbation work overflow".into()));
    }
    let (first_timestamp, last_timestamp) =
        match (&dataset.first_timestamp, &dataset.last_timestamp) {
            (Some(first), Some(last)) => (first.clone(), last.clone()),
            _ => {
                return Err(RetestError::Invalid(
                    "leased dataset has no chronology".into(),
                ));
            }
        };
    let baseline = instantiate(space, base_ordinal(space)?)?;
    if baseline.candidate_id != space.base().strategy_id() {
        return Err(RetestError::Invalid(
            "baseline candidate is not the declared strategy".into(),
        ));
    }
    let range = lease.range();
    let baseline_request = RetestExecutionRequest::seal(
        &baseline.strategy,
        config,
        dataset,
        bars,
        SearchDataLease::exact_partition(lease.stage(), dataset.dataset_id.clone(), range.clone())?,
        BASELINE_ROLE,
        &spec.metric_id,
        spec.root_seed,
    )?;
    let baseline_request_id = baseline_request.request_id().to_string();
    let (baseline_report, baseline_observation, baseline_value) =
        execute_bound_observation(&baseline_request)?;
    if baseline_observation.candidate_id() != baseline.candidate_id || !baseline_value.is_finite() {
        return Err(RetestError::Invalid(
            "canonical baseline observation disagrees with the declared candidate".into(),
        ));
    }
    let mut artifact = PerturbationStudyArtifact {
        schema_version: PERTURBATION_STUDY_SCHEMA_VERSION,
        artifact_id: String::new(),
        baseline_candidate_id: baseline.candidate_id.clone(),
        baseline_request_id,
        baseline_run_id: baseline_report.run_id().to_string(),
        baseline_report_id: baseline_report.report_id().to_string(),
        baseline_value: canonical_zero(baseline_value),
        baseline_report_json: baseline_report.to_json_vec().map_err(invalid)?,
        source_dataset_id: dataset.dataset_id.clone(),
        source_manifest_id: dataset.manifest_id.clone(),
        source_stage: lease.stage(),
        source_first_timestamp: first_timestamp,
        source_last_timestamp: last_timestamp,
        range_start: range.start,
        range_end: range.end,
        config_id: config.config_id().to_string(),
        config_json: serde_json::to_vec(config).map_err(invalid)?,
        base_strategy_json: serde_json::to_vec(space.base()).map_err(invalid)?,
        domains: space
            .domains()
            .iter()
            .map(|domain| PerturbationDomain {
                id: domain.id().to_string(),
                values: domain.values().to_vec(),
            })
            .collect(),
        spec,
        families: Vec::with_capacity(PerturbationFamily::ALL.len()),
    };
    let neighbourhood = jitter_neighbourhood(space, &artifact.spec)?;
    for family in PerturbationFamily::ALL {
        let component_seed = artifact.family_seed(family);
        let evidence = match family_support(family, config) {
            Some(reason) => PerturbationFamilyEvidence {
                family,
                component_seed,
                unsupported_reason: Some(reason.to_string()),
                trials: vec![],
                percentiles: None,
            },
            None => {
                let trials = execute_family(
                    family,
                    component_seed,
                    &neighbourhood,
                    &artifact,
                    space,
                    config,
                    dataset,
                    bars,
                    &baseline,
                )?;
                PerturbationFamilyEvidence {
                    family,
                    component_seed,
                    unsupported_reason: None,
                    percentiles: Some(summarize(&trial_values(&trials))?),
                    trials,
                }
            }
        };
        artifact.families.push(evidence);
    }
    artifact.artifact_id = artifact.compute_id()?;
    artifact.verify()?;
    let _ = artifact.to_json_vec()?;
    Ok(artifact)
}

/// Re-execute a stored study from the same source evidence and refuse anything that is not the
/// artifact it claims to be.
pub fn replay_perturbation_study(
    config: &StrategyExecutionConfig,
    dataset: &DatasetManifest,
    bars: &[Bar],
    lease: SearchDataLease,
    space: &SearchSpace,
    expected: &PerturbationStudyArtifact,
) -> Result<PerturbationStudyArtifact, RetestError> {
    expected.verify()?;
    let replayed =
        execute_perturbation_study(config, dataset, bars, lease, space, expected.spec.clone())?;
    if &replayed != expected {
        return Err(RetestError::Invalid(
            "foreign or non-deterministic perturbation evidence".into(),
        ));
    }
    Ok(replayed)
}

#[allow(clippy::too_many_arguments)]
fn execute_family(
    family: PerturbationFamily,
    component_seed: u64,
    neighbourhood: &[usize],
    artifact: &PerturbationStudyArtifact,
    space: &SearchSpace,
    config: &StrategyExecutionConfig,
    dataset: &DatasetManifest,
    bars: &[Bar],
    baseline: &Candidate,
) -> Result<Vec<PerturbationTrial>, RetestError> {
    let spec = &artifact.spec;
    let draws = match family {
        PerturbationFamily::ParameterJitter => {
            distinct_draws(component_seed, spec.trials_per_family, neighbourhood.len())?
        }
        PerturbationFamily::ExecutionCost => distinct_draws(
            component_seed,
            spec.trials_per_family,
            spec.cost_scale_bps as usize,
        )?,
        PerturbationFamily::DataNoise => (0..spec.trials_per_family).collect(),
        PerturbationFamily::StartOffset => distinct_draws(
            component_seed,
            spec.trials_per_family,
            spec.maximum_start_offset,
        )?,
    };
    let mut trials = Vec::with_capacity(spec.trials_per_family);
    for (index, draw) in draws.into_iter().enumerate() {
        let trial_n = index + 1;
        let seed = trial_seed(component_seed, trial_n);
        let (detail, strategy, trial_config, trial_bars) = match family {
            PerturbationFamily::ParameterJitter => {
                let ordinal = neighbourhood[draw];
                let candidate = instantiate(space, ordinal)?;
                (
                    PerturbationDetail::ParameterJitter {
                        ordinal,
                        assignments: candidate.assignments,
                    },
                    candidate.strategy,
                    config.clone(),
                    None,
                )
            }
            PerturbationFamily::ExecutionCost => {
                let scale_bps = u32::try_from(draw + 1)
                    .map_err(|_| RetestError::Invalid("cost scale overflow".into()))?;
                (
                    PerturbationDetail::ExecutionCost { scale_bps },
                    baseline.strategy.clone(),
                    stressed_config(config, scale_bps)?,
                    None,
                )
            }
            PerturbationFamily::DataNoise => (
                PerturbationDetail::DataNoise,
                baseline.strategy.clone(),
                config.clone(),
                Some((0, noised_bars(bars, seed, spec.data_noise_bps)?)),
            ),
            PerturbationFamily::StartOffset => {
                let offset = draw + 1;
                let remainder = bars
                    .get(offset..)
                    .filter(|rest| rest.len() >= MIN_PERTURBED_BARS)
                    .ok_or_else(|| {
                        RetestError::Invalid("start offset consumes the leased content".into())
                    })?;
                (
                    PerturbationDetail::StartOffset { offset },
                    baseline.strategy.clone(),
                    config.clone(),
                    Some((offset, remainder.to_vec())),
                )
            }
        };
        let (offset, trial_manifest, payload) = match &trial_bars {
            Some((offset, perturbed)) => (
                *offset,
                DatasetManifest::build(&dataset.to_input(), perturbed).map_err(invalid)?,
                perturbed.as_slice(),
            ),
            None => (0, dataset.clone(), bars),
        };
        let range = artifact.range_start + offset..artifact.range_end;
        let request = RetestExecutionRequest::seal(
            &strategy,
            &trial_config,
            &trial_manifest,
            payload,
            SearchDataLease::exact_partition(
                artifact.source_stage,
                trial_manifest.dataset_id.clone(),
                range.clone(),
            )?,
            TRIAL_ROLE,
            &spec.metric_id,
            seed,
        )?;
        let request_id = request.request_id().to_string();
        let (report, observation, value) = execute_bound_observation(&request)?;
        if observation.candidate_id() != strategy.strategy_id() || !value.is_finite() {
            return Err(RetestError::Invalid(
                "canonical perturbation observation disagrees with its trial".into(),
            ));
        }
        let (first_timestamp, last_timestamp) = match (
            &trial_manifest.first_timestamp,
            &trial_manifest.last_timestamp,
        ) {
            (Some(first), Some(last)) => (first.clone(), last.clone()),
            _ => {
                return Err(RetestError::Invalid(
                    "perturbed dataset has no chronology".into(),
                ));
            }
        };
        trials.push(PerturbationTrial {
            trial_n,
            component_seed: seed,
            detail,
            strategy_id: strategy.strategy_id().to_string(),
            config_id: trial_config.config_id().to_string(),
            dataset_id: trial_manifest.dataset_id.clone(),
            manifest_id: trial_manifest.manifest_id.clone(),
            range_start: range.start,
            range_end: range.end,
            first_timestamp,
            last_timestamp,
            request_id,
            run_id: report.run_id().to_string(),
            report_id: report.report_id().to_string(),
            value: canonical_zero(value),
        });
    }
    Ok(trials)
}

fn validate_spec(spec: &PerturbationStudySpec) -> Result<(), RetestError> {
    if spec.trials_per_family == 0
        || spec.trials_per_family > MAX_PERTURBATION_TRIALS_PER_FAMILY
        || spec
            .trials_per_family
            .checked_mul(PerturbationFamily::ALL.len())
            .is_none_or(|total| total > MAX_PERTURBATION_TRIALS)
        || spec.jitter_steps == 0
        || spec.jitter_steps > MAX_PERTURBATION_JITTER_STEPS
        || spec.cost_scale_bps == 0
        || spec.cost_scale_bps > MAX_PERTURBATION_COST_SCALE_BPS
        || spec.data_noise_bps == 0
        || spec.data_noise_bps > MAX_PERTURBATION_NOISE_BPS
        || spec.maximum_start_offset == 0
        || spec.maximum_start_offset > MAX_PERTURBATION_START_OFFSET
        || spec.metric_id.trim().is_empty()
        || spec.evaluations_n == 0
        || spec.evaluations_n > MAX_TRIAL_BUDGET
    {
        return Err(RetestError::Invalid(
            "invalid perturbation study specification".into(),
        ));
    }
    Ok(())
}

/// The deepest start shift must still leave a leasable series behind.
fn validate_start_budget(
    bar_count: usize,
    spec: &PerturbationStudySpec,
) -> Result<(), RetestError> {
    if spec
        .maximum_start_offset
        .checked_add(MIN_PERTURBED_BARS)
        .is_none_or(|needed| needed > bar_count)
    {
        return Err(RetestError::Invalid(
            "start offset budget exceeds the leased content".into(),
        ));
    }
    Ok(())
}

/// Why `family` cannot be perturbed for this baseline, or `None` when it can.
///
/// A family whose *specification* is impossible — too few neighbourhood points, too few start
/// offsets — is a caller error and fails the whole study closed elsewhere. The one genuine
/// non-support is a cost model with nothing positive to scale.
fn family_support(
    family: PerturbationFamily,
    config: &StrategyExecutionConfig,
) -> Option<&'static str> {
    match family {
        PerturbationFamily::ExecutionCost if !has_scalable_cost(config) => {
            Some("sealed execution config declares no positive slippage or spread cost to stress")
        }
        _ => None,
    }
}

pub(crate) fn has_scalable_cost(config: &StrategyExecutionConfig) -> bool {
    let settings = config.settings();
    let slippage = match &settings.slippage {
        SlippageModel::None => 0.0,
        SlippageModel::FixedPriceDistance { distance } => *distance,
        SlippageModel::SpreadFraction { fraction } => *fraction,
        SlippageModel::VolatilityScaled { atr_fraction } => *atr_fraction,
    };
    let spread = match &settings.spread {
        SpreadModel::None | SpreadModel::RecordedQuotes => 0.0,
        SpreadModel::Constant { price_units } => *price_units,
        SpreadModel::PercentOfPrice { percent } => *percent,
    };
    let commission = match &settings.commission {
        CommissionModel::None | CommissionModel::VenueSchedule(_) => 0.0,
        CommissionModel::PerShare { amount, minimum }
        | CommissionModel::PercentOfNotional {
            percent: amount,
            minimum,
        } => amount.max(*minimum),
        CommissionModel::PerOrder { amount } => *amount,
    };
    (slippage.is_finite() && slippage > 0.0)
        || (spread.is_finite() && spread > 0.0)
        || (commission.is_finite() && commission > 0.0)
}

/// Rebuild the sealed execution config with every positive slippage, spread, and configurable
/// commission scalar scaled by `scale_bps`. The rebuilt config is validated and re-identified by
/// `StrategyExecutionConfig`, so a stress that the declared model cannot represent fails closed
/// instead of being clamped. Named venue schedules remain immutable because their sealed version
/// does not declare synthetic multiplier rates.
pub(crate) fn stressed_config(
    config: &StrategyExecutionConfig,
    scale_bps: u32,
) -> Result<StrategyExecutionConfig, RetestError> {
    if scale_bps == 0 {
        return Err(RetestError::Invalid(
            "execution cost stress must change the sealed cost model".into(),
        ));
    }
    let factor = 1.0 + f64::from(scale_bps) / 10_000.0;
    let mut settings = config.to_input();
    let mut stressed = false;
    let mut scale = |value: &mut f64| -> Result<(), RetestError> {
        if value.is_finite() && *value > 0.0 {
            let scaled = *value * factor;
            if !scaled.is_finite() {
                return Err(RetestError::Invalid("stressed cost is not finite".into()));
            }
            *value = scaled;
            stressed = true;
        }
        Ok(())
    };
    match &mut settings.slippage {
        SlippageModel::None => {}
        SlippageModel::FixedPriceDistance { distance } => scale(distance)?,
        SlippageModel::SpreadFraction { fraction } => scale(fraction)?,
        SlippageModel::VolatilityScaled { atr_fraction } => scale(atr_fraction)?,
    }
    match &mut settings.spread {
        SpreadModel::None | SpreadModel::RecordedQuotes => {}
        SpreadModel::Constant { price_units } => scale(price_units)?,
        SpreadModel::PercentOfPrice { percent } => scale(percent)?,
    }
    match &mut settings.commission {
        CommissionModel::None => {}
        CommissionModel::PerShare { amount, minimum }
        | CommissionModel::PercentOfNotional {
            percent: amount,
            minimum,
        } => {
            scale(amount)?;
            scale(minimum)?;
        }
        CommissionModel::PerOrder { amount } => scale(amount)?,
        // Venue schedules are immutable external tariffs. Scaling a named schedule would fabricate
        // rates that its version never declared, so callers need an explicit configurable model.
        CommissionModel::VenueSchedule(_) => {}
    }
    if !stressed {
        return Err(RetestError::Invalid(
            "sealed execution config has no positive cost to stress".into(),
        ));
    }
    StrategyExecutionConfig::build(&settings).map_err(invalid)
}

/// Re-price every bar by a bounded per-bar relative draw. Timestamps, volumes, bar count and bar
/// order are untouched, and the intrabar open/high/low/close geometry is preserved exactly, so the
/// perturbed series cannot become an impossible bar or move a decision earlier in time.
fn noised_bars(bars: &[Bar], seed: u64, noise_bps: u32) -> Result<Vec<Bar>, RetestError> {
    if noise_bps == 0 || noise_bps >= 10_000 {
        return Err(RetestError::Invalid("invalid price noise bound".into()));
    }
    let mut rng = SplitMix64(seed);
    let span = u64::from(noise_bps) * 2 + 1;
    let mut perturbed = Vec::with_capacity(bars.len());
    for bar in bars {
        let delta = (rng.next() % span) as i64 - i64::from(noise_bps);
        let factor = 1.0 + delta as f64 / 10_000.0;
        let repriced = Bar {
            timestamp: bar.timestamp.clone(),
            open: bar.open * factor,
            high: bar.high * factor,
            low: bar.low * factor,
            close: bar.close * factor,
            volume: bar.volume,
        };
        if [repriced.open, repriced.high, repriced.low, repriced.close]
            .iter()
            .any(|price| !price.is_finite() || *price <= 0.0)
        {
            return Err(RetestError::Invalid(
                "price noise produced an unusable bar".into(),
            ));
        }
        perturbed.push(repriced);
    }
    Ok(perturbed)
}

/// Every ordinal within `jitter_steps` of the baseline point on each declared axis, excluding the
/// baseline itself. Enumeration is bounded, so an oversized neighbourhood fails closed rather than
/// silently sampling a sub-region.
fn jitter_neighbourhood(
    space: &SearchSpace,
    spec: &PerturbationStudySpec,
) -> Result<Vec<usize>, RetestError> {
    let base_indices = base_indices(space)?;
    let windows = space
        .domains()
        .iter()
        .zip(&base_indices)
        .map(|(domain, base)| {
            let last = domain.values().len() - 1;
            (base.saturating_sub(spec.jitter_steps)..=(base + spec.jitter_steps).min(last))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let total = windows
        .iter()
        .try_fold(1usize, |total, window| total.checked_mul(window.len()))
        .filter(|total| *total <= MAX_PERTURBATION_NEIGHBOURHOOD)
        .ok_or_else(|| {
            RetestError::Invalid("parameter jitter neighbourhood is unbounded".into())
        })?;
    let centre = indices_ordinal(space, &base_indices);
    let mut ordinals = Vec::with_capacity(total);
    for counter in 0..total {
        let mut rest = counter;
        let mut indices = vec![0usize; windows.len()];
        for axis in (0..windows.len()).rev() {
            indices[axis] = windows[axis][rest % windows[axis].len()];
            rest /= windows[axis].len();
        }
        let ordinal = indices_ordinal(space, &indices);
        if ordinal != centre {
            ordinals.push(ordinal);
        }
    }
    ordinals.sort_unstable();
    ordinals.dedup();
    if ordinals.len() < spec.trials_per_family {
        return Err(RetestError::Invalid(
            "parameter jitter neighbourhood is smaller than the trial budget".into(),
        ));
    }
    Ok(ordinals)
}

/// The baseline's own index on each declared axis. A declared value with no place in its own domain
/// has no neighbourhood, so the study refuses to invent one.
fn base_indices(space: &SearchSpace) -> Result<Vec<usize>, RetestError> {
    let parameters = &space.base().definition().parameters;
    space
        .domains()
        .iter()
        .map(|domain| {
            let declared = parameters
                .iter()
                .find(|parameter| parameter.id == domain.id())
                .ok_or_else(|| {
                    RetestError::Invalid("declared domain names no strategy parameter".into())
                })?;
            domain
                .values()
                .iter()
                .position(|value| value == &declared.value)
                .ok_or_else(|| {
                    RetestError::Invalid(
                        "declared parameter value is outside its own declared domain".into(),
                    )
                })
        })
        .collect()
}

fn base_ordinal(space: &SearchSpace) -> Result<usize, RetestError> {
    Ok(indices_ordinal(space, &base_indices(space)?))
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

/// `count` distinct values below `pool`, drawn from one seeded stream in draw order and completed
/// by a deterministic scan when the stream keeps colliding.
fn distinct_draws(seed: u64, count: usize, pool: usize) -> Result<Vec<usize>, RetestError> {
    if count == 0 || pool < count {
        return Err(RetestError::Invalid(
            "perturbation family cannot draw its trials without replacement".into(),
        ));
    }
    let mut rng = SplitMix64(seed);
    let mut seen = BTreeSet::new();
    let mut draws = Vec::with_capacity(count);
    for _ in 0..count.saturating_mul(16).max(32) {
        if draws.len() == count {
            break;
        }
        let draw = (rng.next() as usize) % pool;
        if seen.insert(draw) {
            draws.push(draw);
        }
    }
    for draw in 0..pool {
        if draws.len() == count {
            break;
        }
        if seen.insert(draw) {
            draws.push(draw);
        }
    }
    if draws.len() != count {
        return Err(RetestError::Invalid(
            "perturbation family exhausted its draw pool".into(),
        ));
    }
    Ok(draws)
}

fn trial_values(trials: &[PerturbationTrial]) -> Vec<f64> {
    trials.iter().map(|trial| trial.value).collect()
}

fn summarize(values: &[f64]) -> Result<PerturbationPercentiles, RetestError> {
    if values.is_empty()
        || values.len() > MAX_PERTURBATION_TRIALS
        || values.iter().any(|value| !value.is_finite())
    {
        return Err(RetestError::Invalid(
            "invalid perturbation sample vector".into(),
        ));
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let pick = |basis_points| sorted[percentile_index(sorted.len(), basis_points)];
    Ok(PerturbationPercentiles {
        confidence_level_bps: CONFIDENCE_LEVEL_BPS,
        p05: pick(500),
        median: pick(5_000),
        p95: pick(9_500),
    })
}

fn trial_seed(family_seed: u64, trial_n: usize) -> u64 {
    let mut hasher = Sha256::new();
    hasher.update(TRIAL_SEED_DOMAIN);
    hasher.update(family_seed.to_be_bytes());
    hasher.update((trial_n as u64).to_be_bytes());
    digest_prefix(hasher)
}

fn parse_instant(value: &str) -> Result<chrono::DateTime<chrono::Utc>, RetestError> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|instant| instant.with_timezone(&chrono::Utc))
        .map_err(|_| RetestError::Invalid("malformed perturbation timestamp".into()))
}

fn digest_prefix(hasher: Sha256) -> u64 {
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
