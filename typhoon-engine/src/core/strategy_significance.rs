//! Multiple-testing-adjusted significance derived from sealed parameter-field studies (§7.7).
//!
//! The study never accepts caller-provided scores or p-values. Its headline values and exact
//! one-sided sign tests are reconstructed from verified System Parameter Permutation samples, and
//! both Bonferroni family-wise correction and Benjamini-Hochberg false-discovery-rate correction
//! bind the complete, bounded candidate family and its exact evaluation count.

use crate::core::strategy_optimization::{
    MAX_ARTIFACT_BYTES, MAX_TRIAL_BUDGET, ObjectiveDirection,
};
use crate::core::strategy_parameter_field::ParameterFieldStudyArtifact;
use crate::core::strategy_retest::RetestError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const SIGNIFICANCE_SCHEMA_VERSION: u32 = 1;
const SIGNIFICANCE_ID_DOMAIN: &[u8] = b"typhoon.strategy.significance-study.v1";
pub const MAX_SIGNIFICANCE_CANDIDATES: usize = 4_096;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignificancePolicy {
    /// Performance value representing no edge for the selected metric.
    pub null_value: f64,
    /// Benjamini-Hochberg discovery threshold in basis points of probability.
    pub false_discovery_rate_bps: u32,
    /// A candidate must have at least this many canonically executed field points.
    pub minimum_observations: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateSignificance {
    candidate_id: String,
    field_artifact_id: String,
    observations_n: usize,
    favourable_observations: usize,
    headline_field_estimate: f64,
    raw_p: f64,
    bonferroni_p: f64,
    false_discovery_rate_q: f64,
    significant: bool,
}
impl CandidateSignificance {
    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }
    pub fn field_artifact_id(&self) -> &str {
        &self.field_artifact_id
    }
    pub fn observations_n(&self) -> usize {
        self.observations_n
    }
    pub fn favourable_observations(&self) -> usize {
        self.favourable_observations
    }
    pub fn headline_field_estimate(&self) -> f64 {
        self.headline_field_estimate
    }
    pub fn raw_p(&self) -> f64 {
        self.raw_p
    }
    pub fn bonferroni_p(&self) -> f64 {
        self.bonferroni_p
    }
    pub fn false_discovery_rate_q(&self) -> f64 {
        self.false_discovery_rate_q
    }
    pub fn significant(&self) -> bool {
        self.significant
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignificanceStudyArtifact {
    schema_version: u32,
    artifact_id: String,
    source_dataset_id: String,
    metric_id: String,
    direction: ObjectiveDirection,
    policy: SignificancePolicy,
    evaluations_n: usize,
    source_field_zstd: Vec<Vec<u8>>,
    candidates: Vec<CandidateSignificance>,
}
impl SignificanceStudyArtifact {
    pub fn artifact_id(&self) -> &str {
        &self.artifact_id
    }
    pub fn source_dataset_id(&self) -> &str {
        &self.source_dataset_id
    }
    pub fn metric_id(&self) -> &str {
        &self.metric_id
    }
    pub fn direction(&self) -> ObjectiveDirection {
        self.direction
    }
    pub fn evaluations_n(&self) -> usize {
        self.evaluations_n
    }
    pub fn candidates(&self) -> &[CandidateSignificance] {
        &self.candidates
    }
    pub fn to_json_vec(&self) -> Result<Vec<u8>, RetestError> {
        self.verify()?;
        let bytes = serde_json::to_vec(self).map_err(invalid)?;
        if bytes.len() > MAX_ARTIFACT_BYTES {
            return Err(invalid("significance artifact is too large"));
        }
        Ok(bytes)
    }
    pub fn from_json_slice(bytes: &[u8]) -> Result<Self, RetestError> {
        if bytes.len() > MAX_ARTIFACT_BYTES {
            return Err(invalid("significance artifact is too large"));
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
        validate_policy(self.policy)?;
        if self.schema_version != SIGNIFICANCE_SCHEMA_VERSION
            || !is_id(&self.artifact_id)
            || !is_id(&self.source_dataset_id)
            || self.metric_id.trim().is_empty()
            || self.source_field_zstd.is_empty()
            || self.source_field_zstd.len() > MAX_SIGNIFICANCE_CANDIDATES
            || self.source_field_zstd.len() != self.candidates.len()
        {
            return Err(invalid("invalid significance artifact structure"));
        }
        let fields = decode_fields(&self.source_field_zstd)?;
        let expected = derive_candidates(&fields, self.policy)?;
        let expected_evaluations = total_evaluations(&fields)?;
        if fields[0].metric_id() != self.metric_id
            || fields[0].source_dataset_id() != self.source_dataset_id
            || fields[0].direction() != self.direction
            || expected_evaluations != self.evaluations_n
            || expected != self.candidates
            || self.compute_id()? != self.artifact_id
        {
            return Err(invalid("significance artifact evidence mismatch"));
        }
        Ok(())
    }
    fn compute_id(&self) -> Result<String, RetestError> {
        let payload = (
            self.schema_version,
            &self.source_dataset_id,
            &self.metric_id,
            self.direction,
            self.policy,
            self.evaluations_n,
            &self.source_field_zstd,
            &self.candidates,
        );
        let bytes = serde_json::to_vec(&payload).map_err(invalid)?;
        let mut hasher = Sha256::new();
        hasher.update(SIGNIFICANCE_ID_DOMAIN);
        hasher.update((bytes.len() as u64).to_be_bytes());
        hasher.update(bytes);
        Ok(hex(hasher.finalize()))
    }
}

/// Derive adjusted significance for one complete, bounded family of sealed parameter studies.
pub fn execute_significance_study(
    fields: &[ParameterFieldStudyArtifact],
    policy: SignificancePolicy,
) -> Result<SignificanceStudyArtifact, RetestError> {
    validate_policy(policy)?;
    if fields.is_empty() || fields.len() > MAX_SIGNIFICANCE_CANDIDATES {
        return Err(invalid("invalid significance candidate count"));
    }
    let metric_id = fields[0].metric_id().to_owned();
    let direction = fields[0].direction();
    let source_dataset_id = fields[0].source_dataset_id().to_owned();
    let mut ordered: Vec<&ParameterFieldStudyArtifact> = fields.iter().collect();
    ordered.sort_by(|left, right| {
        left.profile()
            .selected_candidate_id()
            .cmp(right.profile().selected_candidate_id())
    });
    let mut source_field_zstd = Vec::with_capacity(ordered.len());
    for field in ordered {
        field.verify()?;
        if field.metric_id() != metric_id
            || field.direction() != direction
            || field.source_dataset_id() != source_dataset_id
        {
            return Err(invalid("mixed significance metric or objective direction"));
        }
        source_field_zstd.push(
            zstd::bulk::compress(&field.to_json_vec()?, 3)
                .map_err(|error| invalid(format!("cannot compress field evidence: {error}")))?,
        );
    }
    let canonical_fields = decode_fields(&source_field_zstd)?;
    let evaluations_n = total_evaluations(&canonical_fields)?;
    let candidates = derive_candidates(&canonical_fields, policy)?;
    let mut artifact = SignificanceStudyArtifact {
        schema_version: SIGNIFICANCE_SCHEMA_VERSION,
        artifact_id: String::new(),
        source_dataset_id,
        metric_id,
        direction,
        policy,
        evaluations_n,
        source_field_zstd,
        candidates,
    };
    artifact.artifact_id = artifact.compute_id()?;
    artifact.verify()?;
    Ok(artifact)
}

fn decode_fields(bytes: &[Vec<u8>]) -> Result<Vec<ParameterFieldStudyArtifact>, RetestError> {
    bytes
        .iter()
        .map(|compressed| {
            let json = zstd::bulk::decompress(compressed, MAX_ARTIFACT_BYTES)
                .map_err(|error| invalid(format!("cannot decompress field evidence: {error}")))?;
            ParameterFieldStudyArtifact::from_json_slice(&json)
        })
        .collect()
}

fn total_evaluations(fields: &[ParameterFieldStudyArtifact]) -> Result<usize, RetestError> {
    fields.iter().try_fold(0usize, |total, field| {
        total
            .checked_add(field.evaluations_n())
            .filter(|value| *value <= MAX_TRIAL_BUDGET)
            .ok_or_else(|| invalid("significance evaluation count exceeds bound"))
    })
}

fn derive_candidates(
    fields: &[ParameterFieldStudyArtifact],
    policy: SignificancePolicy,
) -> Result<Vec<CandidateSignificance>, RetestError> {
    if fields.is_empty() || fields.len() > MAX_SIGNIFICANCE_CANDIDATES {
        return Err(invalid("invalid significance candidate family"));
    }
    let metric_id = fields[0].metric_id();
    let direction = fields[0].direction();
    let evaluations_n = total_evaluations(fields)?;
    let mut ids = BTreeSet::new();
    let mut candidates = Vec::with_capacity(fields.len());
    for field in fields {
        field.verify()?;
        let candidate_id = field.profile().selected_candidate_id().to_owned();
        let values = field.spp().sorted_values();
        if field.metric_id() != metric_id
            || field.direction() != direction
            || !ids.insert(candidate_id.clone())
            || values.len() < policy.minimum_observations
        {
            return Err(invalid("invalid significance source family"));
        }
        let favourable_observations = values
            .iter()
            .filter(|value| favourable(**value, policy.null_value, direction))
            .count();
        let raw_p = exact_one_sided_sign_p(values.len(), favourable_observations)?;
        candidates.push(CandidateSignificance {
            candidate_id,
            field_artifact_id: field.artifact_id().to_owned(),
            observations_n: values.len(),
            favourable_observations,
            headline_field_estimate: canonical_zero(field.spp().estimate()),
            raw_p,
            bonferroni_p: canonical_zero((raw_p * evaluations_n as f64).min(1.0)),
            false_discovery_rate_q: 1.0,
            significant: false,
        });
    }
    candidates.sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));
    apply_benjamini_hochberg(&mut candidates, policy.false_discovery_rate_bps);
    Ok(candidates)
}

fn apply_benjamini_hochberg(candidates: &mut [CandidateSignificance], alpha_bps: u32) {
    let mut order: Vec<usize> = (0..candidates.len()).collect();
    order.sort_by(|left, right| {
        candidates[*left]
            .raw_p
            .total_cmp(&candidates[*right].raw_p)
            .then_with(|| {
                candidates[*left]
                    .candidate_id
                    .cmp(&candidates[*right].candidate_id)
            })
    });
    let m = candidates.len() as f64;
    let mut running = 1.0f64;
    for position in (0..order.len()).rev() {
        let index = order[position];
        let rank = (position + 1) as f64;
        running = running.min(candidates[index].raw_p * m / rank).min(1.0);
        candidates[index].false_discovery_rate_q = canonical_zero(running);
    }
    let alpha = alpha_bps as f64 / 10_000.0;
    for candidate in candidates {
        candidate.significant =
            candidate.bonferroni_p <= alpha && candidate.false_discovery_rate_q <= alpha;
    }
}

fn exact_one_sided_sign_p(n: usize, successes: usize) -> Result<f64, RetestError> {
    if n == 0 || successes > n || n > MAX_TRIAL_BUDGET {
        return Err(invalid("invalid sign-test sample"));
    }
    if successes == 0 {
        return Ok(1.0);
    }
    // Accumulate the binomial tail in log space so large bounded fields do not underflow.
    let mut log_terms = Vec::with_capacity(n - successes + 1);
    let mut log_choose = 0.0;
    for k in 0..=n {
        if k >= successes {
            log_terms.push(log_choose - n as f64 * std::f64::consts::LN_2);
        }
        if k < n {
            log_choose += ((n - k) as f64).ln() - ((k + 1) as f64).ln();
        }
    }
    let maximum = log_terms
        .iter()
        .copied()
        .reduce(f64::max)
        .ok_or_else(|| invalid("empty sign-test tail"))?;
    let probability = maximum.exp()
        * log_terms
            .iter()
            .map(|term| (term - maximum).exp())
            .sum::<f64>();
    if !probability.is_finite() || !(0.0..=1.0 + 1e-12).contains(&probability) {
        return Err(invalid("invalid sign-test probability"));
    }
    Ok(canonical_zero(probability.min(1.0)))
}

fn favourable(value: f64, null_value: f64, direction: ObjectiveDirection) -> bool {
    match direction {
        ObjectiveDirection::Maximize => value > null_value,
        ObjectiveDirection::Minimize => value < null_value,
    }
}

fn validate_policy(policy: SignificancePolicy) -> Result<(), RetestError> {
    if !policy.null_value.is_finite()
        || policy.false_discovery_rate_bps == 0
        || policy.false_discovery_rate_bps > 10_000
        || policy.minimum_observations == 0
        || policy.minimum_observations > MAX_TRIAL_BUDGET
    {
        return Err(invalid("invalid significance policy"));
    }
    Ok(())
}

fn canonical_zero(value: f64) -> f64 {
    if value == 0.0 { 0.0 } else { value }
}
fn is_id(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
fn hex(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn invalid(message: impl ToString) -> RetestError {
    RetestError::Invalid(message.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_sign_test_and_false_discovery_adjustment_are_deterministic() {
        assert!((exact_one_sided_sign_p(4, 4).unwrap() - 0.0625).abs() < 1e-12);
        assert!((exact_one_sided_sign_p(4, 3).unwrap() - 0.3125).abs() < 1e-12);
        let mut candidates = vec![
            CandidateSignificance {
                candidate_id: "a".repeat(64),
                field_artifact_id: "b".repeat(64),
                observations_n: 8,
                favourable_observations: 8,
                headline_field_estimate: 1.0,
                raw_p: 0.01,
                bonferroni_p: 0.03,
                false_discovery_rate_q: 1.0,
                significant: false,
            },
            CandidateSignificance {
                candidate_id: "c".repeat(64),
                field_artifact_id: "d".repeat(64),
                observations_n: 8,
                favourable_observations: 7,
                headline_field_estimate: 0.5,
                raw_p: 0.03,
                bonferroni_p: 0.09,
                false_discovery_rate_q: 1.0,
                significant: false,
            },
            CandidateSignificance {
                candidate_id: "e".repeat(64),
                field_artifact_id: "f".repeat(64),
                observations_n: 8,
                favourable_observations: 4,
                headline_field_estimate: 0.0,
                raw_p: 0.5,
                bonferroni_p: 1.0,
                false_discovery_rate_q: 1.0,
                significant: false,
            },
        ];
        apply_benjamini_hochberg(&mut candidates, 500);
        assert_eq!(candidates[0].false_discovery_rate_q, 0.03);
        assert_eq!(candidates[1].false_discovery_rate_q, 0.045);
        assert_eq!(candidates[2].false_discovery_rate_q, 0.5);
        assert!(candidates[0].significant);
        assert!(!candidates[1].significant);
        assert!(!candidates[2].significant);
    }

    #[test]
    fn sign_test_refuses_invalid_samples_and_policy() {
        assert!(exact_one_sided_sign_p(0, 0).is_err());
        assert!(exact_one_sided_sign_p(2, 3).is_err());
        assert!(
            validate_policy(SignificancePolicy {
                null_value: f64::NAN,
                false_discovery_rate_bps: 500,
                minimum_observations: 1,
            })
            .is_err()
        );
        assert!(
            validate_policy(SignificancePolicy {
                null_value: 0.0,
                false_discovery_rate_bps: 0,
                minimum_observations: 1,
            })
            .is_err()
        );
    }
}
