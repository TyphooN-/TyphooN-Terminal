use super::*;

pub(super) fn handle_valuation_quality_risk_compute(
    cmd: BrokerCmd,
    broker_msg_tx_clone: tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    shared_cache_broker: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
) {
    match cmd {
        BrokerCmd::ComputeRelvolSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let bars: Vec<research::HistoricalPriceRow> =
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
                let snap = research::compute_relvol_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::RelvolSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeMarginsSnapshot { symbol } => {
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
                let snap = research::compute_margins_snapshot(&symbol, &today, &statements);
                let _ = msg_tx.send(BrokerMsg::MarginsSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeValSnapshot { symbol } => {
            use typhoon_engine::core::fundamentals;
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let (subject, peers, fcfy, peer_fcf_yields, sector) = if let Some(cache) =
                    shared_cache_broker.read().ok().and_then(|g| g.clone())
                {
                    if let Ok(conn) = cache.connection() {
                        let subj = fundamentals::get_fundamentals(&conn, &symbol)
                            .ok()
                            .flatten();
                        let sector = subj.as_ref().map(|s| s.sector.clone()).unwrap_or_default();
                        let mut peers: Vec<fundamentals::Fundamentals> = Vec::new();
                        if !sector.is_empty() {
                            if let Ok(all) = fundamentals::get_all_fundamentals(&conn) {
                                for f in all {
                                    if f.sector == sector
                                        && f.symbol.to_uppercase() != symbol.to_uppercase()
                                    {
                                        peers.push(f);
                                    }
                                }
                            }
                        }
                        let subj_fcfy = research::get_fcf_yield(&conn, &symbol).ok().flatten();
                        let mut peer_fcfy: Vec<f64> = Vec::new();
                        for p in &peers {
                            if let Some(f) =
                                research::get_fcf_yield(&conn, &p.symbol).ok().flatten()
                            {
                                if f.ttm_fcf_yield_pct.is_finite() && f.ttm_fcf_yield_pct != 0.0 {
                                    peer_fcfy.push(f.ttm_fcf_yield_pct);
                                }
                            }
                        }
                        (subj, peers, subj_fcfy, peer_fcfy, sector)
                    } else {
                        (None, Vec::new(), None, Vec::new(), String::new())
                    }
                } else {
                    (None, Vec::new(), None, Vec::new(), String::new())
                };
                let snap = research::compute_val_snapshot(
                    &symbol,
                    &today,
                    &sector,
                    subject.as_ref(),
                    &peers,
                    fcfy.as_ref(),
                    &peer_fcf_yields,
                );
                let _ = msg_tx.send(BrokerMsg::ValSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeQualSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let (ptfs, margins, acrl, lev) =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            (
                                research::get_piotroski(&conn, &symbol).ok().flatten(),
                                research::get_margins(&conn, &symbol).ok().flatten(),
                                research::get_accruals(&conn, &symbol).ok().flatten(),
                                research::get_leverage(&conn, &symbol).ok().flatten(),
                            )
                        } else {
                            (None, None, None, None)
                        }
                    } else {
                        (None, None, None, None)
                    };
                let snap = research::compute_qual_snapshot(
                    &symbol,
                    &today,
                    ptfs.as_ref(),
                    margins.as_ref(),
                    acrl.as_ref(),
                    lev.as_ref(),
                );
                let _ = msg_tx.send(BrokerMsg::QualSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeRiskSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let (vole, beta, liq, shrt, altz) =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            (
                                research::get_ohlc_vol(&conn, &symbol).ok().flatten(),
                                research::get_beta(&conn, &symbol).ok().flatten(),
                                research::get_liquidity(&conn, &symbol).ok().flatten(),
                                research::get_short_interest(&conn, &symbol).ok().flatten(),
                                research::get_altman_z(&conn, &symbol).ok().flatten(),
                            )
                        } else {
                            (None, None, None, None, None)
                        }
                    } else {
                        (None, None, None, None, None)
                    };
                let snap = research::compute_risk_snapshot(
                    &symbol,
                    &today,
                    vole.as_ref(),
                    beta.as_ref(),
                    liq.as_ref(),
                    shrt.as_ref(),
                    altz.as_ref(),
                );
                let _ = msg_tx.send(BrokerMsg::RiskSnapshotMsg(symbol, snap));
            });
        }
        _ => {}
    }
}
