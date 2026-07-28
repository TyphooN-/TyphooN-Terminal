//! Time-accrued financing, borrow, funding and currency conversion
//! (ADR-135 §6.3).
//!
//! M1 deliberately charged nothing that accrues with *time* — only per-fill
//! commission, spread and slippage. This module is the missing half: what a
//! position costs for being held.
//!
//! # The rule that shapes everything here
//!
//! A rate that is not known is not zero. ADR-135 §14 records short borrow as
//! blocked on paid-only feeds, and the tempting shortcut — assume zero and move
//! on — silently reports a short book as free. So every rate is a
//! [`RateSource`] with three distinct meanings:
//!
//! - [`RateSource::Declared`] — a number, with the provenance that produced it.
//! - [`RateSource::NotApplicable`] — a *stated* zero. Funding on a cash equity
//!   genuinely does not exist, and saying so is different from not knowing.
//! - [`RateSource::Unavailable`] — no number exists. Exposure that would incur
//!   this charge makes the run **fail**, naming the charge and the reason.
//!
//! That third variant is the whole point: the engine refuses to produce a
//! number it cannot justify, rather than producing a flattering one.
//!
//! # Honest limits
//!
//! - Rates are constant across a run. A time-varying borrow or funding series
//!   is another dataset with its own provenance, and is not representable here
//!   rather than being faked by interpolation.
//! - Currency conversion is charged **per fill**: the account holds no foreign
//!   balance, so every trade round-trips the quote currency. That is the
//!   conservative reading, and it is stated rather than tuned.

/// Bumped whenever the meaning of a stored policy changes.
pub const FINANCING_POLICY_SCHEMA_VERSION: u32 = 1;

/// Currency rows one conversion table may declare.
pub const MAX_CURRENCY_RATES: usize = 32;

/// Longest note/source/date text accepted, so operator input cannot grow a
/// content-addressed config without bound.
pub const MAX_RATE_TEXT_LEN: usize = 256;

/// Widest annual percentage accepted. Rates outside this are far likelier to be
/// a units mistake (a fraction typed as a percent, or the reverse) than a real
/// funding cost, and a silent acceptance would compound that mistake daily.
pub const MAX_ANNUAL_PERCENT: f64 = 1_000.0;

/// Widest per-interval funding percentage accepted.
pub const MAX_INTERVAL_PERCENT: f64 = 100.0;

const SECONDS_PER_DAY: i64 = 86_400;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FinancingError {
    InvalidText {
        field: &'static str,
        reason: &'static str,
    },
    NonFiniteRate {
        field: &'static str,
    },
    RateOutOfRange {
        field: &'static str,
        limit: &'static str,
    },
    /// A borrow fee that pays the borrower is not a borrow fee.
    NegativeBorrowRate,
    InvalidAccrualInterval,
    TooManyCurrencyRates {
        limit: usize,
        found: usize,
    },
    DuplicateCurrency {
        currency: String,
    },
    CurrencyRatesOutOfOrder {
        currency: String,
    },
    /// A conversion rate must be a strictly positive price.
    NonPositiveConversionRate {
        currency: String,
    },
    /// The account currency needs no conversion row, and one would be a second
    /// definition of `1.0`.
    AccountCurrencyRateDeclared {
        currency: String,
    },
}

impl std::fmt::Display for FinancingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidText { field, reason } => {
                write!(formatter, "financing text `{field}` {reason}")
            }
            Self::NonFiniteRate { field } => write!(formatter, "rate `{field}` is not finite"),
            Self::RateOutOfRange { field, limit } => {
                write!(formatter, "rate `{field}` must be within {limit}")
            }
            Self::NegativeBorrowRate => {
                formatter.write_str("a short borrow rate may not be negative")
            }
            Self::InvalidAccrualInterval => {
                formatter.write_str("accrual interval must be a positive number of seconds")
            }
            Self::TooManyCurrencyRates { limit, found } => write!(
                formatter,
                "{found} currency rates exceeds the limit of {limit}"
            ),
            Self::DuplicateCurrency { currency } => {
                write!(formatter, "duplicate currency rate `{currency}`")
            }
            Self::CurrencyRatesOutOfOrder { currency } => {
                write!(
                    formatter,
                    "currency rate `{currency}` is out of canonical order"
                )
            }
            Self::NonPositiveConversionRate { currency } => write!(
                formatter,
                "conversion rate for `{currency}` must be strictly positive"
            ),
            Self::AccountCurrencyRateDeclared { currency } => write!(
                formatter,
                "the account currency `{currency}` may not declare a conversion rate"
            ),
        }
    }
}

impl std::error::Error for FinancingError {}

/// Where a declared number came from. Hashed into the config id, so two runs
/// that assumed different things can never share an identity.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RateProvenance {
    /// The operator chose this number. The note is required, and it is what a
    /// report prints beside the charge.
    OperatorAssumption { note: String },
    /// Published by a named source on a named date.
    VendorPublished {
        source: String,
        retrieved_date: String,
    },
}

impl RateProvenance {
    pub const fn wire_id(&self) -> &'static str {
        match self {
            Self::OperatorAssumption { .. } => "operator_assumption",
            Self::VendorPublished { .. } => "vendor_published",
        }
    }

    fn validate(&self) -> Result<(), FinancingError> {
        match self {
            Self::OperatorAssumption { note } => check_text("provenance.note", note),
            Self::VendorPublished {
                source,
                retrieved_date,
            } => {
                check_text("provenance.source", source)?;
                check_text("provenance.retrieved_date", retrieved_date)
            }
        }
    }
}

/// A rate, or an explicit statement about why there is not one.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RateSource {
    Declared {
        percent: f64,
        provenance: RateProvenance,
    },
    /// The charge does not exist for this instrument. A stated zero.
    NotApplicable,
    /// No rate is obtainable. Exposure that would incur this charge fails the
    /// run instead of accruing zero.
    Unavailable { reason: String },
}

impl RateSource {
    pub const fn wire_id(&self) -> &'static str {
        match self {
            Self::Declared { .. } => "declared",
            Self::NotApplicable => "not_applicable",
            Self::Unavailable { .. } => "unavailable",
        }
    }

    /// The percentage this source contributes, or `None` when it is
    /// unavailable — which the caller must turn into a refusal, never a zero.
    pub const fn percent(&self) -> Option<f64> {
        match self {
            Self::Declared { percent, .. } => Some(*percent),
            Self::NotApplicable => Some(0.0),
            Self::Unavailable { .. } => None,
        }
    }

    pub fn unavailable_reason(&self) -> Option<&str> {
        match self {
            Self::Unavailable { reason } => Some(reason.as_str()),
            _ => None,
        }
    }

    fn validate(
        &self,
        field: &'static str,
        limit: f64,
        allow_negative: bool,
    ) -> Result<(), FinancingError> {
        match self {
            Self::NotApplicable => Ok(()),
            Self::Unavailable { reason } => {
                check_text("rate.unavailable_reason", reason).map_err(|_| {
                    FinancingError::InvalidText {
                        field: "rate.unavailable_reason",
                        reason: "must be non-empty, trimmed, and within the length bound",
                    }
                })
            }
            Self::Declared {
                percent,
                provenance,
            } => {
                if !percent.is_finite() {
                    return Err(FinancingError::NonFiniteRate { field });
                }
                if !allow_negative && *percent < 0.0 {
                    return Err(FinancingError::NegativeBorrowRate);
                }
                if percent.abs() > limit {
                    return Err(FinancingError::RateOutOfRange {
                        field,
                        limit: "the declared percentage bound",
                    });
                }
                provenance.validate()
            }
        }
    }
}

/// How a year is measured when an annual rate is prorated to an interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DayCount {
    /// 365 fixed days. The conservative default: it prorates a given annual
    /// rate to the *smaller* daily charge, so it never inflates a cost that was
    /// quoted on a 360-day basis.
    #[default]
    Act365Fixed,
    /// 360 days, the money-market convention most US financing is quoted on.
    Act360,
}

impl DayCount {
    pub const fn wire_id(self) -> &'static str {
        match self {
            Self::Act365Fixed => "act_365_fixed",
            Self::Act360 => "act_360",
        }
    }

    pub const fn year_seconds(self) -> i64 {
        match self {
            Self::Act365Fixed => 365 * SECONDS_PER_DAY,
            Self::Act360 => 360 * SECONDS_PER_DAY,
        }
    }
}

/// When accrual boundaries fall. Every boundary is an event on the simulator's
/// one clock, so a charge is ordered against fills exactly like anything else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AccrualInterval {
    /// Midnight UTC. Deliberately not "the venue's day": a UTC boundary is a
    /// rule that can be stated exactly, and financing is charged by the broker
    /// on its own book, not on the exchange's session.
    UtcDaily,
    /// A fixed period from the UTC epoch — the crypto perpetual-funding shape,
    /// where the venue charges on a repeating clock (8 h is typical).
    FixedSeconds { seconds: u32 },
}

impl AccrualInterval {
    pub const fn wire_id(self) -> &'static str {
        match self {
            Self::UtcDaily => "utc_daily",
            Self::FixedSeconds { .. } => "fixed_seconds",
        }
    }

    pub const fn seconds(self) -> i64 {
        match self {
            Self::UtcDaily => SECONDS_PER_DAY,
            Self::FixedSeconds { seconds } => seconds as i64,
        }
    }

    fn validate(self) -> Result<(), FinancingError> {
        match self {
            Self::UtcDaily => Ok(()),
            Self::FixedSeconds { seconds } => (seconds > 0)
                .then_some(())
                .ok_or(FinancingError::InvalidAccrualInterval),
        }
    }
}

/// The four §6.3 time-accrued charges, each with its own provenance.
///
/// Every field's *name* states its unit; [`RateSource`] states where the number
/// came from. Keeping those separate is what makes "we assumed 4 %" and "the
/// venue publishes 4 %" different run identities.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FinancingPolicy {
    pub day_count: DayCount,
    pub accrual: AccrualInterval,
    /// Charged on the marked value of a long position held across a boundary.
    /// Negative is representable: financing rates have been.
    pub long_financing_annual_percent: RateSource,
    /// Charged on the marked value of a short position. Positive is a debit;
    /// a negative rate credits the short rebate.
    pub short_financing_annual_percent: RateSource,
    /// Stock-loan fee on a short. Never negative — nobody pays you to borrow.
    /// This is ADR-135 §14's blocked feed, so `Unavailable` is the honest state
    /// until an operator declares an assumption.
    pub short_borrow_annual_percent: RateSource,
    /// Perpetual funding, quoted **per accrual interval**, not annualized.
    /// Positive means longs pay shorts, the usual venue convention.
    pub funding_interval_percent: RateSource,
}

impl FinancingPolicy {
    /// A cash-equity shape: financing and borrow must be declared by the
    /// operator, funding genuinely does not apply.
    pub fn cash_equity_unfunded() -> Self {
        Self {
            day_count: DayCount::Act365Fixed,
            accrual: AccrualInterval::UtcDaily,
            long_financing_annual_percent: RateSource::NotApplicable,
            short_financing_annual_percent: RateSource::NotApplicable,
            short_borrow_annual_percent: RateSource::Unavailable {
                reason: "borrow-rate feeds are paid-only (ADR-135 §14)".to_string(),
            },
            funding_interval_percent: RateSource::NotApplicable,
        }
    }

    pub fn validate(&self) -> Result<(), FinancingError> {
        self.accrual.validate()?;
        self.long_financing_annual_percent.validate(
            "long_financing_annual_percent",
            MAX_ANNUAL_PERCENT,
            true,
        )?;
        self.short_financing_annual_percent.validate(
            "short_financing_annual_percent",
            MAX_ANNUAL_PERCENT,
            true,
        )?;
        self.short_borrow_annual_percent.validate(
            "short_borrow_annual_percent",
            MAX_ANNUAL_PERCENT,
            false,
        )?;
        self.funding_interval_percent.validate(
            "funding_interval_percent",
            MAX_INTERVAL_PERCENT,
            true,
        )
    }

    /// The first accrual boundary strictly after `time_ns`.
    pub fn next_boundary_ns(&self, time_ns: i64) -> Option<i64> {
        let period = self.accrual.seconds().checked_mul(1_000_000_000)?;
        if period <= 0 {
            return None;
        }
        let index = time_ns.div_euclid(period);
        index.checked_add(1)?.checked_mul(period)
    }
}

/// Whether a run models time-accrued charges at all.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FinancingModel {
    /// Nothing accrues. Valid, stamped on the run, and never presented as
    /// realistic for a leveraged or short book.
    #[default]
    None,
    Accrued(FinancingPolicy),
}

impl FinancingModel {
    pub const fn wire_id(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Accrued(_) => "accrued",
        }
    }

    pub const fn policy(&self) -> Option<&FinancingPolicy> {
        match self {
            Self::None => None,
            Self::Accrued(policy) => Some(policy),
        }
    }

    pub fn validate(&self) -> Result<(), FinancingError> {
        match self {
            Self::None => Ok(()),
            Self::Accrued(policy) => policy.validate(),
        }
    }
}

/// Which charge a refusal is about, so an error names the missing rate exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinancingCharge {
    LongFinancing,
    ShortFinancing,
    ShortBorrow,
    Funding,
}

impl FinancingCharge {
    pub const fn wire_id(self) -> &'static str {
        match self {
            Self::LongFinancing => "long_financing",
            Self::ShortFinancing => "short_financing",
            Self::ShortBorrow => "short_borrow",
            Self::Funding => "funding",
        }
    }
}

/// One boundary's charges, split so a report can attribute them.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AccrualBreakdown {
    /// Positive is a debit against cash.
    pub financing: f64,
    pub borrow: f64,
    pub funding: f64,
}

impl AccrualBreakdown {
    pub fn total(self) -> f64 {
        self.financing + self.borrow + self.funding
    }
}

/// Compute one boundary's charges for a signed position marked at `mark`.
///
/// `elapsed_seconds` is the time since the previous boundary, so a position
/// opened mid-interval is charged for the interval it was held across — the
/// venue charges on the snapshot, and modelling anything finer would need
/// intraday balances the ledger does not carry.
///
/// Returns the charge that has no rate, rather than a number, when the exposure
/// needs a rate the policy calls [`RateSource::Unavailable`].
pub fn accrue(
    policy: &FinancingPolicy,
    units: f64,
    mark: f64,
    elapsed_seconds: i64,
) -> Result<AccrualBreakdown, FinancingCharge> {
    if units == 0.0 || elapsed_seconds <= 0 || !mark.is_finite() || mark <= 0.0 {
        return Ok(AccrualBreakdown::default());
    }
    let notional = units.abs() * mark;
    let year_fraction = elapsed_seconds as f64 / policy.day_count.year_seconds() as f64;

    let (financing_rate, charge) = if units > 0.0 {
        (
            &policy.long_financing_annual_percent,
            FinancingCharge::LongFinancing,
        )
    } else {
        (
            &policy.short_financing_annual_percent,
            FinancingCharge::ShortFinancing,
        )
    };
    let financing_percent = financing_rate.percent().ok_or(charge)?;
    let financing = notional * financing_percent / 100.0 * year_fraction;

    let borrow = if units < 0.0 {
        let percent = policy
            .short_borrow_annual_percent
            .percent()
            .ok_or(FinancingCharge::ShortBorrow)?;
        notional * percent / 100.0 * year_fraction
    } else {
        0.0
    };

    // Funding is quoted per interval, so it is not prorated by day count: a
    // position held across the boundary pays the whole interval's rate, which
    // is how a perpetual venue charges it. Longs pay a positive rate.
    let funding_percent = policy
        .funding_interval_percent
        .percent()
        .ok_or(FinancingCharge::Funding)?;
    let funding = units.signum() * notional * funding_percent / 100.0;

    Ok(AccrualBreakdown {
        financing,
        borrow,
        funding,
    })
}

// ── Currency conversion ────────────────────────────────────────────

/// One instrument currency's conversion into the account currency.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CurrencyRate {
    pub currency: String,
    /// Units of account currency per one unit of `currency`.
    pub account_per_unit: f64,
    /// Conversion cost charged on the absolute converted amount, per fill.
    pub spread_percent: f64,
    pub provenance: RateProvenance,
}

/// How instrument currencies reach the account currency.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CurrencyConversion {
    /// Every instrument is quoted in the account currency. An instrument that
    /// is not is **rejected**, rather than converted at an invented rate of 1.
    #[default]
    None,
    /// A declared, identity-bearing rate table. Constant for the run.
    Declared { rates: Vec<CurrencyRate> },
}

impl CurrencyConversion {
    pub const fn wire_id(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Declared { .. } => "declared",
        }
    }

    pub fn rates(&self) -> &[CurrencyRate] {
        match self {
            Self::None => &[],
            Self::Declared { rates } => rates,
        }
    }

    /// The conversion for `currency`, or `None` when the table does not carry
    /// it. The account currency itself always converts one-for-one and free.
    pub fn lookup(&self, currency: &str, account_currency: &str) -> Option<(f64, f64)> {
        if currency == account_currency {
            return Some((1.0, 0.0));
        }
        self.rates()
            .iter()
            .find(|rate| rate.currency == currency)
            .map(|rate| (rate.account_per_unit, rate.spread_percent))
    }

    pub fn validate(&self, account_currency: &str) -> Result<(), FinancingError> {
        let rates = self.rates();
        if rates.len() > MAX_CURRENCY_RATES {
            return Err(FinancingError::TooManyCurrencyRates {
                limit: MAX_CURRENCY_RATES,
                found: rates.len(),
            });
        }
        for (index, rate) in rates.iter().enumerate() {
            check_text("currency_rate.currency", &rate.currency).map_err(|_| {
                FinancingError::InvalidText {
                    field: "currency_rate.currency",
                    reason: "must be non-empty, trimmed, and within the length bound",
                }
            })?;
            if rate.currency == account_currency {
                return Err(FinancingError::AccountCurrencyRateDeclared {
                    currency: rate.currency.clone(),
                });
            }
            if !rate.account_per_unit.is_finite() || rate.account_per_unit <= 0.0 {
                return Err(FinancingError::NonPositiveConversionRate {
                    currency: rate.currency.clone(),
                });
            }
            if !rate.spread_percent.is_finite()
                || rate.spread_percent < 0.0
                || rate.spread_percent > MAX_INTERVAL_PERCENT
            {
                return Err(FinancingError::RateOutOfRange {
                    field: "currency_rate.spread_percent",
                    limit: "[0, 100]",
                });
            }
            rate.provenance.validate()?;
            if index > 0 {
                let previous = &rates[index - 1].currency;
                if previous == &rate.currency {
                    return Err(FinancingError::DuplicateCurrency {
                        currency: rate.currency.clone(),
                    });
                }
                if previous.as_str() > rate.currency.as_str() {
                    return Err(FinancingError::CurrencyRatesOutOfOrder {
                        currency: rate.currency.clone(),
                    });
                }
            }
        }
        Ok(())
    }
}

fn check_text(field: &'static str, value: &str) -> Result<(), FinancingError> {
    if value.is_empty()
        || value.trim() != value
        || value.chars().count() > MAX_RATE_TEXT_LEN
        || value.chars().any(char::is_control)
    {
        return Err(FinancingError::InvalidText {
            field,
            reason: "must be non-empty, trimmed, control-free, and within the length bound",
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests;
