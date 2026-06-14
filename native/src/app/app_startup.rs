use super::*;

pub(super) fn install_image_loaders(cc: &eframe::CreationContext<'_>) {
    // Install the egui image loaders (PNG/JPEG/WEBP + HTTP/file URI
    // dispatch) so news article hero images and inline markdown
    // images decode from URLs without manual texture management.
    // Idempotent in practice — egui_extras dedups on tag.
    egui_extras::install_image_loaders(&cc.egui_ctx);
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
    std::sync::mpsc::Receiver<Arc<SqliteCache>>,
) {
    // On a 3.9 GB database, SqliteCache::open() + PRAGMA setup can take 10+ seconds.
    // We defer it: window appears immediately, cache arrives via channel on first frame.
    // The shared_cache is an Arc<RwLock> so the background thread can pick it up later.
    let shared_cache: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>> =
        Arc::new(std::sync::RwLock::new(None));
    let (cache_tx, cache_rx) = std::sync::mpsc::sync_channel::<Arc<SqliteCache>>(1);
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
        tracing::info!("Cache-open thread: opening {}...", db_path.display());
        match SqliteCache::open(&db_path) {
            Ok(c) => {
                tracing::info!("Cache-open thread: opened OK");
                // Repair bar_count=0 entries (from old versions)
                match c.repair_bar_counts() {
                    Ok(n) if n > 0 => {
                        tracing::info!("Cache-open thread: repaired {} bar_count entries", n)
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
                let arc = Arc::new(c);
                // Publish to both: RwLock for background thread, channel for UI
                if let Ok(mut guard) = shared.write() {
                    *guard = Some(arc.clone());
                    tracing::info!("Cache-open thread: published to RwLock");
                }
                let _ = cache_tx.send(arc);
                tracing::info!("Cache-open thread: sent to UI channel");
            }
            Err(e) => {
                tracing::error!("Cache-open thread: FAILED: {e}");
            }
        }
    });
    (shared_cache, cache_rx)
}
