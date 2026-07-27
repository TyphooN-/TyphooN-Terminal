use super::*;
use crate::core::strategy_ir::{
    CommissionModel, ExecutionSettings, FidelityLevel, LatencyModel, MarginPolicy,
    OhlcAmbiguityPolicy, SlippageModel, SpreadModel, StrategyExecutionConfig, TieBreakPolicy,
};
use sha2::{Digest, Sha256};

const MINUTE_NS: i64 = 60_000_000_000;
const SECOND_NS: i64 = 1_000_000_000;
const HALF_SECOND_NS: i64 = SECOND_NS / 2;
const START_CAPITAL: f64 = 100_000.0;
const EPS: f64 = 1e-9;

// ── Fixtures ───────────────────────────────────────────────────────

fn assert_close(actual: f64, expected: f64, what: &str) {
    assert!(
        (actual - expected).abs() < EPS,
        "{what}: expected {expected}, got {actual}"
    );
}

/// Bar `index` on a one-minute grid starting at the epoch. The close stamp is
/// the inclusive last nanosecond of the minute, so consecutive bars never
/// share a timestamp.
fn minute_bar(index: i64, open: f64, high: f64, low: f64, close: f64) -> SimBar {
    SimBar {
        open_time_ns: index * MINUTE_NS,
        close_time_ns: index * MINUTE_NS + MINUTE_NS - 1,
        open,
        high,
        low,
        close,
        volume: 1_000.0,
    }
}

/// `count` bars whose open ramps 100, 101, 102 … so every price in an
/// assertion is readable from the bar index alone.
fn ramp(symbol: &str, count: usize) -> SymbolStream {
    let bars = (0..count)
        .map(|i| {
            let open = 100.0 + i as f64;
            minute_bar(i as i64, open, open + 1.0, open - 1.0, open + 0.5)
        })
        .collect();
    SymbolStream {
        symbol: symbol.to_string(),
        bars,
    }
}

/// The same ramp shifted 30 s later, so its bars stay open across the other
/// stream's closes. Used to exercise the multi-symbol clock.
fn offset_ramp(symbol: &str, count: usize) -> SymbolStream {
    let half = MINUTE_NS / 2;
    let bars = (0..count)
        .map(|i| {
            let open = 200.0 + i as f64;
            let open_time_ns = i as i64 * MINUTE_NS + half;
            SimBar {
                open_time_ns,
                close_time_ns: open_time_ns + MINUTE_NS - 1,
                open,
                high: open + 1.0,
                low: open - 1.0,
                close: open + 0.5,
                volume: 500.0,
            }
        })
        .collect();
    SymbolStream {
        symbol: symbol.to_string(),
        bars,
    }
}

/// A stream built from explicit `(open, high, low, close)` rows on the minute
/// grid, so a golden scenario reads as the price path it is testing.
fn stream_from(symbol: &str, rows: &[(f64, f64, f64, f64)]) -> SymbolStream {
    SymbolStream {
        symbol: symbol.to_string(),
        bars: rows
            .iter()
            .enumerate()
            .map(|(index, (open, high, low, close))| {
                minute_bar(index as i64, *open, *high, *low, *close)
            })
            .collect(),
    }
}

/// Zero-cost settings: the frictionless baseline every accounting assertion
/// starts from.
fn free_settings() -> ExecutionSettings {
    ExecutionSettings {
        initial_capital: START_CAPITAL,
        account_currency: "USD".to_string(),
        commission: CommissionModel::None,
        slippage: SlippageModel::None,
        spread: SpreadModel::None,
        ambiguity: OhlcAmbiguityPolicy::StopFirst,
        tie_break: TieBreakPolicy::TimestampPrioritySequence,
        ..ExecutionSettings::conservative_defaults()
    }
}

/// Every supported cost model switched on at once.
fn costed_settings() -> ExecutionSettings {
    ExecutionSettings {
        commission: CommissionModel::PerShare {
            amount: 0.01,
            minimum: 1.0,
        },
        slippage: SlippageModel::FixedPriceDistance { distance: 0.02 },
        spread: SpreadModel::Constant { price_units: 0.10 },
        ..free_settings()
    }
}

/// Zero costs plus intrabar resolution — the baseline for every stop/target
/// scenario, where the only thing under test is which level was reached.
fn intrabar_settings() -> ExecutionSettings {
    ExecutionSettings {
        fidelity: FidelityLevel::BarOhlc,
        ..free_settings()
    }
}

fn config(settings: ExecutionSettings) -> StrategyExecutionConfig {
    StrategyExecutionConfig::build(&settings).expect("settings are valid")
}

fn run(
    settings: ExecutionSettings,
    streams: &[SymbolStream],
    strategy: &mut dyn ReferenceStrategy,
) -> Result<SimulationReport, SimulationError> {
    run_simulation(
        &config(settings),
        &SimulationSetup::default(),
        streams,
        strategy,
    )
}

fn run_with(
    settings: ExecutionSettings,
    setup: SimulationSetup,
    streams: &[SymbolStream],
    strategy: &mut dyn ReferenceStrategy,
) -> Result<SimulationReport, SimulationError> {
    run_simulation(&config(settings), &setup, streams, strategy)
}

/// The single fill a scenario was built to produce.
fn only_fill(report: &SimulationReport) -> &FillRecord {
    assert_eq!(report.fills.len(), 1, "expected exactly one fill");
    &report.fills[0]
}

// ── Test strategies ────────────────────────────────────────────────

struct NoopStrategy;

impl ReferenceStrategy for NoopStrategy {
    fn on_bar_close(
        &mut self,
        _ctx: &DecisionContext<'_>,
        _orders: &mut OrderIntents,
    ) -> Result<(), StrategyError> {
        Ok(())
    }
}

/// Emits market orders for the deciding symbol on scripted decision indices.
/// Decisions are counted in the order the simulator makes them, which is the
/// property under test everywhere else, so the script is a total order too.
struct ScriptedStrategy {
    script: Vec<(usize, OrderSide, f64)>,
    decisions: usize,
}

impl ScriptedStrategy {
    fn new(script: Vec<(usize, OrderSide, f64)>) -> Self {
        Self {
            script,
            decisions: 0,
        }
    }
}

impl ReferenceStrategy for ScriptedStrategy {
    fn on_bar_close(
        &mut self,
        ctx: &DecisionContext<'_>,
        orders: &mut OrderIntents,
    ) -> Result<(), StrategyError> {
        let now = self.decisions;
        self.decisions += 1;
        for (at, side, quantity) in &self.script {
            if *at == now {
                orders.market(ctx.symbol(), *side, *quantity)?;
            }
        }
        Ok(())
    }
}

/// Emits arbitrary order requests on scripted decision indices, keeping the
/// client ids it was handed so a later decision can cancel or modify them.
struct OrderScript {
    script: Vec<(usize, OrderRequest)>,
    decisions: usize,
    submitted: Vec<ClientOrderId>,
}

impl OrderScript {
    fn new(script: Vec<(usize, OrderRequest)>) -> Self {
        Self {
            script,
            decisions: 0,
            submitted: Vec::new(),
        }
    }
}

impl ReferenceStrategy for OrderScript {
    fn on_bar_close(
        &mut self,
        _ctx: &DecisionContext<'_>,
        orders: &mut OrderIntents,
    ) -> Result<(), StrategyError> {
        let now = self.decisions;
        self.decisions += 1;
        for (at, request) in &self.script {
            if *at == now {
                let id = orders.submit(request.clone())?;
                self.submitted.push(id);
            }
        }
        Ok(())
    }
}

/// Records what the market view answers, so the no-look-ahead guard can be
/// asserted from the outside without the strategy panicking inside the loop.
#[derive(Default)]
struct ProbeStrategy {
    decisions: usize,
    latest_close: Option<f64>,
    one_ago: Option<Result<f64, MarketDataError>>,
    beyond_history: Option<MarketDataError>,
    other_forming_open: Option<Result<f64, MarketDataError>>,
    other_visible: Option<Result<f64, MarketDataError>>,
}

impl ReferenceStrategy for ProbeStrategy {
    fn on_bar_close(
        &mut self,
        ctx: &DecisionContext<'_>,
        _orders: &mut OrderIntents,
    ) -> Result<(), StrategyError> {
        let market = ctx.market();
        if self.decisions == 0 {
            self.latest_close = market.close(ctx.symbol(), 0).ok();
            self.one_ago = Some(market.close(ctx.symbol(), 1));
            self.beyond_history = market.close(ctx.symbol(), 99).err();
            if let Some(other) = market.symbol_id("bbb") {
                self.other_forming_open = Some(market.opening_price(other));
                self.other_visible = Some(market.close(other, 0));
            }
        }
        self.decisions += 1;
        Ok(())
    }
}

/// Fails on a chosen decision index.
struct FailingStrategy {
    fail_at: usize,
    decisions: usize,
}

impl ReferenceStrategy for FailingStrategy {
    fn on_bar_close(
        &mut self,
        _ctx: &DecisionContext<'_>,
        _orders: &mut OrderIntents,
    ) -> Result<(), StrategyError> {
        let now = self.decisions;
        self.decisions += 1;
        if now == self.fail_at {
            return Err(StrategyError::Rejected {
                reason: "scripted failure".to_string(),
            });
        }
        Ok(())
    }
}

/// Pushes more intents than one decision may carry.
struct FloodStrategy;

impl ReferenceStrategy for FloodStrategy {
    fn on_bar_close(
        &mut self,
        ctx: &DecisionContext<'_>,
        orders: &mut OrderIntents,
    ) -> Result<(), StrategyError> {
        for _ in 0..=MAX_INTENTS_PER_DECISION {
            orders.market(ctx.symbol(), OrderSide::Buy, 1.0)?;
        }
        Ok(())
    }
}

// The slices below share every fixture above; `include!` textually
// concatenates them into this one module scope (ADR-118 Technique 2), so a
// fixture defined here stays visible to every slice.
include!("core.rs");
include!("legacy_equivalence.rs");
include!("orders.rs");
include!("golden.rs");
include!("determinism.rs");
include!("lookahead.rs");
