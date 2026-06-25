use std::sync::Arc;

use chrono::Datelike;
use typhoon_engine::broker::alpaca::AlpacaBroker;
use typhoon_engine::broker::protocol::{
    BrokerCmd, BrokerMsg, is_fundamentals_provider_coverage_gap,
    normalize_fundamentals_scrape_symbol, should_emit_fundamentals_scrape_progress,
};
use typhoon_engine::core::cache::SqliteCache;
use typhoon_engine::core::fundamentals;

pub async fn handle_fundamentals_command(
    cmd: BrokerCmd,
    broker: Option<&AlpacaBroker>,
    broker_msg_tx: &tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    shared_cache_broker: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
) {
    match cmd {
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
                if let Some(b) = broker {
                    match b.get_all_assets().await {
                        Ok(assets) => {
                            let syms: Vec<String> = assets
                                .iter()
                                .filter(|a| a.asset_class == "us_equity" && a.tradable)
                                .filter_map(|a| normalize_fundamentals_scrape_symbol(&a.symbol))
                                .collect();
                            let _ = broker_msg_tx.send(BrokerMsg::FundamentalsProgress(format!(
                                "Alpaca: {} stock tickers",
                                syms.len()
                            )));
                            extra_tickers.extend(syms);
                        }
                        Err(e) => {
                            let _ = broker_msg_tx.send(BrokerMsg::FundamentalsProgress(format!(
                                "Alpaca symbols failed: {}",
                                e
                            )));
                        }
                    }
                } else {
                    let _ = broker_msg_tx.send(BrokerMsg::FundamentalsProgress(
                        "Alpaca not connected — skipping".into(),
                    ));
                }
            }
            if use_kraken {
                let syms: Vec<String> =
                    typhoon_chart_ui::cache_keys::normalize_kraken_equity_symbol_list(
                        kraken_equity_symbols.iter(),
                    )
                    .into_iter()
                    .filter_map(|sym| normalize_fundamentals_scrape_symbol(&sym))
                    .collect();
                if syms.is_empty() {
                    let _ = broker_msg_tx.send(
                        BrokerMsg::FundamentalsProgress(
                            "Kraken equities catalog not loaded — fundamentals scrape skipped for Kraken".into(),
                        ),
                    );
                } else {
                    let _ = broker_msg_tx.send(BrokerMsg::FundamentalsProgress(format!(
                        "Kraken equities: {} catalog tickers",
                        syms.len()
                    )));
                    extra_tickers.extend(syms);
                }
            }
            let msg_tx = broker_msg_tx.clone();
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
            let msg_tx = broker_msg_tx.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            let _ = std::thread::Builder::new()
                .name("typhoon-fundamentals-scrape-one".into())
                .spawn(move || {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .unwrap_or_else(|e| {
                            eprintln!("FATAL: tokio runtime init failed: {e}");
                            std::process::exit(1);
                        });
                    rt.block_on(async {
                        match shared_cache_broker
                            .read()
                            .ok()
                            .and_then(|g| g.clone())
                            .ok_or("Cache not ready".to_string())
                        {
                            Ok(cache) => {
                                if let Ok(conn) = cache.connection() {
                                    let _ = fundamentals::create_fundamentals_tables(&conn);
                                    let session = match fundamentals::YahooSession::new().await {
                                        Ok(s) => s,
                                        Err(e) => {
                                            let _ = msg_tx.send(BrokerMsg::Error(format!(
                                                "Yahoo auth failed: {}",
                                                e
                                            )));
                                            return;
                                        }
                                    };
                                    match fundamentals::scrape_ticker(&session, &conn, &ticker)
                                        .await
                                    {
                                        Ok(_f) => {
                                            let _ = msg_tx.send(BrokerMsg::FundamentalsProgress(
                                                format!("Scraped {}: OK", ticker),
                                            ));
                                        }
                                        Err(e) => {
                                            let _ = msg_tx.send(BrokerMsg::FundamentalsProgress(
                                                format!("Scraped {}: FAIL — {}", ticker, e),
                                            ));
                                        }
                                    }
                                } else {
                                    let _ = msg_tx.send(BrokerMsg::Error(
                                        "Fundamentals: could not get DB connection".into(),
                                    ));
                                }
                            }
                            Err(e) => {
                                let _ = msg_tx.send(BrokerMsg::Error(format!(
                                    "Fundamentals: could not open cache: {}",
                                    e
                                )));
                            }
                        }
                    });
                });
        }
        BrokerCmd::ResearchScrape {
            use_alpaca,
            finnhub_key,
            fmp_key,
        } => {
            let mut extra_tickers: Vec<String> = Vec::new();
            if use_alpaca {
                if let Some(b) = broker {
                    if let Ok(assets) = b.get_all_assets().await {
                        extra_tickers.extend(
                            assets
                                .iter()
                                .filter(|a| a.asset_class == "us_equity" && a.tradable)
                                .map(|a| a.symbol.clone()),
                        );
                    }
                }
            }
            let msg_tx = broker_msg_tx.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            let _ = std::thread::Builder::new()
                .name("typhoon-research-scrape".into())
                .spawn(move || {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .unwrap_or_else(|e| {
                            eprintln!("FATAL: tokio runtime init failed: {e}");
                            std::process::exit(1);
                        });
                    rt.block_on(async {
                        use typhoon_engine::core::research;
                        match shared_cache_broker
                            .read()
                            .ok()
                            .and_then(|g| g.clone())
                            .ok_or("Cache not ready".to_string())
                        {
                            Ok(cache) => {
                                let mut all_tickers: std::collections::HashSet<String> =
                                    std::collections::HashSet::new();
                                all_tickers.extend(extra_tickers);
                                let mut tickers: Vec<String> = all_tickers.into_iter().collect();
                                tickers.sort();
                                if let Ok(conn) = cache.connection() {
                                    let _ = research::create_research_tables(&conn);
                                }
                                let _ = msg_tx.send(BrokerMsg::FundamentalsProgress(format!(
                                    "Research scrape: {} tickers queued",
                                    tickers.len()
                                )));
                                let client = reqwest::Client::builder()
                                    .user_agent("TyphooN-Terminal/1.0")
                                    .timeout(std::time::Duration::from_secs(15))
                                    .build()
                                    .unwrap_or_default();
                                let total = tickers.len();
                                let mut done = 0usize;
                                for ticker in &tickers {
                                    let conn_result = cache.connection();
                                    if let Ok(conn) = conn_result {
                                        let tx = msg_tx.clone();
                                        let _ = research::scrape_and_cache_symbol(
                                            &client,
                                            &conn,
                                            ticker,
                                            &finnhub_key,
                                            &fmp_key,
                                            |note| {
                                                let _ = tx.send(BrokerMsg::FundamentalsProgress(
                                                    note.to_string(),
                                                ));
                                            },
                                        )
                                        .await;
                                    }
                                    done += 1;
                                    if done % 10 == 0 || done == total {
                                        let _ = msg_tx.send(BrokerMsg::FundamentalsProgress(
                                            format!("Research scrape: {}/{}", done, total),
                                        ));
                                    }
                                }
                                let _ = msg_tx.send(BrokerMsg::FundamentalsProgress(format!(
                                    "Research scrape complete: {} tickers processed",
                                    total
                                )));
                            }
                            Err(e) => {
                                let _ = msg_tx.send(BrokerMsg::Error(format!(
                                    "Research scrape: cache not ready: {}",
                                    e
                                )));
                            }
                        }
                    });
                });
        }
        _ => unreachable!("non-fundamentals command routed to fundamentals handler"),
    }
}
