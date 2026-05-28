//! Alpaca broker interface.
//!
//! Wraps Alpaca REST API and WebSocket streaming.
//! Provides the same operations as MQL5 CTrade: open, close, partial close, modify, account info.

use futures_util::{SinkExt, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use tokio::sync::Mutex;
use zeroize::Zeroizing;

/// Shared HTTP client for SEC EDGAR requests (reuses TCP connections).
fn sec_client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .pool_max_idle_per_host(2)
            .build()
            .unwrap_or_else(|_| Client::new())
    })
}

/// Cached SEC ticker→CIK map. Fetched once (~8MB), reused for all lookups.
static SEC_TICKER_MAP: tokio::sync::OnceCell<serde_json::Value> =
    tokio::sync::OnceCell::const_new();

async fn get_sec_ticker_map() -> Result<&'static serde_json::Value, String> {
    SEC_TICKER_MAP
        .get_or_try_init(|| async {
            let client = sec_client();
            let resp = client
                .get("https://www.sec.gov/files/company_tickers.json")
                .header(
                    "User-Agent",
                    "TyphooN-Terminal/1.0 typhoon-terminal@example.invalid",
                )
                .send()
                .await
                .map_err(|_| "SEC ticker map request failed".to_string())?;
            resp.json::<serde_json::Value>()
                .await
                .map_err(|_| "SEC ticker map parse failed".to_string())
        })
        .await
}

/// Look up CIK number for a ticker symbol from cached SEC ticker map.
async fn lookup_cik(ticker: &str) -> Result<u64, String> {
    let tickers = get_sec_ticker_map().await?;
    let upper_ticker = ticker.to_uppercase();
    if let Some(obj) = tickers.as_object() {
        for (_, v) in obj {
            if v["ticker"].as_str() == Some(&upper_ticker) {
                if let Some(cik) = v["cik_str"]
                    .as_u64()
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

/// Alpaca Basic historical data is currently documented at 200 req/min.
/// We start there unless the caller provides a higher tier hint or the API
/// confirms a different limit via `X-RateLimit-Limit` headers.
const DEFAULT_BAR_REQUESTS_PER_MINUTE: u32 = 200;

/// If a single chunk takes longer than this, the API is throttling us progressively.
/// Accept what we have rather than spending hours on historical data.
const SLOW_CHUNK_THRESHOLD_SECS: u64 = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BarsLookbackMode {
    /// Tight windows for freshness checks and incremental sync.
    Incremental,
    /// Wider windows sized to hit the terminal's target cache depth without
    /// requesting since-inception history for every missing symbol.
    Targeted,
}

fn bars_per_day(is_crypto: bool, timeframe: &str) -> f64 {
    if is_crypto {
        match timeframe {
            "1Min" => 1440.0,
            "5Min" => 288.0,
            "15Min" => 96.0,
            "30Min" => 48.0,
            "1Hour" => 24.0,
            "4Hour" => 6.0,
            "1Day" => 1.0,
            "1Week" => 0.14,
            _ => 1.0,
        }
    } else {
        match timeframe {
            "1Min" => 390.0,
            "5Min" => 78.0,
            "15Min" => 26.0,
            "30Min" => 13.0,
            "1Hour" => 7.0,
            "4Hour" => 2.0,
            "1Day" => 0.7,
            "1Week" => 0.14,
            _ => 0.7,
        }
    }
}

fn max_lookback_days(is_crypto: bool, timeframe: &str, mode: BarsLookbackMode) -> i64 {
    match (is_crypto, mode, timeframe) {
        (true, BarsLookbackMode::Incremental, "1Min") => 3,
        (true, BarsLookbackMode::Incremental, "5Min" | "15Min" | "30Min") => 14,
        (true, BarsLookbackMode::Incremental, "1Hour") => 90,
        (true, BarsLookbackMode::Incremental, "4Hour") => 180,
        (true, BarsLookbackMode::Incremental, "1Day") => 1825,
        (true, BarsLookbackMode::Incremental, "1Week") => 3650,
        (true, BarsLookbackMode::Incremental, _) => 365,
        (false, BarsLookbackMode::Incremental, "1Min") => 7,
        (false, BarsLookbackMode::Incremental, "5Min" | "15Min" | "30Min") => 30,
        (false, BarsLookbackMode::Incremental, "1Hour") => 365,
        (false, BarsLookbackMode::Incremental, "4Hour") => 730,
        (false, BarsLookbackMode::Incremental, "1Day") => 3650,
        (false, BarsLookbackMode::Incremental, "1Week") => 7300,
        (false, BarsLookbackMode::Incremental, _) => 1825,
        (true, BarsLookbackMode::Targeted, "1Min") => 120,
        (true, BarsLookbackMode::Targeted, "5Min") => 730,
        (true, BarsLookbackMode::Targeted, "15Min" | "30Min") => 1825,
        (true, BarsLookbackMode::Targeted, "1Hour" | "4Hour" | "1Day") => 3650,
        (true, BarsLookbackMode::Targeted, "1Week") => 7300,
        (true, BarsLookbackMode::Targeted, _) => 1825,
        (false, BarsLookbackMode::Targeted, "1Min") => 365,
        (false, BarsLookbackMode::Targeted, "5Min") => 1825,
        (false, BarsLookbackMode::Targeted, "15Min" | "30Min") => 3650,
        (false, BarsLookbackMode::Targeted, "1Hour" | "4Hour" | "1Day" | "1Week") => 7300,
        (false, BarsLookbackMode::Targeted, _) => 3650,
    }
}

fn lookback_days_for_request(
    is_crypto: bool,
    timeframe: &str,
    limit: u32,
    mode: BarsLookbackMode,
) -> i64 {
    let proportional_days =
        ((limit as f64 / bars_per_day(is_crypto, timeframe)) * 1.5).ceil() as i64;
    proportional_days
        .max(7)
        .min(max_lookback_days(is_crypto, timeframe, mode))
}

/// Round price to valid increment for Alpaca orders.
/// Stocks > $1: 2 decimal places (penny). Stocks < $1: 4 decimal places (sub-penny allowed).
/// Crypto: 8 decimal places.
/// Parse a JSON field as f64, handling both string ("123.45") and number (123.45) formats.
/// Alpaca returns some fields as strings, others as numbers depending on endpoint/version.
fn parse_f64_field(json: &serde_json::Value, field: &str) -> f64 {
    // Try as string first (Alpaca's typical format)
    if let Some(s) = json[field].as_str() {
        return s.parse().unwrap_or(0.0);
    }
    // Try as number (some endpoints/versions return raw numbers)
    if let Some(n) = json[field].as_f64() {
        return n;
    }
    // Field is null or missing — not a warning, just absent
    0.0
}

fn format_order_price(price: f64) -> String {
    if price >= 1.0 {
        format!("{:.2}", price) // $1+ → 2 decimals (e.g., 15.68)
    } else if price >= 0.01 {
        format!("{:.4}", price) // $0.01-$0.99 → 4 decimals
    } else {
        format!("{:.8}", price) // sub-penny / crypto → 8 decimals
    }
}

fn rpm_to_interval_ms(rpm: u32) -> u64 {
    let rpm = rpm.max(1) as f64;
    ((60_000.0 / rpm) * 1.05).ceil() as u64
}

/// Centralized rate limiter — shared across all data API requests.
/// On 429, pauses all requests for a cooldown period.
/// Adaptive: backs off when API responses slow down (progressive throttling).
#[derive(Debug, Clone)]
pub struct RateLimiter {
    last_request: Arc<Mutex<std::time::Instant>>,
    cooldown_until: Arc<Mutex<Option<std::time::Instant>>>,
    base_interval_ms: Arc<Mutex<u64>>,
    /// Adaptive pacing: increases when API is slow, resets after cooldown
    adaptive_ms: Arc<Mutex<u64>>,
    requests_per_minute: Arc<std::sync::atomic::AtomicU32>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self::with_requests_per_minute(DEFAULT_BAR_REQUESTS_PER_MINUTE)
    }

    pub fn with_requests_per_minute(rpm: u32) -> Self {
        let rpm = rpm.max(1);
        let interval_ms = rpm_to_interval_ms(rpm);
        Self {
            last_request: Arc::new(Mutex::new(
                std::time::Instant::now() - std::time::Duration::from_secs(1),
            )),
            cooldown_until: Arc::new(Mutex::new(None)),
            base_interval_ms: Arc::new(Mutex::new(interval_ms)),
            adaptive_ms: Arc::new(Mutex::new(interval_ms)),
            requests_per_minute: Arc::new(std::sync::atomic::AtomicU32::new(rpm)),
        }
    }

    /// Wait until we can make another request without hitting rate limit.
    pub async fn wait(&self) -> bool {
        // Check cooldown — read value and drop lock before sleeping
        let cooldown_remaining = {
            let cooldown = self.cooldown_until.lock().await;
            match *cooldown {
                Some(until) if std::time::Instant::now() < until => {
                    Some(until - std::time::Instant::now())
                }
                _ => None,
            }
        };
        if let Some(remaining) = cooldown_remaining {
            tracing::debug!("Rate limiter in cooldown for {}ms", remaining.as_millis());
            tokio::time::sleep(remaining).await;
        }
        // Adaptive pacing
        let interval_ms = { *self.adaptive_ms.lock().await };
        let mut last = self.last_request.lock().await;
        let elapsed = last.elapsed();
        let min_interval = std::time::Duration::from_millis(interval_ms);
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
        // Double adaptive interval on 429 (capped at 5s)
        let base_interval_ms = *self.base_interval_ms.lock().await;
        let mut adaptive = self.adaptive_ms.lock().await;
        *adaptive = (*adaptive * 2).min(5000).max(base_interval_ms);
        tracing::warn!(
            "Rate limit hit — cooling down for 60s (adaptive interval: {}ms)",
            *adaptive
        );
    }

    /// Report how long a request took. If responses are slow, back off.
    pub async fn report_latency(&self, elapsed_ms: u64) {
        let base_interval_ms = *self.base_interval_ms.lock().await;
        let mut adaptive = self.adaptive_ms.lock().await;
        if elapsed_ms > 10_000 {
            // API taking >10s per response — progressive throttling detected
            *adaptive = (*adaptive + 200).min(5000);
        } else if elapsed_ms < 2_000 && *adaptive > base_interval_ms {
            // Fast responses — gradually recover
            *adaptive = adaptive.saturating_sub(50).max(base_interval_ms);
        }
    }

    async fn set_requests_per_minute(&self, rpm: u32) {
        let rpm = rpm.max(1);
        let interval_ms = rpm_to_interval_ms(rpm);
        self.requests_per_minute.store(rpm, Ordering::Relaxed);
        *self.base_interval_ms.lock().await = interval_ms;
        *self.adaptive_ms.lock().await = interval_ms;
    }

    pub async fn apply_requests_per_minute_hint(&self, rpm: u32) {
        self.set_requests_per_minute(rpm).await;
    }

    pub async fn observe_rate_limit_headers(
        &self,
        headers: &reqwest::header::HeaderMap,
    ) -> Option<u32> {
        let Some(limit_header) = headers
            .get("x-ratelimit-limit")
            .or_else(|| headers.get("X-RateLimit-Limit"))
        else {
            return None;
        };
        let Ok(limit_str) = limit_header.to_str() else {
            return None;
        };
        let Ok(rpm) = limit_str.trim().parse::<u32>() else {
            return None;
        };
        if !(60..=100_000).contains(&rpm) {
            return None;
        }
        let prev = self.requests_per_minute.load(Ordering::Relaxed);
        if prev != rpm {
            self.set_requests_per_minute(rpm).await;
            tracing::info!(
                "Alpaca historical bar rate limit observed: {} req/min ({}ms floor)",
                rpm,
                rpm_to_interval_ms(rpm)
            );
            Some(rpm)
        } else {
            None
        }
    }

    pub fn requests_per_minute(&self) -> u32 {
        self.requests_per_minute.load(Ordering::Relaxed)
    }
}

#[derive(Debug, Clone)]
pub struct AlpacaBroker {
    client: Client,
    base_url: String,
    api_key: Zeroizing<String>,
    secret_key: Zeroizing<String>,
    rate_limiter: RateLimiter,
    bar_rate_limiter: RateLimiter,
    sip_bar_feed_unavailable: Arc<AtomicBool>,
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
    pub order_class: Option<String>,
    pub status: String,
    pub limit_price: Option<String>,
    pub stop_price: Option<String>,
    pub trail_price: Option<String>,
    pub trail_percent: Option<String>,
    pub created_at: String,
    pub filled_at: Option<String>,
    pub filled_avg_price: Option<String>,
    /// Bracket order legs (SL/TP child orders). Only present with nested=true.
    pub legs: Option<Vec<OrderInfo>>,
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

/// Outcome of a bar fetch — tells the caller whether to enqueue a retry.
/// `Complete` means the API returned everything it had. `RateLimitedPartial`
/// means we got some bars but paginated pages remain behind a 429 wall.
/// `RateLimitedEmpty` means the first request 429'd before any bars landed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FetchOutcome {
    Complete,
    RateLimitedPartial,
    RateLimitedEmpty,
}

impl AlpacaBroker {
    pub fn new(
        api_key: String,
        secret_key: String,
        paper: bool,
        bar_requests_per_minute: u32,
    ) -> Self {
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
                .unwrap_or_else(|_| Client::new()),
            base_url,
            api_key: Zeroizing::new(api_key),
            secret_key: Zeroizing::new(secret_key),
            rate_limiter: RateLimiter::new(),
            bar_rate_limiter: RateLimiter::with_requests_per_minute(
                bar_requests_per_minute.max(DEFAULT_BAR_REQUESTS_PER_MINUTE),
            ),
            sip_bar_feed_unavailable: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn set_bar_requests_per_minute_hint(&self, rpm: u32) {
        self.bar_rate_limiter
            .apply_requests_per_minute_hint(rpm.max(DEFAULT_BAR_REQUESTS_PER_MINUTE))
            .await;
    }

    pub fn bar_requests_per_minute(&self) -> u32 {
        self.bar_rate_limiter.requests_per_minute()
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

    fn stock_bar_feeds(&self) -> Vec<Option<&'static str>> {
        if self.sip_bar_feed_unavailable.load(Ordering::Relaxed) {
            vec![Some("iex")]
        } else {
            vec![Some("iex"), Some("sip")]
        }
    }

    fn is_sip_bar_entitlement_failure(status: reqwest::StatusCode, body: &str) -> bool {
        if !matches!(status.as_u16(), 401 | 403 | 422) {
            return false;
        }
        let body = body.to_ascii_lowercase();
        body.contains("sip")
            && [
                "subscription",
                "entitle",
                "entitlement",
                "permission",
                "permit",
                "upgrade",
                "plan",
            ]
            .iter()
            .any(|needle| body.contains(needle))
    }

    fn note_sip_bar_entitlement_failure(&self, status: reqwest::StatusCode, body: &str) -> bool {
        if !Self::is_sip_bar_entitlement_failure(status, body) {
            return false;
        }
        if !self.sip_bar_feed_unavailable.swap(true, Ordering::Relaxed) {
            tracing::warn!(
                "Alpaca SIP bar feed unavailable for this session; skipping future SIP probes"
            );
        }
        true
    }

    /// Pre-warm TCP+TLS connection to the data endpoint (data.alpaca.markets).
    /// The main API endpoint (api/paper-api) gets warmed by get_account(),
    /// but bar data goes to a different host. Call this once after connect
    /// to shave ~200ms off the first bar fetch.
    pub async fn warm_data_connection(&self) {
        // HEAD request to data endpoint — establishes connection without fetching data
        let _ = self
            .client
            .head(format!("{}/v2/stocks/AAPL/bars", DATA_BASE))
            .headers(self.headers())
            .send()
            .await;
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
            equity: parse_f64_field(&json, "equity"),
            cash: parse_f64_field(&json, "cash"),
            buying_power: parse_f64_field(&json, "buying_power"),
            portfolio_value: parse_f64_field(&json, "portfolio_value"),
            initial_margin: parse_f64_field(&json, "initial_margin"),
            maintenance_margin: parse_f64_field(&json, "maintenance_margin"),
            currency: json["currency"].as_str().unwrap_or("USD").to_string(),
            pattern_day_trader: json["pattern_day_trader"].as_bool().unwrap_or(false),
            trading_blocked: json["trading_blocked"].as_bool().unwrap_or(false),
            balance: parse_f64_field(&json, "last_equity"),
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

        let status = resp.status();
        if !status.is_success() {
            return Err(format!("Positions request failed: HTTP {status}"));
        }

        let json: Vec<serde_json::Value> = resp
            .json()
            .await
            .map_err(|e| format!("Positions parse failed: {e}"))?;

        Ok(json
            .iter()
            .map(|p| PositionInfo {
                symbol: p["symbol"].as_str().unwrap_or("").to_string(),
                qty: parse_f64_field(p, "qty"),
                side: p["side"].as_str().unwrap_or("").to_string(),
                avg_entry_price: parse_f64_field(p, "avg_entry_price"),
                market_value: parse_f64_field(p, "market_value"),
                unrealized_pl: parse_f64_field(p, "unrealized_pl"),
                asset_class: p["asset_class"].as_str().unwrap_or("").to_string(),
                asset_id: p["asset_id"].as_str().unwrap_or("").to_string(),
            })
            .collect())
    }

    // ── Orders ───────────────────────────────────────────────────────

    pub async fn market_order(
        &self,
        symbol: &str,
        qty: f64,
        side: &str,
    ) -> Result<OrderResult, String> {
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
    pub async fn limit_order(
        &self,
        symbol: &str,
        qty: f64,
        side: &str,
        limit_price: f64,
        tif: &str,
    ) -> Result<OrderResult, String> {
        let body = serde_json::json!({
            "symbol": symbol,
            "qty": qty.to_string(),
            "side": side,
            "type": "limit",
            "limit_price": format_order_price(limit_price),
            "time_in_force": tif,
        });
        self.submit_order(&body).await
    }

    /// Place a stop order.
    pub async fn stop_order(
        &self,
        symbol: &str,
        qty: f64,
        side: &str,
        stop_price: f64,
        tif: &str,
    ) -> Result<OrderResult, String> {
        let body = serde_json::json!({
            "symbol": symbol,
            "qty": qty.to_string(),
            "side": side,
            "type": "stop",
            "stop_price": format_order_price(stop_price),
            "time_in_force": tif,
        });
        self.submit_order(&body).await
    }

    /// Place a stop-limit order.
    pub async fn stop_limit_order(
        &self,
        symbol: &str,
        qty: f64,
        side: &str,
        stop_price: f64,
        limit_price: f64,
        tif: &str,
    ) -> Result<OrderResult, String> {
        let body = serde_json::json!({
            "symbol": symbol,
            "qty": qty.to_string(),
            "side": side,
            "type": "stop_limit",
            "stop_price": format_order_price(stop_price),
            "limit_price": format_order_price(limit_price),
            "time_in_force": tif,
        });
        self.submit_order(&body).await
    }

    /// Place a trailing stop order.
    pub async fn trailing_stop_order(
        &self,
        symbol: &str,
        qty: f64,
        side: &str,
        trail_price: Option<f64>,
        trail_percent: Option<f64>,
        tif: &str,
    ) -> Result<OrderResult, String> {
        let mut body = serde_json::json!({
            "symbol": symbol,
            "qty": qty.to_string(),
            "side": side,
            "type": "trailing_stop",
            "time_in_force": tif,
        });
        if let Some(tp) = trail_price {
            body["trail_price"] = serde_json::json!(format_order_price(tp));
        }
        if let Some(tp) = trail_percent {
            body["trail_percent"] = serde_json::json!(format_order_price(tp));
        }
        self.submit_order(&body).await
    }

    /// Place a bracket order (market entry with TP + SL legs).
    pub async fn bracket_order(
        &self,
        symbol: &str,
        qty: f64,
        side: &str,
        tp_price: f64,
        sl_price: f64,
    ) -> Result<OrderResult, String> {
        // Round prices to valid increments (Alpaca rejects sub-penny for stocks > $1)
        let tp_rounded = format_order_price(tp_price);
        let sl_rounded = format_order_price(sl_price);
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

    /// Place an OCO (one-cancels-other) exit order: TP limit + SL stop on same side.
    /// When one leg fills, the other is automatically cancelled.
    pub async fn oco_order(
        &self,
        symbol: &str,
        qty: f64,
        side: &str,
        tp_price: f64,
        sl_price: f64,
        sl_limit: Option<f64>,
    ) -> Result<OrderResult, String> {
        let mut body = serde_json::json!({
            "symbol": symbol,
            "qty": qty.to_string(),
            "side": side,
            "type": "limit",
            "time_in_force": "gtc",
            "order_class": "oco",
            "take_profit": { "limit_price": format_order_price(tp_price) },
            "stop_loss": { "stop_price": format_order_price(sl_price) },
        });
        if let Some(sl_lim) = sl_limit {
            body["stop_loss"]["limit_price"] = serde_json::json!(format_order_price(sl_lim));
        }
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
                ("nested", "true"), // include bracket legs (SL/TP child orders)
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
    pub async fn modify_order(
        &self,
        order_id: &str,
        qty: Option<f64>,
        limit_price: Option<f64>,
        stop_price: Option<f64>,
        trail: Option<f64>,
    ) -> Result<OrderResult, String> {
        let mut body = serde_json::Map::new();
        if let Some(q) = qty {
            body.insert("qty".into(), serde_json::json!(q.to_string()));
        }
        if let Some(lp) = limit_price {
            body.insert(
                "limit_price".into(),
                serde_json::json!(format_order_price(lp)),
            );
        }
        if let Some(sp) = stop_price {
            body.insert(
                "stop_price".into(),
                serde_json::json!(format_order_price(sp)),
            );
        }
        if let Some(t) = trail {
            body.insert("trail".into(), serde_json::json!(t.to_string()));
        }

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
        // Parse bracket legs recursively
        let legs = o["legs"]
            .as_array()
            .map(|arr| arr.iter().map(Self::parse_order_info).collect());
        // Helper: parse price field that may be string or number
        let price_field = |v: &serde_json::Value| -> Option<String> {
            v.as_str()
                .map(|s| s.to_string())
                .or_else(|| v.as_f64().map(|f| f.to_string()))
        };
        OrderInfo {
            id: o["id"].as_str().unwrap_or("").to_string(),
            symbol: o["symbol"].as_str().unwrap_or("").to_string(),
            qty: o["qty"]
                .as_str()
                .unwrap_or_else(|| {
                    // qty may be numeric in some API responses
                    "0"
                })
                .to_string(),
            filled_qty: o["filled_qty"].as_str().unwrap_or("0").to_string(),
            side: o["side"].as_str().unwrap_or("").to_string(),
            order_type: o["type"].as_str().unwrap_or("").to_string(),
            order_class: o["order_class"].as_str().map(|s| s.to_string()),
            status: o["status"].as_str().unwrap_or("").to_string(),
            limit_price: price_field(&o["limit_price"]),
            stop_price: price_field(&o["stop_price"]),
            trail_price: price_field(&o["trail_price"]),
            trail_percent: price_field(&o["trail_percent"]),
            created_at: o["created_at"].as_str().unwrap_or("").to_string(),
            filled_at: o["filled_at"].as_str().map(|s| s.to_string()),
            filled_avg_price: price_field(&o["filled_avg_price"]),
            legs,
        }
    }

    fn normalize_order_symbol(symbol: &str) -> String {
        symbol.replace('/', "").to_ascii_uppercase()
    }

    fn order_status_is_cancellable(status: &str) -> bool {
        !matches!(
            status.to_ascii_lowercase().as_str(),
            "filled" | "canceled" | "cancelled" | "expired" | "rejected"
        )
    }

    fn collect_cancellable_order_ids_for_symbol(orders: &[OrderInfo], symbol: &str) -> Vec<String> {
        fn walk(
            order: &OrderInfo,
            target_symbol: &str,
            ids: &mut Vec<String>,
            seen: &mut HashSet<String>,
        ) {
            if AlpacaBroker::normalize_order_symbol(&order.symbol) == target_symbol
                && AlpacaBroker::order_status_is_cancellable(&order.status)
                && seen.insert(order.id.clone())
            {
                ids.push(order.id.clone());
            }
            if let Some(legs) = order.legs.as_ref() {
                for leg in legs {
                    walk(leg, target_symbol, ids, seen);
                }
            }
        }

        let target_symbol = Self::normalize_order_symbol(symbol);
        let mut ids = Vec::new();
        let mut seen = HashSet::new();
        for order in orders {
            walk(order, &target_symbol, &mut ids, &mut seen);
        }
        ids
    }

    fn collect_cancellable_exit_order_ids_for_symbol(
        orders: &[OrderInfo],
        symbol: &str,
        exit_side: &str,
    ) -> Vec<String> {
        fn walk(
            order: &OrderInfo,
            target_symbol: &str,
            exit_side: &str,
            ids: &mut Vec<String>,
            seen: &mut HashSet<String>,
        ) {
            if AlpacaBroker::normalize_order_symbol(&order.symbol) == target_symbol
                && order.side.eq_ignore_ascii_case(exit_side)
                && AlpacaBroker::order_status_is_cancellable(&order.status)
                && seen.insert(order.id.clone())
            {
                ids.push(order.id.clone());
            }
            if let Some(legs) = order.legs.as_ref() {
                for leg in legs {
                    walk(leg, target_symbol, exit_side, ids, seen);
                }
            }
        }

        let target_symbol = Self::normalize_order_symbol(symbol);
        let mut ids = Vec::new();
        let mut seen = HashSet::new();
        for order in orders {
            walk(order, &target_symbol, exit_side, &mut ids, &mut seen);
        }
        ids
    }

    fn is_insufficient_qty_close_reject(message: &str) -> bool {
        message
            .to_ascii_lowercase()
            .contains("insufficient qty available")
    }

    async fn cancel_open_orders_for_symbol(&self, symbol: &str) -> Result<usize, String> {
        let orders = self.get_orders("open", 200).await?;
        let ids = Self::collect_cancellable_order_ids_for_symbol(&orders, symbol);
        for order_id in &ids {
            self.cancel_order(order_id)
                .await
                .map_err(|e| format!("Cancel open order {order_id} for {symbol} failed: {e}"))?;
        }
        Ok(ids.len())
    }

    pub async fn sync_position_exits(
        &self,
        symbol: &str,
        sl_price: Option<f64>,
        tp_price: Option<f64>,
    ) -> Result<String, String> {
        let positions = self.get_positions().await?;
        let pos = positions
            .iter()
            .find(|p| p.symbol.eq_ignore_ascii_case(symbol))
            .ok_or_else(|| format!("No position found for {symbol}"))?;
        let exit_side = if pos.side.eq_ignore_ascii_case("long") {
            "sell"
        } else {
            "buy"
        };
        let qty = pos.qty.abs();
        if qty <= 0.0 {
            return Err(format!("Position {symbol} has zero quantity"));
        }

        let orders = self.get_orders("open", 200).await?;
        let ids = Self::collect_cancellable_exit_order_ids_for_symbol(&orders, symbol, exit_side);
        for order_id in &ids {
            self.cancel_order(order_id).await.map_err(|e| {
                format!("Cancel existing exit order {order_id} for {symbol} failed: {e}")
            })?;
        }
        if !ids.is_empty() {
            tokio::time::sleep(std::time::Duration::from_millis(350)).await;
        }

        let placement = match (sl_price, tp_price) {
            (Some(sl), Some(tp)) => {
                self.oco_order(symbol, qty, exit_side, tp, sl, None).await?;
                format!(
                    "synced OCO exits (tp={} sl={})",
                    format_order_price(tp),
                    format_order_price(sl)
                )
            }
            (Some(sl), None) => {
                self.stop_order(symbol, qty, exit_side, sl, "gtc").await?;
                format!("synced SL {}", format_order_price(sl))
            }
            (None, Some(tp)) => {
                self.limit_order(symbol, qty, exit_side, tp, "gtc").await?;
                format!("synced TP {}", format_order_price(tp))
            }
            (None, None) => "cleared exits".to_string(),
        };

        Ok(format!(
            "{} for {} {} {} (cancelled {} existing exit order(s))",
            placement,
            exit_side,
            qty,
            symbol,
            ids.len()
        ))
    }

    async fn close_position_once(
        &self,
        symbol: &str,
        qty: Option<f64>,
    ) -> Result<OrderResult, String> {
        // Alpaca position endpoint uses symbol without slash (BTC/USD → BTCUSD)
        let encoded_symbol = symbol.replace('/', "%2F");
        let url = if let Some(q) = qty {
            format!(
                "{}/v2/positions/{}?qty={}",
                self.base_url, encoded_symbol, q
            )
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

        let status_code = resp.status();
        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Close parse failed: {e}"))?;

        if let Some(msg) = json["message"].as_str() {
            if !msg.is_empty() {
                return Err(format!("Close position rejected: {msg}"));
            }
        }
        if !status_code.is_success() {
            return Err(format!("Close position failed: HTTP {status_code}"));
        }

        Ok(OrderResult {
            id: json["id"].as_str().unwrap_or("").to_string(),
            symbol: json["symbol"].as_str().unwrap_or("").to_string(),
            qty: json["qty"].as_str().unwrap_or("0").to_string(),
            side: json["side"].as_str().unwrap_or("").to_string(),
            status: json["status"].as_str().unwrap_or("").to_string(),
        })
    }

    pub async fn close_position(
        &self,
        symbol: &str,
        qty: Option<f64>,
    ) -> Result<OrderResult, String> {
        let cancelled_orders = match self.cancel_open_orders_for_symbol(symbol).await {
            Ok(count) => count,
            Err(e) => {
                tracing::warn!(
                    "Close {}: failed to pre-cancel open orders before close: {}",
                    symbol,
                    e
                );
                0
            }
        };
        if cancelled_orders > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(350)).await;
        }

        match self.close_position_once(symbol, qty).await {
            Ok(result) => Ok(result),
            Err(e) if cancelled_orders > 0 && Self::is_insufficient_qty_close_reject(&e) => {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                self.close_position_once(symbol, qty)
                    .await
                    .map_err(|retry_err| {
                        format!(
                            "{} (after cancelling {} open order(s) for {})",
                            retry_err, cancelled_orders, symbol
                        )
                    })
            }
            Err(e) if Self::is_insufficient_qty_close_reject(&e) => Err(format!(
                "{} — quantity may be reserved by open orders or the position snapshot is stale",
                e
            )),
            Err(e) => Err(e),
        }
    }

    pub async fn close_all_positions(&self) -> Result<(), String> {
        let resp = self
            .client
            .delete(format!("{}/v2/positions", self.base_url))
            .headers(self.headers())
            .send()
            .await
            .map_err(|e| format!("Close all failed: {e}"))?;
        if !resp.status().is_success() {
            let status_code = resp.status();
            let body = resp.text().await.unwrap_or_default();
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                if let Some(msg) = json["message"].as_str() {
                    if !msg.is_empty() {
                        return Err(format!("Close all rejected: {msg}"));
                    }
                }
            }
            return Err(format!("Close all failed: HTTP {status_code}"));
        }
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
            min_trade_increment: json["min_trade_increment"]
                .as_str()
                .and_then(|s| s.parse().ok()),
            price_increment: json["price_increment"]
                .as_str()
                .and_then(|s| s.parse().ok()),
        })
    }

    // ── News ─────────────────────────────────────────────────────

    pub async fn get_news(
        &self,
        symbol: &str,
        limit: u32,
    ) -> Result<Vec<serde_json::Value>, String> {
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

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("News parse failed: {e}"))?;

        Ok(json["news"].as_array().cloned().unwrap_or_default())
    }

    // ── Finnhub News (secondary source, free API key) ──────────────

    pub async fn get_finnhub_news(
        symbol: &str,
        finnhub_key: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        if finnhub_key.is_empty() {
            return Ok(vec![]);
        }
        // Strip /USD for crypto symbols (Finnhub uses BINANCE:BTCUSDT format for crypto)
        let clean_sym = symbol.replace("/USD", "").replace("/", "");

        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let week_ago = (chrono::Utc::now() - chrono::Duration::days(7))
            .format("%Y-%m-%d")
            .to_string();

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

        let articles: Vec<serde_json::Value> = resp
            .json()
            .await
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

    // ── Alpha Vantage — Earnings (EPS estimates vs actual) ─────────

    pub async fn get_alpha_vantage_earnings(
        &self,
        symbol: &str,
        av_key: &str,
    ) -> Result<serde_json::Value, String> {
        if av_key.is_empty() {
            return Err("Alpha Vantage API key required".into());
        }
        let resp = sec_client()
            .get("https://www.alphavantage.co/query")
            .query(&[
                ("function", "EARNINGS"),
                ("symbol", symbol),
                ("apikey", av_key),
            ])
            .send()
            .await
            .map_err(|e| format!("AV earnings failed: {e}"))?;
        if !resp.status().is_success() {
            return Err(format!("AV earnings: HTTP {}", resp.status()));
        }
        resp.json()
            .await
            .map_err(|e| format!("AV parse failed: {e}"))
    }

    // ── FMP — Analyst Ratings ────────────────────────────────────

    pub async fn get_fmp_analyst_ratings(
        &self,
        symbol: &str,
        fmp_key: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        if fmp_key.is_empty() {
            return Err("FMP API key required".into());
        }
        let resp = sec_client()
            .get(format!(
                "https://financialmodelingprep.com/api/v3/grade/{}",
                symbol
            ))
            .query(&[("apikey", fmp_key), ("limit", "20")])
            .send()
            .await
            .map_err(|e| format!("FMP ratings failed: {e}"))?;
        if !resp.status().is_success() {
            return Err(format!("FMP ratings: HTTP {}", resp.status()));
        }
        resp.json()
            .await
            .map_err(|e| format!("FMP parse failed: {e}"))
    }

    // ── Portfolio History ────────────────────────────────────────

    pub async fn get_portfolio_history(
        &self,
        period: &str,
        timeframe: &str,
    ) -> Result<serde_json::Value, String> {
        self.rate_limiter.wait().await;
        let resp = self
            .client
            .get(format!("{}/v2/account/portfolio/history", self.base_url))
            .headers(self.headers())
            .query(&[("period", period), ("timeframe", timeframe)])
            .send()
            .await
            .map_err(|e| format!("Portfolio history request failed: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("Portfolio history: HTTP {}", resp.status()));
        }
        resp.json()
            .await
            .map_err(|e| format!("Portfolio history parse failed: {e}"))
    }

    // ── Market Clock ─────────────────────────────────────────────

    pub async fn get_market_clock(&self) -> Result<serde_json::Value, String> {
        self.rate_limiter.wait().await;
        let resp = self
            .client
            .get(format!("{}/v2/clock", self.base_url))
            .headers(self.headers())
            .send()
            .await
            .map_err(|e| format!("Market clock request failed: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("Market clock: HTTP {}", resp.status()));
        }
        resp.json()
            .await
            .map_err(|e| format!("Market clock parse failed: {e}"))
    }

    // ── Corporate Actions (Earnings/Dividends) ──────────────────

    pub async fn get_corporate_actions(
        &self,
        symbol: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        self.rate_limiter.wait().await;
        let resp = self
            .client
            .get(format!("{}/v1beta1/corporate-actions", DATA_BASE))
            .headers(self.headers())
            .query(&[
                ("symbols", symbol),
                ("types", "dividend,split,merger,spinoff"),
            ])
            .send()
            .await;

        match resp {
            Ok(r) if r.status().is_success() => {
                let json: serde_json::Value = r
                    .json()
                    .await
                    .map_err(|e| format!("Corporate actions parse failed: {e}"))?;
                Ok(json.as_array().cloned().unwrap_or_default())
            }
            Ok(r) => Err(format!("Corporate actions: HTTP {}", r.status())),
            Err(e) => Err(format!("Corporate actions request failed: {e}")),
        }
    }

    // ── Finnhub Recommendation Trends ────────────────────────────

    pub async fn get_finnhub_recommendations(
        &self,
        symbol: &str,
        finnhub_key: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        if finnhub_key.is_empty() {
            return Err("Finnhub API key required".into());
        }
        let resp = sec_client()
            .get("https://finnhub.io/api/v1/stock/recommendation")
            .query(&[("symbol", symbol), ("token", finnhub_key)])
            .send()
            .await
            .map_err(|e| format!("Finnhub recommendations failed: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("Finnhub recommendations: HTTP {}", resp.status()));
        }
        resp.json()
            .await
            .map_err(|e| format!("Finnhub recommendations parse failed: {e}"))
    }

    // ── Finnhub Price Targets ────────────────────────────────────

    pub async fn get_finnhub_price_target(
        &self,
        symbol: &str,
        finnhub_key: &str,
    ) -> Result<serde_json::Value, String> {
        if finnhub_key.is_empty() {
            return Err("Finnhub API key required".into());
        }
        let resp = sec_client()
            .get("https://finnhub.io/api/v1/stock/price-target")
            .query(&[("symbol", symbol), ("token", finnhub_key)])
            .send()
            .await
            .map_err(|e| format!("Finnhub price target failed: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("Finnhub price target: HTTP {}", resp.status()));
        }
        resp.json()
            .await
            .map_err(|e| format!("Finnhub price target parse failed: {e}"))
    }

    // ── Finnhub Insider Sentiment ────────────────────────────────

    pub async fn get_finnhub_insider_sentiment(
        &self,
        symbol: &str,
        finnhub_key: &str,
    ) -> Result<serde_json::Value, String> {
        if finnhub_key.is_empty() {
            return Err("Finnhub API key required".into());
        }
        let resp = sec_client()
            .get("https://finnhub.io/api/v1/stock/insider-sentiment")
            .query(&[
                ("symbol", symbol),
                ("token", finnhub_key),
                ("from", "2024-01-01"),
            ])
            .send()
            .await
            .map_err(|e| format!("Finnhub insider sentiment failed: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("Finnhub insider sentiment: HTTP {}", resp.status()));
        }
        resp.json()
            .await
            .map_err(|e| format!("Finnhub insider sentiment parse failed: {e}"))
    }

    // ── SEC EDGAR Filings ─────────────────────────────────────────

    /// Fetch recent SEC filings for a company via EDGAR API (free, no auth).
    /// CIK lookup by ticker, then fetch filings.
    /// Hardened: uses .query() to prevent URL parameter injection.
    pub async fn get_sec_filings(
        ticker: &str,
        filing_type: &str,
        _limit: u32,
    ) -> Result<serde_json::Value, String> {
        if ticker.is_empty()
            || ticker.len() > 10
            || !ticker
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '/')
        {
            return Err("Invalid ticker for SEC lookup".to_string());
        }
        if !matches!(
            filing_type,
            "10-K" | "10-Q" | "8-K" | "S-1" | "DEF 14A" | "13F" | "4" | "SC 13D" | "SC 13G"
        ) {
            return Err("Invalid filing type".to_string());
        }
        let client = sec_client();
        let ua = "TyphooN-Terminal/1.0 typhoon-terminal@example.invalid";

        // Step 1: Resolve ticker → CIK via SEC company tickers JSON
        let tickers_resp = client
            .get("https://www.sec.gov/files/company_tickers.json")
            .header("User-Agent", ua)
            .send()
            .await
            .map_err(|e| format!("SEC tickers lookup failed: {e}"))?;

        let tickers_json: serde_json::Value = tickers_resp
            .json()
            .await
            .map_err(|e| format!("SEC tickers parse failed: {e}"))?;

        // Find CIK for this ticker (case-insensitive)
        let upper_ticker = ticker.to_uppercase();
        let mut cik: Option<u64> = None;
        let mut company_name = String::new();
        if let Some(obj) = tickers_json.as_object() {
            for (_, entry) in obj {
                if let Some(t) = entry["ticker"].as_str() {
                    if t.to_uppercase() == upper_ticker {
                        cik = entry["cik_str"]
                            .as_u64()
                            .or_else(|| entry["cik_str"].as_str().and_then(|s| s.parse().ok()));
                        company_name = entry["title"].as_str().unwrap_or("").to_string();
                        break;
                    }
                }
            }
        }

        let cik_num = match cik {
            Some(c) => c,
            None => {
                return Ok(
                    serde_json::json!({ "hits": { "hits": [] }, "error": format!("Ticker '{}' not found in SEC EDGAR", ticker) }),
                );
            }
        };

        // Step 2: Fetch filings by CIK from submissions endpoint
        let cik_padded = format!("{:010}", cik_num);
        let submissions_resp = client
            .get(&format!(
                "https://data.sec.gov/submissions/CIK{}.json",
                cik_padded
            ))
            .header("User-Agent", ua)
            .send()
            .await
            .map_err(|e| format!("SEC submissions failed: {e}"))?;

        if !submissions_resp.status().is_success() {
            return Err(format!(
                "SEC submissions: HTTP {}",
                submissions_resp.status()
            ));
        }

        let submissions: serde_json::Value = submissions_resp
            .json()
            .await
            .map_err(|e| format!("SEC submissions parse failed: {e}"))?;

        // Step 3: Filter recent filings by type
        let recent = &submissions["filings"]["recent"];
        let forms = recent["form"].as_array();
        let dates = recent["filingDate"].as_array();
        let accessions = recent["accessionNumber"].as_array();
        let primary_docs = recent["primaryDocument"].as_array();
        let descriptions = recent["primaryDocDescription"].as_array();

        let mut hits = Vec::new();
        if let (Some(forms), Some(dates), Some(accessions)) = (forms, dates, accessions) {
            for i in 0..forms.len().min(200) {
                let form = forms[i].as_str().unwrap_or("");
                if form != filing_type {
                    continue;
                }
                let date = dates.get(i).and_then(|d| d.as_str()).unwrap_or("");
                let acc = accessions.get(i).and_then(|a| a.as_str()).unwrap_or("");
                let doc = primary_docs
                    .and_then(|d| d.get(i))
                    .and_then(|d| d.as_str())
                    .unwrap_or("");
                let desc = descriptions
                    .and_then(|d| d.get(i))
                    .and_then(|d| d.as_str())
                    .unwrap_or("");
                let acc_clean = acc.replace('-', "");
                let url = format!(
                    "https://www.sec.gov/Archives/edgar/data/{}/{}/{}",
                    cik_num, acc_clean, doc
                );

                hits.push(serde_json::json!({
                    "_source": {
                        "file_date": date,
                        "form_type": form,
                        "display_names": [format!("{} (CIK {})", company_name, cik_padded)],
                        "file_description": desc,
                    },
                    "_id": url,
                }));
                if hits.len() >= 20 {
                    break;
                }
            }
        }

        Ok(serde_json::json!({ "hits": { "hits": hits } }))
    }

    /// Fetch company facts from SEC EDGAR (financials, shares outstanding, etc.)
    /// Hardened: timeout, validated ticker, generic error messages.
    /// Uses cached ticker map and shared HTTP client.
    pub async fn get_sec_company_facts(ticker: &str) -> Result<serde_json::Value, String> {
        if ticker.is_empty()
            || ticker.len() > 10
            || !ticker.chars().all(|c| c.is_ascii_alphanumeric())
        {
            return Err("Invalid ticker for SEC lookup".to_string());
        }
        let client = sec_client();
        let cik = lookup_cik(ticker).await?;
        let cik_padded = format!("CIK{:010}", cik);

        let facts_resp = client
            .get(format!(
                "https://data.sec.gov/api/xbrl/companyfacts/{}.json",
                cik_padded
            ))
            .header(
                "User-Agent",
                "TyphooN-Terminal/1.0 typhoon-terminal@example.invalid",
            )
            .send()
            .await
            .map_err(|e| format!("SEC company facts failed: {e}"))?;

        if !facts_resp.status().is_success() {
            return Err(format!("SEC company facts: HTTP {}", facts_resp.status()));
        }

        let facts: serde_json::Value = facts_resp
            .json()
            .await
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
            "eps_diluted": extract_latest_fact(us_gaap, "EarningsPerShareDiluted"),
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
                min_trade_increment: a["min_trade_increment"]
                    .as_str()
                    .and_then(|s| s.parse().ok()),
                price_increment: a["price_increment"].as_str().and_then(|s| s.parse().ok()),
            })
            .collect())
    }

    // ── Historical Data ──────────────────────────────────────────────

    /// Fetch ALL available bars for a symbol/timeframe from Alpaca.
    /// Paginates from the earliest available data to the present.
    /// No limit cap — continues until the API returns no more data.
    /// Use for initial full history download. Stores progress to callback.
    pub async fn get_all_bars(
        &self,
        symbol: &str,
        timeframe: &str,
        progress: Option<&tokio::sync::mpsc::UnboundedSender<String>>,
    ) -> Result<(Vec<Bar>, FetchOutcome), String> {
        let is_crypto = symbol.contains('/');

        // Monthly: aggregate from weekly
        if timeframe == "1Month" {
            let (weekly, outcome) = Box::pin(self.get_all_bars(symbol, "1Week", progress)).await?;
            return Ok((Self::aggregate_weekly_to_monthly(&weekly), outcome));
        }

        let base = if is_crypto {
            format!("{}/v1beta3/crypto/us/bars", DATA_BASE)
        } else {
            format!("{}/v2/stocks/{}/bars", DATA_BASE, symbol)
        };

        let feeds: Vec<Option<&str>> = if is_crypto {
            vec![None]
        } else {
            self.stock_bar_feeds()
        };

        // Start from the earliest reasonable date
        let start_date = if is_crypto {
            "2015-01-01"
        } else {
            "2000-01-01"
        };
        let mut last_error = String::new();

        for feed in &feeds {
            let mut all_bars: Vec<Bar> = Vec::new();
            let mut next_page_token: Option<String> = None;
            let mut chunk_count = 0u32;
            let fetch_start = std::time::Instant::now();
            // Track whether we aborted mid-pagination due to a 429 so the
            // caller can enqueue a retry for the unfetched tail.
            let mut rate_limited = false;

            loop {
                self.bar_rate_limiter.wait().await;
                let chunk_timer = std::time::Instant::now();

                let mut params = vec![
                    ("timeframe", timeframe.to_string()),
                    ("limit", "10000".to_string()),
                    ("sort", "asc".to_string()),
                ];
                if let Some(ref token) = next_page_token {
                    params.push(("page_token", token.clone()));
                } else {
                    params.push(("start", format!("{}T00:00:00Z", start_date)));
                }
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
                let _ = self
                    .bar_rate_limiter
                    .observe_rate_limit_headers(resp.headers())
                    .await;

                if resp.status().as_u16() == 429 {
                    self.bar_rate_limiter.trigger_cooldown().await;
                    self.bar_rate_limiter.wait().await;
                    rate_limited = true;
                    if all_bars.is_empty() {
                        last_error = "Rate limited".into();
                        break;
                    }
                    // Accept partial data on rate limit
                    break;
                }
                if !resp.status().is_success() {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    if matches!(feed, Some("sip")) {
                        let _ = self.note_sip_bar_entitlement_failure(status, &body);
                    }
                    last_error = format!("HTTP {} (feed={:?})", status, feed);
                    break;
                }

                let json: serde_json::Value = match resp.json().await {
                    Ok(j) => j,
                    Err(e) => {
                        last_error = format!("Parse: {e}");
                        break;
                    }
                };
                self.bar_rate_limiter
                    .report_latency(chunk_timer.elapsed().as_millis() as u64)
                    .await;

                let new_page_token = json
                    .get("next_page_token")
                    .and_then(|t| t.as_str())
                    .map(|s| s.to_string());
                let chunk_bars = Self::parse_bars(&json, symbol, is_crypto);
                let bars_in_chunk = chunk_bars.len();
                chunk_count += 1;

                if bars_in_chunk == 0 {
                    break;
                }

                all_bars.extend(chunk_bars);

                // Report progress
                if let Some(tx) = progress {
                    let elapsed = fetch_start.elapsed().as_secs();
                    let last_ts = all_bars
                        .last()
                        .map(|b| &b.timestamp[..10.min(b.timestamp.len())])
                        .unwrap_or("?");
                    let _ = tx.send(format!(
                        "{} {}: {} bars (chunk #{}, {}s, latest: {})",
                        symbol,
                        timeframe,
                        all_bars.len(),
                        chunk_count,
                        elapsed,
                        last_ts
                    ));
                }

                match new_page_token {
                    Some(token) if !token.is_empty() => {
                        next_page_token = Some(token);
                    }
                    _ => break,
                }
            }

            if !all_bars.is_empty() {
                all_bars.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
                all_bars.dedup_by(|a, b| a.timestamp == b.timestamp);
                let outcome = if rate_limited {
                    FetchOutcome::RateLimitedPartial
                } else {
                    FetchOutcome::Complete
                };
                tracing::debug!(
                    "{} {}: get_all_bars {} — {} bars in {}s",
                    symbol,
                    timeframe,
                    if rate_limited {
                        "partial (rate-limited)"
                    } else {
                        "complete"
                    },
                    all_bars.len(),
                    fetch_start.elapsed().as_secs()
                );
                return Ok((all_bars, outcome));
            }
            if rate_limited {
                // First-page 429 with zero bars — signal to the retry queue.
                return Ok((Vec::new(), FetchOutcome::RateLimitedEmpty));
            }
        }
        if last_error.is_empty() {
            Ok((Vec::new(), FetchOutcome::Complete))
        } else {
            Err(last_error)
        }
    }

    /// Fetch stock bars for multiple symbols with Alpaca's `/v2/stocks/bars`
    /// endpoint. This is intentionally stock-only and targeted at high-timeframe
    /// Kraken-equities assist work where one request can return many symbols
    /// without collapsing provenance into the Kraken cache namespace.
    pub async fn get_stock_bars_batch_targeted(
        &self,
        symbols: &[String],
        timeframe: &str,
        limit: u32,
    ) -> Result<(HashMap<String, Vec<Bar>>, FetchOutcome), String> {
        let symbols: Vec<String> = symbols
            .iter()
            .map(|symbol| symbol.trim().to_ascii_uppercase())
            .filter(|symbol| !symbol.is_empty() && !symbol.contains('/'))
            .collect();
        if symbols.is_empty() {
            return Ok((HashMap::new(), FetchOutcome::Complete));
        }
        if timeframe == "1Month" {
            return Err("Alpaca batch bars do not support 1Month aggregation".to_string());
        }

        let start = chrono::Utc::now()
            - chrono::Duration::days(lookback_days_for_request(
                false,
                timeframe,
                limit,
                BarsLookbackMode::Targeted,
            ));
        let symbol_csv = symbols.join(",");
        let mut last_error = String::new();

        for feed in self.stock_bar_feeds() {
            let mut out: HashMap<String, Vec<Bar>> = HashMap::new();
            let mut next_page_token: Option<String> = None;
            let mut rate_limited = false;
            loop {
                self.bar_rate_limiter.wait().await;
                let mut params = vec![
                    ("symbols", symbol_csv.clone()),
                    ("timeframe", timeframe.to_string()),
                    ("limit", limit.min(10_000).max(1).to_string()),
                    ("sort", "asc".to_string()),
                    ("adjustment", "all".to_string()),
                ];
                if let Some(ref token) = next_page_token {
                    params.push(("page_token", token.clone()));
                } else {
                    params.push(("start", start.format("%Y-%m-%dT00:00:00Z").to_string()));
                }
                if let Some(feed) = feed {
                    params.push(("feed", feed.to_string()));
                }

                let resp = self
                    .client
                    .get(format!("{}/v2/stocks/bars", DATA_BASE))
                    .headers(self.headers())
                    .query(&params)
                    .send()
                    .await
                    .map_err(|e| format!("Batch bars request failed: {e}"))?;
                let _ = self
                    .bar_rate_limiter
                    .observe_rate_limit_headers(resp.headers())
                    .await;
                if resp.status().as_u16() == 429 {
                    self.bar_rate_limiter.trigger_cooldown().await;
                    rate_limited = true;
                    break;
                }
                if !resp.status().is_success() {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    if matches!(feed, Some("sip")) {
                        let _ = self.note_sip_bar_entitlement_failure(status, &body);
                    }
                    last_error = format!("HTTP {} (feed={:?})", status, feed);
                    break;
                }
                let json: serde_json::Value = resp
                    .json()
                    .await
                    .map_err(|e| format!("Batch bars parse failed: {e}"))?;
                for (symbol, mut bars) in Self::parse_stock_bars_by_symbol(&json, &symbols) {
                    out.entry(symbol).or_default().append(&mut bars);
                }
                next_page_token = json
                    .get("next_page_token")
                    .and_then(|t| t.as_str())
                    .filter(|token| !token.is_empty())
                    .map(|token| token.to_string());
                if next_page_token.is_none() {
                    break;
                }
            }

            if !out.is_empty() {
                for bars in out.values_mut() {
                    bars.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
                    bars.dedup_by(|a, b| a.timestamp == b.timestamp);
                }
                return Ok((
                    out,
                    if rate_limited {
                        FetchOutcome::RateLimitedPartial
                    } else {
                        FetchOutcome::Complete
                    },
                ));
            }
            if rate_limited {
                return Ok((HashMap::new(), FetchOutcome::RateLimitedEmpty));
            }
        }

        if last_error.is_empty() {
            Ok((HashMap::new(), FetchOutcome::Complete))
        } else {
            Err(last_error)
        }
    }

    pub async fn get_bars(
        &self,
        symbol: &str,
        timeframe: &str,
        limit: u32,
    ) -> Result<Vec<Bar>, String> {
        // Strip the FetchOutcome here so this trait-exposed method keeps its
        // existing signature. The retry queue runs off direct get_bars_after
        // callers that care about partial/rate-limited results.
        self.get_bars_window(
            symbol,
            timeframe,
            limit,
            None,
            BarsLookbackMode::Targeted,
            None,
        )
        .await
        .map(|(bars, _)| bars)
    }

    /// Fetch up to `limit` bars using a wider lookback window sized for the
    /// terminal's automated sync target. This is deeper than the incremental
    /// freshness path, but still bounded unlike `get_all_bars()`.
    pub async fn get_target_bars(
        &self,
        symbol: &str,
        timeframe: &str,
        limit: u32,
    ) -> Result<(Vec<Bar>, FetchOutcome), String> {
        self.get_bars_window(
            symbol,
            timeframe,
            limit,
            None,
            BarsLookbackMode::Targeted,
            None,
        )
        .await
    }

    /// Fetch bars, optionally starting after a given timestamp (for incremental fetching).
    /// When `after_timestamp` is Some, fetches only bars newer than that timestamp,
    /// dramatically reducing API calls for cached data.
    pub async fn get_bars_after(
        &self,
        symbol: &str,
        timeframe: &str,
        limit: u32,
        after_timestamp: Option<&str>,
    ) -> Result<(Vec<Bar>, FetchOutcome), String> {
        self.get_bars_window(
            symbol,
            timeframe,
            limit,
            after_timestamp,
            BarsLookbackMode::Incremental,
            None,
        )
        .await
    }

    async fn get_bars_window(
        &self,
        symbol: &str,
        timeframe: &str,
        limit: u32,
        after_timestamp: Option<&str>,
        lookback_mode: BarsLookbackMode,
        display_timeframe: Option<&str>,
    ) -> Result<(Vec<Bar>, FetchOutcome), String> {
        let is_crypto = symbol.contains('/');

        // Alpaca doesn't support 1Month — fetch weekly bars and aggregate
        if timeframe == "1Month" {
            // Fetch enough weekly bars: ~4.3 weeks per month, request 5x for safety
            let (weekly, outcome) = Box::pin(self.get_bars_window(
                symbol,
                "1Week",
                (limit * 5).max(1000),
                after_timestamp,
                lookback_mode,
                Some("1Month"),
            ))
            .await?;
            let monthly = Self::aggregate_weekly_to_monthly(&weekly);
            let trimmed = if monthly.len() > limit as usize {
                monthly[monthly.len() - limit as usize..].to_vec()
            } else {
                monthly
            };
            return Ok((trimmed, outcome));
        }

        let actual_tf = timeframe;
        let log_tf = display_timeframe.unwrap_or(actual_tf);
        let aggregation_suffix = if display_timeframe == Some("1Month") && actual_tf == "1Week" {
            " via 1Week aggregation"
        } else {
            ""
        };
        let actual_limit = limit;

        // Try multiple feeds in order: sip (paid) → iex (free) for stocks
        // Crypto uses a different endpoint and doesn't need a feed param
        let feeds: Vec<Option<&str>> = if is_crypto {
            vec![None] // crypto endpoint doesn't use feed param
        } else {
            self.stock_bar_feeds() // try free tier first
        };

        let base = if is_crypto {
            format!("{}/v1beta3/crypto/us/bars", DATA_BASE)
        } else {
            format!("{}/v2/stocks/{}/bars", DATA_BASE, symbol)
        };
        let lookback_days =
            lookback_days_for_request(is_crypto, actual_tf, actual_limit, lookback_mode);

        // Incremental fetch: if caller provides a cached timestamp, start from there
        // instead of the full lookback. This reduces 13+ chunks to 1-2 chunks.
        let earliest_start = if let Some(after_ts) = after_timestamp {
            if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(after_ts) {
                let cached_start = parsed.with_timezone(&chrono::Utc);
                tracing::debug!(
                    "{} @ {}{}: incremental fetch from {} (cache hit)",
                    symbol,
                    log_tf,
                    aggregation_suffix,
                    &after_ts[..19.min(after_ts.len())]
                );
                cached_start
            } else {
                chrono::Utc::now() - chrono::Duration::days(lookback_days)
            }
        } else {
            chrono::Utc::now() - chrono::Duration::days(lookback_days)
        };

        let mut last_error = String::new();

        for feed in &feeds {
            let mut all_bars: Vec<Bar> = Vec::new();
            // Use Alpaca's native page_token pagination for efficient chunk fetching.
            // Fetch oldest→newest so page_token works correctly, then trim to most recent.
            let mut next_page_token: Option<String> = None;
            let mut rate_limit_retries = 0;
            let mut chunk_count = 0u32;
            const MAX_RATE_LIMIT_RETRIES: u32 = 3;
            let fetch_start = std::time::Instant::now();
            // Flip true if we bailed mid-pagination due to exhausted 429 retries
            // so the caller can queue a follow-up fetch for the tail.
            let mut rate_limited = false;

            loop {
                // Centralized rate limiter — respects global request budget + adaptive pacing
                self.bar_rate_limiter.wait().await;

                let chunk_timer = std::time::Instant::now();

                let mut params = vec![
                    ("timeframe", actual_tf.to_string()),
                    ("limit", "10000".to_string()),
                    ("sort", "asc".to_string()),
                ];
                // Stocks: request fully-adjusted bars (splits + dividends).
                // Alpaca defaults to "raw" which leaves pre-split prices untouched,
                // producing flat-then-spike charts for symbols after a reverse split.
                // The crypto endpoint does not accept this parameter.
                if !is_crypto {
                    params.push(("adjustment", "all".to_string()));
                }
                // Use page_token for continuation, start date only for first request
                if let Some(ref token) = next_page_token {
                    params.push(("page_token", token.clone()));
                } else {
                    let start_str = earliest_start.format("%Y-%m-%dT00:00:00Z").to_string();
                    params.push(("start", start_str));
                }
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
                let _ = self
                    .bar_rate_limiter
                    .observe_rate_limit_headers(resp.headers())
                    .await;

                if !resp.status().is_success() {
                    let status = resp.status();
                    if status.as_u16() == 429 {
                        rate_limit_retries += 1;
                        self.bar_rate_limiter.trigger_cooldown().await;
                        if rate_limit_retries <= MAX_RATE_LIMIT_RETRIES {
                            tracing::warn!(
                                "429 rate limit for {} @ {}: retry {}/{} ({} bars so far)",
                                symbol,
                                actual_tf,
                                rate_limit_retries,
                                MAX_RATE_LIMIT_RETRIES,
                                all_bars.len()
                            );
                            self.bar_rate_limiter.wait().await;
                            continue; // retry the same chunk (page_token unchanged)
                        }
                        rate_limited = true;
                        if !all_bars.is_empty() {
                            tracing::warn!(
                                "429 rate limit: max retries for {} @ {}, returning {} bars",
                                symbol,
                                actual_tf,
                                all_bars.len()
                            );
                            break;
                        }
                    }
                    let body = resp.text().await.unwrap_or_default();
                    if matches!(feed, Some("sip")) {
                        let _ = self.note_sip_bar_entitlement_failure(status, &body);
                    }
                    last_error = format!("HTTP {} (feed={:?})", status, feed);
                    break;
                }

                let json: serde_json::Value = match resp.json().await {
                    Ok(j) => j,
                    Err(e) => {
                        last_error = format!("Parse failed: {e}");
                        break;
                    }
                };

                // Report latency for adaptive pacing
                let chunk_elapsed_ms = chunk_timer.elapsed().as_millis() as u64;
                self.bar_rate_limiter.report_latency(chunk_elapsed_ms).await;

                // Extract next_page_token from response for efficient pagination
                let new_page_token = json
                    .get("next_page_token")
                    .and_then(|t| t.as_str())
                    .map(|s| s.to_string());

                let chunk_bars = Self::parse_bars(&json, symbol, is_crypto);
                let bars_in_chunk = chunk_bars.len();
                chunk_count += 1;

                if bars_in_chunk == 0 {
                    break; // no more data
                }

                // Log chunk progress with date range and timing
                if bars_in_chunk > 5 {
                    if let (Some(first), Some(last)) = (chunk_bars.first(), chunk_bars.last()) {
                        let first_date = &first.timestamp[..10.min(first.timestamp.len())];
                        let last_date = &last.timestamp[..10.min(last.timestamp.len())];
                        let total = all_bars.len() + bars_in_chunk;
                        let elapsed_secs = fetch_start.elapsed().as_secs();
                        // Progress %: use expected bars from lookback (not raw limit which may be 50K)
                        let expected_bars = (lookback_days as f64
                            * bars_per_day(is_crypto, actual_tf))
                        .ceil() as usize;
                        let bars_target = expected_bars.min(actual_limit as usize).max(1);
                        let pct = (total * 100) / bars_target;
                        tracing::debug!(
                            "{} @ {}: chunk #{} +{} bars ({} → {}), total {} ({}%, {}s elapsed, {:.0}ms/chunk)",
                            symbol,
                            log_tf,
                            chunk_count,
                            bars_in_chunk,
                            first_date,
                            last_date,
                            total,
                            pct.min(100),
                            elapsed_secs,
                            chunk_elapsed_ms
                        );
                    }
                }

                all_bars.extend(chunk_bars);

                // Stop if we have enough bars
                if all_bars.len() as u32 >= actual_limit {
                    break;
                }

                // Early termination: if chunks are taking too long, accept what we have.
                // This prevents hours-long fetches on free tier progressive throttling.
                if chunk_elapsed_ms > (SLOW_CHUNK_THRESHOLD_SECS * 1000) as u64
                    && all_bars.len() > 100
                {
                    tracing::warn!(
                        "{} @ {}: chunk took {}s (>{} threshold) — accepting {} bars to avoid progressive throttle",
                        symbol,
                        actual_tf,
                        chunk_elapsed_ms / 1000,
                        SLOW_CHUNK_THRESHOLD_SECS,
                        all_bars.len()
                    );
                    break;
                }

                // Use page_token if available, otherwise we're done
                match new_page_token {
                    Some(token) if !token.is_empty() => {
                        next_page_token = Some(token);
                    }
                    _ => break, // no more pages
                }
            }

            if !all_bars.is_empty() {
                // Sort by timestamp (safety) then deduplicate
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
                let elapsed_secs = fetch_start.elapsed().as_secs();
                tracing::debug!(
                    "Loaded {} bars for {} @ {}{} (feed={}, {} chunks, {}s total)",
                    all_bars.len(),
                    symbol,
                    log_tf,
                    aggregation_suffix,
                    feed_label,
                    chunk_count,
                    elapsed_secs
                );
                let outcome = if rate_limited {
                    FetchOutcome::RateLimitedPartial
                } else {
                    FetchOutcome::Complete
                };
                return Ok((all_bars, outcome));
            }
            if rate_limited {
                // First-page 429 with zero bars — caller should retry.
                return Ok((Vec::new(), FetchOutcome::RateLimitedEmpty));
            }
        }

        Err(format!(
            "No bar data for {symbol} @ {timeframe}: {last_error}"
        ))
    }

    /// Aggregate weekly bars into synthetic monthly bars.
    /// Groups by calendar month (year-month), combines OHLCV.
    pub fn aggregate_weekly_to_monthly(weekly: &[Bar]) -> Vec<Bar> {
        if weekly.is_empty() {
            return vec![];
        }
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
            let ym = if bar.timestamp.len() >= 7 {
                &bar.timestamp[..7]
            } else {
                ""
            };
            if ym != cur_month {
                if !cur_month.is_empty() {
                    monthly.push(Bar {
                        timestamp: month_start.clone(),
                        open,
                        high,
                        low,
                        close,
                        volume,
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
                timestamp: month_start,
                open,
                high,
                low,
                close,
                volume,
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
                    .filter_map(|b| {
                        let ts = b["t"].as_str().unwrap_or("");
                        let o = b["o"].as_f64().unwrap_or(0.0);
                        let h = b["h"].as_f64().unwrap_or(0.0);
                        let l = b["l"].as_f64().unwrap_or(0.0);
                        let c = b["c"].as_f64().unwrap_or(0.0);
                        let v = b["v"].as_f64().unwrap_or(0.0);
                        // Reject bars with missing timestamp or zero/NaN prices
                        if ts.is_empty() || o <= 0.0 || c <= 0.0 || !o.is_finite() || !c.is_finite()
                        {
                            return None;
                        }
                        // Fix OHLC: high must be >= all, low must be <= all
                        let true_high = o.max(h).max(l).max(c);
                        let true_low = o.min(l).min(h).min(c);
                        Some(Bar {
                            timestamp: ts.to_string(),
                            open: o,
                            high: true_high,
                            low: true_low,
                            close: c,
                            volume: if v.is_finite() && v >= 0.0 { v } else { 0.0 },
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn parse_stock_bars_by_symbol(
        json: &serde_json::Value,
        symbols: &[String],
    ) -> HashMap<String, Vec<Bar>> {
        let mut out = HashMap::new();
        let Some(bars_by_symbol) = json["bars"].as_object() else {
            return out;
        };
        for symbol in symbols {
            if let Some(bars) = bars_by_symbol.get(symbol) {
                let wrapped = serde_json::json!({ "bars": bars });
                out.insert(symbol.clone(), Self::parse_bars(&wrapped, symbol, false));
            }
        }
        out
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
            .get(format!(
                "{}/v1beta1/options/snapshots/{}",
                DATA_BASE, underlying_symbol
            ))
            .headers(self.headers())
            .query(&[("feed", "indicative"), ("expiration_date", expiry)])
            .send()
            .await
            .map_err(|e| format!("Options request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let _ = resp.text().await;
            return Err(format!("Options request failed: HTTP {status}"));
        }

        let json: serde_json::Value = resp
            .json()
            .await
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
        contracts.sort_by(|a, b| {
            a.strike
                .partial_cmp(&b.strike)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
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
            format!(
                "20{}-{}-{}",
                &date_str[0..2],
                &date_str[2..4],
                &date_str[4..6]
            )
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
        if ticker.is_empty()
            || ticker.len() > 10
            || !ticker.chars().all(|c| c.is_ascii_alphanumeric())
        {
            return Err("Invalid ticker for SEC lookup".to_string());
        }
        let client = sec_client();
        let cik = lookup_cik(ticker).await?;
        let cik_padded = format!("CIK{:010}", cik);

        let facts_resp = client
            .get(format!(
                "https://data.sec.gov/api/xbrl/companyfacts/{}.json",
                cik_padded
            ))
            .header(
                "User-Agent",
                "TyphooN-Terminal/1.0 typhoon-terminal@example.invalid",
            )
            .send()
            .await
            .map_err(|e| format!("SEC company facts failed: {e}"))?;

        if !facts_resp.status().is_success() {
            return Err(format!("SEC company facts: HTTP {}", facts_resp.status()));
        }

        let facts: serde_json::Value = facts_resp
            .json()
            .await
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
        if ticker.is_empty()
            || ticker.len() > 10
            || !ticker.chars().all(|c| c.is_ascii_alphanumeric())
        {
            return Err("Invalid ticker for SEC lookup".to_string());
        }
        let client = sec_client();
        let cik = lookup_cik(ticker).await?;
        let cik_padded = format!("{:010}", cik);

        let subs_resp = client
            .get(format!(
                "https://data.sec.gov/submissions/CIK{}.json",
                cik_padded
            ))
            .header(
                "User-Agent",
                "TyphooN-Terminal/1.0 typhoon-terminal@example.invalid",
            )
            .send()
            .await
            .map_err(|e| format!("SEC submissions request failed: {e}"))?;

        if !subs_resp.status().is_success() {
            return Err(format!("SEC submissions: HTTP {}", subs_resp.status()));
        }

        let subs: serde_json::Value = subs_resp
            .json()
            .await
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
                    if filings_13f.len() >= 20 {
                        break;
                    }
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
            .get(format!(
                "{}/v1beta1/screener/stocks/most-actives",
                DATA_BASE
            ))
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

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Most actives parse failed: {e}"))?;

        Ok(json)
    }

    /// Fetch top movers (gainers/losers) from Alpaca screener API.
    /// market_type: "stocks" or "crypto"
    pub async fn get_top_movers(
        &self,
        market_type: &str,
        top: u32,
    ) -> Result<serde_json::Value, String> {
        if !matches!(market_type, "stocks" | "crypto") {
            return Err("market_type must be 'stocks' or 'crypto'".to_string());
        }
        self.rate_limiter.wait().await;

        let resp = self
            .client
            .get(format!(
                "{}/v1beta1/screener/{}/movers",
                DATA_BASE, market_type
            ))
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

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Top movers parse failed: {e}"))?;

        Ok(json)
    }

    // ── DOM / Level 2 (Crypto Orderbook) ──────────────────────────────

    /// Fetch crypto orderbook snapshot from Alpaca.
    pub async fn get_orderbook(&self, symbol: &str) -> Result<serde_json::Value, String> {
        self.rate_limiter.wait().await;

        let resp = self
            .client
            .get(format!(
                "{}/v1beta1/crypto/us/orderbooks/snapshots",
                DATA_BASE
            ))
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

        let json: serde_json::Value = resp
            .json()
            .await
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
            let resp = self
                .client
                .get(&url)
                .headers(self.headers())
                .query(&[("symbols", symbol)])
                .send()
                .await
                .map_err(|e| format!("Quote request failed: {e}"))?;
            if !resp.status().is_success() {
                return Err(format!("Quote request failed: HTTP {}", resp.status()));
            }
            let json: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| format!("Quote parse failed: {e}"))?;
            let q = json["quotes"][symbol].clone();
            let bid = q["bp"].as_f64().unwrap_or(0.0);
            let ask = q["ap"].as_f64().unwrap_or(0.0);
            Ok(LatestQuote {
                symbol: symbol.to_string(),
                bid,
                ask,
                bid_size: q["bs"].as_f64().unwrap_or(0.0),
                ask_size: q["as"].as_f64().unwrap_or(0.0),
                spread: ask - bid,
                timestamp: q["t"].as_str().unwrap_or("").to_string(),
            })
        } else {
            // Stocks/ETFs: use snapshot endpoint for pre/post-market data
            // Snapshot returns: { latestTrade, latestQuote, minuteBar, dailyBar, prevDailyBar }
            let url = format!("{}/v2/stocks/{}/snapshot", DATA_BASE, symbol);
            let resp = self
                .client
                .get(&url)
                .headers(self.headers())
                .query(&[("feed", "iex")])
                .send()
                .await
                .map_err(|e| format!("Snapshot request failed: {e}"))?;
            if !resp.status().is_success() {
                return Err(format!("Snapshot request failed: HTTP {}", resp.status()));
            }
            let json: serde_json::Value = resp
                .json()
                .await
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

    // ── Snapshot (watchlist) ────────────────────────────────────────

    /// Fetch snapshot for a symbol: last price, prev close, daily volume.
    /// Works for both stocks (v2/stocks snapshot) and crypto (v1beta3 snapshots).
    pub async fn get_snapshot(&self, symbol: &str) -> Result<SnapshotData, String> {
        self.rate_limiter.wait().await;
        let is_crypto = symbol.contains('/');

        if is_crypto {
            // Crypto: use latest bars endpoint for prev close, latest trade for last
            let snap_url = format!("{}/v1beta3/crypto/us/snapshots", DATA_BASE);
            let resp = self
                .client
                .get(&snap_url)
                .headers(self.headers())
                .query(&[("symbols", symbol)])
                .send()
                .await
                .map_err(|e| format!("Crypto snapshot failed: {e}"))?;
            if !resp.status().is_success() {
                return Err(format!("Crypto snapshot HTTP {}", resp.status()));
            }
            let json: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| format!("Crypto snapshot parse: {e}"))?;
            let snap = &json["snapshots"][symbol];
            let last = snap["latestTrade"]["p"].as_f64().unwrap_or(0.0);
            let daily_volume = snap["dailyBar"]["v"].as_f64().unwrap_or(0.0);
            let regular_close = snap["dailyBar"]["c"].as_f64().unwrap_or(0.0);
            let prev_close = snap["prevDailyBar"]["c"].as_f64().unwrap_or(0.0);
            Ok(SnapshotData {
                symbol: symbol.to_string(),
                last,
                prev_close,
                daily_volume,
                regular_close,
            })
        } else {
            // Stock/ETF: v2/stocks/{symbol}/snapshot
            let url = format!("{}/v2/stocks/{}/snapshot", DATA_BASE, symbol);
            // Use SIP feed for snapshots — includes extended hours (pre/post market) trades.
            // IEX feed only reports regular session trades, missing pre/post market entirely.
            let resp = self
                .client
                .get(&url)
                .headers(self.headers())
                .query(&[("feed", "sip")])
                .send()
                .await
                .map_err(|e| format!("Snapshot failed: {e}"))?;
            if !resp.status().is_success() {
                return Err(format!("Snapshot HTTP {}", resp.status()));
            }
            let json: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| format!("Snapshot parse: {e}"))?;
            // latestTrade.p = last trade price
            let trade_price = json["latestTrade"]["p"].as_f64().unwrap_or(0.0);
            // dailyBar.v = today's volume, dailyBar.c = today's last bar close
            let daily_volume = json["dailyBar"]["v"].as_f64().unwrap_or(0.0);
            // prevDailyBar.c = yesterday's close
            let prev_close = json["prevDailyBar"]["c"].as_f64().unwrap_or(0.0);
            // Use trade price for "last" (includes pre/post market)
            let regular_close = json["dailyBar"]["c"].as_f64().unwrap_or(0.0);
            let last = if trade_price > 0.0 {
                trade_price
            } else {
                regular_close
            };
            Ok(SnapshotData {
                symbol: symbol.to_string(),
                last,
                prev_close,
                daily_volume,
                regular_close,
            })
        }
    }

    // ── Account Activities ───────────────────────────────────────────

    /// Fetch account activities (fills, dividends, deposits, etc.)
    pub async fn get_account_activities(
        &self,
        activity_types: &str,
        limit: u32,
    ) -> Result<Vec<AccountActivity>, String> {
        // Validate activity_types: alphanumeric + comma only (prevent path traversal)
        if !activity_types.is_empty()
            && !activity_types
                .chars()
                .all(|c| c.is_alphanumeric() || c == ',' || c == '_')
        {
            return Err("Invalid activity type characters".to_string());
        }
        let url = if activity_types.is_empty() {
            format!("{}/v2/account/activities", self.base_url)
        } else {
            format!("{}/v2/account/activities/{}", self.base_url, activity_types)
        };

        let resp = self
            .client
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

        let json: Vec<serde_json::Value> = resp
            .json()
            .await
            .map_err(|e| format!("Activities parse failed: {e}"))?;

        Ok(json
            .iter()
            .map(|a| {
                let activity_type = a["activity_type"].as_str().unwrap_or("").to_string();
                let description = match activity_type.as_str() {
                    "FILL" => format!(
                        "{} {} {} @ {}",
                        a["side"].as_str().unwrap_or(""),
                        a["qty"].as_str().unwrap_or("0"),
                        a["symbol"].as_str().unwrap_or(""),
                        a["price"].as_str().unwrap_or("?")
                    ),
                    "DIV" | "DIVCGL" | "DIVCGS" | "DIVNRA" | "DIVROC" | "DIVTXEX" => format!(
                        "Dividend {} ${}",
                        a["symbol"].as_str().unwrap_or(""),
                        a["net_amount"].as_str().unwrap_or("0")
                    ),
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
                    date: a["transaction_time"]
                        .as_str()
                        .or_else(|| a["date"].as_str())
                        .unwrap_or("")
                        .to_string(),
                    description,
                }
            })
            .collect())
    }

    // ── Insider Trading (SEC Form 4) ─────────────────────────────────

    /// Fetch insider trades for a ticker via SEC EDGAR (Form 4 filings).
    /// Uses cached ticker map and shared HTTP client.
    pub async fn get_insider_trades(ticker: &str) -> Result<Vec<InsiderTrade>, String> {
        if ticker.is_empty()
            || ticker.len() > 10
            || !ticker.chars().all(|c| c.is_ascii_alphanumeric())
        {
            return Err("Invalid ticker for SEC lookup".to_string());
        }
        let client = sec_client();
        let cik = lookup_cik(ticker).await?;
        let cik_padded = format!("{:010}", cik);

        let subs_resp = client
            .get(format!(
                "https://data.sec.gov/submissions/CIK{}.json",
                cik_padded
            ))
            .header(
                "User-Agent",
                "TyphooN-Terminal/1.0 typhoon-terminal@example.invalid",
            )
            .send()
            .await
            .map_err(|e| format!("SEC submissions fetch failed: {e}"))?;

        if !subs_resp.status().is_success() {
            return Err(format!("SEC submissions: HTTP {}", subs_resp.status()));
        }

        let subs: serde_json::Value = subs_resp
            .json()
            .await
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
                if form != "4" {
                    continue;
                }
                if insider_trades.len() >= 50 {
                    break;
                }

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

    // ── Finnhub Short Interest ────────────────────────────────────

    /// Fetch FINRA short interest data from Finnhub (bi-weekly reports).
    pub async fn get_finnhub_short_interest(
        &self,
        symbol: &str,
        finnhub_key: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        if finnhub_key.is_empty() {
            return Err("Finnhub API key required".into());
        }
        let resp = sec_client()
            .get("https://finnhub.io/api/v1/stock/short-interest")
            .query(&[
                ("symbol", symbol),
                ("token", finnhub_key),
                ("from", "2025-01-01"),
                ("to", "2026-12-31"),
            ])
            .send()
            .await
            .map_err(|e| format!("Finnhub short interest failed: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("Finnhub short interest: HTTP {}", resp.status()));
        }
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Finnhub short interest parse failed: {e}"))?;
        // Finnhub returns { "data": [...], "symbol": "..." }
        match body.get("data").and_then(|d| d.as_array()) {
            Some(arr) => Ok(arr.clone()),
            None => Ok(vec![body]),
        }
    }

    // ── Alpaca Watchlists ────────────────────────────────────────────

    /// Fetch all watchlists from Alpaca.
    pub async fn get_watchlists(&self) -> Result<Vec<serde_json::Value>, String> {
        self.rate_limiter.wait().await;
        let resp = self
            .client
            .get(format!("{}/v2/watchlists", self.base_url))
            .headers(self.headers())
            .send()
            .await
            .map_err(|e| format!("Get watchlists failed: {e}"))?;

        if resp.status().as_u16() == 429 {
            self.rate_limiter.trigger_cooldown().await;
        }
        if !resp.status().is_success() {
            return Err(format!("Get watchlists: HTTP {}", resp.status()));
        }
        resp.json()
            .await
            .map_err(|e| format!("Watchlists parse failed: {e}"))
    }

    /// Create a new watchlist on Alpaca.
    pub async fn create_watchlist(
        &self,
        name: &str,
        symbols: &[String],
    ) -> Result<serde_json::Value, String> {
        self.rate_limiter.wait().await;
        let body = serde_json::json!({
            "name": name,
            "symbols": symbols,
        });
        let resp = self
            .client
            .post(format!("{}/v2/watchlists", self.base_url))
            .headers(self.headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Create watchlist failed: {e}"))?;

        if resp.status().as_u16() == 429 {
            self.rate_limiter.trigger_cooldown().await;
        }
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Create watchlist: HTTP {status} — {text}"));
        }
        resp.json()
            .await
            .map_err(|e| format!("Create watchlist parse failed: {e}"))
    }

    /// Update an existing watchlist on Alpaca (replace symbols).
    pub async fn update_watchlist(
        &self,
        id: &str,
        symbols: &[String],
    ) -> Result<serde_json::Value, String> {
        self.rate_limiter.wait().await;
        let body = serde_json::json!({
            "symbols": symbols,
        });
        let resp = self
            .client
            .put(format!("{}/v2/watchlists/{}", self.base_url, id))
            .headers(self.headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Update watchlist failed: {e}"))?;

        if resp.status().as_u16() == 429 {
            self.rate_limiter.trigger_cooldown().await;
        }
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Update watchlist: HTTP {status} — {text}"));
        }
        resp.json()
            .await
            .map_err(|e| format!("Update watchlist parse failed: {e}"))
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
        let is_crypto = trade_symbols
            .iter()
            .chain(quote_symbols.iter())
            .any(|s| s.contains('/'));
        let ws_url = if is_crypto {
            "wss://stream.data.alpaca.markets/v1beta3/crypto/us"
        } else {
            "wss://stream.data.alpaca.markets/v2/iex"
        };

        // Initial connection to validate credentials before spawning the task
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
            .send(tokio_tungstenite::tungstenite::Message::Text(
                auth_msg.to_string().into(),
            ))
            .await
            .map_err(|e| format!("WebSocket auth send failed: {e}"))?;

        // Wait for auth response
        if let Some(Ok(msg)) = read.next().await {
            tracing::debug!("WS welcome: {msg}");
        }
        if let Some(Ok(msg)) = read.next().await {
            let text = msg.to_text().unwrap_or("");
            if text.contains("\"error\"") {
                return Err(format!("WebSocket auth failed: {text}"));
            }
            tracing::debug!("WS auth response: authorized");
        }

        // Subscribe
        let sub_msg = serde_json::json!({
            "action": "subscribe",
            "trades": &trade_symbols,
            "quotes": &quote_symbols,
        });
        write
            .send(tokio_tungstenite::tungstenite::Message::Text(
                sub_msg.to_string().into(),
            ))
            .await
            .map_err(|e| format!("WebSocket subscribe failed: {e}"))?;

        // Channel for outgoing messages
        let (tx, rx) = tokio::sync::mpsc::channel::<StreamMessage>(1024);

        // Clone credentials for the reconnection task
        let api_key = self.api_key.clone();
        let secret_key = self.secret_key.clone();
        let ws_url = ws_url.to_string();

        // Spawn reader task with reconnection logic
        tokio::spawn(async move {
            let mut current_read = read;
            let mut consecutive_failures: u32 = 0;
            const MAX_RECONNECT_ATTEMPTS: u32 = 10;

            'outer: loop {
                // Read messages from the current connection
                while let Some(result) = current_read.next().await {
                    let msg = match result {
                        Ok(m) => m,
                        Err(e) => {
                            tracing::warn!("WebSocket read error: {e}");
                            break;
                        }
                    };
                    let text = match msg.to_text() {
                        Ok(t) => t.to_string(),
                        Err(_) => continue,
                    };

                    let parsed: Result<Vec<serde_json::Value>, _> = serde_json::from_str(&text);
                    if let Ok(events) = parsed {
                        for event in events {
                            let msg_type = event["T"].as_str().unwrap_or("");
                            let stream_msg = match msg_type {
                                "t" => {
                                    let price = event["p"].as_f64().unwrap_or(0.0);
                                    let size = event["s"].as_f64().unwrap_or(0.0);
                                    if price <= 0.0 {
                                        None
                                    } else {
                                        Some(StreamMessage::Trade(StreamTrade {
                                            symbol: event["S"].as_str().unwrap_or("").to_string(),
                                            price,
                                            size,
                                            timestamp: event["t"]
                                                .as_str()
                                                .unwrap_or("")
                                                .to_string(),
                                        }))
                                    }
                                }
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
                    // Successfully processed a message, reset failure counter
                    consecutive_failures = 0;
                }

                // Stream disconnected — attempt reconnection with exponential backoff
                tracing::warn!("WebSocket stream disconnected, will attempt reconnection");

                loop {
                    if consecutive_failures >= MAX_RECONNECT_ATTEMPTS {
                        tracing::error!(
                            "WebSocket reconnection failed after {MAX_RECONNECT_ATTEMPTS} consecutive attempts, giving up"
                        );
                        break 'outer;
                    }

                    let backoff_secs = std::cmp::min(1u64 << consecutive_failures, 30);
                    consecutive_failures += 1;
                    tracing::warn!(
                        "WebSocket reconnect attempt {}/{MAX_RECONNECT_ATTEMPTS} in {backoff_secs}s",
                        consecutive_failures
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(backoff_secs)).await;

                    // Reconnect
                    let ws_stream = match tokio_tungstenite::connect_async(&ws_url).await {
                        Ok((stream, _)) => stream,
                        Err(e) => {
                            tracing::warn!("WebSocket reconnect failed: {e}");
                            continue;
                        }
                    };
                    let (mut write, new_read) = ws_stream.split();

                    // Re-authenticate
                    let auth_msg = serde_json::json!({
                        "action": "auth",
                        "key": api_key.as_str(),
                        "secret": secret_key.as_str(),
                    });
                    if let Err(e) = write
                        .send(tokio_tungstenite::tungstenite::Message::Text(
                            auth_msg.to_string().into(),
                        ))
                        .await
                    {
                        tracing::warn!("WebSocket reconnect auth send failed: {e}");
                        continue;
                    }

                    // Temporarily bind so we can read welcome/auth responses
                    current_read = new_read;

                    // Read welcome
                    if let Some(Ok(msg)) = current_read.next().await {
                        tracing::debug!("WS reconnect welcome: {msg}");
                    }
                    // Read auth response
                    let auth_ok = if let Some(Ok(msg)) = current_read.next().await {
                        let text = msg.to_text().unwrap_or("");
                        if text.contains("\"error\"") {
                            tracing::warn!("WebSocket reconnect auth failed: {text}");
                            false
                        } else {
                            true
                        }
                    } else {
                        tracing::warn!("WebSocket reconnect: no auth response");
                        false
                    };

                    if !auth_ok {
                        continue;
                    }

                    // Re-subscribe
                    let sub_msg = serde_json::json!({
                        "action": "subscribe",
                        "trades": &trade_symbols,
                        "quotes": &quote_symbols,
                    });
                    if let Err(e) = write
                        .send(tokio_tungstenite::tungstenite::Message::Text(
                            sub_msg.to_string().into(),
                        ))
                        .await
                    {
                        tracing::warn!("WebSocket reconnect subscribe failed: {e}");
                        continue;
                    }

                    tracing::info!("WebSocket reconnected and re-subscribed successfully");
                    consecutive_failures = 0;
                    break; // Break inner retry loop, continue outer read loop
                }
            }
            tracing::info!("WebSocket stream ended permanently");
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

/// Snapshot data for watchlist: last price, prev close, daily volume.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotData {
    pub symbol: String,
    pub last: f64,
    pub prev_close: f64,
    pub daily_volume: f64,
    pub regular_close: f64, // Regular session close (dailyBar.c) — for extended hours change calc
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

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::{HeaderMap, HeaderValue};
    use serde_json::json;

    // ── parse_f64_field ─────────────────────────────────────────────

    #[test]
    fn parse_f64_field_from_string() {
        let j = json!({"equity": "123456.78"});
        assert!((parse_f64_field(&j, "equity") - 123456.78).abs() < 1e-10);
    }

    #[test]
    fn parse_f64_field_from_number() {
        let j = json!({"equity": 99999.50});
        assert!((parse_f64_field(&j, "equity") - 99999.50).abs() < 1e-10);
    }

    #[test]
    fn parse_f64_field_null_returns_zero() {
        let j = json!({"equity": null});
        assert_eq!(parse_f64_field(&j, "equity"), 0.0);
    }

    #[test]
    fn parse_f64_field_missing_returns_zero() {
        let j = json!({});
        assert_eq!(parse_f64_field(&j, "equity"), 0.0);
    }

    #[test]
    fn parse_f64_field_bad_string_returns_zero() {
        let j = json!({"equity": "not_a_number"});
        assert_eq!(parse_f64_field(&j, "equity"), 0.0);
    }

    // ── format_order_price ─────────────────────────────────────────────────

    #[test]
    fn round_price_stock_above_one() {
        assert_eq!(format_order_price(15.6789), "15.68");
        assert_eq!(format_order_price(100.0), "100.00");
        assert_eq!(format_order_price(1.0), "1.00");
    }

    #[test]
    fn round_price_penny_stock() {
        assert_eq!(format_order_price(0.1234), "0.1234");
        assert_eq!(format_order_price(0.01), "0.0100");
        assert_eq!(format_order_price(0.99), "0.9900");
    }

    #[test]
    fn round_price_sub_penny_crypto() {
        assert_eq!(format_order_price(0.00123456), "0.00123456");
        assert_eq!(format_order_price(0.009), "0.00900000");
    }

    #[tokio::test]
    async fn observe_rate_limit_headers_updates_bar_rpm() {
        let limiter = RateLimiter::new();
        let mut headers = HeaderMap::new();
        headers.insert("x-ratelimit-limit", HeaderValue::from_static("10000"));

        assert_eq!(
            limiter.observe_rate_limit_headers(&headers).await,
            Some(10000)
        );
        assert_eq!(limiter.requests_per_minute(), 10000);
    }

    // ── is_crypto detection (symbol.contains('/')) ──────────────────

    #[test]
    fn crypto_detection_by_slash() {
        assert!("BTC/USD".contains('/'));
        assert!("SOL/USD".contains('/'));
        assert!(!"AAPL".contains('/'));
        assert!(!"SPY".contains('/'));
    }

    // ── parse_option_symbol ─────────────────────────────────────────

    #[test]
    fn parse_option_symbol_call() {
        let (strike, opt_type, expiry) = AlpacaBroker::parse_option_symbol("AAPL240119C00150000");
        assert!((strike - 150.0).abs() < 1e-10);
        assert_eq!(opt_type, "call");
        assert_eq!(expiry, "2024-01-19");
    }

    #[test]
    fn parse_option_symbol_put() {
        let (strike, opt_type, expiry) = AlpacaBroker::parse_option_symbol("TSLA250221P00200000");
        assert!((strike - 200.0).abs() < 1e-10);
        assert_eq!(opt_type, "put");
        assert_eq!(expiry, "2025-02-21");
    }

    #[test]
    fn parse_option_symbol_fractional_strike() {
        // Strike 72.50 = 00072500
        let (strike, opt_type, _) = AlpacaBroker::parse_option_symbol("INTC240315C00072500");
        assert!((strike - 72.5).abs() < 1e-10);
        assert_eq!(opt_type, "call");
    }

    #[test]
    fn parse_option_symbol_too_short() {
        let (strike, opt_type, expiry) = AlpacaBroker::parse_option_symbol("SHORT");
        assert_eq!(strike, 0.0);
        assert_eq!(opt_type, "unknown");
        assert!(expiry.is_empty());
    }

    #[test]
    fn targeted_lookback_is_wider_than_incremental_for_equity_minute_sync() {
        let incremental =
            lookback_days_for_request(false, "1Min", 50_000, BarsLookbackMode::Incremental);
        let targeted = lookback_days_for_request(false, "1Min", 50_000, BarsLookbackMode::Targeted);
        assert_eq!(incremental, 7);
        assert!(targeted > incremental);
    }

    #[test]
    fn targeted_lookback_scales_for_equity_hour_sync() {
        let targeted =
            lookback_days_for_request(false, "1Hour", 30_000, BarsLookbackMode::Targeted);
        assert!(targeted >= 6_000);
    }

    #[test]
    fn detects_sip_bar_entitlement_failures() {
        assert!(AlpacaBroker::is_sip_bar_entitlement_failure(
            reqwest::StatusCode::FORBIDDEN,
            "subscription does not permit querying SIP data"
        ));
        assert!(AlpacaBroker::is_sip_bar_entitlement_failure(
            reqwest::StatusCode::UNPROCESSABLE_ENTITY,
            "SIP feed requires plan upgrade"
        ));
    }

    #[test]
    fn ignores_non_entitlement_bar_failures() {
        assert!(!AlpacaBroker::is_sip_bar_entitlement_failure(
            reqwest::StatusCode::NOT_FOUND,
            "not found"
        ));
        assert!(!AlpacaBroker::is_sip_bar_entitlement_failure(
            reqwest::StatusCode::FORBIDDEN,
            "market data temporarily unavailable"
        ));
        assert!(!AlpacaBroker::is_sip_bar_entitlement_failure(
            reqwest::StatusCode::FORBIDDEN,
            "subscription does not permit querying IEX data"
        ));
    }

    // ── parse_bars (mock JSON) ──────────────────────────────────────

    #[test]
    fn parse_stock_bars_by_symbol_batch_valid() {
        let json = json!({
            "bars": {
                "AAPL": [{"t":"2024-01-02T00:00:00Z","o":100.0,"h":110.0,"l":99.0,"c":105.0,"v":1000.0}],
                "MSFT": [{"t":"2024-01-02T00:00:00Z","o":200.0,"h":220.0,"l":190.0,"c":210.0,"v":2000.0}]
            }
        });
        let symbols = vec![
            "AAPL".to_string(),
            "MSFT".to_string(),
            "MISSING".to_string(),
        ];
        let bars = AlpacaBroker::parse_stock_bars_by_symbol(&json, &symbols);
        assert_eq!(bars["AAPL"].len(), 1);
        assert_eq!(bars["AAPL"][0].close, 105.0);
        assert_eq!(bars["MSFT"].len(), 1);
        assert!(!bars.contains_key("MISSING"));
    }

    #[test]
    fn parse_bars_stock_valid() {
        let json = json!({
            "bars": [
                {"t": "2024-01-02T05:00:00Z", "o": 100.0, "h": 105.0, "l": 99.0, "c": 103.0, "v": 50000.0},
                {"t": "2024-01-03T05:00:00Z", "o": 103.0, "h": 107.0, "l": 102.0, "c": 106.0, "v": 60000.0},
            ]
        });
        let bars = AlpacaBroker::parse_bars(&json, "AAPL", false);
        assert_eq!(bars.len(), 2);
        assert_eq!(bars[0].open, 100.0);
        assert_eq!(bars[0].high, 105.0);
        assert_eq!(bars[0].low, 99.0);
        assert_eq!(bars[0].close, 103.0);
        assert_eq!(bars[0].volume, 50000.0);
    }

    #[test]
    fn parse_bars_crypto_nested_by_symbol() {
        let json = json!({
            "bars": {
                "BTC/USD": [
                    {"t": "2024-01-02T00:00:00Z", "o": 42000.0, "h": 43000.0, "l": 41000.0, "c": 42500.0, "v": 100.0},
                ]
            }
        });
        let bars = AlpacaBroker::parse_bars(&json, "BTC/USD", true);
        assert_eq!(bars.len(), 1);
        assert_eq!(bars[0].open, 42000.0);
    }

    #[test]
    fn parse_bars_rejects_zero_open() {
        let json = json!({
            "bars": [
                {"t": "2024-01-02T05:00:00Z", "o": 0.0, "h": 5.0, "l": 0.0, "c": 4.0, "v": 100.0},
            ]
        });
        let bars = AlpacaBroker::parse_bars(&json, "BAD", false);
        assert_eq!(bars.len(), 0);
    }

    #[test]
    fn parse_bars_rejects_missing_timestamp() {
        let json = json!({
            "bars": [
                {"t": "", "o": 10.0, "h": 12.0, "l": 9.0, "c": 11.0, "v": 100.0},
            ]
        });
        let bars = AlpacaBroker::parse_bars(&json, "X", false);
        assert_eq!(bars.len(), 0);
    }

    #[test]
    fn parse_bars_fixes_ohlc_consistency() {
        // h < o should be corrected: true_high = max(o,h,l,c)
        let json = json!({
            "bars": [
                {"t": "2024-01-02T05:00:00Z", "o": 110.0, "h": 105.0, "l": 99.0, "c": 108.0, "v": 100.0},
            ]
        });
        let bars = AlpacaBroker::parse_bars(&json, "FIX", false);
        assert_eq!(bars.len(), 1);
        assert_eq!(bars[0].high, 110.0); // corrected to max(110, 105, 99, 108)
        assert_eq!(bars[0].low, 99.0);
    }

    #[test]
    fn parse_bars_empty_array() {
        let json = json!({"bars": []});
        let bars = AlpacaBroker::parse_bars(&json, "EMPTY", false);
        assert!(bars.is_empty());
    }

    // ── parse_order_info (mock JSON) ────────────────────────────────

    #[test]
    fn parse_order_info_full() {
        let j = json!({
            "id": "abc-123",
            "symbol": "AAPL",
            "qty": "10",
            "filled_qty": "10",
            "side": "buy",
            "type": "limit",
            "order_class": "bracket",
            "status": "filled",
            "limit_price": "150.00",
            "stop_price": null,
            "trail_price": null,
            "trail_percent": null,
            "created_at": "2024-01-02T10:00:00Z",
            "filled_at": "2024-01-02T10:00:05Z",
            "filled_avg_price": "149.98",
            "legs": null,
        });
        let order = AlpacaBroker::parse_order_info(&j);
        assert_eq!(order.id, "abc-123");
        assert_eq!(order.symbol, "AAPL");
        assert_eq!(order.qty, "10");
        assert_eq!(order.side, "buy");
        assert_eq!(order.order_type, "limit");
        assert_eq!(order.order_class, Some("bracket".to_string()));
        assert_eq!(order.status, "filled");
        assert_eq!(order.limit_price, Some("150.00".to_string()));
        assert_eq!(order.filled_avg_price, Some("149.98".to_string()));
    }

    #[test]
    fn parse_order_info_with_bracket_legs() {
        let j = json!({
            "id": "parent-1",
            "symbol": "SPY",
            "qty": "5",
            "filled_qty": "5",
            "side": "buy",
            "type": "market",
            "order_class": "bracket",
            "status": "filled",
            "limit_price": null,
            "stop_price": null,
            "trail_price": null,
            "trail_percent": null,
            "created_at": "2024-01-02T10:00:00Z",
            "filled_at": "2024-01-02T10:00:01Z",
            "filled_avg_price": "470.00",
            "legs": [
                {
                    "id": "tp-leg",
                    "symbol": "SPY",
                    "qty": "5",
                    "filled_qty": "0",
                    "side": "sell",
                    "type": "limit",
                    "status": "new",
                    "limit_price": "480.00",
                    "stop_price": null,
                    "trail_price": null,
                    "trail_percent": null,
                    "created_at": "2024-01-02T10:00:00Z",
                    "filled_at": null,
                    "filled_avg_price": null,
                },
                {
                    "id": "sl-leg",
                    "symbol": "SPY",
                    "qty": "5",
                    "filled_qty": "0",
                    "side": "sell",
                    "type": "stop",
                    "status": "held",
                    "limit_price": null,
                    "stop_price": "460.00",
                    "trail_price": null,
                    "trail_percent": null,
                    "created_at": "2024-01-02T10:00:00Z",
                    "filled_at": null,
                    "filled_avg_price": null,
                },
            ],
        });
        let order = AlpacaBroker::parse_order_info(&j);
        assert_eq!(order.id, "parent-1");
        let legs = order.legs.expect("should have legs");
        assert_eq!(legs.len(), 2);
        assert_eq!(legs[0].id, "tp-leg");
        assert_eq!(legs[0].limit_price, Some("480.00".to_string()));
        assert_eq!(legs[1].id, "sl-leg");
        assert_eq!(legs[1].stop_price, Some("460.00".to_string()));
    }

    #[test]
    fn collect_cancellable_order_ids_for_symbol_skips_filled_parent_and_keeps_open_legs() {
        let parent = OrderInfo {
            id: "parent-1".to_string(),
            symbol: "SPY".to_string(),
            qty: "5".to_string(),
            filled_qty: "5".to_string(),
            side: "buy".to_string(),
            order_type: "market".to_string(),
            order_class: Some("bracket".to_string()),
            status: "filled".to_string(),
            limit_price: None,
            stop_price: None,
            trail_price: None,
            trail_percent: None,
            created_at: "2024-01-02T10:00:00Z".to_string(),
            filled_at: Some("2024-01-02T10:00:01Z".to_string()),
            filled_avg_price: Some("470.00".to_string()),
            legs: Some(vec![
                OrderInfo {
                    id: "tp-leg".to_string(),
                    symbol: "SPY".to_string(),
                    qty: "5".to_string(),
                    filled_qty: "0".to_string(),
                    side: "sell".to_string(),
                    order_type: "limit".to_string(),
                    order_class: None,
                    status: "new".to_string(),
                    limit_price: Some("480.00".to_string()),
                    stop_price: None,
                    trail_price: None,
                    trail_percent: None,
                    created_at: "2024-01-02T10:00:00Z".to_string(),
                    filled_at: None,
                    filled_avg_price: None,
                    legs: None,
                },
                OrderInfo {
                    id: "sl-leg".to_string(),
                    symbol: "SPY".to_string(),
                    qty: "5".to_string(),
                    filled_qty: "0".to_string(),
                    side: "sell".to_string(),
                    order_type: "stop".to_string(),
                    order_class: None,
                    status: "held".to_string(),
                    limit_price: None,
                    stop_price: Some("460.00".to_string()),
                    trail_price: None,
                    trail_percent: None,
                    created_at: "2024-01-02T10:00:00Z".to_string(),
                    filled_at: None,
                    filled_avg_price: None,
                    legs: None,
                },
            ]),
        };

        let ids = AlpacaBroker::collect_cancellable_order_ids_for_symbol(&[parent], "SPY");
        assert_eq!(ids, vec!["tp-leg".to_string(), "sl-leg".to_string()]);
    }

    #[test]
    fn collect_cancellable_order_ids_for_symbol_normalizes_crypto_symbol() {
        let order = OrderInfo {
            id: "crypto-exit".to_string(),
            symbol: "BTCUSD".to_string(),
            qty: "0.2".to_string(),
            filled_qty: "0".to_string(),
            side: "sell".to_string(),
            order_type: "limit".to_string(),
            order_class: Some("oco".to_string()),
            status: "new".to_string(),
            limit_price: Some("70000".to_string()),
            stop_price: None,
            trail_price: None,
            trail_percent: None,
            created_at: "2024-01-02T10:00:00Z".to_string(),
            filled_at: None,
            filled_avg_price: None,
            legs: None,
        };

        let ids = AlpacaBroker::collect_cancellable_order_ids_for_symbol(&[order], "BTC/USD");
        assert_eq!(ids, vec!["crypto-exit".to_string()]);
    }

    #[test]
    fn collect_cancellable_exit_order_ids_for_symbol_filters_by_exit_side() {
        let sell_exit = OrderInfo {
            id: "sell-exit".to_string(),
            symbol: "SPY".to_string(),
            qty: "5".to_string(),
            filled_qty: "0".to_string(),
            side: "sell".to_string(),
            order_type: "limit".to_string(),
            order_class: None,
            status: "new".to_string(),
            limit_price: Some("500.00".to_string()),
            stop_price: None,
            trail_price: None,
            trail_percent: None,
            created_at: "2024-01-02T10:00:00Z".to_string(),
            filled_at: None,
            filled_avg_price: None,
            legs: None,
        };
        let buy_entry = OrderInfo {
            id: "buy-entry".to_string(),
            symbol: "SPY".to_string(),
            qty: "5".to_string(),
            filled_qty: "0".to_string(),
            side: "buy".to_string(),
            order_type: "limit".to_string(),
            order_class: None,
            status: "new".to_string(),
            limit_price: Some("470.00".to_string()),
            stop_price: None,
            trail_price: None,
            trail_percent: None,
            created_at: "2024-01-02T10:01:00Z".to_string(),
            filled_at: None,
            filled_avg_price: None,
            legs: None,
        };

        let ids = AlpacaBroker::collect_cancellable_exit_order_ids_for_symbol(
            &[sell_exit, buy_entry],
            "SPY",
            "sell",
        );
        assert_eq!(ids, vec!["sell-exit".to_string()]);
    }

    // ── AccountInfo parsing from mock JSON ──────────────────────────

    #[test]
    fn parse_account_json_string_fields() {
        // Alpaca returns most numeric fields as strings
        let j = json!({
            "equity": "100000.00",
            "cash": "50000.00",
            "buying_power": "200000.00",
            "portfolio_value": "100000.00",
            "initial_margin": "25000.00",
            "maintenance_margin": "12500.00",
            "currency": "USD",
            "pattern_day_trader": false,
            "trading_blocked": false,
            "last_equity": "99500.00",
        });
        let info = AccountInfo {
            equity: parse_f64_field(&j, "equity"),
            cash: parse_f64_field(&j, "cash"),
            buying_power: parse_f64_field(&j, "buying_power"),
            portfolio_value: parse_f64_field(&j, "portfolio_value"),
            initial_margin: parse_f64_field(&j, "initial_margin"),
            maintenance_margin: parse_f64_field(&j, "maintenance_margin"),
            currency: j["currency"].as_str().unwrap_or("USD").to_string(),
            pattern_day_trader: j["pattern_day_trader"].as_bool().unwrap_or(false),
            trading_blocked: j["trading_blocked"].as_bool().unwrap_or(false),
            balance: parse_f64_field(&j, "last_equity"),
        };
        assert!((info.equity - 100_000.0).abs() < 1e-10);
        assert!((info.cash - 50_000.0).abs() < 1e-10);
        assert!((info.buying_power - 200_000.0).abs() < 1e-10);
        assert!((info.balance - 99_500.0).abs() < 1e-10);
        assert_eq!(info.currency, "USD");
        assert!(!info.pattern_day_trader);
    }

    // ── SnapshotData struct ─────────────────────────────────────────

    #[test]
    fn snapshot_data_construction() {
        let snap = SnapshotData {
            symbol: "AAPL".to_string(),
            last: 178.50,
            prev_close: 177.00,
            daily_volume: 45_000_000.0,
            regular_close: 178.25,
        };
        assert_eq!(snap.symbol, "AAPL");
        // Change % = (last - prev_close) / prev_close
        let change_pct = (snap.last - snap.prev_close) / snap.prev_close * 100.0;
        assert!((change_pct - 0.847457).abs() < 0.001);
    }

    // ── aggregate_weekly_to_monthly ─────────────────────────────────

    #[test]
    fn aggregate_weekly_to_monthly_basic() {
        let weekly = vec![
            Bar {
                timestamp: "2024-01-01T00:00:00Z".into(),
                open: 100.0,
                high: 110.0,
                low: 95.0,
                close: 105.0,
                volume: 1000.0,
            },
            Bar {
                timestamp: "2024-01-08T00:00:00Z".into(),
                open: 105.0,
                high: 112.0,
                low: 100.0,
                close: 108.0,
                volume: 1200.0,
            },
            Bar {
                timestamp: "2024-01-15T00:00:00Z".into(),
                open: 108.0,
                high: 115.0,
                low: 106.0,
                close: 113.0,
                volume: 900.0,
            },
            Bar {
                timestamp: "2024-01-22T00:00:00Z".into(),
                open: 113.0,
                high: 118.0,
                low: 110.0,
                close: 116.0,
                volume: 1100.0,
            },
            // February
            Bar {
                timestamp: "2024-02-05T00:00:00Z".into(),
                open: 116.0,
                high: 120.0,
                low: 114.0,
                close: 119.0,
                volume: 800.0,
            },
        ];
        let monthly = AlpacaBroker::aggregate_weekly_to_monthly(&weekly);
        assert_eq!(monthly.len(), 2);
        // January
        assert_eq!(monthly[0].open, 100.0);
        assert_eq!(monthly[0].high, 118.0);
        assert_eq!(monthly[0].low, 95.0);
        assert_eq!(monthly[0].close, 116.0);
        assert_eq!(monthly[0].volume, 4200.0);
        // February
        assert_eq!(monthly[1].open, 116.0);
        assert_eq!(monthly[1].close, 119.0);
    }

    #[test]
    fn aggregate_weekly_to_monthly_empty() {
        let monthly = AlpacaBroker::aggregate_weekly_to_monthly(&[]);
        assert!(monthly.is_empty());
    }

    // ── OCO order body construction ──

    #[test]
    fn oco_order_body_has_correct_class() {
        // Verify the JSON body shape for OCO orders matches Alpaca's spec
        let body = serde_json::json!({
            "symbol": "SPY",
            "qty": "10",
            "side": "sell",
            "type": "limit",
            "time_in_force": "gtc",
            "order_class": "oco",
            "take_profit": { "limit_price": "500.00" },
            "stop_loss": { "stop_price": "450.00" },
        });
        assert_eq!(body["order_class"], "oco");
        assert_eq!(body["type"], "limit");
        assert_eq!(body["take_profit"]["limit_price"], "500.00");
        assert_eq!(body["stop_loss"]["stop_price"], "450.00");
    }

    #[test]
    fn oco_order_body_with_stop_limit() {
        let body = serde_json::json!({
            "symbol": "AAPL",
            "qty": "5",
            "side": "sell",
            "type": "limit",
            "time_in_force": "gtc",
            "order_class": "oco",
            "take_profit": { "limit_price": "200.00" },
            "stop_loss": { "stop_price": "170.00", "limit_price": "169.50" },
        });
        assert_eq!(body["stop_loss"]["limit_price"], "169.50");
    }

    #[test]
    fn format_order_price_rounds_correctly() {
        assert_eq!(format_order_price(100.123456), "100.12"); // $1+ → 2 decimals
        assert_eq!(format_order_price(0.05), "0.0500"); // $0.01-$0.99 → 4 decimals
        assert_eq!(format_order_price(0.00345), "0.00345000"); // sub-penny → 8 decimals
        assert_eq!(format_order_price(1500.0), "1500.00");
    }
}
