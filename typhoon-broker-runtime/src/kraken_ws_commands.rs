use std::sync::Arc;

use typhoon_engine::broker::protocol::{BrokerCmd, BrokerMsg};
use typhoon_engine::core::cache::SqliteCache;

use crate::kraken_ohlc_pipeline;

/// Stop the WS v2 book resubscribe loop after this many *consecutive* checksum
/// mismatches. A deterministically-failing book (e.g. the xStock fixed-precision
/// checksum bug) would otherwise reconnect a fresh websocket forever; bounding it
/// frees the connection and stops feeding the Kraken WS-connect rate limit.
const KRAKEN_WS_BOOK_MAX_RESUBSCRIBE_ATTEMPTS: u32 = 10;
/// Upper bound (seconds) for the exponential resubscribe backoff.
const KRAKEN_WS_BOOK_RESUBSCRIBE_BACKOFF_CAP_S: u64 = 60;

fn kraken_ws_v2_book_state_json(
    display_symbol: &str,
    state: &typhoon_engine::broker::kraken::KrakenWsBookState,
    checksum: Option<u32>,
    status: &str,
) -> String {
    let timestamp = state
        .last_ts_ms
        .and_then(chrono::DateTime::from_timestamp_millis)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_default();
    let bids: Vec<serde_json::Value> = state
        .bids
        .iter()
        .map(|level| {
            serde_json::json!({
                "price": level.price,
                "size": level.qty,
                "price_text": level.price_text,
                "size_text": level.qty_text,
            })
        })
        .collect();
    let asks: Vec<serde_json::Value> = state
        .asks
        .iter()
        .map(|level| {
            serde_json::json!({
                "price": level.price,
                "size": level.qty,
                "price_text": level.price_text,
                "size_text": level.qty_text,
            })
        })
        .collect();
    serde_json::json!({
        "symbol": display_symbol,
        "ws_symbol": state.symbol,
        "timestamp": timestamp,
        "depth": state.depth,
        "checksum": checksum,
        "server_checksum": state.last_checksum,
        "checksum_status": status,
        "bids": bids,
        "asks": asks,
    })
    .to_string()
}

fn top_of_kraken_ws_v2_book(
    state: &typhoon_engine::broker::kraken::KrakenWsBookState,
) -> Option<(f64, f64)> {
    let bid = state.bids.first()?.price;
    let ask = state.asks.first()?.price;
    (bid > 0.0 && ask > 0.0 && bid.is_finite() && ask.is_finite()).then_some((bid, ask))
}

fn resolve_kraken_chart_book_ws_symbol(symbol: &str) -> Option<String> {
    let bare = symbol
        .trim()
        .trim_end_matches(".EQ")
        .trim_end_matches(".eq")
        .to_ascii_uppercase();
    if bare.is_empty() || bare.contains('/') {
        return None;
    }
    Some(format!("{bare}x/USD"))
}

pub async fn handle_kraken_ws_command(
    cmd: BrokerCmd,
    kraken_broker: Option<&typhoon_engine::broker::kraken::KrakenBroker>,
    kraken_ws_broker: Option<&typhoon_engine::broker::kraken::KrakenBroker>,
    broker_msg_tx: &tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    shared_cache_broker: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
    kraken_public_client: reqwest::Client,
) {
    match cmd {
        BrokerCmd::KrakenStartPrivateWs => {
            let ws_client = kraken_ws_broker.as_ref().or(kraken_broker.as_ref());
            if let Some(kb) = ws_client {
                let msg_tx = broker_msg_tx.clone();
                match kb.start_private_ws().await {
                    Ok(mut rx) => {
                        let value = msg_tx.clone();
                        tokio::spawn(async move {
                            while let Some(msg) = rx.recv().await {
                                // Try to parse as ownTrades update
                                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&msg)
                                {
                                    if parsed.get("event").and_then(|v| v.as_str())
                                        == Some("heartbeat")
                                    {
                                        continue;
                                    }
                                    let trades =
                                        typhoon_engine::broker::kraken::parse_own_trades_messages(
                                            &parsed,
                                        );
                                    if !trades.is_empty() {
                                        for trade in trades {
                                            let _ = value.send(BrokerMsg::KrakenLiveTrade(trade));
                                        }
                                        continue;
                                    }
                                    if parsed.get("event").and_then(|v| v.as_str())
                                        == Some("systemStatus")
                                        || parsed.get("event").and_then(|v| v.as_str())
                                            == Some("subscriptionStatus")
                                    {
                                        let status = parsed
                                            .get("status")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("info")
                                            .to_string();
                                        let channel = parsed
                                            .get("subscription")
                                            .and_then(|v| v.get("name"))
                                            .and_then(|v| v.as_str());
                                        let exchange_message = parsed
                                            .get("errorMessage")
                                            .or_else(|| parsed.get("message"))
                                            .and_then(|v| v.as_str());
                                        let message = match (channel, exchange_message) {
                                            (Some(channel), Some(detail)) => {
                                                format!("{channel}: {detail}")
                                            }
                                            (Some(channel), None) => channel.to_string(),
                                            (None, Some(detail)) => detail.to_string(),
                                            (None, None) => {
                                                "Kraken private WebSocket status".to_string()
                                            }
                                        };
                                        let _ = value
                                            .send(BrokerMsg::KrakenWsStatus { status, message });
                                        continue;
                                    }
                                    let orders =
                                        typhoon_engine::broker::kraken::parse_open_orders_message(
                                            &parsed,
                                        );
                                    if !orders.is_empty() {
                                        let _ = value.send(BrokerMsg::KrakenOpenOrders(orders));
                                        continue;
                                    }
                                }
                                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&msg)
                                {
                                    let kind = parsed
                                        .get("event")
                                        .or_else(|| parsed.get("channelName"))
                                        .or_else(|| parsed.get("channel"))
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("private-update");
                                    tracing::debug!(
                                        "Unhandled Kraken private WebSocket message suppressed from UI log: {}",
                                        kind
                                    );
                                } else {
                                    tracing::debug!(
                                        "Unhandled non-JSON Kraken private WebSocket message suppressed from UI log"
                                    );
                                }
                            }
                        });
                        let _ = msg_tx.send(BrokerMsg::OrderResult(
                            "Kraken private WebSocket started".into(),
                        ));
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("Kraken WS failed: {}", e)));
                    }
                }
            }
        }
        BrokerCmd::KrakenStartOhlcStreamers {
            pairs,
            intervals_min,
        } => {
            // Bridge channels: streamers write bars into the writer;
            // writer reports flushes back to the main loop via BrokerMsg.
            let msg_tx = broker_msg_tx.clone();
            let pair_count = pairs.len();
            if pair_count == 0 {
                let _ = msg_tx.send(BrokerMsg::Error(
                    "KrakenStartOhlcStreamers: no pairs supplied".into(),
                ));
            } else {
                let (commit_tx, mut commit_rx) = tokio::sync::mpsc::unbounded_channel();
                let (status_tx, mut status_rx) = tokio::sync::mpsc::unbounded_channel();
                // Drain commits into BrokerMsg::KrakenWsBarsCommitted so the
                // main loop can update WS-fresh state and skip REST refetch.
                let commit_msg_tx = msg_tx.clone();
                tokio::spawn(async move {
                    while let Some(fresh) = commit_rx.recv().await {
                        let _ = commit_msg_tx.send(BrokerMsg::KrakenWsBarsCommitted { fresh });
                    }
                });
                // Drain lifecycle events into BrokerMsg::KrakenWsOhlcStatus.
                let status_msg_tx = msg_tx.clone();
                tokio::spawn(async move {
                    while let Some(event) = status_rx.recv().await {
                        let (interval_min, kind, detail) = match event {
                            typhoon_engine::broker::kraken::KrakenOhlcStreamerEvent::Connected { interval_min } => {
                                (interval_min, "connected".to_string(), String::new())
                            }
                            typhoon_engine::broker::kraken::KrakenOhlcStreamerEvent::Subscribed { interval_min, batches } => {
                                (interval_min, "subscribed".to_string(), format!("{batches} batches"))
                            }
                            typhoon_engine::broker::kraken::KrakenOhlcStreamerEvent::Disconnected { interval_min, reason } => {
                                (interval_min, "disconnected".to_string(), reason)
                            }
                            typhoon_engine::broker::kraken::KrakenOhlcStreamerEvent::SubscribeFailed { interval_min, reason } => {
                                (interval_min, "subscribe_failed".to_string(), reason)
                            }
                        };
                        let _ = status_msg_tx.send(BrokerMsg::KrakenWsOhlcStatus {
                            interval_min,
                            kind,
                            detail,
                        });
                    }
                });
                kraken_ohlc_pipeline::spawn_kraken_ohlc_pipeline(
                    shared_cache_broker.clone(),
                    pairs,
                    intervals_min.clone(),
                    commit_tx,
                    status_tx,
                );
                let interval_count = intervals_min.len();
                let _ = msg_tx.send(BrokerMsg::OrderResult(format!(
                    "Kraken WS OHLC streamers started: {pair_count} pairs × {interval_count} enabled intervals",
                )));
            }
        }
        BrokerCmd::KrakenOhlcSnapshotSweep {
            interval_min,
            pairs,
        } => {
            let msg_tx = broker_msg_tx.clone();
            let pair_count = pairs.len();
            if pair_count == 0 {
                let _ = msg_tx.send(BrokerMsg::KrakenWsOhlcSnapshotSweepSettled {
                    interval_min,
                    pair_count: 0,
                    error: None,
                });
            } else {
                let (commit_tx, mut commit_rx) = tokio::sync::mpsc::unbounded_channel();
                let (status_tx, mut status_rx) = tokio::sync::mpsc::unbounded_channel();
                let (settled_tx, mut settled_rx) = tokio::sync::mpsc::unbounded_channel();

                let commit_msg_tx = msg_tx.clone();
                tokio::spawn(async move {
                    while let Some(fresh) = commit_rx.recv().await {
                        let _ = commit_msg_tx.send(BrokerMsg::KrakenWsBarsCommitted { fresh });
                    }
                });
                let status_msg_tx = msg_tx.clone();
                tokio::spawn(async move {
                    while let Some(event) = status_rx.recv().await {
                        let (interval_min, kind, detail) = match event {
                            typhoon_engine::broker::kraken::KrakenOhlcStreamerEvent::Connected { interval_min } => {
                                (interval_min, "snapshot_connected".to_string(), String::new())
                            }
                            typhoon_engine::broker::kraken::KrakenOhlcStreamerEvent::Subscribed { interval_min, batches } => {
                                (interval_min, "snapshot_subscribed".to_string(), format!("{batches} batches"))
                            }
                            typhoon_engine::broker::kraken::KrakenOhlcStreamerEvent::Disconnected { interval_min, reason } => {
                                (interval_min, "snapshot_disconnected".to_string(), reason)
                            }
                            typhoon_engine::broker::kraken::KrakenOhlcStreamerEvent::SubscribeFailed { interval_min, reason } => {
                                (interval_min, "snapshot_subscribe_failed".to_string(), reason)
                            }
                        };
                        let _ = status_msg_tx.send(BrokerMsg::KrakenWsOhlcStatus {
                            interval_min,
                            kind,
                            detail,
                        });
                    }
                });
                let settled_msg_tx = msg_tx.clone();
                tokio::spawn(async move {
                    if let Some(result) = settled_rx.recv().await {
                        match result {
                            Ok((interval_min, pair_count)) => {
                                let _ = settled_msg_tx.send(
                                    BrokerMsg::KrakenWsOhlcSnapshotSweepSettled {
                                        interval_min,
                                        pair_count,
                                        error: None,
                                    },
                                );
                            }
                            Err(error) => {
                                let _ = settled_msg_tx.send(
                                    BrokerMsg::KrakenWsOhlcSnapshotSweepSettled {
                                        interval_min,
                                        pair_count,
                                        error: Some(error),
                                    },
                                );
                            }
                        }
                    }
                });
                kraken_ohlc_pipeline::spawn_kraken_ohlc_snapshot_sweep(
                    shared_cache_broker.clone(),
                    interval_min,
                    pairs,
                    commit_tx,
                    status_tx,
                    settled_tx,
                );
            }
        }
        BrokerCmd::KrakenStartOrderbookWs {
            symbol,
            depth,
            publish_dom,
        } => {
            let msg_tx = broker_msg_tx.clone();
            let ws_symbol = typhoon_engine::core::kraken::resolve_kraken_ws_pair(
                &kraken_public_client,
                &symbol,
            )
            .await
            .or_else(|| resolve_kraken_chart_book_ws_symbol(&symbol));
            let Some(ws_symbol) = ws_symbol else {
                let _ = msg_tx.send(BrokerMsg::OrderResult(format!(
                    "Kraken WS v2 book skipped: {symbol} is not a WS-mappable Kraken pair"
                )));
                return;
            };
            let depth = depth.clamp(10, 500);
            let update_msg_tx = msg_tx.clone();
            let display_symbol = symbol.clone();
            let state_symbol = ws_symbol.clone();
            tokio::spawn(async move {
                let mut resubscribe_count: u32 = 0;
                loop {
                    let (book_tx, mut book_rx) = tokio::sync::mpsc::channel::<
                        typhoon_engine::broker::kraken::KrakenWsBookDelta,
                    >(1024);
                    let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel::<
                        typhoon_engine::broker::kraken::KrakenBookStreamerEvent,
                    >();
                    let streamer_symbol = state_symbol.clone();
                    let streamer_handle = tokio::spawn(async move {
                        typhoon_engine::broker::kraken::run_book_streamer(
                            vec![streamer_symbol],
                            depth,
                            book_tx,
                            event_tx,
                        )
                        .await;
                    });
                    let mut state = typhoon_engine::broker::kraken::KrakenWsBookState::new(
                        state_symbol.clone(),
                        depth,
                    );
                    let mut retry_after_mismatch = false;
                    loop {
                        tokio::select! {
                            maybe_delta = book_rx.recv() => {
                                let Some(delta) = maybe_delta else { break; };
                                match state.apply_delta_with_checksum(&delta) {
                                    Ok(checksum) => {
                                        // Healthy snapshot — clear the consecutive-mismatch
                                        // counter so only *sustained* failures trip the cap.
                                        resubscribe_count = 0;
                                        if let Some((bid, ask)) = top_of_kraken_ws_v2_book(&state) {
                                            let _ = update_msg_tx.send(BrokerMsg::KrakenBookQuoteTick {
                                                symbol: display_symbol.clone(),
                                                bid,
                                                ask,
                                            });
                                        }
                                        if publish_dom {
                                            let text = kraken_ws_v2_book_state_json(
                                                &display_symbol,
                                                &state,
                                                checksum,
                                                "ok",
                                            );
                                            let _ = update_msg_tx.send(BrokerMsg::KrakenOrderbookUpdate(text));
                                        }
                                    }
                                    Err(err) => {
                                        if publish_dom {
                                            let text = kraken_ws_v2_book_state_json(
                                                &display_symbol,
                                                &state,
                                                Some(err.actual),
                                                "checksum_mismatch",
                                            );
                                            let _ = update_msg_tx.send(BrokerMsg::KrakenOrderbookUpdate(text));
                                        }
                                        resubscribe_count = resubscribe_count.saturating_add(1);
                                        // Throttle: the first few attempts are useful signal;
                                        // beyond that a persistently-failing book would spam a
                                        // line every backoff tick.
                                        let should_log = resubscribe_count <= 3
                                            || resubscribe_count % 20 == 0;
                                        if should_log {
                                            if publish_dom {
                                                let _ = update_msg_tx.send(BrokerMsg::Error(format!(
                                                    "Kraken WS v2 book checksum mismatch for {}: expected {}, actual {}; resubscribing snapshot attempt {}",
                                                    err.symbol, err.expected, err.actual, resubscribe_count
                                                )));
                                            } else {
                                                tracing::warn!(
                                                    "Kraken WS v2 book checksum mismatch for {}: expected {}, actual {}; resubscribing snapshot attempt {}",
                                                    err.symbol,
                                                    err.expected,
                                                    err.actual,
                                                    resubscribe_count
                                                );
                                            }
                                        }
                                        retry_after_mismatch = true;
                                        break;
                                    }
                                }
                            }
                            maybe_event = event_rx.recv() => {
                                let Some(event) = maybe_event else { continue; };
                                let text = match event {
                                    typhoon_engine::broker::kraken::KrakenBookStreamerEvent::Connected { depth } => {
                                        format!("Kraken WS v2 book connected: {state_symbol} depth {depth}")
                                    }
                                    typhoon_engine::broker::kraken::KrakenBookStreamerEvent::Subscribed { depth, batches } => {
                                        format!("Kraken WS v2 book subscribed: {state_symbol} depth {depth} ({batches} batch)")
                                    }
                                    typhoon_engine::broker::kraken::KrakenBookStreamerEvent::SubscribeFailed { depth, reason } => {
                                        format!("Kraken WS v2 book subscribe failed: {state_symbol} depth {depth}: {reason}")
                                    }
                                    typhoon_engine::broker::kraken::KrakenBookStreamerEvent::Disconnected { depth, reason } => {
                                        format!("Kraken WS v2 book disconnected: {state_symbol} depth {depth}: {reason}")
                                    }
                                };
                                if publish_dom {
                                    let _ = update_msg_tx.send(BrokerMsg::OrderResult(text));
                                } else {
                                    tracing::debug!("{text}");
                                }
                            }
                        }
                    }
                    streamer_handle.abort();
                    if !retry_after_mismatch {
                        break;
                    }
                    // Persistent checksum failure (e.g. the xStock fixed-precision bug):
                    // stop churning a fresh websocket every couple seconds forever. Give
                    // up after a bounded number of consecutive attempts so the connection
                    // is freed and the Kraken WS-connect limiter isn't fed needlessly.
                    if resubscribe_count >= KRAKEN_WS_BOOK_MAX_RESUBSCRIBE_ATTEMPTS {
                        let msg = format!(
                            "Kraken WS v2 book {state_symbol}: persistent checksum mismatch after {resubscribe_count} attempts — giving up resubscribe (quote stale until chart reopens)"
                        );
                        if publish_dom {
                            let _ = update_msg_tx.send(BrokerMsg::Error(msg));
                        } else {
                            tracing::warn!("{msg}");
                        }
                        break;
                    }
                    // Exponential backoff, capped: 2s, 4s, 8s, 16s, 32s, then 60s.
                    let backoff_s = 2u64
                        .pow(resubscribe_count.min(6))
                        .min(KRAKEN_WS_BOOK_RESUBSCRIBE_BACKOFF_CAP_S);
                    tokio::time::sleep(std::time::Duration::from_secs(backoff_s)).await;
                    if publish_dom {
                        let _ = update_msg_tx.send(BrokerMsg::OrderResult(format!(
                            "Kraken WS v2 book resubscribing: {state_symbol} depth {depth}"
                        )));
                    }
                }
            });
            if publish_dom {
                let _ = msg_tx.send(BrokerMsg::OrderResult(format!(
                    "Kraken WS v2 book starting: {ws_symbol} depth {depth}"
                )));
            } else {
                tracing::debug!(
                    "Kraken WS v2 chart book quote starting: {ws_symbol} depth {depth}"
                );
            }
        }
        BrokerCmd::KrakenStartTickerWs { symbol } => {
            let msg_tx = broker_msg_tx.clone();
            let ws_symbol = typhoon_engine::core::kraken::resolve_kraken_ws_pair(
                &kraken_public_client,
                &symbol,
            )
            .await
            .or_else(|| resolve_kraken_chart_book_ws_symbol(&symbol));
            let Some(ws_symbol) = ws_symbol else {
                let _ = msg_tx.send(BrokerMsg::OrderResult(format!(
                    "Kraken WS v2 ticker skipped: {symbol} is not a WS-mappable Kraken pair"
                )));
                return;
            };
            let update_msg_tx = msg_tx.clone();
            let _display_symbol = symbol.clone();
            let state_symbol = ws_symbol.clone();
            tokio::spawn(async move {
                let (ticker_tx, mut ticker_rx) = tokio::sync::mpsc::channel::<
                    typhoon_engine::broker::kraken::KrakenWsTicker,
                >(1024);
                let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel::<
                    typhoon_engine::broker::kraken::KrakenTickerStreamerEvent,
                >();
                let streamer_symbol = state_symbol.clone();
                let streamer_handle = tokio::spawn(async move {
                    typhoon_engine::broker::kraken::run_ticker_streamer(
                        vec![streamer_symbol],
                        ticker_tx,
                        event_tx,
                    )
                    .await;
                });
                loop {
                    tokio::select! {
                        maybe_t = ticker_rx.recv() => {
                            let Some(t) = maybe_t else { break; };
                            // Forward rich L1 ticker data (bid/ask/sizes/last/vol etc.)
                            let _ = update_msg_tx.send(BrokerMsg::KrakenWsTicker(t));
                        }
                        maybe_event = event_rx.recv() => {
                            let Some(event) = maybe_event else { continue; };
                            let text = match event {
                                typhoon_engine::broker::kraken::KrakenTickerStreamerEvent::Connected => {
                                    format!("Kraken WS v2 ticker connected: {state_symbol}")
                                }
                                typhoon_engine::broker::kraken::KrakenTickerStreamerEvent::Subscribed { batches } => {
                                    format!("Kraken WS v2 ticker subscribed: {state_symbol} batches={batches}")
                                }
                                typhoon_engine::broker::kraken::KrakenTickerStreamerEvent::Disconnected { reason } => {
                                    format!("Kraken WS v2 ticker disconnected: {state_symbol} {reason}")
                                }
                                typhoon_engine::broker::kraken::KrakenTickerStreamerEvent::SubscribeFailed { reason } => {
                                    format!("Kraken WS v2 ticker subscribe failed: {state_symbol} {reason}")
                                }
                            };
                            let _ = update_msg_tx.send(BrokerMsg::KrakenWsStatus { status: "ticker".into(), message: text });
                        }
                    }
                }
                let _ = streamer_handle.await;
            });
            let _ = msg_tx.send(BrokerMsg::OrderResult(format!(
                "Kraken WS v2 ticker starting for L1: {ws_symbol}"
            )));
        }
        BrokerCmd::KrakenStartLevel3Ws { symbol } => {
            let msg_tx = broker_msg_tx.clone();
            // Kraken L3 is authenticated per-order book (add/mod/del), richer than L2 but requires token + subscription limits.
            // Foundation URL exists; full streamer not wired yet (use L1 ticker + L2 book for most rich data).
            let _ = msg_tx.send(BrokerMsg::OrderResult(format!(
                "Kraken L3 (per-order) requested for {} — auth/streamer support partial (L1/L2 are primary rich feeds)", symbol
            )));
            // TODO: full impl with auth token + ws-l3 subscribe when needed.
        }
         _ => unreachable!("non-Kraken websocket command routed to Kraken websocket handler"),
     }
 }
