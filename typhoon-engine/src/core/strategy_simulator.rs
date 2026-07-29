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
//! | 1  | `CorporateAction` — a split, dividend, rename or delisting takes effect |
//! | 2  | `OrderActivate` — an order reaches the exchange |
//! | 3  | `OrderCancel` / `OrderModify` — a request takes effect |
//! | 4  | `OrderExpire` — a time-in-force runs out |
//! | 5  | `StopTriggered`, `Fill`, `PartialFill` — execution against the open or the intrabar path |
//! | 6  | `BarClose` — the bar commits and becomes visible history |
//! | 7  | `Fill` — execution at the close (market-on-close, bar-close fidelity) |
//! | 8  | `OrderCancel` — an OCO sibling withdrawn because its partner filled |
//! | 9  | `Decision` — the strategy runs |
//! | 10 | `OrderSubmit` — an intent becomes an order |
//! | 11 | `OrderReject` — an order or request is refused |
//! | 12 | `FundingCharge` — a financing/borrow/funding accrual boundary |
//! | 13 | `MarkToMarket` — equity is sampled |
//!
//! A corporate action sits directly after the bar opens and before anything
//! executes, because a split that resized the position must have resized it
//! before a stop set in the old prices is tested. An accrual sits directly
//! before the mark, so the equity sample already carries the charge.
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
//! - `SubBar` — a finer timeframe supplies the path. Each sub-bar is resolved
//!   in time order under the `BarOhlc` rules, so when a stop and a target are
//!   both reachable inside the parent bar the *earlier sub-bar wins* and the
//!   ambiguity policy is consulted only for a tie inside one sub-bar. A sub-bar
//!   is also its own eligibility boundary, so an order that arrived mid-bar can
//!   claim the sub-bars that started after it.
//!
//! A level the bar *gapped through at its open* always resolves before the
//! ambiguity policy is consulted: the first observable price of the bar already
//! went through it, so no assumption about the path is needed.
//!
//! ## What a fill may take
//!
//! A [`ParticipationModel`] caps how much of a bar's traded volume one run may
//! consume. The cap belongs to the **parent bar** and is shared by every
//! execution inside it — both phases and every sub-bar — so splitting one order
//! into ten cannot route around it. What the cap refuses is not lost: the order
//! keeps its remainder and re-attempts on the next bar, unless its
//! time-in-force says otherwise. A fill-or-kill that cannot take its whole size
//! is cancelled rather than partially filled.
//!
//! ## Sessions, corporate actions, and what a position costs to hold
//!
//! An instrument may carry a [`TradingCalendar`]. Nothing fills while its venue
//! is closed, and an out-of-session submission either queues or is rejected per
//! [`OutsideSessionPolicy`] — never silently filled. Splits, dividends, symbol
//! changes and delistings arrive as events at their effective instant and
//! adjust the position and cash that exist then, rather than by rewriting price
//! history underneath a live stop. Financing, borrow and funding accrue at
//! declared boundaries; a rate the policy calls unavailable **fails the run**
//! instead of accruing zero.
//!
//! ## Rejections
//!
//! An order that cannot be executed is refused and recorded — warm-up not
//! complete, price off the tick lattice, reduce-only that would not reduce,
//! insufficient buying power, a short in a cash account, a bar-delayed
//! submission that ran off the end of the stream, a request naming an order
//! that no longer exists. None of these is a silent drop.

use crate::core::strategy_calendar::{SessionStatus, TradingCalendar};
use crate::core::strategy_corporate::{CorporateAction, CorporateActionKind, FractionalUnitPolicy};
use crate::core::strategy_financing::{AccrualBreakdown, FinancingCharge, FinancingPolicy, accrue};
use crate::core::strategy_interpreter::{CanonicalIrStrategy, InterpreterError};
use crate::core::strategy_intervention::{HybridReplay, InterventionError, InterventionLog};
use crate::core::strategy_ir::{
    CommissionModel, DecisionTiming, ExecutionCompatibility, FeeSide, FidelityLevel, LatencyModel,
    MAX_PRE_CLOSE_OFFSET_SECONDS, MAX_SUBMIT_DELAY_BARS, MarginPolicy, OhlcAmbiguityPolicy,
    OutsideSessionPolicy, SlippageModel, SpreadModel, StrategyExecutionConfig, StrategyIrError,
    TieBreakPolicy,
};
use crate::core::strategy_run::VerifiedRun;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
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
/// Sub-bars one run may carry across every symbol (§6.9 level 3). Bounds the
/// prebuilt clock exactly as [`MAX_TOTAL_BARS`] does for the execution stream.
pub const MAX_TOTAL_SUB_BARS: usize = 2_000_000;
/// Accrual boundaries one run may schedule (§6.3). A run whose financing
/// interval is so short that it would exceed this is refused rather than
/// silently truncated.
pub const MAX_ACCRUAL_BOUNDARIES: usize = 200_000;

const NANOS_PER_DAY: i64 = 86_400_000_000_000;
const NANOS_PER_SECOND: i64 = 1_000_000_000;

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

/// The finer-timeframe path for one symbol at [`FidelityLevel::SubBar`].
///
/// Kept beside the execution stream rather than inside it, because a sub-bar
/// path is a property of the *run's fidelity*, not of the instrument: the same
/// stream is a complete input at levels 1 and 2 and would carry a dead field
/// there.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubBarPath {
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
    /// A fill that took less than the order's remaining quantity because the
    /// participation cap refused the rest (§6.6).
    PartialFill,
    BarClose,
    Decision,
    OrderSubmit,
    OrderReject,
    /// A split, dividend, symbol change or delisting took effect (§6.8).
    CorporateAction,
    /// A financing, borrow or funding accrual boundary (§6.3).
    FundingCharge,
    MarkToMarket,
}

/// Explicit ordering slots. The same kind can occur in two phases of a bar, so
/// the phase — not the kind — decides the order.
mod priority {
    // Keep the M1 priority values stable: they are serialized ledger semantics.
    // Richer-execution events share the adjacent legacy slot where no integer
    // exists between phases; task insertion order is canonical, and sequence is
    // the final deterministic tie-break.
    pub const BAR_OPEN: u8 = 0;
    pub const CORPORATE: u8 = 1;
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
    pub const ACCRUAL: u8 = 11;
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
    /// Size this execution took. Under a participation cap it may be less than
    /// the order asked for; `remaining_quantity` carries what is still working.
    pub quantity: f64,
    /// Order quantity still live after this execution. Zero for a complete
    /// fill, positive for a partial one.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub remaining_quantity: f64,
    /// The mid the execution was resolved against.
    pub reference_price: f64,
    /// The reference plus the half-spread on the taker's side.
    pub quoted_price: f64,
    pub fill_price: f64,
    pub spread_cost: f64,
    pub slippage_cost: f64,
    pub commission: f64,
    /// Account-currency units per unit of the instrument's quote currency.
    /// `1.0` when the instrument is already quoted in the account currency.
    #[serde(
        default = "identity_conversion_rate",
        skip_serializing_if = "is_identity_conversion_rate"
    )]
    pub conversion_rate: f64,
    /// Currency-conversion cost charged on this fill, in account currency.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub conversion_cost: f64,
    /// Realized profit in **account** currency.
    pub realized_pnl: f64,
    pub cash_after: f64,
    pub position_units_after: f64,
    /// Volume-weighted entry in the **instrument's** quote currency — it is a
    /// price, and converting it would make it comparable to nothing.
    pub avg_entry_after: f64,
}

/// One accrual boundary's charges for one symbol (§6.3), in account currency.
/// Positive is a debit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FinancingChargeRecord {
    pub time_ns: i64,
    pub sequence: u64,
    pub symbol: SymbolId,
    /// Signed units held across the boundary.
    pub units: f64,
    /// Mark the charge was computed against.
    pub mark_price: f64,
    pub seconds_accrued: i64,
    pub financing: f64,
    pub borrow: f64,
    pub funding: f64,
    pub total: f64,
    pub cash_after: f64,
}

/// One applied corporate action (§6.8).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CorporateActionRecord {
    pub time_ns: i64,
    pub sequence: u64,
    pub symbol: SymbolId,
    /// Stable action tag — `split`, `cash_dividend`, `symbol_change`,
    /// `delisting`.
    pub kind: String,
    pub units_before: f64,
    pub units_after: f64,
    pub avg_entry_before: f64,
    pub avg_entry_after: f64,
    /// Cash the action paid (positive) or charged (negative), account currency.
    pub cash_delta: f64,
    /// Resting orders withdrawn because their prices no longer mean what they
    /// meant.
    pub orders_cancelled: usize,
}

/// Why an order or request was refused.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// The instrument's calendar says the venue is closed and the run's policy
    /// is to refuse rather than queue (§6.7).
    SessionClosed { reason: String },
    /// The symbol stopped trading at a delisting event (§6.8).
    InstrumentDelisted,
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
    /// Fill-or-kill: the participation cap could not supply the whole size, so
    /// nothing was taken (§6.6).
    FillOrKillUnfilled,
    /// A bracket sibling filled part of its size, leaving this order with
    /// nothing left to protect (§6.6).
    OcoSiblingConsumed,
    /// A corporate action changed what the order's prices mean (§6.8).
    CorporateAction,
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
    /// Quantity already taken by partial fills, so a pending record shows what
    /// is genuinely still working rather than the original size.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub filled_quantity: f64,
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
    /// Accrual boundaries that charged something (§6.3), in run order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub financing_charges: Vec<FinancingChargeRecord>,
    /// Corporate actions applied (§6.8), in effective-time order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub corporate_actions: Vec<CorporateActionRecord>,
    pub final_cash: f64,
    pub final_equity: f64,
    pub final_realized_pnl: f64,
    pub total_commission: f64,
    /// Total currency-conversion cost charged across every fill.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub total_conversion_cost: f64,
    /// Total financing + borrow + funding charged across every accrual.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub total_financing_cost: f64,
}

const fn identity_conversion_rate() -> f64 {
    1.0
}

fn is_identity_conversion_rate(value: &f64) -> bool {
    *value == identity_conversion_rate()
}

fn is_zero(value: &f64) -> bool {
    *value == 0.0
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
    /// Sub-bar fidelity was selected but a symbol has no finer path.
    MissingSubBarPath {
        symbol: String,
    },
    /// A sub-bar path was supplied for a symbol the run does not trade.
    UnknownSubBarSymbol {
        symbol: String,
    },
    /// A sub-bar path was supplied at a fidelity that does not consume one.
    UnexpectedSubBarPath {
        symbol: String,
    },
    TooManySubBars {
        limit: usize,
        found: usize,
    },
    /// A sub-bar does not fit inside the parent bar it claims — in time or in
    /// price. Accepting it would let a fill happen at a price the execution
    /// stream never printed.
    SubBarNotContained {
        symbol: String,
        index: usize,
    },
    /// A parent interval is not fully tiled by consecutive sub-bars.
    SubBarGap {
        symbol: String,
        parent_index: usize,
        expected_open_time_ns: i64,
        actual_open_time_ns: Option<i64>,
    },
    /// Two consecutive sub-bars claim the same part of a parent interval.
    SubBarOverlap {
        symbol: String,
        parent_index: usize,
        previous_index: usize,
        index: usize,
    },
    /// A path bar's half-open duration disagrees with the run's declared
    /// `sub_bar_seconds` fidelity.
    SubBarDurationMismatch {
        symbol: String,
        index: usize,
        expected_ns: i64,
        actual_ns: i64,
    },
    /// More than one path was supplied for the same symbol.
    DuplicateSubBarPath {
        symbol: String,
    },
    /// A corporate action names a symbol the run does not trade.
    UnknownCorporateActionSymbol {
        symbol: String,
    },
    /// A delisting fell before any bar of the symbol had closed, so there is no
    /// mark to cash the position out at.
    DelistingWithoutMark {
        symbol: String,
    },
    /// A symbol-change event does not match the stream it names.
    SymbolChangeMismatch {
        symbol: String,
    },
    /// A split left a fractional unit under a policy that refuses to model one.
    FractionalUnitsRefused {
        symbol: String,
        units: f64,
    },
    /// Cash in lieu was declared, but the split fell before any bar of the
    /// symbol had closed, so the declared price rule has no price to read.
    CashInLieuWithoutMark {
        symbol: String,
    },
    /// A position would accrue a charge whose rate the policy calls
    /// unavailable. The run fails rather than accruing zero (§6.3, §14).
    FinancingRateUnavailable {
        symbol: String,
        charge: &'static str,
    },
    TooManyAccrualBoundaries {
        limit: usize,
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
            Self::MissingSubBarPath { symbol } => {
                write!(
                    f,
                    "symbol `{symbol}` has no sub-bar path for sub-bar fidelity"
                )
            }
            Self::UnknownSubBarSymbol { symbol } => {
                write!(f, "sub-bar path names unknown symbol `{symbol}`")
            }
            Self::UnexpectedSubBarPath { symbol } => write!(
                f,
                "sub-bar path for `{symbol}` supplied at a fidelity that does not use one"
            ),
            Self::TooManySubBars { limit, found } => {
                write!(f, "simulation has {found} sub-bars, limit {limit}")
            }
            Self::SubBarNotContained { symbol, index } => write!(
                f,
                "symbol `{symbol}` sub-bar {index} is not contained in a parent bar"
            ),
            Self::SubBarGap {
                symbol,
                parent_index,
                expected_open_time_ns,
                actual_open_time_ns,
            } => write!(
                f,
                "symbol `{symbol}` parent bar {parent_index} has a sub-bar gap: expected next open {expected_open_time_ns}, got {}",
                actual_open_time_ns
                    .map(|time| time.to_string())
                    .unwrap_or_else(|| "end of path".to_string())
            ),
            Self::SubBarOverlap {
                symbol,
                parent_index,
                previous_index,
                index,
            } => write!(
                f,
                "symbol `{symbol}` sub-bars {previous_index} and {index} overlap while tiling parent bar {parent_index}"
            ),
            Self::SubBarDurationMismatch {
                symbol,
                index,
                expected_ns,
                actual_ns,
            } => write!(
                f,
                "symbol `{symbol}` sub-bar {index} duration is {actual_ns}ns, expected {expected_ns}ns"
            ),
            Self::DuplicateSubBarPath { symbol } => {
                write!(f, "duplicate sub-bar path for symbol `{symbol}`")
            }
            Self::UnknownCorporateActionSymbol { symbol } => {
                write!(f, "corporate action names unknown symbol `{symbol}`")
            }
            Self::DelistingWithoutMark { symbol } => write!(
                f,
                "symbol `{symbol}` is delisted before any bar closed, so it has no cash-out mark"
            ),
            Self::SymbolChangeMismatch { symbol } => {
                write!(f, "symbol change for `{symbol}` does not match its stream")
            }
            Self::FractionalUnitsRefused { symbol, units } => write!(
                f,
                "a split left `{symbol}` holding {units} units and the run refuses \
                 fractional remainders"
            ),
            Self::CashInLieuWithoutMark { symbol } => write!(
                f,
                "cash in lieu for `{symbol}` has no committed mark to price the fraction at"
            ),
            Self::FinancingRateUnavailable { symbol, charge } => write!(
                f,
                "symbol `{symbol}` holds exposure needing a `{charge}` rate the policy calls unavailable"
            ),
            Self::TooManyAccrualBoundaries { limit } => {
                write!(
                    f,
                    "simulation would schedule more than {limit} accrual boundaries"
                )
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

/// What a strategy is allowed to know about the exposure it already holds.
///
/// Every field is a consequence of fills that have already happened, so this
/// reveals no future information — but without it, protective management is
/// impossible to express: a break-even move has no entry price to move to, and
/// a trail has no high-water mark to ratchet against.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PositionView {
    /// Signed units held. Positive is long, negative short, zero flat.
    pub units: f64,
    /// Volume-weighted entry of the exposure currently held; zero when flat.
    pub average_entry: f64,
    /// Realized PnL banked on this symbol so far, across all closed exposure.
    pub realized_pnl: f64,
    /// Fill time that opened the exposure currently held; zero when flat.
    pub opened_time_ns: i64,
    /// Best price reached in the position's favour since it opened, taken from
    /// committed bars only. Equals `average_entry` before any bar has closed.
    pub favorable_extreme: f64,
}

impl PositionView {
    pub const fn is_flat(&self) -> bool {
        self.units == 0.0
    }
    pub const fn is_long(&self) -> bool {
        self.units > 0.0
    }
    pub const fn is_short(&self) -> bool {
        self.units < 0.0
    }
}

pub struct DecisionContext<'a> {
    symbol: SymbolId,
    time_ns: i64,
    forming: Option<FormingBar>,
    market: MarketView<'a>,
    positions: &'a [PositionState],
    orders: &'a [Order],
    session: Option<SessionStatus>,
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

    /// What the deciding instrument's venue is doing right now (§6.7), or
    /// `None` when the instrument declares no calendar.
    ///
    /// This is how a session-relative rule — "no entries in the first fifteen
    /// minutes" — is expressible without the strategy owning a time zone: the
    /// minutes are counted from the venue's own open, projected through its
    /// exchange clock with correct daylight-saving behaviour.
    pub const fn session(&self) -> Option<SessionStatus> {
        self.session
    }

    /// Exposure held on `symbol` as of this decision. Unknown symbols read as
    /// flat rather than erroring: a strategy asking about a symbol it cannot
    /// trade holds nothing in it either way.
    pub fn position(&self, symbol: SymbolId) -> PositionView {
        self.positions.get(symbol.0).map_or(
            PositionView {
                units: 0.0,
                average_entry: 0.0,
                realized_pnl: 0.0,
                opened_time_ns: 0,
                favorable_extreme: 0.0,
            },
            |state| PositionView {
                units: state.units,
                average_entry: state.avg_entry,
                realized_pnl: state.realized_pnl,
                opened_time_ns: state.opened_time_ns,
                favorable_extreme: state.favorable_extreme,
            },
        )
    }

    /// Shorthand for the position on the symbol this decision is about.
    pub fn own_position(&self) -> PositionView {
        self.position(self.symbol)
    }

    /// Whether an order created by this strategy remains submitted or active
    /// as of this decision. Lifecycle managers use this to avoid modifying or
    /// cancelling an order already retired by a fill, OCO sibling, or expiry.
    pub fn is_order_live(&self, client_id: ClientOrderId) -> bool {
        self.orders
            .iter()
            .any(|order| order.client_id == client_id && order.state != OrderState::Done)
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
    /// Size already taken by partial fills (§6.6).
    filled_quantity: f64,
}

impl Order {
    /// Size still working. Clamped at zero so accumulated rounding can never
    /// make a fully-filled order look like it has a sliver left.
    fn remaining(&self) -> f64 {
        stable_decimal(self.quantity - self.filled_quantity).max(0.0)
    }
}

#[derive(Debug, Clone, Default)]
struct PositionState {
    units: f64,
    avg_entry: f64,
    realized_pnl: f64,
    /// Fill time that opened the exposure currently held, so a strategy can
    /// anchor a time stop or a break-even move without being handed the raw
    /// fill stream. Reset when the position goes flat, re-stamped on a flip.
    opened_time_ns: i64,
    /// Extreme mark seen while this exposure has been held, in the direction
    /// that favours it. A trailing stop needs a high-water mark, and deriving
    /// one from `MarketView` would force every strategy to re-scan history.
    favorable_extreme: f64,
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
    BarOpen {
        symbol: usize,
        bar: usize,
    },
    ExecuteOpen {
        symbol: usize,
        bar: usize,
    },
    /// One step of the §6.9 level-3 intrabar path.
    ExecuteSubBar {
        symbol: usize,
        bar: usize,
        sub: usize,
    },
    BarClose {
        symbol: usize,
        bar: usize,
    },
    ExecuteClose {
        symbol: usize,
        bar: usize,
    },
    Decision {
        symbol: usize,
        bar: usize,
    },
    Mark {
        symbol: usize,
        bar: usize,
    },
    Submit {
        order: usize,
    },
    Activate {
        order: usize,
    },
    Expire {
        order: usize,
    },
    ApplyRequest {
        request: usize,
    },
    /// A §6.8 event at its effective instant.
    Corporate {
        action: usize,
    },
    /// A §6.3 accrual boundary for one symbol.
    Accrue {
        symbol: usize,
    },
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

/// Apply one execution to a position and return the realized profit in
/// **account** currency.
///
/// `conversion_rate` is account-currency units per unit of the instrument's
/// quote currency. Prices — `avg_entry`, the high-water mark — stay in the
/// instrument's currency, because they are prices and converting them would
/// make them comparable to nothing. Only the realized amount crosses over.
fn apply_fill(
    position: &mut PositionState,
    side: OrderSide,
    quantity: f64,
    price: f64,
    time_ns: i64,
    conversion_rate: f64,
) -> f64 {
    let delta = side.sign() * quantity;
    let old = position.units;
    if old == 0.0 || old.signum() == delta.signum() {
        let new_units = old + delta;
        position.avg_entry = stable_decimal(if old == 0.0 {
            price
        } else {
            (position.avg_entry * old.abs() + price * quantity) / new_units.abs()
        });
        if old == 0.0 {
            position.opened_time_ns = time_ns;
            position.favorable_extreme = price;
        }
        position.units = new_units;
        return 0.0;
    }
    let closed = old.abs().min(quantity);
    let realized_instrument = if old > 0.0 {
        (price - position.avg_entry) * closed
    } else {
        (position.avg_entry - price) * closed
    };
    let realized = realized_instrument * conversion_rate;
    let new_units = old + delta;
    if new_units == 0.0 {
        position.avg_entry = 0.0;
        position.opened_time_ns = 0;
        position.favorable_extreme = 0.0;
    } else if new_units.signum() != old.signum() {
        // A flip is a new position: its protective anchors must not inherit the
        // reversed one's entry time or high-water mark.
        position.avg_entry = price;
        position.opened_time_ns = time_ns;
        position.favorable_extreme = price;
    }
    position.units = new_units;
    position.realized_pnl = stable_decimal(position.realized_pnl + realized);
    stable_decimal(realized)
}

fn marked_equity(
    cash: f64,
    positions: &[PositionState],
    marks: &[Option<f64>],
    instruments: &[InstrumentRuntime],
) -> f64 {
    positions
        .iter()
        .zip(marks)
        .enumerate()
        .fold(cash, |equity, (index, (position, mark))| {
            let rate = instruments.get(index).map_or(1.0, |i| i.conversion_rate);
            equity + mark.map_or(0.0, |price| position.units * price * rate)
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

/// Everything the execution layer knows about one instrument (§6.3, §6.7).
#[derive(Debug, Clone, Default)]
struct InstrumentRuntime {
    calendar: Option<TradingCalendar>,
    financing: Option<FinancingPolicy>,
    /// Account-currency units per unit of the instrument's quote currency.
    conversion_rate: f64,
    /// Conversion cost charged on the absolute converted amount, per fill.
    conversion_spread_percent: f64,
    /// Per-instrument tick lattice, overriding the run-wide one.
    price_tick: Option<f64>,
}

struct Sim<'a> {
    config: &'a StrategyExecutionConfig,
    setup: &'a SimulationSetup,
    streams: Vec<SymbolStream>,
    /// Finer-timeframe path per symbol; empty at levels 1–2.
    sub_bars: Vec<Vec<SimBar>>,
    /// `sub_index[symbol][bar]` is the half-open sub-bar range of that bar.
    sub_index: Vec<Vec<(usize, usize)>>,
    symbol_count: usize,
    instruments: Vec<InstrumentRuntime>,
    /// Actions still to apply, in canonical order; `Task::Corporate` indexes it.
    corporate: Vec<CorporateAction>,
    /// Symbols whose delisting has taken effect. Nothing trades on them again.
    delisted: Vec<bool>,
    /// Last accrual boundary each symbol was charged at.
    last_accrual_ns: Vec<i64>,
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
    /// Participation consumed inside the parent bar currently open on a symbol,
    /// and which bar that is (§6.6).
    window_bar: Vec<Option<usize>>,
    window_taken: Vec<f64>,
    fills: Vec<FillRecord>,
    rejections: Vec<RejectionRecord>,
    cancellations: Vec<CancelRecord>,
    equity_curve: Vec<EquityPoint>,
    financing_charges: Vec<FinancingChargeRecord>,
    corporate_records: Vec<CorporateActionRecord>,
    cash: f64,
    total_commission: f64,
    total_conversion_cost: f64,
    total_financing_cost: f64,
    realized_pnl_account: f64,
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

    /// What the instrument's calendar says at `time_ns`. `None` when the
    /// instrument declares no calendar, which means it is never gated.
    fn session_status(&self, symbol: usize, time_ns: i64) -> Option<SessionStatus> {
        self.instruments
            .get(symbol)
            .and_then(|runtime| runtime.calendar.as_ref())
            .map(|calendar| calendar.status_at_ns(time_ns))
    }

    /// Whether the venue would accept an execution for `symbol` at `time_ns`.
    /// An instrument with no calendar always would; a delisted one never does.
    fn venue_open(&self, symbol: usize, time_ns: i64) -> bool {
        if self.delisted[symbol] {
            return false;
        }
        self.session_status(symbol, time_ns)
            .is_none_or(SessionStatus::is_open)
    }

    fn conversion_rate(&self, symbol: usize) -> f64 {
        self.instruments
            .get(symbol)
            .map_or(1.0, |runtime| runtime.conversion_rate)
    }

    /// Size the participation cap still allows inside the parent bar currently
    /// open on `symbol`, or `None` when the run is uncapped.
    fn window_available(&mut self, symbol: usize, bar: usize, bar_volume: f64) -> Option<f64> {
        let capacity = self
            .config
            .settings()
            .participation
            .bar_capacity(bar_volume)?;
        if self.window_bar[symbol] != Some(bar) {
            self.window_bar[symbol] = Some(bar);
            self.window_taken[symbol] = 0.0;
        }
        Some((capacity - self.window_taken[symbol]).max(0.0))
    }

    fn consume_window(&mut self, symbol: usize, quantity: f64) {
        self.window_taken[symbol] = stable_decimal(self.window_taken[symbol] + quantity);
    }

    /// The tick lattice an order on `symbol` must sit on: the instrument's own
    /// when it declares one, otherwise the run-wide setting.
    fn price_tick(&self, symbol: usize) -> Option<f64> {
        self.instruments
            .get(symbol)
            .and_then(|runtime| runtime.price_tick)
            .or(self.config.settings().price_tick)
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
    UnsupportedDatasetTimeframe {
        input_id: String,
        timeframe: String,
    },
    InvalidDatasetTimestamp {
        input_id: String,
        timestamp: String,
    },
    DatasetTimeOverflow {
        input_id: String,
    },
    InvalidSubBarDatasetTimestamp {
        parent_input_id: String,
        timestamp: String,
    },
    SubBarDatasetTimeOverflow {
        parent_input_id: String,
    },
    TooManyVerifiedSubBars {
        limit: usize,
        found: usize,
    },
    InvalidInterventionLog(InterventionError),
    MissingInterventionLog,
    UnexpectedInterventionLog,
    InterventionLogIdMismatch {
        expected: String,
        actual: String,
    },
    InterventionReplayIncomplete {
        expected: usize,
        applied: usize,
    },
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
            Self::InvalidSubBarDatasetTimestamp {
                parent_input_id,
                timestamp,
            } => write!(
                formatter,
                "sub-bar dataset for parent `{parent_input_id}` has invalid bar timestamp `{timestamp}`"
            ),
            Self::SubBarDatasetTimeOverflow { parent_input_id } => write!(
                formatter,
                "sub-bar dataset for parent `{parent_input_id}` exceeds simulator time range"
            ),
            Self::TooManyVerifiedSubBars { limit, found } => write!(
                formatter,
                "verified run materializes {found} sub-bars, limit {limit}"
            ),
            Self::InvalidInterventionLog(error) => write!(formatter, "intervention log: {error}"),
            Self::MissingInterventionLog => formatter.write_str(
                "run manifest binds an intervention log, but none was supplied for replay",
            ),
            Self::UnexpectedInterventionLog => formatter.write_str(
                "an intervention log was supplied for a run manifest that does not bind one",
            ),
            Self::InterventionLogIdMismatch { expected, actual } => write!(
                formatter,
                "run manifest intervention log id mismatch: expected {expected}, got {actual}"
            ),
            Self::InterventionReplayIncomplete { expected, applied } => write!(
                formatter,
                "hybrid replay applied {applied} of {expected} bound interventions"
            ),
        }
    }
}

impl Error for VerifiedSimulationError {}

/// Execute an automated identity-bearing run without accepting any simulation
/// knob that could disagree with its sealed strategy and manifest.
///
/// Hybrid runs must use [`run_verified_simulation_with_intervention`]; this
/// preserves the automated API while refusing to silently ignore a bound log.
pub fn run_verified_simulation(
    run: &VerifiedRun<'_>,
) -> Result<SimulationReport, VerifiedSimulationError> {
    if run.intervention_log().is_some() {
        return Err(VerifiedSimulationError::UnexpectedInterventionLog);
    }
    run_verified_automated_simulation(run)
}

/// Execute a hybrid run only when the supplied sealed log is exactly the log
/// included in the verified run manifest identity.
pub fn run_verified_simulation_with_intervention(
    run: &VerifiedRun<'_>,
    intervention_log: Option<&InterventionLog>,
) -> Result<SimulationReport, VerifiedSimulationError> {
    let expected = run.manifest().binding().intervention_log_id.as_ref();
    let log = match (expected, intervention_log) {
        (Some(_), None) | (None, None) => {
            return Err(VerifiedSimulationError::MissingInterventionLog);
        }
        (None, Some(_)) => return Err(VerifiedSimulationError::UnexpectedInterventionLog),
        (Some(expected), Some(log)) => {
            log.verify()
                .map_err(VerifiedSimulationError::InvalidInterventionLog)?;
            if expected != log.log_id() {
                return Err(VerifiedSimulationError::InterventionLogIdMismatch {
                    expected: expected.clone(),
                    actual: log.log_id().to_string(),
                });
            }
            log
        }
    };
    if run.intervention_log().map(InterventionLog::log_id) != Some(log.log_id()) {
        return Err(VerifiedSimulationError::InterventionLogIdMismatch {
            expected: expected.cloned().unwrap_or_default(),
            actual: run
                .intervention_log()
                .map(InterventionLog::log_id)
                .unwrap_or("missing-from-verified-run")
                .to_string(),
        });
    }

    let setup = SimulationSetup::from_verified_run(run);
    let mut strategy =
        CanonicalIrStrategy::new(run.strategy()).map_err(VerifiedSimulationError::Interpreter)?;
    let (streams, sub_bar_paths) = verified_streams_and_paths(run)?;
    let mut replay = HybridReplay::new(&mut strategy, log);
    let report =
        run_simulation_with_paths(run.config(), &setup, &streams, &sub_bar_paths, &mut replay)
            .map_err(VerifiedSimulationError::Simulation)?;
    if replay.applied() != log.interventions().len() {
        return Err(VerifiedSimulationError::InterventionReplayIncomplete {
            expected: log.interventions().len(),
            applied: replay.applied(),
        });
    }
    Ok(report)
}

fn run_verified_automated_simulation(
    run: &VerifiedRun<'_>,
) -> Result<SimulationReport, VerifiedSimulationError> {
    let setup = SimulationSetup::from_verified_run(run);
    let mut strategy =
        CanonicalIrStrategy::new(run.strategy()).map_err(VerifiedSimulationError::Interpreter)?;
    let (streams, sub_bar_paths) = verified_streams_and_paths(run)?;
    run_simulation_with_paths(
        run.config(),
        &setup,
        &streams,
        &sub_bar_paths,
        &mut strategy,
    )
    .map_err(VerifiedSimulationError::Simulation)
}

fn verified_streams_and_paths(
    run: &VerifiedRun<'_>,
) -> Result<(Vec<SymbolStream>, Vec<SubBarPath>), VerifiedSimulationError> {
    let streams = verified_symbol_streams(run)?;
    let total = run
        .sub_bar_datasets()
        .iter()
        .try_fold(0usize, |total, dataset| {
            total.checked_add(dataset.bars.len())
        })
        .unwrap_or(usize::MAX);
    if total > MAX_TOTAL_SUB_BARS {
        return Err(VerifiedSimulationError::TooManyVerifiedSubBars {
            limit: MAX_TOTAL_SUB_BARS,
            found: total,
        });
    }
    let mut paths = Vec::with_capacity(run.sub_bar_datasets().len());
    for parent in run.datasets() {
        let Some(dataset) = run
            .sub_bar_datasets()
            .iter()
            .find(|dataset| dataset.parent_input_id() == parent.input_id())
        else {
            continue;
        };
        let step_seconds = fixed_timeframe_seconds(&dataset.manifest.timeframe)
            .expect("verified run assembly accepts only fixed sub-bar timeframes");
        let step_ns = step_seconds.checked_mul(1_000_000_000).ok_or_else(|| {
            VerifiedSimulationError::SubBarDatasetTimeOverflow {
                parent_input_id: dataset.parent_input_id().to_string(),
            }
        })?;
        let bars = dataset
            .bars
            .iter()
            .map(|bar| {
                let open_time_ns = chrono::DateTime::parse_from_rfc3339(&bar.timestamp)
                    .ok()
                    .and_then(|stamp| stamp.timestamp_nanos_opt())
                    .ok_or_else(|| VerifiedSimulationError::InvalidSubBarDatasetTimestamp {
                        parent_input_id: dataset.parent_input_id().to_string(),
                        timestamp: bar.timestamp.clone(),
                    })?;
                let close_time_ns = open_time_ns
                    .checked_add(step_ns)
                    .and_then(|time| time.checked_sub(1))
                    .ok_or_else(|| VerifiedSimulationError::SubBarDatasetTimeOverflow {
                        parent_input_id: dataset.parent_input_id().to_string(),
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
        paths.push(SubBarPath {
            symbol: parent.manifest.symbol.clone(),
            bars,
        });
    }
    Ok((streams, paths))
}

pub(crate) fn verified_symbol_streams(
    run: &VerifiedRun<'_>,
) -> Result<Vec<SymbolStream>, VerifiedSimulationError> {
    if run.datasets().len() > MAX_SYMBOLS {
        return Err(VerifiedSimulationError::Simulation(
            SimulationError::TooManySymbols {
                limit: MAX_SYMBOLS,
                found: run.datasets().len(),
            },
        ));
    }
    let total_bars = run
        .datasets()
        .iter()
        .try_fold(0usize, |total, dataset| {
            total.checked_add(dataset.bars.len())
        })
        .unwrap_or(usize::MAX);
    if total_bars > MAX_TOTAL_BARS {
        return Err(VerifiedSimulationError::Simulation(
            SimulationError::TooManyTotalBars {
                limit: MAX_TOTAL_BARS,
                found: total_bars,
            },
        ));
    }
    for dataset in run.datasets() {
        if dataset.bars.len() > MAX_BARS_PER_SYMBOL {
            return Err(VerifiedSimulationError::Simulation(
                SimulationError::TooManyBars {
                    symbol: dataset.manifest.symbol.clone(),
                    limit: MAX_BARS_PER_SYMBOL,
                    found: dataset.bars.len(),
                },
            ));
        }
    }
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
    run_simulation_with_paths(config, setup, streams, &[], strategy)
}

/// Run one deterministic simulation with an explicit §6.9 level-3 intrabar
/// path.
///
/// The path is separate from the execution streams because it is a property of
/// the run's declared fidelity, not of the instrument. A path supplied at a
/// fidelity that does not consume one is an error rather than dead input, and a
/// missing path at sub-bar fidelity is an error rather than a silent fall back
/// to the coarser model.
pub fn run_simulation_with_paths(
    config: &StrategyExecutionConfig,
    setup: &SimulationSetup,
    streams: &[SymbolStream],
    sub_bar_paths: &[SubBarPath],
    strategy: &mut dyn ReferenceStrategy,
) -> Result<SimulationReport, SimulationError> {
    config.verify().map_err(SimulationError::Config)?;
    validate_models(config)?;
    validate_setup(setup)?;
    let streams = validate_inputs(streams)?;
    let symbol_count = streams.len();
    let settings = config.settings();
    let (sub_bars, sub_index) = validate_sub_bars(&streams, sub_bar_paths, settings.fidelity)?;
    let instruments = build_instruments(&streams, settings);
    let corporate = resolve_corporate_actions(&streams, settings)?;

    let mut sim = Sim {
        config,
        setup,
        streams,
        sub_bars,
        sub_index,
        symbol_count,
        instruments,
        corporate,
        delisted: vec![false; symbol_count],
        last_accrual_ns: vec![i64::MIN; symbol_count],
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
        window_bar: vec![None; symbol_count],
        window_taken: vec![0.0; symbol_count],
        fills: Vec::new(),
        rejections: Vec::new(),
        cancellations: Vec::new(),
        equity_curve: Vec::new(),
        financing_charges: Vec::new(),
        corporate_records: Vec::new(),
        cash: settings.initial_capital,
        total_commission: 0.0,
        total_conversion_cost: 0.0,
        total_financing_cost: 0.0,
        realized_pnl_account: 0.0,
        next_client_id: 0,
    };

    build_bar_schedule(&mut sim);
    build_corporate_schedule(&mut sim);
    build_accrual_schedule(&mut sim)?;
    sim.queue.sort_by_key(schedule_key);
    sim.schedule_tie = sim.queue.len() as u64;

    while sim.cursor < sim.queue.len() {
        let entry = sim.queue[sim.cursor];
        sim.cursor += 1;
        run_task(&mut sim, entry, strategy)?;
    }

    finish(sim)
}

/// Project the run's instrument registry onto the sorted symbol table.
///
/// A symbol with no spec keeps the M1 model exactly: no calendar, no accrual,
/// parity conversion. That is the honest reading of "we did not say", and the
/// report carries the registry so the omission is visible.
fn build_instruments(
    streams: &[SymbolStream],
    settings: &crate::core::strategy_ir::ExecutionSettings,
) -> Vec<InstrumentRuntime> {
    streams
        .iter()
        .map(|stream| {
            let Some(spec) = settings.instruments.get(&stream.symbol) else {
                return InstrumentRuntime {
                    conversion_rate: 1.0,
                    ..InstrumentRuntime::default()
                };
            };
            // Validation already proved every declared currency resolves, so a
            // missing lookup here is impossible; parity is the only safe value
            // if it ever were, and it matches the no-spec case.
            let (rate, spread) = settings
                .currency_conversion
                .lookup(&spec.currency, &settings.account_currency)
                .unwrap_or((1.0, 0.0));
            InstrumentRuntime {
                calendar: spec.calendar.clone(),
                financing: spec.financing.policy().cloned(),
                conversion_rate: rate,
                conversion_spread_percent: spread,
                price_tick: spec.price_tick,
            }
        })
        .collect()
}

/// Resolve the schedule against the run's symbols. An action naming a symbol
/// the run does not trade is refused rather than ignored: silently dropping it
/// would let a config claim a split that never happened.
fn resolve_corporate_actions(
    streams: &[SymbolStream],
    settings: &crate::core::strategy_ir::ExecutionSettings,
) -> Result<Vec<CorporateAction>, SimulationError> {
    let actions = settings.corporate_actions.actions();
    for action in actions {
        if !streams.iter().any(|stream| stream.symbol == action.symbol) {
            return Err(SimulationError::UnknownCorporateActionSymbol {
                symbol: action.symbol.clone(),
            });
        }
        if let CorporateActionKind::SymbolChange { new_symbol } = &action.kind
            && streams.iter().any(|stream| &stream.symbol == new_symbol)
        {
            // Both identities in one run would mean two streams for one
            // instrument, which the symbol table cannot express.
            return Err(SimulationError::SymbolChangeMismatch {
                symbol: new_symbol.clone(),
            });
        }
    }
    Ok(actions.to_vec())
}

/// Bind each symbol's finer-timeframe path and prove it is a path *inside* the
/// execution stream rather than a second, disagreeing series.
type SubBarBinding = (Vec<Vec<SimBar>>, Vec<Vec<(usize, usize)>>);

fn validate_sub_bars(
    streams: &[SymbolStream],
    paths: &[SubBarPath],
    fidelity: FidelityLevel,
) -> Result<SubBarBinding, SimulationError> {
    let expected_step_ns = match fidelity {
        FidelityLevel::SubBar { sub_bar_seconds } => {
            Some(i64::from(sub_bar_seconds) * NANOS_PER_SECOND)
        }
        _ => None,
    };
    if expected_step_ns.is_none() {
        if let Some(path) = paths.first() {
            return Err(SimulationError::UnexpectedSubBarPath {
                symbol: path.symbol.clone(),
            });
        }
        return Ok((
            vec![Vec::new(); streams.len()],
            vec![Vec::new(); streams.len()],
        ));
    }

    let total = paths
        .iter()
        .try_fold(0usize, |total, path| total.checked_add(path.bars.len()))
        .unwrap_or(usize::MAX);
    if total > MAX_TOTAL_SUB_BARS {
        return Err(SimulationError::TooManySubBars {
            limit: MAX_TOTAL_SUB_BARS,
            found: total,
        });
    }
    let mut seen_paths = BTreeSet::new();
    for path in paths {
        if !seen_paths.insert(path.symbol.as_str()) {
            return Err(SimulationError::DuplicateSubBarPath {
                symbol: path.symbol.clone(),
            });
        }
        if !streams.iter().any(|stream| stream.symbol == path.symbol) {
            return Err(SimulationError::UnknownSubBarSymbol {
                symbol: path.symbol.clone(),
            });
        }
    }

    let mut bars_by_symbol = Vec::with_capacity(streams.len());
    let mut index_by_symbol = Vec::with_capacity(streams.len());
    for stream in streams {
        let Some(path) = paths.iter().find(|path| path.symbol == stream.symbol) else {
            return Err(SimulationError::MissingSubBarPath {
                symbol: stream.symbol.clone(),
            });
        };
        validate_sub_bar_path(stream, path)?;
        let sub = &path.bars;
        let expected_step_ns = expected_step_ns.expect("sub-bar fidelity supplies a duration");

        let mut ranges = Vec::with_capacity(stream.bars.len());
        let mut cursor = 0usize;
        for (parent_index, parent) in stream.bars.iter().enumerate() {
            let start = cursor;
            let mut expected_open_time_ns = parent.open_time_ns;
            let parent_end = parent.close_time_ns.checked_add(1).ok_or_else(|| {
                SimulationError::SubBarNotContained {
                    symbol: stream.symbol.clone(),
                    index: cursor,
                }
            })?;
            while cursor < sub.len() {
                let candidate = sub[cursor];
                if candidate.open_time_ns >= parent_end {
                    break;
                }
                let candidate_end = candidate.close_time_ns.checked_add(1).ok_or_else(|| {
                    SimulationError::SubBarNotContained {
                        symbol: stream.symbol.clone(),
                        index: cursor,
                    }
                })?;
                let inside_time =
                    candidate.open_time_ns >= parent.open_time_ns && candidate_end <= parent_end;
                let inside_price = candidate.high <= parent.high && candidate.low >= parent.low;
                if !inside_time || !inside_price {
                    return Err(SimulationError::SubBarNotContained {
                        symbol: stream.symbol.clone(),
                        index: cursor,
                    });
                }
                if candidate.open_time_ns > expected_open_time_ns {
                    return Err(SimulationError::SubBarGap {
                        symbol: stream.symbol.clone(),
                        parent_index,
                        expected_open_time_ns,
                        actual_open_time_ns: Some(candidate.open_time_ns),
                    });
                }
                if candidate.open_time_ns < expected_open_time_ns {
                    return Err(SimulationError::SubBarOverlap {
                        symbol: stream.symbol.clone(),
                        parent_index,
                        previous_index: cursor.saturating_sub(1),
                        index: cursor,
                    });
                }
                let actual_ns = candidate_end
                    .checked_sub(candidate.open_time_ns)
                    .ok_or_else(|| SimulationError::SubBarNotContained {
                        symbol: stream.symbol.clone(),
                        index: cursor,
                    })?;
                if actual_ns != expected_step_ns {
                    return Err(SimulationError::SubBarDurationMismatch {
                        symbol: stream.symbol.clone(),
                        index: cursor,
                        expected_ns: expected_step_ns,
                        actual_ns,
                    });
                }
                expected_open_time_ns = candidate_end;
                cursor += 1;
            }
            if expected_open_time_ns != parent_end {
                return Err(SimulationError::SubBarGap {
                    symbol: stream.symbol.clone(),
                    parent_index,
                    expected_open_time_ns,
                    actual_open_time_ns: sub.get(cursor).map(|bar| bar.open_time_ns),
                });
            }
            ranges.push((start, cursor));
        }
        if cursor != sub.len() {
            return Err(SimulationError::SubBarNotContained {
                symbol: stream.symbol.clone(),
                index: cursor,
            });
        }
        // The path has passed the global hard cap before this sole ownership
        // clone. Validation above operates on the borrowed bars and therefore
        // cannot amplify an untrusted path before its size is bounded.
        bars_by_symbol.push(sub.to_vec());
        index_by_symbol.push(ranges);
    }
    Ok((bars_by_symbol, index_by_symbol))
}

fn validate_sub_bar_path(stream: &SymbolStream, path: &SubBarPath) -> Result<(), SimulationError> {
    for (index, bar) in path.bars.iter().enumerate() {
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
        if index > 0 && bar.open_time_ns <= path.bars[index - 1].close_time_ns {
            let open = bar.open_time_ns;
            let parent_index = stream
                .bars
                .iter()
                .position(|parent| open >= parent.open_time_ns && open <= parent.close_time_ns)
                .unwrap_or(0);
            return Err(SimulationError::SubBarOverlap {
                symbol: stream.symbol.clone(),
                parent_index,
                previous_index: index - 1,
                index,
            });
        }
    }
    Ok(())
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
            // Level-3 path steps share the open-execution slot and are pushed
            // after the parent's own open execution, so a stable sort keeps
            // them in time order behind it.
            if let Some(&(start, end)) = sim.sub_index[symbol].get(bar) {
                for sub in start..end {
                    sim.push(
                        sim.sub_bars[symbol][sub].close_time_ns,
                        priority::OPEN_EXECUTION,
                        tie,
                        Task::ExecuteSubBar { symbol, bar, sub },
                    );
                }
            }
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

/// Lay down every corporate action at its effective instant (§6.8). The
/// canonical schedule order is already total, so the tie counter simply follows
/// it and two actions at one instant keep their declared precedence.
fn build_corporate_schedule(sim: &mut Sim<'_>) {
    for index in 0..sim.corporate.len() {
        let time_ns = sim.corporate[index].effective_time_ns;
        sim.push(
            time_ns,
            priority::CORPORATE,
            index as u64,
            Task::Corporate { action: index },
        );
    }
}

/// Lay down every accrual boundary inside the run's span (§6.3).
///
/// Boundaries are absolute — a UTC day is a UTC day regardless of when the run
/// starts — so two runs over overlapping ranges charge on the same instants.
fn build_accrual_schedule(sim: &mut Sim<'_>) -> Result<(), SimulationError> {
    let mut scheduled = 0usize;
    for symbol in 0..sim.symbol_count {
        let Some(policy) = sim.instruments[symbol].financing.clone() else {
            continue;
        };
        let (Some(first), Some(last)) = (
            sim.streams[symbol].bars.first().copied(),
            sim.streams[symbol].bars.last().copied(),
        ) else {
            continue;
        };
        // Nothing can be held before the first bar opens, so the first
        // boundary that can charge anything is the one after it.
        let mut boundary = policy.next_boundary_ns(first.open_time_ns);
        while let Some(time_ns) = boundary {
            if time_ns > last.close_time_ns {
                break;
            }
            scheduled += 1;
            if scheduled > MAX_ACCRUAL_BOUNDARIES {
                return Err(SimulationError::TooManyAccrualBoundaries {
                    limit: MAX_ACCRUAL_BOUNDARIES,
                });
            }
            sim.push(
                time_ns,
                priority::ACCRUAL,
                symbol as u64,
                Task::Accrue { symbol },
            );
            boundary = policy.next_boundary_ns(time_ns);
        }
    }
    Ok(())
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
            // Advance the trailing high-water mark only on committed bars, so a
            // trail can never ratchet off a bar the strategy is not allowed to
            // have seen yet.
            let row = sim.bar(symbol, bar);
            let (high, low) = (row.high, row.low);
            let position = &mut sim.positions[symbol];
            if position.units > 0.0 {
                position.favorable_extreme = position.favorable_extreme.max(high);
            } else if position.units < 0.0 {
                position.favorable_extreme = position.favorable_extreme.min(low);
            }
            sim.recorder.event(
                entry.time_ns,
                SimEventKind::BarClose,
                priority::BAR_CLOSE,
                Some(SymbolId(symbol)),
                None,
            )?;
        }
        Task::ExecuteOpen { symbol, bar } => execute_phase(sim, symbol, bar, Phase::Open)?,
        Task::ExecuteSubBar { symbol, bar, sub } => {
            execute_phase(sim, symbol, bar, Phase::SubBar { sub })?;
        }
        Task::ExecuteClose { symbol, bar } => execute_phase(sim, symbol, bar, Phase::Close)?,
        Task::Corporate { action } => apply_corporate_action(sim, action, entry.time_ns)?,
        Task::Accrue { symbol } => accrue_financing(sim, symbol, entry.time_ns)?,
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
                marked_equity(sim.cash, &sim.positions, &sim.marks, &sim.instruments),
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
    /// One step of the level-3 intrabar path, resolved against `sub`'s OHLC.
    SubBar {
        sub: usize,
    },
    Close,
    LegacyClose,
}

impl Phase {
    const fn is_close(self) -> bool {
        matches!(self, Self::Close | Self::LegacyClose)
    }
}

// ── Corporate actions (§6.8) ───────────────────────────────────────

/// Apply one action at its effective instant.
///
/// Everything here operates on the position and cash that exist *now*. Nothing
/// reaches back into price history, which is precisely the hazard §6.8 exists
/// to rule out: a stop written in pre-split prices is retired rather than
/// silently re-interpreted against post-split ones.
fn apply_corporate_action(
    sim: &mut Sim<'_>,
    action_index: usize,
    time_ns: i64,
) -> Result<(), SimulationError> {
    let action = sim.corporate[action_index].clone();
    let Some(symbol) = sim
        .streams
        .iter()
        .position(|stream| stream.symbol == action.symbol)
    else {
        return Err(SimulationError::UnknownCorporateActionSymbol {
            symbol: action.symbol,
        });
    };

    let units_before = sim.positions[symbol].units;
    let avg_entry_before = sim.positions[symbol].avg_entry;
    let mut cash_delta = 0.0;
    let mut cancelled = 0usize;

    match &action.kind {
        CorporateActionKind::Split {
            numerator,
            denominator,
        } => {
            let (units_factor, price_factor) =
                CorporateAction::split_factors(*numerator, *denominator);
            let position = &mut sim.positions[symbol];
            position.units = stable_decimal(position.units * units_factor);
            position.avg_entry = stable_decimal(position.avg_entry * price_factor);
            position.favorable_extreme = stable_decimal(position.favorable_extreme * price_factor);
            // The mark moves with the prices, so equity does not jump on the
            // split itself; the next bar's close replaces it either way.
            if let Some(mark) = sim.marks[symbol] {
                sim.marks[symbol] = Some(stable_decimal(mark * price_factor));
            }
            cancelled = cancel_symbol_orders(sim, symbol, time_ns)?;
            cash_delta = settle_fractional_units(sim, symbol, &action.symbol, time_ns)?;
        }
        CorporateActionKind::CashDividend { amount_per_unit } => {
            // Longs receive, shorts pay. The sign falls straight out of the
            // signed position, so no branch can get it backwards.
            let rate = sim.conversion_rate(symbol);
            cash_delta = stable_decimal(units_before * amount_per_unit * rate);
            sim.cash = finite_accounting("cash", stable_decimal(sim.cash + cash_delta))?;
        }
        CorporateActionKind::SymbolChange { .. } => {
            // No economic effect. Recorded so a report shows the identity
            // change against the exposure that lived through it.
        }
        CorporateActionKind::Delisting => {
            cancelled = cancel_symbol_orders(sim, symbol, time_ns)?;
            if units_before != 0.0 {
                let Some(mark) = sim.marks[symbol] else {
                    return Err(SimulationError::DelistingWithoutMark {
                        symbol: action.symbol,
                    });
                };
                // A cash-out is not a trade: no venue charges a spread,
                // slippage or commission for delisting your position, so
                // modelling one would invent a cost.
                let rate = sim.conversion_rate(symbol);
                let realized = apply_fill(
                    &mut sim.positions[symbol],
                    if units_before > 0.0 {
                        OrderSide::Sell
                    } else {
                        OrderSide::Buy
                    },
                    units_before.abs(),
                    mark,
                    time_ns,
                    rate,
                );
                cash_delta = stable_decimal(units_before * mark * rate);
                sim.cash = finite_accounting("cash", stable_decimal(sim.cash + cash_delta))?;
                sim.realized_pnl_account = stable_decimal(sim.realized_pnl_account + realized);
            }
            sim.delisted[symbol] = true;
            // A delisted instrument has no further marks; keeping the last one
            // would report an equity position in a security that no longer
            // exists.
            sim.marks[symbol] = None;
        }
    }

    let sequence = sim.recorder.event(
        time_ns,
        SimEventKind::CorporateAction,
        priority::CORPORATE,
        Some(SymbolId(symbol)),
        None,
    )?;
    sim.corporate_records.push(CorporateActionRecord {
        time_ns,
        sequence,
        symbol: SymbolId(symbol),
        kind: action.kind.wire_id().to_string(),
        units_before,
        units_after: sim.positions[symbol].units,
        avg_entry_before,
        avg_entry_after: sim.positions[symbol].avg_entry,
        cash_delta,
        orders_cancelled: cancelled,
    });
    Ok(())
}

/// Apply the run's fractional-remainder policy to a just-split position (§6.8).
///
/// Returns the cash the settlement moved, which is zero for every policy but
/// `CashInLieu` and for a position that split into whole units anyway.
///
/// The price is the *post-split* mark, because the caller has already adjusted
/// it: a fraction of a share is worth a fraction of what the share is worth now.
/// Like a delisting cash-out, this is not a trade — no venue charges a spread,
/// slippage or commission to settle a fraction it created — so modelling one
/// would invent a cost.
fn settle_fractional_units(
    sim: &mut Sim<'_>,
    symbol: usize,
    symbol_name: &str,
    time_ns: i64,
) -> Result<f64, SimulationError> {
    let policy = sim.config.settings().fractional_units.clone();
    if matches!(policy, FractionalUnitPolicy::KeepFraction) {
        return Ok(0.0);
    }
    let units = sim.positions[symbol].units;
    let (whole, fraction) = CorporateAction::whole_and_fractional_units(units);
    if fraction == 0.0 {
        return Ok(0.0);
    }
    let FractionalUnitPolicy::CashInLieu { .. } = policy else {
        return Err(SimulationError::FractionalUnitsRefused {
            symbol: symbol_name.to_string(),
            units,
        });
    };
    // The declared rule is "the last committed mark". Without one there is no
    // price at all, and a run that asked for cash in lieu does not get a
    // silent zero instead.
    let Some(mark) = sim.marks[symbol] else {
        return Err(SimulationError::CashInLieuWithoutMark {
            symbol: symbol_name.to_string(),
        });
    };
    let rate = sim.conversion_rate(symbol);
    let realized = apply_fill(
        &mut sim.positions[symbol],
        if fraction > 0.0 {
            OrderSide::Sell
        } else {
            OrderSide::Buy
        },
        fraction.abs(),
        mark,
        time_ns,
        rate,
    );
    // `apply_fill` walks the position down by exactly `fraction`, so this is a
    // restatement of the whole part rather than a second, rounding write.
    sim.positions[symbol].units = whole;
    let cash_delta = stable_decimal(fraction * mark * rate);
    sim.cash = finite_accounting("cash", stable_decimal(sim.cash + cash_delta))?;
    sim.realized_pnl_account = stable_decimal(sim.realized_pnl_account + realized);
    Ok(cash_delta)
}

fn cancel_symbol_orders(
    sim: &mut Sim<'_>,
    symbol: usize,
    time_ns: i64,
) -> Result<usize, SimulationError> {
    let resting: Vec<usize> = sim.live[symbol].clone();
    for index in &resting {
        sim.cancel_order(
            *index,
            time_ns,
            priority::CORPORATE,
            CancelReason::CorporateAction,
        )?;
    }
    Ok(resting.len())
}

// ── Financing accrual (§6.3) ───────────────────────────────────────

/// Charge one accrual boundary for one symbol.
///
/// The charge uses the last *committed* mark, so it can never be computed from
/// a price the run has not yet been allowed to see.
fn accrue_financing(sim: &mut Sim<'_>, symbol: usize, time_ns: i64) -> Result<(), SimulationError> {
    let Some(policy) = sim.instruments[symbol].financing.clone() else {
        return Ok(());
    };
    let previous = sim.last_accrual_ns[symbol];
    sim.last_accrual_ns[symbol] = time_ns;
    let units = sim.positions[symbol].units;
    if units == 0.0 {
        return Ok(());
    }
    let Some(mark) = sim.marks[symbol] else {
        // Nothing has closed yet, so there is no price to charge against. A
        // position cannot exist before the first mark either, so this is
        // unreachable in practice and silent only because there is genuinely
        // nothing to say.
        return Ok(());
    };
    // A run's first boundary charges from the previous boundary of the same
    // grid, not from the run's start: financing is a property of the calendar,
    // not of when someone chose to begin simulating.
    let elapsed_seconds = if previous == i64::MIN {
        policy.accrual.seconds()
    } else {
        (time_ns - previous) / NANOS_PER_SECOND
    };

    let breakdown: AccrualBreakdown =
        accrue(&policy, units, mark, elapsed_seconds).map_err(|charge: FinancingCharge| {
            SimulationError::FinancingRateUnavailable {
                symbol: sim.streams[symbol].symbol.clone(),
                charge: charge.wire_id(),
            }
        })?;
    let rate = sim.conversion_rate(symbol);
    let financing = stable_decimal(breakdown.financing * rate);
    let borrow = stable_decimal(breakdown.borrow * rate);
    let funding = stable_decimal(breakdown.funding * rate);
    let total = stable_decimal(financing + borrow + funding);
    if total == 0.0 && financing == 0.0 && borrow == 0.0 && funding == 0.0 {
        return Ok(());
    }

    sim.cash = finite_accounting("cash", stable_decimal(sim.cash - total))?;
    sim.total_financing_cost = finite_accounting(
        "total_financing_cost",
        stable_decimal(sim.total_financing_cost + total),
    )?;
    let sequence = sim.recorder.event(
        time_ns,
        SimEventKind::FundingCharge,
        priority::ACCRUAL,
        Some(SymbolId(symbol)),
        None,
    )?;
    sim.financing_charges.push(FinancingChargeRecord {
        time_ns,
        sequence,
        symbol: SymbolId(symbol),
        units,
        mark_price: mark,
        seconds_accrued: elapsed_seconds,
        financing,
        borrow,
        funding,
        total,
        cash_after: sim.cash,
    });
    Ok(())
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

    let session = sim.session_status(symbol, time_ns);
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
            positions: &sim.positions,
            orders: &sim.orders,
            session,
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
                    filled_quantity: 0.0,
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
    if let Some(tick) = sim.price_tick(symbol.0) {
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
    if sim.delisted[symbol.0] {
        sim.record_rejection(
            time_ns,
            symbol,
            client_id,
            RejectionReason::InstrumentDelisted,
        )?;
        sim.orders[order_index].state = OrderState::Done;
        return Ok(());
    }
    // §6.7: an out-of-session submission either rests until the venue reopens
    // or is refused, per configuration. It never fills out of session either
    // way — the execution phase checks the calendar again at fill time.
    if sim.config.settings().outside_session == OutsideSessionPolicy::Reject
        && let Some(SessionStatus::Closed(reason)) = sim.session_status(symbol.0, time_ns)
    {
        sim.record_rejection(
            time_ns,
            symbol,
            client_id,
            RejectionReason::SessionClosed {
                reason: reason.wire_id().to_string(),
            },
        )?;
        sim.orders[order_index].state = OrderState::Done;
        return Ok(());
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
    if let Some(tick) = sim.price_tick(request.symbol.0) {
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
    let parent = sim.bar(symbol, bar);
    // The row an execution resolves against: the parent bar at levels 1–2, the
    // path step at level 3. The participation window stays the *parent* bar
    // either way, so a finer path cannot multiply the liquidity available.
    let row = match phase {
        Phase::SubBar { sub } => sim.sub_bars[symbol][sub],
        _ => parent,
    };
    let fidelity = sim.config.settings().fidelity;
    let legacy = sim.config.settings().compatibility == ExecutionCompatibility::LegacySameBarClose;

    let time_ns = match phase {
        Phase::Open => row.open_time_ns,
        Phase::SubBar { .. } => row.close_time_ns,
        Phase::Close | Phase::LegacyClose => row.close_time_ns,
    };
    // Nothing executes while the venue is shut, under either out-of-session
    // policy (§6.7). A delisted instrument is shut permanently.
    if !sim.venue_open(symbol, time_ns) {
        return Ok(());
    }

    // Range-based orders had to be live by the row's open to claim its path.
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
        let resting_limit = match phase {
            Phase::Open => resting_stop_limit_at_boundary(order.kind, order.side, row.open, half),
            // A path step resolves exactly like a level-2 bar, against its own
            // range — that is what makes level 3 the same rules at a finer
            // resolution rather than a second execution model.
            Phase::SubBar { .. } => resting_stop_limit_in_bar(order.kind, order.side, &row, half),
            Phase::Close if fidelity == FidelityLevel::BarOhlc => {
                resting_stop_limit_in_bar(order.kind, order.side, &row, half)
            }
            // At level 3 the parent's close phase only serves market-on-close,
            // whose price is the close; the range was already walked.
            Phase::Close if fidelity.sub_bar_seconds().is_some() => None,
            Phase::Close | Phase::LegacyClose if fidelity == FidelityLevel::BarClose => {
                resting_stop_limit_at_boundary(order.kind, order.side, row.close, half)
            }
            Phase::Close | Phase::LegacyClose => None,
        };
        if resting_limit.is_some() {
            converted.push(index);
            continue;
        }
        let trigger = match phase {
            Phase::Open => boundary_trigger(order.kind, order.side, row.open, half),
            Phase::SubBar { .. } => {
                intrabar_trigger(order.kind, order.side, &row, half).map(|(trigger, _)| trigger)
            }
            Phase::LegacyClose => boundary_trigger(
                order.kind,
                order.side,
                row.close,
                sim.half_spread(row.close),
            ),
            Phase::Close => {
                if matches!(order.kind, OrderKind::MarketOnClose) {
                    Some(Trigger {
                        rank: 0,
                        mid: row.close,
                        marketable: true,
                    })
                } else if fidelity == FidelityLevel::BarOhlc {
                    intrabar_trigger(order.kind, order.side, &row, half).map(|(trigger, _)| trigger)
                } else if fidelity.sub_bar_seconds().is_some() {
                    None
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
            let trigger_priority = match phase {
                Phase::Open | Phase::SubBar { .. } => priority::OPEN_EXECUTION,
                Phase::Close | Phase::LegacyClose => priority::CLOSE_EXECUTION,
            };
            sim.recorder.event(
                time_ns,
                SimEventKind::StopTriggered,
                trigger_priority,
                Some(order_symbol),
                Some(client_id),
            )?;
        }
    }

    resolve_oco(sim, &mut candidates);
    candidates.sort_by_key(|(index, _)| sim.orders[*index].submit_sequence);

    let execution_priority = match phase {
        Phase::Open | Phase::SubBar { .. } => priority::OPEN_EXECUTION,
        Phase::Close => priority::CLOSE_EXECUTION,
        Phase::LegacyClose => priority::SUBMIT,
    };

    for (index, trigger) in candidates {
        if sim.orders[index].state != OrderState::Active {
            continue;
        }
        attempt_fill(
            sim,
            index,
            trigger,
            time_ns,
            execution_priority,
            row,
            bar,
            parent.volume,
        )?;
    }

    // An immediate-or-cancel order gets exactly one parent bar to work in, so a
    // path step is never where one dies.
    if !matches!(phase, Phase::SubBar { .. }) {
        let expiring: Vec<usize> = sim.live[symbol]
            .iter()
            .copied()
            .filter(|index| {
                let order = &sim.orders[*index];
                matches!(order.time_in_force, TimeInForce::Ioc | TimeInForce::Fok)
                    && order.state == OrderState::Active
                    && order.active_time_ns <= parent.open_time_ns
                    && (phase.is_close() || order.had_opportunity)
            })
            .collect();
        for index in expiring {
            sim.cancel_order(index, time_ns, execution_priority, CancelReason::Expired)?;
        }
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
    parent_bar: usize,
    parent_volume: f64,
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
    if let OrderKind::Limit { limit_price } = order.kind {
        fill_price = match order.side {
            OrderSide::Buy => fill_price.min(limit_price),
            OrderSide::Sell => fill_price.max(limit_price),
        };
    }

    let position = sim.positions[order.symbol.0].units;
    let remaining = order.remaining();
    let available = sim.window_available(order.symbol.0, parent_bar, parent_volume);
    if matches!(order.time_in_force, TimeInForce::Fok)
        && available.is_some_and(|capacity| capacity < remaining)
    {
        sim.cancel_order(
            order_index,
            time_ns,
            execution_priority,
            CancelReason::FillOrKillUnfilled,
        )?;
        return Ok(());
    }
    let fill_quantity = available.map_or(remaining, |capacity| remaining.min(capacity));
    if fill_quantity <= 0.0 {
        return Ok(());
    }

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
        if remaining > position.abs() {
            sim.record_rejection(
                time_ns,
                order.symbol,
                order.client_id,
                RejectionReason::ReduceOnlyExceedsPosition {
                    position,
                    quantity: remaining,
                },
            )?;
            sim.retire(order_index);
            return Ok(());
        }
    }

    // Per-order commission is charged once, on the first execution. Other
    // schedules are evaluated per execution against the actual fill.
    let fee = finite_accounting(
        "commission",
        stable_decimal(
            if order.filled_quantity > 0.0
                && matches!(settings.commission, CommissionModel::PerOrder { .. })
            {
                0.0
            } else {
                commission(&settings.commission, order.side, fill_quantity, fill_price)
            },
        ),
    )?;
    let rate = sim.conversion_rate(order.symbol.0);
    let converted_notional = finite_accounting(
        "converted_notional",
        stable_decimal(fill_price * fill_quantity * rate),
    )?;
    let conversion_cost = finite_accounting(
        "conversion_cost",
        stable_decimal(
            converted_notional.abs() * sim.instruments[order.symbol.0].conversion_spread_percent
                / 100.0,
        ),
    )?;
    let next_cash = finite_accounting(
        "cash",
        stable_decimal(sim.cash - order.side.sign() * converted_notional - fee - conversion_cost),
    )?;
    let next_units = position + order.side.sign() * fill_quantity;
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
                    required: stable_decimal(converted_notional + fee + conversion_cost),
                },
            )?;
            sim.retire(order_index);
            return Ok(());
        }
    }

    let spread_cost = finite_accounting(
        "spread_cost",
        stable_decimal(width / 2.0 * fill_quantity * rate),
    )?;
    let slippage_cost =
        finite_accounting("slippage_cost", stable_decimal(slip * fill_quantity * rate))?;
    sim.cash = next_cash;
    sim.total_commission = finite_accounting(
        "total_commission",
        stable_decimal(sim.total_commission + fee),
    )?;
    sim.total_conversion_cost = finite_accounting(
        "total_conversion_cost",
        stable_decimal(sim.total_conversion_cost + conversion_cost),
    )?;
    let realized_pnl = apply_fill(
        &mut sim.positions[order.symbol.0],
        order.side,
        fill_quantity,
        fill_price,
        time_ns,
        rate,
    );
    finite_accounting("realized_pnl", realized_pnl)?;
    sim.realized_pnl_account = finite_accounting(
        "realized_pnl_account",
        stable_decimal(sim.realized_pnl_account + realized_pnl),
    )?;
    finite_accounting("position_units", sim.positions[order.symbol.0].units)?;
    finite_accounting("average_entry", sim.positions[order.symbol.0].avg_entry)?;
    finite_accounting(
        "position_realized_pnl",
        sim.positions[order.symbol.0].realized_pnl,
    )?;

    sim.orders[order_index].filled_quantity =
        stable_decimal(sim.orders[order_index].filled_quantity + fill_quantity);
    sim.consume_window(order.symbol.0, fill_quantity);
    let remaining_quantity = sim.orders[order_index].remaining();
    let complete = remaining_quantity == 0.0;
    let sequence = sim.recorder.event(
        time_ns,
        if complete {
            SimEventKind::Fill
        } else {
            SimEventKind::PartialFill
        },
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
        quantity: fill_quantity,
        remaining_quantity,
        reference_price: reference,
        quoted_price: quoted,
        fill_price,
        spread_cost,
        slippage_cost,
        commission: fee,
        conversion_rate: rate,
        conversion_cost,
        realized_pnl,
        cash_after: sim.cash,
        position_units_after: state.units,
        avg_entry_after: state.avg_entry,
    });
    if complete {
        sim.retire(order_index);
    }

    if let Some(group) = order.oco_group {
        let siblings: Vec<usize> = sim.live[order.symbol.0]
            .iter()
            .copied()
            .filter(|index| *index != order_index && sim.orders[*index].oco_group == Some(group))
            .collect();
        for sibling in siblings {
            if complete {
                sim.cancel_order(
                    sibling,
                    time_ns,
                    priority::OCO_CANCEL,
                    CancelReason::OcoSibling,
                )?;
                continue;
            }
            let sibling_remaining = sim.orders[sibling].remaining();
            if sibling_remaining <= fill_quantity {
                sim.cancel_order(
                    sibling,
                    time_ns,
                    priority::OCO_CANCEL,
                    CancelReason::OcoSiblingConsumed,
                )?;
            } else {
                sim.orders[sibling].quantity =
                    stable_decimal(sim.orders[sibling].quantity - fill_quantity);
            }
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
                    filled_quantity: 0.0,
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
                    usize::MAX,
                    f64::INFINITY,
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
        marked_equity(sim.cash, &sim.positions, &sim.marks, &sim.instruments),
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
            filled_quantity: order.filled_quantity,
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
        financing_charges: sim.financing_charges,
        corporate_actions: sim.corporate_records,
        final_cash: sim.cash,
        final_equity,
        final_realized_pnl,
        total_commission: sim.total_commission,
        total_conversion_cost: sim.total_conversion_cost,
        total_financing_cost: sim.total_financing_cost,
    })
}

#[cfg(test)]
mod tests;
