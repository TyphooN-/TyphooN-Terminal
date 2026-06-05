use super::*;

pub(super) fn handle_darwin_command(
    cmd: BrokerCmd,
    broker_msg_tx_clone: tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    importing_flag: Arc<std::sync::atomic::AtomicBool>,
    shared_cache_broker: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
) {
    match cmd {
        BrokerCmd::DarwinImportAll { dir, db_path: _ } => {
            // Spawn a dedicated thread so we don't block the broker command loop
            let msg_tx = broker_msg_tx_clone.clone();
            let importing = importing_flag.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::task::spawn_blocking(move || {
                // RAII release so a panic in the XLSX import (openpyxl
                // row decode, SQL insert, etc.) doesn't leave the flag
                // stuck true and the background stats worker silently
                // skipping every cycle until terminal restart.
                importing.store(true, std::sync::atomic::Ordering::Relaxed);
                struct ImportingGuard(std::sync::Arc<std::sync::atomic::AtomicBool>);
                impl Drop for ImportingGuard {
                    fn drop(&mut self) {
                        self.0.store(false, std::sync::atomic::Ordering::Relaxed);
                    }
                }
                let _guard = ImportingGuard(importing.clone());
                let _ = msg_tx.send(BrokerMsg::OrderResult(format!(
                    "DARWIN XLSX scan: {}...",
                    dir.display()
                )));
                match std::fs::read_dir(&dir) {
                    Ok(entries) => {
                        let mut xlsx_files: Vec<std::path::PathBuf> = entries
                            .filter_map(|e| e.ok())
                            .map(|e| e.path())
                            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("xlsx"))
                            .collect();
                        xlsx_files.sort();
                        if xlsx_files.is_empty() {
                            let _ = msg_tx.send(BrokerMsg::Error(format!(
                                "No .xlsx files found in {}",
                                dir.display()
                            )));
                        } else {
                            let _ = msg_tx.send(BrokerMsg::OrderResult(format!(
                                "Found {} XLSX files",
                                xlsx_files.len()
                            )));
                            if let Some(cache) =
                                shared_cache_broker.read().ok().and_then(|g| g.clone())
                            {
                                if let Ok(conn) = cache.connection() {
                                    let _ = darwin::create_darwin_tables(&conn);
                                    let mut total_deals = 0usize;
                                    let mut total_positions = 0usize;
                                    let mut imported = 0usize;
                                    for path in &xlsx_files {
                                        let stem =
                                            path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                                        let ticker = stem
                                            .split(&['_', '-', ' '][..])
                                            .next()
                                            .unwrap_or(stem)
                                            .to_uppercase();
                                        if ticker.is_empty() {
                                            continue;
                                        }
                                        match darwin::import_darwin_xlsx(
                                            &conn,
                                            &path.display().to_string(),
                                            &ticker,
                                        ) {
                                            Ok((name, deals, positions)) => {
                                                total_deals += deals;
                                                total_positions += positions;
                                                imported += 1;
                                                let _ =
                                                    msg_tx.send(BrokerMsg::OrderResult(format!(
                                                        "Imported {}: {} deals, {} positions ({})",
                                                        name,
                                                        deals,
                                                        positions,
                                                        path.file_name()
                                                            .unwrap_or_default()
                                                            .to_string_lossy()
                                                    )));
                                            }
                                            Err(e) => {
                                                let _ = msg_tx.send(BrokerMsg::Error(format!(
                                                    "Import {} failed: {}",
                                                    path.file_name()
                                                        .unwrap_or_default()
                                                        .to_string_lossy(),
                                                    e
                                                )));
                                            }
                                        }
                                    }
                                    let _ = msg_tx.send(BrokerMsg::OrderResult(format!(
                                                "DARWIN import complete: {}/{} files, {} deals, {} positions",
                                                imported, xlsx_files.len(), total_deals, total_positions
                                            )));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!(
                            "Cannot read dir {}: {}",
                            dir.display(),
                            e
                        )));
                    }
                }
                // Flag release via ImportingGuard's Drop.
            });
        }
        BrokerCmd::ExportDarwinData => {
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::task::spawn_blocking(move || {
                match shared_cache_broker
                    .read()
                    .ok()
                    .and_then(|g| g.clone())
                    .ok_or("Cache not ready".to_string())
                {
                    Ok(cache) => {
                        if let Ok(conn) = cache.connection() {
                            match darwin::export_darwin_data(&conn) {
                                Ok((json, accts, deals, positions)) => {
                                    let path = dirs_home().join("cache").join("darwin_export.json");
                                    match std::fs::write(&path, &json) {
                                        Ok(_) => {
                                            let _ = msg_tx.send(BrokerMsg::OrderResult(format!(
                                                        "DARWIN export: {} accounts, {} deals, {} positions -> {}",
                                                        accts, deals, positions, path.display()
                                                    )));
                                        }
                                        Err(e) => {
                                            let _ = msg_tx.send(BrokerMsg::Error(format!(
                                                "Write failed: {e}"
                                            )));
                                        }
                                    }
                                }
                                Err(e) => {
                                    let _ = msg_tx
                                        .send(BrokerMsg::Error(format!("Export failed: {e}")));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(e));
                    }
                }
            });
        }
        BrokerCmd::ImportDarwinData { json } => {
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::task::spawn_blocking(move || {
                match shared_cache_broker
                    .read()
                    .ok()
                    .and_then(|g| g.clone())
                    .ok_or("Cache not ready".to_string())
                {
                    Ok(cache) => {
                        if let Ok(conn) = cache.connection() {
                            match darwin::import_darwin_data(&conn, &json) {
                                Ok((accts, deals, positions)) => {
                                    let _ = msg_tx.send(BrokerMsg::OrderResult(format!(
                                        "DARWIN import: {} accounts, {} deals, {} positions",
                                        accts, deals, positions
                                    )));
                                }
                                Err(e) => {
                                    let _ = msg_tx
                                        .send(BrokerMsg::Error(format!("Import failed: {e}")));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(e));
                    }
                }
            });
        }
        _ => unreachable!("non-Darwin command routed to Darwin handler"),
    }
}
