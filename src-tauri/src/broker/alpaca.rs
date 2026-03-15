//! Alpaca broker interface.
//!
//! Wraps Alpaca REST API and WebSocket streaming.
//! Provides the same operations as MQL5 CTrade: open, close, partial close, modify, account info.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

const PAPER_BASE: &str = "https://paper-api.alpaca.markets";
const LIVE_BASE: &str = "https://api.alpaca.markets";
const DATA_BASE: &str = "https://data.alpaca.markets";

/// Alpaca free plan: 200 requests/minute = 1 request per 300ms.
/// We use 320ms to leave headroom.
const RATE_LIMIT_MS: u64 = 320;

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
    api_key: String,
    secret_key: String,
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
            rate_limiter: RateLimiter::new(),
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
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("News failed ({}): {}", status, body));
        }

        let json: serde_json::Value = resp.json().await
            .map_err(|e| format!("News parse failed: {e}"))?;

        Ok(json["news"].as_array().cloned().unwrap_or_default())
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
    pub async fn get_sec_filings(ticker: &str, filing_type: &str, _limit: u32) -> Result<serde_json::Value, String> {
        let client = Client::new();

        // Step 1: Look up CIK from ticker
        let cik_resp = client
            .get("https://efts.sec.gov/LATEST/search-index?q=%22".to_owned() + ticker + "%22&dateRange=custom&startdt=2020-01-01&forms=" + filing_type)
            .header("User-Agent", "TyphooN-Terminal/0.1 (contact@marketwizardry.org)")
            .send()
            .await
            .map_err(|e| format!("SEC EDGAR request failed: {e}"))?;

        if !cik_resp.status().is_success() {
            // Fallback: use full-text search
            let search_resp = client
                .get(format!(
                    "https://efts.sec.gov/LATEST/search-index?q={}&forms={}&dateRange=custom&startdt=2020-01-01",
                    ticker, filing_type
                ))
                .header("User-Agent", "TyphooN-Terminal/0.1 (contact@marketwizardry.org)")
                .send()
                .await
                .map_err(|e| format!("SEC EDGAR search failed: {e}"))?;

            let json: serde_json::Value = search_resp.json().await
                .map_err(|e| format!("SEC EDGAR parse failed: {e}"))?;
            return Ok(json);
        }

        let json: serde_json::Value = cik_resp.json().await
            .map_err(|e| format!("SEC EDGAR parse failed: {e}"))?;
        Ok(json)
    }

    /// Fetch company facts from SEC EDGAR (financials, shares outstanding, etc.)
    pub async fn get_sec_company_facts(ticker: &str) -> Result<serde_json::Value, String> {
        let client = Client::new();

        // First resolve ticker to CIK via SEC ticker map
        let tickers_resp = client
            .get("https://www.sec.gov/files/company_tickers.json")
            .header("User-Agent", "TyphooN-Terminal/0.1 (contact@marketwizardry.org)")
            .send()
            .await
            .map_err(|e| format!("SEC ticker map failed: {e}"))?;

        let tickers: serde_json::Value = tickers_resp.json().await
            .map_err(|e| format!("SEC ticker map parse failed: {e}"))?;

        // Find CIK for ticker
        let upper_ticker = ticker.to_uppercase();
        let mut cik: Option<u64> = None;
        if let Some(obj) = tickers.as_object() {
            for (_, v) in obj {
                if v["ticker"].as_str() == Some(&upper_ticker) {
                    cik = v["cik_str"].as_u64().or_else(|| v["cik_str"].as_str().and_then(|s| s.parse().ok()));
                    break;
                }
            }
        }

        let cik = cik.ok_or_else(|| format!("CIK not found for {ticker}"))?;
        let cik_padded = format!("CIK{:010}", cik);

        // Fetch company facts
        let facts_resp = client
            .get(format!("https://data.sec.gov/api/xbrl/companyfacts/{}.json", cik_padded))
            .header("User-Agent", "TyphooN-Terminal/0.1 (contact@marketwizardry.org)")
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

        // Go back far enough to cover requested bars
        let max_lookback_days = match actual_tf {
            "1Min" => 7,
            "5Min" | "15Min" | "30Min" => 30,
            "1Hour" => 365,
            "4Hour" => 730,
            "1Day" => 3650,
            "1Week" => 7300,
            _ => 1825,
        };
        let earliest_start = chrono::Utc::now() - chrono::Duration::days(max_lookback_days);

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
                    ("limit", "10000".to_string()), // request max, server returns ~260 on free plan
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
                tracing::info!(
                    "Loaded {} bars for {} @ {} (feed={:?}, {} chunks)",
                    all_bars.len(), symbol, actual_tf, feed,
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
}
