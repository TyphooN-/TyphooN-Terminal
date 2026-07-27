//! Immutable, content-addressed strategy-research datasets — the ADR-135 L1
//! foundation slice for milestone M0.
//!
//! This module is deliberately narrow. It defines *what a dataset is* — an
//! ordered bar sequence plus the metadata that gives it identity — and the
//! deterministic QA pass that judges it. It does not materialise datasets from
//! the SQLite cache, does not persist them, and does not render them. Those
//! remain open M0 work; nothing here should be read as M0 being complete.
//!
//! ## Identity
//!
//! [`DatasetManifest::build`] hashes an explicitly framed, domain-separated
//! byte encoding of the metadata and every bar field (see
//! [`compute_dataset_id`]). The encoding is *not* `Debug`, `Display`, or JSON:
//! those are unstable across versions, locales, and float formatting. Every
//! element is written as a `u64` big-endian length prefix followed by its
//! bytes, so no two distinct field sequences can produce the same byte stream.
//! Floats are hashed via [`f64::to_bits`], never via a decimal rendering, and
//! identity performs no floating-point arithmetic.
//!
//! Two normalizations are applied, and only these two:
//!
//! 1. **Negative zero.** `-0.0` is hashed as `+0.0`. The two are numerically
//!    equal, so treating them as different datasets would be a false negative
//!    on every reproducibility check. See
//!    [`canonical_f64_bits`].
//! 2. **Derived fields are not hashed twice.** `bar_count`, `first_timestamp`,
//!    and `last_timestamp` are implied by the framed bar sequence.
//!    [`DatasetManifest::verify`] re-derives and compares them explicitly, so
//!    tampering with them is still detected.
//!
//! Inputs that cannot be encoded unambiguously are **rejected**, not hashed:
//! non-finite floats, empty/whitespace-padded/control-character text. Note the
//! split in responsibility — identity rejects *unencodable* input, while
//! *semantically defective* input (OHLC violations, non-positive prices,
//! disordered timestamps) is hashable and is reported by [`run_dataset_qa`].
//! A defective dataset must still have a stable id, otherwise its QA report
//! could not be attributed to anything.
//!
//! ## Calendar honesty
//!
//! [`CalendarPolicy`] is a versioned two-variant enum, and that is the whole
//! truth: there is **no exchange-holiday table, no venue-local session window,
//! and no early-close handling** here. `WeekdaysOnly` judges a bar by its UTC
//! weekday alone. Gap detection runs only when the timeframe parses to a fixed
//! step *and* the calendar can actually adjudicate the missing slots; otherwise
//! it reports why it declined rather than guessing.

use crate::broker::alpaca::Bar;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Wire-format version of [`DatasetManifest`]. Bump on any change to the
/// hashed encoding or the manifest's field set.
pub const DATASET_MANIFEST_SCHEMA_VERSION: u32 = 1;

/// Wire-format version of [`DatasetQaReport`].
pub const DATASET_QA_SCHEMA_VERSION: u32 = 1;

/// Domain-separation prefix for the dataset-id hash. Any change to the framing
/// rules must change this string *and* the manifest schema version.
const DATASET_ID_DOMAIN: &str = "typhoon.strategy_dataset.id.v1";

/// Upper bound on slots examined for a single gap. A dataset with a multi-year
/// hole must not turn QA into an unbounded loop; the finding is emitted with
/// [`DatasetQaIssue::MissingBars::scan_truncated`] set instead.
const MAX_GAP_SLOTS_SCANNED: u64 = 10_000;

// ── Policies ───────────────────────────────────────────────────────

/// Price-adjustment policy the dataset was materialised under. Recorded
/// explicitly so a run can never silently mix policies across symbols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdjustmentPolicy {
    /// As reported by the venue — no split or dividend back-adjustment.
    Raw,
    /// Split-adjusted only.
    SplitAdjusted,
    /// Split- and dividend-adjusted (total return).
    TotalReturn,
}

impl AdjustmentPolicy {
    /// Stable identifier used in the hashed encoding. Never derive this from
    /// `Debug` — the hash must not move when a variant is renamed.
    pub fn wire_id(self) -> &'static str {
        match self {
            Self::Raw => "raw",
            Self::SplitAdjusted => "split_adjusted",
            Self::TotalReturn => "total_return",
        }
    }
}

/// Dataset-side trading-calendar policy (ADR-135 §11.5).
///
/// Intentionally coarse and versioned. This is **not** an exchange calendar:
/// there is no holiday table, no half-day handling, and no venue-local session
/// window. It answers exactly one question — is a bar at this UTC instant
/// expected to exist?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalendarPolicy {
    /// Continuously traded venue (crypto). Every instant is a trading instant;
    /// weekend bars are normal and are never flagged.
    Continuous24x7,
    /// Monday–Friday UTC only. Saturday/Sunday bars are flagged as unexpected.
    ///
    /// Honest limitation: holidays and venue-local session hours are *not*
    /// modelled, so this policy under-reports (a Christmas-Day bar passes) and
    /// must not be presented as exchange-calendar validation.
    WeekdaysOnly,
}

impl CalendarPolicy {
    /// Versioned identifier recorded in the manifest and QA report, and hashed
    /// into the dataset id.
    pub fn policy_id(self) -> &'static str {
        match self {
            Self::Continuous24x7 => "continuous-24x7.v1",
            Self::WeekdaysOnly => "weekdays-only.v1",
        }
    }

    /// Whether a bar is expected at this UTC instant under the policy.
    fn expects_instant(self, utc: chrono::DateTime<chrono::Utc>) -> bool {
        use chrono::Datelike;
        match self {
            Self::Continuous24x7 => true,
            Self::WeekdaysOnly => {
                !matches!(utc.weekday(), chrono::Weekday::Sat | chrono::Weekday::Sun)
            }
        }
    }
}

// ── Errors ─────────────────────────────────────────────────────────

/// Why a text field could not be encoded unambiguously.
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
}

impl InvalidTextReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::Empty => "empty or whitespace-only",
            Self::SurroundingWhitespace => "leading or trailing whitespace",
            Self::ControlCharacter => "control character",
        }
    }
}

/// Which field of a [`Bar`] a finding or error refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BarField {
    Timestamp,
    Open,
    High,
    Low,
    Close,
    Volume,
}

impl BarField {
    fn as_str(self) -> &'static str {
        match self {
            Self::Timestamp => "timestamp",
            Self::Open => "open",
            Self::High => "high",
            Self::Low => "low",
            Self::Close => "close",
            Self::Volume => "volume",
        }
    }
}

/// Classification of a non-finite float. The offending value itself is not
/// carried: `NaN != NaN` would make errors and reports non-comparable.
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

/// Everything that can go wrong building or verifying a dataset manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DatasetError {
    /// A manifest metadata string could not be encoded unambiguously.
    InvalidMetadataField {
        field: &'static str,
        reason: InvalidTextReason,
    },
    /// A bar timestamp string could not be encoded unambiguously.
    InvalidBarTimestamp {
        index: usize,
        reason: InvalidTextReason,
    },
    /// A bar carried a non-finite price or volume. Non-finite values have no
    /// exact canonical encoding and are refused rather than hashed.
    NonFiniteBarValue {
        index: usize,
        field: BarField,
        kind: NonFiniteKind,
    },
    /// A loaded manifest was written by an incompatible schema version.
    UnsupportedSchemaVersion { found: u32, supported: u32 },
    /// A derived manifest field disagrees with the bars it was verified
    /// against — the manifest was edited, or the wrong bars were supplied.
    ManifestFieldMismatch {
        field: &'static str,
        expected: String,
        actual: String,
    },
    /// The recomputed dataset id does not match the recorded one.
    DatasetIdMismatch { expected: String, actual: String },
}

impl std::fmt::Display for DatasetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidMetadataField { field, reason } => {
                write!(
                    f,
                    "manifest field `{field}` is invalid: {}",
                    reason.as_str()
                )
            }
            Self::InvalidBarTimestamp { index, reason } => {
                write!(
                    f,
                    "bar {index} has an invalid timestamp: {}",
                    reason.as_str()
                )
            }
            Self::NonFiniteBarValue { index, field, kind } => write!(
                f,
                "bar {index} field `{}` is non-finite ({})",
                field.as_str(),
                kind.as_str()
            ),
            Self::UnsupportedSchemaVersion { found, supported } => write!(
                f,
                "dataset manifest schema version {found} is unsupported (this build supports {supported})"
            ),
            Self::ManifestFieldMismatch {
                field,
                expected,
                actual,
            } => write!(
                f,
                "manifest field `{field}` does not match the supplied bars: recorded {expected}, derived {actual}"
            ),
            Self::DatasetIdMismatch { expected, actual } => write!(
                f,
                "dataset id mismatch: recorded {expected}, recomputed {actual}"
            ),
        }
    }
}

impl std::error::Error for DatasetError {}

// ── Manifest ───────────────────────────────────────────────────────

/// Where the bars came from. Every field is identity-bearing: the same symbol
/// and timeframe pulled through a different source, venue, or merge pipeline is
/// a *different* dataset, and must not share an id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatasetProvenance {
    /// Upstream data provider (`"alpaca"`, `"kraken"`, `"merged"`, ...).
    pub source: String,
    /// Venue or feed the bars were priced on (`"IEX"`, `"XNAS"`, ...).
    pub venue: String,
    /// Merge/derivation lineage label — which pipeline produced these bars.
    pub pipeline: String,
}

/// The caller-supplied half of a manifest: everything that is *not* derived
/// from the bars themselves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetManifestInput {
    pub symbol: String,
    pub timeframe: String,
    pub provenance: DatasetProvenance,
    pub adjustment: AdjustmentPolicy,
    pub calendar: CalendarPolicy,
}

/// An immutable, content-addressed description of one bar series.
///
/// Construct with [`DatasetManifest::build`]; never edit the fields of a
/// manifest that is already recorded. A resync produces a *new* dataset id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatasetManifest {
    pub schema_version: u32,
    pub symbol: String,
    pub timeframe: String,
    pub provenance: DatasetProvenance,
    pub adjustment: AdjustmentPolicy,
    pub calendar: CalendarPolicy,
    /// Versioned id of `calendar`, denormalised so a stored manifest stays
    /// readable when the enum grows variants.
    pub calendar_policy_id: String,
    /// Number of bars, in the order supplied.
    pub bar_count: u64,
    /// Timestamp of the **first bar in the supplied order** — not the minimum.
    /// A disordered dataset is a QA defect, not a re-sorting trigger.
    pub first_timestamp: Option<String>,
    /// Timestamp of the **last bar in the supplied order** — not the maximum.
    pub last_timestamp: Option<String>,
    /// Lowercase hex SHA-256 over the canonical encoding of metadata + bars.
    pub dataset_id: String,
}

impl DatasetManifest {
    /// Build a manifest for `bars`, computing the content-addressed id.
    ///
    /// Bar order is significant: reordering the same bars yields a different
    /// dataset. Returns an error if any metadata string or bar value cannot be
    /// canonically encoded; semantic defects are *not* rejected here — run
    /// [`DatasetManifest::run_qa`] for those.
    pub fn build(input: &DatasetManifestInput, bars: &[Bar]) -> Result<Self, DatasetError> {
        let dataset_id = compute_dataset_id(input, bars)?;
        Ok(Self {
            schema_version: DATASET_MANIFEST_SCHEMA_VERSION,
            symbol: input.symbol.clone(),
            timeframe: input.timeframe.clone(),
            provenance: input.provenance.clone(),
            adjustment: input.adjustment,
            calendar: input.calendar,
            calendar_policy_id: input.calendar.policy_id().to_string(),
            bar_count: bars.len() as u64,
            first_timestamp: bars.first().map(|b| b.timestamp.clone()),
            last_timestamp: bars.last().map(|b| b.timestamp.clone()),
            dataset_id,
        })
    }

    /// The caller-supplied half of this manifest, for rebuilding it.
    pub fn to_input(&self) -> DatasetManifestInput {
        DatasetManifestInput {
            symbol: self.symbol.clone(),
            timeframe: self.timeframe.clone(),
            provenance: self.provenance.clone(),
            adjustment: self.adjustment,
            calendar: self.calendar,
        }
    }

    /// Recompute this manifest's dataset id from `bars` without comparing it.
    pub fn recompute_dataset_id(&self, bars: &[Bar]) -> Result<String, DatasetError> {
        self.check_schema_version()?;
        compute_dataset_id(&self.to_input(), bars)
    }

    /// Prove that `bars` are the bars this manifest was built from.
    ///
    /// Checks the schema version, then every derived field, then the id. A
    /// loaded manifest that verifies is byte-equivalent to the one that was
    /// stored, over exactly the data it claims to describe.
    pub fn verify(&self, bars: &[Bar]) -> Result<(), DatasetError> {
        self.check_schema_version()?;
        let rebuilt = Self::build(&self.to_input(), bars)?;

        expect_field("bar_count", self.bar_count, rebuilt.bar_count)?;
        expect_optional_field(
            "first_timestamp",
            self.first_timestamp.as_deref(),
            rebuilt.first_timestamp.as_deref(),
        )?;
        expect_optional_field(
            "last_timestamp",
            self.last_timestamp.as_deref(),
            rebuilt.last_timestamp.as_deref(),
        )?;
        expect_field(
            "calendar_policy_id",
            &self.calendar_policy_id,
            &rebuilt.calendar_policy_id,
        )?;

        if self.dataset_id != rebuilt.dataset_id {
            return Err(DatasetError::DatasetIdMismatch {
                expected: self.dataset_id.clone(),
                actual: rebuilt.dataset_id,
            });
        }
        Ok(())
    }

    /// Run the deterministic QA pass using the timeframe and calendar policy
    /// this manifest recorded — never an ambient default.
    pub fn run_qa(&self, bars: &[Bar]) -> DatasetQaReport {
        run_dataset_qa(&self.timeframe, self.calendar, bars)
    }

    fn check_schema_version(&self) -> Result<(), DatasetError> {
        if self.schema_version == DATASET_MANIFEST_SCHEMA_VERSION {
            Ok(())
        } else {
            Err(DatasetError::UnsupportedSchemaVersion {
                found: self.schema_version,
                supported: DATASET_MANIFEST_SCHEMA_VERSION,
            })
        }
    }
}

fn expect_field<T: PartialEq + std::fmt::Display>(
    field: &'static str,
    recorded: T,
    derived: T,
) -> Result<(), DatasetError> {
    if recorded == derived {
        Ok(())
    } else {
        Err(DatasetError::ManifestFieldMismatch {
            field,
            expected: recorded.to_string(),
            actual: derived.to_string(),
        })
    }
}

fn expect_optional_field(
    field: &'static str,
    recorded: Option<&str>,
    derived: Option<&str>,
) -> Result<(), DatasetError> {
    if recorded == derived {
        Ok(())
    } else {
        Err(DatasetError::ManifestFieldMismatch {
            field,
            expected: recorded.unwrap_or("<none>").to_string(),
            actual: derived.unwrap_or("<none>").to_string(),
        })
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

    /// Hash a finite `f64` by its canonical bits. Non-finite values must have
    /// been rejected upstream.
    fn tagged_f64(&mut self, tag: &str, value: f64) {
        self.frame(tag.as_bytes());
        self.hasher.update(canonical_f64_bits(value).to_be_bytes());
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

/// Reject text that has no unambiguous canonical encoding.
fn validate_text(value: &str) -> Result<(), InvalidTextReason> {
    if value.trim().is_empty() {
        return Err(InvalidTextReason::Empty);
    }
    if value.trim() != value {
        return Err(InvalidTextReason::SurroundingWhitespace);
    }
    if value.chars().any(char::is_control) {
        return Err(InvalidTextReason::ControlCharacter);
    }
    Ok(())
}

fn validate_metadata_field(field: &'static str, value: &str) -> Result<(), DatasetError> {
    validate_text(value).map_err(|reason| DatasetError::InvalidMetadataField { field, reason })
}

/// The content-addressed dataset id: lowercase hex SHA-256 over the canonical
/// encoding of `input` and every field of every bar, in order.
///
/// Field tags and lengths are part of the hashed stream; the bar count is
/// written before the bars so a truncated sequence cannot be re-framed as a
/// shorter dataset.
pub fn compute_dataset_id(
    input: &DatasetManifestInput,
    bars: &[Bar],
) -> Result<String, DatasetError> {
    validate_metadata_field("symbol", &input.symbol)?;
    validate_metadata_field("timeframe", &input.timeframe)?;
    validate_metadata_field("provenance.source", &input.provenance.source)?;
    validate_metadata_field("provenance.venue", &input.provenance.venue)?;
    validate_metadata_field("provenance.pipeline", &input.provenance.pipeline)?;

    let mut digest = CanonicalDigest::new(DATASET_ID_DOMAIN);
    digest.tagged_u64("schema_version", DATASET_MANIFEST_SCHEMA_VERSION as u64);
    digest.tagged_text("symbol", &input.symbol);
    digest.tagged_text("timeframe", &input.timeframe);
    digest.tagged_text("source", &input.provenance.source);
    digest.tagged_text("venue", &input.provenance.venue);
    digest.tagged_text("pipeline", &input.provenance.pipeline);
    digest.tagged_text("adjustment", input.adjustment.wire_id());
    digest.tagged_text("calendar", input.calendar.policy_id());
    digest.tagged_u64("bar_count", bars.len() as u64);

    for (index, bar) in bars.iter().enumerate() {
        validate_text(&bar.timestamp)
            .map_err(|reason| DatasetError::InvalidBarTimestamp { index, reason })?;
        for (field, value) in [
            (BarField::Open, bar.open),
            (BarField::High, bar.high),
            (BarField::Low, bar.low),
            (BarField::Close, bar.close),
            (BarField::Volume, bar.volume),
        ] {
            if let Some(kind) = NonFiniteKind::classify(value) {
                return Err(DatasetError::NonFiniteBarValue { index, field, kind });
            }
        }

        digest.tagged_u64("bar", index as u64);
        digest.tagged_text("ts", &bar.timestamp);
        digest.tagged_f64("o", bar.open);
        digest.tagged_f64("h", bar.high);
        digest.tagged_f64("l", bar.low);
        digest.tagged_f64("c", bar.close);
        digest.tagged_f64("v", bar.volume);
    }

    Ok(digest.finish_hex())
}

// ── QA report ──────────────────────────────────────────────────────

/// How much a finding should weigh. `Error` means the dataset is defective;
/// `Warning` means it is suspicious under the recorded policy and needs a
/// human decision; `Info` is context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatasetQaSeverity {
    Info,
    Warning,
    Error,
}

/// Which OHLC invariant a bar broke. The invariant is
/// `low <= min(open, close) <= max(open, close) <= high`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OhlcViolationKind {
    HighBelowLow,
    HighBelowOpen,
    HighBelowClose,
    LowAboveOpen,
    LowAboveClose,
}

/// A typed, located dataset defect.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatasetQaIssue {
    /// The dataset has no bars at all.
    EmptyDataset,
    /// The timestamp is not RFC 3339, so ordering, weekend, and gap checks
    /// cannot run for this bar.
    UnparsableTimestamp { raw: String },
    /// This bar repeats the preceding bar's instant.
    DuplicateTimestamp {
        previous_index: usize,
        previous_timestamp: String,
    },
    /// This bar precedes the bar before it.
    TimestampOutOfOrder {
        previous_index: usize,
        previous_timestamp: String,
    },
    /// A price or volume is `NaN` or infinite.
    NonFiniteValue {
        field: BarField,
        kind: NonFiniteKind,
    },
    /// A price is zero or negative.
    NonPositivePrice { field: BarField, value: f64 },
    /// Volume is negative. `-0.0` is *not* negative and is not reported.
    NegativeVolume { value: f64 },
    /// The bar breaks an OHLC invariant.
    OhlcViolation { kind: OhlcViolationKind },
    /// A bar exists on a UTC Saturday or Sunday under a weekday-only calendar.
    UnexpectedWeekendBar { weekday: String },
    /// Expected slots are absent between the preceding bar and this one.
    /// Counts only slots the calendar policy says should exist.
    MissingBars {
        /// First absent slot the calendar expected, RFC 3339 UTC.
        expected_next: String,
        missing_slots: u64,
        /// The gap exceeded [`MAX_GAP_SLOTS_SCANNED`]; `missing_slots` is a
        /// floor, not the true count.
        scan_truncated: bool,
    },
}

impl DatasetQaIssue {
    /// Fixed severity per issue kind — never inferred from magnitude.
    pub fn severity(&self) -> DatasetQaSeverity {
        match self {
            Self::EmptyDataset
            | Self::UnparsableTimestamp { .. }
            | Self::DuplicateTimestamp { .. }
            | Self::TimestampOutOfOrder { .. }
            | Self::NonFiniteValue { .. }
            | Self::NonPositivePrice { .. }
            | Self::NegativeVolume { .. }
            | Self::OhlcViolation { .. } => DatasetQaSeverity::Error,
            Self::UnexpectedWeekendBar { .. } | Self::MissingBars { .. } => {
                DatasetQaSeverity::Warning
            }
        }
    }
}

/// One issue, located at the bar that produced it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DatasetQaFinding {
    /// Index into the supplied bar slice; `None` for dataset-wide findings.
    pub bar_index: Option<usize>,
    /// The offending bar's raw timestamp, verbatim.
    pub timestamp: Option<String>,
    pub severity: DatasetQaSeverity,
    pub issue: DatasetQaIssue,
}

/// Whether gap detection ran, and if not, why it declined.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GapDetectionStatus {
    /// Ran with this fixed step.
    Enabled { step_seconds: u64 },
    /// The timeframe label could not be parsed — no step was guessed.
    UnsupportedTimeframe { timeframe: String },
    /// The timeframe parses but has no fixed length (months vary), so a
    /// missing-slot count would be fiction.
    VariableLengthTimeframe { timeframe: String },
    /// Intraday bars under a calendar with no session table: every overnight
    /// close would be reported as a gap, so the check is declined instead.
    UnsupportedForCalendar {
        timeframe: String,
        calendar_policy_id: String,
    },
}

impl GapDetectionStatus {
    fn resolve(timeframe: &str, calendar: CalendarPolicy) -> Self {
        match parse_timeframe(timeframe) {
            None => Self::UnsupportedTimeframe {
                timeframe: timeframe.to_string(),
            },
            Some(TimeframeStep::Variable) => Self::VariableLengthTimeframe {
                timeframe: timeframe.to_string(),
            },
            Some(TimeframeStep::Fixed { seconds }) => {
                // A weekday-only calendar can adjudicate a missing *day*, but
                // not a missing 15-minute slot — that needs session hours we
                // do not model.
                if calendar == CalendarPolicy::WeekdaysOnly && seconds < SECONDS_PER_DAY {
                    Self::UnsupportedForCalendar {
                        timeframe: timeframe.to_string(),
                        calendar_policy_id: calendar.policy_id().to_string(),
                    }
                } else {
                    Self::Enabled {
                        step_seconds: seconds,
                    }
                }
            }
        }
    }
}

/// The deterministic result of a QA pass. Recomputing it over the same bars
/// and policy always yields an identical report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DatasetQaReport {
    pub schema_version: u32,
    pub timeframe: String,
    pub calendar: CalendarPolicy,
    pub calendar_policy_id: String,
    pub gap_detection: GapDetectionStatus,
    pub bars_checked: u64,
    /// Findings ordered by bar index; dataset-wide findings sort first.
    /// Findings for one bar keep a fixed check order.
    pub findings: Vec<DatasetQaFinding>,
}

impl DatasetQaReport {
    pub fn has_errors(&self) -> bool {
        self.error_count() > 0
    }

    pub fn error_count(&self) -> usize {
        self.count_at(DatasetQaSeverity::Error)
    }

    pub fn warning_count(&self) -> usize {
        self.count_at(DatasetQaSeverity::Warning)
    }

    fn count_at(&self, severity: DatasetQaSeverity) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == severity)
            .count()
    }
}

// ── Timeframe parsing ──────────────────────────────────────────────

const SECONDS_PER_DAY: u64 = 86_400;

/// A timeframe's step, when it has one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TimeframeStep {
    Fixed {
        seconds: u64,
    },
    /// Calendar-relative and not a fixed number of seconds (months).
    Variable,
}

/// Parse a `<count><unit>` timeframe label exactly as the cache writes it —
/// `1Min`, `15Min`, `1Hour`, `4Hour`, `1Day`, `1Week`, `1Month`.
///
/// Deliberately strict: the suffix is case-sensitive and unknown labels return
/// `None` rather than a guessed step.
fn parse_timeframe(timeframe: &str) -> Option<TimeframeStep> {
    let digits = timeframe
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(timeframe.len());
    if digits == 0 {
        return None;
    }
    let count: u64 = timeframe[..digits].parse().ok()?;
    if count == 0 {
        return None;
    }

    let unit_seconds = match &timeframe[digits..] {
        "Min" => 60,
        "Hour" => 3_600,
        "Day" => SECONDS_PER_DAY,
        "Week" => 7 * SECONDS_PER_DAY,
        "Month" => return Some(TimeframeStep::Variable),
        _ => return None,
    };

    let seconds = count.checked_mul(unit_seconds)?;
    // Keep the step inside i64 so instant arithmetic below cannot overflow.
    if seconds > i64::MAX as u64 {
        return None;
    }
    Some(TimeframeStep::Fixed { seconds })
}

// ── QA pass ────────────────────────────────────────────────────────

fn parse_utc(timestamp: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(timestamp)
        .ok()
        .map(|dt| dt.with_timezone(&chrono::Utc))
}

fn format_utc(epoch_seconds: i64) -> Option<String> {
    chrono::DateTime::from_timestamp(epoch_seconds, 0)
        .map(|dt| dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
}

/// Run the deterministic dataset QA pass.
///
/// Every check is located (bar index + raw timestamp) and typed. Ordering,
/// weekend, and gap checks compare each bar against the **immediately
/// preceding parsable bar** — the pass never re-sorts, because disorder is
/// itself the defect being reported.
///
/// Scope, stated honestly: this covers structure (emptiness, ordering,
/// duplicates, OHLC invariants, impossible prices and volumes), the weekend
/// diagnostic under `calendar`, and calendar-aware gaps when the timeframe
/// permits. Spike/outlier detection, carry-bar detection, and split-like level
/// shifts are **not** implemented here.
pub fn run_dataset_qa(timeframe: &str, calendar: CalendarPolicy, bars: &[Bar]) -> DatasetQaReport {
    let gap_detection = GapDetectionStatus::resolve(timeframe, calendar);
    let mut findings: Vec<DatasetQaFinding> = Vec::new();

    if bars.is_empty() {
        findings.push(DatasetQaFinding {
            bar_index: None,
            timestamp: None,
            severity: DatasetQaIssue::EmptyDataset.severity(),
            issue: DatasetQaIssue::EmptyDataset,
        });
    }

    // Instant per bar, `None` when unparsable — reused by the gap pass.
    let mut instants: Vec<Option<chrono::DateTime<chrono::Utc>>> = Vec::with_capacity(bars.len());
    let mut previous: Option<(usize, chrono::DateTime<chrono::Utc>, String)> = None;

    for (index, bar) in bars.iter().enumerate() {
        let mut push = |issue: DatasetQaIssue| {
            findings.push(DatasetQaFinding {
                bar_index: Some(index),
                timestamp: Some(bar.timestamp.clone()),
                severity: issue.severity(),
                issue,
            });
        };

        let instant = parse_utc(&bar.timestamp);
        match instant {
            None => push(DatasetQaIssue::UnparsableTimestamp {
                raw: bar.timestamp.clone(),
            }),
            Some(current) => {
                if let Some((previous_index, previous_instant, previous_timestamp)) = &previous {
                    if current == *previous_instant {
                        push(DatasetQaIssue::DuplicateTimestamp {
                            previous_index: *previous_index,
                            previous_timestamp: previous_timestamp.clone(),
                        });
                    } else if current < *previous_instant {
                        push(DatasetQaIssue::TimestampOutOfOrder {
                            previous_index: *previous_index,
                            previous_timestamp: previous_timestamp.clone(),
                        });
                    }
                }
            }
        }

        check_bar_values(bar, &mut push);

        if let Some(current) = instant {
            if !calendar.expects_instant(current) {
                use chrono::Datelike;
                push(DatasetQaIssue::UnexpectedWeekendBar {
                    weekday: current.weekday().to_string(),
                });
            }
            previous = Some((index, current, bar.timestamp.clone()));
        }
        instants.push(instant);
    }

    if let GapDetectionStatus::Enabled { step_seconds } = gap_detection {
        collect_gap_findings(bars, &instants, step_seconds, calendar, &mut findings);
    }

    // Stable sort: dataset-wide findings first, then bar order, with each
    // bar's checks in their fixed emission order.
    findings.sort_by_key(|finding| finding.bar_index);

    DatasetQaReport {
        schema_version: DATASET_QA_SCHEMA_VERSION,
        timeframe: timeframe.to_string(),
        calendar,
        calendar_policy_id: calendar.policy_id().to_string(),
        gap_detection,
        bars_checked: bars.len() as u64,
        findings,
    }
}

/// Price/volume checks for one bar, in fixed order: finiteness and positivity
/// per price field, then the OHLC invariants, then volume.
fn check_bar_values(bar: &Bar, push: &mut impl FnMut(DatasetQaIssue)) {
    let prices = [
        (BarField::Open, bar.open),
        (BarField::High, bar.high),
        (BarField::Low, bar.low),
        (BarField::Close, bar.close),
    ];

    for (field, value) in prices {
        if let Some(kind) = NonFiniteKind::classify(value) {
            push(DatasetQaIssue::NonFiniteValue { field, kind });
        } else if value <= 0.0 {
            push(DatasetQaIssue::NonPositivePrice { field, value });
        }
    }

    // Comparisons against NaN are always false, which would silently suppress
    // these checks — skip them explicitly instead, and let the non-finite
    // findings above carry the defect.
    if prices.iter().all(|(_, value)| value.is_finite()) {
        for (broken, kind) in [
            (bar.high < bar.low, OhlcViolationKind::HighBelowLow),
            (bar.high < bar.open, OhlcViolationKind::HighBelowOpen),
            (bar.high < bar.close, OhlcViolationKind::HighBelowClose),
            (bar.low > bar.open, OhlcViolationKind::LowAboveOpen),
            (bar.low > bar.close, OhlcViolationKind::LowAboveClose),
        ] {
            if broken {
                push(DatasetQaIssue::OhlcViolation { kind });
            }
        }
    }

    if let Some(kind) = NonFiniteKind::classify(bar.volume) {
        push(DatasetQaIssue::NonFiniteValue {
            field: BarField::Volume,
            kind,
        });
    } else if bar.volume < 0.0 {
        push(DatasetQaIssue::NegativeVolume { value: bar.volume });
    }
}

/// Report absent slots between consecutive parsable, strictly increasing bars.
///
/// Counts only slots the calendar expects, so a weekday-only daily series is
/// not accused of missing every Saturday. Bounded by
/// [`MAX_GAP_SLOTS_SCANNED`].
fn collect_gap_findings(
    bars: &[Bar],
    instants: &[Option<chrono::DateTime<chrono::Utc>>],
    step_seconds: u64,
    calendar: CalendarPolicy,
    findings: &mut Vec<DatasetQaFinding>,
) {
    let step = step_seconds as i64;
    let mut previous: Option<chrono::DateTime<chrono::Utc>> = None;

    for (index, instant) in instants.iter().enumerate() {
        let Some(current) = *instant else { continue };
        let Some(start) = previous.replace(current) else {
            continue;
        };
        // Disorder and duplicates are reported by the main pass; do not also
        // read them as gaps.
        if current <= start {
            continue;
        }

        let mut missing_slots = 0u64;
        let mut scanned = 0u64;
        let mut scan_truncated = false;
        let mut expected_next: Option<String> = None;
        let mut slot = start.timestamp();

        loop {
            let Some(next) = slot.checked_add(step) else {
                break;
            };
            slot = next;
            if slot >= current.timestamp() {
                break;
            }
            scanned += 1;
            if scanned > MAX_GAP_SLOTS_SCANNED {
                scan_truncated = true;
                break;
            }
            let Some(slot_instant) = chrono::DateTime::from_timestamp(slot, 0) else {
                break;
            };
            if calendar.expects_instant(slot_instant) {
                missing_slots += 1;
                if expected_next.is_none() {
                    expected_next = format_utc(slot);
                }
            }
        }

        if missing_slots == 0 {
            continue;
        }
        let Some(expected_next) = expected_next else {
            continue;
        };
        let issue = DatasetQaIssue::MissingBars {
            expected_next,
            missing_slots,
            scan_truncated,
        };
        findings.push(DatasetQaFinding {
            bar_index: Some(index),
            timestamp: bars.get(index).map(|bar| bar.timestamp.clone()),
            severity: issue.severity(),
            issue,
        });
    }
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
