use super::prelude::*;

pub(super) async fn handle_kraken_market_command(
    cmd: BrokerCmd,
    kraken_broker: Option<&typhoon_engine::broker::kraken::KrakenBroker>,
    broker_msg_tx: &tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    shared_cache_broker: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
    kraken_equity_fetch_permits: Arc<tokio::sync::Semaphore>,
    yahoo_chart_fetch_permits: Arc<tokio::sync::Semaphore>,
    kraken_public_client: reqwest::Client,
    fallback_bar_client: reqwest::Client,
) {
    match cmd {
        BrokerCmd::KrakenFetchEquityTicker { symbol } => {
            let result = if let Some(kb) = kraken_broker {
                kb.get_equity_ticker(&symbol).await
            } else {
                let kb =
                    typhoon_engine::broker::kraken::KrakenBroker::new(String::new(), String::new());
                kb.get_equity_ticker(&symbol).await
            };
            match result {
                Ok(ticker) => {
                    let _ = broker_msg_tx.send(BrokerMsg::KrakenEquityQuote(ticker));
                }
                Err(e) => {
                    let _ = broker_msg_tx.send(BrokerMsg::Error(e));
                }
            }
        }
        BrokerCmd::KrakenFetchEquityHistory { symbol, timeframe } => {
            // iapi_limiter inside get_equity_history short-circuits
            // with an IAPI_RATE_LIMITED prefixed error during an
            // active cooldown. Do the slow network + cache write in
            // its own capped task; the broker command loop must stay
            // free to process UI-visible commands and status messages.
            let msg_tx = broker_msg_tx.clone();
            let shared_cache = shared_cache_broker.clone();
            let permits = kraken_equity_fetch_permits.clone();
            tokio::spawn(async move {
                let Ok(_permit) = permits.acquire_owned().await else {
                    let _ = msg_tx.send(BrokerMsg::KrakenEquityHistoryError {
                        symbol,
                        timeframe,
                        error: "Kraken equities fetch permit closed".to_string(),
                    });
                    return;
                };
                let interval_minutes = match timeframe.as_str() {
                    "1Min" => 1,
                    "5Min" => 5,
                    "15Min" => 15,
                    "30Min" => 30,
                    "1Hour" => 60,
                    "4Hour" => 240,
                    "1Day" => 1440,
                    "1Week" => 10080,
                    "1Month" => 43200,
                    _ => 1,
                };
                let kb =
                    typhoon_engine::broker::kraken::KrakenBroker::new(String::new(), String::new());
                let result = kb.get_equity_history(&symbol, interval_minutes, None).await;
                match result {
                    Ok(bars) => {
                        let count = bars.len();
                        if count > 0 {
                            let cache_handle = shared_cache.read().ok().and_then(|g| g.clone());
                            if let Some(cache) = cache_handle {
                                let bars_for_cache = bars;
                                let cache_symbol = symbol.clone();
                                let cache_timeframe = timeframe.clone();
                                let cache_result = tokio::task::spawn_blocking(move || {
                                    let json_bars: Vec<_> = bars_for_cache
                                        .iter()
                                        .filter_map(|bar| {
                                            let ts = chrono::DateTime::from_timestamp_millis(
                                                bar.time_ms,
                                            )?
                                            .to_rfc3339();
                                            Some(serde_json::json!({
                                                "timestamp": ts,
                                                "open": bar.open,
                                                "high": bar.high,
                                                "low": bar.low,
                                                "close": bar.close,
                                                "volume": bar.volume,
                                            }))
                                        })
                                        .collect();
                                    let json = serde_json::to_string(&json_bars)
                                        .map_err(|e| e.to_string())?;
                                    let cache_key = format!(
                                        "kraken-equities:{}:{}",
                                        cache_symbol
                                            .replace('/', "")
                                            .trim_end_matches(".EQ")
                                            .to_ascii_uppercase(),
                                        cache_timeframe
                                    );
                                    cache.put_bars(&cache_key, &json)
                                })
                                .await
                                .map_err(|e| format!("cache write task failed: {e}"))
                                .and_then(|result| result);
                                if let Err(e) = cache_result {
                                    let _ = msg_tx.send(BrokerMsg::Error(format!(
                                        "Kraken equities cache write failed for {} {}: {}",
                                        symbol, timeframe, e
                                    )));
                                }
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::KrakenEquityBars {
                            symbol,
                            timeframe,
                            count,
                        });
                    }
                    Err(e) => {
                        // Engine-side iapi_limiter already armed the
                        // cooldown if this is a 429/1015; no extra
                        // handler-side state to update here.
                        let _ = msg_tx.send(BrokerMsg::KrakenEquityHistoryError {
                            symbol,
                            timeframe,
                            error: e,
                        });
                    }
                }
            });
        }
        BrokerCmd::YahooChartFetchBars { symbol, timeframe } => {
            let source = "yahoo-chart".to_string();
            let client = fallback_bar_client.clone();
            let shared_cache = shared_cache_broker.clone();
            let msg_tx = broker_msg_tx.clone();
            let permits = yahoo_chart_fetch_permits.clone();
            tokio::spawn(async move {
                let Ok(_permit) = permits.acquire_owned().await else {
                    let _ = msg_tx.send(BrokerMsg::BarsFetched {
                        source,
                        symbol,
                        timeframe,
                        count: 0,
                    });
                    return;
                };
                let result = async {
                    let bars = typhoon_engine::core::fallback_bars::fetch_yahoo_chart_bars(
                        &client, &symbol, &timeframe,
                    )
                    .await?;
                    let count =
                        if let Some(cache) = shared_cache.read().ok().and_then(|g| g.clone()) {
                            typhoon_engine::core::fallback_bars::store_fallback_bars(
                                &cache, &source, &symbol, &timeframe, &bars,
                            )?
                        } else {
                            return Err("cache unavailable".to_string());
                        };
                    Ok::<usize, String>(count)
                }
                .await;
                match result {
                    Ok(count) => {
                        if count == 0 {
                            let _ = msg_tx.send(BrokerMsg::Unresolvable {
                                broker: source.clone(),
                                symbol: symbol.clone(),
                                timeframe: timeframe.clone(),
                                reason: "provider returned no bars".to_string(),
                            });
                        }
                        let _ = msg_tx.send(BrokerMsg::BarsFetched {
                            source,
                            symbol,
                            timeframe,
                            count,
                        });
                    }
                    Err(error) => {
                        let provider_no_data =
                            typhoon_engine::core::fallback_bars::yahoo_chart_provider_no_data_error(
                                &error,
                            );
                        if provider_no_data {
                            let _ = msg_tx.send(BrokerMsg::Unresolvable {
                                broker: source.clone(),
                                symbol: symbol.clone(),
                                timeframe: timeframe.clone(),
                                reason: error.clone(),
                            });
                        } else {
                            let _ = msg_tx.send(BrokerMsg::Error(format!(
                                "Yahoo Chart fallback failed for {} {}: {}",
                                symbol, timeframe, error
                            )));
                        }
                        let _ = msg_tx.send(BrokerMsg::BarsFetched {
                            source,
                            symbol,
                            timeframe,
                            count: 0,
                        });
                    }
                }
            });
        }

        BrokerCmd::KrakenFetchEquityUniverse => {
            let result = if let Some(kb) = kraken_broker {
                kb.get_equity_markets().await
            } else {
                let kb =
                    typhoon_engine::broker::kraken::KrakenBroker::new(String::new(), String::new());
                kb.get_equity_markets().await
            };
            match result {
                Ok(markets) => {
                    let _ = broker_msg_tx.send(BrokerMsg::KrakenEquityUniverse(markets));
                }
                Err(e) => {
                    let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
                        "Kraken equities universe failed: {e}"
                    )));
                }
            }
        }
        BrokerCmd::KrakenFuturesGetInstruments => {
            match typhoon_engine::core::kraken_futures::discover_instruments(&kraken_public_client)
                .await
            {
                Ok(symbols) => {
                    let _ = broker_msg_tx.send(BrokerMsg::KrakenFuturesInstruments(symbols));
                }
                Err(e) => {
                    let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
                        "Kraken futures instruments: {}",
                        e
                    )));
                }
            }
        }
        _ => unreachable!("non-Kraken market command routed to Kraken market handler"),
    }
}
