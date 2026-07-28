//! Canonical versioned strategy IR and deterministic run identity — the
//! ADR-135 §5.2 foundation slice for milestone M1.
//!
//! This module is deliberately narrow. It defines *what a strategy is* — data,
//! not a Rust `impl` — plus the two identities a reproducible run needs beside
//! it: the execution config and the run manifest. It contains **no
//! interpreter, no simulator, no compiler, and no UI**; those are the bulk of
//! M1 and are not started here. Nothing in this file should be read as M1
//! being complete.
//!
//! ## The three artifacts
//!
//! | Artifact | Definition input | Identity |
//! |---|---|---|
//! | [`StrategyIr`] | [`StrategyDefinition`] | `strategy_id` |
//! | [`StrategyExecutionConfig`] | [`ExecutionSettings`] | `config_id` |
//! | [`StrategyRunManifest`] | [`RunBinding`] | `run_id` |
//!
//! Each is built from its input, carries its own schema version, and can be
//! re-derived and verified. A stored artifact that verifies is semantically
//! equivalent under this schema to the artifact that was sealed.
//!
//! ## Identity
//!
//! Every id is a lowercase hex SHA-256 over an explicitly framed,
//! domain-separated byte encoding (see [`CanonicalDigest`]). The encoding is
//! *not* `Debug`, `Display`, or JSON: those are unstable across versions,
//! locales, and float formatting, and JSON in particular has no defined field
//! order for unordered maps. The rules:
//!
//! - Each artifact has its **own domain string**, so the same bytes hashed as
//!   a strategy and as a config cannot collide.
//! - Every element is `len: u64 BE || bytes`, preceded by its framed field
//!   tag, so `("ab", "c")` and `("a", "bc")` cannot produce the same stream.
//! - Every enum contributes an explicit **variant tag** before its payload,
//!   so `Custom { name: "ema" }` and the built-in `Ema` stay distinct.
//! - Every sequence writes its **length before its elements**, and order is
//!   significant — a truncated or reordered sequence cannot be re-framed.
//! - Floats are hashed by [`f64::to_bits`], never by a decimal rendering, and
//!   identity performs no floating-point arithmetic.
//! - Lookups during validation use ordered containers. Nothing here iterates a
//!   `HashMap`.
//!
//! One numeric normalization is applied, and only one: **`-0.0` is hashed as
//! `+0.0`** ([`canonical_f64_bits`]). The two are numerically equal, so
//! treating them as different strategies would be a false negative on every
//! reproducibility check. Values that cannot be encoded unambiguously are
//! **rejected, not hashed**: non-finite floats, and empty, whitespace-padded,
//! over-long, or control-character text.
//!
//! ## Canonical normalization
//!
//! Construction sorts declaration-order-only collections, tags, session
//! windows, and commutative [`Condition::All`]/[`Condition::Any`] children.
//! Semantically meaningful order, including indicator inputs and trade legs,
//! remains identity-bearing. Constant folding and dead-branch removal are not
//! implemented yet; execution-equivalent databank dedup must account for that.
//!
//! ## Temporal safety
//!
//! `bars_ago` is unsigned and bounded ([`MAX_BARS_AGO`]): `0` is the latest
//! observation visible at the decision event, and a future observation is not
//! representable. That is §6.12's no-look-ahead guarantee enforced by the
//! grammar. The reference simulator separately enforces visibility at runtime.
//!
//! ## Bounds
//!
//! [`Condition`] is recursive, so validation bounds both its depth
//! ([`MAX_CONDITION_DEPTH`]) and its node count ([`MAX_CONDITION_NODES`]) per
//! tree, and collections are individually capped. Sealed artifacts deliberately
//! do not implement `Deserialize`; their `from_json_slice` APIs cap encoded
//! bytes before decoding, reject unrecognized fields, and verify identity.
//! Raw input DTOs remain directly deserializable and must not be treated as
//! validated artifacts.

use crate::core::strategy_metrics::METRICS_SCHEMA_VERSION;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

/// Fee-schedule types are re-exported here because an execution config binds
/// one by value: a caller assembling settings should not have to reach into a
/// second module to name the venue it is charging.
pub use crate::core::strategy_fees::{
    FeeProvenance, FeeSchedule, FeeScheduleBinding, FeeScheduleError, FeeScheduleShape, FeeSide,
    FeeVenue, LiquidityAssumption, VolumeTier,
};

/// The M2 execution-realism types are re-exported for the same reason: an
/// execution config binds a calendar, a financing policy, a corporate-action
/// schedule and an instrument registry by value.
pub use crate::core::strategy_calendar::{
    CalendarError, ClosedReason, EarlyCloseRule, ExchangeTimeZone, LocalSessionWindow, SessionRule,
    SessionStatus, TradingCalendar, TradingCalendarSpec,
};
pub use crate::core::strategy_corporate::{
    CorporateAction, CorporateActionError, CorporateActionKind, CorporateActionSchedule,
};
pub use crate::core::strategy_financing::{
    AccrualInterval, CurrencyConversion, CurrencyRate, DayCount, FinancingCharge, FinancingError,
    FinancingModel, FinancingPolicy, RateProvenance, RateSource,
};
pub use crate::core::strategy_instrument::{InstrumentError, InstrumentRegistry, InstrumentSpec};

// ── Versions and bounds ────────────────────────────────────────────

/// Wire-format version of [`StrategyIr`]. Bump on any change to the hashed
/// encoding, the field set, or the validation rules.
pub const STRATEGY_IR_SCHEMA_VERSION: u32 = 1;

/// Wire-format version of [`StrategyExecutionConfig`].
///
/// v2 added the execution-realism fields of ADR-135 §6.3–§6.5 and §6.9:
/// fidelity level, latency model, margin policy, price tick, warm-up boundary,
/// the legacy-compatibility switch, and venue fee-schedule commissions.
///
/// v3 adds the M2 richer-execution semantics: the bar-volume participation cap
/// and partial-fill behaviour (§6.6), the per-instrument registry carrying
/// trading calendars, quote currencies and time-accrued financing with the
/// out-of-session order policy (§6.3, §6.7), the corporate-action schedule
/// (§6.8), the currency-conversion table (§6.3), and sub-bar fidelity (§6.9).
/// Every one of them changes what a run means, so none is a silent default.
pub const STRATEGY_EXECUTION_CONFIG_SCHEMA_VERSION: u32 = 3;

/// Wire-format version of [`StrategyRunManifest`]. v4 binds acknowledged
/// repaint QA artifacts into run identity; older manifests are intentionally
/// not migrated.
pub const STRATEGY_RUN_MANIFEST_SCHEMA_VERSION: u32 = 4;

/// Maximum encoded JSON size accepted by the sealed-artifact loading APIs.
/// Structural limits are then enforced while sealing the decoded DTO.
pub const MAX_SEALED_ARTIFACT_JSON_BYTES: usize = 1_048_576;

/// Domain-separation prefix for the strategy-id hash. Any change to the
/// framing rules must change this string *and* the schema version.
const STRATEGY_ID_DOMAIN: &str = "typhoon.strategy_ir.strategy_id.v1";

/// Domain-separation prefix for the config-id hash.
const CONFIG_ID_DOMAIN: &str = "typhoon.strategy_ir.config_id.v1";

/// Domain-separation prefix for the run-id hash.
const RUN_ID_DOMAIN: &str = "typhoon.strategy_ir.run_id.v4";

/// Longest root-to-leaf path allowed in one [`Condition`] tree, counting the
/// leaf. Bounds both validation recursion and the future interpreter's.
pub const MAX_CONDITION_DEPTH: usize = 16;

/// Total nodes allowed in one [`Condition`] tree.
pub const MAX_CONDITION_NODES: usize = 512;

/// Upper bound for simultaneously open positions requested by one strategy.
pub const MAX_OPEN_POSITIONS: u32 = 1_024;

/// Upper bound for a time-based exit horizon.
pub const MAX_BARS_IN_TRADE: u32 = 1_000_000;

/// Largest `bars_ago` an operand may look back. Bounds indicator warm-up.
pub const MAX_BARS_AGO: u32 = 4_096;

/// Longest free-text metadata field (name, author, notes, engine version).
pub const MAX_TEXT_LEN: usize = 256;

/// Longest stable reference id (parameter and indicator ids).
pub const MAX_STABLE_ID_LEN: usize = 64;

/// Maximum declared parameters.
pub const MAX_PARAMETERS: usize = 128;

/// Maximum declared indicator nodes.
pub const MAX_INDICATORS: usize = 64;

/// Maximum inputs to a single indicator node.
pub const MAX_INDICATOR_INPUTS: usize = 8;

/// Maximum metadata tags.
pub const MAX_TAGS: usize = 16;

/// Maximum session windows in a session filter.
pub const MAX_SESSION_WINDOWS: usize = 8;

/// Maximum trade-management legs (ADR-135 §5.2 two-leg templates, with room).
pub const MAX_TRADE_LEGS: usize = 4;

/// Maximum datasets one run may bind.
pub const MAX_DATASETS_PER_RUN: usize = 64;

/// Longest news blackout window, in minutes, on either side of an event.
pub const MAX_NEWS_BLOCK_MINUTES: u32 = 1_440;

/// Longest pre-close decision offset, in seconds.
pub const MAX_PRE_CLOSE_OFFSET_SECONDS: u32 = 86_400;

/// Largest decision→submit delay, in bars.
pub const MAX_SUBMIT_DELAY_BARS: u32 = 16;

/// Largest single latency leg, in nanoseconds — one hour. A backtest that
/// assumes a longer delay than that is describing an outage, not execution.
pub const MAX_LATENCY_NS: i64 = 3_600_000_000_000;

/// Largest execution-side warm-up, in bars. Sized to match [`MAX_BARS_AGO`],
/// which bounds how far a strategy may look back in the first place.
pub const MAX_WARMUP_BARS: u32 = MAX_BARS_AGO;

/// Longest sub-bar timeframe accepted for §6.9 level-3 fidelity — one week.
/// A finer path that is coarser than that is not a path.
pub const MAX_SUB_BAR_SECONDS: u32 = 604_800;

/// Minutes in a day. Session windows are expressed in UTC minutes from
/// midnight and may run to, but not past, this bound.
const MINUTES_PER_DAY: u32 = 1_440;

/// Trade-leg fractions are integer basis points and must total exactly this.
/// Integers, not floats, so "the legs add up to 100 %" is an exact check with
/// no tolerance to argue about.
const TOTAL_FRACTION_BPS: u32 = 10_000;

/// Length of a content-addressed id in hex characters (SHA-256).
const DIGEST_ID_LEN: usize = 64;

// ── Error surface ──────────────────────────────────────────────────

/// Why a free-text field could not be encoded unambiguously.
///
/// Mirrors `strategy_dataset::InvalidTextReason`; the two should be unified
/// into one shared canonical-encoding helper once both modules land together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvalidTextReason {
    /// Empty, or entirely whitespace.
    Empty,
    /// Carries leading or trailing whitespace. Rejected rather than trimmed:
    /// silently normalizing would map two distinct inputs onto one id.
    SurroundingWhitespace,
    /// Contains a control character.
    ControlCharacter,
    /// Longer than the field's bound.
    TooLong,
}

impl InvalidTextReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::Empty => "empty or whitespace-only",
            Self::SurroundingWhitespace => "leading or trailing whitespace",
            Self::ControlCharacter => "control character",
            Self::TooLong => "longer than the permitted length",
        }
    }
}

/// Why a stable reference id was rejected.
///
/// Reference ids are deliberately stricter than free text: they are matched
/// exactly across parameters, indicators, roles, sizing, and stops, so a
/// case- or space-variant id would be a silent dangling reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvalidIdReason {
    /// Empty.
    Empty,
    /// Longer than [`MAX_STABLE_ID_LEN`].
    TooLong,
    /// Contains something other than `[a-z0-9_]`.
    IllegalCharacter,
    /// Does not start with an ASCII lowercase letter.
    LeadingNonLetter,
}

impl InvalidIdReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::TooLong => "longer than the permitted length",
            Self::IllegalCharacter => "characters outside [a-z0-9_]",
            Self::LeadingNonLetter => "does not start with a lowercase letter",
        }
    }
}

/// Classification of a non-finite float. The offending value itself is not
/// carried: `NaN != NaN` would make errors non-comparable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NonFiniteKind {
    Nan,
    PositiveInfinity,
    NegativeInfinity,
}

impl NonFiniteKind {
    /// `None` when the value is finite.
    fn classify(value: f64) -> Option<Self> {
        if value.is_nan() {
            Some(Self::Nan)
        } else if value == f64::INFINITY {
            Some(Self::PositiveInfinity)
        } else if value == f64::NEG_INFINITY {
            Some(Self::NegativeInfinity)
        } else {
            None
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Nan => "NaN",
            Self::PositiveInfinity => "+inf",
            Self::NegativeInfinity => "-inf",
        }
    }
}

/// Which namespace a reference id lives in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefKind {
    Parameter,
    Indicator,
    Dataset,
}

impl RefKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Parameter => "parameter",
            Self::Indicator => "indicator",
            Self::Dataset => "dataset",
        }
    }
}

/// Which of the module's three artifacts an error refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    StrategyIr,
    ExecutionConfig,
    RunManifest,
}

impl ArtifactKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::StrategyIr => "strategy IR",
            Self::ExecutionConfig => "execution config",
            Self::RunManifest => "run manifest",
        }
    }

    fn id_field(self) -> &'static str {
        match self {
            Self::StrategyIr => "strategy_id",
            Self::ExecutionConfig => "config_id",
            Self::RunManifest => "run_id",
        }
    }

    fn supported_schema_version(self) -> u32 {
        match self {
            Self::StrategyIr => STRATEGY_IR_SCHEMA_VERSION,
            Self::ExecutionConfig => STRATEGY_EXECUTION_CONFIG_SCHEMA_VERSION,
            Self::RunManifest => STRATEGY_RUN_MANIFEST_SCHEMA_VERSION,
        }
    }
}

/// Everything that can go wrong building or verifying a strategy IR, an
/// execution config, or a run manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StrategyIrError {
    /// A free-text field has no unambiguous canonical encoding.
    InvalidText {
        field: String,
        reason: InvalidTextReason,
    },
    /// A stable reference id is malformed.
    InvalidId {
        field: String,
        id: String,
        reason: InvalidIdReason,
    },
    /// Two declarations claim the same id.
    DuplicateId { kind: RefKind, id: String },
    /// A reference names something that was never declared.
    UnknownRef {
        kind: RefKind,
        id: String,
        context: String,
    },
    /// Two role assignments claim the same slot.
    DuplicateRole { role: IndicatorRole },
    /// Indicator inputs form a cycle. `path` starts and ends on the repeated
    /// id, so the loop is readable directly.
    IndicatorCycle { path: Vec<String> },
    /// A float has no exact canonical encoding and is refused rather than
    /// hashed.
    NonFiniteValue { field: String, kind: NonFiniteKind },
    /// A value is outside the range its field permits.
    OutOfRange {
        field: String,
        value: String,
        expected: &'static str,
    },
    /// A collection exceeded its cap.
    TooMany {
        collection: &'static str,
        limit: usize,
        found: usize,
    },
    /// A condition tree is deeper than [`MAX_CONDITION_DEPTH`].
    ConditionTooDeep { limit: usize, found: usize },
    /// A condition tree has more nodes than [`MAX_CONDITION_NODES`].
    ConditionTooLarge { limit: usize, found: usize },
    /// Neither direction can ever enter, so the strategy is inert.
    NoEnabledDirection,
    /// The decision-timing fields contradict each other.
    InconsistentTiming { detail: &'static str },
    /// The execution settings contradict each other — most often a
    /// compatibility deviation dressed up as a realistic model.
    InconsistentExecution { detail: &'static str },
    /// A nested venue schedule was malformed, including when serde populated
    /// private fields without going through its checked constructor.
    InvalidFeeSchedule(FeeScheduleError),
    /// A nested trading calendar was malformed (§6.7).
    InvalidCalendar(CalendarError),
    /// A nested financing policy or currency table was malformed (§6.3).
    InvalidFinancing(FinancingError),
    /// A nested instrument registry was malformed (§6.3, §6.7).
    InvalidInstrument(InstrumentError),
    /// A nested corporate-action schedule was malformed (§6.8).
    InvalidCorporateAction(CorporateActionError),
    /// A field that must hold a content-addressed id (64 lowercase hex
    /// characters) does not.
    MalformedDigestId { field: String, value: String },
    /// A stored artifact was written by an incompatible schema version.
    UnsupportedSchemaVersion {
        artifact: ArtifactKind,
        found: u32,
        supported: u32,
    },
    /// A run requested a metric contract this build cannot reproduce.
    UnsupportedMetricsVersion {
        found: String,
        supported: &'static str,
    },
    /// The recomputed id does not match the recorded one — the artifact was
    /// edited after it was sealed.
    IdentityMismatch {
        artifact: ArtifactKind,
        expected: String,
        actual: String,
    },
}

impl std::fmt::Display for StrategyIrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidText { field, reason } => {
                write!(f, "field `{field}` is invalid: {}", reason.as_str())
            }
            Self::InvalidId { field, id, reason } => write!(
                f,
                "field `{field}` holds an invalid id `{id}`: {}",
                reason.as_str()
            ),
            Self::DuplicateId { kind, id } => {
                write!(f, "{} id `{id}` is declared more than once", kind.as_str())
            }
            Self::UnknownRef { kind, id, context } => write!(
                f,
                "{context} references {} `{id}`, which is not declared",
                kind.as_str()
            ),
            Self::DuplicateRole { role } => write!(
                f,
                "indicator role `{}` is assigned more than once",
                role.wire_tag()
            ),
            Self::IndicatorCycle { path } => {
                write!(f, "indicator inputs form a cycle: {}", path.join(" -> "))
            }
            Self::NonFiniteValue { field, kind } => write!(
                f,
                "field `{field}` is non-finite ({}) and has no canonical encoding",
                kind.as_str()
            ),
            Self::OutOfRange {
                field,
                value,
                expected,
            } => write!(f, "field `{field}` is {value}, expected {expected}"),
            Self::TooMany {
                collection,
                limit,
                found,
            } => write!(
                f,
                "`{collection}` holds {found} entries, more than the limit of {limit}"
            ),
            Self::ConditionTooDeep { limit, found } => write!(
                f,
                "condition tree is {found} levels deep, more than the limit of {limit}"
            ),
            Self::ConditionTooLarge { limit, found } => write!(
                f,
                "condition tree holds {found} nodes, more than the limit of {limit}"
            ),
            Self::NoEnabledDirection => {
                write!(f, "neither the long nor the short direction is enabled")
            }
            Self::InconsistentTiming { detail } => {
                write!(f, "execution timing is invalid: {detail}")
            }
            Self::InconsistentExecution { detail } => {
                write!(f, "execution settings are inconsistent: {detail}")
            }
            Self::InvalidFeeSchedule(error) => write!(f, "fee schedule is invalid: {error}"),
            Self::InvalidCalendar(error) => write!(f, "trading calendar is invalid: {error}"),
            Self::InvalidFinancing(error) => write!(f, "financing policy is invalid: {error}"),
            Self::InvalidInstrument(error) => {
                write!(f, "instrument registry is invalid: {error}")
            }
            Self::InvalidCorporateAction(error) => {
                write!(f, "corporate-action schedule is invalid: {error}")
            }
            Self::MalformedDigestId { field, value } => write!(
                f,
                "field `{field}` must be 64 lowercase hex characters, got `{value}`"
            ),
            Self::UnsupportedSchemaVersion {
                artifact,
                found,
                supported,
            } => write!(
                f,
                "{} schema version {found} is unsupported (this build supports {supported})",
                artifact.as_str()
            ),
            Self::UnsupportedMetricsVersion { found, supported } => write!(
                f,
                "metrics schema `{found}` is unsupported (this build supports `{supported}`)"
            ),
            Self::IdentityMismatch {
                artifact,
                expected,
                actual,
            } => write!(
                f,
                "{} {} mismatch: recorded {expected}, recomputed {actual}",
                artifact.as_str(),
                artifact.id_field()
            ),
        }
    }
}

impl std::error::Error for StrategyIrError {}

/// Failure while loading a sealed artifact through its bounded JSON API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactLoadError {
    TooLarge { limit: usize, found: usize },
    InvalidJson { message: String },
    InvalidArtifact(StrategyIrError),
}

impl std::fmt::Display for ArtifactLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLarge { limit, found } => {
                write!(f, "artifact JSON is {found} bytes, limit {limit}")
            }
            Self::InvalidJson { message } => write!(f, "invalid artifact JSON: {message}"),
            Self::InvalidArtifact(error) => write!(f, "invalid sealed artifact: {error}"),
        }
    }
}

impl std::error::Error for ArtifactLoadError {}

fn check_artifact_json_size(bytes: &[u8]) -> Result<(), ArtifactLoadError> {
    if bytes.len() > MAX_SEALED_ARTIFACT_JSON_BYTES {
        return Err(ArtifactLoadError::TooLarge {
            limit: MAX_SEALED_ARTIFACT_JSON_BYTES,
            found: bytes.len(),
        });
    }
    Ok(())
}

fn decode_strict_json<T>(bytes: &[u8]) -> Result<T, ArtifactLoadError>
where
    T: DeserializeOwned + Serialize,
{
    check_artifact_json_size(bytes)?;
    let original: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|error| ArtifactLoadError::InvalidJson {
            message: error.to_string(),
        })?;
    let decoded: T = serde_json::from_value(original.clone()).map_err(|error| {
        ArtifactLoadError::InvalidJson {
            message: error.to_string(),
        }
    })?;
    let recognized =
        serde_json::to_value(&decoded).map_err(|error| ArtifactLoadError::InvalidJson {
            message: error.to_string(),
        })?;
    if original != recognized {
        return Err(ArtifactLoadError::InvalidJson {
            message: "artifact contains unknown, omitted, or non-canonical fields".to_string(),
        });
    }
    Ok(decoded)
}

// ── Metadata ───────────────────────────────────────────────────────

/// Human-facing description of a strategy. Identity-bearing: renaming a
/// strategy produces a different id, because a stored result must be
/// attributable to exactly the artifact that produced it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StrategyMetadata {
    pub name: String,
    pub author: String,
    pub notes: Option<String>,
    /// Free-form labels. Order is significant (see the module note on
    /// canonicalization).
    pub tags: Vec<String>,
}

// ── Parameters ─────────────────────────────────────────────────────

/// A typed parameter value. Types are explicit so the future type-checker can
/// reject illegal compositions before any simulation runs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParamValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
}

impl ParamValue {
    fn wire_tag(&self) -> &'static str {
        match self {
            Self::Bool(_) => "bool",
            Self::Int(_) => "int",
            Self::Float(_) => "float",
            Self::Text(_) => "text",
        }
    }

    fn is_numeric(&self) -> bool {
        matches!(self, Self::Int(_) | Self::Float(_))
    }
}

/// An inclusive search range for a numeric parameter — the typed hole the
/// optimizer (§5.5) will later fill. Recorded here so the searched space is
/// part of the strategy's identity rather than an out-of-band setting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParamRange {
    Int { min: i64, max: i64 },
    Float { min: f64, max: f64 },
}

impl ParamRange {
    fn wire_tag(&self) -> &'static str {
        match self {
            Self::Int { .. } => "int",
            Self::Float { .. } => "float",
        }
    }
}

/// One declared parameter. The `id` is the stable name every reference uses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrategyParameter {
    pub id: String,
    pub value: ParamValue,
    /// Present only for numeric parameters, and must contain `value`.
    pub range: Option<ParamRange>,
}

// ── Indicator graph ────────────────────────────────────────────────

/// A field of the current bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PriceField {
    Open,
    High,
    Low,
    Close,
    Volume,
}

impl PriceField {
    fn wire_tag(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::High => "high",
            Self::Low => "low",
            Self::Close => "close",
            Self::Volume => "volume",
        }
    }
}

/// Which indicator an [`IndicatorNode`] computes.
///
/// The built-in set is the one the terminal already ports from the MQL5 NNFX
/// system. [`IndicatorKind::Custom`] carries a display name plus the
/// content-addressed identity of the executable implementation. The digest
/// prevents different plugins that share a name from sharing a strategy id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IndicatorKind {
    Atr,
    Sma,
    Ema,
    Kama,
    Rsi,
    FisherTransform,
    /// Exact formulas used by the pre-ADR-135 backtester. These versioned
    /// variants exist only for the migration/equivalence bridge.
    LegacyRollingKamaV1,
    LegacyUnsmoothedFisherMidpointV1,
    LegacyFisherValueV1,
    LegacyFisherSignalV1,
    LegacyRollingRsiV1,
    Macd,
    Adx,
    StdDev,
    Custom {
        name: String,
        implementation_id: String,
    },
}

impl IndicatorKind {
    fn wire_tag(&self) -> &'static str {
        match self {
            Self::Atr => "atr",
            Self::Sma => "sma",
            Self::Ema => "ema",
            Self::Kama => "kama",
            Self::Rsi => "rsi",
            Self::FisherTransform => "fisher_transform",
            Self::LegacyRollingKamaV1 => "legacy_rolling_kama_v1",
            Self::LegacyUnsmoothedFisherMidpointV1 => "legacy_unsmoothed_fisher_midpoint_v1",
            Self::LegacyFisherValueV1 => "legacy_fisher_value_v1",
            Self::LegacyFisherSignalV1 => "legacy_fisher_signal_v1",
            Self::LegacyRollingRsiV1 => "legacy_rolling_rsi_v1",
            Self::Macd => "macd",
            Self::Adx => "adx",
            Self::StdDev => "std_dev",
            Self::Custom { .. } => "custom",
        }
    }

    fn input_shape(&self) -> Option<&'static [IndicatorInputShape]> {
        use IndicatorInputShape::{Scalar, Series};
        match self {
            Self::Atr | Self::Adx => Some(&[Scalar]),
            Self::Sma
            | Self::Ema
            | Self::Rsi
            | Self::FisherTransform
            | Self::StdDev
            | Self::LegacyRollingRsiV1 => Some(&[Series, Scalar]),
            Self::Kama | Self::Macd | Self::LegacyRollingKamaV1 => {
                Some(&[Series, Scalar, Scalar, Scalar])
            }
            Self::LegacyUnsmoothedFisherMidpointV1
            | Self::LegacyFisherValueV1
            | Self::LegacyFisherSignalV1 => Some(&[Scalar]),
            Self::Custom { .. } => None,
        }
    }
}

/// One input edge of an indicator node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IndicatorInput {
    Constant(f64),
    Parameter(String),
    /// An edge to another indicator. These edges form the graph that must stay
    /// acyclic.
    Indicator(String),
    Price(PriceField),
}

impl IndicatorInput {
    fn wire_tag(&self) -> &'static str {
        match self {
            Self::Constant(_) => "constant",
            Self::Parameter(_) => "parameter",
            Self::Indicator(_) => "indicator",
            Self::Price(_) => "price",
        }
    }

    fn shape(&self) -> IndicatorInputShape {
        match self {
            Self::Price(_) | Self::Indicator(_) => IndicatorInputShape::Series,
            Self::Constant(_) | Self::Parameter(_) => IndicatorInputShape::Scalar,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IndicatorInputShape {
    Series,
    Scalar,
}

/// A node in the indicator graph. Declaration order does not imply dependency
/// order — a node may reference one declared later, as long as no cycle forms.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndicatorNode {
    pub id: String,
    pub kind: IndicatorKind,
    pub inputs: Vec<IndicatorInput>,
}

/// A named slot in the NNFX profile (ADR-135 §5.2's guided editor). Roles are
/// a *view* onto the indicator graph, not a second representation: each one
/// points at a node that must already exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IndicatorRole {
    Atr,
    Baseline,
    Confirmation1,
    Confirmation2,
    Volume,
    Exit,
    Continuation,
}

impl IndicatorRole {
    fn wire_tag(self) -> &'static str {
        match self {
            Self::Atr => "atr",
            Self::Baseline => "baseline",
            Self::Confirmation1 => "confirmation_1",
            Self::Confirmation2 => "confirmation_2",
            Self::Volume => "volume",
            Self::Exit => "exit",
            Self::Continuation => "continuation",
        }
    }
}

/// Binds one role to one indicator node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleAssignment {
    pub role: IndicatorRole,
    pub indicator: String,
}

// ── Condition AST ──────────────────────────────────────────────────

/// A value a condition can read at the current decision event.
///
/// `bars_ago` is unsigned: `0` is the latest observation visible *now*, and a
/// future observation cannot be written down. That is the grammar-level half
/// of §6.12.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operand {
    Constant(f64),
    Parameter(String),
    Price { field: PriceField, bars_ago: u32 },
    Indicator { id: String, bars_ago: u32 },
}

impl Operand {
    fn wire_tag(&self) -> &'static str {
        match self {
            Self::Constant(_) => "constant",
            Self::Parameter(_) => "parameter",
            Self::Price { .. } => "price",
            Self::Indicator { .. } => "indicator",
        }
    }
}

/// Comparison used by [`Condition::Compare`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompareOp {
    Greater,
    GreaterOrEqual,
    Less,
    LessOrEqual,
    Equal,
    NotEqual,
}

impl CompareOp {
    fn wire_tag(self) -> &'static str {
        match self {
            Self::Greater => "gt",
            Self::GreaterOrEqual => "gte",
            Self::Less => "lt",
            Self::LessOrEqual => "lte",
            Self::Equal => "eq",
            Self::NotEqual => "neq",
        }
    }
}

/// A boolean expression over operands.
///
/// Depth and node count are bounded per tree (see the module docs). `All` and
/// `Any` must carry at least one child: an empty combinator reads as a silent
/// constant and is rejected in favour of writing [`Condition::Always`] or
/// [`Condition::Never`] explicitly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Condition {
    Always,
    Never,
    Not(Box<Condition>),
    All(Vec<Condition>),
    Any(Vec<Condition>),
    Compare {
        left: Operand,
        op: CompareOp,
        right: Operand,
    },
    CrossesAbove {
        left: Operand,
        right: Operand,
    },
    CrossesBelow {
        left: Operand,
        right: Operand,
    },
}

impl Condition {
    fn wire_tag(&self) -> &'static str {
        match self {
            Self::Always => "always",
            Self::Never => "never",
            Self::Not(_) => "not",
            Self::All(_) => "all",
            Self::Any(_) => "any",
            Self::Compare { .. } => "compare",
            Self::CrossesAbove { .. } => "crosses_above",
            Self::CrossesBelow { .. } => "crosses_below",
        }
    }
}

/// Entry and exit rules for one direction. A disabled direction is still
/// validated and still hashed — turning it off is a change to the strategy,
/// not a licence to leave broken references behind.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DirectionRules {
    pub enabled: bool,
    pub entry: Condition,
    pub exit: Condition,
}

// ── Filters ────────────────────────────────────────────────────────

/// A trading window in UTC minutes from midnight, `start_minute` inclusive and
/// `end_minute` exclusive.
///
/// Honest limitation: this is a UTC clock window, not an exchange calendar.
/// There is no holiday table, no venue-local session, and no wrap past
/// midnight — a window that straddles 00:00 UTC must be written as two.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionWindow {
    pub start_minute: u32,
    pub end_minute: u32,
}

/// When the strategy may trade. Windows must be ordered and non-overlapping,
/// so the filter has exactly one reading.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionFilter {
    pub enabled: bool,
    pub windows: Vec<SessionWindow>,
    /// Flatten open positions when the last window closes.
    pub close_positions_outside: bool,
}

/// Lowest economic-event impact the news filter reacts to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NewsImpact {
    Low,
    Medium,
    High,
}

impl NewsImpact {
    fn wire_tag(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

/// Blackout around economic events. An enabled filter must block a non-zero
/// span on at least one side, otherwise it silently does nothing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewsFilter {
    pub enabled: bool,
    pub min_impact: NewsImpact,
    pub block_minutes_before: u32,
    pub block_minutes_after: u32,
    pub close_open_positions: bool,
}

// ── Sizing and trade management ────────────────────────────────────

/// How much to trade.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SizingRule {
    FixedUnits {
        units: f64,
    },
    /// Legacy `run_backtest` sizing: original fixed notional divided by each
    /// entry close, without equity compounding.
    LegacyFixedNotionalV1 {
        notional: f64,
    },
    PercentEquity {
        percent: f64,
    },
    /// Risk a percentage of equity across an ATR-derived stop distance. The
    /// referenced indicator must exist; that it actually computes an ATR is
    /// not checked here, since a custom node may legitimately supply one.
    RiskPercentAtr {
        risk_percent: f64,
        atr_multiple: f64,
        atr_indicator: String,
    },
}

impl SizingRule {
    fn wire_tag(&self) -> &'static str {
        match self {
            Self::FixedUnits { .. } => "fixed_units",
            Self::LegacyFixedNotionalV1 { .. } => "legacy_fixed_notional_v1",
            Self::PercentEquity { .. } => "percent_equity",
            Self::RiskPercentAtr { .. } => "risk_percent_atr",
        }
    }
}

/// Sizing plus the portfolio-level cap.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PositionSizing {
    pub rule: SizingRule,
    pub max_open_positions: u32,
}

/// A stop, target, or trail distance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopRule {
    AtrMultiple { indicator: String, multiple: f64 },
    PercentOfEntry { percent: f64 },
    PriceDistance { distance: f64 },
}

impl StopRule {
    fn wire_tag(&self) -> &'static str {
        match self {
            Self::AtrMultiple { .. } => "atr_multiple",
            Self::PercentOfEntry { .. } => "percent_of_entry",
            Self::PriceDistance { .. } => "price_distance",
        }
    }
}

/// A trailing stop, optionally dormant until price has moved `activate_after`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrailingStop {
    pub distance: StopRule,
    pub activate_after: Option<StopRule>,
}

/// One leg of a scale-out template. `fraction_bps` is integer basis points of
/// the position; the legs must total exactly 100 %.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeLeg {
    pub fraction_bps: u32,
    pub stop: Option<StopRule>,
    pub target: Option<StopRule>,
    pub trailing: Option<TrailingStop>,
}

/// The two-leg NNFX template generalised: N legs, each with its own stop,
/// target, and trail, plus position-level break-even and time stops.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeManagement {
    pub legs: Vec<TradeLeg>,
    pub break_even_after: Option<StopRule>,
    pub max_bars_in_trade: Option<u32>,
}

// ── Execution timing ───────────────────────────────────────────────

/// When the strategy makes its decision (ADR-135 §6.13).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionTiming {
    /// At the close of the completed bar.
    ClosedBar,
    /// At the open of the next bar.
    NextBarOpen,
    /// A fixed offset before the bar closes, reading only the forming-bar
    /// state actually available at that instant.
    PreClose { offset_seconds: u32 },
}

impl DecisionTiming {
    fn wire_tag(self) -> &'static str {
        match self {
            Self::ClosedBar => "closed_bar",
            Self::NextBarOpen => "next_bar_open",
            Self::PreClose { .. } => "pre_close",
        }
    }
}

/// Decision timing plus the two knobs that decide what the strategy can see
/// and how long it waits to act.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionTiming {
    pub decision: DecisionTiming,
    /// Whether the forming bar is visible. Only a [`DecisionTiming::PreClose`]
    /// rule may see it — at a closed-bar or next-open decision the forming bar
    /// either does not exist yet or would leak the future.
    pub forming_bar_visible: bool,
    /// Bars between the decision and order submission.
    pub submit_delay_bars: u32,
}

// ── Strategy definition and IR ─────────────────────────────────────

/// A complete strategy, as data. This is the input half of [`StrategyIr`]:
/// everything that is hashed, and nothing that is derived.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StrategyDefinition {
    pub metadata: StrategyMetadata,
    pub parameters: Vec<StrategyParameter>,
    pub indicators: Vec<IndicatorNode>,
    pub roles: Vec<RoleAssignment>,
    pub long: DirectionRules,
    pub short: DirectionRules,
    pub session: SessionFilter,
    pub news: NewsFilter,
    pub sizing: PositionSizing,
    pub trade_management: TradeManagement,
    pub timing: ExecutionTiming,
}

/// A validated, sealed strategy definition and its content-addressed id.
///
/// Construct with [`StrategyIr::build`]. Editing `definition` afterwards
/// invalidates `strategy_id`; [`StrategyIr::verify`] is what proves a loaded
/// artifact was not edited.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StrategyIr {
    schema_version: u32,
    definition: StrategyDefinition,
    /// Lowercase hex SHA-256 over the canonical encoding of `definition`.
    strategy_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StrategyIrWire {
    schema_version: u32,
    definition: StrategyDefinition,
    strategy_id: String,
}

impl StrategyIr {
    /// Validate `definition` and seal it with its id.
    pub fn build(definition: &StrategyDefinition) -> Result<Self, StrategyIrError> {
        let definition = normalize_definition(definition)?;
        let strategy_id = compute_validated_strategy_id(&definition);
        Ok(Self {
            schema_version: STRATEGY_IR_SCHEMA_VERSION,
            definition,
            strategy_id,
        })
    }

    /// Decode, validate, and identity-check a size-bounded JSON artifact.
    pub fn from_json_slice(bytes: &[u8]) -> Result<Self, ArtifactLoadError> {
        let wire: StrategyIrWire = decode_strict_json(bytes)?;
        let artifact = Self {
            schema_version: wire.schema_version,
            definition: wire.definition,
            strategy_id: wire.strategy_id,
        };
        let normalized = normalize_definition(&artifact.definition)
            .map_err(ArtifactLoadError::InvalidArtifact)?;
        if normalized != artifact.definition {
            return Err(ArtifactLoadError::InvalidJson {
                message: "sealed strategy definition is not in canonical order".to_string(),
            });
        }
        artifact
            .verify()
            .map_err(ArtifactLoadError::InvalidArtifact)?;
        Ok(artifact)
    }

    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn definition(&self) -> &StrategyDefinition {
        &self.definition
    }

    pub fn strategy_id(&self) -> &str {
        &self.strategy_id
    }

    /// The definition this IR seals, for rebuilding it.
    pub fn to_input(&self) -> StrategyDefinition {
        self.definition.clone()
    }

    /// Recompute the id from the current definition without comparing it.
    pub fn recompute_strategy_id(&self) -> Result<String, StrategyIrError> {
        check_schema_version(ArtifactKind::StrategyIr, self.schema_version)?;
        compute_strategy_id(&self.definition)
    }

    /// Prove this artifact is the one that was sealed: supported schema, still
    /// valid, and the recorded id still derives from the definition.
    pub fn verify(&self) -> Result<(), StrategyIrError> {
        let actual = self.recompute_strategy_id()?;
        expect_identity(ArtifactKind::StrategyIr, &self.strategy_id, actual)
    }
}

// ── Execution config ───────────────────────────────────────────────

/// How trading costs are charged.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommissionModel {
    /// Explicitly free. Valid, and loudly labelled — never a silent default.
    None,
    PerShare {
        amount: f64,
        minimum: f64,
    },
    PercentOfNotional {
        percent: f64,
        minimum: f64,
    },
    PerOrder {
        amount: f64,
    },
    /// Fees derived from a versioned venue schedule (§6.3). The venue,
    /// schedule version, effective date, tier, liquidity assumption and
    /// provenance note are all part of the config identity, so a historical
    /// run keeps charging what it charged when it was run.
    VenueSchedule(FeeScheduleBinding),
}

impl CommissionModel {
    fn wire_tag(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::PerShare { .. } => "per_share",
            Self::PercentOfNotional { .. } => "percent_of_notional",
            Self::PerOrder { .. } => "per_order",
            Self::VenueSchedule(_) => "venue_schedule",
        }
    }
}

/// How much of the intrabar path the simulator claims to know (§6.9).
///
/// Level 4 — true tick replay — is **not representable**. ADR-135 §11.3 records
/// the blocker honestly: TyphooN retains no versioned tick corpus, so a `Tick`
/// variant here would be a promise the data cannot keep. Sub-bar is the highest
/// honest fidelity today, and the ladder is shaped so tick drops in beside it
/// when a corpus exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FidelityLevel {
    /// Level 1 — only bar opens and closes are execution prices. A resting
    /// order is tested against closes; nothing is resolved inside a bar.
    #[default]
    BarClose,
    /// Level 2 — resting orders resolve against the bar's OHLC under the
    /// [`OhlcAmbiguityPolicy`], and a gapped trigger fills at the open.
    BarOhlc,
    /// Level 3 — a finer timeframe supplies the intrabar path. Each sub-bar is
    /// resolved in time order under the same level-2 rules, so the *sequence*
    /// of two levels inside one bar is observed rather than assumed: the
    /// ambiguity policy is consulted only for a tie inside one sub-bar.
    ///
    /// `sub_bar_seconds` names the finer timeframe. It is part of the config
    /// identity because it decides which bound dataset becomes the path, and a
    /// run resolved against 1-minute bars is not the same run as one resolved
    /// against 5-minute bars.
    SubBar { sub_bar_seconds: u32 },
}

impl FidelityLevel {
    fn wire_tag(self) -> &'static str {
        match self {
            Self::BarClose => "bar_close",
            Self::BarOhlc => "bar_ohlc",
            Self::SubBar { .. } => "sub_bar",
        }
    }

    /// Whether the bar's own range resolves triggers. Both level 2 and the
    /// per-sub-bar resolution of level 3 use the same rule.
    pub const fn resolves_intrabar(self) -> bool {
        matches!(self, Self::BarOhlc | Self::SubBar { .. })
    }

    /// The finer timeframe supplying the intrabar path, when there is one.
    pub const fn sub_bar_seconds(self) -> Option<u32> {
        match self {
            Self::SubBar { sub_bar_seconds } => Some(sub_bar_seconds),
            _ => None,
        }
    }
}

/// How much of a bar's traded volume one run may take (§6.6).
///
/// The cap is shared across every order executing in the same bar phase, so a
/// strategy cannot route around it by splitting one order into ten. What is not
/// filled rests, retries on the next bar, or dies with its time-in-force.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParticipationModel {
    /// No liquidity cap: an order takes whatever size it asks for. Valid, and
    /// stamped on the run — it is the assumption that the market absorbs any
    /// order without moving, which is only safe for small size.
    Unlimited,
    /// Fills are capped at `fraction` of the executing bar's volume.
    BarVolumeFraction { fraction: f64 },
}

impl ParticipationModel {
    fn wire_tag(self) -> &'static str {
        match self {
            Self::Unlimited => "unlimited",
            Self::BarVolumeFraction { .. } => "bar_volume_fraction",
        }
    }

    /// Size available to the whole bar, or `None` when uncapped.
    pub fn bar_capacity(self, bar_volume: f64) -> Option<f64> {
        match self {
            Self::Unlimited => None,
            Self::BarVolumeFraction { fraction } => Some((bar_volume * fraction).max(0.0)),
        }
    }
}

/// What happens to an order submitted while the instrument's calendar says the
/// venue is closed (§6.7). Under both policies it never fills out of session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutsideSessionPolicy {
    /// The order rests until the calendar reopens. The conservative default: it
    /// cannot invent a fill, and it cannot silently destroy an intent the
    /// strategy expressed — which is exactly what a venue's good-til-cancelled
    /// book does over a weekend.
    #[default]
    Queue,
    /// The venue refuses out-of-session submissions. The order is rejected and
    /// reported, never dropped.
    Reject,
}

impl OutsideSessionPolicy {
    fn wire_tag(self) -> &'static str {
        match self {
            Self::Queue => "queue",
            Self::Reject => "reject",
        }
    }
}

/// Decision→submit and submit→exchange delay (§6.4). Every random draw comes
/// from the run's seeded stream; `thread_rng` is never used in simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LatencyModel {
    /// Zero delay. Valid, and stamped on the run — an order still cannot fill
    /// at a price its own decision already saw.
    #[default]
    None,
    Fixed {
        decision_to_submit_ns: i64,
        submit_to_exchange_ns: i64,
    },
    /// Inclusive uniform draws from the run's seeded stream.
    SeededUniform {
        decision_to_submit_min_ns: i64,
        decision_to_submit_max_ns: i64,
        submit_to_exchange_min_ns: i64,
        submit_to_exchange_max_ns: i64,
    },
}

impl LatencyModel {
    fn wire_tag(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Fixed { .. } => "fixed",
            Self::SeededUniform { .. } => "seeded_uniform",
        }
    }
}

/// Whether a fill may consume buying power the account does not have (§6.5's
/// insufficient-margin rejection).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarginPolicy {
    /// No buying-power constraint, so an insufficient-funds rejection can
    /// never occur. Explicit and stamped: leverage is unbounded.
    #[default]
    Unconstrained,
    /// Cash account: a fill may not drive cash negative, and a position may
    /// not go short. Borrow cost and real margin are §6.3/M2 work; refusing
    /// the short is honest, silently shorting for free is not.
    CashOnly,
}

impl MarginPolicy {
    fn wire_tag(self) -> &'static str {
        match self {
            Self::Unconstrained => "unconstrained",
            Self::CashOnly => "cash_only",
        }
    }
}

/// Deliberate deviations from the engine's own execution model.
///
/// The default model never lets a fill happen at a price the decision could
/// already see. Reproducing the legacy `run_backtest` numbers requires exactly
/// that, so it is available — as a named, validated, hashed choice that shows
/// up in every report, and never as a default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionCompatibility {
    /// The engine's own model.
    #[default]
    None,
    /// Legacy bridge: a market order fills at the close of the bar that
    /// decided it. Physically unrealizable, permitted only to prove the new
    /// engine reproduces the old numbers (§13 M1 gate clause 4).
    LegacySameBarClose,
}

impl ExecutionCompatibility {
    fn wire_tag(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::LegacySameBarClose => "legacy_same_bar_close",
        }
    }
}

/// How far fills drift from the decision price.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlippageModel {
    None,
    FixedPriceDistance { distance: f64 },
    SpreadFraction { fraction: f64 },
    VolatilityScaled { atr_fraction: f64 },
}

impl SlippageModel {
    fn wire_tag(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::FixedPriceDistance { .. } => "fixed_price_distance",
            Self::SpreadFraction { .. } => "spread_fraction",
            Self::VolatilityScaled { .. } => "volatility_scaled",
        }
    }
}

/// Where the bid/ask spread comes from (§6.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpreadModel {
    /// No spread at all. Valid, but a deliberately loud choice: it is stamped
    /// on the run and must never be presented as realistic.
    None,
    Constant {
        price_units: f64,
    },
    PercentOfPrice {
        percent: f64,
    },
    /// Use the dataset's recorded quotes.
    RecordedQuotes,
}

impl SpreadModel {
    fn wire_tag(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Constant { .. } => "constant",
            Self::PercentOfPrice { .. } => "percent_of_price",
            Self::RecordedQuotes => "recorded_quotes",
        }
    }
}

/// What the simulator assumes when a stop and a target are both reachable
/// inside one bar (§6.1). The default is the pessimistic one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OhlcAmbiguityPolicy {
    /// Assume the stop filled first.
    #[default]
    StopFirst,
    /// Assume the target filled first.
    TargetFirst,
    /// Walk O→H→L→C on up bars and O→L→H→C on down bars.
    OhlcPath,
}

impl OhlcAmbiguityPolicy {
    fn wire_tag(self) -> &'static str {
        match self {
            Self::StopFirst => "stop_first",
            Self::TargetFirst => "target_first",
            Self::OhlcPath => "ohlc_path",
        }
    }
}

/// How simultaneous events are ordered (§5.3, §6.11). Both variants are total
/// orders — neither ever falls back to hash-map or completion order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TieBreakPolicy {
    /// `(timestamp, event priority, submission sequence)`.
    #[default]
    TimestampPrioritySequence,
    /// `(timestamp, event priority, symbol, submission sequence)` — pins
    /// cross-symbol ties to a lexicographic symbol order.
    TimestampPrioritySymbolSequence,
}

impl TieBreakPolicy {
    fn wire_tag(self) -> &'static str {
        match self {
            Self::TimestampPrioritySequence => "timestamp_priority_sequence",
            Self::TimestampPrioritySymbolSequence => "timestamp_priority_symbol_sequence",
        }
    }
}

/// The cost and execution-policy half of a run: everything that is not the
/// strategy and not the data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionSettings {
    pub initial_capital: f64,
    pub account_currency: String,
    pub commission: CommissionModel,
    pub slippage: SlippageModel,
    pub spread: SpreadModel,
    /// Which of a bar's reachable stop and target is assumed to fill first.
    /// Only consulted at [`FidelityLevel::BarOhlc`] and above, and recorded
    /// either way so a report never has to guess what was assumed.
    pub ambiguity: OhlcAmbiguityPolicy,
    pub tie_break: TieBreakPolicy,
    pub fidelity: FidelityLevel,
    pub latency: LatencyModel,
    pub margin: MarginPolicy,
    /// Price lattice every limit/stop price must sit on. `None` means no tick
    /// constraint — honest for M1, which has no per-instrument spec registry.
    pub price_tick: Option<f64>,
    /// Bars a symbol must have closed before it may submit an order. Orders
    /// before the boundary are *rejected and reported*, never dropped.
    pub warmup_bars: u32,
    pub compatibility: ExecutionCompatibility,
    /// Liquidity cap on fills (§6.6). Uncapped by default, and stamped.
    #[serde(default = "default_participation")]
    pub participation: ParticipationModel,
    /// Per-instrument calendars, currencies and financing (§6.3, §6.7). Empty
    /// by default: no session gating, no accruals, no conversion.
    #[serde(default)]
    pub instruments: InstrumentRegistry,
    /// What an out-of-session submission does, for instruments that have a
    /// calendar (§6.7).
    #[serde(default)]
    pub outside_session: OutsideSessionPolicy,
    /// Corporate actions applied as events at their effective time (§6.8).
    #[serde(default)]
    pub corporate_actions: CorporateActionSchedule,
    /// How a non-account-currency instrument reaches the account currency
    /// (§6.3). `None` refuses such an instrument rather than assuming parity.
    #[serde(default)]
    pub currency_conversion: CurrencyConversion,
}

const fn default_participation() -> ParticipationModel {
    ParticipationModel::Unlimited
}

impl ExecutionSettings {
    /// The conservative baseline of §6: closed-bar fidelity, pessimistic
    /// stop-first ambiguity, no compatibility deviation, no unbounded
    /// warm-up assumption. Costs still have to be chosen deliberately —
    /// there is no "reasonable default" fee.
    pub fn conservative_defaults() -> Self {
        Self {
            initial_capital: 100_000.0,
            account_currency: "USD".to_string(),
            commission: CommissionModel::None,
            slippage: SlippageModel::None,
            spread: SpreadModel::None,
            ambiguity: OhlcAmbiguityPolicy::StopFirst,
            tie_break: TieBreakPolicy::TimestampPrioritySequence,
            fidelity: FidelityLevel::BarClose,
            latency: LatencyModel::None,
            margin: MarginPolicy::Unconstrained,
            price_tick: None,
            warmup_bars: 0,
            compatibility: ExecutionCompatibility::None,
            // The M2 additions all default to "not modelled", which keeps the
            // baseline honest: an unconfigured run reports no liquidity cap, no
            // session gating, no accrual and no corporate action, and the
            // report says so rather than implying they were accounted for.
            participation: ParticipationModel::Unlimited,
            instruments: InstrumentRegistry::empty(),
            outside_session: OutsideSessionPolicy::Queue,
            corporate_actions: CorporateActionSchedule::empty(),
            currency_conversion: CurrencyConversion::None,
        }
    }
}

/// Validated execution settings and their content-addressed id.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StrategyExecutionConfig {
    schema_version: u32,
    settings: ExecutionSettings,
    /// Lowercase hex SHA-256 over the canonical encoding of `settings`.
    config_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StrategyExecutionConfigWire {
    schema_version: u32,
    settings: ExecutionSettings,
    config_id: String,
}

impl StrategyExecutionConfig {
    /// Validate `settings` and seal them with their id.
    pub fn build(settings: &ExecutionSettings) -> Result<Self, StrategyIrError> {
        let config_id = compute_config_id(settings)?;
        Ok(Self {
            schema_version: STRATEGY_EXECUTION_CONFIG_SCHEMA_VERSION,
            settings: settings.clone(),
            config_id,
        })
    }

    /// Decode, validate, and identity-check a size-bounded JSON artifact.
    pub fn from_json_slice(bytes: &[u8]) -> Result<Self, ArtifactLoadError> {
        let wire: StrategyExecutionConfigWire = decode_strict_json(bytes)?;
        let artifact = Self {
            schema_version: wire.schema_version,
            settings: wire.settings,
            config_id: wire.config_id,
        };
        artifact
            .verify()
            .map_err(ArtifactLoadError::InvalidArtifact)?;
        Ok(artifact)
    }

    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn settings(&self) -> &ExecutionSettings {
        &self.settings
    }

    pub fn config_id(&self) -> &str {
        &self.config_id
    }

    /// The settings this config seals, for rebuilding it.
    pub fn to_input(&self) -> ExecutionSettings {
        self.settings.clone()
    }

    /// Recompute the id from the current settings without comparing it.
    pub fn recompute_config_id(&self) -> Result<String, StrategyIrError> {
        check_schema_version(ArtifactKind::ExecutionConfig, self.schema_version)?;
        compute_config_id(&self.settings)
    }

    /// Prove this config is the one that was sealed.
    pub fn verify(&self) -> Result<(), StrategyIrError> {
        let actual = self.recompute_config_id()?;
        expect_identity(ArtifactKind::ExecutionConfig, &self.config_id, actual)
    }
}

// ── Run manifest ───────────────────────────────────────────────────

/// Everything a run is pinned to. Reproducing a result means rebuilding this
/// binding and getting the same `run_id`.
///
/// The id fields are content-addressed digests produced elsewhere:
/// dataset binding ids come from the dataset layer (§5.1), `strategy_id` from
/// [`StrategyIr`], `config_id` from [`StrategyExecutionConfig`], and
/// `intervention_log_id` from the recorded `UserDecision` stream of a hybrid
/// run (§5.3). This module validates their *shape* — 64 lowercase hex
/// characters — and binds them by semantic input id; it does not resolve them, so a
/// manifest can be verified without loading the artifacts it names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DatasetBinding {
    /// Stable semantic input slot, such as `primary` or `confirmation_h4`.
    pub input_id: String,
    /// Content-addressed dataset manifest id.
    pub dataset_id: String,
}

/// The operator disposition for one repaint QA artifact. The disposition and
/// warning note are sealed into the run id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum RepaintAcknowledgement {
    Clean,
    WarningAcknowledged { note: String },
}

/// One repaint QA artifact required to resolve a verified run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepaintQaBinding {
    pub indicator_id: String,
    pub artifact_id: String,
    pub acknowledgement: RepaintAcknowledgement,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunBinding {
    /// Named dataset inputs. Declaration order is canonicalized by `input_id`.
    /// The same immutable dataset may intentionally serve more than one role.
    pub datasets: Vec<DatasetBinding>,
    pub strategy_id: String,
    pub config_id: String,
    /// Root seed for every derived RNG stream (§6.10).
    pub seed: u64,
    pub engine_version: String,
    /// Exact metric definitions used to interpret this run's ledger.
    pub metrics_version: String,
    /// Present only for hybrid runs that recorded operator interventions.
    pub intervention_log_id: Option<String>,
    /// Repaint evidence, canonicalized by indicator id.
    pub repaint_qa: Vec<RepaintQaBinding>,
}

/// A validated run binding and its content-addressed id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StrategyRunManifest {
    schema_version: u32,
    binding: RunBinding,
    /// Lowercase hex SHA-256 over the canonical encoding of `binding`.
    run_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StrategyRunManifestWire {
    schema_version: u32,
    binding: RunBinding,
    run_id: String,
}

impl StrategyRunManifest {
    /// Validate `binding` and seal it with its run id.
    pub fn build(binding: &RunBinding) -> Result<Self, StrategyIrError> {
        let binding = normalize_binding(binding)?;
        let run_id = compute_validated_run_id(&binding);
        Ok(Self {
            schema_version: STRATEGY_RUN_MANIFEST_SCHEMA_VERSION,
            binding,
            run_id,
        })
    }

    /// Decode, validate, and identity-check a size-bounded JSON artifact.
    pub fn from_json_slice(bytes: &[u8]) -> Result<Self, ArtifactLoadError> {
        let wire: StrategyRunManifestWire = decode_strict_json(bytes)?;
        let artifact = Self {
            schema_version: wire.schema_version,
            binding: wire.binding,
            run_id: wire.run_id,
        };
        let normalized =
            normalize_binding(&artifact.binding).map_err(ArtifactLoadError::InvalidArtifact)?;
        if normalized != artifact.binding {
            return Err(ArtifactLoadError::InvalidJson {
                message: "sealed run binding is not in canonical order".to_string(),
            });
        }
        artifact
            .verify()
            .map_err(ArtifactLoadError::InvalidArtifact)?;
        Ok(artifact)
    }

    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn binding(&self) -> &RunBinding {
        &self.binding
    }

    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    /// The binding this manifest seals, for rebuilding it.
    pub fn to_input(&self) -> RunBinding {
        self.binding.clone()
    }

    /// Recompute the id from the current binding without comparing it.
    pub fn recompute_run_id(&self) -> Result<String, StrategyIrError> {
        check_schema_version(ArtifactKind::RunManifest, self.schema_version)?;
        compute_run_id(&self.binding)
    }

    /// Prove this manifest is the one that was sealed. Any edit to the bound
    /// ids, the seed, the engine version, or the dataset order is detected.
    pub fn verify(&self) -> Result<(), StrategyIrError> {
        let actual = self.recompute_run_id()?;
        expect_identity(ArtifactKind::RunManifest, &self.run_id, actual)
    }
}

// ── Canonical encoding ─────────────────────────────────────────────

/// Bit pattern of `-0.0`, normalized to `+0.0` before hashing.
const NEGATIVE_ZERO_BITS: u64 = 0x8000_0000_0000_0000;

/// Exact, platform-independent bits for a finite `f64`.
///
/// `-0.0` maps onto `+0.0` (the one documented numeric normalization); every
/// other value keeps its exact IEEE-754 bit pattern. This is pure integer
/// work — no floating-point arithmetic participates in identity. Callers must
/// reject non-finite values first.
fn canonical_f64_bits(value: f64) -> u64 {
    let bits = value.to_bits();
    if bits == NEGATIVE_ZERO_BITS { 0 } else { bits }
}

/// Length-prefixed, domain-separated SHA-256 writer.
///
/// Every element is `len: u64 BE || bytes`, and every value is preceded by its
/// framed field tag, so `("AB", "C")` and `("A", "BC")` cannot collide.
struct CanonicalDigest {
    hasher: Sha256,
}

impl CanonicalDigest {
    fn new(domain: &str) -> Self {
        let mut digest = Self {
            hasher: Sha256::new(),
        };
        digest.frame(domain.as_bytes());
        digest
    }

    fn frame(&mut self, bytes: &[u8]) {
        self.hasher.update((bytes.len() as u64).to_be_bytes());
        self.hasher.update(bytes);
    }

    fn tagged_text(&mut self, tag: &str, value: &str) {
        self.frame(tag.as_bytes());
        self.frame(value.as_bytes());
    }

    fn tagged_u64(&mut self, tag: &str, value: u64) {
        self.frame(tag.as_bytes());
        self.hasher.update(value.to_be_bytes());
    }

    fn tagged_u32(&mut self, tag: &str, value: u32) {
        self.tagged_u64(tag, u64::from(value));
    }

    /// Signed integers are hashed as their two's-complement big-endian bytes —
    /// no sign-magnitude rendering, no locale.
    fn tagged_i64(&mut self, tag: &str, value: i64) {
        self.frame(tag.as_bytes());
        self.hasher.update(value.to_be_bytes());
    }

    fn tagged_bool(&mut self, tag: &str, value: bool) {
        self.tagged_u64(tag, u64::from(value));
    }

    /// Hash a finite `f64` by its canonical bits. Non-finite values must have
    /// been rejected upstream.
    fn tagged_f64(&mut self, tag: &str, value: f64) {
        self.frame(tag.as_bytes());
        self.hasher.update(canonical_f64_bits(value).to_be_bytes());
    }

    /// Open a sequence by writing its length, so elements cannot be re-framed
    /// across the boundary of an adjacent field.
    fn begin_seq(&mut self, tag: &str, len: usize) {
        self.tagged_u64(tag, len as u64);
    }

    /// Record whether an optional field is present. The payload, if any,
    /// follows.
    fn begin_option(&mut self, tag: &str, present: bool) {
        self.tagged_text(tag, if present { "some" } else { "none" });
    }

    fn finish_hex(self) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let out = self.hasher.finalize();
        let mut hex = String::with_capacity(out.len() * 2);
        for &byte in out.iter() {
            hex.push(HEX[(byte >> 4) as usize] as char);
            hex.push(HEX[(byte & 0x0f) as usize] as char);
        }
        hex
    }
}

// ── Validation helpers ─────────────────────────────────────────────

/// Reject text that has no unambiguous canonical encoding.
fn validate_text(value: &str, max_len: usize) -> Result<(), InvalidTextReason> {
    if value.trim().is_empty() {
        return Err(InvalidTextReason::Empty);
    }
    if value.trim() != value {
        return Err(InvalidTextReason::SurroundingWhitespace);
    }
    if value.chars().any(char::is_control) {
        return Err(InvalidTextReason::ControlCharacter);
    }
    if value.chars().count() > max_len {
        return Err(InvalidTextReason::TooLong);
    }
    Ok(())
}

fn check_text(field: &str, value: &str, max_len: usize) -> Result<(), StrategyIrError> {
    validate_text(value, max_len).map_err(|reason| StrategyIrError::InvalidText {
        field: field.to_string(),
        reason,
    })
}

fn check_optional_text(
    field: &str,
    value: Option<&str>,
    max_len: usize,
) -> Result<(), StrategyIrError> {
    match value {
        Some(text) => check_text(field, text, max_len),
        None => Ok(()),
    }
}

/// Reject reference ids that are not stable, exact-match-safe names.
fn validate_stable_id(value: &str) -> Result<(), InvalidIdReason> {
    if value.is_empty() {
        return Err(InvalidIdReason::Empty);
    }
    if value.len() > MAX_STABLE_ID_LEN {
        return Err(InvalidIdReason::TooLong);
    }
    if !value
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        return Err(InvalidIdReason::IllegalCharacter);
    }
    if !value.starts_with(|c: char| c.is_ascii_lowercase()) {
        return Err(InvalidIdReason::LeadingNonLetter);
    }
    Ok(())
}

fn check_stable_id(field: &str, value: &str) -> Result<(), StrategyIrError> {
    validate_stable_id(value).map_err(|reason| StrategyIrError::InvalidId {
        field: field.to_string(),
        id: value.to_string(),
        reason,
    })
}

/// Reject anything that is not a 64-character lowercase hex digest.
fn check_digest_id(field: &str, value: &str) -> Result<(), StrategyIrError> {
    let well_formed = value.len() == DIGEST_ID_LEN
        && value
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c));
    if well_formed {
        Ok(())
    } else {
        Err(StrategyIrError::MalformedDigestId {
            field: field.to_string(),
            value: value.to_string(),
        })
    }
}

fn check_finite(field: &str, value: f64) -> Result<(), StrategyIrError> {
    match NonFiniteKind::classify(value) {
        Some(kind) => Err(StrategyIrError::NonFiniteValue {
            field: field.to_string(),
            kind,
        }),
        None => Ok(()),
    }
}

/// Finite *and* inside `(low, high]`-style bounds described by `expected`.
fn check_finite_in(
    field: &str,
    value: f64,
    ok: bool,
    expected: &'static str,
) -> Result<(), StrategyIrError> {
    check_finite(field, value)?;
    if ok {
        Ok(())
    } else {
        Err(StrategyIrError::OutOfRange {
            field: field.to_string(),
            value: format_f64(value),
            expected,
        })
    }
}

fn check_size(collection: &'static str, len: usize, limit: usize) -> Result<(), StrategyIrError> {
    if len <= limit {
        Ok(())
    } else {
        Err(StrategyIrError::TooMany {
            collection,
            limit,
            found: len,
        })
    }
}

fn out_of_range<T: std::fmt::Display>(
    field: &str,
    value: T,
    expected: &'static str,
) -> StrategyIrError {
    StrategyIrError::OutOfRange {
        field: field.to_string(),
        value: value.to_string(),
        expected,
    }
}

/// Render a finite float for an error message only — never for hashing.
fn format_f64(value: f64) -> String {
    format!("{value}")
}

fn check_schema_version(artifact: ArtifactKind, found: u32) -> Result<(), StrategyIrError> {
    let supported = artifact.supported_schema_version();
    if found == supported {
        Ok(())
    } else {
        Err(StrategyIrError::UnsupportedSchemaVersion {
            artifact,
            found,
            supported,
        })
    }
}

fn expect_identity(
    artifact: ArtifactKind,
    recorded: &str,
    actual: String,
) -> Result<(), StrategyIrError> {
    if recorded == actual {
        Ok(())
    } else {
        Err(StrategyIrError::IdentityMismatch {
            artifact,
            expected: recorded.to_string(),
            actual,
        })
    }
}

// ── Strategy identity ──────────────────────────────────────────────

/// Resolved declarations, used to check every reference exactly once.
struct DeclaredIds<'a> {
    parameters: BTreeMap<&'a str, &'a ParamValue>,
    indicators: BTreeSet<&'a str>,
}

impl<'a> DeclaredIds<'a> {
    fn check_ref(&self, kind: RefKind, id: &str, context: &str) -> Result<(), StrategyIrError> {
        let known = match kind {
            RefKind::Parameter => self.parameters.contains_key(id),
            RefKind::Indicator => self.indicators.contains(id),
            RefKind::Dataset => false,
        };
        if known {
            Ok(())
        } else {
            Err(StrategyIrError::UnknownRef {
                kind,
                id: id.to_string(),
                context: context.to_string(),
            })
        }
    }

    fn check_numeric_parameter(&self, id: &str, context: &str) -> Result<(), StrategyIrError> {
        self.check_ref(RefKind::Parameter, id, context)?;
        if self.parameters[id].is_numeric() {
            Ok(())
        } else {
            Err(out_of_range(
                context,
                self.parameters[id].wire_tag(),
                "a numeric parameter",
            ))
        }
    }
}

/// The content-addressed strategy id: lowercase hex SHA-256 over the canonical
/// encoding of a fully validated definition.
///
/// Validation runs first and in a fixed order, so the same malformed
/// definition always reports the same error.
pub fn compute_strategy_id(definition: &StrategyDefinition) -> Result<String, StrategyIrError> {
    let definition = normalize_definition(definition)?;
    Ok(compute_validated_strategy_id(&definition))
}

/// Return the one stored representation for order-insensitive declarations.
/// Indicator input order and trade-leg order remain semantically meaningful.
fn normalize_definition(
    definition: &StrategyDefinition,
) -> Result<StrategyDefinition, StrategyIrError> {
    let declared = validate_definition(definition)?;
    drop(declared);

    let mut normalized = definition.clone();
    normalized
        .parameters
        .sort_by(|left, right| left.id.cmp(&right.id));
    normalized
        .indicators
        .sort_by(|left, right| left.id.cmp(&right.id));
    normalized.roles.sort_by(|left, right| {
        left.role
            .wire_tag()
            .cmp(right.role.wire_tag())
            .then_with(|| left.indicator.cmp(&right.indicator))
    });
    normalized.metadata.tags.sort();
    normalized
        .session
        .windows
        .sort_by_key(|window| (window.start_minute, window.end_minute));
    normalize_condition(&mut normalized.long.entry);
    normalize_condition(&mut normalized.long.exit);
    normalize_condition(&mut normalized.short.entry);
    normalize_condition(&mut normalized.short.exit);
    Ok(normalized)
}

fn normalize_condition(condition: &mut Condition) {
    match condition {
        Condition::Not(inner) => normalize_condition(inner),
        Condition::All(children) | Condition::Any(children) => {
            for child in children.iter_mut() {
                normalize_condition(child);
            }
            children.sort_by_cached_key(condition_sort_key);
        }
        Condition::Always
        | Condition::Never
        | Condition::Compare { .. }
        | Condition::CrossesAbove { .. }
        | Condition::CrossesBelow { .. } => {}
    }
}

fn condition_sort_key(condition: &Condition) -> String {
    let mut digest = CanonicalDigest::new("typhoon.strategy.condition-sort.v1");
    hash_condition(&mut digest, condition);
    digest.finish_hex()
}

fn compute_validated_strategy_id(definition: &StrategyDefinition) -> String {
    let mut digest = CanonicalDigest::new(STRATEGY_ID_DOMAIN);
    digest.tagged_u32("schema_version", STRATEGY_IR_SCHEMA_VERSION);
    hash_metadata(&mut digest, &definition.metadata);
    hash_parameters(&mut digest, &definition.parameters);
    hash_indicators(&mut digest, &definition.indicators);
    hash_roles(&mut digest, &definition.roles);
    hash_direction(&mut digest, "long", &definition.long);
    hash_direction(&mut digest, "short", &definition.short);
    hash_session(&mut digest, &definition.session);
    hash_news(&mut digest, &definition.news);
    hash_sizing(&mut digest, &definition.sizing);
    hash_trade_management(&mut digest, &definition.trade_management);
    hash_timing(&mut digest, &definition.timing);
    digest.finish_hex()
}

fn validate_definition(
    definition: &StrategyDefinition,
) -> Result<DeclaredIds<'_>, StrategyIrError> {
    validate_metadata(&definition.metadata)?;
    let parameters = validate_parameters(&definition.parameters)?;
    let indicators = validate_indicator_ids(&definition.indicators)?;
    let declared = DeclaredIds {
        parameters,
        indicators,
    };

    validate_indicator_inputs(&definition.indicators, &declared)?;
    check_indicator_acyclic(&definition.indicators)?;
    validate_roles(&definition.roles, &declared)?;

    for (label, rules) in [("long", &definition.long), ("short", &definition.short)] {
        validate_condition(&rules.entry, &format!("{label}.entry"), &declared)?;
        validate_condition(&rules.exit, &format!("{label}.exit"), &declared)?;
    }
    if !definition.long.enabled && !definition.short.enabled {
        return Err(StrategyIrError::NoEnabledDirection);
    }

    validate_session(&definition.session)?;
    validate_news(&definition.news)?;
    validate_sizing(&definition.sizing, &declared)?;
    validate_trade_management(&definition.trade_management, &declared)?;
    validate_timing(&definition.timing)?;
    Ok(declared)
}

fn validate_metadata(metadata: &StrategyMetadata) -> Result<(), StrategyIrError> {
    check_text("metadata.name", &metadata.name, MAX_TEXT_LEN)?;
    check_text("metadata.author", &metadata.author, MAX_TEXT_LEN)?;
    check_optional_text("metadata.notes", metadata.notes.as_deref(), MAX_TEXT_LEN)?;
    check_size("metadata.tags", metadata.tags.len(), MAX_TAGS)?;
    for (index, tag) in metadata.tags.iter().enumerate() {
        check_text(&format!("metadata.tags[{index}]"), tag, MAX_TEXT_LEN)?;
    }
    Ok(())
}

fn validate_parameters(
    parameters: &[StrategyParameter],
) -> Result<BTreeMap<&str, &ParamValue>, StrategyIrError> {
    check_size("parameters", parameters.len(), MAX_PARAMETERS)?;
    let mut declared = BTreeMap::new();
    for (index, parameter) in parameters.iter().enumerate() {
        let field = format!("parameters[{index}]");
        check_stable_id(&format!("{field}.id"), &parameter.id)?;
        if declared
            .insert(parameter.id.as_str(), &parameter.value)
            .is_some()
        {
            return Err(StrategyIrError::DuplicateId {
                kind: RefKind::Parameter,
                id: parameter.id.clone(),
            });
        }
        validate_parameter_value(&field, parameter)?;
    }
    Ok(declared)
}

fn validate_parameter_value(
    field: &str,
    parameter: &StrategyParameter,
) -> Result<(), StrategyIrError> {
    if let ParamValue::Float(value) = parameter.value {
        check_finite(&format!("{field}.value"), value)?;
    }
    if let ParamValue::Text(text) = &parameter.value {
        check_text(&format!("{field}.value"), text, MAX_TEXT_LEN)?;
    }

    let Some(range) = &parameter.range else {
        return Ok(());
    };
    let range_field = format!("{field}.range");
    match (range, &parameter.value) {
        (ParamRange::Int { min, max }, ParamValue::Int(value)) => {
            if min > max {
                return Err(out_of_range(
                    &range_field,
                    format!("[{min}, {max}]"),
                    "min <= max",
                ));
            }
            if value < min || value > max {
                return Err(out_of_range(
                    &format!("{field}.value"),
                    value,
                    "a value inside the declared range",
                ));
            }
            Ok(())
        }
        (ParamRange::Float { min, max }, ParamValue::Float(value)) => {
            check_finite(&format!("{range_field}.min"), *min)?;
            check_finite(&format!("{range_field}.max"), *max)?;
            if min > max {
                return Err(out_of_range(
                    &range_field,
                    format!("[{}, {}]", format_f64(*min), format_f64(*max)),
                    "min <= max",
                ));
            }
            if value < min || value > max {
                return Err(out_of_range(
                    &format!("{field}.value"),
                    format_f64(*value),
                    "a value inside the declared range",
                ));
            }
            Ok(())
        }
        // A range on a non-numeric parameter, or a range whose type disagrees
        // with the value's, has no meaning the optimizer could act on.
        _ => Err(out_of_range(
            &range_field,
            range.wire_tag(),
            "a range matching the parameter's own type",
        )),
    }
}

fn validate_indicator_ids(indicators: &[IndicatorNode]) -> Result<BTreeSet<&str>, StrategyIrError> {
    check_size("indicators", indicators.len(), MAX_INDICATORS)?;
    let mut declared = BTreeSet::new();
    for (index, indicator) in indicators.iter().enumerate() {
        check_stable_id(&format!("indicators[{index}].id"), &indicator.id)?;
        if !declared.insert(indicator.id.as_str()) {
            return Err(StrategyIrError::DuplicateId {
                kind: RefKind::Indicator,
                id: indicator.id.clone(),
            });
        }
        if let IndicatorKind::Custom {
            name,
            implementation_id,
        } = &indicator.kind
        {
            check_stable_id(&format!("indicators[{index}].kind.name"), name)?;
            check_digest_id(
                &format!("indicators[{index}].kind.implementation_id"),
                implementation_id,
            )?;
        }
    }
    Ok(declared)
}

fn validate_indicator_inputs(
    indicators: &[IndicatorNode],
    declared: &DeclaredIds<'_>,
) -> Result<(), StrategyIrError> {
    for (indicator_index, indicator) in indicators.iter().enumerate() {
        let context = format!("indicator `{}`", indicator.id);
        check_size(
            "indicator inputs",
            indicator.inputs.len(),
            MAX_INDICATOR_INPUTS,
        )?;
        if let Some(expected) = indicator.kind.input_shape() {
            if indicator.inputs.len() != expected.len() {
                return Err(out_of_range(
                    &format!("indicators[{indicator_index}].inputs"),
                    indicator.inputs.len(),
                    "the built-in indicator's exact input arity",
                ));
            }
            for (index, (input, expected_shape)) in
                indicator.inputs.iter().zip(expected).enumerate()
            {
                if input.shape() != *expected_shape {
                    return Err(out_of_range(
                        &format!("indicators[{indicator_index}].inputs[{index}]"),
                        format!("{:?}", input.shape()).to_ascii_lowercase(),
                        match expected_shape {
                            IndicatorInputShape::Series => "a price or indicator series",
                            IndicatorInputShape::Scalar => "a numeric constant or parameter",
                        },
                    ));
                }
            }
        }
        for (index, input) in indicator.inputs.iter().enumerate() {
            match input {
                IndicatorInput::Constant(value) => {
                    check_finite(&format!("{context}.inputs[{index}]"), *value)?
                }
                IndicatorInput::Parameter(id) => declared.check_numeric_parameter(id, &context)?,
                IndicatorInput::Indicator(id) => {
                    declared.check_ref(RefKind::Indicator, id, &context)?
                }
                IndicatorInput::Price(_) => {}
            }
        }
    }
    Ok(())
}

/// Depth-first cycle check over indicator→indicator edges.
///
/// Nodes are visited in declaration order and edges in input order, so the
/// reported cycle is deterministic. References were resolved beforehand, so a
/// missing edge target is impossible here.
fn check_indicator_acyclic(indicators: &[IndicatorNode]) -> Result<(), StrategyIrError> {
    #[derive(Clone, Copy, PartialEq)]
    enum Mark {
        Unvisited,
        InProgress,
        Done,
    }

    let index_of: BTreeMap<&str, usize> = indicators
        .iter()
        .enumerate()
        .map(|(index, indicator)| (indicator.id.as_str(), index))
        .collect();
    let mut marks = vec![Mark::Unvisited; indicators.len()];

    // Explicit stack: recursion here would be bounded by MAX_INDICATORS, but
    // an iterative walk keeps the bound obvious.
    for start in 0..indicators.len() {
        if marks[start] != Mark::Unvisited {
            continue;
        }
        let mut stack = vec![(start, 0usize)];
        marks[start] = Mark::InProgress;

        while let Some((node, cursor)) = stack.pop() {
            let Some(input) = indicators[node].inputs.get(cursor) else {
                marks[node] = Mark::Done;
                continue;
            };
            stack.push((node, cursor + 1));

            let IndicatorInput::Indicator(target_id) = input else {
                continue;
            };
            let target = index_of[target_id.as_str()];
            match marks[target] {
                Mark::Done => {}
                Mark::Unvisited => {
                    marks[target] = Mark::InProgress;
                    stack.push((target, 0));
                }
                Mark::InProgress => {
                    let mut path: Vec<String> = stack
                        .iter()
                        .skip_while(|(node, _)| *node != target)
                        .map(|(node, _)| indicators[*node].id.clone())
                        .collect();
                    path.push(indicators[target].id.clone());
                    return Err(StrategyIrError::IndicatorCycle { path });
                }
            }
        }
    }
    Ok(())
}

fn validate_roles(
    roles: &[RoleAssignment],
    declared: &DeclaredIds<'_>,
) -> Result<(), StrategyIrError> {
    let mut seen = BTreeSet::new();
    for assignment in roles {
        if !seen.insert(assignment.role) {
            return Err(StrategyIrError::DuplicateRole {
                role: assignment.role,
            });
        }
        declared.check_ref(
            RefKind::Indicator,
            &assignment.indicator,
            &format!("role `{}`", assignment.role.wire_tag()),
        )?;
    }
    Ok(())
}

fn validate_condition(
    condition: &Condition,
    context: &str,
    declared: &DeclaredIds<'_>,
) -> Result<(), StrategyIrError> {
    let mut nodes = 0usize;
    validate_condition_node(condition, context, declared, 1, &mut nodes)
}

fn validate_condition_node(
    condition: &Condition,
    context: &str,
    declared: &DeclaredIds<'_>,
    depth: usize,
    nodes: &mut usize,
) -> Result<(), StrategyIrError> {
    if depth > MAX_CONDITION_DEPTH {
        return Err(StrategyIrError::ConditionTooDeep {
            limit: MAX_CONDITION_DEPTH,
            found: depth,
        });
    }
    *nodes += 1;
    if *nodes > MAX_CONDITION_NODES {
        return Err(StrategyIrError::ConditionTooLarge {
            limit: MAX_CONDITION_NODES,
            found: *nodes,
        });
    }

    match condition {
        Condition::Always | Condition::Never => Ok(()),
        Condition::Not(inner) => {
            validate_condition_node(inner, context, declared, depth + 1, nodes)
        }
        Condition::All(children) | Condition::Any(children) => {
            if children.is_empty() {
                return Err(out_of_range(
                    &format!("{context}.{}", condition.wire_tag()),
                    0,
                    "at least one child condition",
                ));
            }
            for child in children {
                validate_condition_node(child, context, declared, depth + 1, nodes)?;
            }
            Ok(())
        }
        Condition::Compare { left, right, .. }
        | Condition::CrossesAbove { left, right }
        | Condition::CrossesBelow { left, right } => {
            validate_operand(left, context, declared)?;
            validate_operand(right, context, declared)
        }
    }
}

fn validate_operand(
    operand: &Operand,
    context: &str,
    declared: &DeclaredIds<'_>,
) -> Result<(), StrategyIrError> {
    match operand {
        Operand::Constant(value) => check_finite(context, *value),
        Operand::Parameter(id) => declared.check_numeric_parameter(id, context),
        Operand::Price { bars_ago, .. } => check_bars_ago(context, *bars_ago),
        Operand::Indicator { id, bars_ago } => {
            declared.check_ref(RefKind::Indicator, id, context)?;
            check_bars_ago(context, *bars_ago)
        }
    }
}

fn check_bars_ago(context: &str, bars_ago: u32) -> Result<(), StrategyIrError> {
    if bars_ago <= MAX_BARS_AGO {
        Ok(())
    } else {
        Err(out_of_range(
            &format!("{context}.bars_ago"),
            bars_ago,
            "a lookback within MAX_BARS_AGO",
        ))
    }
}

fn validate_session(session: &SessionFilter) -> Result<(), StrategyIrError> {
    check_size(
        "session.windows",
        session.windows.len(),
        MAX_SESSION_WINDOWS,
    )?;
    if session.enabled && session.windows.is_empty() {
        return Err(out_of_range(
            "session.windows",
            0,
            "at least one window while the filter is enabled",
        ));
    }

    let mut previous_end: Option<u32> = None;
    for (index, window) in session.windows.iter().enumerate() {
        let field = format!("session.windows[{index}]");
        if window.start_minute >= window.end_minute {
            return Err(out_of_range(
                &field,
                format!("[{}, {})", window.start_minute, window.end_minute),
                "start_minute < end_minute (a window may not wrap past midnight)",
            ));
        }
        if window.end_minute > MINUTES_PER_DAY {
            return Err(out_of_range(
                &format!("{field}.end_minute"),
                window.end_minute,
                "a UTC minute within the day",
            ));
        }
        if previous_end.is_some_and(|end| window.start_minute < end) {
            return Err(out_of_range(
                &field,
                window.start_minute,
                "windows in ascending, non-overlapping order",
            ));
        }
        previous_end = Some(window.end_minute);
    }
    Ok(())
}

fn validate_news(news: &NewsFilter) -> Result<(), StrategyIrError> {
    for (field, minutes) in [
        ("news.block_minutes_before", news.block_minutes_before),
        ("news.block_minutes_after", news.block_minutes_after),
    ] {
        if minutes > MAX_NEWS_BLOCK_MINUTES {
            return Err(out_of_range(
                field,
                minutes,
                "a blackout within MAX_NEWS_BLOCK_MINUTES",
            ));
        }
    }
    if news.enabled && news.block_minutes_before == 0 && news.block_minutes_after == 0 {
        return Err(out_of_range(
            "news.block_minutes_before + news.block_minutes_after",
            0,
            "a non-zero blackout while the filter is enabled",
        ));
    }
    Ok(())
}

fn validate_sizing(
    sizing: &PositionSizing,
    declared: &DeclaredIds<'_>,
) -> Result<(), StrategyIrError> {
    if sizing.max_open_positions == 0 || sizing.max_open_positions > MAX_OPEN_POSITIONS {
        return Err(out_of_range(
            "sizing.max_open_positions",
            sizing.max_open_positions,
            "between 1 and MAX_OPEN_POSITIONS",
        ));
    }
    match &sizing.rule {
        SizingRule::FixedUnits { units } => {
            check_finite_in("sizing.units", *units, *units > 0.0, "a positive quantity")
        }
        SizingRule::LegacyFixedNotionalV1 { notional } => check_finite_in(
            "sizing.notional",
            *notional,
            *notional > 0.0,
            "a positive notional",
        ),
        SizingRule::PercentEquity { percent } => check_finite_in(
            "sizing.percent",
            *percent,
            *percent > 0.0 && *percent <= 100.0,
            "a percentage in (0, 100]",
        ),
        SizingRule::RiskPercentAtr {
            risk_percent,
            atr_multiple,
            atr_indicator,
        } => {
            check_finite_in(
                "sizing.risk_percent",
                *risk_percent,
                *risk_percent > 0.0 && *risk_percent <= 100.0,
                "a percentage in (0, 100]",
            )?;
            check_finite_in(
                "sizing.atr_multiple",
                *atr_multiple,
                *atr_multiple > 0.0,
                "a positive multiple",
            )?;
            declared.check_ref(RefKind::Indicator, atr_indicator, "sizing")
        }
    }
}

fn validate_trade_management(
    management: &TradeManagement,
    declared: &DeclaredIds<'_>,
) -> Result<(), StrategyIrError> {
    check_size(
        "trade_management.legs",
        management.legs.len(),
        MAX_TRADE_LEGS,
    )?;
    if management.legs.is_empty() {
        return Err(out_of_range("trade_management.legs", 0, "at least one leg"));
    }

    let mut total_bps: u64 = 0;
    for (index, leg) in management.legs.iter().enumerate() {
        let field = format!("trade_management.legs[{index}]");
        if leg.fraction_bps == 0 {
            return Err(out_of_range(
                &format!("{field}.fraction_bps"),
                0,
                "a non-zero share of the position",
            ));
        }
        total_bps += u64::from(leg.fraction_bps);

        for (label, rule) in [("stop", &leg.stop), ("target", &leg.target)] {
            if let Some(rule) = rule {
                validate_stop_rule(rule, &format!("{field}.{label}"), declared)?;
            }
        }
        if let Some(trailing) = &leg.trailing {
            validate_stop_rule(
                &trailing.distance,
                &format!("{field}.trailing.distance"),
                declared,
            )?;
            if let Some(activate) = &trailing.activate_after {
                validate_stop_rule(
                    activate,
                    &format!("{field}.trailing.activate_after"),
                    declared,
                )?;
            }
        }
    }
    if total_bps != u64::from(TOTAL_FRACTION_BPS) {
        return Err(out_of_range(
            "trade_management.legs.fraction_bps",
            total_bps,
            "leg fractions totalling exactly 10000 basis points",
        ));
    }

    if let Some(break_even) = &management.break_even_after {
        validate_stop_rule(break_even, "trade_management.break_even_after", declared)?;
    }
    if let Some(max_bars) = management.max_bars_in_trade
        && (max_bars == 0 || max_bars > MAX_BARS_IN_TRADE)
    {
        return Err(out_of_range(
            "trade_management.max_bars_in_trade",
            max_bars,
            "between 1 and the configured maximum",
        ));
    }
    Ok(())
}

fn validate_stop_rule(
    rule: &StopRule,
    field: &str,
    declared: &DeclaredIds<'_>,
) -> Result<(), StrategyIrError> {
    match rule {
        StopRule::AtrMultiple {
            indicator,
            multiple,
        } => {
            check_finite_in(
                &format!("{field}.multiple"),
                *multiple,
                *multiple > 0.0,
                "a positive multiple",
            )?;
            declared.check_ref(RefKind::Indicator, indicator, field)
        }
        StopRule::PercentOfEntry { percent } => check_finite_in(
            &format!("{field}.percent"),
            *percent,
            *percent > 0.0 && *percent <= 100.0,
            "a percentage in (0, 100]",
        ),
        StopRule::PriceDistance { distance } => check_finite_in(
            &format!("{field}.distance"),
            *distance,
            *distance > 0.0,
            "a positive price distance",
        ),
    }
}

fn validate_timing(timing: &ExecutionTiming) -> Result<(), StrategyIrError> {
    if let DecisionTiming::PreClose { offset_seconds } = timing.decision {
        if offset_seconds == 0 {
            return Err(out_of_range(
                "timing.decision.offset_seconds",
                0,
                "a positive offset before the close",
            ));
        }
        if offset_seconds > MAX_PRE_CLOSE_OFFSET_SECONDS {
            return Err(out_of_range(
                "timing.decision.offset_seconds",
                offset_seconds,
                "an offset within MAX_PRE_CLOSE_OFFSET_SECONDS",
            ));
        }
    } else if timing.forming_bar_visible {
        return Err(StrategyIrError::InconsistentTiming {
            detail: "only a pre-close decision may observe the forming bar",
        });
    }

    if timing.submit_delay_bars > MAX_SUBMIT_DELAY_BARS {
        return Err(out_of_range(
            "timing.submit_delay_bars",
            timing.submit_delay_bars,
            "a delay within MAX_SUBMIT_DELAY_BARS",
        ));
    }
    Ok(())
}

// ── Strategy encoding ──────────────────────────────────────────────

fn hash_metadata(digest: &mut CanonicalDigest, metadata: &StrategyMetadata) {
    digest.tagged_text("metadata.name", &metadata.name);
    digest.tagged_text("metadata.author", &metadata.author);
    digest.begin_option("metadata.notes", metadata.notes.is_some());
    if let Some(notes) = &metadata.notes {
        digest.tagged_text("notes", notes);
    }
    digest.begin_seq("metadata.tags", metadata.tags.len());
    for tag in &metadata.tags {
        digest.tagged_text("tag", tag);
    }
}

fn hash_parameters(digest: &mut CanonicalDigest, parameters: &[StrategyParameter]) {
    digest.begin_seq("parameters", parameters.len());
    for parameter in parameters {
        digest.tagged_text("parameter.id", &parameter.id);
        digest.tagged_text("parameter.type", parameter.value.wire_tag());
        match &parameter.value {
            ParamValue::Bool(value) => digest.tagged_bool("value", *value),
            ParamValue::Int(value) => digest.tagged_i64("value", *value),
            ParamValue::Float(value) => digest.tagged_f64("value", *value),
            ParamValue::Text(value) => digest.tagged_text("value", value),
        }
        digest.begin_option("parameter.range", parameter.range.is_some());
        if let Some(range) = &parameter.range {
            digest.tagged_text("range.type", range.wire_tag());
            match range {
                ParamRange::Int { min, max } => {
                    digest.tagged_i64("min", *min);
                    digest.tagged_i64("max", *max);
                }
                ParamRange::Float { min, max } => {
                    digest.tagged_f64("min", *min);
                    digest.tagged_f64("max", *max);
                }
            }
        }
    }
}

fn hash_indicators(digest: &mut CanonicalDigest, indicators: &[IndicatorNode]) {
    digest.begin_seq("indicators", indicators.len());
    for indicator in indicators {
        digest.tagged_text("indicator.id", &indicator.id);
        digest.tagged_text("indicator.kind", indicator.kind.wire_tag());
        if let IndicatorKind::Custom {
            name,
            implementation_id,
        } = &indicator.kind
        {
            digest.tagged_text("kind.name", name);
            digest.tagged_text("kind.implementation_id", implementation_id);
        }
        digest.begin_seq("indicator.inputs", indicator.inputs.len());
        for input in &indicator.inputs {
            digest.tagged_text("input.type", input.wire_tag());
            match input {
                IndicatorInput::Constant(value) => digest.tagged_f64("constant", *value),
                IndicatorInput::Parameter(id) => digest.tagged_text("parameter", id),
                IndicatorInput::Indicator(id) => digest.tagged_text("indicator", id),
                IndicatorInput::Price(field) => digest.tagged_text("price", field.wire_tag()),
            }
        }
    }
}

fn hash_roles(digest: &mut CanonicalDigest, roles: &[RoleAssignment]) {
    digest.begin_seq("roles", roles.len());
    for assignment in roles {
        digest.tagged_text("role", assignment.role.wire_tag());
        digest.tagged_text("role.indicator", &assignment.indicator);
    }
}

fn hash_direction(digest: &mut CanonicalDigest, tag: &str, rules: &DirectionRules) {
    digest.tagged_bool(&format!("{tag}.enabled"), rules.enabled);
    digest.frame(format!("{tag}.entry").as_bytes());
    hash_condition(digest, &rules.entry);
    digest.frame(format!("{tag}.exit").as_bytes());
    hash_condition(digest, &rules.exit);
}

/// Encode a condition tree. The variant tag comes first and every child list
/// writes its length, so neither the shape nor the operand positions can be
/// re-framed into a different tree.
fn hash_condition(digest: &mut CanonicalDigest, condition: &Condition) {
    digest.tagged_text("condition", condition.wire_tag());
    match condition {
        Condition::Always | Condition::Never => {}
        Condition::Not(inner) => hash_condition(digest, inner),
        Condition::All(children) | Condition::Any(children) => {
            digest.begin_seq("children", children.len());
            for child in children {
                hash_condition(digest, child);
            }
        }
        Condition::Compare { left, op, right } => {
            digest.tagged_text("op", op.wire_tag());
            hash_operand(digest, "left", left);
            hash_operand(digest, "right", right);
        }
        Condition::CrossesAbove { left, right } | Condition::CrossesBelow { left, right } => {
            hash_operand(digest, "left", left);
            hash_operand(digest, "right", right);
        }
    }
}

fn hash_operand(digest: &mut CanonicalDigest, position: &str, operand: &Operand) {
    digest.tagged_text(position, operand.wire_tag());
    match operand {
        Operand::Constant(value) => digest.tagged_f64("constant", *value),
        Operand::Parameter(id) => digest.tagged_text("parameter", id),
        Operand::Price { field, bars_ago } => {
            digest.tagged_text("price", field.wire_tag());
            digest.tagged_u32("bars_ago", *bars_ago);
        }
        Operand::Indicator { id, bars_ago } => {
            digest.tagged_text("indicator", id);
            digest.tagged_u32("bars_ago", *bars_ago);
        }
    }
}

fn hash_session(digest: &mut CanonicalDigest, session: &SessionFilter) {
    digest.tagged_bool("session.enabled", session.enabled);
    digest.tagged_bool("session.close_outside", session.close_positions_outside);
    digest.begin_seq("session.windows", session.windows.len());
    for window in &session.windows {
        digest.tagged_u32("window.start", window.start_minute);
        digest.tagged_u32("window.end", window.end_minute);
    }
}

fn hash_news(digest: &mut CanonicalDigest, news: &NewsFilter) {
    digest.tagged_bool("news.enabled", news.enabled);
    digest.tagged_text("news.min_impact", news.min_impact.wire_tag());
    digest.tagged_u32("news.before", news.block_minutes_before);
    digest.tagged_u32("news.after", news.block_minutes_after);
    digest.tagged_bool("news.close_open", news.close_open_positions);
}

fn hash_sizing(digest: &mut CanonicalDigest, sizing: &PositionSizing) {
    digest.tagged_text("sizing.rule", sizing.rule.wire_tag());
    match &sizing.rule {
        SizingRule::FixedUnits { units } => digest.tagged_f64("units", *units),
        SizingRule::LegacyFixedNotionalV1 { notional } => digest.tagged_f64("notional", *notional),
        SizingRule::PercentEquity { percent } => digest.tagged_f64("percent", *percent),
        SizingRule::RiskPercentAtr {
            risk_percent,
            atr_multiple,
            atr_indicator,
        } => {
            digest.tagged_f64("risk_percent", *risk_percent);
            digest.tagged_f64("atr_multiple", *atr_multiple);
            digest.tagged_text("atr_indicator", atr_indicator);
        }
    }
    digest.tagged_u32("sizing.max_open_positions", sizing.max_open_positions);
}

fn hash_trade_management(digest: &mut CanonicalDigest, management: &TradeManagement) {
    digest.begin_seq("trade.legs", management.legs.len());
    for leg in &management.legs {
        digest.tagged_u32("leg.fraction_bps", leg.fraction_bps);
        hash_optional_stop(digest, "leg.stop", leg.stop.as_ref());
        hash_optional_stop(digest, "leg.target", leg.target.as_ref());
        digest.begin_option("leg.trailing", leg.trailing.is_some());
        if let Some(trailing) = &leg.trailing {
            hash_stop_rule(digest, "trailing.distance", &trailing.distance);
            hash_optional_stop(
                digest,
                "trailing.activate_after",
                trailing.activate_after.as_ref(),
            );
        }
    }
    hash_optional_stop(
        digest,
        "trade.break_even_after",
        management.break_even_after.as_ref(),
    );
    digest.begin_option("trade.max_bars", management.max_bars_in_trade.is_some());
    if let Some(bars) = management.max_bars_in_trade {
        digest.tagged_u32("max_bars", bars);
    }
}

fn hash_optional_stop(digest: &mut CanonicalDigest, tag: &str, rule: Option<&StopRule>) {
    digest.begin_option(tag, rule.is_some());
    if let Some(rule) = rule {
        hash_stop_rule(digest, tag, rule);
    }
}

fn hash_stop_rule(digest: &mut CanonicalDigest, tag: &str, rule: &StopRule) {
    digest.tagged_text(tag, rule.wire_tag());
    match rule {
        StopRule::AtrMultiple {
            indicator,
            multiple,
        } => {
            digest.tagged_text("indicator", indicator);
            digest.tagged_f64("multiple", *multiple);
        }
        StopRule::PercentOfEntry { percent } => digest.tagged_f64("percent", *percent),
        StopRule::PriceDistance { distance } => digest.tagged_f64("distance", *distance),
    }
}

fn hash_timing(digest: &mut CanonicalDigest, timing: &ExecutionTiming) {
    digest.tagged_text("timing.decision", timing.decision.wire_tag());
    if let DecisionTiming::PreClose { offset_seconds } = timing.decision {
        digest.tagged_u32("offset_seconds", offset_seconds);
    }
    digest.tagged_bool("timing.forming_bar_visible", timing.forming_bar_visible);
    digest.tagged_u32("timing.submit_delay_bars", timing.submit_delay_bars);
}

// ── Config identity ────────────────────────────────────────────────

/// The content-addressed config id: lowercase hex SHA-256 over the canonical
/// encoding of fully validated execution settings.
pub fn compute_config_id(settings: &ExecutionSettings) -> Result<String, StrategyIrError> {
    validate_settings(settings)?;

    let mut digest = CanonicalDigest::new(CONFIG_ID_DOMAIN);
    digest.tagged_u32("schema_version", STRATEGY_EXECUTION_CONFIG_SCHEMA_VERSION);
    digest.tagged_f64("initial_capital", settings.initial_capital);
    digest.tagged_text("account_currency", &settings.account_currency);

    digest.tagged_text("commission", settings.commission.wire_tag());
    match &settings.commission {
        CommissionModel::None => {}
        CommissionModel::PerShare { amount, minimum } => {
            digest.tagged_f64("amount", *amount);
            digest.tagged_f64("minimum", *minimum);
        }
        CommissionModel::PercentOfNotional { percent, minimum } => {
            digest.tagged_f64("percent", *percent);
            digest.tagged_f64("minimum", *minimum);
        }
        CommissionModel::PerOrder { amount } => digest.tagged_f64("amount", *amount),
        CommissionModel::VenueSchedule(binding) => digest_fee_binding(&mut digest, binding),
    }

    digest.tagged_text("slippage", settings.slippage.wire_tag());
    match &settings.slippage {
        SlippageModel::None => {}
        SlippageModel::FixedPriceDistance { distance } => digest.tagged_f64("distance", *distance),
        SlippageModel::SpreadFraction { fraction } => digest.tagged_f64("fraction", *fraction),
        SlippageModel::VolatilityScaled { atr_fraction } => {
            digest.tagged_f64("atr_fraction", *atr_fraction)
        }
    }

    digest.tagged_text("spread", settings.spread.wire_tag());
    match &settings.spread {
        SpreadModel::None | SpreadModel::RecordedQuotes => {}
        SpreadModel::Constant { price_units } => digest.tagged_f64("price_units", *price_units),
        SpreadModel::PercentOfPrice { percent } => digest.tagged_f64("percent", *percent),
    }

    digest.tagged_text("ambiguity", settings.ambiguity.wire_tag());
    digest.tagged_text("tie_break", settings.tie_break.wire_tag());

    digest.tagged_text("fidelity", settings.fidelity.wire_tag());
    if let Some(seconds) = settings.fidelity.sub_bar_seconds() {
        digest.tagged_u32("fidelity.sub_bar_seconds", seconds);
    }
    digest.tagged_text("latency", settings.latency.wire_tag());
    match settings.latency {
        LatencyModel::None => {}
        LatencyModel::Fixed {
            decision_to_submit_ns,
            submit_to_exchange_ns,
        } => {
            digest.tagged_i64("decision_to_submit_ns", decision_to_submit_ns);
            digest.tagged_i64("submit_to_exchange_ns", submit_to_exchange_ns);
        }
        LatencyModel::SeededUniform {
            decision_to_submit_min_ns,
            decision_to_submit_max_ns,
            submit_to_exchange_min_ns,
            submit_to_exchange_max_ns,
        } => {
            digest.tagged_i64("decision_to_submit_min_ns", decision_to_submit_min_ns);
            digest.tagged_i64("decision_to_submit_max_ns", decision_to_submit_max_ns);
            digest.tagged_i64("submit_to_exchange_min_ns", submit_to_exchange_min_ns);
            digest.tagged_i64("submit_to_exchange_max_ns", submit_to_exchange_max_ns);
        }
    }
    digest.tagged_text("margin", settings.margin.wire_tag());
    digest.begin_option("price_tick", settings.price_tick.is_some());
    if let Some(tick) = settings.price_tick {
        digest.tagged_f64("price_tick.value", tick);
    }
    digest.tagged_u32("warmup_bars", settings.warmup_bars);
    digest.tagged_text("compatibility", settings.compatibility.wire_tag());

    digest.tagged_text("participation", settings.participation.wire_tag());
    if let ParticipationModel::BarVolumeFraction { fraction } = settings.participation {
        digest.tagged_f64("participation.fraction", fraction);
    }
    digest.tagged_text("outside_session", settings.outside_session.wire_tag());
    digest_instruments(&mut digest, &settings.instruments);
    digest_corporate_actions(&mut digest, &settings.corporate_actions);
    digest_currency_conversion(&mut digest, &settings.currency_conversion);
    Ok(digest.finish_hex())
}

/// Canonically encode the per-instrument registry. Every field an operator
/// chose — the calendar id, the quote currency, each rate and its provenance —
/// is framed, so two runs that assumed different venues or different borrow
/// costs can never share a config id.
fn digest_instruments(digest: &mut CanonicalDigest, registry: &InstrumentRegistry) {
    digest.begin_seq("instruments", registry.specs().len());
    for spec in registry.specs() {
        digest.tagged_text("instrument.symbol", &spec.symbol);
        digest.tagged_text("instrument.currency", &spec.currency);
        digest.begin_option("instrument.calendar", spec.calendar.is_some());
        if let Some(calendar) = &spec.calendar {
            // The calendar id is itself a framed digest over the whole spec, so
            // naming it here is exact rather than a summary.
            digest.tagged_u32("instrument.calendar.schema", calendar.schema_version());
            digest.tagged_text("instrument.calendar.id", calendar.calendar_id());
        }
        digest.begin_option("instrument.price_tick", spec.price_tick.is_some());
        if let Some(tick) = spec.price_tick {
            digest.tagged_f64("instrument.price_tick.value", tick);
        }
        digest.tagged_text("instrument.financing", spec.financing.wire_id());
        if let Some(policy) = spec.financing.policy() {
            digest.tagged_text("financing.day_count", policy.day_count.wire_id());
            digest.tagged_text("financing.accrual", policy.accrual.wire_id());
            digest.tagged_u64("financing.accrual_seconds", policy.accrual.seconds() as u64);
            for (tag, rate) in [
                ("financing.long", &policy.long_financing_annual_percent),
                ("financing.short", &policy.short_financing_annual_percent),
                ("financing.borrow", &policy.short_borrow_annual_percent),
                ("financing.funding", &policy.funding_interval_percent),
            ] {
                digest_rate_source(digest, tag, rate);
            }
        }
    }
}

fn digest_rate_source(digest: &mut CanonicalDigest, tag: &str, rate: &RateSource) {
    digest.tagged_text(tag, rate.wire_id());
    match rate {
        RateSource::NotApplicable => {}
        RateSource::Unavailable { reason } => digest.tagged_text("rate.reason", reason),
        RateSource::Declared {
            percent,
            provenance,
        } => {
            digest.tagged_f64("rate.percent", *percent);
            digest_rate_provenance(digest, provenance);
        }
    }
}

fn digest_rate_provenance(digest: &mut CanonicalDigest, provenance: &RateProvenance) {
    digest.tagged_text("rate.provenance", provenance.wire_id());
    match provenance {
        RateProvenance::OperatorAssumption { note } => digest.tagged_text("rate.note", note),
        RateProvenance::VendorPublished {
            source,
            retrieved_date,
        } => {
            digest.tagged_text("rate.source", source);
            digest.tagged_text("rate.retrieved_date", retrieved_date);
        }
    }
}

fn digest_corporate_actions(digest: &mut CanonicalDigest, schedule: &CorporateActionSchedule) {
    digest.begin_seq("corporate_actions", schedule.actions().len());
    for action in schedule.actions() {
        digest.tagged_text("action.symbol", &action.symbol);
        digest.tagged_i64("action.effective_time_ns", action.effective_time_ns);
        digest.tagged_text("action.kind", action.kind.wire_id());
        match &action.kind {
            CorporateActionKind::Split {
                numerator,
                denominator,
            } => {
                digest.tagged_u32("action.numerator", *numerator);
                digest.tagged_u32("action.denominator", *denominator);
            }
            CorporateActionKind::CashDividend { amount_per_unit } => {
                digest.tagged_f64("action.amount_per_unit", *amount_per_unit);
            }
            CorporateActionKind::SymbolChange { new_symbol } => {
                digest.tagged_text("action.new_symbol", new_symbol);
            }
            CorporateActionKind::Delisting => {}
        }
    }
}

fn digest_currency_conversion(digest: &mut CanonicalDigest, conversion: &CurrencyConversion) {
    digest.tagged_text("currency_conversion", conversion.wire_id());
    digest.begin_seq("currency_conversion.rates", conversion.rates().len());
    for rate in conversion.rates() {
        digest.tagged_text("currency.code", &rate.currency);
        digest.tagged_f64("currency.account_per_unit", rate.account_per_unit);
        digest.tagged_f64("currency.spread_percent", rate.spread_percent);
        digest_rate_provenance(digest, &rate.provenance);
    }
}

/// Canonically encode a bound fee schedule. Every field an operator chose —
/// venue, version, effective date, provenance, each rate, the tier, and the
/// maker/taker assumption — is framed, so no two assumptions share an id.
fn digest_fee_binding(digest: &mut CanonicalDigest, binding: &FeeScheduleBinding) {
    let schedule = binding.schedule();
    digest.tagged_u32("fee.schema_version", schedule.schema_version());
    digest.tagged_text("fee.venue", schedule.venue().wire_id());
    digest.tagged_u32("fee.schedule_version", schedule.schedule_version());
    digest.tagged_text("fee.effective_date", schedule.effective_date());
    digest.tagged_text("fee.provenance", schedule.provenance().wire_tag());
    match schedule.provenance() {
        FeeProvenance::OperatorAssumption { note } => digest.tagged_text("fee.note", note),
        FeeProvenance::VendorPublished {
            source,
            retrieved_date,
        } => {
            digest.tagged_text("fee.source", source);
            digest.tagged_text("fee.retrieved_date", retrieved_date);
        }
    }
    digest.tagged_text("fee.shape", schedule.shape().wire_tag());
    match schedule.shape() {
        FeeScheduleShape::KrakenSpot { tiers } => {
            digest.begin_seq("fee.tiers", tiers.len());
            for tier in tiers {
                digest.tagged_f64("fee.tier.min_volume", tier.min_volume);
                digest.tagged_f64("fee.tier.maker_percent", tier.maker_percent);
                digest.tagged_f64("fee.tier.taker_percent", tier.taker_percent);
            }
        }
        FeeScheduleShape::AlpacaUsEquity {
            per_share,
            minimum,
            sell_notional_percent,
            sell_per_share,
            sell_per_order_cap,
        } => {
            digest.tagged_f64("fee.per_share", *per_share);
            digest.tagged_f64("fee.minimum", *minimum);
            digest.tagged_f64("fee.sell_notional_percent", *sell_notional_percent);
            digest.tagged_f64("fee.sell_per_share", *sell_per_share);
            digest.tagged_f64("fee.sell_per_order_cap", *sell_per_order_cap);
        }
    }
    digest.tagged_u64("fee.tier_index", binding.tier_index() as u64);
    digest.tagged_text("fee.liquidity", binding.liquidity().wire_id());
}

fn validate_settings(settings: &ExecutionSettings) -> Result<(), StrategyIrError> {
    check_finite_in(
        "settings.initial_capital",
        settings.initial_capital,
        settings.initial_capital > 0.0,
        "a positive account balance",
    )?;
    check_text(
        "settings.account_currency",
        &settings.account_currency,
        MAX_TEXT_LEN,
    )?;

    // Costs may be zero — that is the zero-cost equivalence baseline of the M1
    // gate — but never negative, which would pay the account to trade.
    let non_negative = "a non-negative cost";
    match &settings.commission {
        CommissionModel::None => {}
        CommissionModel::PerShare { amount, minimum } => {
            check_finite_in(
                "settings.commission.amount",
                *amount,
                *amount >= 0.0,
                non_negative,
            )?;
            check_finite_in(
                "settings.commission.minimum",
                *minimum,
                *minimum >= 0.0,
                non_negative,
            )?;
        }
        CommissionModel::PercentOfNotional { percent, minimum } => {
            check_finite_in(
                "settings.commission.percent",
                *percent,
                *percent >= 0.0 && *percent <= 100.0,
                "a percentage in [0, 100]",
            )?;
            check_finite_in(
                "settings.commission.minimum",
                *minimum,
                *minimum >= 0.0,
                non_negative,
            )?;
        }
        CommissionModel::PerOrder { amount } => check_finite_in(
            "settings.commission.amount",
            *amount,
            *amount >= 0.0,
            non_negative,
        )?,
        CommissionModel::VenueSchedule(binding) => {
            binding
                .validate()
                .map_err(StrategyIrError::InvalidFeeSchedule)?;
        }
    }

    match &settings.slippage {
        SlippageModel::None => {}
        SlippageModel::FixedPriceDistance { distance } => check_finite_in(
            "settings.slippage.distance",
            *distance,
            *distance >= 0.0,
            non_negative,
        )?,
        SlippageModel::SpreadFraction { fraction } => check_finite_in(
            "settings.slippage.fraction",
            *fraction,
            *fraction >= 0.0,
            non_negative,
        )?,
        SlippageModel::VolatilityScaled { atr_fraction } => check_finite_in(
            "settings.slippage.atr_fraction",
            *atr_fraction,
            *atr_fraction >= 0.0,
            non_negative,
        )?,
    }

    match &settings.spread {
        SpreadModel::None | SpreadModel::RecordedQuotes => {}
        SpreadModel::Constant { price_units } => check_finite_in(
            "settings.spread.price_units",
            *price_units,
            *price_units >= 0.0,
            non_negative,
        )?,
        SpreadModel::PercentOfPrice { percent } => check_finite_in(
            "settings.spread.percent",
            *percent,
            *percent >= 0.0 && *percent <= 100.0,
            "a percentage in [0, 100]",
        )?,
    }

    validate_latency(&settings.latency)?;
    if let Some(tick) = settings.price_tick {
        check_finite_in(
            "settings.price_tick",
            tick,
            tick > 0.0,
            "a positive price increment",
        )?;
    }
    if settings.warmup_bars > MAX_WARMUP_BARS {
        return Err(StrategyIrError::OutOfRange {
            field: "settings.warmup_bars".to_string(),
            value: settings.warmup_bars.to_string(),
            expected: "a warm-up within MAX_WARMUP_BARS",
        });
    }
    // The legacy bridge fills at a price its own decision saw. That is only
    // defensible as an isolated comparison, so it may not be combined with
    // anything that would make it look like a realistic model.
    if settings.compatibility == ExecutionCompatibility::LegacySameBarClose {
        if settings.fidelity != FidelityLevel::BarClose {
            return Err(StrategyIrError::InconsistentExecution {
                detail: "legacy same-bar-close compatibility requires bar-close fidelity",
            });
        }
        if settings.latency != LatencyModel::None {
            return Err(StrategyIrError::InconsistentExecution {
                detail: "legacy same-bar-close compatibility requires zero latency",
            });
        }
        // The bridge exists to reproduce one number. Anything that changes what
        // that number means would make the comparison meaningless, so the
        // richer-execution machinery is refused rather than quietly ignored.
        if settings.participation != ParticipationModel::Unlimited
            || !settings.instruments.is_empty()
            || !settings.corporate_actions.is_empty()
            || settings.currency_conversion != CurrencyConversion::None
        {
            return Err(StrategyIrError::InconsistentExecution {
                detail: "legacy same-bar-close compatibility excludes participation caps, \
                         instrument specs, corporate actions and currency conversion",
            });
        }
    }
    validate_realism(settings)
}

/// The M2 execution-realism half of settings validation (§6.3, §6.6–§6.9).
fn validate_realism(settings: &ExecutionSettings) -> Result<(), StrategyIrError> {
    if let FidelityLevel::SubBar { sub_bar_seconds } = settings.fidelity
        && (sub_bar_seconds == 0 || sub_bar_seconds > MAX_SUB_BAR_SECONDS)
    {
        return Err(StrategyIrError::OutOfRange {
            field: "settings.fidelity.sub_bar_seconds".to_string(),
            value: sub_bar_seconds.to_string(),
            expected: "a positive sub-bar timeframe within MAX_SUB_BAR_SECONDS",
        });
    }
    if let ParticipationModel::BarVolumeFraction { fraction } = settings.participation {
        check_finite_in(
            "settings.participation.fraction",
            fraction,
            fraction > 0.0 && fraction <= 1.0,
            "a participation fraction in (0, 1]",
        )?;
    }

    settings
        .currency_conversion
        .validate(&settings.account_currency)
        .map_err(StrategyIrError::InvalidFinancing)?;
    settings
        .instruments
        .validate_shape()
        .map_err(StrategyIrError::InvalidInstrument)?;
    settings
        .instruments
        .validate_against_account(&settings.account_currency, &settings.currency_conversion)
        .map_err(StrategyIrError::InvalidInstrument)?;
    settings
        .corporate_actions
        .validate()
        .map_err(StrategyIrError::InvalidCorporateAction)?;
    Ok(())
}

fn validate_latency(latency: &LatencyModel) -> Result<(), StrategyIrError> {
    let leg = |field: &'static str, value: i64| -> Result<(), StrategyIrError> {
        if (0..=MAX_LATENCY_NS).contains(&value) {
            return Ok(());
        }
        Err(StrategyIrError::OutOfRange {
            field: field.to_string(),
            value: value.to_string(),
            expected: "a delay in [0, MAX_LATENCY_NS] nanoseconds",
        })
    };
    match *latency {
        LatencyModel::None => Ok(()),
        LatencyModel::Fixed {
            decision_to_submit_ns,
            submit_to_exchange_ns,
        } => {
            leg(
                "settings.latency.decision_to_submit_ns",
                decision_to_submit_ns,
            )?;
            leg(
                "settings.latency.submit_to_exchange_ns",
                submit_to_exchange_ns,
            )
        }
        LatencyModel::SeededUniform {
            decision_to_submit_min_ns,
            decision_to_submit_max_ns,
            submit_to_exchange_min_ns,
            submit_to_exchange_max_ns,
        } => {
            leg(
                "settings.latency.decision_to_submit_min_ns",
                decision_to_submit_min_ns,
            )?;
            leg(
                "settings.latency.decision_to_submit_max_ns",
                decision_to_submit_max_ns,
            )?;
            leg(
                "settings.latency.submit_to_exchange_min_ns",
                submit_to_exchange_min_ns,
            )?;
            leg(
                "settings.latency.submit_to_exchange_max_ns",
                submit_to_exchange_max_ns,
            )?;
            if decision_to_submit_max_ns < decision_to_submit_min_ns {
                return Err(StrategyIrError::OutOfRange {
                    field: "settings.latency.decision_to_submit_max_ns".to_string(),
                    value: decision_to_submit_max_ns.to_string(),
                    expected: "a maximum at or above the minimum",
                });
            }
            if submit_to_exchange_max_ns < submit_to_exchange_min_ns {
                return Err(StrategyIrError::OutOfRange {
                    field: "settings.latency.submit_to_exchange_max_ns".to_string(),
                    value: submit_to_exchange_max_ns.to_string(),
                    expected: "a maximum at or above the minimum",
                });
            }
            Ok(())
        }
    }
}

// ── Run identity ───────────────────────────────────────────────────

/// The content-addressed run id: lowercase hex SHA-256 over the canonical
/// encoding of a fully validated binding.
pub fn compute_run_id(binding: &RunBinding) -> Result<String, StrategyIrError> {
    let binding = normalize_binding(binding)?;
    Ok(compute_validated_run_id(&binding))
}

fn normalize_binding(binding: &RunBinding) -> Result<RunBinding, StrategyIrError> {
    validate_binding(binding)?;
    let mut normalized = binding.clone();
    normalized
        .datasets
        .sort_by(|left, right| left.input_id.cmp(&right.input_id));
    normalized
        .repaint_qa
        .sort_by(|left, right| left.indicator_id.cmp(&right.indicator_id));
    Ok(normalized)
}

fn compute_validated_run_id(binding: &RunBinding) -> String {
    let mut digest = CanonicalDigest::new(RUN_ID_DOMAIN);
    digest.tagged_u32("schema_version", STRATEGY_RUN_MANIFEST_SCHEMA_VERSION);
    digest.begin_seq("datasets", binding.datasets.len());
    for dataset in &binding.datasets {
        digest.tagged_text("input_id", &dataset.input_id);
        digest.tagged_text("dataset_id", &dataset.dataset_id);
    }
    digest.tagged_text("strategy_id", &binding.strategy_id);
    digest.tagged_text("config_id", &binding.config_id);
    digest.tagged_u64("seed", binding.seed);
    digest.tagged_text("engine_version", &binding.engine_version);
    digest.tagged_text("metrics_version", &binding.metrics_version);
    digest.begin_option("intervention_log_id", binding.intervention_log_id.is_some());
    if let Some(intervention_log_id) = &binding.intervention_log_id {
        digest.tagged_text("intervention_log_id", intervention_log_id);
    }
    digest.begin_seq("repaint_qa", binding.repaint_qa.len());
    for qa in &binding.repaint_qa {
        digest.tagged_text("indicator_id", &qa.indicator_id);
        digest.tagged_text("artifact_id", &qa.artifact_id);
        match &qa.acknowledgement {
            RepaintAcknowledgement::Clean => {
                digest.tagged_text("acknowledgement", "clean");
            }
            RepaintAcknowledgement::WarningAcknowledged { note } => {
                digest.tagged_text("acknowledgement", "warning_acknowledged");
                digest.tagged_text("acknowledgement_note", note);
            }
        }
    }
    digest.finish_hex()
}

fn validate_binding(binding: &RunBinding) -> Result<(), StrategyIrError> {
    check_size(
        "binding.datasets",
        binding.datasets.len(),
        MAX_DATASETS_PER_RUN,
    )?;
    if binding.datasets.is_empty() {
        return Err(out_of_range(
            "binding.datasets",
            0,
            "at least one bound dataset",
        ));
    }

    let mut seen_inputs = BTreeSet::new();
    for (index, dataset) in binding.datasets.iter().enumerate() {
        check_stable_id(
            &format!("binding.datasets[{index}].input_id"),
            &dataset.input_id,
        )?;
        check_digest_id(
            &format!("binding.datasets[{index}].dataset_id"),
            &dataset.dataset_id,
        )?;
        if !seen_inputs.insert(dataset.input_id.as_str()) {
            return Err(StrategyIrError::DuplicateId {
                kind: RefKind::Dataset,
                id: dataset.input_id.clone(),
            });
        }
    }

    check_digest_id("binding.strategy_id", &binding.strategy_id)?;
    check_digest_id("binding.config_id", &binding.config_id)?;
    if let Some(intervention_log_id) = &binding.intervention_log_id {
        check_digest_id("binding.intervention_log_id", intervention_log_id)?;
    }
    check_size(
        "binding.repaint_qa",
        binding.repaint_qa.len(),
        MAX_INDICATORS,
    )?;
    let mut seen_indicators = BTreeSet::new();
    for (index, qa) in binding.repaint_qa.iter().enumerate() {
        check_digest_id(
            &format!("binding.repaint_qa[{index}].indicator_id"),
            &qa.indicator_id,
        )?;
        check_digest_id(
            &format!("binding.repaint_qa[{index}].artifact_id"),
            &qa.artifact_id,
        )?;
        if !seen_indicators.insert(qa.indicator_id.as_str()) {
            return Err(StrategyIrError::DuplicateId {
                kind: RefKind::Indicator,
                id: qa.indicator_id.clone(),
            });
        }
        if let RepaintAcknowledgement::WarningAcknowledged { note } = &qa.acknowledgement {
            check_text(
                &format!("binding.repaint_qa[{index}].acknowledgement.note"),
                note,
                MAX_TEXT_LEN,
            )?;
        }
    }
    check_text(
        "binding.engine_version",
        &binding.engine_version,
        MAX_TEXT_LEN,
    )?;
    check_text(
        "binding.metrics_version",
        &binding.metrics_version,
        MAX_TEXT_LEN,
    )?;
    if binding.metrics_version != METRICS_SCHEMA_VERSION {
        return Err(StrategyIrError::UnsupportedMetricsVersion {
            found: binding.metrics_version.clone(),
            supported: METRICS_SCHEMA_VERSION,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests;
