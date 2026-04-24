//! Lightweight Alpaca broker client for CLI.
//! Shares the same REST API logic as the main terminal but without Tauri dependencies.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use zeroize::Zeroizing;

const PAPER_BASE: &str = "https://paper-api.alpaca.markets";
const LIVE_BASE: &str = "https://api.alpaca.markets";
const DATA_BASE: &str = "https://data.alpaca.markets";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfo {
    pub equity: f64,
    pub cash: f64,
    pub buying_power: f64,
    pub portfolio_value: f64,
    pub initial_margin: f64,
    pub maintenance_margin: f64,
    pub pattern_day_trader: bool,
    pub trading_blocked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionInfo {
    pub symbol: String,
    pub qty: f64,
    pub side: String,
    pub avg_entry_price: f64,
    pub market_value: f64,
    pub unrealized_pl: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderInfo {
    pub id: String,
    pub symbol: String,
    pub side: String,
    pub qty: String,
    pub order_type: String,
    pub status: String,
    pub limit_price: Option<String>,
    pub stop_price: Option<String>,
    pub created_at: String,
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

fn parse_f64(v: &serde_json::Value, field: &str) -> f64 {
    v[field].as_str().and_then(|s| s.parse().ok())
        .or_else(|| v[field].as_f64())
        .unwrap_or(0.0)
}

#[derive(Clone)]
pub struct AlpacaBroker {
    client: Client,
    base_url: String,
    api_key: Zeroizing<String>,
    secret_key: Zeroizing<String>,
}

impl AlpacaBroker {
    pub fn new(api_key: &str, secret_key: &str, paper: bool) -> Self {
        Self {
            client: Client::new(),
            base_url: if paper { PAPER_BASE } else { LIVE_BASE }.to_string(),
            api_key: Zeroizing::new(api_key.to_string()),
            secret_key: Zeroizing::new(secret_key.to_string()),
        }
    }

    fn headers(&self) -> reqwest::header::HeaderMap {
        let mut h = reqwest::header::HeaderMap::new();
        if let Ok(v) = self.api_key.parse() { h.insert("APCA-API-KEY-ID", v); }
        if let Ok(v) = self.secret_key.parse() { h.insert("APCA-API-SECRET-KEY", v); }
        h
    }

    pub async fn get_account(&self) -> Result<AccountInfo, String> {
        let resp = self.client.get(format!("{}/v2/account", self.base_url))
            .headers(self.headers()).send().await.map_err(|e| e.to_string())?;
        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        Ok(AccountInfo {
            equity: parse_f64(&json, "equity"),
            cash: parse_f64(&json, "cash"),
            buying_power: parse_f64(&json, "buying_power"),
            portfolio_value: parse_f64(&json, "portfolio_value"),
            initial_margin: parse_f64(&json, "initial_margin"),
            maintenance_margin: parse_f64(&json, "maintenance_margin"),
            pattern_day_trader: json["pattern_day_trader"].as_bool().unwrap_or(false),
            trading_blocked: json["trading_blocked"].as_bool().unwrap_or(false),
        })
    }

    pub async fn get_positions(&self) -> Result<Vec<PositionInfo>, String> {
        let resp = self.client.get(format!("{}/v2/positions", self.base_url))
            .headers(self.headers()).send().await.map_err(|e| e.to_string())?;
        let json: Vec<serde_json::Value> = resp.json().await.map_err(|e| e.to_string())?;
        Ok(json.iter().map(|p| PositionInfo {
            symbol: p["symbol"].as_str().unwrap_or("").to_string(),
            qty: parse_f64(p, "qty"),
            side: p["side"].as_str().unwrap_or("").to_string(),
            avg_entry_price: parse_f64(p, "avg_entry_price"),
            market_value: parse_f64(p, "market_value"),
            unrealized_pl: parse_f64(p, "unrealized_pl"),
        }).collect())
    }

    pub async fn get_orders(&self, status: &str, limit: u32) -> Result<Vec<OrderInfo>, String> {
        let resp = self.client.get(format!("{}/v2/orders", self.base_url))
            .headers(self.headers())
            .query(&[("status", status), ("limit", &limit.to_string())])
            .send().await.map_err(|e| e.to_string())?;
        let json: Vec<serde_json::Value> = resp.json().await.map_err(|e| e.to_string())?;
        Ok(json.iter().map(|o| OrderInfo {
            id: o["id"].as_str().unwrap_or("").to_string(),
            symbol: o["symbol"].as_str().unwrap_or("").to_string(),
            side: o["side"].as_str().unwrap_or("").to_string(),
            qty: o["qty"].as_str().unwrap_or("0").to_string(),
            order_type: o["type"].as_str().unwrap_or("").to_string(),
            status: o["status"].as_str().unwrap_or("").to_string(),
            limit_price: o["limit_price"].as_str().map(String::from),
            stop_price: o["stop_price"].as_str().map(String::from),
            created_at: o["created_at"].as_str().unwrap_or("").to_string(),
        }).collect())
    }

    pub async fn market_order(&self, symbol: &str, qty: f64, side: &str) -> Result<OrderResult, String> {
        let mut body = HashMap::new();
        body.insert("symbol", symbol.to_string());
        body.insert("qty", qty.to_string());
        body.insert("side", side.to_string());
        body.insert("type", "market".to_string());
        body.insert("time_in_force", "gtc".to_string());

        let resp = self.client.post(format!("{}/v2/orders", self.base_url))
            .headers(self.headers()).json(&body).send().await.map_err(|e| e.to_string())?;
        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        Ok(OrderResult {
            id: json["id"].as_str().unwrap_or("").to_string(),
            symbol: json["symbol"].as_str().unwrap_or("").to_string(),
            qty: json["qty"].as_str().unwrap_or("0").to_string(),
            side: json["side"].as_str().unwrap_or("").to_string(),
            status: json["status"].as_str().unwrap_or("").to_string(),
        })
    }

    pub async fn close_position(&self, symbol: &str, qty: Option<f64>) -> Result<OrderResult, String> {
        let url = format!("{}/v2/positions/{}", self.base_url, symbol);
        let mut req = self.client.delete(&url).headers(self.headers());
        if let Some(q) = qty {
            req = req.query(&[("qty", q.to_string())]);
        }
        let resp = req.send().await.map_err(|e| e.to_string())?;
        let status_code = resp.status();
        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        if let Some(msg) = json["message"].as_str() {
            if !msg.is_empty() {
                return Err(format!("Close rejected: {msg}"));
            }
        }
        if !status_code.is_success() {
            return Err(format!("Close failed: HTTP {status_code}"));
        }
        Ok(OrderResult {
            id: json["id"].as_str().unwrap_or("").to_string(),
            symbol: json["symbol"].as_str().unwrap_or(symbol).to_string(),
            qty: json["qty"].as_str().unwrap_or("0").to_string(),
            side: json["side"].as_str().unwrap_or("").to_string(),
            status: json["status"].as_str().unwrap_or("closed").to_string(),
        })
    }

    pub async fn cancel_order(&self, order_id: &str) -> Result<(), String> {
        let resp = self.client.delete(format!("{}/v2/orders/{}", self.base_url, order_id))
            .headers(self.headers()).send().await.map_err(|e| e.to_string())?;
        if resp.status().is_success() || resp.status().as_u16() == 204 {
            Ok(())
        } else {
            Err(format!("Cancel failed: HTTP {}", resp.status()))
        }
    }

    pub async fn limit_order(&self, symbol: &str, qty: f64, side: &str, price: f64) -> Result<OrderResult, String> {
        let mut body = HashMap::new();
        body.insert("symbol", symbol.to_string());
        body.insert("qty", qty.to_string());
        body.insert("side", side.to_string());
        body.insert("type", "limit".to_string());
        body.insert("limit_price", format!("{:.2}", price));
        body.insert("time_in_force", "gtc".to_string());

        let resp = self.client.post(format!("{}/v2/orders", self.base_url))
            .headers(self.headers()).json(&body).send().await.map_err(|e| e.to_string())?;
        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        if let Some(msg) = json["message"].as_str() {
            return Err(msg.to_string());
        }
        Ok(OrderResult {
            id: json["id"].as_str().unwrap_or("").to_string(),
            symbol: json["symbol"].as_str().unwrap_or("").to_string(),
            qty: json["qty"].as_str().unwrap_or("0").to_string(),
            side: json["side"].as_str().unwrap_or("").to_string(),
            status: json["status"].as_str().unwrap_or("").to_string(),
        })
    }

    pub async fn stop_order(&self, symbol: &str, qty: f64, side: &str, stop_price: f64) -> Result<OrderResult, String> {
        let mut body = HashMap::new();
        body.insert("symbol", symbol.to_string());
        body.insert("qty", qty.to_string());
        body.insert("side", side.to_string());
        body.insert("type", "stop".to_string());
        body.insert("stop_price", format!("{:.2}", stop_price));
        body.insert("time_in_force", "gtc".to_string());

        let resp = self.client.post(format!("{}/v2/orders", self.base_url))
            .headers(self.headers()).json(&body).send().await.map_err(|e| e.to_string())?;
        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        if let Some(msg) = json["message"].as_str() {
            return Err(msg.to_string());
        }
        Ok(OrderResult {
            id: json["id"].as_str().unwrap_or("").to_string(),
            symbol: json["symbol"].as_str().unwrap_or("").to_string(),
            qty: json["qty"].as_str().unwrap_or("0").to_string(),
            side: json["side"].as_str().unwrap_or("").to_string(),
            status: json["status"].as_str().unwrap_or("").to_string(),
        })
    }

    pub async fn bracket_order(&self, symbol: &str, qty: f64, side: &str, sl: f64, tp: f64) -> Result<OrderResult, String> {
        let body = serde_json::json!({
            "symbol": symbol,
            "qty": qty.to_string(),
            "side": side,
            "type": "market",
            "time_in_force": "gtc",
            "order_class": "bracket",
            "take_profit": { "limit_price": format!("{:.2}", tp) },
            "stop_loss": { "stop_price": format!("{:.2}", sl) },
        });

        let resp = self.client.post(format!("{}/v2/orders", self.base_url))
            .headers(self.headers()).json(&body).send().await.map_err(|e| e.to_string())?;
        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        if let Some(msg) = json["message"].as_str() {
            return Err(msg.to_string());
        }
        Ok(OrderResult {
            id: json["id"].as_str().unwrap_or("").to_string(),
            symbol: json["symbol"].as_str().unwrap_or("").to_string(),
            qty: json["qty"].as_str().unwrap_or("0").to_string(),
            side: json["side"].as_str().unwrap_or("").to_string(),
            status: json["status"].as_str().unwrap_or("").to_string(),
        })
    }

    /// ADR-092: Trailing stop order via Alpaca API.
    pub async fn trailing_stop_order(&self, symbol: &str, qty: f64, side: &str, trail_pct: f64) -> Result<OrderResult, String> {
        let mut body = HashMap::new();
        body.insert("symbol", serde_json::json!(symbol));
        body.insert("qty", serde_json::json!(qty.to_string()));
        body.insert("side", serde_json::json!(side));
        body.insert("type", serde_json::json!("trailing_stop"));
        body.insert("trail_percent", serde_json::json!(trail_pct.to_string()));
        body.insert("time_in_force", serde_json::json!("gtc"));

        let resp = self.client.post(format!("{}/v2/orders", self.base_url))
            .headers(self.headers()).json(&body).send().await.map_err(|e| e.to_string())?;
        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        if let Some(msg) = json.get("message").and_then(|m| m.as_str()) {
            return Err(msg.to_string());
        }
        Ok(OrderResult {
            id: json["id"].as_str().unwrap_or("").to_string(),
            symbol: json["symbol"].as_str().unwrap_or("").to_string(),
            qty: json["qty"].as_str().unwrap_or("").to_string(),
            side: json["side"].as_str().unwrap_or("").to_string(),
            status: json["status"].as_str().unwrap_or("").to_string(),
        })
    }

    pub async fn oco_order(&self, symbol: &str, qty: f64, side: &str, tp: f64, sl: f64, _sl_limit: Option<f64>) -> Result<OrderResult, String> {
        let body = serde_json::json!({
            "symbol": symbol,
            "qty": qty.to_string(),
            "side": side,
            "type": "limit",
            "time_in_force": "gtc",
            "order_class": "oco",
            "take_profit": { "limit_price": format!("{tp:.2}") },
            "stop_loss": { "stop_price": format!("{sl:.2}") },
        });
        let resp = self.client.post(format!("{}/v2/orders", self.base_url))
            .headers(self.headers()).json(&body).send().await.map_err(|e| e.to_string())?;
        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        if let Some(msg) = json.get("message").and_then(|m| m.as_str()) {
            return Err(msg.to_string());
        }
        Ok(OrderResult {
            id: json["id"].as_str().unwrap_or("").to_string(),
            symbol: json["symbol"].as_str().unwrap_or("").to_string(),
            qty: json["qty"].as_str().unwrap_or("").to_string(),
            side: json["side"].as_str().unwrap_or("").to_string(),
            status: json["status"].as_str().unwrap_or("").to_string(),
        })
    }

    pub async fn close_all(&self) -> Result<(), String> {
        let resp = self.client.delete(format!("{}/v2/positions", self.base_url))
            .headers(self.headers()).send().await.map_err(|e| e.to_string())?;
        if resp.status().is_success() || resp.status().as_u16() == 207 {
            Ok(())
        } else {
            Err(format!("Close all failed: HTTP {}", resp.status()))
        }
    }

    pub async fn cancel_all(&self) -> Result<(), String> {
        let resp = self.client.delete(format!("{}/v2/orders", self.base_url))
            .headers(self.headers()).send().await.map_err(|e| e.to_string())?;
        if resp.status().is_success() || resp.status().as_u16() == 207 {
            Ok(())
        } else {
            Err(format!("Cancel all failed: HTTP {}", resp.status()))
        }
    }

    pub async fn get_order_history(&self, limit: u32) -> Result<Vec<OrderInfo>, String> {
        let resp = self.client.get(format!("{}/v2/orders", self.base_url))
            .headers(self.headers())
            .query(&[("status", "closed"), ("limit", &limit.to_string()), ("direction", "desc")])
            .send().await.map_err(|e| e.to_string())?;
        let json: Vec<serde_json::Value> = resp.json().await.map_err(|e| e.to_string())?;
        Ok(json.iter().map(|o| OrderInfo {
            id: o["id"].as_str().unwrap_or("").to_string(),
            symbol: o["symbol"].as_str().unwrap_or("").to_string(),
            side: o["side"].as_str().unwrap_or("").to_string(),
            qty: o["qty"].as_str().unwrap_or("0").to_string(),
            order_type: o["type"].as_str().unwrap_or("").to_string(),
            status: o["status"].as_str().unwrap_or("").to_string(),
            limit_price: o["limit_price"].as_str().map(String::from),
            stop_price: o["stop_price"].as_str().map(String::from),
            created_at: o["created_at"].as_str().unwrap_or("").to_string(),
        }).collect())
    }

    pub async fn get_quote(&self, symbol: &str) -> Result<(f64, f64, f64), String> {
        let is_crypto = symbol.contains('/');
        let url = if is_crypto {
            format!("{}/v1beta3/crypto/us/latest/quotes?symbols={}", DATA_BASE, symbol)
        } else {
            format!("{}/v2/stocks/{}/quotes/latest", DATA_BASE, symbol)
        };

        let resp = self.client.get(&url)
            .headers(self.headers())
            .send().await.map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("HTTP {}", resp.status()));
        }

        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

        if is_crypto {
            let q = &json["quotes"][symbol];
            let bid = q["bp"].as_f64().unwrap_or(0.0);
            let ask = q["ap"].as_f64().unwrap_or(0.0);
            let last = (bid + ask) / 2.0;
            Ok((bid, ask, last))
        } else {
            let q = &json["quote"];
            let bid = q["bp"].as_f64().unwrap_or(0.0);
            let ask = q["ap"].as_f64().unwrap_or(0.0);
            let last = (bid + ask) / 2.0;
            Ok((bid, ask, last))
        }
    }

    /// Check if US stock market is currently open via Alpaca clock endpoint.
    pub async fn get_clock(&self) -> Result<(bool, String), String> {
        let resp = self.client.get(format!("{}/v2/clock", self.base_url))
            .headers(self.headers()).send().await.map_err(|e| e.to_string())?;
        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        let is_open = json["is_open"].as_bool().unwrap_or(false);
        let next_event = if is_open {
            json["next_close"].as_str().unwrap_or("").to_string()
        } else {
            json["next_open"].as_str().unwrap_or("").to_string()
        };
        Ok((is_open, next_event))
    }

    pub async fn get_bars(&self, symbol: &str, timeframe: &str, limit: u32) -> Result<Vec<Bar>, String> {
        let is_crypto = symbol.contains('/');
        let base = if is_crypto {
            format!("{}/v1beta3/crypto/us/bars", DATA_BASE)
        } else {
            format!("{}/v2/stocks/{}/bars", DATA_BASE, symbol)
        };

        // Tighter lookback for crypto (24/7 trading = more bars per day)
        let lookback_days: i64 = if is_crypto {
            match timeframe {
                "1Min" => 3, "5Min" | "15Min" | "30Min" => 14,
                "1Hour" => 90, "4Hour" => 180,
                _ => 365,
            }
        } else { 365 };
        let lookback = chrono::Utc::now() - chrono::Duration::days(lookback_days);
        let start = lookback.format("%Y-%m-%dT00:00:00Z").to_string();

        let mut params = vec![
            ("timeframe", timeframe.to_string()),
            ("limit", limit.to_string()),
            ("start", start),
            ("sort", "asc".to_string()),
        ];
        if is_crypto {
            params.push(("symbols", symbol.to_string()));
        } else {
            params.push(("feed", "iex".to_string()));
        }

        let resp = self.client.get(&base)
            .headers(self.headers())
            .query(&params)
            .send().await.map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("HTTP {}", resp.status()));
        }

        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

        let bars_array = if is_crypto {
            json["bars"][symbol].as_array()
        } else {
            json["bars"].as_array()
        };

        Ok(bars_array.map(|arr| {
            arr.iter().map(|b| Bar {
                timestamp: b["t"].as_str().unwrap_or("").to_string(),
                open: b["o"].as_f64().unwrap_or(0.0),
                high: b["h"].as_f64().unwrap_or(0.0),
                low: b["l"].as_f64().unwrap_or(0.0),
                close: b["c"].as_f64().unwrap_or(0.0),
                volume: b["v"].as_f64().unwrap_or(0.0),
            }).collect()
        }).unwrap_or_default())
    }

    /// Search all tradable assets by symbol or name.
    pub async fn search_symbols(&self, query: &str) -> Result<Vec<(String, String, String)>, String> {
        let resp = self.client.get(format!("{}/v2/assets", self.base_url))
            .headers(self.headers())
            .query(&[("status", "active")])
            .send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(format!("HTTP {}", resp.status())); }
        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        let q = query.to_uppercase();
        let mut matches: Vec<(u8, String, String, String)> = Vec::new();
        if let Some(arr) = json.as_array() {
            for a in arr {
                if a["tradable"].as_bool() != Some(true) { continue; }
                let sym = a["symbol"].as_str().unwrap_or("").to_uppercase();
                let name = a["name"].as_str().unwrap_or("").to_string();
                let class = a["class"].as_str().unwrap_or("").to_string();
                let sym_no_slash = sym.replace('/', "");
                let pri = if sym == q || sym_no_slash == q { 0 }
                    else if sym.starts_with(&q) || sym_no_slash.starts_with(&q) { 1 }
                    else if sym.contains(&q) || sym_no_slash.contains(&q) { 2 }
                    else if name.to_uppercase().contains(&q) { 3 }
                    else { continue; };
                matches.push((pri, sym, name, class));
            }
        }
        matches.sort_by_key(|(pri, _, _, _)| *pri);
        Ok(matches.into_iter().take(20).map(|(_, s, n, c)| (s, n, c)).collect())
    }

    /// Get all tradeable assets grouped by asset class.
    pub async fn list_all_symbols(&self) -> Result<Vec<(String, String, String)>, String> {
        let resp = self.client.get(format!("{}/v2/assets", self.base_url))
            .headers(self.headers())
            .query(&[("status", "active")])
            .send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(format!("HTTP {}", resp.status())); }
        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        let mut symbols: Vec<(String, String, String)> = Vec::new();
        if let Some(arr) = json.as_array() {
            for a in arr {
                if a["tradable"].as_bool() != Some(true) { continue; }
                let sym = a["symbol"].as_str().unwrap_or("").to_string();
                let name = a["name"].as_str().unwrap_or("").to_string();
                let class = a["class"].as_str().unwrap_or("us_equity").to_string();
                symbols.push((sym, name, class));
            }
        }
        symbols.sort_by(|a, b| a.2.cmp(&b.2).then(a.0.cmp(&b.0)));
        Ok(symbols)
    }

    /// Get top market movers (most active by volume).
    pub async fn get_top_movers(&self) -> Result<Vec<(String, f64, f64)>, String> {
        let resp = self.client.get(format!("{}/v1beta1/screener/stocks/most-actives", DATA_BASE))
            .headers(self.headers())
            .query(&[("top", "20")])
            .send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(format!("HTTP {}", resp.status())); }
        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        let movers = json["most_actives"].as_array()
            .map(|arr| arr.iter().map(|m| {
                let sym = m["symbol"].as_str().unwrap_or("").to_string();
                let price = m["price"].as_f64().unwrap_or(0.0);
                let change = m["change"].as_f64().unwrap_or(0.0);
                (sym, price, change)
            }).collect())
            .unwrap_or_default();
        Ok(movers)
    }

    /// Get recent account activities (fills).
    pub async fn get_activities(&self, limit: u32) -> Result<Vec<(String, String, String, String, String)>, String> {
        let resp = self.client.get(format!("{}/v2/account/activities/FILL", self.base_url))
            .headers(self.headers())
            .query(&[("page_size", &limit.to_string())])
            .send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(format!("HTTP {}", resp.status())); }
        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        let fills = json.as_array()
            .map(|arr| arr.iter().map(|a| {
                let sym = a["symbol"].as_str().unwrap_or("").to_string();
                let side = a["side"].as_str().unwrap_or("").to_string();
                let qty = a["qty"].as_str().unwrap_or("0").to_string();
                let price = a["price"].as_str().unwrap_or("0").to_string();
                let ts = a["transaction_time"].as_str().unwrap_or("").to_string();
                (ts, sym, side, qty, price)
            }).collect())
            .unwrap_or_default();
        Ok(fills)
    }
}
