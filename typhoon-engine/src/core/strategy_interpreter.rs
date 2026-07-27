//! Deterministic scalar reference interpreter for the canonical strategy IR.
//!
//! This is the authoritative *scalar* lowering of a sealed [`StrategyIr`] onto
//! the deterministic [`strategy_simulator`](crate::core::strategy_simulator).
//! It is the oracle a future vectorised or GPU evaluator must reproduce, so
//! every rule below is stated rather than inferred, and anything the current
//! simulator cannot express is refused with a typed error instead of being
//! approximated.
//!
//! ## What it evaluates
//!
//! At each `on_bar_close` decision the interpreter advances the deciding
//! symbol's indicator graph by exactly one bar, evaluates the direction rules,
//! and lowers them to market [`OrderIntents`]. It reads only committed bars
//! through [`MarketView`]'s historical accessors — `bars_ago = 0` is the bar
//! that just closed — so no rule can observe a price that has not happened.
//!
//! ## Three-valued conditions
//!
//! An operand that is not yet available (an indicator still warming up, a
//! lookback deeper than the committed history) is *unknown*, not false.
//! Conditions therefore evaluate in Kleene three-valued logic: `All` is false
//! if any child is false and unknown if any remaining child is unknown, `Any`
//! is true if any child is true and unknown if any remaining child is unknown,
//! and `Not` maps unknown to unknown. Only `True` triggers an action, so a
//! `Not` over a warming-up operand cannot fire a trade. Children are all
//! evaluated (no short-circuit) so a malformed reference is reported whatever
//! the data does.
//!
//! ## Indicator semantics
//!
//! Every built-in is a small deterministic state machine advanced one sample
//! per bar. A node whose series input is unavailable for a bar does not
//! consume a sample — warm-up composes down a chain — but the node still
//! records an unavailable value for that bar, so `bars_ago` stays bar-aligned.
//!
//! | kind | inputs | first value | formula |
//! |------|--------|-------------|---------|
//! | `sma` | series, period `n` | sample `n-1` | arithmetic mean of the last `n` samples |
//! | `std_dev` | series, `n` | `n-1` | population standard deviation of the last `n` |
//! | `ema` | series, `n` | `n-1` | seeded with `sma(n)`, then `a*x + (1-a)*prev`, `a = 2/(n+1)` |
//! | `atr` | `n` | bar `n-1` | Wilder: seed `mean(tr)`, then `(prev*(n-1) + tr)/n`; the first visible bar's true range is `high - low` |
//! | `rsi` | series, `n` | sample `n` | Wilder averages of gains/losses; `100` when average loss is zero, `50` when both are zero |
//! | `adx` | `n` | bar `2n-1` | Wilder-summed `+DM`/`-DM`/`TR`, `DX = 100*abs(+DI - -DI)/(+DI + -DI)`, then a Wilder average of `DX` |
//! | `kama` | series, `er`, `fast`, `slow` | sample `er` | efficiency ratio over `er` diffs, `sc = (er*(af - as) + as)^2`; the first value seeds at the sample |
//! | `fisher_transform` | series, `n` | sample `n-1` | Ehlers: `v = 0.66*(raw - 0.5) + 0.67*v'` clamped to ±0.999, `fish = 0.5*ln((1+v)/(1-v)) + 0.5*fish'` |
//! | `macd` | series, `fast`, `slow`, `signal` | sample `max(fast,slow) + signal - 2` | histogram: `ema(fast) - ema(slow)` minus its own `ema(signal)` |
//!
//! `macd` is the histogram rather than the raw line because a scalar-valued
//! node has one output and the declared signal period must mean something.
//! [`IndicatorKind::Custom`] is refused: an opaque implementation has no
//! reference semantics this module could claim to define.
//!
//! ## Lowering to orders
//!
//! Per decision, in this order:
//!
//! 1. If the tracked position is long and `long.exit` is true, sell it; if it
//!    is short and `short.exit` is true, buy it back.
//! 2. If the position is now flat, evaluate both entries. Exactly one true
//!    entry opens a position; two true entries stand aside, because the IR
//!    does not rank directions and guessing one would be silent policy.
//! 3. An entry in the direction already held does nothing — no pyramiding.
//! 4. A position is never flipped by the opposite entry alone. A reversal
//!    happens only when the held direction's exit is true at the same decision
//!    as the opposite entry, and is emitted as two intents (close, then open)
//!    so both legs appear in the fill ledger.
//! 5. `sizing.max_open_positions` counts symbols the interpreter holds. A
//!    reversal keeps the slot it just released, so it is never blocked.
//!
//! Orders are market intents; the simulator fills them at the *next* bar open,
//! so nothing executes on the bar that decided it.
//!
//! ## Bounded state
//!
//! State is one entry per symbol seen (capped by
//! [`strategy_simulator::MAX_SYMBOLS`]), each holding one fixed-capacity ring
//! of past values plus one fixed-capacity window per indicator. Ring capacity
//! is the deepest lookback the strategy references plus two (a cross reads one
//! bar further back). The per-symbol total is capped by
//! [`MAX_STATE_SLOTS_PER_SYMBOL`] at build time. There are no maps and nothing
//! grows with the length of the run.
//!
//! Per-symbol state is only valid for one contiguous replay of one stream. The
//! interpreter counts the bars it has been shown and cross-checks that count
//! against the market view on every decision, so reusing an instance for a
//! second run is reported rather than silently mixing histories. Call
//! [`CanonicalIrStrategy::reset`] between runs.
//!
//! ## Not implemented here
//!
//! Refused at build time with [`InterpreterError::Unsupported`], because the
//! simulator exposes no way to honour them: session and news filters (no
//! decision timestamp), percent-of-equity and risk-based sizing (no account
//! state), protective stops, targets, trails, break-even and time stops (no
//! resting orders), non-`closed_bar` decision timing, and submission delay.

use crate::core::strategy_ir::{
    CompareOp, Condition, DecisionTiming, DirectionRules, IndicatorInput, IndicatorKind,
    IndicatorNode, MAX_BARS_AGO, MAX_CONDITION_DEPTH, Operand, ParamValue, PriceField, SizingRule,
    StrategyDefinition, StrategyIr, StrategyIrError,
};
use crate::core::strategy_simulator::{
    DecisionContext, MAX_ORDER_QUANTITY, MAX_SYMBOLS, MarketDataError, MarketView, OrderIntents,
    OrderSide, ReferenceStrategy, StrategyError, SymbolId,
};
use std::collections::BTreeMap;
use std::collections::VecDeque;
use std::error::Error;
use std::fmt;

/// Largest indicator period the interpreter will build state for.
pub const MAX_INDICATOR_PERIOD: usize = 4_096;

/// Ceiling on one symbol's interpreter state, counted in `f64` slots across
/// every indicator's history ring and internal windows.
pub const MAX_STATE_SLOTS_PER_SYMBOL: usize = 16_384;

// ── Errors ─────────────────────────────────────────────────────────

/// Everything the interpreter refuses to do, at build time or mid-run.
#[derive(Debug, Clone, PartialEq)]
pub enum InterpreterError {
    /// The supplied IR failed its own verification.
    InvalidIr(StrategyIrError),
    /// A declared feature has no faithful lowering onto the simulator.
    Unsupported {
        feature: &'static str,
        detail: &'static str,
    },
    /// An indicator kind the interpreter has no reference formula for.
    UnsupportedIndicator { indicator: String, kind: String },
    /// A reference that survived IR validation but does not resolve here.
    UnknownRef {
        kind: &'static str,
        id: String,
        context: String,
    },
    /// A parameter used as a number does not hold one.
    NonNumericParameter { id: String, context: String },
    /// A period input is not a whole number in `1..=MAX_INDICATOR_PERIOD`.
    InvalidPeriod { indicator: String, value: f64 },
    /// A fixed order size the simulator would reject.
    InvalidQuantity { units: f64 },
    /// The indicator graph does not linearise.
    IndicatorCycle { indicator: String },
    /// A compiled operand points past the end of the indicator table.
    UnknownIndicatorSlot { slot: usize, count: usize },
    /// A compiled operand looks back further than the state it was sized for.
    LookbackOutOfRange { bars_ago: usize, capacity: usize },
    /// One symbol's state would exceed [`MAX_STATE_SLOTS_PER_SYMBOL`].
    StateTooLarge { limit: usize, found: usize },
    /// A condition tree is deeper than the IR permits.
    ConditionTooDeep { limit: usize },
    /// A calculator was handed the wrong input shape.
    InputShapeMismatch {
        indicator: String,
        expected: &'static str,
    },
    /// The simulator addressed a symbol outside its own bound.
    SymbolOutOfRange { id: usize, limit: usize },
    /// The decision stream skipped or repeated a bar for this symbol.
    HistoryDesynchronized {
        symbol: usize,
        expected: usize,
        observed: usize,
    },
    /// The committed-bar probe answered an impossible lookback.
    UnexpectedHistory { symbol: usize },
    /// A market read failed for a reason other than missing history.
    MarketData(MarketDataError),
    /// A guarded formula still produced a non-finite value.
    NonFiniteIndicator { indicator: String },
    /// The simulator rejected an intent the interpreter submitted.
    Order(StrategyError),
}

impl fmt::Display for InterpreterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIr(error) => write!(f, "invalid strategy ir: {error}"),
            Self::Unsupported { feature, detail } => {
                write!(f, "unsupported `{feature}`: {detail}")
            }
            Self::UnsupportedIndicator { indicator, kind } => {
                write!(f, "indicator `{indicator}` has unsupported kind `{kind}`")
            }
            Self::UnknownRef { kind, id, context } => {
                write!(f, "unknown {kind} `{id}` referenced by {context}")
            }
            Self::NonNumericParameter { id, context } => {
                write!(f, "parameter `{id}` used by {context} is not numeric")
            }
            Self::InvalidPeriod { indicator, value } => write!(
                f,
                "indicator `{indicator}` has period {value}, expected a whole number in 1..={MAX_INDICATOR_PERIOD}"
            ),
            Self::InvalidQuantity { units } => write!(f, "invalid fixed order size {units}"),
            Self::IndicatorCycle { indicator } => {
                write!(f, "indicator `{indicator}` takes part in a cycle")
            }
            Self::UnknownIndicatorSlot { slot, count } => {
                write!(f, "indicator slot {slot} is outside the {count} compiled")
            }
            Self::LookbackOutOfRange { bars_ago, capacity } => write!(
                f,
                "lookback {bars_ago} exceeds the {capacity} bars of retained state"
            ),
            Self::StateTooLarge { limit, found } => {
                write!(f, "per-symbol state needs {found} slots, limit {limit}")
            }
            Self::ConditionTooDeep { limit } => {
                write!(f, "condition nests deeper than {limit}")
            }
            Self::InputShapeMismatch {
                indicator,
                expected,
            } => write!(f, "indicator `{indicator}` expects a {expected} input"),
            Self::SymbolOutOfRange { id, limit } => {
                write!(
                    f,
                    "symbol id {id} is outside the {limit} the simulator bounds"
                )
            }
            Self::HistoryDesynchronized {
                symbol,
                expected,
                observed,
            } => write!(
                f,
                "symbol {symbol} committed {observed} bars, interpreter expected {expected}"
            ),
            Self::UnexpectedHistory { symbol } => {
                write!(f, "symbol {symbol} answered an unbounded lookback")
            }
            Self::MarketData(error) => write!(f, "market data unavailable: {error}"),
            Self::NonFiniteIndicator { indicator } => {
                write!(f, "indicator `{indicator}` produced a non-finite value")
            }
            Self::Order(error) => write!(f, "order rejected: {error}"),
        }
    }
}
impl Error for InterpreterError {}

// ── Three-valued logic ─────────────────────────────────────────────

/// Kleene truth. `Unknown` is "not observable yet", never "false".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Truth {
    True,
    False,
    Unknown,
}

impl Truth {
    const fn of(value: bool) -> Self {
        if value { Self::True } else { Self::False }
    }

    const fn negate(self) -> Self {
        match self {
            Self::True => Self::False,
            Self::False => Self::True,
            Self::Unknown => Self::Unknown,
        }
    }
}

// ── Compiled program ───────────────────────────────────────────────

/// Where an indicator node reads its samples.
#[derive(Debug, Clone, Copy, PartialEq)]
enum SourceRef {
    /// The bar itself, for the range-based indicators.
    Bar,
    Price(PriceField),
    Node(usize),
}

/// A built-in indicator with its periods resolved to whole numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompiledKind {
    Sma {
        period: usize,
    },
    StdDev {
        period: usize,
    },
    Ema {
        period: usize,
    },
    Atr {
        period: usize,
    },
    Rsi {
        period: usize,
    },
    Adx {
        period: usize,
    },
    Fisher {
        period: usize,
    },
    Kama {
        er_period: usize,
        fast: usize,
        slow: usize,
    },
    Macd {
        fast: usize,
        slow: usize,
        signal: usize,
    },
}

impl CompiledKind {
    /// `f64` slots the calculator's internal windows occupy.
    const fn window_slots(self) -> usize {
        match self {
            Self::Sma { period }
            | Self::StdDev { period }
            | Self::Ema { period }
            | Self::Atr { period }
            | Self::Fisher { period } => period,
            Self::Rsi { period } => 2 * period,
            Self::Adx { period } => 4 * period,
            Self::Kama { er_period, .. } => er_period + 1,
            Self::Macd { fast, slow, signal } => fast + slow + signal,
        }
    }
}

#[derive(Debug, Clone)]
struct CompiledNode {
    id: String,
    kind: CompiledKind,
    source: SourceRef,
}

/// A condition operand with parameters already folded to their typed values.
#[derive(Debug, Clone, Copy, PartialEq)]
enum CompiledOperand {
    Constant(f64),
    Price { field: PriceField, bars_ago: usize },
    Indicator { slot: usize, bars_ago: usize },
}

#[derive(Debug, Clone, PartialEq)]
enum CompiledCondition {
    Always,
    Never,
    Not(Box<CompiledCondition>),
    All(Vec<CompiledCondition>),
    Any(Vec<CompiledCondition>),
    Compare {
        left: CompiledOperand,
        op: CompareOp,
        right: CompiledOperand,
    },
    Cross {
        left: CompiledOperand,
        right: CompiledOperand,
        above: bool,
    },
}

#[derive(Debug, Clone)]
struct CompiledRules {
    enabled: bool,
    entry: CompiledCondition,
    exit: CompiledCondition,
}

/// The whole strategy, resolved once and then read-only for the run.
#[derive(Debug, Clone)]
struct Program {
    nodes: Vec<CompiledNode>,
    /// Node indices in dependency order.
    order: Vec<usize>,
    /// Retained values per node, per symbol.
    history: usize,
    long: CompiledRules,
    short: CompiledRules,
    units: f64,
    max_open_positions: usize,
}

// ── Bounded runtime state ──────────────────────────────────────────

/// A fixed-capacity window of the most recent samples, oldest first.
#[derive(Debug, Clone)]
struct Window {
    capacity: usize,
    values: VecDeque<f64>,
}

impl Window {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            values: VecDeque::with_capacity(capacity),
        }
    }

    fn push(&mut self, value: f64) {
        if self.values.len() == self.capacity {
            self.values.pop_front();
        }
        self.values.push_back(value);
    }

    fn is_full(&self) -> bool {
        self.values.len() == self.capacity
    }

    /// Summed oldest to newest so the rounding is a property of the series,
    /// not of the traversal.
    fn sum(&self) -> f64 {
        self.values.iter().sum()
    }

    fn mean(&self) -> f64 {
        if self.values.is_empty() {
            0.0
        } else {
            self.sum() / self.values.len() as f64
        }
    }

    fn oldest(&self) -> Option<f64> {
        self.values.front().copied()
    }

    fn extent(&self) -> Option<(f64, f64)> {
        let first = self.values.front().copied()?;
        Some(
            self.values
                .iter()
                .fold((first, first), |(min, max), value| {
                    (min.min(*value), max.max(*value))
                }),
        )
    }

    /// Sum of absolute consecutive differences — KAMA's volatility term.
    fn absolute_travel(&self) -> f64 {
        self.values
            .iter()
            .zip(self.values.iter().skip(1))
            .map(|(previous, current)| (current - previous).abs())
            .sum()
    }
}

/// A fixed-capacity ring of one node's past values, indexed by `bars_ago`.
#[derive(Debug, Clone)]
struct History {
    values: Vec<Option<f64>>,
    cursor: usize,
    written: usize,
}

impl History {
    fn new(capacity: usize) -> Self {
        Self {
            values: vec![None; capacity.max(1)],
            cursor: 0,
            written: 0,
        }
    }

    fn push(&mut self, value: Option<f64>) {
        let capacity = self.values.len();
        self.values[self.cursor] = value;
        self.cursor = (self.cursor + 1) % capacity;
        self.written = self.written.saturating_add(1);
    }

    /// `bars_ago = 0` is the value pushed for the bar that just closed.
    /// A lookback the ring was never sized for is a program defect, not a
    /// warm-up; a lookback deeper than what has been written is warm-up.
    fn get(&self, bars_ago: usize) -> Result<Option<f64>, InterpreterError> {
        let capacity = self.values.len();
        if bars_ago >= capacity {
            return Err(InterpreterError::LookbackOutOfRange { bars_ago, capacity });
        }
        if bars_ago >= self.written {
            return Ok(None);
        }
        let index = (self.cursor + capacity - 1 - bars_ago) % capacity;
        Ok(self.values[index])
    }
}

/// One sample handed to a calculator.
#[derive(Debug, Clone, Copy, PartialEq)]
enum CalcInput {
    Series(f64),
    Bar { high: f64, low: f64, close: f64 },
}

impl CalcInput {
    fn series(self, indicator: &str) -> Result<f64, InterpreterError> {
        match self {
            Self::Series(value) => Ok(value),
            Self::Bar { .. } => Err(InterpreterError::InputShapeMismatch {
                indicator: indicator.to_string(),
                expected: "series",
            }),
        }
    }

    fn bar(self, indicator: &str) -> Result<(f64, f64, f64), InterpreterError> {
        match self {
            Self::Bar { high, low, close } => Ok((high, low, close)),
            Self::Series(_) => Err(InterpreterError::InputShapeMismatch {
                indicator: indicator.to_string(),
                expected: "bar",
            }),
        }
    }
}

/// An exponential average seeded from the simple average of its first window,
/// so it has one value per bar from a stated warm-up onwards.
#[derive(Debug, Clone)]
struct EmaState {
    seed: Window,
    alpha: f64,
    value: Option<f64>,
}

impl EmaState {
    fn new(period: usize) -> Self {
        Self {
            seed: Window::new(period),
            alpha: 2.0 / (period as f64 + 1.0),
            value: None,
        }
    }

    fn update(&mut self, sample: f64) -> Option<f64> {
        let next = match self.value {
            Some(previous) => self.alpha * sample + (1.0 - self.alpha) * previous,
            None => {
                self.seed.push(sample);
                if !self.seed.is_full() {
                    return None;
                }
                self.seed.mean()
            }
        };
        self.value = Some(next);
        Some(next)
    }
}

#[derive(Debug, Clone)]
struct AtrState {
    period: usize,
    seed: Window,
    previous_close: Option<f64>,
    value: Option<f64>,
}

impl AtrState {
    fn new(period: usize) -> Self {
        Self {
            period,
            seed: Window::new(period),
            previous_close: None,
            value: None,
        }
    }

    fn update(&mut self, high: f64, low: f64, close: f64) -> Option<f64> {
        let range = true_range(high, low, self.previous_close);
        self.previous_close = Some(close);
        let next = match self.value {
            Some(previous) => {
                let period = self.period as f64;
                (previous * (period - 1.0) + range) / period
            }
            None => {
                self.seed.push(range);
                if !self.seed.is_full() {
                    return None;
                }
                self.seed.mean()
            }
        };
        self.value = Some(next);
        Some(next)
    }
}

#[derive(Debug, Clone)]
struct RsiState {
    period: usize,
    previous: Option<f64>,
    gains: Window,
    losses: Window,
    average_gain: Option<f64>,
    average_loss: Option<f64>,
}

impl RsiState {
    fn new(period: usize) -> Self {
        Self {
            period,
            previous: None,
            gains: Window::new(period),
            losses: Window::new(period),
            average_gain: None,
            average_loss: None,
        }
    }

    fn update(&mut self, sample: f64) -> Option<f64> {
        let previous = self.previous.replace(sample)?;
        let change = sample - previous;
        let gain = change.max(0.0);
        let loss = (-change).max(0.0);
        let (average_gain, average_loss) = match (self.average_gain, self.average_loss) {
            (Some(up), Some(down)) => {
                let period = self.period as f64;
                (
                    (up * (period - 1.0) + gain) / period,
                    (down * (period - 1.0) + loss) / period,
                )
            }
            _ => {
                self.gains.push(gain);
                self.losses.push(loss);
                if !self.gains.is_full() {
                    return None;
                }
                (self.gains.mean(), self.losses.mean())
            }
        };
        self.average_gain = Some(average_gain);
        self.average_loss = Some(average_loss);
        Some(if average_loss == 0.0 {
            if average_gain == 0.0 { 50.0 } else { 100.0 }
        } else {
            100.0 - 100.0 / (1.0 + average_gain / average_loss)
        })
    }
}

#[derive(Debug, Clone)]
struct AdxState {
    period: usize,
    previous: Option<(f64, f64, f64)>,
    range_seed: Window,
    plus_seed: Window,
    minus_seed: Window,
    smoothed: Option<(f64, f64, f64)>,
    dx_seed: Window,
    value: Option<f64>,
}

impl AdxState {
    fn new(period: usize) -> Self {
        Self {
            period,
            previous: None,
            range_seed: Window::new(period),
            plus_seed: Window::new(period),
            minus_seed: Window::new(period),
            smoothed: None,
            dx_seed: Window::new(period),
            value: None,
        }
    }

    fn update(&mut self, high: f64, low: f64, close: f64) -> Option<f64> {
        let (previous_high, previous_low, previous_close) =
            self.previous.replace((high, low, close))?;
        let up = high - previous_high;
        let down = previous_low - low;
        let plus = if up > down && up > 0.0 { up } else { 0.0 };
        let minus = if down > up && down > 0.0 { down } else { 0.0 };
        let range = true_range(high, low, Some(previous_close));

        let period = self.period as f64;
        let (range_sum, plus_sum, minus_sum) = match self.smoothed {
            Some((total_range, total_plus, total_minus)) => (
                total_range - total_range / period + range,
                total_plus - total_plus / period + plus,
                total_minus - total_minus / period + minus,
            ),
            None => {
                self.range_seed.push(range);
                self.plus_seed.push(plus);
                self.minus_seed.push(minus);
                if !self.range_seed.is_full() {
                    return None;
                }
                (
                    self.range_seed.sum(),
                    self.plus_seed.sum(),
                    self.minus_seed.sum(),
                )
            }
        };
        self.smoothed = Some((range_sum, plus_sum, minus_sum));

        let (plus_di, minus_di) = if range_sum == 0.0 {
            (0.0, 0.0)
        } else {
            (100.0 * plus_sum / range_sum, 100.0 * minus_sum / range_sum)
        };
        let spread = plus_di + minus_di;
        let dx = if spread == 0.0 {
            0.0
        } else {
            100.0 * (plus_di - minus_di).abs() / spread
        };

        let next = match self.value {
            Some(previous) => (previous * (period - 1.0) + dx) / period,
            None => {
                self.dx_seed.push(dx);
                if !self.dx_seed.is_full() {
                    return None;
                }
                self.dx_seed.mean()
            }
        };
        self.value = Some(next);
        Some(next)
    }
}

#[derive(Debug, Clone)]
struct KamaState {
    window: Window,
    fast_alpha: f64,
    slow_alpha: f64,
    value: Option<f64>,
}

impl KamaState {
    fn new(er_period: usize, fast: usize, slow: usize) -> Self {
        Self {
            window: Window::new(er_period + 1),
            fast_alpha: 2.0 / (fast as f64 + 1.0),
            slow_alpha: 2.0 / (slow as f64 + 1.0),
            value: None,
        }
    }

    fn update(&mut self, sample: f64) -> Option<f64> {
        self.window.push(sample);
        if !self.window.is_full() {
            return None;
        }
        let oldest = self.window.oldest()?;
        let travel = self.window.absolute_travel();
        // The direct move can never exceed the path length; the clamp only
        // guards accumulated rounding.
        let efficiency = if travel == 0.0 {
            0.0
        } else {
            ((sample - oldest).abs() / travel).clamp(0.0, 1.0)
        };
        let constant = (efficiency * (self.fast_alpha - self.slow_alpha) + self.slow_alpha).powi(2);
        let next = match self.value {
            Some(previous) => previous + constant * (sample - previous),
            None => sample,
        };
        self.value = Some(next);
        Some(next)
    }
}

#[derive(Debug, Clone)]
struct FisherState {
    window: Window,
    value: f64,
    transform: f64,
}

impl FisherState {
    fn new(period: usize) -> Self {
        Self {
            window: Window::new(period),
            value: 0.0,
            transform: 0.0,
        }
    }

    fn update(&mut self, sample: f64) -> Option<f64> {
        self.window.push(sample);
        if !self.window.is_full() {
            return None;
        }
        let (low, high) = self.window.extent()?;
        let position = if high == low {
            0.5
        } else {
            (sample - low) / (high - low)
        };
        // Clamped before the log so the transform stays finite on a range
        // break-out.
        let value = (0.66 * (position - 0.5) + 0.67 * self.value).clamp(-0.999, 0.999);
        self.value = value;
        self.transform = 0.5 * ((1.0 + value) / (1.0 - value)).ln() + 0.5 * self.transform;
        Some(self.transform)
    }
}

#[derive(Debug, Clone)]
struct MacdState {
    fast: EmaState,
    slow: EmaState,
    signal: EmaState,
}

impl MacdState {
    fn new(fast: usize, slow: usize, signal: usize) -> Self {
        Self {
            fast: EmaState::new(fast),
            slow: EmaState::new(slow),
            signal: EmaState::new(signal),
        }
    }

    fn update(&mut self, sample: f64) -> Option<f64> {
        // Both averages must consume every sample, so neither call is skipped.
        let fast = self.fast.update(sample);
        let slow = self.slow.update(sample);
        let (Some(fast), Some(slow)) = (fast, slow) else {
            return None;
        };
        let line = fast - slow;
        let signal = self.signal.update(line)?;
        Some(line - signal)
    }
}

#[derive(Debug, Clone)]
enum Calc {
    Sma(Window),
    StdDev(Window),
    Ema(EmaState),
    Atr(AtrState),
    Rsi(RsiState),
    Adx(AdxState),
    Fisher(FisherState),
    Kama(KamaState),
    Macd(MacdState),
}

impl Calc {
    fn new(kind: CompiledKind) -> Self {
        match kind {
            CompiledKind::Sma { period } => Self::Sma(Window::new(period)),
            CompiledKind::StdDev { period } => Self::StdDev(Window::new(period)),
            CompiledKind::Ema { period } => Self::Ema(EmaState::new(period)),
            CompiledKind::Atr { period } => Self::Atr(AtrState::new(period)),
            CompiledKind::Rsi { period } => Self::Rsi(RsiState::new(period)),
            CompiledKind::Adx { period } => Self::Adx(AdxState::new(period)),
            CompiledKind::Fisher { period } => Self::Fisher(FisherState::new(period)),
            CompiledKind::Kama {
                er_period,
                fast,
                slow,
            } => Self::Kama(KamaState::new(er_period, fast, slow)),
            CompiledKind::Macd { fast, slow, signal } => {
                Self::Macd(MacdState::new(fast, slow, signal))
            }
        }
    }

    fn update(
        &mut self,
        input: CalcInput,
        indicator: &str,
    ) -> Result<Option<f64>, InterpreterError> {
        let value = match self {
            Self::Sma(window) => {
                window.push(input.series(indicator)?);
                window.is_full().then(|| window.mean())
            }
            Self::StdDev(window) => {
                window.push(input.series(indicator)?);
                window.is_full().then(|| population_deviation(window))
            }
            Self::Ema(state) => state.update(input.series(indicator)?),
            Self::Rsi(state) => state.update(input.series(indicator)?),
            Self::Fisher(state) => state.update(input.series(indicator)?),
            Self::Kama(state) => state.update(input.series(indicator)?),
            Self::Macd(state) => state.update(input.series(indicator)?),
            Self::Atr(state) => {
                let (high, low, close) = input.bar(indicator)?;
                state.update(high, low, close)
            }
            Self::Adx(state) => {
                let (high, low, close) = input.bar(indicator)?;
                state.update(high, low, close)
            }
        };
        // The formulas above are guarded against division by zero and against
        // the log's poles, so this is a defect check rather than a data path.
        if value.is_some_and(|value| !value.is_finite()) {
            return Err(InterpreterError::NonFiniteIndicator {
                indicator: indicator.to_string(),
            });
        }
        Ok(value)
    }
}

fn true_range(high: f64, low: f64, previous_close: Option<f64>) -> f64 {
    match previous_close {
        // The first visible bar has no previous close, so its true range is
        // its own span.
        None => high - low,
        Some(close) => (high - low)
            .max((high - close).abs())
            .max((low - close).abs()),
    }
}

fn population_deviation(window: &Window) -> f64 {
    let mean = window.mean();
    let count = window.values.len() as f64;
    if count == 0.0 {
        return 0.0;
    }
    let variance = window
        .values
        .iter()
        .map(|value| (value - mean) * (value - mean))
        .sum::<f64>()
        / count;
    variance.sqrt()
}

#[derive(Debug, Clone)]
struct NodeState {
    calc: Calc,
    history: History,
}

/// What the interpreter believes it holds. Fills are deterministic, so this
/// tracks the position the simulator will have opened by the next bar.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Position {
    Flat,
    Long { units: f64 },
    Short { units: f64 },
}

#[derive(Debug, Clone)]
struct SymbolState {
    bars: usize,
    nodes: Vec<NodeState>,
    position: Position,
}

impl SymbolState {
    fn new(program: &Program) -> Self {
        Self {
            bars: 0,
            nodes: program
                .nodes
                .iter()
                .map(|node| NodeState {
                    calc: Calc::new(node.kind),
                    history: History::new(program.history),
                })
                .collect(),
            position: Position::Flat,
        }
    }
}

// ── Compilation ────────────────────────────────────────────────────

fn compile(definition: &StrategyDefinition) -> Result<Program, InterpreterError> {
    compile_timing(definition)?;
    compile_filters(definition)?;
    compile_trade_management(definition)?;
    let units = compile_sizing(definition)?;

    let parameters: BTreeMap<&str, &ParamValue> = definition
        .parameters
        .iter()
        .map(|parameter| (parameter.id.as_str(), &parameter.value))
        .collect();
    let slots: BTreeMap<&str, usize> = definition
        .indicators
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.as_str(), index))
        .collect();

    let nodes = definition
        .indicators
        .iter()
        .map(|node| compile_node(node, &parameters, &slots))
        .collect::<Result<Vec<_>, _>>()?;
    let order = dependency_order(&nodes)?;

    let mut deepest = 0usize;
    let long = compile_rules(&definition.long, &parameters, &slots, &mut deepest)?;
    let short = compile_rules(&definition.short, &parameters, &slots, &mut deepest)?;
    // A cross reads one bar behind its deepest operand.
    let history = deepest + 2;

    let slots_needed = nodes
        .iter()
        .map(|node| history + node.kind.window_slots())
        .sum::<usize>();
    if slots_needed > MAX_STATE_SLOTS_PER_SYMBOL {
        return Err(InterpreterError::StateTooLarge {
            limit: MAX_STATE_SLOTS_PER_SYMBOL,
            found: slots_needed,
        });
    }

    Ok(Program {
        nodes,
        order,
        history,
        long,
        short,
        units,
        max_open_positions: definition.sizing.max_open_positions as usize,
    })
}

fn compile_timing(definition: &StrategyDefinition) -> Result<(), InterpreterError> {
    match definition.timing.decision {
        DecisionTiming::ClosedBar => {}
        DecisionTiming::NextBarOpen => {
            return Err(InterpreterError::Unsupported {
                feature: "timing.decision.next_bar_open",
                detail: "the simulator only offers a decision at the bar close",
            });
        }
        DecisionTiming::PreClose { .. } => {
            return Err(InterpreterError::Unsupported {
                feature: "timing.decision.pre_close",
                detail: "the simulator exposes no forming bar to decide against",
            });
        }
    }
    if definition.timing.forming_bar_visible {
        return Err(InterpreterError::Unsupported {
            feature: "timing.forming_bar_visible",
            detail: "only committed bars are readable through the market view",
        });
    }
    if definition.timing.submit_delay_bars != 0 {
        return Err(InterpreterError::Unsupported {
            feature: "timing.submit_delay_bars",
            detail: "intents are submitted at the decision that produced them",
        });
    }
    Ok(())
}

fn compile_filters(definition: &StrategyDefinition) -> Result<(), InterpreterError> {
    if definition.session.enabled {
        return Err(InterpreterError::Unsupported {
            feature: "session filter",
            detail: "the decision context carries no timestamp to place in a window",
        });
    }
    if definition.news.enabled {
        return Err(InterpreterError::Unsupported {
            feature: "news filter",
            detail: "no economic calendar is bound to the simulated clock",
        });
    }
    Ok(())
}

fn compile_trade_management(definition: &StrategyDefinition) -> Result<(), InterpreterError> {
    let management = &definition.trade_management;
    if management.legs.len() != 1 {
        return Err(InterpreterError::Unsupported {
            feature: "trade_management.legs",
            detail: "scale-outs need resting orders the simulator does not model",
        });
    }
    let leg = &management.legs[0];
    if leg.stop.is_some() || leg.target.is_some() || leg.trailing.is_some() {
        return Err(InterpreterError::Unsupported {
            feature: "trade_management.legs[0]",
            detail: "protective stops, targets and trails need intrabar execution",
        });
    }
    if management.break_even_after.is_some() {
        return Err(InterpreterError::Unsupported {
            feature: "trade_management.break_even_after",
            detail: "break-even moves need a resting stop to move",
        });
    }
    if management.max_bars_in_trade.is_some() {
        return Err(InterpreterError::Unsupported {
            feature: "trade_management.max_bars_in_trade",
            detail: "a time stop must anchor on the fill, which is not reported back",
        });
    }
    Ok(())
}

fn compile_sizing(definition: &StrategyDefinition) -> Result<f64, InterpreterError> {
    match &definition.sizing.rule {
        SizingRule::FixedUnits { units } => {
            if !units.is_finite() || *units <= 0.0 || *units > MAX_ORDER_QUANTITY {
                return Err(InterpreterError::InvalidQuantity { units: *units });
            }
            Ok(*units)
        }
        SizingRule::PercentEquity { .. } => Err(InterpreterError::Unsupported {
            feature: "sizing.percent_equity",
            detail: "account equity is not visible from a decision",
        }),
        SizingRule::RiskPercentAtr { .. } => Err(InterpreterError::Unsupported {
            feature: "sizing.risk_percent_atr",
            detail: "account equity is not visible from a decision",
        }),
    }
}

fn compile_node(
    node: &IndicatorNode,
    parameters: &BTreeMap<&str, &ParamValue>,
    slots: &BTreeMap<&str, usize>,
) -> Result<CompiledNode, InterpreterError> {
    let periods = |from: usize, count: usize| -> Result<Vec<usize>, InterpreterError> {
        node.inputs
            .iter()
            .skip(from)
            .take(count)
            .map(|input| period_of(node, input, parameters))
            .collect()
    };
    let series_source = || -> Result<SourceRef, InterpreterError> {
        match node.inputs.first() {
            Some(IndicatorInput::Price(field)) => Ok(SourceRef::Price(*field)),
            Some(IndicatorInput::Indicator(id)) => slots
                .get(id.as_str())
                .copied()
                .map(SourceRef::Node)
                .ok_or_else(|| InterpreterError::UnknownRef {
                    kind: "indicator",
                    id: id.clone(),
                    context: format!("indicator `{}`", node.id),
                }),
            _ => Err(InterpreterError::InputShapeMismatch {
                indicator: node.id.clone(),
                expected: "series",
            }),
        }
    };

    let (kind, source) = match &node.kind {
        IndicatorKind::Atr => (
            CompiledKind::Atr {
                period: periods(0, 1)?[0],
            },
            SourceRef::Bar,
        ),
        IndicatorKind::Adx => (
            CompiledKind::Adx {
                period: periods(0, 1)?[0],
            },
            SourceRef::Bar,
        ),
        IndicatorKind::Sma => (
            CompiledKind::Sma {
                period: periods(1, 1)?[0],
            },
            series_source()?,
        ),
        IndicatorKind::Ema => (
            CompiledKind::Ema {
                period: periods(1, 1)?[0],
            },
            series_source()?,
        ),
        IndicatorKind::Rsi => (
            CompiledKind::Rsi {
                period: periods(1, 1)?[0],
            },
            series_source()?,
        ),
        IndicatorKind::StdDev => (
            CompiledKind::StdDev {
                period: periods(1, 1)?[0],
            },
            series_source()?,
        ),
        IndicatorKind::FisherTransform => (
            CompiledKind::Fisher {
                period: periods(1, 1)?[0],
            },
            series_source()?,
        ),
        IndicatorKind::Kama => {
            let values = periods(1, 3)?;
            let [er_period, fast, slow] = values[..] else {
                return Err(InterpreterError::InputShapeMismatch {
                    indicator: node.id.clone(),
                    expected: "series and three periods",
                });
            };
            (
                CompiledKind::Kama {
                    er_period,
                    fast,
                    slow,
                },
                series_source()?,
            )
        }
        IndicatorKind::Macd => {
            let values = periods(1, 3)?;
            let [fast, slow, signal] = values[..] else {
                return Err(InterpreterError::InputShapeMismatch {
                    indicator: node.id.clone(),
                    expected: "series and three periods",
                });
            };
            (CompiledKind::Macd { fast, slow, signal }, series_source()?)
        }
        IndicatorKind::Custom { name, .. } => {
            return Err(InterpreterError::UnsupportedIndicator {
                indicator: node.id.clone(),
                kind: name.clone(),
            });
        }
    };

    Ok(CompiledNode {
        id: node.id.clone(),
        kind,
        source,
    })
}

/// Resolve a scalar input to a whole period, rejecting anything a window
/// cannot be sized from.
fn period_of(
    node: &IndicatorNode,
    input: &IndicatorInput,
    parameters: &BTreeMap<&str, &ParamValue>,
) -> Result<usize, InterpreterError> {
    let value = scalar_of(input, parameters, &format!("indicator `{}`", node.id))?;
    if !value.is_finite() || value.fract() != 0.0 || value < 1.0 {
        return Err(InterpreterError::InvalidPeriod {
            indicator: node.id.clone(),
            value,
        });
    }
    let period = value as usize;
    if period > MAX_INDICATOR_PERIOD {
        return Err(InterpreterError::InvalidPeriod {
            indicator: node.id.clone(),
            value,
        });
    }
    Ok(period)
}

fn scalar_of(
    input: &IndicatorInput,
    parameters: &BTreeMap<&str, &ParamValue>,
    context: &str,
) -> Result<f64, InterpreterError> {
    match input {
        IndicatorInput::Constant(value) => Ok(*value),
        IndicatorInput::Parameter(id) => numeric_parameter(id, parameters, context),
        IndicatorInput::Price(_) | IndicatorInput::Indicator(_) => {
            Err(InterpreterError::InputShapeMismatch {
                indicator: context.to_string(),
                expected: "scalar",
            })
        }
    }
}

/// Parameters are read as their declared type, never re-parsed from text.
fn numeric_parameter(
    id: &str,
    parameters: &BTreeMap<&str, &ParamValue>,
    context: &str,
) -> Result<f64, InterpreterError> {
    let value = parameters
        .get(id)
        .ok_or_else(|| InterpreterError::UnknownRef {
            kind: "parameter",
            id: id.to_string(),
            context: context.to_string(),
        })?;
    match value {
        ParamValue::Int(value) => Ok(*value as f64),
        ParamValue::Float(value) => Ok(*value),
        ParamValue::Bool(_) | ParamValue::Text(_) => Err(InterpreterError::NonNumericParameter {
            id: id.to_string(),
            context: context.to_string(),
        }),
    }
}

/// Linearise the indicator graph, re-deriving the order rather than trusting
/// the declaration order or the IR's own acyclicity check.
fn dependency_order(nodes: &[CompiledNode]) -> Result<Vec<usize>, InterpreterError> {
    #[derive(Clone, Copy, PartialEq)]
    enum Mark {
        Unvisited,
        InProgress,
        Done,
    }

    let mut marks = vec![Mark::Unvisited; nodes.len()];
    let mut order = Vec::with_capacity(nodes.len());
    for start in 0..nodes.len() {
        if marks[start] != Mark::Unvisited {
            continue;
        }
        let mut stack = vec![start];
        marks[start] = Mark::InProgress;
        while let Some(&node) = stack.last() {
            let parent = match nodes[node].source {
                SourceRef::Node(parent) => Some(parent),
                SourceRef::Bar | SourceRef::Price(_) => None,
            };
            match parent {
                Some(parent) => {
                    let mark = marks.get(parent).copied().ok_or(
                        InterpreterError::UnknownIndicatorSlot {
                            slot: parent,
                            count: nodes.len(),
                        },
                    )?;
                    match mark {
                        Mark::Done => {
                            marks[node] = Mark::Done;
                            order.push(node);
                            stack.pop();
                        }
                        Mark::Unvisited => {
                            marks[parent] = Mark::InProgress;
                            stack.push(parent);
                        }
                        Mark::InProgress => {
                            return Err(InterpreterError::IndicatorCycle {
                                indicator: nodes[parent].id.clone(),
                            });
                        }
                    }
                }
                None => {
                    marks[node] = Mark::Done;
                    order.push(node);
                    stack.pop();
                }
            }
        }
    }
    Ok(order)
}

fn compile_rules(
    rules: &DirectionRules,
    parameters: &BTreeMap<&str, &ParamValue>,
    slots: &BTreeMap<&str, usize>,
    deepest: &mut usize,
) -> Result<CompiledRules, InterpreterError> {
    Ok(CompiledRules {
        enabled: rules.enabled,
        entry: compile_condition(&rules.entry, parameters, slots, deepest, 1)?,
        exit: compile_condition(&rules.exit, parameters, slots, deepest, 1)?,
    })
}

fn compile_condition(
    condition: &Condition,
    parameters: &BTreeMap<&str, &ParamValue>,
    slots: &BTreeMap<&str, usize>,
    deepest: &mut usize,
    depth: usize,
) -> Result<CompiledCondition, InterpreterError> {
    if depth > MAX_CONDITION_DEPTH {
        return Err(InterpreterError::ConditionTooDeep {
            limit: MAX_CONDITION_DEPTH,
        });
    }
    Ok(match condition {
        Condition::Always => CompiledCondition::Always,
        Condition::Never => CompiledCondition::Never,
        Condition::Not(inner) => CompiledCondition::Not(Box::new(compile_condition(
            inner,
            parameters,
            slots,
            deepest,
            depth + 1,
        )?)),
        Condition::All(children) => CompiledCondition::All(compile_children(
            children,
            parameters,
            slots,
            deepest,
            depth + 1,
        )?),
        Condition::Any(children) => CompiledCondition::Any(compile_children(
            children,
            parameters,
            slots,
            deepest,
            depth + 1,
        )?),
        Condition::Compare { left, op, right } => CompiledCondition::Compare {
            left: compile_operand(left, parameters, slots, deepest)?,
            op: *op,
            right: compile_operand(right, parameters, slots, deepest)?,
        },
        Condition::CrossesAbove { left, right } => CompiledCondition::Cross {
            left: compile_operand(left, parameters, slots, deepest)?,
            right: compile_operand(right, parameters, slots, deepest)?,
            above: true,
        },
        Condition::CrossesBelow { left, right } => CompiledCondition::Cross {
            left: compile_operand(left, parameters, slots, deepest)?,
            right: compile_operand(right, parameters, slots, deepest)?,
            above: false,
        },
    })
}

fn compile_children(
    children: &[Condition],
    parameters: &BTreeMap<&str, &ParamValue>,
    slots: &BTreeMap<&str, usize>,
    deepest: &mut usize,
    depth: usize,
) -> Result<Vec<CompiledCondition>, InterpreterError> {
    children
        .iter()
        .map(|child| compile_condition(child, parameters, slots, deepest, depth))
        .collect()
}

fn compile_operand(
    operand: &Operand,
    parameters: &BTreeMap<&str, &ParamValue>,
    slots: &BTreeMap<&str, usize>,
    deepest: &mut usize,
) -> Result<CompiledOperand, InterpreterError> {
    match operand {
        Operand::Constant(value) => Ok(CompiledOperand::Constant(*value)),
        Operand::Parameter(id) => {
            numeric_parameter(id, parameters, "condition").map(CompiledOperand::Constant)
        }
        Operand::Price { field, bars_ago } => Ok(CompiledOperand::Price {
            field: *field,
            bars_ago: checked_lookback(*bars_ago)?,
        }),
        Operand::Indicator { id, bars_ago } => {
            let slot =
                slots
                    .get(id.as_str())
                    .copied()
                    .ok_or_else(|| InterpreterError::UnknownRef {
                        kind: "indicator",
                        id: id.clone(),
                        context: "condition".to_string(),
                    })?;
            let bars_ago = checked_lookback(*bars_ago)?;
            *deepest = (*deepest).max(bars_ago);
            Ok(CompiledOperand::Indicator { slot, bars_ago })
        }
    }
}

fn checked_lookback(bars_ago: u32) -> Result<usize, InterpreterError> {
    if bars_ago > MAX_BARS_AGO {
        return Err(InterpreterError::LookbackOutOfRange {
            bars_ago: bars_ago as usize,
            capacity: MAX_BARS_AGO as usize,
        });
    }
    Ok(bars_ago as usize)
}

// ── Evaluation ─────────────────────────────────────────────────────

/// Read one node's retained value, defending both the slot and the depth
/// against a program that says something the state was not built for.
fn indicator_value(
    state: &SymbolState,
    slot: usize,
    bars_ago: usize,
) -> Result<Option<f64>, InterpreterError> {
    state
        .nodes
        .get(slot)
        .ok_or(InterpreterError::UnknownIndicatorSlot {
            slot,
            count: state.nodes.len(),
        })?
        .history
        .get(bars_ago)
}

fn price_value(
    market: &MarketView<'_>,
    symbol: SymbolId,
    field: PriceField,
    bars_ago: usize,
) -> Result<Option<f64>, InterpreterError> {
    let read = match field {
        PriceField::Open => market.open(symbol, bars_ago),
        PriceField::High => market.high(symbol, bars_ago),
        PriceField::Low => market.low(symbol, bars_ago),
        PriceField::Close => market.close(symbol, bars_ago),
        PriceField::Volume => market.volume(symbol, bars_ago),
    };
    match read {
        Ok(value) => Ok(Some(value)),
        // Not enough history yet is warm-up, not a failure.
        Err(MarketDataError::FutureData { .. }) => Ok(None),
        Err(error) => Err(InterpreterError::MarketData(error)),
    }
}

/// `shift` steps every series operand one bar further back, which is how the
/// cross conditions read the previous observation.
fn operand_value(
    operand: CompiledOperand,
    shift: usize,
    state: &SymbolState,
    symbol: SymbolId,
    market: &MarketView<'_>,
) -> Result<Option<f64>, InterpreterError> {
    match operand {
        CompiledOperand::Constant(value) => Ok(Some(value)),
        CompiledOperand::Price { field, bars_ago } => {
            price_value(market, symbol, field, bars_ago + shift)
        }
        CompiledOperand::Indicator { slot, bars_ago } => {
            indicator_value(state, slot, bars_ago + shift)
        }
    }
}

/// Exact comparison: the IR writes down the operator it means, and rounding a
/// strategy's equality test would be the interpreter inventing a tolerance.
#[allow(clippy::float_cmp)]
fn compare(op: CompareOp, left: f64, right: f64) -> bool {
    match op {
        CompareOp::Greater => left > right,
        CompareOp::GreaterOrEqual => left >= right,
        CompareOp::Less => left < right,
        CompareOp::LessOrEqual => left <= right,
        CompareOp::Equal => left == right,
        CompareOp::NotEqual => left != right,
    }
}

fn evaluate(
    condition: &CompiledCondition,
    state: &SymbolState,
    symbol: SymbolId,
    market: &MarketView<'_>,
    depth: usize,
) -> Result<Truth, InterpreterError> {
    if depth > MAX_CONDITION_DEPTH {
        return Err(InterpreterError::ConditionTooDeep {
            limit: MAX_CONDITION_DEPTH,
        });
    }
    match condition {
        CompiledCondition::Always => Ok(Truth::True),
        CompiledCondition::Never => Ok(Truth::False),
        CompiledCondition::Not(inner) => {
            Ok(evaluate(inner, state, symbol, market, depth + 1)?.negate())
        }
        // Every child is evaluated: a malformed reference must surface
        // whatever the data happens to say about its siblings.
        CompiledCondition::All(children) => {
            let mut result = Truth::True;
            for child in children {
                match evaluate(child, state, symbol, market, depth + 1)? {
                    Truth::False => result = Truth::False,
                    Truth::Unknown if result != Truth::False => result = Truth::Unknown,
                    _ => {}
                }
            }
            Ok(result)
        }
        CompiledCondition::Any(children) => {
            let mut result = Truth::False;
            for child in children {
                match evaluate(child, state, symbol, market, depth + 1)? {
                    Truth::True => result = Truth::True,
                    Truth::Unknown if result != Truth::True => result = Truth::Unknown,
                    _ => {}
                }
            }
            Ok(result)
        }
        CompiledCondition::Compare { left, op, right } => {
            let left = operand_value(*left, 0, state, symbol, market)?;
            let right = operand_value(*right, 0, state, symbol, market)?;
            let (Some(left), Some(right)) = (left, right) else {
                return Ok(Truth::Unknown);
            };
            Ok(Truth::of(compare(*op, left, right)))
        }
        CompiledCondition::Cross { left, right, above } => {
            let left_now = operand_value(*left, 0, state, symbol, market)?;
            let right_now = operand_value(*right, 0, state, symbol, market)?;
            let left_before = operand_value(*left, 1, state, symbol, market)?;
            let right_before = operand_value(*right, 1, state, symbol, market)?;
            let (Some(left_now), Some(right_now), Some(left_before), Some(right_before)) =
                (left_now, right_now, left_before, right_before)
            else {
                return Ok(Truth::Unknown);
            };
            Ok(Truth::of(if *above {
                left_now > right_now && left_before <= right_before
            } else {
                left_now < right_now && left_before >= right_before
            }))
        }
    }
}

/// Advance every node by exactly one bar, parents before children, so a
/// dependent reads the value its source produced for this same bar.
fn advance(
    program: &Program,
    state: &mut SymbolState,
    symbol: SymbolId,
    market: &MarketView<'_>,
) -> Result<(), InterpreterError> {
    let bar = |field: PriceField| price_value(market, symbol, field, 0);
    let open = bar(PriceField::Open)?;
    let high = bar(PriceField::High)?;
    let low = bar(PriceField::Low)?;
    let close = bar(PriceField::Close)?;
    let volume = bar(PriceField::Volume)?;

    for &slot in &program.order {
        let node = program
            .nodes
            .get(slot)
            .ok_or(InterpreterError::UnknownIndicatorSlot {
                slot,
                count: program.nodes.len(),
            })?;
        let input = match node.source {
            SourceRef::Bar => match (high, low, close) {
                (Some(high), Some(low), Some(close)) => Some(CalcInput::Bar { high, low, close }),
                _ => None,
            },
            SourceRef::Price(field) => match field {
                PriceField::Open => open,
                PriceField::High => high,
                PriceField::Low => low,
                PriceField::Close => close,
                PriceField::Volume => volume,
            }
            .map(CalcInput::Series),
            // The parent was advanced earlier in this same pass, so bars_ago 0
            // is its value for this bar. A parent still warming up hands its
            // dependent nothing, and the dependent does not consume a sample.
            SourceRef::Node(parent) => indicator_value(state, parent, 0)?.map(CalcInput::Series),
        };
        let node_state =
            state
                .nodes
                .get_mut(slot)
                .ok_or(InterpreterError::UnknownIndicatorSlot {
                    slot,
                    count: program.nodes.len(),
                })?;
        let value = match input {
            Some(input) => node_state.calc.update(input, &node.id)?,
            None => None,
        };
        node_state.history.push(value);
    }
    Ok(())
}

/// How many bars the simulator has committed for this symbol. The market view
/// reports the count on the error path of an impossible lookback, which is the
/// only way to ask it.
fn committed_bars(market: &MarketView<'_>, symbol: SymbolId) -> Result<usize, InterpreterError> {
    match market.close(symbol, usize::MAX) {
        Err(MarketDataError::FutureData { available, .. }) => Ok(available),
        Err(error) => Err(InterpreterError::MarketData(error)),
        Ok(_) => Err(InterpreterError::UnexpectedHistory { symbol: symbol.0 }),
    }
}

// ── Strategy ───────────────────────────────────────────────────────

/// A validated [`StrategyIr`] lowered onto the reference simulator.
#[derive(Debug, Clone)]
pub struct CanonicalIrStrategy {
    program: Program,
    /// Indexed by [`SymbolId`], allocated on first sight of a symbol.
    symbols: Vec<Option<SymbolState>>,
}

impl CanonicalIrStrategy {
    /// Compile a sealed IR, refusing anything the simulator cannot express.
    pub fn new(ir: &StrategyIr) -> Result<Self, InterpreterError> {
        // The artifact is sealed, but a caller can hand over a struct built by
        // any route; re-verifying costs one pass and removes the assumption.
        ir.verify().map_err(InterpreterError::InvalidIr)?;
        Ok(Self {
            program: compile(ir.definition())?,
            symbols: Vec::new(),
        })
    }

    /// Drop every symbol's indicator state and tracked position. Required
    /// between runs: the state describes one contiguous stream.
    pub fn reset(&mut self) {
        self.symbols.clear();
    }

    /// Symbols the interpreter currently believes it holds.
    pub fn open_positions(&self) -> usize {
        self.symbols
            .iter()
            .flatten()
            .filter(|state| !matches!(state.position, Position::Flat))
            .count()
    }

    fn decide(
        &mut self,
        ctx: &DecisionContext<'_>,
        orders: &mut OrderIntents,
    ) -> Result<(), InterpreterError> {
        let symbol = ctx.symbol();
        let market = ctx.market();
        if symbol.0 >= MAX_SYMBOLS {
            return Err(InterpreterError::SymbolOutOfRange {
                id: symbol.0,
                limit: MAX_SYMBOLS,
            });
        }

        if self.symbols.len() <= symbol.0 {
            self.symbols.resize(symbol.0 + 1, None);
        }
        if self.symbols[symbol.0].is_none() {
            let fresh = SymbolState::new(&self.program);
            self.symbols[symbol.0] = Some(fresh);
        }

        let observed = committed_bars(market, symbol)?;
        let program = &self.program;
        let state = self.symbols[symbol.0]
            .as_mut()
            .ok_or(InterpreterError::UnexpectedHistory { symbol: symbol.0 })?;
        // One decision per committed bar. Anything else means this state
        // belongs to a different stream, so refuse rather than blend them.
        if observed != state.bars + 1 {
            return Err(InterpreterError::HistoryDesynchronized {
                symbol: symbol.0,
                expected: state.bars + 1,
                observed,
            });
        }
        state.bars = observed;
        advance(program, state, symbol, market)?;

        let held = self.open_positions();
        let state = self.symbols[symbol.0]
            .as_ref()
            .ok_or(InterpreterError::UnexpectedHistory { symbol: symbol.0 })?;
        // Positions held elsewhere; this symbol's own slot is free to reuse.
        let elsewhere = held - usize::from(!matches!(state.position, Position::Flat));

        let mut position = state.position;
        let mut actions: Vec<(OrderSide, f64)> = Vec::new();

        match position {
            Position::Long { units } => {
                if evaluate(&program.long.exit, state, symbol, market, 1)? == Truth::True {
                    actions.push((OrderSide::Sell, units));
                    position = Position::Flat;
                }
            }
            Position::Short { units } => {
                if evaluate(&program.short.exit, state, symbol, market, 1)? == Truth::True {
                    actions.push((OrderSide::Buy, units));
                    position = Position::Flat;
                }
            }
            Position::Flat => {}
        }

        if matches!(position, Position::Flat) {
            let long_entry = if program.long.enabled {
                evaluate(&program.long.entry, state, symbol, market, 1)?
            } else {
                Truth::False
            };
            let short_entry = if program.short.enabled {
                evaluate(&program.short.entry, state, symbol, market, 1)?
            } else {
                Truth::False
            };
            // Both directions firing at once is a contradiction the IR does
            // not resolve, so the interpreter stands aside instead of ranking.
            let side = match (long_entry, short_entry) {
                (Truth::True, Truth::True) => None,
                (Truth::True, _) => Some(OrderSide::Buy),
                (_, Truth::True) => Some(OrderSide::Sell),
                _ => None,
            };
            if let Some(side) = side
                && elsewhere < program.max_open_positions
            {
                let units = program.units;
                actions.push((side, units));
                position = match side {
                    OrderSide::Buy => Position::Long { units },
                    OrderSide::Sell => Position::Short { units },
                };
            }
        }

        for (side, quantity) in actions {
            orders
                .market(symbol, side, quantity)
                .map_err(InterpreterError::Order)?;
        }
        if let Some(state) = self.symbols[symbol.0].as_mut() {
            state.position = position;
        }
        Ok(())
    }
}

impl ReferenceStrategy for CanonicalIrStrategy {
    fn on_bar_close(
        &mut self,
        ctx: &DecisionContext<'_>,
        orders: &mut OrderIntents,
    ) -> Result<(), StrategyError> {
        self.decide(ctx, orders).map_err(|error| match error {
            // The simulator's own rejection travels back unchanged.
            InterpreterError::Order(inner) => inner,
            other => StrategyError::Rejected {
                reason: other.to_string(),
            },
        })
    }
}

#[cfg(test)]
mod tests;
