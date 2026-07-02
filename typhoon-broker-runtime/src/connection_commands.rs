use std::sync::Arc;

use typhoon_engine::broker::alpaca::AlpacaBroker;
use typhoon_engine::broker::protocol::{BrokerAccountSpec, BrokerCmd, BrokerMsg, OrderBroker};

use crate::account_pool::{
    AlpacaAccountHandle, AlpacaAccountPool, KrakenAccountHandle, KrakenAccountPool,
};

/// Validate + connect every configured Alpaca account concurrently and build
/// the account pool. Each account gets its own `AlpacaBroker` (thus its own
/// rate limiters), which is what makes N-account bar-sync fan-out multiply
/// the historical request budget (ADR-130).
pub async fn connect_alpaca_pool(
    accounts: Vec<BrokerAccountSpec>,
    primary_id: &str,
    bar_requests_per_minute: u32,
    broker_msg_tx: &tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
) -> AlpacaAccountPool {
    let rpm = bar_requests_per_minute
        .max(typhoon_engine::broker::alpaca::DEFAULT_BAR_REQUESTS_PER_MINUTE);
    let validations = accounts.into_iter().map(|spec| async move {
        let broker = AlpacaBroker::new(spec.api_key.clone(), spec.secret.clone(), spec.paper, rpm);
        let account = broker.get_account().await;
        (spec, broker, account)
    });
    let results = futures_util::future::join_all(validations).await;

    let mut handles = Vec::with_capacity(results.len());
    let mut primary_account_info = None;
    let mut ok_count = 0usize;
    let total = results.len();
    for (spec, broker, account) in results {
        let (connected, equity, detail) = match account {
            Ok(acct) => {
                ok_count += 1;
                let equity = acct.equity;
                if spec.id == primary_id {
                    primary_account_info = Some(acct);
                }
                (true, equity, "Connected".to_string())
            }
            Err(e) => {
                let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
                    "Alpaca account '{}' connection failed: {}",
                    spec.label, e
                )));
                (false, 0.0, e)
            }
        };
        handles.push(AlpacaAccountHandle {
            spec,
            broker,
            equity,
            connected,
            detail,
        });
    }

    let pool = AlpacaAccountPool::new(handles, primary_id);
    if let Some(primary) = pool.primary_broker() {
        // The requested primary may have failed auth; re-pull if the pool fell
        // back to a different connected account.
        let acct = match primary_account_info {
            Some(acct) if pool.primary_id() == Some(primary_id) => Some(acct),
            _ => primary.get_account().await.ok(),
        };
        if let Some(acct) = acct {
            let _ = broker_msg_tx.send(BrokerMsg::Connected(format!(
                "Connected: ${:.2} equity, ${:.2} buying power ({}/{} Alpaca account(s), {} in data-sync rotation)",
                acct.equity,
                acct.buying_power,
                ok_count,
                total,
                pool.data_account_count()
            )));
            let _ = broker_msg_tx.send(BrokerMsg::Account(acct));
        }
        primary.warm_data_connection().await;
    } else {
        let _ = broker_msg_tx.send(BrokerMsg::Error(
            "Connection failed: no Alpaca account authenticated".into(),
        ));
    }
    let _ = broker_msg_tx.send(BrokerMsg::AccountRoster {
        broker: OrderBroker::Alpaca,
        accounts: pool.roster(),
    });
    pool
}

/// Re-emit the primary Alpaca account state (account, positions, open orders,
/// recent fills). Used after connect and after a primary-account switch.
pub async fn emit_alpaca_primary_state(
    broker: &AlpacaBroker,
    broker_msg_tx: &tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
) {
    if let Ok(acct) = broker.get_account().await {
        let _ = broker_msg_tx.send(BrokerMsg::Account(acct));
    }
    if let Ok(pos) = broker.get_positions().await {
        let _ = broker_msg_tx.send(BrokerMsg::Positions(pos));
    }
    if let Ok(orders) = broker.get_orders("open", 100).await {
        let _ = broker_msg_tx.send(BrokerMsg::Orders(orders));
    }
    crate::alpaca_account_data::fetch_and_send_recent_fills(broker, broker_msg_tx, 100).await;
}

async fn refresh_kraken_account_state(
    kb: &typhoon_engine::broker::kraken::KrakenBroker,
    msg_tx: &tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
) -> Result<(), String> {
    use typhoon_engine::broker::kraken::KrakenBroker;
    let balances = kb.get_balance().await?;
    let mut bal_vec: Vec<(String, f64)> = balances.into_iter().filter(|(_, v)| *v > 0.0).collect();
    bal_vec.sort_by(|a, b| a.0.cmp(&b.0));
    let mut pos = kb.get_position_summaries().await.unwrap_or_default();
    pos.extend(KrakenBroker::equity_position_summaries_from_balances(
        &bal_vec,
    ));
    pos.sort_by(|a, b| a.symbol.cmp(&b.symbol));
    let _ = msg_tx.send(BrokerMsg::KrakenBalances(bal_vec));
    let _ = msg_tx.send(BrokerMsg::KrakenPositions(pos));
    Ok(())
}

pub async fn handle_connection_command(
    cmd: BrokerCmd,
    alpaca_pool: &mut AlpacaAccountPool,
    kraken_pool: &mut KrakenAccountPool,
    kraken_ws_broker: &mut Option<typhoon_engine::broker::kraken::KrakenBroker>,
    alpaca_fetch_permits: &mut Arc<tokio::sync::Semaphore>,
    broker_msg_tx: &tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
) {
    match cmd {
        BrokerCmd::Connect {
            accounts,
            primary_id,
            bar_requests_per_minute,
            fetch_permits,
        } => {
            *alpaca_fetch_permits = Arc::new(tokio::sync::Semaphore::new(fetch_permits.max(1)));
            *alpaca_pool = connect_alpaca_pool(
                accounts,
                &primary_id,
                bar_requests_per_minute,
                broker_msg_tx,
            )
            .await;
        }
        BrokerCmd::ConfigureAlpacaSync {
            bar_requests_per_minute,
            fetch_permits,
        } => {
            *alpaca_fetch_permits = Arc::new(tokio::sync::Semaphore::new(fetch_permits.max(1)));
            alpaca_pool
                .apply_bar_rpm_hint(
                    bar_requests_per_minute
                        .max(typhoon_engine::broker::alpaca::DEFAULT_BAR_REQUESTS_PER_MINUTE),
                )
                .await;
        }
        BrokerCmd::SetPrimaryAccount { broker, account_id } => match broker {
            OrderBroker::Alpaca => {
                if alpaca_pool.set_primary(&account_id) {
                    let _ = broker_msg_tx.send(BrokerMsg::AccountRoster {
                        broker: OrderBroker::Alpaca,
                        accounts: alpaca_pool.roster(),
                    });
                    if let Some(primary) = alpaca_pool.primary_broker() {
                        emit_alpaca_primary_state(primary, broker_msg_tx).await;
                    }
                } else {
                    let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
                        "Alpaca primary switch failed: account '{}' is not connected",
                        account_id
                    )));
                    let _ = broker_msg_tx.send(BrokerMsg::AccountRoster {
                        broker: OrderBroker::Alpaca,
                        accounts: alpaca_pool.roster(),
                    });
                }
            }
            OrderBroker::Kraken => {
                if kraken_pool.set_primary(&account_id) {
                    // The WS-token broker follows the new primary (dedicated WS
                    // override for the first account, REST keys otherwise).
                    if let Some((ws_key, ws_secret)) = kraken_pool.ws_keys_for_primary() {
                        *kraken_ws_broker = Some(typhoon_engine::broker::kraken::KrakenBroker::new(
                            ws_key, ws_secret,
                        ));
                    }
                    let _ = broker_msg_tx.send(BrokerMsg::AccountRoster {
                        broker: OrderBroker::Kraken,
                        accounts: kraken_pool.roster(),
                    });
                    if let Some(kb) = kraken_pool.primary_broker() {
                        if let Err(e) = refresh_kraken_account_state(kb, broker_msg_tx).await {
                            let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
                                "Kraken primary refresh failed: {}",
                                e
                            )));
                        }
                    }
                    let _ = broker_msg_tx.send(BrokerMsg::OrderResult(
                        "Kraken primary switched — private WS (ownTrades/openOrders) still \
                         follows the previous account until the next app restart"
                            .into(),
                    ));
                } else {
                    let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
                        "Kraken primary switch failed: account '{}' is not connected",
                        account_id
                    )));
                }
            }
        },
        BrokerCmd::KrakenConnect {
            api_key,
            api_secret,
            ws_api_key,
            ws_api_secret,
            extra_accounts,
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
            let kb = KrakenBroker::new(api_key.clone(), api_secret.clone());
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
                        KrakenBroker::new(ws_api_key.clone(), ws_api_secret.clone())
                    } else {
                        KrakenBroker::new(rest_api_key.clone(), rest_api_secret.clone())
                    });

                    // Build the pool: primary first, then validated extras.
                    let mut handles = vec![KrakenAccountHandle {
                        spec: BrokerAccountSpec {
                            id: "kraken1".into(),
                            label: "Kraken 1".into(),
                            api_key: rest_api_key,
                            secret: rest_api_secret,
                            paper: false,
                            trade_enabled: true,
                            data_sync_enabled: false,
                        },
                        broker: kb,
                        connected: true,
                        detail: "Connected".into(),
                    }];
                    for spec in extra_accounts {
                        if spec.api_key.trim().is_empty() || spec.secret.trim().is_empty() {
                            continue;
                        }
                        let extra =
                            KrakenBroker::new(spec.api_key.clone(), spec.secret.clone());
                        let (connected, detail) = match extra.get_balance().await {
                            Ok(_) => (true, "Connected".to_string()),
                            Err(e) => {
                                let _ = msg_tx.send(BrokerMsg::Error(format!(
                                    "Kraken account '{}' connection failed: {}",
                                    spec.label, e
                                )));
                                (false, e)
                            }
                        };
                        handles.push(KrakenAccountHandle {
                            spec,
                            broker: extra,
                            connected,
                            detail,
                        });
                    }
                    *kraken_pool = KrakenAccountPool::new(handles, "kraken1");
                    if ws_override_ready {
                        kraken_pool.set_ws_override(ws_api_key.clone(), ws_api_secret.clone());
                    }
                    let _ = msg_tx.send(BrokerMsg::AccountRoster {
                        broker: OrderBroker::Kraken,
                        accounts: kraken_pool.roster(),
                    });
                }
                Err(e) => {
                    let _ = msg_tx.send(BrokerMsg::Error(format!("Kraken auth failed: {}", e)));
                }
            }
        }
        _ => unreachable!("non-connection command routed to connection handler"),
    }
}
