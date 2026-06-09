use super::*;

mod external_feeds;
mod lan_sync;
mod news;
mod research_compute;
mod storage;

fn kraken_ws_v2_book_state_json(
    display_symbol: &str,
    state: &typhoon_engine::broker::kraken::KrakenWsBookState,
    checksum: Option<u32>,
    status: &str,
) -> String {
    let timestamp = state
        .last_ts_ms
        .and_then(chrono::DateTime::from_timestamp_millis)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_default();
    let bids: Vec<serde_json::Value> = state
        .bids
        .iter()
        .map(|level| {
            serde_json::json!({
                "price": level.price,
                "size": level.qty,
                "price_text": level.price_text,
                "size_text": level.qty_text,
            })
        })
        .collect();
    let asks: Vec<serde_json::Value> = state
        .asks
        .iter()
        .map(|level| {
            serde_json::json!({
                "price": level.price,
                "size": level.qty,
                "price_text": level.price_text,
                "size_text": level.qty_text,
            })
        })
        .collect();
    serde_json::json!({
        "symbol": display_symbol,
        "ws_symbol": state.symbol,
        "timestamp": timestamp,
        "depth": state.depth,
        "checksum": checksum,
        "server_checksum": state.last_checksum,
        "checksum_status": status,
        "bids": bids,
        "asks": asks,
    })
    .to_string()
}

pub(super) fn spawn_broker_message_processor(
    broker_cmd_rx: tokio::sync::mpsc::UnboundedReceiver<BrokerCmd>,
    broker_msg_tx: tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    importing_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    lan_client_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    rt_handle: tokio::runtime::Handle,
    shared_cache: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
) {
    // Spawn broker message processor
    let broker_msg_tx_clone = broker_msg_tx.clone();
    let importing_flag_broker = importing_flag.clone();
    let lan_client_broker = lan_client_flag.clone();
    let shared_cache_broker = shared_cache.clone(); // shared DB connection for LAN sync
    rt_handle.spawn(async move {
        let mut cmd_rx = broker_cmd_rx;
        let mut broker: Option<AlpacaBroker> = None;
        let mut kraken_broker: Option<typhoon_engine::broker::kraken::KrakenBroker> = None;
        let mut kraken_ws_broker: Option<typhoon_engine::broker::kraken::KrakenBroker> = None;
        // Pre-acquire and per-endpoint spacing are now owned by the
        // engine-side `iapi_limiter` (token bucket + escalating backoff,
        // shared across all iapi endpoints). The handler below just
        // delegates to it instead of maintaining its own gate state.
        let importing_flag = importing_flag_broker;
        let lan_client = lan_client_broker;
        // Shared sender for forwarding requests to LAN sync WebSocket
        let lan_remote_tx: Arc<tokio::sync::Mutex<Option<tokio::sync::mpsc::UnboundedSender<String>>>> =
            Arc::new(tokio::sync::Mutex::new(None));
        let lan_remote_tx_ref = lan_remote_tx.clone();
        let mut lan_reconnect_handle: Option<tokio::task::AbortHandle> = None;
        let mut alpaca_fetch_permits = Arc::new(tokio::sync::Semaphore::new(4));
        let yahoo_chart_fetch_permits = Arc::new(tokio::sync::Semaphore::new(4));
        let kraken_fetch_permits =
            Arc::new(tokio::sync::Semaphore::new(KRAKEN_PUBLIC_FETCH_PERMITS));
        // Kraken Securities/iapi history is slower and can include synchronous cache work.
        // Keep it off the broker command loop and cap it separately so broad equities
        // sync cannot starve UI-visible broker messages (SEC scanner, order state, etc.).
        let kraken_equity_fetch_permits =
            Arc::new(tokio::sync::Semaphore::new(KRAKEN_EQUITIES_FETCH_PERMITS));
        let kraken_public_client = reqwest::Client::builder()
            .user_agent("TyphooN-Terminal/1.0")
            .pool_max_idle_per_host(KRAKEN_PUBLIC_FETCH_PERMITS * 2)
            .build()
            .unwrap_or_default();
        let fallback_bar_client = reqwest::Client::builder()
            .user_agent("TyphooN-Terminal/1.0")
            .pool_max_idle_per_host(8)
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .unwrap_or_default();
        // Cached Yahoo session for watchlist extended hours (avoid re-auth every cycle)
        let mut yahoo_session: Option<fundamentals::YahooSession> = None;
        let mut yahoo_session_created: std::time::Instant = std::time::Instant::now();
        while let Some(cmd) = cmd_rx.recv().await {
            // LAN client: forward external data-fetching commands to server
            if lan_client.load(std::sync::atomic::Ordering::Relaxed) {
                let remote_cmd = match &cmd {
                    BrokerCmd::SecScrape { .. } => Some("SEC_SCRAPE"),
                    BrokerCmd::FundamentalsScrape { force, .. } => Some(if *force { "FUNDAMENTALS_FORCE" } else { "FUNDAMENTALS" }),
                    BrokerCmd::FundamentalsScrapeOne { .. } => Some("FUNDAMENTALS_ONE"),
                    BrokerCmd::KrakenBackfill { .. } => Some("KRAKEN_BACKFILL"),
                    BrokerCmd::KrakenFuturesBackfill { .. } => Some("KRAKEN_FUTURES_BACKFILL"),
                    BrokerCmd::Mt5Sync { .. } => Some("MT5_SYNC"),
                    BrokerCmd::FinnhubNews { .. } => Some("FINNHUB_NEWS"),
                    BrokerCmd::FetchEconCalendar { .. } => Some("CALENDAR"),
                    BrokerCmd::FetchCongressTrades => Some("CONGRESS_TRADES"),
                    BrokerCmd::FredFetch { .. } => Some("FRED_DATA"),
                    // FetchFilingContent NOT forwarded — SEC EDGAR is public, fetch directly
                    // BrokerCmd::FetchFilingContent { .. } => Some("SEC_FILING"),
                    _ => None,
                };
                // Special handling: AlpacaFetchBars includes symbol+TF in args
                // Use try_lock to avoid blocking the broker command loop
                if let BrokerCmd::AlpacaFetchBars { ref symbol, ref timeframe, .. } = cmd {
                    if let Ok(guard) = lan_remote_tx_ref.try_lock() {
                        if let Some(ref tx) = *guard {
                            let _ = tx.send(format!("FETCH_BARS:{},{}", symbol, timeframe));
                            let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult(
                                format!("LAN client: fetching {} {} via server — will sync shortly", symbol, timeframe)
                            ));
                        } else {
                            let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult(
                                format!("LAN client: {} {} — waiting for server connection", symbol, timeframe)
                            ));
                        }
                    }
                    // else: lock contended, silently skip (will retry on next request)
                    continue;
                }
                // Special handling: IngestResearchArticles carries a multiline text payload.
                // We both run it locally (immediate visibility for the pasting user) AND
                // forward a JSON-wrapped copy to the server so the central DB gets the
                // articles too — the LAN sync on research_web_articles + research_news
                // will then propagate to other clients. We do NOT `continue` here so the
                // outer match still executes the local ingest below.
                if let BrokerCmd::IngestResearchArticles { ref text, ref agent_override } = cmd {
                    if let Ok(guard) = lan_remote_tx_ref.try_lock() {
                        if let Some(ref tx) = *guard {
                            let payload = serde_json::json!({
                                "text": text,
                                "agent": agent_override,
                            }).to_string();
                            let _ = tx.send(format!("INGEST_RESEARCH:{}", payload));
                            let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult(
                                "LAN client: research ingest forwarded to server (will sync back).".into()
                            ));
                        }
                    }
                    // fall through — run local ingest too so the pasting user
                    // sees articles in the News panel immediately, even before
                    // the server's copy syncs back.
                }
                if let Some(cmd_name) = remote_cmd {
                    if let Ok(guard) = lan_remote_tx_ref.try_lock() {
                        if let Some(ref tx) = *guard {
                            let _ = tx.send(cmd_name.to_string());
                            let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult(
                                format!("LAN client: '{}' forwarded to server", cmd_name)
                            ));
                        } else {
                            let _ = broker_msg_tx_clone.send(BrokerMsg::Error(
                                format!("LAN client: '{}' — not connected to server", cmd_name)
                            ));
                        }
                    }
                    continue;
                }
            }
            match cmd {
                BrokerCmd::Connect {
                    api_key,
                    secret,
                    paper,
                    bar_requests_per_minute,
                    fetch_permits,
                } => {
                    alpaca_fetch_permits =
                        Arc::new(tokio::sync::Semaphore::new(fetch_permits.max(1)));
                    let b = AlpacaBroker::new(
                        api_key,
                        secret,
                        paper,
                        bar_requests_per_minute.max(ALPACA_DEFAULT_HISTORICAL_RPM),
                    );
                    match b.get_account().await {
                        Ok(acct) => {
                            let _ = broker_msg_tx_clone.send(BrokerMsg::Connected(format!(
                                "Connected: ${:.2} equity, ${:.2} buying power",
                                acct.equity, acct.buying_power
                            )));
                            let _ = broker_msg_tx_clone.send(BrokerMsg::Account(acct));
                            b.warm_data_connection().await;
                            broker = Some(b);
                        }
                        Err(e) => {
                            let _ = broker_msg_tx_clone.send(BrokerMsg::Error(format!("Connection failed: {}", e)));
                        }
                    }
                }
                BrokerCmd::ConfigureAlpacaSync {
                    bar_requests_per_minute,
                    fetch_permits,
                } => {
                    alpaca_fetch_permits =
                        Arc::new(tokio::sync::Semaphore::new(fetch_permits.max(1)));
                    if let Some(ref b) = broker {
                        b.set_bar_requests_per_minute_hint(
                            bar_requests_per_minute.max(ALPACA_DEFAULT_HISTORICAL_RPM),
                        )
                        .await;
                    }
                }
                BrokerCmd::MarkUnresolvable {
                    broker,
                    symbol,
                    timeframe,
                    reason,
                } => {
                    let _ = broker_msg_tx_clone.send(BrokerMsg::Unresolvable {
                        broker,
                        symbol,
                        timeframe,
                        reason,
                    });
                }
                BrokerCmd::GetAccount => {
                    if let Some(ref b) = broker {
                        match b.get_account().await {
                            Ok(acct) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Account(acct)); }
                            Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(e)); }
                        }
                    }
                }
                BrokerCmd::GetPositions => {
                    if let Some(ref b) = broker {
                        match b.get_positions().await {
                            Ok(pos) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Positions(pos)); }
                            Err(e) => { tracing::debug!("Positions request failed: {}", e); }
                        }
                    }
                }
                BrokerCmd::GetOrders => {
                    if let Some(ref b) = broker {
                        match b.get_orders("open", 100).await {
                            Ok(orders) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Orders(orders)); }
                            Err(e) => { tracing::debug!("Orders request failed: {}", e); }
                        }
                    }
                }
                BrokerCmd::CloseAll => {
                    if let Some(ref b) = broker {
                        match b.close_all_positions().await {
                            Ok(_) => { let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult("All positions closed".into())); }
                            Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(e)); }
                        }
                    }
                }
                BrokerCmd::ClosePosition { symbol, qty } => {
                    if let Some(ref b) = broker {
                        match b.close_position(&symbol, qty).await {
                            Ok(r) => { let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult(format!("Closed {}: {}", symbol, r.status))); }
                            Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(e)); }
                        }
                    }
                }
                BrokerCmd::SecScrape { db_path, symbols } => {
                    // Spawn as independent task — SEC scraping can take 10-60s and must not
                    // block the broker command loop (would freeze trading, data fetch, etc.)
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let _ = msg_tx.send(BrokerMsg::OrderResult("SEC scrape started...".into()));
                        match sec_filing::scrape_all_portfolio_symbols(db_path, Some(symbols)).await {
                            Ok(stats) => {
                                let error_suffix = if stats.errors.is_empty() {
                                    String::new()
                                } else {
                                    format!(", {} errors (first: {})", stats.errors.len(), stats.errors[0])
                                };
                                let _ = msg_tx.send(BrokerMsg::SecScrapeResult(
                                    format!("SEC scrape complete: {} tickers, {} filings, {} insider trades, {} alerts{}", stats.tickers_scanned, stats.new_filings, stats.new_insider_trades, stats.new_alerts, error_suffix)
                                ));
                            }
                            Err(e) => {
                                let _ = msg_tx.send(BrokerMsg::SecScrapeResult(format!(
                                    "SEC scrape error: {}",
                                    e
                                )));
                            }
                        }
                    });
                }
                // scrape_filings_for_ticker is called internally by scrape_all_portfolio_symbols
                BrokerCmd::FinnhubNews { symbol, api_key } => {
                    // Finnhub has its own API + key — no dependency on Alpaca state, so don't
                    // gate it on the Alpaca broker being connected. Users on Kraken-only setups
                    // would otherwise see "Connect broker first" even with a valid Finnhub key.
                    match typhoon_engine::broker::alpaca::AlpacaBroker::get_finnhub_news(&symbol, &api_key).await {
                        Ok(articles) => {
                            let results: Vec<(String, String, String)> = articles.iter().filter_map(|a| {
                                let headline = a["headline"].as_str()?.to_string();
                                let source = a["source"].as_str().unwrap_or("Unknown").to_string();
                                let dt = a["datetime"].as_str().unwrap_or("").to_string();
                                Some((headline, source, dt))
                            }).collect();
                            let _ = broker_msg_tx_clone.send(BrokerMsg::FinnhubNewsResult(results));
                        }
                        Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(format!("Finnhub: {}", e))); }
                    }
                }
                BrokerCmd::GetQuote { symbol } => {
                    if let Some(ref b) = broker {
                        match b.get_latest_quote(&symbol).await {
                            Ok(q) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Quote(symbol, q.bid, q.ask, (q.bid + q.ask) / 2.0)); }
                            Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(e)); }
                        }
                    }
                }
                BrokerCmd::GetWatchlistQuotes { symbols } => {
                    // LAN client: skip — watchlist data comes from server via KV sync (broker:watchlist)
                    if lan_client.load(std::sync::atomic::Ordering::Relaxed) {
                        continue;
                    }
                    let mut rows: Vec<WatchlistRow> = symbols
                        .iter()
                        .map(|sym| empty_watchlist_row(sym))
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
                                let crypto_bases = ["BTC","ETH","SOL","DOGE","XRP","ADA","LTC","LINK","AVAX","DOT","XMR","ZEC","DASH"];
                                let su = row.symbol.to_uppercase();
                                crypto_bases.iter().find_map(|base| {
                                    if su.starts_with(base) && su.ends_with("USD") && su.len() == base.len() + 3 {
                                        Some(format!("{}/USD", base))
                                    } else { None }
                                }).unwrap_or_else(|| row.symbol.clone())
                            };
                            // 3s timeout per symbol — don't let one stale symbol block the entire watchlist.
                            // During weekends/off-hours this may fail or return stale/empty data; Yahoo/cache
                            // enrichment below still keeps the watchlist usable.
                            if let Ok(Ok(snap)) = tokio::time::timeout(
                                std::time::Duration::from_secs(3),
                                b.get_snapshot(&api_sym),
                            ).await {
                                let change = snap.last - snap.prev_close;
                                let change_pct = if snap.prev_close > 0.0 { (snap.last / snap.prev_close - 1.0) * 100.0 } else { 0.0 };
                                // Extended hours change: last trade vs regular session close
                                let ext_change_pct = if !regular_session_open
                                    && snap.regular_close > 0.0
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
                                    change,
                                    change_pct,
                                    volume: snap.daily_volume,
                                    ext_change_pct,
                                };
                            }
                        }
                    }

                    // Yahoo Finance enrichment for regular + extended-hours prices. This is deliberately
                    // outside the Alpaca broker branch so the watchlist still refreshes on weekends,
                    // holidays, and Kraken-only/offline-broker sessions.
                    {
                        // Batch all equity symbols into one Yahoo query
                        let equity_syms: Vec<String> = rows.iter()
                            .filter(|r| !r.symbol.contains('/') && !(r.symbol.ends_with("USD") && r.symbol.len() > 5))
                            .map(|r| r.symbol.clone())
                            .collect();
                        if !equity_syms.is_empty() {
                            // Reuse cached Yahoo session (recreate every 30 min to refresh cookies)
                            if yahoo_session.is_none() || yahoo_session_created.elapsed().as_secs() > 1800 {
                                yahoo_session = fundamentals::YahooSession::new().await.ok();
                                yahoo_session_created = std::time::Instant::now();
                            }
                            if let Some(ref session) = yahoo_session {
                                let sym_list = equity_syms.join(",");
                                let crumb_param = if session.crumb().is_empty() { String::new() } else { format!("&crumb={}", session.crumb()) };
                                let url = format!(
                                    "https://query2.finance.yahoo.com/v7/finance/quote?symbols={}&fields=regularMarketPrice,regularMarketPreviousClose,regularMarketVolume,regularMarketTime,marketState,preMarketPrice,preMarketTime,preMarketChangePercent,postMarketPrice,postMarketTime,postMarketChangePercent{}",
                                    sym_list, crumb_param
                                );
                                if let Ok(Ok(resp)) = tokio::time::timeout(
                                    std::time::Duration::from_secs(5),
                                    session.client().get(&url).header("Accept", "application/json").send(),
                                ).await {
                                    if let Ok(json) = resp.json::<serde_json::Value>().await {
                                        if let Some(results) = json["quoteResponse"]["result"].as_array() {
                                            for q in results {
                                                let sym = q["symbol"].as_str().unwrap_or("");
                                                if let Some(row) = rows.iter_mut().find(|r| r.symbol == sym) {
                                                    let reg_price = q["regularMarketPrice"].as_f64().unwrap_or(0.0);
                                                    let reg_prev = q["regularMarketPreviousClose"].as_f64().unwrap_or(0.0);

                                                    let yah_vol = q["regularMarketVolume"].as_f64()
                                                        .or_else(|| q["regularMarketVolume"].as_i64().map(|v| v as f64))
                                                        .or_else(|| q["regularMarketVolume"]["raw"].as_f64())
                                                        .unwrap_or(0.0);

                                                    if row.prev_close <= 0.0 && reg_prev > 0.0 {
                                                        row.prev_close = reg_prev;
                                                    }
                                                    if yah_vol > 0.0 {
                                                        row.volume = yah_vol;
                                                    }

                                                    // Yahoo keeps stale pre/post prices on the quote payload. Only trust
                                                    // them when Yahoo says the symbol is in PRE/POST *and* the extended
                                                    // quote timestamp is at least as fresh as the regular-market timestamp.
                                                    // TNDM exposed the failure mode: marketState=POST, regular price was
                                                    // current, but postMarketPrice was still yesterday's stale after-hours tick.
                                                    let market_state = q["marketState"].as_str().unwrap_or("");
                                                    let allow_ext_quote = yahoo_market_state_allows_extended_quote(market_state);
                                                    let regular_time = q["regularMarketTime"].as_i64().unwrap_or(0);
                                                    let pre_time = q["preMarketTime"].as_i64().unwrap_or(0);
                                                    let post_time = q["postMarketTime"].as_i64().unwrap_or(0);
                                                    let pre_price = if allow_ext_quote
                                                        && yahoo_extended_quote_time_is_fresh(pre_time, regular_time)
                                                    {
                                                        q["preMarketPrice"].as_f64().unwrap_or(0.0)
                                                    } else {
                                                        0.0
                                                    };
                                                    let post_price = if allow_ext_quote
                                                        && yahoo_extended_quote_time_is_fresh(post_time, regular_time)
                                                    {
                                                        q["postMarketPrice"].as_f64().unwrap_or(0.0)
                                                    } else {
                                                        0.0
                                                    };

                                                    // Use whichever extended price is available during active extended sessions.
                                                    let ext_price = if pre_price > 0.0 { pre_price } else if post_price > 0.0 { post_price } else { 0.0 };

                                                    if ext_price > 0.0 && row.prev_close > 0.0 {
                                                        row.last = ext_price;
                                                        row.change = ext_price - row.prev_close;
                                                        row.change_pct = (ext_price / row.prev_close - 1.0) * 100.0;
                                                        // Ext% = change from regular close to ext price
                                                        if reg_price > 0.0 {
                                                            row.ext_change_pct = (ext_price / reg_price - 1.0) * 100.0;
                                                        } else {
                                                            row.ext_change_pct = row.change_pct;
                                                        }
                                                    } else if reg_price > 0.0 && row.prev_close > 0.0 {
                                                        // No ext hours — use Yahoo regular price (may be fresher than Alpaca)
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
                        }
                    }
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        for row in &mut rows {
                            if row.last > 0.0 && row.last.is_finite() {
                                continue;
                            }
                            let mut filled = false;
                            'cache_fallback: for tf in ["quote", "1Day", "4Hour", "1Hour", "30Min", "15Min"] {
                                for source in watchlist_cache_fallback_sources(&row.symbol) {
                                    for key in chart_source_cache_keys(source, &row.symbol, tf) {
                                        let Ok(Some(raw)) = cache.get_bars_raw(&key) else {
                                            continue;
                                        };
                                        if let Some(cached) =
                                            watchlist_row_from_raw_bars(&row.symbol, &key, &raw)
                                        {
                                            *row = cached;
                                            filled = true;
                                            break 'cache_fallback;
                                        }
                                    }
                                }
                            }
                            if !filled {
                                tracing::debug!(
                                    "watchlist: no broker/Yahoo/cache quote for {}",
                                    row.symbol
                                );
                            }
                        }
                    }
                    let _ = broker_msg_tx_clone.send(BrokerMsg::WatchlistQuotes(rows));
                }
                BrokerCmd::GetMarketClock => {
                    if let Some(ref b) = broker {
                        match b.get_market_clock().await {
                            Ok(v) => {
                                let is_open = v["is_open"].as_bool().unwrap_or(false);
                                let next_open = v["next_open"].as_str().unwrap_or("—");
                                let next_close = v["next_close"].as_str().unwrap_or("—");

                                // Session-aware label (pre-market / core / after-hours /
                                // closed) instead of a binary OPEN/CLOSED. Alpaca's clock
                                // `is_open` only flags the core session, so the old text read
                                // "US equities CLOSED" all through pre-market; this overlays the
                                // fixed ET session windows while keeping Alpaca's holiday and
                                // half-day accuracy via is_open/next_open.
                                let next_open_utc = chrono::DateTime::parse_from_rfc3339(next_open)
                                    .ok()
                                    .map(|dt| dt.with_timezone(&chrono::Utc));
                                let next_close_utc = chrono::DateTime::parse_from_rfc3339(next_close)
                                    .ok()
                                    .map(|dt| dt.with_timezone(&chrono::Utc));
                                let msg = crate::app::app_runtime_support::us_equities_session_status_at(
                                    chrono::Utc::now(),
                                    is_open,
                                    next_open_utc,
                                    next_close_utc,
                                );
                                let _ = broker_msg_tx_clone.send(BrokerMsg::MarketClock(msg));
                            }
                            Err(e) => {
                                let _ = broker_msg_tx_clone.send(BrokerMsg::Error(e));
                            }
                        }
                    }
                }
                BrokerCmd::GetActivities { limit } => {
                    if let Some(ref b) = broker {
                        match b.get_account_activities("FILL", limit).await {
                            Ok(activities) => {
                                let text = activities.iter().take(20).map(|a| {
                                    format!("{} {} {} {} {}", a.date, a.side.as_deref().unwrap_or("—"), a.qty.as_deref().unwrap_or("—"), a.symbol.as_deref().unwrap_or("—"), a.net_amount.as_deref().unwrap_or("—"))
                                }).collect::<Vec<_>>().join("\n");
                                let _ = broker_msg_tx_clone.send(BrokerMsg::JsonResult("Account Activities".into(), text));
                                // Also send structured fills for chart overlay
                                let fills: Vec<(String, String, f64, f64, String)> = activities.iter()
                                    .filter(|a| a.activity_type == "FILL")
                                    .filter_map(|a| {
                                        let sym = a.symbol.as_deref()?.to_string();
                                        let side = a.side.as_deref()?.to_string();
                                        let qty: f64 = a.qty.as_deref()?.parse().ok()?;
                                        let price: f64 = a.price.as_deref()?.parse().ok()?;
                                        Some((sym, side, qty, price, a.date.clone()))
                                    }).collect();
                                if !fills.is_empty() {
                                    let _ = broker_msg_tx_clone.send(BrokerMsg::RecentFills(fills));
                                }
                            }
                            Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(e)); }
                        }
                    }
                }
                BrokerCmd::GetTopMovers => {
                    if let Some(ref b) = broker {
                        match b.get_top_movers("stocks", 10).await {
                            Ok(v) => {
                                let text: String = serde_json::to_string_pretty(&v).unwrap_or_default();
                                let _ = broker_msg_tx_clone.send(BrokerMsg::JsonResult("Top Movers".into(), text));
                            }
                            Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(e)); }
                        }
                    }
                }
                BrokerCmd::GetAllAssets => {
                    if let Some(ref b) = broker {
                        match b.get_all_assets().await {
                            Ok(assets) => {
                                let all: Vec<(String, String, String)> = assets.iter()
                                    .map(|a| (a.symbol.clone(), a.name.clone(), a.asset_class.clone()))
                                    .collect();
                                let _ = broker_msg_tx_clone.send(BrokerMsg::AllAssets(all));
                            }
                            Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(format!("Asset fetch failed: {e}"))); }
                        }
                    }
                }
                BrokerCmd::SearchSymbols { query } => {
                    let q = query.to_uppercase();
                    let q_without_usd = q.replace("USD", "");
                    let mut all_suggestions: Vec<(String, String, String)> = Vec::new();
                    let mut suggestion_symbols: HashSet<String> = HashSet::new();

                    // Search Alpaca assets
                    if let Some(ref b) = broker {
                        if let Ok(assets) = b.get_all_assets().await {
                            let mut matches: Vec<(u8, &_)> = assets.iter()
                                .filter_map(|a| {
                                    let sym = a.symbol.to_uppercase();
                                    let sym_no_slash = sym.replace('/', "");
                                    if sym == q || sym_no_slash == q { Some((0, a)) }
                                    else if sym.starts_with(&q) || sym_no_slash.starts_with(&q) { Some((1, a)) }
                                    else if sym.contains(&q) || sym_no_slash.contains(&q) { Some((2, a)) }
                                    else if a.name.to_uppercase().contains(&q) { Some((3, a)) }
                                    else { None }
                                })
                                .collect();
                            matches.sort_by_key(|(pri, _)| *pri);
                            for (_, a) in matches.iter().take(15) {
                                if suggestion_symbols.insert(a.symbol.to_uppercase()) {
                                    all_suggestions.push((a.symbol.clone(), a.name.clone(), format!("Alpaca {}", a.asset_class)));
                                }
                            }
                        }
                    }

                    // Search common Kraken crypto symbols by pattern.
                    {
                        let crypto_bases = ["BTC","ETH","SOL","DOGE","XRP","ADA","LTC","LINK","AVAX","DOT",
                            "XMR","ZEC","DASH","UNI","AAVE","ATOM","NEAR","FIL","ICP","XLM","ALGO",
                            "VET","HBAR","FTM","SAND","MANA","AXS","GRT","ENJ","BAT","COMP","MKR",
                            "SNX","CRV","SUSHI","YFI","TRX","ETC","EOS","XTZ","SHIB","APE","ARB","OP","THETA","KAVA",
                            "MATIC","BCH","DOT"];
                        for base in &crypto_bases {
                            let sym = format!("{}USD", base);
                            if sym.contains(&q) || base.contains(&q_without_usd) {
                                if suggestion_symbols.insert(sym.clone()) {
                                    all_suggestions.push((sym, format!("{} (crypto)", base), "Kraken".into()));
                                }
                            }
                        }
                    }

                    if !all_suggestions.is_empty() {
                        let text = all_suggestions.iter().take(25)
                            .map(|(s, n, src)| format!("{} — {} [{}]", s, n, src))
                            .collect::<Vec<_>>().join("\n");
                        let _ = broker_msg_tx_clone.send(BrokerMsg::JsonResult("Symbol Search".into(), text));
                        let suggestions: Vec<(String, String, String)> = all_suggestions.into_iter().take(25).collect();
                        let _ = broker_msg_tx_clone.send(BrokerMsg::SymbolSuggestions(suggestions));
                    }
                }
                BrokerCmd::GetOrderHistory { limit } => {
                    if let Some(ref b) = broker {
                        match b.get_orders("closed", limit).await {
                            Ok(orders) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Orders(orders)); }
                            Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(e)); }
                        }
                    }
                }
                BrokerCmd::GetFundamentals { ticker } => {
                    match AlpacaBroker::get_financial_analysis(&ticker).await {
                        Ok(v) => {
                            let text: String = serde_json::to_string_pretty(&v).unwrap_or_default();
                            let _ = broker_msg_tx_clone.send(BrokerMsg::JsonResult(format!("Fundamentals: {}", ticker), text));
                        }
                        Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(e)); }
                    }
                }
                BrokerCmd::GetHolders { ticker } => {
                    match AlpacaBroker::get_institutional_holders(&ticker).await {
                        Ok(v) => {
                            let text: String = serde_json::to_string_pretty(&v).unwrap_or_default();
                            let _ = broker_msg_tx_clone.send(BrokerMsg::JsonResult(format!("Holders: {}", ticker), text));
                        }
                        Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(e)); }
                    }
                }
                BrokerCmd::GetAnalyst { symbol, finnhub_key } => {
                    if let Some(ref b) = broker {
                        match b.get_finnhub_recommendations(&symbol, &finnhub_key).await {
                            Ok(v) => {
                                let text: String = serde_json::to_string_pretty(&v).unwrap_or_default();
                                let _ = broker_msg_tx_clone.send(BrokerMsg::JsonResult(format!("Analyst: {}", symbol), text));
                            }
                            Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(e)); }
                        }
                    }
                }
                BrokerCmd::GetOrderbook { symbol } => {
                    let try_kraken =
                        typhoon_engine::core::kraken::to_kraken_pair_lossy(&symbol).is_some();
                    if try_kraken {
                        let kraken_result = if let Some(ref kb) = kraken_broker {
                            kb.get_orderbook_snapshot(&symbol, 100).await
                        } else {
                            let kb = typhoon_engine::broker::kraken::KrakenBroker::new(
                                String::new(),
                                String::new(),
                            );
                            kb.get_orderbook_snapshot(&symbol, 100).await
                        };
                        match kraken_result {
                            Ok(v) => {
                                let text: String = serde_json::to_string_pretty(&v).unwrap_or_default();
                                let _ = broker_msg_tx_clone.send(BrokerMsg::JsonResult(format!("Orderbook: {}", symbol), text));
                                continue;
                            }
                            Err(e) if broker.is_none() => {
                                let _ = broker_msg_tx_clone.send(BrokerMsg::Error(e));
                                continue;
                            }
                            Err(_) => {}
                        }
                    }
                    if let Some(ref b) = broker {
                        match b.get_orderbook(&symbol).await {
                            Ok(v) => {
                                let text: String = serde_json::to_string_pretty(&v).unwrap_or_default();
                                let _ = broker_msg_tx_clone.send(BrokerMsg::JsonResult(format!("Orderbook: {}", symbol), text));
                            }
                            Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(e)); }
                        }
                    }
                }
                BrokerCmd::GetMostActive => {
                    if let Some(ref b) = broker {
                        match b.get_most_active(20).await {
                            Ok(v) => {
                                let text = serde_json::to_string_pretty(&v).unwrap_or_default();
                                let _ = broker_msg_tx_clone.send(BrokerMsg::JsonResult("Most Active".into(), text));
                            }
                            Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(e)); }
                        }
                    }
                }
                BrokerCmd::GetPortfolioHistory { period } => {
                    if let Some(ref b) = broker {
                        match b.get_portfolio_history(&period, "1D").await {
                            Ok(v) => {
                                let text = serde_json::to_string_pretty(&v).unwrap_or_default();
                                let _ = broker_msg_tx_clone.send(BrokerMsg::JsonResult("Portfolio History".into(), text));
                            }
                            Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(e)); }
                        }
                    }
                }
                BrokerCmd::GetPriceTarget { symbol, finnhub_key } => {
                    if let Some(ref b) = broker {
                        match b.get_finnhub_price_target(&symbol, &finnhub_key).await {
                            Ok(v) => {
                                let text = serde_json::to_string_pretty(&v).unwrap_or_default();
                                let _ = broker_msg_tx_clone.send(BrokerMsg::JsonResult(format!("PriceTarget: {}", symbol), text));
                            }
                            Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(e)); }
                        }
                    }
                }
                BrokerCmd::GetShortInterest { symbol, finnhub_key } => {
                    if let Some(ref b) = broker {
                        match b.get_finnhub_short_interest(&symbol, &finnhub_key).await {
                            Ok(v) => {
                                if let Some(cache) =
                                    shared_cache_broker.read().ok().and_then(|g| g.clone())
                                {
                                    if let Ok(conn) = cache.connection() {
                                        let rows = typhoon_engine::core::research::short_interest_history_points_from_json_rows(&v);
                                        if !rows.is_empty() {
                                            let _ = typhoon_engine::core::research::upsert_short_interest_history(
                                                &conn,
                                                &symbol,
                                                &rows,
                                            );
                                        }
                                    }
                                }
                                let text = serde_json::to_string_pretty(&v).unwrap_or_default();
                                let _ = broker_msg_tx_clone.send(BrokerMsg::JsonResult(format!("ShortInterest: {}", symbol), text));
                            }
                            Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(e)); }
                        }
                    }
                }
                BrokerCmd::GetCorporateActions { symbol } => {
                    if let Some(ref b) = broker {
                        match b.get_corporate_actions(&symbol).await {
                            Ok(v) => {
                                let text = serde_json::to_string_pretty(&v).unwrap_or_default();
                                let _ = broker_msg_tx_clone.send(BrokerMsg::JsonResult(format!("CorporateActions: {}", symbol), text));
                            }
                            Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(e)); }
                        }
                    }
                }
                BrokerCmd::GetWatchlists => {
                    if let Some(ref b) = broker {
                        match b.get_watchlists().await {
                            Ok(v) => {
                                let text = serde_json::to_string_pretty(&v).unwrap_or_default();
                                let _ = broker_msg_tx_clone.send(BrokerMsg::JsonResult("Watchlists".into(), text));
                            }
                            Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(e)); }
                        }
                    }
                }
                BrokerCmd::CreateWatchlist { name, symbols } => {
                    if let Some(ref b) = broker {
                        match b.create_watchlist(&name, &symbols).await {
                            Ok(_) => { let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult(format!("Watchlist '{}' created ({} symbols)", name, symbols.len()))); }
                            Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(e)); }
                        }
                    }
                }
                BrokerCmd::GetOptionsChain { symbol, expiry } => {
                    if let Some(ref b) = broker {
                        match b.get_options_chain(&symbol, &expiry).await {
                            Ok(contracts) => {
                                let text = serde_json::to_string_pretty(&contracts).unwrap_or_default();
                                let _ = broker_msg_tx_clone.send(BrokerMsg::JsonResult(format!("OptionsChain: {}", symbol), text));
                            }
                            Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(e)); }
                        }
                    }
                }
                BrokerCmd::AlpacaMarketOrder { symbol, qty, side } => {
                    if let Some(ref b) = broker {
                        match b.market_order(&symbol, qty, &side).await {
                            Ok(r) => { let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult(format!("{} {} {} @ market: {}", side, qty, symbol, r.status))); }
                            Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(format!("Order failed: {}", e))); }
                        }
                    }
                }
                BrokerCmd::AlpacaLimitOrder { symbol, qty, side, limit_price } => {
                    if let Some(ref b) = broker {
                        match b.limit_order(&symbol, qty, &side, limit_price, "gtc").await {
                            Ok(r) => { let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult(format!("{} {} {} limit {}: {}", side, qty, symbol, limit_price, r.status))); }
                            Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(format!("Order failed: {}", e))); }
                        }
                    }
                }
                BrokerCmd::AlpacaStopOrder { symbol, qty, side, stop_price } => {
                    if let Some(ref b) = broker {
                        match b.stop_order(&symbol, qty, &side, stop_price, "gtc").await {
                            Ok(r) => { let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult(format!("{} {} {} stop {}: {}", side, qty, symbol, stop_price, r.status))); }
                            Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(format!("Order failed: {}", e))); }
                        }
                    }
                }
                BrokerCmd::AlpacaBracketOrder { symbol, qty, side, stop_loss, take_profit } => {
                    if let Some(ref b) = broker {
                        match b.bracket_order(&symbol, qty, &side, take_profit, stop_loss).await {
                            Ok(r) => { let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult(format!("Bracket {} {} {}: {}", side, qty, symbol, r.status))); }
                            Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(format!("Bracket order failed: {}", e))); }
                        }
                    }
                }
                BrokerCmd::AlpacaCancelOrder { order_id } => {
                    if let Some(ref b) = broker {
                        match b.cancel_order(&order_id).await {
                            Ok(_) => { let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult(format!("Order {} cancelled", order_id))); }
                            Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(format!("Cancel failed: {}", e))); }
                        }
                    }
                }
                BrokerCmd::AlpacaOcoOrder { symbol, qty, side, tp_price, sl_price } => {
                    if let Some(ref b) = broker {
                        match b.oco_order(&symbol, qty, &side, tp_price, sl_price, None).await {
                            Ok(r) => { let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult(format!("OCO {} {} {} @ TP:{} SL:{}: {}", side, qty, symbol, tp_price, sl_price, r.status))); }
                            Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(format!("OCO failed: {}", e))); }
                        }
                    }
                }
                BrokerCmd::AlpacaModifyOrder { order_id, qty, limit_price, stop_price } => {
                    if let Some(ref b) = broker {
                        match b.modify_order(&order_id, qty, limit_price, stop_price, None).await {
                            Ok(r) => { let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult(format!("Order {} modified: {}", order_id, r.status))); }
                            Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(format!("Modify failed: {}", e))); }
                        }
                    }
                }
                BrokerCmd::AlpacaSyncExits {
                    symbol,
                    sl_price,
                    tp_price,
                    wait_for_qty_at_most,
                } => {
                    if let Some(ref b) = broker {
                        if let Some(max_qty) = wait_for_qty_at_most {
                            let mut ready = false;
                            for _ in 0..12 {
                                match b.get_positions().await {
                                    Ok(positions) => {
                                        if positions.iter().any(|p| {
                                            p.symbol.eq_ignore_ascii_case(&symbol)
                                                && p.qty.abs() > 0.0
                                                && p.qty.abs() <= max_qty + 1e-8
                                        }) {
                                            ready = true;
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        let _ = broker_msg_tx_clone.send(BrokerMsg::Error(
                                            format!(
                                                "Alpaca exit sync {}: position poll failed: {}",
                                                symbol, e
                                            ),
                                        ));
                                        break;
                                    }
                                }
                                tokio::time::sleep(std::time::Duration::from_millis(400)).await;
                            }
                            if !ready {
                                let _ = broker_msg_tx_clone.send(BrokerMsg::Error(format!(
                                    "Alpaca exit sync {}: reduced position not visible yet",
                                    symbol
                                )));
                                continue;
                            }
                        }
                        match b.sync_position_exits(&symbol, sl_price, tp_price).await {
                            Ok(summary) => {
                                let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult(
                                    format!("Alpaca exits {}: {}", symbol, summary),
                                ));
                            }
                            Err(e) => {
                                let _ = broker_msg_tx_clone.send(BrokerMsg::Error(format!(
                                    "Alpaca exit sync failed for {}: {}",
                                    symbol, e
                                )));
                            }
                        }
                    }
                }
                BrokerCmd::AiChat { provider, api_key, message, history, system, model } => {
                    let client = reqwest::Client::new();
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        // The base system prompt: trading assistant + research packet if supplied.
                        let base_system = "You are a trading assistant inside TyphooN-Terminal. \
When the question touches recent news, sentiment, or prices, combine the research packet \
(if provided) with your own live web search and cite the sources you rely on.".to_string();
                        let full_system = match &system {
                            Some(packet) if !packet.is_empty() => format!(
                                "{base_system}\n\n=== RESEARCH PACKET ===\n{packet}\n=== END RESEARCH PACKET ==="
                            ),
                            _ => base_system,
                        };

                        // Build the message chain (history + new user turn).
                        let msgs: Vec<serde_json::Value> = history.iter()
                            .map(|(is_user, text)| serde_json::json!({"role": if *is_user { "user" } else { "assistant" }, "content": text}))
                            .chain(std::iter::once(serde_json::json!({"role": "user", "content": message})))
                            .collect();

                        // ── cross-client AI response cache lookup ──
                        // Compute deterministic hash over the full prompt tuple and check
                        // the LAN-synced cache before spending tokens. On hit, emit the
                        // cached response and skip the HTTP call entirely.
                        use typhoon_engine::core::ai_response_cache as arc_cache;
                        let cache_provider_tag = match provider.as_str() {
                            "claude" => "claude_http",
                            other => other,
                        };
                        let cache_model = model.clone().unwrap_or_else(|| match provider.as_str() {
                            "claude" => "claude-opus-4-5".to_string(),
                            "openai" => "gpt-4o".into(),
                            "gemini" => TyphooNApp::default_gemini_cli_model().into(),
                            "grok" => "grok-3".into(),
                            "mistral" => "mistral-large-latest".into(),
                            "perplexity" => "sonar-pro".into(),
                            "local" => "llama3.2".into(),
                            _ => "unknown".into(),
                        });
                        let prompt_hash = arc_cache::hash_ai_prompt(
                            cache_provider_tag, &cache_model, &full_system, &history, &message,
                        );
                        let cache_snapshot = shared_cache_broker.read().ok().and_then(|g| g.clone());
                        if let Some(cache) = cache_snapshot.as_ref() {
                            if let Ok(Some(hit)) = arc_cache::lookup_response(cache, &prompt_hash) {
                                let _ = msg_tx.send(BrokerMsg::JsonResult("AiChat".into(), hit.response));
                                return;
                            }
                        }

                        if provider == "claude" {
                            // Anthropic uses its own API format (not OpenAI-compatible).
                            // `system` goes in its own top-level field, not as a role.
                            let anth_model = model.clone().unwrap_or_else(|| "claude-opus-4-5".to_string());
                            let body = serde_json::json!({
                                "model": anth_model,
                                "max_tokens": 4096,
                                "system": full_system,
                                "messages": msgs,
                            });
                            match client.post("https://api.anthropic.com/v1/messages")
                                .header("x-api-key", &api_key).header("anthropic-version", "2023-06-01")
                                .header("content-type", "application/json").json(&body).send().await {
                                Ok(resp) => {
                                    let text = resp.json::<serde_json::Value>().await.ok()
                                        .and_then(|j| j["content"][0]["text"].as_str().map(|s| s.to_string()))
                                        .unwrap_or_else(|| "(no response)".into());
                                    // record the fresh response in the LAN-synced cache.
                                    if text != "(no response)" {
                                        if let Some(cache) = cache_snapshot.as_ref() {
                                            let preview: String = message.chars().take(400).collect();
                                            let host = std::env::var("HOSTNAME").unwrap_or_default();
                                            let _ = arc_cache::upsert_response(cache, &arc_cache::AiResponseCacheEntry {
                                                prompt_hash: prompt_hash.clone(),
                                                provider: cache_provider_tag.to_string(),
                                                model: anth_model.clone(),
                                                prompt_preview: preview,
                                                response: text.clone(),
                                                token_count_prompt: arc_cache::estimate_tokens(&full_system) + arc_cache::estimate_tokens(&message),
                                                token_count_completion: arc_cache::estimate_tokens(&text),
                                                created_at: 0, updated_at: 0, hit_count: 0,
                                                source_client: host,
                                            });
                                        }
                                    }
                                    let _ = msg_tx.send(BrokerMsg::JsonResult("AiChat".into(), text));
                                }
                                Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("Claude API: {}", e))); }
                            }
                        } else {
                            // OpenAI-compatible endpoint (GPT, Gemini, Grok, Mistral, Perplexity, Ollama)
                            let (url, default_model, auth_header) = match provider.as_str() {
                                "openai" => ("https://api.openai.com/v1/chat/completions", "gpt-4o", format!("Bearer {}", api_key)),
                                "gemini" => ("https://generativelanguage.googleapis.com/v1beta/openai/chat/completions", TyphooNApp::default_gemini_cli_model(), format!("Bearer {}", api_key)),
                                "grok" => ("https://api.x.ai/v1/chat/completions", "grok-3", format!("Bearer {}", api_key)),
                                "mistral" => ("https://api.mistral.ai/v1/chat/completions", "mistral-large-latest", format!("Bearer {}", api_key)),
                                "perplexity" => ("https://api.perplexity.ai/chat/completions", "sonar-pro", format!("Bearer {}", api_key)),
                                "local" => {
                                    // Ollama / LM Studio: local OpenAI-compatible server
                                    let local_url = if api_key.starts_with("http") { api_key.as_str() } else { "http://localhost:11434" };
                                    (if local_url.contains("11434") { "http://localhost:11434/v1/chat/completions" } else { "http://localhost:1234/v1/chat/completions" },
                                     "llama3.2", String::new())
                                }
                                _ => ("https://api.openai.com/v1/chat/completions", "gpt-4o", format!("Bearer {}", api_key)),
                            };
                            let effective_model = model.clone().unwrap_or_else(|| default_model.to_string());
                            let mut all = vec![serde_json::json!({"role": "system", "content": full_system})];
                            all.extend(msgs);
                            let body = serde_json::json!({"model": effective_model, "messages": all, "max_tokens": 4096});
                            let mut req = client.post(url).header("content-type", "application/json").json(&body);
                            if !auth_header.is_empty() {
                                req = req.header("Authorization", &auth_header);
                            }
                            match req.send().await {
                                Ok(resp) => {
                                    let text = resp.json::<serde_json::Value>().await.ok()
                                        .and_then(|j| j["choices"][0]["message"]["content"].as_str().map(|s| s.to_string()))
                                        .unwrap_or_else(|| "(no response)".into());
                                    // record the fresh response in the LAN-synced cache.
                                    if text != "(no response)" {
                                        if let Some(cache) = cache_snapshot.as_ref() {
                                            let preview: String = message.chars().take(400).collect();
                                            let host = std::env::var("HOSTNAME").unwrap_or_default();
                                            let _ = arc_cache::upsert_response(cache, &arc_cache::AiResponseCacheEntry {
                                                prompt_hash: prompt_hash.clone(),
                                                provider: cache_provider_tag.to_string(),
                                                model: effective_model.clone(),
                                                prompt_preview: preview,
                                                response: text.clone(),
                                                token_count_prompt: arc_cache::estimate_tokens(&full_system) + arc_cache::estimate_tokens(&message),
                                                token_count_completion: arc_cache::estimate_tokens(&text),
                                                created_at: 0, updated_at: 0, hit_count: 0,
                                                source_client: host,
                                            });
                                        }
                                    }
                                    let _ = msg_tx.send(BrokerMsg::JsonResult("AiChat".into(), text));
                                }
                                Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("{} API: {}", provider, e))); }
                            }
                        }
                    });
                }
                BrokerCmd::MatrixJoinRoom { room_id, access_token } => {
                    let client = reqwest::Client::new();
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        // Encode the room ID/alias path segment. matrix.org requires %21/%23/%3A
                        // for the URL path portion.
                        let encoded_room = room_id.replace('!', "%21").replace('#', "%23").replace(':', "%3A");
                        // server_name hint — derived from the suffix of the room id/alias.
                        // For room IDs like "!abc:matrix.org" the Matrix spec recommends passing
                        // ?server_name=matrix.org so your homeserver knows which federation peer
                        // to ask about the room. Without it, homeservers that haven't yet
                        // resolved the room return "M_UNKNOWN: No known servers".
                        let server_name = room_id.rsplit(':').next().unwrap_or("matrix.org").to_string();
                        let url = format!(
                            "https://matrix.org/_matrix/client/v3/join/{}?server_name={}",
                            encoded_room, server_name
                        );
                        match client.post(&url)
                            .header("Authorization", format!("Bearer {}", access_token))
                            .json(&serde_json::json!({}))
                            .send().await {
                            Ok(resp) if resp.status().is_success() => {
                                let _ = msg_tx.send(BrokerMsg::JsonResult("MatrixJoined".into(), "ok".into()));
                            }
                            Ok(resp) => {
                                let text = resp.text().await.unwrap_or_default();
                                // M_ALREADY_JOINED is fine
                                if text.contains("already") {
                                    let _ = msg_tx.send(BrokerMsg::JsonResult("MatrixJoined".into(), "already".into()));
                                } else {
                                    let _ = msg_tx.send(BrokerMsg::Error(format!("Matrix join: {}", text)));
                                }
                            }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("Matrix join: {}", e))); }
                        }
                    });
                }
                BrokerCmd::MatrixFetchMessages { room_id, access_token } => {
                    let client = reqwest::Client::new();
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let encoded_room = room_id.replace('!', "%21").replace(':', "%3A");
                        let url = format!("https://matrix.org/_matrix/client/r0/rooms/{}/messages?dir=b&limit=50", encoded_room);
                        let mut req = client.get(&url).header("User-Agent", "TyphooN-Terminal/1.0");
                        if !access_token.is_empty() {
                            req = req.header("Authorization", format!("Bearer {}", access_token));
                        }
                        match req.send().await {
                            Ok(resp) => {
                                if let Ok(json) = resp.json::<serde_json::Value>().await {
                                    let mut msgs = Vec::new();
                                    if let Some(chunk) = json["chunk"].as_array() {
                                        for ev in chunk.iter().rev() {
                                            if ev["type"].as_str() == Some("m.room.message") {
                                                let sender = ev["sender"].as_str().unwrap_or("?").to_string();
                                                let ts = ev["origin_server_ts"].as_i64().unwrap_or(0);
                                                let dt = chrono::DateTime::from_timestamp(ts / 1000, 0)
                                                    .map(|d| d.format("%H:%M").to_string()).unwrap_or_default();
                                                let body = ev["content"]["body"].as_str().unwrap_or("").to_string();
                                                msgs.push((sender, dt, body));
                                            }
                                        }
                                    }
                                    let text = serde_json::to_string(&msgs).unwrap_or_default();
                                    let _ = msg_tx.send(BrokerMsg::JsonResult("MatrixMessages".into(), text));
                                }
                            }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("Matrix: {}", e))); }
                        }
                    });
                }
                BrokerCmd::MatrixSendImage { room_id, access_token, file_path } => {
                    let client = reqwest::Client::new();
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        // Step 1: Read file
                        let data = match tokio::fs::read(&file_path).await {
                            Ok(d) => d,
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("Read screenshot: {e}"))); return; }
                        };
                        let filename = file_path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_else(|| "screenshot.webp".into());
                        let content_type = if filename.ends_with(".webp") { "image/webp" } else { "image/png" };

                        // Step 2: Upload to Matrix content repository
                        let upload_url = format!("https://matrix.org/_matrix/media/r0/upload?filename={}", filename);
                        match client.post(&upload_url)
                            .header("Authorization", format!("Bearer {}", access_token))
                            .header("Content-Type", content_type)
                            .body(data.clone())
                            .send().await
                        {
                            Ok(resp) => {
                                if let Ok(json) = resp.json::<serde_json::Value>().await {
                                    let mxc_url = json["content_uri"].as_str().unwrap_or("").to_string();
                                    if mxc_url.is_empty() {
                                        let _ = msg_tx.send(BrokerMsg::Error("Matrix upload: no content_uri returned".into()));
                                        return;
                                    }
                                    // Step 3: Send m.image message
                                    let txn_id = format!("typhoon_img_{}", chrono::Utc::now().timestamp_millis());
                                    let encoded_room = room_id.replace('!', "%21").replace(':', "%3A");
                                    let send_url = format!("https://matrix.org/_matrix/client/r0/rooms/{}/send/m.room.message/{}", encoded_room, txn_id);
                                    let msg_body = serde_json::json!({
                                        "msgtype": "m.image",
                                        "body": filename,
                                        "url": mxc_url,
                                        "info": { "mimetype": content_type, "size": data.len() },
                                    });
                                    match client.put(&send_url)
                                        .header("Authorization", format!("Bearer {}", access_token))
                                        .json(&msg_body)
                                        .send().await
                                    {
                                        Ok(r) if r.status().is_success() => {
                                            let _ = msg_tx.send(BrokerMsg::JsonResult("MatrixSent".into(), "image shared".into()));
                                            let _ = msg_tx.send(BrokerMsg::OrderResult(format!("Screenshot shared to community chat")));
                                        }
                                        Ok(r) => {
                                            let text = r.text().await.unwrap_or_default();
                                            let _ = msg_tx.send(BrokerMsg::Error(format!("Matrix send image: {}", text)));
                                        }
                                        Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("Matrix send: {e}"))); }
                                    }
                                }
                            }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("Matrix upload: {e}"))); }
                        }
                    });
                }
                BrokerCmd::MatrixSendMessage { room_id, access_token, body } => {
                    let client = reqwest::Client::new();
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let txn_id = format!("typhoon_{}", chrono::Utc::now().timestamp_millis());
                        let encoded_room = room_id.replace('!', "%21").replace(':', "%3A");
                        let url = format!("https://matrix.org/_matrix/client/r0/rooms/{}/send/m.room.message/{}", encoded_room, txn_id);
                        let msg_body = serde_json::json!({
                            "msgtype": "m.text",
                            "body": body,
                        });
                        match client.put(&url)
                            .header("Authorization", format!("Bearer {}", access_token))
                            .json(&msg_body)
                            .send().await {
                            Ok(resp) => {
                                if resp.status().is_success() {
                                    let _ = msg_tx.send(BrokerMsg::JsonResult("MatrixSent".into(), "ok".into()));
                                } else {
                                    let text = resp.text().await.unwrap_or_default();
                                    let _ = msg_tx.send(BrokerMsg::Error(format!("Matrix send failed: {}", text)));
                                }
                            }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("Matrix send: {}", e))); }
                        }
                    });
                }
                BrokerCmd::AlpacaTrailingStop { symbol, qty, side, trail_percent } => {
                    if let Some(ref b) = broker {
                        match b.trailing_stop_order(&symbol, qty, &side, None, Some(trail_percent), "gtc").await {
                            Ok(r) => { let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult(format!("Trailing stop {} {} {} trail {}%: {}", side, qty, symbol, trail_percent, r.status))); }
                            Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(format!("Trailing stop failed: {}", e))); }
                        }
                    }
                }
                BrokerCmd::AlpacaStopLimitOrder { symbol, qty, side, stop_price, limit_price } => {
                    if let Some(ref b) = broker {
                        match b.stop_limit_order(&symbol, qty, &side, stop_price, limit_price, "gtc").await {
                            Ok(r) => { let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult(format!("Stop-limit {} {} {} stop={} lim={}: {}", side, qty, symbol, stop_price, limit_price, r.status))); }
                            Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(format!("Stop-limit failed: {}", e))); }
                        }
                    }
                }
                BrokerCmd::KrakenSyncExits {
                    pair,
                    sl_price,
                    tp_price,
                    wait_for_position,
                    wait_for_qty_at_most,
                } => {
                    if let Some(ref kb) = kraken_broker {
                        if wait_for_position || wait_for_qty_at_most.is_some() {
                            let mut found = false;
                            for _ in 0..12 {
                                match kb.get_position_summaries().await {
                                    Ok(positions)
                                        if positions.iter().any(|p| {
                                            p.symbol.eq_ignore_ascii_case(&pair)
                                                && p.qty.abs() > 0.0
                                                && wait_for_qty_at_most
                                                    .map(|max_qty| p.qty.abs() <= max_qty + 1e-8)
                                                    .unwrap_or(true)
                                        }) =>
                                    {
                                        found = true;
                                        break;
                                    }
                                    Ok(_) => {
                                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                                    }
                                    Err(e) => {
                                        let _ = broker_msg_tx_clone.send(BrokerMsg::Error(format!(
                                            "Kraken exit sync {}: position poll failed: {}",
                                            pair, e
                                        )));
                                        break;
                                    }
                                }
                            }
                            if !found {
                                let _ = broker_msg_tx_clone.send(BrokerMsg::Error(format!(
                                    "Kraken exit sync {}: position not visible at target size yet",
                                    pair
                                )));
                                continue;
                            }
                        }
                        match kb.sync_position_exits(&pair, sl_price, tp_price).await {
                            Ok(summary) => {
                                let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult(
                                    format!("Kraken exits {}: {}", pair, summary),
                                ));
                            }
                            Err(e) => {
                                let _ = broker_msg_tx_clone.send(BrokerMsg::Error(format!(
                                    "Kraken exit sync failed for {}: {}",
                                    pair, e
                                )));
                            }
                        }
                    } else {
                        let _ = broker_msg_tx_clone.send(BrokerMsg::Error("Kraken: not connected".into()));
                    }
                }
                BrokerCmd::FetchFearGreed => {
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let client = reqwest::Client::new();
                        match client.get("https://api.alternative.me/fng/?limit=1").send().await {
                            Ok(resp) => {
                                if let Ok(json) = resp.json::<serde_json::Value>().await {
                                    if let Some(data) = json["data"].as_array().and_then(|a| a.first()) {
                                        let value = data["value"].as_str().and_then(|v| v.parse::<u32>().ok()).unwrap_or(50);
                                        let label = data["value_classification"].as_str().unwrap_or("Neutral").to_string();
                                        let _ = msg_tx.send(BrokerMsg::JsonResult("FearGreed".into(), format!("{}|{}", value, label)));
                                    }
                                }
                            }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("Fear & Greed: {}", e))); }
                        }
                    });
                }
                BrokerCmd::FetchRedditWSB => {
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let client = reqwest::Client::builder().user_agent("TyphooN-Terminal/1.0").build().unwrap_or_default();
                        match client.get("https://www.reddit.com/r/wallstreetbets/hot.json?limit=25").send().await {
                            Ok(resp) => {
                                if let Ok(json) = resp.json::<serde_json::Value>().await {
                                    let mut posts = Vec::new();
                                    if let Some(children) = json["data"]["children"].as_array() {
                                        for child in children {
                                            let d = &child["data"];
                                            let title = d["title"].as_str().unwrap_or("").to_string();
                                            let url = d["permalink"].as_str().map(|p| format!("https://reddit.com{}", p)).unwrap_or_default();
                                            let score = d["score"].as_u64().unwrap_or(0);
                                            let comments = d["num_comments"].as_u64().unwrap_or(0);
                                            if !title.is_empty() { posts.push((title, url, score, comments)); }
                                        }
                                    }
                                    let text = serde_json::to_string(&posts).unwrap_or_default();
                                    let _ = msg_tx.send(BrokerMsg::JsonResult("RedditWSB".into(), text));
                                }
                            }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("Reddit: {}", e))); }
                        }
                    });
                }
                BrokerCmd::FetchCryptoTop50 => {
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let client = reqwest::Client::new();
                        match client.get("https://api.coingecko.com/api/v3/coins/markets")
                            .query(&[("vs_currency", "usd"), ("order", "market_cap_desc"), ("per_page", "50"), ("page", "1")])
                            .header("User-Agent", "TyphooN-Terminal/1.0")
                            .send().await {
                            Ok(resp) => {
                                if let Ok(json) = resp.json::<serde_json::Value>().await {
                                    if let Some(arr) = json.as_array() {
                                        let data: Vec<(String, f64, f64, f64)> = arr.iter().map(|c| {
                                            let name = format!("{} ({})", c["name"].as_str().unwrap_or("?"), c["symbol"].as_str().unwrap_or("?").to_uppercase());
                                            let price = c["current_price"].as_f64().unwrap_or(0.0);
                                            let change = c["price_change_percentage_24h"].as_f64().unwrap_or(0.0);
                                            let mcap = c["market_cap"].as_f64().unwrap_or(0.0);
                                            (name, price, change, mcap)
                                        }).collect();
                                        let _ = msg_tx.send(BrokerMsg::CryptoTop50(data));
                                    }
                                }
                            }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("CoinGecko: {}", e))); }
                        }
                    });
                }
                // ── Godel parity research handlers (ADR-107) ──
                BrokerCmd::FetchCompanyProfile { symbol, finnhub_key } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let client = reqwest::Client::builder()
                            .user_agent("TyphooN-Terminal/1.0")
                            .timeout(std::time::Duration::from_secs(15))
                            .build().unwrap_or_default();
                        match research::fetch_finnhub_profile(&client, &symbol, &finnhub_key).await {
                            Ok(p) => { let _ = msg_tx.send(BrokerMsg::CompanyProfile(p)); }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("DES profile: {}", e))); }
                        }
                    });
                }
                BrokerCmd::FetchStockPeers { symbol, finnhub_key } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let client = reqwest::Client::builder()
                            .user_agent("TyphooN-Terminal/1.0")
                            .timeout(std::time::Duration::from_secs(15))
                            .build().unwrap_or_default();
                        match research::fetch_finnhub_peers(&client, &symbol, &finnhub_key).await {
                            Ok(peers) => { let _ = msg_tx.send(BrokerMsg::StockPeers(symbol, peers)); }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("PEERS: {}", e))); }
                        }
                    });
                }
                BrokerCmd::FetchEarningsHistory { symbol, finnhub_key } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let client = reqwest::Client::builder()
                            .user_agent("TyphooN-Terminal/1.0")
                            .timeout(std::time::Duration::from_secs(15))
                            .build().unwrap_or_default();
                        match research::fetch_finnhub_earnings(&client, &symbol, &finnhub_key).await {
                            Ok(rows) => { let _ = msg_tx.send(BrokerMsg::EarningsHistory(symbol, rows)); }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("EARNINGS: {}", e))); }
                        }
                    });
                }
                BrokerCmd::FetchIpoCalendar { finnhub_key, days_ahead, days_back } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let client = reqwest::Client::builder()
                            .user_agent("TyphooN-Terminal/1.0")
                            .timeout(std::time::Duration::from_secs(15))
                            .build().unwrap_or_default();
                        let today = chrono::Utc::now();
                        let from = (today - chrono::Duration::days(days_back.max(0))).format("%Y-%m-%d").to_string();
                        let to = (today + chrono::Duration::days(days_ahead.max(0))).format("%Y-%m-%d").to_string();
                        match research::fetch_finnhub_ipo_calendar(&client, &finnhub_key, &from, &to).await {
                            Ok(rows) => { let _ = msg_tx.send(BrokerMsg::IpoCalendar(rows)); }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("IPO: {}", e))); }
                        }
                    });
                }
                BrokerCmd::FetchPressReleases { symbol, finnhub_key } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let client = reqwest::Client::builder()
                            .user_agent("TyphooN-Terminal/1.0")
                            .timeout(std::time::Duration::from_secs(15))
                            .build().unwrap_or_default();
                        match research::fetch_finnhub_press(&client, &symbol, &finnhub_key).await {
                            Ok(rows) => { let _ = msg_tx.send(BrokerMsg::PressReleases(symbol, rows)); }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("PRESS: {}", e))); }
                        }
                    });
                }
                BrokerCmd::FetchSocialSentiment { symbol, finnhub_key } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let client = reqwest::Client::builder()
                            .user_agent("TyphooN-Terminal/1.0")
                            .timeout(std::time::Duration::from_secs(15))
                            .build().unwrap_or_default();
                        match research::fetch_finnhub_social(&client, &symbol, &finnhub_key).await {
                            Ok(rows) => { let _ = msg_tx.send(BrokerMsg::SocialSentiment(symbol, rows)); }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("SENTIMENT: {}", e))); }
                        }
                    });
                }
                BrokerCmd::FetchTranscriptList { symbol, fmp_key } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let client = reqwest::Client::builder()
                            .user_agent("TyphooN-Terminal/1.0")
                            .timeout(std::time::Duration::from_secs(15))
                            .build().unwrap_or_default();
                        match research::fetch_fmp_transcript_list(&client, &symbol, &fmp_key).await {
                            Ok(rows) => { let _ = msg_tx.send(BrokerMsg::TranscriptList(symbol, rows)); }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("TRANSCRIPTS list: {}", e))); }
                        }
                    });
                }
                BrokerCmd::FetchTranscriptBody { symbol, quarter, year, fmp_key } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let client = reqwest::Client::builder()
                            .user_agent("TyphooN-Terminal/1.0")
                            .timeout(std::time::Duration::from_secs(30))
                            .build().unwrap_or_default();
                        match research::fetch_fmp_transcript(&client, &symbol, quarter, year, &fmp_key).await {
                            Ok(t) => { let _ = msg_tx.send(BrokerMsg::TranscriptBody(t)); }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("TRANSCRIPTS body: {}", e))); }
                        }
                    });
                }
                BrokerCmd::FetchCommoditiesQuotes => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let client = reqwest::Client::builder()
                            .user_agent("Mozilla/5.0 (X11; Linux x86_64) TyphooN-Terminal/0.1")
                            .timeout(std::time::Duration::from_secs(20))
                            .build().unwrap_or_default();
                        let symbols: Vec<&str> = research::COMMODITIES_UNIVERSE.iter().map(|(s, _, _)| *s).collect();
                        match research::fetch_yahoo_quotes(&client, &symbols).await {
                            Ok(quotes) => {
                                let quotes_by_symbol: std::collections::HashMap<&str, &_> =
                                    quotes.iter().map(|q| (q.0.as_str(), q)).collect();
                                let out: Vec<research::CommodityQuote> = research::COMMODITIES_UNIVERSE.iter().map(|(sym, display, _)| {
                                    if let Some(q) = quotes_by_symbol.get(*sym).copied() {
                                        research::CommodityQuote {
                                            symbol: sym.to_string(),
                                            display: display.to_string(),
                                            price: q.1,
                                            change: q.2,
                                            change_pct: q.3,
                                        }
                                    } else {
                                        research::CommodityQuote { symbol: sym.to_string(), display: display.to_string(), ..Default::default() }
                                    }
                                }).collect();
                                let _ = msg_tx.send(BrokerMsg::CommoditiesQuotes(out));
                            }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("GLCO: {}", e))); }
                        }
                    });
                }
                BrokerCmd::FetchDividendHistory { symbol, fmp_key } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let client = reqwest::Client::builder()
                            .user_agent("TyphooN-Terminal/1.0")
                            .timeout(std::time::Duration::from_secs(15))
                            .build().unwrap_or_default();
                        match research::fetch_fmp_dividend_history(&client, &symbol, &fmp_key).await {
                            Ok(rows) => { let _ = msg_tx.send(BrokerMsg::DividendHistory(symbol, rows)); }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("DVD: {}", e))); }
                        }
                    });
                }
                BrokerCmd::FetchEarningsEstimates { symbol, fmp_key } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let client = reqwest::Client::builder()
                            .user_agent("TyphooN-Terminal/1.0")
                            .timeout(std::time::Duration::from_secs(15))
                            .build().unwrap_or_default();
                        match research::fetch_fmp_earnings_estimates(&client, &symbol, &fmp_key).await {
                            Ok(rows) => { let _ = msg_tx.send(BrokerMsg::EarningsEstimates(symbol, rows)); }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("EEB: {}", e))); }
                        }
                    });
                }
                BrokerCmd::FetchRatingChanges { symbol, fmp_key } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let client = reqwest::Client::builder()
                            .user_agent("TyphooN-Terminal/1.0")
                            .timeout(std::time::Duration::from_secs(15))
                            .build().unwrap_or_default();
                        match research::fetch_fmp_rating_changes(&client, &symbol, &fmp_key).await {
                            Ok(rows) => { let _ = msg_tx.send(BrokerMsg::RatingChanges(symbol, rows)); }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("UPDG: {}", e))); }
                        }
                    });
                }
                BrokerCmd::FetchTreasuryYields => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let client = reqwest::Client::builder()
                            .user_agent("Mozilla/5.0 (X11; Linux x86_64) TyphooN-Terminal/0.1")
                            .timeout(std::time::Duration::from_secs(20))
                            .build().unwrap_or_default();
                        match research::fetch_treasury_yields(&client).await {
                            Ok(rows) => { let _ = msg_tx.send(BrokerMsg::TreasuryYields(rows)); }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("GY: {}", e))); }
                        }
                    });
                }
                BrokerCmd::FetchFinancialStatements { symbol, fmp_key } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let client = reqwest::Client::builder()
                            .user_agent("TyphooN-Terminal/1.0")
                            .timeout(std::time::Duration::from_secs(30))
                            .build().unwrap_or_default();
                        match research::fetch_fmp_financial_bundle(&client, &symbol, &fmp_key).await {
                            Ok(bundle) => { let _ = msg_tx.send(BrokerMsg::FinancialStatementsMsg(symbol, bundle)); }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("FA: {}", e))); }
                        }
                    });
                }
                BrokerCmd::FetchExecutives { symbol, finnhub_key } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let client = reqwest::Client::builder()
                            .user_agent("TyphooN-Terminal/1.0")
                            .timeout(std::time::Duration::from_secs(15))
                            .build().unwrap_or_default();
                        match research::fetch_finnhub_executives(&client, &symbol, &finnhub_key).await {
                            Ok(rows) => { let _ = msg_tx.send(BrokerMsg::Executives(symbol, rows)); }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("MGMT: {}", e))); }
                        }
                    });
                }
                BrokerCmd::FetchCotReports => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let client = reqwest::Client::builder()
                            .user_agent("Mozilla/5.0 (X11; Linux x86_64) TyphooN-Terminal/0.1")
                            .timeout(std::time::Duration::from_secs(30))
                            .build().unwrap_or_default();
                        match research::fetch_cftc_cot(&client).await {
                            Ok(rows) => { let _ = msg_tx.send(BrokerMsg::CotReports(rows)); }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("COT: {}", e))); }
                        }
                    });
                }
                BrokerCmd::FetchStockSplits { symbol, fmp_key } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let client = reqwest::Client::builder()
                            .user_agent("TyphooN-Terminal/1.0")
                            .timeout(std::time::Duration::from_secs(15))
                            .build().unwrap_or_default();
                        match research::fetch_fmp_stock_splits(&client, &symbol, &fmp_key).await {
                            Ok(rows) => { let _ = msg_tx.send(BrokerMsg::StockSplitsMsg(symbol, rows)); }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("SPLT: {}", e))); }
                        }
                    });
                }
                BrokerCmd::FetchEtfHoldings { symbol, fmp_key } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let client = reqwest::Client::builder()
                            .user_agent("TyphooN-Terminal/1.0")
                            .timeout(std::time::Duration::from_secs(20))
                            .build().unwrap_or_default();
                        match research::fetch_fmp_etf_holdings(&client, &symbol, &fmp_key).await {
                            Ok(rows) => { let _ = msg_tx.send(BrokerMsg::EtfHoldingsMsg(symbol, rows)); }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("ETF: {}", e))); }
                        }
                    });
                }
                BrokerCmd::FetchAnalystRecs { symbol, finnhub_key } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let client = reqwest::Client::builder()
                            .user_agent("TyphooN-Terminal/1.0")
                            .timeout(std::time::Duration::from_secs(15))
                            .build().unwrap_or_default();
                        match research::fetch_finnhub_recommendations(&client, &symbol, &finnhub_key).await {
                            Ok(rows) => { let _ = msg_tx.send(BrokerMsg::AnalystRecsMsg(symbol, rows)); }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("ANR: {}", e))); }
                        }
                    });
                }
                BrokerCmd::FetchPriceTarget { symbol, finnhub_key } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let client = reqwest::Client::builder()
                            .user_agent("TyphooN-Terminal/1.0")
                            .timeout(std::time::Duration::from_secs(15))
                            .build().unwrap_or_default();
                        match research::fetch_finnhub_price_target(&client, &symbol, &finnhub_key).await {
                            Ok(pt) => { let _ = msg_tx.send(BrokerMsg::PriceTargetMsg(symbol, pt)); }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("PT: {}", e))); }
                        }
                    });
                }
                BrokerCmd::FetchEsgScores { symbol, fmp_key } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let client = reqwest::Client::builder()
                            .user_agent("TyphooN-Terminal/1.0")
                            .timeout(std::time::Duration::from_secs(15))
                            .build().unwrap_or_default();
                        match research::fetch_fmp_esg(&client, &symbol, &fmp_key).await {
                            Ok(rows) => { let _ = msg_tx.send(BrokerMsg::EsgScoresMsg(symbol, rows)); }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("ESG: {}", e))); }
                        }
                    });
                }
                BrokerCmd::FetchIndexMembers { index_code, fmp_key } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let client = reqwest::Client::builder()
                            .user_agent("TyphooN-Terminal/1.0")
                            .timeout(std::time::Duration::from_secs(20))
                            .build().unwrap_or_default();
                        match research::fetch_fmp_index_members(&client, &index_code, &fmp_key).await {
                            Ok(rows) => { let _ = msg_tx.send(BrokerMsg::IndexMembersMsg(index_code, rows)); }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("MEMB: {}", e))); }
                        }
                    });
                }
                BrokerCmd::FetchInsiderTrades { symbol, fmp_key } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let client = reqwest::Client::builder()
                            .user_agent("TyphooN-Terminal/1.0")
                            .timeout(std::time::Duration::from_secs(15))
                            .build().unwrap_or_default();
                        match research::fetch_fmp_insider_trades(&client, &symbol, &fmp_key).await {
                            Ok(rows) => { let _ = msg_tx.send(BrokerMsg::InsiderTradesMsg(symbol, rows)); }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("INS: {}", e))); }
                        }
                    });
                }
                BrokerCmd::FetchInstitutionalHolders { symbol, fmp_key } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let client = reqwest::Client::builder()
                            .user_agent("TyphooN-Terminal/1.0")
                            .timeout(std::time::Duration::from_secs(15))
                            .build().unwrap_or_default();
                        match research::fetch_fmp_institutional_holders(&client, &symbol, &fmp_key).await {
                            Ok(rows) => { let _ = msg_tx.send(BrokerMsg::InstitutionalHoldersMsg(symbol, rows)); }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("HDS: {}", e))); }
                        }
                    });
                }
                BrokerCmd::FetchSharesFloat { symbol, fmp_key } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let client = reqwest::Client::builder()
                            .user_agent("TyphooN-Terminal/1.0")
                            .timeout(std::time::Duration::from_secs(15))
                            .build().unwrap_or_default();
                        match research::fetch_fmp_shares_float(&client, &symbol, &fmp_key).await {
                            Ok(snap) => { let _ = msg_tx.send(BrokerMsg::SharesFloatMsg(symbol, snap)); }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("FLOAT: {}", e))); }
                        }
                    });
                }
                BrokerCmd::FetchHistoricalPrice { symbol, fmp_key, limit } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let client = reqwest::Client::builder()
                            .user_agent("TyphooN-Terminal/1.0")
                            .timeout(std::time::Duration::from_secs(25))
                            .build().unwrap_or_default();
                        match research::fetch_fmp_historical_price(&client, &symbol, &fmp_key, limit).await {
                            Ok(rows) => { let _ = msg_tx.send(BrokerMsg::HistoricalPriceMsg(symbol, rows)); }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("HP: {}", e))); }
                        }
                    });
                }
                BrokerCmd::FetchEarningsSurprises { symbol, fmp_key } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let client = reqwest::Client::builder()
                            .user_agent("TyphooN-Terminal/1.0")
                            .timeout(std::time::Duration::from_secs(15))
                            .build().unwrap_or_default();
                        match research::fetch_fmp_earnings_surprises(&client, &symbol, &fmp_key).await {
                            Ok(rows) => { let _ = msg_tx.send(BrokerMsg::EarningsSurpriseMsg(symbol, rows)); }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("EPS: {}", e))); }
                        }
                    });
                }
                // ── Round 6 handlers ──
                BrokerCmd::FetchWorldIndices => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let client = reqwest::Client::builder()
                            .user_agent("Mozilla/5.0 (X11; Linux x86_64) TyphooN-Terminal/0.1")
                            .timeout(std::time::Duration::from_secs(20))
                            .build().unwrap_or_default();
                        match research::fetch_world_indices(&client).await {
                            Ok(rows) => { let _ = msg_tx.send(BrokerMsg::WorldIndicesMsg(rows)); }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("WEI: {}", e))); }
                        }
                    });
                }
                BrokerCmd::FetchMarketMovers { fmp_key } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let client = reqwest::Client::builder()
                            .user_agent("TyphooN-Terminal/1.0")
                            .timeout(std::time::Duration::from_secs(25))
                            .build().unwrap_or_default();
                        match research::fetch_fmp_market_movers(&client, &fmp_key).await {
                            Ok(mov) => { let _ = msg_tx.send(BrokerMsg::MarketMoversMsg(mov)); }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("MOV: {}", e))); }
                        }
                    });
                }
                BrokerCmd::FetchSectorPerformance { fmp_key } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let client = reqwest::Client::builder()
                            .user_agent("TyphooN-Terminal/1.0")
                            .timeout(std::time::Duration::from_secs(15))
                            .build().unwrap_or_default();
                        match research::fetch_fmp_sector_performance(&client, &fmp_key).await {
                            Ok(rows) => { let _ = msg_tx.send(BrokerMsg::SectorPerformanceMsg(rows)); }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("INDU: {}", e))); }
                        }
                    });
                }
                BrokerCmd::FetchWaccSnapshot { symbol, fmp_key, risk_free_pct } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let client = reqwest::Client::builder()
                            .user_agent("TyphooN-Terminal/1.0")
                            .timeout(std::time::Duration::from_secs(25))
                            .build().unwrap_or_default();
                        // Fetch profile (beta + market cap)
                        let profile_url = format!(
                            "https://financialmodelingprep.com/api/v3/profile/{}?apikey={}",
                            symbol, fmp_key
                        );
                        let profile_resp = match client.get(&profile_url).send().await {
                            Ok(r) => r,
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("WACC profile: {e}"))); return; }
                        };
                        let profile_arr: Vec<serde_json::Value> = match profile_resp.json().await {
                            Ok(v) => v,
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("WACC profile parse: {e}"))); return; }
                        };
                        let profile = profile_arr.first().cloned().unwrap_or_default();
                        let beta = profile["beta"].as_f64().unwrap_or(1.0);
                        let market_cap = profile["mktCap"].as_f64().unwrap_or(0.0);
                        // Fetch key metrics TTM (effective tax rate fallback)
                        let km_url = format!(
                            "https://financialmodelingprep.com/api/v3/key-metrics-ttm/{}?apikey={}",
                            symbol, fmp_key
                        );
                        let km_arr: Vec<serde_json::Value> = match client.get(&km_url).send().await {
                            Ok(r) => r.json().await.unwrap_or_default(),
                            Err(_) => Vec::new(),
                        };
                        let km = km_arr.first().cloned().unwrap_or_default();
                        // Fetch income statement for interest expense + tax
                        let is_url = format!(
                            "https://financialmodelingprep.com/api/v3/income-statement/{}?period=annual&limit=1&apikey={}",
                            symbol, fmp_key
                        );
                        let is_arr: Vec<serde_json::Value> = match client.get(&is_url).send().await {
                            Ok(r) => r.json().await.unwrap_or_default(),
                            Err(_) => Vec::new(),
                        };
                        let is_row = is_arr.first().cloned().unwrap_or_default();
                        let interest_expense = is_row["interestExpense"].as_f64().unwrap_or(0.0);
                        let income_before_tax = is_row["incomeBeforeTax"].as_f64().unwrap_or(0.0);
                        let income_tax = is_row["incomeTaxExpense"].as_f64().unwrap_or(0.0);
                        let effective_tax_rate_pct = if income_before_tax.abs() > 1e-6 {
                            (income_tax / income_before_tax) * 100.0
                        } else {
                            km["effectiveTaxRateTTM"].as_f64().unwrap_or(0.21) * 100.0
                        };
                        // Fetch balance sheet for total debt
                        let bs_url = format!(
                            "https://financialmodelingprep.com/api/v3/balance-sheet-statement/{}?period=annual&limit=1&apikey={}",
                            symbol, fmp_key
                        );
                        let bs_arr: Vec<serde_json::Value> = match client.get(&bs_url).send().await {
                            Ok(r) => r.json().await.unwrap_or_default(),
                            Err(_) => Vec::new(),
                        };
                        let bs_row = bs_arr.first().cloned().unwrap_or_default();
                        let total_debt = bs_row["totalDebt"].as_f64()
                            .unwrap_or_else(|| {
                                let lt = bs_row["longTermDebt"].as_f64().unwrap_or(0.0);
                                let st = bs_row["shortTermDebt"].as_f64().unwrap_or(0.0);
                                lt + st
                            });
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let snap = research::compute_wacc_snapshot(
                            &symbol, &today, beta, market_cap, risk_free_pct,
                            total_debt, interest_expense, effective_tax_rate_pct,
                        );
                        let _ = msg_tx.send(BrokerMsg::WaccSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 7 handlers ──
                BrokerCmd::FetchCurrencyRates => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let client = reqwest::Client::builder()
                            .user_agent("Mozilla/5.0 (X11; Linux x86_64) TyphooN-Terminal/0.1")
                            .timeout(std::time::Duration::from_secs(15))
                            .build().unwrap_or_default();
                        match research::fetch_currency_rates(&client).await {
                            Ok(rows) => { let _ = msg_tx.send(BrokerMsg::CurrencyRatesMsg(rows)); }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("WCR: {e}"))); }
                        }
                    });
                }
                BrokerCmd::FetchBetaSnapshot { symbol, fmp_key } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let client = reqwest::Client::builder()
                            .user_agent("TyphooN-Terminal/1.0")
                            .timeout(std::time::Duration::from_secs(30))
                            .build().unwrap_or_default();
                        // Fetch 5 years of bars for both the symbol and SPY.
                        let sym_bars = match research::fetch_fmp_historical_price(&client, &symbol, &fmp_key, 1300).await {
                            Ok(rows) => rows,
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("BETA {symbol} bars: {e}"))); return; }
                        };
                        let mkt_bars = match research::fetch_fmp_historical_price(&client, "SPY", &fmp_key, 1300).await {
                            Ok(rows) => rows,
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("BETA SPY bars: {e}"))); return; }
                        };
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let snap = research::compute_beta_snapshot(&symbol, "SPY", &today, &sym_bars, &mkt_bars);
                        let _ = msg_tx.send(BrokerMsg::BetaSnapshotMsg(symbol, snap));
                    });
                }
                cmd @ (
                    BrokerCmd::ComputeDdmSnapshot { .. }
                    | BrokerCmd::ComputeRelativeValuation { .. }
                    | BrokerCmd::FetchFigiIdentifiers { .. }
                    | BrokerCmd::FetchHraSnapshot { .. }
                    | BrokerCmd::ComputeDcfSnapshot { .. }
                    | BrokerCmd::ComputeSvmSnapshot { .. }
                    | BrokerCmd::FetchOptionsChain { .. }
                    | BrokerCmd::ComputeIvolSnapshot { .. }
                    | BrokerCmd::ComputeSeasonalitySnapshot { .. }
                    | BrokerCmd::ComputeCorrelationMatrix { .. }
                    | BrokerCmd::ComputeTotalReturnSnapshot { .. }
                    | BrokerCmd::ComputeTechnicalsSnapshot { .. }
                    | BrokerCmd::ComputeVolSkewSnapshot { .. }
                    | BrokerCmd::ComputeLeverageSnapshot { .. }
                    | BrokerCmd::ComputeAccrualsSnapshot { .. }
                    | BrokerCmd::ComputeRealizedVolSnapshot { .. }
                    | BrokerCmd::ComputeFcfYieldSnapshot { .. }
                    | BrokerCmd::ComputeShortInterestSnapshot { .. }
                    | BrokerCmd::ComputeAltmanZSnapshot { .. }
                    | BrokerCmd::ComputePiotroskiSnapshot { .. }
                    | BrokerCmd::ComputeOhlcVolSnapshot { .. }
                    | BrokerCmd::ComputeEpsBeatSnapshot { .. }
                    | BrokerCmd::ComputePriceTargetDispersionSnapshot { .. }
                    | BrokerCmd::ComputeInsiderActivitySnapshot { .. }
                    | BrokerCmd::ComputeDivgSnapshot { .. }
                    | BrokerCmd::ComputeEarmSnapshot { .. }
                    | BrokerCmd::ComputeSectorRotationSnapshot { .. }
                    | BrokerCmd::ComputeUpdmSnapshot { .. }
                    | BrokerCmd::ComputeMomentumSnapshot { .. }
                    | BrokerCmd::ComputeLiquiditySnapshot { .. }
                    | BrokerCmd::ComputeBreakoutSnapshot { .. }
                    | BrokerCmd::ComputeCashCycleSnapshot { .. }
                    | BrokerCmd::ComputeCreditSnapshot { .. }
                    | BrokerCmd::ComputeGrowmSnapshot { .. }
                    | BrokerCmd::ComputeFlowSnapshot { .. }
                    | BrokerCmd::ComputeRegimeSnapshot { .. }
                    | BrokerCmd::ComputeRelvolSnapshot { .. }
                    | BrokerCmd::ComputeMarginsSnapshot { .. }
                    | BrokerCmd::ComputeValSnapshot { .. }
                    | BrokerCmd::ComputeQualSnapshot { .. }
                    | BrokerCmd::ComputeRiskSnapshot { .. }
                    | BrokerCmd::ComputeInsstrkSnapshot { .. }
                    | BrokerCmd::ComputeCovgSnapshot { .. }
                    | BrokerCmd::ComputeVrkSnapshot { .. }
                    | BrokerCmd::ComputeQrkSnapshot { .. }
                    | BrokerCmd::ComputeRrkSnapshot { .. }
                    | BrokerCmd::ComputeRelepsgrSnapshot { .. }
                    | BrokerCmd::ComputePeadSnapshot { .. }
                    | BrokerCmd::ComputeSizefSnapshot { .. }
                    | BrokerCmd::ComputeMomfSnapshot { .. }
                    | BrokerCmd::ComputePeadrankSnapshot { .. }
                    | BrokerCmd::ComputeFqmSnapshot { .. }
                    | BrokerCmd::ComputeRevrankSnapshot { .. }
                    | BrokerCmd::ComputeLevrankSnapshot { .. }
                    | BrokerCmd::ComputeOperankSnapshot { .. }
                    | BrokerCmd::ComputeFqmrankSnapshot { .. }
                    | BrokerCmd::ComputeLiqrankSnapshot { .. }
                    | BrokerCmd::ComputeSurpstkSnapshot { .. }
                    | BrokerCmd::ComputeDvdrankSnapshot { .. }
                    | BrokerCmd::ComputeEarmrankSnapshot { .. }
                    | BrokerCmd::ComputeUpdgrankSnapshot { .. }
                    | BrokerCmd::ComputeGySnapshot { .. }
                    | BrokerCmd::ComputeDesSnapshot { .. }
                    | BrokerCmd::ComputeDvdyieldrankSnapshot { .. }
                    | BrokerCmd::ComputeShrankSnapshot { .. }
                    | BrokerCmd::ComputeShortrankDeltaSnapshot { .. }
                    | BrokerCmd::ComputeInsiderconcSnapshot { .. }
                    | BrokerCmd::ComputeAtrannSnapshot { .. }
                    | BrokerCmd::ComputeDdhistSnapshot { .. }
                    | BrokerCmd::ComputePriceperfSnapshot { .. }
                    | BrokerCmd::ComputeMomrankMultiSnapshot { .. }
                    | BrokerCmd::ComputeBetarankSnapshot { .. }
                    | BrokerCmd::ComputePegrankSnapshot { .. }
                    | BrokerCmd::ComputeFhighlowSnapshot { .. }
                    | BrokerCmd::ComputeRvconeSnapshot { .. }
                    | BrokerCmd::ComputeCalpbSnapshot { .. }
                    | BrokerCmd::ComputeCorrstkSnapshot { .. }
                    | BrokerCmd::ComputeTlrankSnapshot { .. }
                    | BrokerCmd::ComputeCorrrankSnapshot { .. }
                    | BrokerCmd::ComputeOperankDeltaSnapshot { .. }
                    | BrokerCmd::ComputeDivaccSnapshot { .. }
                    | BrokerCmd::ComputeEpsaccSnapshot { .. }
                    | BrokerCmd::ComputeVrpSnapshot { .. }
                    | BrokerCmd::ComputeRetskewSnapshot { .. }
                    | BrokerCmd::ComputeRetkurtSnapshot { .. }
                    | BrokerCmd::ComputeTailrSnapshot { .. }
                    | BrokerCmd::ComputeRunlenSnapshot { .. }
                    | BrokerCmd::ComputeDayrangeSnapshot { .. }
                    | BrokerCmd::ComputeAutocorSnapshot { .. }
                    | BrokerCmd::ComputeHurstSnapshot { .. }
                    | BrokerCmd::ComputeHitrateSnapshot { .. }
                    | BrokerCmd::ComputeGlasymSnapshot { .. }
                    | BrokerCmd::ComputeVolratioSnapshot { .. }
                    | BrokerCmd::ComputeDrawupSnapshot { .. }
                    | BrokerCmd::ComputeGapstatsSnapshot { .. }
                    | BrokerCmd::ComputeVolclusterSnapshot { .. }
                    | BrokerCmd::ComputeCloseplcSnapshot { .. }
                    | BrokerCmd::ComputeMrhlSnapshot { .. }
                    | BrokerCmd::ComputeDownvolSnapshot { .. }
                    | BrokerCmd::ComputeSharprSnapshot { .. }
                    | BrokerCmd::ComputeEffratioSnapshot { .. }
                    | BrokerCmd::ComputeWickbiasSnapshot { .. }
                    | BrokerCmd::ComputeVolofvolSnapshot { .. }
                    | BrokerCmd::ComputeCalmarSnapshot { .. }
                    | BrokerCmd::ComputeUlcerSnapshot { .. }
                    | BrokerCmd::ComputeVarratioSnapshot { .. }
                    | BrokerCmd::ComputeAmihudSnapshot { .. }
                    | BrokerCmd::ComputeJbnormSnapshot { .. }
                    | BrokerCmd::ComputeOmegaSnapshot { .. }
                    | BrokerCmd::ComputeDfaSnapshot { .. }
                    | BrokerCmd::ComputeBurkeSnapshot { .. }
                    | BrokerCmd::ComputeMonthseasSnapshot { .. }
                    | BrokerCmd::ComputeRollsprdSnapshot { .. }
                    | BrokerCmd::ComputeParkinsonSnapshot { .. }
                    | BrokerCmd::ComputeGkvolSnapshot { .. }
                    | BrokerCmd::ComputeRsvolSnapshot { .. }
                    | BrokerCmd::ComputeCvarSnapshot { .. }
                    | BrokerCmd::ComputeDoweffectSnapshot { .. }
                    | BrokerCmd::ComputeSterlingSnapshot { .. }
                    | BrokerCmd::ComputeKellyfSnapshot { .. }
                    | BrokerCmd::ComputeLjungbSnapshot { .. }
                    | BrokerCmd::ComputeRunstestSnapshot { .. }
                    | BrokerCmd::ComputeZeroretSnapshot { .. }
                    | BrokerCmd::ComputePsrSnapshot { .. }
                    | BrokerCmd::ComputeAdfSnapshot { .. }
                    | BrokerCmd::ComputeMnkendallSnapshot { .. }
                    | BrokerCmd::ComputeBipowerSnapshot { .. }
                    | BrokerCmd::ComputeDddurSnapshot { .. }
                    | BrokerCmd::ComputeHilltailSnapshot { .. }
                    | BrokerCmd::ComputeArchlmSnapshot { .. }
                    | BrokerCmd::ComputePainratioSnapshot { .. }
                    | BrokerCmd::ComputeCusumSnapshot { .. }
                    | BrokerCmd::ComputeCfvarSnapshot { .. }
                    | BrokerCmd::ComputeEntropySnapshot { .. }
                    | BrokerCmd::ComputeRachevSnapshot { .. }
                    | BrokerCmd::ComputeGprSnapshot { .. }
                    | BrokerCmd::ComputePacfSnapshot { .. }
                    | BrokerCmd::ComputeApenSnapshot { .. }
                    | BrokerCmd::ComputeUprSnapshot { .. }
                    | BrokerCmd::ComputeLevereffSnapshot { .. }
                    | BrokerCmd::ComputeDrawdarSnapshot { .. }
                    | BrokerCmd::ComputeVarhalfSnapshot { .. }
                    | BrokerCmd::ComputeGiniSnapshot { .. }
                    | BrokerCmd::ComputeSampenSnapshot { .. }
                    | BrokerCmd::ComputePermenSnapshot { .. }
                    | BrokerCmd::ComputeRecfactSnapshot { .. }
                    | BrokerCmd::ComputeKpssSnapshot { .. }
                    | BrokerCmd::ComputeSpecentSnapshot { .. }
                    | BrokerCmd::ComputeRobvolSnapshot { .. }
                    | BrokerCmd::ComputeRenyientSnapshot { .. }
                    | BrokerCmd::ComputeRetquantSnapshot { .. }
                    | BrokerCmd::ComputeMsentSnapshot { .. }
                    | BrokerCmd::ComputeEwmavolSnapshot { .. }
                    | BrokerCmd::ComputeKsnormSnapshot { .. }
                    | BrokerCmd::ComputeAdtestSnapshot { .. }
                    | BrokerCmd::ComputeLmomSnapshot { .. }
                    | BrokerCmd::ComputeKylelamSnapshot { .. }
                    | BrokerCmd::ComputePeakoverSnapshot { .. }
                    | BrokerCmd::ComputeHiguchiSnapshot { .. }
                    | BrokerCmd::ComputePickandsSnapshot { .. }
                    | BrokerCmd::ComputeKappa3Snapshot { .. }
                    | BrokerCmd::ComputeLyapunovSnapshot { .. }
                    | BrokerCmd::ComputeRankacSnapshot { .. }
                    | BrokerCmd::ComputeBnsjumpSnapshot { .. }
                    | BrokerCmd::ComputePprootSnapshot { .. }
                    | BrokerCmd::ComputeMfdfaSnapshot { .. }
                    | BrokerCmd::ComputeHillksSnapshot { .. }
                    | BrokerCmd::ComputeTsiSnapshot { .. }
                    | BrokerCmd::ComputeGarch11Snapshot { .. }
                    | BrokerCmd::ComputeSadfSnapshot { .. }
                    | BrokerCmd::ComputeCordimSnapshot { .. }
                    | BrokerCmd::ComputeSkspecSnapshot { .. }
                    | BrokerCmd::ComputeAutomiSnapshot { .. }
                    | BrokerCmd::ComputeDurbinWatsonSnapshot { .. }
                    | BrokerCmd::ComputeBdsTestSnapshot { .. }
                    | BrokerCmd::ComputeBreuschPaganSnapshot { .. }
                    | BrokerCmd::ComputeTurnPtsSnapshot { .. }
                    | BrokerCmd::ComputePeriodogramSnapshot { .. }
                    | BrokerCmd::ComputeMcLeodLiSnapshot { .. }
                    | BrokerCmd::ComputeOuFitSnapshot { .. }
                    | BrokerCmd::ComputeGphSnapshot { .. }
                    | BrokerCmd::ComputeBurgSpecSnapshot { .. }
                    | BrokerCmd::ComputeKendallTauSnapshot { .. }
                    | BrokerCmd::ComputeSqueezeSnapshot { .. }
                    | BrokerCmd::ComputeSqueezeRankSnapshot { .. }
                    | BrokerCmd::RefreshSqueezeWatchlist { .. }
                    | BrokerCmd::ComputeBbsqueezeSnapshot { .. }
                    | BrokerCmd::ComputeDonchianSnapshot { .. }
                    | BrokerCmd::ComputeKamaSnapshot { .. }
                    | BrokerCmd::ComputeIchimokuSnapshot { .. }
                    | BrokerCmd::ComputeSupertrendSnapshot { .. }
                    | BrokerCmd::ComputeKeltnerSnapshot { .. }
                    | BrokerCmd::ComputeFisherSnapshot { .. }
                    | BrokerCmd::ComputeAroonSnapshot { .. }
                    | BrokerCmd::ComputeAdxSnapshot { .. }
                    | BrokerCmd::ComputeCciSnapshot { .. }
                    | BrokerCmd::ComputeCmfSnapshot { .. }
                    | BrokerCmd::ComputeMfiSnapshot { .. }
                    | BrokerCmd::ComputePsarSnapshot { .. }
                    | BrokerCmd::ComputeVortexSnapshot { .. }
                    | BrokerCmd::ComputeChopSnapshot { .. }
                    | BrokerCmd::ComputeObvSnapshot { .. }
                    | BrokerCmd::ComputeTrixSnapshot { .. }
                    | BrokerCmd::ComputeHmaSnapshot { .. }
                    | BrokerCmd::ComputePpoSnapshot { .. }
                    | BrokerCmd::ComputeDpoSnapshot { .. }
                    | BrokerCmd::ComputeKstSnapshot { .. }
                    | BrokerCmd::ComputeUltoscSnapshot { .. }
                    | BrokerCmd::ComputeWillrSnapshot { .. }
                    | BrokerCmd::ComputeMassSnapshot { .. }
                    | BrokerCmd::ComputeChaikoscSnapshot { .. }
                    | BrokerCmd::ComputeKlingerSnapshot { .. }
                    | BrokerCmd::ComputeStochRsiSnapshot { .. }
                    | BrokerCmd::ComputeAwesomeSnapshot { .. }
                    | BrokerCmd::ComputeEfiSnapshot { .. }
                    | BrokerCmd::ComputeEmvSnapshot { .. }
                    | BrokerCmd::ComputeNviSnapshot { .. }
                    | BrokerCmd::ComputePviSnapshot { .. }
                    | BrokerCmd::ComputeCoppockSnapshot { .. }
                    | BrokerCmd::ComputeCmoSnapshot { .. }
                    | BrokerCmd::ComputeQstickSnapshot { .. }
                    | BrokerCmd::ComputeDisparitySnapshot { .. }
                    | BrokerCmd::ComputeBopSnapshot { .. }
                    | BrokerCmd::ComputeSchaffSnapshot { .. }
                    | BrokerCmd::ComputeStochSnapshot { .. }
                    | BrokerCmd::ComputeMacdSnapshot { .. }
                    | BrokerCmd::ComputeVwapSnapshot { .. }
                    | BrokerCmd::ComputeMcgdSnapshot { .. }
                    | BrokerCmd::ComputeRwiSnapshot { .. }
                    | BrokerCmd::ComputeDemaSnapshot { .. }
                    | BrokerCmd::ComputeTemaSnapshot { .. }
                    | BrokerCmd::ComputeLinregSnapshot { .. }
                    | BrokerCmd::ComputePivotsSnapshot { .. }
                    | BrokerCmd::ComputeHeikinSnapshot { .. }
                    | BrokerCmd::ComputeAlmaSnapshot { .. }
                    | BrokerCmd::ComputeZlemaSnapshot { .. }
                    | BrokerCmd::ComputeElderRaySnapshot { .. }
                    | BrokerCmd::ComputeTsfSnapshot { .. }
                    | BrokerCmd::ComputeRviSnapshot { .. }
                    | BrokerCmd::ComputeTrimaSnapshot { .. }
                    | BrokerCmd::ComputeT3Snapshot { .. }
                    | BrokerCmd::ComputeVidyaSnapshot { .. }
                    | BrokerCmd::ComputeSmiSnapshot { .. }
                    | BrokerCmd::ComputePvtSnapshot { .. }
                    | BrokerCmd::ComputeAcSnapshot { .. }
                    | BrokerCmd::ComputeChvolSnapshot { .. }
                    | BrokerCmd::ComputeBbwidthSnapshot { .. }
                    | BrokerCmd::ComputeElderImpSnapshot { .. }
                    | BrokerCmd::ComputeRmiSnapshot { .. }
                    | BrokerCmd::ComputeSymbolExpirations { .. }
                    | BrokerCmd::ComputeSmmaSnapshot { .. }
                    | BrokerCmd::ComputeAlligatorSnapshot { .. }
                    | BrokerCmd::ComputeCrsiSnapshot { .. }
                    | BrokerCmd::ComputeSebSnapshot { .. }
                    | BrokerCmd::ComputeImiSnapshot { .. }
                    | BrokerCmd::ComputeGmmaSnapshot { .. }
                    | BrokerCmd::ComputeMaenvSnapshot { .. }
                    | BrokerCmd::ComputeAdlSnapshot { .. }
                    | BrokerCmd::ComputeVhfSnapshot { .. }
                    | BrokerCmd::ComputeVrocSnapshot { .. }
                    | BrokerCmd::ComputeKdjSnapshot { .. }
                    | BrokerCmd::ComputeQqeSnapshot { .. }
                    | BrokerCmd::ComputePmoSnapshot { .. }
                    | BrokerCmd::ComputeCfoSnapshot { .. }
                    | BrokerCmd::ComputeTmfSnapshot { .. }
                    | BrokerCmd::ComputeFractalsSnapshot { .. }
                    | BrokerCmd::ComputeIftRsiSnapshot { .. }
                    | BrokerCmd::ComputeMamaSnapshot { .. }
                    | BrokerCmd::ComputeCogSnapshot { .. }
                    | BrokerCmd::ComputeDidiSnapshot { .. }
                    | BrokerCmd::ComputeDemarkerSnapshot { .. }
                    | BrokerCmd::ComputeGatorSnapshot { .. }
                    | BrokerCmd::ComputeBwMfiSnapshot { .. }
                    | BrokerCmd::ComputeVwmaSnapshot { .. }
                    | BrokerCmd::ComputeStddevSnapshot { .. }
                    | BrokerCmd::ComputeWmaSnapshot { .. }
                    | BrokerCmd::ComputeRainbowSnapshot { .. }
                    | BrokerCmd::ComputeMesaSineSnapshot { .. }
                    | BrokerCmd::ComputeFramaSnapshot { .. }
                    | BrokerCmd::ComputeIbsSnapshot { .. }
                    | BrokerCmd::ComputeLaguerreRsiSnapshot { .. }
                    | BrokerCmd::ComputeZigzagSnapshot { .. }
                    | BrokerCmd::ComputePgoSnapshot { .. }
                    | BrokerCmd::ComputeHtTrendlineSnapshot { .. }
                    | BrokerCmd::ComputeMidpointSnapshot { .. }
                    | BrokerCmd::ComputeMassIndexSnapshot { .. }
                    | BrokerCmd::ComputeNatrSnapshot { .. }
                    | BrokerCmd::ComputeTtmSqueezeSnapshot { .. }
                    | BrokerCmd::ComputeForceIndexSnapshot { .. }
                    | BrokerCmd::ComputeTrangeSnapshot { .. }
                    | BrokerCmd::ComputeLinearregSlopeSnapshot { .. }
                    | BrokerCmd::ComputeHtDcperiodSnapshot { .. }
                    | BrokerCmd::ComputeHtTrendmodeSnapshot { .. }
                    | BrokerCmd::ComputeAccbandsSnapshot { .. }
                    | BrokerCmd::ComputeStochfSnapshot { .. }
                    | BrokerCmd::ComputeLinearregSnapshot { .. }
                    | BrokerCmd::ComputeLinearregAngleSnapshot { .. }
                    | BrokerCmd::ComputeHtDcphaseSnapshot { .. }
                    | BrokerCmd::ComputeHtSineSnapshot { .. }
                    | BrokerCmd::ComputeHtPhasorSnapshot { .. }
                    | BrokerCmd::ComputeMidpriceSnapshot { .. }
                    | BrokerCmd::ComputeApoSnapshot { .. }
                    | BrokerCmd::ComputeMomSnapshot { .. }
                    | BrokerCmd::ComputeSarextSnapshot { .. }
                    | BrokerCmd::ComputeAdxrSnapshot { .. }
                    | BrokerCmd::ComputeAvgpriceSnapshot { .. }
                    | BrokerCmd::ComputeMedpriceSnapshot { .. }
                    | BrokerCmd::ComputeTypPriceSnapshot { .. }
                    | BrokerCmd::ComputeWclPriceSnapshot { .. }
                    | BrokerCmd::ComputeVarianceSnapshot { .. }
                    | BrokerCmd::ComputePlusDiSnapshot { .. }
                    | BrokerCmd::ComputeMinusDiSnapshot { .. }
                    | BrokerCmd::ComputePlusDmSnapshot { .. }
                    | BrokerCmd::ComputeMinusDmSnapshot { .. }
                    | BrokerCmd::ComputeDxSnapshot { .. }
                    | BrokerCmd::ComputeRocSnapshot { .. }
                    | BrokerCmd::ComputeRocpSnapshot { .. }
                    | BrokerCmd::ComputeRocrSnapshot { .. }
                    | BrokerCmd::ComputeRocr100Snapshot { .. }
                    | BrokerCmd::ComputeCorrelSnapshot { .. }
                    | BrokerCmd::ComputeMinSnapshot { .. }
                    | BrokerCmd::ComputeMaxSnapshot { .. }
                    | BrokerCmd::ComputeMinMaxSnapshot { .. }
                    | BrokerCmd::ComputeMinIndexSnapshot { .. }
                    | BrokerCmd::ComputeMaxIndexSnapshot { .. }
                    | BrokerCmd::ComputeBbandsSnapshot { .. }
                    | BrokerCmd::ComputeAdSnapshot { .. }
                    | BrokerCmd::ComputeAdoscSnapshot { .. }
                    | BrokerCmd::ComputeSumSnapshot { .. }
                    | BrokerCmd::ComputeLinearRegInterceptSnapshot { .. }
                    | BrokerCmd::ComputeAroonoscSnapshot { .. }
                    | BrokerCmd::ComputeMinMaxIndexSnapshot { .. }
                    | BrokerCmd::ComputeMacdextSnapshot { .. }
                    | BrokerCmd::ComputeMacdfixSnapshot { .. }
                    | BrokerCmd::ComputeMavpSnapshot { .. }
                    | BrokerCmd::ComputeCdlDojiSnapshot { .. }
                    | BrokerCmd::ComputeCdlHammerSnapshot { .. }
                    | BrokerCmd::ComputeCdlShootingStarSnapshot { .. }
                    | BrokerCmd::ComputeCdlEngulfingSnapshot { .. }
                    | BrokerCmd::ComputeCdlHaramiSnapshot { .. }
                    | BrokerCmd::ComputeCdlMorningStarSnapshot { .. }
                    | BrokerCmd::ComputeCdlEveningStarSnapshot { .. }
                    | BrokerCmd::ComputeCdlThreeBlackCrowsSnapshot { .. }
                    | BrokerCmd::ComputeCdlThreeWhiteSoldiersSnapshot { .. }
                    | BrokerCmd::ComputeCdlDarkCloudCoverSnapshot { .. }
                    | BrokerCmd::ComputeCdlPiercingSnapshot { .. }
                    | BrokerCmd::ComputeCdlDragonflyDojiSnapshot { .. }
                    | BrokerCmd::ComputeCdlGravestoneDojiSnapshot { .. }
                    | BrokerCmd::ComputeCdlHangingManSnapshot { .. }
                    | BrokerCmd::ComputeCdlInvertedHammerSnapshot { .. }
                    | BrokerCmd::ComputeCdlHaramiCrossSnapshot { .. }
                    | BrokerCmd::ComputeCdlLongLeggedDojiSnapshot { .. }
                    | BrokerCmd::ComputeCdlMarubozuSnapshot { .. }
                    | BrokerCmd::ComputeCdlSpinningTopSnapshot { .. }
                    | BrokerCmd::ComputeCdlTristarSnapshot { .. }
                    | BrokerCmd::ComputeCdlDojiStarSnapshot { .. }
                    | BrokerCmd::ComputeCdlMorningDojiStarSnapshot { .. }
                    | BrokerCmd::ComputeCdlEveningDojiStarSnapshot { .. }
                    | BrokerCmd::ComputeCdlAbandonedBabySnapshot { .. }
                    | BrokerCmd::ComputeCdlThreeInsideSnapshot { .. }
                    | BrokerCmd::ComputeCdlBeltHoldSnapshot { .. }
                    | BrokerCmd::ComputeCdlClosingMarubozuSnapshot { .. }
                    | BrokerCmd::ComputeCdlHighWaveSnapshot { .. }
                    | BrokerCmd::ComputeCdlLongLineSnapshot { .. }
                    | BrokerCmd::ComputeCdlShortLineSnapshot { .. }
                    | BrokerCmd::ComputeCdlCounterattackSnapshot { .. }
                    | BrokerCmd::ComputeCdlHomingPigeonSnapshot { .. }
                    | BrokerCmd::ComputeCdlInNeckSnapshot { .. }
                    | BrokerCmd::ComputeCdlOnNeckSnapshot { .. }
                    | BrokerCmd::ComputeCdlThrustingSnapshot { .. }
                    | BrokerCmd::ComputeCdlTwoCrowsSnapshot { .. }
                    | BrokerCmd::ComputeCdlThreeLineStrikeSnapshot { .. }
                    | BrokerCmd::ComputeCdlThreeOutsideSnapshot { .. }
                    | BrokerCmd::ComputeCdlMatchingLowSnapshot { .. }
                    | BrokerCmd::ComputeCdlSeparatingLinesSnapshot { .. }
                    | BrokerCmd::ComputeCdlStickSandwichSnapshot { .. }
                    | BrokerCmd::ComputeCdlRickshawManSnapshot { .. }
                    | BrokerCmd::ComputeCdlTakuriSnapshot { .. }
                    | BrokerCmd::ComputeCdlThreeStarsInSouthSnapshot { .. }
                    | BrokerCmd::ComputeCdlIdenticalThreeCrowsSnapshot { .. }
                    | BrokerCmd::ComputeCdlKickingSnapshot { .. }
                    | BrokerCmd::ComputeCdlKickingByLengthSnapshot { .. }
                    | BrokerCmd::ComputeCdlLadderBottomSnapshot { .. }
                    | BrokerCmd::ComputeCdlUniqueThreeRiverSnapshot { .. }
                    | BrokerCmd::ComputeCdlAdvanceBlockSnapshot { .. }
                    | BrokerCmd::ComputeCdlBreakawaySnapshot { .. }
                    | BrokerCmd::ComputeCdlGapSideSideWhiteSnapshot { .. }
                    | BrokerCmd::ComputeCdlUpsideGapTwoCrowsSnapshot { .. }
                    | BrokerCmd::ComputeCdlXSideGapThreeMethodsSnapshot { .. }
                    | BrokerCmd::ComputeCdlConcealBabySwallowSnapshot { .. }
                    | BrokerCmd::ComputeCdlHikkakeSnapshot { .. }
                    | BrokerCmd::ComputeCdlHikkakeModSnapshot { .. }
                    | BrokerCmd::ComputeCdlMatHoldSnapshot { .. }
                    | BrokerCmd::ComputeCdlRiseFallThreeMethodsSnapshot { .. }
                    | BrokerCmd::ComputeCdlStalledPatternSnapshot { .. }
                    | BrokerCmd::ComputeCdlTasukiGapSnapshot { .. }
                    | BrokerCmd::ComputeModSharpeSnapshot { .. }
                    | BrokerCmd::ComputeHsiehTestSnapshot { .. }
                    | BrokerCmd::ComputeChowBreakSnapshot { .. }
                    | BrokerCmd::ComputeDriftBurstSnapshot { .. }
                    | BrokerCmd::ComputeHlvClustSnapshot { .. }
                    | BrokerCmd::ComputeYangZhangSnapshot { .. }
                    | BrokerCmd::ComputeKuiperSnapshot { .. }
                    | BrokerCmd::ComputeDagostinoSnapshot { .. }
                    | BrokerCmd::ComputeBaiPerronSnapshot { .. }
                    | BrokerCmd::ComputeKupiecPofSnapshot { .. }
                ) => {
                    research_compute::handle_research_compute_command(
                        cmd,
                        broker_msg_tx_clone.clone(),
                        shared_cache_broker.clone(),
                    );
                }
                cmd @ (
                    BrokerCmd::IngestResearchArticles { .. }
                    | BrokerCmd::FetchNewsMulti { .. }
                    | BrokerCmd::LoadCachedNews { .. }
                    | BrokerCmd::HydrateNewsArticle { .. }
                    | BrokerCmd::SearchNews { .. }
                    | BrokerCmd::NewsScrapeSymbols { .. }
                ) => {
                    news::handle_news_command(
                        cmd,
                        broker_msg_tx_clone.clone(),
                        shared_cache_broker.clone(),
                    );
                }
                BrokerCmd::NewsScrapeAll {
                    use_mt5, use_alpaca, use_kraken,
                    marketaux_key, alpha_vantage_key, fmp_key,
                    finnhub_key, cryptopanic_key,
                } => {
                    // Gather broker-side tickers before spawning thread.
                    let mut extra_tickers: Vec<String> = Vec::new();
                    if use_alpaca {
                        if let Some(ref b) = broker {
                            if let Ok(assets) = b.get_all_assets().await {
                                extra_tickers.extend(assets.iter()
                                    .filter(|a| a.asset_class == "us_equity" && a.tradable)
                                    .map(|a| a.symbol.clone()));
                            }
                        }
                    }
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    let _ = std::thread::Builder::new()
                        .name("typhoon-news-scrape-all".into())
                        .spawn(move || {
                            use typhoon_engine::core::news;
                            let rt = tokio::runtime::Builder::new_current_thread()
                                .enable_all().build().unwrap_or_else(|e| { eprintln!("FATAL: tokio runtime init failed: {e}"); std::process::exit(1); });
                            rt.block_on(async {
                            let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) else {
                                let _ = msg_tx.send(BrokerMsg::Error("NewsScrapeAll: cache not ready".into()));
                                return;
                            };
                            // Gather enabled source-universe tickers from cache/brokers.
                            let mut all_tickers: std::collections::HashSet<String> = std::collections::HashSet::new();
                            all_tickers.extend(extra_tickers);
                            if use_mt5 {
                                if let Ok(conn) = cache.connection() {
                                    if let Ok(mt5_tickers) = fundamentals::extract_stock_tickers_from_cache(&conn) {
                                        let _ = msg_tx.send(BrokerMsg::FundamentalsProgress(
                                            format!("News scrape: {} MT5 tickers", mt5_tickers.len())));
                                        all_tickers.extend(mt5_tickers);
                                    }
                                }
                            }
                            if use_kraken {
                                if let Ok(conn) = cache.connection() {
                                    match extract_news_symbols_from_market_data_cache(
                                        &conn,
                                        &["kraken", "kraken-equities", "kraken-futures"],
                                    ) {
                                        Ok(kraken_tickers) => {
                                            let _ = msg_tx.send(BrokerMsg::FundamentalsProgress(
                                                format!("News scrape: {} Kraken market-data symbols", kraken_tickers.len())));
                                            all_tickers.extend(kraken_tickers);
                                        }
                                        Err(e) => {
                                            let _ = msg_tx.send(BrokerMsg::FundamentalsProgress(
                                                format!("News scrape: Kraken symbols failed: {e}")));
                                        }
                                    }
                                }
                            }
                            let mut tickers: Vec<String> = all_tickers.into_iter().collect();
                            tickers.sort();
                            let _ = msg_tx.send(BrokerMsg::FundamentalsProgress(
                                format!("News scrape: starting for {} symbols", tickers.len())));
                            let fresh_tickers = cache
                                .connection()
                                .ok()
                                .and_then(|conn| {
                                    news::fresh_news_symbols(&conn, &tickers, 30 * 60, 1).ok()
                                })
                                .unwrap_or_default();
                            let client = match reqwest::Client::builder()
                                .user_agent("Mozilla/5.0 (X11; Linux x86_64) TyphooN-Terminal/0.1")
                                .timeout(std::time::Duration::from_secs(25))
                                .build() {
                                Ok(c) => c,
                                Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("News client: {e}"))); return; }
                            };
                            let news_keys = news::NewsApiKeys {
                                marketaux: marketaux_key,
                                alpha_vantage: alpha_vantage_key,
                                fmp: fmp_key,
                                finnhub: finnhub_key,
                                cryptopanic: cryptopanic_key,
                            };
                            let mut ok = 0usize;
                            let mut fail = 0usize;
                            let mut processed_keys = std::collections::HashSet::new();
                            for (i, ticker) in tickers.iter().enumerate() {
                                // Deduplicate crypto fetches by base asset (e.g. ETH/USD and ETH/EUR both fetch ETH)
                                let fetch_key = if news::is_crypto_symbol(ticker) {
                                    news::crypto_base_for_symbol(ticker).unwrap_or_else(|| ticker.clone())
                                } else {
                                    ticker.clone()
                                };

                                if processed_keys.contains(&fetch_key) {
                                    if let Ok(conn) = cache.connection() {
                                        let _ = news::mark_news_scraped(&conn, ticker);
                                    }
                                    ok += 1;
                                    let _ = msg_tx.send(BrokerMsg::FundamentalsProgress(format!(
                                        "News {}: base asset {} already fetched — skipped network ({}/{})",
                                        ticker, fetch_key, i + 1, tickers.len()
                                    )));
                                    continue;
                                }
                                processed_keys.insert(fetch_key);

                                if fresh_tickers.contains(ticker) {
                                    ok += 1;
                                    let _ = msg_tx.send(BrokerMsg::FundamentalsProgress(
                                        format!("News {}: cached/fresh — skipped network ({}/{})", ticker, i + 1, tickers.len())));
                                    continue;
                                }

                                let log_tx = msg_tx.clone();
                                let cb = move |s: &str| {
                                    let _ = log_tx.send(BrokerMsg::FundamentalsProgress(s.to_string()));
                                };
                                match news::fetch_all_sources_for_symbol(
                                    &client, ticker,
                                    &news_keys,
                                    cb,
                                ).await {
                                    Ok(articles) => {
                                        if let Ok(conn) = cache.connection() {
                                            match news::upsert_news_batch(&conn, &articles) {
                                                Ok(n) => {
                                                    let cached = news::mark_news_scraped(&conn, ticker)
                                                        .unwrap_or(n);
                                                    ok += 1;
                                                    let _ = msg_tx.send(BrokerMsg::FundamentalsProgress(
                                                        format!("News {}: {} cached ({}/{})", ticker, cached, i + 1, tickers.len())));
                                                }
                                                Err(e) => {
                                                    fail += 1;
                                                    let _ = msg_tx.send(BrokerMsg::FundamentalsProgress(
                                                        format!("News {} upsert failed: {e}", ticker)));
                                                }
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        fail += 1;
                                        let _ = msg_tx.send(BrokerMsg::FundamentalsProgress(
                                            format!("News {} failed: {e}", ticker)));
                                    }
                                }
                                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                            }
                            let _ = msg_tx.send(BrokerMsg::FundamentalsProgress(
                                format!("News scrape complete: {} OK, {} failed of {}", ok, fail, tickers.len())));
                        });
                    });
                }
                BrokerCmd::KrakenConnect {
                    api_key,
                    api_secret,
                    ws_api_key,
                    ws_api_secret,
                } => {
                    use typhoon_engine::broker::kraken::KrakenBroker;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let rest_ready =
                        !api_key.trim().is_empty() && !api_secret.trim().is_empty();
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
                        continue;
                    }
                    let rest_api_key = api_key.clone();
                    let rest_api_secret = api_secret.clone();
                    let kb = KrakenBroker::new(api_key, api_secret);
                    match kb.get_balance().await {
                        Ok(balances) => {
                            let mut bal_vec: Vec<(String, f64)> = balances.into_iter()
                                .filter(|(_, v)| *v > 0.0)
                                .collect();
                            bal_vec.sort_by(|a, b| a.0.cmp(&b.0));
                            let summary: String = bal_vec.iter()
                                .map(|(a, v)| format!("{}: {:.8}", a, v))
                                .collect::<Vec<_>>().join(", ");
                            let ws_suffix = ws_status
                                .as_ref()
                                .map(|status| format!(" · {}", status))
                                .unwrap_or_else(|| " · WS auth not configured".to_string());
                            let _ = msg_tx.send(BrokerMsg::Connected(format!(
                                "Kraken connected — {} assets ({}){}",
                                bal_vec.len(), summary, ws_suffix
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
                            kraken_ws_broker = Some(if ws_override_ready {
                                KrakenBroker::new(ws_api_key, ws_api_secret)
                            } else {
                                KrakenBroker::new(rest_api_key, rest_api_secret)
                            });
                            kraken_broker = Some(kb);
                        }
                        Err(e) => {
                            let _ = msg_tx.send(BrokerMsg::Error(format!("Kraken auth failed: {}", e)));
                        }
                    }
                }
                BrokerCmd::KrakenGetBalance => {
                    if let Some(ref kb) = kraken_broker {
                        let msg_tx = broker_msg_tx_clone.clone();
                        match kb.get_balance().await {
                            Ok(balances) => {
                                let bal_vec: Vec<(String, f64)> = balances.into_iter()
                                    .filter(|(_, v)| *v > 0.0)
                                    .collect();
                                let _ = msg_tx.send(BrokerMsg::KrakenBalances(bal_vec));
                            }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("Kraken balance: {}", e))); }
                        }
                    } else {
                        let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult("Kraken: connect first".into()));
                    }
                }
                BrokerCmd::KrakenGetPositions => {
                    if let Some(ref kb) = kraken_broker {
                        match kb.get_all_position_summaries().await {
                            Ok(pos) => {
                                let _ = broker_msg_tx_clone.send(BrokerMsg::KrakenPositions(pos));
                            }
                            Err(e) => {
                                let _ = broker_msg_tx_clone.send(BrokerMsg::Error(format!(
                                    "Kraken positions: {}",
                                    e
                                )));
                            }
                        }
                    } else {
                        let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult("Kraken: connect first".into()));
                    }
                }
                BrokerCmd::KrakenPlaceOrder { pair, side, order_type, volume, price, leverage } => {
                    if let Some(ref kb) = kraken_broker {
                        let msg_tx = broker_msg_tx_clone.clone();
                        match kb.place_order_with_leverage(&pair, &side, &order_type, volume, price, leverage.as_deref()).await {
                            Ok(result) => {
                                let text = serde_json::to_string_pretty(&result).unwrap_or_default();
                                let _ = msg_tx.send(BrokerMsg::OrderResult(format!("Kraken order placed: {}", text)));
                            }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("Kraken order failed: {}", e))); }
                        }
                    } else {
                        let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult("Kraken: connect first".into()));
                    }
                }
                BrokerCmd::KrakenPlaceOrderAdvanced { order } => {
                    if let Some(ref kb) = kraken_broker {
                        let msg_tx = broker_msg_tx_clone.clone();
                        match kb.place_order_request(&order).await {
                            Ok(result) => {
                                let text = serde_json::to_string_pretty(&result).unwrap_or_default();
                                let _ = msg_tx.send(BrokerMsg::OrderResult(format!(
                                    "Kraken order placed: {}",
                                    text
                                )));
                            }
                            Err(e) => {
                                let _ = msg_tx.send(BrokerMsg::Error(format!(
                                    "Kraken order failed: {}",
                                    e
                                )));
                            }
                        }
                    } else {
                        let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult("Kraken: connect first".into()));
                    }
                }
                BrokerCmd::KrakenClosePosition { pair, volume } => {
                    if let Some(ref kb) = kraken_broker {
                        match kb.close_position(&pair, volume).await {
                            Ok(result) => {
                                let text = serde_json::to_string_pretty(&result).unwrap_or_default();
                                let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult(format!(
                                    "Kraken close {}: {}",
                                    pair, text
                                )));
                            }
                            Err(e) => {
                                let _ = broker_msg_tx_clone.send(BrokerMsg::Error(format!(
                                    "Kraken close {}: {}",
                                    pair, e
                                )));
                            }
                        }
                    } else {
                        let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult("Kraken: connect first".into()));
                    }
                }
                BrokerCmd::KrakenCancelOrder { txid } => {
                    if let Some(ref kb) = kraken_broker {
                        let msg_tx = broker_msg_tx_clone.clone();
                        match kb.cancel_order(&txid).await {
                            Ok(result) => {
                                let text = serde_json::to_string_pretty(&result).unwrap_or_default();
                                let _ = msg_tx.send(BrokerMsg::OrderResult(format!("Kraken cancel: {}", text)));
                            }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("Kraken cancel failed: {}", e))); }
                        }
                    } else {
                        let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult("Kraken: connect first".into()));
                    }
                }
                BrokerCmd::KrakenCancelAll => {
                    if let Some(ref kb) = kraken_broker {
                        let msg_tx = broker_msg_tx_clone.clone();
                        match kb.cancel_all_orders().await {
                            Ok(result) => {
                                let count = result["count"].as_u64().unwrap_or(0);
                                let _ = msg_tx.send(BrokerMsg::OrderResult(format!("Kraken: cancelled {} orders", count)));
                            }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("Kraken cancel all failed: {}", e))); }
                        }
                    } else {
                        let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult("Kraken: connect first".into()));
                    }
                }

                BrokerCmd::KrakenFetchTrades => {
                    if let Some(ref kb) = kraken_broker {
                        let msg_tx = broker_msg_tx_clone.clone();
                        match kb.get_all_trades_history_parsed(None, None).await {
                            Ok(trades) => {
                                let _ = msg_tx.send(BrokerMsg::KrakenTrades(trades));
                            }
                            Err(e) => {
                                let _ = msg_tx.send(BrokerMsg::Error(format!("Kraken trade history failed: {}", e)));
                            }
                        }
                    }
                }
                BrokerCmd::KrakenFetchOpenOrders => {
                    if let Some(ref kb) = kraken_broker {
                        let msg_tx = broker_msg_tx_clone.clone();
                        match kb.get_open_orders_parsed().await {
                            Ok(orders) => {
                                let _ = msg_tx.send(BrokerMsg::KrakenOpenOrders(orders));
                            }
                            Err(e) => {
                                let _ = msg_tx.send(BrokerMsg::Error(format!("Kraken open orders failed: {}", e)));
                            }
                        }
                    }
                }
                BrokerCmd::KrakenFetchEquityTicker { symbol } => {
                    let result = if let Some(ref kb) = kraken_broker {
                        kb.get_equity_ticker(&symbol).await
                    } else {
                        let kb = typhoon_engine::broker::kraken::KrakenBroker::new(
                            String::new(),
                            String::new(),
                        );
                        kb.get_equity_ticker(&symbol).await
                    };
                    match result {
                        Ok(ticker) => {
                            let _ = broker_msg_tx_clone.send(BrokerMsg::KrakenEquityQuote(ticker));
                        }
                        Err(e) => {
                            let _ = broker_msg_tx_clone.send(BrokerMsg::Error(e));
                        }
                    }
                }
                BrokerCmd::KrakenFetchEquityHistory { symbol, timeframe } => {
                    // iapi_limiter inside get_equity_history short-circuits
                    // with an IAPI_RATE_LIMITED prefixed error during an
                    // active cooldown. Do the slow network + cache write in
                    // its own capped task; the broker command loop must stay
                    // free to process UI-visible commands and status messages.
                    let msg_tx = broker_msg_tx_clone.clone();
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
                        let kb = typhoon_engine::broker::kraken::KrakenBroker::new(
                            String::new(),
                            String::new(),
                        );
                        let result = kb
                            .get_equity_history(&symbol, interval_minutes, None)
                            .await;
                        match result {
                            Ok(bars) => {
                                let count = bars.len();
                                if count > 0 {
                                    let cache_handle = shared_cache
                                        .read()
                                        .ok()
                                        .and_then(|g| g.clone());
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
                    let msg_tx = broker_msg_tx_clone.clone();
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
                            let bars = fetch_yahoo_chart_bars(&client, &symbol, &timeframe).await?;
                            let count = if let Some(cache) = shared_cache.read().ok().and_then(|g| g.clone()) {
                                store_fallback_bars(&cache, &source, &symbol, &timeframe, &bars)?
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
                                let provider_no_data = yahoo_chart_provider_no_data_error(&error);
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
                    let result = if let Some(ref kb) = kraken_broker {
                        kb.get_equity_markets().await
                    } else {
                        let kb = typhoon_engine::broker::kraken::KrakenBroker::new(
                            String::new(),
                            String::new(),
                        );
                        kb.get_equity_markets().await
                    };
                    match result {
                        Ok(markets) => {
                            let _ = broker_msg_tx_clone.send(BrokerMsg::KrakenEquityUniverse(markets));
                        }
                        Err(e) => {
                            let _ = broker_msg_tx_clone
                                .send(BrokerMsg::Error(format!("Kraken equities universe failed: {e}")));
                        }
                    }
                }
                BrokerCmd::KrakenStartPrivateWs => {
                    let ws_client = kraken_ws_broker.as_ref().or(kraken_broker.as_ref());
                    if let Some(kb) = ws_client {
                        let msg_tx = broker_msg_tx_clone.clone();
                        match kb.start_private_ws().await {
                            Ok(mut rx) => {
                                let value = msg_tx.clone();
                                tokio::spawn(async move {
                                    while let Some(msg) = rx.recv().await {
                                        // Try to parse as ownTrades update
                                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&msg) {
                                            if parsed.get("event").and_then(|v| v.as_str()) == Some("heartbeat") {
                                                continue;
                                            }
                                            let trades = typhoon_engine::broker::kraken::parse_own_trades_messages(&parsed);
                                            if !trades.is_empty() {
                                                for trade in trades {
                                                    let _ = value.send(BrokerMsg::KrakenLiveTrade(trade));
                                                }
                                                continue;
                                            }
                                            if parsed.get("event").and_then(|v| v.as_str()) == Some("systemStatus")
                                                || parsed.get("event").and_then(|v| v.as_str()) == Some("subscriptionStatus")
                                            {
                                                let status = parsed
                                                    .get("status")
                                                    .and_then(|v| v.as_str())
                                                    .unwrap_or("info")
                                                    .to_string();
                                                let channel = parsed
                                                    .get("subscription")
                                                    .and_then(|v| v.get("name"))
                                                    .and_then(|v| v.as_str());
                                                let exchange_message = parsed
                                                    .get("errorMessage")
                                                    .or_else(|| parsed.get("message"))
                                                    .and_then(|v| v.as_str());
                                                let message = match (channel, exchange_message) {
                                                    (Some(channel), Some(detail)) => {
                                                        format!("{channel}: {detail}")
                                                    }
                                                    (Some(channel), None) => channel.to_string(),
                                                    (None, Some(detail)) => detail.to_string(),
                                                    (None, None) => {
                                                        "Kraken private WebSocket status".to_string()
                                                    }
                                                };
                                                let _ = value.send(BrokerMsg::KrakenWsStatus { status, message });
                                                continue;
                                            }
                                            let orders = typhoon_engine::broker::kraken::parse_open_orders_message(&parsed);
                                            if !orders.is_empty() {
                                                let _ = value.send(BrokerMsg::KrakenOpenOrders(orders));
                                                continue;
                                            }
                                        }
                                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&msg) {
                                            let kind = parsed
                                                .get("event")
                                                .or_else(|| parsed.get("channelName"))
                                                .or_else(|| parsed.get("channel"))
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("private-update");
                                            tracing::debug!(
                                                "Unhandled Kraken private WebSocket message suppressed from UI log: {}",
                                                kind
                                            );
                                        } else {
                                            tracing::debug!(
                                                "Unhandled non-JSON Kraken private WebSocket message suppressed from UI log"
                                            );
                                        }
                                    }
                                });
                                let _ = msg_tx.send(BrokerMsg::OrderResult("Kraken private WebSocket started".into()));
                            }
                            Err(e) => {
                                let _ = msg_tx.send(BrokerMsg::Error(format!("Kraken WS failed: {}", e)));
                            }
                        }
                    }
                }
                BrokerCmd::KrakenStartOhlcStreamers { pairs } => {
                    // Bridge channels: streamers write bars into the writer;
                    // writer reports flushes back to the main loop via BrokerMsg.
                    let msg_tx = broker_msg_tx_clone.clone();
                    let pair_count = pairs.len();
                    if pair_count == 0 {
                        let _ = msg_tx.send(BrokerMsg::Error(
                            "KrakenStartOhlcStreamers: no pairs supplied".into(),
                        ));
                    } else {
                        let (commit_tx, mut commit_rx) =
                            tokio::sync::mpsc::unbounded_channel();
                        let (status_tx, mut status_rx) =
                            tokio::sync::mpsc::unbounded_channel();
                        // Drain commits into BrokerMsg::KrakenWsBarsCommitted so the
                        // main loop can update WS-fresh state and skip REST refetch.
                        let commit_msg_tx = msg_tx.clone();
                        tokio::spawn(async move {
                            while let Some(fresh) = commit_rx.recv().await {
                                let _ = commit_msg_tx.send(
                                    BrokerMsg::KrakenWsBarsCommitted { fresh },
                                );
                            }
                        });
                        // Drain lifecycle events into BrokerMsg::KrakenWsOhlcStatus.
                        let status_msg_tx = msg_tx.clone();
                        tokio::spawn(async move {
                            while let Some(event) = status_rx.recv().await {
                                let (interval_min, kind, detail) = match event {
                                    typhoon_engine::broker::kraken::KrakenOhlcStreamerEvent::Connected { interval_min } => {
                                        (interval_min, "connected".to_string(), String::new())
                                    }
                                    typhoon_engine::broker::kraken::KrakenOhlcStreamerEvent::Subscribed { interval_min, batches } => {
                                        (interval_min, "subscribed".to_string(), format!("{batches} batches"))
                                    }
                                    typhoon_engine::broker::kraken::KrakenOhlcStreamerEvent::Disconnected { interval_min, reason } => {
                                        (interval_min, "disconnected".to_string(), reason)
                                    }
                                    typhoon_engine::broker::kraken::KrakenOhlcStreamerEvent::SubscribeFailed { interval_min, reason } => {
                                        (interval_min, "subscribe_failed".to_string(), reason)
                                    }
                                };
                                let _ = status_msg_tx.send(BrokerMsg::KrakenWsOhlcStatus {
                                    interval_min,
                                    kind,
                                    detail,
                                });
                            }
                        });
                        kraken_ohlc_ws::spawn_kraken_ohlc_pipeline(
                            shared_cache_broker.clone(),
                            pairs,
                            commit_tx,
                            status_tx,
                        );
                        let _ = msg_tx.send(BrokerMsg::OrderResult(format!(
                            "Kraken WS OHLC streamers started: {pair_count} pairs × 8 intervals",
                        )));
                    }
                }
                BrokerCmd::KrakenStartOrderbookWs { symbol, depth } => {
                    let msg_tx = broker_msg_tx_clone.clone();
                    let Some(ws_symbol) = typhoon_engine::core::kraken::resolve_kraken_ws_pair(
                        &kraken_public_client,
                        &symbol,
                    )
                    .await
                    else {
                        let _ = msg_tx.send(BrokerMsg::OrderResult(format!(
                            "Kraken WS v2 book skipped: {symbol} is not a Kraken spot pair"
                        )));
                        continue;
                    };
                    let depth = depth.clamp(10, 500);
                    let update_msg_tx = msg_tx.clone();
                    let display_symbol = symbol.clone();
                    let state_symbol = ws_symbol.clone();
                    tokio::spawn(async move {
                        let mut resubscribe_count: u32 = 0;
                        loop {
                            let (book_tx, mut book_rx) = tokio::sync::mpsc::channel::<
                                typhoon_engine::broker::kraken::KrakenWsBookDelta,
                            >(1024);
                            let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel::<
                                typhoon_engine::broker::kraken::KrakenBookStreamerEvent,
                            >();
                            let streamer_symbol = state_symbol.clone();
                            let streamer_handle = tokio::spawn(async move {
                                typhoon_engine::broker::kraken::run_book_streamer(
                                    vec![streamer_symbol],
                                    depth,
                                    book_tx,
                                    event_tx,
                                )
                                .await;
                            });
                            let mut state = typhoon_engine::broker::kraken::KrakenWsBookState::new(
                                state_symbol.clone(),
                                depth,
                            );
                            let mut retry_after_mismatch = false;
                            loop {
                                tokio::select! {
                                    maybe_delta = book_rx.recv() => {
                                        let Some(delta) = maybe_delta else { break; };
                                        match state.apply_delta_with_checksum(&delta) {
                                            Ok(checksum) => {
                                                let text = kraken_ws_v2_book_state_json(
                                                    &display_symbol,
                                                    &state,
                                                    checksum,
                                                    "ok",
                                                );
                                                let _ = update_msg_tx.send(BrokerMsg::KrakenOrderbookUpdate(text));
                                            }
                                            Err(err) => {
                                                let text = kraken_ws_v2_book_state_json(
                                                    &display_symbol,
                                                    &state,
                                                    Some(err.actual),
                                                    "checksum_mismatch",
                                                );
                                                let _ = update_msg_tx.send(BrokerMsg::KrakenOrderbookUpdate(text));
                                                resubscribe_count = resubscribe_count.saturating_add(1);
                                                let _ = update_msg_tx.send(BrokerMsg::Error(format!(
                                                    "Kraken WS v2 book checksum mismatch for {}: expected {}, actual {}; resubscribing snapshot attempt {}",
                                                    err.symbol, err.expected, err.actual, resubscribe_count
                                                )));
                                                retry_after_mismatch = true;
                                                break;
                                            }
                                        }
                                    }
                                    maybe_event = event_rx.recv() => {
                                        let Some(event) = maybe_event else { continue; };
                                        let text = match event {
                                            typhoon_engine::broker::kraken::KrakenBookStreamerEvent::Connected { depth } => {
                                                format!("Kraken WS v2 book connected: {state_symbol} depth {depth}")
                                            }
                                            typhoon_engine::broker::kraken::KrakenBookStreamerEvent::Subscribed { depth, batches } => {
                                                format!("Kraken WS v2 book subscribed: {state_symbol} depth {depth} ({batches} batch)")
                                            }
                                            typhoon_engine::broker::kraken::KrakenBookStreamerEvent::SubscribeFailed { depth, reason } => {
                                                format!("Kraken WS v2 book subscribe failed: {state_symbol} depth {depth}: {reason}")
                                            }
                                            typhoon_engine::broker::kraken::KrakenBookStreamerEvent::Disconnected { depth, reason } => {
                                                format!("Kraken WS v2 book disconnected: {state_symbol} depth {depth}: {reason}")
                                            }
                                        };
                                        let _ = update_msg_tx.send(BrokerMsg::OrderResult(text));
                                    }
                                }
                            }
                            streamer_handle.abort();
                            if !retry_after_mismatch {
                                break;
                            }
                            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                            let _ = update_msg_tx.send(BrokerMsg::OrderResult(format!(
                                "Kraken WS v2 book resubscribing: {state_symbol} depth {depth}"
                            )));
                        }
                    });
                    let _ = msg_tx.send(BrokerMsg::OrderResult(format!(
                        "Kraken WS v2 book starting: {ws_symbol} depth {depth}"
                    )));
                }
                BrokerCmd::KrakenCloseAll => {
                    if let Some(ref kb) = kraken_broker {
                        match kb.close_all_positions().await {
                            Ok(count) => {
                                let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult(format!(
                                    "Kraken: closed {} position(s)",
                                    count
                                )));
                            }
                            Err(e) => {
                                let _ = broker_msg_tx_clone.send(BrokerMsg::Error(format!(
                                    "Kraken close all failed: {}",
                                    e
                                )));
                            }
                        }
                    } else {
                        let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult("Kraken: connect first".into()));
                    }
                }
                BrokerCmd::KrakenGetPairs => {
                    // Public endpoint — no auth needed, create temporary broker if none
                    let msg_tx = broker_msg_tx_clone.clone();
                    let kb = if let Some(ref kb) = kraken_broker {
                        kb.get_tradeable_pairs().await
                    } else {
                        let tmp = typhoon_engine::broker::kraken::KrakenBroker::new(String::new(), String::new());
                        tmp.get_tradeable_pairs().await
                    };
                    match kb {
                        Ok(pairs) => {
                            let _ = msg_tx.send(BrokerMsg::KrakenPairs(pairs));
                        }
                        Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("Kraken pairs: {}", e))); }
                    }
                }
                BrokerCmd::KrakenFuturesGetInstruments => {
                    match typhoon_engine::core::kraken_futures::discover_instruments(
                        &kraken_public_client,
                    )
                    .await
                    {
                        Ok(symbols) => {
                            let _ = broker_msg_tx_clone
                                .send(BrokerMsg::KrakenFuturesInstruments(symbols));
                        }
                        Err(e) => {
                            let _ = broker_msg_tx_clone
                                .send(BrokerMsg::Error(format!("Kraken futures instruments: {}", e)));
                        }
                    }
                }
                BrokerCmd::FundamentalsScrape {
                    db_path: _,
                    use_mt5,
                    use_alpaca,
                    use_kraken,
                    kraken_equity_symbols,
                    force,
                } => {
                    // Gather symbols from brokers BEFORE spawning thread (broker vars are in scope here)
                    let mut extra_tickers: Vec<String> = Vec::new();
                    if use_alpaca {
                        if let Some(ref b) = broker {
                            match b.get_all_assets().await {
                                Ok(assets) => {
                                    let syms: Vec<String> = assets.iter()
                                        .filter(|a| a.asset_class == "us_equity" && a.tradable)
                                        .map(|a| a.symbol.clone()).collect();
                                    let _ = broker_msg_tx_clone.send(BrokerMsg::FundamentalsProgress(format!("Alpaca: {} stock tickers", syms.len())));
                                    extra_tickers.extend(syms);
                                }
                                Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::FundamentalsProgress(format!("Alpaca symbols failed: {}", e))); }
                            }
                        } else {
                            let _ = broker_msg_tx_clone.send(BrokerMsg::FundamentalsProgress("Alpaca not connected — skipping".into()));
                        }
                    }
                    if use_kraken {
                        let syms = normalize_kraken_equity_symbol_list(kraken_equity_symbols.iter());
                        if syms.is_empty() {
                            let _ = broker_msg_tx_clone.send(
                                BrokerMsg::FundamentalsProgress(
                                    "Kraken equities catalog not loaded — fundamentals scrape skipped for Kraken".into(),
                                ),
                            );
                        } else {
                            let _ = broker_msg_tx_clone.send(
                                BrokerMsg::FundamentalsProgress(format!(
                                    "Kraken equities: {} catalog tickers",
                                    syms.len()
                                )),
                            );
                            extra_tickers.extend(syms);
                        }
                    }
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    let _ = std::thread::Builder::new()
                        .name("typhoon-fundamentals-scrape".into())
                        .spawn(move || {
                            let rt = tokio::runtime::Builder::new_current_thread()
                                .enable_all().build().unwrap_or_else(|e| { eprintln!("FATAL: tokio runtime init failed: {e}"); std::process::exit(1); });
                            rt.block_on(async {
                            match shared_cache_broker.read().ok().and_then(|g| g.clone()).ok_or("Cache not ready".to_string()) {
                                Ok(cache) => {
                                    let mut all_tickers: std::collections::HashSet<String> = std::collections::HashSet::new();
                                    // Add broker tickers gathered before thread spawn
                                    all_tickers.extend(extra_tickers);
                                    if use_mt5 {
                                        if let Ok(conn) = cache.connection() {
                                            let _ = fundamentals::create_fundamentals_tables(&conn);
                                            if let Ok(mt5_tickers) = fundamentals::extract_stock_tickers_from_cache(&conn) {
                                                if !mt5_tickers.is_empty() {
                                                    let _ = msg_tx.send(BrokerMsg::FundamentalsProgress(format!("MT5: {} stock tickers", mt5_tickers.len())));
                                                }
                                                all_tickers.extend(mt5_tickers);
                                            }
                                        }
                                    }
                                    let mut tickers: Vec<String> = all_tickers.into_iter().collect();
                                    tickers.sort();
                                    if let Ok(conn) = cache.connection() {
                                        let _ = fundamentals::create_fundamentals_tables(&conn);
                                    }
                                    {
                                        let _ = msg_tx.send(BrokerMsg::FundamentalsProgress(format!("Fundamentals scrape: {} stock tickers found", tickers.len())));
                                        let session = match fundamentals::YahooSession::new().await {
                                            Ok(s) => s,
                                            Err(e) => {
                                                let _ = msg_tx.send(BrokerMsg::Error(format!("Yahoo auth failed: {}", e)));
                                                return;
                                            }
                                        };
                                        // Use 72h over weekends (Sat/Sun) because US equity filings
                                        // are extremely rare outside business days.
                                        let skip_hours: i64 = {
                                            let wd = chrono::Utc::now().weekday();
                                            if wd == chrono::Weekday::Sat || wd == chrono::Weekday::Sun {
                                                72
                                            } else {
                                                24
                                            }
                                        };
                                        let cutoff = (chrono::Utc::now() - chrono::Duration::hours(skip_hours))
                                            .format("%Y-%m-%dT%H:%M:%SZ").to_string();
                                        let mut ok = 0usize;
                                        let mut fail = 0usize;
                                        let mut skipped = 0usize;
                                        let mut consecutive_fail = 0usize;
                                        for ticker in &tickers {
                                            // Acquire write lock per-ticker — release between iterations
                                            // so other threads (BG, Mt5Sync, KV writes) aren't starved.
                                            let skip = if force { false } else if let Ok(conn) = cache.connection() {
                                                if let Ok(Some(existing)) = fundamentals::get_fundamentals(&conn, ticker) {
                                                    existing.last_updated >= cutoff
                                                } else { false }
                                            } else { false }; // conn dropped here
                                            if skip { skipped += 1; continue; }

                                            // Check scrape_failures blocklist (404 etc) — FORCE bypasses
                                            if !force {
                                                let blocklisted = if let Ok(conn) = cache.connection() {
                                                    conn.query_row(
                                                        "SELECT reason FROM scrape_failures WHERE symbol = ?1",
                                                        [ticker.as_str()],
                                                        |row| row.get::<_, String>(0),
                                                    ).ok().is_some()
                                                } else { false };
                                                if blocklisted { skipped += 1; continue; }
                                            }

                                            if consecutive_fail >= 10 {
                                                let _ = msg_tx.send(BrokerMsg::FundamentalsProgress(format!(
                                                    "Aborting: {} consecutive failures. {} OK, {} failed, {} skipped (cached) out of {}",
                                                    consecutive_fail, ok, fail, skipped, tickers.len()
                                                )));
                                                break;
                                            }
                                            // Acquire lock, scrape, release — short hold per ticker
                                            let scrape_result = if let Ok(conn) = cache.connection() {
                                                fundamentals::scrape_ticker(&session, &conn, ticker).await
                                            } else {
                                                Err("DB lock failed".into())
                                            }; // conn dropped here
                                            match scrape_result {
                                                Ok(_f) => {
                                                    ok += 1;
                                                    consecutive_fail = 0;
                                                    let processed = ok + fail + skipped;
                                                    tracing::debug!(
                                                        "Scraped {}: OK ({}/{})",
                                                        ticker,
                                                        processed,
                                                        tickers.len()
                                                    );
                                                    if should_emit_fundamentals_scrape_progress(processed, tickers.len()) {
                                                        let _ = msg_tx.send(BrokerMsg::FundamentalsProgress(format!(
                                                            "Fundamentals progress: {} OK, {} failed, {} skipped ({}/{}) latest {}",
                                                            ok,
                                                            fail,
                                                            skipped,
                                                            processed,
                                                            tickers.len(),
                                                            ticker
                                                        )));
                                                    }
                                                }
                                                Err(e) => {
                                                    // Rate limit: cooldown and retry
                                                    if e.contains("429") || e.contains("Too Many") {
                                                        let _ = msg_tx.send(BrokerMsg::FundamentalsProgress(format!(
                                                            "Rate limited — cooling down 60s... ({}/{})", ok + fail + skipped, tickers.len()
                                                        )));
                                                        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                                                        // Retry this ticker after cooldown
                                                        let retry = if let Ok(conn) = cache.connection() {
                                                            fundamentals::scrape_ticker(&session, &conn, ticker).await
                                                        } else { Err("DB lock".into()) };
                                                        match retry {
                                                            Ok(_) => {
                                                                ok += 1;
                                                                consecutive_fail = 0;
                                                                let processed = ok + fail + skipped;
                                                                tracing::debug!(
                                                                    "Scraped {}: OK (retry) ({}/{})",
                                                                    ticker,
                                                                    processed,
                                                                    tickers.len()
                                                                );
                                                                if should_emit_fundamentals_scrape_progress(processed, tickers.len()) {
                                                                    let _ = msg_tx.send(BrokerMsg::FundamentalsProgress(format!(
                                                                        "Fundamentals progress: {} OK, {} failed, {} skipped ({}/{}) latest {} retry",
                                                                        ok,
                                                                        fail,
                                                                        skipped,
                                                                        processed,
                                                                        tickers.len(),
                                                                        ticker
                                                                    )));
                                                                }
                                                            }
                                                            Err(e2) => {
                                                                fail += 1;
                                                                consecutive_fail += 1;
                                                                let _ = msg_tx.send(BrokerMsg::FundamentalsProgress(format!("Scraped {}: FAIL — {} ({}/{})", ticker, e2, ok + fail + skipped, tickers.len())));
                                                            }
                                                        }
                                                    } else {
                                                        // Record terminal provider coverage gaps so routine Yahoo 400/404 misses
                                                        // stop resurfacing as actionable scrape failures.
                                                        let provider_coverage_gap = is_fundamentals_provider_coverage_gap(&e);
                                                        if provider_coverage_gap {
                                                            if let Ok(conn) = cache.connection() {
                                                                let _ = conn.execute(
                                                                    "INSERT OR REPLACE INTO scrape_failures (symbol, reason, failed_at) VALUES (?1, ?2, datetime('now'))",
                                                                    [ticker.as_str(), e.as_str()],
                                                                );
                                                            }
                                                        }
                                                        fail += 1;
                                                        consecutive_fail += 1;
                                                        if provider_coverage_gap {
                                                            tracing::debug!(
                                                                "Fundamentals provider coverage gap for {}: {} ({}/{})",
                                                                ticker,
                                                                e,
                                                                ok + fail + skipped,
                                                                tickers.len()
                                                            );
                                                        } else {
                                                            let _ = msg_tx.send(BrokerMsg::FundamentalsProgress(format!("Scraped {}: FAIL — {} ({}/{})", ticker, e, ok + fail + skipped, tickers.len())));
                                                        }
                                                    }
                                                }
                                            }
                                            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                                        }
                                        let _ = msg_tx.send(BrokerMsg::FundamentalsProgress(format!(
                                            "Fundamentals scrape complete: {} OK, {} failed, {} skipped (cached <24h) out of {}",
                                            ok, fail, skipped, tickers.len()
                                        )));
                                    }
                                }
                                Err(e) => {
                                    let _ = msg_tx.send(BrokerMsg::Error(format!("Fundamentals: could not open cache: {}", e)));
                                }
                            }
                        });
                    });
                }
                BrokerCmd::FundamentalsScrapeOne { ticker, db_path: _ } => {
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    let _ = std::thread::Builder::new()
                        .name("typhoon-fundamentals-scrape-one".into())
                        .spawn(move || {
                            let rt = tokio::runtime::Builder::new_current_thread()
                                .enable_all().build().unwrap_or_else(|e| { eprintln!("FATAL: tokio runtime init failed: {e}"); std::process::exit(1); });
                            rt.block_on(async {
                            match shared_cache_broker.read().ok().and_then(|g| g.clone()).ok_or("Cache not ready".to_string()) {
                                Ok(cache) => {
                                    if let Ok(conn) = cache.connection() {
                                        let _ = fundamentals::create_fundamentals_tables(&conn);
                                        let session = match fundamentals::YahooSession::new().await {
                                            Ok(s) => s,
                                            Err(e) => {
                                                let _ = msg_tx.send(BrokerMsg::Error(format!("Yahoo auth failed: {}", e)));
                                                return;
                                            }
                                        };
                                        match fundamentals::scrape_ticker(&session, &conn, &ticker).await {
                                            Ok(_f) => {
                                                let _ = msg_tx.send(BrokerMsg::FundamentalsProgress(format!("Scraped {}: OK", ticker)));
                                            }
                                            Err(e) => {
                                                let _ = msg_tx.send(BrokerMsg::FundamentalsProgress(format!("Scraped {}: FAIL — {}", ticker, e)));
                                            }
                                        }
                                    } else {
                                        let _ = msg_tx.send(BrokerMsg::Error("Fundamentals: could not get DB connection".into()));
                                    }
                                }
                                Err(e) => {
                                    let _ = msg_tx.send(BrokerMsg::Error(format!("Fundamentals: could not open cache: {}", e)));
                                }
                            }
                        });
                    });
                }
                BrokerCmd::ResearchScrape { use_mt5, use_alpaca, finnhub_key, fmp_key } => {
                    let mut extra_tickers: Vec<String> = Vec::new();
                    if use_alpaca {
                        if let Some(ref b) = broker {
                            if let Ok(assets) = b.get_all_assets().await {
                                extra_tickers.extend(assets.iter().filter(|a| a.asset_class == "us_equity" && a.tradable).map(|a| a.symbol.clone()));
                            }
                        }
                    }
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    let _ = std::thread::Builder::new()
                        .name("typhoon-research-scrape".into())
                        .spawn(move || {
                            let rt = tokio::runtime::Builder::new_current_thread()
                                .enable_all().build().unwrap_or_else(|e| { eprintln!("FATAL: tokio runtime init failed: {e}"); std::process::exit(1); });
                            rt.block_on(async {
                            use typhoon_engine::core::research;
                            match shared_cache_broker.read().ok().and_then(|g| g.clone()).ok_or("Cache not ready".to_string()) {
                                Ok(cache) => {
                                    let mut all_tickers: std::collections::HashSet<String> = std::collections::HashSet::new();
                                    all_tickers.extend(extra_tickers);
                                    if use_mt5 {
                                        if let Ok(conn) = cache.connection() {
                                            if let Ok(mt5_tickers) = fundamentals::extract_stock_tickers_from_cache(&conn) {
                                                all_tickers.extend(mt5_tickers);
                                            }
                                        }
                                    }
                                    let mut tickers: Vec<String> = all_tickers.into_iter().collect();
                                    tickers.sort();
                                    if let Ok(conn) = cache.connection() {
                                        let _ = research::create_research_tables(&conn);
                                    }
                                    let _ = msg_tx.send(BrokerMsg::FundamentalsProgress(format!("Research scrape: {} tickers queued", tickers.len())));
                                    let client = reqwest::Client::builder()
                                        .user_agent("TyphooN-Terminal/1.0")
                                        .timeout(std::time::Duration::from_secs(15))
                                        .build().unwrap_or_default();
                                    let total = tickers.len();
                                    let mut done = 0usize;
                                    for ticker in &tickers {
                                        let conn_result = cache.connection();
                                        if let Ok(conn) = conn_result {
                                            let tx = msg_tx.clone();
                                            let _ = research::scrape_and_cache_symbol(
                                                &client, &conn, ticker, &finnhub_key, &fmp_key,
                                                |note| { let _ = tx.send(BrokerMsg::FundamentalsProgress(note.to_string())); },
                                            ).await;
                                        }
                                        done += 1;
                                        if done % 10 == 0 || done == total {
                                            let _ = msg_tx.send(BrokerMsg::FundamentalsProgress(format!("Research scrape: {}/{}", done, total)));
                                        }
                                    }
                                    let _ = msg_tx.send(BrokerMsg::FundamentalsProgress(format!("Research scrape complete: {} tickers processed", total)));
                                }
                                Err(e) => {
                                    let _ = msg_tx.send(BrokerMsg::Error(format!("Research scrape: cache not ready: {}", e)));
                                }
                            }
                        });
                    });
                }
                cmd @ (
                    BrokerCmd::CompactStorage { .. }
                    | BrokerCmd::ScanUnusualVolume { .. }
                ) => {
                    storage::handle_storage_command(
                        cmd,
                        broker_msg_tx_clone.clone(),
                        importing_flag.clone(),
                        shared_cache_broker.clone(),
                    );
                }
                BrokerCmd::Mt5Sync {
                    sources,
                    enabled_timeframes,
                } => {
                    // In-flight guard. Auto-sync fires every 30 s; on a cold cache
                    // a full pass can briefly exceed that (batch fsyncs + a large
                    // source). Without this guard, the second tick spawns a second
                    // thread that opens its own target connection and contests the
                    // first sync's write batch — seen as SQLITE_BUSY waits up to
                    // 10 s each. CAS false→true lets exactly one sync run; the
                    // overlapping trigger silently drops, the next 30 s tick tries
                    // again.
                    static MT5_SYNC_IN_FLIGHT: std::sync::atomic::AtomicBool =
                        std::sync::atomic::AtomicBool::new(false);
                    if MT5_SYNC_IN_FLIGHT
                        .compare_exchange(
                            false,
                            true,
                            std::sync::atomic::Ordering::AcqRel,
                            std::sync::atomic::Ordering::Acquire,
                        )
                        .is_err()
                    {
                        tracing::debug!("Mt5Sync: previous pass still running, skipping trigger");
                        continue;
                    }

                    // ── O(1) incremental-sync state ──────────────────────────
                    // Persists target_meta across cycles so the full
                    // target-side detailed_stats scan only runs on cold
                    // start. Per-source last_sync_ts lets each source-side
                    // read become get_cache_meta_since(last_ts − 120s)
                    // instead of a full-table scan — steady-state work is
                    // O(delta), not O(total_keys × N_sources).
                    struct Mt5SyncState {
                        target_meta: std::collections::HashMap<String, (i64, i64)>,
                        last_sync_ts_per_source: std::collections::HashMap<String, i64>,
                        target_warm: bool,
                    }
                    static MT5_SYNC_STATE: std::sync::OnceLock<std::sync::Mutex<Mt5SyncState>> =
                        std::sync::OnceLock::new();

                    // Open a SEPARATE connection for the target — NOT the shared Arc.
                    // Using the shared Arc caused Rust Mutex contention: Mt5Sync's tight
                    // put_raw_blob loop starved try_lock callers (UI try_load, bg detailed_stats),
                    // causing "Cache busy" and empty Storage Manager.
                    //
                    // Dedup sources by canonical path so a shared-/dev/shm topology
                    // (N MT5 prefixes symlink to one tmpfs DB) doesn't read the same
                    // DB N times per cycle. Falls back to the original path if
                    // canonicalize fails (path doesn't exist / broken symlink).
                    let sources: Vec<String> = {
                        let mut seen: std::collections::HashSet<std::path::PathBuf> =
                            std::collections::HashSet::new();
                        let mut out: Vec<String> = Vec::with_capacity(sources.len());
                        for s in sources {
                            let canon = std::fs::canonicalize(&s)
                                .unwrap_or_else(|_| std::path::PathBuf::from(&s));
                            if seen.insert(canon) {
                                out.push(s);
                            }
                        }
                        out
                    };
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::task::spawn_blocking(move || {
                        let enabled_timeframes: std::collections::HashSet<String> =
                            enabled_timeframes
                                .into_iter()
                                .filter_map(|tf| normalize_sync_timeframe_key(&tf))
                                .map(str::to_string)
                                .collect();
                        // RAII guard: releases MT5_SYNC_IN_FLIGHT on every exit path
                        // — early returns on target-open failure, unexpected panics,
                        // and the normal happy path. Without this, a single target
                        // open failure (or a thread panic anywhere below) would leak
                        // the flag and silently disable all future Mt5Sync cycles
                        // until the terminal restarts.
                        struct Mt5SyncGuard;
                        impl Drop for Mt5SyncGuard {
                            fn drop(&mut self) {
                                MT5_SYNC_IN_FLIGHT.store(false, std::sync::atomic::Ordering::Release);
                            }
                        }
                        let _mt5_sync_guard = Mt5SyncGuard;

                        // Filter missing sources up front so `last_src_idx` points at an
                        // actually-existing source and the "N sources" log line reports
                        // the real count being processed. Upstream callers already filter
                        // by .exists(), but a stale path slipping through used to make
                        // last_src_idx reference an entry that the loop would then skip —
                        // silently starving the bid/ask harvest of its one scheduled read.
                        // Filter BEFORE opening the target cache so an all-missing source
                        // list short-circuits before paying for the SQLite open +
                        // detailed_stats scan (rare but wastes ~50 ms on a warm 10K-key DB).
                        let sources: Vec<String> = sources.into_iter()
                            .filter(|p| {
                                if std::path::Path::new(p).exists() { return true; }
                                tracing::debug!("Mt5Sync: skipping missing source {}", p);
                                false
                            })
                            .collect();
                        if sources.is_empty() {
                            tracing::debug!("Mt5Sync: no source paths exist — skipping");
                            return;
                        }

                        let target_path = cache_db_path();
                        let target_cache = match typhoon_engine::core::cache::SqliteCache::open(&target_path) {
                            Ok(c) => c,
                            Err(e) => {
                                let _ = msg_tx.send(BrokerMsg::Error(format!("MT5 sync: cannot open target cache: {e}")));
                                return;
                            }
                        };

                        // Acquire incremental-sync state. Cold-start rebuilds
                        // target_meta once via detailed_stats; every subsequent
                        // pass reuses the persisted map (mutated in-place after
                        // each successful batch commit). Mutex poisoning is
                        // recovered silently — a prior panic would have left
                        // target_meta in a consistent-enough state that re-use
                        // is safer than a forced rebuild on every pass.
                        let state_cell = MT5_SYNC_STATE.get_or_init(|| {
                            std::sync::Mutex::new(Mt5SyncState {
                                target_meta: std::collections::HashMap::new(),
                                last_sync_ts_per_source: std::collections::HashMap::new(),
                                target_warm: false,
                            })
                        });
                        let mut state = match state_cell.lock() {
                            Ok(g) => g,
                            Err(p) => p.into_inner(),
                        };
                        if !state.target_warm {
                            state.target_meta = target_cache.detailed_stats()
                                .unwrap_or_default()
                                .into_iter()
                                .map(|(k, count, ts)| (k, (count, ts)))
                                .collect();
                            state.target_warm = true;
                        }

                        let _ = msg_tx.send(BrokerMsg::OrderResult(format!(
                            "MT5 sync: {} sources → main cache ({} existing keys)",
                            sources.len(), state.target_meta.len()
                        )));

                        let mut total_keys = 0usize;
                        let mut updated = 0usize;
                        let mut skipped_unchanged = 0usize;
                        let mut new_keys = 0usize;
                        let mut read_errors_total = 0usize;
                        let mut heartbeats: Vec<(String, String, i64)> = Vec::new();

                        // bid_ask is read from the *last* source (BarCacheWriter writes
                        // the live quote table there every tick). We fold that read into
                        // the main iteration instead of reopening the DB afterwards, so
                        // the sync pass does one open per source total — not two for the
                        // tail.
                        let last_src_idx = sources.len().saturating_sub(1);
                        let mut live_quotes: Vec<(String, f64, f64)> = Vec::new();
                        // Per-source merged-key accumulator for post-sync cleanup.
                        // Populated only on successful put_raw_blobs commit so a
                        // mid-pass commit failure never causes source-side deletes
                        // of data the target didn't receive. Each tuple is
                        // (key, src_ts_at_read) — the ts guards the delete
                        // against BCW races: if BCW rewrote the row between our
                        // read and our delete, its new ts won't match ours and
                        // the row stays for the next sync pass.
                        let mut merged_keys_per_source: Vec<(String, Vec<(String, i64)>)> =
                            Vec::with_capacity(sources.len());
                        for (src_idx, src_path) in sources.iter().enumerate() {
                            let src = std::path::PathBuf::from(src_path);
                            // Stable per-source key for last_sync_ts_per_source. Canonicalize
                            // so /dev/shm symlinks and relative paths collapse to the same
                            // cursor entry; fall back to the raw string if canonicalize fails
                            // (source removed mid-pass).
                            let canon_key = std::fs::canonicalize(&src)
                                .map(|p| p.to_string_lossy().into_owned())
                                .unwrap_or_else(|_| src_path.clone());
                            match typhoon_engine::core::cache::SqliteCache::open_readonly(&src) {
                                Ok(src_cache) => {
                                    // Collect heartbeat first so UI can show staleness even
                                    // when the rest of the sync pass is a no-op. Log both
                                    // Ok(None) (no heartbeat row — BCW not writing) and
                                    // Err(_) (SQLite read failure) so "no heartbeat yet"
                                    // stuck in the UI is diagnosable from the log rather
                                    // than requiring a live SQLite poke at the source DB.
                                    match src_cache.read_mt5_heartbeat("") {
                                        Ok(Some((json, ts))) => {
                                            heartbeats.push((src_path.clone(), json, ts));
                                        }
                                        Ok(None) => {
                                            tracing::info!(
                                                "Mt5Sync: no heartbeat row in {} (BCW has not written one yet — is the EA attached + running?)",
                                                src_path
                                            );
                                        }
                                        Err(e) => {
                                            tracing::warn!(
                                                "Mt5Sync: heartbeat read failed for {}: {}",
                                                src_path, e
                                            );
                                        }
                                    }
                                    // O(delta) source read. Cursor is the max src_ts we
                                    // successfully committed on the previous pass for this
                                    // canonical source path. A 120s overlap covers host↔Wine
                                    // clock skew plus BCW writes whose timestamp lands a few
                                    // seconds below the max we already saw. Overlap cost is
                                    // bounded by BCW write rate × 120s — at BatchSize=10 and
                                    // 30s cycle that's ≤ 40 keys per pass, negligible versus
                                    // the full-scan baseline of all cached keys.
                                    //
                                    // Cold source (last_seen_ts == 0) still uses the same
                                    // path: get_cache_meta_since(-120) returns every row
                                    // because timestamp > -120 holds for all BCW writes.
                                    let last_seen_ts = state
                                        .last_sync_ts_per_source
                                        .get(&canon_key)
                                        .copied()
                                        .unwrap_or(0);
                                    let cutoff = last_seen_ts.saturating_sub(120);
                                    // get_cache_meta_since returns (key, ts, count). Re-map
                                    // to (key, count, ts) so the downstream loop signature
                                    // matches the original detailed_stats shape.
                                    let src_stats: Vec<(String, i64, i64)> = src_cache
                                        .get_cache_meta_since(cutoff)
                                        .unwrap_or_default()
                                        .into_iter()
                                        .map(|(k, ts, count)| (k, count, ts))
                                        .collect();
                                    total_keys += src_stats.len();
                                    // Diagnostic: when the delta query returns 0 rows, it's
                                    // ambiguous whether the source is actually empty or the
                                    // cursor has drifted ahead of BCW's writes. Surface the
                                    // canonical path, last cursor value, and total bar_cache
                                    // row count so we can tell the two cases apart from the
                                    // log alone without instrumenting further.
                                    if src_stats.is_empty() {
                                        let total_rows = src_cache.stats()
                                            .map(|(b, _, _)| b).unwrap_or(-1);
                                        let file_bytes = std::fs::metadata(&src)
                                            .map(|m| m.len() as i64).unwrap_or(-1);
                                        tracing::info!(
                                            "Mt5Sync: {} — 0 delta rows (cursor={}, cutoff={}, total_bar_cache_rows={}, file_bytes={})",
                                            canon_key, last_seen_ts, cutoff, total_rows, file_bytes
                                        );
                                    }
                                    let mut src_updated = 0usize;
                                    let mut src_skipped = 0usize;
                                    let mut src_new = 0usize;
                                    let mut src_errors = 0usize;

                                    // Track the max src_ts seen during this pass. Advanced
                                    // into state.last_sync_ts_per_source[canon_key] only
                                    // after a successful batch commit — if commit fails,
                                    // the cursor stays where it was so the next pass will
                                    // re-read the same delta.
                                    let mut max_src_ts_this_pass = last_seen_ts;

                                    // Collect writes for this source into a batch that's
                                    // flushed in one transaction at the end — amortises the
                                    // SQLite commit cost across 900+ keys instead of paying
                                    // fsync per put_raw_blob.
                                    let mut pending: Vec<(String, Vec<u8>, i64, i64)> =
                                        Vec::with_capacity(src_stats.len());
                                    // Target-meta updates are deferred alongside `pending`
                                    // and applied as a batch only after put_raw_blobs
                                    // succeeds. Prevents target_meta from drifting ahead of
                                    // disk state on a failed commit (which would cause the
                                    // next pass to skip those keys as "already up to date").
                                    let mut pending_meta_updates: Vec<(String, i64, i64)> =
                                        Vec::with_capacity(src_stats.len());

                                    for (key, src_count, src_ts) in &src_stats {
                                        if *src_ts > max_src_ts_this_pass {
                                            max_src_ts_this_pass = *src_ts;
                                        }
                                        // Metadata keys — BarCacheWriter writes them under
                                        // the `mt5:__<NAME>__[:…]` convention (SYMBOLS list,
                                        // SPECS snapshots, SERVER info, HEARTBEAT). All share
                                        // the `mt5:__` prefix so a single starts_with catches
                                        // the full set without fragile substring probes. Skip
                                        // the read+write when target already has the same or
                                        // newer timestamp — these rarely change.
                                        let is_metadata = key.starts_with("mt5:__");
                                        if is_metadata {
                                            if let Some(&(_, target_ts)) = state.target_meta.get(key) {
                                                if *src_ts <= target_ts { continue; }
                                            }
                                            if let Ok(Some((blob, ts, count))) = src_cache.get_raw_blob(key) {
                                                pending.push((key.clone(), blob, ts, count));
                                                pending_meta_updates.push((key.clone(), count, ts));
                                            }
                                            continue;
                                        }
                                        let Some(tf_suffix) = key.rsplit(':').next() else {
                                            continue;
                                        };
                                        if !enabled_timeframes.contains(tf_suffix) {
                                            continue;
                                        }

                                        // Compare with target
                                        if let Some(&(target_count, target_ts)) = state.target_meta.get(key) {
                                            // Skip if source has same or fewer bars AND same timestamp
                                            if *src_count <= target_count && *src_ts <= target_ts {
                                                src_skipped += 1;
                                                continue;
                                            }
                                            // Note: regression check removed. BarCacheWriter caps at 10K bars per key,
                                            // but the terminal cache may have 100K+ from previous full exports.
                                            // Accept source data if it has newer timestamps — recent bars matter more
                                            // than historical depth for live trading.
                                        } else {
                                            src_new += 1;
                                        }

                                        // Sync: source has more/newer data. Stage blob +
                                        // meta update in pending batch. state.target_meta
                                        // is updated after put_raw_blobs succeeds, so a
                                        // second source in the same pass that carries the
                                        // same key will re-read it via the pre-commit
                                        // snapshot — intentional, it guarantees we always
                                        // keep whichever source has the highest src_ts.
                                        match src_cache.get_raw_blob(key) {
                                            Ok(Some((blob, ts, count))) => {
                                                pending.push((key.clone(), blob, ts, count));
                                                pending_meta_updates.push((key.clone(), count, ts));
                                                src_updated += 1;
                                            }
                                            Ok(None) => {}
                                            Err(_) => { src_errors += 1; }
                                        }
                                    }

                                    // Atomic-from-memory commit: put_raw_blobs + meta
                                    // updates + cursor advance all succeed together, or
                                    // none of them do. On failure the cursor does NOT
                                    // advance, and target_meta stays in sync with disk.
                                    let mut merged_this_source: Vec<(String, i64)> =
                                        Vec::with_capacity(pending.len());
                                    if !pending.is_empty() {
                                        match target_cache.put_raw_blobs(&pending) {
                                            Ok(_) => {
                                                for (k, count, ts) in pending_meta_updates.drain(..) {
                                                    state.target_meta.insert(k.clone(), (count, ts));
                                                    merged_this_source.push((k, ts));
                                                }
                                                state.last_sync_ts_per_source
                                                    .insert(canon_key.clone(), max_src_ts_this_pass);
                                            }
                                            Err(e) => {
                                                src_errors = src_errors.saturating_add(pending.len());
                                                tracing::warn!("Mt5Sync: batch commit failed: {e}");
                                            }
                                        }
                                    } else {
                                        // No writes this pass — we successfully scanned the
                                        // delta and found nothing to copy. Safe to advance
                                        // the cursor so the next pass reads a tighter delta.
                                        state.last_sync_ts_per_source
                                            .insert(canon_key.clone(), max_src_ts_this_pass);
                                    }
                                    merged_keys_per_source.push((src_path.clone(), merged_this_source));

                                    // Harvest bid/ask on the last source so the outer-loop
                                    // post-processing doesn't have to reopen the DB.
                                    if src_idx == last_src_idx {
                                        if let Ok(quotes) = src_cache.read_bid_ask() {
                                            live_quotes = quotes.into_iter()
                                                .map(|(sym, bid, ask, _spread)| (sym, bid, ask))
                                                .collect();
                                        }
                                    }

                                    let src_name = src.file_name().unwrap_or_default().to_string_lossy();
                                    let _ = msg_tx.send(BrokerMsg::OrderResult(format!(
                                        "  {} — {} new, {} updated, {} unchanged, {} errors",
                                        src_name, src_new, src_updated, src_skipped, src_errors
                                    )));

                                    updated += src_updated;
                                    skipped_unchanged += src_skipped;
                                    new_keys += src_new;
                                    read_errors_total += src_errors;
                                }
                                Err(e) => {
                                    let _ = msg_tx.send(BrokerMsg::OrderResult(format!(
                                        "  Skipping {} — locked by BarCacheWriter (will retry next cycle)",
                                        src.file_name().unwrap_or_default().to_string_lossy()
                                    )));
                                    tracing::debug!("Mt5Sync: cannot open {}: {e}", src.display());
                                }
                            }
                        }
                        let changed = new_keys + updated;
                        let _ = msg_tx.send(BrokerMsg::OrderResult(format!(
                            "MT5 sync: {} new + {} updated ({} unchanged, {} errors) from {} keys",
                            new_keys, updated, skipped_unchanged, read_errors_total, total_keys
                        )));

                        // ── Post-merge source cleanup ──────────────────────────
                        // After every source's delta has been merged into the
                        // target, reopen the source in RW mode and DELETE the
                        // exact keys we just copied over. VACUUM follows so
                        // tmpfs pages are released back to /dev/shm rather than
                        // sitting on SQLite's freelist — BCW doesn't configure
                        // auto_vacuum so the freelist never self-reclaims.
                        //
                        // Safety: the target is authoritative. When BCW's next
                        // rotation hits one of these (sym, TF) pairs, its
                        // `INSERT OR REPLACE` re-creates the row from fresh
                        // CopyRates output — bar_track is untouched so BCW's
                        // in-EA last-bar-time tracking survives. If BCW is
                        // still mid-write on this pair, busy_timeout=10 s
                        // from open_source_rw waits it out rather than
                        // clobbering or skipping. Metadata keys (mt5:__…)
                        // are filtered out of the DELETE inside SQLite — BCW
                        // regenerates them every cycle but we keep the row
                        // so ShouldEnterInitialBurst (which counts bar_cache
                        // rows on EA attach) doesn't false-positive into burst
                        // mode on every terminal restart.
                        let mut cleanup_total_deleted = 0u64;
                        let mut cleanup_total_freed_bytes: u64 = 0;
                        for (src_path, merged_keys) in merged_keys_per_source.drain(..) {
                            if merged_keys.is_empty() { continue; }
                            let src = std::path::PathBuf::from(&src_path);
                            let src_name = src.file_name().unwrap_or_default()
                                .to_string_lossy().into_owned();
                            match typhoon_engine::core::cache::SqliteCache::open_source_rw(&src) {
                                Ok(rw) => {
                                    let size_before = std::fs::metadata(&src)
                                        .map(|m| m.len()).unwrap_or(0);
                                    match rw.delete_bar_keys(&merged_keys) {
                                        Ok(n) if n > 0 => {
                                            if let Err(e) = rw.vacuum_source() {
                                                tracing::debug!(
                                                    "Mt5Sync cleanup: VACUUM {} failed: {e}",
                                                    src.display()
                                                );
                                            }
                                            let size_after = std::fs::metadata(&src)
                                                .map(|m| m.len()).unwrap_or(0);
                                            let freed = size_before.saturating_sub(size_after);
                                            cleanup_total_deleted += n;
                                            cleanup_total_freed_bytes += freed;
                                            let _ = msg_tx.send(BrokerMsg::OrderResult(format!(
                                                "  cleanup {} — {} keys purged, {:.1} MiB freed",
                                                src_name, n,
                                                freed as f64 / (1024.0 * 1024.0)
                                            )));
                                        }
                                        Ok(_) => {}
                                        Err(e) => {
                                            tracing::debug!(
                                                "Mt5Sync cleanup: delete_bar_keys {} failed: {e}",
                                                src.display()
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::debug!(
                                        "Mt5Sync cleanup: cannot open {} RW (busy): {e}",
                                        src.display()
                                    );
                                }
                            }
                        }
                        if cleanup_total_deleted > 0 {
                            tracing::info!(
                                "Mt5Sync cleanup: purged {} merged keys from /dev/shm sources, freed {:.1} MiB",
                                cleanup_total_deleted,
                                cleanup_total_freed_bytes as f64 / (1024.0 * 1024.0)
                            );
                        }
                        // Mirror the summary to tracing so it shows up in the terminal's
                        // stdout log — OrderResult only flows to the in-app Log panel,
                        // which made MT5 sync activity invisible to anyone tailing stdout.
                        tracing::info!(
                            "Mt5Sync: {} new + {} updated ({} unchanged, {} errors) from {} keys across {} source(s)",
                            new_keys, updated, skipped_unchanged, read_errors_total, total_keys, sources.len()
                        );
                        // Forward heartbeats to UI so the staleness banner always reflects
                        // the freshest reading, regardless of whether any keys were copied.
                        // Log when no heartbeats were collected so "no heartbeat yet" in
                        // the UI is traceable to a concrete cause (the per-source branches
                        // above already log their individual None/Err outcomes).
                        if heartbeats.is_empty() {
                            tracing::info!(
                                "Mt5Sync: no heartbeats collected from {} source(s) — UI will show 'no heartbeat yet' until BCW writes one",
                                sources.len()
                            );
                        } else {
                            let _ = msg_tx.send(BrokerMsg::Mt5Heartbeat(heartbeats));
                        }
                        // Signal chart reload if any data changed
                        if changed > 0 {
                            let _ = msg_tx.send(BrokerMsg::Mt5SyncDone(changed));
                        }
                        // Live bid/ask was harvested inline during the last source's
                        // pass above (bid_ask table written by BarCacheWriter every
                        // tick). Sending it here keeps the send ordering identical to
                        // the prior pattern while avoiding a redundant SQLite open.
                        if !live_quotes.is_empty() {
                            let _ = msg_tx.send(BrokerMsg::Mt5LiveQuotes(live_quotes));
                        }
                        // In-flight flag release is handled by Mt5SyncGuard's Drop
                        // on every exit path (normal completion, early return, and
                        // panic unwind).
                    });
                }
                BrokerCmd::AlpacaFetchBars { symbol, timeframe, db_path: _, backfill_complete } => {
                    if let Some(ref b) = broker {
                        let broker = b.clone();
                        let msg_tx = broker_msg_tx_clone.clone();
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
                            run_alpaca_fetch_task(
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
                        let _ = broker_msg_tx_clone.send(BrokerMsg::Error(
                            "Broker not connected — connect Alpaca first".into()
                        ));
                        let _ = broker_msg_tx_clone.send(BrokerMsg::AlpacaFetchSettled {
                            symbol,
                            timeframe,
                            success: false,
                        });
                    }
                }
                BrokerCmd::AlpacaFetchBarsBatch { symbols, timeframe } => {
                    if let Some(ref b) = broker {
                        let broker = b.clone();
                        let msg_tx = broker_msg_tx_clone.clone();
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
                            run_alpaca_batch_fetch_task(
                                broker,
                                shared_cache,
                                msg_tx,
                                symbols,
                                timeframe,
                            )
                            .await;
                        });
                    } else {
                        let _ = broker_msg_tx_clone.send(BrokerMsg::Error(
                            "Broker not connected — connect Alpaca first".into(),
                        ));
                        for symbol in symbols {
                            let _ = broker_msg_tx_clone.send(BrokerMsg::AlpacaFetchSettled {
                                symbol,
                                timeframe: timeframe.clone(),
                                success: false,
                            });
                        }
                    }
                }
                BrokerCmd::FetchAllBars { symbol, timeframe } => {
                    // Sequential (not spawned) — prevents flooding Alpaca's rate limiter
                    if let Some(ref b) = broker {
                        let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult(format!(
                            "BARDATA: fetching {} {}...", symbol, timeframe)));
                        match b.get_all_bars(&symbol, &timeframe, None).await {
                            Ok((bars, outcome)) => {
                                let count = bars.len();
                                if count > 0 {
                                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                                        let bare = symbol.replace('/', "");
                                        let key = format!("alpaca:{}:{}", bare, timeframe);
                                        let json = serde_json::to_string(&bars).unwrap_or_default();
                                        let _ = cache.put_bars(&key, &json);
                                        let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult(format!(
                                            "BARDATA: {} {} — {} bars stored", symbol, timeframe, count)));
                                        let _ = broker_msg_tx_clone.send(BrokerMsg::Mt5SyncDone(count));
                                    }
                                }
                                match outcome {
                                    typhoon_engine::broker::alpaca::FetchOutcome::RateLimitedPartial => {
                                        let _ = broker_msg_tx_clone.send(BrokerMsg::AlpacaRetryEnqueue {
                                            symbol: symbol.clone(), timeframe: timeframe.clone(),
                                            reason: "rate_limited_partial".into(),
                                        });
                                    }
                                    typhoon_engine::broker::alpaca::FetchOutcome::RateLimitedEmpty => {
                                        let _ = broker_msg_tx_clone.send(BrokerMsg::AlpacaRetryEnqueue {
                                            symbol: symbol.clone(), timeframe: timeframe.clone(),
                                            reason: "rate_limited_empty".into(),
                                        });
                                    }
                                    typhoon_engine::broker::alpaca::FetchOutcome::Complete => {}
                                }
                            }
                            Err(e) => {
                                let is_rate = e.contains("429") || e.to_lowercase().contains("rate limit");
                                let is_no_data = e.contains("No bar data for ");
                                let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult(format!(
                                    "BARDATA: {} {} — {}", symbol, timeframe, e)));
                                if is_no_data {
                                    let _ = broker_msg_tx_clone.send(BrokerMsg::AlpacaNoData {
                                        symbol: symbol.clone(),
                                        timeframe: timeframe.clone(),
                                        reason: e.clone(),
                                    });
                                } else if is_rate {
                                    let _ = broker_msg_tx_clone.send(BrokerMsg::AlpacaRetryEnqueue {
                                        symbol: symbol.clone(), timeframe: timeframe.clone(),
                                        reason: format!("err:{}", e),
                                    });
                                }
                            }
                        }
                    } else {
                        let _ = broker_msg_tx_clone.send(BrokerMsg::Error("Connect Alpaca first for BARDATA".into()));
                    }
                }
                BrokerCmd::KrakenBackfill {
                    symbol,
                    timeframes,
                    db_path: _,
                    backfill_complete,
                    cryptocompare_backfill_enabled,
                } => {
                    let msg_tx = broker_msg_tx_clone.clone();
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
                            run_kraken_fetch_task(
                                shared_cache,
                                msg_tx,
                                client,
                                symbol,
                                timeframe,
                                backfill_complete,
                                cryptocompare_backfill_enabled,
                            )
                            .await;
                        });
                    }
                }
                BrokerCmd::KrakenFuturesBackfill { symbol, timeframes, db_path: _, backfill_complete } => {
                    let msg_tx = broker_msg_tx_clone.clone();
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
                                let _ = msg_tx.send(BrokerMsg::KrakenFuturesFetchSettled { symbol, timeframe });
                                return;
                            };
                            run_kraken_futures_fetch_task(
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
                BrokerCmd::FetchFilingContent { url } => {
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_fetch = shared_cache_broker.clone();
                    // SEC EDGAR requires a descriptive User-Agent.
                    let client = reqwest::Client::builder()
                        .user_agent(sec_filing::SEC_EDGAR_USER_AGENT)
                        .timeout(std::time::Duration::from_secs(15))
                        .build().unwrap_or_default();
                    // Rate limit: SEC allows 10 req/sec, we do 1
                    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                    match client
                        .get(&url)
                        .header("Accept", "text/html,application/xhtml+xml,application/xml")
                        .header("Accept-Encoding", "identity")
                        .send()
                        .await
                    {
                        Ok(resp) => {
                            let status = resp.status();
                            if !status.is_success() {
                                let _ = msg_tx.send(BrokerMsg::Error(format!(
                                    "Fetch filing failed: HTTP {status}"
                                )));
                                continue;
                            }
                            if let Ok(html) = resp.text().await {
                                let result = sec_filing::strip_html_to_text(&html);
                                // Store content in DB for FTS indexing (growing database)
                                // Extract accession from URL: .../data/{cik}/{accession_nodash}/...
                                let accession = url.split('/').rev().nth(1).unwrap_or("").replace('-', "");
                                if !accession.is_empty() {
                                    if let Some(cache) = shared_cache_fetch.read().ok().and_then(|g| g.clone()) {
                                        if let Ok(conn) = cache.connection() {
                                            // Look up filing metadata for FTS indexing
                                            let like_pat = format!("%{}%", &accession);
                                            let meta: Option<(String, String, String)> = conn.query_row(
                                                "SELECT ticker, form_type, company_name FROM sec_filings WHERE accession_number LIKE ?1 LIMIT 1",
                                                [&like_pat],
                                                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                                            ).ok();
                                            if let Some((ticker, form_type, company)) = meta {
                                                let _ = sec_filing::store_filing_content(&conn, &accession, &ticker, &form_type, &company, &result);
                                            }
                                        }
                                    }
                                }
                                let truncated = if result.len() > 80000 { format!("{}...\n\n[Truncated at 80KB]", &result[..80000]) } else { result };
                                let _ = msg_tx.send(BrokerMsg::FilingContent(truncated));
                            }
                        }
                        Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("Fetch filing failed: {}", e))); }
                    }
                }
                cmd @ (
                    BrokerCmd::FredFetch { .. }
                    | BrokerCmd::FetchEconCalendar { .. }
                    | BrokerCmd::FetchCongressTrades
                    | BrokerCmd::SendNotification { .. }
                ) => {
                    external_feeds::handle_external_feed_command(cmd, broker_msg_tx_clone.clone())
                        .await;
                }
                BrokerCmd::StartStream { trade_symbols, quote_symbols } => {
                    if let Some(ref b) = broker {
                        let msg_tx = broker_msg_tx_clone.clone();
                        let total = trade_symbols.len() + quote_symbols.len();
                        match b.start_stream(trade_symbols, quote_symbols).await {
                            Ok(mut rx) => {
                                let _ = msg_tx.send(BrokerMsg::OrderResult(format!("Stream started for {} symbols", total)));
                                tokio::spawn(async move {
                                    while let Some(msg) = rx.recv().await {
                                        match msg {
                                            typhoon_engine::broker::alpaca::StreamMessage::Trade(t) => {
                                                let _ = msg_tx.send(BrokerMsg::StreamTick {
                                                    symbol: t.symbol, price: t.price, size: t.size, timestamp: t.timestamp,
                                                });
                                            }
                                            typhoon_engine::broker::alpaca::StreamMessage::Quote(q) => {
                                                let _ = msg_tx.send(BrokerMsg::StreamQuoteTick {
                                                    symbol: q.symbol, bid: q.bid, ask: q.ask,
                                                });
                                            }
                                        }
                                    }
                                });
                            }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("Stream failed: {}", e))); }
                        }
                    }
                }
                cmd @ (
                    BrokerCmd::LanSyncStart { .. }
                    | BrokerCmd::LanSyncConnect { .. }
                    | BrokerCmd::LanSyncStop
                    | BrokerCmd::LanResyncBars
                ) => {
                    lan_sync::handle_lan_sync_command(
                        cmd,
                        broker_msg_tx_clone.clone(),
                        shared_cache_broker.clone(),
                        lan_remote_tx_ref.clone(),
                        lan_client.clone(),
                        &mut lan_reconnect_handle,
                    )
                    .await;
                }
            }
        }
    });
}
