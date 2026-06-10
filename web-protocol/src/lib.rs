use serde::{Deserialize, Serialize};

/// Maximum symbol length (e.g. "XAUUSD.a" = 8 chars, allow up to 20).
pub const MAX_SYMBOL_LEN: usize = 20;
/// Maximum timeframe length (e.g. "1Month" = 6 chars).
pub const MAX_TIMEFRAME_LEN: usize = 10;
/// Maximum number of watchlist symbols per request.
pub const MAX_WATCHLIST_SYMBOLS: usize = 100;

/// Validate a symbol string: alphanumeric + dots + slashes only, bounded length, no path traversal.
pub fn is_valid_symbol(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= MAX_SYMBOL_LEN
        && !s.contains("..")
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '/' || c == '_')
}

/// Validate a timeframe string: alphanumeric only, bounded length.
pub fn is_valid_timeframe(s: &str) -> bool {
    !s.is_empty() && s.len() <= MAX_TIMEFRAME_LEN && s.chars().all(|c| c.is_ascii_alphanumeric())
}

/// Maximum order quantity (prevents typos like 1000000 lots).
pub const MAX_ORDER_QTY: f64 = 100_000.0;
/// Maximum indicator names per request.
pub const MAX_INDICATOR_NAMES: usize = 50;
/// Maximum alert message length.
pub const MAX_ALERT_MSG_LEN: usize = 256;
/// Maximum news articles returned.
pub const MAX_NEWS_ITEMS: usize = 50;

/// Validate an order side string.
pub fn is_valid_order_side(s: &str) -> bool {
    matches!(s, "buy" | "sell" | "BUY" | "SELL" | "Buy" | "Sell")
}

/// Validate an order type string.
pub fn is_valid_order_type(s: &str) -> bool {
    let normalized = s.trim().replace('-', "_").to_ascii_lowercase();
    matches!(
        normalized.as_str(),
        "market"
            | "limit"
            | "stop"
            | "stoplimit"
            | "stop_limit"
            | "trailing_stop"
            | "stoploss"
            | "stop_loss"
            | "stoploss_limit"
            | "stop_loss_limit"
            | "takeprofit"
            | "take_profit"
            | "takeprofit_limit"
            | "take_profit_limit"
            | "trailingstop"
            | "trailingstop_limit"
            | "trailing_stop_limit"
            | "iceberg"
            | "settle_position"
    )
}

/// Validate an order qty: positive, finite, bounded.
pub fn is_valid_order_qty(q: f64) -> bool {
    q.is_finite() && q > 0.0 && q <= MAX_ORDER_QTY
}

/// Validate broker names accepted for order routing.
pub fn is_valid_order_broker(s: &str) -> bool {
    matches!(
        s.to_ascii_lowercase().as_str(),
        "alpaca" | "tastytrade" | "kraken"
    )
}

/// Validate a risk mode string.
pub fn is_valid_risk_mode(s: &str) -> bool {
    matches!(
        s,
        "standard" | "fixed" | "dynamic" | "var" | "Standard" | "Fixed" | "Dynamic" | "VaR" | "VAR"
    )
}

/// Validate an alert condition string.
pub fn is_valid_alert_condition(s: &str) -> bool {
    matches!(
        s,
        "crosses_above" | "crosses_below" | "reaches" | "breaks_above" | "breaks_below"
    )
}

/// Validate an indicator name: alphanumeric + underscores, bounded.
pub fn is_valid_indicator_name(s: &str) -> bool {
    !s.is_empty() && s.len() <= 32 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Validate an alert ID: alphanumeric + dashes + underscores, bounded.
pub fn is_valid_alert_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

// ── Client → Server ─────────────────────────────────────────────────
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum WebCmd {
    /// First message must be Auth with passphrase.
    Auth {
        passphrase: String,
    },
    GetAccount,
    GetPositions,
    GetOrders,
    GetWatchlistQuotes {
        symbols: Vec<String>,
    },
    GetBars {
        symbol: String,
        timeframe: String,
    },
    GetMarketClock,
    Ping,

    // ── Phase 2: order entry from phone (ADR-073 follow-up) ──
    /// Place a new equity order.
    /// Server validates: symbol format, side, type, qty bounds.
    /// Server rejects the entire command if any field fails validation.
    PlaceOrder {
        symbol: String,
        qty: f64,
        side: String,       // "buy" | "sell"
        order_type: String, // common order type or broker-specific Kraken variant
        limit_price: Option<f64>,
        stop_price: Option<f64>,
        broker: String, // "alpaca" | "tastytrade" | "kraken"
        // ── ADR-092: bracket + trailing + risk mode extensions ──
        take_profit: Option<f64>,
        stop_loss: Option<f64>,
        trail_percent: Option<f64>,
        trail_offset: Option<f64>,
        risk_mode: Option<String>, // "standard" | "fixed" | "dynamic" | "var"
        risk_pct: Option<f64>,
    },
    /// Cancel an open order by broker order ID.
    CancelOrder {
        order_id: String,
        broker: String,
    },
    /// Close an open position at market.
    ClosePosition {
        symbol: String,
        broker: String,
    },

    // ── ADR-092: server-computed indicators ──
    /// Request indicator values computed on the server (GPU).
    GetIndicators {
        symbol: String,
        timeframe: String,
        indicators: Vec<String>, // e.g. ["SMA_200", "RSI_14", "EMA_21"]
    },

    // ── ADR-092: alerts ──
    CreateAlert {
        symbol: String,
        condition: String, // "crosses_above" | "crosses_below" | "reaches"
        price: f64,
        message: String,
    },
    DeleteAlert {
        alert_id: String,
    },
    ListAlerts,

    // ── ADR-092: news ──
    GetNews {
        symbol: Option<String>,
    },

    // ── ADR-092: subscribe to push updates for a symbol ──
    Subscribe {
        symbol: String,
        timeframe: String,
    },
    Unsubscribe {
        symbol: String,
        timeframe: String,
    },
}

// ── Server → Client ─────────────────────────────────────────────────
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum WebMsg {
    /// Authentication result.
    AuthResult {
        ok: bool,
    },
    Account(AccountSnapshot),
    Positions {
        items: Vec<PositionSnapshot>,
    },
    Orders {
        items: Vec<OrderSnapshot>,
    },
    WatchlistQuotes {
        items: Vec<QuoteSnapshot>,
    },
    Bars {
        symbol: String,
        timeframe: String,
        bars: Vec<BarData>,
    },
    MarketClock {
        info: String,
    },
    QuoteTick {
        symbol: String,
        bid: f64,
        ask: f64,
    },
    /// Reply to PlaceOrder / CancelOrder / ClosePosition. Non-error feedback.
    OrderResult {
        ok: bool,
        message: String,
    },
    Error {
        msg: String,
    },
    Pong,

    // ── ADR-092: push updates ──
    /// Real-time bar update for subscribed symbol/timeframe.
    BarUpdate {
        symbol: String,
        timeframe: String,
        bar: BarData,
    },
    /// Pushed when positions change (fill, close, etc.).
    PositionUpdate {
        items: Vec<PositionSnapshot>,
    },
    /// Pushed when account snapshot changes.
    AccountUpdate(AccountSnapshot),

    // ── ADR-092: server-computed indicators ──
    IndicatorData {
        symbol: String,
        timeframe: String,
        name: String,
        values: Vec<Option<f64>>,
    },

    // ── ADR-092: alerts ──
    AlertTriggered {
        alert_id: String,
        symbol: String,
        message: String,
    },
    AlertList {
        items: Vec<AlertSnapshot>,
    },

    // ── ADR-092: news ──
    NewsFeed {
        items: Vec<NewsItem>,
    },
}

// ── Snapshot types ──────────────────────────────────────────────────
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct AccountSnapshot {
    pub equity: f64,
    pub cash: f64,
    pub buying_power: f64,
    pub portfolio_value: f64,
    pub unrealized_pl: f64,
    pub initial_margin: f64,
    pub maintenance_margin: f64,
    pub currency: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct PositionSnapshot {
    pub symbol: String,
    pub qty: f64,
    pub side: String,
    pub avg_entry_price: f64,
    pub market_value: f64,
    pub unrealized_pl: f64,
    pub asset_class: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct OrderSnapshot {
    pub id: String,
    pub symbol: String,
    pub qty: String,
    pub side: String,
    pub order_type: String,
    pub status: String,
    pub limit_price: Option<String>,
    pub stop_price: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct QuoteSnapshot {
    pub symbol: String,
    pub last: f64,
    pub bid: f64,
    pub ask: f64,
    pub change_pct: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct BarData {
    pub timestamp: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

// ── ADR-092: alert + news snapshot types ────────────────────────────
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct AlertSnapshot {
    pub id: String,
    pub symbol: String,
    pub condition: String,
    pub price: f64,
    pub message: String,
    pub active: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct NewsItem {
    pub headline: String,
    pub source: String,
    pub url: String,
    pub symbol: Option<String>,
    pub timestamp: i64,
    pub summary: String,
}

// ── Tests ───────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn webcmd_serde_roundtrip() {
        let cmds = vec![
            WebCmd::Auth {
                passphrase: "test123".into(),
            },
            WebCmd::GetAccount,
            WebCmd::GetPositions,
            WebCmd::GetOrders,
            WebCmd::GetWatchlistQuotes {
                symbols: vec!["AAPL".into(), "MSFT".into()],
            },
            WebCmd::GetBars {
                symbol: "XAUUSD".into(),
                timeframe: "1Day".into(),
            },
            WebCmd::GetMarketClock,
            WebCmd::Ping,
        ];
        for cmd in cmds {
            let json = serde_json::to_string(&cmd).unwrap();
            let back: WebCmd = serde_json::from_str(&json).unwrap();
            assert_eq!(format!("{back:?}"), format!("{cmd:?}"));
        }
    }

    #[test]
    fn webmsg_serde_roundtrip() {
        let msgs = vec![
            WebMsg::AuthResult { ok: true },
            WebMsg::Account(AccountSnapshot {
                equity: 10000.0,
                cash: 5000.0,
                buying_power: 20000.0,
                portfolio_value: 10000.0,
                unrealized_pl: 500.0,
                initial_margin: 2000.0,
                maintenance_margin: 1000.0,
                currency: "USD".into(),
            }),
            WebMsg::Positions {
                items: vec![PositionSnapshot {
                    symbol: "AAPL".into(),
                    qty: 10.0,
                    side: "long".into(),
                    avg_entry_price: 150.0,
                    market_value: 1600.0,
                    unrealized_pl: 100.0,
                    asset_class: "us_equity".into(),
                }],
            },
            WebMsg::Pong,
            WebMsg::Error {
                msg: "test error".into(),
            },
        ];
        for msg in msgs {
            let json = serde_json::to_string(&msg).unwrap();
            let back: WebMsg = serde_json::from_str(&json).unwrap();
            assert_eq!(format!("{back:?}"), format!("{msg:?}"));
        }
    }

    #[test]
    fn deny_unknown_fields_webcmd_struct_variant() {
        // Struct variants with deny_unknown_fields reject extra fields
        let json = r#"{"type":"GetBars","symbol":"AAPL","timeframe":"1Day","admin":true}"#;
        assert!(serde_json::from_str::<WebCmd>(json).is_err());
    }

    #[test]
    fn deny_unknown_fields_snapshot() {
        // Snapshot structs reject unknown fields
        let json = r#"{"equity":1.0,"cash":1.0,"buying_power":1.0,"portfolio_value":1.0,"unrealized_pl":0.0,"initial_margin":0.0,"maintenance_margin":0.0,"currency":"USD","admin":true}"#;
        assert!(serde_json::from_str::<AccountSnapshot>(json).is_err());
    }

    #[test]
    fn invalid_type_tag_rejected() {
        let json = r#"{"type":"DropTable"}"#;
        assert!(serde_json::from_str::<WebCmd>(json).is_err());
    }

    #[test]
    fn symbol_validation() {
        assert!(is_valid_symbol("AAPL"));
        assert!(is_valid_symbol("XAUUSD.a"));
        assert!(is_valid_symbol("BTC/USD"));
        assert!(is_valid_symbol("US500_m"));
        assert!(!is_valid_symbol(""));
        assert!(!is_valid_symbol("A".repeat(21).as_str()));
        assert!(!is_valid_symbol("../../etc/passwd"));
        assert!(!is_valid_symbol("AAPL; DROP TABLE"));
        assert!(!is_valid_symbol("SYM\nINJECT"));
    }

    #[test]
    fn timeframe_validation() {
        assert!(is_valid_timeframe("1Day"));
        assert!(is_valid_timeframe("1Hour"));
        assert!(is_valid_timeframe("5Min"));
        assert!(!is_valid_timeframe(""));
        assert!(!is_valid_timeframe("A".repeat(11).as_str()));
        assert!(!is_valid_timeframe("1Day; rm -rf"));
    }

    #[test]
    fn symbol_boundary_length() {
        // Exactly MAX_SYMBOL_LEN (20) should pass
        assert!(is_valid_symbol("A".repeat(MAX_SYMBOL_LEN).as_str()));
        // MAX_SYMBOL_LEN + 1 should fail
        assert!(!is_valid_symbol("A".repeat(MAX_SYMBOL_LEN + 1).as_str()));
    }

    #[test]
    fn timeframe_boundary_length() {
        assert!(is_valid_timeframe("A".repeat(MAX_TIMEFRAME_LEN).as_str()));
        assert!(!is_valid_timeframe(
            "A".repeat(MAX_TIMEFRAME_LEN + 1).as_str()
        ));
    }

    #[test]
    fn symbol_rejects_unicode() {
        assert!(!is_valid_symbol("AAPL\u{200B}")); // zero-width space
        assert!(!is_valid_symbol("A\u{00E9}PL")); // accented e
    }

    #[test]
    fn symbol_rejects_null_bytes() {
        assert!(!is_valid_symbol("AA\0PL"));
    }

    #[test]
    fn watchlist_symbol_count_limit() {
        assert!(MAX_WATCHLIST_SYMBOLS == 100);
    }

    #[test]
    fn auth_cmd_roundtrip() {
        let cmd = WebCmd::Auth {
            passphrase: "s3cr3t!@#$%".into(),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let back: WebCmd = serde_json::from_str(&json).unwrap();
        match back {
            WebCmd::Auth { passphrase } => assert_eq!(passphrase, "s3cr3t!@#$%"),
            _ => panic!("Expected Auth"),
        }
    }

    #[test]
    fn auth_result_roundtrip() {
        for ok in [true, false] {
            let msg = WebMsg::AuthResult { ok };
            let json = serde_json::to_string(&msg).unwrap();
            let back: WebMsg = serde_json::from_str(&json).unwrap();
            match back {
                WebMsg::AuthResult { ok: v } => assert_eq!(v, ok),
                _ => panic!("Expected AuthResult"),
            }
        }
    }

    #[test]
    fn order_side_validation() {
        assert!(is_valid_order_side("buy"));
        assert!(is_valid_order_side("sell"));
        assert!(is_valid_order_side("Buy"));
        assert!(is_valid_order_side("SELL"));
        assert!(!is_valid_order_side("purchase"));
        assert!(!is_valid_order_side(""));
        assert!(!is_valid_order_side("buy; DROP"));
    }

    #[test]
    fn order_type_validation() {
        assert!(is_valid_order_type("market"));
        assert!(is_valid_order_type("limit"));
        assert!(is_valid_order_type("stop"));
        assert!(is_valid_order_type("stop_limit"));
        assert!(is_valid_order_type("stop-loss-limit"));
        assert!(is_valid_order_type("take_profit"));
        assert!(is_valid_order_type("trailing-stop-limit"));
        assert!(is_valid_order_type("iceberg"));
        assert!(is_valid_order_type("MARKET"));
        assert!(!is_valid_order_type(""));
        assert!(!is_valid_order_type("asap"));
    }

    #[test]
    fn order_broker_validation_accepts_kraken() {
        assert!(is_valid_order_broker("alpaca"));
        assert!(is_valid_order_broker("tastytrade"));
        assert!(is_valid_order_broker("kraken"));
        assert!(is_valid_order_broker("Kraken"));
        assert!(!is_valid_order_broker("binance"));
    }

    #[test]
    fn order_qty_validation() {
        assert!(is_valid_order_qty(1.0));
        assert!(is_valid_order_qty(100.0));
        assert!(is_valid_order_qty(MAX_ORDER_QTY));
        assert!(!is_valid_order_qty(0.0));
        assert!(!is_valid_order_qty(-1.0));
        assert!(!is_valid_order_qty(MAX_ORDER_QTY + 1.0));
        assert!(!is_valid_order_qty(f64::NAN));
        assert!(!is_valid_order_qty(f64::INFINITY));
    }

    #[test]
    fn place_order_serde_roundtrip() {
        let cmd = WebCmd::PlaceOrder {
            symbol: "AAPL".into(),
            qty: 10.0,
            side: "buy".into(),
            order_type: "market".into(),
            limit_price: None,
            stop_price: None,
            broker: "alpaca".into(),
            take_profit: None,
            stop_loss: None,
            trail_percent: None,
            trail_offset: None,
            risk_mode: None,
            risk_pct: None,
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let back: WebCmd = serde_json::from_str(&json).unwrap();
        assert_eq!(format!("{back:?}"), format!("{cmd:?}"));
    }

    #[test]
    fn place_order_limit_roundtrip() {
        let cmd = WebCmd::PlaceOrder {
            symbol: "SPY".into(),
            qty: 5.0,
            side: "sell".into(),
            order_type: "limit".into(),
            limit_price: Some(450.25),
            stop_price: None,
            broker: "tastytrade".into(),
            take_profit: None,
            stop_loss: None,
            trail_percent: None,
            trail_offset: None,
            risk_mode: None,
            risk_pct: None,
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let back: WebCmd = serde_json::from_str(&json).unwrap();
        match back {
            WebCmd::PlaceOrder { limit_price, .. } => assert_eq!(limit_price, Some(450.25)),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn place_order_bracket_roundtrip() {
        let cmd = WebCmd::PlaceOrder {
            symbol: "AAPL".into(),
            qty: 10.0,
            side: "buy".into(),
            order_type: "market".into(),
            limit_price: None,
            stop_price: None,
            broker: "alpaca".into(),
            take_profit: Some(200.0),
            stop_loss: Some(140.0),
            trail_percent: None,
            trail_offset: None,
            risk_mode: Some("standard".into()),
            risk_pct: Some(2.0),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let back: WebCmd = serde_json::from_str(&json).unwrap();
        match back {
            WebCmd::PlaceOrder {
                take_profit,
                stop_loss,
                risk_mode,
                risk_pct,
                ..
            } => {
                assert_eq!(take_profit, Some(200.0));
                assert_eq!(stop_loss, Some(140.0));
                assert_eq!(risk_mode, Some("standard".into()));
                assert_eq!(risk_pct, Some(2.0));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn place_order_trailing_roundtrip() {
        let cmd = WebCmd::PlaceOrder {
            symbol: "TSLA".into(),
            qty: 5.0,
            side: "buy".into(),
            order_type: "trailing_stop".into(),
            limit_price: None,
            stop_price: None,
            broker: "alpaca".into(),
            take_profit: None,
            stop_loss: None,
            trail_percent: Some(2.5),
            trail_offset: None,
            risk_mode: None,
            risk_pct: None,
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let back: WebCmd = serde_json::from_str(&json).unwrap();
        match back {
            WebCmd::PlaceOrder {
                trail_percent,
                order_type,
                ..
            } => {
                assert_eq!(trail_percent, Some(2.5));
                assert_eq!(order_type, "trailing_stop");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn cancel_order_serde_roundtrip() {
        let cmd = WebCmd::CancelOrder {
            order_id: "ORD-123".into(),
            broker: "alpaca".into(),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let back: WebCmd = serde_json::from_str(&json).unwrap();
        assert_eq!(format!("{back:?}"), format!("{cmd:?}"));
    }

    #[test]
    fn close_position_serde_roundtrip() {
        let cmd = WebCmd::ClosePosition {
            symbol: "AAPL".into(),
            broker: "tastytrade".into(),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let back: WebCmd = serde_json::from_str(&json).unwrap();
        assert_eq!(format!("{back:?}"), format!("{cmd:?}"));
    }

    #[test]
    fn order_result_msg_roundtrip() {
        let msg = WebMsg::OrderResult {
            ok: true,
            message: "Order filled".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let back: WebMsg = serde_json::from_str(&json).unwrap();
        match back {
            WebMsg::OrderResult { ok, message } => {
                assert!(ok);
                assert_eq!(message, "Order filled");
            }
            _ => panic!("wrong variant"),
        }
    }

    // ── ADR-092 tests ──────────────────────────────────────────────

    #[test]
    fn get_indicators_roundtrip() {
        let cmd = WebCmd::GetIndicators {
            symbol: "AAPL".into(),
            timeframe: "1Day".into(),
            indicators: vec!["SMA_200".into(), "RSI_14".into()],
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let back: WebCmd = serde_json::from_str(&json).unwrap();
        assert_eq!(format!("{back:?}"), format!("{cmd:?}"));
    }

    #[test]
    fn create_alert_roundtrip() {
        let cmd = WebCmd::CreateAlert {
            symbol: "AAPL".into(),
            condition: "crosses_above".into(),
            price: 200.0,
            message: "AAPL breakout!".into(),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let back: WebCmd = serde_json::from_str(&json).unwrap();
        assert_eq!(format!("{back:?}"), format!("{cmd:?}"));
    }

    #[test]
    fn delete_alert_roundtrip() {
        let cmd = WebCmd::DeleteAlert {
            alert_id: "alert-001".into(),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let back: WebCmd = serde_json::from_str(&json).unwrap();
        assert_eq!(format!("{back:?}"), format!("{cmd:?}"));
    }

    #[test]
    fn list_alerts_roundtrip() {
        let cmd = WebCmd::ListAlerts;
        let json = serde_json::to_string(&cmd).unwrap();
        let back: WebCmd = serde_json::from_str(&json).unwrap();
        assert_eq!(format!("{back:?}"), format!("{cmd:?}"));
    }

    #[test]
    fn get_news_roundtrip() {
        let cmd = WebCmd::GetNews {
            symbol: Some("AAPL".into()),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let back: WebCmd = serde_json::from_str(&json).unwrap();
        assert_eq!(format!("{back:?}"), format!("{cmd:?}"));

        let cmd2 = WebCmd::GetNews { symbol: None };
        let json2 = serde_json::to_string(&cmd2).unwrap();
        let back2: WebCmd = serde_json::from_str(&json2).unwrap();
        assert_eq!(format!("{back2:?}"), format!("{cmd2:?}"));
    }

    #[test]
    fn subscribe_roundtrip() {
        let cmd = WebCmd::Subscribe {
            symbol: "AAPL".into(),
            timeframe: "1Min".into(),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let back: WebCmd = serde_json::from_str(&json).unwrap();
        assert_eq!(format!("{back:?}"), format!("{cmd:?}"));
    }

    #[test]
    fn unsubscribe_roundtrip() {
        let cmd = WebCmd::Unsubscribe {
            symbol: "AAPL".into(),
            timeframe: "1Min".into(),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let back: WebCmd = serde_json::from_str(&json).unwrap();
        assert_eq!(format!("{back:?}"), format!("{cmd:?}"));
    }

    #[test]
    fn bar_update_msg_roundtrip() {
        let msg = WebMsg::BarUpdate {
            symbol: "AAPL".into(),
            timeframe: "1Min".into(),
            bar: BarData {
                timestamp: 1000,
                open: 150.0,
                high: 152.0,
                low: 149.0,
                close: 151.0,
                volume: 1000.0,
            },
        };
        let json = serde_json::to_string(&msg).unwrap();
        let back: WebMsg = serde_json::from_str(&json).unwrap();
        assert_eq!(format!("{back:?}"), format!("{msg:?}"));
    }

    #[test]
    fn position_update_msg_roundtrip() {
        let msg = WebMsg::PositionUpdate {
            items: vec![PositionSnapshot {
                symbol: "AAPL".into(),
                qty: 10.0,
                side: "long".into(),
                avg_entry_price: 150.0,
                market_value: 1520.0,
                unrealized_pl: 20.0,
                asset_class: "us_equity".into(),
            }],
        };
        let json = serde_json::to_string(&msg).unwrap();
        let back: WebMsg = serde_json::from_str(&json).unwrap();
        assert_eq!(format!("{back:?}"), format!("{msg:?}"));
    }

    #[test]
    fn account_update_msg_roundtrip() {
        let msg = WebMsg::AccountUpdate(AccountSnapshot {
            equity: 50000.0,
            cash: 25000.0,
            buying_power: 100000.0,
            portfolio_value: 50000.0,
            unrealized_pl: 1500.0,
            initial_margin: 10000.0,
            maintenance_margin: 5000.0,
            currency: "USD".into(),
        });
        let json = serde_json::to_string(&msg).unwrap();
        let back: WebMsg = serde_json::from_str(&json).unwrap();
        assert_eq!(format!("{back:?}"), format!("{msg:?}"));
    }

    #[test]
    fn indicator_data_msg_roundtrip() {
        let msg = WebMsg::IndicatorData {
            symbol: "AAPL".into(),
            timeframe: "1Day".into(),
            name: "SMA_200".into(),
            values: vec![Some(150.0), Some(150.5), None, Some(151.0)],
        };
        let json = serde_json::to_string(&msg).unwrap();
        let back: WebMsg = serde_json::from_str(&json).unwrap();
        assert_eq!(format!("{back:?}"), format!("{msg:?}"));
    }

    #[test]
    fn alert_triggered_msg_roundtrip() {
        let msg = WebMsg::AlertTriggered {
            alert_id: "alert-001".into(),
            symbol: "AAPL".into(),
            message: "Price crossed above 200".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let back: WebMsg = serde_json::from_str(&json).unwrap();
        assert_eq!(format!("{back:?}"), format!("{msg:?}"));
    }

    #[test]
    fn alert_list_msg_roundtrip() {
        let msg = WebMsg::AlertList {
            items: vec![AlertSnapshot {
                id: "alert-001".into(),
                symbol: "AAPL".into(),
                condition: "crosses_above".into(),
                price: 200.0,
                message: "breakout".into(),
                active: true,
            }],
        };
        let json = serde_json::to_string(&msg).unwrap();
        let back: WebMsg = serde_json::from_str(&json).unwrap();
        assert_eq!(format!("{back:?}"), format!("{msg:?}"));
    }

    #[test]
    fn news_feed_msg_roundtrip() {
        let msg = WebMsg::NewsFeed {
            items: vec![NewsItem {
                headline: "Apple beats Q3".into(),
                source: "Reuters".into(),
                url: "https://example.com/article".into(),
                symbol: Some("AAPL".into()),
                timestamp: 1700000000,
                summary: "Revenue up 15%".into(),
            }],
        };
        let json = serde_json::to_string(&msg).unwrap();
        let back: WebMsg = serde_json::from_str(&json).unwrap();
        assert_eq!(format!("{back:?}"), format!("{msg:?}"));
    }

    #[test]
    fn alert_snapshot_deny_unknown() {
        let json = r#"{"id":"a","symbol":"X","condition":"reaches","price":1.0,"message":"m","active":true,"extra":1}"#;
        assert!(serde_json::from_str::<AlertSnapshot>(json).is_err());
    }

    #[test]
    fn news_item_deny_unknown() {
        let json = r#"{"headline":"h","source":"s","url":"u","symbol":null,"timestamp":0,"summary":"s","extra":1}"#;
        assert!(serde_json::from_str::<NewsItem>(json).is_err());
    }

    #[test]
    fn risk_mode_validation() {
        assert!(is_valid_risk_mode("standard"));
        assert!(is_valid_risk_mode("fixed"));
        assert!(is_valid_risk_mode("dynamic"));
        assert!(is_valid_risk_mode("var"));
        assert!(is_valid_risk_mode("VaR"));
        assert!(!is_valid_risk_mode(""));
        assert!(!is_valid_risk_mode("yolo"));
    }

    #[test]
    fn alert_condition_validation() {
        assert!(is_valid_alert_condition("crosses_above"));
        assert!(is_valid_alert_condition("crosses_below"));
        assert!(is_valid_alert_condition("reaches"));
        assert!(!is_valid_alert_condition(""));
        assert!(!is_valid_alert_condition("explodes"));
    }

    #[test]
    fn indicator_name_validation() {
        assert!(is_valid_indicator_name("SMA_200"));
        assert!(is_valid_indicator_name("RSI_14"));
        assert!(is_valid_indicator_name("EMA21"));
        assert!(!is_valid_indicator_name(""));
        assert!(!is_valid_indicator_name("SMA 200"));
        assert!(!is_valid_indicator_name("SMA;DROP"));
    }

    #[test]
    fn alert_id_validation() {
        assert!(is_valid_alert_id("alert-001"));
        assert!(is_valid_alert_id("my_alert_2"));
        assert!(!is_valid_alert_id(""));
        assert!(!is_valid_alert_id("alert 001"));
        assert!(!is_valid_alert_id("a".repeat(65).as_str()));
    }

    #[test]
    fn trailing_stop_order_type_valid() {
        assert!(is_valid_order_type("trailing_stop"));
        assert!(is_valid_order_type("TRAILING_STOP"));
        assert!(is_valid_order_type("TrailingStop"));
    }

    // ── Additional coverage for order types ──

    #[test]
    fn place_order_oco_style_roundtrip() {
        // OCO exits use take_profit + stop_loss on PlaceOrder
        let cmd = WebCmd::PlaceOrder {
            symbol: "SPY".into(),
            qty: 10.0,
            side: "sell".into(),
            order_type: "limit".into(),
            limit_price: None,
            stop_price: None,
            broker: "alpaca".into(),
            take_profit: Some(500.0),
            stop_loss: Some(450.0),
            trail_percent: None,
            trail_offset: None,
            risk_mode: None,
            risk_pct: None,
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let back: WebCmd = serde_json::from_str(&json).unwrap();
        match back {
            WebCmd::PlaceOrder {
                take_profit,
                stop_loss,
                ..
            } => {
                assert_eq!(take_profit, Some(500.0));
                assert_eq!(stop_loss, Some(450.0));
            }
            _ => panic!("wrong variant"),
        }
    }

}
