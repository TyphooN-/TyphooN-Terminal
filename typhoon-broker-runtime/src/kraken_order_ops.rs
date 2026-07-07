use std::collections::BTreeMap;

use typhoon_engine::broker::kraken::KrakenBroker;
use typhoon_engine::broker::protocol::{
    BrokerCmd, BrokerMsg, KrakenAccountOrders, KrakenAccountPositions, KrakenAccountTrades,
};

use crate::account_pool::KrakenAccountPool;
use crate::alpaca_order_ops::trade_copy_deltas;

/// Kraken spot AddOrder pair for a bare xStock ticker — the tradeable
/// `{TICKER}x/USD` form the WS book/OHLC lanes and the native order path use.
fn kraken_xstock_order_pair(ticker: &str) -> String {
    format!("{}x/USD", ticker.trim().to_ascii_uppercase())
}

/// The app's Kraken position definition split for TradeCopy: signed qty per
/// bare xStock ticker (equity-balance positions, always long) plus the count
/// of margin positions, which spot market orders cannot replicate.
async fn kraken_equity_position_map(
    kb: &KrakenBroker,
) -> Result<(BTreeMap<String, f64>, usize), String> {
    let all = kb.get_all_position_summaries().await?;
    let mut map = BTreeMap::new();
    let mut margin = 0usize;
    for p in all {
        if p.asset_id.starts_with("equity_balance:") {
            let signed = if p.side.eq_ignore_ascii_case("short") {
                -p.qty
            } else {
                p.qty
            };
            if signed.abs() > 0.0 {
                map.insert(p.symbol, signed);
            }
        } else {
            margin += 1;
        }
    }
    Ok((map, margin))
}

/// One-shot Kraken TradeCopy (ADR-130): replicate the source account's xStock
/// equity holdings onto each opted-in target with spot market orders for the
/// per-ticker deltas. Margin positions are reported and skipped. Pairs are
/// checked against the live AssetPairs catalog when it is reachable so a
/// Securities-only holding with no Spot pair warns instead of placing a
/// doomed order.
pub async fn handle_kraken_trade_copy(
    source_id: String,
    target_ids: Vec<String>,
    flatten_extra: bool,
    pool: &KrakenAccountPool,
    broker_msg_tx: &tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
) {
    let Some(source) = pool.broker_by_id(&source_id) else {
        let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
            "TradeCopy: Kraken source account '{}' is not connected",
            source_id
        )));
        return;
    };
    let (source_map, source_margin) = match kraken_equity_position_map(&source.broker).await {
        Ok(v) => v,
        Err(e) => {
            let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
                "TradeCopy: Kraken source positions failed: {}",
                e
            )));
            return;
        }
    };
    if source_margin > 0 {
        let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
            "TradeCopy: {} margin position(s) on {} skipped — only spot xStock holdings copy",
            source_margin, source.spec.label
        )));
    }
    let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
        "TradeCopy: source {} holds {} xStock position(s)",
        source.spec.label,
        source_map.len()
    )));
    // Live catalog as a doomed-order guard; an unreachable catalog degrades to
    // the constructed pair (AddOrder still validates server-side).
    let catalog: Option<Vec<String>> =
        source.broker.get_tradeable_pairs().await.ok().map(|pairs| {
            pairs
                .into_iter()
                .flat_map(|(name, wsname)| [name, wsname])
                .map(|p| p.trim().to_ascii_uppercase())
                .collect()
        });
    let pair_listed = |pair: &str| -> bool {
        match &catalog {
            Some(entries) if !entries.is_empty() => {
                let want = pair.to_ascii_uppercase();
                entries.iter().any(|p| p == &want)
            }
            _ => true,
        }
    };

    for target_id in target_ids {
        if target_id == source_id {
            continue;
        }
        let Some(target) = pool.broker_by_id(&target_id) else {
            let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
                "TradeCopy: Kraken target '{}' is not connected — skipped",
                target_id
            )));
            continue;
        };
        if !target.spec.trade_enabled {
            let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
                "TradeCopy: Kraken target '{}' is not trade-enabled — skipped",
                target.spec.label
            )));
            continue;
        }
        let target_map = match kraken_equity_position_map(&target.broker).await {
            Ok((m, _)) => m,
            Err(e) => {
                let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
                    "TradeCopy: {} positions failed: {} — skipped",
                    target.spec.label, e
                )));
                continue;
            }
        };
        let deltas = trade_copy_deltas(&source_map, &target_map, flatten_extra);
        if deltas.is_empty() {
            let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                "TradeCopy → {}: already in sync",
                target.spec.label
            )));
            continue;
        }
        let mut placed = 0usize;
        let mut failed = 0usize;
        for (ticker, side, qty) in deltas {
            let pair = kraken_xstock_order_pair(&ticker);
            if !pair_listed(&pair) {
                failed += 1;
                let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
                    "TradeCopy → {}: {} has no tradeable Spot pair ({}) — skipped",
                    target.spec.label, ticker, pair
                )));
                continue;
            }
            match target
                .broker
                .place_order(&pair, &side, "market", qty, None)
                .await
            {
                Ok(_) => {
                    placed += 1;
                    let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                        "TradeCopy → {}: {} {} {} @ market",
                        target.spec.label, side, qty, pair
                    )));
                }
                Err(e) => {
                    failed += 1;
                    let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
                        "TradeCopy → {}: {} {} {} failed: {}",
                        target.spec.label, side, qty, pair, e
                    )));
                }
            }
        }
        let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
            "TradeCopy → {}: {} order(s) placed, {} failed",
            target.spec.label, placed, failed
        )));
    }
}

pub async fn handle_kraken_order_command(
    cmd: BrokerCmd,
    kraken_broker: Option<&typhoon_engine::broker::kraken::KrakenBroker>,
    broker_msg_tx: &tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
) {
    let Some(kb) = kraken_broker else {
        let _ = broker_msg_tx.send(BrokerMsg::Error("Kraken: not connected".into()));
        return;
    };

    match cmd {
        BrokerCmd::KrakenSyncExits {
            pair,
            sl_price,
            tp_price,
            wait_for_position,
            wait_for_qty_at_most,
        } => {
            if wait_for_position || wait_for_qty_at_most.is_some() {
                let mut found = false;
                for _ in 0..12 {
                    match kb.get_position_summaries().await {
                        Ok(positions)
                            if positions.iter().any(|p| {
                                p.symbol.eq_ignore_ascii_case(&pair)
                                    && p.qty.abs() > 0.0
                                    && wait_for_qty_at_most
                                        .map(|max_qty| p.qty.abs() <= max_qty + 1e-8)
                                        .unwrap_or(true)
                            }) =>
                        {
                            found = true;
                            break;
                        }
                        Ok(_) => {
                            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        }
                        Err(e) => {
                            let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
                                "Kraken exit sync {}: position poll failed: {}",
                                pair, e
                            )));
                            break;
                        }
                    }
                }
                if !found {
                    let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
                        "Kraken exit sync {}: position not visible at target size yet",
                        pair
                    )));
                    return;
                }
            }
            match kb.sync_position_exits(&pair, sl_price, tp_price).await {
                Ok(summary) => {
                    let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                        "Kraken exits {}: {}",
                        pair, summary
                    )));
                }
                Err(e) => {
                    let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
                        "Kraken exit sync failed for {}: {}",
                        pair, e
                    )));
                }
            }
        }
        _ => unreachable!("non-Kraken order command routed to Kraken order handler"),
    }
}

pub async fn handle_kraken_account_order_command(
    cmd: BrokerCmd,
    kraken_broker: Option<&typhoon_engine::broker::kraken::KrakenBroker>,
    broker_msg_tx: &tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
) {
    match cmd {
        BrokerCmd::KrakenGetBalance => {
            if let Some(ref kb) = kraken_broker {
                let msg_tx = broker_msg_tx.clone();
                match kb.get_balance().await {
                    Ok(balances) => {
                        let bal_vec: Vec<(String, f64)> =
                            balances.into_iter().filter(|(_, v)| *v > 0.0).collect();
                        let _ = msg_tx.send(BrokerMsg::KrakenBalances(bal_vec));
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("Kraken balance: {}", e)));
                    }
                }
            } else {
                let _ = broker_msg_tx.send(BrokerMsg::OrderResult("Kraken: connect first".into()));
            }
        }
        BrokerCmd::KrakenGetPositions => {
            if let Some(ref kb) = kraken_broker {
                match kb.get_all_position_summaries().await {
                    Ok(pos) => {
                        let _ = broker_msg_tx.send(BrokerMsg::KrakenPositions(pos));
                    }
                    Err(e) => {
                        let _ = broker_msg_tx
                            .send(BrokerMsg::Error(format!("Kraken positions: {}", e)));
                    }
                }
            } else {
                let _ = broker_msg_tx.send(BrokerMsg::OrderResult("Kraken: connect first".into()));
            }
        }
        BrokerCmd::KrakenPlaceOrder {
            pair,
            side,
            order_type,
            volume,
            price,
            leverage,
        } => {
            if let Some(ref kb) = kraken_broker {
                let msg_tx = broker_msg_tx.clone();
                match kb
                    .place_order_with_leverage(
                        &pair,
                        &side,
                        &order_type,
                        volume,
                        price,
                        leverage.as_deref(),
                    )
                    .await
                {
                    Ok(result) => {
                        let text = serde_json::to_string_pretty(&result).unwrap_or_default();
                        let _ = msg_tx.send(BrokerMsg::OrderResult(format!(
                            "Kraken order placed: {}",
                            text
                        )));
                    }
                    Err(e) => {
                        let _ =
                            msg_tx.send(BrokerMsg::Error(format!("Kraken order failed: {}", e)));
                    }
                }
            } else {
                let _ = broker_msg_tx.send(BrokerMsg::OrderResult("Kraken: connect first".into()));
            }
        }
        BrokerCmd::KrakenPlaceOrderAdvanced { order } => {
            if let Some(ref kb) = kraken_broker {
                let msg_tx = broker_msg_tx.clone();
                match kb.place_order_request(&order).await {
                    Ok(result) => {
                        let text = serde_json::to_string_pretty(&result).unwrap_or_default();
                        let _ = msg_tx.send(BrokerMsg::OrderResult(format!(
                            "Kraken order placed: {}",
                            text
                        )));
                    }
                    Err(e) => {
                        let _ =
                            msg_tx.send(BrokerMsg::Error(format!("Kraken order failed: {}", e)));
                    }
                }
            } else {
                let _ = broker_msg_tx.send(BrokerMsg::OrderResult("Kraken: connect first".into()));
            }
        }
        BrokerCmd::KrakenClosePosition { pair, volume } => {
            if let Some(ref kb) = kraken_broker {
                match kb.close_position(&pair, volume).await {
                    Ok(result) => {
                        let text = serde_json::to_string_pretty(&result).unwrap_or_default();
                        let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                            "Kraken close {}: {}",
                            pair, text
                        )));
                    }
                    Err(e) => {
                        let _ = broker_msg_tx
                            .send(BrokerMsg::Error(format!("Kraken close {}: {}", pair, e)));
                    }
                }
            } else {
                let _ = broker_msg_tx.send(BrokerMsg::OrderResult("Kraken: connect first".into()));
            }
        }
        BrokerCmd::KrakenCancelOrder { txid } => {
            if let Some(ref kb) = kraken_broker {
                let msg_tx = broker_msg_tx.clone();
                match kb.cancel_order(&txid).await {
                    Ok(result) => {
                        let text = serde_json::to_string_pretty(&result).unwrap_or_default();
                        let _ =
                            msg_tx.send(BrokerMsg::OrderResult(format!("Kraken cancel: {}", text)));
                    }
                    Err(e) => {
                        let _ =
                            msg_tx.send(BrokerMsg::Error(format!("Kraken cancel failed: {}", e)));
                    }
                }
            } else {
                let _ = broker_msg_tx.send(BrokerMsg::OrderResult("Kraken: connect first".into()));
            }
        }
        BrokerCmd::KrakenCancelAll => {
            if let Some(ref kb) = kraken_broker {
                let msg_tx = broker_msg_tx.clone();
                match kb.cancel_all_orders().await {
                    Ok(result) => {
                        let count = result["count"].as_u64().unwrap_or(0);
                        let _ = msg_tx.send(BrokerMsg::OrderResult(format!(
                            "Kraken: cancelled {} orders",
                            count
                        )));
                    }
                    Err(e) => {
                        let _ = msg_tx
                            .send(BrokerMsg::Error(format!("Kraken cancel all failed: {}", e)));
                    }
                }
            } else {
                let _ = broker_msg_tx.send(BrokerMsg::OrderResult("Kraken: connect first".into()));
            }
        }

        BrokerCmd::KrakenFetchTrades => {
            if let Some(ref kb) = kraken_broker {
                let msg_tx = broker_msg_tx.clone();
                match kb.get_all_trades_history_parsed(None, None).await {
                    Ok(trades) => {
                        let _ = msg_tx.send(BrokerMsg::KrakenTrades(trades));
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!(
                            "Kraken trade history failed: {}",
                            e
                        )));
                    }
                }
            }
        }
        BrokerCmd::KrakenFetchOpenOrders => {
            if let Some(ref kb) = kraken_broker {
                let msg_tx = broker_msg_tx.clone();
                match kb.get_open_orders_parsed().await {
                    Ok(orders) => {
                        let _ = msg_tx.send(BrokerMsg::KrakenOpenOrders(orders));
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!(
                            "Kraken open orders failed: {}",
                            e
                        )));
                    }
                }
            }
        }
        BrokerCmd::KrakenCloseAll => {
            if let Some(ref kb) = kraken_broker {
                match kb.close_all_positions().await {
                    Ok(count) => {
                        let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                            "Kraken: closed {} position(s)",
                            count
                        )));
                    }
                    Err(e) => {
                        let _ = broker_msg_tx
                            .send(BrokerMsg::Error(format!("Kraken close all failed: {}", e)));
                    }
                }
            } else {
                let _ = broker_msg_tx.send(BrokerMsg::OrderResult("Kraken: connect first".into()));
            }
        }
        BrokerCmd::KrakenGetPairs => {
            // Public endpoint — no auth needed, create temporary broker if none
            let msg_tx = broker_msg_tx.clone();
            let kb = if let Some(ref kb) = kraken_broker {
                kb.get_tradeable_pairs().await
            } else {
                let tmp =
                    typhoon_engine::broker::kraken::KrakenBroker::new(String::new(), String::new());
                tmp.get_tradeable_pairs().await
            };
            match kb {
                Ok(pairs) => {
                    let _ = msg_tx.send(BrokerMsg::KrakenPairs(pairs));
                }
                Err(e) => {
                    let _ = msg_tx.send(BrokerMsg::Error(format!("Kraken pairs: {}", e)));
                }
            }
        }
        _ => unreachable!("non-Kraken account/order command routed to Kraken handler"),
    }
}

pub async fn fetch_and_send_all_kraken_account_positions(
    pool: &KrakenAccountPool,
    broker_msg_tx: &tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
) {
    let mut snapshots = Vec::new();
    let primary_id = pool.primary_id().map(str::to_string);

    for (_, account) in pool.connected_accounts() {
        match account.broker.get_all_position_summaries().await {
            Ok(mut positions) => {
                positions
                    .retain(|p| p.asset_class != "crypto_spot" && !p.asset_id.starts_with("spot:"));
                let is_primary = primary_id
                    .as_deref()
                    .is_some_and(|id| id == account.spec.id.as_str());
                if is_primary {
                    let _ = broker_msg_tx.send(BrokerMsg::KrakenPositions(positions.clone()));
                }
                snapshots.push(KrakenAccountPositions {
                    account_id: account.spec.id.clone(),
                    label: account.spec.label.clone(),
                    is_primary,
                    positions,
                });
            }
            Err(e) => {
                let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
                    "Kraken positions for {}: {}",
                    account.spec.label, e
                )));
            }
        }
    }

    if !snapshots.is_empty() {
        let _ = broker_msg_tx.send(BrokerMsg::KrakenAccountPositions(snapshots));
    }
}

pub async fn fetch_and_send_all_kraken_account_orders(
    pool: &KrakenAccountPool,
    broker_msg_tx: &tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
) {
    let mut snapshots = Vec::new();
    let primary_id = pool.primary_id().map(str::to_string);

    for (_, account) in pool.connected_accounts() {
        match account.broker.get_open_orders_parsed().await {
            Ok(orders) => {
                let is_primary = primary_id
                    .as_deref()
                    .is_some_and(|id| id == account.spec.id.as_str());
                if is_primary {
                    let _ = broker_msg_tx.send(BrokerMsg::KrakenOpenOrders(orders.clone()));
                }
                snapshots.push(KrakenAccountOrders {
                    account_id: account.spec.id.clone(),
                    label: account.spec.label.clone(),
                    is_primary,
                    orders,
                });
            }
            Err(e) => {
                let _ = broker_msg_tx.send(BrokerMsg::Error(format!(
                    "Kraken open orders for {}: {}",
                    account.spec.label, e
                )));
            }
        }
    }

    if !snapshots.is_empty() {
        let _ = broker_msg_tx.send(BrokerMsg::KrakenAccountOpenOrders(snapshots));
    }
}

pub async fn fetch_and_send_all_kraken_account_trades(
    pool: &KrakenAccountPool,
    broker_msg_tx: &tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
) {
    let mut snapshots = Vec::new();
    let primary_id = pool.primary_id().map(str::to_string);

    for (_, account) in pool.connected_accounts() {
        match account
            .broker
            .get_trades_history_parsed(None, None, None)
            .await
        {
            Ok(mut trades) => {
                trades.sort_by(|a, b| {
                    b.time
                        .partial_cmp(&a.time)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                trades.truncate(100);
                let is_primary = primary_id
                    .as_deref()
                    .is_some_and(|id| id == account.spec.id.as_str());
                if is_primary {
                    let _ = broker_msg_tx.send(BrokerMsg::KrakenTrades(trades.clone()));
                }
                snapshots.push(KrakenAccountTrades {
                    account_id: account.spec.id.clone(),
                    label: account.spec.label.clone(),
                    is_primary,
                    trades,
                });
            }
            Err(e) => {
                tracing::debug!("Kraken trades for {}: {}", account.spec.label, e);
            }
        }
    }

    if !snapshots.is_empty() {
        let _ = broker_msg_tx.send(BrokerMsg::KrakenAccountTrades(snapshots));
    }
}
