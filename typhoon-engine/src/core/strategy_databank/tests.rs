use super::*;
use crate::core::strategy_builder::NnfxBuilderConfig;
use crate::core::strategy_ir::StrategyIr;
use crate::core::strategy_metrics::{MetricResult, MetricValue};
use std::time::{Duration, Instant};

fn metric(id: &str, value: f64) -> MetricResult {
    MetricResult {
        id: id.to_string(),
        value: MetricValue::Defined { value },
    }
}

fn run_input(run_id: String, strategy_id: String, sequence: i64) -> DatabankRunInput {
    DatabankRunInput {
        run_id,
        strategy_id,
        dataset_id: format!("dataset-{}", sequence % 20),
        config_id: format!("config-{}", sequence % 7),
        metrics_version: "strategy-metrics-v1".to_string(),
        seed: sequence as u64,
        created_sequence: sequence,
        metrics: vec![
            metric("net_profit", sequence as f64),
            metric("max_drawdown_percent", (sequence % 100) as f64),
            metric("sharpe_ratio", (sequence % 31) as f64 / 10.0),
        ],
        tags: vec![format!("bucket-{}", sequence % 10), "synthetic".into()],
        parent_run_id: None,
        retest_of_run_id: None,
    }
}

#[test]
fn canonical_equivalents_deduplicate_to_one_strategy_row() {
    let store = DatabankStore::open_in_memory().unwrap();
    let definition = NnfxBuilderConfig::default().to_definition().unwrap();
    let first = StrategyIr::build(&definition).unwrap();
    let mut reordered = definition;
    reordered.indicators.reverse();
    reordered.roles.reverse();
    let second = StrategyIr::build(&reordered).unwrap();

    assert_eq!(
        store.put_strategy(&first).unwrap(),
        PutStrategyOutcome::Inserted
    );
    assert_eq!(
        store.put_strategy(&second).unwrap(),
        PutStrategyOutcome::AlreadyPresent
    );
    assert_eq!(store.strategy_count().unwrap(), 1);
    assert_eq!(store.load_strategy(first.strategy_id()).unwrap(), first);
}

#[test]
fn runs_are_append_only_and_exact_metrics_survive_reload_and_rerun_verification() {
    let store = DatabankStore::open_in_memory().unwrap();
    let strategy = NnfxBuilderConfig::default().to_ir().unwrap();
    store.put_strategy(&strategy).unwrap();
    let input = run_input("run-exact".into(), strategy.strategy_id().into(), 7);
    store.append_run(&input).unwrap();

    let loaded = store.load_run("run-exact").unwrap();
    assert_eq!(loaded.metrics, input.metrics);
    store
        .verify_rerun_metrics("run-exact", &input.metrics)
        .unwrap();
    let mismatch = vec![metric("net_profit", 8.0)];
    assert!(matches!(
        store.verify_rerun_metrics("run-exact", &mismatch),
        Err(DatabankError::MetricsMismatch { .. })
    ));
    assert!(matches!(
        store.append_run(&input),
        Err(DatabankError::ImmutableRun { .. })
    ));
    assert!(store.test_only_update_run("run-exact").is_err());
    assert!(store.test_only_delete_run("run-exact").is_err());
}

#[test]
fn indexed_query_over_one_hundred_thousand_runs_is_bounded() {
    let store = DatabankStore::open_in_memory().unwrap();
    let strategy = NnfxBuilderConfig::default().to_ir().unwrap();
    store.put_strategy(&strategy).unwrap();
    let started = Instant::now();
    store
        .seed_synthetic_runs(100_000, strategy.strategy_id())
        .unwrap();
    assert!(started.elapsed() < Duration::from_secs(30));

    let query = DatabankQuery {
        tag: Some("bucket-7".into()),
        min_net_profit: Some(50_000.0),
        sort: DatabankSort::NetProfitDesc,
        offset: 120,
        limit: MAX_DATABANK_PAGE_SIZE,
        ..DatabankQuery::default()
    };
    let plan = store.explain_query(&query).unwrap();
    assert!(
        plan.iter().any(|line| line.contains("USING INDEX")),
        "{plan:?}"
    );
    assert!(!plan.iter().any(|line| line == "SCAN runs"), "{plan:?}");
    let page = store.query_runs(&query).unwrap();
    assert_eq!(page.rows.len(), MAX_DATABANK_PAGE_SIZE);
    assert!(page.has_more);
    assert!(
        page.rows
            .windows(2)
            .all(|pair| pair[0].net_profit >= pair[1].net_profit)
    );
}

#[test]
fn bounded_worker_executes_sqlite_off_submitter_and_drops_stale_cancelled_work() {
    let worker = DatabankWorker::spawn_in_memory().unwrap();
    let submitter = std::thread::current().id();
    let strategy = NnfxBuilderConfig::default().to_ir().unwrap();
    worker
        .submit(DatabankJob::PutStrategy {
            request_id: 1,
            strategy: Box::new(strategy),
        })
        .unwrap();
    worker.cancel(2);
    worker
        .submit(DatabankJob::Query {
            request_id: 2,
            query: DatabankQuery::default(),
        })
        .unwrap();

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut saw_worker = false;
    let mut saw_put = false;
    let mut saw_cancel = false;
    while Instant::now() < deadline && !(saw_put && saw_cancel) {
        for event in worker.poll() {
            assert_eq!(
                event.request_id(),
                if matches!(event, DatabankWorkerEvent::Cancelled { .. }) {
                    2
                } else {
                    1
                }
            );
            match event {
                DatabankWorkerEvent::Started { worker_thread, .. } => {
                    assert_ne!(worker_thread, submitter);
                    saw_worker = true;
                }
                DatabankWorkerEvent::StrategyPut { .. } => saw_put = true,
                DatabankWorkerEvent::Cancelled { .. } => saw_cancel = true,
                _ => {}
            }
        }
        std::thread::yield_now();
    }
    assert!(saw_worker && saw_put && saw_cancel);
}

#[test]
fn query_and_compare_limits_fail_closed() {
    let store = DatabankStore::open_in_memory().unwrap();
    let too_large = DatabankQuery {
        limit: MAX_DATABANK_PAGE_SIZE + 1,
        ..Default::default()
    };
    assert!(matches!(
        store.query_runs(&too_large),
        Err(DatabankError::LimitExceeded { .. })
    ));
    let ids = (0..=MAX_COMPARE_RUNS)
        .map(|i| format!("run-{i}"))
        .collect::<Vec<_>>();
    assert!(matches!(
        store.compare_runs(&ids),
        Err(DatabankError::LimitExceeded { .. })
    ));
}
