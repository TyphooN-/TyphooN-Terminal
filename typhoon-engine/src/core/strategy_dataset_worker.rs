//! Off-render-thread dataset construction, QA, and paged reads — the ADR-135
//! M0 background-work boundary.
//!
//! Everything expensive about a dataset — hashing the bars, running QA,
//! `fsync`ing three files, streaming a payload digest — happens on one worker
//! thread. The GUI submits a job and drains events; it never waits
//! ([ADR-098](../../../../docs/adr/098-per-frame-o-1-discipline-in-chart-and-sync-paths.md),
//! [ADR-134](../../../../docs/adr/134-render-independent-background-pump.md)).
//!
//! ## The four bounds
//!
//! 1. **Job queue** — [`DATASET_JOB_QUEUE_CAPACITY`] slots. A full queue is
//!    reported as [`DatasetSubmitError::QueueFull`], never waited on: that is
//!    the backpressure signal, and a caller on the render thread must be able
//!    to see it and move on.
//! 2. **Event queue** — [`DATASET_EVENT_QUEUE_CAPACITY`] slots. The worker
//!    parks rather than growing it, so a UI that stops draining slows the
//!    worker instead of consuming memory. Dropping the worker unblocks it.
//! 3. **Poll batch** — [`MAX_EVENTS_PER_POLL`] events per call, so one frame's
//!    drain cost has a ceiling regardless of how far behind the UI has fallen.
//! 4. **Retained state** — [`MAX_TRACKED_CANCELLATIONS`] request ids and
//!    [`MAX_CACHED_RECORDS`] open records. Both evict; neither grows.
//!
//! ## Cancellation granularity, stated honestly
//!
//! Cancellation is checked when a job is dequeued and again after the `Started`
//! event. A stage already running — one `DatasetManifest::build`, one QA pass,
//! one `put` — runs to completion, because those calls are bounded and adding
//! an abort flag through the hashing and QA loops would buy little and cost
//! determinism review. Cancelling therefore means "do not start", not "abort
//! mid-hash".

use crate::broker::alpaca::Bar;
use crate::core::strategy_dataset::{DatasetManifestInput, DatasetQaReport};
use crate::core::strategy_dataset_store::{
    DatasetPage, DatasetPutOutcome, DatasetRecord, DatasetRecordSummary, DatasetStoreError,
    FileDatasetStore,
};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError, TrySendError};
use std::sync::{Arc, Mutex};

/// In-flight jobs the worker will accept before reporting backpressure.
pub const DATASET_JOB_QUEUE_CAPACITY: usize = 4;

/// Events the worker may have outstanding before it parks.
pub const DATASET_EVENT_QUEUE_CAPACITY: usize = 64;

/// Events one [`DatasetWorker::poll`] may return.
pub const MAX_EVENTS_PER_POLL: usize = 16;

/// Cancelled request ids retained. Oldest is evicted first — a cancellation
/// that was never matched to a job is stale by definition.
pub const MAX_TRACKED_CANCELLATIONS: usize = 64;

/// Opened records the worker keeps warm so consecutive pages of one dataset do
/// not re-verify the payload digest every time.
pub const MAX_CACHED_RECORDS: usize = 2;

/// How long the worker parks when the event queue is full.
const EVENT_PARK: std::time::Duration = std::time::Duration::from_millis(1);

// ── Jobs and events ────────────────────────────────────────────────

/// Work the GUI can ask for. Every variant carries a caller-chosen
/// `request_id` so a reply can be matched to the request that is still wanted.
#[derive(Debug)]
pub enum DatasetJob {
    /// Content-address, QA, and publish a dataset.
    Build {
        request_id: u64,
        input: DatasetManifestInput,
        bars: Vec<Bar>,
    },
    /// Summaries of stored datasets, up to `limit`.
    List { request_id: u64, limit: usize },
    /// A bounded window of one dataset's bars, with its QA findings.
    ReadPage {
        request_id: u64,
        dataset_id: String,
        offset: u64,
        limit: usize,
    },
}

impl DatasetJob {
    pub fn request_id(&self) -> u64 {
        match self {
            Self::Build { request_id, .. }
            | Self::List { request_id, .. }
            | Self::ReadPage { request_id, .. } => *request_id,
        }
    }
}

/// What the worker reports back. `Started` is advisory; every job produces
/// exactly one terminal event (`Built`, `Listed`, `Page`, `Failed`, or
/// `Cancelled`).
#[derive(Debug)]
pub enum DatasetWorkerEvent {
    Started {
        request_id: u64,
        /// The thread the job actually ran on. Carried so the off-render-thread
        /// contract is assertable rather than merely intended.
        worker_thread: std::thread::ThreadId,
    },
    Built {
        request_id: u64,
        summary: DatasetRecordSummary,
        outcome: DatasetPutOutcome,
    },
    Listed {
        request_id: u64,
        records: Vec<DatasetRecordSummary>,
    },
    Page {
        request_id: u64,
        summary: DatasetRecordSummary,
        /// Boxed: a page is far larger than the other variants, and an enum
        /// sized by its biggest member would make every queued event costly.
        page: Box<DatasetPage>,
        /// Report-level QA context for the window's header.
        qa_summary: DatasetQaSummary,
    },
    Failed {
        request_id: u64,
        message: String,
    },
    Cancelled {
        request_id: u64,
    },
}

/// The report-level QA facts the inspector header shows, extracted so the UI
/// never holds a whole `DatasetQaReport`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetQaSummary {
    pub error_count: u64,
    pub warning_count: u64,
    pub info_count: u64,
    pub findings_truncated: bool,
    pub findings_omitted: u64,
    pub gap_detection: String,
    pub spike_detection: String,
}

impl DatasetQaSummary {
    pub fn from_report(report: &DatasetQaReport) -> Self {
        use crate::core::strategy_dataset::{GapDetectionStatus, SpikeDetectionStatus};
        let gap_detection = match &report.gap_detection {
            GapDetectionStatus::Enabled { step_seconds } => {
                format!("enabled ({step_seconds}s step)")
            }
            GapDetectionStatus::UnsupportedTimeframe { timeframe } => {
                format!("declined — unsupported timeframe `{timeframe}`")
            }
            GapDetectionStatus::VariableLengthTimeframe { timeframe } => {
                format!("declined — `{timeframe}` has no fixed step")
            }
            GapDetectionStatus::UnsupportedForCalendar {
                timeframe,
                calendar_policy_id,
            } => {
                format!("declined — `{timeframe}` needs session hours `{calendar_policy_id}` lacks")
            }
        };
        let spike_detection = match &report.spike_detection {
            SpikeDetectionStatus::Enabled { band, samples } => {
                format!("band {band:.4} from {samples} moves")
            }
            SpikeDetectionStatus::InsufficientSamples { samples, required } => {
                format!("declined — {samples} of {required} moves")
            }
            SpikeDetectionStatus::Unavailable { reason } => format!("declined — {reason}"),
        };
        Self {
            error_count: report.error_count() as u64,
            warning_count: report.warning_count() as u64,
            info_count: report.info_count() as u64,
            findings_truncated: report.findings_truncated,
            findings_omitted: report.findings_omitted,
            gap_detection,
            spike_detection,
        }
    }
}

/// Why a submission was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatasetSubmitError {
    /// The bounded job queue is full — try again next frame.
    QueueFull,
    /// The worker thread is gone.
    WorkerStopped,
}

impl std::fmt::Display for DatasetSubmitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::QueueFull => write!(f, "dataset worker queue is full"),
            Self::WorkerStopped => write!(f, "dataset worker is not running"),
        }
    }
}

impl std::error::Error for DatasetSubmitError {}

// ── Queue ──────────────────────────────────────────────────────────

/// The submitting half of the bounded job queue.
#[derive(Debug, Clone)]
struct DatasetJobQueue {
    sender: SyncSender<DatasetJob>,
}

impl DatasetJobQueue {
    /// Non-blocking submit. Never waits, so it is safe from a frame callback.
    fn try_submit(&self, job: DatasetJob) -> Result<(), DatasetSubmitError> {
        match self.sender.try_send(job) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => Err(DatasetSubmitError::QueueFull),
            Err(TrySendError::Disconnected(_)) => Err(DatasetSubmitError::WorkerStopped),
        }
    }
}

fn job_channel() -> (DatasetJobQueue, Receiver<DatasetJob>) {
    let (sender, receiver) = std::sync::mpsc::sync_channel(DATASET_JOB_QUEUE_CAPACITY);
    (DatasetJobQueue { sender }, receiver)
}

/// Take up to `limit` ready events without ever blocking.
fn drain_events(receiver: &Receiver<DatasetWorkerEvent>, limit: usize) -> Vec<DatasetWorkerEvent> {
    let mut batch = Vec::new();
    while batch.len() < limit {
        match receiver.try_recv() {
            Ok(event) => batch.push(event),
            Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => break,
        }
    }
    batch
}

/// Bounded set of cancelled request ids.
#[derive(Debug, Default)]
struct CancellationSet {
    ids: BTreeSet<u64>,
}

impl CancellationSet {
    fn insert(&mut self, request_id: u64) {
        self.ids.insert(request_id);
        while self.ids.len() > MAX_TRACKED_CANCELLATIONS {
            // Evict the lowest id: request ids are issued in increasing order,
            // so the lowest outstanding cancellation is the stalest.
            let Some(oldest) = self.ids.iter().next().copied() else {
                break;
            };
            self.ids.remove(&oldest);
        }
    }

    /// Consume a cancellation, if this request was cancelled.
    fn take(&mut self, request_id: u64) -> bool {
        self.ids.remove(&request_id)
    }
}

// ── Worker ─────────────────────────────────────────────────────────

/// A handle to the dataset worker thread.
#[derive(Debug)]
pub struct DatasetWorker {
    queue: DatasetJobQueue,
    events: Receiver<DatasetWorkerEvent>,
    cancellations: Arc<Mutex<CancellationSet>>,
    cached: Arc<std::sync::atomic::AtomicUsize>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl DatasetWorker {
    /// Start a worker that opens its store on the worker thread.
    ///
    /// This is the native-UI constructor: creating directories and validating
    /// the storage root must not run in an egui frame callback. If opening the
    /// store fails, submitted jobs receive bounded `Failed` events.
    pub fn spawn_at(root: PathBuf) -> Result<Self, std::io::Error> {
        let (queue, jobs) = job_channel();
        let (event_tx, events) = std::sync::mpsc::sync_channel(DATASET_EVENT_QUEUE_CAPACITY);
        let cancellations = Arc::new(Mutex::new(CancellationSet::default()));
        let cached = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let worker_cancellations = Arc::clone(&cancellations);
        let worker_cached = Arc::clone(&cached);
        let handle = std::thread::Builder::new()
            .name("typhoon-dataset-worker".to_string())
            .spawn(move || match FileDatasetStore::open(root) {
                Ok(store) => run_worker(store, jobs, event_tx, worker_cancellations, worker_cached),
                Err(error) => run_store_open_failure(jobs, event_tx, error.to_string()),
            })?;

        Ok(Self {
            queue,
            events,
            cancellations,
            cached,
            handle: Some(handle),
        })
    }

    /// Start a worker over `store`.
    ///
    /// Fallible on purpose: the caller is a GUI window, and a machine that has
    /// run out of threads should get a status line, not a panicked frame.
    pub fn spawn(store: FileDatasetStore) -> Result<Self, std::io::Error> {
        let (queue, jobs) = job_channel();
        let (event_tx, events) = std::sync::mpsc::sync_channel(DATASET_EVENT_QUEUE_CAPACITY);
        let cancellations = Arc::new(Mutex::new(CancellationSet::default()));
        let cached = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let worker_cancellations = Arc::clone(&cancellations);
        let worker_cached = Arc::clone(&cached);
        let handle = std::thread::Builder::new()
            .name("typhoon-dataset-worker".to_string())
            .spawn(move || {
                run_worker(store, jobs, event_tx, worker_cancellations, worker_cached)
            })?;

        Ok(Self {
            queue,
            events,
            cancellations,
            cached,
            handle: Some(handle),
        })
    }

    /// Enqueue a job. Returns immediately, including when the queue is full.
    pub fn submit(&self, job: DatasetJob) -> Result<(), DatasetSubmitError> {
        self.queue.try_submit(job)
    }

    /// Take up to [`MAX_EVENTS_PER_POLL`] ready events. Never blocks.
    pub fn poll(&self) -> Vec<DatasetWorkerEvent> {
        drain_events(&self.events, MAX_EVENTS_PER_POLL)
    }

    /// Ask that `request_id` not be started. A job already running finishes.
    pub fn cancel(&self, request_id: u64) {
        if let Ok(mut set) = self.cancellations.lock() {
            set.insert(request_id);
        }
    }

    /// How many cancellations are currently retained (bounded by
    /// [`MAX_TRACKED_CANCELLATIONS`]).
    pub fn tracked_cancellations(&self) -> usize {
        self.cancellations
            .lock()
            .map(|set| set.ids.len())
            .unwrap_or(0)
    }

    /// How many records the worker is holding open (bounded by
    /// [`MAX_CACHED_RECORDS`]).
    pub fn cached_records(&self) -> usize {
        self.cached.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Stop the worker and wait for it. Dropping the handle instead simply
    /// detaches — the thread still exits once the job queue closes.
    pub fn shutdown(mut self) {
        let handle = self.handle.take();
        drop(self);
        if let Some(handle) = handle {
            let _ = handle.join();
        }
    }
}

fn run_store_open_failure(
    jobs: Receiver<DatasetJob>,
    events: SyncSender<DatasetWorkerEvent>,
    message: String,
) {
    let worker_thread = std::thread::current().id();
    while let Ok(job) = jobs.recv() {
        let request_id = job.request_id();
        if !emit(
            &events,
            DatasetWorkerEvent::Started {
                request_id,
                worker_thread,
            },
        ) || !emit(
            &events,
            DatasetWorkerEvent::Failed {
                request_id,
                message: message.clone(),
            },
        ) {
            return;
        }
    }
}

/// Send an event, parking while the bounded queue is full.
///
/// Returns `false` once the receiver is gone, which is how the worker learns
/// that the GUI has shut down while it was parked.
fn emit(sender: &SyncSender<DatasetWorkerEvent>, event: DatasetWorkerEvent) -> bool {
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

/// A one-or-two entry warm cache of opened records, so paging through a
/// dataset does not re-stream its digest per page.
struct RecordCache {
    entries: Vec<DatasetRecord>,
    size: Arc<std::sync::atomic::AtomicUsize>,
}

impl RecordCache {
    fn new(size: Arc<std::sync::atomic::AtomicUsize>) -> Self {
        Self {
            entries: Vec::new(),
            size,
        }
    }

    fn get_or_open(
        &mut self,
        store: &FileDatasetStore,
        dataset_id: &str,
    ) -> Result<&DatasetRecord, DatasetStoreError> {
        if let Some(position) = self
            .entries
            .iter()
            .position(|record| record.manifest().dataset_id == dataset_id)
        {
            // Most-recently-used last.
            let record = self.entries.remove(position);
            self.entries.push(record);
        } else {
            let record = store.open_record(dataset_id)?;
            if self.entries.len() >= MAX_CACHED_RECORDS {
                self.entries.remove(0);
            }
            self.entries.push(record);
        }
        self.size
            .store(self.entries.len(), std::sync::atomic::Ordering::Relaxed);
        Ok(self.entries.last().expect("just inserted"))
    }
}

fn run_worker(
    store: FileDatasetStore,
    jobs: Receiver<DatasetJob>,
    events: SyncSender<DatasetWorkerEvent>,
    cancellations: Arc<Mutex<CancellationSet>>,
    cached: Arc<std::sync::atomic::AtomicUsize>,
) {
    let mut cache = RecordCache::new(cached);
    let thread_id = std::thread::current().id();

    let cancelled = |request_id: u64| -> bool {
        cancellations
            .lock()
            .map(|mut set| set.take(request_id))
            .unwrap_or(false)
    };

    while let Ok(job) = jobs.recv() {
        let request_id = job.request_id();
        if cancelled(request_id) {
            if !emit(&events, DatasetWorkerEvent::Cancelled { request_id }) {
                return;
            }
            continue;
        }
        if !emit(
            &events,
            DatasetWorkerEvent::Started {
                request_id,
                worker_thread: thread_id,
            },
        ) {
            return;
        }
        // Last chance to bail: a cancellation that arrived while the Started
        // event was parked still stops the work.
        if cancelled(request_id) {
            if !emit(&events, DatasetWorkerEvent::Cancelled { request_id }) {
                return;
            }
            continue;
        }

        let outcome = execute(&store, &mut cache, job);
        let event = match outcome {
            Ok(event) => event,
            Err(error) => DatasetWorkerEvent::Failed {
                request_id,
                message: error.to_string(),
            },
        };
        if !emit(&events, event) {
            return;
        }
    }
}

fn execute(
    store: &FileDatasetStore,
    cache: &mut RecordCache,
    job: DatasetJob,
) -> Result<DatasetWorkerEvent, DatasetStoreError> {
    match job {
        DatasetJob::Build {
            request_id,
            input,
            bars,
        } => {
            let stored = store.build_and_put(&input, &bars)?;
            Ok(DatasetWorkerEvent::Built {
                request_id,
                summary: DatasetRecordSummary::from_manifest(&stored.manifest),
                outcome: stored.outcome,
            })
        }
        DatasetJob::List { request_id, limit } => Ok(DatasetWorkerEvent::Listed {
            request_id,
            records: store.list(limit)?,
        }),
        DatasetJob::ReadPage {
            request_id,
            dataset_id,
            offset,
            limit,
        } => {
            let record = cache.get_or_open(store, &dataset_id)?;
            let page = record.read_page(offset, limit)?;
            Ok(DatasetWorkerEvent::Page {
                request_id,
                summary: record.summary(),
                qa_summary: DatasetQaSummary::from_report(record.qa()),
                page: Box::new(page),
            })
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
