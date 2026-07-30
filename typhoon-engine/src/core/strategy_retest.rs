//! Executable, content-bound ADR-135 M4 retest and evidence persistence boundary.
//!
//! This is deliberately the only M4 path that turns a retest request into a metric. It verifies
//! the exact leased dataset bytes, assembles a [`VerifiedRun`](crate::core::strategy_run::VerifiedRun),
//! executes the canonical simulator, seals and re-verifies the report, and only then projects one
//! typed metric into robustness. Persisted evidence is append-only and queried through bounded,
//! indexed windows.

use crate::broker::alpaca::Bar;
use crate::core::strategy_bayesian::BayesianStudyArtifact;
use crate::core::strategy_cross_check::CrossCheckStudyArtifact;
use crate::core::strategy_dataset::DatasetManifest;
use crate::core::strategy_ir::{
    DatasetBinding, RunBinding, StrategyExecutionConfig, StrategyIr, StrategyRunManifest,
};
use crate::core::strategy_metrics::{METRICS_SCHEMA_VERSION, MetricValue};
use crate::core::strategy_optimization::{
    BurnedHoldout, CalendarFoldPlan, CalendarWalkForwardConfig, CalendarWindowBounds, FoldPlan,
    MAX_ARTIFACT_BYTES, MAX_MONTE_CARLO_TRIALS, MAX_SEARCH_COMBINATIONS, MAX_TRIAL_BUDGET,
    ObjectiveDirection, ObjectiveSpec, ObservationRole, OosPlan, OosScheme, OptimizationError,
    ReportObservation, RetestRequest, RetestResult, RobustnessArtifact, RobustnessPipeline,
    SampleRole, SearchBatch, SearchDataLease, SplitMix64, StageAccess, StageVerdict,
    WalkForwardConfig, max_drawdown, percentile_index, select_best,
};
use crate::core::strategy_parameter_field::ParameterFieldStudyArtifact;
use crate::core::strategy_perturbation::PerturbationStudyArtifact;
use crate::core::strategy_problem_recognition::ProblemRecognitionArtifact;
use crate::core::strategy_report::StrategyReportArtifact;
use crate::core::strategy_run::{RunDatasetInput, assemble_verified_run};
use crate::core::strategy_significance::SignificanceStudyArtifact;
use crate::core::strategy_simulator::run_verified_simulation;
use rusqlite::{Connection, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::ops::Range;
use std::path::Path;
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError, TrySendError};
use std::sync::{Arc, Mutex};

pub const MAX_RETEST_QUERY_LIMIT: usize = 200;
pub const MAX_RETEST_EVENTS_PER_POLL: usize = 8;
const MAX_TRACKED_RETEST_CANCELLATIONS: usize = 64;
const REQUEST_DOMAIN: &[u8] = b"typhoon.strategy_retest.execution_request.v1";
const RETEST_ENGINE_VERSION: &str = concat!("typhoon-engine/", env!("CARGO_PKG_VERSION"));

#[derive(Debug)]
pub enum RetestError {
    Invalid(String),
    Optimization(OptimizationError),
    Sqlite(String),
    DuplicateLineage,
    HoldoutAlreadyConsumed,
    Immutable,
}
impl std::fmt::Display for RetestError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "retest error: {self:?}")
    }
}
impl std::error::Error for RetestError {}
impl From<rusqlite::Error> for RetestError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error.to_string())
    }
}
impl From<OptimizationError> for RetestError {
    fn from(error: OptimizationError) -> Self {
        Self::Optimization(error)
    }
}

/// Owned request whose constructor proves that `bars` are the immutable content granted by the
/// lease. The range length must equal the payload length; passing a full dataset under a narrower
/// lease, or a sliced payload under a full manifest, is refused before simulation.
#[derive(Debug)]
pub struct RetestExecutionRequest {
    request_id: String,
    strategy: StrategyIr,
    config: StrategyExecutionConfig,
    dataset: DatasetManifest,
    bars: Vec<Bar>,
    lease: SearchDataLease,
    role: ObservationRole,
    metric_id: String,
    retest: RetestRequest,
}
impl RetestExecutionRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        strategy: &StrategyIr,
        config: &StrategyExecutionConfig,
        dataset: &DatasetManifest,
        bars: &[Bar],
        lease: SearchDataLease,
        role: ObservationRole,
        metric_id: impl Into<String>,
        root_seed: u64,
    ) -> Result<Self, RetestError> {
        strategy.verify().map_err(invalid)?;
        config.verify().map_err(invalid)?;
        dataset.verify(bars).map_err(invalid)?;
        let range = lease.range();
        if lease.dataset_id() != dataset.dataset_id
            || range.start >= range.end
            || range.len() != bars.len()
        {
            return Err(RetestError::Invalid(
                "lease range and content-addressed bar payload disagree".into(),
            ));
        }
        let metric_id = metric_id.into();
        if metric_id.trim().is_empty() {
            return Err(RetestError::Invalid("metric id is empty".into()));
        }
        let retest = RetestRequest::seal(
            strategy,
            &lease,
            config.config_id(),
            METRICS_SCHEMA_VERSION,
            root_seed,
        )?;
        let mut request = Self {
            request_id: String::new(),
            strategy: strategy.clone(),
            config: config.clone(),
            dataset: dataset.clone(),
            bars: bars.to_vec(),
            lease,
            role,
            metric_id,
            retest,
        };
        request.request_id = request.compute_id();
        Ok(request)
    }
    pub fn request_id(&self) -> &str {
        &self.request_id
    }
    pub fn metrics_version(&self) -> &str {
        METRICS_SCHEMA_VERSION
    }
    fn compute_id(&self) -> String {
        execution_request_id(
            &self.retest,
            &self.dataset.manifest_id,
            self.role,
            &self.metric_id,
        )
    }
}

pub(crate) fn execution_request_id(
    retest: &RetestRequest,
    manifest_id: &str,
    role: ObservationRole,
    metric_id: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(REQUEST_DOMAIN);
    frame(&mut hasher, retest.request_id().as_bytes());
    frame(&mut hasher, manifest_id.as_bytes());
    hasher.update([role_key(role)]);
    frame(&mut hasher, metric_id.as_bytes());
    hex(hasher.finalize())
}

#[derive(Debug)]
pub struct CompletedRetest {
    request_id: String,
    report: StrategyReportArtifact,
    observation: ReportObservation,
    robustness: RobustnessArtifact,
    evaluations_n: usize,
    metric_id: String,
    metric_value: f64,
}
impl CompletedRetest {
    pub fn run_id(&self) -> &str {
        self.report.run_id()
    }
    pub fn report(&self) -> &StrategyReportArtifact {
        &self.report
    }
    pub fn observation(&self) -> &ReportObservation {
        &self.observation
    }
    pub fn robustness(&self) -> &RobustnessArtifact {
        &self.robustness
    }
    pub fn evaluations_n(&self) -> usize {
        self.evaluations_n
    }
    pub fn best_label(&self) -> String {
        self.robustness.best_label(self.metric_value)
    }
}

const FINAL_HOLDOUT_REQUEST_DOMAIN: &[u8] = b"typhoon.strategy_retest.final_holdout.v1";
const MAX_HOLDOUT_REASON_BYTES: usize = 512;

/// The sole capability that carries final-holdout bars into execution. It cannot be constructed
/// from a search/robustness lease: callers must consume [`HoldoutQuarantine`](crate::core::strategy_optimization::HoldoutQuarantine)
/// into the non-clone [`BurnedHoldout`] token first.
#[derive(Debug)]
pub struct FinalHoldoutExecutionRequest {
    request_id: String,
    strategy: StrategyIr,
    config: StrategyExecutionConfig,
    search_manifest: DatasetManifest,
    holdout_manifest: DatasetManifest,
    holdout_bars: Vec<Bar>,
    range: Range<usize>,
    reason: String,
    metric_id: String,
    seed: u64,
    evaluations_n: usize,
}
impl FinalHoldoutExecutionRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        strategy: &StrategyIr,
        config: &StrategyExecutionConfig,
        parent_manifest: &DatasetManifest,
        parent_bars: &[Bar],
        burned: BurnedHoldout,
        metric_id: impl Into<String>,
        seed: u64,
        evaluations_n: usize,
    ) -> Result<Self, RetestError> {
        parent_manifest.verify(parent_bars).map_err(invalid)?;
        let range = burned.range();
        if range.start == 0 || range.end != parent_bars.len() || range.start >= range.end {
            return Err(RetestError::Invalid(
                "burn capability does not name a terminal parent partition".into(),
            ));
        }
        let manifest_input = parent_manifest.to_input();
        let search_manifest = DatasetManifest::build(&manifest_input, &parent_bars[..range.start])
            .map_err(invalid)?;
        let holdout_bars = parent_bars[range.clone()].to_vec();
        let holdout_manifest =
            DatasetManifest::build(&manifest_input, &holdout_bars).map_err(invalid)?;
        let mut request = Self {
            request_id: String::new(),
            strategy: strategy.clone(),
            config: config.clone(),
            search_manifest,
            holdout_manifest,
            holdout_bars,
            range,
            reason: burned.reason().to_string(),
            metric_id: metric_id.into(),
            seed,
            evaluations_n,
        };
        if burned.search_dataset_id() != request.search_manifest.dataset_id
            || burned.dataset_id() != request.holdout_manifest.dataset_id
        {
            return Err(RetestError::Invalid(
                "burn capability names foreign dataset content".into(),
            ));
        }
        request.validate()?;
        request.request_id = request.compute_id();
        Ok(request)
    }
    pub fn request_id(&self) -> &str {
        &self.request_id
    }
    pub fn search_dataset_id(&self) -> &str {
        &self.search_manifest.dataset_id
    }
    pub fn holdout_dataset_id(&self) -> &str {
        &self.holdout_manifest.dataset_id
    }
    fn validate(&self) -> Result<(), RetestError> {
        self.strategy.verify().map_err(invalid)?;
        self.config.verify().map_err(invalid)?;
        self.search_manifest.verify_seal().map_err(invalid)?;
        self.holdout_manifest
            .verify(&self.holdout_bars)
            .map_err(invalid)?;
        let search_count = usize::try_from(self.search_manifest.bar_count)
            .map_err(|_| RetestError::Invalid("search dataset is too large".into()))?;
        if self.search_manifest.dataset_id == self.holdout_manifest.dataset_id
            || self.range.start != search_count
            || self.range.end != search_count.saturating_add(self.holdout_bars.len())
            || self.range.len() != self.holdout_bars.len()
            || self.reason.trim().is_empty()
            || self.reason.len() > MAX_HOLDOUT_REASON_BYTES
            || self.metric_id.trim().is_empty()
            || self.evaluations_n == 0
            || self.evaluations_n > MAX_TRIAL_BUDGET
        {
            return Err(RetestError::Invalid(
                "invalid final-holdout execution binding".into(),
            ));
        }
        Ok(())
    }
    fn verify(&self) -> Result<(), RetestError> {
        self.validate()?;
        if self.request_id != self.compute_id() {
            return Err(RetestError::Invalid(
                "final-holdout request identity mismatch".into(),
            ));
        }
        Ok(())
    }
    fn compute_id(&self) -> String {
        final_holdout_request_id(
            self.strategy.strategy_id(),
            self.config.config_id(),
            &self.search_manifest.dataset_id,
            &self.search_manifest.manifest_id,
            &self.holdout_manifest.dataset_id,
            &self.holdout_manifest.manifest_id,
            &self.reason,
            &self.metric_id,
            METRICS_SCHEMA_VERSION,
            &self.range,
            self.seed,
            self.evaluations_n,
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn final_holdout_request_id(
    strategy_id: &str,
    config_id: &str,
    search_dataset_id: &str,
    search_manifest_id: &str,
    holdout_dataset_id: &str,
    holdout_manifest_id: &str,
    reason: &str,
    metric_id: &str,
    metrics_version: &str,
    range: &Range<usize>,
    seed: u64,
    evaluations_n: usize,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(FINAL_HOLDOUT_REQUEST_DOMAIN);
    for value in [
        strategy_id,
        config_id,
        search_dataset_id,
        search_manifest_id,
        holdout_dataset_id,
        holdout_manifest_id,
        reason,
        metric_id,
        metrics_version,
    ] {
        frame(&mut hasher, value.as_bytes());
    }
    hasher.update((range.start as u64).to_be_bytes());
    hasher.update((range.end as u64).to_be_bytes());
    hasher.update(seed.to_be_bytes());
    hasher.update((evaluations_n as u64).to_be_bytes());
    hex(hasher.finalize())
}

#[derive(Debug)]
pub struct CompletedFinalHoldout {
    request_id: String,
    strategy_id: String,
    config_id: String,
    report: StrategyReportArtifact,
    metric_id: String,
    metric_value: f64,
    evaluations_n: usize,
}
impl CompletedFinalHoldout {
    pub fn request_id(&self) -> &str {
        &self.request_id
    }
    pub fn run_id(&self) -> &str {
        self.report.run_id()
    }
    pub fn strategy_id(&self) -> &str {
        &self.strategy_id
    }
    pub fn config_id(&self) -> &str {
        &self.config_id
    }
    pub fn report(&self) -> &StrategyReportArtifact {
        &self.report
    }
    pub fn metric_id(&self) -> &str {
        &self.metric_id
    }
    pub fn metric_value(&self) -> f64 {
        self.metric_value
    }
    pub fn evaluations_n(&self) -> usize {
        self.evaluations_n
    }
}

pub fn execute_retest(
    request: RetestExecutionRequest,
    pipeline: &RobustnessPipeline,
    evaluations_n: usize,
) -> Result<CompletedRetest, RetestError> {
    let (report, observation, metric_value) = execute_bound_observation(&request)?;
    let outcome = pipeline.execute(
        &request.lease,
        request.strategy.strategy_id(),
        evaluations_n,
        vec![observation.clone()],
    )?;
    Ok(CompletedRetest {
        request_id: request.request_id,
        report,
        observation,
        robustness: outcome.artifact().clone(),
        evaluations_n,
        metric_id: request.metric_id,
        metric_value,
    })
}

/// The shared content-bound execution core. Both one-off retests and multi-window studies pass
/// through this exact `VerifiedRun` → canonical simulator → sealed report boundary.
pub(crate) fn execute_bound_observation(
    request: &RetestExecutionRequest,
) -> Result<(StrategyReportArtifact, ReportObservation, f64), RetestError> {
    if request.request_id != request.compute_id() {
        return Err(RetestError::Invalid(
            "execution request identity mismatch".into(),
        ));
    }
    let manifest = StrategyRunManifest::build(&RunBinding {
        datasets: vec![DatasetBinding {
            input_id: "primary".into(),
            dataset_id: request.dataset.dataset_id.clone(),
        }],
        sub_bar_datasets: vec![],
        strategy_id: request.strategy.strategy_id().to_string(),
        config_id: request.config.config_id().to_string(),
        seed: request.retest.root_seed(),
        engine_version: RETEST_ENGINE_VERSION.into(),
        metrics_version: METRICS_SCHEMA_VERSION.into(),
        intervention_log_id: None,
        repaint_qa: vec![],
    })
    .map_err(invalid)?;
    let inputs = [RunDatasetInput {
        input_id: "primary",
        manifest: &request.dataset,
        bars: &request.bars,
    }];
    let verified = assemble_verified_run(&request.strategy, &request.config, &manifest, &inputs)
        .map_err(invalid)?;
    let simulation = run_verified_simulation(&verified).map_err(invalid)?;
    let report = StrategyReportArtifact::build_for_verified_run(
        &verified,
        &simulation,
        request.config.settings().initial_capital,
    )
    .map_err(invalid)?;
    report
        .verify_against(&manifest, &simulation)
        .map_err(invalid)?;
    let result = RetestResult::seal(&request.retest, report.report_id())?;
    let observation = ReportObservation::from_report(
        &request.lease,
        request.role,
        &request.retest,
        &result,
        &report,
        &[request.metric_id.as_str()],
    )?;
    let metric_value = observation
        .metric(&request.metric_id)
        .ok_or_else(|| RetestError::Invalid("report metric is undefined".into()))?;
    Ok((report, observation, metric_value))
}

fn execute_verified_report(
    strategy: &StrategyIr,
    config: &StrategyExecutionConfig,
    dataset: &DatasetManifest,
    bars: &[Bar],
    seed: u64,
) -> Result<StrategyReportArtifact, RetestError> {
    dataset.verify(bars).map_err(invalid)?;
    let manifest = StrategyRunManifest::build(&RunBinding {
        datasets: vec![DatasetBinding {
            input_id: "primary".into(),
            dataset_id: dataset.dataset_id.clone(),
        }],
        sub_bar_datasets: vec![],
        strategy_id: strategy.strategy_id().to_string(),
        config_id: config.config_id().to_string(),
        seed,
        engine_version: RETEST_ENGINE_VERSION.into(),
        metrics_version: METRICS_SCHEMA_VERSION.into(),
        intervention_log_id: None,
        repaint_qa: vec![],
    })
    .map_err(invalid)?;
    let inputs = [RunDatasetInput {
        input_id: "primary",
        manifest: dataset,
        bars,
    }];
    let verified = assemble_verified_run(strategy, config, &manifest, &inputs).map_err(invalid)?;
    let simulation = run_verified_simulation(&verified).map_err(invalid)?;
    let report = StrategyReportArtifact::build_for_verified_run(
        &verified,
        &simulation,
        config.settings().initial_capital,
    )
    .map_err(invalid)?;
    report
        .verify_against(&manifest, &simulation)
        .map_err(invalid)?;
    Ok(report)
}

pub const TRADE_MONTE_CARLO_SCHEMA_VERSION: u32 = 1;
pub const MAX_MONTE_CARLO_TRADES: usize = 10_000;
const TRADE_MONTE_CARLO_DOMAIN: &[u8] = b"typhoon.strategy_retest.trade_monte_carlo.v1";
const TRADE_MONTE_CARLO_CONFIDENCE_BPS: u32 = 9_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradeMonteCarloConfig {
    pub seed: u64,
    pub trials: usize,
    pub trade_skip_bps: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeMonteCarloFamily {
    TradeOrderShuffle,
    RandomTradeSubset,
    BootstrapWithReplacement,
}
impl TradeMonteCarloFamily {
    const ALL: [Self; 3] = [
        Self::TradeOrderShuffle,
        Self::RandomTradeSubset,
        Self::BootstrapWithReplacement,
    ];
    fn tag(self) -> &'static [u8] {
        match self {
            Self::TradeOrderShuffle => b"trade_order_shuffle",
            Self::RandomTradeSubset => b"random_trade_subset",
            Self::BootstrapWithReplacement => b"bootstrap_with_replacement",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TradeMonteCarloSample {
    net_profit: f64,
    max_drawdown: f64,
}
impl TradeMonteCarloSample {
    pub fn net_profit(&self) -> f64 {
        self.net_profit
    }
    pub fn max_drawdown(&self) -> f64 {
        self.max_drawdown
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TradeMonteCarloPercentiles {
    confidence_level_bps: u32,
    p05: f64,
    median: f64,
    p95: f64,
}
impl TradeMonteCarloPercentiles {
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
pub struct TradeMonteCarloFamilyEvidence {
    family: TradeMonteCarloFamily,
    component_seed: u64,
    samples: Vec<TradeMonteCarloSample>,
    net_profit: TradeMonteCarloPercentiles,
    max_drawdown: TradeMonteCarloPercentiles,
}
impl TradeMonteCarloFamilyEvidence {
    pub fn family(&self) -> TradeMonteCarloFamily {
        self.family
    }
    pub fn component_seed(&self) -> u64 {
        self.component_seed
    }
    pub fn samples(&self) -> &[TradeMonteCarloSample] {
        &self.samples
    }
    pub fn net_profit(&self) -> &TradeMonteCarloPercentiles {
        &self.net_profit
    }
    pub fn max_drawdown(&self) -> &TradeMonteCarloPercentiles {
        &self.max_drawdown
    }
}

/// Content-addressed Monte Carlo evidence. Its only production constructor executes the canonical
/// retest boundary above and derives trade PnL from the newly sealed report; no metric ledger or
/// trade vector crosses the public API.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TradeMonteCarloArtifact {
    schema_version: u32,
    artifact_id: String,
    candidate_id: String,
    run_id: String,
    report_id: String,
    dataset_id: String,
    config_id: String,
    root_seed: u64,
    seed: u64,
    evaluations_n: usize,
    trials: usize,
    trade_skip_bps: u32,
    trade_count: usize,
    families: Vec<TradeMonteCarloFamilyEvidence>,
}
impl TradeMonteCarloArtifact {
    pub fn artifact_id(&self) -> &str {
        &self.artifact_id
    }
    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }
    pub fn run_id(&self) -> &str {
        &self.run_id
    }
    pub fn report_id(&self) -> &str {
        &self.report_id
    }
    pub fn dataset_id(&self) -> &str {
        &self.dataset_id
    }
    pub fn config_id(&self) -> &str {
        &self.config_id
    }
    pub fn root_seed(&self) -> u64 {
        self.root_seed
    }
    pub fn seed(&self) -> u64 {
        self.seed
    }
    pub fn evaluations_n(&self) -> usize {
        self.evaluations_n
    }
    pub fn trade_count(&self) -> usize {
        self.trade_count
    }
    pub fn families(&self) -> &[TradeMonteCarloFamilyEvidence] {
        &self.families
    }
    pub fn verify(&self) -> Result<(), RetestError> {
        if self.schema_version != TRADE_MONTE_CARLO_SCHEMA_VERSION
            || self.artifact_id.len() != 64
            || self.candidate_id.trim().is_empty()
            || self.run_id.trim().is_empty()
            || self.report_id.trim().is_empty()
            || self.dataset_id.trim().is_empty()
            || self.config_id.trim().is_empty()
            || self.evaluations_n == 0
            || self.evaluations_n > MAX_TRIAL_BUDGET
            || self.trials == 0
            || self.trials > MAX_MONTE_CARLO_TRIALS
            || self.trade_skip_bps >= 10_000
            || self.trade_count == 0
            || self.trade_count > MAX_MONTE_CARLO_TRADES
            || self
                .trials
                .checked_mul(self.trade_count)
                .is_none_or(|work| work > MAX_SEARCH_COMBINATIONS)
            || self.families.len() != TradeMonteCarloFamily::ALL.len()
        {
            return Err(RetestError::Invalid(
                "invalid Monte Carlo artifact bounds".into(),
            ));
        }
        for (expected, evidence) in TradeMonteCarloFamily::ALL.iter().zip(&self.families) {
            if evidence.family != *expected
                || evidence.component_seed != derive_trade_monte_carlo_seed(self, *expected)
                || evidence.samples.len() != self.trials
                || evidence.samples.iter().any(|sample| {
                    !sample.net_profit.is_finite() || !sample.max_drawdown.is_finite()
                })
                || evidence.net_profit
                    != summarize_samples(&evidence.samples, |sample| sample.net_profit)?
                || evidence.max_drawdown
                    != summarize_samples(&evidence.samples, |sample| sample.max_drawdown)?
            {
                return Err(RetestError::Invalid(
                    "invalid Monte Carlo family evidence".into(),
                ));
            }
        }
        if self.compute_id()? != self.artifact_id {
            return Err(RetestError::Invalid(
                "Monte Carlo artifact identity mismatch".into(),
            ));
        }
        Ok(())
    }
    pub fn to_json_vec(&self) -> Result<Vec<u8>, RetestError> {
        self.verify()?;
        let bytes = serde_json::to_vec(self).map_err(invalid)?;
        if bytes.len() > MAX_ARTIFACT_BYTES {
            return Err(RetestError::Invalid(
                "Monte Carlo artifact is too large".into(),
            ));
        }
        Ok(bytes)
    }
    pub fn from_json_slice(bytes: &[u8]) -> Result<Self, RetestError> {
        if bytes.len() > MAX_ARTIFACT_BYTES {
            return Err(RetestError::Invalid(
                "Monte Carlo artifact is too large".into(),
            ));
        }
        let artifact: Self = serde_json::from_slice(bytes).map_err(invalid)?;
        artifact.verify()?;
        Ok(artifact)
    }
    fn compute_id(&self) -> Result<String, RetestError> {
        let identity = serde_json::to_vec(&(
            self.schema_version,
            &self.candidate_id,
            &self.run_id,
            &self.report_id,
            &self.dataset_id,
            &self.config_id,
            self.root_seed,
            self.seed,
            self.evaluations_n,
            self.trials,
            self.trade_skip_bps,
            self.trade_count,
            &self.families,
        ))
        .map_err(invalid)?;
        let mut hasher = Sha256::new();
        hasher.update(TRADE_MONTE_CARLO_DOMAIN);
        frame(&mut hasher, &identity);
        Ok(hex(hasher.finalize()))
    }
}

pub fn execute_trade_monte_carlo(
    request: RetestExecutionRequest,
    config: TradeMonteCarloConfig,
    evaluations_n: usize,
) -> Result<TradeMonteCarloArtifact, RetestError> {
    if request.lease.stage() != StageAccess::Robustness
        || config.trials == 0
        || config.trials > MAX_MONTE_CARLO_TRIALS
        || config.trade_skip_bps >= 10_000
        || evaluations_n == 0
        || evaluations_n > MAX_TRIAL_BUDGET
    {
        return Err(RetestError::Invalid(
            "invalid Monte Carlo request bounds or stage".into(),
        ));
    }
    let candidate_id = request.strategy.strategy_id().to_string();
    let dataset_id = request.dataset.dataset_id.clone();
    let config_id = request.config.config_id().to_string();
    let root_seed = request.retest.root_seed();
    let (report, _, _) = execute_bound_observation(&request)?;
    let trades = report
        .analysis()
        .trades
        .iter()
        .map(|trade| trade.net_pnl)
        .collect::<Vec<_>>();
    if trades.is_empty()
        || trades.len() > MAX_MONTE_CARLO_TRADES
        || trades.iter().any(|value| !value.is_finite())
        || config
            .trials
            .checked_mul(trades.len())
            .is_none_or(|work| work > MAX_SEARCH_COMBINATIONS)
    {
        return Err(RetestError::Invalid(
            "undefined or unbounded canonical trade evidence".into(),
        ));
    }
    let mut artifact = TradeMonteCarloArtifact {
        schema_version: TRADE_MONTE_CARLO_SCHEMA_VERSION,
        artifact_id: String::new(),
        candidate_id,
        run_id: report.run_id().to_string(),
        report_id: report.report_id().to_string(),
        dataset_id,
        config_id,
        root_seed,
        seed: config.seed,
        evaluations_n,
        trials: config.trials,
        trade_skip_bps: config.trade_skip_bps,
        trade_count: trades.len(),
        families: Vec::new(),
    };
    for family in TradeMonteCarloFamily::ALL {
        let component_seed = derive_trade_monte_carlo_seed(&artifact, family);
        artifact.families.push(execute_trade_monte_carlo_family(
            &trades,
            family,
            component_seed,
            config,
        )?);
    }
    artifact.artifact_id = artifact.compute_id()?;
    artifact.verify()?;
    if artifact.to_json_vec()?.len() > MAX_ARTIFACT_BYTES {
        return Err(RetestError::Invalid(
            "Monte Carlo artifact is too large".into(),
        ));
    }
    Ok(artifact)
}

pub fn replay_trade_monte_carlo(
    request: RetestExecutionRequest,
    expected: &TradeMonteCarloArtifact,
) -> Result<TradeMonteCarloArtifact, RetestError> {
    expected.verify()?;
    let replayed = execute_trade_monte_carlo(
        request,
        TradeMonteCarloConfig {
            seed: expected.seed,
            trials: expected.trials,
            trade_skip_bps: expected.trade_skip_bps,
        },
        expected.evaluations_n,
    )?;
    if &replayed != expected {
        return Err(RetestError::Invalid(
            "foreign or non-deterministic Monte Carlo evidence".into(),
        ));
    }
    Ok(replayed)
}

fn execute_trade_monte_carlo_family(
    trades: &[f64],
    family: TradeMonteCarloFamily,
    component_seed: u64,
    config: TradeMonteCarloConfig,
) -> Result<TradeMonteCarloFamilyEvidence, RetestError> {
    let mut rng = SplitMix64(component_seed);
    let mut work = Vec::with_capacity(trades.len());
    let mut samples = Vec::with_capacity(config.trials);
    for _ in 0..config.trials {
        work.clear();
        match family {
            TradeMonteCarloFamily::TradeOrderShuffle => {
                work.extend_from_slice(trades);
                for index in (1..work.len()).rev() {
                    let other = (rng.next() as usize) % (index + 1);
                    work.swap(index, other);
                }
            }
            TradeMonteCarloFamily::RandomTradeSubset => {
                for &trade in trades {
                    if rng.next() % 10_000 >= u64::from(config.trade_skip_bps) {
                        work.push(trade);
                    }
                }
                if work.is_empty() {
                    work.push(trades[(rng.next() as usize) % trades.len()]);
                }
            }
            TradeMonteCarloFamily::BootstrapWithReplacement => {
                for _ in 0..trades.len() {
                    work.push(trades[(rng.next() as usize) % trades.len()]);
                }
            }
        }
        let net_profit = work.iter().copied().sum::<f64>();
        let max_drawdown = max_drawdown(&work);
        if !net_profit.is_finite() || !max_drawdown.is_finite() {
            return Err(RetestError::Invalid("non-finite Monte Carlo metric".into()));
        }
        samples.push(TradeMonteCarloSample {
            net_profit,
            max_drawdown,
        });
    }
    Ok(TradeMonteCarloFamilyEvidence {
        family,
        component_seed,
        net_profit: summarize_samples(&samples, |sample| sample.net_profit)?,
        max_drawdown: summarize_samples(&samples, |sample| sample.max_drawdown)?,
        samples,
    })
}

fn summarize_samples(
    samples: &[TradeMonteCarloSample],
    project: impl Fn(&TradeMonteCarloSample) -> f64,
) -> Result<TradeMonteCarloPercentiles, RetestError> {
    if samples.is_empty() || samples.len() > MAX_MONTE_CARLO_TRIALS {
        return Err(RetestError::Invalid(
            "invalid Monte Carlo sample vector".into(),
        ));
    }
    let mut values = samples.iter().map(project).collect::<Vec<_>>();
    if values.iter().any(|value| !value.is_finite()) {
        return Err(RetestError::Invalid("non-finite Monte Carlo sample".into()));
    }
    values.sort_by(f64::total_cmp);
    let pick = |basis_points| values[percentile_index(values.len(), basis_points)];
    Ok(TradeMonteCarloPercentiles {
        confidence_level_bps: TRADE_MONTE_CARLO_CONFIDENCE_BPS,
        p05: pick(500),
        median: pick(5_000),
        p95: pick(9_500),
    })
}

fn derive_trade_monte_carlo_seed(
    artifact: &TradeMonteCarloArtifact,
    family: TradeMonteCarloFamily,
) -> u64 {
    let mut hasher = Sha256::new();
    hasher.update(TRADE_MONTE_CARLO_DOMAIN);
    hasher.update(artifact.root_seed.to_be_bytes());
    hasher.update(artifact.seed.to_be_bytes());
    for value in [
        artifact.candidate_id.as_bytes(),
        artifact.run_id.as_bytes(),
        artifact.report_id.as_bytes(),
        artifact.dataset_id.as_bytes(),
        artifact.config_id.as_bytes(),
        family.tag(),
    ] {
        frame(&mut hasher, value);
    }
    let digest = hasher.finalize();
    let mut prefix = [0_u8; 8];
    prefix.copy_from_slice(&digest[..8]);
    u64::from_be_bytes(prefix)
}

pub const MAX_WALK_FORWARD_MATRIX_CELLS: usize = 32;
const MAX_STUDY_WINDOWS: usize = 128;
pub const STUDY_ARTIFACT_SCHEMA_VERSION: u32 = 1;
const OOS_ARTIFACT_DOMAIN: &[u8] = b"typhoon.strategy_retest.oos_artifact.v1";
const WALK_FORWARD_ARTIFACT_DOMAIN: &[u8] = b"typhoon.strategy_retest.walk_forward_artifact.v1";
const CALENDAR_WALK_FORWARD_ARTIFACT_DOMAIN: &[u8] =
    b"typhoon.strategy_retest.calendar_walk_forward_artifact.v1";
const WALK_FORWARD_MATRIX_ARTIFACT_DOMAIN: &[u8] =
    b"typhoon.strategy_retest.walk_forward_matrix_artifact.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExactBarMember {
    pub source_index: usize,
    pub timestamp: String,
    pub role: SampleRole,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExactBarMembership {
    pub role: SampleRole,
    pub ranges: Vec<Range<usize>>,
    pub indices: Vec<usize>,
    pub timestamps: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OosSeamProof {
    pub purged: Range<usize>,
    pub oos: Range<usize>,
    pub embargoed: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutedPartition {
    pub role: SampleRole,
    pub range: Range<usize>,
    pub dataset_id: String,
    pub run_id: String,
    pub report_id: String,
    pub score: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OosExecutionSpec {
    pub scheme: OosScheme,
    pub purge_bars: usize,
    pub embargo_bars: usize,
    pub metric_id: String,
    pub root_seed: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutedOosScheme {
    schema_version: u32,
    artifact_id: String,
    candidate_id: String,
    source_manifest_id: String,
    source_dataset_id: String,
    source_range: Range<usize>,
    config_id: String,
    scheme: OosScheme,
    purge_bars: usize,
    embargo_bars: usize,
    metric_id: String,
    root_seed: u64,
    membership: Vec<ExactBarMember>,
    seams: Vec<OosSeamProof>,
    executed_partitions: Vec<ExecutedPartition>,
}
impl ExecutedOosScheme {
    pub fn artifact_id(&self) -> &str {
        &self.artifact_id
    }
    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }
    pub fn source_dataset_id(&self) -> &str {
        &self.source_dataset_id
    }
    pub fn metric_id(&self) -> &str {
        &self.metric_id
    }
    pub fn config_id(&self) -> &str {
        &self.config_id
    }
    pub fn source_range(&self) -> Range<usize> {
        self.source_range.clone()
    }
    pub fn membership(&self) -> &[ExactBarMember] {
        &self.membership
    }
    pub fn seams(&self) -> &[OosSeamProof] {
        &self.seams
    }
    pub fn executed_partitions(&self) -> &[ExecutedPartition] {
        &self.executed_partitions
    }
    pub fn verify(&self) -> Result<(), RetestError> {
        if self.schema_version != STUDY_ARTIFACT_SCHEMA_VERSION
            || !is_id(&self.artifact_id)
            || !is_id(&self.candidate_id)
            || !is_id(&self.source_manifest_id)
            || !is_id(&self.source_dataset_id)
            || !is_id(&self.config_id)
            || self.source_range.start >= self.source_range.end
            || self.source_range.len() != self.membership.len()
            || self.membership.is_empty()
            || self.membership.len() > MAX_SEARCH_COMBINATIONS
            || self.metric_id.trim().is_empty()
            || self.executed_partitions.is_empty()
            || self.executed_partitions.len() > MAX_STUDY_WINDOWS
            || self.membership.iter().enumerate().any(|(offset, member)| {
                member.source_index != self.source_range.start + offset
                    || member.timestamp.trim().is_empty()
            })
            || self.executed_partitions.iter().any(|partition| {
                partition.range.start < self.source_range.start
                    || partition.range.end > self.source_range.end
                    || partition.range.start >= partition.range.end
                    || !is_id(&partition.dataset_id)
                    || !is_id(&partition.run_id)
                    || !is_id(&partition.report_id)
                    || !partition.score.is_finite()
            })
            || self.seams.len() > MAX_STUDY_WINDOWS
            || self.compute_id()? != self.artifact_id
        {
            return Err(RetestError::Invalid("invalid OOS study artifact".into()));
        }
        Ok(())
    }
    pub fn to_json_vec(&self) -> Result<Vec<u8>, RetestError> {
        encode_study(self, Self::verify)
    }
    pub fn from_json_slice(bytes: &[u8]) -> Result<Self, RetestError> {
        decode_study(bytes, Self::verify)
    }
    fn compute_id(&self) -> Result<String, RetestError> {
        study_identity(
            OOS_ARTIFACT_DOMAIN,
            &(
                self.schema_version,
                &self.candidate_id,
                &self.source_manifest_id,
                &self.source_dataset_id,
                &self.source_range,
                &self.config_id,
                &self.scheme,
                self.purge_bars,
                self.embargo_bars,
                &self.metric_id,
                self.root_seed,
                &self.membership,
                &self.seams,
                &self.executed_partitions,
            ),
        )
    }
}

/// Execute every IS/OOS partition of a declared scheme against child manifests rebuilt from the
/// exact bars admitted by `source_lease`. Purged and embargoed bars are evidence only and never
/// enter a simulator run.
pub fn execute_oos_scheme(
    strategy: &StrategyIr,
    config: &StrategyExecutionConfig,
    source_manifest: &DatasetManifest,
    bars: &[Bar],
    source_lease: SearchDataLease,
    spec: OosExecutionSpec,
) -> Result<ExecutedOosScheme, RetestError> {
    validate_source_content(source_manifest, bars, &source_lease)?;
    if spec.metric_id.trim().is_empty() {
        return Err(RetestError::Invalid("metric id is empty".into()));
    }
    let scheme = spec.scheme.clone();
    let plan = OosPlan::new(bars.len(), spec.scheme, spec.purge_bars, spec.embargo_bars)?;
    let source_start = source_lease.range().start;
    let membership = plan
        .roles()
        .iter()
        .enumerate()
        .map(|(index, role)| ExactBarMember {
            source_index: source_start + index,
            timestamp: bars[index].timestamp.clone(),
            role: *role,
        })
        .collect();
    let seams = plan
        .role_ranges(SampleRole::OutOfSample)
        .iter()
        .map(|range| OosSeamProof {
            purged: source_start + range.start.saturating_sub(spec.purge_bars)
                ..source_start + range.start,
            oos: source_start + range.start..source_start + range.end,
            embargoed: source_start + range.end
                ..source_start + range.end.saturating_add(spec.embargo_bars).min(bars.len()),
        })
        .collect();
    let mut executed_partitions = Vec::new();
    for role in [SampleRole::InSample, SampleRole::OutOfSample] {
        for range in plan.role_ranges(role) {
            let observation_role = match role {
                SampleRole::InSample => ObservationRole::InSample,
                SampleRole::OutOfSample => ObservationRole::OutOfSample,
                _ => unreachable!(),
            };
            let run = execute_partition(
                strategy,
                config,
                source_manifest,
                bars,
                &source_lease,
                range.clone(),
                observation_role,
                &spec.metric_id,
                spec.root_seed,
            )?;
            executed_partitions.push(ExecutedPartition {
                role,
                range: run.global_range,
                dataset_id: run.dataset_id,
                run_id: run.report.run_id().to_string(),
                report_id: run.report.report_id().to_string(),
                score: run.score,
            });
        }
    }
    executed_partitions.sort_by_key(|partition| {
        (
            partition.range.start,
            partition.range.end,
            sample_role_key(partition.role),
        )
    });
    let mut artifact = ExecutedOosScheme {
        schema_version: STUDY_ARTIFACT_SCHEMA_VERSION,
        artifact_id: String::new(),
        candidate_id: strategy.strategy_id().to_string(),
        source_manifest_id: source_manifest.manifest_id.clone(),
        source_dataset_id: source_manifest.dataset_id.clone(),
        source_range: source_lease.range(),
        config_id: config.config_id().to_string(),
        scheme,
        purge_bars: spec.purge_bars,
        embargo_bars: spec.embargo_bars,
        metric_id: spec.metric_id,
        root_seed: spec.root_seed,
        membership,
        seams,
        executed_partitions,
    };
    artifact.artifact_id = artifact.compute_id()?;
    artifact.verify()?;
    Ok(artifact)
}

#[derive(Debug, Clone, PartialEq)]
pub struct WalkForwardOptimizationSpec {
    pub config: WalkForwardConfig,
    pub minimum_windows: usize,
    pub metric_id: String,
    pub direction: ObjectiveDirection,
    pub root_seed: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DegradationObservation {
    pub in_sample_score: f64,
    pub out_of_sample_score: f64,
    pub delta: f64,
    pub ratio_bps: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WalkForwardWindowResult {
    pub ordinal: usize,
    pub selected_candidate_id: String,
    pub evaluations_n: usize,
    pub in_sample: ExactBarMembership,
    pub purged: ExactBarMembership,
    pub embargoed: ExactBarMembership,
    pub out_of_sample: ExactBarMembership,
    pub in_sample_run_id: String,
    pub in_sample_report_id: String,
    pub oos_run_id: String,
    pub oos_report_id: String,
    pub degradation: DegradationObservation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConcatenatedOosResult {
    pub ranges: Vec<Range<usize>>,
    pub run_ids: Vec<String>,
    pub report_ids: Vec<String>,
    pub scores: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutedWalkForward {
    schema_version: u32,
    artifact_id: String,
    candidate_set_id: String,
    source_manifest_id: String,
    source_dataset_id: String,
    source_range: Range<usize>,
    config_id: String,
    walk_forward_config: WalkForwardConfig,
    minimum_windows: usize,
    metric_id: String,
    root_seed: u64,
    windows: Vec<WalkForwardWindowResult>,
    concatenated_oos: ConcatenatedOosResult,
    degradation_distribution: Vec<DegradationObservation>,
}
impl ExecutedWalkForward {
    pub fn artifact_id(&self) -> &str {
        &self.artifact_id
    }
    pub fn candidate_set_id(&self) -> &str {
        &self.candidate_set_id
    }
    pub fn source_dataset_id(&self) -> &str {
        &self.source_dataset_id
    }
    pub fn metric_id(&self) -> &str {
        &self.metric_id
    }
    pub fn windows(&self) -> &[WalkForwardWindowResult] {
        &self.windows
    }
    pub fn concatenated_oos(&self) -> &ConcatenatedOosResult {
        &self.concatenated_oos
    }
    pub fn degradation_distribution(&self) -> &[DegradationObservation] {
        &self.degradation_distribution
    }
    pub fn verify(&self) -> Result<(), RetestError> {
        let count = self.windows.len();
        if self.schema_version != STUDY_ARTIFACT_SCHEMA_VERSION
            || !is_id(&self.artifact_id)
            || !is_id(&self.candidate_set_id)
            || !is_id(&self.source_manifest_id)
            || !is_id(&self.source_dataset_id)
            || !is_id(&self.config_id)
            || self.source_range.start >= self.source_range.end
            || self.metric_id.trim().is_empty()
            || self.minimum_windows < 2
            || self.minimum_windows > MAX_STUDY_WINDOWS
            || count < self.minimum_windows
            || count > MAX_STUDY_WINDOWS
            || self.degradation_distribution.len() != count
            || self.concatenated_oos.ranges.len() != count
            || self.concatenated_oos.run_ids.len() != count
            || self.concatenated_oos.report_ids.len() != count
            || self.concatenated_oos.scores.len() != count
        {
            return Err(RetestError::Invalid(
                "invalid walk-forward artifact bounds".into(),
            ));
        }
        for (ordinal, window) in self.windows.iter().enumerate() {
            if window.ordinal != ordinal
                || !is_id(&window.selected_candidate_id)
                || window.evaluations_n == 0
                || window.evaluations_n > MAX_TRIAL_BUDGET
                || !is_id(&window.in_sample_run_id)
                || !is_id(&window.in_sample_report_id)
                || !is_id(&window.oos_run_id)
                || !is_id(&window.oos_report_id)
                || !window.degradation.in_sample_score.is_finite()
                || !window.degradation.out_of_sample_score.is_finite()
                || !window.degradation.delta.is_finite()
                || self.concatenated_oos.run_ids[ordinal] != window.oos_run_id
                || self.concatenated_oos.report_ids[ordinal] != window.oos_report_id
                || self.concatenated_oos.scores[ordinal] != window.degradation.out_of_sample_score
                || self.degradation_distribution[ordinal] != window.degradation
                || self.concatenated_oos.ranges[ordinal]
                    != range_from_membership(&window.out_of_sample)?
            {
                return Err(RetestError::Invalid(
                    "invalid walk-forward evidence binding".into(),
                ));
            }
            verify_membership(&window.in_sample, &self.source_range)?;
            verify_membership(&window.purged, &self.source_range)?;
            verify_membership(&window.embargoed, &self.source_range)?;
            verify_membership(&window.out_of_sample, &self.source_range)?;
        }
        if self.compute_id()? != self.artifact_id {
            return Err(RetestError::Invalid(
                "walk-forward identity mismatch".into(),
            ));
        }
        Ok(())
    }
    pub fn to_json_vec(&self) -> Result<Vec<u8>, RetestError> {
        encode_study(self, Self::verify)
    }
    pub fn from_json_slice(bytes: &[u8]) -> Result<Self, RetestError> {
        decode_study(bytes, Self::verify)
    }
    fn compute_id(&self) -> Result<String, RetestError> {
        study_identity(
            WALK_FORWARD_ARTIFACT_DOMAIN,
            &(
                self.schema_version,
                &self.candidate_set_id,
                &self.source_manifest_id,
                &self.source_dataset_id,
                &self.source_range,
                &self.config_id,
                &self.walk_forward_config,
                self.minimum_windows,
                &self.metric_id,
                self.root_seed,
                &self.windows,
                &self.concatenated_oos,
                &self.degradation_distribution,
            ),
        )
    }
}

/// Re-optimize every exact IS window from report-derived observations, then execute only the
/// selected canonical candidate on the following OOS content. No metric can enter from callers.
pub fn execute_walk_forward_optimization(
    config: &StrategyExecutionConfig,
    source_manifest: &DatasetManifest,
    bars: &[Bar],
    source_lease: SearchDataLease,
    candidates: &SearchBatch,
    spec: WalkForwardOptimizationSpec,
) -> Result<ExecutedWalkForward, RetestError> {
    validate_source_content(source_manifest, bars, &source_lease)?;
    validate_candidate_batch(candidates)?;
    if spec.metric_id.trim().is_empty()
        || spec.minimum_windows < 2
        || spec.minimum_windows > MAX_STUDY_WINDOWS
    {
        return Err(RetestError::Invalid(
            "invalid walk-forward observation contract".into(),
        ));
    }
    let candidate_set_id = study_identity(
        b"typhoon.strategy_retest.candidate_set.v1",
        &(
            candidates.evaluations_n,
            candidates
                .candidates
                .iter()
                .map(|candidate| candidate.candidate_id.as_str())
                .collect::<Vec<_>>(),
        ),
    )?;
    let walk_forward_config = spec.config;
    let plan = FoldPlan::walk_forward(bars.len(), spec.config)?;
    if plan.folds().len() < spec.minimum_windows {
        return Err(RetestError::Invalid(
            "insufficient complete walk-forward windows".into(),
        ));
    }
    let objective = ObjectiveSpec::new(&spec.metric_id, spec.direction)?;
    let source_start = source_lease.range().start;
    let mut windows = Vec::with_capacity(plan.folds().len());
    for (ordinal, fold) in plan.folds().iter().enumerate() {
        let mut candidate_runs = Vec::with_capacity(candidates.candidates.len());
        for candidate in &candidates.candidates {
            candidate_runs.push(execute_partition(
                &candidate.strategy,
                config,
                source_manifest,
                bars,
                &source_lease,
                fold.train.clone(),
                ObservationRole::SearchEvaluation,
                &spec.metric_id,
                spec.root_seed,
            )?);
        }
        let observations = candidate_runs
            .iter()
            .map(|run| run.observation.clone())
            .collect::<Vec<_>>();
        let best = select_best(&observations, &objective)?;
        let selected_index = candidate_runs
            .iter()
            .position(|run| run.observation.candidate_id() == best.candidate_id())
            .ok_or_else(|| RetestError::Invalid("selected candidate is absent".into()))?;
        let selected_is = &candidate_runs[selected_index];
        let selected = &candidates.candidates[selected_index];
        let selected_oos = execute_partition(
            &selected.strategy,
            config,
            source_manifest,
            bars,
            &source_lease,
            fold.test.clone(),
            ObservationRole::OutOfSample,
            &spec.metric_id,
            spec.root_seed,
        )?;
        let purge_start = fold.train.end;
        let purge_end = purge_start
            .checked_add(spec.config.purge_bars)
            .ok_or_else(|| RetestError::Invalid("walk-forward purge overflow".into()))?;
        let embargo_end = purge_end
            .checked_add(spec.config.embargo_bars)
            .ok_or_else(|| RetestError::Invalid("walk-forward embargo overflow".into()))?;
        if embargo_end != fold.test.start {
            return Err(RetestError::Invalid(
                "walk-forward fold does not prove its purge/embargo seam".into(),
            ));
        }
        let degradation = degradation(selected_is.score, selected_oos.score)?;
        windows.push(WalkForwardWindowResult {
            ordinal,
            selected_candidate_id: selected.candidate_id.clone(),
            evaluations_n: candidates.evaluations_n,
            in_sample: exact_membership(
                SampleRole::InSample,
                std::slice::from_ref(&fold.train),
                bars,
                source_start,
            ),
            purged: exact_membership(
                SampleRole::Purged,
                &[purge_start..purge_end],
                bars,
                source_start,
            ),
            embargoed: exact_membership(
                SampleRole::Embargoed,
                &[purge_end..embargo_end],
                bars,
                source_start,
            ),
            out_of_sample: exact_membership(
                SampleRole::OutOfSample,
                std::slice::from_ref(&fold.test),
                bars,
                source_start,
            ),
            in_sample_run_id: selected_is.report.run_id().to_string(),
            in_sample_report_id: selected_is.report.report_id().to_string(),
            oos_run_id: selected_oos.report.run_id().to_string(),
            oos_report_id: selected_oos.report.report_id().to_string(),
            degradation,
        });
    }
    let concatenated_oos = ConcatenatedOosResult {
        ranges: windows
            .iter()
            .map(|window| range_from_membership(&window.out_of_sample))
            .collect::<Result<_, _>>()?,
        run_ids: windows
            .iter()
            .map(|window| window.oos_run_id.clone())
            .collect(),
        report_ids: windows
            .iter()
            .map(|window| window.oos_report_id.clone())
            .collect(),
        scores: windows
            .iter()
            .map(|window| window.degradation.out_of_sample_score)
            .collect(),
    };
    let degradation_distribution = windows
        .iter()
        .map(|window| window.degradation.clone())
        .collect();
    let mut artifact = ExecutedWalkForward {
        schema_version: STUDY_ARTIFACT_SCHEMA_VERSION,
        artifact_id: String::new(),
        candidate_set_id,
        source_manifest_id: source_manifest.manifest_id.clone(),
        source_dataset_id: source_manifest.dataset_id.clone(),
        source_range: source_lease.range(),
        config_id: config.config_id().to_string(),
        walk_forward_config,
        minimum_windows: spec.minimum_windows,
        metric_id: spec.metric_id,
        root_seed: spec.root_seed,
        windows,
        concatenated_oos,
        degradation_distribution,
    };
    artifact.artifact_id = artifact.compute_id()?;
    artifact.verify()?;
    Ok(artifact)
}

#[derive(Debug, Clone, PartialEq)]
pub struct CalendarWalkForwardOptimizationSpec {
    pub config: CalendarWalkForwardConfig,
    pub minimum_windows: usize,
    pub metric_id: String,
    pub direction: ObjectiveDirection,
    pub root_seed: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalendarWalkForwardWindowResult {
    pub ordinal: usize,
    pub bounds: CalendarWindowBounds,
    pub selected_candidate_id: String,
    pub evaluations_n: usize,
    pub in_sample: ExactBarMembership,
    pub purged: ExactBarMembership,
    pub embargoed: ExactBarMembership,
    pub out_of_sample: ExactBarMembership,
    pub in_sample_run_id: String,
    pub in_sample_report_id: String,
    pub oos_run_id: String,
    pub oos_report_id: String,
    pub degradation: DegradationObservation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutedCalendarWalkForward {
    schema_version: u32,
    artifact_id: String,
    candidate_set_id: String,
    source_manifest_id: String,
    source_dataset_id: String,
    source_range: Range<usize>,
    config_id: String,
    calendar_config: CalendarWalkForwardConfig,
    minimum_windows: usize,
    metric_id: String,
    root_seed: u64,
    windows: Vec<CalendarWalkForwardWindowResult>,
    concatenated_oos: ConcatenatedOosResult,
    degradation_distribution: Vec<DegradationObservation>,
}

impl ExecutedCalendarWalkForward {
    pub fn artifact_id(&self) -> &str {
        &self.artifact_id
    }
    pub fn source_dataset_id(&self) -> &str {
        &self.source_dataset_id
    }
    pub fn windows(&self) -> &[CalendarWalkForwardWindowResult] {
        &self.windows
    }
    pub fn concatenated_oos(&self) -> &ConcatenatedOosResult {
        &self.concatenated_oos
    }
    pub fn degradation_distribution(&self) -> &[DegradationObservation] {
        &self.degradation_distribution
    }
    pub fn verify(&self) -> Result<(), RetestError> {
        self.calendar_config.validate()?;
        let count = self.windows.len();
        if self.schema_version != STUDY_ARTIFACT_SCHEMA_VERSION
            || !is_id(&self.artifact_id)
            || !is_id(&self.candidate_set_id)
            || !is_id(&self.source_manifest_id)
            || !is_id(&self.source_dataset_id)
            || !is_id(&self.config_id)
            || self.source_range.is_empty()
            || self.metric_id.trim().is_empty()
            || self.minimum_windows < 2
            || self.minimum_windows > MAX_STUDY_WINDOWS
            || count < self.minimum_windows
            || count > MAX_STUDY_WINDOWS
            || self.degradation_distribution.len() != count
            || self.concatenated_oos.ranges.len() != count
            || self.concatenated_oos.run_ids.len() != count
            || self.concatenated_oos.report_ids.len() != count
            || self.concatenated_oos.scores.len() != count
        {
            return Err(RetestError::Invalid(
                "invalid calendar walk-forward artifact bounds".into(),
            ));
        }
        let mut previous_test_end = None;
        for (ordinal, window) in self.windows.iter().enumerate() {
            if window.ordinal != ordinal
                || !is_id(&window.selected_candidate_id)
                || window.evaluations_n == 0
                || window.evaluations_n > MAX_TRIAL_BUDGET
                || !is_id(&window.in_sample_run_id)
                || !is_id(&window.in_sample_report_id)
                || !is_id(&window.oos_run_id)
                || !is_id(&window.oos_report_id)
                || !window.degradation.in_sample_score.is_finite()
                || !window.degradation.out_of_sample_score.is_finite()
                || !window.degradation.delta.is_finite()
                || self.concatenated_oos.run_ids[ordinal] != window.oos_run_id
                || self.concatenated_oos.report_ids[ordinal] != window.oos_report_id
                || self.concatenated_oos.scores[ordinal] != window.degradation.out_of_sample_score
                || self.degradation_distribution[ordinal] != window.degradation
                || self.concatenated_oos.ranges[ordinal]
                    != range_from_membership(&window.out_of_sample)?
            {
                return Err(RetestError::Invalid(
                    "invalid calendar walk-forward evidence binding".into(),
                ));
            }
            let boundaries = verify_calendar_bounds(&window.bounds)?;
            if window.in_sample.role != SampleRole::InSample
                || window.purged.role != SampleRole::Purged
                || window.embargoed.role != SampleRole::Embargoed
                || window.out_of_sample.role != SampleRole::OutOfSample
            {
                return Err(RetestError::Invalid(
                    "calendar membership role mismatch".into(),
                ));
            }
            if previous_test_end.is_some_and(|previous| previous > boundaries[3]) {
                return Err(RetestError::Invalid(
                    "calendar OOS windows overlap or regress".into(),
                ));
            }
            previous_test_end = Some(boundaries[4]);
            for membership in [
                &window.in_sample,
                &window.purged,
                &window.embargoed,
                &window.out_of_sample,
            ] {
                verify_membership(membership, &self.source_range)?;
            }
            verify_calendar_membership(&window.in_sample, boundaries[0], boundaries[1])?;
            verify_calendar_membership(&window.purged, boundaries[1], boundaries[2])?;
            verify_calendar_membership(&window.embargoed, boundaries[2], boundaries[3])?;
            verify_calendar_membership(&window.out_of_sample, boundaries[3], boundaries[4])?;
        }
        if self.compute_id()? != self.artifact_id {
            return Err(RetestError::Invalid(
                "calendar walk-forward identity mismatch".into(),
            ));
        }
        Ok(())
    }
    pub fn to_json_vec(&self) -> Result<Vec<u8>, RetestError> {
        encode_study(self, Self::verify)
    }
    pub fn from_json_slice(bytes: &[u8]) -> Result<Self, RetestError> {
        decode_study(bytes, Self::verify)
    }
    fn compute_id(&self) -> Result<String, RetestError> {
        study_identity(
            CALENDAR_WALK_FORWARD_ARTIFACT_DOMAIN,
            &(
                self.schema_version,
                &self.candidate_set_id,
                &self.source_manifest_id,
                &self.source_dataset_id,
                &self.source_range,
                &self.config_id,
                &self.calendar_config,
                self.minimum_windows,
                &self.metric_id,
                self.root_seed,
                &self.windows,
                &self.concatenated_oos,
                &self.degradation_distribution,
            ),
        )
    }
}

pub fn execute_calendar_walk_forward_optimization(
    config: &StrategyExecutionConfig,
    source_manifest: &DatasetManifest,
    bars: &[Bar],
    source_lease: SearchDataLease,
    candidates: &SearchBatch,
    spec: CalendarWalkForwardOptimizationSpec,
) -> Result<ExecutedCalendarWalkForward, RetestError> {
    validate_source_content(source_manifest, bars, &source_lease)?;
    validate_candidate_batch(candidates)?;
    if spec.metric_id.trim().is_empty()
        || spec.minimum_windows < 2
        || spec.minimum_windows > MAX_STUDY_WINDOWS
    {
        return Err(RetestError::Invalid(
            "invalid calendar walk-forward observation contract".into(),
        ));
    }
    let candidate_set_id = candidate_set_id(candidates)?;
    let timestamps = bars
        .iter()
        .map(|bar| bar.timestamp.clone())
        .collect::<Vec<_>>();
    let plan = CalendarFoldPlan::walk_forward(&timestamps, spec.config)?;
    if plan.folds().len() < spec.minimum_windows {
        return Err(RetestError::Invalid(
            "insufficient complete calendar walk-forward windows".into(),
        ));
    }
    let objective = ObjectiveSpec::new(&spec.metric_id, spec.direction)?;
    let source_start = source_lease.range().start;
    let mut windows = Vec::with_capacity(plan.folds().len());
    for (ordinal, fold) in plan.folds().iter().enumerate() {
        let mut candidate_runs = Vec::with_capacity(candidates.candidates.len());
        for candidate in &candidates.candidates {
            candidate_runs.push(execute_partition(
                &candidate.strategy,
                config,
                source_manifest,
                bars,
                &source_lease,
                fold.train.clone(),
                ObservationRole::SearchEvaluation,
                &spec.metric_id,
                spec.root_seed,
            )?);
        }
        let observations = candidate_runs
            .iter()
            .map(|run| run.observation.clone())
            .collect::<Vec<_>>();
        let best = select_best(&observations, &objective)?;
        let selected_index = candidate_runs
            .iter()
            .position(|run| run.observation.candidate_id() == best.candidate_id())
            .ok_or_else(|| RetestError::Invalid("selected candidate is absent".into()))?;
        let selected_is = &candidate_runs[selected_index];
        let selected = &candidates.candidates[selected_index];
        let selected_oos = execute_partition(
            &selected.strategy,
            config,
            source_manifest,
            bars,
            &source_lease,
            fold.test.clone(),
            ObservationRole::OutOfSample,
            &spec.metric_id,
            spec.root_seed,
        )?;
        let degradation = degradation(selected_is.score, selected_oos.score)?;
        windows.push(CalendarWalkForwardWindowResult {
            ordinal,
            bounds: fold.bounds.clone(),
            selected_candidate_id: selected.candidate_id.clone(),
            evaluations_n: candidates.evaluations_n,
            in_sample: exact_membership(
                SampleRole::InSample,
                std::slice::from_ref(&fold.train),
                bars,
                source_start,
            ),
            purged: exact_membership(
                SampleRole::Purged,
                std::slice::from_ref(&fold.purged),
                bars,
                source_start,
            ),
            embargoed: exact_membership(
                SampleRole::Embargoed,
                std::slice::from_ref(&fold.embargoed),
                bars,
                source_start,
            ),
            out_of_sample: exact_membership(
                SampleRole::OutOfSample,
                std::slice::from_ref(&fold.test),
                bars,
                source_start,
            ),
            in_sample_run_id: selected_is.report.run_id().to_string(),
            in_sample_report_id: selected_is.report.report_id().to_string(),
            oos_run_id: selected_oos.report.run_id().to_string(),
            oos_report_id: selected_oos.report.report_id().to_string(),
            degradation,
        });
    }
    let concatenated_oos = ConcatenatedOosResult {
        ranges: windows
            .iter()
            .map(|window| range_from_membership(&window.out_of_sample))
            .collect::<Result<_, _>>()?,
        run_ids: windows
            .iter()
            .map(|window| window.oos_run_id.clone())
            .collect(),
        report_ids: windows
            .iter()
            .map(|window| window.oos_report_id.clone())
            .collect(),
        scores: windows
            .iter()
            .map(|window| window.degradation.out_of_sample_score)
            .collect(),
    };
    let degradation_distribution = windows
        .iter()
        .map(|window| window.degradation.clone())
        .collect();
    let mut artifact = ExecutedCalendarWalkForward {
        schema_version: STUDY_ARTIFACT_SCHEMA_VERSION,
        artifact_id: String::new(),
        candidate_set_id,
        source_manifest_id: source_manifest.manifest_id.clone(),
        source_dataset_id: source_manifest.dataset_id.clone(),
        source_range: source_lease.range(),
        config_id: config.config_id().to_string(),
        calendar_config: spec.config,
        minimum_windows: spec.minimum_windows,
        metric_id: spec.metric_id,
        root_seed: spec.root_seed,
        windows,
        concatenated_oos,
        degradation_distribution,
    };
    artifact.artifact_id = artifact.compute_id()?;
    artifact.verify()?;
    Ok(artifact)
}

fn candidate_set_id(candidates: &SearchBatch) -> Result<String, RetestError> {
    study_identity(
        b"typhoon.strategy_retest.candidate_set.v1",
        &(
            candidates.evaluations_n,
            candidates
                .candidates
                .iter()
                .map(|candidate| candidate.candidate_id.as_str())
                .collect::<Vec<_>>(),
        ),
    )
}

#[derive(Debug, Clone, PartialEq)]
pub struct WalkForwardMatrixSpec {
    pub dimensions: Vec<(usize, usize)>,
    pub step_bars: usize,
    pub purge_bars: usize,
    pub embargo_bars: usize,
    pub anchored: bool,
    pub minimum_windows: usize,
    pub metric_id: String,
    pub direction: ObjectiveDirection,
    pub root_seed: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutedWalkForwardMatrixCell {
    pub train_bars: usize,
    pub test_bars: usize,
    pub result: ExecutedWalkForward,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutedWalkForwardMatrix {
    schema_version: u32,
    artifact_id: String,
    candidate_set_id: String,
    source_dataset_id: String,
    cells: Vec<ExecutedWalkForwardMatrixCell>,
}
impl ExecutedWalkForwardMatrix {
    pub fn artifact_id(&self) -> &str {
        &self.artifact_id
    }
    pub fn source_dataset_id(&self) -> &str {
        &self.source_dataset_id
    }
    pub fn candidate_set_id(&self) -> &str {
        &self.candidate_set_id
    }
    pub fn cells(&self) -> &[ExecutedWalkForwardMatrixCell] {
        &self.cells
    }
    pub fn verify(&self) -> Result<(), RetestError> {
        if self.schema_version != STUDY_ARTIFACT_SCHEMA_VERSION
            || !is_id(&self.artifact_id)
            || !is_id(&self.candidate_set_id)
            || !is_id(&self.source_dataset_id)
            || self.cells.is_empty()
            || self.cells.len() > MAX_WALK_FORWARD_MATRIX_CELLS
            || self.cells.windows(2).any(|pair| {
                (pair[0].train_bars, pair[0].test_bars) >= (pair[1].train_bars, pair[1].test_bars)
            })
        {
            return Err(RetestError::Invalid(
                "invalid walk-forward matrix bounds".into(),
            ));
        }
        for cell in &self.cells {
            cell.result.verify()?;
            if cell.train_bars == 0
                || cell.test_bars == 0
                || cell.result.walk_forward_config.train_bars != cell.train_bars
                || cell.result.walk_forward_config.test_bars != cell.test_bars
                || cell.result.source_dataset_id != self.source_dataset_id
                || cell.result.candidate_set_id != self.candidate_set_id
            {
                return Err(RetestError::Invalid(
                    "inconsistent walk-forward matrix cell".into(),
                ));
            }
        }
        if self.compute_id()? != self.artifact_id {
            return Err(RetestError::Invalid(
                "walk-forward matrix identity mismatch".into(),
            ));
        }
        Ok(())
    }
    pub fn to_json_vec(&self) -> Result<Vec<u8>, RetestError> {
        encode_study(self, Self::verify)
    }
    pub fn from_json_slice(bytes: &[u8]) -> Result<Self, RetestError> {
        decode_study(bytes, Self::verify)
    }
    fn compute_id(&self) -> Result<String, RetestError> {
        study_identity(
            WALK_FORWARD_MATRIX_ARTIFACT_DOMAIN,
            &(
                self.schema_version,
                &self.candidate_set_id,
                &self.source_dataset_id,
                &self.cells,
            ),
        )
    }
}

pub fn execute_walk_forward_matrix(
    config: &StrategyExecutionConfig,
    source_manifest: &DatasetManifest,
    bars: &[Bar],
    source_lease: SearchDataLease,
    candidates: &SearchBatch,
    mut spec: WalkForwardMatrixSpec,
) -> Result<ExecutedWalkForwardMatrix, RetestError> {
    validate_source_content(source_manifest, bars, &source_lease)?;
    if spec.dimensions.is_empty() || spec.dimensions.len() > MAX_WALK_FORWARD_MATRIX_CELLS {
        return Err(RetestError::Invalid(
            "walk-forward matrix exceeds its cell bound".into(),
        ));
    }
    spec.dimensions.sort_unstable();
    if spec.dimensions.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(RetestError::Invalid(
            "walk-forward matrix contains duplicate dimensions".into(),
        ));
    }
    let stage = source_lease.stage();
    let source_range = source_lease.range();
    let mut cells = Vec::with_capacity(spec.dimensions.len());
    for (train_bars, test_bars) in spec.dimensions {
        let lease = SearchDataLease::exact_partition(
            stage,
            source_manifest.dataset_id.clone(),
            source_range.clone(),
        )?;
        let result = execute_walk_forward_optimization(
            config,
            source_manifest,
            bars,
            lease,
            candidates,
            WalkForwardOptimizationSpec {
                config: WalkForwardConfig {
                    train_bars,
                    test_bars,
                    step_bars: spec.step_bars,
                    purge_bars: spec.purge_bars,
                    embargo_bars: spec.embargo_bars,
                    anchored: spec.anchored,
                },
                minimum_windows: spec.minimum_windows,
                metric_id: spec.metric_id.clone(),
                direction: spec.direction,
                root_seed: spec.root_seed,
            },
        )?;
        cells.push(ExecutedWalkForwardMatrixCell {
            train_bars,
            test_bars,
            result,
        });
    }
    let first = cells
        .first()
        .ok_or_else(|| RetestError::Invalid("empty walk-forward matrix".into()))?;
    let mut artifact = ExecutedWalkForwardMatrix {
        schema_version: STUDY_ARTIFACT_SCHEMA_VERSION,
        artifact_id: String::new(),
        candidate_set_id: first.result.candidate_set_id.clone(),
        source_dataset_id: first.result.source_dataset_id.clone(),
        cells,
    };
    artifact.artifact_id = artifact.compute_id()?;
    artifact.verify()?;
    Ok(artifact)
}

struct PartitionRun {
    global_range: Range<usize>,
    dataset_id: String,
    report: StrategyReportArtifact,
    observation: ReportObservation,
    score: f64,
}

#[allow(clippy::too_many_arguments)]
fn execute_partition(
    strategy: &StrategyIr,
    config: &StrategyExecutionConfig,
    source_manifest: &DatasetManifest,
    bars: &[Bar],
    source_lease: &SearchDataLease,
    local_range: Range<usize>,
    role: ObservationRole,
    metric_id: &str,
    root_seed: u64,
) -> Result<PartitionRun, RetestError> {
    if local_range.start >= local_range.end || local_range.end > bars.len() {
        return Err(RetestError::Invalid("invalid study partition range".into()));
    }
    let partition_bars = &bars[local_range.clone()];
    let partition_manifest =
        DatasetManifest::build(&source_manifest.to_input(), partition_bars).map_err(invalid)?;
    let source_start = source_lease.range().start;
    let global_range = source_start + local_range.start..source_start + local_range.end;
    let partition_lease = SearchDataLease::exact_partition(
        source_lease.stage(),
        partition_manifest.dataset_id.clone(),
        global_range.clone(),
    )?;
    let request = RetestExecutionRequest::seal(
        strategy,
        config,
        &partition_manifest,
        partition_bars,
        partition_lease,
        role,
        metric_id,
        root_seed,
    )?;
    let (report, observation, score) = execute_bound_observation(&request)?;
    Ok(PartitionRun {
        global_range,
        dataset_id: partition_manifest.dataset_id,
        report,
        observation,
        score,
    })
}

fn validate_source_content(
    source_manifest: &DatasetManifest,
    bars: &[Bar],
    source_lease: &SearchDataLease,
) -> Result<(), RetestError> {
    source_manifest.verify(bars).map_err(invalid)?;
    let range = source_lease.range();
    if !matches!(
        source_lease.stage(),
        StageAccess::Search | StageAccess::Robustness
    ) || source_lease.dataset_id() != source_manifest.dataset_id
        || range.start >= range.end
        || range.len() != bars.len()
    {
        return Err(RetestError::Invalid(
            "source lease and exact content disagree".into(),
        ));
    }
    Ok(())
}

fn validate_candidate_batch(candidates: &SearchBatch) -> Result<(), RetestError> {
    if candidates.candidates.is_empty()
        || candidates.evaluations_n != candidates.candidates.len()
        || candidates.evaluations_n > crate::core::strategy_optimization::MAX_TRIAL_BUDGET
    {
        return Err(RetestError::Invalid(
            "invalid candidate evaluation set".into(),
        ));
    }
    let mut ids = BTreeSet::new();
    for candidate in &candidates.candidates {
        candidate.strategy.verify().map_err(invalid)?;
        if candidate.candidate_id != candidate.strategy.strategy_id()
            || !ids.insert(candidate.candidate_id.as_str())
        {
            return Err(RetestError::Invalid(
                "candidate set identity mismatch".into(),
            ));
        }
    }
    Ok(())
}

fn exact_membership(
    role: SampleRole,
    ranges: &[Range<usize>],
    bars: &[Bar],
    source_start: usize,
) -> ExactBarMembership {
    let mut indices = Vec::new();
    let mut timestamps = Vec::new();
    let global_ranges = ranges
        .iter()
        .filter(|range| !range.is_empty())
        .map(|range| {
            for index in range.clone() {
                indices.push(source_start + index);
                timestamps.push(bars[index].timestamp.clone());
            }
            source_start + range.start..source_start + range.end
        })
        .collect();
    ExactBarMembership {
        role,
        ranges: global_ranges,
        indices,
        timestamps,
    }
}

fn range_from_membership(membership: &ExactBarMembership) -> Result<Range<usize>, RetestError> {
    if membership.ranges.len() != 1 {
        return Err(RetestError::Invalid(
            "walk-forward OOS membership is not contiguous".into(),
        ));
    }
    Ok(membership.ranges[0].clone())
}

fn degradation(in_sample: f64, out_of_sample: f64) -> Result<DegradationObservation, RetestError> {
    if !in_sample.is_finite() || !out_of_sample.is_finite() {
        return Err(RetestError::Invalid("undefined degradation metric".into()));
    }
    let ratio_bps = if in_sample == 0.0 {
        None
    } else {
        let ratio = (out_of_sample / in_sample * 10_000.0).round();
        if ratio < f64::from(i32::MIN) || ratio > f64::from(i32::MAX) {
            return Err(RetestError::Invalid("degradation ratio overflow".into()));
        }
        Some(ratio as i32)
    };
    Ok(DegradationObservation {
        in_sample_score: in_sample,
        out_of_sample_score: out_of_sample,
        delta: out_of_sample - in_sample,
        ratio_bps,
    })
}

fn sample_role_key(role: SampleRole) -> u8 {
    match role {
        SampleRole::InSample => 0,
        SampleRole::OutOfSample => 1,
        SampleRole::Purged => 2,
        SampleRole::Embargoed => 3,
    }
}

fn is_id(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn study_identity(domain: &[u8], value: &impl Serialize) -> Result<String, RetestError> {
    let bytes = serde_json::to_vec(value).map_err(invalid)?;
    let mut hasher = Sha256::new();
    hasher.update(domain);
    frame(&mut hasher, &bytes);
    Ok(hex(hasher.finalize()))
}

fn encode_study<T: Serialize>(
    value: &T,
    verify: fn(&T) -> Result<(), RetestError>,
) -> Result<Vec<u8>, RetestError> {
    verify(value)?;
    let bytes = serde_json::to_vec(value).map_err(invalid)?;
    if bytes.len() > MAX_ARTIFACT_BYTES {
        return Err(RetestError::Invalid("study artifact is too large".into()));
    }
    Ok(bytes)
}

fn decode_study<T: for<'de> Deserialize<'de>>(
    bytes: &[u8],
    verify: fn(&T) -> Result<(), RetestError>,
) -> Result<T, RetestError> {
    if bytes.len() > MAX_ARTIFACT_BYTES {
        return Err(RetestError::Invalid("study artifact is too large".into()));
    }
    let value = serde_json::from_slice(bytes).map_err(invalid)?;
    verify(&value)?;
    Ok(value)
}

fn verify_membership(
    membership: &ExactBarMembership,
    source_range: &Range<usize>,
) -> Result<(), RetestError> {
    if membership.ranges.len() > MAX_STUDY_WINDOWS
        || membership.indices.len() != membership.timestamps.len()
        || membership.indices.len() > MAX_SEARCH_COMBINATIONS
        || membership
            .indices
            .iter()
            .any(|index| !source_range.contains(index))
        || membership
            .timestamps
            .iter()
            .any(|value| value.trim().is_empty())
        || membership.ranges.iter().any(|range| {
            range.start >= range.end
                || range.start < source_range.start
                || range.end > source_range.end
        })
        || membership.indices.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(RetestError::Invalid(
            "invalid exact study membership".into(),
        ));
    }
    let expanded = membership
        .ranges
        .iter()
        .flat_map(|range| range.clone())
        .collect::<Vec<_>>();
    if expanded != membership.indices {
        return Err(RetestError::Invalid(
            "membership range evidence disagrees".into(),
        ));
    }
    Ok(())
}

fn verify_calendar_bounds(
    bounds: &CalendarWindowBounds,
) -> Result<[chrono::DateTime<chrono::Utc>; 5], RetestError> {
    let parse = |value: &str| {
        chrono::DateTime::parse_from_rfc3339(value)
            .map(|instant| instant.with_timezone(&chrono::Utc))
            .map_err(|_| RetestError::Invalid("malformed calendar artifact boundary".into()))
    };
    let values = [
        parse(&bounds.train_start)?,
        parse(&bounds.train_end)?,
        parse(&bounds.purge_end)?,
        parse(&bounds.test_start)?,
        parse(&bounds.test_end)?,
    ];
    if values[0] >= values[1]
        || values[1] > values[2]
        || values[2] > values[3]
        || values[3] >= values[4]
    {
        return Err(RetestError::Invalid(
            "non-causal calendar artifact boundaries".into(),
        ));
    }
    Ok(values)
}

fn verify_calendar_membership(
    membership: &ExactBarMembership,
    start: chrono::DateTime<chrono::Utc>,
    end: chrono::DateTime<chrono::Utc>,
) -> Result<(), RetestError> {
    let mut previous = None;
    for timestamp in &membership.timestamps {
        let instant = chrono::DateTime::parse_from_rfc3339(timestamp)
            .map_err(|_| RetestError::Invalid("malformed exact membership timestamp".into()))?
            .with_timezone(&chrono::Utc);
        if instant < start || instant >= end || previous.is_some_and(|prior| prior >= instant) {
            return Err(RetestError::Invalid(
                "calendar membership falls outside its exact window".into(),
            ));
        }
        previous = Some(instant);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StudyArtifactKind {
    Oos,
    WalkForward,
    WalkForwardMatrix,
    TradeMonteCarlo,
    CalendarWalkForward,
    BayesianOptimization,
    Perturbation,
    ParameterField,
    CrossCheck,
    Significance,
    ProblemRecognition,
}
impl StudyArtifactKind {
    fn key(self) -> i64 {
        match self {
            Self::Oos => 0,
            Self::WalkForward => 1,
            Self::WalkForwardMatrix => 2,
            Self::TradeMonteCarlo => 3,
            Self::CalendarWalkForward => 4,
            Self::BayesianOptimization => 5,
            Self::Perturbation => 6,
            Self::ParameterField => 7,
            Self::CrossCheck => 8,
            Self::Significance => 9,
            Self::ProblemRecognition => 10,
        }
    }
    fn decode(value: i64) -> Result<Self, RetestError> {
        match value {
            0 => Ok(Self::Oos),
            1 => Ok(Self::WalkForward),
            2 => Ok(Self::WalkForwardMatrix),
            3 => Ok(Self::TradeMonteCarlo),
            4 => Ok(Self::CalendarWalkForward),
            5 => Ok(Self::BayesianOptimization),
            6 => Ok(Self::Perturbation),
            7 => Ok(Self::ParameterField),
            8 => Ok(Self::CrossCheck),
            9 => Ok(Self::Significance),
            10 => Ok(Self::ProblemRecognition),
            _ => Err(RetestError::Invalid("unknown study artifact kind".into())),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum StudyArtifact {
    Oos(ExecutedOosScheme),
    WalkForward(ExecutedWalkForward),
    WalkForwardMatrix(ExecutedWalkForwardMatrix),
    TradeMonteCarlo(TradeMonteCarloArtifact),
    CalendarWalkForward(ExecutedCalendarWalkForward),
    BayesianOptimization(BayesianStudyArtifact),
    Perturbation(PerturbationStudyArtifact),
    ParameterField(ParameterFieldStudyArtifact),
    CrossCheck(CrossCheckStudyArtifact),
    Significance(SignificanceStudyArtifact),
    ProblemRecognition(ProblemRecognitionArtifact),
}
impl StudyArtifact {
    fn decode(kind: StudyArtifactKind, bytes: &[u8]) -> Result<Self, RetestError> {
        Ok(match kind {
            StudyArtifactKind::Oos => Self::Oos(ExecutedOosScheme::from_json_slice(bytes)?),
            StudyArtifactKind::WalkForward => {
                Self::WalkForward(ExecutedWalkForward::from_json_slice(bytes)?)
            }
            StudyArtifactKind::WalkForwardMatrix => {
                Self::WalkForwardMatrix(ExecutedWalkForwardMatrix::from_json_slice(bytes)?)
            }
            StudyArtifactKind::TradeMonteCarlo => {
                Self::TradeMonteCarlo(TradeMonteCarloArtifact::from_json_slice(bytes)?)
            }
            StudyArtifactKind::CalendarWalkForward => {
                Self::CalendarWalkForward(ExecutedCalendarWalkForward::from_json_slice(bytes)?)
            }
            StudyArtifactKind::BayesianOptimization => {
                Self::BayesianOptimization(BayesianStudyArtifact::from_json_slice(bytes)?)
            }
            StudyArtifactKind::Perturbation => {
                Self::Perturbation(PerturbationStudyArtifact::from_json_slice(bytes)?)
            }
            StudyArtifactKind::ParameterField => {
                Self::ParameterField(ParameterFieldStudyArtifact::from_json_slice(bytes)?)
            }
            StudyArtifactKind::CrossCheck => {
                Self::CrossCheck(CrossCheckStudyArtifact::from_json_slice(bytes)?)
            }
            StudyArtifactKind::Significance => {
                Self::Significance(SignificanceStudyArtifact::from_json_slice(bytes)?)
            }
            StudyArtifactKind::ProblemRecognition => {
                Self::ProblemRecognition(ProblemRecognitionArtifact::from_json_slice(bytes)?)
            }
        })
    }
    fn artifact_id(&self) -> &str {
        match self {
            Self::Oos(value) => value.artifact_id(),
            Self::WalkForward(value) => value.artifact_id(),
            Self::WalkForwardMatrix(value) => value.artifact_id(),
            Self::TradeMonteCarlo(value) => value.artifact_id(),
            Self::CalendarWalkForward(value) => value.artifact_id(),
            Self::BayesianOptimization(value) => value.artifact_id(),
            Self::Perturbation(value) => value.artifact_id(),
            Self::ParameterField(value) => value.artifact_id(),
            Self::CrossCheck(value) => value.artifact_id(),
            Self::Significance(value) => value.artifact_id(),
            Self::ProblemRecognition(value) => value.artifact_id(),
        }
    }
    fn source_dataset_id(&self) -> &str {
        match self {
            Self::Oos(value) => value.source_dataset_id(),
            Self::WalkForward(value) => value.source_dataset_id(),
            Self::WalkForwardMatrix(value) => value.source_dataset_id(),
            Self::TradeMonteCarlo(value) => value.dataset_id(),
            Self::CalendarWalkForward(value) => value.source_dataset_id(),
            Self::BayesianOptimization(value) => value.source_dataset_id(),
            Self::Perturbation(value) => value.source_dataset_id(),
            Self::ParameterField(value) => value.source_dataset_id(),
            Self::CrossCheck(value) => value.source_dataset_id(),
            Self::Significance(value) => value.source_dataset_id(),
            Self::ProblemRecognition(value) => value.source_dataset_id(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StudyArtifactQuery {
    pub source_dataset_id: String,
    pub kind: Option<StudyArtifactKind>,
    pub after_sequence: Option<i64>,
    pub limit: usize,
}
#[derive(Debug, Clone, PartialEq)]
pub struct StudyArtifactRecord {
    pub artifact_id: String,
    pub source_dataset_id: String,
    pub kind: StudyArtifactKind,
    pub created_sequence: i64,
    pub artifact: StudyArtifact,
}
#[derive(Debug, Clone, PartialEq)]
pub struct StudyArtifactPage {
    pub records: Vec<StudyArtifactRecord>,
    pub has_more: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RetestEvidenceRecord {
    pub run_id: String,
    pub parent_run_id: Option<String>,
    pub candidate_id: String,
    pub evaluations_n: usize,
    pub robustness_verdict: StageVerdict,
    pub metric_id: String,
    pub metric_value: f64,
    pub range: Range<usize>,
    pub created_sequence: i64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetestEvidenceQuery {
    pub candidate_id: String,
    pub after_sequence: Option<i64>,
    pub limit: usize,
}
#[derive(Debug, Clone, PartialEq)]
pub struct RetestEvidencePage {
    pub records: Vec<RetestEvidenceRecord>,
    pub has_more: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoldoutConsumptionRecord {
    pub dataset_id: String,
    pub range: Range<usize>,
    pub reason: String,
    pub consumed_sequence: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinalHoldoutOutcome {
    Reserved,
    Succeeded,
    Failed,
}
#[derive(Debug, Clone, PartialEq)]
pub struct FinalHoldoutRecord {
    pub request_id: String,
    pub search_dataset_id: String,
    pub search_manifest_id: String,
    pub holdout_dataset_id: String,
    pub holdout_manifest_id: String,
    pub strategy_id: String,
    pub config_id: String,
    pub range: Range<usize>,
    pub seed: u64,
    pub reason: String,
    pub metric_id: String,
    pub metrics_version: String,
    pub evaluations_n: usize,
    pub reserved_sequence: i64,
    pub outcome: FinalHoldoutOutcome,
    pub run_id: Option<String>,
    pub report_id: Option<String>,
    pub metric_value: Option<f64>,
    pub failure: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinalHoldoutQuery {
    pub search_dataset_id: String,
    pub after_sequence: Option<i64>,
    pub limit: usize,
}
#[derive(Debug, Clone, PartialEq)]
pub struct FinalHoldoutPage {
    pub records: Vec<FinalHoldoutRecord>,
    pub has_more: bool,
}

/// Blocking store. GUI/native callers must own it on a bounded background worker, as with the M3
/// databank store; no method here is a render-thread API.
pub struct RetestEvidenceStore {
    connection: RefCell<Connection>,
}
impl RetestEvidenceStore {
    pub fn open(path: &Path) -> Result<Self, RetestError> {
        Self::initialize(Connection::open(path)?)
    }
    pub fn open_in_memory() -> Result<Self, RetestError> {
        Self::initialize(Connection::open_in_memory()?)
    }
    fn initialize(connection: Connection) -> Result<Self, RetestError> {
        connection.execute_batch(
            "PRAGMA foreign_keys=ON;
             PRAGMA journal_mode=WAL;
             PRAGMA busy_timeout=5000;
             CREATE TABLE IF NOT EXISTS retest_evidence(
               run_id TEXT PRIMARY KEY,
               request_id TEXT NOT NULL UNIQUE,
               parent_run_id TEXT,
               candidate_id TEXT NOT NULL,
               dataset_id TEXT NOT NULL,
               range_start INTEGER NOT NULL,
               range_end INTEGER NOT NULL,
               report_id TEXT NOT NULL UNIQUE,
               metric_id TEXT NOT NULL,
               metric_value REAL NOT NULL,
               evaluations_n INTEGER NOT NULL,
               robustness_id TEXT NOT NULL,
               robustness_verdict INTEGER NOT NULL,
               report_json BLOB NOT NULL,
               robustness_json BLOB NOT NULL,
               created_sequence INTEGER NOT NULL,
               UNIQUE(parent_run_id, run_id)
             ) WITHOUT ROWID;
             CREATE INDEX IF NOT EXISTS idx_retest_candidate_sequence
               ON retest_evidence(candidate_id, created_sequence, run_id);
             CREATE TABLE IF NOT EXISTS study_artifact(
               artifact_id TEXT PRIMARY KEY,
               source_dataset_id TEXT NOT NULL,
               kind INTEGER NOT NULL,
               artifact_json BLOB NOT NULL,
               created_sequence INTEGER NOT NULL
             ) WITHOUT ROWID;
             CREATE INDEX IF NOT EXISTS idx_study_dataset_kind_sequence
               ON study_artifact(source_dataset_id, kind, created_sequence, artifact_id);
             CREATE INDEX IF NOT EXISTS idx_study_dataset_sequence
               ON study_artifact(source_dataset_id, created_sequence, artifact_id);
             CREATE TABLE IF NOT EXISTS holdout_consumption(
               dataset_id TEXT PRIMARY KEY,
               range_start INTEGER NOT NULL,
               range_end INTEGER NOT NULL,
               reason TEXT NOT NULL,
               consumed_sequence INTEGER NOT NULL
             ) WITHOUT ROWID;
             CREATE INDEX IF NOT EXISTS idx_holdout_sequence
               ON holdout_consumption(dataset_id, consumed_sequence);
             CREATE TABLE IF NOT EXISTS final_holdout_execution(
               request_id TEXT PRIMARY KEY,
               search_dataset_id TEXT NOT NULL UNIQUE,
               search_manifest_id TEXT NOT NULL,
               holdout_dataset_id TEXT NOT NULL UNIQUE,
               holdout_manifest_id TEXT NOT NULL,
               strategy_id TEXT NOT NULL,
               config_id TEXT NOT NULL,
               range_start INTEGER NOT NULL,
               range_end INTEGER NOT NULL,
               seed INTEGER NOT NULL,
               reason TEXT NOT NULL,
               metric_id TEXT NOT NULL,
               metrics_version TEXT NOT NULL,
               evaluations_n INTEGER NOT NULL,
               reserved_sequence INTEGER NOT NULL,
               outcome TEXT NOT NULL CHECK(outcome IN ('reserved','succeeded','failed')),
               run_id TEXT,
               report_id TEXT,
               metric_value REAL,
               report_json BLOB,
               failure TEXT,
               CHECK((outcome='reserved' AND run_id IS NULL AND report_id IS NULL AND metric_value IS NULL AND report_json IS NULL AND failure IS NULL)
                  OR (outcome='succeeded' AND run_id IS NOT NULL AND report_id IS NOT NULL AND metric_value IS NOT NULL AND report_json IS NOT NULL AND failure IS NULL)
                  OR (outcome='failed' AND run_id IS NULL AND report_id IS NULL AND metric_value IS NULL AND report_json IS NULL AND length(failure)>0))
             ) WITHOUT ROWID;
             CREATE INDEX IF NOT EXISTS idx_final_holdout_search_sequence
               ON final_holdout_execution(search_dataset_id, reserved_sequence, request_id);
             CREATE TRIGGER IF NOT EXISTS immutable_final_holdout_identity
               BEFORE UPDATE ON final_holdout_execution
               WHEN OLD.outcome <> 'reserved'
                 OR NEW.request_id <> OLD.request_id
                 OR NEW.search_dataset_id <> OLD.search_dataset_id
                 OR NEW.search_manifest_id <> OLD.search_manifest_id
                 OR NEW.holdout_dataset_id <> OLD.holdout_dataset_id
                 OR NEW.holdout_manifest_id <> OLD.holdout_manifest_id
                 OR NEW.strategy_id <> OLD.strategy_id OR NEW.config_id <> OLD.config_id
                 OR NEW.range_start <> OLD.range_start OR NEW.range_end <> OLD.range_end
                 OR NEW.seed <> OLD.seed OR NEW.reason <> OLD.reason
                 OR NEW.metric_id <> OLD.metric_id OR NEW.metrics_version <> OLD.metrics_version
                 OR NEW.evaluations_n <> OLD.evaluations_n OR NEW.reserved_sequence <> OLD.reserved_sequence
               BEGIN SELECT RAISE(ABORT, 'final holdout evidence is immutable'); END;
             CREATE TRIGGER IF NOT EXISTS immutable_final_holdout_delete BEFORE DELETE ON final_holdout_execution BEGIN SELECT RAISE(ABORT, 'final holdout evidence is immutable'); END;
             CREATE TRIGGER IF NOT EXISTS immutable_retest_update BEFORE UPDATE ON retest_evidence BEGIN SELECT RAISE(ABORT, 'retest evidence is immutable'); END;
             CREATE TRIGGER IF NOT EXISTS immutable_retest_delete BEFORE DELETE ON retest_evidence BEGIN SELECT RAISE(ABORT, 'retest evidence is immutable'); END;
             CREATE TRIGGER IF NOT EXISTS immutable_study_update BEFORE UPDATE ON study_artifact BEGIN SELECT RAISE(ABORT, 'study artifact is immutable'); END;
             CREATE TRIGGER IF NOT EXISTS immutable_study_delete BEFORE DELETE ON study_artifact BEGIN SELECT RAISE(ABORT, 'study artifact is immutable'); END;
             CREATE TRIGGER IF NOT EXISTS immutable_holdout_update BEFORE UPDATE ON holdout_consumption BEGIN SELECT RAISE(ABORT, 'holdout audit is immutable'); END;
             CREATE TRIGGER IF NOT EXISTS immutable_holdout_delete BEFORE DELETE ON holdout_consumption BEGIN SELECT RAISE(ABORT, 'holdout audit is immutable'); END;",
        )?;
        Ok(Self {
            connection: RefCell::new(connection),
        })
    }
    pub fn execute_final_holdout(
        &self,
        request: FinalHoldoutExecutionRequest,
        reserved_sequence: i64,
    ) -> Result<CompletedFinalHoldout, RetestError> {
        request.verify()?;
        {
            let mut connection = self.connection.borrow_mut();
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let result = transaction.execute(
                "INSERT INTO final_holdout_execution(request_id,search_dataset_id,search_manifest_id,holdout_dataset_id,holdout_manifest_id,strategy_id,config_id,range_start,range_end,seed,reason,metric_id,metrics_version,evaluations_n,reserved_sequence,outcome)
                 VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,'reserved')",
                params![
                    request.request_id,
                    request.search_manifest.dataset_id,
                    request.search_manifest.manifest_id,
                    request.holdout_manifest.dataset_id,
                    request.holdout_manifest.manifest_id,
                    request.strategy.strategy_id(),
                    request.config.config_id(),
                    to_i64(request.range.start)?,
                    to_i64(request.range.end)?,
                    u64_to_sqlite(request.seed),
                    request.reason,
                    request.metric_id,
                    METRICS_SCHEMA_VERSION,
                    to_i64(request.evaluations_n)?,
                    reserved_sequence,
                ],
            );
            if let Err(error) = result {
                if error.to_string().contains("UNIQUE constraint failed") {
                    return Err(RetestError::HoldoutAlreadyConsumed);
                }
                return Err(error.into());
            }
            transaction.commit()?;
        }

        let execution = (|| {
            let report = execute_verified_report(
                &request.strategy,
                &request.config,
                &request.holdout_manifest,
                &request.holdout_bars,
                request.seed,
            )?;
            let metric_value = match report
                .analysis()
                .metric(&request.metric_id)
                .ok_or_else(|| RetestError::Invalid("final-holdout metric is undefined".into()))?
            {
                MetricValue::Defined { value } if value.is_finite() => *value,
                _ => {
                    return Err(RetestError::Invalid(
                        "final-holdout metric is not finite numeric evidence".into(),
                    ));
                }
            };
            let report_json = report.to_json_vec().map_err(invalid)?;
            Ok((report, metric_value, report_json))
        })();
        match execution {
            Ok((report, metric_value, report_json)) => {
                self.connection.borrow().execute(
                    "UPDATE final_holdout_execution SET outcome='succeeded',run_id=?2,report_id=?3,metric_value=?4,report_json=?5 WHERE request_id=?1 AND outcome='reserved'",
                    params![request.request_id, report.run_id(), report.report_id(), metric_value, report_json],
                )?;
                Ok(CompletedFinalHoldout {
                    request_id: request.request_id,
                    strategy_id: request.strategy.strategy_id().to_string(),
                    config_id: request.config.config_id().to_string(),
                    report,
                    metric_id: request.metric_id,
                    metric_value,
                    evaluations_n: request.evaluations_n,
                })
            }
            Err(error) => {
                let failure = error.to_string();
                self.connection.borrow().execute(
                    "UPDATE final_holdout_execution SET outcome='failed',failure=?2 WHERE request_id=?1 AND outcome='reserved'",
                    params![request.request_id, failure],
                )?;
                Err(error)
            }
        }
    }

    pub fn query_final_holdouts(
        &self,
        query: &FinalHoldoutQuery,
    ) -> Result<FinalHoldoutPage, RetestError> {
        validate_page_query(&query.search_dataset_id, query.limit)?;
        let after = query.after_sequence.unwrap_or(i64::MIN);
        let fetch = to_i64(query.limit.saturating_add(1))?;
        let connection = self.connection.borrow();
        let mut statement = connection.prepare(
            "SELECT request_id,search_dataset_id,search_manifest_id,holdout_dataset_id,holdout_manifest_id,strategy_id,config_id,range_start,range_end,seed,reason,metric_id,metrics_version,evaluations_n,reserved_sequence,outcome,run_id,report_id,metric_value,failure,report_json
             FROM final_holdout_execution
             INDEXED BY idx_final_holdout_search_sequence
             WHERE search_dataset_id=?1 AND reserved_sequence>?2
             ORDER BY reserved_sequence,request_id LIMIT ?3",
        )?;
        let mapped =
            statement.query_map(params![query.search_dataset_id, after, fetch], |row| {
                let outcome: String = row.get(15)?;
                let report_json: Option<Vec<u8>> = row.get(20)?;
                Ok((
                    FinalHoldoutRecord {
                        request_id: row.get(0)?,
                        search_dataset_id: row.get(1)?,
                        search_manifest_id: row.get(2)?,
                        holdout_dataset_id: row.get(3)?,
                        holdout_manifest_id: row.get(4)?,
                        strategy_id: row.get(5)?,
                        config_id: row.get(6)?,
                        range: usize::try_from(row.get::<_, i64>(7)?).unwrap_or(usize::MAX)
                            ..usize::try_from(row.get::<_, i64>(8)?).unwrap_or(usize::MAX),
                        seed: sqlite_to_u64(row.get::<_, i64>(9)?),
                        reason: row.get(10)?,
                        metric_id: row.get(11)?,
                        metrics_version: row.get(12)?,
                        evaluations_n: usize::try_from(row.get::<_, i64>(13)?)
                            .unwrap_or(usize::MAX),
                        reserved_sequence: row.get(14)?,
                        outcome: match outcome.as_str() {
                            "reserved" => FinalHoldoutOutcome::Reserved,
                            "succeeded" => FinalHoldoutOutcome::Succeeded,
                            _ => FinalHoldoutOutcome::Failed,
                        },
                        run_id: row.get(16)?,
                        report_id: row.get(17)?,
                        metric_value: row.get(18)?,
                        failure: row.get(19)?,
                    },
                    report_json,
                ))
            })?;
        let mut records = Vec::new();
        for mapped_row in mapped {
            let (record, report_json) = mapped_row?;
            validate_final_holdout_record(&record, report_json.as_deref())?;
            records.push(record);
        }
        let has_more = records.len() > query.limit;
        records.truncate(query.limit);
        Ok(FinalHoldoutPage { records, has_more })
    }

    pub fn explain_final_holdout_query(
        &self,
        query: &FinalHoldoutQuery,
    ) -> Result<Vec<String>, RetestError> {
        validate_page_query(&query.search_dataset_id, query.limit)?;
        let connection = self.connection.borrow();
        let mut statement = connection.prepare(
            "EXPLAIN QUERY PLAN SELECT request_id FROM final_holdout_execution INDEXED BY idx_final_holdout_search_sequence WHERE search_dataset_id=?1 AND reserved_sequence>?2 ORDER BY reserved_sequence,request_id LIMIT ?3",
        )?;
        let rows = statement.query_map(
            params![
                query.search_dataset_id,
                query.after_sequence.unwrap_or(i64::MIN),
                to_i64(query.limit)?
            ],
            |row| row.get(3),
        )?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    #[cfg(test)]
    fn test_only_tamper_final_holdout_report(&self, request_id: &str) -> Result<(), RetestError> {
        self.connection
            .borrow()
            .execute_batch("DROP TRIGGER immutable_final_holdout_identity;")?;
        self.connection.borrow().execute(
            "UPDATE final_holdout_execution SET report_json=X'00' WHERE request_id=?1",
            params![request_id],
        )?;
        Ok(())
    }

    #[cfg(test)]
    fn test_only_tamper_final_holdout_identity(&self, request_id: &str) -> Result<(), RetestError> {
        self.connection
            .borrow()
            .execute_batch("DROP TRIGGER immutable_final_holdout_identity;")?;
        self.connection.borrow().execute(
            "UPDATE final_holdout_execution SET reason=reason||'x' WHERE request_id=?1",
            params![request_id],
        )?;
        Ok(())
    }

    #[cfg(test)]
    fn test_only_reserve_final_holdout(
        &self,
        request: &FinalHoldoutExecutionRequest,
        reserved_sequence: i64,
    ) -> Result<(), RetestError> {
        request.verify()?;
        self.connection.borrow().execute(
            "INSERT INTO final_holdout_execution(request_id,search_dataset_id,search_manifest_id,holdout_dataset_id,holdout_manifest_id,strategy_id,config_id,range_start,range_end,seed,reason,metric_id,metrics_version,evaluations_n,reserved_sequence,outcome)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,'reserved')",
            params![request.request_id,request.search_manifest.dataset_id,request.search_manifest.manifest_id,request.holdout_manifest.dataset_id,request.holdout_manifest.manifest_id,request.strategy.strategy_id(),request.config.config_id(),to_i64(request.range.start)?,to_i64(request.range.end)?,u64_to_sqlite(request.seed),request.reason,request.metric_id,METRICS_SCHEMA_VERSION,to_i64(request.evaluations_n)?,reserved_sequence],
        )?;
        Ok(())
    }

    #[cfg(test)]
    fn test_only_update_final_holdout(&self) -> Result<(), RetestError> {
        self.connection
            .borrow()
            .execute("UPDATE final_holdout_execution SET reason=reason||'x'", [])?;
        Ok(())
    }

    pub fn persist(
        &self,
        completed: &CompletedRetest,
        parent_run_id: Option<&str>,
        created_sequence: i64,
    ) -> Result<(), RetestError> {
        completed.report.verify().map_err(invalid)?;
        completed.robustness.verify()?;
        if completed.robustness.candidate_id() != completed.observation.candidate_id()
            || completed.robustness.evaluations_n() != completed.evaluations_n
            || parent_run_id.is_some_and(|id| id.trim().is_empty())
        {
            return Err(RetestError::Invalid(
                "inconsistent persisted evidence".into(),
            ));
        }
        let report_json = completed.report.to_json_vec().map_err(invalid)?;
        let robustness_json = completed.robustness.to_json_vec()?;
        let range = completed.observation.range();
        let dataset_id = completed
            .report
            .run_manifest()
            .and_then(|manifest| manifest.binding().datasets.first())
            .map(|binding| binding.dataset_id.as_str())
            .ok_or_else(|| RetestError::Invalid("report lacks primary dataset".into()))?;
        let result = self.connection.borrow().execute(
            "INSERT INTO retest_evidence(run_id,request_id,parent_run_id,candidate_id,dataset_id,range_start,range_end,report_id,metric_id,metric_value,evaluations_n,robustness_id,robustness_verdict,report_json,robustness_json,created_sequence)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",
            params![
                completed.run_id(),
                completed.request_id,
                parent_run_id,
                completed.observation.candidate_id(),
                dataset_id,
                to_i64(range.start)?,
                to_i64(range.end)?,
                completed.report.report_id(),
                completed.metric_id,
                completed.metric_value,
                to_i64(completed.evaluations_n)?,
                completed.robustness.artifact_id(),
                verdict_key(completed.robustness.verdict()),
                report_json,
                robustness_json,
                created_sequence,
            ],
        );
        match result {
            Ok(_) => Ok(()),
            Err(error) if error.to_string().contains("UNIQUE constraint failed") => {
                Err(RetestError::DuplicateLineage)
            }
            Err(error) => Err(error.into()),
        }
    }
    pub fn persist_oos(
        &self,
        artifact: &ExecutedOosScheme,
        created_sequence: i64,
    ) -> Result<(), RetestError> {
        self.persist_study(
            StudyArtifactKind::Oos,
            artifact.artifact_id(),
            artifact.source_dataset_id(),
            artifact.to_json_vec()?,
            created_sequence,
        )
    }
    pub fn persist_walk_forward(
        &self,
        artifact: &ExecutedWalkForward,
        created_sequence: i64,
    ) -> Result<(), RetestError> {
        self.persist_study(
            StudyArtifactKind::WalkForward,
            artifact.artifact_id(),
            artifact.source_dataset_id(),
            artifact.to_json_vec()?,
            created_sequence,
        )
    }
    pub fn persist_calendar_walk_forward(
        &self,
        artifact: &ExecutedCalendarWalkForward,
        created_sequence: i64,
    ) -> Result<(), RetestError> {
        self.persist_study(
            StudyArtifactKind::CalendarWalkForward,
            artifact.artifact_id(),
            artifact.source_dataset_id(),
            artifact.to_json_vec()?,
            created_sequence,
        )
    }
    pub fn persist_walk_forward_matrix(
        &self,
        artifact: &ExecutedWalkForwardMatrix,
        created_sequence: i64,
    ) -> Result<(), RetestError> {
        self.persist_study(
            StudyArtifactKind::WalkForwardMatrix,
            artifact.artifact_id(),
            artifact.source_dataset_id(),
            artifact.to_json_vec()?,
            created_sequence,
        )
    }
    pub fn persist_trade_monte_carlo(
        &self,
        artifact: &TradeMonteCarloArtifact,
        created_sequence: i64,
    ) -> Result<(), RetestError> {
        self.persist_study(
            StudyArtifactKind::TradeMonteCarlo,
            artifact.artifact_id(),
            artifact.dataset_id(),
            artifact.to_json_vec()?,
            created_sequence,
        )
    }
    pub fn persist_bayesian_study(
        &self,
        artifact: &BayesianStudyArtifact,
        created_sequence: i64,
    ) -> Result<(), RetestError> {
        self.persist_study(
            StudyArtifactKind::BayesianOptimization,
            artifact.artifact_id(),
            artifact.source_dataset_id(),
            artifact.to_json_vec()?,
            created_sequence,
        )
    }
    pub fn persist_perturbation_study(
        &self,
        artifact: &PerturbationStudyArtifact,
        created_sequence: i64,
    ) -> Result<(), RetestError> {
        self.persist_study(
            StudyArtifactKind::Perturbation,
            artifact.artifact_id(),
            artifact.source_dataset_id(),
            artifact.to_json_vec()?,
            created_sequence,
        )
    }
    pub fn persist_parameter_field_study(
        &self,
        artifact: &ParameterFieldStudyArtifact,
        created_sequence: i64,
    ) -> Result<(), RetestError> {
        self.persist_study(
            StudyArtifactKind::ParameterField,
            artifact.artifact_id(),
            artifact.source_dataset_id(),
            artifact.to_json_vec()?,
            created_sequence,
        )
    }
    pub fn persist_cross_check_study(
        &self,
        artifact: &CrossCheckStudyArtifact,
        created_sequence: i64,
    ) -> Result<(), RetestError> {
        self.persist_study(
            StudyArtifactKind::CrossCheck,
            artifact.artifact_id(),
            artifact.source_dataset_id(),
            artifact.to_json_vec()?,
            created_sequence,
        )
    }
    pub fn persist_significance_study(
        &self,
        artifact: &SignificanceStudyArtifact,
        created_sequence: i64,
    ) -> Result<(), RetestError> {
        self.persist_study(
            StudyArtifactKind::Significance,
            artifact.artifact_id(),
            artifact.source_dataset_id(),
            artifact.to_json_vec()?,
            created_sequence,
        )
    }
    pub fn persist_problem_recognition(
        &self,
        artifact: &ProblemRecognitionArtifact,
        created_sequence: i64,
    ) -> Result<(), RetestError> {
        self.persist_study(
            StudyArtifactKind::ProblemRecognition,
            artifact.artifact_id(),
            artifact.source_dataset_id(),
            artifact.to_json_vec()?,
            created_sequence,
        )
    }
    fn persist_study(
        &self,
        kind: StudyArtifactKind,
        artifact_id: &str,
        source_dataset_id: &str,
        bytes: Vec<u8>,
        created_sequence: i64,
    ) -> Result<(), RetestError> {
        if !is_id(artifact_id) || !is_id(source_dataset_id) || bytes.len() > MAX_ARTIFACT_BYTES {
            return Err(RetestError::Invalid(
                "invalid study persistence input".into(),
            ));
        }
        let result = self.connection.borrow().execute(
            "INSERT INTO study_artifact(artifact_id,source_dataset_id,kind,artifact_json,created_sequence) VALUES (?1,?2,?3,?4,?5)",
            params![artifact_id, source_dataset_id, kind.key(), bytes, created_sequence],
        );
        match result {
            Ok(_) => Ok(()),
            Err(error) if error.to_string().contains("UNIQUE constraint failed") => {
                Err(RetestError::DuplicateLineage)
            }
            Err(error) => Err(error.into()),
        }
    }
    pub fn query_studies(
        &self,
        query: &StudyArtifactQuery,
    ) -> Result<StudyArtifactPage, RetestError> {
        validate_study_query(query)?;
        let connection = self.connection.borrow();
        let after = query.after_sequence.unwrap_or(i64::MIN);
        let limit = to_i64(query.limit + 1)?;
        let raw = if let Some(kind) = query.kind {
            let mut statement = connection.prepare_cached(
                "SELECT artifact_id,source_dataset_id,kind,artifact_json,created_sequence
                 FROM study_artifact INDEXED BY idx_study_dataset_kind_sequence
                 WHERE source_dataset_id=?1 AND kind=?2 AND created_sequence>?3
                 ORDER BY created_sequence,artifact_id LIMIT ?4",
            )?;
            statement
                .query_map(
                    params![query.source_dataset_id, kind.key(), after, limit],
                    decode_study_row,
                )?
                .collect::<Result<Vec<_>, _>>()?
        } else {
            let mut statement = connection.prepare_cached(
                "SELECT artifact_id,source_dataset_id,kind,artifact_json,created_sequence
                 FROM study_artifact INDEXED BY idx_study_dataset_sequence
                 WHERE source_dataset_id=?1 AND created_sequence>?2
                 ORDER BY created_sequence,artifact_id LIMIT ?3",
            )?;
            statement
                .query_map(
                    params![query.source_dataset_id, after, limit],
                    decode_study_row,
                )?
                .collect::<Result<Vec<_>, _>>()?
        };
        let mut records = Vec::with_capacity(raw.len());
        for (artifact_id, source_dataset_id, kind_key, bytes, created_sequence) in raw {
            let kind = StudyArtifactKind::decode(kind_key)?;
            let artifact = StudyArtifact::decode(kind, &bytes)?;
            if artifact.artifact_id() != artifact_id
                || artifact.source_dataset_id() != source_dataset_id
            {
                return Err(RetestError::Invalid(
                    "stored study metadata disagrees with artifact".into(),
                ));
            }
            records.push(StudyArtifactRecord {
                artifact_id,
                source_dataset_id,
                kind,
                created_sequence,
                artifact,
            });
        }
        let has_more = records.len() > query.limit;
        records.truncate(query.limit);
        Ok(StudyArtifactPage { records, has_more })
    }
    pub fn explain_study_query(
        &self,
        query: &StudyArtifactQuery,
    ) -> Result<Vec<String>, RetestError> {
        validate_study_query(query)?;
        let connection = self.connection.borrow();
        let after = query.after_sequence.unwrap_or(i64::MIN);
        let limit = to_i64(query.limit + 1)?;
        if let Some(kind) = query.kind {
            let mut statement = connection.prepare(
                "EXPLAIN QUERY PLAN SELECT artifact_id FROM study_artifact INDEXED BY idx_study_dataset_kind_sequence WHERE source_dataset_id=?1 AND kind=?2 AND created_sequence>?3 ORDER BY created_sequence,artifact_id LIMIT ?4",
            )?;
            Ok(statement
                .query_map(
                    params![query.source_dataset_id, kind.key(), after, limit],
                    |row| row.get(3),
                )?
                .collect::<Result<Vec<_>, _>>()?)
        } else {
            let mut statement = connection.prepare(
                "EXPLAIN QUERY PLAN SELECT artifact_id FROM study_artifact INDEXED BY idx_study_dataset_sequence WHERE source_dataset_id=?1 AND created_sequence>?2 ORDER BY created_sequence,artifact_id LIMIT ?3",
            )?;
            Ok(statement
                .query_map(params![query.source_dataset_id, after, limit], |row| {
                    row.get(3)
                })?
                .collect::<Result<Vec<_>, _>>()?)
        }
    }
    pub fn query(&self, query: &RetestEvidenceQuery) -> Result<RetestEvidencePage, RetestError> {
        validate_query(query)?;
        let connection = self.connection.borrow();
        let mut statement = connection.prepare_cached(
            "SELECT run_id,parent_run_id,candidate_id,evaluations_n,robustness_verdict,metric_id,metric_value,range_start,range_end,created_sequence
             FROM retest_evidence INDEXED BY idx_retest_candidate_sequence
             WHERE candidate_id=?1 AND created_sequence>?2
             ORDER BY created_sequence,run_id LIMIT ?3",
        )?;
        let mut records = statement
            .query_map(
                params![
                    query.candidate_id,
                    query.after_sequence.unwrap_or(i64::MIN),
                    to_i64(query.limit + 1)?
                ],
                |row| {
                    Ok(RetestEvidenceRecord {
                        run_id: row.get(0)?,
                        parent_run_id: row.get(1)?,
                        candidate_id: row.get(2)?,
                        evaluations_n: from_i64(row.get(3)?)?,
                        robustness_verdict: decode_verdict(row.get(4)?)?,
                        metric_id: row.get(5)?,
                        metric_value: row.get(6)?,
                        range: from_i64(row.get(7)?)?..from_i64(row.get(8)?)?,
                        created_sequence: row.get(9)?,
                    })
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;
        let has_more = records.len() > query.limit;
        records.truncate(query.limit);
        Ok(RetestEvidencePage { records, has_more })
    }
    pub fn explain_query(&self, query: &RetestEvidenceQuery) -> Result<Vec<String>, RetestError> {
        validate_query(query)?;
        let connection = self.connection.borrow();
        let mut statement = connection.prepare(
            "EXPLAIN QUERY PLAN SELECT run_id FROM retest_evidence INDEXED BY idx_retest_candidate_sequence WHERE candidate_id=?1 AND created_sequence>?2 ORDER BY created_sequence,run_id LIMIT ?3",
        )?;
        Ok(statement
            .query_map(
                params![
                    query.candidate_id,
                    query.after_sequence.unwrap_or(i64::MIN),
                    to_i64(query.limit + 1)?
                ],
                |row| row.get(3),
            )?
            .collect::<Result<Vec<_>, _>>()?)
    }
    pub fn record_holdout_consumption(
        &self,
        burned: &BurnedHoldout,
        consumed_sequence: i64,
    ) -> Result<(), RetestError> {
        let range = burned.range();
        let result = self.connection.borrow().execute(
            "INSERT INTO holdout_consumption(dataset_id,range_start,range_end,reason,consumed_sequence) VALUES (?1,?2,?3,?4,?5)",
            params![
                burned.dataset_id(),
                to_i64(range.start)?,
                to_i64(range.end)?,
                burned.reason(),
                consumed_sequence
            ],
        );
        match result {
            Ok(_) => Ok(()),
            Err(error) if error.to_string().contains("UNIQUE constraint failed") => {
                Err(RetestError::Immutable)
            }
            Err(error) => Err(error.into()),
        }
    }
    pub fn query_holdout(
        &self,
        dataset_id: &str,
        limit: usize,
    ) -> Result<Vec<HoldoutConsumptionRecord>, RetestError> {
        if dataset_id.trim().is_empty() || limit == 0 || limit > MAX_RETEST_QUERY_LIMIT {
            return Err(RetestError::Invalid("invalid holdout query".into()));
        }
        let connection = self.connection.borrow();
        let mut statement = connection.prepare_cached(
            "SELECT dataset_id,range_start,range_end,reason,consumed_sequence FROM holdout_consumption INDEXED BY idx_holdout_sequence WHERE dataset_id=?1 ORDER BY consumed_sequence LIMIT ?2",
        )?;
        Ok(statement
            .query_map(params![dataset_id, to_i64(limit)?], |row| {
                Ok(HoldoutConsumptionRecord {
                    dataset_id: row.get(0)?,
                    range: from_i64(row.get(1)?)?..from_i64(row.get(2)?)?,
                    reason: row.get(3)?,
                    consumed_sequence: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?)
    }
    #[cfg(test)]
    fn test_only_update_study(&self, artifact_id: &str) -> Result<(), RetestError> {
        self.connection.borrow().execute(
            "UPDATE study_artifact SET created_sequence=created_sequence+1 WHERE artifact_id=?1",
            [artifact_id],
        )?;
        Ok(())
    }
    #[cfg(test)]
    fn test_only_delete_study(&self, artifact_id: &str) -> Result<(), RetestError> {
        self.connection.borrow().execute(
            "DELETE FROM study_artifact WHERE artifact_id=?1",
            [artifact_id],
        )?;
        Ok(())
    }
    #[cfg(test)]
    fn test_only_tamper_study_json(&self, artifact_id: &str) -> Result<(), RetestError> {
        self.connection
            .borrow()
            .execute_batch("DROP TRIGGER immutable_study_update;")?;
        self.connection.borrow().execute(
            "UPDATE study_artifact SET artifact_json=X'7B7D' WHERE artifact_id=?1",
            [artifact_id],
        )?;
        Ok(())
    }
    #[cfg(test)]
    fn test_only_update(&self, run_id: &str) -> Result<(), RetestError> {
        self.connection.borrow().execute(
            "UPDATE retest_evidence SET created_sequence=created_sequence+1 WHERE run_id=?1",
            [run_id],
        )?;
        Ok(())
    }
    #[cfg(test)]
    fn test_only_update_holdout(&self, dataset_id: &str) -> Result<(), RetestError> {
        self.connection.borrow().execute(
            "UPDATE holdout_consumption SET consumed_sequence=consumed_sequence+1 WHERE dataset_id=?1",
            [dataset_id],
        )?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct RetestWorkerJob {
    pub request_id: u64,
    pub execution: RetestExecutionRequest,
    pub pipeline: RobustnessPipeline,
    pub evaluations_n: usize,
    pub parent_run_id: Option<String>,
    pub created_sequence: i64,
}

#[derive(Debug)]
pub enum RetestSubmitError {
    Backpressure(RetestWorkerJob),
    Stopped(RetestWorkerJob),
}

#[derive(Debug)]
pub enum RetestWorkerEvent {
    Started {
        request_id: u64,
        thread_id: std::thread::ThreadId,
    },
    Completed {
        request_id: u64,
        result: Box<CompletedRetest>,
    },
    Cancelled {
        request_id: u64,
    },
    Failed {
        request_id: u64,
        message: String,
    },
}

/// Bounded off-thread executor for the blocking simulation and SQLite boundary.
/// Submission and polling never wait; callers install only a `Completed` event whose request id
/// still matches their active request.
pub struct RetestWorker {
    jobs: SyncSender<RetestWorkerJob>,
    events: Receiver<RetestWorkerEvent>,
    cancelled: Arc<Mutex<BTreeSet<u64>>>,
}

impl RetestWorker {
    pub fn spawn(
        path: &Path,
        job_capacity: usize,
        event_capacity: usize,
    ) -> Result<Self, RetestError> {
        Self::spawn_with_store(
            RetestEvidenceStore::open(path)?,
            job_capacity,
            event_capacity,
        )
    }

    pub fn spawn_in_memory(
        job_capacity: usize,
        event_capacity: usize,
    ) -> Result<Self, RetestError> {
        Self::spawn_with_store(
            RetestEvidenceStore::open_in_memory()?,
            job_capacity,
            event_capacity,
        )
    }

    fn spawn_with_store(
        store: RetestEvidenceStore,
        job_capacity: usize,
        event_capacity: usize,
    ) -> Result<Self, RetestError> {
        if job_capacity == 0 || event_capacity == 0 {
            return Err(RetestError::Invalid(
                "retest worker queue capacities must be positive".into(),
            ));
        }
        let (job_tx, job_rx) = std::sync::mpsc::sync_channel(job_capacity);
        let (event_tx, event_rx) = std::sync::mpsc::sync_channel(event_capacity);
        let cancelled = Arc::new(Mutex::new(BTreeSet::new()));
        let worker_cancelled = cancelled.clone();
        std::thread::Builder::new()
            .name("strategy-retest".into())
            .spawn(move || run_retest_worker(job_rx, event_tx, worker_cancelled, store))
            .map_err(invalid)?;
        Ok(Self {
            jobs: job_tx,
            events: event_rx,
            cancelled,
        })
    }

    pub fn try_submit(&self, job: RetestWorkerJob) -> Result<(), RetestSubmitError> {
        self.jobs.try_send(job).map_err(|error| match error {
            TrySendError::Full(job) => RetestSubmitError::Backpressure(job),
            TrySendError::Disconnected(job) => RetestSubmitError::Stopped(job),
        })
    }

    pub fn cancel(&self, request_id: u64) {
        let mut cancelled = self.cancelled.lock().unwrap_or_else(|p| p.into_inner());
        if cancelled.len() >= MAX_TRACKED_RETEST_CANCELLATIONS {
            if let Some(oldest) = cancelled.first().copied() {
                cancelled.remove(&oldest);
            }
        }
        cancelled.insert(request_id);
    }

    pub fn poll(&self) -> Vec<RetestWorkerEvent> {
        let mut events = Vec::with_capacity(MAX_RETEST_EVENTS_PER_POLL);
        while events.len() < MAX_RETEST_EVENTS_PER_POLL {
            match self.events.try_recv() {
                Ok(event) => events.push(event),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
        events
    }
}

fn run_retest_worker(
    jobs: Receiver<RetestWorkerJob>,
    events: SyncSender<RetestWorkerEvent>,
    cancelled: Arc<Mutex<BTreeSet<u64>>>,
    store: RetestEvidenceStore,
) {
    while let Ok(job) = jobs.recv() {
        let request_id = job.request_id;
        if take_cancellation(&cancelled, request_id) {
            if events
                .send(RetestWorkerEvent::Cancelled { request_id })
                .is_err()
            {
                break;
            }
            continue;
        }
        if events
            .send(RetestWorkerEvent::Started {
                request_id,
                thread_id: std::thread::current().id(),
            })
            .is_err()
        {
            break;
        }
        if take_cancellation(&cancelled, request_id) {
            if events
                .send(RetestWorkerEvent::Cancelled { request_id })
                .is_err()
            {
                break;
            }
            continue;
        }
        let outcome = execute_retest(job.execution, &job.pipeline, job.evaluations_n);
        let event = match outcome {
            Ok(_) if take_cancellation(&cancelled, request_id) => {
                RetestWorkerEvent::Cancelled { request_id }
            }
            Ok(result) => {
                match store.persist(&result, job.parent_run_id.as_deref(), job.created_sequence) {
                    Ok(()) => RetestWorkerEvent::Completed {
                        request_id,
                        result: Box::new(result),
                    },
                    Err(error) => RetestWorkerEvent::Failed {
                        request_id,
                        message: error.to_string(),
                    },
                }
            }
            Err(_) if take_cancellation(&cancelled, request_id) => {
                RetestWorkerEvent::Cancelled { request_id }
            }
            Err(error) => RetestWorkerEvent::Failed {
                request_id,
                message: error.to_string(),
            },
        };
        if events.send(event).is_err() {
            break;
        }
    }
}

fn take_cancellation(cancelled: &Mutex<BTreeSet<u64>>, request_id: u64) -> bool {
    cancelled
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .remove(&request_id)
}

fn validate_study_query(query: &StudyArtifactQuery) -> Result<(), RetestError> {
    if !is_id(&query.source_dataset_id) || query.limit == 0 || query.limit > MAX_RETEST_QUERY_LIMIT
    {
        return Err(RetestError::Invalid("invalid study query".into()));
    }
    Ok(())
}

fn decode_study_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<(String, String, i64, Vec<u8>, i64)> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
    ))
}

fn validate_query(query: &RetestEvidenceQuery) -> Result<(), RetestError> {
    if query.candidate_id.trim().is_empty()
        || query.limit == 0
        || query.limit > MAX_RETEST_QUERY_LIMIT
    {
        return Err(RetestError::Invalid("invalid evidence query".into()));
    }
    Ok(())
}
fn invalid(error: impl std::fmt::Display) -> RetestError {
    RetestError::Invalid(error.to_string())
}
fn role_key(role: ObservationRole) -> u8 {
    match role {
        ObservationRole::SearchEvaluation => 0,
        ObservationRole::InSample => 1,
        ObservationRole::OutOfSample => 2,
        ObservationRole::CrossCheck => 3,
    }
}
fn verdict_key(verdict: StageVerdict) -> i64 {
    match verdict {
        StageVerdict::Pass => 1,
        StageVerdict::Fail => 0,
    }
}
fn decode_verdict(value: i64) -> rusqlite::Result<StageVerdict> {
    match value {
        1 => Ok(StageVerdict::Pass),
        0 => Ok(StageVerdict::Fail),
        _ => Err(rusqlite::Error::IntegralValueOutOfRange(4, value)),
    }
}
fn validate_page_query(id: &str, limit: usize) -> Result<(), RetestError> {
    if !is_id(id) || limit == 0 || limit > MAX_RETEST_QUERY_LIMIT {
        return Err(RetestError::Invalid("invalid final-holdout query".into()));
    }
    Ok(())
}
fn validate_final_holdout_record(
    record: &FinalHoldoutRecord,
    report_json: Option<&[u8]>,
) -> Result<(), RetestError> {
    if !is_id(&record.request_id)
        || !is_id(&record.search_dataset_id)
        || !is_id(&record.search_manifest_id)
        || !is_id(&record.holdout_dataset_id)
        || !is_id(&record.holdout_manifest_id)
        || !is_id(&record.strategy_id)
        || !is_id(&record.config_id)
        || record.metrics_version != METRICS_SCHEMA_VERSION
        || record.range.start >= record.range.end
        || record.evaluations_n == 0
        || record.request_id
            != final_holdout_request_id(
                &record.strategy_id,
                &record.config_id,
                &record.search_dataset_id,
                &record.search_manifest_id,
                &record.holdout_dataset_id,
                &record.holdout_manifest_id,
                &record.reason,
                &record.metric_id,
                &record.metrics_version,
                &record.range,
                record.seed,
                record.evaluations_n,
            )
    {
        return Err(RetestError::Invalid(
            "invalid persisted final-holdout identity".into(),
        ));
    }
    match record.outcome {
        FinalHoldoutOutcome::Reserved => {
            if record.run_id.is_some()
                || record.report_id.is_some()
                || record.metric_value.is_some()
                || record.failure.is_some()
                || report_json.is_some()
            {
                return Err(RetestError::Invalid(
                    "invalid reserved holdout evidence".into(),
                ));
            }
        }
        FinalHoldoutOutcome::Failed => {
            if record.failure.as_deref().is_none_or(str::is_empty)
                || report_json.is_some()
                || record.run_id.is_some()
                || record.report_id.is_some()
            {
                return Err(RetestError::Invalid(
                    "invalid failed holdout evidence".into(),
                ));
            }
        }
        FinalHoldoutOutcome::Succeeded => {
            let bytes = report_json.ok_or_else(|| {
                RetestError::Invalid("successful holdout report evidence is missing".into())
            })?;
            let report = StrategyReportArtifact::from_json_slice(bytes).map_err(invalid)?;
            if record.run_id.as_deref() != Some(report.run_id())
                || record.report_id.as_deref() != Some(report.report_id())
                || report.run_manifest().is_none_or(|manifest| {
                    manifest.binding().strategy_id != record.strategy_id
                        || manifest.binding().config_id != record.config_id
                        || manifest.binding().seed != record.seed
                        || manifest
                            .binding()
                            .datasets
                            .first()
                            .is_none_or(|dataset| dataset.dataset_id != record.holdout_dataset_id)
                })
            {
                return Err(RetestError::Invalid(
                    "persisted final-holdout report identity mismatch".into(),
                ));
            }
            let metric = match report.analysis().metric(&record.metric_id) {
                Some(MetricValue::Defined { value }) if value.is_finite() => Some(*value),
                _ => None,
            };
            if metric != record.metric_value {
                return Err(RetestError::Invalid(
                    "persisted final-holdout metric mismatch".into(),
                ));
            }
        }
    }
    Ok(())
}
fn u64_to_sqlite(value: u64) -> i64 {
    value as i64
}
fn sqlite_to_u64(value: i64) -> u64 {
    value as u64
}
fn to_i64(value: usize) -> Result<i64, RetestError> {
    i64::try_from(value).map_err(|_| RetestError::Invalid("integer exceeds SQLite range".into()))
}
fn from_i64(value: i64) -> rusqlite::Result<usize> {
    usize::try_from(value).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(0, value))
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

#[cfg(test)]
mod tests;
