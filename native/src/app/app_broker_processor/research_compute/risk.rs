use super::*;

mod coverage_relative_event;
mod factor_rank_core;
mod fundamental_risk;
mod growth_flow_regime;
mod valuation_quality_risk;
mod insider_dividend_momentum;
mod market_liquidity_credit;
mod solvency_quality;

pub(super) fn handle_risk_compute(
    cmd: BrokerCmd,
    broker_msg_tx_clone: tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    shared_cache_broker: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
) {
    match cmd {
        // Leverage, accruals, realized-volatility, cash-flow, and short-interest research
        cmd @ (BrokerCmd::ComputeLeverageSnapshot { .. }
            | BrokerCmd::ComputeAccrualsSnapshot { .. }
            | BrokerCmd::ComputeRealizedVolSnapshot { .. }
            | BrokerCmd::ComputeFcfYieldSnapshot { .. }
            | BrokerCmd::ComputeShortInterestSnapshot { .. }) => {
            fundamental_risk::handle_fundamental_risk_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        // Solvency, quality, volatility-estimator, EPS-beat, and price-target research
        cmd @ (BrokerCmd::ComputeAltmanZSnapshot { .. }
            | BrokerCmd::ComputePiotroskiSnapshot { .. }
            | BrokerCmd::ComputeOhlcVolSnapshot { .. }
            | BrokerCmd::ComputeEpsBeatSnapshot { .. }
            | BrokerCmd::ComputePriceTargetDispersionSnapshot { .. }) => {
            solvency_quality::handle_solvency_quality_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        // Insider, dividend-growth, earnings-revision, sector-rotation, and upgrade/downgrade research
        cmd @ (BrokerCmd::ComputeInsiderActivitySnapshot { .. }
            | BrokerCmd::ComputeDivgSnapshot { .. }
            | BrokerCmd::ComputeEarmSnapshot { .. }
            | BrokerCmd::ComputeSectorRotationSnapshot { .. }
            | BrokerCmd::ComputeUpdmSnapshot { .. }) => {
            insider_dividend_momentum::handle_insider_dividend_momentum_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        // Momentum, liquidity, breakout, cash-cycle, and credit research
        cmd @ (BrokerCmd::ComputeMomentumSnapshot { .. }
            | BrokerCmd::ComputeLiquiditySnapshot { .. }
            | BrokerCmd::ComputeBreakoutSnapshot { .. }
            | BrokerCmd::ComputeCashCycleSnapshot { .. }
            | BrokerCmd::ComputeCreditSnapshot { .. }) => {
            market_liquidity_credit::handle_market_liquidity_credit_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        // Growth-momentum, ownership-flow, and market-regime research
        cmd @ (BrokerCmd::ComputeGrowmSnapshot { .. }
            | BrokerCmd::ComputeFlowSnapshot { .. }
            | BrokerCmd::ComputeRegimeSnapshot { .. }) => {
            growth_flow_regime::handle_growth_flow_regime_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        // Relative-volume, margin, valuation, quality, and composite-risk research
        cmd @ (BrokerCmd::ComputeRelvolSnapshot { .. }
            | BrokerCmd::ComputeMarginsSnapshot { .. }
            | BrokerCmd::ComputeValSnapshot { .. }
            | BrokerCmd::ComputeQualSnapshot { .. }
            | BrokerCmd::ComputeRiskSnapshot { .. }) => {
            valuation_quality_risk::handle_valuation_quality_risk_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        // Insider-streak, analyst-coverage, relative rank, earnings-growth, and PEAD research
        cmd @ (BrokerCmd::ComputeInsstrkSnapshot { .. }
            | BrokerCmd::ComputeCovgSnapshot { .. }
            | BrokerCmd::ComputeVrkSnapshot { .. }
            | BrokerCmd::ComputeQrkSnapshot { .. }
            | BrokerCmd::ComputeRrkSnapshot { .. }
            | BrokerCmd::ComputeRelepsgrSnapshot { .. }
            | BrokerCmd::ComputePeadSnapshot { .. }) => {
            coverage_relative_event::handle_coverage_relative_event_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        // Size, momentum, PEAD, quality, reversal, leverage, operating, and liquidity rank research
        cmd @ (BrokerCmd::ComputeSizefSnapshot { .. }
            | BrokerCmd::ComputeMomfSnapshot { .. }
            | BrokerCmd::ComputePeadrankSnapshot { .. }
            | BrokerCmd::ComputeFqmSnapshot { .. }
            | BrokerCmd::ComputeRevrankSnapshot { .. }
            | BrokerCmd::ComputeLevrankSnapshot { .. }
            | BrokerCmd::ComputeOperankSnapshot { .. }
            | BrokerCmd::ComputeFqmrankSnapshot { .. }
            | BrokerCmd::ComputeLiqrankSnapshot { .. }) => {
            factor_rank_core::handle_factor_rank_core_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
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
        // Autocorrelation, Hurst, hit-rate, asymmetry, and volatility-ratio research
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
        // Draw-up, gap-statistics, volatility-cluster, close-position, and range-location research
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
        // Downside-volatility, Sharpe, efficiency, wick-bias, and volatility-of-volatility research
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
        // Calmar, ulcer, variance-ratio, Amihud, and normality-test research
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
        // Omega, DFA, Burke, monthly-seasonality, and roll-spread research
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
        // Parkinson, Garman-Klass, Rogers-Satchell, CVaR, and day-of-week research
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
        // Sterling, Kelly, Ljung-Box, runs-test, and zero-return research
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
        // PSR, ADF, Mann-Kendall, bipower, and drawdown-duration research
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
        // Hill-tail, ARCH-LM, pain-ratio, CUSUM, and Cornish-Fisher VaR research
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
        // Entropy, Rachev, gain-pain, PACF, and approximate-entropy research
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
        // Upside-potential, leverage-effect, drawdown-at-risk, VaR-half-life, and Gini research
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
        // Sample-entropy, permutation-entropy, recurrence-factor, KPSS, and spectral-entropy research
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
        // Robust-volatility, Renyi-entropy, return-quantile, market-sentiment, and EWMA-volatility research
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
        // KS-normality, Anderson-Darling, L-moment, Kyle-lambda, and peak-over-threshold research
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
        // Higuchi, Pickands, kappa, Lyapunov, and rank-autocorrelation research
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
        // Jump-test, Phillips-Perron, MF-DFA, Hill-KS, and trend-strength research
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
        // GARCH, SADF, correlation-dimension, spectral-skew, and automutual-information research
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
        // RPDE, Hurst-cycle, IAAFT, bid-ask bounce, break-even volatility, and related quant research
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
        // Structural-break, ICSS, Hurst-rolling, tail-dependence, volatility-spillover, and correlation-stability research
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
        // Quant-statistics validation, break-test, and coverage-test research
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
