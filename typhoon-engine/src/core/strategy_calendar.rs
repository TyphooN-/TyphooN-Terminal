//! Per-instrument execution calendars, sessions and time zones (ADR-135 §6.7).
//!
//! Internal simulator time is UTC and stays UTC. This module is the one place
//! that projects a UTC instant onto a venue's wall clock, so a session-relative
//! rule — "no entries in the first fifteen minutes" — resolves through the
//! instrument's exchange time zone with correct daylight-saving behaviour
//! instead of against a fixed offset that is wrong for half the year.
//!
//! # What it reuses, and why
//!
//! The trading-*day* rule is [`CalendarPolicy`], the same versioned four-variant
//! family the dataset layer already judges bars against (§11.5). That is
//! deliberate: a dataset whose weekend-candle QA said "this bar should not
//! exist" and a simulator that happily traded it would be two calendars
//! disagreeing under one run id. Here the policy answers "is this a trading
//! day?" and this module adds the intraday layer the dataset side deliberately
//! does not model — session windows, and the rule-based US early closes.
//!
//! # Honest limits
//!
//! - Early closes are a *rule*, not an exchange-published table: July 3, the
//!   Friday after Thanksgiving, and December 24, each when it is otherwise a
//!   trading day. Real exchanges have occasionally deviated (funeral closures,
//!   weather). The rule is identity-bearing, so a run records the assumption it
//!   was made under rather than implying an authoritative calendar.
//! - Windows are stated in exchange-local minutes of day and may not wrap past
//!   local midnight. A venue whose session crosses midnight is expressed as
//!   [`SessionRule::PolicyOnly`] plus a policy that already knows the cycle —
//!   which is exactly the xStocks 24×5 case.
//! - A single time zone per instrument. A venue that moved zones historically is
//!   not representable, and is rejected by being unrepresentable rather than
//!   approximated.

use crate::core::market_session;
use crate::core::strategy_dataset::{CalendarGranularity, CalendarPolicy, CalendarVerdict};

/// Bumped whenever the meaning of a stored calendar changes. Part of the
/// calendar id, so an old id can never be read under new semantics.
pub const TRADING_CALENDAR_SCHEMA_VERSION: u32 = 1;

/// Windows one calendar may declare. Bounded because the spec is operator input
/// that ends up in a content-addressed config.
pub const MAX_SESSION_WINDOWS: usize = 8;
/// Published exceptions one calendar may carry. Roughly a decade of US equity
/// holidays and half days, and bounded for the same reason the windows are.
pub const MAX_CALENDAR_EXCEPTIONS: usize = 4_096;
/// Longest source-record id or label an exception may seal into a calendar id.
pub const MAX_EXCEPTION_TEXT_BYTES: usize = 256;

/// Minutes in a day. A window is stated inside `[0, MINUTES_PER_DAY]`.
pub const MINUTES_PER_DAY: u32 = 24 * 60;

const NANOS_PER_SECOND: i64 = 1_000_000_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalendarError {
    NoWindows,
    TooManyWindows {
        limit: usize,
        found: usize,
    },
    WindowOutOfRange {
        index: usize,
    },
    EmptyWindow {
        index: usize,
    },
    WindowsOutOfOrder {
        index: usize,
    },
    EarlyCloseOutOfRange {
        close_minute: u32,
    },
    /// An early-close rule that no declared window contains cannot shorten
    /// anything, so it would hash into the run as a policy that does nothing.
    EarlyCloseOutsideWindows {
        close_minute: u32,
    },
    /// Exchange-local windows against a policy that has no local-day structure.
    WindowsWithoutTradingDay,
    TooManyExceptions {
        limit: usize,
        found: usize,
    },
    InvalidException {
        index: usize,
    },
    DuplicateExceptionDate {
        index: usize,
    },
    ExceptionsOutOfOrder {
        index: usize,
    },
    MissingExceptionArtifactId,
}

impl std::fmt::Display for CalendarError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoWindows => formatter.write_str("session rule declares no windows"),
            Self::TooManyWindows { limit, found } => {
                write!(
                    formatter,
                    "{found} session windows exceeds the limit of {limit}"
                )
            }
            Self::WindowOutOfRange { index } => write!(
                formatter,
                "session window {index} is outside [0, {MINUTES_PER_DAY}]"
            ),
            Self::EmptyWindow { index } => {
                write!(formatter, "session window {index} is empty")
            }
            Self::WindowsOutOfOrder { index } => write!(
                formatter,
                "session window {index} overlaps or precedes its predecessor"
            ),
            Self::EarlyCloseOutOfRange { close_minute } => write!(
                formatter,
                "early close minute {close_minute} is outside [0, {MINUTES_PER_DAY}]"
            ),
            Self::EarlyCloseOutsideWindows { close_minute } => write!(
                formatter,
                "early close minute {close_minute} shortens no declared window"
            ),
            Self::WindowsWithoutTradingDay => formatter
                .write_str("exchange-local windows require a policy with a trading-day rule"),
            Self::TooManyExceptions { limit, found } => write!(
                formatter,
                "{found} calendar exceptions exceeds the limit of {limit}"
            ),
            Self::InvalidException { index } => write!(
                formatter,
                "calendar exception {index} has invalid local-date override data or empty source identity"
            ),
            Self::DuplicateExceptionDate { index } => write!(
                formatter,
                "calendar exception {index} duplicates a local exchange date"
            ),
            Self::ExceptionsOutOfOrder { index } => write!(
                formatter,
                "calendar exception {index} overlaps, duplicates, or precedes its predecessor"
            ),
            Self::MissingExceptionArtifactId => formatter.write_str(
                "calendar exceptions and their materialized artifact id (64 lowercase hex) are present together or not at all",
            ),
        }
    }
}

impl std::error::Error for CalendarError {}

/// The wall clock a venue states its session rules in.
///
/// There is no general IANA zone here on purpose: a general zone database is a
/// data dependency whose version would have to be sealed into every run id to
/// keep old runs reproducible. These two rules are written down, versioned with
/// the schema, and cover the venues ADR-111 leaves in scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExchangeTimeZone {
    /// The venue states its sessions in UTC. Crypto venues do.
    #[default]
    Utc,
    /// US Eastern under the post-2007 rule: daylight time from 07:00 UTC on the
    /// second Sunday in March to 06:00 UTC on the first Sunday in November.
    /// Shared with the ADR-110 session chip, so the terminal and the simulator
    /// cannot disagree about what time it is at the exchange.
    UsEastern,
}

impl ExchangeTimeZone {
    /// Stable hashed identifier. Never derived from `Debug`.
    pub const fn wire_id(self) -> &'static str {
        match self {
            Self::Utc => "utc",
            Self::UsEastern => "us_eastern",
        }
    }

    /// The venue-local wall clock for a UTC instant.
    fn local(self, utc: chrono::DateTime<chrono::Utc>) -> chrono::NaiveDateTime {
        match self {
            Self::Utc => utc.naive_utc(),
            Self::UsEastern => market_session::us_eastern_datetime(utc),
        }
    }
}

/// A half-open `[start_minute, end_minute)` window in exchange-local minutes of
/// day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalSessionWindow {
    pub start_minute: u32,
    pub end_minute: u32,
}

impl LocalSessionWindow {
    pub const fn new(start_minute: u32, end_minute: u32) -> Self {
        Self {
            start_minute,
            end_minute,
        }
    }

    const fn contains(&self, minute: u32) -> bool {
        minute >= self.start_minute && minute < self.end_minute
    }
}

/// Which early closes a calendar models.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case", tag = "rule")]
pub enum EarlyCloseRule {
    /// No early closes. Loud rather than silent: a run says it assumed full
    /// sessions on half days.
    #[default]
    None,
    /// The three rule-based US equity early closes — July 3, the Friday after
    /// Thanksgiving, and December 24 — each shortened to `close_minute` local.
    /// A date only counts when the policy already calls it a trading day.
    UsEquity { close_minute: u32 },
}

impl EarlyCloseRule {
    pub const fn wire_id(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::UsEquity { .. } => "us_equity",
        }
    }

    /// The local minute trading stops on `date`, when this rule shortens it.
    fn close_minute_on(self, date: chrono::NaiveDate) -> Option<u32> {
        use chrono::{Datelike, Weekday};
        let Self::UsEquity { close_minute } = self else {
            return None;
        };
        let year = date.year();
        // July 3, but only when Independence Day is actually observed on the
        // 4th — a July 4 that lands on a weekend moves the holiday, and the
        // early close moves with it rather than stranding on the 3rd.
        if date.month() == 7
            && date.day() == 3
            && chrono::NaiveDate::from_ymd_opt(year, 7, 4)
                .is_some_and(|fourth| !matches!(fourth.weekday(), Weekday::Sat | Weekday::Sun))
        {
            return Some(close_minute);
        }
        if date.month() == 12 && date.day() == 24 {
            return Some(close_minute);
        }
        // The Friday after the fourth Thursday in November.
        if fourth_thursday_of_november(year)
            .and_then(|day| day.checked_add_signed(chrono::Duration::days(1)))
            == Some(date)
        {
            return Some(close_minute);
        }
        None
    }
}

/// Thanksgiving Day: the fourth Thursday in November.
fn fourth_thursday_of_november(year: i32) -> Option<chrono::NaiveDate> {
    use chrono::{Datelike, Weekday};
    let first = chrono::NaiveDate::from_ymd_opt(year, 11, 1)?;
    let offset = (7 + Weekday::Thu.num_days_from_monday() as i64
        - first.weekday().num_days_from_monday() as i64)
        % 7;
    first.checked_add_signed(chrono::Duration::days(offset + 21))
}

/// How the intraday layer restricts a day the policy already calls tradable.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SessionRule {
    /// The policy alone decides. Correct for a continuous venue, and for the
    /// xStocks 24×5 cycle, whose intraday structure the policy already answers.
    PolicyOnly,
    /// Trading is confined to `windows`, stated in the calendar's exchange-local
    /// clock, on any day the policy calls tradable.
    LocalWindows {
        windows: Vec<LocalSessionWindow>,
        early_close: EarlyCloseRule,
    },
}

impl SessionRule {
    pub const fn wire_id(&self) -> &'static str {
        match self {
            Self::PolicyOnly => "policy_only",
            Self::LocalWindows { .. } => "local_windows",
        }
    }
}

/// Everything a calendar is, before it is sealed with its id.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TradingCalendarSpec {
    /// Trading-day rule, shared with the dataset QA layer (§11.5).
    pub policy: CalendarPolicy,
    pub time_zone: ExchangeTimeZone,
    pub session: SessionRule,
    #[serde(default)]
    pub exceptions: Vec<CalendarException>,
    #[serde(default)]
    pub exception_artifact_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalendarException {
    /// Exchange-local civil date; no UTC offset is baked into the exception.
    pub local_date: chrono::NaiveDate,
    pub source_record_id: String,
    pub label: String,
    pub kind: CalendarExceptionKind,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CalendarExceptionKind {
    Closed,
    /// Published windows for this date. This represents both early closes and
    /// open overrides and intentionally bypasses the rule-only day verdict.
    SessionOverride {
        windows: Vec<LocalSessionWindow>,
    },
}

impl TradingCalendarSpec {
    /// A crypto venue: every instant trades, and no window applies.
    pub fn continuous() -> Self {
        Self {
            policy: CalendarPolicy::Continuous24x7,
            time_zone: ExchangeTimeZone::Utc,
            session: SessionRule::PolicyOnly,
            exceptions: Vec::new(),
            exception_artifact_id: None,
        }
    }

    /// US equities, regular session only (09:30–16:00 ET), with the rule-based
    /// 13:00 ET early closes.
    pub fn us_equity_regular() -> Self {
        Self {
            policy: CalendarPolicy::UsEquityRegular,
            time_zone: ExchangeTimeZone::UsEastern,
            session: SessionRule::LocalWindows {
                windows: vec![LocalSessionWindow::new(9 * 60 + 30, 16 * 60)],
                early_close: EarlyCloseRule::UsEquity {
                    close_minute: 13 * 60,
                },
            },
            exceptions: Vec::new(),
            exception_artifact_id: None,
        }
    }

    /// Kraken tokenized equities: the 24×5 cycle, whose structure the policy
    /// already carries, so no intraday window is layered on top.
    pub fn xstock_24x5() -> Self {
        Self {
            policy: CalendarPolicy::XStock24x5,
            time_zone: ExchangeTimeZone::UsEastern,
            session: SessionRule::PolicyOnly,
            exceptions: Vec::new(),
            exception_artifact_id: None,
        }
    }
}

/// Why the venue is not accepting trades at an instant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClosedReason {
    Weekend,
    /// The *rule* says the venue is shut. Derived, not published.
    Holiday {
        name: &'static str,
    },
    /// A materialized calendar-exception artifact says the venue published a
    /// full closure on this exchange-local date. Distinct from [`Self::Holiday`]
    /// so a report never presents a derived guess as an exchange statement.
    PublishedClosure,
    /// Inside a trading day but outside every declared window.
    OutsideWindow,
    /// Inside a window that an early close shortened past this instant.
    EarlyClose {
        close_minute: u32,
    },
}

impl ClosedReason {
    pub const fn wire_id(self) -> &'static str {
        match self {
            Self::Weekend => "weekend",
            Self::Holiday { .. } => "holiday",
            Self::PublishedClosure => "published_closure",
            Self::OutsideWindow => "outside_window",
            Self::EarlyClose { .. } => "early_close",
        }
    }
}

/// What the venue is doing at an instant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionStatus {
    Open {
        /// Minutes since the containing window opened, in exchange-local time.
        /// This is what a session-relative rule counts, and it is DST-correct
        /// because the projection to local time is.
        minutes_since_open: u32,
        /// Whether an early-close rule shortened the day this instant is in.
        early_close: bool,
    },
    Closed(ClosedReason),
}

impl SessionStatus {
    pub const fn is_open(self) -> bool {
        matches!(self, Self::Open { .. })
    }

    pub const fn closed_reason(self) -> Option<ClosedReason> {
        match self {
            Self::Closed(reason) => Some(reason),
            Self::Open { .. } => None,
        }
    }
}

/// A validated calendar and its content-addressed id.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct TradingCalendar {
    schema_version: u32,
    spec: TradingCalendarSpec,
    calendar_id: String,
}

impl TradingCalendar {
    /// Validate `spec` and seal it with its id.
    pub fn build(spec: &TradingCalendarSpec) -> Result<Self, CalendarError> {
        validate_spec(spec)?;
        Ok(Self {
            schema_version: TRADING_CALENDAR_SCHEMA_VERSION,
            spec: spec.clone(),
            calendar_id: calendar_id(spec),
        })
    }

    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub const fn spec(&self) -> &TradingCalendarSpec {
        &self.spec
    }

    /// `cal.v<schema>.<policy>:<16 hex>` — versioned, self-describing, and
    /// different for every distinct assumption.
    pub fn calendar_id(&self) -> &str {
        &self.calendar_id
    }

    /// Adjudicate one UTC nanosecond instant.
    pub fn status_at_ns(&self, utc_ns: i64) -> SessionStatus {
        let Some(utc) = utc_from_nanos(utc_ns) else {
            // An instant outside the representable calendar range cannot be
            // placed in a session, so it is not tradable. Refusing is the only
            // answer that cannot invent a fill.
            return SessionStatus::Closed(ClosedReason::OutsideWindow);
        };
        self.status_at(utc)
    }

    /// The local minute a regular day stops, when the rule declares windows.
    /// `None` for a policy-only calendar, which has no rule-declared close to
    /// call an override "early" against.
    fn regular_close_minute(&self) -> Option<u32> {
        match &self.spec.session {
            SessionRule::PolicyOnly => None,
            SessionRule::LocalWindows { windows, .. } => windows.last().map(|w| w.end_minute),
        }
    }

    pub fn status_at(&self, utc: chrono::DateTime<chrono::Utc>) -> SessionStatus {
        let local = self.spec.time_zone.local(utc);
        // Exceptions are validated unique and ascending by local date, so this
        // is a binary search rather than the linear scan a per-bar hot path
        // cannot afford at the 4096-exception bound.
        if let Ok(index) = self
            .spec
            .exceptions
            .binary_search_by(|candidate| candidate.local_date.cmp(&local.date()))
        {
            // A published exception is the exchange's own statement about this
            // date and outranks every derived rule below, including the policy's
            // weekend and holiday verdicts.
            return match &self.spec.exceptions[index].kind {
                CalendarExceptionKind::Closed => {
                    SessionStatus::Closed(ClosedReason::PublishedClosure)
                }
                CalendarExceptionKind::SessionOverride { windows } => {
                    status_in_windows(local, windows, self.regular_close_minute())
                }
            };
        }
        let granularity = match self.spec.session {
            // With no intraday layer the policy is asked the intraday question
            // directly — that is what carries the xStocks 24×5 cycle.
            SessionRule::PolicyOnly => CalendarGranularity::Intraday,
            // With windows, the policy decides only the *day*; the windows
            // decide the time, so the policy must not also apply its own
            // intraday rule and reject a morning it would call out-of-session.
            SessionRule::LocalWindows { .. } => CalendarGranularity::Session,
        };
        match self.spec.policy.verdict_at(utc, granularity) {
            CalendarVerdict::Weekend { .. } => return SessionStatus::Closed(ClosedReason::Weekend),
            CalendarVerdict::Holiday { name } => {
                return SessionStatus::Closed(ClosedReason::Holiday { name });
            }
            CalendarVerdict::OutsideSession { .. } => {
                return SessionStatus::Closed(ClosedReason::OutsideWindow);
            }
            CalendarVerdict::Expected => {}
        }

        let SessionRule::LocalWindows {
            windows,
            early_close,
        } = &self.spec.session
        else {
            return SessionStatus::Open {
                minutes_since_open: 0,
                early_close: false,
            };
        };

        let minute = minute_of_day(local);
        let shortened = early_close.close_minute_on(local.date());
        let Some(window) = windows.iter().find(|window| window.contains(minute)) else {
            return SessionStatus::Closed(ClosedReason::OutsideWindow);
        };
        if let Some(close_minute) = shortened
            && minute >= close_minute
        {
            return SessionStatus::Closed(ClosedReason::EarlyClose { close_minute });
        }
        SessionStatus::Open {
            minutes_since_open: minute - window.start_minute,
            early_close: shortened.is_some(),
        }
    }

    pub fn is_open_at_ns(&self, utc_ns: i64) -> bool {
        self.status_at_ns(utc_ns).is_open()
    }
}

/// Adjudicate an instant against the windows a published exception declares.
///
/// `regular_close` is the rule's ordinary close, used only to answer whether
/// this published day is genuinely shorter than a regular one. An override that
/// runs to the usual bell is not an early close and must not claim to be.
fn status_in_windows(
    local: chrono::NaiveDateTime,
    windows: &[LocalSessionWindow],
    regular_close: Option<u32>,
) -> SessionStatus {
    let minute = minute_of_day(local);
    let Some(window) = windows.iter().find(|window| window.contains(minute)) else {
        return SessionStatus::Closed(ClosedReason::OutsideWindow);
    };
    let published_close = windows.last().map(|last| last.end_minute);
    SessionStatus::Open {
        minutes_since_open: minute - window.start_minute,
        early_close: match (regular_close, published_close) {
            (Some(regular), Some(published)) => published < regular,
            _ => false,
        },
    }
}

/// Truncate `windows` at `close_minute`, dropping windows that start at or
/// after it. This is how a published early close becomes an explicit session
/// override for *any* venue, instead of assuming one venue's opening bell.
///
/// `None` when nothing survives — an "early close" at or before the day's first
/// open is a closure, and the caller must say so rather than seal an empty
/// session that would silently read as a normal day.
pub fn shorten_windows(
    windows: &[LocalSessionWindow],
    close_minute: u32,
) -> Option<Vec<LocalSessionWindow>> {
    let shortened: Vec<_> = windows
        .iter()
        .filter(|window| window.start_minute < close_minute)
        .map(|window| {
            LocalSessionWindow::new(window.start_minute, window.end_minute.min(close_minute))
        })
        .collect();
    (!shortened.is_empty()).then_some(shortened)
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct TradingCalendarWire {
    schema_version: u32,
    spec: TradingCalendarSpec,
    calendar_id: String,
}

impl<'de> serde::Deserialize<'de> for TradingCalendar {
    /// A stored calendar is re-validated and its id recomputed on the way in.
    /// A hand-edited window that kept the old id is rejected here rather than
    /// silently changing what a sealed run traded under.
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error as _;
        let wire = TradingCalendarWire::deserialize(deserializer)?;
        if wire.schema_version != TRADING_CALENDAR_SCHEMA_VERSION {
            return Err(D::Error::custom(format!(
                "trading calendar schema version {} is not supported (this build reads {TRADING_CALENDAR_SCHEMA_VERSION})",
                wire.schema_version
            )));
        }
        let rebuilt = Self::build(&wire.spec).map_err(D::Error::custom)?;
        if rebuilt.calendar_id != wire.calendar_id {
            return Err(D::Error::custom(format!(
                "trading calendar id mismatch: sealed {}, recomputed {}",
                wire.calendar_id, rebuilt.calendar_id
            )));
        }
        Ok(rebuilt)
    }
}

fn minute_of_day(local: chrono::NaiveDateTime) -> u32 {
    use chrono::Timelike;
    local.hour() * 60 + local.minute()
}

fn utc_from_nanos(utc_ns: i64) -> Option<chrono::DateTime<chrono::Utc>> {
    let seconds = utc_ns.div_euclid(NANOS_PER_SECOND);
    let nanos = utc_ns.rem_euclid(NANOS_PER_SECOND) as u32;
    chrono::DateTime::from_timestamp(seconds, nanos)
}

fn validate_spec(spec: &TradingCalendarSpec) -> Result<(), CalendarError> {
    if spec.exceptions.len() > MAX_CALENDAR_EXCEPTIONS {
        return Err(CalendarError::TooManyExceptions {
            limit: MAX_CALENDAR_EXCEPTIONS,
            found: spec.exceptions.len(),
        });
    }
    // An exception set is only ever produced by materializing a source batch,
    // so it must name the artifact it came from. Without that, a calendar could
    // claim published closures with no provenance to check them against.
    if spec.exceptions.is_empty() != spec.exception_artifact_id.is_none() {
        return Err(CalendarError::MissingExceptionArtifactId);
    }
    if spec.exception_artifact_id.as_deref().is_some_and(|id| {
        id.len() != 64
            || !id
                .bytes()
                .all(|b| b.is_ascii_digit() || b.is_ascii_lowercase() && b <= b'f')
    }) {
        return Err(CalendarError::MissingExceptionArtifactId);
    }
    for (index, exception) in spec.exceptions.iter().enumerate() {
        if !is_bounded_identity(&exception.source_record_id)
            || !is_bounded_identity(&exception.label)
        {
            return Err(CalendarError::InvalidException { index });
        }
        if let CalendarExceptionKind::SessionOverride { windows } = &exception.kind {
            validate_exception_windows(windows, index)?;
        }
        if index > 0 && spec.exceptions[index - 1].local_date > exception.local_date {
            return Err(CalendarError::ExceptionsOutOfOrder { index });
        }
        if index > 0 && spec.exceptions[index - 1].local_date == exception.local_date {
            return Err(CalendarError::DuplicateExceptionDate { index });
        }
    }
    let SessionRule::LocalWindows {
        windows,
        early_close,
    } = &spec.session
    else {
        return Ok(());
    };
    if spec.policy == CalendarPolicy::Continuous24x7 {
        // A continuous venue has no local trading day to hang a window on; the
        // combination would silently mean "trade 09:30–16:00 every day of the
        // year including Christmas", which nobody asked for.
        return Err(CalendarError::WindowsWithoutTradingDay);
    }
    if windows.is_empty() {
        return Err(CalendarError::NoWindows);
    }
    if windows.len() > MAX_SESSION_WINDOWS {
        return Err(CalendarError::TooManyWindows {
            limit: MAX_SESSION_WINDOWS,
            found: windows.len(),
        });
    }
    for (index, window) in windows.iter().enumerate() {
        if window.start_minute > MINUTES_PER_DAY || window.end_minute > MINUTES_PER_DAY {
            return Err(CalendarError::WindowOutOfRange { index });
        }
        if window.end_minute <= window.start_minute {
            return Err(CalendarError::EmptyWindow { index });
        }
        if index > 0 && window.start_minute < windows[index - 1].end_minute {
            return Err(CalendarError::WindowsOutOfOrder { index });
        }
    }
    if let EarlyCloseRule::UsEquity { close_minute } = *early_close {
        if close_minute > MINUTES_PER_DAY {
            return Err(CalendarError::EarlyCloseOutOfRange { close_minute });
        }
        if !windows
            .iter()
            .any(|window| window.contains(close_minute) || close_minute == window.start_minute)
        {
            return Err(CalendarError::EarlyCloseOutsideWindows { close_minute });
        }
    }
    Ok(())
}

/// Text that may be sealed into a calendar id: present, trimmed, printable and
/// bounded. Unbounded operator text in a content-addressed artifact is how a
/// config id becomes a place to hide a payload.
fn is_bounded_identity(text: &str) -> bool {
    !text.is_empty()
        && text.trim() == text
        && text.len() <= MAX_EXCEPTION_TEXT_BYTES
        && !text.chars().any(char::is_control)
}

fn validate_exception_windows(
    windows: &[LocalSessionWindow],
    index: usize,
) -> Result<(), CalendarError> {
    if windows.is_empty() || windows.len() > MAX_SESSION_WINDOWS {
        return Err(CalendarError::InvalidException { index });
    }
    for (window_index, window) in windows.iter().enumerate() {
        if window.start_minute >= window.end_minute || window.end_minute > MINUTES_PER_DAY {
            return Err(CalendarError::InvalidException { index });
        }
        if window_index > 0 && windows[window_index - 1].end_minute > window.start_minute {
            return Err(CalendarError::InvalidException { index });
        }
    }
    Ok(())
}

fn calendar_id(spec: &TradingCalendarSpec) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    let mut frame = |bytes: &[u8]| {
        hasher.update((bytes.len() as u64).to_be_bytes());
        hasher.update(bytes);
    };
    frame(b"typhoon.strategy.calendar.v1");
    frame(&TRADING_CALENDAR_SCHEMA_VERSION.to_be_bytes());
    frame(spec.policy.policy_id().as_bytes());
    frame(spec.time_zone.wire_id().as_bytes());
    frame(spec.session.wire_id().as_bytes());
    if let SessionRule::LocalWindows {
        windows,
        early_close,
    } = &spec.session
    {
        frame(&(windows.len() as u64).to_be_bytes());
        for window in windows {
            frame(&window.start_minute.to_be_bytes());
            frame(&window.end_minute.to_be_bytes());
        }
        frame(early_close.wire_id().as_bytes());
        if let EarlyCloseRule::UsEquity { close_minute } = *early_close {
            frame(&close_minute.to_be_bytes());
        }
    }
    frame(
        spec.exception_artifact_id
            .as_deref()
            .unwrap_or("")
            .as_bytes(),
    );
    frame(&(spec.exceptions.len() as u64).to_be_bytes());
    for exception in &spec.exceptions {
        frame(exception.local_date.to_string().as_bytes());
        frame(exception.source_record_id.as_bytes());
        frame(exception.label.as_bytes());
        match &exception.kind {
            CalendarExceptionKind::Closed => frame(b"closed"),
            CalendarExceptionKind::SessionOverride { windows } => {
                frame(b"session_override");
                frame(&(windows.len() as u64).to_be_bytes());
                for window in windows {
                    frame(&window.start_minute.to_be_bytes());
                    frame(&window.end_minute.to_be_bytes());
                }
            }
        }
    }
    let hash = hasher.finalize();
    let hex: String = hash.iter().map(|byte| format!("{byte:02x}")).collect();
    format!(
        "cal.v{TRADING_CALENDAR_SCHEMA_VERSION}.{}:{}",
        spec.session.wire_id(),
        &hex[..16]
    )
}

#[cfg(test)]
mod tests;
