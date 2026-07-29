//! Deterministic, bounded reference-data artifacts for strategy execution
//! (ADR-135 §6.7, §6.8, milestone M2).
//!
//! Fetching lives outside this module on purpose. Workers persist raw provider
//! responses first; this boundary accepts an explicitly bounded snapshot of
//! those persisted records and seals an executable calendar-exception or
//! corporate-action artifact from them.
//!
//! The production inputs are named for what they are. The Yahoo chart endpoint
//! is a keyless public feed, the research database is a cache of some identified
//! upstream, and the built-in NYSE/xStocks calendars are derived rules. **None
//! of those is authoritative**, and a request with `require_authoritative`
//! refuses them rather than dressing a guess as an exchange statement. Exchange
//! publications and contracted vendors are the two classes that clear that bar.
//!
//! Artifact identity covers source class/system, authority, the completeness
//! claim, the covered range, the as-of instant, venue/symbol/time-zone/currency/
//! adjustment policy, every source-record id with its raw SHA-256, and the
//! canonical executable events. Retrieval time is audit metadata and is
//! deliberately *excluded*: re-downloading identical as-of bytes tomorrow is the
//! same semantic artifact and must not produce a different run id.

use crate::core::strategy_calendar::{
    CalendarError, CalendarException, CalendarExceptionKind, ExchangeTimeZone, LocalSessionWindow,
    SessionRule, TradingCalendar, TradingCalendarSpec, shorten_windows,
};
use crate::core::strategy_corporate::{
    CorporateAction, CorporateActionKind, CorporateActionSchedule,
};
use crate::core::strategy_dataset::AdjustmentPolicy;
use crate::core::strategy_financing::FinancingModel;
use crate::core::strategy_instrument::{InstrumentRegistry, InstrumentSpec};
use crate::core::strategy_ir::{ExecutionSettings, ReferenceDataBindings};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Wire-format version of both artifacts. Bumping it invalidates every sealed
/// artifact id, which is the point.
pub const REFERENCE_DATA_SCHEMA_VERSION: u32 = 1;
/// Source records one artifact may carry.
pub const MAX_REFERENCE_RECORDS: usize = 4_096;
/// Longest raw provider record one source record may preserve.
pub const MAX_RAW_SOURCE_BYTES: usize = 256 * 1024;
/// Hard ceiling on an encoded artifact, applied before parsing so a hostile or
/// corrupt file is never decoded into memory first and rejected second.
pub const MAX_REFERENCE_ARTIFACT_BYTES: usize = 8 * 1024 * 1024;
const MAX_TEXT_BYTES: usize = 256;

// ── Source identity and authority ──────────────────────────────────

/// How much weight a run may put on a source. This is a claim the caller makes,
/// and [`validate_source`] checks it against the source system itself, so a
/// keyless feed cannot be labelled official.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceAuthorityClass {
    /// The exchange published it.
    ExchangeOfficial,
    /// A vendor under contract published it, with a support path behind it.
    ContractedVendor,
    /// A public endpoint with no contract, no SLA and no correction feed.
    UnverifiedPublic,
    /// Computed from rules this codebase wrote down. An assumption.
    DerivedRule,
    /// The intended source could not be reached at all.
    Unavailable,
}

impl SourceAuthorityClass {
    pub const fn wire_id(self) -> &'static str {
        match self {
            Self::ExchangeOfficial => "exchange_official",
            Self::ContractedVendor => "contracted_vendor",
            Self::UnverifiedPublic => "unverified_public",
            Self::DerivedRule => "derived_rule",
            Self::Unavailable => "unavailable",
        }
    }

    /// Whether this class may back a run that demands authoritative reference
    /// data. Rule-derived and keyless-public never do.
    pub const fn is_authoritative(self) -> bool {
        matches!(self, Self::ExchangeOfficial | Self::ContractedVendor)
    }
}

/// The concrete system a batch came from. The authority class is *derived* from
/// this, never taken on trust.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "system", rename_all = "snake_case", deny_unknown_fields)]
pub enum SourceSystem {
    ExchangePublication {
        exchange: String,
    },
    ContractedVendor {
        vendor: String,
        product: String,
    },
    /// `query1.finance.yahoo.com/v8/finance/chart` — the keyless feed the split
    /// and dividend scrapers already use. Explicitly not authoritative.
    YahooChartKeyless,
    /// The built-in [`TradingCalendarSpec`] rules. An assumption, not a source.
    RuleDerived {
        ruleset: String,
    },
    /// The local research database. A cache inherits the authority of whatever
    /// it cached and nothing more.
    ResearchDatabaseCache {
        upstream: Box<SourceSystem>,
    },
    Unavailable {
        intended_source: String,
    },
}

/// Which timestamps identity is allowed to depend on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityMetadataPolicy {
    /// As-of is semantic and hashed; retrieval time is audit-only and is not.
    AsOfIncludedRetrievalExcluded,
}

/// What the batch actually covers — the answer to "is this range complete?".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SourceCoverage {
    /// Exchange-local civil dates, inclusive of both ends. Calendars are stated
    /// in exchange dates, never in UTC instants.
    ExchangeDateRange {
        start: chrono::NaiveDate,
        end_inclusive: chrono::NaiveDate,
    },
    /// Half-open `[start_ns, end_ns)` UTC nanoseconds.
    UtcRange { start_ns: i64, end_ns: i64 },
}

/// One retrieval of one source, with every claim it makes stated explicitly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceBatch {
    pub source: SourceSystem,
    pub authority: SourceAuthorityClass,
    pub coverage: SourceCoverage,
    /// Whether the batch is the *whole* covered range. A partial batch is
    /// refused rather than sealed as if the missing days had no events.
    pub complete: bool,
    /// The instant the source's contents are current as of. Semantic; hashed.
    pub as_of_ns: i64,
    /// When this process fetched it. Audit metadata; deliberately not hashed.
    pub retrieved_at_ns: i64,
    pub identity_metadata_policy: IdentityMetadataPolicy,
}

// ── Source records ─────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CalendarExceptionSourceKind {
    /// The venue published a full closure.
    Closed,
    /// The venue published an early close at this exchange-local minute. The
    /// resulting session is the base calendar's own windows truncated there —
    /// no opening bell is assumed on the venue's behalf.
    EarlyClose { close_minute: u32 },
    /// The venue published windows that replace the rule outright, including
    /// opening a date the rule calls closed.
    OpenOverride { windows: Vec<LocalSessionWindow> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalendarSourceRecord {
    pub source_record_id: String,
    /// SHA-256 of `raw_source`, checked on every materialization and verify, so
    /// a hand-edited raw record cannot ride along under a sealed id.
    pub raw_record_sha256: String,
    pub venue: String,
    pub time_zone: ExchangeTimeZone,
    pub local_date: chrono::NaiveDate,
    pub kind: CalendarExceptionSourceKind,
    pub label: String,
    /// The provider's own bytes, preserved verbatim.
    pub raw_source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalendarMaterializationRequest {
    pub venue: String,
    pub time_zone: ExchangeTimeZone,
    pub range_start: chrono::NaiveDate,
    pub range_end_inclusive: chrono::NaiveDate,
    /// Refuse anything below exchange/vendor authority. A run that cares about
    /// venue closures sets this.
    pub require_authoritative: bool,
    pub source: SourceBatch,
    /// The rule-only base. It stays an assumption even when the exceptions are
    /// exchange-published: authority attaches to the covered exception set, not
    /// to the weekday/session rule underneath it.
    pub base: TradingCalendarSpec,
    pub records: Vec<CalendarSourceRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CorporateActionSourceKind {
    /// Exact decimal integer strings exactly as the provider delivered them.
    /// Strings, not floats, so `2` and `2.0` cannot both mean the same split.
    Split {
        numerator: String,
        denominator: String,
    },
    /// Positive base-currency decimal with at most eight fractional digits, in
    /// one canonical spelling.
    CashDividend {
        amount_per_unit: String,
    },
    SymbolChange {
        new_symbol: String,
    },
    Delisting,
    /// A record class this build cannot model. Named rather than dropped, and
    /// refused rather than approximated.
    Unsupported {
        action_type: String,
    },
}

impl CorporateActionSourceKind {
    /// Ordering rank matching [`CorporateActionKind::order_rank`], so source
    /// records sort into the same canonical order the sealed schedule uses and
    /// the two lists stay one order rather than two independent sorts.
    /// Mirrored rather than delegated because the schedule's ranks belong to
    /// constructed actions and an unsupported record has no action to construct.
    /// `source_ranks_match_the_schedule` pins the two tables together.
    const fn order_rank(&self) -> u8 {
        match self {
            Self::Split { .. } => 0,
            Self::CashDividend { .. } => 1,
            Self::SymbolChange { .. } => 2,
            Self::Delisting => 3,
            // Sorts last, and is refused before it can reach a schedule.
            Self::Unsupported { .. } => u8::MAX,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorporateActionSourceRecord {
    pub source_record_id: String,
    pub raw_record_sha256: String,
    pub venue: String,
    pub symbol: String,
    pub time_zone: ExchangeTimeZone,
    pub currency: String,
    /// RFC-3339 with an explicit `Z`. An offsetless or local-time stamp is
    /// ambiguous across a DST boundary and is refused rather than guessed.
    pub effective_utc: String,
    pub kind: CorporateActionSourceKind,
    pub raw_source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorporateActionMaterializationRequest {
    pub venue: String,
    pub symbol: String,
    pub time_zone: ExchangeTimeZone,
    pub currency: String,
    pub range_start_ns: i64,
    pub range_end_ns: i64,
    pub require_authoritative: bool,
    /// The dataset's adjustment policy. Events already baked into those prices
    /// are refused here rather than applied a second time in the simulator.
    pub adjustment: AdjustmentPolicy,
    pub source: SourceBatch,
    pub records: Vec<CorporateActionSourceRecord>,
}

// ── Errors ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceDataError {
    /// The intended source could not be reached. Never silently substituted.
    SourceUnavailable,
    /// The declared authority class does not match the source system.
    DishonestAuthority,
    /// The source cleared its own bar but not the run's `require_authoritative`.
    NotAuthoritative,
    /// The batch does not claim to be complete, or does not cover the request.
    IncompleteRange,
    /// The requested range is empty or inverted.
    InvalidRange,
    /// As-of is later than retrieval: the batch describes a future it cannot
    /// have seen.
    AsOfAfterRetrieval,
    UnsupportedIdentityPolicy,
    TooManyRecords {
        found: usize,
    },
    InvalidText {
        field: &'static str,
    },
    OversizeRawSource {
        source_record_id: String,
    },
    /// `raw_record_sha256` does not hash `raw_source`.
    RawRecordIdentityMismatch {
        source_record_id: String,
    },
    /// A record belongs to a different venue, symbol, currency or time zone.
    ScopeMismatch {
        source_record_id: String,
    },
    /// A timestamp carries no unambiguous UTC offset.
    TimezoneAmbiguous {
        value: String,
    },
    MalformedRecord {
        source_record_id: String,
    },
    DuplicateSourceRecord {
        source_record_id: String,
    },
    /// Two records state the same event at the same instant or exchange date.
    DuplicateEvent {
        source_record_id: String,
    },
    /// Two records state *different* events of one class at one instant or
    /// exchange date.
    ConflictingEvent {
        source_record_id: String,
    },
    UnsupportedActionType {
        action_type: String,
    },
    AdjustedPriceDoubleCounting {
        adjustment: AdjustmentPolicy,
    },
    ArtifactIdentityMismatch,
    /// Bytes that parse but are not the one canonical encoding.
    NonCanonicalArtifact,
    ArtifactTooLarge,
    Io(String),
    Decode(String),
    Calendar(String),
    Corporate(String),
    Config(String),
}

impl std::fmt::Display for ReferenceDataError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SourceUnavailable => formatter.write_str(
                "the intended reference-data source was unavailable; no substitute was used",
            ),
            Self::DishonestAuthority => {
                formatter.write_str("the declared authority class does not match the source system")
            }
            Self::NotAuthoritative => formatter.write_str(
                "this run requires exchange-official or contracted-vendor reference data",
            ),
            Self::IncompleteRange => formatter
                .write_str("the source batch does not completely cover the requested range"),
            Self::InvalidRange => formatter.write_str("the requested range is empty or inverted"),
            Self::AsOfAfterRetrieval => {
                formatter.write_str("the batch is as-of an instant later than its retrieval")
            }
            Self::UnsupportedIdentityPolicy => {
                formatter.write_str("unsupported identity-metadata policy")
            }
            Self::TooManyRecords { found } => write!(
                formatter,
                "{found} source records exceeds the limit of {MAX_REFERENCE_RECORDS}"
            ),
            Self::InvalidText { field } => write!(
                formatter,
                "`{field}` is empty, untrimmed, unprintable or over {MAX_TEXT_BYTES} bytes"
            ),
            Self::OversizeRawSource { source_record_id } => write!(
                formatter,
                "raw source for `{source_record_id}` is empty or over {MAX_RAW_SOURCE_BYTES} bytes"
            ),
            Self::RawRecordIdentityMismatch { source_record_id } => write!(
                formatter,
                "raw source for `{source_record_id}` does not hash to its recorded digest"
            ),
            Self::ScopeMismatch { source_record_id } => write!(
                formatter,
                "`{source_record_id}` belongs to a different venue, symbol, currency or time zone"
            ),
            Self::TimezoneAmbiguous { value } => write!(
                formatter,
                "`{value}` has no unambiguous UTC offset; an exact `Z` timestamp is required"
            ),
            Self::MalformedRecord { source_record_id } => write!(
                formatter,
                "`{source_record_id}` is malformed or outside the requested range"
            ),
            Self::DuplicateSourceRecord { source_record_id } => write!(
                formatter,
                "source record id `{source_record_id}` appears twice"
            ),
            Self::DuplicateEvent { source_record_id } => write!(
                formatter,
                "`{source_record_id}` repeats an event already stated for that instant or exchange date"
            ),
            Self::ConflictingEvent { source_record_id } => write!(
                formatter,
                "`{source_record_id}` states a different event from another record for the same instant or exchange date"
            ),
            Self::UnsupportedActionType { action_type } => write!(
                formatter,
                "corporate action type `{action_type}` is not modelled by this build"
            ),
            Self::AdjustedPriceDoubleCounting { adjustment } => write!(
                formatter,
                "these actions are already baked into `{}` prices",
                adjustment.wire_id()
            ),
            Self::ArtifactIdentityMismatch => {
                formatter.write_str("the artifact does not hash to its sealed id")
            }
            Self::NonCanonicalArtifact => {
                formatter.write_str("the bytes are not the canonical encoding of this artifact")
            }
            Self::ArtifactTooLarge => write!(
                formatter,
                "the artifact exceeds the {MAX_REFERENCE_ARTIFACT_BYTES}-byte bound"
            ),
            Self::Io(detail) => write!(formatter, "reference artifact store: {detail}"),
            Self::Decode(detail) => write!(formatter, "reference artifact decode: {detail}"),
            Self::Calendar(detail) => write!(formatter, "calendar: {detail}"),
            Self::Corporate(detail) => write!(formatter, "corporate actions: {detail}"),
            Self::Config(detail) => write!(formatter, "execution config: {detail}"),
        }
    }
}

impl std::error::Error for ReferenceDataError {}

impl From<CalendarError> for ReferenceDataError {
    fn from(error: CalendarError) -> Self {
        Self::Calendar(error.to_string())
    }
}

// ── Artifacts ──────────────────────────────────────────────────────

/// A sealed, content-addressed set of published calendar exceptions plus the
/// executable calendar they produce.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalendarExceptionArtifact {
    schema_version: u32,
    artifact_id: String,
    venue: String,
    time_zone: ExchangeTimeZone,
    range_start: chrono::NaiveDate,
    range_end_inclusive: chrono::NaiveDate,
    /// The bar this artifact was actually held to when it was sealed. Recorded
    /// so a consumer can check it rather than take a caller's word for it later.
    require_authoritative: bool,
    source: SourceBatch,
    base: TradingCalendarSpec,
    source_records: Vec<CalendarSourceRecord>,
    exceptions: Vec<CalendarException>,
    calendar: TradingCalendar,
}

impl CalendarExceptionArtifact {
    pub fn artifact_id(&self) -> &str {
        &self.artifact_id
    }

    pub fn venue(&self) -> &str {
        &self.venue
    }

    pub const fn time_zone(&self) -> ExchangeTimeZone {
        self.time_zone
    }

    /// The inclusive exchange-local dates this artifact speaks for. Outside it,
    /// the artifact says nothing and the rule-only base stands.
    pub const fn covered_range(&self) -> (chrono::NaiveDate, chrono::NaiveDate) {
        (self.range_start, self.range_end_inclusive)
    }

    pub const fn source(&self) -> &SourceBatch {
        &self.source
    }

    pub fn exceptions(&self) -> &[CalendarException] {
        &self.exceptions
    }

    pub fn source_records(&self) -> &[CalendarSourceRecord] {
        &self.source_records
    }

    pub const fn calendar(&self) -> &TradingCalendar {
        &self.calendar
    }

    /// Whether this artifact is backed by exchange or contracted-vendor data.
    pub const fn is_authoritative(&self) -> bool {
        self.source.authority.is_authoritative()
    }

    /// Re-derive everything from the preserved records and prove the result is
    /// exactly what the sealed id says it is.
    pub fn verify(&self) -> Result<(), ReferenceDataError> {
        if self.schema_version != REFERENCE_DATA_SCHEMA_VERSION {
            return Err(ReferenceDataError::ArtifactIdentityMismatch);
        }
        // Validation runs first: deriving the id builds the base calendar, and
        // a decoded artifact must never be able to panic its way through.
        let base_calendar_id = validate_calendar_parts(
            &self.venue,
            self.time_zone,
            self.range_start,
            self.range_end_inclusive,
            self.require_authoritative,
            &self.source,
            &self.base,
            &self.source_records,
            &self.exceptions,
        )?;
        if self.artifact_id != calendar_artifact_id(self, &base_calendar_id) {
            return Err(ReferenceDataError::ArtifactIdentityMismatch);
        }
        if calendar_with_exceptions(&self.base, &self.exceptions, &self.artifact_id)?
            != self.calendar
        {
            return Err(ReferenceDataError::NonCanonicalArtifact);
        }
        Ok(())
    }
}

/// A sealed, content-addressed corporate-action schedule for one symbol.
///
/// Not `Eq`: a cash dividend is an `f64`. Equality of two artifacts is settled
/// by their ids, which hash a canonicalized bit pattern, not by float compare.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorporateActionArtifact {
    schema_version: u32,
    artifact_id: String,
    venue: String,
    symbol: String,
    time_zone: ExchangeTimeZone,
    currency: String,
    range_start_ns: i64,
    range_end_ns: i64,
    adjustment: AdjustmentPolicy,
    require_authoritative: bool,
    source: SourceBatch,
    source_records: Vec<CorporateActionSourceRecord>,
    schedule: CorporateActionSchedule,
}

impl CorporateActionArtifact {
    pub fn artifact_id(&self) -> &str {
        &self.artifact_id
    }

    pub const fn schedule(&self) -> &CorporateActionSchedule {
        &self.schedule
    }

    pub fn source_records(&self) -> &[CorporateActionSourceRecord] {
        &self.source_records
    }

    pub fn venue(&self) -> &str {
        &self.venue
    }

    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    pub fn currency(&self) -> &str {
        &self.currency
    }

    /// The dataset adjustment policy this schedule was checked against. Binding
    /// it to a differently adjusted dataset would double-count.
    pub const fn adjustment(&self) -> AdjustmentPolicy {
        self.adjustment
    }

    /// The half-open `[start, end)` UTC range this artifact speaks for.
    pub const fn covered_range_ns(&self) -> (i64, i64) {
        (self.range_start_ns, self.range_end_ns)
    }

    pub const fn source(&self) -> &SourceBatch {
        &self.source
    }

    pub const fn is_authoritative(&self) -> bool {
        self.source.authority.is_authoritative()
    }

    pub fn verify(&self) -> Result<(), ReferenceDataError> {
        if self.schema_version != REFERENCE_DATA_SCHEMA_VERSION {
            return Err(ReferenceDataError::ArtifactIdentityMismatch);
        }
        validate_batch_utc(
            &self.source,
            self.range_start_ns,
            self.range_end_ns,
            self.require_authoritative,
        )?;
        let rebuilt = convert_corporate_records(
            &self.venue,
            &self.symbol,
            self.time_zone,
            &self.currency,
            self.range_start_ns,
            self.range_end_ns,
            self.adjustment,
            &self.source_records,
        )?;
        if self.artifact_id != corporate_artifact_id(self) {
            return Err(ReferenceDataError::ArtifactIdentityMismatch);
        }
        if rebuilt != self.schedule {
            return Err(ReferenceDataError::NonCanonicalArtifact);
        }
        Ok(())
    }
}

/// The digest a source record must carry for its preserved raw bytes.
pub fn raw_source_sha256(raw: &str) -> String {
    sha256_hex(raw.as_bytes())
}

// ── Source validation ──────────────────────────────────────────────

/// The authority a source system can honestly claim. A cache is worth exactly
/// what it cached.
fn source_expected_authority(source: &SourceSystem) -> SourceAuthorityClass {
    match source {
        SourceSystem::ExchangePublication { .. } => SourceAuthorityClass::ExchangeOfficial,
        SourceSystem::ContractedVendor { .. } => SourceAuthorityClass::ContractedVendor,
        SourceSystem::YahooChartKeyless => SourceAuthorityClass::UnverifiedPublic,
        SourceSystem::RuleDerived { .. } => SourceAuthorityClass::DerivedRule,
        SourceSystem::ResearchDatabaseCache { upstream } => source_expected_authority(upstream),
        SourceSystem::Unavailable { .. } => SourceAuthorityClass::Unavailable,
    }
}

fn validate_source(
    batch: &SourceBatch,
    require_authoritative: bool,
) -> Result<(), ReferenceDataError> {
    if batch.identity_metadata_policy != IdentityMetadataPolicy::AsOfIncludedRetrievalExcluded {
        return Err(ReferenceDataError::UnsupportedIdentityPolicy);
    }
    if batch.as_of_ns > batch.retrieved_at_ns {
        return Err(ReferenceDataError::AsOfAfterRetrieval);
    }
    if batch.authority != source_expected_authority(&batch.source) {
        return Err(ReferenceDataError::DishonestAuthority);
    }
    if batch.authority == SourceAuthorityClass::Unavailable {
        return Err(ReferenceDataError::SourceUnavailable);
    }
    if require_authoritative && !batch.authority.is_authoritative() {
        return Err(ReferenceDataError::NotAuthoritative);
    }
    if !batch.complete {
        return Err(ReferenceDataError::IncompleteRange);
    }
    Ok(())
}

fn validate_batch_dates(
    batch: &SourceBatch,
    start: chrono::NaiveDate,
    end: chrono::NaiveDate,
    require_authoritative: bool,
) -> Result<(), ReferenceDataError> {
    if start > end {
        return Err(ReferenceDataError::InvalidRange);
    }
    validate_source(batch, require_authoritative)?;
    match batch.coverage {
        SourceCoverage::ExchangeDateRange {
            start: covered,
            end_inclusive,
        } if covered <= start && end_inclusive >= end => Ok(()),
        _ => Err(ReferenceDataError::IncompleteRange),
    }
}

fn validate_batch_utc(
    batch: &SourceBatch,
    start: i64,
    end: i64,
    require_authoritative: bool,
) -> Result<(), ReferenceDataError> {
    if start >= end {
        return Err(ReferenceDataError::InvalidRange);
    }
    validate_source(batch, require_authoritative)?;
    match batch.coverage {
        SourceCoverage::UtcRange { start_ns, end_ns } if start_ns <= start && end_ns >= end => {
            Ok(())
        }
        _ => Err(ReferenceDataError::IncompleteRange),
    }
}

/// The record bound, applied before a batch is cloned, sorted or hashed. An
/// oversized batch costs one comparison rather than an `n log n` sort it was
/// always going to be rejected after.
fn check_record_count(found: usize) -> Result<(), ReferenceDataError> {
    if found > MAX_REFERENCE_RECORDS {
        Err(ReferenceDataError::TooManyRecords { found })
    } else {
        Ok(())
    }
}

fn check_text(field: &'static str, text: &str) -> Result<(), ReferenceDataError> {
    if text.is_empty()
        || text.trim() != text
        || text.len() > MAX_TEXT_BYTES
        || text.chars().any(char::is_control)
    {
        Err(ReferenceDataError::InvalidText { field })
    } else {
        Ok(())
    }
}

/// Parse an RFC-3339 instant that states UTC explicitly.
///
/// A bare local time is genuinely ambiguous on a DST boundary, and a `+00:00`
/// spelling is a second encoding of one instant that would let two byte strings
/// share an artifact id. Both are refused rather than normalized.
pub fn parse_utc_ns(value: &str) -> Result<i64, ReferenceDataError> {
    if !value.ends_with('Z') {
        return Err(ReferenceDataError::TimezoneAmbiguous {
            value: value.into(),
        });
    }
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .and_then(|value| value.timestamp_nanos_opt())
        .ok_or_else(|| ReferenceDataError::TimezoneAmbiguous {
            value: value.into(),
        })
}

fn validate_raw(id: &str, expected_hash: &str, raw: &str) -> Result<(), ReferenceDataError> {
    check_text("source_record_id", id)?;
    if raw.is_empty() || raw.len() > MAX_RAW_SOURCE_BYTES {
        return Err(ReferenceDataError::OversizeRawSource {
            source_record_id: id.into(),
        });
    }
    if !is_digest(expected_hash) || raw_source_sha256(raw) != expected_hash {
        return Err(ReferenceDataError::RawRecordIdentityMismatch {
            source_record_id: id.into(),
        });
    }
    Ok(())
}

// ── Calendar materialization ───────────────────────────────────────

/// Seal published calendar exceptions into a verified artifact.
pub fn materialize_calendar(
    request: &CalendarMaterializationRequest,
) -> Result<CalendarExceptionArtifact, ReferenceDataError> {
    validate_batch_dates(
        &request.source,
        request.range_start,
        request.range_end_inclusive,
        request.require_authoritative,
    )?;
    check_record_count(request.records.len())?;
    // Canonical order is exchange-local date, then source id. Two operators who
    // listed the same closures in different orders seal the same artifact.
    let mut records = request.records.clone();
    records.sort_by(|left, right| {
        (left.local_date, &left.source_record_id).cmp(&(right.local_date, &right.source_record_id))
    });
    let exceptions = records
        .iter()
        .map(|record| calendar_exception_from_record(record, &request.base))
        .collect::<Result<Vec<_>, _>>()?;
    let base_calendar_id = validate_calendar_parts(
        &request.venue,
        request.time_zone,
        request.range_start,
        request.range_end_inclusive,
        request.require_authoritative,
        &request.source,
        &request.base,
        &records,
        &exceptions,
    )?;

    let mut artifact = CalendarExceptionArtifact {
        schema_version: REFERENCE_DATA_SCHEMA_VERSION,
        artifact_id: String::new(),
        venue: request.venue.clone(),
        time_zone: request.time_zone,
        range_start: request.range_start,
        range_end_inclusive: request.range_end_inclusive,
        require_authoritative: request.require_authoritative,
        source: request.source.clone(),
        base: request.base.clone(),
        source_records: records,
        exceptions,
        calendar: TradingCalendar::build(&request.base)?,
    };
    artifact.artifact_id = calendar_artifact_id(&artifact, &base_calendar_id);
    artifact.calendar =
        calendar_with_exceptions(&artifact.base, &artifact.exceptions, &artifact.artifact_id)?;
    artifact.verify()?;
    Ok(artifact)
}

/// Project one published record onto the base calendar's own session shape.
fn calendar_exception_from_record(
    record: &CalendarSourceRecord,
    base: &TradingCalendarSpec,
) -> Result<CalendarException, ReferenceDataError> {
    let malformed = || ReferenceDataError::MalformedRecord {
        source_record_id: record.source_record_id.clone(),
    };
    let kind = match &record.kind {
        CalendarExceptionSourceKind::Closed => CalendarExceptionKind::Closed,
        CalendarExceptionSourceKind::EarlyClose { close_minute } => {
            // The shortened day is the venue's *own* declared windows cut at the
            // published minute. A policy-only calendar declares none, so there
            // is nothing to shorten and the record is refused rather than handed
            // an invented opening bell.
            let SessionRule::LocalWindows { windows, .. } = &base.session else {
                return Err(malformed());
            };
            CalendarExceptionKind::SessionOverride {
                windows: shorten_windows(windows, *close_minute).ok_or_else(malformed)?,
            }
        }
        CalendarExceptionSourceKind::OpenOverride { windows } => {
            CalendarExceptionKind::SessionOverride {
                windows: windows.clone(),
            }
        }
    };
    Ok(CalendarException {
        local_date: record.local_date,
        source_record_id: record.source_record_id.clone(),
        label: record.label.clone(),
        kind,
    })
}

/// Check every calendar invariant and return the base calendar's id.
#[allow(clippy::too_many_arguments)]
fn validate_calendar_parts(
    venue: &str,
    time_zone: ExchangeTimeZone,
    range_start: chrono::NaiveDate,
    range_end: chrono::NaiveDate,
    require_authoritative: bool,
    source: &SourceBatch,
    base: &TradingCalendarSpec,
    records: &[CalendarSourceRecord],
    exceptions: &[CalendarException],
) -> Result<String, ReferenceDataError> {
    check_text("venue", venue)?;
    validate_batch_dates(source, range_start, range_end, require_authoritative)?;
    // The base is the rule-only half. A base that already carries exceptions
    // would put two exception sets in one calendar with no way to say which
    // artifact published which date.
    if base.time_zone != time_zone
        || !base.exceptions.is_empty()
        || base.exception_artifact_id.is_some()
    {
        return Err(ReferenceDataError::ScopeMismatch {
            source_record_id: "base-calendar".into(),
        });
    }
    check_record_count(records.len())?;
    if records.len() != exceptions.len() {
        return Err(ReferenceDataError::NonCanonicalArtifact);
    }
    let mut ids = BTreeSet::new();
    let mut dates = BTreeMap::<chrono::NaiveDate, &CalendarExceptionKind>::new();
    let mut previous: Option<(chrono::NaiveDate, &str)> = None;
    for (record, exception) in records.iter().zip(exceptions) {
        validate_raw(
            &record.source_record_id,
            &record.raw_record_sha256,
            &record.raw_source,
        )?;
        check_text("label", &record.label)?;
        if record.venue != venue || record.time_zone != time_zone {
            return Err(ReferenceDataError::ScopeMismatch {
                source_record_id: record.source_record_id.clone(),
            });
        }
        if record.local_date < range_start || record.local_date > range_end {
            return Err(ReferenceDataError::MalformedRecord {
                source_record_id: record.source_record_id.clone(),
            });
        }
        if !ids.insert(record.source_record_id.clone()) {
            return Err(ReferenceDataError::DuplicateSourceRecord {
                source_record_id: record.source_record_id.clone(),
            });
        }
        // One exchange-local date carries exactly one published verdict. A
        // repeat of the identical verdict is a duplicate; a different one is a
        // source conflict. Neither may be silently collapsed into one
        // exception, and the two are named apart because the operator fixes
        // them differently — one record is redundant, the other is wrong.
        if let Some(prior) = dates.get(&record.local_date) {
            return Err(if **prior == exception.kind {
                ReferenceDataError::DuplicateEvent {
                    source_record_id: record.source_record_id.clone(),
                }
            } else {
                ReferenceDataError::ConflictingEvent {
                    source_record_id: record.source_record_id.clone(),
                }
            });
        }
        dates.insert(record.local_date, &exception.kind);
        // A decoded artifact must present its records already in canonical
        // order, or two orderings of one set would hash to two ids.
        if previous.is_some_and(|prior| prior > (record.local_date, &record.source_record_id)) {
            return Err(ReferenceDataError::NonCanonicalArtifact);
        }
        previous = Some((record.local_date, &record.source_record_id));
        if calendar_exception_from_record(record, base)? != *exception {
            return Err(ReferenceDataError::NonCanonicalArtifact);
        }
    }
    Ok(TradingCalendar::build(base)?.calendar_id().to_string())
}

fn calendar_with_exceptions(
    base: &TradingCalendarSpec,
    exceptions: &[CalendarException],
    artifact_id: &str,
) -> Result<TradingCalendar, ReferenceDataError> {
    let mut spec = base.clone();
    spec.exceptions = exceptions.to_vec();
    spec.exception_artifact_id = (!exceptions.is_empty()).then(|| artifact_id.to_string());
    Ok(TradingCalendar::build(&spec)?)
}

// ── Corporate-action materialization ───────────────────────────────

/// Seal published corporate actions for one symbol into a verified artifact.
pub fn materialize_corporate_actions(
    request: &CorporateActionMaterializationRequest,
) -> Result<CorporateActionArtifact, ReferenceDataError> {
    validate_batch_utc(
        &request.source,
        request.range_start_ns,
        request.range_end_ns,
        request.require_authoritative,
    )?;
    check_record_count(request.records.len())?;
    // Sort into the schedule's own canonical order — effective instant, then
    // action rank, then source id — so the record list and the sealed action
    // list are two views of one order rather than two independent sorts.
    let mut records = request.records.clone();
    records.sort_by(|left, right| corporate_sort_key(left).cmp(&corporate_sort_key(right)));
    let schedule = convert_corporate_records(
        &request.venue,
        &request.symbol,
        request.time_zone,
        &request.currency,
        request.range_start_ns,
        request.range_end_ns,
        request.adjustment,
        &records,
    )?;
    let mut artifact = CorporateActionArtifact {
        schema_version: REFERENCE_DATA_SCHEMA_VERSION,
        artifact_id: String::new(),
        venue: request.venue.clone(),
        symbol: request.symbol.clone(),
        time_zone: request.time_zone,
        currency: request.currency.clone(),
        range_start_ns: request.range_start_ns,
        range_end_ns: request.range_end_ns,
        adjustment: request.adjustment,
        require_authoritative: request.require_authoritative,
        source: request.source.clone(),
        source_records: records,
        schedule,
    };
    artifact.artifact_id = corporate_artifact_id(&artifact);
    artifact.verify()?;
    Ok(artifact)
}

/// Canonical sort key. An unparseable timestamp sorts first and is rejected by
/// [`convert_corporate_records`] moments later, so ordering never depends on
/// how a malformed record happened to compare.
fn corporate_sort_key(record: &CorporateActionSourceRecord) -> (i64, u8, &str) {
    (
        parse_utc_ns(&record.effective_utc).unwrap_or(i64::MIN),
        record.kind.order_rank(),
        &record.source_record_id,
    )
}

#[allow(clippy::too_many_arguments)]
fn convert_corporate_records(
    venue: &str,
    symbol: &str,
    time_zone: ExchangeTimeZone,
    currency: &str,
    range_start: i64,
    range_end: i64,
    adjustment: AdjustmentPolicy,
    records: &[CorporateActionSourceRecord],
) -> Result<CorporateActionSchedule, ReferenceDataError> {
    check_text("venue", venue)?;
    check_text("symbol", symbol)?;
    check_text("currency", currency)?;
    if range_start >= range_end {
        return Err(ReferenceDataError::InvalidRange);
    }
    check_record_count(records.len())?;
    let mut ids = BTreeSet::new();
    let mut semantic = BTreeMap::<(i64, u8), String>::new();
    let mut actions = Vec::with_capacity(records.len());
    let mut previous: Option<(i64, u8, &str)> = None;
    for record in records {
        validate_raw(
            &record.source_record_id,
            &record.raw_record_sha256,
            &record.raw_source,
        )?;
        if record.venue != venue
            || record.symbol != symbol
            || record.time_zone != time_zone
            || record.currency != currency
        {
            return Err(ReferenceDataError::ScopeMismatch {
                source_record_id: record.source_record_id.clone(),
            });
        }
        if !ids.insert(record.source_record_id.clone()) {
            return Err(ReferenceDataError::DuplicateSourceRecord {
                source_record_id: record.source_record_id.clone(),
            });
        }
        let at = parse_utc_ns(&record.effective_utc)?;
        if at < range_start || at >= range_end {
            return Err(ReferenceDataError::MalformedRecord {
                source_record_id: record.source_record_id.clone(),
            });
        }
        let kind = convert_action_kind(record)?;
        let payload = canonical_action_payload(&kind);
        // One instant carries at most one action of each class. A repeat of the
        // identical event is a duplicate; a different one is a source conflict.
        // Neither may be silently collapsed into a single applied event, and
        // both name the record that arrived second — that is the one an
        // operator either drops as redundant or resolves as wrong.
        if let Some(prior_payload) = semantic.get(&(at, record.kind.order_rank())) {
            return Err(if *prior_payload == payload {
                ReferenceDataError::DuplicateEvent {
                    source_record_id: record.source_record_id.clone(),
                }
            } else {
                ReferenceDataError::ConflictingEvent {
                    source_record_id: record.source_record_id.clone(),
                }
            });
        }
        semantic.insert((at, record.kind.order_rank()), payload);
        // Decoded records must already be canonically ordered; otherwise one
        // set of records would have as many ids as it has orderings.
        let key = (
            at,
            record.kind.order_rank(),
            record.source_record_id.as_str(),
        );
        if previous.is_some_and(|prior| prior > key) {
            return Err(ReferenceDataError::NonCanonicalArtifact);
        }
        previous = Some(key);
        actions.push(CorporateAction {
            symbol: symbol.into(),
            effective_time_ns: at,
            kind,
        });
    }
    let schedule = CorporateActionSchedule::build(&actions)
        .map_err(|error| ReferenceDataError::Corporate(error.to_string()))?;
    schedule
        .check_adjustment_consistency(adjustment)
        .map_err(|_| ReferenceDataError::AdjustedPriceDoubleCounting { adjustment })?;
    Ok(schedule)
}

fn convert_action_kind(
    record: &CorporateActionSourceRecord,
) -> Result<CorporateActionKind, ReferenceDataError> {
    match &record.kind {
        CorporateActionSourceKind::Split {
            numerator,
            denominator,
        } => Ok(CorporateActionKind::Split {
            numerator: parse_ratio_leg(numerator, &record.source_record_id)?,
            denominator: parse_ratio_leg(denominator, &record.source_record_id)?,
        }),
        CorporateActionSourceKind::CashDividend { amount_per_unit } => {
            Ok(CorporateActionKind::CashDividend {
                amount_per_unit: parse_decimal_amount(amount_per_unit, &record.source_record_id)?,
            })
        }
        CorporateActionSourceKind::SymbolChange { new_symbol } => {
            Ok(CorporateActionKind::SymbolChange {
                new_symbol: new_symbol.clone(),
            })
        }
        CorporateActionSourceKind::Delisting => Ok(CorporateActionKind::Delisting),
        CorporateActionSourceKind::Unsupported { action_type } => {
            Err(ReferenceDataError::UnsupportedActionType {
                action_type: action_type.clone(),
            })
        }
    }
}

/// One split leg: a positive decimal integer in exactly one spelling. `2.0`,
/// `02` and `0` are all refused, so a ratio has a single wire encoding.
fn parse_ratio_leg(value: &str, id: &str) -> Result<u32, ReferenceDataError> {
    let malformed = || ReferenceDataError::MalformedRecord {
        source_record_id: id.into(),
    };
    if value.is_empty()
        || value.starts_with('0')
        || !value.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(malformed());
    }
    value.parse().map_err(|_| malformed())
}

/// A positive decimal amount in exactly one spelling: no sign, no leading
/// zeros, no trailing point, no trailing fractional zero, at most eight
/// fractional digits. `0.250` and `0.25` must not both reach the digest.
fn parse_decimal_amount(value: &str, id: &str) -> Result<f64, ReferenceDataError> {
    let malformed = || ReferenceDataError::MalformedRecord {
        source_record_id: id.into(),
    };
    if value.ends_with('.') {
        return Err(malformed());
    }
    let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
    if whole.is_empty()
        || (whole.starts_with('0') && whole != "0")
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || fraction.len() > 8
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
        || fraction.ends_with('0')
    {
        return Err(malformed());
    }
    let whole: u64 = whole.parse().map_err(|_| malformed())?;
    let fraction_value: u64 = if fraction.is_empty() {
        0
    } else {
        fraction.parse().map_err(|_| malformed())?
    };
    if whole > 1_000_000_000_000 || (whole == 0 && fraction_value == 0) {
        return Err(malformed());
    }
    let scale = 10_u64.pow(fraction.len() as u32);
    Ok(whole as f64 + fraction_value as f64 / scale as f64)
}

/// A collision-free textual form of an action's economic content, used to tell
/// a duplicate record apart from a conflicting one.
fn canonical_action_payload(kind: &CorporateActionKind) -> String {
    match kind {
        CorporateActionKind::Split {
            numerator,
            denominator,
        } => format!("split:{numerator}:{denominator}"),
        CorporateActionKind::CashDividend { amount_per_unit } => {
            format!("dividend:{:016x}", canonical_f64_bits(*amount_per_unit))
        }
        CorporateActionKind::SymbolChange { new_symbol } => format!("symbol:{new_symbol}"),
        CorporateActionKind::Delisting => "delisting".into(),
    }
}

// ── Identity ───────────────────────────────────────────────────────

/// Length-framed SHA-256. Framing every field is what stops `"ab" + "c"` and
/// `"a" + "bc"` from hashing alike.
struct CanonicalHash(Sha256);

impl CanonicalHash {
    fn new(domain: &str) -> Self {
        let mut this = Self(Sha256::new());
        this.text(domain);
        this
    }

    fn bytes(&mut self, value: &[u8]) {
        self.0.update((value.len() as u64).to_be_bytes());
        self.0.update(value);
    }

    fn text(&mut self, value: &str) {
        self.bytes(value.as_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.bytes(&value.to_be_bytes());
    }

    fn i64(&mut self, value: i64) {
        self.bytes(&value.to_be_bytes());
    }

    fn flag(&mut self, value: bool) {
        self.u64(u64::from(value));
    }

    fn finish(self) -> String {
        hex(&self.0.finalize())
    }
}

fn hash_source(hash: &mut CanonicalHash, source: &SourceBatch) {
    hash.text(&serde_json::to_string(&source.source).expect("source system serializes"));
    hash.text(source.authority.wire_id());
    hash.text(&serde_json::to_string(&source.coverage).expect("coverage serializes"));
    hash.flag(source.complete);
    hash.i64(source.as_of_ns);
    // Named rather than skipped, so the exclusion is a recorded decision a
    // future schema change has to step over deliberately.
    hash.text("retrieved_at_ns:excluded");
    hash.text("as_of_included_retrieval_excluded");
}

fn calendar_artifact_id(artifact: &CalendarExceptionArtifact, base_calendar_id: &str) -> String {
    let mut hash = CanonicalHash::new("typhoon.strategy.calendar-exceptions.v1");
    hash.u64(u64::from(artifact.schema_version));
    hash.text(&artifact.venue);
    hash.text(artifact.time_zone.wire_id());
    hash.text(&artifact.range_start.to_string());
    hash.text(&artifact.range_end_inclusive.to_string());
    hash.flag(artifact.require_authoritative);
    hash_source(&mut hash, &artifact.source);
    hash.text(base_calendar_id);
    hash.u64(artifact.source_records.len() as u64);
    for record in &artifact.source_records {
        hash.text(&record.source_record_id);
        hash.text(&record.raw_record_sha256);
        hash.text(&record.venue);
        hash.text(record.time_zone.wire_id());
        hash.text(&record.local_date.to_string());
        hash.text(&serde_json::to_string(&record.kind).expect("source kind serializes"));
        hash.text(&record.label);
    }
    // The executable exceptions hash separately from the records they came
    // from: projecting an early close onto the base calendar's windows is part
    // of what this artifact *means*, not a detail a reader may re-derive
    // differently under a later build.
    hash.u64(artifact.exceptions.len() as u64);
    for exception in &artifact.exceptions {
        hash.text(&exception.local_date.to_string());
        hash.text(&exception.source_record_id);
        hash.text(&serde_json::to_string(&exception.kind).expect("exception kind serializes"));
    }
    hash.finish()
}

fn corporate_artifact_id(artifact: &CorporateActionArtifact) -> String {
    let mut hash = CanonicalHash::new("typhoon.strategy.corporate-actions.v1");
    hash.u64(u64::from(artifact.schema_version));
    hash.text(&artifact.venue);
    hash.text(&artifact.symbol);
    hash.text(artifact.time_zone.wire_id());
    hash.text(&artifact.currency);
    hash.i64(artifact.range_start_ns);
    hash.i64(artifact.range_end_ns);
    hash.text(artifact.adjustment.wire_id());
    hash.flag(artifact.require_authoritative);
    hash_source(&mut hash, &artifact.source);
    hash.u64(artifact.source_records.len() as u64);
    for record in &artifact.source_records {
        hash.text(&record.source_record_id);
        hash.text(&record.raw_record_sha256);
        hash.text(&record.effective_utc);
    }
    hash.u64(artifact.schedule.actions().len() as u64);
    for action in artifact.schedule.actions() {
        hash.i64(action.effective_time_ns);
        hash.text(&action.symbol);
        hash.text(&canonical_action_payload(&action.kind));
    }
    hash.finish()
}

/// `-0.0` and `0.0` are the same amount, so only one bit pattern may reach a
/// hash. A `NaN` amount never gets here: the schedule rejects non-finite
/// dividends before an artifact can be sealed.
fn canonical_f64_bits(value: f64) -> u64 {
    if value == 0.0 { 0 } else { value.to_bits() }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

// ── Bounded codec ──────────────────────────────────────────────────

pub fn encode_calendar_artifact(
    artifact: &CalendarExceptionArtifact,
) -> Result<Vec<u8>, ReferenceDataError> {
    artifact.verify()?;
    encode_bounded(artifact)
}

pub fn decode_calendar_artifact(
    bytes: &[u8],
) -> Result<CalendarExceptionArtifact, ReferenceDataError> {
    let artifact: CalendarExceptionArtifact = decode_bounded(bytes)?;
    artifact.verify()?;
    Ok(artifact)
}

pub fn encode_corporate_action_artifact(
    artifact: &CorporateActionArtifact,
) -> Result<Vec<u8>, ReferenceDataError> {
    artifact.verify()?;
    encode_bounded(artifact)
}

pub fn decode_corporate_action_artifact(
    bytes: &[u8],
) -> Result<CorporateActionArtifact, ReferenceDataError> {
    let artifact: CorporateActionArtifact = decode_bounded(bytes)?;
    artifact.verify()?;
    Ok(artifact)
}

fn encode_bounded<T: Serialize>(value: &T) -> Result<Vec<u8>, ReferenceDataError> {
    let bytes =
        serde_json::to_vec(value).map_err(|error| ReferenceDataError::Decode(error.to_string()))?;
    if bytes.len() > MAX_REFERENCE_ARTIFACT_BYTES {
        return Err(ReferenceDataError::ArtifactTooLarge);
    }
    Ok(bytes)
}

/// Decode strictly: bound the bytes *before* parsing, reject unknown fields,
/// and require the input to be the one canonical encoding. Re-serializing and
/// comparing is what stops two byte strings from sharing one artifact id.
fn decode_bounded<T: DeserializeOwned + Serialize>(bytes: &[u8]) -> Result<T, ReferenceDataError> {
    if bytes.len() > MAX_REFERENCE_ARTIFACT_BYTES {
        return Err(ReferenceDataError::ArtifactTooLarge);
    }
    let value: T = serde_json::from_slice(bytes)
        .map_err(|error| ReferenceDataError::Decode(error.to_string()))?;
    let canonical = serde_json::to_vec(&value)
        .map_err(|error| ReferenceDataError::Decode(error.to_string()))?;
    if canonical != bytes {
        return Err(ReferenceDataError::NonCanonicalArtifact);
    }
    Ok(value)
}

// ── Binding into an execution config ───────────────────────────────

/// Bind verified artifacts into execution settings for one instrument.
///
/// The calendar becomes the instrument's calendar, the schedule becomes the
/// run's corporate actions, and both artifact ids are recorded so the config id
/// changes when the reference data does. Nothing here defaults: an unverifiable
/// artifact is an error, not a quiet fall back to the rule-only calendar.
pub fn bind_reference_artifacts(
    settings: &ExecutionSettings,
    symbol: &str,
    currency: &str,
    calendar: &CalendarExceptionArtifact,
    actions: &CorporateActionArtifact,
) -> Result<ExecutionSettings, ReferenceDataError> {
    calendar.verify()?;
    actions.verify()?;
    if symbol != actions.symbol()
        || currency != actions.currency()
        || calendar.venue != actions.venue
        || calendar.time_zone != actions.time_zone
    {
        return Err(ReferenceDataError::ScopeMismatch {
            source_record_id: symbol.into(),
        });
    }
    // One config carries one corporate-action schedule. Binding a second,
    // different artifact would silently drop the first symbol's events, so it is
    // refused; multi-symbol schedules remain an M2 remainder.
    if settings
        .reference_data
        .corporate_action_artifact_id
        .as_deref()
        .is_some_and(|bound| bound != actions.artifact_id())
    {
        return Err(ReferenceDataError::Config(
            "a different corporate-action artifact is already bound to these settings".into(),
        ));
    }

    let mut output = settings.clone();
    let mut specs = output.instruments.specs().to_vec();
    if let Some(spec) = specs.iter_mut().find(|spec| spec.symbol == symbol) {
        if spec.currency != currency {
            return Err(ReferenceDataError::ScopeMismatch {
                source_record_id: symbol.into(),
            });
        }
        spec.calendar = Some(calendar.calendar.clone());
    } else {
        specs.push(InstrumentSpec {
            symbol: symbol.into(),
            currency: currency.into(),
            calendar: Some(calendar.calendar.clone()),
            financing: FinancingModel::None,
            price_tick: None,
        });
    }
    output.instruments = InstrumentRegistry::build(&specs)
        .map_err(|error| ReferenceDataError::Config(error.to_string()))?;
    output.corporate_actions = actions.schedule.clone();

    // Calendar ids accumulate as a sorted set: a config may bind one published
    // calendar per instrument, and re-binding the same one is idempotent.
    let mut calendar_ids: BTreeSet<String> = output
        .reference_data
        .calendar_artifact_ids
        .iter()
        .cloned()
        .collect();
    calendar_ids.insert(calendar.artifact_id.clone());
    output.reference_data = ReferenceDataBindings {
        calendar_artifact_ids: calendar_ids.into_iter().collect(),
        corporate_action_artifact_id: Some(actions.artifact_id.clone()),
    };
    Ok(output)
}

// ── Content-addressed store ────────────────────────────────────────

/// Which artifact family a stored file belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceArtifactKind {
    Calendar,
    CorporateActions,
}

impl ReferenceArtifactKind {
    pub const fn suffix(self) -> &'static str {
        match self {
            Self::Calendar => "calendar",
            Self::CorporateActions => "corporate",
        }
    }
}

/// A flat content-addressed directory of sealed artifacts.
///
/// Files are named by artifact id, so writing the same artifact twice is a
/// no-op. Reads are id-addressed rather than path-addressed: a caller cannot
/// hand this store a path outside its own root and have the bytes there decoded
/// as a trusted artifact.
pub struct ReferenceArtifactStore {
    root: PathBuf,
}

impl ReferenceArtifactStore {
    pub fn open(root: impl AsRef<Path>) -> Result<Self, ReferenceDataError> {
        std::fs::create_dir_all(root.as_ref())
            .map_err(|error| ReferenceDataError::Io(error.to_string()))?;
        Ok(Self {
            root: root.as_ref().to_path_buf(),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The file an artifact id maps to. Errors on anything that is not a
    /// 64-character lowercase digest, so no id can escape the root.
    pub fn path_for(
        &self,
        kind: ReferenceArtifactKind,
        id: &str,
    ) -> Result<PathBuf, ReferenceDataError> {
        if !is_digest(id) {
            return Err(ReferenceDataError::ArtifactIdentityMismatch);
        }
        Ok(self.root.join(format!("{id}.{}.json", kind.suffix())))
    }

    pub fn put_calendar(
        &self,
        artifact: &CalendarExceptionArtifact,
    ) -> Result<PathBuf, ReferenceDataError> {
        self.put(
            ReferenceArtifactKind::Calendar,
            artifact.artifact_id(),
            &encode_calendar_artifact(artifact)?,
        )
    }

    pub fn put_corporate_actions(
        &self,
        artifact: &CorporateActionArtifact,
    ) -> Result<PathBuf, ReferenceDataError> {
        self.put(
            ReferenceArtifactKind::CorporateActions,
            artifact.artifact_id(),
            &encode_corporate_action_artifact(artifact)?,
        )
    }

    fn put(
        &self,
        kind: ReferenceArtifactKind,
        id: &str,
        bytes: &[u8],
    ) -> Result<PathBuf, ReferenceDataError> {
        let path = self.path_for(kind, id)?;
        // Write-then-rename: a reader never observes a half-written artifact,
        // and a crash leaves the previous bytes rather than a truncated file.
        let temporary = self.root.join(format!(".{id}.{}.tmp", kind.suffix()));
        std::fs::write(&temporary, bytes)
            .map_err(|error| ReferenceDataError::Io(error.to_string()))?;
        std::fs::rename(&temporary, &path)
            .map_err(|error| ReferenceDataError::Io(error.to_string()))?;
        Ok(path)
    }

    fn read(&self, path: &Path) -> Result<Vec<u8>, ReferenceDataError> {
        // Size is checked from the metadata first, so an oversized file is
        // never read into memory just to be rejected afterwards.
        let metadata =
            std::fs::metadata(path).map_err(|error| ReferenceDataError::Io(error.to_string()))?;
        if metadata.len() > MAX_REFERENCE_ARTIFACT_BYTES as u64 {
            return Err(ReferenceDataError::ArtifactTooLarge);
        }
        let bytes =
            std::fs::read(path).map_err(|error| ReferenceDataError::Io(error.to_string()))?;
        if bytes.len() > MAX_REFERENCE_ARTIFACT_BYTES {
            return Err(ReferenceDataError::ArtifactTooLarge);
        }
        Ok(bytes)
    }

    pub fn load_calendar(&self, id: &str) -> Result<CalendarExceptionArtifact, ReferenceDataError> {
        let path = self.path_for(ReferenceArtifactKind::Calendar, id)?;
        let artifact = decode_calendar_artifact(&self.read(&path)?)?;
        if artifact.artifact_id() != id {
            return Err(ReferenceDataError::ArtifactIdentityMismatch);
        }
        Ok(artifact)
    }

    pub fn load_corporate_actions(
        &self,
        id: &str,
    ) -> Result<CorporateActionArtifact, ReferenceDataError> {
        let path = self.path_for(ReferenceArtifactKind::CorporateActions, id)?;
        let artifact = decode_corporate_action_artifact(&self.read(&path)?)?;
        if artifact.artifact_id() != id {
            return Err(ReferenceDataError::ArtifactIdentityMismatch);
        }
        Ok(artifact)
    }

    /// Every stored id of one kind, ascending. Names only — nothing is decoded,
    /// so a worker can list a store far larger than it could verify.
    pub fn list_ids(&self, kind: ReferenceArtifactKind) -> Result<Vec<String>, ReferenceDataError> {
        let suffix = format!(".{}.json", kind.suffix());
        let entries = std::fs::read_dir(&self.root)
            .map_err(|error| ReferenceDataError::Io(error.to_string()))?;
        let mut ids = BTreeSet::new();
        for entry in entries {
            let entry = entry.map_err(|error| ReferenceDataError::Io(error.to_string()))?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            let Some(id) = name.strip_suffix(&suffix) else {
                continue;
            };
            if is_digest(id) {
                ids.insert(id.to_string());
            }
        }
        Ok(ids.into_iter().collect())
    }
}

#[cfg(test)]
mod tests;
