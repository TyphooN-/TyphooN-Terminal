use super::*;
use crate::app::chart_ops::{MTF_GRID_TIMEFRAMES, mtf_visible_chart_groups};

use super::app_runtime_support::*;
// egui 0.34: Panel::show(ctx) deprecated in favor of show_inside(ui).
// Full migration to ui() pattern is deferred while this runtime pass focuses on module boundaries.
#[allow(deprecated)]
impl eframe::App for TyphooNApp {
    fn on_exit(&mut self) {
        self.save_session();
        // Explicit WAL checkpoint on exit — keeps WAL file small for next startup.
        if let Some(ref cache) = self.cache {
            if let Ok(conn) = cache.connection() {
                let _ = conn.execute("PRAGMA wal_checkpoint(TRUNCATE)", []);
            }
        }
    }

    fn ui(&mut self, _ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Panels rendered in update() via ctx — egui 0.34 migration to ui() pattern is deferred
        // since our 16K-line update() mixes panel rendering with floating windows + state logic
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frame_count += 1;
        let now_instant = std::time::Instant::now();
        let perf_pre_broker_ms;
        let perf_broker_drain_ms;
        let perf_after_broker_started;
        let perf_post_broker_setup_ms;
        let perf_chrome_panels_ms;
        let perf_floating_windows_ms;
        // Track user activity for the auto-compact idle gate. Any input event in
        // the frame counts as activity. Cheap — `events` is always queried below.
        if ctx.input(|i| !i.events.is_empty()) {
            self.auto_compact_last_input_at = std::time::Instant::now();
        }
        self.tick_auto_compact();
        self.clear_stale_ui_busy_flags(now_instant);
        // Alpaca retry queue: internally throttled to 10s between ticks.
        // Loads persisted state on first call, re-dispatches due entries.
        self.poll_alpaca_retry_queue();
        // PERF: Broad sync/scrape work must not leave egui in continuous full-rate
        // repaint mode. The flag was previously initialized but never driven, so
        // a 12k-symbol universe sync + news/SEC/fundamentals passes still rendered
        // every idle frame. Input frames still request immediate repaint below.
        let pending_market_data_fetches = self.total_pending_market_data_fetches();
        self.heavy_sync_in_progress = ui_heavy_sync_active(
            pending_market_data_fetches,
            self.deferred_chart_loads.len(),
            self.news_loading,
            self.scrape_fund_running,
            self.scrape_sec_running,
            self.auto_compact_in_progress,
        );
        // PERF: rebuild scope HashSet only when bg data loaded or scope changed,
        // not every frame. Steady state = zero work.
        let scope_key = (self.bg_rev, self.broker_scope);
        if self.cached_scope_key != Some(scope_key) {
            self.cached_scope_syms = self.broker_scope_symbols();
            self.cached_scope_key = Some(scope_key);
        }
        // PERF: Cache active_symbols() + HashSet until its chart/position/watchlist inputs change
        // (used by 5+ windows for "Active Only" filters).
        let active_symbols_key = self.active_symbols_cache_key();
        if self.cached_active_symbols_key != Some(active_symbols_key) {
            self.cached_active_symbols = self.active_symbols();
            self.cached_active_symbols_set = self.cached_active_symbols.iter().cloned().collect();
            self.cached_active_symbols_key = Some(active_symbols_key);
        }
        // PERF: Cache scoped_fundamentals_owned() only when bg/scope changes — not per frame.
        // Was cloning ~500 Fundamentals structs (≈1 MB) every frame for no reason.
        if self.cached_scoped_fundamentals_key != Some(scope_key) {
            self.cached_scoped_fundamentals = self.scoped_fundamentals_owned();
            self.cached_scoped_fundamentals_key = Some(scope_key);
        }
        if self.cached_alpaca_sync_state_rev != Some(self.bg_rev)
            && (!self.heavy_sync_in_progress || self.cached_alpaca_sync_state.is_empty())
        {
            let previous = self.cached_alpaca_sync_state.clone();
            let mut rebuilt = self.build_alpaca_cache_state_map();
            merge_recent_sync_overrides(&mut rebuilt, &previous, chrono::Utc::now().timestamp());
            self.cached_alpaca_sync_state = rebuilt;
            self.cached_alpaca_sync_state_rev = Some(self.bg_rev);
        }
        ctx.set_visuals(Self::dark_visuals());
        // Bound log size to prevent unbounded memory growth.
        // 200 is a steady-state cap — small enough that pop_front is amortized O(1)
        // even during bulk imports that push dozens of lines per frame.
        while self.log.len() > 200 {
            self.log.pop_front();
        }

        if self.cache_loaded
            && self.kraken_enabled
            && self.kraken_any_spot_scrape_enabled()
            && self.kraken_pairs.is_empty()
            && !self.kraken_pairs_requested
        {
            let _ = self.broker_tx.send(BrokerCmd::KrakenGetPairs);
            self.kraken_pairs_requested = true;
        }
        let now_ts = chrono::Utc::now().timestamp();
        if self.cache_loaded
            && self.kraken_enabled
            && self.kraken_scrape_xstocks
            && self.kraken_equity_universe_symbols.is_empty()
            && (!self.kraken_equity_universe_requested
                || now_ts >= self.kraken_equity_universe_retry_after_ts)
        {
            let _ = self.broker_tx.send(BrokerCmd::KrakenFetchEquityUniverse);
            self.kraken_equity_universe_requested = true;
            self.kraken_equity_universe_retry_after_ts = now_ts + 120;
        }
        if self.cache_loaded
            && self.kraken_enabled
            && self.kraken_scrape_futures
            && self.kraken_futures_symbols.is_empty()
            && !self.kraken_futures_requested
        {
            let _ = self.broker_tx.send(BrokerCmd::KrakenFuturesGetInstruments);
            self.kraken_futures_requested = true;
        }

        // Periodic crypto bar refresh (every ~60 seconds at 4fps = every 240 frames)
        // Periodic crypto bar refresh (~60s).
        // Uses Kraken (free, no auth) as primary source, Alpaca as fallback
        if now_instant.duration_since(self.periodic_crypto_last_refresh)
            >= std::time::Duration::from_secs(60)
            && self.cache_loaded
        {
            self.periodic_crypto_last_refresh = now_instant;
            if let Some(chart) = self.charts.get(self.active_tab) {
                let sym = chart.symbol.clone();
                let bare = sym.split(':').last().unwrap_or(&sym).to_string();
                let crypto_bases = [
                    "BTC", "ETH", "SOL", "DOGE", "XRP", "ADA", "LTC", "LINK", "AVAX", "DOT", "XMR",
                    "ZEC", "DASH",
                ];
                let su = bare.to_uppercase();
                let is_crypto = crypto_bases
                    .iter()
                    .any(|b| su.starts_with(b) && su.ends_with("USD"));
                if is_crypto {
                    let tf_label = chart.timeframe.cache_suffix().to_string(); // "1Month" not "MN1"
                    if self.sync_timeframe_enabled(&tf_label) {
                        let tf_minutes = chart.timeframe.minutes();
                        // Fetch from Kraken (free, no auth, works on weekends)
                        // Fetch chart's TF + all lower TFs for gap fill and forming bar synthesis
                        let mut timeframes = vec![tf_label.clone()];
                        let all_tfs = [
                            "1Week", "1Day", "4Hour", "1Hour", "30Min", "15Min", "5Min", "1Min",
                        ];
                        for ltf in &all_tfs {
                            let ltf_min: u32 = match *ltf {
                                "1Week" => 10080,
                                "1Day" => 1440,
                                "4Hour" => 240,
                                "1Hour" => 60,
                                "30Min" => 30,
                                "15Min" => 15,
                                "5Min" => 5,
                                _ => 1,
                            };
                            if ltf_min < tf_minutes && !timeframes.contains(&ltf.to_string()) {
                                timeframes.push(ltf.to_string());
                                break; // just the next lower TF for forming bar (Kraken has rate limits)
                            }
                        }
                        // Server/standalone: queue through the scheduler path so periodic chart
                        // refreshes respect pending slots, no-data tombstones, and persisted
                        // backfill-complete markers instead of forcing full-history attempts.
                        let timeframes =
                            self.filtered_sync_timeframes(timeframes.iter().map(|tf| tf.as_str()));
                        if self.kraken_spot_symbol_scrape_enabled(&bare) {
                            for tf in timeframes {
                                self.queue_kraken_fetch(&bare, &tf);
                            }
                        }
                    }
                }
            }
        }

        // Watchlist quotes used to be fetched only when the user manually added a
        // symbol, so a session-restored watchlist sat empty ("No cached data …
        // never") until poked. Refresh once on startup (auto_refresh_at == None)
        // and every 30s after. The GetWatchlistQuotes handler enriches from Yahoo
        // even with no broker connected, so this also works offline / on weekends.
        if self.cache_loaded && !self.user_watchlist.is_empty() {
            // Intraday: refresh every 30s. While the xStocks market is closed for the
            // weekend, watchlist quotes are static (no new prints), so stop re-polling
            // Yahoo every 30s — refresh only on a slow safety heartbeat or when the
            // watchlist set itself changes (symbol added/removed). Friday's last
            // after-hours snapshot is retained for display in the meantime.
            let interval = if super::app_runtime_support::kraken_xstocks_weekend_closed_now() {
                std::time::Duration::from_secs(300)
            } else {
                std::time::Duration::from_secs(30)
            };
            let watchlist_changed =
                self.watchlist_quotes_fetched_count != self.user_watchlist.len();
            let due = watchlist_changed
                || self
                    .watchlist_auto_refresh_at
                    .map(|t| now_instant.duration_since(t) >= interval)
                    .unwrap_or(true);
            if due {
                self.watchlist_auto_refresh_at = Some(now_instant);
                self.watchlist_quotes_fetched_count = self.user_watchlist.len();
                let _ = self.broker_tx.send(BrokerCmd::GetWatchlistQuotes {
                    symbols: self.user_watchlist.clone(),
                });
            }
        }

        // Positions/orders are trading-critical UI, not five-minute background
        // metadata. Reconcile them periodically without tying the cadence to the
        // broad cache refresh loop; the dispatch timestamp prevents per-frame spam
        // if a broker response is slow.
        let positions_due = self
            .positions_auto_refresh_at
            .map(|t| now_instant.duration_since(t) >= std::time::Duration::from_secs(30))
            .unwrap_or(true);
        if positions_due {
            let mut requested = false;
            if self.alpaca_enabled && self.broker_connected {
                let _ = self.broker_tx.send(BrokerCmd::GetPositions);
                let _ = self.broker_tx.send(BrokerCmd::GetOrders);
                requested = true;
            }
            if self.kraken_enabled && self.kraken_connected {
                let _ = self.broker_tx.send(BrokerCmd::KrakenGetBalance);
                let _ = self.broker_tx.send(BrokerCmd::KrakenGetPositions);
                let _ = self.broker_tx.send(BrokerCmd::KrakenFetchOpenOrders);
                requested = true;
            }
            if requested {
                self.positions_auto_refresh_at = Some(now_instant);
            }
        }

        // Refresh the cached Sync Status coverage % so auto-full-tilt sees
        // current data even when the Sync Status window isn't open. The
        // full xStocks/Merged matrix scan runs on a blocking worker (never the
        // render thread); poll applies any finished result, refresh dispatches
        // a new snapshot compute when the cached rows go stale.
        if self.cache_loaded {
            self.poll_bar_sync_compute();
            self.refresh_bar_sync_rows_if_stale();
        }

        if now_instant.duration_since(self.kraken_universe_last_schedule)
            >= self.market_data_sync_interval()
            && self.cache_loaded
            && self.kraken_enabled
            && self.kraken_full_bar_sync_enabled
            && (self.kraken_any_spot_scrape_enabled()
                || (self.kraken_scrape_xstocks && !self.kraken_equity_universe_symbols.is_empty()))
        {
            self.kraken_universe_last_schedule = now_instant;
            let _ = self.schedule_kraken_equities_universe();
            let _ = self.schedule_kraken_universe_sectors();
            let _ = self.maybe_schedule_kraken_ws_ohlc_snapshot_sweep();
        }

        // WS OHLC spawn is pair-discovery/settings driven. At startup the
        // settings loop can run before Kraken AssetPairs have landed, in
        // which case maybe_start_kraken_ws_ohlc defers without flipping
        // `started=true`. Retry every 15s so the full-universe streamers come
        // up once pair discovery completes, without forcing the user to toggle
        // the setting. Cheap idempotent no-op once `started=true`.
        if !self.kraken_ws_ohlc_started
            && self.kraken_ws_ohlc_enabled
            && self.kraken_enabled
            && now_instant.duration_since(self.kraken_ws_ohlc_last_spawn_retry)
                >= std::time::Duration::from_secs(15)
        {
            self.kraken_ws_ohlc_last_spawn_retry = now_instant;
            self.maybe_start_kraken_ws_ohlc();
        }

        // Chart bid/ask should prefer Kraken's WS v2 L2 top-of-book when the
        // active chart is a Kraken spot or xStock symbol. OHLC updates are bar
        // cadence; ticker/iapi can lag or be delayed. The book stream is the
        // freshest public best bid/ask feed we have and validates CRC32 before
        // publishing top-of-book ticks back into ChartState.
        if self.kraken_enabled
            && now_instant.duration_since(self.kraken_chart_l2_last_start_attempt)
                >= std::time::Duration::from_secs(5)
            && let Some(chart) = self.charts.get(self.active_tab)
        {
            let source = cache_source_from_key(&chart.symbol);
            let bare = bare_symbol_from_key(&chart.symbol)
                .trim_end_matches(".EQ")
                .to_ascii_uppercase();
            let kraken_chart = matches!(source, "kraken" | "kraken-equities")
                || chart.symbol.to_ascii_uppercase().contains("KRAKEN")
                || chart.symbol.to_ascii_uppercase().contains(".EQ")
                || self
                    .kraken_equity_universe_symbols
                    .iter()
                    .any(|symbol| symbol.trim_end_matches(".EQ").eq_ignore_ascii_case(&bare));
            if kraken_chart
                && !bare.is_empty()
                && !self.kraken_chart_l2_ws_symbol.eq_ignore_ascii_case(&bare)
            {
                self.kraken_chart_l2_last_start_attempt = now_instant;
                self.kraken_chart_l2_ws_symbol = bare.clone();
                let _ = self.broker_tx.send(BrokerCmd::KrakenStartOrderbookWs {
                    symbol: bare,
                    depth: 10,
                    publish_dom: false,
                });
            }
        }

        // News body hydrator: fetch the full article text for rows that
        // still only have the provider summary. Throttled by
        // HYDRATE_INTERVAL_SECS and gated on `in_flight` so we never have
        // two tokio tasks racing on the same cache rows.
        if self.cache_loaded
            && !self.news_body_hydrate_in_flight
            && now_instant.duration_since(self.news_body_last_hydrate)
                >= std::time::Duration::from_secs(super::news_ingest::HYDRATE_INTERVAL_SECS)
        {
            if let Some(cache) = self.cache.clone() {
                self.news_body_last_hydrate = now_instant;
                self.news_body_hydrate_in_flight = true;
                let symbol_hint = self
                    .charts
                    .first()
                    .map(|c| c.symbol.clone())
                    .filter(|s| !s.is_empty());
                let rt = self.rt_handle.clone();
                rt.spawn(async move {
                    let _ = super::news_ingest::hydrate_missing_bodies(cache, symbol_hint).await;
                    // No callback channel: the next tick simply observes the
                    // `in_flight` flag being reset after the task completes.
                    // We can't poke `self` from here, so the gate is released
                    // on the next `update()` by a separate fast path below.
                });
            }
        }
        // Release the in-flight flag after a generous timeout — covers the
        // case where the spawned task is still running but a new tick wants
        // to re-arm. We don't need exact synchronisation: the new task will
        // pick up whatever rows are still empty.
        if self.news_body_hydrate_in_flight
            && now_instant.duration_since(self.news_body_last_hydrate)
                >= std::time::Duration::from_secs(super::news_ingest::HYDRATE_INTERVAL_SECS * 2)
        {
            self.news_body_hydrate_in_flight = false;
        }

        if now_instant.duration_since(self.kraken_futures_universe_last_schedule)
            >= self.market_data_sync_interval()
            && self.cache_loaded
            && self.kraken_enabled
        {
            self.kraken_futures_universe_last_schedule = now_instant;
            let _ = self.schedule_kraken_futures_universe_sectors();
        }

        // ── Screenshot: issue capture command ────────────────────────────
        if self.screenshot_requested {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            self.screenshot_requested = false;
        }

        // ── Screenshot: handle captured image (offload PNG encode to background thread) ──
        {
            let screenshot_data: Option<(Vec<u8>, u32, u32, std::path::PathBuf)> = ctx.input(|i| {
                for event in &i.events {
                    if let egui::Event::Screenshot { image, .. } = event {
                        let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");
                        let pictures_dir = if let Ok(home) = std::env::var("HOME") {
                            let p = std::path::PathBuf::from(home).join("Pictures");
                            let _ = std::fs::create_dir_all(&p);
                            p
                        } else {
                            std::path::PathBuf::from("/tmp")
                        };
                        let path = pictures_dir.join(format!("typhoon_chart_{}.webp", ts));
                        let w = image.width() as u32;
                        let h = image.height() as u32;
                        let rgba: Vec<u8> = image
                            .pixels
                            .iter()
                            .flat_map(|c| [c.r(), c.g(), c.b(), c.a()])
                            .collect();
                        return Some((rgba, w, h, path));
                    }
                }
                None
            });
            if let Some((rgba, w, h, path)) = screenshot_data {
                // Lossless WebP encoding on background thread (smaller than PNG, no quality loss)
                let last_screenshot_path = path.clone();
                self.log.push_back(LogEntry::info(format!(
                    "Saving screenshot ({w}x{h}) to {}...",
                    path.display()
                )));
                self.last_screenshot_path = Some(last_screenshot_path);
                self.rt_handle.spawn_blocking(move || {
                    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba) {
                        let dyn_img = image::DynamicImage::ImageRgba8(img);
                        match dyn_img.save(&path) {
                            Ok(()) => tracing::info!("Screenshot saved: {}", path.display()),
                            Err(e) => tracing::error!("Screenshot save failed: {}", e),
                        }
                    } else {
                        tracing::error!("Screenshot: failed to construct image from RGBA data");
                    }
                });
            }
        }

        // ── Receive async cache open result ──────────────────────────────
        if self.cache.is_none() {
            if let Some(ref rx) = self.cache_rx {
                if let Ok(c) = rx.try_recv() {
                    self.log.push_back(LogEntry::info("Cache opened"));
                    self.cache = Some(c);
                    self.cache_rx = None; // done, drop receiver
                }
            }
        }
        // Load charts once cache arrives
        if !self.cache_loaded && self.cache.is_some() {
            self.cache_loaded = true;
            self.hydrate_loaded_charts();
            self.sync_preferences_load();
            self.alpaca_retry_load();
            self.alpaca_no_data_load();
            self.unresolvable_load();
            self.alpaca_backfill_complete_load();
            self.kraken_backfill_complete_load();
            self.kraken_futures_backfill_complete_load();
            if !self.alpaca_no_data_pairs.is_empty() {
                self.log.push_back(LogEntry::info(format!(
                    "Loaded {} Alpaca no-data mark(s) from cache",
                    self.alpaca_no_data_pairs.len()
                )));
            }
            if !self.alpaca_backfill_complete_pairs.is_empty() {
                self.log.push_back(LogEntry::info(format!(
                    "Loaded {} Alpaca backfill-complete mark(s) from cache",
                    self.alpaca_backfill_complete_pairs.len()
                )));
            }
            if !self.kraken_backfill_complete_pairs.is_empty()
                || !self.kraken_futures_backfill_complete_pairs.is_empty()
            {
                self.log.push_back(LogEntry::info(format!(
                    "Loaded {} Kraken spot / {} futures backfill-complete mark(s) from cache",
                    self.kraken_backfill_complete_pairs.len(),
                    self.kraken_futures_backfill_complete_pairs.len()
                )));
            }
            // Load credentials FIRST (needed for broker auto-connect)
            {
                let mut keyring_ok = true;
                let cache_ref = self.cache.clone();
                let cred_keys = [
                    (keyring::keys::ALPACA_API_KEY, "alpaca_api_key"),
                    (keyring::keys::ALPACA_SECRET, "alpaca_secret"),
                    (keyring::keys::FINNHUB_KEY, "finnhub_key"),
                    (keyring::keys::FRED_KEY, "fred_key"),
                    (keyring::keys::DISCORD_WEBHOOK, "discord_webhook"),
                    (keyring::keys::PUSHOVER_TOKEN, "pushover_token"),
                    (keyring::keys::PUSHOVER_USER, "pushover_user"),
                    (keyring::keys::NTFY_TOPIC, "ntfy_topic"),
                    (keyring::keys::ANTHROPIC_KEY, "anthropic_key"),
                    (keyring::keys::OPENAI_KEY, "openai_key"),
                    (keyring::keys::KRAKEN_API_KEY, "kraken_api_key"),
                    (keyring::keys::KRAKEN_API_SECRET, "kraken_api_secret"),
                    (keyring::keys::KRAKEN_WS_API_KEY, "kraken_ws_api_key"),
                    (keyring::keys::KRAKEN_WS_API_SECRET, "kraken_ws_api_secret"),
                    (keyring::keys::GEMINI_KEY, "gemini_key"),
                    (keyring::keys::XAI_KEY, "xai_key"),
                    (keyring::keys::MISTRAL_KEY, "mistral_key"),
                    (keyring::keys::PERPLEXITY_KEY, "perplexity_key"),
                    (keyring::keys::MATRIX_ACCESS_TOKEN, "matrix_access_token"),
                    (keyring::keys::MATRIX_USER_ID, "matrix_user_id"),
                    (keyring::keys::CRYPTOPANIC_KEY, "cryptopanic_key"),
                ];
                let mut loaded_values: Vec<(String, String)> = Vec::new();
                for (kr_key, _label) in &cred_keys {
                    match keyring::load(kr_key) {
                        Ok(Some(v)) if !v.is_empty() => {
                            loaded_values.push((kr_key.to_string(), v));
                        }
                        Ok(_) => {
                            if let Some(ref cache) = cache_ref {
                                if let Ok(Some(v)) = cache.get_kv(&format!("cred:{}", kr_key)) {
                                    if !v.is_empty() {
                                        loaded_values.push((kr_key.to_string(), v));
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            keyring_ok = false;
                            self.log.push_back(LogEntry::warn(format!(
                                "Keyring load '{}' failed: {}",
                                kr_key, e
                            )));
                            if let Some(ref cache) = cache_ref {
                                if let Ok(Some(v)) = cache.get_kv(&format!("cred:{}", kr_key)) {
                                    if !v.is_empty() {
                                        loaded_values.push((kr_key.to_string(), v));
                                    }
                                }
                            }
                        }
                    }
                }
                for (key, val) in &loaded_values {
                    match key.as_str() {
                        k if k == keyring::keys::ALPACA_API_KEY => {
                            self.broker_api_key = val.clone()
                        }
                        k if k == keyring::keys::ALPACA_SECRET => self.broker_secret = val.clone(),
                        k if k == keyring::keys::FINNHUB_KEY => self.finnhub_key = val.clone(),
                        k if k == keyring::keys::FRED_KEY => self.fred_key = val.clone(),
                        k if k == keyring::keys::DISCORD_WEBHOOK => {
                            self.discord_webhook = val.clone()
                        }
                        k if k == keyring::keys::PUSHOVER_TOKEN => {
                            self.pushover_token = val.clone()
                        }
                        k if k == keyring::keys::PUSHOVER_USER => self.pushover_user = val.clone(),
                        k if k == keyring::keys::NTFY_TOPIC => self.ntfy_topic = val.clone(),
                        k if k == keyring::keys::ANTHROPIC_KEY => self.anthropic_key = val.clone(),
                        k if k == keyring::keys::OPENAI_KEY => self.openai_key = val.clone(),
                        k if k == keyring::keys::KRAKEN_API_KEY => {
                            self.kraken_api_key = val.clone()
                        }
                        k if k == keyring::keys::KRAKEN_API_SECRET => {
                            self.kraken_api_secret = val.clone()
                        }
                        k if k == keyring::keys::KRAKEN_WS_API_KEY => {
                            self.kraken_ws_api_key = val.clone()
                        }
                        k if k == keyring::keys::KRAKEN_WS_API_SECRET => {
                            self.kraken_ws_api_secret = val.clone()
                        }
                        k if k == keyring::keys::GEMINI_KEY => self.gemini_key = val.clone(),
                        k if k == keyring::keys::XAI_KEY => self.xai_key = val.clone(),
                        k if k == keyring::keys::MISTRAL_KEY => self.mistral_key = val.clone(),
                        k if k == keyring::keys::PERPLEXITY_KEY => {
                            self.perplexity_key = val.clone()
                        }
                        k if k == keyring::keys::MATRIX_ACCESS_TOKEN => {
                            self.matrix_access_token = val.clone()
                        }
                        k if k == keyring::keys::MATRIX_USER_ID => {
                            self.matrix_user_id = val.clone()
                        }
                        k if k == keyring::keys::CRYPTOPANIC_KEY => {
                            self.cryptopanic_key = val.clone()
                        }
                        _ => {}
                    }
                }
                if !loaded_values.is_empty() {
                    let src = if keyring_ok {
                        "system keyring"
                    } else {
                        "SQLite fallback"
                    };
                    self.log.push_back(LogEntry::info(format!(
                        "Credentials loaded from {} ({} keys)",
                        src,
                        loaded_values.len()
                    )));
                }
            }
            // Auto-connect Alpaca if credentials are available.
            if self.alpaca_enabled
                && !self.broker_api_key.is_empty()
                && !self.broker_secret.is_empty()
            {
                let capacity = self.alpaca_sync_capacity();
                let _ = self.broker_tx.send(BrokerCmd::Connect {
                    api_key: self.broker_api_key.clone(),
                    secret: self.broker_secret.clone(),
                    paper: self.broker_paper,
                    bar_requests_per_minute: self.alpaca_effective_historical_rpm(),
                    fetch_permits: capacity.fetch_permits,
                });
                self.log.push_back(LogEntry::info(format!(
                    "Alpaca auto-connecting ({}) — {} req/min startup budget, {} workers",
                    if self.broker_paper { "Paper" } else { "Live" },
                    self.alpaca_effective_historical_rpm(),
                    capacity.fetch_permits
                )));
            }
            // Auto-connect Kraken if credentials are available
            if self.kraken_enabled
                && !self.kraken_api_key.is_empty()
                && !self.kraken_api_secret.is_empty()
            {
                let _ = self.broker_tx.send(BrokerCmd::KrakenConnect {
                    api_key: self.kraken_api_key.clone(),
                    api_secret: self.kraken_api_secret.clone(),
                    ws_api_key: self.kraken_ws_api_key.clone(),
                    ws_api_secret: self.kraken_ws_api_secret.clone(),
                });
                self.log
                    .push_back(LogEntry::info("Kraken auto-connecting..."));
            }

            // Defer chart loading to subsequent frames — don't block the first frame
            // Charts will load progressively (one per frame) via the deferred_chart_loads mechanism
            self.deferred_chart_loads = if self.mtf_enabled {
                self.charts
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| c.bars.is_empty())
                    .map(|(i, _)| i)
                    .collect()
            } else {
                let idx = self.active_tab;
                if self
                    .charts
                    .get(idx)
                    .map(|c| c.bars.is_empty())
                    .unwrap_or(false)
                {
                    VecDeque::from([idx])
                } else {
                    VecDeque::new()
                }
            };
            self.deferred_chart_load_set = self.deferred_chart_loads.iter().copied().collect();
            {
                // ── Startup data fetching ─────────────────────────────────────
                // Auto SEC scrape on startup. Scope-derived universes may still
                // be empty while broker/universe startup tasks are loading; do
                // not send a misleading 0-symbol scrape. The universe-loaded
                // BrokerMsg handler retries this once symbols arrive.
                {
                    let symbols = self.sec_scrape_scope_symbols();
                    let symbol_count = symbols.len();
                    if should_auto_start_background_scope_scrape(self.broker_scope, symbol_count) {
                        let db_path = cache_db_path();
                        let _ = self
                            .broker_tx
                            .send(BrokerCmd::SecScrape { db_path, symbols });
                        self.scrape_sec_running = true;
                        self.scrape_sec_last_msg = format!(
                            "scraping Scope {} ({} symbols)...",
                            self.broker_scope_label(),
                            symbol_count
                        );
                        self.log.push_back(LogEntry::info(format!(
                            "SEC EDGAR scrape started for Scope {} ({} symbols)...",
                            self.broker_scope_label(),
                            symbol_count
                        )));
                    } else if symbol_count == 0 {
                        self.auto_sec_scrape_deferred = true;
                        self.log.push_back(LogEntry::info(format!(
                            "SEC EDGAR auto-scrape deferred: Scope {} has no symbols yet",
                            self.broker_scope_label()
                        )));
                    } else {
                        self.auto_sec_scrape_deferred = false;
                        self.log.push_back(LogEntry::info(format!(
                                "SEC EDGAR auto-scrape skipped for broad Scope {} ({} symbols); use manual SEC scrape for full-universe backfill",
                                self.broker_scope_label(),
                                symbol_count
                            )));
                    }
                }
                // Auto EVSCRAPE on startup (fundamentals, skips if updated <24h).
                // Kraken equities arrive asynchronously, so do not launch a
                // misleading scrape when Kraken is selected but the
                // xStocks universe has not landed yet.
                {
                    let needs_kraken_universe = self.fund_source_kraken
                        && self.kraken_enabled
                        && self.kraken_scrape_xstocks
                        && self.kraken_equity_universe_symbols.is_empty();
                    if needs_kraken_universe {
                        self.auto_fundamentals_deferred = true;
                        self.log.push_back(LogEntry::info(
                                "Fundamentals auto-scrape deferred: waiting for Kraken equities universe",
                            ));
                    } else if self.fund_source_kraken
                        && self.kraken_enabled
                        && self.kraken_scrape_xstocks
                        && !should_auto_start_kraken_fundamentals_scrape(
                            self.kraken_equity_universe_symbols.len(),
                        )
                    {
                        self.auto_fundamentals_deferred = false;
                        self.auto_fundamentals_started = false;
                        self.log.push_back(LogEntry::info(format!(
                                "Fundamentals auto-scrape skipped for broad Kraken xStocks universe ({} symbols); use manual Fundamentals scrape for full-universe backfill",
                                self.kraken_equity_universe_symbols.len()
                            )));
                    } else {
                        let db_path = cache_db_path();
                        let _ = self.broker_tx.send(BrokerCmd::FundamentalsScrape {
                            db_path,
                            use_alpaca: self.fund_source_alpaca,
                            use_kraken: self.fund_source_kraken,
                            kraken_equity_symbols: self.kraken_equity_universe_symbols.clone(),
                            force: false,
                        });
                        self.auto_fundamentals_started = true;
                        self.log.push_back(LogEntry::info(
                            "Fundamentals scrape started for selected source universes...",
                        ));
                    }
                }
            }
        }

        // ── Global font/spacing to match old WebKit (Consolas 11px) ──────
        if self.frame_count == 1 {
            let mut style = (*ctx.global_style()).clone();
            // ── AESTHETIC: Godel Terminal + old WebKit ──
            // Monospace everything, compact, square, green accents
            style.text_styles.insert(
                egui::TextStyle::Small,
                egui::FontId::new(10.0, egui::FontFamily::Monospace),
            );
            style.text_styles.insert(
                egui::TextStyle::Body,
                egui::FontId::new(11.0, egui::FontFamily::Monospace),
            );
            style.text_styles.insert(
                egui::TextStyle::Monospace,
                egui::FontId::new(11.0, egui::FontFamily::Monospace),
            );
            style.text_styles.insert(
                egui::TextStyle::Button,
                egui::FontId::new(10.0, egui::FontFamily::Monospace),
            );
            style.text_styles.insert(
                egui::TextStyle::Heading,
                egui::FontId::new(12.0, egui::FontFamily::Monospace),
            );
            // Compact but readable spacing
            style.spacing.item_spacing = egui::vec2(6.0, 2.0);
            style.spacing.button_padding = egui::vec2(4.0, 1.0);
            style.spacing.interact_size = egui::vec2(16.0, 14.0);
            style.spacing.indent = 8.0;
            style.spacing.scroll = egui::style::ScrollStyle {
                bar_width: 4.0,
                ..style.spacing.scroll
            };
            // ALL SQUARE — zero corner radius
            style.visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(0);
            style.visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(0);
            style.visuals.widgets.active.corner_radius = egui::CornerRadius::same(0);
            style.visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(0);
            // Thin widget borders
            style.visuals.widgets.inactive.bg_stroke =
                egui::Stroke::new(0.5, egui::Color32::from_rgb(35, 40, 55));
            style.visuals.widgets.hovered.bg_stroke =
                egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 65, 90));
            style.visuals.widgets.noninteractive.bg_stroke =
                egui::Stroke::new(0.0, egui::Color32::TRANSPARENT);
            ctx.set_global_style(style);
        }

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
            if !self.heavy_sync_in_progress || bg_window_visible {
                self.replace_bg_snapshot_off_ui_drop(data);
            } else {
                tracing::debug!(
                    "Deferred BG snapshot apply during heavy sync (sec_filings={}, details={})",
                    data.sec_filings.len(),
                    data.detailed_stats.len()
                );
                self.drop_bg_snapshot_off_ui(data);
            }
        }

        // ── deferred chart loading: non-blocking, paced attempts ──
        // Uses try_load() which returns false if cache Mutex is contended (compaction, broker sync).
        // Failed loads stay queued. The actual load is still expensive — cache read + GPU
        // indicators + MTF overlays — so pace restored MTF grids instead of burning
        // consecutive UI frames while broad sync/news/SEC/fundamentals are active.
        if !self.deferred_chart_loads.is_empty() {
            let load_interval =
                deferred_chart_load_interval(self.heavy_sync_in_progress, self.mtf_enabled);
            if now_instant.duration_since(self.deferred_chart_last_load_at) >= load_interval {
                let idx = self.deferred_chart_loads[0]; // VecDeque supports indexing
                let focused_chart = self.mtf_focused.unwrap_or(self.active_tab);
                let defer_inactive_mtf_cell = self.heavy_sync_in_progress
                    && self.mtf_enabled
                    && idx != self.active_tab
                    && idx != focused_chart;
                if defer_inactive_mtf_cell {
                    if let Some(skipped_idx) = self.deferred_chart_loads.pop_front() {
                        self.deferred_chart_loads.push_back(skipped_idx);
                    }
                    self.deferred_chart_last_load_at = now_instant;
                    ctx.request_repaint_after(load_interval);
                } else {
                    let mut loaded = false;
                    if let Some(cache) = self.cache.clone() {
                        if let Some(chart) = self.charts.get_mut(idx) {
                            let mut gpu = self.gpu_indicators.take();
                            loaded = chart.try_load(&cache, &mut self.log, gpu.as_mut());
                            self.gpu_indicators = gpu;
                        } else {
                            loaded = true; // invalid index, skip
                        }
                    }
                    if loaded {
                        self.deferred_chart_last_load_at = now_instant;
                        if let Some(done_idx) = self.deferred_chart_loads.pop_front() {
                            self.deferred_chart_load_set.remove(&done_idx);
                        }
                    }
                    // If !loaded, leave in queue — will retry after the pacing interval
                    // when the Mutex is free.
                }
            }
        }

        // ── recompute indicators when periods changed in UI ──────────────
        if self.indicators_dirty {
            self.indicators_dirty = false;
            let mut gpu = self.gpu_indicators.take();
            // MAX PERFORMANCE: During heavy sync, completely skip indicator computation
            // for everything except the single active chart, and even then only if
            // we are not in a forming bar update (which has its own O(1) path).
            if let Some(chart) = self.charts.get_mut(self.active_tab) {
                if chart.bars.is_empty() {
                    // O(1) skip
                } else if !self.heavy_sync_in_progress {
                    chart.compute_indicators_gpu(gpu.as_mut());
                } else if chart.forming_bar_dirty {
                    chart.compute_indicators_gpu(gpu.as_mut());
                }
            }
            self.gpu_indicators = gpu;
        }

        // ── receive MTF grid status from background thread (non-blocking) ──
        if let Some(ref rx) = self.mtf_grid_rx {
            if let Ok(results) = rx.try_recv() {
                // Merge with any preloaded data already in mtf_grid_status
                self.mtf_grid_status.extend(results);
                self.mtf_grid_status.sort_by_key(|r| {
                    MTF_GRID_TIMEFRAMES
                        .iter()
                        .position(|(label, _)| *label == r.0)
                        .unwrap_or(usize::MAX)
                });
                self.mtf_grid_rx = None; // done
            }
        }

        // ── receive Reg SHO cached prices from background thread (non-blocking) ──
        if let Some(ref rx) = self.regulatory_prices_rx {
            if let Ok(results) = rx.try_recv() {
                for (sym, row) in results {
                    self.regulatory_prices.insert(sym, row);
                }
                self.regulatory_prices_rx = None; // done
            }
        }

        // ── poll async broker messages ───────────────────────────────────
        perf_pre_broker_ms = now_instant.elapsed().as_secs_f64() * 1000.0;
        // Cap drain per frame so a flood of messages can't stall the render thread.
        // Anything left over waits for next frame; we repaint immediately in that case.
        let mut msgs_drained = 0usize;
        let broker_drain_max = 48;
        let broker_drain_started = std::time::Instant::now();
        let broker_drain_budget = if self.heavy_sync_in_progress {
            std::time::Duration::from_millis(4)
        } else {
            std::time::Duration::from_millis(8)
        };
        let mut market_data_refill_requested = false;
        while msgs_drained < broker_drain_max
            && broker_drain_started.elapsed() < broker_drain_budget
            && let Ok(msg) = self.broker_rx.try_recv()
        {
            msgs_drained += 1;
            let msg_kind = broker_msg_kind(&msg);
            let msg_started = std::time::Instant::now();
            match msg {
                BrokerMsg::Connected(s) => {
                    self.handle_broker_connected(s);
                }
                BrokerMsg::KrakenTrades(trades) => {
                    self.handle_kraken_trades(trades);
                }
                BrokerMsg::KrakenLiveTrade(trade) => {
                    self.handle_kraken_live_trade(trade);
                }
                BrokerMsg::KrakenOpenOrders(orders) => {
                    self.handle_kraken_open_orders(orders);
                }
                BrokerMsg::KrakenWsStatus { status, message } => {
                    self.handle_kraken_ws_status(status, message);
                }
                BrokerMsg::KrakenOrderbookUpdate(text) => {
                    let was_empty = self.orderbook_result.is_empty();
                    self.orderbook_result = text;
                    self.show_orderbook_window = true;
                    if was_empty {
                        self.log
                            .push_back(LogEntry::info("Kraken orderbook WS: live depth streaming"));
                    }
                }
                BrokerMsg::KrakenBookQuoteTick { symbol, bid, ask } => {
                    self.handle_kraken_book_quote_tick(symbol, bid, ask);
                }
                BrokerMsg::KrakenWsBarsCommitted { fresh } => {
                    self.handle_kraken_ws_bars_committed(fresh);
                }
                BrokerMsg::KrakenWsOhlcStatus {
                    interval_min,
                    kind,
                    detail,
                } => {
                    self.handle_kraken_ws_ohlc_status(interval_min, kind, detail);
                }
                BrokerMsg::KrakenWsOhlcSnapshotSweepSettled {
                    interval_min,
                    pair_count,
                    error,
                } => {
                    self.handle_kraken_ws_ohlc_snapshot_sweep_settled(
                        interval_min,
                        pair_count,
                        error,
                    );
                }
                BrokerMsg::Error(e) => {
                    let now = chrono::Utc::now().timestamp();
                    self.handle_broker_error(e, now);
                }
                BrokerMsg::Unresolvable {
                    broker,
                    symbol,
                    timeframe,
                    reason,
                } => {
                    self.unresolvable_mark(&broker, &symbol, &timeframe, &reason);
                }
                BrokerMsg::Account(acct) => {
                    self.handle_alpaca_account(acct);
                }
                BrokerMsg::Positions(pos) => {
                    self.handle_alpaca_positions(pos);
                }
                BrokerMsg::AllAssets(assets) => {
                    self.handle_alpaca_all_assets(assets);
                }
                BrokerMsg::RecentFills(fills) => {
                    self.handle_alpaca_recent_fills(fills);
                }
                BrokerMsg::BarsSynced(changed) => {
                    // Reload all visible charts to pick up newly-synced bars
                    if changed > 0 {
                        if self.mtf_enabled {
                            for i in 0..self.charts.len() {
                                self.queue_chart_reload(i);
                            }
                        } else {
                            self.queue_chart_reload(self.active_tab);
                        }
                    }
                }
                BrokerMsg::KrakenPositions(pos) => {
                    self.handle_kraken_positions(pos);
                }
                BrokerMsg::Orders(orders) => {
                    self.handle_alpaca_orders(orders);
                }
                BrokerMsg::OrderResult(msg) => {
                    self.handle_order_result(msg);
                }
                msg @ (BrokerMsg::SecScrapeResult(_)
                | BrokerMsg::FilingContent(_)
                | BrokerMsg::FinnhubNewsResult(_)) => {
                    self.handle_news_sec_result_msg(msg);
                }
                BrokerMsg::KrakenEquityUniverse(markets) => {
                    market_data_refill_requested |= self.handle_kraken_equity_universe(markets);
                }
                BrokerMsg::KrakenEquityQuote(ticker) => {
                    self.handle_kraken_equity_quote(ticker);
                }
                BrokerMsg::KrakenEquityBars {
                    symbol,
                    timeframe,
                    count,
                } => {
                    market_data_refill_requested |=
                        self.handle_kraken_equity_bars(symbol, timeframe, count);
                }
                BrokerMsg::KrakenEquityHistoryError {
                    symbol,
                    timeframe,
                    error,
                } => {
                    market_data_refill_requested |=
                        self.handle_kraken_equity_history_error(symbol, timeframe, error);
                }
                BrokerMsg::Quote(symbol, bid, ask, last) => {
                    self.handle_broker_quote(symbol, bid, ask, last);
                }
                BrokerMsg::WatchlistQuotes(rows) => {
                    self.handle_watchlist_quotes(rows);
                }
                BrokerMsg::KrakenBalances(balances) => {
                    self.handle_kraken_balances(balances);
                }
                BrokerMsg::KrakenPairs(pairs) => {
                    self.handle_kraken_pairs(pairs);
                }
                BrokerMsg::KrakenFuturesInstruments(symbols) => {
                    self.handle_kraken_futures_instruments(symbols);
                }
                msg @ (BrokerMsg::CryptoTop50(_)
                | BrokerMsg::FredData(_, _)
                | BrokerMsg::EconCalendarData(_)
                | BrokerMsg::CongressData(_)) => {
                    self.handle_macro_alt_data_msg(msg);
                }
                msg @ (BrokerMsg::CompanyProfile(_)
                | BrokerMsg::StockPeers(_, _)
                | BrokerMsg::EarningsHistory(_, _)
                | BrokerMsg::IpoCalendar(_)
                | BrokerMsg::PressReleases(_, _)
                | BrokerMsg::SocialSentiment(_, _)
                | BrokerMsg::TranscriptList(_, _)
                | BrokerMsg::TranscriptBody(_)
                | BrokerMsg::CommoditiesQuotes(_)
                | BrokerMsg::DividendHistory(_, _)
                | BrokerMsg::EarningsEstimates(_, _)
                | BrokerMsg::RatingChanges(_, _)
                | BrokerMsg::TreasuryYields(_)
                | BrokerMsg::FinancialStatementsMsg(_, _)
                | BrokerMsg::Executives(_, _)
                | BrokerMsg::CotReports(_)
                | BrokerMsg::StockSplitsMsg(_, _)
                | BrokerMsg::EtfHoldingsMsg(_, _)
                | BrokerMsg::AnalystRecsMsg(_, _)
                | BrokerMsg::PriceTargetMsg(_, _)
                | BrokerMsg::EsgScoresMsg(_, _)
                | BrokerMsg::IndexMembersMsg(_, _)
                | BrokerMsg::InsiderTradesMsg(_, _)
                | BrokerMsg::InstitutionalHoldersMsg(_, _)
                | BrokerMsg::SharesFloatMsg(_, _)
                | BrokerMsg::HistoricalPriceMsg(_, _)
                | BrokerMsg::EarningsSurpriseMsg(_, _)) => {
                    self.handle_research_core_msg(msg);
                }
                msg @ (BrokerMsg::WorldIndicesMsg(_)
                | BrokerMsg::MarketMoversMsg(_)
                | BrokerMsg::SectorPerformanceMsg(_)
                | BrokerMsg::WaccSnapshotMsg(_, _)
                | BrokerMsg::CurrencyRatesMsg(_)
                | BrokerMsg::BetaSnapshotMsg(_, _)
                | BrokerMsg::DdmSnapshotMsg(_, _)
                | BrokerMsg::RelativeValuationMsg(_, _)
                | BrokerMsg::FigiSnapshotMsg(_, _)
                | BrokerMsg::HraSnapshotMsg(_, _)
                | BrokerMsg::DcfSnapshotMsg(_, _)
                | BrokerMsg::SvmSnapshotMsg(_, _)
                | BrokerMsg::OptionsChainMsg(_, _)
                | BrokerMsg::IvolSnapshotMsg(_, _)
                | BrokerMsg::SeasonalitySnapshotMsg(_, _)
                | BrokerMsg::CorrelationMatrixMsg(_, _)
                | BrokerMsg::TotalReturnSnapshotMsg(_, _)
                | BrokerMsg::TechnicalsSnapshotMsg(_, _)
                | BrokerMsg::VolSkewSnapshotMsg(_, _)) => {
                    self.handle_research_macro_valuation_msg(msg);
                }
                msg @ (BrokerMsg::LeverageSnapshotMsg(_, _)
                | BrokerMsg::AccrualsSnapshotMsg(_, _)
                | BrokerMsg::RealizedVolSnapshotMsg(_, _)
                | BrokerMsg::FcfYieldSnapshotMsg(_, _)
                | BrokerMsg::ShortInterestSnapshotMsg(_, _)
                | BrokerMsg::AltmanZSnapshotMsg(_, _)
                | BrokerMsg::PiotroskiSnapshotMsg(_, _)
                | BrokerMsg::OhlcVolSnapshotMsg(_, _)
                | BrokerMsg::EpsBeatSnapshotMsg(_, _)
                | BrokerMsg::PriceTargetDispersionSnapshotMsg(_, _)
                | BrokerMsg::InsiderActivitySnapshotMsg(_, _)
                | BrokerMsg::DivgSnapshotMsg(_, _)
                | BrokerMsg::EarmSnapshotMsg(_, _)
                | BrokerMsg::SectorRotationSnapshotMsg(_, _)
                | BrokerMsg::UpdmSnapshotMsg(_, _)
                | BrokerMsg::MomentumSnapshotMsg(_, _)
                | BrokerMsg::LiquiditySnapshotMsg(_, _)
                | BrokerMsg::BreakoutSnapshotMsg(_, _)
                | BrokerMsg::CashCycleSnapshotMsg(_, _)
                | BrokerMsg::CreditSnapshotMsg(_, _)
                | BrokerMsg::GrowmSnapshotMsg(_, _)
                | BrokerMsg::FlowSnapshotMsg(_, _)
                | BrokerMsg::RegimeSnapshotMsg(_, _)
                | BrokerMsg::RelvolSnapshotMsg(_, _)
                | BrokerMsg::MarginsSnapshotMsg(_, _)
                | BrokerMsg::ValSnapshotMsg(_, _)
                | BrokerMsg::QualSnapshotMsg(_, _)
                | BrokerMsg::RiskSnapshotMsg(_, _)
                | BrokerMsg::InsstrkSnapshotMsg(_, _)
                | BrokerMsg::CovgSnapshotMsg(_, _)
                | BrokerMsg::VrkSnapshotMsg(_, _)
                | BrokerMsg::QrkSnapshotMsg(_, _)
                | BrokerMsg::RrkSnapshotMsg(_, _)
                | BrokerMsg::RelepsgrSnapshotMsg(_, _)
                | BrokerMsg::PeadSnapshotMsg(_, _)) => {
                    self.handle_research_snapshot_msg(msg);
                }
                msg @ (BrokerMsg::SizefSnapshotMsg(_, _)
                | BrokerMsg::MomfSnapshotMsg(_, _)
                | BrokerMsg::PeadrankSnapshotMsg(_, _)
                | BrokerMsg::FqmSnapshotMsg(_, _)
                | BrokerMsg::RevrankSnapshotMsg(_, _)
                | BrokerMsg::LevrankSnapshotMsg(_, _)
                | BrokerMsg::OperankSnapshotMsg(_, _)
                | BrokerMsg::FqmrankSnapshotMsg(_, _)
                | BrokerMsg::LiqrankSnapshotMsg(_, _)
                | BrokerMsg::SurpstkSnapshotMsg(_, _)
                | BrokerMsg::DvdrankSnapshotMsg(_, _)
                | BrokerMsg::EarmrankSnapshotMsg(_, _)
                | BrokerMsg::UpdgrankSnapshotMsg(_, _)
                | BrokerMsg::GySnapshotMsg(_, _)
                | BrokerMsg::DesSnapshotMsg(_, _)
                | BrokerMsg::DvdyieldrankSnapshotMsg(_, _)
                | BrokerMsg::ShrankSnapshotMsg(_, _)
                | BrokerMsg::ShortrankDeltaSnapshotMsg(_, _)
                | BrokerMsg::InsiderconcSnapshotMsg(_, _)
                | BrokerMsg::AtrannSnapshotMsg(_, _)
                | BrokerMsg::DdhistSnapshotMsg(_, _)
                | BrokerMsg::PriceperfSnapshotMsg(_, _)
                | BrokerMsg::MomrankMultiSnapshotMsg(_, _)
                | BrokerMsg::BetarankSnapshotMsg(_, _)
                | BrokerMsg::PegrankSnapshotMsg(_, _)
                | BrokerMsg::FhighlowSnapshotMsg(_, _)
                | BrokerMsg::RvconeSnapshotMsg(_, _)
                | BrokerMsg::CalpbSnapshotMsg(_, _)
                | BrokerMsg::CorrstkSnapshotMsg(_, _)
                | BrokerMsg::TlrankSnapshotMsg(_, _)
                | BrokerMsg::CorrrankSnapshotMsg(_, _)
                | BrokerMsg::OperankDeltaSnapshotMsg(_, _)
                | BrokerMsg::DivaccSnapshotMsg(_, _)
                | BrokerMsg::EpsaccSnapshotMsg(_, _)
                | BrokerMsg::VrpSnapshotMsg(_, _)
                | BrokerMsg::RetskewSnapshotMsg(_, _)
                | BrokerMsg::RetkurtSnapshotMsg(_, _)
                | BrokerMsg::TailrSnapshotMsg(_, _)
                | BrokerMsg::RunlenSnapshotMsg(_, _)
                | BrokerMsg::DayrangeSnapshotMsg(_, _)
                | BrokerMsg::AutocorSnapshotMsg(_, _)
                | BrokerMsg::HurstSnapshotMsg(_, _)
                | BrokerMsg::HitrateSnapshotMsg(_, _)
                | BrokerMsg::GlasymSnapshotMsg(_, _)
                | BrokerMsg::VolratioSnapshotMsg(_, _)
                | BrokerMsg::DrawupSnapshotMsg(_, _)
                | BrokerMsg::GapstatsSnapshotMsg(_, _)
                | BrokerMsg::VolclusterSnapshotMsg(_, _)
                | BrokerMsg::CloseplcSnapshotMsg(_, _)
                | BrokerMsg::MrhlSnapshotMsg(_, _)
                | BrokerMsg::DownvolSnapshotMsg(_, _)
                | BrokerMsg::SharprSnapshotMsg(_, _)
                | BrokerMsg::EffratioSnapshotMsg(_, _)
                | BrokerMsg::WickbiasSnapshotMsg(_, _)
                | BrokerMsg::VolofvolSnapshotMsg(_, _)) => {
                    self.handle_research_rank_snapshot_msg(msg);
                }
                msg @ (BrokerMsg::CalmarSnapshotMsg(_, _)
                | BrokerMsg::UlcerSnapshotMsg(_, _)
                | BrokerMsg::VarratioSnapshotMsg(_, _)
                | BrokerMsg::AmihudSnapshotMsg(_, _)
                | BrokerMsg::JbnormSnapshotMsg(_, _)
                | BrokerMsg::OmegaSnapshotMsg(_, _)
                | BrokerMsg::DfaSnapshotMsg(_, _)
                | BrokerMsg::BurkeSnapshotMsg(_, _)
                | BrokerMsg::MonthseasSnapshotMsg(_, _)
                | BrokerMsg::RollsprdSnapshotMsg(_, _)
                | BrokerMsg::ParkinsonSnapshotMsg(_, _)
                | BrokerMsg::GkvolSnapshotMsg(_, _)
                | BrokerMsg::RsvolSnapshotMsg(_, _)
                | BrokerMsg::CvarSnapshotMsg(_, _)
                | BrokerMsg::DoweffectSnapshotMsg(_, _)
                | BrokerMsg::SterlingSnapshotMsg(_, _)
                | BrokerMsg::KellyfSnapshotMsg(_, _)
                | BrokerMsg::LjungbSnapshotMsg(_, _)
                | BrokerMsg::RunstestSnapshotMsg(_, _)
                | BrokerMsg::ZeroretSnapshotMsg(_, _)
                | BrokerMsg::PsrSnapshotMsg(_, _)
                | BrokerMsg::AdfSnapshotMsg(_, _)
                | BrokerMsg::MnkendallSnapshotMsg(_, _)
                | BrokerMsg::BipowerSnapshotMsg(_, _)
                | BrokerMsg::DddurSnapshotMsg(_, _)
                | BrokerMsg::HilltailSnapshotMsg(_, _)
                | BrokerMsg::ArchlmSnapshotMsg(_, _)
                | BrokerMsg::PainratioSnapshotMsg(_, _)
                | BrokerMsg::CusumSnapshotMsg(_, _)
                | BrokerMsg::CfvarSnapshotMsg(_, _)
                | BrokerMsg::EntropySnapshotMsg(_, _)
                | BrokerMsg::RachevSnapshotMsg(_, _)
                | BrokerMsg::GprSnapshotMsg(_, _)
                | BrokerMsg::PacfSnapshotMsg(_, _)
                | BrokerMsg::ApenSnapshotMsg(_, _)
                | BrokerMsg::UprSnapshotMsg(_, _)
                | BrokerMsg::LevereffSnapshotMsg(_, _)
                | BrokerMsg::DrawdarSnapshotMsg(_, _)
                | BrokerMsg::VarhalfSnapshotMsg(_, _)
                | BrokerMsg::GiniSnapshotMsg(_, _)
                | BrokerMsg::SampenSnapshotMsg(_, _)
                | BrokerMsg::PermenSnapshotMsg(_, _)
                | BrokerMsg::RecfactSnapshotMsg(_, _)
                | BrokerMsg::KpssSnapshotMsg(_, _)
                | BrokerMsg::SpecentSnapshotMsg(_, _)
                | BrokerMsg::RobvolSnapshotMsg(_, _)
                | BrokerMsg::RenyientSnapshotMsg(_, _)
                | BrokerMsg::RetquantSnapshotMsg(_, _)
                | BrokerMsg::MsentSnapshotMsg(_, _)
                | BrokerMsg::EwmavolSnapshotMsg(_, _)
                | BrokerMsg::KsnormSnapshotMsg(_, _)
                | BrokerMsg::AdtestSnapshotMsg(_, _)
                | BrokerMsg::LmomSnapshotMsg(_, _)
                | BrokerMsg::KylelamSnapshotMsg(_, _)
                | BrokerMsg::PeakoverSnapshotMsg(_, _)
                | BrokerMsg::HiguchiSnapshotMsg(_, _)
                | BrokerMsg::PickandsSnapshotMsg(_, _)
                | BrokerMsg::Kappa3SnapshotMsg(_, _)
                | BrokerMsg::LyapunovSnapshotMsg(_, _)
                | BrokerMsg::RankacSnapshotMsg(_, _)
                | BrokerMsg::BnsjumpSnapshotMsg(_, _)
                | BrokerMsg::PprootSnapshotMsg(_, _)
                | BrokerMsg::MfdfaSnapshotMsg(_, _)
                | BrokerMsg::HillksSnapshotMsg(_, _)
                | BrokerMsg::TsiSnapshotMsg(_, _)
                | BrokerMsg::Garch11SnapshotMsg(_, _)
                | BrokerMsg::SadfSnapshotMsg(_, _)
                | BrokerMsg::CordimSnapshotMsg(_, _)
                | BrokerMsg::SkspecSnapshotMsg(_, _)
                | BrokerMsg::AutomiSnapshotMsg(_, _)) => {
                    self.handle_research_quant_snapshot_msg(msg);
                }
                msg @ (BrokerMsg::DurbinWatsonSnapshotMsg(_, _)
                | BrokerMsg::BdsTestSnapshotMsg(_, _)
                | BrokerMsg::BreuschPaganSnapshotMsg(_, _)
                | BrokerMsg::TurnPtsSnapshotMsg(_, _)
                | BrokerMsg::PeriodogramSnapshotMsg(_, _)
                | BrokerMsg::McLeodLiSnapshotMsg(_, _)
                | BrokerMsg::OuFitSnapshotMsg(_, _)
                | BrokerMsg::GphSnapshotMsg(_, _)
                | BrokerMsg::BurgSpecSnapshotMsg(_, _)
                | BrokerMsg::KendallTauSnapshotMsg(_, _)
                | BrokerMsg::SqueezeSnapshotMsg(_, _)
                | BrokerMsg::SqueezeRankSnapshotMsg(_, _)
                | BrokerMsg::SqueezeWatchlistLoaded(_)
                | BrokerMsg::BbsqueezeSnapshotMsg(_, _)
                | BrokerMsg::DonchianSnapshotMsg(_, _)
                | BrokerMsg::KamaSnapshotMsg(_, _)
                | BrokerMsg::IchimokuSnapshotMsg(_, _)
                | BrokerMsg::SupertrendSnapshotMsg(_, _)
                | BrokerMsg::KeltnerSnapshotMsg(_, _)
                | BrokerMsg::FisherSnapshotMsg(_, _)
                | BrokerMsg::AroonSnapshotMsg(_, _)
                | BrokerMsg::AdxSnapshotMsg(_, _)
                | BrokerMsg::CciSnapshotMsg(_, _)
                | BrokerMsg::CmfSnapshotMsg(_, _)
                | BrokerMsg::MfiSnapshotMsg(_, _)
                | BrokerMsg::PsarSnapshotMsg(_, _)
                | BrokerMsg::VortexSnapshotMsg(_, _)
                | BrokerMsg::ChopSnapshotMsg(_, _)
                | BrokerMsg::ObvSnapshotMsg(_, _)
                | BrokerMsg::TrixSnapshotMsg(_, _)
                | BrokerMsg::HmaSnapshotMsg(_, _)
                | BrokerMsg::PpoSnapshotMsg(_, _)
                | BrokerMsg::DpoSnapshotMsg(_, _)
                | BrokerMsg::KstSnapshotMsg(_, _)
                | BrokerMsg::UltoscSnapshotMsg(_, _)
                | BrokerMsg::WillrSnapshotMsg(_, _)
                | BrokerMsg::MassSnapshotMsg(_, _)
                | BrokerMsg::ChaikoscSnapshotMsg(_, _)
                | BrokerMsg::KlingerSnapshotMsg(_, _)
                | BrokerMsg::StochRsiSnapshotMsg(_, _)
                | BrokerMsg::AwesomeSnapshotMsg(_, _)
                | BrokerMsg::EfiSnapshotMsg(_, _)
                | BrokerMsg::EmvSnapshotMsg(_, _)
                | BrokerMsg::NviSnapshotMsg(_, _)
                | BrokerMsg::PviSnapshotMsg(_, _)
                | BrokerMsg::CoppockSnapshotMsg(_, _)
                | BrokerMsg::CmoSnapshotMsg(_, _)
                | BrokerMsg::QstickSnapshotMsg(_, _)
                | BrokerMsg::DisparitySnapshotMsg(_, _)
                | BrokerMsg::BopSnapshotMsg(_, _)
                | BrokerMsg::SchaffSnapshotMsg(_, _)
                | BrokerMsg::StochSnapshotMsg(_, _)
                | BrokerMsg::MacdSnapshotMsg(_, _)
                | BrokerMsg::VwapSnapshotMsg(_, _)
                | BrokerMsg::McgdSnapshotMsg(_, _)
                | BrokerMsg::RwiSnapshotMsg(_, _)) => {
                    self.handle_indicator_snapshot_msg(msg);
                }
                msg @ (BrokerMsg::DemaSnapshotMsg(_, _)
                | BrokerMsg::TemaSnapshotMsg(_, _)
                | BrokerMsg::LinregSnapshotMsg(_, _)
                | BrokerMsg::PivotsSnapshotMsg(_, _)
                | BrokerMsg::HeikinSnapshotMsg(_, _)
                | BrokerMsg::AlmaSnapshotMsg(_, _)
                | BrokerMsg::ZlemaSnapshotMsg(_, _)
                | BrokerMsg::ElderRaySnapshotMsg(_, _)
                | BrokerMsg::TsfSnapshotMsg(_, _)
                | BrokerMsg::RviSnapshotMsg(_, _)
                | BrokerMsg::TrimaSnapshotMsg(_, _)
                | BrokerMsg::T3SnapshotMsg(_, _)
                | BrokerMsg::VidyaSnapshotMsg(_, _)
                | BrokerMsg::SmiSnapshotMsg(_, _)
                | BrokerMsg::PvtSnapshotMsg(_, _)
                | BrokerMsg::AcSnapshotMsg(_, _)
                | BrokerMsg::ChvolSnapshotMsg(_, _)
                | BrokerMsg::BbwidthSnapshotMsg(_, _)
                | BrokerMsg::ElderImpSnapshotMsg(_, _)
                | BrokerMsg::RmiSnapshotMsg(_, _)
                | BrokerMsg::SymbolExpirationsMsg(_, _)
                | BrokerMsg::SmmaSnapshotMsg(_, _)
                | BrokerMsg::AlligatorSnapshotMsg(_, _)
                | BrokerMsg::CrsiSnapshotMsg(_, _)
                | BrokerMsg::SebSnapshotMsg(_, _)
                | BrokerMsg::ImiSnapshotMsg(_, _)
                | BrokerMsg::GmmaSnapshotMsg(_, _)
                | BrokerMsg::MaenvSnapshotMsg(_, _)
                | BrokerMsg::AdlSnapshotMsg(_, _)
                | BrokerMsg::VhfSnapshotMsg(_, _)
                | BrokerMsg::VrocSnapshotMsg(_, _)
                | BrokerMsg::KdjSnapshotMsg(_, _)
                | BrokerMsg::QqeSnapshotMsg(_, _)
                | BrokerMsg::PmoSnapshotMsg(_, _)
                | BrokerMsg::CfoSnapshotMsg(_, _)
                | BrokerMsg::TmfSnapshotMsg(_, _)
                | BrokerMsg::FractalsSnapshotMsg(_, _)
                | BrokerMsg::IftRsiSnapshotMsg(_, _)
                | BrokerMsg::MamaSnapshotMsg(_, _)
                | BrokerMsg::CogSnapshotMsg(_, _)
                | BrokerMsg::DidiSnapshotMsg(_, _)
                | BrokerMsg::DemarkerSnapshotMsg(_, _)
                | BrokerMsg::GatorSnapshotMsg(_, _)
                | BrokerMsg::BwMfiSnapshotMsg(_, _)
                | BrokerMsg::VwmaSnapshotMsg(_, _)
                | BrokerMsg::StddevSnapshotMsg(_, _)
                | BrokerMsg::WmaSnapshotMsg(_, _)
                | BrokerMsg::RainbowSnapshotMsg(_, _)
                | BrokerMsg::MesaSineSnapshotMsg(_, _)
                | BrokerMsg::FramaSnapshotMsg(_, _)
                | BrokerMsg::IbsSnapshotMsg(_, _)
                | BrokerMsg::LaguerreRsiSnapshotMsg(_, _)
                | BrokerMsg::ZigzagSnapshotMsg(_, _)
                | BrokerMsg::PgoSnapshotMsg(_, _)
                | BrokerMsg::HtTrendlineSnapshotMsg(_, _)
                | BrokerMsg::MidpointSnapshotMsg(_, _)
                | BrokerMsg::MassIndexSnapshotMsg(_, _)
                | BrokerMsg::NatrSnapshotMsg(_, _)
                | BrokerMsg::TtmSqueezeSnapshotMsg(_, _)
                | BrokerMsg::ForceIndexSnapshotMsg(_, _)
                | BrokerMsg::TrangeSnapshotMsg(_, _)
                | BrokerMsg::LinearregSlopeSnapshotMsg(_, _)
                | BrokerMsg::HtDcperiodSnapshotMsg(_, _)
                | BrokerMsg::HtTrendmodeSnapshotMsg(_, _)
                | BrokerMsg::AccbandsSnapshotMsg(_, _)
                | BrokerMsg::StochfSnapshotMsg(_, _)
                | BrokerMsg::LinearregSnapshotMsg(_, _)
                | BrokerMsg::LinearregAngleSnapshotMsg(_, _)
                | BrokerMsg::HtDcphaseSnapshotMsg(_, _)
                | BrokerMsg::HtSineSnapshotMsg(_, _)
                | BrokerMsg::HtPhasorSnapshotMsg(_, _)
                | BrokerMsg::MidpriceSnapshotMsg(_, _)
                | BrokerMsg::ApoSnapshotMsg(_, _)
                | BrokerMsg::MomSnapshotMsg(_, _)
                | BrokerMsg::SarextSnapshotMsg(_, _)
                | BrokerMsg::AdxrSnapshotMsg(_, _)
                | BrokerMsg::AvgpriceSnapshotMsg(_, _)
                | BrokerMsg::MedpriceSnapshotMsg(_, _)
                | BrokerMsg::TypPriceSnapshotMsg(_, _)
                | BrokerMsg::WclPriceSnapshotMsg(_, _)
                | BrokerMsg::VarianceSnapshotMsg(_, _)
                | BrokerMsg::PlusDiSnapshotMsg(_, _)
                | BrokerMsg::MinusDiSnapshotMsg(_, _)
                | BrokerMsg::PlusDmSnapshotMsg(_, _)
                | BrokerMsg::MinusDmSnapshotMsg(_, _)
                | BrokerMsg::DxSnapshotMsg(_, _)
                | BrokerMsg::RocSnapshotMsg(_, _)
                | BrokerMsg::RocpSnapshotMsg(_, _)
                | BrokerMsg::RocrSnapshotMsg(_, _)
                | BrokerMsg::Rocr100SnapshotMsg(_, _)
                | BrokerMsg::CorrelSnapshotMsg(_, _)
                | BrokerMsg::MinSnapshotMsg(_, _)
                | BrokerMsg::MaxSnapshotMsg(_, _)
                | BrokerMsg::MinMaxSnapshotMsg(_, _)
                | BrokerMsg::MinIndexSnapshotMsg(_, _)
                | BrokerMsg::MaxIndexSnapshotMsg(_, _)
                | BrokerMsg::BbandsSnapshotMsg(_, _)
                | BrokerMsg::AdSnapshotMsg(_, _)
                | BrokerMsg::AdoscSnapshotMsg(_, _)
                | BrokerMsg::SumSnapshotMsg(_, _)
                | BrokerMsg::LinearRegInterceptSnapshotMsg(_, _)
                | BrokerMsg::AroonoscSnapshotMsg(_, _)
                | BrokerMsg::MinMaxIndexSnapshotMsg(_, _)
                | BrokerMsg::MacdextSnapshotMsg(_, _)
                | BrokerMsg::MacdfixSnapshotMsg(_, _)
                | BrokerMsg::MavpSnapshotMsg(_, _)
                | BrokerMsg::CdlDojiSnapshotMsg(_, _)
                | BrokerMsg::CdlHammerSnapshotMsg(_, _)
                | BrokerMsg::CdlShootingStarSnapshotMsg(_, _)
                | BrokerMsg::CdlEngulfingSnapshotMsg(_, _)
                | BrokerMsg::CdlHaramiSnapshotMsg(_, _)
                | BrokerMsg::CdlMorningStarSnapshotMsg(_, _)
                | BrokerMsg::CdlEveningStarSnapshotMsg(_, _)
                | BrokerMsg::CdlThreeBlackCrowsSnapshotMsg(_, _)
                | BrokerMsg::CdlThreeWhiteSoldiersSnapshotMsg(_, _)
                | BrokerMsg::CdlDarkCloudCoverSnapshotMsg(_, _)
                | BrokerMsg::CdlPiercingSnapshotMsg(_, _)
                | BrokerMsg::CdlDragonflyDojiSnapshotMsg(_, _)
                | BrokerMsg::CdlGravestoneDojiSnapshotMsg(_, _)
                | BrokerMsg::CdlHangingManSnapshotMsg(_, _)
                | BrokerMsg::CdlInvertedHammerSnapshotMsg(_, _)
                | BrokerMsg::CdlHaramiCrossSnapshotMsg(_, _)
                | BrokerMsg::CdlLongLeggedDojiSnapshotMsg(_, _)
                | BrokerMsg::CdlMarubozuSnapshotMsg(_, _)
                | BrokerMsg::CdlSpinningTopSnapshotMsg(_, _)
                | BrokerMsg::CdlTristarSnapshotMsg(_, _)
                | BrokerMsg::CdlDojiStarSnapshotMsg(_, _)
                | BrokerMsg::CdlMorningDojiStarSnapshotMsg(_, _)
                | BrokerMsg::CdlEveningDojiStarSnapshotMsg(_, _)
                | BrokerMsg::CdlAbandonedBabySnapshotMsg(_, _)
                | BrokerMsg::CdlThreeInsideSnapshotMsg(_, _)
                | BrokerMsg::CdlBeltHoldSnapshotMsg(_, _)
                | BrokerMsg::CdlClosingMarubozuSnapshotMsg(_, _)
                | BrokerMsg::CdlHighWaveSnapshotMsg(_, _)
                | BrokerMsg::CdlLongLineSnapshotMsg(_, _)
                | BrokerMsg::CdlShortLineSnapshotMsg(_, _)
                | BrokerMsg::CdlCounterattackSnapshotMsg(_, _)
                | BrokerMsg::CdlHomingPigeonSnapshotMsg(_, _)
                | BrokerMsg::CdlInNeckSnapshotMsg(_, _)
                | BrokerMsg::CdlOnNeckSnapshotMsg(_, _)
                | BrokerMsg::CdlThrustingSnapshotMsg(_, _)
                | BrokerMsg::CdlTwoCrowsSnapshotMsg(_, _)
                | BrokerMsg::CdlThreeLineStrikeSnapshotMsg(_, _)
                | BrokerMsg::CdlThreeOutsideSnapshotMsg(_, _)
                | BrokerMsg::CdlMatchingLowSnapshotMsg(_, _)
                | BrokerMsg::CdlSeparatingLinesSnapshotMsg(_, _)
                | BrokerMsg::CdlStickSandwichSnapshotMsg(_, _)
                | BrokerMsg::CdlRickshawManSnapshotMsg(_, _)
                | BrokerMsg::CdlTakuriSnapshotMsg(_, _)
                | BrokerMsg::CdlThreeStarsInSouthSnapshotMsg(_, _)
                | BrokerMsg::CdlIdenticalThreeCrowsSnapshotMsg(_, _)
                | BrokerMsg::CdlKickingSnapshotMsg(_, _)
                | BrokerMsg::CdlKickingByLengthSnapshotMsg(_, _)
                | BrokerMsg::CdlLadderBottomSnapshotMsg(_, _)
                | BrokerMsg::CdlUniqueThreeRiverSnapshotMsg(_, _)
                | BrokerMsg::CdlAdvanceBlockSnapshotMsg(_, _)
                | BrokerMsg::CdlBreakawaySnapshotMsg(_, _)
                | BrokerMsg::CdlGapSideSideWhiteSnapshotMsg(_, _)
                | BrokerMsg::CdlUpsideGapTwoCrowsSnapshotMsg(_, _)
                | BrokerMsg::CdlXSideGapThreeMethodsSnapshotMsg(_, _)
                | BrokerMsg::CdlConcealBabySwallowSnapshotMsg(_, _)
                | BrokerMsg::CdlHikkakeSnapshotMsg(_, _)
                | BrokerMsg::CdlHikkakeModSnapshotMsg(_, _)
                | BrokerMsg::CdlMatHoldSnapshotMsg(_, _)
                | BrokerMsg::CdlRiseFallThreeMethodsSnapshotMsg(_, _)
                | BrokerMsg::CdlStalledPatternSnapshotMsg(_, _)
                | BrokerMsg::CdlTasukiGapSnapshotMsg(_, _)
                | BrokerMsg::ModSharpeSnapshotMsg(_, _)
                | BrokerMsg::HsiehTestSnapshotMsg(_, _)
                | BrokerMsg::ChowBreakSnapshotMsg(_, _)
                | BrokerMsg::DriftBurstSnapshotMsg(_, _)
                | BrokerMsg::HlvClustSnapshotMsg(_, _)
                | BrokerMsg::YangZhangSnapshotMsg(_, _)
                | BrokerMsg::KuiperSnapshotMsg(_, _)
                | BrokerMsg::DagostinoSnapshotMsg(_, _)
                | BrokerMsg::BaiPerronSnapshotMsg(_, _)
                | BrokerMsg::KupiecPofSnapshotMsg(_, _)) => {
                    self.handle_extended_indicator_snapshot_msg(msg);
                }
                msg @ (BrokerMsg::IngestResearchResult { .. }
                | BrokerMsg::NewsArticlesLoaded { .. }
                | BrokerMsg::NewsDbTotal(_)) => {
                    self.handle_news_ingest_msg(msg);
                }
                msg @ (BrokerMsg::UnusualVolumeResults(_) | BrokerMsg::MarketClock(_)) => {
                    self.handle_misc_broker_msg(msg);
                }
                BrokerMsg::JsonResult(label, text) => {
                    if label == "Kraken WS" {
                        tracing::debug!(
                            "Suppressed raw Kraken private WebSocket payload from UI log ({} bytes)",
                            text.len()
                        );
                        continue;
                    }
                    if label == "Account Activities" {
                        tracing::debug!(
                            "Suppressed raw account activities from UI log ({} bytes)",
                            text.len()
                        );
                        continue;
                    }
                    // Route structured results to their windows; log everything
                    if label.starts_with("Analyst:") {
                        self.analyst_result = text.clone();
                        self.show_analyst = true;
                    } else if label.starts_with("PriceTarget:") {
                        // Append price target to analyst window
                        self.analyst_result.push_str("\n---PRICE_TARGET---\n");
                        self.analyst_result.push_str(&text);
                        self.show_analyst = true;
                    } else if label.starts_with("Holders:") {
                        self.holders_result = text.clone();
                        self.show_holders = true;
                    } else if label.starts_with("Orderbook:") {
                        self.orderbook_result = text.clone();
                        self.show_orderbook_window = true;
                    } else if label == "FearGreed" {
                        // Parse "value|label" format
                        let parts: Vec<&str> = text.splitn(2, '|').collect();
                        if parts.len() == 2 {
                            self.fear_greed_value = parts[0].parse::<u32>().unwrap_or(50);
                            self.fear_greed_label = parts[1].to_string();
                        }
                    } else if label == "AiChat" {
                        self.maybe_queue_ingest_from_ai_response("ai_chat", &text);
                        self.ai_chat_history.push((false, text.clone()));
                        let sid = Self::ensure_session_id(&mut self.ai_chat_session_id);
                        let model = self.ai_model.clone();
                        let history = self.ai_chat_history.clone();
                        self.persist_ai_turn("ai_chat", &sid, None, &history, &model);
                    } else if label == "RedditWSB" {
                        if let Ok(posts) =
                            serde_json::from_str::<Vec<(String, String, u64, u64)>>(&text)
                        {
                            self.reddit_posts = posts;
                        }
                    } else if label == "MatrixMessages" {
                        if let Ok(msgs) =
                            serde_json::from_str::<Vec<(String, String, String)>>(&text)
                        {
                            self.matrix_messages = msgs;
                        }
                    } else if label == "MatrixJoined" {
                        self.log
                            .push_back(LogEntry::info("Matrix: joined community room"));
                    } else if label == "MatrixSent" {
                        // Re-fetch messages after sending
                        let _ = self.broker_tx.send(BrokerCmd::MatrixFetchMessages {
                            room_id: self.matrix_room.clone(),
                            access_token: self.matrix_access_token.clone(),
                        });
                    } else if let Some((card, summary)) = json_result_card_from_text(&label, &text)
                    {
                        self.result_card = Some((card, std::time::Instant::now()));
                        self.log.push_back(LogEntry::info(summary));
                        continue;
                    }
                    self.log
                        .push_back(LogEntry::info(format!("{}:\n{}", label, text)));
                }
                BrokerMsg::FundamentalsProgress(ref msg) => {
                    self.scrape_fund_last_msg = msg.clone();
                    // Parse progress from messages like "Scraped X: OK (5/100)" or "complete: X OK, Y failed..."
                    if msg.contains("stock tickers") || msg.contains("tickers found") {
                        self.scrape_fund_running = true;
                        self.scrape_fund_ok = 0;
                        self.scrape_fund_fail = 0;
                        self.scrape_fund_skipped = 0;
                        if let Some(n) =
                            msg.split_whitespace().find_map(|w| w.parse::<usize>().ok())
                        {
                            self.scrape_fund_total = self.scrape_fund_total.max(n);
                        }
                    } else if msg.contains(": OK") {
                        self.scrape_fund_ok += 1;
                    } else if msg.contains(": FAIL") {
                        self.scrape_fund_fail += 1;
                    } else if msg.contains("complete")
                        || msg.contains("Aborting")
                        || msg.starts_with("Fundamentals progress:")
                    {
                        if !msg.starts_with("Fundamentals progress:") {
                            self.scrape_fund_running = false;
                        }
                        // Parse final/progress counts from "X OK, Y failed, Z skipped ... out of N"
                        let parts: Vec<&str> = msg.split_whitespace().collect();
                        for (i, w) in parts.iter().enumerate() {
                            if *w == "OK," {
                                if let Some(n) = parts
                                    .get(i.wrapping_sub(1))
                                    .and_then(|s| s.parse::<usize>().ok())
                                {
                                    self.scrape_fund_ok = n;
                                }
                            }
                            if *w == "failed," {
                                if let Some(n) = parts
                                    .get(i.wrapping_sub(1))
                                    .and_then(|s| s.parse::<usize>().ok())
                                {
                                    self.scrape_fund_fail = n;
                                }
                            }
                            if *w == "skipped" {
                                if let Some(n) = parts
                                    .get(i.wrapping_sub(1))
                                    .and_then(|s| s.parse::<usize>().ok())
                                {
                                    self.scrape_fund_skipped = n;
                                }
                            }
                        }
                    }
                    if msg.starts_with("Fundamentals progress:") || is_routine_news_progress(msg) {
                        tracing::debug!("{}", msg);
                    } else {
                        self.log.push_back(LogEntry::info(msg.clone()));
                    }
                }
                BrokerMsg::SymbolSuggestions(results) => {
                    // Merge broker search results into autocomplete (if dropdown still visible)
                    // Normalize: remove slash from crypto (BTC/USD → BTCUSD) to avoid duplicates
                    if self.symbol_ac_visible {
                        for (sym, name, class) in results {
                            let normalized = sym.replace('/', "");
                            let existing = self.symbol_suggestions.iter_mut().find(|(s, _, _)| {
                                s.replace('/', "").eq_ignore_ascii_case(&normalized)
                            });
                            match existing {
                                // Already present from a local source. Local cache/universe
                                // entries carry an empty company name (e.g. WOK from
                                // cached_active_symbols); the broker search result *does*
                                // resolve the name ("WORK Medical Technology…"), so fill in
                                // the blanks instead of dropping the richer result.
                                Some((_, ex_name, ex_class)) => {
                                    if ex_name.trim().is_empty() && !name.trim().is_empty() {
                                        *ex_name = name;
                                    }
                                    if ex_class.trim().is_empty() && !class.trim().is_empty() {
                                        *ex_class = class;
                                    }
                                }
                                None => self.symbol_suggestions.push((normalized, name, class)),
                            }
                        }
                        self.symbol_suggestions.truncate(20);
                    }
                }
                msg @ (BrokerMsg::BarsFetched { .. }
                | BrokerMsg::AlpacaFetchSettled { .. }
                | BrokerMsg::KrakenFetchSettled { .. }
                | BrokerMsg::KrakenBackfillComplete { .. }
                | BrokerMsg::KrakenFuturesFetchSettled { .. }
                | BrokerMsg::KrakenFuturesBackfillComplete { .. }) => {
                    market_data_refill_requested |= self.handle_market_data_fetch_result_msg(msg);
                }
                BrokerMsg::AlpacaRateLimitObserved { historical_rpm } => {
                    if historical_rpm > 0 && self.alpaca_historical_rpm_observed != historical_rpm {
                        self.alpaca_historical_rpm_observed = historical_rpm;
                        let capacity = self.alpaca_sync_capacity();
                        self.push_alpaca_sync_runtime_config();
                        self.log.push_back(LogEntry::info(format!(
                            "Alpaca sync speed: detected {} req/min historical tier — {} workers, queue {}, batch {}",
                            historical_rpm,
                            capacity.fetch_permits,
                            capacity.queue_window,
                            capacity.batch_size
                        )));
                    }
                }
                BrokerMsg::AlpacaRetryEnqueue {
                    symbol,
                    timeframe,
                    reason,
                } => {
                    self.alpaca_retry_enqueue(&symbol, &timeframe, &reason);
                    let queue_len = self.alpaca_retry_queue.len();
                    tracing::debug!(
                        "Alpaca {} {}: queued for retry ({}) — {} in queue",
                        symbol,
                        timeframe,
                        reason,
                        queue_len
                    );
                    if should_emit_alpaca_retry_queue_log(queue_len) {
                        self.log.push_back(LogEntry::info(format!(
                            "Alpaca retry queue: {} symbols awaiting targeted probes (latest: {} {} — {})",
                            queue_len, symbol, timeframe, reason
                        )));
                    }
                }
                BrokerMsg::AlpacaNoData {
                    symbol,
                    timeframe,
                    reason,
                } => {
                    self.alpaca_retry_drain(&symbol, &timeframe);
                    let changed = self.alpaca_no_data_mark(&symbol, &timeframe, &reason);
                    let marker_count = self.alpaca_no_data_pairs.len();
                    let prefix = if changed {
                        "marked no-data"
                    } else {
                        "still no-data"
                    };
                    tracing::debug!(
                        "Alpaca {} {}: {} — automated sync will skip it ({} marked)",
                        symbol,
                        timeframe,
                        prefix,
                        marker_count
                    );
                    if changed && marker_count.is_multiple_of(100) {
                        self.log.push_back(LogEntry::warn(format!(
                            "Alpaca no-data milestone: {} provider-unavailable pairs tombstoned",
                            marker_count
                        )));
                    }
                }
                BrokerMsg::AlpacaBackfillComplete {
                    symbol,
                    timeframe,
                    bar_count,
                    target_bars,
                } => {
                    let changed = self.alpaca_backfill_complete_mark(
                        &symbol,
                        &timeframe,
                        bar_count,
                        target_bars,
                    );
                    if changed {
                        let marker_count = self.alpaca_backfill_complete_pairs.len();
                        tracing::debug!(
                            "Alpaca {} {}: marked backfill-complete at {}/{} bars ({} marked)",
                            symbol,
                            timeframe,
                            bar_count,
                            target_bars,
                            marker_count
                        );
                        if marker_count.is_multiple_of(100) {
                            self.log.push_back(LogEntry::info(format!(
                                "Alpaca backfill milestone: {} pairs at provider-window saturation",
                                marker_count
                            )));
                        }
                    }
                }
            }
            let msg_elapsed = msg_started.elapsed();
            if msg_elapsed > std::time::Duration::from_millis(25) {
                tracing::warn!(
                    "BrokerMsg::{msg_kind} handling took {:.2}ms on UI thread",
                    msg_elapsed.as_secs_f64() * 1000.0
                );
            }
        }
        if market_data_refill_requested {
            self.refill_market_data_sync_slots();
        }
        perf_broker_drain_ms = broker_drain_started.elapsed().as_secs_f64() * 1000.0;
        perf_after_broker_started = std::time::Instant::now();
        // If we hit the drain cap there are more messages waiting — repaint
        // immediately to process the next batch rather than waiting on the idle tick.
        if msgs_drained >= broker_drain_max || broker_drain_started.elapsed() >= broker_drain_budget
        {
            // Throttle live Kraken WS forming-bar updates to ~10 fps.
            // Full immediate repaint is only needed for closed bars or user action.
            // The forming_bar_dirty flag on ChartState is the signal from the WS path.
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }

        let post_broker_setup_started = std::time::Instant::now();
        self.sync_cross_timeframe_drawings();

        let pointer_over_floating = self.handle_runtime_input(ctx);
        perf_post_broker_setup_ms = post_broker_setup_started.elapsed().as_secs_f64() * 1000.0;

        let chrome_panels_started = std::time::Instant::now();
        self.render_menu_bar(ctx);
        self.render_symbol_timeframe_toolbar(ctx);
        self.render_symbol_autocomplete_dropdown(ctx);

        self.render_tab_bar(ctx);
        self.render_bottom_panels(ctx);

        self.render_right_panel(ctx);
        perf_chrome_panels_ms = chrome_panels_started.elapsed().as_secs_f64() * 1000.0;

        // ── floating windows ─────────────────────────────────────────────────
        // Always call draw_floating_windows so close buttons work.
        // Performance: all background data reads from self.bg (background-computed).
        let floating_windows_started = std::time::Instant::now();
        self.draw_floating_windows(ctx);
        perf_floating_windows_ms = floating_windows_started.elapsed().as_secs_f64() * 1000.0;

        // === Regulatory floating windows (Reg SHO + Halts) ===
        // Populate price columns for every regulatory-alert symbol (not just the
        // few in the watchlist). On open, force a market-data refresh ordered
        // least-fresh / no-data first; then re-read cached daily bars off the
        // render thread on a throttle so fetched bars surface while open.
        if self.show_reg_sho_window || self.show_halts_window {
            if !self.regulatory_prices_loaded && !self.bg.regulatory_alerts_by_symbol.is_empty() {
                self.refresh_regulatory_prices();
                self.regulatory_prices_loaded = true;
            }
            let read_due = self
                .regulatory_price_read_at
                .map(|at| at.elapsed() >= std::time::Duration::from_secs(3))
                .unwrap_or(true);
            if read_due && self.regulatory_prices_rx.is_none() {
                self.spawn_regulatory_price_load();
                self.regulatory_price_read_at = Some(std::time::Instant::now());
            }
        }

        if self.show_reg_sho_window {
            let mut open = true;
            // Button clicks are collected here and applied after the window
            // closure (which holds an immutable borrow of self).
            let mut reg_sho_action: Option<SymbolAction> = None;
            let mut reg_sho_refresh = false;
            egui::Window::new("Reg SHO Threshold Securities")
                .open(&mut open)
                .default_width(960.0)
                .default_height(500.0)
                .show(ctx, |ui| {
                    ui.label("All symbols currently on the Nasdaq Reg SHO Threshold List (live from cache)");
                    ui.separator();

                    let alerts_map = &self.bg.regulatory_alerts_by_symbol;
                    if alerts_map.is_empty() {
                        ui.label("No Reg SHO symbols loaded yet.");
                        return;
                    }

                    // Build table data — this window is Reg SHO threshold only, so
                    // exclude symbols whose only alert is another kind (e.g. a
                    // trade halt), which shares the regulatory_alerts map.
                    let mut rows: Vec<_> = alerts_map
                        .iter()
                        .filter(|(_, alerts)| {
                            alerts.iter().any(|a| a.kind == "reg_sho_threshold")
                        })
                        .collect();
                    rows.sort_by_key(|(sym, _)| *sym);

                    ui.horizontal(|ui| {
                        if ui
                            .button("Refresh prices")
                            .on_hover_text(
                                "Re-fetch the daily bar for every row — least-fresh / no-data symbols first",
                            )
                            .clicked()
                        {
                            reg_sho_refresh = true;
                        }
                        ui.label(
                            egui::RichText::new(format!("{} symbols", rows.len())).weak(),
                        );
                    });

                    let table = egui_extras::TableBuilder::new(ui)
                        .striped(true)
                        .resizable(true)
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .column(egui_extras::Column::auto().at_least(80.0))   // Symbol
                        .column(egui_extras::Column::auto().at_least(70.0))   // Last
                        .column(egui_extras::Column::auto().at_least(70.0))   // Bid
                        .column(egui_extras::Column::auto().at_least(70.0))   // Ask
                        .column(egui_extras::Column::auto().at_least(80.0))   // Daily Close
                        .column(egui_extras::Column::auto().at_least(70.0))   // Chg%
                        .column(egui_extras::Column::auto().at_least(120.0))  // Actions
                        .column(egui_extras::Column::remainder().at_least(200.0)); // Details

                    // Cells show "—" when a value is absent (0.0) instead of a
                    // misleading 0.0000.
                    let fmt_px = |v: f64| -> String {
                        if v > 0.0 { format!("{:.4}", v) } else { "—".to_string() }
                    };

                    // Apply user-selected sort (if any)
                    if let Some((col, asc)) = self.reg_sho_sort {
                        rows.sort_by(|a, b| {
                            let (sym_a, alerts_a) = a;
                            let (sym_b, alerts_b) = b;
                            let wa = self.watchlist_rows.iter().find(|r| &r.symbol == *sym_a)
                                .or_else(|| self.regulatory_prices.get(sym_a.as_str()));
                            let wb = self.watchlist_rows.iter().find(|r| &r.symbol == *sym_b)
                                .or_else(|| self.regulatory_prices.get(sym_b.as_str()));
                            let cmp = match col {
                                0 => sym_a.cmp(sym_b),
                                1 => wa.map(|w| w.last).partial_cmp(&wb.map(|w| w.last)).unwrap_or(std::cmp::Ordering::Equal),
                                2 => wa.map(|w| w.live_bid).partial_cmp(&wb.map(|w| w.live_bid)).unwrap_or(std::cmp::Ordering::Equal),
                                3 => wa.map(|w| w.live_ask).partial_cmp(&wb.map(|w| w.live_ask)).unwrap_or(std::cmp::Ordering::Equal),
                                4 => wa.map(|w| w.prev_close).partial_cmp(&wb.map(|w| w.prev_close)).unwrap_or(std::cmp::Ordering::Equal),
                                5 => {
                                    let ca = wa.map(|w| if w.prev_close > 0.0 { (w.last - w.prev_close) / w.prev_close * 100.0 } else { 0.0 });
                                    let cb = wb.map(|w| if w.prev_close > 0.0 { (w.last - w.prev_close) / w.prev_close * 100.0 } else { 0.0 });
                                    ca.partial_cmp(&cb).unwrap_or(std::cmp::Ordering::Equal)
                                }
                                _ => sym_a.cmp(sym_b),
                            };
                            if asc { cmp } else { cmp.reverse() }
                        });
                    }

                    table.header(20.0, |mut header| {
                        let mut sort_click = |ui: &mut egui::Ui, label: &str, col_idx: usize| {
                            let resp = ui.strong(label);
                            if resp.clicked() {
                                self.reg_sho_sort = match self.reg_sho_sort {
                                    Some((c, asc)) if c == col_idx => Some((c, !asc)),
                                    _ => Some((col_idx, true)),
                                };
                            }
                            resp
                        };
                        header.col(|ui| { sort_click(ui, "Symbol", 0); });
                        header.col(|ui| { sort_click(ui, "Last", 1); });
                        header.col(|ui| { sort_click(ui, "Bid", 2); });
                        header.col(|ui| { sort_click(ui, "Ask", 3); });
                        header.col(|ui| { sort_click(ui, "Dly Close", 4); });
                        header.col(|ui| { sort_click(ui, "Chg%", 5); });
                        header.col(|ui| { ui.strong("Actions"); });
                        header.col(|ui| { ui.strong("Details"); });
                    })
                    .body(|mut body| {
                        for (sym, alerts) in rows {
                            let alert = alerts
                                .iter()
                                .find(|a| a.kind == "reg_sho_threshold")
                                .unwrap_or(&alerts[0]);
                            // Live watchlist row first (has bid/ask); otherwise the
                            // cache-loaded snapshot so every symbol's columns fill.
                            let wl = self
                                .watchlist_rows
                                .iter()
                                .find(|r| &r.symbol == sym)
                                .or_else(|| self.regulatory_prices.get(sym.as_str()));

                            body.row(18.0, |mut row| {
                                row.col(|ui| {
                                    ui.label(egui::RichText::new(sym).monospace());
                                });
                                row.col(|ui| {
                                    ui.label(wl.map(|w| fmt_px(w.last)).unwrap_or_else(|| "—".into()));
                                });
                                row.col(|ui| {
                                    ui.label(wl.map(|w| fmt_px(w.live_bid)).unwrap_or_else(|| "—".into()));
                                });
                                row.col(|ui| {
                                    ui.label(wl.map(|w| fmt_px(w.live_ask)).unwrap_or_else(|| "—".into()));
                                });
                                row.col(|ui| {
                                    ui.label(wl.map(|w| fmt_px(w.regular_close)).unwrap_or_else(|| "—".into()));
                                });
                                row.col(|ui| {
                                    match wl {
                                        Some(w) if w.last > 0.0 => {
                                            let c = if w.change_pct >= 0.0 { egui::Color32::from_rgb(0,200,0) } else { egui::Color32::from_rgb(200,0,0) };
                                            ui.colored_label(c, format!("{:.2}%", w.change_pct));
                                        }
                                        _ => { ui.label("—"); }
                                    }
                                });
                                row.col(|ui| {
                                    ui.spacing_mut().item_spacing.x = 3.0;
                                    let already_watched = self
                                        .user_watchlist
                                        .iter()
                                        .any(|s| s.eq_ignore_ascii_case(sym));
                                    if already_watched {
                                        ui.add_enabled(false, egui::Button::new(egui::RichText::new("✓WL").small()))
                                            .on_hover_text("Already in watchlist");
                                    } else if ui.add(egui::Button::new(egui::RichText::new("+WL").small())).on_hover_text("Add to watchlist").clicked() {
                                        reg_sho_action = Some(SymbolAction::AddWatchlist(sym.clone()));
                                    }
                                    if ui.add(egui::Button::new(egui::RichText::new("D1").small())).on_hover_text("Open D1 chart").clicked() {
                                        reg_sho_action = Some(SymbolAction::OpenChartTf(sym.clone(), Timeframe::D1));
                                    }
                                    if ui.add(egui::Button::new(egui::RichText::new("W1").small())).on_hover_text("Open W1 chart").clicked() {
                                        reg_sho_action = Some(SymbolAction::OpenChartTf(sym.clone(), Timeframe::W1));
                                    }
                                });
                                row.col(|ui| {
                                    ui.label(&alert.details);
                                });
                            });
                        }
                    });
                });

            if let Some(action) = reg_sho_action {
                self.deferred_symbol_action = action;
            }
            if reg_sho_refresh {
                self.refresh_regulatory_prices();
            }
            if !open {
                self.show_reg_sho_window = false;
            }
        }

        // === Trading Halts / LULD floating window ===
        if self.show_halts_window {
            let mut open = true;
            let mut halts_action: Option<SymbolAction> = None;
            let mut halts_refresh = false;
            egui::Window::new("Trading Halts / LULD Pauses")
                .open(&mut open)
                .default_width(820.0)
                .default_height(460.0)
                .show(ctx, |ui| {
                    ui.label("Securities currently halted (live NasdaqTrader feed, cached)");
                    ui.separator();

                    let alerts_map = &self.bg.regulatory_alerts_by_symbol;
                    let mut rows: Vec<_> = alerts_map
                        .iter()
                        .filter(|(_, alerts)| alerts.iter().any(|a| a.kind == "trade_halt"))
                        .collect();
                    if rows.is_empty() {
                        ui.label("No active trading halts.");
                        return;
                    }
                    rows.sort_by_key(|(sym, _)| *sym);

                    ui.horizontal(|ui| {
                        if ui
                            .button("Refresh prices")
                            .on_hover_text(
                                "Re-fetch the daily bar for every row — least-fresh / no-data symbols first",
                            )
                            .clicked()
                        {
                            halts_refresh = true;
                        }
                        ui.label(
                            egui::RichText::new(format!("{} symbols", rows.len())).weak(),
                        );
                    });

                    let fmt_px = |v: f64| -> String {
                        if v > 0.0 { format!("{:.4}", v) } else { "—".to_string() }
                    };

                    let table = egui_extras::TableBuilder::new(ui)
                        .striped(true)
                        .resizable(true)
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .column(egui_extras::Column::auto().at_least(80.0))   // Symbol
                        .column(egui_extras::Column::auto().at_least(70.0))   // Last
                        .column(egui_extras::Column::auto().at_least(70.0))   // Chg%
                        .column(egui_extras::Column::auto().at_least(120.0))  // Actions
                        .column(egui_extras::Column::remainder().at_least(240.0)); // Halt info

                    // Apply user-selected sort (if any)
                    if let Some((col, asc)) = self.halts_sort {
                        rows.sort_by(|a, b| {
                            let (sym_a, alerts_a) = a;
                            let (sym_b, alerts_b) = b;
                            let wa = self.watchlist_rows.iter().find(|r| &r.symbol == *sym_a)
                                .or_else(|| self.regulatory_prices.get(sym_a.as_str()));
                            let wb = self.watchlist_rows.iter().find(|r| &r.symbol == *sym_b)
                                .or_else(|| self.regulatory_prices.get(sym_b.as_str()));
                            let cmp = match col {
                                0 => sym_a.cmp(sym_b),
                                1 => wa.map(|w| w.last).partial_cmp(&wb.map(|w| w.last)).unwrap_or(std::cmp::Ordering::Equal),
                                2 => {
                                    let ca = wa.map(|w| if w.prev_close > 0.0 { (w.last - w.prev_close) / w.prev_close * 100.0 } else { 0.0 });
                                    let cb = wb.map(|w| if w.prev_close > 0.0 { (w.last - w.prev_close) / w.prev_close * 100.0 } else { 0.0 });
                                    ca.partial_cmp(&cb).unwrap_or(std::cmp::Ordering::Equal)
                                }
                                _ => sym_a.cmp(sym_b),
                            };
                            if asc { cmp } else { cmp.reverse() }
                        });
                    }

                    table.header(20.0, |mut header| {
                        let mut sort_click = |ui: &mut egui::Ui, label: &str, col_idx: usize| {
                            let resp = ui.strong(label);
                            if resp.clicked() {
                                self.halts_sort = match self.halts_sort {
                                    Some((c, asc)) if c == col_idx => Some((c, !asc)),
                                    _ => Some((col_idx, true)),
                                };
                            }
                            resp
                        };
                        header.col(|ui| { sort_click(ui, "Symbol", 0); });
                        header.col(|ui| { sort_click(ui, "Last", 1); });
                        header.col(|ui| { sort_click(ui, "Chg%", 2); });
                        header.col(|ui| { ui.strong("Actions"); });
                        header.col(|ui| { ui.strong("Halt info"); });
                    })
                    .body(|mut body| {
                        for (sym, alerts) in rows {
                            let alert = alerts
                                .iter()
                                .find(|a| a.kind == "trade_halt")
                                .unwrap_or(&alerts[0]);
                            let wl = self
                                .watchlist_rows
                                .iter()
                                .find(|r| &r.symbol == sym)
                                .or_else(|| self.regulatory_prices.get(sym.as_str()));
                            body.row(18.0, |mut row| {
                                row.col(|ui| {
                                    ui.label(egui::RichText::new(sym).monospace().color(egui::Color32::from_rgb(255, 90, 90)));
                                });
                                row.col(|ui| {
                                    ui.label(wl.map(|w| fmt_px(w.last)).unwrap_or_else(|| "—".into()));
                                });
                                row.col(|ui| {
                                    match wl {
                                        Some(w) if w.last > 0.0 => {
                                            let c = if w.change_pct >= 0.0 { egui::Color32::from_rgb(0,200,0) } else { egui::Color32::from_rgb(200,0,0) };
                                            ui.colored_label(c, format!("{:.2}%", w.change_pct));
                                        }
                                        _ => { ui.label("—"); }
                                    }
                                });
                                row.col(|ui| {
                                    ui.spacing_mut().item_spacing.x = 3.0;
                                    let already_watched = self
                                        .user_watchlist
                                        .iter()
                                        .any(|s| s.eq_ignore_ascii_case(sym));
                                    if already_watched {
                                        ui.add_enabled(false, egui::Button::new(egui::RichText::new("✓WL").small()))
                                            .on_hover_text("Already in watchlist");
                                    } else if ui.add(egui::Button::new(egui::RichText::new("+WL").small())).on_hover_text("Add to watchlist").clicked() {
                                        halts_action = Some(SymbolAction::AddWatchlist(sym.clone()));
                                    }
                                    if ui.add(egui::Button::new(egui::RichText::new("D1").small())).on_hover_text("Open D1 chart").clicked() {
                                        halts_action = Some(SymbolAction::OpenChartTf(sym.clone(), Timeframe::D1));
                                    }
                                    if ui.add(egui::Button::new(egui::RichText::new("W1").small())).on_hover_text("Open W1 chart").clicked() {
                                        halts_action = Some(SymbolAction::OpenChartTf(sym.clone(), Timeframe::W1));
                                    }
                                });
                                row.col(|ui| {
                                    ui.label(&alert.details);
                                });
                            });
                        }
                    });
                });

            if let Some(action) = halts_action {
                self.deferred_symbol_action = action;
            }
            if halts_refresh {
                self.refresh_regulatory_prices();
            }
            if !open {
                self.show_halts_window = false;
            }
        }

        // Both regulatory windows closed → drop the one-shot fetch kick and the
        // read throttle so the next open re-fetches and re-reads fresh prices.
        if !self.show_reg_sho_window && !self.show_halts_window {
            self.regulatory_prices_loaded = false;
            self.regulatory_price_read_at = None;
        }

        // ── central panel (chart area) ────────────────────────────────────────
        // ── Drawing toolbar (horizontal top bar, TradingView style) ─────────
        egui::Panel::top("drawing_toolbar")
            .max_height(24.0)
            .frame(
                egui::Frame::NONE
                    .fill(egui::Color32::from_rgb(18, 18, 25))
                    .inner_margin(egui::Margin::symmetric(4, 1)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(2.0, 0.0);
                    let dm = self.draw_mode;
                    let active_col = egui::Color32::from_rgb(80, 200, 255);
                    let normal_col = egui::Color32::from_rgb(140, 140, 160);
                    let drawing_count = self
                        .charts
                        .get(self.active_tab)
                        .map(|c| c.drawings.len())
                        .unwrap_or(0);

                    // ── Lines group ──
                    ui.menu_button(
                        egui::RichText::new("Lines").small().color(normal_col),
                        |ui| {
                            if ui.button("─  Horizontal Line").clicked() {
                                self.draw_mode = DrawMode::PlacingHLine;
                                ui.close();
                            }
                            if ui.button("│  Vertical Line").clicked() {
                                self.draw_mode = DrawMode::PlacingVLine;
                                ui.close();
                            }
                            if ui.button("╲  Trendline").clicked() {
                                self.draw_mode = DrawMode::PlacingTrendP1;
                                ui.close();
                            }
                            if ui.button("╱  Ray").clicked() {
                                self.draw_mode = DrawMode::PlacingRayP1;
                                ui.close();
                            }
                            if ui.button("↔  Extended Line").clicked() {
                                self.draw_mode = DrawMode::PlacingExtLineP1;
                                ui.close();
                            }
                            if ui.button("→  Horizontal Ray").clicked() {
                                self.draw_mode = DrawMode::PlacingHRay;
                                ui.close();
                            }
                            if ui.button("+  Cross Line").clicked() {
                                self.draw_mode = DrawMode::PlacingCrossLine;
                                ui.close();
                            }
                            if ui.button("➤  Arrow Line").clicked() {
                                self.draw_mode = DrawMode::PlacingArrowP1;
                                ui.close();
                            }
                            if ui.button("ℹ  Info Line").clicked() {
                                self.draw_mode = DrawMode::PlacingInfoLineP1;
                                ui.close();
                            }
                            if ui.button("∠  Trend Angle").clicked() {
                                self.draw_mode = DrawMode::PlacingTrendAngleP1;
                                ui.close();
                            }
                            if ui.button("⫼  Parallel Channel").clicked() {
                                self.draw_mode = DrawMode::PlacingParallelChP1;
                                ui.close();
                            }
                            if ui.button("~  Polyline (dbl-click end)").clicked() {
                                self.draw_mode = DrawMode::PlacingPolyline;
                                self.polyline_points.clear();
                                ui.close();
                            }
                            if ui.button("⫽  Trend Channel (3 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingTrendChannelP1;
                                ui.close();
                            }
                        },
                    );
                    ui.separator();

                    // ── Gann & Fib group ──
                    ui.menu_button(
                        egui::RichText::new("Fib/Gann").small().color(normal_col),
                        |ui| {
                            if ui.button("Fib Retracement").clicked() {
                                self.draw_mode = DrawMode::PlacingFiboP1;
                                ui.close();
                            }
                            if ui.button("Fib Extension (3 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingFiboExtP1;
                                ui.close();
                            }
                            if ui.button("Fib Channel (3 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingFibChannelP1;
                                ui.close();
                            }
                            if ui.button("Fib Time Zones").clicked() {
                                self.draw_mode = DrawMode::PlacingFibTimeZones;
                                ui.close();
                            }
                            if ui.button("Andrews Pitchfork (3 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingPitchforkP1;
                                ui.close();
                            }
                            if ui.button("Schiff Pitchfork (3 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingSchiffPitchforkP1;
                                ui.close();
                            }
                            if ui.button("Mod Schiff Pitchfork (3 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingModSchiffPitchforkP1;
                                ui.close();
                            }
                            if ui.button("Gann Fan").clicked() {
                                self.draw_mode = DrawMode::PlacingGannFan;
                                ui.close();
                            }
                            if ui.button("Gann Box (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingGannBoxP1;
                                ui.close();
                            }
                            if ui.button("Cyclic Lines (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingCyclicLinesP1;
                                ui.close();
                            }
                            if ui.button("Sine Wave (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingSineWaveP1;
                                ui.close();
                            }
                            if ui.button("Fib Circle (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingFibCircleP1;
                                ui.close();
                            }
                            if ui.button("Fib Spiral (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingFibSpiralP1;
                                ui.close();
                            }
                            if ui.button("Speed Resistance Fan (3 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingSpeedFanP1;
                                ui.close();
                            }
                            if ui.button("Speed Resistance Arc (3 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingSpeedArcP1;
                                ui.close();
                            }
                            if ui.button("Time Cycle (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingTimeCycleP1;
                                ui.close();
                            }
                            if ui.button("Inside Pitchfork (3 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingInsidePitchforkP1;
                                ui.close();
                            }
                            if ui.button("Fib Wedge (3 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingFibWedgeP1;
                                ui.close();
                            }
                            if ui.button("Pitch Fan (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingPitchFanP1;
                                ui.close();
                            }
                            if ui.button("Trend Fib Time (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingTrendFibTimeP1;
                                ui.close();
                            }
                            if ui.button("Gann Square (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingGannSquareP1;
                                ui.close();
                            }
                            if ui.button("Gann Square Fixed (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingGannSquareFixedP1;
                                ui.close();
                            }
                        },
                    );
                    ui.separator();

                    // ── Elliott Wave group ──
                    ui.menu_button(
                        egui::RichText::new("Elliott").small().color(normal_col),
                        |ui| {
                            if ui.button("Elliott Wave 1-5 (5 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingElliottWave;
                                self.multi_click_points.clear();
                                ui.close();
                            }
                            if ui.button("ABC Correction (3 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingAbcCorrection;
                                self.multi_click_points.clear();
                                ui.close();
                            }
                            if ui.button("Elliott Double WXY (3 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingElliottDouble;
                                self.multi_click_points.clear();
                                ui.close();
                            }
                            if ui.button("Elliott Triangle ABCDE (5 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingElliottTriangle;
                                self.multi_click_points.clear();
                                ui.close();
                            }
                            if ui.button("Elliott Triple WXYXZ (5 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingElliottTripleCombo;
                                self.multi_click_points.clear();
                                ui.close();
                            }
                        },
                    );
                    ui.separator();

                    // ── Measurement group ──
                    ui.menu_button(
                        egui::RichText::new("Measure").small().color(normal_col),
                        |ui| {
                            if ui.button("Date Range (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingDateRangeP1;
                                ui.close();
                            }
                            if ui.button("Price Range (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingPriceRangeP1;
                                ui.close();
                            }
                            if ui.button("Date & Price Range (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingDatePriceRangeP1;
                                ui.close();
                            }
                            if ui.button("Ruler (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingRulerP1;
                                ui.close();
                            }
                            if ui.button("Measure Tool (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingMeasureToolP1;
                                ui.close();
                            }
                            if ui.button("Bars Pattern (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingBarsPatternP1;
                                ui.close();
                            }
                            if ui.button("Projection (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingProjectionP1;
                                ui.close();
                            }
                        },
                    );
                    ui.separator();

                    // ── Patterns group ──
                    ui.menu_button(
                        egui::RichText::new("Patterns").small().color(normal_col),
                        |ui| {
                            if ui.button("Head & Shoulders (5 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingHeadShoulders;
                                self.multi_click_points.clear();
                                ui.close();
                            }
                            if ui.button("XABCD Pattern (5 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingXabcdPattern;
                                self.multi_click_points.clear();
                                ui.close();
                            }
                            if ui.button("Triangle Pattern (3 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingTrianglePattern;
                                self.multi_click_points.clear();
                                ui.close();
                            }
                            if ui.button("Three Drives (3 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingThreeDrives;
                                self.multi_click_points.clear();
                                ui.close();
                            }
                            if ui.button("ABCD Pattern (4 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingAbcdPattern;
                                self.multi_click_points.clear();
                                ui.close();
                            }
                            if ui.button("Cypher Pattern (5 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingCypherPattern;
                                self.multi_click_points.clear();
                                ui.close();
                            }
                        },
                    );
                    ui.separator();

                    // ── Shapes group ──
                    ui.menu_button(
                        egui::RichText::new("Shapes").small().color(normal_col),
                        |ui| {
                            if ui.button("▭  Rectangle").clicked() {
                                self.draw_mode = DrawMode::PlacingRectP1;
                                ui.close();
                            }
                            if ui.button("═  Channel (3 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingChannelP1;
                                ui.close();
                            }
                            if ui.button("◯  Ellipse").clicked() {
                                self.draw_mode = DrawMode::PlacingEllipseP1;
                                ui.close();
                            }
                            if ui.button("△  Triangle (3 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingTriangleP1;
                                ui.close();
                            }
                            if ui.button("▮  Highlighter").clicked() {
                                self.draw_mode = DrawMode::PlacingHighlighterP1;
                                ui.close();
                            }
                            if ui.button("⊞  Regression Channel").clicked() {
                                self.draw_mode = DrawMode::PlacingRegressionChP1;
                                ui.close();
                            }
                            if ui.button("◇  Rotated Rectangle (3 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingRotatedRectP1;
                                ui.close();
                            }
                            if ui.button("⌒  Arc (3 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingArcP1;
                                ui.close();
                            }
                            if ui.button("∿  Curve (4 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingCurveP1;
                                ui.close();
                            }
                            if ui.button("⤳  Path (multi-click)").clicked() {
                                self.draw_mode = DrawMode::PlacingPath;
                                self.polyline_points.clear();
                                ui.close();
                            }
                            if ui.button("◯  Circle (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingCircleP1;
                                ui.close();
                            }
                            if ui.button("∿  Double Curve (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingDoubleCurveP1;
                                ui.close();
                            }
                        },
                    );
                    ui.separator();

                    // ── Annotations ──
                    ui.menu_button(
                        egui::RichText::new("Annotate").small().color(normal_col),
                        |ui| {
                            if ui.button("T  Text Label").clicked() {
                                self.draw_mode = DrawMode::PlacingTextLabel;
                                ui.close();
                            }
                            if ui.button("▲  Arrow Up").clicked() {
                                self.draw_mode = DrawMode::PlacingArrowMarkerUp;
                                ui.close();
                            }
                            if ui.button("▼  Arrow Down").clicked() {
                                self.draw_mode = DrawMode::PlacingArrowMarkerDown;
                                ui.close();
                            }
                            if ui.button("+  Cross Marker").clicked() {
                                self.draw_mode = DrawMode::PlacingCrossMarker;
                                ui.close();
                            }
                            if ui.button("$  Price Label").clicked() {
                                self.draw_mode = DrawMode::PlacingPriceLabel;
                                ui.close();
                            }
                            if ui.button("⌐  Callout").clicked() {
                                self.draw_mode = DrawMode::PlacingCalloutP1;
                                ui.close();
                            }
                            if ui.button("☰  Anchor Note").clicked() {
                                self.draw_mode = DrawMode::PlacingAnchorNote;
                                ui.close();
                            }
                            if ui.button("✎  Brush/Freehand").clicked() {
                                self.draw_mode = DrawMode::PlacingBrush;
                                self.brush_points.clear();
                                ui.close();
                            }
                            if ui.button("\u{1F3AF}  Emoji").clicked() {
                                self.draw_mode = DrawMode::PlacingEmoji;
                                ui.close();
                            }
                            if ui.button("\u{1F6A9}  Flag").clicked() {
                                self.draw_mode = DrawMode::PlacingFlag;
                                ui.close();
                            }
                            if ui.button("\u{1F4AC}  Balloon (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingBalloonP1;
                                ui.close();
                            }
                            if ui.button("|  Session Break").clicked() {
                                self.draw_mode = DrawMode::PlacingSessionBreak;
                                ui.close();
                            }
                            if ui.button("\u{1F9F2}  Magnet Level").clicked() {
                                self.draw_mode = DrawMode::PlacingMagnetLevel;
                                ui.close();
                            }
                            if ui.button("\u{1F9ED}  Signpost").clicked() {
                                self.draw_mode = DrawMode::PlacingSignpost;
                                ui.close();
                            }
                            if ui.button("\u{1F4C8}  Forecast (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingForecastP1;
                                ui.close();
                            }
                            if ui.button("\u{1F47B}  Ghost Feed (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingGhostFeedP1;
                                ui.close();
                            }
                            if ui.button("\u{1F4CA}  Anchored VWAP").clicked() {
                                self.draw_mode = DrawMode::PlacingAnchoredVwap;
                                ui.close();
                            }
                            if ui.button("\u{1F4DD}  Price Note").clicked() {
                                self.draw_mode = DrawMode::PlacingPriceNote;
                                ui.close();
                            }
                            if ui.button("A  Anchored Text").clicked() {
                                self.draw_mode = DrawMode::PlacingAnchoredText;
                                ui.close();
                            }
                            if ui.button("#  Comment").clicked() {
                                self.draw_mode = DrawMode::PlacingComment;
                                ui.close();
                            }
                            if ui.button("<  Arrow Left").clicked() {
                                self.draw_mode = DrawMode::PlacingArrowMarkerLeft;
                                ui.close();
                            }
                            if ui.button(">  Arrow Right").clicked() {
                                self.draw_mode = DrawMode::PlacingArrowMarkerRight;
                                ui.close();
                            }
                        },
                    );
                    ui.separator();

                    // ── Trading ──
                    ui.menu_button(
                        egui::RichText::new("Trade").small().color(normal_col),
                        |ui| {
                            if ui.button("Long Position (3 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingLongPosP1;
                                ui.close();
                            }
                            if ui.button("Short Position (3 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingShortPosP1;
                                ui.close();
                            }
                            if ui.button("Price Range (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingPriceRangeP1;
                                ui.close();
                            }
                            if ui.button("Risk/Reward Box (3 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingRiskRewardP1;
                                ui.close();
                            }
                        },
                    );
                    ui.separator();

                    // ── Manage group ──
                    ui.menu_button(
                        egui::RichText::new("Manage").small().color(normal_col),
                        |ui| {
                            if ui.button("Object List...").clicked() {
                                self.show_object_list = true;
                                ui.close();
                            }
                            ui.separator();
                            if ui.button("Undo Last (Ctrl+Z)").clicked() {
                                if let Some(c) = self.charts.get_mut(self.active_tab) {
                                    if let Some(d) = c.drawings.pop() {
                                        c.drawings_undo.push(d);
                                    }
                                }
                                ui.close();
                            }
                            if ui.button("Redo (Ctrl+Shift+Z)").clicked() {
                                if let Some(c) = self.charts.get_mut(self.active_tab) {
                                    if let Some(d) = c.drawings_undo.pop() {
                                        c.drawings.push(d);
                                    }
                                }
                                ui.close();
                            }
                            ui.separator();
                            if ui.button("Clear All Drawings").clicked() {
                                if let Some(c) = self.charts.get_mut(self.active_tab) {
                                    c.drawings.clear();
                                    c.drawings_undo.clear();
                                }
                                ui.close();
                            }
                        },
                    );

                    // ── Quick trashcan button (always visible) ──
                    if drawing_count > 0 {
                        if ui
                            .small_button(
                                egui::RichText::new("\u{1F5D1}")
                                    .small()
                                    .color(egui::Color32::from_rgb(200, 80, 80)),
                            )
                            .on_hover_text("Delete last drawing (Ctrl+Z to undo)")
                            .clicked()
                        {
                            if let Some(c) = self.charts.get_mut(self.active_tab) {
                                if let Some(d) = c.drawings.pop() {
                                    c.drawing_styles.pop();
                                    c.drawings_undo.push(d);
                                }
                            }
                        }
                    }

                    ui.separator();

                    // ── Magnet (OHLC snap) toggle ──
                    let mag_color = if self.snap_enabled {
                        egui::Color32::from_rgb(26, 188, 156)
                    } else {
                        egui::Color32::from_rgb(100, 100, 110)
                    };
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("\u{1F9F2}").small().color(mag_color),
                            )
                            .min_size(egui::vec2(24.0, 20.0)),
                        )
                        .on_hover_text(if self.snap_enabled {
                            "Magnet ON (OHLC snap)"
                        } else {
                            "Magnet OFF"
                        })
                        .clicked()
                    {
                        self.snap_enabled = !self.snap_enabled;
                    }

                    // Cross-TF sync toggle
                    let xtf_color = if self.drawings_cross_tf {
                        egui::Color32::from_rgb(26, 188, 156)
                    } else {
                        egui::Color32::from_rgb(100, 100, 110)
                    };
                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new("TF").small().color(xtf_color))
                                .min_size(egui::vec2(24.0, 20.0)),
                        )
                        .on_hover_text(if self.drawings_cross_tf {
                            "Cross-TF drawings ON"
                        } else {
                            "Cross-TF drawings OFF"
                        })
                        .clicked()
                    {
                        self.drawings_cross_tf = !self.drawings_cross_tf;
                    }

                    // ── Line width selector ──
                    let widths = [1.0_f32, 1.5, 2.0, 3.0, 4.0];
                    for w in &widths {
                        let is_sel = (self.draw_width - w).abs() < 0.01;
                        let col = if is_sel {
                            egui::Color32::WHITE
                        } else {
                            egui::Color32::from_rgb(80, 80, 90)
                        };
                        let lbl = format!(
                            "{}px",
                            if *w == w.round() {
                                format!("{}", *w as u32)
                            } else {
                                format!("{:.1}", w)
                            }
                        );
                        if ui
                            .add(
                                egui::Button::new(egui::RichText::new(&lbl).small().color(col))
                                    .min_size(egui::vec2(28.0, 20.0)),
                            )
                            .clicked()
                        {
                            self.draw_width = *w;
                        }
                    }

                    // ── Line style selector ──
                    let styles = [
                        (LineStyle::Solid, "━"),
                        (LineStyle::Dashed, "╌"),
                        (LineStyle::Dotted, "┈"),
                    ];
                    for (s, lbl) in &styles {
                        let is_sel = self.draw_line_style == *s;
                        let col = if is_sel {
                            egui::Color32::WHITE
                        } else {
                            egui::Color32::from_rgb(80, 80, 90)
                        };
                        if ui
                            .add(
                                egui::Button::new(egui::RichText::new(*lbl).small().color(col))
                                    .min_size(egui::vec2(24.0, 20.0)),
                            )
                            .clicked()
                        {
                            self.draw_line_style = *s;
                        }
                    }

                    // ── Color picker (pre-placement) ──
                    ui.separator();
                    let colors = [
                        ("W", egui::Color32::WHITE),
                        ("Y", egui::Color32::from_rgb(255, 200, 50)),
                        ("G", egui::Color32::from_rgb(0, 200, 100)),
                        ("R", egui::Color32::from_rgb(220, 50, 50)),
                        ("C", egui::Color32::from_rgb(0, 188, 212)),
                        ("M", egui::Color32::from_rgb(200, 50, 200)),
                        ("O", egui::Color32::from_rgb(255, 140, 50)),
                        ("B", egui::Color32::from_rgb(80, 120, 255)),
                    ];
                    for (lbl, col) in &colors {
                        let is_sel = self.draw_color == *col;
                        let btn = egui::Button::new(
                            egui::RichText::new(*lbl).small().color(*col).strong(),
                        )
                        .min_size(egui::vec2(20.0, 20.0))
                        .fill(if is_sel {
                            egui::Color32::from_rgb(40, 40, 60)
                        } else {
                            egui::Color32::TRANSPARENT
                        });
                        if ui.add(btn).clicked() {
                            self.draw_color = *col;
                        }
                    }

                    // ── Follow latest toggle ──
                    ui.separator();
                    let follow_col = if self.follow_latest {
                        egui::Color32::from_rgb(0, 200, 200)
                    } else {
                        egui::Color32::from_rgb(80, 80, 90)
                    };
                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new("⟫").small().color(follow_col))
                                .min_size(egui::vec2(22.0, 20.0)),
                        )
                        .on_hover_text("Follow latest bar (auto-scroll)")
                        .clicked()
                    {
                        self.follow_latest = !self.follow_latest;
                    }

                    // ── Status ──
                    if dm != DrawMode::None {
                        ui.separator();
                        let mode_name = match dm {
                            DrawMode::PlacingHLine => "HLine: click price level",
                            DrawMode::PlacingVLine => "VLine: click bar position",
                            DrawMode::PlacingTrendP1 => "Trendline: click start",
                            DrawMode::PlacingTrendP2 { .. } => "Trendline: click end",
                            DrawMode::PlacingRayP1 => "Ray: click origin",
                            DrawMode::PlacingRayP2 { .. } => "Ray: click direction",
                            DrawMode::PlacingRectP1 => "Rect: click corner 1",
                            DrawMode::PlacingRectP2 { .. } => "Rect: click corner 2",
                            DrawMode::PlacingChannelP1 => "Channel: click point 1 of 3",
                            DrawMode::PlacingChannelP2 { .. } => "Channel: click point 2 of 3",
                            DrawMode::PlacingChannelP3 { .. } => {
                                "Channel: click point 3 of 3 (width)"
                            }
                            DrawMode::PlacingFiboP1 => "Fib: click start",
                            DrawMode::PlacingFiboP2 { .. } => "Fib: click end",
                            DrawMode::PlacingExtLineP1 => "Ext Line: click P1",
                            DrawMode::PlacingExtLineP2 { .. } => "Ext Line: click P2",
                            DrawMode::PlacingHRay => "HRay: click start point",
                            DrawMode::PlacingCrossLine => "CrossLine: click intersection",
                            DrawMode::PlacingArrowP1 => "Arrow: click start",
                            DrawMode::PlacingArrowP2 { .. } => "Arrow: click end",
                            DrawMode::PlacingInfoLineP1 => "Info: click start",
                            DrawMode::PlacingInfoLineP2 { .. } => "Info: click end",
                            DrawMode::PlacingPitchforkP1 => "Pitchfork: click point 1 of 3 (pivot)",
                            DrawMode::PlacingPitchforkP2 { .. } => "Pitchfork: click point 2 of 3",
                            DrawMode::PlacingPitchforkP3 { .. } => "Pitchfork: click point 3 of 3",
                            DrawMode::PlacingFiboExtP1 => "Fib Ext: click point 1 of 3",
                            DrawMode::PlacingFiboExtP2 { .. } => "Fib Ext: click point 2 of 3",
                            DrawMode::PlacingFiboExtP3 { .. } => "Fib Ext: click P3",
                            DrawMode::PlacingGannFan => "Gann: click origin",
                            DrawMode::PlacingLongPosP1 => "Long: click entry",
                            DrawMode::PlacingLongPosP2 { .. } => "Long: click stop",
                            DrawMode::PlacingLongPosP3 { .. } => "Long: click target",
                            DrawMode::PlacingShortPosP1 => "Short: click entry",
                            DrawMode::PlacingShortPosP2 { .. } => "Short: click stop",
                            DrawMode::PlacingShortPosP3 { .. } => "Short: click target",
                            DrawMode::PlacingPriceRangeP1 => "Range: click P1",
                            DrawMode::PlacingPriceRangeP2 { .. } => "Range: click P2",
                            DrawMode::PlacingTextLabel => "Text: click to place label",
                            DrawMode::PlacingArrowMarkerUp => "Arrow Up: click to place",
                            DrawMode::PlacingArrowMarkerDown => "Arrow Down: click to place",
                            DrawMode::PlacingEllipseP1 => "Ellipse: click corner 1",
                            DrawMode::PlacingEllipseP2 { .. } => "Ellipse: click corner 2",
                            DrawMode::PlacingTriangleP1 => "Triangle: click P1",
                            DrawMode::PlacingTriangleP2 { .. } => "Triangle: click P2",
                            DrawMode::PlacingTriangleP3 { .. } => "Triangle: click P3",
                            DrawMode::PlacingTrendAngleP1 => "Angle: click start",
                            DrawMode::PlacingTrendAngleP2 { .. } => "Angle: click end",
                            DrawMode::PlacingParallelChP1 => "Parallel Ch: click P1",
                            DrawMode::PlacingParallelChP2 { .. } => {
                                "Parallel Ch: click P2 (offset from midline)"
                            }
                            DrawMode::PlacingFibChannelP1 => "Fib Ch: click P1",
                            DrawMode::PlacingFibChannelP2 { .. } => "Fib Ch: click P2",
                            DrawMode::PlacingFibChannelP3 { .. } => "Fib Ch: click width",
                            DrawMode::PlacingFibTimeZones => "Fib Time: click start",
                            DrawMode::PlacingPriceLabel => "PriceLabel: click price level",
                            DrawMode::PlacingCalloutP1 => "Callout: click anchor",
                            DrawMode::PlacingCalloutP2 { .. } => "Callout: click label pos",
                            DrawMode::PlacingHighlighterP1 => "Highlighter: click corner 1",
                            DrawMode::PlacingHighlighterP2 { .. } => "Highlighter: click corner 2",
                            DrawMode::PlacingCrossMarker => "CrossMarker: click to place",
                            DrawMode::PlacingPolyline => "Polyline: click points, dbl-click end",
                            DrawMode::PlacingAnchorNote => "AnchorNote: click to place",
                            DrawMode::PlacingRegressionChP1 => "Regression: click start",
                            DrawMode::PlacingRegressionChP2 { .. } => "Regression: click end",
                            DrawMode::PlacingGannBoxP1 => "Gann Box: click corner 1",
                            DrawMode::PlacingGannBoxP2 { .. } => "Gann Box: click corner 2",
                            DrawMode::PlacingElliottWave => "Elliott: click swing points (5)",
                            DrawMode::PlacingAbcCorrection => "ABC: click swing points (3)",
                            DrawMode::PlacingDateRangeP1 => "Date Range: click start",
                            DrawMode::PlacingDateRangeP2 { .. } => "Date Range: click end",
                            DrawMode::PlacingDatePriceRangeP1 => "Date+Price: click start",
                            DrawMode::PlacingDatePriceRangeP2 { .. } => "Date+Price: click end",
                            DrawMode::PlacingHeadShoulders => "H&S: click points (5)",
                            DrawMode::PlacingXabcdPattern => "XABCD: click points (5)",
                            DrawMode::PlacingBrush => "Brush: click-drag to draw",
                            DrawMode::PlacingSchiffPitchforkP1 => "Schiff Fork: click pivot",
                            DrawMode::PlacingSchiffPitchforkP2 { .. } => "Schiff Fork: click P2",
                            DrawMode::PlacingSchiffPitchforkP3 { .. } => "Schiff Fork: click P3",
                            DrawMode::PlacingModSchiffPitchforkP1 => "Mod Schiff: click pivot",
                            DrawMode::PlacingModSchiffPitchforkP2 { .. } => "Mod Schiff: click P2",
                            DrawMode::PlacingModSchiffPitchforkP3 { .. } => "Mod Schiff: click P3",
                            DrawMode::PlacingCyclicLinesP1 => "Cyclic: click start",
                            DrawMode::PlacingCyclicLinesP2 { .. } => "Cyclic: click end (interval)",
                            DrawMode::PlacingSineWaveP1 => "Sine: click start",
                            DrawMode::PlacingSineWaveP2 { .. } => "Sine: click end (period/amp)",
                            DrawMode::PlacingEmoji => "Emoji: click to place",
                            DrawMode::PlacingFlag => "Flag: click to place",
                            DrawMode::PlacingBalloonP1 => "Balloon: click anchor",
                            DrawMode::PlacingBalloonP2 { .. } => "Balloon: click label pos",
                            DrawMode::PlacingSessionBreak => "Session Break: click",
                            DrawMode::PlacingMagnetLevel => "Magnet: click price level",
                            DrawMode::PlacingRiskRewardP1 => "R:R Box: click entry",
                            DrawMode::PlacingRiskRewardP2 { .. } => "R:R Box: click stop",
                            DrawMode::PlacingRiskRewardP3 { .. } => "R:R Box: click target",
                            DrawMode::PlacingFibCircleP1 => "Fib Circle: click center",
                            DrawMode::PlacingFibCircleP2 { .. } => "Fib Circle: click radius",
                            DrawMode::PlacingArcP1 => "Arc: click start",
                            DrawMode::PlacingArcP2 { .. } => "Arc: click midpoint",
                            DrawMode::PlacingArcP3 { .. } => "Arc: click end",
                            DrawMode::PlacingCurveP1 => "Curve: click start",
                            DrawMode::PlacingCurveP2 { .. } => "Curve: click ctrl1",
                            DrawMode::PlacingCurveP3 { .. } => "Curve: click ctrl2",
                            DrawMode::PlacingCurveP4 { .. } => "Curve: click end",
                            DrawMode::PlacingPath => "Path: click points, dbl-click end",
                            DrawMode::PlacingForecastP1 => "Forecast: click start",
                            DrawMode::PlacingForecastP2 { .. } => "Forecast: click end",
                            DrawMode::PlacingGhostFeedP1 => "Ghost Feed: click start",
                            DrawMode::PlacingGhostFeedP2 { .. } => "Ghost Feed: click end",
                            DrawMode::PlacingSignpost => "Signpost: click to place",
                            DrawMode::PlacingRulerP1 => "Ruler: click start",
                            DrawMode::PlacingRulerP2 { .. } => "Ruler: click end",
                            DrawMode::PlacingTimeCycleP1 => "Time Cycle: click start",
                            DrawMode::PlacingTimeCycleP2 { .. } => {
                                "Time Cycle: click end (interval)"
                            }
                            DrawMode::PlacingSpeedFanP1 => "Speed Fan: click low",
                            DrawMode::PlacingSpeedFanP2 { .. } => "Speed Fan: click high",
                            DrawMode::PlacingSpeedFanP3 { .. } => "Speed Fan: click time ref",
                            DrawMode::PlacingSpeedArcP1 => "Speed Arc: click low",
                            DrawMode::PlacingSpeedArcP2 { .. } => "Speed Arc: click high",
                            DrawMode::PlacingSpeedArcP3 { .. } => "Speed Arc: click time ref",
                            DrawMode::PlacingFibSpiralP1 => "Fib Spiral: click center",
                            DrawMode::PlacingFibSpiralP2 { .. } => "Fib Spiral: click radius",
                            DrawMode::PlacingRotatedRectP1 => "Rotated Rect: click P1",
                            DrawMode::PlacingRotatedRectP2 { .. } => "Rotated Rect: click P2",
                            DrawMode::PlacingRotatedRectP3 { .. } => "Rotated Rect: click height",
                            DrawMode::PlacingAnchoredVwap => "Anchored VWAP: click anchor bar",
                            DrawMode::PlacingTrendChannelP1 => "Trend Channel: click P1",
                            DrawMode::PlacingTrendChannelP2 { .. } => "Trend Channel: click P2",
                            DrawMode::PlacingTrendChannelP3 { .. } => "Trend Channel: click width",
                            DrawMode::PlacingInsidePitchforkP1 => "Inside Pitchfork: click pivot",
                            DrawMode::PlacingInsidePitchforkP2 { .. } => {
                                "Inside Pitchfork: click P2"
                            }
                            DrawMode::PlacingInsidePitchforkP3 { .. } => {
                                "Inside Pitchfork: click P3"
                            }
                            DrawMode::PlacingFibWedgeP1 => "Fib Wedge: click apex",
                            DrawMode::PlacingFibWedgeP2 { .. } => "Fib Wedge: click P2",
                            DrawMode::PlacingFibWedgeP3 { .. } => "Fib Wedge: click P3",
                            DrawMode::PlacingPriceNote => "Price Note: click price level",
                            DrawMode::PlacingMeasureToolP1 => "Measure: click start",
                            DrawMode::PlacingMeasureToolP2 { .. } => "Measure: click end",
                            DrawMode::PlacingAnchoredText => "Anchored Text: click",
                            DrawMode::PlacingComment => "Comment: click",
                            DrawMode::PlacingArrowMarkerLeft => "Arrow Left: click",
                            DrawMode::PlacingArrowMarkerRight => "Arrow Right: click",
                            DrawMode::PlacingCircleP1 => "Circle: click center",
                            DrawMode::PlacingCircleP2 { .. } => "Circle: click radius",
                            DrawMode::PlacingPitchFanP1 => "Pitch Fan: click start",
                            DrawMode::PlacingPitchFanP2 { .. } => "Pitch Fan: click end",
                            DrawMode::PlacingTrendFibTimeP1 => "Trend Fib Time: click start",
                            DrawMode::PlacingTrendFibTimeP2 { .. } => "Trend Fib Time: click end",
                            DrawMode::PlacingGannSquareP1 => "Gann Square: click corner 1",
                            DrawMode::PlacingGannSquareP2 { .. } => "Gann Square: click corner 2",
                            DrawMode::PlacingGannSquareFixedP1 => {
                                "Gann Square Fixed: click corner 1"
                            }
                            DrawMode::PlacingGannSquareFixedP2 { .. } => {
                                "Gann Square Fixed: click corner 2"
                            }
                            DrawMode::PlacingBarsPatternP1 => "Bars Pattern: click start",
                            DrawMode::PlacingBarsPatternP2 { .. } => "Bars Pattern: click end",
                            DrawMode::PlacingProjectionP1 => "Projection: click start",
                            DrawMode::PlacingProjectionP2 { .. } => "Projection: click end",
                            DrawMode::PlacingDoubleCurveP1 => "Double Curve: click start",
                            DrawMode::PlacingDoubleCurveP2 { .. } => "Double Curve: click end",
                            DrawMode::PlacingTrianglePattern => "Triangle Pattern: click (3)",
                            DrawMode::PlacingThreeDrives => "Three Drives: click (3)",
                            DrawMode::PlacingElliottDouble => "Elliott WXY: click (3)",
                            DrawMode::PlacingAbcdPattern => "ABCD: click (4)",
                            DrawMode::PlacingCypherPattern => "Cypher: click (5)",
                            DrawMode::PlacingElliottTriangle => "Elliott ABCDE: click (5)",
                            DrawMode::PlacingElliottTripleCombo => "Elliott WXYXZ: click (5)",
                            DrawMode::Eraser => "ERASER: click near drawing to delete",
                            DrawMode::None => "",
                        };
                        ui.label(egui::RichText::new(mode_name).small().color(active_col));
                        if ui.small_button("Esc").clicked() {
                            self.draw_mode = DrawMode::None;
                        }
                    }

                    // ── Drawing count ──
                    if drawing_count > 0 {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(format!("{} drawings", drawing_count))
                                    .small()
                                    .color(egui::Color32::from_rgb(80, 80, 100)),
                            );
                        });
                    }
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            let available = ui.available_rect_before_wrap();

            // ── Price axis rect (right 70px of chart — TradingView-style scale) ──
            let price_axis_w = 70.0_f32;
            let price_axis_rect = egui::Rect::from_min_max(
                egui::pos2(available.right() - price_axis_w, available.top()),
                available.max,
            );
            let chart_body_rect = egui::Rect::from_min_max(
                available.min,
                egui::pos2(available.right() - price_axis_w, available.bottom()),
            );

            let hover_pos = ctx.input(|i| i.pointer.hover_pos().unwrap_or_default());
            // Don't interact with chart when pointer is over a floating window or egui wants pointer
            let egui_hover = ctx.egui_wants_pointer_input() || ctx.egui_is_using_pointer() || ctx.dragged_id().is_some();
            let layer_at_hover = ctx.layer_id_at(hover_pos);
            let hover_over_window = egui_hover || layer_at_hover
                .map(|id| id.order == egui::Order::Middle || id.order == egui::Order::Foreground)
                .unwrap_or(false);
            let on_price_axis = price_axis_rect.contains(hover_pos) && !hover_over_window;
            let on_chart_body = chart_body_rect.contains(hover_pos) && !hover_over_window;

            // Scroll → zoom (only when not over a floating window, skip in MTF mode — cells handle own zoom)
            let scroll_delta = ctx.input(|i| i.smooth_scroll_delta.y);
            if scroll_delta != 0.0 && !hover_over_window && !self.mtf_enabled {
                if on_price_axis {
                    // Scroll on price axis → vertical zoom (TradingView style: squish/expand)
                    if let Some(chart) = self.charts.get_mut(self.active_tab) {
                        let pct = (scroll_delta * 0.002).clamp(-0.08, 0.08);
                        let factor = (1.0 + pct as f64).clamp(0.1, 20.0);
                        chart.zoom_chart_price_by(factor);
                    }
                } else if on_chart_body {
                    let ctrl_held = ctx.input(|i| i.modifiers.ctrl);
                    if ctrl_held {
                        // Ctrl+scroll on chart → vertical zoom (progressive)
                        if let Some(chart) = self.charts.get_mut(self.active_tab) {
                            let pct = (scroll_delta * 0.002).clamp(-0.08, 0.08);
                            let factor = (1.0 + pct as f64).clamp(0.1, 20.0);
                            chart.zoom_chart_price_by(factor);
                        }
                    } else {
                        // Scroll on chart → horizontal zoom (time axis, progressive)
                        for chart in &mut self.charts {
                            Self::handle_zoom(chart, scroll_delta);
                        }
                    }
                }
            }

            // Double-click while placing polyline → finalize it
            if ctx.input(|i| i.pointer.button_double_clicked(egui::PointerButton::Primary)) && self.draw_mode == DrawMode::PlacingPolyline {
                if self.polyline_points.len() >= 2 {
                    let pts = std::mem::take(&mut self.polyline_points);
                    if let Some(chart) = self.charts.get_mut(self.active_tab) {
                        chart.drawings.push(Drawing::Polyline { points: pts, color: self.draw_color });
                    }
                }
                self.polyline_points.clear();
                self.draw_mode = DrawMode::None;
            }

            // Double-click while placing path → finalize it
            if ctx.input(|i| i.pointer.button_double_clicked(egui::PointerButton::Primary)) && self.draw_mode == DrawMode::PlacingPath {
                if self.polyline_points.len() >= 2 {
                    let pts = std::mem::take(&mut self.polyline_points);
                    if let Some(chart) = self.charts.get_mut(self.active_tab) {
                        chart.drawings.push(Drawing::PathDraw { points: pts, color: self.draw_color });
                    }
                }
                self.polyline_points.clear();
                self.draw_mode = DrawMode::None;
            }

            // Brush: accumulate points while dragging, finalize on mouse release
            if self.draw_mode == DrawMode::PlacingBrush {
                let is_down = ctx.input(|i| i.pointer.primary_down());
                if is_down {
                    if let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) {
                        if on_chart_body {
                            if let Some(chart) = self.charts.get(self.active_tab) {
                                let (si, ei) = chart.visible_range();
                                let bar_w_f = if ei > si { chart_body_rect.width() / (ei - si) as f32 } else { 1.0 };
                                let rel_x = pos.x - chart_body_rect.left();
                                let bar_float = rel_x / bar_w_f;
                                let bar_local = bar_float as usize;
                                let abs_idx = si + bar_local;
                                let vis = &chart.bars[si..ei];
                                if !vis.is_empty() {
                                    let hi = vis.iter().map(|b| b.high).fold(f64::MIN, f64::max);
                                    let lo = vis.iter().map(|b| b.low).fold(f64::MAX, f64::min);
                                    let pad = (hi - lo) * 0.05;
                                    let top = hi + pad;
                                    let bot = lo - pad;
                                    let frac = ((pos.y - chart_body_rect.top()) / chart_body_rect.height()) as f64;
                                    let price = top - frac * (top - bot);
                                    self.brush_points.push((abs_idx, price));
                                }
                            }
                        }
                    }
                } else if !self.brush_points.is_empty() {
                    // Mouse released → finalize brush
                    let pts = std::mem::take(&mut self.brush_points);
                    if pts.len() >= 2 {
                        if let Some(chart) = self.charts.get_mut(self.active_tab) {
                            chart.drawings.push(Drawing::Brush { points: pts, color: self.draw_color });
                        }
                    }
                    self.draw_mode = DrawMode::None;
                }
            }

            // Double-click → reset zoom/pan
            if ctx.input(|i| i.pointer.button_double_clicked(egui::PointerButton::Primary)) && self.draw_mode == DrawMode::None {
                if on_price_axis {
                    // Double-click price axis → auto-fit vertical only
                    if let Some(chart) = self.charts.get_mut(self.active_tab) {
                        chart.price_zoom = 1.0;
                        chart.price_pan = 0.0;
                        chart.manual_view_override = false;
                        chart.reset_camera_from_legacy();
                    }
                } else if on_chart_body {
                    if self.mtf_enabled {
                        // Double-click in MTF grid → toggle single chart focus
                        self.mtf_enabled = false;
                        self.log.push_back(LogEntry::info(format!("Focused: {} [{}] — double-click to return to MTF grid",
                            self.charts.get(self.active_tab).map(|c| c.symbol.as_str()).unwrap_or("?"),
                            self.charts.get(self.active_tab).map(|c| c.timeframe.label()).unwrap_or("?"))));
                    } else if self.charts.len() > 1 {
                        // Double-click in single mode with multiple tabs → return to MTF grid
                        self.mtf_enabled = true;
                        // Load any charts with empty bars so all grid cells render
                        if let Some(ref cache) = self.cache.clone() {
                            let mut retry_first_chart = false;
                            for chart in &mut self.charts {
                                if chart.bars.is_empty() {
                                    {
                                        let mut gpu = self.gpu_indicators.take();
                                        if !chart.try_load(
                                            Arc::as_ref(cache),
                                            &mut self.log,
                                            gpu.as_mut(),
                                        ) {
                                            retry_first_chart = true;
                                        }
                                        self.gpu_indicators = gpu;
                                    }
                                }
                            }
                            if retry_first_chart {
                                self.queue_chart_reload(0);
                            }
                        }
                        self.log.push_back(LogEntry::info("MTF grid restored"));
                    } else {
                        // Single chart, no MTF → reset zoom/pan
                        if let Some(chart) = self.charts.get_mut(self.active_tab) {
                            chart.price_zoom = 1.0;
                            chart.price_pan = 0.0;
                            chart.visible_bars = 200;
                            chart.view_offset = chart.bars.len().saturating_sub(1) + CHART_RIGHT_MARGIN;
                            chart.manual_view_override = false;
                        }
                    }
                }
            }

            // Drag interactions — only when pointer is NOT over a floating window
            let pointer    = ctx.input(|i| i.pointer.clone());
            let drag_delta = ctx.input(|i| i.pointer.delta());
            // Block chart interaction when ANY egui widget/window is using the pointer
            let egui_wants_pointer = ctx.egui_wants_pointer_input() || ctx.egui_is_using_pointer();
            let anything_dragged = ctx.dragged_id().is_some();
            let layer_id_at_pointer = ctx.layer_id_at(pointer.hover_pos().unwrap_or_default());
            let pointer_over_window = egui_wants_pointer || anything_dragged || layer_id_at_pointer
                .map(|id| id.order == egui::Order::Middle || id.order == egui::Order::Foreground)
                .unwrap_or(false);

            // Skip drag in MTF mode — individual cells handle their own interaction
            if !self.mtf_enabled {
            let mut sync_trade_inputs = false;
            for chart in &mut self.charts {
                if pointer.primary_pressed() {
                    let press_pos = pointer.press_origin().unwrap_or_default();
                    // Price-axis scaling is handled by the dedicated widget below
                    // (`single_chart_price_axis`). Don't double-handle the press here —
                    // egui's hit-test on that widget already routes correctly even when
                    // a floating window overlaps the right scale strip. We only need to
                    // intercept the press so it doesn't fall through to the chart-pan
                    // branch and start dragging the chart instead.
                    if price_axis_rect.contains(press_pos) {
                        // No-op: widget owns the scale gesture.
                    } else if available.contains(press_pos) && !pointer_over_window {
                        // Check if press is near SL or TP line (draggable)
                        let mut sl_tp_drag = false;
                        if self.draw_mode == DrawMode::None {
                            let (si, ei) = chart.visible_range();
                            if ei > si && !chart.bars.is_empty() {
                                let vis = &chart.bars[si..ei];
                                let p_min = vis.iter().map(|b| b.low).fold(f64::MAX, f64::min);
                                let p_max = vis.iter().map(|b| b.high).fold(f64::MIN, f64::max);
                                let pad = (p_max - p_min) * 0.05;
                                let centre = (p_max + p_min + 2.0 * pad) * 0.5 + chart.price_pan;
                                let half = (p_max - p_min + 2.0 * pad) * 0.5 / chart.price_zoom;
                                let pm = centre - half;
                                let px = centre + half;
                                let price_to_y_drag = |p: f64| -> f32 {
                                    let frac = (px - p) / (px - pm);
                                    available.top() + frac as f32 * available.height()
                                };
                                if let Some(sl) = self.sl_price {
                                    let sl_y = price_to_y_drag(sl);
                                    if (press_pos.y - sl_y).abs() < 8.0 {
                                        self.dragging_sl = true;
                                        sl_tp_drag = true;
                                    }
                                }
                                if !sl_tp_drag {
                                    if let Some(tp) = self.tp_price {
                                        let tp_y = price_to_y_drag(tp);
                                        if (press_pos.y - tp_y).abs() < 8.0 {
                                            self.dragging_tp = true;
                                            sl_tp_drag = true;
                                        }
                                    }
                                }
                            }
                        }
                        if sl_tp_drag {
                            chart.is_dragging = false;
                            chart.is_drawing_drag = false;
                            chart.is_scaling_price = false;
                        } else if chart.selected_drawing.is_some() && self.draw_mode == DrawMode::None {
                            // Check if click is near a control point (for resize) vs whole-drawing drag
                            chart.dragging_cp = None; // reset
                            if let Some(sel) = chart.selected_drawing {
                                if let Some(drawing) = chart.drawings.get(sel) {
                                    let (si, ei) = chart.visible_range();
                                    let vis_count = (ei - si).max(1) as f32;
                                    let bw = available.width() / vis_count;
                                    // Collect control points
                                    let mut cps: Vec<(usize, f64)> = Vec::new();
                                    match drawing {
                                        Drawing::TrendLine { p1, p2, .. } | Drawing::ExtendedLine { p1, p2, .. }
                                        | Drawing::Rectangle { p1, p2, .. } | Drawing::Ellipse { p1, p2, .. }
                                        | Drawing::ArrowLine { p1, p2, .. } | Drawing::InfoLine { p1, p2, .. }
                                        | Drawing::Channel { p1, p2, .. } | Drawing::Ruler { p1, p2, .. } => {
                                            cps.push(*p1); cps.push(*p2);
                                        }
                                        Drawing::Pitchfork { pivot, p2, p3, .. } | Drawing::SchiffPitchfork { pivot, p2, p3, .. } => {
                                            cps.push(*pivot); cps.push(*p2); cps.push(*p3);
                                        }
                                        Drawing::FiboExtension { p1, p2, p3, .. } | Drawing::Triangle { p1, p2, p3, .. } => {
                                            cps.push(*p1); cps.push(*p2); cps.push(*p3);
                                        }
                                        _ => {}
                                    }
                                    // Check if click is within 10px of any control point
                                    for (cp_idx, (bi, pr)) in cps.iter().enumerate() {
                                        if *bi >= si && *bi < ei {
                                            let cpx = available.left() + ((*bi - si) as f32 + 0.5) * bw;
                                            let cpy = {
                                                let slice = &chart.bars[si..ei];
                                                let hi = slice.iter().map(|b| b.high).fold(0.0_f64, f64::max);
                                                let lo = slice.iter().map(|b| b.low).fold(f64::MAX, f64::min);
                                                let pad = (hi - lo) * 0.05;
                                                let centre = (hi + lo + 2.0 * pad) * 0.5 + chart.price_pan;
                                                let half = (hi - lo + 2.0 * pad) * 0.5 / chart.price_zoom;
                                                let px = centre + half;
                                                let pm = centre - half;
                                                let frac = (px - pr) / (px - pm);
                                                available.top() + frac as f32 * available.height()
                                            };
                                            let dist = ((press_pos.x - cpx).powi(2) + (press_pos.y - cpy).powi(2)).sqrt();
                                            if dist < 10.0 {
                                                chart.dragging_cp = Some(cp_idx);
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                            chart.is_drawing_drag = true;
                            chart.is_dragging = false;
                            chart.is_scaling_price = false;
                        } else {
                            // Normal chart pan is owned exclusively by the dedicated
                            // `single_chart_body_drag` widget registered after drawing.
                            // This legacy pre-render path used to start a second camera
                            // drag for every chart tab, then the widget path mutated the
                            // active chart again in the same gesture. That split-brain
                            // ownership made TradingView-style free-look feel random or
                            // completely dead under release builds.
                        }
                    }
                } else if pointer.primary_released() {
                    // Stop dragging when mouse released
                    chart.is_dragging = false;
                    chart.is_drawing_drag = false;
                    chart.is_scaling_price = false;
                    chart.dragging_cp = None;
                    chart.drag_start = None;
                    self.dragging_sl = false;
                    self.dragging_tp = false;
                } else if pointer_over_window && !chart.is_scaling_price && !chart.is_dragging && !chart.is_drawing_drag {
                    // Cancel pending drag state if pointer moves over a floating window
                    // but don't interrupt active drags/scaling
                    chart.drag_start = None;
                }

                // SL/TP line drag → update price from mouse Y position
                if (self.dragging_sl || self.dragging_tp) && drag_delta.y.abs() > 0.0 {
                    let (si, ei) = chart.visible_range();
                    if ei > si && !chart.bars.is_empty() {
                        let vis = &chart.bars[si..ei];
                        let p_min = vis.iter().map(|b| b.low).fold(f64::MAX, f64::min);
                        let p_max = vis.iter().map(|b| b.high).fold(f64::MIN, f64::max);
                        let pad = (p_max - p_min) * 0.05;
                        let range = p_max - p_min + 2.0 * pad;
                        let price_delta = -drag_delta.y as f64 * range / available.height() as f64 / chart.price_zoom;
                        if self.dragging_sl {
                            if let Some(ref mut sl) = self.sl_price { *sl += price_delta; }
                        }
                        if self.dragging_tp {
                            if let Some(ref mut tp) = self.tp_price { *tp += price_delta; }
                        }
                        sync_trade_inputs = true;
                    }
                }

                // Price axis drag → handled by the dedicated `single_chart_price_axis`
                // widget below. Don't re-apply zoom here or every drag delta double-counts.

                // Drawing drag — move selected drawing by delta
                if chart.is_drawing_drag && (drag_delta.x.abs() > 0.0 || drag_delta.y.abs() > 0.0) {
                    if let Some(sel) = chart.selected_drawing {
                        let (si, ei) = chart.visible_range();
                        let vis_count = (ei - si).max(1) as f32;
                        // bar_delta: positive = move right (later bars)
                        let bar_delta = (drag_delta.x / (available.width() / vis_count)) as i64;
                        // price_delta: drag down = lower price (y increases down, price increases up)
                        let price_delta = if !chart.bars.is_empty() {
                            let slice = &chart.bars[si..ei];
                            let hi = slice.iter().map(|b| b.high).fold(0.0_f64, f64::max);
                            let lo = slice.iter().map(|b| b.low).fold(f64::MAX, f64::min);
                            let range = hi - lo;
                            -drag_delta.y as f64 * range / available.height() as f64
                        } else { 0.0 };

                        // Helper to clamp bar index
                        let move_bar = |idx: usize| -> usize {
                            let new_idx = idx as i64 + bar_delta;
                            new_idx.clamp(0, chart.bars.len().saturating_sub(1) as i64) as usize
                        };

                        // Control point resize: move only the dragged point
                        if let Some(cp_idx) = chart.dragging_cp {
                            if let Some(d) = chart.drawings.get_mut(sel) {
                                let move_pt = |pt: &mut (usize, f64)| {
                                    pt.0 = (pt.0 as i64 + bar_delta).clamp(0, chart.bars.len().saturating_sub(1) as i64) as usize;
                                    pt.1 += price_delta;
                                };
                                match d {
                                    Drawing::TrendLine { p1, p2, .. } | Drawing::ExtendedLine { p1, p2, .. }
                                    | Drawing::Rectangle { p1, p2, .. } | Drawing::Ellipse { p1, p2, .. }
                                    | Drawing::ArrowLine { p1, p2, .. } | Drawing::InfoLine { p1, p2, .. }
                                    | Drawing::Channel { p1, p2, .. } | Drawing::Ruler { p1, p2, .. } => {
                                        if cp_idx == 0 { move_pt(p1); } else { move_pt(p2); }
                                    }
                                    Drawing::Pitchfork { pivot, p2, p3, .. } | Drawing::SchiffPitchfork { pivot, p2, p3, .. } => {
                                        match cp_idx { 0 => move_pt(pivot), 1 => move_pt(p2), _ => move_pt(p3) }
                                    }
                                    Drawing::FiboExtension { p1, p2, p3, .. } | Drawing::Triangle { p1, p2, p3, .. }
                                    | Drawing::FibChannel { p1, p2, p3, .. } => {
                                        match cp_idx { 0 => move_pt(p1), 1 => move_pt(p2), _ => move_pt(p3) }
                                    }
                                    // Vec-of-points drawings: index directly into the points vector
                                    Drawing::Polyline { points, .. }
                                    | Drawing::PathDraw { points, .. }
                                    | Drawing::Brush { points, .. }
                                    | Drawing::ElliottWave { points, .. }
                                    | Drawing::AbcCorrection { points, .. }
                                    | Drawing::HeadShoulders { points, .. }
                                    | Drawing::XabcdPattern { points, .. }
                                    | Drawing::TrianglePattern { points, .. }
                                    | Drawing::ThreeDrives { points, .. }
                                    | Drawing::ElliottDouble { points, .. }
                                    | Drawing::AbcdPattern { points, .. }
                                    | Drawing::CypherPattern { points, .. }
                                    | Drawing::ElliottTriangle { points, .. }
                                    | Drawing::ElliottTripleCombo { points, .. } => {
                                        if let Some(pt) = points.get_mut(cp_idx) {
                                            move_pt(pt);
                                        }
                                    }
                                    _ => {} // fallback: whole-drawing move
                                }
                            }
                        } else if let Some(d) = chart.drawings.get_mut(sel) {
                            match d {
                                // Single-price horizontal
                                Drawing::HLine { price, .. } | Drawing::MagnetLevel { price, .. }
                                | Drawing::PriceNote { price, .. } => { *price += price_delta; }
                                // Single-bar vertical
                                Drawing::VLine { bar_idx, .. } | Drawing::AnchoredVwapLine { bar_idx, .. }
                                | Drawing::SessionBreak { bar_idx, .. } | Drawing::FibTimeZones { bar_idx, .. } => {
                                    *bar_idx = move_bar(*bar_idx);
                                }
                                // Two-point (p1, p2)
                                Drawing::TrendLine { p1, p2, .. } | Drawing::TrendAngle { p1, p2, .. }
                                | Drawing::ExtendedLine { p1, p2, .. } | Drawing::Channel { p1, p2, .. }
                                | Drawing::InfoLine { p1, p2, .. } | Drawing::ArrowLine { p1, p2, .. }
                                | Drawing::Ruler { p1, p2, .. } | Drawing::MeasureTool { p1, p2, .. }
                                | Drawing::Forecast { p1, p2, .. } | Drawing::Rectangle { p1, p2, .. }
                                | Drawing::Highlighter { p1, p2, .. } | Drawing::Ellipse { p1, p2, .. }
                                | Drawing::SineWave { p1, p2, .. } | Drawing::RegressionChannel { p1, p2, .. }
                                | Drawing::GannBox { p1, p2, .. } | Drawing::GhostFeed { p1, p2, .. }
                                | Drawing::FibWedge { p1, p2, .. } | Drawing::DateRange { p1, p2, .. }
                                | Drawing::DatePriceRange { p1, p2, .. } | Drawing::PriceRange { p1, p2, .. }
                                | Drawing::ParallelChannel { p1, p2, .. }
                                | Drawing::Circle { p1, p2, .. } | Drawing::PitchFan { p1, p2, .. }
                                | Drawing::TrendFibTime { p1, p2, .. } | Drawing::GannSquare { p1, p2, .. }
                                | Drawing::GannSquareFixed { p1, p2, .. } | Drawing::BarsPattern { p1, p2, .. }
                                | Drawing::Projection { p1, p2, .. } | Drawing::DoubleCurve { p1, p2, .. } => {
                                    p1.0 = move_bar(p1.0); p1.1 += price_delta;
                                    p2.0 = move_bar(p2.0); p2.1 += price_delta;
                                }
                                // bar_idx + price
                                Drawing::HRay { bar_idx, price, .. } | Drawing::CrossLine { bar_idx, price, .. }
                                | Drawing::TextLabel { bar_idx, price, .. } | Drawing::PriceLabel { bar_idx, price, .. }
                                | Drawing::Signpost { bar_idx, price, .. } | Drawing::Flag { bar_idx, price, .. }
                                | Drawing::ArrowMarker { bar_idx, price, .. } | Drawing::CrossMarker { bar_idx, price, .. }
                                | Drawing::AnchorNote { bar_idx, price, .. } | Drawing::Emoji { bar_idx, price, .. }
                                | Drawing::AnchoredText { bar_idx, price, .. } | Drawing::Comment { bar_idx, price, .. }
                                | Drawing::ArrowMarkerLeft { bar_idx, price, .. } | Drawing::ArrowMarkerRight { bar_idx, price, .. } => {
                                    *bar_idx = move_bar(*bar_idx); *price += price_delta;
                                }
                                // origin + slope
                                Drawing::Ray { origin, .. } => {
                                    origin.0 = move_bar(origin.0); origin.1 += price_delta;
                                }
                                // pivot + p2 + p3 (pitchforks)
                                Drawing::Pitchfork { pivot, p2, p3, .. } | Drawing::SchiffPitchfork { pivot, p2, p3, .. }
                                | Drawing::ModSchiffPitchfork { pivot, p2, p3, .. } | Drawing::InsidePitchfork { pivot, p2, p3, .. } => {
                                    pivot.0 = move_bar(pivot.0); pivot.1 += price_delta;
                                    p2.0 = move_bar(p2.0); p2.1 += price_delta;
                                    p3.0 = move_bar(p3.0); p3.1 += price_delta;
                                }
                                // p1 + p2 + p3
                                Drawing::FiboExtension { p1, p2, p3, .. } | Drawing::FibChannel { p1, p2, p3, .. }
                                | Drawing::TrendChannel { p1, p2, p3, .. } | Drawing::ArcDraw { p1, p2, p3, .. }
                                | Drawing::RotatedRectangle { p1, p2, p3, .. } | Drawing::SpeedResistanceFan { p1, p2, p3, .. }
                                | Drawing::SpeedResistanceArc { p1, p2, p3, .. } | Drawing::Triangle { p1, p2, p3, .. } => {
                                    p1.0 = move_bar(p1.0); p1.1 += price_delta;
                                    p2.0 = move_bar(p2.0); p2.1 += price_delta;
                                    p3.0 = move_bar(p3.0); p3.1 += price_delta;
                                }
                                // CurveDraw: p1, ctrl1, ctrl2, p2
                                Drawing::CurveDraw { p1, ctrl1, ctrl2, p2, .. } => {
                                    p1.0 = move_bar(p1.0); p1.1 += price_delta;
                                    ctrl1.0 = move_bar(ctrl1.0); ctrl1.1 += price_delta;
                                    ctrl2.0 = move_bar(ctrl2.0); ctrl2.1 += price_delta;
                                    p2.0 = move_bar(p2.0); p2.1 += price_delta;
                                }
                                // Bezier path / multi-point
                                Drawing::Polyline { points, .. } | Drawing::ElliottWave { points, .. }
                                | Drawing::AbcCorrection { points, .. } | Drawing::HeadShoulders { points, .. }
                                | Drawing::XabcdPattern { points, .. } | Drawing::Brush { points, .. }
                                | Drawing::PathDraw { points, .. }
                                | Drawing::TrianglePattern { points, .. } | Drawing::ThreeDrives { points, .. }
                                | Drawing::ElliottDouble { points, .. } | Drawing::AbcdPattern { points, .. }
                                | Drawing::CypherPattern { points, .. } | Drawing::ElliottTriangle { points, .. }
                                | Drawing::ElliottTripleCombo { points, .. } => {
                                    for pt in points.iter_mut() {
                                        pt.0 = move_bar(pt.0); pt.1 += price_delta;
                                    }
                                }
                                // center + radius_pt
                                Drawing::FibCircle { center, radius_pt, .. } | Drawing::FibSpiral { center, radius_pt, .. } => {
                                    center.0 = move_bar(center.0); center.1 += price_delta;
                                    radius_pt.0 = move_bar(radius_pt.0); radius_pt.1 += price_delta;
                                }
                                // anchor + label_pos
                                Drawing::Callout { anchor, label_pos, .. } | Drawing::Balloon { anchor, label_pos, .. } => {
                                    anchor.0 = move_bar(anchor.0); anchor.1 += price_delta;
                                    label_pos.0 = move_bar(label_pos.0); label_pos.1 += price_delta;
                                }
                                // entry + stop/target (single bar point)
                                Drawing::LongPosition { entry, stop, target } | Drawing::ShortPosition { entry, stop, target }
                                | Drawing::RiskRewardBox { entry, stop, target } => {
                                    entry.0 = move_bar(entry.0); entry.1 += price_delta;
                                    *stop += price_delta; *target += price_delta;
                                }
                                // Fib retracement uses high/low/bar_start/bar_end
                                Drawing::FiboRetrace { high, low, bar_start, bar_end } => {
                                    *high += price_delta; *low += price_delta;
                                    *bar_start = move_bar(*bar_start); *bar_end = move_bar(*bar_end);
                                }
                                // GannFan: origin + scale (scale doesn't change on drag)
                                Drawing::GannFan { origin, .. } => {
                                    origin.0 = move_bar(origin.0); origin.1 += price_delta;
                                }
                                // CyclicLines / TimeCycle: bar_start + bar_end
                                Drawing::CyclicLines { bar_start, bar_end, .. } | Drawing::TimeCycle { bar_start, bar_end, .. } => {
                                    *bar_start = move_bar(*bar_start); *bar_end = move_bar(*bar_end);
                                }
                            }
                        }
                    }
                }

                // Normal chart body pan is handled by `single_chart_body_drag`
                // after drawing. Keep this legacy pre-render block limited to
                // SL/TP and drawing-object drags; applying camera pan here races
                // the widget-owned gesture and can move the active chart twice.
            }
            if sync_trade_inputs {
                self.sync_trade_line_inputs();
            }
            } // end !mtf_enabled drag guard

            // Console is rendered as egui::Window after CentralPanel (see below)

            // ── chart drawing ────────────────────────────────────────────────
            let crosshair = self.crosshair;
            let flags = self.indicator_flags();
            let show_rsi = self.show_rsi;
            let show_fisher = self.show_fisher;
            let show_macd = self.show_macd;
            let show_volume_pane = self.show_volume_pane;
            let show_stochastic = self.show_stochastic;
            let show_adx = self.show_adx;
            let show_cci = self.show_cci;
            let show_williams_r = self.show_williams_r;
            let show_obv = self.show_obv;
            let show_momentum = self.show_momentum;
            let show_cmo = self.show_cmo;
            let show_qstick = self.show_qstick;
            let show_disparity = self.show_disparity;
            let show_bop = self.show_bop;
            let show_stddev = self.show_stddev;
            let show_mfi = self.show_mfi;
            let show_trix = self.show_trix;
            let show_ppo = self.show_ppo;
            let show_ultosc = self.show_ultosc;
            let show_stochrsi = self.show_stochrsi;
            let show_var_oscillator = self.show_var_oscillator;
            let show_better_volume = self.show_better_volume;
            let show_ehlers_ebsw = self.show_ehlers_ebsw;
            let show_ehlers_cyber = self.show_ehlers_cyber;
            let show_ehlers_cg = self.show_ehlers_cg;
            let show_ehlers_roof = self.show_ehlers_roof;
            let render_cache = self.cache.clone();
            let sl_price = self.sl_price;
            let tp_price = self.tp_price;
            for chart in &mut self.charts {
                let symbol = regulatory_alerts::normalize_regulatory_symbol(&chart.symbol);
                chart.regulatory_alerts = self
                    .bg
                    .regulatory_alerts_by_symbol
                    .get(&symbol)
                    .cloned()
                    .unwrap_or_default();
            }
            let active_sub_pane_count = [
                show_rsi,
                show_fisher,
                show_macd,
                show_volume_pane,
                show_stochastic,
                show_adx,
                show_cci,
                show_williams_r,
                show_obv,
                show_momentum,
                show_cmo,
                show_qstick,
                show_disparity,
                show_bop,
                show_stddev,
                show_mfi,
                show_trix,
                show_ppo,
                show_ultosc,
                show_stochrsi,
                show_var_oscillator,
                show_better_volume,
                show_ehlers_ebsw,
                show_ehlers_cyber,
                show_ehlers_cg,
                show_ehlers_roof,
                self.show_squeeze,
            ]
            .into_iter()
            .filter(|enabled| *enabled)
            .count() as u8;

            if self.mtf_enabled {
                // Filter to visible, supported MTF charts and group them by symbol. Each
                // symbol gets its own multi-timeframe grid; the supported set is owned by
                // MTF_GRID_TIMEFRAMES, including M1/M5 where source data is available.
                while self.mtf_visible.len() < self.charts.len() {
                    self.mtf_visible.push(true);
                }
                let mtf_groups = mtf_visible_chart_groups(&self.charts, &self.mtf_visible);
                if mtf_groups.is_empty() {
                    ui.painter().text(
                        available.center(),
                        egui::Align2::CENTER_CENTER,
                        "No supported MTF Grid charts (M15+ only)",
                        egui::FontId::proportional(14.0),
                        AXIS_TEXT,
                    );
                    return;
                }
                let cols = self.mtf_cols.max(1);
                let header_h = 0.0_f32; // no per-group symbol header — each cell self-labels "SYM [TF]"
                let row_gap = 4.0_f32;
                let group_layout: Vec<(usize, usize)> = mtf_groups
                    .iter()
                    .map(|group| {
                        let group_cols = cols.min(group.indices.len().max(1));
                        let rows = (group.indices.len() + group_cols - 1) / group_cols;
                        (group_cols, rows.max(1))
                    })
                    .collect();
                let total_chart_rows: usize = group_layout.iter().map(|(_, rows)| *rows).sum();
                let reserved_h = header_h * mtf_groups.len() as f32
                    + row_gap * mtf_groups.len().saturating_sub(1) as f32;
                let chart_row_h = ((available.height() - reserved_h).max(80.0)
                    / total_chart_rows.max(1) as f32)
                    .max(80.0);

                // Detect click on grid cell to focus it
                let click_pos = if ctx.input(|i| i.pointer.primary_clicked()) {
                    ctx.input(|i| i.pointer.interact_pos())
                } else { None };

                // Lazy-load bars for visible MTF grid charts through the paced
                // deferred loader. Doing a synchronous `try_load()` directly from
                // this render loop produced multi-second UI stalls while restored
                // MTF grids pulled M1/M5/M15 merged rows and recomputed overlays.
                // `queue_chart_reload` is O(1)-deduped by `deferred_chart_load_set`.
                for group in &mtf_groups {
                    for &vi in &group.indices {
                        if self.charts[vi].bars.is_empty() {
                            self.queue_chart_reload(vi);
                        }
                    }
                }

                let mut group_top = available.top();
                for (group_idx, group) in mtf_groups.iter().enumerate() {
                    let (cols, rows) = group_layout[group_idx];
                    group_top += header_h;
                    let cell_w = available.width() / cols as f32;
                    let cell_h = chart_row_h;

                    for (grid_pos, &vi) in group.indices.iter().enumerate() {
                        // Rebuild trade overlay every 120 frames (~30s) or on first load.
                        // During heavy sync, keep the cached overlay: rebuilding every
                        // restored MTF cell adds avoidable work to already overloaded frames.
                        let fc = self.frame_count;
                        if !self.heavy_sync_in_progress
                            && (self.charts[vi].cached_trade_overlay_frame == 0
                                || fc.wrapping_sub(self.charts[vi].cached_trade_overlay_frame)
                                    > 120)
                        {
                            self.charts[vi].cached_trade_overlay =
                                self.build_trade_overlay(&self.charts[vi]);
                            self.charts[vi].cached_trade_overlay_frame = fc;
                        }
                        // Move the cached overlay out for the duration of this cell render — avoids
                        // a Vec<TradeMarker> clone (with String tickers) per cell per frame. We
                        // restore it once draw_chart returns, before the next cell iterates.
                        let trade_ov = std::mem::take(&mut self.charts[vi].cached_trade_overlay);
                        let chart = &mut self.charts[vi];
                        let idx = grid_pos;
                    let col = idx % cols;
                    let row = idx / cols;
                    let cell_rect = egui::Rect::from_min_size(
                        egui::pos2(
                            available.left() + col as f32 * cell_w,
                            group_top + row as f32 * cell_h,
                        ),
                        egui::vec2(cell_w - 2.0, cell_h - 2.0),
                    );

                    // Click to focus this cell (vi = actual chart index, not grid position)
                    if let Some(pos) = click_pos {
                        if cell_rect.contains(pos) {
                            self.mtf_focused = Some(vi);
                            self.active_tab = vi;
                        }
                    }

                    // Pointer in cell detection (for zoom/pan, NOT for focus change)
                    let ptr_in_cell = !pointer_over_floating && ctx.input(|i| i.pointer.hover_pos().map(|p| cell_rect.contains(p)).unwrap_or(false));
                    let is_focused = self.mtf_focused == Some(vi);

                    // Price-axis vertical scaling for this cell — same pattern as the
                    // single-chart path so MTF grid cells also respond to dragging the
                    // right scale strip.
                    let cell_price_axis_w = 70.0_f32;
                    let cell_price_axis_rect = egui::Rect::from_min_max(
                        egui::pos2(cell_rect.right() - cell_price_axis_w, cell_rect.top()),
                        cell_rect.max,
                    );
                    let cell_scale_resp = ui
                        .interact(
                            cell_price_axis_rect,
                            ui.id().with(("mtf_cell_price_axis", vi)),
                            egui::Sense::click_and_drag(),
                        )
                        .on_hover_cursor(egui::CursorIcon::ResizeVertical);
                    let scaling_this_cell = cell_scale_resp.is_pointer_button_down_on();
                    if scaling_this_cell {
                        let dy = ctx.input(|i| i.pointer.delta().y);
                        if dy.abs() > 0.0 {
                            let zoom_delta = -dy as f64 * 0.003;
                            let factor = (1.0 + zoom_delta).clamp(0.1, 20.0);
                            chart.zoom_chart_price_by(factor);
                        }
                    }
                    if cell_scale_resp.double_clicked() {
                        chart.price_zoom = 1.0;
                        chart.price_pan = 0.0;
                        chart.manual_view_override = false;
                        chart.reset_camera_from_legacy();
                    }

                    let cell_chart_body_rect = egui::Rect::from_min_max(
                        cell_rect.min,
                        egui::pos2(cell_rect.right() - cell_price_axis_w, cell_rect.bottom()),
                    );
                    let cell_body_resp = ui
                        .interact(
                            cell_chart_body_rect,
                            ui.id().with(("mtf_cell_chart_body", vi)),
                            egui::Sense::click_and_drag(),
                        )
                        .on_hover_cursor(egui::CursorIcon::Grab);
                    let cell_body_started = cell_body_resp.is_pointer_button_down_on()
                        && !scaling_this_cell
                        && self.draw_mode == DrawMode::None;
                    let cell_body_press = (cell_body_started
                        || (chart.is_dragging && ctx.input(|i| i.pointer.primary_down())))
                        && !scaling_this_cell
                        && self.draw_mode == DrawMode::None;
                    if cell_body_started && !chart.is_dragging {
                        chart.is_dragging = true;
                        chart.is_drawing_drag = false;
                        chart.is_scaling_price = false;
                        chart.drag_start = ctx.input(|i| {
                            i.pointer
                                .press_origin()
                                .or_else(|| i.pointer.interact_pos())
                                .or_else(|| i.pointer.hover_pos())
                        });
                        let price_pane_h = chart_price_pane_height(
                            cell_chart_body_rect.height(),
                            active_sub_pane_count,
                        );
                        chart.begin_chart_camera_pan(cell_chart_body_rect.width(), price_pane_h);
                    }
                    if cell_body_press && chart.is_dragging {
                        ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::Grabbing);
                        if let (Some(start), Some(pos)) =
                            (chart.drag_start, ctx.input(|i| i.pointer.interact_pos()))
                        {
                            let total_drag = pos - start;
                            if total_drag.x.abs() > 0.0 || total_drag.y.abs() > 0.0 {
                                let price_pane_h = chart_price_pane_height(
                                    cell_chart_body_rect.height(),
                                    active_sub_pane_count,
                                );
                                chart.pan_chart_camera_pixels(
                                    total_drag,
                                    cell_chart_body_rect.width(),
                                    price_pane_h,
                                );
                                    }
                        }
                    }
                    if !cell_body_press && chart.is_dragging {
                        chart.is_dragging = false;
                        chart.drag_start = None;
                    }

                    // Zoom when pointer is in this cell (no focus-click required) — but
                    // skip while the user is actively dragging the price scale so the
                    // scroll-zoom and body pan don't fight the vertical scaling.
                    if ptr_in_cell && !scaling_this_cell {
                        let scroll = ctx.input(|i| i.smooth_scroll_delta.y);
                        if scroll != 0.0 {
                            Self::handle_zoom(chart, scroll);
                        }
                    }

                    if ChartState::should_ensure_mql_mtf_overlays_for_render(
                        self.heavy_sync_in_progress,
                        self.mtf_enabled,
                        is_focused,
                    ) {
                        if let Some(cache) = render_cache.as_ref() {
                            chart.ensure_mql_mtf_overlays_for_render(
                                std::sync::Arc::as_ref(cache),
                                flags.sma200,
                                flags.kama,
                            );
                        }
                    }
                    let painter = ui.painter_at(cell_rect);
                    draw_chart(&painter, chart, cell_rect, crosshair, &flags, show_rsi, show_fisher, show_macd, show_volume_pane, show_stochastic, show_adx, show_cci, show_williams_r, show_obv, show_momentum, show_cmo, show_qstick, show_disparity, show_bop, show_stddev, show_mfi, show_trix, show_ppo, show_ultosc, show_stochrsi, show_var_oscillator, show_better_volume, show_ehlers_ebsw, show_ehlers_cyber, show_ehlers_cg, show_ehlers_roof, self.show_squeeze, sl_price, tp_price, &trade_ov, &self.alerts, &chart.regulatory_alerts, &self.draw_mode, chart_overlay_company_name(&self.bg.all_fundamentals, &self.kraken_equity_names, &chart.symbol).as_deref());
                    // Restore the cached overlay we moved out above.
                    self.charts[vi].cached_trade_overlay = trade_ov;

                    // Border: green for focused, dim for others (WebKit: .mtf-grid-cell:hover outline)
                    let border_color = if is_focused {
                        egui::Color32::from_rgb(76, 175, 80) // green — focused
                    } else {
                        egui::Color32::from_rgb(40, 40, 60) // dim
                    };
                    let border_width = if is_focused { 2.0 } else { 1.0 };
                    ui.painter_at(cell_rect).rect_stroke(
                        cell_rect,
                        0.0,
                        egui::Stroke::new(border_width, border_color),
                        egui::StrokeKind::Outside,
                    );
                    }
                    group_top += rows as f32 * cell_h + row_gap;
                }
            } else {
                // Allocate the visual chart area as hover-only, then create separate
                // interaction targets for the chart body and the price axis. A full-rect
                // click/drag response steals the pointer before the narrow price scale can
                // own it, which regressed TradingView-style scale dragging.
                let (rect, _chart_alloc_resp) = ui.allocate_exact_size(available.size(), egui::Sense::hover());
                let price_axis_w = 70.0_f32;
                let price_axis_rect = egui::Rect::from_min_max(
                    egui::pos2(rect.right() - price_axis_w, rect.top()),
                    rect.max,
                );
                let chart_body_interact_rect = egui::Rect::from_min_max(
                    rect.min,
                    egui::pos2(rect.right() - price_axis_w, rect.bottom()),
                );
                // Single click_and_drag widget for the price axis. Previous attempts
                // layered a Sense::drag widget and a Sense::click widget on the same
                // rect — but later-registered widgets win egui's hit-test, so the click
                // widget swallowed the press and the drag widget never saw the gesture.
                // The original reason for splitting was that `dragged()` defers until
                // egui decides the gesture is "decidedly dragging" (eats slow scale
                // flicks). We sidestep that by reading drag movement from the raw
                // pointer delta whenever `is_pointer_button_down_on()` is true, which
                // fires from the press frame onward without any movement threshold.
                // Egui's z-order still routes presses on overlapping floating windows
                // to the window, so the old `pointer_over_window` guard is no longer
                // needed for this widget.
                let price_axis_resp = ui
                    .interact(
                        price_axis_rect,
                        ui.id().with(("single_chart_price_axis", self.active_tab)),
                        egui::Sense::click_and_drag(),
                    )
                    .on_hover_cursor(egui::CursorIcon::ResizeVertical);
                let resp = ui.interact(
                    chart_body_interact_rect,
                    ui.id().with(("single_chart_body_drag", self.active_tab)),
                    egui::Sense::click_and_drag(),
                );
                if let Some(chart) = self.charts.get_mut(self.active_tab) {
                    let scale_press = price_axis_resp.is_pointer_button_down_on();
                    if scale_press && !chart.is_scaling_price {
                        chart.is_scaling_price = true;
                        chart.is_dragging = false;
                        chart.is_drawing_drag = false;
                        chart.scale_start_zoom = chart.price_zoom;
                        chart.scale_start_y = price_axis_resp
                            .interact_pointer_pos()
                            .map(|pos| pos.y)
                            .unwrap_or(chart.scale_start_y);
                    }
                    if scale_press {
                        let dy = ctx.input(|i| i.pointer.delta().y);
                        if dy.abs() > 0.0 {
                            let zoom_delta = -dy as f64 * 0.003;
                            let factor = (1.0 + zoom_delta).clamp(0.1, 20.0);
                            chart.zoom_chart_price_by(factor);
                            chart.is_dragging = false;
                            }
                    } else if chart.is_scaling_price {
                        chart.is_scaling_price = false;
                    }
                    if price_axis_resp.double_clicked() {
                        chart.price_zoom = 1.0;
                        chart.price_pan = 0.0;
                        chart.manual_view_override = false;
                        chart.reset_camera_from_legacy();
                    }

                    let body_started = resp.is_pointer_button_down_on()
                        && self.draw_mode == DrawMode::None
                        && !scale_press;
                    let body_press = (body_started
                        || (chart.is_dragging && ctx.input(|i| i.pointer.primary_down())))
                        && self.draw_mode == DrawMode::None
                        && !scale_press;
                    if resp.hovered() && self.draw_mode == DrawMode::None && !scale_press {
                        ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::Grab);
                    }
                    if body_started && !chart.is_dragging {
                        chart.is_dragging = true;
                        chart.is_drawing_drag = false;
                        chart.is_scaling_price = false;
                        chart.drag_start = ctx.input(|i| {
                            i.pointer
                                .press_origin()
                                .or_else(|| i.pointer.interact_pos())
                                .or_else(|| i.pointer.hover_pos())
                        });
                        let price_pane_h = chart_price_pane_height(
                            chart_body_interact_rect.height(),
                            active_sub_pane_count,
                        );
                        chart.begin_chart_camera_pan(chart_body_interact_rect.width(), price_pane_h);
                    }
                    if body_press && chart.is_dragging && !chart.is_scaling_price {
                        ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::Grabbing);
                        if let (Some(start), Some(pos)) = (chart.drag_start, ctx.input(|i| i.pointer.interact_pos())) {
                            let total_drag = pos - start;
                            if total_drag.x.abs() > 0.0 || total_drag.y.abs() > 0.0 {
                                let price_pane_h = chart_price_pane_height(
                                    chart_body_interact_rect.height(),
                                    active_sub_pane_count,
                                );
                                chart.pan_chart_camera_pixels(
                                    total_drag,
                                    chart_body_interact_rect.width(),
                                    price_pane_h,
                                );
                                    }
                        }
                    }
                    if !body_press && chart.is_dragging {
                        chart.is_dragging = false;
                        chart.drag_start = None;
                    }
                }

                // Rebuild trade overlay every 120 frames (~30s) or on first load
                let fc = self.frame_count;
                if let Some(c) = self.charts.get(self.active_tab) {
                    if !self.heavy_sync_in_progress && c.cached_trade_overlay_frame == 0 || fc.wrapping_sub(c.cached_trade_overlay_frame) > 120 {
                        let ov = self.build_trade_overlay(c);
                        self.charts[self.active_tab].cached_trade_overlay = ov;
                        self.charts[self.active_tab].cached_trade_overlay_frame = fc;
                    }
                }
                // Trade overlay is moved into the chart-mutating block below and
                // restored after draw_chart — same trick as the MTF grid above. Avoids
                // cloning Vec<TradeMarker> (with String tickers) every frame.

                // Replay mode: clamp view to only show replay_bar_idx bars
                if self.replay_active {
                    if let Some(chart) = self.charts.get_mut(self.active_tab) {
                        let count = self.replay_bar_idx.max(1).min(chart.bars.len());
                        chart.view_offset = count.saturating_sub(1);
                        chart.visible_bars = chart.visible_bars.min(count);
                    }
                }

                if let Some(chart) = self.charts.get_mut(self.active_tab) {
                    if let Some(cache) = render_cache.as_ref() {
                        chart.ensure_mql_mtf_overlays_for_render(
                            std::sync::Arc::as_ref(cache),
                            flags.sma200,
                            flags.kama,
                        );
                    }
                    let trade_ov = std::mem::take(&mut chart.cached_trade_overlay);
                    let painter = ui.painter_at(rect);
                    draw_chart(&painter, chart, rect, crosshair, &flags, show_rsi, show_fisher, show_macd, show_volume_pane, show_stochastic, show_adx, show_cci, show_williams_r, show_obv, show_momentum, show_cmo, show_qstick, show_disparity, show_bop, show_stddev, show_mfi, show_trix, show_ppo, show_ultosc, show_stochrsi, show_var_oscillator, show_better_volume, show_ehlers_ebsw, show_ehlers_cyber, show_ehlers_cg, show_ehlers_roof, self.show_squeeze, sl_price, tp_price, &trade_ov, &self.alerts, &chart.regulatory_alerts, &self.draw_mode, chart_overlay_company_name(&self.bg.all_fundamentals, &self.kraken_equity_names, &chart.symbol).as_deref());
                    chart.cached_trade_overlay = trade_ov;

                    // Replay overlay: show bar count and speed
                    if self.replay_active {
                        let replay_text = format!(
                            "REPLAY {}/{} | {} | {:.1} bars/s",
                            self.replay_bar_idx,
                            chart.bars.len(),
                            if self.replay_playing { "▶ PLAY" } else { "⏸ PAUSED" },
                            self.replay_speed,
                        );
                        painter.text(
                            egui::pos2(rect.left() + 8.0, rect.top() + 8.0),
                            egui::Align2::LEFT_TOP,
                            &replay_text,
                            egui::FontId::monospace(12.0),
                            egui::Color32::from_rgb(255, 200, 50),
                        );
                    }

                    // ── drawing selection via click (DrawMode::None) or eraser delete ─────
                    if resp.clicked() && (self.draw_mode == DrawMode::None || self.draw_mode == DrawMode::Eraser) {
                        if let Some(click_pos) = ctx.input(|i| i.pointer.interact_pos()) {
                            let price_axis_w = 70.0_f32;
                            let chart_area = egui::Rect::from_min_max(rect.min, egui::pos2(rect.right() - price_axis_w, rect.bottom()));
                            if chart_area.contains(click_pos) {
                                let (start_idx, end_idx) = chart.visible_range();
                                let bar_w = chart_area.width() / (end_idx - start_idx).max(1) as f32;
                                let mut vis_bars_range = None;
                                if end_idx > start_idx && !chart.bars.is_empty() {
                                    let vis = &chart.bars[start_idx..end_idx];
                                    let price_min = vis.iter().map(|b| b.low).fold(f64::MAX, f64::min);
                                    let price_max = vis.iter().map(|b| b.high).fold(f64::MIN, f64::max);
                                    let padding = (price_max - price_min) * 0.05;
                                    let pmin = price_min - padding;
                                    let pmax = price_max + padding;
                                    let centre = (pmax + pmin) * 0.5 + chart.price_pan;
                                    let half = (pmax - pmin) * 0.5 / chart.price_zoom;
                                    vis_bars_range = Some((centre - half, centre + half));
                                }
                                if let Some((pmin, pmax)) = vis_bars_range {
                                    let price_to_y = |p: f64| -> f32 {
                                        let frac = (pmax - p) / (pmax - pmin);
                                        chart_area.top() + frac as f32 * chart_area.height()
                                    };
                                    let bar_to_x = |idx: usize| -> f32 {
                                        chart_area.left() + ((idx - start_idx) as f32 + 0.5) * bar_w
                                    };
                                    const HIT_THRESHOLD: f32 = 8.0;
                                    // Point-to-line-segment distance
                                    let pt_line_dist = |p: egui::Pos2, a: egui::Pos2, b: egui::Pos2| -> f32 {
                                        let ab = egui::vec2(b.x - a.x, b.y - a.y);
                                        let ap = egui::vec2(p.x - a.x, p.y - a.y);
                                        let ab_len_sq = ab.x * ab.x + ab.y * ab.y;
                                        if ab_len_sq < 0.001 {
                                            return (ap.x * ap.x + ap.y * ap.y).sqrt();
                                        }
                                        let t = ((ap.x * ab.x + ap.y * ab.y) / ab_len_sq).clamp(0.0, 1.0);
                                        let proj = egui::pos2(a.x + t * ab.x, a.y + t * ab.y);
                                        ((p.x - proj.x).powi(2) + (p.y - proj.y).powi(2)).sqrt()
                                    };
                                    let mut best_idx: Option<usize> = None;
                                    let mut best_dist = HIT_THRESHOLD;
                                    for (i, drawing) in chart.drawings.iter().enumerate() {
                                        let dist = match drawing {
                                            Drawing::HLine { price, .. } => {
                                                let y = price_to_y(*price);
                                                (click_pos.y - y).abs()
                                            }
                                            Drawing::VLine { bar_idx, .. } if *bar_idx >= start_idx && *bar_idx < end_idx => {
                                                let x = bar_to_x(*bar_idx);
                                                (click_pos.x - x).abs()
                                            }
                                            Drawing::TrendLine { p1, p2, .. } if p1.0 >= start_idx && p1.0 < end_idx && p2.0 >= start_idx && p2.0 < end_idx => {
                                                let a = egui::pos2(bar_to_x(p1.0), price_to_y(p1.1));
                                                let b = egui::pos2(bar_to_x(p2.0), price_to_y(p2.1));
                                                pt_line_dist(click_pos, a, b)
                                            }
                                            Drawing::HRay { bar_idx, price, .. } => {
                                                let y = price_to_y(*price);
                                                let x_start = if *bar_idx >= start_idx && *bar_idx < end_idx { bar_to_x(*bar_idx) } else { chart_area.left() };
                                                pt_line_dist(click_pos, egui::pos2(x_start, y), egui::pos2(chart_area.right(), y))
                                            }
                                            Drawing::Ray { origin, slope, .. } if origin.0 >= start_idx && origin.0 < end_idx => {
                                                let x1 = bar_to_x(origin.0);
                                                let y1 = price_to_y(origin.1);
                                                let bars_to_edge = ((chart_area.right() - x1) / bar_w) as f64;
                                                let y2 = price_to_y(origin.1 + slope * bars_to_edge);
                                                pt_line_dist(click_pos, egui::pos2(x1, y1), egui::pos2(chart_area.right(), y2))
                                            }
                                            Drawing::Rectangle { p1, p2, .. } | Drawing::Highlighter { p1, p2, .. } if p1.0 >= start_idx && p1.0 < end_idx && p2.0 >= start_idx && p2.0 < end_idx => {
                                                let r = egui::Rect::from_two_pos(egui::pos2(bar_to_x(p1.0), price_to_y(p1.1)), egui::pos2(bar_to_x(p2.0), price_to_y(p2.1)));
                                                // Inside rect = select too (not just border)
                                                if r.contains(click_pos) { 0.0 }
                                                else {
                                                    let dx = (click_pos.x - r.center().x).abs() - r.width() / 2.0;
                                                    let dy = (click_pos.y - r.center().y).abs() - r.height() / 2.0;
                                                    dx.max(0.0).hypot(dy.max(0.0))
                                                }
                                            }
                                            Drawing::TrendAngle { p1, p2, .. } | Drawing::ExtendedLine { p1, p2, .. } | Drawing::Channel { p1, p2, .. } if p1.0 >= start_idx && p1.0 < end_idx && p2.0 >= start_idx && p2.0 < end_idx => {
                                                let a = egui::pos2(bar_to_x(p1.0), price_to_y(p1.1));
                                                let b = egui::pos2(bar_to_x(p2.0), price_to_y(p2.1));
                                                pt_line_dist(click_pos, a, b)
                                            }
                                            Drawing::CrossLine { bar_idx, price, .. } if *bar_idx >= start_idx && *bar_idx < end_idx => {
                                                let x = bar_to_x(*bar_idx);
                                                let y = price_to_y(*price);
                                                let dh = (click_pos.y - y).abs();
                                                let dv = (click_pos.x - x).abs();
                                                dh.min(dv)
                                            }
                                            Drawing::InfoLine { p1, p2, .. } | Drawing::ArrowLine { p1, p2, .. } | Drawing::Ruler { p1, p2, .. } | Drawing::MeasureTool { p1, p2, .. } | Drawing::Forecast { p1, p2, .. } | Drawing::TrendChannel { p1, p2, .. } | Drawing::Circle { p1, p2, .. } | Drawing::PitchFan { p1, p2, .. } | Drawing::TrendFibTime { p1, p2, .. } | Drawing::GannSquare { p1, p2, .. } | Drawing::GannSquareFixed { p1, p2, .. } | Drawing::BarsPattern { p1, p2, .. } | Drawing::Projection { p1, p2, .. } | Drawing::DoubleCurve { p1, p2, .. } if p1.0 >= start_idx && p1.0 < end_idx && p2.0 >= start_idx && p2.0 < end_idx => {
                                                let a = egui::pos2(bar_to_x(p1.0), price_to_y(p1.1));
                                                let b = egui::pos2(bar_to_x(p2.0), price_to_y(p2.1));
                                                pt_line_dist(click_pos, a, b)
                                            }
                                            Drawing::Polyline { points, .. } | Drawing::ElliottWave { points, .. } | Drawing::AbcCorrection { points, .. } | Drawing::HeadShoulders { points, .. } | Drawing::XabcdPattern { points, .. } | Drawing::TrianglePattern { points, .. } | Drawing::ThreeDrives { points, .. } | Drawing::ElliottDouble { points, .. } | Drawing::AbcdPattern { points, .. } | Drawing::CypherPattern { points, .. } | Drawing::ElliottTriangle { points, .. } | Drawing::ElliottTripleCombo { points, .. } => {
                                                // Min distance to any segment
                                                let pts: Vec<egui::Pos2> = points.iter().filter(|(idx, _)| *idx >= start_idx && *idx < end_idx)
                                                    .map(|(idx, price)| egui::pos2(bar_to_x(*idx), price_to_y(*price))).collect();
                                                pts.windows(2).map(|w| pt_line_dist(click_pos, w[0], w[1])).fold(HIT_THRESHOLD + 1.0, f32::min)
                                            }
                                            Drawing::TextLabel { bar_idx, price, .. } | Drawing::ArrowMarker { bar_idx, price, .. } | Drawing::CrossMarker { bar_idx, price, .. } | Drawing::PriceLabel { bar_idx, price, .. } | Drawing::Signpost { bar_idx, price, .. } | Drawing::Flag { bar_idx, price, .. } | Drawing::AnchoredText { bar_idx, price, .. } | Drawing::Comment { bar_idx, price, .. } | Drawing::ArrowMarkerLeft { bar_idx, price, .. } | Drawing::ArrowMarkerRight { bar_idx, price, .. } if *bar_idx >= start_idx && *bar_idx < end_idx => {
                                                let x = bar_to_x(*bar_idx);
                                                let y = price_to_y(*price);
                                                ((click_pos.x - x).powi(2) + (click_pos.y - y).powi(2)).sqrt()
                                            }
                                            // Pitchfork family (3-point): min dist to any of the 3 lines
                                            Drawing::Pitchfork { pivot, p2, p3, .. } | Drawing::SchiffPitchfork { pivot, p2, p3, .. } | Drawing::ModSchiffPitchfork { pivot, p2, p3, .. } | Drawing::InsidePitchfork { pivot, p2, p3, .. } => {
                                                let pv = egui::pos2(bar_to_x(pivot.0), price_to_y(pivot.1));
                                                let a = egui::pos2(bar_to_x(p2.0), price_to_y(p2.1));
                                                let b = egui::pos2(bar_to_x(p3.0), price_to_y(p3.1));
                                                let mid = egui::pos2((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
                                                pt_line_dist(click_pos, pv, mid)
                                                    .min(pt_line_dist(click_pos, a, b))
                                                    .min(pt_line_dist(click_pos, pv, a))
                                                    .min(pt_line_dist(click_pos, pv, b))
                                            }
                                            // Ellipse (2-point bounding box): inside = 0, outside = distance to border
                                            Drawing::Ellipse { p1, p2, .. } if p1.0 >= start_idx && p2.0 >= start_idx => {
                                                let cx = (bar_to_x(p1.0) + bar_to_x(p2.0)) / 2.0;
                                                let cy = (price_to_y(p1.1) + price_to_y(p2.1)) / 2.0;
                                                let rx = (bar_to_x(p1.0) - bar_to_x(p2.0)).abs() / 2.0;
                                                let ry = (price_to_y(p1.1) - price_to_y(p2.1)).abs() / 2.0;
                                                if rx > 0.0 && ry > 0.0 {
                                                    let norm = ((click_pos.x - cx) / rx).powi(2) + ((click_pos.y - cy) / ry).powi(2);
                                                    if norm <= 1.0 { 0.0 } else { (norm.sqrt() - 1.0) * rx.min(ry) }
                                                } else { HIT_THRESHOLD + 1.0 }
                                            }
                                            // GannFan (origin point): distance to origin
                                            Drawing::GannFan { origin, .. } => {
                                                let x = bar_to_x(origin.0);
                                                let y = price_to_y(origin.1);
                                                ((click_pos.x - x).powi(2) + (click_pos.y - y).powi(2)).sqrt()
                                            }
                                            // FibWedge (3-point): min distance to segments
                                            Drawing::FibWedge { p1, p2, p3, .. } => {
                                                let a = egui::pos2(bar_to_x(p1.0), price_to_y(p1.1));
                                                let b = egui::pos2(bar_to_x(p2.0), price_to_y(p2.1));
                                                let c = egui::pos2(bar_to_x(p3.0), price_to_y(p3.1));
                                                pt_line_dist(click_pos, a, b).min(pt_line_dist(click_pos, b, c)).min(pt_line_dist(click_pos, a, c))
                                            }
                                            // FibCircle (center+radius): distance to circle border
                                            Drawing::FibCircle { center, radius_pt, .. } => {
                                                let cx = bar_to_x(center.0);
                                                let cy = price_to_y(center.1);
                                                let rx = bar_to_x(radius_pt.0);
                                                let ry = price_to_y(radius_pt.1);
                                                let r = ((cx - rx).powi(2) + (cy - ry).powi(2)).sqrt();
                                                let d = ((click_pos.x - cx).powi(2) + (click_pos.y - cy).powi(2)).sqrt();
                                                (d - r).abs()
                                            }
                                            // FibSpiral (center+radius): distance to center
                                            Drawing::FibSpiral { center, .. } => {
                                                let cx = bar_to_x(center.0);
                                                let cy = price_to_y(center.1);
                                                ((click_pos.x - cx).powi(2) + (click_pos.y - cy).powi(2)).sqrt()
                                            }
                                            _ => HIT_THRESHOLD + 1.0, // remaining niche types
                                        };
                                        if dist < best_dist {
                                            best_dist = dist;
                                            best_idx = Some(i);
                                        }
                                    }
                                    if self.draw_mode == DrawMode::Eraser {
                                        // Eraser mode: delete the nearest drawing on click
                                        if let Some(idx) = best_idx {
                                            let d = chart.drawings.remove(idx);
                                            if idx < chart.drawing_styles.len() { chart.drawing_styles.remove(idx); }
                                            chart.drawings_undo.push(d);
                                            chart.selected_drawing = None;
                                        }
                                    } else if best_idx != chart.selected_drawing {
                                        chart.selected_drawing = best_idx;
                                    } else if best_idx.is_none() {
                                        // Click on empty space → deselect
                                        chart.selected_drawing = None;
                                    }
                                }
                            }
                        }
                    }
                    // ESC → deselect drawing
                    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) && chart.selected_drawing.is_some() {
                        chart.selected_drawing = None;
                    }

                    // ── drawing mode click handling ──────────────────────
                    if resp.clicked() && self.draw_mode != DrawMode::None && self.draw_mode != DrawMode::Eraser {
                        if let Some(pos) = crosshair {
                            // Calculate bar index and price from click position
                            let price_axis_w = 70.0_f32;
                            let chart_rect = egui::Rect::from_min_max(
                                rect.min,
                                egui::pos2(rect.right() - price_axis_w, rect.bottom()),
                            );
                            let (start_idx, end_idx) = chart.visible_range();
                            let vis_bars = &chart.bars[start_idx..end_idx];
                            if !vis_bars.is_empty() && chart_rect.contains(pos) {
                                let bar_w = chart_rect.width() / vis_bars.len() as f32;
                                let rel_idx = ((pos.x - chart_rect.left()) / bar_w) as usize;
                                let abs_idx = start_idx + rel_idx.min(vis_bars.len().saturating_sub(1));

                                // Price from y position
                                let mut price_min = vis_bars.iter().map(|b| b.low).fold(f64::MAX, f64::min);
                                let mut price_max = vis_bars.iter().map(|b| b.high).fold(f64::MIN, f64::max);
                                let padding = (price_max - price_min) * 0.05;
                                price_min -= padding;
                                price_max += padding;
                                let range = price_max - price_min;
                                let centre = (price_max + price_min) * 0.5 + chart.price_pan;
                                let half = range * 0.5 / chart.price_zoom;
                                let pmin = centre - half;
                                let pmax = centre + half;
                                let frac = (pos.y - chart_rect.top()) / chart_rect.height();
                                let raw_price = pmax - frac as f64 * (pmax - pmin);

                                // OHLC Snap (magnet): snap to nearest candlestick OHLC price
                                // if within threshold. Toggle via snap_enabled.
                                let price = if self.snap_enabled && abs_idx < chart.bars.len() {
                                    let bar = &chart.bars[abs_idx];
                                    let ohlc = [bar.open, bar.high, bar.low, bar.close];
                                    let snap_threshold = (pmax - pmin) * 0.015;
                                    let mut best = raw_price;
                                    let mut best_dist = f64::MAX;
                                    for &level in &ohlc {
                                        let dist = (raw_price - level).abs();
                                        if dist < snap_threshold && dist < best_dist {
                                            best = level;
                                            best_dist = dist;
                                        }
                                    }
                                    best
                                } else {
                                    raw_price
                                };

                                let dc = self.draw_color; // pre-placement color
                                match self.draw_mode {
                                    DrawMode::Eraser | DrawMode::None => {} // handled above
                                    DrawMode::PlacingHLine => {
                                        chart.drawings.push(Drawing::HLine { price, color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingTrendP1 => {
                                        self.draw_mode = DrawMode::PlacingTrendP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingTrendP2 { bar1, price1 } => {
                                        chart.drawings.push(Drawing::TrendLine {
                                            p1: (bar1, price1),
                                            p2: (abs_idx, price),
                                            color: TRENDLINE_COL,
                                        });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingFiboP1 => {
                                        self.draw_mode = DrawMode::PlacingFiboP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingFiboP2 { bar1, price1 } => {
                                        let (high, low) = if price1 > price { (price1, price) } else { (price, price1) };
                                        chart.drawings.push(Drawing::FiboRetrace {
                                            high, low, bar_start: bar1, bar_end: abs_idx,
                                        });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingVLine => {
                                        chart.drawings.push(Drawing::VLine { bar_idx: abs_idx, color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingRectP1 => {
                                        self.draw_mode = DrawMode::PlacingRectP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingRectP2 { bar1, price1 } => {
                                        chart.drawings.push(Drawing::Rectangle {
                                            p1: (bar1, price1), p2: (abs_idx, price),
                                            color: egui::Color32::from_rgba_premultiplied(100, 150, 255, 40),
                                        });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingRayP1 => {
                                        self.draw_mode = DrawMode::PlacingRayP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingRayP2 { bar1, price1 } => {
                                        let slope = if abs_idx != bar1 { (price - price1) / (abs_idx as f64 - bar1 as f64) } else { 0.0 };
                                        chart.drawings.push(Drawing::Ray {
                                            origin: (bar1, price1), slope,
                                            color: egui::Color32::from_rgb(100, 200, 255),
                                        });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingChannelP1 => {
                                        self.draw_mode = DrawMode::PlacingChannelP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingChannelP2 { bar1, price1 } => {
                                        self.draw_mode = DrawMode::PlacingChannelP3 { bar1, price1, bar2: abs_idx, price2: price };
                                    }
                                    DrawMode::PlacingChannelP3 { bar1, price1, bar2, price2 } => {
                                        let width = price - price1; // offset from first line
                                        chart.drawings.push(Drawing::Channel {
                                            p1: (bar1, price1), p2: (bar2, price2), width,
                                            color: egui::Color32::from_rgb(150, 200, 100),
                                        });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    // ── New drawing tool handlers ──
                                    DrawMode::PlacingExtLineP1 => {
                                        self.draw_mode = DrawMode::PlacingExtLineP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingExtLineP2 { bar1, price1 } => {
                                        chart.drawings.push(Drawing::ExtendedLine { p1: (bar1, price1), p2: (abs_idx, price), color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingHRay => {
                                        chart.drawings.push(Drawing::HRay { bar_idx: abs_idx, price, color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingCrossLine => {
                                        chart.drawings.push(Drawing::CrossLine { bar_idx: abs_idx, price, color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingArrowP1 => {
                                        self.draw_mode = DrawMode::PlacingArrowP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingArrowP2 { bar1, price1 } => {
                                        chart.drawings.push(Drawing::ArrowLine { p1: (bar1, price1), p2: (abs_idx, price), color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingInfoLineP1 => {
                                        self.draw_mode = DrawMode::PlacingInfoLineP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingInfoLineP2 { bar1, price1 } => {
                                        chart.drawings.push(Drawing::InfoLine { p1: (bar1, price1), p2: (abs_idx, price), color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingPitchforkP1 => {
                                        self.draw_mode = DrawMode::PlacingPitchforkP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingPitchforkP2 { bar1, price1 } => {
                                        self.draw_mode = DrawMode::PlacingPitchforkP3 { bar1, price1, bar2: abs_idx, price2: price };
                                    }
                                    DrawMode::PlacingPitchforkP3 { bar1, price1, bar2, price2 } => {
                                        chart.drawings.push(Drawing::Pitchfork { pivot: (bar1, price1), p2: (bar2, price2), p3: (abs_idx, price), color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingFiboExtP1 => {
                                        self.draw_mode = DrawMode::PlacingFiboExtP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingFiboExtP2 { bar1, price1 } => {
                                        self.draw_mode = DrawMode::PlacingFiboExtP3 { bar1, price1, bar2: abs_idx, price2: price };
                                    }
                                    DrawMode::PlacingFiboExtP3 { bar1, price1, bar2, price2 } => {
                                        chart.drawings.push(Drawing::FiboExtension { p1: (bar1, price1), p2: (bar2, price2), p3: (abs_idx, price), color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingGannFan => {
                                        // Scale = visible price range / visible bars (1×1 angle baseline)
                                        let (si, ei) = chart.visible_range();
                                        let vis = &chart.bars[si..ei];
                                        let hi = vis.iter().map(|b| b.high).fold(f64::MIN, f64::max);
                                        let lo = vis.iter().map(|b| b.low).fold(f64::MAX, f64::min);
                                        let scale = if vis.len() > 1 { (hi - lo) / vis.len() as f64 } else { 1.0 };
                                        chart.drawings.push(Drawing::GannFan { origin: (abs_idx, price), scale, color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingLongPosP1 => {
                                        self.draw_mode = DrawMode::PlacingLongPosP2 { bar1: abs_idx, entry: price };
                                    }
                                    DrawMode::PlacingLongPosP2 { bar1, entry } => {
                                        self.draw_mode = DrawMode::PlacingLongPosP3 { bar1, entry, stop: price };
                                    }
                                    DrawMode::PlacingLongPosP3 { bar1, entry, stop } => {
                                        chart.drawings.push(Drawing::LongPosition { entry: (bar1, entry), stop, target: price });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingShortPosP1 => {
                                        self.draw_mode = DrawMode::PlacingShortPosP2 { bar1: abs_idx, entry: price };
                                    }
                                    DrawMode::PlacingShortPosP2 { bar1, entry } => {
                                        self.draw_mode = DrawMode::PlacingShortPosP3 { bar1, entry, stop: price };
                                    }
                                    DrawMode::PlacingShortPosP3 { bar1, entry, stop } => {
                                        chart.drawings.push(Drawing::ShortPosition { entry: (bar1, entry), stop, target: price });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingPriceRangeP1 => {
                                        self.draw_mode = DrawMode::PlacingPriceRangeP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingPriceRangeP2 { bar1, price1 } => {
                                        chart.drawings.push(Drawing::PriceRange { p1: (bar1, price1), p2: (abs_idx, price) });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingTextLabel => {
                                        chart.drawings.push(Drawing::TextLabel { bar_idx: abs_idx, price, text: "Label".to_string(), color: egui::Color32::WHITE });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingArrowMarkerUp => {
                                        chart.drawings.push(Drawing::ArrowMarker { bar_idx: abs_idx, price, is_up: true, color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingArrowMarkerDown => {
                                        chart.drawings.push(Drawing::ArrowMarker { bar_idx: abs_idx, price, is_up: false, color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingEllipseP1 => {
                                        self.draw_mode = DrawMode::PlacingEllipseP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingEllipseP2 { bar1, price1 } => {
                                        chart.drawings.push(Drawing::Ellipse { p1: (bar1, price1), p2: (abs_idx, price), color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingTriangleP1 => {
                                        self.draw_mode = DrawMode::PlacingTriangleP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingTriangleP2 { bar1, price1 } => {
                                        self.draw_mode = DrawMode::PlacingTriangleP3 { bar1, price1, bar2: abs_idx, price2: price };
                                    }
                                    DrawMode::PlacingTriangleP3 { bar1, price1, bar2, price2 } => {
                                        chart.drawings.push(Drawing::Triangle { p1: (bar1, price1), p2: (bar2, price2), p3: (abs_idx, price), color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingTrendAngleP1 => {
                                        self.draw_mode = DrawMode::PlacingTrendAngleP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingTrendAngleP2 { bar1, price1 } => {
                                        chart.drawings.push(Drawing::TrendAngle { p1: (bar1, price1), p2: (abs_idx, price), color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingParallelChP1 => {
                                        self.draw_mode = DrawMode::PlacingParallelChP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingParallelChP2 { bar1, price1 } => {
                                        // Offset = half the vertical distance between p1 and p2 (user clicks define center + one edge)
                                        let offset = (price - (price1 + (price - price1) * 0.5)).abs().max(0.0001);
                                        let mid_price2 = (price1 + price) / 2.0;
                                        chart.drawings.push(Drawing::ParallelChannel {
                                            p1: (bar1, price1), p2: (abs_idx, mid_price2),
                                            offset,
                                            color: egui::Color32::from_rgb(150, 200, 100),
                                        });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingFibChannelP1 => {
                                        self.draw_mode = DrawMode::PlacingFibChannelP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingFibChannelP2 { bar1, price1 } => {
                                        self.draw_mode = DrawMode::PlacingFibChannelP3 { bar1, price1, bar2: abs_idx, price2: price };
                                    }
                                    DrawMode::PlacingFibChannelP3 { bar1, price1, bar2, price2 } => {
                                        chart.drawings.push(Drawing::FibChannel { p1: (bar1, price1), p2: (bar2, price2), p3: (abs_idx, price), color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingFibTimeZones => {
                                        chart.drawings.push(Drawing::FibTimeZones { bar_idx: abs_idx, color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingPriceLabel => {
                                        chart.drawings.push(Drawing::PriceLabel { bar_idx: abs_idx, price, color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingCalloutP1 => {
                                        self.draw_mode = DrawMode::PlacingCalloutP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingCalloutP2 { bar1, price1 } => {
                                        chart.drawings.push(Drawing::Callout { anchor: (bar1, price1), label_pos: (abs_idx, price), text: "Note".to_string(), color: egui::Color32::WHITE });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingHighlighterP1 => {
                                        self.draw_mode = DrawMode::PlacingHighlighterP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingHighlighterP2 { bar1, price1 } => {
                                        chart.drawings.push(Drawing::Highlighter { p1: (bar1, price1), p2: (abs_idx, price), color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingCrossMarker => {
                                        chart.drawings.push(Drawing::CrossMarker { bar_idx: abs_idx, price, color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingPolyline => {
                                        self.polyline_points.push((abs_idx, price));
                                        // Don't change draw_mode — keep collecting points
                                    }
                                    DrawMode::PlacingAnchorNote => {
                                        chart.drawings.push(Drawing::AnchorNote { bar_idx: abs_idx, price, text: "Note".to_string(), color: egui::Color32::WHITE });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingRegressionChP1 => {
                                        self.draw_mode = DrawMode::PlacingRegressionChP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingRegressionChP2 { bar1, price1 } => {
                                        chart.drawings.push(Drawing::RegressionChannel { p1: (bar1, price1), p2: (abs_idx, price), color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingGannBoxP1 => {
                                        self.draw_mode = DrawMode::PlacingGannBoxP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingGannBoxP2 { bar1, price1 } => {
                                        chart.drawings.push(Drawing::GannBox { p1: (bar1, price1), p2: (abs_idx, price), color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingElliottWave => {
                                        self.multi_click_points.push((abs_idx, price));
                                        if self.multi_click_points.len() >= 5 {
                                            let pts = std::mem::take(&mut self.multi_click_points);
                                            chart.drawings.push(Drawing::ElliottWave { points: pts, color: dc });
                                            self.draw_mode = DrawMode::None;
                                        }
                                    }
                                    DrawMode::PlacingAbcCorrection => {
                                        self.multi_click_points.push((abs_idx, price));
                                        if self.multi_click_points.len() >= 3 {
                                            let pts = std::mem::take(&mut self.multi_click_points);
                                            chart.drawings.push(Drawing::AbcCorrection { points: pts, color: dc });
                                            self.draw_mode = DrawMode::None;
                                        }
                                    }
                                    DrawMode::PlacingDateRangeP1 => {
                                        self.draw_mode = DrawMode::PlacingDateRangeP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingDateRangeP2 { bar1, price1 } => {
                                        chart.drawings.push(Drawing::DateRange { p1: (bar1, price1), p2: (abs_idx, price) });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingDatePriceRangeP1 => {
                                        self.draw_mode = DrawMode::PlacingDatePriceRangeP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingDatePriceRangeP2 { bar1, price1 } => {
                                        chart.drawings.push(Drawing::DatePriceRange { p1: (bar1, price1), p2: (abs_idx, price) });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingHeadShoulders => {
                                        self.multi_click_points.push((abs_idx, price));
                                        if self.multi_click_points.len() >= 5 {
                                            let pts = std::mem::take(&mut self.multi_click_points);
                                            chart.drawings.push(Drawing::HeadShoulders { points: pts, color: dc });
                                            self.draw_mode = DrawMode::None;
                                        }
                                    }
                                    DrawMode::PlacingXabcdPattern => {
                                        self.multi_click_points.push((abs_idx, price));
                                        if self.multi_click_points.len() >= 5 {
                                            let pts = std::mem::take(&mut self.multi_click_points);
                                            chart.drawings.push(Drawing::XabcdPattern { points: pts, color: dc });
                                            self.draw_mode = DrawMode::None;
                                        }
                                    }
                                    DrawMode::PlacingBrush => {
                                        // Single click adds a point; drag handling below adds more
                                        self.brush_points.push((abs_idx, price));
                                    }
                                    DrawMode::PlacingSchiffPitchforkP1 => {
                                        self.draw_mode = DrawMode::PlacingSchiffPitchforkP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingSchiffPitchforkP2 { bar1, price1 } => {
                                        self.draw_mode = DrawMode::PlacingSchiffPitchforkP3 { bar1, price1, bar2: abs_idx, price2: price };
                                    }
                                    DrawMode::PlacingSchiffPitchforkP3 { bar1, price1, bar2, price2 } => {
                                        chart.drawings.push(Drawing::SchiffPitchfork { pivot: (bar1, price1), p2: (bar2, price2), p3: (abs_idx, price), color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingModSchiffPitchforkP1 => {
                                        self.draw_mode = DrawMode::PlacingModSchiffPitchforkP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingModSchiffPitchforkP2 { bar1, price1 } => {
                                        self.draw_mode = DrawMode::PlacingModSchiffPitchforkP3 { bar1, price1, bar2: abs_idx, price2: price };
                                    }
                                    DrawMode::PlacingModSchiffPitchforkP3 { bar1, price1, bar2, price2 } => {
                                        chart.drawings.push(Drawing::ModSchiffPitchfork { pivot: (bar1, price1), p2: (bar2, price2), p3: (abs_idx, price), color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingCyclicLinesP1 => {
                                        self.draw_mode = DrawMode::PlacingCyclicLinesP2 { bar1: abs_idx };
                                    }
                                    DrawMode::PlacingCyclicLinesP2 { bar1 } => {
                                        chart.drawings.push(Drawing::CyclicLines { bar_start: bar1, bar_end: abs_idx, color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingSineWaveP1 => {
                                        self.draw_mode = DrawMode::PlacingSineWaveP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingSineWaveP2 { bar1, price1 } => {
                                        chart.drawings.push(Drawing::SineWave { p1: (bar1, price1), p2: (abs_idx, price), color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingEmoji => {
                                        chart.drawings.push(Drawing::Emoji { bar_idx: abs_idx, price, emoji: "\u{1F3AF}".to_string() });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingFlag => {
                                        chart.drawings.push(Drawing::Flag { bar_idx: abs_idx, price, color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingBalloonP1 => {
                                        self.draw_mode = DrawMode::PlacingBalloonP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingBalloonP2 { bar1, price1 } => {
                                        chart.drawings.push(Drawing::Balloon { anchor: (bar1, price1), label_pos: (abs_idx, price), text: "Note".to_string(), color: egui::Color32::WHITE });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingSessionBreak => {
                                        chart.drawings.push(Drawing::SessionBreak { bar_idx: abs_idx, color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingMagnetLevel => {
                                        chart.drawings.push(Drawing::MagnetLevel { price, color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingRiskRewardP1 => {
                                        self.draw_mode = DrawMode::PlacingRiskRewardP2 { bar1: abs_idx, entry: price };
                                    }
                                    DrawMode::PlacingRiskRewardP2 { bar1, entry } => {
                                        self.draw_mode = DrawMode::PlacingRiskRewardP3 { bar1, entry, stop: price };
                                    }
                                    DrawMode::PlacingRiskRewardP3 { bar1, entry, stop } => {
                                        chart.drawings.push(Drawing::RiskRewardBox { entry: (bar1, entry), stop, target: price });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingFibCircleP1 => {
                                        self.draw_mode = DrawMode::PlacingFibCircleP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingFibCircleP2 { bar1, price1 } => {
                                        chart.drawings.push(Drawing::FibCircle { center: (bar1, price1), radius_pt: (abs_idx, price), color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingArcP1 => {
                                        self.draw_mode = DrawMode::PlacingArcP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingArcP2 { bar1, price1 } => {
                                        self.draw_mode = DrawMode::PlacingArcP3 { bar1, price1, bar2: abs_idx, price2: price };
                                    }
                                    DrawMode::PlacingArcP3 { bar1, price1, bar2, price2 } => {
                                        chart.drawings.push(Drawing::ArcDraw { p1: (bar1, price1), p2: (bar2, price2), p3: (abs_idx, price), color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingCurveP1 => {
                                        self.draw_mode = DrawMode::PlacingCurveP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingCurveP2 { bar1, price1 } => {
                                        self.draw_mode = DrawMode::PlacingCurveP3 { bar1, price1, bar2: abs_idx, price2: price };
                                    }
                                    DrawMode::PlacingCurveP3 { bar1, price1, bar2, price2 } => {
                                        self.draw_mode = DrawMode::PlacingCurveP4 { bar1, price1, bar2, price2, bar3: abs_idx, price3: price };
                                    }
                                    DrawMode::PlacingCurveP4 { bar1, price1, bar2, price2, bar3, price3 } => {
                                        chart.drawings.push(Drawing::CurveDraw { p1: (bar1, price1), ctrl1: (bar2, price2), ctrl2: (bar3, price3), p2: (abs_idx, price), color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingPath => {
                                        self.polyline_points.push((abs_idx, price));
                                        // Keep collecting — double-click finishes (handled in polyline dbl-click)
                                    }
                                    DrawMode::PlacingForecastP1 => {
                                        self.draw_mode = DrawMode::PlacingForecastP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingForecastP2 { bar1, price1 } => {
                                        chart.drawings.push(Drawing::Forecast { p1: (bar1, price1), p2: (abs_idx, price), color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingGhostFeedP1 => {
                                        self.draw_mode = DrawMode::PlacingGhostFeedP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingGhostFeedP2 { bar1, price1 } => {
                                        chart.drawings.push(Drawing::GhostFeed { p1: (bar1, price1), p2: (abs_idx, price), color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingSignpost => {
                                        chart.drawings.push(Drawing::Signpost { bar_idx: abs_idx, price, color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingRulerP1 => {
                                        self.draw_mode = DrawMode::PlacingRulerP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingRulerP2 { bar1, price1 } => {
                                        chart.drawings.push(Drawing::Ruler { p1: (bar1, price1), p2: (abs_idx, price), color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingTimeCycleP1 => {
                                        self.draw_mode = DrawMode::PlacingTimeCycleP2 { bar1: abs_idx };
                                    }
                                    DrawMode::PlacingTimeCycleP2 { bar1 } => {
                                        chart.drawings.push(Drawing::TimeCycle { bar_start: bar1, bar_end: abs_idx, color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingSpeedFanP1 => {
                                        self.draw_mode = DrawMode::PlacingSpeedFanP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingSpeedFanP2 { bar1, price1 } => {
                                        self.draw_mode = DrawMode::PlacingSpeedFanP3 { bar1, price1, bar2: abs_idx, price2: price };
                                    }
                                    DrawMode::PlacingSpeedFanP3 { bar1, price1, bar2, price2 } => {
                                        chart.drawings.push(Drawing::SpeedResistanceFan { p1: (bar1, price1), p2: (bar2, price2), p3: (abs_idx, price), color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingSpeedArcP1 => {
                                        self.draw_mode = DrawMode::PlacingSpeedArcP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingSpeedArcP2 { bar1, price1 } => {
                                        self.draw_mode = DrawMode::PlacingSpeedArcP3 { bar1, price1, bar2: abs_idx, price2: price };
                                    }
                                    DrawMode::PlacingSpeedArcP3 { bar1, price1, bar2, price2 } => {
                                        chart.drawings.push(Drawing::SpeedResistanceArc { p1: (bar1, price1), p2: (bar2, price2), p3: (abs_idx, price), color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingFibSpiralP1 => {
                                        self.draw_mode = DrawMode::PlacingFibSpiralP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingFibSpiralP2 { bar1, price1 } => {
                                        chart.drawings.push(Drawing::FibSpiral { center: (bar1, price1), radius_pt: (abs_idx, price), color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingRotatedRectP1 => {
                                        self.draw_mode = DrawMode::PlacingRotatedRectP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingRotatedRectP2 { bar1, price1 } => {
                                        self.draw_mode = DrawMode::PlacingRotatedRectP3 { bar1, price1, bar2: abs_idx, price2: price };
                                    }
                                    DrawMode::PlacingRotatedRectP3 { bar1, price1, bar2, price2 } => {
                                        chart.drawings.push(Drawing::RotatedRectangle { p1: (bar1, price1), p2: (bar2, price2), p3: (abs_idx, price), color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingAnchoredVwap => {
                                        chart.drawings.push(Drawing::AnchoredVwapLine { bar_idx: abs_idx, color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingTrendChannelP1 => {
                                        self.draw_mode = DrawMode::PlacingTrendChannelP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingTrendChannelP2 { bar1, price1 } => {
                                        self.draw_mode = DrawMode::PlacingTrendChannelP3 { bar1, price1, bar2: abs_idx, price2: price };
                                    }
                                    DrawMode::PlacingTrendChannelP3 { bar1, price1, bar2, price2 } => {
                                        chart.drawings.push(Drawing::TrendChannel { p1: (bar1, price1), p2: (bar2, price2), p3: (abs_idx, price), color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingInsidePitchforkP1 => {
                                        self.draw_mode = DrawMode::PlacingInsidePitchforkP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingInsidePitchforkP2 { bar1, price1 } => {
                                        self.draw_mode = DrawMode::PlacingInsidePitchforkP3 { bar1, price1, bar2: abs_idx, price2: price };
                                    }
                                    DrawMode::PlacingInsidePitchforkP3 { bar1, price1, bar2, price2 } => {
                                        chart.drawings.push(Drawing::InsidePitchfork { pivot: (bar1, price1), p2: (bar2, price2), p3: (abs_idx, price), color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingFibWedgeP1 => {
                                        self.draw_mode = DrawMode::PlacingFibWedgeP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingFibWedgeP2 { bar1, price1 } => {
                                        self.draw_mode = DrawMode::PlacingFibWedgeP3 { bar1, price1, bar2: abs_idx, price2: price };
                                    }
                                    DrawMode::PlacingFibWedgeP3 { bar1, price1, bar2, price2 } => {
                                        chart.drawings.push(Drawing::FibWedge { p1: (bar1, price1), p2: (bar2, price2), p3: (abs_idx, price), color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingPriceNote => {
                                        chart.drawings.push(Drawing::PriceNote { price, text: "Note".to_string(), color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingMeasureToolP1 => {
                                        self.draw_mode = DrawMode::PlacingMeasureToolP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingMeasureToolP2 { bar1, price1 } => {
                                        chart.drawings.push(Drawing::MeasureTool { p1: (bar1, price1), p2: (abs_idx, price), color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    // ── New 1-click tools ──
                                    DrawMode::PlacingAnchoredText => {
                                        chart.drawings.push(Drawing::AnchoredText { bar_idx: abs_idx, price, text: "Text".to_string(), color: egui::Color32::WHITE });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingComment => {
                                        chart.drawings.push(Drawing::Comment { bar_idx: abs_idx, price, text: "Comment".to_string(), color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingArrowMarkerLeft => {
                                        chart.drawings.push(Drawing::ArrowMarkerLeft { bar_idx: abs_idx, price, color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingArrowMarkerRight => {
                                        chart.drawings.push(Drawing::ArrowMarkerRight { bar_idx: abs_idx, price, color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    // ── New 2-click tools ──
                                    DrawMode::PlacingCircleP1 => {
                                        self.draw_mode = DrawMode::PlacingCircleP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingCircleP2 { bar1, price1 } => {
                                        chart.drawings.push(Drawing::Circle { p1: (bar1, price1), p2: (abs_idx, price), color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingPitchFanP1 => {
                                        self.draw_mode = DrawMode::PlacingPitchFanP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingPitchFanP2 { bar1, price1 } => {
                                        chart.drawings.push(Drawing::PitchFan { p1: (bar1, price1), p2: (abs_idx, price), color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingTrendFibTimeP1 => {
                                        self.draw_mode = DrawMode::PlacingTrendFibTimeP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingTrendFibTimeP2 { bar1, price1 } => {
                                        chart.drawings.push(Drawing::TrendFibTime { p1: (bar1, price1), p2: (abs_idx, price), color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingGannSquareP1 => {
                                        self.draw_mode = DrawMode::PlacingGannSquareP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingGannSquareP2 { bar1, price1 } => {
                                        chart.drawings.push(Drawing::GannSquare { p1: (bar1, price1), p2: (abs_idx, price), color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingGannSquareFixedP1 => {
                                        self.draw_mode = DrawMode::PlacingGannSquareFixedP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingGannSquareFixedP2 { bar1, price1 } => {
                                        chart.drawings.push(Drawing::GannSquareFixed { p1: (bar1, price1), p2: (abs_idx, price), color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingBarsPatternP1 => {
                                        self.draw_mode = DrawMode::PlacingBarsPatternP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingBarsPatternP2 { bar1, price1 } => {
                                        chart.drawings.push(Drawing::BarsPattern { p1: (bar1, price1), p2: (abs_idx, price), color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingProjectionP1 => {
                                        self.draw_mode = DrawMode::PlacingProjectionP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingProjectionP2 { bar1, price1 } => {
                                        chart.drawings.push(Drawing::Projection { p1: (bar1, price1), p2: (abs_idx, price), color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingDoubleCurveP1 => {
                                        self.draw_mode = DrawMode::PlacingDoubleCurveP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingDoubleCurveP2 { bar1, price1 } => {
                                        chart.drawings.push(Drawing::DoubleCurve { p1: (bar1, price1), p2: (abs_idx, price), color: dc });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    // ── New multi-click tools ──
                                    DrawMode::PlacingTrianglePattern => {
                                        self.multi_click_points.push((abs_idx, price));
                                        if self.multi_click_points.len() >= 3 {
                                            let pts = self.multi_click_points.drain(..).collect();
                                            chart.drawings.push(Drawing::TrianglePattern { points: pts, color: dc });
                                            self.draw_mode = DrawMode::None;
                                        }
                                    }
                                    DrawMode::PlacingThreeDrives => {
                                        self.multi_click_points.push((abs_idx, price));
                                        if self.multi_click_points.len() >= 3 {
                                            let pts = self.multi_click_points.drain(..).collect();
                                            chart.drawings.push(Drawing::ThreeDrives { points: pts, color: dc });
                                            self.draw_mode = DrawMode::None;
                                        }
                                    }
                                    DrawMode::PlacingElliottDouble => {
                                        self.multi_click_points.push((abs_idx, price));
                                        if self.multi_click_points.len() >= 3 {
                                            let pts = self.multi_click_points.drain(..).collect();
                                            chart.drawings.push(Drawing::ElliottDouble { points: pts, color: dc });
                                            self.draw_mode = DrawMode::None;
                                        }
                                    }
                                    DrawMode::PlacingAbcdPattern => {
                                        self.multi_click_points.push((abs_idx, price));
                                        if self.multi_click_points.len() >= 4 {
                                            let pts = self.multi_click_points.drain(..).collect();
                                            chart.drawings.push(Drawing::AbcdPattern { points: pts, color: dc });
                                            self.draw_mode = DrawMode::None;
                                        }
                                    }
                                    DrawMode::PlacingCypherPattern => {
                                        self.multi_click_points.push((abs_idx, price));
                                        if self.multi_click_points.len() >= 5 {
                                            let pts = self.multi_click_points.drain(..).collect();
                                            chart.drawings.push(Drawing::CypherPattern { points: pts, color: dc });
                                            self.draw_mode = DrawMode::None;
                                        }
                                    }
                                    DrawMode::PlacingElliottTriangle => {
                                        self.multi_click_points.push((abs_idx, price));
                                        if self.multi_click_points.len() >= 5 {
                                            let pts = self.multi_click_points.drain(..).collect();
                                            chart.drawings.push(Drawing::ElliottTriangle { points: pts, color: dc });
                                            self.draw_mode = DrawMode::None;
                                        }
                                    }
                                    DrawMode::PlacingElliottTripleCombo => {
                                        self.multi_click_points.push((abs_idx, price));
                                        if self.multi_click_points.len() >= 5 {
                                            let pts = self.multi_click_points.drain(..).collect();
                                            chart.drawings.push(Drawing::ElliottTripleCombo { points: pts, color: dc });
                                            self.draw_mode = DrawMode::None;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // ── right-click context menu ─────────────────────────
                    resp.context_menu(|ui| {
                        ui.label(egui::RichText::new("Drawing Tools").color(ACCENT).strong());
                        ui.separator();
                        if ui.button("Horizontal Line").clicked() {
                            self.draw_mode = DrawMode::PlacingHLine;
                            ui.close();
                        }
                        if ui.button("Trendline (2 clicks)").clicked() {
                            self.draw_mode = DrawMode::PlacingTrendP1;
                            ui.close();
                        }
                        if ui.button("Fibonacci Retracement").clicked() {
                            self.draw_mode = DrawMode::PlacingFiboP1;
                            ui.close();
                        }
                        if ui.button("Vertical Line").clicked() {
                            self.draw_mode = DrawMode::PlacingVLine;
                            ui.close();
                        }
                        if ui.button("Rectangle (2 clicks)").clicked() {
                            self.draw_mode = DrawMode::PlacingRectP1;
                            ui.close();
                        }
                        if ui.button("Ray (2 clicks)").clicked() {
                            self.draw_mode = DrawMode::PlacingRayP1;
                            ui.close();
                        }
                        if ui.button("Channel (3 clicks)").clicked() {
                            self.draw_mode = DrawMode::PlacingChannelP1;
                            ui.close();
                        }
                        ui.separator();
                        if !chart.drawings.is_empty() {
                            ui.menu_button("Drawing Color", |ui| {
                                let colors = [
                                    ("White", egui::Color32::WHITE),
                                    ("Yellow", egui::Color32::from_rgb(255, 200, 50)),
                                    ("Green", egui::Color32::from_rgb(0, 220, 80)),
                                    ("Red", egui::Color32::from_rgb(220, 40, 40)),
                                    ("Cyan", egui::Color32::from_rgb(0, 200, 255)),
                                    ("Magenta", egui::Color32::from_rgb(255, 100, 255)),
                                    ("Orange", egui::Color32::from_rgb(255, 140, 0)),
                                    ("Blue", egui::Color32::from_rgb(80, 120, 255)),
                                ];
                                for (name, color) in &colors {
                                    if ui.button(egui::RichText::new(*name).color(*color)).clicked() {
                                        // Apply color to selected drawing, or last if none selected
                                        let target_idx = chart.selected_drawing.unwrap_or(chart.drawings.len().saturating_sub(1));
                                        if let Some(d) = chart.drawings.get_mut(target_idx) {
                                            // Generic: try setting color on common variants
                                            macro_rules! set_color {
                                                ($d:expr, $c:expr, $($variant:ident),+) => {
                                                    match $d {
                                                        $(Drawing::$variant { color: col, .. } => *col = $c,)+
                                                        _ => {}
                                                    }
                                                };
                                            }
                                            set_color!(d, *color,
                                                HLine, TrendLine, Rectangle, Ray, Channel,
                                                ExtendedLine, HRay, CrossLine, ArrowLine,
                                                InfoLine, Pitchfork, FiboExtension, GannFan,
                                                TextLabel, ArrowMarker, Ellipse, Triangle,
                                                TrendAngle, ParallelChannel, FibChannel,
                                                FibTimeZones, Callout, Highlighter, Polyline,
                                                AnchorNote, RegressionChannel, GannBox,
                                                ElliottWave, AbcCorrection, HeadShoulders,
                                                XabcdPattern, Brush, SchiffPitchfork,
                                                ModSchiffPitchfork, CyclicLines, SineWave,
                                                Flag, Balloon, SessionBreak, MagnetLevel,
                                                FibCircle, ArcDraw, CurveDraw, PathDraw,
                                                Ruler, TimeCycle, SpeedResistanceFan,
                                                SpeedResistanceArc, FibSpiral, RotatedRectangle,
                                                AnchoredVwapLine, TrendChannel, InsidePitchfork,
                                                FibWedge, PriceNote, MeasureTool, PriceLabel,
                                                CrossMarker, Forecast, GhostFeed, Signpost,
                                                VLine, AnchoredText, Comment, ArrowMarkerLeft,
                                                ArrowMarkerRight, Circle, PitchFan, TrendFibTime,
                                                GannSquare, GannSquareFixed, BarsPattern, Projection,
                                                DoubleCurve, TrianglePattern, ThreeDrives,
                                                ElliottDouble, AbcdPattern, CypherPattern,
                                                ElliottTriangle, ElliottTripleCombo
                                            );
                                        }
                                        ui.close();
                                    }
                                }
                            });
                        }
                        // Per-drawing width/style editor (for selected drawing)
                        if let Some(sel) = chart.selected_drawing {
                            ui.menu_button("Drawing Width", |ui| {
                                for w in [1.0_f32, 1.5, 2.0, 3.0, 4.0] {
                                    if ui.button(format!("{}px", w)).clicked() {
                                        if let Some(style) = chart.drawing_styles.get_mut(sel) {
                                            style.0 = w;
                                        }
                                        ui.close();
                                    }
                                }
                            });
                            ui.menu_button("Drawing Style", |ui| {
                                if ui.button("━ Solid").clicked() {
                                    if let Some(style) = chart.drawing_styles.get_mut(sel) { style.1 = LineStyle::Solid; }
                                    ui.close();
                                }
                                if ui.button("╌ Dashed").clicked() {
                                    if let Some(style) = chart.drawing_styles.get_mut(sel) { style.1 = LineStyle::Dashed; }
                                    ui.close();
                                }
                                if ui.button("┈ Dotted").clicked() {
                                    if let Some(style) = chart.drawing_styles.get_mut(sel) { style.1 = LineStyle::Dotted; }
                                    ui.close();
                                }
                            });
                            if ui.button("Delete Selected").clicked() {
                                let d = chart.drawings.remove(sel);
                                if sel < chart.drawing_styles.len() { chart.drawing_styles.remove(sel); }
                                chart.drawings_undo.push(d);
                                chart.selected_drawing = None;
                                ui.close();
                            }
                            ui.separator();
                        }
                        if ui.button("Remove Last Drawing").clicked() {
                            chart.drawings.pop();
                            ui.close();
                        }
                        if ui.button("Clear All Drawings").clicked() {
                            chart.drawings.clear();
                            ui.close();
                        }
                        ui.separator();
                        ui.label(egui::RichText::new("Chart").color(ACCENT).strong());
                        ui.separator();
                        if ui.button("Reset Zoom / Pan").clicked() {
                            chart.price_zoom = 1.0;
                            chart.price_pan = 0.0;
                            chart.visible_bars = 200;
                            chart.view_offset = chart.bars.len().saturating_sub(1) + CHART_RIGHT_MARGIN;
                            chart.manual_view_override = false;
                            ui.close();
                        }
                        if ui.button(if chart.log_scale { "● Log Scale" } else { "  Log Scale" }).clicked() {
                            chart.log_scale = !chart.log_scale;
                            ui.close();
                        }
                        if ui.button("Fit All Bars").clicked() {
                            chart.visible_bars = chart.bars.len().max(50);
                            chart.view_offset = chart.bars.len().saturating_sub(1) + CHART_RIGHT_MARGIN;
                            chart.price_zoom = 1.0;
                            chart.price_pan = 0.0;
                            chart.manual_view_override = false;
                            ui.close();
                        }
                        ui.separator();
                        for &ct in &[ChartType::Candle, ChartType::HeikinAshi, ChartType::Line, ChartType::OhlcBars, ChartType::Renko] {
                            let label = if chart.chart_type == ct { format!("● {}", ct.label()) } else { format!("  {}", ct.label()) };
                            if ui.button(label).clicked() {
                                chart.chart_type = ct;
                                ui.close();
                            }
                        }
                        ui.separator();
                        ui.label(egui::RichText::new("Timeframe").color(ACCENT).strong());
                        ui.separator();
                        for &tf in Timeframe::all() {
                            let label = if chart.timeframe == tf { format!("● {}", tf.label()) } else { format!("  {}", tf.label()) };
                            if ui.button(label).clicked() {
                                chart.timeframe = tf;
                                if let Some(ref cache_arc) = self.cache {
                                    let mut gpu = self.gpu_indicators.take();
                                    chart.try_load(Arc::as_ref(cache_arc), &mut self.log, gpu.as_mut());
                                    self.gpu_indicators = gpu;
                                }
                                ui.close();
                            }
                        }
                        ui.separator();
                        ui.label(egui::RichText::new("Windows").color(ACCENT).strong());
                        ui.separator();
                        if ui.button("Indicators…").clicked() { self.show_indicators_panel = true; ui.close(); }
                        if ui.button("Data Window").clicked() { self.show_data_window = true; ui.close(); }
                        if ui.button("Volume Profile").clicked() { self.show_volume_profile = true; ui.close(); }
                        if ui.button("Price Alerts…").clicked() { self.show_alerts = true; ui.close(); }
                        // ADR-094: Open command palette with chart context
                        if ui.button("Command Palette…").clicked() {
                            self.palette_context = PaletteContext::Chart;
                            self.command_open = true;
                            self.command_input.clear();
                            ui.close();
                        }
                        // Copy price at crosshair
                        if let Some(pos) = crosshair {
                            ui.separator();
                            if ui.button("Copy Price at Cursor").clicked() {
                                let frac = (pos.y - rect.top()) / (rect.height() - 80.0);
                                let (si, ei) = chart.visible_range();
                                let vis = &chart.bars[si..ei];
                                if !vis.is_empty() {
                                    let hi = vis.iter().map(|b| b.high).fold(f64::MIN, f64::max);
                                    let lo = vis.iter().map(|b| b.low).fold(f64::MAX, f64::min);
                                    let price = hi - frac as f64 * (hi - lo);
                                    ctx.copy_text(format_price(price));
                                }
                                ui.close();
                            }
                        }
                    });
                }
            }
        });

        // ── Console (egui::Window for proper focus/interaction on Wayland) ────
        if self.command_open {
            // ADR-092: When filter is empty, show recent commands first
            let filter_empty = self.command_input.trim().is_empty();
            // ADR-094: Context-aware command filtering
            let context_filter: Option<&[&str]> = match self.palette_context {
                PaletteContext::Global => None,
                PaletteContext::Chart => Some(&[
                    "DRAW_HLINE",
                    "DRAW_TRENDLINE",
                    "DRAW_FIBO",
                    "DRAW_VLINE",
                    "DRAW_RECT",
                    "DRAW_RAY",
                    "DRAW_CHANNEL",
                    "DRAW_PARALLEL_CH",
                    "DRAW_FIB_CHANNEL",
                    "DRAW_REGRESSION",
                    "NNFX",
                    "RESET_IND",
                    "SESSIONS",
                    "SUPERTREND",
                    "DONCHIAN",
                    "KELTNER",
                    "BOLLINGER",
                    "ICHIMOKU",
                    "SQUEEZE",
                    "REGRESSION",
                    "FVG",
                    "ORDER_BLOCKS",
                    "CANDLE",
                    "HEIKINASHI",
                    "LINE",
                    "OHLC",
                    "RENKO",
                    "M1",
                    "M5",
                    "M15",
                    "M30",
                    "H1",
                    "H4",
                    "D1",
                    "W1",
                    "MN1",
                    "SCREENSHOT",
                    "COPY_CHART",
                    "REPLAY",
                    "VOLUME_PROFILE",
                    "VWAP",
                ]),
                PaletteContext::Watchlist => Some(&[
                    "SEARCH",
                    "QUOTE",
                    "FUNDAMENTALS",
                    "SEC",
                    "INSIDER",
                    "EV",
                    "EARNINGS",
                    "DIVIDENDS",
                    "ANALYST",
                    "SHORT_INTEREST",
                    "ALERTS",
                    "NEWS",
                    "OPTIONS",
                ]),
            };
            let palette_commands: Vec<&Command> =
                if filter_empty && !self.recent_commands.is_empty() {
                    // Show recent commands first, then all commands
                    let mut cmds: Vec<&Command> = Vec::new();
                    for name in &self.recent_commands {
                        if let Some(c) = COMMANDS.iter().find(|c| c.name == name.as_str()) {
                            if !cmds.iter().any(|x: &&Command| x.name == c.name) {
                                cmds.push(c);
                            }
                        }
                    }
                    for c in COMMANDS.iter() {
                        if !cmds.iter().any(|x: &&Command| x.name == c.name) {
                            cmds.push(c);
                        }
                    }
                    cmds
                } else {
                    // PERF: lowercase the query ONCE, read pre-lowercased name/desc from COMMANDS_LOWER.
                    let query_lower = self.command_input.to_lowercase();
                    let mut scored: Vec<(i32, &Command)> = COMMANDS
                        .iter()
                        .enumerate()
                        .filter_map(|(idx, c)| {
                            let ctx_match =
                                context_filter.map_or(true, |allowed| allowed.contains(&c.name));
                            if !ctx_match {
                                return None;
                            }
                            let (ref name_lc, ref desc_lc) = COMMANDS_LOWER[idx];
                            let name_score = fuzzy_score(&query_lower, name_lc);
                            let desc_score = fuzzy_score(&query_lower, desc_lc).map(|s| s + 500);
                            match (name_score, desc_score) {
                                (Some(n), Some(d)) => Some((n.min(d), c)),
                                (Some(n), None) => Some((n, c)),
                                (None, Some(d)) => Some((d, c)),
                                (None, None) => None,
                            }
                        })
                        .collect();
                    scored.sort_by_key(|(s, _)| *s);
                    scored.into_iter().map(|(_, c)| c).collect()
                };
            // Reset context to Global after opening (one-shot filtering)
            if filter_empty && self.palette_context != PaletteContext::Global {
                // Keep context while palette is open — reset on close
            }

            let num_visible = palette_commands.len().clamp(1, 15);
            let console_height = (num_visible as f32) * 24.0 + 52.0;

            let screen_width = ctx.input(|i| i.viewport_rect()).width();
            egui::Window::new("__console__")
                .title_bar(false)
                .anchor(egui::Align2::LEFT_TOP, [0.0, 0.0])
                .fixed_size([screen_width, console_height])
                .frame(
                    egui::Frame::window(&ctx.global_style())
                        .fill(egui::Color32::from_rgba_premultiplied(8, 8, 24, 247))
                        .inner_margin(8.0)
                        .stroke(egui::Stroke::new(2.0, egui::Color32::from_rgb(76, 175, 80))),
                )
                .show(ctx, |ui| {
                    let input_resp = ui.add(
                        egui::TextEdit::singleline(&mut self.command_input)
                            .desired_width(screen_width - 24.0)
                            .hint_text("type a command… (Esc to close)")
                            .font(egui::FontId::monospace(14.0))
                            .text_color(egui::Color32::from_rgb(76, 175, 80)),
                    );
                    input_resp.request_focus();

                    // Arrow key navigation
                    let cmd_count = palette_commands.len();
                    let arrow_down = ctx.input(|i| i.key_pressed(egui::Key::ArrowDown));
                    let arrow_up = ctx.input(|i| i.key_pressed(egui::Key::ArrowUp));
                    if arrow_down && cmd_count > 0 {
                        self.console_selected =
                            (self.console_selected + 1).min(cmd_count.saturating_sub(1));
                    }
                    if arrow_up && cmd_count > 0 {
                        self.console_selected = self.console_selected.saturating_sub(1);
                    }
                    // Reset selection only when user actually types (not arrow-key driven changes)
                    if input_resp.changed() && !arrow_down && !arrow_up {
                        self.console_selected = 0;
                    }

                    ui.separator();

                    let mut execute: Option<String> = None;
                    // Build the MRU set once — was running iter().take(10).any() per row × N commands.
                    let recent_set: std::collections::HashSet<&str> = if filter_empty {
                        self.recent_commands
                            .iter()
                            .take(10)
                            .map(|s| s.as_str())
                            .collect()
                    } else {
                        std::collections::HashSet::new()
                    };
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .max_height(console_height - 52.0)
                        .show(ui, |ui| {
                            for (i, cmd) in palette_commands.iter().enumerate() {
                                let is_selected = i == self.console_selected;
                                let row_bg = if is_selected {
                                    egui::Color32::from_rgb(15, 52, 96)
                                } else {
                                    egui::Color32::TRANSPARENT
                                };
                                let name_col = if is_selected {
                                    egui::Color32::WHITE
                                } else {
                                    egui::Color32::from_rgb(136, 255, 255)
                                };

                                let row = ui.horizontal(|ui| {
                                    // Selected row background
                                    let rect = ui.available_rect_before_wrap();
                                    let row_rect = egui::Rect::from_min_size(
                                        rect.min,
                                        egui::vec2(rect.width(), 20.0),
                                    );
                                    ui.painter().rect_filled(row_rect, 0.0, row_bg);

                                    ui.label(
                                        egui::RichText::new(cmd.name)
                                            .color(name_col)
                                            .monospace()
                                            .strong()
                                            .size(13.0),
                                    );
                                    // ADR-092: show RECENT badge for MRU commands (O(1) HashSet lookup).
                                    if recent_set.contains(cmd.name) {
                                        ui.label(
                                            egui::RichText::new("RECENT")
                                                .color(egui::Color32::from_rgb(76, 175, 80))
                                                .size(9.0),
                                        );
                                    }
                                    ui.add_space(12.0);
                                    ui.label(
                                        egui::RichText::new(cmd.desc)
                                            .color(egui::Color32::from_rgb(136, 136, 136))
                                            .size(11.0),
                                    );
                                });
                                // Click: execute the selected palette row verbatim (no arguments).
                                if row.response.interact(egui::Sense::click()).clicked() {
                                    execute = Some(cmd.name.to_string());
                                }
                            }
                        });

                    // Enter key: if the user typed arguments (whitespace present after the
                    // command name), pass the raw input through so commands like
                    // `ASKGEMINI CC,NCLH what's their debt?` keep their arguments. Otherwise
                    // use the currently-selected palette entry so fuzzy-match still works.
                    if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                        let raw = self.command_input.trim().to_string();
                        if raw.contains(char::is_whitespace) {
                            // User typed command + args — honour them verbatim.
                            execute = Some(raw);
                        } else {
                            execute = palette_commands
                                .get(self.console_selected)
                                .map(|c| c.name.to_string());
                        }
                    }
                    if let Some(cmd_name) = execute {
                        self.command_open = false;
                        // ADR-092: track recent commands (MRU, max 10). For commands with
                        // arguments we only remember the leading token so the MRU list stays
                        // clean and repeat-able.
                        let mru_key = cmd_name
                            .split_whitespace()
                            .next()
                            .unwrap_or(&cmd_name)
                            .to_uppercase();
                        self.recent_commands.retain(|n| n != &mru_key);
                        self.recent_commands.push_front(mru_key);
                        self.recent_commands.truncate(10);
                        self.handle_command(&cmd_name, ctx);
                    }
                });
        }

        // Auto-save session + keyring sync every 60 seconds — runs off UI thread
        if now_instant.duration_since(self.session_last_autosave)
            >= std::time::Duration::from_secs(60)
        {
            self.session_last_autosave = now_instant;
            // Collect all state needed for save (cheap copies of strings + JSON)
            let session_json = self.build_session_json();
            self.sync_preferences_save();
            let creds: Vec<(String, String)> = [
                (keyring::keys::ALPACA_API_KEY, &self.broker_api_key),
                (keyring::keys::ALPACA_SECRET, &self.broker_secret),
                (keyring::keys::FINNHUB_KEY, &self.finnhub_key),
                (keyring::keys::FRED_KEY, &self.fred_key),
                (keyring::keys::CRYPTOPANIC_KEY, &self.cryptopanic_key),
            ]
            .iter()
            .filter(|(_, v)| !v.is_empty())
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
            let cache_clone = self.cache.clone();
            let rt_handle = self.rt_handle.clone();
            rt_handle.spawn_blocking(move || {
                // Write session JSON to disk
                let mut path = dirs_home();
                path.push("session.json");
                let _ = std::fs::write(&path, session_json);
                // Sync credentials to keyring (only if changed)
                for (key, val) in &creds {
                    if let Ok(Some(existing)) = keyring::load(key) {
                        if &existing == val {
                            continue;
                        }
                    }
                    let _ = keyring::store(key, val);
                    // Also write to cache fallback
                    if let Some(ref cache) = cache_clone {
                        let _ = cache.put_kv(&format!("cred:{}", key), val);
                    }
                }
            });
        }

        // Update Prometheus metrics every ~5 seconds. Keep this wall-clock gated;
        // frame_count-based throttles become pathological under 144/240Hz repaint.
        if now_instant.duration_since(self.metrics_last_update) >= std::time::Duration::from_secs(5)
        {
            self.metrics_last_update = now_instant;
            if let Some(ref reg) = self.metrics_registry {
                let mut snap = crate::metrics::MetricsSnapshot::default();

                // Uptime
                snap.uptime_seconds = self.metrics_start.elapsed().as_secs_f64();

                // Broker connection
                snap.broker_connected.push((
                    "alpaca".to_string(),
                    if self.broker_connected { 1.0 } else { 0.0 },
                ));

                // Account equity from live account
                if let Some(ref acct) = self.live_account {
                    snap.account_equity
                        .push(("alpaca".to_string(), acct.equity));
                }

                // Open positions count
                snap.positions_open
                    .push(("alpaca".to_string(), self.live_positions.len() as f64));

                // Price alerts
                snap.alerts_active = self.alerts.len() as f64 + self.indicator_alerts.len() as f64;

                // Cache stats: (rows, kv_entries, size_bytes)
                if let Some((rows, _kv, size)) = self.bg.cache_stats {
                    snap.cache_size_bytes = size as f64;
                    snap.cache_symbols_total = rows as f64;
                }

                // Detailed stats: bar counts per symbol/TF (skip metadata keys)
                for (key, count, _size) in &self.bg.detailed_stats {
                    // Cache metadata rows all follow `<prefix>:__<NAME>__[…]`
                    // (SYMBOLS, SPECS, SERVER, HEARTBEAT, …). Matching the `:__` segment
                    // covers any new metadata name without a hardcoded allow-list.
                    if key.contains(":__") {
                        continue;
                    }
                    // Skip 0-count entries to reduce cardinality
                    if *count == 0 {
                        continue;
                    }
                    // key format: "source:SYMBOL:TF" or "SYMBOL:TF"
                    let parts: Vec<&str> = key.rsplitn(2, ':').collect();
                    if parts.len() == 2 {
                        snap.bars
                            .push((parts[1].to_string(), parts[0].to_string(), *count as f64));
                    }
                }

                reg.update(&snap);
            }
        }

        // ── Data sync ───────────────────────────────────────────────────────
        // No API calls or data operations before cache is loaded.
        if self.cache_loaded {
            // Weekend crypto sync via Kraken. Runs every ~60s, one symbol per cycle.
            // Symbols come from a hardcoded floor plus any crypto in chart tabs
            // or the user watchlist so user-added coins (incl. XMR/ZEC/DASH
            // which Alpaca doesn't list) still get weekend refresh coverage.
            if now_instant.duration_since(self.weekend_crypto_last_sync)
                >= std::time::Duration::from_secs(60)
            {
                self.weekend_crypto_last_sync = now_instant;
                let now_utc = chrono::Utc::now();
                let eastern = now_utc.with_timezone(
                    &chrono::FixedOffset::west_opt(5 * 3600)
                        .unwrap_or(chrono::FixedOffset::east(0)),
                );
                use chrono::Datelike;
                let is_weekend = matches!(
                    eastern.weekday(),
                    chrono::Weekday::Sat | chrono::Weekday::Sun
                );
                if is_weekend {
                    let mut crypto_syms: Vec<String> = [
                        "BTCUSD", "ETHUSD", "SOLUSD", "DOGEUSD", "XRPUSD", "ADAUSD", "LTCUSD",
                        "LINKUSD", "AVAXUSD", "DOTUSD",
                    ]
                    .iter()
                    .map(|s| s.to_string())
                    .collect();
                    for chart in &self.charts {
                        let bare = bare_symbol_from_key(&chart.symbol).to_uppercase();
                        if Self::demand_is_crypto(&bare) && !crypto_syms.contains(&bare) {
                            crypto_syms.push(bare);
                        }
                    }
                    for wl in &self.user_watchlist {
                        let wlu = wl.to_uppercase();
                        if Self::demand_is_crypto(&wlu) && !crypto_syms.contains(&wlu) {
                            crypto_syms.push(wlu);
                        }
                    }
                    if !crypto_syms.is_empty() {
                        let sym_idx = ((self.frame_count / 240) as usize) % crypto_syms.len();
                        let sym = crypto_syms[sym_idx].clone();
                        let db_path = cache_db_path();
                        let kraken_tfs = self.filtered_sync_timeframes([
                            "1Day", "1Hour", "4Hour", "15Min", "30Min", "5Min",
                        ]);
                        if !kraken_tfs.is_empty() && self.kraken_spot_symbol_scrape_enabled(&sym) {
                            let _ = self.broker_tx.send(BrokerCmd::KrakenBackfill {
                                symbol: sym,
                                timeframes: kraken_tfs,
                                db_path,
                                backfill_complete: false,
                            });
                        }
                    }
                }
            }

            // Alpaca equity rotation — iterate Alpaca's full us_equity tradable
            // universe (~11000 symbols), plus a chart/watchlist floor that holds
            // even before the asset-list fetch completes. Runs 7 days/week —
            // stocks don't trade on weekends but the historical backfill can
            // still progress.
            if now_instant.duration_since(self.alpaca_rotation_last_sync)
                >= self.market_data_sync_interval()
            {
                self.alpaca_rotation_last_sync = now_instant;
                if self.alpaca_enabled {
                    self.maybe_request_alpaca_asset_universe();
                    self.push_alpaca_sync_runtime_config();
                    let equity_syms = self.alpaca_equity_rotation_symbols();
                    self.schedule_alpaca_pairs(&equity_syms);
                }
            }
        }

        // Repaint strategy:
        // - Trading terminals should not idle at 4-10 FPS while prices, cursor
        //   overlays, live bars, and background sync state are moving.
        // - Request the next frame every update and let wgpu/eframe vsync cap it
        //   at the monitor's native refresh rate. This keeps UI latency low while
        //   still avoiding runaway uncapped presentation.
        // - TYPHOON_IDLE_FPS can force an explicit refresh-rate cap for
        //   profiling/problem displays; unset/0 means native-refresh continuous
        //   repaint through vsync/GSYNC/FreeSync.
        let session_save_started = std::time::Instant::now();
        let render_after_broker_ms = session_save_started
            .saturating_duration_since(perf_after_broker_started)
            .as_secs_f64()
            * 1000.0;
        self.maybe_incremental_session_save(ctx);
        let session_save_ms = session_save_started.elapsed().as_secs_f64() * 1000.0;

        let update_ms = now_instant.elapsed().as_secs_f64() * 1000.0;
        // Sampled once per frame and shared by both perf-stall logs below (the
        // per-frame detail warn and the 5s summary). Reading /proc VmRSS is a
        // few microseconds — negligible against the frame budget.
        let rss_mb = crate::app::market_data_sync::current_process_rss_mb();

        if update_ms >= 250.0 {
            let render_residual_ms = (render_after_broker_ms
                - perf_post_broker_setup_ms
                - perf_chrome_panels_ms
                - perf_floating_windows_ms)
                .max(0.0);
            tracing::warn!(
                "UI frame stall detail: update_ms={:.2} pre_broker_ms={:.2} broker_drain_ms={:.2} render_after_broker_ms={:.2} post_broker_setup_ms={:.2} chrome_panels_ms={:.2} floating_windows_ms={:.2} render_residual_ms={:.2} session_save_ms={:.2} msgs_drained={} pending_fetches={} heavy_sync={} news_loading={} fund_scrape={} sec_scrape={} compact={} rss_mb={}",
                update_ms,
                perf_pre_broker_ms,
                perf_broker_drain_ms,
                render_after_broker_ms,
                perf_post_broker_setup_ms,
                perf_chrome_panels_ms,
                perf_floating_windows_ms,
                render_residual_ms,
                session_save_ms,
                msgs_drained,
                self.total_pending_market_data_fetches(),
                self.heavy_sync_in_progress,
                self.news_loading,
                self.scrape_fund_running,
                self.scrape_sec_running,
                self.auto_compact_in_progress,
                rss_mb,
            );
        }
        if update_ms > 16.7 {
            self.perf_slow_frame_count = self.perf_slow_frame_count.saturating_add(1);
        }
        self.perf_max_update_ms = self.perf_max_update_ms.max(update_ms);
        self.perf_broker_msgs_drained = self
            .perf_broker_msgs_drained
            .saturating_add(msgs_drained as u32);
        if now_instant.duration_since(self.perf_last_report) >= std::time::Duration::from_secs(5) {
            if self.perf_slow_frame_count > 0 || self.perf_broker_msgs_drained > 0 {
                let pending_fetches = self.total_pending_market_data_fetches();
                if self.perf_max_update_ms >= 250.0 {
                    tracing::warn!(
                        "UI frame stall: max_update_ms={:.2} slow_frames={} broker_msgs={} pending_fetches={} deferred_chart_loads={} rss_mb={} heavy_sync={} news_loading={} fund_scrape={} sec_scrape={} compact={} log_entries={}",
                        self.perf_max_update_ms,
                        self.perf_slow_frame_count,
                        self.perf_broker_msgs_drained,
                        pending_fetches,
                        self.deferred_chart_loads.len(),
                        rss_mb,
                        self.heavy_sync_in_progress,
                        self.news_loading,
                        self.scrape_fund_running,
                        self.scrape_sec_running,
                        self.auto_compact_in_progress,
                        self.log.len()
                    );
                } else {
                    tracing::debug!(
                        "frame perf: max_update_ms={:.2} slow_frames={} broker_msgs={} pending_fetches={} deferred_chart_loads={} log_entries={}",
                        self.perf_max_update_ms,
                        self.perf_slow_frame_count,
                        self.perf_broker_msgs_drained,
                        pending_fetches,
                        self.deferred_chart_loads.len(),
                        self.log.len()
                    );
                }
            }
            self.perf_last_report = now_instant;
            self.perf_slow_frame_count = 0;
            self.perf_max_update_ms = 0.0;
            self.perf_broker_msgs_drained = 0;
        }

        static IDLE_FPS_CAP: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
        let idle_fps_cap = *IDLE_FPS_CAP.get_or_init(|| {
            std::env::var("TYPHOON_IDLE_FPS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(0)
        });
        if self.heavy_sync_in_progress {
            let frame_ms = if idle_fps_cap > 0 {
                (1000 / idle_fps_cap.max(1)).max(1)
            } else {
                // Keep visible progress/animations fluid under sync pressure while
                // avoiding unconstrained native-refresh repaint competing with the
                // background sync workers and the compositor.
                16
            };
            ctx.request_repaint_after(std::time::Duration::from_millis(frame_ms));
        } else if idle_fps_cap > 0 {
            let frame_ms = (1000 / idle_fps_cap.max(1)).max(1);
            ctx.request_repaint_after(std::time::Duration::from_millis(frame_ms));
        } else {
            ctx.request_repaint();
        }

        // UX3: Apply any deferred symbol context-menu action from right-panel renders
        if !matches!(self.deferred_symbol_action, SymbolAction::None) {
            let action = std::mem::replace(&mut self.deferred_symbol_action, SymbolAction::None);
            self.apply_symbol_action(action);
        }
    }
}
