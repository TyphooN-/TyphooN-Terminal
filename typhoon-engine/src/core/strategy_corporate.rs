//! Corporate actions as simulated events (ADR-135 §6.8).
//!
//! A split is not a price edit. Rewriting history under a live position is how
//! a backtest quietly reports that a stop set at $40 was hit when the shares it
//! protected became worth $2 each on a 1:20 reverse split. So splits,
//! dividends, symbol changes and delistings are **events at their effective
//! time**, ordered on the same clock as every fill, adjusting the position and
//! the cash that exist at that instant.
//!
//! # The consistency rule
//!
//! Price history already carries some of these adjustments. A dataset built
//! under [`AdjustmentPolicy::SplitAdjusted`] has the split *in the prices*;
//! replaying a split event on top would apply it twice. A total-return series
//! has the dividend in the prices too. So a schedule is checked against the
//! dataset's declared adjustment policy and a double-counting combination is
//! **refused** — the engine does not silently pick one.
//!
//! # Ratios are integers
//!
//! `numerator:denominator` are integers, so a 3:2 split multiplies units by
//! exactly 3/2 and never by a decimal that has no exact binary form. A 1:20
//! reverse split is `1:20`. Both directions are the same node.
//!
//! # Honest limits
//!
//! - Fractional units survive a split. Real brokers pay cash in lieu of a
//!   fractional share at a price nobody records; keeping the fraction is exact
//!   arithmetic, and inventing the cash-in-lieu price would not be.
//! - A symbol change has no economic effect and does not re-key the symbol
//!   table: a run's symbol set is fixed when its datasets are bound. The event
//!   is validated against the stream it names and recorded, so a report shows
//!   the identity change; a dataset that spans both identities under one symbol
//!   is the data layer's job, not the simulator's.

use crate::core::strategy_dataset::AdjustmentPolicy;

/// Bumped whenever the meaning of a stored schedule changes.
pub const CORPORATE_ACTION_SCHEMA_VERSION: u32 = 1;

/// Events one schedule may carry. Bounded because the schedule is operator
/// input that ends up in a content-addressed config.
pub const MAX_CORPORATE_ACTIONS: usize = 1_024;

/// Widest split ratio leg accepted. A 1:1000 reverse split is already extreme;
/// beyond this a ratio is far likelier to be a typo than a corporate action.
pub const MAX_SPLIT_RATIO: u32 = 10_000;

/// Longest symbol accepted in an action, matching the simulator's own bound.
pub const MAX_ACTION_SYMBOL_LEN: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CorporateActionError {
    TooManyActions {
        limit: usize,
        found: usize,
    },
    InvalidSymbol {
        symbol: String,
    },
    InvalidRatio {
        numerator: u32,
        denominator: u32,
    },
    /// A 1:1 split changes nothing and would hash into a run as a no-op event.
    IdentityRatio,
    NonFiniteAmount {
        field: &'static str,
    },
    NonPositiveDividend,
    /// A rename to the same symbol says nothing.
    IdentitySymbolChange {
        symbol: String,
    },
    OutOfOrder {
        index: usize,
    },
    DuplicateAction {
        index: usize,
    },
    /// The dataset already carries this adjustment in its prices.
    DoubleCounted {
        symbol: String,
        action: &'static str,
        adjustment: AdjustmentPolicy,
    },
}

impl std::fmt::Display for CorporateActionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooManyActions { limit, found } => write!(
                formatter,
                "{found} corporate actions exceeds the limit of {limit}"
            ),
            Self::InvalidSymbol { symbol } => {
                write!(formatter, "invalid corporate-action symbol `{symbol}`")
            }
            Self::InvalidRatio {
                numerator,
                denominator,
            } => write!(
                formatter,
                "split ratio {numerator}:{denominator} is outside the accepted range"
            ),
            Self::IdentityRatio => formatter.write_str("a 1:1 split has no effect"),
            Self::NonFiniteAmount { field } => {
                write!(formatter, "corporate-action `{field}` is not finite")
            }
            Self::NonPositiveDividend => {
                formatter.write_str("dividend amount must be strictly positive")
            }
            Self::IdentitySymbolChange { symbol } => {
                write!(formatter, "symbol change to the same symbol `{symbol}`")
            }
            Self::OutOfOrder { index } => write!(
                formatter,
                "corporate action {index} is not in canonical (time, symbol, kind) order"
            ),
            Self::DuplicateAction { index } => {
                write!(formatter, "duplicate corporate action at index {index}")
            }
            Self::DoubleCounted {
                symbol,
                action,
                adjustment,
            } => write!(
                formatter,
                "a `{action}` event on `{symbol}` double-counts an adjustment already in `{}` prices",
                adjustment.wire_id()
            ),
        }
    }
}

impl std::error::Error for CorporateActionError {}

/// What happened to the instrument.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CorporateActionKind {
    /// `numerator` new units for every `denominator` old ones. A forward 2:1 is
    /// `2:1`; a 1:20 reverse split is `1:20`.
    Split { numerator: u32, denominator: u32 },
    /// Cash per unit held at the effective instant. Longs receive it, shorts
    /// pay it.
    CashDividend { amount_per_unit: f64 },
    /// The instrument's ticker changed. No economic effect.
    SymbolChange { new_symbol: String },
    /// Trading ends. Any open position is closed at the last committed mark and
    /// nothing on the symbol trades afterwards.
    Delisting,
}

impl CorporateActionKind {
    pub const fn wire_id(&self) -> &'static str {
        match self {
            Self::Split { .. } => "split",
            Self::CashDividend { .. } => "cash_dividend",
            Self::SymbolChange { .. } => "symbol_change",
            Self::Delisting => "delisting",
        }
    }

    /// Ordering rank inside one `(time, symbol)`, so two actions at the same
    /// instant have exactly one legal order. A split resizes the position a
    /// dividend then pays on, and a delisting is always last.
    const fn order_rank(&self) -> u8 {
        match self {
            Self::Split { .. } => 0,
            Self::CashDividend { .. } => 1,
            Self::SymbolChange { .. } => 2,
            Self::Delisting => 3,
        }
    }

    /// Whether `adjustment` already carries this action inside its prices.
    const fn double_counts(&self, adjustment: AdjustmentPolicy) -> bool {
        match self {
            Self::Split { .. } => matches!(
                adjustment,
                AdjustmentPolicy::SplitAdjusted | AdjustmentPolicy::TotalReturn
            ),
            Self::CashDividend { .. } => matches!(adjustment, AdjustmentPolicy::TotalReturn),
            Self::SymbolChange { .. } | Self::Delisting => false,
        }
    }
}

/// One action, anchored to a UTC instant.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorporateAction {
    pub symbol: String,
    /// Effective instant, UTC nanoseconds — the same clock as every event.
    pub effective_time_ns: i64,
    pub kind: CorporateActionKind,
}

impl CorporateAction {
    /// Total order key. Never a hash-map order and never insertion order.
    fn order_key(&self) -> (i64, &str, u8) {
        (
            self.effective_time_ns,
            self.symbol.as_str(),
            self.kind.order_rank(),
        )
    }

    /// Position and average-entry multipliers for a split. Both are exact
    /// integer ratios, applied as one division so the product of the two stays
    /// the pre-split notional.
    pub fn split_factors(numerator: u32, denominator: u32) -> (f64, f64) {
        let units_factor = f64::from(numerator) / f64::from(denominator);
        let price_factor = f64::from(denominator) / f64::from(numerator);
        (units_factor, price_factor)
    }
}

/// A validated schedule in canonical order.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorporateActionSchedule {
    actions: Vec<CorporateAction>,
}

impl CorporateActionSchedule {
    /// The empty schedule: no corporate actions are modelled. Explicit rather
    /// than implied — a report says so.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Sort into canonical order and validate. Sorting is part of building, so
    /// two operators who listed the same actions in different orders produce
    /// the same config id.
    pub fn build(actions: &[CorporateAction]) -> Result<Self, CorporateActionError> {
        if actions.len() > MAX_CORPORATE_ACTIONS {
            return Err(CorporateActionError::TooManyActions {
                limit: MAX_CORPORATE_ACTIONS,
                found: actions.len(),
            });
        }
        let mut sorted = actions.to_vec();
        sorted.sort_by(|left, right| left.order_key().cmp(&right.order_key()));
        let schedule = Self { actions: sorted };
        schedule.validate()?;
        Ok(schedule)
    }

    pub fn actions(&self) -> &[CorporateAction] {
        &self.actions
    }

    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    /// Actions naming `symbol`, in canonical order.
    pub fn for_symbol<'a>(&'a self, symbol: &'a str) -> impl Iterator<Item = &'a CorporateAction> {
        self.actions
            .iter()
            .filter(move |action| action.symbol == symbol)
    }

    pub fn validate(&self) -> Result<(), CorporateActionError> {
        if self.actions.len() > MAX_CORPORATE_ACTIONS {
            return Err(CorporateActionError::TooManyActions {
                limit: MAX_CORPORATE_ACTIONS,
                found: self.actions.len(),
            });
        }
        for (index, action) in self.actions.iter().enumerate() {
            validate_symbol(&action.symbol)?;
            match &action.kind {
                CorporateActionKind::Split {
                    numerator,
                    denominator,
                } => {
                    if *numerator == 0
                        || *denominator == 0
                        || *numerator > MAX_SPLIT_RATIO
                        || *denominator > MAX_SPLIT_RATIO
                    {
                        return Err(CorporateActionError::InvalidRatio {
                            numerator: *numerator,
                            denominator: *denominator,
                        });
                    }
                    if numerator == denominator {
                        return Err(CorporateActionError::IdentityRatio);
                    }
                }
                CorporateActionKind::CashDividend { amount_per_unit } => {
                    if !amount_per_unit.is_finite() {
                        return Err(CorporateActionError::NonFiniteAmount {
                            field: "amount_per_unit",
                        });
                    }
                    if *amount_per_unit <= 0.0 {
                        return Err(CorporateActionError::NonPositiveDividend);
                    }
                }
                CorporateActionKind::SymbolChange { new_symbol } => {
                    validate_symbol(new_symbol)?;
                    if *new_symbol == action.symbol {
                        return Err(CorporateActionError::IdentitySymbolChange {
                            symbol: new_symbol.clone(),
                        });
                    }
                }
                CorporateActionKind::Delisting => {}
            }
            if index > 0 {
                let previous = self.actions[index - 1].order_key();
                let current = action.order_key();
                if previous > current {
                    return Err(CorporateActionError::OutOfOrder { index });
                }
                if previous == current {
                    return Err(CorporateActionError::DuplicateAction { index });
                }
            }
        }
        Ok(())
    }

    /// Refuse a schedule whose events are already baked into the price series.
    ///
    /// This is the §6.8 mutual-consistency requirement: the dataset's
    /// adjustment policy and the simulator's corporate-action handling must not
    /// both apply the same adjustment.
    pub fn check_adjustment_consistency(
        &self,
        adjustment: AdjustmentPolicy,
    ) -> Result<(), CorporateActionError> {
        for action in &self.actions {
            if action.kind.double_counts(adjustment) {
                return Err(CorporateActionError::DoubleCounted {
                    symbol: action.symbol.clone(),
                    action: action.kind.wire_id(),
                    adjustment,
                });
            }
        }
        Ok(())
    }
}

fn validate_symbol(symbol: &str) -> Result<(), CorporateActionError> {
    if symbol.is_empty()
        || symbol.trim() != symbol
        || symbol.chars().count() > MAX_ACTION_SYMBOL_LEN
        || symbol.chars().any(char::is_control)
    {
        return Err(CorporateActionError::InvalidSymbol {
            symbol: symbol.to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests;
