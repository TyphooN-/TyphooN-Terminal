use super::prelude::*;

pub(super) fn handle_coverage_relative_event_compute(
    cmd: BrokerCmd,
    broker_msg_tx_clone: tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    shared_cache_broker: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
) {
    match cmd {
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
        _ => {}
    }
}
