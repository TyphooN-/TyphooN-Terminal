//! Kraken broker interface (Phase 1: Authentication + Account).
//!
//! Wraps Kraken REST API for account management and trading.
//! Separate from `core/kraken.rs` which handles public OHLCV data only.
//! See ADR-072 for the full integration plan.

mod equities;
mod helpers;
mod iapi_limiter;
mod limiter;
mod ohlc_ws;
mod order_types;
mod private_ws;
mod public_book;

pub use self::equities::{
    IAPI_RATE_LIMITED_ERR_PREFIX, KrakenEquityBar, KrakenEquityMarket, KrakenEquityTicker,
    iapi_rate_limited_for_secs,
};
pub use self::iapi_limiter::{
    IapiLimiter, IapiLimiterConfig, iapi_limiter, iapi_limiter_init, log_iapi_response_headers,
};
pub use self::ohlc_ws::{
    KRAKEN_WS_OHLC_INTERVALS_MIN, KRAKEN_WS_V2_URL, KrakenOhlcStreamerEvent, KrakenWsOhlcBar,
    build_subscribe_frames, build_unsubscribe_frame, compute_reconnect_backoff,
    is_heartbeat_or_status, is_subscribe_ack, kraken_ws_bar_to_json,
    kraken_ws_interval_to_tf_label, kraken_ws_symbol_to_cache_key, parse_ohlc_message,
    run_ohlc_streamer, ws_bar_is_closed,
};
pub use self::order_types::{KrakenConditionalClose, KrakenOrderRequest};
pub use self::private_ws::{
    KrakenOrder, KrakenPrivateWs, KrakenTrade, parse_open_orders_message, parse_own_trades_message,
    parse_own_trades_messages,
};

use self::equities::{
    IAPI_RATE_LIMITED_ERR_PREFIX as IAPI_RL_PREFIX, arm_iapi_backoff, parse_json_i64,
    parse_json_number,
};
use self::private_ws::{
    kraken_order_from_object, kraken_ws_status_message, parse_trades_history_result,
};
use self::public_book::{
    apply_kraken_public_book_message, connect_kraken_public_book_once,
    kraken_public_book_error_message, kraken_public_book_snapshot_json,
    kraken_public_book_status_message,
};

use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use futures_util::{SinkExt, StreamExt};
use hmac::{Hmac, KeyInit, Mac};
use reqwest::Client;
use sha2::{Digest, Sha256, Sha512};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;
use zeroize::Zeroizing;

use self::helpers::{
    encode_form_params, format_f64_param, kraken_client, kraken_private_rest_counter_cost,
    sanitize_api_error_body,
};
use self::limiter::{KRAKEN_PRIVATE_REST_MAX_ATTEMPTS, KrakenPrivateRestLimiter};

type HmacSha512 = Hmac<Sha512>;

const KRAKEN_BASE_URL: &str = "https://api.kraken.com";
const KRAKEN_INTERNAL_API_BASE_URL: &str = "https://iapi.kraken.com/api/internal";

const KRAKEN_WS_INSTRUMENT_SNAPSHOT_TIMEOUT: Duration = Duration::from_secs(20);

async fn fetch_ws_instrument_snapshot(
    include_tokenized_assets: bool,
) -> Result<serde_json::Value, String> {
    let (ws_stream, _) = connect_async(KRAKEN_WS_V2_URL)
        .await
        .map_err(|e| format!("Kraken WS instrument connect failed: {e}"))?;
    let (mut sink, mut stream) = ws_stream.split();
    let subscribe = serde_json::json!({
        "method": "subscribe",
        "params": {
            "channel": "instrument",
            "include_tokenized_assets": include_tokenized_assets,
        },
        "req_id": 1,
    });
    sink.send(Message::Text(subscribe.to_string().into()))
        .await
        .map_err(|e| format!("Kraken WS instrument subscribe failed: {e}"))?;

    tokio::time::timeout(KRAKEN_WS_INSTRUMENT_SNAPSHOT_TIMEOUT, async move {
        while let Some(msg) = stream.next().await {
            let msg = msg.map_err(|e| format!("Kraken WS instrument read failed: {e}"))?;
            let Message::Text(text) = msg else {
                continue;
            };
            let value: serde_json::Value = serde_json::from_str(&text)
                .map_err(|e| format!("Kraken WS instrument JSON parse failed: {e}"))?;
            if value.get("method").and_then(|v| v.as_str()) == Some("subscribe") {
                if value.get("success").and_then(|v| v.as_bool()) == Some(false) {
                    return Err(format!("Kraken WS instrument subscribe rejected: {value}"));
                }
                continue;
            }
            if value.get("channel").and_then(|v| v.as_str()) != Some("instrument") {
                continue;
            }
            if value.get("type").and_then(|v| v.as_str()) != Some("snapshot") {
                continue;
            }
            return value
                .get("data")
                .cloned()
                .ok_or_else(|| "Kraken WS instrument snapshot missing data".to_string());
        }
        Err("Kraken WS instrument stream ended before snapshot".to_string())
    })
    .await
    .map_err(|_| "Kraken WS instrument snapshot timed out".to_string())?
}

fn parse_tokenized_equity_markets(
    snapshot_data: &serde_json::Value,
) -> Result<Vec<KrakenEquityMarket>, String> {
    let assets = snapshot_data
        .get("assets")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "Kraken WS instrument snapshot missing assets".to_string())?;
    let pairs = snapshot_data
        .get("pairs")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "Kraken WS instrument snapshot missing pairs".to_string())?;

    let mut underlying_by_asset: HashMap<String, String> = HashMap::new();
    for asset in assets {
        if asset.get("class").and_then(|v| v.as_str()) != Some("tokenized_asset") {
            continue;
        }
        let Some(id) = asset.get("id").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(underlying) = asset.get("underlying_symbol").and_then(|v| v.as_str()) else {
            continue;
        };
        let underlying = normalize_kraken_tokenized_underlying(underlying);
        if !underlying.is_empty() {
            underlying_by_asset.insert(id.to_string(), underlying);
        }
    }

    let mut by_symbol: HashMap<String, KrakenEquityMarket> = HashMap::new();
    for pair in pairs {
        let Some(base) = pair.get("base").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(symbol) = underlying_by_asset.get(base).cloned() else {
            continue;
        };
        let quote = pair
            .get("quote")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        if quote != "USD" {
            continue;
        }
        let status = pair
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("online")
            .to_string();
        let tradable = matches!(status.as_str(), "online" | "post_only");
        by_symbol
            .entry(symbol.clone())
            .and_modify(|market| {
                if tradable && !market.tradable {
                    market.tradable = true;
                    market.status = Some(status.clone());
                    market.instrument_status = Some("enabled".to_string());
                }
            })
            .or_insert_with(|| KrakenEquityMarket {
                symbol,
                name: None,
                tradable,
                status: Some(status),
                // TyphooN's consumer filters out `disabled`; this is a public
                // WS instrument pair, so mark the instrument side enabled.
                instrument_status: Some("enabled".to_string()),
            });
    }

    let mut out: Vec<KrakenEquityMarket> = by_symbol.into_values().collect();
    out.sort_by(|a, b| a.symbol.cmp(&b.symbol));
    Ok(out)
}

fn normalize_kraken_tokenized_underlying(raw: &str) -> String {
    raw.trim()
        .trim_end_matches(".EQ")
        .replace('/', "")
        .to_ascii_uppercase()
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

    /// Fetch the full public Kraken xStocks universe from WS v2 `instrument`.
    ///
    /// Kraken's REST `AssetPairs` defaults to crypto-only and Kraken Pro's iapi
    /// equity catalog has been incomplete/stale for us. WS v2 `instrument` with
    /// `include_tokenized_assets=true` is the public source that returns xStock
    /// tokenized assets plus their tradeable pairs (for example `AAPLx/USD`).
    /// We collapse Kraken's primary/SPV pair variants to the underlying equity
    /// symbol (`AAPL`, `BRK.B`, etc.) so the rest of TyphooN's existing xStocks
    /// quote/history paths keep their current API.
    pub async fn get_equity_markets(&self) -> Result<Vec<KrakenEquityMarket>, String> {
        let snapshot = fetch_ws_instrument_snapshot(true).await?;
        parse_tokenized_equity_markets(&snapshot)
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
        if let Err(secs) = iapi_limiter().acquire(1.0).await {
            return Err(format!("{IAPI_RL_PREFIX} ({secs}s remaining)"));
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
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS || body.contains("1015") {
                let secs = arm_iapi_backoff(&body).await;
                tracing::warn!(
                    "Kraken iapi rate-limited via equity ticker (HTTP {status}); backoff {secs}s engaged"
                );
                return Err(format!("{IAPI_RL_PREFIX} ({secs}s remaining)"));
            }
            return Err(format!(
                "Kraken equity ticker request failed: HTTP {status}: {body}"
            ));
        }
        log_iapi_response_headers(resp.headers(), "ticker");
        iapi_limiter().record_success().await;

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
            time_ms: result
                .get("time")
                .and_then(parse_json_i64)
                .unwrap_or_default(),
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
        if let Err(secs) = iapi_limiter().acquire(1.0).await {
            return Err(format!("{IAPI_RL_PREFIX} ({secs}s remaining)"));
        }
        let interval = interval_minutes.max(1).to_string();
        let since = since_seconds.unwrap_or(0).max(0).to_string();
        // Endpoint lives under `/markets/equities/{symbol}/...` like the ticker
        // call above; the previous path without the `equities/` segment now
        // 404s with `{"errors":[{"type":"Unknown method"}]}`. Kraken doesn't
        // dispatch on `asset_class` alone — the URL has to be in the equities
        // namespace.
        let url =
            format!("{KRAKEN_INTERNAL_API_BASE_URL}/markets/equities/{symbol}/ticker/history");
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
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS || body.contains("1015") {
                let secs = arm_iapi_backoff(&body).await;
                tracing::warn!(
                    "Kraken iapi rate-limited via equity history (HTTP {status}); backoff {secs}s engaged"
                );
                return Err(format!("{IAPI_RL_PREFIX} ({secs}s remaining)"));
            }
            return Err(format!(
                "Kraken equity history request failed: HTTP {status}: {body}"
            ));
        }
        log_iapi_response_headers(resp.headers(), "history");
        iapi_limiter().record_success().await;
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
            let Some(open) = row.get("open").and_then(parse_json_number) else {
                continue;
            };
            let Some(high) = row.get("high").and_then(parse_json_number) else {
                continue;
            };
            let Some(low) = row.get("low").and_then(parse_json_number) else {
                continue;
            };
            let Some(close) = row.get("close").and_then(parse_json_number) else {
                continue;
            };
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

    #[test]
    fn ws_instrument_parser_extracts_tokenized_assets_as_equity_universe() {
        let snapshot = serde_json::json!({
            "assets": [
                {"id":"BTC", "class":"currency"},
                {"id":"AAPLx", "underlying_symbol":"AAPL", "class":"tokenized_asset"},
                {"id":"AAPLSPV", "underlying_symbol":"AAPL", "class":"tokenized_asset"},
                {"id":"BRK.Bx", "underlying_symbol":"BRK.B", "class":"tokenized_asset"}
            ],
            "pairs": [
                {"symbol":"BTC/USD", "base":"BTC", "quote":"USD", "status":"online"},
                {"symbol":"AAPLx/USD", "base":"AAPLx", "quote":"USD", "status":"online"},
                {"symbol":"AAPLSPV/USD", "base":"AAPLx", "quote":"USD", "status":"online"},
                {"symbol":"BRK.Bx/USD", "base":"BRK.Bx", "quote":"USD", "status":"post_only"},
                {"symbol":"AAPLx/EUR", "base":"AAPLx", "quote":"EUR", "status":"online"}
            ]
        });

        let markets = parse_tokenized_equity_markets(&snapshot).unwrap();
        let symbols: Vec<_> = markets.iter().map(|m| m.symbol.as_str()).collect();
        assert_eq!(symbols, vec!["AAPL", "BRK.B"]);
        assert!(markets.iter().all(|m| m.tradable));
        assert!(
            markets
                .iter()
                .all(|m| m.instrument_status.as_deref() == Some("enabled"))
        );
    }

    #[tokio::test]
    #[ignore] // Requires network access — run with `cargo test -- --ignored`
    async fn get_equity_markets_public_ws_instrument_includes_xstocks() {
        let broker = KrakenBroker::new(String::new(), String::new());
        let markets = broker.get_equity_markets().await.unwrap();
        assert!(markets.len() > 50, "got {} markets", markets.len());
        assert!(markets.iter().any(|market| market.symbol == "AAPL"));
        assert!(markets.iter().any(|market| market.symbol == "TSLA"));
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
// Private WebSocket Client (Basic Implementation)
// ============================================================================

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
