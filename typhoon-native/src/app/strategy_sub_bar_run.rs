//! Pure request state and bounded worker foundation for identity-bound sub-bar runs.
//!
//! The UI boundary is intentionally absent: callers must prepare immutable strategy,
//! execution-config, and run-manifest artifacts before submission, then install an
//! output only through [`SubBarRunState::accept_terminal`].

use crate::app::strategy_report_view::{StrategyResultView, StrategyResultViewError};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError, TrySendError};
use std::sync::{Arc, Mutex};
use typhoon_engine::broker::alpaca::Bar;
use typhoon_engine::core::strategy_dataset::DatasetManifest;
use typhoon_engine::core::strategy_dataset_store::{
    DatasetRecord, DatasetRecordSummary, FileDatasetStore,
};
use typhoon_engine::core::strategy_ir::{
    FidelityLevel, StrategyExecutionConfig, StrategyIr, StrategyRunManifest,
};
use typhoon_engine::core::strategy_report::StrategyReportArtifact;
use typhoon_engine::core::strategy_run::{
    RunDatasetInput, RunSubBarDatasetInput, assemble_verified_run_with_sub_bars,
};
use typhoon_engine::core::strategy_simulator::{
    MAX_BARS_PER_SYMBOL, MAX_SYMBOLS, MAX_TOTAL_BARS, MAX_TOTAL_SUB_BARS, SymbolId,
    run_verified_simulation,
};

pub(crate) const STRATEGY_RUN_JOB_QUEUE_CAPACITY: usize = 1;
pub(crate) const STRATEGY_RUN_EVENT_QUEUE_CAPACITY: usize = 2;
pub(crate) const MAX_STRATEGY_RUN_EVENTS_PER_POLL: usize = 2;
pub(crate) const MAX_RUN_ARTIFACT_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Debug, Default)]
pub(crate) struct SubBarRunUiState {
    pub(crate) parent_dataset_id: String,
    pub(crate) finer_dataset_id: String,
    pub(crate) strategy_path: String,
    pub(crate) config_path: String,
    pub(crate) manifest_path: String,
}

fn read_bounded(path: &str, label: &str) -> Result<Vec<u8>, String> {
    if path.trim().is_empty() {
        return Err(format!("select a sealed {label} JSON artifact"));
    }
    let metadata = std::fs::metadata(path).map_err(|error| format!("{label} `{path}`: {error}"))?;
    if metadata.len() == 0 || metadata.len() > MAX_RUN_ARTIFACT_BYTES {
        return Err(format!(
            "{label} must contain 1..={MAX_RUN_ARTIFACT_BYTES} bytes"
        ));
    }
    std::fs::read(path).map_err(|error| format!("{label} `{path}`: {error}"))
}

pub(crate) fn load_execution_config(path: &str) -> Result<StrategyExecutionConfig, String> {
    StrategyExecutionConfig::from_json_slice(&read_bounded(path, "execution config")?)
        .map_err(|error| format!("execution config artifact: {error}"))
}

pub(crate) fn load_sealed_artifacts(
    ui: &SubBarRunUiState,
) -> Result<(StrategyIr, StrategyExecutionConfig, StrategyRunManifest), String> {
    let strategy = StrategyIr::from_json_slice(&read_bounded(&ui.strategy_path, "strategy")?)
        .map_err(|error| format!("strategy artifact: {error}"))?;
    let config = load_execution_config(&ui.config_path)?;
    let manifest =
        StrategyRunManifest::from_json_slice(&read_bounded(&ui.manifest_path, "run manifest")?)
            .map_err(|error| format!("run manifest artifact: {error}"))?;
    Ok((strategy, config, manifest))
}

#[derive(Debug, Clone)]
pub(crate) struct ValidatedRunSelection {
    pub(crate) parent: DatasetRecordSummary,
    pub(crate) finer: DatasetRecordSummary,
    pub(crate) sub_bar_seconds: u32,
}

pub(crate) fn fixed_timeframe_seconds(timeframe: &str) -> Option<u32> {
    let digits = timeframe
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(timeframe.len());
    if digits == 0 {
        return None;
    }
    let count = timeframe[..digits].parse::<u32>().ok()?;
    let unit = match &timeframe[digits..] {
        "Min" => 60_u32,
        "Hour" => 3_600,
        "Day" => 86_400,
        "Week" => 604_800,
        _ => return None,
    };
    count.checked_mul(unit).filter(|seconds| *seconds > 0)
}

pub(crate) fn validate_run_selection(
    parent_id: &str,
    finer_id: &str,
    records: &[DatasetRecordSummary],
) -> Result<ValidatedRunSelection, String> {
    let resolve = |role: &str, id: &str| {
        records
            .iter()
            .find(|record| record.dataset_id == id)
            .cloned()
            .ok_or_else(|| {
                format!(
                    "{role} dataset `{id}` is not in the bounded dataset list; refresh and reselect"
                )
            })
    };
    let parent = resolve("parent", parent_id)?;
    let finer = resolve("finer", finer_id)?;
    if parent.symbol != finer.symbol {
        return Err(format!(
            "finer dataset symbol `{}` does not match parent symbol `{}`",
            finer.symbol, parent.symbol
        ));
    }
    if parent.adjustment != finer.adjustment {
        return Err(format!(
            "finer dataset adjustment `{}` does not match parent adjustment `{}`",
            finer.adjustment.wire_id(),
            parent.adjustment.wire_id()
        ));
    }
    if parent.calendar_policy_id != finer.calendar_policy_id {
        return Err("finer dataset calendar policy does not match parent calendar policy".into());
    }
    let parent_seconds = fixed_timeframe_seconds(&parent.timeframe).ok_or_else(|| {
        format!(
            "parent `{}` is not a supported fixed timeframe",
            parent.timeframe
        )
    })?;
    let finer_seconds = fixed_timeframe_seconds(&finer.timeframe).ok_or_else(|| {
        format!(
            "finer `{}` is not a supported fixed timeframe",
            finer.timeframe
        )
    })?;
    if finer_seconds >= parent_seconds {
        return Err(format!(
            "finer timeframe `{}` must be strictly finer than parent timeframe `{}`",
            finer.timeframe, parent.timeframe
        ));
    }
    Ok(ValidatedRunSelection {
        parent,
        finer,
        sub_bar_seconds: finer_seconds,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RunRequestIdentity {
    pub(crate) request_id: u64,
    pub(crate) generation: u64,
}

#[derive(Debug, Default)]
pub(crate) struct SubBarRunState {
    pending: Option<RunRequestIdentity>,
    next_request_id: u64,
    next_generation: u64,
    pub(crate) status: String,
    pub(crate) installed_report_id: Option<String>,
}

impl SubBarRunState {
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

    /// Accept only the exact currently-pending generation. The caller may install
    /// a completed view only when this returns `true`.
    pub(crate) fn accept_terminal(
        &mut self,
        identity: RunRequestIdentity,
        result: Result<&str, &str>,
    ) -> bool {
        if self.pending != Some(identity) {
            return false;
        }
        self.pending = None;
        match result {
            Ok(report_id) => {
                self.status = format!("Verified sub-bar report {report_id} ready");
                self.installed_report_id = Some(report_id.to_owned());
            }
            Err(error) => self.status = format!("Error: {error}"),
        }
        true
    }

    pub(crate) fn cancel(&mut self) -> RunRequestIdentity {
        let identity = self.begin_request();
        self.pending = None;
        self.status = "Cancelled; queued and in-flight results were superseded".into();
        identity
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RunChartContext {
    pub(crate) chart_index: usize,
    pub(crate) bars_generation: u64,
    pub(crate) symbol: String,
    pub(crate) bar_times_ms: Arc<[i64]>,
}

/// A self-contained immutable request. Dataset payloads are resolved by the exact
/// ids sealed into `manifest`; no mutable path or UI selection is consulted by the
/// worker.
#[derive(Debug, Clone)]
pub(crate) struct StrategyRunJob {
    pub(crate) identity: RunRequestIdentity,
    pub(crate) strategy: StrategyIr,
    pub(crate) config: StrategyExecutionConfig,
    pub(crate) manifest: StrategyRunManifest,
    pub(crate) chart: RunChartContext,
}

#[derive(Debug)]
pub(crate) struct StrategyRunOutput {
    pub(crate) manifest: StrategyRunManifest,
    pub(crate) view: StrategyResultView,
    pub(crate) chart: RunChartContext,
}

#[derive(Debug)]
pub(crate) enum StrategyRunEvent {
    Completed {
        identity: RunRequestIdentity,
        output: Box<StrategyRunOutput>,
    },
    Failed {
        identity: RunRequestIdentity,
        message: String,
    },
    Cancelled {
        identity: RunRequestIdentity,
    },
}

impl StrategyRunEvent {
    pub(crate) fn identity(&self) -> RunRequestIdentity {
        match self {
            Self::Completed { identity, .. }
            | Self::Failed { identity, .. }
            | Self::Cancelled { identity } => *identity,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StrategyRunSubmitError {
    QueueFull,
    WorkerStopped,
}

impl std::fmt::Display for StrategyRunSubmitError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::QueueFull => write!(formatter, "verified-run worker queue is full"),
            Self::WorkerStopped => write!(formatter, "verified-run worker is not running"),
        }
    }
}

#[derive(Debug)]
pub(crate) struct StrategyRunWorker {
    jobs: SyncSender<StrategyRunJob>,
    events: Receiver<StrategyRunEvent>,
    current: Arc<Mutex<Option<RunRequestIdentity>>>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl StrategyRunWorker {
    pub(crate) fn spawn_at(root: PathBuf) -> Result<Self, std::io::Error> {
        let (job_tx, jobs) = std::sync::mpsc::sync_channel(STRATEGY_RUN_JOB_QUEUE_CAPACITY);
        let (event_tx, events) = std::sync::mpsc::sync_channel(STRATEGY_RUN_EVENT_QUEUE_CAPACITY);
        let current = Arc::new(Mutex::new(None));
        let worker_current = Arc::clone(&current);
        let handle = std::thread::Builder::new()
            .name("typhoon-verified-run-worker".into())
            .spawn(move || run_worker(root, jobs, event_tx, worker_current))?;
        Ok(Self {
            jobs: job_tx,
            events,
            current,
            handle: Some(handle),
        })
    }

    pub(crate) fn submit(&self, job: StrategyRunJob) -> Result<(), StrategyRunSubmitError> {
        let identity = job.identity;
        let mut current = self.current.lock().expect("run-current mutex poisoned");
        let previous = current.replace(identity);
        match self.jobs.try_send(job) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => {
                *current = previous;
                Err(StrategyRunSubmitError::QueueFull)
            }
            Err(TrySendError::Disconnected(_)) => {
                *current = previous;
                Err(StrategyRunSubmitError::WorkerStopped)
            }
        }
    }

    pub(crate) fn supersede_with(&self, identity: RunRequestIdentity) {
        *self.current.lock().expect("run-current mutex poisoned") = Some(identity);
    }

    pub(crate) fn poll(&self) -> Vec<StrategyRunEvent> {
        let mut events = Vec::new();
        while events.len() < MAX_STRATEGY_RUN_EVENTS_PER_POLL {
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
    sender: &SyncSender<StrategyRunEvent>,
    current: &Mutex<Option<RunRequestIdentity>>,
    identity: RunRequestIdentity,
    mut event: StrategyRunEvent,
) -> bool {
    loop {
        let guard = current.lock().expect("run-current mutex poisoned");
        if *guard != Some(identity) {
            event = StrategyRunEvent::Cancelled { identity };
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
    jobs: Receiver<StrategyRunJob>,
    events: SyncSender<StrategyRunEvent>,
    current: Arc<Mutex<Option<RunRequestIdentity>>>,
) {
    while let Ok(job) = jobs.recv() {
        let identity = job.identity;
        let is_current = *current.lock().expect("run-current mutex poisoned") == Some(identity);
        let event = if !is_current {
            StrategyRunEvent::Cancelled { identity }
        } else {
            match execute_strategy_run_job(&root, &job) {
                Ok(output) => StrategyRunEvent::Completed {
                    identity,
                    output: Box::new(output),
                },
                Err(message) => StrategyRunEvent::Failed { identity, message },
            }
        };
        if !emit_terminal(&events, &current, identity, event) {
            return;
        }
    }
}

struct LoadedInput {
    input_id: String,
    parent_manifest: DatasetManifest,
    parent_bars: Vec<Bar>,
    finer_manifest: DatasetManifest,
    finer_bars: Vec<Bar>,
}

fn load_record_bounded(
    record: &DatasetRecord,
    label: &str,
    per_record_limit: usize,
    running_total: &mut usize,
    total_limit: usize,
) -> Result<Vec<Bar>, String> {
    let count = usize::try_from(record.bar_count())
        .map_err(|_| format!("{label} bar count does not fit this platform"))?;
    if count > per_record_limit {
        return Err(format!(
            "{label} has {count} bars; the worker limit is {per_record_limit}"
        ));
    }
    let total = running_total
        .checked_add(count)
        .ok_or_else(|| format!("{label} bar total overflow"))?;
    if total > total_limit {
        return Err(format!(
            "{label} would raise the run total to {total} bars; the worker limit is {total_limit}"
        ));
    }
    let bars = record
        .load_bars()
        .map_err(|error| format!("{label} payload: {error}"))?;
    *running_total = total;
    Ok(bars)
}

pub(crate) fn execute_strategy_run_job(
    root: &Path,
    job: &StrategyRunJob,
) -> Result<StrategyRunOutput, String> {
    job.strategy
        .verify()
        .map_err(|error| format!("invalid strategy artifact: {error}"))?;
    job.config
        .verify()
        .map_err(|error| format!("invalid execution config: {error}"))?;
    job.manifest
        .verify()
        .map_err(|error| format!("invalid run manifest: {error}"))?;
    let binding = job.manifest.binding();
    if binding.datasets.is_empty() || binding.datasets.len() > MAX_SYMBOLS {
        return Err(format!(
            "run requires 1..={MAX_SYMBOLS} dataset inputs, found {}",
            binding.datasets.len()
        ));
    }
    let FidelityLevel::SubBar { sub_bar_seconds } = job.config.settings().fidelity else {
        return Err("execution config does not select sub-bar fidelity".into());
    };
    if binding.sub_bar_datasets.len() != binding.datasets.len() {
        return Err("run manifest does not bind exactly one finer dataset per parent input".into());
    }

    let store = FileDatasetStore::open(root).map_err(|error| format!("dataset store: {error}"))?;
    let mut loaded = Vec::with_capacity(binding.datasets.len());
    let mut parent_total = 0usize;
    let mut finer_total = 0usize;
    for parent_binding in &binding.datasets {
        let finer_binding = binding
            .sub_bar_datasets
            .iter()
            .find(|candidate| candidate.parent_input_id == parent_binding.input_id)
            .ok_or_else(|| {
                format!(
                    "run manifest has no finer dataset for parent input `{}`",
                    parent_binding.input_id
                )
            })?;
        let parent = store
            .open_record(&parent_binding.dataset_id)
            .map_err(|error| format!("parent input `{}`: {error}", parent_binding.input_id))?;
        let finer = store
            .open_record(&finer_binding.dataset_id)
            .map_err(|error| format!("finer input `{}`: {error}", parent_binding.input_id))?;
        let actual_seconds =
            fixed_timeframe_seconds(&finer.manifest().timeframe).ok_or_else(|| {
                format!(
                    "finer input `{}` has unsupported fixed timeframe `{}`",
                    parent_binding.input_id,
                    finer.manifest().timeframe
                )
            })?;
        if actual_seconds != sub_bar_seconds {
            return Err(format!(
                "finer input `{}` is {actual_seconds}s but config binds {sub_bar_seconds}s",
                parent_binding.input_id
            ));
        }
        loaded.push(LoadedInput {
            input_id: parent_binding.input_id.clone(),
            parent_manifest: parent.manifest().clone(),
            parent_bars: load_record_bounded(
                &parent,
                &format!("parent input `{}`", parent_binding.input_id),
                MAX_BARS_PER_SYMBOL,
                &mut parent_total,
                MAX_TOTAL_BARS,
            )?,
            finer_manifest: finer.manifest().clone(),
            finer_bars: load_record_bounded(
                &finer,
                &format!("finer input `{}`", parent_binding.input_id),
                MAX_TOTAL_SUB_BARS,
                &mut finer_total,
                MAX_TOTAL_SUB_BARS,
            )?,
        });
    }

    let parents: Vec<_> = loaded
        .iter()
        .map(|input| RunDatasetInput {
            input_id: &input.input_id,
            manifest: &input.parent_manifest,
            bars: &input.parent_bars,
        })
        .collect();
    let finer: Vec<_> = loaded
        .iter()
        .map(|input| RunSubBarDatasetInput {
            parent_input_id: &input.input_id,
            manifest: &input.finer_manifest,
            bars: &input.finer_bars,
        })
        .collect();
    let verified = assemble_verified_run_with_sub_bars(
        &job.strategy,
        &job.config,
        &job.manifest,
        &parents,
        &finer,
    )
    .map_err(|error| format!("verified run assembly rejected the selected datasets: {error}"))?;
    let report = run_verified_simulation(&verified)
        .map_err(|error| format!("verified sub-bar simulation failed: {error}"))?;
    let artifact = StrategyReportArtifact::build_for_verified_run(
        &verified,
        &report,
        job.config.settings().initial_capital,
    )
    .map_err(|error| format!("cannot seal verified report: {error}"))?;
    artifact
        .verify_simulation_report(&report)
        .map_err(|error| format!("report identity verification failed: {error}"))?;
    let symbol_id = report
        .symbols
        .iter()
        .position(|symbol| symbol.eq_ignore_ascii_case(&job.chart.symbol))
        .map(SymbolId)
        .ok_or_else(|| {
            format!(
                "verified report has no symbol matching chart `{}`",
                job.chart.symbol
            )
        })?;
    let mut view =
        StrategyResultView::prepare(&artifact, &report, symbol_id, &job.chart.bar_times_ms)
            .map_err(|error: StrategyResultViewError| {
                format!("cannot prepare report viewer: {error}")
            })?;
    view.report_artifact_json = artifact
        .to_json_vec()
        .map_err(|error| format!("cannot encode sealed report: {error}"))?;
    view.simulation_report_json = serde_json::to_vec(&report)
        .map_err(|error| format!("cannot encode simulation report: {error}"))?;
    Ok(StrategyRunOutput {
        manifest: job.manifest.clone(),
        view,
        chart: job.chart.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use typhoon_engine::core::strategy_dataset::{
        AdjustmentPolicy, CalendarPolicy, DatasetManifestInput, DatasetProvenance, DatasetQaPolicy,
    };
    use typhoon_engine::core::strategy_ir::*;

    fn temp_root(label: &str) -> PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let ticket = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "typhoon-sub-bar-run-{label}-{}-{ticket}",
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

    fn dataset_input(symbol: &str, timeframe: &str) -> DatasetManifestInput {
        DatasetManifestInput {
            symbol: symbol.into(),
            timeframe: timeframe.into(),
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

    fn store_pair(root: &Path, finer_symbol: &str) -> (String, String) {
        let store = FileDatasetStore::open(root).expect("store");
        let parent = store
            .build_and_put(
                &dataset_input("BTCUSD", "1Hour"),
                &[bar("2024-01-01T00:00:00Z")],
            )
            .expect("parent");
        let finer = store
            .build_and_put(
                &dataset_input(finer_symbol, "15Min"),
                &[
                    bar("2024-01-01T00:00:00Z"),
                    bar("2024-01-01T00:15:00Z"),
                    bar("2024-01-01T00:30:00Z"),
                    bar("2024-01-01T00:45:00Z"),
                ],
            )
            .expect("finer");
        (parent.manifest.dataset_id, finer.manifest.dataset_id)
    }

    fn idle_strategy() -> StrategyIr {
        StrategyIr::build(&StrategyDefinition {
            metadata: StrategyMetadata {
                name: "idle".into(),
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

    fn job(parent: String, finer: String, identity: RunRequestIdentity) -> StrategyRunJob {
        let strategy = idle_strategy();
        let mut settings = ExecutionSettings::conservative_defaults();
        settings.fidelity = FidelityLevel::SubBar {
            sub_bar_seconds: 900,
        };
        settings.initial_capital = 10_000.0;
        let config = StrategyExecutionConfig::build(&settings).expect("config");
        let manifest = StrategyRunManifest::build(&RunBinding {
            datasets: vec![DatasetBinding {
                input_id: "primary".into(),
                dataset_id: parent,
            }],
            sub_bar_datasets: vec![SubBarDatasetBinding {
                parent_input_id: "primary".into(),
                dataset_id: finer,
            }],
            strategy_id: strategy.strategy_id().into(),
            config_id: config.config_id().into(),
            seed: 7,
            engine_version: "typhoon-engine/test".into(),
            metrics_version: typhoon_engine::core::strategy_metrics::METRICS_SCHEMA_VERSION.into(),
            intervention_log_id: None,
            repaint_qa: vec![],
        })
        .expect("manifest");
        StrategyRunJob {
            identity,
            strategy,
            config,
            manifest,
            chart: RunChartContext {
                chart_index: 0,
                bars_generation: 11,
                symbol: "BTCUSD".into(),
                bar_times_ms: Arc::from([1_704_067_200_000_i64]),
            },
        }
    }

    fn identity(generation: u64) -> RunRequestIdentity {
        RunRequestIdentity {
            request_id: generation,
            generation,
        }
    }

    #[test]
    fn selection_requires_exact_compatible_parent_and_finer_records() {
        let root = temp_root("selection");
        let (parent_id, finer_id) = store_pair(&root, "BTCUSD");
        let records = FileDatasetStore::open(&root).unwrap().list(8).unwrap();
        let selected = validate_run_selection(&parent_id, &finer_id, &records).expect("pair");
        assert_eq!(selected.parent.dataset_id, parent_id);
        assert_eq!(selected.finer.dataset_id, finer_id);
        assert_eq!(selected.sub_bar_seconds, 900);
        assert!(validate_run_selection(&parent_id, "missing", &records).is_err());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn state_rejects_stale_generation_and_preserves_installed_report_on_failure() {
        let mut state = SubBarRunState::default();
        state.installed_report_id = Some("old".into());
        let stale = state.begin_request();
        let current = state.begin_request();
        assert!(!state.accept_terminal(stale, Ok("stale")));
        assert_eq!(state.installed_report_id.as_deref(), Some("old"));
        assert!(state.accept_terminal(current, Err("precise failure")));
        assert_eq!(state.installed_report_id.as_deref(), Some("old"));
    }

    #[test]
    fn state_submission_busy_cancel_and_stale_completion_are_explicit() {
        let mut state = SubBarRunState::default();
        let submitted = state.begin_request();
        assert!(state.is_busy());
        let cancel_generation = state.cancel();
        assert!(!state.is_busy());
        assert!(cancel_generation.generation > submitted.generation);
        assert!(!state.accept_terminal(submitted, Ok("stale")));
        assert_eq!(state.installed_report_id, None);
        assert!(state.status.starts_with("Cancelled"));
    }

    #[test]
    fn worker_busy_queue_rejects_a_second_submission_without_replacing_identity() {
        let (jobs, job_rx) = std::sync::mpsc::sync_channel(1);
        let (_events, event_rx) = std::sync::mpsc::sync_channel(1);
        let current = Arc::new(Mutex::new(None));
        let worker = StrategyRunWorker {
            jobs,
            events: event_rx,
            current: Arc::clone(&current),
            handle: None,
        };
        worker
            .jobs
            .try_send(job("a".repeat(64), "b".repeat(64), identity(1)))
            .unwrap();
        assert_eq!(
            worker.submit(job("c".repeat(64), "d".repeat(64), identity(2))),
            Err(StrategyRunSubmitError::QueueFull)
        );
        assert_eq!(*current.lock().unwrap(), None);
        drop(job_rx);
    }

    #[test]
    fn worker_executes_exact_identity_bound_pair_and_prepares_verified_view() {
        let root = temp_root("success");
        let (parent, finer) = store_pair(&root, "BTCUSD");
        let worker = StrategyRunWorker::spawn_at(root.clone()).expect("worker");
        worker
            .submit(job(parent.clone(), finer.clone(), identity(1)))
            .expect("submit");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let event = loop {
            if let Some(event) = worker.poll().into_iter().next() {
                break event;
            }
            assert!(std::time::Instant::now() < deadline, "worker timeout");
            std::thread::sleep(std::time::Duration::from_millis(5));
        };
        let StrategyRunEvent::Completed {
            identity: completed_identity,
            output,
        } = event
        else {
            panic!("expected completed worker event")
        };
        assert_eq!(completed_identity, identity(1));
        assert_eq!(output.manifest.schema_version(), 5);
        assert_eq!(output.manifest.binding().datasets[0].dataset_id, parent);
        assert_eq!(
            output.manifest.binding().sub_bar_datasets[0].dataset_id,
            finer
        );
        assert_eq!(output.view.run_id, output.manifest.run_id());
        assert_eq!(output.view.symbol, "BTCUSD");
        assert_eq!(output.chart.chart_index, 0);
        assert_eq!(output.chart.bars_generation, 11);
        assert!(!output.view.report_artifact_json.is_empty());
        assert!(!output.view.simulation_report_json.is_empty());
        worker.supersede_with(identity(2));
        worker.shutdown();
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn worker_fails_closed_for_missing_and_mismatched_records() {
        let mismatch_root = temp_root("mismatch");
        let (parent, finer) = store_pair(&mismatch_root, "ETHUSD");
        let mismatch =
            execute_strategy_run_job(&mismatch_root, &job(parent, finer, identity(1))).unwrap_err();
        assert!(mismatch.contains("symbol"), "{mismatch}");

        let missing_root = temp_root("missing");
        let (parent, _) = store_pair(&missing_root, "BTCUSD");
        let missing =
            execute_strategy_run_job(&missing_root, &job(parent, "e".repeat(64), identity(2)))
                .unwrap_err();
        assert!(missing.contains("no stored dataset"), "{missing}");
        let _ = std::fs::remove_dir_all(mismatch_root);
        let _ = std::fs::remove_dir_all(missing_root);
    }

    #[test]
    fn dataset_record_is_refused_before_unbounded_load() {
        let root = temp_root("bounds");
        let store = FileDatasetStore::open(&root).unwrap();
        let stored = store
            .build_and_put(
                &dataset_input("BTCUSD", "1Min"),
                &[bar("2024-01-01T00:00:00Z"), bar("2024-01-01T00:01:00Z")],
            )
            .unwrap();
        let record = store.open_record(&stored.manifest.dataset_id).unwrap();
        let mut total = 0;
        let error = load_record_bounded(&record, "bounded fixture", 1, &mut total, 1).unwrap_err();
        assert!(error.contains("worker limit is 1"), "{error}");
        assert_eq!(total, 0);
        assert_eq!(STRATEGY_RUN_JOB_QUEUE_CAPACITY, 1);
        assert!(MAX_STRATEGY_RUN_EVENTS_PER_POLL <= STRATEGY_RUN_EVENT_QUEUE_CAPACITY);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn worker_never_executes_a_job_from_a_superseded_generation() {
        let root = temp_root("stale-worker");
        let (parent, finer) = store_pair(&root, "BTCUSD");
        let (job_tx, jobs) = std::sync::mpsc::sync_channel(STRATEGY_RUN_JOB_QUEUE_CAPACITY);
        let (event_tx, events) = std::sync::mpsc::sync_channel(STRATEGY_RUN_EVENT_QUEUE_CAPACITY);
        job_tx
            .try_send(job(parent, finer, identity(1)))
            .expect("queue stale job");
        drop(job_tx);
        let current = Arc::new(Mutex::new(Some(identity(2))));
        run_worker(root.clone(), jobs, event_tx, current);
        let event = events.try_recv().expect("cancelled event");
        assert_eq!(event.identity(), identity(1));
        assert!(matches!(event, StrategyRunEvent::Cancelled { .. }));
        assert!(events.try_recv().is_err());
        let _ = std::fs::remove_dir_all(root);
    }
}
