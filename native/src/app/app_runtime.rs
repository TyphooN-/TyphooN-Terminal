use super::*;

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
        // Track user activity for the auto-compact idle gate. Any input event in
        // the frame counts as activity. Cheap — `events` is always queried below.
        if ctx.input(|i| !i.events.is_empty()) {
            self.auto_compact_last_input_at = std::time::Instant::now();
        }
        self.tick_auto_compact();
        // Alpaca retry queue: internally throttled to 10s between ticks.
        // Loads persisted state on first call, re-dispatches due entries.
        self.poll_alpaca_retry_queue();
        // PERF: rebuild scope HashSet only when bg data loaded or scope changed,
        // not every frame. Steady state = zero work.
        let scope_key = (self.bg_rev, self.broker_scope);
        if !self.user_interacting && self.cached_scope_key != Some(scope_key) {
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
        // PERF: Cache Darwin tradability + MT5/tasty bar-coverage sets on bg_rev.
        // Was rebuilding per frame in hot windows and sync loops.
        if self.cached_mt5_symbols_rev != Some(self.bg_rev) {
            // Start with Darwinex symbols from MT5 specs (if any)
            let mut darwin: std::collections::HashSet<String> = self
                .bg
                .darwinex_specs
                .iter()
                .filter(|(_, _, _, trade_mode, _, _, _, _, _)| *trade_mode != 0)
                .map(|(sym, _, _, _, _, _, _, _, _)| normalize_market_data_symbol(sym))
                .collect();

            // Merge the full hardcoded Darwinex Zero USA Stocks + ETFs universe.
            // This allows viewing/analyzing these symbols using Kraken/Alpaca data
            // even when not syncing MT5.
            for s in crate::app::darwin_universe::darwinex_usa_equity_symbols() {
                darwin.insert(normalize_market_data_symbol(s));
            }
            self.cached_darwin_symbols = darwin;
            self.cached_mt5_symbols = self
                .bg
                .detailed_stats
                .iter()
                .filter_map(|(k, _, _)| {
                    // BarCacheWriter's only bar-key shape is `mt5:{SYM}:{TF}`. Metadata
                    // (`mt5:__SYMBOLS__`, `mt5:__HEARTBEAT__:acct`, …) lives under the
                    // `mt5:__` prefix — skip it so we don't land bogus synthetic
                    // symbols like "MT5" or "__HEARTBEAT__" in the tradable set.
                    let rest = k.strip_prefix("mt5:")?;
                    if rest.starts_with("__") {
                        return None;
                    }
                    let mut it = rest.split(':');
                    let sym = it.next()?;
                    let _tf = it.next()?;
                    if it.next().is_some() || sym.is_empty() {
                        return None;
                    }
                    Some(normalize_market_data_symbol(sym))
                })
                .collect();
            // Tastytrade bar keys are `tastytrade:{SYM}:{TF}` (3-part, no
            // metadata sub-prefix in use). Same scan, same revision gate.
            self.cached_tastytrade_symbols = self
                .bg
                .detailed_stats
                .iter()
                .filter_map(|(k, _, _)| {
                    let rest = k.strip_prefix("tastytrade:")?;
                    let mut it = rest.split(':');
                    let sym = it.next()?;
                    let _tf = it.next()?;
                    if it.next().is_some() || sym.is_empty() {
                        return None;
                    }
                    Some(
                        normalize_market_data_symbol(sym)
                            .replace('/', "")
                            .to_ascii_uppercase(),
                    )
                })
                .collect();
            self.cached_mt5_symbols_rev = Some(self.bg_rev);
        }
        if self.cached_alpaca_sync_state_rev != Some(self.bg_rev) {
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
            && self.kraken_any_spot_scrape_enabled()
            && self.kraken_pairs.is_empty()
            && !self.kraken_pairs_requested
        {
            let _ = self.broker_tx.send(BrokerCmd::KrakenGetPairs);
            self.kraken_pairs_requested = true;
        }
        if self.cache_loaded
            && self.lan_sync_mode != "client"
            && self.kraken_scrape_futures
            && self.kraken_futures_symbols.is_empty()
            && !self.kraken_futures_requested
        {
            let _ = self.broker_tx.send(BrokerCmd::KrakenFuturesGetInstruments);
            self.kraken_futures_requested = true;
        }
        if self.cache_loaded
            && self.tt_connected
            && !self.tastytrade_universe_requested
            && chrono::Utc::now().timestamp() >= self.tastytrade_universe_retry_after_ts
            && self.lan_sync_mode != "client"
        {
            let _ = self.broker_tx.send(BrokerCmd::TastytradeGetUniverse);
            self.tastytrade_universe_requested = true;
        }

        // Periodic crypto bar refresh (every ~60 seconds at 4fps = every 240 frames)
        // Periodic crypto bar refresh (~60s) — works on both server and LAN client
        // Uses Kraken (free, no auth) as primary source, Alpaca as fallback
        // Periodic crypto bar refresh — SERVER/STANDALONE ONLY
        // LAN clients get ALL data from server via sync — no direct API calls
        if self.frame_count % 240 == 120 && self.lan_sync_mode != "client" && self.cache_loaded {
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
                        let db_path = cache_db_path();
                        // Server/standalone: fetch directly from Kraken (LAN clients excluded by outer guard)
                        let timeframes =
                            self.filtered_sync_timeframes(timeframes.iter().map(|tf| tf.as_str()));
                        if !timeframes.is_empty() && self.kraken_spot_symbol_scrape_enabled(&bare) {
                            let _ = self.broker_tx.send(BrokerCmd::KrakenBackfill {
                                symbol: bare.clone(),
                                timeframes,
                                db_path: db_path.clone(),
                            });
                        }
                    }
                }
            }
        }

        if self.frame_count % 240 == 20
            && self.frame_count > 0
            && self.cache_loaded
            && self.lan_sync_mode != "client"
            && !self.kraken_pairs.is_empty()
        {
            let _ = self.schedule_kraken_universe_sectors();
        }

        if self.frame_count % 240 == 40
            && self.frame_count > 0
            && self.cache_loaded
            && self.lan_sync_mode != "client"
        {
            let _ = self.schedule_kraken_futures_universe_sectors();
        }

        if self.frame_count % 240 == 170
            && self.frame_count > 0
            && self.cache_loaded
            && self.lan_sync_mode != "client"
            && self.tt_connected
        {
            let symbols = self.tastytrade_sync_symbols();
            let _ = self.schedule_tastytrade_symbols(&symbols);
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
            self.tastytrade_backfill_complete_load();
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
                    (keyring::keys::TT_USERNAME, "tt_username"),
                    (keyring::keys::TT_PASSWORD, "tt_password"),
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
                        k if k == keyring::keys::TT_USERNAME => self.tt_username = val.clone(),
                        k if k == keyring::keys::TT_PASSWORD => self.tt_password = val.clone(),
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
            if !self.broker_api_key.is_empty()
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
            if !self.kraken_api_key.is_empty() && !self.kraken_api_secret.is_empty() {
                let _ = self.broker_tx.send(BrokerCmd::KrakenConnect {
                    api_key: self.kraken_api_key.clone(),
                    api_secret: self.kraken_api_secret.clone(),
                    ws_api_key: self.kraken_ws_api_key.clone(),
                    ws_api_secret: self.kraken_ws_api_secret.clone(),
                });
                self.log
                    .push_back(LogEntry::info("Kraken auto-connecting..."));
            }

            // ── Restore Darwinex web scraping config from cache (ADR-093) ──
            if let Some(ref cache) = self.cache {
                if let Ok(Some(json)) =
                    cache.get_kv(typhoon_engine::core::darwin_web::cache_keys::CONFIG)
                {
                    if let Ok(cfg) = serde_json::from_str::<
                        typhoon_engine::core::darwin_web::DarwinWebConfig,
                    >(&json)
                    {
                        self.dwx_config = cfg;
                        if !self.dwx_config.managed_darwins.is_empty() {
                            self.log.push_back(LogEntry::info(format!(
                                "DWX config restored: {} DARWINs [{}], auto={}",
                                self.dwx_config.managed_darwins.len(),
                                self.dwx_config.managed_darwins.join(", "),
                                self.dwx_config.auto_scrape
                            )));
                        }
                    }
                }
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
                // Auto-import DARWIN XLSX if needed (not on LAN client)
                if !self.darwin_xlsx_dir.is_empty() && !self.lan_client_enabled {
                    let dir = std::path::PathBuf::from(&self.darwin_xlsx_dir);
                    if dir.is_dir() {
                        let has_accounts = self
                            .cache
                            .as_ref()
                            .and_then(|c| {
                                c.connection().ok().and_then(|conn| {
                                    let _ = darwin::create_darwin_tables(&conn);
                                    darwin::list_darwin_accounts(&conn).ok()
                                })
                            })
                            .map(|a| !a.is_empty())
                            .unwrap_or(false);
                        if !has_accounts {
                            let db_path = cache_db_path();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::DarwinImportAll { dir, db_path });
                            self.log.push_back(LogEntry::info(format!(
                                "Auto-importing DARWIN XLSX from {}",
                                self.darwin_xlsx_dir
                            )));
                        } else {
                            self.log.push_back(LogEntry::info(
                                "DARWIN data already imported (use Import button to reimport)",
                            ));
                        }
                    }
                }
                // ── Startup data fetching (disabled for LAN client — server provides all data) ──
                if !self.lan_client_enabled {
                    // Auto MT5SYNC on startup if data dirs are configured
                    {
                        let paths: Vec<String> = self
                            .mt5_db_paths
                            .iter()
                            .filter(|p| !p.is_empty() && std::path::Path::new(p.as_str()).exists())
                            .cloned()
                            .collect();
                        if !paths.is_empty() {
                            let _ = self.broker_tx.send(BrokerCmd::Mt5Sync {
                                sources: paths.clone(),
                                enabled_timeframes: self.enabled_standard_sync_timeframes(),
                            });
                            self.log.push_back(LogEntry::info(format!(
                                "Auto MT5SYNC: {} sources",
                                paths.len()
                            )));
                        }
                    }
                    // Auto SEC scrape on startup
                    {
                        let db_path = cache_db_path();
                        let _ = self.broker_tx.send(BrokerCmd::SecScrape { db_path });
                        self.log
                            .push_back(LogEntry::info("SEC EDGAR scrape started..."));
                    }
                    // Auto EVSCRAPE on startup (fundamentals, skips if updated <24h)
                    {
                        let db_path = cache_db_path();
                        let _ = self.broker_tx.send(BrokerCmd::FundamentalsScrape {
                            db_path,
                            use_mt5: self.fund_source_mt5,
                            use_alpaca: self.fund_source_alpaca,
                            use_tastytrade: self.fund_source_tastytrade,
                            use_kraken: self.fund_source_kraken,
                            force: false,
                        });
                        self.log.push_back(LogEntry::info(
                            "Fundamentals scrape started for all MT5 symbols...",
                        ));
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

        // ── drain background DARWIN data ─────────────────────────────────
        while let Ok(data) = self.bg_rx.try_recv() {
            // Auto-populate darwinex_radar_data from BG-loaded specs so Darwinex scope
            // filtering works without requiring manual DARWINEXRADAR command.
            if !data.darwinex_specs.is_empty() {
                self.darwinex_radar_data = data.darwinex_specs.clone();
            }
            self.bg = data;
            self.bg_rev = self.bg_rev.wrapping_add(1);
        }

        // ── LAN client: load server's broker positions/account from KV cache ──
        // The server stores broker:account/positions/orders to KV on every update.
        // LAN sync's 15s incremental KV sync delivers them to the client's cache.
        // Reload every ~5s (200 frames at 250ms idle repaint) for near-live updates.
        // LAN client: reload positions/orders from server KV.
        // Check every ~5s (200 frames). Cheap local SQLite read — only deserializes
        // when KV actually changed (server writes only on position/order updates).
        if self.lan_sync_mode == "client" {
            if self.frame_count % 200 == 0
                || (self.live_positions.is_empty() && self.frame_count > 10)
            {
                if let Some(ref cache) = self.cache {
                    if let Ok(Some(json)) = cache.get_kv("broker:positions") {
                        if let Ok(pos) = serde_json::from_str::<Vec<PositionInfo>>(&json) {
                            self.live_positions = pos;
                        }
                    }
                    if let Ok(Some(json)) = cache.get_kv("broker:tt_positions") {
                        if let Ok(pos) = serde_json::from_str::<Vec<PositionInfo>>(&json) {
                            self.tt_positions = pos;
                        }
                    }
                    if let Ok(Some(json)) = cache.get_kv("broker:kr_positions") {
                        if let Ok(mut pos) = serde_json::from_str::<Vec<PositionInfo>>(&json) {
                            pos.retain(|p| {
                                p.asset_class != "crypto_spot" && !p.asset_id.starts_with("spot:")
                            });
                            self.kr_positions = pos;
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
                    // DARWIN open positions: read server's computed positions from KV.
                    // This bypasses the broken recompute-from-45K-deals pipeline —
                    // the server already computed the correct 3 positions, stored as KV.
                    if let Ok(Some(json)) = cache.get_kv("darwin:open_positions") {
                        if let Ok(pos) =
                            serde_json::from_str::<Vec<darwin::PortfolioOpenPosition>>(&json)
                        {
                            self.bg.open_positions = pos;
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

        // ── deferred chart loading: non-blocking, one attempt per frame ──
        // Uses try_load() which returns false if cache Mutex is contended (compaction, MT5 sync).
        // Failed loads go back to the queue and retry next frame — UI never blocks.
        if !self.deferred_chart_loads.is_empty() {
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
                if let Some(done_idx) = self.deferred_chart_loads.pop_front() {
                    self.deferred_chart_load_set.remove(&done_idx);
                }
            }
            // If !loaded, leave in queue — will retry next frame when Mutex is free
        }

        // ── recompute indicators when periods changed in UI ──────────────
        if self.user_interacting {
            // Skip expensive recomputes during interaction to protect frame rate
        }
        if self.indicators_dirty && !self.user_interacting {
            self.indicators_dirty = false;
            let mut gpu = self.gpu_indicators.take();
            if let Some(chart) = self.charts.get_mut(self.active_tab) {
                chart.compute_indicators_gpu(gpu.as_mut());
            }
            self.gpu_indicators = gpu;
        }

        // ── receive MTF grid status from background thread (non-blocking) ──
        if let Some(ref rx) = self.mtf_grid_rx {
            if let Ok(results) = rx.try_recv() {
                // Merge with any preloaded data already in mtf_grid_status
                self.mtf_grid_status.extend(results);
                self.mtf_grid_status.sort_by_key(|r| match r.0 {
                    "M1" => 0,
                    "M5" => 1,
                    "M15" => 2,
                    "M30" => 3,
                    "H1" => 4,
                    "H4" => 5,
                    "D1" => 6,
                    "W1" => 7,
                    "MN1" => 8,
                    _ => 99u8,
                });
                self.mtf_grid_rx = None; // done
            }
        }

        // ── receive Darwinex web scrape results (ADR-093) ─────────────
        if let Some(ref rx) = self.dwx_rx {
            if let Ok((result, driver_handle)) = rx.try_recv() {
                // Store the WebDriver handle if provided (from DWXLOGIN)
                if let Some(driver_arc) = driver_handle {
                    self.dwx_driver = Some(driver_arc);
                }
                match result {
                    Ok(update) => {
                        let n_snap = update.snapshots.len();
                        let n_corr = update.correlations.len();
                        let n_alerts = update.correlation_alerts.len();
                        // Log excluded DARWINs
                        for snap in &update.snapshots {
                            if snap.excluded {
                                self.log.push_back(LogEntry::warn(format!(
                                    "DARWIN {} EXCLUDED: {}",
                                    snap.ticker,
                                    if snap.exclusion_reason.is_empty() {
                                        "no reason"
                                    } else {
                                        &snap.exclusion_reason
                                    }
                                )));
                            }
                        }
                        // Log correlation alerts
                        for alert in &update.correlation_alerts {
                            self.log.push_back(LogEntry::err(format!(
                                "CORRELATION BREACH: {} × {} = {:.4} — {}",
                                alert.darwin_a, alert.darwin_b, alert.correlation, alert.suggestion
                            )));
                            // ADR-094: Toast for each correlation breach
                            self.toasts.push(Toast {
                                message: format!(
                                    "{} × {} = {:.4}",
                                    alert.darwin_a, alert.darwin_b, alert.correlation
                                ),
                                color: egui::Color32::from_rgb(255, 80, 80),
                                created: std::time::Instant::now(),
                                duration: std::time::Duration::from_secs(60),
                                dismissable: true,
                                dismissed: false,
                            });
                        }
                        let n_monthly = update.monthly_returns.len();
                        let n_alloc = update.allocations.len();
                        let has_perf = update.portfolio_performance.is_some();
                        let has_risk = update.portfolio_risk.is_some();
                        self.dwx_last_update = Some(update);
                        self.dwx_logged_in = true;
                        self.log.push_back(LogEntry::info(format!(
                            "DWX scrape complete: {} DARWINs (all tabs), {} correlations, {} alerts, {} allocations{}{}",
                            n_snap, n_corr, n_alerts, n_alloc,
                            if has_perf { ", portfolio perf" } else { "" },
                            if has_risk { ", portfolio risk" } else { "" },
                        )));
                        if n_monthly > 0 {
                            self.log.push_back(LogEntry::info(format!(
                                "  Monthly returns: {n_monthly}, equity curves: {n_snap}, VaR histories: {n_snap}, D-Score histories: {n_snap}, investor flows: {n_snap}"
                            )));
                        }
                    }
                    Err(e) => {
                        self.log
                            .push_back(LogEntry::err(format!("DWX scrape failed: {}", e)));
                    }
                }
                self.dwx_rx = None;
            }
        }

        // ── poll async broker messages ───────────────────────────────────
        // Cap drain per frame so a flood of messages can't stall the render thread.
        // Anything left over waits for next frame; we repaint immediately in that case.
        let mut msgs_drained = 0usize;
        const BROKER_DRAIN_MAX: usize = 128;
        while msgs_drained < BROKER_DRAIN_MAX
            && let Ok(msg) = self.broker_rx.try_recv()
        {
            msgs_drained += 1;
            match msg {
                BrokerMsg::Connected(s) => {
                    if s.contains("Kraken") {
                        self.kraken_connected = true;
                        // Auto-fetch trade history when Kraken connects
                        let _ = self.broker_tx.send(BrokerCmd::KrakenFetchTrades);
                        // Auto-fetch open orders before WS deltas start arriving.
                        let _ = self.broker_tx.send(BrokerCmd::KrakenFetchOpenOrders);
                        // Start private WebSocket for real-time ownTrades / openOrders
                        let _ = self.broker_tx.send(BrokerCmd::KrakenStartPrivateWs);
                    } else if s.contains("tastytrade") {
                        self.tt_connected = true;
                        self.tastytrade_universe_symbols.clear();
                        self.tastytrade_universe_requested = false;
                        self.tastytrade_universe_retry_after_ts = 0;
                        self.tastytrade_sync_pause_until_ts = 0;
                        self.tastytrade_sync_pause_reason.clear();
                    } else {
                        self.broker_connected = true;
                        // Auto-fetch positions, orders, and recent fills (Alpaca)
                        let _ = self.broker_tx.send(BrokerCmd::GetPositions);
                        let _ = self.broker_tx.send(BrokerCmd::GetOrders);
                        let _ = self.broker_tx.send(BrokerCmd::GetActivities { limit: 100 });
                        let _ = self.broker_tx.send(BrokerCmd::GetMarketClock);
                    }
                    self.log.push_back(LogEntry::info(s));
                }
                BrokerMsg::KrakenTrades(mut trades) => {
                    trades.sort_by(|a, b| {
                        b.time
                            .partial_cmp(&a.time)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                    if trades.len() > KRAKEN_TRADE_HISTORY_CAP {
                        trades.truncate(KRAKEN_TRADE_HISTORY_CAP);
                    }
                    self.kraken_trades = VecDeque::from(trades);
                    self.rebuild_kraken_trade_indexes();
                    self.refresh_kraken_position_costs();
                    self.log.push_back(LogEntry::info(format!(
                        "Kraken: loaded {} trades; cost basis for {} assets",
                        self.kraken_trades.len(),
                        self.kraken_cost_basis.len()
                    )));
                }
                BrokerMsg::KrakenLiveTrade(trade) => {
                    if self.insert_kraken_live_trade(trade) {
                        self.refresh_kraken_position_costs();
                        // ownTrades is the low-latency fill signal. Reconcile REST snapshots
                        // after fills so balances, position P/L, and open-order state catch up
                        // without waiting for a manual refresh.
                        let _ = self.broker_tx.send(BrokerCmd::KrakenGetBalance);
                        let _ = self.broker_tx.send(BrokerCmd::KrakenGetPositions);
                        let _ = self.broker_tx.send(BrokerCmd::KrakenFetchOpenOrders);
                    }
                }
                BrokerMsg::KrakenOpenOrders(orders) => {
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
                    let text = format!("Kraken WS {status}: {message}");
                    if matches!(status.as_str(), "error" | "closed") {
                        self.log.push_back(LogEntry::warn(text));
                    } else {
                        self.log.push_back(LogEntry::info(text));
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
                    if e.starts_with("Kraken pairs:") {
                        self.kraken_pairs_requested = false;
                    } else if e.starts_with("Kraken futures instruments:") {
                        self.kraken_futures_requested = false;
                    } else if e.starts_with("tastytrade connection unavailable:")
                        || e.starts_with("tastytrade market data unavailable:")
                    {
                        self.tt_connected = false;
                        self.tt_positions.clear();
                        self.tastytrade_universe_symbols.clear();
                        self.tastytrade_universe_requested = false;
                        self.tastytrade_universe_retry_after_ts =
                            now + tastytrade_sync_backoff_secs(&e);
                        self.tastytrade_sync_pause_until_ts =
                            now + tastytrade_sync_backoff_secs(&e);
                        self.tastytrade_sync_pause_reason = e.clone();
                    } else if e.starts_with("tastytrade universe failed:") {
                        self.tastytrade_universe_requested = false;
                        self.tastytrade_universe_retry_after_ts =
                            now + tastytrade_sync_backoff_secs(&e);
                    } else if e.starts_with("DXLink token failed:")
                        || e.starts_with("DXLink stream failed:")
                    {
                        self.tastytrade_sync_pause_until_ts =
                            now + tastytrade_sync_backoff_secs(&e);
                        self.tastytrade_sync_pause_reason = e.clone();
                    }
                    // Disconnect on auth failure to stop error spam
                    if e.contains("401") || e.contains("Unauthorized") || e.contains("403") {
                        if self.broker_connected {
                            self.broker_connected = false;
                            self.log.push_back(LogEntry::err(format!(
                                "{} — disconnected (check API keys in Settings)",
                                e
                            )));
                        }
                        // Don't log repeated auth failures
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
                    self.all_broker_assets = assets;
                    self.all_broker_assets_fetched = true;
                }
                BrokerMsg::RecentFills(fills) => {
                    self.recent_fills = fills;
                    // Invalidate trade overlay cache so fills show immediately
                    for c in &mut self.charts {
                        c.cached_trade_overlay_frame = 0;
                    }
                }
                BrokerMsg::Mt5SyncDone(changed) => {
                    // Reload all visible charts to pick up forming bar updates
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
                BrokerMsg::Mt5LiveQuotes(quotes) => {
                    // Update forming bar (last bar) on all charts from MT5 live bid/ask.
                    // O(1) per quote: pre-build symbol→chart-indices map to avoid O(quotes×charts).
                    // rsplit instead of split+Vec to avoid the intermediate Vec<&str> per chart.
                    let mut sym_to_charts: std::collections::HashMap<String, Vec<usize>> =
                        std::collections::HashMap::with_capacity(self.charts.len());
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
                        sym_to_charts.entry(bare).or_default().push(ci);
                    }
                    for (sym, bid, ask) in &quotes {
                        let mid = (bid + ask) / 2.0;
                        if mid <= 0.0 {
                            continue;
                        }
                        let sym_upper = sym.to_uppercase();
                        if let Some(indices) = sym_to_charts.get(&sym_upper) {
                            for &ci in indices {
                                if let Some(bar) = self.charts[ci].bars.last_mut() {
                                    bar.close = mid;
                                    if mid > bar.high {
                                        bar.high = mid;
                                    }
                                    if mid < bar.low {
                                        bar.low = mid;
                                    }
                                }
                            }
                        }
                    }
                }
                BrokerMsg::Mt5Heartbeat(beats) => {
                    // Merge incoming heartbeats into app state. Each source is keyed
                    // by its DB path; the freshest row wins. `received_at` is stamped
                    // locally so the staleness display works even if the EA clock and
                    // our clock disagree.
                    let now = chrono::Utc::now().timestamp();
                    for (path, json, row_ts) in beats {
                        let entry = self.mt5_heartbeats.iter_mut().find(|h| h.0 == path);
                        match entry {
                            Some(e) => {
                                e.1 = json;
                                e.2 = row_ts;
                                e.3 = now;
                            }
                            None => {
                                self.mt5_heartbeats.push((path, json, row_ts, now));
                            }
                        }
                    }
                    // Writer is alive — run gap detection against current charts
                    // and always re-emit demand.txt. Previously we only wrote
                    // when gaps were pending or burst was active, which meant
                    // a newly-opened chart tab for an already-fresh symbol
                    // never propagated to the EA's demand list → rotation
                    // didn't prioritise it. Rewriting every heartbeat is cheap
                    // (~1KB to /dev/shm) and guarantees the EA sees the
                    // current (sym, tf) set within one demand-refresh cycle.
                    self.detect_mt5_gaps();
                    self.write_mt5_demand_txt();
                }
                BrokerMsg::TastytradePositions(pos) => {
                    self.positions_last_update_ts = chrono::Utc::now().timestamp();
                    let prev_symbols: std::collections::HashSet<String> = self
                        .tt_positions
                        .iter()
                        .filter(|p| p.qty.abs() > 0.0)
                        .map(|p| p.symbol.to_ascii_uppercase())
                        .collect();
                    let next_symbols: std::collections::HashSet<String> = pos
                        .iter()
                        .filter(|p| p.qty.abs() > 0.0)
                        .map(|p| p.symbol.to_ascii_uppercase())
                        .collect();
                    for symbol in prev_symbols.difference(&next_symbols) {
                        let _ = self.broker_tx.send(BrokerCmd::TastytradeCancelLiveExits {
                            symbol: symbol.clone(),
                        });
                    }
                    if let Ok(json) = serde_json::to_string(&pos) {
                        self.put_kv_dedup("broker:tt_positions", &json);
                    }
                    self.tt_positions = pos;
                }
                BrokerMsg::TastytradeBalances(bal) => {
                    self.tt_balances = Some(bal);
                }
                BrokerMsg::KrakenPositions(mut pos) => {
                    self.positions_last_update_ts = chrono::Utc::now().timestamp();
                    pos.retain(|p| {
                        p.asset_class != "crypto_spot" && !p.asset_id.starts_with("spot:")
                    });
                    if let Ok(json) = serde_json::to_string(&pos) {
                        self.put_kv_dedup("broker:kr_positions", &json);
                    }
                    self.kr_positions = pos;
                    self.refresh_kraken_position_costs();
                }
                BrokerMsg::Orders(orders) => {
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
                    if is_trade && self.broker_connected {
                        let _ = self.broker_tx.send(BrokerCmd::GetPositions);
                        let _ = self.broker_tx.send(BrokerCmd::GetOrders);
                    }
                    if is_trade
                        && self.tt_connected
                        && msg.to_ascii_lowercase().contains("tastytrade")
                    {
                        let _ = self.broker_tx.send(BrokerCmd::TastytradePositions);
                        let _ = self.broker_tx.send(BrokerCmd::TastytradeGetBalances);
                    }
                    if is_trade
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
                BrokerMsg::WatchlistQuotes(rows) => {
                    self.watchlist_last_update_ts = chrono::Utc::now().timestamp();
                    // Store to KV for LAN clients — dedup to avoid timestamp churn
                    if let Ok(j) = serde_json::to_string(&rows) {
                        self.put_kv_dedup("broker:watchlist", &j);
                    }
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
                    for row in &rows {
                        if row.last <= 0.0 {
                            continue;
                        }
                        let row_sym = row.symbol.replace('/', "").to_uppercase();
                        // Fast path: exact match via HashMap
                        let mut matched_indices: Vec<usize> = Vec::new();
                        if let Some(indices) = wl_sym_to_charts.get(&row_sym) {
                            matched_indices.extend(indices);
                        }
                        // Slow path fallback: partial contains match (rare — only for symbols like "BTCUSD" matching "BTC")
                        for (ci, bare) in wl_chart_bares.iter().enumerate() {
                            if !matched_indices.contains(&ci)
                                && (bare.contains(&row_sym) || row_sym.contains(bare.as_str()))
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
                    // O(1) per row: static HashSets for symbol classification.
                    if self.show_world_indices || self.show_forex_matrix {
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
                }
                BrokerMsg::CryptoTop50(data) => {
                    self.log.push_back(LogEntry::info(format!(
                        "CoinGecko: {} coins loaded",
                        data.len()
                    )));
                    self.crypto_top50 = data;
                }
                BrokerMsg::KrakenBalances(balances) => {
                    self.kraken_balances = balances;
                    let active_tf = self
                        .charts
                        .get(self.active_tab)
                        .map(|chart| chart.timeframe.cache_suffix())
                        .unwrap_or("1Day");
                    let mut queued = 0usize;
                    let balance_pairs: Vec<String> = self
                        .kraken_balances
                        .iter()
                        .filter(|(asset, qty)| {
                            qty.is_finite()
                                && *qty > 0.0
                                && !Self::kraken_is_cash_balance_asset(asset)
                        })
                        .map(|(asset, _)| Self::kraken_spot_pair_for_balance_asset(asset))
                        .collect();
                    for pair in balance_pairs {
                        if self.queue_kraken_fetch(&pair, active_tf) {
                            queued += 1;
                        }
                        if active_tf != "1Day" && self.queue_kraken_fetch(&pair, "1Day") {
                            queued += 1;
                        }
                    }
                    let _ = self.broker_tx.send(BrokerCmd::KrakenFetchTrades);
                    self.log.push_back(LogEntry::info(format!(
                        "Kraken: {} assets with balance; queued {} Kraken bar fetches",
                        self.kraken_balances.len(),
                        queued
                    )));
                }
                BrokerMsg::KrakenPairs(pairs) => {
                    self.log.push_back(LogEntry::info(format!(
                        "Kraken: {} tradeable pairs loaded",
                        pairs.len()
                    )));
                    self.kraken_pairs_requested = true;
                    self.kraken_pairs = pairs;
                    self.refill_market_data_sync_slots();
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
                BrokerMsg::TastytradeUniverse(symbols) => {
                    self.log.push_back(LogEntry::info(format!(
                        "tastytrade: {} market-data symbols loaded",
                        symbols.len()
                    )));
                    self.tastytrade_universe_requested = true;
                    self.tastytrade_universe_retry_after_ts = 0;
                    self.tastytrade_universe_symbols = symbols;
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
                // ── ADR-113 Round 6 receive arms ──
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
                // ── ADR-114 Godel Parity Round 7 ──
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
                // ── ADR-115 Round 8 receive arms ──
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
                // ── ADR-116 Round 9 receive arms ──
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
                // ── ADR-117 Godel Parity Round 10 ──
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
                // ── ADR-118 Godel Parity Round 11 ──
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
                // ── ADR-119 Godel Parity Round 12 ──
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
                // ── ADR-123 Round 16 ─────────────────────────────────────────
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
                // ── ADR-124 Round 17 ──
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
                // ── ADR-125 Round 18 ──
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
                // ── ADR-134 Round 26 receive ──
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
                // ── ADR-135 Round 27 receive ──
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
                // ── ADR-136 Round 28 receive ──
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
                // ── ADR-137 Round 29 receive ──
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
                // ── ADR-138 Round 30 receive ──
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
                // ── ADR-139 Round 31 receive ──
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
                // ── ADR-140 Round 32 receive ──
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
                // ── ADR-141 Round 33 receive ──
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
                // ── ADR-142 Round 34 receive ──
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
                // ── ADR-149 Round 40 receive ──
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
                // ── ADR-151 Round 42 receive arms ──
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
                // ── ADR-153 Round 44 receive arms ──
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
                // ── ADR-154 Round 45 receive arms ──
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
                // ── ADR-155 Round 46 receive arms ──
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
                // ── ADR-156 Round 47 receive arms ──
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
                // ── ADR-161 Round 51 result handlers ──
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
                // ── ADR-163 Round 52 result handlers ──
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
                // ── ADR-167 Round 55 receive arms ──
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
                // ── ADR-174 Round 62 match arms ──
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
                // ── ADR-175 Round 63 match arms ──
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
                // ── ADR-176 Round 64 match arms ──
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
                    // Clear selection if the selected index is now out of range.
                    if let Some(idx) = self.news_selected {
                        if idx >= self.news_full_articles.len() {
                            self.news_selected = None;
                        }
                    }
                    if self.news_selected.is_none() && !self.news_full_articles.is_empty() {
                        self.news_selected = Some(0);
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
                    } else if msg.contains("complete") || msg.contains("Aborting") {
                        self.scrape_fund_running = false;
                        // Parse final counts from "X OK, Y failed, Z skipped ... out of N"
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
                    self.log.push_back(LogEntry::info(msg.clone()));
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
                BrokerMsg::DarwinFtpScanResult(results) => {
                    self.scrape_darwin_running = false;
                    self.scrape_darwin_last_msg = format!("{} DARWINs scanned", results.len());
                    self.log.push_back(LogEntry::info(format!(
                        "DARWIN FTP: {} results loaded",
                        results.len()
                    )));
                    self.ftp_scan_results = results;
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
                        "tastytrade" => "tastytrade",
                        "mt5" => "MT5",
                        "kraken" => "Kraken",
                        "kraken-futures" => "Kraken Futures",
                        _ => source.as_str(),
                    };
                    let log_msg = if should_reload {
                        format!(
                            "{} fetched {} bars for {} {} — queued active chart reload",
                            source_label, count, symbol, timeframe
                        )
                    } else {
                        format!(
                            "{} fetched {} bars for {} {}",
                            source_label, count, symbol, timeframe
                        )
                    };
                    self.log.push_back(LogEntry::info(log_msg));
                    let source_has_terminal_settlement = matches!(
                        source.as_str(),
                        "alpaca" | "kraken" | "kraken-futures" | "tastytrade"
                    );
                    if !source_has_terminal_settlement {
                        self.settle_market_data_fetch(&source, &symbol, &timeframe);
                    }
                    if source_has_terminal_settlement {
                        self.note_cached_sync_success(&source, &symbol, &timeframe, count);
                    }
                    if source == "tastytrade" {
                        self.tastytrade_sync_pause_until_ts = 0;
                        self.tastytrade_sync_pause_reason.clear();
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
                        && matches!(source.as_str(), "kraken" | "kraken-futures" | "tastytrade")
                    {
                        self.refill_market_data_sync_slots();
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
                        self.refill_market_data_sync_slots();
                    }
                }
                BrokerMsg::KrakenFetchSettled { symbol, timeframe } => {
                    self.settle_market_data_fetch("kraken", &symbol, &timeframe);
                    self.refill_market_data_sync_slots();
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
                        self.log.push_back(LogEntry::info(format!(
                            "Kraken {} {}: marked backfill-complete at {}/{} bars — full history exhausted; automated sync will keep it current ({} marked)",
                            symbol, timeframe, bar_count, target_bars, marker_count
                        )));
                    }
                }
                BrokerMsg::KrakenFuturesFetchSettled { symbol, timeframe } => {
                    self.settle_market_data_fetch("kraken-futures", &symbol, &timeframe);
                    self.refill_market_data_sync_slots();
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
                BrokerMsg::TastytradeFetchSettled { symbol, timeframe } => {
                    self.settle_market_data_fetch("tastytrade", &symbol, &timeframe);
                    self.refill_market_data_sync_slots();
                }
                BrokerMsg::TastytradeBackfillComplete {
                    symbol,
                    timeframe,
                    bar_count,
                    target_bars,
                } => {
                    let changed = self.tastytrade_backfill_complete_mark(
                        &symbol,
                        &timeframe,
                        bar_count,
                        target_bars,
                    );
                    if changed {
                        let marker_count = self.tastytrade_backfill_complete_pairs.len();
                        self.log.push_back(LogEntry::info(format!(
                            "tastytrade {} {}: marked backfill-complete at {}/{} bars — full history exhausted; automated sync will keep it current ({} marked)",
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
                    self.log.push_back(LogEntry::warn(format!(
                        "Alpaca {} {}: queued for retry ({}) — {} in queue",
                        symbol,
                        timeframe,
                        reason,
                        self.alpaca_retry_queue.len()
                    )));
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
                    self.log.push_back(LogEntry::warn(format!(
                        "Alpaca {} {}: {} — automated sync will skip it ({} marked)",
                        symbol, timeframe, prefix, marker_count
                    )));
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
                        self.log.push_back(LogEntry::info(format!(
                            "Alpaca {} {}: marked backfill-complete at {}/{} bars — full history exhausted; automated sync will keep it current ({} marked)",
                            symbol, timeframe, bar_count, target_bars, marker_count
                        )));
                    }
                }
                BrokerMsg::DarwinFtpReturns(returns_data) => {
                    self.log.push_back(LogEntry::info(format!(
                        "GPU: uploading {} DARWINs to VRAM...",
                        returns_data.len()
                    )));
                    if let Some(ref mut gpu) = self.gpu_darwin {
                        let max_days =
                            returns_data.iter().map(|(_, r)| r.len()).max().unwrap_or(0) as u32;
                        let tickers: Vec<String> =
                            returns_data.iter().map(|(t, _)| t.clone()).collect();
                        let series: Vec<Vec<f32>> =
                            returns_data.into_iter().map(|(_, r)| r).collect();
                        gpu.upload_returns(&series, max_days);
                        if let Some(stats) = gpu.compute_all_batches() {
                            self.log.push_back(LogEntry::info(format!(
                                "GPU: {} DARWIN stats computed",
                                stats.len()
                            )));
                            self.ftp_scan_results.clear();
                            for (i, s) in stats.iter().enumerate() {
                                if i < tickers.len() {
                                    self.ftp_scan_results.push(darwin_ftp::DarwinFtpSummary {
                                        ticker: tickers[i].clone(),
                                        trading_days: 0,
                                        total_return_pct: s.total_return as f64 * 100.0,
                                        max_drawdown_pct: s.max_drawdown as f64 * 100.0,
                                        sharpe: s.sharpe as f64,
                                        sortino: s.sortino as f64,
                                        daily_vol: s.variance.sqrt() as f64,
                                        best_day_pct: s.best_day as f64 * 100.0,
                                        worst_day_pct: s.worst_day as f64 * 100.0,
                                        last_quote: 0.0,
                                        has_dscore: false,
                                        has_quotes: false,
                                        has_former_var10: false,
                                        experience_score: 0.0,
                                        risk_stability_score: 0.0,
                                        performance_score: 0.0,
                                    });
                                }
                            }
                            self.ftp_scan_results.sort_by(|a, b| {
                                b.sharpe
                                    .partial_cmp(&a.sharpe)
                                    .unwrap_or(std::cmp::Ordering::Equal)
                            });
                            self.log.push_back(LogEntry::info(format!(
                                "GPU scan complete: {} DARWINs ranked by Sharpe",
                                self.ftp_scan_results.len()
                            )));
                        }
                    } else {
                        self.log
                            .push_back(LogEntry::warn("GPU not available — cannot compute stats"));
                    }
                }
            }
        }
        // If we hit the drain cap there are more messages waiting — repaint
        // immediately to process the next batch rather than waiting on the idle tick.
        if msgs_drained >= BROKER_DRAIN_MAX {
            ctx.request_repaint();
        }

        // ── drain web client commands ────────────────────────────────────
        if let Some(ref mut rx) = self.web_cmd_rx {
            let mut web_cmds_drained = 0usize;
            const WEB_CMD_DRAIN_MAX: usize = 64;
            while web_cmds_drained < WEB_CMD_DRAIN_MAX {
                let Ok(cmd) = rx.try_recv() else {
                    break;
                };
                web_cmds_drained += 1;
                match cmd {
                    typhoon_web_protocol::WebCmd::GetAccount => {
                        let _ = self.broker_tx.send(BrokerCmd::GetAccount);
                    }
                    typhoon_web_protocol::WebCmd::GetPositions => {
                        let _ = self.broker_tx.send(BrokerCmd::GetPositions);
                    }
                    typhoon_web_protocol::WebCmd::GetOrders => {
                        let _ = self.broker_tx.send(BrokerCmd::GetOrders);
                    }
                    typhoon_web_protocol::WebCmd::GetWatchlistQuotes { symbols } => {
                        let _ = self
                            .broker_tx
                            .send(BrokerCmd::GetWatchlistQuotes { symbols });
                    }
                    typhoon_web_protocol::WebCmd::GetMarketClock => {
                        let _ = self.broker_tx.send(BrokerCmd::GetMarketClock);
                    }
                    typhoon_web_protocol::WebCmd::GetBars { symbol, timeframe } => {
                        // Read bars directly from cache and broadcast
                        if let Some(ref cache) = self.cache {
                            let key = format!("mt5:{}:{}", symbol, timeframe);
                            if let Ok(Some(data)) = cache.get_bars_raw(&key) {
                                let bars: Vec<typhoon_web_protocol::BarData> = data
                                    .iter()
                                    .map(|b| typhoon_web_protocol::BarData {
                                        timestamp: b.0,
                                        open: b.1,
                                        high: b.2,
                                        low: b.3,
                                        close: b.4,
                                        volume: b.5,
                                    })
                                    .collect();
                                if let Some(ref tx) = self.web_msg_tx {
                                    let _ = tx.send(typhoon_web_protocol::WebMsg::Bars {
                                        symbol,
                                        timeframe,
                                        bars,
                                    });
                                }
                            }
                        }
                    }
                    typhoon_web_protocol::WebCmd::Ping => {
                        if let Some(ref tx) = self.web_msg_tx {
                            let _ = tx.send(typhoon_web_protocol::WebMsg::Pong);
                        }
                    }
                    typhoon_web_protocol::WebCmd::Auth { .. } => {
                        // Auth is handled by web-server before relay — ignore here
                    }
                    // ── Phase 2: order entry from phone ──
                    // Server-side validation already happened in web-server.
                    // We still confirm the broker selection matches a connected broker,
                    // translate to the native BrokerCmd, and reply via the broadcast channel.
                    typhoon_web_protocol::WebCmd::PlaceOrder {
                        symbol,
                        qty,
                        side,
                        order_type,
                        limit_price,
                        stop_price,
                        take_profit,
                        stop_loss,
                        broker,
                        ..
                    } => {
                        let lower_side = side.to_ascii_lowercase();
                        let lower_type = order_type.trim().replace('-', "_").to_ascii_lowercase();
                        let broker_key = broker.to_ascii_lowercase();
                        let reply = match broker_key.as_str() {
                            "alpaca" => {
                                if !self.broker_connected {
                                    typhoon_web_protocol::WebMsg::OrderResult {
                                        ok: false,
                                        message: "Alpaca broker not connected on host".into(),
                                    }
                                } else {
                                    // Dispatch based on order_type
                                    match lower_type.as_str() {
                                        "market" => {
                                            let _ =
                                                self.broker_tx.send(BrokerCmd::AlpacaMarketOrder {
                                                    symbol: symbol.clone(),
                                                    qty,
                                                    side: lower_side.clone(),
                                                });
                                        }
                                        "limit" => {
                                            let lp = limit_price.unwrap_or(0.0);
                                            let _ =
                                                self.broker_tx.send(BrokerCmd::AlpacaLimitOrder {
                                                    symbol: symbol.clone(),
                                                    qty,
                                                    side: lower_side.clone(),
                                                    limit_price: lp,
                                                });
                                        }
                                        "stop" => {
                                            let sp = stop_price.unwrap_or(0.0);
                                            let _ =
                                                self.broker_tx.send(BrokerCmd::AlpacaStopOrder {
                                                    symbol: symbol.clone(),
                                                    qty,
                                                    side: lower_side.clone(),
                                                    stop_price: sp,
                                                });
                                        }
                                        _ => {}
                                    }
                                    typhoon_web_protocol::WebMsg::OrderResult {
                                        ok: true,
                                        message: format!(
                                            "{} {} {} {} dispatched to Alpaca",
                                            lower_side, qty, symbol, lower_type
                                        ),
                                    }
                                }
                            }
                            "tastytrade" => {
                                let _ = self.broker_tx.send(BrokerCmd::TastytradeEquityOrder {
                                    symbol: symbol.clone(),
                                    qty: qty as i64,
                                    side: if lower_side == "buy" {
                                        "Buy to Open"
                                    } else {
                                        "Sell to Open"
                                    }
                                    .into(),
                                    order_type: if lower_type == "market" {
                                        "Market"
                                    } else {
                                        "Limit"
                                    }
                                    .into(),
                                    price: limit_price,
                                });
                                typhoon_web_protocol::WebMsg::OrderResult {
                                    ok: true,
                                    message: format!(
                                        "{} {} {} {} dispatched to Tastytrade",
                                        lower_side, qty, symbol, lower_type
                                    ),
                                }
                            }
                            "kraken" => {
                                if !self.kraken_connected {
                                    typhoon_web_protocol::WebMsg::OrderResult {
                                        ok: false,
                                        message: "Kraken broker not connected on host".into(),
                                    }
                                } else {
                                    let kraken_type = match lower_type.as_str() {
                                        "market" => "market",
                                        "limit" => "limit",
                                        "stop" | "stoploss" | "stop_loss" => "stop-loss",
                                        "stoplimit" | "stop_limit" | "stoploss_limit"
                                        | "stop_loss_limit" => "stop-loss-limit",
                                        "takeprofit" | "take_profit" => "take-profit",
                                        "takeprofit_limit" | "take_profit_limit" => {
                                            "take-profit-limit"
                                        }
                                        "trailingstop" | "trailing_stop" => "trailing-stop",
                                        "trailingstop_limit" | "trailing_stop_limit" => {
                                            "trailing-stop-limit"
                                        }
                                        "iceberg" => "iceberg",
                                        "settle_position" => "settle-position",
                                        _ => lower_type.as_str(),
                                    };
                                    let mut order =
                                        typhoon_engine::broker::kraken_broker::KrakenOrderRequest::basic(
                                            symbol.clone(),
                                            lower_side.clone(),
                                            kraken_type,
                                            qty,
                                        );
                                    let primary_price = match kraken_type {
                                        "limit" | "iceberg" => limit_price,
                                        "stop-loss" | "take-profit" | "trailing-stop" => {
                                            stop_price.or(limit_price)
                                        }
                                        "stop-loss-limit"
                                        | "take-profit-limit"
                                        | "trailing-stop-limit" => stop_price,
                                        _ => None,
                                    };
                                    if let Some(price) = primary_price {
                                        order.price = Some(price.to_string());
                                    }
                                    if matches!(
                                        kraken_type,
                                        "stop-loss-limit"
                                            | "take-profit-limit"
                                            | "trailing-stop-limit"
                                    ) && let Some(price2) = limit_price
                                    {
                                        order.price2 = Some(price2.to_string());
                                    }
                                    match order.validate() {
                                        Ok(()) => {
                                            let _ = self.broker_tx.send(
                                                BrokerCmd::KrakenPlaceOrderAdvanced { order },
                                            );
                                            if stop_loss.is_some() || take_profit.is_some() {
                                                let _ = self.broker_tx.send(
                                                    BrokerCmd::KrakenSyncExits {
                                                        pair: symbol.clone(),
                                                        sl_price: stop_loss,
                                                        tp_price: take_profit,
                                                        wait_for_position: true,
                                                        wait_for_qty_at_most: None,
                                                    },
                                                );
                                            }
                                            typhoon_web_protocol::WebMsg::OrderResult {
                                                ok: true,
                                                message: format!(
                                                    "{} {} {} {} dispatched to Kraken",
                                                    lower_side, qty, symbol, kraken_type
                                                ),
                                            }
                                        }
                                        Err(e) => typhoon_web_protocol::WebMsg::OrderResult {
                                            ok: false,
                                            message: format!("Kraken order rejected locally: {e}"),
                                        },
                                    }
                                }
                            }
                            other => typhoon_web_protocol::WebMsg::OrderResult {
                                ok: false,
                                message: format!("Unknown broker: {other}"),
                            },
                        };
                        if let Some(ref tx) = self.web_msg_tx {
                            let _ = tx.send(reply);
                        }
                        // Mirror to local log so the host operator sees web-originated orders.
                        self.log.push_back(LogEntry::info(format!(
                            "Web order: {} {} {} {} via {}",
                            side, qty, symbol, order_type, broker
                        )));
                    }
                    typhoon_web_protocol::WebCmd::CancelOrder { order_id, broker } => {
                        let broker_key = broker.to_ascii_lowercase();
                        let reply = match broker_key.as_str() {
                            "alpaca" => {
                                let _ = self.broker_tx.send(BrokerCmd::AlpacaCancelOrder {
                                    order_id: order_id.clone(),
                                });
                                typhoon_web_protocol::WebMsg::OrderResult {
                                    ok: true,
                                    message: format!("Cancel {} dispatched to Alpaca", order_id),
                                }
                            }
                            "tastytrade" => {
                                let _ = self.broker_tx.send(BrokerCmd::TastytradeCancelOrder {
                                    order_id: order_id.clone(),
                                });
                                typhoon_web_protocol::WebMsg::OrderResult {
                                    ok: true,
                                    message: format!(
                                        "Cancel {} dispatched to tastytrade",
                                        order_id
                                    ),
                                }
                            }
                            "kraken" => {
                                let _ = self.broker_tx.send(BrokerCmd::KrakenCancelOrder {
                                    txid: order_id.clone(),
                                });
                                typhoon_web_protocol::WebMsg::OrderResult {
                                    ok: true,
                                    message: format!("Cancel {} dispatched to Kraken", order_id),
                                }
                            }
                            other => typhoon_web_protocol::WebMsg::OrderResult {
                                ok: false,
                                message: format!("Unknown broker: {other}"),
                            },
                        };
                        if let Some(ref tx) = self.web_msg_tx {
                            let _ = tx.send(reply);
                        }
                        self.log.push_back(LogEntry::info(format!(
                            "Web cancel: {} via {}",
                            order_id, broker
                        )));
                    }
                    typhoon_web_protocol::WebCmd::ClosePosition { symbol, broker } => {
                        let broker_key = broker.to_ascii_lowercase();
                        let reply = match broker_key.as_str() {
                            "alpaca" => {
                                let _ = self.broker_tx.send(BrokerCmd::ClosePosition {
                                    symbol: symbol.clone(),
                                    qty: None,
                                });
                                typhoon_web_protocol::WebMsg::OrderResult {
                                    ok: true,
                                    message: format!("Close {} dispatched to Alpaca", symbol),
                                }
                            }
                            "tastytrade" => {
                                let _ =
                                    self.broker_tx.send(BrokerCmd::TastytradeClosePositionQty {
                                        symbol: symbol.clone(),
                                        qty: None,
                                    });
                                typhoon_web_protocol::WebMsg::OrderResult {
                                    ok: true,
                                    message: format!("Close {} dispatched to Tastytrade", symbol),
                                }
                            }
                            "kraken" => {
                                let _ = self.broker_tx.send(BrokerCmd::KrakenClosePosition {
                                    pair: symbol.clone(),
                                    volume: None,
                                });
                                typhoon_web_protocol::WebMsg::OrderResult {
                                    ok: true,
                                    message: format!("Close {} dispatched to Kraken", symbol),
                                }
                            }
                            other => typhoon_web_protocol::WebMsg::OrderResult {
                                ok: false,
                                message: format!("Unknown broker: {other}"),
                            },
                        };
                        if let Some(ref tx) = self.web_msg_tx {
                            let _ = tx.send(reply);
                        }
                        self.log.push_back(LogEntry::info(format!(
                            "Web close: {} via {}",
                            symbol, broker
                        )));
                    }
                    // ── ADR-092: new WebCmd handlers ──
                    typhoon_web_protocol::WebCmd::GetIndicators {
                        symbol,
                        timeframe,
                        indicators,
                    } => {
                        self.log.push_back(LogEntry::info(format!(
                            "Web indicator request: {symbol} {timeframe} {:?}",
                            indicators
                        )));
                        // Send indicator data from current chart cache
                        // Full GPU dispatch integration will be wired in GPU compute task
                        for name in &indicators {
                            if let Some(ref tx) = self.web_msg_tx {
                                let _ = tx.send(typhoon_web_protocol::WebMsg::IndicatorData {
                                    symbol: symbol.clone(),
                                    timeframe: timeframe.clone(),
                                    name: name.clone(),
                                    values: Vec::new(),
                                });
                            }
                        }
                    }
                    typhoon_web_protocol::WebCmd::CreateAlert {
                        symbol,
                        condition: _,
                        price,
                        message,
                    } => {
                        self.log
                            .push_back(LogEntry::info(format!("Web alert: {symbol} @ {price}")));
                        let label = if message.is_empty() { symbol } else { message };
                        self.alerts.push((price, label));
                    }
                    typhoon_web_protocol::WebCmd::DeleteAlert { alert_id } => {
                        // alert_id is index-based from web: "web-N"
                        if let Some(idx) = alert_id
                            .strip_prefix("web-")
                            .and_then(|s| s.parse::<usize>().ok())
                        {
                            if idx < self.alerts.len() {
                                self.alerts.remove(idx);
                            }
                        }
                    }
                    typhoon_web_protocol::WebCmd::ListAlerts => {
                        if let Some(ref tx) = self.web_msg_tx {
                            let _ = tx.send(typhoon_web_protocol::WebMsg::AlertList {
                                items: self
                                    .alerts
                                    .iter()
                                    .enumerate()
                                    .map(|(i, (price, label))| {
                                        typhoon_web_protocol::AlertSnapshot {
                                            id: format!("web-{i}"),
                                            symbol: label.clone(),
                                            condition: "reaches".into(),
                                            price: *price,
                                            message: label.clone(),
                                            active: true,
                                        }
                                    })
                                    .collect(),
                            });
                        }
                    }
                    typhoon_web_protocol::WebCmd::GetNews { symbol } => {
                        self.log
                            .push_back(LogEntry::info(format!("Web news: {:?}", symbol)));
                        if let Some(ref tx) = self.web_msg_tx {
                            let items: Vec<typhoon_web_protocol::NewsItem> = self
                                .news_articles
                                .iter()
                                .filter(|n| {
                                    symbol
                                        .as_ref()
                                        .map(|s| n.0.contains(s) || n.1.contains(s))
                                        .unwrap_or(true)
                                })
                                .take(typhoon_web_protocol::MAX_NEWS_ITEMS)
                                .map(|n| typhoon_web_protocol::NewsItem {
                                    headline: n.0.clone(),
                                    source: n.2.clone(),
                                    url: n.1.clone(),
                                    symbol: symbol.clone(),
                                    timestamp: 0,
                                    summary: String::new(),
                                })
                                .collect();
                            let _ = tx.send(typhoon_web_protocol::WebMsg::NewsFeed { items });
                        }
                    }
                    typhoon_web_protocol::WebCmd::Subscribe { symbol, timeframe } => {
                        self.log.push_back(LogEntry::info(format!(
                            "Web subscribe: {symbol}:{timeframe}"
                        )));
                    }
                    typhoon_web_protocol::WebCmd::Unsubscribe { .. } => {}
                    typhoon_web_protocol::WebCmd::GetDarwinWeb { ticker } => {
                        // Return cached DWX web data to web client
                        if let Some(ref update) = self.dwx_last_update {
                            let ticker_filter = ticker.as_ref().map(|t| t.to_uppercase());
                            let matches_ticker =
                                |t: &str| ticker_filter.as_ref().map_or(true, |f| t == f);

                            let snapshots: Vec<typhoon_web_protocol::DarwinWebSnapshot> = update
                                .snapshots
                                .iter()
                                .filter(|s| matches_ticker(&s.ticker))
                                .map(|s| typhoon_web_protocol::DarwinWebSnapshot {
                                    ticker: s.ticker.clone(),
                                    timestamp_ms: s.timestamp_ms,
                                    quote: s.quote,
                                    daily_return_pct: s.daily_return_pct,
                                    monthly_return_pct: s.monthly_return_pct,
                                    ytd_return_pct: s.ytd_return_pct,
                                    all_time_return_pct: s.all_time_return_pct,
                                    dscore: s.dscore,
                                    ds_experience: s.ds_experience,
                                    ds_risk_mgmt: s.ds_risk_mgmt,
                                    ds_risk_adjustment: s.ds_risk_adjustment,
                                    ds_performance: s.ds_performance,
                                    ds_scalability: s.ds_scalability,
                                    ds_market_correlation: s.ds_market_correlation,
                                    var_monthly: s.var_monthly,
                                    max_drawdown_pct: s.max_drawdown_pct,
                                    volatility_annual: s.volatility_annual,
                                    sharpe_ratio: s.sharpe_ratio,
                                    sortino_ratio: s.sortino_ratio,
                                    investors: s.investors,
                                    aum: s.aum,
                                    capacity_remaining_pct: s.capacity_remaining_pct,
                                    total_trades: s.total_trades,
                                    win_rate: s.win_rate,
                                    profit_factor: s.profit_factor,
                                    avg_holding_time_hours: s.avg_holding_time_hours,
                                    avg_trade_return_pct: s.avg_trade_return_pct,
                                    symbols_traded: s.symbols_traded,
                                    excluded: s.excluded,
                                    exclusion_reason: s.exclusion_reason.clone(),
                                    correlation_portfolio: s.correlation_portfolio,
                                })
                                .collect();
                            let correlations: Vec<typhoon_web_protocol::DarwinWebCorrelation> =
                                update
                                    .correlations
                                    .iter()
                                    .map(|c| typhoon_web_protocol::DarwinWebCorrelation {
                                        darwin_a: c.darwin_a.clone(),
                                        darwin_b: c.darwin_b.clone(),
                                        correlation: c.correlation,
                                    })
                                    .collect();
                            let alerts: Vec<typhoon_web_protocol::DarwinCorrelationAlert> = update
                                .correlation_alerts
                                .iter()
                                .map(|a| typhoon_web_protocol::DarwinCorrelationAlert {
                                    darwin_a: a.darwin_a.clone(),
                                    darwin_b: a.darwin_b.clone(),
                                    correlation: a.correlation,
                                    threshold: a.threshold,
                                    suggestion: a.suggestion.clone(),
                                })
                                .collect();
                            // Map expanded tab data
                            let monthly_returns: Vec<typhoon_web_protocol::DarwinMonthlyReturns> =
                                update
                                    .monthly_returns
                                    .iter()
                                    .filter(|mr| matches_ticker(&mr.ticker))
                                    .map(|mr| typhoon_web_protocol::DarwinMonthlyReturns {
                                        ticker: mr.ticker.clone(),
                                        rows: mr
                                            .rows
                                            .iter()
                                            .map(|r| typhoon_web_protocol::MonthlyReturnRow {
                                                year: r.year,
                                                months: r.months,
                                                year_total: r.year_total,
                                            })
                                            .collect(),
                                        cagr: mr.cagr,
                                        best_month_pct: mr.best_month_pct,
                                        worst_month_pct: mr.worst_month_pct,
                                        avg_month_pct: mr.avg_month_pct,
                                        positive_months: mr.positive_months,
                                        negative_months: mr.negative_months,
                                    })
                                    .collect();
                            let equity_curves: Vec<typhoon_web_protocol::DarwinEquityCurve> =
                                update
                                    .equity_curves
                                    .iter()
                                    .filter(|ec| matches_ticker(&ec.ticker))
                                    .map(|ec| typhoon_web_protocol::DarwinEquityCurve {
                                        ticker: ec.ticker.clone(),
                                        points: ec
                                            .points
                                            .iter()
                                            .map(|p| typhoon_web_protocol::EquityPoint {
                                                timestamp_ms: p.timestamp_ms,
                                                value: p.value,
                                            })
                                            .collect(),
                                    })
                                    .collect();
                            let var_histories: Vec<typhoon_web_protocol::DarwinVaRHistory> = update
                                .var_histories
                                .iter()
                                .filter(|vh| matches_ticker(&vh.ticker))
                                .map(|vh| typhoon_web_protocol::DarwinVaRHistory {
                                    ticker: vh.ticker.clone(),
                                    points: vh
                                        .points
                                        .iter()
                                        .map(|p| typhoon_web_protocol::VaRPoint {
                                            timestamp_ms: p.timestamp_ms,
                                            var_pct: p.var_pct,
                                        })
                                        .collect(),
                                    current_var: vh.current_var,
                                    avg_var: vh.avg_var,
                                    max_var: vh.max_var,
                                    min_var: vh.min_var,
                                    var_violations: vh.var_violations,
                                    drawdown_periods: vh
                                        .drawdown_periods
                                        .iter()
                                        .map(|dd| typhoon_web_protocol::DrawdownPeriod {
                                            start_ms: dd.start_ms,
                                            end_ms: dd.end_ms,
                                            depth_pct: dd.depth_pct,
                                            recovery_days: dd.recovery_days,
                                        })
                                        .collect(),
                                })
                                .collect();
                            let dscore_histories: Vec<typhoon_web_protocol::DarwinDScoreHistory> =
                                update
                                    .dscore_histories
                                    .iter()
                                    .filter(|dh| matches_ticker(&dh.ticker))
                                    .map(|dh| typhoon_web_protocol::DarwinDScoreHistory {
                                        ticker: dh.ticker.clone(),
                                        points: dh
                                            .points
                                            .iter()
                                            .map(|p| typhoon_web_protocol::DScorePoint {
                                                timestamp_ms: p.timestamp_ms,
                                                dscore: p.dscore,
                                                experience: p.experience,
                                                risk_stability: p.risk_stability,
                                                risk_adjustment: p.risk_adjustment,
                                                performance: p.performance,
                                                scalability: p.scalability,
                                                market_correlation: p.market_correlation,
                                            })
                                            .collect(),
                                    })
                                    .collect();
                            let investor_flows: Vec<typhoon_web_protocol::DarwinInvestorFlow> =
                                update
                                    .investor_flows
                                    .iter()
                                    .filter(|ifl| matches_ticker(&ifl.ticker))
                                    .map(|ifl| typhoon_web_protocol::DarwinInvestorFlow {
                                        ticker: ifl.ticker.clone(),
                                        points: ifl
                                            .points
                                            .iter()
                                            .map(|p| typhoon_web_protocol::InvestorFlowPoint {
                                                timestamp_ms: p.timestamp_ms,
                                                investor_count: p.investor_count,
                                                aum: p.aum,
                                            })
                                            .collect(),
                                        capital_in: ifl.capital_in,
                                        capital_out: ifl.capital_out,
                                        net_flow: ifl.net_flow,
                                        divergence_pct: ifl.divergence_pct,
                                    })
                                    .collect();
                            let portfolio_performance =
                                update.portfolio_performance.as_ref().map(|pp| {
                                    typhoon_web_protocol::PortfolioPerformance {
                                        total_return_pct: pp.total_return_pct,
                                        cagr: pp.cagr,
                                        best_month_pct: pp.best_month_pct,
                                        worst_month_pct: pp.worst_month_pct,
                                        monthly_returns: pp
                                            .monthly_returns
                                            .iter()
                                            .map(|r| typhoon_web_protocol::MonthlyReturnRow {
                                                year: r.year,
                                                months: r.months,
                                                year_total: r.year_total,
                                            })
                                            .collect(),
                                        equity_points: pp
                                            .equity_points
                                            .iter()
                                            .map(|p| typhoon_web_protocol::EquityPoint {
                                                timestamp_ms: p.timestamp_ms,
                                                value: p.value,
                                            })
                                            .collect(),
                                    }
                                });
                            let portfolio_risk = update.portfolio_risk.as_ref().map(|pr| {
                                typhoon_web_protocol::PortfolioRisk {
                                    current_var: pr.current_var,
                                    max_drawdown_pct: pr.max_drawdown_pct,
                                    diversification_benefit_pct: pr.diversification_benefit_pct,
                                    var_history: pr
                                        .var_history
                                        .iter()
                                        .map(|p| typhoon_web_protocol::VaRPoint {
                                            timestamp_ms: p.timestamp_ms,
                                            var_pct: p.var_pct,
                                        })
                                        .collect(),
                                }
                            });
                            let allocations: Vec<typhoon_web_protocol::DarwinAllocation> = update
                                .allocations
                                .iter()
                                .map(|a| typhoon_web_protocol::DarwinAllocation {
                                    ticker: a.ticker.clone(),
                                    weight_pct: a.weight_pct,
                                    invested: a.invested,
                                    pnl: a.pnl,
                                })
                                .collect();
                            if let Some(ref tx) = self.web_msg_tx {
                                let _ = tx.send(typhoon_web_protocol::WebMsg::DarwinWebUpdate {
                                    snapshots,
                                    correlations,
                                    correlation_alerts: alerts,
                                    monthly_returns,
                                    equity_curves,
                                    var_histories,
                                    dscore_histories,
                                    investor_flows,
                                    portfolio_performance,
                                    portfolio_risk,
                                    allocations,
                                });
                            }
                        }
                    }
                }
            }
            if web_cmds_drained >= WEB_CMD_DRAIN_MAX {
                ctx.request_repaint();
            }
        }

        // ── Cross-TF drawing sync ────────────────────────────────────────
        // When drawings_cross_tf is enabled, sync price-based drawings (HLine, FiboRetrace)
        // to all charts with the same symbol. Only syncs HLines (price-only, TF-independent).
        if self.drawings_cross_tf && self.charts.len() > 1 {
            let active = self.active_tab;
            if let Some(src) = self.charts.get(active) {
                let src_sym = src
                    .symbol
                    .split(':')
                    .next()
                    .unwrap_or(&src.symbol)
                    .to_uppercase();
                let src_drawings = src.drawings.clone();
                let src_styles = src.drawing_styles.clone();
                for (i, chart) in self.charts.iter_mut().enumerate() {
                    if i == active {
                        continue;
                    }
                    let chart_sym = chart
                        .symbol
                        .split(':')
                        .next()
                        .unwrap_or(&chart.symbol)
                        .to_uppercase();
                    if chart_sym != src_sym {
                        continue;
                    }
                    // Sync HLines (price-only drawings are TF-independent)
                    for (di, d) in src_drawings.iter().enumerate() {
                        if let Drawing::HLine { price, color } = d {
                            let already = chart.drawings.iter().any(|existing| {
                                matches!(existing, Drawing::HLine { price: p, .. } if (*p - price).abs() < 1e-10)
                            });
                            if !already {
                                chart.drawings.push(Drawing::HLine {
                                    price: *price,
                                    color: *color,
                                });
                                if di < src_styles.len() {
                                    chart.drawing_styles.push(src_styles[di]);
                                }
                            }
                        }
                    }
                }
            }
        }

        // ── Quake console toggle ─────────────────────────────────────────
        // Scans ALL input events for any sign of backtick/tilde/grave key.
        // Logs the first 20 unrecognized events for debugging Wayland issues.
        let open_palette = ctx.input_mut(|i| {
            let mut found = false;

            // Check all key methods
            if i.key_pressed(egui::Key::Backtick) {
                found = true;
            }

            // Scan every event
            i.events.retain(|e| {
                match e {
                    egui::Event::Text(t) if t == "`" || t == "~" => {
                        found = true;
                        false // consume
                    }
                    egui::Event::Key {
                        key: egui::Key::Backtick,
                        pressed: true,
                        ..
                    } => {
                        found = true;
                        false // consume
                    }
                    // Catch ANY key press and check the physical key
                    egui::Event::Key {
                        key,
                        pressed: true,
                        physical_key,
                        ..
                    } => {
                        // Check if physical_key matches backtick/grave
                        if let Some(pk) = physical_key {
                            if *pk == egui::Key::Backtick {
                                found = true;
                                return false; // consume
                            }
                        }
                        // Also check if the logical key name contains "grave" or "backtick"
                        let key_name = format!("{:?}", key);
                        if key_name.contains("Backtick") || key_name.contains("Grave") {
                            found = true;
                            return false;
                        }
                        true
                    }
                    _ => true,
                }
            });
            found
        });
        if open_palette {
            self.command_open = !self.command_open;
            if self.command_open {
                self.command_input.clear();
            } else {
                // Strip any trailing ` or ~ from input that might have leaked
                self.command_input = self
                    .command_input
                    .trim_matches(|c| c == '`' || c == '~')
                    .to_string();
            }
        }

        // ── Esc → close palette ──────────────────────────────────────────────
        if self.command_open && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.command_open = false;
            self.palette_context = PaletteContext::Global; // reset context on close
        }

        // ── crosshair from pointer ───────────────────────────────────────────
        // Suppress crosshair when pointer is over a floating window (dragging, resizing, scrolling)
        let pointer_over_ui = ctx.egui_wants_pointer_input()
            || ctx.egui_is_using_pointer()
            || ctx.dragged_id().is_some();
        let pointer_over_floating = if !pointer_over_ui {
            let hp = ctx.input(|i| i.pointer.hover_pos().unwrap_or_default());
            ctx.layer_id_at(hp)
                .map(|id| id.order == egui::Order::Middle || id.order == egui::Order::Foreground)
                .unwrap_or(false)
        } else {
            true
        };
        self.crosshair = if pointer_over_floating {
            None
        } else {
            ctx.input(|i| i.pointer.hover_pos())
        };

        // ── keyboard shortcuts ───────────────────────────────────────────────
        if !self.command_open {
            let left = ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft));
            let right = ctx.input(|i| i.key_pressed(egui::Key::ArrowRight));
            let home = ctx.input(|i| i.key_pressed(egui::Key::Home));
            let end = ctx.input(|i| i.key_pressed(egui::Key::End));
            let pgup = ctx.input(|i| i.key_pressed(egui::Key::PageUp));
            let pgdn = ctx.input(|i| i.key_pressed(egui::Key::PageDown));
            let plus =
                ctx.input(|i| i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals));
            let minus = ctx.input(|i| i.key_pressed(egui::Key::Minus));
            let delete = ctx
                .input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace));

            // Ctrl+N = new tab, Ctrl+W = close tab
            if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::N)) {
                let tf = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| c.timeframe)
                    .unwrap_or(Timeframe::H4);
                let mut new_chart = ChartState::new(&self.symbol_input, tf);
                if let Some(ref cache) = self.cache.clone() {
                    {
                        let mut gpu = self.gpu_indicators.take();
                        new_chart.try_load(Arc::as_ref(cache), &mut self.log, gpu.as_mut());
                        self.gpu_indicators = gpu;
                    }
                }
                self.charts.push(new_chart);
                self.active_tab = self.charts.len() - 1;
            }
            if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::W)) {
                if self.charts.len() > 1 {
                    self.charts.remove(self.active_tab);
                    if self.active_tab >= self.charts.len() {
                        self.active_tab = self.charts.len().saturating_sub(1);
                    }
                }
            }

            // ADR-094: Analytics keyboard shortcuts (Alt+key)
            if ctx.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::V)) {
                self.command_input = "VAR".to_string();
                self.log.push_back(LogEntry::info("Shortcut: Alt+V → VAR"));
            }
            if ctx.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::C)) {
                self.command_input = "CORRELATION".to_string();
                self.log
                    .push_back(LogEntry::info("Shortcut: Alt+C → CORRELATION"));
            }
            if ctx.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::S)) {
                self.command_input = "SCREENER".to_string();
                self.log
                    .push_back(LogEntry::info("Shortcut: Alt+S → SCREENER"));
            }
            if ctx.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::R)) {
                self.command_input = "RISK_CALC".to_string();
                self.log
                    .push_back(LogEntry::info("Shortcut: Alt+R → RISK_CALC"));
            }
            if ctx.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::B)) {
                self.command_input = "BACKTEST".to_string();
                self.log
                    .push_back(LogEntry::info("Shortcut: Alt+B → BACKTEST"));
            }
            if ctx.input(|i| i.key_pressed(egui::Key::F5)) {
                self.log
                    .push_back(LogEntry::info("F5: Refreshing all analytics..."));
                self.indicators_dirty = true;
            }
            // Esc: dismiss result card or close topmost window
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) && !self.command_open {
                if self.result_card.is_some() {
                    self.result_card = None;
                }
            }
            // Ctrl+1..9 = jump to tab by number
            for digit in 1..=9_u32 {
                let key = match digit {
                    1 => egui::Key::Num1,
                    2 => egui::Key::Num2,
                    3 => egui::Key::Num3,
                    4 => egui::Key::Num4,
                    5 => egui::Key::Num5,
                    6 => egui::Key::Num6,
                    7 => egui::Key::Num7,
                    8 => egui::Key::Num8,
                    9 => egui::Key::Num9,
                    _ => continue,
                };
                if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(key)) {
                    let idx = (digit - 1) as usize;
                    if idx < self.charts.len() {
                        self.active_tab = idx;
                    }
                }
            }

            // Ctrl+Tab / Ctrl+Shift+Tab = cycle tabs
            if !self.charts.is_empty()
                && ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Tab))
            {
                if ctx.input(|i| i.modifiers.shift) {
                    self.active_tab = if self.active_tab == 0 {
                        self.charts.len() - 1
                    } else {
                        self.active_tab - 1
                    };
                } else {
                    self.active_tab = (self.active_tab + 1) % self.charts.len();
                }
            }

            // Delete/Backspace = remove selected drawing, or last drawing if none selected
            if delete {
                if let Some(chart) = self.charts.get_mut(self.active_tab) {
                    if let Some(sel) = chart.selected_drawing {
                        if sel < chart.drawings.len() {
                            let d = chart.drawings.remove(sel);
                            chart.drawing_styles.remove(sel);
                            chart.drawings_undo.push(d);
                            chart.selected_drawing = None;
                        }
                    } else if let Some(d) = chart.drawings.pop() {
                        chart.drawing_styles.pop();
                        chart.drawings_undo.push(d);
                    }
                }
            }
            // Ctrl+Z = undo last drawing (same as delete but explicit)
            if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Z) && !i.modifiers.shift)
            {
                if let Some(chart) = self.charts.get_mut(self.active_tab) {
                    if let Some(d) = chart.drawings.pop() {
                        chart.drawing_styles.pop();
                        chart.drawings_undo.push(d);
                        chart.selected_drawing = None;
                        self.log.push_back(LogEntry::info("Undo: drawing removed"));
                    }
                }
            }
            // Ctrl+Shift+Z = redo (restore from undo stack)
            if ctx.input(|i| i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::Z)) {
                if let Some(chart) = self.charts.get_mut(self.active_tab) {
                    if let Some(d) = chart.drawings_undo.pop() {
                        chart.drawings.push(d);
                        chart.drawing_styles.push((1.5, LineStyle::Solid));
                        self.log.push_back(LogEntry::info("Redo: drawing restored"));
                    }
                }
            }

            // Escape = cancel drawing mode or exit replay
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                if self.replay_active {
                    self.replay_active = false;
                    self.replay_playing = false;
                    self.replay_bar_idx = 0;
                } else {
                    self.draw_mode = DrawMode::None;
                }
            }

            // Alt+1-9 = quick timeframe switch (TradingView standard)
            {
                let alt_tf = ctx.input(|i| {
                    if !i.modifiers.alt {
                        return None;
                    }
                    if i.key_pressed(egui::Key::Num1) {
                        Some(Timeframe::M1)
                    } else if i.key_pressed(egui::Key::Num2) {
                        Some(Timeframe::M5)
                    } else if i.key_pressed(egui::Key::Num3) {
                        Some(Timeframe::M15)
                    } else if i.key_pressed(egui::Key::Num4) {
                        Some(Timeframe::M30)
                    } else if i.key_pressed(egui::Key::Num5) {
                        Some(Timeframe::H1)
                    } else if i.key_pressed(egui::Key::Num6) {
                        Some(Timeframe::H4)
                    } else if i.key_pressed(egui::Key::Num7) {
                        Some(Timeframe::D1)
                    } else if i.key_pressed(egui::Key::Num8) {
                        Some(Timeframe::W1)
                    } else if i.key_pressed(egui::Key::Num9) {
                        Some(Timeframe::MN1)
                    } else {
                        None
                    }
                });
                if let Some(tf) = alt_tf {
                    if let Some(chart) = self.charts.get_mut(self.active_tab) {
                        chart.timeframe = tf;
                        if let Some(ref cache) = self.cache {
                            let mut gpu = self.gpu_indicators.take();
                            chart.try_load(Arc::as_ref(cache), &mut self.log, gpu.as_mut());
                            self.gpu_indicators = gpu;
                        }
                    }
                }
            }

            // Alt+letter = drawing tool shortcuts (TradingView standard)
            if ctx.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::H)) {
                self.draw_mode = DrawMode::PlacingHLine;
            }
            if ctx.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::V)) {
                self.draw_mode = DrawMode::PlacingVLine;
            }
            if ctx.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::T)) {
                self.draw_mode = DrawMode::PlacingTrendP1;
            }
            if ctx.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::F)) {
                self.draw_mode = DrawMode::PlacingFiboP1;
            }
            if ctx.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::R)) {
                self.draw_mode = DrawMode::PlacingRectP1;
            }
            if ctx.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::E)) {
                self.draw_mode = DrawMode::Eraser;
            }
            if ctx.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::L)) {
                if let Some(chart) = self.charts.get_mut(self.active_tab) {
                    chart.log_scale = !chart.log_scale;
                }
            }
            if ctx.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::C)) {
                // Alt+C = cycle chart type
                if let Some(chart) = self.charts.get_mut(self.active_tab) {
                    chart.chart_type = match chart.chart_type {
                        ChartType::Candle => ChartType::HeikinAshi,
                        ChartType::HeikinAshi => ChartType::Line,
                        ChartType::Line => ChartType::OhlcBars,
                        ChartType::OhlcBars => ChartType::Renko,
                        ChartType::Renko => ChartType::Candle,
                    };
                }
            }

            // Replay mode controls
            if self.replay_active {
                let total_bars = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| c.bars.len())
                    .unwrap_or(0);
                if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
                    self.replay_playing = !self.replay_playing;
                }
                if right && !self.replay_playing {
                    self.replay_bar_idx = (self.replay_bar_idx + 1).min(total_bars);
                }
                if left && !self.replay_playing {
                    self.replay_bar_idx = self.replay_bar_idx.saturating_sub(1).max(1);
                }
                // Up/Down = adjust speed
                let up = ctx.input(|i| i.key_pressed(egui::Key::ArrowUp));
                let down = ctx.input(|i| i.key_pressed(egui::Key::ArrowDown));
                if up {
                    self.replay_speed = (self.replay_speed * 1.5).min(60.0);
                }
                if down {
                    self.replay_speed = (self.replay_speed / 1.5).max(0.5);
                }

                // Auto-play timer
                if self.replay_playing {
                    let dt = ctx.input(|i| i.stable_dt);
                    self.replay_timer += dt;
                    let interval = 1.0 / self.replay_speed;
                    while self.replay_timer >= interval {
                        self.replay_timer -= interval;
                        self.replay_bar_idx = (self.replay_bar_idx + 1).min(total_bars);
                        if self.replay_bar_idx >= total_bars {
                            self.replay_playing = false;
                        }
                    }
                    ctx.request_repaint(); // keep animating
                }
                // Sync replay_bar_cap + view_offset on active chart
                if let Some(chart) = self.charts.get_mut(self.active_tab) {
                    chart.replay_bar_cap = Some(self.replay_bar_idx);
                    // Lock view to replay position so chart scrolls with replay
                    let half_vis = chart.visible_bars / 2;
                    chart.view_offset = self.replay_bar_idx.saturating_sub(1) + half_vis.min(10);
                }
            } else {
                // Replay not active — ensure cap is cleared
                if let Some(chart) = self.charts.get_mut(self.active_tab) {
                    if chart.replay_bar_cap.is_some() {
                        chart.replay_bar_cap = None;
                    }
                }
            }

            if let Some(chart) = self.charts.get_mut(self.active_tab) {
                // In replay mode, arrow keys are used for bar stepping, not panning
                if !self.replay_active {
                    if left {
                        chart.view_offset = chart.view_offset.saturating_sub(1);
                    }
                    if right {
                        chart.view_offset = (chart.view_offset + 1)
                            .min(chart.bars.len().saturating_sub(1) + CHART_RIGHT_MARGIN);
                    }
                    if home {
                        chart.view_offset =
                            chart.visible_bars.min(chart.bars.len()).saturating_sub(1);
                    }
                    if end {
                        chart.view_offset = chart.bars.len().saturating_sub(1) + CHART_RIGHT_MARGIN;
                    }
                    if pgup {
                        chart.view_offset =
                            chart.view_offset.saturating_sub(chart.visible_bars / 2);
                    }
                    if pgdn {
                        chart.view_offset = (chart.view_offset + chart.visible_bars / 2)
                            .min(chart.bars.len().saturating_sub(1) + CHART_RIGHT_MARGIN);
                    }
                }
                if plus {
                    Self::handle_zoom(chart, 1.0);
                }
                if minus {
                    Self::handle_zoom(chart, -1.0);
                }
            }
        }

        // ── top menu bar ─────────────────────────────────────────────────────
        egui::Panel::top("menu_bar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Connect to Broker…").clicked() {
                        self.show_connect = true;
                        ui.close();
                    }
                    if ui.button("Settings").clicked() {
                        self.show_settings = true;
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Quit  Alt+F4").clicked() {
                        self.save_session();
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("View", |ui| {
                    let mtf_label = if self.mtf_enabled {
                        "Single Chart".to_string()
                    } else {
                        format!("MTF Grid ({} charts)", self.charts.len())
                    };
                    if ui.button(&mtf_label).clicked() {
                        self.mtf_enabled = !self.mtf_enabled;
                        ui.close();
                    }
                    ui.menu_button("Grid Layout", |ui| {
                        if ui.button("2×2 (4 charts)").clicked() {
                            self.setup_mtf_grid(2, 4);
                            ui.close();
                        }
                        if ui.button("3×2 (6 charts)").clicked() {
                            self.setup_mtf_grid(3, 6);
                            ui.close();
                        }
                        if ui.button("3×3 (9 charts)").clicked() {
                            self.setup_mtf_grid(3, 9);
                            ui.close();
                        }
                        if ui.button("4×3 (12 charts)").clicked() {
                            self.setup_mtf_grid(4, 12);
                            ui.close();
                        }
                        if ui.button("4×4 (16 charts)").clicked() {
                            self.setup_mtf_grid(4, 16);
                            ui.close();
                        }
                    });
                    // MTF tab visibility checkboxes
                    if self.charts.len() > 1 {
                        ui.menu_button("MTF Tabs", |ui| {
                            // Ensure mtf_visible is the right size
                            while self.mtf_visible.len() < self.charts.len() {
                                self.mtf_visible.push(true);
                            }
                            ui.horizontal(|ui| {
                                if ui.small_button("All").clicked() {
                                    self.mtf_visible.iter_mut().for_each(|v| *v = true);
                                }
                                if ui.small_button("None").clicked() {
                                    self.mtf_visible.iter_mut().for_each(|v| *v = false);
                                    self.mtf_visible[0] = true;
                                }
                            });
                            ui.separator();
                            for (i, chart) in self.charts.iter().enumerate() {
                                let label = format!(
                                    "{} [{}]",
                                    chart
                                        .symbol
                                        .split(':')
                                        .nth(1)
                                        .or(Some(&chart.symbol))
                                        .unwrap_or(&chart.symbol),
                                    chart.timeframe.label()
                                );
                                if i < self.mtf_visible.len() {
                                    ui.checkbox(&mut self.mtf_visible[i], label);
                                }
                            }
                        });
                    }
                    if ui.button("Indicators…").clicked() {
                        self.show_indicators_panel = true;
                        ui.close();
                    }
                    ui.separator();
                    ui.label(egui::RichText::new("Chart Type").color(AXIS_TEXT).small());
                    let ct = self
                        .charts
                        .get(self.active_tab)
                        .map(|c| c.chart_type)
                        .unwrap_or(ChartType::Candle);
                    for &chart_type in &[
                        ChartType::Candle,
                        ChartType::HeikinAshi,
                        ChartType::Line,
                        ChartType::OhlcBars,
                        ChartType::Renko,
                    ] {
                        let selected = ct == chart_type;
                        let label = if selected {
                            format!("● {}", chart_type.label())
                        } else {
                            format!("  {}", chart_type.label())
                        };
                        if ui.button(label).clicked() {
                            if let Some(c) = self.charts.get_mut(self.active_tab) {
                                c.chart_type = chart_type;
                            }
                            ui.close();
                        }
                    }
                    ui.separator();
                    ui.label(
                        egui::RichText::new("Overlay Indicators")
                            .color(AXIS_TEXT)
                            .small(),
                    );
                    ui.checkbox(&mut self.show_sma200, "SMA 200");
                    ui.checkbox(&mut self.show_sma100, "SMA 100");
                    ui.checkbox(&mut self.show_kama, "KAMA(10,2,30)");
                    ui.checkbox(&mut self.show_ema21, "EMA 21");
                    ui.checkbox(&mut self.show_bollinger, "Bollinger Bands");
                    ui.separator();
                    ui.checkbox(&mut self.show_ichimoku, "Ichimoku Cloud");
                    ui.checkbox(&mut self.show_wma, "WMA(20)");
                    ui.checkbox(&mut self.show_hma, "HMA(20)");
                    ui.checkbox(&mut self.show_psar, "Parabolic SAR");
                    ui.checkbox(&mut self.show_atr_proj, "ATR Projection");
                    ui.checkbox(&mut self.show_prev_levels, "Prev Candle Levels (D/W)");
                    ui.checkbox(&mut self.show_pivots, "Pivot Points (P/R1/R2/S1/S2)");
                    ui.checkbox(&mut self.show_supply_demand, "Supply/Demand Zones");
                    ui.checkbox(&mut self.show_fvg, "Fair Value Gaps (FVG)");
                    ui.checkbox(&mut self.show_order_blocks, "Order Blocks (ICT/SMC)");
                    ui.separator();
                    ui.label(
                        egui::RichText::new("Pattern Recognition")
                            .color(AXIS_TEXT)
                            .small(),
                    );
                    ui.checkbox(&mut self.show_fractals, "Fractals (Bill Williams)");
                    ui.checkbox(&mut self.show_harmonics, "Harmonic Patterns (Carney)");
                    ui.checkbox(&mut self.show_auto_fib, "Auto Fibonacci");
                    ui.separator();
                    ui.label(
                        egui::RichText::new("Ehlers (Overlay)")
                            .color(AXIS_TEXT)
                            .small(),
                    );
                    ui.checkbox(&mut self.show_ehlers_ss, "Super Smoother(10)");
                    ui.checkbox(&mut self.show_ehlers_decycler, "Decycler(20)");
                    ui.checkbox(&mut self.show_ehlers_itl, "Instant. Trendline");
                    ui.checkbox(&mut self.show_ehlers_mama, "MAMA / FAMA");
                    ui.separator();
                    ui.label(egui::RichText::new("Sub-Panes").color(AXIS_TEXT).small());
                    ui.checkbox(&mut self.show_rsi, "RSI(14)");
                    ui.checkbox(&mut self.show_fisher, "Fisher Transform");
                    ui.checkbox(&mut self.show_macd, "MACD(12,26,9)");
                    ui.checkbox(&mut self.show_stochastic, "Stochastic(14,3,3)");
                    ui.checkbox(&mut self.show_adx, "ADX(14)");
                    ui.checkbox(&mut self.show_cci, "CCI(20)");
                    ui.checkbox(&mut self.show_williams_r, "Williams %R(14)");
                    ui.checkbox(&mut self.show_obv, "OBV");
                    ui.checkbox(&mut self.show_momentum, "Momentum(10)");
                    ui.checkbox(&mut self.show_cmo, "CMO(9)");
                    ui.checkbox(&mut self.show_qstick, "QStick(14)");
                    ui.checkbox(&mut self.show_disparity, "Disparity(14)");
                    ui.checkbox(&mut self.show_bop, "BOP(14)");
                    ui.checkbox(&mut self.show_stddev, "StdDev(20)");
                    ui.checkbox(&mut self.show_mfi, "MFI(14)");
                    ui.checkbox(&mut self.show_trix, "TRIX(15,9)");
                    ui.checkbox(&mut self.show_ppo, "PPO(12,26,9)");
                    ui.checkbox(&mut self.show_ultosc, "ULTOSC(7,14,28)");
                    ui.checkbox(&mut self.show_stochrsi, "StochRSI(14,14,3,3)");
                    ui.checkbox(&mut self.show_var_oscillator, "VaR Oscillator(20,95%)");
                    ui.checkbox(&mut self.show_better_volume, "Better Volume");
                    ui.checkbox(&mut self.show_volume_pane, "Volume");
                });
                ui.menu_button("Trading", |ui| {
                    if ui.button("Open Trade").clicked() {
                        self.submit_quick_trade();
                        ui.close();
                    }
                    if ui.button("Close All").clicked() {
                        self.close_all_selected_brokers();
                        ui.close();
                    }
                    if ui.button("Close Partial").clicked() {
                        self.close_partial_active_symbol();
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Set SL").clicked() {
                        self.apply_current_sl_to_positions();
                        ui.close();
                    }
                    if ui.button("Set TP").clicked() {
                        self.apply_current_tp_to_positions();
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Open MG (Martingale Hedge)").clicked() {
                        self.log.push_back(LogEntry::info(
                            "Trading: Open MG — connect to broker first",
                        ));
                        ui.close();
                    }
                    if ui.button("Buy Lines").clicked() {
                        match self.set_visible_range_trade_lines(true) {
                            Ok((sl, tp)) => {
                                self.log.push_back(LogEntry::info(format!(
                                    "Buy Lines: SL {} TP {} (drag to adjust)",
                                    format_price(sl),
                                    format_price(tp)
                                )));
                            }
                            Err(e) => self.log.push_back(LogEntry::warn(e)),
                        }
                        ui.close();
                    }
                    if ui.button("Sell Lines").clicked() {
                        match self.set_visible_range_trade_lines(false) {
                            Ok((sl, tp)) => {
                                self.log.push_back(LogEntry::info(format!(
                                    "Sell Lines: SL {} TP {} (drag to adjust)",
                                    format_price(sl),
                                    format_price(tp)
                                )));
                            }
                            Err(e) => self.log.push_back(LogEntry::warn(e)),
                        }
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Set SL Line").clicked() {
                        self.apply_current_sl_to_positions();
                        ui.close();
                    }
                    if ui.button("Set TP Line").clicked() {
                        self.apply_current_tp_to_positions();
                        ui.close();
                    }
                    if self.sl_price.is_some() || self.tp_price.is_some() {
                        if ui.button("Clear SL/TP Lines").clicked() {
                            self.clear_trade_lines();
                            ui.close();
                        }
                    }
                });
                ui.menu_button("Tools", |ui| {
                    if ui.button("Console (~)").clicked() {
                        self.command_open = !self.command_open;
                        if self.command_open {
                            self.command_input.clear();
                        }
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Symbol Overlap").clicked() {
                        self.show_symbol_overlap = true;
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Backtest").clicked() {
                        self.show_backtest = true;
                        ui.close();
                    }
                    if ui.button("Screener").clicked() {
                        self.show_screener = true;
                        ui.close();
                    }
                    if ui.button("Optimizer").clicked() {
                        self.show_optimizer = true;
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Risk Calculator").clicked() {
                        self.show_risk_calc = true;
                        ui.close();
                    }
                    if ui.button("VaR Multiplier").clicked() {
                        self.show_var_mult = true;
                        ui.close();
                    }
                    if ui.button("Margin Monitor").clicked() {
                        self.show_margin_monitor = true;
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Cache Statistics").clicked() {
                        self.show_cache_stats = true;
                        ui.close();
                    }
                });
                ui.menu_button("Research", |ui| {
                    if ui.button("News & Events").clicked() {
                        self.show_news = true;
                        ui.close();
                    }
                    if ui.button("Economic Calendar").clicked() {
                        self.show_calendar = true;
                        ui.close();
                    }
                    if ui.button("SEC Filings").clicked() {
                        self.show_sec = true;
                        ui.close();
                    }
                    if ui.button("Insider Trades").clicked() {
                        self.show_insider = true;
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Fundamentals").clicked() {
                        self.show_fundamentals = true;
                        ui.close();
                    }
                    if ui.button("Analyst Ratings").clicked() {
                        self.show_analyst = true;
                        ui.close();
                    }
                    if ui.button("Institutional Holders").clicked() {
                        self.show_holders = true;
                        ui.close();
                    }
                });
                ui.menu_button("Analysis", |ui| {
                    if ui.button("Correlation Matrix").clicked() {
                        self.show_correlation = true;
                        ui.close();
                    }
                    if ui.button("Seasonals").clicked() {
                        self.show_seasonals = true;
                        ui.close();
                    }
                    if ui.button("Monte Carlo VaR").clicked() {
                        self.show_montecarlo = true;
                        ui.close();
                    }
                    if ui.button("Stress Test").clicked() {
                        self.show_stress_test = true;
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Volume Profile").clicked() {
                        self.show_volume_profile = true;
                        ui.close();
                    }
                    if ui.button("Order Flow").clicked() {
                        self.show_order_flow = true;
                        ui.close();
                    }
                    if ui.button("Bookmap Heatmap").clicked() {
                        self.show_bookmap = true;
                        ui.close();
                    }
                });
                ui.menu_button("Help", |ui| {
                    if ui.button("Keyboard Shortcuts").clicked() {
                        self.show_help = true;
                        ui.close();
                    }
                    ui.separator();
                    ui.label(
                        egui::RichText::new("TyphooN Terminal v0.1.0")
                            .color(AXIS_TEXT)
                            .small(),
                    );
                    ui.label(egui::RichText::new("egui + wgpu").color(AXIS_TEXT).small());
                });
                ui.separator();
                ui.label(
                    egui::RichText::new("TyphooN Terminal")
                        .color(ACCENT)
                        .strong(),
                );
                // Broker scope indicator — click to cycle through scopes.
                // Shows the current global filter so the trader always knows what
                // data universe they're looking at (All / Alpaca / Darwinex / Tasty / Kraken).
                ui.separator();
                let (scope_lbl, scope_col) = match self.broker_scope {
                    EventSource::All => ("ALL", egui::Color32::from_rgb(140, 140, 160)),
                    EventSource::Alpaca => ("ALPACA", egui::Color32::from_rgb(255, 160, 60)),
                    EventSource::Darwinex => ("DARWINEX", egui::Color32::from_rgb(100, 180, 255)),
                    EventSource::Tasty => ("TASTY", egui::Color32::from_rgb(200, 130, 255)),
                    EventSource::Kraken => ("KRAKEN", egui::Color32::from_rgb(0, 170, 160)),
                    EventSource::Positions => ("POSITIONS", egui::Color32::from_rgb(80, 220, 120)),
                };
                let scope_btn = egui::Button::new(
                    egui::RichText::new(format!("Scope: {}", scope_lbl))
                        .strong()
                        .color(egui::Color32::WHITE),
                )
                .fill(scope_col);
                if ui
                    .add(scope_btn)
                    .on_hover_text("Left-click: cycle scope. Right-click: open scope settings.")
                    .clicked()
                {
                    self.broker_scope = match self.broker_scope {
                        EventSource::All => EventSource::Alpaca,
                        EventSource::Alpaca => EventSource::Darwinex,
                        EventSource::Darwinex => EventSource::Tasty,
                        EventSource::Tasty => EventSource::Kraken,
                        EventSource::Kraken => EventSource::Positions,
                        EventSource::Positions => EventSource::All,
                    };
                    // Sync fund_source toggles
                    match self.broker_scope {
                        EventSource::All => {
                            self.fund_source_mt5 = true;
                            self.fund_source_alpaca = true;
                            self.fund_source_tastytrade = true;
                            self.fund_source_kraken = true;
                        }
                        EventSource::Alpaca => {
                            self.fund_source_mt5 = false;
                            self.fund_source_alpaca = true;
                            self.fund_source_tastytrade = false;
                            self.fund_source_kraken = false;
                        }
                        EventSource::Darwinex => {
                            self.fund_source_mt5 = true;
                            self.fund_source_alpaca = false;
                            self.fund_source_tastytrade = false;
                            self.fund_source_kraken = false;
                        }
                        EventSource::Tasty => {
                            self.fund_source_mt5 = false;
                            self.fund_source_alpaca = false;
                            self.fund_source_tastytrade = true;
                            self.fund_source_kraken = false;
                        }
                        EventSource::Kraken => {
                            self.fund_source_mt5 = false;
                            self.fund_source_alpaca = false;
                            self.fund_source_tastytrade = false;
                            self.fund_source_kraken = true;
                        }
                        EventSource::Positions => {
                            self.fund_source_mt5 = true;
                            self.fund_source_alpaca = true;
                            self.fund_source_tastytrade = true;
                            self.fund_source_kraken = true;
                        }
                    }
                    let n = self.scoped_fundamentals().len();
                    self.log.push_back(LogEntry::info(format!(
                        "Broker scope → {} ({} fundamentals in scope)",
                        self.broker_scope_label(),
                        n
                    )));
                }
                // Alert breach badge — visible red counter when alerts have fired.
                // Clicking clears the counter and opens the alerts window.
                if self.alert_breach_count > 0 {
                    ui.separator();
                    let breach_label = format!("🔔 {} ALERT", self.alert_breach_count);
                    let tooltip = if self.alert_last_breach_msg.is_empty() {
                        format!(
                            "{} alert(s) fired — click to view and clear",
                            self.alert_breach_count
                        )
                    } else {
                        format!(
                            "{} alert(s) fired — latest:\n{}\n\nClick to view and clear.",
                            self.alert_breach_count, self.alert_last_breach_msg
                        )
                    };
                    let btn = egui::Button::new(
                        egui::RichText::new(breach_label)
                            .strong()
                            .color(egui::Color32::WHITE),
                    )
                    .fill(egui::Color32::from_rgb(231, 76, 60));
                    if ui.add(btn).on_hover_text(tooltip).clicked() {
                        self.show_alert_builder = true;
                        self.alert_breach_count = 0;
                        self.alert_last_breach_msg.clear();
                    }
                }
            });
        });

        // ── symbol + timeframe toolbar ───────────────────────────────────────
        egui::Panel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT).small());
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.symbol_input)
                        .desired_width(180.0)
                        .font(egui::FontId::monospace(13.0)),
                );

                // Update autocomplete suggestions when text changes
                if resp.changed() {
                    let query = self.symbol_input.trim().to_uppercase();
                    if query.len() >= 1 {
                        self.symbol_ac_visible = true;
                        self.symbol_ac_selected = 0;
                        // Build suggestions from cache keys + fundamentals
                        let mut suggestions = Vec::new();
                        // From fundamentals (has company name + sector).
                        // f.symbol is guaranteed uppercase (parse_yahoo_data), so skip the alloc.
                        for f in &self.bg.all_fundamentals {
                            if f.symbol.contains(&query)
                                || f.company_name.to_uppercase().contains(&query)
                            {
                                suggestions.push((
                                    f.symbol.clone(),
                                    f.company_name.clone(),
                                    f.sector.clone(),
                                ));
                            }
                        }
                        // From cache keys (all prefixes: mt5, kraken, alpaca, default, etc.)
                        if let Some(ref cache) = self.cache {
                            if let Ok(keys) = cache.all_keys() {
                                for key in &keys {
                                    // Skip BarCacheWriter metadata rows (__SYMBOLS__,
                                    // __SPECS__:…, __SERVER__:…, __HEARTBEAT__:…) so
                                    // their middle parts don't land as bogus symbol
                                    // suggestions in the autocomplete list.
                                    if key.starts_with("mt5:__") {
                                        continue;
                                    }
                                    let parts: Vec<&str> = key.split(':').collect();
                                    // Canonical 3-part key shape across providers:
                                    //   "mt5:SOLUSD:1Day" → SOLUSD
                                    //   "kraken:SOLUSD:1Day" → SOLUSD
                                    //   "alpaca:SOL/USD:4Hour" → SOL/USD
                                    // Fallback: bare "SYMBOL:TF" from legacy Alpaca paper arms.
                                    let sym = if parts.len() >= 3 {
                                        parts[parts.len() - 2].to_uppercase()
                                    } else if parts.len() == 2 {
                                        parts[0].to_uppercase()
                                    } else {
                                        continue;
                                    };
                                    // Normalize: remove slash for dedup (SOL/USD == SOLUSD)
                                    let sym_norm = sym.replace('/', "");
                                    let query_norm = query.replace('/', "");
                                    if sym_norm.contains(&query_norm)
                                        && !suggestions.iter().any(|(s, _, _)| {
                                            s.replace('/', "").to_uppercase() == sym_norm
                                        })
                                    {
                                        // Label crypto vs equity, always use no-slash form
                                        let class = if sym_norm.ends_with("USD")
                                            && !sym_norm.contains('.')
                                            && sym_norm.len() <= 10
                                        {
                                            "crypto".to_string()
                                        } else {
                                            String::new()
                                        };
                                        suggestions.push((sym_norm, String::new(), class));
                                    }
                                }
                            }
                        }
                        // From Kraken tradeable pairs (if loaded)
                        for (pair_name, display_name) in &self.kraken_pairs {
                            let pn = pair_name.to_uppercase();
                            let dn = display_name.to_uppercase();
                            if pn.contains(&query) || dn.contains(&query) {
                                if !suggestions.iter().any(|(s, _, _)| s.to_uppercase() == pn) {
                                    suggestions.push((
                                        display_name.clone(),
                                        pair_name.clone(),
                                        kraken_pair_asset_class(pair_name, display_name)
                                            .to_string(),
                                    ));
                                }
                            }
                        }
                        suggestions.sort_by(|a, b| {
                            // Exact prefix match first, then alphabetical
                            let a_starts = a.0.to_uppercase().starts_with(&query);
                            let b_starts = b.0.to_uppercase().starts_with(&query);
                            b_starts.cmp(&a_starts).then(a.0.cmp(&b.0))
                        });
                        suggestions.truncate(12);
                        self.symbol_suggestions = suggestions;
                        // If few local results and query >= 2 chars, also search Alpaca
                        if self.symbol_suggestions.len() < 5 && query.len() >= 2 {
                            let _ = self.broker_tx.send(BrokerCmd::SearchSymbols {
                                query: query.clone(),
                            });
                        }
                    } else {
                        self.symbol_ac_visible = false;
                    }
                }

                // Handle Enter: load symbol or select from autocomplete
                if resp.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                    if self.symbol_ac_visible
                        && self.symbol_ac_selected < self.symbol_suggestions.len()
                    {
                        self.symbol_input =
                            self.symbol_suggestions[self.symbol_ac_selected].0.clone();
                    }
                    let sym = self.symbol_input.trim().to_string();
                    let tf = self
                        .charts
                        .get(self.active_tab)
                        .map(|c| c.timeframe)
                        .unwrap_or(Timeframe::H4);
                    self.reload_symbol(&sym, tf);
                    self.symbol_ac_visible = false;
                }

                // Arrow keys to navigate suggestions
                if self.symbol_ac_visible && resp.has_focus() {
                    if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                        self.symbol_ac_selected = (self.symbol_ac_selected + 1)
                            .min(self.symbol_suggestions.len().saturating_sub(1));
                    }
                    if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                        self.symbol_ac_selected = self.symbol_ac_selected.saturating_sub(1);
                    }
                    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                        self.symbol_ac_visible = false;
                    }
                }

                // Hide autocomplete when input loses focus
                if !resp.has_focus() && !ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                    // Delay hide slightly so clicks on suggestions register
                    // (egui processes click after focus loss)
                }

                ui.separator();

                // Timeframe dropdown (ComboBox — type to search, e.g. "H4")
                let cur_tf = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| c.timeframe)
                    .unwrap_or(Timeframe::H4);
                let mut new_tf = cur_tf;
                egui::ComboBox::from_id_salt("tf_combo")
                    .selected_text(
                        egui::RichText::new(cur_tf.label())
                            .color(ACCENT)
                            .strong()
                            .small(),
                    )
                    .width(55.0)
                    .show_ui(ui, |ui| {
                        for &tf in Timeframe::all() {
                            ui.selectable_value(&mut new_tf, tf, tf.label());
                        }
                    });
                if new_tf != cur_tf {
                    let sym = self
                        .charts
                        .get(self.active_tab)
                        .map(|c| c.symbol.clone())
                        .unwrap_or_else(|| self.symbol_input.trim().to_string());
                    self.reload_symbol(&sym, new_tf);
                }

                let source_state = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        (
                            c.symbol.clone(),
                            c.primary_source,
                            c.source_override.clone(),
                        )
                    })
                    .unwrap_or_else(|| (self.symbol_input.trim().to_string(), "", None));
                let cached_sources = self.chart_source_options(&source_state.0, cur_tf);
                let old_source = source_state.2.clone().unwrap_or_else(|| "auto".to_string());
                let mut new_source = old_source.clone();
                let auto_label = if source_state.1.is_empty() {
                    "Auto".to_string()
                } else {
                    format!("Auto ({})", cache_source_label(source_state.1))
                };
                let selected_source_label = if old_source == "auto" {
                    auto_label.as_str()
                } else {
                    cache_source_label(&old_source)
                };
                egui::ComboBox::from_id_salt("source_combo")
                    .selected_text(
                        egui::RichText::new(selected_source_label)
                            .color(ACCENT)
                            .strong()
                            .small(),
                    )
                    .width(125.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut new_source, "auto".to_string(), auto_label);
                        ui.separator();
                        for (source, label) in CHART_SOURCE_ORDER {
                            let has_cached_bars = cached_sources
                                .iter()
                                .any(|(cached, _)| cached.eq_ignore_ascii_case(source));
                            if !has_cached_bars && source != "kraken" {
                                continue;
                            }
                            let label = if has_cached_bars {
                                format!("{label} ✓")
                            } else {
                                label.to_string()
                            };
                            ui.selectable_value(&mut new_source, source.to_string(), label);
                        }
                    });
                if new_source != old_source {
                    let source_override = (new_source != "auto").then_some(new_source);
                    self.reload_symbol_with_source(&source_state.0, cur_tf, source_override);
                }

                ui.separator();

                // MTF toggle
                let mtf_txt = if self.mtf_enabled {
                    egui::RichText::new("MTF ON").color(ACCENT).small().strong()
                } else {
                    egui::RichText::new("MTF").color(AXIS_TEXT).small()
                };
                if ui.small_button(mtf_txt).clicked() {
                    self.mtf_enabled = !self.mtf_enabled;
                }

                // MTF tab visibility checkboxes (inline, when MTF is on)
                if self.mtf_enabled && self.charts.len() > 1 {
                    while self.mtf_visible.len() < self.charts.len() {
                        self.mtf_visible.push(true);
                    }
                    ui.separator();
                    for (i, chart) in self.charts.iter().enumerate() {
                        if i >= self.mtf_visible.len() {
                            break;
                        }
                        // Second-to-last `:`-separated segment, without allocating a Vec.
                        let sym_short = {
                            let mut it = chart.symbol.rsplit(':');
                            let _last = it.next();
                            let s = it.next().unwrap_or(chart.symbol.as_str());
                            if s.len() > 8 { &s[..8] } else { s }
                        };
                        let label = format!("{} {}", sym_short, chart.timeframe.label());
                        let color = if self.mtf_visible[i] {
                            ACCENT
                        } else {
                            AXIS_TEXT
                        };
                        if ui
                            .add(egui::SelectableLabel::new(
                                self.mtf_visible[i],
                                egui::RichText::new(&label).color(color).small().monospace(),
                            ))
                            .clicked()
                        {
                            self.mtf_visible[i] = !self.mtf_visible[i];
                            // Ensure at least one is visible
                            if self.mtf_visible.iter().all(|v| !v) {
                                self.mtf_visible[i] = true;
                            }
                        }
                    }
                }

                ui.separator();

                // Bar count + active-position entry price
                if let Some(c) = self.charts.get(self.active_tab) {
                    ui.label(
                        egui::RichText::new(format!("{} bars", c.bars.len()))
                            .color(AXIS_TEXT)
                            .small(),
                    );
                    let active_symbol = bare_symbol_from_key(&c.symbol).to_ascii_uppercase();
                    let active_entry = self
                        .live_positions
                        .iter()
                        .chain(self.tt_positions.iter())
                        .find_map(|pos| {
                            let pos_symbol = bare_symbol_from_key(&pos.symbol).to_ascii_uppercase();
                            (pos_symbol == active_symbol
                                && pos.avg_entry_price.is_finite()
                                && pos.avg_entry_price > 0.0)
                                .then_some(pos.avg_entry_price)
                        })
                        .or_else(|| {
                            self.kr_positions.iter().find_map(|pos| {
                                let pos_symbol =
                                    bare_symbol_from_key(&pos.symbol).to_ascii_uppercase();
                                if pos_symbol != active_symbol {
                                    return None;
                                }
                                if pos.avg_entry_price.is_finite() && pos.avg_entry_price > 0.0 {
                                    Some(pos.avg_entry_price)
                                } else {
                                    self.kraken_position_avg_price(&pos.symbol)
                                }
                            })
                        });
                    if let Some(entry) = active_entry {
                        ui.label(
                            egui::RichText::new(format!("Entry {}", format_price(entry)))
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    }
                }

                if self.cache.is_none() {
                    ui.label(
                        egui::RichText::new("NO CACHE")
                            .color(egui::Color32::from_rgb(255, 80, 80))
                            .small()
                            .strong(),
                    );
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new("~").color(AXIS_TEXT).small());
                    ui.separator();
                    // Determine if we have any data source (broker, MT5, LAN, API keys)
                    let has_broker =
                        self.broker_connected || self.tt_connected || self.kraken_connected;
                    let has_mt5 = !self.mt5_db_paths.iter().all(|p| p.is_empty());
                    let has_lan = self.lan_sync_mode == "client" || self.lan_sync_mode == "server";
                    let has_api = !self.finnhub_key.is_empty() || !self.fred_key.is_empty();
                    let has_cache = self.cache.is_some()
                        && self
                            .bg
                            .cache_stats
                            .map(|(bars, _, _)| bars > 0)
                            .unwrap_or(false);
                    if has_broker || has_mt5 || has_lan || has_api || has_cache {
                        let mut sources: Vec<String> = Vec::new();
                        if self.broker_connected {
                            let mode = if self.broker_paper { "Paper" } else { "Live" };
                            sources.push(format!("Alpaca ({})", mode));
                        }
                        if self.kraken_connected {
                            sources.push("Kraken (REST + WS)".into());
                        }
                        if self.tt_connected {
                            let mode = if self.tt_sandbox { "Sandbox" } else { "Live" };
                            sources.push(format!("tastytrade ({})", mode));
                        }
                        if !self.mt5_db_paths.iter().all(|p| p.is_empty()) {
                            sources.push("MT5".into());
                        }
                        if !self.finnhub_key.is_empty() {
                            sources.push("Finnhub".into());
                        }
                        if !self.fred_key.is_empty() {
                            sources.push("FRED".into());
                        }
                        // LAN client is the provider — everything is served through the
                        // LAN server, we don't call any external source directly. Replace
                        // the locally-derived source list with a single `LAN <ip>` chip,
                        // optionally annotated with the server's source list so the user
                        // sees WHAT is being synced in from the LAN (Alpaca+MT5+FRED, etc)
                        // without implying the client is calling those APIs itself.
                        if self.lan_sync_mode == "client" {
                            let ip = if self.lan_server_ip.is_empty() {
                                self.lan_sync_host.clone()
                            } else {
                                self.lan_server_ip.clone()
                            };
                            let server_src = self
                                .cache
                                .as_ref()
                                .and_then(|c| c.get_kv("lan:server:sources").ok().flatten())
                                .filter(|s| !s.is_empty());
                            sources.clear();
                            sources.push(match server_src {
                                Some(s) => format!("LAN {}: {}", ip, s),
                                None => format!("LAN {}", ip),
                            });
                        } else if self.lan_sync_mode == "server" {
                            // Publish our locally-derived source list so connected LAN
                            // clients can render it inside their `LAN <ip>: …` chip.
                            let server_sources = sources.join(" + ");
                            if let Some(ref cache) = self.cache {
                                let _ = cache.put_kv("lan:server:sources", &server_sources);
                            }
                            sources.push("LAN Server".into());
                        }
                        let src_text = sources.join(" + ");
                        // Any data source connected = Connected. OFFLINE only when nothing connected.
                        // Market hours per-symbol can be refined later using symbol specs.
                        let (status, color) = ("Connected", UP);
                        ui.label(
                            egui::RichText::new(format!("\u{25CF} {} [{}]", status, src_text))
                                .color(color)
                                .small(),
                        );
                        let active_session = self.charts.get(self.active_tab).and_then(|chart| {
                            let symbol = chart
                                .symbol
                                .split(':')
                                .rev()
                                .nth(1)
                                .or_else(|| chart.symbol.split(':').last())
                                .unwrap_or("");
                            let kraken_crypto_pair = symbol.contains('/')
                                && !symbol.to_ascii_uppercase().contains(".EQ");
                            if self.kraken_connected && kraken_crypto_pair {
                                Some("24/7".to_string())
                            } else if self.broker_connected && !self.market_clock_status.is_empty()
                            {
                                Some(self.market_clock_status.clone())
                            } else {
                                None
                            }
                        });
                        if let Some(session) = active_session {
                            let session_upper = session.to_ascii_uppercase();
                            let session_color = if session_upper.contains("CLOSED") {
                                DOWN
                            } else if session_upper.contains("OPEN") || session == "24/7" {
                                UP
                            } else {
                                AXIS_TEXT
                            };
                            ui.label(
                                egui::RichText::new(format!("[Session {}]", session))
                                    .color(session_color)
                                    .small(),
                            );
                        }
                        if self.kraken_connected {
                            let kraken_balance = self.kraken_usd_equivalent_balance();
                            ui.label(
                                egui::RichText::new(format!(
                                    "[Kraken (Live) ${:.0}]",
                                    kraken_balance
                                ))
                                .color(UP)
                                .small(),
                            );
                        }
                        if self.tt_connected {
                            let mode = if self.tt_sandbox { "Sandbox" } else { "Live" };
                            let color = if self.tt_sandbox {
                                egui::Color32::WHITE
                            } else {
                                UP
                            };
                            if let Some(ref bal) = self.tt_balances {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "[tastytrade ({}) ${:.0}]",
                                        mode, bal.net_liquidating_value
                                    ))
                                    .color(color)
                                    .small(),
                                );
                            } else {
                                ui.label(
                                    egui::RichText::new(format!("[tastytrade ({})]", mode))
                                        .color(color)
                                        .small(),
                                );
                            }
                        }
                        if let Some(ref acct) = self.live_account {
                            let mode = if self.broker_paper { "Paper" } else { "Live" };
                            let color = if self.broker_paper {
                                egui::Color32::WHITE
                            } else {
                                UP
                            };
                            ui.label(
                                egui::RichText::new(format!(
                                    "[Alpaca ({}) ${:.0}]",
                                    mode, acct.equity
                                ))
                                .color(color)
                                .small(),
                            );
                        }
                    } else {
                        let mut offline_sources = Vec::new();
                        if !self.mt5_db_paths.iter().all(|p| p.is_empty()) {
                            offline_sources.push("MT5 cache");
                        }
                        let src = if offline_sources.is_empty() {
                            "no sources".to_string()
                        } else {
                            offline_sources.join(" + ")
                        };
                        ui.label(
                            egui::RichText::new(format!("\u{25CB} OFFLINE [{}]", src))
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    }
                });
            });
        });

        // ── Symbol autocomplete dropdown ─────────────────────────────────────
        if self.symbol_ac_visible && !self.symbol_suggestions.is_empty() {
            let ac_cyan = egui::Color32::from_rgb(26, 188, 156);
            let ac_dim = egui::Color32::from_rgb(100, 100, 120);
            let ac_bg = egui::Color32::from_rgb(20, 22, 35);
            let ac_sel = egui::Color32::from_rgb(30, 40, 65);

            egui::Area::new(egui::Id::new("symbol_autocomplete"))
                .fixed_pos(egui::pos2(80.0, 45.0)) // below symbol input
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    egui::Frame::NONE
                        .fill(ac_bg)
                        .inner_margin(4.0)
                        .corner_radius(4.0)
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 50, 70)))
                        .show(ui, |ui| {
                            ui.set_min_width(350.0);
                            let suggestions: Vec<_> = self.symbol_suggestions.clone();
                            let mut clicked_sym: Option<String> = None;
                            for (idx, (sym, company, sector)) in suggestions.iter().enumerate() {
                                let selected = idx == self.symbol_ac_selected;
                                let bg = if selected { ac_sel } else { ac_bg };
                                egui::Frame::NONE.fill(bg).inner_margin(4.0).show(ui, |ui| {
                                    let resp = ui
                                        .horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new(sym)
                                                    .strong()
                                                    .color(ac_cyan)
                                                    .monospace(),
                                            );
                                            if !company.is_empty() {
                                                ui.label(
                                                    egui::RichText::new(company).small().color(
                                                        egui::Color32::from_rgb(180, 180, 190),
                                                    ),
                                                );
                                            }
                                            if !sector.is_empty() {
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(
                                                        egui::Align::Center,
                                                    ),
                                                    |ui| {
                                                        ui.label(
                                                            egui::RichText::new(sector)
                                                                .small()
                                                                .color(ac_dim),
                                                        );
                                                    },
                                                );
                                            }
                                        })
                                        .response;
                                    if resp.clicked() {
                                        clicked_sym = Some(sym.clone());
                                    }
                                    if resp.hovered() {
                                        self.symbol_ac_selected = idx;
                                    }
                                });
                            }
                            if let Some(sym) = clicked_sym {
                                self.symbol_input = sym.clone();
                                let tf = self
                                    .charts
                                    .get(self.active_tab)
                                    .map(|c| c.timeframe)
                                    .unwrap_or(Timeframe::H4);
                                self.reload_symbol(&sym, tf);
                                self.symbol_ac_visible = false;
                            }
                        });
                });
        }

        // ── tab bar ───────────────────────────────────────────────────────────
        egui::Panel::top("tab_bar")
            .exact_size(26.0) // WebKit: height: 26px
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    let mut switch_to: Option<usize> = None;
                    let mut close_tab: Option<usize> = None;
                    let mut drop_target: Option<(usize, usize)> = None; // (drag_src, insert_at)
                    let pointer_pos = ctx.input(|i| i.pointer.hover_pos());
                    let pointer_released = ctx.input(|i| i.pointer.primary_released());

                    // Collect tab rects for drag detection
                    let mut tab_rects: Vec<egui::Rect> = Vec::new();

                    for (idx, chart) in self.charts.iter().enumerate() {
                        let active = idx == self.active_tab;
                        let is_dragging_this = self.dragging_tab == Some(idx);
                        let label = format!("{} [{}]", chart.symbol, chart.timeframe.label());

                        // Tab colours
                        let tab_bg = if is_dragging_this {
                            egui::Color32::from_rgb(20, 50, 80)
                        } else if active {
                            BG_BUTTON
                        } else {
                            egui::Color32::from_rgb(10, 10, 10)
                        };
                        let tab_text = if active {
                            egui::Color32::WHITE
                        } else {
                            egui::Color32::from_rgb(136, 136, 136)
                        };

                        let tab_w = label.len() as f32 * 6.5 + 28.0;

                        // Allocate space for this tab
                        let (tab_rect, tab_resp) = ui.allocate_exact_size(
                            egui::vec2(tab_w, 24.0),
                            egui::Sense::click_and_drag(),
                        );
                        tab_rects.push(tab_rect);

                        // Draw tab background
                        ui.painter().rect_filled(tab_rect, 0.0, tab_bg);

                        // Active tab: green bottom border
                        if active {
                            ui.painter().line_segment(
                                [
                                    egui::pos2(tab_rect.left(), tab_rect.bottom()),
                                    egui::pos2(tab_rect.right(), tab_rect.bottom()),
                                ],
                                egui::Stroke::new(2.0, egui::Color32::from_rgb(76, 175, 80)),
                            );
                        }

                        // Right border separator
                        ui.painter().line_segment(
                            [
                                egui::pos2(tab_rect.right(), tab_rect.top()),
                                egui::pos2(tab_rect.right(), tab_rect.bottom()),
                            ],
                            egui::Stroke::new(1.0, egui::Color32::from_rgb(34, 34, 34)),
                        );

                        // Draw drag indicator (green left/right border when hovering during drag)
                        if let Some(drag_src) = self.dragging_tab {
                            if drag_src != idx {
                                if let Some(pos) = pointer_pos {
                                    if tab_rect.contains(pos) {
                                        let mid = tab_rect.center().x;
                                        let side = if pos.x < mid {
                                            tab_rect.left()
                                        } else {
                                            tab_rect.right()
                                        };
                                        ui.painter().line_segment(
                                            [
                                                egui::pos2(side, tab_rect.top()),
                                                egui::pos2(side, tab_rect.bottom()),
                                            ],
                                            egui::Stroke::new(
                                                2.0,
                                                egui::Color32::from_rgb(76, 175, 80),
                                            ),
                                        );
                                    }
                                }
                            }
                        }

                        // Tab label text
                        let text_pos = egui::pos2(tab_rect.left() + 6.0, tab_rect.center().y);
                        ui.painter().text(
                            text_pos,
                            egui::Align2::LEFT_CENTER,
                            &label,
                            egui::FontId::monospace(10.0),
                            tab_text,
                        );

                        // Close button (×) — right side of tab
                        if self.charts.len() > 1 {
                            let close_rect = egui::Rect::from_min_size(
                                egui::pos2(tab_rect.right() - 14.0, tab_rect.top() + 4.0),
                                egui::vec2(12.0, 16.0),
                            );
                            let close_hovered =
                                pointer_pos.map(|p| close_rect.contains(p)).unwrap_or(false);
                            let close_col = if close_hovered {
                                egui::Color32::from_rgb(255, 80, 80)
                            } else {
                                egui::Color32::from_rgb(85, 85, 85)
                            };
                            ui.painter().text(
                                close_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                "×",
                                egui::FontId::monospace(11.0),
                                close_col,
                            );
                            if tab_resp.clicked() && close_hovered {
                                close_tab = Some(idx);
                            } else if tab_resp.clicked() {
                                switch_to = Some(idx);
                            }
                        } else if tab_resp.clicked() {
                            switch_to = Some(idx);
                        }

                        // Middle-click to close tab
                        if tab_resp.middle_clicked() && self.charts.len() > 1 {
                            close_tab = Some(idx);
                        }

                        // Start drag
                        if tab_resp.dragged() && self.dragging_tab.is_none() {
                            self.dragging_tab = Some(idx);
                        }
                    }

                    // Handle drop on release
                    if pointer_released {
                        if let Some(drag_src) = self.dragging_tab {
                            if let Some(pos) = pointer_pos {
                                for (idx, rect) in tab_rects.iter().enumerate() {
                                    if rect.contains(pos) && idx != drag_src {
                                        let mid = rect.center().x;
                                        // Insert before idx if dropping on left half, after if right half
                                        let insert_at = if pos.x < mid { idx } else { idx + 1 };
                                        drop_target = Some((drag_src, insert_at));
                                        break;
                                    }
                                }
                            }
                            self.dragging_tab = None;
                        }
                    }

                    // + button (WebKit: .tab-add)
                    if ui
                        .add(
                            egui::Label::new(
                                egui::RichText::new("+")
                                    .color(egui::Color32::from_rgb(85, 85, 85))
                                    .size(14.0),
                            )
                            .sense(egui::Sense::click()),
                        )
                        .clicked()
                    {
                        let tf = self
                            .charts
                            .get(self.active_tab)
                            .map(|c| c.timeframe)
                            .unwrap_or(Timeframe::H4);
                        let mut new_chart = ChartState::new(&self.symbol_input, tf);
                        if let Some(ref cache) = self.cache.clone() {
                            {
                                let mut gpu = self.gpu_indicators.take();
                                new_chart.try_load(Arc::as_ref(cache), &mut self.log, gpu.as_mut());
                                self.gpu_indicators = gpu;
                            }
                        }
                        self.charts.push(new_chart);
                        self.active_tab = self.charts.len() - 1;
                    }

                    // Chart type indicator (right-aligned)
                    if let Some(c) = self.charts.get(self.active_tab) {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(c.chart_type.label())
                                    .color(AXIS_TEXT)
                                    .small(),
                            );
                        });
                    }

                    // Apply deferred actions
                    if let Some(idx) = switch_to {
                        self.active_tab = idx;
                        // Sync symbol_input to the clicked tab's symbol.
                        // Without this, clicking a timeframe button after switching tabs
                        // reloads the OLD symbol (from the text box) instead of the tab's symbol.
                        if let Some(chart) = self.charts.get(idx) {
                            self.symbol_input = chart.symbol.clone();
                        }
                        // Lazy-load chart bars on first tab switch
                        if let Some(chart) = self.charts.get_mut(idx) {
                            if chart.bars.is_empty() {
                                if let Some(ref cache) = self.cache {
                                    {
                                        let mut gpu = self.gpu_indicators.take();
                                        if !chart.try_load(cache, &mut self.log, gpu.as_mut()) {
                                            self.queue_chart_reload(0);
                                        }
                                        self.gpu_indicators = gpu;
                                    }
                                }
                            }
                        }
                    }
                    if let Some(idx) = close_tab {
                        self.charts.remove(idx);
                        if self.active_tab >= self.charts.len() {
                            self.active_tab = self.charts.len().saturating_sub(1);
                        }
                    }
                    if let Some((drag_src, insert_at)) = drop_target {
                        if drag_src < self.charts.len() {
                            let chart = self.charts.remove(drag_src);
                            // Adjust insert_at since removal shifts indices
                            let adjusted = if insert_at > drag_src {
                                insert_at - 1
                            } else {
                                insert_at
                            };
                            let adjusted = adjusted.min(self.charts.len());
                            self.charts.insert(adjusted, chart);
                            self.active_tab = adjusted;
                        }
                    }
                });
            });

        // ── bottom panel (log / volume) ──────────────────────────────────────
        // ── ADR-094: Result card rendering (above log) ─────────────
        // Auto-dismiss after 30 seconds
        if let Some((_, created)) = &self.result_card {
            if created.elapsed() > std::time::Duration::from_secs(30) {
                self.result_card = None;
            }
        }

        egui::Panel::bottom("bottom_panel")
            .resizable(true)
            .min_size(80.0)
            .default_size(140.0)
            .show(ctx, |ui| {
                // ── Result card (above log) ──
                if let Some((card, _)) = &self.result_card {
                    ui.group(|ui| {
                        match card {
                            ResultCard::Summary { title, metrics } => {
                                ui.horizontal(|ui| {
                                    ui.strong(title);
                                    if ui.small_button("\u{2716}").clicked() { /* dismiss below */ }
                                });
                                ui.horizontal_wrapped(|ui| {
                                    for (label, value, color) in metrics {
                                        ui.label(
                                            egui::RichText::new(label)
                                                .small()
                                                .color(egui::Color32::GRAY),
                                        );
                                        ui.label(
                                            egui::RichText::new(value)
                                                .strong()
                                                .color(*color)
                                                .monospace(),
                                        );
                                        ui.add_space(12.0);
                                    }
                                });
                            }
                            ResultCard::Table {
                                title,
                                headers,
                                rows,
                                sort_col,
                                sort_asc,
                            } => {
                                ui.horizontal(|ui| {
                                    ui.strong(title);
                                    ui.label(
                                        egui::RichText::new(format!("({} rows)", rows.len()))
                                            .small()
                                            .color(egui::Color32::GRAY),
                                    );
                                });
                                egui::ScrollArea::vertical()
                                    .auto_shrink(false)
                                    .max_height(100.0)
                                    .show(ui, |ui| {
                                        egui::Grid::new("result_table").striped(true).show(
                                            ui,
                                            |ui| {
                                                for (i, h) in headers.iter().enumerate() {
                                                    let arrow = if i == *sort_col {
                                                        if *sort_asc {
                                                            " \u{25B2}"
                                                        } else {
                                                            " \u{25BC}"
                                                        }
                                                    } else {
                                                        ""
                                                    };
                                                    ui.label(
                                                        egui::RichText::new(format!("{h}{arrow}"))
                                                            .small()
                                                            .strong(),
                                                    );
                                                }
                                                ui.end_row();
                                                for row in rows.iter().take(50) {
                                                    for cell in row {
                                                        ui.label(
                                                            egui::RichText::new(cell)
                                                                .small()
                                                                .monospace(),
                                                        );
                                                    }
                                                    ui.end_row();
                                                }
                                            },
                                        );
                                    });
                            }
                            ResultCard::Chart {
                                title,
                                label,
                                values,
                            } => {
                                ui.horizontal(|ui| {
                                    ui.strong(title);
                                    if let Some(last) = values.last() {
                                        ui.label(
                                            egui::RichText::new(format!("{label}: {last:.4}"))
                                                .monospace(),
                                        );
                                    }
                                });
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::vec2(ui.available_width().min(300.0), 40.0),
                                    egui::Sense::hover(),
                                );
                                draw_sparkline(
                                    ui.painter(),
                                    rect,
                                    values,
                                    egui::Color32::from_rgb(0, 180, 255),
                                );
                            }
                            ResultCard::Gauge {
                                title,
                                label,
                                value,
                                min,
                                max,
                                danger_low,
                                danger_high,
                            } => {
                                ui.horizontal(|ui| {
                                    ui.strong(title);
                                    let color = if *value < *danger_low || *value > *danger_high {
                                        egui::Color32::from_rgb(255, 80, 80)
                                    } else {
                                        egui::Color32::from_rgb(80, 220, 120)
                                    };
                                    ui.label(
                                        egui::RichText::new(format!("{label}: {value:.2}%"))
                                            .strong()
                                            .color(color)
                                            .monospace(),
                                    );
                                });
                                // Draw gauge bar
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::vec2(200.0, 12.0),
                                    egui::Sense::hover(),
                                );
                                let painter = ui.painter();
                                painter.rect_filled(rect, 3.0, egui::Color32::from_rgb(40, 40, 50));
                                let range = max - min;
                                if range > 0.0 {
                                    let frac = ((value - min) / range).clamp(0.0, 1.0) as f32;
                                    let fill_rect = egui::Rect::from_min_size(
                                        rect.min,
                                        egui::vec2(rect.width() * frac, rect.height()),
                                    );
                                    let fill_color =
                                        if *value < *danger_low || *value > *danger_high {
                                            egui::Color32::from_rgb(255, 80, 80)
                                        } else {
                                            egui::Color32::from_rgb(80, 220, 120)
                                        };
                                    painter.rect_filled(fill_rect, 3.0, fill_color);
                                    // Danger zone markers
                                    let low_x = rect.min.x
                                        + ((danger_low - min) / range) as f32 * rect.width();
                                    let high_x = rect.min.x
                                        + ((danger_high - min) / range) as f32 * rect.width();
                                    painter.line_segment(
                                        [
                                            egui::pos2(low_x, rect.min.y),
                                            egui::pos2(low_x, rect.max.y),
                                        ],
                                        egui::Stroke::new(1.0, egui::Color32::YELLOW),
                                    );
                                    painter.line_segment(
                                        [
                                            egui::pos2(high_x, rect.min.y),
                                            egui::pos2(high_x, rect.max.y),
                                        ],
                                        egui::Stroke::new(1.0, egui::Color32::YELLOW),
                                    );
                                }
                            }
                        }
                    });
                    ui.separator();
                }

                // ── Log panel header with filter ──
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.bottom_tab, BottomTab::Log, "Log");
                    ui.separator();
                    ui.label(
                        egui::RichText::new("Filter:")
                            .small()
                            .color(egui::Color32::GRAY),
                    );
                    egui::ComboBox::from_id_salt("log_filter")
                        .width(70.0)
                        .selected_text(match self.log_filter {
                            LogFilter::All => "All",
                            LogFilter::Info => "Info",
                            LogFilter::Warn => "Warn",
                            LogFilter::Error => "Error",
                            LogFilter::Trade => "Trade",
                            LogFilter::Alert => "Alert",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.log_filter, LogFilter::All, "All");
                            ui.selectable_value(
                                &mut self.log_filter,
                                LogFilter::Info,
                                "\u{2139} Info",
                            );
                            ui.selectable_value(
                                &mut self.log_filter,
                                LogFilter::Warn,
                                "\u{26A0} Warn",
                            );
                            ui.selectable_value(
                                &mut self.log_filter,
                                LogFilter::Error,
                                "\u{2716} Error",
                            );
                            ui.selectable_value(
                                &mut self.log_filter,
                                LogFilter::Trade,
                                "\u{1F4B0} Trade",
                            );
                            ui.selectable_value(
                                &mut self.log_filter,
                                LogFilter::Alert,
                                "\u{1F514} Alert",
                            );
                        });
                    // Dismiss result card button
                    if self.result_card.is_some() {
                        if ui.small_button("Dismiss Card").clicked() {
                            self.result_card = None;
                        }
                    }
                });
                ui.separator();
                match self.bottom_tab {
                    BottomTab::Log => {
                        egui::ScrollArea::both()
                            .stick_to_bottom(true)
                            .auto_shrink(false)
                            .show(ui, |ui| {
                                for entry in &self.log {
                                    if !entry.matches_filter(self.log_filter) {
                                        continue;
                                    }
                                    let response = ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(&entry.display)
                                                .color(entry.color())
                                                .font(egui::FontId::monospace(11.0)),
                                        )
                                        .wrap_mode(egui::TextWrapMode::Extend)
                                        .sense(egui::Sense::click()),
                                    );
                                    // Clickable log entries: detect ticker symbols (ALL CAPS, 2-6 chars)
                                    if response.clicked() {
                                        // Extract first uppercase word that looks like a ticker
                                        if let Some(ticker) =
                                            entry.msg.split_whitespace().find(|w| {
                                                w.len() >= 2
                                                    && w.len() <= 8
                                                    && w.chars().all(|c| {
                                                        c.is_ascii_uppercase()
                                                            || c == '.'
                                                            || c == '/'
                                                    })
                                            })
                                        {
                                            self.symbol_input = ticker.to_string();
                                        }
                                    }
                                }
                            });
                    }
                }
            });

        // ── ADR-094: Toast notification overlay (top-right, stacked) ──────
        self.toasts.retain(|t| !t.is_expired());
        if !self.toasts.is_empty() {
            let mut y_offset = 40.0_f32;
            for (i, toast) in self.toasts.iter_mut().enumerate() {
                let toast_id = egui::Id::new("toast").with(i);
                egui::Area::new(toast_id)
                    .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-10.0, y_offset))
                    .order(egui::Order::Foreground)
                    .show(ctx, |ui| {
                        egui::Frame::popup(ui.style())
                            .fill(egui::Color32::from_rgb(30, 30, 40))
                            .inner_margin(8.0)
                            .rounding(6.0)
                            .stroke(egui::Stroke::new(1.0, toast.color))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(&toast.message).color(toast.color),
                                    );
                                    if toast.dismissable {
                                        if ui.small_button("\u{2716}").clicked() {
                                            toast.dismissed = true;
                                        }
                                    }
                                });
                            });
                    });
                y_offset += 36.0;
            }
        }

        // ── bottom status bar ────────────────────────────────────────────────
        egui::Panel::bottom("status_bar")
            .exact_size(20.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let n_bars = self.charts.first().map(|c| c.bars.len()).unwrap_or(0);
                    let sym = self
                        .charts
                        .first()
                        .map(|c| c.symbol.as_str())
                        .unwrap_or("—");
                    let tf = self
                        .charts
                        .first()
                        .map(|c| c.timeframe.label())
                        .unwrap_or("—");
                    let data_source = self
                        .charts
                        .first()
                        .map(|c| match c.source_override.as_deref() {
                            Some(source) => {
                                format!("Data: {} (selected)", cache_source_label(source))
                            }
                            None if c.primary_source.is_empty() => "Data: unresolved".to_string(),
                            None => {
                                format!("Data: Auto → {}", cache_source_label(c.primary_source))
                            }
                        })
                        .unwrap_or_else(|| "Data: unresolved".to_string());
                    ui.label(
                        egui::RichText::new(format!("TyphooN Terminal"))
                            .color(QUAKE_CMD)
                            .small()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("|")
                            .color(egui::Color32::from_rgb(40, 50, 70))
                            .small(),
                    );
                    ui.label(
                        egui::RichText::new(format!("{} [{}]", sym, tf))
                            .color(egui::Color32::WHITE)
                            .small()
                            .monospace(),
                    );
                    ui.label(
                        egui::RichText::new("|")
                            .color(egui::Color32::from_rgb(40, 50, 70))
                            .small(),
                    );
                    ui.label(
                        egui::RichText::new(format!("{} bars", n_bars))
                            .color(AXIS_TEXT)
                            .small(),
                    );
                    ui.label(
                        egui::RichText::new("|")
                            .color(egui::Color32::from_rgb(40, 50, 70))
                            .small(),
                    );
                    ui.label(
                        egui::RichText::new(data_source)
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    if let Some(err) = &self.cache_err {
                        ui.label(
                            egui::RichText::new(format!(" | {}", err))
                                .color(egui::Color32::from_rgb(255, 80, 80))
                                .small(),
                        );
                    }
                    // Right-aligned: account info
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.broker_connected {
                            ui.label(
                                egui::RichText::new("|")
                                    .color(egui::Color32::from_rgb(40, 50, 70))
                                    .small(),
                            );
                            ui.label(
                                egui::RichText::new(format!("{} pos", self.live_positions.len()))
                                    .color(AXIS_TEXT)
                                    .small(),
                            );
                        }
                    });
                });
            });

        // ── right panel (collapsible sections — all visible, individually expandable) ──
        egui::Panel::right("right_panel")
            .min_size(220.0)
            .max_size(500.0)
            .default_size(320.0)
            .resizable(true)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        let right_panel_order = self.normalized_right_panel_order();
                        for section in right_panel_order {
                            match section {
                                RightPanelSectionId::Trading => {
                        // ── Trading Section ──────────────────────────────────
                        let trading_section = egui::CollapsingHeader::new(
                            egui::RichText::new("☰ Trading").strong().small(),
                        )
                        .default_open(self.right_trading_open)
                        .show(ui, |ui| {
                            // ── Trading Buttons Grid (exact WebKit CSS: #button-grid) ──
                            // LAN client: read-only — disable all trading buttons
                            if self.lan_sync_mode == "client" {
                                ui.add_space(4.0);
                                ui.label(
                                    egui::RichText::new("Read-Only View (LAN Client)")
                                        .color(AXIS_TEXT)
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new("Trade execution disabled — use server")
                                        .color(AXIS_TEXT)
                                        .small(),
                                );
                                ui.add_space(4.0);
                            }
                            let trading_enabled = self.lan_sync_mode != "client";
                            ui.set_enabled(trading_enabled);
                            ui.add_space(8.0);
                            ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                            let btn_w = (ui.available_width() - 4.0) / 2.0;
                            let btn_size = egui::vec2(btn_w, 28.0); // padding: 8px 4px ≈ 28px

                            // Row 1: Open Trade (.btn-action) | Buy Lines (.btn-lines)
                            ui.horizontal(|ui| {
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new("Open Trade")
                                                .color(BTN_GREEN_TEXT)
                                                .small()
                                                .strong(),
                                        )
                                        .fill(BTN_GREEN)
                                        .min_size(btn_size),
                                    )
                                    .clicked()
                                {
                                    self.submit_quick_trade();
                                }
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new("Buy Lines")
                                                .color(BTN_BLUE_TEXT)
                                                .small()
                                                .strong(),
                                        )
                                        .fill(BTN_BLUE)
                                        .min_size(btn_size),
                                    )
                                    .clicked()
                                {
                                    match self.set_visible_range_trade_lines(true) {
                                        Ok((sl, tp)) => {
                                            self.log.push_back(LogEntry::info(format!(
                                                "Buy Lines: SL {} TP {} (drag to adjust)",
                                                format_price(sl),
                                                format_price(tp)
                                            )));
                                        }
                                        Err(e) => self.log.push_back(LogEntry::warn(e)),
                                    }
                                }
                            });
                            // Row 2: Sell Lines (.btn-lines) | Destroy Lines (.btn-lines)
                            ui.horizontal(|ui| {
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new("Sell Lines")
                                                .color(BTN_BLUE_TEXT)
                                                .small()
                                                .strong(),
                                        )
                                        .fill(BTN_BLUE)
                                        .min_size(btn_size),
                                    )
                                    .clicked()
                                {
                                    match self.set_visible_range_trade_lines(false) {
                                        Ok((sl, tp)) => {
                                            self.log.push_back(LogEntry::info(format!(
                                                "Sell Lines: SL {} TP {} (drag to adjust)",
                                                format_price(sl),
                                                format_price(tp)
                                            )));
                                        }
                                        Err(e) => self.log.push_back(LogEntry::warn(e)),
                                    }
                                }
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new("Destroy Lines")
                                                .color(BTN_BLUE_TEXT)
                                                .small()
                                                .strong(),
                                        )
                                        .fill(BTN_BLUE)
                                        .min_size(btn_size),
                                    )
                                    .on_hover_text("Remove all buy/sell planning lines from chart")
                                    .clicked()
                                {
                                    self.clear_trade_lines();
                                }
                            });
                            // Row 3: Open MG (.btn-mg) | Close All (.btn-danger)
                            ui.horizontal(|ui| {
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new("Open MG")
                                                .color(BTN_MG_TEXT)
                                                .small()
                                                .strong(),
                                        )
                                        .fill(BTN_MG)
                                        .min_size(btn_size),
                                    )
                                    .on_hover_text("Open Martingale grid order")
                                    .clicked()
                                {
                                    self.log.push_back(LogEntry::info(
                                        "Martingale: connect broker first",
                                    ));
                                }
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new("Close All")
                                                .color(BTN_RED_TEXT)
                                                .small()
                                                .strong(),
                                        )
                                        .fill(BTN_RED)
                                        .min_size(btn_size),
                                    )
                                    .clicked()
                                {
                                    self.close_all_selected_brokers();
                                }
                            });
                            // Row 4: Close Partial (.btn-danger) | Set SL (.btn-lines)
                            ui.horizontal(|ui| {
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new("Close Partial")
                                                .color(BTN_RED_TEXT)
                                                .small()
                                                .strong(),
                                        )
                                        .fill(BTN_RED)
                                        .min_size(btn_size),
                                    )
                                    .on_hover_text("Close a percentage of open position")
                                    .clicked()
                                {
                                    self.close_partial_active_symbol();
                                }
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new("Set SL")
                                                .color(BTN_BLUE_TEXT)
                                                .small()
                                                .strong(),
                                        )
                                        .fill(BTN_BLUE)
                                        .min_size(btn_size),
                                    )
                                    .clicked()
                                {
                                    self.apply_current_sl_to_positions();
                                }
                            });
                            // Row 5: Set TP (.btn-lines)
                            ui.horizontal(|ui| {
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new("Set TP")
                                                .color(BTN_BLUE_TEXT)
                                                .small()
                                                .strong(),
                                        )
                                        .fill(BTN_BLUE)
                                        .min_size(btn_size),
                                    )
                                    .clicked()
                                {
                                    self.apply_current_tp_to_positions();
                                }
                            });
                            // ADR-094: Position context palette
                            ui.horizontal(|ui| {
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new("Commands…").small().strong(),
                                        )
                                        .min_size(btn_size),
                                    )
                                    .clicked()
                                {
                                    self.palette_context = PaletteContext::Position;
                                    self.command_open = true;
                                    self.command_input.clear();
                                }
                            });
                            ui.add_space(6.0);

                            // ── SL / TP Price Inputs ──────────────────────────
                            ui.separator();
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut self.sl_enabled, "");
                                ui.label(egui::RichText::new("SL Price").color(AXIS_TEXT).small());
                                let resp = ui.add(
                                    egui::TextEdit::singleline(&mut self.sl_input)
                                        .desired_width(100.0)
                                        .hint_text("0.0")
                                        .font(egui::TextStyle::Small),
                                );
                                if resp.lost_focus() && self.sl_enabled {
                                    self.sl_price = self.sl_input.parse().ok();
                                    self.sync_trade_line_inputs();
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut self.tp_enabled, "");
                                ui.label(egui::RichText::new("TP Price").color(AXIS_TEXT).small());
                                let resp = ui.add(
                                    egui::TextEdit::singleline(&mut self.tp_input)
                                        .desired_width(100.0)
                                        .hint_text("0.0")
                                        .font(egui::TextStyle::Small),
                                );
                                if resp.lost_focus() && self.tp_enabled {
                                    self.tp_price = self.tp_input.parse().ok();
                                    self.sync_trade_line_inputs();
                                }
                            });
                            ui.add_space(6.0);

                            // ── Mode / Broker Controls ──────────────────────────
                            ui.separator();
                            let wants_kraken_pro = self.kraken_connected
                                && matches!(self.risk_mode, RiskMode::KrakenPro);
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Mode").color(AXIS_TEXT).small());
                                egui::ComboBox::from_id_salt("risk_mode_combo")
                                    .selected_text(self.risk_mode.label())
                                    .width(96.0)
                                    .show_ui(ui, |ui| {
                                        for mode in [
                                            RiskMode::VaR,
                                            RiskMode::Standard,
                                            RiskMode::Fixed,
                                            RiskMode::Dynamic,
                                        ] {
                                            ui.selectable_value(
                                                &mut self.risk_mode,
                                                mode,
                                                mode.label(),
                                            );
                                        }
                                        if self.kraken_connected
                                            && ui
                                                .selectable_value(
                                                    &mut self.risk_mode,
                                                    RiskMode::KrakenPro,
                                                    RiskMode::KrakenPro.label(),
                                                )
                                                .clicked()
                                        {
                                            self.order_broker = OrderBroker::Kraken;
                                        }
                                    });
                            });
                            if !wants_kraken_pro {
                                match self.risk_mode {
                                    RiskMode::Standard => {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new("Risk %")
                                                    .color(AXIS_TEXT)
                                                    .small(),
                                            );
                                            ui.add(
                                                egui::TextEdit::singleline(
                                                    &mut self.trade_risk_pct_input,
                                                )
                                                .desired_width(64.0)
                                                .hint_text("0.5")
                                                .font(egui::TextStyle::Small),
                                            );
                                        });
                                    }
                                    RiskMode::Fixed => {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new("Qty")
                                                    .color(AXIS_TEXT)
                                                    .small(),
                                            );
                                            ui.add(
                                                egui::TextEdit::singleline(&mut self.order_qty)
                                                    .desired_width(80.0)
                                                    .hint_text("1.0")
                                                    .font(egui::TextStyle::Small),
                                            );
                                        });
                                    }
                                    RiskMode::Dynamic => {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new("Min Bal")
                                                    .color(AXIS_TEXT)
                                                    .small(),
                                            );
                                            ui.add(
                                                egui::TextEdit::singleline(
                                                    &mut self.trade_min_balance_input,
                                                )
                                                .desired_width(80.0)
                                                .hint_text("96100")
                                                .font(egui::TextStyle::Small),
                                            );
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new("Losses")
                                                    .color(AXIS_TEXT)
                                                    .small(),
                                            );
                                            ui.add(
                                                egui::TextEdit::singleline(
                                                    &mut self.trade_losses_to_min_input,
                                                )
                                                .desired_width(64.0)
                                                .hint_text("10")
                                                .font(egui::TextStyle::Small),
                                            );
                                        });
                                    }
                                    RiskMode::VaR | RiskMode::KrakenPro => {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new("VaR %")
                                                    .color(AXIS_TEXT)
                                                    .small(),
                                            );
                                            ui.add(
                                                egui::TextEdit::singleline(
                                                    &mut self.trade_var_risk_pct_input,
                                                )
                                                .desired_width(64.0)
                                                .hint_text("0.9")
                                                .font(egui::TextStyle::Small),
                                            );
                                        });
                                    }
                                }
                                if let Ok(plan) = self.quick_trade_plan() {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "Setup {} {:.4}",
                                                if plan.side_idx == 0 { "BUY" } else { "SELL" },
                                                plan.qty
                                            ))
                                            .color(if plan.side_idx == 0 { UP } else { DOWN })
                                            .small()
                                            .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new(plan.symbol.clone())
                                                .color(AXIS_TEXT)
                                                .small(),
                                        );
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "Risk ${:.2}",
                                                plan.risk_dollars
                                            ))
                                            .color(DOWN)
                                            .small(),
                                        );
                                        if let Some(risk_pct) = plan.risk_pct {
                                            ui.label(
                                                egui::RichText::new(format!("({:.2}%)", risk_pct))
                                                    .color(AXIS_TEXT)
                                                    .small(),
                                            );
                                        }
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "TP ${:.2}",
                                                plan.reward_dollars
                                            ))
                                            .color(UP)
                                            .small(),
                                        );
                                        if let Some(rr) = plan.rr {
                                            ui.label(
                                                egui::RichText::new(format!("RR {:.2}", rr))
                                                    .color(AXIS_TEXT)
                                                    .small(),
                                            );
                                        }
                                    });
                                } else if self.sl_price.is_some() || self.tp_price.is_some() {
                                    if let Err(e) = self.quick_trade_plan() {
                                        ui.label(
                                            egui::RichText::new(e)
                                                .color(AXIS_TEXT)
                                                .small(),
                                        );
                                    }
                                }
                            }
                            // Broker target selector (only show when any broker connected)
                            if self.broker_connected || self.tt_connected || self.kraken_connected {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new("Broker").color(AXIS_TEXT).small(),
                                    );
                                    egui::ComboBox::from_id_salt("order_broker_combo")
                                        .selected_text(self.order_broker.label())
                                        .width(90.0)
                                        .show_ui(ui, |ui| {
                                            if self.broker_connected {
                                                ui.selectable_value(
                                                    &mut self.order_broker,
                                                    OrderBroker::Alpaca,
                                                    "Alpaca",
                                                );
                                            }
                                            if self.tt_connected {
                                                ui.selectable_value(
                                                    &mut self.order_broker,
                                                    OrderBroker::Tastytrade,
                                                    "tastytrade",
                                                );
                                            }
                                            if self.kraken_connected {
                                                ui.selectable_value(
                                                    &mut self.order_broker,
                                                    OrderBroker::Kraken,
                                                    "Kraken",
                                                );
                                            }
                                            if self.broker_connected && self.tt_connected {
                                                ui.selectable_value(
                                                    &mut self.order_broker,
                                                    OrderBroker::Both,
                                                    "Both",
                                                );
                                            }
                                        });
                                });
                            }
                            ui.add_space(6.0);

                            if wants_kraken_pro {
                                self.render_kraken_spot_buy_controls(ui);
                                ui.add_space(6.0);
                            }

                            // ── Position Info Block ────────────────────────────
                            ui.separator();
                            if let Some(chart) = self.charts.get(self.active_tab) {
                                if let Some(bar) = chart.bars.last() {
                                    let close = bar.close;
                                    // Show current position info if any
                                    let mut has_pos = false;
                                    let chart_symbol = chart.symbol.split(':').last().unwrap_or("");
                                    let all_positions = self
                                        .live_positions
                                        .iter()
                                        .chain(self.tt_positions.iter())
                                        .chain(self.kr_positions.iter());
                                    for pos in all_positions {
                                        if pos
                                            .symbol
                                            .contains(chart_symbol)
                                        {
                                            let side_c = if pos.side == "long" { UP } else { DOWN };
                                            let side_label =
                                                if pos.side == "long" { "Long" } else { "Short" };
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{} {:.2} lots",
                                                    side_label, pos.qty
                                                ))
                                                .color(side_c)
                                                .strong(),
                                            );
                                            let pl_c =
                                                if pos.unrealized_pl >= 0.0 { UP } else { DOWN };
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "P&L: ${:.2}",
                                                    pos.unrealized_pl
                                                ))
                                                .color(pl_c),
                                            );

                                            // SL/TP P&L if set
                                            if let Some(sl) = self.sl_price {
                                                let sl_pl = (close - sl)
                                                    * pos.qty
                                                    * if pos.side == "long" { 1.0 } else { -1.0 };
                                                let sl_c = if sl_pl >= 0.0 { UP } else { DOWN };
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "SL P/L: ${:.2}",
                                                        sl_pl
                                                    ))
                                                    .color(sl_c)
                                                    .small(),
                                                );
                                            }
                                            if let Some(tp) = self.tp_price {
                                                let tp_pl = (tp - close)
                                                    * pos.qty
                                                    * if pos.side == "long" { 1.0 } else { -1.0 };
                                                let tp_c = if tp_pl >= 0.0 { UP } else { DOWN };
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "TP P/L: ${:.2}",
                                                        tp_pl
                                                    ))
                                                    .color(tp_c)
                                                    .small(),
                                                );
                                            }
                                            if let (Some(sl), Some(tp)) =
                                                (self.sl_price, self.tp_price)
                                            {
                                                let risk = (close - sl).abs();
                                                let reward = (tp - close).abs();
                                                let rr =
                                                    if risk > 0.0 { reward / risk } else { 0.0 };
                                                ui.label(
                                                    egui::RichText::new(format!("R:R {:.2}", rr))
                                                        .color(AXIS_TEXT)
                                                        .small(),
                                                );
                                            }
                                            has_pos = true;
                                            break;
                                        }
                                    }
                                    if !has_pos {
                                        ui.label(
                                            egui::RichText::new("No position")
                                                .color(AXIS_TEXT)
                                                .small(),
                                        );
                                    }

                                    let account_snaps: Vec<_> = self
                                        .selected_trade_account_snapshots()
                                        .into_iter()
                                        .filter(|snap| match snap.broker {
                                            "Alpaca" => self.show_alpaca_positions,
                                            "tastytrade" | "Tastytrade" | "Tasty" => {
                                                self.show_tt_positions
                                            }
                                            "Kraken" => self.show_kr_positions,
                                            _ => true,
                                        })
                                        .collect();
                                    if !account_snaps.is_empty() {
                                        ui.add_space(4.0);
                                        ui.horizontal_wrapped(|ui| {
                                            for snap in &account_snaps {
                                                let is_alpaca = snap.broker == "Alpaca";
                                                let is_live = !is_alpaca || !self.broker_paper;
                                                let mode = if is_alpaca && self.broker_paper {
                                                    "Paper"
                                                } else {
                                                    "Live"
                                                };
                                                let color = if is_live { UP } else { egui::Color32::WHITE };
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "[{} ({}) ${:.0}]",
                                                        snap.broker, mode, snap.buying_power
                                                    ))
                                                    .color(color)
                                                    .small()
                                                    .strong(),
                                                );
                                            }
                                        });
                                    }
                                }
                            }
                        });
                        self.right_trading_open = trading_section.fully_open();
                        self.handle_right_panel_section_drag(
                            ui,
                            RightPanelSectionId::Trading,
                            &trading_section.header_response,
                        );
                                }
                                RightPanelSectionId::Positions => {

                        // ── Positions Section ─────────────────────────────────
                        let darwin_count = if self.show_darwin_positions {
                            self.bg.open_positions.len()
                        } else {
                            0
                        };
                        let alpaca_count = if self.show_alpaca_positions {
                            self.live_positions.len()
                        } else {
                            0
                        };
                        let tt_count = if self.show_tt_positions {
                            self.tt_positions.len()
                        } else {
                            0
                        };
                        let kr_count = if self.show_kr_positions {
                            self.kr_positions.len()
                        } else {
                            0
                        };
                        let pos_count = darwin_count + alpaca_count + tt_count + kr_count;
                        let (pos_stale_lbl, pos_stale_col) =
                            self.staleness_badge(self.positions_last_update_ts);
                        let pos_header = format!("☰ Positions ({})  •  {}", pos_count, pos_stale_lbl);
                        let positions_section = egui::CollapsingHeader::new(
                            egui::RichText::new(pos_header)
                                .strong()
                                .small()
                                .color(pos_stale_col),
                        )
                        .id_salt("positions_section")
                        .default_open(self.right_positions_open)
                        .show(ui, |ui| {
                            // Visibility toggles (compact horizontal checkboxes)
                            ui.horizontal(|ui| {
                                ui.checkbox(
                                    &mut self.show_darwin_positions,
                                    egui::RichText::new("DARWIN").small(),
                                );
                                ui.checkbox(
                                    &mut self.show_alpaca_positions,
                                    egui::RichText::new("Alpaca").small(),
                                );
                                ui.checkbox(
                                    &mut self.show_tt_positions,
                                    egui::RichText::new("Tasty").small(),
                                );
                                ui.checkbox(
                                    &mut self.show_kr_positions,
                                    egui::RichText::new("Kraken").small(),
                                );
                            });
                            ui.add_space(4.0);
                            let mut has_positions = false;
                            // DARWIN positions — show current chart symbol first, then others dimmed
                            {
                                let positions = &self.bg.open_positions;
                                if !positions.is_empty() && self.show_darwin_positions {
                                    has_positions = true;
                                    // PERF: rsplit avoids allocating Vec<&str> via split().collect().
                                    let active_sym: String = self
                                        .charts
                                        .get(self.active_tab)
                                        .map(|c| {
                                            let s = c.symbol.as_str();
                                            let mut it = s.rsplit(':');
                                            let last = it.next().unwrap_or("");
                                            let is_tf = matches!(
                                                last,
                                                "1Min"
                                                    | "5Min"
                                                    | "15Min"
                                                    | "30Min"
                                                    | "1Hour"
                                                    | "4Hour"
                                                    | "1Day"
                                                    | "1Week"
                                                    | "1Month"
                                            );
                                            if is_tf {
                                                it.next().unwrap_or(last).to_string()
                                            } else {
                                                last.to_string()
                                            }
                                        })
                                        .unwrap_or_default();
                                    // Current symbol positions first
                                    for pos in positions.iter() {
                                        let is_active = pos.symbol == active_sym;
                                        if !is_active {
                                            continue;
                                        }
                                        let side_c = if pos.side == "buy" { UP } else { DOWN };
                                        ui.horizontal_wrapped(|ui| {
                                            ui.label(
                                                egui::RichText::new(&pos.symbol).small().strong(),
                                            );
                                            let side_label =
                                                if pos.side == "buy" { "L" } else { "S" };
                                            ui.label(
                                                egui::RichText::new(side_label)
                                                    .color(side_c)
                                                    .small(),
                                                );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{:.2}",
                                                    pos.total_volume
                                                ))
                                                .small(),
                                            );
                                            let pl_c = if pos.notional >= 0.0 { UP } else { DOWN };
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "${:.0}",
                                                    pos.notional
                                                ))
                                                .color(pl_c)
                                                .small(),
                                            );
                                        });
                                        // PERF: &str join skips N String::clone() allocations per frame per position.
                                        let darwins: Vec<&str> = pos
                                            .darwin_breakdown
                                            .iter()
                                            .map(|(d, _, _)| d.as_str())
                                            .collect();
                                        ui.add(
                                            egui::Label::new(
                                                egui::RichText::new(darwins.join(", "))
                                                    .color(AXIS_TEXT)
                                                    .small(),
                                            )
                                            .wrap(),
                                        );
                                        ui.separator();
                                    }
                                    // Other positions (dimmed)
                                    for pos in positions.iter() {
                                        let is_active = pos.symbol == active_sym;
                                        if is_active {
                                            continue;
                                        }
                                        let dim = egui::Color32::from_rgb(90, 90, 100);
                                        let side_c = if pos.side == "buy" {
                                            egui::Color32::from_rgb(60, 120, 60)
                                        } else {
                                            egui::Color32::from_rgb(120, 60, 60)
                                        };
                                        ui.horizontal_wrapped(|ui| {
                                            ui.label(
                                                egui::RichText::new(&pos.symbol).small().color(dim),
                                            );
                                            let side_label =
                                                if pos.side == "buy" { "L" } else { "S" };
                                            ui.label(
                                                egui::RichText::new(side_label)
                                                    .color(side_c)
                                                    .small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{:.2}",
                                                    pos.total_volume
                                                ))
                                                .small()
                                                .color(dim),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "${:.0}",
                                                    pos.notional
                                                ))
                                                .color(dim)
                                                .small(),
                                            );
                                        });
                                        let darwins: Vec<&str> = pos
                                            .darwin_breakdown
                                            .iter()
                                            .map(|(d, _, _)| d.as_str())
                                            .collect();
                                        ui.add(
                                            egui::Label::new(
                                                egui::RichText::new(darwins.join(", "))
                                                    .color(egui::Color32::from_rgb(60, 60, 70))
                                                    .small(),
                                            )
                                            .wrap(),
                                        );
                                        ui.separator();
                                    }
                                }
                            }
                            // Live broker positions (from Alpaca or synced from LAN server via KV)
                            let has_live = (self.broker_connected
                                || self.lan_sync_mode == "client")
                                && self.show_alpaca_positions;
                            if has_live && !self.live_positions.is_empty() {
                                has_positions = true;
                                let mut close_sym: Option<String> = None;
                                let mut lp_action = SymbolAction::None;
                                for pos in &self.live_positions {
                                    let side_c = if pos.side == "long" { UP } else { DOWN };
                                    let side_label = if pos.side == "long" { "L" } else { "S" };
                                    let current_price = if pos.qty.abs() > f64::EPSILON {
                                        Some(pos.market_value.abs() / pos.qty.abs())
                                    } else {
                                        None
                                    };
                                    ui.horizontal_wrapped(|ui| {
                                        let (_, act) = symbol_label_with_menu(
                                            ui,
                                            &pos.symbol,
                                            egui::RichText::new(&pos.symbol).small().strong(),
                                        );
                                        if !matches!(act, SymbolAction::None) {
                                            lp_action = act;
                                        }
                                        ui.label(
                                            egui::RichText::new(side_label).color(side_c).small(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!("{:.2}", pos.qty)).small(),
                                        );
                                        let pl_c = if pos.unrealized_pl >= 0.0 { UP } else { DOWN };
                                        let pl_pct = if pos.market_value.abs() > 0.01 {
                                            pos.unrealized_pl
                                                / (pos.market_value - pos.unrealized_pl)
                                                * 100.0
                                        } else {
                                            0.0
                                        };
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "${:.2} ({:+.1}%)",
                                                pos.unrealized_pl, pl_pct
                                            ))
                                            .color(pl_c)
                                            .small(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "entry {}  cur {}",
                                                format_price(pos.avg_entry_price),
                                                current_price
                                                    .map(format_price)
                                                    .unwrap_or_else(|| "—".to_string())
                                            ))
                                            .color(AXIS_TEXT)
                                            .small(),
                                        );
                                        if self.broker_connected && self.lan_sync_mode != "client" {
                                            if ui
                                                .small_button(egui::RichText::new("x").color(DOWN))
                                                .on_hover_text("Close position")
                                                .clicked()
                                            {
                                                close_sym = Some(pos.symbol.clone());
                                            }
                                        }
                                    });
                                    ui.separator();
                                }
                                if let Some(sym) = close_sym {
                                    let _ = self
                                        .broker_tx
                                        .send(BrokerCmd::ClosePosition { symbol: sym, qty: None });
                                }
                                if !matches!(lp_action, SymbolAction::None) {
                                    self.deferred_symbol_action = lp_action;
                                }
                            }
                            // tastytrade positions
                            if self.show_tt_positions && !self.tt_positions.is_empty() {
                                has_positions = true;
                                let mut close_sym: Option<String> = None;
                                let mut tt_action = SymbolAction::None;
                                for pos in &self.tt_positions {
                                    let side_c = if pos.side == "long" { UP } else { DOWN };
                                    let side_label = if pos.side == "long" { "L" } else { "S" };
                                    let current_price = if pos.qty.abs() > f64::EPSILON {
                                        Some(pos.market_value.abs() / pos.qty.abs())
                                    } else {
                                        None
                                    };
                                    ui.horizontal_wrapped(|ui| {
                                        let (_, act) = symbol_label_with_menu(
                                            ui,
                                            &pos.symbol,
                                            egui::RichText::new(&pos.symbol).small().strong(),
                                        );
                                        if !matches!(act, SymbolAction::None) {
                                            tt_action = act;
                                        }
                                        ui.label(
                                            egui::RichText::new(format!("[Tasty] {}", side_label))
                                                .color(side_c)
                                                .small(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!("{:.2}", pos.qty)).small(),
                                        );
                                        let pl_c = if pos.unrealized_pl >= 0.0 { UP } else { DOWN };
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "${:.2}",
                                                pos.unrealized_pl
                                            ))
                                            .color(pl_c)
                                            .small(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "entry {}  cur {}",
                                                format_price(pos.avg_entry_price),
                                                current_price
                                                    .map(format_price)
                                                    .unwrap_or_else(|| "—".to_string())
                                            ))
                                            .color(AXIS_TEXT)
                                            .small(),
                                        );
                                        // Close button — submits a market order in the opposite direction.
                                        if ui
                                            .small_button("X")
                                            .on_hover_text(format!(
                                                "Close {} position at market",
                                                pos.symbol
                                            ))
                                            .clicked()
                                        {
                                            close_sym = Some(pos.symbol.clone());
                                        }
                                    });
                                    ui.separator();
                                }
                                if let Some(sym) = close_sym {
                                    let _ = self.broker_tx.send(
                                        BrokerCmd::TastytradeClosePositionQty {
                                            symbol: sym.clone(),
                                            qty: None,
                                        },
                                    );
                                    self.log.push_back(LogEntry::info(format!(
                                        "Tastytrade: closing {sym} at market"
                                    )));
                                }
                                if !matches!(tt_action, SymbolAction::None) {
                                    self.deferred_symbol_action = tt_action;
                                }
                            }
                            if self.show_kr_positions && !self.kr_positions.is_empty() {
                                has_positions = true;
                                let mut close_sym: Option<String> = None;
                                let mut kr_action = SymbolAction::None;
                                for pos in &self.kr_positions {
                                    let side_c = if pos.side == "long" { UP } else { DOWN };
                                    let side_label = if pos.side == "long" { "L" } else { "S" };
                                    let avg_entry = if pos.avg_entry_price > 0.0 {
                                        Some(pos.avg_entry_price)
                                    } else {
                                        self.kraken_position_avg_price(&pos.symbol)
                                    };
                                    let current_price = self.latest_cached_price_for_symbol(&pos.symbol);
                                    let derived_unrealized_pl = avg_entry.zip(current_price).map(|(avg, cur)| {
                                        let dir = if pos.side == "short" { -1.0 } else { 1.0 };
                                        (cur - avg) * pos.qty * dir
                                    });
                                    let display_pl = if pos.unrealized_pl.abs() > f64::EPSILON {
                                        pos.unrealized_pl
                                    } else {
                                        derived_unrealized_pl.unwrap_or(0.0)
                                    };
                                    ui.horizontal_wrapped(|ui| {
                                        let (_, act) = symbol_label_with_menu(
                                            ui,
                                            &pos.symbol,
                                            egui::RichText::new(&pos.symbol).small().strong(),
                                        );
                                        if !matches!(act, SymbolAction::None) {
                                            kr_action = act;
                                        }
                                        ui.label(
                                            egui::RichText::new(format!("[Kraken] {}", side_label))
                                                .color(side_c)
                                                .small(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!("{:.6}", pos.qty)).small(),
                                        );
                                        let pl_c = if display_pl >= 0.0 { UP } else { DOWN };
                                        ui.label(
                                            egui::RichText::new(format!("${:.2}", display_pl))
                                                .color(pl_c)
                                                .small(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "entry {}  cur {}",
                                                avg_entry
                                                    .map(format_price)
                                                    .unwrap_or_else(|| "—".to_string()),
                                                current_price
                                                    .map(format_price)
                                                    .unwrap_or_else(|| "—".to_string())
                                            ))
                                            .color(AXIS_TEXT)
                                            .small(),
                                        );
                                        if ui
                                            .small_button("Close")
                                            .on_hover_text(format!(
                                                "Close active Kraken position {} at market",
                                                pos.symbol
                                            ))
                                            .clicked()
                                        {
                                            close_sym = Some(pos.symbol.clone());
                                        }
                                    });
                                    ui.separator();
                                }
                                if let Some(sym) = close_sym {
                                    let _ = self.broker_tx.send(BrokerCmd::KrakenClosePosition {
                                        pair: sym.clone(),
                                        volume: None,
                                    });
                                    self.log.push_back(LogEntry::info(format!(
                                        "Kraken: closing active position {sym} at market"
                                    )));
                                }
                                if !matches!(kr_action, SymbolAction::None) {
                                    self.deferred_symbol_action = kr_action;
                                }
                            }
                            let kraken_sellable_balances: Vec<(String, f64)> = self
                                .kraken_balances
                                .iter()
                                .filter(|(asset, qty)| {
                                    qty.is_finite()
                                        && *qty > 0.0
                                        && !Self::kraken_is_cash_balance_asset(asset)
                                })
                                .cloned()
                                .collect();
                            if self.show_kr_positions && !kraken_sellable_balances.is_empty() {
                                let mut sell_balance: Option<(String, f64)> = None;
                                for (asset, qty) in kraken_sellable_balances {
                                    let display_asset = Self::kraken_display_asset(&asset);
                                    let display_holding = display_asset
                                        .strip_suffix(".EQ")
                                        .unwrap_or(display_asset.as_str())
                                        .to_string();
                                    let pair = Self::kraken_spot_pair_for_balance_asset(&asset);
                                    let avg_price = self.kraken_balance_avg_price(&asset);
                                    let current_price = self.latest_cached_price_for_symbol(&pair);
                                    let pl = avg_price
                                        .zip(current_price)
                                        .map(|(avg, cur)| ((cur - avg) * qty, (cur - avg) / avg * 100.0));
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new(format!("[Kraken] {display_holding}"))
                                                .small()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!("{qty:.8} {display_holding}"))
                                                .small()
                                                .monospace(),
                                        );
                                        if let Some(avg) = avg_price {
                                            ui.label(
                                                egui::RichText::new(format!("avg {}", format_price(avg)))
                                                    .color(AXIS_TEXT)
                                                    .small(),
                                            );
                                        }
                                        if let Some(cur) = current_price {
                                            ui.label(
                                                egui::RichText::new(format!("cur {}", format_price(cur)))
                                                    .color(AXIS_TEXT)
                                                    .small(),
                                            );
                                        }
                                        if let Some((pl_value, pl_pct)) = pl {
                                            let c = if pl_value >= 0.0 { UP } else { DOWN };
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "P/L ${:+.2} ({:+.2}%)",
                                                    pl_value, pl_pct
                                                ))
                                                .color(c)
                                                .small(),
                                            );
                                        }
                                        if ui
                                            .small_button("Sell…")
                                            .on_hover_text(format!(
                                                "Open Kraken sell ticket for {display_asset}; choose lots with a slider"
                                            ))
                                            .clicked()
                                        {
                                            sell_balance = Some((asset.clone(), qty));
                                        }
                                    });
                                }
                                if let Some((asset, qty)) = sell_balance {
                                    self.open_kraken_spot_sell_dialog(asset, qty);
                                }
                                ui.separator();
                            }
                            if !has_positions {
                                ui.label(
                                    egui::RichText::new("No open positions.")
                                        .color(AXIS_TEXT)
                                        .small(),
                                );
                            }
                        });
                        self.right_positions_open = positions_section.fully_open();
                        self.handle_right_panel_section_drag(
                            ui,
                            RightPanelSectionId::Positions,
                            &positions_section.header_response,
                        );
                                }
                                RightPanelSectionId::RecentFills => {

                        // ── Recent Fills Section ──────────────────────────────
                        let mut visible_recent_fills: Vec<(String, String, f64, f64, String)> =
                            Vec::new();
                        if self.show_alpaca_positions {
                            visible_recent_fills.extend(self.recent_fills.iter().cloned());
                        }
                        if self.show_kr_positions {
                            visible_recent_fills.extend(self.kraken_trades.iter().take(100).map(|t| {
                                let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(
                                    t.time as i64,
                                    0,
                                )
                                .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                                .unwrap_or_else(|| format!("{:.0}", t.time));
                                (
                                    format!(
                                        "[Kraken] {}",
                                        Self::kraken_base_asset_for_pair(&t.pair)
                                    ),
                                    t.side.clone(),
                                    t.vol,
                                    t.price,
                                    dt,
                                )
                            }));
                        }
                        let fills_count2 = visible_recent_fills.len();
                        let recent_fills_section = egui::CollapsingHeader::new(
                            egui::RichText::new(format!("☰ Recent Fills ({})", fills_count2))
                                .strong()
                                .small(),
                        )
                        .id_salt("recent_fills_top")
                        .default_open(self.right_recent_fills_open)
                        .show(ui, |ui| {
                            if visible_recent_fills.is_empty() {
                                ui.label(
                                    egui::RichText::new("No recent fills.")
                                        .color(AXIS_TEXT)
                                        .small(),
                                );
                            } else {
                                for (sym, side, qty, price, time) in &visible_recent_fills {
                                        let c = if side == "buy" { UP } else { DOWN };
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new(sym).small().strong());
                                            ui.label(egui::RichText::new(side).color(c).small());
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{:.2}@{}",
                                                    qty,
                                                    format_price(*price)
                                                ))
                                                .small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(time).color(AXIS_TEXT).small(),
                                            );
                                        });
                                    }
                            }
                        });
                        self.right_recent_fills_open = recent_fills_section.fully_open();
                        self.handle_right_panel_section_drag(
                            ui,
                            RightPanelSectionId::RecentFills,
                            &recent_fills_section.header_response,
                        );
                                }
                                RightPanelSectionId::Orders => {

                        // ── Orders Section ────────────────────────────────────
                        let ord_count = if self.show_alpaca_positions {
                            self.live_orders.len()
                        } else {
                            0
                        };
                        let (ord_stale_lbl, ord_stale_col) =
                            self.staleness_badge(self.orders_last_update_ts);
                        let ord_header = format!("☰ Orders ({})  •  {}", ord_count, ord_stale_lbl);
                        let orders_section = egui::CollapsingHeader::new(
                            egui::RichText::new(ord_header)
                                .strong()
                                .small()
                                .color(ord_stale_col),
                        )
                        .id_salt("orders_section")
                        .default_open(self.right_orders_open)
                        .show(ui, |ui| {
                            ui.add_space(4.0);
                            if (self.broker_connected || self.lan_sync_mode == "client")
                                && self.show_alpaca_positions
                                && !self.live_orders.is_empty()
                            {
                                let mut cancel_id: Option<String> = None;
                                let mut lo_action = SymbolAction::None;
                                for order in &self.live_orders {
                                    ui.horizontal(|ui| {
                                        let (_, act) = symbol_label_with_menu(
                                            ui,
                                            &order.symbol,
                                            egui::RichText::new(&order.symbol).small().strong(),
                                        );
                                        if !matches!(act, SymbolAction::None) {
                                            lo_action = act;
                                        }
                                        let side_c = if order.side == "buy" { UP } else { DOWN };
                                        ui.label(
                                            egui::RichText::new(&order.side).color(side_c).small(),
                                        );
                                        ui.label(
                                            egui::RichText::new(&order.order_type)
                                                .color(AXIS_TEXT)
                                                .small(),
                                        );
                                        if self.broker_connected && self.lan_sync_mode != "client" {
                                            if ui
                                                .small_button(egui::RichText::new("X").color(DOWN))
                                                .on_hover_text("Cancel order")
                                                .clicked()
                                            {
                                                cancel_id = Some(order.id.clone());
                                            }
                                        }
                                    });
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "qty: {} | {}",
                                            order.qty, order.status
                                        ))
                                        .color(ACCENT)
                                        .small(),
                                    );
                                    ui.separator();
                                }
                                if let Some(oid) = cancel_id {
                                    let _ = self
                                        .broker_tx
                                        .send(BrokerCmd::AlpacaCancelOrder { order_id: oid });
                                }
                                if !matches!(lo_action, SymbolAction::None) {
                                    self.deferred_symbol_action = lo_action;
                                }
                            } else {
                                let msg = if self.broker_connected || self.lan_sync_mode == "client"
                                {
                                    "No open orders."
                                } else {
                                    "Connect broker for live orders."
                                };
                                ui.label(egui::RichText::new(msg).color(AXIS_TEXT).small());
                            }
                        });
                        self.right_orders_open = orders_section.fully_open();
                        self.handle_right_panel_section_drag(
                            ui,
                            RightPanelSectionId::Orders,
                            &orders_section.header_response,
                        );
                                }
                                RightPanelSectionId::Watchlist => {

                        // ── Watchlist: populate from cache for symbols not yet in rows ──
                        {
                            let have_syms: std::collections::HashSet<&str> = self
                                .watchlist_rows
                                .iter()
                                .map(|r| r.symbol.as_str())
                                .collect();
                            let missing: Vec<String> = self
                                .user_watchlist
                                .iter()
                                .filter(|s| !have_syms.contains(s.as_str()))
                                .cloned()
                                .collect();
                            if !missing.is_empty() && !self.watchlist_cache_tried {
                                self.watchlist_cache_tried = true;
                                if let Some(ref cache) = self.cache {
                                    let tf = self
                                        .charts
                                        .get(self.active_tab)
                                        .map(|c| c.timeframe.cache_suffix().to_string())
                                        .unwrap_or_else(|| "1Day".to_string());
                                    let mut rows: Vec<WatchlistRow> = self.watchlist_rows.clone();
                                    for sym in &missing {
                                        let candidates = [
                                            format!("mt5:{}:{}", sym, tf),
                                            format!("mt5:Darwinex:{}:{}", sym, tf),
                                            format!("{}:{}", sym, tf),
                                            format!("alpaca:{}:{}", sym, tf),
                                            format!("default:{}:{}", sym, tf),
                                        ];
                                        let mut found = false;
                                        for key in &candidates {
                                            if let Ok(Some(raw)) = cache.get_bars_raw(key) {
                                                if raw.len() >= 2 {
                                                    let last_bar = &raw[raw.len() - 1];
                                                    let prev_bar = &raw[raw.len() - 2];
                                                    let change = last_bar.3 - prev_bar.3;
                                                    let change_pct = if prev_bar.3 > 0.0 {
                                                        change / prev_bar.3 * 100.0
                                                    } else {
                                                        0.0
                                                    };
                                                    rows.push(WatchlistRow {
                                                        symbol: sym.clone(),
                                                        cache_key: key.clone(),
                                                        last: last_bar.3,
                                                        prev_close: prev_bar.3,
                                                        change,
                                                        change_pct,
                                                        volume: last_bar.5,
                                                        ext_change_pct: 0.0,
                                                    });
                                                    found = true;
                                                    break;
                                                }
                                            }
                                        }
                                        if !found {
                                            {
                                                let stats = &self.bg.detailed_stats;
                                                let sym_lower = sym.to_lowercase();
                                                let tf_lower = tf.to_lowercase();
                                                for (k, _, _) in stats {
                                                    // BCW metadata keys (`mt5:__NAME__:…`) never
                                                    // hold bar blobs — the contains-match below
                                                    // could otherwise hit them for a symbol like
                                                    // "HEART" or a TF equal to an account tag.
                                                    if k.contains(":__") {
                                                        continue;
                                                    }
                                                    let kl = k.to_lowercase();
                                                    if kl.contains(&sym_lower)
                                                        && kl.ends_with(&tf_lower)
                                                    {
                                                        if let Ok(Some(raw)) = cache.get_bars_raw(k)
                                                        {
                                                            if raw.len() >= 2 {
                                                                let last_bar = &raw[raw.len() - 1];
                                                                let prev_bar = &raw[raw.len() - 2];
                                                                let change =
                                                                    last_bar.3 - prev_bar.3;
                                                                let change_pct = if prev_bar.3 > 0.0
                                                                {
                                                                    change / prev_bar.3 * 100.0
                                                                } else {
                                                                    0.0
                                                                };
                                                                rows.push(WatchlistRow {
                                                                    symbol: sym.clone(),
                                                                    cache_key: k.clone(),
                                                                    last: last_bar.3,
                                                                    prev_close: prev_bar.3,
                                                                    change,
                                                                    change_pct,
                                                                    volume: last_bar.5,
                                                                    ext_change_pct: 0.0,
                                                                });
                                                                break;
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    if rows.len() > self.watchlist_rows.len() {
                                        self.watchlist_rows = rows;
                                    }
                                }
                            }
                        }

                        // ── Watchlist Section ─────────────────────────────────
                        let wl_count = self.user_watchlist.len();
                        let (wl_stale_lbl, wl_stale_col) =
                            self.staleness_badge(self.watchlist_last_update_ts);
                        let wl_header = format!("☰ Watchlist ({})  •  {}", wl_count, wl_stale_lbl);
                        let watchlist_section = egui::CollapsingHeader::new(
                            egui::RichText::new(wl_header)
                                .strong()
                                .small()
                                .color(wl_stale_col),
                        )
                        .id_salt("watchlist_section") // stable ID — don't reset on count change
                        .default_open(self.right_watchlist_open)
                        .show(ui, |ui| {
                            // ── Add symbol input ──────────────────────────
                            ui.add_space(2.0);
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 4.0;
                                let te = egui::TextEdit::singleline(&mut self.watchlist_input)
                                    .desired_width(80.0)
                                    .hint_text("Symbol")
                                    .font(egui::TextStyle::Small)
                                    .text_color(egui::Color32::WHITE);
                                let te_resp = ui.add(te);
                                let enter_pressed = te_resp.lost_focus()
                                    && ui.input(|i| i.key_pressed(egui::Key::Enter));
                                if (ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new("+").color(UP).small(),
                                        )
                                        .min_size(egui::vec2(20.0, 18.0)),
                                    )
                                    .clicked()
                                    || enter_pressed)
                                    && !self.watchlist_input.trim().is_empty()
                                {
                                    let sym = self.watchlist_input.trim().to_uppercase();
                                    if !self.user_watchlist.contains(&sym) {
                                        self.user_watchlist.push(sym);
                                        self.watchlist_cache_tried = false; // retry cache lookup
                                        // Trigger immediate refresh
                                        if self.broker_connected {
                                            let _ = self.broker_tx.send(
                                                BrokerCmd::GetWatchlistQuotes {
                                                    symbols: self.user_watchlist.clone(),
                                                },
                                            );
                                        }
                                    }
                                    self.watchlist_input.clear();
                                }
                            });
                            ui.add_space(2.0);

                            // Sort watchlist rows
                            let mut sorted_wl: Vec<&WatchlistRow> =
                                self.watchlist_rows.iter().collect();
                            match self.watchlist_sort.column {
                                0 => sorted_wl.sort_by(|a, b| a.symbol.cmp(&b.symbol)),
                                1 => sorted_wl.sort_by(|a, b| {
                                    a.last
                                        .partial_cmp(&b.last)
                                        .unwrap_or(std::cmp::Ordering::Equal)
                                }),
                                2 => sorted_wl.sort_by(|a, b| {
                                    a.change
                                        .partial_cmp(&b.change)
                                        .unwrap_or(std::cmp::Ordering::Equal)
                                }),
                                3 => sorted_wl.sort_by(|a, b| {
                                    a.change_pct
                                        .partial_cmp(&b.change_pct)
                                        .unwrap_or(std::cmp::Ordering::Equal)
                                }),
                                4 => sorted_wl.sort_by(|a, b| {
                                    a.volume
                                        .partial_cmp(&b.volume)
                                        .unwrap_or(std::cmp::Ordering::Equal)
                                }),
                                5 => sorted_wl.sort_by(|a, b| {
                                    a.ext_change_pct
                                        .partial_cmp(&b.ext_change_pct)
                                        .unwrap_or(std::cmp::Ordering::Equal)
                                }),
                                _ => {}
                            }
                            if !self.watchlist_sort.ascending {
                                sorted_wl.reverse();
                            }

                            if self.watchlist_rows.is_empty() && self.user_watchlist.is_empty() {
                                ui.label(
                                    egui::RichText::new("Add symbols above.")
                                        .color(AXIS_TEXT)
                                        .small(),
                                );
                            } else if self.watchlist_rows.is_empty() {
                                ui.label(
                                    egui::RichText::new("No cached data.")
                                        .color(AXIS_TEXT)
                                        .small(),
                                );
                                for sym in &self.user_watchlist {
                                    ui.label(
                                        egui::RichText::new(sym)
                                            .color(egui::Color32::from_rgb(100, 100, 110))
                                            .small()
                                            .monospace(),
                                    );
                                }
                            } else {
                                let mut load_key: Option<String> = None;
                                let mut remove_sym: Option<String> = None;
                                let mut open_new_sym: Option<String> = None;
                                let mut move_up_sym: Option<String> = None;
                                let mut move_down_sym: Option<String> = None;
                                let mut move_top_sym: Option<String> = None;
                                let row_h = 18.0_f32;
                                let font = egui::FontId::monospace(10.0);
                                let hdr_font = egui::FontId::monospace(9.0);
                                let avail_w = ui.available_width();

                                // Column layout: Symbol | Last | Chg | Chg% | Ext% | Vol | + | x
                                let col_last = avail_w * 0.26;
                                let col_chg = avail_w * 0.42;
                                let col_pct = avail_w * 0.56;
                                let col_ext = avail_w * 0.70; // Extended hours change%
                                let col_vol = avail_w * 0.82;
                                let col_x = avail_w - 12.0;
                                let col_plus = avail_w - 28.0; // "+" button (open new chart)

                                // Sortable header row
                                let (hdr_rect, hdr_resp) = ui.allocate_exact_size(
                                    egui::vec2(avail_w, row_h),
                                    egui::Sense::click(),
                                );
                                let hp = ui.painter_at(hdr_rect);
                                let hy = hdr_rect.center().y;
                                let hdr_col = egui::Color32::from_rgb(120, 120, 140);
                                let sort_arrow = |col: usize| -> &str {
                                    if self.watchlist_sort.column == col {
                                        if self.watchlist_sort.ascending {
                                            " \u{25B2}"
                                        } else {
                                            " \u{25BC}"
                                        }
                                    } else {
                                        ""
                                    }
                                };
                                hp.text(
                                    egui::pos2(hdr_rect.left() + 2.0, hy),
                                    egui::Align2::LEFT_CENTER,
                                    &format!("Symbol{}", sort_arrow(0)),
                                    hdr_font.clone(),
                                    hdr_col,
                                );
                                hp.text(
                                    egui::pos2(hdr_rect.left() + col_last - 2.0, hy),
                                    egui::Align2::RIGHT_CENTER,
                                    &format!("Last{}", sort_arrow(1)),
                                    hdr_font.clone(),
                                    hdr_col,
                                );
                                hp.text(
                                    egui::pos2(hdr_rect.left() + col_chg - 2.0, hy),
                                    egui::Align2::RIGHT_CENTER,
                                    &format!("Chg{}", sort_arrow(2)),
                                    hdr_font.clone(),
                                    hdr_col,
                                );
                                hp.text(
                                    egui::pos2(hdr_rect.left() + col_pct - 2.0, hy),
                                    egui::Align2::RIGHT_CENTER,
                                    &format!("Chg%{}", sort_arrow(3)),
                                    hdr_font.clone(),
                                    hdr_col,
                                );
                                hp.text(
                                    egui::pos2(hdr_rect.left() + col_ext - 2.0, hy),
                                    egui::Align2::RIGHT_CENTER,
                                    &format!("Ext%{}", sort_arrow(5)),
                                    hdr_font.clone(),
                                    hdr_col,
                                );
                                hp.text(
                                    egui::pos2(hdr_rect.left() + col_vol - 2.0, hy),
                                    egui::Align2::RIGHT_CENTER,
                                    &format!("Vol{}", sort_arrow(4)),
                                    hdr_font.clone(),
                                    hdr_col,
                                );
                                // Click header to sort
                                if hdr_resp.clicked() {
                                    if let Some(pos) = hdr_resp.interact_pointer_pos() {
                                        let rx = pos.x - hdr_rect.left();
                                        let col = if rx < col_last * 0.5 {
                                            0
                                        } else if rx < (col_last + col_chg) * 0.5 {
                                            1
                                        } else if rx < (col_chg + col_pct) * 0.5 {
                                            2
                                        } else if rx < (col_pct + col_ext) * 0.5 {
                                            3
                                        } else if rx < (col_ext + col_vol) * 0.5 {
                                            5
                                        } else {
                                            4
                                        };
                                        self.watchlist_sort.toggle(col);
                                    }
                                }
                                // Separator
                                let sep_y = hdr_rect.bottom();
                                ui.painter().line_segment(
                                    [
                                        egui::pos2(hdr_rect.left(), sep_y),
                                        egui::pos2(hdr_rect.right(), sep_y),
                                    ],
                                    egui::Stroke::new(1.0, egui::Color32::from_rgb(40, 40, 55)),
                                );

                                // Data rows
                                for (idx, wl) in sorted_wl.iter().enumerate() {
                                    let sym_color = WL_COLORS[idx % WL_COLORS.len()];
                                    let chg_color = if wl.change >= 0.0 { UP } else { DOWN };
                                    let is_selected = self
                                        .charts
                                        .get(self.active_tab)
                                        .map(|c| {
                                            c.symbol == wl.cache_key
                                                || c.symbol.contains(&wl.symbol)
                                        })
                                        .unwrap_or(false);

                                    let (row_rect, row_resp) = ui.allocate_exact_size(
                                        egui::vec2(avail_w, row_h),
                                        egui::Sense::click(),
                                    );
                                    let rp = ui.painter_at(row_rect);

                                    // ADR-092: Row background with P&L heatmap intensity
                                    let heat = (wl.change_pct.abs() * 8.0).min(40.0) as u8;
                                    let row_bg = if is_selected {
                                        egui::Color32::from_rgb(15, 25, 45)
                                    } else if heat > 0 {
                                        if wl.change_pct >= 0.0 {
                                            egui::Color32::from_rgb(0, heat / 2, 0)
                                        } else {
                                            egui::Color32::from_rgb(heat / 2, 0, 0)
                                        }
                                    } else if idx % 2 == 1 {
                                        egui::Color32::from_rgb(8, 8, 14)
                                    } else {
                                        egui::Color32::TRANSPARENT
                                    };
                                    rp.rect_filled(row_rect, 0.0, row_bg);

                                    let ry = row_rect.center().y;
                                    let rx = row_rect.left();

                                    // Symbol with colored dot
                                    rp.text(
                                        egui::pos2(rx + 2.0, ry),
                                        egui::Align2::LEFT_CENTER,
                                        "\u{25CF}",
                                        font.clone(),
                                        sym_color,
                                    );
                                    rp.text(
                                        egui::pos2(rx + 14.0, ry),
                                        egui::Align2::LEFT_CENTER,
                                        &wl.symbol,
                                        font.clone(),
                                        egui::Color32::WHITE,
                                    );

                                    // Last / Change / Change% — show extended hours price if available
                                    let (disp_last, disp_chg, disp_pct, disp_color) =
                                        if wl.ext_change_pct.abs() > 0.001 && wl.prev_close > 0.0 {
                                            // Extended hours: derive price from prev_close + ext%
                                            let ext_price =
                                                wl.prev_close * (1.0 + wl.ext_change_pct / 100.0);
                                            let ext_chg = ext_price - wl.prev_close;
                                            let c =
                                                if wl.ext_change_pct >= 0.0 { UP } else { DOWN };
                                            (ext_price, ext_chg, wl.ext_change_pct, c)
                                        } else {
                                            (wl.last, wl.change, wl.change_pct, chg_color)
                                        };
                                    rp.text(
                                        egui::pos2(rx + col_last - 2.0, ry),
                                        egui::Align2::RIGHT_CENTER,
                                        &format_price(disp_last),
                                        font.clone(),
                                        egui::Color32::WHITE,
                                    );

                                    let chg_str = if disp_chg >= 0.0 {
                                        format_price(disp_chg)
                                    } else {
                                        format!("-{}", format_price(disp_chg.abs()))
                                    };
                                    rp.text(
                                        egui::pos2(rx + col_chg - 2.0, ry),
                                        egui::Align2::RIGHT_CENTER,
                                        &chg_str,
                                        font.clone(),
                                        disp_color,
                                    );

                                    rp.text(
                                        egui::pos2(rx + col_pct - 2.0, ry),
                                        egui::Align2::RIGHT_CENTER,
                                        &format!("{:.2}%", disp_pct),
                                        font.clone(),
                                        disp_color,
                                    );

                                    // Extended hours change % (right-aligned, colored, dimmed if zero)
                                    if wl.ext_change_pct.abs() > 0.001 {
                                        let ext_color =
                                            if wl.ext_change_pct >= 0.0 { UP } else { DOWN };
                                        rp.text(
                                            egui::pos2(rx + col_ext - 2.0, ry),
                                            egui::Align2::RIGHT_CENTER,
                                            &format!("{:+.2}%", wl.ext_change_pct),
                                            font.clone(),
                                            ext_color,
                                        );
                                    } else {
                                        rp.text(
                                            egui::pos2(rx + col_ext - 2.0, ry),
                                            egui::Align2::RIGHT_CENTER,
                                            "-",
                                            font.clone(),
                                            egui::Color32::from_rgb(60, 60, 70),
                                        );
                                    }

                                    // Volume (right-aligned, dimmed)
                                    let vol_str = if wl.volume >= 1_000_000.0 {
                                        format!("{:.2}M", wl.volume / 1_000_000.0)
                                    } else if wl.volume >= 1_000.0 {
                                        format!("{:.1}K", wl.volume / 1_000.0)
                                    } else {
                                        format!("{:.0}", wl.volume)
                                    };
                                    rp.text(
                                        egui::pos2(rx + col_vol - 2.0, ry),
                                        egui::Align2::RIGHT_CENTER,
                                        &vol_str,
                                        font.clone(),
                                        AXIS_TEXT,
                                    );

                                    // "+" button (open new chart tab)
                                    rp.text(
                                        egui::pos2(rx + col_plus, ry),
                                        egui::Align2::CENTER_CENTER,
                                        "+",
                                        egui::FontId::monospace(10.0),
                                        egui::Color32::from_rgb(80, 180, 80),
                                    );
                                    // Remove button (x)
                                    rp.text(
                                        egui::pos2(rx + col_x, ry),
                                        egui::Align2::CENTER_CENTER,
                                        "x",
                                        egui::FontId::monospace(9.0),
                                        egui::Color32::from_rgb(100, 50, 50),
                                    );

                                    // Interactions
                                    if row_resp.clicked() {
                                        if let Some(pos) = row_resp.interact_pointer_pos() {
                                            let rel_x = pos.x - rx;
                                            if rel_x >= col_x - 8.0 {
                                                remove_sym = Some(wl.symbol.clone()); // clicked x
                                            } else if rel_x >= col_plus - 8.0
                                                && rel_x < col_plus + 8.0
                                            {
                                                open_new_sym = Some(wl.symbol.clone()); // clicked +
                                            } else {
                                                load_key = Some(wl.cache_key.clone()); // clicked row
                                            }
                                        }
                                    }
                                    row_resp.context_menu(|ui| {
                                        if ui.button(format!("Chart {}", wl.symbol)).clicked() {
                                            load_key = Some(wl.cache_key.clone());
                                            ui.close();
                                        }
                                        if ui.button("View fundamentals").clicked() {
                                            self.show_fundamentals = true;
                                            ui.close();
                                        }
                                        if ui.button("View SEC filings").clicked() {
                                            self.show_sec = true;
                                            self.sec_search_query = wl.symbol.clone();
                                            ui.close();
                                        }
                                        if ui.button("View insider trades").clicked() {
                                            self.show_insider = true;
                                            ui.close();
                                        }
                                        ui.separator();
                                        if ui.button(format!("Move Up  {}", wl.symbol)).clicked() {
                                            move_up_sym = Some(wl.symbol.clone());
                                            ui.close();
                                        }
                                        if ui.button(format!("Move Down  {}", wl.symbol)).clicked()
                                        {
                                            move_down_sym = Some(wl.symbol.clone());
                                            ui.close();
                                        }
                                        if ui
                                            .button(format!("Move to Top  {}", wl.symbol))
                                            .clicked()
                                        {
                                            move_top_sym = Some(wl.symbol.clone());
                                            ui.close();
                                        }
                                        if ui.button(format!("Remove {}", wl.symbol)).clicked() {
                                            remove_sym = Some(wl.symbol.clone());
                                            ui.close();
                                        }
                                        ui.separator();
                                        if ui.button("Command Palette…").clicked() {
                                            self.palette_context = PaletteContext::Watchlist;
                                            self.command_open = true;
                                            self.command_input.clear();
                                            ui.close();
                                        }
                                    });
                                }
                                // Handle reorder — one-step neighbour swap or jump-to-top
                                if let Some(ref sym) = move_up_sym {
                                    if let Some(idx) =
                                        self.user_watchlist.iter().position(|s| s == sym)
                                    {
                                        if idx > 0 {
                                            self.user_watchlist.swap(idx, idx - 1);
                                        }
                                    }
                                }
                                if let Some(ref sym) = move_down_sym {
                                    if let Some(idx) =
                                        self.user_watchlist.iter().position(|s| s == sym)
                                    {
                                        if idx + 1 < self.user_watchlist.len() {
                                            self.user_watchlist.swap(idx, idx + 1);
                                        }
                                    }
                                }
                                if let Some(ref sym) = move_top_sym {
                                    if let Some(idx) =
                                        self.user_watchlist.iter().position(|s| s == sym)
                                    {
                                        if idx > 0 {
                                            let item = self.user_watchlist.remove(idx);
                                            self.user_watchlist.insert(0, item);
                                        }
                                    }
                                }
                                // Handle remove
                                if let Some(ref sym) = remove_sym {
                                    self.user_watchlist.retain(|s| s != sym);
                                    self.watchlist_rows.retain(|r| &r.symbol != sym);
                                }
                                // Handle + button → open new chart tab
                                if let Some(sym) = open_new_sym {
                                    self.deferred_symbol_action = SymbolAction::OpenChart(sym);
                                }
                                // Handle load
                                if let Some(key) = load_key {
                                    // First try loading from cache
                                    let mut loaded = false;
                                    if let Some(ref cache) = self.cache {
                                        if let Some(chart) = self.charts.get_mut(self.active_tab) {
                                            match cache.get_bars_raw(&key) {
                                                Ok(Some(raw)) if !raw.is_empty() => {
                                                    chart.bars = raw
                                                        .into_iter()
                                                        .map(|(ts, o, h, l, c, v)| Bar {
                                                            ts_ms: ts,
                                                            open: o,
                                                            high: h,
                                                            low: l,
                                                            close: c,
                                                            volume: v,
                                                        })
                                                        .collect();
                                                    chart.view_offset =
                                                        chart.bars.len().saturating_sub(1)
                                                            + CHART_RIGHT_MARGIN;
                                                    chart.symbol = bare_symbol_from_key(&key);
                                                    chart.compute_indicators();
                                                    self.log.push_back(LogEntry::info(format!(
                                                        "Loaded {} bars from {}",
                                                        chart.bars.len(),
                                                        key
                                                    )));
                                                    loaded = true;
                                                }
                                                Ok(_) => {
                                                    // Key not found or empty — will try Alpaca below
                                                }
                                                Err(e) => {
                                                    self.log.push_back(LogEntry::err(format!(
                                                        "Load error: {}",
                                                        e
                                                    )));
                                                }
                                            }
                                        }
                                    }
                                    // Fetch from Alpaca if no cached data and broker is connected
                                    if !loaded && self.broker_connected {
                                        let tf = self
                                            .charts
                                            .get(self.active_tab)
                                            .map(|c| c.timeframe.cache_suffix().to_string())
                                            .unwrap_or_else(|| "1Day".to_string());
                                        if self.sync_timeframe_enabled(&tf) {
                                            self.queue_alpaca_fetch(&key, &tf);
                                            self.log.push_back(LogEntry::info(format!(
                                                "Fetching {} from Alpaca...",
                                                key
                                            )));
                                        } else {
                                            self.log.push_back(LogEntry::warn(format!(
                                                "Skipped {} fetch — sync for {} is disabled",
                                                key,
                                                sync_timeframe_short_label(&tf)
                                            )));
                                        }
                                    }
                                }
                            }
                        });
                        self.right_watchlist_open = watchlist_section.fully_open();
                        self.handle_right_panel_section_drag(
                            ui,
                            RightPanelSectionId::Watchlist,
                            &watchlist_section.header_response,
                        );
                                }
                                RightPanelSectionId::Risk => {

                        // ── Risk & Account Section ───────────────────────────
                        let risk_section = egui::CollapsingHeader::new(
                            egui::RichText::new("☰ Risk & Account").strong().small(),
                        )
                        .default_open(self.right_risk_open)
                        .show(ui, |ui| {
                            ui.add_space(4.0);
                            // Live broker account data for selected target(s)
                            let account_snaps = self.selected_trade_account_snapshots();
                            for (idx, snap) in account_snaps.iter().enumerate() {
                                ui.label(
                                    egui::RichText::new(snap.broker)
                                        .color(AXIS_TEXT)
                                        .small()
                                        .strong(),
                                );
                                egui::Grid::new(format!("live_risk_grid_{idx}"))
                                    .striped(true)
                                    .num_columns(2)
                                    .show(ui, |ui| {
                                        ui.label(
                                            egui::RichText::new("Equity").color(AXIS_TEXT).small(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!("${:.2}", snap.equity))
                                                .small(),
                                        );
                                        ui.end_row();
                                        ui.label(
                                            egui::RichText::new("Balance").color(AXIS_TEXT).small(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!("${:.2}", snap.balance))
                                                .small(),
                                        );
                                        ui.end_row();
                                        ui.label(
                                            egui::RichText::new("Buying Power")
                                                .color(AXIS_TEXT)
                                                .small(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "${:.2}",
                                                snap.buying_power
                                            ))
                                            .small(),
                                        );
                                        ui.end_row();
                                        ui.label(
                                            egui::RichText::new("Margin Used")
                                                .color(AXIS_TEXT)
                                                .small(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "${:.2}",
                                                snap.margin_used
                                            ))
                                            .small(),
                                        );
                                        ui.end_row();
                                    });
                                if snap.broker == "Kraken" {
                                    ui.label(
                                        egui::RichText::new(
                                            "Kraken sizing uses USD/stable cash balance for spot orders.",
                                        )
                                        .color(AXIS_TEXT)
                                        .small(),
                                    );
                                }
                                ui.add_space(5.0);
                            }
                            // DARWIN portfolio data — from bg cache
                            if let Some(ref portfolio) = self.bg.portfolio {
                                if !portfolio.accounts.is_empty() {
                                    egui::Grid::new("risk_grid")
                                        .striped(true)
                                        .num_columns(2)
                                        .show(ui, |ui| {
                                            ui.label(
                                                egui::RichText::new("Accounts")
                                                    .color(AXIS_TEXT)
                                                    .small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{}",
                                                    portfolio.accounts.len()
                                                ))
                                                .small(),
                                            );
                                            ui.end_row();
                                            ui.label(
                                                egui::RichText::new("Equity")
                                                    .color(AXIS_TEXT)
                                                    .small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "${:.0}",
                                                    portfolio.total_final_balance
                                                ))
                                                .small(),
                                            );
                                            ui.end_row();
                                            let pnl_c = if portfolio.total_net_pnl >= 0.0 {
                                                UP
                                            } else {
                                                DOWN
                                            };
                                            ui.label(
                                                egui::RichText::new("Net P&L")
                                                    .color(AXIS_TEXT)
                                                    .small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "${:.0}",
                                                    portfolio.total_net_pnl
                                                ))
                                                .color(pnl_c)
                                                .small(),
                                            );
                                            ui.end_row();
                                            ui.label(
                                                egui::RichText::new("Max DD")
                                                    .color(AXIS_TEXT)
                                                    .small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{:.1}%",
                                                    portfolio.combined_max_drawdown_pct
                                                ))
                                                .small(),
                                            );
                                            ui.end_row();
                                            ui.label(
                                                egui::RichText::new("Deals")
                                                    .color(AXIS_TEXT)
                                                    .small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{}",
                                                    portfolio.total_deals
                                                ))
                                                .small(),
                                            );
                                            ui.end_row();
                                        });
                                    // VaR — from bg cache
                                    if let Some(ref vs) = self.bg.var_stats {
                                        ui.add_space(4.0);
                                        egui::Grid::new("risk_var")
                                            .striped(true)
                                            .num_columns(2)
                                            .show(ui, |ui| {
                                                ui.label(
                                                    egui::RichText::new("VaR 95%")
                                                        .color(AXIS_TEXT)
                                                        .small(),
                                                );
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "${:.0}",
                                                        vs.var_95
                                                    ))
                                                    .small(),
                                                );
                                                ui.end_row();
                                                ui.label(
                                                    egui::RichText::new("Sharpe")
                                                        .color(AXIS_TEXT)
                                                        .small(),
                                                );
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "{:.3}",
                                                        vs.sharpe
                                                    ))
                                                    .small(),
                                                );
                                                ui.end_row();
                                            });
                                    }
                                } else {
                                    ui.label(
                                        egui::RichText::new("Import DARWIN data")
                                            .color(AXIS_TEXT)
                                            .small(),
                                    );
                                }
                            }
                            ui.add_space(6.0);
                            ui.separator();
                    });
                        self.right_risk_open = risk_section.fully_open();
                        self.handle_right_panel_section_drag(
                            ui,
                            RightPanelSectionId::Risk,
                            &risk_section.header_response,
                        );
                                }
                                RightPanelSectionId::News => {

                        // ── News Section (Finnhub) ─────────────────────────
                        {
                            let news_count = self.news_articles.len();
                            let news_section = egui::CollapsingHeader::new(
                                egui::RichText::new(format!("☰ News ({})", news_count))
                                    .strong()
                                    .small(),
                            )
                            .id_salt("news_section")
                            .default_open(self.right_news_open || news_count > 0)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    if !self.finnhub_key.is_empty() {
                                        let button_label = if news_count == 0 {
                                            "Fetch News"
                                        } else {
                                            "Fetch News"
                                        };
                                        if ui
                                            .add_enabled(
                                                !self.news_loading,
                                                egui::Button::new(
                                                    egui::RichText::new(button_label).small(),
                                                )
                                                .fill(BTN_BLUE)
                                                .min_size(egui::vec2(80.0, 18.0)),
                                            )
                                            .clicked()
                                        {
                                            let sym = self
                                                .charts
                                                .get(self.active_tab)
                                                .map(|c| {
                                                    c.symbol
                                                        .split(':')
                                                        .rev()
                                                        .nth(1)
                                                        .or_else(|| c.symbol.split(':').last())
                                                        .unwrap_or("AAPL")
                                                        .to_string()
                                                })
                                                .unwrap_or_else(|| "AAPL".to_string());
                                            self.news_loading = true;
                                            let _ = self.broker_tx.send(BrokerCmd::FinnhubNews {
                                                symbol: sym.clone(),
                                                api_key: self.finnhub_key.clone(),
                                            });
                                            self.log.push_back(LogEntry::info(format!(
                                                "Finnhub: fetching news for {sym}"
                                            )));
                                        }
                                        if self.news_loading {
                                            ui.spinner();
                                        }
                                    } else {
                                        ui.label(
                                            egui::RichText::new("Set Finnhub key in Settings")
                                                .color(AXIS_TEXT)
                                                .small(),
                                        );
                                    }
                                });
                                if news_count == 0 {
                                    ui.label(
                                        egui::RichText::new("No news loaded for the active symbol.")
                                            .color(AXIS_TEXT)
                                            .small(),
                                    );
                                } else {
                                    egui::ScrollArea::vertical()
                                        .auto_shrink(false)
                                        .max_height(180.0)
                                        .id_salt("news_scroll_r")
                                        .show(ui, |ui| {
                                            for (headline, source, dt) in &self.news_articles {
                                                ui.horizontal(|ui| {
                                                    ui.spacing_mut().item_spacing.x = 4.0;
                                                    ui.label(
                                                        egui::RichText::new(dt)
                                                            .color(egui::Color32::from_rgb(
                                                                80, 80, 95,
                                                            ))
                                                            .small(),
                                                    );
                                                    ui.label(
                                                        egui::RichText::new(source)
                                                            .color(egui::Color32::from_rgb(
                                                                100, 100, 120,
                                                            ))
                                                            .small(),
                                                    );
                                                });
                                                let hl = headline.to_lowercase();
                                                let bullish = [
                                                    "surge",
                                                    "rally",
                                                    "beat",
                                                    "up ",
                                                    "soar",
                                                    "gain",
                                                    "rise",
                                                    "jump",
                                                    "bull",
                                                    "record high",
                                                ];
                                                let bearish = [
                                                    "crash", "fall", "miss", "down ", "plunge",
                                                    "drop", "sink", "bear", "sell-off", "selloff",
                                                    "decline",
                                                ];
                                                let is_bull =
                                                    bullish.iter().any(|w| hl.contains(w));
                                                let is_bear =
                                                    bearish.iter().any(|w| hl.contains(w));
                                                let hl_color = if is_bull {
                                                    UP
                                                } else if is_bear {
                                                    DOWN
                                                } else {
                                                    egui::Color32::from_rgb(190, 190, 200)
                                                };
                                                ui.label(
                                                    egui::RichText::new(headline)
                                                        .color(hl_color)
                                                        .small(),
                                                );
                                                ui.add_space(2.0);
                                            }
                                    });
                                }
                            });
                            self.right_news_open = news_section.fully_open();
                            self.handle_right_panel_section_drag(
                                ui,
                                RightPanelSectionId::News,
                                &news_section.header_response,
                            );
                        }
                                }
                                RightPanelSectionId::MtfGrid => {

                        // ── MTF Grid ────────────────────────────────────────
                        let mtf_grid_section = egui::CollapsingHeader::new(
                            egui::RichText::new("☰ MTF Grid")
                                .color(AXIS_TEXT)
                                .small()
                                .strong(),
                        )
                        .id_salt("mtf_grid_section")
                        .default_open(self.right_mtf_grid_open)
                        .show(ui, |ui| {
                        let tf_labels = ["M1", "M5", "M15", "M30", "H1", "H4", "D1", "W1"];
                        let ma_labels = ["SMA200", "KAMA", "Fisher"];
                        egui::Grid::new("mtf_ma_grid")
                            .spacing(egui::vec2(4.0, 2.0))
                            .show(ui, |ui| {
                                // Header row
                                ui.label(egui::RichText::new("").small());
                                for tf in &tf_labels {
                                    ui.label(egui::RichText::new(*tf).color(AXIS_TEXT).small());
                                }
                                ui.end_row();
                                // Data rows from mtf_grid_status
                                for ma in &ma_labels {
                                    ui.label(egui::RichText::new(*ma).color(AXIS_TEXT).small());
                                    for tf in &tf_labels {
                                        // Find status for this TF
                                        let status =
                                            self.mtf_grid_status.iter().find(|s| s.0 == *tf);
                                        let dot_color =
                                            if let Some(&(_, close, sma, kama, fisher, fsig)) =
                                                status
                                            {
                                                let bullish = match *ma {
                                                    "SMA200" => match (close, sma) {
                                                        (Some(c), Some(s)) => Some(c > s),
                                                        _ => None,
                                                    },
                                                    "KAMA" => match (close, kama) {
                                                        (Some(c), Some(k)) => Some(c > k),
                                                        _ => None,
                                                    },
                                                    "Fisher" => match (fisher, fsig) {
                                                        (Some(f), Some(s)) => Some(f > s),
                                                        _ => None,
                                                    },
                                                    _ => None,
                                                };
                                                match bullish {
                                                    Some(true) => UP,
                                                    Some(false) => DOWN,
                                                    None => AXIS_TEXT,
                                                }
                                            } else {
                                                egui::Color32::from_rgb(50, 50, 60)
                                            };
                                        ui.label(
                                            egui::RichText::new("\u{25CF}")
                                                .color(dot_color)
                                                .small(),
                                        );
                                    }
                                    ui.end_row();
                                }
                            });
                        });
                        self.right_mtf_grid_open = mtf_grid_section.fully_open();
                        self.handle_right_panel_section_drag(
                            ui,
                            RightPanelSectionId::MtfGrid,
                            &mtf_grid_section.header_response,
                        );
                                }
                            }
                        }
                        if self.dragging_right_panel_section.is_some()
                            && !ui.input(|i| i.pointer.primary_down())
                        {
                            self.dragging_right_panel_section = None;
                        }
                    });
            });

        // ── floating windows ─────────────────────────────────────────────────
        // Always call draw_floating_windows so close buttons work.
        // Performance: all DARWIN data reads from self.bg (background-computed).
        self.draw_floating_windows(ctx);

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
                        chart.price_zoom = (chart.price_zoom * (1.0 + pct as f64)).clamp(0.1, 20.0);
                    }
                } else if on_chart_body {
                    let ctrl_held = ctx.input(|i| i.modifiers.ctrl);
                    if ctrl_held {
                        // Ctrl+scroll on chart → vertical zoom (progressive)
                        if let Some(chart) = self.charts.get_mut(self.active_tab) {
                            let pct = (scroll_delta * 0.002).clamp(-0.08, 0.08);
                            chart.price_zoom = (chart.price_zoom * (1.0 + pct as f64)).clamp(0.1, 20.0);
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
                    // Price-axis scaling is chart-only. Floating windows and side-panel
                    // widgets can overlap this x-range while resizing or dragging.
                    if price_axis_rect.contains(press_pos) && !pointer_over_window {
                        // Start price-axis scaling drag (TradingView style)
                        chart.is_scaling_price = true;
                        chart.is_dragging = false;
                    self.user_interacting = false;
                        chart.is_drawing_drag = false;
                        chart.scale_start_zoom = chart.price_zoom;
                        chart.scale_start_y = press_pos.y;
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
                            // Start normal chart pan drag — only if inside the chart area
                            chart.is_dragging = true;
                            self.user_interacting = true;
                            chart.is_drawing_drag = false;
                            chart.is_scaling_price = false;
                            chart.drag_start = pointer.press_origin();
                            chart.drag_start_offset = chart.view_offset;
                            chart.drag_start_ppan = chart.price_pan;
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

                // Price axis drag → vertical zoom (like TradingView)
                if chart.is_scaling_price && drag_delta.y.abs() > 0.0 {
                    // Drag up = zoom in (expand), drag down = zoom out (compress)
                    // Drag up = expand (zoom in), drag down = squish (zoom out)
                    // TradingView-style progressive scaling
                    let sensitivity = 0.003;
                    let zoom_delta = -drag_delta.y as f64 * sensitivity;
                    chart.price_zoom = (chart.price_zoom * (1.0 + zoom_delta)).clamp(0.1, 20.0);
                }

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

                // Normal chart body drag → pan
                if chart.is_dragging && (drag_delta.x.abs() > 0.0 || drag_delta.y.abs() > 0.0) {
                    Self::handle_pan_h(chart, drag_delta.x, available.width());
                    if drag_delta.y.abs() > 0.5 {
                        let range = {
                            let bars = &chart.bars;
                            if bars.is_empty() { 1.0 }
                            else {
                                let (si, ei) = chart.visible_range();
                                let slice = &bars[si..ei];
                                let hi = slice.iter().map(|b| b.high).fold(0.0_f64, f64::max);
                                let lo = slice.iter().map(|b| b.low).fold(f64::MAX, f64::min);
                                hi - lo
                            }
                        };
                        chart.price_pan += drag_delta.y as f64 * range / available.height() as f64;
                    }
                }
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
            let sl_price = self.sl_price;
            let tp_price = self.tp_price;

            if self.mtf_enabled {
                // Filter to visible charts only
                while self.mtf_visible.len() < self.charts.len() { self.mtf_visible.push(true); }
                let visible_indices: Vec<usize> = (0..self.charts.len())
                    .filter(|&i| self.mtf_visible.get(i).copied().unwrap_or(true))
                    .take(16)
                    .collect();
                let total = visible_indices.len().max(1);
                let cols   = self.mtf_cols.max(1).min(total);
                let rows   = (total + cols - 1) / cols;
                let cell_w = available.width()  / cols  as f32;
                let cell_h = available.height() / rows  as f32;

                // Detect click on grid cell to focus it
                let click_pos = if ctx.input(|i| i.pointer.primary_clicked()) {
                    ctx.input(|i| i.pointer.interact_pos())
                } else { None };

                // Lazy-load bars for visible MTF grid charts
                if let Some(ref cache) = self.cache {
                    for &vi in &visible_indices {
                    let chart = &mut self.charts[vi];
                        if chart.bars.is_empty() {
                            let loaded = { let mut gpu = self.gpu_indicators.take(); let r = chart.try_load(cache, &mut self.log, gpu.as_mut()); self.gpu_indicators = gpu; r };
                            if loaded {
                                break; // loaded one, continue next frame
                            } else {
                                break; // lock contended, try next frame
                            }
                        }
                    }
                }

                for (grid_pos, &vi) in visible_indices.iter().enumerate() {
                // Rebuild trade overlay every 120 frames (~30s) or on first load
                let fc = self.frame_count;
                if self.charts[vi].cached_trade_overlay_frame == 0 || fc.wrapping_sub(self.charts[vi].cached_trade_overlay_frame) > 120 {
                    self.charts[vi].cached_trade_overlay = self.build_trade_overlay(&self.charts[vi]);
                    self.charts[vi].cached_trade_overlay_frame = fc;
                }
                let trade_ov = self.charts[vi].cached_trade_overlay.clone();
                let chart = &mut self.charts[vi];
                let idx = grid_pos;
                    let col = idx % cols;
                    let row = idx / cols;
                    let cell_rect = egui::Rect::from_min_size(
                        egui::pos2(
                            available.left() + col as f32 * cell_w,
                            available.top()  + row as f32 * cell_h,
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

                    // Zoom when pointer is in this cell (no focus-click required)
                    if ptr_in_cell {
                        let scroll = ctx.input(|i| i.smooth_scroll_delta.y);
                        if scroll != 0.0 {
                            Self::handle_zoom(chart, scroll);
                        }
                        // Drag pan for this cell
                        let drag = ctx.input(|i| i.pointer.delta());
                        if ctx.input(|i| i.pointer.primary_down()) && drag.x.abs() > 0.5 {
                            Self::handle_pan_h(chart, drag.x, cell_rect.width());
                        }
                    }

                    let painter = ui.painter_at(cell_rect);
                    draw_chart(&painter, chart, cell_rect, crosshair, &flags, show_rsi, show_fisher, show_macd, show_volume_pane, show_stochastic, show_adx, show_cci, show_williams_r, show_obv, show_momentum, show_cmo, show_qstick, show_disparity, show_bop, show_stddev, show_mfi, show_trix, show_ppo, show_ultosc, show_stochrsi, show_var_oscillator, show_better_volume, show_ehlers_ebsw, show_ehlers_cyber, show_ehlers_cg, show_ehlers_roof, self.show_squeeze, sl_price, tp_price, &trade_ov, &self.alerts, &self.draw_mode);

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
            } else {
                let (rect, resp) = ui.allocate_exact_size(available.size(), egui::Sense::click_and_drag());

                // Rebuild trade overlay every 120 frames (~30s) or on first load
                let fc = self.frame_count;
                if let Some(c) = self.charts.get(self.active_tab) {
                    if c.cached_trade_overlay_frame == 0 || fc.wrapping_sub(c.cached_trade_overlay_frame) > 120 {
                        let ov = self.build_trade_overlay(c);
                        self.charts[self.active_tab].cached_trade_overlay = ov;
                        self.charts[self.active_tab].cached_trade_overlay_frame = fc;
                    }
                }
                let trade_ov = self.charts.get(self.active_tab).map(|c| c.cached_trade_overlay.clone()).unwrap_or_default();

                // Replay mode: clamp view to only show replay_bar_idx bars
                if self.replay_active {
                    if let Some(chart) = self.charts.get_mut(self.active_tab) {
                        let count = self.replay_bar_idx.max(1).min(chart.bars.len());
                        chart.view_offset = count.saturating_sub(1);
                        chart.visible_bars = chart.visible_bars.min(count);
                    }
                }

                if let Some(chart) = self.charts.get_mut(self.active_tab) {
                    let painter = ui.painter_at(rect);
                    draw_chart(&painter, chart, rect, crosshair, &flags, show_rsi, show_fisher, show_macd, show_volume_pane, show_stochastic, show_adx, show_cci, show_williams_r, show_obv, show_momentum, show_cmo, show_qstick, show_disparity, show_bop, show_stddev, show_mfi, show_trix, show_ppo, show_ultosc, show_stochrsi, show_var_oscillator, show_better_volume, show_ehlers_ebsw, show_ehlers_cyber, show_ehlers_cg, show_ehlers_roof, self.show_squeeze, sl_price, tp_price, &trade_ov, &self.alerts, &self.draw_mode);

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
                PaletteContext::Position => Some(&[
                    "CLOSE_ALL",
                    "CLOSE_PARTIAL",
                    "SET_SL",
                    "SET_TP",
                    "OPEN_MG",
                    "EQUITY",
                    "TRADESTATS",
                    "PROFILE",
                    "RISK_CALC",
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
                PaletteContext::Darwin => Some(&[
                    "DARWIN",
                    "PORTFOLIO",
                    "DRAWDOWN",
                    "REBALANCE",
                    "DARWIN_TRADES",
                    "DARWINVAR",
                    "DARWINIA_SCAN",
                    "CORRELATION",
                    "DWXSYNC",
                    "DWXSTATUS",
                    "EXPORT_DARWIN",
                    "DELETE_DARWIN",
                    "SWAPHARVEST",
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
        if self.frame_count % 240 == 0 && self.frame_count > 0 {
            // Collect all state needed for save (cheap copies of strings + JSON)
            let session_json = self.build_session_json();
            self.sync_preferences_save();
            let creds: Vec<(String, String)> = [
                (keyring::keys::ALPACA_API_KEY, &self.broker_api_key),
                (keyring::keys::ALPACA_SECRET, &self.broker_secret),
                (keyring::keys::FINNHUB_KEY, &self.finnhub_key),
                (keyring::keys::FRED_KEY, &self.fred_key),
                (keyring::keys::TT_USERNAME, &self.tt_username),
                (keyring::keys::TT_PASSWORD, &self.tt_password),
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

        // Update Prometheus metrics every ~5 seconds (20 frames at 250ms idle repaint)
        if self.frame_count % 20 == 3 {
            if let Some(ref reg) = self.metrics_registry {
                let mut snap = crate::metrics::MetricsSnapshot::default();

                // Uptime
                snap.uptime_seconds = self.metrics_start.elapsed().as_secs_f64();

                // Broker connection
                snap.broker_connected.push((
                    "alpaca".to_string(),
                    if self.broker_connected { 1.0 } else { 0.0 },
                ));
                snap.broker_connected
                    .push(("tt".to_string(), if self.tt_connected { 1.0 } else { 0.0 }));

                // Account equity from live account
                if let Some(ref acct) = self.live_account {
                    snap.account_equity
                        .push(("alpaca".to_string(), acct.equity));
                }

                // Open positions count
                snap.positions_open
                    .push(("alpaca".to_string(), self.live_positions.len() as f64));

                // DARWIN portfolio open positions count
                if !self.bg.open_positions.is_empty() {
                    snap.positions_open
                        .push(("darwin".to_string(), self.bg.open_positions.len() as f64));
                }

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
        if self.frame_count % 20 == 3 && self.lan_sync_mode == "server" {
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
                                                    let _ = self.broker_tx.send(
                                                        BrokerCmd::KrakenBackfill {
                                                            symbol: symbol.to_string(),
                                                            timeframes: vec![tf_norm.to_string()],
                                                            db_path: db_path.clone(),
                                                        },
                                                    );
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
                                                use_mt5: self.fund_source_mt5,
                                                use_alpaca: self.fund_source_alpaca,
                                                use_tastytrade: self.fund_source_tastytrade,
                                                use_kraken: self.fund_source_kraken,
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
                                        let db_path = cache_db_path();
                                        let _ =
                                            self.broker_tx.send(BrokerCmd::SecScrape { db_path });
                                        self.log.push_back(LogEntry::info(
                                            "LAN remote: SEC scrape started",
                                        ));
                                    }
                                    "MT5_SYNC" => {
                                        let paths: Vec<String> = self
                                            .mt5_db_paths
                                            .iter()
                                            .filter(|p| {
                                                !p.is_empty()
                                                    && std::path::Path::new(p.as_str()).exists()
                                            })
                                            .cloned()
                                            .collect();
                                        if !paths.is_empty() {
                                            let _ = self.broker_tx.send(BrokerCmd::Mt5Sync {
                                                sources: paths,
                                                enabled_timeframes: self
                                                    .enabled_standard_sync_timeframes(),
                                            });
                                            self.log.push_back(LogEntry::info(
                                                "LAN remote: MT5 sync started",
                                            ));
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
                                                },
                                            );
                                            self.log.push_back(LogEntry::info(format!(
                                                "LAN remote: Kraken Futures backfill {} started",
                                                sym
                                            )));
                                        }
                                    }
                                    "DARWIN_IMPORT" => {
                                        if !self.darwin_xlsx_dir.is_empty() {
                                            let db_path = cache_db_path();
                                            let _ =
                                                self.broker_tx.send(BrokerCmd::DarwinImportAll {
                                                    dir: std::path::PathBuf::from(
                                                        &self.darwin_xlsx_dir,
                                                    ),
                                                    db_path,
                                                });
                                            self.log.push_back(LogEntry::info(
                                                "LAN remote: DARWIN XLSX import started",
                                            ));
                                        }
                                    }
                                    "EVSCRAPE" | "EVSCRAPE_FORCE" => {
                                        let force = cmd == "EVSCRAPE_FORCE";
                                        let db_path = cache_db_path();
                                        let _ =
                                            self.broker_tx.send(BrokerCmd::FundamentalsScrape {
                                                db_path,
                                                use_mt5: self.fund_source_mt5,
                                                use_alpaca: self.fund_source_alpaca,
                                                use_tastytrade: self.fund_source_tastytrade,
                                                use_kraken: self.fund_source_kraken,
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

        // Poll tastytrade positions every ~60 seconds so terminal-managed exit changes
        // and broker fills eventually converge back into the unified position view.
        if self.frame_count % 240 == 30
            && self.tt_connected
            && !self.lan_client_enabled
            && self.cache_loaded
        {
            let _ = self.broker_tx.send(BrokerCmd::TastytradePositions);
            let _ = self.broker_tx.send(BrokerCmd::TastytradeGetBalances);
        }
        if self.frame_count % 240 == 40
            && self.kraken_connected
            && !self.lan_client_enabled
            && self.cache_loaded
        {
            let _ = self.broker_tx.send(BrokerCmd::KrakenGetPositions);
            let _ = self.broker_tx.send(BrokerCmd::KrakenGetBalance);
        }

        // Poll watchlist quotes every ~15 seconds at 60fps (900 frames). Disabled for LAN client.
        // Includes Alpaca snapshot + Yahoo extended hours enrichment per cycle.
        if self.frame_count % 900 == 5
            && !self.user_watchlist.is_empty()
            && self.broker_connected
            && !self.lan_client_enabled
            && self.cache_loaded
            && !self.alpaca_bar_backlog_active()
        {
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
            if self.frame_count % 240 == 150 && self.frame_count > 0 {
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
                            });
                        }
                    }
                }
            }

            // Auto MT5 bar sync every ~60s on weekdays (smart: skips unchanged keys).
            // Filter for file existence here — a missing path would trigger the
            // Mt5Sync handler's "locked by BarCacheWriter" fallback message even
            // though the DB isn't locked, it simply isn't there. Matches the
            // bid/ask refresh below which already guards with .exists().
            if self.frame_count % 240 == 100 && self.frame_count > 0 {
                let now_utc = chrono::Utc::now();
                let eastern = now_utc.with_timezone(
                    &chrono::FixedOffset::west_opt(5 * 3600)
                        .unwrap_or(chrono::FixedOffset::east(0)),
                );
                use chrono::Datelike;
                let is_weekday = !matches!(
                    eastern.weekday(),
                    chrono::Weekday::Sat | chrono::Weekday::Sun
                );
                if is_weekday {
                    let paths: Vec<String> = self
                        .mt5_db_paths
                        .iter()
                        .filter(|p| !p.is_empty() && std::path::Path::new(p.as_str()).exists())
                        .cloned()
                        .collect();
                    if !paths.is_empty() {
                        let _ = self.broker_tx.send(BrokerCmd::Mt5Sync {
                            sources: paths,
                            enabled_timeframes: self.enabled_standard_sync_timeframes(),
                        });
                    }
                }
            }

            // Alpaca equity rotation — iterate Alpaca's full us_equity tradable
            // universe (~11000 symbols) minus anything MT5 (Darwinex) already
            // covers, plus a chart/watchlist floor that holds even before the
            // asset-list fetch completes. MT5 is authoritative for its own
            // symbols; Alpaca fills the gap (US stocks + ETFs Darwinex doesn't
            // list). Runs 7 days/week — stocks don't trade on weekends but the
            // historical backfill can still progress.
            if self.frame_count % 240 == 200 && self.frame_count > 0 {
                self.maybe_request_alpaca_asset_universe();
                let equity_syms = self.alpaca_equity_rotation_symbols();
                self.schedule_alpaca_pairs(&equity_syms);
            }

            // MT5 live bid/ask refresh every ~30s — fast read of bid_ask table only (no bar sync).
            // Updates forming bars on all charts with latest MT5 mid prices.
            // Reads from /dev/shm ramdisk — sub-millisecond, safe on UI thread.
            if self.frame_count % 120 == 60 && self.frame_count > 0 {
                let paths: Vec<String> = self
                    .mt5_db_paths
                    .iter()
                    .filter(|p| !p.is_empty() && std::path::Path::new(p.as_str()).exists())
                    .cloned()
                    .collect();
                if let Some(last_src) = paths.last() {
                    let src_path = std::path::PathBuf::from(last_src);
                    if let Ok(src) =
                        typhoon_engine::core::cache::SqliteCache::open_readonly(&src_path)
                    {
                        if let Ok(quotes) = src.read_bid_ask() {
                            // PERF: precompute `chart_bare` once per chart, rsplit-based.
                            let chart_bares: Vec<String> = self
                                .charts
                                .iter()
                                .map(|chart| {
                                    let mut s = chart.symbol.replace('/', "");
                                    s.make_ascii_uppercase();
                                    let bare_opt = {
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
                                            Some(it.next().unwrap_or(last).to_string())
                                        } else {
                                            None
                                        }
                                    };
                                    bare_opt.unwrap_or(s)
                                })
                                .collect();
                            for (sym, bid, ask, _spread) in &quotes {
                                let mid = (bid + ask) / 2.0;
                                if mid <= 0.0 {
                                    continue;
                                }
                                let sym_upper = sym.to_uppercase();
                                for (ci, chart) in self.charts.iter_mut().enumerate() {
                                    if chart_bares[ci] == sym_upper {
                                        if let Some(bar) = chart.bars.last_mut() {
                                            bar.close = mid;
                                            if mid > bar.high {
                                                bar.high = mid;
                                            }
                                            if mid < bar.low {
                                                bar.low = mid;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Periodic MT5 bar sync — every ~30 seconds when auto-sync is enabled and
            // at least one MT5 path is configured. Matches BarCacheWriter's 30s write
            // cadence (UpdateIntervalSec=30) so the terminal never lags behind the EA.
            // Idle repaint ≈ 250 ms → 120 frames ≈ 30 s.
            if self.mt5_auto_sync && self.frame_count % 120 == 60 && self.frame_count > 0 {
                let paths: Vec<String> = self
                    .mt5_db_paths
                    .iter()
                    .filter(|p| !p.is_empty() && std::path::Path::new(p.as_str()).exists())
                    .cloned()
                    .collect();
                if !paths.is_empty() {
                    let _ = self.broker_tx.send(BrokerCmd::Mt5Sync {
                        sources: paths,
                        enabled_timeframes: self.enabled_standard_sync_timeframes(),
                    });
                    // Silent — don't spam the log with auto-sync notifications
                }
            }

            // Periodic MT5 self-heal — runs every ~30s regardless of
            // mt5_auto_sync so gap detection + demand.txt refresh always
            // happens even when users don't run full cache sync. Keeps
            // every cached (sym, tf) fresh within 5×TF via pass-2
            // self-healing. Offset by 30 frames (~7.5s) from the full
            // sync trigger so the two passes don't collide on the same
            // frame.
            if self.frame_count % 120 == 90 && self.frame_count > 0 {
                let has_mt5 = self
                    .mt5_db_paths
                    .iter()
                    .any(|p| !p.is_empty() && std::path::Path::new(p.as_str()).exists());
                if has_mt5 {
                    self.detect_mt5_gaps();
                    self.write_mt5_demand_txt();
                }
            }

            // ~1Hz demand.txt refresh for fresh-tab-open latency. detect_mt5_gaps
            // runs on the 30s cadence above; this flush propagates a newly-
            // opened chart to the EA's demand list within ~1 second instead of
            // waiting up to 30s for the next heartbeat. 4 frames ≈ 1 s (idle
            // repaint ~250 ms). Content-hash dedup makes the no-op path free.
            //
            // Must pass `include_gap_requests=true` — a `false` flush writes
            // demand.txt with the same pair set but a zeroed gap section, which
            // silently clobbers the gap-fill rows the previous heartbeat
            // staged. BCW reloads demand.txt only every 2 cycles (~60s), so at
            // `false` the gap rows were overwritten within 1 s of being
            // written and the EA never saw them — gap-fill was effectively a
            // no-op. `mt5_gap_requests` is already cleared + rebuilt by
            // `detect_mt5_gaps`; re-emitting the existing vector is free.
            if self.frame_count % 4 == 0 && self.frame_count > 0 {
                let has_mt5 = self
                    .mt5_db_paths
                    .iter()
                    .any(|p| !p.is_empty() && std::path::Path::new(p.as_str()).exists());
                if has_mt5 {
                    self.flush_mt5_demand_txt(true);
                }
            }
        }

        // Repaint strategy:
        // - egui auto-repaints on ANY user interaction (mouse move, click, scroll, key)
        // - We set a slow idle repaint for background updates (live data, time)
        // - Charts stay responsive because mouse events trigger instant repaints
        // - Floating windows with DB queries only update on idle repaints
        // Repaint scheduling: fast during startup, then idle.
        // egui internally triggers repaints on hover/click/animation — we only set
        // the MINIMUM idle repaint interval (for chart tick updates, clock, etc.).
        let startup_loading = self.bg.portfolio.is_none() && self.cache.is_some();
        let idle_ms = if startup_loading || !self.cache_loaded {
            100
        } else {
            250
        };
        self.maybe_incremental_session_save(ctx);
        ctx.request_repaint_after(std::time::Duration::from_millis(idle_ms));

        // UX3: Apply any deferred symbol context-menu action from right-panel renders
        if !matches!(self.deferred_symbol_action, SymbolAction::None) {
            let action = std::mem::replace(&mut self.deferred_symbol_action, SymbolAction::None);
            self.apply_symbol_action(action);
        }
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────
