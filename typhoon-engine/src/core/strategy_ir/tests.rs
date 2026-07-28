use super::*;
use crate::core::strategy_metrics::METRICS_SCHEMA_VERSION;
use std::collections::BTreeSet;

const CUSTOM_IMPLEMENTATION_ID: &str =
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

// ── Fixtures ───────────────────────────────────────────────────────

fn metadata() -> StrategyMetadata {
    StrategyMetadata {
        name: "NNFX baseline crossover".to_string(),
        author: "typhoon".to_string(),
        notes: Some("M1 identity fixture".to_string()),
        tags: vec!["nnfx".to_string(), "trend".to_string()],
    }
}

fn parameters() -> Vec<StrategyParameter> {
    vec![
        StrategyParameter {
            id: "atr_period".to_string(),
            value: ParamValue::Int(14),
            range: Some(ParamRange::Int { min: 5, max: 50 }),
        },
        StrategyParameter {
            id: "baseline_period".to_string(),
            value: ParamValue::Int(20),
            range: Some(ParamRange::Int { min: 5, max: 200 }),
        },
        StrategyParameter {
            id: "risk_percent".to_string(),
            value: ParamValue::Float(1.0),
            range: Some(ParamRange::Float { min: 0.1, max: 5.0 }),
        },
        StrategyParameter {
            id: "allow_shorts".to_string(),
            value: ParamValue::Bool(true),
            range: None,
        },
        StrategyParameter {
            id: "session_label".to_string(),
            value: ParamValue::Text("london".to_string()),
            range: None,
        },
    ]
}

fn indicators() -> Vec<IndicatorNode> {
    vec![
        IndicatorNode {
            id: "atr".to_string(),
            kind: IndicatorKind::Atr,
            inputs: vec![IndicatorInput::Parameter("atr_period".to_string())],
        },
        IndicatorNode {
            id: "baseline".to_string(),
            kind: IndicatorKind::Ema,
            inputs: vec![
                IndicatorInput::Price(PriceField::Close),
                IndicatorInput::Parameter("baseline_period".to_string()),
            ],
        },
        IndicatorNode {
            id: "confirmation".to_string(),
            kind: IndicatorKind::Custom {
                name: "waddah_attar".to_string(),
                implementation_id: CUSTOM_IMPLEMENTATION_ID.to_string(),
            },
            inputs: vec![
                IndicatorInput::Indicator("baseline".to_string()),
                IndicatorInput::Constant(2.0),
            ],
        },
    ]
}

fn roles() -> Vec<RoleAssignment> {
    vec![
        RoleAssignment {
            role: IndicatorRole::Atr,
            indicator: "atr".to_string(),
        },
        RoleAssignment {
            role: IndicatorRole::Baseline,
            indicator: "baseline".to_string(),
        },
        RoleAssignment {
            role: IndicatorRole::Confirmation1,
            indicator: "confirmation".to_string(),
        },
    ]
}

fn close(bars_ago: u32) -> Operand {
    Operand::Price {
        field: PriceField::Close,
        bars_ago,
    }
}

fn indicator_operand(id: &str, bars_ago: u32) -> Operand {
    Operand::Indicator {
        id: id.to_string(),
        bars_ago,
    }
}

fn long_rules() -> DirectionRules {
    DirectionRules {
        enabled: true,
        entry: Condition::All(vec![
            Condition::CrossesAbove {
                left: close(0),
                right: indicator_operand("baseline", 0),
            },
            Condition::Compare {
                left: indicator_operand("confirmation", 0),
                op: CompareOp::Greater,
                right: Operand::Constant(0.0),
            },
        ]),
        exit: Condition::CrossesBelow {
            left: close(0),
            right: indicator_operand("baseline", 0),
        },
    }
}

fn short_rules() -> DirectionRules {
    DirectionRules {
        enabled: true,
        entry: Condition::All(vec![
            Condition::CrossesBelow {
                left: close(0),
                right: indicator_operand("baseline", 0),
            },
            Condition::Compare {
                left: indicator_operand("confirmation", 0),
                op: CompareOp::Less,
                right: Operand::Constant(0.0),
            },
        ]),
        exit: Condition::CrossesAbove {
            left: close(0),
            right: indicator_operand("baseline", 0),
        },
    }
}

fn trade_management() -> TradeManagement {
    TradeManagement {
        legs: vec![
            TradeLeg {
                fraction_bps: 5_000,
                stop: Some(StopRule::AtrMultiple {
                    indicator: "atr".to_string(),
                    multiple: 1.5,
                }),
                target: Some(StopRule::AtrMultiple {
                    indicator: "atr".to_string(),
                    multiple: 1.0,
                }),
                trailing: None,
            },
            TradeLeg {
                fraction_bps: 5_000,
                stop: Some(StopRule::AtrMultiple {
                    indicator: "atr".to_string(),
                    multiple: 1.5,
                }),
                target: None,
                trailing: Some(TrailingStop {
                    distance: StopRule::AtrMultiple {
                        indicator: "atr".to_string(),
                        multiple: 1.5,
                    },
                    activate_after: Some(StopRule::AtrMultiple {
                        indicator: "atr".to_string(),
                        multiple: 1.0,
                    }),
                }),
            },
        ],
        break_even_after: Some(StopRule::AtrMultiple {
            indicator: "atr".to_string(),
            multiple: 1.0,
        }),
        max_bars_in_trade: Some(200),
    }
}

fn definition() -> StrategyDefinition {
    StrategyDefinition {
        metadata: metadata(),
        parameters: parameters(),
        indicators: indicators(),
        roles: roles(),
        long: long_rules(),
        short: short_rules(),
        session: SessionFilter {
            enabled: true,
            windows: vec![SessionWindow {
                start_minute: 480,
                end_minute: 1_020,
            }],
            close_positions_outside: false,
        },
        news: NewsFilter {
            enabled: true,
            min_impact: NewsImpact::High,
            block_minutes_before: 30,
            block_minutes_after: 30,
            close_open_positions: false,
        },
        sizing: PositionSizing {
            rule: SizingRule::RiskPercentAtr {
                risk_percent: 1.0,
                atr_multiple: 1.5,
                atr_indicator: "atr".to_string(),
            },
            max_open_positions: 1,
        },
        trade_management: trade_management(),
        timing: ExecutionTiming {
            decision: DecisionTiming::ClosedBar,
            forming_bar_visible: false,
            submit_delay_bars: 0,
        },
    }
}

fn settings() -> ExecutionSettings {
    ExecutionSettings {
        initial_capital: 100_000.0,
        account_currency: "USD".to_string(),
        commission: CommissionModel::PerShare {
            amount: 0.005,
            minimum: 1.0,
        },
        slippage: SlippageModel::SpreadFraction { fraction: 0.5 },
        spread: SpreadModel::Constant { price_units: 0.01 },
        ambiguity: OhlcAmbiguityPolicy::StopFirst,
        tie_break: TieBreakPolicy::TimestampPrioritySequence,
        ..ExecutionSettings::conservative_defaults()
    }
}

/// A syntactically valid content-addressed id: 64 lowercase hex characters.
fn hex_id(fill: char) -> String {
    std::iter::repeat_n(fill, 64).collect()
}

fn binding() -> RunBinding {
    RunBinding {
        datasets: vec![
            DatasetBinding {
                input_id: "primary".to_string(),
                dataset_id: hex_id('a'),
            },
            DatasetBinding {
                input_id: "confirmation".to_string(),
                dataset_id: hex_id('b'),
            },
        ],
        strategy_id: hex_id('c'),
        config_id: hex_id('d'),
        seed: 42,
        engine_version: "typhoon-engine/0.1.0".to_string(),
        metrics_version: METRICS_SCHEMA_VERSION.to_string(),
        intervention_log_id: Some(hex_id('e')),
        repaint_qa: vec![],
    }
}

fn ir() -> StrategyIr {
    StrategyIr::build(&definition()).expect("fixture definition is valid")
}

fn strategy_id_of(definition: &StrategyDefinition) -> String {
    StrategyIr::build(definition)
        .expect("definition is valid")
        .strategy_id
}

fn config_id_of(settings: &ExecutionSettings) -> String {
    StrategyExecutionConfig::build(settings)
        .expect("settings are valid")
        .config_id
}

fn run_id_of(binding: &RunBinding) -> String {
    StrategyRunManifest::build(binding)
        .expect("binding is valid")
        .run_id
}

fn is_lowercase_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
}

// ── Identity: shape, repeatability, round trips ────────────────────

#[test]
fn strategy_id_is_repeatable_and_lowercase_sha256() {
    let first = StrategyIr::build(&definition()).expect("builds");
    let second = StrategyIr::build(&definition()).expect("builds");

    assert_eq!(first.strategy_id, second.strategy_id);
    assert_eq!(first, second);
    assert!(
        is_lowercase_sha256_hex(&first.strategy_id),
        "strategy id is not 64 lowercase hex chars: {}",
        first.strategy_id
    );
    assert_eq!(first.schema_version, STRATEGY_IR_SCHEMA_VERSION);
}

#[test]
fn config_id_is_repeatable_and_lowercase_sha256() {
    let first = StrategyExecutionConfig::build(&settings()).expect("builds");
    let second = StrategyExecutionConfig::build(&settings()).expect("builds");

    assert_eq!(first.config_id, second.config_id);
    assert_eq!(first, second);
    assert!(is_lowercase_sha256_hex(&first.config_id));
    assert_eq!(
        first.schema_version,
        STRATEGY_EXECUTION_CONFIG_SCHEMA_VERSION
    );
}

#[test]
fn run_id_is_repeatable_and_lowercase_sha256() {
    let first = StrategyRunManifest::build(&binding()).expect("builds");
    let second = StrategyRunManifest::build(&binding()).expect("builds");

    assert_eq!(first.run_id, second.run_id);
    assert_eq!(first, second);
    assert!(is_lowercase_sha256_hex(&first.run_id));
    assert_eq!(first.schema_version, STRATEGY_RUN_MANIFEST_SCHEMA_VERSION);
}

#[test]
fn the_three_identities_are_domain_separated() {
    let mut strategy = CanonicalDigest::new(STRATEGY_ID_DOMAIN);
    strategy.tagged_text("payload", "identical");
    let mut config = CanonicalDigest::new(CONFIG_ID_DOMAIN);
    config.tagged_text("payload", "identical");
    let mut run = CanonicalDigest::new(RUN_ID_DOMAIN);
    run.tagged_text("payload", "identical");
    let ids = BTreeSet::from([strategy.finish_hex(), config.finish_hex(), run.finish_hex()]);
    assert_eq!(ids.len(), 3);
}

#[test]
fn current_schema_identity_vectors_are_stable() {
    assert_eq!(
        strategy_id_of(&definition()),
        "026a44d4dbc84a67b49e65019ff18c7ce38a8fb9e26b258cd2230d6858ef33ed"
    );
    assert_eq!(
        config_id_of(&settings()),
        "cc27d29b253c9fd0b5cd7d8c8294021b76dd8a97083fc60c1128f32eaba09ce8"
    );
    assert_eq!(
        run_id_of(&binding()),
        "277cd003c1619a2373f9de3c772ff0205e3f640ebf732bd8c5b70548bc71f1b0"
    );
}

#[test]
fn strategy_ir_round_trips_through_serde() {
    let built = ir();
    let json = serde_json::to_string(&built).expect("serializes");
    let restored = StrategyIr::from_json_slice(json.as_bytes()).expect("deserializes");

    assert_eq!(built, restored);
    assert_eq!(built.strategy_id(), restored.strategy_id());
    restored.verify().expect("round-tripped IR still verifies");
}

#[test]
fn execution_config_round_trips_through_serde() {
    let built = StrategyExecutionConfig::build(&settings()).expect("builds");
    let json = serde_json::to_string(&built).expect("serializes");
    let restored = StrategyExecutionConfig::from_json_slice(json.as_bytes()).expect("deserializes");

    assert_eq!(built, restored);
    restored
        .verify()
        .expect("round-tripped config still verifies");
}

#[test]
fn run_manifest_round_trips_through_serde() {
    let built = StrategyRunManifest::build(&binding()).expect("builds");
    let json = serde_json::to_string(&built).expect("serializes");
    let restored = StrategyRunManifest::from_json_slice(json.as_bytes()).expect("deserializes");

    assert_eq!(built, restored);
    restored
        .verify()
        .expect("round-tripped manifest still verifies");
}

#[test]
fn build_verify_and_recompute_agree() {
    let built = ir();
    built.verify().expect("freshly built IR verifies");
    assert_eq!(
        built.recompute_strategy_id().expect("recomputes"),
        built.strategy_id
    );
    assert_eq!(strategy_id_of(&built.to_input()), built.strategy_id);

    let config = StrategyExecutionConfig::build(&settings()).expect("builds");
    config.verify().expect("freshly built config verifies");
    assert_eq!(
        config.recompute_config_id().expect("recomputes"),
        config.config_id
    );

    let manifest = StrategyRunManifest::build(&binding()).expect("builds");
    manifest.verify().expect("freshly built manifest verifies");
    assert_eq!(
        manifest.recompute_run_id().expect("recomputes"),
        manifest.run_id
    );
}

// ── Identity: per-field sensitivity ────────────────────────────────

/// One mutation for every identity-bearing StrategyDefinition field. Keeping
/// this as data makes both sensitivity and tamper tests exhaustive, and also
/// catches field-boundary framing bugs.
fn parameter_mut<'a>(
    definition: &'a mut StrategyDefinition,
    id: &str,
) -> &'a mut StrategyParameter {
    definition
        .parameters
        .iter_mut()
        .find(|parameter| parameter.id == id)
        .expect("fixture parameter exists")
}

fn definition_mutations() -> Vec<(&'static str, fn(&mut StrategyDefinition))> {
    vec![
        ("metadata.name", |d| d.metadata.name = "other name".into()),
        ("metadata.author", |d| d.metadata.author = "someone".into()),
        ("metadata.notes.some", |d| {
            d.metadata.notes = Some("different".into())
        }),
        ("metadata.notes.none", |d| d.metadata.notes = None),
        ("metadata.tags.push", |d| {
            d.metadata.tags.push("extra".into())
        }),
        ("metadata.tags.pop", |d| {
            d.metadata.tags.pop();
        }),
        ("parameters.int_value", |d| {
            parameter_mut(d, "atr_period").value = ParamValue::Int(21)
        }),
        ("parameters.float_value", |d| {
            parameter_mut(d, "risk_percent").value = ParamValue::Float(2.0)
        }),
        ("parameters.bool_value", |d| {
            parameter_mut(d, "allow_shorts").value = ParamValue::Bool(false)
        }),
        ("parameters.text_value", |d| {
            parameter_mut(d, "session_label").value = ParamValue::Text("tokyo".into())
        }),
        ("parameters.range_min", |d| {
            parameter_mut(d, "atr_period").range = Some(ParamRange::Int { min: 6, max: 50 })
        }),
        ("parameters.range_max", |d| {
            parameter_mut(d, "atr_period").range = Some(ParamRange::Int { min: 5, max: 60 })
        }),
        ("parameters.range_none", |d| {
            parameter_mut(d, "atr_period").range = None
        }),
        ("parameters.push", |d| {
            d.parameters.push(StrategyParameter {
                id: "extra_param".into(),
                value: ParamValue::Int(3),
                range: None,
            })
        }),
        ("indicators.kind", |d| {
            d.indicators[1].kind = IndicatorKind::Sma
        }),
        ("indicators.custom_name", |d| {
            d.indicators[2].kind = IndicatorKind::Custom {
                name: "other_custom".into(),
                implementation_id: CUSTOM_IMPLEMENTATION_ID.into(),
            }
        }),
        ("indicators.custom_implementation", |d| {
            d.indicators[2].kind = IndicatorKind::Custom {
                name: "waddah_attar".into(),
                implementation_id:
                    "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
            }
        }),
        ("indicators.input_constant", |d| {
            d.indicators[2].inputs[1] = IndicatorInput::Constant(3.0)
        }),
        ("indicators.input_push", |d| {
            d.indicators[2].inputs.push(IndicatorInput::Constant(4.0))
        }),
        ("indicators.input_price_field", |d| {
            d.indicators[1].inputs[0] = IndicatorInput::Price(PriceField::Open)
        }),
        ("roles.role", |d| {
            d.roles[2].role = IndicatorRole::Confirmation2
        }),
        ("roles.indicator", |d| {
            d.roles[2].indicator = "baseline".into()
        }),
        ("roles.pop", |d| {
            d.roles.pop();
        }),
        ("long.enabled", |d| d.long.enabled = false),
        ("long.entry", |d| {
            d.long.entry = Condition::Compare {
                left: close(0),
                op: CompareOp::GreaterOrEqual,
                right: Operand::Constant(1.0),
            }
        }),
        ("long.exit", |d| d.long.exit = Condition::Never),
        ("short.enabled", |d| d.short.enabled = false),
        ("short.entry", |d| d.short.entry = Condition::Always),
        ("short.exit", |d| d.short.exit = Condition::Never),
        ("session.enabled", |d| d.session.enabled = false),
        ("session.window_start", |d| {
            d.session.windows[0].start_minute = 500
        }),
        ("session.window_end", |d| {
            d.session.windows[0].end_minute = 1_000
        }),
        ("session.window_push", |d| {
            d.session.windows.push(SessionWindow {
                start_minute: 1_100,
                end_minute: 1_200,
            })
        }),
        ("session.close_outside", |d| {
            d.session.close_positions_outside = true
        }),
        ("news.enabled", |d| d.news.enabled = false),
        ("news.min_impact", |d| d.news.min_impact = NewsImpact::Low),
        ("news.before", |d| d.news.block_minutes_before = 45),
        ("news.after", |d| d.news.block_minutes_after = 45),
        ("news.close_open", |d| d.news.close_open_positions = true),
        ("sizing.rule_variant", |d| {
            d.sizing.rule = SizingRule::PercentEquity { percent: 2.0 }
        }),
        ("sizing.risk_percent", |d| {
            d.sizing.rule = SizingRule::RiskPercentAtr {
                risk_percent: 2.0,
                atr_multiple: 1.5,
                atr_indicator: "atr".into(),
            }
        }),
        ("sizing.atr_multiple", |d| {
            d.sizing.rule = SizingRule::RiskPercentAtr {
                risk_percent: 1.0,
                atr_multiple: 2.5,
                atr_indicator: "atr".into(),
            }
        }),
        ("sizing.atr_indicator", |d| {
            d.sizing.rule = SizingRule::RiskPercentAtr {
                risk_percent: 1.0,
                atr_multiple: 1.5,
                atr_indicator: "baseline".into(),
            }
        }),
        ("sizing.max_open", |d| d.sizing.max_open_positions = 2),
        ("trade.leg_fractions", |d| {
            d.trade_management.legs[0].fraction_bps = 3_000;
            d.trade_management.legs[1].fraction_bps = 7_000;
        }),
        ("trade.leg_stop", |d| {
            d.trade_management.legs[0].stop = Some(StopRule::PercentOfEntry { percent: 1.0 })
        }),
        ("trade.leg_target_none", |d| {
            d.trade_management.legs[0].target = None
        }),
        ("trade.leg_trailing_none", |d| {
            d.trade_management.legs[1].trailing = None
        }),
        ("trade.trailing_activate_none", |d| {
            d.trade_management.legs[1].trailing = Some(TrailingStop {
                distance: StopRule::AtrMultiple {
                    indicator: "atr".into(),
                    multiple: 1.5,
                },
                activate_after: None,
            })
        }),
        ("trade.break_even_none", |d| {
            d.trade_management.break_even_after = None
        }),
        ("trade.max_bars", |d| {
            d.trade_management.max_bars_in_trade = Some(100)
        }),
        ("trade.max_bars_none", |d| {
            d.trade_management.max_bars_in_trade = None
        }),
        ("timing.decision", |d| {
            d.timing.decision = DecisionTiming::NextBarOpen
        }),
        ("timing.pre_close", |d| {
            d.timing.decision = DecisionTiming::PreClose { offset_seconds: 30 }
        }),
        ("timing.forming_bar", |d| {
            d.timing.decision = DecisionTiming::PreClose { offset_seconds: 30 };
            d.timing.forming_bar_visible = true;
        }),
        ("timing.submit_delay", |d| d.timing.submit_delay_bars = 1),
    ]
}

#[test]
fn every_identity_bearing_definition_field_changes_the_strategy_id() {
    let baseline = strategy_id_of(&definition());
    let mut seen = BTreeSet::from([baseline.clone()]);

    for (label, mutate) in definition_mutations() {
        let mut mutated = definition();
        mutate(&mut mutated);
        let id = StrategyIr::build(&mutated)
            .unwrap_or_else(|e| panic!("mutation `{label}` produced an invalid definition: {e}"))
            .strategy_id;
        assert_ne!(id, baseline, "mutation `{label}` did not change the id");
        assert!(
            seen.insert(id),
            "mutation `{label}` collided with another mutation's id"
        );
    }
}

fn settings_mutations() -> Vec<(&'static str, fn(&mut ExecutionSettings))> {
    vec![
        ("initial_capital", |s| s.initial_capital = 50_000.0),
        ("account_currency", |s| s.account_currency = "EUR".into()),
        ("commission.none", |s| s.commission = CommissionModel::None),
        ("commission.amount", |s| {
            s.commission = CommissionModel::PerShare {
                amount: 0.01,
                minimum: 1.0,
            }
        }),
        ("commission.minimum", |s| {
            s.commission = CommissionModel::PerShare {
                amount: 0.005,
                minimum: 2.0,
            }
        }),
        ("commission.variant", |s| {
            s.commission = CommissionModel::PercentOfNotional {
                percent: 0.005,
                minimum: 1.0,
            }
        }),
        ("commission.per_order", |s| {
            s.commission = CommissionModel::PerOrder { amount: 0.005 }
        }),
        ("slippage.none", |s| s.slippage = SlippageModel::None),
        ("slippage.fraction", |s| {
            s.slippage = SlippageModel::SpreadFraction { fraction: 1.0 }
        }),
        ("slippage.variant", |s| {
            s.slippage = SlippageModel::FixedPriceDistance { distance: 0.5 }
        }),
        ("slippage.volatility", |s| {
            s.slippage = SlippageModel::VolatilityScaled { atr_fraction: 0.5 }
        }),
        ("spread.none", |s| s.spread = SpreadModel::None),
        ("spread.units", |s| {
            s.spread = SpreadModel::Constant { price_units: 0.02 }
        }),
        ("spread.variant", |s| {
            s.spread = SpreadModel::PercentOfPrice { percent: 0.01 }
        }),
        ("spread.recorded", |s| {
            s.spread = SpreadModel::RecordedQuotes
        }),
        ("ambiguity", |s| {
            s.ambiguity = OhlcAmbiguityPolicy::TargetFirst
        }),
        ("ambiguity.path", |s| {
            s.ambiguity = OhlcAmbiguityPolicy::OhlcPath
        }),
        ("tie_break", |s| {
            s.tie_break = TieBreakPolicy::TimestampPrioritySymbolSequence
        }),
    ]
}

#[test]
fn every_identity_bearing_settings_field_changes_the_config_id() {
    let baseline = config_id_of(&settings());
    let mut seen = BTreeSet::from([baseline.clone()]);

    for (label, mutate) in settings_mutations() {
        let mut mutated = settings();
        mutate(&mut mutated);
        let id = StrategyExecutionConfig::build(&mutated)
            .unwrap_or_else(|e| panic!("mutation `{label}` produced invalid settings: {e}"))
            .config_id;
        assert_ne!(id, baseline, "mutation `{label}` did not change the id");
        assert!(
            seen.insert(id),
            "mutation `{label}` collided with another mutation's id"
        );
    }
}

fn binding_mutations() -> Vec<(&'static str, fn(&mut RunBinding))> {
    vec![
        ("datasets.input_id", |b| {
            b.datasets[0].input_id = "secondary".to_string();
        }),
        ("datasets.dataset_id", |b| {
            b.datasets[0].dataset_id = hex_id('f');
        }),
        ("datasets.push", |b| {
            b.datasets.push(DatasetBinding {
                input_id: "external".to_string(),
                dataset_id: hex_id('0'),
            });
        }),
        ("datasets.pop", |b| {
            b.datasets.pop();
        }),
        ("strategy_id", |b| b.strategy_id = hex_id('1')),
        ("config_id", |b| b.config_id = hex_id('2')),
        ("seed", |b| b.seed = 43),
        ("seed.zero", |b| b.seed = 0),
        ("engine_version", |b| {
            b.engine_version = "typhoon-engine/0.2.0".into()
        }),
        ("intervention_log_id.some", |b| {
            b.intervention_log_id = Some(hex_id('3'))
        }),
        ("intervention_log_id.none", |b| b.intervention_log_id = None),
    ]
}

#[test]
fn every_identity_bearing_binding_field_changes_the_run_id() {
    let baseline = run_id_of(&binding());
    let mut seen = BTreeSet::from([baseline.clone()]);

    for (label, mutate) in binding_mutations() {
        let mut mutated = binding();
        mutate(&mut mutated);
        let id = StrategyRunManifest::build(&mutated)
            .unwrap_or_else(|e| panic!("mutation `{label}` produced an invalid binding: {e}"))
            .run_id;
        assert_ne!(id, baseline, "mutation `{label}` did not change the id");
        assert!(
            seen.insert(id),
            "mutation `{label}` collided with another mutation's id"
        );
    }
}

// ── Identity: canonical ordering ───────────────────────────────────

#[test]
fn parameter_declaration_order_does_not_change_the_strategy_id() {
    let mut swapped = definition();
    swapped.parameters.swap(0, 1);
    assert_eq!(strategy_id_of(&swapped), strategy_id_of(&definition()));
}

#[test]
fn indicator_declaration_order_does_not_change_the_strategy_id() {
    let mut swapped = definition();
    swapped.indicators.swap(0, 1);
    assert_eq!(strategy_id_of(&swapped), strategy_id_of(&definition()));
}

#[test]
fn indicator_input_order_changes_the_strategy_id() {
    let mut swapped = definition();
    swapped.indicators[2].inputs.swap(0, 1);
    assert_ne!(strategy_id_of(&swapped), strategy_id_of(&definition()));
}

#[test]
fn tag_order_does_not_change_the_strategy_id() {
    let mut swapped = definition();
    swapped.metadata.tags.swap(0, 1);
    assert_eq!(strategy_id_of(&swapped), strategy_id_of(&definition()));
}

#[test]
fn role_declaration_order_does_not_change_the_strategy_id() {
    let mut swapped = definition();
    swapped.roles.swap(0, 1);
    assert_eq!(strategy_id_of(&swapped), strategy_id_of(&definition()));
}

#[test]
fn commutative_condition_child_order_does_not_change_the_strategy_id() {
    let mut swapped = definition();
    match &mut swapped.long.entry {
        Condition::All(children) => children.swap(0, 1),
        other => panic!("fixture entry is not an All node: {other:?}"),
    }
    assert_eq!(strategy_id_of(&swapped), strategy_id_of(&definition()));
}

#[test]
fn build_stores_the_normalized_canonical_definition() {
    let mut shuffled = definition();
    shuffled.parameters.reverse();
    shuffled.indicators.reverse();
    shuffled.roles.reverse();
    shuffled.metadata.tags.reverse();
    if let Condition::All(children) = &mut shuffled.long.entry {
        children.reverse();
    }

    let built = StrategyIr::build(&shuffled).expect("shuffled definition is valid");
    let canonical = StrategyIr::build(&definition()).expect("fixture is valid");
    assert_eq!(built.definition, canonical.definition);
    assert_eq!(built.strategy_id, canonical.strategy_id);
}

#[test]
fn trade_leg_order_changes_the_strategy_id() {
    let mut swapped = definition();
    swapped.trade_management.legs.swap(0, 1);
    assert_ne!(strategy_id_of(&swapped), strategy_id_of(&definition()));
}

#[test]
fn dataset_binding_declaration_order_does_not_change_the_run_id() {
    let mut swapped = binding();
    swapped.datasets.swap(0, 1);
    assert_eq!(run_id_of(&swapped), run_id_of(&binding()));
}

#[test]
fn repaint_qa_declaration_order_does_not_change_the_run_id() {
    let mut canonical = binding();
    canonical.repaint_qa = vec![
        RepaintQaBinding {
            indicator_id: hex_id('1'),
            artifact_id: hex_id('a'),
            acknowledgement: RepaintAcknowledgement::Clean,
        },
        RepaintQaBinding {
            indicator_id: hex_id('2'),
            artifact_id: hex_id('b'),
            acknowledgement: RepaintAcknowledgement::WarningAcknowledged {
                note: "reviewed exact evidence".to_string(),
            },
        },
    ];
    let mut swapped = canonical.clone();
    swapped.repaint_qa.swap(0, 1);

    assert_eq!(run_id_of(&swapped), run_id_of(&canonical));
    assert_eq!(
        StrategyRunManifest::build(&swapped)
            .expect("manifest")
            .binding()
            .repaint_qa,
        canonical.repaint_qa
    );
}

// ── Identity: framing collision resistance ─────────────────────────

#[test]
fn adjacent_text_fields_cannot_be_reframed() {
    // ("ab", "c") and ("a", "bc") concatenate to the same bytes; length
    // framing must keep them apart.
    let mut left = definition();
    left.metadata.name = "ab".to_string();
    left.metadata.author = "c".to_string();

    let mut right = definition();
    right.metadata.name = "a".to_string();
    right.metadata.author = "bc".to_string();

    assert_ne!(strategy_id_of(&left), strategy_id_of(&right));
}

#[test]
fn tag_boundaries_cannot_be_reframed() {
    let mut left = definition();
    left.metadata.tags = vec!["ab".to_string(), "c".to_string()];

    let mut right = definition();
    right.metadata.tags = vec!["a".to_string(), "bc".to_string()];

    let mut single = definition();
    single.metadata.tags = vec!["abc".to_string()];

    let ids = BTreeSet::from([
        strategy_id_of(&left),
        strategy_id_of(&right),
        strategy_id_of(&single),
    ]);
    assert_eq!(ids.len(), 3);
}

#[test]
fn nested_condition_shape_changes_the_strategy_id() {
    // All[A, All[B, C]] must not hash like All[A, B, C].
    let a = Condition::Compare {
        left: close(0),
        op: CompareOp::Greater,
        right: Operand::Constant(1.0),
    };
    let b = Condition::Compare {
        left: close(1),
        op: CompareOp::Greater,
        right: Operand::Constant(2.0),
    };
    let c = Condition::Compare {
        left: close(2),
        op: CompareOp::Greater,
        right: Operand::Constant(3.0),
    };

    let mut flat = definition();
    flat.long.entry = Condition::All(vec![a.clone(), b.clone(), c.clone()]);

    let mut nested = definition();
    nested.long.entry = Condition::All(vec![a, Condition::All(vec![b, c])]);

    assert_ne!(strategy_id_of(&flat), strategy_id_of(&nested));
}

#[test]
fn combinator_variants_do_not_collide() {
    let children = vec![
        Condition::Compare {
            left: close(0),
            op: CompareOp::Greater,
            right: Operand::Constant(1.0),
        },
        Condition::Compare {
            left: close(1),
            op: CompareOp::Less,
            right: Operand::Constant(2.0),
        },
    ];

    let mut all = definition();
    all.long.entry = Condition::All(children.clone());

    let mut any = definition();
    any.long.entry = Condition::Any(children.clone());

    let mut not_all = definition();
    not_all.long.entry = Condition::Not(Box::new(Condition::All(children)));

    let ids = BTreeSet::from([
        strategy_id_of(&all),
        strategy_id_of(&any),
        strategy_id_of(&not_all),
    ]);
    assert_eq!(ids.len(), 3);
}

#[test]
fn comparison_operands_are_position_framed() {
    let mut forward = definition();
    forward.long.entry = Condition::Compare {
        left: close(0),
        op: CompareOp::Greater,
        right: indicator_operand("baseline", 0),
    };

    let mut reversed = definition();
    reversed.long.entry = Condition::Compare {
        left: indicator_operand("baseline", 0),
        op: CompareOp::Greater,
        right: close(0),
    };

    assert_ne!(strategy_id_of(&forward), strategy_id_of(&reversed));
}

#[test]
fn custom_indicator_name_cannot_collide_with_a_builtin_kind() {
    let mut custom = definition();
    custom.indicators[1].kind = IndicatorKind::Custom {
        name: "ema".to_string(),
        implementation_id: CUSTOM_IMPLEMENTATION_ID.to_string(),
    };
    assert_ne!(strategy_id_of(&custom), strategy_id_of(&definition()));
}

#[test]
fn custom_indicator_implementation_is_identity_bearing_and_must_be_content_addressed() {
    let mut changed = definition();
    if let IndicatorKind::Custom {
        implementation_id, ..
    } = &mut changed.indicators[2].kind
    {
        *implementation_id =
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into();
    } else {
        panic!("fixture confirmation is not custom");
    }
    assert_ne!(strategy_id_of(&changed), strategy_id_of(&definition()));

    if let IndicatorKind::Custom {
        implementation_id, ..
    } = &mut changed.indicators[2].kind
    {
        *implementation_id = "human-version-name".into();
    }
    assert!(matches!(
        StrategyIr::build(&changed),
        Err(StrategyIrError::MalformedDigestId { .. })
    ));
}

#[test]
fn built_in_indicators_reject_wrong_input_arity_and_shape() {
    let mut missing_period = definition();
    missing_period.indicators[1].inputs.pop();
    assert!(matches!(
        StrategyIr::build(&missing_period),
        Err(StrategyIrError::OutOfRange { ref field, .. })
            if field.contains("indicators[1].inputs")
    ));

    let mut swapped_shape = definition();
    swapped_shape.indicators[1].inputs.swap(0, 1);
    assert!(matches!(
        StrategyIr::build(&swapped_shape),
        Err(StrategyIrError::OutOfRange { ref field, .. })
            if field.contains("indicators[1].inputs[0]")
    ));
}

#[test]
fn numeric_expressions_reject_boolean_and_text_parameters() {
    for parameter in ["allow_shorts", "session_label"] {
        let mut condition = definition();
        condition.long.entry = Condition::Compare {
            left: Operand::Parameter(parameter.into()),
            op: CompareOp::Equal,
            right: Operand::Constant(1.0),
        };
        assert!(matches!(
            StrategyIr::build(&condition),
            Err(StrategyIrError::OutOfRange { ref field, .. })
                if field.contains("long.entry")
        ));

        let mut indicator = definition();
        indicator.indicators[0].inputs[0] = IndicatorInput::Parameter(parameter.into());
        assert!(matches!(
            StrategyIr::build(&indicator),
            Err(StrategyIrError::OutOfRange { ref field, .. })
                if field.contains("indicator `atr`")
        ));
    }
}

#[test]
fn operand_variants_do_not_collide_on_shared_payloads() {
    let mut as_indicator = definition();
    as_indicator.long.entry = Condition::Compare {
        left: indicator_operand("baseline", 0),
        op: CompareOp::Greater,
        right: Operand::Constant(0.0),
    };

    let mut as_parameter = definition();
    as_parameter.long.entry = Condition::Compare {
        left: Operand::Parameter("baseline_period".to_string()),
        op: CompareOp::Greater,
        right: Operand::Constant(0.0),
    };

    let mut bars_ago = definition();
    bars_ago.long.entry = Condition::Compare {
        left: indicator_operand("baseline", 1),
        op: CompareOp::Greater,
        right: Operand::Constant(0.0),
    };

    let ids = BTreeSet::from([
        strategy_id_of(&as_indicator),
        strategy_id_of(&as_parameter),
        strategy_id_of(&bars_ago),
    ]);
    assert_eq!(ids.len(), 3);
}

#[test]
fn optional_field_presence_is_framed() {
    let mut none = definition();
    none.trade_management.max_bars_in_trade = None;

    let mut zeroish = definition();
    zeroish.trade_management.max_bars_in_trade = Some(1);

    assert_ne!(strategy_id_of(&none), strategy_id_of(&zeroish));
}

// ── Identity: float encoding ───────────────────────────────────────

#[test]
fn negative_zero_hashes_as_positive_zero() {
    let mut positive = definition();
    positive.long.entry = Condition::Compare {
        left: close(0),
        op: CompareOp::Greater,
        right: Operand::Constant(0.0),
    };

    let mut negative = definition();
    negative.long.entry = Condition::Compare {
        left: close(0),
        op: CompareOp::Greater,
        right: Operand::Constant(-0.0),
    };

    assert_eq!(strategy_id_of(&positive), strategy_id_of(&negative));
}

#[test]
fn negative_zero_is_normalized_in_parameters_and_settings() {
    let mut positive_param = definition();
    positive_param.parameters[2].value = ParamValue::Float(0.0);
    positive_param.parameters[2].range = Some(ParamRange::Float { min: 0.0, max: 1.0 });

    let mut negative_param = definition();
    negative_param.parameters[2].value = ParamValue::Float(-0.0);
    negative_param.parameters[2].range = Some(ParamRange::Float {
        min: -0.0,
        max: 1.0,
    });

    assert_eq!(
        strategy_id_of(&positive_param),
        strategy_id_of(&negative_param)
    );

    let mut positive_settings = settings();
    positive_settings.commission = CommissionModel::PerOrder { amount: 0.0 };
    let mut negative_settings = settings();
    negative_settings.commission = CommissionModel::PerOrder { amount: -0.0 };

    assert_eq!(
        config_id_of(&positive_settings),
        config_id_of(&negative_settings)
    );
}

#[test]
fn neighbouring_floats_are_distinguished_by_bits_not_by_formatting() {
    let mut base = definition();
    base.sizing.rule = SizingRule::PercentEquity { percent: 1.0 };

    let mut nudged = definition();
    nudged.sizing.rule = SizingRule::PercentEquity {
        percent: 1.0 + f64::EPSILON,
    };

    assert_ne!(strategy_id_of(&base), strategy_id_of(&nudged));
}

#[test]
fn non_finite_parameter_values_are_rejected() {
    for (value, kind) in [
        (f64::NAN, NonFiniteKind::Nan),
        (f64::INFINITY, NonFiniteKind::PositiveInfinity),
        (f64::NEG_INFINITY, NonFiniteKind::NegativeInfinity),
    ] {
        let mut invalid = definition();
        invalid.parameters[2].value = ParamValue::Float(value);
        invalid.parameters[2].range = None;
        match StrategyIr::build(&invalid) {
            Err(StrategyIrError::NonFiniteValue { kind: found, .. }) => assert_eq!(found, kind),
            other => panic!("expected a non-finite rejection, got {other:?}"),
        }
    }
}

#[test]
fn non_finite_values_are_rejected_everywhere_they_can_appear() {
    let mut condition_constant = definition();
    condition_constant.long.entry = Condition::Compare {
        left: close(0),
        op: CompareOp::Greater,
        right: Operand::Constant(f64::NAN),
    };

    let mut indicator_constant = definition();
    indicator_constant.indicators[2].inputs[1] = IndicatorInput::Constant(f64::INFINITY);

    let mut range_bound = definition();
    range_bound.parameters[2].range = Some(ParamRange::Float {
        min: f64::NEG_INFINITY,
        max: 5.0,
    });

    let mut stop_multiple = definition();
    stop_multiple.trade_management.legs[0].stop = Some(StopRule::AtrMultiple {
        indicator: "atr".to_string(),
        multiple: f64::NAN,
    });

    let mut sizing_value = definition();
    sizing_value.sizing.rule = SizingRule::PercentEquity {
        percent: f64::INFINITY,
    };

    for (label, candidate) in [
        ("condition constant", condition_constant),
        ("indicator constant", indicator_constant),
        ("parameter range bound", range_bound),
        ("stop multiple", stop_multiple),
        ("sizing percent", sizing_value),
    ] {
        assert!(
            matches!(
                StrategyIr::build(&candidate),
                Err(StrategyIrError::NonFiniteValue { .. })
            ),
            "{label} was not rejected as non-finite"
        );
    }
}

#[test]
fn non_finite_settings_values_are_rejected() {
    let mut capital = settings();
    capital.initial_capital = f64::NAN;

    let mut commission = settings();
    commission.commission = CommissionModel::PerShare {
        amount: f64::INFINITY,
        minimum: 1.0,
    };

    let mut slippage = settings();
    slippage.slippage = SlippageModel::SpreadFraction { fraction: f64::NAN };

    let mut spread = settings();
    spread.spread = SpreadModel::Constant {
        price_units: f64::NEG_INFINITY,
    };

    for (label, candidate) in [
        ("initial capital", capital),
        ("commission", commission),
        ("slippage", slippage),
        ("spread", spread),
    ] {
        assert!(
            matches!(
                StrategyExecutionConfig::build(&candidate),
                Err(StrategyIrError::NonFiniteValue { .. })
            ),
            "{label} was not rejected as non-finite"
        );
    }
}

// ── Validation: identifiers, duplicates, references ────────────────

#[test]
fn blank_and_control_padded_text_is_rejected() {
    for (label, value, reason) in [
        ("empty", "", InvalidTextReason::Empty),
        ("whitespace only", "   ", InvalidTextReason::Empty),
        (
            "leading space",
            " name",
            InvalidTextReason::SurroundingWhitespace,
        ),
        (
            "trailing space",
            "name ",
            InvalidTextReason::SurroundingWhitespace,
        ),
        (
            "control char",
            "na\u{7}me",
            InvalidTextReason::ControlCharacter,
        ),
        ("newline", "na\nme", InvalidTextReason::ControlCharacter),
    ] {
        let mut invalid = definition();
        invalid.metadata.name = value.to_string();
        match StrategyIr::build(&invalid) {
            Err(StrategyIrError::InvalidText { reason: found, .. }) => {
                assert_eq!(found, reason, "wrong reason for {label}")
            }
            other => panic!("`{label}` was not rejected: {other:?}"),
        }
    }
}

#[test]
fn overlong_text_is_rejected() {
    let mut invalid = definition();
    invalid.metadata.name = "n".repeat(MAX_TEXT_LEN + 1);
    assert!(matches!(
        StrategyIr::build(&invalid),
        Err(StrategyIrError::InvalidText {
            reason: InvalidTextReason::TooLong,
            ..
        })
    ));
}

#[test]
fn optional_notes_are_validated_like_required_text() {
    let mut invalid = definition();
    invalid.metadata.notes = Some("  padded  ".to_string());
    assert!(matches!(
        StrategyIr::build(&invalid),
        Err(StrategyIrError::InvalidText { .. })
    ));
}

#[test]
fn malformed_stable_ids_are_rejected() {
    for (label, id, reason) in [
        ("empty", String::new(), InvalidIdReason::Empty),
        (
            "uppercase",
            "AtrPeriod".to_string(),
            InvalidIdReason::IllegalCharacter,
        ),
        (
            "space",
            "atr period".to_string(),
            InvalidIdReason::IllegalCharacter,
        ),
        (
            "leading digit",
            "1atr".to_string(),
            InvalidIdReason::LeadingNonLetter,
        ),
        (
            "too long",
            "a".repeat(MAX_STABLE_ID_LEN + 1),
            InvalidIdReason::TooLong,
        ),
    ] {
        let mut invalid = definition();
        invalid.parameters[0].id = id;
        // The indicator that referenced the old id now dangles; drop it so the
        // id check is what fails.
        invalid.indicators[0].inputs.clear();
        match StrategyIr::build(&invalid) {
            Err(StrategyIrError::InvalidId { reason: found, .. }) => {
                assert_eq!(found, reason, "wrong reason for {label}")
            }
            other => panic!("`{label}` was not rejected: {other:?}"),
        }
    }
}

#[test]
fn duplicate_parameter_ids_are_rejected() {
    let mut invalid = definition();
    invalid.parameters[1].id = "atr_period".to_string();
    match StrategyIr::build(&invalid) {
        Err(StrategyIrError::DuplicateId { kind, id }) => {
            assert_eq!(kind, RefKind::Parameter);
            assert_eq!(id, "atr_period");
        }
        other => panic!("duplicate parameter id was not rejected: {other:?}"),
    }
}

#[test]
fn duplicate_indicator_ids_are_rejected() {
    let mut invalid = definition();
    invalid.indicators[1].id = "atr".to_string();
    match StrategyIr::build(&invalid) {
        Err(StrategyIrError::DuplicateId { kind, id }) => {
            assert_eq!(kind, RefKind::Indicator);
            assert_eq!(id, "atr");
        }
        other => panic!("duplicate indicator id was not rejected: {other:?}"),
    }
}

#[test]
fn duplicate_roles_are_rejected() {
    let mut invalid = definition();
    invalid.roles[1].role = IndicatorRole::Atr;
    match StrategyIr::build(&invalid) {
        Err(StrategyIrError::DuplicateRole { role }) => assert_eq!(role, IndicatorRole::Atr),
        other => panic!("duplicate role was not rejected: {other:?}"),
    }
}

#[test]
fn unknown_references_are_rejected_wherever_they_appear() {
    let mut condition_indicator = definition();
    condition_indicator.long.exit = Condition::CrossesBelow {
        left: close(0),
        right: indicator_operand("missing", 0),
    };

    let mut condition_parameter = definition();
    condition_parameter.short.entry = Condition::Compare {
        left: Operand::Parameter("missing".to_string()),
        op: CompareOp::Greater,
        right: Operand::Constant(0.0),
    };

    let mut indicator_input_parameter = definition();
    indicator_input_parameter.indicators[0].inputs =
        vec![IndicatorInput::Parameter("missing".to_string())];

    let mut indicator_input_indicator = definition();
    indicator_input_indicator.indicators[2].inputs[0] =
        IndicatorInput::Indicator("missing".to_string());

    let mut role_target = definition();
    role_target.roles[0].indicator = "missing".to_string();

    let mut sizing_target = definition();
    sizing_target.sizing.rule = SizingRule::RiskPercentAtr {
        risk_percent: 1.0,
        atr_multiple: 1.5,
        atr_indicator: "missing".to_string(),
    };

    let mut stop_target = definition();
    stop_target.trade_management.legs[0].stop = Some(StopRule::AtrMultiple {
        indicator: "missing".to_string(),
        multiple: 1.5,
    });

    let mut trailing_target = definition();
    trailing_target.trade_management.legs[1].trailing = Some(TrailingStop {
        distance: StopRule::AtrMultiple {
            indicator: "missing".to_string(),
            multiple: 1.5,
        },
        activate_after: None,
    });

    let mut break_even_target = definition();
    break_even_target.trade_management.break_even_after = Some(StopRule::AtrMultiple {
        indicator: "missing".to_string(),
        multiple: 1.0,
    });

    for (label, expected_kind, candidate) in [
        (
            "condition indicator",
            RefKind::Indicator,
            condition_indicator,
        ),
        (
            "condition parameter",
            RefKind::Parameter,
            condition_parameter,
        ),
        (
            "indicator input parameter",
            RefKind::Parameter,
            indicator_input_parameter,
        ),
        (
            "indicator input indicator",
            RefKind::Indicator,
            indicator_input_indicator,
        ),
        ("role target", RefKind::Indicator, role_target),
        ("sizing target", RefKind::Indicator, sizing_target),
        ("stop target", RefKind::Indicator, stop_target),
        ("trailing target", RefKind::Indicator, trailing_target),
        ("break-even target", RefKind::Indicator, break_even_target),
    ] {
        match StrategyIr::build(&candidate) {
            Err(StrategyIrError::UnknownRef { kind, id, .. }) => {
                assert_eq!(kind, expected_kind, "wrong ref kind for {label}");
                assert_eq!(id, "missing", "wrong id for {label}");
            }
            other => panic!("`{label}` did not report an unknown ref: {other:?}"),
        }
    }
}

#[test]
fn indicator_self_reference_is_a_cycle() {
    let mut invalid = definition();
    invalid.indicators[2].inputs = vec![IndicatorInput::Indicator("confirmation".to_string())];
    match StrategyIr::build(&invalid) {
        Err(StrategyIrError::IndicatorCycle { path }) => {
            assert!(
                path.first() == Some(&"confirmation".to_string())
                    && path.last() == Some(&"confirmation".to_string()),
                "cycle path does not close on itself: {path:?}"
            );
        }
        other => panic!("self reference was not reported as a cycle: {other:?}"),
    }
}

#[test]
fn indirect_indicator_cycles_are_rejected() {
    let mut invalid = definition();
    // baseline -> confirmation -> baseline
    invalid.indicators[1].inputs[0] = IndicatorInput::Indicator("confirmation".to_string());
    match StrategyIr::build(&invalid) {
        Err(StrategyIrError::IndicatorCycle { path }) => {
            assert!(path.len() >= 3, "cycle path is too short: {path:?}");
            assert_eq!(path.first(), path.last());
        }
        other => panic!("indirect cycle was not rejected: {other:?}"),
    }
}

#[test]
fn a_forward_reference_is_not_a_cycle() {
    let mut forward = definition();
    // Move the custom confirmation node ahead of the baseline it reads:
    // declaration order must not matter.
    forward.indicators.swap(0, 2);
    StrategyIr::build(&forward).expect("forward references are a valid DAG");
}

// ── Validation: recursion bounds ───────────────────────────────────

fn nested_not(depth: usize) -> Condition {
    let mut condition = Condition::Always;
    for _ in 0..depth {
        condition = Condition::Not(Box::new(condition));
    }
    condition
}

#[test]
fn condition_depth_limit_is_enforced() {
    let mut at_limit = definition();
    at_limit.long.entry = nested_not(MAX_CONDITION_DEPTH - 1);
    StrategyIr::build(&at_limit).expect("a tree exactly at the depth limit is accepted");

    let mut over_limit = definition();
    over_limit.long.entry = nested_not(MAX_CONDITION_DEPTH);
    match StrategyIr::build(&over_limit) {
        Err(StrategyIrError::ConditionTooDeep { limit, found }) => {
            assert_eq!(limit, MAX_CONDITION_DEPTH);
            assert_eq!(found, MAX_CONDITION_DEPTH + 1);
        }
        other => panic!("over-deep condition was not rejected: {other:?}"),
    }
}

#[test]
fn condition_node_limit_is_enforced() {
    let mut at_limit = definition();
    at_limit.long.entry = Condition::All(vec![Condition::Always; MAX_CONDITION_NODES - 1]);
    StrategyIr::build(&at_limit).expect("a tree exactly at the node limit is accepted");

    let mut over_limit = definition();
    over_limit.long.entry = Condition::All(vec![Condition::Always; MAX_CONDITION_NODES]);
    match StrategyIr::build(&over_limit) {
        Err(StrategyIrError::ConditionTooLarge { limit, found }) => {
            assert_eq!(limit, MAX_CONDITION_NODES);
            assert_eq!(found, MAX_CONDITION_NODES + 1);
        }
        other => panic!("over-large condition was not rejected: {other:?}"),
    }
}

#[test]
fn every_condition_slot_is_bounded() {
    for (label, apply) in [
        (
            "long.exit",
            (|d: &mut StrategyDefinition| d.long.exit = nested_not(MAX_CONDITION_DEPTH))
                as fn(&mut StrategyDefinition),
        ),
        ("short.entry", |d: &mut StrategyDefinition| {
            d.short.entry = nested_not(MAX_CONDITION_DEPTH)
        }),
        ("short.exit", |d: &mut StrategyDefinition| {
            d.short.exit = nested_not(MAX_CONDITION_DEPTH)
        }),
    ] {
        let mut invalid = definition();
        apply(&mut invalid);
        assert!(
            matches!(
                StrategyIr::build(&invalid),
                Err(StrategyIrError::ConditionTooDeep { .. })
            ),
            "{label} was not depth-checked"
        );
    }
}

#[test]
fn empty_combinators_are_rejected() {
    for empty in [Condition::All(Vec::new()), Condition::Any(Vec::new())] {
        let mut invalid = definition();
        invalid.long.entry = empty;
        assert!(
            matches!(
                StrategyIr::build(&invalid),
                Err(StrategyIrError::OutOfRange { .. })
            ),
            "an empty combinator was accepted"
        );
    }
}

#[test]
fn bars_ago_is_bounded() {
    let mut at_limit = definition();
    at_limit.long.entry = Condition::Compare {
        left: close(MAX_BARS_AGO),
        op: CompareOp::Greater,
        right: Operand::Constant(0.0),
    };
    StrategyIr::build(&at_limit).expect("bars_ago at the limit is accepted");

    let mut over_limit = definition();
    over_limit.long.entry = Condition::Compare {
        left: close(MAX_BARS_AGO + 1),
        op: CompareOp::Greater,
        right: Operand::Constant(0.0),
    };
    assert!(matches!(
        StrategyIr::build(&over_limit),
        Err(StrategyIrError::OutOfRange { .. })
    ));
}

#[test]
fn collection_sizes_are_bounded() {
    let mut too_many_parameters = definition();
    too_many_parameters.parameters = (0..=MAX_PARAMETERS)
        .map(|i| StrategyParameter {
            id: format!("p_{i}"),
            value: ParamValue::Int(i as i64),
            range: None,
        })
        .collect();
    too_many_parameters.indicators[0].inputs.clear();
    too_many_parameters.indicators[1].inputs = vec![IndicatorInput::Price(PriceField::Close)];

    let mut too_many_tags = definition();
    too_many_tags.metadata.tags = (0..=MAX_TAGS).map(|i| format!("t_{i}")).collect();

    let mut too_many_windows = definition();
    too_many_windows.session.windows = (0..=MAX_SESSION_WINDOWS as u32)
        .map(|i| SessionWindow {
            start_minute: i * 2,
            end_minute: i * 2 + 1,
        })
        .collect();

    for (label, candidate) in [
        ("parameters", too_many_parameters),
        ("tags", too_many_tags),
        ("session windows", too_many_windows),
    ] {
        assert!(
            matches!(
                StrategyIr::build(&candidate),
                Err(StrategyIrError::TooMany { .. })
            ),
            "{label} was not size-checked"
        );
    }
}

// ── Validation: ranges and semantic consistency ────────────────────

#[test]
fn invalid_parameter_ranges_are_rejected() {
    let mut inverted_int = definition();
    inverted_int.parameters[0].range = Some(ParamRange::Int { min: 50, max: 5 });

    let mut inverted_float = definition();
    inverted_float.parameters[2].range = Some(ParamRange::Float { min: 5.0, max: 0.1 });

    let mut value_below = definition();
    value_below.parameters[0].range = Some(ParamRange::Int { min: 20, max: 50 });

    let mut value_above = definition();
    value_above.parameters[2].range = Some(ParamRange::Float { min: 0.1, max: 0.5 });

    let mut type_mismatch = definition();
    type_mismatch.parameters[3].range = Some(ParamRange::Int { min: 0, max: 1 });

    for (label, candidate) in [
        ("inverted int range", inverted_int),
        ("inverted float range", inverted_float),
        ("value below range", value_below),
        ("value above range", value_above),
        ("range on a bool parameter", type_mismatch),
    ] {
        assert!(
            matches!(
                StrategyIr::build(&candidate),
                Err(StrategyIrError::OutOfRange { .. })
            ),
            "{label} was accepted"
        );
    }
}

#[test]
fn invalid_session_windows_are_rejected() {
    let mut inverted = definition();
    inverted.session.windows = vec![SessionWindow {
        start_minute: 1_020,
        end_minute: 480,
    }];

    let mut past_midnight = definition();
    past_midnight.session.windows = vec![SessionWindow {
        start_minute: 480,
        end_minute: 1_441,
    }];

    let mut empty_when_enabled = definition();
    empty_when_enabled.session.windows = Vec::new();

    let mut overlapping = definition();
    overlapping.session.windows = vec![
        SessionWindow {
            start_minute: 480,
            end_minute: 1_020,
        },
        SessionWindow {
            start_minute: 1_000,
            end_minute: 1_100,
        },
    ];

    let mut unordered = definition();
    unordered.session.windows = vec![
        SessionWindow {
            start_minute: 1_100,
            end_minute: 1_200,
        },
        SessionWindow {
            start_minute: 480,
            end_minute: 1_020,
        },
    ];

    for (label, candidate) in [
        ("inverted window", inverted),
        ("window past midnight", past_midnight),
        ("no windows while enabled", empty_when_enabled),
        ("overlapping windows", overlapping),
        ("unordered windows", unordered),
    ] {
        assert!(
            matches!(
                StrategyIr::build(&candidate),
                Err(StrategyIrError::OutOfRange { .. })
            ),
            "{label} was accepted"
        );
    }
}

#[test]
fn a_disabled_session_filter_may_be_empty() {
    let mut disabled = definition();
    disabled.session.enabled = false;
    disabled.session.windows = Vec::new();
    StrategyIr::build(&disabled).expect("a disabled session filter needs no windows");
}

#[test]
fn invalid_news_filter_windows_are_rejected() {
    let mut invalid = definition();
    invalid.news.block_minutes_before = MAX_NEWS_BLOCK_MINUTES + 1;
    assert!(matches!(
        StrategyIr::build(&invalid),
        Err(StrategyIrError::OutOfRange { .. })
    ));

    let mut zero_window = definition();
    zero_window.news.block_minutes_before = 0;
    zero_window.news.block_minutes_after = 0;
    assert!(
        matches!(
            StrategyIr::build(&zero_window),
            Err(StrategyIrError::OutOfRange { .. })
        ),
        "an enabled news filter that blocks nothing was accepted"
    );
}

#[test]
fn invalid_sizing_is_rejected() {
    let mut zero_percent = definition();
    zero_percent.sizing.rule = SizingRule::PercentEquity { percent: 0.0 };

    let mut over_percent = definition();
    over_percent.sizing.rule = SizingRule::PercentEquity { percent: 100.1 };

    let mut negative_units = definition();
    negative_units.sizing.rule = SizingRule::FixedUnits { units: -1.0 };

    let mut zero_positions = definition();
    zero_positions.sizing.max_open_positions = 0;

    let mut excessive_positions = definition();
    excessive_positions.sizing.max_open_positions = MAX_OPEN_POSITIONS + 1;

    let mut negative_multiple = definition();
    negative_multiple.sizing.rule = SizingRule::RiskPercentAtr {
        risk_percent: 1.0,
        atr_multiple: -1.5,
        atr_indicator: "atr".to_string(),
    };

    for (label, candidate) in [
        ("zero percent equity", zero_percent),
        ("percent above 100", over_percent),
        ("negative units", negative_units),
        ("zero max open positions", zero_positions),
        ("excessive max open positions", excessive_positions),
        ("negative atr multiple", negative_multiple),
    ] {
        assert!(
            matches!(
                StrategyIr::build(&candidate),
                Err(StrategyIrError::OutOfRange { .. })
            ),
            "{label} was accepted"
        );
    }
}

#[test]
fn invalid_trade_legs_are_rejected() {
    let mut no_legs = definition();
    no_legs.trade_management.legs = Vec::new();

    let mut short_total = definition();
    short_total.trade_management.legs[0].fraction_bps = 4_000;

    let mut long_total = definition();
    long_total.trade_management.legs[0].fraction_bps = 6_000;

    let mut zero_leg = definition();
    zero_leg.trade_management.legs[0].fraction_bps = 0;
    zero_leg.trade_management.legs[1].fraction_bps = 10_000;

    let mut zero_bars = definition();
    zero_bars.trade_management.max_bars_in_trade = Some(0);

    let mut excessive_bars = definition();
    excessive_bars.trade_management.max_bars_in_trade = Some(MAX_BARS_IN_TRADE + 1);

    for (label, candidate) in [
        ("no legs", no_legs),
        ("fractions below 100%", short_total),
        ("fractions above 100%", long_total),
        ("empty leg", zero_leg),
        ("zero max bars in trade", zero_bars),
        ("excessive max bars in trade", excessive_bars),
    ] {
        assert!(
            matches!(
                StrategyIr::build(&candidate),
                Err(StrategyIrError::OutOfRange { .. })
            ),
            "{label} was accepted"
        );
    }
}

#[test]
fn a_strategy_that_can_never_enter_is_rejected() {
    let mut invalid = definition();
    invalid.long.enabled = false;
    invalid.short.enabled = false;
    assert!(matches!(
        StrategyIr::build(&invalid),
        Err(StrategyIrError::NoEnabledDirection)
    ));
}

#[test]
fn disabled_direction_rules_are_still_validated() {
    let mut invalid = definition();
    invalid.short.enabled = false;
    invalid.short.entry = Condition::CrossesBelow {
        left: close(0),
        right: indicator_operand("missing", 0),
    };
    assert!(
        matches!(
            StrategyIr::build(&invalid),
            Err(StrategyIrError::UnknownRef { .. })
        ),
        "a dangling reference in a disabled direction was not caught"
    );
}

#[test]
fn forming_bar_visibility_requires_pre_close_timing() {
    for decision in [DecisionTiming::ClosedBar, DecisionTiming::NextBarOpen] {
        let mut invalid = definition();
        invalid.timing.decision = decision;
        invalid.timing.forming_bar_visible = true;
        assert!(
            matches!(
                StrategyIr::build(&invalid),
                Err(StrategyIrError::InconsistentTiming { .. })
            ),
            "forming-bar visibility was accepted outside a pre-close decision"
        );
    }

    let mut valid = definition();
    valid.timing.decision = DecisionTiming::PreClose { offset_seconds: 30 };
    valid.timing.forming_bar_visible = true;
    StrategyIr::build(&valid).expect("a pre-close rule may see the forming bar");
}

#[test]
fn pre_close_offsets_and_submit_delays_are_bounded() {
    let mut zero_offset = definition();
    zero_offset.timing.decision = DecisionTiming::PreClose { offset_seconds: 0 };

    let mut huge_offset = definition();
    huge_offset.timing.decision = DecisionTiming::PreClose {
        offset_seconds: MAX_PRE_CLOSE_OFFSET_SECONDS + 1,
    };

    let mut huge_delay = definition();
    huge_delay.timing.submit_delay_bars = MAX_SUBMIT_DELAY_BARS + 1;

    for (label, candidate) in [
        ("zero pre-close offset", zero_offset),
        ("oversized pre-close offset", huge_offset),
        ("oversized submit delay", huge_delay),
    ] {
        assert!(
            matches!(
                StrategyIr::build(&candidate),
                Err(StrategyIrError::OutOfRange { .. })
            ),
            "{label} was accepted"
        );
    }
}

// ── Execution config validation ────────────────────────────────────

#[test]
fn invalid_execution_settings_are_rejected() {
    let mut zero_capital = settings();
    zero_capital.initial_capital = 0.0;

    let mut negative_capital = settings();
    negative_capital.initial_capital = -1.0;

    let mut negative_commission = settings();
    negative_commission.commission = CommissionModel::PerShare {
        amount: -0.005,
        minimum: 1.0,
    };

    let mut negative_spread = settings();
    negative_spread.spread = SpreadModel::Constant { price_units: -0.01 };

    let mut negative_slippage = settings();
    negative_slippage.slippage = SlippageModel::SpreadFraction { fraction: -0.5 };

    for (label, candidate) in [
        ("zero capital", zero_capital),
        ("negative capital", negative_capital),
        ("negative commission", negative_commission),
        ("negative spread", negative_spread),
        ("negative slippage", negative_slippage),
    ] {
        assert!(
            matches!(
                StrategyExecutionConfig::build(&candidate),
                Err(StrategyIrError::OutOfRange { .. })
            ),
            "{label} was accepted"
        );
    }
}

#[test]
fn blank_account_currency_is_rejected() {
    let mut invalid = settings();
    invalid.account_currency = " USD".to_string();
    assert!(matches!(
        StrategyExecutionConfig::build(&invalid),
        Err(StrategyIrError::InvalidText { .. })
    ));
}

#[test]
fn conservative_policies_are_the_defaults() {
    // ADR-135 §6.1: when a stop and a target are both reachable inside one bar,
    // the default must assume the stop filled first.
    assert_eq!(
        OhlcAmbiguityPolicy::default(),
        OhlcAmbiguityPolicy::StopFirst
    );
    assert_eq!(
        TieBreakPolicy::default(),
        TieBreakPolicy::TimestampPrioritySequence
    );
}

// ── Run manifest: id shape, binding, tampering ─────────────────────

#[test]
fn malformed_content_addressed_ids_are_rejected() {
    let malformed = [
        ("empty", String::new()),
        ("too short", "a".repeat(63)),
        ("too long", "a".repeat(65)),
        ("uppercase", "A".repeat(64)),
        ("non hex", "g".repeat(64)),
        ("padded", format!(" {}", "a".repeat(63))),
    ];

    for (label, value) in malformed {
        let mut dataset = binding();
        dataset.datasets[1].dataset_id = value.clone();

        let mut strategy = binding();
        strategy.strategy_id = value.clone();

        let mut config = binding();
        config.config_id = value.clone();

        let mut intervention = binding();
        intervention.intervention_log_id = Some(value);

        for (field, candidate) in [
            ("datasets", dataset),
            ("strategy_id", strategy),
            ("config_id", config),
            ("intervention_log_id", intervention),
        ] {
            assert!(
                matches!(
                    StrategyRunManifest::build(&candidate),
                    Err(StrategyIrError::MalformedDigestId { .. })
                ),
                "`{label}` in {field} was accepted"
            );
        }
    }
}

#[test]
fn a_run_must_bind_at_least_one_dataset() {
    let mut invalid = binding();
    invalid.datasets = Vec::new();
    assert!(matches!(
        StrategyRunManifest::build(&invalid),
        Err(StrategyIrError::OutOfRange { .. })
    ));
}

#[test]
fn duplicate_dataset_input_ids_are_rejected_but_reused_content_is_allowed() {
    let mut invalid = binding();
    invalid.datasets[1].input_id = invalid.datasets[0].input_id.clone();
    assert!(matches!(
        StrategyRunManifest::build(&invalid),
        Err(StrategyIrError::DuplicateId { .. })
    ));

    let mut reused = binding();
    reused.datasets[1].dataset_id = reused.datasets[0].dataset_id.clone();
    StrategyRunManifest::build(&reused).expect("one dataset may serve distinct named inputs");
}

#[test]
fn too_many_datasets_are_rejected() {
    let mut invalid = binding();
    invalid.datasets = (0..=MAX_DATASETS_PER_RUN)
        .map(|i| DatasetBinding {
            input_id: format!("input_{i}"),
            dataset_id: format!("{i:064x}"),
        })
        .collect();
    assert!(matches!(
        StrategyRunManifest::build(&invalid),
        Err(StrategyIrError::TooMany { .. })
    ));
}

#[test]
fn too_many_repaint_qa_bindings_are_rejected() {
    let mut invalid = binding();
    invalid.repaint_qa = (0..=MAX_INDICATORS)
        .map(|index| RepaintQaBinding {
            indicator_id: format!("{index:064x}"),
            artifact_id: hex_id('a'),
            acknowledgement: RepaintAcknowledgement::Clean,
        })
        .collect();
    assert!(matches!(
        StrategyRunManifest::build(&invalid),
        Err(StrategyIrError::TooMany { .. })
    ));
}

#[test]
fn blank_engine_version_is_rejected() {
    let mut invalid = binding();
    invalid.engine_version = String::new();
    assert!(matches!(
        StrategyRunManifest::build(&invalid),
        Err(StrategyIrError::InvalidText { .. })
    ));
}

#[test]
fn unsupported_metrics_version_is_rejected_fail_closed() {
    let mut invalid = binding();
    invalid.metrics_version = "strategy-metrics/v0".to_string();
    assert!(matches!(
        StrategyRunManifest::build(&invalid),
        Err(StrategyIrError::UnsupportedMetricsVersion { .. })
    ));
}

#[test]
fn pre_repaint_binding_manifest_json_is_rejected_instead_of_silently_migrated() {
    let built = StrategyRunManifest::build(&binding()).expect("builds");
    let mut json = serde_json::to_value(&built).expect("serializes");
    json["schema_version"] = serde_json::json!(3);
    json["binding"]
        .as_object_mut()
        .expect("binding object")
        .remove("repaint_qa");

    let bytes = serde_json::to_vec(&json).expect("serializes legacy shape");
    assert!(matches!(
        StrategyRunManifest::from_json_slice(&bytes),
        Err(ArtifactLoadError::InvalidJson { .. })
    ));
}

#[test]
fn run_manifest_rejects_unknown_nested_binding_fields() {
    let built = StrategyRunManifest::build(&binding()).expect("builds");
    let mut json = serde_json::to_value(&built).expect("serializes");
    json["binding"]["surprise"] = serde_json::json!(true);

    assert!(matches!(
        StrategyRunManifest::from_json_slice(&serde_json::to_vec(&json).expect("json")),
        Err(ArtifactLoadError::InvalidJson { .. })
    ));
}

#[test]
fn a_manifest_binds_a_real_strategy_and_config() {
    let built_ir = ir();
    let config = StrategyExecutionConfig::build(&settings()).expect("builds");
    let manifest = StrategyRunManifest::build(&RunBinding {
        datasets: vec![DatasetBinding {
            input_id: "primary".to_string(),
            dataset_id: hex_id('a'),
        }],
        strategy_id: built_ir.strategy_id.clone(),
        config_id: config.config_id.clone(),
        seed: 7,
        engine_version: "typhoon-engine/0.1.0".to_string(),
        metrics_version: METRICS_SCHEMA_VERSION.to_string(),
        intervention_log_id: None,
        repaint_qa: vec![],
    })
    .expect("binding built from real ids is valid");

    manifest.verify().expect("verifies");
    assert_eq!(manifest.binding.strategy_id, built_ir.strategy_id);
    assert_eq!(manifest.binding.config_id, config.config_id);
}

fn run_dataset_fixture(
    input_id: &str,
    symbol: &str,
    adjustment: crate::core::strategy_dataset::AdjustmentPolicy,
) -> (
    Vec<crate::broker::alpaca::Bar>,
    crate::core::strategy_dataset::DatasetManifest,
) {
    use crate::broker::alpaca::Bar;
    use crate::core::strategy_dataset::{
        CalendarPolicy, DatasetManifest, DatasetManifestInput, DatasetProvenance, DatasetQaPolicy,
    };
    let bars = vec![Bar {
        timestamp: "2024-01-02T00:00:00Z".to_string(),
        open: 10.0,
        high: 11.0,
        low: 9.0,
        close: 10.5,
        volume: 100.0,
    }];
    let manifest = DatasetManifest::build(
        &DatasetManifestInput {
            symbol: symbol.to_string(),
            timeframe: "1Day".to_string(),
            provenance: DatasetProvenance {
                source: "fixture".to_string(),
                venue: input_id.to_string(),
                pipeline: "strategy-run-test/v1".to_string(),
            },
            adjustment,
            calendar: CalendarPolicy::WeekdaysOnly,
            qa_policy: DatasetQaPolicy::default(),
        },
        &bars,
    )
    .expect("dataset manifest builds");
    (bars, manifest)
}

#[test]
fn verified_run_assembly_resolves_and_verifies_every_bound_artifact() {
    use crate::core::strategy_dataset::AdjustmentPolicy;
    use crate::core::strategy_run::{RunDatasetInput, assemble_verified_run};
    use crate::core::strategy_simulator::{
        DecisionPoint, SimulationSetup, run_verified_simulation,
    };

    let mut executable_definition = ir().definition().clone();
    executable_definition.session.enabled = false;
    executable_definition.news.enabled = false;
    for indicator in &mut executable_definition.indicators {
        if matches!(indicator.kind, IndicatorKind::Custom { .. }) {
            indicator.kind = IndicatorKind::Sma;
            indicator.inputs = vec![
                IndicatorInput::Price(PriceField::Close),
                IndicatorInput::Constant(1.0),
            ];
        }
    }
    executable_definition.trade_management = TradeManagement {
        legs: vec![TradeLeg {
            fraction_bps: 10_000,
            stop: None,
            target: None,
            trailing: None,
        }],
        break_even_after: None,
        max_bars_in_trade: None,
    };
    executable_definition.sizing.rule = SizingRule::FixedUnits { units: 1.0 };
    let strategy = StrategyIr::build(&executable_definition).expect("executable strategy builds");
    let config = StrategyExecutionConfig::build(&settings()).expect("config builds");
    let (bars, dataset) = run_dataset_fixture("primary", "AAPL", AdjustmentPolicy::Raw);
    let manifest = StrategyRunManifest::build(&RunBinding {
        datasets: vec![DatasetBinding {
            input_id: "primary".to_string(),
            dataset_id: dataset.dataset_id.clone(),
        }],
        strategy_id: strategy.strategy_id().to_string(),
        config_id: config.config_id().to_string(),
        seed: 7,
        engine_version: "0.1.0-test".to_string(),
        metrics_version: METRICS_SCHEMA_VERSION.to_string(),
        intervention_log_id: None,
        repaint_qa: vec![],
    })
    .expect("run manifest builds");

    let verified = assemble_verified_run(
        &strategy,
        &config,
        &manifest,
        &[RunDatasetInput {
            input_id: "primary",
            manifest: &dataset,
            bars: &bars,
        }],
    )
    .expect("artifacts resolve");

    assert_eq!(verified.run_id(), manifest.run_id());
    assert_eq!(verified.datasets().len(), 1);
    assert_eq!(verified.datasets()[0].input_id(), "primary");
    assert_eq!(
        SimulationSetup::from_verified_run(&verified),
        SimulationSetup {
            seed: 7,
            decision_point: DecisionPoint::ClosedBar,
            submit_delay_bars: strategy.definition().timing.submit_delay_bars,
        },
        "identified simulation inputs derive only from the sealed strategy and manifest"
    );
    let report = run_verified_simulation(&verified)
        .expect("verified execution materializes only its identity-bound dataset bars");
    assert_eq!(report.symbols, vec![dataset.symbol.clone()]);

    for timing in [
        ExecutionTiming {
            decision: DecisionTiming::NextBarOpen,
            forming_bar_visible: false,
            submit_delay_bars: 0,
        },
        ExecutionTiming {
            decision: DecisionTiming::PreClose { offset_seconds: 30 },
            forming_bar_visible: true,
            submit_delay_bars: 1,
        },
    ] {
        let mut timed_definition = executable_definition.clone();
        timed_definition.timing = timing;
        let timed_strategy = StrategyIr::build(&timed_definition).expect("timed strategy builds");
        let timed_manifest = StrategyRunManifest::build(&RunBinding {
            datasets: vec![DatasetBinding {
                input_id: "primary".to_string(),
                dataset_id: dataset.dataset_id.clone(),
            }],
            strategy_id: timed_strategy.strategy_id().to_string(),
            config_id: config.config_id().to_string(),
            seed: 7,
            engine_version: "0.1.0-test".to_string(),
            metrics_version: METRICS_SCHEMA_VERSION.to_string(),
            intervention_log_id: None,
            repaint_qa: vec![],
        })
        .expect("timed manifest builds");
        let timed_run = assemble_verified_run(
            &timed_strategy,
            &config,
            &timed_manifest,
            &[RunDatasetInput {
                input_id: "primary",
                manifest: &dataset,
                bars: &bars,
            }],
        )
        .expect("timed run assembles");
        run_verified_simulation(&timed_run)
            .unwrap_or_else(|error| panic!("identity timing {timing:?} must execute: {error}"));
    }
}

#[test]
fn verified_run_assembly_refuses_corporate_actions_already_baked_into_prices() {
    use crate::core::strategy_corporate::{
        CorporateAction, CorporateActionKind, CorporateActionSchedule,
    };
    use crate::core::strategy_dataset::AdjustmentPolicy;
    use crate::core::strategy_run::{RunAssemblyError, RunDatasetInput, assemble_verified_run};

    let strategy = ir();
    let mut execution = settings();
    execution.corporate_actions = CorporateActionSchedule::build(&[CorporateAction {
        symbol: "AAPL".to_string(),
        effective_time_ns: 1,
        kind: CorporateActionKind::Split {
            numerator: 2,
            denominator: 1,
        },
    }])
    .expect("schedule builds");
    let config = StrategyExecutionConfig::build(&execution).expect("config builds");
    let (bars, dataset) = run_dataset_fixture("primary", "AAPL", AdjustmentPolicy::SplitAdjusted);
    let manifest = StrategyRunManifest::build(&RunBinding {
        datasets: vec![DatasetBinding {
            input_id: "primary".to_string(),
            dataset_id: dataset.dataset_id.clone(),
        }],
        strategy_id: strategy.strategy_id().to_string(),
        config_id: config.config_id().to_string(),
        seed: 7,
        engine_version: "0.1.0-test".to_string(),
        metrics_version: METRICS_SCHEMA_VERSION.to_string(),
        intervention_log_id: None,
        repaint_qa: vec![],
    })
    .expect("manifest builds");

    assert!(matches!(
        assemble_verified_run(
            &strategy,
            &config,
            &manifest,
            &[RunDatasetInput {
                input_id: "primary",
                manifest: &dataset,
                bars: &bars,
            }],
        ),
        Err(RunAssemblyError::CorporateActionAdjustmentConflict { .. })
    ));
}

#[test]
fn verified_run_assembly_binds_and_verifies_the_manifest_intervention_log() {
    use crate::core::strategy_dataset::AdjustmentPolicy;
    use crate::core::strategy_intervention::InterventionLog;
    use crate::core::strategy_run::{
        RunAssemblyError, RunDatasetInput, assemble_verified_run_with_intervention,
    };

    let strategy = ir();
    let config = StrategyExecutionConfig::build(&settings()).expect("config builds");
    let (bars, dataset) = run_dataset_fixture("primary", "AAPL", AdjustmentPolicy::Raw);
    let log = InterventionLog::empty();
    let mut bound = binding();
    bound.datasets = vec![DatasetBinding {
        input_id: "primary".to_string(),
        dataset_id: dataset.dataset_id.clone(),
    }];
    bound.strategy_id = strategy.strategy_id().to_string();
    bound.config_id = config.config_id().to_string();
    bound.intervention_log_id = Some(log.log_id().to_string());
    let manifest = StrategyRunManifest::build(&bound).expect("hybrid manifest builds");
    let input = RunDatasetInput {
        input_id: "primary",
        manifest: &dataset,
        bars: &bars,
    };

    let verified = assemble_verified_run_with_intervention(
        &strategy,
        &config,
        &manifest,
        &[input],
        Some(&log),
    )
    .expect("the identity-bound log resolves");
    assert_eq!(verified.intervention_log(), Some(&log));

    let foreign = InterventionLog::build(vec![crate::core::strategy_intervention::Intervention {
        decision_index: 0,
        note: "different".to_string(),
        action: crate::core::strategy_intervention::InterventionAction::Submit {
            request: crate::core::strategy_simulator::OrderRequest::market(
                crate::core::strategy_simulator::SymbolId(0),
                crate::core::strategy_simulator::OrderSide::Buy,
                1.0,
            ),
        },
    }])
    .expect("foreign log builds");
    assert!(matches!(
        assemble_verified_run_with_intervention(
            &strategy,
            &config,
            &manifest,
            &[input],
            Some(&foreign),
        ),
        Err(RunAssemblyError::InterventionLogIdMismatch { .. })
    ));
    assert!(matches!(
        assemble_verified_run_with_intervention(&strategy, &config, &manifest, &[input], None),
        Err(RunAssemblyError::MissingInterventionLog)
    ));

    let mut automated = bound;
    automated.intervention_log_id = None;
    let automated_manifest =
        StrategyRunManifest::build(&automated).expect("automated manifest builds");
    assert!(matches!(
        assemble_verified_run_with_intervention(
            &strategy,
            &config,
            &automated_manifest,
            &[input],
            Some(&log),
        ),
        Err(RunAssemblyError::UnexpectedInterventionLog)
    ));
}

#[test]
fn verified_run_assembly_binds_acknowledged_repaint_qa_fail_closed() {
    use crate::core::strategy_dataset::AdjustmentPolicy;
    use crate::core::strategy_repaint::{
        REPAINT_REPORT_SCHEMA_VERSION, RepaintFinding, RepaintPolicy, RepaintQaArtifact,
        RepaintReport, RepaintValue,
    };
    use crate::core::strategy_run::{
        RunAssemblyError, RunDatasetInput, assemble_verified_run_with_artifacts,
    };

    let strategy = ir();
    let config = StrategyExecutionConfig::build(&settings()).expect("config builds");
    let (bars, dataset) = run_dataset_fixture("primary", "AAPL", AdjustmentPolicy::Raw);
    let report = RepaintReport {
        schema_version: REPAINT_REPORT_SCHEMA_VERSION,
        policy: RepaintPolicy::default(),
        bars_scanned: 3,
        outputs_scanned: 1,
        findings: vec![RepaintFinding {
            output_index: 0,
            output_name: "signal".to_string(),
            bar_index: 0,
            observed_after_bars: 3,
            bars_back: 2,
            previous_value: RepaintValue::defined(1.0),
            new_value: RepaintValue::defined(2.0),
        }],
        findings_omitted: 0,
    };
    let qa = RepaintQaArtifact::build(&hex_id('f'), "primary", &dataset.dataset_id, report)
        .expect("QA artifact builds");
    let mut bound = binding();
    bound.datasets = vec![DatasetBinding {
        input_id: "primary".to_string(),
        dataset_id: dataset.dataset_id.clone(),
    }];
    bound.strategy_id = strategy.strategy_id().to_string();
    bound.config_id = config.config_id().to_string();
    bound.intervention_log_id = None;
    bound.repaint_qa = vec![RepaintQaBinding {
        indicator_id: qa.indicator_id().to_string(),
        artifact_id: qa.artifact_id().to_string(),
        acknowledgement: RepaintAcknowledgement::WarningAcknowledged {
            note: "reviewed exact changed bar and accepted for this run".to_string(),
        },
    }];
    let manifest = StrategyRunManifest::build(&bound).expect("manifest builds");
    let mut differently_acknowledged = bound.clone();
    differently_acknowledged.repaint_qa[0].acknowledgement =
        RepaintAcknowledgement::WarningAcknowledged {
            note: "a different explicit review".to_string(),
        };
    assert_ne!(
        StrategyRunManifest::build(&differently_acknowledged)
            .expect("alternate acknowledgement builds")
            .run_id(),
        manifest.run_id(),
        "acknowledgement identity is part of run identity"
    );
    let input = RunDatasetInput {
        input_id: "primary",
        manifest: &dataset,
        bars: &bars,
    };

    let verified =
        assemble_verified_run_with_artifacts(&strategy, &config, &manifest, &[input], None, &[&qa])
            .expect("acknowledged exact artifact resolves");
    assert_eq!(verified.repaint_qa_artifacts(), &[&qa]);

    let mut false_clean = bound.clone();
    false_clean.repaint_qa[0].acknowledgement = RepaintAcknowledgement::Clean;
    let false_clean = StrategyRunManifest::build(&false_clean).expect("shape-valid manifest");
    assert!(matches!(
        assemble_verified_run_with_artifacts(
            &strategy,
            &config,
            &false_clean,
            &[input],
            None,
            &[&qa],
        ),
        Err(RunAssemblyError::RepaintQaAcknowledgementMismatch { .. })
    ));

    assert!(matches!(
        assemble_verified_run_with_artifacts(&strategy, &config, &manifest, &[input], None, &[],),
        Err(RunAssemblyError::MissingRepaintQaArtifact { .. })
    ));
    assert!(matches!(
        assemble_verified_run_with_artifacts(
            &strategy,
            &config,
            &manifest,
            &[input],
            None,
            &[&qa, &qa],
        ),
        Err(RunAssemblyError::DuplicateRepaintQaArtifact { .. })
    ));
    let foreign = RepaintQaArtifact::build(
        &hex_id('f'),
        "primary",
        &dataset.dataset_id,
        RepaintReport {
            findings: vec![],
            ..qa.report().clone()
        },
    )
    .expect("foreign QA builds");
    assert!(matches!(
        assemble_verified_run_with_artifacts(
            &strategy,
            &config,
            &manifest,
            &[input],
            None,
            &[&foreign],
        ),
        Err(RunAssemblyError::RepaintQaArtifactIdMismatch { .. })
    ));

    let wrong_dataset_qa =
        RepaintQaArtifact::build(&hex_id('f'), "primary", &hex_id('9'), qa.report().clone())
            .expect("foreign-dataset QA builds");
    let mut wrong_dataset_bound = bound.clone();
    wrong_dataset_bound.repaint_qa[0].artifact_id = wrong_dataset_qa.artifact_id().to_string();
    let wrong_dataset_manifest =
        StrategyRunManifest::build(&wrong_dataset_bound).expect("shape-valid manifest");
    assert!(matches!(
        assemble_verified_run_with_artifacts(
            &strategy,
            &config,
            &wrong_dataset_manifest,
            &[input],
            None,
            &[&wrong_dataset_qa],
        ),
        Err(RunAssemblyError::RepaintQaDatasetMismatch { .. })
    ));

    let mut automated = bound;
    automated.repaint_qa.clear();
    let automated = StrategyRunManifest::build(&automated).expect("manifest builds");
    assert!(matches!(
        assemble_verified_run_with_artifacts(
            &strategy,
            &config,
            &automated,
            &[input],
            None,
            &[&qa],
        ),
        Err(RunAssemblyError::UnexpectedRepaintQaArtifact { .. })
    ));
}

#[test]
fn verified_run_assembly_rejects_identity_mismatch_and_dataset_tampering() {
    use crate::core::strategy_dataset::AdjustmentPolicy;
    use crate::core::strategy_run::{RunAssemblyError, RunDatasetInput, assemble_verified_run};

    let strategy = ir();
    let config = StrategyExecutionConfig::build(&settings()).expect("config builds");
    let (mut bars, dataset) = run_dataset_fixture("primary", "AAPL", AdjustmentPolicy::Raw);
    let mut bound = binding();
    bound.datasets = vec![DatasetBinding {
        input_id: "primary".to_string(),
        dataset_id: dataset.dataset_id.clone(),
    }];
    bound.config_id = config.config_id().to_string();
    bound.intervention_log_id = None;
    let wrong_strategy = StrategyRunManifest::build(&bound).expect("shape-valid manifest");
    assert!(matches!(
        assemble_verified_run(
            &strategy,
            &config,
            &wrong_strategy,
            &[RunDatasetInput {
                input_id: "primary",
                manifest: &dataset,
                bars: &bars,
            }],
        ),
        Err(RunAssemblyError::StrategyIdMismatch { .. })
    ));

    bound.strategy_id = strategy.strategy_id().to_string();
    let manifest = StrategyRunManifest::build(&bound).expect("manifest builds");
    bars[0].close = 999.0;
    let tampered = assemble_verified_run(
        &strategy,
        &config,
        &manifest,
        &[RunDatasetInput {
            input_id: "primary",
            manifest: &dataset,
            bars: &bars,
        }],
    );
    assert!(
        matches!(tampered, Err(RunAssemblyError::InvalidDataset { .. })),
        "unexpected result: {tampered:?}"
    );
}

#[test]
fn verified_run_assembly_rejects_missing_duplicate_and_mixed_policy_inputs() {
    use crate::core::strategy_dataset::AdjustmentPolicy;
    use crate::core::strategy_run::{RunAssemblyError, RunDatasetInput, assemble_verified_run};

    let strategy = ir();
    let config = StrategyExecutionConfig::build(&settings()).expect("config builds");
    let (bars_a, dataset_a) = run_dataset_fixture("primary", "AAPL", AdjustmentPolicy::Raw);
    let (bars_b, dataset_b) =
        run_dataset_fixture("confirmation", "MSFT", AdjustmentPolicy::SplitAdjusted);
    let manifest = StrategyRunManifest::build(&RunBinding {
        datasets: vec![
            DatasetBinding {
                input_id: "primary".to_string(),
                dataset_id: dataset_a.dataset_id.clone(),
            },
            DatasetBinding {
                input_id: "confirmation".to_string(),
                dataset_id: dataset_b.dataset_id.clone(),
            },
        ],
        strategy_id: strategy.strategy_id().to_string(),
        config_id: config.config_id().to_string(),
        seed: 9,
        engine_version: "0.1.0-test".to_string(),
        metrics_version: METRICS_SCHEMA_VERSION.to_string(),
        intervention_log_id: None,
        repaint_qa: vec![],
    })
    .expect("manifest builds");

    let primary = RunDatasetInput {
        input_id: "primary",
        manifest: &dataset_a,
        bars: &bars_a,
    };
    assert!(matches!(
        assemble_verified_run(&strategy, &config, &manifest, &[primary]),
        Err(RunAssemblyError::MissingDatasetInput { .. })
    ));
    assert!(matches!(
        assemble_verified_run(&strategy, &config, &manifest, &[primary, primary]),
        Err(RunAssemblyError::DuplicateDatasetInput { .. })
    ));

    let confirmation = RunDatasetInput {
        input_id: "confirmation",
        manifest: &dataset_b,
        bars: &bars_b,
    };
    assert!(matches!(
        assemble_verified_run(&strategy, &config, &manifest, &[primary, confirmation],),
        Err(RunAssemblyError::MixedAdjustmentPolicy { .. })
    ));
}

#[test]
fn tampering_with_a_run_manifest_fails_verification() {
    for (label, mutate) in binding_mutations() {
        let mut manifest = StrategyRunManifest::build(&binding()).expect("builds");
        mutate(&mut manifest.binding);
        match manifest.verify() {
            Err(StrategyIrError::IdentityMismatch { artifact, .. }) => {
                assert_eq!(artifact, ArtifactKind::RunManifest)
            }
            other => panic!("tampering with `{label}` was not detected: {other:?}"),
        }
    }
}

#[test]
fn tampering_with_a_recorded_run_id_fails_verification() {
    let mut manifest = StrategyRunManifest::build(&binding()).expect("builds");
    manifest.run_id = hex_id('9');
    assert!(matches!(
        manifest.verify(),
        Err(StrategyIrError::IdentityMismatch { .. })
    ));
}

#[test]
fn tampering_with_a_strategy_definition_fails_verification() {
    for (label, mutate) in definition_mutations() {
        let mut built = ir();
        mutate(&mut built.definition);
        match built.verify() {
            Err(StrategyIrError::IdentityMismatch { artifact, .. }) => {
                assert_eq!(artifact, ArtifactKind::StrategyIr)
            }
            other => panic!("tampering with `{label}` was not detected: {other:?}"),
        }
    }
}

#[test]
fn tampering_with_execution_settings_fails_verification() {
    for (label, mutate) in settings_mutations() {
        let mut config = StrategyExecutionConfig::build(&settings()).expect("builds");
        mutate(&mut config.settings);
        match config.verify() {
            Err(StrategyIrError::IdentityMismatch { artifact, .. }) => {
                assert_eq!(artifact, ArtifactKind::ExecutionConfig)
            }
            other => panic!("tampering with `{label}` was not detected: {other:?}"),
        }
    }
}

#[test]
fn verification_reports_invalidity_before_identity() {
    // A tampered artifact that is also structurally invalid must report the
    // structural fault, not a hash mismatch — the fault is the more actionable
    // of the two.
    let mut built = ir();
    built.definition.roles[0].indicator = "missing".to_string();
    assert!(matches!(
        built.verify(),
        Err(StrategyIrError::UnknownRef { .. })
    ));
}

// ── Schema versioning ──────────────────────────────────────────────

#[test]
fn unsupported_schema_versions_are_rejected() {
    let mut built = ir();
    built.schema_version = STRATEGY_IR_SCHEMA_VERSION + 1;
    match built.verify() {
        Err(StrategyIrError::UnsupportedSchemaVersion {
            artifact,
            found,
            supported,
        }) => {
            assert_eq!(artifact, ArtifactKind::StrategyIr);
            assert_eq!(found, STRATEGY_IR_SCHEMA_VERSION + 1);
            assert_eq!(supported, STRATEGY_IR_SCHEMA_VERSION);
        }
        other => panic!("unsupported IR schema version was not rejected: {other:?}"),
    }
    assert!(matches!(
        built.recompute_strategy_id(),
        Err(StrategyIrError::UnsupportedSchemaVersion { .. })
    ));

    let mut config = StrategyExecutionConfig::build(&settings()).expect("builds");
    config.schema_version = 0;
    assert!(matches!(
        config.verify(),
        Err(StrategyIrError::UnsupportedSchemaVersion {
            artifact: ArtifactKind::ExecutionConfig,
            ..
        })
    ));

    let mut manifest = StrategyRunManifest::build(&binding()).expect("builds");
    manifest.schema_version = STRATEGY_RUN_MANIFEST_SCHEMA_VERSION + 7;
    assert!(matches!(
        manifest.verify(),
        Err(StrategyIrError::UnsupportedSchemaVersion {
            artifact: ArtifactKind::RunManifest,
            ..
        })
    ));
}

#[test]
fn a_stored_artifact_from_a_future_schema_is_not_silently_reinterpreted() {
    let json = serde_json::to_string(&ir()).expect("serializes");
    let bumped = json.replace(
        &format!("\"schema_version\":{STRATEGY_IR_SCHEMA_VERSION}"),
        &format!("\"schema_version\":{}", STRATEGY_IR_SCHEMA_VERSION + 1),
    );
    assert!(matches!(
        StrategyIr::from_json_slice(bumped.as_bytes()),
        Err(ArtifactLoadError::InvalidArtifact(
            StrategyIrError::UnsupportedSchemaVersion { .. }
        ))
    ));
}

#[test]
fn sealed_artifact_loading_rejects_tampering_and_unknown_fields() {
    let mut tampered = serde_json::to_value(ir()).expect("serializes");
    tampered["strategy_id"] = serde_json::Value::String(hex_id('9'));
    let tampered = serde_json::to_vec(&tampered).expect("serializes tampered artifact");
    assert!(matches!(
        StrategyIr::from_json_slice(&tampered),
        Err(ArtifactLoadError::InvalidArtifact(
            StrategyIrError::IdentityMismatch { .. }
        ))
    ));

    let mut unknown = serde_json::to_value(ir()).expect("serializes");
    unknown["definition"]["metadata"]["misspelled_name"] =
        serde_json::Value::String("ignored before strict loading".to_string());
    let unknown = serde_json::to_vec(&unknown).expect("serializes unknown field");
    assert!(matches!(
        StrategyIr::from_json_slice(&unknown),
        Err(ArtifactLoadError::InvalidJson { .. })
    ));
}

#[test]
fn sealed_artifact_loading_rejects_oversized_json_before_decoding() {
    let oversized = vec![b' '; MAX_SEALED_ARTIFACT_JSON_BYTES + 1];
    assert!(matches!(
        StrategyIr::from_json_slice(&oversized),
        Err(ArtifactLoadError::TooLarge { limit, found })
            if limit == MAX_SEALED_ARTIFACT_JSON_BYTES && found == oversized.len()
    ));
}

// ── Execution realism surface (§6.3–§6.5, §6.9) ────────────────────

fn kraken_binding() -> FeeScheduleBinding {
    let schedule = FeeSchedule::build(
        FeeVenue::KrakenSpot,
        1,
        "2026-07-27",
        FeeProvenance::OperatorAssumption {
            note: "M1 identity fixture".to_string(),
        },
        FeeScheduleShape::KrakenSpot {
            tiers: vec![VolumeTier {
                min_volume: 0.0,
                maker_percent: 0.25,
                taker_percent: 0.40,
            }],
        },
    )
    .expect("shape is valid");
    FeeScheduleBinding::build(schedule, 0, LiquidityAssumption::Taker).expect("tier 0 exists")
}

#[test]
fn the_default_execution_model_is_the_conservative_one() {
    let settings = ExecutionSettings::conservative_defaults();
    assert_eq!(settings.fidelity, FidelityLevel::BarClose);
    assert_eq!(settings.latency, LatencyModel::None);
    assert_eq!(settings.compatibility, ExecutionCompatibility::None);
    assert_eq!(settings.ambiguity, OhlcAmbiguityPolicy::StopFirst);
    assert_eq!(settings.margin, MarginPolicy::Unconstrained);
    assert_eq!(settings.warmup_bars, 0);
    assert_eq!(settings.price_tick, None);
    StrategyExecutionConfig::build(&settings).expect("the defaults are a valid config");
}

#[test]
fn a_venue_fee_schedule_can_be_bound_to_a_config() {
    let mut bound = settings();
    bound.commission = CommissionModel::VenueSchedule(kraken_binding());
    let config = StrategyExecutionConfig::build(&bound).expect("a bound schedule is valid");
    let CommissionModel::VenueSchedule(binding) = &config.settings().commission else {
        panic!("the schedule did not survive the round trip");
    };
    assert_eq!(binding.schedule().venue(), FeeVenue::KrakenSpot);
    assert_eq!(binding.schedule().effective_date(), "2026-07-27");
    assert_eq!(binding.liquidity(), LiquidityAssumption::Taker);
}

#[test]
fn a_deserialized_fee_schedule_is_revalidated_before_it_is_sealed() {
    let mut bound = settings();
    bound.commission = CommissionModel::VenueSchedule(kraken_binding());
    let mut wire = serde_json::to_value(&bound).expect("settings serialize");
    wire["commission"]["venue_schedule"]["schedule"]["shape"]["kraken_spot"]["tiers"] =
        serde_json::json!([]);

    let malformed: ExecutionSettings =
        serde_json::from_value(wire).expect("serde alone does not validate private fields");
    assert!(
        StrategyExecutionConfig::build(&malformed).is_err(),
        "a malformed nested schedule must not become an identity-bearing config"
    );
}

#[test]
fn latency_bounds_are_enforced_in_both_directions() {
    let over = ExecutionSettings {
        latency: LatencyModel::Fixed {
            decision_to_submit_ns: MAX_LATENCY_NS + 1,
            submit_to_exchange_ns: 0,
        },
        ..settings()
    };
    assert!(matches!(
        StrategyExecutionConfig::build(&over),
        Err(StrategyIrError::OutOfRange { ref field, .. })
            if field == "settings.latency.decision_to_submit_ns"
    ));

    let negative = ExecutionSettings {
        latency: LatencyModel::Fixed {
            decision_to_submit_ns: 0,
            submit_to_exchange_ns: -1,
        },
        ..settings()
    };
    assert!(matches!(
        StrategyExecutionConfig::build(&negative),
        Err(StrategyIrError::OutOfRange { ref field, .. })
            if field == "settings.latency.submit_to_exchange_ns"
    ));

    let inverted = ExecutionSettings {
        latency: LatencyModel::SeededUniform {
            decision_to_submit_min_ns: 10,
            decision_to_submit_max_ns: 5,
            submit_to_exchange_min_ns: 0,
            submit_to_exchange_max_ns: 0,
        },
        ..settings()
    };
    assert!(matches!(
        StrategyExecutionConfig::build(&inverted),
        Err(StrategyIrError::OutOfRange { ref field, .. })
            if field == "settings.latency.decision_to_submit_max_ns"
    ));

    let ok = ExecutionSettings {
        latency: LatencyModel::SeededUniform {
            decision_to_submit_min_ns: 5,
            decision_to_submit_max_ns: 10,
            submit_to_exchange_min_ns: 1,
            submit_to_exchange_max_ns: 1,
        },
        ..settings()
    };
    StrategyExecutionConfig::build(&ok).expect("an ordered draw range is valid");
}

#[test]
fn a_non_positive_price_tick_is_refused() {
    for tick in [0.0, -0.01, f64::NAN] {
        let settings = ExecutionSettings {
            price_tick: Some(tick),
            ..settings()
        };
        assert!(
            StrategyExecutionConfig::build(&settings).is_err(),
            "tick {tick} must not build a config"
        );
    }
    StrategyExecutionConfig::build(&ExecutionSettings {
        price_tick: Some(0.01),
        ..settings()
    })
    .expect("a positive tick is valid");
}

#[test]
fn the_warmup_boundary_is_bounded() {
    let over = ExecutionSettings {
        warmup_bars: MAX_WARMUP_BARS + 1,
        ..settings()
    };
    assert!(matches!(
        StrategyExecutionConfig::build(&over),
        Err(StrategyIrError::OutOfRange { ref field, .. }) if field == "settings.warmup_bars"
    ));
    StrategyExecutionConfig::build(&ExecutionSettings {
        warmup_bars: MAX_WARMUP_BARS,
        ..settings()
    })
    .expect("the cap itself is valid");
}

#[test]
fn legacy_same_close_compatibility_cannot_hide_inside_a_realistic_config() {
    let with_latency = ExecutionSettings {
        compatibility: ExecutionCompatibility::LegacySameBarClose,
        fidelity: FidelityLevel::BarClose,
        latency: LatencyModel::Fixed {
            decision_to_submit_ns: 1,
            submit_to_exchange_ns: 0,
        },
        ..settings()
    };
    assert!(matches!(
        StrategyExecutionConfig::build(&with_latency),
        Err(StrategyIrError::InconsistentExecution { .. })
    ));

    let with_intrabar = ExecutionSettings {
        compatibility: ExecutionCompatibility::LegacySameBarClose,
        fidelity: FidelityLevel::BarOhlc,
        latency: LatencyModel::None,
        ..settings()
    };
    assert!(matches!(
        StrategyExecutionConfig::build(&with_intrabar),
        Err(StrategyIrError::InconsistentExecution { .. })
    ));

    StrategyExecutionConfig::build(&ExecutionSettings {
        compatibility: ExecutionCompatibility::LegacySameBarClose,
        fidelity: FidelityLevel::BarClose,
        latency: LatencyModel::None,
        ..settings()
    })
    .expect("the declared compatibility combination is valid");
}

#[test]
fn every_new_execution_field_changes_the_config_id() {
    let baseline = config_id_of(&settings());
    let mut seen = BTreeSet::from([baseline.clone()]);
    let mutations: Vec<(&'static str, fn(&mut ExecutionSettings))> = vec![
        ("fidelity", |s| s.fidelity = FidelityLevel::BarOhlc),
        ("latency.fixed", |s| {
            s.latency = LatencyModel::Fixed {
                decision_to_submit_ns: 1,
                submit_to_exchange_ns: 2,
            }
        }),
        ("latency.seeded", |s| {
            s.latency = LatencyModel::SeededUniform {
                decision_to_submit_min_ns: 1,
                decision_to_submit_max_ns: 2,
                submit_to_exchange_min_ns: 3,
                submit_to_exchange_max_ns: 4,
            }
        }),
        ("compatibility", |s| {
            s.compatibility = ExecutionCompatibility::LegacySameBarClose;
            s.fidelity = FidelityLevel::BarClose;
            s.latency = LatencyModel::None;
        }),
        ("margin", |s| s.margin = MarginPolicy::CashOnly),
        ("price_tick", |s| s.price_tick = Some(0.01)),
        ("warmup_bars", |s| s.warmup_bars = 7),
        ("commission.venue_schedule", |s| {
            s.commission = CommissionModel::VenueSchedule(kraken_binding())
        }),
    ];
    for (label, mutate) in mutations {
        let mut mutated = settings();
        mutate(&mut mutated);
        let id = StrategyExecutionConfig::build(&mutated)
            .unwrap_or_else(|e| panic!("mutation `{label}` produced invalid settings: {e}"))
            .config_id;
        assert_ne!(id, baseline, "mutation `{label}` did not change the id");
        assert!(
            seen.insert(id),
            "mutation `{label}` collided with another mutation's id"
        );
    }
}

#[test]
fn every_fee_schedule_choice_changes_the_config_id() {
    let mut seen = BTreeSet::new();
    let schedule = |version: u32, date: &str, note: &str, taker: f64| {
        FeeSchedule::build(
            FeeVenue::KrakenSpot,
            version,
            date,
            FeeProvenance::OperatorAssumption {
                note: note.to_string(),
            },
            FeeScheduleShape::KrakenSpot {
                tiers: vec![VolumeTier {
                    min_volume: 0.0,
                    maker_percent: 0.25,
                    taker_percent: taker,
                }],
            },
        )
        .expect("valid")
    };
    let cases = [
        ("baseline", schedule(1, "2026-07-27", "note", 0.40)),
        ("version", schedule(2, "2026-07-27", "note", 0.40)),
        ("effective_date", schedule(1, "2026-07-28", "note", 0.40)),
        ("provenance", schedule(1, "2026-07-27", "other note", 0.40)),
        ("rate", schedule(1, "2026-07-27", "note", 0.41)),
    ];
    for (label, schedule) in cases {
        let binding = FeeScheduleBinding::build(schedule, 0, LiquidityAssumption::Taker)
            .expect("tier 0 exists");
        let settings = ExecutionSettings {
            commission: CommissionModel::VenueSchedule(binding),
            ..settings()
        };
        let id = config_id_of(&settings);
        assert!(seen.insert(id), "`{label}` collided with another schedule");
    }
    assert_eq!(seen.len(), 5);
}

#[test]
fn the_execution_config_schema_version_records_the_realism_fields() {
    assert_eq!(
        STRATEGY_EXECUTION_CONFIG_SCHEMA_VERSION, 3,
        "adding execution-realism fields is a schema change, not a silent reinterpretation"
    );
}

// ── Error surface ──────────────────────────────────────────────────

#[test]
fn errors_render_without_debug_formatting() {
    let mut invalid = definition();
    invalid.roles[0].indicator = "missing".to_string();
    let error = StrategyIr::build(&invalid).expect_err("is invalid");
    let rendered = error.to_string();
    assert!(
        rendered.contains("missing"),
        "unhelpful message: {rendered}"
    );
    assert!(
        !rendered.contains("UnknownRef"),
        "message leaks the Debug variant name: {rendered}"
    );
}
