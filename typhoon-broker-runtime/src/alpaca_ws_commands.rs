//! Alpaca trading WebSocket (`trade_updates`) command handling.
//!
//! Replaces REST-polling latency for order/fill state: on every `trade_updates`
//! event the broker re-pulls positions/orders/account and emits the existing
//! `BrokerMsg`s, so the trading panel reflects a fill the instant Alpaca reports
//! it. The periodic REST poll stays as a safety net for anything the socket
//! misses (drops, the brief reconnect window).

use typhoon_engine::broker::alpaca::AlpacaBroker;
use typhoon_engine::broker::protocol::{BrokerCmd, BrokerMsg};

pub async fn handle_alpaca_ws_command(
    cmd: BrokerCmd,
    broker: Option<AlpacaBroker>,
    broker_msg_tx: &tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
) {
    if !matches!(cmd, BrokerCmd::AlpacaStartTradeStream) {
        return;
    }
    let Some(b) = broker else {
        return;
    };
    match b.start_trade_updates_ws().await {
        Ok(mut rx) => {
            let _ = broker_msg_tx.send(BrokerMsg::OrderResult(
                "Alpaca trade stream connected — real-time fills/orders".into(),
            ));
            let tx = broker_msg_tx.clone();
            tokio::spawn(async move {
                while let Some(raw) = rx.recv().await {
                    let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) else {
                        continue;
                    };
                    match v.get("stream").and_then(|s| s.as_str()).unwrap_or("") {
                        "trade_updates" => {
                            log_trade_update(&v, &tx);
                            refresh_account_state(&b, &tx).await;
                        }
                        "authorization" => {
                            if v.pointer("/data/status").and_then(|s| s.as_str())
                                == Some("unauthorized")
                            {
                                let _ = tx.send(BrokerMsg::Error(
                                    "Alpaca trade stream rejected: unauthorized".into(),
                                ));
                            }
                        }
                        _ => {}
                    }
                }
                tracing::info!("Alpaca trade-stream forwarder ended");
            });
        }
        Err(e) => {
            let _ = broker_msg_tx.send(BrokerMsg::Error(format!("Alpaca trade stream failed: {e}")));
        }
    }
}

/// Surface the event as a concise log line (a fill carries price/qty; lifecycle
/// events like new/canceled/replaced carry just the order).
fn log_trade_update(v: &serde_json::Value, tx: &tokio::sync::mpsc::UnboundedSender<BrokerMsg>) {
    if let Some(data) = v.get("data") {
        let _ = tx.send(BrokerMsg::OrderResult(trade_update_log_line(data)));
    }
}

/// Pure formatter for a `trade_updates` `data` object.
fn trade_update_log_line(data: &serde_json::Value) -> String {
    let event = data.get("event").and_then(|e| e.as_str()).unwrap_or("update");
    let order = data.get("order");
    let symbol = order
        .and_then(|o| o.get("symbol"))
        .and_then(|s| s.as_str())
        .unwrap_or("?");
    let side = order
        .and_then(|o| o.get("side"))
        .and_then(|s| s.as_str())
        .unwrap_or("");
    let price = data.get("price").and_then(|p| p.as_str());
    let qty = data.get("qty").and_then(|q| q.as_str());
    match (event, price, qty) {
        ("fill" | "partial_fill", Some(p), Some(q)) => {
            format!("Alpaca {event}: {side} {q} {symbol} @ {p}")
        }
        _ => format!("Alpaca order {event}: {side} {symbol}"),
    }
}

/// Re-pull authoritative positions/orders/account and emit the existing
/// messages so the UI updates immediately on a fill.
async fn refresh_account_state(b: &AlpacaBroker, tx: &tokio::sync::mpsc::UnboundedSender<BrokerMsg>) {
    if let Ok(pos) = b.get_positions().await {
        let _ = tx.send(BrokerMsg::Positions(pos));
    }
    if let Ok(orders) = b.get_orders("open", 100).await {
        let _ = tx.send(BrokerMsg::Orders(orders));
    }
    if let Ok(acct) = b.get_account().await {
        let _ = tx.send(BrokerMsg::Account(acct));
    }
}

/// Start the Alpaca market-data WebSocket (auto-detected SIP/IEX), spawn the
/// quote forwarder, and return the control sender the processor uses to push the
/// live subscription set. `None` if the stream couldn't be opened.
pub async fn start_alpaca_quote_stream(
    b: AlpacaBroker,
    broker_msg_tx: &tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
) -> Option<tokio::sync::mpsc::Sender<Vec<String>>> {
    match b.start_market_data_ws().await {
        Ok((mut rx, control, feed)) => {
            let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                "Alpaca market-data stream connected ({})",
                feed.to_uppercase()
            )));
            let tx = broker_msg_tx.clone();
            tokio::spawn(async move {
                while let Some(raw) = rx.recv().await {
                    for (sym, bid, ask) in parse_market_data_quotes(&raw) {
                        let _ = tx.send(BrokerMsg::AlpacaQuote(sym, bid, ask));
                    }
                }
                tracing::info!("Alpaca data-stream forwarder ended");
            });
            Some(control)
        }
        Err(e) => {
            let _ = broker_msg_tx
                .send(BrokerMsg::Error(format!("Alpaca market-data stream failed: {e}")));
            None
        }
    }
}

/// Parse a decoded market-data frame into `(symbol, bid, ask)` ticks. A quote
/// (`T="q"`) carries bid/ask; a trade (`T="t"`) is delivered as bid==ask==last.
/// Messages arrive as arrays (sometimes a single object).
pub fn parse_market_data_quotes(raw: &str) -> Vec<(String, f64, f64)> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(raw) else {
        return Vec::new();
    };
    let items = match v {
        serde_json::Value::Array(a) => a,
        other => vec![other],
    };
    let mut out = Vec::new();
    for item in &items {
        match item.get("T").and_then(|t| t.as_str()).unwrap_or("") {
            "q" => {
                let sym = item.get("S").and_then(|s| s.as_str());
                let bid = item.get("bp").and_then(serde_json::Value::as_f64);
                let ask = item.get("ap").and_then(serde_json::Value::as_f64);
                if let (Some(sym), Some(bid), Some(ask)) = (sym, bid, ask) {
                    if bid > 0.0 && ask > 0.0 {
                        out.push((sym.to_string(), bid, ask));
                    }
                }
            }
            "t" => {
                let sym = item.get("S").and_then(|s| s.as_str());
                let price = item.get("p").and_then(serde_json::Value::as_f64);
                if let (Some(sym), Some(price)) = (sym, price) {
                    if price > 0.0 {
                        out.push((sym.to_string(), price, price));
                    }
                }
            }
            "error" => tracing::warn!(
                "Alpaca data-stream error: {}",
                item.get("msg").and_then(|m| m.as_str()).unwrap_or("")
            ),
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::trade_update_log_line;

    #[test]
    fn fill_event_includes_side_qty_symbol_price() {
        let data = serde_json::json!({
            "event": "fill",
            "price": "0.2900",
            "qty": "8",
            "order": { "symbol": "HKIT", "side": "buy" }
        });
        assert_eq!(
            trade_update_log_line(&data),
            "Alpaca fill: buy 8 HKIT @ 0.2900"
        );
    }

    #[test]
    fn lifecycle_event_without_price_falls_back_to_order_line() {
        let data = serde_json::json!({
            "event": "new",
            "order": { "symbol": "HKIT", "side": "sell" }
        });
        assert_eq!(trade_update_log_line(&data), "Alpaca order new: sell HKIT");
    }

    #[test]
    fn parses_quote_and_trade_ticks_and_skips_control_frames() {
        use super::parse_market_data_quotes;
        // Quote ⇒ (sym, bid, ask).
        assert_eq!(
            parse_market_data_quotes(r#"[{"T":"q","S":"HKIT","bp":0.28,"ap":0.29}]"#),
            vec![("HKIT".to_string(), 0.28, 0.29)]
        );
        // Trade ⇒ bid==ask==last.
        assert_eq!(
            parse_market_data_quotes(r#"[{"T":"t","S":"HKIT","p":0.285}]"#),
            vec![("HKIT".to_string(), 0.285, 0.285)]
        );
        // Control/handshake frames produce no ticks.
        assert!(parse_market_data_quotes(r#"[{"T":"success","msg":"authenticated"}]"#).is_empty());
        // Zero/missing prices are dropped; a valid sibling still parses.
        assert_eq!(
            parse_market_data_quotes(
                r#"[{"T":"q","S":"A","bp":0,"ap":1.0},{"T":"q","S":"B","bp":2.0,"ap":2.1}]"#
            ),
            vec![("B".to_string(), 2.0, 2.1)]
        );
    }
}
