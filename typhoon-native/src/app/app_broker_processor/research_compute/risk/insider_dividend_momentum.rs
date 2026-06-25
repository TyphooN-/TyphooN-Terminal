use super::prelude::*;

pub(super) fn handle_insider_dividend_momentum_compute(
    cmd: BrokerCmd,
    broker_msg_tx_clone: tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    shared_cache_broker: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
) {
    match cmd {
        BrokerCmd::ComputeInsiderActivitySnapshot {
            symbol,
            window_days,
        } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let mut trades: Vec<research::InsiderTrade> = Vec::new();
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        if let Ok(Some(t)) = research::get_insider_trades(&conn, &symbol) {
                            trades = t;
                        }
                    }
                }
                let snap = research::compute_insider_activity_snapshot(
                    &symbol,
                    &today,
                    &trades,
                    window_days,
                );
                let _ = msg_tx.send(BrokerMsg::InsiderActivitySnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeDivgSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let mut divs: Vec<research::DividendRecord> = Vec::new();
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        if let Ok(Some(d)) = research::get_dividends(&conn, &symbol) {
                            divs = d;
                        }
                    }
                }
                let snap = research::compute_divg_snapshot(&symbol, &today, &divs);
                let _ = msg_tx.send(BrokerMsg::DivgSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeEarmSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let mut statements = research::FinancialStatements::default();
                let mut surprises: Vec<research::EarningsSurprise> = Vec::new();
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        if let Ok(Some(s)) = research::get_financials(&conn, &symbol) {
                            statements = s;
                        }
                        if let Ok(Some(r)) = research::get_earnings_surprises(&conn, &symbol) {
                            surprises = r;
                        }
                    }
                }
                let snap =
                    research::compute_earm_snapshot(&symbol, &today, &statements, &surprises);
                let _ = msg_tx.send(BrokerMsg::EarmSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeSectorRotationSnapshot {
            symbol,
            symbol_sector,
        } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let mut sectors: Vec<research::SectorPerformance> = Vec::new();
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        if let Ok(Some(rows)) = research::get_sector_performance(&conn) {
                            sectors = rows;
                        }
                    }
                }
                let snap = research::compute_sector_rotation_snapshot(
                    &symbol,
                    &today,
                    &symbol_sector,
                    &sectors,
                );
                let _ = msg_tx.send(BrokerMsg::SectorRotationSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeUpdmSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let mut actions: Vec<research::RatingChange> = Vec::new();
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        if let Ok(Some(a)) = research::get_rating_changes(&conn, &symbol) {
                            actions = a;
                        }
                    }
                }
                let snap = research::compute_updm_snapshot(&symbol, &today, &actions);
                let _ = msg_tx.send(BrokerMsg::UpdmSnapshotMsg(symbol, snap));
            });
        }
        _ => {}
    }
}
