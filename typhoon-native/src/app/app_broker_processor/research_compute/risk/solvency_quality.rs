use super::*;

pub(super) fn handle_solvency_quality_compute(
    cmd: BrokerCmd,
    broker_msg_tx_clone: tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    shared_cache_broker: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
) {
    match cmd {
        // Solvency, quality, volatility-estimator, EPS-beat, and price-target research
        BrokerCmd::ComputeAltmanZSnapshot {
            symbol,
            market_value_equity,
        } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let mut statements = research::FinancialStatements::default();
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        if let Ok(Some(s)) = research::get_financials(&conn, &symbol) {
                            statements = s;
                        }
                    }
                }
                let snap = research::compute_altman_z_snapshot(
                    &symbol,
                    &today,
                    &statements,
                    market_value_equity,
                );
                let _ = msg_tx.send(BrokerMsg::AltmanZSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputePiotroskiSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let mut statements = research::FinancialStatements::default();
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        if let Ok(Some(s)) = research::get_financials(&conn, &symbol) {
                            statements = s;
                        }
                    }
                }
                let snap = research::compute_piotroski_snapshot(&symbol, &today, &statements);
                let _ = msg_tx.send(BrokerMsg::PiotroskiSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeOhlcVolSnapshot {
            symbol,
            window_days,
            bars_json,
        } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let bars: Vec<research::HistoricalPriceRow> =
                    serde_json::from_str(&bars_json).unwrap_or_default();
                let snap = research::compute_ohlc_vol_snapshot(&symbol, &today, &bars, window_days);
                let _ = msg_tx.send(BrokerMsg::OhlcVolSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeEpsBeatSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let mut reports: Vec<research::EarningsSurprise> = Vec::new();
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        if let Ok(Some(r)) = research::get_earnings_surprises(&conn, &symbol) {
                            reports = r;
                        }
                    }
                }
                let snap = research::compute_eps_beat_snapshot(&symbol, &today, &reports);
                let _ = msg_tx.send(BrokerMsg::EpsBeatSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputePriceTargetDispersionSnapshot {
            symbol,
            current_price,
        } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let mut target: Option<research::PriceTarget> = None;
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        if let Ok(Some(t)) = research::get_price_target(&conn, &symbol) {
                            target = Some(t);
                        }
                    }
                }
                let snap = research::compute_price_target_dispersion(
                    &symbol,
                    &today,
                    current_price,
                    target.as_ref(),
                );
                let _ = msg_tx.send(BrokerMsg::PriceTargetDispersionSnapshotMsg(symbol, snap));
            });
        }
        _ => {}
    }
}
