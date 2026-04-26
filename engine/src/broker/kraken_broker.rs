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
use std::time::{SystemTime, UNIX_EPOCH};
use zeroize::Zeroizing;

type HmacSha512 = Hmac<Sha512>;

const KRAKEN_BASE_URL: &str = "https://api.kraken.com";

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

/// Kraken broker client with HMAC-SHA512 request signing.
pub struct KrakenBroker {
    client: &'static Client,
    api_key: Zeroizing<String>,
    api_secret: Zeroizing<String>, // base64-encoded, zeroized on drop
    nonce: AtomicU64,
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
        if !self.is_authenticated() {
            return Err("Kraken API credentials not configured".to_string());
        }

        let nonce = self.next_nonce();
        let nonce_str = nonce.to_string();

        // Build POST body
        let mut params: Vec<(&str, &str)> = vec![("nonce", &nonce_str)];
        params.extend_from_slice(extra_params);
        let post_data: String = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

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

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Kraken response parse failed: {}", e))?;

        // Kraken returns {"error": [...], "result": {...}}
        if let Some(errors) = body.get("error").and_then(|e| e.as_array()) {
            let errs: Vec<&str> = errors.iter().filter_map(|e| e.as_str()).collect();
            if !errs.is_empty() {
                return Err(format!("Kraken API error: {}", errs.join(", ")));
            }
        }

        body.get("result")
            .cloned()
            .ok_or_else(|| "Kraken response missing 'result' field".to_string())
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

    pub async fn get_position_summaries(
        &self,
    ) -> Result<Vec<crate::broker::alpaca::PositionInfo>, String> {
        let positions = self.get_open_positions().await?;
        Ok(Self::position_summaries_from_raw(&positions))
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
        let order_pair = crate::core::kraken::to_kraken_pair_lossy(pair)
            .unwrap_or_else(|| pair.to_string());
        let cancelled = self
            .cancel_live_exit_orders_for_pair(pair, exit_side)
            .await?;
        let mut placements = Vec::new();
        let qty_abs = net_volume.abs();
        if let Some(sl) = sl_price {
            self.place_order(&order_pair, exit_side, "stop-loss", qty_abs, Some(sl))
                .await?;
            placements.push(format!("SL {}", sl));
        }
        if let Some(tp) = tp_price {
            self.place_order(&order_pair, exit_side, "take-profit", qty_abs, Some(tp))
                .await?;
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
        let order_pair = crate::core::kraken::to_kraken_pair_lossy(pair)
            .unwrap_or_else(|| pair.to_string());
        self.place_order(&order_pair, close_side, "market", qty_abs, None)
            .await
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
        let vol_str = volume.to_string();
        let mut params: Vec<(&str, &str)> = vec![
            ("pair", pair),
            ("type", side),
            ("ordertype", order_type),
            ("volume", &vol_str),
        ];

        let price_str;
        if let Some(p) = price {
            price_str = p.to_string();
            params.push(("price", &price_str));
        }

        if let Some(lev) = leverage {
            params.push(("leverage", lev));
        }

        self.private_post("/0/private/AddOrder", &params).await
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
