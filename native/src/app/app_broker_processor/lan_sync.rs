use super::*;

pub(super) async fn handle_lan_sync_command(
    cmd: BrokerCmd,
    broker_msg_tx_clone: tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    shared_cache_broker: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
    lan_remote_tx_ref: Arc<tokio::sync::Mutex<Option<tokio::sync::mpsc::UnboundedSender<String>>>>,
    lan_client: Arc<std::sync::atomic::AtomicBool>,
    lan_reconnect_handle: &mut Option<tokio::task::AbortHandle>,
) {
    match cmd {
        BrokerCmd::LanSyncStart {
            port, passphrase, ..
        } => {
            use typhoon_engine::core::lan_sync::LanSyncServer;
            // Spawn as independent task — cert generation is CPU-heavy (100-500ms)
            // and must not block the broker command loop
            let msg_tx = broker_msg_tx_clone.clone();
            let shared = shared_cache_broker.clone();
            tokio::spawn(async move {
                // Wait for cache to be ready (up to 30s)
                let mut cache_arc = shared.read().ok().and_then(|g| g.clone());
                if cache_arc.is_none() {
                    for _ in 0..30 {
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        cache_arc = shared.read().ok().and_then(|g| g.clone());
                        if cache_arc.is_some() {
                            break;
                        }
                    }
                }
                if let Some(cache_arc) = cache_arc {
                    match LanSyncServer::start(cache_arc, port, &passphrase).await {
                        Ok(_server) => {
                            let _ = msg_tx.send(BrokerMsg::OrderResult(format!(
                                "LAN sync server running on wss://0.0.0.0:{}",
                                port
                            )));
                            // Keep server alive — don't let _server drop
                            // The accept loop runs inside a spawned task, so it survives
                            // even after _server is dropped (JoinHandle detaches on drop)
                        }
                        Err(e) => {
                            let _ = msg_tx
                                .send(BrokerMsg::Error(format!("LAN sync server failed: {}", e)));
                        }
                    }
                } else {
                    let _ = msg_tx.send(BrokerMsg::Error("LAN sync: cache not ready yet".into()));
                }
            });
        }
        BrokerCmd::LanSyncConnect {
            host,
            port,
            passphrase,
            ..
        } => {
            use typhoon_engine::core::lan_sync::LanSyncClient;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared = shared_cache_broker.clone();
            let lan_remote = lan_remote_tx_ref.clone();
            let lan_flag = lan_client.clone();
            // Store abort handle so LanSyncStop can kill the reconnect loop
            let reconnect_task = tokio::spawn(async move {
                // Wait for cache to be ready (up to 30s) — handles startup race
                // where LAN auto-connect fires before async cache-open completes.
                let mut cache_arc = shared.read().ok().and_then(|g| g.clone());
                if cache_arc.is_none() {
                    for _ in 0..30 {
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        cache_arc = shared.read().ok().and_then(|g| g.clone());
                        if cache_arc.is_some() {
                            break;
                        }
                    }
                }
                let Some(cache_arc) = cache_arc else {
                    let _ = msg_tx.send(BrokerMsg::Error("LAN sync: cache not ready yet".into()));
                    return;
                };

                // Auto-reconnect loop: retry every 30s on failure.
                // The WebSocket stays connected and uses incremental re-sync (every 15s)
                // for bars, KV, and tables. Full reconnect only on connection drop or
                // very long intervals (2 hours) to refresh TLS certificate.
                const RESYNC_INTERVAL_SECS: u64 = 2 * 60 * 60; // 2 hours
                loop {
                    match tokio::time::timeout(
                        std::time::Duration::from_secs(10),
                        LanSyncClient::connect(cache_arc.clone(), &host, port, &passphrase),
                    )
                    .await
                    {
                        Ok(Ok((client, remote_tx))) => {
                            {
                                let mut guard = lan_remote.lock().await;
                                *guard = Some(remote_tx);
                            }
                            lan_flag.store(true, std::sync::atomic::Ordering::Relaxed);
                            let _ = msg_tx.send(BrokerMsg::OrderResult(format!(
                                "LAN sync connected to wss://{}:{}",
                                host, port
                            )));
                            // Wait for sync to complete (up to RESYNC_INTERVAL then force reconnect)
                            let timed_out = tokio::time::timeout(
                                std::time::Duration::from_secs(RESYNC_INTERVAL_SECS),
                                client.wait(),
                            )
                            .await
                            .is_err();
                            // Trigger chart reload — bars may have been synced
                            let _ = msg_tx.send(BrokerMsg::Mt5SyncDone(1));
                            // Connection dropped or periodic resync — clear state and retry
                            {
                                let mut guard = lan_remote.lock().await;
                                *guard = None;
                            }
                            lan_flag.store(false, std::sync::atomic::Ordering::Relaxed);
                            if timed_out {
                                // Periodic resync — reconnect immediately (no sleep)
                                continue;
                            }
                            let _ = msg_tx.send(BrokerMsg::Error(
                                "LAN sync disconnected — reconnecting in 30s...".into(),
                            ));
                        }
                        Ok(Err(e)) => {
                            let _ = msg_tx.send(BrokerMsg::Error(format!(
                                "LAN sync failed: {} — retrying in 30s...",
                                e
                            )));
                        }
                        Err(_) => {
                            let _ = msg_tx.send(BrokerMsg::Error(
                                "LAN sync timed out — retrying in 30s...".into(),
                            ));
                        }
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                }
            });
            // Store the abort handle for LanSyncStop
            *lan_reconnect_handle = Some(reconnect_task.abort_handle());
        }
        BrokerCmd::LanSyncStop => {
            // Abort the auto-reconnect loop task
            if let Some(handle) = lan_reconnect_handle.take() {
                handle.abort();
            }
            // Clear the LAN remote channel so commands stop being forwarded
            {
                let mut guard = lan_remote_tx_ref.lock().await;
                *guard = None;
            }
            // Clear the LAN client flag so broker commands execute locally again
            lan_client.store(false, std::sync::atomic::Ordering::Relaxed);
            let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult("LAN sync stopped".into()));
        }
        BrokerCmd::LanResyncBars => {
            let guard = lan_remote_tx_ref.lock().await;
            if let Some(ref tx) = *guard {
                let _ = tx.send("RESYNC_BARS".to_string());
                let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult(
                    "LAN resync bars requested...".into(),
                ));
            } else {
                let _ = broker_msg_tx_clone
                    .send(BrokerMsg::Error("Not connected to LAN server".into()));
            }
        }
        BrokerCmd::LanResyncDarwin => {
            let guard = lan_remote_tx_ref.lock().await;
            if let Some(ref tx) = *guard {
                let _ = tx.send("RESYNC_DARWIN".to_string());
                let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult(
                    "LAN resync DARWIN requested...".into(),
                ));
            } else {
                let _ = broker_msg_tx_clone
                    .send(BrokerMsg::Error("Not connected to LAN server".into()));
            }
        }
        _ => unreachable!("non-LAN-sync command routed to LAN sync handler"),
    }
}
