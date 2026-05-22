//! Kraken broker interface (Phase 1: Authentication + Account).
//!
//! Wraps Kraken REST API for account management and trading.
//! Separate from `core/kraken.rs` which handles public OHLCV data only.
//! See ADR-072 for the full integration plan.

use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use hmac::{Hmac, KeyInit, Mac};
use reqwest::Client;
use sha2::{Digest, Sha256, Sha512};
use std::collections::HashMap;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use zeroize::Zeroizing;

type HmacSha512 = Hmac<Sha512>;

const KRAKEN_BASE_URL: &str = "https://api.kraken.com";
const KRAKEN_INTERNAL_API_BASE_URL: &str = "https://iapi.kraken.com/api/internal";
const KRAKEN_PRIVATE_REST_MAX_COUNTER: f64 = 20.0;
const KRAKEN_PRIVATE_REST_DECAY_PER_SEC: f64 = 0.5;
const KRAKEN_PRIVATE_REST_BASE_COOLDOWN: Duration = Duration::from_secs(5);
const KRAKEN_PRIVATE_REST_MAX_COOLDOWN: Duration = Duration::from_secs(60);
const KRAKEN_PRIVATE_REST_MAX_ATTEMPTS: usize = 3;

#[derive(Debug)]
struct KrakenPrivateRestLimiter {
    state: Mutex<KrakenPrivateRestState>,
}

#[derive(Debug)]
struct KrakenPrivateRestState {
    counter: f64,
    last_decay: Instant,
    cooldown_until: Option<Instant>,
    cooldown: Duration,
}

impl KrakenPrivateRestLimiter {
    fn new() -> Self {
        Self {
            state: Mutex::new(KrakenPrivateRestState {
                counter: 0.0,
                last_decay: Instant::now(),
                cooldown_until: None,
                cooldown: Duration::ZERO,
            }),
        }
    }

    async fn wait(&self, cost: f64) {
        if cost <= 0.0 {
            return;
        }

        loop {
            let wait = {
                let now = Instant::now();
                let mut state = self.state.lock().await;
                state.decay(now);

                let cooldown_wait = if let Some(cooldown_until) = state.cooldown_until {
                    if cooldown_until > now {
                        Some(cooldown_until.saturating_duration_since(now))
                    } else {
                        state.cooldown_until = None;
                        state.cooldown = Duration::ZERO;
                        None
                    }
                } else {
                    None
                };

                if let Some(wait) = cooldown_wait {
                    Some(wait)
                } else if state.counter + cost <= KRAKEN_PRIVATE_REST_MAX_COUNTER {
                    state.counter += cost;
                    None
                } else {
                    let excess = (state.counter + cost) - KRAKEN_PRIVATE_REST_MAX_COUNTER;
                    Some(Duration::from_secs_f64(
                        (excess / KRAKEN_PRIVATE_REST_DECAY_PER_SEC).max(0.25),
                    ))
                }
            };

            if let Some(wait) = wait {
                if !wait.is_zero() {
                    tokio::time::sleep(wait).await;
                }
                continue;
            }
            return;
        }
    }

    async fn record_rate_limited(&self, message: &str) -> Duration {
        let explicit_wait = crate::core::kraken::kraken_throttled_wait(message).map(|wait| {
            if wait.is_zero() {
                KRAKEN_PRIVATE_REST_BASE_COOLDOWN
            } else {
                wait.min(KRAKEN_PRIVATE_REST_MAX_COOLDOWN)
            }
        });
        let now = Instant::now();
        let mut state = self.state.lock().await;
        state.decay(now);
        let wait = if let Some(wait) = explicit_wait {
            wait
        } else if state.cooldown_until.is_some_and(|until| until > now) {
            state
                .cooldown
                .max(KRAKEN_PRIVATE_REST_BASE_COOLDOWN)
                .saturating_mul(2)
                .min(KRAKEN_PRIVATE_REST_MAX_COOLDOWN)
        } else {
            KRAKEN_PRIVATE_REST_BASE_COOLDOWN
        };
        state.cooldown = wait;
        let cooldown_until = now + wait;
        state.cooldown_until = Some(
            state
                .cooldown_until
                .map(|existing| existing.max(cooldown_until))
                .unwrap_or(cooldown_until),
        );
        wait
    }

    async fn record_success(&self) {
        let now = Instant::now();
        let mut state = self.state.lock().await;
        state.decay(now);
        if state
            .cooldown_until
            .is_some_and(|cooldown_until| cooldown_until <= now)
        {
            state.cooldown_until = None;
            state.cooldown = Duration::ZERO;
        }
    }
}

impl KrakenPrivateRestState {
    fn decay(&mut self, now: Instant) {
        let elapsed = now.saturating_duration_since(self.last_decay);
        if !elapsed.is_zero() {
            self.counter =
                (self.counter - elapsed.as_secs_f64() * KRAKEN_PRIVATE_REST_DECAY_PER_SEC).max(0.0);
            self.last_decay = now;
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct KrakenConditionalClose {
    pub order_type: String,
    pub price: Option<String>,
    pub price2: Option<String>,
}

impl KrakenConditionalClose {
    pub fn new(order_type: impl Into<String>) -> Self {
        Self {
            order_type: order_type.into(),
            price: None,
            price2: None,
        }
    }
}

/// Full Kraken Spot REST AddOrder request.
///
/// This uses Kraken's REST/WebSocket v1 field names because REST AddOrder is a
/// form endpoint: `type`, `ordertype`, `price`, `price2`, `oflags`, `starttm`,
/// `expiretm`, `timeinforce`, and `close[...]`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct KrakenOrderRequest {
    pub pair: String,
    pub side: String,
    pub order_type: String,
    pub volume: String,
    pub price: Option<String>,
    pub price2: Option<String>,
    pub display_volume: Option<String>,
    pub leverage: Option<String>,
    pub margin: Option<bool>,
    pub reduce_only: bool,
    pub oflags: Vec<String>,
    pub start_time: Option<String>,
    pub expire_time: Option<String>,
    pub deadline: Option<String>,
    pub client_order_id: Option<String>,
    pub userref: Option<String>,
    pub sender_sub_id: Option<String>,
    pub stp_type: Option<String>,
    pub validate: bool,
    pub time_in_force: Option<String>,
    pub close: Option<KrakenConditionalClose>,
    pub req_id: Option<i64>,
}

impl KrakenOrderRequest {
    pub fn basic(
        pair: impl Into<String>,
        side: impl Into<String>,
        order_type: impl Into<String>,
        volume: f64,
    ) -> Self {
        Self {
            pair: pair.into(),
            side: side.into(),
            order_type: order_type.into(),
            volume: format_f64_param(volume),
            ..Self::default()
        }
    }

    pub fn with_price(mut self, price: f64) -> Self {
        self.price = Some(format_f64_param(price));
        self
    }

    pub fn with_price2(mut self, price2: f64) -> Self {
        self.price2 = Some(format_f64_param(price2));
        self
    }

    pub fn with_display_volume(mut self, display_volume: f64) -> Self {
        self.display_volume = Some(format_f64_param(display_volume));
        self
    }

    fn normalized_order_type(&self) -> String {
        normalize_kraken_order_type(&self.order_type)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.pair.trim().is_empty() {
            return Err("Kraken order pair is required".to_string());
        }
        if !matches!(self.side.to_ascii_lowercase().as_str(), "buy" | "sell") {
            return Err(format!("Unsupported Kraken order side: {}", self.side));
        }
        let order_type = self.normalized_order_type();
        if !is_supported_kraken_order_type(&order_type) {
            return Err(format!(
                "Unsupported Kraken order type: {}",
                self.order_type
            ));
        }
        let volume = self
            .volume
            .parse::<f64>()
            .map_err(|_| format!("Invalid Kraken order volume: {}", self.volume))?;
        if !volume.is_finite() || (volume <= 0.0 && order_type != "settle-position") {
            return Err(format!("Invalid Kraken order volume: {}", self.volume));
        }
        if order_type == "iceberg" && self.display_volume.as_deref().unwrap_or("").is_empty() {
            return Err("Kraken iceberg order requires displayvol".to_string());
        }
        if requires_primary_price(&order_type) && self.price.as_deref().unwrap_or("").is_empty() {
            return Err(format!("Kraken {order_type} order requires price"));
        }
        if requires_secondary_price(&order_type) && self.price2.as_deref().unwrap_or("").is_empty()
        {
            return Err(format!("Kraken {order_type} order requires price2"));
        }
        if self.client_order_id.is_some() && self.userref.is_some() {
            return Err("Kraken cl_ord_id and userref are mutually exclusive".to_string());
        }
        if let Some(tif) = &self.time_in_force {
            let tif = tif.to_ascii_uppercase();
            if !matches!(tif.as_str(), "GTC" | "GTD" | "IOC") {
                return Err(format!("Unsupported Kraken timeinforce: {tif}"));
            }
        }
        if let Some(stp) = &self.stp_type {
            if !matches!(
                stp.to_ascii_lowercase().as_str(),
                "cancel_newest" | "cancel_oldest" | "cancel_both"
            ) {
                return Err(format!("Unsupported Kraken stp_type: {stp}"));
            }
        }
        if let Some(close) = &self.close {
            let close_type = normalize_kraken_order_type(&close.order_type);
            if !is_supported_kraken_close_order_type(&close_type) {
                return Err(format!(
                    "Unsupported Kraken conditional close order type: {}",
                    close.order_type
                ));
            }
            if requires_primary_price(&close_type)
                && close.price.as_deref().unwrap_or("").is_empty()
            {
                return Err(format!("Kraken conditional {close_type} requires price"));
            }
            if requires_secondary_price(&close_type)
                && close.price2.as_deref().unwrap_or("").is_empty()
            {
                return Err(format!("Kraken conditional {close_type} requires price2"));
            }
        }
        Ok(())
    }

    fn to_params(&self) -> Vec<(String, String)> {
        let mut params = vec![
            ("pair".to_string(), self.pair.clone()),
            ("type".to_string(), self.side.to_ascii_lowercase()),
            ("ordertype".to_string(), self.rest_order_type()),
            ("volume".to_string(), self.volume.clone()),
        ];
        push_opt_param(&mut params, "price", self.price.as_deref());
        push_opt_param(&mut params, "price2", self.price2.as_deref());
        push_opt_param(&mut params, "displayvol", self.display_volume.as_deref());
        push_opt_param(&mut params, "leverage", self.leverage.as_deref());
        if let Some(margin) = self.margin {
            params.push(("margin".to_string(), margin.to_string()));
        }
        if self.reduce_only {
            params.push(("reduce_only".to_string(), "true".to_string()));
        }
        if !self.oflags.is_empty() {
            params.push(("oflags".to_string(), self.oflags.join(",")));
        }
        push_opt_param(&mut params, "starttm", self.start_time.as_deref());
        push_opt_param(&mut params, "expiretm", self.expire_time.as_deref());
        push_opt_param(&mut params, "deadline", self.deadline.as_deref());
        push_opt_param(&mut params, "cl_ord_id", self.client_order_id.as_deref());
        push_opt_param(&mut params, "userref", self.userref.as_deref());
        push_opt_param(&mut params, "sender_sub_id", self.sender_sub_id.as_deref());
        push_opt_param(&mut params, "stp_type", self.stp_type.as_deref());
        if self.validate {
            params.push(("validate".to_string(), "true".to_string()));
        }
        if let Some(tif) = &self.time_in_force {
            params.push(("timeinforce".to_string(), tif.to_ascii_uppercase()));
        }
        if let Some(close) = &self.close {
            params.push((
                "close[ordertype]".to_string(),
                normalize_kraken_order_type(&close.order_type),
            ));
            push_opt_param(&mut params, "close[price]", close.price.as_deref());
            push_opt_param(&mut params, "close[price2]", close.price2.as_deref());
        }
        if let Some(req_id) = self.req_id {
            params.push(("reqid".to_string(), req_id.to_string()));
        }
        params
    }

    fn rest_order_type(&self) -> String {
        let order_type = self.normalized_order_type();
        if order_type == "iceberg" {
            "limit".to_string()
        } else {
            order_type
        }
    }
}

/// Shared HTTP client for Kraken API requests (reuses TCP connections).
fn kraken_client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .pool_max_idle_per_host(2)
            .build()
            .unwrap_or_else(|_| Client::new())
    })
}

fn format_f64_param(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        value.to_string()
    }
}

fn push_opt_param(params: &mut Vec<(String, String)>, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        if !value.is_empty() {
            params.push((key.to_string(), value.to_string()));
        }
    }
}

fn normalize_kraken_order_type(order_type: &str) -> String {
    order_type.trim().replace('_', "-").to_ascii_lowercase()
}

fn is_supported_kraken_order_type(order_type: &str) -> bool {
    matches!(
        order_type,
        "market"
            | "limit"
            | "iceberg"
            | "stop-loss"
            | "stop-loss-limit"
            | "take-profit"
            | "take-profit-limit"
            | "trailing-stop"
            | "trailing-stop-limit"
            | "settle-position"
    )
}

fn is_supported_kraken_close_order_type(order_type: &str) -> bool {
    matches!(
        order_type,
        "limit"
            | "stop-loss"
            | "stop-loss-limit"
            | "take-profit"
            | "take-profit-limit"
            | "trailing-stop"
            | "trailing-stop-limit"
    )
}

fn requires_primary_price(order_type: &str) -> bool {
    matches!(
        order_type,
        "limit"
            | "iceberg"
            | "stop-loss"
            | "stop-loss-limit"
            | "take-profit"
            | "take-profit-limit"
            | "trailing-stop"
            | "trailing-stop-limit"
    )
}

fn requires_secondary_price(order_type: &str) -> bool {
    matches!(
        order_type,
        "stop-loss-limit" | "take-profit-limit" | "trailing-stop-limit"
    )
}

fn encode_form_component(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn encode_form_params(params: &[(String, String)]) -> String {
    params
        .iter()
        .map(|(k, v)| format!("{}={}", encode_form_component(k), encode_form_component(v)))
        .collect::<Vec<_>>()
        .join("&")
}

fn sanitize_api_error_body(body: &str) -> String {
    let mut clean = body.split_whitespace().collect::<Vec<_>>().join(" ");
    if clean.len() > 512 {
        clean.truncate(512);
        clean.push('…');
    }
    clean
}

fn kraken_private_rest_counter_cost(path: &str) -> f64 {
    let endpoint = path.rsplit('/').next().unwrap_or(path);
    if matches!(
        endpoint,
        "AddOrder"
            | "AddOrderBatch"
            | "AmendOrder"
            | "EditOrder"
            | "CancelOrder"
            | "CancelOrderBatch"
            | "CancelAll"
            | "CancelAllOrdersAfter"
    ) {
        0.0
    } else if matches!(
        endpoint,
        "Ledgers" | "QueryLedgers" | "TradesHistory" | "QueryTrades" | "ClosedOrders"
    ) {
        4.0
    } else {
        1.0
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KrakenEquityTicker {
    pub symbol: String,
    pub bid: f64,
    pub ask: f64,
    pub price: f64,
    pub volume: f64,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub previous_close: Option<f64>,
    pub time_ms: i64,
    pub delayed: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KrakenEquityBar {
    pub time_ms: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KrakenEquityMarket {
    pub symbol: String,
    pub name: Option<String>,
    pub tradable: bool,
    pub status: Option<String>,
    pub instrument_status: Option<String>,
}

fn parse_json_number(value: &serde_json::Value) -> Option<f64> {
    match value {
        serde_json::Value::String(s) => s.parse::<f64>().ok(),
        serde_json::Value::Number(n) => n.as_f64(),
        _ => None,
    }
    .filter(|v| v.is_finite())
}

fn parse_json_i64(value: &serde_json::Value) -> Option<i64> {
    match value {
        serde_json::Value::String(s) => s.parse::<i64>().ok(),
        serde_json::Value::Number(n) => n.as_i64().or_else(|| n.as_u64().map(|v| v as i64)),
        _ => None,
    }
}

/// Kraken broker client with HMAC-SHA512 request signing.
pub struct KrakenBroker {
    client: &'static Client,
    api_key: Zeroizing<String>,
    api_secret: Zeroizing<String>, // base64-encoded, zeroized on drop
    nonce: AtomicU64,
    private_limiter: KrakenPrivateRestLimiter,
}

impl KrakenBroker {
    /// Create a new Kraken broker instance.
    /// Pass empty strings for unauthenticated (public-endpoint-only) usage.
    pub fn new(api_key: String, api_secret: String) -> Self {
        let api_key = Zeroizing::new(api_key);
        let api_secret = Zeroizing::new(api_secret);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Self {
            client: kraken_client(),
            api_key,
            api_secret,
            nonce: AtomicU64::new(now),
            private_limiter: KrakenPrivateRestLimiter::new(),
        }
    }

    /// Returns true if API credentials are configured.
    pub fn is_authenticated(&self) -> bool {
        !self.api_key.is_empty() && !self.api_secret.is_empty()
    }

    /// Fetch the full public Kraken Securities/equities universe from Kraken Pro's
    /// internal market catalog. This endpoint is unauthenticated but needs the same
    /// frontend headers Kraken Pro sends; without them iapi currently returns 404.
    pub async fn get_equity_markets(&self) -> Result<Vec<KrakenEquityMarket>, String> {
        const PAGE_SIZE: usize = 1000;
        let mut page = 0usize;
        let mut out = Vec::new();
        loop {
            let page_s = page.to_string();
            let page_size_s = PAGE_SIZE.to_string();
            let url = format!("{KRAKEN_INTERNAL_API_BASE_URL}/markets/all/equities");
            let resp = self
                .client
                .get(&url)
                .header("Accept", "application/json")
                .header("Referer", "https://pro.kraken.com/app/")
                .header("Origin", "https://pro.kraken.com")
                .header("x-handler-environment", "stable")
                .header("x-initiated-through", "@frontend/cts-core")
                .query(&[
                    ("delayed", "true"),
                    ("tradable", "true"),
                    ("page_size", page_size_s.as_str()),
                    ("page", page_s.as_str()),
                ])
                .send()
                .await
                .map_err(|e| format!("Kraken equity catalog request failed: {e}"))?;
            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                return Err(format!(
                    "Kraken equity catalog request failed: HTTP {status}: {body}"
                ));
            }
            let body: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| format!("Kraken equity catalog parse failed: {e}"))?;
            if let Some(errors) = body.get("errors").and_then(|v| v.as_array()) {
                if !errors.is_empty() {
                    let msg = errors
                        .iter()
                        .map(|e| {
                            e.get("msg")
                                .or_else(|| e.get("type"))
                                .or_else(|| e.get("error"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown Kraken equity catalog error")
                                .to_string()
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    return Err(format!("Kraken equity catalog error: {msg}"));
                }
            }
            let Some(result) = body.get("result") else {
                break;
            };
            let data = result
                .get("data")
                .and_then(|v| v.as_array())
                .ok_or_else(|| "Kraken equity catalog: missing result.data".to_string())?;
            if data.is_empty() {
                break;
            }
            for item in data {
                let Some(symbol) = item.get("symbol").and_then(|v| v.as_str()) else {
                    continue;
                };
                let symbol = symbol.trim().trim_end_matches(".EQ").to_ascii_uppercase();
                if symbol.is_empty() {
                    continue;
                }
                out.push(KrakenEquityMarket {
                    symbol,
                    name: item
                        .get("name")
                        .or_else(|| item.get("short_name"))
                        .and_then(|v| v.as_str())
                        .map(str::to_string),
                    tradable: item.get("tradable").and_then(|v| v.as_bool()).unwrap_or(true),
                    status: item.get("status").and_then(|v| v.as_str()).map(str::to_string),
                    instrument_status: item
                        .get("instrument_status")
                        .and_then(|v| v.as_str())
                        .map(str::to_string),
                });
            }
            let total_results = result
                .get("total_results")
                .and_then(|v| v.as_u64())
                .unwrap_or(out.len() as u64) as usize;
            if out.len() >= total_results || data.len() < PAGE_SIZE {
                break;
            }
            page += 1;
            if page > 100 {
                return Err("Kraken equity catalog: pagination safety limit hit".to_string());
            }
        }
        out.sort_by(|a, b| a.symbol.cmp(&b.symbol));
        out.dedup_by(|a, b| a.symbol == b.symbol);
        Ok(out)
    }

    /// Fetch delayed Kraken Securities/equities quote data from Kraken Pro's
    /// internal equities market-data API. This is separate from Kraken Spot:
    /// xStock/equity holdings such as `WOK.EQ` are not in public AssetPairs.
    pub async fn get_equity_ticker(&self, symbol: &str) -> Result<KrakenEquityTicker, String> {
        let symbol = symbol
            .trim()
            .trim_end_matches(".EQ")
            .replace('/', "")
            .to_ascii_uppercase();
        if symbol.is_empty() {
            return Err("Kraken equity ticker: empty symbol".to_string());
        }

        let url = format!("{KRAKEN_INTERNAL_API_BASE_URL}/markets/equities/{symbol}/ticker");
        let resp = self
            .client
            .get(&url)
            .header("Accept", "application/json")
            .header("Referer", "https://pro.kraken.com/app/")
            .query(&[("delayed", "true")])
            .send()
            .await
            .map_err(|e| format!("Kraken equity ticker request failed: {e}"))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!(
                "Kraken equity ticker request failed: HTTP {status}: {body}"
            ));
        }

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Kraken equity ticker parse failed: {e}"))?;
        if let Some(errors) = body.get("errors").and_then(|v| v.as_array()) {
            if !errors.is_empty() {
                let msg = errors
                    .iter()
                    .map(|e| {
                        e.get("msg")
                            .or_else(|| e.get("type"))
                            .or_else(|| e.get("error"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown Kraken equity ticker error")
                            .to_string()
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                return Err(format!("Kraken equity ticker error: {msg}"));
            }
        }

        let result = body
            .get("result")
            .filter(|v| !v.is_null())
            .ok_or_else(|| format!("Kraken equity ticker: no data for {symbol}"))?;

        let parse = |field: &str| -> Option<f64> {
            result
                .get(field)
                .and_then(parse_json_number)
                .filter(|v| v.is_finite() && *v >= 0.0)
        };
        let bid = parse("bid").unwrap_or(0.0);
        let ask = parse("ask").unwrap_or(0.0);
        let price = parse("price")
            .or_else(|| parse("last"))
            .or_else(|| match (bid > 0.0, ask > 0.0) {
                (true, true) => Some((bid + ask) / 2.0),
                (true, false) => Some(bid),
                (false, true) => Some(ask),
                _ => None,
            })
            .ok_or_else(|| format!("Kraken equity ticker: missing price for {symbol}"))?;

        Ok(KrakenEquityTicker {
            symbol,
            bid,
            ask,
            price,
            volume: parse("volume").unwrap_or(0.0),
            open: parse("open"),
            high: parse("high"),
            low: parse("low"),
            previous_close: parse("prev_close"),
            time_ms: result.get("time").and_then(parse_json_i64).unwrap_or_default(),
            delayed: true,
        })
    }

    /// Fetch delayed historical candles for Kraken Securities/equities from
    /// Kraken Pro's internal equities market-data API. The interval is minutes.
    pub async fn get_equity_history(
        &self,
        symbol: &str,
        interval_minutes: u32,
        since_seconds: Option<i64>,
    ) -> Result<Vec<KrakenEquityBar>, String> {
        let symbol = symbol
            .trim()
            .trim_end_matches(".EQ")
            .replace('/', "")
            .to_ascii_uppercase();
        if symbol.is_empty() {
            return Err("Kraken equity history: empty symbol".to_string());
        }
        let interval = interval_minutes.max(1).to_string();
        let since = since_seconds.unwrap_or(0).max(0).to_string();
        let url = format!("{KRAKEN_INTERNAL_API_BASE_URL}/markets/{symbol}/ticker/history");
        let resp = self
            .client
            .get(&url)
            .header("Accept", "application/json")
            .header("Referer", "https://pro.kraken.com/app/")
            .query(&[
                ("interval", interval.as_str()),
                ("since", since.as_str()),
                ("asset_class", "equity"),
                ("include_range", "market-hours"),
                ("delayed", "true"),
            ])
            .send()
            .await
            .map_err(|e| format!("Kraken equity history request failed: {e}"))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!(
                "Kraken equity history request failed: HTTP {status}: {body}"
            ));
        }
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Kraken equity history parse failed: {e}"))?;
        if let Some(errors) = body.get("errors").and_then(|v| v.as_array()) {
            if !errors.is_empty() {
                let msg = errors
                    .iter()
                    .map(|e| {
                        e.get("msg")
                            .or_else(|| e.get("type"))
                            .or_else(|| e.get("error"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown Kraken equity history error")
                            .to_string()
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                return Err(format!("Kraken equity history error: {msg}"));
            }
        }
        let rows = body
            .get("result")
            .and_then(|v| v.get("data"))
            .and_then(|v| v.as_array())
            .ok_or_else(|| format!("Kraken equity history: no data for {symbol}"))?;
        let mut bars = Vec::with_capacity(rows.len());
        for row in rows {
            let time_s = row.get("time").and_then(parse_json_i64).unwrap_or_default();
            let Some(open) = row.get("open").and_then(parse_json_number) else { continue; };
            let Some(high) = row.get("high").and_then(parse_json_number) else { continue; };
            let Some(low) = row.get("low").and_then(parse_json_number) else { continue; };
            let Some(close) = row.get("close").and_then(parse_json_number) else { continue; };
            if time_s <= 0 || !(open > 0.0 && high > 0.0 && low > 0.0 && close > 0.0) {
                continue;
            }
            bars.push(KrakenEquityBar {
                time_ms: time_s.saturating_mul(1000),
                open,
                high,
                low,
                close,
                volume: row.get("volume").and_then(parse_json_number).unwrap_or(0.0),
            });
        }
        bars.sort_by_key(|bar| bar.time_ms);
        bars.dedup_by_key(|bar| bar.time_ms);
        Ok(bars)
    }

    /// Generate the next nonce (monotonically increasing).
    fn next_nonce(&self) -> u64 {
        self.nonce.fetch_add(1, Ordering::SeqCst)
    }

    /// Compute Kraken HMAC-SHA512 signature for a private API request.
    ///
    /// Algorithm:
    ///   message  = nonce + post_data
    ///   hash     = SHA256(message)
    ///   hmac_in  = path_bytes ++ hash
    ///   sig      = HMAC-SHA512(hmac_in, base64_decode(api_secret))
    ///   result   = base64_encode(sig)
    pub fn sign_request(&self, path: &str, nonce: u64, post_data: &str) -> String {
        let message = format!("{}{}", nonce, post_data);
        let sha256_hash = Sha256::digest(message.as_bytes());

        let mut hmac_input = Vec::with_capacity(path.len() + 32);
        hmac_input.extend_from_slice(path.as_bytes());
        hmac_input.extend_from_slice(&sha256_hash);

        let secret_bytes = BASE64.decode(&self.api_secret).unwrap_or_default();

        let mut mac = match HmacSha512::new_from_slice(&secret_bytes) {
            Ok(m) => m,
            Err(_) => return String::new(), // HMAC init failed — return empty signature
        };
        mac.update(&hmac_input);
        let result = mac.finalize().into_bytes();

        BASE64.encode(result)
    }

    /// Execute a signed POST request to a Kraken private endpoint.
    async fn private_post(
        &self,
        path: &str,
        extra_params: &[(&str, &str)],
    ) -> Result<serde_json::Value, String> {
        let params = extra_params
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect::<Vec<_>>();
        self.private_post_owned(path, &params).await
    }

    /// Execute a signed POST request to a Kraken private endpoint with owned
    /// parameter pairs. This is the escape hatch for the full Spot REST surface:
    /// callers can pass any current Kraken parameter while keeping nonce,
    /// signing, encoding, and error handling centralized.
    pub async fn private_post_owned(
        &self,
        path: &str,
        extra_params: &[(String, String)],
    ) -> Result<serde_json::Value, String> {
        if !self.is_authenticated() {
            return Err("Kraken API credentials not configured".to_string());
        }

        let cost = kraken_private_rest_counter_cost(path);
        let max_attempts = if cost > 0.0 {
            KRAKEN_PRIVATE_REST_MAX_ATTEMPTS
        } else {
            1
        };

        for attempt in 0..max_attempts {
            self.private_limiter.wait(cost).await;

            let nonce = self.next_nonce();
            let nonce_str = nonce.to_string();

            let mut params = Vec::with_capacity(extra_params.len() + 1);
            params.push(("nonce".to_string(), nonce_str));
            params.extend_from_slice(extra_params);
            let post_data = encode_form_params(&params);

            let signature = self.sign_request(path, nonce, &post_data);
            let url = format!("{}{}", KRAKEN_BASE_URL, path);

            let resp = self
                .client
                .post(&url)
                .header("API-Key", self.api_key.as_str())
                .header("API-Sign", &signature)
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(post_data)
                .send()
                .await
                .map_err(|e| format!("Kraken request failed: {}", e))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                let body = sanitize_api_error_body(&body);
                let err = if body.is_empty() {
                    format!("Kraken API error {status}")
                } else {
                    format!("Kraken API error {status}: {body}")
                };
                if cost > 0.0 && crate::core::kraken::is_kraken_rate_limit_error(&err) {
                    let wait = self.private_limiter.record_rate_limited(&err).await;
                    if attempt + 1 < max_attempts {
                        tracing::warn!(
                            "Kraken private REST rate-limited on {}; cooling down {}s before retry {}/{}",
                            path,
                            wait.as_secs(),
                            attempt + 2,
                            max_attempts
                        );
                        continue;
                    }
                }
                return Err(err);
            }

            let body: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| format!("Kraken response parse failed: {}", e))?;

            // Kraken returns {"error": [...], "result": {...}}
            if let Some(errors) = body.get("error").and_then(|e| e.as_array()) {
                let errs: Vec<&str> = errors.iter().filter_map(|e| e.as_str()).collect();
                if !errs.is_empty() {
                    let err = format!("Kraken API error: {}", errs.join(", "));
                    if cost > 0.0 && crate::core::kraken::is_kraken_rate_limit_error(&err) {
                        let wait = self.private_limiter.record_rate_limited(&err).await;
                        if attempt + 1 < max_attempts {
                            tracing::warn!(
                                "Kraken private REST rate-limited on {}; cooling down {}s before retry {}/{}",
                                path,
                                wait.as_secs(),
                                attempt + 2,
                                max_attempts
                            );
                            continue;
                        }
                    }
                    return Err(err);
                }
            }

            self.private_limiter.record_success().await;
            return body
                .get("result")
                .cloned()
                .ok_or_else(|| "Kraken response missing 'result' field".to_string());
        }

        Err("Kraken private REST request failed after rate-limit retries".to_string())
    }

    /// Get account balances. Returns asset → balance map.
    pub async fn get_balance(&self) -> Result<HashMap<String, f64>, String> {
        let result = self.private_post("/0/private/Balance", &[]).await?;
        let obj = result
            .as_object()
            .ok_or("Expected object in Balance response")?;

        let mut balances = HashMap::new();
        for (asset, value) in obj {
            if let Some(s) = value.as_str() {
                if let Ok(v) = s.parse::<f64>() {
                    balances.insert(asset.clone(), v);
                }
            }
        }
        Ok(balances)
    }

    /// Get all open orders.
    pub async fn get_open_orders(&self) -> Result<Vec<serde_json::Value>, String> {
        let result = self.private_post("/0/private/OpenOrders", &[]).await?;
        let open = result
            .get("open")
            .and_then(|o| o.as_object())
            .ok_or("Expected 'open' object in OpenOrders response")?;

        let mut orders = Vec::new();
        for (txid, order) in open {
            let mut o = order.clone();
            if let Some(obj) = o.as_object_mut() {
                obj.insert("txid".to_string(), serde_json::Value::String(txid.clone()));
            }
            orders.push(o);
        }
        Ok(orders)
    }

    /// Get all open positions.
    pub async fn get_open_positions(&self) -> Result<Vec<serde_json::Value>, String> {
        let result = self.private_post("/0/private/OpenPositions", &[]).await?;
        let obj = result
            .as_object()
            .ok_or("Expected object in OpenPositions response")?;

        let mut positions = Vec::new();
        for (posid, pos) in obj {
            let mut p = pos.clone();
            if let Some(obj) = p.as_object_mut() {
                obj.insert(
                    "posid".to_string(),
                    serde_json::Value::String(posid.clone()),
                );
            }
            positions.push(p);
        }
        Ok(positions)
    }

    fn parse_f64_field(value: &serde_json::Value) -> f64 {
        value
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .or_else(|| value.as_f64())
            .unwrap_or(0.0)
    }

    fn canonical_pair_forms(pair: &str) -> Vec<String> {
        let raw = Self::normalized_pair_key(pair);
        let mut forms = vec![raw.clone()];
        if let Some(mapped) = crate::core::kraken::to_kraken_pair_lossy(&raw) {
            let mapped = Self::normalized_pair_key(&mapped);
            if !forms.iter().any(|f| f == &mapped) {
                forms.push(mapped);
            }
        }
        forms
    }

    fn normalized_pair_key(pair: &str) -> String {
        crate::core::kraken::normalize_pair_symbol(pair)
    }

    fn display_pair(pair: &str) -> String {
        match Self::normalized_pair_key(pair).as_str() {
            "XBTUSD" => "BTCUSD".to_string(),
            "XDGUSD" => "DOGEUSD".to_string(),
            other => other.to_string(),
        }
    }

    fn display_asset(asset: &str) -> String {
        let raw = asset.trim().to_ascii_uppercase();
        match raw.as_str() {
            "XXBT" | "XBT" => "BTC".to_string(),
            "XXDG" | "XDG" => "DOGE".to_string(),
            "ZUSD" => "USD".to_string(),
            "ZEUR" => "EUR".to_string(),
            "ZGBP" => "GBP".to_string(),
            "ZJPY" => "JPY".to_string(),
            other if other.len() == 4 && (other.starts_with('X') || other.starts_with('Z')) => {
                other[1..].to_string()
            }
            other => other.to_string(),
        }
    }

    fn is_cash_asset(asset: &str) -> bool {
        matches!(
            Self::display_asset(asset).as_str(),
            "USD"
                | "EUR"
                | "GBP"
                | "JPY"
                | "CAD"
                | "AUD"
                | "CHF"
                | "USDT"
                | "USDC"
                | "USDG"
                | "DAI"
                | "PYUSD"
        )
    }

    fn pair_matches(candidate: &str, target: &str) -> bool {
        let candidate = Self::normalized_pair_key(candidate);
        let candidate_base = Self::base_asset_key(&candidate);
        Self::canonical_pair_forms(target).into_iter().any(|form| {
            let form_base = Self::base_asset_key(&form);
            candidate == form
                || candidate.ends_with(&form)
                || form.ends_with(&candidate)
                || (!candidate_base.is_empty() && candidate_base == form_base)
        })
    }

    fn base_asset_key(pair: &str) -> String {
        let normalized = crate::core::kraken::normalize_pair_symbol(pair);
        normalized
            .strip_suffix("USDT")
            .or_else(|| normalized.strip_suffix("USDC"))
            .or_else(|| normalized.strip_suffix("USD"))
            .unwrap_or(normalized.as_str())
            .strip_suffix(".EQ")
            .unwrap_or_else(|| {
                normalized
                    .strip_suffix("USDT")
                    .or_else(|| normalized.strip_suffix("USDC"))
                    .or_else(|| normalized.strip_suffix("USD"))
                    .unwrap_or(normalized.as_str())
            })
            .to_string()
    }

    fn net_position_volume(raw_positions: &[serde_json::Value], pair: &str) -> f64 {
        let mut net_volume = 0.0_f64;
        for pos in raw_positions {
            let pos_pair = pos["pair"].as_str().unwrap_or("");
            if !Self::pair_matches(pos_pair, pair) {
                continue;
            }
            let volume = Self::parse_f64_field(&pos["vol"]);
            let closed = Self::parse_f64_field(&pos["vol_closed"]);
            let open_volume = (volume - closed).max(0.0);
            if open_volume <= 0.0 {
                continue;
            }
            let side = pos["type"].as_str().unwrap_or("");
            if side.eq_ignore_ascii_case("buy") || side.eq_ignore_ascii_case("long") {
                net_volume += open_volume;
            } else {
                net_volume -= open_volume;
            }
        }
        net_volume
    }

    pub fn position_summaries_from_raw(
        raw_positions: &[serde_json::Value],
    ) -> Vec<crate::broker::alpaca::PositionInfo> {
        raw_positions
            .iter()
            .filter_map(|pos| {
                let pair = pos["pair"].as_str().unwrap_or("");
                if pair.is_empty() {
                    return None;
                }
                let volume = Self::parse_f64_field(&pos["vol"]);
                let closed = Self::parse_f64_field(&pos["vol_closed"]);
                let open_volume = (volume - closed).max(0.0);
                if open_volume <= 0.0 {
                    return None;
                }
                let cost = Self::parse_f64_field(&pos["cost"]);
                let value = Self::parse_f64_field(&pos["value"]);
                let net = Self::parse_f64_field(&pos["net"]);
                let avg_entry = if volume > 0.0 { cost / volume } else { 0.0 };
                let side = if pos["type"]
                    .as_str()
                    .unwrap_or("")
                    .eq_ignore_ascii_case("sell")
                {
                    "short"
                } else {
                    "long"
                };
                Some(crate::broker::alpaca::PositionInfo {
                    symbol: Self::display_pair(pair),
                    qty: open_volume,
                    side: side.to_string(),
                    avg_entry_price: avg_entry,
                    market_value: if value > 0.0 { value } else { cost + net },
                    unrealized_pl: net,
                    asset_class: "crypto".to_string(),
                    asset_id: pos["posid"].as_str().unwrap_or("").to_string(),
                })
            })
            .collect()
    }

    fn equity_balance_ticker(asset: &str) -> Option<String> {
        let display = Self::display_asset(asset);
        let ticker = display.strip_suffix(".EQ")?.trim();
        (!ticker.is_empty()).then(|| ticker.to_string())
    }

    fn is_equity_balance_asset(asset: &str) -> bool {
        Self::equity_balance_ticker(asset).is_some()
    }

    pub fn equity_position_summaries_from_balances(
        balances: &[(String, f64)],
    ) -> Vec<crate::broker::alpaca::PositionInfo> {
        balances
            .iter()
            .filter_map(|(asset, qty)| {
                if !qty.is_finite()
                    || *qty <= 0.0
                    || Self::is_cash_asset(asset)
                    || !Self::is_equity_balance_asset(asset)
                {
                    return None;
                }
                let Some(ticker) = Self::equity_balance_ticker(asset) else {
                    return None;
                };
                if ticker.is_empty() {
                    return None;
                }
                Some(crate::broker::alpaca::PositionInfo {
                    symbol: ticker,
                    qty: *qty,
                    side: "long".to_string(),
                    avg_entry_price: 0.0,
                    market_value: 0.0,
                    unrealized_pl: 0.0,
                    asset_class: "stock".to_string(),
                    asset_id: format!("equity_balance:{asset}"),
                })
            })
            .collect()
    }

    pub async fn get_position_summaries(
        &self,
    ) -> Result<Vec<crate::broker::alpaca::PositionInfo>, String> {
        let positions = self.get_open_positions().await?;
        Ok(Self::position_summaries_from_raw(&positions))
    }

    pub async fn get_all_position_summaries(
        &self,
    ) -> Result<Vec<crate::broker::alpaca::PositionInfo>, String> {
        let mut out = self.get_position_summaries().await.unwrap_or_default();
        if let Ok(balance_map) = self.get_balance().await {
            let mut balances: Vec<(String, f64)> = balance_map.into_iter().collect();
            balances.sort_by(|a, b| a.0.cmp(&b.0));
            out.extend(Self::equity_position_summaries_from_balances(&balances));
        }
        out.sort_by(|a, b| a.symbol.cmp(&b.symbol));
        Ok(out)
    }

    async fn cancel_live_exit_orders_for_pair(
        &self,
        pair: &str,
        exit_side: &str,
    ) -> Result<usize, String> {
        let orders = self.get_open_orders().await?;
        let mut cancelled = 0usize;
        for order in orders {
            let txid = order["txid"].as_str().unwrap_or("").to_string();
            let descr = &order["descr"];
            let order_pair = descr["pair"].as_str().unwrap_or("");
            let order_side = descr["type"].as_str().unwrap_or("");
            if txid.is_empty()
                || !Self::pair_matches(order_pair, pair)
                || !order_side.eq_ignore_ascii_case(exit_side)
            {
                continue;
            }
            self.cancel_order(&txid)
                .await
                .map_err(|e| format!("Cancel open exit order {txid} for {pair} failed: {e}"))?;
            cancelled += 1;
        }
        Ok(cancelled)
    }

    pub async fn sync_position_exits(
        &self,
        pair: &str,
        sl_price: Option<f64>,
        tp_price: Option<f64>,
    ) -> Result<String, String> {
        let positions = self.get_open_positions().await?;
        let net_volume = Self::net_position_volume(&positions, pair);

        if net_volume.abs() <= f64::EPSILON {
            return Err(format!("No open Kraken position found for {pair}"));
        }

        let exit_side = if net_volume > 0.0 { "sell" } else { "buy" };
        let order_pair =
            crate::core::kraken::to_kraken_pair_lossy(pair).unwrap_or_else(|| pair.to_string());
        let cancelled = self
            .cancel_live_exit_orders_for_pair(pair, exit_side)
            .await?;
        let mut placements = Vec::new();
        let qty_abs = net_volume.abs();
        if let Some(sl) = sl_price {
            let mut order = KrakenOrderRequest::basic(&order_pair, exit_side, "stop-loss", qty_abs)
                .with_price(sl);
            order.reduce_only = true;
            self.place_order_request(&order).await?;
            placements.push(format!("SL {}", sl));
        }
        if let Some(tp) = tp_price {
            let mut order =
                KrakenOrderRequest::basic(&order_pair, exit_side, "take-profit", qty_abs)
                    .with_price(tp);
            order.reduce_only = true;
            self.place_order_request(&order).await?;
            placements.push(format!("TP {}", tp));
        }
        if placements.is_empty() {
            placements.push("cleared exits".to_string());
        } else if sl_price.is_some() && tp_price.is_some() {
            placements
                .push("broker-native link unavailable; exits placed independently".to_string());
        }

        Ok(format!(
            "{} for {} {} {} (cancelled {} existing exit order(s))",
            placements.join(", "),
            exit_side,
            qty_abs,
            order_pair,
            cancelled
        ))
    }

    pub async fn close_position(
        &self,
        pair: &str,
        volume: Option<f64>,
    ) -> Result<serde_json::Value, String> {
        let positions = self.get_open_positions().await?;
        let net_volume = Self::net_position_volume(&positions, pair);
        if net_volume.abs() <= f64::EPSILON {
            return Err(format!("No open Kraken position found for {pair}"));
        }

        let close_side = if net_volume > 0.0 { "sell" } else { "buy" };
        let qty_abs = volume.unwrap_or(net_volume.abs()).min(net_volume.abs());
        if qty_abs <= f64::EPSILON {
            return Err(format!("Close size for {pair} is zero"));
        }
        let _ = self
            .cancel_live_exit_orders_for_pair(pair, close_side)
            .await;
        let order_pair =
            crate::core::kraken::to_kraken_pair_lossy(pair).unwrap_or_else(|| pair.to_string());
        let mut order = KrakenOrderRequest::basic(&order_pair, close_side, "market", qty_abs);
        order.reduce_only = true;
        self.place_order_request(&order).await
    }

    pub async fn close_all_positions(&self) -> Result<usize, String> {
        let positions = self.get_open_positions().await?;
        let summaries = Self::position_summaries_from_raw(&positions);
        let mut closed = 0usize;
        for pos in summaries {
            self.close_position(&pos.symbol, None).await?;
            closed += 1;
        }
        Ok(closed)
    }

    /// Place an order on Kraken.
    ///
    /// - `pair`: trading pair (e.g. "XXBTZUSD")
    /// - `side`: "buy" or "sell"
    /// - `order_type`: "market", "limit", "stop-loss", "take-profit", etc.
    /// - `volume`: order size
    /// - `price`: required for limit/stop-loss/take-profit orders
    pub async fn place_order(
        &self,
        pair: &str,
        side: &str,
        order_type: &str,
        volume: f64,
        price: Option<f64>,
    ) -> Result<serde_json::Value, String> {
        self.place_order_with_leverage(pair, side, order_type, volume, price, None)
            .await
    }

    /// Place an order with optional leverage (margin trading).
    /// `leverage`: e.g. Some("2:1"), Some("3:1"), Some("5:1"). None = no margin.
    pub async fn place_order_with_leverage(
        &self,
        pair: &str,
        side: &str,
        order_type: &str,
        volume: f64,
        price: Option<f64>,
        leverage: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        let mut order = KrakenOrderRequest::basic(pair, side, order_type, volume);
        if let Some(p) = price {
            order.price = Some(format_f64_param(p));
        }
        if let Some(lev) = leverage {
            order.leverage = Some(lev.to_string());
        }
        self.place_order_request(&order).await
    }

    /// Place a full Kraken Spot REST AddOrder request.
    pub async fn place_order_request(
        &self,
        order: &KrakenOrderRequest,
    ) -> Result<serde_json::Value, String> {
        order.validate()?;
        self.private_post_owned("/0/private/AddOrder", &order.to_params())
            .await
    }

    /// Add a batch of orders using Kraken's `/private/AddOrderBatch`.
    ///
    /// Kraken currently requires 2-15 orders, all for one pair. `orders_json`
    /// is passed through so callers can track new Kraken fields without waiting
    /// for a TyphooN release.
    pub async fn add_order_batch(
        &self,
        pair: &str,
        orders_json: &str,
        validate: bool,
    ) -> Result<serde_json::Value, String> {
        let params = vec![
            ("pair".to_string(), pair.to_string()),
            ("orders".to_string(), orders_json.to_string()),
            ("validate".to_string(), validate.to_string()),
        ];
        self.private_post_owned("/0/private/AddOrderBatch", &params)
            .await
    }

    /// Amend an order in place using Kraken's `/private/AmendOrder`.
    pub async fn amend_order(
        &self,
        params: &[(String, String)],
    ) -> Result<serde_json::Value, String> {
        self.private_post_owned("/0/private/AmendOrder", params)
            .await
    }

    /// Edit an order using Kraken's legacy cancel-and-replace endpoint.
    pub async fn edit_order(
        &self,
        params: &[(String, String)],
    ) -> Result<serde_json::Value, String> {
        self.private_post_owned("/0/private/EditOrder", params)
            .await
    }

    /// Cancel a batch of open orders by txid/userref/cl_ord_id.
    pub async fn cancel_order_batch(&self, txids: &[String]) -> Result<serde_json::Value, String> {
        let txid = txids.join(",");
        self.private_post_owned("/0/private/CancelOrderBatch", &[("txid".to_string(), txid)])
            .await
    }

    /// Configure Kraken's dead man's switch. `timeout = 0` disables it.
    pub async fn cancel_all_orders_after(
        &self,
        timeout_secs: u32,
    ) -> Result<serde_json::Value, String> {
        self.private_post_owned(
            "/0/private/CancelAllOrdersAfter",
            &[("timeout".to_string(), timeout_secs.to_string())],
        )
        .await
    }

    /// Cancel an order by transaction ID.
    pub async fn cancel_order(&self, txid: &str) -> Result<serde_json::Value, String> {
        self.private_post("/0/private/CancelOrder", &[("txid", txid)])
            .await
    }

    /// Cancel all open orders.
    pub async fn cancel_all_orders(&self) -> Result<serde_json::Value, String> {
        self.private_post("/0/private/CancelAll", &[]).await
    }

    pub async fn get_extended_balance(&self) -> Result<serde_json::Value, String> {
        self.private_post("/0/private/BalanceEx", &[]).await
    }

    pub async fn get_trade_balance(
        &self,
        asset: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        match asset {
            Some(asset) => {
                self.private_post("/0/private/TradeBalance", &[("asset", asset)])
                    .await
            }
            None => self.private_post("/0/private/TradeBalance", &[]).await,
        }
    }

    pub async fn get_closed_orders(
        &self,
        params: &[(String, String)],
    ) -> Result<serde_json::Value, String> {
        self.private_post_owned("/0/private/ClosedOrders", params)
            .await
    }

    pub async fn query_orders(
        &self,
        params: &[(String, String)],
    ) -> Result<serde_json::Value, String> {
        self.private_post_owned("/0/private/QueryOrders", params)
            .await
    }

    pub async fn get_order_amends(
        &self,
        params: &[(String, String)],
    ) -> Result<serde_json::Value, String> {
        self.private_post_owned("/0/private/OrderAmends", params)
            .await
    }

    pub async fn get_trades_history(
        &self,
        params: &[(String, String)],
    ) -> Result<serde_json::Value, String> {
        self.private_post_owned("/0/private/TradesHistory", params)
            .await
    }

    pub async fn query_trades(
        &self,
        params: &[(String, String)],
    ) -> Result<serde_json::Value, String> {
        self.private_post_owned("/0/private/QueryTrades", params)
            .await
    }

    pub async fn get_ledgers(
        &self,
        params: &[(String, String)],
    ) -> Result<serde_json::Value, String> {
        self.private_post_owned("/0/private/Ledgers", params).await
    }

    pub async fn query_ledgers(
        &self,
        params: &[(String, String)],
    ) -> Result<serde_json::Value, String> {
        self.private_post_owned("/0/private/QueryLedgers", params)
            .await
    }

    pub async fn get_trade_volume(
        &self,
        params: &[(String, String)],
    ) -> Result<serde_json::Value, String> {
        self.private_post_owned("/0/private/TradeVolume", params)
            .await
    }

    pub async fn get_api_key_info(&self) -> Result<serde_json::Value, String> {
        self.private_post("/0/private/GetApiKeyInfo", &[]).await
    }

    pub async fn get_websockets_token(&self) -> Result<serde_json::Value, String> {
        self.private_post("/0/private/GetWebSocketsToken", &[])
            .await
    }

    pub async fn get_websockets_token_string(&self) -> Result<String, String> {
        let value = self.get_websockets_token().await?;
        value
            .get("token")
            .and_then(|token| token.as_str())
            .filter(|token| !token.is_empty())
            .map(str::to_string)
            .ok_or_else(|| "Kraken WebSocket token response missing token".to_string())
    }

    /// Fetch a public Kraken order-book snapshot and normalize it to TyphooN's
    /// common DOM shape.
    pub async fn get_orderbook_snapshot(
        &self,
        symbol: &str,
        count: usize,
    ) -> Result<serde_json::Value, String> {
        let pair = crate::core::kraken::to_kraken_pair_lossy(symbol)
            .ok_or_else(|| format!("Kraken orderbook: unsupported pair {symbol}"))?;
        let url = format!("{}/0/public/Depth", KRAKEN_BASE_URL);
        let count = count.min(500).to_string();
        let resp = self
            .client
            .get(&url)
            .query(&[("pair", pair.as_str()), ("count", count.as_str())])
            .send()
            .await
            .map_err(|e| format!("Kraken orderbook request failed: {e}"))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!(
                "Kraken orderbook request failed: HTTP {status}: {body}"
            ));
        }
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Kraken orderbook parse failed: {e}"))?;
        if let Some(errors) = body.get("error").and_then(|v| v.as_array())
            && !errors.is_empty()
        {
            let msg = errors
                .iter()
                .filter_map(|e| e.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(format!("Kraken orderbook error: {msg}"));
        }
        let result = body
            .get("result")
            .and_then(|v| v.as_object())
            .ok_or_else(|| "Kraken orderbook response missing result".to_string())?;
        let book = result
            .values()
            .next()
            .ok_or_else(|| "Kraken orderbook response missing book".to_string())?;
        let parse_side = |side: &str| -> Vec<serde_json::Value> {
            book.get(side)
                .and_then(|v| v.as_array())
                .map(|levels| {
                    levels
                        .iter()
                        .filter_map(|entry| {
                            let arr = entry.as_array()?;
                            let price = Self::parse_f64_field(arr.first()?);
                            let size = Self::parse_f64_field(arr.get(1)?);
                            if price > 0.0 && size > 0.0 {
                                Some(serde_json::json!({
                                    "price": price,
                                    "size": size,
                                }))
                            } else {
                                None
                            }
                        })
                        .collect()
                })
                .unwrap_or_default()
        };
        Ok(serde_json::json!({
            "source": "kraken",
            "symbol": crate::core::kraken::normalize_pair_symbol(symbol),
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "bids": parse_side("bids"),
            "asks": parse_side("asks"),
        }))
    }

    /// Get all tradeable asset pairs (public endpoint, no auth required).
    /// Returns list of (pair_name, wsname/altname) tuples.
    pub async fn get_tradeable_pairs(&self) -> Result<Vec<(String, String)>, String> {
        let url = format!("{}/0/public/AssetPairs", KRAKEN_BASE_URL);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Kraken AssetPairs request failed: {}", e))?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Kraken AssetPairs parse failed: {}", e))?;

        if let Some(errors) = body.get("error").and_then(|e| e.as_array()) {
            let errs: Vec<&str> = errors.iter().filter_map(|e| e.as_str()).collect();
            if !errs.is_empty() {
                return Err(format!("Kraken API error: {}", errs.join(", ")));
            }
        }

        let result = body
            .get("result")
            .and_then(|r| r.as_object())
            .ok_or("Expected object in AssetPairs response")?;

        let mut pairs = Vec::new();
        for (name, info) in result {
            // Prefer wsname (WebSocket name) for display, fall back to altname
            let display = info
                .get("wsname")
                .or_else(|| info.get("altname"))
                .and_then(|v| v.as_str())
                .unwrap_or(name)
                .to_string();
            pairs.push((name.clone(), display));
        }
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(pairs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_broker() {
        let broker = KrakenBroker::new(String::new(), String::new());
        assert!(!broker.is_authenticated());
        assert!(broker.api_key.is_empty());
    }

    #[test]
    fn is_authenticated_false_when_empty() {
        let broker = KrakenBroker::new(String::new(), String::new());
        assert!(!broker.is_authenticated());
    }

    #[test]
    fn is_authenticated_true_when_set() {
        let broker = KrakenBroker::new("key".into(), "c2VjcmV0".into());
        assert!(broker.is_authenticated());
    }

    #[test]
    fn sign_request_produces_nonempty_base64() {
        // Use a known base64-encoded secret
        let secret = BASE64.encode(b"test_secret_key_1234567890");
        let broker = KrakenBroker::new("test_key".into(), secret);
        let sig = broker.sign_request("/0/private/Balance", 1234567890, "nonce=1234567890");
        assert!(!sig.is_empty());
        // Verify it's valid base64
        assert!(BASE64.decode(&sig).is_ok());
    }

    #[test]
    fn sign_request_deterministic() {
        let secret = BASE64.encode(b"deterministic_secret");
        let broker = KrakenBroker::new("key".into(), secret);
        let sig1 = broker.sign_request("/0/private/Balance", 100, "nonce=100");
        let sig2 = broker.sign_request("/0/private/Balance", 100, "nonce=100");
        assert_eq!(sig1, sig2);
    }

    #[test]
    fn sign_request_matches_kraken_doc_vector() {
        let broker = KrakenBroker::new(
            "key".into(),
            "kQH5HW/8p1uGOVjbgWA7FunAmGO8lsSUXNsu3eow76sz84Q18fWxnyRzBHCd3pd5nE9qa99HAZtuZuj6F1huXg=="
                .into(),
        );
        let post_data =
            "nonce=1616492376594&ordertype=limit&pair=XBTUSD&price=37500&type=buy&volume=1.25";
        let sig = broker.sign_request("/0/private/AddOrder", 1616492376594, post_data);
        assert_eq!(
            sig,
            "4/dpxb3iT4tp/ZCVEwSnEsLxx0bqyhLpdfOpc6fn7OR8+UClSV5n9E6aSS8MPtnRfp32bAb0nmbRn6H8ndwLUQ=="
        );
    }

    #[test]
    fn encode_form_params_percent_encodes_order_fields() {
        let params = vec![
            ("price".to_string(), "+2%".to_string()),
            (
                "close[ordertype]".to_string(),
                "stop-loss-limit".to_string(),
            ),
            ("close[price2]".to_string(), "28400".to_string()),
        ];
        assert_eq!(
            encode_form_params(&params),
            "price=%2B2%25&close%5Bordertype%5D=stop-loss-limit&close%5Bprice2%5D=28400"
        );
    }

    #[test]
    fn kraken_private_rest_counter_cost_matches_rate_limit_docs() {
        assert_eq!(kraken_private_rest_counter_cost("/0/private/Balance"), 1.0);
        assert_eq!(
            kraken_private_rest_counter_cost("/0/private/OpenOrders"),
            1.0
        );
        assert_eq!(kraken_private_rest_counter_cost("/0/private/Ledgers"), 4.0);
        assert_eq!(
            kraken_private_rest_counter_cost("/0/private/QueryTrades"),
            4.0
        );
        assert_eq!(kraken_private_rest_counter_cost("/0/private/AddOrder"), 0.0);
        assert_eq!(
            kraken_private_rest_counter_cost("/0/private/CancelOrder"),
            0.0
        );
    }

    #[test]
    fn trades_history_result_parses_unwrapped_result_and_count() {
        let payload = serde_json::json!({
            "count": 123,
            "trades": {
                "T-HRTX-1": {
                    "ordertxid": "O-HRTX-1",
                    "pair": "HRTX.EQUSD",
                    "time": 1778841060.0,
                    "type": "buy",
                    "ordertype": "market",
                    "price": "0.88",
                    "cost": "230.56",
                    "fee": "0.10",
                    "vol": "262.0",
                    "margin": "0"
                }
            }
        });

        let (trades, count) = parse_trades_history_result(&payload).unwrap();
        assert_eq!(count, Some(123));
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].pair, "HRTX.EQUSD");
        assert_eq!(trades[0].side, "buy");
        assert_eq!(trades[0].price, 0.88);
        assert_eq!(trades[0].vol, 262.0);
    }

    #[test]
    fn equity_position_summaries_convert_only_equity_balances() {
        let balances = vec![
            ("XXBT".to_string(), 0.25),
            ("BABY".to_string(), 123.0),
            ("XXMR".to_string(), 1.5),
            ("HRTX.EQ".to_string(), 7.0),
            ("ZUSD".to_string(), 1000.0),
            ("USDT".to_string(), 50.0),
        ];
        let positions = KrakenBroker::equity_position_summaries_from_balances(&balances);

        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].symbol, "HRTX");
        assert_eq!(positions[0].qty, 7.0);
        assert_eq!(positions[0].asset_class, "stock");
        assert_eq!(positions[0].side, "long");
    }

    #[test]
    fn kraken_order_request_builds_full_add_order_params() {
        let mut order = KrakenOrderRequest::basic("XBTUSD", "BUY", "stop_loss_limit", 1.25)
            .with_price(37000.0)
            .with_price2(36950.0);
        order.leverage = Some("2".to_string());
        order.reduce_only = true;
        order.oflags = vec!["post".to_string(), "fciq".to_string()];
        order.time_in_force = Some("gtd".to_string());
        order.expire_time = Some("+60".to_string());
        order.client_order_id = Some("typhoon-kr-0001".to_string());
        order.close = Some(KrakenConditionalClose {
            order_type: "take_profit".to_string(),
            price: Some("39000".to_string()),
            price2: None,
        });

        order.validate().unwrap();
        let params = order.to_params();
        assert!(params.contains(&("pair".to_string(), "XBTUSD".to_string())));
        assert!(params.contains(&("type".to_string(), "buy".to_string())));
        assert!(params.contains(&("ordertype".to_string(), "stop-loss-limit".to_string())));
        assert!(params.contains(&("price".to_string(), "37000".to_string())));
        assert!(params.contains(&("price2".to_string(), "36950".to_string())));
        assert!(params.contains(&("reduce_only".to_string(), "true".to_string())));
        assert!(params.contains(&("oflags".to_string(), "post,fciq".to_string())));
        assert!(params.contains(&("timeinforce".to_string(), "GTD".to_string())));
        assert!(params.contains(&("close[ordertype]".to_string(), "take-profit".to_string())));
    }

    #[test]
    fn kraken_order_request_rejects_missing_stop_limit_price2() {
        let order =
            KrakenOrderRequest::basic("XBTUSD", "sell", "stop-loss-limit", 1.0).with_price(37000.0);
        let err = order.validate().unwrap_err();
        assert!(err.contains("price2"));
    }

    #[test]
    fn kraken_iceberg_uses_rest_displayvol() {
        let order = KrakenOrderRequest::basic("XBTUSD", "sell", "iceberg", 10.0)
            .with_price(50000.0)
            .with_display_volume(1.0);
        order.validate().unwrap();
        let params = order.to_params();
        assert!(params.contains(&("ordertype".to_string(), "limit".to_string())));
        assert!(params.contains(&("displayvol".to_string(), "1".to_string())));
    }

    #[test]
    fn kraken_settle_position_allows_zero_volume() {
        let mut order = KrakenOrderRequest::basic("XBTUSD", "buy", "settle-position", 0.0);
        order.leverage = Some("5".to_string());
        order.validate().unwrap();
        let params = order.to_params();
        assert!(params.contains(&("ordertype".to_string(), "settle-position".to_string())));
        assert!(params.contains(&("volume".to_string(), "0".to_string())));
    }

    #[test]
    fn nonce_increments() {
        let broker = KrakenBroker::new(String::new(), String::new());
        let n1 = broker.next_nonce();
        let n2 = broker.next_nonce();
        assert!(n2 > n1);
    }

    #[test]
    fn new_broker_has_empty_api_key() {
        let broker = KrakenBroker::new(String::new(), String::new());
        assert!(broker.api_key.is_empty());
    }

    #[test]
    fn new_broker_with_credentials() {
        let broker = KrakenBroker::new("my_key".to_string(), "my_secret".to_string());
        assert_eq!(broker.api_key.as_str(), "my_key");
        assert_eq!(broker.api_secret.as_str(), "my_secret");
    }

    #[test]
    fn pair_matches_normalizes_btc_aliases() {
        assert!(KrakenBroker::pair_matches("XXBTZUSD", "BTCUSD"));
        assert!(KrakenBroker::pair_matches("POMZUSD", "POMUSD"));
        assert!(KrakenBroker::pair_matches("HRTXZUSD", "HRTX"));
        assert!(KrakenBroker::pair_matches("XBTUSD", "BTC/USD"));
        assert!(KrakenBroker::pair_matches("XDGUSD", "DOGEUSD"));
        assert!(!KrakenBroker::pair_matches("ETHUSD", "SOLUSD"));
    }

    #[test]
    fn position_summaries_from_raw_normalizes_display_pairs() {
        let raw = vec![
            serde_json::json!({
                "pair": "XXBTZUSD",
                "type": "buy",
                "vol": "0.75",
                "vol_closed": "0.25",
                "cost": "30000",
                "value": "31000",
                "net": "1000",
                "posid": "abc123"
            }),
            serde_json::json!({
                "pair": "XDGUSD",
                "type": "sell",
                "vol": "1000",
                "vol_closed": "400",
                "cost": "60",
                "value": "55",
                "net": "-5",
                "posid": "doge456"
            }),
        ];
        let summaries = KrakenBroker::position_summaries_from_raw(&raw);
        assert_eq!(summaries.len(), 2);
        assert_eq!(summaries[0].symbol, "BTCUSD");
        assert_eq!(summaries[0].side, "long");
        assert!((summaries[0].qty - 0.5).abs() < f64::EPSILON);
        assert_eq!(summaries[1].symbol, "DOGEUSD");
        assert_eq!(summaries[1].side, "short");
        assert!((summaries[1].qty - 600.0).abs() < f64::EPSILON);
    }

    #[test]
    fn net_position_volume_offsets_long_and_short() {
        let raw = vec![
            serde_json::json!({
                "pair": "XXBTZUSD",
                "type": "buy",
                "vol": "1.5",
                "vol_closed": "0.25"
            }),
            serde_json::json!({
                "pair": "XBTUSD",
                "type": "sell",
                "vol": "0.5",
                "vol_closed": "0.1"
            }),
        ];
        let net = KrakenBroker::net_position_volume(&raw, "BTCUSD");
        assert!((net - 0.85).abs() < 1e-9);
    }

    #[tokio::test]
    #[ignore] // Requires network access — run with `cargo test -- --ignored`
    async fn get_tradeable_pairs_public() {
        let broker = KrakenBroker::new(String::new(), String::new());
        let pairs = broker.get_tradeable_pairs().await.unwrap();
        assert!(!pairs.is_empty());
        assert!(
            pairs
                .iter()
                .any(|(name, _)| name.contains("BTC") || name.contains("XBT"))
        );
    }
}

// ============================================================================
// Typed Models for Kraken Trading
// ============================================================================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KrakenTrade {
    pub trade_id: String,
    pub ordertxid: String,
    pub pair: String,
    pub time: f64,
    pub side: String,      // "buy" or "sell"
    pub ordertype: String, // "market", "limit", etc.
    pub price: f64,
    pub cost: f64,
    pub fee: f64,
    pub vol: f64,
    pub margin: f64,
    pub misc: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KrakenOrder {
    pub txid: String,
    pub refid: Option<String>,
    pub userref: Option<i64>,
    pub status: String,
    pub opentm: f64,
    pub starttm: Option<f64>,
    pub expiretm: Option<f64>,
    pub pair: String,
    pub r#type: String, // "buy" or "sell"
    pub ordertype: String,
    pub price: f64,
    pub price2: Option<f64>,
    pub vol: f64,
    pub vol_exec: f64,
    pub cost: f64,
    pub fee: f64,
    pub stopprice: Option<f64>,
    pub limitprice: Option<f64>,
    pub misc: Option<String>,
    pub trades: Vec<String>,
}

impl KrakenBroker {
    /// Fetch trade history with parsed results.
    pub async fn get_trades_history_parsed(
        &self,
        start: Option<i64>,
        end: Option<i64>,
        ofs: Option<u64>,
    ) -> Result<Vec<KrakenTrade>, String> {
        let mut params = Vec::new();
        if let Some(s) = start {
            params.push(("start".to_string(), s.to_string()));
        }
        if let Some(e) = end {
            params.push(("end".to_string(), e.to_string()));
        }
        if let Some(o) = ofs {
            params.push(("ofs".to_string(), o.to_string()));
        }

        let (trades, _) = self.get_trades_history_page_parsed(start, end, ofs).await?;
        Ok(trades)
    }

    /// Fetch one Kraken trade-history page with parsed results and Kraken's total count.
    pub async fn get_trades_history_page_parsed(
        &self,
        start: Option<i64>,
        end: Option<i64>,
        ofs: Option<u64>,
    ) -> Result<(Vec<KrakenTrade>, Option<u64>), String> {
        let mut params = Vec::new();
        if let Some(s) = start {
            params.push(("start".to_string(), s.to_string()));
        }
        if let Some(e) = end {
            params.push(("end".to_string(), e.to_string()));
        }
        if let Some(o) = ofs {
            params.push(("ofs".to_string(), o.to_string()));
        }

        let resp = self.get_trades_history(&params).await?;
        parse_trades_history_result(&resp)
    }

    /// Fetch all Kraken trade-history pages available from the private REST API.
    pub async fn get_all_trades_history_parsed(
        &self,
        start: Option<i64>,
        end: Option<i64>,
    ) -> Result<Vec<KrakenTrade>, String> {
        const MAX_TRADES: usize = 20_000;
        const MAX_PAGES: u64 = 400;

        let mut all = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let mut ofs = 0_u64;

        for _ in 0..MAX_PAGES {
            let (page, count) = self
                .get_trades_history_page_parsed(start, end, Some(ofs))
                .await?;
            if page.is_empty() {
                break;
            }

            let page_len = page.len() as u64;
            for trade in page {
                let key = if trade.trade_id.is_empty() {
                    format!(
                        "{}:{}:{}:{}",
                        trade.ordertxid, trade.pair, trade.time, trade.vol
                    )
                } else {
                    trade.trade_id.clone()
                };
                if seen.insert(key) {
                    all.push(trade);
                }
            }

            if all.len() >= MAX_TRADES {
                break;
            }
            ofs += page_len;
            if let Some(total) = count {
                if ofs >= total {
                    break;
                }
            }
        }

        all.sort_by(|a, b| {
            b.time
                .partial_cmp(&a.time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        all.truncate(MAX_TRADES);
        Ok(all)
    }

    /// Fetch currently open Kraken orders with parsed typed results.
    pub async fn get_open_orders_parsed(&self) -> Result<Vec<KrakenOrder>, String> {
        let result = self.private_post("/0/private/OpenOrders", &[]).await?;
        let open = result
            .get("open")
            .and_then(|o| o.as_object())
            .ok_or_else(|| "Kraken OpenOrders missing open object".to_string())?;

        let mut orders: Vec<KrakenOrder> = open
            .iter()
            .filter_map(|(txid, order)| {
                order
                    .as_object()
                    .map(|obj| kraken_order_from_object(txid, obj))
            })
            .collect();
        orders.sort_by(|a, b| {
            b.opentm
                .partial_cmp(&a.opentm)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(orders)
    }
}

// ============================================================================
// Private WebSocket Support (Basic)
// ============================================================================

pub struct KrakenPrivateWs {
    pub token: String,
}

impl KrakenPrivateWs {
    pub fn new(token: String) -> Self {
        Self { token }
    }

    /// Basic subscription message for ownTrades.
    pub fn own_trades_subscription(&self) -> String {
        serde_json::json!({
            "event": "subscribe",
            "subscription": {
                "name": "ownTrades",
                "token": self.token
            }
        })
        .to_string()
    }
}

// ============================================================================
// Private WebSocket Client (Basic Implementation)
// ============================================================================

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

impl KrakenBroker {
    /// Connect to Kraken private WebSocket and subscribe to ownTrades + openOrders.
    /// Returns a channel receiver for incoming messages.
    ///
    /// The reader owns reconnect/resubscribe. REST remains the authoritative
    /// snapshot/reconciliation path; private WS is the low-latency delta path.
    pub async fn start_private_ws(&self) -> Result<tokio::sync::mpsc::Receiver<String>, String> {
        let mut ws_stream = self.connect_private_ws_once().await?;
        let api_key = self.api_key.to_string();
        let api_secret = self.api_secret.to_string();
        let (tx, rx) = tokio::sync::mpsc::channel(256);

        tokio::spawn(async move {
            const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(20);
            const STALE_AFTER: Duration = Duration::from_secs(75);

            let mut reconnect_delay = Duration::from_secs(1);
            loop {
                let mut keepalive = tokio::time::interval(KEEPALIVE_INTERVAL);
                keepalive.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
                let mut last_seen = Instant::now();

                loop {
                    tokio::select! {
                        msg = ws_stream.next() => {
                            let Some(msg) = msg else {
                                let _ = tx
                                    .send(kraken_ws_status_message(
                                        "closed",
                                        "Kraken WS stream ended; reconnecting",
                                    ))
                                    .await;
                                break;
                            };

                            match msg {
                                Ok(Message::Text(text)) => {
                                    last_seen = Instant::now();
                                    reconnect_delay = Duration::from_secs(1);
                                    if tx.send(text.to_string()).await.is_err() {
                                        return;
                                    }
                                }
                                Ok(Message::Ping(payload)) => {
                                    last_seen = Instant::now();
                                    reconnect_delay = Duration::from_secs(1);
                                    if let Err(e) = ws_stream.send(Message::Pong(payload)).await {
                                        let _ = tx
                                            .send(kraken_ws_status_message(
                                                "error",
                                                format!("Kraken WS pong failed: {e}"),
                                            ))
                                            .await;
                                        break;
                                    }
                                }
                                Ok(Message::Pong(_)) => {
                                    last_seen = Instant::now();
                                    reconnect_delay = Duration::from_secs(1);
                                }
                                Ok(Message::Binary(_)) | Ok(Message::Frame(_)) => {
                                    last_seen = Instant::now();
                                    reconnect_delay = Duration::from_secs(1);
                                }
                                Ok(Message::Close(frame)) => {
                                    let reason = frame
                                        .as_ref()
                                        .map(|f| f.reason.to_string())
                                        .unwrap_or_else(|| "closed".to_string());
                                    let _ = tx
                                        .send(kraken_ws_status_message(
                                            "closed",
                                            format!("Kraken WS closed: {reason}"),
                                        ))
                                        .await;
                                    break;
                                }
                                Err(e) => {
                                    let _ = tx
                                        .send(kraken_ws_status_message(
                                            "error",
                                            format!("Kraken WS read failed: {e}"),
                                        ))
                                        .await;
                                    break;
                                }
                            }
                        }
                        _ = keepalive.tick() => {
                            let idle_for = last_seen.elapsed();
                            if idle_for >= STALE_AFTER {
                                let _ = tx
                                    .send(kraken_ws_status_message(
                                        "closed",
                                        format!("Kraken WS stale for {}s; reconnecting", idle_for.as_secs()),
                                    ))
                                    .await;
                                let _ = ws_stream.close(None).await;
                                break;
                            }
                            if let Err(e) = ws_stream.send(Message::Ping(Vec::new().into())).await {
                                let _ = tx
                                    .send(kraken_ws_status_message(
                                        "error",
                                        format!("Kraken WS keepalive ping failed: {e}"),
                                    ))
                                    .await;
                                break;
                            }
                        }
                    }
                }

                tokio::time::sleep(reconnect_delay).await;
                reconnect_delay = (reconnect_delay * 2).min(Duration::from_secs(30));

                let reconnect_broker = KrakenBroker::new(api_key.clone(), api_secret.clone());
                match reconnect_broker.connect_private_ws_once().await {
                    Ok(next_stream) => {
                        ws_stream = next_stream;
                        let _ = tx
                            .send(kraken_ws_status_message("online", "Kraken WS reconnected"))
                            .await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(kraken_ws_status_message(
                                "error",
                                format!("Kraken WS reconnect failed: {e}"),
                            ))
                            .await;
                    }
                }
            }
        });

        Ok(rx)
    }

    async fn connect_private_ws_once(
        &self,
    ) -> Result<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        String,
    > {
        let token = self.get_websockets_token_string().await?;
        // Authenticated private account/trading channels are rejected on Kraken's
        // public WS endpoint. Public books/tickers stay on ws.kraken.com; private
        // ownTrades/openOrders must use ws-auth.kraken.com with a REST-issued token.
        let (mut ws_stream, _) = connect_async("wss://ws-auth.kraken.com")
            .await
            .map_err(|e| format!("Kraken private WS connect failed: {e}"))?;

        for channel in ["ownTrades", "openOrders"] {
            let sub = serde_json::json!({
                "event": "subscribe",
                "subscription": {
                    "name": channel,
                    "token": token.clone()
                }
            });
            ws_stream
                .send(Message::Text(sub.to_string().into()))
                .await
                .map_err(|e| format!("Failed to subscribe to {channel}: {e}"))?;
        }

        Ok(ws_stream)
    }

    /// Stream Kraken public Level 2 order-book updates for one Spot/xStocks pair.
    ///
    /// The task keeps a local book from the initial WS snapshot plus deltas and
    /// emits TyphooN's existing normalized orderbook JSON shape on every update.
    pub async fn start_public_orderbook_ws(
        &self,
        symbol: &str,
        depth: usize,
    ) -> Result<tokio::sync::mpsc::Receiver<String>, String> {
        let ws_pair = crate::core::kraken::resolve_kraken_ws_pair(&self.client, symbol)
            .await
            .ok_or_else(|| format!("Kraken orderbook WS: unsupported pair {symbol}"))?;
        let display_symbol = crate::core::kraken::normalize_pair_symbol(symbol);
        let depth = match depth {
            10 | 25 | 100 | 500 | 1000 => depth,
            value if value < 25 => 10,
            value if value < 100 => 25,
            value if value < 500 => 100,
            value if value < 1000 => 500,
            _ => 1000,
        };
        let (tx, rx) = tokio::sync::mpsc::channel(512);

        tokio::spawn(async move {
            let mut reconnect_delay = Duration::from_secs(1);
            let mut bids: Vec<(f64, f64)> = Vec::new();
            let mut asks: Vec<(f64, f64)> = Vec::new();
            loop {
                match connect_kraken_public_book_once(&ws_pair, depth).await {
                    Ok(mut ws_stream) => {
                        reconnect_delay = Duration::from_secs(1);
                        let _ = tx
                            .send(kraken_public_book_status_message(
                                &display_symbol,
                                &ws_pair,
                                "online",
                            ))
                            .await;
                        while let Some(msg) = ws_stream.next().await {
                            match msg {
                                Ok(Message::Text(text)) => {
                                    if apply_kraken_public_book_message(
                                        &text, &mut bids, &mut asks, depth,
                                    ) {
                                        let update = kraken_public_book_snapshot_json(
                                            &display_symbol,
                                            &ws_pair,
                                            &bids,
                                            &asks,
                                        );
                                        if tx.send(update).await.is_err() {
                                            return;
                                        }
                                    }
                                }
                                Ok(Message::Ping(payload)) => {
                                    if ws_stream.send(Message::Pong(payload)).await.is_err() {
                                        break;
                                    }
                                }
                                Ok(Message::Pong(_)) => {}
                                Ok(Message::Binary(_)) => {}
                                Ok(Message::Frame(_)) => {}
                                Ok(Message::Close(_)) => break,
                                Err(_) => break,
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx
                            .send(kraken_public_book_error_message(
                                &display_symbol,
                                &ws_pair,
                                &e,
                            ))
                            .await;
                    }
                }

                tokio::time::sleep(reconnect_delay).await;
                reconnect_delay = (reconnect_delay * 2).min(Duration::from_secs(30));
                bids.clear();
                asks.clear();
            }
        });

        Ok(rx)
    }
}

async fn connect_kraken_public_book_once(
    ws_pair: &str,
    depth: usize,
) -> Result<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    String,
> {
    let (mut ws_stream, _) = connect_async("wss://ws.kraken.com")
        .await
        .map_err(|e| format!("Kraken public WS connect failed: {e}"))?;
    let sub = serde_json::json!({
        "event": "subscribe",
        "pair": [ws_pair],
        "subscription": { "name": "book", "depth": depth }
    });
    ws_stream
        .send(Message::Text(sub.to_string().into()))
        .await
        .map_err(|e| format!("Kraken public WS subscribe failed: {e}"))?;
    Ok(ws_stream)
}

fn apply_kraken_public_book_message(
    text: &str,
    bids: &mut Vec<(f64, f64)>,
    asks: &mut Vec<(f64, f64)>,
    depth: usize,
) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return false;
    };
    let Some(arr) = value.as_array() else {
        return false;
    };
    if !arr
        .iter()
        .any(|v| v.as_str().is_some_and(|s| s.starts_with("book-")))
    {
        return false;
    }
    let mut changed = false;
    for payload in arr.iter().filter_map(|v| v.as_object()) {
        if let Some(levels) = payload.get("as").and_then(|v| v.as_array()) {
            asks.clear();
            apply_kraken_book_levels(asks, levels);
            changed = true;
        }
        if let Some(levels) = payload.get("bs").and_then(|v| v.as_array()) {
            bids.clear();
            apply_kraken_book_levels(bids, levels);
            changed = true;
        }
        if let Some(levels) = payload.get("a").and_then(|v| v.as_array()) {
            apply_kraken_book_levels(asks, levels);
            changed = true;
        }
        if let Some(levels) = payload.get("b").and_then(|v| v.as_array()) {
            apply_kraken_book_levels(bids, levels);
            changed = true;
        }
    }
    if changed {
        bids.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        asks.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        bids.truncate(depth);
        asks.truncate(depth);
    }
    changed
}

fn apply_kraken_book_levels(side: &mut Vec<(f64, f64)>, levels: &[serde_json::Value]) {
    for level in levels {
        let Some(arr) = level.as_array() else {
            continue;
        };
        let Some(price) = arr.first().and_then(kraken_json_f64) else {
            continue;
        };
        let Some(size) = arr.get(1).and_then(kraken_json_f64) else {
            continue;
        };
        if let Some(existing_idx) = side
            .iter()
            .position(|(existing_price, _)| (*existing_price - price).abs() <= f64::EPSILON)
        {
            if size <= 0.0 {
                side.remove(existing_idx);
            } else {
                side[existing_idx] = (price, size);
            }
        } else if size > 0.0 {
            side.push((price, size));
        }
    }
}

fn kraken_json_f64(value: &serde_json::Value) -> Option<f64> {
    match value {
        serde_json::Value::String(s) => s.parse::<f64>().ok(),
        serde_json::Value::Number(n) => n.as_f64(),
        _ => None,
    }
    .filter(|v| v.is_finite())
}

fn kraken_public_book_snapshot_json(
    display_symbol: &str,
    ws_pair: &str,
    bids: &[(f64, f64)],
    asks: &[(f64, f64)],
) -> String {
    let side_json = |levels: &[(f64, f64)]| -> Vec<serde_json::Value> {
        levels
            .iter()
            .map(|(price, size)| serde_json::json!({ "price": price, "size": size }))
            .collect()
    };
    serde_json::json!({
        "source": "kraken_ws",
        "symbol": display_symbol,
        "ws_pair": ws_pair,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "bids": side_json(bids),
        "asks": side_json(asks),
    })
    .to_string()
}

fn kraken_public_book_status_message(display_symbol: &str, ws_pair: &str, status: &str) -> String {
    serde_json::json!({
        "source": "kraken_ws",
        "symbol": display_symbol,
        "ws_pair": ws_pair,
        "status": status,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "bids": [],
        "asks": [],
    })
    .to_string()
}

fn kraken_public_book_error_message(display_symbol: &str, ws_pair: &str, error: &str) -> String {
    serde_json::json!({
        "source": "kraken_ws",
        "symbol": display_symbol,
        "ws_pair": ws_pair,
        "status": "error",
        "error": error,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "bids": [],
        "asks": [],
    })
    .to_string()
}

fn kraken_ws_status_message(status: &str, message: impl Into<String>) -> String {
    serde_json::json!({
        "event": "systemStatus",
        "status": status,
        "connectionID": 0,
        "version": "TyphooN",
        "message": message.into(),
    })
    .to_string()
}

fn parse_trades_history_result(
    resp: &serde_json::Value,
) -> Result<(Vec<KrakenTrade>, Option<u64>), String> {
    // private_post_owned already unwraps Kraken's {error,result} envelope and
    // returns the result object. Older callers/tests may still pass a full
    // envelope, so accept both shapes instead of logging a false
    // "TradesHistory missing result" error.
    let result = resp.get("result").unwrap_or(resp);

    let trades_obj = result
        .get("trades")
        .and_then(|v| v.as_object())
        .ok_or_else(|| "Kraken TradesHistory missing trades object".to_string())?;

    let mut trades = Vec::new();
    for (trade_id, trade_value) in trades_obj {
        if let Some(trade) = trade_value.as_object() {
            trades.push(kraken_trade_from_object(trade_id.clone(), trade));
        }
    }

    let count = result.get("count").and_then(|v| {
        v.as_u64()
            .or_else(|| v.as_str().and_then(|s| s.parse::<u64>().ok()))
    });

    Ok((trades, count))
}

fn kraken_trade_from_object(
    trade_id: String,
    trade: &serde_json::Map<String, serde_json::Value>,
) -> KrakenTrade {
    KrakenTrade {
        trade_id,
        ordertxid: trade
            .get("ordertxid")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        pair: trade
            .get("pair")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        time: trade
            .get("time")
            .and_then(|v| {
                v.as_f64()
                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            })
            .unwrap_or(0.0),
        side: trade
            .get("side")
            .or_else(|| trade.get("type"))
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        ordertype: trade
            .get("ordertype")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        price: trade
            .get("price")
            .and_then(|v| {
                v.as_f64()
                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            })
            .unwrap_or(0.0),
        cost: trade
            .get("cost")
            .and_then(|v| {
                v.as_f64()
                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            })
            .unwrap_or(0.0),
        fee: trade
            .get("fee")
            .and_then(|v| {
                v.as_f64()
                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            })
            .unwrap_or(0.0),
        vol: trade
            .get("vol")
            .and_then(|v| {
                v.as_f64()
                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            })
            .unwrap_or(0.0),
        margin: trade
            .get("margin")
            .and_then(|v| {
                v.as_f64()
                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            })
            .unwrap_or(0.0),
        misc: trade
            .get("misc")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    }
}

/// Parse all Kraken WebSocket `ownTrades` entries in a message.
pub fn parse_own_trades_messages(msg: &serde_json::Value) -> Vec<KrakenTrade> {
    let Some(arr) = msg.as_array() else {
        return Vec::new();
    };
    if !arr.iter().any(|v| v.as_str() == Some("ownTrades")) {
        return Vec::new();
    }
    let Some(obj) = arr.first().and_then(|v| v.as_object()) else {
        return Vec::new();
    };

    obj.iter()
        .filter_map(|(trade_id, trade_val)| {
            trade_val
                .as_object()
                .map(|trade| kraken_trade_from_object(trade_id.clone(), trade))
        })
        .collect()
}

/// Attempt to parse a Kraken WebSocket ownTrades message into one KrakenTrade.
pub fn parse_own_trades_message(msg: &serde_json::Value) -> Option<KrakenTrade> {
    parse_own_trades_messages(msg).into_iter().next()
}

fn kraken_ws_f64(obj: &serde_json::Map<String, serde_json::Value>, key: &str) -> f64 {
    obj.get(key)
        .and_then(|v| {
            v.as_f64()
                .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        })
        .unwrap_or(0.0)
}

fn kraken_ws_opt_f64(obj: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<f64> {
    obj.get(key).and_then(|v| {
        v.as_f64()
            .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
    })
}

fn kraken_ws_string(obj: &serde_json::Map<String, serde_json::Value>, key: &str) -> String {
    obj.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

fn kraken_order_from_object(
    txid: &str,
    order: &serde_json::Map<String, serde_json::Value>,
) -> KrakenOrder {
    let descr = order.get("descr").and_then(|v| v.as_object());
    let descr_string = |key: &str| -> String {
        descr
            .and_then(|d| d.get(key))
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string()
    };
    let descr_f64 = |key: &str| -> f64 {
        descr
            .and_then(|d| d.get(key))
            .and_then(|v| {
                v.as_f64()
                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            })
            .unwrap_or(0.0)
    };
    let descr_opt_f64 = |key: &str| -> Option<f64> {
        descr.and_then(|d| d.get(key)).and_then(|v| {
            v.as_f64()
                .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        })
    };
    let trades = order
        .get("trades")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    KrakenOrder {
        txid: txid.to_string(),
        refid: order
            .get("refid")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        userref: order.get("userref").and_then(|v| v.as_i64()),
        status: kraken_ws_string(order, "status"),
        opentm: kraken_ws_f64(order, "opentm"),
        starttm: kraken_ws_opt_f64(order, "starttm"),
        expiretm: kraken_ws_opt_f64(order, "expiretm"),
        pair: descr_string("pair"),
        r#type: descr_string("type"),
        ordertype: descr_string("ordertype"),
        price: descr_f64("price"),
        price2: descr_opt_f64("price2"),
        vol: kraken_ws_f64(order, "vol"),
        vol_exec: kraken_ws_f64(order, "vol_exec"),
        cost: kraken_ws_f64(order, "cost"),
        fee: kraken_ws_f64(order, "fee"),
        stopprice: kraken_ws_opt_f64(order, "stopprice"),
        limitprice: kraken_ws_opt_f64(order, "limitprice"),
        misc: order
            .get("misc")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        trades,
    }
}

/// Parse Kraken private WebSocket `openOrders` messages into typed orders.
///
/// Kraken v1 private messages are array-wrapped and commonly arrive as:
/// `[ { "ORDER_TXID": { "status": "open", "descr": { ... }, ... } }, "openOrders", <channel_id> ]`.
/// Snapshot and incremental updates use the same shape; callers should upsert
/// open/pending orders and remove terminal statuses (`closed`, `canceled`, `expired`).
pub fn parse_open_orders_message(msg: &serde_json::Value) -> Vec<KrakenOrder> {
    let Some(arr) = msg.as_array() else {
        return Vec::new();
    };
    if !arr.iter().any(|v| v.as_str() == Some("openOrders")) {
        return Vec::new();
    }
    let Some(orders_obj) = arr.first().and_then(|v| v.as_object()) else {
        return Vec::new();
    };

    orders_obj
        .iter()
        .filter_map(|(txid, order_val)| {
            let order = order_val.as_object()?;
            let descr = order.get("descr").and_then(|v| v.as_object());

            let descr_string = |key: &str| -> String {
                descr
                    .and_then(|d| d.get(key))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string()
            };
            let descr_f64 = |key: &str| -> f64 {
                descr
                    .and_then(|d| d.get(key))
                    .and_then(|v| {
                        v.as_f64()
                            .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
                    })
                    .unwrap_or(0.0)
            };
            let descr_opt_f64 = |key: &str| -> Option<f64> {
                descr.and_then(|d| d.get(key)).and_then(|v| {
                    v.as_f64()
                        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
                })
            };

            let trades = order
                .get("trades")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            Some(KrakenOrder {
                txid: txid.clone(),
                refid: order
                    .get("refid")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                userref: order.get("userref").and_then(|v| v.as_i64()),
                status: kraken_ws_string(order, "status"),
                opentm: kraken_ws_f64(order, "opentm"),
                starttm: kraken_ws_opt_f64(order, "starttm"),
                expiretm: kraken_ws_opt_f64(order, "expiretm"),
                pair: descr_string("pair"),
                r#type: descr_string("type"),
                ordertype: descr_string("ordertype"),
                price: descr_f64("price"),
                price2: descr_opt_f64("price2"),
                vol: kraken_ws_f64(order, "vol"),
                vol_exec: kraken_ws_f64(order, "vol_exec"),
                cost: kraken_ws_f64(order, "cost"),
                fee: kraken_ws_f64(order, "fee"),
                stopprice: kraken_ws_opt_f64(order, "stopprice"),
                limitprice: kraken_ws_opt_f64(order, "limitprice"),
                misc: order
                    .get("misc")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                trades,
            })
        })
        .collect()
}

#[cfg(test)]
mod kraken_private_ws_parse_tests {
    use super::*;

    #[test]
    fn parse_batched_own_trades_ws_message() {
        let msg = serde_json::json!([
            {
                "T1": {
                    "ordertxid": "O1",
                    "pair": "XXBTZUSD",
                    "time": "1700000001.5",
                    "side": "buy",
                    "ordertype": "limit",
                    "price": "35000.0",
                    "cost": "3500.0",
                    "fee": "1.0",
                    "vol": "0.1",
                    "margin": "0.0"
                },
                "T2": {
                    "ordertxid": "O2",
                    "pair": "XETHZUSD",
                    "time": 1700000002.5,
                    "side": "sell",
                    "ordertype": "market",
                    "price": "2500.0",
                    "cost": "500.0",
                    "fee": "0.5",
                    "vol": "0.2",
                    "margin": "0.0"
                }
            },
            "ownTrades",
            8
        ]);
        let trades = parse_own_trades_messages(&msg);
        assert_eq!(trades.len(), 2);
        assert!(
            trades
                .iter()
                .any(|t| t.trade_id == "T1" && t.ordertxid == "O1")
        );
        assert!(
            trades
                .iter()
                .any(|t| t.trade_id == "T2" && t.ordertxid == "O2")
        );
    }

    #[test]
    fn parse_open_orders_ws_message() {
        let msg = serde_json::json!([
            {
                "OABCDEF-GHIJK-LMNOPQ": {
                    "refid": null,
                    "userref": 42,
                    "status": "open",
                    "opentm": 1700000000.123,
                    "starttm": 0,
                    "expiretm": 0,
                    "descr": {
                        "pair": "XXBTZUSD",
                        "type": "buy",
                        "ordertype": "limit",
                        "price": "35000.0",
                        "price2": "0"
                    },
                    "vol": "0.25",
                    "vol_exec": "0.10",
                    "cost": "3500.0",
                    "fee": "1.2",
                    "misc": ""
                }
            },
            "openOrders",
            7
        ]);
        let orders = parse_open_orders_message(&msg);
        assert_eq!(orders.len(), 1);
        let order = &orders[0];
        assert_eq!(order.txid, "OABCDEF-GHIJK-LMNOPQ");
        assert_eq!(order.userref, Some(42));
        assert_eq!(order.status, "open");
        assert_eq!(order.pair, "XXBTZUSD");
        assert_eq!(order.r#type, "buy");
        assert_eq!(order.ordertype, "limit");
        assert!((order.price - 35000.0).abs() < 1e-9);
        assert!((order.vol - 0.25).abs() < 1e-9);
        assert!((order.vol_exec - 0.10).abs() < 1e-9);
    }
}
