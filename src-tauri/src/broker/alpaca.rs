//! Alpaca broker interface.
//!
//! Wraps Alpaca REST API and WebSocket streaming.
//! Provides the same operations as MQL5 CTrade: open, close, partial close, modify, account info.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const PAPER_BASE: &str = "https://paper-api.alpaca.markets";
const LIVE_BASE: &str = "https://api.alpaca.markets";
const DATA_BASE: &str = "https://data.alpaca.markets";

#[derive(Debug, Clone)]
pub struct AlpacaBroker {
    client: Client,
    base_url: String,
    api_key: String,
    secret_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfo {
    pub equity: f64,
    pub cash: f64,
    pub buying_power: f64,
    pub portfolio_value: f64,
    pub initial_margin: f64,
    pub maintenance_margin: f64,
    pub currency: String,
    pub pattern_day_trader: bool,
    pub trading_blocked: bool,
    pub balance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionInfo {
    pub symbol: String,
    pub qty: f64,
    pub side: String,
    pub avg_entry_price: f64,
    pub market_value: f64,
    pub unrealized_pl: f64,
    pub asset_class: String,
    pub asset_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetInfo {
    pub symbol: String,
    pub name: String,
    pub asset_class: String,
    pub tradable: bool,
    pub marginable: bool,
    pub shortable: bool,
    pub fractionable: bool,
    pub min_order_size: Option<f64>,
    pub min_trade_increment: Option<f64>,
    pub price_increment: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderResult {
    pub id: String,
    pub symbol: String,
    pub qty: String,
    pub side: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bar {
    pub timestamp: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

impl AlpacaBroker {
    pub fn new(api_key: String, secret_key: String, paper: bool) -> Self {
        let base_url = if paper {
            PAPER_BASE.to_string()
        } else {
            LIVE_BASE.to_string()
        };
        Self {
            client: Client::new(),
            base_url,
            api_key,
            secret_key,
        }
    }

    fn headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("APCA-API-KEY-ID", self.api_key.parse().unwrap());
        headers.insert("APCA-API-SECRET-KEY", self.secret_key.parse().unwrap());
        headers
    }

    // ── Account ──────────────────────────────────────────────────────

    pub async fn get_account(&self) -> Result<AccountInfo, String> {
        let resp = self
            .client
            .get(format!("{}/v2/account", self.base_url))
            .headers(self.headers())
            .send()
            .await
            .map_err(|e| format!("Account request failed: {e}"))?;

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Account parse failed: {e}"))?;

        Ok(AccountInfo {
            equity: json["equity"].as_str().unwrap_or("0").parse().unwrap_or(0.0),
            cash: json["cash"].as_str().unwrap_or("0").parse().unwrap_or(0.0),
            buying_power: json["buying_power"].as_str().unwrap_or("0").parse().unwrap_or(0.0),
            portfolio_value: json["portfolio_value"].as_str().unwrap_or("0").parse().unwrap_or(0.0),
            initial_margin: json["initial_margin"].as_str().unwrap_or("0").parse().unwrap_or(0.0),
            maintenance_margin: json["maintenance_margin"].as_str().unwrap_or("0").parse().unwrap_or(0.0),
            currency: json["currency"].as_str().unwrap_or("USD").to_string(),
            pattern_day_trader: json["pattern_day_trader"].as_bool().unwrap_or(false),
            trading_blocked: json["trading_blocked"].as_bool().unwrap_or(false),
            balance: json["last_equity"].as_str().unwrap_or("0").parse().unwrap_or(0.0),
        })
    }

    // ── Positions ────────────────────────────────────────────────────

    pub async fn get_positions(&self) -> Result<Vec<PositionInfo>, String> {
        let resp = self
            .client
            .get(format!("{}/v2/positions", self.base_url))
            .headers(self.headers())
            .send()
            .await
            .map_err(|e| format!("Positions request failed: {e}"))?;

        let json: Vec<serde_json::Value> = resp
            .json()
            .await
            .map_err(|e| format!("Positions parse failed: {e}"))?;

        Ok(json
            .iter()
            .map(|p| PositionInfo {
                symbol: p["symbol"].as_str().unwrap_or("").to_string(),
                qty: p["qty"].as_str().unwrap_or("0").parse().unwrap_or(0.0),
                side: p["side"].as_str().unwrap_or("").to_string(),
                avg_entry_price: p["avg_entry_price"].as_str().unwrap_or("0").parse().unwrap_or(0.0),
                market_value: p["market_value"].as_str().unwrap_or("0").parse().unwrap_or(0.0),
                unrealized_pl: p["unrealized_pl"].as_str().unwrap_or("0").parse().unwrap_or(0.0),
                asset_class: p["asset_class"].as_str().unwrap_or("").to_string(),
                asset_id: p["asset_id"].as_str().unwrap_or("").to_string(),
            })
            .collect())
    }

    // ── Orders ───────────────────────────────────────────────────────

    pub async fn market_order(&self, symbol: &str, qty: f64, side: &str) -> Result<OrderResult, String> {
        let mut body = HashMap::new();
        body.insert("symbol", symbol.to_string());
        body.insert("qty", qty.to_string());
        body.insert("side", side.to_string());
        body.insert("type", "market".to_string());
        body.insert("time_in_force", "gtc".to_string());

        let resp = self
            .client
            .post(format!("{}/v2/orders", self.base_url))
            .headers(self.headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Order request failed: {e}"))?;

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Order parse failed: {e}"))?;

        Ok(OrderResult {
            id: json["id"].as_str().unwrap_or("").to_string(),
            symbol: json["symbol"].as_str().unwrap_or("").to_string(),
            qty: json["qty"].as_str().unwrap_or("0").to_string(),
            side: json["side"].as_str().unwrap_or("").to_string(),
            status: json["status"].as_str().unwrap_or("").to_string(),
        })
    }

    pub async fn close_position(&self, symbol: &str, qty: Option<f64>) -> Result<OrderResult, String> {
        let url = if let Some(q) = qty {
            format!("{}/v2/positions/{}?qty={}", self.base_url, symbol, q)
        } else {
            format!("{}/v2/positions/{}", self.base_url, symbol)
        };

        let resp = self
            .client
            .delete(&url)
            .headers(self.headers())
            .send()
            .await
            .map_err(|e| format!("Close position failed: {e}"))?;

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Close parse failed: {e}"))?;

        Ok(OrderResult {
            id: json["id"].as_str().unwrap_or("").to_string(),
            symbol: json["symbol"].as_str().unwrap_or("").to_string(),
            qty: json["qty"].as_str().unwrap_or("0").to_string(),
            side: json["side"].as_str().unwrap_or("").to_string(),
            status: json["status"].as_str().unwrap_or("").to_string(),
        })
    }

    pub async fn close_all_positions(&self) -> Result<(), String> {
        self.client
            .delete(format!("{}/v2/positions", self.base_url))
            .headers(self.headers())
            .send()
            .await
            .map_err(|e| format!("Close all failed: {e}"))?;
        Ok(())
    }

    // ── Asset Info ───────────────────────────────────────────────────

    pub async fn get_asset(&self, symbol: &str) -> Result<AssetInfo, String> {
        let resp = self
            .client
            .get(format!("{}/v2/assets/{}", self.base_url, symbol))
            .headers(self.headers())
            .send()
            .await
            .map_err(|e| format!("Asset request failed: {e}"))?;

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Asset parse failed: {e}"))?;

        Ok(AssetInfo {
            symbol: json["symbol"].as_str().unwrap_or("").to_string(),
            name: json["name"].as_str().unwrap_or("").to_string(),
            asset_class: json["class"].as_str().unwrap_or("").to_string(),
            tradable: json["tradable"].as_bool().unwrap_or(false),
            marginable: json["marginable"].as_bool().unwrap_or(false),
            shortable: json["shortable"].as_bool().unwrap_or(false),
            fractionable: json["fractionable"].as_bool().unwrap_or(false),
            min_order_size: json["min_order_size"].as_str().and_then(|s| s.parse().ok()),
            min_trade_increment: json["min_trade_increment"].as_str().and_then(|s| s.parse().ok()),
            price_increment: json["price_increment"].as_str().and_then(|s| s.parse().ok()),
        })
    }

    // ── Symbol List ────────────────────────────────────────────────

    pub async fn get_all_assets(&self) -> Result<Vec<AssetInfo>, String> {
        let resp = self
            .client
            .get(format!("{}/v2/assets", self.base_url))
            .headers(self.headers())
            .query(&[("status", "active")])
            .send()
            .await
            .map_err(|e| format!("Assets request failed: {e}"))?;

        let json: Vec<serde_json::Value> = resp
            .json()
            .await
            .map_err(|e| format!("Assets parse failed: {e}"))?;

        Ok(json
            .iter()
            .filter(|a| a["tradable"].as_bool().unwrap_or(false))
            .map(|a| AssetInfo {
                symbol: a["symbol"].as_str().unwrap_or("").to_string(),
                name: a["name"].as_str().unwrap_or("").to_string(),
                asset_class: a["class"].as_str().unwrap_or("").to_string(),
                tradable: true,
                marginable: a["marginable"].as_bool().unwrap_or(false),
                shortable: a["shortable"].as_bool().unwrap_or(false),
                fractionable: a["fractionable"].as_bool().unwrap_or(false),
                min_order_size: a["min_order_size"].as_str().and_then(|s| s.parse().ok()),
                min_trade_increment: a["min_trade_increment"].as_str().and_then(|s| s.parse().ok()),
                price_increment: a["price_increment"].as_str().and_then(|s| s.parse().ok()),
            })
            .collect())
    }

    // ── Historical Data ──────────────────────────────────────────────

    pub async fn get_bars(
        &self,
        symbol: &str,
        timeframe: &str,
        limit: u32,
    ) -> Result<Vec<Bar>, String> {
        let is_crypto = symbol.contains('/');

        // Alpaca doesn't support 1Month — map to 1Week with more bars
        let (actual_tf, actual_limit) = match timeframe {
            "1Month" => ("1Week", limit * 4),
            other => (other, limit),
        };

        // Try multiple feeds in order: sip (paid) → iex (free) for stocks
        // Crypto uses a different endpoint and doesn't need a feed param
        let feeds: Vec<Option<&str>> = if is_crypto {
            vec![None] // crypto endpoint doesn't use feed param
        } else {
            vec![Some("iex"), Some("sip")] // try free tier first
        };

        let base = if is_crypto {
            format!("{}/v1beta3/crypto/us/bars", DATA_BASE)
        } else {
            format!("{}/v2/stocks/{}/bars", DATA_BASE, symbol)
        };

        // Alpaca requires a start date for bar queries — go back far enough
        let start = chrono::Utc::now() - chrono::Duration::days(match actual_tf {
            "1Min" => 5,
            "5Min" | "15Min" => 10,
            "1Hour" => 60,
            "4Hour" => 180,
            "1Day" => 730,
            "1Week" => 1825,
            _ => 365,
        });
        let start_str = start.format("%Y-%m-%dT00:00:00Z").to_string();

        let mut last_error = String::new();

        for feed in &feeds {
            let mut params = vec![
                ("timeframe", actual_tf.to_string()),
                ("limit", actual_limit.to_string()),
                ("start", start_str.clone()),
                ("sort", "asc".to_string()),
            ];
            if let Some(f) = feed {
                params.push(("feed", f.to_string()));
            }
            if is_crypto {
                params.push(("symbols", symbol.to_string()));
            }

            let resp = match self
                .client
                .get(&base)
                .headers(self.headers())
                .query(&params)
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    last_error = format!("Request failed: {e}");
                    continue;
                }
            };

            if !resp.status().is_success() {
                last_error = format!("HTTP {} (feed={:?})", resp.status(), feed);
                let _ = resp.text().await; // consume body
                continue;
            }

            let json: serde_json::Value = match resp.json().await {
                Ok(j) => j,
                Err(e) => {
                    last_error = format!("Parse failed: {e}");
                    continue;
                }
            };

            let bars = Self::parse_bars(&json, symbol, is_crypto);
            if !bars.is_empty() {
                return Ok(bars);
            }
            last_error = format!("Empty bars (feed={:?})", feed);
        }

        Err(format!("No bar data for {symbol} @ {timeframe}: {last_error}"))
    }

    fn parse_bars(json: &serde_json::Value, symbol: &str, is_crypto: bool) -> Vec<Bar> {
        let bars_array = if is_crypto {
            json["bars"][symbol].as_array()
        } else {
            json["bars"].as_array()
        };

        bars_array
            .map(|arr| {
                arr.iter()
                    .map(|b| Bar {
                        timestamp: b["t"].as_str().unwrap_or("").to_string(),
                        open: b["o"].as_f64().unwrap_or(0.0),
                        high: b["h"].as_f64().unwrap_or(0.0),
                        low: b["l"].as_f64().unwrap_or(0.0),
                        close: b["c"].as_f64().unwrap_or(0.0),
                        volume: b["v"].as_f64().unwrap_or(0.0),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}
