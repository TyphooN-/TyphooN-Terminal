//! Versioned venue fee schedules (ADR-135 §6.3).
//!
//! A backtest that charges "some commission" is not reproducible: the number
//! has to be attributable. This module makes a fee schedule an *artifact* —
//! a venue, a schedule version, an effective date, a provenance statement, and
//! a rate table — that an [`ExecutionSettings`](crate::core::strategy_ir::ExecutionSettings)
//! binds by value, so a run performed today can be re-derived years later
//! without asking what the rates "currently" are.
//!
//! ## What this module asserts, and what it does not
//!
//! It asserts the **shape** of each supported venue's published fee model:
//!
//! - [`FeeScheduleShape::KrakenSpot`] — percentage-of-notional maker and taker
//!   rates, banded by rolling 30-day traded volume. Which band and which side
//!   of the book applies is an operator assumption, because a backtest has no
//!   account history and cannot know whether a given order would have rested.
//! - [`FeeScheduleShape::AlpacaUsEquity`] — a per-share base commission plus
//!   regulatory pass-throughs that apply to **sells only** (a percentage of
//!   sell notional, and a per-share charge capped per order).
//!
//! It does **not** assert any venue's current rates. This checkout has no
//! network access and no account entitlement, so no primary source could be
//! read; every constructor therefore takes its numbers from the caller and
//! demands a [`FeeProvenance`] that says where they came from.
//! [`FeeProvenance::OperatorAssumption`] is the honest default and is what the
//! test corpus uses. [`FeeProvenance::VendorPublished`] exists for an operator
//! who has actually read a primary source, and it requires that source to be
//! named — it is never constructed by this crate.
//!
//! Because the schedule is bound by value and hashed into the execution
//! config's identity, changing a rate, a tier, a side assumption, an effective
//! date, or the provenance note produces a *different* config id. Historical
//! runs stay reproducible; they do not silently re-price.
//!
//! ## Bounds and arithmetic
//!
//! Tier tables are capped at [`MAX_FEE_TIERS`] and must be strictly ordered by
//! their lower volume bound, so exactly one band matches any volume. Rates are
//! validated finite and within `[0, 100]` percent at construction, and
//! [`FeeScheduleBinding::charge`] returns `0.0` rather than a `NaN` when handed
//! a non-finite or non-positive trade — a fee is never allowed to become the
//! reason an accounting invariant breaks.

use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;

/// Wire version of the fee-schedule artifact.
pub const FEE_SCHEDULE_SCHEMA_VERSION: u32 = 1;

/// Largest tier table a schedule may declare.
pub const MAX_FEE_TIERS: usize = 16;

/// Longest provenance/source text a schedule may carry.
pub const MAX_FEE_TEXT_LEN: usize = 256;

// ── Errors ─────────────────────────────────────────────────────────

/// Everything that can go wrong building or binding a fee schedule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeeScheduleError {
    /// A tiered shape declared no tier, which would silently charge nothing.
    EmptyTiers,
    /// The tier table exceeded [`MAX_FEE_TIERS`].
    TooManyTiers { limit: usize, found: usize },
    /// Tier `index` does not start strictly above its predecessor, so two
    /// bands could match one volume.
    UnorderedTiers { index: usize },
    /// A binding selected a tier the schedule does not declare.
    UnknownTier { index: usize, count: usize },
    /// A rate is not finite, so it has no canonical encoding.
    NonFiniteValue { field: &'static str },
    /// A rate is outside the range its field permits.
    OutOfRange {
        field: &'static str,
        expected: &'static str,
    },
    /// A provenance/source string has no unambiguous canonical form.
    InvalidText {
        field: &'static str,
        reason: &'static str,
    },
    /// The effective date is not a real `YYYY-MM-DD` calendar day.
    InvalidEffectiveDate { value: String },
}

impl fmt::Display for FeeScheduleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyTiers => f.write_str("fee schedule declares no tier"),
            Self::TooManyTiers { limit, found } => {
                write!(f, "fee schedule declares {found} tiers, limit {limit}")
            }
            Self::UnorderedTiers { index } => {
                write!(f, "fee tier {index} does not start above the previous band")
            }
            Self::UnknownTier { index, count } => {
                write!(f, "fee tier {index} is outside the {count} declared")
            }
            Self::NonFiniteValue { field } => {
                write!(f, "fee field `{field}` is not finite")
            }
            Self::OutOfRange { field, expected } => {
                write!(f, "fee field `{field}` must be {expected}")
            }
            Self::InvalidText { field, reason } => {
                write!(f, "fee field `{field}` is invalid: {reason}")
            }
            Self::InvalidEffectiveDate { value } => {
                write!(
                    f,
                    "effective date `{value}` is not a YYYY-MM-DD calendar day"
                )
            }
        }
    }
}
impl Error for FeeScheduleError {}

// ── Venue, provenance, and tiers ───────────────────────────────────

/// A venue whose fee model this crate can express. Scope is Kraken + Alpaca
/// per [ADR-111](../../../docs/adr/111-broker-scope-reduction-kraken-alpaca-only.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeeVenue {
    KrakenSpot,
    AlpacaUsEquity,
}

impl FeeVenue {
    /// Stable identifier used in canonical encodings and reports.
    pub const fn wire_id(self) -> &'static str {
        match self {
            Self::KrakenSpot => "kraken_spot",
            Self::AlpacaUsEquity => "alpaca_us_equity",
        }
    }
}

/// Where a schedule's numbers came from. This is not decoration: a rate with
/// no stated origin is a guess, and a guess must not read as a venue fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeeProvenance {
    /// The operator supplied these numbers as an explicit assumption. `note`
    /// records what was assumed and why.
    OperatorAssumption { note: String },
    /// The operator transcribed these numbers from a primary published
    /// schedule. Constructing this asserts that transcription happened; the
    /// source document and the date it was read must both be named.
    VendorPublished {
        source: String,
        retrieved_date: String,
    },
}

impl FeeProvenance {
    /// Stable identifier used in canonical encodings and reports.
    pub const fn wire_tag(&self) -> &'static str {
        match self {
            Self::OperatorAssumption { .. } => "operator_assumption",
            Self::VendorPublished { .. } => "vendor_published",
        }
    }

    fn validate(&self) -> Result<(), FeeScheduleError> {
        match self {
            Self::OperatorAssumption { note } => check_text("provenance.note", note),
            Self::VendorPublished {
                source,
                retrieved_date,
            } => {
                check_text("provenance.source", source)?;
                check_calendar_date("provenance.retrieved_date", retrieved_date)
            }
        }
    }
}

/// One volume band of a tiered maker/taker schedule.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VolumeTier {
    /// Inclusive lower bound of the rolling-volume band, in account currency.
    pub min_volume: f64,
    pub maker_percent: f64,
    pub taker_percent: f64,
}

/// Which side of the book a run assumes its orders land on. A bar-resolution
/// backtest cannot know, so it must be declared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiquidityAssumption {
    Maker,
    Taker,
}

impl LiquidityAssumption {
    /// Stable identifier used in canonical encodings and reports.
    pub const fn wire_id(self) -> &'static str {
        match self {
            Self::Maker => "maker",
            Self::Taker => "taker",
        }
    }
}

/// Which side of a trade a fee is being charged on. Local to this module so
/// the fee layer does not depend on the simulator's order model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeeSide {
    Buy,
    Sell,
}

// ── Schedule shapes ────────────────────────────────────────────────

/// The rate table of a venue, in that venue's published *shape*.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeeScheduleShape {
    /// Percentage-of-notional maker/taker rates banded by rolling volume.
    KrakenSpot { tiers: Vec<VolumeTier> },
    /// A per-share base commission plus sell-only regulatory pass-throughs.
    AlpacaUsEquity {
        /// Charged on every fill, both sides.
        per_share: f64,
        /// Floor applied to the per-share base, both sides.
        minimum: f64,
        /// Percentage of sell notional (the SEC Section 31 shape).
        sell_notional_percent: f64,
        /// Per-share charge on sells (the FINRA TAF shape).
        sell_per_share: f64,
        /// Cap applied to `sell_per_share` for one order.
        sell_per_order_cap: f64,
    },
}

impl FeeScheduleShape {
    /// Stable identifier used in canonical encodings and reports.
    pub const fn wire_tag(&self) -> &'static str {
        match self {
            Self::KrakenSpot { .. } => "kraken_spot",
            Self::AlpacaUsEquity { .. } => "alpaca_us_equity",
        }
    }

    /// How many tiers a binding may select between.
    pub fn tier_count(&self) -> usize {
        match self {
            Self::KrakenSpot { tiers } => tiers.len(),
            // The equity shape is flat; it still has exactly one selectable
            // slot so bindings share one code path.
            Self::AlpacaUsEquity { .. } => 1,
        }
    }

    fn venue(&self) -> FeeVenue {
        match self {
            Self::KrakenSpot { .. } => FeeVenue::KrakenSpot,
            Self::AlpacaUsEquity { .. } => FeeVenue::AlpacaUsEquity,
        }
    }

    fn validate(&self) -> Result<(), FeeScheduleError> {
        match self {
            Self::KrakenSpot { tiers } => {
                if tiers.is_empty() {
                    return Err(FeeScheduleError::EmptyTiers);
                }
                if tiers.len() > MAX_FEE_TIERS {
                    return Err(FeeScheduleError::TooManyTiers {
                        limit: MAX_FEE_TIERS,
                        found: tiers.len(),
                    });
                }
                for (index, tier) in tiers.iter().enumerate() {
                    check_non_negative(tier_field(index, "min_volume"), tier.min_volume)?;
                    check_percent(tier_field(index, "maker_percent"), tier.maker_percent)?;
                    check_percent(tier_field(index, "taker_percent"), tier.taker_percent)?;
                    if index > 0 && tier.min_volume <= tiers[index - 1].min_volume {
                        return Err(FeeScheduleError::UnorderedTiers { index });
                    }
                }
                Ok(())
            }
            Self::AlpacaUsEquity {
                per_share,
                minimum,
                sell_notional_percent,
                sell_per_share,
                sell_per_order_cap,
            } => {
                check_non_negative("alpaca_us_equity.per_share", *per_share)?;
                check_non_negative("alpaca_us_equity.minimum", *minimum)?;
                check_percent(
                    "alpaca_us_equity.sell_notional_percent",
                    *sell_notional_percent,
                )?;
                check_non_negative("alpaca_us_equity.sell_per_share", *sell_per_share)?;
                check_non_negative("alpaca_us_equity.sell_per_order_cap", *sell_per_order_cap)
            }
        }
    }
}

/// A tier field name, resolved to a `&'static str` so errors stay allocation-free
/// for the bounded tier table.
fn tier_field(index: usize, field: &'static str) -> &'static str {
    const MAKER: [&str; MAX_FEE_TIERS] = [
        "kraken_spot.tiers[0].maker_percent",
        "kraken_spot.tiers[1].maker_percent",
        "kraken_spot.tiers[2].maker_percent",
        "kraken_spot.tiers[3].maker_percent",
        "kraken_spot.tiers[4].maker_percent",
        "kraken_spot.tiers[5].maker_percent",
        "kraken_spot.tiers[6].maker_percent",
        "kraken_spot.tiers[7].maker_percent",
        "kraken_spot.tiers[8].maker_percent",
        "kraken_spot.tiers[9].maker_percent",
        "kraken_spot.tiers[10].maker_percent",
        "kraken_spot.tiers[11].maker_percent",
        "kraken_spot.tiers[12].maker_percent",
        "kraken_spot.tiers[13].maker_percent",
        "kraken_spot.tiers[14].maker_percent",
        "kraken_spot.tiers[15].maker_percent",
    ];
    const TAKER: [&str; MAX_FEE_TIERS] = [
        "kraken_spot.tiers[0].taker_percent",
        "kraken_spot.tiers[1].taker_percent",
        "kraken_spot.tiers[2].taker_percent",
        "kraken_spot.tiers[3].taker_percent",
        "kraken_spot.tiers[4].taker_percent",
        "kraken_spot.tiers[5].taker_percent",
        "kraken_spot.tiers[6].taker_percent",
        "kraken_spot.tiers[7].taker_percent",
        "kraken_spot.tiers[8].taker_percent",
        "kraken_spot.tiers[9].taker_percent",
        "kraken_spot.tiers[10].taker_percent",
        "kraken_spot.tiers[11].taker_percent",
        "kraken_spot.tiers[12].taker_percent",
        "kraken_spot.tiers[13].taker_percent",
        "kraken_spot.tiers[14].taker_percent",
        "kraken_spot.tiers[15].taker_percent",
    ];
    const VOLUME: [&str; MAX_FEE_TIERS] = [
        "kraken_spot.tiers[0].min_volume",
        "kraken_spot.tiers[1].min_volume",
        "kraken_spot.tiers[2].min_volume",
        "kraken_spot.tiers[3].min_volume",
        "kraken_spot.tiers[4].min_volume",
        "kraken_spot.tiers[5].min_volume",
        "kraken_spot.tiers[6].min_volume",
        "kraken_spot.tiers[7].min_volume",
        "kraken_spot.tiers[8].min_volume",
        "kraken_spot.tiers[9].min_volume",
        "kraken_spot.tiers[10].min_volume",
        "kraken_spot.tiers[11].min_volume",
        "kraken_spot.tiers[12].min_volume",
        "kraken_spot.tiers[13].min_volume",
        "kraken_spot.tiers[14].min_volume",
        "kraken_spot.tiers[15].min_volume",
    ];
    let table = match field {
        "maker_percent" => &MAKER,
        "taker_percent" => &TAKER,
        _ => &VOLUME,
    };
    table.get(index).copied().unwrap_or("kraken_spot.tiers")
}

// ── Schedule ───────────────────────────────────────────────────────

/// A validated, versioned, attributable rate table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeeSchedule {
    schema_version: u32,
    venue: FeeVenue,
    schedule_version: u32,
    effective_date: String,
    provenance: FeeProvenance,
    shape: FeeScheduleShape,
}

impl FeeSchedule {
    /// Validate a rate table and seal it with its venue, version, effective
    /// date and provenance.
    ///
    /// `effective_date` is the day the schedule the numbers describe took
    /// effect, as `YYYY-MM-DD`. It is not read from the clock: a schedule is
    /// data, and a run that pins one stays reproducible.
    pub fn build(
        venue: FeeVenue,
        schedule_version: u32,
        effective_date: &str,
        provenance: FeeProvenance,
        shape: FeeScheduleShape,
    ) -> Result<Self, FeeScheduleError> {
        if schedule_version == 0 {
            return Err(FeeScheduleError::OutOfRange {
                field: "schedule_version",
                expected: "a version of at least 1",
            });
        }
        check_calendar_date("effective_date", effective_date)?;
        provenance.validate()?;
        shape.validate()?;
        if shape.venue() != venue {
            return Err(FeeScheduleError::OutOfRange {
                field: "shape",
                expected: "a rate table in the declared venue's shape",
            });
        }
        Ok(Self {
            schema_version: FEE_SCHEDULE_SCHEMA_VERSION,
            venue,
            schedule_version,
            effective_date: effective_date.to_string(),
            provenance,
            shape,
        })
    }

    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }
    pub const fn venue(&self) -> FeeVenue {
        self.venue
    }
    pub const fn schedule_version(&self) -> u32 {
        self.schedule_version
    }
    pub fn effective_date(&self) -> &str {
        &self.effective_date
    }
    pub const fn provenance(&self) -> &FeeProvenance {
        &self.provenance
    }
    pub const fn shape(&self) -> &FeeScheduleShape {
        &self.shape
    }

    /// Revalidate a schedule after deserialization. Serde can populate private
    /// fields without passing through [`Self::build`], so every trust boundary
    /// must call this before the schedule becomes identity-bearing.
    pub(crate) fn validate(&self) -> Result<(), FeeScheduleError> {
        if self.schema_version != FEE_SCHEDULE_SCHEMA_VERSION {
            return Err(FeeScheduleError::OutOfRange {
                field: "schema_version",
                expected: "the supported fee-schedule schema version",
            });
        }
        if self.schedule_version == 0 {
            return Err(FeeScheduleError::OutOfRange {
                field: "schedule_version",
                expected: "a version of at least 1",
            });
        }
        check_calendar_date("effective_date", &self.effective_date)?;
        self.provenance.validate()?;
        self.shape.validate()?;
        if self.shape.venue() != self.venue {
            return Err(FeeScheduleError::OutOfRange {
                field: "shape",
                expected: "a rate table in the declared venue's shape",
            });
        }
        Ok(())
    }
}

// ── Binding ────────────────────────────────────────────────────────

/// A schedule plus the two choices a backtest has to make for it: which volume
/// band the account sits in, and whether its orders are assumed to make or
/// take liquidity. Both are recorded, both change the config id.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeeScheduleBinding {
    schedule: FeeSchedule,
    tier_index: usize,
    liquidity: LiquidityAssumption,
}

impl FeeScheduleBinding {
    /// Bind a schedule to a tier and a liquidity assumption.
    pub fn build(
        schedule: FeeSchedule,
        tier_index: usize,
        liquidity: LiquidityAssumption,
    ) -> Result<Self, FeeScheduleError> {
        let count = schedule.shape.tier_count();
        if tier_index >= count {
            return Err(FeeScheduleError::UnknownTier {
                index: tier_index,
                count,
            });
        }
        Ok(Self {
            schedule,
            tier_index,
            liquidity,
        })
    }

    pub const fn schedule(&self) -> &FeeSchedule {
        &self.schedule
    }
    pub const fn tier_index(&self) -> usize {
        self.tier_index
    }
    pub const fn liquidity(&self) -> LiquidityAssumption {
        self.liquidity
    }

    pub(crate) fn validate(&self) -> Result<(), FeeScheduleError> {
        self.schedule.validate()?;
        let count = self.schedule.shape.tier_count();
        if self.tier_index >= count {
            return Err(FeeScheduleError::UnknownTier {
                index: self.tier_index,
                count,
            });
        }
        Ok(())
    }

    /// The fee for one fill, in account currency.
    ///
    /// Returns `0.0` for a non-finite or non-positive trade rather than
    /// propagating a `NaN` into the ledger; the simulator rejects such an
    /// order before it ever reaches this call, so the guard is a floor, not a
    /// silent path.
    pub fn charge(&self, side: FeeSide, quantity: f64, price: f64) -> f64 {
        if !quantity.is_finite() || !price.is_finite() || quantity <= 0.0 || price <= 0.0 {
            return 0.0;
        }
        let notional = quantity * price;
        let fee = match &self.schedule.shape {
            FeeScheduleShape::KrakenSpot { tiers } => {
                let Some(tier) = tiers.get(self.tier_index) else {
                    return 0.0;
                };
                let percent = match self.liquidity {
                    LiquidityAssumption::Maker => tier.maker_percent,
                    LiquidityAssumption::Taker => tier.taker_percent,
                };
                notional * percent / 100.0
            }
            FeeScheduleShape::AlpacaUsEquity {
                per_share,
                minimum,
                sell_notional_percent,
                sell_per_share,
                sell_per_order_cap,
            } => {
                let base = (per_share * quantity).max(*minimum);
                match side {
                    FeeSide::Buy => base,
                    FeeSide::Sell => {
                        let taf = (sell_per_share * quantity).min(*sell_per_order_cap);
                        base + notional * sell_notional_percent / 100.0 + taf
                    }
                }
            }
        };
        if fee.is_finite() && fee > 0.0 {
            fee
        } else {
            0.0
        }
    }
}

// ── Validation helpers ─────────────────────────────────────────────

fn check_text(field: &'static str, value: &str) -> Result<(), FeeScheduleError> {
    if value.trim().is_empty() {
        return Err(FeeScheduleError::InvalidText {
            field,
            reason: "must not be empty",
        });
    }
    if value.trim() != value {
        return Err(FeeScheduleError::InvalidText {
            field,
            reason: "must not have surrounding whitespace",
        });
    }
    if value.chars().any(char::is_control) {
        return Err(FeeScheduleError::InvalidText {
            field,
            reason: "must not contain control characters",
        });
    }
    if value.chars().count() > MAX_FEE_TEXT_LEN {
        return Err(FeeScheduleError::InvalidText {
            field,
            reason: "is longer than the field permits",
        });
    }
    Ok(())
}

/// A strict `YYYY-MM-DD` check. `chrono` parses the calendar, so 2026-02-30 is
/// rejected rather than rolled forward; the extra length/shape test rejects the
/// abbreviated forms `chrono` would otherwise accept.
fn check_calendar_date(field: &'static str, value: &str) -> Result<(), FeeScheduleError> {
    let invalid = || {
        if field == "effective_date" {
            FeeScheduleError::InvalidEffectiveDate {
                value: value.to_string(),
            }
        } else {
            FeeScheduleError::InvalidText {
                field,
                reason: "must be a YYYY-MM-DD calendar day",
            }
        }
    };
    let bytes = value.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return Err(invalid());
    }
    if !bytes
        .iter()
        .enumerate()
        .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit())
    {
        return Err(invalid());
    }
    chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map(|_| ())
        .map_err(|_| invalid())
}

fn check_non_negative(field: &'static str, value: f64) -> Result<(), FeeScheduleError> {
    if !value.is_finite() {
        return Err(FeeScheduleError::NonFiniteValue { field });
    }
    if value < 0.0 {
        return Err(FeeScheduleError::OutOfRange {
            field,
            expected: "a non-negative amount",
        });
    }
    Ok(())
}

fn check_percent(field: &'static str, value: f64) -> Result<(), FeeScheduleError> {
    if !value.is_finite() {
        return Err(FeeScheduleError::NonFiniteValue { field });
    }
    if !(0.0..=100.0).contains(&value) {
        return Err(FeeScheduleError::OutOfRange {
            field,
            expected: "a percentage in [0, 100]",
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests;
