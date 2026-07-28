//! Per-instrument execution specifications (ADR-135 §6.3, §6.6, §6.7).
//!
//! §6.7 asks for *per-instrument* trading calendars and §6.3 for costs that
//! depend on what the instrument is — a crypto perpetual funds, a cash equity
//! borrows, a foreign-listed share converts. Those are properties of the
//! instrument, not of the run, so they live here rather than as one global
//! setting that would be wrong for every symbol but one.
//!
//! The registry is **optional and empty by default**. An empty registry means
//! no calendar gating, no accruals and no conversion — the M1 behaviour,
//! unchanged. A symbol that is not in the registry is likewise ungated, and the
//! report says how many symbols were specified, so "we forgot to add it" is
//! visible rather than silent.

use crate::core::strategy_calendar::{CalendarError, TradingCalendar};
use crate::core::strategy_financing::{FinancingError, FinancingModel};

/// Instruments one registry may carry. Bounded to the simulator's own symbol
/// ceiling: a spec for a symbol no run can hold is dead weight in a config id.
pub const MAX_INSTRUMENT_SPECS: usize = 256;

/// Longest symbol and currency accepted.
pub const MAX_INSTRUMENT_TEXT_LEN: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstrumentError {
    TooManySpecs {
        limit: usize,
        found: usize,
    },
    InvalidText {
        field: &'static str,
        value: String,
    },
    DuplicateSymbol {
        symbol: String,
    },
    OutOfOrder {
        symbol: String,
    },
    InvalidCalendar {
        symbol: String,
        source: CalendarError,
    },
    InvalidFinancing {
        symbol: String,
        source: FinancingError,
    },
    NonPositiveTick {
        symbol: String,
    },
    /// The instrument is quoted in a currency the run cannot convert.
    UnconvertibleCurrency {
        symbol: String,
        currency: String,
    },
}

impl std::fmt::Display for InstrumentError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooManySpecs { limit, found } => write!(
                formatter,
                "{found} instrument specs exceeds the limit of {limit}"
            ),
            Self::InvalidText { field, value } => {
                write!(formatter, "invalid instrument `{field}` value `{value}`")
            }
            Self::DuplicateSymbol { symbol } => {
                write!(formatter, "duplicate instrument spec for `{symbol}`")
            }
            Self::OutOfOrder { symbol } => write!(
                formatter,
                "instrument spec `{symbol}` is out of canonical order"
            ),
            Self::InvalidCalendar { symbol, source } => {
                write!(formatter, "instrument `{symbol}` calendar: {source}")
            }
            Self::InvalidFinancing { symbol, source } => {
                write!(formatter, "instrument `{symbol}` financing: {source}")
            }
            Self::NonPositiveTick { symbol } => write!(
                formatter,
                "instrument `{symbol}` price tick must be strictly positive"
            ),
            Self::UnconvertibleCurrency { symbol, currency } => write!(
                formatter,
                "instrument `{symbol}` is quoted in `{currency}`, which the run declares no conversion for"
            ),
        }
    }
}

impl std::error::Error for InstrumentError {}

/// Everything the execution layer needs to know about one instrument.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstrumentSpec {
    pub symbol: String,
    /// Currency the instrument's prices are quoted in. Equal to the account
    /// currency means no conversion; anything else needs a declared rate.
    pub currency: String,
    /// Trading calendar. `None` means the instrument is never session-gated,
    /// which is honest for a venue whose calendar has not been established.
    pub calendar: Option<TradingCalendar>,
    pub financing: FinancingModel,
    /// Price lattice for this instrument, overriding the run-wide tick.
    pub price_tick: Option<f64>,
}

impl InstrumentSpec {
    /// A spec that gates nothing and charges nothing — the shape of "we know
    /// the symbol trades in the account currency and nothing else".
    pub fn plain(symbol: &str, currency: &str) -> Self {
        Self {
            symbol: symbol.to_string(),
            currency: currency.to_string(),
            calendar: None,
            financing: FinancingModel::None,
            price_tick: None,
        }
    }

    #[must_use]
    pub fn with_calendar(mut self, calendar: TradingCalendar) -> Self {
        self.calendar = Some(calendar);
        self
    }

    #[must_use]
    pub fn with_financing(mut self, financing: FinancingModel) -> Self {
        self.financing = financing;
        self
    }

    #[must_use]
    pub fn with_price_tick(mut self, tick: f64) -> Self {
        self.price_tick = Some(tick);
        self
    }
}

/// A validated set of instrument specs in canonical symbol order.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstrumentRegistry {
    specs: Vec<InstrumentSpec>,
}

impl InstrumentRegistry {
    /// No instrument is specified. Calendars, accruals and conversion are all
    /// inactive, which is exactly the M1 model.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Sort into canonical order and validate. Sorting is part of building, so
    /// declaration order cannot change a config id.
    pub fn build(specs: &[InstrumentSpec]) -> Result<Self, InstrumentError> {
        if specs.len() > MAX_INSTRUMENT_SPECS {
            return Err(InstrumentError::TooManySpecs {
                limit: MAX_INSTRUMENT_SPECS,
                found: specs.len(),
            });
        }
        let mut sorted = specs.to_vec();
        sorted.sort_by(|left, right| left.symbol.cmp(&right.symbol));
        let registry = Self { specs: sorted };
        registry.validate_shape()?;
        Ok(registry)
    }

    pub fn specs(&self) -> &[InstrumentSpec] {
        &self.specs
    }

    pub fn is_empty(&self) -> bool {
        self.specs.is_empty()
    }

    /// The spec for `symbol`, by binary search over the canonical order.
    pub fn get(&self, symbol: &str) -> Option<&InstrumentSpec> {
        self.specs
            .binary_search_by(|spec| spec.symbol.as_str().cmp(symbol))
            .ok()
            .map(|index| &self.specs[index])
    }

    /// Shape-only validation: text, ordering, ticks, calendars and financing.
    /// Currency reachability needs the run's conversion table and is checked by
    /// [`Self::validate_against_account`].
    pub fn validate_shape(&self) -> Result<(), InstrumentError> {
        if self.specs.len() > MAX_INSTRUMENT_SPECS {
            return Err(InstrumentError::TooManySpecs {
                limit: MAX_INSTRUMENT_SPECS,
                found: self.specs.len(),
            });
        }
        for (index, spec) in self.specs.iter().enumerate() {
            check_text("symbol", &spec.symbol)?;
            check_text("currency", &spec.currency)?;
            if index > 0 {
                let previous = self.specs[index - 1].symbol.as_str();
                if previous == spec.symbol {
                    return Err(InstrumentError::DuplicateSymbol {
                        symbol: spec.symbol.clone(),
                    });
                }
                if previous > spec.symbol.as_str() {
                    return Err(InstrumentError::OutOfOrder {
                        symbol: spec.symbol.clone(),
                    });
                }
            }
            if let Some(tick) = spec.price_tick
                && (!tick.is_finite() || tick <= 0.0)
            {
                return Err(InstrumentError::NonPositiveTick {
                    symbol: spec.symbol.clone(),
                });
            }
            if let Some(calendar) = &spec.calendar {
                TradingCalendar::build(calendar.spec()).map_err(|source| {
                    InstrumentError::InvalidCalendar {
                        symbol: spec.symbol.clone(),
                        source,
                    }
                })?;
            }
            spec.financing
                .validate()
                .map_err(|source| InstrumentError::InvalidFinancing {
                    symbol: spec.symbol.clone(),
                    source,
                })?;
        }
        Ok(())
    }

    /// Prove every instrument's currency reaches the account currency. A
    /// missing rate is a refusal, never an assumed 1.0.
    pub fn validate_against_account(
        &self,
        account_currency: &str,
        conversion: &crate::core::strategy_financing::CurrencyConversion,
    ) -> Result<(), InstrumentError> {
        for spec in &self.specs {
            if conversion
                .lookup(&spec.currency, account_currency)
                .is_none()
            {
                return Err(InstrumentError::UnconvertibleCurrency {
                    symbol: spec.symbol.clone(),
                    currency: spec.currency.clone(),
                });
            }
        }
        Ok(())
    }
}

fn check_text(field: &'static str, value: &str) -> Result<(), InstrumentError> {
    if value.is_empty()
        || value.trim() != value
        || value.chars().count() > MAX_INSTRUMENT_TEXT_LEN
        || value.chars().any(char::is_control)
    {
        return Err(InstrumentError::InvalidText {
            field,
            value: value.to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests;
