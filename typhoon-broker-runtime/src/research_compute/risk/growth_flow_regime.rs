use super::prelude::*;

pub(super) fn handle_growth_flow_regime_compute(
    cmd: BrokerCmd,
    broker_msg_tx_clone: tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    shared_cache_broker: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
) {
    match cmd {
        BrokerCmd::ComputeGrowmSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let (mom, earm, divg) =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            (
                                research::get_momentum(&conn, &symbol).ok().flatten(),
                                research::get_earm(&conn, &symbol).ok().flatten(),
                                research::get_divg(&conn, &symbol).ok().flatten(),
                            )
                        } else {
                            (None, None, None)
                        }
                    } else {
                        (None, None, None)
                    };
                let snap = research::compute_growm_snapshot(
                    &symbol,
                    &today,
                    mom.as_ref(),
                    earm.as_ref(),
                    divg.as_ref(),
                );
                let _ = msg_tx.send(BrokerMsg::GrowmSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeFlowSnapshot {
            symbol,
            window_days,
        } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let (trades, holders) =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            (
                                research::get_insider_trades(&conn, &symbol)
                                    .ok()
                                    .flatten()
                                    .unwrap_or_default(),
                                research::get_institutional_holders(&conn, &symbol)
                                    .ok()
                                    .flatten()
                                    .unwrap_or_default(),
                            )
                        } else {
                            (Vec::new(), Vec::new())
                        }
                    } else {
                        (Vec::new(), Vec::new())
                    };
                let snap = research::compute_flow_snapshot(
                    &symbol,
                    &today,
                    &trades,
                    &holders,
                    window_days,
                );
                let _ = msg_tx.send(BrokerMsg::FlowSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeRegimeSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let (vole, tech, hra) =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            (
                                research::get_ohlc_vol(&conn, &symbol).ok().flatten(),
                                research::get_technicals(&conn, &symbol).ok().flatten(),
                                research::get_hra(&conn, &symbol).ok().flatten(),
                            )
                        } else {
                            (None, None, None)
                        }
                    } else {
                        (None, None, None)
                    };
                let snap = research::compute_regime_snapshot(
                    &symbol,
                    &today,
                    vole.as_ref(),
                    tech.as_ref(),
                    hra.as_ref(),
                );
                let _ = msg_tx.send(BrokerMsg::RegimeSnapshotMsg(symbol, snap));
            });
        }
        _ => {}
    }
}
