use super::*;

pub(super) fn handle_analytics_compute(
    cmd: BrokerCmd,
    broker_msg_tx_clone: tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    shared_cache_broker: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
) {
    match cmd {
        // ── handlers ──
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
        _ => { /* not analytics */ }
    }
}
