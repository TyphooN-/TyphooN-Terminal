use super::*;
use crate::core::strategy_ir::{
    CommissionModel, ExecutionSettings, OhlcAmbiguityPolicy, SlippageModel, SpreadModel,
    StrategyExecutionConfig, TieBreakPolicy,
};

const MINUTE_NS: i64 = 60_000_000_000;
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

fn config(settings: ExecutionSettings) -> StrategyExecutionConfig {
    StrategyExecutionConfig::build(&settings).expect("settings are valid")
}

fn run(
    settings: ExecutionSettings,
    streams: &[SymbolStream],
    strategy: &mut dyn ReferenceStrategy,
) -> Result<SimulationReport, SimulationError> {
    run_simulation(&config(settings), streams, strategy)
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

// ── Ordering and determinism ───────────────────────────────────────

#[test]
fn event_log_is_strictly_ordered_by_time_priority_sequence() {
    let streams = [ramp("aaa", 4), ramp("zzz", 4)];
    let report = run(
        free_settings(),
        &streams,
        &mut ScriptedStrategy::new(vec![(0, OrderSide::Buy, 3.0)]),
    )
    .expect("run succeeds");

    let mut previous: Option<(i64, u8, u64)> = None;
    for record in &report.events {
        let key = (record.time_ns, record.kind.priority(), record.sequence);
        if let Some(prev) = previous {
            assert!(
                prev < key,
                "event log is out of order: {prev:?} then {key:?}"
            );
        }
        previous = Some(key);
    }
    assert!(report.events.len() > 8, "expected a populated event log");
}

#[test]
fn cross_symbol_ties_resolve_by_symbol_table_order() {
    let streams = [ramp("zzz", 3), ramp("aaa", 3)];
    let report = run(free_settings(), &streams, &mut NoopStrategy).expect("run succeeds");

    // The symbol table is sorted, not input-ordered.
    assert_eq!(report.symbols, vec!["aaa".to_string(), "zzz".to_string()]);

    let opens_at_bar_one: Vec<SymbolId> = report
        .events
        .iter()
        .filter(|e| e.kind == SimEventKind::BarOpen && e.time_ns == MINUTE_NS)
        .filter_map(|e| e.symbol)
        .collect();
    assert_eq!(opens_at_bar_one, vec![SymbolId(0), SymbolId(1)]);

    let closes_at_bar_zero: Vec<SymbolId> = report
        .events
        .iter()
        .filter(|e| e.kind == SimEventKind::BarClose && e.time_ns == MINUTE_NS - 1)
        .filter_map(|e| e.symbol)
        .collect();
    assert_eq!(closes_at_bar_zero, vec![SymbolId(0), SymbolId(1)]);
}

#[test]
fn identical_input_produces_byte_identical_json() {
    let streams = [ramp("aaa", 6), offset_ramp("bbb", 6)];
    let script = vec![(0, OrderSide::Buy, 4.0), (5, OrderSide::Sell, 9.0)];

    let first = run(
        costed_settings(),
        &streams,
        &mut ScriptedStrategy::new(script.clone()),
    )
    .expect("first run succeeds");
    let second = run(
        costed_settings(),
        &streams,
        &mut ScriptedStrategy::new(script),
    )
    .expect("second run succeeds");

    assert_eq!(first, second);
    let left = serde_json::to_string(&first).expect("report serializes");
    let right = serde_json::to_string(&second).expect("report serializes");
    assert_eq!(left, right);

    let round_tripped: SimulationReport = serde_json::from_str(&left).expect("report parses");
    assert_eq!(
        serde_json::to_string(&round_tripped).expect("report re-serializes"),
        left
    );
}

// ── Decision / execution separation ────────────────────────────────

#[test]
fn market_order_submitted_at_close_fills_at_next_open() {
    let streams = [ramp("aaa", 4)];
    let report = run(
        free_settings(),
        &streams,
        &mut ScriptedStrategy::new(vec![(0, OrderSide::Buy, 10.0)]),
    )
    .expect("run succeeds");

    assert_eq!(report.fills.len(), 1);
    let fill = &report.fills[0];
    assert_eq!(fill.time_ns, MINUTE_NS, "fill belongs to the next bar open");
    assert_close(fill.fill_price, 101.0, "fill price is the next bar open");
    assert_close(fill.quantity, 10.0, "quantity");

    let decision = report
        .events
        .iter()
        .find(|e| e.kind == SimEventKind::Decision)
        .expect("a decision was made");
    assert_eq!(decision.time_ns, MINUTE_NS - 1);
    assert!(
        fill.time_ns > decision.time_ns,
        "decision and execution collapsed onto one timestamp"
    );
    assert!(fill.sequence > decision.sequence);

    // The closing bar's own prices are never a fill price.
    assert!(
        !report
            .fills
            .iter()
            .any(|f| (f.fill_price - 100.5).abs() < EPS),
        "an order filled at the decision bar's close"
    );
}

#[test]
fn submit_activate_fill_are_explicit_ordered_events() {
    let streams = [ramp("aaa", 3)];
    let report = run(
        free_settings(),
        &streams,
        &mut ScriptedStrategy::new(vec![(0, OrderSide::Buy, 2.0)]),
    )
    .expect("run succeeds");

    let seq_of = |kind: SimEventKind| -> u64 {
        report
            .events
            .iter()
            .find(|e| e.kind == kind)
            .unwrap_or_else(|| panic!("missing {kind:?} event"))
            .sequence
    };
    let submit = seq_of(SimEventKind::OrderSubmit);
    let activate = seq_of(SimEventKind::OrderActivate);
    let fill = seq_of(SimEventKind::Fill);
    assert!(submit < activate && activate < fill);

    let submit_time = report
        .events
        .iter()
        .find(|e| e.kind == SimEventKind::OrderSubmit)
        .map(|e| e.time_ns)
        .expect("submit event");
    let activate_time = report
        .events
        .iter()
        .find(|e| e.kind == SimEventKind::OrderActivate)
        .map(|e| e.time_ns)
        .expect("activate event");
    assert!(activate_time > submit_time);
}

#[test]
fn market_view_refuses_data_beyond_the_committed_bar() {
    let streams = [ramp("aaa", 4), offset_ramp("bbb", 4)];
    let mut probe = ProbeStrategy::default();
    let report = run(free_settings(), &streams, &mut probe).expect("run succeeds");
    assert_eq!(report.symbols, vec!["aaa".to_string(), "bbb".to_string()]);

    assert_close(
        probe.latest_close.expect("latest close is visible"),
        100.5,
        "bars_ago 0 is the bar that just closed",
    );
    assert_eq!(
        probe.one_ago,
        Some(Err(MarketDataError::FutureData {
            symbol: SymbolId(0),
            bars_ago: 1,
            available: 1,
        })),
        "there is no bar before the first one"
    );
    assert_eq!(
        probe.beyond_history,
        Some(MarketDataError::FutureData {
            symbol: SymbolId(0),
            bars_ago: 99,
            available: 1,
        })
    );

    // The other symbol's bar is open but not closed: its open is readable and
    // nothing else is.
    assert_eq!(probe.other_forming_open, Some(Ok(200.0)));
    assert_eq!(
        probe.other_visible,
        Some(Err(MarketDataError::FutureData {
            symbol: SymbolId(1),
            bars_ago: 0,
            available: 0,
        })),
        "a forming bar must not be readable as a completed one"
    );
}

// ── Cost model ─────────────────────────────────────────────────────

#[test]
fn buy_pays_the_ask_plus_slippage_and_commission() {
    let streams = [ramp("aaa", 3)];
    let report = run(
        costed_settings(),
        &streams,
        &mut ScriptedStrategy::new(vec![(0, OrderSide::Buy, 10.0)]),
    )
    .expect("run succeeds");

    let fill = report.fills.first().expect("one fill");
    assert_eq!(fill.side, OrderSide::Buy);
    assert_close(fill.reference_price, 101.0, "reference is the bar open");
    assert_close(fill.quoted_price, 101.05, "buys quote the ask");
    assert_close(fill.fill_price, 101.07, "slippage is adverse");
    assert_close(fill.spread_cost, 0.5, "half spread on 10 units");
    assert_close(fill.slippage_cost, 0.2, "slippage on 10 units");
    assert_close(
        fill.commission,
        1.0,
        "per-share commission hits its minimum",
    );
    assert_close(
        fill.cash_after,
        START_CAPITAL - 101.07 * 10.0 - 1.0,
        "cash after a buy",
    );
    assert_close(
        fill.avg_entry_after,
        101.07,
        "average entry is the fill price",
    );
}

#[test]
fn sell_receives_the_bid_minus_slippage_and_commission() {
    let streams = [ramp("aaa", 3)];
    let report = run(
        costed_settings(),
        &streams,
        &mut ScriptedStrategy::new(vec![(0, OrderSide::Sell, 10.0)]),
    )
    .expect("run succeeds");

    let fill = report.fills.first().expect("one fill");
    assert_eq!(fill.side, OrderSide::Sell);
    assert_close(fill.quoted_price, 100.95, "sells quote the bid");
    assert_close(fill.fill_price, 100.93, "slippage is adverse");
    assert_close(
        fill.cash_after,
        START_CAPITAL + 100.93 * 10.0 - 1.0,
        "cash after a short sale",
    );
    assert_close(fill.position_units_after, -10.0, "short position is signed");
}

#[test]
fn percent_cost_models_are_applied_per_fill() {
    let settings = ExecutionSettings {
        commission: CommissionModel::PercentOfNotional {
            percent: 0.1,
            minimum: 0.0,
        },
        slippage: SlippageModel::SpreadFraction { fraction: 0.5 },
        spread: SpreadModel::PercentOfPrice { percent: 1.0 },
        ..free_settings()
    };
    let streams = [ramp("aaa", 3)];
    let report = run(
        settings,
        &streams,
        &mut ScriptedStrategy::new(vec![(0, OrderSide::Buy, 2.0)]),
    )
    .expect("run succeeds");

    let fill = report.fills.first().expect("one fill");
    let width = 101.0 * 0.01;
    assert_close(
        fill.quoted_price,
        101.0 + width / 2.0,
        "ask from percent spread",
    );
    assert_close(
        fill.fill_price,
        101.0 + width / 2.0 + width * 0.5,
        "slippage as a fraction of the spread",
    );
    assert_close(
        fill.commission,
        fill.fill_price * 2.0 * 0.001,
        "commission as a percent of notional",
    );
}

#[test]
fn unsupported_execution_models_are_rejected() {
    let streams = [ramp("aaa", 3)];

    let volatility = ExecutionSettings {
        slippage: SlippageModel::VolatilityScaled { atr_fraction: 0.5 },
        ..free_settings()
    };
    assert_eq!(
        run(volatility, &streams, &mut NoopStrategy),
        Err(SimulationError::UnsupportedModel {
            field: "settings.slippage",
            model: "volatility_scaled",
        })
    );

    let quotes = ExecutionSettings {
        spread: SpreadModel::RecordedQuotes,
        ..free_settings()
    };
    assert_eq!(
        run(quotes, &streams, &mut NoopStrategy),
        Err(SimulationError::UnsupportedModel {
            field: "settings.spread",
            model: "recorded_quotes",
        })
    );

    let tie_break = ExecutionSettings {
        tie_break: TieBreakPolicy::TimestampPrioritySymbolSequence,
        ..free_settings()
    };
    assert_eq!(
        run(tie_break, &streams, &mut NoopStrategy),
        Err(SimulationError::UnsupportedModel {
            field: "settings.tie_break",
            model: "timestamp_priority_symbol_sequence",
        })
    );
}

// ── Accounting ─────────────────────────────────────────────────────

#[test]
fn round_trip_accounting_is_exact_under_zero_costs() {
    let streams = [ramp("aaa", 6)];
    let report = run(
        free_settings(),
        &streams,
        &mut ScriptedStrategy::new(vec![(0, OrderSide::Buy, 10.0), (2, OrderSide::Sell, 10.0)]),
    )
    .expect("run succeeds");

    assert_eq!(report.fills.len(), 2);
    assert_close(report.fills[0].fill_price, 101.0, "entry at bar 1 open");
    assert_close(report.fills[1].fill_price, 103.0, "exit at bar 3 open");
    assert_close(report.final_realized_pnl, 20.0, "10 units over 2 points");
    assert_close(
        report.final_cash,
        START_CAPITAL + 20.0,
        "cash after a flat round trip",
    );
    assert_close(
        report.final_equity,
        START_CAPITAL + 20.0,
        "equity when flat",
    );
    assert_close(report.total_commission, 0.0, "zero-cost baseline");

    let position = report.positions.first().expect("one position record");
    assert_close(position.units, 0.0, "position is flat");
    assert_close(position.unrealized_pnl, 0.0, "nothing left to mark");
}

#[test]
fn equity_identity_holds_with_costs() {
    let streams = [ramp("aaa", 8)];
    let report = run(
        costed_settings(),
        &streams,
        &mut ScriptedStrategy::new(vec![(0, OrderSide::Buy, 10.0), (3, OrderSide::Sell, 4.0)]),
    )
    .expect("run succeeds");

    let unrealized: f64 = report.positions.iter().map(|p| p.unrealized_pnl).sum();
    assert_close(
        report.final_equity,
        START_CAPITAL + report.final_realized_pnl - report.total_commission + unrealized,
        "equity = capital + realized - commission + unrealized",
    );
    assert!(report.total_commission > 0.0, "commission was charged");

    let last = report.equity_curve.last().expect("an equity curve");
    assert_close(
        last.equity,
        report.final_equity,
        "curve ends at final equity",
    );
    assert_close(last.cash, report.final_cash, "curve ends at final cash");
}

#[test]
fn equity_curve_has_one_point_per_bar_close_plus_the_final_mark() {
    let streams = [ramp("aaa", 5), ramp("zzz", 5)];
    let report = run(free_settings(), &streams, &mut NoopStrategy).expect("run succeeds");
    assert_eq!(report.equity_curve.len(), 5 * 2 + 1);
    assert!(
        report
            .equity_curve
            .windows(2)
            .all(|w| w[0].sequence < w[1].sequence),
        "equity points must carry increasing sequences"
    );
}

#[test]
fn reversal_realizes_the_old_side_and_opens_the_new_one() {
    let streams = [ramp("aaa", 6)];
    let report = run(
        free_settings(),
        &streams,
        &mut ScriptedStrategy::new(vec![(0, OrderSide::Buy, 10.0), (2, OrderSide::Sell, 25.0)]),
    )
    .expect("run succeeds");

    assert_eq!(report.fills.len(), 2);
    let reversal = &report.fills[1];
    assert_close(reversal.realized_pnl, 20.0, "10 long units closed at +2");
    assert_close(reversal.position_units_after, -15.0, "flipped to short");
    assert_close(
        reversal.avg_entry_after,
        103.0,
        "new basis is the fill price",
    );

    let position = report.positions.first().expect("one position record");
    assert_close(position.units, -15.0, "still short at the end");
    assert_close(
        position.realized_pnl,
        20.0,
        "realized PnL survives the flip",
    );
}

#[test]
fn partial_close_keeps_the_original_basis() {
    let streams = [ramp("aaa", 6)];
    let report = run(
        free_settings(),
        &streams,
        &mut ScriptedStrategy::new(vec![(0, OrderSide::Buy, 10.0), (2, OrderSide::Sell, 4.0)]),
    )
    .expect("run succeeds");

    let close = &report.fills[1];
    assert_close(close.realized_pnl, 8.0, "4 units over 2 points");
    assert_close(close.position_units_after, 6.0, "6 units remain");
    assert_close(
        close.avg_entry_after,
        101.0,
        "basis is untouched by a partial close",
    );
}

#[test]
fn end_of_data_marks_to_market_without_a_synthetic_fill() {
    let streams = [ramp("aaa", 4)];
    let report = run(
        free_settings(),
        &streams,
        &mut ScriptedStrategy::new(vec![(0, OrderSide::Buy, 10.0)]),
    )
    .expect("run succeeds");

    assert_eq!(report.fills.len(), 1, "no closing fill may be fabricated");
    let last_close = 103.5;
    let position = report.positions.first().expect("one position record");
    assert_close(position.units, 10.0, "the position stays open");
    assert_close(
        position.mark_price.expect("marked"),
        last_close,
        "final mark",
    );
    assert_close(
        position.unrealized_pnl,
        10.0 * (last_close - 101.0),
        "unrealized",
    );
    assert_close(
        report.final_equity,
        report.final_cash + 10.0 * last_close,
        "equity marks the open position",
    );
    assert_close(report.final_realized_pnl, 0.0, "nothing was realized");

    let final_event = report.events.last().expect("a final event");
    assert_eq!(final_event.kind, SimEventKind::MarkToMarket);
    assert_eq!(
        final_event.time_ns,
        3 * MINUTE_NS + MINUTE_NS - 1,
        "the final mark sits on the last bar close"
    );
}

#[test]
fn an_order_with_no_later_bar_open_is_reported_unfilled() {
    let streams = [ramp("aaa", 3)];
    let report = run(
        free_settings(),
        &streams,
        &mut ScriptedStrategy::new(vec![(2, OrderSide::Buy, 10.0)]),
    )
    .expect("run succeeds");

    assert!(
        report.fills.is_empty(),
        "there is no bar left to fill against"
    );
    assert_eq!(report.pending_orders.len(), 1);
    let pending = &report.pending_orders[0];
    assert_eq!(pending.side, OrderSide::Buy);
    assert_close(pending.quantity, 10.0, "pending quantity");
    assert_close(
        report.final_cash,
        START_CAPITAL,
        "an unfilled order costs nothing",
    );
}

// ── Failure modes ──────────────────────────────────────────────────

#[test]
fn a_strategy_error_aborts_deterministically() {
    let streams = [ramp("aaa", 5)];
    let expected = SimulationError::Strategy {
        time_ns: MINUTE_NS + MINUTE_NS - 1,
        error: StrategyError::Rejected {
            reason: "scripted failure".to_string(),
        },
    };
    for _ in 0..2 {
        let result = run(
            free_settings(),
            &streams,
            &mut FailingStrategy {
                fail_at: 1,
                decisions: 0,
            },
        );
        assert_eq!(result, Err(expected.clone()));
    }
}

#[test]
fn one_decision_may_not_flood_the_queue() {
    let streams = [ramp("aaa", 3)];
    let result = run(free_settings(), &streams, &mut FloodStrategy);
    assert_eq!(
        result,
        Err(SimulationError::Strategy {
            time_ns: MINUTE_NS - 1,
            error: StrategyError::TooManyIntents {
                limit: MAX_INTENTS_PER_DECISION,
            },
        })
    );
}

#[test]
fn invalid_order_quantities_are_rejected() {
    struct BadQuantity(f64);
    impl ReferenceStrategy for BadQuantity {
        fn on_bar_close(
            &mut self,
            ctx: &DecisionContext<'_>,
            orders: &mut OrderIntents,
        ) -> Result<(), StrategyError> {
            orders.market(ctx.symbol(), OrderSide::Buy, self.0)
        }
    }
    let streams = [ramp("aaa", 3)];
    for quantity in [0.0, -1.0, f64::NAN, f64::INFINITY, MAX_ORDER_QUANTITY * 2.0] {
        let result = run(free_settings(), &streams, &mut BadQuantity(quantity));
        assert!(
            matches!(
                result,
                Err(SimulationError::Strategy {
                    error: StrategyError::InvalidQuantity { .. },
                    ..
                })
            ),
            "quantity {quantity} was accepted"
        );
    }
}

#[test]
fn an_unknown_symbol_id_is_rejected() {
    struct WrongSymbol;
    impl ReferenceStrategy for WrongSymbol {
        fn on_bar_close(
            &mut self,
            _ctx: &DecisionContext<'_>,
            orders: &mut OrderIntents,
        ) -> Result<(), StrategyError> {
            orders.market(SymbolId(9), OrderSide::Buy, 1.0)
        }
    }
    let streams = [ramp("aaa", 3)];
    assert_eq!(
        run(free_settings(), &streams, &mut WrongSymbol),
        Err(SimulationError::Strategy {
            time_ns: MINUTE_NS - 1,
            error: StrategyError::UnknownSymbol { id: 9 },
        })
    );
}

#[test]
fn empty_and_duplicated_input_is_rejected() {
    assert_eq!(
        run(free_settings(), &[], &mut NoopStrategy),
        Err(SimulationError::NoSymbols)
    );
    assert_eq!(
        run(
            free_settings(),
            &[ramp("aaa", 3), ramp("aaa", 3)],
            &mut NoopStrategy
        ),
        Err(SimulationError::DuplicateSymbol {
            symbol: "aaa".to_string()
        })
    );
    assert_eq!(
        run(
            free_settings(),
            &[SymbolStream {
                symbol: "aaa".to_string(),
                bars: Vec::new(),
            }],
            &mut NoopStrategy
        ),
        Err(SimulationError::EmptyStream {
            symbol: "aaa".to_string()
        })
    );
    assert!(matches!(
        run(free_settings(), &[ramp(" aaa", 3)], &mut NoopStrategy),
        Err(SimulationError::InvalidSymbol { .. })
    ));
    assert!(matches!(
        run(
            free_settings(),
            &[ramp(&"a".repeat(MAX_SYMBOL_LEN + 1), 3)],
            &mut NoopStrategy
        ),
        Err(SimulationError::InvalidSymbol { .. })
    ));
}

#[test]
fn malformed_bars_are_rejected() {
    let defective = |mutate: fn(&mut SimBar)| {
        let mut stream = ramp("aaa", 3);
        if let Some(bar) = stream.bars.get_mut(1) {
            mutate(bar);
        }
        run(free_settings(), &[stream], &mut NoopStrategy)
    };

    assert!(matches!(
        defective(|bar| bar.high = f64::NAN),
        Err(SimulationError::NonFiniteBar { .. })
    ));
    assert!(matches!(
        defective(|bar| bar.high = bar.low - 1.0),
        Err(SimulationError::InconsistentBar {
            defect: BarDefect::HighBelowLow,
            ..
        })
    ));
    assert!(matches!(
        defective(|bar| bar.open = bar.high + 1.0),
        Err(SimulationError::InconsistentBar {
            defect: BarDefect::OpenOutsideRange,
            ..
        })
    ));
    assert!(matches!(
        defective(|bar| bar.close = bar.low - 1.0),
        Err(SimulationError::InconsistentBar {
            defect: BarDefect::CloseOutsideRange,
            ..
        })
    ));
    assert!(matches!(
        defective(|bar| bar.low = -1.0),
        Err(SimulationError::InconsistentBar {
            defect: BarDefect::NonPositivePrice,
            ..
        })
    ));
    assert!(matches!(
        defective(|bar| bar.volume = -1.0),
        Err(SimulationError::InconsistentBar {
            defect: BarDefect::NegativeVolume,
            ..
        })
    ));
    assert!(matches!(
        defective(|bar| bar.close_time_ns = bar.open_time_ns),
        Err(SimulationError::InconsistentBar {
            defect: BarDefect::NonPositiveDuration,
            ..
        })
    ));
    // Bar 1 now opens on bar 0's closing nanosecond.
    assert!(matches!(
        defective(|bar| bar.open_time_ns = MINUTE_NS - 1),
        Err(SimulationError::OverlappingBars { .. })
    ));
    // …and out of order entirely.
    assert!(matches!(
        defective(|bar| {
            bar.open_time_ns = 0;
            bar.close_time_ns = 10;
        }),
        Err(SimulationError::OverlappingBars { .. })
    ));
}

#[test]
fn oversized_input_is_rejected() {
    let too_many_symbols: Vec<SymbolStream> = (0..=MAX_SYMBOLS)
        .map(|i| ramp(&format!("s{i:04}"), 2))
        .collect();
    assert_eq!(
        run(free_settings(), &too_many_symbols, &mut NoopStrategy),
        Err(SimulationError::TooManySymbols {
            limit: MAX_SYMBOLS,
            found: MAX_SYMBOLS + 1,
        })
    );

    let mut long = ramp("aaa", 1);
    let template = long.bars[0];
    long.bars = (0..MAX_BARS_PER_SYMBOL + 1)
        .map(|i| SimBar {
            open_time_ns: i as i64 * MINUTE_NS,
            close_time_ns: i as i64 * MINUTE_NS + MINUTE_NS - 1,
            ..template
        })
        .collect();
    assert_eq!(
        run(free_settings(), &[long], &mut NoopStrategy),
        Err(SimulationError::TooManyBars {
            symbol: "aaa".to_string(),
            limit: MAX_BARS_PER_SYMBOL,
            found: MAX_BARS_PER_SYMBOL + 1,
        })
    );
}

#[test]
fn total_bar_budget_is_bounded_across_symbols() {
    let per_symbol = MAX_TOTAL_BARS / 2 + 1;
    let streams = [ramp("aaa", per_symbol), ramp("bbb", per_symbol)];
    assert!(matches!(
        run(free_settings(), &streams, &mut NoopStrategy),
        Err(SimulationError::TooManyTotalBars { limit, found })
            if limit == MAX_TOTAL_BARS && found == per_symbol * 2
    ));
}

#[test]
fn non_finite_accounting_results_are_rejected() {
    let price = f64::MAX / 4.0;
    let bars = vec![
        minute_bar(0, price, price, price, price),
        minute_bar(1, price, price, price, price),
    ];
    let streams = [SymbolStream {
        symbol: "aaa".to_string(),
        bars,
    }];
    let result = run(
        free_settings(),
        &streams,
        &mut ScriptedStrategy::new(vec![(0, OrderSide::Buy, MAX_ORDER_QUANTITY)]),
    );
    assert!(matches!(
        result,
        Err(SimulationError::NonFiniteAccounting { .. })
    ));
}

#[test]
fn a_sealed_config_is_verified_before_the_run() {
    let mut broken = config(free_settings());
    broken.config_id = "0".repeat(64);
    let streams = [ramp("aaa", 3)];
    assert!(matches!(
        run_simulation(&broken, &streams, &mut NoopStrategy),
        Err(SimulationError::Config(_))
    ));
}

#[test]
fn errors_render_without_leaking_internals() {
    let rendered = SimulationError::UnsupportedModel {
        field: "settings.spread",
        model: "recorded_quotes",
    }
    .to_string();
    assert!(rendered.contains("settings.spread"));
    assert!(rendered.contains("recorded_quotes"));
}
