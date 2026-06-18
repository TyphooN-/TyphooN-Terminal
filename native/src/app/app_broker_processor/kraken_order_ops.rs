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
