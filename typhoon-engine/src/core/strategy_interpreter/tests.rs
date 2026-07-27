use super::*;
use crate::core::strategy_ir::{
    CommissionModel, DecisionTiming, ExecutionSettings, ExecutionTiming, IndicatorRole, NewsFilter,
    NewsImpact, OhlcAmbiguityPolicy, PositionSizing, RoleAssignment, SessionFilter, SessionWindow,
    SlippageModel, SpreadModel, StopRule, StrategyExecutionConfig, StrategyMetadata,
    StrategyParameter, TieBreakPolicy, TradeLeg, TradeManagement, TrailingStop,
};
use crate::core::strategy_simulator::{
    FillRecord, SimBar, SimulationError, SimulationReport, SimulationSetup, SymbolStream,
    run_simulation,
};

const MINUTE_NS: i64 = 60_000_000_000;
const EPS: f64 = 1e-9;

// ── Fixtures ───────────────────────────────────────────────────────

fn assert_close(actual: f64, expected: f64, what: &str) {
    assert!(
        (actual - expected).abs() < EPS,
        "{what}: expected {expected}, got {actual}"
    );
}

/// A stream whose closes are exactly `closes`, on a one-minute grid. Each bar
/// spans one point either side of its close so the range is predictable.
fn stream(symbol: &str, closes: &[f64]) -> SymbolStream {
    let bars = closes
        .iter()
        .enumerate()
        .map(|(index, close)| SimBar {
            open_time_ns: index as i64 * MINUTE_NS,
            close_time_ns: index as i64 * MINUTE_NS + MINUTE_NS - 1,
            open: *close,
            high: close + 1.0,
            low: close - 1.0,
            close: *close,
            volume: 1_000.0,
        })
        .collect();
    SymbolStream {
        symbol: symbol.to_string(),
        bars,
    }
}

fn settings() -> ExecutionSettings {
    ExecutionSettings {
        initial_capital: 100_000.0,
        account_currency: "USD".to_string(),
        commission: CommissionModel::None,
        slippage: SlippageModel::None,
        spread: SpreadModel::None,
        ambiguity: OhlcAmbiguityPolicy::StopFirst,
        tie_break: TieBreakPolicy::TimestampPrioritySequence,
        ..ExecutionSettings::conservative_defaults()
    }
}

fn config() -> StrategyExecutionConfig {
    StrategyExecutionConfig::build(&settings()).expect("settings are valid")
}

fn metadata() -> StrategyMetadata {
    StrategyMetadata {
        name: "interpreter fixture".to_string(),
        author: "typhoon".to_string(),
        notes: None,
        tags: Vec::new(),
    }
}

fn single_leg() -> TradeManagement {
    TradeManagement {
        legs: vec![TradeLeg {
            fraction_bps: 10_000,
            stop: None,
            target: None,
            trailing: None,
        }],
        break_even_after: None,
        max_bars_in_trade: None,
    }
}

/// The minimum strategy the interpreter accepts: closed-bar decisions, fixed
/// size, no filters, one whole-position leg.
fn definition(
    indicators: Vec<IndicatorNode>,
    long: DirectionRules,
    short: DirectionRules,
) -> StrategyDefinition {
    StrategyDefinition {
        metadata: metadata(),
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
            rule: SizingRule::FixedUnits { units: 10.0 },
            max_open_positions: 8,
        },
        trade_management: single_leg(),
        timing: ExecutionTiming {
            decision: DecisionTiming::ClosedBar,
            forming_bar_visible: false,
            submit_delay_bars: 0,
        },
    }
}

fn rules(enabled: bool, entry: Condition, exit: Condition) -> DirectionRules {
    DirectionRules {
        enabled,
        entry,
        exit,
    }
}

fn idle() -> DirectionRules {
    rules(false, Condition::Never, Condition::Never)
}

fn close_at(bars_ago: u32) -> Operand {
    Operand::Price {
        field: PriceField::Close,
        bars_ago,
    }
}

fn indicator_at(id: &str, bars_ago: u32) -> Operand {
    Operand::Indicator {
        id: id.to_string(),
        bars_ago,
    }
}

fn above(threshold: f64) -> Condition {
    Condition::Compare {
        left: close_at(0),
        op: CompareOp::Greater,
        right: Operand::Constant(threshold),
    }
}

fn below(threshold: f64) -> Condition {
    Condition::Compare {
        left: close_at(0),
        op: CompareOp::Less,
        right: Operand::Constant(threshold),
    }
}

fn strategy(definition: &StrategyDefinition) -> CanonicalIrStrategy {
    let ir = StrategyIr::build(definition).expect("definition is valid");
    CanonicalIrStrategy::new(&ir).expect("definition is interpretable")
}

fn build_error(definition: &StrategyDefinition) -> InterpreterError {
    let ir = StrategyIr::build(definition).expect("definition is valid");
    CanonicalIrStrategy::new(&ir).expect_err("definition must be refused")
}

fn run(definition: &StrategyDefinition, streams: &[SymbolStream]) -> SimulationReport {
    let mut interpreter = strategy(definition);
    run_simulation(
        &config(),
        &SimulationSetup::default(),
        streams,
        &mut interpreter,
    )
    .expect("simulation succeeds")
}

fn rejection(error: &SimulationError) -> String {
    match error {
        SimulationError::Strategy {
            error: StrategyError::Rejected { reason },
            ..
        } => reason.clone(),
        other => panic!("expected a strategy rejection, got {other:?}"),
    }
}

/// Feed a calculator a plain series and collect what it produces per sample.
fn series(kind: CompiledKind, samples: &[f64]) -> Vec<Option<f64>> {
    let mut calc = Calc::new(kind);
    samples
        .iter()
        .map(|sample| {
            calc.update(CalcInput::Series(*sample), "probe")
                .expect("series calculator accepts a series")
        })
        .collect()
}

/// Feed a calculator bars as `(high, low, close)`.
fn bars(kind: CompiledKind, samples: &[(f64, f64, f64)]) -> Vec<Option<f64>> {
    let mut calc = Calc::new(kind);
    samples
        .iter()
        .map(|(high, low, close)| {
            calc.update(
                CalcInput::Bar {
                    high: *high,
                    low: *low,
                    close: *close,
                },
                "probe",
            )
            .expect("bar calculator accepts a bar")
        })
        .collect()
}

fn first_value_index(values: &[Option<f64>]) -> Option<usize> {
    values.iter().position(Option::is_some)
}

fn ramp(count: usize) -> Vec<f64> {
    (0..count).map(|index| 100.0 + index as f64).collect()
}

// ── Indicator formulas ─────────────────────────────────────────────

#[test]
fn sma_averages_its_window_after_warm_up() {
    let values = series(CompiledKind::Sma { period: 3 }, &[1.0, 2.0, 3.0, 4.0]);
    assert_eq!(values[0], None);
    assert_eq!(values[1], None);
    assert_close(values[2].expect("warm"), 2.0, "sma at index 2");
    assert_close(values[3].expect("warm"), 3.0, "sma at index 3");
}

#[test]
fn std_dev_uses_the_population_deviation() {
    let values = series(CompiledKind::StdDev { period: 3 }, &[1.0, 2.0, 3.0]);
    assert_eq!(first_value_index(&values), Some(2));
    // Population variance of {1, 2, 3} is 2/3.
    assert_close(
        values[2].expect("warm"),
        (2.0f64 / 3.0).sqrt(),
        "population deviation",
    );
}

#[test]
fn ema_seeds_from_a_simple_average_then_smooths() {
    let values = series(CompiledKind::Ema { period: 3 }, &[1.0, 2.0, 3.0, 4.0]);
    assert_eq!(first_value_index(&values), Some(2));
    assert_close(values[2].expect("warm"), 2.0, "ema seed");
    // alpha = 2 / (3 + 1) = 0.5, so the next value is halfway to 4.
    assert_close(values[3].expect("warm"), 3.0, "ema step");
}

#[test]
fn atr_treats_the_first_visible_bar_as_its_own_range() {
    // Each bar spans 2.0 and steps 1.0, so every true range is exactly 2.0.
    let samples: Vec<(f64, f64, f64)> = ramp(6)
        .into_iter()
        .map(|close| (close + 1.0, close - 1.0, close))
        .collect();
    let values = bars(CompiledKind::Atr { period: 3 }, &samples);
    assert_eq!(first_value_index(&values), Some(2));
    for (index, value) in values.iter().enumerate().skip(2) {
        assert_close(value.expect("warm"), 2.0, &format!("atr at index {index}"));
    }
}

#[test]
fn atr_smooths_a_range_expansion_the_wilder_way() {
    let mut samples: Vec<(f64, f64, f64)> = vec![
        (101.0, 99.0, 100.0),
        (101.0, 99.0, 100.0),
        (101.0, 99.0, 100.0),
    ];
    // A bar spanning 8.0 after three quiet 2.0 bars.
    samples.push((104.0, 96.0, 100.0));
    let values = bars(CompiledKind::Atr { period: 3 }, &samples);
    assert_close(values[2].expect("warm"), 2.0, "atr seed");
    // (2 * (3 - 1) + 8) / 3
    assert_close(values[3].expect("warm"), 4.0, "wilder step");
}

#[test]
fn rsi_matches_a_hand_computed_wilder_average() {
    let values = series(CompiledKind::Rsi { period: 2 }, &[10.0, 11.0, 10.5, 12.0]);
    assert_eq!(first_value_index(&values), Some(2));
    // Gains {1, 0} average 0.5, losses {0, 0.5} average 0.25, so RS = 2.
    assert_close(values[2].expect("warm"), 100.0 - 100.0 / 3.0, "rsi seed");
    // Wilder: gain (0.5 + 1.5) / 2 = 1.0, loss (0.25 + 0) / 2 = 0.125, RS = 8.
    assert_close(values[3].expect("warm"), 100.0 - 100.0 / 9.0, "rsi step");
}

#[test]
fn rsi_pins_at_the_extremes_without_dividing_by_zero() {
    let rising = series(CompiledKind::Rsi { period: 3 }, &ramp(8));
    assert_close(rising[7].expect("warm"), 100.0, "rsi with no losses");

    let falling: Vec<f64> = ramp(8).into_iter().rev().collect();
    let falling = series(CompiledKind::Rsi { period: 3 }, &falling);
    assert_close(falling[7].expect("warm"), 0.0, "rsi with no gains");

    let flat = series(CompiledKind::Rsi { period: 3 }, &[5.0; 8]);
    assert_close(flat[7].expect("warm"), 50.0, "rsi with no movement");
}

#[test]
fn adx_needs_two_periods_and_stays_inside_its_scale() {
    let quiet = vec![(100.0, 100.0, 100.0); 12];
    let values = bars(CompiledKind::Adx { period: 3 }, &quiet);
    // One bar to seed the directional movement, three to sum it, three more
    // to average the resulting DX: 2 * period - 1.
    assert_eq!(first_value_index(&values), Some(5));
    assert_close(values[5].expect("warm"), 0.0, "adx of a flat market");

    let trend: Vec<(f64, f64, f64)> = ramp(20)
        .into_iter()
        .map(|close| (close + 1.0, close - 1.0, close))
        .collect();
    let trending = bars(CompiledKind::Adx { period: 3 }, &trend);
    let last = trending[19].expect("warm");
    assert!(
        (0.0..=100.0).contains(&last),
        "adx must stay on its scale, got {last}"
    );
    assert!(
        last > 50.0,
        "a clean uptrend should read strong, got {last}"
    );
}

#[test]
fn kama_seeds_at_the_sample_then_adapts_to_efficiency() {
    let values = series(
        CompiledKind::Kama {
            er_period: 2,
            fast: 2,
            slow: 30,
        },
        &[100.0, 101.0, 102.0, 103.0],
    );
    assert_eq!(first_value_index(&values), Some(2));
    assert_close(values[2].expect("warm"), 102.0, "kama seed");
    // A straight ramp is perfectly efficient, so the constant is the fast
    // alpha squared: (2 / 3)^2 = 4 / 9.
    let expected = 102.0 + (4.0 / 9.0) * (103.0 - 102.0);
    assert_close(values[3].expect("warm"), expected, "kama step");
}

#[test]
fn kama_holds_still_when_the_series_does_not_move() {
    let values = series(
        CompiledKind::Kama {
            er_period: 3,
            fast: 2,
            slow: 30,
        },
        &[42.0; 6],
    );
    assert_eq!(first_value_index(&values), Some(3));
    assert_close(values[5].expect("warm"), 42.0, "kama on a flat series");
}

#[test]
fn fisher_transform_is_centred_on_a_flat_series() {
    let values = series(CompiledKind::Fisher { period: 3 }, &[7.0; 6]);
    assert_eq!(first_value_index(&values), Some(2));
    assert_close(values[5].expect("warm"), 0.0, "fisher of a flat range");
}

#[test]
fn fisher_transform_leans_positive_on_a_rising_series() {
    let values = series(CompiledKind::Fisher { period: 3 }, &ramp(8));
    assert_eq!(first_value_index(&values), Some(2));
    let last = values[7].expect("warm");
    assert!(
        last > 0.0,
        "a rising series should read positive, got {last}"
    );
    assert!(last.is_finite(), "the transform must stay finite");
}

#[test]
fn macd_reports_the_histogram_and_flattens_on_a_flat_series() {
    let values = series(
        CompiledKind::Macd {
            fast: 2,
            slow: 4,
            signal: 3,
        },
        &[50.0; 10],
    );
    // Both averages warm at max(fast, slow) = 4 samples, then the signal
    // needs 3 of the resulting line: index 3 + 3 - 1 = 5.
    assert_eq!(first_value_index(&values), Some(5));
    assert_close(values[9].expect("warm"), 0.0, "macd histogram, flat series");
}

#[test]
fn macd_histogram_tracks_building_momentum() {
    // A straight ramp has constant momentum, so the line and its signal
    // converge and the histogram reads flat.
    let steady = series(
        CompiledKind::Macd {
            fast: 2,
            slow: 4,
            signal: 3,
        },
        &ramp(24),
    );
    assert_close(
        steady[23].expect("warm"),
        0.0,
        "histogram of a constant-slope ramp",
    );

    // An accelerating series keeps the line ahead of its own average.
    let accelerating: Vec<f64> = (0..24).map(|index| 100.0 * 1.05f64.powi(index)).collect();
    let values = series(
        CompiledKind::Macd {
            fast: 2,
            slow: 4,
            signal: 3,
        },
        &accelerating,
    );
    let last = values[23].expect("warm");
    assert!(
        last > 0.0,
        "building momentum should read positive, got {last}"
    );
}

#[test]
fn every_built_in_states_its_warm_up() {
    let samples = ramp(64);
    let expectations = [
        (CompiledKind::Sma { period: 5 }, 4usize),
        (CompiledKind::StdDev { period: 5 }, 4),
        (CompiledKind::Ema { period: 5 }, 4),
        (CompiledKind::Rsi { period: 5 }, 5),
        (CompiledKind::Fisher { period: 5 }, 4),
        (
            CompiledKind::Kama {
                er_period: 5,
                fast: 2,
                slow: 30,
            },
            5,
        ),
        (
            CompiledKind::Macd {
                fast: 5,
                slow: 8,
                signal: 3,
            },
            9,
        ),
    ];
    for (kind, expected) in expectations {
        let values = series(kind, &samples);
        assert_eq!(
            first_value_index(&values),
            Some(expected),
            "warm-up for {kind:?}"
        );
    }

    let bar_samples: Vec<(f64, f64, f64)> = samples
        .iter()
        .map(|close| (close + 1.0, close - 1.0, *close))
        .collect();
    for (kind, expected) in [
        (CompiledKind::Atr { period: 5 }, 4usize),
        (CompiledKind::Adx { period: 5 }, 9),
    ] {
        let values = bars(kind, &bar_samples);
        assert_eq!(
            first_value_index(&values),
            Some(expected),
            "warm-up for {kind:?}"
        );
    }
}

#[test]
fn a_calculator_refuses_the_wrong_input_shape() {
    let mut calc = Calc::new(CompiledKind::Atr { period: 3 });
    let error = calc
        .update(CalcInput::Series(1.0), "atr")
        .expect_err("an atr needs a bar");
    assert!(matches!(
        error,
        InterpreterError::InputShapeMismatch {
            expected: "bar",
            ..
        }
    ));

    let mut calc = Calc::new(CompiledKind::Sma { period: 3 });
    let error = calc
        .update(
            CalcInput::Bar {
                high: 1.0,
                low: 1.0,
                close: 1.0,
            },
            "sma",
        )
        .expect_err("an sma needs a series");
    assert!(matches!(
        error,
        InterpreterError::InputShapeMismatch {
            expected: "series",
            ..
        }
    ));
}

// ── Conditions ─────────────────────────────────────────────────────

/// Entry on `condition`, no exit, so a fill is proof the condition was true.
fn entry_only(condition: Condition) -> StrategyDefinition {
    definition(Vec::new(), rules(true, condition, Condition::Never), idle())
}

fn entry_bar_indices(report: &SimulationReport) -> Vec<i64> {
    report
        .fills
        .iter()
        .map(|fill| fill.time_ns / MINUTE_NS)
        .collect()
}

#[test]
fn comparison_operators_read_the_committed_close() {
    // The trailing bar exists so the last possible entry still has an open to
    // fill against.
    let closes = [10.0, 20.0, 30.0, 30.0];
    for op in [
        CompareOp::Greater,
        CompareOp::GreaterOrEqual,
        CompareOp::Less,
        CompareOp::LessOrEqual,
        CompareOp::Equal,
        CompareOp::NotEqual,
    ] {
        let definition = entry_only(Condition::Compare {
            left: close_at(0),
            op,
            right: Operand::Constant(20.0),
        });
        let report = run(&definition, &[stream("AAA", &closes)]);
        let expected: Vec<i64> = match op {
            // The first bar that satisfies the operator decides; the fill
            // lands on the bar after it.
            CompareOp::Greater => vec![3],
            CompareOp::GreaterOrEqual | CompareOp::Equal => vec![2],
            CompareOp::Less | CompareOp::LessOrEqual | CompareOp::NotEqual => vec![1],
        };
        assert_eq!(
            entry_bar_indices(&report),
            expected,
            "first fill for {op:?}"
        );
    }
}

#[test]
fn crosses_above_needs_the_previous_bar_not_to_be_above() {
    let definition = entry_only(Condition::CrossesAbove {
        left: close_at(0),
        right: Operand::Constant(20.0),
    });
    // Already above at bar 0, dips under, then crosses at bar 3.
    let report = run(
        &definition,
        &[stream("AAA", &[30.0, 10.0, 15.0, 25.0, 26.0])],
    );
    assert_eq!(
        entry_bar_indices(&report),
        vec![4],
        "only the genuine crossing fires"
    );
}

#[test]
fn crosses_below_needs_the_previous_bar_not_to_be_below() {
    let definition = entry_only(Condition::CrossesBelow {
        left: close_at(0),
        right: Operand::Constant(20.0),
    });
    let report = run(
        &definition,
        &[stream("AAA", &[10.0, 30.0, 25.0, 15.0, 14.0])],
    );
    assert_eq!(entry_bar_indices(&report), vec![4], "one crossing");
}

#[test]
fn a_cross_is_unknown_until_two_bars_exist() {
    let definition = entry_only(Condition::CrossesAbove {
        left: close_at(0),
        right: Operand::Constant(5.0),
    });
    // Bar 0 is already above the level, but there is no previous bar to have
    // been below it, so nothing crosses.
    let report = run(&definition, &[stream("AAA", &[10.0, 11.0])]);
    assert!(report.fills.is_empty(), "no fill from a one-bar history");
    assert_eq!(report.pending_orders.len(), 0, "and nothing submitted");
}

#[test]
fn all_any_and_not_combine_children() {
    // Both closes sit above 20 and below 40, so each combinator has one
    // reading for the whole run.
    let closes = [30.0, 31.0];
    let cases: [(Condition, bool); 6] = [
        (Condition::All(vec![above(5.0), above(20.0)]), true),
        (Condition::All(vec![above(5.0), above(40.0)]), false),
        (Condition::Any(vec![above(40.0), above(20.0)]), true),
        (Condition::Any(vec![above(40.0), above(50.0)]), false),
        (Condition::Not(Box::new(above(40.0))), true),
        (Condition::Not(Box::new(above(20.0))), false),
    ];
    for (condition, expected) in cases {
        let definition = entry_only(condition.clone());
        let report = run(&definition, &[stream("AAA", &closes)]);
        assert_eq!(
            !report.fills.is_empty() || !report.pending_orders.is_empty(),
            expected,
            "combinator {condition:?}"
        );
    }
}

#[test]
fn unknown_operands_keep_conditions_unknown_through_not() {
    // `close` five bars back does not exist yet, so the comparison is unknown
    // and its negation stays unknown rather than firing an entry.
    let definition = entry_only(Condition::Not(Box::new(Condition::Compare {
        left: close_at(5),
        op: CompareOp::Greater,
        right: Operand::Constant(0.0),
    })));
    let report = run(&definition, &[stream("AAA", &[10.0, 11.0, 12.0])]);
    assert!(
        report.fills.is_empty() && report.pending_orders.is_empty(),
        "a negated unknown must not trade"
    );

    // With enough history the same rule resolves and stays false, because
    // every close is above zero.
    let report = run(&definition, &[stream("AAA", &[10.0; 8])]);
    assert!(report.fills.is_empty(), "the resolved rule is false");
}

#[test]
fn an_unknown_child_does_not_rescue_a_false_all() {
    let definition = entry_only(Condition::All(vec![
        Condition::Never,
        Condition::Compare {
            left: close_at(5),
            op: CompareOp::Greater,
            right: Operand::Constant(0.0),
        },
    ]));
    let report = run(&definition, &[stream("AAA", &[10.0; 8])]);
    assert!(report.fills.is_empty(), "false dominates unknown in All");
}

#[test]
fn indicator_operands_stay_unknown_through_warm_up() {
    let indicators = vec![IndicatorNode {
        id: "sma".to_string(),
        kind: IndicatorKind::Sma,
        inputs: vec![
            IndicatorInput::Price(PriceField::Close),
            IndicatorInput::Constant(4.0),
        ],
    }];
    let definition = definition(
        indicators,
        rules(
            true,
            Condition::Compare {
                left: close_at(0),
                op: CompareOp::Greater,
                right: indicator_at("sma", 0),
            },
            Condition::Never,
        ),
        idle(),
    );
    // A rising ramp: the close is above its own average from the first bar
    // the average exists, which is bar index 3.
    let report = run(&definition, &[stream("AAA", &ramp(6))]);
    assert_eq!(
        entry_bar_indices(&report),
        vec![4],
        "the first decision after warm-up trades"
    );
}

#[test]
fn parameter_operands_use_the_declared_typed_value() {
    let mut definition = entry_only(Condition::Compare {
        left: close_at(0),
        op: CompareOp::Greater,
        right: Operand::Parameter("level".to_string()),
    });
    definition.parameters = vec![StrategyParameter {
        id: "level".to_string(),
        // An integer parameter is read as the integer it declares, not
        // re-parsed from its text form.
        value: ParamValue::Int(25),
        range: None,
    }];
    let report = run(&definition, &[stream("AAA", &[10.0, 20.0, 30.0, 30.0])]);
    assert_eq!(entry_bar_indices(&report), vec![3], "clears 25 at bar 2");
}

#[test]
fn a_parameter_period_sizes_the_indicator_window() {
    let mut definition = definition(
        vec![IndicatorNode {
            id: "sma".to_string(),
            kind: IndicatorKind::Sma,
            inputs: vec![
                IndicatorInput::Price(PriceField::Close),
                IndicatorInput::Parameter("period".to_string()),
            ],
        }],
        rules(
            true,
            Condition::Compare {
                left: indicator_at("sma", 0),
                op: CompareOp::Greater,
                right: Operand::Constant(0.0),
            },
            Condition::Never,
        ),
        idle(),
    );
    definition.parameters = vec![StrategyParameter {
        id: "period".to_string(),
        value: ParamValue::Int(3),
        range: None,
    }];
    let report = run(&definition, &[stream("AAA", &ramp(6))]);
    // The average exists from bar 2, so the order fills at the open of bar 3.
    assert_eq!(entry_bar_indices(&report), vec![3], "period-3 warm-up");
}

// ── Lowering to orders ─────────────────────────────────────────────

#[test]
fn a_long_entry_fills_at_the_next_bar_open() {
    let definition = entry_only(above(20.0));
    let closes = [10.0, 30.0, 31.0];
    let streams = [stream("AAA", &closes)];
    let report = run(&definition, &streams);

    assert_eq!(report.fills.len(), 1, "one entry");
    let fill: &FillRecord = &report.fills[0];
    assert_eq!(fill.side, OrderSide::Buy);
    assert_close(fill.quantity, 10.0, "fixed size");
    // The decision happened at the close of bar 1; the fill takes bar 2's
    // open, never bar 1's own prices.
    assert_eq!(fill.time_ns, 2 * MINUTE_NS, "fill lands on the next bar");
    assert_close(fill.reference_price, 31.0, "next bar open");
    assert_close(report.positions[0].units, 10.0, "position is long");
}

#[test]
fn a_long_exit_closes_the_position() {
    let definition = definition(Vec::new(), rules(true, above(20.0), below(15.0)), idle());
    let report = run(
        &definition,
        &[stream("AAA", &[10.0, 30.0, 31.0, 10.0, 11.0])],
    );
    let sides: Vec<OrderSide> = report.fills.iter().map(|fill| fill.side).collect();
    assert_eq!(sides, vec![OrderSide::Buy, OrderSide::Sell], "in and out");
    assert_close(report.positions[0].units, 0.0, "flat at the end");
    assert_eq!(
        report.fills[1].time_ns,
        4 * MINUTE_NS,
        "exit fills next bar"
    );
}

#[test]
fn a_short_entry_and_exit_mirror_the_long_side() {
    let definition = definition(Vec::new(), idle(), rules(true, below(15.0), above(25.0)));
    let report = run(
        &definition,
        &[stream("AAA", &[20.0, 10.0, 11.0, 30.0, 31.0])],
    );
    let sides: Vec<OrderSide> = report.fills.iter().map(|fill| fill.side).collect();
    assert_eq!(
        sides,
        vec![OrderSide::Sell, OrderSide::Buy],
        "short then cover"
    );
    assert_close(report.fills[0].quantity, 10.0, "fixed size");
    assert_close(report.positions[0].units, 0.0, "flat at the end");
}

#[test]
fn a_repeated_entry_signal_does_not_pyramid() {
    let definition = entry_only(above(20.0));
    // The entry condition holds for five consecutive bars.
    let report = run(&definition, &[stream("AAA", &[30.0; 5])]);
    assert_eq!(report.fills.len(), 1, "one entry only");
    assert_close(report.positions[0].units, 10.0, "one unit of size");
}

#[test]
fn an_opposite_entry_alone_never_flips_a_position() {
    // Long entry above 20 with an exit that never fires; the short entry
    // fires later but the long position is still held.
    let definition = definition(
        Vec::new(),
        rules(true, above(20.0), Condition::Never),
        rules(true, below(15.0), Condition::Never),
    );
    let report = run(&definition, &[stream("AAA", &[30.0, 31.0, 10.0, 11.0])]);
    assert_eq!(report.fills.len(), 1, "only the original entry");
    assert_eq!(report.fills[0].side, OrderSide::Buy);
    assert_close(report.positions[0].units, 10.0, "still long");
}

#[test]
fn an_exit_and_an_opposite_entry_reverse_in_one_decision() {
    let definition = definition(
        Vec::new(),
        rules(true, above(20.0), below(15.0)),
        rules(true, below(15.0), above(25.0)),
    );
    // Bar 1 closes above 20 (long), bar 2 closes below 15, which both exits
    // the long and enters the short at the same decision.
    let report = run(&definition, &[stream("AAA", &[18.0, 30.0, 10.0, 11.0])]);
    let sides: Vec<OrderSide> = report.fills.iter().map(|fill| fill.side).collect();
    assert_eq!(
        sides,
        vec![OrderSide::Buy, OrderSide::Sell, OrderSide::Sell],
        "close and reverse are separate intents"
    );
    // The close and the new short both fill at the same next open, in the
    // order they were submitted.
    assert_eq!(report.fills[1].time_ns, report.fills[2].time_ns);
    assert!(report.fills[1].sequence < report.fills[2].sequence);
    assert_close(report.positions[0].units, -10.0, "now short");
}

#[test]
fn two_true_entries_stand_aside() {
    let definition = definition(
        Vec::new(),
        rules(true, Condition::Always, Condition::Never),
        rules(true, Condition::Always, Condition::Never),
    );
    let report = run(&definition, &[stream("AAA", &[10.0, 11.0, 12.0])]);
    assert!(
        report.fills.is_empty() && report.pending_orders.is_empty(),
        "a contradiction produces no order"
    );
}

#[test]
fn a_disabled_direction_never_trades() {
    let definition = definition(
        Vec::new(),
        rules(false, Condition::Always, Condition::Never),
        rules(true, below(15.0), Condition::Never),
    );
    let report = run(&definition, &[stream("AAA", &[30.0, 31.0, 32.0])]);
    assert!(report.fills.is_empty(), "the long side is switched off");
}

#[test]
fn max_open_positions_caps_concurrent_symbols() {
    let mut definition = entry_only(Condition::Always);
    definition.sizing.max_open_positions = 2;
    let streams = [
        stream("AAA", &[10.0, 11.0, 12.0]),
        stream("BBB", &[20.0, 21.0, 22.0]),
        stream("CCC", &[30.0, 31.0, 32.0]),
    ];
    let report = run(&definition, &streams);
    let held = report
        .positions
        .iter()
        .filter(|position| position.units != 0.0)
        .count();
    assert_eq!(held, 2, "the third symbol is refused a slot");
    // Symbols are decided in sorted order, so the cap lands on CCC.
    assert_close(report.positions[2].units, 0.0, "CCC stays flat");
}

// ── Refusals at build time ─────────────────────────────────────────

#[test]
fn a_custom_indicator_is_refused() {
    let definition = definition(
        vec![IndicatorNode {
            id: "plugin".to_string(),
            kind: IndicatorKind::Custom {
                name: "waddah_attar".to_string(),
                implementation_id: "a".repeat(64),
            },
            inputs: vec![IndicatorInput::Price(PriceField::Close)],
        }],
        rules(
            true,
            Condition::Compare {
                left: indicator_at("plugin", 0),
                op: CompareOp::Greater,
                right: Operand::Constant(0.0),
            },
            Condition::Never,
        ),
        idle(),
    );
    assert!(
        matches!(
            build_error(&definition),
            InterpreterError::UnsupportedIndicator { .. }
        ),
        "a custom implementation has no reference semantics"
    );
}

#[test]
fn equity_based_sizing_is_refused() {
    for rule in [
        SizingRule::PercentEquity { percent: 2.0 },
        SizingRule::RiskPercentAtr {
            risk_percent: 1.0,
            atr_multiple: 1.5,
            atr_indicator: "atr".to_string(),
        },
    ] {
        let mut definition = definition(
            vec![IndicatorNode {
                id: "atr".to_string(),
                kind: IndicatorKind::Atr,
                inputs: vec![IndicatorInput::Constant(14.0)],
            }],
            rules(true, Condition::Always, Condition::Never),
            idle(),
        );
        definition.sizing.rule = rule;
        assert!(
            matches!(
                build_error(&definition),
                InterpreterError::Unsupported { .. }
            ),
            "account state is not visible from a decision"
        );
    }
}

#[test]
fn all_identity_bearing_timing_modes_compile_for_the_simulator() {
    for timing in [
        ExecutionTiming {
            decision: DecisionTiming::NextBarOpen,
            forming_bar_visible: false,
            submit_delay_bars: 0,
        },
        ExecutionTiming {
            decision: DecisionTiming::PreClose { offset_seconds: 30 },
            forming_bar_visible: true,
            submit_delay_bars: 0,
        },
        ExecutionTiming {
            decision: DecisionTiming::ClosedBar,
            forming_bar_visible: false,
            submit_delay_bars: 2,
        },
    ] {
        let mut definition = definition(
            Vec::new(),
            rules(true, Condition::Always, Condition::Never),
            idle(),
        );
        definition.timing = timing;
        let ir = StrategyIr::build(&definition).expect("timing-valid strategy seals");
        CanonicalIrStrategy::new(&ir)
            .unwrap_or_else(|error| panic!("timing {timing:?} must compile: {error}"));
    }
}

#[test]
fn session_and_news_filters_are_refused() {
    let base = definition(
        Vec::new(),
        rules(true, Condition::Always, Condition::Never),
        idle(),
    );

    let mut session = base.clone();
    session.session = SessionFilter {
        enabled: true,
        windows: vec![SessionWindow {
            start_minute: 480,
            end_minute: 1_020,
        }],
        close_positions_outside: false,
    };
    assert!(matches!(
        build_error(&session),
        InterpreterError::Unsupported {
            feature: "session filter",
            ..
        }
    ));

    let mut news = base;
    news.news = NewsFilter {
        enabled: true,
        min_impact: NewsImpact::High,
        block_minutes_before: 30,
        block_minutes_after: 30,
        close_open_positions: false,
    };
    assert!(matches!(
        build_error(&news),
        InterpreterError::Unsupported {
            feature: "news filter",
            ..
        }
    ));
}

#[test]
fn resting_order_trade_management_is_refused() {
    let base = definition(
        Vec::new(),
        rules(true, Condition::Always, Condition::Never),
        idle(),
    );

    let mut stops = base.clone();
    stops.trade_management.legs[0].stop = Some(StopRule::PercentOfEntry { percent: 2.0 });
    assert!(matches!(
        build_error(&stops),
        InterpreterError::Unsupported { .. }
    ));

    let mut trailing = base.clone();
    trailing.trade_management.legs[0].trailing = Some(TrailingStop {
        distance: StopRule::PriceDistance { distance: 1.0 },
        activate_after: None,
    });
    assert!(matches!(
        build_error(&trailing),
        InterpreterError::Unsupported { .. }
    ));

    let mut scaled = base.clone();
    scaled.trade_management.legs = vec![
        TradeLeg {
            fraction_bps: 5_000,
            stop: None,
            target: None,
            trailing: None,
        },
        TradeLeg {
            fraction_bps: 5_000,
            stop: None,
            target: None,
            trailing: None,
        },
    ];
    assert!(matches!(
        build_error(&scaled),
        InterpreterError::Unsupported {
            feature: "trade_management.legs",
            ..
        }
    ));

    let mut timed = base;
    timed.trade_management.max_bars_in_trade = Some(20);
    assert!(matches!(
        build_error(&timed),
        InterpreterError::Unsupported { .. }
    ));
}

#[test]
fn a_fractional_period_is_refused() {
    let definition = definition(
        vec![IndicatorNode {
            id: "sma".to_string(),
            kind: IndicatorKind::Sma,
            inputs: vec![
                IndicatorInput::Price(PriceField::Close),
                IndicatorInput::Constant(4.5),
            ],
        }],
        rules(
            true,
            Condition::Compare {
                left: indicator_at("sma", 0),
                op: CompareOp::Greater,
                right: Operand::Constant(0.0),
            },
            Condition::Never,
        ),
        idle(),
    );
    assert!(matches!(
        build_error(&definition),
        InterpreterError::InvalidPeriod { .. }
    ));
}

#[test]
fn an_oversized_period_is_refused() {
    let definition = definition(
        vec![IndicatorNode {
            id: "sma".to_string(),
            kind: IndicatorKind::Sma,
            inputs: vec![
                IndicatorInput::Price(PriceField::Close),
                IndicatorInput::Constant((MAX_INDICATOR_PERIOD + 1) as f64),
            ],
        }],
        rules(
            true,
            Condition::Compare {
                left: indicator_at("sma", 0),
                op: CompareOp::Greater,
                right: Operand::Constant(0.0),
            },
            Condition::Never,
        ),
        idle(),
    );
    assert!(matches!(
        build_error(&definition),
        InterpreterError::InvalidPeriod { .. }
    ));
}

#[test]
fn per_symbol_state_stays_inside_its_budget() {
    let definition = definition(
        vec![IndicatorNode {
            id: "adx".to_string(),
            kind: IndicatorKind::Adx,
            // 4 * 4096 window slots is past the per-symbol budget.
            inputs: vec![IndicatorInput::Constant(MAX_INDICATOR_PERIOD as f64)],
        }],
        rules(
            true,
            Condition::Compare {
                left: indicator_at("adx", 0),
                op: CompareOp::Greater,
                right: Operand::Constant(0.0),
            },
            Condition::Never,
        ),
        idle(),
    );
    assert!(matches!(
        build_error(&definition),
        InterpreterError::StateTooLarge { .. }
    ));
}

#[test]
fn a_chained_indicator_warms_up_behind_its_source() {
    let definition = definition(
        vec![
            IndicatorNode {
                id: "inner".to_string(),
                kind: IndicatorKind::Sma,
                inputs: vec![
                    IndicatorInput::Price(PriceField::Close),
                    IndicatorInput::Constant(2.0),
                ],
            },
            IndicatorNode {
                id: "outer".to_string(),
                kind: IndicatorKind::Sma,
                inputs: vec![
                    IndicatorInput::Indicator("inner".to_string()),
                    IndicatorInput::Constant(2.0),
                ],
            },
        ],
        rules(
            true,
            Condition::Compare {
                left: indicator_at("outer", 0),
                op: CompareOp::Greater,
                right: Operand::Constant(0.0),
            },
            Condition::Never,
        ),
        idle(),
    );
    // `inner` warms at bar 1, so `outer` needs bar 2 for its second sample.
    let report = run(&definition, &[stream("AAA", &ramp(6))]);
    assert_eq!(entry_bar_indices(&report), vec![3], "chained warm-up");
}

// ── Runtime defence ────────────────────────────────────────────────

#[test]
fn an_indicator_slot_past_the_table_is_reported_not_panicked() {
    let definition = entry_only(Condition::Always);
    let mut interpreter = strategy(&definition);
    // The IR could never say this; the interpreter still must not index blind.
    interpreter.program.long.entry = CompiledCondition::Compare {
        left: CompiledOperand::Indicator {
            slot: 7,
            bars_ago: 0,
        },
        op: CompareOp::Greater,
        right: CompiledOperand::Constant(0.0),
    };
    let error = run_simulation(
        &config(),
        &SimulationSetup::default(),
        &[stream("AAA", &[10.0, 11.0])],
        &mut interpreter,
    )
    .expect_err("a bad slot must fail the run");
    assert!(
        rejection(&error).contains("indicator slot 7"),
        "unexpected rejection: {}",
        rejection(&error)
    );
}

#[test]
fn a_lookback_past_the_retained_state_is_reported() {
    let definition = definition(
        vec![IndicatorNode {
            id: "sma".to_string(),
            kind: IndicatorKind::Sma,
            inputs: vec![
                IndicatorInput::Price(PriceField::Close),
                IndicatorInput::Constant(2.0),
            ],
        }],
        rules(
            true,
            Condition::Compare {
                left: indicator_at("sma", 0),
                op: CompareOp::Greater,
                right: Operand::Constant(0.0),
            },
            Condition::Never,
        ),
        idle(),
    );
    let mut interpreter = strategy(&definition);
    interpreter.program.long.entry = CompiledCondition::Compare {
        left: CompiledOperand::Indicator {
            slot: 0,
            // The ring was sized for the deepest declared lookback plus two.
            bars_ago: 64,
        },
        op: CompareOp::Greater,
        right: CompiledOperand::Constant(0.0),
    };
    let error = run_simulation(
        &config(),
        &SimulationSetup::default(),
        &[stream("AAA", &ramp(4))],
        &mut interpreter,
    )
    .expect_err("an oversized lookback must fail the run");
    assert!(
        rejection(&error).contains("lookback 64"),
        "unexpected rejection: {}",
        rejection(&error)
    );
}

#[test]
fn a_history_ring_reports_warm_up_and_defect_differently() {
    let mut history = History::new(2);
    // Nothing written yet: a legal depth is unknown, an illegal one is an
    // error.
    assert_eq!(history.get(1), Ok(None));
    assert!(matches!(
        history.get(2),
        Err(InterpreterError::LookbackOutOfRange { .. })
    ));

    history.push(Some(1.0));
    history.push(Some(2.0));
    history.push(Some(3.0));
    assert_eq!(history.get(0), Ok(Some(3.0)));
    assert_eq!(history.get(1), Ok(Some(2.0)));
}

#[test]
fn an_unknown_indicator_slot_is_caught_by_the_reader() {
    let definition = entry_only(Condition::Always);
    let interpreter = strategy(&definition);
    let state = SymbolState::new(&interpreter.program);
    assert!(matches!(
        indicator_value(&state, 3, 0),
        Err(InterpreterError::UnknownIndicatorSlot { slot: 3, count: 0 })
    ));
}

#[test]
fn replaying_without_a_reset_is_refused() {
    let definition = entry_only(Condition::Never);
    let mut interpreter = strategy(&definition);
    let streams = [stream("AAA", &ramp(4))];
    run_simulation(
        &config(),
        &SimulationSetup::default(),
        &streams,
        &mut interpreter,
    )
    .expect("first run succeeds");

    let error = run_simulation(
        &config(),
        &SimulationSetup::default(),
        &streams,
        &mut interpreter,
    )
    .expect_err("stale state must not be reused");
    assert!(
        rejection(&error).contains("interpreter expected"),
        "unexpected rejection: {}",
        rejection(&error)
    );
}

#[test]
fn reset_restores_a_reusable_interpreter() {
    let definition = definition(Vec::new(), rules(true, above(20.0), below(15.0)), idle());
    let streams = [stream("AAA", &[10.0, 30.0, 31.0, 10.0, 11.0])];
    let mut interpreter = strategy(&definition);
    let first = run_simulation(
        &config(),
        &SimulationSetup::default(),
        &streams,
        &mut interpreter,
    )
    .expect("first run");
    interpreter.reset();
    let second = run_simulation(
        &config(),
        &SimulationSetup::default(),
        &streams,
        &mut interpreter,
    )
    .expect("second run");
    assert_eq!(first, second, "a reset interpreter replays identically");
}

// ── Determinism ────────────────────────────────────────────────────

#[test]
fn two_fresh_interpreters_produce_identical_reports() {
    let definition = definition(
        vec![IndicatorNode {
            id: "ema".to_string(),
            kind: IndicatorKind::Ema,
            inputs: vec![
                IndicatorInput::Price(PriceField::Close),
                IndicatorInput::Constant(3.0),
            ],
        }],
        rules(
            true,
            Condition::CrossesAbove {
                left: close_at(0),
                right: indicator_at("ema", 0),
            },
            Condition::CrossesBelow {
                left: close_at(0),
                right: indicator_at("ema", 0),
            },
        ),
        rules(
            true,
            Condition::CrossesBelow {
                left: close_at(0),
                right: indicator_at("ema", 0),
            },
            Condition::CrossesAbove {
                left: close_at(0),
                right: indicator_at("ema", 0),
            },
        ),
    );
    let closes: Vec<f64> = (0..40)
        .map(|index| 100.0 + ((index as f64) * 0.7).sin() * 5.0)
        .collect();
    let streams = [stream("AAA", &closes), stream("BBB", &ramp(40))];

    let first = run(&definition, &streams);
    let second = run(&definition, &streams);
    assert_eq!(first, second, "identical inputs give identical reports");
    assert!(!first.fills.is_empty(), "the fixture must actually trade");

    let encoded = serde_json::to_string(&first).expect("report serializes");
    let re_encoded = serde_json::to_string(&second).expect("report serializes");
    assert_eq!(encoded, re_encoded, "and identical encodings");
}

#[test]
fn decisions_do_not_depend_on_bars_that_have_not_happened() {
    let definition = definition(
        vec![IndicatorNode {
            id: "sma".to_string(),
            kind: IndicatorKind::Sma,
            inputs: vec![
                IndicatorInput::Price(PriceField::Close),
                IndicatorInput::Constant(3.0),
            ],
        }],
        rules(
            true,
            Condition::CrossesAbove {
                left: close_at(0),
                right: indicator_at("sma", 0),
            },
            Condition::CrossesBelow {
                left: close_at(0),
                right: indicator_at("sma", 0),
            },
        ),
        idle(),
    );
    let closes: Vec<f64> = (0..24)
        .map(|index| 100.0 + ((index as f64) * 0.9).sin() * 4.0)
        .collect();

    let full = run(&definition, &[stream("AAA", &closes)]);
    let prefix_length = 12usize;
    let prefix = run(&definition, &[stream("AAA", &closes[..prefix_length])]);

    // Every fill the truncated run produced must appear, identically, in the
    // full run. A rule that peeked at a later bar would disagree here.
    let horizon = prefix_length as i64 * MINUTE_NS;
    let full_within: Vec<(i64, OrderSide, f64)> = full
        .fills
        .iter()
        .filter(|fill| fill.time_ns < horizon)
        .map(|fill| (fill.time_ns, fill.side, fill.fill_price))
        .collect();
    let prefix_within: Vec<(i64, OrderSide, f64)> = prefix
        .fills
        .iter()
        .filter(|fill| fill.time_ns < horizon)
        .map(|fill| (fill.time_ns, fill.side, fill.fill_price))
        .collect();
    assert_eq!(
        full_within, prefix_within,
        "a prefix of the data must produce a prefix of the decisions"
    );
    assert!(!prefix_within.is_empty(), "the fixture must trade early");
}

#[test]
fn indicators_track_each_symbol_separately() {
    let definition = definition(
        vec![IndicatorNode {
            id: "sma".to_string(),
            kind: IndicatorKind::Sma,
            inputs: vec![
                IndicatorInput::Price(PriceField::Close),
                IndicatorInput::Constant(2.0),
            ],
        }],
        rules(
            true,
            Condition::Compare {
                left: indicator_at("sma", 0),
                op: CompareOp::Greater,
                right: Operand::Constant(500.0),
            },
            Condition::Never,
        ),
        idle(),
    );
    // Only BBB averages above 500; a shared state machine would smear the two
    // symbols together and trade both.
    let streams = [
        stream("AAA", &[10.0, 11.0, 12.0]),
        stream("BBB", &[900.0, 910.0, 920.0]),
    ];
    let report = run(&definition, &streams);
    assert_close(report.positions[0].units, 0.0, "AAA stays flat");
    assert_close(report.positions[1].units, 10.0, "BBB trades");
}

#[test]
fn roles_do_not_change_what_the_interpreter_evaluates() {
    let indicators = vec![IndicatorNode {
        id: "atr".to_string(),
        kind: IndicatorKind::Atr,
        inputs: vec![IndicatorInput::Constant(3.0)],
    }];
    let mut with_roles = definition(
        indicators.clone(),
        rules(
            true,
            Condition::Compare {
                left: indicator_at("atr", 0),
                op: CompareOp::Greater,
                right: Operand::Constant(1.0),
            },
            Condition::Never,
        ),
        idle(),
    );
    let without_roles = with_roles.clone();
    with_roles.roles = vec![RoleAssignment {
        role: IndicatorRole::Atr,
        indicator: "atr".to_string(),
    }];

    let streams = [stream("AAA", &ramp(8))];
    assert_eq!(
        run(&with_roles, &streams).fills,
        run(&without_roles, &streams).fills,
        "roles are a view, not a second rule set"
    );
}

#[test]
fn a_bar_indicator_drives_a_real_decision() {
    // ATR of a two-point range is 2.0, so the entry fires once warm.
    let definition = definition(
        vec![IndicatorNode {
            id: "atr".to_string(),
            kind: IndicatorKind::Atr,
            inputs: vec![IndicatorInput::Constant(3.0)],
        }],
        rules(
            true,
            Condition::Compare {
                left: indicator_at("atr", 0),
                op: CompareOp::GreaterOrEqual,
                right: Operand::Constant(2.0),
            },
            Condition::Never,
        ),
        idle(),
    );
    let report = run(&definition, &[stream("AAA", &ramp(6))]);
    assert_eq!(entry_bar_indices(&report), vec![3], "atr warms at bar 2");
}
