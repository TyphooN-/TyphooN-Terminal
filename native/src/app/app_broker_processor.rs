use super::*;

mod external_feeds;
mod market_data_commands;
mod alpaca_account_data;
mod alpaca_order_ops;
mod news;
mod research_compute;
mod storage;
mod symbol_search;
mod watchlist_quotes;

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

fn top_of_kraken_ws_v2_book(
    state: &typhoon_engine::broker::kraken::KrakenWsBookState,
) -> Option<(f64, f64)> {
    let bid = state.bids.first()?.price;
    let ask = state.asks.first()?.price;
    (bid > 0.0 && ask > 0.0 && bid.is_finite() && ask.is_finite()).then_some((bid, ask))
}

fn resolve_kraken_chart_book_ws_symbol(symbol: &str) -> Option<String> {
    let bare = symbol
        .trim()
        .trim_end_matches(".EQ")
        .trim_end_matches(".eq")
        .to_ascii_uppercase();
    if bare.is_empty() || bare.contains('/') {
        return None;
    }
    Some(format!("{bare}x/USD"))
}

pub(super) fn spawn_broker_message_processor(
    broker_cmd_rx: tokio::sync::mpsc::UnboundedReceiver<BrokerCmd>,
    broker_msg_tx: tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    importing_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    rt_handle: tokio::runtime::Handle,
    shared_cache: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
) {
    // Spawn broker message processor
    let broker_msg_tx_clone = broker_msg_tx.clone();
    let importing_flag_broker = importing_flag.clone();
    let shared_cache_broker = shared_cache.clone();
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
        while let Some(cmd) = cmd_rx.recv().await {
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
                cmd @ (BrokerCmd::GetAccount
                | BrokerCmd::GetPositions
                | BrokerCmd::GetOrders
                | BrokerCmd::GetOrderHistory { .. }) => {
                    alpaca_account_data::handle_alpaca_account_data_command(
                        cmd,
                        broker.as_ref(),
                        &broker_msg_tx_clone,
                    )
                    .await;
                }
                cmd @ (BrokerCmd::CloseAll | BrokerCmd::ClosePosition { .. }) => {
                    alpaca_order_ops::handle_alpaca_order_command(
                        cmd,
                        broker.as_ref(),
                        &broker_msg_tx_clone,
                    )
                    .await;
                }
                BrokerCmd::SecScrape { db_path, symbols } => {
                    // Spawn as independent task — SEC scraping can take 10-60s and must not
                    // block the broker command loop (would freeze trading, data fetch, etc.)
                    let msg_tx = broker_msg_tx_clone.clone();
                    tokio::spawn(async move {
                        let _ = msg_tx.send(BrokerMsg::OrderResult("SEC scrape started...".into()));
                        // Overall cap so a stalled batch (slow EDGAR pacing or SQLite
                        // write-lock contention under heavy sync — the per-request
                        // client timeout is only 15s, but the whole batch can still
                        // grind) always reports back and clears the UI busy flag in
                        // minutes rather than waiting out the 30-min stale watchdog.
                        let scrape = sec_filing::scrape_all_portfolio_symbols(db_path, Some(symbols));
                        match tokio::time::timeout(std::time::Duration::from_secs(600), scrape).await
                        {
                            Ok(Ok(stats)) => {
                                let error_suffix = if stats.errors.is_empty() {
                                    String::new()
                                } else {
                                    format!(", {} errors (first: {})", stats.errors.len(), stats.errors[0])
                                };
                                let _ = msg_tx.send(BrokerMsg::SecScrapeResult(
                                    format!("SEC scrape complete: {} tickers, {} filings, {} insider trades, {} alerts{}", stats.tickers_scanned, stats.new_filings, stats.new_insider_trades, stats.new_alerts, error_suffix)
                                ));
                            }
                            Ok(Err(e)) => {
                                let _ = msg_tx.send(BrokerMsg::SecScrapeResult(format!(
                                    "SEC scrape error: {}",
                                    e
                                )));
                            }
                            Err(_) => {
                                let _ = msg_tx.send(BrokerMsg::SecScrapeResult(
                                    "SEC scrape timed out after 10m — busy flag cleared".into(),
                                ));
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
                    watchlist_quotes::spawn_watchlist_quotes_task(
                        symbols,
                        broker.clone(),
                        broker_msg_tx_clone.clone(),
                        shared_cache_broker.clone(),
                    );
                }
                BrokerCmd::GetMarketClock => {
                    // US-equity/xStock session status is sourced from Alpaca's market clock.
                    // Kraken crypto pairs are shown separately as 24/7 in the toolbar.
                    if let Some(ref b) = broker {
                        match b.get_market_clock().await {
                            Ok(v) => {
                                let is_open = v["is_open"].as_bool().unwrap_or(false);
                                let next_open = v["next_open"].as_str().unwrap_or("—");
                                let next_close = v["next_close"].as_str().unwrap_or("—");

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
                cmd @ (BrokerCmd::GetActivities { .. }
                | BrokerCmd::GetTopMovers
                | BrokerCmd::GetAllAssets) => {
                    alpaca_account_data::handle_alpaca_account_data_command(
                        cmd,
                        broker.as_ref(),
                        &broker_msg_tx_clone,
                    )
                    .await;
                }
                BrokerCmd::SearchSymbols { query } => {
                    symbol_search::handle_symbol_search_command(
                        query,
                        broker.as_ref(),
                        &broker_msg_tx_clone,
                    )
                    .await;
                }

                cmd @ (BrokerCmd::GetFundamentals { .. }
                | BrokerCmd::GetHolders { .. }
                | BrokerCmd::GetAnalyst { .. }
                | BrokerCmd::GetOrderbook { .. }
                | BrokerCmd::GetMostActive
                | BrokerCmd::GetPortfolioHistory { .. }
                | BrokerCmd::GetPriceTarget { .. }
                | BrokerCmd::GetShortInterest { .. }
                | BrokerCmd::GetCorporateActions { .. }
                | BrokerCmd::GetWatchlists
                | BrokerCmd::CreateWatchlist { .. }
                | BrokerCmd::GetOptionsChain { .. }) => {
                    market_data_commands::handle_market_data_command(
                        cmd,
                        broker.as_ref(),
                        kraken_broker.as_ref(),
                        &shared_cache_broker,
                        &broker_msg_tx_clone,
                    )
                    .await;
                }
                cmd @ (BrokerCmd::AlpacaMarketOrder { .. }
                | BrokerCmd::AlpacaLimitOrder { .. }
                | BrokerCmd::AlpacaStopOrder { .. }
                | BrokerCmd::AlpacaBracketOrder { .. }
                | BrokerCmd::AlpacaCancelOrder { .. }
                | BrokerCmd::AlpacaOcoOrder { .. }
                | BrokerCmd::AlpacaModifyOrder { .. }
                | BrokerCmd::AlpacaSyncExits { .. }) => {
                    alpaca_order_ops::handle_alpaca_order_command(
                        cmd,
                        broker.as_ref(),
                        &broker_msg_tx_clone,
                    )
                    .await;
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
                        // the AI response cache before spending tokens. On hit, emit the
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
                                    // record the fresh response in the AI response cache.
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
                                    // record the fresh response in the AI response cache.
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
                    use_alpaca, use_kraken,
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
                BrokerCmd::KrakenStartOhlcStreamers {
                    pairs,
                    intervals_min,
                } => {
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
                            intervals_min.clone(),
                            commit_tx,
                            status_tx,
                        );
                        let interval_count = intervals_min.len();
                        let _ = msg_tx.send(BrokerMsg::OrderResult(format!(
                            "Kraken WS OHLC streamers started: {pair_count} pairs × {interval_count} enabled intervals",
                        )));
                    }
                }
                BrokerCmd::KrakenOhlcSnapshotSweep {
                    interval_min,
                    pairs,
                } => {
                    let msg_tx = broker_msg_tx_clone.clone();
                    let pair_count = pairs.len();
                    if pair_count == 0 {
                        let _ = msg_tx.send(BrokerMsg::KrakenWsOhlcSnapshotSweepSettled {
                            interval_min,
                            pair_count: 0,
                            error: None,
                        });
                    } else {
                        let (commit_tx, mut commit_rx) = tokio::sync::mpsc::unbounded_channel();
                        let (status_tx, mut status_rx) = tokio::sync::mpsc::unbounded_channel();
                        let (settled_tx, mut settled_rx) = tokio::sync::mpsc::unbounded_channel();

                        let commit_msg_tx = msg_tx.clone();
                        tokio::spawn(async move {
                            while let Some(fresh) = commit_rx.recv().await {
                                let _ = commit_msg_tx.send(BrokerMsg::KrakenWsBarsCommitted { fresh });
                            }
                        });
                        let status_msg_tx = msg_tx.clone();
                        tokio::spawn(async move {
                            while let Some(event) = status_rx.recv().await {
                                let (interval_min, kind, detail) = match event {
                                    typhoon_engine::broker::kraken::KrakenOhlcStreamerEvent::Connected { interval_min } => {
                                        (interval_min, "snapshot_connected".to_string(), String::new())
                                    }
                                    typhoon_engine::broker::kraken::KrakenOhlcStreamerEvent::Subscribed { interval_min, batches } => {
                                        (interval_min, "snapshot_subscribed".to_string(), format!("{batches} batches"))
                                    }
                                    typhoon_engine::broker::kraken::KrakenOhlcStreamerEvent::Disconnected { interval_min, reason } => {
                                        (interval_min, "snapshot_disconnected".to_string(), reason)
                                    }
                                    typhoon_engine::broker::kraken::KrakenOhlcStreamerEvent::SubscribeFailed { interval_min, reason } => {
                                        (interval_min, "snapshot_subscribe_failed".to_string(), reason)
                                    }
                                };
                                let _ = status_msg_tx.send(BrokerMsg::KrakenWsOhlcStatus {
                                    interval_min,
                                    kind,
                                    detail,
                                });
                            }
                        });
                        let settled_msg_tx = msg_tx.clone();
                        tokio::spawn(async move {
                            if let Some(result) = settled_rx.recv().await {
                                match result {
                                    Ok((interval_min, pair_count)) => {
                                        let _ = settled_msg_tx.send(BrokerMsg::KrakenWsOhlcSnapshotSweepSettled {
                                            interval_min,
                                            pair_count,
                                            error: None,
                                        });
                                    }
                                    Err(error) => {
                                        let _ = settled_msg_tx.send(BrokerMsg::KrakenWsOhlcSnapshotSweepSettled {
                                            interval_min,
                                            pair_count,
                                            error: Some(error),
                                        });
                                    }
                                }
                            }
                        });
                        kraken_ohlc_ws::spawn_kraken_ohlc_snapshot_sweep(
                            shared_cache_broker.clone(),
                            interval_min,
                            pairs,
                            commit_tx,
                            status_tx,
                            settled_tx,
                        );
                    }
                }
                BrokerCmd::KrakenStartOrderbookWs {
                    symbol,
                    depth,
                    publish_dom,
                } => {
                    let msg_tx = broker_msg_tx_clone.clone();
                    let ws_symbol = typhoon_engine::core::kraken::resolve_kraken_ws_pair(
                        &kraken_public_client,
                        &symbol,
                    )
                    .await
                    .or_else(|| resolve_kraken_chart_book_ws_symbol(&symbol));
                    let Some(ws_symbol) = ws_symbol else {
                        let _ = msg_tx.send(BrokerMsg::OrderResult(format!(
                            "Kraken WS v2 book skipped: {symbol} is not a WS-mappable Kraken pair"
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
                                                if let Some((bid, ask)) = top_of_kraken_ws_v2_book(&state) {
                                                    let _ = update_msg_tx.send(BrokerMsg::KrakenBookQuoteTick {
                                                        symbol: display_symbol.clone(),
                                                        bid,
                                                        ask,
                                                    });
                                                }
                                                if publish_dom {
                                                    let text = kraken_ws_v2_book_state_json(
                                                        &display_symbol,
                                                        &state,
                                                        checksum,
                                                        "ok",
                                                    );
                                                    let _ = update_msg_tx.send(BrokerMsg::KrakenOrderbookUpdate(text));
                                                }
                                            }
                                            Err(err) => {
                                                if publish_dom {
                                                    let text = kraken_ws_v2_book_state_json(
                                                        &display_symbol,
                                                        &state,
                                                        Some(err.actual),
                                                        "checksum_mismatch",
                                                    );
                                                    let _ = update_msg_tx.send(BrokerMsg::KrakenOrderbookUpdate(text));
                                                }
                                                resubscribe_count = resubscribe_count.saturating_add(1);
                                                if publish_dom {
                                                    let _ = update_msg_tx.send(BrokerMsg::Error(format!(
                                                        "Kraken WS v2 book checksum mismatch for {}: expected {}, actual {}; resubscribing snapshot attempt {}",
                                                        err.symbol, err.expected, err.actual, resubscribe_count
                                                    )));
                                                } else {
                                                    tracing::warn!(
                                                        "Kraken WS v2 book checksum mismatch for {}: expected {}, actual {}; resubscribing snapshot attempt {}",
                                                        err.symbol,
                                                        err.expected,
                                                        err.actual,
                                                        resubscribe_count
                                                    );
                                                }
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
                                        if publish_dom {
                                            let _ = update_msg_tx.send(BrokerMsg::OrderResult(text));
                                        } else {
                                            tracing::debug!("{text}");
                                        }
                                    }
                                }
                            }
                            streamer_handle.abort();
                            if !retry_after_mismatch {
                                break;
                            }
                            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                            if publish_dom {
                                let _ = update_msg_tx.send(BrokerMsg::OrderResult(format!(
                                    "Kraken WS v2 book resubscribing: {state_symbol} depth {depth}"
                                )));
                            }
                        }
                    });
                    if publish_dom {
                        let _ = msg_tx.send(BrokerMsg::OrderResult(format!(
                            "Kraken WS v2 book starting: {ws_symbol} depth {depth}"
                        )));
                    } else {
                        tracing::debug!(
                            "Kraken WS v2 chart book quote starting: {ws_symbol} depth {depth}"
                        );
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
                BrokerCmd::FundamentalsScrape {
                    db_path: _,
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
                                        .filter_map(|a| normalize_fundamentals_scrape_symbol(&a.symbol))
                                        .collect();
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
                        let syms: Vec<String> = normalize_kraken_equity_symbol_list(kraken_equity_symbols.iter())
                            .into_iter()
                            .filter_map(|sym| normalize_fundamentals_scrape_symbol(&sym))
                            .collect();
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
                                    all_tickers.extend(
                                        extra_tickers
                                            .into_iter()
                                            .filter_map(|ticker| normalize_fundamentals_scrape_symbol(&ticker)),
                                    );
                                    let mut tickers: Vec<String> = all_tickers.into_iter().collect();
                                    tickers.sort();
                                    if let Ok(conn) = cache.connection() {
                                        let _ = fundamentals::create_fundamentals_tables(&conn);
                                        fundamentals::prioritize_fundamentals_symbols(&conn, &mut tickers, force);
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
                                            // so other threads (BG, KV writes) aren't starved.
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
                BrokerCmd::ResearchScrape { use_alpaca, finnhub_key, fmp_key } => {
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
                                        let _ = broker_msg_tx_clone.send(BrokerMsg::BarsSynced(count));
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
                BrokerCmd::IgnoreNewsArticle { symbol, url_hash } => {
                    // Persist the removal: delete the row + remember the hash so the
                    // next GDELT/Finnhub fetch can't resurrect it. The UI removes the
                    // row optimistically; this makes it stick across reloads.
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        match cache.connection() {
                            Ok(conn) => {
                                match typhoon_engine::core::news::delete_news(
                                    &conn, &url_hash, &symbol,
                                ) {
                                    Ok(_) => tracing::info!(
                                        "News: deleted + ignored {} ({})",
                                        url_hash,
                                        symbol
                                    ),
                                    Err(e) => tracing::warn!(
                                        "News: failed to delete {}: {}",
                                        url_hash,
                                        e
                                    ),
                                }
                            }
                            Err(e) => tracing::warn!(
                                "News: no DB connection to delete {}: {}",
                                url_hash,
                                e
                            ),
                        }
                    }
                }
            }
        }
    });
}
