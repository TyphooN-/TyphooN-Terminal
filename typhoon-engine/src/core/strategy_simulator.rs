//! Deterministic scalar strategy simulator.

use crate::core::strategy_ir::{
    CommissionModel, SlippageModel, SpreadModel, StrategyExecutionConfig, StrategyIrError,
    TieBreakPolicy,
};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SymbolId(pub usize);

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
    fn sign(self) -> f64 {
        match self {
            Self::Buy => 1.0,
            Self::Sell => -1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SimEventKind {
    BarOpen,
    OrderActivate,
    Fill,
    BarClose,
    Decision,
    OrderSubmit,
    MarkToMarket,
}

impl SimEventKind {
    pub const fn priority(self) -> u8 {
        match self {
            Self::BarOpen => 0,
            Self::OrderActivate => 1,
            Self::Fill => 2,
            Self::BarClose => 3,
            Self::Decision => 4,
            Self::OrderSubmit => 5,
            Self::MarkToMarket => 6,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimEventRecord {
    pub time_ns: i64,
    pub kind: SimEventKind,
    pub sequence: u64,
    pub symbol: Option<SymbolId>,
    pub order_id: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FillRecord {
    pub order_id: u64,
    pub time_ns: i64,
    pub sequence: u64,
    pub symbol: SymbolId,
    pub side: OrderSide,
    pub quantity: f64,
    pub reference_price: f64,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingOrderRecord {
    pub order_id: u64,
    pub submitted_time_ns: i64,
    pub symbol: SymbolId,
    pub side: OrderSide,
    pub quantity: f64,
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
    pub pending_orders: Vec<PendingOrderRecord>,
    pub positions: Vec<PositionRecord>,
    pub equity_curve: Vec<EquityPoint>,
    pub final_cash: f64,
    pub final_equity: f64,
    pub final_realized_pnl: f64,
    pub total_commission: f64,
}

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
    UnknownSymbol { id: usize },
}

impl fmt::Display for StrategyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rejected { reason } => write!(f, "strategy rejected decision: {reason}"),
            Self::TooManyIntents { limit } => write!(f, "strategy exceeded {limit} intents"),
            Self::InvalidQuantity { quantity } => write!(f, "invalid order quantity {quantity}"),
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

#[derive(Debug, Clone, Copy)]
struct OrderIntent {
    symbol: SymbolId,
    side: OrderSide,
    quantity: f64,
}

pub struct OrderIntents {
    symbol_count: usize,
    intents: Vec<OrderIntent>,
}

impl OrderIntents {
    fn new(symbol_count: usize) -> Self {
        Self {
            symbol_count,
            intents: Vec::new(),
        }
    }

    pub fn market(
        &mut self,
        symbol: SymbolId,
        side: OrderSide,
        quantity: f64,
    ) -> Result<(), StrategyError> {
        if symbol.0 >= self.symbol_count {
            return Err(StrategyError::UnknownSymbol { id: symbol.0 });
        }
        if !quantity.is_finite() || quantity <= 0.0 || quantity > MAX_ORDER_QUANTITY {
            return Err(StrategyError::InvalidQuantity { quantity });
        }
        if self.intents.len() >= MAX_INTENTS_PER_DECISION {
            return Err(StrategyError::TooManyIntents {
                limit: MAX_INTENTS_PER_DECISION,
            });
        }
        self.intents.push(OrderIntent {
            symbol,
            side,
            quantity,
        });
        Ok(())
    }
}

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
        let available = self
            .committed
            .get(symbol.0)
            .copied()
            .map_or(0, |value| value);
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
    market: MarketView<'a>,
}

impl DecisionContext<'_> {
    pub const fn symbol(&self) -> SymbolId {
        self.symbol
    }
    pub const fn market(&self) -> &MarketView<'_> {
        &self.market
    }
}

pub trait ReferenceStrategy {
    fn on_bar_close(
        &mut self,
        ctx: &DecisionContext<'_>,
        orders: &mut OrderIntents,
    ) -> Result<(), StrategyError>;
}

#[derive(Debug, Clone)]
struct PendingOrder {
    order_id: u64,
    submit_sequence: u64,
    submitted_time_ns: i64,
    symbol: SymbolId,
    side: OrderSide,
    quantity: f64,
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
        symbol: Option<SymbolId>,
        order_id: Option<u64>,
    ) -> u64 {
        let sequence = self.sequence;
        self.sequence = self.sequence.saturating_add(1);
        self.events.push(SimEventRecord {
            time_ns,
            kind,
            sequence,
            symbol,
            order_id,
        });
        sequence
    }
    fn reserve_sequence(&mut self) -> u64 {
        let sequence = self.sequence;
        self.sequence = self.sequence.saturating_add(1);
        sequence
    }
}

#[derive(Clone, Copy)]
struct ClockEvent {
    time_ns: i64,
    symbol: SymbolId,
    bar_index: usize,
    is_open: bool,
}

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
        config.settings.slippage,
        SlippageModel::VolatilityScaled { .. }
    ) {
        return Err(SimulationError::UnsupportedModel {
            field: "settings.slippage",
            model: "volatility_scaled",
        });
    }
    if matches!(config.settings.spread, SpreadModel::RecordedQuotes) {
        return Err(SimulationError::UnsupportedModel {
            field: "settings.spread",
            model: "recorded_quotes",
        });
    }
    if matches!(
        config.settings.tie_break,
        TieBreakPolicy::TimestampPrioritySymbolSequence
    ) {
        return Err(SimulationError::UnsupportedModel {
            field: "settings.tie_break",
            model: "timestamp_priority_symbol_sequence",
        });
    }
    Ok(())
}

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

fn commission(model: &CommissionModel, quantity: f64, fill_price: f64) -> f64 {
    match model {
        CommissionModel::None => 0.0,
        CommissionModel::PerShare { amount, minimum } => (amount * quantity).max(*minimum),
        CommissionModel::PercentOfNotional { percent, minimum } => {
            (fill_price * quantity * percent / 100.0).max(*minimum)
        }
        CommissionModel::PerOrder { amount } => *amount,
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

pub fn run_simulation(
    config: &StrategyExecutionConfig,
    streams: &[SymbolStream],
    strategy: &mut dyn ReferenceStrategy,
) -> Result<SimulationReport, SimulationError> {
    config.verify().map_err(SimulationError::Config)?;
    validate_models(config)?;
    let streams = validate_inputs(streams)?;
    let symbol_count = streams.len();

    let mut clock = Vec::new();
    for (symbol_index, stream) in streams.iter().enumerate() {
        for (bar_index, bar) in stream.bars.iter().enumerate() {
            clock.push(ClockEvent {
                time_ns: bar.open_time_ns,
                symbol: SymbolId(symbol_index),
                bar_index,
                is_open: true,
            });
            clock.push(ClockEvent {
                time_ns: bar.close_time_ns,
                symbol: SymbolId(symbol_index),
                bar_index,
                is_open: false,
            });
        }
    }
    clock.sort_by_key(|event| (event.time_ns, !event.is_open, event.symbol, event.bar_index));

    let mut recorder = Recorder {
        sequence: 0,
        events: Vec::new(),
    };
    let mut committed = vec![0usize; symbol_count];
    let mut opened = vec![None; symbol_count];
    let mut marks = vec![None; symbol_count];
    let mut positions = vec![PositionState::default(); symbol_count];
    let mut pending: Vec<PendingOrder> = Vec::new();
    let mut fills = Vec::new();
    let mut equity_curve = Vec::new();
    let mut cash = config.settings.initial_capital;
    let mut total_commission = 0.0;
    let mut next_order_id = 0u64;

    let mut cursor = 0usize;
    while cursor < clock.len() {
        let time_ns = clock[cursor].time_ns;
        let group_start = cursor;
        while cursor < clock.len() && clock[cursor].time_ns == time_ns {
            cursor += 1;
        }
        let group = &clock[group_start..cursor];

        let mut opens: Vec<ClockEvent> = group
            .iter()
            .copied()
            .filter(|event| event.is_open)
            .collect();
        opens.sort_by_key(|event| event.symbol);
        for event in &opens {
            opened[event.symbol.0] = Some(event.bar_index);
            recorder.event(time_ns, SimEventKind::BarOpen, Some(event.symbol), None);
        }

        let mut fill_orders: Vec<(usize, f64)> = Vec::new();
        for event in &opens {
            if let Some(bar) = streams[event.symbol.0].bars.get(event.bar_index) {
                for (pending_index, order) in pending.iter().enumerate() {
                    if order.symbol == event.symbol && order.submitted_time_ns < time_ns {
                        fill_orders.push((pending_index, bar.open));
                    }
                }
            }
        }
        fill_orders.sort_by_key(|(index, _)| pending[*index].submit_sequence);
        for (index, _) in &fill_orders {
            let order = &pending[*index];
            recorder.event(
                time_ns,
                SimEventKind::OrderActivate,
                Some(order.symbol),
                Some(order.order_id),
            );
        }
        for (index, reference_price) in &fill_orders {
            let order = &pending[*index];
            let width = finite_accounting(
                "spread_width",
                stable_decimal(spread_width(&config.settings.spread, *reference_price)),
            )?;
            let quoted_price = finite_accounting(
                "quoted_price",
                stable_decimal(*reference_price + order.side.sign() * width / 2.0),
            )?;
            let slip = finite_accounting(
                "slippage",
                stable_decimal(slippage_distance(&config.settings.slippage, width)),
            )?;
            let fill_price = finite_accounting(
                "fill_price",
                stable_decimal(quoted_price + order.side.sign() * slip),
            )?;
            let fee = finite_accounting(
                "commission",
                stable_decimal(commission(
                    &config.settings.commission,
                    order.quantity,
                    fill_price,
                )),
            )?;
            let spread_cost =
                finite_accounting("spread_cost", stable_decimal(width / 2.0 * order.quantity))?;
            let slippage_cost =
                finite_accounting("slippage_cost", stable_decimal(slip * order.quantity))?;
            cash = finite_accounting(
                "cash",
                stable_decimal(cash - order.side.sign() * fill_price * order.quantity - fee),
            )?;
            total_commission =
                finite_accounting("total_commission", stable_decimal(total_commission + fee))?;
            let realized_pnl = apply_fill(
                &mut positions[order.symbol.0],
                order.side,
                order.quantity,
                fill_price,
            );
            finite_accounting("realized_pnl", realized_pnl)?;
            finite_accounting("position_units", positions[order.symbol.0].units)?;
            finite_accounting("average_entry", positions[order.symbol.0].avg_entry)?;
            finite_accounting(
                "position_realized_pnl",
                positions[order.symbol.0].realized_pnl,
            )?;
            let sequence = recorder.event(
                time_ns,
                SimEventKind::Fill,
                Some(order.symbol),
                Some(order.order_id),
            );
            let position = &positions[order.symbol.0];
            fills.push(FillRecord {
                order_id: order.order_id,
                time_ns,
                sequence,
                symbol: order.symbol,
                side: order.side,
                quantity: order.quantity,
                reference_price: *reference_price,
                quoted_price,
                fill_price,
                spread_cost,
                slippage_cost,
                commission: fee,
                realized_pnl,
                cash_after: cash,
                position_units_after: position.units,
                avg_entry_after: position.avg_entry,
            });
        }
        if !fill_orders.is_empty() {
            let mut filled = vec![false; pending.len()];
            for (index, _) in fill_orders {
                filled[index] = true;
            }
            pending = pending
                .into_iter()
                .enumerate()
                .filter_map(|(index, order)| (!filled[index]).then_some(order))
                .collect();
        }

        let mut closes: Vec<ClockEvent> = group
            .iter()
            .copied()
            .filter(|event| !event.is_open)
            .collect();
        closes.sort_by_key(|event| event.symbol);
        for event in &closes {
            committed[event.symbol.0] = event.bar_index + 1;
            if let Some(bar) = streams[event.symbol.0].bars.get(event.bar_index) {
                marks[event.symbol.0] = Some(bar.close);
            }
            recorder.event(time_ns, SimEventKind::BarClose, Some(event.symbol), None);
        }

        let mut decision_intents: Vec<Vec<OrderIntent>> = Vec::with_capacity(closes.len());
        for event in &closes {
            recorder.event(time_ns, SimEventKind::Decision, Some(event.symbol), None);
            let market = MarketView {
                streams: &streams,
                committed: &committed,
                opened: &opened,
            };
            let ctx = DecisionContext {
                symbol: event.symbol,
                market,
            };
            let mut orders = OrderIntents::new(symbol_count);
            strategy
                .on_bar_close(&ctx, &mut orders)
                .map_err(|error| SimulationError::Strategy { time_ns, error })?;
            decision_intents.push(orders.intents);
        }
        for intents in decision_intents {
            for intent in intents {
                let order_id = next_order_id;
                next_order_id = next_order_id.saturating_add(1);
                let submit_sequence = recorder.event(
                    time_ns,
                    SimEventKind::OrderSubmit,
                    Some(intent.symbol),
                    Some(order_id),
                );
                pending.push(PendingOrder {
                    order_id,
                    submit_sequence,
                    submitted_time_ns: time_ns,
                    symbol: intent.symbol,
                    side: intent.side,
                    quantity: intent.quantity,
                });
            }
        }
        for _ in &closes {
            let sequence = recorder.reserve_sequence();
            let equity = finite_accounting("equity", marked_equity(cash, &positions, &marks))?;
            equity_curve.push(EquityPoint {
                time_ns,
                sequence,
                cash,
                equity,
            });
        }
    }

    let final_time_ns = streams
        .iter()
        .filter_map(|stream| stream.bars.last().map(|bar| bar.close_time_ns))
        .max()
        .map_or(0, |time_ns| time_ns);
    let final_mark_sequence = recorder.event(final_time_ns, SimEventKind::MarkToMarket, None, None);
    let final_equity = finite_accounting("final_equity", marked_equity(cash, &positions, &marks))?;
    equity_curve.push(EquityPoint {
        time_ns: final_time_ns,
        sequence: final_mark_sequence,
        cash,
        equity: final_equity,
    });

    let position_records = positions
        .iter()
        .enumerate()
        .map(|(index, position)| {
            let mark_price = marks[index];
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
    let pending_orders = pending
        .into_iter()
        .map(|order| PendingOrderRecord {
            order_id: order.order_id,
            submitted_time_ns: order.submitted_time_ns,
            symbol: order.symbol,
            side: order.side,
            quantity: order.quantity,
        })
        .collect();
    let final_realized_pnl = finite_accounting(
        "final_realized_pnl",
        positions.iter().map(|position| position.realized_pnl).sum(),
    )?;

    Ok(SimulationReport {
        symbols: streams.into_iter().map(|stream| stream.symbol).collect(),
        events: recorder.events,
        fills,
        pending_orders,
        positions: position_records,
        equity_curve,
        final_cash: cash,
        final_equity,
        final_realized_pnl,
        total_commission,
    })
}

#[cfg(test)]
mod tests;
