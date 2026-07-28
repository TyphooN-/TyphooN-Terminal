// ── Two-leg lifecycle corpus (ADR-135 §13 M2 gate, §10.3) ──────────
//
// Every price, fee and cash balance below is derived by hand in the comment
// that precedes it, from the bar rows in the same test. Nothing is a recorded
// snapshot: if the engine changes its mind about a fill, one of these numbers
// has to be re-derived on paper first.

use super::{LegPlan, ProtectiveError, ProtectiveManager, ProtectivePlan, TrailingPlan};
use crate::core::strategy_ir::{
    CommissionModel, ExecutionSettings, FidelityLevel, OhlcAmbiguityPolicy, SlippageModel,
    SpreadModel, StrategyExecutionConfig, TieBreakPolicy,
};
use crate::core::strategy_simulator::{
    DecisionContext, OrderIntents, OrderSide, SimBar, SimulationReport, SimulationSetup,
    StrategyError, SymbolStream, run_simulation,
};

const MINUTE_NS: i64 = 60_000_000_000;
const EPS: f64 = 1e-9;

fn assert_close(actual: f64, expected: f64, what: &str) {
    assert!(
        (actual - expected).abs() < EPS,
        "{what}: expected {expected}, got {actual}"
    );
}

fn bar(index: i64, open: f64, high: f64, low: f64, close: f64) -> SimBar {
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

/// Intrabar resolution and a per-share commission with a 1.00 floor, but no
/// spread or slippage — so a fill price is the protective level itself and the
/// only cost to account for is the fee.
fn managed_settings() -> ExecutionSettings {
    ExecutionSettings {
        fidelity: FidelityLevel::BarOhlc,
        commission: CommissionModel::PerShare {
            amount: 0.01,
            minimum: 1.0,
        },
        slippage: SlippageModel::None,
        spread: SpreadModel::None,
        ambiguity: OhlcAmbiguityPolicy::StopFirst,
        tie_break: TieBreakPolicy::TimestampPrioritySequence,
        ..ExecutionSettings::conservative_defaults()
    }
}

/// Enters once at a scripted decision, then hands every later decision to the
/// protective manager.
struct ManagedEntry {
    decisions: usize,
    enter_at: usize,
    /// Decision at which the strategy closes the position itself, as an IR exit
    /// condition would, leaving the bracket to be cleaned up.
    exit_at: Option<usize>,
    reference_price: f64,
    side: OrderSide,
    plan: ProtectivePlan,
    manager: ProtectiveManager,
    /// Stop price observed for each leg after every decision, so the
    /// break-even and trailing transitions can be asserted in order.
    stop_trace: Vec<(usize, Option<f64>, Option<f64>)>,
}

impl ManagedEntry {
    fn new(enter_at: usize, side: OrderSide, reference_price: f64, plan: ProtectivePlan) -> Self {
        Self {
            decisions: 0,
            enter_at,
            exit_at: None,
            reference_price,
            side,
            plan,
            manager: ProtectiveManager::new(),
            stop_trace: Vec::new(),
        }
    }

    fn closing_at(mut self, decision: usize) -> Self {
        self.exit_at = Some(decision);
        self
    }
}

impl crate::core::strategy_simulator::ReferenceStrategy for ManagedEntry {
    fn on_bar_close(
        &mut self,
        ctx: &DecisionContext<'_>,
        orders: &mut OrderIntents,
    ) -> Result<(), StrategyError> {
        let now = self.decisions;
        self.decisions += 1;
        if now == self.enter_at {
            self.manager.enter(
                ctx.symbol(),
                self.side,
                self.reference_price,
                &self.plan,
                orders,
            )?;
        } else {
            if self.exit_at == Some(now) {
                let position = ctx.own_position();
                let exit_side = match self.side {
                    OrderSide::Buy => OrderSide::Sell,
                    OrderSide::Sell => OrderSide::Buy,
                };
                orders.submit(
                    crate::core::strategy_simulator::OrderRequest::market(
                        ctx.symbol(),
                        exit_side,
                        position.units.abs(),
                    )
                    .reduce_only(),
                )?;
            }
            self.manager.on_decision(ctx, orders)?;
        }
        self.stop_trace.push((
            now,
            self.manager.leg_stop_price(0),
            self.manager.leg_stop_price(1),
        ));
        Ok(())
    }
}

fn run(streams: &[SymbolStream], strategy: &mut ManagedEntry) -> SimulationReport {
    let config = StrategyExecutionConfig::build(&managed_settings()).expect("settings are valid");
    run_simulation(&config, &SimulationSetup::default(), streams, strategy).expect("runs")
}

/// The full NNFX two-leg lifecycle, hand-computed end to end.
///
/// Bars (open, high, low, close), one minute apart:
///   0: 100.0 100.5  99.5 100.0  decision 0: enter long 10 at reference 100
///   1: 100.0 101.0  99.8 100.8  the entry fills at this open
///   2: 100.8 105.0 100.5 104.5  leg 0's target at 104 fills intrabar
///   3: 104.5 107.0 104.0 106.5  nothing fills; the trail arms
///   4: 106.5 107.0 103.0 103.5  leg 1's trailed stop at 104 fills intrabar
///   5: 103.5 104.0 103.0 103.5  flat; final mark to market
///
/// Plan: two 5-unit legs, both stopped 2.00 below entry (98.00). Leg 0 targets
/// +4.00 (104.00). Leg 1 has no target and trails 3.00 behind the high-water
/// mark once price has run +6.00. Break-even moves every surviving stop to the
/// entry once price has run +4.00.
///
/// Lifecycle by decision:
///   d1 (bar 1 close): extreme 101.00, favourable +1.00 — below both triggers,
///      so leg 1's stop stays at its initial 98.00.
///   d2 (bar 2 close): leg 0 has already banked its target. Extreme 105.00,
///      favourable +5.00 ≥ 4.00 → break-even moves leg 1's stop 98.00 → 100.00.
///      The trail needs +6.00, so it stays dormant.
///   d3 (bar 3 close): extreme 107.00, favourable +7.00 ≥ 6.00 → the trail
///      arms and proposes 107.00 − 3.00 = 104.00, which tightens 100.00.
///   bar 4: low 103.00 ≤ 104.00 → leg 1 exits at 104.00.
///
/// Ledger, from 100 000.00 of capital and a fee of max(0.01 × qty, 1.00):
///   buy 10 @ 100.00, fee 1.00  → cash 100 000 − 1 000 − 1     =  98 999.00
///   sell 5 @ 104.00, fee 1.00  → cash  98 999 +   520 − 1     =  99 518.00
///                                 realized (104 − 100) × 5    =      20.00
///   sell 5 @ 104.00, fee 1.00  → cash  99 518 +   520 − 1     = 100 037.00
///                                 realized (104 − 100) × 5    =      20.00
///   final: realized 40.00, fees 3.00, equity 100 037.00
#[test]
fn a_hand_computed_two_leg_trade_banks_its_target_moves_to_break_even_then_trails() {
    let stream = SymbolStream {
        symbol: "aaa".to_string(),
        bars: vec![
            bar(0, 100.0, 100.5, 99.5, 100.0),
            bar(1, 100.0, 101.0, 99.8, 100.8),
            bar(2, 100.8, 105.0, 100.5, 104.5),
            bar(3, 104.5, 107.0, 104.0, 106.5),
            bar(4, 106.5, 107.0, 103.0, 103.5),
            bar(5, 103.5, 104.0, 103.0, 103.5),
        ],
    };
    let plan = ProtectivePlan {
        legs: vec![
            LegPlan {
                quantity: 5.0,
                stop_distance: Some(2.0),
                target_distance: Some(4.0),
                trailing: None,
            },
            LegPlan {
                quantity: 5.0,
                stop_distance: Some(2.0),
                target_distance: None,
                trailing: Some(TrailingPlan {
                    distance: 3.0,
                    activate_after: Some(6.0),
                }),
            },
        ],
        break_even_after: Some(4.0),
        max_bars_in_trade: None,
    };
    let mut strategy = ManagedEntry::new(0, OrderSide::Buy, 100.0, plan);
    let report = run(&[stream], &mut strategy);

    // ── Entry ──────────────────────────────────────────────────────
    assert_eq!(report.fills.len(), 3, "entry plus one exit per leg");
    let entry = &report.fills[0];
    assert_eq!(entry.time_ns, MINUTE_NS, "fills at bar 1's open");
    assert_eq!(entry.side, OrderSide::Buy);
    assert_close(entry.quantity, 10.0, "both legs enter together");
    assert_close(entry.fill_price, 100.0, "entry fill");
    assert_close(entry.commission, 1.0, "entry fee is the 1.00 floor");
    assert_close(entry.cash_after, 98_999.0, "cash after entry");
    assert_close(entry.position_units_after, 10.0, "long 10");

    // ── Leg 0 exits at its own target, independent of leg 1 ────────
    let target = &report.fills[1];
    assert_eq!(target.time_ns, 2 * MINUTE_NS + MINUTE_NS - 1, "bar 2 close");
    assert_eq!(target.side, OrderSide::Sell);
    assert_close(target.quantity, 5.0, "only leg 0 leaves");
    assert_close(target.fill_price, 104.0, "the limit fills at its own price");
    assert_close(target.commission, 1.0, "fee floor again");
    assert_close(target.realized_pnl, 20.0, "(104 - 100) * 5");
    assert_close(target.cash_after, 99_518.0, "cash after the target");
    assert_close(target.position_units_after, 5.0, "leg 1 still runs");

    // ── Break-even, then the trailing step ─────────────────────────
    let stops: Vec<(usize, Option<f64>, Option<f64>)> = strategy.stop_trace.clone();
    assert_eq!(
        stops[0],
        (0, Some(98.0), Some(98.0)),
        "both legs open stopped 2.00 below the entry"
    );
    assert_eq!(
        stops[1],
        (1, Some(98.0), Some(98.0)),
        "+1.00 is below both the break-even and the trail trigger"
    );
    assert_eq!(
        stops[2].2,
        Some(100.0),
        "+5.00 moves the runner to break-even, not to a trail"
    );
    assert_eq!(
        stops[3].2,
        Some(104.0),
        "+7.00 arms the trail: 107.00 high-water less 3.00"
    );

    // ── Leg 1 exits on the trailed stop ────────────────────────────
    let trailed = &report.fills[2];
    assert_eq!(
        trailed.time_ns,
        4 * MINUTE_NS + MINUTE_NS - 1,
        "bar 4 close"
    );
    assert_eq!(trailed.side, OrderSide::Sell);
    assert_close(trailed.quantity, 5.0, "the runner");
    assert_close(trailed.fill_price, 104.0, "fills at the trailed stop");
    assert_close(trailed.commission, 1.0, "fee floor");
    assert_close(trailed.realized_pnl, 20.0, "(104 - 100) * 5");
    assert_close(trailed.cash_after, 100_037.0, "cash after the runner exits");
    assert_close(trailed.position_units_after, 0.0, "flat");

    // ── Ledger and equity ──────────────────────────────────────────
    assert_close(report.final_realized_pnl, 40.0, "20.00 per leg");
    assert_close(
        report.total_commission,
        3.0,
        "three fills at the 1.00 floor",
    );
    assert_close(report.final_cash, 100_037.0, "100 000 + 40 - 3");
    assert_close(report.final_equity, 100_037.0, "flat, so equity is cash");
}

/// The same plan, but price falls straight through both initial stops. Both
/// legs must exit at 98.00 — the stop is shared in level only, never in
/// lifecycle, so each leg is filled separately for its own quantity.
///
/// Bars: 0 opens the trade at 100.00; bar 2 trades down to 97.00.
///   sell 5 @ 98.00 twice, fee 1.00 each, realized (98 − 100) × 5 = −10.00 each
///   cash 98 999 + 490 − 1 + 490 − 1 = 99 977.00
#[test]
fn both_legs_stop_out_independently_at_the_same_initial_level() {
    let stream = SymbolStream {
        symbol: "aaa".to_string(),
        bars: vec![
            bar(0, 100.0, 100.5, 99.5, 100.0),
            bar(1, 100.0, 101.0, 99.8, 100.8),
            bar(2, 100.5, 100.8, 97.0, 97.5),
            bar(3, 97.5, 98.0, 97.0, 97.5),
        ],
    };
    let plan = ProtectivePlan {
        legs: vec![
            LegPlan {
                quantity: 5.0,
                stop_distance: Some(2.0),
                target_distance: Some(4.0),
                trailing: None,
            },
            LegPlan {
                quantity: 5.0,
                stop_distance: Some(2.0),
                target_distance: None,
                trailing: None,
            },
        ],
        break_even_after: Some(4.0),
        max_bars_in_trade: None,
    };
    let mut strategy = ManagedEntry::new(0, OrderSide::Buy, 100.0, plan);
    let report = run(&[stream], &mut strategy);

    assert_eq!(report.fills.len(), 3, "entry plus both stops");
    for (index, fill) in report.fills.iter().skip(1).enumerate() {
        assert_close(fill.quantity, 5.0, "each leg stops for its own size");
        assert_close(fill.fill_price, 98.0, "both stops sit at 98.00");
        assert_close(fill.realized_pnl, -10.0, "(98 - 100) * 5");
        assert_close(fill.commission, 1.0, "fee floor");
        let _ = index;
    }
    assert_close(report.final_realized_pnl, -20.0, "both legs lose 10.00");
    assert_close(report.total_commission, 3.0, "three fills");
    assert_close(report.final_cash, 99_977.0, "100 000 - 20 - 3");
    assert_close(report.final_equity, 99_977.0, "flat, so equity is cash");
}

/// A short two-leg trade mirrors the long one: the target sits below the entry,
/// the stop above, and the trail ratchets down.
///
/// Bars: enter short 10 at 100.00 on bar 0; bar 2 trades to 96.00, filling
/// leg 0's target at 96.00; bar 3's low of 93.00 sets the extreme; leg 1's
/// trail then rests 3.00 above at 96.00 and fills on bar 4's rally to 97.00.
#[test]
fn a_short_two_leg_trade_mirrors_the_long_lifecycle() {
    let stream = SymbolStream {
        symbol: "aaa".to_string(),
        bars: vec![
            bar(0, 100.0, 100.5, 99.5, 100.0),
            bar(1, 100.0, 100.2, 99.0, 99.5),
            bar(2, 99.5, 99.5, 95.0, 96.0),
            bar(3, 96.0, 96.0, 93.0, 93.5),
            bar(4, 93.5, 97.0, 93.0, 96.5),
            bar(5, 96.5, 97.0, 96.0, 96.5),
        ],
    };
    let plan = ProtectivePlan {
        legs: vec![
            LegPlan {
                quantity: 5.0,
                stop_distance: Some(2.0),
                target_distance: Some(4.0),
                trailing: None,
            },
            LegPlan {
                quantity: 5.0,
                stop_distance: Some(2.0),
                target_distance: None,
                trailing: Some(TrailingPlan {
                    distance: 3.0,
                    activate_after: Some(6.0),
                }),
            },
        ],
        break_even_after: Some(4.0),
        max_bars_in_trade: None,
    };
    let mut strategy = ManagedEntry::new(0, OrderSide::Sell, 100.0, plan);
    let report = run(&[stream], &mut strategy);

    assert_eq!(report.fills.len(), 3, "entry plus one exit per leg");
    assert_eq!(report.fills[0].side, OrderSide::Sell, "shorts sell to open");
    assert_close(report.fills[0].fill_price, 100.0, "entry");
    assert_close(report.fills[0].position_units_after, -10.0, "short 10");

    // Leg 0's target is 4.00 *below* a short's entry.
    assert_close(report.fills[1].fill_price, 96.0, "short target");
    assert_close(report.fills[1].realized_pnl, 20.0, "(100 - 96) * 5");

    // Extreme 93.00 → break-even to 100.00 first, then the trail rests 3.00
    // above the extreme at 96.00.
    let stops = strategy.stop_trace.clone();
    assert_eq!(stops[0].1, Some(102.0), "a short's stop sits above entry");
    assert_eq!(stops[3].2, Some(96.0), "93.00 extreme plus 3.00");
    assert_close(report.fills[2].fill_price, 96.0, "trailed stop");
    assert_close(report.fills[2].realized_pnl, 20.0, "(100 - 96) * 5");

    assert_close(report.final_realized_pnl, 40.0, "20.00 per leg");
    assert_close(report.final_cash, 100_037.0, "100 000 + 40 - 3");
}

/// A stop only ever moves in the protective direction. Once the trail has
/// ratcheted, a pullback that lowers the proposal must leave it alone.
///
/// Bars: enter long at 100.00; bar 2 runs to 110.00 so the trail proposes
/// 110 − 3 = 107.00; bar 3 only reaches 108.00, whose proposal of 105.00 is
/// looser and must be ignored.
#[test]
fn a_trailing_stop_never_loosens_on_a_pullback() {
    let stream = SymbolStream {
        symbol: "aaa".to_string(),
        bars: vec![
            bar(0, 100.0, 100.5, 99.5, 100.0),
            bar(1, 100.0, 101.0, 99.8, 100.8),
            bar(2, 100.8, 110.0, 100.5, 109.0),
            bar(3, 109.0, 109.0, 107.5, 108.0),
            bar(4, 108.0, 108.5, 107.5, 108.0),
        ],
    };
    let plan = ProtectivePlan {
        legs: vec![LegPlan {
            quantity: 5.0,
            stop_distance: Some(2.0),
            target_distance: None,
            trailing: Some(TrailingPlan {
                distance: 3.0,
                activate_after: None,
            }),
        }],
        break_even_after: None,
        max_bars_in_trade: None,
    };
    let mut strategy = ManagedEntry::new(0, OrderSide::Buy, 100.0, plan);
    let report = run(&[stream], &mut strategy);

    let stops = strategy.stop_trace.clone();
    assert_eq!(stops[1].0, 1, "decision 1 follows bar 1");
    // Bar 1's high of 101.00 proposes 98.00, which is exactly the initial stop.
    assert_eq!(stops[1].1, Some(98.0), "no movement yet");
    assert_eq!(stops[2].1, Some(107.0), "110.00 extreme less 3.00");
    assert_eq!(
        stops[3].1,
        Some(107.0),
        "bar 3's lower high must not loosen the trail"
    );
    assert_eq!(report.fills.len(), 1, "the trail is never reached");
}

/// A time stop closes whatever is left at market once the bar budget is spent.
///
/// Enter long 10 at bar 0 with `max_bars_in_trade: 2`. Decisions 1 and 2 count
/// as the two held bars, so decision 2 submits the market exit, which fills at
/// bar 3's open of 103.00.
///   sell 10 @ 103.00, fee 1.00 → realized (103 − 100) × 10 = 30.00
///   cash 98 999 + 1 030 − 1 = 100 028.00
#[test]
fn a_time_stop_closes_the_remainder_at_market() {
    let stream = SymbolStream {
        symbol: "aaa".to_string(),
        bars: vec![
            bar(0, 100.0, 100.5, 99.5, 100.0),
            bar(1, 100.0, 101.0, 99.8, 100.8),
            bar(2, 101.0, 102.0, 100.5, 101.5),
            bar(3, 103.0, 103.5, 102.5, 103.0),
            bar(4, 103.0, 103.5, 102.5, 103.0),
        ],
    };
    let plan = ProtectivePlan {
        legs: vec![LegPlan {
            quantity: 10.0,
            stop_distance: Some(5.0),
            target_distance: None,
            trailing: None,
        }],
        break_even_after: None,
        max_bars_in_trade: Some(2),
    };
    let mut strategy = ManagedEntry::new(0, OrderSide::Buy, 100.0, plan);
    let report = run(&[stream], &mut strategy);

    assert_eq!(report.fills.len(), 2, "entry plus the time stop");
    let exit = &report.fills[1];
    assert_eq!(exit.time_ns, 3 * MINUTE_NS, "fills at bar 3's open");
    assert_close(exit.quantity, 10.0, "the whole remainder");
    assert_close(exit.fill_price, 103.0, "a market exit takes the open");
    assert_close(exit.realized_pnl, 30.0, "(103 - 100) * 10");
    assert_close(exit.cash_after, 100_028.0, "100 000 + 30 - 2");
    assert_close(report.final_equity, 100_028.0, "flat, so equity is cash");
}

#[test]
fn an_unexecutable_plan_is_rejected_rather_than_approximated() {
    let leg = LegPlan {
        quantity: 5.0,
        stop_distance: Some(2.0),
        target_distance: None,
        trailing: None,
    };
    let plan = |legs: Vec<LegPlan>| ProtectivePlan {
        legs,
        break_even_after: None,
        max_bars_in_trade: None,
    };
    assert_eq!(plan(vec![]).validate(), Err(ProtectiveError::NoLegs));
    assert_eq!(
        plan(vec![leg; super::MAX_PROTECTIVE_LEGS + 1]).validate(),
        Err(ProtectiveError::TooManyLegs {
            limit: super::MAX_PROTECTIVE_LEGS,
            found: super::MAX_PROTECTIVE_LEGS + 1,
        })
    );
    assert_eq!(
        plan(vec![LegPlan {
            quantity: 0.0,
            ..leg
        }])
        .validate(),
        Err(ProtectiveError::InvalidQuantity)
    );
    // A zero-distance stop would sit exactly on the entry and fill or not by
    // tie-breaking, so it is refused rather than silently nudged.
    assert_eq!(
        plan(vec![LegPlan {
            stop_distance: Some(0.0),
            ..leg
        }])
        .validate(),
        Err(ProtectiveError::InvalidDistance)
    );
    assert_eq!(
        plan(vec![LegPlan {
            trailing: Some(TrailingPlan {
                distance: f64::NAN,
                activate_after: None,
            }),
            ..leg
        }])
        .validate(),
        Err(ProtectiveError::InvalidDistance)
    );
}

/// When a leg's own stop fills, its sibling target is retired by the
/// simulator's OCO group — the manager does not need to ask.
#[test]
fn a_legs_stop_and_target_retire_each_other_through_their_oco_group() {
    let stream = SymbolStream {
        symbol: "aaa".to_string(),
        bars: vec![
            bar(0, 100.0, 100.5, 99.5, 100.0),
            bar(1, 100.0, 101.0, 99.8, 100.8),
            bar(2, 100.5, 100.8, 97.0, 97.5),
            bar(3, 97.5, 98.0, 97.0, 97.5),
        ],
    };
    let plan = ProtectivePlan {
        legs: vec![LegPlan {
            quantity: 5.0,
            stop_distance: Some(2.0),
            target_distance: Some(20.0),
            trailing: None,
        }],
        break_even_after: None,
        max_bars_in_trade: None,
    };
    let mut strategy = ManagedEntry::new(0, OrderSide::Buy, 100.0, plan);
    let report = run(&[stream], &mut strategy);

    assert_eq!(report.fills.len(), 2, "entry plus the stop");
    assert_close(report.final_realized_pnl, -10.0, "(98 - 100) * 5");
    assert!(
        report.cancellations.iter().any(
            |record| record.reason == crate::core::strategy_simulator::CancelReason::OcoSibling
        ),
        "the far target is retired by its bracket partner"
    );
    assert!(
        !strategy.manager.is_active(),
        "a flat position retires the lifecycle"
    );
}

/// A strategy that closes the position itself — an IR exit condition firing
/// while the bracket still rests — leaves orders the OCO groups will never
/// retire, because nothing in those groups filled. The manager must withdraw
/// them, or they would fill against a later re-entry.
///
/// Enter long 5 at bar 0 with a stop 2.00 away and a target 20.00 away, then
/// close at market on decision 2. The manager sees flat at decision 3 and
/// cancels both survivors by request.
#[test]
fn a_strategy_exit_leaves_the_manager_to_cancel_the_resting_bracket() {
    let stream = SymbolStream {
        symbol: "aaa".to_string(),
        bars: vec![
            bar(0, 100.0, 100.5, 99.5, 100.0),
            bar(1, 100.0, 101.0, 99.8, 100.8),
            bar(2, 100.8, 101.5, 100.5, 101.0),
            bar(3, 101.0, 101.5, 100.5, 101.0),
            bar(4, 101.0, 101.5, 100.5, 101.0),
            bar(5, 101.0, 101.5, 100.5, 101.0),
        ],
    };
    let plan = ProtectivePlan {
        legs: vec![LegPlan {
            quantity: 5.0,
            stop_distance: Some(2.0),
            target_distance: Some(20.0),
            trailing: None,
        }],
        break_even_after: None,
        max_bars_in_trade: None,
    };
    let mut strategy = ManagedEntry::new(0, OrderSide::Buy, 100.0, plan).closing_at(2);
    let report = run(&[stream], &mut strategy);

    // Entry at bar 1's open (100.00) and the scripted exit at bar 3's open
    // (101.00): realized (101 - 100) * 5 = 5.00.
    assert_eq!(report.fills.len(), 2, "entry plus the strategy's own exit");
    assert_close(report.fills[1].fill_price, 101.0, "market exit at the open");
    assert_close(report.fills[1].realized_pnl, 5.0, "(101 - 100) * 5");

    let requested: Vec<_> = report
        .cancellations
        .iter()
        .filter(|record| record.reason == crate::core::strategy_simulator::CancelReason::Requested)
        .collect();
    assert_eq!(
        requested.len(),
        2,
        "both the stop and the target are withdrawn by request"
    );
    assert!(
        !strategy.manager.is_active(),
        "the lifecycle retires with the position"
    );
    assert!(
        report.pending_orders.is_empty(),
        "no protective order may outlive the position that justified it"
    );
}
