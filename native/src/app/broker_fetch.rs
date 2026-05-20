use super::*;

pub(super) async fn run_alpaca_fetch_task(
    broker: AlpacaBroker,
    shared_cache: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
    broker_msg_tx: tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    symbol: String,
    timeframe: String,
) {
    let symbol = normalize_market_data_symbol(&symbol);
    let timeframe = normalize_sync_timeframe_key(&timeframe)
        .unwrap_or(timeframe.as_str())
        .to_string();
    let tf_alpaca = match timeframe.as_str() {
        "1Min" | "M1" => "1Min",
        "5Min" | "M5" => "5Min",
        "15Min" | "M15" => "15Min",
        "30Min" | "M30" => "30Min",
        "1Hour" | "H1" => "1Hour",
        "4Hour" | "H4" => "4Hour",
        "1Day" | "D1" => "1Day",
        "1Week" | "W1" => "1Week",
        "1Month" | "MN1" => "1Month",
        _ => "1Day",
    };

    let api_symbol = if symbol.contains('/') {
        symbol.clone()
    } else {
        let crypto_bases = [
            "BTC", "ETH", "SOL", "DOGE", "XRP", "ADA", "LTC", "LINK", "AVAX", "DOT", "UNI", "AAVE",
            "MATIC", "SHIB", "ATOM", "ALGO", "FTM", "NEAR", "APE", "ARB", "OP", "MKR", "COMP",
            "SNX", "CRV", "SUSHI", "YFI", "BAT", "MANA", "SAND", "AXS", "BCH", "ETC", "XLM", "FIL",
            "HBAR", "ICP", "VET", "THETA",
        ];
        let su = symbol.to_uppercase();
        crypto_bases
            .iter()
            .find_map(|base| {
                if su.starts_with(base) && su.ends_with("USD") && su.len() == base.len() + 3 {
                    Some(format!("{base}/USD"))
                } else {
                    None
                }
            })
            .unwrap_or_else(|| symbol.clone())
    };

    let bare_upper = symbol.replace('/', "").to_uppercase();
    let mt5_key = format!("mt5:{bare_upper}:{timeframe}");
    let tasty_key = format!("tastytrade:{bare_upper}:{timeframe}");
    let cache_key = format!("alpaca:{symbol}:{timeframe}");
    let cache_handle = shared_cache.read().ok().and_then(|g| g.clone());
    let mt5_has_bars = cache_handle
        .as_ref()
        .and_then(|c| c.get_incremental_start(&mt5_key).ok().flatten())
        .is_some();
    let tasty_has_bars = cache_handle
        .as_ref()
        .and_then(|c| c.get_incremental_start(&tasty_key).ok().flatten())
        .is_some();

    let incremental = cache_handle
        .as_ref()
        .and_then(|c| c.get_incremental_start(&cache_key).ok().flatten());
    let cached_count = incremental
        .as_ref()
        .map(|(_, count)| *count as i64)
        .unwrap_or(0);
    let mut after_ts = incremental.as_ref().map(|(ts, _)| ts.clone());
    let shallow = alpaca_sync_target_bars(&timeframe)
        .map(|target| cached_count > 0 && cached_count * 100 < (target as i64) * 95)
        .unwrap_or(false);
    let target_limit = alpaca_sync_target_bars(&timeframe).unwrap_or(10_000);

    let mut success = false;
    if mt5_has_bars {
        let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
            "Alpaca {} {}: MT5/Darwinex has this symbol — skipping",
            symbol, timeframe
        )));
        success = true;
    } else if tasty_has_bars {
        let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
            "Alpaca {} {}: tastytrade has this symbol — skipping",
            symbol, timeframe
        )));
        success = true;
    } else {
        if shallow {
            after_ts = None;
        }

        let result = if after_ts.is_none() {
            let msg = if incremental.is_some() && shallow {
                format!(
                    "Alpaca {} {}: shallow cache ({} bars) — syncing target depth ({} bars)...",
                    api_symbol, timeframe, cached_count, target_limit
                )
            } else {
                format!(
                    "Alpaca {} {}: fetching target depth ({} bars, first sync)...",
                    api_symbol, timeframe, target_limit
                )
            };
            let _ = broker_msg_tx.send(BrokerMsg::OrderResult(msg));
            broker
                .get_target_bars(&api_symbol, tf_alpaca, target_limit)
                .await
        } else {
            if let Some(ref ts) = after_ts {
                let delta_limit = alpaca_incremental_fetch_limit(&timeframe, after_ts.as_deref());
                let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                    "Alpaca {} {} delta since {} (limit {})...",
                    api_symbol,
                    timeframe,
                    &ts[..19.min(ts.len())],
                    delta_limit
                )));
                broker
                    .get_bars_after(&api_symbol, tf_alpaca, delta_limit, after_ts.as_deref())
                    .await
            } else {
                broker
                    .get_bars_after(&api_symbol, tf_alpaca, 1000, after_ts.as_deref())
                    .await
            }
        };

        match result {
            Ok((new_bars, outcome)) => {
                success = matches!(
                    outcome,
                    typhoon_engine::broker::alpaca::FetchOutcome::Complete
                );
                if new_bars.is_empty()
                    && after_ts.is_some()
                    && matches!(
                        outcome,
                        typhoon_engine::broker::alpaca::FetchOutcome::Complete
                    )
                {
                    let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                        "{} {} already up to date",
                        symbol, timeframe
                    )));
                } else if let Some(cache) = cache_handle.as_ref() {
                    let merged: Vec<_> = if after_ts.is_some() {
                        match cache.get_bars_raw(&cache_key) {
                            Ok(Some(existing_raw)) => {
                                let mut combined: Vec<typhoon_engine::broker::alpaca::Bar> =
                                    existing_raw
                                        .into_iter()
                                        .map(|(ts_ms, o, h, l, c, v)| {
                                            let dt = chrono::DateTime::from_timestamp_millis(ts_ms)
                                                .unwrap_or_default();
                                            typhoon_engine::broker::alpaca::Bar {
                                                timestamp: dt.to_rfc3339(),
                                                open: o,
                                                high: h,
                                                low: l,
                                                close: c,
                                                volume: v,
                                            }
                                        })
                                        .collect();
                                combined.extend(new_bars);
                                combined.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
                                combined.dedup_by(|a, b| a.timestamp == b.timestamp);
                                combined
                            }
                            _ => new_bars,
                        }
                    } else {
                        new_bars
                    };
                    let count = merged.len();
                    if count > 0 {
                        let json = serde_json::to_string(&merged).unwrap_or_default();
                        let _ = cache.put_bars(&cache_key, &json);
                        if after_ts.is_none()
                            && matches!(
                                outcome,
                                typhoon_engine::broker::alpaca::FetchOutcome::Complete
                            )
                            && count < target_limit as usize
                        {
                            let _ = broker_msg_tx.send(BrokerMsg::AlpacaBackfillComplete {
                                symbol: symbol.clone(),
                                timeframe: timeframe.clone(),
                                bar_count: count,
                                target_bars: target_limit as usize,
                            });
                        }
                        let _ = broker_msg_tx.send(BrokerMsg::BarsFetched {
                            source: "alpaca".into(),
                            symbol: symbol.clone(),
                            timeframe: timeframe.clone(),
                            count,
                        });
                    } else {
                        let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                            "No bars returned for {} {}",
                            symbol, timeframe
                        )));
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
                let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
                    "Fetch bars failed for {} {}: {}",
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
                        reason: format!("err:{e}"),
                    });
                }
            }
        }
    }

    let _ = broker_msg_tx.send(BrokerMsg::AlpacaRateLimitObserved {
        historical_rpm: broker.bar_requests_per_minute(),
    });
    let _ = broker_msg_tx.send(BrokerMsg::AlpacaFetchSettled {
        symbol,
        timeframe,
        success,
    });
}

fn raw_bars_to_json_values(raw: Vec<(i64, f64, f64, f64, f64, f64)>) -> Vec<serde_json::Value> {
    raw.into_iter()
        .map(|(ts_ms, o, h, l, c, v)| {
            let dt = chrono::DateTime::from_timestamp_millis(ts_ms).unwrap_or_default();
            serde_json::json!({
                "timestamp": dt.to_rfc3339(),
                "open": o,
                "high": h,
                "low": l,
                "close": c,
                "volume": v,
            })
        })
        .collect()
}

fn merge_json_bars(
    mut existing: Vec<serde_json::Value>,
    mut incoming: Vec<serde_json::Value>,
) -> Vec<serde_json::Value> {
    existing.append(&mut incoming);
    existing.sort_by(|a, b| {
        a["timestamp"]
            .as_str()
            .unwrap_or("")
            .cmp(b["timestamp"].as_str().unwrap_or(""))
    });
    existing.dedup_by(|a, b| a["timestamp"] == b["timestamp"]);
    existing
}

async fn store_json_bars_in_cache(
    cache_handle: Option<std::sync::Arc<SqliteCache>>,
    cache_key: String,
    bars: Vec<serde_json::Value>,
    merge_existing: bool,
) -> Result<usize, String> {
    if bars.is_empty() {
        return Ok(0);
    }
    let Some(cache) = cache_handle else {
        return Ok(0);
    };
    tokio::task::spawn_blocking(move || {
        let merged = if merge_existing {
            match cache.get_bars_raw(&cache_key) {
                Ok(Some(existing_raw)) => {
                    merge_json_bars(raw_bars_to_json_values(existing_raw), bars)
                }
                _ => bars,
            }
        } else {
            bars
        };
        let count = merged.len();
        if count > 0 {
            let json = serde_json::to_string(&merged).unwrap_or_default();
            cache.put_bars(&cache_key, &json)?;
        }
        Ok::<usize, String>(count)
    })
    .await
    .map_err(|e| format!("cache write task failed: {e}"))?
}

pub(super) async fn run_kraken_fetch_task(
    shared_cache: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
    broker_msg_tx: tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    client: reqwest::Client,
    symbol: String,
    timeframe: String,
) {
    let symbol = typhoon_engine::core::kraken::normalize_pair_symbol(&symbol);
    let timeframe = normalize_sync_timeframe_key(&timeframe)
        .unwrap_or(timeframe.as_str())
        .to_string();
    let cache_key = format!("kraken:{symbol}:{timeframe}");
    let cache_handle = shared_cache.read().ok().and_then(|g| g.clone());
    let incremental = cache_handle
        .as_ref()
        .and_then(|c| c.get_incremental_start(&cache_key).ok().flatten());
    let cached_count = incremental
        .as_ref()
        .map(|(_, count)| *count as i64)
        .unwrap_or(0);
    let mut after_ts = incremental.as_ref().map(|(ts, _)| ts.clone());
    let shallow = kraken_sync_target_bars(&timeframe)
        .map(|target| cached_count > 0 && cached_count * 100 < (target as i64) * 95)
        .unwrap_or(false);

    if shallow {
        after_ts = None;
    }
    let start_ms = after_ts
        .as_deref()
        .and_then(|ts| chrono::DateTime::parse_from_rfc3339(ts).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc).timestamp_millis())
        .unwrap_or(0);
    let now_ms = chrono::Utc::now().timestamp_millis();
    let log_msg = if incremental.is_some() && shallow {
        format!(
            "Kraken {} {}: limited-history cache ({} bars) — refreshing recent window...",
            symbol, timeframe, cached_count
        )
    } else if let Some(ref ts) = after_ts {
        format!(
            "Kraken {} {} delta since {}...",
            symbol,
            timeframe,
            &ts[..19.min(ts.len())]
        )
    } else {
        format!("Kraken {} {}: fetching recent window...", symbol, timeframe)
    };
    let _ = broker_msg_tx.send(BrokerMsg::OrderResult(log_msg));
    match typhoon_engine::core::kraken::fetch_binance_klines(
        &client, &symbol, &timeframe, start_ms, now_ms,
    )
    .await
    {
        Ok(new_bars) => {
            if new_bars.is_empty() && after_ts.is_some() {
                let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                    "Kraken {} {} already up to date",
                    symbol, timeframe
                )));
            } else {
                match store_json_bars_in_cache(
                    cache_handle.clone(),
                    cache_key.clone(),
                    new_bars,
                    after_ts.is_some(),
                )
                .await
                {
                    Ok(count) if count > 0 => {
                        if after_ts.is_none()
                            && kraken_sync_target_bars(&timeframe)
                                .is_some_and(|target| count < target as usize)
                        {
                            let _ = broker_msg_tx.send(BrokerMsg::KrakenBackfillComplete {
                                symbol: symbol.clone(),
                                timeframe: timeframe.clone(),
                                bar_count: count,
                                target_bars: kraken_sync_target_bars(&timeframe).unwrap_or(0)
                                    as usize,
                            });
                        }
                        let _ = broker_msg_tx.send(BrokerMsg::BarsFetched {
                            source: "kraken".into(),
                            symbol: symbol.clone(),
                            timeframe: timeframe.clone(),
                            count,
                        });
                    }
                    Ok(_) => {
                        let reason = format!("Kraken {} {}: no bars returned", symbol, timeframe);
                        let _ = broker_msg_tx.send(BrokerMsg::OrderResult(reason.clone()));
                        if after_ts.is_none() {
                            let _ = broker_msg_tx.send(BrokerMsg::Unresolvable {
                                broker: "kraken".to_string(),
                                symbol: symbol.clone(),
                                timeframe: timeframe.clone(),
                                reason,
                            });
                        }
                    }
                    Err(e) => {
                        let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
                            "Kraken cache write failed for {} {}: {}",
                            symbol, timeframe, e
                        )));
                    }
                }
            }
        }
        Err(e) => {
            let reason = format!("Kraken fetch failed for {} {}: {}", symbol, timeframe, e);
            if reason.contains("Unsupported symbol") || reason.contains("Unknown asset pair") {
                let _ = broker_msg_tx.send(BrokerMsg::Unresolvable {
                    broker: "kraken".to_string(),
                    symbol: symbol.clone(),
                    timeframe: timeframe.clone(),
                    reason: reason.clone(),
                });
            }
            let _ = broker_msg_tx.send(BrokerMsg::Error(reason));
        }
    }

    let _ = broker_msg_tx.send(BrokerMsg::KrakenFetchSettled { symbol, timeframe });
}

pub(super) async fn run_kraken_futures_fetch_task(
    shared_cache: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
    broker_msg_tx: tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    client: reqwest::Client,
    symbol: String,
    timeframe: String,
) {
    let symbol = typhoon_engine::core::kraken_futures::normalize_futures_symbol(&symbol);
    let timeframe = normalize_sync_timeframe_key(&timeframe)
        .unwrap_or(timeframe.as_str())
        .to_string();
    let cache_key = format!("kraken-futures:{symbol}:{timeframe}");
    let cache_handle = shared_cache.read().ok().and_then(|g| g.clone());
    let incremental = cache_handle
        .as_ref()
        .and_then(|c| c.get_incremental_start(&cache_key).ok().flatten());
    let cached_count = incremental
        .as_ref()
        .map(|(_, count)| *count as i64)
        .unwrap_or(0);
    let mut after_ts = incremental.as_ref().map(|(ts, _)| ts.clone());
    let shallow = kraken_sync_target_bars(&timeframe)
        .map(|target| cached_count > 0 && cached_count * 100 < (target as i64) * 95)
        .unwrap_or(false);

    if shallow {
        after_ts = None;
    }

    let now_ms = chrono::Utc::now().timestamp_millis();
    let start_ms = after_ts
        .as_deref()
        .and_then(|ts| chrono::DateTime::parse_from_rfc3339(ts).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc).timestamp_millis())
        .unwrap_or_else(|| {
            let target = kraken_sync_target_bars(&timeframe).unwrap_or(720) as i64;
            let headroom = (target / 10).max(24);
            let period_s = sync_timeframe_period_secs(&timeframe).unwrap_or(60);
            now_ms.saturating_sub(
                period_s
                    .saturating_mul(1000)
                    .saturating_mul(target.saturating_add(headroom)),
            )
        });
    let log_msg = if incremental.is_some() && shallow {
        format!(
            "Kraken Futures {} {}: limited-history cache ({} bars) — refreshing target window...",
            symbol, timeframe, cached_count
        )
    } else if let Some(ref ts) = after_ts {
        format!(
            "Kraken Futures {} {} delta since {}...",
            symbol,
            timeframe,
            &ts[..19.min(ts.len())]
        )
    } else {
        format!(
            "Kraken Futures {} {}: fetching target window...",
            symbol, timeframe
        )
    };
    let _ = broker_msg_tx.send(BrokerMsg::OrderResult(log_msg));
    match typhoon_engine::core::kraken_futures::fetch_candles(
        &client, &symbol, &timeframe, start_ms, now_ms,
    )
    .await
    {
        Ok(new_bars) => {
            if new_bars.is_empty() && after_ts.is_some() {
                let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                    "Kraken Futures {} {} already up to date",
                    symbol, timeframe
                )));
            } else {
                match store_json_bars_in_cache(
                    cache_handle.clone(),
                    cache_key.clone(),
                    new_bars,
                    after_ts.is_some(),
                )
                .await
                {
                    Ok(count) if count > 0 => {
                        if after_ts.is_none()
                            && kraken_sync_target_bars(&timeframe)
                                .is_some_and(|target| count < target as usize)
                        {
                            let _ = broker_msg_tx.send(BrokerMsg::KrakenFuturesBackfillComplete {
                                symbol: symbol.clone(),
                                timeframe: timeframe.clone(),
                                bar_count: count,
                                target_bars: kraken_sync_target_bars(&timeframe).unwrap_or(0)
                                    as usize,
                            });
                        }
                        let _ = broker_msg_tx.send(BrokerMsg::BarsFetched {
                            source: "kraken-futures".into(),
                            symbol: symbol.clone(),
                            timeframe: timeframe.clone(),
                            count,
                        });
                    }
                    Ok(_) => {
                        let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                            "Kraken Futures {} {}: no bars returned",
                            symbol, timeframe
                        )));
                    }
                    Err(e) => {
                        let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
                            "Kraken Futures cache write failed for {} {}: {}",
                            symbol, timeframe, e
                        )));
                    }
                }
            }
        }
        Err(e) => {
            let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
                "Kraken Futures fetch failed for {} {}: {}",
                symbol, timeframe, e
            )));
        }
    }

    let _ = broker_msg_tx.send(BrokerMsg::KrakenFuturesFetchSettled { symbol, timeframe });
}

enum TastytradeDxTokenAcquire {
    Ready(typhoon_engine::broker::dxlink::DxLinkToken),
    Paused,
}

async fn acquire_tastytrade_dx_token(
    broker: &typhoon_engine::broker::tastytrade::TastytradeBroker,
    dx_token_cache: &Arc<tokio::sync::Mutex<Option<typhoon_engine::broker::dxlink::DxLinkToken>>>,
    dx_backoff_until: &Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
) -> Result<TastytradeDxTokenAcquire, String> {
    let now = std::time::Instant::now();
    if dx_backoff_until
        .lock()
        .await
        .as_ref()
        .is_some_and(|until| *until > now)
    {
        return Ok(TastytradeDxTokenAcquire::Paused);
    }

    let mut token_guard = dx_token_cache.lock().await;
    if let Some(token) = token_guard.clone() {
        return Ok(TastytradeDxTokenAcquire::Ready(token));
    }

    let now = std::time::Instant::now();
    if dx_backoff_until
        .lock()
        .await
        .as_ref()
        .is_some_and(|until| *until > now)
    {
        return Ok(TastytradeDxTokenAcquire::Paused);
    }

    match broker.get_streaming_token().await {
        Ok(token) => {
            *token_guard = Some(token.clone());
            *dx_backoff_until.lock().await = None;
            Ok(TastytradeDxTokenAcquire::Ready(token))
        }
        Err(e) => {
            *dx_backoff_until.lock().await = Some(
                std::time::Instant::now()
                    + std::time::Duration::from_secs(tastytrade_sync_backoff_secs(&e) as u64),
            );
            Err(e)
        }
    }
}

pub(super) async fn run_tastytrade_fetch_task(
    broker: typhoon_engine::broker::tastytrade::TastytradeBroker,
    dx_token_cache: Arc<tokio::sync::Mutex<Option<typhoon_engine::broker::dxlink::DxLinkToken>>>,
    dx_backoff_until: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    shared_cache: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
    broker_msg_tx: tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    symbol: String,
    timeframe: String,
) {
    let symbol = normalize_market_data_symbol(&symbol);
    let timeframe = normalize_sync_timeframe_key(&timeframe)
        .unwrap_or(timeframe.as_str())
        .to_string();
    let bare_upper = symbol.replace('/', "").to_uppercase();
    let mt5_key = format!("mt5:{bare_upper}:{timeframe}");
    let cache_key = format!("tastytrade:{symbol}:{timeframe}");
    let cache_handle = shared_cache.read().ok().and_then(|g| g.clone());
    let mt5_has_bars = cache_handle
        .as_ref()
        .and_then(|c| c.get_incremental_start(&mt5_key).ok().flatten())
        .is_some();
    let incremental = cache_handle
        .as_ref()
        .and_then(|c| c.get_incremental_start(&cache_key).ok().flatten());
    let cached_count = incremental
        .as_ref()
        .map(|(_, count)| *count as i64)
        .unwrap_or(0);
    let mut after_ts = incremental.as_ref().map(|(ts, _)| ts.clone());
    let shallow = tastytrade_sync_target_bars(&timeframe)
        .map(|target| cached_count > 0 && cached_count * 100 < (target as i64) * 95)
        .unwrap_or(false);
    let interval = match timeframe.as_str() {
        "1Min" => "1m",
        "5Min" => "5m",
        "15Min" => "15m",
        "30Min" => "30m",
        "1Hour" => "1h",
        "4Hour" => "4h",
        "1Day" => "1d",
        "1Week" => "1w",
        "1Month" => "1mo",
        _ => "1d",
    };
    let paused_until = dx_backoff_until.lock().await.as_ref().copied();
    if paused_until.is_some_and(|until| until > std::time::Instant::now()) {
        let _ = broker_msg_tx.send(BrokerMsg::TastytradeFetchSettled { symbol, timeframe });
        return;
    }

    if mt5_has_bars {
        let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
            "tastytrade {} {}: MT5/Darwinex has this symbol — skipping",
            symbol, timeframe
        )));
    } else {
        if shallow {
            after_ts = None;
        }
        let from_time_ms = after_ts
            .as_deref()
            .and_then(|ts| chrono::DateTime::parse_from_rfc3339(ts).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc).timestamp_millis())
            .unwrap_or_else(|| {
                tastytrade_initial_from_time_ms(&timeframe, chrono::Utc::now().timestamp_millis())
            });
        let log_msg = if incremental.is_some() && shallow {
            format!(
                "tastytrade {} {}: shallow cache ({} bars) — syncing target depth ({} bars)...",
                symbol,
                timeframe,
                cached_count,
                tastytrade_sync_target_bars(&timeframe).unwrap_or_default()
            )
        } else if let Some(ref ts) = after_ts {
            format!(
                "tastytrade {} {} delta since {} via DXLink...",
                symbol,
                timeframe,
                &ts[..19.min(ts.len())]
            )
        } else {
            format!(
                "tastytrade: fetching {} {} via DXLink (target window)...",
                symbol, timeframe
            )
        };
        let _ = broker_msg_tx.send(BrokerMsg::OrderResult(log_msg));

        let mut last_error: Option<String> = None;
        for attempt in 0..2 {
            let token = match acquire_tastytrade_dx_token(
                &broker,
                &dx_token_cache,
                &dx_backoff_until,
            )
            .await
            {
                Ok(TastytradeDxTokenAcquire::Ready(token)) => token,
                Ok(TastytradeDxTokenAcquire::Paused) => break,
                Err(e) => {
                    last_error = Some(if tastytrade_quote_streamer_customer_missing(&e) {
                        tastytrade_quote_streamer_customer_missing_message("current", &e)
                    } else {
                        format!("DXLink token failed: {e}")
                    });
                    break;
                }
            };
            match typhoon_engine::broker::dxlink::fetch_candles(
                &token,
                &symbol,
                interval,
                from_time_ms,
            )
            .await
            {
                Ok(candles) => {
                    if candles.is_empty() && after_ts.is_some() {
                        let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                            "tastytrade {} {} already up to date",
                            symbol, timeframe
                        )));
                    } else {
                        let new_bars: Vec<serde_json::Value> = candles
                            .iter()
                            .map(|c| {
                                serde_json::json!({
                                    "timestamp": chrono::DateTime::from_timestamp_millis(c.time)
                                        .map(|dt| dt.to_rfc3339())
                                        .unwrap_or_default(),
                                    "open": c.open,
                                    "high": c.high,
                                    "low": c.low,
                                    "close": c.close,
                                    "volume": c.volume,
                                })
                            })
                            .collect();
                        match store_json_bars_in_cache(
                            cache_handle.clone(),
                            cache_key.clone(),
                            new_bars,
                            after_ts.is_some(),
                        )
                        .await
                        {
                            Ok(count) if count > 0 => {
                                let target_bars =
                                    tastytrade_sync_target_bars(&timeframe).unwrap_or(0) as usize;
                                if after_ts.is_none() && target_bars > 0 && count < target_bars {
                                    let _ =
                                        broker_msg_tx.send(BrokerMsg::TastytradeBackfillComplete {
                                            symbol: symbol.clone(),
                                            timeframe: timeframe.clone(),
                                            bar_count: count,
                                            target_bars,
                                        });
                                }
                                let _ = broker_msg_tx.send(BrokerMsg::BarsFetched {
                                    source: "tastytrade".into(),
                                    symbol: symbol.clone(),
                                    timeframe: timeframe.clone(),
                                    count,
                                });
                            }
                            Ok(_) => {
                                let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                                    "tastytrade: no candles for {} {}",
                                    symbol, timeframe
                                )));
                            }
                            Err(e) => {
                                let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
                                    "tastytrade cache write failed for {} {}: {}",
                                    symbol, timeframe, e
                                )));
                            }
                        }
                    }
                    last_error = None;
                    break;
                }
                Err(e) => {
                    last_error = Some(format!("DXLink stream failed: {e}"));
                    if attempt == 0 {
                        let mut guard = dx_token_cache.lock().await;
                        *guard = None;
                        continue;
                    }
                }
            }
        }
        if let Some(error) = last_error {
            *dx_backoff_until.lock().await = Some(
                std::time::Instant::now()
                    + std::time::Duration::from_secs(tastytrade_sync_backoff_secs(&error) as u64),
            );
            let _ = broker_msg_tx.send(BrokerMsg::Error(error));
        }
    }

    let _ = broker_msg_tx.send(BrokerMsg::TastytradeFetchSettled { symbol, timeframe });
}
