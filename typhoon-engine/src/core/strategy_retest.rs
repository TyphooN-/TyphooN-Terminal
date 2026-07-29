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
    BurnedHoldout, ObservationRole, OptimizationError, ReportObservation, RetestRequest,
    RetestResult, RobustnessArtifact, RobustnessPipeline, SearchDataLease, StageVerdict,
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
