//! Kraken private WebSocket v1 protocol: subscription builder, message
//! parsers, and the typed `KrakenTrade` / `KrakenOrder` results.
//!
//! The WS feed delivers `ownTrades` and `openOrders` as array-wrapped frames:
//! `[{ "TXID": { ... } }, "channelName", channelId]`. Snapshot and incremental
//! updates use the same shape; callers should upsert open/pending orders and
//! drop terminal statuses (`closed`, `canceled`, `expired`).
//!
//! Numeric fields arrive as either JSON numbers or JSON strings (Kraken is
//! inconsistent), so the `kraken_ws_*` helpers accept both. The REST
//! TradesHistory parser lives here too because it reuses the same trade-shape
//! parsing logic via `kraken_trade_from_object`.

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KrakenTrade {
    pub trade_id: String,
    pub ordertxid: String,
    pub pair: String,
    pub time: f64,
    pub side: String,      // "buy" or "sell"
    pub ordertype: String, // "market", "limit", etc.
    pub price: f64,
    pub cost: f64,
    pub fee: f64,
    pub vol: f64,
    pub margin: f64,
    pub misc: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KrakenOrder {
    pub txid: String,
    pub refid: Option<String>,
    pub userref: Option<i64>,
    pub status: String,
    pub opentm: f64,
    pub starttm: Option<f64>,
    pub expiretm: Option<f64>,
    pub pair: String,
    pub r#type: String, // "buy" or "sell"
    pub ordertype: String,
    pub price: f64,
    pub price2: Option<f64>,
    pub vol: f64,
    pub vol_exec: f64,
    pub cost: f64,
    pub fee: f64,
    pub stopprice: Option<f64>,
    pub limitprice: Option<f64>,
    pub misc: Option<String>,
    pub trades: Vec<String>,
}

pub struct KrakenPrivateWs {
    pub token: String,
}

impl KrakenPrivateWs {
    pub fn new(token: String) -> Self {
        Self { token }
    }

    /// Basic subscription message for ownTrades.
    pub fn own_trades_subscription(&self) -> String {
        serde_json::json!({
            "event": "subscribe",
            "subscription": {
                "name": "ownTrades",
                "token": self.token
            }
        })
        .to_string()
    }
}

pub(super) fn kraken_ws_status_message(status: &str, message: impl Into<String>) -> String {
    serde_json::json!({
        "event": "systemStatus",
        "status": status,
        "connectionID": 0,
        "version": "TyphooN",
        "message": message.into(),
    })
    .to_string()
}

pub(super) fn parse_trades_history_result(
    resp: &serde_json::Value,
) -> Result<(Vec<KrakenTrade>, Option<u64>), String> {
    // private_post_owned already unwraps Kraken's {error,result} envelope and
    // returns the result object. Older callers/tests may still pass a full
    // envelope, so accept both shapes instead of logging a false
    // "TradesHistory missing result" error.
    let result = resp.get("result").unwrap_or(resp);

    let trades_obj = result
        .get("trades")
        .and_then(|v| v.as_object())
        .ok_or_else(|| "Kraken TradesHistory missing trades object".to_string())?;

    let mut trades = Vec::new();
    for (trade_id, trade_value) in trades_obj {
        if let Some(trade) = trade_value.as_object() {
            trades.push(kraken_trade_from_object(trade_id.clone(), trade));
        }
    }

    let count = result.get("count").and_then(|v| {
        v.as_u64()
            .or_else(|| v.as_str().and_then(|s| s.parse::<u64>().ok()))
    });

    Ok((trades, count))
}

fn kraken_trade_from_object(
    trade_id: String,
    trade: &serde_json::Map<String, serde_json::Value>,
) -> KrakenTrade {
    KrakenTrade {
        trade_id,
        ordertxid: trade
            .get("ordertxid")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        pair: trade
            .get("pair")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        time: trade
            .get("time")
            .and_then(|v| {
                v.as_f64()
                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            })
            .unwrap_or(0.0),
        side: trade
            .get("side")
            .or_else(|| trade.get("type"))
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        ordertype: trade
            .get("ordertype")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        price: trade
            .get("price")
            .and_then(|v| {
                v.as_f64()
                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            })
            .unwrap_or(0.0),
        cost: trade
            .get("cost")
            .and_then(|v| {
                v.as_f64()
                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            })
            .unwrap_or(0.0),
        fee: trade
            .get("fee")
            .and_then(|v| {
                v.as_f64()
                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            })
            .unwrap_or(0.0),
        vol: trade
            .get("vol")
            .and_then(|v| {
                v.as_f64()
                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            })
            .unwrap_or(0.0),
        margin: trade
            .get("margin")
            .and_then(|v| {
                v.as_f64()
                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            })
            .unwrap_or(0.0),
        misc: trade
            .get("misc")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    }
}

/// Parse all Kraken WebSocket `ownTrades` entries in a message.
pub fn parse_own_trades_messages(msg: &serde_json::Value) -> Vec<KrakenTrade> {
    let Some(arr) = msg.as_array() else {
        return Vec::new();
    };
    if !arr.iter().any(|v| v.as_str() == Some("ownTrades")) {
        return Vec::new();
    }
    let Some(obj) = arr.first().and_then(|v| v.as_object()) else {
        return Vec::new();
    };

    obj.iter()
        .filter_map(|(trade_id, trade_val)| {
            trade_val
                .as_object()
                .map(|trade| kraken_trade_from_object(trade_id.clone(), trade))
        })
        .collect()
}

/// Attempt to parse a Kraken WebSocket ownTrades message into one KrakenTrade.
pub fn parse_own_trades_message(msg: &serde_json::Value) -> Option<KrakenTrade> {
    parse_own_trades_messages(msg).into_iter().next()
}

fn kraken_ws_f64(obj: &serde_json::Map<String, serde_json::Value>, key: &str) -> f64 {
    obj.get(key)
        .and_then(|v| {
            v.as_f64()
                .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        })
        .unwrap_or(0.0)
}

fn kraken_ws_opt_f64(obj: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<f64> {
    obj.get(key).and_then(|v| {
        v.as_f64()
            .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
    })
}

fn kraken_ws_string(obj: &serde_json::Map<String, serde_json::Value>, key: &str) -> String {
    obj.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

pub(super) fn kraken_order_from_object(
    txid: &str,
    order: &serde_json::Map<String, serde_json::Value>,
) -> KrakenOrder {
    let descr = order.get("descr").and_then(|v| v.as_object());
    let descr_string = |key: &str| -> String {
        descr
            .and_then(|d| d.get(key))
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string()
    };
    let descr_f64 = |key: &str| -> f64 {
        descr
            .and_then(|d| d.get(key))
            .and_then(|v| {
                v.as_f64()
                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            })
            .unwrap_or(0.0)
    };
    let descr_opt_f64 = |key: &str| -> Option<f64> {
        descr.and_then(|d| d.get(key)).and_then(|v| {
            v.as_f64()
                .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        })
    };
    let trades = order
        .get("trades")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    KrakenOrder {
        txid: txid.to_string(),
        refid: order
            .get("refid")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        userref: order.get("userref").and_then(|v| v.as_i64()),
        status: kraken_ws_string(order, "status"),
        opentm: kraken_ws_f64(order, "opentm"),
        starttm: kraken_ws_opt_f64(order, "starttm"),
        expiretm: kraken_ws_opt_f64(order, "expiretm"),
        pair: descr_string("pair"),
        r#type: descr_string("type"),
        ordertype: descr_string("ordertype"),
        price: descr_f64("price"),
        price2: descr_opt_f64("price2"),
        vol: kraken_ws_f64(order, "vol"),
        vol_exec: kraken_ws_f64(order, "vol_exec"),
        cost: kraken_ws_f64(order, "cost"),
        fee: kraken_ws_f64(order, "fee"),
        stopprice: kraken_ws_opt_f64(order, "stopprice"),
        limitprice: kraken_ws_opt_f64(order, "limitprice"),
        misc: order
            .get("misc")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        trades,
    }
}

/// Parse Kraken private WebSocket `openOrders` messages into typed orders.
///
/// Kraken v1 private messages are array-wrapped and commonly arrive as:
/// `[ { "ORDER_TXID": { "status": "open", "descr": { ... }, ... } }, "openOrders", <channel_id> ]`.
/// Snapshot and incremental updates use the same shape; callers should upsert
/// open/pending orders and remove terminal statuses (`closed`, `canceled`, `expired`).
pub fn parse_open_orders_message(msg: &serde_json::Value) -> Vec<KrakenOrder> {
    let Some(arr) = msg.as_array() else {
        return Vec::new();
    };
    if !arr.iter().any(|v| v.as_str() == Some("openOrders")) {
        return Vec::new();
    }
    let Some(orders_obj) = arr.first().and_then(|v| v.as_object()) else {
        return Vec::new();
    };

    orders_obj
        .iter()
        .filter_map(|(txid, order_val)| {
            order_val
                .as_object()
                .map(|order| kraken_order_from_object(txid, order))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_batched_own_trades_ws_message() {
        let msg = serde_json::json!([
            {
                "T1": {
                    "ordertxid": "O1",
                    "pair": "XXBTZUSD",
                    "time": "1700000001.5",
                    "side": "buy",
                    "ordertype": "limit",
                    "price": "35000.0",
                    "cost": "3500.0",
                    "fee": "1.0",
                    "vol": "0.1",
                    "margin": "0.0"
                },
                "T2": {
                    "ordertxid": "O2",
                    "pair": "XETHZUSD",
                    "time": 1700000002.5,
                    "side": "sell",
                    "ordertype": "market",
                    "price": "2500.0",
                    "cost": "500.0",
                    "fee": "0.5",
                    "vol": "0.2",
                    "margin": "0.0"
                }
            },
            "ownTrades",
            8
        ]);
        let trades = parse_own_trades_messages(&msg);
        assert_eq!(trades.len(), 2);
        assert!(
            trades
                .iter()
                .any(|t| t.trade_id == "T1" && t.ordertxid == "O1")
        );
        assert!(
            trades
                .iter()
                .any(|t| t.trade_id == "T2" && t.ordertxid == "O2")
        );
    }

    #[test]
    fn parse_own_trades_message_returns_first_when_present() {
        let msg = serde_json::json!([
            { "T1": { "ordertxid": "O1", "pair": "XXBTZUSD", "time": 1.0, "side": "buy",
                      "ordertype": "limit", "price": "1", "cost": "1", "fee": "0",
                      "vol": "1", "margin": "0" } },
            "ownTrades",
            8
        ]);
        let trade = parse_own_trades_message(&msg).expect("expected at least one trade");
        assert_eq!(trade.trade_id, "T1");
    }

    #[test]
    fn parse_own_trades_messages_ignores_non_owntrades_frames() {
        let msg = serde_json::json!([{}, "openOrders", 7]);
        assert!(parse_own_trades_messages(&msg).is_empty());
    }

    #[test]
    fn parse_open_orders_ws_message() {
        let msg = serde_json::json!([
            {
                "OABCDEF-GHIJK-LMNOPQ": {
                    "refid": null,
                    "userref": 42,
                    "status": "open",
                    "opentm": 1700000000.123,
                    "starttm": 0,
                    "expiretm": 0,
                    "descr": {
                        "pair": "XXBTZUSD",
                        "type": "buy",
                        "ordertype": "limit",
                        "price": "35000.0",
                        "price2": "0"
                    },
                    "vol": "0.25",
                    "vol_exec": "0.10",
                    "cost": "3500.0",
                    "fee": "1.2",
                    "misc": ""
                }
            },
            "openOrders",
            7
        ]);
        let orders = parse_open_orders_message(&msg);
        assert_eq!(orders.len(), 1);
        let order = &orders[0];
        assert_eq!(order.txid, "OABCDEF-GHIJK-LMNOPQ");
        assert_eq!(order.userref, Some(42));
        assert_eq!(order.status, "open");
        assert_eq!(order.pair, "XXBTZUSD");
        assert_eq!(order.r#type, "buy");
        assert_eq!(order.ordertype, "limit");
        assert!((order.price - 35000.0).abs() < 1e-9);
        assert!((order.vol - 0.25).abs() < 1e-9);
        assert!((order.vol_exec - 0.10).abs() < 1e-9);
    }

    #[test]
    fn parse_open_orders_message_ignores_non_open_orders_frame() {
        let msg = serde_json::json!([{}, "ownTrades", 8]);
        assert!(parse_open_orders_message(&msg).is_empty());
    }

    #[test]
    fn parse_trades_history_result_accepts_bare_and_wrapped_result() {
        let bare = serde_json::json!({
            "trades": {
                "T1": { "ordertxid": "O1", "pair": "XXBTZUSD", "time": 1.0,
                        "side": "buy", "ordertype": "limit",
                        "price": "1", "cost": "1", "fee": "0",
                        "vol": "1", "margin": "0" }
            },
            "count": 1
        });
        let wrapped = serde_json::json!({ "result": bare.clone() });

        let (b, count_b) = parse_trades_history_result(&bare).unwrap();
        let (w, count_w) = parse_trades_history_result(&wrapped).unwrap();
        assert_eq!(b.len(), 1);
        assert_eq!(w.len(), 1);
        assert_eq!(count_b, Some(1));
        assert_eq!(count_w, Some(1));
    }

    #[test]
    fn own_trades_subscription_includes_token() {
        let ws = KrakenPrivateWs::new("TOKEN-123".into());
        let body = ws.own_trades_subscription();
        assert!(body.contains("TOKEN-123"));
        assert!(body.contains("ownTrades"));
    }

    #[test]
    fn kraken_ws_status_message_uses_typhoon_version() {
        let msg = kraken_ws_status_message("online", "hello");
        assert!(msg.contains("TyphooN"));
        assert!(msg.contains("online"));
        assert!(msg.contains("hello"));
    }
}
