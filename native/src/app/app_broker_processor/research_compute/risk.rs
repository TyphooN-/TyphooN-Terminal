use super::*;

pub(super) fn handle_risk_compute(
    cmd: BrokerCmd,
    broker_msg_tx_clone: tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    shared_cache_broker: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
) {
    match cmd {
        // ── Godel Parity Round 10 ──
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
        // ── Godel Parity Round 11 ──
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
        BrokerCmd::ComputeInsstrkSnapshot {
            symbol,
            window_days,
        } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let trades =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            research::get_insider_trades(&conn, &symbol)
                                .ok()
                                .flatten()
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };
                let snap =
                    research::compute_insstrk_snapshot(&symbol, &today, &trades, window_days);
                let _ = msg_tx.send(BrokerMsg::InsstrkSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCovgSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let (pt, recs, updm) =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            (
                                research::get_price_target(&conn, &symbol).ok().flatten(),
                                research::get_analyst_recs(&conn, &symbol)
                                    .ok()
                                    .flatten()
                                    .unwrap_or_default(),
                                research::get_updm(&conn, &symbol).ok().flatten(),
                            )
                        } else {
                            (None, Vec::new(), None)
                        }
                    } else {
                        (None, Vec::new(), None)
                    };
                let snap = research::compute_covg_snapshot(
                    &symbol,
                    &today,
                    pt.as_ref(),
                    &recs,
                    updm.as_ref(),
                );
                let _ = msg_tx.send(BrokerMsg::CovgSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeVrkSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let (subject, peers) = if let Some(cache) =
                    shared_cache_broker.read().ok().and_then(|g| g.clone())
                {
                    if let Ok(conn) = cache.connection() {
                        let subj = research::get_val(&conn, &symbol).ok().flatten();
                        let sector = subj.as_ref().map(|s| s.sector.clone()).unwrap_or_default();
                        let all = research::get_all_val(&conn).unwrap_or_default();
                        let peers: Vec<research::ValueSnapshot> = all
                            .into_iter()
                            .filter(|v| {
                                !sector.is_empty()
                                    && v.sector == sector
                                    && v.symbol.to_uppercase() != symbol.to_uppercase()
                            })
                            .collect();
                        (subj, peers)
                    } else {
                        (None, Vec::new())
                    }
                } else {
                    (None, Vec::new())
                };
                let peer_refs: Vec<&research::ValueSnapshot> = peers.iter().collect();
                let snap =
                    research::compute_vrk_snapshot(&symbol, &today, subject.as_ref(), &peer_refs);
                let _ = msg_tx.send(BrokerMsg::VrkSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeQrkSnapshot { symbol } => {
            use typhoon_engine::core::fundamentals;
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let (subject, peers, sector) = if let Some(cache) =
                    shared_cache_broker.read().ok().and_then(|g| g.clone())
                {
                    if let Ok(conn) = cache.connection() {
                        let subj = research::get_qual(&conn, &symbol).ok().flatten();
                        let fund = fundamentals::get_fundamentals(&conn, &symbol)
                            .ok()
                            .flatten();
                        let sector = fund.as_ref().map(|f| f.sector.clone()).unwrap_or_default();
                        let mut peers: Vec<research::QualitySnapshot> = Vec::new();
                        if !sector.is_empty() {
                            let all = research::get_all_qual(&conn).unwrap_or_default();
                            for q in all {
                                if q.symbol.to_uppercase() == symbol.to_uppercase() {
                                    continue;
                                }
                                if let Ok(Some(pf)) =
                                    fundamentals::get_fundamentals(&conn, &q.symbol)
                                {
                                    if pf.sector == sector {
                                        peers.push(q);
                                    }
                                }
                            }
                        }
                        (subj, peers, sector)
                    } else {
                        (None, Vec::new(), String::new())
                    }
                } else {
                    (None, Vec::new(), String::new())
                };
                let peer_refs: Vec<&research::QualitySnapshot> = peers.iter().collect();
                let snap = research::compute_qrk_snapshot(
                    &symbol,
                    &today,
                    &sector,
                    subject.as_ref(),
                    &peer_refs,
                );
                let _ = msg_tx.send(BrokerMsg::QrkSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeRrkSnapshot { symbol } => {
            use typhoon_engine::core::fundamentals;
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let (subject, peers, sector) = if let Some(cache) =
                    shared_cache_broker.read().ok().and_then(|g| g.clone())
                {
                    if let Ok(conn) = cache.connection() {
                        let subj = research::get_risk(&conn, &symbol).ok().flatten();
                        let fund = fundamentals::get_fundamentals(&conn, &symbol)
                            .ok()
                            .flatten();
                        let sector = fund.as_ref().map(|f| f.sector.clone()).unwrap_or_default();
                        let mut peers: Vec<research::RiskSnapshot> = Vec::new();
                        if !sector.is_empty() {
                            let all = research::get_all_risk(&conn).unwrap_or_default();
                            for r in all {
                                if r.symbol.to_uppercase() == symbol.to_uppercase() {
                                    continue;
                                }
                                if let Ok(Some(pf)) =
                                    fundamentals::get_fundamentals(&conn, &r.symbol)
                                {
                                    if pf.sector == sector {
                                        peers.push(r);
                                    }
                                }
                            }
                        }
                        (subj, peers, sector)
                    } else {
                        (None, Vec::new(), String::new())
                    }
                } else {
                    (None, Vec::new(), String::new())
                };
                let peer_refs: Vec<&research::RiskSnapshot> = peers.iter().collect();
                let snap = research::compute_rrk_snapshot(
                    &symbol,
                    &today,
                    &sector,
                    subject.as_ref(),
                    &peer_refs,
                );
                let _ = msg_tx.send(BrokerMsg::RrkSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeRelepsgrSnapshot { symbol } => {
            use typhoon_engine::core::fundamentals;
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let (subject, peer_stmts, sector) = if let Some(cache) =
                    shared_cache_broker.read().ok().and_then(|g| g.clone())
                {
                    if let Ok(conn) = cache.connection() {
                        let subj = research::get_financials(&conn, &symbol).ok().flatten();
                        let fund = fundamentals::get_fundamentals(&conn, &symbol)
                            .ok()
                            .flatten();
                        let sector = fund.as_ref().map(|f| f.sector.clone()).unwrap_or_default();
                        let mut peers: Vec<(String, research::FinancialStatements)> = Vec::new();
                        if !sector.is_empty() {
                            if let Ok(all_f) = fundamentals::get_all_fundamentals(&conn) {
                                for f in all_f {
                                    if f.sector != sector {
                                        continue;
                                    }
                                    if f.symbol.to_uppercase() == symbol.to_uppercase() {
                                        continue;
                                    }
                                    if let Ok(Some(st)) = research::get_financials(&conn, &f.symbol)
                                    {
                                        peers.push((f.symbol.clone(), st));
                                    }
                                }
                            }
                        }
                        (subj, peers, sector)
                    } else {
                        (None, Vec::new(), String::new())
                    }
                } else {
                    (None, Vec::new(), String::new())
                };
                let snap = research::compute_relepsgr_snapshot(
                    &symbol,
                    &today,
                    &sector,
                    subject.as_ref(),
                    &peer_stmts,
                );
                let _ = msg_tx.send(BrokerMsg::RelepsgrSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputePeadSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let (surprises, bars) =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            (
                                research::get_earnings_surprises(&conn, &symbol)
                                    .ok()
                                    .flatten()
                                    .unwrap_or_default(),
                                research::get_historical_price(&conn, &symbol)
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
                let snap = research::compute_pead_snapshot(&symbol, &today, &surprises, &bars);
                let _ = msg_tx.send(BrokerMsg::PeadSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 17 ──
        BrokerCmd::ComputeSizefSnapshot { symbol } => {
            use typhoon_engine::core::fundamentals;
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let (subject_cap, peers, sector) = if let Some(cache) =
                    shared_cache_broker.read().ok().and_then(|g| g.clone())
                {
                    if let Ok(conn) = cache.connection() {
                        let fund = fundamentals::get_fundamentals(&conn, &symbol)
                            .ok()
                            .flatten();
                        let sector = fund.as_ref().map(|f| f.sector.clone()).unwrap_or_default();
                        let subj_cap: Option<f64> = fund
                            .as_ref()
                            .and_then(|f| f.market_cap)
                            .filter(|c| *c > 0.0);
                        let mut peers: Vec<(String, f64)> = Vec::new();
                        if !sector.is_empty() {
                            if let Ok(all_f) = fundamentals::get_all_fundamentals(&conn) {
                                for f in all_f {
                                    if f.sector != sector {
                                        continue;
                                    }
                                    if f.symbol.to_uppercase() == symbol.to_uppercase() {
                                        continue;
                                    }
                                    if let Some(cap) = f.market_cap {
                                        if cap > 0.0 {
                                            peers.push((f.symbol.clone(), cap));
                                        }
                                    }
                                }
                            }
                        }
                        (subj_cap, peers, sector)
                    } else {
                        (None, Vec::new(), String::new())
                    }
                } else {
                    (None, Vec::new(), String::new())
                };
                let snap =
                    research::compute_sizef_snapshot(&symbol, &today, &sector, subject_cap, &peers);
                let _ = msg_tx.send(BrokerMsg::SizefSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeMomfSnapshot { symbol } => {
            use typhoon_engine::core::fundamentals;
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let (subject, peers, sector) = if let Some(cache) =
                    shared_cache_broker.read().ok().and_then(|g| g.clone())
                {
                    if let Ok(conn) = cache.connection() {
                        let subj = research::get_momentum(&conn, &symbol).ok().flatten();
                        let fund = fundamentals::get_fundamentals(&conn, &symbol)
                            .ok()
                            .flatten();
                        let sector = fund.as_ref().map(|f| f.sector.clone()).unwrap_or_default();
                        let mut peers: Vec<research::MomentumSnapshot> = Vec::new();
                        if !sector.is_empty() {
                            let all = research::get_all_momentum(&conn).unwrap_or_default();
                            for m in all {
                                if m.symbol.to_uppercase() == symbol.to_uppercase() {
                                    continue;
                                }
                                if let Ok(Some(pf)) =
                                    fundamentals::get_fundamentals(&conn, &m.symbol)
                                {
                                    if pf.sector == sector {
                                        peers.push(m);
                                    }
                                }
                            }
                        }
                        (subj, peers, sector)
                    } else {
                        (None, Vec::new(), String::new())
                    }
                } else {
                    (None, Vec::new(), String::new())
                };
                let peer_refs: Vec<&research::MomentumSnapshot> = peers.iter().collect();
                let snap = research::compute_momf_snapshot(
                    &symbol,
                    &today,
                    &sector,
                    subject.as_ref(),
                    &peer_refs,
                );
                let _ = msg_tx.send(BrokerMsg::MomfSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputePeadrankSnapshot { symbol } => {
            use typhoon_engine::core::fundamentals;
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let (subject, peers, sector) = if let Some(cache) =
                    shared_cache_broker.read().ok().and_then(|g| g.clone())
                {
                    if let Ok(conn) = cache.connection() {
                        let subj = research::get_pead(&conn, &symbol).ok().flatten();
                        let fund = fundamentals::get_fundamentals(&conn, &symbol)
                            .ok()
                            .flatten();
                        let sector = fund.as_ref().map(|f| f.sector.clone()).unwrap_or_default();
                        let mut peers: Vec<research::PeadSnapshot> = Vec::new();
                        if !sector.is_empty() {
                            let all = research::get_all_pead(&conn).unwrap_or_default();
                            for p in all {
                                if p.symbol.to_uppercase() == symbol.to_uppercase() {
                                    continue;
                                }
                                if let Ok(Some(pf)) =
                                    fundamentals::get_fundamentals(&conn, &p.symbol)
                                {
                                    if pf.sector == sector {
                                        peers.push(p);
                                    }
                                }
                            }
                        }
                        (subj, peers, sector)
                    } else {
                        (None, Vec::new(), String::new())
                    }
                } else {
                    (None, Vec::new(), String::new())
                };
                let peer_refs: Vec<&research::PeadSnapshot> = peers.iter().collect();
                let snap = research::compute_peadrank_snapshot(
                    &symbol,
                    &today,
                    &sector,
                    subject.as_ref(),
                    &peer_refs,
                );
                let _ = msg_tx.send(BrokerMsg::PeadrankSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeFqmSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let (ptfs, margins, accruals) =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            (
                                research::get_piotroski(&conn, &symbol).ok().flatten(),
                                research::get_margins(&conn, &symbol).ok().flatten(),
                                research::get_accruals(&conn, &symbol).ok().flatten(),
                            )
                        } else {
                            (None, None, None)
                        }
                    } else {
                        (None, None, None)
                    };
                let snap = research::compute_fqm_snapshot(
                    &symbol,
                    &today,
                    ptfs.as_ref(),
                    margins.as_ref(),
                    accruals.as_ref(),
                );
                let _ = msg_tx.send(BrokerMsg::FqmSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeRevrankSnapshot { symbol } => {
            use typhoon_engine::core::fundamentals;
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let (subject, peer_stmts, sector) = if let Some(cache) =
                    shared_cache_broker.read().ok().and_then(|g| g.clone())
                {
                    if let Ok(conn) = cache.connection() {
                        let subj = research::get_financials(&conn, &symbol).ok().flatten();
                        let fund = fundamentals::get_fundamentals(&conn, &symbol)
                            .ok()
                            .flatten();
                        let sector = fund.as_ref().map(|f| f.sector.clone()).unwrap_or_default();
                        let mut peers: Vec<(String, research::FinancialStatements)> = Vec::new();
                        if !sector.is_empty() {
                            if let Ok(all_f) = fundamentals::get_all_fundamentals(&conn) {
                                for f in all_f {
                                    if f.sector != sector {
                                        continue;
                                    }
                                    if f.symbol.to_uppercase() == symbol.to_uppercase() {
                                        continue;
                                    }
                                    if let Ok(Some(st)) = research::get_financials(&conn, &f.symbol)
                                    {
                                        peers.push((f.symbol.clone(), st));
                                    }
                                }
                            }
                        }
                        (subj, peers, sector)
                    } else {
                        (None, Vec::new(), String::new())
                    }
                } else {
                    (None, Vec::new(), String::new())
                };
                let snap = research::compute_revrank_snapshot(
                    &symbol,
                    &today,
                    &sector,
                    subject.as_ref(),
                    &peer_stmts,
                );
                let _ = msg_tx.send(BrokerMsg::RevrankSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeLevrankSnapshot { symbol } => {
            use typhoon_engine::core::fundamentals;
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let (subject, peers, sector) = if let Some(cache) =
                    shared_cache_broker.read().ok().and_then(|g| g.clone())
                {
                    if let Ok(conn) = cache.connection() {
                        let subj = research::get_leverage(&conn, &symbol).ok().flatten();
                        let fund = fundamentals::get_fundamentals(&conn, &symbol)
                            .ok()
                            .flatten();
                        let sector = fund.as_ref().map(|f| f.sector.clone()).unwrap_or_default();
                        let mut peers: Vec<research::LeverageSnapshot> = Vec::new();
                        if !sector.is_empty() {
                            let all = research::get_all_leverage(&conn).unwrap_or_default();
                            for p in all {
                                if p.symbol.to_uppercase() == symbol.to_uppercase() {
                                    continue;
                                }
                                if let Ok(Some(pf)) =
                                    fundamentals::get_fundamentals(&conn, &p.symbol)
                                {
                                    if pf.sector == sector {
                                        peers.push(p);
                                    }
                                }
                            }
                        }
                        (subj, peers, sector)
                    } else {
                        (None, Vec::new(), String::new())
                    }
                } else {
                    (None, Vec::new(), String::new())
                };
                let peer_refs: Vec<&research::LeverageSnapshot> = peers.iter().collect();
                let snap = research::compute_levrank_snapshot(
                    &symbol,
                    &today,
                    &sector,
                    subject.as_ref(),
                    &peer_refs,
                );
                let _ = msg_tx.send(BrokerMsg::LevrankSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeOperankSnapshot { symbol } => {
            use typhoon_engine::core::fundamentals;
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let (subject, peers, sector) = if let Some(cache) =
                    shared_cache_broker.read().ok().and_then(|g| g.clone())
                {
                    if let Ok(conn) = cache.connection() {
                        let subj = research::get_margins(&conn, &symbol).ok().flatten();
                        let fund = fundamentals::get_fundamentals(&conn, &symbol)
                            .ok()
                            .flatten();
                        let sector = fund.as_ref().map(|f| f.sector.clone()).unwrap_or_default();
                        let mut peers: Vec<research::MarginsSnapshot> = Vec::new();
                        if !sector.is_empty() {
                            let all = research::get_all_margins(&conn).unwrap_or_default();
                            for p in all {
                                if p.symbol.to_uppercase() == symbol.to_uppercase() {
                                    continue;
                                }
                                if let Ok(Some(pf)) =
                                    fundamentals::get_fundamentals(&conn, &p.symbol)
                                {
                                    if pf.sector == sector {
                                        peers.push(p);
                                    }
                                }
                            }
                        }
                        (subj, peers, sector)
                    } else {
                        (None, Vec::new(), String::new())
                    }
                } else {
                    (None, Vec::new(), String::new())
                };
                let peer_refs: Vec<&research::MarginsSnapshot> = peers.iter().collect();
                let snap = research::compute_operank_snapshot(
                    &symbol,
                    &today,
                    &sector,
                    subject.as_ref(),
                    &peer_refs,
                );
                let _ = msg_tx.send(BrokerMsg::OperankSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeFqmrankSnapshot { symbol } => {
            use typhoon_engine::core::fundamentals;
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let (subject, peers, sector) = if let Some(cache) =
                    shared_cache_broker.read().ok().and_then(|g| g.clone())
                {
                    if let Ok(conn) = cache.connection() {
                        let subj = research::get_fqm(&conn, &symbol).ok().flatten();
                        let fund = fundamentals::get_fundamentals(&conn, &symbol)
                            .ok()
                            .flatten();
                        let sector = fund.as_ref().map(|f| f.sector.clone()).unwrap_or_default();
                        let mut peers: Vec<research::FundamentalQualityMeterSnapshot> = Vec::new();
                        if !sector.is_empty() {
                            let all = research::get_all_fqm(&conn).unwrap_or_default();
                            for p in all {
                                if p.symbol.to_uppercase() == symbol.to_uppercase() {
                                    continue;
                                }
                                if let Ok(Some(pf)) =
                                    fundamentals::get_fundamentals(&conn, &p.symbol)
                                {
                                    if pf.sector == sector {
                                        peers.push(p);
                                    }
                                }
                            }
                        }
                        (subj, peers, sector)
                    } else {
                        (None, Vec::new(), String::new())
                    }
                } else {
                    (None, Vec::new(), String::new())
                };
                let peer_refs: Vec<&research::FundamentalQualityMeterSnapshot> =
                    peers.iter().collect();
                let snap = research::compute_fqmrank_snapshot(
                    &symbol,
                    &today,
                    &sector,
                    subject.as_ref(),
                    &peer_refs,
                );
                let _ = msg_tx.send(BrokerMsg::FqmrankSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeLiqrankSnapshot { symbol } => {
            use typhoon_engine::core::fundamentals;
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let (subject, peers, sector) = if let Some(cache) =
                    shared_cache_broker.read().ok().and_then(|g| g.clone())
                {
                    if let Ok(conn) = cache.connection() {
                        let subj = research::get_liquidity(&conn, &symbol).ok().flatten();
                        let fund = fundamentals::get_fundamentals(&conn, &symbol)
                            .ok()
                            .flatten();
                        let sector = fund.as_ref().map(|f| f.sector.clone()).unwrap_or_default();
                        let mut peers: Vec<research::LiquiditySnapshot> = Vec::new();
                        if !sector.is_empty() {
                            let all = research::get_all_liquidity(&conn).unwrap_or_default();
                            for p in all {
                                if p.symbol.to_uppercase() == symbol.to_uppercase() {
                                    continue;
                                }
                                if let Ok(Some(pf)) =
                                    fundamentals::get_fundamentals(&conn, &p.symbol)
                                {
                                    if pf.sector == sector {
                                        peers.push(p);
                                    }
                                }
                            }
                        }
                        (subj, peers, sector)
                    } else {
                        (None, Vec::new(), String::new())
                    }
                } else {
                    (None, Vec::new(), String::new())
                };
                let peer_refs: Vec<&research::LiquiditySnapshot> = peers.iter().collect();
                let snap = research::compute_liqrank_snapshot(
                    &symbol,
                    &today,
                    &sector,
                    subject.as_ref(),
                    &peer_refs,
                );
                let _ = msg_tx.send(BrokerMsg::LiqrankSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeSurpstkSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let surprises =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            research::get_earnings_surprises(&conn, &symbol)
                                .ok()
                                .flatten()
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };
                let snap = research::compute_surpstk_snapshot(&symbol, &today, &surprises);
                let _ = msg_tx.send(BrokerMsg::SurpstkSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeDvdrankSnapshot { symbol } => {
            use typhoon_engine::core::fundamentals;
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let (subject, peers, sector) = if let Some(cache) =
                    shared_cache_broker.read().ok().and_then(|g| g.clone())
                {
                    if let Ok(conn) = cache.connection() {
                        let subj = research::get_divg(&conn, &symbol).ok().flatten();
                        let fund = fundamentals::get_fundamentals(&conn, &symbol)
                            .ok()
                            .flatten();
                        let sector = fund.as_ref().map(|f| f.sector.clone()).unwrap_or_default();
                        let mut peers: Vec<research::DivgSnapshot> = Vec::new();
                        if !sector.is_empty() {
                            let all = research::get_all_divg(&conn).unwrap_or_default();
                            for p in all {
                                if p.symbol.to_uppercase() == symbol.to_uppercase() {
                                    continue;
                                }
                                if let Ok(Some(pf)) =
                                    fundamentals::get_fundamentals(&conn, &p.symbol)
                                {
                                    if pf.sector == sector {
                                        peers.push(p);
                                    }
                                }
                            }
                        }
                        (subj, peers, sector)
                    } else {
                        (None, Vec::new(), String::new())
                    }
                } else {
                    (None, Vec::new(), String::new())
                };
                let peer_refs: Vec<&research::DivgSnapshot> = peers.iter().collect();
                let snap = research::compute_dvdrank_snapshot(
                    &symbol,
                    &today,
                    &sector,
                    subject.as_ref(),
                    &peer_refs,
                );
                let _ = msg_tx.send(BrokerMsg::DvdrankSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeEarmrankSnapshot { symbol } => {
            use typhoon_engine::core::fundamentals;
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let (subject, peers, sector) = if let Some(cache) =
                    shared_cache_broker.read().ok().and_then(|g| g.clone())
                {
                    if let Ok(conn) = cache.connection() {
                        let subj = research::get_earm(&conn, &symbol).ok().flatten();
                        let fund = fundamentals::get_fundamentals(&conn, &symbol)
                            .ok()
                            .flatten();
                        let sector = fund.as_ref().map(|f| f.sector.clone()).unwrap_or_default();
                        let mut peers: Vec<research::EarmSnapshot> = Vec::new();
                        if !sector.is_empty() {
                            let all = research::get_all_earm(&conn).unwrap_or_default();
                            for p in all {
                                if p.symbol.to_uppercase() == symbol.to_uppercase() {
                                    continue;
                                }
                                if let Ok(Some(pf)) =
                                    fundamentals::get_fundamentals(&conn, &p.symbol)
                                {
                                    if pf.sector == sector {
                                        peers.push(p);
                                    }
                                }
                            }
                        }
                        (subj, peers, sector)
                    } else {
                        (None, Vec::new(), String::new())
                    }
                } else {
                    (None, Vec::new(), String::new())
                };
                let peer_refs: Vec<&research::EarmSnapshot> = peers.iter().collect();
                let snap = research::compute_earmrank_snapshot(
                    &symbol,
                    &today,
                    &sector,
                    subject.as_ref(),
                    &peer_refs,
                );
                let _ = msg_tx.send(BrokerMsg::EarmrankSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeUpdgrankSnapshot { symbol } => {
            use typhoon_engine::core::fundamentals;
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let (subject, peers, sector) = if let Some(cache) =
                    shared_cache_broker.read().ok().and_then(|g| g.clone())
                {
                    if let Ok(conn) = cache.connection() {
                        let subj = research::get_updm(&conn, &symbol).ok().flatten();
                        let fund = fundamentals::get_fundamentals(&conn, &symbol)
                            .ok()
                            .flatten();
                        let sector = fund.as_ref().map(|f| f.sector.clone()).unwrap_or_default();
                        let mut peers: Vec<research::UpdmSnapshot> = Vec::new();
                        if !sector.is_empty() {
                            let all = research::get_all_updm(&conn).unwrap_or_default();
                            for p in all {
                                if p.symbol.to_uppercase() == symbol.to_uppercase() {
                                    continue;
                                }
                                if let Ok(Some(pf)) =
                                    fundamentals::get_fundamentals(&conn, &p.symbol)
                                {
                                    if pf.sector == sector {
                                        peers.push(p);
                                    }
                                }
                            }
                        }
                        (subj, peers, sector)
                    } else {
                        (None, Vec::new(), String::new())
                    }
                } else {
                    (None, Vec::new(), String::new())
                };
                let peer_refs: Vec<&research::UpdmSnapshot> = peers.iter().collect();
                let snap = research::compute_updgrank_snapshot(
                    &symbol,
                    &today,
                    &sector,
                    subject.as_ref(),
                    &peer_refs,
                );
                let _ = msg_tx.send(BrokerMsg::UpdgrankSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeGySnapshot { symbol } => {
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
                let snap = research::compute_gy_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::GySnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeDesSnapshot { symbol } => {
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
                let snap = research::compute_des_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::DesSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeDvdyieldrankSnapshot { symbol } => {
            use typhoon_engine::core::fundamentals;
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let (subject_yield, peers, sector) = if let Some(cache) =
                    shared_cache_broker.read().ok().and_then(|g| g.clone())
                {
                    if let Ok(conn) = cache.connection() {
                        let fund = fundamentals::get_fundamentals(&conn, &symbol)
                            .ok()
                            .flatten();
                        let sector = fund.as_ref().map(|f| f.sector.clone()).unwrap_or_default();
                        let subj_y = fund.as_ref().and_then(|f| f.dividend_yield);
                        let mut peers: Vec<(String, Option<f64>)> = Vec::new();
                        if !sector.is_empty() {
                            let all = fundamentals::get_all_fundamentals(&conn).unwrap_or_default();
                            for p in all {
                                if p.symbol.to_uppercase() == symbol.to_uppercase() {
                                    continue;
                                }
                                if p.sector == sector {
                                    peers.push((p.symbol.clone(), p.dividend_yield));
                                }
                            }
                        }
                        (subj_y, peers, sector)
                    } else {
                        (None, Vec::new(), String::new())
                    }
                } else {
                    (None, Vec::new(), String::new())
                };
                let snap = research::compute_dvdyieldrank_snapshot(
                    &symbol,
                    &today,
                    &sector,
                    subject_yield,
                    &peers,
                );
                let _ = msg_tx.send(BrokerMsg::DvdyieldrankSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeShrankSnapshot { symbol } => {
            use typhoon_engine::core::fundamentals;
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let (subject_short, peers, sector) = if let Some(cache) =
                    shared_cache_broker.read().ok().and_then(|g| g.clone())
                {
                    if let Ok(conn) = cache.connection() {
                        let fund = fundamentals::get_fundamentals(&conn, &symbol)
                            .ok()
                            .flatten();
                        let sector = fund.as_ref().map(|f| f.sector.clone()).unwrap_or_default();
                        let subj_s = fund.as_ref().and_then(|f| f.short_percent_of_float);
                        let mut peers: Vec<(String, Option<f64>)> = Vec::new();
                        if !sector.is_empty() {
                            let all = fundamentals::get_all_fundamentals(&conn).unwrap_or_default();
                            for p in all {
                                if p.symbol.to_uppercase() == symbol.to_uppercase() {
                                    continue;
                                }
                                if p.sector == sector {
                                    peers.push((p.symbol.clone(), p.short_percent_of_float));
                                }
                            }
                        }
                        (subj_s, peers, sector)
                    } else {
                        (None, Vec::new(), String::new())
                    }
                } else {
                    (None, Vec::new(), String::new())
                };
                let snap = research::compute_shrank_snapshot(
                    &symbol,
                    &today,
                    &sector,
                    subject_short,
                    &peers,
                );
                let _ = msg_tx.send(BrokerMsg::ShrankSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeShortrankDeltaSnapshot { symbol } => {
            use typhoon_engine::core::fundamentals;
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let (subject_history, peers, sector) = if let Some(cache) =
                    shared_cache_broker.read().ok().and_then(|g| g.clone())
                {
                    if let Ok(conn) = cache.connection() {
                        let fund = fundamentals::get_fundamentals(&conn, &symbol)
                            .ok()
                            .flatten();
                        let sector = fund.as_ref().map(|f| f.sector.clone()).unwrap_or_default();
                        let subject_history = research::get_short_interest_history(&conn, &symbol)
                            .ok()
                            .flatten()
                            .unwrap_or_default();
                        let mut peers: Vec<(String, Vec<research::ShortInterestHistoryPoint>)> =
                            Vec::new();
                        if !sector.is_empty() {
                            let all = fundamentals::get_all_fundamentals(&conn).unwrap_or_default();
                            for p in all {
                                if p.symbol.to_uppercase() == symbol.to_uppercase() {
                                    continue;
                                }
                                if p.sector == sector {
                                    let history =
                                        research::get_short_interest_history(&conn, &p.symbol)
                                            .ok()
                                            .flatten()
                                            .unwrap_or_default();
                                    peers.push((p.symbol.clone(), history));
                                }
                            }
                        }
                        (subject_history, peers, sector)
                    } else {
                        (Vec::new(), Vec::new(), String::new())
                    }
                } else {
                    (Vec::new(), Vec::new(), String::new())
                };
                let snap = research::compute_shortrank_delta_snapshot(
                    &symbol,
                    &today,
                    &sector,
                    &subject_history,
                    &peers,
                );
                let _ = msg_tx.send(BrokerMsg::ShortrankDeltaSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeInsiderconcSnapshot { symbol } => {
            use typhoon_engine::core::fundamentals;
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let (subject_shares_outstanding, subject_trades, peers, sector) =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            let fund = fundamentals::get_fundamentals(&conn, &symbol)
                                .ok()
                                .flatten();
                            let sector =
                                fund.as_ref().map(|f| f.sector.clone()).unwrap_or_default();
                            let subject_shares_outstanding =
                                fund.as_ref().and_then(|f| f.shares_outstanding);
                            let subject_trades = research::get_insider_trades(&conn, &symbol)
                                .ok()
                                .flatten()
                                .unwrap_or_default();
                            let mut peers: Vec<(String, Option<f64>, Vec<research::InsiderTrade>)> =
                                Vec::new();
                            if !sector.is_empty() {
                                let all =
                                    fundamentals::get_all_fundamentals(&conn).unwrap_or_default();
                                for p in all {
                                    if p.symbol.to_uppercase() == symbol.to_uppercase() {
                                        continue;
                                    }
                                    if p.sector == sector {
                                        let trades = research::get_insider_trades(&conn, &p.symbol)
                                            .ok()
                                            .flatten()
                                            .unwrap_or_default();
                                        peers.push((
                                            p.symbol.clone(),
                                            p.shares_outstanding,
                                            trades,
                                        ));
                                    }
                                }
                            }
                            (subject_shares_outstanding, subject_trades, peers, sector)
                        } else {
                            (None, Vec::new(), Vec::new(), String::new())
                        }
                    } else {
                        (None, Vec::new(), Vec::new(), String::new())
                    };
                let snap = research::compute_insiderconc_snapshot(
                    &symbol,
                    &today,
                    &sector,
                    subject_shares_outstanding,
                    &subject_trades,
                    &peers,
                );
                let _ = msg_tx.send(BrokerMsg::InsiderconcSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeAtrannSnapshot { symbol } => {
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
                let snap = research::compute_atrann_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::AtrannSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeDdhistSnapshot { symbol } => {
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
                let snap = research::compute_ddhist_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::DdhistSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputePriceperfSnapshot { symbol } => {
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
                let snap = research::compute_priceperf_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::PriceperfSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeMomrankMultiSnapshot { symbol } => {
            use typhoon_engine::core::fundamentals;
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let (subject, peers, sector) = if let Some(cache) =
                    shared_cache_broker.read().ok().and_then(|g| g.clone())
                {
                    if let Ok(conn) = cache.connection() {
                        let fund = fundamentals::get_fundamentals(&conn, &symbol)
                            .ok()
                            .flatten();
                        let sector = fund.as_ref().map(|f| f.sector.clone()).unwrap_or_default();
                        let subject = research::get_priceperf(&conn, &symbol).ok().flatten();
                        let mut peers: Vec<(String, Option<research::PricePerformanceSnapshot>)> =
                            Vec::new();
                        if !sector.is_empty() {
                            let all = fundamentals::get_all_fundamentals(&conn).unwrap_or_default();
                            for p in all {
                                if p.symbol.eq_ignore_ascii_case(&symbol) || p.sector != sector {
                                    continue;
                                }
                                peers.push((
                                    p.symbol.clone(),
                                    research::get_priceperf(&conn, &p.symbol).ok().flatten(),
                                ));
                            }
                        }
                        (subject, peers, sector)
                    } else {
                        (None, Vec::new(), String::new())
                    }
                } else {
                    (None, Vec::new(), String::new())
                };
                let snap = research::compute_momrank_multi_snapshot(
                    &symbol,
                    &today,
                    &sector,
                    subject.as_ref(),
                    &peers,
                );
                let _ = msg_tx.send(BrokerMsg::MomrankMultiSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeBetarankSnapshot { symbol } => {
            use typhoon_engine::core::fundamentals;
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let (subject_beta, peers, sector) = if let Some(cache) =
                    shared_cache_broker.read().ok().and_then(|g| g.clone())
                {
                    if let Ok(conn) = cache.connection() {
                        let fund = fundamentals::get_fundamentals(&conn, &symbol)
                            .ok()
                            .flatten();
                        let sector = fund.as_ref().map(|f| f.sector.clone()).unwrap_or_default();
                        let subj_b = fund.as_ref().and_then(|f| f.beta);
                        let mut peers: Vec<(String, Option<f64>)> = Vec::new();
                        if !sector.is_empty() {
                            let all = fundamentals::get_all_fundamentals(&conn).unwrap_or_default();
                            for p in all {
                                if p.symbol.to_uppercase() == symbol.to_uppercase() {
                                    continue;
                                }
                                if p.sector == sector {
                                    peers.push((p.symbol.clone(), p.beta));
                                }
                            }
                        }
                        (subj_b, peers, sector)
                    } else {
                        (None, Vec::new(), String::new())
                    }
                } else {
                    (None, Vec::new(), String::new())
                };
                let snap = research::compute_betarank_snapshot(
                    &symbol,
                    &today,
                    &sector,
                    subject_beta,
                    &peers,
                );
                let _ = msg_tx.send(BrokerMsg::BetarankSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputePegrankSnapshot { symbol } => {
            use typhoon_engine::core::fundamentals;
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let (subject_peg, peers, sector) = if let Some(cache) =
                    shared_cache_broker.read().ok().and_then(|g| g.clone())
                {
                    if let Ok(conn) = cache.connection() {
                        let fund = fundamentals::get_fundamentals(&conn, &symbol)
                            .ok()
                            .flatten();
                        let sector = fund.as_ref().map(|f| f.sector.clone()).unwrap_or_default();
                        let subj_p = fund.as_ref().and_then(|f| f.peg_ratio);
                        let mut peers: Vec<(String, Option<f64>)> = Vec::new();
                        if !sector.is_empty() {
                            let all = fundamentals::get_all_fundamentals(&conn).unwrap_or_default();
                            for p in all {
                                if p.symbol.to_uppercase() == symbol.to_uppercase() {
                                    continue;
                                }
                                if p.sector == sector {
                                    peers.push((p.symbol.clone(), p.peg_ratio));
                                }
                            }
                        }
                        (subj_p, peers, sector)
                    } else {
                        (None, Vec::new(), String::new())
                    }
                } else {
                    (None, Vec::new(), String::new())
                };
                let snap = research::compute_pegrank_snapshot(
                    &symbol,
                    &today,
                    &sector,
                    subject_peg,
                    &peers,
                );
                let _ = msg_tx.send(BrokerMsg::PegrankSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeFhighlowSnapshot { symbol } => {
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
                let snap = research::compute_fhighlow_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::FhighlowSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeRvconeSnapshot { symbol } => {
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
                let snap = research::compute_rvcone_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::RvconeSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCalpbSnapshot { symbol } => {
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
                let snap = research::compute_calpb_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::CalpbSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCorrstkSnapshot {
            symbol,
            symbol_sector,
            fmp_key,
        } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let sector_benchmark =
                    research::sector_to_benchmark_etf(&symbol_sector).map(str::to_string);
                let (mut subject_bars, mut market_bars, mut sector_bars) =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            (
                                research::get_historical_price(&conn, &symbol)
                                    .ok()
                                    .flatten()
                                    .unwrap_or_default(),
                                research::get_historical_price(&conn, "SPY")
                                    .ok()
                                    .flatten()
                                    .unwrap_or_default(),
                                sector_benchmark
                                    .as_ref()
                                    .and_then(|etf| {
                                        research::get_historical_price(&conn, etf).ok().flatten()
                                    })
                                    .unwrap_or_default(),
                            )
                        } else {
                            (Vec::new(), Vec::new(), Vec::new())
                        }
                    } else {
                        (Vec::new(), Vec::new(), Vec::new())
                    };

                if !fmp_key.trim().is_empty() {
                    let client = reqwest::Client::builder()
                        .user_agent("TyphooN-Terminal/1.0")
                        .timeout(std::time::Duration::from_secs(30))
                        .build()
                        .unwrap_or_default();
                    if subject_bars.len() < 260 {
                        if let Ok(rows) =
                            research::fetch_fmp_historical_price(&client, &symbol, &fmp_key, 1300)
                                .await
                        {
                            subject_bars = rows;
                        }
                    }
                    if market_bars.len() < 260 {
                        if let Ok(rows) =
                            research::fetch_fmp_historical_price(&client, "SPY", &fmp_key, 1300)
                                .await
                        {
                            market_bars = rows;
                        }
                    }
                    if let Some(ref etf) = sector_benchmark {
                        if sector_bars.len() < 260 {
                            if let Ok(rows) =
                                research::fetch_fmp_historical_price(&client, etf, &fmp_key, 1300)
                                    .await
                            {
                                sector_bars = rows;
                            }
                        }
                    }
                }

                let snap = research::compute_corrstk_snapshot(
                    &symbol,
                    &today,
                    &symbol_sector,
                    "SPY",
                    &subject_bars,
                    &market_bars,
                    sector_benchmark.as_deref(),
                    &sector_bars,
                );
                let _ = msg_tx.send(BrokerMsg::CorrstkSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeTlrankSnapshot { symbol } => {
            use typhoon_engine::core::fundamentals;
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let (subject_bars, peers, sector) = if let Some(cache) =
                    shared_cache_broker.read().ok().and_then(|g| g.clone())
                {
                    if let Ok(conn) = cache.connection() {
                        let subject_bars = research::get_historical_price(&conn, &symbol)
                            .ok()
                            .flatten()
                            .unwrap_or_default();
                        let subject_sector = fundamentals::get_fundamentals(&conn, &symbol)
                            .ok()
                            .flatten()
                            .map(|f| f.sector)
                            .unwrap_or_default();
                        let mut peers = Vec::new();
                        if !subject_sector.is_empty() {
                            let all = fundamentals::get_all_fundamentals(&conn).unwrap_or_default();
                            for peer in all {
                                if peer.symbol.eq_ignore_ascii_case(&symbol) {
                                    continue;
                                }
                                if peer.sector == subject_sector {
                                    let bars = research::get_historical_price(&conn, &peer.symbol)
                                        .ok()
                                        .flatten()
                                        .unwrap_or_default();
                                    peers.push((peer.symbol, bars));
                                }
                            }
                        }
                        (subject_bars, peers, subject_sector)
                    } else {
                        (Vec::new(), Vec::new(), String::new())
                    }
                } else {
                    (Vec::new(), Vec::new(), String::new())
                };
                let snap = research::compute_tlrank_snapshot(
                    &symbol,
                    &today,
                    &sector,
                    &subject_bars,
                    &peers,
                );
                let _ = msg_tx.send(BrokerMsg::TlrankSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCorrrankSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let (subject, peers, sector) =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            let subject = research::get_corrstk(&conn, &symbol).ok().flatten();
                            let subject_sector = subject
                                .as_ref()
                                .map(|s| s.symbol_sector.clone())
                                .unwrap_or_default();
                            let peers = if !subject_sector.is_empty() {
                                research::get_all_corrstk(&conn)
                                    .unwrap_or_default()
                                    .into_iter()
                                    .filter(|p| {
                                        !p.symbol.eq_ignore_ascii_case(&symbol)
                                            && p.symbol_sector == subject_sector
                                    })
                                    .collect::<Vec<_>>()
                            } else {
                                Vec::new()
                            };
                            (subject, peers, subject_sector)
                        } else {
                            (None, Vec::new(), String::new())
                        }
                    } else {
                        (None, Vec::new(), String::new())
                    };
                let peer_refs: Vec<&research::CorrStkSnapshot> = peers.iter().collect();
                let snap = research::compute_corrrank_snapshot(
                    &symbol,
                    &today,
                    &sector,
                    subject.as_ref(),
                    &peer_refs,
                );
                let _ = msg_tx.send(BrokerMsg::CorrrankSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeOperankDeltaSnapshot { symbol } => {
            use typhoon_engine::core::fundamentals;
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let (subject, peers, sector) = if let Some(cache) =
                    shared_cache_broker.read().ok().and_then(|g| g.clone())
                {
                    if let Ok(conn) = cache.connection() {
                        let subj = research::get_margins(&conn, &symbol).ok().flatten();
                        let fund = fundamentals::get_fundamentals(&conn, &symbol)
                            .ok()
                            .flatten();
                        let sector = fund.as_ref().map(|f| f.sector.clone()).unwrap_or_default();
                        let mut peers: Vec<research::MarginsSnapshot> = Vec::new();
                        if !sector.is_empty() {
                            let all = research::get_all_margins(&conn).unwrap_or_default();
                            for p in all {
                                if p.symbol.eq_ignore_ascii_case(&symbol) {
                                    continue;
                                }
                                if let Ok(Some(pf)) =
                                    fundamentals::get_fundamentals(&conn, &p.symbol)
                                {
                                    if pf.sector == sector {
                                        peers.push(p);
                                    }
                                }
                            }
                        }
                        (subj, peers, sector)
                    } else {
                        (None, Vec::new(), String::new())
                    }
                } else {
                    (None, Vec::new(), String::new())
                };
                let peer_refs: Vec<&research::MarginsSnapshot> = peers.iter().collect();
                let snap = research::compute_operank_delta_snapshot(
                    &symbol,
                    &today,
                    &sector,
                    subject.as_ref(),
                    &peer_refs,
                );
                let _ = msg_tx.send(BrokerMsg::OperankDeltaSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeDivaccSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let dividends =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            research::get_dividends(&conn, &symbol)
                                .ok()
                                .flatten()
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };
                let snap = research::compute_divacc_snapshot(&symbol, &today, &dividends);
                let _ = msg_tx.send(BrokerMsg::DivaccSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeEpsaccSnapshot { symbol } => {
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
                let snap = research::compute_epsacc_snapshot(&symbol, &today, &statements);
                let _ = msg_tx.send(BrokerMsg::EpsaccSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeVrpSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let (ivol, rvcone) =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            (
                                research::get_ivol(&conn, &symbol).ok().flatten(),
                                research::get_rvcone(&conn, &symbol).ok().flatten(),
                            )
                        } else {
                            (None, None)
                        }
                    } else {
                        (None, None)
                    };
                let snap =
                    research::compute_vrp_snapshot(&symbol, &today, ivol.as_ref(), rvcone.as_ref());
                let _ = msg_tx.send(BrokerMsg::VrpSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeRetskewSnapshot { symbol } => {
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
                let snap = research::compute_retskew_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::RetskewSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeRetkurtSnapshot { symbol } => {
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
                let snap = research::compute_retkurt_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::RetkurtSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeTailrSnapshot { symbol } => {
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
                let snap = research::compute_tailr_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::TailrSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeRunlenSnapshot { symbol } => {
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
                let snap = research::compute_runlen_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::RunlenSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeDayrangeSnapshot { symbol } => {
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
                let snap = research::compute_dayrange_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::DayrangeSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 23 handlers ──
        BrokerCmd::ComputeAutocorSnapshot { symbol } => {
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
                let snap = research::compute_autocor_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::AutocorSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeHurstSnapshot { symbol } => {
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
                let snap = research::compute_hurst_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::HurstSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeHitrateSnapshot { symbol } => {
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
                let snap = research::compute_hitrate_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::HitrateSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeGlasymSnapshot { symbol } => {
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
                let snap = research::compute_glasym_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::GlasymSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeVolratioSnapshot { symbol } => {
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
                let snap = research::compute_volratio_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::VolratioSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 24 handlers ──
        BrokerCmd::ComputeDrawupSnapshot { symbol } => {
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
                let snap = research::compute_drawup_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::DrawupSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeGapstatsSnapshot { symbol } => {
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
                let snap = research::compute_gapstats_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::GapstatsSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeVolclusterSnapshot { symbol } => {
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
                let snap = research::compute_volcluster_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::VolclusterSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCloseplcSnapshot { symbol } => {
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
                let snap = research::compute_closeplc_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::CloseplcSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeMrhlSnapshot { symbol } => {
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
                let snap = research::compute_mrhl_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::MrhlSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 25 handlers ──
        BrokerCmd::ComputeDownvolSnapshot { symbol } => {
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
                let snap = research::compute_downvol_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::DownvolSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeSharprSnapshot { symbol } => {
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
                let snap = research::compute_sharpr_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::SharprSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeEffratioSnapshot { symbol } => {
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
                let snap = research::compute_effratio_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::EffratioSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeWickbiasSnapshot { symbol } => {
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
                let snap = research::compute_wickbias_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::WickbiasSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeVolofvolSnapshot { symbol } => {
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
                let snap = research::compute_volofvol_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::VolofvolSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 26 handlers ──
        BrokerCmd::ComputeCalmarSnapshot { symbol } => {
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
                let snap = research::compute_calmar_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::CalmarSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeUlcerSnapshot { symbol } => {
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
                let snap = research::compute_ulcer_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::UlcerSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeVarratioSnapshot { symbol } => {
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
                let snap = research::compute_varratio_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::VarratioSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeAmihudSnapshot { symbol } => {
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
                let snap = research::compute_amihud_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::AmihudSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeJbnormSnapshot { symbol } => {
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
                let snap = research::compute_jbnorm_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::JbnormSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 27 handlers ──
        BrokerCmd::ComputeOmegaSnapshot { symbol } => {
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
                let snap = research::compute_omega_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::OmegaSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeDfaSnapshot { symbol } => {
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
                let snap = research::compute_dfa_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::DfaSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeBurkeSnapshot { symbol } => {
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
                let snap = research::compute_burke_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::BurkeSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeMonthseasSnapshot { symbol } => {
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
                let snap = research::compute_monthseas_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::MonthseasSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeRollsprdSnapshot { symbol } => {
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
                let snap = research::compute_rollsprd_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::RollsprdSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 28 handlers ──
        BrokerCmd::ComputeParkinsonSnapshot { symbol } => {
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
                let snap = research::compute_parkinson_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::ParkinsonSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeGkvolSnapshot { symbol } => {
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
                let snap = research::compute_gkvol_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::GkvolSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeRsvolSnapshot { symbol } => {
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
                let snap = research::compute_rsvol_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::RsvolSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCvarSnapshot { symbol } => {
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
                let snap = research::compute_cvar_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::CvarSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeDoweffectSnapshot { symbol } => {
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
                let snap = research::compute_doweffect_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::DoweffectSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 29 handlers ──
        BrokerCmd::ComputeSterlingSnapshot { symbol } => {
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
                let snap = research::compute_sterling_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::SterlingSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeKellyfSnapshot { symbol } => {
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
                let snap = research::compute_kellyf_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::KellyfSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeLjungbSnapshot { symbol } => {
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
                let snap = research::compute_ljungb_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::LjungbSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeRunstestSnapshot { symbol } => {
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
                let snap = research::compute_runstest_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::RunstestSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeZeroretSnapshot { symbol } => {
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
                let snap = research::compute_zeroret_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::ZeroretSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 30 handlers ──
        BrokerCmd::ComputePsrSnapshot { symbol } => {
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
                let snap = research::compute_psr_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::PsrSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeAdfSnapshot { symbol } => {
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
                let snap = research::compute_adf_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::AdfSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeMnkendallSnapshot { symbol } => {
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
                let snap = research::compute_mnkendall_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::MnkendallSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeBipowerSnapshot { symbol } => {
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
                let snap = research::compute_bipower_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::BipowerSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeDddurSnapshot { symbol } => {
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
                let snap = research::compute_dddur_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::DddurSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 31 handlers ──
        BrokerCmd::ComputeHilltailSnapshot { symbol } => {
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
                let snap = research::compute_hilltail_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::HilltailSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeArchlmSnapshot { symbol } => {
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
                let snap = research::compute_archlm_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::ArchlmSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputePainratioSnapshot { symbol } => {
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
                let snap = research::compute_painratio_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::PainratioSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCusumSnapshot { symbol } => {
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
                let snap = research::compute_cusum_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::CusumSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCfvarSnapshot { symbol } => {
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
                let snap = research::compute_cfvar_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::CfvarSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 32 handlers ──
        BrokerCmd::ComputeEntropySnapshot { symbol } => {
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
                let snap = research::compute_entropy_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::EntropySnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeRachevSnapshot { symbol } => {
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
                let snap = research::compute_rachev_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::RachevSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeGprSnapshot { symbol } => {
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
                let snap = research::compute_gpr_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::GprSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputePacfSnapshot { symbol } => {
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
                let snap = research::compute_pacf_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::PacfSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeApenSnapshot { symbol } => {
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
                let snap = research::compute_apen_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::ApenSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 33 handlers ──
        BrokerCmd::ComputeUprSnapshot { symbol } => {
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
                let snap = research::compute_upr_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::UprSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeLevereffSnapshot { symbol } => {
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
                let snap = research::compute_levereff_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::LevereffSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeDrawdarSnapshot { symbol } => {
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
                let snap = research::compute_drawdar_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::DrawdarSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeVarhalfSnapshot { symbol } => {
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
                let snap = research::compute_varhalf_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::VarhalfSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeGiniSnapshot { symbol } => {
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
                let snap = research::compute_gini_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::GiniSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 34 handlers ──
        BrokerCmd::ComputeSampenSnapshot { symbol } => {
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
                let snap = research::compute_sampen_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::SampenSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputePermenSnapshot { symbol } => {
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
                let snap = research::compute_permen_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::PermenSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeRecfactSnapshot { symbol } => {
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
                let snap = research::compute_recfact_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::RecfactSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeKpssSnapshot { symbol } => {
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
                let snap = research::compute_kpss_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::KpssSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeSpecentSnapshot { symbol } => {
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
                let snap = research::compute_specent_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::SpecentSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 35 handlers ──
        BrokerCmd::ComputeRobvolSnapshot { symbol } => {
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
                let snap = research::compute_robvol_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::RobvolSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeRenyientSnapshot { symbol } => {
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
                let snap = research::compute_renyient_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::RenyientSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeRetquantSnapshot { symbol } => {
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
                let snap = research::compute_retquant_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::RetquantSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeMsentSnapshot { symbol } => {
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
                let snap = research::compute_msent_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::MsentSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeEwmavolSnapshot { symbol } => {
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
                let snap = research::compute_ewmavol_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::EwmavolSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 36 handlers ──
        BrokerCmd::ComputeKsnormSnapshot { symbol } => {
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
                let snap = research::compute_ksnorm_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::KsnormSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeAdtestSnapshot { symbol } => {
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
                let snap = research::compute_adtest_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::AdtestSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeLmomSnapshot { symbol } => {
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
                let snap = research::compute_lmom_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::LmomSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeKylelamSnapshot { symbol } => {
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
                let snap = research::compute_kylelam_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::KylelamSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputePeakoverSnapshot { symbol } => {
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
                let snap = research::compute_peakover_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::PeakoverSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 37 handlers ──
        BrokerCmd::ComputeHiguchiSnapshot { symbol } => {
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
                let snap = research::compute_higuchi_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::HiguchiSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputePickandsSnapshot { symbol } => {
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
                let snap = research::compute_pickands_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::PickandsSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeKappa3Snapshot { symbol } => {
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
                let snap = research::compute_kappa3_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::Kappa3SnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeLyapunovSnapshot { symbol } => {
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
                let snap = research::compute_lyapunov_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::LyapunovSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeRankacSnapshot { symbol } => {
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
                let snap = research::compute_rankac_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::RankacSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 38 handlers ──
        BrokerCmd::ComputeBnsjumpSnapshot { symbol } => {
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
                let snap = research::compute_bnsjump_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::BnsjumpSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputePprootSnapshot { symbol } => {
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
                let snap = research::compute_pproot_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::PprootSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeMfdfaSnapshot { symbol } => {
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
                let snap = research::compute_mfdfa_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::MfdfaSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeHillksSnapshot { symbol } => {
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
                let snap = research::compute_hillks_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::HillksSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeTsiSnapshot { symbol } => {
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
                let snap = research::compute_tsi_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::TsiSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 39 handlers ──
        BrokerCmd::ComputeGarch11Snapshot { symbol } => {
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
                let snap = research::compute_garch11_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::Garch11SnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeSadfSnapshot { symbol } => {
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
                let snap = research::compute_sadf_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::SadfSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCordimSnapshot { symbol } => {
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
                let snap = research::compute_cordim_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::CordimSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeSkspecSnapshot { symbol } => {
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
                let snap = research::compute_skspec_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::SkspecSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeAutomiSnapshot { symbol } => {
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
                let snap = research::compute_automi_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::AutomiSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 40 handlers ──
        BrokerCmd::ComputeDurbinWatsonSnapshot { symbol } => {
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
                let snap = research::compute_durbinwatson_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::DurbinWatsonSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeBdsTestSnapshot { symbol } => {
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
                let snap = research::compute_bdstest_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::BdsTestSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeBreuschPaganSnapshot { symbol } => {
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
                let snap = research::compute_breuschpagan_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::BreuschPaganSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeTurnPtsSnapshot { symbol } => {
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
                let snap = research::compute_turnpts_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::TurnPtsSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputePeriodogramSnapshot { symbol } => {
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
                let snap = research::compute_periodogram_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::PeriodogramSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeMcLeodLiSnapshot { symbol } => {
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
                let snap = research::compute_mcleodli_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::McLeodLiSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeOuFitSnapshot { symbol } => {
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
                let snap = research::compute_oufit_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::OuFitSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeGphSnapshot { symbol } => {
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
                let snap = research::compute_gph_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::GphSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeBurgSpecSnapshot { symbol } => {
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
                let snap = research::compute_burgspec_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::BurgSpecSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeKendallTauSnapshot { symbol } => {
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
                let snap = research::compute_kendalltau_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::KendallTauSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 42 handlers ──
        cmd @ (BrokerCmd::ComputeSqueezeSnapshot { .. }
        | BrokerCmd::ComputeSqueezeRankSnapshot { .. }
        | BrokerCmd::RefreshSqueezeWatchlist
        | BrokerCmd::ComputeBbsqueezeSnapshot { .. }
        | BrokerCmd::ComputeDonchianSnapshot { .. }
        | BrokerCmd::ComputeKamaSnapshot { .. }
        | BrokerCmd::ComputeIchimokuSnapshot { .. }
        | BrokerCmd::ComputeSupertrendSnapshot { .. }
        | BrokerCmd::ComputeKeltnerSnapshot { .. }
        | BrokerCmd::ComputeFisherSnapshot { .. }
        | BrokerCmd::ComputeAroonSnapshot { .. }
        | BrokerCmd::ComputeAdxSnapshot { .. }
        | BrokerCmd::ComputeCciSnapshot { .. }
        | BrokerCmd::ComputeCmfSnapshot { .. }
        | BrokerCmd::ComputeMfiSnapshot { .. }
        | BrokerCmd::ComputePsarSnapshot { .. }
        | BrokerCmd::ComputeVortexSnapshot { .. }
        | BrokerCmd::ComputeChopSnapshot { .. }
        | BrokerCmd::ComputeObvSnapshot { .. }
        | BrokerCmd::ComputeTrixSnapshot { .. }
        | BrokerCmd::ComputeHmaSnapshot { .. }
        | BrokerCmd::ComputePpoSnapshot { .. }
        | BrokerCmd::ComputeDpoSnapshot { .. }
        | BrokerCmd::ComputeKstSnapshot { .. }
        | BrokerCmd::ComputeUltoscSnapshot { .. }
        | BrokerCmd::ComputeWillrSnapshot { .. }
        | BrokerCmd::ComputeMassSnapshot { .. }
        | BrokerCmd::ComputeChaikoscSnapshot { .. }
        | BrokerCmd::ComputeKlingerSnapshot { .. }
        | BrokerCmd::ComputeStochRsiSnapshot { .. }
        | BrokerCmd::ComputeAwesomeSnapshot { .. }
        | BrokerCmd::ComputeEfiSnapshot { .. }
        | BrokerCmd::ComputeEmvSnapshot { .. }
        | BrokerCmd::ComputeNviSnapshot { .. }
        | BrokerCmd::ComputePviSnapshot { .. }
        | BrokerCmd::ComputeCoppockSnapshot { .. }
        | BrokerCmd::ComputeCmoSnapshot { .. }
        | BrokerCmd::ComputeQstickSnapshot { .. }
        | BrokerCmd::ComputeDisparitySnapshot { .. }
        | BrokerCmd::ComputeBopSnapshot { .. }
        | BrokerCmd::ComputeSchaffSnapshot { .. }
        | BrokerCmd::ComputeStochSnapshot { .. }
        | BrokerCmd::ComputeMacdSnapshot { .. }
        | BrokerCmd::ComputeVwapSnapshot { .. }
        | BrokerCmd::ComputeMcgdSnapshot { .. }
        | BrokerCmd::ComputeRwiSnapshot { .. }
        | BrokerCmd::ComputeDemaSnapshot { .. }
        | BrokerCmd::ComputeTemaSnapshot { .. }
        | BrokerCmd::ComputeLinregSnapshot { .. }
        | BrokerCmd::ComputePivotsSnapshot { .. }
        | BrokerCmd::ComputeHeikinSnapshot { .. }
        | BrokerCmd::ComputeAlmaSnapshot { .. }
        | BrokerCmd::ComputeZlemaSnapshot { .. }
        | BrokerCmd::ComputeElderRaySnapshot { .. }
        | BrokerCmd::ComputeTsfSnapshot { .. }
        | BrokerCmd::ComputeRviSnapshot { .. }
        | BrokerCmd::ComputeTrimaSnapshot { .. }
        | BrokerCmd::ComputeT3Snapshot { .. }
        | BrokerCmd::ComputeVidyaSnapshot { .. }
        | BrokerCmd::ComputeSmiSnapshot { .. }
        | BrokerCmd::ComputePvtSnapshot { .. }
        | BrokerCmd::ComputeAcSnapshot { .. }
        | BrokerCmd::ComputeChvolSnapshot { .. }
        | BrokerCmd::ComputeBbwidthSnapshot { .. }
        | BrokerCmd::ComputeElderImpSnapshot { .. }
        | BrokerCmd::ComputeRmiSnapshot { .. }
        | BrokerCmd::ComputeSymbolExpirations { .. }
        | BrokerCmd::ComputeSmmaSnapshot { .. }
        | BrokerCmd::ComputeAlligatorSnapshot { .. }
        | BrokerCmd::ComputeCrsiSnapshot { .. }
        | BrokerCmd::ComputeSebSnapshot { .. }
        | BrokerCmd::ComputeImiSnapshot { .. }
        | BrokerCmd::ComputeGmmaSnapshot { .. }
        | BrokerCmd::ComputeMaenvSnapshot { .. }
        | BrokerCmd::ComputeAdlSnapshot { .. }
        | BrokerCmd::ComputeVhfSnapshot { .. }
        | BrokerCmd::ComputeVrocSnapshot { .. }
        | BrokerCmd::ComputeKdjSnapshot { .. }
        | BrokerCmd::ComputeQqeSnapshot { .. }
        | BrokerCmd::ComputePmoSnapshot { .. }
        | BrokerCmd::ComputeCfoSnapshot { .. }
        | BrokerCmd::ComputeTmfSnapshot { .. }
        | BrokerCmd::ComputeFractalsSnapshot { .. }
        | BrokerCmd::ComputeIftRsiSnapshot { .. }
        | BrokerCmd::ComputeMamaSnapshot { .. }
        | BrokerCmd::ComputeCogSnapshot { .. }
        | BrokerCmd::ComputeDidiSnapshot { .. }
        | BrokerCmd::ComputeDemarkerSnapshot { .. }
        | BrokerCmd::ComputeGatorSnapshot { .. }
        | BrokerCmd::ComputeBwMfiSnapshot { .. }
        | BrokerCmd::ComputeVwmaSnapshot { .. }
        | BrokerCmd::ComputeStddevSnapshot { .. }
        | BrokerCmd::ComputeWmaSnapshot { .. }
        | BrokerCmd::ComputeRainbowSnapshot { .. }
        | BrokerCmd::ComputeMesaSineSnapshot { .. }
        | BrokerCmd::ComputeFramaSnapshot { .. }
        | BrokerCmd::ComputeIbsSnapshot { .. }
        | BrokerCmd::ComputeLaguerreRsiSnapshot { .. }
        | BrokerCmd::ComputeZigzagSnapshot { .. }
        | BrokerCmd::ComputePgoSnapshot { .. }
        | BrokerCmd::ComputeHtTrendlineSnapshot { .. }
        | BrokerCmd::ComputeMidpointSnapshot { .. }
        | BrokerCmd::ComputeMassIndexSnapshot { .. }
        | BrokerCmd::ComputeNatrSnapshot { .. }
        | BrokerCmd::ComputeTtmSqueezeSnapshot { .. }
        | BrokerCmd::ComputeForceIndexSnapshot { .. }
        | BrokerCmd::ComputeTrangeSnapshot { .. }
        | BrokerCmd::ComputeLinearregSlopeSnapshot { .. }
        | BrokerCmd::ComputeHtDcperiodSnapshot { .. }
        | BrokerCmd::ComputeHtTrendmodeSnapshot { .. }
        | BrokerCmd::ComputeAccbandsSnapshot { .. }
        | BrokerCmd::ComputeStochfSnapshot { .. }
        | BrokerCmd::ComputeLinearregSnapshot { .. }
        | BrokerCmd::ComputeLinearregAngleSnapshot { .. }
        | BrokerCmd::ComputeHtDcphaseSnapshot { .. }
        | BrokerCmd::ComputeHtSineSnapshot { .. }
        | BrokerCmd::ComputeHtPhasorSnapshot { .. }
        | BrokerCmd::ComputeMidpriceSnapshot { .. }
        | BrokerCmd::ComputeApoSnapshot { .. }
        | BrokerCmd::ComputeMomSnapshot { .. }
        | BrokerCmd::ComputeSarextSnapshot { .. }
        | BrokerCmd::ComputeAdxrSnapshot { .. }
        | BrokerCmd::ComputeAvgpriceSnapshot { .. }
        | BrokerCmd::ComputeMedpriceSnapshot { .. }
        | BrokerCmd::ComputeTypPriceSnapshot { .. }
        | BrokerCmd::ComputeWclPriceSnapshot { .. }
        | BrokerCmd::ComputeVarianceSnapshot { .. }
        | BrokerCmd::ComputePlusDiSnapshot { .. }
        | BrokerCmd::ComputeMinusDiSnapshot { .. }
        | BrokerCmd::ComputePlusDmSnapshot { .. }
        | BrokerCmd::ComputeMinusDmSnapshot { .. }
        | BrokerCmd::ComputeDxSnapshot { .. }
        | BrokerCmd::ComputeRocSnapshot { .. }
        | BrokerCmd::ComputeRocpSnapshot { .. }
        | BrokerCmd::ComputeRocrSnapshot { .. }
        | BrokerCmd::ComputeRocr100Snapshot { .. }
        | BrokerCmd::ComputeCorrelSnapshot { .. }
        | BrokerCmd::ComputeMinSnapshot { .. }
        | BrokerCmd::ComputeMaxSnapshot { .. }
        | BrokerCmd::ComputeMinMaxSnapshot { .. }
        | BrokerCmd::ComputeMinIndexSnapshot { .. }
        | BrokerCmd::ComputeMaxIndexSnapshot { .. }
        | BrokerCmd::ComputeBbandsSnapshot { .. }
        | BrokerCmd::ComputeAdSnapshot { .. }
        | BrokerCmd::ComputeAdoscSnapshot { .. }
        | BrokerCmd::ComputeSumSnapshot { .. }
        | BrokerCmd::ComputeLinearRegInterceptSnapshot { .. }
        | BrokerCmd::ComputeAroonoscSnapshot { .. }
        | BrokerCmd::ComputeMinMaxIndexSnapshot { .. }
        | BrokerCmd::ComputeMacdextSnapshot { .. }
        | BrokerCmd::ComputeMacdfixSnapshot { .. }
        | BrokerCmd::ComputeMavpSnapshot { .. }
        | BrokerCmd::ComputeCdlDojiSnapshot { .. }
        | BrokerCmd::ComputeCdlHammerSnapshot { .. }
        | BrokerCmd::ComputeCdlShootingStarSnapshot { .. }
        | BrokerCmd::ComputeCdlEngulfingSnapshot { .. }
        | BrokerCmd::ComputeCdlHaramiSnapshot { .. }
        | BrokerCmd::ComputeCdlMorningStarSnapshot { .. }
        | BrokerCmd::ComputeCdlEveningStarSnapshot { .. }
        | BrokerCmd::ComputeCdlThreeBlackCrowsSnapshot { .. }
        | BrokerCmd::ComputeCdlThreeWhiteSoldiersSnapshot { .. }
        | BrokerCmd::ComputeCdlDarkCloudCoverSnapshot { .. }
        | BrokerCmd::ComputeCdlPiercingSnapshot { .. }
        | BrokerCmd::ComputeCdlDragonflyDojiSnapshot { .. }
        | BrokerCmd::ComputeCdlGravestoneDojiSnapshot { .. }
        | BrokerCmd::ComputeCdlHangingManSnapshot { .. }
        | BrokerCmd::ComputeCdlInvertedHammerSnapshot { .. }
        | BrokerCmd::ComputeCdlHaramiCrossSnapshot { .. }
        | BrokerCmd::ComputeCdlLongLeggedDojiSnapshot { .. }
        | BrokerCmd::ComputeCdlMarubozuSnapshot { .. }
        | BrokerCmd::ComputeCdlSpinningTopSnapshot { .. }
        | BrokerCmd::ComputeCdlTristarSnapshot { .. }
        | BrokerCmd::ComputeCdlDojiStarSnapshot { .. }
        | BrokerCmd::ComputeCdlMorningDojiStarSnapshot { .. }
        | BrokerCmd::ComputeCdlEveningDojiStarSnapshot { .. }
        | BrokerCmd::ComputeCdlAbandonedBabySnapshot { .. }
        | BrokerCmd::ComputeCdlThreeInsideSnapshot { .. }
        | BrokerCmd::ComputeCdlBeltHoldSnapshot { .. }
        | BrokerCmd::ComputeCdlClosingMarubozuSnapshot { .. }
        | BrokerCmd::ComputeCdlHighWaveSnapshot { .. }
        | BrokerCmd::ComputeCdlLongLineSnapshot { .. }
        | BrokerCmd::ComputeCdlShortLineSnapshot { .. }
        | BrokerCmd::ComputeCdlCounterattackSnapshot { .. }
        | BrokerCmd::ComputeCdlHomingPigeonSnapshot { .. }
        | BrokerCmd::ComputeCdlInNeckSnapshot { .. }
        | BrokerCmd::ComputeCdlOnNeckSnapshot { .. }
        | BrokerCmd::ComputeCdlThrustingSnapshot { .. }
        | BrokerCmd::ComputeCdlTwoCrowsSnapshot { .. }
        | BrokerCmd::ComputeCdlThreeLineStrikeSnapshot { .. }
        | BrokerCmd::ComputeCdlThreeOutsideSnapshot { .. }
        | BrokerCmd::ComputeCdlMatchingLowSnapshot { .. }
        | BrokerCmd::ComputeCdlSeparatingLinesSnapshot { .. }
        | BrokerCmd::ComputeCdlStickSandwichSnapshot { .. }
        | BrokerCmd::ComputeCdlRickshawManSnapshot { .. }
        | BrokerCmd::ComputeCdlTakuriSnapshot { .. }
        | BrokerCmd::ComputeCdlThreeStarsInSouthSnapshot { .. }
        | BrokerCmd::ComputeCdlIdenticalThreeCrowsSnapshot { .. }
        | BrokerCmd::ComputeCdlKickingSnapshot { .. }
        | BrokerCmd::ComputeCdlKickingByLengthSnapshot { .. }
        | BrokerCmd::ComputeCdlLadderBottomSnapshot { .. }
        | BrokerCmd::ComputeCdlUniqueThreeRiverSnapshot { .. }
        | BrokerCmd::ComputeCdlAdvanceBlockSnapshot { .. }
        | BrokerCmd::ComputeCdlBreakawaySnapshot { .. }
        | BrokerCmd::ComputeCdlGapSideSideWhiteSnapshot { .. }
        | BrokerCmd::ComputeCdlUpsideGapTwoCrowsSnapshot { .. }
        | BrokerCmd::ComputeCdlXSideGapThreeMethodsSnapshot { .. }
        | BrokerCmd::ComputeCdlConcealBabySwallowSnapshot { .. }
        | BrokerCmd::ComputeCdlHikkakeSnapshot { .. }
        | BrokerCmd::ComputeCdlHikkakeModSnapshot { .. }
        | BrokerCmd::ComputeCdlMatHoldSnapshot { .. }
        | BrokerCmd::ComputeCdlRiseFallThreeMethodsSnapshot { .. }
        | BrokerCmd::ComputeCdlStalledPatternSnapshot { .. }
        | BrokerCmd::ComputeCdlTasukiGapSnapshot { .. }) => {
            technical_indicators::handle_technical_indicator_command(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        BrokerCmd::ComputeModSharpeSnapshot { symbol } => {
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
                let snap = research::compute_modsharpe_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_modsharpe(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::ModSharpeSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeHsiehTestSnapshot { symbol } => {
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
                let snap = research::compute_hsieh_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_hsiehtest(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::HsiehTestSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeChowBreakSnapshot { symbol } => {
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
                let snap = research::compute_chowbreak_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_chowbreak(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::ChowBreakSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeDriftBurstSnapshot { symbol } => {
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
                let snap = research::compute_driftburst_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_driftburst(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::DriftBurstSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeHlvClustSnapshot { symbol } => {
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
                let snap = research::compute_hlvclust_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_hlvclust(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::HlvClustSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 77 (Quant Stats) handlers ──
        BrokerCmd::ComputeYangZhangSnapshot { symbol } => {
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
                let snap = research::compute_yangzhang_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_yangzhang(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::YangZhangSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeKuiperSnapshot { symbol } => {
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
                let snap = research::compute_kuiper_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_kuiper(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::KuiperSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeDagostinoSnapshot { symbol } => {
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
                let snap = research::compute_dagostino_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_dagostino(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::DagostinoSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeBaiPerronSnapshot { symbol } => {
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
                let snap = research::compute_baiperron_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_baiperron(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::BaiPerronSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeKupiecPofSnapshot { symbol } => {
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
                let snap = research::compute_kupiecpof_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_kupiecpof(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::KupiecPofSnapshotMsg(symbol, snap));
            });
        }
        _ => { /* not risk */ }
    }
}
