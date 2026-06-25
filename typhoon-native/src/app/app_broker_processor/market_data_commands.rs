use super::prelude::*;

type KrakenBroker = typhoon_engine::broker::kraken::KrakenBroker;

pub(super) async fn handle_market_data_command(
    cmd: BrokerCmd,
    broker: Option<&AlpacaBroker>,
    kraken_broker: Option<&KrakenBroker>,
    shared_cache: &Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
    broker_msg_tx: &tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
) {
    match cmd {
        BrokerCmd::GetFundamentals { ticker } => {
            match AlpacaBroker::get_financial_analysis(&ticker).await {
                Ok(v) => send_json(broker_msg_tx, format!("Fundamentals: {}", ticker), &v),
                Err(e) => send_error(broker_msg_tx, e),
            }
        }
        BrokerCmd::GetHolders { ticker } => {
            match AlpacaBroker::get_institutional_holders(&ticker).await {
                Ok(v) => send_json(broker_msg_tx, format!("Holders: {}", ticker), &v),
                Err(e) => send_error(broker_msg_tx, e),
            }
        }
        BrokerCmd::GetOrderbook { symbol } => {
            handle_orderbook(symbol, broker, kraken_broker, broker_msg_tx).await;
        }
        BrokerCmd::GetMostActive => {
            let Some(b) = broker else {
                return;
            };
            match b.get_most_active(20).await {
                Ok(v) => send_json(broker_msg_tx, "Most Active", &v),
                Err(e) => send_error(broker_msg_tx, e),
            }
        }
        BrokerCmd::GetPortfolioHistory { period } => {
            let Some(b) = broker else {
                return;
            };
            match b.get_portfolio_history(&period, "1D").await {
                Ok(v) => send_json(broker_msg_tx, "Portfolio History", &v),
                Err(e) => send_error(broker_msg_tx, e),
            }
        }
        BrokerCmd::GetAnalyst {
            symbol,
            finnhub_key,
        } => {
            let Some(b) = broker else {
                return;
            };
            match b.get_finnhub_recommendations(&symbol, &finnhub_key).await {
                Ok(v) => send_json(broker_msg_tx, format!("Analyst: {}", symbol), &v),
                Err(e) => send_error(broker_msg_tx, e),
            }
        }
        BrokerCmd::GetPriceTarget {
            symbol,
            finnhub_key,
        } => {
            let Some(b) = broker else {
                return;
            };
            match b.get_finnhub_price_target(&symbol, &finnhub_key).await {
                Ok(v) => send_json(broker_msg_tx, format!("PriceTarget: {}", symbol), &v),
                Err(e) => send_error(broker_msg_tx, e),
            }
        }
        BrokerCmd::GetShortInterest {
            symbol,
            finnhub_key,
        } => {
            let Some(b) = broker else {
                return;
            };
            match b.get_finnhub_short_interest(&symbol, &finnhub_key).await {
                Ok(v) => {
                    persist_short_interest_history(&symbol, &v, shared_cache);
                    send_json(broker_msg_tx, format!("ShortInterest: {}", symbol), &v);
                }
                Err(e) => send_error(broker_msg_tx, e),
            }
        }
        BrokerCmd::GetCorporateActions { symbol } => {
            let Some(b) = broker else {
                return;
            };
            match b.get_corporate_actions(&symbol).await {
                Ok(v) => send_json(broker_msg_tx, format!("CorporateActions: {}", symbol), &v),
                Err(e) => send_error(broker_msg_tx, e),
            }
        }
        BrokerCmd::GetWatchlists => {
            let Some(b) = broker else {
                return;
            };
            match b.get_watchlists().await {
                Ok(v) => send_json(broker_msg_tx, "Watchlists", &v),
                Err(e) => send_error(broker_msg_tx, e),
            }
        }
        BrokerCmd::CreateWatchlist { name, symbols } => {
            let Some(b) = broker else {
                return;
            };
            match b.create_watchlist(&name, &symbols).await {
                Ok(_) => {
                    let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                        "Watchlist '{}' created ({} symbols)",
                        name,
                        symbols.len()
                    )));
                }
                Err(e) => send_error(broker_msg_tx, e),
            }
        }
        BrokerCmd::GetOptionsChain { symbol, expiry } => {
            let Some(b) = broker else {
                return;
            };
            match b.get_options_chain(&symbol, &expiry).await {
                Ok(contracts) => send_json(
                    broker_msg_tx,
                    format!("OptionsChain: {}", symbol),
                    &contracts,
                ),
                Err(e) => send_error(broker_msg_tx, e),
            }
        }
        _ => {}
    }
}

async fn handle_orderbook(
    symbol: String,
    broker: Option<&AlpacaBroker>,
    kraken_broker: Option<&KrakenBroker>,
    broker_msg_tx: &tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
) {
    let try_kraken = typhoon_engine::core::kraken::to_kraken_pair_lossy(&symbol).is_some();
    if try_kraken {
        let kraken_result = if let Some(kb) = kraken_broker {
            kb.get_orderbook_snapshot(&symbol, 100).await
        } else {
            let kb = KrakenBroker::new(String::new(), String::new());
            kb.get_orderbook_snapshot(&symbol, 100).await
        };
        match kraken_result {
            Ok(v) => {
                send_json(broker_msg_tx, format!("Orderbook: {}", symbol), &v);
                return;
            }
            Err(e) if broker.is_none() => {
                send_error(broker_msg_tx, e);
                return;
            }
            Err(_) => {}
        }
    }

    let Some(b) = broker else {
        return;
    };
    match b.get_orderbook(&symbol).await {
        Ok(v) => send_json(broker_msg_tx, format!("Orderbook: {}", symbol), &v),
        Err(e) => send_error(broker_msg_tx, e),
    }
}

fn persist_short_interest_history(
    symbol: &str,
    value: &[serde_json::Value],
    shared_cache: &Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
) {
    let Some(cache) = shared_cache.read().ok().and_then(|g| g.clone()) else {
        return;
    };
    let Ok(conn) = cache.connection() else {
        return;
    };
    let rows = typhoon_engine::core::research::short_interest_history_points_from_json_rows(value);
    if !rows.is_empty() {
        let _ = typhoon_engine::core::research::upsert_short_interest_history(&conn, symbol, &rows);
    }
}

fn send_json(
    broker_msg_tx: &tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    title: impl Into<String>,
    value: &impl serde::Serialize,
) {
    let text = serde_json::to_string_pretty(value).unwrap_or_default();
    let _ = broker_msg_tx.send(BrokerMsg::JsonResult(title.into(), text));
}

fn send_error(
    broker_msg_tx: &tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    error: impl Into<String>,
) {
    let _ = broker_msg_tx.send(BrokerMsg::Error(error.into()));
}
