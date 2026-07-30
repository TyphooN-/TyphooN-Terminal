//! Canonical report-derived degradation gates and problem recognition for ADR-135 §7.6.
//!
//! The executor accepts only immutable artifacts produced by exact leased runs: a sealed
//! cross-check study, the executed OOS scheme for the same candidate, and the multiple-testing
//! adjusted significance study covering it. Trade count, PnL concentration, exposure,
//! sample-boundary reliance, cost degradation, OOS degradation and adjusted significance are all
//! reconstructed from that evidence through the canonical metric registry and the documented
//! retention formula, so a caller can supply neither an observation nor a verdict. The artifact
//! seals the exact source bytes it judged, which makes `verify` a complete replay.

use crate::core::strategy_cross_check::{
    COST_MULTIPLIER_2X_BPS, CrossCheckKind, CrossCheckStudyArtifact, retention_bps,
};
use crate::core::strategy_metrics::MetricValue;
use crate::core::strategy_optimization::{
    MAX_ARTIFACT_BYTES, MAX_TRIAL_BUDGET, ObjectiveDirection, ProblemObservations, ProblemPolicy,
    SampleRole, StageEvidence, StageVerdict, problem_recognition_gates,
};
use crate::core::strategy_report::StrategyReportArtifact;
use crate::core::strategy_retest::{ExecutedOosScheme, RetestError};
use crate::core::strategy_significance::{CandidateSignificance, SignificanceStudyArtifact};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const PROBLEM_RECOGNITION_SCHEMA_VERSION: u32 = 1;
const ARTIFACT_DOMAIN: &[u8] = b"typhoon.strategy.problem-recognition.v1";
/// The six report-derived primitive gates (§7.6) plus the adjusted-significance gate (§7.7).
const PROBLEM_RECOGNITION_STAGES: usize = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProblemRecognitionPolicy {
    pub minimum_trades: usize,
    pub maximum_top_trade_share_bps: u32,
    pub maximum_time_in_market_bps: u32,
    /// Width of each sample-edge band as a share of the observed calendar duration.
    pub boundary_width_bps: u32,
    pub maximum_boundary_trade_share_bps: u32,
    pub minimum_cost_2x_ratio_bps: u32,
    pub minimum_oos_is_ratio_bps: u32,
}
impl ProblemRecognitionPolicy {
    fn gates(self) -> ProblemPolicy {
        ProblemPolicy {
            minimum_trades: self.minimum_trades,
            maximum_top_trade_share_bps: self.maximum_top_trade_share_bps,
            maximum_time_in_market_bps: self.maximum_time_in_market_bps,
            maximum_boundary_trade_share_bps: self.maximum_boundary_trade_share_bps,
            minimum_cost_2x_ratio_bps: self.minimum_cost_2x_ratio_bps,
            minimum_oos_is_ratio_bps: self.minimum_oos_is_ratio_bps,
        }
    }
}

/// Every observation is reconstructed from sealed evidence; none is caller-supplied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportProblemObservations {
    pub trade_count: usize,
    pub top_trade_share_bps: u32,
    pub time_in_market_bps: u32,
    pub boundary_trade_share_bps: u32,
    pub cost_2x_ratio_bps: u32,
    pub oos_is_ratio_bps: u32,
}
impl From<ReportProblemObservations> for ProblemObservations {
    fn from(value: ReportProblemObservations) -> Self {
        Self {
            trade_count: value.trade_count,
            top_trade_share_bps: value.top_trade_share_bps,
            time_in_market_bps: value.time_in_market_bps,
            boundary_trade_share_bps: value.boundary_trade_share_bps,
            cost_2x_ratio_bps: value.cost_2x_ratio_bps,
            oos_is_ratio_bps: value.oos_is_ratio_bps,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProblemRecognitionArtifact {
    schema_version: u32,
    artifact_id: String,
    source_dataset_id: String,
    strategy_id: String,
    metric_id: String,
    policy: ProblemRecognitionPolicy,
    observations: ReportProblemObservations,
    source_cross_check_zstd: Vec<u8>,
    source_oos_zstd: Vec<u8>,
    source_significance_zstd: Vec<u8>,
    stages: Vec<StageEvidence>,
    passed: bool,
}
impl ProblemRecognitionArtifact {
    pub fn artifact_id(&self) -> &str {
        &self.artifact_id
    }
    pub fn source_dataset_id(&self) -> &str {
        &self.source_dataset_id
    }
    pub fn strategy_id(&self) -> &str {
        &self.strategy_id
    }
    pub fn metric_id(&self) -> &str {
        &self.metric_id
    }
    pub fn policy(&self) -> ProblemRecognitionPolicy {
        self.policy
    }
    pub fn observations(&self) -> ReportProblemObservations {
        self.observations
    }
    pub fn stages(&self) -> &[StageEvidence] {
        &self.stages
    }
    pub fn passed(&self) -> bool {
        self.passed
    }
    /// The exact cross-check study this verdict was derived from.
    pub fn source_cross_check(&self) -> Result<CrossCheckStudyArtifact, RetestError> {
        decode_cross(&self.source_cross_check_zstd)
    }
    /// The exact executed OOS scheme this verdict was derived from.
    pub fn source_oos(&self) -> Result<ExecutedOosScheme, RetestError> {
        decode_oos(&self.source_oos_zstd)
    }
    /// The exact adjusted-significance study this verdict was derived from.
    pub fn source_significance(&self) -> Result<SignificanceStudyArtifact, RetestError> {
        decode_significance(&self.source_significance_zstd)
    }
    pub fn to_json_vec(&self) -> Result<Vec<u8>, RetestError> {
        self.verify()?;
        let bytes = serde_json::to_vec(self).map_err(invalid)?;
        if bytes.len() > MAX_ARTIFACT_BYTES {
            return Err(invalid("problem-recognition artifact is too large"));
        }
        Ok(bytes)
    }
    pub fn from_json_slice(bytes: &[u8]) -> Result<Self, RetestError> {
        if bytes.len() > MAX_ARTIFACT_BYTES {
            return Err(invalid("problem-recognition artifact is too large"));
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
    /// Re-derive every observation, gate and verdict from the sealed source evidence.
    pub fn verify(&self) -> Result<(), RetestError> {
        validate_policy(self.policy)?;
        if self.schema_version != PROBLEM_RECOGNITION_SCHEMA_VERSION
            || !is_id(&self.artifact_id)
            || !is_id(&self.source_dataset_id)
            || !is_id(&self.strategy_id)
            || self.metric_id.trim().is_empty()
            || self.stages.len() != PROBLEM_RECOGNITION_STAGES
        {
            return Err(invalid("invalid problem-recognition artifact structure"));
        }
        let cross = decode_cross(&self.source_cross_check_zstd)?;
        let oos = decode_oos(&self.source_oos_zstd)?;
        let significance = decode_significance(&self.source_significance_zstd)?;
        let (observations, stages, passed) = derive(&cross, &oos, &significance, self.policy)?;
        if self.source_dataset_id != cross.source_dataset_id()
            || self.strategy_id != cross.strategy_id()
            || self.metric_id != cross.metric_id()
            || self.observations != observations
            || self.stages != stages
            || self.passed != passed
            || self.compute_id()? != self.artifact_id
        {
            return Err(invalid("problem-recognition evidence mismatch"));
        }
        Ok(())
    }
    fn compute_id(&self) -> Result<String, RetestError> {
        let mut canonical = self.clone();
        canonical.artifact_id.clear();
        let bytes = serde_json::to_vec(&canonical).map_err(invalid)?;
        let mut hasher = Sha256::new();
        hasher.update(ARTIFACT_DOMAIN);
        hasher.update((bytes.len() as u64).to_be_bytes());
        hasher.update(bytes);
        Ok(hex(hasher.finalize()))
    }
}

/// Recognize problems in one candidate from its sealed cross-check, OOS and significance evidence.
pub fn execute_problem_recognition(
    cross: &CrossCheckStudyArtifact,
    oos: &ExecutedOosScheme,
    significance: &SignificanceStudyArtifact,
    policy: ProblemRecognitionPolicy,
) -> Result<ProblemRecognitionArtifact, RetestError> {
    validate_policy(policy)?;
    // Each encoder verifies its own artifact, so unsealed evidence never reaches a gate.
    let source_cross_check_zstd = compress(&cross.to_json_vec()?)?;
    let source_oos_zstd = compress(&oos.to_json_vec()?)?;
    let source_significance_zstd = compress(&significance.to_json_vec()?)?;
    let canonical_cross = decode_cross(&source_cross_check_zstd)?;
    let canonical_oos = decode_oos(&source_oos_zstd)?;
    let canonical_significance = decode_significance(&source_significance_zstd)?;
    let (observations, stages, passed) = derive(
        &canonical_cross,
        &canonical_oos,
        &canonical_significance,
        policy,
    )?;
    let mut artifact = ProblemRecognitionArtifact {
        schema_version: PROBLEM_RECOGNITION_SCHEMA_VERSION,
        artifact_id: String::new(),
        source_dataset_id: canonical_cross.source_dataset_id().to_owned(),
        strategy_id: canonical_cross.strategy_id().to_owned(),
        metric_id: canonical_cross.metric_id().to_owned(),
        policy,
        observations,
        source_cross_check_zstd,
        source_oos_zstd,
        source_significance_zstd,
        stages,
        passed,
    };
    artifact.artifact_id = artifact.compute_id()?;
    artifact.verify()?;
    Ok(artifact)
}

/// Replay a sealed verdict from the evidence it embedded, and prove the result is identical.
pub fn replay_problem_recognition(
    expected: &ProblemRecognitionArtifact,
) -> Result<ProblemRecognitionArtifact, RetestError> {
    expected.verify()?;
    let replay = execute_problem_recognition(
        &expected.source_cross_check()?,
        &expected.source_oos()?,
        &expected.source_significance()?,
        expected.policy,
    )?;
    if &replay != expected {
        return Err(invalid("problem-recognition replay mismatch"));
    }
    Ok(replay)
}

/// Reconstruct the gates. Every input here is already a decoded, self-verified artifact.
fn derive(
    cross: &CrossCheckStudyArtifact,
    oos: &ExecutedOosScheme,
    significance: &SignificanceStudyArtifact,
    policy: ProblemRecognitionPolicy,
) -> Result<(ReportProblemObservations, Vec<StageEvidence>, bool), RetestError> {
    let candidate = significance
        .candidates()
        .iter()
        .find(|candidate| candidate.candidate_id() == cross.strategy_id())
        .ok_or_else(|| invalid("candidate lacks adjusted significance"))?;
    if cross.strategy_id() != oos.candidate_id()
        || cross.source_dataset_id() != oos.source_dataset_id()
        || cross.source_dataset_id() != significance.source_dataset_id()
        || cross.metric_id() != oos.metric_id()
        || cross.metric_id() != significance.metric_id()
        || cross.direction() != significance.direction()
        || cross.baseline().config_id != oos.config_id()
        || cross.baseline().range() != oos.source_range()
    {
        return Err(invalid("foreign problem-recognition evidence"));
    }
    let report = cross.baseline_report()?;
    let observations = derive_observations(&report, cross, oos, policy)?;
    let mut stages = problem_recognition_gates(observations.into(), policy.gates())?;
    stages.push(significance_stage(significance, candidate));
    if stages.len() != PROBLEM_RECOGNITION_STAGES {
        return Err(invalid("unexpected problem-recognition gate count"));
    }
    let passed = stages
        .iter()
        .all(|stage| stage.verdict == StageVerdict::Pass);
    Ok((observations, stages, passed))
}

fn derive_observations(
    report: &StrategyReportArtifact,
    cross: &CrossCheckStudyArtifact,
    oos: &ExecutedOosScheme,
    policy: ProblemRecognitionPolicy,
) -> Result<ReportProblemObservations, RetestError> {
    let trade_count = report.analysis().trades.len();
    if trade_count == 0 || trade_count > MAX_TRIAL_BUDGET {
        return Err(invalid(
            "problem recognition requires bounded closed trades",
        ));
    }
    Ok(ReportProblemObservations {
        trade_count,
        top_trade_share_bps: metric_ratio_bps(report, "top_decile_pnl_share")?,
        time_in_market_bps: metric_ratio_bps(report, "time_in_market")?,
        boundary_trade_share_bps: boundary_share_bps(report, policy.boundary_width_bps)?,
        cost_2x_ratio_bps: cost_2x_ratio_bps(cross)?,
        oos_is_ratio_bps: oos_ratio_bps(oos, cross.direction())?,
    })
}

fn significance_stage(
    significance: &SignificanceStudyArtifact,
    candidate: &CandidateSignificance,
) -> StageEvidence {
    let evaluations_n = significance.evaluations_n();
    let reason = format!(
        "bonferroni p={} and false-discovery-rate q={} over N={evaluations_n} evaluations",
        candidate.bonferroni_p(),
        candidate.false_discovery_rate_q()
    );
    if candidate.significant() {
        StageEvidence::pass("adjusted-significance", evaluations_n, reason)
    } else {
        StageEvidence::fail("adjusted-significance", evaluations_n, reason)
    }
}

/// One registry metric that is contractually a `[0, 1]` ratio, in basis points.
fn metric_ratio_bps(report: &StrategyReportArtifact, metric: &str) -> Result<u32, RetestError> {
    let value = match report.analysis().metric(metric) {
        Some(MetricValue::Defined { value }) if value.is_finite() => *value,
        _ => return Err(invalid(format!("undefined problem metric {metric}"))),
    };
    if !(0.0..=1.0).contains(&value) {
        return Err(invalid(format!("invalid ratio metric {metric}")));
    }
    Ok((value * 10_000.0).round() as u32)
}

/// Share of closed trades that touch either edge band of the observed calendar (§7.6 "systematic
/// reliance on the very first or last bars"). A trade counts once, whichever edge it reaches.
fn boundary_share_bps(
    report: &StrategyReportArtifact,
    boundary_width_bps: u32,
) -> Result<u32, RetestError> {
    let daily = &report.analysis().calendar.daily;
    let start = daily
        .first()
        .map(|point| point.closing_time_ns)
        .ok_or_else(|| invalid("missing daily calendar evidence"))?;
    let end = daily
        .last()
        .map(|point| point.closing_time_ns)
        .ok_or_else(|| invalid("missing daily calendar evidence"))?;
    let duration = end
        .checked_sub(start)
        .filter(|value| *value > 0)
        .ok_or_else(|| {
            invalid("problem recognition requires a positive observed calendar duration")
        })?;
    let width = (i128::from(duration) * i128::from(boundary_width_bps) / 10_000)
        .try_into()
        .map_err(|_| invalid("boundary duration overflow"))?;
    let lower = start.saturating_add(width);
    let upper = end.saturating_sub(width);
    let boundary = report
        .analysis()
        .trades
        .iter()
        .filter(|trade| trade.entry_time_ns <= lower || trade.exit_time_ns >= upper)
        .count();
    count_bps(boundary, report.analysis().trades.len())
}

/// Performance retained under the sealed 2× cost observation, capped at the gate's ratio domain.
fn cost_2x_ratio_bps(cross: &CrossCheckStudyArtifact) -> Result<u32, RetestError> {
    cross
        .checks()
        .iter()
        .find_map(|check| match check.kind {
            CrossCheckKind::CostSensitivity { multiplier_bps }
                if multiplier_bps == COST_MULTIPLIER_2X_BPS =>
            {
                Some(check.retention_bps.min(10_000))
            }
            _ => None,
        })
        .ok_or_else(|| invalid("missing 2x cost evidence"))
}

/// Out-of-sample degradation: the documented cross-check retention formula applied to the mean
/// executed OOS score against the mean executed IS score. Purged and embargoed bars never scored.
fn oos_ratio_bps(
    oos: &ExecutedOosScheme,
    direction: ObjectiveDirection,
) -> Result<u32, RetestError> {
    let mut in_sample_sum = 0.0;
    let mut in_sample_n = 0usize;
    let mut out_of_sample_sum = 0.0;
    let mut out_of_sample_n = 0usize;
    for partition in oos.executed_partitions() {
        match partition.role {
            SampleRole::InSample => {
                in_sample_sum += partition.score;
                in_sample_n += 1;
            }
            SampleRole::OutOfSample => {
                out_of_sample_sum += partition.score;
                out_of_sample_n += 1;
            }
            SampleRole::Purged | SampleRole::Embargoed => {}
        }
    }
    if in_sample_n == 0 || out_of_sample_n == 0 {
        return Err(invalid("missing executed IS/OOS evidence"));
    }
    let in_sample = in_sample_sum / in_sample_n as f64;
    let out_of_sample = out_of_sample_sum / out_of_sample_n as f64;
    Ok(retention_bps(in_sample, out_of_sample, direction)?.min(10_000))
}

fn count_bps(count: usize, total: usize) -> Result<u32, RetestError> {
    if total == 0 || count > total {
        return Err(invalid("invalid problem-recognition count"));
    }
    let scaled = count
        .checked_mul(10_000)
        .ok_or_else(|| invalid("problem-recognition count overflow"))?;
    Ok(((scaled + total / 2) / total) as u32)
}
fn validate_policy(policy: ProblemRecognitionPolicy) -> Result<(), RetestError> {
    let ratios = [
        policy.maximum_top_trade_share_bps,
        policy.maximum_time_in_market_bps,
        policy.boundary_width_bps,
        policy.maximum_boundary_trade_share_bps,
        policy.minimum_cost_2x_ratio_bps,
        policy.minimum_oos_is_ratio_bps,
    ];
    if policy.minimum_trades == 0
        || policy.minimum_trades > MAX_TRIAL_BUDGET
        || policy.boundary_width_bps == 0
        || policy.boundary_width_bps > 5_000
        || ratios.iter().any(|value| *value > 10_000)
    {
        return Err(invalid("invalid problem-recognition policy"));
    }
    Ok(())
}
fn compress(bytes: &[u8]) -> Result<Vec<u8>, RetestError> {
    zstd::bulk::compress(bytes, 3).map_err(invalid)
}
fn decompress(bytes: &[u8]) -> Result<Vec<u8>, RetestError> {
    zstd::bulk::decompress(bytes, MAX_ARTIFACT_BYTES).map_err(invalid)
}
fn decode_cross(bytes: &[u8]) -> Result<CrossCheckStudyArtifact, RetestError> {
    CrossCheckStudyArtifact::from_json_slice(&decompress(bytes)?)
}
fn decode_oos(bytes: &[u8]) -> Result<ExecutedOosScheme, RetestError> {
    ExecutedOosScheme::from_json_slice(&decompress(bytes)?)
}
fn decode_significance(bytes: &[u8]) -> Result<SignificanceStudyArtifact, RetestError> {
    SignificanceStudyArtifact::from_json_slice(&decompress(bytes)?)
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
    fn bounded_share_and_policy_helpers_are_exact_and_fail_closed() {
        assert_eq!(count_bps(1, 3).unwrap(), 3_333);
        assert_eq!(count_bps(0, 4).unwrap(), 0);
        assert_eq!(count_bps(4, 4).unwrap(), 10_000);
        assert!(count_bps(2, 1).is_err());
        assert!(count_bps(0, 0).is_err());
        let valid = ProblemRecognitionPolicy {
            minimum_trades: 1,
            maximum_top_trade_share_bps: 10_000,
            maximum_time_in_market_bps: 10_000,
            boundary_width_bps: 1_000,
            maximum_boundary_trade_share_bps: 10_000,
            minimum_cost_2x_ratio_bps: 0,
            minimum_oos_is_ratio_bps: 0,
        };
        validate_policy(valid).unwrap();
        for broken in [
            ProblemRecognitionPolicy {
                minimum_trades: 0,
                ..valid
            },
            ProblemRecognitionPolicy {
                boundary_width_bps: 0,
                ..valid
            },
            ProblemRecognitionPolicy {
                boundary_width_bps: 5_001,
                ..valid
            },
            ProblemRecognitionPolicy {
                maximum_top_trade_share_bps: 10_001,
                ..valid
            },
            ProblemRecognitionPolicy {
                minimum_oos_is_ratio_bps: 10_001,
                ..valid
            },
        ] {
            assert!(validate_policy(broken).is_err());
        }
    }
}
