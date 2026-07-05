use std::sync::Arc;

use crate::broker::alpaca::AlpacaBroker;
use crate::broker::protocol::BrokerMsg;
use crate::core::cache::SqliteCache;

const KRAKEN_SPOT_PROVIDER_WINDOW_BARS: u32 = 720;
const STANDARD_SYNC_TIMEFRAMES: [(&str, &str); 9] = [
    ("M1", "1Min"),
    ("M5", "5Min"),
    ("M15", "15Min"),
    ("M30", "30Min"),
    ("H1", "1Hour"),
    ("H4", "4Hour"),
    ("D1", "1Day"),
    ("W1", "1Week"),
    ("MN1", "1Month"),
];

fn bare_symbol_from_key(key: &str) -> String {
    let parts: Vec<&str> = key.split(':').collect();
    match parts.as_slice() {
        [_src, sym, _tf] => (*sym).to_string(),
        [sym, _tf] => (*sym).to_string(),
        _ => key.to_string(),
    }
}

fn normalize_market_data_symbol(symbol: &str) -> String {
    let bare = bare_symbol_from_key(symbol).to_uppercase();
    match bare.rsplit_once('.') {
        Some((head, suffix))
            if (2..=4).contains(&suffix.len())
                && suffix.chars().all(|c| c.is_ascii_uppercase()) =>
        {
            head.to_string()
        }
        _ => bare,
    }
}

fn normalize_sync_timeframe_key(tf: &str) -> Option<&'static str> {
    STANDARD_SYNC_TIMEFRAMES.iter().find_map(|(short, cache)| {
        if tf.eq_ignore_ascii_case(short) || tf.eq_ignore_ascii_case(cache) {
            Some(*cache)
        } else {
            None
        }
    })
}

fn sync_timeframe_period_secs(tf: &str) -> Option<i64> {
    match normalize_sync_timeframe_key(tf)? {
        "1Min" => Some(60),
        "5Min" => Some(5 * 60),
        "15Min" => Some(15 * 60),
        "30Min" => Some(30 * 60),
        "1Hour" => Some(60 * 60),
        "4Hour" => Some(4 * 60 * 60),
        "1Day" => Some(24 * 60 * 60),
        "1Week" => Some(7 * 24 * 60 * 60),
        "1Month" => Some(30 * 24 * 60 * 60),
        _ => None,
    }
}

fn alpaca_sync_target_bars(tf: &str) -> Option<u32> {
    match normalize_sync_timeframe_key(tf)? {
        "1Min" | "5Min" => None,
        _ => Some(u32::MAX),
    }
}

fn alpaca_incremental_fetch_limit_at(
    now_s: i64,
    timeframe: &str,
    after_timestamp: Option<&str>,
) -> u32 {
    let Some(after_ts) = after_timestamp else {
        return 1000;
    };
    let Some(period_s) = sync_timeframe_period_secs(timeframe) else {
        return 1000;
    };
    let parsed = match chrono::DateTime::parse_from_rfc3339(after_ts) {
        Ok(dt) => dt.with_timezone(&chrono::Utc),
        Err(_) => return 1000,
    };
    let age_s = now_s.saturating_sub(parsed.timestamp()).max(0);
    let gap_bars = ((age_s + period_s - 1) / period_s).max(1) as u32;
    let headroom = (gap_bars / 2).max(8);
    gap_bars.saturating_add(headroom).clamp(32, 1000)
}

fn alpaca_incremental_fetch_limit(timeframe: &str, after_timestamp: Option<&str>) -> u32 {
    alpaca_incremental_fetch_limit_at(chrono::Utc::now().timestamp(), timeframe, after_timestamp)
}

fn kraken_spot_native_timeframe(tf: &str) -> bool {
    matches!(
        tf,
        "1Min" | "5Min" | "15Min" | "30Min" | "1Hour" | "4Hour" | "1Day" | "1Week"
    )
}

fn kraken_sync_target_bars(tf: &str) -> Option<u32> {
    let tf = normalize_sync_timeframe_key(tf)?;
    kraken_spot_native_timeframe(tf).then_some(KRAKEN_SPOT_PROVIDER_WINDOW_BARS)
}

fn kraken_futures_sync_target_bars(tf: &str) -> Option<u32> {
    normalize_sync_timeframe_key(tf).map(|_| u32::MAX)
}

fn alpaca_batch_missing_symbol_retry_reason(
    outcome: crate::broker::alpaca::FetchOutcome,
) -> Option<&'static str> {
    match outcome {
        crate::broker::alpaca::FetchOutcome::RateLimitedPartial => {
            Some("batch_rate_limited_partial")
        }
        crate::broker::alpaca::FetchOutcome::RateLimitedEmpty => Some("batch_rate_limited_empty"),
        crate::broker::alpaca::FetchOutcome::Complete => None,
    }
}

/// Depth (bars) at and above which a batch request means "full provider
/// history". Only at this depth is a Complete-outcome symbol omission
/// authoritative evidence that Alpaca has no rows at all for the pair —
/// a shallow top-up window can legitimately miss an illiquid symbol.
const ALPACA_BATCH_DEEP_HISTORY_BARS: u32 = 10_000;

pub async fn run_alpaca_batch_fetch_task(
    broker: AlpacaBroker,
    shared_cache: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
    broker_msg_tx: tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    symbols: Vec<String>,
    timeframe: String,
    lookback_bars: u32,
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
    let lookback_bars = lookback_bars.clamp(1, ALPACA_BATCH_DEEP_HISTORY_BARS);
    let deep_window = lookback_bars >= ALPACA_BATCH_DEEP_HISTORY_BARS;
    let result = broker
        .get_stock_bars_batch_targeted(&symbols, tf_alpaca, lookback_bars)
        .await;
    match result {
        Ok((mut bars_by_symbol, outcome)) => {
            if matches!(
                outcome,
                crate::broker::alpaca::FetchOutcome::RateLimitedEmpty
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
                    } else if deep_window {
                        // Complete outcome over the full-history window: Alpaca
                        // definitively has no rows for this pair. Tombstone it,
                        // or it stays an eternal Missing candidate the scheduler
                        // re-selects every tick (and, with multi-day cooldowns
                        // blocking dispatch, wedges the whole timeframe's
                        // high-TF-first descent). A future successful fetch via
                        // any path drains the tombstone.
                        let _ = broker_msg_tx.send(BrokerMsg::AlpacaNoData {
                            symbol: symbol.clone(),
                            timeframe: timeframe.clone(),
                            reason: "batch full-history window returned no rows".into(),
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
                        if matches!(outcome, crate::broker::alpaca::FetchOutcome::Complete) {
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
                    crate::broker::alpaca::FetchOutcome::RateLimitedPartial
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
                    success: matches!(outcome, crate::broker::alpaca::FetchOutcome::Complete),
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

pub async fn run_alpaca_fetch_task(
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

    // Self-heal a stalled native 1H/4H. Alpaca's META 1Hour/4Hour stopped at
    // 2024-01-25 while its 15Min kept printing to today, leaving an 860-day hole
    // the count target can't see (6302 bars — just the wrong, pre-2024 ones) and
    // a single incremental delta (≤1000 bars ≈ 150 days) can't reach. If the
    // SAME symbol's 15Min tail is far fresher than this series' tail, Alpaca has
    // fillable history, so re-pull full server history once. Comparing against
    // 15Min makes it self-limiting: once this series catches up the gap closes
    // and it stops; a symbol Alpaca has no recent data for never triggers (no
    // re-pull loop, no persisted marker needed).
    let force_intraday_heal = if matches!(timeframe.as_str(), "1Hour" | "4Hour") {
        let parse_s = |ts: &str| {
            chrono::DateTime::parse_from_rfc3339(ts)
                .ok()
                .map(|dt| dt.timestamp())
        };
        let fine_tail_s = cache_handle
            .as_ref()
            .and_then(|c| {
                c.get_incremental_start(&format!("alpaca:{symbol}:15Min"))
                    .ok()
                    .flatten()
            })
            .and_then(|(ts, _)| parse_s(&ts));
        let this_tail_s = after_ts.as_deref().and_then(parse_s);
        intraday_stall_needs_full_pull(fine_tail_s, this_tail_s)
    } else {
        false
    };

    let needs_backfill = force_intraday_heal
        || should_request_full_backfill(
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
                success = matches!(outcome, crate::broker::alpaca::FetchOutcome::Complete);
                if new_bars.is_empty()
                    && after_ts.is_some()
                    && matches!(outcome, crate::broker::alpaca::FetchOutcome::Complete)
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
                                && matches!(outcome, crate::broker::alpaca::FetchOutcome::Complete)
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
                    crate::broker::alpaca::FetchOutcome::RateLimitedPartial => {
                        let _ = broker_msg_tx.send(BrokerMsg::AlpacaRetryEnqueue {
                            symbol: symbol.clone(),
                            timeframe: timeframe.clone(),
                            reason: "rate_limited_partial".into(),
                        });
                    }
                    crate::broker::alpaca::FetchOutcome::RateLimitedEmpty => {
                        let _ = broker_msg_tx.send(BrokerMsg::AlpacaRetryEnqueue {
                            symbol: symbol.clone(),
                            timeframe: timeframe.clone(),
                            reason: "rate_limited_empty".into(),
                        });
                    }
                    crate::broker::alpaca::FetchOutcome::Complete => {}
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

/// Whether a native 1H/4H series should be re-pulled in full because it stalled
/// while the same symbol's 15Min feed kept printing. `fine_tail_s` is the 15Min
/// tail epoch-seconds; `this_tail_s` is the 1H/4H tail (None = no native series).
/// Comparing the two makes the heal self-limiting — it fires only while 15Min is
/// ≥30 days fresher, so a healed series stops triggering and a symbol Alpaca has
/// no recent data for never triggers.
fn intraday_stall_needs_full_pull(fine_tail_s: Option<i64>, this_tail_s: Option<i64>) -> bool {
    const STALL_GAP_S: i64 = 30 * 86_400;
    match (fine_tail_s, this_tail_s) {
        (Some(fine), Some(this)) => fine - this > STALL_GAP_S,
        (Some(_), None) => true, // have 15Min but no native 1H/4H ⇒ pull the base once
        _ => false,
    }
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

            // Derive window-safe higher timeframes from this base write so the
            // scheduler doesn't spend a separate rate-limited request on each — the
            // only real speedup against fixed provider rate walls. Assist-priority
            // and no-op for native crypto lanes / non-base timeframes.
            let _derived = cache.derive_and_store_higher_tfs(&cache_key);
        }
        Ok::<usize, String>(count)
    })
    .await
    .map_err(|e| format!("cache write task failed: {e}"))?
}

pub async fn run_kraken_fetch_task(
    shared_cache: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
    broker_msg_tx: tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    client: reqwest::Client,
    symbol: String,
    timeframe: String,
    backfill_already_complete: bool,
) {
    let symbol = crate::core::kraken::normalize_pair_symbol(&symbol);
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
    match crate::core::kraken::fetch_binance_klines(&client, &symbol, &timeframe, start_ms, now_ms)
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

pub async fn run_kraken_futures_fetch_task(
    shared_cache: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
    broker_msg_tx: tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    client: reqwest::Client,
    symbol: String,
    timeframe: String,
    backfill_already_complete: bool,
) {
    let symbol = crate::core::kraken_futures::normalize_futures_symbol(&symbol);
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
    match crate::core::kraken_futures::fetch_candles(&client, &symbol, &timeframe, start_ms, now_ms)
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
    use crate::core::cache::{
        aggregate_raw_to_rollup, source_derives_higher_tfs, tf_derivations, Rollup,
    };

    #[test]
    fn tf_derivation_eligibility_is_assist_lanes_and_base_timeframes_only() {
        assert_eq!(tf_derivations("15Min"), &[("30Min", Rollup::Fixed(1_800))]);
        assert_eq!(tf_derivations("1Hour"), &[("4Hour", Rollup::Fixed(14_400))]);
        assert_eq!(
            tf_derivations("1Day"),
            &[("1Week", Rollup::Week), ("1Month", Rollup::Month)]
        );
        assert!(tf_derivations("30Min").is_empty());
        assert!(tf_derivations("4Hour").is_empty());
        assert!(source_derives_higher_tfs("alpaca"));
        assert!(source_derives_higher_tfs("yahoo-chart"));
        assert!(source_derives_higher_tfs("kraken-equities"));
        // Native crypto lanes already return every timeframe — never derived.
        assert!(!source_derives_higher_tfs("kraken"));
        assert!(!source_derives_higher_tfs("kraken-futures"));
    }

    #[test]
    fn aggregate_1d_to_weekly_buckets_on_monday_with_correct_ohlcv() {
        // 2024-01-01 is a Monday. Three daily bars in week 1, two in week 2.
        let d = 86_400_000i64;
        let mon1 = 1_704_067_200_000i64; // 2024-01-01 00:00 UTC
        let raw = vec![
            (mon1, 10.0, 12.0, 9.0, 11.0, 100.0),         // Mon Jan 1
            (mon1 + d, 11.0, 13.0, 10.0, 12.0, 110.0),    // Tue Jan 2
            (mon1 + 2 * d, 12.0, 14.0, 11.0, 13.0, 120.0), // Wed Jan 3
            (mon1 + 7 * d, 20.0, 22.0, 19.0, 21.0, 200.0), // Mon Jan 8
            (mon1 + 8 * d, 21.0, 23.0, 20.0, 22.0, 210.0), // Tue Jan 9
        ];
        let out = aggregate_raw_to_rollup(&raw, Rollup::Week);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0]["timestamp"].as_str().unwrap(), "2024-01-01T00:00:00+00:00");
        assert_eq!(out[0]["open"].as_f64().unwrap(), 10.0);
        assert_eq!(out[0]["high"].as_f64().unwrap(), 14.0);
        assert_eq!(out[0]["low"].as_f64().unwrap(), 9.0);
        assert_eq!(out[0]["close"].as_f64().unwrap(), 13.0);
        assert_eq!(out[0]["volume"].as_f64().unwrap(), 330.0);
        assert_eq!(out[1]["timestamp"].as_str().unwrap(), "2024-01-08T00:00:00+00:00");
        assert_eq!(out[1]["close"].as_f64().unwrap(), 22.0);
        assert_eq!(out[1]["volume"].as_f64().unwrap(), 410.0);
    }

    #[test]
    fn aggregate_1d_to_monthly_buckets_on_first_of_month() {
        let d = 86_400_000i64;
        let jan1 = 1_704_067_200_000i64; // 2024-01-01 00:00 UTC
        let raw = vec![
            (jan1, 10.0, 12.0, 9.0, 11.0, 100.0),          // Jan 1
            (jan1 + 30 * d, 15.0, 16.0, 14.0, 15.0, 150.0), // Jan 31
            (jan1 + 31 * d, 20.0, 22.0, 19.0, 21.0, 200.0), // Feb 1
        ];
        let out = aggregate_raw_to_rollup(&raw, Rollup::Month);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0]["timestamp"].as_str().unwrap(), "2024-01-01T00:00:00+00:00");
        assert_eq!(out[0]["open"].as_f64().unwrap(), 10.0);
        assert_eq!(out[0]["high"].as_f64().unwrap(), 16.0);
        assert_eq!(out[0]["close"].as_f64().unwrap(), 15.0);
        assert_eq!(out[0]["volume"].as_f64().unwrap(), 250.0);
        assert_eq!(out[1]["timestamp"].as_str().unwrap(), "2024-02-01T00:00:00+00:00");
        assert_eq!(out[1]["open"].as_f64().unwrap(), 20.0);
    }

    #[test]
    fn aggregate_15m_to_30m_buckets_on_utc_boundaries_with_correct_ohlcv() {
        // 2024-01-01 14:00 UTC = 1_704_117_600 s. Four 15m bars -> two 30m bars.
        let s = 1_704_117_600_000i64;
        let raw = vec![
            (s, 10.0, 12.0, 9.0, 11.0, 100.0),         // 14:00
            (s + 900_000, 11.0, 13.0, 10.0, 12.0, 150.0), // 14:15
            (s + 1_800_000, 12.0, 14.0, 11.0, 13.0, 200.0), // 14:30
            (s + 2_700_000, 13.0, 15.0, 12.0, 14.0, 250.0), // 14:45
        ];
        let out = aggregate_raw_to_rollup(&raw, Rollup::Fixed(1_800));
        assert_eq!(out.len(), 2);
        // Bucket 1 (14:00-14:30): open=first, close=last, high/low=extremes, vol=sum.
        assert_eq!(out[0]["timestamp"].as_str().unwrap(), "2024-01-01T14:00:00+00:00");
        assert_eq!(out[0]["open"].as_f64().unwrap(), 10.0);
        assert_eq!(out[0]["high"].as_f64().unwrap(), 13.0);
        assert_eq!(out[0]["low"].as_f64().unwrap(), 9.0);
        assert_eq!(out[0]["close"].as_f64().unwrap(), 12.0);
        assert_eq!(out[0]["volume"].as_f64().unwrap(), 250.0);
        // Bucket 2 (14:30-15:00).
        assert_eq!(out[1]["timestamp"].as_str().unwrap(), "2024-01-01T14:30:00+00:00");
        assert_eq!(out[1]["open"].as_f64().unwrap(), 12.0);
        assert_eq!(out[1]["high"].as_f64().unwrap(), 15.0);
        assert_eq!(out[1]["low"].as_f64().unwrap(), 11.0);
        assert_eq!(out[1]["close"].as_f64().unwrap(), 14.0);
        assert_eq!(out[1]["volume"].as_f64().unwrap(), 450.0);
    }

    #[test]
    fn aggregate_skips_invalid_bars_and_handles_unsorted_input() {
        let s = 1_704_117_600_000i64;
        let raw = vec![
            (s + 900_000, 11.0, 13.0, 10.0, 12.0, 150.0), // 14:15 (out of order)
            (s, 10.0, 12.0, 9.0, 11.0, 100.0),            // 14:00
            (s + 1_000, -1.0, 1.0, 1.0, 1.0, 1.0),        // invalid (price <= 0) -> skipped
        ];
        let out = aggregate_raw_to_rollup(&raw, Rollup::Fixed(1_800));
        assert_eq!(out.len(), 1);
        // open must come from the chronologically-first bar (14:00), not input order.
        assert_eq!(out[0]["open"].as_f64().unwrap(), 10.0);
        assert_eq!(out[0]["close"].as_f64().unwrap(), 12.0);
        assert_eq!(out[0]["volume"].as_f64().unwrap(), 250.0);
    }

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
    fn intraday_stall_full_pull_fires_only_when_15min_is_far_fresher() {
        let day = 86_400i64;
        let now = 1_900_000_000i64;
        // META case: 15Min current, 1H tail 860 days stale ⇒ heal.
        assert!(intraday_stall_needs_full_pull(Some(now), Some(now - 860 * day)));
        // Have 15Min but no native 1H/4H at all ⇒ pull the base once.
        assert!(intraday_stall_needs_full_pull(Some(now), None));
        // Healthy: 1H within a normal weekend of the 15Min tail ⇒ no full pull.
        assert!(!intraday_stall_needs_full_pull(Some(now), Some(now - 3 * day)));
        // Moderately stale (< 30d) heals via the cheap incremental walk, not a
        // full pull.
        assert!(!intraday_stall_needs_full_pull(Some(now), Some(now - 20 * day)));
        // No 15Min reference (Alpaca has no recent data) ⇒ never loops.
        assert!(!intraday_stall_needs_full_pull(None, Some(now - 999 * day)));
        assert!(!intraday_stall_needs_full_pull(None, None));
    }

    #[test]
    fn successful_batch_omissions_do_not_spawn_targeted_retry_storms() {
        assert_eq!(
            alpaca_batch_missing_symbol_retry_reason(crate::broker::alpaca::FetchOutcome::Complete),
            None
        );
        assert_eq!(
            alpaca_batch_missing_symbol_retry_reason(
                crate::broker::alpaca::FetchOutcome::RateLimitedPartial
            ),
            Some("batch_rate_limited_partial")
        );
        assert_eq!(
            alpaca_batch_missing_symbol_retry_reason(
                crate::broker::alpaca::FetchOutcome::RateLimitedEmpty
            ),
            Some("batch_rate_limited_empty")
        );
    }
}
