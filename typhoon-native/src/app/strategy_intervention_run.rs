//! Native manifest-bound intervention replay and promotion.
//!
//! Artifact parsing, identity checks, dataset loading, exact replay, and report
//! preparation all happen on a bounded worker. The render thread only snapshots
//! paths, selected dataset identities, and chart identity.

use crate::app::strategy_report_view::{StrategyResultView, StrategyResultViewError};
use crate::app::strategy_sub_bar_run::{RunChartContext, RunRequestIdentity};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError, TrySendError};
use std::sync::{Arc, Mutex};
use typhoon_engine::broker::alpaca::Bar;
use typhoon_engine::core::strategy_dataset::DatasetManifest;
use typhoon_engine::core::strategy_dataset_store::{DatasetRecord, FileDatasetStore};
use typhoon_engine::core::strategy_intervention::{
    InterventionLog, MAX_INTERVENTION_LOG_JSON_BYTES,
};
use typhoon_engine::core::strategy_ir::{StrategyExecutionConfig, StrategyIr, StrategyRunManifest};
use typhoon_engine::core::strategy_report::StrategyReportArtifact;
use typhoon_engine::core::strategy_run::{
    RunDatasetInput, assemble_verified_run_with_intervention,
};
use typhoon_engine::core::strategy_simulator::{
    MAX_BARS_PER_SYMBOL, MAX_SYMBOLS, MAX_TOTAL_BARS, SymbolId,
    run_verified_simulation_with_intervention,
};

pub(crate) const INTERVENTION_RUN_JOB_QUEUE_CAPACITY: usize = 1;
pub(crate) const INTERVENTION_RUN_EVENT_QUEUE_CAPACITY: usize = 2;
pub(crate) const MAX_INTERVENTION_RUN_EVENTS_PER_POLL: usize = 2;
const MAX_SEALED_RUN_ARTIFACT_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Debug, Default)]
pub(crate) struct InterventionRunUiState {
    pub(crate) selected_dataset_ids: BTreeSet<String>,
    pub(crate) strategy_path: String,
    pub(crate) config_path: String,
    pub(crate) manifest_path: String,
    pub(crate) intervention_log_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PromotedRunIdentities {
    pub(crate) run_id: String,
    pub(crate) log_id: String,
    pub(crate) report_id: String,
}

#[derive(Debug, Default)]
pub(crate) struct InterventionRunState {
    pending: Option<RunRequestIdentity>,
    next_request_id: u64,
    next_generation: u64,
    pub(crate) status: String,
    pub(crate) installed: Option<PromotedRunIdentities>,
}

impl InterventionRunState {
    pub(crate) fn is_busy(&self) -> bool {
        self.pending.is_some()
    }

    pub(crate) fn begin_request(&mut self) -> RunRequestIdentity {
        self.next_request_id = self.next_request_id.wrapping_add(1).max(1);
        self.next_generation = self.next_generation.wrapping_add(1).max(1);
        let identity = RunRequestIdentity {
            request_id: self.next_request_id,
            generation: self.next_generation,
        };
        self.pending = Some(identity);
        identity
    }

    pub(crate) fn accept_terminal(
        &mut self,
        identity: RunRequestIdentity,
        result: Result<&PromotedRunIdentities, &str>,
    ) -> bool {
        if self.pending != Some(identity) {
            return false;
        }
        self.pending = None;
        match result {
            Ok(identities) => {
                self.installed = Some(identities.clone());
                self.status = format!(
                    "Exact intervention replay verified · run {} · log {} · report {}",
                    identities.run_id, identities.log_id, identities.report_id
                );
            }
            Err(error) => self.status = format!("Error: {error}"),
        }
        true
    }

    pub(crate) fn cancel(&mut self) -> RunRequestIdentity {
        let identity = self.begin_request();
        self.pending = None;
        self.status = "Cancelled; queued and in-flight intervention results were superseded".into();
        identity
    }
}

#[derive(Debug, Clone)]
pub(crate) struct InterventionRunJob {
    pub(crate) identity: RunRequestIdentity,
    pub(crate) selected_dataset_ids: Vec<String>,
    pub(crate) strategy_path: String,
    pub(crate) config_path: String,
    pub(crate) manifest_path: String,
    pub(crate) intervention_log_path: String,
    pub(crate) chart: RunChartContext,
}

#[derive(Debug)]
pub(crate) struct InterventionRunOutput {
    pub(crate) manifest: StrategyRunManifest,
    pub(crate) log_id: String,
    pub(crate) view: StrategyResultView,
    pub(crate) chart: RunChartContext,
}

#[derive(Debug)]
pub(crate) enum InterventionRunEvent {
    Completed {
        identity: RunRequestIdentity,
        output: Box<InterventionRunOutput>,
    },
    Failed {
        identity: RunRequestIdentity,
        message: String,
    },
    Cancelled {
        identity: RunRequestIdentity,
    },
}

impl InterventionRunEvent {
    pub(crate) fn identity(&self) -> RunRequestIdentity {
        match self {
            Self::Completed { identity, .. }
            | Self::Failed { identity, .. }
            | Self::Cancelled { identity } => *identity,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InterventionRunSubmitError {
    QueueFull,
    WorkerStopped,
}

impl std::fmt::Display for InterventionRunSubmitError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::QueueFull => formatter.write_str("intervention-run worker queue is full"),
            Self::WorkerStopped => formatter.write_str("intervention-run worker is not running"),
        }
    }
}

#[derive(Debug)]
pub(crate) struct InterventionRunWorker {
    jobs: SyncSender<InterventionRunJob>,
    events: Receiver<InterventionRunEvent>,
    current: Arc<Mutex<Option<RunRequestIdentity>>>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl InterventionRunWorker {
    pub(crate) fn spawn_at(root: PathBuf) -> Result<Self, std::io::Error> {
        let (job_tx, jobs) = std::sync::mpsc::sync_channel(INTERVENTION_RUN_JOB_QUEUE_CAPACITY);
        let (event_tx, events) =
            std::sync::mpsc::sync_channel(INTERVENTION_RUN_EVENT_QUEUE_CAPACITY);
        let current = Arc::new(Mutex::new(None));
        let worker_current = Arc::clone(&current);
        let handle = std::thread::Builder::new()
            .name("typhoon-intervention-run-worker".into())
            .spawn(move || run_worker(root, jobs, event_tx, worker_current))?;
        Ok(Self {
            jobs: job_tx,
            events,
            current,
            handle: Some(handle),
        })
    }

    pub(crate) fn submit(&self, job: InterventionRunJob) -> Result<(), InterventionRunSubmitError> {
        let identity = job.identity;
        let mut current = self
            .current
            .lock()
            .expect("intervention-current mutex poisoned");
        let previous = current.replace(identity);
        match self.jobs.try_send(job) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => {
                *current = previous;
                Err(InterventionRunSubmitError::QueueFull)
            }
            Err(TrySendError::Disconnected(_)) => {
                *current = previous;
                Err(InterventionRunSubmitError::WorkerStopped)
            }
        }
    }

    pub(crate) fn supersede_with(&self, identity: RunRequestIdentity) {
        *self
            .current
            .lock()
            .expect("intervention-current mutex poisoned") = Some(identity);
    }

    pub(crate) fn poll(&self) -> Vec<InterventionRunEvent> {
        let mut events = Vec::new();
        while events.len() < MAX_INTERVENTION_RUN_EVENTS_PER_POLL {
            match self.events.try_recv() {
                Ok(event) => events.push(event),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
        events
    }

    pub(crate) fn shutdown(mut self) {
        let handle = self.handle.take();
        drop(self);
        if let Some(handle) = handle {
            let _ = handle.join();
        }
    }
}

fn emit_terminal(
    sender: &SyncSender<InterventionRunEvent>,
    current: &Mutex<Option<RunRequestIdentity>>,
    identity: RunRequestIdentity,
    mut event: InterventionRunEvent,
) -> bool {
    loop {
        let guard = current.lock().expect("intervention-current mutex poisoned");
        if *guard != Some(identity) {
            event = InterventionRunEvent::Cancelled { identity };
        }
        match sender.try_send(event) {
            Ok(()) => return true,
            Err(TrySendError::Full(returned)) => event = returned,
            Err(TrySendError::Disconnected(_)) => return false,
        }
        drop(guard);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

fn run_worker(
    root: PathBuf,
    jobs: Receiver<InterventionRunJob>,
    events: SyncSender<InterventionRunEvent>,
    current: Arc<Mutex<Option<RunRequestIdentity>>>,
) {
    while let Ok(job) = jobs.recv() {
        let identity = job.identity;
        let is_current =
            *current.lock().expect("intervention-current mutex poisoned") == Some(identity);
        let event = if !is_current {
            InterventionRunEvent::Cancelled { identity }
        } else {
            match execute_intervention_run_job(&root, &job) {
                Ok(output) => InterventionRunEvent::Completed {
                    identity,
                    output: Box::new(output),
                },
                Err(message) => InterventionRunEvent::Failed { identity, message },
            }
        };
        if !emit_terminal(&events, &current, identity, event) {
            return;
        }
    }
}

fn read_bounded(path: &str, label: &str, limit: u64) -> Result<Vec<u8>, String> {
    if path.trim().is_empty() {
        return Err(format!("select a sealed {label} JSON artifact"));
    }
    let metadata = std::fs::metadata(path).map_err(|error| format!("{label} `{path}`: {error}"))?;
    if metadata.len() == 0 || metadata.len() > limit {
        return Err(format!("{label} must contain 1..={limit} bytes"));
    }
    std::fs::read(path).map_err(|error| format!("{label} `{path}`: {error}"))
}

struct LoadedInput {
    input_id: String,
    manifest: DatasetManifest,
    bars: Vec<Bar>,
}

fn load_record_bounded(
    record: &DatasetRecord,
    label: &str,
    running_total: &mut usize,
) -> Result<Vec<Bar>, String> {
    let count = usize::try_from(record.bar_count())
        .map_err(|_| format!("{label} bar count does not fit this platform"))?;
    if count > MAX_BARS_PER_SYMBOL {
        return Err(format!(
            "{label} has {count} bars; the worker limit is {MAX_BARS_PER_SYMBOL}"
        ));
    }
    let total = running_total
        .checked_add(count)
        .ok_or_else(|| format!("{label} bar total overflow"))?;
    if total > MAX_TOTAL_BARS {
        return Err(format!(
            "{label} would raise the run total to {total} bars; the worker limit is {MAX_TOTAL_BARS}"
        ));
    }
    let bars = record
        .load_bars()
        .map_err(|error| format!("{label} payload: {error}"))?;
    *running_total = total;
    Ok(bars)
}

pub(crate) fn execute_intervention_run_job(
    root: &Path,
    job: &InterventionRunJob,
) -> Result<InterventionRunOutput, String> {
    let strategy = StrategyIr::from_json_slice(&read_bounded(
        &job.strategy_path,
        "strategy",
        MAX_SEALED_RUN_ARTIFACT_BYTES,
    )?)
    .map_err(|error| format!("strategy artifact: {error}"))?;
    let config = StrategyExecutionConfig::from_json_slice(&read_bounded(
        &job.config_path,
        "execution config",
        MAX_SEALED_RUN_ARTIFACT_BYTES,
    )?)
    .map_err(|error| format!("execution config artifact: {error}"))?;
    let manifest = StrategyRunManifest::from_json_slice(&read_bounded(
        &job.manifest_path,
        "run manifest",
        MAX_SEALED_RUN_ARTIFACT_BYTES,
    )?)
    .map_err(|error| format!("run manifest artifact: {error}"))?;
    let intervention_log = InterventionLog::from_json_slice(&read_bounded(
        &job.intervention_log_path,
        "candidate intervention log",
        MAX_INTERVENTION_LOG_JSON_BYTES as u64,
    )?)
    .map_err(|error| format!("candidate intervention log: {error}"))?;

    let binding = manifest.binding();
    if binding.datasets.is_empty() || binding.datasets.len() > MAX_SYMBOLS {
        return Err(format!(
            "run requires 1..={MAX_SYMBOLS} parent dataset inputs, found {}",
            binding.datasets.len()
        ));
    }
    match binding.intervention_log_id.as_deref() {
        None => return Err("run manifest does not bind an intervention log".into()),
        Some(expected) if expected != intervention_log.log_id() => {
            return Err(format!(
                "run manifest intervention log id mismatch: expected {expected}, got {}",
                intervention_log.log_id()
            ));
        }
        Some(_) => {}
    }

    let selected: BTreeSet<_> = job
        .selected_dataset_ids
        .iter()
        .map(String::as_str)
        .collect();
    let expected: BTreeSet<_> = binding
        .datasets
        .iter()
        .map(|dataset| dataset.dataset_id.as_str())
        .collect();
    if selected != expected || selected.len() != job.selected_dataset_ids.len() {
        return Err(
            "selected parent dataset IDs must exactly equal the run manifest dataset bindings"
                .into(),
        );
    }

    let store = FileDatasetStore::open(root).map_err(|error| format!("dataset store: {error}"))?;
    let mut loaded = Vec::with_capacity(binding.datasets.len());
    let mut total = 0usize;
    for expected in &binding.datasets {
        let record = store
            .open_record(&expected.dataset_id)
            .map_err(|error| format!("parent input `{}`: {error}", expected.input_id))?;
        loaded.push(LoadedInput {
            input_id: expected.input_id.clone(),
            manifest: record.manifest().clone(),
            bars: load_record_bounded(
                &record,
                &format!("parent input `{}`", expected.input_id),
                &mut total,
            )?,
        });
    }
    let datasets: Vec<_> = loaded
        .iter()
        .map(|input| RunDatasetInput {
            input_id: &input.input_id,
            manifest: &input.manifest,
            bars: &input.bars,
        })
        .collect();
    let verified = assemble_verified_run_with_intervention(
        &strategy,
        &config,
        &manifest,
        &datasets,
        Some(&intervention_log),
    )
    .map_err(|error| {
        format!("verified intervention run assembly rejected the artifacts: {error}")
    })?;
    let report = run_verified_simulation_with_intervention(&verified, Some(&intervention_log))
        .map_err(|error| format!("exact intervention replay failed: {error}"))?;
    let artifact = StrategyReportArtifact::build_for_verified_run(
        &verified,
        &report,
        config.settings().initial_capital,
    )
    .map_err(|error| format!("cannot seal replay report: {error}"))?;
    artifact
        .verify_simulation_report(&report)
        .map_err(|error| format!("replay report identity verification failed: {error}"))?;
    let symbol_id = report
        .symbols
        .iter()
        .position(|symbol| symbol.eq_ignore_ascii_case(&job.chart.symbol))
        .map(SymbolId)
        .ok_or_else(|| {
            format!(
                "verified replay report has no symbol matching chart `{}`",
                job.chart.symbol
            )
        })?;
    let mut view =
        StrategyResultView::prepare(&artifact, &report, symbol_id, &job.chart.bar_times_ms)
            .map_err(|error: StrategyResultViewError| {
                format!("cannot prepare replay report viewer: {error}")
            })?;
    view.report_artifact_json = artifact
        .to_json_vec()
        .map_err(|error| format!("cannot encode sealed replay report: {error}"))?;
    view.simulation_report_json = serde_json::to_vec(&report)
        .map_err(|error| format!("cannot encode replay simulation report: {error}"))?;
    Ok(InterventionRunOutput {
        manifest,
        log_id: intervention_log.log_id().to_owned(),
        view,
        chart: job.chart.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use typhoon_engine::broker::alpaca::Bar;
    use typhoon_engine::core::strategy_dataset::{
        AdjustmentPolicy, CalendarPolicy, DatasetManifestInput, DatasetProvenance, DatasetQaPolicy,
    };
    use typhoon_engine::core::strategy_dataset_store::FileDatasetStore;
    use typhoon_engine::core::strategy_intervention::{
        Intervention, InterventionAction, InterventionLog,
    };
    use typhoon_engine::core::strategy_ir::*;
    use typhoon_engine::core::strategy_simulator::{OrderRequest, OrderSide, SymbolId};

    fn temp_root(label: &str) -> PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let ticket = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "typhoon-intervention-run-{label}-{}-{ticket}",
            std::process::id()
        ))
    }

    fn bar(timestamp: &str) -> Bar {
        Bar {
            timestamp: timestamp.into(),
            open: 100.0,
            high: 101.0,
            low: 99.0,
            close: 100.5,
            volume: 1_000.0,
        }
    }

    fn dataset_input(symbol: &str) -> DatasetManifestInput {
        DatasetManifestInput {
            symbol: symbol.into(),
            timeframe: "1Hour".into(),
            provenance: DatasetProvenance {
                source: "test".into(),
                venue: "test".into(),
                pipeline: "test/v1".into(),
            },
            adjustment: AdjustmentPolicy::Raw,
            calendar: CalendarPolicy::Continuous24x7,
            qa_policy: DatasetQaPolicy::default(),
        }
    }

    fn idle_strategy() -> StrategyIr {
        StrategyIr::build(&StrategyDefinition {
            metadata: StrategyMetadata {
                name: "idle intervention replay".into(),
                author: "test".into(),
                notes: None,
                tags: vec![],
            },
            parameters: vec![],
            indicators: vec![],
            roles: vec![],
            long: DirectionRules {
                enabled: true,
                entry: Condition::Never,
                exit: Condition::Never,
            },
            short: DirectionRules {
                enabled: false,
                entry: Condition::Never,
                exit: Condition::Never,
            },
            session: SessionFilter {
                enabled: false,
                windows: vec![],
                close_positions_outside: false,
            },
            news: NewsFilter {
                enabled: false,
                min_impact: NewsImpact::High,
                block_minutes_before: 0,
                block_minutes_after: 0,
                close_open_positions: false,
            },
            sizing: PositionSizing {
                rule: SizingRule::FixedUnits { units: 1.0 },
                max_open_positions: 1,
            },
            trade_management: TradeManagement {
                legs: vec![TradeLeg {
                    fraction_bps: 10_000,
                    stop: None,
                    target: None,
                    trailing: None,
                }],
                break_even_after: None,
                max_bars_in_trade: None,
            },
            timing: ExecutionTiming {
                decision: DecisionTiming::ClosedBar,
                forming_bar_visible: false,
                submit_delay_bars: 0,
            },
        })
        .expect("strategy")
    }

    fn config() -> StrategyExecutionConfig {
        let mut settings = ExecutionSettings::conservative_defaults();
        settings.initial_capital = 10_000.0;
        StrategyExecutionConfig::build(&settings).expect("config")
    }

    fn trailing_log() -> InterventionLog {
        InterventionLog::build(vec![Intervention {
            decision_index: 99,
            note: "must remain unapplied".into(),
            action: InterventionAction::Submit {
                request: OrderRequest::market(SymbolId(0), OrderSide::Buy, 1.0),
            },
        }])
        .expect("trailing log")
    }

    fn write_artifacts(
        root: &Path,
        dataset_id: &str,
        log: &InterventionLog,
        bound_log_id: Option<String>,
    ) -> InterventionRunJob {
        let strategy = idle_strategy();
        let config = config();
        let manifest = StrategyRunManifest::build(&RunBinding {
            datasets: vec![DatasetBinding {
                input_id: "primary".into(),
                dataset_id: dataset_id.into(),
            }],
            sub_bar_datasets: vec![],
            strategy_id: strategy.strategy_id().into(),
            config_id: config.config_id().into(),
            seed: 7,
            engine_version: "typhoon-engine/test".into(),
            metrics_version: typhoon_engine::core::strategy_metrics::METRICS_SCHEMA_VERSION.into(),
            intervention_log_id: bound_log_id,
            repaint_qa: vec![],
        })
        .expect("manifest");
        let artifact_dir = root.join("artifacts");
        std::fs::create_dir_all(&artifact_dir).unwrap();
        let strategy_path = artifact_dir.join("strategy.json");
        let config_path = artifact_dir.join("config.json");
        let manifest_path = artifact_dir.join("manifest.json");
        let intervention_log_path = artifact_dir.join("intervention.json");
        std::fs::write(&strategy_path, serde_json::to_vec(&strategy).unwrap()).unwrap();
        std::fs::write(&config_path, serde_json::to_vec(&config).unwrap()).unwrap();
        std::fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        std::fs::write(&intervention_log_path, log.to_json_vec().unwrap()).unwrap();
        InterventionRunJob {
            identity: identity(1),
            selected_dataset_ids: vec![dataset_id.into()],
            strategy_path: strategy_path.to_string_lossy().into_owned(),
            config_path: config_path.to_string_lossy().into_owned(),
            manifest_path: manifest_path.to_string_lossy().into_owned(),
            intervention_log_path: intervention_log_path.to_string_lossy().into_owned(),
            chart: RunChartContext {
                chart_index: 0,
                bars_generation: 11,
                symbol: "BTCUSD".into(),
                bar_times_ms: Arc::from([1_704_067_200_000_i64]),
            },
        }
    }

    fn stored_job(root: &Path, log: &InterventionLog) -> (String, InterventionRunJob) {
        let store = FileDatasetStore::open(root).expect("store");
        let record = store
            .build_and_put(&dataset_input("BTCUSD"), &[bar("2024-01-01T00:00:00Z")])
            .expect("dataset");
        let id = record.manifest.dataset_id;
        let job = write_artifacts(root, &id, log, Some(log.log_id().into()));
        (id, job)
    }

    fn identity(generation: u64) -> RunRequestIdentity {
        RunRequestIdentity {
            request_id: generation,
            generation,
        }
    }

    #[test]
    fn real_store_worker_replays_bound_candidate_and_builds_verified_report() {
        let root = temp_root("success");
        let log = InterventionLog::empty();
        let (dataset_id, mut job) = stored_job(&root, &log);
        job.identity = identity(1);
        let worker = InterventionRunWorker::spawn_at(root.clone()).expect("worker");
        worker.submit(job).expect("submit");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let event = loop {
            if let Some(event) = worker.poll().into_iter().next() {
                break event;
            }
            assert!(std::time::Instant::now() < deadline, "worker timeout");
            std::thread::sleep(std::time::Duration::from_millis(5));
        };
        let InterventionRunEvent::Completed { output, .. } = event else {
            panic!("expected completed intervention worker event")
        };
        assert_eq!(output.manifest.binding().datasets[0].dataset_id, dataset_id);
        assert_eq!(
            output.manifest.binding().intervention_log_id.as_deref(),
            Some(log.log_id())
        );
        assert_eq!(output.log_id, log.log_id());
        assert_eq!(output.view.run_id, output.manifest.run_id());
        assert!(!output.view.report_id.is_empty());
        assert!(!output.view.report_artifact_json.is_empty());
        assert!(!output.view.simulation_report_json.is_empty());
        worker.supersede_with(identity(2));
        worker.shutdown();
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn real_store_worker_fails_closed_for_missing_unexpected_and_mismatched_inputs() {
        let missing_root = temp_root("missing");
        let log = InterventionLog::empty();
        let (_, mut missing_job) = stored_job(&missing_root, &log);
        std::fs::remove_file(&missing_job.intervention_log_path).unwrap();
        let missing_log = execute_intervention_run_job(&missing_root, &missing_job).unwrap_err();
        assert!(
            missing_log.contains("candidate intervention log"),
            "{missing_log}"
        );
        std::fs::write(
            &missing_job.intervention_log_path,
            log.to_json_vec().unwrap(),
        )
        .unwrap();
        missing_job.selected_dataset_ids = vec!["f".repeat(64)];
        let missing = execute_intervention_run_job(&missing_root, &missing_job).unwrap_err();
        assert!(missing.contains("selected parent dataset IDs"), "{missing}");

        let unexpected_root = temp_root("unexpected");
        let store = FileDatasetStore::open(&unexpected_root).unwrap();
        let record = store
            .build_and_put(&dataset_input("BTCUSD"), &[bar("2024-01-01T00:00:00Z")])
            .unwrap();
        let unexpected_job =
            write_artifacts(&unexpected_root, &record.manifest.dataset_id, &log, None);
        let unexpected =
            execute_intervention_run_job(&unexpected_root, &unexpected_job).unwrap_err();
        assert!(
            unexpected.contains("does not bind an intervention log"),
            "{unexpected}"
        );

        let mismatch_root = temp_root("mismatch");
        let foreign = trailing_log();
        let (_, mismatch_job) = stored_job(&mismatch_root, &log);
        std::fs::write(
            &mismatch_job.intervention_log_path,
            foreign.to_json_vec().unwrap(),
        )
        .unwrap();
        let mismatch = execute_intervention_run_job(&mismatch_root, &mismatch_job).unwrap_err();
        assert!(
            mismatch.contains("intervention log id mismatch"),
            "{mismatch}"
        );
        let _ = std::fs::remove_dir_all(missing_root);
        let _ = std::fs::remove_dir_all(unexpected_root);
        let _ = std::fs::remove_dir_all(mismatch_root);
    }

    #[test]
    fn real_store_worker_rejects_malformed_and_trailing_logs() {
        let malformed_root = temp_root("malformed");
        let log = InterventionLog::empty();
        let (_, malformed_job) = stored_job(&malformed_root, &log);
        std::fs::write(&malformed_job.intervention_log_path, b"{not-json").unwrap();
        let malformed = execute_intervention_run_job(&malformed_root, &malformed_job).unwrap_err();
        assert!(
            malformed.contains("candidate intervention log"),
            "{malformed}"
        );

        let trailing_root = temp_root("trailing");
        let trailing = trailing_log();
        let (_, trailing_job) = stored_job(&trailing_root, &trailing);
        let error = execute_intervention_run_job(&trailing_root, &trailing_job).unwrap_err();
        assert!(error.contains("applied 0 of 1"), "{error}");
        let _ = std::fs::remove_dir_all(malformed_root);
        let _ = std::fs::remove_dir_all(trailing_root);
    }

    #[test]
    fn ui_state_promotes_all_identities_only_for_current_success() {
        let mut state = InterventionRunState::default();
        state.installed = Some(PromotedRunIdentities {
            run_id: "old-run".into(),
            log_id: "old-log".into(),
            report_id: "old-report".into(),
        });
        let stale = state.begin_request();
        let current = state.begin_request();
        let promoted = PromotedRunIdentities {
            run_id: "run".into(),
            log_id: "log".into(),
            report_id: "report".into(),
        };
        assert!(!state.accept_terminal(stale, Ok(&promoted)));
        assert_eq!(state.installed.as_ref().unwrap().report_id, "old-report");
        assert!(state.accept_terminal(current, Err("fail closed")));
        assert_eq!(state.installed.as_ref().unwrap().report_id, "old-report");
        let success = state.begin_request();
        assert!(state.accept_terminal(success, Ok(&promoted)));
        assert_eq!(state.installed.as_ref(), Some(&promoted));
    }

    #[test]
    fn worker_marks_superseded_and_cancelled_requests_without_promotion() {
        let root = temp_root("stale-cancel");
        let log = InterventionLog::empty();
        let (_, mut job) = stored_job(&root, &log);
        job.identity = identity(1);
        let (job_tx, jobs) = std::sync::mpsc::sync_channel(INTERVENTION_RUN_JOB_QUEUE_CAPACITY);
        let (event_tx, events) =
            std::sync::mpsc::sync_channel(INTERVENTION_RUN_EVENT_QUEUE_CAPACITY);
        job_tx.try_send(job).unwrap();
        drop(job_tx);
        let current = Arc::new(std::sync::Mutex::new(Some(identity(2))));
        run_worker(root.clone(), jobs, event_tx, current);
        assert!(matches!(
            events.try_recv().unwrap(),
            InterventionRunEvent::Cancelled { .. }
        ));

        let mut state = InterventionRunState::default();
        state.installed = Some(PromotedRunIdentities {
            run_id: "old-run".into(),
            log_id: "old-log".into(),
            report_id: "old-report".into(),
        });
        let submitted = state.begin_request();
        let superseding = state.cancel();
        assert!(superseding.generation > submitted.generation);
        assert!(!state.accept_terminal(submitted, Err("late failure")));
        assert_eq!(state.installed.as_ref().unwrap().report_id, "old-report");
        let _ = std::fs::remove_dir_all(root);
    }
}
