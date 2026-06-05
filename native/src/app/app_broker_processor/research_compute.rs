use super::*;

pub(super) fn handle_research_compute_command(
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
        // ── Round 8 handlers ──
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
        BrokerCmd::ComputeIvolSnapshot {
            symbol,
            current_atm_iv_pct,
            history_json,
        } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let history: Vec<research::IvolObservation> =
                    serde_json::from_str(&history_json).unwrap_or_default();
                let snap =
                    research::compute_ivol_snapshot(&symbol, &today, current_atm_iv_pct, &history);
                let _ = msg_tx.send(BrokerMsg::IvolSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 9 handlers ──
        BrokerCmd::ComputeSeasonalitySnapshot { symbol } => {
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
                if bars.len() >= 2 && bars[0].date > bars[bars.len() - 1].date {
                    bars.reverse();
                }
                let snap = research::compute_seasonality_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::SeasonalitySnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCorrelationMatrix {
            symbol,
            window_days,
            peer_series_json,
        } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let mut subject: Vec<research::HistoricalPriceRow> = Vec::new();
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        if let Ok(Some(rows)) = research::get_historical_price(&conn, &symbol) {
                            subject = rows;
                        }
                    }
                }
                if subject.len() >= 2 && subject[0].date > subject[subject.len() - 1].date {
                    subject.reverse();
                }
                let peers: Vec<(String, Vec<research::HistoricalPriceRow>)> =
                    serde_json::from_str(&peer_series_json).unwrap_or_default();
                let snap = research::compute_correlation_matrix(
                    &symbol,
                    &today,
                    window_days,
                    &subject,
                    &peers,
                );
                let _ = msg_tx.send(BrokerMsg::CorrelationMatrixMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeTotalReturnSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let mut bars: Vec<research::HistoricalPriceRow> = Vec::new();
                let mut divs: Vec<research::DividendRecord> = Vec::new();
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        if let Ok(Some(rows)) = research::get_historical_price(&conn, &symbol) {
                            bars = rows;
                        }
                        if let Ok(Some(d)) = research::get_dividends(&conn, &symbol) {
                            divs = d;
                        }
                    }
                }
                if bars.len() >= 2 && bars[0].date > bars[bars.len() - 1].date {
                    bars.reverse();
                }
                let snap = research::compute_total_return_snapshot(&symbol, &today, &bars, &divs);
                let _ = msg_tx.send(BrokerMsg::TotalReturnSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeTechnicalsSnapshot { symbol } => {
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
                if bars.len() >= 2 && bars[0].date > bars[bars.len() - 1].date {
                    bars.reverse();
                }
                let snap = research::compute_technical_indicators(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::TechnicalsSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeVolSkewSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let mut chain: Option<research::OptionsChainSnapshot> = None;
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        if let Ok(c) = research::get_options_chain(&conn, &symbol) {
                            chain = c;
                        }
                    }
                }
                let snap = match chain {
                    Some(c) => research::compute_volatility_skew(&symbol, &today, &c),
                    None => research::VolatilitySkew {
                        symbol: symbol.to_uppercase(),
                        as_of: today,
                        note: "no cached OMON chain — run OMON first".to_string(),
                        ..Default::default()
                    },
                };
                let _ = msg_tx.send(BrokerMsg::VolSkewSnapshotMsg(symbol, snap));
            });
        }
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
        BrokerCmd::ComputeSqueezeSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let (bars, si, iv, rv) =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            let bars = research::get_historical_price(&conn, &symbol)
                                .ok()
                                .flatten()
                                .unwrap_or_default();
                            let si = research::get_short_interest(&conn, &symbol).ok().flatten();
                            let iv = research::get_ivol(&conn, &symbol).ok().flatten();
                            let rv = research::get_relvol(&conn, &symbol).ok().flatten();
                            (bars, si, iv, rv)
                        } else {
                            (Vec::new(), None, None, None)
                        }
                    } else {
                        (Vec::new(), None, None, None)
                    };
                let snap = research::compute_squeeze_snapshot(
                    &symbol,
                    &today,
                    &bars,
                    si.as_ref(),
                    iv.as_ref(),
                    rv.as_ref(),
                );
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_squeeze(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::SqueezeSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeSqueezeRankSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let (subject, all) =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            let s = research::get_squeeze(&conn, &symbol).ok().flatten();
                            let a = research::get_all_squeeze(&conn).unwrap_or_default();
                            (s, a)
                        } else {
                            (None, Vec::new())
                        }
                    } else {
                        (None, Vec::new())
                    };
                let snap =
                    research::compute_squeezerank_snapshot(&symbol, &today, subject.as_ref(), &all);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_squeezerank(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::SqueezeRankSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::RefreshSqueezeWatchlist => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let mut rows: Vec<research::SqueezeSnapshot> = Vec::new();
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        // Recompute SQUEEZE for every symbol that has both historical prices
                        // and any of (short_interest, ivol, relvol). We walk the SHORT_INTEREST
                        // table as the source-of-truth set since that is the strongest predicate.
                        let syms =
                            research::get_all_short_interest_symbols(&conn).unwrap_or_default();
                        for sym in syms {
                            let bars = research::get_historical_price(&conn, &sym)
                                .ok()
                                .flatten()
                                .unwrap_or_default();
                            if bars.is_empty() {
                                continue;
                            }
                            let si = research::get_short_interest(&conn, &sym).ok().flatten();
                            let iv = research::get_ivol(&conn, &sym).ok().flatten();
                            let rv = research::get_relvol(&conn, &sym).ok().flatten();
                            let snap = research::compute_squeeze_snapshot(
                                &sym,
                                &today,
                                &bars,
                                si.as_ref(),
                                iv.as_ref(),
                                rv.as_ref(),
                            );
                            let _ = research::upsert_squeeze(&conn, &sym, &snap);
                            rows.push(snap);
                        }
                        // Now populate SQUEEZERANK across the full set we just computed.
                        let all = rows.clone();
                        for s in &all {
                            if s.squeeze_label == "INSUFFICIENT_DATA" {
                                continue;
                            }
                            let rsnap = research::compute_squeezerank_snapshot(
                                &s.symbol,
                                &today,
                                Some(s),
                                &all,
                            );
                            let _ = research::upsert_squeezerank(&conn, &s.symbol, &rsnap);
                        }
                    }
                }
                // Sort by composite desc for UI.
                rows.sort_by(|a, b| {
                    b.composite_score
                        .partial_cmp(&a.composite_score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                let _ = msg_tx.send(BrokerMsg::SqueezeWatchlistLoaded(rows));
            });
        }
        BrokerCmd::ComputeBbsqueezeSnapshot { symbol } => {
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
                let snap = research::compute_bbsqueeze_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_bbsqueeze(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::BbsqueezeSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeDonchianSnapshot { symbol } => {
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
                let snap = research::compute_donchian_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_donchian(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::DonchianSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeKamaSnapshot { symbol } => {
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
                let snap = research::compute_kama_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_kama(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::KamaSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeIchimokuSnapshot { symbol } => {
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
                let snap = research::compute_ichimoku_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_ichimoku(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::IchimokuSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeSupertrendSnapshot { symbol } => {
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
                let snap = research::compute_supertrend_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_supertrend(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::SupertrendSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeKeltnerSnapshot { symbol } => {
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
                let snap = research::compute_keltner_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_keltner(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::KeltnerSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeFisherSnapshot { symbol } => {
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
                let snap = research::compute_fisher_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_fisher(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::FisherSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeAroonSnapshot { symbol } => {
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
                let snap = research::compute_aroon_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_aroon(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::AroonSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 44 handlers ──
        BrokerCmd::ComputeAdxSnapshot { symbol } => {
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
                let snap = research::compute_adx_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_adx(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::AdxSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCciSnapshot { symbol } => {
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
                let snap = research::compute_cci_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cci(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CciSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCmfSnapshot { symbol } => {
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
                let snap = research::compute_cmf_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cmf(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CmfSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeMfiSnapshot { symbol } => {
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
                let snap = research::compute_mfi_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_mfi(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::MfiSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputePsarSnapshot { symbol } => {
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
                let snap = research::compute_psar_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_psar(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::PsarSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 45 handlers ──
        BrokerCmd::ComputeVortexSnapshot { symbol } => {
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
                let snap = research::compute_vortex_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_vortex(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::VortexSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeChopSnapshot { symbol } => {
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
                let snap = research::compute_chop_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_chop(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::ChopSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeObvSnapshot { symbol } => {
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
                let snap = research::compute_obv_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_obv(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::ObvSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeTrixSnapshot { symbol } => {
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
                let snap = research::compute_trix_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_trix(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::TrixSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeHmaSnapshot { symbol } => {
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
                let snap = research::compute_hma_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_hma(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::HmaSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 46 handlers ──
        BrokerCmd::ComputePpoSnapshot { symbol } => {
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
                let snap = research::compute_ppo_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_ppo(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::PpoSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeDpoSnapshot { symbol } => {
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
                let snap = research::compute_dpo_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_dpo(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::DpoSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeKstSnapshot { symbol } => {
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
                let snap = research::compute_kst_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_kst(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::KstSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeUltoscSnapshot { symbol } => {
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
                let snap = research::compute_ultosc_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_ultosc(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::UltoscSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeWillrSnapshot { symbol } => {
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
                let snap = research::compute_willr_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_willr(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::WillrSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 47 handlers ──
        BrokerCmd::ComputeMassSnapshot { symbol } => {
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
                let snap = research::compute_mass_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_mass(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::MassSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeChaikoscSnapshot { symbol } => {
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
                let snap = research::compute_chaikosc_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_chaikosc(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::ChaikoscSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeKlingerSnapshot { symbol } => {
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
                let snap = research::compute_klinger_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_klinger(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::KlingerSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeStochRsiSnapshot { symbol } => {
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
                let snap = research::compute_stochrsi_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_stochrsi(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::StochRsiSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeAwesomeSnapshot { symbol } => {
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
                let snap = research::compute_awesome_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_awesome(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::AwesomeSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 48 handlers ──
        BrokerCmd::ComputeEfiSnapshot { symbol } => {
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
                let snap = research::compute_efi_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_efi(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::EfiSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeEmvSnapshot { symbol } => {
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
                let snap = research::compute_emv_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_emv(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::EmvSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeNviSnapshot { symbol } => {
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
                let snap = research::compute_nvi_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_nvi(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::NviSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputePviSnapshot { symbol } => {
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
                let snap = research::compute_pvi_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_pvi(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::PviSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCoppockSnapshot { symbol } => {
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
                let snap = research::compute_coppock_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_coppock(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CoppockSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCmoSnapshot { symbol } => {
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
                let snap = research::compute_cmo_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cmo(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CmoSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeQstickSnapshot { symbol } => {
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
                let snap = research::compute_qstick_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_qstick(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::QstickSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeDisparitySnapshot { symbol } => {
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
                let snap = research::compute_disparity_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_disparity(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::DisparitySnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeBopSnapshot { symbol } => {
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
                let snap = research::compute_bop_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_bop(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::BopSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeSchaffSnapshot { symbol } => {
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
                let snap = research::compute_schaff_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_schaff(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::SchaffSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeStochSnapshot { symbol } => {
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
                let snap = research::compute_stoch_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_stoch(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::StochSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeMacdSnapshot { symbol } => {
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
                let snap = research::compute_macd_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_macd(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::MacdSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeVwapSnapshot { symbol } => {
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
                let snap = research::compute_vwap_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_vwap(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::VwapSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeMcgdSnapshot { symbol } => {
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
                let snap = research::compute_mcgd_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_mcgd(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::McgdSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeRwiSnapshot { symbol } => {
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
                let snap = research::compute_rwi_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_rwi(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::RwiSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 51 compute handlers ──
        BrokerCmd::ComputeDemaSnapshot { symbol } => {
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
                let snap = research::compute_dema_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_dema(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::DemaSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeTemaSnapshot { symbol } => {
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
                let snap = research::compute_tema_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_tema(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::TemaSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeLinregSnapshot { symbol } => {
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
                let snap = research::compute_linreg_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_linreg(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::LinregSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputePivotsSnapshot { symbol } => {
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
                let snap = research::compute_pivots_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_pivots(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::PivotsSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeHeikinSnapshot { symbol } => {
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
                let snap = research::compute_heikin_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_heikin(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::HeikinSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 52 compute handlers ──
        BrokerCmd::ComputeAlmaSnapshot { symbol } => {
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
                let snap = research::compute_alma_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_alma(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::AlmaSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeZlemaSnapshot { symbol } => {
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
                let snap = research::compute_zlema_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_zlema(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::ZlemaSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeElderRaySnapshot { symbol } => {
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
                let snap = research::compute_elderray_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_elderray(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::ElderRaySnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeTsfSnapshot { symbol } => {
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
                let snap = research::compute_tsf_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_tsf(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::TsfSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeRviSnapshot { symbol } => {
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
                let snap = research::compute_rvi_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_rvi(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::RviSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeTrimaSnapshot { symbol } => {
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
                let snap = research::compute_trima_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_trima(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::TrimaSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeT3Snapshot { symbol } => {
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
                let snap = research::compute_t3_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_t3(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::T3SnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeVidyaSnapshot { symbol } => {
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
                let snap = research::compute_vidya_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_vidya(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::VidyaSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeSmiSnapshot { symbol } => {
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
                let snap = research::compute_smi_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_smi(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::SmiSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputePvtSnapshot { symbol } => {
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
                let snap = research::compute_pvt_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_pvt(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::PvtSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeAcSnapshot { symbol } => {
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
                let snap = research::compute_ac_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_ac(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::AcSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeChvolSnapshot { symbol } => {
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
                let snap = research::compute_chvol_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_chvol(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::ChvolSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeBbwidthSnapshot { symbol } => {
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
                let snap = research::compute_bbwidth_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_bbwidth(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::BbwidthSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeElderImpSnapshot { symbol } => {
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
                let snap = research::compute_elder_impulse_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_elderimp(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::ElderImpSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeRmiSnapshot { symbol } => {
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
                let snap = research::compute_rmi_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_rmi(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::RmiSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeSymbolExpirations { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let snap =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            let computed = research::compute_symbol_expirations(&conn, &symbol)
                                .unwrap_or_default();
                            let _ = research::upsert_symbol_expirations(&conn, &symbol, &computed);
                            computed
                        } else {
                            Default::default()
                        }
                    } else {
                        Default::default()
                    };
                let _ = msg_tx.send(BrokerMsg::SymbolExpirationsMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeSmmaSnapshot { symbol } => {
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
                let snap = research::compute_smma_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_smma(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::SmmaSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeAlligatorSnapshot { symbol } => {
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
                let snap = research::compute_alligator_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_alligator(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::AlligatorSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCrsiSnapshot { symbol } => {
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
                let snap = research::compute_crsi_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_crsi(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CrsiSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeSebSnapshot { symbol } => {
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
                let snap = research::compute_seb_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_seb(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::SebSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeImiSnapshot { symbol } => {
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
                let snap = research::compute_imi_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_imi(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::ImiSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeGmmaSnapshot { symbol } => {
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
                let snap = research::compute_gmma_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_gmma(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::GmmaSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeMaenvSnapshot { symbol } => {
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
                let snap = research::compute_maenv_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_maenv(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::MaenvSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeAdlSnapshot { symbol } => {
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
                let snap = research::compute_adl_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_adl(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::AdlSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeVhfSnapshot { symbol } => {
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
                let snap = research::compute_vhf_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_vhf(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::VhfSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeVrocSnapshot { symbol } => {
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
                let snap = research::compute_vroc_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_vroc(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::VrocSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeKdjSnapshot { symbol } => {
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
                let snap = research::compute_kdj_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_kdj(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::KdjSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeQqeSnapshot { symbol } => {
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
                let snap = research::compute_qqe_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_qqe(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::QqeSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputePmoSnapshot { symbol } => {
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
                let snap = research::compute_pmo_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_pmo(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::PmoSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCfoSnapshot { symbol } => {
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
                let snap = research::compute_cfo_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cfo(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CfoSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeTmfSnapshot { symbol } => {
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
                let snap = research::compute_tmf_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_tmf(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::TmfSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeFractalsSnapshot { symbol } => {
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
                let snap = research::compute_fractals_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_fractals(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::FractalsSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeIftRsiSnapshot { symbol } => {
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
                let snap = research::compute_ift_rsi_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_ift_rsi(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::IftRsiSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeMamaSnapshot { symbol } => {
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
                let snap = research::compute_mama_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_mama(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::MamaSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCogSnapshot { symbol } => {
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
                let snap = research::compute_cog_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cog(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CogSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeDidiSnapshot { symbol } => {
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
                let snap = research::compute_didi_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_didi(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::DidiSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeDemarkerSnapshot { symbol } => {
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
                let snap = research::compute_demarker_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_demarker(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::DemarkerSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeGatorSnapshot { symbol } => {
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
                let snap = research::compute_gator_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_gator(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::GatorSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeBwMfiSnapshot { symbol } => {
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
                let snap = research::compute_bw_mfi_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_bw_mfi(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::BwMfiSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeVwmaSnapshot { symbol } => {
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
                let snap = research::compute_vwma_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_vwma(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::VwmaSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeStddevSnapshot { symbol } => {
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
                let snap = research::compute_stddev_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_stddev(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::StddevSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeWmaSnapshot { symbol } => {
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
                let snap = research::compute_wma_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_wma(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::WmaSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeRainbowSnapshot { symbol } => {
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
                let snap = research::compute_rainbow_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_rainbow(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::RainbowSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeMesaSineSnapshot { symbol } => {
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
                let snap = research::compute_mesa_sine_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_mesa_sine(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::MesaSineSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeFramaSnapshot { symbol } => {
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
                let snap = research::compute_frama_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_frama(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::FramaSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeIbsSnapshot { symbol } => {
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
                let snap = research::compute_ibs_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_ibs(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::IbsSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeLaguerreRsiSnapshot { symbol } => {
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
                let snap = research::compute_laguerre_rsi_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_laguerre_rsi(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::LaguerreRsiSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeZigzagSnapshot { symbol } => {
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
                let snap = research::compute_zigzag_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_zigzag(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::ZigzagSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputePgoSnapshot { symbol } => {
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
                let snap = research::compute_pgo_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_pgo(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::PgoSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeHtTrendlineSnapshot { symbol } => {
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
                let snap = research::compute_ht_trendline_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_ht_trendline(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::HtTrendlineSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeMidpointSnapshot { symbol } => {
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
                let snap = research::compute_midpoint_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_midpoint(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::MidpointSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 62 handlers ──
        BrokerCmd::ComputeMassIndexSnapshot { symbol } => {
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
                let snap = research::compute_mass_index_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_mass_index(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::MassIndexSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeNatrSnapshot { symbol } => {
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
                let snap = research::compute_natr_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_natr(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::NatrSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeTtmSqueezeSnapshot { symbol } => {
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
                let snap = research::compute_ttm_squeeze_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_ttm_squeeze(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::TtmSqueezeSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeForceIndexSnapshot { symbol } => {
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
                let snap = research::compute_force_index_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_force_index(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::ForceIndexSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeTrangeSnapshot { symbol } => {
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
                let snap = research::compute_trange_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_trange(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::TrangeSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 63 handlers ──
        BrokerCmd::ComputeLinearregSlopeSnapshot { symbol } => {
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
                let snap = research::compute_linearreg_slope_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_linearreg_slope(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::LinearregSlopeSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeHtDcperiodSnapshot { symbol } => {
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
                let snap = research::compute_ht_dcperiod_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_ht_dcperiod(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::HtDcperiodSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeHtTrendmodeSnapshot { symbol } => {
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
                let snap = research::compute_ht_trendmode_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_ht_trendmode(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::HtTrendmodeSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeAccbandsSnapshot { symbol } => {
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
                let snap = research::compute_accbands_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_accbands(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::AccbandsSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeStochfSnapshot { symbol } => {
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
                let snap = research::compute_stochf_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_stochf(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::StochfSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 64 handlers ──
        BrokerCmd::ComputeLinearregSnapshot { symbol } => {
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
                let snap = research::compute_linearreg_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_linearreg(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::LinearregSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeLinearregAngleSnapshot { symbol } => {
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
                let snap = research::compute_linearreg_angle_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_linearreg_angle(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::LinearregAngleSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeHtDcphaseSnapshot { symbol } => {
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
                let snap = research::compute_ht_dcphase_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_ht_dcphase(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::HtDcphaseSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeHtSineSnapshot { symbol } => {
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
                let snap = research::compute_ht_sine_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_ht_sine(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::HtSineSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeHtPhasorSnapshot { symbol } => {
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
                let snap = research::compute_ht_phasor_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_ht_phasor(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::HtPhasorSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 65 handlers ──
        BrokerCmd::ComputeMidpriceSnapshot { symbol } => {
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
                let snap = research::compute_midprice_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_midprice(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::MidpriceSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeApoSnapshot { symbol } => {
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
                let snap = research::compute_apo_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_apo(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::ApoSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeMomSnapshot { symbol } => {
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
                let snap = research::compute_mom_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_mom(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::MomSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeSarextSnapshot { symbol } => {
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
                let snap = research::compute_sarext_snapshot(
                    &symbol, &today, &bars, 0.0, 0.02, 0.02, 0.20, 0.02, 0.02, 0.20,
                );
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_sarext(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::SarextSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeAdxrSnapshot { symbol } => {
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
                let snap = research::compute_adxr_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_adxr(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::AdxrSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 66: AVGPRICE / MEDPRICE / TYPPRICE / WCLPRICE / VARIANCE ──
        BrokerCmd::ComputeAvgpriceSnapshot { symbol } => {
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
                let snap = research::compute_avgprice_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_avgprice(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::AvgpriceSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeMedpriceSnapshot { symbol } => {
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
                let snap = research::compute_medprice_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_medprice(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::MedpriceSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeTypPriceSnapshot { symbol } => {
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
                let snap = research::compute_typprice_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_typprice(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::TypPriceSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeWclPriceSnapshot { symbol } => {
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
                let snap = research::compute_wclprice_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_wclprice(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::WclPriceSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeVarianceSnapshot { symbol } => {
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
                let snap = research::compute_variance_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_variance(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::VarianceSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 67: PLUS_DI / MINUS_DI / PLUS_DM / MINUS_DM / DX ──
        BrokerCmd::ComputePlusDiSnapshot { symbol } => {
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
                let snap = research::compute_plus_di_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_plus_di(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::PlusDiSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeMinusDiSnapshot { symbol } => {
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
                let snap = research::compute_minus_di_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_minus_di(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::MinusDiSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputePlusDmSnapshot { symbol } => {
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
                let snap = research::compute_plus_dm_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_plus_dm(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::PlusDmSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeMinusDmSnapshot { symbol } => {
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
                let snap = research::compute_minus_dm_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_minus_dm(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::MinusDmSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeDxSnapshot { symbol } => {
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
                let snap = research::compute_dx_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_dx(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::DxSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 68 handlers ──
        BrokerCmd::ComputeRocSnapshot { symbol } => {
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
                let snap = research::compute_roc_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_roc(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::RocSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeRocpSnapshot { symbol } => {
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
                let snap = research::compute_rocp_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_rocp(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::RocpSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeRocrSnapshot { symbol } => {
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
                let snap = research::compute_rocr_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_rocr(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::RocrSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeRocr100Snapshot { symbol } => {
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
                let snap = research::compute_rocr100_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_rocr100(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::Rocr100SnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCorrelSnapshot { symbol } => {
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
                let snap = research::compute_correl_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_correl(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CorrelSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 69 handlers ──
        BrokerCmd::ComputeMinSnapshot { symbol } => {
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
                let snap = research::compute_min_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_min(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::MinSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeMaxSnapshot { symbol } => {
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
                let snap = research::compute_max_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_max(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::MaxSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeMinMaxSnapshot { symbol } => {
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
                let snap = research::compute_minmax_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_minmax(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::MinMaxSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeMinIndexSnapshot { symbol } => {
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
                let snap = research::compute_minindex_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_minindex(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::MinIndexSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeMaxIndexSnapshot { symbol } => {
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
                let snap = research::compute_maxindex_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_maxindex(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::MaxIndexSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 70 broker handlers ──
        BrokerCmd::ComputeBbandsSnapshot { symbol } => {
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
                let snap = research::compute_bbands_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_bbands(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::BbandsSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeAdSnapshot { symbol } => {
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
                let snap = research::compute_ad_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_ad(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::AdSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeAdoscSnapshot { symbol } => {
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
                let snap = research::compute_adosc_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_adosc(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::AdoscSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeSumSnapshot { symbol } => {
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
                let snap = research::compute_sum_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_sum(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::SumSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeLinearRegInterceptSnapshot { symbol } => {
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
                let snap = research::compute_linearreg_intercept_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_linreg_intercept(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::LinearRegInterceptSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 71 handlers ──
        BrokerCmd::ComputeAroonoscSnapshot { symbol } => {
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
                let snap = research::compute_aroonosc_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_aroonosc(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::AroonoscSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeMinMaxIndexSnapshot { symbol } => {
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
                let snap = research::compute_minmaxindex_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_minmaxindex(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::MinMaxIndexSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeMacdextSnapshot { symbol } => {
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
                let snap = research::compute_macdext_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_macdext(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::MacdextSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeMacdfixSnapshot { symbol } => {
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
                let snap = research::compute_macdfix_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_macdfix(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::MacdfixSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeMavpSnapshot { symbol } => {
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
                let snap = research::compute_mavp_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_mavp(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::MavpSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 72 CDL* handlers ──
        BrokerCmd::ComputeCdlDojiSnapshot { symbol } => {
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
                let snap = research::compute_cdl_doji_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_doji(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlDojiSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlHammerSnapshot { symbol } => {
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
                let snap = research::compute_cdl_hammer_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_hammer(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlHammerSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlShootingStarSnapshot { symbol } => {
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
                let snap = research::compute_cdl_shooting_star_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_shooting_star(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlShootingStarSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlEngulfingSnapshot { symbol } => {
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
                let snap = research::compute_cdl_engulfing_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_engulfing(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlEngulfingSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlHaramiSnapshot { symbol } => {
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
                let snap = research::compute_cdl_harami_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_harami(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlHaramiSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 73 handlers — CDL* 3-bar / 2-bar patterns ──
        BrokerCmd::ComputeCdlMorningStarSnapshot { symbol } => {
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
                let snap = research::compute_cdl_morning_star_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_morning_star(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlMorningStarSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlEveningStarSnapshot { symbol } => {
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
                let snap = research::compute_cdl_evening_star_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_evening_star(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlEveningStarSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlThreeBlackCrowsSnapshot { symbol } => {
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
                let snap = research::compute_cdl_three_black_crows_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_three_black_crows(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlThreeBlackCrowsSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlThreeWhiteSoldiersSnapshot { symbol } => {
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
                let snap =
                    research::compute_cdl_three_white_soldiers_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_three_white_soldiers(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlThreeWhiteSoldiersSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlDarkCloudCoverSnapshot { symbol } => {
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
                let snap = research::compute_cdl_dark_cloud_cover_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_dark_cloud_cover(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlDarkCloudCoverSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 74 handlers — CDL* piercing / doji variants / hammer mirrors ──
        BrokerCmd::ComputeCdlPiercingSnapshot { symbol } => {
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
                let snap = research::compute_cdl_piercing_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_piercing(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlPiercingSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlDragonflyDojiSnapshot { symbol } => {
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
                let snap = research::compute_cdl_dragonfly_doji_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_dragonfly_doji(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlDragonflyDojiSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlGravestoneDojiSnapshot { symbol } => {
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
                let snap = research::compute_cdl_gravestone_doji_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_gravestone_doji(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlGravestoneDojiSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlHangingManSnapshot { symbol } => {
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
                let snap = research::compute_cdl_hanging_man_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_hanging_man(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlHangingManSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlInvertedHammerSnapshot { symbol } => {
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
                let snap = research::compute_cdl_inverted_hammer_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_inverted_hammer(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlInvertedHammerSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 75 handlers — CDL* harami cross / long-legged doji / marubozu / spinning top / tristar ──
        BrokerCmd::ComputeCdlHaramiCrossSnapshot { symbol } => {
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
                let snap = research::compute_cdl_harami_cross_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_harami_cross(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlHaramiCrossSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlLongLeggedDojiSnapshot { symbol } => {
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
                let snap = research::compute_cdl_long_legged_doji_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_long_legged_doji(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlLongLeggedDojiSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlMarubozuSnapshot { symbol } => {
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
                let snap = research::compute_cdl_marubozu_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_marubozu(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlMarubozuSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlSpinningTopSnapshot { symbol } => {
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
                let snap = research::compute_cdl_spinning_top_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_spinning_top(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlSpinningTopSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlTristarSnapshot { symbol } => {
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
                let snap = research::compute_cdl_tristar_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_tristar(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlTristarSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 76 handlers — CDL* doji star / morning doji star / evening doji star / abandoned baby / three inside ──
        BrokerCmd::ComputeCdlDojiStarSnapshot { symbol } => {
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
                let snap = research::compute_cdl_doji_star_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_doji_star(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlDojiStarSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlMorningDojiStarSnapshot { symbol } => {
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
                let snap = research::compute_cdl_morning_doji_star_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_morning_doji_star(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlMorningDojiStarSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlEveningDojiStarSnapshot { symbol } => {
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
                let snap = research::compute_cdl_evening_doji_star_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_evening_doji_star(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlEveningDojiStarSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlAbandonedBabySnapshot { symbol } => {
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
                let snap = research::compute_cdl_abandoned_baby_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_abandoned_baby(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlAbandonedBabySnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlThreeInsideSnapshot { symbol } => {
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
                let snap = research::compute_cdl_three_inside_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_three_inside(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlThreeInsideSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 77 handlers — CDL* belt hold / closing marubozu / high wave / long line / short line ──
        BrokerCmd::ComputeCdlBeltHoldSnapshot { symbol } => {
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
                let snap = research::compute_cdl_belt_hold_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_belt_hold(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlBeltHoldSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlClosingMarubozuSnapshot { symbol } => {
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
                let snap = research::compute_cdl_closing_marubozu_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_closing_marubozu(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlClosingMarubozuSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlHighWaveSnapshot { symbol } => {
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
                let snap = research::compute_cdl_high_wave_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_high_wave(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlHighWaveSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlLongLineSnapshot { symbol } => {
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
                let snap = research::compute_cdl_long_line_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_long_line(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlLongLineSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlShortLineSnapshot { symbol } => {
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
                let snap = research::compute_cdl_short_line_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_short_line(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlShortLineSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 78 handlers — CDL* counterattack / homing pigeon / in-neck / on-neck / thrusting ──
        BrokerCmd::ComputeCdlCounterattackSnapshot { symbol } => {
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
                let snap = research::compute_cdl_counterattack_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_counterattack(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlCounterattackSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlHomingPigeonSnapshot { symbol } => {
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
                let snap = research::compute_cdl_homing_pigeon_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_homing_pigeon(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlHomingPigeonSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlInNeckSnapshot { symbol } => {
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
                let snap = research::compute_cdl_in_neck_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_in_neck(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlInNeckSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlOnNeckSnapshot { symbol } => {
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
                let snap = research::compute_cdl_on_neck_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_on_neck(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlOnNeckSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlThrustingSnapshot { symbol } => {
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
                let snap = research::compute_cdl_thrusting_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_thrusting(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlThrustingSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 79/80 handlers — additional CDL* parity windows ──
        BrokerCmd::ComputeCdlTwoCrowsSnapshot { symbol } => {
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
                let snap = research::compute_cdl_two_crows_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_two_crows(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlTwoCrowsSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlThreeLineStrikeSnapshot { symbol } => {
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
                let snap = research::compute_cdl_three_line_strike_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_three_line_strike(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlThreeLineStrikeSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlThreeOutsideSnapshot { symbol } => {
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
                let snap = research::compute_cdl_three_outside_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_three_outside(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlThreeOutsideSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlMatchingLowSnapshot { symbol } => {
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
                let snap = research::compute_cdl_matching_low_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_matching_low(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlMatchingLowSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlSeparatingLinesSnapshot { symbol } => {
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
                let snap = research::compute_cdl_separating_lines_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_separating_lines(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlSeparatingLinesSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlStickSandwichSnapshot { symbol } => {
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
                let snap = research::compute_cdl_stick_sandwich_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_stick_sandwich(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlStickSandwichSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlRickshawManSnapshot { symbol } => {
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
                let snap = research::compute_cdl_rickshaw_man_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_rickshaw_man(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlRickshawManSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlTakuriSnapshot { symbol } => {
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
                let snap = research::compute_cdl_takuri_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_takuri(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlTakuriSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 81/82 handlers — harder CDL* parity windows ──
        BrokerCmd::ComputeCdlThreeStarsInSouthSnapshot { symbol } => {
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
                let snap =
                    research::compute_cdl_three_stars_in_south_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_three_stars_in_south(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlThreeStarsInSouthSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlIdenticalThreeCrowsSnapshot { symbol } => {
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
                let snap =
                    research::compute_cdl_identical_three_crows_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_identical_three_crows(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlIdenticalThreeCrowsSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlKickingSnapshot { symbol } => {
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
                let snap = research::compute_cdl_kicking_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_kicking(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlKickingSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlKickingByLengthSnapshot { symbol } => {
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
                let snap = research::compute_cdl_kicking_by_length_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_kicking_by_length(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlKickingByLengthSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlLadderBottomSnapshot { symbol } => {
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
                let snap = research::compute_cdl_ladder_bottom_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_ladder_bottom(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlLadderBottomSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlUniqueThreeRiverSnapshot { symbol } => {
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
                let snap =
                    research::compute_cdl_unique_three_river_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_unique_three_river(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlUniqueThreeRiverSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 83/84 handlers — additional multi-bar CDL* parity windows ──
        BrokerCmd::ComputeCdlAdvanceBlockSnapshot { symbol } => {
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
                let snap = research::compute_cdl_advance_block_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_advance_block(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlAdvanceBlockSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlBreakawaySnapshot { symbol } => {
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
                let snap = research::compute_cdl_breakaway_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_breakaway(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlBreakawaySnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlGapSideSideWhiteSnapshot { symbol } => {
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
                let snap =
                    research::compute_cdl_gap_side_side_white_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_gap_side_side_white(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlGapSideSideWhiteSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlUpsideGapTwoCrowsSnapshot { symbol } => {
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
                let snap =
                    research::compute_cdl_upside_gap_two_crows_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_upside_gap_two_crows(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlUpsideGapTwoCrowsSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlXSideGapThreeMethodsSnapshot { symbol } => {
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
                let snap =
                    research::compute_cdl_xside_gap_three_methods_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_xside_gap_three_methods(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlXSideGapThreeMethodsSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlConcealBabySwallowSnapshot { symbol } => {
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
                let snap =
                    research::compute_cdl_conceal_baby_swallow_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_conceal_baby_swallow(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlConcealBabySwallowSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 85/86 handlers — stateful CDL* parity windows ──
        BrokerCmd::ComputeCdlHikkakeSnapshot { symbol } => {
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
                let snap = research::compute_cdl_hikkake_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_hikkake(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlHikkakeSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlHikkakeModSnapshot { symbol } => {
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
                let snap = research::compute_cdl_hikkake_mod_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_hikkake_mod(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlHikkakeModSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlMatHoldSnapshot { symbol } => {
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
                let snap = research::compute_cdl_mat_hold_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_mat_hold(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlMatHoldSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlRiseFallThreeMethodsSnapshot { symbol } => {
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
                let snap =
                    research::compute_cdl_rise_fall_three_methods_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_rise_fall_three_methods(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlRiseFallThreeMethodsSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 87/88 handlers — final CDL* parity windows ──
        BrokerCmd::ComputeCdlStalledPatternSnapshot { symbol } => {
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
                let snap = research::compute_cdl_stalled_pattern_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_stalled_pattern(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlStalledPatternSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCdlTasukiGapSnapshot { symbol } => {
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
                let snap = research::compute_cdl_tasuki_gap_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_cdl_tasuki_gap(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::CdlTasukiGapSnapshotMsg(symbol, snap));
            });
        }
        // ── Round 76 (Quant Stats) handlers ──
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
        // ── web article ingestion handler ──
        _ => unreachable!("non research-compute command routed to research_compute"),
    }
}
