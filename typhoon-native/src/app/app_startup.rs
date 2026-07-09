use super::*;
use crate::app::trade_ops::obsolete_nonspot_low_timeframe;
use super::state::PreloadedCacheMarks;
use crate::app::app_runtime_support::{
    should_auto_start_background_scope_scrape, should_auto_start_kraken_fundamentals_scrape,
};

pub(super) fn install_image_loaders(cc: &eframe::CreationContext<'_>) {
    // Install the egui image loaders (PNG/JPEG/WEBP + HTTP/file URI
    // dispatch) so news article hero images and inline markdown
    // images decode from URLs without manual texture management.
    // Idempotent in practice — egui_extras dedups on tag.
    egui_extras::install_image_loaders(&cc.egui_ctx);
}

pub(super) fn spawn_ui_repaint_wake_pump(
    ctx: &egui::Context,
) -> Arc<std::sync::atomic::AtomicBool> {
    let alive = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let alive_thread = alive.clone();
    let ctx = ctx.clone();
    let wake_ms = std::env::var("TYPHOON_UI_WAKE_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(16);
    if wake_ms == 0 {
        return alive;
    }
    let _ = std::thread::Builder::new()
        .name("typhoon-ui-repaint-wake".to_string())
        .spawn(move || {
            let interval = std::time::Duration::from_millis(wake_ms.max(1));
            while alive_thread.load(std::sync::atomic::Ordering::Relaxed) {
                std::thread::sleep(interval);
                ctx.request_repaint();
            }
        });
    alive
}

pub(super) fn init_kraken_iapi_limiter() {
    // Initialize the process-wide iapi limiter with persistence pointing at
    // the config dir. This must happen before any KrakenBroker iapi call;
    // we run it here so a partial cooldown from a previous session is
    // restored before the broker thread starts dispatching.
    // Best-effort: a duplicate init returns Err which we silently ignore.
    let mut backoff_path = dirs_home();
    backoff_path.push("kraken_iapi_backoff.json");
    if let Some(parent) = backoff_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut config = typhoon_engine::broker::kraken::IapiLimiterConfig {
        persistence_path: Some(backoff_path),
        ..Default::default()
    };
    if let Ok(raw_max_rate) = std::env::var("TYPHOON_KRAKEN_IAPI_AIMD_MAX_RATE") {
        match raw_max_rate.trim().parse::<f64>() {
            Ok(rate) if rate.is_finite() && rate >= config.aimd_min_rate => {
                config.aimd_max_rate = rate;
                tracing::info!(
                    "Kraken iapi AIMD max-rate override: {:.2} req/s",
                    config.aimd_max_rate
                );
            }
            _ => tracing::warn!(
                "Ignoring invalid TYPHOON_KRAKEN_IAPI_AIMD_MAX_RATE={raw_max_rate:?}"
            ),
        }
    }
    let _ = typhoon_engine::broker::kraken::iapi_limiter_init(config);
}

pub(super) fn spawn_async_cache_open(
    rt_handle: &tokio::runtime::Handle,
) -> (
    Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
    std::sync::mpsc::Receiver<(Arc<SqliteCache>, PreloadedCacheMarks)>,
) {
    // On a 3.9 GB database, SqliteCache::open() + PRAGMA setup can take 10+ seconds.
    // We defer it: window appears immediately, cache arrives via channel on first frame.
    // The shared_cache is an Arc<RwLock> so the background thread can pick it up later.
    let shared_cache: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>> =
        Arc::new(std::sync::RwLock::new(None));
    let (cache_tx, cache_rx) = std::sync::mpsc::sync_channel::<(Arc<SqliteCache>, PreloadedCacheMarks)>(1);
    let shared = shared_cache.clone();
    rt_handle.spawn_blocking(move || {
        let parent = cache_dir();
        // Create cache directory if it doesn't exist (fresh install).
        // For a custom-configured NAS dir this also creates the leaf dir if
        // the user pointed at something like `/mnt/nas/typhoon/cache` that
        // exists only as a parent mount.
        if let Err(e) = std::fs::create_dir_all(&parent) {
            tracing::warn!("Failed to create cache dir {}: {}", parent.display(), e);
        }
        let db_path = cache_db_path();
        tracing::debug!("Cache-open thread: opening {}...", db_path.display());
        match SqliteCache::open(&db_path) {
            Ok(c) => {
                tracing::debug!("Cache-open thread: opened OK");
                // Repair bar_count=0 entries (from old versions)
                match c.repair_bar_counts() {
                    Ok(n) if n > 0 => {
                        tracing::debug!("Cache-open thread: repaired {} bar_count entries", n)
                    }
                    Ok(_) => {}
                    Err(e) => tracing::warn!("Cache-open thread: repair_bar_counts failed: {e}"),
                }
                // One-shot migration: M1/M5 are valid native Kraken targets
                // (Spot + Equities/xStocks). Drop stale low-TF provider-assist rows
                // from Alpaca/Yahoo so freed pages host bars we actually use.
                // Flagged so we don't re-run.
                const NON_SPOT_M1M5_PURGE_KEY: &str = "migration:non_spot_provider_m1m5_purged_v1";
                let already_purged = matches!(c.get_kv(NON_SPOT_M1M5_PURGE_KEY), Ok(Some(_)));
                if !already_purged {
                    match c.delete_non_spot_low_timeframe_bars() {
                        Ok((deleted, freed)) => {
                            tracing::info!(
                                "Cache-open thread: purged {deleted} provider-assist M1/M5 rows, freed {} MB",
                                freed / 1_048_576
                            );
                            if let Err(e) = c.put_kv(
                                NON_SPOT_M1M5_PURGE_KEY,
                                &chrono::Utc::now().to_rfc3339(),
                            ) {
                                tracing::warn!(
                                    "Cache-open thread: failed to record purge flag: {e}"
                                );
                            }
                        }
                        Err(e) => tracing::warn!(
                            "Cache-open thread: provider-assist M1/M5 purge failed: {e}"
                        ),
                    }
                }
                // Load heavy mark data HERE in the blocking thread (get_kv + deserialize
                // + filter + HashMap collect) using &c. Then wrap.
                let mut preloaded = PreloadedCacheMarks::default();

                // Alpaca no-data
                if let Ok(Some(json)) = c.get_kv("alpaca:no_data_pairs") {
                    if let Some(entries) = deserialize_alpaca_no_data_pairs(&json) {
                        preloaded.alpaca_no_data_pairs = entries
                            .into_iter()
                            .filter(|entry| !obsolete_nonspot_low_timeframe("alpaca", &entry.timeframe))
                            .map(|entry| (alpaca_fetch_key(&entry.symbol, &entry.timeframe), entry))
                            .collect();
                    }
                }

                // Alpaca backfill complete
                if let Ok(Some(json)) = c.get_kv("alpaca:backfill_complete_pairs") {
                    if let Ok(entries) = serde_json::from_str::<Vec<AlpacaBackfillCompletePair>>(&json) {
                        preloaded.alpaca_backfill_complete_pairs = entries
                            .into_iter()
                            .filter(|entry| !obsolete_nonspot_low_timeframe("alpaca", &entry.timeframe))
                            .map(|entry| (alpaca_fetch_key(&entry.symbol, &entry.timeframe), entry))
                            .collect();
                    }
                }

                // Kraken backfill
                if let Ok(Some(json)) = c.get_kv("kraken:backfill_complete_pairs") {
                    if let Ok(entries) = serde_json::from_str::<Vec<AlpacaBackfillCompletePair>>(&json) {
                        preloaded.kraken_backfill_complete_pairs = entries
                            .into_iter()
                            .map(|entry| (alpaca_fetch_key(&entry.symbol, &entry.timeframe), entry))
                            .collect();
                    }
                }

                // Kraken futures backfill
                if let Ok(Some(json)) = c.get_kv("kraken-futures:backfill_complete_pairs") {
                    if let Ok(entries) = serde_json::from_str::<Vec<AlpacaBackfillCompletePair>>(&json) {
                        preloaded.kraken_futures_backfill_complete_pairs = entries
                            .into_iter()
                            .map(|entry| (alpaca_fetch_key(&entry.symbol, &entry.timeframe), entry))
                            .collect();
                    }
                }

                let arc = Arc::new(c);

                // Publish to both: RwLock for background thread, channel for UI
                if let Ok(mut guard) = shared.write() {
                    *guard = Some(arc.clone());
                    tracing::debug!("Cache-open thread: published to RwLock");
                }
                let _ = cache_tx.send((arc, preloaded));
                tracing::debug!("Cache-open thread: sent to UI channel (with preloaded marks)");
            }
            Err(e) => {
                tracing::error!("Cache-open thread: FAILED: {e}");
            }
        }
    });
    (shared_cache, cache_rx)
}

impl TyphooNApp {
    /// One-time cleanup: drop the orphaned Yahoo intraday bar KVs left over from
    /// before the Yahoo chart lane was narrowed to daily-and-up. M15/M30/H1 are no
    /// longer fetched, merged, or counted, so those rows are dead weight. Gated by
    /// a persisted KV flag (runs at most once) and executed on a blocking worker so
    /// the DELETE never touches the render thread.
    fn maybe_purge_orphaned_yahoo_intraday(&self) {
        let Some(cache) = self.cache.clone() else {
            return;
        };
        self.rt_handle.spawn_blocking(move || {
            const FLAG: &str = "maint:yahoo_intraday_purged_v1";
            if cache.get_kv(FLAG).ok().flatten().is_some() {
                return;
            }
            // Let the cold-start chart loads + initial sync settle before touching
            // the cache. Combined with the chunked delete, the purge then never
            // contends the conn lock against the startup chart reads.
            std::thread::sleep(std::time::Duration::from_secs(45));
            match cache
                .purge_bars_for_source_timeframes("yahoo-chart", &["15Min", "30Min", "1Hour"])
            {
                Ok(n) => {
                    tracing::info!(
                        "Purged {n} orphaned Yahoo intraday bar KVs (M15/M30/H1 no longer synced)"
                    );
                    let _ = cache.put_kv(FLAG, "1");
                }
                Err(e) => tracing::warn!("Yahoo intraday purge skipped: {e}"),
            }
        });
    }

    pub(crate) fn tick_cache_startup(&mut self) {
        if self.heavy_sync_in_progress {
            // During heavy (startup backfill per log), the cache receive and basic loads still happen,
            // but skip slower hydrate/sync parts in this tick to keep pre_broker fast.
            // Full load happens over frames.
            if self.cache.is_none() {
                // still drain the rx
            } else {
                return;
            }
        }
        // ── Receive async cache open result (with preloaded marks) ───────
        if self.cache.is_none() {
            if let Some(ref rx) = self.cache_rx {
                if let Ok((c, preloaded)) = rx.try_recv() {
                    self.log.push_back(LogEntry::info("Cache opened"));
                    self.cache = Some(c);
                    self.cache_rx = None; // done, drop receiver

                    // Assign preloaded (deser + HashMap build already done in blocking thread)
                    self.alpaca_no_data_pairs = preloaded.alpaca_no_data_pairs;
                    self.alpaca_backfill_complete_pairs = preloaded.alpaca_backfill_complete_pairs;
                    self.kraken_backfill_complete_pairs = preloaded.kraken_backfill_complete_pairs;
                    self.kraken_futures_backfill_complete_pairs = preloaded.kraken_futures_backfill_complete_pairs;

                    self.alpaca_no_data_loaded = true;
                    self.alpaca_backfill_complete_loaded = true;
                    self.kraken_backfill_complete_loaded = true;
                    self.kraken_futures_backfill_complete_loaded = true;
                    self.alpaca_no_data_dirty_since = None;
                    self.alpaca_backfill_complete_dirty_since = None;
                    self.kraken_backfill_complete_dirty_since = None;
                    self.kraken_futures_backfill_complete_dirty_since = None;
                }
            }
        }
        // Load charts + lighter state once cache arrives. Heavy mark loads moved to open thread.
        if !self.cache_loaded && self.cache.is_some() {
            self.cache_loaded = true;
            self.maybe_purge_orphaned_yahoo_intraday();
            self.hydrate_loaded_charts();
            self.sync_preferences_load();
            self.alpaca_retry_load();
            self.unresolvable_load();
            // Mark loads skipped here (pre-populated above)
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
                // Multi-account credential slots 2–4 (ADR-130). Same
                // keyring-then-SQLite-fallback rule as the table above.
                let load_credential = |key: &str| -> Option<String> {
                    match keyring::load(key) {
                        Ok(Some(v)) if !v.is_empty() => Some(v),
                        _ => cache_ref
                            .as_ref()
                            .and_then(|cache| cache.get_kv(&format!("cred:{}", key)).ok().flatten())
                            .filter(|v| !v.is_empty()),
                    }
                };
                let mut extra_slots_loaded = 0usize;
                // Load creds for every persisted slot (dynamic account count) —
                // the slot Vecs were sized by `sync_preferences_load()` above.
                let extra_slot_count = self
                    .alpaca_extra_accounts
                    .len()
                    .max(self.kraken_extra_accounts.len());
                for slot in 2..=(extra_slot_count + 1) {
                    let idx = slot - 2;
                    let (ak, sk) = super::broker_accounts::alpaca_slot_keyring_keys(slot);
                    if let (Some(key), Some(secret)) = (load_credential(&ak), load_credential(&sk))
                    {
                        if let Some(acct) = self.alpaca_extra_accounts.get_mut(idx) {
                            acct.api_key = key;
                            acct.secret = secret;
                            extra_slots_loaded += 1;
                        }
                    }
                    let (kk, ks) = super::broker_accounts::kraken_slot_keyring_keys(slot);
                    if let (Some(key), Some(secret)) = (load_credential(&kk), load_credential(&ks))
                    {
                        if let Some(acct) = self.kraken_extra_accounts.get_mut(idx) {
                            acct.api_key = key;
                            acct.secret = secret;
                            extra_slots_loaded += 1;
                        }
                    }
                }
                if extra_slots_loaded > 0 {
                    self.log.push_back(LogEntry::info(format!(
                        "Loaded {} extra broker account slot(s)",
                        extra_slots_loaded
                    )));
                }
            }
            // Auto-connect all configured Alpaca accounts (pooled, ADR-130).
            if self.alpaca_enabled {
                self.send_alpaca_connect();
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
                    extra_accounts: self.kraken_extra_account_specs(),
                    primary_paper: self.kraken_paper,
                });
                self.log
                    .push_back(LogEntry::info("Kraken auto-connecting..."));
            }

            // Defer chart loading to subsequent frames — don't block the first frame
            // Charts will load progressively (one per frame) via the deferred_chart_loads mechanism
            self.deferred_chart_loads = if self.mtf_enabled {
                let active_key = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| super::chart_ops::mtf_grid_symbol_key(&c.symbol).to_ascii_uppercase());
                let mut idxs: Vec<usize> = self
                    .charts
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| c.bars.is_empty())
                    .map(|(i, _)| i)
                    .collect();
                // Front-load the focused symbol's cells so the visible MTF grid fills
                // first; the background loaders are capped, so ordering decides which
                // cells the user sees populate soonest.
                if let Some(active_key) = active_key {
                    idxs.sort_by_key(|&i| {
                        self.charts
                            .get(i)
                            .map(|c| {
                                super::chart_ops::mtf_grid_symbol_key(&c.symbol)
                                    .to_ascii_uppercase()
                                    != active_key
                            })
                            .unwrap_or(true)
                    });
                }
                idxs.into()
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
    }
}
