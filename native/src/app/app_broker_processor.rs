use super::*;

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
        let mut tt_broker: Option<typhoon_engine::broker::tastytrade::TastytradeBroker> = None;
        let tt_dx_token: Arc<
            tokio::sync::Mutex<Option<typhoon_engine::broker::dxlink::DxLinkToken>>,
        > = Arc::new(tokio::sync::Mutex::new(None));
        let tt_dx_backoff_until: Arc<tokio::sync::Mutex<Option<std::time::Instant>>> =
            Arc::new(tokio::sync::Mutex::new(None));
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
        let kraken_equity_fetch_permits = Arc::new(tokio::sync::Semaphore::new(2));
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
        let tastytrade_fetch_permits = Arc::new(tokio::sync::Semaphore::new(2));
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
                    BrokerCmd::DarwinImportAll { .. } => Some("DARWIN_IMPORT"),
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

                                // Robust target selection: prefer the one that is actually in the future.
                                // This fixes the "wrong open/closed timing" when the broker returns
                                // a stale next_close/next_open relative to the is_open flag.
                                let mut target = if is_open { next_close } else { next_open };
                                let mut target_dt = chrono::DateTime::parse_from_rfc3339(target).ok()
                                    .map(|dt| dt.with_timezone(&chrono::Utc));

                                if target_dt.map_or(true, |dt| dt <= chrono::Utc::now()) {
                                    // Chosen target is in the past — flip to the other one
                                    target = if is_open { next_open } else { next_close };
                                    target_dt = chrono::DateTime::parse_from_rfc3339(target).ok()
                                        .map(|dt| dt.with_timezone(&chrono::Utc));
                                }

                                let countdown = target_dt
                                    .map(|dt| dt - chrono::Utc::now())
                                    .filter(|d| d.num_seconds() > 0)
                                    .map(|d| {
                                        let hours = d.num_hours();
                                        let minutes = (d.num_minutes() % 60).abs();
                                        if hours >= 24 {
                                            format!("{}d {}h", hours / 24, hours % 24)
                                        } else if hours > 0 {
                                            format!("{}h {}m", hours, minutes)
                                        } else {
                                            format!("{}m", minutes.max(1))
                                        }
                                    })
                                    .unwrap_or_else(|| "—".to_string());

                                let msg = if is_open {
                                    format!("US equities OPEN · closes in {countdown}")
                                } else {
                                    format!("US equities CLOSED · opens in {countdown}")
                                };
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

                    // Search tastytrade (if connected)
                    if let Some(ref tb) = tt_broker {
                        if tb.is_authenticated() {
                            if let Ok(results) = tb.search_symbols(&q).await {
                                for item in results.iter().take(10) {
                                    let sym = item["symbol"].as_str().unwrap_or("").to_string();
                                    let desc = item["description"].as_str().unwrap_or("").to_string();
                                    if !sym.is_empty() {
                                        // Deduplicate in O(1): skip if already from Alpaca/tastytrade.
                                        if suggestion_symbols.insert(sym.to_uppercase()) {
                                            all_suggestions.push((sym, desc, "tastytrade".into()));
                                        }
                                    }
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
                BrokerCmd::TastytradeCancelOrder { order_id } => {
                    if let Some(ref mut tt) = tt_broker {
                        match tt.cancel_order(&order_id).await {
                            Ok(_) => { let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult(format!("Tastytrade order {} cancelled", order_id))); }
                            Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(format!("Tastytrade cancel failed: {}", e))); }
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
                BrokerCmd::TastytradeEquityOrder { symbol, qty, side, order_type, price } => {
                    if let Some(ref tb) = tt_broker {
                        match tb.place_equity_order(&symbol, qty as i64, &side, &order_type, price, "GTC").await {
                            Ok(r) => {
                                let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult(format!("tastytrade: {} {} {} {} — order {}", side, qty, symbol, order_type, &r[..r.len().min(60)])));
                            }
                            Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(format!("tastytrade order failed: {}", e))); }
                        }
                    }
                }
                BrokerCmd::TastytradeClosePosition { symbol } => {
                    if let Some(ref tb) = tt_broker {
                        match tb.close_equity_position(&symbol).await {
                            Ok(_) => {
                                let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult(format!("tastytrade: closed position {symbol}")));
                            }
                            Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(format!("tastytrade close {symbol}: {e}"))); }
                        }
                    } else {
                        let _ = broker_msg_tx_clone.send(BrokerMsg::Error("tastytrade: not connected".into()));
                    }
                }
                BrokerCmd::TastytradeClosePositionQty { symbol, qty } => {
                    if let Some(ref tb) = tt_broker {
                        let result = match qty {
                            Some(q) => tb.close_equity_position_qty(&symbol, q).await,
                            None => tb.close_equity_position(&symbol).await,
                        };
                        match result {
                            Ok(_) => {
                                let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult(format!(
                                    "tastytrade: closed {}{}",
                                    symbol,
                                    qty.map(|q| format!(" qty {}", q)).unwrap_or_default()
                                )));
                            }
                            Err(e) => {
                                let _ = broker_msg_tx_clone.send(BrokerMsg::Error(format!(
                                    "tastytrade close {}: {}",
                                    symbol, e
                                )));
                            }
                        }
                    } else {
                        let _ = broker_msg_tx_clone.send(BrokerMsg::Error("tastytrade: not connected".into()));
                    }
                }
                BrokerCmd::TastytradeCloseAll => {
                    if let Some(ref tb) = tt_broker {
                        match tb.close_all_equity_positions().await {
                            Ok(count) => {
                                let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult(format!(
                                    "tastytrade: closed {} position(s)",
                                    count
                                )));
                            }
                            Err(e) => {
                                let _ = broker_msg_tx_clone.send(BrokerMsg::Error(format!(
                                    "tastytrade close all: {}",
                                    e
                                )));
                            }
                        }
                    } else {
                        let _ = broker_msg_tx_clone.send(BrokerMsg::Error("tastytrade: not connected".into()));
                    }
                }
                BrokerCmd::TastytradeCancelLiveExits { symbol } => {
                    if let Some(ref tb) = tt_broker {
                        match tb.cancel_live_exit_orders_for_symbol(&symbol).await {
                            Ok(cancelled) => {
                                if cancelled > 0 {
                                    let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult(format!(
                                        "tastytrade: cancelled {} stale exit order(s) for {}",
                                        cancelled, symbol
                                    )));
                                }
                            }
                            Err(e) => {
                                let _ = broker_msg_tx_clone.send(BrokerMsg::Error(format!(
                                    "tastytrade stale exit cleanup {}: {}",
                                    symbol, e
                                )));
                            }
                        }
                    }
                }
                BrokerCmd::TastytradeSyncExits {
                    symbol,
                    sl_price,
                    tp_price,
                    wait_for_position,
                    wait_for_qty_at_most,
                } => {
                    if let Some(ref tb) = tt_broker {
                        if wait_for_position || wait_for_qty_at_most.is_some() {
                            let mut found = false;
                            for _ in 0..12 {
                                match tb.get_positions().await {
                                    Ok(positions)
                                        if positions.iter().any(|p| {
                                            p.symbol.eq_ignore_ascii_case(&symbol)
                                                && p.quantity.abs() > 0.0
                                                && wait_for_qty_at_most
                                                    .map(|max_qty| p.quantity.abs() <= max_qty + 1e-8)
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
                                            "tastytrade exit sync {}: position poll failed: {}",
                                            symbol, e
                                        )));
                                        break;
                                    }
                                }
                            }
                            if !found {
                                let _ = broker_msg_tx_clone.send(BrokerMsg::Error(format!(
                                    "tastytrade exit sync {}: position not visible at target size yet",
                                    symbol
                                )));
                                continue;
                            }
                        }
                        match tb.sync_equity_position_exits(&symbol, sl_price, tp_price).await {
                            Ok(summary) => {
                                let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult(
                                    format!("tastytrade exits {}: {}", symbol, summary),
                                ));
                                let _ = broker_msg_tx_clone.send(BrokerMsg::TastytradePositions(
                                    tb.get_positions()
                                        .await
                                        .unwrap_or_default()
                                        .iter()
                                        .map(|p| PositionInfo {
                                            symbol: p.symbol.clone(),
                                            qty: p.quantity.abs(),
                                            side: if p.quantity_direction == "Long" {
                                                "long".into()
                                            } else {
                                                "short".into()
                                            },
                                            avg_entry_price: p.average_open_price,
                                            market_value: p.mark_price.unwrap_or(p.close_price)
                                                * p.quantity.abs(),
                                            unrealized_pl: p.unrealized_pnl.unwrap_or(0.0),
                                            asset_class: p.instrument_type.clone(),
                                            asset_id: String::new(),
                                        })
                                        .collect(),
                                ));
                            }
                            Err(e) => {
                                let _ = broker_msg_tx_clone.send(BrokerMsg::Error(format!(
                                    "tastytrade exit sync failed for {}: {}",
                                    symbol, e
                                )));
                            }
                        }
                    } else {
                        let _ = broker_msg_tx_clone.send(BrokerMsg::Error("tastytrade: not connected".into()));
                    }
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
                BrokerCmd::ComputeDdmSnapshot { symbol, required_return_pct, return_source } => {
                    // Pure compute: read cached dividends on the broker thread, call the
                    // compute function, emit a snapshot. Kept on an async task for uniformity
                    // with other research handlers.
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let mut divs: Vec<research::DividendRecord> = Vec::new();
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                if let Ok(Some(d)) = research::get_dividends(&conn, &symbol) {
                                    divs = d;
                                }
                            }
                        }
                        let snap = research::compute_ddm_snapshot(
                            &symbol, &today, &divs, required_return_pct, &return_source,
                        );
                        let _ = msg_tx.send(BrokerMsg::DdmSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeRelativeValuation { symbol, sector, self_json, peers_json } => {
                    use typhoon_engine::core::research;
                    use typhoon_engine::core::fundamentals::Fundamentals;
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let self_fund: Fundamentals = serde_json::from_str(&self_json).unwrap_or_default();
                        let peers: Vec<Fundamentals> = serde_json::from_str(&peers_json).unwrap_or_default();
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let collect = |g: fn(&Fundamentals) -> Option<f64>| -> Vec<f64> {
                            peers.iter().filter_map(g).collect()
                        };
                        let inputs = vec![
                            research::RvMetricInput { metric: "P/E",        value: self_fund.pe_ratio,              peer_values: collect(|f| f.pe_ratio) },
                            research::RvMetricInput { metric: "Fwd P/E",    value: self_fund.forward_pe,            peer_values: collect(|f| f.forward_pe) },
                            research::RvMetricInput { metric: "P/B",        value: self_fund.price_to_book,         peer_values: collect(|f| f.price_to_book) },
                            research::RvMetricInput { metric: "P/S",        value: self_fund.price_to_sales,        peer_values: collect(|f| f.price_to_sales) },
                            research::RvMetricInput { metric: "EV/EBITDA",  value: self_fund.ev_to_ebitda,          peer_values: collect(|f| f.ev_to_ebitda) },
                            research::RvMetricInput { metric: "Profit %",   value: self_fund.profit_margin,         peer_values: collect(|f| f.profit_margin) },
                            research::RvMetricInput { metric: "ROE",        value: self_fund.roe,                   peer_values: collect(|f| f.roe) },
                            research::RvMetricInput { metric: "Beta",       value: self_fund.beta,                  peer_values: collect(|f| f.beta) },
                            research::RvMetricInput { metric: "Div Yield",  value: self_fund.dividend_yield,        peer_values: collect(|f| f.dividend_yield) },
                        ];
                        let rv = research::compute_relative_valuation(&symbol, &sector, &today, &inputs);
                        let _ = msg_tx.send(BrokerMsg::RelativeValuationMsg(symbol, rv));
                    });
                }
                BrokerCmd::FetchFigiIdentifiers { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let client = reqwest::Client::builder()
                            .user_agent("TyphooN-Terminal/1.0")
                            .timeout(std::time::Duration::from_secs(15))
                            .build().unwrap_or_default();
                        match research::fetch_openfigi_identifiers(&client, &symbol).await {
                            Ok(ids) => {
                                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                                let snap = research::FigiSnapshot {
                                    symbol: symbol.to_uppercase(),
                                    as_of: today,
                                    identifiers: ids,
                                };
                                let _ = msg_tx.send(BrokerMsg::FigiSnapshotMsg(symbol, snap));
                            }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("FIGI: {e}"))); }
                        }
                    });
                }
                // ── Round 8 handlers ──
                BrokerCmd::FetchHraSnapshot { symbol, risk_free_pct } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let mut bars: Vec<research::HistoricalPriceRow> = Vec::new();
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                if let Ok(Some(rows)) = research::get_historical_price(&conn, &symbol) {
                                    bars = rows;
                                }
                            }
                        }
                        // compute_hra_snapshot expects oldest-first; cache stores newest-first.
                        if bars.len() >= 2 && bars[0].date > bars[bars.len()-1].date {
                            bars.reverse();
                        }
                        let snap = research::compute_hra_snapshot(&symbol, &today, &bars, risk_free_pct);
                        let _ = msg_tx.send(BrokerMsg::HraSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeDcfSnapshot {
                    symbol,
                    base_revenue, base_fcff,
                    growth_pct, terminal_growth_pct,
                    wacc_pct, tax_rate_pct,
                    projection_years,
                    total_debt, cash_and_equivalents, shares_outstanding,
                } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let snap = research::compute_dcf_snapshot(
                            &symbol, &today,
                            base_revenue, base_fcff,
                            growth_pct, terminal_growth_pct,
                            wacc_pct, tax_rate_pct,
                            projection_years,
                            total_debt, cash_and_equivalents, shares_outstanding,
                        );
                        let _ = msg_tx.send(BrokerMsg::DcfSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeSvmSnapshot {
                    symbol, current_price, ddm_json, dcf_json,
                    peer_pe_tuple_json, peer_ev_tuple_json, peer_pb_tuple_json,
                } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let ddm: Option<research::DdmSnapshot> = serde_json::from_str(&ddm_json).ok();
                        let dcf: Option<research::DcfSnapshot> = serde_json::from_str(&dcf_json).ok();
                        let peer_pe: Option<(f64, f64)> =
                            serde_json::from_str(&peer_pe_tuple_json).unwrap_or(None);
                        let peer_ev: Option<(f64, f64, f64, f64, f64)> =
                            serde_json::from_str(&peer_ev_tuple_json).unwrap_or(None);
                        let peer_pb: Option<(f64, f64)> =
                            serde_json::from_str(&peer_pb_tuple_json).unwrap_or(None);
                        let snap = research::compute_svm_snapshot(
                            &symbol, &today, current_price,
                            ddm.as_ref(), dcf.as_ref(),
                            peer_pe, peer_ev, peer_pb,
                        );
                        let _ = msg_tx.send(BrokerMsg::SvmSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::FetchOptionsChain { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let client = reqwest::Client::builder()
                            .user_agent("Mozilla/5.0 (X11; Linux x86_64) TyphooN-Terminal/0.1")
                            .timeout(std::time::Duration::from_secs(20))
                            .build().unwrap_or_default();
                        match research::fetch_yahoo_options_chain(&client, &symbol).await {
                            Ok(snap) => { let _ = msg_tx.send(BrokerMsg::OptionsChainMsg(symbol, snap)); }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("OMON {symbol}: {e}"))); }
                        }
                    });
                }
                BrokerCmd::ComputeIvolSnapshot { symbol, current_atm_iv_pct, history_json } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let history: Vec<research::IvolObservation> =
                            serde_json::from_str(&history_json).unwrap_or_default();
                        let snap = research::compute_ivol_snapshot(&symbol, &today, current_atm_iv_pct, &history);
                        let _ = msg_tx.send(BrokerMsg::IvolSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 9 handlers ──
                BrokerCmd::ComputeSeasonalitySnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let mut bars: Vec<research::HistoricalPriceRow> = Vec::new();
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                if let Ok(Some(rows)) = research::get_historical_price(&conn, &symbol) {
                                    bars = rows;
                                }
                            }
                        }
                        if bars.len() >= 2 && bars[0].date > bars[bars.len()-1].date {
                            bars.reverse();
                        }
                        let snap = research::compute_seasonality_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::SeasonalitySnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCorrelationMatrix { symbol, window_days, peer_series_json } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let mut subject: Vec<research::HistoricalPriceRow> = Vec::new();
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                if let Ok(Some(rows)) = research::get_historical_price(&conn, &symbol) {
                                    subject = rows;
                                }
                            }
                        }
                        if subject.len() >= 2 && subject[0].date > subject[subject.len()-1].date {
                            subject.reverse();
                        }
                        let peers: Vec<(String, Vec<research::HistoricalPriceRow>)> =
                            serde_json::from_str(&peer_series_json).unwrap_or_default();
                        let snap = research::compute_correlation_matrix(&symbol, &today, window_days, &subject, &peers);
                        let _ = msg_tx.send(BrokerMsg::CorrelationMatrixMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeTotalReturnSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let mut bars: Vec<research::HistoricalPriceRow> = Vec::new();
                        let mut divs: Vec<research::DividendRecord> = Vec::new();
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                if let Ok(Some(rows)) = research::get_historical_price(&conn, &symbol) {
                                    bars = rows;
                                }
                                if let Ok(Some(d)) = research::get_dividends(&conn, &symbol) {
                                    divs = d;
                                }
                            }
                        }
                        if bars.len() >= 2 && bars[0].date > bars[bars.len()-1].date {
                            bars.reverse();
                        }
                        let snap = research::compute_total_return_snapshot(&symbol, &today, &bars, &divs);
                        let _ = msg_tx.send(BrokerMsg::TotalReturnSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeTechnicalsSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let mut bars: Vec<research::HistoricalPriceRow> = Vec::new();
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                if let Ok(Some(rows)) = research::get_historical_price(&conn, &symbol) {
                                    bars = rows;
                                }
                            }
                        }
                        if bars.len() >= 2 && bars[0].date > bars[bars.len()-1].date {
                            bars.reverse();
                        }
                        let snap = research::compute_technical_indicators(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::TechnicalsSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeVolSkewSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let mut chain: Option<research::OptionsChainSnapshot> = None;
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                if let Ok(c) = research::get_options_chain(&conn, &symbol) {
                                    chain = c;
                                }
                            }
                        }
                        let snap = match chain {
                            Some(c) => research::compute_volatility_skew(&symbol, &today, &c),
                            None => research::VolatilitySkew {
                                symbol: symbol.to_uppercase(),
                                as_of: today,
                                note: "no cached OMON chain — run OMON first".to_string(),
                                ..Default::default()
                            },
                        };
                        let _ = msg_tx.send(BrokerMsg::VolSkewSnapshotMsg(symbol, snap));
                    });
                }
                // ── Godel Parity Round 10 ──
                BrokerCmd::ComputeLeverageSnapshot { symbol, total_debt_fund, cash_fund } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let mut statements = research::FinancialStatements::default();
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                if let Ok(Some(s)) = research::get_financials(&conn, &symbol) {
                                    statements = s;
                                }
                            }
                        }
                        let snap = research::compute_leverage_snapshot(&symbol, &today, &statements, total_debt_fund, cash_fund);
                        let _ = msg_tx.send(BrokerMsg::LeverageSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeAccrualsSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let mut statements = research::FinancialStatements::default();
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                if let Ok(Some(s)) = research::get_financials(&conn, &symbol) {
                                    statements = s;
                                }
                            }
                        }
                        let snap = research::compute_accruals_snapshot(&symbol, &today, &statements);
                        let _ = msg_tx.send(BrokerMsg::AccrualsSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeRealizedVolSnapshot { symbol, current_atm_iv_pct, bars_json } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars: Vec<research::HistoricalPriceRow> =
                            serde_json::from_str(&bars_json).unwrap_or_default();
                        let iv = current_atm_iv_pct.unwrap_or(0.0);
                        let snap = research::compute_realized_vol_snapshot(&symbol, &today, &bars, iv);
                        let _ = msg_tx.send(BrokerMsg::RealizedVolSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeFcfYieldSnapshot { symbol, market_cap, stock_price } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let mut statements = research::FinancialStatements::default();
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                if let Ok(Some(s)) = research::get_financials(&conn, &symbol) {
                                    statements = s;
                                }
                            }
                        }
                        let snap = research::compute_fcf_yield_snapshot(&symbol, &today, &statements, market_cap, stock_price);
                        let _ = msg_tx.send(BrokerMsg::FcfYieldSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeShortInterestSnapshot {
                    symbol, shares_out, float_shares, short_pct_of_float, short_ratio_reported, bars_json,
                } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars: Vec<research::HistoricalPriceRow> =
                            serde_json::from_str(&bars_json).unwrap_or_default();
                        let snap = research::compute_short_interest_snapshot(
                            &symbol, &today, shares_out, float_shares,
                            short_pct_of_float, short_ratio_reported, &bars,
                        );
                        let _ = msg_tx.send(BrokerMsg::ShortInterestSnapshotMsg(symbol, snap));
                    });
                }
                // ── Godel Parity Round 11 ──
                BrokerCmd::ComputeAltmanZSnapshot { symbol, market_value_equity } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let mut statements = research::FinancialStatements::default();
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                if let Ok(Some(s)) = research::get_financials(&conn, &symbol) {
                                    statements = s;
                                }
                            }
                        }
                        let snap = research::compute_altman_z_snapshot(&symbol, &today, &statements, market_value_equity);
                        let _ = msg_tx.send(BrokerMsg::AltmanZSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputePiotroskiSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let mut statements = research::FinancialStatements::default();
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                if let Ok(Some(s)) = research::get_financials(&conn, &symbol) {
                                    statements = s;
                                }
                            }
                        }
                        let snap = research::compute_piotroski_snapshot(&symbol, &today, &statements);
                        let _ = msg_tx.send(BrokerMsg::PiotroskiSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeOhlcVolSnapshot { symbol, window_days, bars_json } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars: Vec<research::HistoricalPriceRow> =
                            serde_json::from_str(&bars_json).unwrap_or_default();
                        let snap = research::compute_ohlc_vol_snapshot(&symbol, &today, &bars, window_days);
                        let _ = msg_tx.send(BrokerMsg::OhlcVolSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeEpsBeatSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let mut reports: Vec<research::EarningsSurprise> = Vec::new();
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                if let Ok(Some(r)) = research::get_earnings_surprises(&conn, &symbol) {
                                    reports = r;
                                }
                            }
                        }
                        let snap = research::compute_eps_beat_snapshot(&symbol, &today, &reports);
                        let _ = msg_tx.send(BrokerMsg::EpsBeatSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputePriceTargetDispersionSnapshot { symbol, current_price } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let mut target: Option<research::PriceTarget> = None;
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                if let Ok(Some(t)) = research::get_price_target(&conn, &symbol) {
                                    target = Some(t);
                                }
                            }
                        }
                        let snap = research::compute_price_target_dispersion(&symbol, &today, current_price, target.as_ref());
                        let _ = msg_tx.send(BrokerMsg::PriceTargetDispersionSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeInsiderActivitySnapshot { symbol, window_days } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let mut trades: Vec<research::InsiderTrade> = Vec::new();
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                if let Ok(Some(t)) = research::get_insider_trades(&conn, &symbol) {
                                    trades = t;
                                }
                            }
                        }
                        let snap = research::compute_insider_activity_snapshot(&symbol, &today, &trades, window_days);
                        let _ = msg_tx.send(BrokerMsg::InsiderActivitySnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeDivgSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let mut divs: Vec<research::DividendRecord> = Vec::new();
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                if let Ok(Some(d)) = research::get_dividends(&conn, &symbol) {
                                    divs = d;
                                }
                            }
                        }
                        let snap = research::compute_divg_snapshot(&symbol, &today, &divs);
                        let _ = msg_tx.send(BrokerMsg::DivgSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeEarmSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let mut statements = research::FinancialStatements::default();
                        let mut surprises: Vec<research::EarningsSurprise> = Vec::new();
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                if let Ok(Some(s)) = research::get_financials(&conn, &symbol) {
                                    statements = s;
                                }
                                if let Ok(Some(r)) = research::get_earnings_surprises(&conn, &symbol) {
                                    surprises = r;
                                }
                            }
                        }
                        let snap = research::compute_earm_snapshot(&symbol, &today, &statements, &surprises);
                        let _ = msg_tx.send(BrokerMsg::EarmSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeSectorRotationSnapshot { symbol, symbol_sector } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let mut sectors: Vec<research::SectorPerformance> = Vec::new();
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                if let Ok(Some(rows)) = research::get_sector_performance(&conn) {
                                    sectors = rows;
                                }
                            }
                        }
                        let snap = research::compute_sector_rotation_snapshot(&symbol, &today, &symbol_sector, &sectors);
                        let _ = msg_tx.send(BrokerMsg::SectorRotationSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeUpdmSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let mut actions: Vec<research::RatingChange> = Vec::new();
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                if let Ok(Some(a)) = research::get_rating_changes(&conn, &symbol) {
                                    actions = a;
                                }
                            }
                        }
                        let snap = research::compute_updm_snapshot(&symbol, &today, &actions);
                        let _ = msg_tx.send(BrokerMsg::UpdmSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeMomentumSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let mut bars: Vec<research::HistoricalPriceRow> = Vec::new();
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                if let Ok(Some(rows)) = research::get_historical_price(&conn, &symbol) {
                                    bars = rows;
                                }
                            }
                        }
                        let snap = research::compute_momentum_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::MomentumSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeLiquiditySnapshot { symbol, window_days, shares_outstanding } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let mut bars: Vec<research::HistoricalPriceRow> = Vec::new();
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                if let Ok(Some(rows)) = research::get_historical_price(&conn, &symbol) {
                                    bars = rows;
                                }
                            }
                        }
                        let snap = research::compute_liquidity_snapshot(&symbol, &today, &bars, shares_outstanding, window_days);
                        let _ = msg_tx.send(BrokerMsg::LiquiditySnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeBreakoutSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let mut bars: Vec<research::HistoricalPriceRow> = Vec::new();
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                if let Ok(Some(rows)) = research::get_historical_price(&conn, &symbol) {
                                    bars = rows;
                                }
                            }
                        }
                        let snap = research::compute_breakout_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::BreakoutSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCashCycleSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let statements = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_financials(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { research::FinancialStatements::default() }
                        } else { research::FinancialStatements::default() };
                        let snap = research::compute_cash_cycle_snapshot(&symbol, &today, &statements);
                        let _ = msg_tx.send(BrokerMsg::CashCycleSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCreditSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let (altz, ptfs, lev, acrl) = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                (
                                    research::get_altman_z(&conn, &symbol).ok().flatten(),
                                    research::get_piotroski(&conn, &symbol).ok().flatten(),
                                    research::get_leverage(&conn, &symbol).ok().flatten(),
                                    research::get_accruals(&conn, &symbol).ok().flatten(),
                                )
                            } else { (None, None, None, None) }
                        } else { (None, None, None, None) };
                        let snap = research::compute_credit_snapshot(
                            &symbol, &today,
                            altz.as_ref(), ptfs.as_ref(), lev.as_ref(), acrl.as_ref(),
                        );
                        let _ = msg_tx.send(BrokerMsg::CreditSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeGrowmSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let (mom, earm, divg) = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                (
                                    research::get_momentum(&conn, &symbol).ok().flatten(),
                                    research::get_earm(&conn, &symbol).ok().flatten(),
                                    research::get_divg(&conn, &symbol).ok().flatten(),
                                )
                            } else { (None, None, None) }
                        } else { (None, None, None) };
                        let snap = research::compute_growm_snapshot(
                            &symbol, &today,
                            mom.as_ref(), earm.as_ref(), divg.as_ref(),
                        );
                        let _ = msg_tx.send(BrokerMsg::GrowmSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeFlowSnapshot { symbol, window_days } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let (trades, holders) = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                (
                                    research::get_insider_trades(&conn, &symbol).ok().flatten().unwrap_or_default(),
                                    research::get_institutional_holders(&conn, &symbol).ok().flatten().unwrap_or_default(),
                                )
                            } else { (Vec::new(), Vec::new()) }
                        } else { (Vec::new(), Vec::new()) };
                        let snap = research::compute_flow_snapshot(&symbol, &today, &trades, &holders, window_days);
                        let _ = msg_tx.send(BrokerMsg::FlowSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeRegimeSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let (vole, tech, hra) = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                (
                                    research::get_ohlc_vol(&conn, &symbol).ok().flatten(),
                                    research::get_technicals(&conn, &symbol).ok().flatten(),
                                    research::get_hra(&conn, &symbol).ok().flatten(),
                                )
                            } else { (None, None, None) }
                        } else { (None, None, None) };
                        let snap = research::compute_regime_snapshot(
                            &symbol, &today,
                            vole.as_ref(), tech.as_ref(), hra.as_ref(),
                        );
                        let _ = msg_tx.send(BrokerMsg::RegimeSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeRelvolSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars: Vec<research::HistoricalPriceRow> = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_relvol_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::RelvolSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeMarginsSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let statements = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_financials(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { research::FinancialStatements::default() }
                        } else { research::FinancialStatements::default() };
                        let snap = research::compute_margins_snapshot(&symbol, &today, &statements);
                        let _ = msg_tx.send(BrokerMsg::MarginsSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeValSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    use typhoon_engine::core::fundamentals;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let (subject, peers, fcfy, peer_fcf_yields, sector) = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let subj = fundamentals::get_fundamentals(&conn, &symbol).ok().flatten();
                                let sector = subj.as_ref().map(|s| s.sector.clone()).unwrap_or_default();
                                let mut peers: Vec<fundamentals::Fundamentals> = Vec::new();
                                if !sector.is_empty() {
                                    if let Ok(all) = fundamentals::get_all_fundamentals(&conn) {
                                        for f in all {
                                            if f.sector == sector && f.symbol.to_uppercase() != symbol.to_uppercase() {
                                                peers.push(f);
                                            }
                                        }
                                    }
                                }
                                let subj_fcfy = research::get_fcf_yield(&conn, &symbol).ok().flatten();
                                let mut peer_fcfy: Vec<f64> = Vec::new();
                                for p in &peers {
                                    if let Some(f) = research::get_fcf_yield(&conn, &p.symbol).ok().flatten() {
                                        if f.ttm_fcf_yield_pct.is_finite() && f.ttm_fcf_yield_pct != 0.0 {
                                            peer_fcfy.push(f.ttm_fcf_yield_pct);
                                        }
                                    }
                                }
                                (subj, peers, subj_fcfy, peer_fcfy, sector)
                            } else { (None, Vec::new(), None, Vec::new(), String::new()) }
                        } else { (None, Vec::new(), None, Vec::new(), String::new()) };
                        let snap = research::compute_val_snapshot(
                            &symbol, &today, &sector,
                            subject.as_ref(), &peers, fcfy.as_ref(), &peer_fcf_yields,
                        );
                        let _ = msg_tx.send(BrokerMsg::ValSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeQualSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let (ptfs, margins, acrl, lev) = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                (
                                    research::get_piotroski(&conn, &symbol).ok().flatten(),
                                    research::get_margins(&conn, &symbol).ok().flatten(),
                                    research::get_accruals(&conn, &symbol).ok().flatten(),
                                    research::get_leverage(&conn, &symbol).ok().flatten(),
                                )
                            } else { (None, None, None, None) }
                        } else { (None, None, None, None) };
                        let snap = research::compute_qual_snapshot(
                            &symbol, &today,
                            ptfs.as_ref(), margins.as_ref(), acrl.as_ref(), lev.as_ref(),
                        );
                        let _ = msg_tx.send(BrokerMsg::QualSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeRiskSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let (vole, beta, liq, shrt, altz) = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                (
                                    research::get_ohlc_vol(&conn, &symbol).ok().flatten(),
                                    research::get_beta(&conn, &symbol).ok().flatten(),
                                    research::get_liquidity(&conn, &symbol).ok().flatten(),
                                    research::get_short_interest(&conn, &symbol).ok().flatten(),
                                    research::get_altman_z(&conn, &symbol).ok().flatten(),
                                )
                            } else { (None, None, None, None, None) }
                        } else { (None, None, None, None, None) };
                        let snap = research::compute_risk_snapshot(
                            &symbol, &today,
                            vole.as_ref(), beta.as_ref(), liq.as_ref(), shrt.as_ref(), altz.as_ref(),
                        );
                        let _ = msg_tx.send(BrokerMsg::RiskSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeInsstrkSnapshot { symbol, window_days } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let trades = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_insider_trades(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_insstrk_snapshot(&symbol, &today, &trades, window_days);
                        let _ = msg_tx.send(BrokerMsg::InsstrkSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCovgSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let (pt, recs, updm) = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                (
                                    research::get_price_target(&conn, &symbol).ok().flatten(),
                                    research::get_analyst_recs(&conn, &symbol).ok().flatten().unwrap_or_default(),
                                    research::get_updm(&conn, &symbol).ok().flatten(),
                                )
                            } else { (None, Vec::new(), None) }
                        } else { (None, Vec::new(), None) };
                        let snap = research::compute_covg_snapshot(
                            &symbol, &today,
                            pt.as_ref(), &recs, updm.as_ref(),
                        );
                        let _ = msg_tx.send(BrokerMsg::CovgSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeVrkSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let (subject, peers) = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let subj = research::get_val(&conn, &symbol).ok().flatten();
                                let sector = subj.as_ref().map(|s| s.sector.clone()).unwrap_or_default();
                                let all = research::get_all_val(&conn).unwrap_or_default();
                                let peers: Vec<research::ValueSnapshot> = all.into_iter()
                                    .filter(|v| !sector.is_empty()
                                        && v.sector == sector
                                        && v.symbol.to_uppercase() != symbol.to_uppercase())
                                    .collect();
                                (subj, peers)
                            } else { (None, Vec::new()) }
                        } else { (None, Vec::new()) };
                        let peer_refs: Vec<&research::ValueSnapshot> = peers.iter().collect();
                        let snap = research::compute_vrk_snapshot(&symbol, &today, subject.as_ref(), &peer_refs);
                        let _ = msg_tx.send(BrokerMsg::VrkSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeQrkSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    use typhoon_engine::core::fundamentals;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let (subject, peers, sector) = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let subj = research::get_qual(&conn, &symbol).ok().flatten();
                                let fund = fundamentals::get_fundamentals(&conn, &symbol).ok().flatten();
                                let sector = fund.as_ref().map(|f| f.sector.clone()).unwrap_or_default();
                                let mut peers: Vec<research::QualitySnapshot> = Vec::new();
                                if !sector.is_empty() {
                                    let all = research::get_all_qual(&conn).unwrap_or_default();
                                    for q in all {
                                        if q.symbol.to_uppercase() == symbol.to_uppercase() { continue; }
                                        if let Ok(Some(pf)) = fundamentals::get_fundamentals(&conn, &q.symbol) {
                                            if pf.sector == sector { peers.push(q); }
                                        }
                                    }
                                }
                                (subj, peers, sector)
                            } else { (None, Vec::new(), String::new()) }
                        } else { (None, Vec::new(), String::new()) };
                        let peer_refs: Vec<&research::QualitySnapshot> = peers.iter().collect();
                        let snap = research::compute_qrk_snapshot(&symbol, &today, &sector, subject.as_ref(), &peer_refs);
                        let _ = msg_tx.send(BrokerMsg::QrkSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeRrkSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    use typhoon_engine::core::fundamentals;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let (subject, peers, sector) = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let subj = research::get_risk(&conn, &symbol).ok().flatten();
                                let fund = fundamentals::get_fundamentals(&conn, &symbol).ok().flatten();
                                let sector = fund.as_ref().map(|f| f.sector.clone()).unwrap_or_default();
                                let mut peers: Vec<research::RiskSnapshot> = Vec::new();
                                if !sector.is_empty() {
                                    let all = research::get_all_risk(&conn).unwrap_or_default();
                                    for r in all {
                                        if r.symbol.to_uppercase() == symbol.to_uppercase() { continue; }
                                        if let Ok(Some(pf)) = fundamentals::get_fundamentals(&conn, &r.symbol) {
                                            if pf.sector == sector { peers.push(r); }
                                        }
                                    }
                                }
                                (subj, peers, sector)
                            } else { (None, Vec::new(), String::new()) }
                        } else { (None, Vec::new(), String::new()) };
                        let peer_refs: Vec<&research::RiskSnapshot> = peers.iter().collect();
                        let snap = research::compute_rrk_snapshot(&symbol, &today, &sector, subject.as_ref(), &peer_refs);
                        let _ = msg_tx.send(BrokerMsg::RrkSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeRelepsgrSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    use typhoon_engine::core::fundamentals;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let (subject, peer_stmts, sector) = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let subj = research::get_financials(&conn, &symbol).ok().flatten();
                                let fund = fundamentals::get_fundamentals(&conn, &symbol).ok().flatten();
                                let sector = fund.as_ref().map(|f| f.sector.clone()).unwrap_or_default();
                                let mut peers: Vec<(String, research::FinancialStatements)> = Vec::new();
                                if !sector.is_empty() {
                                    if let Ok(all_f) = fundamentals::get_all_fundamentals(&conn) {
                                        for f in all_f {
                                            if f.sector != sector { continue; }
                                            if f.symbol.to_uppercase() == symbol.to_uppercase() { continue; }
                                            if let Ok(Some(st)) = research::get_financials(&conn, &f.symbol) {
                                                peers.push((f.symbol.clone(), st));
                                            }
                                        }
                                    }
                                }
                                (subj, peers, sector)
                            } else { (None, Vec::new(), String::new()) }
                        } else { (None, Vec::new(), String::new()) };
                        let snap = research::compute_relepsgr_snapshot(
                            &symbol, &today, &sector, subject.as_ref(), &peer_stmts,
                        );
                        let _ = msg_tx.send(BrokerMsg::RelepsgrSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputePeadSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let (surprises, bars) = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                (
                                    research::get_earnings_surprises(&conn, &symbol).ok().flatten().unwrap_or_default(),
                                    research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default(),
                                )
                            } else { (Vec::new(), Vec::new()) }
                        } else { (Vec::new(), Vec::new()) };
                        let snap = research::compute_pead_snapshot(&symbol, &today, &surprises, &bars);
                        let _ = msg_tx.send(BrokerMsg::PeadSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 17 ──
                BrokerCmd::ComputeSizefSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    use typhoon_engine::core::fundamentals;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let (subject_cap, peers, sector) = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let fund = fundamentals::get_fundamentals(&conn, &symbol).ok().flatten();
                                let sector = fund.as_ref().map(|f| f.sector.clone()).unwrap_or_default();
                                let subj_cap: Option<f64> = fund.as_ref().and_then(|f| f.market_cap).filter(|c| *c > 0.0);
                                let mut peers: Vec<(String, f64)> = Vec::new();
                                if !sector.is_empty() {
                                    if let Ok(all_f) = fundamentals::get_all_fundamentals(&conn) {
                                        for f in all_f {
                                            if f.sector != sector { continue; }
                                            if f.symbol.to_uppercase() == symbol.to_uppercase() { continue; }
                                            if let Some(cap) = f.market_cap {
                                                if cap > 0.0 {
                                                    peers.push((f.symbol.clone(), cap));
                                                }
                                            }
                                        }
                                    }
                                }
                                (subj_cap, peers, sector)
                            } else { (None, Vec::new(), String::new()) }
                        } else { (None, Vec::new(), String::new()) };
                        let snap = research::compute_sizef_snapshot(&symbol, &today, &sector, subject_cap, &peers);
                        let _ = msg_tx.send(BrokerMsg::SizefSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeMomfSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    use typhoon_engine::core::fundamentals;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let (subject, peers, sector) = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let subj = research::get_momentum(&conn, &symbol).ok().flatten();
                                let fund = fundamentals::get_fundamentals(&conn, &symbol).ok().flatten();
                                let sector = fund.as_ref().map(|f| f.sector.clone()).unwrap_or_default();
                                let mut peers: Vec<research::MomentumSnapshot> = Vec::new();
                                if !sector.is_empty() {
                                    let all = research::get_all_momentum(&conn).unwrap_or_default();
                                    for m in all {
                                        if m.symbol.to_uppercase() == symbol.to_uppercase() { continue; }
                                        if let Ok(Some(pf)) = fundamentals::get_fundamentals(&conn, &m.symbol) {
                                            if pf.sector == sector { peers.push(m); }
                                        }
                                    }
                                }
                                (subj, peers, sector)
                            } else { (None, Vec::new(), String::new()) }
                        } else { (None, Vec::new(), String::new()) };
                        let peer_refs: Vec<&research::MomentumSnapshot> = peers.iter().collect();
                        let snap = research::compute_momf_snapshot(&symbol, &today, &sector, subject.as_ref(), &peer_refs);
                        let _ = msg_tx.send(BrokerMsg::MomfSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputePeadrankSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    use typhoon_engine::core::fundamentals;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let (subject, peers, sector) = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let subj = research::get_pead(&conn, &symbol).ok().flatten();
                                let fund = fundamentals::get_fundamentals(&conn, &symbol).ok().flatten();
                                let sector = fund.as_ref().map(|f| f.sector.clone()).unwrap_or_default();
                                let mut peers: Vec<research::PeadSnapshot> = Vec::new();
                                if !sector.is_empty() {
                                    let all = research::get_all_pead(&conn).unwrap_or_default();
                                    for p in all {
                                        if p.symbol.to_uppercase() == symbol.to_uppercase() { continue; }
                                        if let Ok(Some(pf)) = fundamentals::get_fundamentals(&conn, &p.symbol) {
                                            if pf.sector == sector { peers.push(p); }
                                        }
                                    }
                                }
                                (subj, peers, sector)
                            } else { (None, Vec::new(), String::new()) }
                        } else { (None, Vec::new(), String::new()) };
                        let peer_refs: Vec<&research::PeadSnapshot> = peers.iter().collect();
                        let snap = research::compute_peadrank_snapshot(&symbol, &today, &sector, subject.as_ref(), &peer_refs);
                        let _ = msg_tx.send(BrokerMsg::PeadrankSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeFqmSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let (ptfs, margins, accruals) = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                (
                                    research::get_piotroski(&conn, &symbol).ok().flatten(),
                                    research::get_margins(&conn, &symbol).ok().flatten(),
                                    research::get_accruals(&conn, &symbol).ok().flatten(),
                                )
                            } else { (None, None, None) }
                        } else { (None, None, None) };
                        let snap = research::compute_fqm_snapshot(&symbol, &today, ptfs.as_ref(), margins.as_ref(), accruals.as_ref());
                        let _ = msg_tx.send(BrokerMsg::FqmSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeRevrankSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    use typhoon_engine::core::fundamentals;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let (subject, peer_stmts, sector) = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let subj = research::get_financials(&conn, &symbol).ok().flatten();
                                let fund = fundamentals::get_fundamentals(&conn, &symbol).ok().flatten();
                                let sector = fund.as_ref().map(|f| f.sector.clone()).unwrap_or_default();
                                let mut peers: Vec<(String, research::FinancialStatements)> = Vec::new();
                                if !sector.is_empty() {
                                    if let Ok(all_f) = fundamentals::get_all_fundamentals(&conn) {
                                        for f in all_f {
                                            if f.sector != sector { continue; }
                                            if f.symbol.to_uppercase() == symbol.to_uppercase() { continue; }
                                            if let Ok(Some(st)) = research::get_financials(&conn, &f.symbol) {
                                                peers.push((f.symbol.clone(), st));
                                            }
                                        }
                                    }
                                }
                                (subj, peers, sector)
                            } else { (None, Vec::new(), String::new()) }
                        } else { (None, Vec::new(), String::new()) };
                        let snap = research::compute_revrank_snapshot(
                            &symbol, &today, &sector, subject.as_ref(), &peer_stmts,
                        );
                        let _ = msg_tx.send(BrokerMsg::RevrankSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeLevrankSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    use typhoon_engine::core::fundamentals;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let (subject, peers, sector) = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let subj = research::get_leverage(&conn, &symbol).ok().flatten();
                                let fund = fundamentals::get_fundamentals(&conn, &symbol).ok().flatten();
                                let sector = fund.as_ref().map(|f| f.sector.clone()).unwrap_or_default();
                                let mut peers: Vec<research::LeverageSnapshot> = Vec::new();
                                if !sector.is_empty() {
                                    let all = research::get_all_leverage(&conn).unwrap_or_default();
                                    for p in all {
                                        if p.symbol.to_uppercase() == symbol.to_uppercase() { continue; }
                                        if let Ok(Some(pf)) = fundamentals::get_fundamentals(&conn, &p.symbol) {
                                            if pf.sector == sector { peers.push(p); }
                                        }
                                    }
                                }
                                (subj, peers, sector)
                            } else { (None, Vec::new(), String::new()) }
                        } else { (None, Vec::new(), String::new()) };
                        let peer_refs: Vec<&research::LeverageSnapshot> = peers.iter().collect();
                        let snap = research::compute_levrank_snapshot(&symbol, &today, &sector, subject.as_ref(), &peer_refs);
                        let _ = msg_tx.send(BrokerMsg::LevrankSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeOperankSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    use typhoon_engine::core::fundamentals;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let (subject, peers, sector) = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let subj = research::get_margins(&conn, &symbol).ok().flatten();
                                let fund = fundamentals::get_fundamentals(&conn, &symbol).ok().flatten();
                                let sector = fund.as_ref().map(|f| f.sector.clone()).unwrap_or_default();
                                let mut peers: Vec<research::MarginsSnapshot> = Vec::new();
                                if !sector.is_empty() {
                                    let all = research::get_all_margins(&conn).unwrap_or_default();
                                    for p in all {
                                        if p.symbol.to_uppercase() == symbol.to_uppercase() { continue; }
                                        if let Ok(Some(pf)) = fundamentals::get_fundamentals(&conn, &p.symbol) {
                                            if pf.sector == sector { peers.push(p); }
                                        }
                                    }
                                }
                                (subj, peers, sector)
                            } else { (None, Vec::new(), String::new()) }
                        } else { (None, Vec::new(), String::new()) };
                        let peer_refs: Vec<&research::MarginsSnapshot> = peers.iter().collect();
                        let snap = research::compute_operank_snapshot(&symbol, &today, &sector, subject.as_ref(), &peer_refs);
                        let _ = msg_tx.send(BrokerMsg::OperankSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeFqmrankSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    use typhoon_engine::core::fundamentals;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let (subject, peers, sector) = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let subj = research::get_fqm(&conn, &symbol).ok().flatten();
                                let fund = fundamentals::get_fundamentals(&conn, &symbol).ok().flatten();
                                let sector = fund.as_ref().map(|f| f.sector.clone()).unwrap_or_default();
                                let mut peers: Vec<research::FundamentalQualityMeterSnapshot> = Vec::new();
                                if !sector.is_empty() {
                                    let all = research::get_all_fqm(&conn).unwrap_or_default();
                                    for p in all {
                                        if p.symbol.to_uppercase() == symbol.to_uppercase() { continue; }
                                        if let Ok(Some(pf)) = fundamentals::get_fundamentals(&conn, &p.symbol) {
                                            if pf.sector == sector { peers.push(p); }
                                        }
                                    }
                                }
                                (subj, peers, sector)
                            } else { (None, Vec::new(), String::new()) }
                        } else { (None, Vec::new(), String::new()) };
                        let peer_refs: Vec<&research::FundamentalQualityMeterSnapshot> = peers.iter().collect();
                        let snap = research::compute_fqmrank_snapshot(&symbol, &today, &sector, subject.as_ref(), &peer_refs);
                        let _ = msg_tx.send(BrokerMsg::FqmrankSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeLiqrankSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    use typhoon_engine::core::fundamentals;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let (subject, peers, sector) = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let subj = research::get_liquidity(&conn, &symbol).ok().flatten();
                                let fund = fundamentals::get_fundamentals(&conn, &symbol).ok().flatten();
                                let sector = fund.as_ref().map(|f| f.sector.clone()).unwrap_or_default();
                                let mut peers: Vec<research::LiquiditySnapshot> = Vec::new();
                                if !sector.is_empty() {
                                    let all = research::get_all_liquidity(&conn).unwrap_or_default();
                                    for p in all {
                                        if p.symbol.to_uppercase() == symbol.to_uppercase() { continue; }
                                        if let Ok(Some(pf)) = fundamentals::get_fundamentals(&conn, &p.symbol) {
                                            if pf.sector == sector { peers.push(p); }
                                        }
                                    }
                                }
                                (subj, peers, sector)
                            } else { (None, Vec::new(), String::new()) }
                        } else { (None, Vec::new(), String::new()) };
                        let peer_refs: Vec<&research::LiquiditySnapshot> = peers.iter().collect();
                        let snap = research::compute_liqrank_snapshot(&symbol, &today, &sector, subject.as_ref(), &peer_refs);
                        let _ = msg_tx.send(BrokerMsg::LiqrankSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeSurpstkSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let surprises = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_earnings_surprises(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_surpstk_snapshot(&symbol, &today, &surprises);
                        let _ = msg_tx.send(BrokerMsg::SurpstkSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeDvdrankSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    use typhoon_engine::core::fundamentals;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let (subject, peers, sector) = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let subj = research::get_divg(&conn, &symbol).ok().flatten();
                                let fund = fundamentals::get_fundamentals(&conn, &symbol).ok().flatten();
                                let sector = fund.as_ref().map(|f| f.sector.clone()).unwrap_or_default();
                                let mut peers: Vec<research::DivgSnapshot> = Vec::new();
                                if !sector.is_empty() {
                                    let all = research::get_all_divg(&conn).unwrap_or_default();
                                    for p in all {
                                        if p.symbol.to_uppercase() == symbol.to_uppercase() { continue; }
                                        if let Ok(Some(pf)) = fundamentals::get_fundamentals(&conn, &p.symbol) {
                                            if pf.sector == sector { peers.push(p); }
                                        }
                                    }
                                }
                                (subj, peers, sector)
                            } else { (None, Vec::new(), String::new()) }
                        } else { (None, Vec::new(), String::new()) };
                        let peer_refs: Vec<&research::DivgSnapshot> = peers.iter().collect();
                        let snap = research::compute_dvdrank_snapshot(&symbol, &today, &sector, subject.as_ref(), &peer_refs);
                        let _ = msg_tx.send(BrokerMsg::DvdrankSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeEarmrankSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    use typhoon_engine::core::fundamentals;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let (subject, peers, sector) = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let subj = research::get_earm(&conn, &symbol).ok().flatten();
                                let fund = fundamentals::get_fundamentals(&conn, &symbol).ok().flatten();
                                let sector = fund.as_ref().map(|f| f.sector.clone()).unwrap_or_default();
                                let mut peers: Vec<research::EarmSnapshot> = Vec::new();
                                if !sector.is_empty() {
                                    let all = research::get_all_earm(&conn).unwrap_or_default();
                                    for p in all {
                                        if p.symbol.to_uppercase() == symbol.to_uppercase() { continue; }
                                        if let Ok(Some(pf)) = fundamentals::get_fundamentals(&conn, &p.symbol) {
                                            if pf.sector == sector { peers.push(p); }
                                        }
                                    }
                                }
                                (subj, peers, sector)
                            } else { (None, Vec::new(), String::new()) }
                        } else { (None, Vec::new(), String::new()) };
                        let peer_refs: Vec<&research::EarmSnapshot> = peers.iter().collect();
                        let snap = research::compute_earmrank_snapshot(&symbol, &today, &sector, subject.as_ref(), &peer_refs);
                        let _ = msg_tx.send(BrokerMsg::EarmrankSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeUpdgrankSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    use typhoon_engine::core::fundamentals;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let (subject, peers, sector) = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let subj = research::get_updm(&conn, &symbol).ok().flatten();
                                let fund = fundamentals::get_fundamentals(&conn, &symbol).ok().flatten();
                                let sector = fund.as_ref().map(|f| f.sector.clone()).unwrap_or_default();
                                let mut peers: Vec<research::UpdmSnapshot> = Vec::new();
                                if !sector.is_empty() {
                                    let all = research::get_all_updm(&conn).unwrap_or_default();
                                    for p in all {
                                        if p.symbol.to_uppercase() == symbol.to_uppercase() { continue; }
                                        if let Ok(Some(pf)) = fundamentals::get_fundamentals(&conn, &p.symbol) {
                                            if pf.sector == sector { peers.push(p); }
                                        }
                                    }
                                }
                                (subj, peers, sector)
                            } else { (None, Vec::new(), String::new()) }
                        } else { (None, Vec::new(), String::new()) };
                        let peer_refs: Vec<&research::UpdmSnapshot> = peers.iter().collect();
                        let snap = research::compute_updgrank_snapshot(&symbol, &today, &sector, subject.as_ref(), &peer_refs);
                        let _ = msg_tx.send(BrokerMsg::UpdgrankSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeGySnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_gy_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::GySnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeDesSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_des_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::DesSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeDvdyieldrankSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    use typhoon_engine::core::fundamentals;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let (subject_yield, peers, sector) = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let fund = fundamentals::get_fundamentals(&conn, &symbol).ok().flatten();
                                let sector = fund.as_ref().map(|f| f.sector.clone()).unwrap_or_default();
                                let subj_y = fund.as_ref().and_then(|f| f.dividend_yield);
                                let mut peers: Vec<(String, Option<f64>)> = Vec::new();
                                if !sector.is_empty() {
                                    let all = fundamentals::get_all_fundamentals(&conn).unwrap_or_default();
                                    for p in all {
                                        if p.symbol.to_uppercase() == symbol.to_uppercase() { continue; }
                                        if p.sector == sector {
                                            peers.push((p.symbol.clone(), p.dividend_yield));
                                        }
                                    }
                                }
                                (subj_y, peers, sector)
                            } else { (None, Vec::new(), String::new()) }
                        } else { (None, Vec::new(), String::new()) };
                        let snap = research::compute_dvdyieldrank_snapshot(&symbol, &today, &sector, subject_yield, &peers);
                        let _ = msg_tx.send(BrokerMsg::DvdyieldrankSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeShrankSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    use typhoon_engine::core::fundamentals;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let (subject_short, peers, sector) = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let fund = fundamentals::get_fundamentals(&conn, &symbol).ok().flatten();
                                let sector = fund.as_ref().map(|f| f.sector.clone()).unwrap_or_default();
                                let subj_s = fund.as_ref().and_then(|f| f.short_percent_of_float);
                                let mut peers: Vec<(String, Option<f64>)> = Vec::new();
                                if !sector.is_empty() {
                                    let all = fundamentals::get_all_fundamentals(&conn).unwrap_or_default();
                                    for p in all {
                                        if p.symbol.to_uppercase() == symbol.to_uppercase() { continue; }
                                        if p.sector == sector {
                                            peers.push((p.symbol.clone(), p.short_percent_of_float));
                                        }
                                    }
                                }
                                (subj_s, peers, sector)
                            } else { (None, Vec::new(), String::new()) }
                        } else { (None, Vec::new(), String::new()) };
                        let snap = research::compute_shrank_snapshot(&symbol, &today, &sector, subject_short, &peers);
                        let _ = msg_tx.send(BrokerMsg::ShrankSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeShortrankDeltaSnapshot { symbol } => {
                    use typhoon_engine::core::fundamentals;
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let (subject_history, peers, sector) = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let fund = fundamentals::get_fundamentals(&conn, &symbol).ok().flatten();
                                let sector = fund.as_ref().map(|f| f.sector.clone()).unwrap_or_default();
                                let subject_history = research::get_short_interest_history(&conn, &symbol)
                                    .ok()
                                    .flatten()
                                    .unwrap_or_default();
                                let mut peers: Vec<(String, Vec<research::ShortInterestHistoryPoint>)> = Vec::new();
                                if !sector.is_empty() {
                                    let all = fundamentals::get_all_fundamentals(&conn).unwrap_or_default();
                                    for p in all {
                                        if p.symbol.to_uppercase() == symbol.to_uppercase() {
                                            continue;
                                        }
                                        if p.sector == sector {
                                            let history = research::get_short_interest_history(&conn, &p.symbol)
                                                .ok()
                                                .flatten()
                                                .unwrap_or_default();
                                            peers.push((p.symbol.clone(), history));
                                        }
                                    }
                                }
                                (subject_history, peers, sector)
                            } else { (Vec::new(), Vec::new(), String::new()) }
                        } else { (Vec::new(), Vec::new(), String::new()) };
                        let snap = research::compute_shortrank_delta_snapshot(
                            &symbol,
                            &today,
                            &sector,
                            &subject_history,
                            &peers,
                        );
                        let _ = msg_tx.send(BrokerMsg::ShortrankDeltaSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeInsiderconcSnapshot { symbol } => {
                    use typhoon_engine::core::fundamentals;
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let (subject_shares_outstanding, subject_trades, peers, sector) =
                            if let Some(cache) =
                                shared_cache_broker.read().ok().and_then(|g| g.clone())
                            {
                                if let Ok(conn) = cache.connection() {
                                    let fund =
                                        fundamentals::get_fundamentals(&conn, &symbol).ok().flatten();
                                    let sector = fund
                                        .as_ref()
                                        .map(|f| f.sector.clone())
                                        .unwrap_or_default();
                                    let subject_shares_outstanding =
                                        fund.as_ref().and_then(|f| f.shares_outstanding);
                                    let subject_trades =
                                        research::get_insider_trades(&conn, &symbol)
                                            .ok()
                                            .flatten()
                                            .unwrap_or_default();
                                    let mut peers: Vec<(
                                        String,
                                        Option<f64>,
                                        Vec<research::InsiderTrade>,
                                    )> = Vec::new();
                                    if !sector.is_empty() {
                                        let all =
                                            fundamentals::get_all_fundamentals(&conn).unwrap_or_default();
                                        for p in all {
                                            if p.symbol.to_uppercase() == symbol.to_uppercase() {
                                                continue;
                                            }
                                            if p.sector == sector {
                                                let trades = research::get_insider_trades(
                                                    &conn, &p.symbol,
                                                )
                                                .ok()
                                                .flatten()
                                                .unwrap_or_default();
                                                peers.push((
                                                    p.symbol.clone(),
                                                    p.shares_outstanding,
                                                    trades,
                                                ));
                                            }
                                        }
                                    }
                                    (subject_shares_outstanding, subject_trades, peers, sector)
                                } else {
                                    (None, Vec::new(), Vec::new(), String::new())
                                }
                            } else {
                                (None, Vec::new(), Vec::new(), String::new())
                            };
                        let snap = research::compute_insiderconc_snapshot(
                            &symbol,
                            &today,
                            &sector,
                            subject_shares_outstanding,
                            &subject_trades,
                            &peers,
                        );
                        let _ = msg_tx.send(BrokerMsg::InsiderconcSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeAtrannSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_atrann_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::AtrannSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeDdhistSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_ddhist_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::DdhistSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputePriceperfSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_priceperf_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::PriceperfSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeMomrankMultiSnapshot { symbol } => {
                    use typhoon_engine::core::fundamentals;
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let (subject, peers, sector) = if let Some(cache) =
                            shared_cache_broker.read().ok().and_then(|g| g.clone())
                        {
                            if let Ok(conn) = cache.connection() {
                                let fund =
                                    fundamentals::get_fundamentals(&conn, &symbol).ok().flatten();
                                let sector = fund
                                    .as_ref()
                                    .map(|f| f.sector.clone())
                                    .unwrap_or_default();
                                let subject =
                                    research::get_priceperf(&conn, &symbol).ok().flatten();
                                let mut peers: Vec<(
                                    String,
                                    Option<research::PricePerformanceSnapshot>,
                                )> = Vec::new();
                                if !sector.is_empty() {
                                    let all =
                                        fundamentals::get_all_fundamentals(&conn).unwrap_or_default();
                                    for p in all {
                                        if p.symbol.eq_ignore_ascii_case(&symbol) || p.sector != sector {
                                            continue;
                                        }
                                        peers.push((
                                            p.symbol.clone(),
                                            research::get_priceperf(&conn, &p.symbol)
                                                .ok()
                                                .flatten(),
                                        ));
                                    }
                                }
                                (subject, peers, sector)
                            } else {
                                (None, Vec::new(), String::new())
                            }
                        } else {
                            (None, Vec::new(), String::new())
                        };
                        let snap = research::compute_momrank_multi_snapshot(
                            &symbol,
                            &today,
                            &sector,
                            subject.as_ref(),
                            &peers,
                        );
                        let _ = msg_tx.send(BrokerMsg::MomrankMultiSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeBetarankSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    use typhoon_engine::core::fundamentals;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let (subject_beta, peers, sector) = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let fund = fundamentals::get_fundamentals(&conn, &symbol).ok().flatten();
                                let sector = fund.as_ref().map(|f| f.sector.clone()).unwrap_or_default();
                                let subj_b = fund.as_ref().and_then(|f| f.beta);
                                let mut peers: Vec<(String, Option<f64>)> = Vec::new();
                                if !sector.is_empty() {
                                    let all = fundamentals::get_all_fundamentals(&conn).unwrap_or_default();
                                    for p in all {
                                        if p.symbol.to_uppercase() == symbol.to_uppercase() { continue; }
                                        if p.sector == sector {
                                            peers.push((p.symbol.clone(), p.beta));
                                        }
                                    }
                                }
                                (subj_b, peers, sector)
                            } else { (None, Vec::new(), String::new()) }
                        } else { (None, Vec::new(), String::new()) };
                        let snap = research::compute_betarank_snapshot(&symbol, &today, &sector, subject_beta, &peers);
                        let _ = msg_tx.send(BrokerMsg::BetarankSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputePegrankSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    use typhoon_engine::core::fundamentals;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let (subject_peg, peers, sector) = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let fund = fundamentals::get_fundamentals(&conn, &symbol).ok().flatten();
                                let sector = fund.as_ref().map(|f| f.sector.clone()).unwrap_or_default();
                                let subj_p = fund.as_ref().and_then(|f| f.peg_ratio);
                                let mut peers: Vec<(String, Option<f64>)> = Vec::new();
                                if !sector.is_empty() {
                                    let all = fundamentals::get_all_fundamentals(&conn).unwrap_or_default();
                                    for p in all {
                                        if p.symbol.to_uppercase() == symbol.to_uppercase() { continue; }
                                        if p.sector == sector {
                                            peers.push((p.symbol.clone(), p.peg_ratio));
                                        }
                                    }
                                }
                                (subj_p, peers, sector)
                            } else { (None, Vec::new(), String::new()) }
                        } else { (None, Vec::new(), String::new()) };
                        let snap = research::compute_pegrank_snapshot(&symbol, &today, &sector, subject_peg, &peers);
                        let _ = msg_tx.send(BrokerMsg::PegrankSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeFhighlowSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_fhighlow_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::FhighlowSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeRvconeSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_rvcone_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::RvconeSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCalpbSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_calpb_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::CalpbSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCorrstkSnapshot {
                    symbol,
                    symbol_sector,
                    fmp_key,
                } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let sector_benchmark =
                            research::sector_to_benchmark_etf(&symbol_sector).map(str::to_string);
                        let (mut subject_bars, mut market_bars, mut sector_bars) =
                            if let Some(cache) =
                                shared_cache_broker.read().ok().and_then(|g| g.clone())
                            {
                                if let Ok(conn) = cache.connection() {
                                    (
                                        research::get_historical_price(&conn, &symbol)
                                            .ok()
                                            .flatten()
                                            .unwrap_or_default(),
                                        research::get_historical_price(&conn, "SPY")
                                            .ok()
                                            .flatten()
                                            .unwrap_or_default(),
                                        sector_benchmark
                                            .as_ref()
                                            .and_then(|etf| {
                                                research::get_historical_price(&conn, etf)
                                                    .ok()
                                                    .flatten()
                                            })
                                            .unwrap_or_default(),
                                    )
                                } else {
                                    (Vec::new(), Vec::new(), Vec::new())
                                }
                            } else {
                                (Vec::new(), Vec::new(), Vec::new())
                            };

                        if !fmp_key.trim().is_empty() {
                            let client = reqwest::Client::builder()
                                .user_agent("TyphooN-Terminal/1.0")
                                .timeout(std::time::Duration::from_secs(30))
                                .build()
                                .unwrap_or_default();
                            if subject_bars.len() < 260 {
                                if let Ok(rows) = research::fetch_fmp_historical_price(
                                    &client, &symbol, &fmp_key, 1300,
                                )
                                .await
                                {
                                    subject_bars = rows;
                                }
                            }
                            if market_bars.len() < 260 {
                                if let Ok(rows) = research::fetch_fmp_historical_price(
                                    &client, "SPY", &fmp_key, 1300,
                                )
                                .await
                                {
                                    market_bars = rows;
                                }
                            }
                            if let Some(ref etf) = sector_benchmark {
                                if sector_bars.len() < 260 {
                                    if let Ok(rows) = research::fetch_fmp_historical_price(
                                        &client, etf, &fmp_key, 1300,
                                    )
                                    .await
                                    {
                                        sector_bars = rows;
                                    }
                                }
                            }
                        }

                        let snap = research::compute_corrstk_snapshot(
                            &symbol,
                            &today,
                            &symbol_sector,
                            "SPY",
                            &subject_bars,
                            &market_bars,
                            sector_benchmark.as_deref(),
                            &sector_bars,
                        );
                        let _ = msg_tx.send(BrokerMsg::CorrstkSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeTlrankSnapshot { symbol } => {
                    use typhoon_engine::core::fundamentals;
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let (subject_bars, peers, sector) = if let Some(cache) =
                            shared_cache_broker.read().ok().and_then(|g| g.clone())
                        {
                            if let Ok(conn) = cache.connection() {
                                let subject_bars = research::get_historical_price(&conn, &symbol)
                                    .ok()
                                    .flatten()
                                    .unwrap_or_default();
                                let subject_sector =
                                    fundamentals::get_fundamentals(&conn, &symbol)
                                        .ok()
                                        .flatten()
                                        .map(|f| f.sector)
                                        .unwrap_or_default();
                                let mut peers = Vec::new();
                                if !subject_sector.is_empty() {
                                    let all =
                                        fundamentals::get_all_fundamentals(&conn).unwrap_or_default();
                                    for peer in all {
                                        if peer.symbol.eq_ignore_ascii_case(&symbol) {
                                            continue;
                                        }
                                        if peer.sector == subject_sector {
                                            let bars = research::get_historical_price(
                                                &conn,
                                                &peer.symbol,
                                            )
                                            .ok()
                                            .flatten()
                                            .unwrap_or_default();
                                            peers.push((peer.symbol, bars));
                                        }
                                    }
                                }
                                (subject_bars, peers, subject_sector)
                            } else {
                                (Vec::new(), Vec::new(), String::new())
                            }
                        } else {
                            (Vec::new(), Vec::new(), String::new())
                        };
                        let snap = research::compute_tlrank_snapshot(
                            &symbol,
                            &today,
                            &sector,
                            &subject_bars,
                            &peers,
                        );
                        let _ = msg_tx.send(BrokerMsg::TlrankSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCorrrankSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let (subject, peers, sector) = if let Some(cache) =
                            shared_cache_broker.read().ok().and_then(|g| g.clone())
                        {
                            if let Ok(conn) = cache.connection() {
                                let subject = research::get_corrstk(&conn, &symbol)
                                    .ok()
                                    .flatten();
                                let subject_sector = subject
                                    .as_ref()
                                    .map(|s| s.symbol_sector.clone())
                                    .unwrap_or_default();
                                let peers = if !subject_sector.is_empty() {
                                    research::get_all_corrstk(&conn)
                                        .unwrap_or_default()
                                        .into_iter()
                                        .filter(|p| {
                                            !p.symbol.eq_ignore_ascii_case(&symbol)
                                                && p.symbol_sector == subject_sector
                                        })
                                        .collect::<Vec<_>>()
                                } else {
                                    Vec::new()
                                };
                                (subject, peers, subject_sector)
                            } else {
                                (None, Vec::new(), String::new())
                            }
                        } else {
                            (None, Vec::new(), String::new())
                        };
                        let peer_refs: Vec<&research::CorrStkSnapshot> = peers.iter().collect();
                        let snap = research::compute_corrrank_snapshot(
                            &symbol,
                            &today,
                            &sector,
                            subject.as_ref(),
                            &peer_refs,
                        );
                        let _ = msg_tx.send(BrokerMsg::CorrrankSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeOperankDeltaSnapshot { symbol } => {
                    use typhoon_engine::core::fundamentals;
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let (subject, peers, sector) = if let Some(cache) =
                            shared_cache_broker.read().ok().and_then(|g| g.clone())
                        {
                            if let Ok(conn) = cache.connection() {
                                let subj = research::get_margins(&conn, &symbol).ok().flatten();
                                let fund =
                                    fundamentals::get_fundamentals(&conn, &symbol).ok().flatten();
                                let sector =
                                    fund.as_ref().map(|f| f.sector.clone()).unwrap_or_default();
                                let mut peers: Vec<research::MarginsSnapshot> = Vec::new();
                                if !sector.is_empty() {
                                    let all = research::get_all_margins(&conn).unwrap_or_default();
                                    for p in all {
                                        if p.symbol.eq_ignore_ascii_case(&symbol) {
                                            continue;
                                        }
                                        if let Ok(Some(pf)) =
                                            fundamentals::get_fundamentals(&conn, &p.symbol)
                                        {
                                            if pf.sector == sector {
                                                peers.push(p);
                                            }
                                        }
                                    }
                                }
                                (subj, peers, sector)
                            } else {
                                (None, Vec::new(), String::new())
                            }
                        } else {
                            (None, Vec::new(), String::new())
                        };
                        let peer_refs: Vec<&research::MarginsSnapshot> = peers.iter().collect();
                        let snap = research::compute_operank_delta_snapshot(
                            &symbol,
                            &today,
                            &sector,
                            subject.as_ref(),
                            &peer_refs,
                        );
                        let _ = msg_tx.send(BrokerMsg::OperankDeltaSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeDivaccSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let dividends = if let Some(cache) =
                            shared_cache_broker.read().ok().and_then(|g| g.clone())
                        {
                            if let Ok(conn) = cache.connection() {
                                research::get_dividends(&conn, &symbol)
                                    .ok()
                                    .flatten()
                                    .unwrap_or_default()
                            } else {
                                Vec::new()
                            }
                        } else {
                            Vec::new()
                        };
                        let snap = research::compute_divacc_snapshot(&symbol, &today, &dividends);
                        let _ = msg_tx.send(BrokerMsg::DivaccSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeEpsaccSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let statements = if let Some(cache) =
                            shared_cache_broker.read().ok().and_then(|g| g.clone())
                        {
                            if let Ok(conn) = cache.connection() {
                                research::get_financials(&conn, &symbol)
                                    .ok()
                                    .flatten()
                                    .unwrap_or_default()
                            } else {
                                research::FinancialStatements::default()
                            }
                        } else {
                            research::FinancialStatements::default()
                        };
                        let snap = research::compute_epsacc_snapshot(&symbol, &today, &statements);
                        let _ = msg_tx.send(BrokerMsg::EpsaccSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeVrpSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let (ivol, rvcone) = if let Some(cache) =
                            shared_cache_broker.read().ok().and_then(|g| g.clone())
                        {
                            if let Ok(conn) = cache.connection() {
                                (
                                    research::get_ivol(&conn, &symbol).ok().flatten(),
                                    research::get_rvcone(&conn, &symbol).ok().flatten(),
                                )
                            } else {
                                (None, None)
                            }
                        } else {
                            (None, None)
                        };
                        let snap = research::compute_vrp_snapshot(
                            &symbol,
                            &today,
                            ivol.as_ref(),
                            rvcone.as_ref(),
                        );
                        let _ = msg_tx.send(BrokerMsg::VrpSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeRetskewSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_retskew_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::RetskewSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeRetkurtSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_retkurt_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::RetkurtSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeTailrSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_tailr_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::TailrSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeRunlenSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_runlen_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::RunlenSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeDayrangeSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_dayrange_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::DayrangeSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 23 handlers ──
                BrokerCmd::ComputeAutocorSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_autocor_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::AutocorSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeHurstSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_hurst_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::HurstSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeHitrateSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_hitrate_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::HitrateSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeGlasymSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_glasym_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::GlasymSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeVolratioSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_volratio_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::VolratioSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 24 handlers ──
                BrokerCmd::ComputeDrawupSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_drawup_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::DrawupSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeGapstatsSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_gapstats_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::GapstatsSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeVolclusterSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_volcluster_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::VolclusterSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCloseplcSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_closeplc_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::CloseplcSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeMrhlSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_mrhl_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::MrhlSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 25 handlers ──
                BrokerCmd::ComputeDownvolSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_downvol_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::DownvolSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeSharprSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_sharpr_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::SharprSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeEffratioSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_effratio_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::EffratioSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeWickbiasSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_wickbias_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::WickbiasSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeVolofvolSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_volofvol_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::VolofvolSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 26 handlers ──
                BrokerCmd::ComputeCalmarSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_calmar_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::CalmarSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeUlcerSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_ulcer_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::UlcerSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeVarratioSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_varratio_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::VarratioSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeAmihudSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_amihud_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::AmihudSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeJbnormSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_jbnorm_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::JbnormSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 27 handlers ──
                BrokerCmd::ComputeOmegaSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_omega_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::OmegaSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeDfaSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_dfa_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::DfaSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeBurkeSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_burke_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::BurkeSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeMonthseasSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_monthseas_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::MonthseasSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeRollsprdSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_rollsprd_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::RollsprdSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 28 handlers ──
                BrokerCmd::ComputeParkinsonSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_parkinson_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::ParkinsonSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeGkvolSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_gkvol_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::GkvolSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeRsvolSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_rsvol_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::RsvolSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCvarSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cvar_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::CvarSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeDoweffectSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_doweffect_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::DoweffectSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 29 handlers ──
                BrokerCmd::ComputeSterlingSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_sterling_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::SterlingSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeKellyfSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_kellyf_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::KellyfSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeLjungbSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_ljungb_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::LjungbSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeRunstestSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_runstest_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::RunstestSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeZeroretSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_zeroret_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::ZeroretSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 30 handlers ──
                BrokerCmd::ComputePsrSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_psr_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::PsrSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeAdfSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_adf_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::AdfSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeMnkendallSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_mnkendall_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::MnkendallSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeBipowerSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_bipower_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::BipowerSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeDddurSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_dddur_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::DddurSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 31 handlers ──
                BrokerCmd::ComputeHilltailSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_hilltail_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::HilltailSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeArchlmSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_archlm_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::ArchlmSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputePainratioSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_painratio_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::PainratioSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCusumSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cusum_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::CusumSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCfvarSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cfvar_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::CfvarSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 32 handlers ──
                BrokerCmd::ComputeEntropySnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_entropy_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::EntropySnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeRachevSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_rachev_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::RachevSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeGprSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_gpr_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::GprSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputePacfSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_pacf_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::PacfSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeApenSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_apen_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::ApenSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 33 handlers ──
                BrokerCmd::ComputeUprSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_upr_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::UprSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeLevereffSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_levereff_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::LevereffSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeDrawdarSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_drawdar_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::DrawdarSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeVarhalfSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_varhalf_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::VarhalfSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeGiniSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_gini_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::GiniSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 34 handlers ──
                BrokerCmd::ComputeSampenSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_sampen_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::SampenSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputePermenSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_permen_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::PermenSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeRecfactSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_recfact_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::RecfactSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeKpssSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_kpss_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::KpssSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeSpecentSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_specent_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::SpecentSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 35 handlers ──
                BrokerCmd::ComputeRobvolSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_robvol_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::RobvolSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeRenyientSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_renyient_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::RenyientSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeRetquantSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_retquant_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::RetquantSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeMsentSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_msent_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::MsentSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeEwmavolSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_ewmavol_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::EwmavolSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 36 handlers ──
                BrokerCmd::ComputeKsnormSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_ksnorm_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::KsnormSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeAdtestSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_adtest_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::AdtestSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeLmomSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_lmom_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::LmomSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeKylelamSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_kylelam_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::KylelamSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputePeakoverSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_peakover_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::PeakoverSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 37 handlers ──
                BrokerCmd::ComputeHiguchiSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_higuchi_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::HiguchiSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputePickandsSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_pickands_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::PickandsSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeKappa3Snapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_kappa3_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::Kappa3SnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeLyapunovSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_lyapunov_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::LyapunovSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeRankacSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_rankac_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::RankacSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 38 handlers ──
                BrokerCmd::ComputeBnsjumpSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_bnsjump_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::BnsjumpSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputePprootSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_pproot_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::PprootSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeMfdfaSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_mfdfa_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::MfdfaSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeHillksSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_hillks_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::HillksSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeTsiSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_tsi_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::TsiSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 39 handlers ──
                BrokerCmd::ComputeGarch11Snapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_garch11_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::Garch11SnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeSadfSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_sadf_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::SadfSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCordimSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cordim_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::CordimSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeSkspecSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_skspec_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::SkspecSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeAutomiSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_automi_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::AutomiSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 40 handlers ──
                BrokerCmd::ComputeDurbinWatsonSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_durbinwatson_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::DurbinWatsonSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeBdsTestSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_bdstest_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::BdsTestSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeBreuschPaganSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_breuschpagan_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::BreuschPaganSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeTurnPtsSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_turnpts_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::TurnPtsSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputePeriodogramSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_periodogram_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::PeriodogramSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeMcLeodLiSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_mcleodli_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::McLeodLiSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeOuFitSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_oufit_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::OuFitSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeGphSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_gph_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::GphSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeBurgSpecSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_burgspec_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::BurgSpecSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeKendallTauSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_kendalltau_snapshot(&symbol, &today, &bars);
                        let _ = msg_tx.send(BrokerMsg::KendallTauSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 42 handlers ──
                BrokerCmd::ComputeSqueezeSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let (bars, si, iv, rv) = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let bars = research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default();
                                let si = research::get_short_interest(&conn, &symbol).ok().flatten();
                                let iv = research::get_ivol(&conn, &symbol).ok().flatten();
                                let rv = research::get_relvol(&conn, &symbol).ok().flatten();
                                (bars, si, iv, rv)
                            } else { (Vec::new(), None, None, None) }
                        } else { (Vec::new(), None, None, None) };
                        let snap = research::compute_squeeze_snapshot(
                            &symbol, &today, &bars,
                            si.as_ref(), iv.as_ref(), rv.as_ref(),
                        );
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_squeeze(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::SqueezeSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeSqueezeRankSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let (subject, all) = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let s = research::get_squeeze(&conn, &symbol).ok().flatten();
                                let a = research::get_all_squeeze(&conn).unwrap_or_default();
                                (s, a)
                            } else { (None, Vec::new()) }
                        } else { (None, Vec::new()) };
                        let snap = research::compute_squeezerank_snapshot(&symbol, &today, subject.as_ref(), &all);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_squeezerank(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::SqueezeRankSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::RefreshSqueezeWatchlist => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let mut rows: Vec<research::SqueezeSnapshot> = Vec::new();
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                // Recompute SQUEEZE for every symbol that has both historical prices
                                // and any of (short_interest, ivol, relvol). We walk the SHORT_INTEREST
                                // table as the source-of-truth set since that is the strongest predicate.
                                let syms = research::get_all_short_interest_symbols(&conn).unwrap_or_default();
                                for sym in syms {
                                    let bars = research::get_historical_price(&conn, &sym).ok().flatten().unwrap_or_default();
                                    if bars.is_empty() { continue; }
                                    let si = research::get_short_interest(&conn, &sym).ok().flatten();
                                    let iv = research::get_ivol(&conn, &sym).ok().flatten();
                                    let rv = research::get_relvol(&conn, &sym).ok().flatten();
                                    let snap = research::compute_squeeze_snapshot(
                                        &sym, &today, &bars,
                                        si.as_ref(), iv.as_ref(), rv.as_ref(),
                                    );
                                    let _ = research::upsert_squeeze(&conn, &sym, &snap);
                                    rows.push(snap);
                                }
                                // Now populate SQUEEZERANK across the full set we just computed.
                                let all = rows.clone();
                                for s in &all {
                                    if s.squeeze_label == "INSUFFICIENT_DATA" { continue; }
                                    let rsnap = research::compute_squeezerank_snapshot(&s.symbol, &today, Some(s), &all);
                                    let _ = research::upsert_squeezerank(&conn, &s.symbol, &rsnap);
                                }
                            }
                        }
                        // Sort by composite desc for UI.
                        rows.sort_by(|a, b| b.composite_score.partial_cmp(&a.composite_score).unwrap_or(std::cmp::Ordering::Equal));
                        let _ = msg_tx.send(BrokerMsg::SqueezeWatchlistLoaded(rows));
                    });
                }
                BrokerCmd::ComputeBbsqueezeSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_bbsqueeze_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_bbsqueeze(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::BbsqueezeSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeDonchianSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_donchian_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_donchian(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::DonchianSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeKamaSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_kama_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_kama(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::KamaSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeIchimokuSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_ichimoku_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_ichimoku(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::IchimokuSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeSupertrendSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_supertrend_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_supertrend(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::SupertrendSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeKeltnerSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_keltner_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_keltner(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::KeltnerSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeFisherSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_fisher_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_fisher(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::FisherSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeAroonSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_aroon_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_aroon(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::AroonSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 44 handlers ──
                BrokerCmd::ComputeAdxSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_adx_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_adx(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::AdxSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCciSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cci_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cci(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CciSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCmfSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cmf_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cmf(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CmfSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeMfiSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_mfi_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_mfi(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::MfiSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputePsarSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_psar_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_psar(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::PsarSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 45 handlers ──
                BrokerCmd::ComputeVortexSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_vortex_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_vortex(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::VortexSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeChopSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_chop_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_chop(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::ChopSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeObvSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_obv_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_obv(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::ObvSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeTrixSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_trix_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_trix(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::TrixSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeHmaSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_hma_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_hma(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::HmaSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 46 handlers ──
                BrokerCmd::ComputePpoSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_ppo_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_ppo(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::PpoSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeDpoSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_dpo_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_dpo(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::DpoSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeKstSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_kst_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_kst(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::KstSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeUltoscSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_ultosc_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_ultosc(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::UltoscSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeWillrSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_willr_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_willr(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::WillrSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 47 handlers ──
                BrokerCmd::ComputeMassSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_mass_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_mass(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::MassSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeChaikoscSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_chaikosc_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_chaikosc(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::ChaikoscSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeKlingerSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_klinger_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_klinger(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::KlingerSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeStochRsiSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_stochrsi_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_stochrsi(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::StochRsiSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeAwesomeSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_awesome_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_awesome(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::AwesomeSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 48 handlers ──
                BrokerCmd::ComputeEfiSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_efi_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_efi(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::EfiSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeEmvSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_emv_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_emv(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::EmvSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeNviSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_nvi_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_nvi(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::NviSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputePviSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_pvi_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_pvi(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::PviSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCoppockSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_coppock_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_coppock(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CoppockSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCmoSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cmo_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cmo(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CmoSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeQstickSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_qstick_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_qstick(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::QstickSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeDisparitySnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_disparity_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_disparity(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::DisparitySnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeBopSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_bop_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_bop(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::BopSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeSchaffSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_schaff_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_schaff(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::SchaffSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeStochSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_stoch_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_stoch(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::StochSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeMacdSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_macd_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_macd(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::MacdSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeVwapSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_vwap_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_vwap(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::VwapSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeMcgdSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_mcgd_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_mcgd(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::McgdSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeRwiSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_rwi_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_rwi(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::RwiSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 51 compute handlers ──
                BrokerCmd::ComputeDemaSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_dema_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_dema(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::DemaSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeTemaSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_tema_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_tema(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::TemaSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeLinregSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_linreg_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_linreg(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::LinregSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputePivotsSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_pivots_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_pivots(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::PivotsSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeHeikinSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_heikin_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_heikin(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::HeikinSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 52 compute handlers ──
                BrokerCmd::ComputeAlmaSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_alma_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_alma(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::AlmaSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeZlemaSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_zlema_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_zlema(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::ZlemaSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeElderRaySnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_elderray_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_elderray(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::ElderRaySnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeTsfSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_tsf_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_tsf(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::TsfSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeRviSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_rvi_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_rvi(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::RviSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeTrimaSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_trima_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_trima(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::TrimaSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeT3Snapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_t3_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_t3(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::T3SnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeVidyaSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_vidya_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_vidya(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::VidyaSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeSmiSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_smi_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_smi(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::SmiSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputePvtSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_pvt_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_pvt(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::PvtSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeAcSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_ac_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_ac(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::AcSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeChvolSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_chvol_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_chvol(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::ChvolSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeBbwidthSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_bbwidth_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_bbwidth(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::BbwidthSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeElderImpSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_elder_impulse_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_elderimp(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::ElderImpSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeRmiSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_rmi_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_rmi(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::RmiSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeSymbolExpirations { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let snap = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let computed = research::compute_symbol_expirations(&conn, &symbol)
                                    .unwrap_or_default();
                                let _ = research::upsert_symbol_expirations(&conn, &symbol, &computed);
                                computed
                            } else { Default::default() }
                        } else { Default::default() };
                        let _ = msg_tx.send(BrokerMsg::SymbolExpirationsMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeSmmaSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_smma_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_smma(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::SmmaSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeAlligatorSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_alligator_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_alligator(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::AlligatorSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCrsiSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_crsi_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_crsi(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CrsiSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeSebSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_seb_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_seb(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::SebSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeImiSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_imi_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_imi(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::ImiSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeGmmaSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_gmma_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_gmma(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::GmmaSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeMaenvSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_maenv_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_maenv(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::MaenvSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeAdlSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_adl_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_adl(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::AdlSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeVhfSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_vhf_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_vhf(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::VhfSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeVrocSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_vroc_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_vroc(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::VrocSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeKdjSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_kdj_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_kdj(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::KdjSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeQqeSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_qqe_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_qqe(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::QqeSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputePmoSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_pmo_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_pmo(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::PmoSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCfoSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cfo_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cfo(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CfoSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeTmfSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_tmf_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_tmf(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::TmfSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeFractalsSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_fractals_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_fractals(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::FractalsSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeIftRsiSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_ift_rsi_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_ift_rsi(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::IftRsiSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeMamaSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_mama_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_mama(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::MamaSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCogSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cog_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cog(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CogSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeDidiSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_didi_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_didi(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::DidiSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeDemarkerSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_demarker_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_demarker(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::DemarkerSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeGatorSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_gator_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_gator(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::GatorSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeBwMfiSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_bw_mfi_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_bw_mfi(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::BwMfiSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeVwmaSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_vwma_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_vwma(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::VwmaSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeStddevSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_stddev_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_stddev(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::StddevSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeWmaSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_wma_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_wma(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::WmaSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeRainbowSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_rainbow_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_rainbow(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::RainbowSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeMesaSineSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_mesa_sine_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_mesa_sine(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::MesaSineSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeFramaSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_frama_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_frama(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::FramaSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeIbsSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_ibs_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_ibs(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::IbsSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeLaguerreRsiSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_laguerre_rsi_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_laguerre_rsi(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::LaguerreRsiSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeZigzagSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_zigzag_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_zigzag(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::ZigzagSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputePgoSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_pgo_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_pgo(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::PgoSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeHtTrendlineSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_ht_trendline_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_ht_trendline(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::HtTrendlineSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeMidpointSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_midpoint_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_midpoint(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::MidpointSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 62 handlers ──
                BrokerCmd::ComputeMassIndexSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_mass_index_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_mass_index(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::MassIndexSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeNatrSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_natr_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_natr(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::NatrSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeTtmSqueezeSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_ttm_squeeze_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_ttm_squeeze(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::TtmSqueezeSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeForceIndexSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_force_index_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_force_index(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::ForceIndexSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeTrangeSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_trange_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_trange(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::TrangeSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 63 handlers ──
                BrokerCmd::ComputeLinearregSlopeSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_linearreg_slope_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_linearreg_slope(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::LinearregSlopeSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeHtDcperiodSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_ht_dcperiod_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_ht_dcperiod(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::HtDcperiodSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeHtTrendmodeSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_ht_trendmode_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_ht_trendmode(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::HtTrendmodeSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeAccbandsSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_accbands_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_accbands(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::AccbandsSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeStochfSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_stochf_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_stochf(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::StochfSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 64 handlers ──
                BrokerCmd::ComputeLinearregSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_linearreg_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_linearreg(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::LinearregSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeLinearregAngleSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_linearreg_angle_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_linearreg_angle(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::LinearregAngleSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeHtDcphaseSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_ht_dcphase_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_ht_dcphase(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::HtDcphaseSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeHtSineSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_ht_sine_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_ht_sine(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::HtSineSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeHtPhasorSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_ht_phasor_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_ht_phasor(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::HtPhasorSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 65 handlers ──
                BrokerCmd::ComputeMidpriceSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_midprice_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_midprice(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::MidpriceSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeApoSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_apo_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_apo(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::ApoSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeMomSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_mom_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_mom(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::MomSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeSarextSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_sarext_snapshot(
                            &symbol, &today, &bars,
                            0.0, 0.02, 0.02, 0.20, 0.02, 0.02, 0.20,
                        );
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_sarext(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::SarextSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeAdxrSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_adxr_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_adxr(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::AdxrSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 66: AVGPRICE / MEDPRICE / TYPPRICE / WCLPRICE / VARIANCE ──
                BrokerCmd::ComputeAvgpriceSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_avgprice_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_avgprice(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::AvgpriceSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeMedpriceSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_medprice_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_medprice(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::MedpriceSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeTypPriceSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_typprice_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_typprice(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::TypPriceSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeWclPriceSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_wclprice_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_wclprice(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::WclPriceSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeVarianceSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_variance_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_variance(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::VarianceSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 67: PLUS_DI / MINUS_DI / PLUS_DM / MINUS_DM / DX ──
                BrokerCmd::ComputePlusDiSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_plus_di_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_plus_di(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::PlusDiSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeMinusDiSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_minus_di_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_minus_di(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::MinusDiSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputePlusDmSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_plus_dm_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_plus_dm(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::PlusDmSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeMinusDmSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_minus_dm_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_minus_dm(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::MinusDmSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeDxSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_dx_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_dx(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::DxSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 68 handlers ──
                BrokerCmd::ComputeRocSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_roc_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_roc(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::RocSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeRocpSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_rocp_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_rocp(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::RocpSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeRocrSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_rocr_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_rocr(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::RocrSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeRocr100Snapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_rocr100_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_rocr100(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::Rocr100SnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCorrelSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_correl_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_correl(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CorrelSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 69 handlers ──
                BrokerCmd::ComputeMinSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_min_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_min(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::MinSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeMaxSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_max_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_max(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::MaxSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeMinMaxSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_minmax_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_minmax(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::MinMaxSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeMinIndexSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_minindex_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_minindex(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::MinIndexSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeMaxIndexSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_maxindex_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_maxindex(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::MaxIndexSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 70 broker handlers ──
                BrokerCmd::ComputeBbandsSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_bbands_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_bbands(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::BbandsSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeAdSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_ad_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_ad(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::AdSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeAdoscSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_adosc_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_adosc(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::AdoscSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeSumSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_sum_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_sum(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::SumSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeLinearRegInterceptSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_linearreg_intercept_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_linreg_intercept(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::LinearRegInterceptSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 71 handlers ──
                BrokerCmd::ComputeAroonoscSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_aroonosc_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_aroonosc(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::AroonoscSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeMinMaxIndexSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_minmaxindex_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_minmaxindex(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::MinMaxIndexSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeMacdextSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_macdext_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_macdext(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::MacdextSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeMacdfixSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_macdfix_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_macdfix(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::MacdfixSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeMavpSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_mavp_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_mavp(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::MavpSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 72 CDL* handlers ──
                BrokerCmd::ComputeCdlDojiSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_doji_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_doji(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlDojiSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlHammerSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_hammer_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_hammer(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlHammerSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlShootingStarSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_shooting_star_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_shooting_star(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlShootingStarSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlEngulfingSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_engulfing_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_engulfing(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlEngulfingSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlHaramiSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_harami_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_harami(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlHaramiSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 73 handlers — CDL* 3-bar / 2-bar patterns ──
                BrokerCmd::ComputeCdlMorningStarSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_morning_star_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_morning_star(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlMorningStarSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlEveningStarSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_evening_star_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_evening_star(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlEveningStarSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlThreeBlackCrowsSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_three_black_crows_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_three_black_crows(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlThreeBlackCrowsSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlThreeWhiteSoldiersSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_three_white_soldiers_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_three_white_soldiers(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlThreeWhiteSoldiersSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlDarkCloudCoverSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_dark_cloud_cover_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_dark_cloud_cover(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlDarkCloudCoverSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 74 handlers — CDL* piercing / doji variants / hammer mirrors ──
                BrokerCmd::ComputeCdlPiercingSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_piercing_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_piercing(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlPiercingSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlDragonflyDojiSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_dragonfly_doji_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_dragonfly_doji(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlDragonflyDojiSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlGravestoneDojiSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_gravestone_doji_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_gravestone_doji(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlGravestoneDojiSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlHangingManSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_hanging_man_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_hanging_man(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlHangingManSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlInvertedHammerSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_inverted_hammer_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_inverted_hammer(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlInvertedHammerSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 75 handlers — CDL* harami cross / long-legged doji / marubozu / spinning top / tristar ──
                BrokerCmd::ComputeCdlHaramiCrossSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_harami_cross_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_harami_cross(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlHaramiCrossSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlLongLeggedDojiSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_long_legged_doji_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_long_legged_doji(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlLongLeggedDojiSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlMarubozuSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_marubozu_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_marubozu(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlMarubozuSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlSpinningTopSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_spinning_top_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_spinning_top(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlSpinningTopSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlTristarSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_tristar_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_tristar(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlTristarSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 76 handlers — CDL* doji star / morning doji star / evening doji star / abandoned baby / three inside ──
                BrokerCmd::ComputeCdlDojiStarSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_doji_star_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_doji_star(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlDojiStarSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlMorningDojiStarSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_morning_doji_star_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_morning_doji_star(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlMorningDojiStarSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlEveningDojiStarSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_evening_doji_star_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_evening_doji_star(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlEveningDojiStarSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlAbandonedBabySnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_abandoned_baby_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_abandoned_baby(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlAbandonedBabySnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlThreeInsideSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_three_inside_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_three_inside(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlThreeInsideSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 77 handlers — CDL* belt hold / closing marubozu / high wave / long line / short line ──
                BrokerCmd::ComputeCdlBeltHoldSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_belt_hold_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_belt_hold(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlBeltHoldSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlClosingMarubozuSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_closing_marubozu_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_closing_marubozu(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlClosingMarubozuSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlHighWaveSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_high_wave_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_high_wave(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlHighWaveSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlLongLineSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_long_line_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_long_line(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlLongLineSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlShortLineSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_short_line_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_short_line(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlShortLineSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 78 handlers — CDL* counterattack / homing pigeon / in-neck / on-neck / thrusting ──
                BrokerCmd::ComputeCdlCounterattackSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_counterattack_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_counterattack(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlCounterattackSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlHomingPigeonSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_homing_pigeon_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_homing_pigeon(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlHomingPigeonSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlInNeckSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_in_neck_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_in_neck(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlInNeckSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlOnNeckSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_on_neck_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_on_neck(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlOnNeckSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlThrustingSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_thrusting_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_thrusting(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlThrustingSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 79/80 handlers — additional CDL* parity windows ──
                BrokerCmd::ComputeCdlTwoCrowsSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_two_crows_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_two_crows(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlTwoCrowsSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlThreeLineStrikeSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_three_line_strike_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_three_line_strike(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlThreeLineStrikeSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlThreeOutsideSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_three_outside_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_three_outside(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlThreeOutsideSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlMatchingLowSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_matching_low_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_matching_low(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlMatchingLowSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlSeparatingLinesSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_separating_lines_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_separating_lines(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlSeparatingLinesSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlStickSandwichSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_stick_sandwich_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_stick_sandwich(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlStickSandwichSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlRickshawManSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_rickshaw_man_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_rickshaw_man(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlRickshawManSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlTakuriSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_takuri_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_takuri(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlTakuriSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 81/82 handlers — harder CDL* parity windows ──
                BrokerCmd::ComputeCdlThreeStarsInSouthSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) =
                            shared_cache_broker.read().ok().and_then(|g| g.clone())
                        {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol)
                                    .ok()
                                    .flatten()
                                    .unwrap_or_default()
                            } else {
                                Vec::new()
                            }
                        } else {
                            Vec::new()
                        };
                        let snap =
                            research::compute_cdl_three_stars_in_south_snapshot(
                                &symbol, &today, &bars,
                            );
                        if let Some(cache) =
                            shared_cache_broker.read().ok().and_then(|g| g.clone())
                        {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_three_stars_in_south(
                                    &conn, &symbol, &snap,
                                );
                            }
                        }
                        let _ = msg_tx.send(
                            BrokerMsg::CdlThreeStarsInSouthSnapshotMsg(symbol, snap),
                        );
                    });
                }
                BrokerCmd::ComputeCdlIdenticalThreeCrowsSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) =
                            shared_cache_broker.read().ok().and_then(|g| g.clone())
                        {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol)
                                    .ok()
                                    .flatten()
                                    .unwrap_or_default()
                            } else {
                                Vec::new()
                            }
                        } else {
                            Vec::new()
                        };
                        let snap =
                            research::compute_cdl_identical_three_crows_snapshot(
                                &symbol, &today, &bars,
                            );
                        if let Some(cache) =
                            shared_cache_broker.read().ok().and_then(|g| g.clone())
                        {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_identical_three_crows(
                                    &conn, &symbol, &snap,
                                );
                            }
                        }
                        let _ = msg_tx.send(
                            BrokerMsg::CdlIdenticalThreeCrowsSnapshotMsg(symbol, snap),
                        );
                    });
                }
                BrokerCmd::ComputeCdlKickingSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) =
                            shared_cache_broker.read().ok().and_then(|g| g.clone())
                        {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol)
                                    .ok()
                                    .flatten()
                                    .unwrap_or_default()
                            } else {
                                Vec::new()
                            }
                        } else {
                            Vec::new()
                        };
                        let snap = research::compute_cdl_kicking_snapshot(
                            &symbol, &today, &bars,
                        );
                        if let Some(cache) =
                            shared_cache_broker.read().ok().and_then(|g| g.clone())
                        {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_kicking(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlKickingSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlKickingByLengthSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) =
                            shared_cache_broker.read().ok().and_then(|g| g.clone())
                        {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol)
                                    .ok()
                                    .flatten()
                                    .unwrap_or_default()
                            } else {
                                Vec::new()
                            }
                        } else {
                            Vec::new()
                        };
                        let snap =
                            research::compute_cdl_kicking_by_length_snapshot(
                                &symbol, &today, &bars,
                            );
                        if let Some(cache) =
                            shared_cache_broker.read().ok().and_then(|g| g.clone())
                        {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_kicking_by_length(
                                    &conn, &symbol, &snap,
                                );
                            }
                        }
                        let _ = msg_tx.send(
                            BrokerMsg::CdlKickingByLengthSnapshotMsg(symbol, snap),
                        );
                    });
                }
                BrokerCmd::ComputeCdlLadderBottomSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) =
                            shared_cache_broker.read().ok().and_then(|g| g.clone())
                        {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol)
                                    .ok()
                                    .flatten()
                                    .unwrap_or_default()
                            } else {
                                Vec::new()
                            }
                        } else {
                            Vec::new()
                        };
                        let snap = research::compute_cdl_ladder_bottom_snapshot(
                            &symbol, &today, &bars,
                        );
                        if let Some(cache) =
                            shared_cache_broker.read().ok().and_then(|g| g.clone())
                        {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_ladder_bottom(
                                    &conn, &symbol, &snap,
                                );
                            }
                        }
                        let _ = msg_tx.send(
                            BrokerMsg::CdlLadderBottomSnapshotMsg(symbol, snap),
                        );
                    });
                }
                BrokerCmd::ComputeCdlUniqueThreeRiverSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) =
                            shared_cache_broker.read().ok().and_then(|g| g.clone())
                        {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol)
                                    .ok()
                                    .flatten()
                                    .unwrap_or_default()
                            } else {
                                Vec::new()
                            }
                        } else {
                            Vec::new()
                        };
                        let snap =
                            research::compute_cdl_unique_three_river_snapshot(
                                &symbol, &today, &bars,
                            );
                        if let Some(cache) =
                            shared_cache_broker.read().ok().and_then(|g| g.clone())
                        {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_unique_three_river(
                                    &conn, &symbol, &snap,
                                );
                            }
                        }
                        let _ = msg_tx.send(
                            BrokerMsg::CdlUniqueThreeRiverSnapshotMsg(symbol, snap),
                        );
                    });
                }
                // ── Round 83/84 handlers — additional multi-bar CDL* parity windows ──
                BrokerCmd::ComputeCdlAdvanceBlockSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) =
                            shared_cache_broker.read().ok().and_then(|g| g.clone())
                        {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol)
                                    .ok()
                                    .flatten()
                                    .unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_advance_block_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_advance_block(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlAdvanceBlockSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlBreakawaySnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) =
                            shared_cache_broker.read().ok().and_then(|g| g.clone())
                        {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol)
                                    .ok()
                                    .flatten()
                                    .unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_breakaway_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_breakaway(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlBreakawaySnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlGapSideSideWhiteSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) =
                            shared_cache_broker.read().ok().and_then(|g| g.clone())
                        {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol)
                                    .ok()
                                    .flatten()
                                    .unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_gap_side_side_white_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_gap_side_side_white(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlGapSideSideWhiteSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlUpsideGapTwoCrowsSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) =
                            shared_cache_broker.read().ok().and_then(|g| g.clone())
                        {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol)
                                    .ok()
                                    .flatten()
                                    .unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_upside_gap_two_crows_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_upside_gap_two_crows(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlUpsideGapTwoCrowsSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlXSideGapThreeMethodsSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) =
                            shared_cache_broker.read().ok().and_then(|g| g.clone())
                        {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol)
                                    .ok()
                                    .flatten()
                                    .unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_xside_gap_three_methods_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_xside_gap_three_methods(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlXSideGapThreeMethodsSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlConcealBabySwallowSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) =
                            shared_cache_broker.read().ok().and_then(|g| g.clone())
                        {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol)
                                    .ok()
                                    .flatten()
                                    .unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_conceal_baby_swallow_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_conceal_baby_swallow(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlConcealBabySwallowSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 85/86 handlers — stateful CDL* parity windows ──
                BrokerCmd::ComputeCdlHikkakeSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) =
                            shared_cache_broker.read().ok().and_then(|g| g.clone())
                        {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol)
                                    .ok()
                                    .flatten()
                                    .unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_hikkake_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_hikkake(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlHikkakeSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlHikkakeModSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) =
                            shared_cache_broker.read().ok().and_then(|g| g.clone())
                        {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol)
                                    .ok()
                                    .flatten()
                                    .unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_hikkake_mod_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_hikkake_mod(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlHikkakeModSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlMatHoldSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) =
                            shared_cache_broker.read().ok().and_then(|g| g.clone())
                        {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol)
                                    .ok()
                                    .flatten()
                                    .unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_mat_hold_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_mat_hold(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlMatHoldSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlRiseFallThreeMethodsSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) =
                            shared_cache_broker.read().ok().and_then(|g| g.clone())
                        {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol)
                                    .ok()
                                    .flatten()
                                    .unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_rise_fall_three_methods_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_rise_fall_three_methods(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlRiseFallThreeMethodsSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 87/88 handlers — final CDL* parity windows ──
                BrokerCmd::ComputeCdlStalledPatternSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) =
                            shared_cache_broker.read().ok().and_then(|g| g.clone())
                        {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol)
                                    .ok()
                                    .flatten()
                                    .unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_stalled_pattern_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_stalled_pattern(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlStalledPatternSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeCdlTasukiGapSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) =
                            shared_cache_broker.read().ok().and_then(|g| g.clone())
                        {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol)
                                    .ok()
                                    .flatten()
                                    .unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_cdl_tasuki_gap_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_cdl_tasuki_gap(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::CdlTasukiGapSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 76 (Quant Stats) handlers ──
                BrokerCmd::ComputeModSharpeSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_modsharpe_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_modsharpe(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::ModSharpeSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeHsiehTestSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_hsieh_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_hsiehtest(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::HsiehTestSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeChowBreakSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_chowbreak_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_chowbreak(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::ChowBreakSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeDriftBurstSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_driftburst_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_driftburst(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::DriftBurstSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeHlvClustSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_hlvclust_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_hlvclust(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::HlvClustSnapshotMsg(symbol, snap));
                    });
                }
                // ── Round 77 (Quant Stats) handlers ──
                BrokerCmd::ComputeYangZhangSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_yangzhang_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_yangzhang(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::YangZhangSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeKuiperSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_kuiper_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_kuiper(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::KuiperSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeDagostinoSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_dagostino_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_dagostino(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::DagostinoSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeBaiPerronSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_baiperron_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_baiperron(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::BaiPerronSnapshotMsg(symbol, snap));
                    });
                }
                BrokerCmd::ComputeKupiecPofSnapshot { symbol } => {
                    use typhoon_engine::core::research;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let bars = if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                research::get_historical_price(&conn, &symbol).ok().flatten().unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() };
                        let snap = research::compute_kupiecpof_snapshot(&symbol, &today, &bars);
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                let _ = research::upsert_kupiecpof(&conn, &symbol, &snap);
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::KupiecPofSnapshotMsg(symbol, snap));
                    });
                }
                // ── web article ingestion handler ──
                BrokerCmd::IngestResearchArticles { text, agent_override } => {
                    use typhoon_engine::core::{news, research};
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let parsed = research::parse_ingest_block(&text);
                        let mut per_symbol: Vec<(String, usize, usize)> = Vec::new();
                        let mut errors: Vec<String> = Vec::new();
                        if parsed.is_empty() {
                            errors.push("No ===TYPHOON_INGEST=== block found in the pasted text.".into());
                            let _ = msg_tx.send(BrokerMsg::IngestResearchResult { per_symbol_added: per_symbol, errors });
                            return;
                        }
                        let cache_opt = shared_cache_broker.read().ok().and_then(|g| g.clone());
                        let conn = match cache_opt.as_ref().and_then(|c| c.connection().ok()) {
                            Some(c) => c,
                            None => {
                                errors.push("Cache unavailable — cannot persist ingested articles.".into());
                                let _ = msg_tx.send(BrokerMsg::IngestResearchResult { per_symbol_added: per_symbol, errors });
                                return;
                            }
                        };
                        for (sym, mut articles) in parsed {
                            if !agent_override.trim().is_empty() {
                                for a in articles.iter_mut() {
                                    if a.agent_used.trim().is_empty() {
                                        a.agent_used = agent_override.clone();
                                    }
                                }
                            }
                            // Clone before moving into append: we also promote these into
                            // research_news so the NEWS window sees AI-ingested articles
                            // (otherwise they sit only in research_web_articles where the
                            // NEWS query never looks).
                            let news_articles = articles.clone();
                            match research::append_ingested_articles(&conn, &sym, articles) {
                                Ok((added, total)) => {
                                    per_symbol.push((sym.clone(), added, total));
                                    for wa in news_articles.into_iter() {
                                        if wa.url.trim().is_empty() { continue; }
                                        let art = news::NewsArticle {
                                            url_hash: String::new(),
                                            symbol: sym.clone(),
                                            source: if wa.agent_used.trim().is_empty() {
                                                "Ingested".into()
                                            } else {
                                                format!("Ingested/{}", wa.agent_used)
                                            },
                                            provider: wa.source,
                                            headline: wa.title,
                                            summary: wa.summary,
                                            url: wa.url,
                                            published_at: news::parse_iso_ts(&wa.published_at),
                                            image_url: String::new(),
                                            sentiment: String::new(),
                                            sentiment_score: 0.0,
                                            tickers: vec![sym.clone()],
                                            categories: vec![],
                                            body: String::new(),
                                            body_fetch_attempts: 0,
                                        };
                                        if let Err(e) = news::upsert_news(&conn, &art) {
                                            tracing::warn!("ingest news upsert {}: {}", sym, e);
                                        }
                                    }
                                }
                                Err(e) => errors.push(format!("{}: {}", sym, e)),
                            }
                        }
                        let _ = msg_tx.send(BrokerMsg::IngestResearchResult { per_symbol_added: per_symbol, errors });
                    });
                }
                BrokerCmd::FetchNewsMulti { symbol, marketaux_key, alpha_vantage_key, fmp_key, finnhub_key, cryptopanic_key } => {
                    use typhoon_engine::core::news;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            if let Ok(conn) = cache.connection() {
                                if news::news_cache_is_fresh(&conn, &symbol, 30 * 60, 1)
                                    .unwrap_or(false)
                                {
                                    let _ = msg_tx.send(BrokerMsg::FundamentalsProgress(format!(
                                        "news/{}: cached/fresh — skipped network",
                                        symbol
                                    )));
                                    match news::get_news_by_symbol(&conn, &symbol, 200) {
                                        Ok(list) => {
                                            let _ = msg_tx.send(BrokerMsg::NewsArticlesLoaded {
                                                symbol: symbol.clone(),
                                                articles: list,
                                            });
                                        }
                                        Err(e) => {
                                            let _ = msg_tx.send(BrokerMsg::Error(format!(
                                                "News read: {e}"
                                            )));
                                        }
                                    }
                                    return;
                                }
                            }
                        }
                        let client = match reqwest::Client::builder()
                            .user_agent("Mozilla/5.0 (X11; Linux x86_64) TyphooN-Terminal/0.1")
                            .timeout(std::time::Duration::from_secs(25))
                            .build() {
                            Ok(c) => c,
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("News client: {e}"))); return; }
                        };
                        let tx_log = msg_tx.clone();
                        let cb = move |s: &str| {
                            let _ = tx_log.send(BrokerMsg::FundamentalsProgress(s.to_string()));
                        };
                        let news_keys = news::NewsApiKeys {
                            marketaux: marketaux_key,
                            alpha_vantage: alpha_vantage_key,
                            fmp: fmp_key,
                            finnhub: finnhub_key,
                            cryptopanic: cryptopanic_key,
                        };
                        let articles = match news::fetch_all_sources_for_symbol(
                            &client, &symbol,
                            &news_keys,
                            cb,
                        ).await {
                            Ok(v) => v,
                            Err(e) => {
                                let _ = msg_tx.send(BrokerMsg::Error(format!("News fetch {}: {e}", symbol)));
                                return;
                            }
                        };
                        // DB work must run off the tokio worker to avoid holding &Connection across await.
                        let sym_for_db = symbol.clone();
                        let msg_tx_db = msg_tx.clone();
                        let shared_cache_for_first = shared_cache_broker.clone();
                        let _ = tokio::task::spawn_blocking(move || {
                            let Some(cache) = shared_cache_for_first.read().ok().and_then(|g| g.clone()) else {
                                let _ = msg_tx_db.send(BrokerMsg::Error("News: cache not ready".into()));
                                return;
                            };
                            let Ok(conn) = cache.connection() else {
                                let _ = msg_tx_db.send(BrokerMsg::Error("News: conn failed".into()));
                                return;
                            };
                            // Deduplicate: (function article_exists_by_url_hash not yet implemented)
                            // For now we pass all articles; dedup will be added when the helper exists.
                            match news::upsert_news_batch(&conn, &articles) {
                                Ok(n) => {
                                    let cached = news::mark_news_scraped(&conn, &sym_for_db)
                                        .unwrap_or(n);
                                    let _ = msg_tx_db.send(BrokerMsg::FundamentalsProgress(
                                        format!("news/{}: {} cached (deduped)", sym_for_db, cached)));
                                }
                                Err(e) => {
                                    let _ = msg_tx_db.send(BrokerMsg::Error(format!("News upsert: {e}")));
                                    return;
                                }
                            }
                            match news::get_news_by_symbol(&conn, &sym_for_db, 200) {
                                Ok(list) => { let _ = msg_tx_db.send(BrokerMsg::NewsArticlesLoaded { symbol: sym_for_db, articles: list }); }
                                Err(e) => { let _ = msg_tx_db.send(BrokerMsg::Error(format!("News read: {e}"))); }
                            }
                        }).await;
                        // Foreground hydrate the bodies for this symbol so they
                        // arrive in the cache while the user is still scanning
                        // the headline list. The first NewsArticlesLoaded above
                        // landed with whatever bodies were already cached;
                        // re-send after hydration so the UI swaps placeholders
                        // for real article text without waiting for the next
                        // background tick.
                        let sym_for_hydrate = symbol.clone();
                        let msg_tx_hydrate = msg_tx.clone();
                        let shared_cache_hydrate = shared_cache_broker.clone();
                        tokio::spawn(async move {
                            let Some(cache) = shared_cache_hydrate.read().ok().and_then(|g| g.clone()) else {
                                return;
                            };
                            let written = news_ingest::hydrate_missing_bodies(
                                cache.clone(),
                                Some(sym_for_hydrate.clone()),
                            )
                            .await;
                            if written == 0 {
                                return;
                            }
                            let _ = tokio::task::spawn_blocking(move || {
                                let Ok(conn) = cache.connection() else { return; };
                                if let Ok(list) =
                                    typhoon_engine::core::news::get_news_by_symbol(
                                        &conn,
                                        &sym_for_hydrate,
                                        200,
                                    )
                                {
                                    let _ = msg_tx_hydrate.send(BrokerMsg::NewsArticlesLoaded {
                                        symbol: sym_for_hydrate,
                                        articles: list,
                                    });
                                }
                            })
                            .await;
                        });
                    });
                }
                BrokerCmd::LoadCachedNews { symbol, limit } => {
                    use typhoon_engine::core::news;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::task::spawn_blocking(move || {
                        let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) else {
                            let _ = msg_tx.send(BrokerMsg::Error("News cache: not ready".into()));
                            return;
                        };
                        let Ok(conn) = cache.connection() else {
                            let _ = msg_tx.send(BrokerMsg::Error("News cache: connection failed".into()));
                            return;
                        };
                        match news::get_news_by_symbol(&conn, &symbol, limit) {
                            Ok(list) => { let _ = msg_tx.send(BrokerMsg::NewsArticlesLoaded { symbol, articles: list }); }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("Cached news read: {e}"))); }
                        }
                    });
                }
                BrokerCmd::HydrateNewsArticle { symbol, url_hash, url } => {
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) else {
                            return;
                        };
                        let written = news_ingest::hydrate_one_url(
                            cache.clone(),
                            url_hash,
                            url,
                        )
                        .await;
                        // Always refresh the symbol's article list — even
                        // a failure bumps body_fetch_attempts, which the
                        // UI uses to decide whether to keep the "still
                        // hydrating" placeholder or switch to "body
                        // unavailable". A re-read keeps the placeholder
                        // state in sync with the counter.
                        let _ = written;
                        let _ = tokio::task::spawn_blocking(move || {
                            let Ok(conn) = cache.connection() else { return; };
                            if let Ok(list) =
                                typhoon_engine::core::news::get_news_by_symbol(
                                    &conn, &symbol, 200,
                                )
                            {
                                let _ = msg_tx.send(BrokerMsg::NewsArticlesLoaded {
                                    symbol,
                                    articles: list,
                                });
                            }
                        })
                        .await;
                    });
                }
                BrokerCmd::SearchNews { query, limit } => {
                    use typhoon_engine::core::news;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::task::spawn_blocking(move || {
                        let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) else {
                            let _ = msg_tx.send(BrokerMsg::Error("News cache: not ready".into()));
                            return;
                        };
                        let Ok(conn) = cache.connection() else {
                            let _ = msg_tx.send(BrokerMsg::Error("News cache: connection failed".into()));
                            return;
                        };
                        match news::search_news(&conn, &query, limit) {
                            Ok(list) => { let _ = msg_tx.send(BrokerMsg::NewsArticlesLoaded { symbol: String::new(), articles: list }); }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("News search: {e}"))); }
                        }
                    });
                }
                BrokerCmd::NewsScrapeSymbols {
                    symbols,
                    marketaux_key,
                    alpha_vantage_key,
                    fmp_key,
                    finnhub_key,
                    cryptopanic_key,
                } => {
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    let _ = std::thread::Builder::new()
                        .name("typhoon-news-scrape-symbols".into())
                        .spawn(move || {
                            use typhoon_engine::core::news;
                            let rt = tokio::runtime::Builder::new_current_thread()
                                .enable_all()
                                .build()
                                .unwrap_or_else(|e| {
                                    eprintln!("FATAL: tokio runtime init failed: {e}");
                                    std::process::exit(1);
                                });
                            rt.block_on(async {
                                let Some(cache) = shared_cache_broker
                                    .read()
                                    .ok()
                                    .and_then(|g| g.clone())
                                else {
                                    let _ = msg_tx.send(BrokerMsg::Error(
                                        "NewsScrapeSymbols: cache not ready".into(),
                                    ));
                                    return;
                                };
                                let mut tickers: Vec<String> = symbols
                                    .into_iter()
                                    .map(|s| s.trim().to_uppercase())
                                    .filter(|s| !s.is_empty())
                                    .collect::<std::collections::BTreeSet<_>>()
                                    .into_iter()
                                    .collect();
                                tickers.sort();
                                if tickers.is_empty() {
                                    let _ = msg_tx.send(BrokerMsg::FundamentalsProgress(
                                        "News scrape: no symbols".into(),
                                    ));
                                    return;
                                }
                                let _ = msg_tx.send(BrokerMsg::FundamentalsProgress(
                                    format_news_scope_scrape_start(&tickers),
                                ));
                                let fresh_tickers = cache
                                    .connection()
                                    .ok()
                                    .and_then(|conn| {
                                        news::fresh_news_symbols(&conn, &tickers, 30 * 60, 1).ok()
                                    })
                                    .unwrap_or_default();
                                let client = match reqwest::Client::builder()
                                    .user_agent(
                                        "Mozilla/5.0 (X11; Linux x86_64) TyphooN-Terminal/0.1",
                                    )
                                    .timeout(std::time::Duration::from_secs(25))
                                    .build()
                                {
                                    Ok(c) => c,
                                    Err(e) => {
                                        let _ = msg_tx.send(BrokerMsg::Error(format!(
                                            "News client: {e}"
                                        )));
                                        return;
                                    }
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
                                let total = tickers.len();
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
                                            ticker, fetch_key, i + 1, total
                                        )));
                                        continue;
                                    }
                                    processed_keys.insert(fetch_key);

                                    if fresh_tickers.contains(ticker) {
                                        ok += 1;
                                        let _ = msg_tx.send(
                                            BrokerMsg::FundamentalsProgress(format!(
                                                "News {}: cached/fresh — skipped network ({}/{})",
                                                ticker,
                                                i + 1,
                                                total
                                            )),
                                        );
                                        continue;
                                    }
                                    let log_tx = msg_tx.clone();
                                    let cb = move |s: &str| {
                                        let _ = log_tx.send(BrokerMsg::FundamentalsProgress(
                                            s.to_string(),
                                        ));
                                    };
                                    match news::fetch_all_sources_for_symbol(
                                        &client,
                                        ticker,
                                        &news_keys,
                                        cb,
                                    )
                                    .await
                                    {
                                        Ok(articles) => {
                                            if let Ok(conn) = cache.connection() {
                                                match news::upsert_news_batch(&conn, &articles) {
                                                    Ok(n) => {
                                                        let cached = news::mark_news_scraped(&conn, ticker)
                                                            .unwrap_or(n);
                                                        ok += 1;
                                                        let _ = msg_tx.send(
                                                            BrokerMsg::FundamentalsProgress(
                                                                format!(
                                                                    "News {}: {} cached ({}/{})",
                                                                    ticker,
                                                                    cached,
                                                                    i + 1,
                                                                    total
                                                                ),
                                                            ),
                                                        );
                                                    }
                                                    Err(e) => {
                                                        fail += 1;
                                                        let _ = msg_tx.send(
                                                            BrokerMsg::FundamentalsProgress(
                                                                format!(
                                                                    "News {} upsert failed: {e}",
                                                                    ticker
                                                                ),
                                                            ),
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            fail += 1;
                                            let _ = msg_tx.send(BrokerMsg::FundamentalsProgress(
                                                format!("News {} failed: {e}", ticker),
                                            ));
                                        }
                                    }
                                    tokio::time::sleep(std::time::Duration::from_millis(500))
                                        .await;
                                }
                                let _ = msg_tx.send(BrokerMsg::FundamentalsProgress(format!(
                                    "News scrape complete: {} OK, {} failed of {} symbol(s)",
                                    ok, fail, total
                                )));
                                if let Some(first) = tickers.first() {
                                    if let Ok(conn) = cache.connection() {
                                        if let Ok(list) = news::get_news_by_symbol(&conn, first, 200) {
                                            let _ = msg_tx.send(BrokerMsg::NewsArticlesLoaded {
                                                symbol: first.clone(),
                                                articles: list,
                                            });
                                        }
                                    }
                                }
                            });
                        });
                }
                BrokerCmd::NewsScrapeAll {
                    use_mt5, use_alpaca, use_tastytrade, use_kraken,
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
                    if use_tastytrade {
                        if let Some(ref tt) = tt_broker {
                            if let Ok(positions) = tt.get_positions().await {
                                extra_tickers.extend(positions.iter()
                                    .filter(|p| p.instrument_type == "Equity")
                                    .map(|p| p.symbol.clone()));
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
                    if typhoon_engine::core::kraken::to_kraken_pair_lossy(&symbol).is_none() {
                        let _ = msg_tx.send(BrokerMsg::OrderResult(format!(
                            "Kraken orderbook WS skipped: {symbol} is not a Kraken spot pair"
                        )));
                        continue;
                    }
                    let stream_result = if let Some(ref kb) = kraken_broker {
                        kb.start_public_orderbook_ws(&symbol, depth).await
                    } else {
                        let kb = typhoon_engine::broker::kraken::KrakenBroker::new(
                            String::new(),
                            String::new(),
                        );
                        kb.start_public_orderbook_ws(&symbol, depth).await
                    };
                    match stream_result {
                        Ok(mut rx) => {
                            let stream_tx = msg_tx.clone();
                            tokio::spawn(async move {
                                while let Some(update) = rx.recv().await {
                                    let _ = stream_tx.send(BrokerMsg::KrakenOrderbookUpdate(update));
                                }
                            });
                            let _ = msg_tx.send(BrokerMsg::OrderResult(format!(
                                "Kraken orderbook WS started: {symbol} depth {depth}"
                            )));
                        }
                        Err(e) => {
                            let _ = msg_tx.send(BrokerMsg::Error(format!(
                                "Kraken orderbook WS failed: {e}"
                            )));
                        }
                    }
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
                BrokerCmd::DarwinImportAll { dir, db_path: _ } => {
                    // Spawn a dedicated thread so we don't block the broker command loop
                    let msg_tx = broker_msg_tx_clone.clone();
                    let importing = importing_flag.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::task::spawn_blocking(move || {
                        // RAII release so a panic in the XLSX import (openpyxl
                        // row decode, SQL insert, etc.) doesn't leave the flag
                        // stuck true and the background stats worker silently
                        // skipping every cycle until terminal restart.
                        importing.store(true, std::sync::atomic::Ordering::Relaxed);
                        struct ImportingGuard(std::sync::Arc<std::sync::atomic::AtomicBool>);
                        impl Drop for ImportingGuard {
                            fn drop(&mut self) {
                                self.0.store(false, std::sync::atomic::Ordering::Relaxed);
                            }
                        }
                        let _guard = ImportingGuard(importing.clone());
                        let _ = msg_tx.send(BrokerMsg::OrderResult(format!("DARWIN XLSX scan: {}...", dir.display())));
                        match std::fs::read_dir(&dir) {
                            Ok(entries) => {
                                let mut xlsx_files: Vec<std::path::PathBuf> = entries
                                    .filter_map(|e| e.ok())
                                    .map(|e| e.path())
                                    .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("xlsx"))
                                    .collect();
                                xlsx_files.sort();
                                if xlsx_files.is_empty() {
                                    let _ = msg_tx.send(BrokerMsg::Error(format!("No .xlsx files found in {}", dir.display())));
                                } else {
                                    let _ = msg_tx.send(BrokerMsg::OrderResult(format!("Found {} XLSX files", xlsx_files.len())));
                                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                                        if let Ok(conn) = cache.connection() {
                                            let _ = darwin::create_darwin_tables(&conn);
                                            let mut total_deals = 0usize;
                                            let mut total_positions = 0usize;
                                            let mut imported = 0usize;
                                            for path in &xlsx_files {
                                                let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                                                let ticker = stem.split(&['_', '-', ' '][..]).next().unwrap_or(stem).to_uppercase();
                                                if ticker.is_empty() { continue; }
                                                match darwin::import_darwin_xlsx(&conn, &path.display().to_string(), &ticker) {
                                                    Ok((name, deals, positions)) => {
                                                        total_deals += deals;
                                                        total_positions += positions;
                                                        imported += 1;
                                                        let _ = msg_tx.send(BrokerMsg::OrderResult(format!(
                                                            "Imported {}: {} deals, {} positions ({})",
                                                            name, deals, positions, path.file_name().unwrap_or_default().to_string_lossy()
                                                        )));
                                                    }
                                                    Err(e) => {
                                                        let _ = msg_tx.send(BrokerMsg::Error(format!(
                                                            "Import {} failed: {}", path.file_name().unwrap_or_default().to_string_lossy(), e
                                                        )));
                                                    }
                                                }
                                            }
                                            let _ = msg_tx.send(BrokerMsg::OrderResult(format!(
                                                "DARWIN import complete: {}/{} files, {} deals, {} positions",
                                                imported, xlsx_files.len(), total_deals, total_positions
                                            )));
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                let _ = msg_tx.send(BrokerMsg::Error(format!("Cannot read dir {}: {}", dir.display(), e)));
                            }
                        }
                        // Flag release via ImportingGuard's Drop.
                    });
                }
                BrokerCmd::ExportDarwinData => {
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::task::spawn_blocking(move || {
                        match shared_cache_broker.read().ok().and_then(|g| g.clone()).ok_or("Cache not ready".to_string()) {
                            Ok(cache) => {
                                if let Ok(conn) = cache.connection() {
                                    match darwin::export_darwin_data(&conn) {
                                        Ok((json, accts, deals, positions)) => {
                                            let path = dirs_home().join("cache").join("darwin_export.json");
                                            match std::fs::write(&path, &json) {
                                                Ok(_) => {
                                                    let _ = msg_tx.send(BrokerMsg::OrderResult(format!(
                                                        "DARWIN export: {} accounts, {} deals, {} positions -> {}",
                                                        accts, deals, positions, path.display()
                                                    )));
                                                }
                                                Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("Write failed: {e}"))); }
                                            }
                                        }
                                        Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("Export failed: {e}"))); }
                                    }
                                }
                            }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(e)); }
                        }
                    });
                }
                BrokerCmd::ImportDarwinData { json } => {
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::task::spawn_blocking(move || {
                        match shared_cache_broker.read().ok().and_then(|g| g.clone()).ok_or("Cache not ready".to_string()) {
                            Ok(cache) => {
                                if let Ok(conn) = cache.connection() {
                                    match darwin::import_darwin_data(&conn, &json) {
                                        Ok((accts, deals, positions)) => {
                                            let _ = msg_tx.send(BrokerMsg::OrderResult(format!(
                                                "DARWIN import: {} accounts, {} deals, {} positions",
                                                accts, deals, positions
                                            )));
                                        }
                                        Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("Import failed: {e}"))); }
                                    }
                                }
                            }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(e)); }
                        }
                    });
                }
                BrokerCmd::FundamentalsScrape {
                    db_path: _,
                    use_mt5,
                    use_alpaca,
                    use_tastytrade,
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
                    if use_tastytrade {
                        if let Some(ref tt) = tt_broker {
                            match tt.get_positions().await {
                                Ok(positions) => {
                                    let syms: Vec<String> = positions.iter()
                                        .filter(|p| p.instrument_type == "Equity")
                                        .map(|p| p.symbol.clone()).collect();
                                    let _ = broker_msg_tx_clone.send(BrokerMsg::FundamentalsProgress(format!("TastyTrade: {} equity positions", syms.len())));
                                    extra_tickers.extend(syms);
                                }
                                Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::FundamentalsProgress(format!("TastyTrade positions failed: {}", e))); }
                            }
                        } else {
                            let _ = broker_msg_tx_clone.send(BrokerMsg::FundamentalsProgress("TastyTrade not connected — skipping".into()));
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
                BrokerCmd::ResearchScrape { use_mt5, use_alpaca, use_tastytrade, finnhub_key, fmp_key } => {
                    let mut extra_tickers: Vec<String> = Vec::new();
                    if use_alpaca {
                        if let Some(ref b) = broker {
                            if let Ok(assets) = b.get_all_assets().await {
                                extra_tickers.extend(assets.iter().filter(|a| a.asset_class == "us_equity" && a.tradable).map(|a| a.symbol.clone()));
                            }
                        }
                    }
                    if use_tastytrade {
                        if let Some(ref tt) = tt_broker {
                            if let Ok(positions) = tt.get_positions().await {
                                extra_tickers.extend(positions.iter().filter(|p| p.instrument_type == "Equity").map(|p| p.symbol.clone()));
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
                BrokerCmd::CompactStorage { db_path: _, level } => {
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    let importing = importing_flag.clone();
                    tokio::task::spawn_blocking(move || {
                        // RAII guard — flag flip back to false happens on every
                        // exit (Ok/Err arms + panic unwind) so a compact crash
                        // can't wedge the background stats worker permanently.
                        importing.store(true, std::sync::atomic::Ordering::Relaxed);
                        struct ImportingGuard(std::sync::Arc<std::sync::atomic::AtomicBool>);
                        impl Drop for ImportingGuard {
                            fn drop(&mut self) {
                                self.0.store(false, std::sync::atomic::Ordering::Relaxed);
                            }
                        }
                        let _guard = ImportingGuard(importing.clone());
                        match shared_cache_broker.read().ok().and_then(|g| g.clone()).ok_or("Cache not ready".to_string()) {
                            Ok(cache) => {
                                let msg_tx2 = msg_tx.clone();
                                match cache.compact_storage(level, Some(&|processed, total, key, old_size, new_size| {
                                    if processed % 200 == 0 || processed == total {
                                        let _ = msg_tx2.send(BrokerMsg::OrderResult(format!(
                                            "Compact: {}/{} — {} ({} → {} bytes)",
                                            processed, total, key, old_size, new_size
                                        )));
                                    }
                                })) {
                                    Ok((count, saved)) => {
                                        // Reclaim freed pages after compaction reduced blob sizes
                                        let _ = cache.incremental_vacuum(10000);
                                        let _ = msg_tx.send(BrokerMsg::OrderResult(format!(
                                            "Compact complete: {} entries, {:.1} MB saved",
                                            count, saved as f64 / 1024.0 / 1024.0
                                        )));
                                    }
                                    Err(e) => {
                                        let _ = msg_tx.send(BrokerMsg::Error(format!("Compact failed: {}", e)));
                                    }
                                }
                            }
                            Err(e) => {
                                let _ = msg_tx.send(BrokerMsg::Error(format!("Cannot open cache: {e}")));
                            }
                        }
                    });
                }
                BrokerCmd::ScanUnusualVolume { keys } => {
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared_cache_broker = shared_cache_broker.clone();
                    tokio::task::spawn_blocking(move || {
                        let mut results: Vec<(String, f64, f64, f64)> = Vec::new();
                        if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                            for (key, count) in &keys {
                                if *count < 30 { continue; }
                                if !key.contains(":1Day") { continue; }
                                if let Ok(Some(raw)) = cache.get_bars_raw(key) {
                                    let n = raw.len();
                                    if n < 21 { continue; }
                                    let today_vol = raw[n-1].5;
                                    let avg_vol: f64 = raw[n-21..n-1].iter().map(|r| r.5).sum::<f64>() / 20.0;
                                    if avg_vol > 0.0 {
                                        let ratio = today_vol / avg_vol;
                                        if ratio > 1.5 {
                                            let parts: Vec<&str> = key.split(':').collect();
                                            let sym = if parts.len() >= 3 { parts[parts.len()-2] } else { key.as_str() };
                                            // Upper-case once at creation so the per-frame filter below skips the alloc.
                                            results.push((sym.to_uppercase(), today_vol, avg_vol, ratio));
                                        }
                                    }
                                }
                            }
                        }
                        results.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));
                        let _ = msg_tx.send(BrokerMsg::UnusualVolumeResults(results));
                    });
                }
                BrokerCmd::StartDxLinkStream { symbols } => {
                    if let Some(ref tb) = tt_broker {
                        let msg_tx = broker_msg_tx_clone.clone();
                        let symbol_count = symbols.len();
                        let paused = tt_dx_backoff_until
                            .lock()
                            .await
                            .as_ref()
                            .copied()
                            .is_some_and(|until| until > std::time::Instant::now());
                        if paused {
                            let _ = msg_tx.send(BrokerMsg::Error(
                                "DXLink token failed: tastytrade market data is paused after a recent token failure"
                                    .into(),
                            ));
                            continue;
                        }
                        match tb.get_streaming_token().await {
                            Ok(dx_token) => {
                                *tt_dx_backoff_until.lock().await = None;
                                match typhoon_engine::broker::dxlink::subscribe_quotes(&dx_token, symbols).await {
                                    Ok(mut rx) => {
                                        let _ = msg_tx.send(BrokerMsg::OrderResult(
                                            format!("DXLink stream started for {} symbols", symbol_count)
                                        ));
                                        tokio::spawn(async move {
                                            while let Some(q) = rx.recv().await {
                                                if msg_tx.send(BrokerMsg::StreamQuoteTick {
                                                    symbol: q.symbol,
                                                    bid: q.bid,
                                                    ask: q.ask,
                                                }).is_err() {
                                                    break; // UI dropped
                                                }
                                            }
                                        });
                                    }
                                    Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("DXLink stream failed: {e}"))); }
                                }
                            }
                            Err(e) => {
                                *tt_dx_backoff_until.lock().await = Some(
                                    std::time::Instant::now()
                                        + std::time::Duration::from_secs(
                                            tastytrade_sync_backoff_secs(&e) as u64,
                                        ),
                                );
                                let msg = if tastytrade_quote_streamer_customer_missing(&e) {
                                    tastytrade_quote_streamer_customer_missing_message(
                                        "current", &e,
                                    )
                                } else {
                                    format!("DXLink token failed: {e}")
                                };
                                let _ = msg_tx.send(BrokerMsg::Error(msg));
                            }
                        }
                    } else {
                        let _ = broker_msg_tx_clone.send(BrokerMsg::Error("Connect tastytrade first for DXLink stream".into()));
                    }
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
                BrokerCmd::DarwinFtpScan { ftp_dir, min_days } => {
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::task::spawn_blocking(move || {
                        let _ = msg_tx.send(BrokerMsg::OrderResult("DARWIN FTP scan started...".into()));
                        let ftp_path = std::path::Path::new(&ftp_dir);
                        match darwin_ftp::scan_universe(ftp_path, min_days, Some(&|done, total| {
                            let _ = msg_tx.send(BrokerMsg::OrderResult(format!("FTP scan: {}/{} DARWINs...", done, total)));
                        })) {
                            Ok(results) => {
                                let count = results.len();
                                let _ = msg_tx.send(BrokerMsg::OrderResult(format!("FTP scan complete: {} DARWINs with {}+ days", count, min_days)));
                                let _ = msg_tx.send(BrokerMsg::DarwinFtpScanResult(results));
                            }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("FTP scan failed: {}", e))); }
                        }
                    });
                }
                BrokerCmd::DarwinGpuScan { ftp_dir, min_days } => {
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::task::spawn_blocking(move || {
                        let _ = msg_tx.send(BrokerMsg::OrderResult("GPU scan: reading FTP return files...".into()));
                        let ftp_path = std::path::Path::new(&ftp_dir);
                        match darwin_ftp::list_all_darwins(ftp_path) {
                            Ok(tickers) => {
                                let mut all_returns: Vec<(String, Vec<f32>)> = Vec::new();
                                let total = tickers.len();
                                for (i, ticker) in tickers.iter().enumerate() {
                                    if i % 5000 == 0 {
                                        let _ = msg_tx.send(BrokerMsg::OrderResult(format!("GPU scan: reading {}/{}...", i, total)));
                                    }
                                    let return_path = ftp_path.join(ticker).join("RETURN");
                                    if !return_path.is_file() { continue; }
                                    if let Ok(returns) = darwin_ftp::read_return_file(ftp_path, ticker) {
                                        if returns.len() >= min_days {
                                            let daily = darwin_ftp::compute_daily_returns_from_ftp(&returns);
                                            let daily_f32: Vec<f32> = daily.iter().map(|&r| r as f32).collect();
                                            if !daily_f32.is_empty() {
                                                all_returns.push((ticker.clone(), daily_f32));
                                            }
                                        }
                                    }
                                }
                                let _ = msg_tx.send(BrokerMsg::OrderResult(format!("GPU scan: {} DARWINs read, sending to GPU...", all_returns.len())));
                                let _ = msg_tx.send(BrokerMsg::DarwinFtpReturns(all_returns));
                            }
                            Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("GPU scan failed: {}", e))); }
                        }
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
                BrokerCmd::TastytradeConnect { username, password, sandbox } => {
                    use typhoon_engine::broker::tastytrade::TastytradeBroker;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let mut tb = TastytradeBroker::new(sandbox);
                    match tb.login(&username, &password).await {
                        Ok(_session) => {
                            let Some(acct) = tb.account_number().map(str::to_string) else {
                                *tt_dx_token.lock().await = None;
                                *tt_dx_backoff_until.lock().await = None;
                                let env = if sandbox { "sandbox" } else { "production" };
                                let detail = tb
                                    .last_accounts_error()
                                    .map(str::to_string)
                                    .unwrap_or_else(|| {
                                        "no accounts were returned by /customers/me/accounts".to_string()
                                    });
                                tt_broker = None;
                                let _ = msg_tx.send(BrokerMsg::Error(format!(
                                    "tastytrade {env} login authenticated, but no trading account could be selected. Detail: {detail}. If you see a customer record but no account, attach a trading account to this {env} user on https://developer.tastytrade.com (sandbox) or your live account."
                                )));
                                continue;
                            };
                            let mut token_preflight_error: Option<String> = None;
                            match tb.get_streaming_token().await {
                                Ok(dx_token) => {
                                    *tt_dx_token.lock().await = Some(dx_token);
                                    *tt_dx_backoff_until.lock().await = None;
                                }
                                Err(e) if tastytrade_quote_streamer_customer_missing(&e) => {
                                    *tt_dx_token.lock().await = None;
                                    *tt_dx_backoff_until.lock().await = Some(
                                        std::time::Instant::now()
                                            + std::time::Duration::from_secs(
                                                tastytrade_sync_backoff_secs(&e) as u64,
                                            ),
                                    );
                                    tt_broker = None;
                                    let _ = msg_tx.send(BrokerMsg::Error(
                                        tastytrade_quote_streamer_customer_missing_message(
                                            if sandbox { "sandbox" } else { "production" },
                                            &e,
                                        ),
                                    ));
                                    continue;
                                }
                                Err(e) => {
                                    *tt_dx_token.lock().await = None;
                                    *tt_dx_backoff_until.lock().await = Some(
                                        std::time::Instant::now()
                                            + std::time::Duration::from_secs(
                                                tastytrade_sync_backoff_secs(&e) as u64,
                                            ),
                                    );
                                    token_preflight_error =
                                        Some(format!("DXLink token failed: {e}"));
                                }
                            }
                            let _ = msg_tx.send(BrokerMsg::Connected(format!(
                                "tastytrade {} — account {}",
                                if sandbox { "Sandbox" } else { "Production" },
                                acct
                            )));
                            if let Some(e) = token_preflight_error {
                                let _ = msg_tx.send(BrokerMsg::Error(e));
                            }
                            // Fetch balances
                            if let Ok(bal) = tb.get_balances().await {
                                let _ = msg_tx.send(BrokerMsg::TastytradeBalances(bal.clone()));
                                let _ = msg_tx.send(BrokerMsg::OrderResult(format!(
                                    "tastytrade: NLV ${:.2}, BP ${:.2}, Cash ${:.2}",
                                    bal.net_liquidating_value, bal.equity_buying_power, bal.cash_balance
                                )));
                            }
                            // Fetch and convert positions to unified format
                            if let Ok(positions) = tb.get_positions().await {
                                let _ = msg_tx.send(BrokerMsg::OrderResult(format!(
                                    "tastytrade: {} open positions", positions.len()
                                )));
                                let unified: Vec<PositionInfo> = positions.iter().map(|p| PositionInfo {
                                    symbol: p.symbol.clone(),
                                    qty: p.quantity.abs(),
                                    side: if p.quantity_direction == "Long" { "long".into() } else { "short".into() },
                                    avg_entry_price: p.average_open_price,
                                    market_value: p.mark_price.unwrap_or(p.close_price) * p.quantity.abs(),
                                    unrealized_pl: p.unrealized_pnl.unwrap_or(0.0),
                                    asset_class: p.instrument_type.clone(),
                                    asset_id: String::new(),
                                }).collect();
                                let _ = msg_tx.send(BrokerMsg::TastytradePositions(unified));
                            }
                            match tb.get_market_data_universe_symbols().await {
                                Ok(symbols) => {
                                    let _ = msg_tx.send(BrokerMsg::TastytradeUniverse(symbols));
                                }
                                Err(e) => {
                                    let _ = msg_tx.send(BrokerMsg::Error(format!(
                                        "tastytrade universe failed: {}",
                                        e
                                    )));
                                }
                            }
                            tt_broker = Some(tb);
                        }
                        Err(e) => {
                            let _ = msg_tx.send(BrokerMsg::Error(format!("tastytrade login failed: {}", e)));
                        }
                    }
                }
                BrokerCmd::TastytradeGetUniverse => {
                    if let Some(ref tb) = tt_broker {
                        match tb.get_market_data_universe_symbols().await {
                            Ok(symbols) => {
                                let _ = broker_msg_tx_clone
                                    .send(BrokerMsg::TastytradeUniverse(symbols));
                            }
                            Err(e) => {
                                let _ = broker_msg_tx_clone.send(BrokerMsg::Error(format!(
                                    "tastytrade universe failed: {}",
                                    e
                                )));
                            }
                        }
                    } else {
                        let _ = broker_msg_tx_clone
                            .send(BrokerMsg::OrderResult("tastytrade: connect first".into()));
                    }
                }
                BrokerCmd::TastytradeGetBalances => {
                    if let Some(ref tb) = tt_broker {
                        match tb.get_balances().await {
                            Ok(bal) => {
                                let _ = broker_msg_tx_clone.send(BrokerMsg::TastytradeBalances(bal));
                            }
                            Err(e) => {
                                let _ = broker_msg_tx_clone.send(BrokerMsg::Error(format!(
                                    "tastytrade balances: {}",
                                    e
                                )));
                            }
                        }
                    }
                }
                BrokerCmd::TastytradePositions => {
                    if let Some(ref tb) = tt_broker {
                        match tb.get_positions().await {
                            Ok(positions) => {
                                let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult(format!(
                                    "tastytrade: {} positions", positions.len()
                                )));
                                // Convert to unified PositionInfo for chart overlay
                                let unified: Vec<PositionInfo> = positions.iter().map(|p| PositionInfo {
                                    symbol: p.symbol.clone(),
                                    qty: p.quantity.abs(),
                                    side: if p.quantity_direction == "Long" { "long".into() } else { "short".into() },
                                    avg_entry_price: p.average_open_price,
                                    market_value: p.mark_price.unwrap_or(p.close_price) * p.quantity.abs(),
                                    unrealized_pl: p.unrealized_pnl.unwrap_or(0.0),
                                    asset_class: p.instrument_type.clone(),
                                    asset_id: String::new(),
                                }).collect();
                                let _ = broker_msg_tx_clone.send(BrokerMsg::TastytradePositions(unified));
                            }
                            Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(e)); }
                        }
                    } else {
                        let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult("tastytrade: connect first".into()));
                    }
                }
                BrokerCmd::TastytradeOptionChain { symbol } => {
                    if let Some(ref tb) = tt_broker {
                        match tb.get_option_chain(&symbol).await {
                            Ok(expirations) => {
                                let total_strikes: usize = expirations.iter().map(|e| e.strikes.len()).sum();
                                let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult(format!(
                                    "tastytrade: {} option chain — {} expirations, {} strikes",
                                    symbol, expirations.len(), total_strikes
                                )));
                                // Store to KV for UI display
                                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                                    if let Ok(json) = serde_json::to_string(&expirations) {
                                        let _ = cache.put_kv(&format!("tt:options:{}", symbol), &json);
                                    }
                                }
                            }
                            Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(format!("tastytrade option chain: {}", e))); }
                        }
                    } else {
                        let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult("tastytrade: connect first".into()));
                    }
                }
                BrokerCmd::TastyTradeFetchBars { symbol, timeframe, backfill_complete } => {
                    if let Some(ref tb) = tt_broker {
                        let broker = tb.clone();
                        let dx_token_cache = tt_dx_token.clone();
                        let dx_backoff_until = tt_dx_backoff_until.clone();
                        let msg_tx = broker_msg_tx_clone.clone();
                        let shared_cache = shared_cache_broker.clone();
                        let permits = tastytrade_fetch_permits.clone();
                        tokio::spawn(async move {
                            let Ok(_permit) = permits.acquire_owned().await else {
                                let _ = msg_tx.send(BrokerMsg::TastytradeFetchSettled {
                                    symbol,
                                    timeframe,
                                });
                                return;
                            };
                            run_tastytrade_fetch_task(
                                broker,
                                dx_token_cache,
                                dx_backoff_until,
                                shared_cache,
                                msg_tx,
                                symbol,
                                timeframe,
                                backfill_complete,
                            )
                            .await;
                        });
                    } else {
                        let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult("tastytrade: connect first for DXLink bars".into()));
                        let _ = broker_msg_tx_clone.send(BrokerMsg::TastytradeFetchSettled {
                            symbol,
                            timeframe,
                        });
                    }
                }
                BrokerCmd::FredFetch { api_key } => {
                    use typhoon_engine::core::fred;
                    let client = reqwest::Client::new();
                    let mut series_data = Vec::new();
                    for (id, _name) in fred::KEY_SERIES.iter() {
                        if let Ok(s) = fred::fetch_series(&client, &api_key, id, 60).await {
                            series_data.push(s);
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                    }
                    let yield_curve = fred::fetch_yield_curve(&client, &api_key).await.unwrap_or_default();
                    let _ = broker_msg_tx_clone.send(BrokerMsg::FredData(series_data, yield_curve));
                }
                BrokerCmd::FetchEconCalendar { finnhub_key } => {
                    // Strategy: if Finnhub key present, use Finnhub (richer — includes "actual" values).
                    // Otherwise fall back to ForexFactory weekly XML (free, no key, ForexFactory-parity data).
                    let client = reqwest::Client::new();
                    if !finnhub_key.is_empty() {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let next_week = (chrono::Utc::now() + chrono::Duration::days(7)).format("%Y-%m-%d").to_string();
                        let url = format!("https://finnhub.io/api/v1/calendar/economic?from={}&to={}&token={}", today, next_week, finnhub_key);
                        match client.get(&url).send().await {
                            Ok(resp) => {
                                if let Ok(body) = resp.json::<serde_json::Value>().await {
                                    let mut events = Vec::new();
                                    if let Some(arr) = body["economicCalendar"].as_array() {
                                        for e in arr {
                                            let date = e["time"].as_str().unwrap_or("").to_string();
                                            let country = e["country"].as_str().unwrap_or("").to_string();
                                            let event_name = e["event"].as_str().unwrap_or("").to_string();
                                            let impact = e["impact"].as_str().unwrap_or("low").to_string();
                                            let actual = e["actual"].as_f64().map(|v| format!("{:.2}", v)).unwrap_or("\u{2014}".into());
                                            events.push((date, country, event_name, impact, actual));
                                        }
                                    }
                                    let _ = broker_msg_tx_clone.send(BrokerMsg::EconCalendarData(events));
                                    continue;
                                }
                            }
                            Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(format!("Finnhub econ calendar: {}", e))); }
                        }
                    }
                    // ForexFactory fallback (keyless)
                    match typhoon_engine::core::econ_calendar::fetch_forexfactory_week(&client).await {
                        Ok(ff_events) => {
                            let events: Vec<(String, String, String, String, String)> = ff_events.into_iter()
                                .map(|e| {
                                    // Flatten MM-DD-YYYY + time into ISO-ish "YYYY-MM-DD HH:MM"
                                    let date_str = if let Ok(d) = chrono::NaiveDate::parse_from_str(&e.date, "%m-%d-%Y") {
                                        format!("{} {}", d.format("%Y-%m-%d"), e.time)
                                    } else {
                                        format!("{} {}", e.date, e.time)
                                    };
                                    let prev = if e.previous.is_empty() { "\u{2014}".to_string() } else { e.previous.clone() };
                                    let actual = if e.forecast.is_empty() { prev } else { format!("fc:{} (prev:{})", e.forecast, if e.previous.is_empty() { "-" } else { &e.previous }) };
                                    (date_str, e.country, e.title, e.impact.label().to_lowercase(), actual)
                                })
                                .collect();
                            let _ = broker_msg_tx_clone.send(BrokerMsg::EconCalendarData(events));
                        }
                        Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(format!("ForexFactory fallback: {}", e))); }
                    }
                }
                BrokerCmd::FetchCongressTrades => {
                    let client = reqwest::Client::builder().user_agent("TyphooN-Terminal/1.0").build().unwrap_or_default();
                    match client.get("https://house-stock-watcher-data.s3-us-west-2.amazonaws.com/data/all_transactions.json")
                        .timeout(std::time::Duration::from_secs(30))
                        .send().await {
                        Ok(resp) => {
                            if let Ok(body) = resp.json::<serde_json::Value>().await {
                                let mut trades = Vec::new();
                                if let Some(arr) = body.as_array() {
                                    // Take last 200 (most recent)
                                    for t in arr.iter().rev().take(200) {
                                        let date = t["transaction_date"].as_str().unwrap_or("").to_string();
                                        let rep = t["representative"].as_str().unwrap_or("").to_string();
                                        let ticker = t["ticker"].as_str().unwrap_or("").to_string();
                                        let tx_type = t["type"].as_str().unwrap_or("").to_string();
                                        let amount = t["amount"].as_str().unwrap_or("").to_string();
                                        let party = t["party"].as_str().unwrap_or("").to_string();
                                        if !ticker.is_empty() && ticker != "--" {
                                            trades.push((date, rep, ticker, tx_type, amount, party));
                                        }
                                    }
                                }
                                let _ = broker_msg_tx_clone.send(BrokerMsg::CongressData(trades));
                            }
                        }
                        Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(format!("Congress trades: {}", e))); }
                    }
                }
                BrokerCmd::SendNotification { discord_webhook, pushover_token, pushover_user, ntfy_topic, message } => {
                    use typhoon_engine::notifications;
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let mut sent = false;
                        if !discord_webhook.is_empty() {
                            if let Err(e) = notifications::send_discord(&discord_webhook, &message).await {
                                let _ = msg_tx.send(BrokerMsg::Error(format!("Discord: {}", e)));
                            } else { sent = true; }
                        }
                        if !pushover_token.is_empty() && !pushover_user.is_empty() {
                            if let Err(e) = notifications::send_pushover(&pushover_token, &pushover_user, &message).await {
                                let _ = msg_tx.send(BrokerMsg::Error(format!("Pushover: {}", e)));
                            } else { sent = true; }
                        }
                        if !ntfy_topic.is_empty() {
                            if let Err(e) = notifications::send_ntfy(&ntfy_topic, &message).await {
                                let _ = msg_tx.send(BrokerMsg::Error(format!("ntfy: {}", e)));
                            } else { sent = true; }
                        }
                        if sent {
                            let _ = msg_tx.send(BrokerMsg::OrderResult(format!("Notification sent: {}", &message[..message.len().min(60)])));
                        }
                    });
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
                BrokerCmd::LanSyncStart { port, passphrase, .. } => {
                    use typhoon_engine::core::lan_sync::LanSyncServer;
                    // Spawn as independent task — cert generation is CPU-heavy (100-500ms)
                    // and must not block the broker command loop
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared = shared_cache_broker.clone();
                    tokio::spawn(async move {
                        // Wait for cache to be ready (up to 30s)
                        let mut cache_arc = shared.read().ok().and_then(|g| g.clone());
                        if cache_arc.is_none() {
                            for _ in 0..30 {
                                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                                cache_arc = shared.read().ok().and_then(|g| g.clone());
                                if cache_arc.is_some() { break; }
                            }
                        }
                        if let Some(cache_arc) = cache_arc {
                            match LanSyncServer::start(cache_arc, port, &passphrase).await {
                                Ok(_server) => {
                                    let _ = msg_tx.send(BrokerMsg::OrderResult(format!("LAN sync server running on wss://0.0.0.0:{}", port)));
                                    // Keep server alive — don't let _server drop
                                    // The accept loop runs inside a spawned task, so it survives
                                    // even after _server is dropped (JoinHandle detaches on drop)
                                }
                                Err(e) => { let _ = msg_tx.send(BrokerMsg::Error(format!("LAN sync server failed: {}", e))); }
                            }
                        } else {
                            let _ = msg_tx.send(BrokerMsg::Error("LAN sync: cache not ready yet".into()));
                        }
                    });
                }
                BrokerCmd::LanSyncConnect { host, port, passphrase, .. } => {
                    use typhoon_engine::core::lan_sync::LanSyncClient;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let shared = shared_cache_broker.clone();
                    let lan_remote = lan_remote_tx_ref.clone();
                    let lan_flag = lan_client.clone();
                    // Store abort handle so LanSyncStop can kill the reconnect loop
                    let reconnect_task = tokio::spawn(async move {
                        // Wait for cache to be ready (up to 30s) — handles startup race
                        // where LAN auto-connect fires before async cache-open completes.
                        let mut cache_arc = shared.read().ok().and_then(|g| g.clone());
                        if cache_arc.is_none() {
                            for _ in 0..30 {
                                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                                cache_arc = shared.read().ok().and_then(|g| g.clone());
                                if cache_arc.is_some() { break; }
                            }
                        }
                        let Some(cache_arc) = cache_arc else {
                            let _ = msg_tx.send(BrokerMsg::Error("LAN sync: cache not ready yet".into()));
                            return;
                        };

                        // Auto-reconnect loop: retry every 30s on failure.
                        // The WebSocket stays connected and uses incremental re-sync (every 15s)
                        // for bars, KV, and tables. Full reconnect only on connection drop or
                        // very long intervals (2 hours) to refresh TLS certificate.
                        const RESYNC_INTERVAL_SECS: u64 = 2 * 60 * 60; // 2 hours
                        loop {
                            match tokio::time::timeout(
                                std::time::Duration::from_secs(10),
                                LanSyncClient::connect(cache_arc.clone(), &host, port, &passphrase),
                            ).await {
                                Ok(Ok((client, remote_tx))) => {
                                    { let mut guard = lan_remote.lock().await; *guard = Some(remote_tx); }
                                    lan_flag.store(true, std::sync::atomic::Ordering::Relaxed);
                                    let _ = msg_tx.send(BrokerMsg::OrderResult(format!("LAN sync connected to wss://{}:{}", host, port)));
                                    // Wait for sync to complete (up to RESYNC_INTERVAL then force reconnect)
                                    let timed_out = tokio::time::timeout(
                                        std::time::Duration::from_secs(RESYNC_INTERVAL_SECS),
                                        client.wait(),
                                    ).await.is_err();
                                    // Trigger chart reload — bars may have been synced
                                    let _ = msg_tx.send(BrokerMsg::Mt5SyncDone(1));
                                    // Connection dropped or periodic resync — clear state and retry
                                    { let mut guard = lan_remote.lock().await; *guard = None; }
                                    lan_flag.store(false, std::sync::atomic::Ordering::Relaxed);
                                    if timed_out {
                                        // Periodic resync — reconnect immediately (no sleep)
                                        continue;
                                    }
                                    let _ = msg_tx.send(BrokerMsg::Error("LAN sync disconnected — reconnecting in 30s...".into()));
                                }
                                Ok(Err(e)) => {
                                    let _ = msg_tx.send(BrokerMsg::Error(format!("LAN sync failed: {} — retrying in 30s...", e)));
                                }
                                Err(_) => {
                                    let _ = msg_tx.send(BrokerMsg::Error("LAN sync timed out — retrying in 30s...".into()));
                                }
                            }
                            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                        }
                    });
                    // Store the abort handle for LanSyncStop
                    lan_reconnect_handle = Some(reconnect_task.abort_handle());
                }
                BrokerCmd::LanSyncStop => {
                    // Abort the auto-reconnect loop task
                    if let Some(handle) = lan_reconnect_handle.take() {
                        handle.abort();
                    }
                    // Clear the LAN remote channel so commands stop being forwarded
                    { let mut guard = lan_remote_tx_ref.lock().await; *guard = None; }
                    // Clear the LAN client flag so broker commands execute locally again
                    lan_client.store(false, std::sync::atomic::Ordering::Relaxed);
                    let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult("LAN sync stopped".into()));
                }
                BrokerCmd::LanResyncBars => {
                    let guard = lan_remote_tx_ref.lock().await;
                    if let Some(ref tx) = *guard {
                        let _ = tx.send("RESYNC_BARS".to_string());
                        let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult("LAN resync bars requested...".into()));
                    } else {
                        let _ = broker_msg_tx_clone.send(BrokerMsg::Error("Not connected to LAN server".into()));
                    }
                }
                BrokerCmd::LanResyncDarwin => {
                    let guard = lan_remote_tx_ref.lock().await;
                    if let Some(ref tx) = *guard {
                        let _ = tx.send("RESYNC_DARWIN".to_string());
                        let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult("LAN resync DARWIN requested...".into()));
                    } else {
                        let _ = broker_msg_tx_clone.send(BrokerMsg::Error("Not connected to LAN server".into()));
                    }
                }
            }
        }
    });
}
