use super::*;

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
        Ok((bars_by_symbol, outcome)) => {
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
                let new_bars = bars_by_symbol.get(symbol).cloned().unwrap_or_default();
                if new_bars.is_empty() {
                    // Alpaca's multi-symbol endpoint may omit a symbol from an otherwise
                    // successful batch even when the symbol is valid/tradable. Treat that
                    // as inconclusive and let the retry queue perform a targeted
                    // single-symbol probe before any persistent no-data tombstone is
                    // written. Targeted fetches still own the real no-data decision.
                    let reason = if matches!(
                        outcome,
                        typhoon_engine::broker::alpaca::FetchOutcome::Complete
                    ) {
                        "batch_symbol_omitted_probe_required"
                    } else {
                        "batch_rate_limited_partial"
                    };
                    let _ = broker_msg_tx.send(BrokerMsg::AlpacaRetryEnqueue {
                        symbol: symbol.clone(),
                        timeframe: timeframe.clone(),
                        reason: reason.into(),
                    });
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
    let needs_backfill = should_request_full_backfill(
        backfill_already_complete,
        alpaca_sync_target_bars(&timeframe),
        cached_count,
    );

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
                                "No bars returned for {} {}",
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

pub(super) fn cryptocompare_backfill_symbol(symbol: &str) -> Option<String> {
    let symbol = typhoon_engine::core::kraken::normalize_pair_symbol(symbol);
    if symbol.is_empty() || symbol.contains(".EQ") {
        return None;
    }
    const FIAT: &[&str] = &["USD", "EUR", "GBP", "CAD", "AUD", "JPY", "CHF"];
    const USD_QUOTES: &[&str] = &["USDG", "USDT", "USDC", "USD"];
    for quote in USD_QUOTES {
        if let Some(base) = symbol.strip_suffix(quote) {
            if !base.is_empty() && !FIAT.contains(&base) {
                return Some(format!("{base}USD"));
            }
        }
    }
    None
}

fn cryptocompare_backfill_floor_ms() -> i64 {
    chrono::NaiveDate::from_ymd_opt(2010, 1, 1)
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|ndt| ndt.and_utc().timestamp_millis())
        .unwrap_or(0)
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
    backfill_already_complete: bool,
    cryptocompare_backfill_enabled: bool,
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
    let mut cryptocompare_backfill = Vec::new();
    if after_ts.is_none() && cryptocompare_backfill_enabled {
        if let Some(cc_symbol) = cryptocompare_backfill_symbol(&symbol) {
            if let Some(secs) = typhoon_engine::core::cryptocompare::rate_limited_for_secs() {
                let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                    "CryptoCompare backfill skipped for {} {}: rate-limit backoff {}s; using Kraken provider window",
                    symbol, timeframe, secs
                )));
            } else {
                let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                    "CryptoCompare backfill for {} {} before Kraken recent window...",
                    symbol, timeframe
                )));
                match typhoon_engine::core::cryptocompare::fetch_ohlcv(
                    &client,
                    &cc_symbol,
                    &timeframe,
                    cryptocompare_backfill_floor_ms(),
                    now_ms,
                )
                .await
                {
                    Ok(bars) => {
                        if !bars.is_empty() {
                            let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                                "CryptoCompare backfill for {} {} returned {} bars",
                                symbol,
                                timeframe,
                                bars.len()
                            )));
                            cryptocompare_backfill = bars;
                        }
                    }
                    Err(e) => {
                        let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                            "CryptoCompare backfill skipped for {} {}: {}; using Kraken provider window",
                            symbol, timeframe, e
                        )));
                    }
                }
            }
        }
    }
    match typhoon_engine::core::kraken::fetch_binance_klines(
        &client, &symbol, &timeframe, start_ms, now_ms,
    )
    .await
    {
        Ok(mut new_bars) => {
            if !cryptocompare_backfill.is_empty() {
                new_bars = merge_json_bars(cryptocompare_backfill, new_bars);
            }
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
    backfill_already_complete: bool,
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
    let needs_backfill = should_request_full_backfill(
        backfill_already_complete,
        tastytrade_sync_target_bars(&timeframe),
        cached_count,
    );
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
        if needs_backfill {
            after_ts = None;
        }
        let from_time_ms = after_ts
            .as_deref()
            .and_then(|ts| chrono::DateTime::parse_from_rfc3339(ts).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc).timestamp_millis())
            .unwrap_or_else(|| {
                tastytrade_initial_from_time_ms(&timeframe, chrono::Utc::now().timestamp_millis())
            });
        let log_msg = if incremental.is_some() && needs_backfill {
            format!(
                "tastytrade {} {}: cache has {} bars — syncing full DXLink history...",
                symbol, timeframe, cached_count
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
                "tastytrade: fetching {} {} full DXLink history...",
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
            let mut page_from_time_ms = from_time_ms;
            let mut all_candles = Vec::new();
            let mut status = typhoon_engine::broker::dxlink::DxSnapshotStatus::TimedOut;
            let mut paging_error: Option<String> = None;
            let period_ms = sync_timeframe_period_secs(&timeframe)
                .unwrap_or(60)
                .saturating_mul(1000);
            let max_pages = 2_048usize;
            let mut page_guard_exhausted = false;
            for page in 0..max_pages {
                match typhoon_engine::broker::dxlink::fetch_candles_with_status(
                    &token,
                    &symbol,
                    interval,
                    page_from_time_ms,
                )
                .await
                {
                    Ok(fetch) => {
                        if fetch.candles.is_empty() {
                            status = fetch.status;
                            break;
                        }
                        let last_time = fetch
                            .candles
                            .iter()
                            .map(|c| c.time)
                            .max()
                            .unwrap_or(page_from_time_ms);
                        all_candles.extend(fetch.candles);
                        status = fetch.status;
                        if status != typhoon_engine::broker::dxlink::DxSnapshotStatus::Snipped {
                            break;
                        }
                        if page + 1 >= max_pages {
                            page_guard_exhausted = true;
                            break;
                        }
                        let next_from = last_time.saturating_add(period_ms.max(1));
                        if next_from <= page_from_time_ms {
                            break;
                        }
                        page_from_time_ms = next_from;
                    }
                    Err(e) => {
                        paging_error = Some(e);
                        break;
                    }
                }
            }
            let candle_result = match paging_error {
                Some(e) => Err(e),
                None => Ok((all_candles, status, page_guard_exhausted)),
            };
            match candle_result {
                Ok((candles, status, page_guard_exhausted)) => {
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
                                if after_ts.is_none()
                                    && (status
                                        == typhoon_engine::broker::dxlink::DxSnapshotStatus::Complete
                                        || page_guard_exhausted)
                                {
                                    if page_guard_exhausted {
                                        let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                                            "tastytrade {} {}: DXLink snapshot still snipped after {} pages; stored guarded maximum history and automated sync will keep it current",
                                            symbol, timeframe, max_pages
                                        )));
                                    }
                                    let _ =
                                        broker_msg_tx.send(BrokerMsg::TastytradeBackfillComplete {
                                            symbol: symbol.clone(),
                                            timeframe: timeframe.clone(),
                                            bar_count: count,
                                            target_bars: count,
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
}
