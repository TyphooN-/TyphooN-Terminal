// ── Hybrid replay corpus (ADR-135 §13 M2 gate, §6.13) ──────────────
//
// The gate clause is that "replaying a recorded hybrid run reproduces its
// ledger bit-for-bit". These tests record a session in which an automated
// strategy and an operator both act, seal the log, replay it, and compare the
// serialized ledgers byte for byte.

use super::{
    HybridRecorder, HybridReplay, Intervention, InterventionAction, InterventionError,
    InterventionLog,
};
use crate::core::strategy_ir::{
    CommissionModel, ExecutionSettings, FidelityLevel, LatencyModel, OhlcAmbiguityPolicy,
    SlippageModel, SpreadModel, StrategyExecutionConfig, TieBreakPolicy,
};
use crate::core::strategy_simulator::{
    ClientOrderId, DecisionContext, ModifyRequest, OrderIntents, OrderRequest, OrderSide,
    ReferenceStrategy, SimBar, SimulationReport, SimulationSetup, StrategyError, SymbolId,
    SymbolStream, run_simulation,
};

const MINUTE_NS: i64 = 60_000_000_000;
const SECOND_NS: i64 = 1_000_000_000;

/// The canonical serialization of one run. Two runs are "the same run" only if
/// these are byte-identical.
fn ledger_bytes(report: &SimulationReport) -> Vec<u8> {
    serde_json::to_vec(report).expect("the report serializes")
}

fn ramp(symbol: &str, count: usize) -> SymbolStream {
    let bars = (0..count)
        .map(|index| {
            let open = 100.0 + index as f64;
            SimBar {
                open_time_ns: index as i64 * MINUTE_NS,
                close_time_ns: index as i64 * MINUTE_NS + MINUTE_NS - 1,
                open,
                high: open + 1.0,
                low: open - 1.0,
                close: open + 0.5,
                volume: 1_000.0,
            }
        })
        .collect();
    SymbolStream {
        symbol: symbol.to_string(),
        bars,
    }
}

/// Costs, intrabar resolution and seeded latency all switched on, so replay has
/// to reproduce a stochastic-looking run rather than a trivially flat one.
fn hybrid_settings() -> ExecutionSettings {
    ExecutionSettings {
        fidelity: FidelityLevel::BarOhlc,
        commission: CommissionModel::PerShare {
            amount: 0.01,
            minimum: 1.0,
        },
        slippage: SlippageModel::FixedPriceDistance { distance: 0.02 },
        spread: SpreadModel::Constant { price_units: 0.10 },
        latency: LatencyModel::SeededUniform {
            decision_to_submit_min_ns: 0,
            decision_to_submit_max_ns: 5 * SECOND_NS,
            submit_to_exchange_min_ns: 0,
            submit_to_exchange_max_ns: SECOND_NS,
        },
        ambiguity: OhlcAmbiguityPolicy::StopFirst,
        tie_break: TieBreakPolicy::TimestampPrioritySequence,
        ..ExecutionSettings::conservative_defaults()
    }
}

fn run(streams: &[SymbolStream], strategy: &mut dyn ReferenceStrategy) -> SimulationReport {
    let config = StrategyExecutionConfig::build(&hybrid_settings()).expect("settings are valid");
    run_simulation(&config, &SimulationSetup::default(), streams, strategy).expect("runs")
}

/// Stands in for the automated half of a hybrid run: buys on decision 1 and
/// rests a limit exit on decision 2, so the operator has something to override.
#[derive(Default)]
struct AutomatedHalf {
    decisions: usize,
    resting_exit: Option<ClientOrderId>,
}

impl ReferenceStrategy for AutomatedHalf {
    fn on_bar_close(
        &mut self,
        ctx: &DecisionContext<'_>,
        orders: &mut OrderIntents,
    ) -> Result<(), StrategyError> {
        let now = self.decisions;
        self.decisions += 1;
        if ctx.symbol() != SymbolId(0) {
            return Ok(());
        }
        match now {
            1 => {
                orders.market(ctx.symbol(), OrderSide::Buy, 4.0)?;
            }
            2 => {
                let id = orders.submit(OrderRequest::limit(
                    ctx.symbol(),
                    OrderSide::Sell,
                    4.0,
                    130.0,
                ))?;
                self.resting_exit = Some(id);
            }
            _ => {}
        }
        Ok(())
    }
}

/// The operator script: at decision 4 they reprice the automated exit down to
/// something reachable, at decision 6 they add a hedge, and at decision 8 they
/// pull it. Client order ids are the ones `AutomatedHalf` and the operator
/// themselves were handed, which is what makes the log replayable.
fn operator_script() -> Vec<Intervention> {
    vec![
        Intervention {
            decision_index: 4,
            note: "target unreachable, bringing it in".to_string(),
            action: InterventionAction::Modify {
                target: ClientOrderId(1),
                change: ModifyRequest::limit_price(108.0),
            },
        },
        Intervention {
            decision_index: 6,
            note: "hedging into the close".to_string(),
            action: InterventionAction::Submit {
                request: OrderRequest::market(SymbolId(0), OrderSide::Sell, 1.0),
            },
        },
        Intervention {
            decision_index: 8,
            note: "adding a protective stop".to_string(),
            action: InterventionAction::Submit {
                request: OrderRequest::stop(SymbolId(0), OrderSide::Sell, 1.0, 95.0),
            },
        },
    ]
}

/// Records a hybrid session and returns both its ledger and the sealed log.
fn record_session() -> (SimulationReport, InterventionLog) {
    let streams = vec![ramp("aaa", 16)];
    let mut automated = AutomatedHalf::default();
    let script = operator_script();
    let mut recorder = HybridRecorder::new(&mut automated, move |index, _ctx| {
        script
            .iter()
            .filter(|entry| entry.decision_index == index)
            .cloned()
            .collect()
    });
    let report = run(&streams, &mut recorder);
    let log = recorder.into_log().expect("the session seals");
    (report, log)
}

/// The M2 gate clause.
#[test]
fn replaying_a_recorded_hybrid_run_reproduces_its_ledger_bit_for_bit() {
    let (recorded, log) = record_session();
    assert_eq!(log.interventions().len(), 3, "all three actions recorded");
    assert!(
        !recorded.fills.is_empty(),
        "the session must actually have traded, or the comparison is vacuous"
    );

    let streams = vec![ramp("aaa", 16)];
    let mut automated = AutomatedHalf::default();
    let mut replay = HybridReplay::new(&mut automated, &log);
    let replayed = run(&streams, &mut replay);

    assert_eq!(replay.applied(), 3, "every intervention was consumed");
    assert_eq!(
        ledger_bytes(&replayed),
        ledger_bytes(&recorded),
        "a replayed hybrid run must be byte-identical to the session it came from"
    );
}

/// Replay must survive the log going to disk and back, not just staying in
/// memory: the sealed artifact is what a stored run refers to.
#[test]
fn a_hybrid_run_replays_identically_from_a_round_tripped_log() {
    let (recorded, log) = record_session();
    let bytes = log.to_json_vec().expect("serializes");
    let restored = InterventionLog::from_json_slice(&bytes).expect("round trips");
    assert_eq!(restored, log);
    assert_eq!(restored.log_id(), log.log_id());

    let streams = vec![ramp("aaa", 16)];
    let mut automated = AutomatedHalf::default();
    let mut replay = HybridReplay::new(&mut automated, &restored);
    let replayed = run(&streams, &mut replay);

    assert_eq!(ledger_bytes(&replayed), ledger_bytes(&recorded));
}

/// The comparison above is only meaningful if dropping the log actually changes
/// the ledger. Otherwise the interventions were inert and replay proved nothing.
#[test]
fn the_operator_actions_are_what_make_the_two_ledgers_match() {
    let (recorded, _log) = record_session();
    let streams = vec![ramp("aaa", 16)];
    let mut automated = AutomatedHalf::default();
    let empty = InterventionLog::empty();
    let mut replay = HybridReplay::new(&mut automated, &empty);
    let without = run(&streams, &mut replay);

    assert_ne!(
        ledger_bytes(&without),
        ledger_bytes(&recorded),
        "an empty log must not reproduce a session that had interventions"
    );
}

#[test]
fn a_tampered_log_fails_verification_instead_of_replaying() {
    let (_report, log) = record_session();
    let bytes = log.to_json_vec().expect("serializes");
    let mut wire: serde_json::Value = serde_json::from_slice(&bytes).expect("json");

    // Move an intervention one decision later: the same actions, a different
    // run. The content address must notice.
    wire["interventions"][1]["decision_index"] = serde_json::json!(7);
    assert!(matches!(
        InterventionLog::from_json_slice(&serde_json::to_vec(&wire).expect("json")),
        Err(InterventionError::IdentityMismatch { .. })
    ));

    // Rewriting only the note is still a different log: the operator's stated
    // reason is part of the record, not decoration.
    let mut renoted: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
    renoted["interventions"][0]["note"] = serde_json::json!("something else entirely");
    assert!(matches!(
        InterventionLog::from_json_slice(&serde_json::to_vec(&renoted).expect("json")),
        Err(InterventionError::IdentityMismatch { .. })
    ));
}

#[test]
fn the_log_id_separates_order_from_content() {
    let first = InterventionLog::build(operator_script()).expect("builds");
    let same = InterventionLog::build(operator_script()).expect("builds");
    assert_eq!(first.log_id(), same.log_id(), "identity is repeatable");

    // Two interventions on the *same* decision are order-sensitive: which one
    // the exchange sees first can decide whether the second is even valid.
    let collide = |first_note: &str, second_note: &str| {
        InterventionLog::build(vec![
            Intervention {
                decision_index: 3,
                note: first_note.to_string(),
                action: InterventionAction::Cancel {
                    target: ClientOrderId(1),
                },
            },
            Intervention {
                decision_index: 3,
                note: second_note.to_string(),
                action: InterventionAction::Submit {
                    request: OrderRequest::market(SymbolId(0), OrderSide::Sell, 1.0),
                },
            },
        ])
        .expect("builds")
    };
    assert_ne!(
        collide("a", "b").log_id(),
        collide("b", "a").log_id(),
        "swapping two same-decision actions is a different log"
    );

    assert_ne!(
        first.log_id(),
        InterventionLog::empty().log_id(),
        "an empty log is not the same as a populated one"
    );
}

#[test]
fn an_out_of_order_log_is_rejected_rather_than_sorted() {
    let backwards = vec![
        Intervention {
            decision_index: 5,
            note: String::new(),
            action: InterventionAction::Cancel {
                target: ClientOrderId(0),
            },
        },
        Intervention {
            decision_index: 2,
            note: String::new(),
            action: InterventionAction::Cancel {
                target: ClientOrderId(1),
            },
        },
    ];
    assert_eq!(
        InterventionLog::build(backwards),
        Err(InterventionError::Unordered { at: 1 })
    );
}

#[test]
fn the_loader_is_bounded_and_version_checked_before_it_trusts_anything() {
    let oversized = vec![b' '; super::MAX_INTERVENTION_LOG_JSON_BYTES + 1];
    assert!(matches!(
        InterventionLog::from_json_slice(&oversized),
        Err(InterventionError::TooLarge { .. })
    ));

    let log = InterventionLog::build(operator_script()).expect("builds");
    let mut wire: serde_json::Value =
        serde_json::from_slice(&log.to_json_vec().expect("serializes")).expect("json");
    wire["schema_version"] = serde_json::json!(99);
    assert_eq!(
        InterventionLog::from_json_slice(&serde_json::to_vec(&wire).expect("json")),
        Err(InterventionError::UnsupportedSchemaVersion {
            found: 99,
            supported: super::INTERVENTION_LOG_SCHEMA_VERSION,
        })
    );

    let long_note = vec![Intervention {
        decision_index: 0,
        note: "x".repeat(super::MAX_INTERVENTIONS),
        action: InterventionAction::Cancel {
            target: ClientOrderId(0),
        },
    }];
    assert_eq!(
        InterventionLog::build(long_note),
        Err(InterventionError::NoteTooLong { at: 0 })
    );
}
