//! Native-authoring models for canonical strategy IR.
//!
//! Both editors in this module lower directly to [`StrategyDefinition`].  The
//! guided NNFX form is deliberately only a constrained projection of the same
//! graph consumed by [`GeneralStrategyBuilder`]; it has no executable runtime
//! representation of its own.

use crate::core::strategy_ir::*;

#[derive(Debug, Clone, PartialEq)]
pub struct BuilderError(pub String);

impl std::fmt::Display for BuilderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for BuilderError {}

impl From<StrategyIrError> for BuilderError {
    fn from(value: StrategyIrError) -> Self {
        Self(value.to_string())
    }
}

impl From<ArtifactLoadError> for BuilderError {
    fn from(value: ArtifactLoadError) -> Self {
        Self(value.to_string())
    }
}

/// A palette item used by the graphical editor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndicatorDraft {
    pub id: String,
    pub kind: IndicatorKind,
    pub period: u32,
}

impl IndicatorDraft {
    pub fn new(id: impl Into<String>, kind: IndicatorKind, period: u32) -> Self {
        Self {
            id: id.into(),
            kind,
            period,
        }
    }

    fn node(&self) -> IndicatorNode {
        let scalar = IndicatorInput::Constant(f64::from(self.period));
        let inputs = match self.kind {
            IndicatorKind::Atr | IndicatorKind::Adx => vec![scalar],
            IndicatorKind::Kama | IndicatorKind::Macd | IndicatorKind::LegacyRollingKamaV1 => {
                vec![
                    IndicatorInput::Price(PriceField::Close),
                    scalar,
                    IndicatorInput::Constant(2.0),
                    IndicatorInput::Constant(30.0),
                ]
            }
            IndicatorKind::LegacyUnsmoothedFisherMidpointV1
            | IndicatorKind::LegacyFisherValueV1
            | IndicatorKind::LegacyFisherSignalV1 => vec![scalar],
            IndicatorKind::Custom { .. } => vec![IndicatorInput::Price(PriceField::Close), scalar],
            _ => vec![IndicatorInput::Price(PriceField::Close), scalar],
        };
        IndicatorNode {
            id: self.id.clone(),
            kind: self.kind.clone(),
            inputs,
        }
    }
}

/// Mutable visual graph state. Invalid in-progress edits are retained for the
/// UI to repair while `last_valid_ir` remains available for preview/rerun.
#[derive(Debug, Clone)]
pub struct GeneralStrategyBuilder {
    definition: StrategyDefinition,
    last_valid_ir: Option<StrategyIr>,
}

impl GeneralStrategyBuilder {
    pub fn new(name: impl Into<String>, author: impl Into<String>) -> Self {
        Self::from_definition(empty_definition(name.into(), author.into()))
    }

    pub fn from_definition(definition: StrategyDefinition) -> Self {
        let last_valid_ir = StrategyIr::build(&definition).ok();
        Self {
            definition,
            last_valid_ir,
        }
    }

    pub fn from_canonical_text(text: &str) -> Result<Self, BuilderError> {
        let ir = StrategyIr::from_json_slice(text.as_bytes())?;
        Ok(Self {
            definition: ir.to_input(),
            last_valid_ir: Some(ir),
        })
    }

    pub fn definition(&self) -> &StrategyDefinition {
        &self.definition
    }

    pub fn definition_mut(&mut self) -> &mut StrategyDefinition {
        &mut self.definition
    }

    pub fn last_valid_ir(&self) -> Option<&StrategyIr> {
        self.last_valid_ir.as_ref()
    }

    pub fn add_indicator(&mut self, draft: IndicatorDraft) {
        self.definition.indicators.push(draft.node());
    }

    pub fn set_baseline_crossover(&mut self, indicator: &str) {
        self.definition
            .roles
            .retain(|role| role.role != IndicatorRole::Baseline);
        self.definition.roles.push(RoleAssignment {
            role: IndicatorRole::Baseline,
            indicator: indicator.to_string(),
        });
        self.definition.long.entry = cross(indicator, true);
        self.definition.long.exit = cross(indicator, false);
        self.definition.short.entry = cross(indicator, false);
        self.definition.short.exit = cross(indicator, true);
    }

    pub fn validation(&self) -> Result<StrategyIr, BuilderError> {
        StrategyIr::build(&self.definition).map_err(Into::into)
    }

    pub fn seal(&mut self) -> Result<StrategyIr, BuilderError> {
        let ir = self.validation()?;
        self.definition = ir.to_input();
        self.last_valid_ir = Some(ir.clone());
        Ok(ir)
    }

    /// Plain-text canonical view shown by the GUI and persisted by the store.
    pub fn canonical_text(&self) -> Result<String, BuilderError> {
        let ir = self.validation()?;
        serde_json::to_string_pretty(&ir).map_err(|error| BuilderError(error.to_string()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NnfxProfile {
    Full,
    Confirmation1Only,
    BaselineOnly,
    BaselineConfirmation1,
}

impl NnfxProfile {
    pub const ALL: [Self; 4] = [
        Self::Full,
        Self::Confirmation1Only,
        Self::BaselineOnly,
        Self::BaselineConfirmation1,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NnfxEntryMode {
    Baseline,
    Standard,
    Continuation,
    Pullback,
}

impl NnfxEntryMode {
    pub const ALL: [Self; 4] = [
        Self::Baseline,
        Self::Standard,
        Self::Continuation,
        Self::Pullback,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectionConstraint {
    Both,
    LongOnly,
    ShortOnly,
}

impl DirectionConstraint {
    pub const ALL: [Self; 3] = [Self::Both, Self::LongOnly, Self::ShortOnly];
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NnfxSlot {
    pub kind: IndicatorKind,
    pub period: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NnfxSlots {
    pub atr: NnfxSlot,
    pub baseline: NnfxSlot,
    pub confirmation_1: NnfxSlot,
    pub confirmation_2: NnfxSlot,
    pub volume: NnfxSlot,
    pub exit: NnfxSlot,
    pub continuation: NnfxSlot,
}

impl Default for NnfxSlots {
    fn default() -> Self {
        Self {
            atr: NnfxSlot {
                kind: IndicatorKind::Atr,
                period: 14,
            },
            baseline: NnfxSlot {
                kind: IndicatorKind::Ema,
                period: 20,
            },
            confirmation_1: NnfxSlot {
                kind: IndicatorKind::Rsi,
                period: 14,
            },
            confirmation_2: NnfxSlot {
                kind: IndicatorKind::FisherTransform,
                period: 10,
            },
            volume: NnfxSlot {
                kind: IndicatorKind::StdDev,
                period: 20,
            },
            exit: NnfxSlot {
                kind: IndicatorKind::Ema,
                period: 8,
            },
            continuation: NnfxSlot {
                kind: IndicatorKind::Adx,
                period: 14,
            },
        }
    }
}

/// Complete guided form state. News and external-market filters are explicit
/// switches; the market filter lowers to an ordinary graph node and condition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NnfxBuilderConfig {
    pub profile: NnfxProfile,
    pub entry_mode: NnfxEntryMode,
    pub direction: DirectionConstraint,
    pub one_candle_rule: bool,
    pub bridge_too_far_rule: bool,
    pub news_filter: bool,
    pub market_filter: bool,
    pub slots: NnfxSlots,
}

impl Default for NnfxBuilderConfig {
    fn default() -> Self {
        Self {
            profile: NnfxProfile::Full,
            entry_mode: NnfxEntryMode::Standard,
            direction: DirectionConstraint::Both,
            one_candle_rule: true,
            bridge_too_far_rule: true,
            news_filter: true,
            market_filter: true,
            slots: NnfxSlots::default(),
        }
    }
}

impl NnfxBuilderConfig {
    pub fn to_definition(&self) -> Result<StrategyDefinition, BuilderError> {
        let definition = self.lower_graph();
        StrategyIr::build(&definition)
            .map(|ir| ir.to_input())
            .map_err(Into::into)
    }

    pub fn to_ir(&self) -> Result<StrategyIr, BuilderError> {
        StrategyIr::build(&self.lower_graph()).map_err(Into::into)
    }

    /// The graph displayed after "Open in general builder". This method is
    /// intentionally expressed in StrategyDefinition, not a guided runtime AST.
    pub fn equivalent_general_definition(&self) -> Result<StrategyDefinition, BuilderError> {
        self.to_definition()
    }

    fn lower_graph(&self) -> StrategyDefinition {
        let slot_specs = [
            ("atr", IndicatorRole::Atr, &self.slots.atr),
            ("baseline", IndicatorRole::Baseline, &self.slots.baseline),
            (
                "confirmation_1",
                IndicatorRole::Confirmation1,
                &self.slots.confirmation_1,
            ),
            (
                "confirmation_2",
                IndicatorRole::Confirmation2,
                &self.slots.confirmation_2,
            ),
            ("volume", IndicatorRole::Volume, &self.slots.volume),
            ("exit", IndicatorRole::Exit, &self.slots.exit),
            (
                "continuation",
                IndicatorRole::Continuation,
                &self.slots.continuation,
            ),
        ];
        let mut indicators = Vec::with_capacity(slot_specs.len() + usize::from(self.market_filter));
        let mut roles = Vec::with_capacity(slot_specs.len());
        for (id, role, slot) in slot_specs {
            indicators.push(IndicatorDraft::new(id, slot.kind.clone(), slot.period).node());
            roles.push(RoleAssignment {
                role,
                indicator: id.to_string(),
            });
        }
        if self.market_filter {
            indicators.push(IndicatorDraft::new("market_filter", IndicatorKind::Sma, 200).node());
        }

        let (long_enabled, short_enabled) = match self.direction {
            DirectionConstraint::Both => (true, true),
            DirectionConstraint::LongOnly => (true, false),
            DirectionConstraint::ShortOnly => (false, true),
        };
        let long_entry = self.entry_condition(true);
        let short_entry = self.entry_condition(false);
        StrategyDefinition {
            metadata: StrategyMetadata {
                name: "NNFX guided strategy".into(),
                author: "TyphooN Builder".into(),
                notes: Some("Canonical graph produced by the guided NNFX editor".into()),
                tags: vec!["nnfx".into(), "builder".into()],
            },
            parameters: Vec::new(),
            indicators,
            roles,
            long: DirectionRules {
                enabled: long_enabled,
                entry: long_entry,
                exit: indicator_sign("exit", false),
            },
            short: DirectionRules {
                enabled: short_enabled,
                entry: short_entry,
                exit: indicator_sign("exit", true),
            },
            session: SessionFilter {
                enabled: false,
                windows: Vec::new(),
                close_positions_outside: false,
            },
            news: NewsFilter {
                enabled: self.news_filter,
                min_impact: NewsImpact::High,
                block_minutes_before: if self.news_filter { 30 } else { 0 },
                block_minutes_after: if self.news_filter { 30 } else { 0 },
                close_open_positions: false,
            },
            sizing: PositionSizing {
                rule: SizingRule::FixedUnits { units: 1.0 },
                max_open_positions: 1,
            },
            trade_management: single_leg_management(),
            timing: ExecutionTiming {
                decision: DecisionTiming::ClosedBar,
                forming_bar_visible: false,
                submit_delay_bars: 0,
            },
        }
    }

    fn entry_condition(&self, long: bool) -> Condition {
        let mut rules = match self.entry_mode {
            NnfxEntryMode::Baseline => vec![cross("baseline", long)],
            _ => self.profile_rules(long),
        };
        match self.entry_mode {
            NnfxEntryMode::Continuation => rules.push(indicator_sign("continuation", long)),
            NnfxEntryMode::Pullback => rules.push(Condition::Compare {
                left: Operand::Price {
                    field: PriceField::Close,
                    bars_ago: 0,
                },
                op: if long {
                    CompareOp::GreaterOrEqual
                } else {
                    CompareOp::LessOrEqual
                },
                right: indicator_operand("baseline", 0),
            }),
            _ => {}
        }
        if self.one_candle_rule {
            rules.push(Condition::Compare {
                left: indicator_operand("confirmation_1", 0),
                op: if long {
                    CompareOp::GreaterOrEqual
                } else {
                    CompareOp::LessOrEqual
                },
                right: indicator_operand("confirmation_1", 1),
            });
        }
        if self.bridge_too_far_rule {
            rules.push(Condition::Compare {
                left: indicator_operand("atr", 0),
                op: CompareOp::Greater,
                right: Operand::Constant(0.0),
            });
        }
        if self.market_filter {
            rules.push(Condition::Compare {
                left: Operand::Price {
                    field: PriceField::Close,
                    bars_ago: 0,
                },
                op: if long {
                    CompareOp::Greater
                } else {
                    CompareOp::Less
                },
                right: indicator_operand("market_filter", 0),
            });
        }
        if rules.len() == 1 {
            rules.pop().unwrap()
        } else {
            Condition::All(rules)
        }
    }

    fn profile_rules(&self, long: bool) -> Vec<Condition> {
        match self.profile {
            NnfxProfile::Full => vec![
                cross("baseline", long),
                indicator_sign("confirmation_1", long),
                indicator_sign("confirmation_2", long),
                indicator_sign("volume", true),
            ],
            NnfxProfile::Confirmation1Only => vec![indicator_sign("confirmation_1", long)],
            NnfxProfile::BaselineOnly => vec![cross("baseline", long)],
            NnfxProfile::BaselineConfirmation1 => {
                vec![
                    cross("baseline", long),
                    indicator_sign("confirmation_1", long),
                ]
            }
        }
    }
}

fn empty_definition(name: String, author: String) -> StrategyDefinition {
    StrategyDefinition {
        metadata: StrategyMetadata {
            name,
            author,
            notes: None,
            tags: Vec::new(),
        },
        parameters: Vec::new(),
        indicators: Vec::new(),
        roles: Vec::new(),
        long: DirectionRules {
            enabled: true,
            entry: Condition::Never,
            exit: Condition::Never,
        },
        short: DirectionRules {
            enabled: true,
            entry: Condition::Never,
            exit: Condition::Never,
        },
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
            rule: SizingRule::FixedUnits { units: 1.0 },
            max_open_positions: 1,
        },
        trade_management: single_leg_management(),
        timing: ExecutionTiming {
            decision: DecisionTiming::ClosedBar,
            forming_bar_visible: false,
            submit_delay_bars: 0,
        },
    }
}

fn single_leg_management() -> TradeManagement {
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

fn indicator_operand(id: &str, bars_ago: u32) -> Operand {
    Operand::Indicator {
        id: id.to_string(),
        bars_ago,
    }
}

fn indicator_sign(id: &str, positive: bool) -> Condition {
    Condition::Compare {
        left: indicator_operand(id, 0),
        op: if positive {
            CompareOp::Greater
        } else {
            CompareOp::Less
        },
        right: Operand::Constant(0.0),
    }
}

fn cross(id: &str, above: bool) -> Condition {
    let left = Operand::Price {
        field: PriceField::Close,
        bars_ago: 0,
    };
    let right = indicator_operand(id, 0);
    if above {
        Condition::CrossesAbove { left, right }
    } else {
        Condition::CrossesBelow { left, right }
    }
}

#[cfg(test)]
mod tests;
