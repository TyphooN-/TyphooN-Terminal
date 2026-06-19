use super::*;

pub(super) async fn handle_connection_command(
    cmd: BrokerCmd,
    broker: &mut Option<AlpacaBroker>,
    kraken_broker: &mut Option<typhoon_engine::broker::kraken::KrakenBroker>,
    kraken_ws_broker: &mut Option<typhoon_engine::broker::kraken::KrakenBroker>,
    alpaca_fetch_permits: &mut Arc<tokio::sync::Semaphore>,
    broker_msg_tx: &tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
) {
    match cmd {
        BrokerCmd::Connect {
            api_key,
            secret,
            paper,
            bar_requests_per_minute,
            fetch_permits,
        } => {
            *alpaca_fetch_permits = Arc::new(tokio::sync::Semaphore::new(fetch_permits.max(1)));
            let b = AlpacaBroker::new(
                api_key,
                secret,
                paper,
                bar_requests_per_minute.max(ALPACA_DEFAULT_HISTORICAL_RPM),
            );
            match b.get_account().await {
                Ok(acct) => {
                    let _ = broker_msg_tx.send(BrokerMsg::Connected(format!(
                        "Connected: ${:.2} equity, ${:.2} buying power",
                        acct.equity, acct.buying_power
                    )));
                    let _ = broker_msg_tx.send(BrokerMsg::Account(acct));
                    b.warm_data_connection().await;
                    *broker = Some(b);
                }
                Err(e) => {
                    let _ =
                        broker_msg_tx.send(BrokerMsg::Error(format!("Connection failed: {}", e)));
                }
            }
        }
        BrokerCmd::ConfigureAlpacaSync {
            bar_requests_per_minute,
            fetch_permits,
        } => {
            *alpaca_fetch_permits = Arc::new(tokio::sync::Semaphore::new(fetch_permits.max(1)));
            if let Some(b) = broker.as_ref() {
                b.set_bar_requests_per_minute_hint(
                    bar_requests_per_minute.max(ALPACA_DEFAULT_HISTORICAL_RPM),
                )
                .await;
            }
        }
        BrokerCmd::KrakenConnect {
            api_key,
            api_secret,
            ws_api_key,
            ws_api_secret,
        } => {
            use typhoon_engine::broker::kraken::KrakenBroker;
            let msg_tx = broker_msg_tx.clone();
            let rest_ready = !api_key.trim().is_empty() && !api_secret.trim().is_empty();
            let ws_override_ready =
                !ws_api_key.trim().is_empty() && !ws_api_secret.trim().is_empty();
            let ws_creds = if ws_override_ready {
                Some((ws_api_key.clone(), ws_api_secret.clone(), "WebSocket"))
            } else if rest_ready {
                Some((api_key.clone(), api_secret.clone(), "REST"))
            } else {
                None
            };
            let mut ws_status: Option<String> = None;
            if let Some((ws_key, ws_secret, label)) = ws_creds {
                let ws_kb = KrakenBroker::new(ws_key, ws_secret);
                ws_status = Some(match ws_kb.get_websockets_token_string().await {
                    Ok(_token) => format!("WS auth ready via {} key", label),
                    Err(e) => format!("WS auth unavailable via {} key: {}", label, e),
                });
            }
            if !rest_ready {
                let suffix = ws_status
                    .as_ref()
                    .map(|status| format!(" ({})", status))
                    .unwrap_or_default();
                let _ = msg_tx.send(BrokerMsg::Error(format!(
                    "Kraken REST key required for account/trading{}",
                    suffix
                )));
                return;
            }
            let rest_api_key = api_key.clone();
            let rest_api_secret = api_secret.clone();
            let kb = KrakenBroker::new(api_key, api_secret);
            match kb.get_balance().await {
                Ok(balances) => {
                    let mut bal_vec: Vec<(String, f64)> =
                        balances.into_iter().filter(|(_, v)| *v > 0.0).collect();
                    bal_vec.sort_by(|a, b| a.0.cmp(&b.0));
                    let summary: String = bal_vec
                        .iter()
                        .map(|(a, v)| format!("{}: {:.8}", a, v))
                        .collect::<Vec<_>>()
                        .join(", ");
                    let ws_suffix = ws_status
                        .as_ref()
                        .map(|status| format!(" · {}", status))
                        .unwrap_or_else(|| " · WS auth not configured".to_string());
                    let _ = msg_tx.send(BrokerMsg::Connected(format!(
                        "Kraken connected — {} assets ({}){}",
                        bal_vec.len(),
                        summary,
                        ws_suffix
                    )));
                    let mut pos = kb.get_position_summaries().await.unwrap_or_default();
                    pos.extend(KrakenBroker::equity_position_summaries_from_balances(
                        &bal_vec,
                    ));
                    pos.sort_by(|a, b| a.symbol.cmp(&b.symbol));
                    let _ = msg_tx.send(BrokerMsg::KrakenBalances(bal_vec));
                    let _ = msg_tx.send(BrokerMsg::KrakenPositions(pos));
                    if let Ok(pairs) = kb.get_tradeable_pairs().await {
                        let _ = msg_tx.send(BrokerMsg::KrakenPairs(pairs));
                    }
                    *kraken_ws_broker = Some(if ws_override_ready {
                        KrakenBroker::new(ws_api_key, ws_api_secret)
                    } else {
                        KrakenBroker::new(rest_api_key, rest_api_secret)
                    });
                    *kraken_broker = Some(kb);
                }
                Err(e) => {
                    let _ = msg_tx.send(BrokerMsg::Error(format!("Kraken auth failed: {}", e)));
                }
            }
        }
        _ => unreachable!("non-connection command routed to connection handler"),
    }
}
