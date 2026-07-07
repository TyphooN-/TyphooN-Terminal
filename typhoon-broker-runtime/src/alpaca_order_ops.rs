use std::collections::BTreeMap;

use typhoon_engine::broker::alpaca::AlpacaBroker;
use typhoon_engine::broker::protocol::{BrokerCmd, BrokerMsg};

use crate::account_pool::AlpacaAccountPool;

/// Order routes for one command: always the primary account, plus every other
/// trade-enabled account when live order mirroring (TradeCopy) is on. The tag
/// is appended to result lines so mirrored placements are attributable.
fn order_routes<'a>(pool: &'a AlpacaAccountPool, mirror: bool) -> Vec<(String, &'a AlpacaBroker)> {
    let mut routes: Vec<(String, &AlpacaBroker)> = Vec::new();
    if let Some(primary) = pool.primary_broker() {
        routes.push((String::new(), primary));
    }
    if mirror {
        for (spec, broker) in pool.mirror_targets() {
            routes.push((format!(" [mirror → {}]", spec.label), broker));
        }
    }
    routes
}

pub async fn handle_alpaca_order_command(
    cmd: BrokerCmd,
    pool: &AlpacaAccountPool,
    broker_msg_tx: &tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
) {
    if pool.primary_broker().is_none() {
        return;
    }
    let mirror = pool.mirror_orders();

    match cmd {
        BrokerCmd::CloseAll => {
            for (tag, b) in order_routes(pool, mirror) {
                match b.close_all_positions().await {
                    Ok(_) => {
                        let _ = broker_msg_tx
                            .send(BrokerMsg::OrderResult(format!("All positions closed{tag}")));
                    }
                    Err(e) => {
                        let _ = broker_msg_tx.send(BrokerMsg::Error(format!("{e}{tag}")));
                    }
                }
            }
        }
        BrokerCmd::ClosePosition { symbol, qty } => {
            for (tag, b) in order_routes(pool, mirror) {
                match b.close_position(&symbol, qty).await {
                    Ok(r) => {
                        let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                            "Closed {}: {}{tag}",
                            symbol, r.status
                        )));
                    }
                    Err(e) => {
                        let _ = broker_msg_tx.send(BrokerMsg::Error(format!("{e}{tag}")));
                    }
                }
            }
        }
        BrokerCmd::ClosePositionForAccount {
            account_id,
            symbol,
            qty,
        } => {
            if let Some(account) = pool.broker_by_id(&account_id) {
                let tag = format!(" [{}]", account.spec.label);
                match account.broker.close_position(&symbol, qty).await {
                    Ok(r) => {
                        let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                            "Closed {}: {}{tag}",
                            symbol, r.status
                        )));
                    }
                    Err(e) => {
                        let _ = broker_msg_tx.send(BrokerMsg::Error(format!("{e}{tag}")));
                    }
                }
            } else {
                let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
                    "Alpaca close failed: account {account_id} is not connected"
                )));
            }
        }
        BrokerCmd::AlpacaClosePositionPercent { symbol, percentage } => {
            for (tag, b) in order_routes(pool, mirror) {
                match b.close_position_percent(&symbol, percentage).await {
                    Ok(r) => {
                        let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                            "Closed {:.0}% of {}: {}{tag}",
                            percentage, symbol, r.status
                        )));
                    }
                    Err(e) => {
                        let _ = broker_msg_tx.send(BrokerMsg::Error(format!("{e}{tag}")));
                    }
                }
            }
        }
        BrokerCmd::AlpacaClosePositionPercentForAccount {
            account_id,
            symbol,
            percentage,
        } => {
            if let Some(account) = pool.broker_by_id(&account_id) {
                let tag = format!(" [{}]", account.spec.label);
                match account
                    .broker
                    .close_position_percent(&symbol, percentage)
                    .await
                {
                    Ok(r) => {
                        let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                            "Closed {:.0}% of {}: {}{tag}",
                            percentage, symbol, r.status
                        )));
                    }
                    Err(e) => {
                        let _ = broker_msg_tx.send(BrokerMsg::Error(format!("{e}{tag}")));
                    }
                }
            } else {
                let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
                    "Alpaca close failed: account {account_id} is not connected"
                )));
            }
        }
        BrokerCmd::AlpacaMarketOrder { symbol, qty, side } => {
            for (tag, b) in order_routes(pool, mirror) {
                match b.market_order(&symbol, qty, &side).await {
                    Ok(r) => {
                        let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                            "{} {} {} @ market: {}{tag}",
                            side, qty, symbol, r.status
                        )));
                    }
                    Err(e) => {
                        let _ = broker_msg_tx
                            .send(BrokerMsg::Error(format!("Order failed: {}{tag}", e)));
                    }
                }
            }
        }
        BrokerCmd::AlpacaMarketOrderNotional {
            symbol,
            notional,
            side,
        } => {
            for (tag, b) in order_routes(pool, mirror) {
                match b.market_order_notional(&symbol, notional, &side).await {
                    Ok(r) => {
                        let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                            "{} ${:.2} {} @ market: {}{tag}",
                            side, notional, symbol, r.status
                        )));
                    }
                    Err(e) => {
                        let _ = broker_msg_tx
                            .send(BrokerMsg::Error(format!("Order failed: {}{tag}", e)));
                    }
                }
            }
        }
        BrokerCmd::AlpacaLimitOrder {
            symbol,
            qty,
            side,
            limit_price,
        } => {
            for (tag, b) in order_routes(pool, mirror) {
                match b.limit_order(&symbol, qty, &side, limit_price, "gtc").await {
                    Ok(r) => {
                        let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                            "{} {} {} limit {}: {}{tag}",
                            side, qty, symbol, limit_price, r.status
                        )));
                    }
                    Err(e) => {
                        let _ = broker_msg_tx
                            .send(BrokerMsg::Error(format!("Order failed: {}{tag}", e)));
                    }
                }
            }
        }
        BrokerCmd::AlpacaStopOrder {
            symbol,
            qty,
            side,
            stop_price,
        } => {
            for (tag, b) in order_routes(pool, mirror) {
                match b.stop_order(&symbol, qty, &side, stop_price, "gtc").await {
                    Ok(r) => {
                        let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                            "{} {} {} stop {}: {}{tag}",
                            side, qty, symbol, stop_price, r.status
                        )));
                    }
                    Err(e) => {
                        let _ = broker_msg_tx
                            .send(BrokerMsg::Error(format!("Order failed: {}{tag}", e)));
                    }
                }
            }
        }
        BrokerCmd::AlpacaBracketOrder {
            symbol,
            qty,
            side,
            stop_loss,
            take_profit,
        } => {
            for (tag, b) in order_routes(pool, mirror) {
                match b
                    .bracket_order(&symbol, qty, &side, take_profit, stop_loss)
                    .await
                {
                    Ok(r) => {
                        let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                            "Bracket {} {} {}: {}{tag}",
                            side, qty, symbol, r.status
                        )));
                    }
                    Err(e) => {
                        let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
                            "Bracket order failed: {}{tag}",
                            e
                        )));
                    }
                }
            }
        }
        BrokerCmd::AlpacaCancelOrder { order_id } => {
            // Order ids are account-specific — never mirrored.
            if let Some(b) = pool.primary_broker() {
                match b.cancel_order(&order_id).await {
                    Ok(_) => {
                        let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                            "Order {} cancelled",
                            order_id
                        )));
                        if mirror {
                            let _ = broker_msg_tx.send(BrokerMsg::OrderResult(
                                "mirror: cancel not replicated (order ids are account-specific)"
                                    .into(),
                            ));
                        }
                    }
                    Err(e) => {
                        let _ =
                            broker_msg_tx.send(BrokerMsg::Error(format!("Cancel failed: {}", e)));
                    }
                }
            }
        }
        BrokerCmd::AlpacaOcoOrder {
            symbol,
            qty,
            side,
            tp_price,
            sl_price,
        } => {
            for (tag, b) in order_routes(pool, mirror) {
                match b
                    .oco_order(&symbol, qty, &side, tp_price, sl_price, None)
                    .await
                {
                    Ok(r) => {
                        let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                            "OCO {} {} {} @ TP:{} SL:{}: {}{tag}",
                            side, qty, symbol, tp_price, sl_price, r.status
                        )));
                    }
                    Err(e) => {
                        let _ =
                            broker_msg_tx.send(BrokerMsg::Error(format!("OCO failed: {}{tag}", e)));
                    }
                }
            }
        }
        BrokerCmd::AlpacaModifyOrder {
            order_id,
            qty,
            limit_price,
            stop_price,
        } => {
            // Order ids are account-specific — never mirrored.
            if let Some(b) = pool.primary_broker() {
                match b
                    .modify_order(&order_id, qty, limit_price, stop_price, None)
                    .await
                {
                    Ok(r) => {
                        let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                            "Order {} modified: {}",
                            order_id, r.status
                        )));
                        if mirror {
                            let _ = broker_msg_tx.send(BrokerMsg::OrderResult(
                                "mirror: modify not replicated (order ids are account-specific)"
                                    .into(),
                            ));
                        }
                    }
                    Err(e) => {
                        let _ =
                            broker_msg_tx.send(BrokerMsg::Error(format!("Modify failed: {}", e)));
                    }
                }
            }
        }
        BrokerCmd::AlpacaTrailingStop {
            symbol,
            qty,
            side,
            trail_percent,
        } => {
            for (tag, b) in order_routes(pool, mirror) {
                match b
                    .trailing_stop_order(&symbol, qty, &side, None, Some(trail_percent), "gtc")
                    .await
                {
                    Ok(r) => {
                        let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                            "Trailing stop {} {} {} trail {}%: {}{tag}",
                            side, qty, symbol, trail_percent, r.status
                        )));
                    }
                    Err(e) => {
                        let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
                            "Trailing stop failed: {}{tag}",
                            e
                        )));
                    }
                }
            }
        }
        BrokerCmd::AlpacaStopLimitOrder {
            symbol,
            qty,
            side,
            stop_price,
            limit_price,
        } => {
            for (tag, b) in order_routes(pool, mirror) {
                match b
                    .stop_limit_order(&symbol, qty, &side, stop_price, limit_price, "gtc")
                    .await
                {
                    Ok(r) => {
                        let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                            "Stop-limit {} {} {} stop={} lim={}: {}{tag}",
                            side, qty, symbol, stop_price, limit_price, r.status
                        )));
                    }
                    Err(e) => {
                        let _ = broker_msg_tx
                            .send(BrokerMsg::Error(format!("Stop-limit failed: {}{tag}", e)));
                    }
                }
            }
        }
        BrokerCmd::AlpacaSyncExits {
            symbol,
            sl_price,
            tp_price,
            wait_for_qty_at_most,
        } => {
            for (tag, b) in order_routes(pool, mirror) {
                if let Some(max_qty) = wait_for_qty_at_most {
                    let mut ready = false;
                    for _ in 0..12 {
                        match b.get_positions().await {
                            Ok(positions) => {
                                let has_sym = positions.iter().any(|p| p.symbol.eq_ignore_ascii_case(&symbol) && p.qty.abs() > 0.0);
                                if has_sym && positions.iter().any(|p| p.symbol.eq_ignore_ascii_case(&symbol) && p.qty.abs() <= max_qty + 1e-8) {
                                    ready = true;
                                    break;
                                }
                            }
                            Err(e) => {
                                let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
                                    "Alpaca exit sync {}: position poll failed: {}{tag}",
                                    symbol, e
                                )));
                                break;
                            }
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
                    }
                    if !ready {
                        let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
                            "Alpaca exit sync {}: reduced position not visible yet{tag}",
                            symbol
                        )));
                        continue;
                    }
                }
                match b.sync_position_exits(&symbol, sl_price, tp_price).await {
                    Ok(summary) => {
                        let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                            "Alpaca exits {}: {}{tag}",
                            symbol, summary
                        )));
                    }
                    Err(e) => {
                        let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
                            "Alpaca exit sync failed for {}: {}{tag}",
                            symbol, e
                        )));
                    }
                }
            }
        }
        _ => {}
    }
}

/// Signed position quantity per symbol (short → negative).
fn signed_position_map(
    positions: &[typhoon_engine::broker::alpaca::PositionInfo],
) -> BTreeMap<String, f64> {
    let mut map = BTreeMap::new();
    for p in positions {
        let qty = if p.side.eq_ignore_ascii_case("short") {
            -p.qty.abs()
        } else {
            p.qty
        };
        if qty.abs() > 1e-9 {
            *map.entry(p.symbol.to_ascii_uppercase()).or_insert(0.0) += qty;
        }
    }
    map
}

/// Per-symbol market-order deltas that bring `target` in line with `source`.
/// `flatten_extra` also unwinds target positions the source does not hold.
/// Returned as (symbol, side, qty) — pure so it is unit-testable.
pub(crate) fn trade_copy_deltas(
    source: &BTreeMap<String, f64>,
    target: &BTreeMap<String, f64>,
    flatten_extra: bool,
) -> Vec<(String, String, f64)> {
    let mut out = Vec::new();
    for (symbol, &src_qty) in source {
        let tgt_qty = target.get(symbol).copied().unwrap_or(0.0);
        let delta = src_qty - tgt_qty;
        if delta.abs() < 1e-9 {
            continue;
        }
        let side = if delta > 0.0 { "buy" } else { "sell" };
        out.push((symbol.clone(), side.to_string(), delta.abs()));
    }
    if flatten_extra {
        for (symbol, &tgt_qty) in target {
            if source.contains_key(symbol) || tgt_qty.abs() < 1e-9 {
                continue;
            }
            let side = if tgt_qty > 0.0 { "sell" } else { "buy" };
            out.push((symbol.clone(), side.to_string(), tgt_qty.abs()));
        }
    }
    out
}

/// One-shot TradeCopy: replicate the source account's open positions onto each
/// target account with market orders for the per-symbol deltas (ADR-130).
pub async fn handle_alpaca_trade_copy(
    source_id: String,
    target_ids: Vec<String>,
    flatten_extra: bool,
    pool: &AlpacaAccountPool,
    broker_msg_tx: &tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
) {
    let Some(source) = pool.broker_by_id(&source_id) else {
        let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
            "TradeCopy: source account '{}' is not connected",
            source_id
        )));
        return;
    };
    let source_positions = match source.broker.get_positions().await {
        Ok(p) => p,
        Err(e) => {
            let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
                "TradeCopy: source positions failed: {}",
                e
            )));
            return;
        }
    };
    let source_map = signed_position_map(&source_positions);
    let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
        "TradeCopy: source {} holds {} position(s)",
        source.spec.label,
        source_map.len()
    )));

    for target_id in target_ids {
        if target_id == source_id {
            continue;
        }
        let Some(target) = pool.broker_by_id(&target_id) else {
            let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
                "TradeCopy: target '{}' is not connected — skipped",
                target_id
            )));
            continue;
        };
        if !target.spec.trade_enabled {
            let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
                "TradeCopy: target '{}' is not trade-enabled — skipped",
                target.spec.label
            )));
            continue;
        }
        let target_map = match target.broker.get_positions().await {
            Ok(p) => signed_position_map(&p),
            Err(e) => {
                let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
                    "TradeCopy: {} positions failed: {} — skipped",
                    target.spec.label, e
                )));
                continue;
            }
        };
        let deltas = trade_copy_deltas(&source_map, &target_map, flatten_extra);
        if deltas.is_empty() {
            let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                "TradeCopy → {}: already in sync",
                target.spec.label
            )));
            continue;
        }
        let mut placed = 0usize;
        let mut failed = 0usize;
        for (symbol, side, qty) in deltas {
            match target.broker.market_order(&symbol, qty, &side).await {
                Ok(r) => {
                    placed += 1;
                    let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                        "TradeCopy → {}: {} {} {} @ market: {}",
                        target.spec.label, side, qty, symbol, r.status
                    )));
                }
                Err(e) => {
                    failed += 1;
                    let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
                        "TradeCopy → {}: {} {} {} failed: {}",
                        target.spec.label, side, qty, symbol, e
                    )));
                }
            }
        }
        let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
            "TradeCopy → {}: {} order(s) placed, {} failed",
            target.spec.label, placed, failed
        )));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(entries: &[(&str, f64)]) -> BTreeMap<String, f64> {
        entries.iter().map(|(s, q)| (s.to_string(), *q)).collect()
    }

    #[test]
    fn trade_copy_deltas_buy_sell_and_flatten() {
        let source = map(&[("AAPL", 10.0), ("AMC", 52631.0), ("TSLA", -5.0)]);
        let target = map(&[("AAPL", 4.0), ("MSFT", 7.0), ("TSLA", -5.0)]);

        let deltas = trade_copy_deltas(&source, &target, false);
        assert_eq!(
            deltas,
            vec![
                ("AAPL".to_string(), "buy".to_string(), 6.0),
                ("AMC".to_string(), "buy".to_string(), 52631.0),
            ]
        );

        let with_flatten = trade_copy_deltas(&source, &target, true);
        assert!(with_flatten.contains(&("MSFT".to_string(), "sell".to_string(), 7.0)));
    }

    #[test]
    fn trade_copy_deltas_reverse_direction_sells_down() {
        let source = map(&[("WEN", 100.0)]);
        let target = map(&[("WEN", 250.0)]);
        assert_eq!(
            trade_copy_deltas(&source, &target, false),
            vec![("WEN".to_string(), "sell".to_string(), 150.0)]
        );
    }

    #[test]
    fn signed_position_map_negates_shorts() {
        let positions = vec![
            typhoon_engine::broker::alpaca::PositionInfo {
                symbol: "TSLA".into(),
                qty: 5.0,
                qty_available: 5.0,
                side: "short".into(),
                avg_entry_price: 100.0,
                market_value: -500.0,
                unrealized_pl: 0.0,
                asset_class: "us_equity".into(),
                asset_id: String::new(),
            },
            typhoon_engine::broker::alpaca::PositionInfo {
                symbol: "AAPL".into(),
                qty: 3.0,
                qty_available: 3.0,
                side: "long".into(),
                avg_entry_price: 100.0,
                market_value: 300.0,
                unrealized_pl: 0.0,
                asset_class: "us_equity".into(),
                asset_id: String::new(),
            },
        ];
        let map = signed_position_map(&positions);
        assert_eq!(map.get("TSLA"), Some(&-5.0));
        assert_eq!(map.get("AAPL"), Some(&3.0));
    }
}
