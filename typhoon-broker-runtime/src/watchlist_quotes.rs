use std::sync::Arc;

use typhoon_engine::broker::alpaca::AlpacaBroker;
use typhoon_engine::broker::protocol::BrokerMsg;
use typhoon_engine::core::cache::SqliteCache;
use typhoon_engine::core::fundamentals;
use typhoon_engine::core::watchlist::WatchlistRow;

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
            for row in &mut rows {
                let api_sym = {
                    let crypto_bases = [
                        "BTC", "ETH", "SOL", "DOGE", "XRP", "ADA", "LTC", "LINK", "AVAX", "DOT",
                        "XMR", "ZEC", "DASH",
                    ];
                    let su = row.symbol.to_uppercase();
                    crypto_bases
                        .iter()
                        .find_map(|base| {
                            if su.starts_with(base)
                                && su.ends_with("USD")
                                && su.len() == base.len() + 3
                            {
                                Some(format!("{}/USD", base))
                            } else {
                                None
                            }
                        })
                        .unwrap_or_else(|| row.symbol.clone())
                };
                // 3s timeout per symbol — don't let one stale symbol block the entire watchlist.
                // During weekends/off-hours this may fail or return stale/empty data; Yahoo/cache
                // enrichment below still keeps the watchlist usable.
                if let Ok(Ok(snap)) = tokio::time::timeout(
                    std::time::Duration::from_secs(3),
                    b.get_snapshot(&api_sym),
                )
                .await
                {
                    let change = snap.last - snap.prev_close;
                    let change_pct = if snap.prev_close > 0.0 {
                        (snap.last / snap.prev_close - 1.0) * 100.0
                    } else {
                        0.0
                    };
                    // Extended hours change: last trade vs regular session close.
                    // Reset ext_change_pct during regular hours to avoid carrying over
                    // yesterday's extended hours change as the starting point.
                    let ext_change_pct = if regular_session_open {
                        0.0
                    } else if snap.regular_close > 0.0
                        && (snap.last - snap.regular_close).abs() > 1e-10
                    {
                        (snap.last / snap.regular_close - 1.0) * 100.0
                    } else {
                        0.0
                    };
                    *row = WatchlistRow {
                        symbol: row.symbol.clone(),
                        cache_key: row.symbol.clone(),
                        last: snap.last,
                        prev_close: snap.prev_close,
                        regular_close: snap.regular_close,
                        change,
                        change_pct,
                        volume: snap.daily_volume,
                        ext_change_pct,
                        live_bid: 0.0,
                        live_ask: 0.0,
                        live_quote_at: None,
                    };
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
            for row in &mut rows {
                if row.last > 0.0 && row.last.is_finite() {
                    continue;
                }
                let mut filled = false;
                'cache_fallback: for tf in ["quote", "1Day", "4Hour", "1Hour", "30Min", "15Min"] {
                    for source in typhoon_engine::core::watchlist::watchlist_cache_fallback_sources(
                        &row.symbol,
                    ) {
                        for key in typhoon_chart_ui::cache_keys::chart_source_cache_keys(
                            source,
                            &row.symbol,
                            tf,
                        ) {
                            let Ok(Some(raw)) = cache.get_bars_raw(&key) else {
                                continue;
                            };
                            if let Some(cached) =
                                typhoon_engine::core::watchlist::watchlist_row_from_raw_bars(
                                    &row.symbol,
                                    &key,
                                    &raw,
                                )
                            {
                                *row = cached;
                                filled = true;
                                break 'cache_fallback;
                            }
                        }
                    }
                }
                if !filled {
                    tracing::debug!("watchlist: no broker/Yahoo/cache quote for {}", row.symbol);
                }
            }
        }
        let _ = broker_msg_tx.send(BrokerMsg::WatchlistQuotes(rows));
    });
}
