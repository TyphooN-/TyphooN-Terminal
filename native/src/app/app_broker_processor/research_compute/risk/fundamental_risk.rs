use super::*;

pub(super) fn handle_fundamental_risk_compute(
    cmd: BrokerCmd,
    broker_msg_tx_clone: tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    shared_cache_broker: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
) {
    match cmd {
        // Leverage, accruals, realized-volatility, cash-flow, and short-interest research
        BrokerCmd::ComputeLeverageSnapshot {
            symbol,
            total_debt_fund,
            cash_fund,
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
                let snap = research::compute_leverage_snapshot(
                    &symbol,
                    &today,
                    &statements,
                    total_debt_fund,
                    cash_fund,
                );
                let _ = msg_tx.send(BrokerMsg::LeverageSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeAccrualsSnapshot { symbol } => {
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
                let snap = research::compute_accruals_snapshot(&symbol, &today, &statements);
                let _ = msg_tx.send(BrokerMsg::AccrualsSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeRealizedVolSnapshot {
            symbol,
            current_atm_iv_pct,
            bars_json,
        } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let bars: Vec<research::HistoricalPriceRow> =
                    serde_json::from_str(&bars_json).unwrap_or_default();
                let iv = current_atm_iv_pct.unwrap_or(0.0);
                let snap = research::compute_realized_vol_snapshot(&symbol, &today, &bars, iv);
                let _ = msg_tx.send(BrokerMsg::RealizedVolSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeFcfYieldSnapshot {
            symbol,
            market_cap,
            stock_price,
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
                let snap = research::compute_fcf_yield_snapshot(
                    &symbol,
                    &today,
                    &statements,
                    market_cap,
                    stock_price,
                );
                let _ = msg_tx.send(BrokerMsg::FcfYieldSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeShortInterestSnapshot {
            symbol,
            shares_out,
            float_shares,
            short_pct_of_float,
            short_ratio_reported,
            bars_json,
        } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let bars: Vec<research::HistoricalPriceRow> =
                    serde_json::from_str(&bars_json).unwrap_or_default();
                let snap = research::compute_short_interest_snapshot(
                    &symbol,
                    &today,
                    shares_out,
                    float_shares,
                    short_pct_of_float,
                    short_ratio_reported,
                    &bars,
                );
                let _ = msg_tx.send(BrokerMsg::ShortInterestSnapshotMsg(symbol, snap));
            });
        }
        _ => {}
    }
}
