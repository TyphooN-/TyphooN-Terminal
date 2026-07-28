use crate::core::strategy_calendar::{TradingCalendar, TradingCalendarSpec};
use crate::core::strategy_corporate::{
    CorporateAction, CorporateActionKind, CorporateActionSchedule,
};
use crate::core::strategy_financing::{
    AccrualInterval, DayCount, FinancingModel, FinancingPolicy, RateProvenance, RateSource,
};
use crate::core::strategy_instrument::{InstrumentRegistry, InstrumentSpec};
use crate::core::strategy_ir::{OutsideSessionPolicy, ParticipationModel};

fn declared_rate(percent: f64) -> RateSource {
    RateSource::Declared {
        percent,
        provenance: RateProvenance::OperatorAssumption {
            note: "richer execution invariant test".into(),
        },
    }
}

#[test]
fn participation_cap_partially_fills_and_preserves_remainder() {
    let settings = ExecutionSettings {
        participation: ParticipationModel::BarVolumeFraction { fraction: 0.005 },
        ..free_settings()
    };
    let report = run(
        settings,
        &[ramp("aaa", 4)],
        &mut ScriptedStrategy::new(vec![(0, OrderSide::Buy, 12.0)]),
    )
    .expect("run succeeds");

    assert_eq!(
        report
            .fills
            .iter()
            .map(|fill| fill.quantity)
            .collect::<Vec<_>>(),
        vec![5.0, 5.0, 2.0]
    );
    assert_eq!(
        report
            .fills
            .iter()
            .map(|fill| fill.remaining_quantity)
            .collect::<Vec<_>>(),
        vec![7.0, 2.0, 0.0]
    );
    assert_eq!(
        report
            .events
            .iter()
            .filter(|event| event.kind == SimEventKind::PartialFill)
            .count(),
        2
    );
    assert!(report.pending_orders.is_empty());
    assert_close(report.positions[0].units, 12.0, "filled position");
}

#[test]
fn fill_deserialization_defaults_legacy_currency_fields_to_parity() {
    let report = run(
        free_settings(),
        &[ramp("aaa", 3)],
        &mut ScriptedStrategy::new(vec![(0, OrderSide::Buy, 1.0)]),
    )
    .unwrap();
    let mut value = serde_json::to_value(&report.fills[0]).unwrap();
    let object = value.as_object_mut().unwrap();
    object.remove("remaining_quantity");
    object.remove("conversion_rate");
    object.remove("conversion_cost");
    let legacy: FillRecord = serde_json::from_value(value).unwrap();
    assert_eq!(legacy.remaining_quantity, 0.0);
    assert_eq!(legacy.conversion_rate, 1.0);
    assert_eq!(legacy.conversion_cost, 0.0);
}

#[test]
fn closed_session_rejects_or_queues_without_an_out_of_session_fill() {
    let monday_open_ns = chrono::DateTime::parse_from_rfc3339("2026-03-09T13:29:00Z")
        .unwrap()
        .timestamp_nanos_opt()
        .unwrap();
    let bars = (0..3)
        .map(|index| SimBar {
            open_time_ns: monday_open_ns + index * MINUTE_NS,
            close_time_ns: monday_open_ns + (index + 1) * MINUTE_NS - 1,
            open: 100.0 + index as f64,
            high: 101.0 + index as f64,
            low: 99.0 + index as f64,
            close: 100.5 + index as f64,
            volume: 1_000.0,
        })
        .collect();
    let stream = SymbolStream {
        symbol: "aaa".into(),
        bars,
    };
    let calendar = TradingCalendar::build(&TradingCalendarSpec::us_equity_regular()).unwrap();
    let instruments =
        InstrumentRegistry::build(&[InstrumentSpec::plain("aaa", "USD").with_calendar(calendar)])
            .unwrap();

    let rejected = run(
        ExecutionSettings {
            instruments: instruments.clone(),
            outside_session: OutsideSessionPolicy::Reject,
            ..free_settings()
        },
        std::slice::from_ref(&stream),
        &mut ScriptedStrategy::new(vec![(0, OrderSide::Buy, 1.0)]),
    )
    .unwrap();
    assert!(rejected.fills.is_empty());
    assert!(matches!(
        rejected.rejections.as_slice(),
        [RejectionRecord {
            reason: RejectionReason::SessionClosed { reason },
            ..
        }] if reason == "outside_window"
    ));

    let queued = run(
        ExecutionSettings {
            instruments,
            outside_session: OutsideSessionPolicy::Queue,
            ..free_settings()
        },
        &[stream],
        &mut ScriptedStrategy::new(vec![(0, OrderSide::Buy, 1.0)]),
    )
    .unwrap();
    assert_eq!(queued.rejections, vec![]);
    assert_eq!(queued.fills.len(), 1);
    assert_eq!(queued.fills[0].time_ns, monday_open_ns + MINUTE_NS);
    assert_eq!(queued.fills[0].fill_price, 101.0);
}

#[test]
fn split_then_dividend_adjusts_live_position_and_cash_in_canonical_order() {
    let actions = CorporateActionSchedule::build(&[
        CorporateAction {
            symbol: "aaa".into(),
            effective_time_ns: 2 * MINUTE_NS,
            kind: CorporateActionKind::CashDividend {
                amount_per_unit: 1.0,
            },
        },
        CorporateAction {
            symbol: "aaa".into(),
            effective_time_ns: 2 * MINUTE_NS,
            kind: CorporateActionKind::Split {
                numerator: 2,
                denominator: 1,
            },
        },
    ])
    .unwrap();
    let report = run(
        ExecutionSettings {
            corporate_actions: actions,
            ..free_settings()
        },
        &[ramp("aaa", 4)],
        &mut ScriptedStrategy::new(vec![(0, OrderSide::Buy, 1.0)]),
    )
    .unwrap();

    assert_eq!(
        report
            .corporate_actions
            .iter()
            .map(|record| record.kind.as_str())
            .collect::<Vec<_>>(),
        vec!["split", "cash_dividend"]
    );
    assert_close(
        report.corporate_actions[0].units_before,
        1.0,
        "pre-split units",
    );
    assert_close(
        report.corporate_actions[0].units_after,
        2.0,
        "post-split units",
    );
    assert_close(
        report.corporate_actions[0].avg_entry_after,
        50.5,
        "post-split basis",
    );
    assert_close(
        report.corporate_actions[1].cash_delta,
        2.0,
        "post-split dividend",
    );
    assert_close(report.positions[0].units, 2.0, "final split units");
    assert_close(
        report.final_cash,
        START_CAPITAL - 101.0 + 2.0,
        "cash ledger",
    );
}

#[test]
fn sub_bar_fidelity_uses_the_earlier_path_step_before_parent_ambiguity() {
    let stream = stream_from(
        "aaa",
        &[
            (100.0, 101.0, 99.0, 100.0),
            (100.0, 101.0, 99.0, 100.0),
            (100.0, 111.0, 94.0, 101.0),
        ],
    );
    let half = MINUTE_NS / 2;
    let sub_bar =
        |parent: i64, half_index: i64, open: f64, high: f64, low: f64, close: f64| SimBar {
            open_time_ns: parent * MINUTE_NS + half_index * half,
            close_time_ns: parent * MINUTE_NS + (half_index + 1) * half - 1,
            open,
            high,
            low,
            close,
            volume: 500.0,
        };
    let path = SubBarPath {
        symbol: "aaa".into(),
        bars: vec![
            sub_bar(0, 0, 100.0, 100.5, 99.0, 100.0),
            sub_bar(0, 1, 100.0, 101.0, 99.5, 100.0),
            sub_bar(1, 0, 100.0, 100.5, 99.0, 100.0),
            sub_bar(1, 1, 100.0, 101.0, 99.5, 100.0),
            // The target is reached before the stop. The parent bar reaches
            // both, so a stop-first parent-level ambiguity rule would choose
            // the opposite exit if level-3 sequencing were not actually used.
            sub_bar(2, 0, 100.0, 111.0, 99.0, 110.0),
            sub_bar(2, 1, 110.0, 110.0, 94.0, 101.0),
        ],
    };
    let mut strategy = OrderScript::new(vec![
        (0, OrderRequest::market(SymbolId(0), OrderSide::Buy, 1.0)),
        (
            1,
            OrderRequest::stop(SymbolId(0), OrderSide::Sell, 1.0, 95.0).with_oco(1),
        ),
        (
            1,
            OrderRequest::limit(SymbolId(0), OrderSide::Sell, 1.0, 110.0).with_oco(1),
        ),
    ]);
    let settings = ExecutionSettings {
        fidelity: FidelityLevel::SubBar {
            sub_bar_seconds: 30,
        },
        ambiguity: OhlcAmbiguityPolicy::StopFirst,
        ..free_settings()
    };
    let report = run_simulation_with_paths(
        &config(settings),
        &SimulationSetup::default(),
        &[stream],
        &[path],
        &mut strategy,
    )
    .expect("sub-bar path is complete and contained");

    assert_eq!(report.fills.len(), 2);
    assert_close(report.fills[1].fill_price, 110.0, "earlier sub-bar target");
    assert!(
        report
            .cancellations
            .iter()
            .any(|cancel| cancel.reason == CancelReason::OcoSibling)
    );
}

#[test]
fn financing_uses_last_committed_mark_and_reconciles_report_totals() {
    let policy = FinancingPolicy {
        day_count: DayCount::Act365Fixed,
        accrual: AccrualInterval::FixedSeconds { seconds: 120 },
        long_financing_annual_percent: declared_rate(365.0),
        short_financing_annual_percent: RateSource::NotApplicable,
        short_borrow_annual_percent: RateSource::NotApplicable,
        funding_interval_percent: declared_rate(1.0),
    };
    let instruments = InstrumentRegistry::build(&[
        InstrumentSpec::plain("aaa", "USD").with_financing(FinancingModel::Accrued(policy))
    ])
    .unwrap();
    let report = run(
        ExecutionSettings {
            instruments,
            ..free_settings()
        },
        &[ramp("aaa", 4)],
        &mut ScriptedStrategy::new(vec![(0, OrderSide::Buy, 1.0)]),
    )
    .unwrap();

    assert_eq!(report.financing_charges.len(), 1);
    let charge = &report.financing_charges[0];
    assert_eq!(charge.time_ns, 2 * MINUTE_NS);
    assert_eq!(charge.seconds_accrued, 120);
    assert_close(charge.units, 1.0, "financed units");
    assert_close(charge.mark_price, 101.5, "last committed mark");
    let expected_financing = 101.5 * 365.0 / 100.0 * 120.0 / (365.0 * 86_400.0);
    let expected_funding = 101.5 * 1.0 / 100.0;
    assert_close(charge.financing, expected_financing, "financing debit");
    assert_close(charge.funding, expected_funding, "funding debit");
    assert_close(
        charge.total,
        charge.financing + charge.funding,
        "charge identity",
    );
    assert_close(
        report.total_financing_cost,
        charge.total,
        "report financing total",
    );
    assert_close(
        report.final_cash,
        START_CAPITAL - 101.0 - charge.total,
        "cash after financing",
    );
}
