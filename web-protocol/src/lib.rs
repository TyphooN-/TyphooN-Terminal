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
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '/' || c == '_')
}

/// Validate a timeframe string: alphanumeric only, bounded length.
pub fn is_valid_timeframe(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= MAX_TIMEFRAME_LEN
        && s.chars().all(|c| c.is_ascii_alphanumeric())
}

// ── Client → Server ─────────────────────────────────────────────────
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum WebCmd {
    /// First message must be Auth with passphrase.
    Auth { passphrase: String },
    GetAccount,
    GetPositions,
    GetOrders,
    GetWatchlistQuotes { symbols: Vec<String> },
    GetBars { symbol: String, timeframe: String },
    GetMarketClock,
    Ping,
}

// ── Server → Client ─────────────────────────────────────────────────
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum WebMsg {
    /// Authentication result.
    AuthResult { ok: bool },
    Account(AccountSnapshot),
    Positions { items: Vec<PositionSnapshot> },
    Orders { items: Vec<OrderSnapshot> },
    WatchlistQuotes { items: Vec<QuoteSnapshot> },
    Bars {
        symbol: String,
        timeframe: String,
        bars: Vec<BarData>,
    },
    MarketClock { info: String },
    QuoteTick {
        symbol: String,
        bid: f64,
        ask: f64,
    },
    Error { msg: String },
    Pong,
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

// ── Tests ───────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn webcmd_serde_roundtrip() {
        let cmds = vec![
            WebCmd::Auth { passphrase: "test123".into() },
            WebCmd::GetAccount,
            WebCmd::GetPositions,
            WebCmd::GetOrders,
            WebCmd::GetWatchlistQuotes { symbols: vec!["AAPL".into(), "MSFT".into()] },
            WebCmd::GetBars { symbol: "XAUUSD".into(), timeframe: "1Day".into() },
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
                equity: 10000.0, cash: 5000.0, buying_power: 20000.0,
                portfolio_value: 10000.0, unrealized_pl: 500.0,
                initial_margin: 2000.0, maintenance_margin: 1000.0,
                currency: "USD".into(),
            }),
            WebMsg::Positions { items: vec![PositionSnapshot {
                symbol: "AAPL".into(), qty: 10.0, side: "long".into(),
                avg_entry_price: 150.0, market_value: 1600.0,
                unrealized_pl: 100.0, asset_class: "us_equity".into(),
            }] },
            WebMsg::Pong,
            WebMsg::Error { msg: "test error".into() },
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
}
