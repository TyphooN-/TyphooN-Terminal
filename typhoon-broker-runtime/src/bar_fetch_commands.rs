use std::sync::Arc;

use typhoon_engine::broker::protocol::{BrokerCmd, BrokerMsg};
use typhoon_engine::core::cache::SqliteCache;

use crate::account_pool::AlpacaAccountPool;

pub async fn handle_bar_fetch_command(
    cmd: BrokerCmd,
    alpaca_pool: &AlpacaAccountPool,
    broker_msg_tx: &tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    shared_cache_broker: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
    alpaca_fetch_permits: Arc<tokio::sync::Semaphore>,
    kraken_fetch_permits: Arc<tokio::sync::Semaphore>,
    kraken_public_client: reqwest::Client,
) {
    match cmd {
        BrokerCmd::AlpacaFetchBars {
            symbol,
            timeframe,
            db_path: _,
            backfill_complete,
        } => {
            // Round-robin over the data-sync account rotation: each account
            // owns an independent rate limiter, so N accounts multiply the
            // aggregate historical budget (ADR-130).
            if let Some(broker) = alpaca_pool.next_data_broker() {
                let msg_tx = broker_msg_tx.clone();
                let shared_cache = shared_cache_broker.clone();
                let permits = alpaca_fetch_permits.clone();
                tokio::spawn(async move {
                    let Ok(_permit) = permits.acquire_owned().await else {
                        let _ = msg_tx.send(BrokerMsg::AlpacaFetchSettled {
                            symbol,
                            timeframe,
                            success: false,
                        });
                        return;
                    };
                    typhoon_engine::broker::bar_fetch::run_alpaca_fetch_task(
                        broker,
                        shared_cache,
                        msg_tx,
                        symbol,
                        timeframe,
                        backfill_complete,
                    )
                    .await;
                });
            } else {
                let _ = broker_msg_tx.send(BrokerMsg::Error(
                    "Broker not connected — connect Alpaca first".into(),
                ));
                let _ = broker_msg_tx.send(BrokerMsg::AlpacaFetchSettled {
                    symbol,
                    timeframe,
                    success: false,
                });
            }
        }
        BrokerCmd::AlpacaFetchBarsBatch { symbols, timeframe } => {
            if let Some(broker) = alpaca_pool.next_data_broker() {
                let msg_tx = broker_msg_tx.clone();
                let shared_cache = shared_cache_broker.clone();
                let permits = alpaca_fetch_permits.clone();
                tokio::spawn(async move {
                    let Ok(_permit) = permits.acquire_owned().await else {
                        for symbol in symbols {
                            let _ = msg_tx.send(BrokerMsg::AlpacaFetchSettled {
                                symbol,
                                timeframe: timeframe.clone(),
                                success: false,
                            });
                        }
                        return;
                    };
                    typhoon_engine::broker::bar_fetch::run_alpaca_batch_fetch_task(
                        broker,
                        shared_cache,
                        msg_tx,
                        symbols,
                        timeframe,
                    )
                    .await;
                });
            } else {
                let _ = broker_msg_tx.send(BrokerMsg::Error(
                    "Broker not connected — connect Alpaca first".into(),
                ));
                for symbol in symbols {
                    let _ = broker_msg_tx.send(BrokerMsg::AlpacaFetchSettled {
                        symbol,
                        timeframe: timeframe.clone(),
                        success: false,
                    });
                }
            }
        }
        BrokerCmd::FetchAllBars { symbol, timeframe } => {
            // Sequential (not spawned) — prevents flooding Alpaca's rate limiter
            if let Some(b) = alpaca_pool.next_data_broker() {
                let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                    "BARDATA: fetching {} {}...",
                    symbol, timeframe
                )));
                match b.get_all_bars(&symbol, &timeframe, None).await {
                    Ok((bars, outcome)) => {
                        let count = bars.len();
                        if count > 0 {
                            if let Some(cache) =
                                shared_cache_broker.read().ok().and_then(|g| g.clone())
                            {
                                let bare = symbol.replace('/', "");
                                let key = format!("alpaca:{}:{}", bare, timeframe);
                                let json = serde_json::to_string(&bars).unwrap_or_default();
                                let _ = cache.put_bars(&key, &json);
                                let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                                    "BARDATA: {} {} — {} bars stored",
                                    symbol, timeframe, count
                                )));
                                let _ = broker_msg_tx.send(BrokerMsg::BarsSynced(count));
                            }
                        }
                        match outcome {
                            typhoon_engine::broker::alpaca::FetchOutcome::RateLimitedPartial => {
                                let _ = broker_msg_tx.send(BrokerMsg::AlpacaRetryEnqueue {
                                    symbol: symbol.clone(),
                                    timeframe: timeframe.clone(),
                                    reason: "rate_limited_partial".into(),
                                });
                            }
                            typhoon_engine::broker::alpaca::FetchOutcome::RateLimitedEmpty => {
                                let _ = broker_msg_tx.send(BrokerMsg::AlpacaRetryEnqueue {
                                    symbol: symbol.clone(),
                                    timeframe: timeframe.clone(),
                                    reason: "rate_limited_empty".into(),
                                });
                            }
                            typhoon_engine::broker::alpaca::FetchOutcome::Complete => {}
                        }
                    }
                    Err(e) => {
                        let is_rate = e.contains("429") || e.to_lowercase().contains("rate limit");
                        let is_no_data = e.contains("No bar data for ");
                        let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                            "BARDATA: {} {} — {}",
                            symbol, timeframe, e
                        )));
                        if is_no_data {
                            let _ = broker_msg_tx.send(BrokerMsg::AlpacaNoData {
                                symbol: symbol.clone(),
                                timeframe: timeframe.clone(),
                                reason: e.clone(),
                            });
                        } else if is_rate {
                            let _ = broker_msg_tx.send(BrokerMsg::AlpacaRetryEnqueue {
                                symbol: symbol.clone(),
                                timeframe: timeframe.clone(),
                                reason: format!("err:{}", e),
                            });
                        }
                    }
                }
            } else {
                let _ =
                    broker_msg_tx.send(BrokerMsg::Error("Connect Alpaca first for BARDATA".into()));
            }
        }
        BrokerCmd::KrakenBackfill {
            symbol,
            timeframes,
            db_path: _,
            backfill_complete,
        } => {
            let msg_tx = broker_msg_tx.clone();
            let shared_cache = shared_cache_broker.clone();
            let permits = kraken_fetch_permits.clone();
            for timeframe in timeframes {
                let msg_tx = msg_tx.clone();
                let shared_cache = shared_cache.clone();
                let permits = permits.clone();
                let client = kraken_public_client.clone();
                let symbol = symbol.clone();
                tokio::spawn(async move {
                    let Ok(_permit) = permits.acquire_owned().await else {
                        let _ = msg_tx.send(BrokerMsg::KrakenFetchSettled { symbol, timeframe });
                        return;
                    };
                    typhoon_engine::broker::bar_fetch::run_kraken_fetch_task(
                        shared_cache,
                        msg_tx,
                        client,
                        symbol,
                        timeframe,
                        backfill_complete,
                    )
                    .await;
                });
            }
        }
        BrokerCmd::KrakenFuturesBackfill {
            symbol,
            timeframes,
            db_path: _,
            backfill_complete,
        } => {
            let msg_tx = broker_msg_tx.clone();
            let shared_cache = shared_cache_broker.clone();
            let permits = kraken_fetch_permits.clone();
            for timeframe in timeframes {
                let msg_tx = msg_tx.clone();
                let shared_cache = shared_cache.clone();
                let permits = permits.clone();
                let client = kraken_public_client.clone();
                let symbol = symbol.clone();
                tokio::spawn(async move {
                    let Ok(_permit) = permits.acquire_owned().await else {
                        let _ =
                            msg_tx.send(BrokerMsg::KrakenFuturesFetchSettled { symbol, timeframe });
                        return;
                    };
                    typhoon_engine::broker::bar_fetch::run_kraken_futures_fetch_task(
                        shared_cache,
                        msg_tx,
                        client,
                        symbol,
                        timeframe,
                        backfill_complete,
                    )
                    .await;
                });
            }
        }
        _ => unreachable!("non-bar-fetch command routed to bar fetch handler"),
    }
}
