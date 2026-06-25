use super::prelude::*;

pub(super) fn handle_price_stat_transform_compute(
    cmd: BrokerCmd,
    broker_msg_tx_clone: tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    shared_cache_broker: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
) {
    match cmd {
        // ── price / statistical transform family ──
        BrokerCmd::ComputeVwmaSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let bars =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            research::get_historical_price(&conn, &symbol)
                                .ok()
                                .flatten()
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };
                let snap = research::compute_vwma_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_vwma(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::VwmaSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeStddevSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let bars =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            research::get_historical_price(&conn, &symbol)
                                .ok()
                                .flatten()
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };
                let snap = research::compute_stddev_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_stddev(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::StddevSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeWmaSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let bars =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            research::get_historical_price(&conn, &symbol)
                                .ok()
                                .flatten()
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };
                let snap = research::compute_wma_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_wma(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::WmaSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeRainbowSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let bars =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            research::get_historical_price(&conn, &symbol)
                                .ok()
                                .flatten()
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };
                let snap = research::compute_rainbow_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_rainbow(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::RainbowSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeMidpointSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let bars =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            research::get_historical_price(&conn, &symbol)
                                .ok()
                                .flatten()
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };
                let snap = research::compute_midpoint_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_midpoint(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::MidpointSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeMidpriceSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let bars =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            research::get_historical_price(&conn, &symbol)
                                .ok()
                                .flatten()
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };
                let snap = research::compute_midprice_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_midprice(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::MidpriceSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeAvgpriceSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let bars =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            research::get_historical_price(&conn, &symbol)
                                .ok()
                                .flatten()
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };
                let snap = research::compute_avgprice_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_avgprice(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::AvgpriceSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeMedpriceSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let bars =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            research::get_historical_price(&conn, &symbol)
                                .ok()
                                .flatten()
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };
                let snap = research::compute_medprice_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_medprice(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::MedpriceSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeTypPriceSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let bars =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            research::get_historical_price(&conn, &symbol)
                                .ok()
                                .flatten()
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };
                let snap = research::compute_typprice_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_typprice(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::TypPriceSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeWclPriceSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let bars =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            research::get_historical_price(&conn, &symbol)
                                .ok()
                                .flatten()
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };
                let snap = research::compute_wclprice_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_wclprice(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::WclPriceSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeVarianceSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let bars =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            research::get_historical_price(&conn, &symbol)
                                .ok()
                                .flatten()
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };
                let snap = research::compute_variance_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_variance(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::VarianceSnapshotMsg(symbol, snap));
            });
        }
        _ => {}
    }
}
