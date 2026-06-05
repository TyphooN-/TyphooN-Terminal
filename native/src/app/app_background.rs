use super::*;

pub(super) fn spawn_darwin_background_refresh(
    app: &mut TyphooNApp,
    shared_cache: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
    shared_ftp_dir: std::sync::Arc<std::sync::Mutex<String>>,
    lan_client_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    importing_flag_bg: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    // Spawn background DARWIN data refresh thread (mpsc channel, capacity 1)
    {
        let (bg_tx, bg_rx) = std::sync::mpsc::channel::<BgDarwinData>();
        app.bg_rx = bg_rx;
        let shared_cache_bg = shared_cache.clone();
        let shared_ftp_dir_bg = shared_ftp_dir.clone();
        let lan_client_bg = lan_client_flag.clone();
        let _ = std::thread::Builder::new()
            .name("darwin-bg-refresh".to_string())
            .spawn(move || {
            let importing_flag_bg = importing_flag_bg;
            let mut full_refresh_done = false;
            let _last_full_refresh = std::time::Instant::now();
            let mut last_vacuum = std::time::Instant::now();
            const FULL_REFRESH_INTERVAL: std::time::Duration =
                std::time::Duration::from_secs(300); // 5 minutes
            const VACUUM_INTERVAL: std::time::Duration = std::time::Duration::from_secs(21600); // 6 hours
            // Persist data across loops so lightweight refreshes keep expensive Phase 2-8 results
            let mut data = BgDarwinData::default();
            let mut kv_hashes: std::collections::HashMap<String, u64> =
                std::collections::HashMap::new();
            // Macro for hash-based dedup of KV writes — skip put_kv if JSON identical to last write
            macro_rules! put_kv_if_changed {
                ($cache:expr, $key:expr, $json:expr, $hashes:expr) => {{
                    let hash = {
                        use std::hash::{Hash, Hasher};
                        let mut h = std::collections::hash_map::DefaultHasher::new();
                        $json.hash(&mut h);
                        h.finish()
                    };
                    let prev = $hashes.get($key).copied().unwrap_or(0);
                    if hash != prev {
                        let _ = $cache.put_kv($key, &$json);
                        $hashes.insert($key.to_string(), hash);
                    }
                }};
            }
            // BG thread opens its OWN read-only SQLite connection via SqliteCache::open_bg_read_connection().
            // This eliminates all Mutex contention with the UI thread's read_conn.
            // WAL mode allows unlimited concurrent readers — each connection reads
            // independently without blocking the others.
            let mut bg_conn: Option<typhoon_engine::core::cache::BgConnection> = None;
            let mut bg_cycle_count: u64 = 0;
            loop {
                std::thread::sleep(std::time::Duration::from_secs(3));

                if importing_flag_bg.load(std::sync::atomic::Ordering::Relaxed) {
                    continue;
                }

                let cache_arc = shared_cache_bg.read().ok().and_then(|g| g.clone());
                if cache_arc.is_none() {
                    // Log once, not every 3 seconds
                    static LOGGED: std::sync::atomic::AtomicBool =
                        std::sync::atomic::AtomicBool::new(false);
                    if !LOGGED.swap(true, std::sync::atomic::Ordering::Relaxed) {
                        tracing::info!("BG thread: waiting for cache to open...");
                    }
                    continue;
                }

                // Reopen BG read connection EVERY cycle so it always sees the latest
                // WAL writes from import_darwin_data, Mt5Sync, LAN sync, etc.
                // A persistent read-only connection may hold a stale WAL snapshot.
                // Opening is cheap (~1ms) compared to the 3s sleep between cycles.
                if let Some(ref cache) = cache_arc {
                    match cache.open_bg_read_connection() {
                        Ok(c) => {
                            bg_conn = Some(c);
                        }
                        Err(e) => {
                            tracing::warn!("BG thread: failed to open read connection: {e}");
                            continue;
                        }
                    }
                }
                let conn = match bg_conn.as_ref() {
                    Some(c) => c,
                    None => continue,
                };

                if let Some(ref cache) = cache_arc {
                    let phase_start = std::time::Instant::now();
                    tracing::trace!("BG thread: lightweight refresh...");

                    // Phase 1a: list accounts, filtering out user-deleted DARWINs
                    data.accounts = darwin::list_darwin_accounts(conn).unwrap_or_default();
                    // Blacklist filter: check KV for darwin:deleted:TICKER keys
                    // PERF: HashSet for O(1) retain check
                    let blacklist: std::collections::HashSet<String> = data
                        .accounts
                        .iter()
                        .filter(|a| {
                            cache
                                .get_kv(&format!("darwin:deleted:{}", a.darwin_ticker))
                                .ok()
                                .flatten()
                                .is_some()
                        })
                        .map(|a| a.darwin_ticker.clone())
                        .collect();
                    if !blacklist.is_empty() {
                        data.accounts
                            .retain(|a| !blacklist.contains(&a.darwin_ticker));
                    }
                    tracing::trace!(
                        "BG: Phase 1a done in {}ms",
                        phase_start.elapsed().as_millis()
                    );
                    let _ = bg_tx.send(data.clone());

                    // Phase 1b: table creation needs write conn (CREATE TABLE IF NOT EXISTS)
                    if let Ok(wconn) = cache.connection() {
                        let _ = darwin::create_darwin_tables(&wconn);
                        let _ = sec_filing::create_sec_tables(&wconn);
                        let _ = fundamentals::create_fundamentals_tables(&wconn);
                    }
                    // SEC data + cache stats — all via BG's own connection (growing database — no limit)
                    data.sec_filings = sec_filing::get_all_filings(conn).unwrap_or_default();
                    data.sec_alerts =
                        sec_filing::get_filing_alerts(conn, false).unwrap_or_default();
                    data.sec_content_stats = sec_filing::filing_content_stats(conn);
                    // Keep the BG summary consistent with Storage Manager's on-demand
                    // refresh path: user-visible rows + total on-disk footprint.
                    if let Ok(stats) = cache.stats() {
                        data.cache_stats = Some(stats);
                    }
                    let _ = bg_tx.send(data.clone());

                    let is_lan_client =
                        lan_client_bg.load(std::sync::atomic::Ordering::Relaxed);
                    if is_lan_client {
                        // LAN client: read ALL deal-dependent analytics from KV (server-computed).
                        // Local deal import produces wrong results — never compute from local deals.
                        if let Ok(Some(json)) = cache.get_kv("darwin:open_positions") {
                            if let Ok(pos) = serde_json::from_str::<
                                Vec<darwin::PortfolioOpenPosition>,
                            >(&json)
                            {
                                data.open_positions = pos;
                            }
                        }
                        if let Ok(Some(json)) = cache.get_kv("darwin:portfolio") {
                            if let Ok(v) = serde_json::from_str(&json) {
                                data.portfolio = Some(v);
                            }
                        }
                        if let Ok(Some(json)) = cache.get_kv("darwin:exposure") {
                            if let Ok(v) = serde_json::from_str(&json) {
                                data.exposure = v;
                            }
                        }
                        if let Ok(Some(json)) = cache.get_kv("darwin:correlations") {
                            if let Ok(v) = serde_json::from_str(&json) {
                                data.correlations = v;
                            }
                        }
                        if let Ok(Some(json)) = cache.get_kv("darwin:daily_returns") {
                            if let Ok(v) = serde_json::from_str(&json) {
                                data.daily_returns = v;
                            }
                        }
                    } else {
                        // Server/standalone: compute from deals, store to KV for LAN clients
                        data.portfolio = darwin::get_portfolio_summary(conn).ok();
                        data.daily_returns =
                            darwin::get_portfolio_daily_returns(conn).unwrap_or_default();
                        data.correlations =
                            darwin::get_darwin_correlations(conn).unwrap_or_default();
                        data.exposure =
                            darwin::get_portfolio_exposure(conn).unwrap_or_default();
                        data.open_positions =
                            darwin::get_portfolio_open_positions(conn).unwrap_or_default();
                        // Store to KV for LAN clients — only write if data changed (reduces KV churn + LAN sync bandwidth)
                        if let Ok(j) = serde_json::to_string(&data.open_positions) {
                            put_kv_if_changed!(cache, "darwin:open_positions", j, kv_hashes);
                        }
                        if let Some(ref p) = data.portfolio {
                            if let Ok(j) = serde_json::to_string(p) {
                                put_kv_if_changed!(cache, "darwin:portfolio", j, kv_hashes);
                            }
                        }
                        if let Ok(j) = serde_json::to_string(&data.exposure) {
                            put_kv_if_changed!(cache, "darwin:exposure", j, kv_hashes);
                        }
                        if let Ok(j) = serde_json::to_string(&data.correlations) {
                            put_kv_if_changed!(cache, "darwin:correlations", j, kv_hashes);
                        }
                        if let Ok(j) = serde_json::to_string(&data.daily_returns) {
                            put_kv_if_changed!(cache, "darwin:daily_returns", j, kv_hashes);
                        }
                    }
                    let _ = bg_tx.send(data.clone());

                    // Phase 1c: heavier queries — all use BG's own connection (zero contention with UI)
                    {
                        let t = std::time::Instant::now();
                        // detailed_stats via BG's own connection (prepare_cached: reuse parse across cycles)
                        let mut stmt = conn.prepare_cached(
                            "SELECT key, bar_count, timestamp, LENGTH(data) FROM bar_cache ORDER BY key"
                        ).ok();
                        if let Some(ref mut s) = stmt {
                            let rows = s.query_map([], |row| {
                                Ok((
                                    row.get::<_, String>(0)?,
                                    row.get::<_, i64>(1)?,
                                    row.get::<_, i64>(2)?,
                                    row.get::<_, i64>(3)?,
                                ))
                            });
                            if let Ok(rows) = rows {
                                let mut detailed_stats = Vec::new();
                                let mut cache_blob_sizes = std::collections::HashMap::new();
                                for (key, bar_count, timestamp, blob_bytes) in
                                    rows.filter_map(|r| r.ok())
                                {
                                    cache_blob_sizes.insert(key.clone(), blob_bytes);
                                    detailed_stats.push((key, bar_count, timestamp));
                                }
                                data.detailed_stats = detailed_stats;
                                data.cache_blob_sizes = cache_blob_sizes;
                            }
                        }
                        tracing::trace!(
                            "BG: detailed_stats {}ms (n={})",
                            t.elapsed().as_millis(),
                            data.detailed_stats.len()
                        );

                        // Incremental bar-range cache: first/last bar timestamps per key
                        // for Storage Manager and sync views. Previously limited to
                        // crypto prefixes and rebuilt from scratch every
                        // cycle; now covers every key in detailed_stats but only decompresses
                        // entries that are new or whose write_ts has advanced since the last
                        // extraction. Rate-limited to BAR_TS_CACHE_DECOMPRESSIONS_PER_CYCLE so
                        // a cold startup with ~7500 keys doesn't monopolise the BG loop
                        // (~600 µs per decompression → 300 ms at 500/cycle, full backfill
                        // in ~15 cycles ≈ 45 s).
                        const BAR_TS_CACHE_DECOMPRESSIONS_PER_CYCLE: usize = 500;
                        let mut ts_cache = std::mem::take(&mut data.bar_ts_cache);
                        let current_keys: std::collections::HashSet<&str> = data
                            .detailed_stats
                            .iter()
                            .map(|(k, _, _)| k.as_str())
                            .collect();
                        // Prune entries for keys that no longer exist (deleted / renamed).
                        ts_cache.retain(|k, _| current_keys.contains(k.as_str()));
                        let mut decompressions = 0usize;
                        let ts_t = std::time::Instant::now();
                        for (key, _, write_ts) in &data.detailed_stats {
                            if decompressions >= BAR_TS_CACHE_DECOMPRESSIONS_PER_CYCLE {
                                break;
                            }
                            let needs_refresh = match ts_cache.get(key) {
                                Some((_, _, cached_ts)) => *cached_ts != *write_ts,
                                None => true,
                            };
                            if !needs_refresh {
                                continue;
                            }
                            if let Some((first, last)) =
                                SqliteCache::get_bar_timestamp_range_with_conn(conn, key)
                            {
                                ts_cache.insert(key.clone(), (first, last, *write_ts));
                            }
                            decompressions += 1;
                        }
                        if decompressions > 0 {
                            tracing::trace!(
                                "BG: bar_ts_cache +{} in {}ms (size={}, total_keys={})",
                                decompressions,
                                ts_t.elapsed().as_millis(),
                                ts_cache.len(),
                                data.detailed_stats.len()
                            );
                        }
                        data.bar_ts_cache = ts_cache;
                    }
                    if is_lan_client {
                        // LAN client: load Phase 1c from KV (server-computed)
                        macro_rules! kv_load {
                            ($key:expr, $field:ident) => {
                                if let Ok(Some(j)) = cache.get_kv($key) {
                                    if let Ok(v) = serde_json::from_str(&j) {
                                        data.$field = v;
                                    }
                                }
                            };
                            ($key:expr, $field:ident, opt) => {
                                if let Ok(Some(j)) = cache.get_kv($key) {
                                    if let Ok(v) = serde_json::from_str(&j) {
                                        data.$field = Some(v);
                                    }
                                }
                            };
                        }
                        kv_load!("darwin:equity_curve", equity_curve);
                        kv_load!("darwin:trade_overlaps", trade_overlaps);
                        kv_load!("darwin:symbol_overlaps", symbol_overlaps);
                        kv_load!("darwin:sector_exposure", sector_exposure);
                        kv_load!("darwin:liquidity_risk", liquidity_risk);
                        kv_load!("darwin:exposure_treemap", exposure_treemap, opt);
                        kv_load!("darwin:timing_divergences", timing_divergences);
                        kv_load!("darwin:regime_performance", regime_performance);
                        kv_load!("darwin:darwin_alerts", darwin_alerts);
                    } else {
                        // Server: compute from deals, store to KV
                        data.equity_curve =
                            darwin::get_portfolio_equity_curve(conn).unwrap_or_default();
                        data.trade_overlaps =
                            darwin::get_trade_overlaps(conn).unwrap_or_default();
                        data.symbol_overlaps =
                            darwin::get_symbol_overlap(conn).unwrap_or_default();
                        data.sector_exposure =
                            darwin::get_sector_exposure(conn).unwrap_or_default();
                        data.liquidity_risk =
                            darwin::get_liquidity_risk(conn).unwrap_or_default();
                        data.exposure_treemap = darwin::get_exposure_treemap(conn).ok();
                        data.timing_divergences =
                            darwin::get_timing_divergences(conn).unwrap_or_default();
                        data.regime_performance =
                            darwin::get_regime_performance(conn).unwrap_or_default();
                        // Store Phase 1c to KV for LAN clients
                        macro_rules! kv_store {
                            ($key:expr, $val:expr) => {
                                if let Ok(j) = serde_json::to_string($val) {
                                    let _ = cache.put_kv($key, &j);
                                }
                            };
                        }
                        kv_store!("darwin:equity_curve", &data.equity_curve);
                        kv_store!("darwin:trade_overlaps", &data.trade_overlaps);
                        kv_store!("darwin:symbol_overlaps", &data.symbol_overlaps);
                        kv_store!("darwin:sector_exposure", &data.sector_exposure);
                        kv_store!("darwin:liquidity_risk", &data.liquidity_risk);
                        if let Some(ref v) = data.exposure_treemap {
                            kv_store!("darwin:exposure_treemap", v);
                        }
                        kv_store!("darwin:timing_divergences", &data.timing_divergences);
                        kv_store!("darwin:regime_performance", &data.regime_performance);
                    }
                    // Fundamentals come from research tables (synced via LAN) — query locally on both
                    data.all_fundamentals =
                        fundamentals::get_all_fundamentals(conn).unwrap_or_default();
                    data.upcoming_earnings =
                        fundamentals::get_upcoming_earnings(conn, 50).unwrap_or_default();
                    data.upcoming_dividends =
                        fundamentals::get_upcoming_dividends(conn, 50).unwrap_or_default();
                    // Darwinex specs for broker_scope filtering (loaded from __SPECS__ CSV in bar_cache)
                    data.darwinex_specs =
                        darwin::load_all_specs_parsed(conn).unwrap_or_default();
                    // PERF: normalize SEC filing tickers to uppercase once so per-frame
                    // scope filters can use O(1) `contains(ticker.as_str())` without allocating.
                    for f in &mut data.sec_filings {
                        f.ticker.make_ascii_uppercase();
                    }
                    tracing::trace!(
                        "BG: Phase 1c done in {}ms",
                        phase_start.elapsed().as_millis()
                    );
                    let _ = bg_tx.send(data.clone());

                    // Periodic DB maintenance: incremental_vacuum every 30 minutes.
                    // Reclaims freed pages from DELETEs/compaction without full VACUUM.
                    if last_vacuum.elapsed() >= VACUUM_INTERVAL {
                        let _ = cache.incremental_vacuum(500);
                        // LRU eviction: bar_cache soft limit = 500 MB.
                        // Skips entries newer than 7 days.
                        if let Ok((evicted, freed)) = cache.evict_lru(500 * 1024 * 1024) {
                            if evicted > 0 {
                                tracing::info!(
                                    "BG: LRU evicted {} bar_cache entries ({} MB freed)",
                                    evicted,
                                    freed / (1024 * 1024)
                                );
                            }
                        }
                        last_vacuum = std::time::Instant::now();
                        tracing::info!("BG: incremental_vacuum(500) completed");
                    }

                    // Phases 2-8: expensive DARWIN computation — once per startup only
                    // DARWIN trade data is static (imported from XLSX), no need to rescan repeatedly
                    let need_full_refresh = !full_refresh_done;
                    if !need_full_refresh {
                        // Lightweight refresh only — skip expensive phases
                        // Low-priority: backfill SEC filing content (15 per ~30s cycle).
                        // SEC permits 10 req/s for identified user agents
                        // (we set a User-Agent with contact) and the loop
                        // sleeps 250 ms per request, so 15/cycle stays an
                        // order of magnitude below their cap while still
                        // making real progress on the backlog.
                        // Spawns a short-lived thread with its own tokio runtime (same pattern as scrape thread).
                        if !is_lan_client && bg_cycle_count % 10 == 5 {
                            if let Ok(unfetched) = sec_filing::get_unfetched_filings(conn, 15) {
                                if !unfetched.is_empty() {
                                    if let Some(ref wr_cache) = cache_arc {
                                        let wr_cache = wr_cache.clone();
                                        let unfetched = unfetched.clone();
                                        let _ = std::thread::Builder::new()
                                            .name("typhoon-sec-filing-backfill".into())
                                            .spawn(move || {
                                                let rt =
                                                    match tokio::runtime::Builder::new_current_thread()
                                                        .enable_all()
                                                        .build()
                                                    {
                                                        Ok(rt) => rt,
                                                        Err(e) => {
                                                            tracing::warn!("SEC content backfill skipped: runtime build failed: {e}");
                                                            return;
                                                        }
                                                    };
                                                rt.block_on(async {
                                                    let client = reqwest::Client::builder()
                                                        .user_agent(sec_filing::SEC_EDGAR_USER_AGENT)
                                                        .timeout(std::time::Duration::from_secs(10))
                                                        .build()
                                                        .unwrap_or_default();
                                                    let mut fetched = 0usize;
                                                    let mut failed = 0usize;
                                                    let mut consecutive_forbidden = 0usize;

                                                    let wr_conn = match wr_cache.connection() {
                                                        Ok(conn) => conn,
                                                        Err(e) => {
                                                            tracing::warn!("SEC content backfill skipped: DB connection failed: {e}");
                                                            return;
                                                        }
                                                    };
                                                    let keywords = sec_filing::get_keywords(&wr_conn)
                                                        .unwrap_or_default();

                                                    for filing in &unfetched {
                                                        if filing.url.is_empty() {
                                                            let _ = sec_filing::mark_filing_content_fetch_failed(
                                                                &wr_conn,
                                                                &filing.accession_number,
                                                                "empty filing URL",
                                                            );
                                                            failed += 1;
                                                            continue;
                                                        }
                                                        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                                                        let resp = match client
                                                            .get(&filing.url)
                                                            .header(
                                                                "Accept",
                                                                "text/html,application/xhtml+xml,application/xml",
                                                            )
                                                            .header("Accept-Encoding", "identity")
                                                            .send()
                                                            .await
                                                        {
                                                            Ok(resp) => resp,
                                                            Err(e) => {
                                                                let _ = sec_filing::mark_filing_content_fetch_failed(
                                                                    &wr_conn,
                                                                    &filing.accession_number,
                                                                    &format!("request failed: {e}"),
                                                                );
                                                                failed += 1;
                                                                continue;
                                                            }
                                                        };
                                                        let status = resp.status();
                                                        if !status.is_success() {
                                                            let _ = sec_filing::mark_filing_content_fetch_failed(
                                                                &wr_conn,
                                                                &filing.accession_number,
                                                                &format!("HTTP {status}"),
                                                            );
                                                            failed += 1;
                                                            if status == reqwest::StatusCode::FORBIDDEN {
                                                                consecutive_forbidden += 1;
                                                                if consecutive_forbidden >= 3 {
                                                                    tracing::warn!(
                                                                        "SEC content backfill paused: {} consecutive HTTP 403 responses; check EDGAR User-Agent / block status",
                                                                        consecutive_forbidden
                                                                    );
                                                                    break;
                                                                }
                                                            } else {
                                                                consecutive_forbidden = 0;
                                                            }
                                                            continue;
                                                        }
                                                        consecutive_forbidden = 0;
                                                        let html = match resp.text().await {
                                                            Ok(html) => html,
                                                            Err(e) => {
                                                                let _ = sec_filing::mark_filing_content_fetch_failed(
                                                                    &wr_conn,
                                                                    &filing.accession_number,
                                                                    &format!("read failed: {e}"),
                                                                );
                                                                failed += 1;
                                                                continue;
                                                            }
                                                        };
                                                        let content = sec_filing::strip_html_to_text(&html);
                                                        match sec_filing::store_filing_content(
                                                            &wr_conn,
                                                            &filing.accession_number,
                                                            &filing.ticker,
                                                            &filing.form_type,
                                                            &filing.company_name,
                                                            &content,
                                                        ) {
                                                            Ok(()) => {
                                                                fetched += 1;
                                                                // Check keyword watchlist for alerts without re-querying the DB per filing.
                                                                let matched_kw = sec_filing::check_keywords_in(&keywords, &content);
                                                                for kw in &matched_kw {
                                                                    let msg = format!(
                                                                        "Keyword '{}' found in {} {} ({})",
                                                                        kw, filing.ticker, filing.form_type, filing.filing_date
                                                                    );
                                                                    let _ = wr_conn.execute(
                                                                        "INSERT INTO sec_filing_alerts (ticker, alert_type, message, filing_accession, importance, created_at, dismissed)
                                                                         SELECT ?1, 'KEYWORD_MATCH', ?2, ?3, 70, ?4, FALSE
                                                                         WHERE NOT EXISTS (SELECT 1 FROM sec_filing_alerts WHERE ticker=?1 AND alert_type='KEYWORD_MATCH' AND filing_accession=?3)",
                                                                        [&filing.ticker, &msg, &filing.accession_number, &chrono::Utc::now().timestamp().to_string()],
                                                                    );
                                                                }
                                                            }
                                                            Err(e) => {
                                                                let _ = sec_filing::mark_filing_content_fetch_failed(
                                                                    &wr_conn,
                                                                    &filing.accession_number,
                                                                    &e,
                                                                );
                                                                failed += 1;
                                                            }
                                                        }
                                                    }
                                                    if fetched > 0 {
                                                        tracing::debug!(
                                                            "BG: SEC content backfill: {} stored, {} failed",
                                                            fetched,
                                                            failed
                                                        );
                                                    } else if failed > 0 {
                                                        tracing::debug!(
                                                            "BG: SEC content backfill: {} stored, {} failed",
                                                            fetched,
                                                            failed
                                                        );
                                                    }
                                                });
                                        });
                                    }
                                }
                            }
                        }
                        bg_cycle_count += 1;
                        let _ = bg_tx.send(data.clone());
                        continue;
                    }

                    // Phase 2: pure computation from daily_returns
                    // LAN client loads these pre-computed from KV in Phase 3-4 block below.
                    // Server computes here and stores to KV later.
                    if !is_lan_client && !data.daily_returns.is_empty() {
                        data.var_stats = Some(darwin::compute_var(&data.daily_returns));
                        data.monte_carlo =
                            Some(darwin::monte_carlo_var(&data.daily_returns, 252, 1000));
                        data.rolling_var = darwin::get_rolling_var(&data.daily_returns, 30);
                        data.var_forecast =
                            Some(darwin::forecast_var(&data.daily_returns, 6.5));
                        data.conditional_var =
                            darwin::compute_conditional_var(&data.daily_returns);
                        data.market_regime =
                            Some(darwin::detect_market_regime(&data.daily_returns));
                        data.tail_risk = Some(darwin::compute_tail_risk(&data.daily_returns));
                        data.seasonal_analysis =
                            darwin::get_seasonal_analysis(&data.daily_returns);
                    }

                    // Phase 3-4: deal-dependent analytics
                    if is_lan_client {
                        // LAN client: read ALL from KV (server-computed). Zero local deal queries.
                        macro_rules! kv_load {
                            ($key:expr, $field:ident) => {
                                if let Ok(Some(j)) = cache.get_kv($key) {
                                    if let Ok(v) = serde_json::from_str(&j) {
                                        data.$field = v;
                                    }
                                }
                            };
                            ($key:expr, $field:ident, opt) => {
                                if let Ok(Some(j)) = cache.get_kv($key) {
                                    if let Ok(v) = serde_json::from_str(&j) {
                                        data.$field = Some(v);
                                    }
                                }
                            };
                        }
                        kv_load!("darwin:optimal_allocation", optimal_allocation);
                        kv_load!("darwin:rebalance", rebalance, opt);
                        kv_load!("darwin:stress_tests", stress_tests);
                        kv_load!("darwin:margin_call_sim", margin_call_sim, opt);
                        kv_load!("darwin:drawdown_dashboard", drawdown_dashboard, opt);
                        kv_load!("darwin:drawdown_attribution", drawdown_attribution);
                        kv_load!("darwin:risk_budget", risk_budget);
                        kv_load!("darwin:signal_decay", signal_decay);
                        kv_load!("darwin:var_multipliers", var_multipliers);
                        kv_load!("darwin:floating_equity", floating_equity, opt);
                        kv_load!("darwin:per_darwin_var", per_darwin_var);
                        kv_load!("darwin:var_stats", var_stats, opt);
                        kv_load!("darwin:monte_carlo", monte_carlo, opt);
                        kv_load!("darwin:rolling_var", rolling_var);
                        kv_load!("darwin:var_forecast", var_forecast, opt);
                        kv_load!("darwin:conditional_var", conditional_var);
                        kv_load!("darwin:market_regime", market_regime, opt);
                        kv_load!("darwin:tail_risk", tail_risk, opt);
                        kv_load!("darwin:seasonal_analysis", seasonal_analysis);
                        kv_load!("darwin:rolling_correlations", rolling_correlations);
                        kv_load!("darwin:low_correlation_darwins", low_correlation_darwins);
                    } else {
                        // Server/standalone: compute from deals, store ALL to KV for clients
                        data.optimal_allocation =
                            darwin::compute_optimal_allocation(conn).unwrap_or_default();
                        {
                            let prices = std::collections::HashMap::new();
                            data.rebalance =
                                darwin::compute_rebalance_suggestions(conn, &prices).ok();
                        }
                        data.stress_tests = darwin::run_stress_tests(conn).unwrap_or_default();
                        data.margin_call_sim = darwin::simulate_margin_call(conn).ok();
                        data.drawdown_dashboard =
                            darwin::get_combined_drawdown_dashboard(conn, 5).ok();
                        data.drawdown_attribution =
                            darwin::compute_drawdown_attribution(conn).unwrap_or_default();
                        data.risk_budget =
                            darwin::compute_risk_budget(conn).unwrap_or_default();
                        {
                            let mut decays = Vec::new();
                            for acct in &data.accounts {
                                if let Ok(decay) =
                                    darwin::compute_signal_decay(conn, &acct.darwin_ticker, 90)
                                {
                                    decays.push(decay);
                                }
                            }
                            data.signal_decay = decays;
                        }
                        data.var_multipliers =
                            darwin::compute_var_multipliers(conn).unwrap_or_default();
                        {
                            let prices = std::collections::HashMap::new();
                            data.floating_equity =
                                darwin::compute_floating_equity(conn, &prices).ok();
                        }
                        {
                            let mut per_var = Vec::new();
                            for acct in &data.accounts {
                                if let Ok(daily) =
                                    darwin::get_daily_returns(conn, &acct.darwin_ticker)
                                {
                                    if !daily.is_empty() {
                                        per_var.push((
                                            acct.darwin_ticker.clone(),
                                            darwin::compute_var(&daily),
                                        ));
                                    }
                                }
                            }
                            data.per_darwin_var = per_var;
                        }
                        // Rolling correlation: O(n^2) pairs of DARWINs
                        {
                            let tickers: Vec<String> = data
                                .accounts
                                .iter()
                                .map(|a| a.darwin_ticker.clone())
                                .collect();
                            let mut corrs = Vec::new();
                            for i in 0..tickers.len() {
                                for j in (i + 1)..tickers.len() {
                                    if let Ok(rc) = darwin::compute_rolling_correlation(
                                        conn,
                                        &tickers[i],
                                        &tickers[j],
                                        45,
                                    ) {
                                        corrs.push(rc);
                                    }
                                }
                            }
                            data.rolling_correlations = corrs;
                        }
                        // Low-correlation DARWIN finder (FTP scan)
                        {
                            let ftp_dir = shared_ftp_dir_bg
                                .lock()
                                .ok()
                                .map(|d| d.clone())
                                .unwrap_or_default();
                            if !ftp_dir.is_empty() {
                                data.low_correlation_darwins =
                                    darwin::find_low_correlation_darwins(conn, &ftp_dir, 20)
                                        .unwrap_or_default();
                            }
                        }
                        // Store ALL analytics to KV for LAN clients
                        macro_rules! kv_store {
                            ($key:expr, $val:expr) => {
                                if let Ok(j) = serde_json::to_string($val) {
                                    let _ = cache.put_kv($key, &j);
                                }
                            };
                        }
                        kv_store!("darwin:optimal_allocation", &data.optimal_allocation);
                        if let Some(ref v) = data.rebalance {
                            kv_store!("darwin:rebalance", v);
                        }
                        kv_store!("darwin:stress_tests", &data.stress_tests);
                        if let Some(ref v) = data.margin_call_sim {
                            kv_store!("darwin:margin_call_sim", v);
                        }
                        if let Some(ref v) = data.drawdown_dashboard {
                            kv_store!("darwin:drawdown_dashboard", v);
                        }
                        kv_store!("darwin:drawdown_attribution", &data.drawdown_attribution);
                        kv_store!("darwin:risk_budget", &data.risk_budget);
                        kv_store!("darwin:signal_decay", &data.signal_decay);
                        kv_store!("darwin:var_multipliers", &data.var_multipliers);
                        if let Some(ref v) = data.floating_equity {
                            kv_store!("darwin:floating_equity", v);
                        }
                        kv_store!("darwin:per_darwin_var", &data.per_darwin_var);
                        if let Some(ref v) = data.var_stats {
                            kv_store!("darwin:var_stats", v);
                        }
                        if let Some(ref v) = data.monte_carlo {
                            kv_store!("darwin:monte_carlo", v);
                        }
                        kv_store!("darwin:rolling_var", &data.rolling_var);
                        if let Some(ref v) = data.var_forecast {
                            kv_store!("darwin:var_forecast", v);
                        }
                        kv_store!("darwin:conditional_var", &data.conditional_var);
                        if let Some(ref v) = data.market_regime {
                            kv_store!("darwin:market_regime", v);
                        }
                        if let Some(ref v) = data.tail_risk {
                            kv_store!("darwin:tail_risk", v);
                        }
                        kv_store!("darwin:seasonal_analysis", &data.seasonal_analysis);
                        kv_store!("darwin:rolling_correlations", &data.rolling_correlations);
                        kv_store!(
                            "darwin:low_correlation_darwins",
                            &data.low_correlation_darwins
                        );
                    }

                    // Phase 5: per-account detailed analytics (DARWIN Accounts window)
                    if is_lan_client {
                        // LAN client: load from KV, filter out deleted DARWINs
                        if let Ok(Some(j)) = cache.get_kv("darwin:account_details") {
                            if let Ok(mut v) =
                                serde_json::from_str::<Vec<AccountDetailCache>>(&j)
                            {
                                v.retain(|d| {
                                    cache
                                        .get_kv(&format!("darwin:deleted:{}", d.ticker))
                                        .ok()
                                        .flatten()
                                        .is_none()
                                });
                                data.account_details = v;
                            }
                        }
                    } else {
                        let accounts_snapshot: Vec<String> = data
                            .accounts
                            .iter()
                            .map(|a| a.darwin_ticker.clone())
                            .collect();
                        let daily_returns_ref = &data.daily_returns;
                        let ftp_dir_str = shared_ftp_dir_bg
                            .lock()
                            .ok()
                            .map(|d| d.clone())
                            .unwrap_or_default();

                        let details: Vec<AccountDetailCache> = std::thread::scope(|s| {
                            let handles: Vec<_> = accounts_snapshot
                                .iter()
                                .map(|ticker| {
                                    let cache_ref = &cache;
                                    let ticker = ticker.clone();
                                    let ftp_dir = ftp_dir_str.clone();
                                    let daily_rets = daily_returns_ref;
                                    s.spawn(move || {
                                        let t = std::time::Instant::now();
                                        let mut det = AccountDetailCache {
                                            ticker: ticker.clone(),
                                            ..Default::default()
                                        };
                                        // Each scoped thread opens its own read connection — zero contention with UI
                                        if let Ok(ref conn) =
                                            cache_ref.open_bg_read_connection()
                                        {
                                            det.summary =
                                                darwin::get_darwin_summary(&conn, &ticker).ok();
                                            det.streaks =
                                                darwin::get_streak_analysis(&conn, &ticker)
                                                    .ok();
                                            det.hourly_pnl =
                                                darwin::get_hourly_pnl(&conn, &ticker)
                                                    .unwrap_or_default();
                                            det.equity_curve =
                                                darwin::get_darwin_equity_curve(&conn, &ticker)
                                                    .unwrap_or_default();
                                            det.pnl_by_symbol =
                                                darwin::get_darwin_pnl_by_symbol(
                                                    &conn, &ticker,
                                                )
                                                .unwrap_or_default();
                                            det.day_of_week =
                                                darwin::get_day_of_week_pnl(&conn, &ticker)
                                                    .unwrap_or_default();
                                            det.hold_time =
                                                darwin::get_hold_time_stats(&conn, &ticker)
                                                    .ok();
                                            det.kelly =
                                                darwin::compute_kelly(&conn, &ticker).ok();
                                            det.cost_analysis =
                                                darwin::get_cost_analysis(&conn, &ticker).ok();
                                            det.dscore =
                                                darwin::estimate_dscore(&conn, &ticker).ok();
                                            det.slippage =
                                                darwin::analyze_slippage(&conn, &ticker).ok();
                                            det.mae_mfe =
                                                darwin::estimate_mae_mfe(&conn, &ticker).ok();
                                            det.sizing_efficiency =
                                                darwin::get_sizing_efficiency(&conn, &ticker)
                                                    .unwrap_or_default();
                                            det.symbol_rotation =
                                                darwin::get_symbol_rotation(&conn, &ticker)
                                                    .unwrap_or_default();
                                            det.open_positions =
                                                darwin::get_darwin_open_positions(
                                                    &conn, &ticker,
                                                )
                                                .unwrap_or_default();
                                            det.pyramiding =
                                                darwin::analyze_pyramiding(&conn, &ticker)
                                                    .unwrap_or_default();
                                            det.bursts =
                                                darwin::detect_trading_bursts(&conn, &ticker)
                                                    .unwrap_or_default();
                                            det.autocorrelation =
                                                darwin::compute_trade_autocorrelation(
                                                    &conn, &ticker,
                                                )
                                                .ok();
                                            det.recent_deals = darwin::get_darwin_deals(
                                                &conn, &ticker, None, None,
                                            )
                                            .unwrap_or_default();
                                            det.closed_positions =
                                                darwin::get_darwin_positions(
                                                    &conn, &ticker, None, None,
                                                )
                                                .unwrap_or_default();
                                            det.equity_snapshots =
                                                darwin::get_equity_history(&conn, &ticker, 10)
                                                    .unwrap_or_default();
                                            det.benchmark = darwin::compare_to_benchmark(
                                                &conn, &ticker, daily_rets,
                                            )
                                            .ok();
                                            det.sector_classification = det
                                                .pnl_by_symbol
                                                .iter()
                                                .take(10)
                                                .map(|(sym, _, _, _, _)| {
                                                    (
                                                        sym.clone(),
                                                        darwin::classify_sector(sym)
                                                            .to_string(),
                                                    )
                                                })
                                                .collect();
                                            if let Ok(daily) =
                                                darwin::get_daily_returns(&conn, &ticker)
                                            {
                                                if !daily.is_empty() {
                                                    det.var_stats =
                                                        Some(darwin::compute_var(&daily));
                                                    det.monthly_returns =
                                                        darwin::get_monthly_returns(&daily);
                                                    det.rolling_var =
                                                        darwin::get_rolling_var(&daily, 30);
                                                    det.cagr = darwin::compute_cagr(&daily);
                                                    det.recovery_factor =
                                                        darwin::compute_recovery_factor(&daily);
                                                    det.dd_duration =
                                                        darwin::compute_drawdown_duration(
                                                            &daily,
                                                        );
                                                    det.daily_returns = daily;
                                                }
                                            }
                                        }
                                        // Equity snapshot write uses the write connection (separate lock)
                                        if let Some(ref summary) = det.summary {
                                            if let Ok(conn) = cache_ref.connection() {
                                                let _ = darwin::record_equity_snapshot(
                                                    &conn,
                                                    &ticker,
                                                    summary.final_balance,
                                                    0.0,
                                                    0,
                                                );
                                            }
                                        }
                                        // FTP DARWIN quote data
                                        if !ftp_dir.is_empty() {
                                            let ftp_path = std::path::Path::new(&ftp_dir);
                                            if ftp_path.is_dir() {
                                                if let Ok(returns) =
                                                    darwin_ftp::read_return_file(
                                                        ftp_path, &ticker,
                                                    )
                                                {
                                                    let summary =
                                                        darwin_ftp::compute_return_summary(
                                                            &ticker, &returns,
                                                        );
                                                    let eq: Vec<(f64, f64)> = returns
                                                        .iter()
                                                        .enumerate()
                                                        .filter_map(|(i, r)| {
                                                            r.cumulative_returns
                                                                .last()
                                                                .map(|&v| (i as f64, v * 100.0))
                                                        })
                                                        .collect();
                                                    let mut peak = 100.0_f64;
                                                    let dd: Vec<(f64, f64)> = eq
                                                        .iter()
                                                        .map(|&(x, price)| {
                                                            if price > peak {
                                                                peak = price;
                                                            }
                                                            let dd_pct = if peak > 0.0 {
                                                                (peak - price) / peak * 100.0
                                                            } else {
                                                                0.0
                                                            };
                                                            (x, -dd_pct)
                                                        })
                                                        .collect();
                                                    det.ftp_summary = Some(summary);
                                                    det.ftp_equity_curve = eq;
                                                    det.ftp_drawdown_curve = dd;
                                                }
                                            }
                                        }
                                        // Divergence index
                                        if !det.ftp_equity_curve.is_empty() {
                                            if let Ok(conn) = cache_ref.connection() {
                                                if let Ok(daily) =
                                                    darwin::get_daily_returns(&conn, &ticker)
                                                {
                                                    if !daily.is_empty() {
                                                        det.divergence =
                                                            darwin::compute_divergence_index(
                                                                &daily,
                                                                &det.ftp_equity_curve,
                                                            );
                                                    }
                                                }
                                            }
                                        }
                                        // Performance attribution (per-symbol P&L contribution)
                                        if let Ok(ref conn) =
                                            cache_ref.open_bg_read_connection()
                                        {
                                            det.performance_attribution =
                                                darwin::compute_performance_attribution(
                                                    conn, &ticker,
                                                )
                                                .unwrap_or_default();
                                        }
                                        // Tax lots (current year)
                                        if let Ok(ref conn) =
                                            cache_ref.open_bg_read_connection()
                                        {
                                            let year = chrono::Utc::now().year();
                                            det.tax_lots =
                                                darwin::compute_tax_lots(conn, &ticker, year)
                                                    .ok();
                                        }
                                        // D-Score components from FTP
                                        if !ftp_dir.is_empty() {
                                            det.dscore_components =
                                                darwin::get_dscore_components(
                                                    &ftp_dir, &ticker,
                                                )
                                                .ok();
                                            // Investor flow (raw) + investment velocity
                                            if let Ok(flow) =
                                                darwin::get_investor_flow(&ftp_dir, &ticker)
                                            {
                                                det.investor_flow = flow.clone();
                                                det.investment_velocity =
                                                    darwin::compute_investment_velocity(&flow);
                                            }
                                        }
                                        tracing::info!(
                                            "BG: account {} detail {}ms",
                                            ticker,
                                            t.elapsed().as_millis()
                                        );
                                        det
                                    })
                                })
                                .collect();
                            handles.into_iter().filter_map(|h| h.join().ok()).collect()
                        });

                        data.account_details = details;
                        // Store to KV for LAN clients — only write if changed
                        if let Ok(j) = serde_json::to_string(&data.account_details) {
                            put_kv_if_changed!(cache, "darwin:account_details", j, kv_hashes);
                        }
                        let _ = bg_tx.send(data.clone());
                    }

                    // Phase 6: DARWIN risk alerts + Phase 8: insider trades
                    if !is_lan_client {
                        data.darwin_alerts = darwin::check_alerts(conn).unwrap_or_default();
                        {
                            // Load ALL insider trades (growing database — no date cutoff)
                            let all_trades =
                                sec_filing::get_all_insider_trades(conn).unwrap_or_default();
                            let mut insider_map: std::collections::HashMap<
                                String,
                                Vec<sec_filing::InsiderTrade>,
                            > = std::collections::HashMap::new();
                            for trade in all_trades {
                                // PERF: uppercase key so per-frame filters can skip the alloc.
                                let mut key = trade.ticker.clone();
                                key.make_ascii_uppercase();
                                insider_map.entry(key).or_default().push(trade);
                            }
                            data.insider_trades = insider_map;
                        }
                        // Store to KV for LAN clients — only write if changed
                        if let Ok(j) = serde_json::to_string(&data.darwin_alerts) {
                            put_kv_if_changed!(cache, "darwin:alerts", j, kv_hashes);
                        }
                        if let Ok(j) = serde_json::to_string(&data.insider_trades) {
                            put_kv_if_changed!(cache, "darwin:insider_trades", j, kv_hashes);
                        }
                    } else {
                        // LAN client: load from KV
                        if let Ok(Some(j)) = cache.get_kv("darwin:alerts") {
                            if let Ok(v) = serde_json::from_str(&j) {
                                data.darwin_alerts = v;
                            }
                        }
                        if let Ok(Some(j)) = cache.get_kv("darwin:insider_trades") {
                            if let Ok(v) = serde_json::from_str(&j) {
                                data.insider_trades = v;
                            }
                        }
                    }

                    // Mark full refresh complete
                    full_refresh_done = true;
                    // DARWIN data is static — no periodic refresh needed
                    tracing::info!(
                        "BG: full refresh complete in {}ms — next in {}s",
                        phase_start.elapsed().as_millis(),
                        FULL_REFRESH_INTERVAL.as_secs()
                    );

                    // Send to UI thread (non-blocking — drops if channel full)
                    let _ = bg_tx.send(data.clone());
                }
            }
        });
    }
}
