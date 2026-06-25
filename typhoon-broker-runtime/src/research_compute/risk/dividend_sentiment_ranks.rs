use super::prelude::*;

pub(super) fn handle_dividend_sentiment_rank_compute(
    cmd: BrokerCmd,
    broker_msg_tx_clone: tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    shared_cache_broker: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
) {
    match cmd {
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
                                if p.symbol.eq_ignore_ascii_case(&symbol) {
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
                                if p.symbol.eq_ignore_ascii_case(&symbol) {
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
                                if p.symbol.eq_ignore_ascii_case(&symbol) {
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
                                    if p.symbol.eq_ignore_ascii_case(&symbol) {
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
        _ => {}
    }
}
