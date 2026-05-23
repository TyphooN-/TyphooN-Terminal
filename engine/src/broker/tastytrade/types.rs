//! Tastytrade REST/streaming type definitions.
//!
//! Mirrors the shapes tastytrade returns at the API boundary; conversion
//! into TyphooN-internal models happens at the call sites in
//! `super::TastytradeBroker`. Numeric fields can arrive as JSON numbers or
//! JSON strings depending on the endpoint, so the shared `parse_num` helper
//! accepts both.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TastySession {
    pub session_token: String,
    pub remember_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TastyAccount {
    pub account_number: String,
    pub account_type: String,
    pub nickname: Option<String>,
    pub margin_or_cash: String,
    pub is_closed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TastyPosition {
    pub symbol: String,
    pub instrument_type: String, // "Equity", "Equity Option", "Future", etc.
    pub quantity: f64,
    pub quantity_direction: String, // "Long" or "Short"
    pub close_price: f64,
    pub average_open_price: f64,
    pub mark_price: Option<f64>,
    pub unrealized_pnl: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TastyOrder {
    pub id: String,
    pub order_type: String,
    pub time_in_force: String,
    pub status: String,
    pub legs: Vec<TastyOrderLeg>,
    pub price: Option<f64>,
    pub size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TastyOrderLeg {
    pub instrument_type: String,
    pub symbol: String,
    pub action: String, // "Buy to Open", "Sell to Close", etc.
    pub quantity: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TastyOptionChain {
    pub underlying_symbol: String,
    pub expirations: Vec<TastyExpiration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TastyExpiration {
    pub expiration_date: String,
    pub strikes: Vec<TastyStrike>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TastyStrike {
    pub strike_price: f64,
    pub call_symbol: String,
    pub put_symbol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TastyGreeks {
    pub delta: f64,
    pub gamma: f64,
    pub theta: f64,
    pub vega: f64,
    pub rho: f64,
    pub implied_volatility: f64,
}

/// Quote snapshot from tastytrade REST API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TastyQuote {
    pub symbol: String,
    pub bid: f64,
    pub ask: f64,
    pub last: f64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub prev_close: f64,
    pub volume: i64,
    pub bid_size: i64,
    pub ask_size: i64,
}

/// Market metrics (IV, liquidity) from tastytrade REST API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TastyMarketMetric {
    pub symbol: String,
    pub iv_index: f64,
    pub iv_rank: f64,
    pub iv_percentile: f64,
    pub liquidity_rating: i32,
    pub liquidity_rank: f64,
    pub beta: f64,
    pub earnings_date: Option<String>,
}

/// Account balances from tastytrade REST API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TastyBalances {
    pub cash_balance: f64,
    pub net_liquidating_value: f64,
    pub equity_buying_power: f64,
    pub maintenance_requirement: f64,
    pub pending_cash: f64,
}

/// Helper: parse number from tastytrade JSON (may be string or number).
pub(super) fn parse_num(v: &serde_json::Value) -> f64 {
    v.as_f64()
        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn tasty_session_construction() {
        let session = TastySession {
            session_token: "tok_abc123".to_string(),
            remember_token: Some("rem_xyz".to_string()),
        };
        assert_eq!(session.session_token, "tok_abc123");
        assert_eq!(session.remember_token.as_deref(), Some("rem_xyz"));
    }

    #[test]
    fn tasty_session_no_remember_token() {
        let session = TastySession {
            session_token: "tok_only".to_string(),
            remember_token: None,
        };
        assert_eq!(session.session_token, "tok_only");
        assert!(session.remember_token.is_none());
    }

    #[test]
    fn tasty_account_construction() {
        let acct = TastyAccount {
            account_number: "5YZ12345".to_string(),
            account_type: "Individual".to_string(),
            nickname: Some("Main".to_string()),
            margin_or_cash: "Margin".to_string(),
            is_closed: false,
        };
        assert_eq!(acct.account_number, "5YZ12345");
        assert!(!acct.is_closed);
    }

    #[test]
    fn tasty_account_closed() {
        let acct = TastyAccount {
            account_number: "CLOSED1".to_string(),
            account_type: "Entity".to_string(),
            nickname: None,
            margin_or_cash: "Cash".to_string(),
            is_closed: true,
        };
        assert!(acct.is_closed);
        assert!(acct.nickname.is_none());
    }

    #[test]
    fn tasty_position_long() {
        let pos = TastyPosition {
            symbol: "AAPL".to_string(),
            instrument_type: "Equity".to_string(),
            quantity: 100.0,
            quantity_direction: "Long".to_string(),
            close_price: 175.50,
            average_open_price: 150.00,
            mark_price: Some(176.00),
            unrealized_pnl: Some(2600.0),
        };
        assert_eq!(pos.quantity_direction, "Long");
        assert_eq!(pos.mark_price, Some(176.00));
    }

    #[test]
    fn tasty_position_short_no_mark() {
        let pos = TastyPosition {
            symbol: "SPY".to_string(),
            instrument_type: "Equity Option".to_string(),
            quantity: 5.0,
            quantity_direction: "Short".to_string(),
            close_price: 3.20,
            average_open_price: 4.50,
            mark_price: None,
            unrealized_pnl: None,
        };
        assert_eq!(pos.quantity_direction, "Short");
        assert!(pos.mark_price.is_none());
    }

    #[test]
    fn tasty_order_with_legs() {
        let leg = TastyOrderLeg {
            instrument_type: "Equity".to_string(),
            symbol: "MSFT".to_string(),
            action: "Buy to Open".to_string(),
            quantity: 10,
        };
        let order = TastyOrder {
            id: "ORD-12345".to_string(),
            order_type: "Limit".to_string(),
            time_in_force: "Day".to_string(),
            status: "Received".to_string(),
            legs: vec![leg],
            price: Some(350.00),
            size: 10,
        };
        assert_eq!(order.id, "ORD-12345");
        assert_eq!(order.legs.len(), 1);
    }

    #[test]
    fn tasty_order_no_price() {
        let order = TastyOrder {
            id: "ORD-99".to_string(),
            order_type: "Market".to_string(),
            time_in_force: "GTC".to_string(),
            status: "Filled".to_string(),
            legs: vec![],
            price: None,
            size: 50,
        };
        assert!(order.price.is_none());
    }

    #[test]
    fn tasty_greeks_construction() {
        let greeks = TastyGreeks {
            delta: 0.45,
            gamma: 0.03,
            theta: -0.12,
            vega: 0.25,
            rho: 0.01,
            implied_volatility: 0.32,
        };
        assert!((greeks.delta - 0.45).abs() < f64::EPSILON);
        assert!((greeks.implied_volatility - 0.32).abs() < f64::EPSILON);
    }

    #[test]
    fn tasty_expiration_with_strikes() {
        let strike = TastyStrike {
            strike_price: 150.0,
            call_symbol: "AAPL  260418C00150000".to_string(),
            put_symbol: "AAPL  260418P00150000".to_string(),
        };
        let exp = TastyExpiration {
            expiration_date: "2026-04-18".to_string(),
            strikes: vec![strike],
        };
        assert_eq!(exp.strikes.len(), 1);
        assert!(exp.strikes[0].call_symbol.contains('C'));
        assert!(exp.strikes[0].put_symbol.contains('P'));
    }

    #[test]
    fn tasty_option_chain_construction() {
        let chain = TastyOptionChain {
            underlying_symbol: "AAPL".to_string(),
            expirations: vec![TastyExpiration {
                expiration_date: "2026-04-18".to_string(),
                strikes: vec![],
            }],
        };
        assert_eq!(chain.underlying_symbol, "AAPL");
    }

    #[test]
    fn tasty_quote_construction() {
        let quote = TastyQuote {
            symbol: "NVDA".to_string(),
            bid: 880.0,
            ask: 881.0,
            last: 880.50,
            open: 875.0,
            high: 890.0,
            low: 870.0,
            close: 880.50,
            prev_close: 872.0,
            volume: 50_000_000,
            bid_size: 100,
            ask_size: 200,
        };
        assert_eq!(quote.volume, 50_000_000);
    }

    #[test]
    fn tasty_balances_construction() {
        let bal = TastyBalances {
            cash_balance: 10_000.0,
            net_liquidating_value: 50_000.0,
            equity_buying_power: 100_000.0,
            maintenance_requirement: 25_000.0,
            pending_cash: 0.0,
        };
        assert!((bal.net_liquidating_value - 50_000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn tasty_market_metric_construction() {
        let metric = TastyMarketMetric {
            symbol: "SPY".to_string(),
            iv_index: 0.18,
            iv_rank: 0.35,
            iv_percentile: 0.42,
            liquidity_rating: 5,
            liquidity_rank: 0.99,
            beta: 1.0,
            earnings_date: None,
        };
        assert_eq!(metric.liquidity_rating, 5);
        assert!(metric.earnings_date.is_none());
    }

    #[test]
    fn parse_num_accepts_number_string_null_and_garbage() {
        assert!((parse_num(&json!(42.5)) - 42.5).abs() < f64::EPSILON);
        assert!((parse_num(&json!("99.9")) - 99.9).abs() < f64::EPSILON);
        assert_eq!(parse_num(&json!(null)), 0.0);
        assert_eq!(parse_num(&json!("not_a_number")), 0.0);
        assert_eq!(parse_num(&json!(true)), 0.0);
    }
}
