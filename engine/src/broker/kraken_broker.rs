//! Kraken broker interface (Phase 1: Authentication + Account).
//!
//! Wraps Kraken REST API for account management and trading.
//! Separate from `core/kraken.rs` which handles public OHLCV data only.
//! See ADR-072 for the full integration plan.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use hmac::{Hmac, Mac};
use reqwest::Client;
use sha2::{Digest, Sha256, Sha512};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
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

        let secret_bytes = BASE64
            .decode(&self.api_secret)
            .unwrap_or_default();

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
                obj.insert("posid".to_string(), serde_json::Value::String(posid.clone()));
            }
            positions.push(p);
        }
        Ok(positions)
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
        self.place_order_with_leverage(pair, side, order_type, volume, price, None).await
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

    #[tokio::test]
    #[ignore] // Requires network access — run with `cargo test -- --ignored`
    async fn get_tradeable_pairs_public() {
        let broker = KrakenBroker::new(String::new(), String::new());
        let pairs = broker.get_tradeable_pairs().await.unwrap();
        assert!(!pairs.is_empty());
        assert!(pairs.iter().any(|(name, _)| name.contains("BTC") || name.contains("XBT")));
    }
}
