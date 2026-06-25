use super::prelude::*;

pub(super) fn handle_factor_rank_core_compute(
    cmd: BrokerCmd,
    broker_msg_tx_clone: tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    shared_cache_broker: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
) {
    match cmd {
        // Size, momentum, PEAD, quality, reversal, leverage, operating, and liquidity rank research
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
        _ => {}
    }
}
