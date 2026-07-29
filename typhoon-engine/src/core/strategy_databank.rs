//! Indexed, immutable SQLite experiment databank and its bounded worker.
//!
//! A `DatabankStore` is blocking by design and must be owned by the dedicated
//! [`DatabankWorker`] in GUI code. Queries always carry a bounded page and use a
//! fixed allow-list of indexed filters/sorts.

use crate::core::strategy_ir::StrategyIr;
use crate::core::strategy_metrics::{MetricResult, MetricValue};
use rusqlite::{Connection, OptionalExtension, params, params_from_iter, types::Value};
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError, TrySendError};
use std::sync::{Arc, Mutex};

pub const MAX_DATABANK_PAGE_SIZE: usize = 200;
pub const MAX_COMPARE_RUNS: usize = 4;
pub const DATABANK_JOB_QUEUE_CAPACITY: usize = 4;
pub const DATABANK_EVENT_QUEUE_CAPACITY: usize = 32;
pub const MAX_DATABANK_EVENTS_PER_POLL: usize = 8;
const MAX_TRACKED_CANCELLATIONS: usize = 64;

#[derive(Debug)]
pub enum DatabankError {
    Sqlite(String),
    InvalidArtifact(String),
    InvalidMetrics(String),
    NotFound {
        kind: &'static str,
        id: String,
    },
    ImmutableRun {
        run_id: String,
    },
    MetricsMismatch {
        run_id: String,
    },
    LimitExceeded {
        field: &'static str,
        limit: usize,
        found: usize,
    },
}

impl std::fmt::Display for DatabankError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "databank error: {self:?}")
    }
}

impl std::error::Error for DatabankError {}

impl From<rusqlite::Error> for DatabankError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Sqlite(value.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PutStrategyOutcome {
    Inserted,
    AlreadyPresent,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DatabankRunInput {
    pub run_id: String,
    pub strategy_id: String,
    pub dataset_id: String,
    pub config_id: String,
    pub metrics_version: String,
    pub seed: u64,
    pub created_sequence: i64,
    pub metrics: Vec<MetricResult>,
    pub tags: Vec<String>,
    pub parent_run_id: Option<String>,
    pub retest_of_run_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StoredRun {
    pub run_id: String,
    pub strategy_id: String,
    pub dataset_id: String,
    pub config_id: String,
    pub metrics_version: String,
    pub seed: u64,
    pub created_sequence: i64,
    pub metrics: Vec<MetricResult>,
    pub tags: Vec<String>,
    pub parent_run_id: Option<String>,
    pub retest_of_run_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DatabankSort {
    #[default]
    CreatedDesc,
    NetProfitDesc,
    DrawdownAsc,
    SharpeDesc,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DatabankQuery {
    pub strategy_id: Option<String>,
    pub dataset_id: Option<String>,
    pub tag: Option<String>,
    pub min_net_profit: Option<f64>,
    pub max_drawdown_percent: Option<f64>,
    pub sort: DatabankSort,
    pub offset: usize,
    pub limit: usize,
}

impl Default for DatabankQuery {
    fn default() -> Self {
        Self {
            strategy_id: None,
            dataset_id: None,
            tag: None,
            min_net_profit: None,
            max_drawdown_percent: None,
            sort: DatabankSort::CreatedDesc,
            offset: 0,
            limit: 50,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DatabankRow {
    pub run_id: String,
    pub strategy_id: String,
    pub dataset_id: String,
    pub created_sequence: i64,
    pub net_profit: f64,
    pub max_drawdown_percent: f64,
    pub sharpe_ratio: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DatabankPage {
    pub rows: Vec<DatabankRow>,
    pub has_more: bool,
}

pub struct DatabankStore {
    connection: RefCell<Connection>,
}

impl DatabankStore {
    pub fn open(path: &Path) -> Result<Self, DatabankError> {
        let connection = Connection::open(path)?;
        Self::initialize(connection)
    }

    pub fn open_in_memory() -> Result<Self, DatabankError> {
        Self::initialize(Connection::open_in_memory()?)
    }

    fn initialize(connection: Connection) -> Result<Self, DatabankError> {
        connection.execute_batch(
            "PRAGMA foreign_keys=ON;
             PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS strategies(
               strategy_id TEXT PRIMARY KEY,
               canonical_ir BLOB NOT NULL
             ) WITHOUT ROWID;
             CREATE TABLE IF NOT EXISTS runs(
               run_id TEXT PRIMARY KEY,
               strategy_id TEXT NOT NULL REFERENCES strategies(strategy_id),
               dataset_id TEXT NOT NULL,
               config_id TEXT NOT NULL,
               metrics_version TEXT NOT NULL,
               seed INTEGER NOT NULL,
               created_sequence INTEGER NOT NULL,
               metrics_json BLOB NOT NULL,
               net_profit REAL NOT NULL,
               max_drawdown_percent REAL NOT NULL,
               sharpe_ratio REAL NOT NULL,
               parent_run_id TEXT,
               retest_of_run_id TEXT
             ) WITHOUT ROWID;
             CREATE TABLE IF NOT EXISTS run_tags(
               run_id TEXT NOT NULL REFERENCES runs(run_id),
               tag TEXT NOT NULL,
               net_profit REAL NOT NULL,
               PRIMARY KEY(run_id, tag)
             ) WITHOUT ROWID;
             CREATE INDEX IF NOT EXISTS idx_runs_created ON runs(created_sequence DESC, run_id);
             CREATE INDEX IF NOT EXISTS idx_runs_profit ON runs(net_profit DESC, run_id);
             CREATE INDEX IF NOT EXISTS idx_runs_drawdown ON runs(max_drawdown_percent ASC, run_id);
             CREATE INDEX IF NOT EXISTS idx_runs_sharpe ON runs(sharpe_ratio DESC, run_id);
             CREATE INDEX IF NOT EXISTS idx_runs_strategy_profit ON runs(strategy_id, net_profit DESC, run_id);
             CREATE INDEX IF NOT EXISTS idx_runs_dataset_created ON runs(dataset_id, created_sequence DESC, run_id);
             CREATE INDEX IF NOT EXISTS idx_tags_tag_profit ON run_tags(tag, net_profit DESC, run_id);
             CREATE TRIGGER IF NOT EXISTS immutable_runs_update BEFORE UPDATE ON runs BEGIN SELECT RAISE(ABORT, 'runs are immutable'); END;
             CREATE TRIGGER IF NOT EXISTS immutable_runs_delete BEFORE DELETE ON runs BEGIN SELECT RAISE(ABORT, 'runs are immutable'); END;
             CREATE TRIGGER IF NOT EXISTS immutable_tags_update BEFORE UPDATE ON run_tags BEGIN SELECT RAISE(ABORT, 'run tags are immutable'); END;
             CREATE TRIGGER IF NOT EXISTS immutable_tags_delete BEFORE DELETE ON run_tags BEGIN SELECT RAISE(ABORT, 'run tags are immutable'); END;"
        )?;
        Ok(Self {
            connection: RefCell::new(connection),
        })
    }

    pub fn put_strategy(&self, strategy: &StrategyIr) -> Result<PutStrategyOutcome, DatabankError> {
        strategy
            .verify()
            .map_err(|e| DatabankError::InvalidArtifact(e.to_string()))?;
        let bytes = serde_json::to_vec(strategy)
            .map_err(|e| DatabankError::InvalidArtifact(e.to_string()))?;
        let connection = self.connection.borrow();
        let changed = connection.execute(
            "INSERT OR IGNORE INTO strategies(strategy_id, canonical_ir) VALUES (?1, ?2)",
            params![strategy.strategy_id(), bytes],
        )?;
        if changed == 0 {
            let stored: Vec<u8> = connection.query_row(
                "SELECT canonical_ir FROM strategies WHERE strategy_id=?1",
                [strategy.strategy_id()],
                |row| row.get(0),
            )?;
            if stored
                != serde_json::to_vec(strategy)
                    .map_err(|e| DatabankError::InvalidArtifact(e.to_string()))?
            {
                return Err(DatabankError::InvalidArtifact(
                    "strategy id collision with different canonical bytes".into(),
                ));
            }
            Ok(PutStrategyOutcome::AlreadyPresent)
        } else {
            Ok(PutStrategyOutcome::Inserted)
        }
    }

    pub fn strategy_count(&self) -> Result<u64, DatabankError> {
        let count: i64 =
            self.connection
                .borrow()
                .query_row("SELECT count(*) FROM strategies", [], |row| row.get(0))?;
        Ok(count as u64)
    }

    pub fn load_strategy(&self, strategy_id: &str) -> Result<StrategyIr, DatabankError> {
        let bytes: Option<Vec<u8>> = self
            .connection
            .borrow()
            .query_row(
                "SELECT canonical_ir FROM strategies WHERE strategy_id=?1",
                [strategy_id],
                |row| row.get(0),
            )
            .optional()?;
        let bytes = bytes.ok_or_else(|| DatabankError::NotFound {
            kind: "strategy",
            id: strategy_id.into(),
        })?;
        StrategyIr::from_json_slice(&bytes)
            .map_err(|e| DatabankError::InvalidArtifact(e.to_string()))
    }

    pub fn append_run(&self, input: &DatabankRunInput) -> Result<(), DatabankError> {
        let metrics_json = serde_json::to_vec(&input.metrics)
            .map_err(|e| DatabankError::InvalidMetrics(e.to_string()))?;
        let net_profit = defined_metric(&input.metrics, "net_profit")?;
        let drawdown = defined_metric(&input.metrics, "max_drawdown_percent")?;
        let sharpe = defined_metric(&input.metrics, "sharpe_ratio")?;
        let seed = i64::try_from(input.seed).map_err(|_| {
            DatabankError::InvalidMetrics("seed exceeds SQLite signed integer".into())
        })?;
        let mut connection = self.connection.borrow_mut();
        let transaction = connection.transaction()?;
        let result = transaction.execute(
            "INSERT INTO runs(run_id,strategy_id,dataset_id,config_id,metrics_version,seed,created_sequence,metrics_json,net_profit,max_drawdown_percent,sharpe_ratio,parent_run_id,retest_of_run_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
            params![input.run_id,input.strategy_id,input.dataset_id,input.config_id,input.metrics_version,seed,input.created_sequence,metrics_json,net_profit,drawdown,sharpe,input.parent_run_id,input.retest_of_run_id]
        );
        if let Err(error) = result {
            if error.to_string().contains("UNIQUE constraint failed") {
                return Err(DatabankError::ImmutableRun {
                    run_id: input.run_id.clone(),
                });
            }
            return Err(error.into());
        }
        let mut tags = input.tags.clone();
        tags.sort();
        tags.dedup();
        for tag in tags {
            transaction.execute(
                "INSERT INTO run_tags(run_id,tag,net_profit) VALUES (?1,?2,?3)",
                params![input.run_id, tag, net_profit],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn load_run(&self, run_id: &str) -> Result<StoredRun, DatabankError> {
        let connection = self.connection.borrow();
        let mut run: StoredRun = connection.query_row(
            "SELECT run_id,strategy_id,dataset_id,config_id,metrics_version,seed,created_sequence,metrics_json,parent_run_id,retest_of_run_id FROM runs WHERE run_id=?1",
            [run_id], |row| {
                let seed: i64 = row.get(5)?; let metrics: Vec<u8> = row.get(7)?;
                Ok(StoredRun { run_id: row.get(0)?, strategy_id: row.get(1)?, dataset_id: row.get(2)?, config_id: row.get(3)?, metrics_version: row.get(4)?, seed: seed as u64, created_sequence: row.get(6)?, metrics: serde_json::from_slice(&metrics).map_err(|e| rusqlite::Error::FromSqlConversionFailure(metrics.len(), rusqlite::types::Type::Blob, Box::new(e)))?, tags: Vec::new(), parent_run_id: row.get(8)?, retest_of_run_id: row.get(9)? })
            }).optional()?.ok_or_else(|| DatabankError::NotFound { kind: "run", id: run_id.into() })?;
        let mut statement =
            connection.prepare_cached("SELECT tag FROM run_tags WHERE run_id=?1 ORDER BY tag")?;
        run.tags = statement
            .query_map([run_id], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;
        Ok(run)
    }

    pub fn verify_rerun_metrics(
        &self,
        run_id: &str,
        actual: &[MetricResult],
    ) -> Result<(), DatabankError> {
        let expected = self.load_run(run_id)?.metrics;
        if expected == actual {
            Ok(())
        } else {
            Err(DatabankError::MetricsMismatch {
                run_id: run_id.into(),
            })
        }
    }

    pub fn query_runs(&self, query: &DatabankQuery) -> Result<DatabankPage, DatabankError> {
        let (sql, values) = query_sql(query, false)?;
        let connection = self.connection.borrow();
        let mut statement = connection.prepare_cached(&sql)?;
        let mut rows = statement.query(params_from_iter(values))?;
        let mut output = Vec::with_capacity(query.limit + 1);
        while let Some(row) = rows.next()? {
            output.push(DatabankRow {
                run_id: row.get(0)?,
                strategy_id: row.get(1)?,
                dataset_id: row.get(2)?,
                created_sequence: row.get(3)?,
                net_profit: row.get(4)?,
                max_drawdown_percent: row.get(5)?,
                sharpe_ratio: row.get(6)?,
            });
        }
        let has_more = output.len() > query.limit;
        output.truncate(query.limit);
        Ok(DatabankPage {
            rows: output,
            has_more,
        })
    }

    pub fn explain_query(&self, query: &DatabankQuery) -> Result<Vec<String>, DatabankError> {
        let (sql, values) = query_sql(query, true)?;
        let connection = self.connection.borrow();
        let mut statement = connection.prepare(&sql)?;
        Ok(statement
            .query_map(params_from_iter(values), |row| row.get(3))?
            .collect::<Result<Vec<String>, _>>()?)
    }

    pub fn compare_runs(&self, run_ids: &[String]) -> Result<Vec<StoredRun>, DatabankError> {
        if run_ids.len() > MAX_COMPARE_RUNS {
            return Err(DatabankError::LimitExceeded {
                field: "compare runs",
                limit: MAX_COMPARE_RUNS,
                found: run_ids.len(),
            });
        }
        run_ids.iter().map(|id| self.load_run(id)).collect()
    }

    pub fn seed_synthetic_runs(
        &self,
        count: usize,
        strategy_id: &str,
    ) -> Result<(), DatabankError> {
        let mut connection = self.connection.borrow_mut();
        let transaction = connection.transaction()?;
        {
            let mut run_statement = transaction.prepare_cached("INSERT INTO runs(run_id,strategy_id,dataset_id,config_id,metrics_version,seed,created_sequence,metrics_json,net_profit,max_drawdown_percent,sharpe_ratio,parent_run_id,retest_of_run_id) VALUES (?1,?2,?3,?4,'strategy-metrics-v1',?5,?6,?7,?8,?9,?10,NULL,NULL)")?;
            let mut tag_statement = transaction
                .prepare_cached("INSERT INTO run_tags(run_id,tag,net_profit) VALUES (?1,?2,?3)")?;
            for sequence in 0..count {
                let run_id = format!("synthetic-{sequence:06}");
                let profit = sequence as f64;
                let drawdown = (sequence % 100) as f64;
                let sharpe = (sequence % 31) as f64 / 10.0;
                let metrics = vec![
                    MetricResult {
                        id: "net_profit".into(),
                        value: MetricValue::Defined { value: profit },
                    },
                    MetricResult {
                        id: "max_drawdown_percent".into(),
                        value: MetricValue::Defined { value: drawdown },
                    },
                    MetricResult {
                        id: "sharpe_ratio".into(),
                        value: MetricValue::Defined { value: sharpe },
                    },
                ];
                let json = serde_json::to_vec(&metrics)
                    .map_err(|e| DatabankError::InvalidMetrics(e.to_string()))?;
                run_statement.execute(params![
                    run_id,
                    strategy_id,
                    format!("dataset-{}", sequence % 20),
                    format!("config-{}", sequence % 7),
                    sequence as i64,
                    sequence as i64,
                    json,
                    profit,
                    drawdown,
                    sharpe
                ])?;
                tag_statement.execute(params![
                    run_id,
                    format!("bucket-{}", sequence % 10),
                    profit
                ])?;
                tag_statement.execute(params![run_id, "synthetic", profit])?;
            }
        }
        transaction.commit()?;
        Ok(())
    }

    #[cfg(test)]
    fn test_only_update_run(&self, run_id: &str) -> Result<(), DatabankError> {
        self.connection
            .borrow()
            .execute("UPDATE runs SET seed=seed+1 WHERE run_id=?1", [run_id])?;
        Ok(())
    }
    #[cfg(test)]
    fn test_only_delete_run(&self, run_id: &str) -> Result<(), DatabankError> {
        self.connection
            .borrow()
            .execute("DELETE FROM runs WHERE run_id=?1", [run_id])?;
        Ok(())
    }
}

fn defined_metric(metrics: &[MetricResult], id: &'static str) -> Result<f64, DatabankError> {
    metrics
        .iter()
        .find(|m| m.id == id)
        .and_then(|m| match m.value {
            MetricValue::Defined { value } if value.is_finite() => Some(value),
            _ => None,
        })
        .ok_or_else(|| {
            DatabankError::InvalidMetrics(format!(
                "required sortable metric `{id}` must be finite and defined"
            ))
        })
}

fn query_sql(query: &DatabankQuery, explain: bool) -> Result<(String, Vec<Value>), DatabankError> {
    if query.limit == 0 || query.limit > MAX_DATABANK_PAGE_SIZE {
        return Err(DatabankError::LimitExceeded {
            field: "page size",
            limit: MAX_DATABANK_PAGE_SIZE,
            found: query.limit,
        });
    }
    let mut sql = if explain {
        "EXPLAIN QUERY PLAN ".to_string()
    } else {
        String::new()
    };
    sql.push_str("SELECT r.run_id,r.strategy_id,r.dataset_id,r.created_sequence,r.net_profit,r.max_drawdown_percent,r.sharpe_ratio FROM runs r");
    let mut conditions = Vec::new();
    let mut values = Vec::new();
    if query.tag.is_some() {
        sql.push_str(" INDEXED BY idx_runs_profit JOIN run_tags t INDEXED BY idx_tags_tag_profit ON t.run_id=r.run_id");
    }
    if let Some(value) = &query.strategy_id {
        conditions.push("r.strategy_id=?");
        values.push(Value::Text(value.clone()));
    }
    if let Some(value) = &query.dataset_id {
        conditions.push("r.dataset_id=?");
        values.push(Value::Text(value.clone()));
    }
    if let Some(value) = &query.tag {
        conditions.push("t.tag=?");
        values.push(Value::Text(value.clone()));
    }
    if let Some(value) = query.min_net_profit {
        conditions.push("r.net_profit>=?");
        values.push(Value::Real(value));
    }
    if let Some(value) = query.max_drawdown_percent {
        conditions.push("r.max_drawdown_percent<=?");
        values.push(Value::Real(value));
    }
    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }
    sql.push_str(match query.sort {
        DatabankSort::CreatedDesc => " ORDER BY r.created_sequence DESC,r.run_id",
        DatabankSort::NetProfitDesc => " ORDER BY r.net_profit DESC,r.run_id",
        DatabankSort::DrawdownAsc => " ORDER BY r.max_drawdown_percent ASC,r.run_id",
        DatabankSort::SharpeDesc => " ORDER BY r.sharpe_ratio DESC,r.run_id",
    });
    sql.push_str(" LIMIT ? OFFSET ?");
    values.push(Value::Integer((query.limit + 1) as i64));
    values.push(Value::Integer(query.offset as i64));
    Ok((sql, values))
}

#[derive(Debug)]
pub enum DatabankJob {
    PutStrategy {
        request_id: u64,
        strategy: Box<StrategyIr>,
    },
    LoadStrategy {
        request_id: u64,
        strategy_id: String,
    },
    Query {
        request_id: u64,
        query: DatabankQuery,
    },
    Compare {
        request_id: u64,
        run_ids: Vec<String>,
    },
    AppendRun {
        request_id: u64,
        run: DatabankRunInput,
    },
    VerifyRerun {
        request_id: u64,
        run_id: String,
        metrics: Vec<MetricResult>,
    },
}
impl DatabankJob {
    pub fn request_id(&self) -> u64 {
        match self {
            Self::PutStrategy { request_id, .. }
            | Self::LoadStrategy { request_id, .. }
            | Self::Query { request_id, .. }
            | Self::Compare { request_id, .. }
            | Self::AppendRun { request_id, .. }
            | Self::VerifyRerun { request_id, .. } => *request_id,
        }
    }
}

#[derive(Debug)]
pub enum DatabankWorkerEvent {
    Started {
        request_id: u64,
        worker_thread: std::thread::ThreadId,
    },
    StrategyPut {
        request_id: u64,
        outcome: PutStrategyOutcome,
    },
    StrategyLoaded {
        request_id: u64,
        strategy: Box<StrategyIr>,
    },
    Page {
        request_id: u64,
        page: DatabankPage,
    },
    Comparison {
        request_id: u64,
        runs: Vec<StoredRun>,
    },
    RunAppended {
        request_id: u64,
    },
    RerunVerified {
        request_id: u64,
    },
    Failed {
        request_id: u64,
        message: String,
    },
    Cancelled {
        request_id: u64,
    },
}
impl DatabankWorkerEvent {
    pub fn request_id(&self) -> u64 {
        match self {
            Self::Started { request_id, .. }
            | Self::StrategyPut { request_id, .. }
            | Self::StrategyLoaded { request_id, .. }
            | Self::Page { request_id, .. }
            | Self::Comparison { request_id, .. }
            | Self::RunAppended { request_id }
            | Self::RerunVerified { request_id }
            | Self::Failed { request_id, .. }
            | Self::Cancelled { request_id } => *request_id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabankSubmitError {
    QueueFull,
    Stopped,
}

pub struct DatabankWorker {
    jobs: SyncSender<DatabankJob>,
    events: Receiver<DatabankWorkerEvent>,
    cancelled: Arc<Mutex<BTreeSet<u64>>>,
}
impl DatabankWorker {
    pub fn spawn(path: impl AsRef<Path>) -> Result<Self, DatabankError> {
        let path = path.as_ref().to_path_buf();
        Self::spawn_with(move || DatabankStore::open(&path))
    }
    pub fn spawn_in_memory() -> Result<Self, DatabankError> {
        Self::spawn_with(DatabankStore::open_in_memory)
    }
    fn spawn_with(
        open: impl FnOnce() -> Result<DatabankStore, DatabankError> + Send + 'static,
    ) -> Result<Self, DatabankError> {
        let store = open()?;
        let (job_tx, job_rx) = std::sync::mpsc::sync_channel(DATABANK_JOB_QUEUE_CAPACITY);
        let (event_tx, event_rx) = std::sync::mpsc::sync_channel(DATABANK_EVENT_QUEUE_CAPACITY);
        let cancelled = Arc::new(Mutex::new(BTreeSet::new()));
        let worker_cancelled = cancelled.clone();
        std::thread::Builder::new()
            .name("strategy-databank".into())
            .spawn(move || worker_loop(store, job_rx, event_tx, worker_cancelled))
            .map_err(|e| DatabankError::Sqlite(e.to_string()))?;
        Ok(Self {
            jobs: job_tx,
            events: event_rx,
            cancelled,
        })
    }
    pub fn submit(&self, job: DatabankJob) -> Result<(), DatabankSubmitError> {
        match self.jobs.try_send(job) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => Err(DatabankSubmitError::QueueFull),
            Err(TrySendError::Disconnected(_)) => Err(DatabankSubmitError::Stopped),
        }
    }
    pub fn cancel(&self, request_id: u64) {
        let mut cancelled = self.cancelled.lock().unwrap_or_else(|p| p.into_inner());
        if cancelled.len() >= MAX_TRACKED_CANCELLATIONS
            && let Some(first) = cancelled.first().copied()
        {
            cancelled.remove(&first);
        }
        cancelled.insert(request_id);
    }
    pub fn poll(&self) -> Vec<DatabankWorkerEvent> {
        let mut events = Vec::new();
        for _ in 0..MAX_DATABANK_EVENTS_PER_POLL {
            match self.events.try_recv() {
                Ok(event) => events.push(event),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
        events
    }
}

fn worker_loop(
    store: DatabankStore,
    jobs: Receiver<DatabankJob>,
    events: SyncSender<DatabankWorkerEvent>,
    cancelled: Arc<Mutex<BTreeSet<u64>>>,
) {
    while let Ok(job) = jobs.recv() {
        let request_id = job.request_id();
        let is_cancelled = cancelled
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .remove(&request_id);
        if is_cancelled {
            if events
                .send(DatabankWorkerEvent::Cancelled { request_id })
                .is_err()
            {
                break;
            }
            continue;
        }
        if events
            .send(DatabankWorkerEvent::Started {
                request_id,
                worker_thread: std::thread::current().id(),
            })
            .is_err()
        {
            break;
        }
        let terminal =
            match job {
                DatabankJob::PutStrategy { strategy, .. } => store
                    .put_strategy(strategy.as_ref())
                    .map(|outcome| DatabankWorkerEvent::StrategyPut {
                        request_id,
                        outcome,
                    }),
                DatabankJob::LoadStrategy { strategy_id, .. } => store
                    .load_strategy(&strategy_id)
                    .map(|strategy| DatabankWorkerEvent::StrategyLoaded {
                        request_id,
                        strategy: Box::new(strategy),
                    }),
                DatabankJob::Query { query, .. } => store
                    .query_runs(&query)
                    .map(|page| DatabankWorkerEvent::Page { request_id, page }),
                DatabankJob::Compare { run_ids, .. } => store
                    .compare_runs(&run_ids)
                    .map(|runs| DatabankWorkerEvent::Comparison { request_id, runs }),
                DatabankJob::AppendRun { run, .. } => store
                    .append_run(&run)
                    .map(|()| DatabankWorkerEvent::RunAppended { request_id }),
                DatabankJob::VerifyRerun {
                    run_id, metrics, ..
                } => store
                    .verify_rerun_metrics(&run_id, &metrics)
                    .map(|()| DatabankWorkerEvent::RerunVerified { request_id }),
            }
            .unwrap_or_else(|error| DatabankWorkerEvent::Failed {
                request_id,
                message: error.to_string(),
            });
        if events.send(terminal).is_err() {
            break;
        }
    }
}

#[cfg(test)]
mod tests;
