use typhoon_engine::broker::alpaca::AlpacaBroker;
use typhoon_engine::broker::protocol::{BrokerCmd, BrokerMsg};

pub async fn handle_alpaca_order_command(
    cmd: BrokerCmd,
    broker: Option<&AlpacaBroker>,
    broker_msg_tx: &tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
) {
    let Some(b) = broker else {
        return;
    };

    match cmd {
        BrokerCmd::CloseAll => match b.close_all_positions().await {
            Ok(_) => {
                let _ = broker_msg_tx.send(BrokerMsg::OrderResult("All positions closed".into()));
            }
            Err(e) => {
                let _ = broker_msg_tx.send(BrokerMsg::Error(e));
            }
        },
        BrokerCmd::ClosePosition { symbol, qty } => match b.close_position(&symbol, qty).await {
            Ok(r) => {
                let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                    "Closed {}: {}",
                    symbol, r.status
                )));
            }
            Err(e) => {
                let _ = broker_msg_tx.send(BrokerMsg::Error(e));
            }
        },
        BrokerCmd::AlpacaClosePositionPercent { symbol, percentage } => {
            match b.close_position_percent(&symbol, percentage).await {
                Ok(r) => {
                    let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                        "Closed {:.0}% of {}: {}",
                        percentage, symbol, r.status
                    )));
                }
                Err(e) => {
                    let _ = broker_msg_tx.send(BrokerMsg::Error(e));
                }
            }
        }
        BrokerCmd::AlpacaMarketOrder { symbol, qty, side } => {
            match b.market_order(&symbol, qty, &side).await {
                Ok(r) => {
                    let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                        "{} {} {} @ market: {}",
                        side, qty, symbol, r.status
                    )));
                }
                Err(e) => {
                    let _ = broker_msg_tx.send(BrokerMsg::Error(format!("Order failed: {}", e)));
                }
            }
        }
        BrokerCmd::AlpacaMarketOrderNotional {
            symbol,
            notional,
            side,
        } => match b.market_order_notional(&symbol, notional, &side).await {
            Ok(r) => {
                let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                    "{} ${:.2} {} @ market: {}",
                    side, notional, symbol, r.status
                )));
            }
            Err(e) => {
                let _ = broker_msg_tx.send(BrokerMsg::Error(format!("Order failed: {}", e)));
            }
        },
        BrokerCmd::AlpacaLimitOrder {
            symbol,
            qty,
            side,
            limit_price,
        } => match b.limit_order(&symbol, qty, &side, limit_price, "gtc").await {
            Ok(r) => {
                let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                    "{} {} {} limit {}: {}",
                    side, qty, symbol, limit_price, r.status
                )));
            }
            Err(e) => {
                let _ = broker_msg_tx.send(BrokerMsg::Error(format!("Order failed: {}", e)));
            }
        },
        BrokerCmd::AlpacaStopOrder {
            symbol,
            qty,
            side,
            stop_price,
        } => match b.stop_order(&symbol, qty, &side, stop_price, "gtc").await {
            Ok(r) => {
                let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                    "{} {} {} stop {}: {}",
                    side, qty, symbol, stop_price, r.status
                )));
            }
            Err(e) => {
                let _ = broker_msg_tx.send(BrokerMsg::Error(format!("Order failed: {}", e)));
            }
        },
        BrokerCmd::AlpacaBracketOrder {
            symbol,
            qty,
            side,
            stop_loss,
            take_profit,
        } => match b
            .bracket_order(&symbol, qty, &side, take_profit, stop_loss)
            .await
        {
            Ok(r) => {
                let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                    "Bracket {} {} {}: {}",
                    side, qty, symbol, r.status
                )));
            }
            Err(e) => {
                let _ =
                    broker_msg_tx.send(BrokerMsg::Error(format!("Bracket order failed: {}", e)));
            }
        },
        BrokerCmd::AlpacaCancelOrder { order_id } => match b.cancel_order(&order_id).await {
            Ok(_) => {
                let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                    "Order {} cancelled",
                    order_id
                )));
            }
            Err(e) => {
                let _ = broker_msg_tx.send(BrokerMsg::Error(format!("Cancel failed: {}", e)));
            }
        },
        BrokerCmd::AlpacaOcoOrder {
            symbol,
            qty,
            side,
            tp_price,
            sl_price,
        } => match b
            .oco_order(&symbol, qty, &side, tp_price, sl_price, None)
            .await
        {
            Ok(r) => {
                let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                    "OCO {} {} {} @ TP:{} SL:{}: {}",
                    side, qty, symbol, tp_price, sl_price, r.status
                )));
            }
            Err(e) => {
                let _ = broker_msg_tx.send(BrokerMsg::Error(format!("OCO failed: {}", e)));
            }
        },
        BrokerCmd::AlpacaModifyOrder {
            order_id,
            qty,
            limit_price,
            stop_price,
        } => match b
            .modify_order(&order_id, qty, limit_price, stop_price, None)
            .await
        {
            Ok(r) => {
                let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                    "Order {} modified: {}",
                    order_id, r.status
                )));
            }
            Err(e) => {
                let _ = broker_msg_tx.send(BrokerMsg::Error(format!("Modify failed: {}", e)));
            }
        },
        BrokerCmd::AlpacaTrailingStop {
            symbol,
            qty,
            side,
            trail_percent,
        } => match b
            .trailing_stop_order(&symbol, qty, &side, None, Some(trail_percent), "gtc")
            .await
        {
            Ok(r) => {
                let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                    "Trailing stop {} {} {} trail {}%: {}",
                    side, qty, symbol, trail_percent, r.status
                )));
            }
            Err(e) => {
                let _ =
                    broker_msg_tx.send(BrokerMsg::Error(format!("Trailing stop failed: {}", e)));
            }
        },
        BrokerCmd::AlpacaStopLimitOrder {
            symbol,
            qty,
            side,
            stop_price,
            limit_price,
        } => match b
            .stop_limit_order(&symbol, qty, &side, stop_price, limit_price, "gtc")
            .await
        {
            Ok(r) => {
                let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                    "Stop-limit {} {} {} stop={} lim={}: {}",
                    side, qty, symbol, stop_price, limit_price, r.status
                )));
            }
            Err(e) => {
                let _ = broker_msg_tx.send(BrokerMsg::Error(format!("Stop-limit failed: {}", e)));
            }
        },
        BrokerCmd::AlpacaSyncExits {
            symbol,
            sl_price,
            tp_price,
            wait_for_qty_at_most,
        } => {
            if let Some(max_qty) = wait_for_qty_at_most {
                let mut ready = false;
                for _ in 0..12 {
                    match b.get_positions().await {
                        Ok(positions) => {
                            if positions.iter().any(|p| {
                                p.symbol.eq_ignore_ascii_case(&symbol)
                                    && p.qty.abs() > 0.0
                                    && p.qty.abs() <= max_qty + 1e-8
                            }) {
                                ready = true;
                                break;
                            }
                        }
                        Err(e) => {
                            let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
                                "Alpaca exit sync {}: position poll failed: {}",
                                symbol, e
                            )));
                            break;
                        }
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
                }
                if !ready {
                    let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
                        "Alpaca exit sync {}: reduced position not visible yet",
                        symbol
                    )));
                    return;
                }
            }
            match b.sync_position_exits(&symbol, sl_price, tp_price).await {
                Ok(summary) => {
                    let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                        "Alpaca exits {}: {}",
                        symbol, summary
                    )));
                }
                Err(e) => {
                    let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
                        "Alpaca exit sync failed for {}: {}",
                        symbol, e
                    )));
                }
            }
        }
        _ => {}
    }
}
