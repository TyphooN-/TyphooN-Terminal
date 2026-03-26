//! tastytrade Broker Integration — REST API client for tastytrade/tastyworks.
//!
//! Implements session authentication, account info, positions, orders,
//! and options chain data via tastytrade's REST API.
//!
//! API docs: https://developer.tastytrade.com/

use serde::{Deserialize, Serialize};

const API_BASE: &str = "https://api.tastytrade.com";
const SANDBOX_BASE: &str = "https://api.cert.tastytrade.com";

/// tastytrade broker client.
pub struct TastytradeBroker {
    client: reqwest::Client,
    base_url: String,
    session_token: Option<String>,
    account_number: Option<String>,
}

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
    pub instrument_type: String,  // "Equity", "Equity Option", "Future", etc.
    pub quantity: f64,
    pub quantity_direction: String,  // "Long" or "Short"
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
    pub action: String,  // "Buy to Open", "Sell to Close", etc.
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

impl TastytradeBroker {
    /// Create a new tastytrade broker client.
    pub fn new(sandbox: bool) -> Self {
        let base_url = if sandbox { SANDBOX_BASE } else { API_BASE };
        Self {
            client: reqwest::Client::builder()
                .user_agent("TyphooN-Terminal/1.0")
                .build()
                .unwrap_or_default(),
            base_url: base_url.to_string(),
            session_token: None,
            account_number: None,
        }
    }

    /// Authenticate with username and password.
    pub async fn login(&mut self, username: &str, password: &str) -> Result<TastySession, String> {
        let url = format!("{}/sessions", self.base_url);
        let body = serde_json::json!({
            "login": username,
            "password": password,
        });

        let resp = self.client.post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send().await
            .map_err(|e| format!("tastytrade login failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("tastytrade login returned {}: {}", status, text));
        }

        let data: serde_json::Value = resp.json().await
            .map_err(|e| format!("tastytrade parse failed: {e}"))?;

        let session_token = data["data"]["session-token"]
            .as_str()
            .ok_or_else(|| "No session token in response".to_string())?
            .to_string();

        self.session_token = Some(session_token.clone());

        // Get accounts
        if let Ok(accounts) = self.get_accounts().await {
            if let Some(first) = accounts.first() {
                self.account_number = Some(first.account_number.clone());
            }
        }

        Ok(TastySession {
            session_token,
            remember_token: data["data"]["remember-token"].as_str().map(|s| s.to_string()),
        })
    }

    /// Helper: add auth header to request.
    fn auth_header(&self) -> Option<String> {
        self.session_token.clone()
    }

    /// Get all accounts for the authenticated user.
    pub async fn get_accounts(&self) -> Result<Vec<TastyAccount>, String> {
        let token = self.auth_header().ok_or("Not authenticated")?;
        let url = format!("{}/customers/me/accounts", self.base_url);

        let resp = self.client.get(&url)
            .header("Authorization", &token)
            .send().await
            .map_err(|e| format!("Get accounts failed: {e}"))?;

        let data: serde_json::Value = resp.json().await
            .map_err(|e| format!("Parse accounts failed: {e}"))?;

        let items = data["data"]["items"].as_array()
            .ok_or_else(|| "No accounts array".to_string())?;

        let mut accounts = Vec::new();
        for item in items {
            let acct = &item["account"];
            accounts.push(TastyAccount {
                account_number: acct["account-number"].as_str().unwrap_or("").to_string(),
                account_type: acct["account-type-name"].as_str().unwrap_or("").to_string(),
                nickname: acct["nickname"].as_str().map(|s| s.to_string()),
                margin_or_cash: acct["margin-or-cash"].as_str().unwrap_or("Cash").to_string(),
                is_closed: acct["is-closed"].as_bool().unwrap_or(false),
            });
        }
        Ok(accounts)
    }

    /// Get positions for the primary account.
    pub async fn get_positions(&self) -> Result<Vec<TastyPosition>, String> {
        let token = self.auth_header().ok_or("Not authenticated")?;
        let acct = self.account_number.as_ref().ok_or("No account selected")?;
        let url = format!("{}/accounts/{}/positions", self.base_url, acct);

        let resp = self.client.get(&url)
            .header("Authorization", &token)
            .send().await
            .map_err(|e| format!("Get positions failed: {e}"))?;

        let data: serde_json::Value = resp.json().await
            .map_err(|e| format!("Parse positions failed: {e}"))?;

        let items = data["data"]["items"].as_array()
            .ok_or_else(|| "No positions array".to_string())?;

        let mut positions = Vec::new();
        for item in items {
            positions.push(TastyPosition {
                symbol: item["symbol"].as_str().unwrap_or("").to_string(),
                instrument_type: item["instrument-type"].as_str().unwrap_or("").to_string(),
                quantity: item["quantity"].as_f64().unwrap_or(0.0),
                quantity_direction: item["quantity-direction"].as_str().unwrap_or("").to_string(),
                close_price: item["close-price"].as_str().and_then(|s| s.parse().ok()).unwrap_or(0.0),
                average_open_price: item["average-open-price"].as_str().and_then(|s| s.parse().ok()).unwrap_or(0.0),
                mark_price: item["mark-price"].as_str().and_then(|s| s.parse().ok()),
                unrealized_pnl: None, // Computed client-side
            });
        }
        Ok(positions)
    }

    /// Get orders for the primary account.
    pub async fn get_orders(&self, status: &str) -> Result<Vec<TastyOrder>, String> {
        let token = self.auth_header().ok_or("Not authenticated")?;
        let acct = self.account_number.as_ref().ok_or("No account selected")?;
        let url = format!("{}/accounts/{}/orders?status={}", self.base_url, acct, status);

        let resp = self.client.get(&url)
            .header("Authorization", &token)
            .send().await
            .map_err(|e| format!("Get orders failed: {e}"))?;

        let data: serde_json::Value = resp.json().await
            .map_err(|e| format!("Parse orders failed: {e}"))?;

        let items = data["data"]["items"].as_array()
            .ok_or_else(|| "No orders array".to_string())?;

        let mut orders = Vec::new();
        for item in items {
            let mut legs = Vec::new();
            if let Some(leg_arr) = item["legs"].as_array() {
                for leg in leg_arr {
                    legs.push(TastyOrderLeg {
                        instrument_type: leg["instrument-type"].as_str().unwrap_or("").to_string(),
                        symbol: leg["symbol"].as_str().unwrap_or("").to_string(),
                        action: leg["action"].as_str().unwrap_or("").to_string(),
                        quantity: leg["quantity"].as_i64().unwrap_or(0),
                    });
                }
            }
            orders.push(TastyOrder {
                id: item["id"].as_str().or(item["id"].as_i64().map(|_| "")).unwrap_or("").to_string(),
                order_type: item["order-type"].as_str().unwrap_or("").to_string(),
                time_in_force: item["time-in-force"].as_str().unwrap_or("").to_string(),
                status: item["status"].as_str().unwrap_or("").to_string(),
                legs,
                price: item["price"].as_str().and_then(|s| s.parse().ok()),
                size: item["size"].as_i64().unwrap_or(0),
            });
        }
        Ok(orders)
    }

    /// Place an equity order.
    pub async fn place_equity_order(
        &self, symbol: &str, qty: i64, action: &str, order_type: &str, price: Option<f64>, tif: &str,
    ) -> Result<String, String> {
        let token = self.auth_header().ok_or("Not authenticated")?;
        let acct = self.account_number.as_ref().ok_or("No account selected")?;
        let url = format!("{}/accounts/{}/orders", self.base_url, acct);

        let mut order = serde_json::json!({
            "time-in-force": tif,
            "order-type": order_type,
            "legs": [{
                "instrument-type": "Equity",
                "symbol": symbol,
                "action": action,
                "quantity": qty,
            }],
        });
        if let Some(p) = price {
            order["price"] = serde_json::json!(format!("{:.2}", p));
        }

        let resp = self.client.post(&url)
            .header("Authorization", &token)
            .header("Content-Type", "application/json")
            .json(&order)
            .send().await
            .map_err(|e| format!("Place order failed: {e}"))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Order rejected: {}", text));
        }

        let data: serde_json::Value = resp.json().await.unwrap_or_default();
        Ok(data["data"]["order"]["id"].as_str().unwrap_or("ok").to_string())
    }

    /// Get option chain for a symbol.
    pub async fn get_option_chain(&self, symbol: &str) -> Result<Vec<TastyExpiration>, String> {
        let token = self.auth_header().ok_or("Not authenticated")?;
        let url = format!("{}/option-chains/{}/nested", self.base_url, symbol);

        let resp = self.client.get(&url)
            .header("Authorization", &token)
            .send().await
            .map_err(|e| format!("Get option chain failed: {e}"))?;

        let data: serde_json::Value = resp.json().await
            .map_err(|e| format!("Parse chain failed: {e}"))?;

        let items = data["data"]["items"].as_array()
            .ok_or_else(|| "No chain data".to_string())?;

        let mut expirations = Vec::new();
        for item in items {
            if let Some(exp_arr) = item["expirations"].as_array() {
                for exp in exp_arr {
                    let mut strikes = Vec::new();
                    if let Some(strike_arr) = exp["strikes"].as_array() {
                        for s in strike_arr {
                            strikes.push(TastyStrike {
                                strike_price: s["strike-price"].as_str().and_then(|p| p.parse().ok()).unwrap_or(0.0),
                                call_symbol: s["call"].as_str().unwrap_or("").to_string(),
                                put_symbol: s["put"].as_str().unwrap_or("").to_string(),
                            });
                        }
                    }
                    expirations.push(TastyExpiration {
                        expiration_date: exp["expiration-date"].as_str().unwrap_or("").to_string(),
                        strikes,
                    });
                }
            }
        }
        Ok(expirations)
    }

    pub fn is_authenticated(&self) -> bool { self.session_token.is_some() }
    pub fn account_number(&self) -> Option<&str> { self.account_number.as_deref() }
}
