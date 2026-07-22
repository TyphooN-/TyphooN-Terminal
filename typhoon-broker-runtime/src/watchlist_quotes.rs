use std::collections::HashMap;
use std::sync::Arc;

use typhoon_engine::broker::alpaca::{AlpacaBroker, SnapshotData};
use typhoon_engine::broker::protocol::BrokerMsg;
use typhoon_engine::core::cache::SqliteCache;
use typhoon_engine::core::fundamentals;
use typhoon_engine::core::watchlist::WatchlistRow;

const ALPACA_SNAPSHOT_CONCURRENCY: usize = 4;
const WATCHLIST_CACHE_FALLBACK_TIMEFRAMES: [&str; 6] =
    ["quote", "1Day", "4Hour", "1Hour", "30Min", "15Min"];

#[derive(Debug, PartialEq, Eq)]
struct WatchlistCacheFallback {
    row_index: usize,
    candidate_key_indices: Vec<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct WatchlistCacheCursor {
    fallback_index: usize,
    candidate_index: usize,
}

fn watchlist_cache_fallback_plan(
    rows: &[WatchlistRow],
) -> (Vec<WatchlistCacheFallback>, Vec<String>) {
    let mut key_indices = HashMap::new();
    let mut unique_keys = Vec::new();
    let mut fallbacks = Vec::new();
    for (row_index, row) in rows.iter().enumerate() {
        if row.last > 0.0 && row.last.is_finite() {
            continue;
        }
        let mut candidate_key_indices = Vec::new();
        for timeframe in WATCHLIST_CACHE_FALLBACK_TIMEFRAMES {
            for source in
                typhoon_engine::core::watchlist::watchlist_cache_fallback_sources(&row.symbol)
            {
                for key in typhoon_engine::broker::cache_keys::chart_source_cache_keys(
                    source,
                    &row.symbol,
                    timeframe,
                ) {
                    let key_index = if let Some(&index) = key_indices.get(&key) {
                        index
                    } else {
                        let index = unique_keys.len();
                        key_indices.insert(key.clone(), index);
                        unique_keys.push(key);
                        index
                    };
                    candidate_key_indices.push(key_index);
                }
            }
        }
        if !candidate_key_indices.is_empty() {
            fallbacks.push(WatchlistCacheFallback {
                row_index,
                candidate_key_indices,
            });
        }
    }
    (fallbacks, unique_keys)
}

fn apply_watchlist_cache_fallback_round(
    rows: &mut [WatchlistRow],
    fallbacks: &[WatchlistCacheFallback],
    keys: &[String],
    cursors: &[WatchlistCacheCursor],
    cached_by_key: &typhoon_engine::core::cache::RawBarsByKey,
) -> Vec<WatchlistCacheCursor> {
    let mut next = Vec::new();
    for cursor in cursors {
        let fallback = &fallbacks[cursor.fallback_index];
        let row = &mut rows[fallback.row_index];
        let key_index = fallback.candidate_key_indices[cursor.candidate_index];
        let key = &keys[key_index];
        let cached = cached_by_key
            .get(key)
            .and_then(|result| result.as_ref().ok())
            .and_then(|raw| {
                typhoon_engine::core::watchlist::watchlist_row_from_raw_bars(&row.symbol, key, raw)
            });
        if let Some(cached) = cached {
            *row = cached;
        } else if cursor.candidate_index + 1 < fallback.candidate_key_indices.len() {
            next.push(WatchlistCacheCursor {
                fallback_index: cursor.fallback_index,
                candidate_index: cursor.candidate_index + 1,
            });
        }
    }
    next
}

fn alpaca_snapshot_symbol(symbol: &str) -> String {
    const CRYPTO_BASES: &[&str] = &[
        "BTC", "ETH", "SOL", "DOGE", "XRP", "ADA", "LTC", "LINK", "AVAX", "DOT", "XMR", "ZEC",
        "DASH",
    ];

    let upper = symbol.to_uppercase();
    CRYPTO_BASES
        .iter()
        .find_map(|base| {
            (upper.starts_with(base) && upper.ends_with("USD") && upper.len() == base.len() + 3)
                .then(|| format!("{base}/USD"))
        })
        .unwrap_or_else(|| symbol.to_string())
}

fn apply_alpaca_snapshot(
    row: &mut WatchlistRow,
    snapshot: SnapshotData,
    regular_session_open: bool,
) {
    let change = snapshot.last - snapshot.prev_close;
    let change_pct = if snapshot.prev_close > 0.0 {
        (snapshot.last / snapshot.prev_close - 1.0) * 100.0
    } else {
        0.0
    };
    let ext_change_pct = if regular_session_open {
        0.0
    } else if snapshot.regular_close > 0.0 && (snapshot.last - snapshot.regular_close).abs() > 1e-10
    {
        (snapshot.last / snapshot.regular_close - 1.0) * 100.0
    } else {
        0.0
    };
    *row = WatchlistRow {
        symbol: row.symbol.clone(),
        cache_key: row.symbol.clone(),
        last: snapshot.last,
        prev_close: snapshot.prev_close,
        regular_close: snapshot.regular_close,
        change,
        change_pct,
        volume: snapshot.daily_volume,
        ext_change_pct,
        live_bid: 0.0,
        live_ask: 0.0,
        live_bid_size: 0.0,
        live_ask_size: 0.0,
        live_quote_at: None,
    };
}

pub fn spawn_watchlist_quotes_task(
    symbols: Vec<String>,
    broker: Option<AlpacaBroker>,
    broker_msg_tx: tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    shared_cache: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
) {
    // Run OFF the serial broker loop. This fetch makes up to one broker snapshot
    // per watchlist symbol (3s timeout each) plus a Yahoo round-trip; on the
    // shared loop its periodic refresh starved trading-critical commands
    // (GetPositions/GetOrders), leaving positions stale. Spawning keeps it
    // concurrent — the same off-loop principle the equities sync already follows.
    tokio::spawn(async move {
        let mut rows: Vec<WatchlistRow> = symbols
            .iter()
            .map(|sym| typhoon_engine::core::watchlist::empty_watchlist_row(sym))
            .collect();

        let regular_session_open = if let Some(ref b) = broker {
            b.get_market_clock()
                .await
                .ok()
                .and_then(|v| v["is_open"].as_bool())
                .unwrap_or(false)
        } else {
            false
        };

        if let Some(ref b) = broker {
            // Bound fan-out to avoid both the previous O(symbols × 3s) tail latency
            // and an unbounded burst against Alpaca's snapshot endpoint.
            for chunk in rows.chunks_mut(ALPACA_SNAPSHOT_CONCURRENCY) {
                let fetches = chunk.iter().map(|row| {
                    let api_symbol = alpaca_snapshot_symbol(&row.symbol);
                    async move {
                        tokio::time::timeout(
                            std::time::Duration::from_secs(3),
                            b.get_snapshot(&api_symbol),
                        )
                        .await
                        .ok()
                        .and_then(Result::ok)
                    }
                });
                let snapshots = futures_util::future::join_all(fetches).await;
                for (row, snapshot) in chunk.iter_mut().zip(snapshots) {
                    if let Some(snapshot) = snapshot {
                        apply_alpaca_snapshot(row, snapshot, regular_session_open);
                    }
                }
            }
        }

        // Yahoo Finance enrichment for regular + extended-hours prices. This is deliberately
        // outside the Alpaca broker branch so the watchlist still refreshes on weekends,
        // holidays, and Kraken-only/offline-broker sessions.
        // Build an index once so quote enrichment is O(results) instead of O(results × rows).
        let row_by_symbol: std::collections::HashMap<String, usize> = rows
            .iter()
            .enumerate()
            .map(|(idx, row)| (row.symbol.clone(), idx))
            .collect();

        let equity_syms: Vec<String> = rows
            .iter()
            .filter(|r| {
                !r.symbol.contains('/') && !(r.symbol.ends_with("USD") && r.symbol.len() > 5)
            })
            .map(|r| r.symbol.clone())
            .collect();
        if !equity_syms.is_empty() {
            // Fresh Yahoo session per refresh. This now runs in a spawned task off the broker
            // loop, so the auth round-trip is not on any critical path.
            let yahoo_session = fundamentals::YahooSession::new().await.ok();
            if let Some(ref session) = yahoo_session {
                let sym_list = equity_syms.join(",");
                let crumb_param = if session.crumb().is_empty() {
                    String::new()
                } else {
                    format!("&crumb={}", session.crumb())
                };
                let url = format!(
                    "https://query2.finance.yahoo.com/v7/finance/quote?symbols={}&fields=regularMarketPrice,regularMarketPreviousClose,regularMarketVolume,regularMarketTime,marketState,preMarketPrice,preMarketTime,preMarketChangePercent,postMarketPrice,postMarketTime,postMarketChangePercent{}",
                    sym_list, crumb_param
                );
                if let Ok(Ok(resp)) = tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    session
                        .client()
                        .get(&url)
                        .header("Accept", "application/json")
                        .send(),
                )
                .await
                {
                    if let Ok(json) = resp.json::<serde_json::Value>().await {
                        if let Some(results) = json["quoteResponse"]["result"].as_array() {
                            for q in results {
                                let sym = q["symbol"].as_str().unwrap_or("");
                                let Some(&row_idx) = row_by_symbol.get(sym) else {
                                    continue;
                                };
                                let row = &mut rows[row_idx];
                                let reg_price = q["regularMarketPrice"].as_f64().unwrap_or(0.0);
                                let reg_prev =
                                    q["regularMarketPreviousClose"].as_f64().unwrap_or(0.0);

                                let yah_vol = q["regularMarketVolume"]
                                    .as_f64()
                                    .or_else(|| q["regularMarketVolume"].as_i64().map(|v| v as f64))
                                    .or_else(|| q["regularMarketVolume"]["raw"].as_f64())
                                    .unwrap_or(0.0);

                                if row.prev_close <= 0.0 && reg_prev > 0.0 {
                                    row.prev_close = reg_prev;
                                }
                                // Yahoo's regular price is the authoritative current-day close and is
                                // often fresher than Alpaca's snapshot, so the ext "Daily Close" badge
                                // agrees across timeframes.
                                if reg_price > 0.0 {
                                    row.regular_close = reg_price;
                                }
                                if yah_vol > 0.0 {
                                    row.volume = yah_vol;
                                }

                                // Yahoo keeps stale pre/post prices on the quote payload. Only trust them
                                // when Yahoo says the symbol is in PRE/POST *and* the extended quote
                                // timestamp is at least as fresh as the regular-market timestamp.
                                // TNDM exposed the failure mode: marketState=POST, regular price was
                                // current, but postMarketPrice was still yesterday's stale after-hours tick.
                                let market_state = q["marketState"].as_str().unwrap_or("");
                                let allow_ext_quote = typhoon_engine::core::watchlist::yahoo_market_state_allows_extended_quote(market_state);
                                let regular_time = q["regularMarketTime"].as_i64().unwrap_or(0);
                                let pre_time = q["preMarketTime"].as_i64().unwrap_or(0);
                                let post_time = q["postMarketTime"].as_i64().unwrap_or(0);
                                let pre_price = if allow_ext_quote
                                    && typhoon_engine::core::watchlist::yahoo_extended_quote_time_is_fresh(pre_time, regular_time)
                                {
                                    q["preMarketPrice"].as_f64().unwrap_or(0.0)
                                } else {
                                    0.0
                                };
                                let post_price = if allow_ext_quote
                                    && typhoon_engine::core::watchlist::yahoo_extended_quote_time_is_fresh(post_time, regular_time)
                                {
                                    q["postMarketPrice"].as_f64().unwrap_or(0.0)
                                } else {
                                    0.0
                                };

                                // Use whichever extended price is available during active extended sessions.
                                let ext_price = if pre_price > 0.0 {
                                    pre_price
                                } else if post_price > 0.0 {
                                    post_price
                                } else {
                                    0.0
                                };

                                if ext_price > 0.0 && row.prev_close > 0.0 {
                                    row.last = ext_price;
                                    row.change = ext_price - row.prev_close;
                                    row.change_pct = (ext_price / row.prev_close - 1.0) * 100.0;
                                    // Ext% = change from regular close to ext price.
                                    row.ext_change_pct = if reg_price > 0.0 {
                                        (ext_price / reg_price - 1.0) * 100.0
                                    } else {
                                        row.change_pct
                                    };
                                } else if reg_price > 0.0 && row.prev_close > 0.0 {
                                    // No ext hours — use Yahoo regular price (may be fresher than Alpaca).
                                    row.last = reg_price;
                                    row.change = reg_price - row.prev_close;
                                    row.change_pct = (reg_price / row.prev_close - 1.0) * 100.0;
                                } else if row.last <= 0.0 && reg_price > 0.0 {
                                    row.last = reg_price;
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(cache) = shared_cache.read().ok().and_then(|g| g.clone()) {
            let (fallbacks, keys) = watchlist_cache_fallback_plan(&rows);
            let mut cursors: Vec<WatchlistCacheCursor> = (0..fallbacks.len())
                .map(|fallback_index| WatchlistCacheCursor {
                    fallback_index,
                    candidate_index: 0,
                })
                .collect();
            while !cursors.is_empty() {
                let mut seen = std::collections::HashSet::with_capacity(cursors.len());
                let round_keys: Vec<String> = cursors
                    .iter()
                    .filter_map(|cursor| {
                        let fallback = &fallbacks[cursor.fallback_index];
                        let key_index = fallback.candidate_key_indices[cursor.candidate_index];
                        seen.insert(key_index).then(|| keys[key_index].clone())
                    })
                    .collect();
                let Ok(cached_by_key) = cache.get_bars_raw_many(&round_keys) else {
                    break;
                };
                cursors = apply_watchlist_cache_fallback_round(
                    &mut rows,
                    &fallbacks,
                    &keys,
                    &cursors,
                    &cached_by_key,
                );
            }
            for fallback in fallbacks {
                let row = &rows[fallback.row_index];
                if row.last <= 0.0 || !row.last.is_finite() {
                    tracing::debug!("watchlist: no broker/Yahoo/cache quote for {}", row.symbol);
                }
            }
        }
        let _ = broker_msg_tx.send(BrokerMsg::WatchlistQuotes(rows));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alpaca_snapshot_symbol_normalizes_supported_crypto_pairs_only() {
        assert_eq!(alpaca_snapshot_symbol("BTCUSD"), "BTC/USD");
        assert_eq!(alpaca_snapshot_symbol("ethusd"), "ETH/USD");
        assert_eq!(alpaca_snapshot_symbol("AAPL"), "AAPL");
        assert_eq!(alpaca_snapshot_symbol("BABYUSD"), "BABYUSD");
    }

    #[test]
    fn snapshot_application_preserves_display_symbol_and_calculates_changes() {
        let mut row = typhoon_engine::core::watchlist::empty_watchlist_row("BTCUSD");
        apply_alpaca_snapshot(
            &mut row,
            SnapshotData {
                symbol: "BTC/USD".to_string(),
                last: 105.0,
                prev_close: 100.0,
                daily_volume: 42.0,
                regular_close: 102.0,
            },
            false,
        );
        assert_eq!(row.symbol, "BTCUSD");
        assert_eq!(row.last, 105.0);
        assert_eq!(row.change, 5.0);
        assert!((row.change_pct - 5.0).abs() < 1e-12);
        assert!((row.ext_change_pct - (105.0 / 102.0 - 1.0) * 100.0).abs() < 1e-12);
    }

    #[test]
    fn cache_fallback_plan_deduplicates_queries_without_changing_row_order() {
        let rows = vec![
            typhoon_engine::core::watchlist::empty_watchlist_row("AAPL"),
            typhoon_engine::core::watchlist::empty_watchlist_row("AAPL"),
        ];

        let (fallbacks, keys) = watchlist_cache_fallback_plan(&rows);

        assert_eq!(fallbacks.len(), 2);
        assert_eq!(fallbacks[0].row_index, 0);
        assert_eq!(fallbacks[1].row_index, 1);
        assert_eq!(
            fallbacks[0].candidate_key_indices,
            fallbacks[1].candidate_key_indices
        );
        assert_eq!(keys.len(), fallbacks[0].candidate_key_indices.len());
    }

    #[test]
    fn cache_fallback_plan_preserves_legacy_candidate_precedence() {
        for symbol in ["AAPL", "BTCUSD"] {
            let rows = vec![typhoon_engine::core::watchlist::empty_watchlist_row(symbol)];
            let (fallbacks, keys) = watchlist_cache_fallback_plan(&rows);
            let planned: Vec<&str> = fallbacks[0]
                .candidate_key_indices
                .iter()
                .map(|&index| keys[index].as_str())
                .collect();
            let expected: Vec<String> = WATCHLIST_CACHE_FALLBACK_TIMEFRAMES
                .iter()
                .flat_map(|timeframe| {
                    typhoon_engine::core::watchlist::watchlist_cache_fallback_sources(symbol)
                        .iter()
                        .flat_map(move |source| {
                            typhoon_engine::broker::cache_keys::chart_source_cache_keys(
                                source, symbol, timeframe,
                            )
                        })
                })
                .collect();

            assert_eq!(planned, expected);
        }
    }

    #[test]
    fn cache_fallback_continues_after_corrupt_preferred_candidate() {
        let mut rows = vec![typhoon_engine::core::watchlist::empty_watchlist_row("AAPL")];
        let (fallbacks, keys) = watchlist_cache_fallback_plan(&rows);
        assert!(keys.len() >= 2);
        let mut cached_by_key = typhoon_engine::core::cache::RawBarsByKey::new();
        cached_by_key.insert(keys[0].clone(), Err("corrupt".to_string()));
        let cursors = vec![WatchlistCacheCursor {
            fallback_index: 0,
            candidate_index: 0,
        }];
        let cursors = apply_watchlist_cache_fallback_round(
            &mut rows,
            &fallbacks,
            &keys,
            &cursors,
            &cached_by_key,
        );
        assert_eq!(
            cursors,
            vec![WatchlistCacheCursor {
                fallback_index: 0,
                candidate_index: 1,
            }]
        );
        assert_eq!(rows[0].last, 0.0);

        let mut cached_by_key = typhoon_engine::core::cache::RawBarsByKey::new();
        cached_by_key.insert(
            keys[1].clone(),
            Ok(vec![
                (1_000, 99.0, 101.0, 98.0, 100.0, 10.0),
                (2_000, 100.0, 106.0, 99.0, 105.0, 20.0),
            ]),
        );

        let cursors = apply_watchlist_cache_fallback_round(
            &mut rows,
            &fallbacks,
            &keys,
            &cursors,
            &cached_by_key,
        );

        assert!(cursors.is_empty());
        assert_eq!(rows[0].cache_key, keys[1]);
        assert_eq!(rows[0].last, 105.0);
        assert_eq!(rows[0].prev_close, 100.0);
    }
}
