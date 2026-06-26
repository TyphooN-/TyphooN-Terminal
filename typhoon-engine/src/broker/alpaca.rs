//! Alpaca broker interface.
//!
//! Wraps Alpaca REST API.
//! Provides Alpaca REST trading operations: open, close, partial close by
//! quantity/percentage where the API supports it, modify, and account info.

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
pub const DEFAULT_BAR_REQUESTS_PER_MINUTE: u32 = 200;

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

fn parse_f64_value(value: &serde_json::Value) -> f64 {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|s| s.parse().ok()))
        .unwrap_or(0.0)
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

fn alpaca_error_message(json: &serde_json::Value) -> Option<String> {
    json["message"]
        .as_str()
        .or_else(|| json["error"].as_str())
        .filter(|msg| !msg.trim().is_empty())
        .map(|msg| msg.to_string())
}

fn string_or_number(value: &serde_json::Value, default: &str) -> String {
    value
        .as_str()
        .map(|s| s.to_string())
        .or_else(|| value.as_f64().map(|n| n.to_string()))
        .unwrap_or_else(|| default.to_string())
}

fn optional_string_or_number(value: &serde_json::Value) -> Option<String> {
    value
        .as_str()
        .map(|s| s.to_string())
        .or_else(|| value.as_f64().map(|n| n.to_string()))
}

fn optional_f64_value(value: &serde_json::Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|s| s.parse().ok()))
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
        let now = std::time::Instant::now();
        let until = std::time::Instant::now() + std::time::Duration::from_secs(60);
        let already_cooling_down = cooldown.is_some_and(|existing| existing > now);
        *cooldown = Some(match *cooldown {
            Some(existing) if existing > until => existing,
            _ => until,
        });
        // Double adaptive interval on 429 (capped at 5s)
        let base_interval_ms = *self.base_interval_ms.lock().await;
        let mut adaptive = self.adaptive_ms.lock().await;
        *adaptive = (*adaptive * 2).min(5000).max(base_interval_ms);
        if already_cooling_down {
            tracing::debug!(
                "Rate limit hit while cooldown already active — keeping cooldown (adaptive interval: {}ms)",
                *adaptive
            );
        } else {
            tracing::warn!(
                "Rate limit hit — cooling down for 60s (adaptive interval: {}ms)",
                *adaptive
            );
        }
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
        // Raise the floor without discarding an active 429 backoff. Clobbering
        // `adaptive_ms` here let an observed `x-ratelimit-limit` header reset the
        // backoff to base on the very next response, so the interval bounced
        // (630↔1260ms in the field log) and the AIMD throttle never actually bit
        // — driving repeat 429 storms. Keep the larger of the two; `report_latency`
        // still recovers it gradually once responses are healthy.
        let mut adaptive = self.adaptive_ms.lock().await;
        *adaptive = (*adaptive).max(interval_ms);
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
    /// Alpaca's previous/last close equity (`last_equity`). This is stale by
    /// design during the current session; use it for day-change display only,
    /// not as current balance/risk capital.
    pub last_equity: f64,
    /// Legacy alias for `last_equity`; kept for cached account JSON compatibility.
    pub balance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionInfo {
    pub symbol: String,
    pub qty: f64,
    /// Alpaca `qty_available`: position quantity not locked by open orders.
    /// Older cached position snapshots do not have this field.
    #[serde(default)]
    pub qty_available: f64,
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

    async fn json_or_error(
        resp: reqwest::Response,
        context: &str,
    ) -> Result<serde_json::Value, String> {
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| format!("{context} body read failed: {e}"))?;
        let json: serde_json::Value = match serde_json::from_str(&text) {
            Ok(json) => json,
            Err(e) if !status.is_success() => {
                let snippet: String = text.chars().take(240).collect();
                return Err(format!("{context} failed: HTTP {status}: {snippet}"));
            }
            Err(e) => return Err(format!("{context} parse failed: {e}")),
        };
        if !status.is_success() {
            if let Some(msg) = alpaca_error_message(&json) {
                return Err(format!("{context} rejected: {msg}"));
            }
            return Err(format!("{context} failed: HTTP {status}"));
        }
        if let Some(msg) = alpaca_error_message(&json) {
            return Err(format!("{context} rejected: {msg}"));
        }
        Ok(json)
    }

    async fn order_response(resp: reqwest::Response, context: &str) -> Result<OrderResult, String> {
        let json = Self::json_or_error(resp, context).await?;
        Ok(Self::parse_order_result(&json))
    }

    fn parse_order_result(json: &serde_json::Value) -> OrderResult {
        OrderResult {
            id: json["id"].as_str().unwrap_or("").to_string(),
            symbol: json["symbol"].as_str().unwrap_or("").to_string(),
            qty: string_or_number(&json["qty"], "0"),
            side: json["side"].as_str().unwrap_or("").to_string(),
            status: json["status"].as_str().unwrap_or("").to_string(),
        }
    }

    fn normalize_order_side(side: &str) -> Result<&'static str, String> {
        match side.trim().to_ascii_lowercase().as_str() {
            "buy" => Ok("buy"),
            "sell" => Ok("sell"),
            other => Err(format!("Order rejected: invalid side '{other}'")),
        }
    }

    fn require_symbol(symbol: &str, context: &str) -> Result<(), String> {
        if symbol.trim().is_empty() {
            return Err(format!("{context} rejected: symbol is required"));
        }
        Ok(())
    }

    fn require_nonblank(value: &str, context: &str, field: &str) -> Result<(), String> {
        if value.trim().is_empty() {
            return Err(format!("{context} rejected: {field} is required"));
        }
        Ok(())
    }

    fn require_positive_qty(qty: f64, context: &str) -> Result<(), String> {
        if !qty.is_finite() || qty <= 0.0 {
            return Err(format!("{context} rejected: qty must be positive"));
        }
        Ok(())
    }

    fn require_positive_price(price: f64, context: &str, field: &str) -> Result<(), String> {
        if !price.is_finite() || price <= 0.0 {
            return Err(format!("{context} rejected: {field} must be positive"));
        }
        Ok(())
    }

    fn normalize_time_in_force(tif: &str) -> Result<&'static str, String> {
        match tif.trim().to_ascii_lowercase().as_str() {
            "day" => Ok("day"),
            "gtc" => Ok("gtc"),
            "opg" => Ok("opg"),
            "cls" => Ok("cls"),
            "ioc" => Ok("ioc"),
            "fok" => Ok("fok"),
            other => Err(format!("Order rejected: invalid time_in_force '{other}'")),
        }
    }

    fn market_order_time_in_force(symbol: &str) -> &'static str {
        if symbol.contains('/') { "gtc" } else { "day" }
    }

    fn advanced_order_time_in_force(_symbol: &str) -> &'static str {
        // Alpaca's bracket/OCO docs allow day/gtc and examples use GTC.
        // Prefer GTC for persistent risk-management order groups.
        "gtc"
    }

    fn market_order_body(symbol: &str, qty: f64, side: &str) -> Result<serde_json::Value, String> {
        Self::require_symbol(symbol, "Market order")?;
        Self::require_positive_qty(qty, "Market order")?;
        let side = Self::normalize_order_side(side)?;
        Ok(serde_json::json!({
            "symbol": symbol,
            "qty": qty.to_string(),
            "side": side,
            "type": "market",
            "time_in_force": Self::market_order_time_in_force(symbol),
        }))
    }

    fn market_notional_order_body(
        symbol: &str,
        notional: f64,
        side: &str,
    ) -> Result<serde_json::Value, String> {
        Self::require_symbol(symbol, "Market notional order")?;
        if !notional.is_finite() || notional <= 0.0 {
            return Err("Market notional order rejected: notional must be positive".into());
        }
        let side = Self::normalize_order_side(side)?;
        Ok(serde_json::json!({
            "symbol": symbol,
            "notional": format_order_price(notional),
            "side": side,
            "type": "market",
            "time_in_force": "day",
        }))
    }

    fn limit_order_body(
        symbol: &str,
        qty: f64,
        side: &str,
        limit_price: f64,
        tif: &str,
    ) -> Result<serde_json::Value, String> {
        Self::require_symbol(symbol, "Limit order")?;
        Self::require_positive_qty(qty, "Limit order")?;
        Self::require_positive_price(limit_price, "Limit order", "limit_price")?;
        let side = Self::normalize_order_side(side)?;
        let tif = Self::normalize_time_in_force(tif)?;
        Ok(serde_json::json!({
            "symbol": symbol,
            "qty": qty.to_string(),
            "side": side,
            "type": "limit",
            "limit_price": format_order_price(limit_price),
            "time_in_force": tif,
        }))
    }

    fn stop_order_body(
        symbol: &str,
        qty: f64,
        side: &str,
        stop_price: f64,
        tif: &str,
    ) -> Result<serde_json::Value, String> {
        Self::require_symbol(symbol, "Stop order")?;
        Self::require_positive_qty(qty, "Stop order")?;
        Self::require_positive_price(stop_price, "Stop order", "stop_price")?;
        let side = Self::normalize_order_side(side)?;
        let tif = Self::normalize_time_in_force(tif)?;
        Ok(serde_json::json!({
            "symbol": symbol,
            "qty": qty.to_string(),
            "side": side,
            "type": "stop",
            "stop_price": format_order_price(stop_price),
            "time_in_force": tif,
        }))
    }

    fn stop_limit_order_body(
        symbol: &str,
        qty: f64,
        side: &str,
        stop_price: f64,
        limit_price: f64,
        tif: &str,
    ) -> Result<serde_json::Value, String> {
        Self::require_symbol(symbol, "Stop-limit order")?;
        Self::require_positive_qty(qty, "Stop-limit order")?;
        Self::require_positive_price(stop_price, "Stop-limit order", "stop_price")?;
        Self::require_positive_price(limit_price, "Stop-limit order", "limit_price")?;
        let side = Self::normalize_order_side(side)?;
        let tif = Self::normalize_time_in_force(tif)?;
        Ok(serde_json::json!({
            "symbol": symbol,
            "qty": qty.to_string(),
            "side": side,
            "type": "stop_limit",
            "stop_price": format_order_price(stop_price),
            "limit_price": format_order_price(limit_price),
            "time_in_force": tif,
        }))
    }

    fn trailing_stop_order_body(
        symbol: &str,
        qty: f64,
        side: &str,
        trail_price: Option<f64>,
        trail_percent: Option<f64>,
        tif: &str,
    ) -> Result<serde_json::Value, String> {
        Self::require_symbol(symbol, "Trailing stop order")?;
        Self::require_positive_qty(qty, "Trailing stop order")?;
        let side = Self::normalize_order_side(side)?;
        let tif = Self::normalize_time_in_force(tif)?;
        let mut body = serde_json::json!({
            "symbol": symbol,
            "qty": qty.to_string(),
            "side": side,
            "type": "trailing_stop",
            "time_in_force": tif,
        });
        match (trail_price, trail_percent) {
            (Some(_), Some(_)) => {
                return Err(
                    "Trailing stop order rejected: choose trail_price or trail_percent, not both"
                        .into(),
                );
            }
            (None, None) => {
                return Err(
                    "Trailing stop order rejected: trail_price or trail_percent is required".into(),
                );
            }
            (Some(price), None) => {
                Self::require_positive_price(price, "Trailing stop order", "trail_price")?;
                body["trail_price"] = serde_json::json!(format_order_price(price));
            }
            (None, Some(percent)) => {
                Self::require_positive_price(percent, "Trailing stop order", "trail_percent")?;
                body["trail_percent"] = serde_json::json!(format_order_price(percent));
            }
        }
        Ok(body)
    }

    fn validate_exit_price_relationship(
        side: &str,
        tp_price: f64,
        sl_price: f64,
        context: &str,
    ) -> Result<(), String> {
        let valid = if side == "buy" {
            tp_price < sl_price
        } else {
            tp_price > sl_price
        };
        if !valid {
            return Err(format!(
                "{context} rejected: take-profit must be {} stop-loss for {side} exits",
                if side == "buy" { "below" } else { "above" }
            ));
        }
        Ok(())
    }

    fn bracket_order_body(
        symbol: &str,
        qty: f64,
        side: &str,
        tp_price: f64,
        sl_price: f64,
    ) -> Result<serde_json::Value, String> {
        Self::require_symbol(symbol, "Bracket order")?;
        Self::require_positive_qty(qty, "Bracket order")?;
        Self::require_positive_price(tp_price, "Bracket order", "take_profit.limit_price")?;
        Self::require_positive_price(sl_price, "Bracket order", "stop_loss.stop_price")?;
        let side = Self::normalize_order_side(side)?;
        let exit_side = if side == "buy" { "sell" } else { "buy" };
        Self::validate_exit_price_relationship(exit_side, tp_price, sl_price, "Bracket order")?;
        Ok(serde_json::json!({
            "symbol": symbol,
            "qty": qty.to_string(),
            "side": side,
            "type": "market",
            "time_in_force": Self::advanced_order_time_in_force(symbol),
            "order_class": "bracket",
            "take_profit": { "limit_price": format_order_price(tp_price) },
            "stop_loss": { "stop_price": format_order_price(sl_price) },
        }))
    }

    fn oco_order_body(
        symbol: &str,
        qty: f64,
        side: &str,
        tp_price: f64,
        sl_price: f64,
        sl_limit: Option<f64>,
    ) -> Result<serde_json::Value, String> {
        Self::require_symbol(symbol, "OCO order")?;
        Self::require_positive_qty(qty, "OCO order")?;
        Self::require_positive_price(tp_price, "OCO order", "take_profit.limit_price")?;
        Self::require_positive_price(sl_price, "OCO order", "stop_loss.stop_price")?;
        let side = Self::normalize_order_side(side)?;
        Self::validate_exit_price_relationship(side, tp_price, sl_price, "OCO order")?;
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
            Self::require_positive_price(sl_lim, "OCO order", "stop_loss.limit_price")?;
            body["stop_loss"]["limit_price"] = serde_json::json!(format_order_price(sl_lim));
        }
        Ok(body)
    }

    fn modify_order_body(
        qty: Option<f64>,
        limit_price: Option<f64>,
        stop_price: Option<f64>,
        trail: Option<f64>,
    ) -> Result<serde_json::Value, String> {
        let mut body = serde_json::Map::new();
        if let Some(q) = qty {
            Self::require_positive_qty(q, "Modify order")?;
            body.insert("qty".into(), serde_json::json!(q.to_string()));
        }
        if let Some(lp) = limit_price {
            Self::require_positive_price(lp, "Modify order", "limit_price")?;
            body.insert(
                "limit_price".into(),
                serde_json::json!(format_order_price(lp)),
            );
        }
        if let Some(sp) = stop_price {
            Self::require_positive_price(sp, "Modify order", "stop_price")?;
            body.insert(
                "stop_price".into(),
                serde_json::json!(format_order_price(sp)),
            );
        }
        if let Some(t) = trail {
            Self::require_positive_price(t, "Modify order", "trail")?;
            body.insert("trail".into(), serde_json::json!(t.to_string()));
        }
        if body.is_empty() {
            return Err("Modify order rejected: no changes provided".into());
        }
        Ok(serde_json::Value::Object(body))
    }

    fn close_all_positions_failures(json: &serde_json::Value) -> Vec<String> {
        let Some(rows) = json.as_array() else {
            return Vec::new();
        };
        rows.iter()
            .filter_map(|row| {
                let status = row["status"].as_u64().unwrap_or(0);
                if status < 400 {
                    return None;
                }
                let symbol = row["symbol"]
                    .as_str()
                    .or_else(|| row["body"]["symbol"].as_str())
                    .unwrap_or("unknown");
                let message = alpaca_error_message(&row["body"])
                    .or_else(|| alpaca_error_message(row))
                    .unwrap_or_else(|| format!("HTTP {status}"));
                Some(format!("{symbol}: {message}"))
            })
            .collect()
    }

    fn normalize_order_query_status(status: &str) -> Result<&'static str, String> {
        match status.trim().to_ascii_lowercase().as_str() {
            "" | "open" => Ok("open"),
            "closed" => Ok("closed"),
            "all" => Ok("all"),
            other => Err(format!("Orders rejected: invalid status '{other}'")),
        }
    }

    fn normalize_order_query_limit(limit: u32) -> u32 {
        limit.clamp(1, 500)
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

        let json = Self::json_or_error(resp, "Account").await?;

        let last_equity = parse_f64_field(&json, "last_equity");
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
            last_equity,
            balance: last_equity,
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

        let json = Self::json_or_error(resp, "Positions")
            .await?
            .as_array()
            .cloned()
            .ok_or_else(|| "Positions parse failed: expected array".to_string())?;

        Ok(json
            .iter()
            .map(|p| PositionInfo {
                symbol: p["symbol"].as_str().unwrap_or("").to_string(),
                qty: parse_f64_field(p, "qty"),
                qty_available: parse_f64_field(p, "qty_available"),
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
        let body = Self::market_order_body(symbol, qty, side)?;
        self.submit_order(&body).await
    }

    /// Place a market order by dollar notional. Alpaca documents `notional` as
    /// mutually exclusive with `qty`, and limited to market/day orders.
    pub async fn market_order_notional(
        &self,
        symbol: &str,
        notional: f64,
        side: &str,
    ) -> Result<OrderResult, String> {
        let body = Self::market_notional_order_body(symbol, notional, side)?;
        self.submit_order(&body).await
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
        let body = Self::limit_order_body(symbol, qty, side, limit_price, tif)?;
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
        let body = Self::stop_order_body(symbol, qty, side, stop_price, tif)?;
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
        let body = Self::stop_limit_order_body(symbol, qty, side, stop_price, limit_price, tif)?;
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
        let body =
            Self::trailing_stop_order_body(symbol, qty, side, trail_price, trail_percent, tif)?;
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
        let body = Self::bracket_order_body(symbol, qty, side, tp_price, sl_price)?;
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
        let body = Self::oco_order_body(symbol, qty, side, tp_price, sl_price, sl_limit)?;
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

        Self::order_response(resp, "Order").await
    }

    /// Get orders by status (open, closed, all).
    pub async fn get_orders(&self, status: &str, limit: u32) -> Result<Vec<OrderInfo>, String> {
        let status = Self::normalize_order_query_status(status)?;
        let limit = Self::normalize_order_query_limit(limit).to_string();
        let resp = self
            .client
            .get(format!("{}/v2/orders", self.base_url))
            .headers(self.headers())
            .query(&[
                ("status", status),
                ("limit", limit.as_str()),
                ("direction", "desc"),
                ("nested", "true"), // include bracket legs (SL/TP child orders)
            ])
            .send()
            .await
            .map_err(|e| format!("Orders request failed: {e}"))?;

        // Parse as generic Value first — Alpaca returns arrays for success and
        // JSON objects with message/error for rejections.
        let json = Self::json_or_error(resp, "Orders").await?;

        let Some(arr) = json.as_array() else {
            return Err(format!(
                "Orders failed: expected array response, got {json}"
            ));
        };
        Ok(arr.iter().map(Self::parse_order_info).collect())
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
        if order_id.trim().is_empty() {
            return Err("Modify order rejected: order_id is required".into());
        }
        let body = Self::modify_order_body(qty, limit_price, stop_price, trail)?;

        let resp = self
            .client
            .patch(format!("{}/v2/orders/{}", self.base_url, order_id))
            .headers(self.headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Modify order failed: {e}"))?;

        Self::order_response(resp, "Modify order").await
    }

    /// Cancel a pending order.
    pub async fn cancel_order(&self, order_id: &str) -> Result<(), String> {
        if order_id.trim().is_empty() {
            return Err("Cancel order rejected: order_id is required".into());
        }
        let resp = self
            .client
            .delete(format!("{}/v2/orders/{}", self.base_url, order_id))
            .headers(self.headers())
            .send()
            .await
            .map_err(|e| format!("Cancel order failed: {e}"))?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if status.as_u16() == 404 {
            return Ok(());
        }
        if !status.is_success() {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Some(msg) = alpaca_error_message(&json) {
                    return Err(format!("Cancel order rejected: {msg}"));
                }
            }
            return Err(format!("Cancel order failed: HTTP {status}"));
        }
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
            qty: string_or_number(&o["qty"], "0"),
            filled_qty: string_or_number(&o["filled_qty"], "0"),
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
        percentage: Option<f64>,
    ) -> Result<OrderResult, String> {
        Self::require_symbol(symbol, "Close position")?;
        if qty.is_some() && percentage.is_some() {
            return Err(
                "Close position rejected: Alpaca accepts qty or percentage, not both".into(),
            );
        }
        if let Some(q) = qty {
            if !q.is_finite() || q <= 0.0 {
                return Err("Close position rejected: qty must be positive".into());
            }
        }
        if let Some(pct) = percentage {
            if !pct.is_finite() || pct <= 0.0 || pct > 100.0 {
                return Err("Close position rejected: percentage must be > 0 and <= 100".into());
            }
        }

        // Alpaca close-position endpoint: DELETE /v2/positions/{symbol_or_asset_id}
        // with optional query `qty` OR `percentage` (mutually exclusive).
        let encoded_symbol = symbol.replace('/', "%2F");
        let url = format!("{}/v2/positions/{}", self.base_url, encoded_symbol);
        let mut req = self.client.delete(&url).headers(self.headers());
        let qty_query;
        let pct_query;
        if let Some(q) = qty {
            qty_query = format!("{q:.9}");
            req = req.query(&[("qty", qty_query.as_str())]);
        } else if let Some(pct) = percentage {
            pct_query = format!("{pct:.9}");
            req = req.query(&[("percentage", pct_query.as_str())]);
        }

        let resp = req
            .send()
            .await
            .map_err(|e| format!("Close position failed: {e}"))?;

        Self::order_response(resp, "Close position").await
    }

    pub async fn close_position(
        &self,
        symbol: &str,
        qty: Option<f64>,
    ) -> Result<OrderResult, String> {
        self.close_position_by_amount(symbol, qty, None).await
    }

    pub async fn close_position_percent(
        &self,
        symbol: &str,
        percentage: f64,
    ) -> Result<OrderResult, String> {
        self.close_position_by_amount(symbol, None, Some(percentage))
            .await
    }

    async fn close_position_by_amount(
        &self,
        symbol: &str,
        qty: Option<f64>,
        percentage: Option<f64>,
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

        match self.close_position_once(symbol, qty, percentage).await {
            Ok(result) => Ok(result),
            Err(e) if cancelled_orders > 0 && Self::is_insufficient_qty_close_reject(&e) => {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                self.close_position_once(symbol, qty, percentage)
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
            .query(&[("cancel_orders", "true")])
            .send()
            .await
            .map_err(|e| format!("Close all failed: {e}"))?;
        let status_code = resp.status();
        let body = resp.text().await.unwrap_or_default();
        let json = serde_json::from_str::<serde_json::Value>(&body).ok();
        if !status_code.is_success() {
            if let Some(json) = json.as_ref() {
                if let Some(msg) = alpaca_error_message(json) {
                    return Err(format!("Close all rejected: {msg}"));
                }
            }
            return Err(format!("Close all failed: HTTP {status_code}"));
        }
        if let Some(json) = json.as_ref() {
            let failures = Self::close_all_positions_failures(json);
            if !failures.is_empty() {
                return Err(format!(
                    "Close all partially failed: {}",
                    failures.join("; ")
                ));
            }
        }
        Ok(())
    }

    // ── Asset Info ───────────────────────────────────────────────────

    pub async fn get_asset(&self, symbol: &str) -> Result<AssetInfo, String> {
        Self::require_symbol(symbol, "Asset")?;
        let encoded_symbol = symbol.replace('/', "%2F");
        let resp = self
            .client
            .get(format!("{}/v2/assets/{}", self.base_url, encoded_symbol))
            .headers(self.headers())
            .send()
            .await
            .map_err(|e| format!("Asset request failed: {e}"))?;

        let json = Self::json_or_error(resp, "Asset").await?;
        Ok(Self::parse_asset_info(&json))
    }

    fn parse_asset_info(json: &serde_json::Value) -> AssetInfo {
        AssetInfo {
            symbol: json["symbol"].as_str().unwrap_or("").to_string(),
            name: json["name"].as_str().unwrap_or("").to_string(),
            asset_class: json["class"].as_str().unwrap_or("").to_string(),
            tradable: json["tradable"].as_bool().unwrap_or(false),
            marginable: json["marginable"].as_bool().unwrap_or(false),
            shortable: json["shortable"].as_bool().unwrap_or(false),
            fractionable: json["fractionable"].as_bool().unwrap_or(false),
            min_order_size: optional_f64_value(&json["min_order_size"]),
            min_trade_increment: optional_f64_value(&json["min_trade_increment"]),
            price_increment: optional_f64_value(&json["price_increment"]),
        }
    }

    // ── News ─────────────────────────────────────────────────────

    fn normalize_news_limit(limit: u32) -> u32 {
        limit.clamp(1, 50)
    }

    fn parse_news_response(json: &serde_json::Value) -> Result<Vec<serde_json::Value>, String> {
        let Some(news) = json["news"].as_array() else {
            return Err(format!(
                "News failed: expected news array response, got {json}"
            ));
        };
        Ok(news.clone())
    }

    pub async fn get_news(
        &self,
        symbol: &str,
        limit: u32,
    ) -> Result<Vec<serde_json::Value>, String> {
        Self::require_symbol(symbol, "News")?;
        self.rate_limiter.wait().await;
        let limit = Self::normalize_news_limit(limit).to_string();
        let resp = self
            .client
            .get(format!("{}/v1beta1/news", DATA_BASE))
            .headers(self.headers())
            .query(&[
                ("symbols", symbol),
                ("limit", limit.as_str()),
                ("sort", "desc"),
            ])
            .send()
            .await
            .map_err(|e| format!("News request failed: {e}"))?;

        let json = Self::json_or_error(resp, "News").await?;
        Self::parse_news_response(&json)
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
        Self::require_nonblank(period, "Portfolio history", "period")?;
        Self::require_nonblank(timeframe, "Portfolio history", "timeframe")?;
        self.rate_limiter.wait().await;
        let resp = self
            .client
            .get(format!("{}/v2/account/portfolio/history", self.base_url))
            .headers(self.headers())
            .query(&[("period", period), ("timeframe", timeframe)])
            .send()
            .await
            .map_err(|e| format!("Portfolio history request failed: {e}"))?;

        Self::json_or_error(resp, "Portfolio history").await
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

        Self::json_or_error(resp, "Market clock").await
    }

    // ── Corporate Actions (Earnings/Dividends) ──────────────────

    fn parse_corporate_actions_response(
        json: &serde_json::Value,
    ) -> Result<Vec<serde_json::Value>, String> {
        let Some(actions) = json.as_array() else {
            return Err(format!(
                "Corporate actions failed: expected array response, got {json}"
            ));
        };
        Ok(actions.clone())
    }

    pub async fn get_corporate_actions(
        &self,
        symbol: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        Self::require_symbol(symbol, "Corporate actions")?;
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
            Ok(r) => {
                let json = Self::json_or_error(r, "Corporate actions").await?;
                Self::parse_corporate_actions_response(&json)
            }
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

    fn parse_assets_response(json: &serde_json::Value) -> Result<Vec<AssetInfo>, String> {
        let Some(rows) = json.as_array() else {
            return Err(format!(
                "Assets parse failed: expected array response, got {json}"
            ));
        };
        Ok(rows
            .iter()
            .filter(|a| a["tradable"].as_bool().unwrap_or(false))
            .map(Self::parse_asset_info)
            .collect())
    }

    pub async fn get_all_assets(&self) -> Result<Vec<AssetInfo>, String> {
        let resp = self
            .client
            .get(format!("{}/v2/assets", self.base_url))
            .headers(self.headers())
            .query(&[("status", "active")])
            .send()
            .await
            .map_err(|e| format!("Assets request failed: {e}"))?;

        let json = Self::json_or_error(resp, "Assets").await?;
        Self::parse_assets_response(&json)
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

        // Alpaca's stock bars API documents native Month aggregations
        // (`[1,2,3,4,6,12]Month`), so do not synthesize 1Month provider KVs.
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

    fn normalize_bar_limit(limit: u32) -> u32 {
        limit.clamp(1, 10_000)
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
        Self::require_symbol(symbol, "Bars")?;
        Self::require_nonblank(timeframe, "Bars", "timeframe")?;
        let is_crypto = symbol.contains('/');

        // Alpaca supports native Month bars; keep 1Month provider caches native
        // instead of writing weekly-aggregated bars under `alpaca:*:1Month`.
        let actual_tf = timeframe;
        let log_tf = display_timeframe.unwrap_or(actual_tf);
        let aggregation_suffix = if display_timeframe == Some("1Month") && actual_tf == "1Week" {
            " via 1Week aggregation"
        } else {
            ""
        };
        let actual_limit = Self::normalize_bar_limit(limit);

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
                        let o = parse_f64_value(&b["o"]);
                        let h = parse_f64_value(&b["h"]);
                        let l = parse_f64_value(&b["l"]);
                        let c = parse_f64_value(&b["c"]);
                        let v = parse_f64_value(&b["v"]);
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

    /// Fetch options chain from Alpaca options APIs.
    ///
    /// Trading API docs expose `/v2/options/contracts` as the authoritative
    /// contract list. Market-data snapshots are best-effort enrichment for
    /// quotes/greeks and may be missing depending on entitlements/feed.
    pub async fn get_options_chain(
        &self,
        underlying_symbol: &str,
        expiry: &str,
    ) -> Result<Vec<OptionContract>, String> {
        Self::require_symbol(underlying_symbol, "Options chain")?;
        Self::require_nonblank(expiry, "Options chain", "expiration_date")?;
        self.rate_limiter.wait().await;

        let contracts = self
            .fetch_option_contracts(underlying_symbol, expiry)
            .await?;
        if contracts.is_empty() {
            return Ok(contracts);
        }

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
            let text = resp.text().await.unwrap_or_default();
            tracing::warn!(
                "Alpaca option snapshot enrichment failed for {} {}: HTTP {} {}",
                underlying_symbol,
                expiry,
                status,
                text
            );
            return Ok(contracts);
        }

        let json: serde_json::Value = match resp.json().await {
            Ok(json) => json,
            Err(e) => {
                tracing::warn!(
                    "Alpaca option snapshot enrichment parse failed for {} {}: {}",
                    underlying_symbol,
                    expiry,
                    e
                );
                return Ok(contracts);
            }
        };

        let mut contracts = contracts;
        Self::apply_option_snapshots(&mut contracts, &json);

        // Sort by strike price
        contracts.sort_by(|a, b| {
            a.strike
                .partial_cmp(&b.strike)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(contracts)
    }

    fn apply_option_snapshots(contracts: &mut [OptionContract], json: &serde_json::Value) {
        if let Some(snapshots) = json["snapshots"].as_object() {
            for contract in contracts {
                if let Some(snap) = snapshots.get(&contract.symbol) {
                    let latest_quote = &snap["latestQuote"];
                    let greeks = &snap["greeks"];
                    contract.bid = parse_f64_value(&latest_quote["bp"]);
                    contract.ask = parse_f64_value(&latest_quote["ap"]);
                    contract.last_price = parse_f64_value(&snap["latestTrade"]["p"]);
                    contract.volume = parse_f64_value(&snap["dailyBar"]["v"]) as u64;
                    contract.open_interest = snap["openInterest"]
                        .as_u64()
                        .unwrap_or_else(|| parse_f64_value(&snap["openInterest"]) as u64);
                    contract.implied_volatility = parse_f64_value(&greeks["impliedVolatility"]);
                    contract.delta = parse_f64_value(&greeks["delta"]);
                    contract.gamma = parse_f64_value(&greeks["gamma"]);
                    contract.theta = parse_f64_value(&greeks["theta"]);
                    contract.vega = parse_f64_value(&greeks["vega"]);
                    contract.rho = parse_f64_value(&greeks["rho"]);
                }
            }
        }
    }

    async fn fetch_option_contracts(
        &self,
        underlying_symbol: &str,
        expiry: &str,
    ) -> Result<Vec<OptionContract>, String> {
        Self::require_symbol(underlying_symbol, "Options contracts")?;
        Self::require_nonblank(expiry, "Options contracts", "expiration_date")?;
        let resp = self
            .client
            .get(format!("{}/v2/options/contracts", self.base_url))
            .headers(self.headers())
            .query(&[
                ("underlying_symbols", underlying_symbol),
                ("status", "active"),
                ("expiration_date", expiry),
                ("show_deliverables", "true"),
            ])
            .send()
            .await
            .map_err(|e| format!("Options contracts request failed: {e}"))?;
        let json = Self::json_or_error(resp, "Options contracts").await?;
        let Some(rows) = json["option_contracts"].as_array() else {
            return Err(format!(
                "Options contracts failed: expected option_contracts array response, got {json}"
            ));
        };
        Ok(rows
            .iter()
            .map(|c| {
                let symbol = c["symbol"].as_str().unwrap_or("").to_string();
                let (parsed_strike, parsed_type, parsed_expiry) =
                    Self::parse_option_symbol(&symbol);
                OptionContract {
                    symbol,
                    underlying: c["underlying_symbol"]
                        .as_str()
                        .unwrap_or(underlying_symbol)
                        .to_string(),
                    strike: parse_f64_value(&c["strike_price"]).max(parsed_strike),
                    expiry: c["expiration_date"]
                        .as_str()
                        .unwrap_or(&parsed_expiry)
                        .to_string(),
                    option_type: c["type"].as_str().unwrap_or(&parsed_type).to_string(),
                    bid: 0.0,
                    ask: 0.0,
                    last_price: 0.0,
                    volume: 0,
                    open_interest: parse_f64_value(&c["open_interest"]) as u64,
                    implied_volatility: 0.0,
                    delta: 0.0,
                    gamma: 0.0,
                    theta: 0.0,
                    vega: 0.0,
                    rho: 0.0,
                }
            })
            .collect())
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

    fn normalize_screener_top(top: u32) -> u32 {
        top.clamp(1, 50)
    }

    fn normalize_screener_market_type(market_type: &str) -> Result<&'static str, String> {
        match market_type.trim().to_ascii_lowercase().as_str() {
            "stocks" => Ok("stocks"),
            "crypto" => Ok("crypto"),
            other => Err(format!(
                "Screener rejected: market_type must be 'stocks' or 'crypto', got '{other}'"
            )),
        }
    }

    /// Fetch most active stocks by volume/trade count from Alpaca screener API.
    pub async fn get_most_active(&self, top: u32) -> Result<serde_json::Value, String> {
        self.rate_limiter.wait().await;
        let top = Self::normalize_screener_top(top).to_string();

        let resp = self
            .client
            .get(format!(
                "{}/v1beta1/screener/stocks/most-actives",
                DATA_BASE
            ))
            .headers(self.headers())
            .query(&[("top", top.as_str())])
            .send()
            .await
            .map_err(|e| format!("Most actives request failed: {e}"))?;

        Self::json_or_error(resp, "Most actives").await
    }

    /// Fetch top movers (gainers/losers) from Alpaca screener API.
    /// market_type: "stocks" or "crypto"
    pub async fn get_top_movers(
        &self,
        market_type: &str,
        top: u32,
    ) -> Result<serde_json::Value, String> {
        let market_type = Self::normalize_screener_market_type(market_type)?;
        let top = Self::normalize_screener_top(top).to_string();
        self.rate_limiter.wait().await;

        let resp = self
            .client
            .get(format!(
                "{}/v1beta1/screener/{}/movers",
                DATA_BASE, market_type
            ))
            .headers(self.headers())
            .query(&[("top", top.as_str())])
            .send()
            .await
            .map_err(|e| format!("Top movers request failed: {e}"))?;

        Self::json_or_error(resp, "Top movers").await
    }

    // ── DOM / Level 2 (Crypto Orderbook) ──────────────────────────────

    /// Fetch crypto orderbook snapshot from Alpaca.
    pub async fn get_orderbook(&self, symbol: &str) -> Result<serde_json::Value, String> {
        Self::require_symbol(symbol, "Orderbook")?;
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

        let json = Self::json_or_error(resp, "Orderbook").await?;

        Self::parse_crypto_orderbook_snapshot(symbol, &json)
    }

    fn parse_crypto_orderbook_snapshot(
        symbol: &str,
        json: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let orderbook = &json["orderbooks"][symbol];
        if orderbook.is_null() {
            return Err(format!("No orderbook data for {symbol}"));
        }

        let parse_level = |entry: &serde_json::Value| -> serde_json::Value {
            serde_json::json!({
                "price": parse_f64_value(&entry["p"]),
                "size": parse_f64_value(&entry["s"]),
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

    fn stock_snapshot_feeds(&self) -> Vec<Option<&'static str>> {
        if self.sip_bar_feed_unavailable.load(Ordering::Relaxed) {
            vec![Some("iex")]
        } else {
            vec![Some("sip"), Some("iex")]
        }
    }

    fn parse_latest_quote_from_snapshot(symbol: &str, json: &serde_json::Value) -> LatestQuote {
        let q = &json["latestQuote"];
        let bid = parse_f64_value(&q["bp"]);
        let ask = parse_f64_value(&q["ap"]);

        // Latest trade can be the only useful price outside regular hours.
        let t = &json["latestTrade"];
        let trade_price = parse_f64_value(&t["p"]);
        let trade_ts = t["t"].as_str().unwrap_or("");

        let (final_bid, final_ask) = if bid > 0.0 && ask > 0.0 {
            (bid, ask)
        } else if trade_price > 0.0 {
            (trade_price, trade_price)
        } else {
            (0.0, 0.0)
        };

        LatestQuote {
            symbol: symbol.to_string(),
            bid: final_bid,
            ask: final_ask,
            bid_size: parse_f64_value(&q["bs"]),
            ask_size: parse_f64_value(&q["as"]),
            spread: final_ask - final_bid,
            timestamp: if trade_ts.is_empty() {
                q["t"].as_str().unwrap_or("").to_string()
            } else {
                trade_ts.to_string()
            },
        }
    }

    fn parse_crypto_latest_quote(
        symbol: &str,
        json: &serde_json::Value,
    ) -> Result<LatestQuote, String> {
        let q = &json["quotes"][symbol];
        if q.is_null() {
            return Err(format!("Quote rejected: no quote returned for {symbol}"));
        }
        let bid = parse_f64_value(&q["bp"]);
        let ask = parse_f64_value(&q["ap"]);
        Ok(LatestQuote {
            symbol: symbol.to_string(),
            bid,
            ask,
            bid_size: parse_f64_value(&q["bs"]),
            ask_size: parse_f64_value(&q["as"]),
            spread: ask - bid,
            timestamp: q["t"].as_str().unwrap_or("").to_string(),
        })
    }

    fn parse_snapshot_data(symbol: &str, json: &serde_json::Value) -> SnapshotData {
        let trade_price = parse_f64_value(&json["latestTrade"]["p"]);
        let daily_volume = parse_f64_value(&json["dailyBar"]["v"]);
        let prev_close = parse_f64_value(&json["prevDailyBar"]["c"]);
        let regular_close = parse_f64_value(&json["dailyBar"]["c"]);
        let last = if trade_price > 0.0 {
            trade_price
        } else {
            regular_close
        };
        SnapshotData {
            symbol: symbol.to_string(),
            last,
            prev_close,
            daily_volume,
            regular_close,
        }
    }

    fn parse_crypto_snapshot_data(
        symbol: &str,
        json: &serde_json::Value,
    ) -> Result<SnapshotData, String> {
        let snap = &json["snapshots"][symbol];
        if snap.is_null() {
            return Err(format!(
                "Crypto snapshot rejected: no snapshot returned for {symbol}"
            ));
        }
        let trade_price = parse_f64_value(&snap["latestTrade"]["p"]);
        let regular_close = parse_f64_value(&snap["dailyBar"]["c"]);
        let last = if trade_price > 0.0 {
            trade_price
        } else {
            regular_close
        };
        Ok(SnapshotData {
            symbol: symbol.to_string(),
            last,
            prev_close: parse_f64_value(&snap["prevDailyBar"]["c"]),
            daily_volume: parse_f64_value(&snap["dailyBar"]["v"]),
            regular_close,
        })
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
            Self::parse_crypto_latest_quote(symbol, &json)
        } else {
            // Stocks/ETFs: use snapshot endpoint for quote + trade fallback.
            // Prefer SIP when entitled, but fall back to IEX instead of failing
            // watchlist/position UI on free-tier accounts.
            let url = format!("{}/v2/stocks/{}/snapshot", DATA_BASE, symbol);
            let mut last_error = String::new();
            for feed in self.stock_snapshot_feeds() {
                let mut req = self.client.get(&url).headers(self.headers());
                if let Some(feed) = feed {
                    req = req.query(&[("feed", feed)]);
                }
                let resp = req
                    .send()
                    .await
                    .map_err(|e| format!("Snapshot request failed: {e}"))?;
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                if !status.is_success() {
                    if feed == Some("sip") && Self::is_sip_bar_entitlement_failure(status, &text) {
                        self.sip_bar_feed_unavailable.store(true, Ordering::Relaxed);
                        last_error = format!("SIP snapshot unavailable: HTTP {status}");
                        continue;
                    }
                    return Err(format!("Snapshot request failed: HTTP {status}"));
                }
                let json: serde_json::Value = serde_json::from_str(&text)
                    .map_err(|e| format!("Snapshot parse failed: {e}"))?;
                return Ok(Self::parse_latest_quote_from_snapshot(symbol, &json));
            }
            Err(if last_error.is_empty() {
                "Snapshot request failed: no feed attempted".to_string()
            } else {
                last_error
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
            Self::parse_crypto_snapshot_data(symbol, &json)
        } else {
            // Stock/ETF snapshot. Prefer SIP for extended-hours trades when
            // entitled, but degrade to IEX for free-tier accounts.
            let url = format!("{}/v2/stocks/{}/snapshot", DATA_BASE, symbol);
            let mut last_error = String::new();
            for feed in self.stock_snapshot_feeds() {
                let mut req = self.client.get(&url).headers(self.headers());
                if let Some(feed) = feed {
                    req = req.query(&[("feed", feed)]);
                }
                let resp = req
                    .send()
                    .await
                    .map_err(|e| format!("Snapshot failed: {e}"))?;
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                if !status.is_success() {
                    if feed == Some("sip") && Self::is_sip_bar_entitlement_failure(status, &text) {
                        self.sip_bar_feed_unavailable.store(true, Ordering::Relaxed);
                        last_error = format!("SIP snapshot unavailable: HTTP {status}");
                        continue;
                    }
                    return Err(format!("Snapshot HTTP {status}"));
                }
                let json: serde_json::Value =
                    serde_json::from_str(&text).map_err(|e| format!("Snapshot parse: {e}"))?;
                return Ok(Self::parse_snapshot_data(symbol, &json));
            }
            Err(if last_error.is_empty() {
                "Snapshot failed: no feed attempted".to_string()
            } else {
                last_error
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

        let json = Self::json_or_error(resp, "Activities").await?;
        let Some(json) = json.as_array() else {
            return Err(format!(
                "Activities failed: expected array response, got {json}"
            ));
        };

        Ok(json.iter().map(Self::parse_account_activity).collect())
    }

    fn parse_account_activity(a: &serde_json::Value) -> AccountActivity {
        let activity_type = a["activity_type"].as_str().unwrap_or("").to_string();
        let qty = optional_string_or_number(&a["qty"]);
        let price = optional_string_or_number(&a["price"]);
        let net_amount = optional_string_or_number(&a["net_amount"]);
        let description = match activity_type.as_str() {
            "FILL" => format!(
                "{} {} {} @ {}",
                a["side"].as_str().unwrap_or(""),
                qty.as_deref().unwrap_or("0"),
                a["symbol"].as_str().unwrap_or(""),
                price.as_deref().unwrap_or("?")
            ),
            "DIV" | "DIVCGL" | "DIVCGS" | "DIVNRA" | "DIVROC" | "DIVTXEX" => format!(
                "Dividend {} ${}",
                a["symbol"].as_str().unwrap_or(""),
                net_amount.as_deref().unwrap_or("0")
            ),
            "CSD" => format!("Deposit ${}", net_amount.as_deref().unwrap_or("0")),
            "CSW" => format!("Withdrawal ${}", net_amount.as_deref().unwrap_or("0")),
            _ => format!("{} {}", activity_type, a["symbol"].as_str().unwrap_or("")),
        };
        AccountActivity {
            id: a["id"].as_str().unwrap_or("").to_string(),
            activity_type,
            symbol: a["symbol"].as_str().map(|s| s.to_string()),
            side: a["side"].as_str().map(|s| s.to_string()),
            qty,
            price,
            net_amount,
            date: a["transaction_time"]
                .as_str()
                .or_else(|| a["date"].as_str())
                .unwrap_or("")
                .to_string(),
            description,
        }
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

    fn normalize_watchlist_symbols(
        symbols: &[String],
        context: &str,
    ) -> Result<Vec<String>, String> {
        let out: Vec<String> = symbols
            .iter()
            .map(|symbol| symbol.trim())
            .filter(|symbol| !symbol.is_empty())
            .map(|symbol| symbol.to_string())
            .collect();
        if out.is_empty() {
            return Err(format!(
                "{context} rejected: at least one symbol is required"
            ));
        }
        Ok(out)
    }

    fn create_watchlist_body(name: &str, symbols: &[String]) -> Result<serde_json::Value, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Create watchlist rejected: name is required".into());
        }
        let symbols = Self::normalize_watchlist_symbols(symbols, "Create watchlist")?;
        Ok(serde_json::json!({
            "name": name,
            "symbols": symbols,
        }))
    }

    fn update_watchlist_body(symbols: &[String]) -> Result<serde_json::Value, String> {
        let symbols = Self::normalize_watchlist_symbols(symbols, "Update watchlist")?;
        Ok(serde_json::json!({
            "symbols": symbols,
        }))
    }

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

        let status = resp.status();
        if status.as_u16() == 429 {
            self.rate_limiter.trigger_cooldown().await;
        }
        let json = Self::json_or_error(resp, "Get watchlists").await?;
        let Some(arr) = json.as_array() else {
            return Err(format!(
                "Get watchlists failed: expected array response, got {json}"
            ));
        };
        Ok(arr.clone())
    }

    /// Create a new watchlist on Alpaca.
    pub async fn create_watchlist(
        &self,
        name: &str,
        symbols: &[String],
    ) -> Result<serde_json::Value, String> {
        self.rate_limiter.wait().await;
        let body = Self::create_watchlist_body(name, symbols)?;
        let resp = self
            .client
            .post(format!("{}/v2/watchlists", self.base_url))
            .headers(self.headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Create watchlist failed: {e}"))?;

        let status = resp.status();
        if status.as_u16() == 429 {
            self.rate_limiter.trigger_cooldown().await;
        }
        Self::json_or_error(resp, "Create watchlist").await
    }

    /// Update an existing watchlist on Alpaca (replace symbols).
    pub async fn update_watchlist(
        &self,
        id: &str,
        symbols: &[String],
    ) -> Result<serde_json::Value, String> {
        if id.trim().is_empty() {
            return Err("Update watchlist rejected: id is required".into());
        }
        self.rate_limiter.wait().await;
        let body = Self::update_watchlist_body(symbols)?;
        let resp = self
            .client
            .put(format!("{}/v2/watchlists/{}", self.base_url, id.trim()))
            .headers(self.headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Update watchlist failed: {e}"))?;

        let status = resp.status();
        if status.as_u16() == 429 {
            self.rate_limiter.trigger_cooldown().await;
        }
        Self::json_or_error(resp, "Update watchlist").await
    }
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
mod tests;
