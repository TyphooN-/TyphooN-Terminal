//! Deterministic scalar strategy simulator (ADR-135 §5.3, §6.1–§6.5, §6.9–§6.13).
//!
//! One time-ordered event queue drives every symbol. Nothing in here reads a
//! clock, draws from a system RNG, or depends on the order a container happened
//! to iterate in: the same inputs produce the same ledger, byte for byte, on
//! any thread.
//!
//! ## The total order
//!
//! Every event carries `(time_ns, priority, sequence)` and the log is strictly
//! increasing in that tuple. `priority` is explicit rather than derived from
//! the event kind, because the same kind occurs in two different phases of a
//! bar and the phase is what orders them:
//!
//! | priority | what happens |
//! |---|---|
//! | 0  | `BarOpen` — the bar's open price becomes the current reference |
//! | 1  | `OrderActivate` — an order reaches the exchange |
//! | 2  | `OrderCancel` / `OrderModify` — a request takes effect |
//! | 3  | `OrderExpire` — a time-in-force runs out |
//! | 4  | `StopTriggered`, `Fill` — execution against the open or the intrabar path |
//! | 5  | `BarClose` — the bar commits and becomes visible history |
//! | 6  | `Fill` — execution at the close (market-on-close, bar-close fidelity) |
//! | 7  | `OrderCancel` — an OCO sibling withdrawn because its partner filled |
//! | 8  | `Decision` — the strategy runs |
//! | 9  | `OrderSubmit` — an intent becomes an order |
//! | 10 | `OrderReject` — an order or request is refused |
//! | 11 | `MarkToMarket` — equity is sampled |
//!
//! Ties inside one `(time, priority)` resolve by a scheduling counter assigned
//! deterministically: bar events by symbol-table order, order events by
//! submission order.
//!
//! ## What a decision may see
//!
//! [`MarketView`] answers only from *committed* bars — those whose `BarClose`
//! has already been processed. `bars_ago` is `usize`, so a future observation
//! is not expressible, and asking past the committed history returns
//! [`MarketDataError::FutureData`] rather than a number. The view hands out no
//! slice, no length and no iterator, so "scan the whole series" has no spelling
//! either.
//!
//! At a [`DecisionPoint::PreClose`] decision the bar in progress is visible
//! through [`FormingBar`], which carries its open and how long it has been
//! forming — and no high, low or close, because those have not printed yet.
//!
//! ## Execution model
//!
//! An order is eligible for a bar only when it was already live at that bar's
//! **open** (`active_time_ns ≤ bar.open_time_ns`). An order that arrives while
//! a bar is already trading cannot claim that bar's range, because nobody knows
//! whether the extreme it wants happened before it got there.
//!
//! Bar prices are treated as the mid series. Buys lift the ask
//! (`mid + half-spread`), sells hit the bid, and marketable orders — market,
//! market-on-close, and triggered stops — additionally pay slippage. A limit
//! order never fills worse than its limit; slippage cannot push it through.
//! The half-spread of a bar is derived from that bar's open, except for
//! market-on-close, which derives it from the close it executes at.
//!
//! Triggers resolve per [`FidelityLevel`]:
//!
//! - `BarClose` — only the open and the close are execution prices. A resting
//!   order is tested against each, and an extreme reached inside the bar is
//!   invisible, because at this fidelity the engine does not claim to know it.
//! - `BarOhlc` — the bar's range resolves triggers, an order gapped through at
//!   the open fills at the open, and two levels reachable in the same bar
//!   resolve under the [`OhlcAmbiguityPolicy`].
//!
//! A level the bar *gapped through at its open* always resolves before the
//! ambiguity policy is consulted: the first observable price of the bar already
//! went through it, so no assumption about the path is needed.
//!
//! ## Rejections
//!
//! An order that cannot be executed is refused and recorded — warm-up not
//! complete, price off the tick lattice, reduce-only that would not reduce,
//! insufficient buying power, a short in a cash account, a bar-delayed
//! submission that ran off the end of the stream, a request naming an order
//! that no longer exists. None of these is a silent drop.

use crate::core::strategy_interpreter::{CanonicalIrStrategy, InterpreterError};
use crate::core::strategy_ir::{
    CommissionModel, DecisionTiming, ExecutionCompatibility, FeeSide, FidelityLevel, LatencyModel,
    MAX_PRE_CLOSE_OFFSET_SECONDS, MAX_SUBMIT_DELAY_BARS, MarginPolicy, OhlcAmbiguityPolicy,
    SlippageModel, SpreadModel, StrategyExecutionConfig, StrategyIrError, TieBreakPolicy,
};
use crate::core::strategy_run::VerifiedRun;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;

pub const MAX_SYMBOLS: usize = 256;
pub const MAX_SYMBOL_LEN: usize = 64;
pub const MAX_BARS_PER_SYMBOL: usize = 100_000;
/// Global bound protects the prebuilt clock, event ledger, and equity series.
pub const MAX_TOTAL_BARS: usize = 250_000;
pub const MAX_INTENTS_PER_DECISION: usize = 1_024;
pub const MAX_ORDER_QUANTITY: f64 = 1.0e12;
/// Orders one run may create, across every symbol and the whole run.
pub const MAX_TOTAL_ORDERS: usize = 1_000_000;
/// Orders that may rest on one symbol at once.
pub const MAX_LIVE_ORDERS_PER_SYMBOL: usize = 4_096;
/// Events one run may record.
pub const MAX_EVENTS: usize = 8_000_000;

const NANOS_PER_DAY: i64 = 86_400_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SymbolId(pub usize);

/// The identity a strategy is handed when it submits an order, and the handle
/// it uses to cancel or modify that order later. Ids are assigned in
/// submission order from zero, so they are reproducible across runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ClientOrderId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SimBar {
    pub open_time_ns: i64,
    pub close_time_ns: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SymbolStream {
    pub symbol: String,
    pub bars: Vec<SimBar>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderSide {
    Buy,
    Sell,
}

impl OrderSide {
    const fn sign(self) -> f64 {
        match self {
            Self::Buy => 1.0,
            Self::Sell => -1.0,
        }
    }

    const fn fee_side(self) -> FeeSide {
        match self {
            Self::Buy => FeeSide::Buy,
            Self::Sell => FeeSide::Sell,
        }
    }
}

// ── Order model (§6.5) ─────────────────────────────────────────────

/// The order types the simulator executes. Partial fills and liquidity caps
/// are §6.6 work and are deliberately absent rather than approximated.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderKind {
    Market,
    Limit {
        limit_price: f64,
    },
    Stop {
        stop_price: f64,
    },
    /// A stop that becomes a limit once triggered. If the trigger price is not
    /// inside the limit, the order rests as a plain limit from the next bar:
    /// the path inside the trigger bar is unknown, so nothing is assumed.
    StopLimit {
        stop_price: f64,
        limit_price: f64,
    },
    MarketOnClose,
}

impl OrderKind {
    const fn is_stop_like(self) -> bool {
        matches!(self, Self::Stop { .. } | Self::StopLimit { .. })
    }

    const fn is_limit_like(self) -> bool {
        matches!(self, Self::Limit { .. })
    }
}

/// How long an unfilled order stays live.
///
/// `Day` is resolved against the **UTC calendar day**, not an exchange
/// session: per-instrument trading calendars are §6.7/M2 work, and a UTC day
/// is a rule that can be stated exactly rather than guessed at.
///
/// With no partial fills in this milestone, `Fok` and `Ioc` behave alike — the
/// whole quantity either executes at the first opportunity or the order is
/// cancelled. They stay distinct because §6.6 will separate them.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeInForce {
    Ioc,
    Fok,
    Day,
    Gtc,
    Gtd { expire_time_ns: i64 },
}

/// What a strategy asks for. Construct through the helpers so the defaults
/// (good-til-cancelled, not reduce-only, no bracket) are explicit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderRequest {
    pub symbol: SymbolId,
    pub side: OrderSide,
    pub quantity: f64,
    pub kind: OrderKind,
    pub time_in_force: TimeInForce,
    /// May only reduce an existing position; rejected if it would open, grow,
    /// or flip one.
    pub reduce_only: bool,
    /// Bracket tag. When one order in a group fills, every live sibling is
    /// cancelled.
    pub oco_group: Option<u32>,
}

impl OrderRequest {
    fn of(symbol: SymbolId, side: OrderSide, quantity: f64, kind: OrderKind) -> Self {
        Self {
            symbol,
            side,
            quantity,
            kind,
            time_in_force: TimeInForce::Gtc,
            reduce_only: false,
            oco_group: None,
        }
    }

    pub fn market(symbol: SymbolId, side: OrderSide, quantity: f64) -> Self {
        Self::of(symbol, side, quantity, OrderKind::Market)
    }

    pub fn limit(symbol: SymbolId, side: OrderSide, quantity: f64, limit_price: f64) -> Self {
        Self::of(symbol, side, quantity, OrderKind::Limit { limit_price })
    }

    pub fn stop(symbol: SymbolId, side: OrderSide, quantity: f64, stop_price: f64) -> Self {
        Self::of(symbol, side, quantity, OrderKind::Stop { stop_price })
    }

    pub fn stop_limit(
        symbol: SymbolId,
        side: OrderSide,
        quantity: f64,
        stop_price: f64,
        limit_price: f64,
    ) -> Self {
        Self::of(
            symbol,
            side,
            quantity,
            OrderKind::StopLimit {
                stop_price,
                limit_price,
            },
        )
    }

    pub fn market_on_close(symbol: SymbolId, side: OrderSide, quantity: f64) -> Self {
        Self::of(symbol, side, quantity, OrderKind::MarketOnClose)
    }

    #[must_use]
    pub fn with_tif(mut self, time_in_force: TimeInForce) -> Self {
        self.time_in_force = time_in_force;
        self
    }

    #[must_use]
    pub fn with_oco(mut self, group: u32) -> Self {
        self.oco_group = Some(group);
        self
    }

    #[must_use]
    pub fn reduce_only(mut self) -> Self {
        self.reduce_only = true;
        self
    }
}

/// A change to a resting order. Every field is optional; the ones that are set
/// must apply to the order's kind, otherwise the request is rejected rather
/// than partially applied.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct ModifyRequest {
    pub quantity: Option<f64>,
    pub limit_price: Option<f64>,
    pub stop_price: Option<f64>,
}

impl ModifyRequest {
    pub fn quantity(quantity: f64) -> Self {
        Self {
            quantity: Some(quantity),
            ..Self::default()
        }
    }
    pub fn limit_price(limit_price: f64) -> Self {
        Self {
            limit_price: Some(limit_price),
            ..Self::default()
        }
    }
    pub fn stop_price(stop_price: f64) -> Self {
        Self {
            stop_price: Some(stop_price),
            ..Self::default()
        }
    }
}

// ── Decision timing (§6.13) ────────────────────────────────────────

/// When the strategy is asked to decide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionPoint {
    /// At each bar's close, seeing that bar.
    #[default]
    ClosedBar,
    /// At each bar's open, seeing every previous bar and this bar's open — but
    /// not trading that open, which has already printed.
    NextBarOpen,
    /// A fixed offset before each bar closes, seeing the forming bar's open and
    /// nothing else about it.
    PreClose { offset_ns: i64 },
}

/// The bar in progress at a pre-close decision.
///
/// There is deliberately no `high`, `low` or `close` here. At bar resolution
/// the only thing known about a bar that has not finished is where it started
/// and how long it has been running; a field for the values it has not printed
/// yet would be a field for the future.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FormingBar {
    pub open_time_ns: i64,
    pub open: f64,
    pub elapsed_ns: i64,
}

/// The run-level knobs that come from the strategy rather than the cost model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimulationSetup {
    /// Root seed for every derived RNG stream (§6.10).
    pub seed: u64,
    pub decision_point: DecisionPoint,
    /// Bars between a decision and the submission it produced.
    pub submit_delay_bars: u32,
}

impl Default for SimulationSetup {
    fn default() -> Self {
        Self {
            seed: 0,
            decision_point: DecisionPoint::ClosedBar,
            submit_delay_bars: 0,
        }
    }
}

impl SimulationSetup {
    /// Derive every simulator-affecting run knob from verified,
    /// identity-bearing artifacts. Identified runs must use this path rather
    /// than accepting an unrelated mutable setup beside the manifest.
    pub fn from_verified_run(run: &VerifiedRun<'_>) -> Self {
        let timing = run.strategy().definition().timing;
        let decision_point = match timing.decision {
            DecisionTiming::ClosedBar => DecisionPoint::ClosedBar,
            DecisionTiming::NextBarOpen => DecisionPoint::NextBarOpen,
            DecisionTiming::PreClose { offset_seconds } => DecisionPoint::PreClose {
                offset_ns: i64::from(offset_seconds) * 1_000_000_000,
            },
        };
        Self {
            seed: run.manifest().binding().seed,
            decision_point,
            submit_delay_bars: timing.submit_delay_bars,
        }
    }
}

// ── Ledger records ─────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SimEventKind {
    BarOpen,
    OrderActivate,
    OrderCancel,
    OrderModify,
    OrderExpire,
    StopTriggered,
    Fill,
    BarClose,
    Decision,
    OrderSubmit,
    OrderReject,
    MarkToMarket,
}

/// Explicit ordering slots. The same kind can occur in two phases of a bar, so
/// the phase — not the kind — decides the order.
mod priority {
    pub const BAR_OPEN: u8 = 0;
    pub const ACTIVATE: u8 = 1;
    pub const REQUEST: u8 = 2;
    pub const EXPIRE: u8 = 3;
    pub const OPEN_EXECUTION: u8 = 4;
    pub const BAR_CLOSE: u8 = 5;
    pub const CLOSE_EXECUTION: u8 = 6;
    pub const OCO_CANCEL: u8 = 7;
    pub const DECISION: u8 = 8;
    pub const SUBMIT: u8 = 9;
    pub const REJECT: u8 = 10;
    pub const MARK: u8 = 11;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimEventRecord {
    pub time_ns: i64,
    pub kind: SimEventKind,
    /// Ordering slot within a timestamp; see the module table.
    pub priority: u8,
    pub sequence: u64,
    pub symbol: Option<SymbolId>,
    pub order_id: Option<ClientOrderId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FillRecord {
    pub order_id: ClientOrderId,
    pub time_ns: i64,
    pub sequence: u64,
    pub symbol: SymbolId,
    pub side: OrderSide,
    pub quantity: f64,
    /// The mid the execution was resolved against.
    pub reference_price: f64,
    /// The reference plus the half-spread on the taker's side.
    pub quoted_price: f64,
    pub fill_price: f64,
    pub spread_cost: f64,
    pub slippage_cost: f64,
    pub commission: f64,
    pub realized_pnl: f64,
    pub cash_after: f64,
    pub position_units_after: f64,
    pub avg_entry_after: f64,
}

/// Why an order or request was refused.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectionReason {
    /// The symbol has not closed enough bars yet (§6.9 warm-up).
    WarmupIncomplete { committed: usize, required: u32 },
    /// A limit or stop price is not on the configured tick lattice.
    PriceOffTick { price: f64, tick: f64 },
    /// A reduce-only order against a flat or same-side position.
    ReduceOnlyWouldNotReduce,
    /// A reduce-only order larger than the position it would reduce.
    ReduceOnlyExceedsPosition { position: f64, quantity: f64 },
    /// The fill would drive a cash account negative.
    InsufficientBuyingPower { cash: f64, required: f64 },
    /// A cash account cannot hold a short position.
    ShortNotPermitted,
    /// A bar-delayed submission ran off the end of the symbol's stream.
    SubmitWindowUnavailable { delay_bars: u32 },
    /// A cancel or modify named an order that is not live.
    UnknownOrder,
    /// A modify named a field the order's kind does not have.
    ModifyNotApplicable,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RejectionRecord {
    pub client_order_id: ClientOrderId,
    pub time_ns: i64,
    pub sequence: u64,
    pub symbol: SymbolId,
    pub reason: RejectionReason,
}

/// Why a live order stopped being live without filling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CancelReason {
    /// The strategy asked for it.
    Requested,
    /// Its time-in-force ran out.
    Expired,
    /// Its bracket partner filled.
    OcoSibling,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CancelRecord {
    pub client_order_id: ClientOrderId,
    pub time_ns: i64,
    pub sequence: u64,
    pub symbol: SymbolId,
    pub reason: CancelReason,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingOrderRecord {
    pub order_id: ClientOrderId,
    pub submitted_time_ns: i64,
    pub symbol: SymbolId,
    pub side: OrderSide,
    pub quantity: f64,
    pub kind: OrderKind,
    pub time_in_force: TimeInForce,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PositionRecord {
    pub symbol: SymbolId,
    pub units: f64,
    pub avg_entry: f64,
    pub realized_pnl: f64,
    pub mark_price: Option<f64>,
    pub unrealized_pnl: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EquityPoint {
    pub time_ns: i64,
    pub sequence: u64,
    pub cash: f64,
    pub equity: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulationReport {
    pub symbols: Vec<String>,
    pub events: Vec<SimEventRecord>,
    pub fills: Vec<FillRecord>,
    pub rejections: Vec<RejectionRecord>,
    pub cancellations: Vec<CancelRecord>,
    pub pending_orders: Vec<PendingOrderRecord>,
    pub positions: Vec<PositionRecord>,
    pub equity_curve: Vec<EquityPoint>,
    pub final_cash: f64,
    pub final_equity: f64,
    pub final_realized_pnl: f64,
    pub total_commission: f64,
}

// ── Errors ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BarDefect {
    HighBelowLow,
    OpenOutsideRange,
    CloseOutsideRange,
    NonPositivePrice,
    NegativeVolume,
    NonPositiveDuration,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MarketDataError {
    UnknownSymbol {
        id: usize,
    },
    FutureData {
        symbol: SymbolId,
        bars_ago: usize,
        available: usize,
    },
}

impl fmt::Display for MarketDataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownSymbol { id } => write!(f, "unknown symbol id {id}"),
            Self::FutureData {
                symbol,
                bars_ago,
                available,
            } => write!(
                f,
                "bar {bars_ago} ago for symbol {} is unavailable ({available} committed)",
                symbol.0
            ),
        }
    }
}
impl Error for MarketDataError {}

#[derive(Debug, Clone, PartialEq)]
pub enum StrategyError {
    Rejected { reason: String },
    TooManyIntents { limit: usize },
    InvalidQuantity { quantity: f64 },
    InvalidPrice { price: f64 },
    UnknownSymbol { id: usize },
}

impl fmt::Display for StrategyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rejected { reason } => write!(f, "strategy rejected decision: {reason}"),
            Self::TooManyIntents { limit } => write!(f, "strategy exceeded {limit} intents"),
            Self::InvalidQuantity { quantity } => write!(f, "invalid order quantity {quantity}"),
            Self::InvalidPrice { price } => write!(f, "invalid order price {price}"),
            Self::UnknownSymbol { id } => write!(f, "unknown symbol id {id}"),
        }
    }
}
impl Error for StrategyError {}

#[derive(Debug, Clone, PartialEq)]
pub enum SimulationError {
    Config(StrategyIrError),
    UnsupportedModel {
        field: &'static str,
        model: &'static str,
    },
    InvalidSetup {
        field: &'static str,
        detail: &'static str,
    },
    NoSymbols,
    TooManySymbols {
        limit: usize,
        found: usize,
    },
    InvalidSymbol {
        symbol: String,
    },
    DuplicateSymbol {
        symbol: String,
    },
    EmptyStream {
        symbol: String,
    },
    TooManyBars {
        symbol: String,
        limit: usize,
        found: usize,
    },
    TooManyTotalBars {
        limit: usize,
        found: usize,
    },
    TooManyOrders {
        limit: usize,
    },
    TooManyLiveOrders {
        symbol: String,
        limit: usize,
    },
    TooManyEvents {
        limit: usize,
    },
    NonFiniteBar {
        symbol: String,
        index: usize,
    },
    InconsistentBar {
        symbol: String,
        index: usize,
        defect: BarDefect,
    },
    OverlappingBars {
        symbol: String,
        previous_index: usize,
        index: usize,
    },
    Strategy {
        time_ns: i64,
        error: StrategyError,
    },
    NonFiniteAccounting {
        field: &'static str,
    },
}

impl fmt::Display for SimulationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(error) => write!(f, "invalid execution config: {error}"),
            Self::UnsupportedModel { field, model } => {
                write!(f, "unsupported model `{model}` in {field}")
            }
            Self::InvalidSetup { field, detail } => {
                write!(f, "invalid simulation setup `{field}`: {detail}")
            }
            Self::NoSymbols => f.write_str("simulation has no symbols"),
            Self::TooManySymbols { limit, found } => {
                write!(f, "too many symbols: {found}, limit {limit}")
            }
            Self::InvalidSymbol { symbol } => write!(f, "invalid symbol `{symbol}`"),
            Self::DuplicateSymbol { symbol } => write!(f, "duplicate symbol `{symbol}`"),
            Self::EmptyStream { symbol } => write!(f, "symbol `{symbol}` has no bars"),
            Self::TooManyBars {
                symbol,
                limit,
                found,
            } => write!(f, "symbol `{symbol}` has {found} bars, limit {limit}"),
            Self::TooManyTotalBars { limit, found } => {
                write!(f, "simulation has {found} total bars, limit {limit}")
            }
            Self::TooManyOrders { limit } => {
                write!(f, "simulation created more than {limit} orders")
            }
            Self::TooManyLiveOrders { symbol, limit } => {
                write!(
                    f,
                    "symbol `{symbol}` rests more than {limit} orders at once"
                )
            }
            Self::TooManyEvents { limit } => {
                write!(f, "simulation recorded more than {limit} events")
            }
            Self::NonFiniteBar { symbol, index } => write!(
                f,
                "symbol `{symbol}` bar {index} contains a non-finite value"
            ),
            Self::InconsistentBar {
                symbol,
                index,
                defect,
            } => write!(
                f,
                "symbol `{symbol}` bar {index} is inconsistent: {defect:?}"
            ),
            Self::OverlappingBars {
                symbol,
                previous_index,
                index,
            } => write!(
                f,
                "symbol `{symbol}` bars {previous_index} and {index} overlap"
            ),
            Self::Strategy { time_ns, error } => write!(f, "strategy failed at {time_ns}: {error}"),
            Self::NonFiniteAccounting { field } => {
                write!(f, "accounting produced a non-finite `{field}` value")
            }
        }
    }
}
impl Error for SimulationError {}

// ── Intents ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum Intent {
    Submit {
        client_id: ClientOrderId,
        request: OrderRequest,
    },
    Cancel {
        target: ClientOrderId,
    },
    Modify {
        target: ClientOrderId,
        change: ModifyRequest,
    },
}

/// What one decision may ask for. Every method validates its arguments before
/// the intent is recorded, so a malformed order never reaches the book.
pub struct OrderIntents {
    symbol_count: usize,
    next_client_id: u64,
    intents: Vec<Intent>,
}

impl OrderIntents {
    fn new(symbol_count: usize, next_client_id: u64) -> Self {
        Self {
            symbol_count,
            next_client_id,
            intents: Vec::new(),
        }
    }

    fn reserve(&mut self) -> Result<(), StrategyError> {
        if self.intents.len() >= MAX_INTENTS_PER_DECISION {
            return Err(StrategyError::TooManyIntents {
                limit: MAX_INTENTS_PER_DECISION,
            });
        }
        Ok(())
    }

    /// Shorthand for a plain market order.
    pub fn market(
        &mut self,
        symbol: SymbolId,
        side: OrderSide,
        quantity: f64,
    ) -> Result<ClientOrderId, StrategyError> {
        self.submit(OrderRequest::market(symbol, side, quantity))
    }

    /// Submit any order. The returned id is the handle for a later cancel or
    /// modify.
    pub fn submit(&mut self, request: OrderRequest) -> Result<ClientOrderId, StrategyError> {
        if request.symbol.0 >= self.symbol_count {
            return Err(StrategyError::UnknownSymbol {
                id: request.symbol.0,
            });
        }
        if !request.quantity.is_finite()
            || request.quantity <= 0.0
            || request.quantity > MAX_ORDER_QUANTITY
        {
            return Err(StrategyError::InvalidQuantity {
                quantity: request.quantity,
            });
        }
        for price in order_prices(&request.kind) {
            if !price.is_finite() || price <= 0.0 {
                return Err(StrategyError::InvalidPrice { price });
            }
        }
        self.reserve()?;
        let client_id = ClientOrderId(self.next_client_id);
        self.next_client_id = self.next_client_id.saturating_add(1);
        self.intents.push(Intent::Submit { client_id, request });
        Ok(client_id)
    }

    /// Ask for a resting order to be withdrawn. The request travels with the
    /// same latency as an order, and is reported if it names an order that is
    /// no longer live.
    pub fn cancel(&mut self, target: ClientOrderId) -> Result<(), StrategyError> {
        self.reserve()?;
        self.intents.push(Intent::Cancel { target });
        Ok(())
    }

    /// Ask for a resting order to be repriced or resized.
    pub fn modify(
        &mut self,
        target: ClientOrderId,
        change: ModifyRequest,
    ) -> Result<(), StrategyError> {
        if let Some(quantity) = change.quantity
            && (!quantity.is_finite() || quantity <= 0.0 || quantity > MAX_ORDER_QUANTITY)
        {
            return Err(StrategyError::InvalidQuantity { quantity });
        }
        for price in [change.limit_price, change.stop_price]
            .into_iter()
            .flatten()
        {
            if !price.is_finite() || price <= 0.0 {
                return Err(StrategyError::InvalidPrice { price });
            }
        }
        self.reserve()?;
        self.intents.push(Intent::Modify { target, change });
        Ok(())
    }
}

fn order_prices(kind: &OrderKind) -> Vec<f64> {
    match *kind {
        OrderKind::Market | OrderKind::MarketOnClose => Vec::new(),
        OrderKind::Limit { limit_price } => vec![limit_price],
        OrderKind::Stop { stop_price } => vec![stop_price],
        OrderKind::StopLimit {
            stop_price,
            limit_price,
        } => vec![stop_price, limit_price],
    }
}

// ── Market view ────────────────────────────────────────────────────

/// Read-only access to everything a decision is allowed to know.
///
/// There is no accessor that returns a slice, a length, or an iterator, and
/// `bars_ago` is unsigned. Those two facts are what make look-ahead
/// unrepresentable rather than merely discouraged.
pub struct MarketView<'a> {
    streams: &'a [SymbolStream],
    committed: &'a [usize],
    opened: &'a [Option<usize>],
}

impl MarketView<'_> {
    pub fn symbol_id(&self, symbol: &str) -> Option<SymbolId> {
        self.streams
            .binary_search_by(|stream| stream.symbol.as_str().cmp(symbol))
            .ok()
            .map(SymbolId)
    }

    fn completed_bar(&self, symbol: SymbolId, bars_ago: usize) -> Result<&SimBar, MarketDataError> {
        let Some(stream) = self.streams.get(symbol.0) else {
            return Err(MarketDataError::UnknownSymbol { id: symbol.0 });
        };
        let available = self.committed.get(symbol.0).copied().unwrap_or(0);
        if bars_ago >= available {
            return Err(MarketDataError::FutureData {
                symbol,
                bars_ago,
                available,
            });
        }
        let index = available - bars_ago - 1;
        stream.bars.get(index).ok_or(MarketDataError::FutureData {
            symbol,
            bars_ago,
            available,
        })
    }

    pub fn open(&self, symbol: SymbolId, bars_ago: usize) -> Result<f64, MarketDataError> {
        self.completed_bar(symbol, bars_ago).map(|bar| bar.open)
    }
    pub fn high(&self, symbol: SymbolId, bars_ago: usize) -> Result<f64, MarketDataError> {
        self.completed_bar(symbol, bars_ago).map(|bar| bar.high)
    }
    pub fn low(&self, symbol: SymbolId, bars_ago: usize) -> Result<f64, MarketDataError> {
        self.completed_bar(symbol, bars_ago).map(|bar| bar.low)
    }
    pub fn close(&self, symbol: SymbolId, bars_ago: usize) -> Result<f64, MarketDataError> {
        self.completed_bar(symbol, bars_ago).map(|bar| bar.close)
    }
    pub fn volume(&self, symbol: SymbolId, bars_ago: usize) -> Result<f64, MarketDataError> {
        self.completed_bar(symbol, bars_ago).map(|bar| bar.volume)
    }

    /// The open of the bar currently in progress for `symbol`. This is the one
    /// price of an unfinished bar that has already printed.
    pub fn opening_price(&self, symbol: SymbolId) -> Result<f64, MarketDataError> {
        let Some(stream) = self.streams.get(symbol.0) else {
            return Err(MarketDataError::UnknownSymbol { id: symbol.0 });
        };
        let Some(index) = self.opened.get(symbol.0).copied().flatten() else {
            return Err(MarketDataError::FutureData {
                symbol,
                bars_ago: 0,
                available: 0,
            });
        };
        stream
            .bars
            .get(index)
            .map(|bar| bar.open)
            .ok_or(MarketDataError::FutureData {
                symbol,
                bars_ago: 0,
                available: 0,
            })
    }
}

pub struct DecisionContext<'a> {
    symbol: SymbolId,
    time_ns: i64,
    forming: Option<FormingBar>,
    market: MarketView<'a>,
}

impl DecisionContext<'_> {
    pub const fn symbol(&self) -> SymbolId {
        self.symbol
    }
    pub const fn market(&self) -> &MarketView<'_> {
        &self.market
    }
    /// The event time this decision is being made at.
    pub const fn decision_time_ns(&self) -> i64 {
        self.time_ns
    }
    /// The bar in progress, at a pre-close decision only. `None` at a
    /// closed-bar decision, where the next bar has not started.
    pub const fn forming_bar(&self) -> Option<FormingBar> {
        self.forming
    }
}

pub trait ReferenceStrategy {
    fn on_bar_close(
        &mut self,
        ctx: &DecisionContext<'_>,
        orders: &mut OrderIntents,
    ) -> Result<(), StrategyError>;
}

// ── Internal state ─────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OrderState {
    /// Submitted, waiting for the exchange.
    Submitted,
    /// Live and eligible from the next bar open at or after `active_time_ns`.
    Active,
    /// Terminal.
    Done,
}

#[derive(Debug, Clone)]
struct Order {
    client_id: ClientOrderId,
    symbol: SymbolId,
    side: OrderSide,
    quantity: f64,
    kind: OrderKind,
    time_in_force: TimeInForce,
    reduce_only: bool,
    oco_group: Option<u32>,
    submit_time_ns: i64,
    active_time_ns: i64,
    submit_sequence: u64,
    state: OrderState,
    /// Set once an IOC/FOK order has had its one execution opportunity.
    had_opportunity: bool,
}

#[derive(Debug, Clone, Default)]
struct PositionState {
    units: f64,
    avg_entry: f64,
    realized_pnl: f64,
}

struct Recorder {
    sequence: u64,
    events: Vec<SimEventRecord>,
}

impl Recorder {
    fn event(
        &mut self,
        time_ns: i64,
        kind: SimEventKind,
        priority: u8,
        symbol: Option<SymbolId>,
        order_id: Option<ClientOrderId>,
    ) -> Result<u64, SimulationError> {
        if self.events.len() >= MAX_EVENTS {
            return Err(SimulationError::TooManyEvents { limit: MAX_EVENTS });
        }
        let sequence = self.sequence;
        self.sequence = self.sequence.saturating_add(1);
        self.events.push(SimEventRecord {
            time_ns,
            kind,
            priority,
            sequence,
            symbol,
            order_id,
        });
        Ok(sequence)
    }
}

/// A scheduled unit of work. Order-driven tasks carry an arena index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Task {
    BarOpen { symbol: usize, bar: usize },
    ExecuteOpen { symbol: usize, bar: usize },
    BarClose { symbol: usize, bar: usize },
    ExecuteClose { symbol: usize, bar: usize },
    Decision { symbol: usize, bar: usize },
    Mark { symbol: usize, bar: usize },
    Submit { order: usize },
    Activate { order: usize },
    Expire { order: usize },
    ApplyRequest { request: usize },
}

#[derive(Debug, Clone, Copy)]
struct ScheduledTask {
    time_ns: i64,
    priority: u8,
    tie: u64,
    task: Task,
}

#[derive(Debug, Clone)]
struct PendingRequest {
    symbol: SymbolId,
    target: ClientOrderId,
    change: Option<ModifyRequest>,
}

// ── Deterministic draws (§6.4, §6.10) ──────────────────────────────

/// SplitMix64. Used as a pure hash of `(seed, order, leg)` rather than as a
/// stateful stream, so a draw cannot depend on how many draws came before it —
/// which is what makes parallel and reordered execution produce the same
/// ledger.
const fn splitmix64(seed: u64) -> u64 {
    let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn draw_inclusive(seed: u64, stream: u64, leg: u64, min: i64, max: i64) -> i64 {
    if max <= min {
        return min;
    }
    let span = max.wrapping_sub(min) as u64;
    let mixed =
        splitmix64(seed ^ splitmix64(stream.wrapping_mul(0x2545_F491_4F6C_DD1D) ^ splitmix64(leg)));
    min.wrapping_add((mixed % span.saturating_add(1)) as i64)
}

/// The two latency legs for one order or request.
fn latency_legs(model: &LatencyModel, seed: u64, stream: u64) -> (i64, i64) {
    match *model {
        LatencyModel::None => (0, 0),
        LatencyModel::Fixed {
            decision_to_submit_ns,
            submit_to_exchange_ns,
        } => (decision_to_submit_ns, submit_to_exchange_ns),
        LatencyModel::SeededUniform {
            decision_to_submit_min_ns,
            decision_to_submit_max_ns,
            submit_to_exchange_min_ns,
            submit_to_exchange_max_ns,
        } => (
            draw_inclusive(
                seed,
                stream,
                0,
                decision_to_submit_min_ns,
                decision_to_submit_max_ns,
            ),
            draw_inclusive(
                seed,
                stream,
                1,
                submit_to_exchange_min_ns,
                submit_to_exchange_max_ns,
            ),
        ),
    }
}

// ── Cost helpers ───────────────────────────────────────────────────

fn spread_width(model: &SpreadModel, reference: f64) -> f64 {
    match model {
        SpreadModel::None => 0.0,
        SpreadModel::Constant { price_units } => *price_units,
        SpreadModel::PercentOfPrice { percent } => reference * percent / 100.0,
        SpreadModel::RecordedQuotes => 0.0,
    }
}

fn slippage_distance(model: &SlippageModel, width: f64) -> f64 {
    match model {
        SlippageModel::None => 0.0,
        SlippageModel::FixedPriceDistance { distance } => *distance,
        SlippageModel::SpreadFraction { fraction } => width * fraction,
        SlippageModel::VolatilityScaled { .. } => 0.0,
    }
}

fn commission(model: &CommissionModel, side: OrderSide, quantity: f64, fill_price: f64) -> f64 {
    match model {
        CommissionModel::None => 0.0,
        CommissionModel::PerShare { amount, minimum } => (amount * quantity).max(*minimum),
        CommissionModel::PercentOfNotional { percent, minimum } => {
            (fill_price * quantity * percent / 100.0).max(*minimum)
        }
        CommissionModel::PerOrder { amount } => *amount,
        CommissionModel::VenueSchedule(binding) => {
            binding.charge(side.fee_side(), quantity, fill_price)
        }
    }
}

/// Place computed monetary values on a stable decimal lattice so an already
/// serialized report remains byte-identical after a serde JSON round trip.
fn stable_decimal(value: f64) -> f64 {
    const SCALE: f64 = 1_000_000_000_000.0;
    let rounded = if value.abs() <= f64::MAX / SCALE {
        (value * SCALE).round() / SCALE
    } else {
        value
    };
    if rounded == 0.0 { 0.0 } else { rounded }
}

fn finite_accounting(field: &'static str, value: f64) -> Result<f64, SimulationError> {
    value
        .is_finite()
        .then_some(value)
        .ok_or(SimulationError::NonFiniteAccounting { field })
}

fn apply_fill(position: &mut PositionState, side: OrderSide, quantity: f64, price: f64) -> f64 {
    let delta = side.sign() * quantity;
    let old = position.units;
    if old == 0.0 || old.signum() == delta.signum() {
        let new_units = old + delta;
        position.avg_entry = stable_decimal(if old == 0.0 {
            price
        } else {
            (position.avg_entry * old.abs() + price * quantity) / new_units.abs()
        });
        position.units = new_units;
        return 0.0;
    }
    let closed = old.abs().min(quantity);
    let realized = if old > 0.0 {
        (price - position.avg_entry) * closed
    } else {
        (position.avg_entry - price) * closed
    };
    let new_units = old + delta;
    if new_units == 0.0 {
        position.avg_entry = 0.0;
    } else if new_units.signum() != old.signum() {
        position.avg_entry = price;
    }
    position.units = new_units;
    position.realized_pnl = stable_decimal(position.realized_pnl + realized);
    stable_decimal(realized)
}

fn marked_equity(cash: f64, positions: &[PositionState], marks: &[Option<f64>]) -> f64 {
    positions
        .iter()
        .zip(marks)
        .fold(cash, |equity, (position, mark)| {
            equity + mark.map_or(0.0, |price| position.units * price)
        })
}

/// The first instant of the UTC day after `time_ns`.
const fn next_utc_day_start(time_ns: i64) -> i64 {
    let day = time_ns.div_euclid(NANOS_PER_DAY);
    (day + 1) * NANOS_PER_DAY
}

// ── Input validation ───────────────────────────────────────────────

fn validate_inputs(streams: &[SymbolStream]) -> Result<Vec<SymbolStream>, SimulationError> {
    if streams.is_empty() {
        return Err(SimulationError::NoSymbols);
    }
    if streams.len() > MAX_SYMBOLS {
        return Err(SimulationError::TooManySymbols {
            limit: MAX_SYMBOLS,
            found: streams.len(),
        });
    }
    let total_bars = streams.iter().map(|stream| stream.bars.len()).sum();
    if total_bars > MAX_TOTAL_BARS {
        return Err(SimulationError::TooManyTotalBars {
            limit: MAX_TOTAL_BARS,
            found: total_bars,
        });
    }
    let mut sorted = streams.to_vec();
    sorted.sort_by(|a, b| a.symbol.cmp(&b.symbol));
    for (stream_index, stream) in sorted.iter().enumerate() {
        if stream.symbol.is_empty()
            || stream.symbol.trim() != stream.symbol
            || stream.symbol.chars().count() > MAX_SYMBOL_LEN
            || stream.symbol.chars().any(char::is_control)
        {
            return Err(SimulationError::InvalidSymbol {
                symbol: stream.symbol.clone(),
            });
        }
        if stream_index > 0 && sorted[stream_index - 1].symbol == stream.symbol {
            return Err(SimulationError::DuplicateSymbol {
                symbol: stream.symbol.clone(),
            });
        }
        if stream.bars.is_empty() {
            return Err(SimulationError::EmptyStream {
                symbol: stream.symbol.clone(),
            });
        }
        if stream.bars.len() > MAX_BARS_PER_SYMBOL {
            return Err(SimulationError::TooManyBars {
                symbol: stream.symbol.clone(),
                limit: MAX_BARS_PER_SYMBOL,
                found: stream.bars.len(),
            });
        }
        for (index, bar) in stream.bars.iter().enumerate() {
            if ![bar.open, bar.high, bar.low, bar.close, bar.volume]
                .iter()
                .all(|value| value.is_finite())
            {
                return Err(SimulationError::NonFiniteBar {
                    symbol: stream.symbol.clone(),
                    index,
                });
            }
            let defect = if bar.close_time_ns <= bar.open_time_ns {
                Some(BarDefect::NonPositiveDuration)
            } else if bar.open <= 0.0 || bar.high <= 0.0 || bar.low <= 0.0 || bar.close <= 0.0 {
                Some(BarDefect::NonPositivePrice)
            } else if bar.volume < 0.0 {
                Some(BarDefect::NegativeVolume)
            } else if bar.high < bar.low {
                Some(BarDefect::HighBelowLow)
            } else if bar.open < bar.low || bar.open > bar.high {
                Some(BarDefect::OpenOutsideRange)
            } else if bar.close < bar.low || bar.close > bar.high {
                Some(BarDefect::CloseOutsideRange)
            } else {
                None
            };
            if let Some(defect) = defect {
                return Err(SimulationError::InconsistentBar {
                    symbol: stream.symbol.clone(),
                    index,
                    defect,
                });
            }
            if index > 0 && bar.open_time_ns <= stream.bars[index - 1].close_time_ns {
                return Err(SimulationError::OverlappingBars {
                    symbol: stream.symbol.clone(),
                    previous_index: index - 1,
                    index,
                });
            }
        }
    }
    Ok(sorted)
}

fn validate_models(config: &StrategyExecutionConfig) -> Result<(), SimulationError> {
    if matches!(
        config.settings().slippage,
        SlippageModel::VolatilityScaled { .. }
    ) {
        return Err(SimulationError::UnsupportedModel {
            field: "settings.slippage",
            model: "volatility_scaled",
        });
    }
    if matches!(config.settings().spread, SpreadModel::RecordedQuotes) {
        return Err(SimulationError::UnsupportedModel {
            field: "settings.spread",
            model: "recorded_quotes",
        });
    }
    if matches!(
        config.settings().tie_break,
        TieBreakPolicy::TimestampPrioritySymbolSequence
    ) {
        return Err(SimulationError::UnsupportedModel {
            field: "settings.tie_break",
            model: "timestamp_priority_symbol_sequence",
        });
    }
    Ok(())
}

fn validate_setup(setup: &SimulationSetup) -> Result<(), SimulationError> {
    if setup.submit_delay_bars > MAX_SUBMIT_DELAY_BARS {
        return Err(SimulationError::InvalidSetup {
            field: "submit_delay_bars",
            detail: "exceeds MAX_SUBMIT_DELAY_BARS",
        });
    }
    if let DecisionPoint::PreClose { offset_ns } = setup.decision_point {
        let max = i64::from(MAX_PRE_CLOSE_OFFSET_SECONDS) * 1_000_000_000;
        if offset_ns <= 0 || offset_ns > max {
            return Err(SimulationError::InvalidSetup {
                field: "decision_point.pre_close.offset_ns",
                // A zero offset would put the decision on the close stamp,
                // where the bar has already committed — that is the leak the
                // pre-close mode exists to avoid.
                detail: "must be a positive offset within MAX_PRE_CLOSE_OFFSET_SECONDS",
            });
        }
    }
    Ok(())
}

// ── Trigger resolution (§6.1) ──────────────────────────────────────

/// What a bar does to one resting order.
#[derive(Debug, Clone, Copy)]
struct Trigger {
    /// Position along the assumed intrabar path: `0` is "already through it at
    /// the open", higher is later. Only meaningful under `OhlcPath`.
    rank: u8,
    /// The mid the execution resolves against.
    mid: f64,
    /// Whether the order pays slippage. Limits do not: they cannot fill worse
    /// than their price.
    marketable: bool,
}

/// Resolve one order against one bar at `BarOhlc` fidelity.
///
/// `half` is the bar's half-spread. Buy triggers are tested against the ask
/// (`mid + half`) and sell triggers against the bid (`mid - half`), because
/// that is the price the order would actually have to reach.
fn intrabar_trigger(
    kind: OrderKind,
    side: OrderSide,
    bar: &SimBar,
    half: f64,
) -> Option<(Trigger, bool)> {
    let up = bar.close >= bar.open;
    match (kind, side) {
        (OrderKind::Market, _) => Some((
            Trigger {
                rank: 0,
                mid: bar.open,
                marketable: true,
            },
            false,
        )),
        (OrderKind::MarketOnClose, _) => None,
        (OrderKind::Limit { limit_price }, OrderSide::Buy) => {
            if bar.open + half <= limit_price {
                return Some((
                    Trigger {
                        rank: 0,
                        mid: bar.open,
                        marketable: false,
                    },
                    false,
                ));
            }
            (bar.low + half <= limit_price).then(|| {
                (
                    Trigger {
                        rank: if up { 2 } else { 1 },
                        mid: limit_price - half,
                        marketable: false,
                    },
                    false,
                )
            })
        }
        (OrderKind::Limit { limit_price }, OrderSide::Sell) => {
            if bar.open - half >= limit_price {
                return Some((
                    Trigger {
                        rank: 0,
                        mid: bar.open,
                        marketable: false,
                    },
                    false,
                ));
            }
            (bar.high - half >= limit_price).then(|| {
                (
                    Trigger {
                        rank: if up { 1 } else { 2 },
                        mid: limit_price + half,
                        marketable: false,
                    },
                    false,
                )
            })
        }
        (OrderKind::Stop { stop_price }, OrderSide::Buy) => {
            if bar.open + half >= stop_price {
                return Some((
                    Trigger {
                        rank: 0,
                        mid: bar.open,
                        marketable: true,
                    },
                    false,
                ));
            }
            (bar.high + half >= stop_price).then(|| {
                (
                    Trigger {
                        rank: if up { 1 } else { 2 },
                        mid: stop_price - half,
                        marketable: true,
                    },
                    false,
                )
            })
        }
        (OrderKind::Stop { stop_price }, OrderSide::Sell) => {
            if bar.open - half <= stop_price {
                return Some((
                    Trigger {
                        rank: 0,
                        mid: bar.open,
                        marketable: true,
                    },
                    false,
                ));
            }
            (bar.low - half <= stop_price).then(|| {
                (
                    Trigger {
                        rank: if up { 2 } else { 1 },
                        mid: stop_price + half,
                        marketable: true,
                    },
                    false,
                )
            })
        }
        (
            OrderKind::StopLimit {
                stop_price,
                limit_price,
            },
            _,
        ) => {
            let (stop_trigger, _) =
                intrabar_trigger(OrderKind::Stop { stop_price }, side, bar, half)?;
            let executed = stop_trigger.mid + side.sign() * half;
            let marketable = match side {
                OrderSide::Buy => executed <= limit_price,
                OrderSide::Sell => executed >= limit_price,
            };
            if marketable {
                Some((
                    Trigger {
                        // The limit protects the price, so no slippage past it.
                        marketable: false,
                        ..stop_trigger
                    },
                    true,
                ))
            } else {
                None
            }
        }
    }
}

fn resting_stop_limit_at_boundary(
    kind: OrderKind,
    side: OrderSide,
    mid: f64,
    half: f64,
) -> Option<f64> {
    let OrderKind::StopLimit {
        stop_price,
        limit_price,
    } = kind
    else {
        return None;
    };
    let triggered = match side {
        OrderSide::Buy => mid + half >= stop_price,
        OrderSide::Sell => mid - half <= stop_price,
    };
    let executed = mid + side.sign() * half;
    let marketable = match side {
        OrderSide::Buy => executed <= limit_price,
        OrderSide::Sell => executed >= limit_price,
    };
    (triggered && !marketable).then_some(limit_price)
}

fn resting_stop_limit_in_bar(
    kind: OrderKind,
    side: OrderSide,
    bar: &SimBar,
    half: f64,
) -> Option<f64> {
    let OrderKind::StopLimit {
        stop_price,
        limit_price,
    } = kind
    else {
        return None;
    };
    let triggered = match side {
        OrderSide::Buy => bar.high + half >= stop_price,
        OrderSide::Sell => bar.low - half <= stop_price,
    };
    let marketable = match side {
        OrderSide::Buy => stop_price <= limit_price,
        OrderSide::Sell => stop_price >= limit_price,
    };
    (triggered && !marketable).then_some(limit_price)
}

/// Resolve one order against a single execution price — the rule at
/// `BarClose` fidelity, where only bar boundaries are execution prices.
fn boundary_trigger(kind: OrderKind, side: OrderSide, mid: f64, half: f64) -> Option<Trigger> {
    let hit = |ok: bool, marketable: bool, at: f64| {
        ok.then_some(Trigger {
            rank: 0,
            mid: at,
            marketable,
        })
    };
    match (kind, side) {
        (OrderKind::Market, _) => hit(true, true, mid),
        (OrderKind::MarketOnClose, _) => None,
        (OrderKind::Limit { limit_price }, OrderSide::Buy) => {
            hit(mid + half <= limit_price, false, mid)
        }
        (OrderKind::Limit { limit_price }, OrderSide::Sell) => {
            hit(mid - half >= limit_price, false, mid)
        }
        (OrderKind::Stop { stop_price }, OrderSide::Buy) => {
            hit(mid + half >= stop_price, true, mid)
        }
        (OrderKind::Stop { stop_price }, OrderSide::Sell) => {
            hit(mid - half <= stop_price, true, mid)
        }
        (
            OrderKind::StopLimit {
                stop_price,
                limit_price,
            },
            _,
        ) => {
            let triggered = match side {
                OrderSide::Buy => mid + half >= stop_price,
                OrderSide::Sell => mid - half <= stop_price,
            };
            if !triggered {
                return None;
            }
            let executed = mid + side.sign() * half;
            let marketable = match side {
                OrderSide::Buy => executed <= limit_price,
                OrderSide::Sell => executed >= limit_price,
            };
            marketable.then_some(Trigger {
                rank: 0,
                mid,
                marketable: false,
            })
        }
    }
}

// ── Simulation ─────────────────────────────────────────────────────

struct Sim<'a> {
    config: &'a StrategyExecutionConfig,
    setup: &'a SimulationSetup,
    streams: Vec<SymbolStream>,
    symbol_count: usize,
    recorder: Recorder,
    queue: Vec<ScheduledTask>,
    cursor: usize,
    schedule_tie: u64,
    orders: Vec<Order>,
    requests: Vec<PendingRequest>,
    live: Vec<Vec<usize>>,
    committed: Vec<usize>,
    opened: Vec<Option<usize>>,
    marks: Vec<Option<f64>>,
    positions: Vec<PositionState>,
    fills: Vec<FillRecord>,
    rejections: Vec<RejectionRecord>,
    cancellations: Vec<CancelRecord>,
    equity_curve: Vec<EquityPoint>,
    cash: f64,
    total_commission: f64,
    next_client_id: u64,
}

/// Deterministic ordering of scheduled work: time, then the phase table, then
/// the scheduling counter. Never a hash map's iteration order.
fn schedule_key(task: &ScheduledTask) -> (i64, u8, u64) {
    (task.time_ns, task.priority, task.tie)
}

impl Sim<'_> {
    fn push(&mut self, time_ns: i64, priority: u8, tie: u64, task: Task) {
        self.queue.push(ScheduledTask {
            time_ns,
            priority,
            tie,
            task,
        });
    }

    /// Schedule a task discovered mid-run, keeping the queue sorted from the
    /// cursor onward. The tail is short — one run only ever has a handful of
    /// in-flight orders per bar — so an insertion sort is both simplest and
    /// exactly reproducible.
    fn schedule_late(&mut self, time_ns: i64, priority: u8, task: Task) {
        let tie = self.schedule_tie;
        self.schedule_tie = self.schedule_tie.saturating_add(1);
        let entry = ScheduledTask {
            time_ns,
            priority,
            tie,
            task,
        };
        let key = schedule_key(&entry);
        let mut index = self.queue.len();
        while index > self.cursor && schedule_key(&self.queue[index - 1]) > key {
            index -= 1;
        }
        self.queue.insert(index, entry);
    }

    fn bar(&self, symbol: usize, bar: usize) -> SimBar {
        self.streams[symbol].bars[bar]
    }

    fn half_spread(&self, reference: f64) -> f64 {
        stable_decimal(spread_width(&self.config.settings().spread, reference)) / 2.0
    }

    fn record_rejection(
        &mut self,
        time_ns: i64,
        symbol: SymbolId,
        client_order_id: ClientOrderId,
        reason: RejectionReason,
    ) -> Result<(), SimulationError> {
        let sequence = self.recorder.event(
            time_ns,
            SimEventKind::OrderReject,
            priority::REJECT,
            Some(symbol),
            Some(client_order_id),
        )?;
        self.rejections.push(RejectionRecord {
            client_order_id,
            time_ns,
            sequence,
            symbol,
            reason,
        });
        Ok(())
    }

    fn retire(&mut self, order_index: usize) {
        let symbol = self.orders[order_index].symbol.0;
        self.orders[order_index].state = OrderState::Done;
        self.live[symbol].retain(|index| *index != order_index);
    }

    fn cancel_order(
        &mut self,
        order_index: usize,
        time_ns: i64,
        priority: u8,
        reason: CancelReason,
    ) -> Result<(), SimulationError> {
        let (client_order_id, symbol) = {
            let order = &self.orders[order_index];
            (order.client_id, order.symbol)
        };
        let sequence = self.recorder.event(
            time_ns,
            match reason {
                CancelReason::Expired => SimEventKind::OrderExpire,
                _ => SimEventKind::OrderCancel,
            },
            priority,
            Some(symbol),
            Some(client_order_id),
        )?;
        self.cancellations.push(CancelRecord {
            client_order_id,
            time_ns,
            sequence,
            symbol,
            reason,
        });
        self.retire(order_index);
        Ok(())
    }
}

#[derive(Debug)]
pub enum VerifiedSimulationError {
    Interpreter(InterpreterError),
    Simulation(SimulationError),
    UnsupportedDatasetTimeframe { input_id: String, timeframe: String },
    InvalidDatasetTimestamp { input_id: String, timestamp: String },
    DatasetTimeOverflow { input_id: String },
}

impl fmt::Display for VerifiedSimulationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Interpreter(error) => write!(formatter, "strategy interpreter: {error}"),
            Self::Simulation(error) => write!(formatter, "simulation: {error}"),
            Self::UnsupportedDatasetTimeframe {
                input_id,
                timeframe,
            } => write!(
                formatter,
                "dataset `{input_id}` has unsupported simulation timeframe `{timeframe}`"
            ),
            Self::InvalidDatasetTimestamp {
                input_id,
                timestamp,
            } => write!(
                formatter,
                "dataset `{input_id}` has invalid bar timestamp `{timestamp}`"
            ),
            Self::DatasetTimeOverflow { input_id } => {
                write!(
                    formatter,
                    "dataset `{input_id}` bar time exceeds simulator range"
                )
            }
        }
    }
}

impl Error for VerifiedSimulationError {}

/// Execute an identity-bearing run without accepting any simulation knob that
/// could disagree with its sealed strategy and manifest.
pub fn run_verified_simulation(
    run: &VerifiedRun<'_>,
) -> Result<SimulationReport, VerifiedSimulationError> {
    let setup = SimulationSetup::from_verified_run(run);
    let mut strategy =
        CanonicalIrStrategy::new(run.strategy()).map_err(VerifiedSimulationError::Interpreter)?;
    let streams = verified_streams(run)?;
    run_simulation(run.config(), &setup, &streams, &mut strategy)
        .map_err(VerifiedSimulationError::Simulation)
}

fn verified_streams(run: &VerifiedRun<'_>) -> Result<Vec<SymbolStream>, VerifiedSimulationError> {
    run.datasets()
        .iter()
        .map(|dataset| {
            let input_id = dataset.input_id();
            let timeframe = &dataset.manifest.timeframe;
            let step_seconds = fixed_timeframe_seconds(timeframe).ok_or_else(|| {
                VerifiedSimulationError::UnsupportedDatasetTimeframe {
                    input_id: input_id.to_string(),
                    timeframe: timeframe.clone(),
                }
            })?;
            let step_ns = step_seconds.checked_mul(1_000_000_000).ok_or_else(|| {
                VerifiedSimulationError::DatasetTimeOverflow {
                    input_id: input_id.to_string(),
                }
            })?;
            let bars = dataset
                .bars
                .iter()
                .map(|bar| {
                    let open_time_ns = chrono::DateTime::parse_from_rfc3339(&bar.timestamp)
                        .ok()
                        .and_then(|stamp| stamp.timestamp_nanos_opt())
                        .ok_or_else(|| VerifiedSimulationError::InvalidDatasetTimestamp {
                            input_id: input_id.to_string(),
                            timestamp: bar.timestamp.clone(),
                        })?;
                    let close_time_ns = open_time_ns
                        .checked_add(step_ns)
                        .and_then(|time| time.checked_sub(1))
                        .ok_or_else(|| VerifiedSimulationError::DatasetTimeOverflow {
                            input_id: input_id.to_string(),
                        })?;
                    Ok(SimBar {
                        open_time_ns,
                        close_time_ns,
                        open: bar.open,
                        high: bar.high,
                        low: bar.low,
                        close: bar.close,
                        volume: bar.volume,
                    })
                })
                .collect::<Result<Vec<_>, VerifiedSimulationError>>()?;
            Ok(SymbolStream {
                symbol: dataset.manifest.symbol.clone(),
                bars,
            })
        })
        .collect()
}

fn fixed_timeframe_seconds(timeframe: &str) -> Option<i64> {
    let digits = timeframe
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(timeframe.len());
    if digits == 0 {
        return None;
    }
    let count = timeframe[..digits].parse::<i64>().ok()?;
    if count <= 0 {
        return None;
    }
    let unit = match &timeframe[digits..] {
        "Min" => 60,
        "Hour" => 3_600,
        "Day" => 86_400,
        "Week" => 604_800,
        _ => return None,
    };
    count.checked_mul(unit)
}

/// Run one deterministic simulation.
///
/// This is the raw kernel API for tests and callers that are not publishing a
/// run identity. Persisted/identified runs must use [`run_verified_simulation`],
/// which derives setup and strategy from verified artifacts and has no mutable
/// setup parameter that can disagree with the run id.
///
/// `setup` carries what the strategy decides (when it decides, how long it
/// waits to submit, and the run's root seed); `config` carries what the venue
/// and the account impose.
pub fn run_simulation(
    config: &StrategyExecutionConfig,
    setup: &SimulationSetup,
    streams: &[SymbolStream],
    strategy: &mut dyn ReferenceStrategy,
) -> Result<SimulationReport, SimulationError> {
    config.verify().map_err(SimulationError::Config)?;
    validate_models(config)?;
    validate_setup(setup)?;
    let streams = validate_inputs(streams)?;
    let symbol_count = streams.len();

    let mut sim = Sim {
        config,
        setup,
        streams,
        symbol_count,
        recorder: Recorder {
            sequence: 0,
            events: Vec::new(),
        },
        queue: Vec::new(),
        cursor: 0,
        schedule_tie: 0,
        orders: Vec::new(),
        requests: Vec::new(),
        live: vec![Vec::new(); symbol_count],
        committed: vec![0; symbol_count],
        opened: vec![None; symbol_count],
        marks: vec![None; symbol_count],
        positions: vec![PositionState::default(); symbol_count],
        fills: Vec::new(),
        rejections: Vec::new(),
        cancellations: Vec::new(),
        equity_curve: Vec::new(),
        cash: config.settings().initial_capital,
        total_commission: 0.0,
        next_client_id: 0,
    };

    build_bar_schedule(&mut sim);
    sim.queue.sort_by_key(schedule_key);
    sim.schedule_tie = sim.queue.len() as u64;

    while sim.cursor < sim.queue.len() {
        let entry = sim.queue[sim.cursor];
        sim.cursor += 1;
        run_task(&mut sim, entry, strategy)?;
    }

    finish(sim)
}

/// Lay down every bar-driven event up front. Ties inside one `(time, priority)`
/// resolve by symbol-table order, which is what makes cross-symbol behaviour
/// reproducible.
fn build_bar_schedule(sim: &mut Sim<'_>) {
    for symbol in 0..sim.symbol_count {
        for bar in 0..sim.streams[symbol].bars.len() {
            let row = sim.streams[symbol].bars[bar];
            let tie = symbol as u64;
            sim.push(
                row.open_time_ns,
                priority::BAR_OPEN,
                tie,
                Task::BarOpen { symbol, bar },
            );
            sim.push(
                row.open_time_ns,
                priority::OPEN_EXECUTION,
                tie,
                Task::ExecuteOpen { symbol, bar },
            );
            sim.push(
                row.close_time_ns,
                priority::BAR_CLOSE,
                tie,
                Task::BarClose { symbol, bar },
            );
            sim.push(
                row.close_time_ns,
                priority::CLOSE_EXECUTION,
                tie,
                Task::ExecuteClose { symbol, bar },
            );
            let decision_time = match sim.setup.decision_point {
                DecisionPoint::ClosedBar => Some(row.close_time_ns),
                DecisionPoint::NextBarOpen => Some(row.open_time_ns),
                DecisionPoint::PreClose { offset_ns } => {
                    Some((row.close_time_ns - offset_ns).max(row.open_time_ns))
                }
            };
            if let Some(time_ns) = decision_time {
                sim.push(
                    time_ns,
                    priority::DECISION,
                    tie,
                    Task::Decision { symbol, bar },
                );
            }
            sim.push(
                row.close_time_ns,
                priority::MARK,
                tie,
                Task::Mark { symbol, bar },
            );
        }
    }
}

fn run_task(
    sim: &mut Sim<'_>,
    entry: ScheduledTask,
    strategy: &mut dyn ReferenceStrategy,
) -> Result<(), SimulationError> {
    match entry.task {
        Task::BarOpen { symbol, bar } => {
            sim.opened[symbol] = Some(bar);
            sim.recorder.event(
                entry.time_ns,
                SimEventKind::BarOpen,
                priority::BAR_OPEN,
                Some(SymbolId(symbol)),
                None,
            )?;
        }
        Task::BarClose { symbol, bar } => {
            sim.committed[symbol] = bar + 1;
            sim.marks[symbol] = Some(sim.bar(symbol, bar).close);
            sim.recorder.event(
                entry.time_ns,
                SimEventKind::BarClose,
                priority::BAR_CLOSE,
                Some(SymbolId(symbol)),
                None,
            )?;
        }
        Task::ExecuteOpen { symbol, bar } => execute_phase(sim, symbol, bar, Phase::Open)?,
        Task::ExecuteClose { symbol, bar } => execute_phase(sim, symbol, bar, Phase::Close)?,
        Task::Decision { symbol, bar } => decide(sim, symbol, bar, entry.time_ns, strategy)?,
        Task::Submit { order } => submit_order(sim, order, entry.time_ns)?,
        Task::Activate { order } => {
            if sim.orders[order].state == OrderState::Submitted {
                sim.orders[order].state = OrderState::Active;
                let client_id = sim.orders[order].client_id;
                let symbol = sim.orders[order].symbol;
                sim.recorder.event(
                    entry.time_ns,
                    SimEventKind::OrderActivate,
                    priority::ACTIVATE,
                    Some(symbol),
                    Some(client_id),
                )?;
                schedule_expiry(sim, order, entry.time_ns);
            }
        }
        Task::Expire { order } => {
            if sim.orders[order].state != OrderState::Done {
                sim.cancel_order(
                    order,
                    entry.time_ns,
                    priority::EXPIRE,
                    CancelReason::Expired,
                )?;
            }
        }
        Task::ApplyRequest { request } => apply_request(sim, request, entry.time_ns)?,
        Task::Mark { symbol, bar } => {
            let _ = bar;
            let sequence = sim.recorder.event(
                entry.time_ns,
                SimEventKind::MarkToMarket,
                priority::MARK,
                Some(SymbolId(symbol)),
                None,
            )?;
            let equity = finite_accounting(
                "equity",
                marked_equity(sim.cash, &sim.positions, &sim.marks),
            )?;
            sim.equity_curve.push(EquityPoint {
                time_ns: entry.time_ns,
                sequence,
                cash: sim.cash,
                equity,
            });
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Open,
    Close,
    LegacyClose,
}

fn schedule_expiry(sim: &mut Sim<'_>, order_index: usize, active_time_ns: i64) {
    let expire_at = match sim.orders[order_index].time_in_force {
        TimeInForce::Gtc | TimeInForce::Ioc | TimeInForce::Fok => None,
        TimeInForce::Day => Some(next_utc_day_start(active_time_ns)),
        TimeInForce::Gtd { expire_time_ns } => Some(expire_time_ns),
    };
    if let Some(time_ns) = expire_at {
        sim.schedule_late(
            time_ns.max(active_time_ns),
            priority::EXPIRE,
            Task::Expire { order: order_index },
        );
    }
}

// ── Decisions ──────────────────────────────────────────────────────

fn decide(
    sim: &mut Sim<'_>,
    symbol: usize,
    bar: usize,
    time_ns: i64,
    strategy: &mut dyn ReferenceStrategy,
) -> Result<(), SimulationError> {
    sim.recorder.event(
        time_ns,
        SimEventKind::Decision,
        priority::DECISION,
        Some(SymbolId(symbol)),
        None,
    )?;

    let forming = match sim.setup.decision_point {
        DecisionPoint::PreClose { .. } => {
            let row = sim.bar(symbol, bar);
            Some(FormingBar {
                open_time_ns: row.open_time_ns,
                open: row.open,
                elapsed_ns: time_ns - row.open_time_ns,
            })
        }
        DecisionPoint::ClosedBar | DecisionPoint::NextBarOpen => None,
    };

    let intents = {
        let ctx = DecisionContext {
            symbol: SymbolId(symbol),
            time_ns,
            forming,
            market: MarketView {
                streams: &sim.streams,
                committed: &sim.committed,
                opened: &sim.opened,
            },
        };
        let mut orders = OrderIntents::new(sim.symbol_count, sim.next_client_id);
        strategy
            .on_bar_close(&ctx, &mut orders)
            .map_err(|error| SimulationError::Strategy { time_ns, error })?;
        sim.next_client_id = orders.next_client_id;
        orders.intents
    };

    // Where the order enters the world: immediately, or after the configured
    // number of this symbol's bars have opened.
    let anchor = if sim.setup.submit_delay_bars == 0 {
        Some(time_ns)
    } else {
        let target = bar + sim.setup.submit_delay_bars as usize;
        sim.streams[symbol]
            .bars
            .get(target)
            .map(|row| row.open_time_ns)
    };

    for intent in intents {
        match intent {
            Intent::Submit { client_id, request } => {
                let Some(anchor_time) = anchor else {
                    sim.record_rejection(
                        time_ns,
                        request.symbol,
                        client_id,
                        RejectionReason::SubmitWindowUnavailable {
                            delay_bars: sim.setup.submit_delay_bars,
                        },
                    )?;
                    continue;
                };
                if sim.orders.len() >= MAX_TOTAL_ORDERS {
                    return Err(SimulationError::TooManyOrders {
                        limit: MAX_TOTAL_ORDERS,
                    });
                }
                let (to_submit, to_exchange) =
                    latency_legs(&sim.config.settings().latency, sim.setup.seed, client_id.0);
                let submit_time_ns = anchor_time.saturating_add(to_submit);
                // Submission causally precedes exchange activation even when
                // both configured latency legs are zero. A one-nanosecond
                // logical step prevents a dynamically scheduled ACTIVATE from
                // time-travelling into an earlier priority slot at the same
                // timestamp.
                let legacy_same_close = sim.config.settings().compatibility
                    == ExecutionCompatibility::LegacySameBarClose;
                let active_time_ns = if legacy_same_close {
                    submit_time_ns.saturating_add(to_exchange)
                } else {
                    submit_time_ns
                        .saturating_add(to_exchange)
                        .max(submit_time_ns.saturating_add(1))
                };
                let order_index = sim.orders.len();
                sim.orders.push(Order {
                    client_id,
                    symbol: request.symbol,
                    side: request.side,
                    quantity: request.quantity,
                    kind: request.kind,
                    time_in_force: request.time_in_force,
                    reduce_only: request.reduce_only,
                    oco_group: request.oco_group,
                    submit_time_ns,
                    active_time_ns,
                    submit_sequence: 0,
                    state: OrderState::Submitted,
                    had_opportunity: false,
                });
                let submit_time = sim.orders[order_index].submit_time_ns;
                if legacy_same_close {
                    submit_order(sim, order_index, submit_time)?;
                } else {
                    sim.schedule_late(
                        submit_time,
                        priority::SUBMIT,
                        Task::Submit { order: order_index },
                    );
                }
            }
            Intent::Cancel { target } | Intent::Modify { target, .. } => {
                let change = match intent {
                    Intent::Modify { change, .. } => Some(change),
                    _ => None,
                };
                let Some(anchor_time) = anchor else {
                    sim.record_rejection(
                        time_ns,
                        SymbolId(symbol),
                        target,
                        RejectionReason::SubmitWindowUnavailable {
                            delay_bars: sim.setup.submit_delay_bars,
                        },
                    )?;
                    continue;
                };
                // Requests ride the same wire as orders, so they arrive with
                // the same delay. Their RNG stream is offset past the order id
                // space so a request never draws an order's delay.
                let stream = u64::MAX - sim.requests.len() as u64;
                let (to_submit, to_exchange) =
                    latency_legs(&sim.config.settings().latency, sim.setup.seed, stream);
                let request_index = sim.requests.len();
                sim.requests.push(PendingRequest {
                    symbol: SymbolId(symbol),
                    target,
                    change,
                });
                let arrival_time = anchor_time
                    .saturating_add(to_submit)
                    .saturating_add(to_exchange)
                    .max(time_ns.saturating_add(1));
                sim.schedule_late(
                    arrival_time,
                    priority::REQUEST,
                    Task::ApplyRequest {
                        request: request_index,
                    },
                );
            }
        }
    }
    if sim.config.settings().compatibility == ExecutionCompatibility::LegacySameBarClose {
        execute_phase(sim, symbol, bar, Phase::LegacyClose)?;
    }
    Ok(())
}

fn submit_order(
    sim: &mut Sim<'_>,
    order_index: usize,
    time_ns: i64,
) -> Result<(), SimulationError> {
    let (client_id, symbol, kind) = {
        let order = &sim.orders[order_index];
        (order.client_id, order.symbol, order.kind)
    };
    let sequence = sim.recorder.event(
        time_ns,
        SimEventKind::OrderSubmit,
        priority::SUBMIT,
        Some(symbol),
        Some(client_id),
    )?;
    sim.orders[order_index].submit_sequence = sequence;

    let committed = sim.committed[symbol.0];
    let required = sim.config.settings().warmup_bars;
    if committed < required as usize {
        sim.record_rejection(
            time_ns,
            symbol,
            client_id,
            RejectionReason::WarmupIncomplete {
                committed,
                required,
            },
        )?;
        sim.orders[order_index].state = OrderState::Done;
        return Ok(());
    }
    if let Some(tick) = sim.config.settings().price_tick {
        for price in order_prices(&kind) {
            if !on_tick(price, tick) {
                sim.record_rejection(
                    time_ns,
                    symbol,
                    client_id,
                    RejectionReason::PriceOffTick { price, tick },
                )?;
                sim.orders[order_index].state = OrderState::Done;
                return Ok(());
            }
        }
    }
    if sim.live[symbol.0].len() >= MAX_LIVE_ORDERS_PER_SYMBOL {
        return Err(SimulationError::TooManyLiveOrders {
            symbol: sim.streams[symbol.0].symbol.clone(),
            limit: MAX_LIVE_ORDERS_PER_SYMBOL,
        });
    }
    sim.live[symbol.0].push(order_index);
    let active_time = sim.orders[order_index].active_time_ns;
    if sim.config.settings().compatibility == ExecutionCompatibility::LegacySameBarClose {
        sim.orders[order_index].state = OrderState::Active;
        sim.recorder.event(
            active_time,
            SimEventKind::OrderActivate,
            priority::SUBMIT,
            Some(symbol),
            Some(client_id),
        )?;
        schedule_expiry(sim, order_index, active_time);
    } else {
        sim.schedule_late(
            active_time,
            priority::ACTIVATE,
            Task::Activate { order: order_index },
        );
    }
    Ok(())
}

/// Whether `price` sits on the `tick` lattice, within the rounding noise of a
/// twelve-decimal representation.
fn on_tick(price: f64, tick: f64) -> bool {
    if !tick.is_finite() || tick <= 0.0 {
        return true;
    }
    let steps = (price / tick).round();
    (price - steps * tick).abs() <= tick * 1.0e-9
}

fn apply_request(
    sim: &mut Sim<'_>,
    request_index: usize,
    time_ns: i64,
) -> Result<(), SimulationError> {
    let request = sim.requests[request_index].clone();
    let found = sim
        .orders
        .iter()
        .position(|order| order.client_id == request.target && order.state != OrderState::Done);
    let Some(order_index) = found else {
        return sim.record_rejection(
            time_ns,
            request.symbol,
            request.target,
            RejectionReason::UnknownOrder,
        );
    };
    let Some(change) = request.change else {
        return sim.cancel_order(
            order_index,
            time_ns,
            priority::REQUEST,
            CancelReason::Requested,
        );
    };

    let mut kind = sim.orders[order_index].kind;
    let quantity = change.quantity.unwrap_or(sim.orders[order_index].quantity);
    let mut applicable = true;
    if let Some(limit_price) = change.limit_price {
        match &mut kind {
            OrderKind::Limit { limit_price: value }
            | OrderKind::StopLimit {
                limit_price: value, ..
            } => *value = limit_price,
            _ => applicable = false,
        }
    }
    if let Some(stop_price) = change.stop_price {
        match &mut kind {
            OrderKind::Stop { stop_price: value }
            | OrderKind::StopLimit {
                stop_price: value, ..
            } => *value = stop_price,
            _ => applicable = false,
        }
    }
    if !applicable
        || (change.quantity.is_none()
            && change.limit_price.is_none()
            && change.stop_price.is_none())
    {
        return sim.record_rejection(
            time_ns,
            request.symbol,
            request.target,
            RejectionReason::ModifyNotApplicable,
        );
    }
    if let Some(tick) = sim.config.settings().price_tick {
        for price in order_prices(&kind) {
            if !on_tick(price, tick) {
                return sim.record_rejection(
                    time_ns,
                    request.symbol,
                    request.target,
                    RejectionReason::PriceOffTick { price, tick },
                );
            }
        }
    }
    sim.orders[order_index].quantity = quantity;
    sim.orders[order_index].kind = kind;
    let client_id = sim.orders[order_index].client_id;
    sim.recorder.event(
        time_ns,
        SimEventKind::OrderModify,
        priority::REQUEST,
        Some(request.symbol),
        Some(client_id),
    )?;
    Ok(())
}

// ── Execution ──────────────────────────────────────────────────────

fn execute_phase(
    sim: &mut Sim<'_>,
    symbol: usize,
    bar: usize,
    phase: Phase,
) -> Result<(), SimulationError> {
    if sim.live[symbol].is_empty() {
        return Ok(());
    }
    let row = sim.bar(symbol, bar);
    let fidelity = sim.config.settings().fidelity;
    let legacy = sim.config.settings().compatibility == ExecutionCompatibility::LegacySameBarClose;

    // Range-based orders had to be live by the open to claim the bar's path.
    // MOC consumes only the close, so it may become active during the bar.
    let eligible: Vec<usize> = sim.live[symbol]
        .iter()
        .copied()
        .filter(|index| {
            let order = &sim.orders[*index];
            order.state == OrderState::Active
                && (legacy
                    || (matches!(phase, Phase::Close)
                        && matches!(order.kind, OrderKind::MarketOnClose)
                        && order.active_time_ns <= row.close_time_ns)
                    || order.active_time_ns <= row.open_time_ns)
        })
        .collect();
    if eligible.is_empty() {
        return Ok(());
    }

    let mut candidates: Vec<(usize, Trigger)> = Vec::new();
    let mut converted: Vec<usize> = Vec::new();
    for index in eligible.iter().copied() {
        let order = sim.orders[index].clone();
        let half = sim.half_spread(row.open);
        let resting_limit = match (phase, fidelity) {
            (Phase::Open, _) => {
                resting_stop_limit_at_boundary(order.kind, order.side, row.open, half)
            }
            (Phase::Close, FidelityLevel::BarOhlc) => {
                resting_stop_limit_in_bar(order.kind, order.side, &row, half)
            }
            (Phase::Close | Phase::LegacyClose, FidelityLevel::BarClose) => {
                resting_stop_limit_at_boundary(order.kind, order.side, row.close, half)
            }
            (Phase::LegacyClose, FidelityLevel::BarOhlc) => None,
        };
        if resting_limit.is_some() {
            converted.push(index);
            continue;
        }
        let trigger = match (phase, fidelity) {
            (Phase::Open, _) => boundary_trigger(order.kind, order.side, row.open, half),
            (Phase::LegacyClose, _) => boundary_trigger(
                order.kind,
                order.side,
                row.close,
                sim.half_spread(row.close),
            ),
            (Phase::Close, _) => {
                if matches!(order.kind, OrderKind::MarketOnClose) {
                    Some(Trigger {
                        rank: 0,
                        mid: row.close,
                        marketable: true,
                    })
                } else if fidelity == FidelityLevel::BarOhlc {
                    intrabar_trigger(order.kind, order.side, &row, half).map(|(trigger, _)| trigger)
                } else if !matches!(order.kind, OrderKind::Market) {
                    boundary_trigger(order.kind, order.side, row.close, half)
                } else {
                    None
                }
            }
        };
        if let Some(trigger) = trigger {
            candidates.push((index, trigger));
        }
    }

    for index in converted {
        if let OrderKind::StopLimit { limit_price, .. } = sim.orders[index].kind {
            sim.orders[index].kind = OrderKind::Limit { limit_price };
            let (client_id, order_symbol) = {
                let order = &sim.orders[index];
                (order.client_id, order.symbol)
            };
            let (trigger_time, trigger_priority) = match phase {
                Phase::Open => (row.open_time_ns, priority::OPEN_EXECUTION),
                Phase::Close | Phase::LegacyClose => (row.close_time_ns, priority::CLOSE_EXECUTION),
            };
            sim.recorder.event(
                trigger_time,
                SimEventKind::StopTriggered,
                trigger_priority,
                Some(order_symbol),
                Some(client_id),
            )?;
        }
    }

    resolve_oco(sim, &mut candidates);
    candidates.sort_by_key(|(index, _)| sim.orders[*index].submit_sequence);

    let time_ns = match phase {
        Phase::Open => row.open_time_ns,
        Phase::Close | Phase::LegacyClose => row.close_time_ns,
    };
    let execution_priority = match phase {
        Phase::Open => priority::OPEN_EXECUTION,
        Phase::Close => priority::CLOSE_EXECUTION,
        Phase::LegacyClose => priority::SUBMIT,
    };

    for (index, trigger) in candidates {
        if sim.orders[index].state != OrderState::Active {
            continue;
        }
        attempt_fill(sim, index, trigger, time_ns, execution_priority, row)?;
    }

    // An immediate-or-cancel order gets exactly one bar to work in.
    let expiring: Vec<usize> = sim.live[symbol]
        .iter()
        .copied()
        .filter(|index| {
            let order = &sim.orders[*index];
            matches!(order.time_in_force, TimeInForce::Ioc | TimeInForce::Fok)
                && order.state == OrderState::Active
                && order.active_time_ns <= row.open_time_ns
                && (matches!(phase, Phase::Close | Phase::LegacyClose) || order.had_opportunity)
        })
        .collect();
    for index in expiring {
        sim.cancel_order(index, time_ns, execution_priority, CancelReason::Expired)?;
    }
    for index in eligible {
        if sim.orders[index].state == OrderState::Active {
            sim.orders[index].had_opportunity = true;
        }
    }
    Ok(())
}

/// Decide which leg of each bracket executes when more than one is reachable
/// in the same bar (§6.1).
fn resolve_oco(sim: &Sim<'_>, candidates: &mut Vec<(usize, Trigger)>) {
    let policy = sim.config.settings().ambiguity;
    let groups: Vec<u32> = {
        let mut seen: Vec<u32> = candidates
            .iter()
            .filter_map(|(index, _)| sim.orders[*index].oco_group)
            .collect();
        seen.sort_unstable();
        seen.dedup();
        seen
    };
    for group in groups {
        let members: Vec<usize> = candidates
            .iter()
            .enumerate()
            .filter(|(_, (index, _))| sim.orders[*index].oco_group == Some(group))
            .map(|(position, _)| position)
            .collect();
        if members.len() < 2 {
            continue;
        }
        let gapped: Vec<usize> = members
            .iter()
            .copied()
            .filter(|position| candidates[*position].1.rank == 0)
            .collect();
        // A level the bar gapped through at its open resolves before any
        // assumption about the path is needed.
        let winner = if gapped.len() == 1 {
            gapped[0]
        } else {
            let pool = if gapped.is_empty() { &members } else { &gapped };
            match policy {
                // Two levels gapped through at once, or a policy that names a
                // role no member has, both fall back to the pessimistic
                // stop-first reading and then to submission order.
                OhlcAmbiguityPolicy::StopFirst => {
                    pick_role(sim, candidates, pool, OrderKind::is_stop_like)
                }
                OhlcAmbiguityPolicy::TargetFirst if gapped.is_empty() => {
                    pick_role(sim, candidates, pool, OrderKind::is_limit_like)
                }
                OhlcAmbiguityPolicy::TargetFirst => {
                    pick_role(sim, candidates, pool, OrderKind::is_stop_like)
                }
                OhlcAmbiguityPolicy::OhlcPath => {
                    let best = pool
                        .iter()
                        .map(|position| candidates[*position].1.rank)
                        .min()
                        .unwrap_or(0);
                    let tied: Vec<usize> = pool
                        .iter()
                        .copied()
                        .filter(|position| candidates[*position].1.rank == best)
                        .collect();
                    pick_role(sim, candidates, &tied, OrderKind::is_stop_like)
                }
            }
        };
        let keep = candidates[winner].0;
        candidates
            .retain(|(index, _)| sim.orders[*index].oco_group != Some(group) || *index == keep);
    }
}

/// The member matching `role`, or — when none does — the one submitted first.
fn pick_role(
    sim: &Sim<'_>,
    candidates: &[(usize, Trigger)],
    pool: &[usize],
    role: fn(OrderKind) -> bool,
) -> usize {
    pool.iter()
        .copied()
        .find(|position| role(sim.orders[candidates[*position].0].kind))
        .unwrap_or_else(|| {
            pool.iter()
                .copied()
                .min_by_key(|position| sim.orders[candidates[*position].0].submit_sequence)
                .unwrap_or(pool[0])
        })
}

fn attempt_fill(
    sim: &mut Sim<'_>,
    order_index: usize,
    trigger: Trigger,
    time_ns: i64,
    execution_priority: u8,
    row: SimBar,
) -> Result<(), SimulationError> {
    let order = sim.orders[order_index].clone();
    let settings = sim.config.settings();

    let reference = trigger.mid;
    let width = finite_accounting(
        "spread_width",
        stable_decimal(spread_width(
            &settings.spread,
            reference_for_width(&order, &row),
        )),
    )?;
    let quoted = finite_accounting(
        "quoted_price",
        stable_decimal(reference + order.side.sign() * width / 2.0),
    )?;
    let slip = if trigger.marketable {
        finite_accounting(
            "slippage",
            stable_decimal(slippage_distance(&settings.slippage, width)),
        )?
    } else {
        0.0
    };
    let mut fill_price = finite_accounting(
        "fill_price",
        stable_decimal(quoted + order.side.sign() * slip),
    )?;
    // A limit never fills worse than its price — that is what a limit is.
    if let OrderKind::Limit { limit_price } = order.kind {
        fill_price = match order.side {
            OrderSide::Buy => fill_price.min(limit_price),
            OrderSide::Sell => fill_price.max(limit_price),
        };
    }

    let position = sim.positions[order.symbol.0].units;
    if order.reduce_only {
        let reduces = position != 0.0 && position.signum() != order.side.sign();
        if !reduces {
            sim.record_rejection(
                time_ns,
                order.symbol,
                order.client_id,
                RejectionReason::ReduceOnlyWouldNotReduce,
            )?;
            sim.retire(order_index);
            return Ok(());
        }
        if order.quantity > position.abs() {
            sim.record_rejection(
                time_ns,
                order.symbol,
                order.client_id,
                RejectionReason::ReduceOnlyExceedsPosition {
                    position,
                    quantity: order.quantity,
                },
            )?;
            sim.retire(order_index);
            return Ok(());
        }
    }

    let fee = finite_accounting(
        "commission",
        stable_decimal(commission(
            &settings.commission,
            order.side,
            order.quantity,
            fill_price,
        )),
    )?;
    let next_cash = finite_accounting(
        "cash",
        stable_decimal(sim.cash - order.side.sign() * fill_price * order.quantity - fee),
    )?;
    let next_units = position + order.side.sign() * order.quantity;
    if settings.margin == MarginPolicy::CashOnly {
        if next_units < 0.0 {
            sim.record_rejection(
                time_ns,
                order.symbol,
                order.client_id,
                RejectionReason::ShortNotPermitted,
            )?;
            sim.retire(order_index);
            return Ok(());
        }
        if next_cash < 0.0 {
            sim.record_rejection(
                time_ns,
                order.symbol,
                order.client_id,
                RejectionReason::InsufficientBuyingPower {
                    cash: sim.cash,
                    required: stable_decimal(fill_price * order.quantity + fee),
                },
            )?;
            sim.retire(order_index);
            return Ok(());
        }
    }

    let spread_cost =
        finite_accounting("spread_cost", stable_decimal(width / 2.0 * order.quantity))?;
    let slippage_cost = finite_accounting("slippage_cost", stable_decimal(slip * order.quantity))?;
    sim.cash = next_cash;
    sim.total_commission = finite_accounting(
        "total_commission",
        stable_decimal(sim.total_commission + fee),
    )?;
    let realized_pnl = apply_fill(
        &mut sim.positions[order.symbol.0],
        order.side,
        order.quantity,
        fill_price,
    );
    finite_accounting("realized_pnl", realized_pnl)?;
    finite_accounting("position_units", sim.positions[order.symbol.0].units)?;
    finite_accounting("average_entry", sim.positions[order.symbol.0].avg_entry)?;
    finite_accounting(
        "position_realized_pnl",
        sim.positions[order.symbol.0].realized_pnl,
    )?;

    let sequence = sim.recorder.event(
        time_ns,
        SimEventKind::Fill,
        execution_priority,
        Some(order.symbol),
        Some(order.client_id),
    )?;
    let state = sim.positions[order.symbol.0].clone();
    sim.fills.push(FillRecord {
        order_id: order.client_id,
        time_ns,
        sequence,
        symbol: order.symbol,
        side: order.side,
        quantity: order.quantity,
        reference_price: reference,
        quoted_price: quoted,
        fill_price,
        spread_cost,
        slippage_cost,
        commission: fee,
        realized_pnl,
        cash_after: sim.cash,
        position_units_after: state.units,
        avg_entry_after: state.avg_entry,
    });
    sim.retire(order_index);

    if let Some(group) = order.oco_group {
        let siblings: Vec<usize> = sim.live[order.symbol.0]
            .iter()
            .copied()
            .filter(|index| sim.orders[*index].oco_group == Some(group))
            .collect();
        for sibling in siblings {
            sim.cancel_order(
                sibling,
                time_ns,
                priority::OCO_CANCEL,
                CancelReason::OcoSibling,
            )?;
        }
    }
    Ok(())
}

/// The price the spread width is derived from: the bar's own open for every
/// execution inside it, except market-on-close, which executes at the close.
const fn reference_for_width(order: &Order, row: &SimBar) -> f64 {
    match order.kind {
        OrderKind::MarketOnClose => row.close,
        _ => row.open,
    }
}

// ── Finish ─────────────────────────────────────────────────────────

fn finish(mut sim: Sim<'_>) -> Result<SimulationReport, SimulationError> {
    let stream_end_ns = sim
        .streams
        .iter()
        .filter_map(|stream| stream.bars.last().map(|bar| bar.close_time_ns))
        .max()
        .unwrap_or(0);
    let final_time_ns =
        if sim.config.settings().compatibility == ExecutionCompatibility::LegacySameBarClose {
            let liquidation_time = stream_end_ns.saturating_add(1);
            for symbol_index in 0..sim.positions.len() {
                let units = sim.positions[symbol_index].units;
                if units == 0.0 {
                    continue;
                }
                let row = *sim.streams[symbol_index]
                    .bars
                    .last()
                    .expect("validated streams are non-empty");
                let side = if units > 0.0 {
                    OrderSide::Sell
                } else {
                    OrderSide::Buy
                };
                let client_id = ClientOrderId(sim.next_client_id);
                sim.next_client_id += 1;
                let order_index = sim.orders.len();
                sim.orders.push(Order {
                    client_id,
                    symbol: SymbolId(symbol_index),
                    side,
                    quantity: units.abs(),
                    kind: OrderKind::Market,
                    time_in_force: TimeInForce::Day,
                    reduce_only: true,
                    oco_group: None,
                    submit_time_ns: liquidation_time,
                    active_time_ns: liquidation_time,
                    submit_sequence: 0,
                    state: OrderState::Active,
                    had_opportunity: true,
                });
                sim.live[symbol_index].push(order_index);
                attempt_fill(
                    &mut sim,
                    order_index,
                    Trigger {
                        rank: 0,
                        mid: row.close,
                        marketable: true,
                    },
                    liquidation_time,
                    priority::CLOSE_EXECUTION,
                    row,
                )?;
            }
            liquidation_time
        } else {
            stream_end_ns
        };
    let final_mark_sequence = sim.recorder.event(
        final_time_ns,
        SimEventKind::MarkToMarket,
        priority::MARK,
        None,
        None,
    )?;
    let final_equity = finite_accounting(
        "final_equity",
        marked_equity(sim.cash, &sim.positions, &sim.marks),
    )?;
    sim.equity_curve.push(EquityPoint {
        time_ns: final_time_ns,
        sequence: final_mark_sequence,
        cash: sim.cash,
        equity: final_equity,
    });

    let positions = sim
        .positions
        .iter()
        .enumerate()
        .map(|(index, position)| {
            let mark_price = sim.marks[index];
            let unrealized_pnl = stable_decimal(mark_price.map_or(0.0, |mark| {
                if position.units >= 0.0 {
                    (mark - position.avg_entry) * position.units
                } else {
                    (position.avg_entry - mark) * -position.units
                }
            }));
            PositionRecord {
                symbol: SymbolId(index),
                units: position.units,
                avg_entry: position.avg_entry,
                realized_pnl: position.realized_pnl,
                mark_price,
                unrealized_pnl,
            }
        })
        .collect::<Vec<_>>();

    let mut pending: Vec<PendingOrderRecord> = sim
        .orders
        .iter()
        .filter(|order| order.state != OrderState::Done)
        .map(|order| PendingOrderRecord {
            order_id: order.client_id,
            submitted_time_ns: order.submit_time_ns,
            symbol: order.symbol,
            side: order.side,
            quantity: order.quantity,
            kind: order.kind,
            time_in_force: order.time_in_force,
        })
        .collect();
    pending.sort_by_key(|record| record.order_id);

    let final_realized_pnl = finite_accounting(
        "final_realized_pnl",
        sim.positions
            .iter()
            .map(|position| position.realized_pnl)
            .sum(),
    )?;

    Ok(SimulationReport {
        symbols: sim
            .streams
            .into_iter()
            .map(|stream| stream.symbol)
            .collect(),
        events: sim.recorder.events,
        fills: sim.fills,
        rejections: sim.rejections,
        cancellations: sim.cancellations,
        pending_orders: pending,
        positions,
        equity_curve: sim.equity_curve,
        final_cash: sim.cash,
        final_equity,
        final_realized_pnl,
        total_commission: sim.total_commission,
    })
}

#[cfg(test)]
mod tests;
