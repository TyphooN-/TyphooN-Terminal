use super::*;

pub(super) fn handle_price_rank_risk_overlay_compute(
    cmd: BrokerCmd,
    broker_msg_tx_clone: tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    shared_cache_broker: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
) {
    match cmd {
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
                                if p.symbol.eq_ignore_ascii_case(&symbol) {
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
                                if p.symbol.eq_ignore_ascii_case(&symbol) {
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
        _ => {}
    }
}
