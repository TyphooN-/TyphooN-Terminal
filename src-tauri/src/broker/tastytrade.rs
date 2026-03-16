//! Tastytrade broker interface.
//!
//! Implements the same operations as AlpacaBroker for Tastytrade's REST API.
//! Base URLs:
//!   Production: https://api.tastyworks.com
//!   Sandbox:    https://api.cert.tastyworks.com

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use zeroize::Zeroizing;

const PROD_BASE: &str = "https://api.tastyworks.com";
const CERT_BASE: &str = "https://api.cert.tastyworks.com";

#[derive(Debug, Clone)]
pub struct TastytradeBroker {
    client: Client,
    base_url: String,
    session_token: Arc<Mutex<Zeroizing<String>>>,
    account_number: Arc<Mutex<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TastyAccountInfo {
    pub account_number: String,
    pub equity: f64,
    pub cash: f64,
    pub buying_power: f64,
    pub net_liquidating_value: f64,
    pub maintenance_requirement: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TastyPosition {
    pub symbol: String,
    pub qty: f64,
    pub side: String,
    pub avg_entry_price: f64,
    pub market_value: f64,
    pub unrealized_pl: f64,
    pub instrument_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TastyOrderResult {
    pub id: String,
    pub symbol: String,
    pub qty: String,
    pub side: String,
    pub status: String,
}

impl TastytradeBroker {
    /// Create a new Tastytrade broker and authenticate.
    pub async fn login(username: String, password: String, is_sandbox: bool) -> Result<Self, String> {
        let base_url = if is_sandbox { CERT_BASE } else { PROD_BASE };
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        // Authenticate (password consumed here, then dropped)
        let body = serde_json::json!({
            "login": username,
            "password": password,
        });
        // password String is dropped after this point
        let resp = client
            .post(format!("{}/sessions", base_url))
            .json(&body)
            .send()
            .await
            .map_err(|_| "Tastytrade login request failed".to_string())?;

        if !resp.status().is_success() {
            let status = resp.status();
            let _ = resp.text().await;
            return Err(format!("Tastytrade login failed: HTTP {status}"));
        }

        let json: serde_json::Value = resp.json().await
            .map_err(|_| "Tastytrade login parse failed".to_string())?;

        let token = json["data"]["session-token"]
            .as_str()
            .ok_or("No session token in response")?
            .to_string();

        let broker = Self {
            client,
            base_url: base_url.to_string(),
            session_token: Arc::new(Mutex::new(Zeroizing::new(token))),
            account_number: Arc::new(Mutex::new(String::new())),
        };

        // Fetch first account number
        let accounts = broker.get_accounts().await?;
        if let Some(first) = accounts.first() {
            *broker.account_number.lock().await = first.clone();
        }

        Ok(broker)
    }

    async fn auth_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        let token = self.session_token.lock().await;
        if let Ok(val) = format!("{}", token.as_str()).parse() {
            headers.insert("Authorization", val);
        }
        headers
    }

    async fn get_accounts(&self) -> Result<Vec<String>, String> {
        let resp = self.client
            .get(format!("{}/customers/me/accounts", self.base_url))
            .headers(self.auth_headers().await)
            .send()
            .await
            .map_err(|_| "Tastytrade accounts request failed".to_string())?;

        if !resp.status().is_success() {
            let status = resp.status();
            let _ = resp.text().await;
            return Err(format!("Tastytrade accounts failed: HTTP {status}"));
        }

        let json: serde_json::Value = resp.json().await
            .map_err(|_| "Tastytrade accounts parse failed".to_string())?;

        let accounts: Vec<String> = json["data"]["items"]
            .as_array()
            .map(|arr| arr.iter()
                .filter_map(|item| item["account"]["account-number"].as_str().map(|s| s.to_string()))
                .collect()
            )
            .unwrap_or_default();

        Ok(accounts)
    }

    /// Get account balance/equity info.
    pub async fn get_account_info(&self) -> Result<TastyAccountInfo, String> {
        let acct = self.account_number.lock().await.clone();
        if acct.is_empty() { return Err("No account number set".into()); }

        let resp = self.client
            .get(format!("{}/accounts/{}/balances", self.base_url, acct))
            .headers(self.auth_headers().await)
            .send()
            .await
            .map_err(|_| "Tastytrade balance request failed".to_string())?;

        if !resp.status().is_success() {
            let status = resp.status();
            let _ = resp.text().await;
            return Err(format!("Tastytrade balance failed: HTTP {status}"));
        }

        let json: serde_json::Value = resp.json().await
            .map_err(|_| "Tastytrade balance parse failed".to_string())?;

        let data = &json["data"];
        Ok(TastyAccountInfo {
            account_number: acct,
            equity: data["equity-buying-power"].as_f64().unwrap_or(0.0),
            cash: data["cash-balance"].as_f64().unwrap_or(0.0),
            buying_power: data["derivative-buying-power"].as_f64().unwrap_or(0.0),
            net_liquidating_value: data["net-liquidating-value"].as_f64().unwrap_or(0.0),
            maintenance_requirement: data["maintenance-requirement"].as_f64().unwrap_or(0.0),
        })
    }

    /// Get open positions.
    pub async fn get_positions(&self) -> Result<Vec<TastyPosition>, String> {
        let acct = self.account_number.lock().await.clone();
        if acct.is_empty() { return Err("No account number set".into()); }

        let resp = self.client
            .get(format!("{}/accounts/{}/positions", self.base_url, acct))
            .headers(self.auth_headers().await)
            .send()
            .await
            .map_err(|_| "Tastytrade positions request failed".to_string())?;

        if !resp.status().is_success() {
            let status = resp.status();
            let _ = resp.text().await;
            return Err(format!("Tastytrade positions failed: HTTP {status}"));
        }

        let json: serde_json::Value = resp.json().await
            .map_err(|_| "Tastytrade positions parse failed".to_string())?;

        let positions: Vec<TastyPosition> = json["data"]["items"]
            .as_array()
            .map(|arr| arr.iter().map(|p| {
                let qty: f64 = p["quantity"].as_str().unwrap_or("0").parse().unwrap_or(0.0);
                let direction = p["quantity-direction"].as_str().unwrap_or("Long");
                TastyPosition {
                    symbol: p["symbol"].as_str().unwrap_or("").to_string(),
                    qty: qty.abs(),
                    side: if direction == "Short" { "short".to_string() } else { "long".to_string() },
                    avg_entry_price: p["average-open-price"].as_str().unwrap_or("0").parse().unwrap_or(0.0),
                    market_value: p["mark"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0) * qty.abs(),
                    unrealized_pl: p["unrealized-day-gain"].as_str().unwrap_or("0").parse().unwrap_or(0.0),
                    instrument_type: p["instrument-type"].as_str().unwrap_or("").to_string(),
                }
            }).collect())
            .unwrap_or_default();

        Ok(positions)
    }

    /// Place a market order.
    pub async fn market_order(&self, symbol: &str, qty: f64, side: &str) -> Result<TastyOrderResult, String> {
        let acct = self.account_number.lock().await.clone();
        if acct.is_empty() { return Err("No account number set".into()); }

        let action = if side == "buy" { "Buy to Open" } else { "Sell to Open" };

        let body = serde_json::json!({
            "time-in-force": "Day",
            "order-type": "Market",
            "legs": [{
                "instrument-type": "Equity",
                "symbol": symbol,
                "quantity": qty,
                "action": action,
            }],
        });

        let resp = self.client
            .post(format!("{}/accounts/{}/orders", self.base_url, acct))
            .headers(self.auth_headers().await)
            .json(&body)
            .send()
            .await
            .map_err(|_| "Tastytrade order request failed".to_string())?;

        if !resp.status().is_success() {
            let status = resp.status();
            let _ = resp.text().await;
            return Err(format!("Tastytrade order failed: HTTP {status}"));
        }

        let json: serde_json::Value = resp.json().await
            .map_err(|_| "Tastytrade order parse failed".to_string())?;

        let data = &json["data"]["order"];
        Ok(TastyOrderResult {
            id: data["id"].as_u64().map(|n| n.to_string()).unwrap_or_default(),
            symbol: symbol.to_string(),
            qty: qty.to_string(),
            side: side.to_string(),
            status: data["status"].as_str().unwrap_or("").to_string(),
        })
    }
}
