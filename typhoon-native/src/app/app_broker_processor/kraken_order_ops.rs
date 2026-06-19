use super::*;

pub(super) async fn handle_kraken_order_command(
    cmd: BrokerCmd,
    kraken_broker: Option<&typhoon_engine::broker::kraken::KrakenBroker>,
    broker_msg_tx: &tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
) {
    let Some(kb) = kraken_broker else {
        let _ = broker_msg_tx.send(BrokerMsg::Error("Kraken: not connected".into()));
        return;
    };

    match cmd {
        BrokerCmd::KrakenSyncExits {
            pair,
            sl_price,
            tp_price,
            wait_for_position,
            wait_for_qty_at_most,
        } => {
            if wait_for_position || wait_for_qty_at_most.is_some() {
                let mut found = false;
                for _ in 0..12 {
                    match kb.get_position_summaries().await {
                        Ok(positions)
                            if positions.iter().any(|p| {
                                p.symbol.eq_ignore_ascii_case(&pair)
                                    && p.qty.abs() > 0.0
                                    && wait_for_qty_at_most
                                        .map(|max_qty| p.qty.abs() <= max_qty + 1e-8)
                                        .unwrap_or(true)
                            }) =>
                        {
                            found = true;
                            break;
                        }
                        Ok(_) => {
                            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        }
                        Err(e) => {
                            let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
                                "Kraken exit sync {}: position poll failed: {}",
                                pair, e
                            )));
                            break;
                        }
                    }
                }
                if !found {
                    let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
                        "Kraken exit sync {}: position not visible at target size yet",
                        pair
                    )));
                    return;
                }
            }
            match kb.sync_position_exits(&pair, sl_price, tp_price).await {
                Ok(summary) => {
                    let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                        "Kraken exits {}: {}",
                        pair, summary
                    )));
                }
                Err(e) => {
                    let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
                        "Kraken exit sync failed for {}: {}",
                        pair, e
                    )));
                }
            }
        }
        _ => unreachable!("non-Kraken order command routed to Kraken order handler"),
    }
}

pub(super) async fn handle_kraken_account_order_command(
    cmd: BrokerCmd,
    kraken_broker: Option<&typhoon_engine::broker::kraken::KrakenBroker>,
    broker_msg_tx: &tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
) {
    match cmd {
        BrokerCmd::KrakenGetBalance => {
            if let Some(ref kb) = kraken_broker {
                let msg_tx = broker_msg_tx.clone();
                match kb.get_balance().await {
                    Ok(balances) => {
                        let bal_vec: Vec<(String, f64)> =
                            balances.into_iter().filter(|(_, v)| *v > 0.0).collect();
                        let _ = msg_tx.send(BrokerMsg::KrakenBalances(bal_vec));
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("Kraken balance: {}", e)));
                    }
                }
            } else {
                let _ = broker_msg_tx.send(BrokerMsg::OrderResult("Kraken: connect first".into()));
            }
        }
        BrokerCmd::KrakenGetPositions => {
            if let Some(ref kb) = kraken_broker {
                match kb.get_all_position_summaries().await {
                    Ok(pos) => {
                        let _ = broker_msg_tx.send(BrokerMsg::KrakenPositions(pos));
                    }
                    Err(e) => {
                        let _ = broker_msg_tx
                            .send(BrokerMsg::Error(format!("Kraken positions: {}", e)));
                    }
                }
            } else {
                let _ = broker_msg_tx.send(BrokerMsg::OrderResult("Kraken: connect first".into()));
            }
        }
        BrokerCmd::KrakenPlaceOrder {
            pair,
            side,
            order_type,
            volume,
            price,
            leverage,
        } => {
            if let Some(ref kb) = kraken_broker {
                let msg_tx = broker_msg_tx.clone();
                match kb
                    .place_order_with_leverage(
                        &pair,
                        &side,
                        &order_type,
                        volume,
                        price,
                        leverage.as_deref(),
                    )
                    .await
                {
                    Ok(result) => {
                        let text = serde_json::to_string_pretty(&result).unwrap_or_default();
                        let _ = msg_tx.send(BrokerMsg::OrderResult(format!(
                            "Kraken order placed: {}",
                            text
                        )));
                    }
                    Err(e) => {
                        let _ =
                            msg_tx.send(BrokerMsg::Error(format!("Kraken order failed: {}", e)));
                    }
                }
            } else {
                let _ = broker_msg_tx.send(BrokerMsg::OrderResult("Kraken: connect first".into()));
            }
        }
        BrokerCmd::KrakenPlaceOrderAdvanced { order } => {
            if let Some(ref kb) = kraken_broker {
                let msg_tx = broker_msg_tx.clone();
                match kb.place_order_request(&order).await {
                    Ok(result) => {
                        let text = serde_json::to_string_pretty(&result).unwrap_or_default();
                        let _ = msg_tx.send(BrokerMsg::OrderResult(format!(
                            "Kraken order placed: {}",
                            text
                        )));
                    }
                    Err(e) => {
                        let _ =
                            msg_tx.send(BrokerMsg::Error(format!("Kraken order failed: {}", e)));
                    }
                }
            } else {
                let _ = broker_msg_tx.send(BrokerMsg::OrderResult("Kraken: connect first".into()));
            }
        }
        BrokerCmd::KrakenClosePosition { pair, volume } => {
            if let Some(ref kb) = kraken_broker {
                match kb.close_position(&pair, volume).await {
                    Ok(result) => {
                        let text = serde_json::to_string_pretty(&result).unwrap_or_default();
                        let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                            "Kraken close {}: {}",
                            pair, text
                        )));
                    }
                    Err(e) => {
                        let _ = broker_msg_tx
                            .send(BrokerMsg::Error(format!("Kraken close {}: {}", pair, e)));
                    }
                }
            } else {
                let _ = broker_msg_tx.send(BrokerMsg::OrderResult("Kraken: connect first".into()));
            }
        }
        BrokerCmd::KrakenCancelOrder { txid } => {
            if let Some(ref kb) = kraken_broker {
                let msg_tx = broker_msg_tx.clone();
                match kb.cancel_order(&txid).await {
                    Ok(result) => {
                        let text = serde_json::to_string_pretty(&result).unwrap_or_default();
                        let _ =
                            msg_tx.send(BrokerMsg::OrderResult(format!("Kraken cancel: {}", text)));
                    }
                    Err(e) => {
                        let _ =
                            msg_tx.send(BrokerMsg::Error(format!("Kraken cancel failed: {}", e)));
                    }
                }
            } else {
                let _ = broker_msg_tx.send(BrokerMsg::OrderResult("Kraken: connect first".into()));
            }
        }
        BrokerCmd::KrakenCancelAll => {
            if let Some(ref kb) = kraken_broker {
                let msg_tx = broker_msg_tx.clone();
                match kb.cancel_all_orders().await {
                    Ok(result) => {
                        let count = result["count"].as_u64().unwrap_or(0);
                        let _ = msg_tx.send(BrokerMsg::OrderResult(format!(
                            "Kraken: cancelled {} orders",
                            count
                        )));
                    }
                    Err(e) => {
                        let _ = msg_tx
                            .send(BrokerMsg::Error(format!("Kraken cancel all failed: {}", e)));
                    }
                }
            } else {
                let _ = broker_msg_tx.send(BrokerMsg::OrderResult("Kraken: connect first".into()));
            }
        }

        BrokerCmd::KrakenFetchTrades => {
            if let Some(ref kb) = kraken_broker {
                let msg_tx = broker_msg_tx.clone();
                match kb.get_all_trades_history_parsed(None, None).await {
                    Ok(trades) => {
                        let _ = msg_tx.send(BrokerMsg::KrakenTrades(trades));
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!(
                            "Kraken trade history failed: {}",
                            e
                        )));
                    }
                }
            }
        }
        BrokerCmd::KrakenFetchOpenOrders => {
            if let Some(ref kb) = kraken_broker {
                let msg_tx = broker_msg_tx.clone();
                match kb.get_open_orders_parsed().await {
                    Ok(orders) => {
                        let _ = msg_tx.send(BrokerMsg::KrakenOpenOrders(orders));
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!(
                            "Kraken open orders failed: {}",
                            e
                        )));
                    }
                }
            }
        }
        BrokerCmd::KrakenCloseAll => {
            if let Some(ref kb) = kraken_broker {
                match kb.close_all_positions().await {
                    Ok(count) => {
                        let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                            "Kraken: closed {} position(s)",
                            count
                        )));
                    }
                    Err(e) => {
                        let _ = broker_msg_tx
                            .send(BrokerMsg::Error(format!("Kraken close all failed: {}", e)));
                    }
                }
            } else {
                let _ = broker_msg_tx.send(BrokerMsg::OrderResult("Kraken: connect first".into()));
            }
        }
        BrokerCmd::KrakenGetPairs => {
            // Public endpoint — no auth needed, create temporary broker if none
            let msg_tx = broker_msg_tx.clone();
            let kb = if let Some(ref kb) = kraken_broker {
                kb.get_tradeable_pairs().await
            } else {
                let tmp =
                    typhoon_engine::broker::kraken::KrakenBroker::new(String::new(), String::new());
                tmp.get_tradeable_pairs().await
            };
            match kb {
                Ok(pairs) => {
                    let _ = msg_tx.send(BrokerMsg::KrakenPairs(pairs));
                }
                Err(e) => {
                    let _ = msg_tx.send(BrokerMsg::Error(format!("Kraken pairs: {}", e)));
                }
            }
        }
        _ => unreachable!("non-Kraken account/order command routed to Kraken handler"),
    }
}
