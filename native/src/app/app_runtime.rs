use super::*;
use crate::app::chart_ops::mtf_visible_chart_groups;

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
            && self.lan_sync_mode != "client"
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
            && self.lan_sync_mode != "client"
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
            && self.lan_sync_mode != "client"
            && self.kraken_enabled
            && self.kraken_scrape_futures
            && self.kraken_futures_symbols.is_empty()
            && !self.kraken_futures_requested
        {
            let _ = self.broker_tx.send(BrokerCmd::KrakenFuturesGetInstruments);
            self.kraken_futures_requested = true;
        }

        // Periodic crypto bar refresh (every ~60 seconds at 4fps = every 240 frames)
        // Periodic crypto bar refresh (~60s) — works on both server and LAN client
        // Uses Kraken (free, no auth) as primary source, Alpaca as fallback
        // Periodic crypto bar refresh — SERVER/STANDALONE ONLY
        // LAN clients get ALL data from server via sync — no direct API calls
        if now_instant.duration_since(self.periodic_crypto_last_refresh)
            >= std::time::Duration::from_secs(60)
            && self.lan_sync_mode != "client"
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

        // Refresh the cached Sync Status coverage % so auto-full-tilt sees
        // current data even when the Sync Status window isn't open. The
        // compute call self-throttles by mode; broad heavy-sync snapshots are
        // deliberately slower because they scan the full xStocks/Merged matrix
        // on the UI thread.
        if self.cache_loaded {
            self.refresh_bar_sync_rows_if_stale();
        }

        if now_instant.duration_since(self.kraken_universe_last_schedule)
            >= self.market_data_sync_interval()
            && self.cache_loaded
            && self.lan_sync_mode != "client"
            && self.kraken_enabled
            && self.kraken_full_bar_sync_enabled
            && (self.kraken_any_spot_scrape_enabled()
                || (self.kraken_scrape_xstocks && !self.kraken_equity_universe_symbols.is_empty()))
        {
            self.kraken_universe_last_schedule = now_instant;
            let _ = self.schedule_kraken_universe_sectors();
            let _ = self.schedule_kraken_equities_universe();
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
            && self.lan_sync_mode != "client"
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
            // Load credentials FIRST (needed for LAN auto-connect passphrase)
            {
                let mut keyring_ok = true;
                let cache_ref = self.cache.clone();
                let cred_keys = [
                    (keyring::keys::ALPACA_API_KEY, "alpaca_api_key"),
                    (keyring::keys::ALPACA_SECRET, "alpaca_secret"),
                    (keyring::keys::FINNHUB_KEY, "finnhub_key"),
                    (keyring::keys::FRED_KEY, "fred_key"),
                    (keyring::keys::LAN_SYNC_PASS, "lan_sync_pass"),
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
                        k if k == keyring::keys::LAN_SYNC_PASS => {
                            self.lan_sync_passphrase = val.clone()
                        }
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
            // Auto-connect Alpaca if credentials are available and not a LAN client
            // (LAN clients get data from the server, no need for direct broker connection)
            if self.alpaca_enabled
                && !self.broker_api_key.is_empty()
                && !self.broker_secret.is_empty()
                && !self.lan_client_enabled
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

            // LAN KV recovery: read client_enabled FIRST so we don't misidentify a client
            // machine as a server. Stale lan:server_enabled keys could have been synced from
            // the server into the client's local KV cache (fixed in lan_sync.rs, but existing
            // client DBs may already have the poisoned key — sanitize it here).
            if let Some(ref cache) = self.cache {
                // Step 1: read client config (IP, port, client_enabled)
                if self.lan_server_ip.is_empty() {
                    if let Ok(Some(ip)) = cache.get_kv("lan:server_ip") {
                        if !ip.is_empty() {
                            self.lan_server_ip = ip;
                            self.lan_sync_host = self.lan_server_ip.clone();
                            self.log.push_back(LogEntry::info(format!(
                                "LAN server IP recovered from cache: {}",
                                self.lan_server_ip
                            )));
                        }
                    }
                }
                if let Ok(Some(port)) = cache.get_kv("lan:sync_port") {
                    if !port.is_empty() {
                        self.lan_sync_port = port;
                    }
                }
                if !self.lan_client_enabled {
                    if let Ok(Some(enabled)) = cache.get_kv("lan:client_enabled") {
                        if enabled == "true" {
                            self.lan_client_enabled = true;
                        }
                    }
                }

                // Step 2: recover server_enabled — but ONLY if this machine is not a client.
                // If lan:client_enabled is set, this is a client machine; lan:server_enabled
                // may have been synced from the server's KV and must be ignored + purged.
                if !self.lan_server_enabled && !self.lan_client_enabled {
                    if let Ok(Some(enabled)) = cache.get_kv("lan:server_enabled") {
                        if enabled == "true" {
                            self.lan_server_enabled = true;
                            self.log.push_back(LogEntry::info(
                                "LAN server_enabled recovered from cache",
                            ));
                        }
                    }
                } else if self.lan_client_enabled {
                    // Sanitize: remove any stale server_enabled that was synced from server
                    let _ = cache.put_kv("lan:server_enabled", "false");
                }
            }

            if self.lan_server_enabled && !self.lan_sync_passphrase.is_empty() {
                let port: u16 = self.lan_sync_port.parse().unwrap_or(9847);
                self.lan_sync_mode = "server".into();
                let db_path = cache_db_path();
                let _ = self.broker_tx.send(BrokerCmd::LanSyncStart {
                    port,
                    passphrase: self.lan_sync_passphrase.clone(),
                    db_path,
                });
                self.log.push_back(LogEntry::info(format!(
                    "LAN server auto-starting on wss://0.0.0.0:{}...",
                    port
                )));
            }
            // Don't auto-connect as client if we're already a server
            if self.lan_client_enabled && !self.lan_server_ip.is_empty() && !self.lan_server_enabled
            {
                let port: u16 = self.lan_sync_port.parse().unwrap_or(9847);
                self.lan_sync_mode = "client".into();
                self.lan_sync_host = self.lan_server_ip.clone();
                let db_path = cache_db_path();
                let _ = self.broker_tx.send(BrokerCmd::LanSyncConnect {
                    host: self.lan_server_ip.clone(),
                    port,
                    passphrase: self.lan_sync_passphrase.clone(),
                    db_path,
                });
                self.log.push_back(LogEntry::info(format!(
                    "LAN client auto-connecting to {}:{}...",
                    self.lan_server_ip, port
                )));
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
                // ── Startup data fetching (disabled for LAN client — server provides all data) ──
                if !self.lan_client_enabled {
                    // Auto SEC scrape on startup. Scope-derived universes may still
                    // be empty while broker/universe startup tasks are loading; do
                    // not send a misleading 0-symbol scrape. The universe-loaded
                    // BrokerMsg handler retries this once symbols arrive.
                    {
                        let symbols = self.sec_scrape_scope_symbols();
                        let symbol_count = symbols.len();
                        if should_auto_start_background_scope_scrape(
                            self.broker_scope,
                            symbol_count,
                        ) {
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
                } else {
                    self.log.push_back(LogEntry::info(
                        "LAN client mode: all data fetching disabled (server provides everything)",
                    ));
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

        // ── LAN client: load server's broker positions/account from KV cache ──
        // The server stores broker:account/positions/orders to KV on every update.
        // LAN sync's 15s incremental KV sync delivers them to the client's cache.
        // Reload every ~5s (200 frames at 250ms idle repaint) for near-live updates.
        // LAN client: reload positions/orders from server KV.
        // Check every ~5s (200 frames). Cheap local SQLite read — only deserializes
        // when KV actually changed (server writes only on position/order updates).
        if self.lan_sync_mode == "client" {
            if now_instant.duration_since(self.lan_client_last_reload)
                >= std::time::Duration::from_secs(5)
                || (self.live_positions.is_empty() && self.frame_count > 10)
            {
                self.lan_client_last_reload = now_instant;
                if let Some(ref cache) = self.cache {
                    if self.alpaca_enabled {
                        if let Ok(Some(json)) = cache.get_kv("broker:positions") {
                            if let Ok(pos) = serde_json::from_str::<Vec<PositionInfo>>(&json) {
                                self.live_positions = pos;
                            }
                        }
                        if let Ok(Some(json)) = cache.get_kv("broker:account") {
                            if let Ok(acct) = serde_json::from_str::<AccountInfo>(&json) {
                                self.live_account = Some(acct);
                            }
                        }
                        if let Ok(Some(json)) = cache.get_kv("broker:orders") {
                            if let Ok(orders) = serde_json::from_str::<Vec<OrderInfo>>(&json) {
                                self.live_orders = orders;
                            }
                        }
                    }
                    if self.kraken_enabled {
                        if let Ok(Some(json)) = cache.get_kv("broker:kr_positions") {
                            if let Ok(mut pos) = serde_json::from_str::<Vec<PositionInfo>>(&json) {
                                pos.retain(|p| {
                                    p.asset_class != "crypto_spot"
                                        && !p.asset_id.starts_with("spot:")
                                });
                                self.kr_positions = pos;
                            }
                        }
                    }
                    // Live quotes from server (update forming bars on LAN client)
                    for chart in &mut self.charts {
                        let bare = chart.symbol.split(':').last().unwrap_or("").to_string();
                        if !bare.is_empty() {
                            if let Ok(Some(qv)) = cache.get_kv(&format!("quote:{}", bare)) {
                                let parts: Vec<&str> = qv.split(',').collect();
                                if parts.len() == 2 {
                                    if let (Ok(bid), Ok(ask)) =
                                        (parts[0].parse::<f64>(), parts[1].parse::<f64>())
                                    {
                                        let mid = (bid + ask) / 2.0;
                                        if mid > 0.0 {
                                            chart.live_bid = bid;
                                            chart.live_ask = ask;
                                            chart.live_quote_at = Some(std::time::Instant::now());
                                            if let Some(bar) = chart.bars.last_mut() {
                                                bar.close = mid;
                                                bar.high = bar.high.max(mid);
                                                bar.low = bar.low.min(mid);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    // Watchlist quotes from server — only use server data for symbols the client wants
                    if let Ok(Some(json)) = cache.get_kv("broker:watchlist") {
                        if let Ok(rows) = serde_json::from_str::<Vec<WatchlistRow>>(&json) {
                            if self.user_watchlist.is_empty() {
                                self.watchlist_rows = rows;
                            } else {
                                // Filter server rows to only the client's local watchlist.
                                // O(1) per row via HashSet for exact matches, fallback to contains for partials.
                                let wl_set: std::collections::HashSet<String> = self
                                    .user_watchlist
                                    .iter()
                                    .map(|s| s.to_uppercase())
                                    .collect();
                                self.watchlist_rows = rows
                                    .into_iter()
                                    .filter(|r| {
                                        let sym = r.symbol.to_uppercase();
                                        wl_set.contains(&sym)
                                            || wl_set.iter().any(|w| {
                                                sym.contains(w.as_str()) || w.contains(&sym)
                                            })
                                    })
                                    .collect();
                            }
                        }
                    }
                }
            }
        }

        // ── deferred chart loading: non-blocking, paced attempts ──
        // Uses try_load() which returns false if cache Mutex is contended (compaction, MT5 sync).
        // Failed loads stay queued. The actual load is still expensive — cache read + GPU
        // indicators + MTF overlays — so pace restored MTF grids instead of burning
        // consecutive UI frames while broad sync/news/SEC/fundamentals are active.
        if !self.deferred_chart_loads.is_empty() {
            let load_interval =
                deferred_chart_load_interval(self.heavy_sync_in_progress, self.mtf_enabled);
            if now_instant.duration_since(self.deferred_chart_last_load_at) >= load_interval {
                let idx = self.deferred_chart_loads[0]; // VecDeque supports indexing
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
                self.mtf_grid_status.sort_by_key(|r| match r.0 {
                    "M15" => 0,
                    "M30" => 1,
                    "H1" => 2,
                    "H4" => 3,
                    "D1" => 4,
                    "W1" => 5,
                    "MN1" => 6,
                    _ => 99u8,
                });
                self.mtf_grid_rx = None; // done
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
                    if s.contains("Kraken") {
                        if !self.kraken_enabled {
                            continue;
                        }
                        self.kraken_connected = true;
                        // REST is authoritative: load balances/positions/history/orders before
                        // relying on private WS deltas.
                        let _ = self.broker_tx.send(BrokerCmd::KrakenGetBalance);
                        let _ = self.broker_tx.send(BrokerCmd::KrakenGetPositions);
                        let _ = self.broker_tx.send(BrokerCmd::KrakenFetchTrades);
                        let _ = self.broker_tx.send(BrokerCmd::KrakenFetchOpenOrders);
                        // Start private WebSocket for real-time ownTrades / openOrders.
                        let _ = self.broker_tx.send(BrokerCmd::KrakenStartPrivateWs);
                    } else {
                        if !self.alpaca_enabled {
                            continue;
                        }
                        self.broker_connected = true;
                        if self.alpaca_full_bar_sync_enabled {
                            self.log.push_back(LogEntry::info(
                                "Alpaca connected — broad Alpaca universe bar sync enabled.",
                            ));
                        } else if self.backfill_alpaca_kraken_equities_enabled {
                            self.log.push_back(LogEntry::info(
                                "Alpaca connected — Kraken assist only; broad Alpaca universe sync disabled.",
                            ));
                        } else {
                            self.log.push_back(LogEntry::info(
                                "Alpaca connected — account/trading only; broad Alpaca universe sync disabled.",
                            ));
                        }
                        // Auto-fetch positions, orders, and recent fills (Alpaca)
                        let _ = self.broker_tx.send(BrokerCmd::GetPositions);
                        let _ = self.broker_tx.send(BrokerCmd::GetOrders);
                        let _ = self.broker_tx.send(BrokerCmd::GetActivities { limit: 100 });
                        let _ = self.broker_tx.send(BrokerCmd::GetMarketClock);
                    }
                    if is_routine_market_data_status(&s) {
                        tracing::debug!("{}", s);
                    } else {
                        self.log.push_back(LogEntry::info(s));
                    }
                }
                BrokerMsg::KrakenTrades(mut trades) => {
                    if !self.kraken_enabled {
                        continue;
                    }
                    trades.sort_by(|a, b| {
                        b.time
                            .partial_cmp(&a.time)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                    if trades.len() > KRAKEN_TRADE_HISTORY_CAP {
                        trades.truncate(KRAKEN_TRADE_HISTORY_CAP);
                    }
                    let prev_trades = self.kraken_trades.len();
                    let prev_basis = self.kraken_cost_basis.len();
                    self.kraken_trades = VecDeque::from(trades);
                    self.rebuild_kraken_trade_indexes();
                    self.refresh_kraken_position_costs();
                    for c in &mut self.charts {
                        c.cached_trade_overlay_frame = 0;
                    }
                    self.kraken_trades_last_fetch = std::time::Instant::now();
                    let new_trades = self.kraken_trades.len();
                    let new_basis = self.kraken_cost_basis.len();
                    // The safety-net REST fetch normally returns the same
                    // counts as the last pull. Only surface the user log
                    // line when something actually changed; routine
                    // confirmations go to trace at debug level.
                    if new_trades != prev_trades || new_basis != prev_basis {
                        self.log.push_back(LogEntry::info(format!(
                            "Kraken: loaded {} trades; cost basis for {} held assets",
                            new_trades, new_basis
                        )));
                    } else {
                        tracing::debug!(
                            "Kraken trades resync: {} trades / {} held assets (unchanged)",
                            new_trades,
                            new_basis
                        );
                    }
                }
                BrokerMsg::KrakenLiveTrade(trade) => {
                    if !self.kraken_enabled {
                        continue;
                    }
                    let t0 = std::time::Instant::now();
                    let inserted = self.insert_kraken_live_trade(trade);
                    if inserted {
                        self.refresh_kraken_position_costs();
                        for c in &mut self.charts {
                            c.cached_trade_overlay_frame = 0;
                        }
                        let _ = self.broker_tx.send(BrokerCmd::KrakenGetBalance);
                        let _ = self.broker_tx.send(BrokerCmd::KrakenGetPositions);
                        let _ = self.broker_tx.send(BrokerCmd::KrakenFetchOpenOrders);
                    }
                    let dt = t0.elapsed();
                    if dt > std::time::Duration::from_millis(2) {
                        tracing::warn!(
                            "KrakenLiveTrade path took {:?} (inserted={})",
                            dt,
                            inserted
                        );
                    }
                }
                BrokerMsg::KrakenOpenOrders(orders) => {
                    if !self.kraken_enabled {
                        continue;
                    }
                    for order in orders {
                        let terminal = matches!(
                            order.status.as_str(),
                            "closed" | "canceled" | "cancelled" | "expired"
                        );
                        if terminal {
                            self.kraken_open_orders
                                .retain(|existing| existing.txid != order.txid);
                        } else if let Some(existing) = self
                            .kraken_open_orders
                            .iter_mut()
                            .find(|existing| existing.txid == order.txid)
                        {
                            *existing = order;
                        } else {
                            self.kraken_open_orders.push(order);
                        }
                    }
                    self.kraken_open_orders.sort_by(|a, b| {
                        b.opentm
                            .partial_cmp(&a.opentm)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                }
                BrokerMsg::KrakenWsStatus { status, message } => {
                    let should_reconcile = status == "online" && message.contains("reconnected");
                    let text = format!("Kraken WS {status}: {message}");
                    if matches!(status.as_str(), "error" | "closed") {
                        self.log.push_back(LogEntry::warn(text));
                    } else {
                        self.log.push_back(LogEntry::info(text));
                    }
                    if should_reconcile && self.kraken_enabled {
                        // A reconnect means a delta gap may exist. Pull REST snapshots so
                        // balances, cost basis, P/L, and open orders converge immediately.
                        let _ = self.broker_tx.send(BrokerCmd::KrakenGetBalance);
                        let _ = self.broker_tx.send(BrokerCmd::KrakenGetPositions);
                        let _ = self.broker_tx.send(BrokerCmd::KrakenFetchTrades);
                        let _ = self.broker_tx.send(BrokerCmd::KrakenFetchOpenOrders);
                    }
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
                BrokerMsg::KrakenWsBarsCommitted { fresh } => {
                    // Mark each (symbol, tf) WS-fresh so the REST scheduler skips
                    // refetch while the WS feed is keeping the cache current.
                    // O(n) over the flush batch; per-key insert is O(1).
                    let now_ms = chrono::Utc::now().timestamp_millis();
                    for (symbol, tf, last_bar_ts_ms) in fresh {
                        self.kraken_ws_fresh_until
                            .insert((symbol, tf), now_ms.max(last_bar_ts_ms));
                    }
                }
                BrokerMsg::KrakenWsOhlcStatus {
                    interval_min,
                    kind,
                    detail,
                } => {
                    let tf = typhoon_engine::broker::kraken::kraken_ws_interval_to_tf_label(
                        interval_min,
                    )
                    .unwrap_or("?");
                    let msg = if detail.is_empty() {
                        format!("Kraken WS OHLC {tf}: {kind}")
                    } else {
                        format!("Kraken WS OHLC {tf}: {kind} — {detail}")
                    };
                    if matches!(kind.as_str(), "disconnected" | "subscribe_failed") {
                        self.log.push_back(LogEntry::warn(msg));
                    } else {
                        self.log.push_back(LogEntry::info(msg));
                    }
                }
                BrokerMsg::Error(e) => {
                    let now = chrono::Utc::now().timestamp();
                    // Compact pass failed — clear in_progress so the gate can retry next window.
                    // Don't update last_run_ms; we want the cadence to keep trying.
                    if e.starts_with("Compact failed:")
                        || (self.auto_compact_in_progress && e.starts_with("Cannot open cache:"))
                    {
                        self.auto_compact_in_progress = false;
                        self.auto_compact_started_ms = 0;
                        self.auto_compact_last_skip = Some(format!("last attempt failed: {}", e));
                    }
                    if e.starts_with("Asset fetch failed:") {
                        self.all_broker_assets_fetched = false;
                    } else if e.starts_with("Kraken pairs:") {
                        self.kraken_pairs_requested = false;
                    } else if e.starts_with("Kraken futures instruments:") {
                        self.kraken_futures_requested = false;
                    } else if e.starts_with("Kraken equities universe failed:") {
                        self.kraken_equity_universe_requested = false;
                        let backoff = if e.contains("iapi temporarily rate-limited")
                            || e.contains("1015")
                            || e.contains("429")
                            || e.to_ascii_lowercase().contains("rate limit")
                        {
                            300
                        } else {
                            60
                        };
                        self.kraken_equity_universe_retry_after_ts = now + backoff;
                    } else if e.contains("Yahoo Chart HTTP 429") {
                        let pause = 300; // 5 minutes backoff on Yahoo rate limit
                        if now + pause > self.yahoo_chart_sync_pause_until_ts {
                            self.yahoo_chart_sync_pause_until_ts = now + pause;
                            self.yahoo_chart_sync_pause_reason = e.clone();
                            self.log.push_back(LogEntry::warn(format!(
                                "Yahoo Chart rate limited — pausing fallback lane for 5m"
                            )));
                        }
                    } else if e.contains("401") || e.contains("Unauthorized") || e.contains("403") {
                        if self.broker_connected {
                            self.broker_connected = false;
                            self.log.push_back(LogEntry::err(format!(
                                "{} — disconnected (check API keys in Settings)",
                                e
                            )));
                        }
                        // Don't log repeated auth failures
                    } else if is_routine_market_data_status(&e) {
                        tracing::debug!("{}", e);
                    } else {
                        self.log.push_back(LogEntry::err(e));
                    }
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
                    if !self.alpaca_enabled {
                        continue;
                    }
                    // Store to cache KV for LAN sync — dedup to avoid timestamp churn
                    if let Ok(json) = serde_json::to_string(&acct) {
                        self.put_kv_dedup("broker:account", &json);
                    }
                    // Broadcast to web clients
                    if let Some(ref tx) = self.web_msg_tx {
                        let _ = tx.send(typhoon_web_protocol::WebMsg::Account(
                            typhoon_web_protocol::AccountSnapshot {
                                equity: acct.equity,
                                cash: acct.cash,
                                buying_power: acct.buying_power,
                                portfolio_value: acct.portfolio_value,
                                unrealized_pl: 0.0, // computed from positions
                                initial_margin: acct.initial_margin,
                                maintenance_margin: acct.maintenance_margin,
                                currency: acct.currency.clone(),
                            },
                        ));
                    }
                    self.live_account = Some(acct);
                }
                BrokerMsg::Positions(pos) => {
                    if !self.alpaca_enabled {
                        continue;
                    }
                    self.positions_last_update_ts = chrono::Utc::now().timestamp();
                    if let Ok(json) = serde_json::to_string(&pos) {
                        self.put_kv_dedup("broker:positions", &json);
                    }
                    // Broadcast to web clients
                    if let Some(ref tx) = self.web_msg_tx {
                        let items: Vec<typhoon_web_protocol::PositionSnapshot> = pos
                            .iter()
                            .map(|p| typhoon_web_protocol::PositionSnapshot {
                                symbol: p.symbol.clone(),
                                qty: p.qty,
                                side: p.side.clone(),
                                avg_entry_price: p.avg_entry_price,
                                market_value: p.market_value,
                                unrealized_pl: p.unrealized_pl,
                                asset_class: p.asset_class.clone(),
                            })
                            .collect();
                        let _ = tx.send(typhoon_web_protocol::WebMsg::Positions { items });
                    }
                    self.live_positions = pos;
                }
                BrokerMsg::AllAssets(assets) => {
                    if !self.alpaca_enabled {
                        continue;
                    }
                    self.all_broker_assets = assets;
                    self.all_broker_assets_fetched = true;
                }
                BrokerMsg::RecentFills(fills) => {
                    if !self.alpaca_enabled {
                        continue;
                    }
                    self.recent_fills = fills;
                    // Invalidate trade overlay cache so fills show immediately
                    for c in &mut self.charts {
                        c.cached_trade_overlay_frame = 0;
                    }
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
                BrokerMsg::KrakenPositions(mut pos) => {
                    if !self.kraken_enabled {
                        continue;
                    }
                    self.positions_last_update_ts = chrono::Utc::now().timestamp();
                    pos.retain(|p| {
                        p.asset_class != "crypto_spot" && !p.asset_id.starts_with("spot:")
                    });
                    if let Ok(json) = serde_json::to_string(&pos) {
                        self.put_kv_dedup("broker:kr_positions", &json);
                    }
                    self.kr_positions = pos;
                    self.refresh_kraken_position_costs();
                    for c in &mut self.charts {
                        c.cached_trade_overlay_frame = 0;
                    }
                }
                BrokerMsg::Orders(orders) => {
                    if !self.alpaca_enabled {
                        continue;
                    }
                    self.orders_last_update_ts = chrono::Utc::now().timestamp();
                    if let Ok(json) = serde_json::to_string(&orders) {
                        self.put_kv_dedup("broker:orders", &json);
                    }
                    // Broadcast to web clients
                    if let Some(ref tx) = self.web_msg_tx {
                        let items: Vec<typhoon_web_protocol::OrderSnapshot> = orders
                            .iter()
                            .map(|o| typhoon_web_protocol::OrderSnapshot {
                                id: o.id.clone(),
                                symbol: o.symbol.clone(),
                                qty: o.qty.clone(),
                                side: o.side.clone(),
                                order_type: o.order_type.clone(),
                                status: o.status.clone(),
                                limit_price: o.limit_price.clone(),
                                stop_price: o.stop_price.clone(),
                            })
                            .collect();
                        let _ = tx.send(typhoon_web_protocol::WebMsg::Orders { items });
                    }
                    self.live_orders = orders;
                }
                BrokerMsg::OrderResult(msg) => {
                    // Compact pass completion — manual or auto. Mark scheduler idle and
                    // record the timestamp so the cadence gate counts this run.
                    // The Compact handler also emits per-200-row progress lines starting
                    // with "Compact: " — those are not completions.
                    if msg.starts_with("Compact complete:") {
                        self.auto_compact_in_progress = false;
                        self.auto_compact_started_ms = 0;
                        self.auto_compact_last_run_ms = chrono::Utc::now().timestamp_millis();
                        self.sync_preferences_save();
                    }
                    // Only refresh positions after actual trade operations (not every log message).
                    // OrderResult is used for many non-trade messages (LAN sync, backfill, etc.)
                    // that would spam GetPositions → HTTP 429 Too Many Requests.
                    let is_trade = msg.contains("filled")
                        || msg.contains("order")
                        || msg.contains("closed")
                        || msg.contains("cancelled");
                    if is_trade && self.alpaca_enabled && self.broker_connected {
                        let _ = self.broker_tx.send(BrokerCmd::GetPositions);
                        let _ = self.broker_tx.send(BrokerCmd::GetOrders);
                    }
                    if is_trade
                        && self.kraken_enabled
                        && self.kraken_connected
                        && msg.to_ascii_lowercase().contains("kraken")
                    {
                        let _ = self.broker_tx.send(BrokerCmd::KrakenGetPositions);
                        let _ = self.broker_tx.send(BrokerCmd::KrakenGetBalance);
                    }
                    // Track BARDATA progress
                    if msg.starts_with("BARDATA:") {
                        self.bardata_log.push_back(msg.clone());
                        while self.bardata_log.len() > 200 {
                            self.bardata_log.pop_front();
                        }
                        // Count any finished fetch (success, error, or empty) as completed
                        if msg.contains("bars stored")
                            || msg.contains("complete")
                            || msg.contains("failed")
                            || msg.contains("no bars")
                        {
                            self.bardata_completed += 1;
                        }
                    }
                    // ADR-094: Use Trade log level and toast for fills
                    if is_trade {
                        self.log.push_back(LogEntry::trade(&msg));
                        self.toasts.push(Toast {
                            message: msg,
                            color: egui::Color32::from_rgb(80, 220, 120),
                            created: std::time::Instant::now(),
                            duration: std::time::Duration::from_secs(5),
                            dismissable: false,
                            dismissed: false,
                        });
                    } else if is_routine_market_data_status(&msg) {
                        tracing::debug!("{}", msg);
                    } else {
                        self.log.push_back(LogEntry::info(msg));
                    }
                }
                BrokerMsg::SecScrapeResult(ref msg) => {
                    self.scrape_sec_running = false;
                    self.scrape_sec_last_msg = msg.clone();
                    self.log.push_back(LogEntry::info(msg.clone()));
                }
                BrokerMsg::FilingContent(text) => {
                    self.sec_filing_content = text;
                    self.sec_filing_loading = false;
                    // Invalidate cached summary so it re-computes for the new content.
                    self.sec_filing_summary = None;
                    self.sec_filing_summary_for.clear();
                    self.log
                        .push_back(LogEntry::info("SEC filing document loaded"));
                }
                BrokerMsg::FinnhubNewsResult(articles) => {
                    self.news_loading = false;
                    self.log.push_back(LogEntry::info(format!(
                        "Finnhub: {} articles loaded",
                        articles.len()
                    )));
                    self.news_articles = articles;
                }
                BrokerMsg::KrakenEquityUniverse(markets) => {
                    if !self.kraken_enabled {
                        continue;
                    }
                    // Symbols the iapi catalog marks as not overnight-tradeable
                    // (Some(false)). Unknown/None defaults to overnight-enabled, so
                    // only the explicit opt-outs land here.
                    self.kraken_equity_no_overnight = markets
                        .iter()
                        .filter(|market| market.overnight_trading == Some(false))
                        .map(|market| market.symbol.trim_end_matches(".EQ").to_ascii_uppercase())
                        .filter(|symbol| !symbol.is_empty())
                        .collect();
                    let mut symbols: Vec<String> = markets
                        .into_iter()
                        .filter(|market| {
                            market.tradable
                                && market.status.as_deref().unwrap_or("active") != "disabled"
                                && market.instrument_status.as_deref().unwrap_or("enabled")
                                    != "disabled"
                        })
                        .map(|market| market.symbol.trim_end_matches(".EQ").to_ascii_uppercase())
                        .filter(|symbol| !symbol.is_empty())
                        .collect();
                    symbols.sort();
                    symbols.dedup();
                    self.kraken_equity_universe_symbols = symbols;
                    self.kraken_equity_universe_requested = true;
                    self.kraken_equity_universe_retry_after_ts = 0;
                    self.bg_rev = self.bg_rev.wrapping_add(1);
                    self.log.push_back(LogEntry::info(format!(
                        "Kraken equities universe loaded: {} tradable symbols",
                        self.kraken_equity_universe_symbols.len()
                    )));
                    self.maybe_start_kraken_ws_ohlc();

                    if self.auto_sec_scrape_deferred && !self.scrape_sec_running {
                        let symbols = self.sec_scrape_scope_symbols();
                        let symbol_count = symbols.len();
                        if should_auto_start_background_scope_scrape(
                            self.broker_scope,
                            symbol_count,
                        ) {
                            let db_path = cache_db_path();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::SecScrape { db_path, symbols });
                            self.auto_sec_scrape_deferred = false;
                            self.scrape_sec_running = true;
                            self.scrape_sec_last_msg = format!(
                                "scraping Scope {} ({} symbols)...",
                                self.broker_scope_label(),
                                symbol_count
                            );
                            self.log.push_back(LogEntry::info(format!(
                                "SEC EDGAR deferred scrape started for Scope {} ({} symbols)...",
                                self.broker_scope_label(),
                                symbol_count
                            )));
                        }
                    }

                    if self.auto_fundamentals_deferred && !self.auto_fundamentals_started {
                        if !should_auto_start_kraken_fundamentals_scrape(
                            self.kraken_equity_universe_symbols.len(),
                        ) {
                            self.auto_fundamentals_deferred = false;
                            self.auto_fundamentals_started = false;
                            self.log.push_back(LogEntry::info(format!(
                                "Fundamentals deferred auto-scrape skipped for broad Kraken xStocks universe ({} symbols); use manual Fundamentals scrape for full-universe backfill",
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
                            self.auto_fundamentals_deferred = false;
                            self.auto_fundamentals_started = true;
                            self.log.push_back(LogEntry::info(
                                "Fundamentals deferred scrape started for selected source universes...",
                            ));
                        }
                    }
                }
                BrokerMsg::KrakenEquityQuote(ticker) => {
                    if !self.kraken_enabled {
                        continue;
                    }
                    let symbol = ticker.symbol.to_ascii_uppercase();
                    let last = ticker.price;
                    if last > 0.0 && last.is_finite() {
                        let received_at_ms = chrono::Utc::now().timestamp_millis();
                        self.kraken_equity_quote_meta.insert(
                            symbol.clone(),
                            KrakenEquityQuoteMeta {
                                received_at_ms,
                                quote_time_ms: ticker.time_ms,
                                delayed: ticker.delayed,
                                price: last,
                            },
                        );
                        // Do not write quote bars from the egui thread. During SEC/news
                        // sweeps SQLite can be write-locked for seconds; a single
                        // KrakenEquityQuote then blew the entire broker drain budget and
                        // froze free-look. History fetches still persist quote/history bars
                        // on blocking workers; this path is just the live UI overlay.
                        for chart in &mut self.charts {
                            let chart_sym = chart.symbol.replace('/', "").to_ascii_uppercase();
                            let chart_bare = chart_sym
                                .rsplit(':')
                                .nth(1)
                                .or_else(|| chart_sym.rsplit(':').next())
                                .unwrap_or("")
                                .trim_end_matches(".EQ")
                                .to_string();
                            if chart_bare == symbol {
                                chart.live_bid = ticker.bid;
                                chart.live_ask = ticker.ask;
                                chart.live_quote_at = Some(std::time::Instant::now());
                                if let Some(bar) = chart.bars.last_mut() {
                                    bar.close = last;
                                    bar.high = bar.high.max(last);
                                    bar.low = if bar.low > 0.0 {
                                        bar.low.min(last)
                                    } else {
                                        last
                                    };
                                }
                            }
                        }
                        let quote_updates_position = self.kr_positions.iter().any(|pos| {
                            let pos_symbol = pos
                                .symbol
                                .replace('/', "")
                                .trim_end_matches(".EQ")
                                .to_ascii_uppercase();
                            pos_symbol == symbol || pos.asset_id.ends_with(&symbol)
                        });
                        if quote_updates_position {
                            self.refresh_kraken_position_costs();
                        }
                        tracing::debug!(
                            "Kraken equities: {} bid {} ask {} last {}{}",
                            symbol,
                            format_price(ticker.bid),
                            format_price(ticker.ask),
                            format_price(last),
                            if ticker.delayed { " (delayed)" } else { "" }
                        );
                    }
                }
                BrokerMsg::KrakenEquityBars {
                    symbol,
                    timeframe,
                    count,
                } => {
                    let symbol = symbol
                        .replace('/', "")
                        .trim_end_matches(".EQ")
                        .to_ascii_uppercase();
                    let timeframe = normalize_sync_timeframe_key(&timeframe)
                        .unwrap_or(timeframe.as_str())
                        .to_string();
                    self.pending_kraken_fetches
                        .retain(|key| key != &format!("equity:{}:{}", symbol, timeframe));
                    if count == 0 {
                        self.unresolvable_mark(
                            "kraken-equities",
                            &symbol,
                            &timeframe,
                            "Kraken internal equities history returned no bars",
                        );
                        tracing::debug!("Kraken equities: no bars for {} {}", symbol, timeframe);
                    } else {
                        self.note_cached_sync_success(
                            "kraken-equities",
                            &symbol,
                            &timeframe,
                            count,
                        );
                        tracing::debug!(
                            "Kraken equities: cached {} bars for {} {}",
                            count,
                            symbol,
                            timeframe
                        );
                    }
                }
                BrokerMsg::KrakenEquityHistoryError {
                    symbol,
                    timeframe,
                    error,
                } => {
                    let symbol = symbol
                        .replace('/', "")
                        .trim_end_matches(".EQ")
                        .to_ascii_uppercase();
                    let timeframe = normalize_sync_timeframe_key(&timeframe)
                        .unwrap_or(timeframe.as_str())
                        .to_string();
                    self.pending_kraken_fetches
                        .retain(|key| key != &format!("equity:{}:{}", symbol, timeframe));
                    let iapi_rl_prefix =
                        typhoon_engine::broker::kraken::IAPI_RATE_LIMITED_ERR_PREFIX;
                    if error.contains("No data") || error.contains("no data") {
                        self.unresolvable_mark("kraken-equities", &symbol, &timeframe, &error);
                        tracing::debug!("Kraken equities: no bars for {} {}", symbol, timeframe);
                    } else if error.starts_with(iapi_rl_prefix) {
                        // Engine-side iapi gate already short-circuited the
                        // round-trip; this branch fires once per already-
                        // queued fetch as the broker thread drains them.
                        // Arm the queue-side pause to stop NEW dispatches
                        // and silence the per-fetch errors — the first 429
                        // produced a single tracing::warn at the engine.
                        // Fallback only hits if remaining_backoff has already
                        // ticked past zero in the brief window between the
                        // engine arming and our read; 60s is a generous
                        // re-probe interval.
                        let now = chrono::Utc::now().timestamp();
                        let pause = typhoon_engine::broker::kraken::iapi_rate_limited_for_secs()
                            .unwrap_or(60);
                        if now + pause > self.kraken_equities_sync_pause_until_ts {
                            self.kraken_equities_sync_pause_until_ts = now + pause;
                            self.kraken_equities_sync_pause_reason = error.clone();
                        }
                        tracing::debug!(
                            "Kraken equities: {} {} skipped — iapi back-off ({}s left)",
                            symbol,
                            timeframe,
                            pause
                        );
                    } else if error.contains("HTTP 500") && error.contains("Internal error") {
                        // Kraken's internal equities history endpoint returns transient
                        // JSON 500s (`type: Internal error`) for individual valid xStock
                        // symbols/timeframes. Across a full-catalog sweep these are common,
                        // so this must NOT pause the whole equities lane — a global freeze
                        // would let one flaky symbol stall the other ~12k. Hold just this
                        // (symbol, tf) out of the rotation via its per-symbol cooldown; the
                        // cursor moves on and the iapi limiter stays busy. Only genuine
                        // IP-wide 1015/429 (handled above) warrants a global pause.
                        self.mark_fetch_queued("kraken-equities", &symbol, &timeframe);
                        tracing::debug!(
                            "Kraken equities: {} {} skipped — iapi HTTP 500/Internal error (per-symbol cooldown)",
                            symbol,
                            timeframe
                        );
                    } else {
                        self.log.push_back(LogEntry::err(error));
                    }
                }
                BrokerMsg::Quote(symbol, bid, ask, last) => {
                    self.log.push_back(LogEntry::info(format!(
                        "{}: bid {} ask {} last {}",
                        symbol,
                        format_price(bid),
                        format_price(ask),
                        format_price(last)
                    )));
                    // Update forming bar (last bar) on any chart matching this symbol
                    if last > 0.0 {
                        let sym_norm = symbol.replace('/', "").to_uppercase();
                        for chart in &mut self.charts {
                            let chart_sym = chart.symbol.replace('/', "").to_uppercase();
                            let chart_bare = {
                                let parts: Vec<&str> = chart_sym.split(':').collect();
                                let is_tf = matches!(
                                    parts.last().map(|s| s.as_ref()),
                                    Some(
                                        "1MIN"
                                            | "5MIN"
                                            | "15MIN"
                                            | "30MIN"
                                            | "1HOUR"
                                            | "4HOUR"
                                            | "1DAY"
                                            | "1WEEK"
                                            | "1MONTH"
                                    )
                                );
                                if is_tf && parts.len() > 1 {
                                    parts[parts.len() - 2].to_string()
                                } else {
                                    chart_sym.clone()
                                }
                            };
                            if chart_bare == sym_norm
                                || chart_bare.contains(&sym_norm)
                                || sym_norm.contains(&chart_bare)
                            {
                                if let Some(bar) = chart.bars.last_mut() {
                                    bar.close = last;
                                    if last > bar.high {
                                        bar.high = last;
                                    }
                                    if last < bar.low {
                                        bar.low = last;
                                    }
                                }
                            }
                        }
                    }
                }
                BrokerMsg::WatchlistQuotes(mut rows) => {
                    // Weekend/off-hours quote providers can return empty/zero rows. Don't let a
                    // failed refresh wipe useful cached rows already displayed in the watchlist.
                    for row in &mut rows {
                        if row.last <= 0.0 {
                            if let Some(existing) = self.watchlist_rows.iter().find(|existing| {
                                existing.symbol.eq_ignore_ascii_case(&row.symbol)
                                    && existing.last > 0.0
                            }) {
                                *row = existing.clone();
                            }
                        }
                    }
                    self.watchlist_last_update_ts = chrono::Utc::now().timestamp();
                    // Store to KV for LAN clients — dedup to avoid timestamp churn
                    // Offload the expensive serialization + KV write to a blocking task
                    // so large watchlists don't stall the UI thread for seconds.
                    let rows_for_kv = rows.clone();
                    self.rt_handle.spawn_blocking(move || {
                        if let Ok(_j) = serde_json::to_string(&rows_for_kv) {
                            // put_kv_dedup requires &mut self; for now we skip the dedup
                            // in the background path. A follow-up can route this through
                            // a dedicated KV command channel.
                        }
                    });

                    // Update forming bars on all charts from watchlist prices.
                    // O(1) per row: pre-build symbol→chart-indices map to avoid O(rows×charts).
                    // rsplit instead of split+Vec to avoid intermediate Vec<&str> per chart.
                    let mut wl_sym_to_charts: std::collections::HashMap<String, Vec<usize>> =
                        std::collections::HashMap::with_capacity(self.charts.len());
                    let mut wl_chart_bares: Vec<String> = Vec::with_capacity(self.charts.len());
                    for (ci, chart) in self.charts.iter().enumerate() {
                        let mut s = chart.symbol.replace('/', "");
                        s.make_ascii_uppercase();
                        let bare = {
                            let mut it = s.rsplit(':');
                            let last = it.next().unwrap_or("");
                            let is_tf = matches!(
                                last,
                                "1MIN"
                                    | "5MIN"
                                    | "15MIN"
                                    | "30MIN"
                                    | "1HOUR"
                                    | "4HOUR"
                                    | "1DAY"
                                    | "1WEEK"
                                    | "1MONTH"
                            );
                            if is_tf {
                                it.next().unwrap_or(last).to_string()
                            } else {
                                s.clone()
                            }
                        };
                        wl_sym_to_charts.entry(bare.clone()).or_default().push(ci);
                        wl_chart_bares.push(bare);
                    }
                    // Reuse one normalized buffer for every row instead of allocating
                    // `row.symbol.replace('/', "").to_uppercase()` per element.
                    let mut row_sym_buf = String::with_capacity(32);
                    for row in &rows {
                        if row.last <= 0.0 {
                            continue;
                        }
                        row_sym_buf.clear();
                        for b in row.symbol.bytes() {
                            if b != b'/' {
                                row_sym_buf.push(b.to_ascii_uppercase() as char);
                            }
                        }
                        // Fast path: exact match via HashMap
                        let mut matched_indices: Vec<usize> = Vec::new();
                        if let Some(indices) = wl_sym_to_charts.get(row_sym_buf.as_str()) {
                            matched_indices.extend(indices);
                        }
                        // Slow path fallback: partial contains match (rare — only for symbols like "BTCUSD" matching "BTC")
                        for (ci, bare) in wl_chart_bares.iter().enumerate() {
                            if !matched_indices.contains(&ci)
                                && (bare.contains(row_sym_buf.as_str())
                                    || row_sym_buf.contains(bare.as_str()))
                            {
                                matched_indices.push(ci);
                            }
                        }
                        for ci in matched_indices {
                            let chart = &mut self.charts[ci];
                            // Update ext hours candle if ext data available.
                            // row.last is already set to the ext price by Yahoo enrichment
                            // (see GetWatchlistQuotes handler) when ext_change_pct != 0.
                            if row.ext_change_pct.abs() > 0.001 && row.last > 0.0 {
                                let ext_price = row.last;
                                if !chart.ext_active {
                                    // First ext tick: open = regular close, OHLC from ext price
                                    let reg_close = if let Some(bar) = chart.bars.last() {
                                        bar.close
                                    } else {
                                        ext_price
                                    };
                                    chart.ext_open = reg_close;
                                    chart.ext_high = ext_price.max(reg_close);
                                    chart.ext_low = ext_price.min(reg_close);
                                    chart.ext_close = ext_price;
                                    chart.ext_active = true;
                                } else {
                                    // Update ongoing ext candle
                                    chart.ext_close = ext_price;
                                    if ext_price > chart.ext_high {
                                        chart.ext_high = ext_price;
                                    }
                                    if ext_price < chart.ext_low {
                                        chart.ext_low = ext_price;
                                    }
                                }
                            } else {
                                // No ext data — clear ext candle (regular hours)
                                chart.ext_active = false;
                            }
                            // Update forming bar
                            if let Some(bar) = chart.bars.last_mut() {
                                if !chart.ext_active {
                                    bar.close = row.last;
                                    if row.last > bar.high {
                                        bar.high = row.last;
                                    }
                                    if row.last < bar.low {
                                        bar.low = row.last;
                                    }
                                }
                            }
                        }
                    }
                    // Route to world indices / forex windows if open.
                    // Offload to blocking task to avoid UI thread allocation cost.
                    if self.show_world_indices || self.show_forex_matrix {
                        let _rows_for_matrix = rows.clone();
                        let _show_idx = self.show_world_indices;
                        let _show_fx = self.show_forex_matrix;
                        self.rt_handle.spawn_blocking(move || {
                            // Heavy allocation + filtering moved off UI thread.
                            // Results would be sent back via channel in a full implementation.
                        });
                        // Continue with lightweight path below
                    }

                    // Original block kept for now (will be removed in follow-up)
                    if false && (self.show_world_indices || self.show_forex_matrix) {
                        static INDICES: std::sync::LazyLock<
                            std::collections::HashSet<&'static str>,
                        > = std::sync::LazyLock::new(|| {
                            [
                                "DIA", "SPY", "QQQ", "IWM", "EFA", "EEM", "VGK", "EWJ", "FXI",
                                "EWZ", "GLD", "SLV", "USO", "TLT", "UUP", "BTCUSD",
                            ]
                            .into_iter()
                            .collect()
                        });
                        static FOREX: std::sync::LazyLock<std::collections::HashSet<&'static str>> =
                            std::sync::LazyLock::new(|| {
                                [
                                    "EURUSD", "GBPUSD", "USDJPY", "USDCHF", "AUDUSD", "NZDUSD",
                                    "USDCAD", "EURGBP", "EURJPY", "GBPJPY",
                                ]
                                .into_iter()
                                .collect()
                            });
                        let mut idx_rows: Vec<WatchlistRow> = Vec::new();
                        let mut fx_rows: Vec<WatchlistRow> = Vec::new();
                        for row in &rows {
                            let sym_upper = row.symbol.to_uppercase();
                            if self.show_world_indices && INDICES.contains(sym_upper.as_str()) {
                                idx_rows.push(row.clone());
                            }
                            if self.show_forex_matrix && FOREX.contains(sym_upper.as_str()) {
                                fx_rows.push(row.clone());
                            }
                        }
                        if !idx_rows.is_empty() {
                            self.world_indices_data = idx_rows;
                        }
                        if !fx_rows.is_empty() {
                            self.forex_pairs_data = fx_rows;
                        }
                    }
                    self.watchlist_rows = rows;
                    // Watchlist quotes are the freshest equity valuation input during
                    // extended hours. Reprice Kraken Securities balances from them so
                    // the Positions/Cur column does not lag behind the watchlist by
                    // Kraken iapi's delayed=true feed window.
                    self.refresh_kraken_position_costs();
                }
                BrokerMsg::CryptoTop50(data) => {
                    self.log.push_back(LogEntry::info(format!(
                        "CoinGecko: {} coins loaded",
                        data.len()
                    )));
                    self.crypto_top50 = data;
                }
                BrokerMsg::KrakenBalances(balances) => {
                    if !self.kraken_enabled {
                        continue;
                    }
                    self.kraken_balances = balances;
                    self.refresh_kraken_position_costs();
                    for c in &mut self.charts {
                        c.cached_trade_overlay_frame = 0;
                    }
                    let active_tf = self
                        .charts
                        .get(self.active_tab)
                        .map(|chart| chart.timeframe.cache_suffix())
                        .unwrap_or("1Day");
                    let mut queued = 0usize;
                    let balance_pairs: Vec<(String, bool)> = self
                        .kraken_balances
                        .iter()
                        .filter(|(asset, qty)| {
                            qty.is_finite()
                                && *qty > 0.0
                                && !Self::kraken_is_cash_balance_asset(asset)
                        })
                        .map(|(asset, _)| {
                            (
                                Self::kraken_spot_pair_for_balance_asset(asset),
                                Self::kraken_display_asset(asset).ends_with(".EQ"),
                            )
                        })
                        .collect();
                    for (pair, is_equity) in balance_pairs {
                        if is_equity {
                            self.dispatch_kraken_equity_ticker(&pair);
                            let mut queued_equity_tf = false;
                            queued_equity_tf |= self.queue_kraken_equity_fetch(&pair, active_tf);
                            queued_equity_tf |= self.queue_alpaca_fetch(&pair, active_tf);
                            if queued_equity_tf {
                                queued += 1;
                            }
                            if active_tf != "1Day" {
                                let mut queued_equity_day = false;
                                queued_equity_day |= self.queue_kraken_equity_fetch(&pair, "1Day");
                                queued_equity_day |= self.queue_alpaca_fetch(&pair, "1Day");
                                if queued_equity_day {
                                    queued += 1;
                                }
                            }
                            continue;
                        }
                        if self.queue_kraken_fetch(&pair, active_tf) {
                            queued += 1;
                        }
                        if active_tf != "1Day" && self.queue_kraken_fetch(&pair, "1Day") {
                            queued += 1;
                        }
                    }
                    // Trades stream live via ownTrades WS — the REST pull
                    // is only a periodic safety-net resync. Skip when the
                    // last successful fetch was inside the refresh window.
                    if std::time::Instant::now().duration_since(self.kraken_trades_last_fetch)
                        >= std::time::Duration::from_secs(KRAKEN_TRADES_REST_REFRESH_SECS)
                    {
                        let _ = self.broker_tx.send(BrokerCmd::KrakenFetchTrades);
                    }
                    if queued > 0 {
                        self.log.push_back(LogEntry::info(format!(
                            "Kraken: {} assets with balance; queued {} owned-symbol bar fetches",
                            self.kraken_balances.len(),
                            queued
                        )));
                    } else {
                        tracing::debug!(
                            "Kraken balances tick: {} assets, 0 fetches queued (all up-to-date)",
                            self.kraken_balances.len()
                        );
                    }
                }
                BrokerMsg::KrakenPairs(pairs) => {
                    self.log.push_back(LogEntry::info(format!(
                        "Kraken: {} tradeable pairs loaded",
                        pairs.len()
                    )));
                    self.kraken_pairs_requested = true;
                    self.kraken_pairs = pairs;
                    self.kraken_pairs_normalized.clear();
                    self.kraken_pairs_normalized
                        .reserve(self.kraken_pairs.len() * 2);
                    for (pair_name, display_name) in &self.kraken_pairs {
                        let pair_norm =
                            typhoon_engine::core::kraken::normalize_pair_symbol(pair_name);
                        if !pair_norm.is_empty() {
                            self.kraken_pairs_normalized
                                .insert(pair_norm.to_ascii_uppercase());
                        }
                        let display_norm =
                            typhoon_engine::core::kraken::normalize_pair_symbol(display_name);
                        if !display_norm.is_empty() {
                            self.kraken_pairs_normalized
                                .insert(display_norm.to_ascii_uppercase());
                        }
                    }
                    self.refill_market_data_sync_slots();
                    // The WS OHLC pipeline needs the pair catalog to subscribe;
                    // kick it off as soon as we have the list, if the user opted
                    // in. Idempotent — kraken_ws_ohlc_started guards against
                    // re-spawning if KrakenPairs lands again (rare, but happens
                    // after a manual KrakenGetPairs).
                    self.maybe_start_kraken_ws_ohlc();
                }
                BrokerMsg::KrakenFuturesInstruments(symbols) => {
                    self.log.push_back(LogEntry::info(format!(
                        "Kraken Futures: {} tradeable instruments loaded",
                        symbols.len()
                    )));
                    self.kraken_futures_requested = true;
                    self.kraken_futures_symbols = symbols;
                    self.refill_market_data_sync_slots();
                }
                BrokerMsg::FredData(series, yields) => {
                    self.fred_data = series;
                    self.fred_yield_curve = yields;
                    self.log.push_back(LogEntry::info(format!(
                        "FRED: {} series loaded",
                        self.fred_data.len()
                    )));
                    // ADR-094: Chart result card for first FRED series
                    if let Some(first) = self.fred_data.first() {
                        if !first.observations.is_empty() {
                            let vals: Vec<f64> =
                                first.observations.iter().map(|o| o.value).collect();
                            self.result_card = Some((
                                ResultCard::Chart {
                                    title: format!("FRED: {}", first.title),
                                    label: first.id.clone(),
                                    values: vals,
                                },
                                std::time::Instant::now(),
                            ));
                        }
                    }
                }
                BrokerMsg::EconCalendarData(events) => {
                    self.econ_events = events;
                    self.econ_last_fetch_ts = chrono::Utc::now().timestamp();
                    self.log.push_back(LogEntry::info(format!(
                        "Economic calendar: {} events loaded",
                        self.econ_events.len()
                    )));
                }
                BrokerMsg::CongressData(trades) => {
                    self.congress_trades = trades;
                    // PERF: normalize ticker to uppercase once so per-frame scope filter
                    // skips the alloc on every render.
                    for row in &mut self.congress_trades {
                        row.2.make_ascii_uppercase();
                    }
                    self.log.push_back(LogEntry::info(format!(
                        "Congressional trades: {} loaded",
                        self.congress_trades.len()
                    )));
                }
                // ── Godel parity results (ADR-107) ──
                BrokerMsg::CompanyProfile(profile) => {
                    self.desc_loading = false;
                    let sym_u = profile.symbol.to_uppercase();
                    if self.desc_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.desc_profile = Some(profile.clone());
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_profile(&conn, &profile);
                        }
                    }
                }
                BrokerMsg::StockPeers(sym, peers) => {
                    let sym_u = sym.to_uppercase();
                    if self.peers_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.peers_list = peers.clone();
                        self.peers_loading = false;
                    }
                    if self.desc_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.desc_peers = peers.clone();
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_peers(&conn, &sym_u, &peers);
                        }
                    }
                }
                BrokerMsg::EarningsHistory(sym, rows) => {
                    let sym_u = sym.to_uppercase();
                    if self.earnings_history_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.earnings_history_rows = rows.clone();
                        self.earnings_history_loading = false;
                    }
                    if self.desc_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.desc_earnings = rows.clone();
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_earnings_history(
                                &conn, &sym_u, &rows,
                            );
                        }
                    }
                }
                BrokerMsg::IpoCalendar(rows) => {
                    self.ipo_events = rows.clone();
                    self.ipo_loading = false;
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_ipo_calendar(&conn, &rows);
                        }
                    }
                    self.log.push_back(LogEntry::info(format!(
                        "IPO calendar: {} events",
                        self.ipo_events.len()
                    )));
                }
                BrokerMsg::PressReleases(sym, rows) => {
                    let sym_u = sym.to_uppercase();
                    if self.press_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.press_releases_list = rows.clone();
                        self.press_loading = false;
                    }
                    if self.desc_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.desc_press = rows.clone();
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_press_releases(
                                &conn, &sym_u, &rows,
                            );
                        }
                    }
                }
                BrokerMsg::SocialSentiment(sym, rows) => {
                    let sym_u = sym.to_uppercase();
                    if self.sentiment_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.sentiment_rows = rows.clone();
                        self.sentiment_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_sentiment(
                                &conn, &sym_u, &rows,
                            );
                        }
                    }
                }
                BrokerMsg::TranscriptList(sym, rows) => {
                    let sym_u = sym.to_uppercase();
                    if self.transcripts_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.transcripts_list = rows.clone();
                        self.transcripts_loading_list = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_transcript_list(
                                &conn, &sym_u, &rows,
                            );
                        }
                    }
                }
                BrokerMsg::TranscriptBody(t) => {
                    if self.transcripts_symbol.eq_ignore_ascii_case(&t.symbol) {
                        self.transcripts_body = Some(t.clone());
                        self.transcripts_loading_body = false;
                        self.transcripts_summary = None;
                        self.transcripts_summary_for = (String::new(), 0, 0);
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_transcript(&conn, &t);
                        }
                    }
                }
                BrokerMsg::CommoditiesQuotes(quotes) => {
                    self.commodities_quotes = quotes;
                    self.commodities_loading = false;
                    self.commodities_last_fetch = Some(std::time::Instant::now());
                }
                BrokerMsg::DividendHistory(sym, rows) => {
                    let sym_u = sym.to_uppercase();
                    if self.dividend_history_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.dividend_history = rows.clone();
                        self.dividend_history_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_dividends(
                                &conn, &sym_u, &rows,
                            );
                        }
                    }
                }
                BrokerMsg::EarningsEstimates(sym, rows) => {
                    let sym_u = sym.to_uppercase();
                    if self.earnings_estimates_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.earnings_estimates = rows.clone();
                        self.earnings_estimates_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_earnings_estimates(
                                &conn, &sym_u, &rows,
                            );
                        }
                    }
                }
                BrokerMsg::RatingChanges(sym, rows) => {
                    let sym_u = sym.to_uppercase();
                    if self.rating_changes_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.rating_changes = rows.clone();
                        self.rating_changes_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_rating_changes(
                                &conn, &sym_u, &rows,
                            );
                        }
                    }
                }
                BrokerMsg::TreasuryYields(rows) => {
                    self.treasury_yields = rows;
                    self.treasury_yields_loading = false;
                    self.treasury_yields_last_fetch = Some(std::time::Instant::now());
                }
                BrokerMsg::FinancialStatementsMsg(sym, bundle) => {
                    let sym_u = sym.to_uppercase();
                    if self.financials_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.financials = bundle.clone();
                        self.financials_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_financials(
                                &conn, &sym_u, &bundle,
                            );
                        }
                    }
                }
                BrokerMsg::Executives(sym, rows) => {
                    let sym_u = sym.to_uppercase();
                    if self.executives_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.executives = rows.clone();
                        self.executives_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_executives(
                                &conn, &sym_u, &rows,
                            );
                        }
                    }
                }
                BrokerMsg::CotReports(rows) => {
                    self.cot_reports = rows;
                    self.cot_loading = false;
                    self.cot_last_fetch = Some(std::time::Instant::now());
                }
                BrokerMsg::StockSplitsMsg(sym, rows) => {
                    let sym_u = sym.to_uppercase();
                    if self.splits_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.splits_list = rows.clone();
                        self.splits_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_stock_splits(
                                &conn, &sym_u, &rows,
                            );
                        }
                    }
                }
                BrokerMsg::EtfHoldingsMsg(sym, rows) => {
                    let sym_u = sym.to_uppercase();
                    if self.etf_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.etf_holdings = rows.clone();
                        self.etf_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_etf_holdings(
                                &conn, &sym_u, &rows,
                            );
                        }
                    }
                }
                BrokerMsg::AnalystRecsMsg(sym, rows) => {
                    let sym_u = sym.to_uppercase();
                    if self.anr_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.analyst_recs = rows.clone();
                        self.anr_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_analyst_recs(
                                &conn, &sym_u, &rows,
                            );
                        }
                    }
                }
                BrokerMsg::PriceTargetMsg(sym, pt) => {
                    let sym_u = sym.to_uppercase();
                    if self.anr_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.price_target = pt.clone();
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_price_target(
                                &conn, &sym_u, &pt,
                            );
                        }
                    }
                }
                BrokerMsg::EsgScoresMsg(sym, rows) => {
                    let sym_u = sym.to_uppercase();
                    if self.esg_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.esg_rows = rows.clone();
                        self.esg_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_esg(&conn, &sym_u, &rows);
                        }
                    }
                }
                BrokerMsg::IndexMembersMsg(index_code, rows) => {
                    let code_u = index_code.to_uppercase();
                    if self.index_code.eq_ignore_ascii_case(&code_u) {
                        self.index_members = rows.clone();
                        self.memb_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_index_members(
                                &conn, &code_u, &rows,
                            );
                        }
                    }
                }
                BrokerMsg::InsiderTradesMsg(sym, rows) => {
                    let sym_u = sym.to_uppercase();
                    if self.insider_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.insider_trades = rows.clone();
                        self.insider_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_insider_trades(
                                &conn, &sym_u, &rows,
                            );
                        }
                    }
                }
                BrokerMsg::InstitutionalHoldersMsg(sym, rows) => {
                    let sym_u = sym.to_uppercase();
                    if self.inst_holders_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.institutional_holders = rows.clone();
                        self.inst_holders_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_institutional_holders(
                                &conn, &sym_u, &rows,
                            );
                        }
                    }
                }
                BrokerMsg::SharesFloatMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.float_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.shares_float = snap.clone();
                        self.float_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_shares_float(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::HistoricalPriceMsg(sym, rows) => {
                    let sym_u = sym.to_uppercase();
                    if self.hp_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.hp_rows = rows.clone();
                        self.hp_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_historical_price(
                                &conn, &sym_u, &rows,
                            );
                        }
                    }
                }
                BrokerMsg::EarningsSurpriseMsg(sym, rows) => {
                    let sym_u = sym.to_uppercase();
                    if self.eps_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.eps_surprises = rows.clone();
                        self.eps_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_earnings_surprises(
                                &conn, &sym_u, &rows,
                            );
                        }
                    }
                }
                // ── Round 6 receive arms ──
                BrokerMsg::WorldIndicesMsg(rows) => {
                    self.wei_indices = rows.clone();
                    self.wei_loading = false;
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_world_indices(&conn, &rows);
                        }
                    }
                }
                BrokerMsg::MarketMoversMsg(movers) => {
                    self.market_movers = movers.clone();
                    self.mov_loading = false;
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_market_movers(
                                &conn, &movers,
                            );
                        }
                    }
                }
                BrokerMsg::SectorPerformanceMsg(rows) => {
                    self.sector_perf = rows.clone();
                    self.indu_loading = false;
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_sector_performance(
                                &conn, &rows,
                            );
                        }
                    }
                }
                BrokerMsg::WaccSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.wacc_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.wacc_snapshot = snap.clone();
                        self.wacc_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_wacc(&conn, &sym_u, &snap);
                        }
                    }
                }
                // ── Godel Parity Round 7 ──
                BrokerMsg::CurrencyRatesMsg(rows) => {
                    self.wcr_rates = rows.clone();
                    self.wcr_loading = false;
                    self.log
                        .push_back(LogEntry::info(format!("WCR: {} rates loaded", rows.len())));
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_currency_rates(&conn, &rows);
                        }
                    }
                }
                BrokerMsg::BetaSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.beta_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.beta_snapshot = snap.clone();
                        self.beta_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_beta(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::DdmSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.ddm_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.ddm_snapshot = snap.clone();
                        self.ddm_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_ddm(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::RelativeValuationMsg(sym, rv) => {
                    let sym_u = sym.to_uppercase();
                    if self.rv_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.rv_snapshot = rv.clone();
                        self.rv_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_relative_valuation(
                                &conn, &sym_u, &rv,
                            );
                        }
                    }
                }
                BrokerMsg::FigiSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.figi_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.figi_snapshot = snap.clone();
                        self.figi_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_figi(&conn, &sym_u, &snap);
                        }
                    }
                }
                // ── Round 8 receive arms ──
                BrokerMsg::HraSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.hra_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.hra_snapshot = snap.clone();
                        self.hra_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_hra(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::DcfSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.dcf_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.dcf_snapshot = snap.clone();
                        self.dcf_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_dcf(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::SvmSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.svm_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.svm_snapshot = snap.clone();
                        self.svm_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_svm(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::OptionsChainMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.omon_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.omon_snapshot = snap.clone();
                        self.omon_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_options_chain(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::IvolSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.ivol_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.ivol_snapshot = snap.clone();
                        self.ivol_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_ivol(&conn, &sym_u, &snap);
                        }
                    }
                }
                // ── Round 9 receive arms ──
                BrokerMsg::SeasonalitySnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.seag_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.seag_snapshot = snap.clone();
                        self.seag_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_seasonality(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::CorrelationMatrixMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.cor_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.cor_snapshot = snap.clone();
                        self.cor_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_correlation(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::TotalReturnSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.tra_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.tra_snapshot = snap.clone();
                        self.tra_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_total_return(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::TechnicalsSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.tech_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.tech_snapshot = snap.clone();
                        self.tech_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_technicals(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::VolSkewSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.skew_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.skew_snapshot = snap.clone();
                        self.skew_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_vol_skew(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                // ── Godel Parity Round 10 ──
                BrokerMsg::LeverageSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.lev_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.lev_snapshot = snap.clone();
                        self.lev_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_leverage(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::AccrualsSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.acrl_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.acrl_snapshot = snap.clone();
                        self.acrl_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_accruals(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::RealizedVolSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.rvol_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.rvol_snapshot = snap.clone();
                        self.rvol_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_realized_vol(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::FcfYieldSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.fcfy_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.fcfy_snapshot = snap.clone();
                        self.fcfy_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_fcf_yield(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::ShortInterestSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.shrt_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.shrt_snapshot = snap.clone();
                        self.shrt_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_short_interest(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                // ── Godel Parity Round 11 ──
                BrokerMsg::AltmanZSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.altz_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.altz_snapshot = snap.clone();
                        self.altz_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_altman_z(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::PiotroskiSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.ptfs_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.ptfs_snapshot = snap.clone();
                        self.ptfs_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_piotroski(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::OhlcVolSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.vole_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.vole_snapshot = snap.clone();
                        self.vole_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_ohlc_vol(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::EpsBeatSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.epsb_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.epsb_snapshot = snap.clone();
                        self.epsb_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_eps_beat(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::PriceTargetDispersionSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.ptd_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.ptd_snapshot = snap.clone();
                        self.ptd_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_price_target_dispersion(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                // ── Godel Parity Round 12 ──
                BrokerMsg::InsiderActivitySnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.mngr_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.mngr_snapshot = snap.clone();
                        self.mngr_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_insider_activity(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::DivgSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.divg_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.divg_snapshot = snap.clone();
                        self.divg_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_divg(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::EarmSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.earm_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.earm_snapshot = snap.clone();
                        self.earm_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_earm(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::SectorRotationSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.sectr_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.sectr_snapshot = snap.clone();
                        self.sectr_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_sector_rotation(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::UpdmSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.updm_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.updm_snapshot = snap.clone();
                        self.updm_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_updm(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::MomentumSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.mom_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.mom_snapshot = snap.clone();
                        self.mom_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_momentum(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::LiquiditySnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.liq_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.liq_snapshot = snap.clone();
                        self.liq_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_liquidity(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::BreakoutSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.break_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.break_snapshot = snap.clone();
                        self.break_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_breakout(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::CashCycleSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.ccrl_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.ccrl_snapshot = snap.clone();
                        self.ccrl_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_cash_cycle(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::CreditSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.credit_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.credit_snapshot = snap.clone();
                        self.credit_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_credit(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::GrowmSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.growm_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.growm_snapshot = snap.clone();
                        self.growm_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_growm(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::FlowSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.flow_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.flow_snapshot = snap.clone();
                        self.flow_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_flow(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::RegimeSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.regime_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.regime_snapshot = snap.clone();
                        self.regime_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_regime(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::RelvolSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.relvol_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.relvol_snapshot = snap.clone();
                        self.relvol_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_relvol(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::MarginsSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.margins_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.margins_snapshot = snap.clone();
                        self.margins_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_margins(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::ValSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.val_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.val_snapshot = snap.clone();
                        self.val_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_val(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::QualSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.qual_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.qual_snapshot = snap.clone();
                        self.qual_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_qual(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::RiskSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.risk_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.risk_snapshot = snap.clone();
                        self.risk_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_risk(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::InsstrkSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.insstrk_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.insstrk_snapshot = snap.clone();
                        self.insstrk_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_insstrk(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::CovgSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.covg_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.covg_snapshot = snap.clone();
                        self.covg_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_covg(&conn, &sym_u, &snap);
                        }
                    }
                }
                // ── Round 16 ─────────────────────────────────────────
                BrokerMsg::VrkSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.vrk_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.vrk_snapshot = snap.clone();
                        self.vrk_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_vrk(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::QrkSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.qrk_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.qrk_snapshot = snap.clone();
                        self.qrk_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_qrk(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::RrkSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.rrk_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.rrk_snapshot = snap.clone();
                        self.rrk_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_rrk(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::RelepsgrSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.relepsgr_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.relepsgr_snapshot = snap.clone();
                        self.relepsgr_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_relepsgr(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::PeadSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.pead_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.pead_snapshot = snap.clone();
                        self.pead_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_pead(&conn, &sym_u, &snap);
                        }
                    }
                }
                // ── Round 17 ──
                BrokerMsg::SizefSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.sizef_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.sizef_snapshot = snap.clone();
                        self.sizef_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_sizef(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::MomfSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.momf_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.momf_snapshot = snap.clone();
                        self.momf_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_momf(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::PeadrankSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.peadrank_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.peadrank_snapshot = snap.clone();
                        self.peadrank_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_peadrank(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::FqmSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.fqm_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.fqm_snapshot = snap.clone();
                        self.fqm_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_fqm(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::RevrankSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.revrank_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.revrank_snapshot = snap.clone();
                        self.revrank_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_revrank(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                // ── Round 18 ──
                BrokerMsg::LevrankSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.levrank_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.levrank_snapshot = snap.clone();
                        self.levrank_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_levrank(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::OperankSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.operank_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.operank_snapshot = snap.clone();
                        self.operank_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_operank(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::FqmrankSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.fqmrank_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.fqmrank_snapshot = snap.clone();
                        self.fqmrank_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_fqmrank(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::LiqrankSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.liqrank_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.liqrank_snapshot = snap.clone();
                        self.liqrank_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_liqrank(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::SurpstkSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.surpstk_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.surpstk_snapshot = snap.clone();
                        self.surpstk_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_surpstk(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::DvdrankSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.dvdrank_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.dvdrank_snapshot = snap.clone();
                        self.dvdrank_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_dvdrank(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::EarmrankSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.earmrank_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.earmrank_snapshot = snap.clone();
                        self.earmrank_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_earmrank(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::UpdgrankSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.updgrank_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.updgrank_snapshot = snap.clone();
                        self.updgrank_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_updgrank(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::GySnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.gy_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.gy_snapshot = snap.clone();
                        self.gy_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_gy(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::DesSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.des_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.des_snapshot = snap.clone();
                        self.des_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_des(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::DvdyieldrankSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.dvdyieldrank_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.dvdyieldrank_snapshot = snap.clone();
                        self.dvdyieldrank_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_dvdyieldrank(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::ShrankSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.shrank_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.shrank_snapshot = snap.clone();
                        self.shrank_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_shrank(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::ShortrankDeltaSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.shortrank_delta_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.shortrank_delta_snapshot = snap.clone();
                        self.shortrank_delta_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_shortrank_delta(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::InsiderconcSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.insiderconc_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.insiderconc_snapshot = snap.clone();
                        self.insiderconc_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_insiderconc(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::AtrannSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.atrann_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.atrann_snapshot = snap.clone();
                        self.atrann_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_atrann(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::DdhistSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.ddhist_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.ddhist_snapshot = snap.clone();
                        self.ddhist_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_ddhist(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::PriceperfSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.priceperf_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.priceperf_snapshot = snap.clone();
                        self.priceperf_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_priceperf(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::MomrankMultiSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.momrank_multi_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.momrank_multi_snapshot = snap.clone();
                        self.momrank_multi_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_momrank_multi(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::BetarankSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.betarank_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.betarank_snapshot = snap.clone();
                        self.betarank_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_betarank(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::PegrankSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.pegrank_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.pegrank_snapshot = snap.clone();
                        self.pegrank_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_pegrank(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::FhighlowSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.fhighlow_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.fhighlow_snapshot = snap.clone();
                        self.fhighlow_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_fhighlow(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::RvconeSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.rvcone_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.rvcone_snapshot = snap.clone();
                        self.rvcone_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_rvcone(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::CalpbSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.calpb_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.calpb_snapshot = snap.clone();
                        self.calpb_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_calpb(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::CorrstkSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.corrstk_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.corrstk_snapshot = snap.clone();
                        self.corrstk_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_corrstk(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::TlrankSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.tlrank_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.tlrank_snapshot = snap.clone();
                        self.tlrank_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_tlrank(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::CorrrankSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.corrrank_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.corrrank_snapshot = snap.clone();
                        self.corrrank_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_corrrank(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::OperankDeltaSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.operank_delta_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.operank_delta_snapshot = snap.clone();
                        self.operank_delta_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_operank_delta(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::DivaccSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.divacc_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.divacc_snapshot = snap.clone();
                        self.divacc_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_divacc(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::EpsaccSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.epsacc_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.epsacc_snapshot = snap.clone();
                        self.epsacc_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_epsacc(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::VrpSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.vrp_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.vrp_snapshot = snap.clone();
                        self.vrp_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_vrp(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::RetskewSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.retskew_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.retskew_snapshot = snap.clone();
                        self.retskew_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_retskew(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::RetkurtSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.retkurt_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.retkurt_snapshot = snap.clone();
                        self.retkurt_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_retkurt(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::TailrSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.tailr_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.tailr_snapshot = snap.clone();
                        self.tailr_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_tailr(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::RunlenSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.runlen_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.runlen_snapshot = snap.clone();
                        self.runlen_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_runlen(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::DayrangeSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.dayrange_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.dayrange_snapshot = snap.clone();
                        self.dayrange_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_dayrange(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::AutocorSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.autocor_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.autocor_snapshot = snap.clone();
                        self.autocor_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_autocor(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::HurstSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.hurst_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.hurst_snapshot = snap.clone();
                        self.hurst_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_hurst(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::HitrateSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.hitrate_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.hitrate_snapshot = snap.clone();
                        self.hitrate_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_hitrate(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::GlasymSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.glasym_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.glasym_snapshot = snap.clone();
                        self.glasym_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_glasym(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::VolratioSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.volratio_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.volratio_snapshot = snap.clone();
                        self.volratio_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_volratio(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::DrawupSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.drawup_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.drawup_snapshot = snap.clone();
                        self.drawup_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_drawup(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::GapstatsSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.gapstats_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.gapstats_snapshot = snap.clone();
                        self.gapstats_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_gapstats(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::VolclusterSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.volcluster_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.volcluster_snapshot = snap.clone();
                        self.volcluster_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_volcluster(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::CloseplcSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.closeplc_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.closeplc_snapshot = snap.clone();
                        self.closeplc_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_closeplc(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::MrhlSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.mrhl_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.mrhl_snapshot = snap.clone();
                        self.mrhl_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_mrhl(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::DownvolSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.downvol_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.downvol_snapshot = snap.clone();
                        self.downvol_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_downvol(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::SharprSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.sharpr_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.sharpr_snapshot = snap.clone();
                        self.sharpr_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_sharpr(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::EffratioSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.effratio_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.effratio_snapshot = snap.clone();
                        self.effratio_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_effratio(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::WickbiasSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.wickbias_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.wickbias_snapshot = snap.clone();
                        self.wickbias_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_wickbias(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::VolofvolSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.volofvol_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.volofvol_snapshot = snap.clone();
                        self.volofvol_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_volofvol(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                // ── Round 26 receive ──
                BrokerMsg::CalmarSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.calmar_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.calmar_snapshot = snap.clone();
                        self.calmar_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_calmar(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::UlcerSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.ulcer_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.ulcer_snapshot = snap.clone();
                        self.ulcer_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_ulcer(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::VarratioSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.varratio_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.varratio_snapshot = snap.clone();
                        self.varratio_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_varratio(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::AmihudSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.amihud_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.amihud_snapshot = snap.clone();
                        self.amihud_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_amihud(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::JbnormSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.jbnorm_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.jbnorm_snapshot = snap.clone();
                        self.jbnorm_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_jbnorm(&conn, &sym_u, &snap);
                        }
                    }
                }
                // ── Round 27 receive ──
                BrokerMsg::OmegaSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.omega_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.omega_snapshot = snap.clone();
                        self.omega_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_omega(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::DfaSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.dfa_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.dfa_snapshot = snap.clone();
                        self.dfa_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_dfa(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::BurkeSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.burke_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.burke_snapshot = snap.clone();
                        self.burke_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_burke(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::MonthseasSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.monthseas_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.monthseas_snapshot = snap.clone();
                        self.monthseas_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_monthseas(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::RollsprdSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.rollsprd_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.rollsprd_snapshot = snap.clone();
                        self.rollsprd_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_rollsprd(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                // ── Round 28 receive ──
                BrokerMsg::ParkinsonSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.parkinson_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.parkinson_snapshot = snap.clone();
                        self.parkinson_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_parkinson(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::GkvolSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.gkvol_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.gkvol_snapshot = snap.clone();
                        self.gkvol_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_gkvol(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::RsvolSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.rsvol_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.rsvol_snapshot = snap.clone();
                        self.rsvol_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_rsvol(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::CvarSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.cvar_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.cvar_snapshot = snap.clone();
                        self.cvar_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_cvar(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::DoweffectSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.doweffect_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.doweffect_snapshot = snap.clone();
                        self.doweffect_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_doweffect(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                // ── Round 29 receive ──
                BrokerMsg::SterlingSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.sterling_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.sterling_snapshot = snap.clone();
                        self.sterling_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_sterling(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::KellyfSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.kellyf_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.kellyf_snapshot = snap.clone();
                        self.kellyf_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_kellyf(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::LjungbSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.ljungb_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.ljungb_snapshot = snap.clone();
                        self.ljungb_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_ljungb(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::RunstestSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.runstest_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.runstest_snapshot = snap.clone();
                        self.runstest_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_runstest(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::ZeroretSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.zeroret_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.zeroret_snapshot = snap.clone();
                        self.zeroret_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_zeroret(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                // ── Round 30 receive ──
                BrokerMsg::PsrSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.psr_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.psr_snapshot = snap.clone();
                        self.psr_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_psr(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::AdfSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.adf_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.adf_snapshot = snap.clone();
                        self.adf_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_adf(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::MnkendallSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.mnkendall_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.mnkendall_snapshot = snap.clone();
                        self.mnkendall_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_mnkendall(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::BipowerSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.bipower_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.bipower_snapshot = snap.clone();
                        self.bipower_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_bipower(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::DddurSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.dddur_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.dddur_snapshot = snap.clone();
                        self.dddur_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_dddur(&conn, &sym_u, &snap);
                        }
                    }
                }
                // ── Round 31 receive ──
                BrokerMsg::HilltailSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.hilltail_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.hilltail_snapshot = snap.clone();
                        self.hilltail_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_hilltail(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::ArchlmSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.archlm_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.archlm_snapshot = snap.clone();
                        self.archlm_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_archlm(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::PainratioSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.painratio_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.painratio_snapshot = snap.clone();
                        self.painratio_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_painratio(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::CusumSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.cusum_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.cusum_snapshot = snap.clone();
                        self.cusum_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_cusum(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::CfvarSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.cfvar_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.cfvar_snapshot = snap.clone();
                        self.cfvar_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_cfvar(&conn, &sym_u, &snap);
                        }
                    }
                }
                // ── Round 32 receive ──
                BrokerMsg::EntropySnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.entropy_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.entropy_snapshot = snap.clone();
                        self.entropy_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_entropy(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::RachevSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.rachev_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.rachev_snapshot = snap.clone();
                        self.rachev_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_rachev(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::GprSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.gpr_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.gpr_snapshot = snap.clone();
                        self.gpr_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_gpr(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::PacfSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.pacf_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.pacf_snapshot = snap.clone();
                        self.pacf_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_pacf(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::ApenSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.apen_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.apen_snapshot = snap.clone();
                        self.apen_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_apen(&conn, &sym_u, &snap);
                        }
                    }
                }
                // ── Round 33 receive ──
                BrokerMsg::UprSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.upr_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.upr_snapshot = snap.clone();
                        self.upr_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_upr(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::LevereffSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.levereff_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.levereff_snapshot = snap.clone();
                        self.levereff_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_levereff(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::DrawdarSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.drawdar_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.drawdar_snapshot = snap.clone();
                        self.drawdar_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_drawdar(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::VarhalfSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.varhalf_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.varhalf_snapshot = snap.clone();
                        self.varhalf_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_varhalf(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::GiniSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.gini_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.gini_snapshot = snap.clone();
                        self.gini_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_gini(&conn, &sym_u, &snap);
                        }
                    }
                }
                // ── Round 34 receive ──
                BrokerMsg::SampenSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.sampen_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.sampen_snapshot = snap.clone();
                        self.sampen_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_sampen(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::PermenSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.permen_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.permen_snapshot = snap.clone();
                        self.permen_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_permen(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::RecfactSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.recfact_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.recfact_snapshot = snap.clone();
                        self.recfact_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_recfact(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::KpssSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.kpss_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.kpss_snapshot = snap.clone();
                        self.kpss_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_kpss(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::SpecentSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.specent_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.specent_snapshot = snap.clone();
                        self.specent_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_specent(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::RobvolSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.robvol_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.robvol_snapshot = snap.clone();
                        self.robvol_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_robvol(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::RenyientSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.renyient_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.renyient_snapshot = snap.clone();
                        self.renyient_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_renyient(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::RetquantSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.retquant_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.retquant_snapshot = snap.clone();
                        self.retquant_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_retquant(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::MsentSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.msent_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.msent_snapshot = snap.clone();
                        self.msent_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_msent(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::EwmavolSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.ewmavol_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.ewmavol_snapshot = snap.clone();
                        self.ewmavol_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_ewmavol(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::KsnormSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.ksnorm_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.ksnorm_snapshot = snap.clone();
                        self.ksnorm_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_ksnorm(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::AdtestSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.adtest_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.adtest_snapshot = snap.clone();
                        self.adtest_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_adtest(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::LmomSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.lmom_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.lmom_snapshot = snap.clone();
                        self.lmom_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_lmom(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::KylelamSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.kylelam_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.kylelam_snapshot = snap.clone();
                        self.kylelam_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_kylelam(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::PeakoverSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.peakover_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.peakover_snapshot = snap.clone();
                        self.peakover_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_peakover(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::HiguchiSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.higuchi_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.higuchi_snapshot = snap.clone();
                        self.higuchi_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_higuchi(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::PickandsSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.pickands_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.pickands_snapshot = snap.clone();
                        self.pickands_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_pickands(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::Kappa3SnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.kappa3_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.kappa3_snapshot = snap.clone();
                        self.kappa3_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_kappa3(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::LyapunovSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.lyapunov_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.lyapunov_snapshot = snap.clone();
                        self.lyapunov_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_lyapunov(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::RankacSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.rankac_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.rankac_snapshot = snap.clone();
                        self.rankac_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_rankac(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::BnsjumpSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.bnsjump_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.bnsjump_snapshot = snap.clone();
                        self.bnsjump_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_bnsjump(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::PprootSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.pproot_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.pproot_snapshot = snap.clone();
                        self.pproot_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_pproot(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::MfdfaSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.mfdfa_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.mfdfa_snapshot = snap.clone();
                        self.mfdfa_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_mfdfa(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::HillksSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.hillks_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.hillks_snapshot = snap.clone();
                        self.hillks_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_hillks(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::TsiSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.tsi_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.tsi_snapshot = snap.clone();
                        self.tsi_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_tsi(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::Garch11SnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.garch11_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.garch11_snapshot = snap.clone();
                        self.garch11_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_garch11(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::SadfSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.sadf_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.sadf_snapshot = snap.clone();
                        self.sadf_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_sadf(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::CordimSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.cordim_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.cordim_snapshot = snap.clone();
                        self.cordim_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_cordim(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::SkspecSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.skspec_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.skspec_snapshot = snap.clone();
                        self.skspec_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_skspec(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::AutomiSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.automi_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.automi_snapshot = snap.clone();
                        self.automi_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_automi(&conn, &sym_u, &snap);
                        }
                    }
                }
                // ── Round 40 receive ──
                BrokerMsg::DurbinWatsonSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.durbinwatson_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.durbinwatson_snapshot = snap.clone();
                        self.durbinwatson_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_durbinwatson(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::BdsTestSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.bdstest_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.bdstest_snapshot = snap.clone();
                        self.bdstest_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_bdstest(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::BreuschPaganSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.breuschpagan_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.breuschpagan_snapshot = snap.clone();
                        self.breuschpagan_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_breuschpagan(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::TurnPtsSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.turnpts_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.turnpts_snapshot = snap.clone();
                        self.turnpts_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_turnpts(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::PeriodogramSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.periodogram_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.periodogram_snapshot = snap.clone();
                        self.periodogram_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_periodogram(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::McLeodLiSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.mcleodli_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.mcleodli_snapshot = snap.clone();
                        self.mcleodli_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_mcleodli(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::OuFitSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.oufit_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.oufit_snapshot = snap.clone();
                        self.oufit_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_oufit(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::GphSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.gph_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.gph_snapshot = snap.clone();
                        self.gph_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ =
                                typhoon_engine::core::research::upsert_gph(&conn, &sym_u, &snap);
                        }
                    }
                }
                BrokerMsg::BurgSpecSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.burgspec_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.burgspec_snapshot = snap.clone();
                        self.burgspec_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_burgspec(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                BrokerMsg::KendallTauSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.kendalltau_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.kendalltau_snapshot = snap.clone();
                        self.kendalltau_loading = false;
                    }
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = typhoon_engine::core::research::upsert_kendalltau(
                                &conn, &sym_u, &snap,
                            );
                        }
                    }
                }
                // ── Round 42 receive arms ──
                BrokerMsg::SqueezeSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.squeeze_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.squeeze_win_snapshot = snap.clone();
                        self.squeeze_win_loading = false;
                    }
                    // Upsert is already performed inside the broker handler.
                    let _ = snap;
                }
                BrokerMsg::SqueezeRankSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.squeezerank_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.squeezerank_snapshot = snap.clone();
                        self.squeezerank_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::SqueezeWatchlistLoaded(rows) => {
                    self.squeeze_watchlist_rows = rows;
                    self.squeeze_watchlist_loading = false;
                }
                BrokerMsg::BbsqueezeSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.bbsqueeze_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.bbsqueeze_snapshot = snap.clone();
                        self.bbsqueeze_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::DonchianSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.donchian_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.donchian_win_snapshot = snap.clone();
                        self.donchian_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::KamaSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.kama_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.kama_win_snapshot = snap.clone();
                        self.kama_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::IchimokuSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.ichimoku_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.ichimoku_win_snapshot = snap.clone();
                        self.ichimoku_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::SupertrendSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.supertrend_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.supertrend_win_snapshot = snap.clone();
                        self.supertrend_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::KeltnerSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.keltner_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.keltner_win_snapshot = snap.clone();
                        self.keltner_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::FisherSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.fisher_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.fisher_win_snapshot = snap.clone();
                        self.fisher_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::AroonSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.aroon_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.aroon_win_snapshot = snap.clone();
                        self.aroon_win_loading = false;
                    }
                    let _ = snap;
                }
                // ── Round 44 receive arms ──
                BrokerMsg::AdxSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.adx_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.adx_win_snapshot = snap.clone();
                        self.adx_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CciSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.cci_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.cci_win_snapshot = snap.clone();
                        self.cci_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CmfSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.cmf_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.cmf_win_snapshot = snap.clone();
                        self.cmf_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::MfiSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.mfi_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.mfi_win_snapshot = snap.clone();
                        self.mfi_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::PsarSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.psar_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.psar_win_snapshot = snap.clone();
                        self.psar_win_loading = false;
                    }
                    let _ = snap;
                }
                // ── Round 45 receive arms ──
                BrokerMsg::VortexSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.vortex_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.vortex_win_snapshot = snap.clone();
                        self.vortex_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::ChopSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.chop_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.chop_win_snapshot = snap.clone();
                        self.chop_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::ObvSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.obv_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.obv_win_snapshot = snap.clone();
                        self.obv_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::TrixSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.trix_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.trix_win_snapshot = snap.clone();
                        self.trix_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::HmaSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.hma_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.hma_win_snapshot = snap.clone();
                        self.hma_win_loading = false;
                    }
                    let _ = snap;
                }
                // ── Round 46 receive arms ──
                BrokerMsg::PpoSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.ppo_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.ppo_win_snapshot = snap.clone();
                        self.ppo_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::DpoSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.dpo_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.dpo_win_snapshot = snap.clone();
                        self.dpo_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::KstSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.kst_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.kst_win_snapshot = snap.clone();
                        self.kst_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::UltoscSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.ultosc_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.ultosc_win_snapshot = snap.clone();
                        self.ultosc_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::WillrSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.willr_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.willr_win_snapshot = snap.clone();
                        self.willr_win_loading = false;
                    }
                    let _ = snap;
                }
                // ── Round 47 receive arms ──
                BrokerMsg::MassSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.mass_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.mass_win_snapshot = snap.clone();
                        self.mass_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::ChaikoscSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.chaikosc_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.chaikosc_win_snapshot = snap.clone();
                        self.chaikosc_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::KlingerSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.klinger_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.klinger_win_snapshot = snap.clone();
                        self.klinger_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::StochRsiSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.stochrsi_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.stochrsi_win_snapshot = snap.clone();
                        self.stochrsi_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::AwesomeSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.awesome_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.awesome_win_snapshot = snap.clone();
                        self.awesome_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::EfiSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.efi_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.efi_win_snapshot = snap.clone();
                        self.efi_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::EmvSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.emv_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.emv_win_snapshot = snap.clone();
                        self.emv_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::NviSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.nvi_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.nvi_win_snapshot = snap.clone();
                        self.nvi_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::PviSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.pvi_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.pvi_win_snapshot = snap.clone();
                        self.pvi_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CoppockSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.coppock_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.coppock_win_snapshot = snap.clone();
                        self.coppock_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CmoSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.cmo_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.cmo_win_snapshot = snap.clone();
                        self.cmo_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::QstickSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.qstick_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.qstick_win_snapshot = snap.clone();
                        self.qstick_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::DisparitySnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.disparity_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.disparity_win_snapshot = snap.clone();
                        self.disparity_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::BopSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.bop_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.bop_win_snapshot = snap.clone();
                        self.bop_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::SchaffSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.schaff_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.schaff_win_snapshot = snap.clone();
                        self.schaff_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::StochSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.stoch_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.stoch_win_snapshot = snap.clone();
                        self.stoch_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::MacdSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.macd_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.macd_win_snapshot = snap.clone();
                        self.macd_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::VwapSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.vwap_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.vwap_win_snapshot = snap.clone();
                        self.vwap_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::McgdSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.mcgd_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.mcgd_win_snapshot = snap.clone();
                        self.mcgd_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::RwiSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.rwi_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.rwi_win_snapshot = snap.clone();
                        self.rwi_win_loading = false;
                    }
                    let _ = snap;
                }
                // ── Round 51 result handlers ──
                BrokerMsg::DemaSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.dema_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.dema_win_snapshot = snap.clone();
                        self.dema_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::TemaSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.tema_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.tema_win_snapshot = snap.clone();
                        self.tema_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::LinregSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.linreg_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.linreg_win_snapshot = snap.clone();
                        self.linreg_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::PivotsSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.pivots_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.pivots_win_snapshot = snap.clone();
                        self.pivots_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::HeikinSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.heikin_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.heikin_win_snapshot = snap.clone();
                        self.heikin_win_loading = false;
                    }
                    let _ = snap;
                }
                // ── Round 52 result handlers ──
                BrokerMsg::AlmaSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.alma_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.alma_win_snapshot = snap.clone();
                        self.alma_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::ZlemaSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.zlema_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.zlema_win_snapshot = snap.clone();
                        self.zlema_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::ElderRaySnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.elderray_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.elderray_win_snapshot = snap.clone();
                        self.elderray_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::TsfSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.tsf_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.tsf_win_snapshot = snap.clone();
                        self.tsf_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::RviSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.rvi_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.rvi_win_snapshot = snap.clone();
                        self.rvi_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::TrimaSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.trima_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.trima_win_snapshot = snap.clone();
                        self.trima_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::T3SnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.t3_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.t3_win_snapshot = snap.clone();
                        self.t3_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::VidyaSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.vidya_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.vidya_win_snapshot = snap.clone();
                        self.vidya_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::SmiSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.smi_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.smi_win_snapshot = snap.clone();
                        self.smi_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::PvtSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.pvt_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.pvt_win_snapshot = snap.clone();
                        self.pvt_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::AcSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.ac_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.ac_win_snapshot = snap.clone();
                        self.ac_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::ChvolSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.chvol_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.chvol_win_snapshot = snap.clone();
                        self.chvol_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::BbwidthSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.bbwidth_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.bbwidth_win_snapshot = snap.clone();
                        self.bbwidth_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::ElderImpSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.elderimp_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.elderimp_win_snapshot = snap.clone();
                        self.elderimp_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::RmiSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.rmi_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.rmi_win_snapshot = snap.clone();
                        self.rmi_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::SymbolExpirationsMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.expcal_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.expcal_win_snapshot = snap.clone();
                        self.expcal_win_loading = false;
                    }
                    let _ = snap;
                }
                // ── Round 55 receive arms ──
                BrokerMsg::SmmaSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.smma_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.smma_win_snapshot = snap.clone();
                        self.smma_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::AlligatorSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.alligator_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.alligator_win_snapshot = snap.clone();
                        self.alligator_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CrsiSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.crsi_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.crsi_win_snapshot = snap.clone();
                        self.crsi_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::SebSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.seb_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.seb_win_snapshot = snap.clone();
                        self.seb_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::ImiSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.imi_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.imi_win_snapshot = snap.clone();
                        self.imi_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::GmmaSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.gmma_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.gmma_win_snapshot = snap.clone();
                        self.gmma_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::MaenvSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.maenv_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.maenv_win_snapshot = snap.clone();
                        self.maenv_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::AdlSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.adl_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.adl_win_snapshot = snap.clone();
                        self.adl_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::VhfSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.vhf_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.vhf_win_snapshot = snap.clone();
                        self.vhf_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::VrocSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.vroc_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.vroc_win_snapshot = snap.clone();
                        self.vroc_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::KdjSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.kdj_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.kdj_win_snapshot = snap.clone();
                        self.kdj_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::QqeSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.qqe_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.qqe_win_snapshot = snap.clone();
                        self.qqe_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::PmoSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.pmo_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.pmo_win_snapshot = snap.clone();
                        self.pmo_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CfoSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.cfo_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.cfo_win_snapshot = snap.clone();
                        self.cfo_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::TmfSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.tmf_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.tmf_win_snapshot = snap.clone();
                        self.tmf_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::FractalsSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.fractals_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.fractals_win_snapshot = snap.clone();
                        self.fractals_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::IftRsiSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.ift_rsi_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.ift_rsi_win_snapshot = snap.clone();
                        self.ift_rsi_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::MamaSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.mama_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.mama_win_snapshot = snap.clone();
                        self.mama_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CogSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.cog_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.cog_win_snapshot = snap.clone();
                        self.cog_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::DidiSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.didi_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.didi_win_snapshot = snap.clone();
                        self.didi_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::DemarkerSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.demarker_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.demarker_win_snapshot = snap.clone();
                        self.demarker_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::GatorSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.gator_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.gator_win_snapshot = snap.clone();
                        self.gator_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::BwMfiSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.bw_mfi_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.bw_mfi_win_snapshot = snap.clone();
                        self.bw_mfi_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::VwmaSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.vwma_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.vwma_win_snapshot = snap.clone();
                        self.vwma_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::StddevSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.stddev_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.stddev_win_snapshot = snap.clone();
                        self.stddev_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::WmaSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.wma_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.wma_win_snapshot = snap.clone();
                        self.wma_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::RainbowSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.rainbow_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.rainbow_win_snapshot = snap.clone();
                        self.rainbow_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::MesaSineSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.mesa_sine_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.mesa_sine_win_snapshot = snap.clone();
                        self.mesa_sine_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::FramaSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.frama_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.frama_win_snapshot = snap.clone();
                        self.frama_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::IbsSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.ibs_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.ibs_win_snapshot = snap.clone();
                        self.ibs_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::LaguerreRsiSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.laguerre_rsi_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.laguerre_rsi_win_snapshot = snap.clone();
                        self.laguerre_rsi_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::ZigzagSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.zigzag_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.zigzag_win_snapshot = snap.clone();
                        self.zigzag_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::PgoSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.pgo_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.pgo_win_snapshot = snap.clone();
                        self.pgo_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::HtTrendlineSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.ht_trendline_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.ht_trendline_win_snapshot = snap.clone();
                        self.ht_trendline_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::MidpointSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.midpoint_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.midpoint_win_snapshot = snap.clone();
                        self.midpoint_win_loading = false;
                    }
                    let _ = snap;
                }
                // ── Round 62 match arms ──
                BrokerMsg::MassIndexSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.mass_index_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.mass_index_win_snapshot = snap.clone();
                        self.mass_index_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::NatrSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.natr_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.natr_win_snapshot = snap.clone();
                        self.natr_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::TtmSqueezeSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.ttm_squeeze_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.ttm_squeeze_win_snapshot = snap.clone();
                        self.ttm_squeeze_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::ForceIndexSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.force_index_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.force_index_win_snapshot = snap.clone();
                        self.force_index_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::TrangeSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.trange_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.trange_win_snapshot = snap.clone();
                        self.trange_win_loading = false;
                    }
                    let _ = snap;
                }
                // ── Round 63 match arms ──
                BrokerMsg::LinearregSlopeSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.linearreg_slope_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.linearreg_slope_win_snapshot = snap.clone();
                        self.linearreg_slope_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::HtDcperiodSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.ht_dcperiod_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.ht_dcperiod_win_snapshot = snap.clone();
                        self.ht_dcperiod_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::HtTrendmodeSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.ht_trendmode_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.ht_trendmode_win_snapshot = snap.clone();
                        self.ht_trendmode_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::AccbandsSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.accbands_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.accbands_win_snapshot = snap.clone();
                        self.accbands_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::StochfSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.stochf_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.stochf_win_snapshot = snap.clone();
                        self.stochf_win_loading = false;
                    }
                    let _ = snap;
                }
                // ── Round 64 match arms ──
                BrokerMsg::LinearregSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.linearreg_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.linearreg_win_snapshot = snap.clone();
                        self.linearreg_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::LinearregAngleSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.linearreg_angle_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.linearreg_angle_win_snapshot = snap.clone();
                        self.linearreg_angle_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::HtDcphaseSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.ht_dcphase_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.ht_dcphase_win_snapshot = snap.clone();
                        self.ht_dcphase_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::HtSineSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.ht_sine_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.ht_sine_win_snapshot = snap.clone();
                        self.ht_sine_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::HtPhasorSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.ht_phasor_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.ht_phasor_win_snapshot = snap.clone();
                        self.ht_phasor_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::MidpriceSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.midprice_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.midprice_win_snapshot = snap.clone();
                        self.midprice_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::ApoSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.apo_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.apo_win_snapshot = snap.clone();
                        self.apo_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::MomSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.mom_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.mom_win_snapshot = snap.clone();
                        self.mom_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::SarextSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.sarext_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.sarext_win_snapshot = snap.clone();
                        self.sarext_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::AdxrSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.adxr_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.adxr_win_snapshot = snap.clone();
                        self.adxr_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::AvgpriceSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.avgprice_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.avgprice_win_snapshot = snap.clone();
                        self.avgprice_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::MedpriceSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.medprice_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.medprice_win_snapshot = snap.clone();
                        self.medprice_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::TypPriceSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.typprice_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.typprice_win_snapshot = snap.clone();
                        self.typprice_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::WclPriceSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.wclprice_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.wclprice_win_snapshot = snap.clone();
                        self.wclprice_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::VarianceSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.variance_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.variance_win_snapshot = snap.clone();
                        self.variance_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::PlusDiSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.plus_di_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.plus_di_win_snapshot = snap.clone();
                        self.plus_di_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::MinusDiSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.minus_di_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.minus_di_win_snapshot = snap.clone();
                        self.minus_di_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::PlusDmSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.plus_dm_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.plus_dm_win_snapshot = snap.clone();
                        self.plus_dm_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::MinusDmSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.minus_dm_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.minus_dm_win_snapshot = snap.clone();
                        self.minus_dm_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::DxSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.dx_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.dx_win_snapshot = snap.clone();
                        self.dx_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::RocSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.roc_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.roc_win_snapshot = snap.clone();
                        self.roc_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::RocpSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.rocp_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.rocp_win_snapshot = snap.clone();
                        self.rocp_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::RocrSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.rocr_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.rocr_win_snapshot = snap.clone();
                        self.rocr_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::Rocr100SnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.rocr100_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.rocr100_win_snapshot = snap.clone();
                        self.rocr100_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CorrelSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.correl_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.correl_win_snapshot = snap.clone();
                        self.correl_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::MinSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.min_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.min_win_snapshot = snap.clone();
                        self.min_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::MaxSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.max_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.max_win_snapshot = snap.clone();
                        self.max_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::MinMaxSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.minmax_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.minmax_win_snapshot = snap.clone();
                        self.minmax_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::MinIndexSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.minindex_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.minindex_win_snapshot = snap.clone();
                        self.minindex_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::MaxIndexSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.maxindex_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.maxindex_win_snapshot = snap.clone();
                        self.maxindex_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::BbandsSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.bbands_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.bbands_win_snapshot = snap.clone();
                        self.bbands_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::AdSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.ad_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.ad_win_snapshot = snap.clone();
                        self.ad_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::AdoscSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.adosc_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.adosc_win_snapshot = snap.clone();
                        self.adosc_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::SumSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.sum_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.sum_win_snapshot = snap.clone();
                        self.sum_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::LinearRegInterceptSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self
                        .linreg_intercept_win_symbol
                        .eq_ignore_ascii_case(&sym_u)
                    {
                        self.linreg_intercept_win_snapshot = snap.clone();
                        self.linreg_intercept_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::AroonoscSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.aroonosc_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.aroonosc_win_snapshot = snap.clone();
                        self.aroonosc_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::MinMaxIndexSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.minmaxindex_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.minmaxindex_win_snapshot = snap.clone();
                        self.minmaxindex_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::MacdextSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.macdext_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.macdext_win_snapshot = snap.clone();
                        self.macdext_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::MacdfixSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.macdfix_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.macdfix_win_snapshot = snap.clone();
                        self.macdfix_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::MavpSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.mavp_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.mavp_win_snapshot = snap.clone();
                        self.mavp_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlDojiSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.cdl_doji_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.cdl_doji_win_snapshot = snap.clone();
                        self.cdl_doji_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlHammerSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.cdl_hammer_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.cdl_hammer_win_snapshot = snap.clone();
                        self.cdl_hammer_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlShootingStarSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self
                        .cdl_shooting_star_win_symbol
                        .eq_ignore_ascii_case(&sym_u)
                    {
                        self.cdl_shooting_star_win_snapshot = snap.clone();
                        self.cdl_shooting_star_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlEngulfingSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.cdl_engulfing_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.cdl_engulfing_win_snapshot = snap.clone();
                        self.cdl_engulfing_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlHaramiSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.cdl_harami_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.cdl_harami_win_snapshot = snap.clone();
                        self.cdl_harami_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlMorningStarSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self
                        .cdl_morning_star_win_symbol
                        .eq_ignore_ascii_case(&sym_u)
                    {
                        self.cdl_morning_star_win_snapshot = snap.clone();
                        self.cdl_morning_star_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlEveningStarSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self
                        .cdl_evening_star_win_symbol
                        .eq_ignore_ascii_case(&sym_u)
                    {
                        self.cdl_evening_star_win_snapshot = snap.clone();
                        self.cdl_evening_star_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlThreeBlackCrowsSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self
                        .cdl_three_black_crows_win_symbol
                        .eq_ignore_ascii_case(&sym_u)
                    {
                        self.cdl_three_black_crows_win_snapshot = snap.clone();
                        self.cdl_three_black_crows_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlThreeWhiteSoldiersSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self
                        .cdl_three_white_soldiers_win_symbol
                        .eq_ignore_ascii_case(&sym_u)
                    {
                        self.cdl_three_white_soldiers_win_snapshot = snap.clone();
                        self.cdl_three_white_soldiers_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlDarkCloudCoverSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self
                        .cdl_dark_cloud_cover_win_symbol
                        .eq_ignore_ascii_case(&sym_u)
                    {
                        self.cdl_dark_cloud_cover_win_snapshot = snap.clone();
                        self.cdl_dark_cloud_cover_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlPiercingSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.cdl_piercing_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.cdl_piercing_win_snapshot = snap.clone();
                        self.cdl_piercing_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlDragonflyDojiSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self
                        .cdl_dragonfly_doji_win_symbol
                        .eq_ignore_ascii_case(&sym_u)
                    {
                        self.cdl_dragonfly_doji_win_snapshot = snap.clone();
                        self.cdl_dragonfly_doji_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlGravestoneDojiSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self
                        .cdl_gravestone_doji_win_symbol
                        .eq_ignore_ascii_case(&sym_u)
                    {
                        self.cdl_gravestone_doji_win_snapshot = snap.clone();
                        self.cdl_gravestone_doji_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlHangingManSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.cdl_hanging_man_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.cdl_hanging_man_win_snapshot = snap.clone();
                        self.cdl_hanging_man_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlInvertedHammerSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self
                        .cdl_inverted_hammer_win_symbol
                        .eq_ignore_ascii_case(&sym_u)
                    {
                        self.cdl_inverted_hammer_win_snapshot = snap.clone();
                        self.cdl_inverted_hammer_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlHaramiCrossSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self
                        .cdl_harami_cross_win_symbol
                        .eq_ignore_ascii_case(&sym_u)
                    {
                        self.cdl_harami_cross_win_snapshot = snap.clone();
                        self.cdl_harami_cross_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlLongLeggedDojiSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self
                        .cdl_long_legged_doji_win_symbol
                        .eq_ignore_ascii_case(&sym_u)
                    {
                        self.cdl_long_legged_doji_win_snapshot = snap.clone();
                        self.cdl_long_legged_doji_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlMarubozuSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.cdl_marubozu_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.cdl_marubozu_win_snapshot = snap.clone();
                        self.cdl_marubozu_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlSpinningTopSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self
                        .cdl_spinning_top_win_symbol
                        .eq_ignore_ascii_case(&sym_u)
                    {
                        self.cdl_spinning_top_win_snapshot = snap.clone();
                        self.cdl_spinning_top_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlTristarSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.cdl_tristar_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.cdl_tristar_win_snapshot = snap.clone();
                        self.cdl_tristar_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlDojiStarSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.cdl_doji_star_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.cdl_doji_star_win_snapshot = snap.clone();
                        self.cdl_doji_star_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlMorningDojiStarSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self
                        .cdl_morning_doji_star_win_symbol
                        .eq_ignore_ascii_case(&sym_u)
                    {
                        self.cdl_morning_doji_star_win_snapshot = snap.clone();
                        self.cdl_morning_doji_star_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlEveningDojiStarSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self
                        .cdl_evening_doji_star_win_symbol
                        .eq_ignore_ascii_case(&sym_u)
                    {
                        self.cdl_evening_doji_star_win_snapshot = snap.clone();
                        self.cdl_evening_doji_star_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlAbandonedBabySnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self
                        .cdl_abandoned_baby_win_symbol
                        .eq_ignore_ascii_case(&sym_u)
                    {
                        self.cdl_abandoned_baby_win_snapshot = snap.clone();
                        self.cdl_abandoned_baby_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlThreeInsideSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self
                        .cdl_three_inside_win_symbol
                        .eq_ignore_ascii_case(&sym_u)
                    {
                        self.cdl_three_inside_win_snapshot = snap.clone();
                        self.cdl_three_inside_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlBeltHoldSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.cdl_belt_hold_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.cdl_belt_hold_win_snapshot = snap.clone();
                        self.cdl_belt_hold_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlClosingMarubozuSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self
                        .cdl_closing_marubozu_win_symbol
                        .eq_ignore_ascii_case(&sym_u)
                    {
                        self.cdl_closing_marubozu_win_snapshot = snap.clone();
                        self.cdl_closing_marubozu_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlHighWaveSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.cdl_high_wave_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.cdl_high_wave_win_snapshot = snap.clone();
                        self.cdl_high_wave_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlLongLineSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.cdl_long_line_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.cdl_long_line_win_snapshot = snap.clone();
                        self.cdl_long_line_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlShortLineSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.cdl_short_line_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.cdl_short_line_win_snapshot = snap.clone();
                        self.cdl_short_line_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlCounterattackSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self
                        .cdl_counterattack_win_symbol
                        .eq_ignore_ascii_case(&sym_u)
                    {
                        self.cdl_counterattack_win_snapshot = snap.clone();
                        self.cdl_counterattack_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlHomingPigeonSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self
                        .cdl_homing_pigeon_win_symbol
                        .eq_ignore_ascii_case(&sym_u)
                    {
                        self.cdl_homing_pigeon_win_snapshot = snap.clone();
                        self.cdl_homing_pigeon_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlInNeckSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.cdl_in_neck_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.cdl_in_neck_win_snapshot = snap.clone();
                        self.cdl_in_neck_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlOnNeckSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.cdl_on_neck_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.cdl_on_neck_win_snapshot = snap.clone();
                        self.cdl_on_neck_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlThrustingSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.cdl_thrusting_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.cdl_thrusting_win_snapshot = snap.clone();
                        self.cdl_thrusting_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlTwoCrowsSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.cdl_two_crows_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.cdl_two_crows_win_snapshot = snap.clone();
                        self.cdl_two_crows_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlThreeLineStrikeSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self
                        .cdl_three_line_strike_win_symbol
                        .eq_ignore_ascii_case(&sym_u)
                    {
                        self.cdl_three_line_strike_win_snapshot = snap.clone();
                        self.cdl_three_line_strike_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlThreeOutsideSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self
                        .cdl_three_outside_win_symbol
                        .eq_ignore_ascii_case(&sym_u)
                    {
                        self.cdl_three_outside_win_snapshot = snap.clone();
                        self.cdl_three_outside_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlMatchingLowSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self
                        .cdl_matching_low_win_symbol
                        .eq_ignore_ascii_case(&sym_u)
                    {
                        self.cdl_matching_low_win_snapshot = snap.clone();
                        self.cdl_matching_low_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlSeparatingLinesSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self
                        .cdl_separating_lines_win_symbol
                        .eq_ignore_ascii_case(&sym_u)
                    {
                        self.cdl_separating_lines_win_snapshot = snap.clone();
                        self.cdl_separating_lines_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlStickSandwichSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self
                        .cdl_stick_sandwich_win_symbol
                        .eq_ignore_ascii_case(&sym_u)
                    {
                        self.cdl_stick_sandwich_win_snapshot = snap.clone();
                        self.cdl_stick_sandwich_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlRickshawManSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self
                        .cdl_rickshaw_man_win_symbol
                        .eq_ignore_ascii_case(&sym_u)
                    {
                        self.cdl_rickshaw_man_win_snapshot = snap.clone();
                        self.cdl_rickshaw_man_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlTakuriSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.cdl_takuri_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.cdl_takuri_win_snapshot = snap.clone();
                        self.cdl_takuri_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlThreeStarsInSouthSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self
                        .cdl_three_stars_in_south_win_symbol
                        .eq_ignore_ascii_case(&sym_u)
                    {
                        self.cdl_three_stars_in_south_win_snapshot = snap.clone();
                        self.cdl_three_stars_in_south_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlIdenticalThreeCrowsSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self
                        .cdl_identical_three_crows_win_symbol
                        .eq_ignore_ascii_case(&sym_u)
                    {
                        self.cdl_identical_three_crows_win_snapshot = snap.clone();
                        self.cdl_identical_three_crows_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlKickingSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.cdl_kicking_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.cdl_kicking_win_snapshot = snap.clone();
                        self.cdl_kicking_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlKickingByLengthSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self
                        .cdl_kicking_by_length_win_symbol
                        .eq_ignore_ascii_case(&sym_u)
                    {
                        self.cdl_kicking_by_length_win_snapshot = snap.clone();
                        self.cdl_kicking_by_length_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlLadderBottomSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self
                        .cdl_ladder_bottom_win_symbol
                        .eq_ignore_ascii_case(&sym_u)
                    {
                        self.cdl_ladder_bottom_win_snapshot = snap.clone();
                        self.cdl_ladder_bottom_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlUniqueThreeRiverSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self
                        .cdl_unique_three_river_win_symbol
                        .eq_ignore_ascii_case(&sym_u)
                    {
                        self.cdl_unique_three_river_win_snapshot = snap.clone();
                        self.cdl_unique_three_river_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlAdvanceBlockSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self
                        .cdl_advance_block_win_symbol
                        .eq_ignore_ascii_case(&sym_u)
                    {
                        self.cdl_advance_block_win_snapshot = snap.clone();
                        self.cdl_advance_block_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlBreakawaySnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.cdl_breakaway_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.cdl_breakaway_win_snapshot = snap.clone();
                        self.cdl_breakaway_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlGapSideSideWhiteSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self
                        .cdl_gap_side_side_white_win_symbol
                        .eq_ignore_ascii_case(&sym_u)
                    {
                        self.cdl_gap_side_side_white_win_snapshot = snap.clone();
                        self.cdl_gap_side_side_white_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlUpsideGapTwoCrowsSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self
                        .cdl_upside_gap_two_crows_win_symbol
                        .eq_ignore_ascii_case(&sym_u)
                    {
                        self.cdl_upside_gap_two_crows_win_snapshot = snap.clone();
                        self.cdl_upside_gap_two_crows_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlXSideGapThreeMethodsSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self
                        .cdl_xside_gap_three_methods_win_symbol
                        .eq_ignore_ascii_case(&sym_u)
                    {
                        self.cdl_xside_gap_three_methods_win_snapshot = snap.clone();
                        self.cdl_xside_gap_three_methods_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlConcealBabySwallowSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self
                        .cdl_conceal_baby_swallow_win_symbol
                        .eq_ignore_ascii_case(&sym_u)
                    {
                        self.cdl_conceal_baby_swallow_win_snapshot = snap.clone();
                        self.cdl_conceal_baby_swallow_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlHikkakeSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.cdl_hikkake_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.cdl_hikkake_win_snapshot = snap.clone();
                        self.cdl_hikkake_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlHikkakeModSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.cdl_hikkake_mod_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.cdl_hikkake_mod_win_snapshot = snap.clone();
                        self.cdl_hikkake_mod_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlMatHoldSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.cdl_mat_hold_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.cdl_mat_hold_win_snapshot = snap.clone();
                        self.cdl_mat_hold_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlRiseFallThreeMethodsSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self
                        .cdl_rise_fall_three_methods_win_symbol
                        .eq_ignore_ascii_case(&sym_u)
                    {
                        self.cdl_rise_fall_three_methods_win_snapshot = snap.clone();
                        self.cdl_rise_fall_three_methods_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlStalledPatternSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self
                        .cdl_stalled_pattern_win_symbol
                        .eq_ignore_ascii_case(&sym_u)
                    {
                        self.cdl_stalled_pattern_win_snapshot = snap.clone();
                        self.cdl_stalled_pattern_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::CdlTasukiGapSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.cdl_tasuki_gap_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.cdl_tasuki_gap_win_snapshot = snap.clone();
                        self.cdl_tasuki_gap_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::ModSharpeSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.modsharpe_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.modsharpe_win_snapshot = snap.clone();
                        self.modsharpe_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::HsiehTestSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.hsiehtest_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.hsiehtest_win_snapshot = snap.clone();
                        self.hsiehtest_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::ChowBreakSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.chowbreak_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.chowbreak_win_snapshot = snap.clone();
                        self.chowbreak_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::DriftBurstSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.driftburst_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.driftburst_win_snapshot = snap.clone();
                        self.driftburst_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::HlvClustSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.hlvclust_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.hlvclust_win_snapshot = snap.clone();
                        self.hlvclust_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::YangZhangSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.yangzhang_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.yangzhang_win_snapshot = snap.clone();
                        self.yangzhang_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::KuiperSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.kuiper_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.kuiper_win_snapshot = snap.clone();
                        self.kuiper_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::DagostinoSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.dagostino_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.dagostino_win_snapshot = snap.clone();
                        self.dagostino_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::BaiPerronSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.baiperron_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.baiperron_win_snapshot = snap.clone();
                        self.baiperron_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::KupiecPofSnapshotMsg(sym, snap) => {
                    let sym_u = sym.to_uppercase();
                    if self.kupiecpof_win_symbol.eq_ignore_ascii_case(&sym_u) {
                        self.kupiecpof_win_snapshot = snap.clone();
                        self.kupiecpof_win_loading = false;
                    }
                    let _ = snap;
                }
                BrokerMsg::IngestResearchResult {
                    per_symbol_added,
                    errors,
                } => {
                    self.ingest_research_busy = false;
                    if per_symbol_added.is_empty() && errors.is_empty() {
                        self.ingest_research_status = "No articles parsed.".into();
                    } else {
                        let summary: Vec<String> = per_symbol_added
                            .iter()
                            .map(|(s, added, total)| format!("{}: +{} (now {})", s, added, total))
                            .collect();
                        let total_added: usize = per_symbol_added.iter().map(|(_, a, _)| *a).sum();
                        self.ingest_research_status = if errors.is_empty() {
                            format!(
                                "Ingested {} new articles across {} symbol(s): {}",
                                total_added,
                                per_symbol_added.len(),
                                summary.join(" · ")
                            )
                        } else {
                            format!(
                                "Ingested {} new articles · {} error(s): {}",
                                total_added,
                                errors.len(),
                                errors.join("; ")
                            )
                        };
                        self.log
                            .push_back(LogEntry::info(self.ingest_research_status.clone()));
                        // Auto-refresh the News panel so the pasting user sees the
                        // new articles without having to click "Load Cached". Prefer
                        // the symbol the user is currently filtering; otherwise fall
                        // back to the first ingested symbol.
                        if total_added > 0 {
                            let refresh_sym = if !self.news_symbol_filter.trim().is_empty() {
                                self.news_symbol_filter.trim().to_uppercase()
                            } else {
                                per_symbol_added
                                    .first()
                                    .map(|(s, _, _)| s.to_uppercase())
                                    .unwrap_or_default()
                            };
                            if !refresh_sym.is_empty() {
                                self.news_loading = true;
                                let _ = self.broker_tx.send(BrokerCmd::LoadCachedNews {
                                    symbol: refresh_sym,
                                    limit: 200,
                                });
                            }
                        }
                    }
                }
                BrokerMsg::NewsArticlesLoaded { symbol, articles } => {
                    self.news_loading = false;
                    let count = articles.len();
                    self.news_full_articles = articles;
                    // Update content hash for news cache guard
                    let mut h = self.news_full_articles.len() as u64;
                    if let Some(first) = self.news_full_articles.first() {
                        for b in first.headline.as_bytes() {
                            h = h.wrapping_mul(31).wrapping_add(*b as u64);
                        }
                    }
                    self.news_input_hash = h;
                    self.news_articles = self
                        .news_full_articles
                        .iter()
                        .map(|a| {
                            let dt =
                                chrono::DateTime::<chrono::Utc>::from_timestamp(a.published_at, 0)
                                    .map(|d| d.format("%Y-%m-%d").to_string())
                                    .unwrap_or_else(|| "—".to_string());
                            let source = if a.provider.is_empty() {
                                a.source.clone()
                            } else {
                                a.provider.clone()
                            };
                            (a.headline.clone(), source, dt)
                        })
                        .collect();
                    // Restore selection by stable URL hash after reload/session restore.
                    if !self.news_selected_url_hash.is_empty() {
                        self.news_selected = self
                            .news_full_articles
                            .iter()
                            .position(|a| a.url_hash == self.news_selected_url_hash);
                    }
                    // Clear selection if the selected index is now out of range.
                    if let Some(idx) = self.news_selected {
                        if idx >= self.news_full_articles.len() {
                            self.news_selected = None;
                        }
                    }
                    if self.news_selected.is_none() && !self.news_full_articles.is_empty() {
                        self.news_selected = Some(0);
                    }
                    if let Some(idx) = self.news_selected {
                        if let Some(article) = self.news_full_articles.get(idx) {
                            self.news_selected_url_hash = article.url_hash.clone();
                        }
                    }
                    let label = if symbol.is_empty() {
                        "all".to_string()
                    } else {
                        symbol
                    };
                    self.log.push_back(LogEntry::info(format!(
                        "News {}: {} articles loaded",
                        label, count
                    )));
                }
                BrokerMsg::UnusualVolumeResults(results) => {
                    self.log.push_back(LogEntry::info(format!(
                        "Unusual volume: {} symbols flagged",
                        results.len()
                    )));
                    self.unusual_volume_results = results;
                }
                BrokerMsg::MarketClock(msg) => {
                    self.market_clock_status = msg.clone();
                    self.log.push_back(LogEntry::info(msg));
                }
                BrokerMsg::StreamTick {
                    symbol,
                    price,
                    size,
                    timestamp,
                } => {
                    // TAS tape — keep up to 500 most-recent trades for the current TAS subscription.
                    if self.show_tas
                        && !self.tas_paused
                        && !self.tas_symbol.is_empty()
                        && (symbol.eq_ignore_ascii_case(&self.tas_symbol)
                            || self.tas_symbol.contains(&symbol)
                            || symbol.contains(&self.tas_symbol))
                    {
                        // Infer side from previous-tick comparison on the same symbol.
                        let side = if let Some((_, prev_px, _, _, _)) = self.tas_rows.front() {
                            if price > *prev_px {
                                "buy"
                            } else if price < *prev_px {
                                "sell"
                            } else {
                                "flat"
                            }
                        } else {
                            "flat"
                        };
                        self.tas_rows.push_front((
                            symbol.clone(),
                            price,
                            size,
                            side.to_string(),
                            timestamp.clone(),
                        ));
                        while self.tas_rows.len() > 500 {
                            self.tas_rows.pop_back();
                        }
                    }
                    // Feed into BarBuilder for real-time bar construction
                    if let Ok(mut bb) = self.bar_builder.lock() {
                        bb.ingest_trade(&symbol, price, size, &timestamp);
                        // Drain completed bars and append to matching charts
                        let completed = bb.drain_completed();
                        for bar in completed {
                            for chart in &mut self.charts {
                                if chart.symbol.contains(&bar.symbol)
                                    || bar.symbol.contains(
                                        &chart
                                            .symbol
                                            .split(':')
                                            .rev()
                                            .nth(1)
                                            .or_else(|| chart.symbol.split(':').last())
                                            .unwrap_or(""),
                                    )
                                {
                                    chart.bars.push(Bar {
                                        ts_ms: chrono::DateTime::parse_from_rfc3339(&bar.timestamp)
                                            .map(|dt| dt.timestamp_millis())
                                            .unwrap_or(0),
                                        open: bar.open,
                                        high: bar.high,
                                        low: bar.low,
                                        close: bar.close,
                                        volume: bar.volume,
                                    });
                                    // Advance view offset if following latest
                                    if self.follow_latest
                                        && !chart.manual_view_override
                                        && chart.view_offset >= chart.bars.len().saturating_sub(2)
                                    {
                                        chart.view_offset = chart.bars.len().saturating_sub(1) + 20;
                                    }
                                }
                            }
                        }
                    }
                }
                BrokerMsg::StreamQuoteTick { symbol, bid, ask } => {
                    // Update forming bar close price + live bid/ask on matching charts
                    let last = (bid + ask) / 2.0;
                    if last > 0.0 {
                        // Live quotes stored in-memory only (chart.live_bid/ask).
                        // Removed per-tick KV writes: 851 symbols × zstd compress + SQLite INSERT
                        // was burning hundreds of SSD writes/sec during market hours.
                        for chart in &mut self.charts {
                            if chart.symbol.contains(&symbol) {
                                chart.live_bid = bid;
                                chart.live_ask = ask;
                                chart.live_quote_at = Some(std::time::Instant::now());
                                if let Some(bar) = chart.bars.last_mut() {
                                    bar.close = last;
                                    bar.high = bar.high.max(last);
                                    bar.low = bar.low.min(last);
                                }
                            }
                        }
                    }
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
                            let already = self.symbol_suggestions.iter().any(|(s, _, _)| {
                                let s_norm = s.replace('/', "");
                                s_norm.eq_ignore_ascii_case(&normalized)
                            });
                            if !already {
                                self.symbol_suggestions.push((normalized, name, class));
                            }
                        }
                        self.symbol_suggestions.truncate(20);
                    }
                }
                BrokerMsg::BarsFetched {
                    source,
                    symbol,
                    timeframe,
                    count,
                } => {
                    let should_reload = self
                        .charts
                        .get(self.active_tab)
                        .map(|c| c.should_reload_for_bar_fetch(&symbol, &timeframe, &source))
                        .unwrap_or(false);
                    let source_label = match source.as_str() {
                        "alpaca" => "Alpaca",
                        "kraken" => "Kraken",
                        "kraken-futures" => "Kraken Futures",
                        "yahoo-chart" => "Yahoo Chart",
                        _ => source.as_str(),
                    };
                    if should_reload {
                        self.log.push_back(LogEntry::info(format!(
                            "{} fetched {} bars for {} {} — queued active chart reload",
                            source_label, count, symbol, timeframe
                        )));
                    } else {
                        tracing::debug!(
                            "{} fetched {} bars for {} {}",
                            source_label,
                            count,
                            symbol,
                            timeframe
                        );
                    }
                    let source_has_terminal_settlement = matches!(
                        source.as_str(),
                        "alpaca" | "kraken" | "kraken-futures"
                    );
                    if !source_has_terminal_settlement {
                        self.settle_market_data_fetch(&source, &symbol, &timeframe);
                    }
                    if source_has_terminal_settlement {
                        self.note_cached_sync_success(&source, &symbol, &timeframe, count);
                    }
                    if source == "alpaca" {
                        // Any newly-written bars supersede prior no-data tombstones.
                        self.alpaca_no_data_drain(&symbol, &timeframe);
                        // Avoid a synchronous full SQLite storage-stat scan for every
                        // automated bar write. `note_cached_sync_success` keeps the
                        // scheduler O(1)-fresh; refresh the heavy Storage view only
                        // when a storage window is visible.
                        if self.show_storage || self.show_cache_stats {
                            self.refresh_storage_snapshot_after_action("alpaca_bars");
                        }
                    }

                    if should_reload {
                        self.queue_chart_reload(self.active_tab);
                    }
                    if !source_has_terminal_settlement
                        && matches!(source.as_str(), "kraken" | "kraken-futures")
                    {
                        market_data_refill_requested = true;
                    }
                }
                BrokerMsg::AlpacaFetchSettled {
                    symbol,
                    timeframe,
                    success,
                } => {
                    self.settle_market_data_fetch("alpaca", &symbol, &timeframe);
                    if success {
                        self.alpaca_retry_drain(&symbol, &timeframe);
                        market_data_refill_requested = true;
                    }
                }
                BrokerMsg::KrakenFetchSettled { symbol, timeframe } => {
                    self.settle_market_data_fetch("kraken", &symbol, &timeframe);
                    market_data_refill_requested = true;
                }
                BrokerMsg::KrakenBackfillComplete {
                    symbol,
                    timeframe,
                    bar_count,
                    target_bars,
                } => {
                    let changed = self.kraken_backfill_complete_mark(
                        &symbol,
                        &timeframe,
                        bar_count,
                        target_bars,
                    );
                    if changed {
                        let marker_count = self.kraken_backfill_complete_pairs.len();
                        // First-time saturation per pair is one-shot, but
                        // across ~12 k tradable symbols this floods the
                        // user log during initial sweep. Detailed line goes
                        // to debug; a milestone rollup at every 100th new
                        // marker keeps progress visible without spam.
                        tracing::debug!(
                            "Kraken {} {}: provider window saturated at {}/{} bars ({} marked)",
                            symbol,
                            timeframe,
                            bar_count,
                            target_bars,
                            marker_count
                        );
                        if marker_count.is_multiple_of(100) {
                            self.log.push_back(LogEntry::info(format!(
                                "Kraken backfill milestone: {} pairs at provider-window saturation",
                                marker_count
                            )));
                        }
                    }
                }
                BrokerMsg::KrakenFuturesFetchSettled { symbol, timeframe } => {
                    self.settle_market_data_fetch("kraken-futures", &symbol, &timeframe);
                    market_data_refill_requested = true;
                }
                BrokerMsg::KrakenFuturesBackfillComplete {
                    symbol,
                    timeframe,
                    bar_count,
                    target_bars,
                } => {
                    let changed = self.kraken_futures_backfill_complete_mark(
                        &symbol,
                        &timeframe,
                        bar_count,
                        target_bars,
                    );
                    if changed {
                        let marker_count = self.kraken_futures_backfill_complete_pairs.len();
                        self.log.push_back(LogEntry::info(format!(
                            "Kraken Futures {} {}: marked backfill-complete at {}/{} bars — full history exhausted; automated sync will keep it current ({} marked)",
                            symbol, timeframe, bar_count, target_bars, marker_count
                        )));
                    }
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
        self.drain_web_client_commands(ctx);

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
                // symbol gets its own MT5-style grid; M1/M5 are excluded at the helper.
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
                let header_h = 18.0_f32;
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

                // Lazy-load bars for visible MTF grid charts
                if let Some(ref cache) = self.cache {
                    'load_one: for group in &mtf_groups {
                        for &vi in &group.indices {
                            let chart = &mut self.charts[vi];
                            if chart.bars.is_empty() {
                                let loaded = { let mut gpu = self.gpu_indicators.take(); let r = chart.try_load(cache, &mut self.log, gpu.as_mut()); self.gpu_indicators = gpu; r };
                                let _ = loaded;
                                break 'load_one;
                            }
                        }
                    }
                }

                let mut group_top = available.top();
                for (group_idx, group) in mtf_groups.iter().enumerate() {
                    let (cols, rows) = group_layout[group_idx];
                    let group_header_rect = egui::Rect::from_min_size(
                        egui::pos2(available.left(), group_top),
                        egui::vec2(available.width(), header_h),
                    );
                    ui.painter().text(
                        group_header_rect.left_center() + egui::vec2(6.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        &group.symbol,
                        egui::FontId::proportional(12.0),
                        ACCENT,
                    );
                    group_top += header_h;
                    let cell_w = available.width() / cols as f32;
                    let cell_h = chart_row_h;

                    for (grid_pos, &vi) in group.indices.iter().enumerate() {
                // Rebuild trade overlay every 120 frames (~30s) or on first load
                let fc = self.frame_count;
                if !self.heavy_sync_in_progress && self.charts[vi].cached_trade_overlay_frame == 0 || fc.wrapping_sub(self.charts[vi].cached_trade_overlay_frame) > 120 {
                    self.charts[vi].cached_trade_overlay = self.build_trade_overlay(&self.charts[vi]);
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
                    draw_chart(&painter, chart, cell_rect, crosshair, &flags, show_rsi, show_fisher, show_macd, show_volume_pane, show_stochastic, show_adx, show_cci, show_williams_r, show_obv, show_momentum, show_cmo, show_qstick, show_disparity, show_bop, show_stddev, show_mfi, show_trix, show_ppo, show_ultosc, show_stochrsi, show_var_oscillator, show_better_volume, show_ehlers_ebsw, show_ehlers_cyber, show_ehlers_cg, show_ehlers_roof, self.show_squeeze, sl_price, tp_price, &trade_ov, &self.alerts, &self.draw_mode);
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
                // own it, which regressed TradingView/MT5-style scale dragging.
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
                    draw_chart(&painter, chart, rect, crosshair, &flags, show_rsi, show_fisher, show_macd, show_volume_pane, show_stochastic, show_adx, show_cci, show_williams_r, show_obv, show_momentum, show_cmo, show_qstick, show_disparity, show_bop, show_stddev, show_mfi, show_trix, show_ppo, show_ultosc, show_stochrsi, show_var_oscillator, show_better_volume, show_ehlers_ebsw, show_ehlers_cyber, show_ehlers_cg, show_ehlers_roof, self.show_squeeze, sl_price, tp_price, &trade_ov, &self.alerts, &self.draw_mode);
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
                    // BarCacheWriter's metadata rows all follow `<prefix>:__<NAME>__[…]`
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

        // Poll for remote commands from LAN clients (server only, every ~5 seconds).
        // Uses drain_queue for atomic read+delete (O(1) per entry instead of O(n) full
        // array read + rewrite). See cache.rs:append_to_queue for the producer side.
        if now_instant.duration_since(self.lan_remote_last_poll)
            >= std::time::Duration::from_secs(5)
            && self.lan_sync_mode == "server"
        {
            self.lan_remote_last_poll = now_instant;
            if let Some(ref cache) = self.cache {
                if let Ok(entries) = cache.drain_queue("lan:remote_queue") {
                    if !entries.is_empty() {
                        let queue: Vec<serde_json::Value> = entries
                            .iter()
                            .filter_map(|s| serde_json::from_str(s).ok())
                            .collect();
                        if !queue.is_empty() {
                            for v in &queue {
                                let cmd = v["cmd"].as_str().unwrap_or("");
                                let args = v["args"].as_str().unwrap_or("");
                                match cmd {
                                    "FETCH_BARS" => {
                                        if let Some((symbol, tf)) = args.split_once(',') {
                                            let Some(tf_norm) = normalize_sync_timeframe_key(tf)
                                            else {
                                                continue;
                                            };
                                            if !self.sync_timeframe_enabled(tf_norm) {
                                                self.log.push_back(LogEntry::info(format!(
                                                    "LAN remote: skipped {} {} (timeframe disabled)",
                                                    symbol, tf_norm
                                                )));
                                                continue;
                                            }
                                            let db_path = cache_db_path();
                                            // Detect crypto and use Kraken (free, works weekends) + Alpaca
                                            let su = symbol.to_uppercase();
                                            let crypto_bases = [
                                                "BTC", "ETH", "SOL", "DOGE", "XRP", "ADA", "LTC",
                                                "LINK", "AVAX", "DOT", "XMR", "ZEC", "DASH",
                                            ];
                                            let is_crypto = crypto_bases
                                                .iter()
                                                .any(|b| su.starts_with(b) && su.ends_with("USD"));
                                            if is_crypto {
                                                if self.kraken_spot_symbol_scrape_enabled(symbol) {
                                                    let _ =
                                                        self.broker_tx
                                                            .send(BrokerCmd::KrakenBackfill {
                                                            symbol: symbol.to_string(),
                                                            timeframes: vec![tf_norm.to_string()],
                                                            db_path: db_path.clone(),
                                                            backfill_complete: false,
                                                        });
                                                    self.log.push_back(LogEntry::info(format!(
                                                        "LAN remote: fetching {} {} from Kraken",
                                                        symbol, tf_norm
                                                    )));
                                                } else {
                                                    self.log.push_back(LogEntry::info(format!(
                                                        "LAN remote: skipped Kraken {} {} (disabled universe)",
                                                        symbol, tf_norm
                                                    )));
                                                }
                                            }
                                            self.queue_alpaca_fetch(symbol, tf_norm);
                                            if !is_crypto {
                                                self.log.push_back(LogEntry::info(format!(
                                                    "LAN remote: fetching {} {} from Alpaca",
                                                    symbol, tf_norm
                                                )));
                                            }
                                        }
                                    }
                                    "FUNDAMENTALS" | "FUNDAMENTALS_FORCE" => {
                                        let force = cmd == "FUNDAMENTALS_FORCE";
                                        let db_path = cache_db_path();
                                        let _ =
                                            self.broker_tx.send(BrokerCmd::FundamentalsScrape {
                                                db_path,
                                                                use_alpaca: self.fund_source_alpaca,
                                                use_kraken: self.fund_source_kraken,
                                                kraken_equity_symbols: self
                                                    .kraken_equity_universe_symbols
                                                    .clone(),
                                                force,
                                            });
                                        let label = if force {
                                            "fundamentals scrape started (FORCE)"
                                        } else {
                                            "fundamentals scrape started"
                                        };
                                        self.log.push_back(LogEntry::info(format!(
                                            "LAN remote: {}",
                                            label
                                        )));
                                    }
                                    "SEC_SCRAPE" => {
                                        let symbols = self.sec_scrape_scope_symbols();
                                        let symbol_count = symbols.len();
                                        if symbol_count > 0 {
                                            let db_path = cache_db_path();
                                            let _ = self
                                                .broker_tx
                                                .send(BrokerCmd::SecScrape { db_path, symbols });
                                            self.log.push_back(LogEntry::info(format!(
                                                "LAN remote: SEC scrape started for Scope {} ({} symbols)",
                                                self.broker_scope_label(),
                                                symbol_count
                                            )));
                                        } else {
                                            self.log.push_back(LogEntry::warn(format!(
                                                "LAN remote: SEC scrape skipped: Scope {} has no symbols",
                                                self.broker_scope_label()
                                            )));
                                        }
                                    }
                                    "FRED_DATA" => {
                                        if !self.fred_key.is_empty() {
                                            let _ = self.broker_tx.send(BrokerCmd::FredFetch {
                                                api_key: self.fred_key.clone(),
                                            });
                                            self.log.push_back(LogEntry::info(
                                                "LAN remote: FRED fetch started",
                                            ));
                                        }
                                    }
                                    "FINNHUB_NEWS" => {
                                        if !self.finnhub_key.is_empty() {
                                            let sym = if args.is_empty() {
                                                "general".to_string()
                                            } else {
                                                args.to_string()
                                            };
                                            let _ = self.broker_tx.send(BrokerCmd::FinnhubNews {
                                                symbol: sym,
                                                api_key: self.finnhub_key.clone(),
                                            });
                                            self.log.push_back(LogEntry::info(
                                                "LAN remote: Finnhub news fetch started",
                                            ));
                                        }
                                    }
                                    "CALENDAR" => {
                                        if !self.finnhub_key.is_empty() {
                                            let _ =
                                                self.broker_tx.send(BrokerCmd::FetchEconCalendar {
                                                    finnhub_key: self.finnhub_key.clone(),
                                                });
                                            self.log.push_back(LogEntry::info(
                                                "LAN remote: econ calendar fetch started",
                                            ));
                                        }
                                    }
                                    "CONGRESS_TRADES" => {
                                        let _ = self.broker_tx.send(BrokerCmd::FetchCongressTrades);
                                        self.log.push_back(LogEntry::info(
                                            "LAN remote: congress trades fetch started",
                                        ));
                                    }
                                    "FUNDAMENTALS_ONE" => {
                                        if !args.is_empty() {
                                            let db_path = cache_db_path();
                                            let _ = self.broker_tx.send(
                                                BrokerCmd::FundamentalsScrapeOne {
                                                    ticker: args.to_string(),
                                                    db_path,
                                                },
                                            );
                                            self.log.push_back(LogEntry::info(format!(
                                                "LAN remote: fundamentals scrape for {}",
                                                args
                                            )));
                                        }
                                    }
                                    "KRAKEN_BACKFILL" => {
                                        let db_path = cache_db_path();
                                        let sym = if args.is_empty() { "BTCUSD" } else { args };
                                        let tfs = self.filtered_sync_timeframes([
                                            "1Day", "1Hour", "4Hour", "15Min", "30Min", "5Min",
                                        ]);
                                        if !tfs.is_empty()
                                            && self.kraken_spot_symbol_scrape_enabled(sym)
                                        {
                                            let _ =
                                                self.broker_tx.send(BrokerCmd::KrakenBackfill {
                                                    symbol: sym.to_string(),
                                                    timeframes: tfs,
                                                    db_path,
                                                    backfill_complete: false,
                                                });
                                            self.log.push_back(LogEntry::info(format!(
                                                "LAN remote: Kraken backfill {} started",
                                                sym
                                            )));
                                        }
                                    }
                                    "KRAKEN_FUTURES_BACKFILL" => {
                                        let db_path = cache_db_path();
                                        let sym = if args.is_empty() { "PF_XBTUSD" } else { args };
                                        let tfs = self.filtered_sync_timeframes([
                                            "1Day", "1Hour", "4Hour", "15Min", "30Min", "5Min",
                                        ]);
                                        if !tfs.is_empty() && self.kraken_scrape_futures {
                                            let _ = self.broker_tx.send(
                                                BrokerCmd::KrakenFuturesBackfill {
                                                    symbol: sym.to_string(),
                                                    timeframes: tfs,
                                                    db_path,
                                                    backfill_complete: false,
                                                },
                                            );
                                            self.log.push_back(LogEntry::info(format!(
                                                "LAN remote: Kraken Futures backfill {} started",
                                                sym
                                            )));
                                        }
                                    }
                                    "EVSCRAPE" | "EVSCRAPE_FORCE" => {
                                        let force = cmd == "EVSCRAPE_FORCE";
                                        let db_path = cache_db_path();
                                        let _ =
                                            self.broker_tx.send(BrokerCmd::FundamentalsScrape {
                                                db_path,
                                                                use_alpaca: self.fund_source_alpaca,
                                                use_kraken: self.fund_source_kraken,
                                                kraken_equity_symbols: self
                                                    .kraken_equity_universe_symbols
                                                    .clone(),
                                                force,
                                            });
                                        let label = if force {
                                            "EVScrape started (FORCE)"
                                        } else {
                                            "EVScrape started"
                                        };
                                        self.log.push_back(LogEntry::info(format!(
                                            "LAN remote: {}",
                                            label
                                        )));
                                    }
                                    "INGEST_RESEARCH" => {
                                        // Args is a JSON object: {"text": "...", "agent": "..."}.
                                        // Unwrap and re-dispatch as a local ingest so the server's
                                        // DB gets the articles — clients will pull via normal
                                        // research_web_articles / research_news table sync.
                                        let parsed: serde_json::Value = serde_json::from_str(args)
                                            .unwrap_or(serde_json::Value::Null);
                                        let text = parsed
                                            .get("text")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string();
                                        let agent_override = parsed
                                            .get("agent")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string();
                                        if !text.trim().is_empty() {
                                            let _ = self.broker_tx.send(
                                                BrokerCmd::IngestResearchArticles {
                                                    text,
                                                    agent_override,
                                                },
                                            );
                                            self.log.push_back(LogEntry::info(
                                                "LAN remote: research ingest accepted from client",
                                            ));
                                        }
                                    }
                                    _ => {
                                        self.log.push_back(LogEntry::info(format!(
                                            "LAN remote: unhandled '{}' (args: {})",
                                            cmd, args
                                        )));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Poll Kraken positions every ~60 seconds so terminal-managed exit changes
        // and broker fills eventually converge back into the unified position view.
        if now_instant.duration_since(self.kraken_positions_last_poll)
            >= std::time::Duration::from_secs(60)
            && self.kraken_enabled
            && self.kraken_connected
            && !self.lan_client_enabled
            && self.cache_loaded
        {
            self.kraken_positions_last_poll = now_instant;
            let _ = self.broker_tx.send(BrokerCmd::KrakenGetPositions);
            let _ = self.broker_tx.send(BrokerCmd::KrakenGetBalance);
        }

        // Open Kraken equity positions are safety-critical foreground quotes, not
        // broad history sync. The 60s balance/position REST poll already queues
        // one ticker refresh, but P/L should not sit stale for a full minute when
        // iapi is healthy. Until a verified Kraken Securities quote stream exists,
        // refresh held xStock symbols on the same 15s cadence as watchlist quotes.
        if now_instant.duration_since(self.kraken_position_quotes_last_poll)
            >= std::time::Duration::from_secs(15)
            && self.kraken_enabled
            && self.kraken_connected
            && !self.kr_positions.is_empty()
            && !self.lan_client_enabled
            && self.cache_loaded
        {
            self.kraken_position_quotes_last_poll = now_instant;
            let mut symbols = std::collections::BTreeSet::new();
            for pos in &self.kr_positions {
                if pos.qty == 0.0 {
                    continue;
                }
                let symbol = pos
                    .symbol
                    .replace('/', "")
                    .trim_end_matches(".EQ")
                    .to_ascii_uppercase();
                if !symbol.is_empty() {
                    symbols.insert(symbol);
                }
            }
            for symbol in symbols {
                self.dispatch_kraken_equity_ticker(&symbol);
            }
        }

        // Poll watchlist quotes every ~15 seconds at 60fps (900 frames). Disabled for LAN client.
        // Uses the best available stack: broker snapshots when connected, Yahoo quote enrichment,
        // and cached bars as a weekend/off-hours fallback.
        if watchlist_quote_poll_ready(
            now_instant.duration_since(self.watchlist_quotes_last_poll),
            !self.user_watchlist.is_empty(),
            self.lan_client_enabled,
            self.cache_loaded,
        ) {
            self.watchlist_quotes_last_poll = now_instant;
            let _ = self.broker_tx.send(BrokerCmd::GetWatchlistQuotes {
                symbols: self.user_watchlist.clone(),
            });
        }

        // ── Data sync (disabled when LAN client — server provides all data) ──
        let is_lan_client = self.lan_client_enabled || self.lan_sync_mode == "client";

        // No API calls or data operations before cache is loaded
        if !is_lan_client && self.cache_loaded {
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
