use super::*;

pub(super) async fn handle_alpaca_account_data_command(
    cmd: BrokerCmd,
    broker: Option<&AlpacaBroker>,
    broker_msg_tx: &tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
) {
    let Some(b) = broker else {
        return;
    };

    match cmd {
        BrokerCmd::GetAccount => match b.get_account().await {
            Ok(acct) => {
                let _ = broker_msg_tx.send(BrokerMsg::Account(acct));
            }
            Err(e) => {
                let _ = broker_msg_tx.send(BrokerMsg::Error(e));
            }
        },
        BrokerCmd::GetPositions => match b.get_positions().await {
            Ok(pos) => {
                let _ = broker_msg_tx.send(BrokerMsg::Positions(pos));
            }
            Err(e) => {
                tracing::debug!("Positions request failed: {}", e);
            }
        },
        BrokerCmd::GetOrders => match b.get_orders("open", 100).await {
            Ok(orders) => {
                let _ = broker_msg_tx.send(BrokerMsg::Orders(orders));
            }
            Err(e) => {
                tracing::debug!("Orders request failed: {}", e);
            }
        },
        BrokerCmd::GetOrderHistory { limit } => match b.get_orders("closed", limit).await {
            Ok(orders) => {
                let _ = broker_msg_tx.send(BrokerMsg::Orders(orders));
            }
            Err(e) => {
                let _ = broker_msg_tx.send(BrokerMsg::Error(e));
            }
        },
        BrokerCmd::GetActivities { limit } => match b.get_account_activities("FILL", limit).await {
            Ok(activities) => {
                let text = activities
                    .iter()
                    .take(20)
                    .map(|a| {
                        format!(
                            "{} {} {} {} {}",
                            a.date,
                            a.side.as_deref().unwrap_or("—"),
                            a.qty.as_deref().unwrap_or("—"),
                            a.symbol.as_deref().unwrap_or("—"),
                            a.net_amount.as_deref().unwrap_or("—")
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                let _ = broker_msg_tx.send(BrokerMsg::JsonResult("Account Activities".into(), text));

                // Also send structured fills for chart overlay.
                let fills: Vec<(String, String, f64, f64, String)> = activities
                    .iter()
                    .filter(|a| a.activity_type == "FILL")
                    .filter_map(|a| {
                        let sym = a.symbol.as_deref()?.to_string();
                        let side = a.side.as_deref()?.to_string();
                        let qty: f64 = a.qty.as_deref()?.parse().ok()?;
                        let price: f64 = a.price.as_deref()?.parse().ok()?;
                        Some((sym, side, qty, price, a.date.clone()))
                    })
                    .collect();
                if !fills.is_empty() {
                    let _ = broker_msg_tx.send(BrokerMsg::RecentFills(fills));
                }
            }
            Err(e) => {
                let _ = broker_msg_tx.send(BrokerMsg::Error(e));
            }
        },
        BrokerCmd::GetTopMovers => match b.get_top_movers("stocks", 10).await {
            Ok(v) => {
                let text = serde_json::to_string_pretty(&v).unwrap_or_default();
                let _ = broker_msg_tx.send(BrokerMsg::JsonResult("Top Movers".into(), text));
            }
            Err(e) => {
                let _ = broker_msg_tx.send(BrokerMsg::Error(e));
            }
        },
        BrokerCmd::GetAllAssets => match b.get_all_assets().await {
            Ok(assets) => {
                let all: Vec<(String, String, String)> = assets
                    .iter()
                    .map(|a| (a.symbol.clone(), a.name.clone(), a.asset_class.clone()))
                    .collect();
                let _ = broker_msg_tx.send(BrokerMsg::AllAssets(all));
            }
            Err(e) => {
                let _ = broker_msg_tx.send(BrokerMsg::Error(format!("Asset fetch failed: {e}")));
            }
        },
        _ => {}
    }
}
