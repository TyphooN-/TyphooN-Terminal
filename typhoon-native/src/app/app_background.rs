use super::*;

const BG_SNAPSHOT_CHANNEL_CAPACITY: usize = 1;

fn try_publish_bg_snapshot(tx: &std::sync::mpsc::SyncSender<BgData>, data: &BgData) -> bool {
    tx.try_send(data.clone()).is_ok()
}

pub(super) fn spawn_background_refresh(
    app: &mut TyphooNApp,
    shared_cache: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
    importing_flag_bg: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    // Spawn the background data-refresh thread (SEC filings, fundamentals,
    // cache/storage stats, insider trades). Keep exactly one unpublished full
    // snapshot: BgData can be several GB during broad sync, so an unbounded
    // channel retained a new clone every 3 seconds whenever egui stalled. That
    // drove VmHWM above 45 GB and amplified allocator/UI stalls. A stale queued
    // snapshot is sufficient; the next cycle republishes after the UI catches up.
    {
        let (bg_tx, bg_rx) = std::sync::mpsc::sync_channel::<BgData>(BG_SNAPSHOT_CHANNEL_CAPACITY);
        app.bg_rx = bg_rx;
        let shared_cache_bg = shared_cache.clone();
        let _ = std::thread::Builder::new()
            .name("typhoon-bg-refresh".to_string())
            .spawn(move || {
            let importing_flag_bg = importing_flag_bg;
            let mut full_refresh_done = false;
            let mut last_vacuum = std::time::Instant::now();
            const FULL_REFRESH_INTERVAL: std::time::Duration =
                std::time::Duration::from_secs(300); // 5 minutes
            const VACUUM_INTERVAL: std::time::Duration = std::time::Duration::from_secs(21600); // 6 hours
            // News retention: bound research_news (and its FTS mirror) by both
            // age and row count so a full-universe "Fetch (All)" scrape can't
            // leave the table — and the header COUNT / FTS search that walk it —
            // growing without limit. Runs on the 6-hour VACUUM cadence. See ADR-121.
            const NEWS_RETENTION_DAYS: i64 = 45;
            const NEWS_MAX_ROWS: i64 = 250_000;
            // Persist data across loops so lightweight refreshes keep prior results.
            let mut data = BgData::default();
            let mut last_regsho_refresh: Option<std::time::Instant> = None;
            // Halts are transient (LULD pauses can resolve in minutes), so this
            // refreshes far more often than the daily Reg SHO list.
            let mut last_halt_refresh: Option<std::time::Instant> = None;
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
                        tracing::debug!("BG thread: waiting for cache to open...");
                    }
                    continue;
                }

                // Reopen BG read connection EVERY cycle so it always sees the latest
                // WAL writes from broker fetches and bar sync, etc.
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

                    // Phase 1b: table creation needs the write conn (CREATE TABLE IF
                    // NOT EXISTS). Take it ONLY for this — released immediately. The
                    // Reg SHO / halts refreshes below must NOT hold the write
                    // connection across their HTTP fetches; doing so stalled every
                    // bar-sync writer for the whole network round-trip. They fetch
                    // unlocked, then re-take the lock only for the quick DELETE+INSERT.
                    if let Ok(wconn) = cache.connection() {
                        let _ = sec_filing::create_sec_tables(&wconn);
                        let _ = fundamentals::create_fundamentals_tables(&wconn);
                        let _ = regulatory_alerts::create_regulatory_alert_tables(&wconn);
                    }

                    let regsho_due = last_regsho_refresh
                        .map(|t| t.elapsed() >= std::time::Duration::from_secs(30 * 60))
                        .unwrap_or(true);
                    if regsho_due {
                        // Cached as_of read under a brief lock (smart-skip input).
                        let cached_as_of = cache.connection().ok().and_then(|c| {
                            regulatory_alerts::get_latest_regsho_as_of(&c).ok().flatten()
                        });
                        // Network fetch with NO DB lock held.
                        let fetched = tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                            .map_err(|e| format!("runtime build failed: {e}"))
                            .and_then(|rt| {
                                rt.block_on(regulatory_alerts::fetch_regsho_threshold_entries())
                            });
                        match fetched {
                            Ok((remote_as_of, rows)) => {
                                // Smart refresh: write only when the remote file is
                                // newer; the lock is held just for the DELETE+INSERT.
                                if cached_as_of.as_deref() != Some(remote_as_of.as_str()) {
                                    if let Ok(wconn) = cache.connection() {
                                        match regulatory_alerts::replace_regsho_threshold_alerts(
                                            &wconn,
                                            &remote_as_of,
                                            &rows,
                                        ) {
                                            Ok(n) => {
                                                if n > 0 {
                                                    tracing::info!(
                                                        "Reg SHO threshold list refreshed: {n} symbols"
                                                    );
                                                }
                                            }
                                            Err(e) => tracing::warn!(
                                                "Reg SHO threshold list refresh failed: {e}"
                                            ),
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Reg SHO threshold list refresh failed: {e}")
                            }
                        }
                        last_regsho_refresh = Some(std::time::Instant::now());
                    }

                    // Trading halts / LULD pauses — short cadence while a US session
                    // could be live, but a halt cannot post overnight or on weekends,
                    // so back the poll off hard then (the every-2-min weekend refresh
                    // was pure waste). Coarse, clock-free, holiday-blind — good enough
                    // to gate a poll.
                    let halt_cadence = if typhoon_engine::core::market_session::us_equities_extended_session_possible(
                        chrono::Utc::now(),
                    ) {
                        std::time::Duration::from_secs(2 * 60)
                    } else {
                        std::time::Duration::from_secs(30 * 60)
                    };
                    let halts_due = last_halt_refresh
                        .map(|t| t.elapsed() >= halt_cadence)
                        .unwrap_or(true);
                    if halts_due {
                        // Network fetch with NO DB lock held; write under a brief lock.
                        let fetched = tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                            .map_err(|e| format!("runtime build failed: {e}"))
                            .and_then(|rt| {
                                rt.block_on(regulatory_alerts::fetch_trade_halt_entries())
                            });
                        match fetched {
                            Ok(rows) => {
                                if let Ok(wconn) = cache.connection() {
                                    match regulatory_alerts::replace_trade_halt_alerts(
                                        &wconn, &rows,
                                    ) {
                                        Ok(n) => {
                                            if n > 0 {
                                                tracing::info!("Trading halts refreshed: {n} active");
                                            }
                                        }
                                        Err(e) => {
                                            tracing::warn!("Trading halts refresh failed: {e}")
                                        }
                                    }
                                }
                            }
                            Err(e) => tracing::warn!("Trading halts refresh failed: {e}"),
                        }
                        last_halt_refresh = Some(std::time::Instant::now());
                    }
                    // SEC data + cache stats — all via BG's own connection.
                    // Do NOT hydrate the full `sec_filings` table here. A full-universe
                    // scrape can leave 1M+ rows; loading and cloning that snapshot during
                    // startup has repeatedly pushed release/max into the OOM killer before
                    // market-data backpressure can even engage. Keep the always-on BG
                    // snapshot to the recent visible set; deeper SEC browsing/search must
                    // stay on-demand instead of living in every app snapshot.
                    // Keep the previous snapshot on error instead of publishing an
                    // empty one. `unwrap_or_default()` here made a failed query
                    // (e.g. SQLITE_BUSY while the broad EDGAR scraper holds the
                    // write lock) indistinguishable from "no filings", so the
                    // scanner told the user to "Click Scrape Now to fetch from SEC
                    // EDGAR" while a million rows sat in the table.
                    // Global browse window. Still bounded — an unbounded
                    // snapshot of a 1M-row corpus is what reached the OOM
                    // killer — but 1000 rows spanned only ~5 weeks and ~130
                    // tickers of a table going back to 1994, which is not a
                    // usable default view. Per-symbol depth is the on-demand
                    // `SecFilingHistory` query, not this snapshot.
                    match sec_filing::get_recent_filings(conn, None, 20_000) {
                        Ok(filings) => data.sec_filings = filings,
                        Err(e) => tracing::warn!(
                            "SEC recent-filings snapshot failed, keeping {} cached row(s): {e}",
                            data.sec_filings.len()
                        ),
                    }
                    match sec_filing::get_filing_alerts(conn, false) {
                        Ok(alerts) => data.sec_alerts = alerts,
                        Err(e) => tracing::warn!(
                            "SEC filing-alerts snapshot failed, keeping {} cached row(s): {e}",
                            data.sec_alerts.len()
                        ),
                    }
                    data.sec_content_stats = sec_filing::filing_content_stats(conn);
                    data.regulatory_alerts_by_symbol = regulatory_alerts::regulatory_alert_map(
                        &regulatory_alerts::get_regulatory_alerts(conn).unwrap_or_default(),
                    );
                    // Keep the BG summary consistent with Storage Manager's on-demand
                    // refresh path: user-visible rows + total on-disk footprint.
                    if let Ok(stats) = cache.stats() {
                        data.cache_stats = Some(stats);
                    }
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
                        // a cold startup doesn't monopolise the BG loop. This runs on
                        // the BG thread's own read-only WAL connection (zero UI/render
                        // contention), so the only cost of a larger budget is delaying
                        // the *other* BG phases within a warm-up cycle. At ~600 µs per
                        // decompression, 2000/cycle ≈ 1.2 s of work and warms the real
                        // ~68k-key universe in ~34 cycles (~1.7 min) instead of ~7 min at
                        // 500. After warm-up the write_ts gate skips everything, so steady
                        // state is ~free. (Persisting the cache across restarts is the
                        // proper follow-up so this warm-up only happens once, ever.)
                        const BAR_TS_CACHE_DECOMPRESSIONS_PER_CYCLE: usize = 2000;
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
                        // One catalog scan on the BG thread builds every sync
                        // lane's (symbol, timeframe) state map, so the render
                        // thread reads them instead of rescanning per lane.
                        data.source_sync_state = super::market_data_sync::build_source_sync_state_maps(
                            &data.detailed_stats,
                            &data.bar_ts_cache,
                        );
                        data.sync_state_ready = true;
                    }
                    // Fundamentals come from research tables (synced via LAN) — query locally on both
                    data.all_fundamentals =
                        fundamentals::get_all_fundamentals(conn).unwrap_or_default();
                    data.market_map_model = Arc::new(market_map_model::build_market_map_model(
                        &data.all_fundamentals,
                    ));
                    data.fundamentals_company_names = Arc::new(
                        data.all_fundamentals
                            .iter()
                            .filter_map(|row| {
                                let symbol = bare_symbol_from_key(row.symbol.trim())
                                    .replace('/', "")
                                    .trim_end_matches(".EQ")
                                    .trim_end_matches(".eq")
                                    .to_ascii_uppercase();
                                let name = row.company_name.trim();
                                (!symbol.is_empty() && !name.is_empty())
                                    .then(|| (symbol, name.to_string()))
                            })
                            .collect(),
                    );
                    data.upcoming_earnings = fundamentals::get_upcoming_earnings(conn, 50)
                        .unwrap_or_default()
                        .into_iter()
                        .map(|(symbol, company, date)| {
                            UpcomingEarningsRow::from_raw(symbol, company, date)
                        })
                        .collect::<Vec<_>>()
                        .into();
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
                        // News retention: age purge + hard row cap. Keeps the
                        // news corpus bounded so the header COUNT and FTS search
                        // stay cheap regardless of how many full-universe news
                        // scrapes have run.
                        let news_cutoff = chrono::Utc::now().timestamp()
                            - NEWS_RETENTION_DAYS * 24 * 60 * 60;
                        match cache.enforce_news_retention(news_cutoff, NEWS_MAX_ROWS) {
                            Ok((by_age, by_cap)) if by_age + by_cap > 0 => {
                                tracing::info!(
                                    "BG: news retention purged {} (age >{}d) + {} (cap {}) article(s)",
                                    by_age,
                                    NEWS_RETENTION_DAYS,
                                    by_cap,
                                    NEWS_MAX_ROWS
                                );
                            }
                            Ok(_) => {}
                            Err(e) => tracing::warn!("BG: news retention failed: {e}"),
                        }
                        last_vacuum = std::time::Instant::now();
                        tracing::info!("BG: incremental_vacuum(500) completed");
                    }

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
                        if bg_cycle_count % 10 == 5 {
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
                                // Form 4 insider backfill, offset from the content
                                // backfill cycle so the two never share a slot and
                                // double SEC request pressure.
                                //
                                // Insider parsing only ever ran inline over filings
                                // inserted by the current scrape pass, so anything
                                // that failed was never retried — and until the
                                // `xslF345X0*` URL fix every Form 4 failed (the
                                // parser was handed SEC's rendered HTML, not XML).
                                // That is why 537k stored Form 4 filings yielded an
                                // empty sec_insider_trades table and an "Insiders (0)"
                                // tab. This drains the backlog newest-first.
                                if bg_cycle_count % 10 == 7 {
                                    let db_path = crate::app::platform::cache_db_path();
                                    let _ = std::thread::Builder::new()
                                        .name("typhoon-sec-insider-backfill".into())
                                        .spawn(move || {
                                            let rt = match tokio::runtime::Builder::new_current_thread()
                                                .enable_all()
                                                .build()
                                            {
                                                Ok(rt) => rt,
                                                Err(e) => {
                                                    tracing::warn!("SEC insider backfill skipped: runtime build failed: {e}");
                                                    return;
                                                }
                                            };
                                            rt.block_on(async {
                                                let client = reqwest::Client::builder()
                                                    .user_agent(sec_filing::SEC_EDGAR_USER_AGENT)
                                                    .timeout(std::time::Duration::from_secs(10))
                                                    .build()
                                                    .unwrap_or_default();
                                                match sec_filing::backfill_insider_trades(
                                                    &db_path, &client, 15,
                                                )
                                                .await
                                                {
                                                    Ok((0, 0, 0)) => {}
                                                    Ok((trades, alerts, failures)) => {
                                                        tracing::info!(
                                                            "BG: SEC insider backfill: {trades} trade(s), {alerts} alert(s), {failures} failed"
                                                        );
                                                    }
                                                    Err(e) => tracing::warn!(
                                                        "SEC insider backfill failed: {e}"
                                                    ),
                                                }
                                            });
                                        });
                                }
                            }
                        }
                        bg_cycle_count += 1;
                        let _ = try_publish_bg_snapshot(&bg_tx, &data);
                        continue;
                    }

                    // Insider trades: load ALL from the local sec_insider_trades table
                    // (a research table), grouped by uppercased ticker.
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

                    // Send without retaining a backlog of enormous snapshots when
                    // rendering is temporarily delayed.
                    let _ = try_publish_bg_snapshot(&bg_tx, &data);
                }
            }
        });
    }
}

impl TyphooNApp {
    pub(crate) fn tick_background_snapshot_drain(&mut self) {
        // ── drain background data ────────────────────────────────────────
        // Keep this bounded: a backend burst must not monopolize the render thread.
        // The channel carries full snapshots, so newest wins; apply at most one per frame.
        if let Ok(mut data) = self.bg_rx.try_recv() {
            while let Ok(newer) = self.bg_rx.try_recv() {
                let older = std::mem::replace(&mut data, newer);
                self.drop_bg_snapshot_off_ui(older);
            }
            // Applying a BG snapshot can move hundreds of thousands of SEC rows
            // into `self.bg` and drop the previous vector on the egui thread. During
            // heavy sync/SEC/news sweeps that showed up as 250ms+ pre_broker stalls
            // every refresh cycle, which makes chart drag feel like snap-back. If no
            // window that needs the BG tables is visible, drop this refresh and let
            // the next 3s BG cycle republish after the hot path cools down.
            let bg_window_visible = self.show_sec
                || self.show_fundamentals
                || self.show_storage
                || self.show_cache_stats
                // Sync Status reads bg.detailed_stats/cache_stats; keep feeding it
                // (instead of dropping the snapshot during heavy sync) now that it
                // no longer has a synchronous render-thread refresh fallback.
                || self.show_sync_status;
            // The first snapshot carrying a completed detailed-stats pass must
            // always apply: the broad sync lanes are gated on
            // `bg.sync_state_ready`, and dropping every snapshot while e.g.
            // news_loading holds heavy_sync would leave them gated (and the
            // scheduler blind) indefinitely.
            let first_ready_snapshot = data.sync_state_ready && !self.bg.sync_state_ready;
            // Heavy sync suppresses applies to protect the render thread, but a
            // catch-up can hold the flag for hours — the staleness bound keeps
            // the schedulers and coverage %/auto-full-tilt fed on a slow cadence
            // instead of freezing them for the whole run.
            if app_runtime_support::should_apply_bg_snapshot(
                self.heavy_sync_in_progress,
                bg_window_visible,
                first_ready_snapshot,
                self.bg_snapshot_last_applied.elapsed(),
            ) {
                self.replace_bg_snapshot_off_ui_drop(data);
                if first_ready_snapshot {
                    // The broad bar schedulers are intentionally gated until the
                    // BG thread has produced the first cache coverage snapshot;
                    // before this point every catalog cell looks Missing and a
                    // cold start can stampede full-history requests. Catalog and
                    // broker-connect messages can arrive earlier, so their refill
                    // attempts no-op under the gate. Kick the scheduler exactly
                    // when the gate opens instead of waiting for the next periodic
                    // tick or a chart-hover/user-demand path to enqueue work.
                    let pending_before = self.total_pending_market_data_fetches();
                    self.refill_market_data_sync_slots();
                    let pending_after = self.total_pending_market_data_fetches();
                    if pending_after > pending_before {
                        self.log.push_back(LogEntry::info(format!(
                            "Bar sync scheduler started from startup cache snapshot — queued {} fetch(es), {} pending",
                            pending_after - pending_before,
                            pending_after
                        )));
                    }
                }
            } else {
                tracing::debug!(
                    "Deferred BG snapshot apply during heavy sync (sec_filings={}, details={})",
                    data.sec_filings.len(),
                    data.detailed_stats.len()
                );
                self.drop_bg_snapshot_off_ui(data);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn background_snapshot_channel_keeps_at_most_one_unpublished_clone() {
        let (tx, rx) = std::sync::mpsc::sync_channel(BG_SNAPSHOT_CHANNEL_CAPACITY);
        let data = BgData::default();

        assert!(try_publish_bg_snapshot(&tx, &data));
        assert!(!try_publish_bg_snapshot(&tx, &data));
        assert!(rx.try_recv().is_ok());
        assert!(try_publish_bg_snapshot(&tx, &data));
    }
}
