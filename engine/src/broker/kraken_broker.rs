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
                let err = format!("Kraken API error {status}: {body}");
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
        let raw = pair.replace('/', "").to_ascii_uppercase();
        let bytes = raw.as_bytes();
        if raw.len() == 8
            && matches!(bytes[0] as char, 'X' | 'Z')
            && matches!(bytes[4] as char, 'X' | 'Z')
        {
            format!("{}{}", &raw[1..4], &raw[5..8])
        } else {
            raw
        }
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
        Self::canonical_pair_forms(target).into_iter().any(|form| {
            candidate == form || candidate.ends_with(&form) || form.ends_with(&candidate)
        })
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

    pub fn spot_position_summaries_from_balances(
        balances: &[(String, f64)],
    ) -> Vec<crate::broker::alpaca::PositionInfo> {
        balances
            .iter()
            .filter_map(|(asset, qty)| {
                if !qty.is_finite() || *qty <= 0.0 || Self::is_cash_asset(asset) {
                    return None;
                }
                let display_asset = Self::display_asset(asset);
                if display_asset.is_empty() {
                    return None;
                }
                Some(crate::broker::alpaca::PositionInfo {
                    symbol: format!("{}USD", display_asset),
                    qty: *qty,
                    side: "long".to_string(),
                    avg_entry_price: 0.0,
                    market_value: 0.0,
                    unrealized_pl: 0.0,
                    asset_class: "crypto_spot".to_string(),
                    asset_id: format!("spot:{asset}"),
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
            out.extend(Self::spot_position_summaries_from_balances(&balances));
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
            return Err(format!("Kraken orderbook request failed: HTTP {status}: {body}"));
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
    fn spot_position_summaries_convert_non_cash_balances() {
        let balances = vec![
            ("XXBT".to_string(), 0.25),
            ("ZUSD".to_string(), 1000.0),
            ("USDT".to_string(), 50.0),
        ];
        let positions = KrakenBroker::spot_position_summaries_from_balances(&balances);

        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].symbol, "BTCUSD");
        assert_eq!(positions[0].qty, 0.25);
        assert_eq!(positions[0].asset_class, "crypto_spot");
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
