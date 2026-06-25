use super::prelude::*;

pub(super) fn handle_valuation_compute(
    cmd: BrokerCmd,
    broker_msg_tx_clone: tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    shared_cache_broker: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
) {
    match cmd {
        BrokerCmd::ComputeDdmSnapshot {
            symbol,
            required_return_pct,
            return_source,
        } => {
            // Pure compute: read cached dividends on the broker thread, call the
            // compute function, emit a snapshot. Kept on an async task for uniformity
            // with other research handlers.
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
                let snap = research::compute_ddm_snapshot(
                    &symbol,
                    &today,
                    &divs,
                    required_return_pct,
                    &return_source,
                );
                let _ = msg_tx.send(BrokerMsg::DdmSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeRelativeValuation {
            symbol,
            sector,
            self_json,
            peers_json,
        } => {
            use typhoon_engine::core::fundamentals::Fundamentals;
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let self_fund: Fundamentals = serde_json::from_str(&self_json).unwrap_or_default();
                let peers: Vec<Fundamentals> =
                    serde_json::from_str(&peers_json).unwrap_or_default();
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let collect = |g: fn(&Fundamentals) -> Option<f64>| -> Vec<f64> {
                    peers.iter().filter_map(g).collect()
                };
                let inputs = vec![
                    research::RvMetricInput {
                        metric: "P/E",
                        value: self_fund.pe_ratio,
                        peer_values: collect(|f| f.pe_ratio),
                    },
                    research::RvMetricInput {
                        metric: "Fwd P/E",
                        value: self_fund.forward_pe,
                        peer_values: collect(|f| f.forward_pe),
                    },
                    research::RvMetricInput {
                        metric: "P/B",
                        value: self_fund.price_to_book,
                        peer_values: collect(|f| f.price_to_book),
                    },
                    research::RvMetricInput {
                        metric: "P/S",
                        value: self_fund.price_to_sales,
                        peer_values: collect(|f| f.price_to_sales),
                    },
                    research::RvMetricInput {
                        metric: "EV/EBITDA",
                        value: self_fund.ev_to_ebitda,
                        peer_values: collect(|f| f.ev_to_ebitda),
                    },
                    research::RvMetricInput {
                        metric: "Profit %",
                        value: self_fund.profit_margin,
                        peer_values: collect(|f| f.profit_margin),
                    },
                    research::RvMetricInput {
                        metric: "ROE",
                        value: self_fund.roe,
                        peer_values: collect(|f| f.roe),
                    },
                    research::RvMetricInput {
                        metric: "Beta",
                        value: self_fund.beta,
                        peer_values: collect(|f| f.beta),
                    },
                    research::RvMetricInput {
                        metric: "Div Yield",
                        value: self_fund.dividend_yield,
                        peer_values: collect(|f| f.dividend_yield),
                    },
                ];
                let rv = research::compute_relative_valuation(&symbol, &sector, &today, &inputs);
                let _ = msg_tx.send(BrokerMsg::RelativeValuationMsg(symbol, rv));
            });
        }
        BrokerCmd::FetchFigiIdentifiers { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::builder()
                    .user_agent("TyphooN-Terminal/1.0")
                    .timeout(std::time::Duration::from_secs(15))
                    .build()
                    .unwrap_or_default();
                match research::fetch_openfigi_identifiers(&client, &symbol).await {
                    Ok(ids) => {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        let snap = research::FigiSnapshot {
                            symbol: symbol.to_uppercase(),
                            as_of: today,
                            identifiers: ids,
                        };
                        let _ = msg_tx.send(BrokerMsg::FigiSnapshotMsg(symbol, snap));
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("FIGI: {e}")));
                    }
                }
            });
        }
        // ── handlers ──
        BrokerCmd::FetchHraSnapshot {
            symbol,
            risk_free_pct,
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
                // compute_hra_snapshot expects oldest-first; cache stores newest-first.
                if bars.len() >= 2 && bars[0].date > bars[bars.len() - 1].date {
                    bars.reverse();
                }
                let snap = research::compute_hra_snapshot(&symbol, &today, &bars, risk_free_pct);
                let _ = msg_tx.send(BrokerMsg::HraSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeDcfSnapshot {
            symbol,
            base_revenue,
            base_fcff,
            growth_pct,
            terminal_growth_pct,
            wacc_pct,
            tax_rate_pct,
            projection_years,
            total_debt,
            cash_and_equivalents,
            shares_outstanding,
        } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let snap = research::compute_dcf_snapshot(
                    &symbol,
                    &today,
                    base_revenue,
                    base_fcff,
                    growth_pct,
                    terminal_growth_pct,
                    wacc_pct,
                    tax_rate_pct,
                    projection_years,
                    total_debt,
                    cash_and_equivalents,
                    shares_outstanding,
                );
                let _ = msg_tx.send(BrokerMsg::DcfSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeSvmSnapshot {
            symbol,
            current_price,
            ddm_json,
            dcf_json,
            peer_pe_tuple_json,
            peer_ev_tuple_json,
            peer_pb_tuple_json,
        } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let ddm: Option<research::DdmSnapshot> = serde_json::from_str(&ddm_json).ok();
                let dcf: Option<research::DcfSnapshot> = serde_json::from_str(&dcf_json).ok();
                let peer_pe: Option<(f64, f64)> =
                    serde_json::from_str(&peer_pe_tuple_json).unwrap_or(None);
                let peer_ev: Option<(f64, f64, f64, f64, f64)> =
                    serde_json::from_str(&peer_ev_tuple_json).unwrap_or(None);
                let peer_pb: Option<(f64, f64)> =
                    serde_json::from_str(&peer_pb_tuple_json).unwrap_or(None);
                let snap = research::compute_svm_snapshot(
                    &symbol,
                    &today,
                    current_price,
                    ddm.as_ref(),
                    dcf.as_ref(),
                    peer_pe,
                    peer_ev,
                    peer_pb,
                );
                let _ = msg_tx.send(BrokerMsg::SvmSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::FetchOptionsChain { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::builder()
                    .user_agent("Mozilla/5.0 (X11; Linux x86_64) TyphooN-Terminal/0.1")
                    .timeout(std::time::Duration::from_secs(20))
                    .build()
                    .unwrap_or_default();
                match research::fetch_yahoo_options_chain(&client, &symbol).await {
                    Ok(snap) => {
                        let _ = msg_tx.send(BrokerMsg::OptionsChainMsg(symbol, snap));
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("OMON {symbol}: {e}")));
                    }
                }
            });
        }
        _ => { /* not valuation */ }
    }
}
