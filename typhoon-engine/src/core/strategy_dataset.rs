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
//! One normalization is applied, and only this one:
//!
//! 1. **No NaN normalization.** NaN and infinities are rejected before hashing;
//!    accepting their many payloads and then choosing a canonical NaN would
//!    hide corrupt market data.
//!
//! Finite floats, including `+0.0` and `-0.0`, retain their exact bits so the
//! identity agrees with the byte-identical payload persisted by the store.
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
//! ## Two hashes, two jobs
//!
//! - [`DatasetManifest::dataset_id`] is the **data** address: metadata plus
//!   every bar field. Two identical bar series pulled under different QA
//!   thresholds share it, because they are the same data.
//! - [`DatasetManifest::manifest_id`] is the **seal**: the dataset id plus the
//!   calendar policy, the QA policy, the QA report hash, and the QA headline
//!   counts. This is the hash that ADR-135 §11.1 requires to cover "both the
//!   data and the QA report", and it is what [`DatasetManifest::verify`]
//!   proves. Retuning a QA threshold keeps the dataset id and moves the seal.
//!
//! ## Calendar honesty
//!
//! [`CalendarPolicy`] is a versioned four-variant enum, and its limits are
//! stated per variant rather than implied. There is **no venue-local
//! early-close handling and no per-symbol 24×7 xStock tier** here (see
//! ADR-110's own deferred list). Daily-or-coarser bars are judged by session
//! *date*; intraday bars are additionally judged against the venue window
//! where one is modelled. Gap detection runs only when the timeframe parses to
//! a fixed step *and* the calendar can adjudicate the missing slots; otherwise
//! it reports why it declined rather than guessing.

use crate::broker::alpaca::Bar;
use crate::core::market_session;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Wire-format version of [`DatasetManifest`]. Bump on any change to the
/// hashed encoding or the manifest's field set.
pub const DATASET_MANIFEST_SCHEMA_VERSION: u32 = 3;

/// Wire-format version of [`DatasetQaReport`].
pub const DATASET_QA_SCHEMA_VERSION: u32 = 2;

/// Wire-format version of [`DatasetQaPolicy`].
pub const DATASET_QA_POLICY_SCHEMA_VERSION: u32 = 1;

/// Domain-separation prefix for the dataset-id hash. Any change to the framing
/// rules must change this string *and* the manifest schema version.
const DATASET_ID_DOMAIN: &str = "typhoon.strategy_dataset.id.v3";

/// Domain-separation prefix for the QA-report hash.
const DATASET_QA_DOMAIN: &str = "typhoon.strategy_dataset.qa.v1";

/// Domain-separation prefix for the QA-policy hash (used for `qa_policy_id`).
const DATASET_QA_POLICY_DOMAIN: &str = "typhoon.strategy_dataset.qa_policy.v1";

/// Domain-separation prefix for the sealed-manifest hash.
const DATASET_MANIFEST_SEAL_DOMAIN: &str = "typhoon.strategy_dataset.manifest.v1";

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
/// Intentionally coarse and versioned. This is **not** a full exchange
/// calendar: there is no half-day/early-close handling and no per-symbol
/// xStock 24×7 tier (both are on ADR-110's own deferred list). It answers
/// exactly one question — is a bar at this UTC instant expected to exist?
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
    /// US equities: weekdays minus the rule-based US market holidays already
    /// shared with the session chip
    /// ([`market_session::is_us_market_trading_day`], ADR-110).
    ///
    /// Honest limitations: the trading date is the bar's **UTC** date, so a
    /// feed that stamps a session at the previous UTC day would be misjudged;
    /// intraday session hours (04:00–20:00 ET) are *not* enforced, because the
    /// dataset layer does not model early closes.
    UsEquityRegular,
    /// Kraken tokenized stocks: the 24×5 cycle from Sunday 20:00 ET to Friday
    /// 20:00 ET, minus US market holidays (ADR-110).
    ///
    /// Daily-or-coarser bars are judged exactly like [`Self::UsEquityRegular`]
    /// — by UTC calendar date — because a daily bar stamped at UTC midnight
    /// lands at 19:00 ET the day before and would otherwise be attributed to
    /// the wrong session. Intraday bars are judged against the ET window.
    ///
    /// Honest limitation: the ~10 symbols that also trade weekends are not
    /// distinguishable from the catalog flags (ADR-110 deferred item 2), so a
    /// genuine weekend xStock bar is reported as unexpected.
    XStock24x5,
}

/// How much time one bar stands for. A daily-or-coarser bar represents a whole
/// session and is judged by session *date*; an intraday bar represents an
/// instant and can be judged against a venue window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalendarGranularity {
    Intraday,
    Session,
}

/// Why the calendar does — or does not — expect a bar at an instant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalendarVerdict {
    Expected,
    /// A non-trading weekday-of-week under the policy.
    Weekend {
        weekday: chrono::Weekday,
    },
    /// A recognised US market holiday.
    Holiday {
        name: &'static str,
    },
    /// Inside a trading week but outside the venue's session window.
    OutsideSession {
        window: &'static str,
    },
}

impl CalendarPolicy {
    /// Versioned identifier recorded in the manifest and QA report, and hashed
    /// into the dataset id.
    pub fn policy_id(self) -> &'static str {
        match self {
            Self::Continuous24x7 => "continuous-24x7.v1",
            Self::WeekdaysOnly => "weekdays-only.v1",
            Self::UsEquityRegular => "us-equity-regular.v1",
            Self::XStock24x5 => "kraken-xstock-24x5.v1",
        }
    }

    /// Adjudicate one UTC instant. `granularity` decides whether a venue
    /// session window applies at all.
    pub fn verdict_at(
        self,
        utc: chrono::DateTime<chrono::Utc>,
        granularity: CalendarGranularity,
    ) -> CalendarVerdict {
        use chrono::{Datelike, Timelike, Weekday};
        match self {
            Self::Continuous24x7 => CalendarVerdict::Expected,
            Self::WeekdaysOnly => match utc.weekday() {
                weekday @ (Weekday::Sat | Weekday::Sun) => CalendarVerdict::Weekend { weekday },
                _ => CalendarVerdict::Expected,
            },
            Self::UsEquityRegular => {
                let date = utc.date_naive();
                match date.weekday() {
                    weekday @ (Weekday::Sat | Weekday::Sun) => CalendarVerdict::Weekend { weekday },
                    _ => match market_session::us_market_holiday(date) {
                        Some(name) => CalendarVerdict::Holiday { name },
                        None => CalendarVerdict::Expected,
                    },
                }
            }
            Self::XStock24x5 => {
                if granularity == CalendarGranularity::Session {
                    // A daily bar stands for a whole session and is attributed
                    // to its UTC calendar date, exactly like `UsEquityRegular`
                    // — a UTC-midnight stamp is 19:00 ET the day before, so
                    // reading the ET date here would shift every daily bar by
                    // one day.
                    return Self::UsEquityRegular.verdict_at(utc, granularity);
                }
                // Intraday: the 24×5 ET window plus the holiday rule. An
                // evening session belongs to the *next* calendar day's trading
                // date, so it is refused when that day is a holiday (ADR-110's
                // conservative "never promise an overnight into a holiday").
                let eastern = market_session::us_eastern_datetime(utc);
                let date = eastern.date();
                if let Some(name) = market_session::us_market_holiday(date) {
                    return CalendarVerdict::Holiday { name };
                }
                let evening = eastern.hour() >= 20;
                if evening {
                    let next = date + chrono::Duration::days(1);
                    if let Some(name) = market_session::us_market_holiday(next) {
                        return CalendarVerdict::Holiday { name };
                    }
                }
                // Friday 20:00 ET → Sunday 20:00 ET is one contiguous closure,
                // not two weekend days: Friday evening and Sunday daytime are
                // part of it, and Sunday evening is not.
                let weekend_closed = match date.weekday() {
                    Weekday::Sat => true,
                    Weekday::Sun => !evening,
                    Weekday::Fri => evening,
                    _ => false,
                };
                if weekend_closed {
                    return CalendarVerdict::OutsideSession {
                        window: "the Friday 20:00 ET – Sunday 20:00 ET weekend close",
                    };
                }
                CalendarVerdict::Expected
            }
        }
    }

    /// Whether a bar is expected at this UTC instant under the policy.
    fn expects_instant(
        self,
        utc: chrono::DateTime<chrono::Utc>,
        granularity: CalendarGranularity,
    ) -> bool {
        self.verdict_at(utc, granularity) == CalendarVerdict::Expected
    }

    /// Whether missing-slot counting can be adjudicated for a fixed step. A
    /// calendar with intraday session structure cannot judge an intraday slot
    /// without the session hours this layer deliberately does not model.
    fn can_adjudicate_step(self, step_seconds: u64) -> bool {
        match self {
            Self::Continuous24x7 => true,
            Self::WeekdaysOnly | Self::UsEquityRegular | Self::XStock24x5 => {
                step_seconds >= SECONDS_PER_DAY
            }
        }
    }
}

// ── QA policy ──────────────────────────────────────────────────────

/// Thresholds for the checks that need one. Identity-bearing: the sealed
/// manifest hashes this, so a retuned threshold produces a different
/// [`DatasetManifest::manifest_id`] and cannot be swapped in under a stored
/// dataset's back.
///
/// All arithmetic driven by these fields is restricted to IEEE-754 basic
/// operations (compare, add, subtract, multiply, divide, round) so a report —
/// and therefore its hash — is reproducible across platforms. No transcendental
/// function participates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DatasetQaPolicy {
    pub schema_version: u32,
    /// Multiple of the median-absolute-deviation added to the median relative
    /// move to form the robust spike band.
    pub spike_band_multiple: f64,
    /// Minimum number of usable relative moves before the band is computed at
    /// all. Below this the report says so rather than flagging noise.
    pub spike_min_samples: u32,
    /// Absolute floor on the relative move, so a flat series (MAD ≈ 0) cannot
    /// turn ordinary ticks into spikes.
    pub spike_min_relative_move: f64,
    /// Largest integer split ratio considered (`n:1` or `1:n`).
    pub level_shift_max_ratio: u32,
    /// Relative tolerance when matching a close-to-close ratio against an
    /// integer ratio, and when deciding that the new level held.
    pub level_shift_tolerance: f64,
    /// Hard cap on findings retained in one report.
    pub max_findings: u32,
}

impl Default for DatasetQaPolicy {
    fn default() -> Self {
        Self {
            schema_version: DATASET_QA_POLICY_SCHEMA_VERSION,
            spike_band_multiple: 8.0,
            spike_min_samples: 20,
            spike_min_relative_move: 0.10,
            level_shift_max_ratio: 1_000,
            level_shift_tolerance: 0.01,
            max_findings: 10_000,
        }
    }
}

impl DatasetQaPolicy {
    /// Reject anything that has no exact canonical encoding or would make a
    /// check meaningless. Every bound is stated here rather than clamped
    /// silently, because a clamped policy would hash as the value the caller
    /// asked for while behaving as something else.
    pub fn validate(&self) -> Result<(), DatasetError> {
        fn invalid(field: &'static str, reason: &str) -> DatasetError {
            DatasetError::InvalidQaPolicy {
                field,
                reason: reason.to_string(),
            }
        }
        fn finite_in(
            field: &'static str,
            value: f64,
            low: f64,
            high: f64,
        ) -> Result<(), DatasetError> {
            if !value.is_finite() {
                return Err(invalid(field, "must be finite"));
            }
            if value < low || value > high {
                return Err(invalid(field, &format!("must be within [{low}, {high}]")));
            }
            Ok(())
        }

        if self.schema_version != DATASET_QA_POLICY_SCHEMA_VERSION {
            return Err(invalid(
                "schema_version",
                &format!("this build supports {DATASET_QA_POLICY_SCHEMA_VERSION}"),
            ));
        }
        finite_in(
            "spike_band_multiple",
            self.spike_band_multiple,
            1.0,
            1_000.0,
        )?;
        finite_in(
            "spike_min_relative_move",
            self.spike_min_relative_move,
            f64::MIN_POSITIVE,
            100.0,
        )?;
        finite_in(
            "level_shift_tolerance",
            self.level_shift_tolerance,
            f64::MIN_POSITIVE,
            0.5,
        )?;
        if !(3..=1_000_000).contains(&self.spike_min_samples) {
            return Err(invalid("spike_min_samples", "must be within [3, 1000000]"));
        }
        if !(2..=100_000).contains(&self.level_shift_max_ratio) {
            return Err(invalid(
                "level_shift_max_ratio",
                "must be within [2, 100000]",
            ));
        }
        if !(1..=1_000_000).contains(&self.max_findings) {
            return Err(invalid("max_findings", "must be within [1, 1000000]"));
        }
        Ok(())
    }

    /// Versioned, content-addressed id — `qa.v<schema>:<16 hex>`. Recorded in
    /// the manifest and the QA report so a stored artifact names the exact
    /// thresholds it was judged under.
    pub fn policy_id(&self) -> String {
        let mut digest = CanonicalDigest::new(DATASET_QA_POLICY_DOMAIN);
        self.encode_into(&mut digest);
        let hash = digest.finish_hex();
        format!("qa.v{}:{}", self.schema_version, &hash[..16])
    }

    fn encode_into(&self, digest: &mut CanonicalDigest) {
        digest.tagged_u64("qa_policy.schema_version", self.schema_version as u64);
        digest.tagged_f64("qa_policy.spike_band_multiple", self.spike_band_multiple);
        digest.tagged_u64("qa_policy.spike_min_samples", self.spike_min_samples as u64);
        digest.tagged_f64(
            "qa_policy.spike_min_relative_move",
            self.spike_min_relative_move,
        );
        digest.tagged_u64(
            "qa_policy.level_shift_max_ratio",
            self.level_shift_max_ratio as u64,
        );
        digest.tagged_f64(
            "qa_policy.level_shift_tolerance",
            self.level_shift_tolerance,
        );
        digest.tagged_u64("qa_policy.max_findings", self.max_findings as u64);
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
    /// A QA policy setting has no exact encoding or is out of range.
    InvalidQaPolicy { field: &'static str, reason: String },
    /// A derived manifest field disagrees with the bars it was verified
    /// against — the manifest was edited, or the wrong bars were supplied.
    ManifestFieldMismatch {
        field: &'static str,
        expected: String,
        actual: String,
    },
    /// The recomputed dataset id does not match the recorded one.
    DatasetIdMismatch { expected: String, actual: String },
    /// The recomputed manifest seal does not match the recorded one.
    ManifestIdMismatch { expected: String, actual: String },
    /// A QA report was presented for a manifest it does not belong to.
    QaReportHashMismatch { expected: String, actual: String },
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
            Self::InvalidQaPolicy { field, reason } => {
                write!(f, "QA policy field `{field}` is invalid: {reason}")
            }
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
            Self::ManifestIdMismatch { expected, actual } => write!(
                f,
                "manifest seal mismatch: recorded {expected}, recomputed {actual}"
            ),
            Self::QaReportHashMismatch { expected, actual } => write!(
                f,
                "QA report hash mismatch: manifest seals {expected}, report hashes {actual}"
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
#[derive(Debug, Clone, PartialEq)]
pub struct DatasetManifestInput {
    pub symbol: String,
    pub timeframe: String,
    pub provenance: DatasetProvenance,
    pub adjustment: AdjustmentPolicy,
    pub calendar: CalendarPolicy,
    /// Thresholds the QA pass runs under. Not part of `dataset_id` — the bars
    /// are the same bars — but sealed into `manifest_id`.
    pub qa_policy: DatasetQaPolicy,
}

/// An immutable, content-addressed description of one bar series.
///
/// Construct with [`DatasetManifest::build`]; never edit the fields of a
/// manifest that is already recorded. A resync produces a *new* dataset id.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
    /// Thresholds the recorded QA report was produced under.
    pub qa_policy: DatasetQaPolicy,
    /// Versioned, content-addressed id of `qa_policy`.
    pub qa_policy_id: String,
    /// Number of bars, in the order supplied.
    pub bar_count: u64,
    /// Timestamp of the **first bar in the supplied order** — not the minimum.
    /// A disordered dataset is a QA defect, not a re-sorting trigger.
    pub first_timestamp: Option<String>,
    /// Timestamp of the **last bar in the supplied order** — not the maximum.
    pub last_timestamp: Option<String>,
    /// Lowercase hex SHA-256 over the canonical encoding of metadata + bars.
    pub dataset_id: String,
    /// Lowercase hex SHA-256 of the QA report produced for these bars.
    pub qa_report_hash: String,
    pub qa_error_count: u64,
    pub qa_warning_count: u64,
    /// Whether the QA pass hit its findings cap. Sealed so a truncated report
    /// cannot be presented as a complete one.
    pub qa_findings_truncated: bool,
    /// Lowercase hex SHA-256 sealing the dataset id together with the calendar
    /// policy, the QA policy, and the QA report (ADR-135 §11.1).
    pub manifest_id: String,
}

impl DatasetManifest {
    /// Build a manifest for `bars`: content-address the data, run the QA pass
    /// under `input.qa_policy`, and seal both together.
    ///
    /// Bar order is significant: reordering the same bars yields a different
    /// dataset. Returns an error if the QA policy is invalid or if any
    /// metadata string or bar value cannot be canonically encoded; semantic
    /// defects are *not* rejected here — they are what the QA report is for.
    pub fn build(input: &DatasetManifestInput, bars: &[Bar]) -> Result<Self, DatasetError> {
        Self::build_with_qa(input, bars).map(|(manifest, _)| manifest)
    }

    /// Build a manifest and hand back the QA report it sealed.
    ///
    /// QA is a pure function of `(bars, timeframe, calendar, qa_policy)`, so
    /// [`DatasetManifest::run_qa`] can always re-derive it — but re-deriving it
    /// costs another sort over every bar, and a caller that is about to store
    /// the report already needs it. This is that caller's entry point.
    pub fn build_with_qa(
        input: &DatasetManifestInput,
        bars: &[Bar],
    ) -> Result<(Self, DatasetQaReport), DatasetError> {
        input.qa_policy.validate()?;
        let dataset_id = compute_dataset_id(input, bars)?;
        let qa =
            run_dataset_qa_with_policy(&input.timeframe, input.calendar, &input.qa_policy, bars);
        let mut manifest = Self {
            schema_version: DATASET_MANIFEST_SCHEMA_VERSION,
            symbol: input.symbol.clone(),
            timeframe: input.timeframe.clone(),
            provenance: input.provenance.clone(),
            adjustment: input.adjustment,
            calendar: input.calendar,
            calendar_policy_id: input.calendar.policy_id().to_string(),
            qa_policy: input.qa_policy.clone(),
            qa_policy_id: input.qa_policy.policy_id(),
            bar_count: bars.len() as u64,
            first_timestamp: bars.first().map(|b| b.timestamp.clone()),
            last_timestamp: bars.last().map(|b| b.timestamp.clone()),
            dataset_id,
            qa_report_hash: qa.report_hash(),
            qa_error_count: qa.error_count() as u64,
            qa_warning_count: qa.warning_count() as u64,
            qa_findings_truncated: qa.findings_truncated,
            manifest_id: String::new(),
        };
        manifest.manifest_id = manifest.compute_seal();
        Ok((manifest, qa))
    }

    /// The caller-supplied half of this manifest, for rebuilding it.
    pub fn to_input(&self) -> DatasetManifestInput {
        DatasetManifestInput {
            symbol: self.symbol.clone(),
            timeframe: self.timeframe.clone(),
            provenance: self.provenance.clone(),
            adjustment: self.adjustment,
            calendar: self.calendar,
            qa_policy: self.qa_policy.clone(),
        }
    }

    /// The sealed hash over this manifest's identity-bearing fields. Excludes
    /// `manifest_id` itself; everything else — including the QA policy, the QA
    /// report hash, and the QA counts — is covered.
    fn compute_seal(&self) -> String {
        let mut digest = CanonicalDigest::new(DATASET_MANIFEST_SEAL_DOMAIN);
        digest.tagged_u64("schema_version", self.schema_version as u64);
        digest.tagged_text("dataset_id", &self.dataset_id);
        digest.tagged_text("symbol", &self.symbol);
        digest.tagged_text("timeframe", &self.timeframe);
        digest.tagged_text("source", &self.provenance.source);
        digest.tagged_text("venue", &self.provenance.venue);
        digest.tagged_text("pipeline", &self.provenance.pipeline);
        digest.tagged_text("adjustment", self.adjustment.wire_id());
        digest.tagged_text("calendar_policy_id", &self.calendar_policy_id);
        self.qa_policy.encode_into(&mut digest);
        digest.tagged_text("qa_policy_id", &self.qa_policy_id);
        digest.tagged_u64("bar_count", self.bar_count);
        digest.tagged_optional_text("first_timestamp", self.first_timestamp.as_deref());
        digest.tagged_optional_text("last_timestamp", self.last_timestamp.as_deref());
        digest.tagged_text("qa_report_hash", &self.qa_report_hash);
        digest.tagged_u64("qa_error_count", self.qa_error_count);
        digest.tagged_u64("qa_warning_count", self.qa_warning_count);
        digest.tagged_u64(
            "qa_findings_truncated",
            u64::from(self.qa_findings_truncated),
        );
        digest.finish_hex()
    }

    /// Prove that this manifest is internally consistent — its schema version
    /// is supported and its recorded seal matches its own fields — **without**
    /// the bars.
    ///
    /// This is the check a loader can afford before touching a payload. It
    /// catches an edited manifest; it cannot catch a payload that does not
    /// belong to it, which is what [`DatasetManifest::verify`] is for.
    pub fn verify_seal(&self) -> Result<(), DatasetError> {
        self.check_schema_version()?;
        let seal = self.compute_seal();
        if self.manifest_id == seal {
            Ok(())
        } else {
            Err(DatasetError::ManifestIdMismatch {
                expected: self.manifest_id.clone(),
                actual: seal,
            })
        }
    }

    /// Prove that `report` is the QA report this manifest sealed.
    pub fn verify_qa_report(&self, report: &DatasetQaReport) -> Result<(), DatasetError> {
        let actual = report.report_hash();
        if actual == self.qa_report_hash {
            Ok(())
        } else {
            Err(DatasetError::QaReportHashMismatch {
                expected: self.qa_report_hash.clone(),
                actual,
            })
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
        expect_field("qa_policy_id", &self.qa_policy_id, &rebuilt.qa_policy_id)?;
        expect_field(
            "qa_report_hash",
            &self.qa_report_hash,
            &rebuilt.qa_report_hash,
        )?;
        expect_field(
            "qa_error_count",
            self.qa_error_count,
            rebuilt.qa_error_count,
        )?;
        expect_field(
            "qa_warning_count",
            self.qa_warning_count,
            rebuilt.qa_warning_count,
        )?;
        expect_field(
            "qa_findings_truncated",
            self.qa_findings_truncated,
            rebuilt.qa_findings_truncated,
        )?;

        if self.dataset_id != rebuilt.dataset_id {
            return Err(DatasetError::DatasetIdMismatch {
                expected: self.dataset_id.clone(),
                actual: rebuilt.dataset_id,
            });
        }
        // Recomputed from *this* manifest's own fields, so an edit that the
        // per-field checks above cannot see (a swapped provenance venue, say)
        // still moves the seal.
        let seal = self.compute_seal();
        if self.manifest_id != seal {
            return Err(DatasetError::ManifestIdMismatch {
                expected: self.manifest_id.clone(),
                actual: seal,
            });
        }
        Ok(())
    }

    /// Run the deterministic QA pass using the timeframe, calendar policy, and
    /// QA thresholds this manifest recorded — never an ambient default.
    pub fn run_qa(&self, bars: &[Bar]) -> DatasetQaReport {
        run_dataset_qa_with_policy(&self.timeframe, self.calendar, &self.qa_policy, bars)
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

/// Exact, platform-independent bits for a finite `f64`.
///
/// Every value, including either zero sign, keeps its exact IEEE-754 bit
/// pattern. This is pure integer work — no floating-point arithmetic
/// participates in identity. Callers must reject non-finite values first.
fn canonical_f64_bits(value: f64) -> u64 {
    value.to_bits()
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

    /// `None` and `Some("")` must not collide, so presence is framed as its own
    /// byte before any payload.
    fn tagged_optional_text(&mut self, tag: &str, value: Option<&str>) {
        self.frame(tag.as_bytes());
        match value {
            None => self.hasher.update([0u8]),
            Some(text) => {
                self.hasher.update([1u8]);
                self.frame(text.as_bytes());
            }
        }
    }

    fn tagged_optional_usize(&mut self, tag: &str, value: Option<usize>) {
        self.frame(tag.as_bytes());
        match value {
            None => self.hasher.update([0u8]),
            Some(number) => {
                self.hasher.update([1u8]);
                self.hasher.update((number as u64).to_be_bytes());
            }
        }
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

impl DatasetQaSeverity {
    /// Stable wire tag for the report hash and for UI labels.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
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

impl OhlcViolationKind {
    fn wire_tag(self) -> &'static str {
        match self {
            Self::HighBelowLow => "high_below_low",
            Self::HighBelowOpen => "high_below_open",
            Self::HighBelowClose => "high_below_close",
            Self::LowAboveOpen => "low_above_open",
            Self::LowAboveClose => "low_above_close",
        }
    }
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
    /// A bar exists on a non-trading weekend day under the recorded calendar.
    UnexpectedWeekendBar { weekday: String },
    /// A bar exists on a recognised US market holiday.
    UnexpectedHolidayBar { holiday: String },
    /// A bar exists inside a trading week but outside the venue's session
    /// window (currently only the xStock 24×5 cycle models one).
    UnexpectedSessionBar { window: String },
    /// The bar's excursion from the previous close is an outlier against the
    /// robust band, and the move did not hold — a bad tick, not a re-levelling.
    PriceSpike { relative_move: f64, band: f64 },
    /// Consecutive closes moved by (near) an integer ratio and the new level
    /// held on the next bar — the signature of an unapplied split.
    SuspiciousLevelShift {
        ratio_numerator: u32,
        ratio_denominator: u32,
        previous_close: f64,
        close: f64,
    },
    /// Every price in this bar equals the previous bar's close — the
    /// carried-forward synthetic bar that ADR-135 §6.11 forbids treating as
    /// data. `zero_volume` records the Alpaca `v=0` signature.
    CarryForwardBar {
        previous_index: usize,
        zero_volume: bool,
    },
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
            Self::UnexpectedWeekendBar { .. }
            | Self::UnexpectedHolidayBar { .. }
            | Self::UnexpectedSessionBar { .. }
            | Self::PriceSpike { .. }
            | Self::SuspiciousLevelShift { .. }
            | Self::CarryForwardBar { .. }
            | Self::MissingBars { .. } => DatasetQaSeverity::Warning,
        }
    }

    /// Stable wire tag for the report hash. Never derived from `Debug`.
    fn wire_tag(&self) -> &'static str {
        match self {
            Self::EmptyDataset => "empty_dataset",
            Self::UnparsableTimestamp { .. } => "unparsable_timestamp",
            Self::DuplicateTimestamp { .. } => "duplicate_timestamp",
            Self::TimestampOutOfOrder { .. } => "timestamp_out_of_order",
            Self::NonFiniteValue { .. } => "non_finite_value",
            Self::NonPositivePrice { .. } => "non_positive_price",
            Self::NegativeVolume { .. } => "negative_volume",
            Self::OhlcViolation { .. } => "ohlc_violation",
            Self::UnexpectedWeekendBar { .. } => "unexpected_weekend_bar",
            Self::UnexpectedHolidayBar { .. } => "unexpected_holiday_bar",
            Self::UnexpectedSessionBar { .. } => "unexpected_session_bar",
            Self::PriceSpike { .. } => "price_spike",
            Self::SuspiciousLevelShift { .. } => "suspicious_level_shift",
            Self::CarryForwardBar { .. } => "carry_forward_bar",
            Self::MissingBars { .. } => "missing_bars",
        }
    }

    /// Frame this issue's payload into the report digest. Every field is
    /// tagged, so no two payload shapes can produce the same byte stream.
    fn encode_into(&self, digest: &mut CanonicalDigest) {
        digest.tagged_text("issue", self.wire_tag());
        match self {
            Self::EmptyDataset => {}
            Self::UnparsableTimestamp { raw } => digest.tagged_text("raw", raw),
            Self::DuplicateTimestamp {
                previous_index,
                previous_timestamp,
            }
            | Self::TimestampOutOfOrder {
                previous_index,
                previous_timestamp,
            } => {
                digest.tagged_u64("previous_index", *previous_index as u64);
                digest.tagged_text("previous_timestamp", previous_timestamp);
            }
            Self::NonFiniteValue { field, kind } => {
                digest.tagged_text("field", field.as_str());
                digest.tagged_text("kind", kind.as_str());
            }
            Self::NonPositivePrice { field, value } => {
                digest.tagged_text("field", field.as_str());
                digest.tagged_f64("value", *value);
            }
            Self::NegativeVolume { value } => digest.tagged_f64("value", *value),
            Self::OhlcViolation { kind } => digest.tagged_text("kind", kind.wire_tag()),
            Self::UnexpectedWeekendBar { weekday } => digest.tagged_text("weekday", weekday),
            Self::UnexpectedHolidayBar { holiday } => digest.tagged_text("holiday", holiday),
            Self::UnexpectedSessionBar { window } => digest.tagged_text("window", window),
            Self::PriceSpike {
                relative_move,
                band,
            } => {
                digest.tagged_f64("relative_move", *relative_move);
                digest.tagged_f64("band", *band);
            }
            Self::SuspiciousLevelShift {
                ratio_numerator,
                ratio_denominator,
                previous_close,
                close,
            } => {
                digest.tagged_u64("ratio_numerator", *ratio_numerator as u64);
                digest.tagged_u64("ratio_denominator", *ratio_denominator as u64);
                digest.tagged_f64("previous_close", *previous_close);
                digest.tagged_f64("close", *close);
            }
            Self::CarryForwardBar {
                previous_index,
                zero_volume,
            } => {
                digest.tagged_u64("previous_index", *previous_index as u64);
                digest.tagged_u64("zero_volume", u64::from(*zero_volume));
            }
            Self::MissingBars {
                expected_next,
                missing_slots,
                scan_truncated,
            } => {
                digest.tagged_text("expected_next", expected_next);
                digest.tagged_u64("missing_slots", *missing_slots);
                digest.tagged_u64("scan_truncated", u64::from(*scan_truncated));
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
                // A session-structured calendar can adjudicate a missing *day*,
                // but not a missing 15-minute slot — that needs the session
                // hours this layer does not model.
                if calendar.can_adjudicate_step(seconds) {
                    Self::Enabled {
                        step_seconds: seconds,
                    }
                } else {
                    Self::UnsupportedForCalendar {
                        timeframe: timeframe.to_string(),
                        calendar_policy_id: calendar.policy_id().to_string(),
                    }
                }
            }
        }
    }

    fn encode_into(&self, digest: &mut CanonicalDigest) {
        match self {
            Self::Enabled { step_seconds } => {
                digest.tagged_text("gap_detection", "enabled");
                digest.tagged_u64("step_seconds", *step_seconds);
            }
            Self::UnsupportedTimeframe { timeframe } => {
                digest.tagged_text("gap_detection", "unsupported_timeframe");
                digest.tagged_text("timeframe", timeframe);
            }
            Self::VariableLengthTimeframe { timeframe } => {
                digest.tagged_text("gap_detection", "variable_length_timeframe");
                digest.tagged_text("timeframe", timeframe);
            }
            Self::UnsupportedForCalendar {
                timeframe,
                calendar_policy_id,
            } => {
                digest.tagged_text("gap_detection", "unsupported_for_calendar");
                digest.tagged_text("timeframe", timeframe);
                digest.tagged_text("calendar_policy_id", calendar_policy_id);
            }
        }
    }
}

/// Whether the robust spike band could be established, and if not, why.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpikeDetectionStatus {
    /// Ran against this band, derived from `samples` usable relative moves.
    Enabled { band: f64, samples: u64 },
    /// Fewer usable relative moves than the policy requires. No band was
    /// invented from the handful available.
    InsufficientSamples { samples: u64, required: u64 },
    /// The band could not be expressed as a finite number (a dataset whose
    /// prices overflow the arithmetic). Reported rather than clamped.
    Unavailable { reason: String },
}

impl SpikeDetectionStatus {
    fn encode_into(&self, digest: &mut CanonicalDigest) {
        match self {
            Self::Enabled { band, samples } => {
                digest.tagged_text("spike_detection", "enabled");
                digest.tagged_f64("band", *band);
                digest.tagged_u64("samples", *samples);
            }
            Self::InsufficientSamples { samples, required } => {
                digest.tagged_text("spike_detection", "insufficient_samples");
                digest.tagged_u64("samples", *samples);
                digest.tagged_u64("required", *required);
            }
            Self::Unavailable { reason } => {
                digest.tagged_text("spike_detection", "unavailable");
                digest.tagged_text("reason", reason);
            }
        }
    }
}

/// The deterministic result of a QA pass. Recomputing it over the same bars
/// and policy always yields an identical report — and therefore an identical
/// [`DatasetQaReport::report_hash`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DatasetQaReport {
    pub schema_version: u32,
    pub timeframe: String,
    pub calendar: CalendarPolicy,
    pub calendar_policy_id: String,
    pub qa_policy: DatasetQaPolicy,
    pub qa_policy_id: String,
    pub gap_detection: GapDetectionStatus,
    pub spike_detection: SpikeDetectionStatus,
    pub bars_checked: u64,
    /// Findings ordered by bar index; dataset-wide findings sort first.
    /// Findings for one bar keep a fixed check order.
    pub findings: Vec<DatasetQaFinding>,
    /// Set when the policy's `max_findings` cap was reached. The report is
    /// then a bounded sample, and says so rather than reading as complete.
    pub findings_truncated: bool,
    /// How many findings the cap dropped.
    pub findings_omitted: u64,
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

    pub fn info_count(&self) -> usize {
        self.count_at(DatasetQaSeverity::Info)
    }

    fn count_at(&self, severity: DatasetQaSeverity) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == severity)
            .count()
    }

    /// Findings attached to bars in `[offset, offset + limit)`, for a bounded
    /// inspector window. Linear in the report's (capped) finding count and
    /// allocation-bounded by `limit`-sized windows of the caller's choosing —
    /// never a scan of the bar payload.
    pub fn findings_in_range(&self, offset: u64, limit: usize) -> Vec<DatasetQaFinding> {
        let end = offset.saturating_add(limit as u64);
        self.findings
            .iter()
            .filter(|finding| match finding.bar_index {
                Some(index) => {
                    let index = index as u64;
                    index >= offset && index < end
                }
                // Dataset-wide findings belong to the first window.
                None => offset == 0,
            })
            .cloned()
            .collect()
    }

    /// Lowercase hex SHA-256 over a framed, domain-separated encoding of the
    /// whole report. Not derived from JSON: field order, float formatting, and
    /// serde attributes are all free to change without moving this hash's
    /// meaning.
    pub fn report_hash(&self) -> String {
        let mut digest = CanonicalDigest::new(DATASET_QA_DOMAIN);
        digest.tagged_u64("schema_version", self.schema_version as u64);
        digest.tagged_text("timeframe", &self.timeframe);
        digest.tagged_text("calendar_policy_id", &self.calendar_policy_id);
        self.qa_policy.encode_into(&mut digest);
        digest.tagged_text("qa_policy_id", &self.qa_policy_id);
        self.gap_detection.encode_into(&mut digest);
        self.spike_detection.encode_into(&mut digest);
        digest.tagged_u64("bars_checked", self.bars_checked);
        digest.tagged_u64("findings_truncated", u64::from(self.findings_truncated));
        digest.tagged_u64("findings_omitted", self.findings_omitted);
        digest.tagged_u64("finding_count", self.findings.len() as u64);
        for finding in &self.findings {
            digest.tagged_optional_usize("bar_index", finding.bar_index);
            digest.tagged_optional_text("timestamp", finding.timestamp.as_deref());
            digest.tagged_text("severity", finding.severity.as_str());
            finding.issue.encode_into(&mut digest);
        }
        digest.finish_hex()
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

/// Run the deterministic dataset QA pass under the default policy.
pub fn run_dataset_qa(timeframe: &str, calendar: CalendarPolicy, bars: &[Bar]) -> DatasetQaReport {
    run_dataset_qa_with_policy(timeframe, calendar, &DatasetQaPolicy::default(), bars)
}

/// A findings collector that cannot outgrow the policy's cap.
///
/// The cap is applied at push time, not by truncating a fully-materialised
/// vector, so a dataset with millions of defective bars costs `max_findings`
/// of memory rather than one finding per bar.
struct BoundedFindings {
    findings: Vec<DatasetQaFinding>,
    limit: usize,
    omitted: u64,
}

impl BoundedFindings {
    fn new(limit: usize) -> Self {
        Self {
            findings: Vec::new(),
            limit,
            omitted: 0,
        }
    }

    fn push(&mut self, bar_index: Option<usize>, timestamp: Option<&str>, issue: DatasetQaIssue) {
        if self.findings.len() >= self.limit {
            self.omitted += 1;
            return;
        }
        self.findings.push(DatasetQaFinding {
            bar_index,
            timestamp: timestamp.map(str::to_string),
            severity: issue.severity(),
            issue,
        });
    }
}

/// Run the deterministic dataset QA pass.
///
/// Every check is located (bar index + raw timestamp) and typed. Ordering,
/// weekend, spike, level-shift, carry-forward, and gap checks compare each bar
/// against the **immediately preceding parsable bar** — the pass never
/// re-sorts, because disorder is itself the defect being reported.
///
/// Scope, stated honestly: this covers structure (emptiness, ordering,
/// duplicates, OHLC invariants, impossible prices and volumes), the
/// session/weekend/holiday diagnostic under `calendar`, calendar-aware gaps
/// when the timeframe permits, robust-band price spikes, split-like level
/// shifts, and carried-forward bars. It does **not** cover corporate-action
/// correctness, cross-source agreement, or indicator repainting — those are
/// other layers' jobs (ADR-135 §7.5, §11.5).
///
/// Determinism: only IEEE-754 basic operations participate, so the report and
/// its hash reproduce across platforms.
pub fn run_dataset_qa_with_policy(
    timeframe: &str,
    calendar: CalendarPolicy,
    qa_policy: &DatasetQaPolicy,
    bars: &[Bar],
) -> DatasetQaReport {
    let gap_detection = GapDetectionStatus::resolve(timeframe, calendar);
    let granularity = timeframe_granularity(timeframe);
    let moves = relative_moves(bars);
    let spike_detection = resolve_spike_band(&moves, qa_policy);
    let spike_band = match spike_detection {
        SpikeDetectionStatus::Enabled { band, .. } => Some(band),
        _ => None,
    };

    let mut findings = BoundedFindings::new(qa_policy.max_findings as usize);

    if bars.is_empty() {
        findings.push(None, None, DatasetQaIssue::EmptyDataset);
    }

    let mut previous: Option<(usize, chrono::DateTime<chrono::Utc>, String)> = None;

    for (index, bar) in bars.iter().enumerate() {
        let instant = parse_utc(&bar.timestamp);
        let timestamp = bar.timestamp.as_str();

        match instant {
            None => findings.push(
                Some(index),
                Some(timestamp),
                DatasetQaIssue::UnparsableTimestamp {
                    raw: bar.timestamp.clone(),
                },
            ),
            Some(current) => {
                if let Some((previous_index, previous_instant, previous_timestamp)) = &previous {
                    if current == *previous_instant {
                        findings.push(
                            Some(index),
                            Some(timestamp),
                            DatasetQaIssue::DuplicateTimestamp {
                                previous_index: *previous_index,
                                previous_timestamp: previous_timestamp.clone(),
                            },
                        );
                    } else if current < *previous_instant {
                        findings.push(
                            Some(index),
                            Some(timestamp),
                            DatasetQaIssue::TimestampOutOfOrder {
                                previous_index: *previous_index,
                                previous_timestamp: previous_timestamp.clone(),
                            },
                        );
                    } else if let GapDetectionStatus::Enabled { step_seconds } = gap_detection {
                        // Gaps live in the same pass as everything else, so one
                        // cap governs the whole report.
                        if let Some(issue) = gap_issue(
                            *previous_instant,
                            current,
                            step_seconds,
                            calendar,
                            granularity,
                        ) {
                            findings.push(Some(index), Some(timestamp), issue);
                        }
                    }
                }
            }
        }

        check_bar_values(bar, &mut |issue| {
            findings.push(Some(index), Some(timestamp), issue)
        });

        if index > 0 {
            check_series_shape(bars, index, qa_policy, spike_band, &moves, &mut |issue| {
                findings.push(Some(index), Some(timestamp), issue)
            });
        }

        if let Some(current) = instant {
            match calendar.verdict_at(current, granularity) {
                CalendarVerdict::Expected => {}
                CalendarVerdict::Weekend { weekday } => findings.push(
                    Some(index),
                    Some(timestamp),
                    DatasetQaIssue::UnexpectedWeekendBar {
                        weekday: weekday.to_string(),
                    },
                ),
                CalendarVerdict::Holiday { name } => findings.push(
                    Some(index),
                    Some(timestamp),
                    DatasetQaIssue::UnexpectedHolidayBar {
                        holiday: name.to_string(),
                    },
                ),
                CalendarVerdict::OutsideSession { window } => findings.push(
                    Some(index),
                    Some(timestamp),
                    DatasetQaIssue::UnexpectedSessionBar {
                        window: window.to_string(),
                    },
                ),
            }
            previous = Some((index, current, bar.timestamp.clone()));
        }
    }

    let BoundedFindings {
        mut findings,
        limit,
        omitted,
    } = findings;
    // Stable sort: dataset-wide findings first, then bar order, with each
    // bar's checks in their fixed emission order.
    findings.sort_by_key(|finding| finding.bar_index);

    DatasetQaReport {
        schema_version: DATASET_QA_SCHEMA_VERSION,
        timeframe: timeframe.to_string(),
        calendar,
        calendar_policy_id: calendar.policy_id().to_string(),
        qa_policy: qa_policy.clone(),
        qa_policy_id: qa_policy.policy_id(),
        gap_detection,
        spike_detection,
        bars_checked: bars.len() as u64,
        findings_truncated: omitted > 0 || findings.len() > limit,
        findings_omitted: omitted,
        findings,
    }
}

/// A daily-or-coarser bar stands for a whole session; anything shorter stands
/// for an instant. An unparsable timeframe is treated as a session, the
/// conservative choice — an intraday reading would accuse every bar of being
/// out-of-session on a calendar with a window.
fn timeframe_granularity(timeframe: &str) -> CalendarGranularity {
    match parse_timeframe(timeframe) {
        Some(TimeframeStep::Fixed { seconds }) if seconds < SECONDS_PER_DAY => {
            CalendarGranularity::Intraday
        }
        _ => CalendarGranularity::Session,
    }
}

/// `|extreme − previous close| / previous close` for each bar, or `None` where
/// the comparison is undefined (first bar, non-finite or non-positive prices).
/// Uses only compare/subtract/divide, so it is bit-reproducible.
fn relative_moves(bars: &[Bar]) -> Vec<Option<f64>> {
    let mut moves = Vec::with_capacity(bars.len());
    for (index, bar) in bars.iter().enumerate() {
        if index == 0 {
            moves.push(None);
            continue;
        }
        let previous_close = bars[index - 1].close;
        let usable = previous_close.is_finite()
            && previous_close > 0.0
            && bar.high.is_finite()
            && bar.low.is_finite()
            && bar.close.is_finite();
        if !usable {
            moves.push(None);
            continue;
        }
        let deviation = (bar.high - previous_close)
            .abs()
            .max((bar.low - previous_close).abs())
            .max((bar.close - previous_close).abs());
        let relative = deviation / previous_close;
        moves.push(relative.is_finite().then_some(relative));
    }
    moves
}

/// Median of a finite slice. `values` must already be sorted ascending.
fn median_of_sorted(values: &[f64]) -> f64 {
    let middle = values.len() / 2;
    if values.len() % 2 == 1 {
        values[middle]
    } else {
        (values[middle - 1] + values[middle]) / 2.0
    }
}

/// The robust spike band: `max(median + k × MAD, policy floor)`.
fn resolve_spike_band(moves: &[Option<f64>], policy: &DatasetQaPolicy) -> SpikeDetectionStatus {
    let mut samples: Vec<f64> = moves.iter().filter_map(|value| *value).collect();
    let count = samples.len() as u64;
    if count < policy.spike_min_samples as u64 {
        return SpikeDetectionStatus::InsufficientSamples {
            samples: count,
            required: policy.spike_min_samples as u64,
        };
    }
    // `total_cmp` rather than `partial_cmp().unwrap()`: the samples are finite
    // by construction, but a subtraction below can still overflow to infinity
    // on a pathological series, and a total order sorts that deterministically
    // instead of panicking on a comparison that returned `None`.
    samples.sort_by(f64::total_cmp);
    let median = median_of_sorted(&samples);
    let mut deviations: Vec<f64> = samples.iter().map(|value| (value - median).abs()).collect();
    deviations.sort_by(f64::total_cmp);
    let mad = median_of_sorted(&deviations);
    let band = (median + policy.spike_band_multiple * mad).max(policy.spike_min_relative_move);
    if !band.is_finite() {
        return SpikeDetectionStatus::Unavailable {
            reason: "the robust band overflowed to a non-finite value".to_string(),
        };
    }
    SpikeDetectionStatus::Enabled {
        band,
        samples: count,
    }
}

/// Checks that need the previous bar: carry-forward, split-like level shift,
/// and price spike. Emission order is fixed.
fn check_series_shape(
    bars: &[Bar],
    index: usize,
    policy: &DatasetQaPolicy,
    spike_band: Option<f64>,
    moves: &[Option<f64>],
    push: &mut impl FnMut(DatasetQaIssue),
) {
    let bar = &bars[index];
    let previous = &bars[index - 1];
    let previous_close = previous.close;
    if !previous_close.is_finite() || previous_close <= 0.0 {
        return;
    }
    if !(bar.open.is_finite()
        && bar.high.is_finite()
        && bar.low.is_finite()
        && bar.close.is_finite())
    {
        return;
    }

    // Carry-forward: a flat bar pinned to the previous close.
    if bar.open == previous_close
        && bar.high == previous_close
        && bar.low == previous_close
        && bar.close == previous_close
    {
        push(DatasetQaIssue::CarryForwardBar {
            previous_index: index - 1,
            zero_volume: bar.volume == 0.0,
        });
        return;
    }

    let level_shift = classify_level_shift(bars, index, policy);
    if let Some((numerator, denominator)) = level_shift {
        push(DatasetQaIssue::SuspiciousLevelShift {
            ratio_numerator: numerator,
            ratio_denominator: denominator,
            previous_close,
            close: bar.close,
        });
        // A re-levelling is not also a bad tick.
        return;
    }

    if let (Some(band), Some(Some(relative_move))) = (spike_band, moves.get(index)) {
        if *relative_move > band {
            push(DatasetQaIssue::PriceSpike {
                relative_move: *relative_move,
                band,
            });
        }
    }
}

/// Recognise a split-like discontinuity: consecutive closes moved by (near) an
/// integer ratio `n:1` or `1:n`, with a stable level *before* the move and the
/// new level holding *after* it.
///
/// Both stability requirements matter, and each rules out one half of a bad
/// tick. Without the follow-through check the spike bar itself would look like
/// a shift; without the prior-stability check the bar where a spike *reverts*
/// would. A bar at either end of the dataset has nothing to compare against on
/// one side, so it is never classified as a shift.
fn classify_level_shift(
    bars: &[Bar],
    index: usize,
    policy: &DatasetQaPolicy,
) -> Option<(u32, u32)> {
    let previous_close = bars[index - 1].close;
    let close = bars[index].close;
    if !(previous_close.is_finite() && previous_close > 0.0 && close.is_finite() && close > 0.0) {
        return None;
    }
    let ratio = close / previous_close;
    if !ratio.is_finite() {
        return None;
    }

    let tolerance = policy.level_shift_tolerance;
    let max_ratio = policy.level_shift_max_ratio as f64;
    let candidate = if ratio >= 1.5 {
        let n = ratio.round();
        (n >= 2.0 && n <= max_ratio && (ratio / n - 1.0).abs() <= tolerance)
            .then(|| (n as u32, 1u32))
    } else if ratio <= 0.75 {
        let inverse = (1.0 / ratio).round();
        (inverse >= 2.0 && inverse <= max_ratio && (ratio * inverse - 1.0).abs() <= tolerance)
            .then(|| (1u32, inverse as u32))
    } else {
        None
    }?;

    // The level must have been stable before the move...
    let before = bars.get(index.checked_sub(2)?)?;
    if !(before.close.is_finite() && before.close > 0.0) {
        return None;
    }
    let run_up = previous_close / before.close;
    if !run_up.is_finite() || (run_up - 1.0).abs() > tolerance {
        return None;
    }

    // ...and the new level must hold on the next bar.
    let next = bars.get(index + 1)?;
    if !(next.close.is_finite() && next.close > 0.0) {
        return None;
    }
    let follow_through = next.close / close;
    if !follow_through.is_finite() || (follow_through - 1.0).abs() > tolerance {
        return None;
    }
    Some(candidate)
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

/// Absent slots between two consecutive, strictly increasing bar instants.
///
/// Counts only slots the calendar expects, so a weekday-only daily series is
/// not accused of missing every Saturday and a US-equity series is not accused
/// of missing Thanksgiving. Bounded by [`MAX_GAP_SLOTS_SCANNED`].
fn gap_issue(
    start: chrono::DateTime<chrono::Utc>,
    current: chrono::DateTime<chrono::Utc>,
    step_seconds: u64,
    calendar: CalendarPolicy,
    granularity: CalendarGranularity,
) -> Option<DatasetQaIssue> {
    let step = step_seconds as i64;
    let mut missing_slots = 0u64;
    let mut scanned = 0u64;
    let mut scan_truncated = false;
    let mut expected_next: Option<String> = None;
    let mut slot = start.timestamp();

    loop {
        let next = slot.checked_add(step)?;
        slot = next;
        if slot >= current.timestamp() {
            break;
        }
        scanned += 1;
        if scanned > MAX_GAP_SLOTS_SCANNED {
            scan_truncated = true;
            break;
        }
        let slot_instant = chrono::DateTime::from_timestamp(slot, 0)?;
        if calendar.expects_instant(slot_instant, granularity) {
            missing_slots += 1;
            if expected_next.is_none() {
                expected_next = format_utc(slot);
            }
        }
    }

    if missing_slots == 0 {
        return None;
    }
    Some(DatasetQaIssue::MissingBars {
        expected_next: expected_next?,
        missing_slots,
        scan_truncated,
    })
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
