//! Alpaca broker interface.
//!
//! Wraps Alpaca REST API and WebSocket streaming.
//! Provides the same operations as MQL5 CTrade: open, close, partial close, modify, account info.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use tokio::sync::Mutex;
use zeroize::Zeroizing;
use futures_util::{SinkExt, StreamExt};

/// Shared HTTP client for SEC EDGAR requests (reuses TCP connections).
fn sec_client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .pool_max_idle_per_host(2)
            .build()
            .expect("Failed to build SEC HTTP client")
    })
}

/// Cached SEC ticker→CIK map. Fetched once (~8MB), reused for all lookups.
static SEC_TICKER_MAP: tokio::sync::OnceCell<serde_json::Value> = tokio::sync::OnceCell::const_new();

async fn get_sec_ticker_map() -> Result<&'static serde_json::Value, String> {
    SEC_TICKER_MAP.get_or_try_init(|| async {
        let client = sec_client();
        let resp = client
            .get("https://www.sec.gov/files/company_tickers.json")
            .header("User-Agent", "TyphooN-Terminal/0.1 (support@marketwizardry.org)")
            .send()
            .await
            .map_err(|_| "SEC ticker map request failed".to_string())?;
        resp.json::<serde_json::Value>().await
            .map_err(|_| "SEC ticker map parse failed".to_string())
    }).await
}

/// Look up CIK number for a ticker symbol from cached SEC ticker map.
async fn lookup_cik(ticker: &str) -> Result<u64, String> {
    let tickers = get_sec_ticker_map().await?;
    let upper_ticker = ticker.to_uppercase();
    if let Some(obj) = tickers.as_object() {
        for (_, v) in obj {
            if v["ticker"].as_str() == Some(&upper_ticker) {
                if let Some(cik) = v["cik_str"].as_u64()
                    .or_else(|| v["cik_str"].as_str().and_then(|s| s.parse().ok()))
                {
                    return Ok(cik);
                }
            }
        }
    }
    Err(format!("CIK not found for {ticker}"))
}

const PAPER_BASE: &str = "https://paper-api.alpaca.markets";
const LIVE_BASE: &str = "https://api.alpaca.markets";
const DATA_BASE: &str = "https://data.alpaca.markets";

/// Alpaca free plan: 200 requests/minute = 1 request per 300ms.
/// We use 320ms to leave headroom.
const RATE_LIMIT_MS: u64 = 320;

/// Round price to valid increment for Alpaca orders.
/// Stocks > $1: 2 decimal places (penny). Stocks < $1: 4 decimal places (sub-penny allowed).
/// Crypto: 8 decimal places.
fn round_price(price: f64) -> String {
    if price >= 1.0 {
        format!("{:.2}", price) // $1+ → 2 decimals (e.g., 15.68)
    } else if price >= 0.01 {
        format!("{:.4}", price) // $0.01-$0.99 → 4 decimals
    } else {
        format!("{:.8}", price) // sub-penny / crypto → 8 decimals
    }
}

/// Centralized rate limiter — shared across all data API requests.
/// On 429, pauses all requests for a cooldown period.
#[derive(Debug, Clone)]
pub struct RateLimiter {
    last_request: Arc<Mutex<std::time::Instant>>,
    cooldown_until: Arc<Mutex<Option<std::time::Instant>>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            last_request: Arc::new(Mutex::new(std::time::Instant::now() - std::time::Duration::from_secs(1))),
            cooldown_until: Arc::new(Mutex::new(None)),
        }
    }

    /// Wait until we can make another request without hitting rate limit.
    /// Returns false if in cooldown (caller should skip/fail gracefully).
    pub async fn wait(&self) -> bool {
        // Check cooldown
        {
            let cooldown = self.cooldown_until.lock().await;
            if let Some(until) = *cooldown {
                if std::time::Instant::now() < until {
                    let remaining = until - std::time::Instant::now();
                    tracing::debug!("Rate limiter in cooldown for {}ms", remaining.as_millis());
                    tokio::time::sleep(remaining).await;
                }
            }
        }
        // Normal pacing
        let mut last = self.last_request.lock().await;
        let elapsed = last.elapsed();
        let min_interval = std::time::Duration::from_millis(RATE_LIMIT_MS);
        if elapsed < min_interval {
            tokio::time::sleep(min_interval - elapsed).await;
        }
        *last = std::time::Instant::now();
        true
    }

    /// Called when a 429 response is received. Pauses all requests for 60 seconds.
    pub async fn trigger_cooldown(&self) {
        let mut cooldown = self.cooldown_until.lock().await;
        let until = std::time::Instant::now() + std::time::Duration::from_secs(60);
        *cooldown = Some(until);
        tracing::warn!("Rate limit hit — cooling down for 60s");
    }
}

#[derive(Debug, Clone)]
pub struct AlpacaBroker {
    client: Client,
    base_url: String,
    api_key: Zeroizing<String>,
    secret_key: Zeroizing<String>,
    rate_limiter: RateLimiter,
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

/// Full order info for history/management.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderInfo {
    pub id: String,
    pub symbol: String,
    pub qty: String,
    pub filled_qty: String,
    pub side: String,
    pub order_type: String,
    pub status: String,
    pub limit_price: Option<String>,
    pub stop_price: Option<String>,
    pub trail_price: Option<String>,
    pub trail_percent: Option<String>,
    pub created_at: String,
    pub filled_at: Option<String>,
    pub filled_avg_price: Option<String>,
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
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .pool_max_idle_per_host(5)
                .tcp_keepalive(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to build HTTP client"),
            base_url,
            api_key: Zeroizing::new(api_key),
            secret_key: Zeroizing::new(secret_key),
            rate_limiter: RateLimiter::new(),
        }
    }

    fn headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        if let Ok(key) = self.api_key.parse() {
            headers.insert("APCA-API-KEY-ID", key);
        }
        if let Ok(secret) = self.secret_key.parse() {
            headers.insert("APCA-API-SECRET-KEY", secret);
        }
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

    /// Place a limit order.
    pub async fn limit_order(&self, symbol: &str, qty: f64, side: &str, limit_price: f64, tif: &str) -> Result<OrderResult, String> {
        let body = serde_json::json!({
            "symbol": symbol,
            "qty": qty.to_string(),
            "side": side,
            "type": "limit",
            "limit_price": round_price(limit_price),
            "time_in_force": tif,
        });
        self.submit_order(&body).await
    }

    /// Place a stop order.
    pub async fn stop_order(&self, symbol: &str, qty: f64, side: &str, stop_price: f64, tif: &str) -> Result<OrderResult, String> {
        let body = serde_json::json!({
            "symbol": symbol,
            "qty": qty.to_string(),
            "side": side,
            "type": "stop",
            "stop_price": round_price(stop_price),
            "time_in_force": tif,
        });
        self.submit_order(&body).await
    }

    /// Place a stop-limit order.
    pub async fn stop_limit_order(&self, symbol: &str, qty: f64, side: &str, stop_price: f64, limit_price: f64, tif: &str) -> Result<OrderResult, String> {
        let body = serde_json::json!({
            "symbol": symbol,
            "qty": qty.to_string(),
            "side": side,
            "type": "stop_limit",
            "stop_price": round_price(stop_price),
            "limit_price": round_price(limit_price),
            "time_in_force": tif,
        });
        self.submit_order(&body).await
    }

    /// Place a trailing stop order.
    pub async fn trailing_stop_order(&self, symbol: &str, qty: f64, side: &str, trail_price: Option<f64>, trail_percent: Option<f64>, tif: &str) -> Result<OrderResult, String> {
        let mut body = serde_json::json!({
            "symbol": symbol,
            "qty": qty.to_string(),
            "side": side,
            "type": "trailing_stop",
            "time_in_force": tif,
        });
        if let Some(tp) = trail_price {
            body["trail_price"] = serde_json::json!(round_price(tp));
        }
        if let Some(tp) = trail_percent {
            body["trail_percent"] = serde_json::json!(round_price(tp));
        }
        self.submit_order(&body).await
    }

    /// Place a bracket order (market entry with TP + SL legs).
    pub async fn bracket_order(&self, symbol: &str, qty: f64, side: &str, tp_price: f64, sl_price: f64) -> Result<OrderResult, String> {
        // Round prices to valid increments (Alpaca rejects sub-penny for stocks > $1)
        let tp_rounded = round_price(tp_price);
        let sl_rounded = round_price(sl_price);
        let body = serde_json::json!({
            "symbol": symbol,
            "qty": qty.to_string(),
            "side": side,
            "type": "market",
            "time_in_force": "gtc",
            "order_class": "bracket",
            "take_profit": { "limit_price": tp_rounded },
            "stop_loss": { "stop_price": sl_rounded },
        });
        self.submit_order(&body).await
    }

    /// Common order submission logic.
    async fn submit_order(&self, body: &serde_json::Value) -> Result<OrderResult, String> {
        let resp = self
            .client
            .post(format!("{}/v2/orders", self.base_url))
            .headers(self.headers())
            .json(body)
            .send()
            .await
            .map_err(|e| format!("Order request failed: {e}"))?;

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Order parse failed: {e}"))?;

        if let Some(msg) = json["message"].as_str() {
            if !msg.is_empty() {
                return Err(format!("Order rejected: {msg}"));
            }
        }

        Ok(OrderResult {
            id: json["id"].as_str().unwrap_or("").to_string(),
            symbol: json["symbol"].as_str().unwrap_or("").to_string(),
            qty: json["qty"].as_str().unwrap_or("0").to_string(),
            side: json["side"].as_str().unwrap_or("").to_string(),
            status: json["status"].as_str().unwrap_or("").to_string(),
        })
    }

    /// Get orders by status (open, closed, all).
    pub async fn get_orders(&self, status: &str, limit: u32) -> Result<Vec<OrderInfo>, String> {
        let resp = self
            .client
            .get(format!("{}/v2/orders", self.base_url))
            .headers(self.headers())
            .query(&[
                ("status", status),
                ("limit", &limit.to_string()),
                ("direction", "desc"),
            ])
            .send()
            .await
            .map_err(|e| format!("Orders request failed: {e}"))?;

        if !resp.status().is_success() {
            let status_code = resp.status();
            let _ = resp.text().await;
            return Err(format!("Orders request failed: HTTP {status_code}"));
        }

        // Parse as generic Value first — handle both array and error responses
        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Orders parse failed: {e}"))?;

        let orders = match json.as_array() {
            Some(arr) => arr.iter().map(Self::parse_order_info).collect(),
            None => {
                // Alpaca might return an object with a message on error
                tracing::warn!("Orders response was not an array: {}", json);
                vec![]
            }
        };
        Ok(orders)
    }

    /// Modify a pending order.
    pub async fn modify_order(&self, order_id: &str, qty: Option<f64>, limit_price: Option<f64>, stop_price: Option<f64>, trail: Option<f64>) -> Result<OrderResult, String> {
        let mut body = serde_json::Map::new();
        if let Some(q) = qty { body.insert("qty".into(), serde_json::json!(q.to_string())); }
        if let Some(lp) = limit_price { body.insert("limit_price".into(), serde_json::json!(round_price(lp))); }
        if let Some(sp) = stop_price { body.insert("stop_price".into(), serde_json::json!(round_price(sp))); }
        if let Some(t) = trail { body.insert("trail".into(), serde_json::json!(t.to_string())); }

        let resp = self
            .client
            .patch(format!("{}/v2/orders/{}", self.base_url, order_id))
            .headers(self.headers())
            .json(&serde_json::Value::Object(body))
            .send()
            .await
            .map_err(|e| format!("Modify order failed: {e}"))?;

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Modify parse failed: {e}"))?;

        Ok(OrderResult {
            id: json["id"].as_str().unwrap_or("").to_string(),
            symbol: json["symbol"].as_str().unwrap_or("").to_string(),
            qty: json["qty"].as_str().unwrap_or("0").to_string(),
            side: json["side"].as_str().unwrap_or("").to_string(),
            status: json["status"].as_str().unwrap_or("").to_string(),
        })
    }

    /// Cancel a pending order.
    pub async fn cancel_order(&self, order_id: &str) -> Result<(), String> {
        self.client
            .delete(format!("{}/v2/orders/{}", self.base_url, order_id))
            .headers(self.headers())
            .send()
            .await
            .map_err(|e| format!("Cancel order failed: {e}"))?;
        Ok(())
    }

    fn parse_order_info(o: &serde_json::Value) -> OrderInfo {
        OrderInfo {
            id: o["id"].as_str().unwrap_or("").to_string(),
            symbol: o["symbol"].as_str().unwrap_or("").to_string(),
            qty: o["qty"].as_str().unwrap_or("0").to_string(),
            filled_qty: o["filled_qty"].as_str().unwrap_or("0").to_string(),
            side: o["side"].as_str().unwrap_or("").to_string(),
            order_type: o["type"].as_str().unwrap_or("").to_string(),
            status: o["status"].as_str().unwrap_or("").to_string(),
            limit_price: o["limit_price"].as_str().map(|s| s.to_string()),
            stop_price: o["stop_price"].as_str().map(|s| s.to_string()),
            trail_price: o["trail_price"].as_str().map(|s| s.to_string()),
            trail_percent: o["trail_percent"].as_str().map(|s| s.to_string()),
            created_at: o["created_at"].as_str().unwrap_or("").to_string(),
            filled_at: o["filled_at"].as_str().map(|s| s.to_string()),
            filled_avg_price: o["filled_avg_price"].as_str().map(|s| s.to_string()),
        }
    }

    pub async fn close_position(&self, symbol: &str, qty: Option<f64>) -> Result<OrderResult, String> {
        // Alpaca position endpoint uses symbol without slash (BTC/USD → BTCUSD)
        let encoded_symbol = symbol.replace('/', "%2F");
        let url = if let Some(q) = qty {
            format!("{}/v2/positions/{}?qty={}", self.base_url, encoded_symbol, q)
        } else {
            format!("{}/v2/positions/{}", self.base_url, encoded_symbol)
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
        let encoded_symbol = symbol.replace('/', "%2F");
        let resp = self
            .client
            .get(format!("{}/v2/assets/{}", self.base_url, encoded_symbol))
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

    // ── News ─────────────────────────────────────────────────────

    pub async fn get_news(&self, symbol: &str, limit: u32) -> Result<Vec<serde_json::Value>, String> {
        self.rate_limiter.wait().await;
        let resp = self
            .client
            .get(format!("{}/v1beta1/news", DATA_BASE))
            .headers(self.headers())
            .query(&[
                ("symbols", symbol),
                ("limit", &limit.to_string()),
                ("sort", "desc"),
            ])
            .send()
            .await
            .map_err(|e| format!("News request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let _ = resp.text().await; // consume body without exposing it
            return Err(format!("News request failed: HTTP {}", status));
        }

        let json: serde_json::Value = resp.json().await
            .map_err(|e| format!("News parse failed: {e}"))?;

        Ok(json["news"].as_array().cloned().unwrap_or_default())
    }

    // ── Finnhub News (secondary source, free API key) ──────────────

    pub async fn get_finnhub_news(&self, symbol: &str, finnhub_key: &str) -> Result<Vec<serde_json::Value>, String> {
        if finnhub_key.is_empty() { return Ok(vec![]); }
        // Strip /USD for crypto symbols (Finnhub uses BINANCE:BTCUSDT format for crypto)
        let clean_sym = symbol.replace("/USD", "").replace("/", "");

        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let week_ago = (chrono::Utc::now() - chrono::Duration::days(7)).format("%Y-%m-%d").to_string();

        let resp = sec_client()
            .get("https://finnhub.io/api/v1/company-news")
            .query(&[
                ("symbol", clean_sym.as_str()),
                ("from", week_ago.as_str()),
                ("to", today.as_str()),
                ("token", finnhub_key),
            ])
            .send()
            .await
            .map_err(|e| format!("Finnhub news failed: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("Finnhub news: HTTP {}", resp.status()));
        }

        let articles: Vec<serde_json::Value> = resp.json().await
            .map_err(|e| format!("Finnhub parse failed: {e}"))?;

        // Normalize to same format as Alpaca news
        Ok(articles.iter().map(|a| {
            serde_json::json!({
                "headline": a["headline"].as_str().unwrap_or(""),
                "summary": a["summary"].as_str().unwrap_or(""),
                "url": a["url"].as_str().unwrap_or(""),
                "source": a["source"].as_str().unwrap_or("Finnhub"),
                "created_at": chrono::DateTime::from_timestamp(a["datetime"].as_i64().unwrap_or(0), 0)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default(),
                "images": [],
            })
        }).take(20).collect())
    }

    // ── Corporate Actions (Earnings/Dividends) ──────────────────

    pub async fn get_corporate_actions(&self, symbol: &str, types: &str) -> Result<Vec<serde_json::Value>, String> {
        self.rate_limiter.wait().await;
        // Alpaca corporate actions endpoint
        let resp = self
            .client
            .get(format!("{}/v1/corporate-actions", self.base_url))
            .headers(self.headers())
            .query(&[
                ("symbols", symbol),
                ("types", types), // "dividend", "merger", "spinoff", "split"
            ])
            .send()
            .await;

        match resp {
            Ok(r) if r.status().is_success() => {
                let json: serde_json::Value = r.json().await
                    .map_err(|e| format!("Corporate actions parse failed: {e}"))?;
                Ok(json.as_array().cloned().unwrap_or_default())
            }
            Ok(r) => Err(format!("Corporate actions: HTTP {}", r.status())),
            Err(e) => Err(format!("Corporate actions request failed: {e}")),
        }
    }

    // ── SEC EDGAR Filings ─────────────────────────────────────────

    /// Fetch recent SEC filings for a company via EDGAR API (free, no auth).
    /// CIK lookup by ticker, then fetch filings.
    /// Hardened: uses .query() to prevent URL parameter injection.
    pub async fn get_sec_filings(ticker: &str, filing_type: &str, _limit: u32) -> Result<serde_json::Value, String> {
        // Validate ticker format (alphanumeric only, no injection)
        if ticker.is_empty() || ticker.len() > 10 || !ticker.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err("Invalid ticker for SEC lookup".to_string());
        }
        // Validate filing type
        if !matches!(filing_type, "10-K" | "10-Q" | "8-K" | "S-1" | "DEF 14A" | "13F" | "4" | "SC 13D" | "SC 13G") {
            return Err("Invalid filing type".to_string());
        }
        let client = sec_client();

        let quoted_ticker = format!("\"{}\"", ticker);
        let cik_resp = client
            .get("https://efts.sec.gov/LATEST/search-index")
            .query(&[
                ("q", quoted_ticker.as_str()),
                ("dateRange", "custom"),
                ("startdt", "2020-01-01"),
                ("forms", filing_type),
            ])
            .header("User-Agent", "TyphooN-Terminal/0.1 (support@marketwizardry.org)")
            .send()
            .await
            .map_err(|_| "SEC EDGAR request failed".to_string())?;

        if !cik_resp.status().is_success() {
            let search_resp = client
                .get("https://efts.sec.gov/LATEST/search-index")
                .query(&[
                    ("q", ticker),
                    ("forms", filing_type),
                    ("dateRange", "custom"),
                    ("startdt", "2020-01-01"),
                ])
                .header("User-Agent", "TyphooN-Terminal/0.1 (support@marketwizardry.org)")
                .send()
                .await
                .map_err(|_| "SEC EDGAR search failed".to_string())?;

            let json: serde_json::Value = search_resp.json().await
                .map_err(|_| "SEC EDGAR parse failed".to_string())?;
            return Ok(json);
        }

        let json: serde_json::Value = cik_resp.json().await
            .map_err(|_| "SEC EDGAR parse failed".to_string())?;
        Ok(json)
    }

    /// Fetch company facts from SEC EDGAR (financials, shares outstanding, etc.)
    /// Hardened: timeout, validated ticker, generic error messages.
    /// Uses cached ticker map and shared HTTP client.
    pub async fn get_sec_company_facts(ticker: &str) -> Result<serde_json::Value, String> {
        if ticker.is_empty() || ticker.len() > 10 || !ticker.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err("Invalid ticker for SEC lookup".to_string());
        }
        let client = sec_client();
        let cik = lookup_cik(ticker).await?;
        let cik_padded = format!("CIK{:010}", cik);

        let facts_resp = client
            .get(format!("https://data.sec.gov/api/xbrl/companyfacts/{}.json", cik_padded))
            .header("User-Agent", "TyphooN-Terminal/0.1 (support@marketwizardry.org)")
            .send()
            .await
            .map_err(|e| format!("SEC company facts failed: {e}"))?;

        if !facts_resp.status().is_success() {
            return Err(format!("SEC company facts: HTTP {}", facts_resp.status()));
        }

        let facts: serde_json::Value = facts_resp.json().await
            .map_err(|e| format!("SEC facts parse failed: {e}"))?;

        // Extract key metrics
        let us_gaap = &facts["facts"]["us-gaap"];
        let result = serde_json::json!({
            "cik": cik,
            "entity": facts["entityName"],
            "revenue": extract_latest_fact(us_gaap, "Revenues"),
            "net_income": extract_latest_fact(us_gaap, "NetIncomeLoss"),
            "total_assets": extract_latest_fact(us_gaap, "Assets"),
            "total_liabilities": extract_latest_fact(us_gaap, "Liabilities"),
            "shares_outstanding": extract_latest_fact(us_gaap, "CommonStockSharesOutstanding"),
            "stockholders_equity": extract_latest_fact(us_gaap, "StockholdersEquity"),
            "eps": extract_latest_fact(us_gaap, "EarningsPerShareBasic"),
        });

        Ok(result)
    }
}

fn extract_latest_fact(gaap: &serde_json::Value, concept: &str) -> serde_json::Value {
    if let Some(units) = gaap[concept]["units"].as_object() {
        for (_, entries) in units {
            if let Some(arr) = entries.as_array() {
                if let Some(last) = arr.last() {
                    return serde_json::json!({
                        "value": last["val"],
                        "period": last["end"],
                        "form": last["form"],
                    });
                }
            }
        }
    }
    serde_json::Value::Null
}

impl AlpacaBroker {
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

        // Alpaca doesn't support 1Month — fetch weekly bars and aggregate
        if timeframe == "1Month" {
            // Fetch enough weekly bars: ~4.3 weeks per month, request 5x for safety
            let weekly = Box::pin(self.get_bars(symbol, "1Week", (limit * 5).max(1000))).await?;
            let monthly = Self::aggregate_weekly_to_monthly(&weekly);
            let trimmed = if monthly.len() > limit as usize {
                monthly[monthly.len() - limit as usize..].to_vec()
            } else {
                monthly
            };
            return Ok(trimmed);
        }

        let actual_tf = timeframe;
        let actual_limit = limit;

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

        // Bars per calendar day — crypto trades 24/7, stocks ~6.5h/day
        let bars_per_day: f64 = if is_crypto {
            match actual_tf {
                "1Min" => 1440.0, "5Min" => 288.0, "15Min" => 96.0,
                "30Min" => 48.0, "1Hour" => 24.0, "4Hour" => 6.0,
                "1Day" => 1.0, "1Week" => 0.14, _ => 1.0,
            }
        } else {
            match actual_tf {
                "1Min" => 390.0, "5Min" => 78.0, "15Min" => 26.0,
                "30Min" => 13.0, "1Hour" => 7.0, "4Hour" => 2.0,
                "1Day" => 0.7, "1Week" => 0.14, _ => 0.7,
            }
        };
        let max_lookback_days: i64 = match actual_tf {
            "1Min" => 7,
            "5Min" | "15Min" | "30Min" => 30,
            "1Hour" => 365,
            "4Hour" => 730,
            "1Day" => 3650,
            "1Week" => 7300,
            _ => 1825,
        };
        // Proportional lookback: bars_needed / bars_per_day * 1.5 safety margin
        let proportional_days = ((actual_limit as f64 / bars_per_day) * 1.5).ceil() as i64;
        let lookback_days = proportional_days.max(7).min(max_lookback_days);
        let earliest_start = chrono::Utc::now() - chrono::Duration::days(lookback_days);

        let mut last_error = String::new();

        for feed in &feeds {
            let mut all_bars: Vec<Bar> = Vec::new();
            // Sequential chunk fetching: IEX caps at ~260 bars per request.
            // We fetch chunks from oldest to newest, advancing the start date each time.
            let mut chunk_start = earliest_start;
            let mut consecutive_empty = 0;

            loop {
                // Centralized rate limiter — respects global request budget
                self.rate_limiter.wait().await;

                let start_str = chunk_start.format("%Y-%m-%dT00:00:00Z").to_string();
                let mut params = vec![
                    ("timeframe", actual_tf.to_string()),
                    ("limit", "10000".to_string()), // max per-request; total capped by actual_limit below
                    ("start", start_str),
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
                        break;
                    }
                };

                if !resp.status().is_success() {
                    let status = resp.status();
                    if status.as_u16() == 429 {
                        self.rate_limiter.trigger_cooldown().await;
                        // Return what we have so far rather than retrying
                        if !all_bars.is_empty() {
                            tracing::info!("429 hit, returning {} bars collected so far", all_bars.len());
                            break;
                        }
                    }
                    last_error = format!("HTTP {} (feed={:?})", status, feed);
                    let _ = resp.text().await;
                    break;
                }

                let json: serde_json::Value = match resp.json().await {
                    Ok(j) => j,
                    Err(e) => {
                        last_error = format!("Parse failed: {e}");
                        break;
                    }
                };

                let chunk_bars = Self::parse_bars(&json, symbol, is_crypto);
                let chunk_count = chunk_bars.len();

                if chunk_count == 0 {
                    consecutive_empty += 1;
                    if consecutive_empty >= 3 {
                        break; // no more data available
                    }
                    // Advance start by a chunk to skip empty gaps
                    chunk_start = chunk_start + chrono::Duration::days(match actual_tf {
                        "1Min" | "5Min" | "15Min" | "30Min" => 1,
                        "1Hour" => 30,
                        "4Hour" => 90,
                        "1Day" => 365,
                        "1Week" => 730,
                        _ => 90,
                    });
                    continue;
                }

                consecutive_empty = 0;

                // Detect stale chunk: if the chunk's last bar date matches what we already
                // have, we've reached the end of available data (API keeps returning same bars)
                if !all_bars.is_empty() {
                    let last_existing_date = all_bars.last().map(|b| &b.timestamp[..10.min(b.timestamp.len())]);
                    let last_new_date = chunk_bars.last().map(|b| &b.timestamp[..10.min(b.timestamp.len())]);
                    if last_existing_date == last_new_date {
                        tracing::info!("{} @ {}: reached end of data ({} bars total)", symbol, actual_tf, all_bars.len());
                        break;
                    }
                }

                // Log chunk progress with date range (only for meaningful chunks)
                if chunk_count > 5 {
                    if let (Some(first), Some(last)) = (chunk_bars.first(), chunk_bars.last()) {
                        let first_date = &first.timestamp[..10.min(first.timestamp.len())];
                        let last_date = &last.timestamp[..10.min(last.timestamp.len())];
                        tracing::info!(
                            "{} @ {}: chunk +{} bars ({} → {}), total {}",
                            symbol, actual_tf, chunk_count, first_date, last_date, all_bars.len() + chunk_count
                        );
                    }
                }

                // Advance start past the last bar — use period-appropriate jump
                if let Some(last_bar) = chunk_bars.last() {
                    if let Ok(last_time) = chrono::DateTime::parse_from_rfc3339(&last_bar.timestamp) {
                        // Jump by at least one period to avoid re-fetching the same bar
                        let jump = match actual_tf {
                            "1Min" => chrono::Duration::minutes(1),
                            "5Min" => chrono::Duration::minutes(5),
                            "15Min" => chrono::Duration::minutes(15),
                            "30Min" => chrono::Duration::minutes(30),
                            "1Hour" => chrono::Duration::hours(1),
                            "4Hour" => chrono::Duration::hours(4),
                            "1Day" => chrono::Duration::days(1),
                            "1Week" => chrono::Duration::weeks(1),
                            _ => chrono::Duration::days(1),
                        };
                        chunk_start = last_time.with_timezone(&chrono::Utc) + jump;
                    } else {
                        chunk_start = chunk_start + chrono::Duration::days(30);
                    }
                }

                all_bars.extend(chunk_bars);

                // Stop if we have enough bars or reached the present
                if all_bars.len() as u32 >= actual_limit {
                    break;
                }
                if chunk_start >= chrono::Utc::now() {
                    break;
                }

                // Rate limiting handled by self.rate_limiter.wait() at top of loop
            }

            if !all_bars.is_empty() {
                // Sort by timestamp (chunks may overlap) then deduplicate
                all_bars.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
                all_bars.dedup_by(|a, b| a.timestamp == b.timestamp);
                // Trim to requested limit (keep most recent)
                if all_bars.len() > actual_limit as usize {
                    let skip = all_bars.len() - actual_limit as usize;
                    all_bars.drain(..skip);
                }
                let feed_label = match feed {
                    Some(f) => *f,
                    None => "crypto",
                };
                tracing::info!(
                    "Loaded {} bars for {} @ {} (feed={}, {} chunks)",
                    all_bars.len(), symbol, actual_tf, feed_label,
                    (all_bars.len() as f64 / 260.0).ceil() as u32
                );
                return Ok(all_bars);
            }
        }

        Err(format!("No bar data for {symbol} @ {timeframe}: {last_error}"))
    }

    /// Aggregate weekly bars into synthetic monthly bars.
    /// Groups by calendar month (year-month), combines OHLCV.
    pub fn aggregate_weekly_to_monthly(weekly: &[Bar]) -> Vec<Bar> {
        if weekly.is_empty() { return vec![]; }
        let mut monthly: Vec<Bar> = Vec::new();
        let mut cur_month = String::new();
        let mut open = 0.0;
        let mut high = f64::MIN;
        let mut low = f64::MAX;
        let mut close = 0.0;
        let mut volume = 0.0;
        let mut month_start = String::new();

        for bar in weekly {
            // Extract YYYY-MM from timestamp
            let ym = if bar.timestamp.len() >= 7 { &bar.timestamp[..7] } else { "" };
            if ym != cur_month {
                if !cur_month.is_empty() {
                    monthly.push(Bar {
                        timestamp: month_start.clone(),
                        open, high, low, close, volume,
                    });
                }
                cur_month = ym.to_string();
                month_start = bar.timestamp.clone();
                open = bar.open;
                high = bar.high;
                low = bar.low;
                close = bar.close;
                volume = bar.volume;
            } else {
                high = high.max(bar.high);
                low = low.min(bar.low);
                close = bar.close;
                volume += bar.volume;
            }
        }
        if !cur_month.is_empty() {
            monthly.push(Bar {
                timestamp: month_start, open, high, low, close, volume,
            });
        }
        monthly
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

    // ── Options Chain ───────────────────────────────────────────────

    /// Fetch options chain from Alpaca options API.
    pub async fn get_options_chain(
        &self,
        underlying_symbol: &str,
        expiry: &str,
    ) -> Result<Vec<OptionContract>, String> {
        self.rate_limiter.wait().await;

        let resp = self
            .client
            .get(format!("{}/v1beta1/options/snapshots/{}", DATA_BASE, underlying_symbol))
            .headers(self.headers())
            .query(&[
                ("feed", "indicative"),
                ("expiration_date", expiry),
            ])
            .send()
            .await
            .map_err(|e| format!("Options request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let _ = resp.text().await;
            return Err(format!("Options request failed: HTTP {status}"));
        }

        let json: serde_json::Value = resp.json().await
            .map_err(|e| format!("Options parse failed: {e}"))?;

        let mut contracts = Vec::new();

        if let Some(snapshots) = json["snapshots"].as_object() {
            for (symbol, snap) in snapshots {
                let latest_quote = &snap["latestQuote"];
                let greeks = &snap["greeks"];

                // Parse option symbol to extract strike, type, expiry
                // Alpaca option symbols: AAPL240119C00150000 (symbol + YYMMDD + C/P + strike*1000)
                let (strike, option_type, parsed_expiry) = Self::parse_option_symbol(symbol);

                contracts.push(OptionContract {
                    symbol: symbol.clone(),
                    underlying: underlying_symbol.to_string(),
                    strike,
                    expiry: parsed_expiry,
                    option_type,
                    bid: latest_quote["bp"].as_f64().unwrap_or(0.0),
                    ask: latest_quote["ap"].as_f64().unwrap_or(0.0),
                    last_price: snap["latestTrade"]["p"].as_f64().unwrap_or(0.0),
                    volume: snap["dailyBar"]["v"].as_f64().unwrap_or(0.0) as u64,
                    open_interest: snap["openInterest"].as_u64().unwrap_or(0),
                    implied_volatility: greeks["impliedVolatility"].as_f64().unwrap_or(0.0),
                    delta: greeks["delta"].as_f64().unwrap_or(0.0),
                    gamma: greeks["gamma"].as_f64().unwrap_or(0.0),
                    theta: greeks["theta"].as_f64().unwrap_or(0.0),
                    vega: greeks["vega"].as_f64().unwrap_or(0.0),
                    rho: greeks["rho"].as_f64().unwrap_or(0.0),
                });
            }
        }

        // Sort by strike price
        contracts.sort_by(|a, b| a.strike.partial_cmp(&b.strike).unwrap_or(std::cmp::Ordering::Equal));
        Ok(contracts)
    }

    /// Parse an OCC option symbol like "AAPL240119C00150000" into (strike, type, expiry).
    fn parse_option_symbol(sym: &str) -> (f64, String, String) {
        // OCC format: underlying (variable) + YYMMDD + C/P + strike*1000 (8 digits)
        let len = sym.len();
        if len < 15 {
            return (0.0, "unknown".to_string(), String::new());
        }
        // Last 8 digits are strike * 1000
        let strike_str = &sym[len - 8..];
        let strike = strike_str.parse::<f64>().unwrap_or(0.0) / 1000.0;
        // C or P is at len - 9
        let option_type = match sym.chars().nth(len - 9) {
            Some('C') => "call".to_string(),
            Some('P') => "put".to_string(),
            _ => "unknown".to_string(),
        };
        // YYMMDD is at len-15..len-9
        let date_str = &sym[len - 15..len - 9];
        let expiry = if date_str.len() == 6 {
            format!("20{}-{}-{}", &date_str[0..2], &date_str[2..4], &date_str[4..6])
        } else {
            String::new()
        };
        (strike, option_type, expiry)
    }

    // ── Financial Analysis (extended SEC EDGAR) ─────────────────────

    /// Fetch comprehensive financial analysis from SEC EDGAR companyfacts.
    /// Returns income statement, balance sheet, and cash flow data.
    /// Uses cached ticker map and shared HTTP client.
    pub async fn get_financial_analysis(ticker: &str) -> Result<serde_json::Value, String> {
        if ticker.is_empty() || ticker.len() > 10 || !ticker.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err("Invalid ticker for SEC lookup".to_string());
        }
        let client = sec_client();
        let cik = lookup_cik(ticker).await?;
        let cik_padded = format!("CIK{:010}", cik);

        let facts_resp = client
            .get(format!("https://data.sec.gov/api/xbrl/companyfacts/{}.json", cik_padded))
            .header("User-Agent", "TyphooN-Terminal/0.1 (support@marketwizardry.org)")
            .send()
            .await
            .map_err(|e| format!("SEC company facts failed: {e}"))?;

        if !facts_resp.status().is_success() {
            return Err(format!("SEC company facts: HTTP {}", facts_resp.status()));
        }

        let facts: serde_json::Value = facts_resp.json().await
            .map_err(|e| format!("SEC facts parse failed: {e}"))?;

        let us_gaap = &facts["facts"]["us-gaap"];

        let result = serde_json::json!({
            "cik": cik,
            "entity": facts["entityName"],
            // Income Statement
            "income_statement": {
                "revenue": extract_latest_fact(us_gaap, "Revenues"),
                "revenue_alt": extract_latest_fact(us_gaap, "RevenueFromContractWithCustomerExcludingAssessedTax"),
                "cost_of_goods_sold": extract_latest_fact(us_gaap, "CostOfGoodsSold"),
                "cogs_alt": extract_latest_fact(us_gaap, "CostOfGoodsAndServicesSold"),
                "gross_profit": extract_latest_fact(us_gaap, "GrossProfit"),
                "operating_income": extract_latest_fact(us_gaap, "OperatingIncomeLoss"),
                "net_income": extract_latest_fact(us_gaap, "NetIncomeLoss"),
                "ebitda": extract_latest_fact(us_gaap, "EarningsBeforeInterestTaxesDepreciationAndAmortization"),
                "eps_basic": extract_latest_fact(us_gaap, "EarningsPerShareBasic"),
                "eps_diluted": extract_latest_fact(us_gaap, "EarningsPerShareDiluted"),
                "research_and_development": extract_latest_fact(us_gaap, "ResearchAndDevelopmentExpense"),
                "sga_expense": extract_latest_fact(us_gaap, "SellingGeneralAndAdministrativeExpense"),
            },
            // Balance Sheet
            "balance_sheet": {
                "total_assets": extract_latest_fact(us_gaap, "Assets"),
                "total_liabilities": extract_latest_fact(us_gaap, "Liabilities"),
                "stockholders_equity": extract_latest_fact(us_gaap, "StockholdersEquity"),
                "cash": extract_latest_fact(us_gaap, "CashAndCashEquivalentsAtCarryingValue"),
                "short_term_investments": extract_latest_fact(us_gaap, "ShortTermInvestments"),
                "accounts_receivable": extract_latest_fact(us_gaap, "AccountsReceivableNetCurrent"),
                "inventory": extract_latest_fact(us_gaap, "InventoryNet"),
                "current_assets": extract_latest_fact(us_gaap, "AssetsCurrent"),
                "current_liabilities": extract_latest_fact(us_gaap, "LiabilitiesCurrent"),
                "long_term_debt": extract_latest_fact(us_gaap, "LongTermDebt"),
                "long_term_debt_alt": extract_latest_fact(us_gaap, "LongTermDebtNoncurrent"),
                "total_debt": extract_latest_fact(us_gaap, "DebtCurrent"),
                "shares_outstanding": extract_latest_fact(us_gaap, "CommonStockSharesOutstanding"),
                "retained_earnings": extract_latest_fact(us_gaap, "RetainedEarningsAccumulatedDeficit"),
                "goodwill": extract_latest_fact(us_gaap, "Goodwill"),
                "intangible_assets": extract_latest_fact(us_gaap, "IntangibleAssetsNetExcludingGoodwill"),
                "property_plant_equipment": extract_latest_fact(us_gaap, "PropertyPlantAndEquipmentNet"),
            },
            // Cash Flow
            "cash_flow": {
                "operating_cash_flow": extract_latest_fact(us_gaap, "NetCashProvidedByOperatingActivities"),
                "investing_cash_flow": extract_latest_fact(us_gaap, "NetCashProvidedByInvestingActivities"),
                "financing_cash_flow": extract_latest_fact(us_gaap, "NetCashProvidedByFinancingActivities"),
                "capex": extract_latest_fact(us_gaap, "PaymentsToAcquirePropertyPlantAndEquipment"),
                "depreciation": extract_latest_fact(us_gaap, "DepreciationDepletionAndAmortization"),
                "stock_based_compensation": extract_latest_fact(us_gaap, "ShareBasedCompensation"),
                "dividends_paid": extract_latest_fact(us_gaap, "PaymentsOfDividends"),
                "share_repurchases": extract_latest_fact(us_gaap, "PaymentsForRepurchaseOfCommonStock"),
                "free_cash_flow_note": "Calculate as operating_cash_flow - capex",
            },
        });

        Ok(result)
    }

    // ── Institutional Holders (13F filings) ───────────────────────────

    /// Fetch institutional holder info from SEC EDGAR submissions.
    /// Looks for 13F filings in the company's filing history.
    /// Uses cached ticker map and shared HTTP client.
    pub async fn get_institutional_holders(ticker: &str) -> Result<serde_json::Value, String> {
        if ticker.is_empty() || ticker.len() > 10 || !ticker.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err("Invalid ticker for SEC lookup".to_string());
        }
        let client = sec_client();
        let cik = lookup_cik(ticker).await?;
        let cik_padded = format!("{:010}", cik);

        let subs_resp = client
            .get(format!("https://data.sec.gov/submissions/CIK{}.json", cik_padded))
            .header("User-Agent", "TyphooN-Terminal/0.1 (support@marketwizardry.org)")
            .send()
            .await
            .map_err(|e| format!("SEC submissions request failed: {e}"))?;

        if !subs_resp.status().is_success() {
            return Err(format!("SEC submissions: HTTP {}", subs_resp.status()));
        }

        let subs: serde_json::Value = subs_resp.json().await
            .map_err(|e| format!("SEC submissions parse failed: {e}"))?;

        // Extract company info
        let entity_name = subs["name"].as_str().unwrap_or("");
        let sic = subs["sic"].as_str().unwrap_or("");
        let sic_description = subs["sicDescription"].as_str().unwrap_or("");
        let state = subs["stateOfIncorporation"].as_str().unwrap_or("");
        let fiscal_year_end = subs["fiscalYearEnd"].as_str().unwrap_or("");

        // Search recent filings for 13F entries
        let recent = &subs["filings"]["recent"];
        let forms = recent["form"].as_array();
        let dates = recent["filingDate"].as_array();
        let accessions = recent["accessionNumber"].as_array();
        let primary_docs = recent["primaryDocument"].as_array();

        let mut filings_13f = Vec::new();
        if let (Some(forms), Some(dates), Some(accessions), Some(primary_docs)) =
            (forms, dates, accessions, primary_docs)
        {
            for i in 0..forms.len().min(200) {
                let form = forms.get(i).and_then(|v| v.as_str()).unwrap_or("");
                if form.starts_with("13F") {
                    let date = dates.get(i).and_then(|v| v.as_str()).unwrap_or("");
                    let accession = accessions.get(i).and_then(|v| v.as_str()).unwrap_or("");
                    let doc = primary_docs.get(i).and_then(|v| v.as_str()).unwrap_or("");
                    filings_13f.push(serde_json::json!({
                        "form": form,
                        "filing_date": date,
                        "accession_number": accession,
                        "primary_document": doc,
                        "url": format!(
                            "https://www.sec.gov/Archives/edgar/data/{}/{}",
                            cik,
                            accession.replace('-', "")
                        ),
                    }));
                    if filings_13f.len() >= 20 { break; }
                }
            }
        }

        let result = serde_json::json!({
            "cik": cik,
            "entity_name": entity_name,
            "sic": sic,
            "sic_description": sic_description,
            "state_of_incorporation": state,
            "fiscal_year_end": fiscal_year_end,
            "filings_13f": filings_13f,
            "total_13f_found": filings_13f.len(),
        });

        Ok(result)
    }

    // ── Most Active / Top Movers (Alpaca Screener API) ────────────────

    /// Fetch most active stocks by volume/trade count from Alpaca screener API.
    pub async fn get_most_active(&self, top: u32) -> Result<serde_json::Value, String> {
        self.rate_limiter.wait().await;

        let resp = self
            .client
            .get(format!("{}/v1beta1/screener/stocks/most-actives", DATA_BASE))
            .headers(self.headers())
            .query(&[("top", top.to_string())])
            .send()
            .await
            .map_err(|e| format!("Most actives request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let _ = resp.text().await;
            return Err(format!("Most actives request failed: HTTP {status}"));
        }

        let json: serde_json::Value = resp.json().await
            .map_err(|e| format!("Most actives parse failed: {e}"))?;

        Ok(json)
    }

    /// Fetch top movers (gainers/losers) from Alpaca screener API.
    /// market_type: "stocks" or "crypto"
    pub async fn get_top_movers(&self, market_type: &str, top: u32) -> Result<serde_json::Value, String> {
        if !matches!(market_type, "stocks" | "crypto") {
            return Err("market_type must be 'stocks' or 'crypto'".to_string());
        }
        self.rate_limiter.wait().await;

        let resp = self
            .client
            .get(format!("{}/v1beta1/screener/{}/movers", DATA_BASE, market_type))
            .headers(self.headers())
            .query(&[("top", top.to_string())])
            .send()
            .await
            .map_err(|e| format!("Top movers request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let _ = resp.text().await;
            return Err(format!("Top movers request failed: HTTP {status}"));
        }

        let json: serde_json::Value = resp.json().await
            .map_err(|e| format!("Top movers parse failed: {e}"))?;

        Ok(json)
    }

    // ── DOM / Level 2 (Crypto Orderbook) ──────────────────────────────

    /// Fetch crypto orderbook snapshot from Alpaca.
    pub async fn get_orderbook(&self, symbol: &str) -> Result<serde_json::Value, String> {
        self.rate_limiter.wait().await;

        let resp = self
            .client
            .get(format!("{}/v1beta1/crypto/us/orderbooks/snapshots", DATA_BASE))
            .headers(self.headers())
            .query(&[("symbols", symbol)])
            .send()
            .await
            .map_err(|e| format!("Orderbook request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let _ = resp.text().await;
            return Err(format!("Orderbook request failed: HTTP {status}"));
        }

        let json: serde_json::Value = resp.json().await
            .map_err(|e| format!("Orderbook parse failed: {e}"))?;

        // Extract the orderbook for the requested symbol and structure it
        let orderbook = &json["orderbooks"][symbol];
        if orderbook.is_null() {
            return Err(format!("No orderbook data for {symbol}"));
        }

        let parse_level = |entry: &serde_json::Value| -> serde_json::Value {
            serde_json::json!({
                "price": entry["p"].as_f64().unwrap_or(0.0),
                "size": entry["s"].as_f64().unwrap_or(0.0),
            })
        };

        let bids: Vec<serde_json::Value> = orderbook["b"]
            .as_array()
            .map(|arr| arr.iter().map(parse_level).collect())
            .unwrap_or_default();

        let asks: Vec<serde_json::Value> = orderbook["a"]
            .as_array()
            .map(|arr| arr.iter().map(parse_level).collect())
            .unwrap_or_default();

        Ok(serde_json::json!({
            "symbol": symbol,
            "timestamp": orderbook["t"],
            "bids": bids,
            "asks": asks,
        }))
    }

    // ── Latest Quote ────────────────────────────────────────────────

    /// Fetch the latest bid/ask quote for a symbol.
    /// For stocks/ETFs: uses snapshot endpoint which includes pre/post-market data.
    /// For crypto: uses latest quotes endpoint (24/7).
    pub async fn get_latest_quote(&self, symbol: &str) -> Result<LatestQuote, String> {
        self.rate_limiter.wait().await;
        let is_crypto = symbol.contains('/');

        if is_crypto {
            let url = format!("{}/v1beta3/crypto/us/latest/quotes", DATA_BASE);
            let resp = self.client.get(&url)
                .headers(self.headers())
                .query(&[("symbols", symbol)])
                .send().await
                .map_err(|e| format!("Quote request failed: {e}"))?;
            if !resp.status().is_success() {
                return Err(format!("Quote request failed: HTTP {}", resp.status()));
            }
            let json: serde_json::Value = resp.json().await
                .map_err(|e| format!("Quote parse failed: {e}"))?;
            let q = json["quotes"][symbol].clone();
            let bid = q["bp"].as_f64().unwrap_or(0.0);
            let ask = q["ap"].as_f64().unwrap_or(0.0);
            Ok(LatestQuote {
                symbol: symbol.to_string(), bid, ask,
                bid_size: q["bs"].as_f64().unwrap_or(0.0),
                ask_size: q["as"].as_f64().unwrap_or(0.0),
                spread: ask - bid,
                timestamp: q["t"].as_str().unwrap_or("").to_string(),
            })
        } else {
            // Stocks/ETFs: use snapshot endpoint for pre/post-market data
            // Snapshot returns: { latestTrade, latestQuote, minuteBar, dailyBar, prevDailyBar }
            let url = format!("{}/v2/stocks/{}/snapshot", DATA_BASE, symbol);
            let resp = self.client.get(&url)
                .headers(self.headers())
                .query(&[("feed", "iex")])
                .send().await
                .map_err(|e| format!("Snapshot request failed: {e}"))?;
            if !resp.status().is_success() {
                return Err(format!("Snapshot request failed: HTTP {}", resp.status()));
            }
            let json: serde_json::Value = resp.json().await
                .map_err(|e| format!("Snapshot parse failed: {e}"))?;

            // Latest quote (may be regular hours only on IEX)
            let q = &json["latestQuote"];
            let bid = q["bp"].as_f64().unwrap_or(0.0);
            let ask = q["ap"].as_f64().unwrap_or(0.0);

            // Latest trade includes pre/post-market on IEX
            let t = &json["latestTrade"];
            let trade_price = t["p"].as_f64().unwrap_or(0.0);
            let trade_ts = t["t"].as_str().unwrap_or("");

            // Use trade price as mid if quote is stale (outside market hours)
            let (final_bid, final_ask) = if bid > 0.0 && ask > 0.0 {
                (bid, ask)
            } else if trade_price > 0.0 {
                // No live quote (pre/post market) — use last trade as both bid and ask
                (trade_price, trade_price)
            } else {
                (0.0, 0.0)
            };

            Ok(LatestQuote {
                symbol: symbol.to_string(),
                bid: final_bid,
                ask: final_ask,
                bid_size: q["bs"].as_f64().unwrap_or(0.0),
                ask_size: q["as"].as_f64().unwrap_or(0.0),
                spread: final_ask - final_bid,
                timestamp: if trade_ts.is_empty() {
                    q["t"].as_str().unwrap_or("").to_string()
                } else {
                    trade_ts.to_string()
                },
            })
        }
    }

    // ── Account Activities ───────────────────────────────────────────

    /// Fetch account activities (fills, dividends, deposits, etc.)
    pub async fn get_account_activities(&self, activity_types: &str, limit: u32) -> Result<Vec<AccountActivity>, String> {
        // Validate activity_types: alphanumeric + comma only (prevent path traversal)
        if !activity_types.is_empty() && !activity_types.chars().all(|c| c.is_alphanumeric() || c == ',' || c == '_') {
            return Err("Invalid activity type characters".to_string());
        }
        let url = if activity_types.is_empty() {
            format!("{}/v2/account/activities", self.base_url)
        } else {
            format!("{}/v2/account/activities/{}", self.base_url, activity_types)
        };

        let resp = self.client
            .get(&url)
            .headers(self.headers())
            .query(&[("direction", "desc"), ("page_size", &limit.to_string())])
            .send()
            .await
            .map_err(|e| format!("Activities request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let _ = resp.text().await;
            return Err(format!("Activities request failed: HTTP {}", status));
        }

        let json: Vec<serde_json::Value> = resp.json().await
            .map_err(|e| format!("Activities parse failed: {e}"))?;

        Ok(json.iter().map(|a| {
            let activity_type = a["activity_type"].as_str().unwrap_or("").to_string();
            let description = match activity_type.as_str() {
                "FILL" => format!("{} {} {} @ {}",
                    a["side"].as_str().unwrap_or(""),
                    a["qty"].as_str().unwrap_or("0"),
                    a["symbol"].as_str().unwrap_or(""),
                    a["price"].as_str().unwrap_or("?")),
                "DIV" | "DIVCGL" | "DIVCGS" | "DIVNRA" | "DIVROC" | "DIVTXEX" =>
                    format!("Dividend {} ${}", a["symbol"].as_str().unwrap_or(""), a["net_amount"].as_str().unwrap_or("0")),
                "CSD" => format!("Deposit ${}", a["net_amount"].as_str().unwrap_or("0")),
                "CSW" => format!("Withdrawal ${}", a["net_amount"].as_str().unwrap_or("0")),
                _ => format!("{} {}", activity_type, a["symbol"].as_str().unwrap_or("")),
            };
            AccountActivity {
                id: a["id"].as_str().unwrap_or("").to_string(),
                activity_type,
                symbol: a["symbol"].as_str().map(|s| s.to_string()),
                side: a["side"].as_str().map(|s| s.to_string()),
                qty: a["qty"].as_str().map(|s| s.to_string()),
                price: a["price"].as_str().map(|s| s.to_string()),
                net_amount: a["net_amount"].as_str().map(|s| s.to_string()),
                date: a["transaction_time"].as_str()
                    .or_else(|| a["date"].as_str())
                    .unwrap_or("").to_string(),
                description,
            }
        }).collect())
    }

    // ── Insider Trading (SEC Form 4) ─────────────────────────────────

    /// Fetch insider trades for a ticker via SEC EDGAR (Form 4 filings).
    /// Uses cached ticker map and shared HTTP client.
    pub async fn get_insider_trades(ticker: &str) -> Result<Vec<InsiderTrade>, String> {
        if ticker.is_empty() || ticker.len() > 10 || !ticker.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err("Invalid ticker for SEC lookup".to_string());
        }
        let client = sec_client();
        let cik = lookup_cik(ticker).await?;
        let cik_padded = format!("{:010}", cik);

        let subs_resp = client
            .get(format!("https://data.sec.gov/submissions/CIK{}.json", cik_padded))
            .header("User-Agent", "TyphooN-Terminal/0.1 (support@marketwizardry.org)")
            .send()
            .await
            .map_err(|e| format!("SEC submissions fetch failed: {e}"))?;

        if !subs_resp.status().is_success() {
            return Err(format!("SEC submissions: HTTP {}", subs_resp.status()));
        }

        let subs: serde_json::Value = subs_resp.json().await
            .map_err(|e| format!("SEC submissions parse failed: {e}"))?;

        // Step 3: Filter for Form 4 filings from recent filings
        let recent = &subs["filings"]["recent"];
        let forms = recent["form"].as_array();
        let dates = recent["filingDate"].as_array();
        let accessions = recent["accessionNumber"].as_array();
        let primary_docs = recent["primaryDocument"].as_array();

        let mut insider_trades = Vec::new();

        if let (Some(forms), Some(dates), Some(accessions), Some(_docs)) =
            (forms, dates, accessions, primary_docs)
        {
            for i in 0..forms.len().min(200) {
                let form = forms[i].as_str().unwrap_or("");
                if form != "4" { continue; }
                if insider_trades.len() >= 50 { break; }

                let filing_date = dates[i].as_str().unwrap_or("").to_string();
                let accession = accessions[i].as_str().unwrap_or("").to_string();

                // Parse owner name from the filing index
                // For efficiency, we extract basic info from the submissions JSON
                let owner_name = subs["filings"]["recent"]["reportOwner"]
                    .as_array()
                    .and_then(|arr| arr.get(i))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown")
                    .to_string();

                let form_url = format!(
                    "https://www.sec.gov/Archives/edgar/data/{}/{}",
                    cik,
                    accession.replace('-', "")
                );

                insider_trades.push(InsiderTrade {
                    filing_date: filing_date.clone(),
                    report_date: filing_date,
                    owner_name,
                    owner_title: String::new(),
                    transaction_type: "Form 4".to_string(),
                    shares: 0.0,
                    price_per_share: 0.0,
                    total_value: 0.0,
                    shares_owned_after: 0.0,
                    form_url,
                });
            }
        }

        Ok(insider_trades)
    }

    // ── WebSocket Streaming ─────────────────────────────────────────

    /// Start a WebSocket connection to Alpaca's real-time data stream.
    /// Returns a receiver for incoming StreamMessage events.
    /// Subscribes to the given trade and quote symbols.
    pub async fn start_stream(
        &self,
        trade_symbols: Vec<String>,
        quote_symbols: Vec<String>,
    ) -> Result<tokio::sync::mpsc::Receiver<StreamMessage>, String> {
        let is_crypto = trade_symbols.iter().chain(quote_symbols.iter()).any(|s| s.contains('/'));
        let ws_url = if is_crypto {
            "wss://stream.data.alpaca.markets/v1beta3/crypto/us"
        } else {
            "wss://stream.data.alpaca.markets/v2/iex"
        };

        let (ws_stream, _) = tokio_tungstenite::connect_async(ws_url)
            .await
            .map_err(|e| format!("WebSocket connect failed: {e}"))?;

        let (mut write, mut read) = ws_stream.split();

        // Authenticate
        let auth_msg = serde_json::json!({
            "action": "auth",
            "key": self.api_key.as_str(),
            "secret": self.secret_key.as_str(),
        });
        write
            .send(tokio_tungstenite::tungstenite::Message::Text(auth_msg.to_string().into()))
            .await
            .map_err(|e| format!("WebSocket auth send failed: {e}"))?;

        // Wait for auth response
        if let Some(Ok(msg)) = read.next().await {
            // First message is connection welcome
            tracing::debug!("WS welcome: {msg}");
        }
        if let Some(Ok(msg)) = read.next().await {
            let text = msg.to_text().unwrap_or("");
            if text.contains("\"error\"") {
                return Err(format!("WebSocket auth failed: {text}"));
            }
            tracing::debug!("WS auth response: {text}");
        }

        // Subscribe
        let sub_msg = serde_json::json!({
            "action": "subscribe",
            "trades": trade_symbols,
            "quotes": quote_symbols,
        });
        write
            .send(tokio_tungstenite::tungstenite::Message::Text(sub_msg.to_string().into()))
            .await
            .map_err(|e| format!("WebSocket subscribe failed: {e}"))?;

        // Channel for outgoing messages
        let (tx, rx) = tokio::sync::mpsc::channel::<StreamMessage>(1024);

        // Spawn reader task
        tokio::spawn(async move {
            while let Some(Ok(msg)) = read.next().await {
                let text = match msg.to_text() {
                    Ok(t) => t.to_string(),
                    Err(_) => continue,
                };

                let parsed: Result<Vec<serde_json::Value>, _> = serde_json::from_str(&text);
                if let Ok(events) = parsed {
                    for event in events {
                        let msg_type = event["T"].as_str().unwrap_or("");
                        let stream_msg = match msg_type {
                            "t" => Some(StreamMessage::Trade(StreamTrade {
                                symbol: event["S"].as_str().unwrap_or("").to_string(),
                                price: event["p"].as_f64().unwrap_or(0.0),
                                size: event["s"].as_f64().unwrap_or(0.0),
                                timestamp: event["t"].as_str().unwrap_or("").to_string(),
                            })),
                            "q" => Some(StreamMessage::Quote(StreamQuote {
                                symbol: event["S"].as_str().unwrap_or("").to_string(),
                                bid: event["bp"].as_f64().unwrap_or(0.0),
                                ask: event["ap"].as_f64().unwrap_or(0.0),
                                bid_size: event["bs"].as_f64().unwrap_or(0.0),
                                ask_size: event["as"].as_f64().unwrap_or(0.0),
                                timestamp: event["t"].as_str().unwrap_or("").to_string(),
                            })),
                            _ => None,
                        };
                        if let Some(sm) = stream_msg {
                            if tx.send(sm).await.is_err() {
                                tracing::info!("Stream receiver dropped, closing WS");
                                return;
                            }
                        }
                    }
                }
            }
            tracing::info!("WebSocket stream ended");
        });

        Ok(rx)
    }
}

// ── Streaming Types ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamMessage {
    Trade(StreamTrade),
    Quote(StreamQuote),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamTrade {
    pub symbol: String,
    pub price: f64,
    pub size: f64,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamQuote {
    pub symbol: String,
    pub bid: f64,
    pub ask: f64,
    pub bid_size: f64,
    pub ask_size: f64,
    pub timestamp: String,
}

// ── Quote / Activity / Insider Types ────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatestQuote {
    pub symbol: String,
    pub bid: f64,
    pub ask: f64,
    pub bid_size: f64,
    pub ask_size: f64,
    pub spread: f64,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountActivity {
    pub id: String,
    pub activity_type: String,
    pub symbol: Option<String>,
    pub side: Option<String>,
    pub qty: Option<String>,
    pub price: Option<String>,
    pub net_amount: Option<String>,
    pub date: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsiderTrade {
    pub filing_date: String,
    pub report_date: String,
    pub owner_name: String,
    pub owner_title: String,
    pub transaction_type: String,
    pub shares: f64,
    pub price_per_share: f64,
    pub total_value: f64,
    pub shares_owned_after: f64,
    pub form_url: String,
}

// ── Options Types ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionContract {
    pub symbol: String,
    pub underlying: String,
    pub strike: f64,
    pub expiry: String,
    pub option_type: String, // "call" or "put"
    pub bid: f64,
    pub ask: f64,
    pub last_price: f64,
    pub volume: u64,
    pub open_interest: u64,
    pub implied_volatility: f64,
    pub delta: f64,
    pub gamma: f64,
    pub theta: f64,
    pub vega: f64,
    pub rho: f64,
}
