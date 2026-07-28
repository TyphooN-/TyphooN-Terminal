//! Two-leg protective order lifecycle (ADR-135 §10.3).
//!
//! The NNFX template is: enter once, split the position into legs, give each leg
//! its own stop and target, move the runner to break-even when the first leg
//! banks its target, then trail the runner. This module owns that state machine
//! and nothing else — it computes no fills and keeps no accounting. It emits
//! order intents and reads back the position the simulator actually holds.
//!
//! # Why this is expressible now
//!
//! Two simulator facts make a real bracket possible rather than approximate:
//!
//! 1. Orders execute within a bar in submission order
//!    (`candidates.sort_by_key(submit_sequence)`), so an entry submitted before
//!    its protective siblings fills first and the stops are live for the very
//!    bar that opened the trade.
//! 2. `reduce_only` is enforced when an order executes, not when it is
//!    submitted, so protective orders may be submitted alongside an entry that
//!    has not filled yet without being rejected as "would not reduce".
//!
//! # Distances, not rules
//!
//! Callers pass resolved *price distances*. Turning an
//! [`crate::core::strategy_ir::StopRule`] into a distance needs indicator state
//! that lives in the interpreter, so that resolution stays there and this module
//! stays a pure lifecycle.

use crate::core::strategy_simulator::{
    ClientOrderId, DecisionContext, OrderIntents, OrderRequest, OrderSide, StrategyError, SymbolId,
};

/// Upper bound on legs in one plan. The NNFX template uses two; the bound keeps
/// a malformed plan from generating unbounded resting orders.
pub const MAX_PROTECTIVE_LEGS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrailingPlan {
    /// Distance held behind the favourable extreme, in price units.
    pub distance: f64,
    /// Favourable movement required before the trail engages. `None` trails
    /// from the entry bar.
    pub activate_after: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LegPlan {
    pub quantity: f64,
    /// Initial protective stop distance from the entry price.
    pub stop_distance: Option<f64>,
    /// Profit target distance from the entry price.
    pub target_distance: Option<f64>,
    pub trailing: Option<TrailingPlan>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProtectivePlan {
    pub legs: Vec<LegPlan>,
    /// Favourable movement after which every surviving leg's stop moves to the
    /// entry price.
    pub break_even_after: Option<f64>,
    /// Committed bars after which the remainder is closed at market.
    pub max_bars_in_trade: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtectiveError {
    NoLegs,
    TooManyLegs { limit: usize, found: usize },
    InvalidQuantity,
    InvalidDistance,
}

impl std::fmt::Display for ProtectiveError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "invalid protective plan: {self:?}")
    }
}

impl std::error::Error for ProtectiveError {}

impl ProtectivePlan {
    /// Rejects a plan that could not be executed as written. Distances are
    /// strictly positive: a zero-distance stop would sit exactly on the entry
    /// and fill or not depending on tie-breaking, which is not a contract worth
    /// having.
    pub fn validate(&self) -> Result<(), ProtectiveError> {
        if self.legs.is_empty() {
            return Err(ProtectiveError::NoLegs);
        }
        if self.legs.len() > MAX_PROTECTIVE_LEGS {
            return Err(ProtectiveError::TooManyLegs {
                limit: MAX_PROTECTIVE_LEGS,
                found: self.legs.len(),
            });
        }
        for leg in &self.legs {
            if !leg.quantity.is_finite() || leg.quantity <= 0.0 {
                return Err(ProtectiveError::InvalidQuantity);
            }
            for distance in [leg.stop_distance, leg.target_distance]
                .into_iter()
                .flatten()
                .chain(leg.trailing.map(|trail| trail.distance))
                .chain(leg.trailing.and_then(|trail| trail.activate_after))
            {
                if !distance.is_finite() || distance <= 0.0 {
                    return Err(ProtectiveError::InvalidDistance);
                }
            }
        }
        if let Some(distance) = self.break_even_after
            && (!distance.is_finite() || distance <= 0.0)
        {
            return Err(ProtectiveError::InvalidDistance);
        }
        Ok(())
    }

    pub fn total_quantity(&self) -> f64 {
        self.legs.iter().map(|leg| leg.quantity).sum()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct LiveLeg {
    quantity: f64,
    stop: Option<ClientOrderId>,
    target: Option<ClientOrderId>,
    /// Current stop price, tracked so break-even and trailing only ever move it
    /// in the protective direction.
    stop_price: Option<f64>,
    trailing: Option<TrailingPlan>,
}

/// Which way a stop may be moved. A protective stop that can loosen is not a
/// protective stop, so every adjustment goes through this check.
fn tightens(side: OrderSide, current: f64, proposed: f64) -> bool {
    match side {
        OrderSide::Buy => proposed > current,
        OrderSide::Sell => proposed < current,
    }
}

#[derive(Debug, Clone, PartialEq)]
struct ActiveTrade {
    symbol: SymbolId,
    /// Direction of the *position*, not of the protective orders.
    side: OrderSide,
    entry_price: f64,
    legs: Vec<LiveLeg>,
    break_even_after: Option<f64>,
    break_even_done: bool,
    max_bars_in_trade: Option<u32>,
    bars_held: u32,
    time_stop_sent: bool,
}

/// Drives one symbol's protective lifecycle.
///
/// The manager is deliberately not a `ReferenceStrategy`: entries are the
/// strategy's decision, and this only manages what happens after one.
#[derive(Debug, Clone, Default)]
pub struct ProtectiveManager {
    active: Option<ActiveTrade>,
    next_oco_group: u32,
}

impl ProtectiveManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_active(&self) -> bool {
        self.active.is_some()
    }

    /// Committed bars the current trade has been held for, or zero when flat.
    pub fn bars_held(&self) -> u32 {
        self.active.as_ref().map_or(0, |trade| trade.bars_held)
    }

    /// Stop price currently resting for `leg`, if that leg still has one.
    pub fn leg_stop_price(&self, leg: usize) -> Option<f64> {
        self.active
            .as_ref()
            .and_then(|trade| trade.legs.get(leg))
            .and_then(|leg| leg.stop_price)
    }

    /// Submits the entry and every protective order for `plan`.
    ///
    /// `reference_price` is the price the distances are measured from — the
    /// strategy's expected entry. The real fill may differ by spread, slippage
    /// or a gap; protective levels stay anchored to the reference so the
    /// risk the strategy authored is the risk it gets, and the difference shows
    /// up as execution cost rather than silently resizing the stop.
    pub fn enter(
        &mut self,
        symbol: SymbolId,
        side: OrderSide,
        reference_price: f64,
        plan: &ProtectivePlan,
        orders: &mut OrderIntents,
    ) -> Result<(), StrategyError> {
        plan.validate().map_err(|error| StrategyError::Rejected {
            reason: error.to_string(),
        })?;
        let exit_side = match side {
            OrderSide::Buy => OrderSide::Sell,
            OrderSide::Sell => OrderSide::Buy,
        };
        let signed = |distance: f64| match side {
            OrderSide::Buy => reference_price + distance,
            OrderSide::Sell => reference_price - distance,
        };
        let adverse = |distance: f64| match side {
            OrderSide::Buy => reference_price - distance,
            OrderSide::Sell => reference_price + distance,
        };

        // The entry goes first so it holds the lowest submit sequence and fills
        // ahead of its own protection within the entry bar.
        orders.market(symbol, side, plan.total_quantity())?;

        let mut legs = Vec::with_capacity(plan.legs.len());
        for leg in &plan.legs {
            // One OCO group per leg: a leg's stop and target cancel each other,
            // and never a sibling leg's protection.
            let group = self.next_oco_group;
            self.next_oco_group = self.next_oco_group.wrapping_add(1);
            let stop_price = leg.stop_distance.map(adverse);
            let stop = match stop_price {
                Some(price) => Some(
                    orders.submit(
                        OrderRequest::stop(symbol, exit_side, leg.quantity, price)
                            .reduce_only()
                            .with_oco(group),
                    )?,
                ),
                None => None,
            };
            let target = match leg.target_distance {
                Some(distance) => Some(
                    orders.submit(
                        OrderRequest::limit(symbol, exit_side, leg.quantity, signed(distance))
                            .reduce_only()
                            .with_oco(group),
                    )?,
                ),
                None => None,
            };
            legs.push(LiveLeg {
                quantity: leg.quantity,
                stop,
                target,
                stop_price,
                trailing: leg.trailing,
            });
        }

        self.active = Some(ActiveTrade {
            symbol,
            side,
            entry_price: reference_price,
            legs,
            break_even_after: plan.break_even_after,
            break_even_done: false,
            max_bars_in_trade: plan.max_bars_in_trade,
            bars_held: 0,
            time_stop_sent: false,
        });
        Ok(())
    }

    /// Advances the lifecycle by one decision: reconciles against the position
    /// and order book the simulator actually holds, then applies break-even,
    /// trailing and the time stop. Returns `true` when the time stop submitted
    /// an exit, so the owning strategy does not submit a competing exit.
    pub fn on_decision(
        &mut self,
        ctx: &DecisionContext<'_>,
        orders: &mut OrderIntents,
    ) -> Result<bool, StrategyError> {
        let Some(trade) = self.active.as_mut() else {
            return Ok(false);
        };
        reconcile_orders(trade, ctx);
        let position = ctx.position(trade.symbol);
        // Protective orders exit without telling the strategy. A flat position
        // is the authoritative signal that every leg is done; cancel whatever
        // is still resting so a later re-entry cannot inherit stale protection.
        if position.is_flat() {
            cancel_resting(trade, ctx, orders)?;
            self.active = None;
            return Ok(false);
        }
        trade.bars_held = trade.bars_held.saturating_add(1);

        if trade
            .max_bars_in_trade
            .is_some_and(|limit| trade.bars_held >= limit)
            && !trade.time_stop_sent
        {
            trade.time_stop_sent = true;
            let exit_side = match trade.side {
                OrderSide::Buy => OrderSide::Sell,
                OrderSide::Sell => OrderSide::Buy,
            };
            cancel_resting(trade, ctx, orders)?;
            orders.submit(
                OrderRequest::market(trade.symbol, exit_side, position.units.abs()).reduce_only(),
            )?;
            self.active = None;
            return Ok(true);
        }

        let favorable = match trade.side {
            OrderSide::Buy => position.favorable_extreme - trade.entry_price,
            OrderSide::Sell => trade.entry_price - position.favorable_extreme,
        };

        // Break-even first: it is a one-shot floor, and a trail computed in the
        // same decision may already be tighter than the entry.
        if !trade.break_even_done
            && trade
                .break_even_after
                .is_some_and(|distance| favorable >= distance)
        {
            trade.break_even_done = true;
            for leg in &mut trade.legs {
                move_stop(trade.side, leg, trade.entry_price, orders)?;
            }
        }

        for leg in &mut trade.legs {
            let Some(trailing) = leg.trailing else {
                continue;
            };
            if trailing
                .activate_after
                .is_some_and(|threshold| favorable < threshold)
            {
                continue;
            }
            let proposed = match trade.side {
                OrderSide::Buy => position.favorable_extreme - trailing.distance,
                OrderSide::Sell => position.favorable_extreme + trailing.distance,
            };
            move_stop(trade.side, leg, proposed, orders)?;
        }
        Ok(false)
    }

    /// Retires every still-live protective order when the owning strategy
    /// submits its own exit. Cancellation is emitted on the same decision as
    /// the exit, so no bracket can survive until a later flat-state poll.
    pub fn retire(
        &mut self,
        ctx: &DecisionContext<'_>,
        orders: &mut OrderIntents,
    ) -> Result<(), StrategyError> {
        let Some(mut trade) = self.active.take() else {
            return Ok(());
        };
        reconcile_orders(&mut trade, ctx);
        cancel_resting(&trade, ctx, orders)
    }

    /// Abandons the lifecycle without emitting orders. For a strategy that
    /// closes a position itself and has already cancelled the brackets.
    pub fn forget(&mut self) {
        self.active = None;
    }
}

fn reconcile_orders(trade: &mut ActiveTrade, ctx: &DecisionContext<'_>) {
    for leg in &mut trade.legs {
        if leg.stop.is_some_and(|order| !ctx.is_order_live(order)) {
            leg.stop = None;
            leg.stop_price = None;
        }
        if leg.target.is_some_and(|order| !ctx.is_order_live(order)) {
            leg.target = None;
        }
    }
}

fn cancel_resting(
    trade: &ActiveTrade,
    ctx: &DecisionContext<'_>,
    orders: &mut OrderIntents,
) -> Result<(), StrategyError> {
    for leg in &trade.legs {
        for order in [leg.stop, leg.target].into_iter().flatten() {
            if ctx.is_order_live(order) {
                orders.cancel(order)?;
            }
        }
    }
    Ok(())
}

/// Moves one leg's resting stop, but only ever tighter.
fn move_stop(
    side: OrderSide,
    leg: &mut LiveLeg,
    proposed: f64,
    orders: &mut OrderIntents,
) -> Result<(), StrategyError> {
    if !proposed.is_finite() || proposed <= 0.0 {
        return Ok(());
    }
    let Some(order) = leg.stop else {
        return Ok(());
    };
    match leg.stop_price {
        Some(current) if !tightens(side, current, proposed) => return Ok(()),
        _ => {}
    }
    leg.stop_price = Some(proposed);
    orders.modify(
        order,
        crate::core::strategy_simulator::ModifyRequest::stop_price(proposed),
    )
}

#[cfg(test)]
mod tests;
