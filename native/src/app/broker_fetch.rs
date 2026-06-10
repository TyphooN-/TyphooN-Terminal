use super::*;

fn alpaca_batch_missing_symbol_retry_reason(
    outcome: typhoon_engine::broker::alpaca::FetchOutcome,
) -> Option<&'static str> {
    match outcome {
        typhoon_engine::broker::alpaca::FetchOutcome::RateLimitedPartial => {
            Some("batch_rate_limited_partial")
        }
        typhoon_engine::broker::alpaca::FetchOutcome::RateLimitedEmpty => {
            Some("batch_rate_limited_empty")
        }
        typhoon_engine::broker::alpaca::FetchOutcome::Complete => None,
    }
}

pub(super) async fn run_alpaca_batch_fetch_task(
    broker: AlpacaBroker,
    shared_cache: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
    broker_msg_tx: tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    symbols: Vec<String>,
    timeframe: String,
) {
    let Some(tf) = normalize_sync_timeframe_key(&timeframe) else {
        return;
    };
    let timeframe = tf.to_string();
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
    let symbols: Vec<String> = symbols
        .into_iter()
        .map(|symbol| normalize_market_data_symbol(&symbol).replace('/', ""))
        .filter(|symbol| !symbol.is_empty())
        .collect();
    if symbols.is_empty() {
        return;
    }
    let limit = alpaca_sync_target_bars(&timeframe)
        .unwrap_or(1500)
        .min(10_000);
    let result = broker
        .get_stock_bars_batch_targeted(&symbols, tf_alpaca, limit)
        .await;
    match result {
        Ok((mut bars_by_symbol, outcome)) => {
            if matches!(
                outcome,
                typhoon_engine::broker::alpaca::FetchOutcome::RateLimitedEmpty
            ) {
                for symbol in symbols {
                    let _ = broker_msg_tx.send(BrokerMsg::AlpacaRetryEnqueue {
                        symbol: symbol.clone(),
                        timeframe: timeframe.clone(),
                        reason: "batch_rate_limited_empty".into(),
                    });
                    let _ = broker_msg_tx.send(BrokerMsg::AlpacaFetchSettled {
                        symbol,
                        timeframe: timeframe.clone(),
                        success: false,
                    });
                }
                return;
            }
            for symbol in &symbols {
                let new_bars = bars_by_symbol.remove(symbol).unwrap_or_default();
                if new_bars.is_empty() {
                    // A successful multi-symbol response can omit symbols that have
                    // no rows for that timeframe/window. Do not explode a broad
                    // batch omission into thousands of targeted probes; the broad
                    // scheduler's fetch cooldown is enough to retry later, while
                    // focused symbols already use the single-symbol fetch path.
                    if let Some(reason) = alpaca_batch_missing_symbol_retry_reason(outcome) {
                        let _ = broker_msg_tx.send(BrokerMsg::AlpacaRetryEnqueue {
                            symbol: symbol.clone(),
                            timeframe: timeframe.clone(),
                            reason: reason.into(),
                        });
                    }
                    let _ = broker_msg_tx.send(BrokerMsg::AlpacaFetchSettled {
                        symbol: symbol.clone(),
                        timeframe: timeframe.clone(),
                        success: false,
                    });
                    continue;
                }
                let cache_key = format!("alpaca:{symbol}:{timeframe}");
                let bars: Vec<serde_json::Value> = new_bars
                    .into_iter()
                    .filter_map(|bar| serde_json::to_value(bar).ok())
                    .collect();
                match store_json_bars_in_cache(
                    shared_cache.read().ok().and_then(|g| g.clone()),
                    cache_key,
                    bars,
                    true,
                )
                .await
                {
                    Ok(count) if count > 0 => {
                        if matches!(
                            outcome,
                            typhoon_engine::broker::alpaca::FetchOutcome::Complete
                        ) {
                            let _ = broker_msg_tx.send(BrokerMsg::AlpacaBackfillComplete {
                                symbol: symbol.clone(),
                                timeframe: timeframe.clone(),
                                bar_count: count,
                                target_bars: count,
                            });
                        }
                        let _ = broker_msg_tx.send(BrokerMsg::BarsFetched {
                            source: "alpaca".into(),
                            symbol: symbol.clone(),
                            timeframe: timeframe.clone(),
                            count,
                        });
                    }
                    Ok(_) => {}
                    Err(e) => {
                        let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
                            "Alpaca batch cache write failed for {} {}: {}",
                            symbol, timeframe, e
                        )));
                    }
                }
                if matches!(
                    outcome,
                    typhoon_engine::broker::alpaca::FetchOutcome::RateLimitedPartial
                ) {
                    let _ = broker_msg_tx.send(BrokerMsg::AlpacaRetryEnqueue {
                        symbol: symbol.clone(),
                        timeframe: timeframe.clone(),
                        reason: "batch_rate_limited_partial".into(),
                    });
                }
                let _ = broker_msg_tx.send(BrokerMsg::AlpacaFetchSettled {
                    symbol: symbol.clone(),
                    timeframe: timeframe.clone(),
                    success: matches!(
                        outcome,
                        typhoon_engine::broker::alpaca::FetchOutcome::Complete
                    ),
                });
            }
        }
        Err(e) => {
            let is_rate = e.contains("429") || e.to_lowercase().contains("rate limit");
            for symbol in symbols {
                if is_rate {
                    let _ = broker_msg_tx.send(BrokerMsg::AlpacaRetryEnqueue {
                        symbol: symbol.clone(),
                        timeframe: timeframe.clone(),
                        reason: format!("batch_err:{e}"),
                    });
                } else {
                    let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
                        "Alpaca batch fetch failed for {} {}: {}",
                        symbol, timeframe, e
                    )));
                }
                let _ = broker_msg_tx.send(BrokerMsg::AlpacaFetchSettled {
                    symbol,
                    timeframe: timeframe.clone(),
                    success: false,
                });
            }
        }
    }
}

pub(super) async fn run_alpaca_fetch_task(
    broker: AlpacaBroker,
    shared_cache: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
    broker_msg_tx: tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    symbol: String,
    timeframe: String,
    backfill_already_complete: bool,
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

    let cache_key = format!("alpaca:{symbol}:{timeframe}");
    let cache_handle = shared_cache.read().ok().and_then(|g| g.clone());

    let incremental = cache_handle
        .as_ref()
        .and_then(|c| c.get_incremental_start(&cache_key).ok().flatten());
    let cached_count = incremental
        .as_ref()
        .map(|(_, count)| *count as i64)
        .unwrap_or(0);
    let mut after_ts = incremental.as_ref().map(|(ts, _)| ts.clone());
    let needs_backfill = should_request_full_backfill(
        backfill_already_complete,
        alpaca_sync_target_bars(&timeframe),
        cached_count,
    );

    let mut success = false;
    {
        if needs_backfill {
            after_ts = None;
        }

        let result = if after_ts.is_none() {
            let msg = if incremental.is_some() && needs_backfill {
                format!(
                    "Alpaca {} {}: cache has {} bars — syncing full server history...",
                    api_symbol, timeframe, cached_count
                )
            } else {
                format!(
                    "Alpaca {} {}: fetching full server history (first sync)...",
                    api_symbol, timeframe
                )
            };
            let _ = broker_msg_tx.send(BrokerMsg::OrderResult(msg));
            broker.get_all_bars(&api_symbol, tf_alpaca, None).await
        } else {
            let ts = after_ts.as_deref().expect("delta branch requires after_ts");
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
                } else {
                    let bars: Vec<serde_json::Value> = new_bars
                        .into_iter()
                        .filter_map(|bar| serde_json::to_value(bar).ok())
                        .collect();
                    match store_json_bars_in_cache(
                        cache_handle.clone(),
                        cache_key.clone(),
                        bars,
                        after_ts.is_some(),
                    )
                    .await
                    {
                        Ok(count) if count > 0 => {
                            if after_ts.is_none()
                                && matches!(
                                    outcome,
                                    typhoon_engine::broker::alpaca::FetchOutcome::Complete
                                )
                            {
                                let _ = broker_msg_tx.send(BrokerMsg::AlpacaBackfillComplete {
                                    symbol: symbol.clone(),
                                    timeframe: timeframe.clone(),
                                    bar_count: count,
                                    target_bars: count,
                                });
                            }
                            let _ = broker_msg_tx.send(BrokerMsg::BarsFetched {
                                source: "alpaca".into(),
                                symbol: symbol.clone(),
                                timeframe: timeframe.clone(),
                                count,
                            });
                        }
                        Ok(_) => {
                            let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                                "Alpaca {} {}: no bars returned",
                                symbol, timeframe
                            )));
                        }
                        Err(e) => {
                            let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
                                "Alpaca cache write failed for {} {}: {}",
                                symbol, timeframe, e
                            )));
                        }
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
                    "Alpaca fetch bars failed for {} {}: {}",
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


fn should_request_full_backfill(
    backfill_already_complete: bool,
    target_bars: Option<u32>,
    cached_count: i64,
) -> bool {
    !backfill_already_complete
        && target_bars
            .map(|target| cached_count > 0 && (cached_count as i128) * 100 < (target as i128) * 95)
            .unwrap_or(false)
}

#[allow(dead_code)]
fn merged_equity_materialize_target_from_cache_key(cache_key: &str) -> Option<(String, String)> {
    let mut parts = cache_key.splitn(3, ':');
    let source = parts.next()?;
    if !matches!(
        source,
        "kraken-equities" | "alpaca" | "yahoo-chart" | "default"
    ) {
        return None;
    }
    let symbol = parts.next()?.trim();
    let timeframe = normalize_sync_timeframe_key(parts.next()?.trim())?;
    let symbol = normalize_market_data_symbol(symbol)
        .replace('/', "")
        .trim_end_matches(".EQ")
        .to_ascii_uppercase();
    if symbol.is_empty() {
        return None;
    }
    Some((symbol, timeframe.to_string()))
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
        let count = if merge_existing {
            let json = serde_json::to_string(&bars).unwrap_or_default();
            let merged_json = cache.merge_bars(&cache_key, &json, 0)?;
            serde_json::from_str::<Vec<serde_json::Value>>(&merged_json)
                .map(|merged| merged.len())
                .unwrap_or(0)
        } else {
            let json = serde_json::to_string(&bars).unwrap_or_default();
            cache.put_bars(&cache_key, &json)?;
            cache
                .get_bars_raw(&cache_key)
                .ok()
                .flatten()
                .map(|raw| raw.len())
                .unwrap_or(0)
        };
        if count > 0 {
            // Do NOT materialize merged on every provider write.
            // Materialization is expensive (full rebuild + persist) and causes
            // massive temporary allocations + CPU spikes during broad sync.
            // Let chart reads trigger on-demand materialization instead.
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
    backfill_already_complete: bool,
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
    let needs_backfill = should_request_full_backfill(
        backfill_already_complete,
        kraken_sync_target_bars(&timeframe),
        cached_count,
    );

    if needs_backfill {
        after_ts = None;
    }
    let now_ms = chrono::Utc::now().timestamp_millis();
    let start_ms = after_ts
        .as_deref()
        .and_then(|ts| chrono::DateTime::parse_from_rfc3339(ts).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc).timestamp_millis())
        .unwrap_or_else(|| {
            let target = kraken_sync_target_bars(&timeframe)
                .unwrap_or(KRAKEN_SPOT_PROVIDER_WINDOW_BARS) as i64;
            let headroom = (target / 10).max(24);
            let period_s = sync_timeframe_period_secs(&timeframe).unwrap_or(60);
            now_ms.saturating_sub(
                period_s
                    .saturating_mul(1000)
                    .saturating_mul(target.saturating_add(headroom)),
            )
        });
    let log_msg = if incremental.is_some() && needs_backfill {
        format!(
            "Kraken {} {}: provider-window cache ({} bars) — refreshing recent window...",
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
    backfill_already_complete: bool,
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
    let needs_backfill = should_request_full_backfill(
        backfill_already_complete,
        kraken_futures_sync_target_bars(&timeframe),
        cached_count,
    );

    if needs_backfill {
        after_ts = None;
    }

    let now_ms = chrono::Utc::now().timestamp_millis();
    let start_ms = after_ts
        .as_deref()
        .and_then(|ts| chrono::DateTime::parse_from_rfc3339(ts).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc).timestamp_millis())
        .unwrap_or_else(|| {
            chrono::NaiveDate::from_ymd_opt(2018, 1, 1)
                .and_then(|d| d.and_hms_opt(0, 0, 0))
                .map(|ndt| ndt.and_utc().timestamp_millis())
                .unwrap_or(0)
        });
    let log_msg = if incremental.is_some() && needs_backfill {
        format!(
            "Kraken Futures {} {}: cache has {} bars — syncing full server history...",
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
            "Kraken Futures {} {}: fetching full server history...",
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
                        if after_ts.is_none() {
                            let _ = broker_msg_tx.send(BrokerMsg::KrakenFuturesBackfillComplete {
                                symbol: symbol.clone(),
                                timeframe: timeframe.clone(),
                                bar_count: count,
                                target_bars: count,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_backfill_needed_when_cached_dataset_is_below_target_and_not_complete() {
        assert!(should_request_full_backfill(false, Some(1_000), 949));
        assert!(should_request_full_backfill(false, Some(u32::MAX), 10_000));
    }

    #[test]
    fn full_backfill_not_needed_without_existing_cache_or_target() {
        assert!(!should_request_full_backfill(false, Some(1_000), 0));
        assert!(!should_request_full_backfill(false, None, 100));
    }

    #[test]
    fn full_backfill_not_needed_when_cached_dataset_reaches_threshold() {
        assert!(!should_request_full_backfill(false, Some(1_000), 950));
        assert!(!should_request_full_backfill(false, Some(1_000), 1_000));
    }

    #[test]
    fn backfill_complete_marker_forces_incremental_even_for_thin_history() {
        assert!(!should_request_full_backfill(true, Some(1_000), 1));
        assert!(!should_request_full_backfill(true, Some(u32::MAX), 10_000));
    }

    #[test]
    fn successful_batch_omissions_do_not_spawn_targeted_retry_storms() {
        assert_eq!(
            alpaca_batch_missing_symbol_retry_reason(
                typhoon_engine::broker::alpaca::FetchOutcome::Complete
            ),
            None
        );
        assert_eq!(
            alpaca_batch_missing_symbol_retry_reason(
                typhoon_engine::broker::alpaca::FetchOutcome::RateLimitedPartial
            ),
            Some("batch_rate_limited_partial")
        );
        assert_eq!(
            alpaca_batch_missing_symbol_retry_reason(
                typhoon_engine::broker::alpaca::FetchOutcome::RateLimitedEmpty
            ),
            Some("batch_rate_limited_empty")
        );
    }
}
