use super::*;

pub(super) fn handle_volatility_compute(
    cmd: BrokerCmd,
    broker_msg_tx_clone: tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    _shared_cache_broker: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
) {
    match cmd {
        BrokerCmd::ComputeIvolSnapshot {
            symbol,
            current_atm_iv_pct,
            history_json,
        } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let history: Vec<research::IvolObservation> =
                    serde_json::from_str(&history_json).unwrap_or_default();
                let snap =
                    research::compute_ivol_snapshot(&symbol, &today, current_atm_iv_pct, &history);
                let _ = msg_tx.send(BrokerMsg::IvolSnapshotMsg(symbol, snap));
            });
        }
        _ => { /* not volatility */ }
    }
}
