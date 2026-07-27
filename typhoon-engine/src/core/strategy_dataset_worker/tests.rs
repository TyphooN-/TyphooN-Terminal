use super::*;
use crate::core::strategy_dataset::{
    AdjustmentPolicy, CalendarPolicy, DatasetProvenance, DatasetQaPolicy,
};
use crate::core::strategy_dataset_store::MAX_PAGE_BARS;

// ── Helpers ────────────────────────────────────────────────────────

fn input(symbol: &str) -> DatasetManifestInput {
    DatasetManifestInput {
        symbol: symbol.to_string(),
        timeframe: "1Day".to_string(),
        provenance: DatasetProvenance {
            source: "kraken".to_string(),
            venue: "kraken-spot".to_string(),
            pipeline: "cache-merge/v1".to_string(),
        },
        adjustment: AdjustmentPolicy::Raw,
        calendar: CalendarPolicy::Continuous24x7,
        qa_policy: DatasetQaPolicy::default(),
    }
}

fn bars(count: usize) -> Vec<Bar> {
    let start = chrono::DateTime::from_timestamp(1_704_067_200, 0).expect("epoch");
    let mut close = 100.0_f64;
    (0..count)
        .map(|index| {
            let open = close;
            close = open * 1.005;
            Bar {
                timestamp: (start + chrono::Duration::days(index as i64))
                    .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                open,
                high: close * 1.005,
                low: open * 0.995,
                close,
                volume: 1_000.0,
            }
        })
        .collect()
}

/// Drain events until one satisfies `wanted`, or the budget runs out.
///
/// The budget is generous and the sleep short: this is a test-side wait, not a
/// timing assertion. A worker that never answers fails the test rather than
/// hanging the suite.
fn wait_for(
    worker: &DatasetWorker,
    seen: &mut Vec<DatasetWorkerEvent>,
    wanted: impl Fn(&DatasetWorkerEvent) -> bool,
) -> DatasetWorkerEvent {
    for _ in 0..4_000 {
        for event in worker.poll() {
            if wanted(&event) {
                return event;
            }
            seen.push(event);
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    panic!("worker produced no matching event; saw {seen:?}");
}

fn is_terminal(event: &DatasetWorkerEvent) -> bool {
    !matches!(event, DatasetWorkerEvent::Started { .. })
}

// ── Off-thread execution ───────────────────────────────────────────

#[test]
fn deferred_store_open_and_dataset_build_run_on_the_worker_thread() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().join("not-created-by-render-thread");
    assert!(!root.exists());
    let worker = DatasetWorker::spawn_at(root.clone()).expect("worker spawns");
    let mut seen = Vec::new();

    worker
        .submit(DatasetJob::Build {
            request_id: 7,
            input: input("BTC/USD"),
            bars: bars(8),
        })
        .expect("submit");

    let built = wait_for(&worker, &mut seen, |event| {
        matches!(event, DatasetWorkerEvent::Built { request_id: 7, .. })
    });
    assert!(matches!(built, DatasetWorkerEvent::Built { .. }));
    assert!(
        root.exists(),
        "the worker should initialize its store lazily"
    );
    worker.shutdown();
}

#[test]
fn dataset_work_runs_off_the_submitting_thread() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = FileDatasetStore::open(temp.path()).expect("store");
    let worker = DatasetWorker::spawn(store.clone()).expect("worker spawns");
    let mut seen = Vec::new();

    worker
        .submit(DatasetJob::Build {
            request_id: 1,
            input: input("BTC/USD"),
            bars: bars(64),
        })
        .expect("submit");

    let started = wait_for(&worker, &mut seen, |event| {
        matches!(event, DatasetWorkerEvent::Started { .. })
    });
    match started {
        DatasetWorkerEvent::Started {
            request_id,
            worker_thread,
        } => {
            assert_eq!(request_id, 1);
            assert_ne!(
                worker_thread,
                std::thread::current().id(),
                "dataset work must not run on the submitting (render) thread"
            );
        }
        other => panic!("expected Started, got {other:?}"),
    }

    let built = wait_for(&worker, &mut seen, is_terminal);
    match built {
        DatasetWorkerEvent::Built {
            request_id,
            summary,
            outcome,
        } => {
            assert_eq!(request_id, 1);
            assert_eq!(outcome, DatasetPutOutcome::Stored);
            assert_eq!(summary.symbol, "BTC/USD");
            assert_eq!(summary.bar_count, 64);
            assert!(store.contains(&summary.dataset_id).expect("contains"));
        }
        other => panic!("expected Built, got {other:?}"),
    }
    worker.shutdown();
}

#[test]
fn submitting_and_polling_never_block_the_caller() {
    let temp = tempfile::tempdir().expect("tempdir");
    let worker = DatasetWorker::spawn(FileDatasetStore::open(temp.path()).expect("store"))
        .expect("worker spawns");

    // An idle worker yields nothing and returns immediately.
    assert!(worker.poll().is_empty());

    worker
        .submit(DatasetJob::List {
            request_id: 7,
            limit: 8,
        })
        .expect("submit");

    let mut seen = Vec::new();
    match wait_for(&worker, &mut seen, is_terminal) {
        DatasetWorkerEvent::Listed {
            request_id,
            records,
        } => {
            assert_eq!(request_id, 7);
            assert!(records.is_empty());
        }
        other => panic!("expected Listed, got {other:?}"),
    }
    worker.shutdown();
}

// ── Bounded queues and backpressure ────────────────────────────────

#[test]
fn the_job_queue_is_bounded_and_reports_backpressure() {
    // No worker consumes this queue, so capacity is reached deterministically.
    let (queue, receiver) = job_channel();
    for request_id in 0..DATASET_JOB_QUEUE_CAPACITY as u64 {
        queue
            .try_submit(DatasetJob::List {
                request_id,
                limit: 1,
            })
            .expect("within capacity");
    }
    assert!(matches!(
        queue.try_submit(DatasetJob::List {
            request_id: 999,
            limit: 1
        }),
        Err(DatasetSubmitError::QueueFull)
    ));

    // Draining one slot makes room for exactly one more.
    receiver.recv().expect("drain one");
    queue
        .try_submit(DatasetJob::List {
            request_id: 1_000,
            limit: 1,
        })
        .expect("one slot freed");
    assert!(matches!(
        queue.try_submit(DatasetJob::List {
            request_id: 1_001,
            limit: 1
        }),
        Err(DatasetSubmitError::QueueFull)
    ));

    drop(receiver);
    assert!(matches!(
        queue.try_submit(DatasetJob::List {
            request_id: 1_002,
            limit: 1
        }),
        Err(DatasetSubmitError::WorkerStopped)
    ));
}

#[test]
fn each_poll_drains_at_most_one_bounded_batch() {
    let (sender, receiver) = std::sync::mpsc::sync_channel(DATASET_EVENT_QUEUE_CAPACITY);
    for request_id in 0..DATASET_EVENT_QUEUE_CAPACITY as u64 {
        sender
            .try_send(DatasetWorkerEvent::Cancelled { request_id })
            .expect("fill the event queue");
    }
    assert!(DATASET_EVENT_QUEUE_CAPACITY > MAX_EVENTS_PER_POLL);

    let batch = drain_events(&receiver, MAX_EVENTS_PER_POLL);
    assert_eq!(batch.len(), MAX_EVENTS_PER_POLL);

    let remaining = drain_events(&receiver, DATASET_EVENT_QUEUE_CAPACITY);
    assert_eq!(
        remaining.len(),
        DATASET_EVENT_QUEUE_CAPACITY - MAX_EVENTS_PER_POLL
    );
    assert!(drain_events(&receiver, MAX_EVENTS_PER_POLL).is_empty());
}

// ── Cancellation ───────────────────────────────────────────────────

#[test]
fn a_cancelled_request_is_never_executed() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = FileDatasetStore::open(temp.path()).expect("store");
    let worker = DatasetWorker::spawn(store.clone()).expect("worker spawns");

    // Cancelling before submitting makes the outcome deterministic: the worker
    // sees the request already cancelled when it dequeues it.
    worker.cancel(42);
    worker
        .submit(DatasetJob::Build {
            request_id: 42,
            input: input("BTC/USD"),
            bars: bars(32),
        })
        .expect("submit");

    let mut seen = Vec::new();
    match wait_for(&worker, &mut seen, is_terminal) {
        DatasetWorkerEvent::Cancelled { request_id } => assert_eq!(request_id, 42),
        other => panic!("expected Cancelled, got {other:?}"),
    }
    assert!(store.list(8).expect("list").is_empty());

    // A cancellation is consumed by the request it names and does not leak
    // onto the next one.
    worker
        .submit(DatasetJob::Build {
            request_id: 43,
            input: input("BTC/USD"),
            bars: bars(32),
        })
        .expect("submit");
    match wait_for(&worker, &mut seen, is_terminal) {
        DatasetWorkerEvent::Built { request_id, .. } => assert_eq!(request_id, 43),
        other => panic!("expected Built, got {other:?}"),
    }
    worker.shutdown();
}

#[test]
fn the_cancellation_set_cannot_grow_without_bound() {
    let temp = tempfile::tempdir().expect("tempdir");
    let worker = DatasetWorker::spawn(FileDatasetStore::open(temp.path()).expect("store"))
        .expect("worker spawns");

    for request_id in 0..(MAX_TRACKED_CANCELLATIONS as u64 * 8) {
        worker.cancel(request_id);
        assert!(worker.tracked_cancellations() <= MAX_TRACKED_CANCELLATIONS);
    }
    assert_eq!(worker.tracked_cancellations(), MAX_TRACKED_CANCELLATIONS);
    worker.shutdown();
}

// ── Error delivery ─────────────────────────────────────────────────

#[test]
fn failures_arrive_as_events_and_leave_the_worker_running() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = FileDatasetStore::open(temp.path()).expect("store");
    let worker = DatasetWorker::spawn(store.clone()).expect("worker spawns");
    let mut seen = Vec::new();

    worker
        .submit(DatasetJob::ReadPage {
            request_id: 1,
            dataset_id: "../../etc/passwd".to_string(),
            offset: 0,
            limit: 10,
        })
        .expect("submit");
    match wait_for(&worker, &mut seen, is_terminal) {
        DatasetWorkerEvent::Failed {
            request_id,
            message,
        } => {
            assert_eq!(request_id, 1);
            assert!(!message.is_empty());
        }
        other => panic!("expected Failed, got {other:?}"),
    }

    worker
        .submit(DatasetJob::ReadPage {
            request_id: 2,
            dataset_id: "0".repeat(64),
            offset: 0,
            limit: 10,
        })
        .expect("submit");
    match wait_for(&worker, &mut seen, is_terminal) {
        DatasetWorkerEvent::Failed { request_id, .. } => assert_eq!(request_id, 2),
        other => panic!("expected Failed, got {other:?}"),
    }

    // Still healthy after two failures.
    worker
        .submit(DatasetJob::Build {
            request_id: 3,
            input: input("ETH/USD"),
            bars: bars(16),
        })
        .expect("submit");
    match wait_for(&worker, &mut seen, is_terminal) {
        DatasetWorkerEvent::Built { request_id, .. } => assert_eq!(request_id, 3),
        other => panic!("expected Built, got {other:?}"),
    }
    worker.shutdown();
}

// ── Paged reads through the worker ─────────────────────────────────

#[test]
fn page_jobs_return_bounded_windows_with_their_manifest() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = FileDatasetStore::open(temp.path()).expect("store");
    let worker = DatasetWorker::spawn(store.clone()).expect("worker spawns");
    let mut seen = Vec::new();

    let source = bars(300);
    worker
        .submit(DatasetJob::Build {
            request_id: 1,
            input: input("BTC/USD"),
            bars: source.clone(),
        })
        .expect("submit");
    let dataset_id = match wait_for(&worker, &mut seen, is_terminal) {
        DatasetWorkerEvent::Built { summary, .. } => summary.dataset_id,
        other => panic!("expected Built, got {other:?}"),
    };

    for (request_id, offset) in [(2u64, 0u64), (3, 100), (4, 250)] {
        worker
            .submit(DatasetJob::ReadPage {
                request_id,
                dataset_id: dataset_id.clone(),
                offset,
                limit: 100,
            })
            .expect("submit");
        match wait_for(&worker, &mut seen, is_terminal) {
            DatasetWorkerEvent::Page { summary, page, .. } => {
                assert_eq!(summary.dataset_id, dataset_id);
                assert_eq!(page.offset, offset);
                assert_eq!(page.total_bars, 300);
                assert!(page.bars.len() <= 100);
                assert_eq!(page.bars.len(), (300 - offset).min(100) as usize);
                assert_eq!(page.bars[0].timestamp, source[offset as usize].timestamp);
            }
            other => panic!("expected Page, got {other:?}"),
        }
    }

    // An over-large page is refused, not silently clamped.
    worker
        .submit(DatasetJob::ReadPage {
            request_id: 5,
            dataset_id: dataset_id.clone(),
            offset: 0,
            limit: MAX_PAGE_BARS + 1,
        })
        .expect("submit");
    match wait_for(&worker, &mut seen, is_terminal) {
        DatasetWorkerEvent::Failed { request_id, .. } => assert_eq!(request_id, 5),
        other => panic!("expected Failed, got {other:?}"),
    }
    worker.shutdown();
}

#[test]
fn the_worker_caches_at_most_one_open_record() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = FileDatasetStore::open(temp.path()).expect("store");
    let worker = DatasetWorker::spawn(store.clone()).expect("worker spawns");
    let mut seen = Vec::new();
    let mut ids = Vec::new();

    for (request_id, symbol) in [(1u64, "BTC/USD"), (2, "ETH/USD")] {
        worker
            .submit(DatasetJob::Build {
                request_id,
                input: input(symbol),
                bars: bars(40),
            })
            .expect("submit");
        match wait_for(&worker, &mut seen, is_terminal) {
            DatasetWorkerEvent::Built { summary, .. } => ids.push(summary.dataset_id),
            other => panic!("expected Built, got {other:?}"),
        }
    }

    // Alternating between two datasets must keep working — a one-slot cache
    // that mixed up its key would hand back the wrong bars.
    for (request_id, index) in [(10u64, 0usize), (11, 1), (12, 0), (13, 1)] {
        worker
            .submit(DatasetJob::ReadPage {
                request_id,
                dataset_id: ids[index].clone(),
                offset: 0,
                limit: 10,
            })
            .expect("submit");
        match wait_for(&worker, &mut seen, is_terminal) {
            DatasetWorkerEvent::Page { summary, page, .. } => {
                assert_eq!(summary.dataset_id, ids[index]);
                assert_eq!(page.bars.len(), 10);
            }
            other => panic!("expected Page, got {other:?}"),
        }
    }
    assert!(worker.cached_records() <= MAX_CACHED_RECORDS);
    worker.shutdown();
}
