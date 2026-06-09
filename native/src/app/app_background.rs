use super::*;

pub(super) fn spawn_background_refresh(
    app: &mut TyphooNApp,
    shared_cache: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
    lan_client_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    importing_flag_bg: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    // Spawn the background data-refresh thread (SEC filings, fundamentals,
    // cache/storage stats, insider trades). mpsc channel, capacity unbounded.
    {
        let (bg_tx, bg_rx) = std::sync::mpsc::channel::<BgData>();
        app.bg_rx = bg_rx;
        let shared_cache_bg = shared_cache.clone();
        let lan_client_bg = lan_client_flag.clone();
        let _ = std::thread::Builder::new()
            .name("typhoon-bg-refresh".to_string())
            .spawn(move || {
            let importing_flag_bg = importing_flag_bg;
            let mut full_refresh_done = false;
            let mut last_vacuum = std::time::Instant::now();
            const FULL_REFRESH_INTERVAL: std::time::Duration =
                std::time::Duration::from_secs(300); // 5 minutes
            const VACUUM_INTERVAL: std::time::Duration = std::time::Duration::from_secs(21600); // 6 hours
            // Persist data across loops so lightweight refreshes keep prior results.
            let mut data = BgData::default();
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
                // WAL writes from Mt5Sync, LAN sync, broker fetches, etc.
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

                    // Phase 1b: table creation needs write conn (CREATE TABLE IF NOT EXISTS)
                    if let Ok(wconn) = cache.connection() {
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
                    // Fundamentals come from research tables (synced via LAN) — query locally on both
                    data.all_fundamentals =
                        fundamentals::get_all_fundamentals(conn).unwrap_or_default();
                    data.upcoming_earnings =
                        fundamentals::get_upcoming_earnings(conn, 50).unwrap_or_default();
                    data.upcoming_dividends =
                        fundamentals::get_upcoming_dividends(conn, 50).unwrap_or_default();
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

                    let is_lan_client =
                        lan_client_bg.load(std::sync::atomic::Ordering::Relaxed);
                    // Full refresh: insider-trade grouping — once per startup only.
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

                    // Insider trades: load ALL from the local sec_insider_trades table
                    // (LAN-synced as a research table), grouped by uppercased ticker.
                    {
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

                    // Mark full refresh complete
                    full_refresh_done = true;
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
