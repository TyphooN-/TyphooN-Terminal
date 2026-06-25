use std::sync::Arc;

use typhoon_engine::broker::protocol::{BrokerCmd, BrokerMsg};
use typhoon_engine::core::cache::SqliteCache;

pub fn handle_storage_command(
    cmd: BrokerCmd,
    broker_msg_tx_clone: tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    importing_flag: Arc<std::sync::atomic::AtomicBool>,
    shared_cache_broker: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
) {
    match cmd {
        BrokerCmd::CompactStorage { db_path: _, level } => {
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            let importing = importing_flag.clone();
            tokio::task::spawn_blocking(move || {
                // RAII guard — flag flip back to false happens on every
                // exit (Ok/Err arms + panic unwind) so a compact crash
                // can't wedge the background stats worker permanently.
                importing.store(true, std::sync::atomic::Ordering::Relaxed);
                struct ImportingGuard(std::sync::Arc<std::sync::atomic::AtomicBool>);
                impl Drop for ImportingGuard {
                    fn drop(&mut self) {
                        self.0.store(false, std::sync::atomic::Ordering::Relaxed);
                    }
                }
                let _guard = ImportingGuard(importing.clone());
                match shared_cache_broker
                    .read()
                    .ok()
                    .and_then(|g| g.clone())
                    .ok_or("Cache not ready".to_string())
                {
                    Ok(cache) => {
                        let msg_tx2 = msg_tx.clone();
                        match cache.compact_storage(
                            level,
                            Some(&|processed, total, key, old_size, new_size| {
                                if processed % 200 == 0 || processed == total {
                                    let _ = msg_tx2.send(BrokerMsg::OrderResult(format!(
                                        "Compact: {}/{} — {} ({} → {} bytes)",
                                        processed, total, key, old_size, new_size
                                    )));
                                }
                            }),
                        ) {
                            Ok((count, saved)) => {
                                // Reclaim freed pages after compaction reduced blob sizes
                                let _ = cache.incremental_vacuum(10000);
                                let _ = msg_tx.send(BrokerMsg::OrderResult(format!(
                                    "Compact complete: {} entries, {:.1} MB saved",
                                    count,
                                    saved as f64 / 1024.0 / 1024.0
                                )));
                            }
                            Err(e) => {
                                let _ =
                                    msg_tx.send(BrokerMsg::Error(format!("Compact failed: {}", e)));
                            }
                        }
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("Cannot open cache: {e}")));
                    }
                }
            });
        }
        BrokerCmd::ScanUnusualVolume { keys } => {
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::task::spawn_blocking(move || {
                let mut results: Vec<(String, f64, f64, f64)> = Vec::new();
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    for (key, count) in &keys {
                        if *count < 30 {
                            continue;
                        }
                        if !key.contains(":1Day") {
                            continue;
                        }
                        if let Ok(Some(raw)) = cache.get_bars_raw(key) {
                            let n = raw.len();
                            if n < 21 {
                                continue;
                            }
                            let today_vol = raw[n - 1].5;
                            let avg_vol: f64 =
                                raw[n - 21..n - 1].iter().map(|r| r.5).sum::<f64>() / 20.0;
                            if avg_vol > 0.0 {
                                let ratio = today_vol / avg_vol;
                                if ratio > 1.5 {
                                    let parts: Vec<&str> = key.split(':').collect();
                                    let sym = if parts.len() >= 3 {
                                        parts[parts.len() - 2]
                                    } else {
                                        key.as_str()
                                    };
                                    // Upper-case once at creation so the per-frame filter below skips the alloc.
                                    results.push((sym.to_uppercase(), today_vol, avg_vol, ratio));
                                }
                            }
                        }
                    }
                }
                results.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));
                let _ = msg_tx.send(BrokerMsg::UnusualVolumeResults(results));
            });
        }
        _ => unreachable!("non-storage command routed to storage handler"),
    }
}
