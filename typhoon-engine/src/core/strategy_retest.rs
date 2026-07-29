//! Executable, content-bound ADR-135 M4 retest and evidence persistence boundary.
//!
//! This is deliberately the only M4 path that turns a retest request into a metric. It verifies
//! the exact leased dataset bytes, assembles a [`VerifiedRun`](crate::core::strategy_run::VerifiedRun),
//! executes the canonical simulator, seals and re-verifies the report, and only then projects one
//! typed metric into robustness. Persisted evidence is append-only and queried through bounded,
//! indexed windows.

use crate::broker::alpaca::Bar;
use crate::core::strategy_dataset::DatasetManifest;
use crate::core::strategy_ir::{
    DatasetBinding, RunBinding, StrategyExecutionConfig, StrategyIr, StrategyRunManifest,
};
use crate::core::strategy_metrics::METRICS_SCHEMA_VERSION;
use crate::core::strategy_optimization::{
    BurnedHoldout, FoldPlan, ObjectiveDirection, ObjectiveSpec, ObservationRole, OosPlan,
    OosScheme, OptimizationError, ReportObservation, RetestRequest, RetestResult,
    RobustnessArtifact, RobustnessPipeline, SampleRole, SearchBatch, SearchDataLease, StageAccess,
    StageVerdict, WalkForwardConfig, select_best,
};
use crate::core::strategy_report::StrategyReportArtifact;
use crate::core::strategy_run::{RunDatasetInput, assemble_verified_run};
use crate::core::strategy_simulator::run_verified_simulation;
use rusqlite::{Connection, params};
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
        let mut hasher = Sha256::new();
        hasher.update(REQUEST_DOMAIN);
        frame(&mut hasher, self.retest.request_id().as_bytes());
        frame(&mut hasher, self.dataset.manifest_id.as_bytes());
        hasher.update([role_key(self.role)]);
        frame(&mut hasher, self.metric_id.as_bytes());
        hex(hasher.finalize())
    }
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
fn execute_bound_observation(
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

pub const MAX_WALK_FORWARD_MATRIX_CELLS: usize = 32;
const MAX_STUDY_WINDOWS: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactBarMember {
    pub source_index: usize,
    pub timestamp: String,
    pub role: SampleRole,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactBarMembership {
    pub role: SampleRole,
    pub ranges: Vec<Range<usize>>,
    pub indices: Vec<usize>,
    pub timestamps: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OosSeamProof {
    pub purged: Range<usize>,
    pub oos: Range<usize>,
    pub embargoed: Range<usize>,
}

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, PartialEq)]
pub struct ExecutedOosScheme {
    source_dataset_id: String,
    membership: Vec<ExactBarMember>,
    seams: Vec<OosSeamProof>,
    executed_partitions: Vec<ExecutedPartition>,
}
impl ExecutedOosScheme {
    pub fn source_dataset_id(&self) -> &str {
        &self.source_dataset_id
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
    Ok(ExecutedOosScheme {
        source_dataset_id: source_manifest.dataset_id.clone(),
        membership,
        seams,
        executed_partitions,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct WalkForwardOptimizationSpec {
    pub config: WalkForwardConfig,
    pub minimum_windows: usize,
    pub metric_id: String,
    pub direction: ObjectiveDirection,
    pub root_seed: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DegradationObservation {
    pub in_sample_score: f64,
    pub out_of_sample_score: f64,
    pub delta: f64,
    pub ratio_bps: Option<i32>,
}

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, PartialEq)]
pub struct ConcatenatedOosResult {
    pub ranges: Vec<Range<usize>>,
    pub run_ids: Vec<String>,
    pub report_ids: Vec<String>,
    pub scores: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExecutedWalkForward {
    source_dataset_id: String,
    metric_id: String,
    windows: Vec<WalkForwardWindowResult>,
    concatenated_oos: ConcatenatedOosResult,
    degradation_distribution: Vec<DegradationObservation>,
}
impl ExecutedWalkForward {
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
    Ok(ExecutedWalkForward {
        source_dataset_id: source_manifest.dataset_id.clone(),
        metric_id: spec.metric_id,
        windows,
        concatenated_oos,
        degradation_distribution,
    })
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

#[derive(Debug, Clone, PartialEq)]
pub struct ExecutedWalkForwardMatrixCell {
    pub train_bars: usize,
    pub test_bars: usize,
    pub result: ExecutedWalkForward,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExecutedWalkForwardMatrix {
    cells: Vec<ExecutedWalkForwardMatrixCell>,
}
impl ExecutedWalkForwardMatrix {
    pub fn cells(&self) -> &[ExecutedWalkForwardMatrixCell] {
        &self.cells
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
    Ok(ExecutedWalkForwardMatrix { cells })
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
             CREATE TABLE IF NOT EXISTS holdout_consumption(
               dataset_id TEXT PRIMARY KEY,
               range_start INTEGER NOT NULL,
               range_end INTEGER NOT NULL,
               reason TEXT NOT NULL,
               consumed_sequence INTEGER NOT NULL
             ) WITHOUT ROWID;
             CREATE INDEX IF NOT EXISTS idx_holdout_sequence
               ON holdout_consumption(dataset_id, consumed_sequence);
             CREATE TRIGGER IF NOT EXISTS immutable_retest_update BEFORE UPDATE ON retest_evidence BEGIN SELECT RAISE(ABORT, 'retest evidence is immutable'); END;
             CREATE TRIGGER IF NOT EXISTS immutable_retest_delete BEFORE DELETE ON retest_evidence BEGIN SELECT RAISE(ABORT, 'retest evidence is immutable'); END;
             CREATE TRIGGER IF NOT EXISTS immutable_holdout_update BEFORE UPDATE ON holdout_consumption BEGIN SELECT RAISE(ABORT, 'holdout audit is immutable'); END;
             CREATE TRIGGER IF NOT EXISTS immutable_holdout_delete BEFORE DELETE ON holdout_consumption BEGIN SELECT RAISE(ABORT, 'holdout audit is immutable'); END;",
        )?;
        Ok(Self {
            connection: RefCell::new(connection),
        })
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
    pub fn spawn_in_memory(
        job_capacity: usize,
        event_capacity: usize,
    ) -> Result<Self, RetestError> {
        if job_capacity == 0 || event_capacity == 0 {
            return Err(RetestError::Invalid(
                "retest worker queue capacities must be positive".into(),
            ));
        }
        let store = RetestEvidenceStore::open_in_memory()?;
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
