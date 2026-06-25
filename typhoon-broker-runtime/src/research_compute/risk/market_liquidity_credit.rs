use super::prelude::*;

pub(super) fn handle_market_liquidity_credit_compute(
    cmd: BrokerCmd,
    broker_msg_tx_clone: tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    shared_cache_broker: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
) {
    match cmd {
        BrokerCmd::ComputeMomentumSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let mut bars: Vec<research::HistoricalPriceRow> = Vec::new();
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        if let Ok(Some(rows)) = research::get_historical_price(&conn, &symbol) {
                            bars = rows;
                        }
                    }
                }
                let snap = research::compute_momentum_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::MomentumSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeLiquiditySnapshot {
            symbol,
            window_days,
            shares_outstanding,
        } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let mut bars: Vec<research::HistoricalPriceRow> = Vec::new();
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        if let Ok(Some(rows)) = research::get_historical_price(&conn, &symbol) {
                            bars = rows;
                        }
                    }
                }
                let snap = research::compute_liquidity_snapshot(
                    &symbol,
                    &today,
                    &bars,
                    shares_outstanding,
                    window_days,
                );
                let _ = msg_tx.send(BrokerMsg::LiquiditySnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeBreakoutSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let mut bars: Vec<research::HistoricalPriceRow> = Vec::new();
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        if let Ok(Some(rows)) = research::get_historical_price(&conn, &symbol) {
                            bars = rows;
                        }
                    }
                }
                let snap = research::compute_breakout_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::BreakoutSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCashCycleSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let statements =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            research::get_financials(&conn, &symbol)
                                .ok()
                                .flatten()
                                .unwrap_or_default()
                        } else {
                            research::FinancialStatements::default()
                        }
                    } else {
                        research::FinancialStatements::default()
                    };
                let snap = research::compute_cash_cycle_snapshot(&symbol, &today, &statements);
                let _ = msg_tx.send(BrokerMsg::CashCycleSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCreditSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let (altz, ptfs, lev, acrl) =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            (
                                research::get_altman_z(&conn, &symbol).ok().flatten(),
                                research::get_piotroski(&conn, &symbol).ok().flatten(),
                                research::get_leverage(&conn, &symbol).ok().flatten(),
                                research::get_accruals(&conn, &symbol).ok().flatten(),
                            )
                        } else {
                            (None, None, None, None)
                        }
                    } else {
                        (None, None, None, None)
                    };
                let snap = research::compute_credit_snapshot(
                    &symbol,
                    &today,
                    altz.as_ref(),
                    ptfs.as_ref(),
                    lev.as_ref(),
                    acrl.as_ref(),
                );
                let _ = msg_tx.send(BrokerMsg::CreditSnapshotMsg(symbol, snap));
            });
        }
        _ => {}
    }
}
