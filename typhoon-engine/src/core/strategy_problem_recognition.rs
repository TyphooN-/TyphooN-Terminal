//! Canonical report-derived degradation gates and problem recognition for ADR-135 §7.6.
//!
//! The executor accepts only immutable artifacts produced by exact leased runs: a sealed
//! cross-check study, the executed OOS scheme for the same candidate, and the multiple-testing
//! adjusted significance study covering it. Every §7.6 red flag is reconstructed from that
//! evidence — trade count, PnL concentration, exposure, sample-boundary reliance, cost degradation
//! at both sealed rungs of the §7.5 ladder, calendar/symbol/side edge concentration, absurd metric
//! bounds, the ±1 parameter-step sensitivity cliff, OOS degradation, and the §7.7 adjusted
//! significance verdict — through the canonical metric registry and the documented retention
//! formula, so a caller can supply neither an observation nor a verdict. The parameter-field
//! evidence the step cliff needs is read out of the significance study that already binds the
//! candidate, never accepted separately. The artifact seals the exact source bytes it judged, which
//! makes `verify` a complete replay.
//!
//! Two §7.6 clauses need their scope stated because the ADR words them loosely:
//!
//! - Bullet 7 ("performance collapses under ±1 parameter step or 2× costs") is two gates here:
//!   `cost-degradation` for the cost half and `parameter-step-cliff` for the step half. The §7.5
//!   ladder also seals a 3× rung; leaving sealed evidence ungated is exactly the robustness
//!   theatre §14 warns about, so `cost-degradation-3x` gates it against its own stated bound.
//! - Bullet 6 ("PF at the sentinel") has no numeric sentinel in this metric registry: profit factor
//!   is *undefined* with no losing trades. That degenerate value is the sentinel, so it is what the
//!   absurd-metric gate flags.

use crate::core::strategy_cross_check::{
    COST_MULTIPLIER_2X_BPS, COST_MULTIPLIER_3X_BPS, CrossCheckKind, CrossCheckStudyArtifact,
    retention_bps,
};
use crate::core::strategy_metrics::{
    CalendarEquity, CalendarPoint, MetricValue, StrategyAnalysis, TradeDirection, UndefinedReason,
};
use crate::core::strategy_optimization::{
    MAX_ARTIFACT_BYTES, MAX_TRIAL_BUDGET, ObjectiveDirection, ProblemObservations, ProblemPolicy,
    SampleRole, StageEvidence, StageVerdict, problem_recognition_gates,
};
use crate::core::strategy_parameter_field::ParameterFieldStudyArtifact;
use crate::core::strategy_report::StrategyReportArtifact;
use crate::core::strategy_retest::{ExecutedOosScheme, RetestError};
use crate::core::strategy_significance::{CandidateSignificance, SignificanceStudyArtifact};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

const PROBLEM_RECOGNITION_SCHEMA_VERSION: u32 = 2;
const ARTIFACT_DOMAIN: &[u8] = b"typhoon.strategy.problem-recognition.v2";
/// The six shared primitive gates, the four report-derived §7.6 gates completed here, and the §7.7
/// adjusted-significance gate.
const PROBLEM_RECOGNITION_STAGES: usize = 11;
/// A "plausible range" for the Sharpe ratio stops meaning anything past this, so a policy may not
/// disable the absurd-metric gate by naming a bound above it. 100.0 annualized.
const MAXIMUM_SHARPE_CEILING_BPS: u32 = 1_000_000;

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
    pub minimum_cost_3x_ratio_bps: u32,
    pub minimum_oos_is_ratio_bps: u32,
    /// Largest share of the edge one calendar period, symbol, or side may account for.
    pub maximum_edge_concentration_bps: u32,
    pub maximum_absolute_sharpe_bps: u32,
    /// Smallest maximum drawdown that is still credible; below it the curve is too clean to be real.
    pub minimum_max_drawdown_bps: u32,
    pub minimum_parameter_step_ratio_bps: u32,
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

/// Which §7.6 bullet-3 family the edge turned out to be most concentrated in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConcentrationFamily {
    CalendarPeriod,
    Symbol,
    Side,
}
impl ConcentrationFamily {
    fn label(self) -> &'static str {
        match self {
            Self::CalendarPeriod => "calendar-period",
            Self::Symbol => "symbol",
            Self::Side => "side",
        }
    }
}

/// The calendar granularity the concentration measure was taken at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalendarGranularity {
    Annual,
    Monthly,
    Weekly,
    Daily,
}

/// §7.6 "edge concentrated in one calendar period, one symbol, or one side". Each family's share is
/// the largest single bucket's gain over the total gain across buckets. A family with one populated
/// gain bucket is 100% concentrated; a family with no gain at all has no edge to attribute and is
/// left out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EdgeConcentration {
    /// Coarsest granularity of the sealed calendar that resolves at least two periods.
    pub calendar_granularity: CalendarGranularity,
    pub calendar_periods: usize,
    pub calendar_share_bps: Option<u32>,
    pub symbols: usize,
    pub symbol_share_bps: Option<u32>,
    pub sides: usize,
    pub side_share_bps: Option<u32>,
    /// The most concentrated evaluable family and its share, or `None` when none is evaluable.
    pub worst: Option<(ConcentrationFamily, u32)>,
}

/// §7.6 "absurd metrics", read straight off the canonical registry values of the sealed report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AbsurdMetricObservations {
    /// `|sharpe_ratio|` in basis points, clamped to the `u32` domain; `None` when the registry
    /// leaves the ratio undefined, which for a zero-variance equity curve is itself absurd.
    pub absolute_sharpe_bps: Option<u32>,
    /// `max_drawdown_percent` in basis points; `None` when the registry leaves it undefined.
    pub max_drawdown_bps: Option<u32>,
    /// True when `profit_factor` sits at the registry's degenerate no-losing-trades value.
    pub profit_factor_at_sentinel: bool,
}

/// §7.6 "performance collapses under ±1 parameter step", measured over the immutable single-axis
/// neighbours the sealed parameter field already executed around its selected point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParameterStepObservations {
    pub steps_n: usize,
    /// Worst direction-aware retention across those neighbours, in the gate's ratio domain.
    pub worst_step_ratio_bps: u32,
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
    pub cost_3x_ratio_bps: u32,
    pub oos_is_ratio_bps: u32,
    pub edge_concentration: EdgeConcentration,
    pub absurd_metrics: AbsurdMetricObservations,
    pub parameter_step: ParameterStepObservations,
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
    /// The exact adjusted-significance study this verdict was derived from, which also carries the
    /// sealed parameter-field evidence the step-cliff gate read.
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
        // Verification is a complete replay of the gate set, so an artifact sealed against a
        // different gate set can never re-derive. Say so instead of reporting a shape problem.
        if self.schema_version != PROBLEM_RECOGNITION_SCHEMA_VERSION {
            return Err(invalid(format!(
                "unsupported problem-recognition schema version {} (supported {PROBLEM_RECOGNITION_SCHEMA_VERSION})",
                self.schema_version
            )));
        }
        if !is_id(&self.artifact_id)
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
    let field = candidate_field(significance, candidate)?;
    if field.source_dataset_id() != cross.source_dataset_id()
        || field.metric_id() != cross.metric_id()
        || field.direction() != cross.direction()
        || field.config_id() != cross.baseline().config_id
        || field.range() != cross.baseline().range()
    {
        return Err(invalid("foreign problem-recognition field evidence"));
    }
    let report = cross.baseline_report()?;
    let observations = derive_observations(&report, cross, oos, &field, policy)?;
    let mut stages = problem_recognition_gates(observations.into(), policy.gates())?;
    stages.push(cost_3x_stage(&observations, policy));
    stages.push(edge_concentration_stage(
        &observations.edge_concentration,
        policy,
    ));
    stages.push(absurd_metric_stage(&observations.absurd_metrics, policy));
    stages.push(parameter_step_stage(&observations.parameter_step, policy));
    stages.push(significance_stage(significance, candidate));
    if stages.len() != PROBLEM_RECOGNITION_STAGES {
        return Err(invalid("unexpected problem-recognition gate count"));
    }
    let passed = stages
        .iter()
        .all(|stage| stage.verdict == StageVerdict::Pass);
    Ok((observations, stages, passed))
}

/// The sealed parameter field the significance study judged this candidate from. Both the study's
/// recorded artifact id and the candidate the field itself selected have to agree, so a field
/// belonging to a sibling candidate of the same family cannot stand in for this one.
fn candidate_field(
    significance: &SignificanceStudyArtifact,
    candidate: &CandidateSignificance,
) -> Result<ParameterFieldStudyArtifact, RetestError> {
    significance
        .source_fields()?
        .into_iter()
        .find(|field| {
            field.artifact_id() == candidate.field_artifact_id()
                && field.profile().selected_candidate_id() == candidate.candidate_id()
        })
        .ok_or_else(|| invalid("candidate lacks sealed parameter-field evidence"))
}

fn derive_observations(
    report: &StrategyReportArtifact,
    cross: &CrossCheckStudyArtifact,
    oos: &ExecutedOosScheme,
    field: &ParameterFieldStudyArtifact,
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
        cost_2x_ratio_bps: cost_ratio_bps(cross, COST_MULTIPLIER_2X_BPS)?,
        cost_3x_ratio_bps: cost_ratio_bps(cross, COST_MULTIPLIER_3X_BPS)?,
        oos_is_ratio_bps: oos_ratio_bps(oos, cross.direction())?,
        edge_concentration: edge_concentration(report.analysis())?,
        absurd_metrics: absurd_metrics(report.analysis())?,
        parameter_step: parameter_step(field)?,
    })
}

fn cost_3x_stage(
    observations: &ReportProblemObservations,
    policy: ProblemRecognitionPolicy,
) -> StageEvidence {
    let reason = format!(
        "{} >= {} bps at 3x cost",
        observations.cost_3x_ratio_bps, policy.minimum_cost_3x_ratio_bps
    );
    stage(
        "cost-degradation-3x",
        observations.cost_3x_ratio_bps >= policy.minimum_cost_3x_ratio_bps,
        1,
        reason,
    )
}

fn edge_concentration_stage(
    observations: &EdgeConcentration,
    policy: ProblemRecognitionPolicy,
) -> StageEvidence {
    let detail = format!(
        "calendar-period {}/{} bps, symbol {}/{} bps, side {}/{} bps",
        observations.calendar_periods,
        share_text(observations.calendar_share_bps),
        observations.symbols,
        share_text(observations.symbol_share_bps),
        observations.sides,
        share_text(observations.side_share_bps),
    );
    match observations.worst {
        Some((family, share)) => stage(
            "edge-concentration",
            share <= policy.maximum_edge_concentration_bps,
            observations.evaluable_buckets(family),
            format!(
                "worst family {} at {share} <= {} bps; {detail}",
                family.label(),
                policy.maximum_edge_concentration_bps
            ),
        ),
        // Nothing to compare against means nothing certifies diversification.
        None => StageEvidence::fail(
            "edge-concentration",
            0,
            format!("no evaluable concentration family; {detail}"),
        ),
    }
}

fn absurd_metric_stage(
    observations: &AbsurdMetricObservations,
    policy: ProblemRecognitionPolicy,
) -> StageEvidence {
    let plausible = observations
        .absolute_sharpe_bps
        .is_some_and(|bps| bps <= policy.maximum_absolute_sharpe_bps)
        && observations
            .max_drawdown_bps
            .is_some_and(|bps| bps >= policy.minimum_max_drawdown_bps)
        && !observations.profit_factor_at_sentinel;
    let reason = format!(
        "abs sharpe {} <= {} bps, max drawdown {} >= {} bps, profit factor at sentinel {}",
        share_text(observations.absolute_sharpe_bps),
        policy.maximum_absolute_sharpe_bps,
        share_text(observations.max_drawdown_bps),
        policy.minimum_max_drawdown_bps,
        observations.profit_factor_at_sentinel
    );
    stage("absurd-metrics", plausible, 3, reason)
}

fn parameter_step_stage(
    observations: &ParameterStepObservations,
    policy: ProblemRecognitionPolicy,
) -> StageEvidence {
    let reason = format!(
        "worst of N={} single-axis +/-1 steps retained {} >= {} bps",
        observations.steps_n,
        observations.worst_step_ratio_bps,
        policy.minimum_parameter_step_ratio_bps
    );
    stage(
        "parameter-step-cliff",
        observations.worst_step_ratio_bps >= policy.minimum_parameter_step_ratio_bps,
        observations.steps_n,
        reason,
    )
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
    stage(
        "adjusted-significance",
        candidate.significant(),
        evaluations_n,
        reason,
    )
}

fn stage(name: &str, passed: bool, observations_n: usize, reason: String) -> StageEvidence {
    if passed {
        StageEvidence::pass(name, observations_n, reason)
    } else {
        StageEvidence::fail(name, observations_n, reason)
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

/// Performance retained under one sealed rung of the §7.5 cost ladder, capped at the gate's ratio
/// domain. Both rungs the ladder always seals are read; neither is optional.
fn cost_ratio_bps(
    cross: &CrossCheckStudyArtifact,
    multiplier_bps: u32,
) -> Result<u32, RetestError> {
    cross
        .checks()
        .iter()
        .find_map(|check| match check.kind {
            CrossCheckKind::CostSensitivity {
                multiplier_bps: sealed,
            } if sealed == multiplier_bps => Some(check.retention_bps.min(10_000)),
            _ => None,
        })
        .ok_or_else(|| invalid(format!("missing {multiplier_bps} bps cost evidence")))
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

/// §7.6 bullet 3. The calendar family is measured on the sealed mark-to-market calendar, whose
/// per-period changes telescope to the run's total equity change; the symbol and side families
/// partition the sealed closed-trade list's gross profit.
fn edge_concentration(analysis: &StrategyAnalysis) -> Result<EdgeConcentration, RetestError> {
    let (calendar_granularity, calendar_periods, calendar_share_bps) =
        calendar_concentration(&analysis.calendar)?;
    let mut symbol_gains: BTreeMap<usize, f64> = BTreeMap::new();
    let mut side_gains: BTreeMap<u8, f64> = BTreeMap::new();
    for trade in &analysis.trades {
        if !trade.net_pnl.is_finite() {
            return Err(invalid("non-finite trade profit in problem evidence"));
        }
        let gain = trade.net_pnl.max(0.0);
        *symbol_gains.entry(trade.symbol.0).or_default() += gain;
        *side_gains
            .entry(match trade.direction {
                TradeDirection::Long => 0,
                TradeDirection::Short => 1,
            })
            .or_default() += gain;
    }
    let symbol_share_bps = dominant_share_bps(symbol_gains.values().copied())?;
    let side_share_bps = dominant_share_bps(side_gains.values().copied())?;
    let worst = [
        (ConcentrationFamily::CalendarPeriod, calendar_share_bps),
        (ConcentrationFamily::Symbol, symbol_share_bps),
        (ConcentrationFamily::Side, side_share_bps),
    ]
    .into_iter()
    .filter_map(|(family, share)| share.map(|share| (family, share)))
    .max_by_key(|(_, share)| *share);
    Ok(EdgeConcentration {
        calendar_granularity,
        calendar_periods,
        calendar_share_bps,
        symbols: symbol_gains.len(),
        symbol_share_bps,
        sides: side_gains.len(),
        side_share_bps,
        worst,
    })
}

/// The coarsest sealed granularity that resolves at least two periods, because "one calendar
/// period" means the longest real period the run actually spans more than one of.
fn calendar_concentration(
    calendar: &CalendarEquity,
) -> Result<(CalendarGranularity, usize, Option<u32>), RetestError> {
    for (granularity, series) in [
        (CalendarGranularity::Annual, &calendar.annual),
        (CalendarGranularity::Monthly, &calendar.monthly),
        (CalendarGranularity::Weekly, &calendar.weekly),
        (CalendarGranularity::Daily, &calendar.daily),
    ] {
        if series.len() < 2 {
            continue;
        }
        let share = dominant_share_bps(series.iter().map(gain))?;
        return Ok((granularity, series.len(), share));
    }
    Err(invalid(
        "problem recognition requires at least two calendar periods",
    ))
}

fn gain(point: &CalendarPoint) -> f64 {
    point.change.max(0.0)
}

/// Largest bucket's gain over the total gain across buckets, in basis points. One positive bucket
/// is fully concentrated. `None` means there is no gain to attribute at all.
fn dominant_share_bps(gains: impl IntoIterator<Item = f64>) -> Result<Option<u32>, RetestError> {
    let mut total = 0.0f64;
    let mut largest = 0.0f64;
    for gain in gains {
        if !gain.is_finite() || gain < 0.0 {
            return Err(invalid("invalid concentration weight"));
        }
        total += gain;
        largest = largest.max(gain);
    }
    if total <= 0.0 {
        return Ok(None);
    }
    Ok(Some(
        ((largest / total) * 10_000.0).round().clamp(0.0, 10_000.0) as u32,
    ))
}

/// §7.6 bullet 6, in the metric semantics this registry actually publishes.
fn absurd_metrics(analysis: &StrategyAnalysis) -> Result<AbsurdMetricObservations, RetestError> {
    Ok(AbsurdMetricObservations {
        absolute_sharpe_bps: optional_magnitude_bps(analysis, "sharpe_ratio")?,
        max_drawdown_bps: optional_magnitude_bps(analysis, "max_drawdown_percent")?,
        profit_factor_at_sentinel: matches!(
            analysis.metric("profit_factor"),
            Some(MetricValue::Undefined {
                reason: UndefinedReason::NoLosingTrades
            })
        ),
    })
}

/// A registry metric's magnitude in basis points, clamped to the `u32` domain so an absurd value
/// is still an exact bound rather than an overflow. `None` when the registry left it undefined; a
/// metric missing from the report altogether is foreign evidence.
fn optional_magnitude_bps(
    analysis: &StrategyAnalysis,
    metric: &str,
) -> Result<Option<u32>, RetestError> {
    match analysis.metric(metric) {
        Some(MetricValue::Defined { value }) if value.is_finite() => Ok(Some(
            (value.abs() * 10_000.0)
                .round()
                .clamp(0.0, f64::from(u32::MAX)) as u32,
        )),
        Some(MetricValue::Defined { .. }) => {
            Err(invalid(format!("non-finite problem metric {metric}")))
        }
        Some(MetricValue::Undefined { .. }) => Ok(None),
        None => Err(invalid(format!("problem metric {metric} is absent"))),
    }
}

/// §7.6 bullet 7's step half. The sealed field executed every ordinal within its neighbour radius
/// of the selected point, so the single-axis ±1 coordinates are already immutable evidence.
fn parameter_step(
    field: &ParameterFieldStudyArtifact,
) -> Result<ParameterStepObservations, RetestError> {
    let centre_ordinal = field.plateau().centre_ordinal();
    let centre = field
        .points()
        .iter()
        .find(|point| point.ordinal == centre_ordinal)
        .ok_or_else(|| invalid("sealed parameter field lacks its selected point"))?;
    let mut steps_n = 0usize;
    let mut worst: Option<u32> = None;
    for point in field.points() {
        if !is_single_axis_step(&centre.axis_indices, &point.axis_indices) {
            continue;
        }
        steps_n += 1;
        let retained = retention_bps(centre.value, point.value, field.direction())?.min(10_000);
        worst = Some(worst.map_or(retained, |current| current.min(retained)));
    }
    match worst {
        Some(worst_step_ratio_bps) => Ok(ParameterStepObservations {
            steps_n,
            worst_step_ratio_bps,
        }),
        None => Err(invalid(
            "sealed parameter field has no executed +/-1 step neighbour",
        )),
    }
}

/// Exactly one axis moved, and it moved exactly one step.
fn is_single_axis_step(centre: &[usize], other: &[usize]) -> bool {
    centre.len() == other.len()
        && centre
            .iter()
            .zip(other)
            .map(|(left, right)| left.abs_diff(*right))
            .try_fold(0usize, |moved, distance| match distance {
                0 => Some(moved),
                1 => Some(moved + 1),
                _ => None,
            })
            == Some(1)
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
fn share_text(share_bps: Option<u32>) -> String {
    share_bps.map_or_else(|| "undefined".to_string(), |value| value.to_string())
}
fn validate_policy(policy: ProblemRecognitionPolicy) -> Result<(), RetestError> {
    let ratios = [
        policy.maximum_top_trade_share_bps,
        policy.maximum_time_in_market_bps,
        policy.boundary_width_bps,
        policy.maximum_boundary_trade_share_bps,
        policy.minimum_cost_2x_ratio_bps,
        policy.minimum_cost_3x_ratio_bps,
        policy.minimum_oos_is_ratio_bps,
        policy.maximum_edge_concentration_bps,
        policy.minimum_max_drawdown_bps,
        policy.minimum_parameter_step_ratio_bps,
    ];
    if policy.minimum_trades == 0
        || policy.minimum_trades > MAX_TRIAL_BUDGET
        || policy.boundary_width_bps == 0
        || policy.boundary_width_bps > 5_000
        || policy.maximum_absolute_sharpe_bps == 0
        || policy.maximum_absolute_sharpe_bps > MAXIMUM_SHARPE_CEILING_BPS
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

impl EdgeConcentration {
    /// The bucket count behind the family that produced the reported worst share, so the stage can
    /// state the exact N its claim rests on.
    fn evaluable_buckets(&self, family: ConcentrationFamily) -> usize {
        match family {
            ConcentrationFamily::CalendarPeriod => self.calendar_periods,
            ConcentrationFamily::Symbol => self.symbols,
            ConcentrationFamily::Side => self.sides,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_policy() -> ProblemRecognitionPolicy {
        ProblemRecognitionPolicy {
            minimum_trades: 1,
            maximum_top_trade_share_bps: 10_000,
            maximum_time_in_market_bps: 10_000,
            boundary_width_bps: 1_000,
            maximum_boundary_trade_share_bps: 10_000,
            minimum_cost_2x_ratio_bps: 0,
            minimum_cost_3x_ratio_bps: 0,
            minimum_oos_is_ratio_bps: 0,
            maximum_edge_concentration_bps: 10_000,
            maximum_absolute_sharpe_bps: MAXIMUM_SHARPE_CEILING_BPS,
            minimum_max_drawdown_bps: 0,
            minimum_parameter_step_ratio_bps: 0,
        }
    }

    #[test]
    fn bounded_share_and_policy_helpers_are_exact_and_fail_closed() {
        assert_eq!(count_bps(1, 3).unwrap(), 3_333);
        assert_eq!(count_bps(0, 4).unwrap(), 0);
        assert_eq!(count_bps(4, 4).unwrap(), 10_000);
        assert!(count_bps(2, 1).is_err());
        assert!(count_bps(0, 0).is_err());
        let valid = valid_policy();
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
            ProblemRecognitionPolicy {
                minimum_cost_3x_ratio_bps: 10_001,
                ..valid
            },
            ProblemRecognitionPolicy {
                maximum_edge_concentration_bps: 10_001,
                ..valid
            },
            ProblemRecognitionPolicy {
                minimum_max_drawdown_bps: 10_001,
                ..valid
            },
            ProblemRecognitionPolicy {
                minimum_parameter_step_ratio_bps: 10_001,
                ..valid
            },
            // The absurd-metric gate may not be switched off by naming a vacuous bound.
            ProblemRecognitionPolicy {
                maximum_absolute_sharpe_bps: 0,
                ..valid
            },
            ProblemRecognitionPolicy {
                maximum_absolute_sharpe_bps: MAXIMUM_SHARPE_CEILING_BPS + 1,
                ..valid
            },
        ] {
            assert!(validate_policy(broken).is_err());
        }
    }

    #[test]
    fn dominant_share_needs_two_buckets_and_a_real_gain() {
        assert_eq!(dominant_share_bps([3.0, 1.0]).unwrap(), Some(7_500));
        assert_eq!(
            dominant_share_bps([1.0, 1.0, 1.0, 1.0]).unwrap(),
            Some(2_500)
        );
        assert_eq!(dominant_share_bps([5.0]).unwrap(), Some(10_000));
        // No gain is nothing to attribute.
        assert_eq!(dominant_share_bps([0.0, 0.0]).unwrap(), None);
        assert_eq!(dominant_share_bps([]).unwrap(), None);
        assert!(dominant_share_bps([1.0, -1.0]).is_err());
        assert!(dominant_share_bps([1.0, f64::NAN]).is_err());
    }

    #[test]
    fn only_a_single_axis_single_step_move_is_a_parameter_step() {
        assert!(is_single_axis_step(&[1, 1], &[0, 1]));
        assert!(is_single_axis_step(&[1, 1], &[1, 2]));
        assert!(is_single_axis_step(&[4], &[3]));
        // The centre itself, a diagonal, and a two-step move are all not ±1 steps.
        assert!(!is_single_axis_step(&[1, 1], &[1, 1]));
        assert!(!is_single_axis_step(&[1, 1], &[0, 0]));
        assert!(!is_single_axis_step(&[1, 1], &[1, 3]));
        assert!(!is_single_axis_step(&[1, 1], &[1]));
    }
}
