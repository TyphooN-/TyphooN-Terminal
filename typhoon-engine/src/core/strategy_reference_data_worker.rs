//! Off-render-thread reference-data loading, materialization and selection —
//! the ADR-135 M2 background-work boundary.
//!
//! Every filesystem read, digest, verify and bind in the reference-data path
//! happens on this worker thread. The GUI submits a job and drains events; it
//! never opens a snapshot, hashes a record, or builds a config on the frame
//! thread ([ADR-098], [ADR-134]).
//!
//! [ADR-098]: ../../../../docs/adr/098-per-frame-o-1-discipline-in-chart-and-sync-paths.md
//! [ADR-134]: ../../../../docs/adr/134-render-independent-background-pump.md
//!
//! ## What it will not do
//!
//! It does not fetch. A snapshot is bytes some other process already persisted,
//! and this worker's job is to say honestly what those bytes are: which system
//! produced them, what authority that system carries, whether they cover the
//! range they claim, and — before anything is promoted — whether they would
//! materialize at all. A snapshot that cannot clear its own bar is reported as
//! blocked, never quietly downgraded to a rule-derived stand-in.

use crate::core::strategy_ir::{ExecutionSettings, StrategyExecutionConfig};
use crate::core::strategy_reference_data::{
    CalendarExceptionArtifact, CalendarMaterializationRequest, CorporateActionArtifact,
    CorporateActionMaterializationRequest, MAX_REFERENCE_ARTIFACT_BYTES, ReferenceArtifactKind,
    ReferenceArtifactStore, ReferenceDataError, SourceBatch, SourceCoverage,
    bind_reference_artifacts, materialize_calendar, materialize_corporate_actions,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError, TrySendError};

/// In-flight jobs the worker accepts before reporting backpressure.
pub const REFERENCE_JOB_QUEUE_CAPACITY: usize = 4;
/// Events the worker may have outstanding before it parks.
pub const REFERENCE_EVENT_QUEUE_CAPACITY: usize = 32;
/// Events one [`ReferenceDataWorker::poll`] may return, so a frame's drain cost
/// has a ceiling however far behind the UI has fallen.
pub const MAX_REFERENCE_EVENTS_PER_POLL: usize = 8;
/// Artifacts one listing may summarize. A larger store lists a prefix and says
/// so rather than growing the window's memory.
pub const MAX_LISTED_ARTIFACTS: usize = 256;
/// Ceiling on a persisted snapshot file, applied before it is parsed.
pub const MAX_SNAPSHOT_BYTES: usize = MAX_REFERENCE_ARTIFACT_BYTES;

/// How long the worker parks when the event queue is full.
const EVENT_PARK: std::time::Duration = std::time::Duration::from_millis(1);

// ── Persisted snapshots ────────────────────────────────────────────

/// A raw source snapshot as some fetcher persisted it: exactly one
/// materialization request, with every provider record and its raw bytes.
///
/// This is the only thing the worker will load. It is deliberately the request
/// type rather than a looser "records plus some metadata", so a snapshot that
/// omits its authority, coverage or completeness claim fails to parse instead
/// of being materialized under assumed defaults.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ReferenceSourceSnapshot {
    Calendar(Box<CalendarMaterializationRequest>),
    CorporateActions(Box<CorporateActionMaterializationRequest>),
}

impl ReferenceSourceSnapshot {
    pub const fn kind_label(&self) -> &'static str {
        match self {
            Self::Calendar(_) => "calendar exceptions",
            Self::CorporateActions(_) => "corporate actions",
        }
    }

    fn source(&self) -> &SourceBatch {
        match self {
            Self::Calendar(request) => &request.source,
            Self::CorporateActions(request) => &request.source,
        }
    }

    fn scope(&self) -> String {
        match self {
            Self::Calendar(request) => {
                format!("{} · {}", request.venue, request.time_zone.wire_id())
            }
            Self::CorporateActions(request) => format!(
                "{} · {} · {}",
                request.venue, request.symbol, request.currency
            ),
        }
    }

    fn requested_range(&self) -> String {
        match self {
            Self::Calendar(request) => format!(
                "{} … {} (exchange dates)",
                request.range_start, request.range_end_inclusive
            ),
            Self::CorporateActions(request) => format!(
                "{} … {} (UTC ns)",
                request.range_start_ns, request.range_end_ns
            ),
        }
    }

    fn record_count(&self) -> usize {
        match self {
            Self::Calendar(request) => request.records.len(),
            Self::CorporateActions(request) => request.records.len(),
        }
    }

    const fn require_authoritative(&self) -> bool {
        match self {
            Self::Calendar(request) => request.require_authoritative,
            Self::CorporateActions(request) => request.require_authoritative,
        }
    }

    /// Materialize and store. The artifact is verified by construction, and the
    /// store is content-addressed, so re-materializing identical bytes is a
    /// no-op rather than a second artifact.
    fn materialize_into(
        &self,
        store: &ReferenceArtifactStore,
    ) -> Result<ReferenceArtifactSummary, ReferenceDataError> {
        match self {
            Self::Calendar(request) => {
                let artifact = materialize_calendar(request)?;
                store.put_calendar(&artifact)?;
                Ok(ReferenceArtifactSummary::calendar(&artifact))
            }
            Self::CorporateActions(request) => {
                let artifact = materialize_corporate_actions(request)?;
                store.put_corporate_actions(&artifact)?;
                Ok(ReferenceArtifactSummary::corporate_actions(&artifact))
            }
        }
    }
}

fn describe_coverage(coverage: &SourceCoverage) -> String {
    match coverage {
        SourceCoverage::ExchangeDateRange {
            start,
            end_inclusive,
        } => format!("{start} … {end_inclusive} (exchange dates)"),
        SourceCoverage::UtcRange { start_ns, end_ns } => {
            format!("{start_ns} … {end_ns} (UTC ns)")
        }
    }
}

/// What a snapshot claims, plus whether it would actually materialize.
///
/// `blocked` is the honest half: it carries the exact refusal a promotion
/// attempt would hit, so the UI can disable the button *and* say why instead of
/// offering an action that will fail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceSourceSummary {
    pub path: String,
    pub kind: &'static str,
    pub scope: String,
    pub source_system: String,
    pub authority: &'static str,
    /// Exchange-published or contracted-vendor. Rule-derived and keyless-public
    /// feeds are false here, whatever the snapshot would like to claim.
    pub authoritative: bool,
    pub complete: bool,
    pub covered_range: String,
    pub requested_range: String,
    pub as_of_ns: i64,
    pub retrieved_at_ns: i64,
    pub record_count: usize,
    pub require_authoritative: bool,
    /// The refusal materializing this snapshot produces, if it produces one.
    pub blocked: Option<String>,
}

impl ReferenceSourceSummary {
    fn describe(path: &str, snapshot: &ReferenceSourceSnapshot, blocked: Option<String>) -> Self {
        let source = snapshot.source();
        Self {
            path: path.to_string(),
            kind: snapshot.kind_label(),
            scope: snapshot.scope(),
            source_system: serde_json::to_string(&source.source)
                .unwrap_or_else(|_| "unserializable".to_string()),
            authority: source.authority.wire_id(),
            authoritative: source.authority.is_authoritative(),
            complete: source.complete,
            covered_range: describe_coverage(&source.coverage),
            requested_range: snapshot.requested_range(),
            as_of_ns: source.as_of_ns,
            retrieved_at_ns: source.retrieved_at_ns,
            record_count: snapshot.record_count(),
            require_authoritative: snapshot.require_authoritative(),
            blocked,
        }
    }

    /// Whether this snapshot can be promoted to a sealed artifact right now.
    pub const fn is_promotable(&self) -> bool {
        self.blocked.is_none()
    }
}

/// A sealed artifact reduced to what a picker needs. Never holds the artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceArtifactSummary {
    pub artifact_id: String,
    pub kind: &'static str,
    pub scope: String,
    pub symbol: Option<String>,
    pub currency: Option<String>,
    pub source_system: String,
    pub authority: &'static str,
    pub authoritative: bool,
    pub covered_range: String,
    pub as_of_ns: i64,
    pub record_count: usize,
    /// Executable events sealed in the artifact: exceptions, or actions.
    pub event_count: usize,
    /// The dataset adjustment policy a corporate-action artifact was checked
    /// against. `None` for a calendar, which has no price series to double-count.
    pub adjustment: Option<&'static str>,
}

impl ReferenceArtifactSummary {
    fn calendar(artifact: &CalendarExceptionArtifact) -> Self {
        let (start, end) = artifact.covered_range();
        let source = artifact.source();
        Self {
            artifact_id: artifact.artifact_id().to_string(),
            kind: "calendar exceptions",
            scope: format!("{} · {}", artifact.venue(), artifact.time_zone().wire_id()),
            symbol: None,
            currency: None,
            source_system: serde_json::to_string(&source.source)
                .unwrap_or_else(|_| "unserializable".to_string()),
            authority: source.authority.wire_id(),
            authoritative: artifact.is_authoritative(),
            covered_range: format!("{start} … {end} (exchange dates)"),
            as_of_ns: source.as_of_ns,
            record_count: artifact.source_records().len(),
            event_count: artifact.exceptions().len(),
            adjustment: None,
        }
    }

    fn corporate_actions(artifact: &CorporateActionArtifact) -> Self {
        let (start, end) = artifact.covered_range_ns();
        let source = artifact.source();
        Self {
            artifact_id: artifact.artifact_id().to_string(),
            kind: "corporate actions",
            scope: format!(
                "{} · {} · {}",
                artifact.venue(),
                artifact.symbol(),
                artifact.currency()
            ),
            symbol: Some(artifact.symbol().to_string()),
            currency: Some(artifact.currency().to_string()),
            source_system: serde_json::to_string(&source.source)
                .unwrap_or_else(|_| "unserializable".to_string()),
            authority: source.authority.wire_id(),
            authoritative: artifact.is_authoritative(),
            covered_range: format!("{start} … {end} (UTC ns)"),
            as_of_ns: source.as_of_ns,
            record_count: artifact.source_records().len(),
            event_count: artifact.schedule().actions().len(),
            adjustment: Some(artifact.adjustment().wire_id()),
        }
    }
}

// ── Jobs and events ────────────────────────────────────────────────

/// Work the GUI can ask for. Every variant carries a caller-chosen
/// `request_id` so a reply can be matched to the request still wanted.
#[derive(Debug)]
pub enum ReferenceDataJob {
    /// Read and describe a persisted snapshot without promoting anything.
    InspectSnapshot { request_id: u64, path: PathBuf },
    /// Materialize a snapshot into the content-addressed store.
    MaterializeSnapshot { request_id: u64, path: PathBuf },
    /// Summarize the artifacts already in the store, up to `limit` (itself
    /// capped at [`MAX_LISTED_ARTIFACTS`]).
    ListArtifacts { request_id: u64, limit: usize },
    /// Verify the chosen artifacts and bind them into an execution config.
    SelectIntoConfig {
        request_id: u64,
        settings: Box<ExecutionSettings>,
        symbol: String,
        currency: String,
        calendar_artifact_id: String,
        corporate_action_artifact_id: String,
    },
}

impl ReferenceDataJob {
    pub const fn request_id(&self) -> u64 {
        match self {
            Self::InspectSnapshot { request_id, .. }
            | Self::MaterializeSnapshot { request_id, .. }
            | Self::ListArtifacts { request_id, .. }
            | Self::SelectIntoConfig { request_id, .. } => *request_id,
        }
    }
}

/// What the worker reports back. `Started` is advisory; every job produces
/// exactly one terminal event.
#[derive(Debug)]
pub enum ReferenceDataWorkerEvent {
    Started {
        request_id: u64,
        /// The thread the job actually ran on, so the off-render-thread contract
        /// is assertable rather than merely intended.
        worker_thread: std::thread::ThreadId,
    },
    SnapshotInspected {
        request_id: u64,
        summary: Box<ReferenceSourceSummary>,
    },
    Materialized {
        request_id: u64,
        summary: Box<ReferenceArtifactSummary>,
    },
    ArtifactsListed {
        request_id: u64,
        summaries: Vec<ReferenceArtifactSummary>,
        /// Ids present in the store beyond the listed prefix. Reported rather
        /// than silently dropped, so a truncated list never reads as complete.
        omitted: usize,
    },
    /// The chosen artifacts verified and bound into a sealed config.
    Selected {
        request_id: u64,
        config_id: String,
        settings: Box<ExecutionSettings>,
        calendar_artifact_id: String,
        corporate_action_artifact_id: String,
        /// Every corporate-action artifact the resulting config now binds, in
        /// sorted order. A multi-symbol config accumulates one per symbol, so
        /// this is the honest answer to "what reference data is this run on?" —
        /// the single id above is only the artifact this selection added.
        bound_corporate_action_artifact_ids: Vec<String>,
        /// False when either artifact is below exchange/vendor authority. The
        /// bind still happened and is honestly labelled; it is the caller's
        /// decision whether an unverified-public source may back a run.
        authoritative: bool,
    },
    Failed {
        request_id: u64,
        message: String,
    },
}

/// Why a submission was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceSubmitError {
    /// The bounded job queue is full — try again next frame.
    QueueFull,
    /// The worker thread is gone.
    WorkerStopped,
}

impl std::fmt::Display for ReferenceSubmitError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::QueueFull => formatter.write_str("reference-data worker queue is full"),
            Self::WorkerStopped => formatter.write_str("reference-data worker is not running"),
        }
    }
}

impl std::error::Error for ReferenceSubmitError {}

// ── Worker ─────────────────────────────────────────────────────────

/// A background thread owning one [`ReferenceArtifactStore`].
pub struct ReferenceDataWorker {
    jobs: SyncSender<ReferenceDataJob>,
    events: Receiver<ReferenceDataWorkerEvent>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl ReferenceDataWorker {
    /// Spawn a worker over the store rooted at `root`, creating it if needed.
    pub fn spawn_at(root: PathBuf) -> Result<Self, std::io::Error> {
        let (jobs, job_rx) = std::sync::mpsc::sync_channel(REFERENCE_JOB_QUEUE_CAPACITY);
        let (event_tx, events) = std::sync::mpsc::sync_channel(REFERENCE_EVENT_QUEUE_CAPACITY);
        let handle = std::thread::Builder::new()
            .name("typhoon-reference-data".to_string())
            .spawn(move || match ReferenceArtifactStore::open(&root) {
                Ok(store) => run(&store, &job_rx, &event_tx),
                Err(error) => {
                    // Fail every submitted job with the reason the store never
                    // opened, rather than dropping the channel and leaving the
                    // UI to guess at a silent disconnect.
                    while let Ok(job) = job_rx.recv() {
                        let failed = ReferenceDataWorkerEvent::Failed {
                            request_id: job.request_id(),
                            message: error.to_string(),
                        };
                        if !emit(&event_tx, failed) {
                            break;
                        }
                    }
                }
            })?;
        Ok(Self {
            jobs,
            events,
            handle: Some(handle),
        })
    }

    /// Non-blocking submit. Safe from a frame callback: it never waits.
    pub fn submit(&self, job: ReferenceDataJob) -> Result<(), ReferenceSubmitError> {
        match self.jobs.try_send(job) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => Err(ReferenceSubmitError::QueueFull),
            Err(TrySendError::Disconnected(_)) => Err(ReferenceSubmitError::WorkerStopped),
        }
    }

    /// Take up to [`MAX_REFERENCE_EVENTS_PER_POLL`] ready events. Never blocks.
    pub fn poll(&self) -> Vec<ReferenceDataWorkerEvent> {
        let mut batch = Vec::new();
        while batch.len() < MAX_REFERENCE_EVENTS_PER_POLL {
            match self.events.try_recv() {
                Ok(event) => batch.push(event),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
        batch
    }
}

impl Drop for ReferenceDataWorker {
    fn drop(&mut self) {
        // Closing the job channel ends the loop; joining keeps a half-written
        // artifact rename from racing process shutdown.
        let (dead, _) = std::sync::mpsc::sync_channel(1);
        drop(std::mem::replace(&mut self.jobs, dead));
        let Some(handle) = self.handle.take() else {
            return;
        };
        // A worker parked on a full event queue only moves when something
        // drains it, and this receiver is the only thing that can. Draining
        // while we wait is what stops the join below from waiting on a thread
        // that is waiting on us. The loop terminates because the job channel is
        // already closed, so the worker has a bounded number of events left.
        while !handle.is_finished() {
            if self.events.try_recv().is_err() {
                std::thread::sleep(EVENT_PARK);
            }
        }
        let _ = handle.join();
    }
}

impl std::fmt::Debug for ReferenceDataWorker {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ReferenceDataWorker")
    }
}

/// Push one event, parking rather than growing the queue. `false` means the
/// receiver is gone and the worker should stop.
fn emit(sender: &SyncSender<ReferenceDataWorkerEvent>, event: ReferenceDataWorkerEvent) -> bool {
    let mut pending = event;
    loop {
        match sender.try_send(pending) {
            Ok(()) => return true,
            Err(TrySendError::Full(returned)) => {
                pending = returned;
                std::thread::sleep(EVENT_PARK);
            }
            Err(TrySendError::Disconnected(_)) => return false,
        }
    }
}

fn run(
    store: &ReferenceArtifactStore,
    jobs: &Receiver<ReferenceDataJob>,
    events: &SyncSender<ReferenceDataWorkerEvent>,
) {
    while let Ok(job) = jobs.recv() {
        let request_id = job.request_id();
        let started = ReferenceDataWorkerEvent::Started {
            request_id,
            worker_thread: std::thread::current().id(),
        };
        if !emit(events, started) {
            return;
        }
        let terminal = match execute(store, job) {
            Ok(event) => event,
            Err(error) => ReferenceDataWorkerEvent::Failed {
                request_id,
                message: error.to_string(),
            },
        };
        if !emit(events, terminal) {
            return;
        }
    }
}

fn execute(
    store: &ReferenceArtifactStore,
    job: ReferenceDataJob,
) -> Result<ReferenceDataWorkerEvent, ReferenceDataError> {
    match job {
        ReferenceDataJob::InspectSnapshot { request_id, path } => {
            let label = path.display().to_string();
            let snapshot = load_snapshot(&path)?;
            // A dry run is the only honest answer to "can this be promoted?":
            // it exercises the same refusals promotion would.
            let blocked = match &snapshot {
                ReferenceSourceSnapshot::Calendar(request) => {
                    materialize_calendar(request).err().map(|e| e.to_string())
                }
                ReferenceSourceSnapshot::CorporateActions(request) => {
                    materialize_corporate_actions(request)
                        .err()
                        .map(|e| e.to_string())
                }
            };
            Ok(ReferenceDataWorkerEvent::SnapshotInspected {
                request_id,
                summary: Box::new(ReferenceSourceSummary::describe(&label, &snapshot, blocked)),
            })
        }
        ReferenceDataJob::MaterializeSnapshot { request_id, path } => {
            let snapshot = load_snapshot(&path)?;
            Ok(ReferenceDataWorkerEvent::Materialized {
                request_id,
                summary: Box::new(snapshot.materialize_into(store)?),
            })
        }
        ReferenceDataJob::ListArtifacts { request_id, limit } => {
            // The caller's limit is honoured but never trusted above the
            // worker's own bound, so one request can't ask for the whole store.
            let limit = limit.min(MAX_LISTED_ARTIFACTS);
            let mut summaries = Vec::new();
            let mut omitted = 0;
            for id in store.list_ids(ReferenceArtifactKind::Calendar)? {
                if summaries.len() == limit {
                    omitted += 1;
                    continue;
                }
                summaries.push(ReferenceArtifactSummary::calendar(
                    &store.load_calendar(&id)?,
                ));
            }
            for id in store.list_ids(ReferenceArtifactKind::CorporateActions)? {
                if summaries.len() == limit {
                    omitted += 1;
                    continue;
                }
                summaries.push(ReferenceArtifactSummary::corporate_actions(
                    &store.load_corporate_actions(&id)?,
                ));
            }
            Ok(ReferenceDataWorkerEvent::ArtifactsListed {
                request_id,
                summaries,
                omitted,
            })
        }
        ReferenceDataJob::SelectIntoConfig {
            request_id,
            settings,
            symbol,
            currency,
            calendar_artifact_id,
            corporate_action_artifact_id,
        } => {
            // Loading through the store re-verifies both artifacts against
            // their ids, so a hand-edited file cannot be selected into a run.
            let calendar = store.load_calendar(&calendar_artifact_id)?;
            let actions = store.load_corporate_actions(&corporate_action_artifact_id)?;
            let authoritative = calendar.is_authoritative() && actions.is_authoritative();
            let bound =
                bind_reference_artifacts(&settings, &symbol, &currency, &calendar, &actions)?;
            let config = StrategyExecutionConfig::build(&bound)
                .map_err(|error| ReferenceDataError::Config(error.to_string()))?;
            let bound_corporate_action_artifact_ids =
                bound.reference_data.corporate_action_artifact_ids.clone();
            Ok(ReferenceDataWorkerEvent::Selected {
                request_id,
                config_id: config.config_id().to_string(),
                settings: Box::new(bound),
                calendar_artifact_id,
                corporate_action_artifact_id,
                bound_corporate_action_artifact_ids,
                authoritative,
            })
        }
    }
}

/// Read and strictly decode a persisted snapshot.
///
/// The size ceiling is applied from the file metadata first, so an oversized or
/// wrong file is refused without ever being read into memory.
fn load_snapshot(path: &std::path::Path) -> Result<ReferenceSourceSnapshot, ReferenceDataError> {
    let metadata =
        std::fs::metadata(path).map_err(|error| ReferenceDataError::Io(error.to_string()))?;
    if metadata.len() > MAX_SNAPSHOT_BYTES as u64 {
        return Err(ReferenceDataError::ArtifactTooLarge);
    }
    let bytes = std::fs::read(path).map_err(|error| ReferenceDataError::Io(error.to_string()))?;
    if bytes.len() > MAX_SNAPSHOT_BYTES {
        return Err(ReferenceDataError::ArtifactTooLarge);
    }
    serde_json::from_slice(&bytes).map_err(|error| ReferenceDataError::Decode(error.to_string()))
}

#[cfg(test)]
mod tests;
