use super::prelude::*;

pub(super) fn handle_squeeze_compute(
    cmd: BrokerCmd,
    broker_msg_tx_clone: tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    shared_cache_broker: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
) {
    match cmd {
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
        _ => { /* not squeeze */ }
    }
}
