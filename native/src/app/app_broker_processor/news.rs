use super::*;

pub(super) fn handle_news_command(
    cmd: BrokerCmd,
    broker_msg_tx_clone: tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    shared_cache_broker: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
) {
    match cmd {
        BrokerCmd::IngestResearchArticles {
            text,
            agent_override,
        } => {
            use typhoon_engine::core::{news, research};
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let parsed = research::parse_ingest_block(&text);
                let mut per_symbol: Vec<(String, usize, usize)> = Vec::new();
                let mut errors: Vec<String> = Vec::new();
                if parsed.is_empty() {
                    errors.push("No ===TYPHOON_INGEST=== block found in the pasted text.".into());
                    let _ = msg_tx.send(BrokerMsg::IngestResearchResult {
                        per_symbol_added: per_symbol,
                        errors,
                    });
                    return;
                }
                let cache_opt = shared_cache_broker.read().ok().and_then(|g| g.clone());
                let conn = match cache_opt.as_ref().and_then(|c| c.connection().ok()) {
                    Some(c) => c,
                    None => {
                        errors.push("Cache unavailable — cannot persist ingested articles.".into());
                        let _ = msg_tx.send(BrokerMsg::IngestResearchResult {
                            per_symbol_added: per_symbol,
                            errors,
                        });
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
                                if wa.url.trim().is_empty() {
                                    continue;
                                }
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
                let _ = msg_tx.send(BrokerMsg::IngestResearchResult {
                    per_symbol_added: per_symbol,
                    errors,
                });
            });
        }
        BrokerCmd::FetchNewsMulti {
            symbol,
            marketaux_key,
            alpha_vantage_key,
            fmp_key,
            finnhub_key,
            cryptopanic_key,
        } => {
            use typhoon_engine::core::news;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        if news::news_cache_is_fresh(&conn, &symbol, 30 * 60, 1).unwrap_or(false) {
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
                                    let _ =
                                        msg_tx.send(BrokerMsg::Error(format!("News read: {e}")));
                                }
                            }
                            return;
                        }
                    }
                }
                let client = match reqwest::Client::builder()
                    .user_agent("Mozilla/5.0 (X11; Linux x86_64) TyphooN-Terminal/0.1")
                    .timeout(std::time::Duration::from_secs(25))
                    .build()
                {
                    Ok(c) => c,
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("News client: {e}")));
                        return;
                    }
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
                    &client, &symbol, &news_keys, cb,
                )
                .await
                {
                    Ok(v) => v,
                    Err(e) => {
                        let _ =
                            msg_tx.send(BrokerMsg::Error(format!("News fetch {}: {e}", symbol)));
                        return;
                    }
                };
                // DB work must run off the tokio worker to avoid holding &Connection across await.
                let sym_for_db = symbol.clone();
                let msg_tx_db = msg_tx.clone();
                let shared_cache_for_first = shared_cache_broker.clone();
                let _ = tokio::task::spawn_blocking(move || {
                    let Some(cache) = shared_cache_for_first.read().ok().and_then(|g| g.clone())
                    else {
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
                            let cached = news::mark_news_scraped(&conn, &sym_for_db).unwrap_or(n);
                            let _ = msg_tx_db.send(BrokerMsg::FundamentalsProgress(format!(
                                "news/{}: {} cached (deduped)",
                                sym_for_db, cached
                            )));
                        }
                        Err(e) => {
                            let _ = msg_tx_db.send(BrokerMsg::Error(format!("News upsert: {e}")));
                            return;
                        }
                    }
                    match news::get_news_by_symbol(&conn, &sym_for_db, 200) {
                        Ok(list) => {
                            let _ = msg_tx_db.send(BrokerMsg::NewsArticlesLoaded {
                                symbol: sym_for_db,
                                articles: list,
                            });
                        }
                        Err(e) => {
                            let _ = msg_tx_db.send(BrokerMsg::Error(format!("News read: {e}")));
                        }
                    }
                })
                .await;
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
                    let Some(cache) = shared_cache_hydrate.read().ok().and_then(|g| g.clone())
                    else {
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
                        let Ok(conn) = cache.connection() else {
                            return;
                        };
                        if let Ok(list) = typhoon_engine::core::news::get_news_by_symbol(
                            &conn,
                            &sym_for_hydrate,
                            200,
                        ) {
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
                    Ok(list) => {
                        let _ = msg_tx.send(BrokerMsg::NewsArticlesLoaded {
                            symbol,
                            articles: list,
                        });
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("Cached news read: {e}")));
                    }
                }
            });
        }
        BrokerCmd::HydrateNewsArticle {
            symbol,
            url_hash,
            url,
        } => {
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) else {
                    return;
                };
                let written = news_ingest::hydrate_one_url(cache.clone(), url_hash, url).await;
                // Always refresh the symbol's article list — even
                // a failure bumps body_fetch_attempts, which the
                // UI uses to decide whether to keep the "still
                // hydrating" placeholder or switch to "body
                // unavailable". A re-read keeps the placeholder
                // state in sync with the counter.
                let _ = written;
                let _ = tokio::task::spawn_blocking(move || {
                    let Ok(conn) = cache.connection() else {
                        return;
                    };
                    if let Ok(list) =
                        typhoon_engine::core::news::get_news_by_symbol(&conn, &symbol, 200)
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
                    Ok(list) => {
                        let _ = msg_tx.send(BrokerMsg::NewsArticlesLoaded {
                            symbol: String::new(),
                            articles: list,
                        });
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("News search: {e}")));
                    }
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
        _ => unreachable!("non-news command routed to news handler"),
    }
}
