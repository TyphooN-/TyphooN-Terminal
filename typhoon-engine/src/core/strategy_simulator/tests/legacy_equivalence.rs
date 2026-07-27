use crate::broker::alpaca::Bar;
use crate::core::backtest::{
    FisherCrossStrategy, KAMACrossStrategy, NNFXStrategy, RSIMeanRevStrategy, SMACrossStrategy,
    Strategy, run_backtest,
};
use crate::core::strategy_interpreter::CanonicalIrStrategy;
use crate::core::strategy_ir::{
    CompareOp, Condition, DecisionTiming, DirectionRules, ExecutionCompatibility, ExecutionTiming,
    IndicatorInput, IndicatorKind, IndicatorNode, NewsFilter, NewsImpact, Operand, PositionSizing,
    PriceField, SessionFilter, SizingRule, StrategyDefinition, StrategyIr, StrategyMetadata,
    TradeLeg, TradeManagement,
};

fn operand(id: &str) -> Operand {
    Operand::Indicator {
        id: id.to_string(),
        bars_ago: 0,
    }
}

fn price() -> Operand {
    Operand::Price {
        field: PriceField::Close,
        bars_ago: 0,
    }
}

fn cross_above(left: Operand, right: Operand) -> Condition {
    Condition::CrossesAbove { left, right }
}

fn cross_below(left: Operand, right: Operand) -> Condition {
    Condition::CrossesBelow { left, right }
}

fn compare(left: Operand, op: CompareOp, right: f64) -> Condition {
    Condition::Compare {
        left,
        op,
        right: Operand::Constant(right),
    }
}

fn rules(entry: Condition, exit: Condition) -> DirectionRules {
    DirectionRules {
        enabled: true,
        entry,
        exit,
    }
}

fn node(id: &str, kind: IndicatorKind, inputs: Vec<IndicatorInput>) -> IndicatorNode {
    IndicatorNode {
        id: id.to_string(),
        kind,
        inputs,
    }
}

fn base_definition(
    name: &str,
    indicators: Vec<IndicatorNode>,
    long: DirectionRules,
    short: DirectionRules,
    initial_equity: f64,
) -> StrategyDefinition {
    StrategyDefinition {
        metadata: StrategyMetadata {
            name: name.to_string(),
            author: "legacy migration fixture".to_string(),
            notes: Some("versioned pre-ADR-135 semantics".to_string()),
            tags: vec!["legacy-v1".to_string()],
        },
        parameters: Vec::new(),
        indicators,
        roles: Vec::new(),
        long,
        short,
        session: SessionFilter {
            enabled: false,
            windows: Vec::new(),
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
            rule: SizingRule::LegacyFixedNotionalV1 {
                notional: initial_equity,
            },
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
    }
}

fn sma_definition(initial: f64) -> StrategyDefinition {
    let fast = operand("fast");
    let slow = operand("slow");
    let up = cross_above(fast.clone(), slow.clone());
    let down = cross_below(fast, slow);
    base_definition(
        "legacy SMA cross v1",
        vec![
            node(
                "fast",
                IndicatorKind::Sma,
                vec![
                    IndicatorInput::Price(PriceField::Close),
                    IndicatorInput::Constant(2.0),
                ],
            ),
            node(
                "slow",
                IndicatorKind::Sma,
                vec![
                    IndicatorInput::Price(PriceField::Close),
                    IndicatorInput::Constant(5.0),
                ],
            ),
        ],
        rules(up.clone(), down.clone()),
        rules(down, up),
        initial,
    )
}

fn nnfx_definition(initial: f64) -> StrategyDefinition {
    let up = Condition::All(vec![
        cross_above(price(), operand("kama")),
        compare(operand("fisher"), CompareOp::Greater, 0.0),
    ]);
    let down = Condition::All(vec![
        cross_below(price(), operand("kama")),
        compare(operand("fisher"), CompareOp::Less, 0.0),
    ]);
    base_definition(
        "legacy NNFX v1",
        vec![
            node(
                "kama",
                IndicatorKind::LegacyRollingKamaV1,
                vec![
                    IndicatorInput::Price(PriceField::Open),
                    IndicatorInput::Constant(3.0),
                    IndicatorInput::Constant(2.0),
                    IndicatorInput::Constant(30.0),
                ],
            ),
            node(
                "fisher",
                IndicatorKind::LegacyUnsmoothedFisherMidpointV1,
                vec![IndicatorInput::Constant(3.0)],
            ),
        ],
        rules(up.clone(), down.clone()),
        rules(down, up),
        initial,
    )
}

fn kama_definition(initial: f64) -> StrategyDefinition {
    let up = cross_above(price(), operand("kama"));
    let down = cross_below(price(), operand("kama"));
    base_definition(
        "legacy KAMA cross v1",
        vec![node(
            "kama",
            IndicatorKind::LegacyRollingKamaV1,
            vec![
                IndicatorInput::Price(PriceField::Close),
                IndicatorInput::Constant(3.0),
                IndicatorInput::Constant(2.0),
                IndicatorInput::Constant(10.0),
            ],
        )],
        rules(up.clone(), down.clone()),
        rules(down, up),
        initial,
    )
}

fn fisher_definition(initial: f64) -> StrategyDefinition {
    let up = cross_above(operand("fisher"), operand("signal"));
    let down = cross_below(operand("fisher"), operand("signal"));
    base_definition(
        "legacy Fisher cross v1",
        vec![
            node(
                "fisher",
                IndicatorKind::LegacyFisherValueV1,
                vec![IndicatorInput::Constant(3.0)],
            ),
            node(
                "signal",
                IndicatorKind::LegacyFisherSignalV1,
                vec![IndicatorInput::Constant(3.0)],
            ),
        ],
        rules(up.clone(), down.clone()),
        rules(down, up),
        initial,
    )
}

fn rsi_definition(initial: f64) -> StrategyDefinition {
    let long_entry = compare(operand("rsi"), CompareOp::Less, 35.0);
    let long_exit = compare(operand("rsi"), CompareOp::Greater, 50.0);
    let short_entry = compare(operand("rsi"), CompareOp::Greater, 65.0);
    let short_exit = compare(operand("rsi"), CompareOp::Less, 50.0);
    base_definition(
        "legacy RSI mean reversion v1",
        vec![node(
            "rsi",
            IndicatorKind::LegacyRollingRsiV1,
            vec![
                IndicatorInput::Price(PriceField::Close),
                IndicatorInput::Constant(3.0),
            ],
        )],
        rules(long_entry, long_exit),
        rules(short_entry, short_exit),
        initial,
    )
}

fn oscillating_corpus() -> (Vec<Bar>, SymbolStream) {
    let closes: [f64; 48] = [
        100.0, 98.0, 96.0, 94.0, 97.0, 101.0, 105.0, 108.0, 104.0, 100.0, 95.0, 91.0, 94.0, 99.0,
        104.0, 109.0, 106.0, 101.0, 96.0, 92.0, 95.0, 100.0, 106.0, 111.0, 107.0, 102.0, 97.0,
        93.0, 96.0, 102.0, 108.0, 112.0, 108.0, 103.0, 98.0, 94.0, 97.0, 103.0, 109.0, 113.0,
        109.0, 104.0, 99.0, 95.0, 98.0, 104.0, 110.0, 114.0,
    ];
    let mut legacy = Vec::with_capacity(closes.len());
    let mut simulated = Vec::with_capacity(closes.len());
    for (index, close) in closes.into_iter().enumerate() {
        let open = if index == 0 { close } else { closes[index - 1] };
        let high = open.max(close) + 1.0;
        let low = open.min(close) - 1.0;
        legacy.push(Bar {
            timestamp: format!("bar-{index:03}"),
            open,
            high,
            low,
            close,
            volume: 1_000.0 + index as f64,
        });
        simulated.push(SimBar {
            open_time_ns: index as i64 * MINUTE_NS,
            close_time_ns: (index as i64 + 1) * MINUTE_NS - 1,
            open,
            high,
            low,
            close,
            volume: 1_000.0 + index as f64,
        });
    }
    (
        legacy,
        SymbolStream {
            symbol: "legacy".into(),
            bars: simulated,
        },
    )
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct NormalizedFill {
    side: OrderSide,
    quantity: f64,
    price: f64,
}

fn legacy_fills(
    result: &crate::core::backtest::BacktestResult,
    initial: f64,
) -> Vec<NormalizedFill> {
    let mut fills = Vec::with_capacity(result.trades.len() * 2);
    for trade in &result.trades {
        let entry_side = if trade.side == "long" {
            OrderSide::Buy
        } else {
            OrderSide::Sell
        };
        let exit_side = if entry_side == OrderSide::Buy {
            OrderSide::Sell
        } else {
            OrderSide::Buy
        };
        fills.push(NormalizedFill {
            side: entry_side,
            quantity: initial / trade.entry_price,
            price: trade.entry_price,
        });
        fills.push(NormalizedFill {
            side: exit_side,
            quantity: initial / trade.entry_price,
            price: trade.exit_price,
        });
    }
    fills
}

fn simulated_round_trips(report: &SimulationReport) -> Vec<NormalizedFill> {
    report
        .fills
        .iter()
        .map(|fill| NormalizedFill {
            side: fill.side,
            quantity: fill.quantity,
            price: fill.fill_price,
        })
        .collect()
}

fn assert_fills(actual: &[NormalizedFill], expected: &[NormalizedFill], name: &str) {
    assert_eq!(actual.len(), expected.len(), "{name} fill count");
    for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        assert_eq!(actual.side, expected.side, "{name} fill {index} side");
        assert_close(
            actual.quantity,
            expected.quantity,
            &format!("{name} fill {index} quantity"),
        );
        assert_close(
            actual.price,
            expected.price,
            &format!("{name} fill {index} price"),
        );
    }
}

fn assert_legacy_equivalent<S: Strategy>(
    mut legacy_strategy: S,
    definition: StrategyDefinition,
    name: &str,
) {
    let (bars, stream) = oscillating_corpus();
    let initial_equity = START_CAPITAL;
    let legacy = run_backtest(&bars, &mut legacy_strategy, initial_equity);
    assert!(
        legacy.trades.len() >= 2,
        "{name} must exercise entry/exit or reversal plus final liquidation"
    );

    let ir = StrategyIr::build(&definition).expect("legacy definition seals");
    let mut canonical = CanonicalIrStrategy::new(&ir).expect("legacy definition compiles");
    let settings = ExecutionSettings {
        compatibility: ExecutionCompatibility::LegacySameBarClose,
        ..free_settings()
    };
    let config = StrategyExecutionConfig::build(&settings).expect("legacy config");
    let report = run_simulation(
        &config,
        &SimulationSetup::default(),
        &[stream],
        &mut canonical,
    )
    .expect("canonical legacy simulation");

    assert!(
        report.rejections.is_empty(),
        "{name} rejection ledger: {:?}",
        report.rejections
    );
    assert_fills(
        &simulated_round_trips(&report),
        &legacy_fills(&legacy, initial_equity),
        name,
    );
    assert_close(report.final_realized_pnl, legacy.report.total_pnl, name);
    assert_close(
        report.final_cash,
        initial_equity + legacy.report.total_pnl,
        name,
    );
    assert_close(
        report.final_equity,
        initial_equity + legacy.report.total_pnl,
        name,
    );
    assert_eq!(report.positions[0].units, 0.0, "{name} forced liquidation");
}

#[test]
fn canonical_ir_matches_all_five_legacy_strategies_end_to_end() {
    let initial = START_CAPITAL;
    assert_legacy_equivalent(
        SMACrossStrategy::new(2, 5),
        sma_definition(initial),
        "SMA Cross",
    );
    assert_legacy_equivalent(NNFXStrategy::new(3, 3), nnfx_definition(initial), "NNFX");
    assert_legacy_equivalent(
        KAMACrossStrategy::new(3, 2, 10),
        kama_definition(initial),
        "KAMA Cross",
    );
    assert_legacy_equivalent(
        FisherCrossStrategy::new(3),
        fisher_definition(initial),
        "Fisher Cross",
    );
    assert_legacy_equivalent(
        RSIMeanRevStrategy::new(3, 35.0, 65.0),
        rsi_definition(initial),
        "RSI Mean Reversion",
    );
}

#[test]
fn modern_fixed_units_and_persistent_indicator_defaults_remain_distinct() {
    let mut definition = kama_definition(START_CAPITAL);
    definition.metadata.name = "modern KAMA".to_string();
    definition.indicators[0].kind = IndicatorKind::Kama;
    definition.sizing.rule = SizingRule::FixedUnits { units: 7.0 };
    let ir = StrategyIr::build(&definition).expect("modern definition seals");
    let mut strategy = CanonicalIrStrategy::new(&ir).expect("modern definition compiles");
    let (_, stream) = oscillating_corpus();
    let report = run_simulation(
        &config(free_settings()),
        &SimulationSetup::default(),
        &[stream],
        &mut strategy,
    )
    .expect("modern simulation");
    assert!(report.fills.iter().all(|fill| fill.quantity == 7.0));
    assert!(
        report.positions[0].units != 0.0,
        "modern mode does not force liquidation"
    );
}
