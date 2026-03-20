//! TyphooN Terminal — Risk management trading terminal for Alpaca Markets.
//!
//! Full port of TyphooN EA v1.420 from MQL5:
//! - 4 order modes (Standard, Fixed, Dynamic, VaR)
//! - Forward-looking TRIM martingale
//! - Dynamic PROTECT with urgency scaling
//! - VaR calculation with configurable confidence
//! - Draggable SL/TP lines, one-click order placement
//! - Real-time dashboard (position, risk, P/L, margin level)
//! - Discord webhook notifications

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(dead_code)]

mod broker;
mod core;
mod notifications;
mod strategies;

use broker::alpaca::{AlpacaBroker, StreamMessage};
use core::risk::{self, OrderMode, RiskConfig, SymbolSpec};
use core::var;
use core::margin;
use core::backtest::{self as backtest_engine, SMACrossStrategy, NNFXStrategy, BacktestResult, BarByBarResult};
use core::cache::SqliteCache;
use core::screener::{self as screener_engine, ScreenerFilter, ScreenerSymbol};
use strategies::martingale::{MartingaleConfig, MartingaleMode, MartingaleState};
use std::sync::{Arc, OnceLock};
use tokio::sync::Mutex;
use tauri::State;
use chrono::Datelike;

/// Shared HTTP client for non-broker requests (articles, FRED, AI chat).
/// Reuses TCP connections across calls. Per-request timeouts override the default.
fn shared_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .pool_max_idle_per_host(4)
            .build()
            .expect("Failed to build shared HTTP client")
    })
}

/// Shared application state.
struct AppState {
    broker: Option<AlpacaBroker>,
    risk_config: RiskConfig,
    martingale: MartingaleState,
    /// Per-symbol SL/TP tracked locally (Alpaca can't modify after placement).
    sl_levels: std::collections::HashMap<String, f64>,
    tp_levels: std::collections::HashMap<String, f64>,
    /// Cached symbol list for autocomplete.
    symbols: Vec<(String, String)>, // (symbol, name)
    /// Active WebSocket stream receiver.
    stream_rx: Option<tokio::sync::mpsc::Receiver<StreamMessage>>,
    /// Account protection: equity TP/SL (port of MQL5 EnableEquityTP/SL).
    equity_tp: Option<f64>,
    equity_sl: Option<f64>,
    /// SQLite cache for unlimited structured storage.
    db_cache: Option<SqliteCache>,
    /// WebSocket-driven bar builder: constructs 1-min OHLCV from trade stream.
    bar_builder: core::bar_builder::BarBuilder,
    /// In-flight bar requests — dedup concurrent fetches for same symbol:TF.
    /// Contains "symbol:timeframe" keys currently being fetched. Prevents page reloads
    /// from spawning duplicate multi-chunk fetches that hammer the API.
    bar_inflight: std::collections::HashSet<String>,
}

type SharedState = Arc<Mutex<AppState>>;

// ── Input Validation ────────────────────────────────────────────────

/// Strict symbol validation: 1-10 alphanumeric chars, optional single "/" for crypto pairs.
fn is_valid_symbol(symbol: &str) -> bool {
    if symbol.is_empty() || symbol.len() > 10 {
        return false;
    }
    let slash_count = symbol.chars().filter(|&c| c == '/').count();
    if slash_count > 1 {
        return false;
    }
    symbol.chars().all(|c| c.is_ascii_alphanumeric() || c == '/' || c == '.')
}

/// Validate timeframe input against known Alpaca timeframes.
fn is_valid_timeframe(tf: &str) -> bool {
    matches!(tf, "1Min" | "5Min" | "15Min" | "30Min" | "1Hour" | "4Hour" | "1Day" | "1Week" | "1Month")
}

// ── Broker Commands ─────────────────────────────────────────────────

#[tauri::command]
async fn connect(
    state: State<'_, SharedState>,
    api_key: String,
    secret_key: String,
    paper: bool,
) -> Result<String, String> {
    // Validate API key format (Alpaca keys are 20 alphanumeric chars)
    if api_key.is_empty() || api_key.len() > 100 || !api_key.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err("Invalid API key format".to_string());
    }
    if secret_key.is_empty() || secret_key.len() > 100 || !secret_key.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err("Invalid secret key format".to_string());
    }
    let broker = AlpacaBroker::new(api_key, secret_key, paper);
    let account = broker.get_account().await?;
    // Pre-warm data endpoint connection (data.alpaca.markets is a different host than api.alpaca.markets)
    // This shaves ~200ms off the first bar fetch by establishing TCP+TLS in advance.
    broker.warm_data_connection().await;
    let mut s = state.lock().await;
    s.broker = Some(broker);
    Ok(serde_json::to_string(&account).map_err(|e| format!("JSON error: {e}"))?)
}


// ── Credential Storage (AES-256-GCM encrypted, SQLite-backed) ───────

const KEYCHAIN_SERVICE: &str = "typhoon-terminal";

/// Validate account name: printable ASCII, no control chars, no path separators.
fn is_valid_account_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 100
        && name.chars().all(|c| c.is_ascii_graphic() || c == ' ')
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains("..")
}

/// Persistent salt file path — created once, reused forever.
/// 32 random bytes stored in ~/.config/typhoon-terminal/.cred_salt
fn get_or_create_salt() -> [u8; 32] {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let salt_path = std::path::PathBuf::from(home)
        .join(".config").join("typhoon-terminal").join(".cred_salt");

    // Try to read existing salt
    if let Ok(bytes) = std::fs::read(&salt_path) {
        if bytes.len() == 32 {
            let mut salt = [0u8; 32];
            salt.copy_from_slice(&bytes);
            return salt;
        }
    }

    // Generate new random salt and persist it
    let mut salt = [0u8; 32];
    rand::fill(&mut salt);
    if let Some(parent) = salt_path.parent() { std::fs::create_dir_all(parent).ok(); }
    std::fs::write(&salt_path, &salt).ok();
    tracing::info!("Generated new credential encryption salt");
    salt
}

/// Derive AES-256 key using PBKDF2-HMAC-SHA256 (100K iterations).
/// Input: machine hostname + username + persistent random salt.
/// 100K iterations makes brute-force attacks computationally expensive
/// while adding <50ms to each encrypt/decrypt (imperceptible to user).
fn derive_encryption_key() -> [u8; 32] {
    let hostname = std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("HOST"))
        .unwrap_or_else(|_| "typhoon".to_string());
    let username = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "default".to_string());
    let salt = get_or_create_salt();

    // Combine machine-specific info into password material
    let password = format!("TyphooN-Terminal-v2-{hostname}-{username}-credential-key");

    let mut key = [0u8; 32];
    pbkdf2::pbkdf2_hmac::<sha2::Sha256>(
        password.as_bytes(),
        &salt,
        100_000, // 100K iterations — ~30ms on modern hardware
        &mut key,
    );
    key
}

/// Encrypt plaintext with AES-256-GCM. Returns base64(nonce + ciphertext).
/// Nonce: 12 random bytes (unique per encryption).
/// Key: PBKDF2-HMAC-SHA256(hostname+username, random_salt, 100K iterations).
fn encrypt_credential(plaintext: &str) -> Result<String, String> {
    use aes_gcm::{Aes256Gcm, KeyInit, aead::Aead};
    use aes_gcm::Nonce;
    let key_bytes = derive_encryption_key();
    let cipher = Aes256Gcm::new_from_slice(&key_bytes)
        .map_err(|e| format!("Cipher init failed: {e}"))?;
    let mut nonce_bytes = [0u8; 12];
    rand::fill(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher.encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| format!("Encryption failed: {e}"))?;
    let mut combined = nonce_bytes.to_vec();
    combined.extend(ciphertext);
    Ok(base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &combined))
}

/// Decrypt base64(nonce + ciphertext) with AES-256-GCM.
fn decrypt_credential(encrypted_b64: &str) -> Result<String, String> {
    use aes_gcm::{Aes256Gcm, KeyInit, aead::Aead};
    use aes_gcm::Nonce;

    let combined = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encrypted_b64)
        .map_err(|e| format!("Base64 decode failed: {e}"))?;
    if combined.len() < 13 { return Err("Encrypted data too short".into()); }
    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let key_bytes = derive_encryption_key();
    let cipher = Aes256Gcm::new_from_slice(&key_bytes)
        .map_err(|e| format!("Cipher init failed: {e}"))?;
    let nonce = Nonce::from_slice(nonce_bytes);
    let plaintext = cipher.decrypt(nonce, ciphertext)
        .map_err(|_| "Decryption failed — wrong machine or corrupted data".to_string())?;
    String::from_utf8(plaintext).map_err(|e| format!("UTF-8 decode failed: {e}"))
}

/// Save credentials: AES-256-GCM encrypted, stored in SQLite KV (zstd compressed).
/// Key derived from machine hostname + username — portable within same user account.
#[tauri::command]
async fn keychain_save(state: State<'_, SharedState>, account_name: String, api_key: String, secret_key: String) -> Result<(), String> {
    if !is_valid_account_name(&account_name) {
        return Err("Invalid account name".into());
    }
    if api_key.is_empty() || api_key.len() > 100 || !api_key.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err("Invalid API key format".into());
    }
    if secret_key.is_empty() || secret_key.len() > 100 || !secret_key.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err("Invalid secret key format".into());
    }
    let cred_json = serde_json::json!({
        "apiKey": api_key,
        "secretKey": secret_key,
    }).to_string();

    let encrypted = encrypt_credential(&cred_json)?;
    let db_key = format!("cred:{account_name}");
    let s = state.lock().await;
    let cache = s.db_cache.as_ref().ok_or("SQLite cache not available")?;
    cache.put_kv(&db_key, &encrypted)?;

    tracing::info!("Saved encrypted credentials for '{}' to SQLite", account_name);
    Ok(())
}

/// Load credentials: decrypt from SQLite KV store.
#[tauri::command]
async fn keychain_load(state: State<'_, SharedState>, account_name: String) -> Result<String, String> {
    if !is_valid_account_name(&account_name) {
        return Err("Invalid account name".into());
    }
    let db_key = format!("cred:{account_name}");
    let s = state.lock().await;
    let cache = s.db_cache.as_ref().ok_or("SQLite cache not available")?;
    match cache.get_kv(&db_key)? {
        Some(encrypted) => {
            match decrypt_credential(&encrypted) {
                Ok(json) => Ok(json),
                Err(_) => Err("Decryption failed — stored credential is corrupt or was not encrypted".into()),
            }
        }
        None => Err("No credentials found".into()),
    }
}

/// Delete credentials from SQLite KV store.
#[tauri::command]
async fn keychain_delete(state: State<'_, SharedState>, account_name: String) -> Result<(), String> {
    if !is_valid_account_name(&account_name) {
        return Err("Invalid account name".into());
    }
    let db_key = format!("cred:{account_name}");
    let s = state.lock().await;
    if let Some(cache) = s.db_cache.as_ref() {
        cache.put_kv(&db_key, "{}")?;
    }
    tracing::info!("Deleted credentials for '{}'", account_name);
    Ok(())
}

#[tauri::command]
async fn get_account(state: State<'_, SharedState>) -> Result<String, String> {
    let broker = {
        let s = state.lock().await;
        s.broker.as_ref().ok_or("Not connected")?.clone()
    };
    let account = broker.get_account().await?;
    Ok(serde_json::to_string(&account).map_err(|e| format!("JSON error: {e}"))?)
}

#[tauri::command]
async fn get_positions(state: State<'_, SharedState>) -> Result<String, String> {
    let broker = {
        let s = state.lock().await;
        s.broker.as_ref().ok_or("Not connected")?.clone()
    };
    let positions = broker.get_positions().await?;
    Ok(serde_json::to_string(&positions).map_err(|e| format!("JSON error: {e}"))?)
}

#[tauri::command]
async fn get_bars(
    state: State<'_, SharedState>,
    symbol: String,
    timeframe: String,
    limit: u32,
) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    if !is_valid_timeframe(&timeframe) { return Err("Invalid timeframe".into()); }
    let limit = limit.min(50_000);
    // Clone broker and drop lock — get_bars can take seconds (multi-chunk fetch)
    let broker = {
        let s = state.lock().await;
        s.broker.as_ref().ok_or("Not connected")?.clone()
    };
    let bars = broker.get_bars(&symbol, &timeframe, limit).await?;
    Ok(serde_json::to_string(&bars).map_err(|e| format!("JSON error: {e}"))?)
}

/// Incremental bar fetch: checks SQLite cache for existing data, fetches only the gap.
/// The last cached candle is always re-fetched because it may still be forming (live candle).
/// Returns the full merged dataset. On first call, behaves like get_bars. On subsequent
/// calls, typically fetches 1-2 chunks instead of 13+.
#[tauri::command]
async fn get_bars_incremental(
    state: State<'_, SharedState>,
    symbol: String,
    timeframe: String,
    limit: u32,
    broker_id: Option<String>,
) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    if !is_valid_timeframe(&timeframe) { return Err("Invalid timeframe".into()); }
    let limit = limit.min(50_000);

    // Backend-level dedup: if same symbol:TF is already being fetched (e.g., Vite page reload
    // spawned a duplicate), wait briefly then return from cache instead of hitting API again.
    let dedup_key = format!("{}:{}", symbol, timeframe);
    {
        let s = state.lock().await;
        if s.bar_inflight.contains(&dedup_key) {
            // Another request is fetching this — wait for it and return from cache
            drop(s);
            tracing::debug!("{} @ {}: dedup — waiting for in-flight request", symbol, timeframe);
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            // Return whatever is in cache now (the other request should have stored it)
            let s = state.lock().await;
            if let Some(cache) = s.db_cache.as_ref() {
                let bid = broker_id.as_deref().unwrap_or("default");
                let key = format!("{bid}:{symbol}:{timeframe}");
                if let Ok(Some((json, _))) = cache.get_bars(&key) {
                    if json.len() > 10 && json.contains("\"timestamp\"") {
                        return Ok(json);
                    }
                }
                // Try default key
                let key = format!("default:{symbol}:{timeframe}");
                if let Ok(Some((json, _))) = cache.get_bars(&key) {
                    if json.len() > 10 && json.contains("\"timestamp\"") {
                        return Ok(json);
                    }
                }
            }
            // Fall through if cache still empty — proceed with fetch
        }
    }

    // Mark as in-flight
    {
        let mut s = state.lock().await;
        s.bar_inflight.insert(dedup_key.clone());
    }

    // Check SQLite cache for existing bars
    // Try caller-provided broker_id first, then "default" as fallback
    let (broker, cache_key, incremental_start) = {
        let s = state.lock().await;
        let broker = s.broker.as_ref().ok_or("Not connected")?.clone();
        let bid = broker_id.as_deref().unwrap_or("default");
        let primary_key = format!("{bid}:{symbol}:{timeframe}");
        let fallback_key = format!("default:{symbol}:{timeframe}");

        // Try primary key first, then fallback
        let start = s.db_cache.as_ref().and_then(|cache| {
            cache.get_incremental_start(&primary_key).ok().flatten()
                .map(|s| (primary_key.clone(), s))
                .or_else(|| {
                    if primary_key != fallback_key {
                        cache.get_incremental_start(&fallback_key).ok().flatten()
                            .map(|s| (fallback_key.clone(), s))
                    } else { None }
                })
        });

        match start {
            Some((key, (ts, count))) => (broker, key, Some((ts, count))),
            None => (broker, primary_key, None),
        }
    };

    if let Some((after_ts, cached_count)) = &incremental_start {
        // Fast path: if cache was updated very recently, return it without API call.
        // Prevents the SLV 1Day polling loop (re-fetching every 60s for no reason).
        // Freshness threshold = one period of the timeframe.
        let freshness_secs: i64 = match timeframe.as_str() {
            "1Min" => 60, "5Min" => 300, "15Min" => 900, "30Min" => 1800,
            "1Hour" => 3600, "4Hour" => 14400, "1Day" => 86400, "1Week" => 604800,
            _ => 3600,
        };
        {
            let s = state.lock().await;
            if let Some(cache) = s.db_cache.as_ref() {
                if let Ok(Some(age)) = cache.get_cache_age_secs(&cache_key) {
                    if age < freshness_secs {
                        // Cache is fresh enough — return without API call
                        if let Ok(Some((json, _))) = cache.get_bars(&cache_key) {
                            // Only use if non-empty (prevents returning empty/partial data)
                            if json.len() > 10 && json.contains("\"timestamp\"") {
                                tracing::debug!(
                                    "{} @ {}: cache fresh ({}s old), skipping API",
                                    symbol, timeframe, age
                                );
                                drop(s);
                                { let mut s = state.lock().await; s.bar_inflight.remove(&dedup_key); }
                                return Ok(json);
                            }
                        }
                    }
                }
            }
        }

        // Incremental: fetch only bars newer than the second-to-last cached bar
        let new_bars = broker.get_bars_after(&symbol, &timeframe, limit, Some(after_ts)).await?;
        let new_json = serde_json::to_string(&new_bars).map_err(|e| format!("JSON error: {e}"))?;
        let new_count = new_bars.len();

        // Merge new bars into cache
        let result = {
            let s = state.lock().await;
            if let Some(cache) = s.db_cache.as_ref() {
                let merged = cache.merge_bars(&cache_key, &new_json, limit as usize)?;
                tracing::info!(
                    "{} @ {}: incremental merge — {} cached + {} new bars fetched",
                    symbol, timeframe, cached_count, new_count
                );
                // Trim to limit (keep most recent)
                let all: Vec<serde_json::Value> = serde_json::from_str(&merged).unwrap_or_default();
                if all.len() > limit as usize {
                    let trimmed = &all[all.len() - limit as usize..];
                    serde_json::to_string(trimmed)
                        .map_err(|e| format!("JSON error: {e}"))?
                } else {
                    merged
                }
            } else {
                new_json
            }
        };
        // Clean up in-flight tracker
        { let mut s = state.lock().await; s.bar_inflight.remove(&dedup_key); }
        return Ok(result);
    }

    // No cache — full fetch, then store
    let bars = broker.get_bars(&symbol, &timeframe, limit).await?;
    let json = serde_json::to_string(&bars).map_err(|e| format!("JSON error: {e}"))?;

    // Store to SQLite cache for future incremental fetches
    {
        let s = state.lock().await;
        if let Some(cache) = s.db_cache.as_ref() {
            let _ = cache.put_bars(&cache_key, &json);
        }
    }

    // Clean up in-flight tracker
    {
        let mut s = state.lock().await;
        s.bar_inflight.remove(&dedup_key);
    }

    Ok(json)
}

/// Fetch bars from multiple timeframes for a symbol (for MultiKAMA, ATR_Projection, PreviousCandleLevels).
/// Returns JSON: { "1Hour": [...bars], "4Hour": [...bars], "1Day": [...bars], ... }
#[tauri::command]
async fn get_multi_tf_bars(
    state: State<'_, SharedState>,
    symbol: String,
    timeframes: Vec<String>,
    limit: u32,
) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    if timeframes.len() > 10 { return Err("Too many timeframes".into()); }
    for tf in &timeframes {
        if !is_valid_timeframe(tf) { return Err(format!("Invalid timeframe: {tf}")); }
    }
    let limit = limit.min(50_000);
    // Clone broker and drop lock before API calls to avoid blocking other commands
    let broker = {
        let s = state.lock().await;
        s.broker.as_ref().ok_or("Not connected")?.clone()
    };
    // Fetch all timeframes concurrently (rate limiter paces them internally)
    let futures: Vec<_> = timeframes.iter().map(|tf| {
        let b = broker.clone();
        let sym = symbol.clone();
        let tf = tf.clone();
        async move {
            match b.get_bars(&sym, &tf, limit).await {
                Ok(bars) => serde_json::to_value(&bars).ok().map(|v| (tf, v)),
                Err(e) => {
                    tracing::warn!("MTF bars {sym} @ {tf}: {e}");
                    None
                }
            }
        }
    }).collect();
    let results = futures_util::future::join_all(futures).await;
    let mut result = serde_json::Map::new();
    for item in results.into_iter().flatten() {
        result.insert(item.0, item.1);
    }
    Ok(serde_json::Value::Object(result).to_string())
}

#[tauri::command]
async fn place_order(
    state: State<'_, SharedState>,
    symbol: String,
    qty: f64,
    side: String,
) -> Result<String, String> {
    // Input validation — strict symbol whitelist
    if !is_valid_symbol(&symbol) {
        return Err("Invalid symbol".to_string());
    }
    if qty <= 0.0 || qty > 1_000_000.0 || !qty.is_finite() {
        return Err(format!("Invalid quantity: {qty}. Must be 0 < qty <= 1,000,000"));
    }
    if side != "buy" && side != "sell" {
        return Err(format!("Invalid side: {side}. Must be 'buy' or 'sell'"));
    }
    let broker = {
        let s = state.lock().await;
        s.broker.as_ref().ok_or("Not connected")?.clone()
    };
    // Verify account is not blocked for trading
    let account = broker.get_account().await?;
    if account.trading_blocked {
        return Err("Trading is blocked on this account".to_string());
    }
    let result = broker.market_order(&symbol, qty, &side).await?;
    Ok(serde_json::to_string(&result).map_err(|e| format!("JSON error: {e}"))?)
}

#[tauri::command]
async fn close_position(
    state: State<'_, SharedState>,
    symbol: String,
    qty: Option<f64>,
) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    if let Some(q) = qty {
        if q <= 0.0 || !q.is_finite() { return Err("Invalid quantity".into()); }
    }
    let broker = {
        let s = state.lock().await;
        s.broker.as_ref().ok_or("Not connected")?.clone()
    };
    let result = broker.close_position(&symbol, qty).await?;
    Ok(serde_json::to_string(&result).map_err(|e| format!("JSON error: {e}"))?)
}

#[tauri::command]
async fn place_limit_order(
    state: State<'_, SharedState>,
    symbol: String,
    qty: f64,
    side: String,
    limit_price: f64,
    tif: String,
) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    if qty <= 0.0 || qty > 1_000_000.0 || !qty.is_finite() { return Err("Invalid quantity".into()); }
    if side != "buy" && side != "sell" { return Err("Invalid side".into()); }
    if !limit_price.is_finite() || limit_price <= 0.0 { return Err("Invalid limit price".into()); }
    let tif = if matches!(tif.as_str(), "day" | "gtc" | "ioc" | "fok") { tif } else { "gtc".to_string() };
    let broker = {
        let s = state.lock().await;
        s.broker.as_ref().ok_or("Not connected")?.clone()
    };
    let result = broker.limit_order(&symbol, qty, &side, limit_price, &tif).await?;
    Ok(serde_json::to_string(&result).map_err(|e| format!("JSON error: {e}"))?)
}

#[tauri::command]
async fn place_stop_order(
    state: State<'_, SharedState>,
    symbol: String,
    qty: f64,
    side: String,
    stop_price: f64,
    tif: String,
) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    if qty <= 0.0 || qty > 1_000_000.0 || !qty.is_finite() { return Err("Invalid quantity".into()); }
    if side != "buy" && side != "sell" { return Err("Invalid side".into()); }
    if !stop_price.is_finite() || stop_price <= 0.0 { return Err("Invalid stop price".into()); }
    let tif = if matches!(tif.as_str(), "day" | "gtc" | "ioc" | "fok") { tif } else { "gtc".to_string() };
    let broker = {
        let s = state.lock().await;
        s.broker.as_ref().ok_or("Not connected")?.clone()
    };
    let result = broker.stop_order(&symbol, qty, &side, stop_price, &tif).await?;
    Ok(serde_json::to_string(&result).map_err(|e| format!("JSON error: {e}"))?)
}

#[tauri::command]
async fn place_stop_limit_order(
    state: State<'_, SharedState>,
    symbol: String,
    qty: f64,
    side: String,
    stop_price: f64,
    limit_price: f64,
    tif: String,
) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    if qty <= 0.0 || qty > 1_000_000.0 || !qty.is_finite() { return Err("Invalid quantity".into()); }
    if side != "buy" && side != "sell" { return Err("Invalid side".into()); }
    if !stop_price.is_finite() || stop_price <= 0.0 || !limit_price.is_finite() || limit_price <= 0.0 {
        return Err("Invalid price".into());
    }
    let tif = if matches!(tif.as_str(), "day" | "gtc" | "ioc" | "fok") { tif } else { "gtc".to_string() };
    let broker = {
        let s = state.lock().await;
        s.broker.as_ref().ok_or("Not connected")?.clone()
    };
    let result = broker.stop_limit_order(&symbol, qty, &side, stop_price, limit_price, &tif).await?;
    Ok(serde_json::to_string(&result).map_err(|e| format!("JSON error: {e}"))?)
}

#[tauri::command]
async fn place_trailing_stop(
    state: State<'_, SharedState>,
    symbol: String,
    qty: f64,
    side: String,
    trail_price: Option<f64>,
    trail_percent: Option<f64>,
) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    if qty <= 0.0 || qty > 1_000_000.0 || !qty.is_finite() { return Err("Invalid quantity".into()); }
    if side != "buy" && side != "sell" { return Err("Invalid side".into()); }
    if trail_price.is_none() && trail_percent.is_none() { return Err("Must specify trail_price or trail_percent".into()); }
    if let Some(tp) = trail_price { if !tp.is_finite() || tp <= 0.0 { return Err("Invalid trail price".into()); } }
    if let Some(tp) = trail_percent { if !tp.is_finite() || tp <= 0.0 || tp > 50.0 { return Err("Invalid trail percent".into()); } }
    let broker = {
        let s = state.lock().await;
        s.broker.as_ref().ok_or("Not connected")?.clone()
    };
    let result = broker.trailing_stop_order(&symbol, qty, &side, trail_price, trail_percent, "gtc").await?;
    Ok(serde_json::to_string(&result).map_err(|e| format!("JSON error: {e}"))?)
}

#[tauri::command]
async fn place_bracket_order(
    state: State<'_, SharedState>,
    symbol: String,
    qty: f64,
    side: String,
    tp_price: f64,
    sl_price: f64,
) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    if qty <= 0.0 || qty > 1_000_000.0 || !qty.is_finite() { return Err("Invalid quantity".into()); }
    if side != "buy" && side != "sell" { return Err("Invalid side".into()); }
    if !tp_price.is_finite() || tp_price <= 0.0 || !sl_price.is_finite() || sl_price <= 0.0 {
        return Err("Invalid TP/SL price".into());
    }
    let broker = {
        let s = state.lock().await;
        s.broker.as_ref().ok_or("Not connected")?.clone()
    };
    let result = broker.bracket_order(&symbol, qty, &side, tp_price, sl_price).await?;
    Ok(serde_json::to_string(&result).map_err(|e| format!("JSON error: {e}"))?)
}

#[tauri::command]
async fn get_open_orders(state: State<'_, SharedState>) -> Result<String, String> {
    let broker = {
        let s = state.lock().await;
        s.broker.as_ref().ok_or("Not connected")?.clone()
    };
    let orders = broker.get_orders("open", 100).await?;
    Ok(serde_json::to_string(&orders).map_err(|e| format!("JSON error: {e}"))?)
}

#[tauri::command]
async fn get_order_history(state: State<'_, SharedState>, limit: u32) -> Result<String, String> {
    let limit = limit.min(500);
    let broker = {
        let s = state.lock().await;
        s.broker.as_ref().ok_or("Not connected")?.clone()
    };
    let orders = broker.get_orders("closed", limit).await?;
    Ok(serde_json::to_string(&orders).map_err(|e| format!("JSON error: {e}"))?)
}

#[tauri::command]
async fn modify_order(
    state: State<'_, SharedState>,
    order_id: String,
    qty: Option<f64>,
    limit_price: Option<f64>,
    stop_price: Option<f64>,
    trail: Option<f64>,
) -> Result<String, String> {
    if order_id.is_empty() || order_id.len() > 100 { return Err("Invalid order ID".into()); }
    let broker = {
        let s = state.lock().await;
        s.broker.as_ref().ok_or("Not connected")?.clone()
    };
    let result = broker.modify_order(&order_id, qty, limit_price, stop_price, trail).await?;
    Ok(serde_json::to_string(&result).map_err(|e| format!("JSON error: {e}"))?)
}

#[tauri::command]
async fn cancel_order(state: State<'_, SharedState>, order_id: String) -> Result<(), String> {
    if order_id.is_empty() || order_id.len() > 100 { return Err("Invalid order ID".into()); }
    let broker = {
        let s = state.lock().await;
        s.broker.as_ref().ok_or("Not connected")?.clone()
    };
    broker.cancel_order(&order_id).await
}

#[tauri::command]
async fn close_all(state: State<'_, SharedState>) -> Result<(), String> {
    let broker = {
        let s = state.lock().await;
        s.broker.as_ref().ok_or("Not connected")?.clone()
    };
    broker.close_all_positions().await
}

#[tauri::command]
async fn load_symbols(state: State<'_, SharedState>) -> Result<String, String> {
    // Clone broker and drop lock — get_all_assets fetches 11K+ symbols
    let broker = {
        let s = state.lock().await;
        s.broker.as_ref().ok_or("Not connected")?.clone()
    };
    let assets = broker.get_all_assets().await?;
    let symbols: Vec<(String, String)> = assets
        .iter()
        .map(|a| (a.symbol.clone(), a.name.clone()))
        .collect();
    let count = symbols.len();
    // Re-acquire lock to write cached symbols
    let mut s = state.lock().await;
    s.symbols = symbols;
    Ok(format!("{count}"))
}

#[tauri::command]
async fn search_symbols(state: State<'_, SharedState>, query: String) -> Result<String, String> {
    if query.len() > 50 { return Err("Query too long".into()); }
    let s = state.lock().await;
    let q = query.to_uppercase();
    let matches: Vec<&(String, String)> = s.symbols
        .iter()
        .filter(|(sym, name)| {
            sym.starts_with(&q) || name.to_uppercase().contains(&q)
        })
        .take(20)
        .collect();
    Ok(serde_json::to_string(&matches).map_err(|e| format!("JSON error: {e}"))?)
}

#[tauri::command]
async fn get_asset(state: State<'_, SharedState>, symbol: String) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    let broker = {
        let s = state.lock().await;
        s.broker.as_ref().ok_or("Not connected")?.clone()
    };
    let asset = broker.get_asset(&symbol).await?;
    Ok(serde_json::to_string(&asset).map_err(|e| format!("JSON error: {e}"))?)
}

#[tauri::command]
async fn send_discord_notification(webhook_url: String, message: String) -> Result<(), String> {
    notifications::send_discord(&webhook_url, &message).await
}

// ── Risk & Lot Calculation Commands ─────────────────────────────────

#[tauri::command]
async fn calculate_lots(
    state: State<'_, SharedState>,
    symbol: String,
    sl_price: f64,
    tp_price: f64,
    current_price: f64,
) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    if !sl_price.is_finite() || !tp_price.is_finite() || !current_price.is_finite() {
        return Err("Invalid price value".into());
    }
    if sl_price <= 0.0 || tp_price <= 0.0 || current_price <= 0.0 {
        return Err("Prices must be positive".into());
    }
    // Clone broker + config and drop lock before API calls to avoid blocking other commands
    let (broker, risk_config, sl_level) = {
        let s = state.lock().await;
        let broker = s.broker.as_ref().ok_or("Not connected")?.clone();
        let config = s.risk_config.clone();
        let sl = s.sl_levels.get(&symbol).copied();
        (broker, config, sl)
    };

    let account = broker.get_account().await?;
    let balance = account.balance;
    let equity = account.equity;

    // Determine direction from TP/SL
    let is_buy = tp_price > sl_price;
    let sl_distance = if is_buy {
        current_price - sl_price
    } else {
        sl_price - current_price
    };

    if sl_distance <= 0.0 {
        return Err("SL must be on the opposite side of entry price".to_string());
    }

    // Get asset specs
    let asset = broker.get_asset(&symbol).await?;
    let spec = SymbolSpec {
        symbol: symbol.clone(),
        tick_size: asset.price_increment.unwrap_or(0.01),
        tick_value: asset.price_increment.unwrap_or(0.01), // 1:1 for stocks
        volume_min: asset.min_order_size.unwrap_or(1.0),
        volume_max: 1_000_000.0,
        volume_step: asset.min_trade_increment.unwrap_or(1.0),
        contract_size: 1.0,
        margin_rate: 1.0,
    };

    // VaR per lot (if VaR mode)
    let var_per_lot = if risk_config.order_mode == OrderMode::VaR {
        let bars = broker.get_bars(&symbol, &risk_config.var_timeframe, risk_config.var_periods + 1).await?;
        let closes: Vec<f64> = bars.iter().map(|b| b.close).collect();
        var::calculate_var(&closes, 1.0, spec.tick_value, spec.tick_size, current_price, risk_config.var_confidence)
            .map(|r| r.var_dollars)
            .unwrap_or(0.0)
    } else {
        0.0
    };

    // Break-even detection: check if any existing position on this symbol has SL ≈ entry
    let has_break_even = {
        let positions = broker.get_positions().await.unwrap_or_default();
        let tick = spec.tick_size;
        let symbol_no_slash = symbol.replace("/", "");
        positions.iter().any(|p| {
            (p.symbol == symbol || p.symbol == symbol_no_slash) && {
                if let Some(sl) = sl_level {
                    (sl - p.avg_entry_price).abs() < tick * 0.5
                } else {
                    false
                }
            }
        })
    };

    let (lots, count) = risk::calculate_lots(
        &risk_config,
        &spec,
        balance,
        equity,
        sl_distance,
        has_break_even,
        var_per_lot,
    );

    let side = if is_buy { "buy" } else { "sell" };

    Ok(serde_json::to_string(&serde_json::json!({
        "lots": lots,
        "count": count,
        "side": side,
        "sl_distance": sl_distance,
        "mode": format!("{:?}", risk_config.order_mode),
        "risk_money": if risk_config.order_mode == OrderMode::Standard {
            balance * (risk_config.risk_pct / 100.0)
        } else { 0.0 },
    })).unwrap())
}

#[tauri::command]
async fn calculate_position_var(
    state: State<'_, SharedState>,
    symbol: String,
    position_size: f64,
    current_price: f64,
) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    if !position_size.is_finite() || !current_price.is_finite() || current_price <= 0.0 {
        return Err("Invalid price or position size".into());
    }
    // Clone broker + config and drop lock before API calls
    let (broker, var_tf, var_periods, var_confidence) = {
        let s = state.lock().await;
        let broker = s.broker.as_ref().ok_or("Not connected")?.clone();
        (broker, s.risk_config.var_timeframe.clone(), s.risk_config.var_periods, s.risk_config.var_confidence)
    };

    let bars = broker.get_bars(&symbol, &var_tf, var_periods + 1).await?;
    let closes: Vec<f64> = bars.iter().map(|b| b.close).collect();

    let asset = broker.get_asset(&symbol).await?;
    let tick_size = asset.price_increment.unwrap_or(0.01);
    let tick_value = tick_size; // 1:1 for stocks

    match var::calculate_var(&closes, position_size, tick_value, tick_size, current_price, var_confidence) {
        Some(result) => Ok(serde_json::to_string(&result).map_err(|e| format!("JSON error: {e}"))?),
        None => Err("VaR calculation failed — insufficient price data".to_string()),
    }
}

/// Portfolio-aware VaR lot sizing: how many lots of a new symbol can be added
/// before total portfolio VaR exceeds the target budget.
/// Returns { lots, marginal_var_per_lot, existing_portfolio_var, diversification_factor, avg_correlation }.
#[tauri::command]
async fn calculate_portfolio_var_lots(
    state: State<'_, SharedState>,
    symbol: String,
    current_price: f64,
) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    if !current_price.is_finite() || current_price <= 0.0 { return Err("Invalid price".into()); }

    let (broker, var_tf, var_periods, var_confidence, var_pct) = {
        let s = state.lock().await;
        let broker = s.broker.as_ref().ok_or("Not connected")?.clone();
        let rc = &s.risk_config;
        (broker, rc.var_timeframe.clone(), rc.var_periods, rc.var_confidence, rc.var_risk_pct)
    };

    // Fetch live equity from account
    let account = broker.get_account().await?;
    let equity = account.equity;

    // Get all existing positions
    let positions = broker.get_positions().await.unwrap_or_default();

    // Fetch close prices for the new symbol
    let new_bars = broker.get_bars(&symbol, &var_tf, var_periods + 1).await?;
    let new_closes: Vec<f64> = new_bars.iter().map(|b| b.close).collect();

    // Fetch close prices for each existing position (parallel)
    let mut existing_data: Vec<(Vec<f64>, f64, f64)> = Vec::new();
    for p in &positions {
        let sym = &p.symbol;
        let qty = p.qty.abs();
        let mv = p.market_value.abs();
        let price = if qty > 0.0 { mv / qty } else { p.avg_entry_price };
        match broker.get_bars(sym, &var_tf, var_periods + 1).await {
            Ok(bars) => {
                let closes: Vec<f64> = bars.iter().map(|b| b.close).collect();
                existing_data.push((closes, qty, price));
            }
            Err(_) => {} // skip positions we can't get data for
        }
    }

    let asset = broker.get_asset(&symbol).await?;
    let tick_size = asset.price_increment.unwrap_or(0.01);
    let tick_value = tick_size;

    match var::portfolio_aware_lot_size(
        &new_closes, &existing_data,
        tick_value, tick_size, current_price,
        var_confidence, equity, var_pct,
    ) {
        Some((lots, marginal_var, existing_var)) => {
            Ok(serde_json::to_string(&serde_json::json!({
                "lots": lots,
                "marginal_var_per_lot": marginal_var,
                "existing_portfolio_var": existing_var,
                "var_budget": (var_pct / 100.0) * equity,
                "remaining_budget": ((var_pct / 100.0) * equity - existing_var).max(0.0),
                "equity": equity,
                "var_pct_target": var_pct,
                "positions_count": positions.len(),
            })).unwrap())
        }
        None => Err("Portfolio VaR calculation failed — insufficient data".into()),
    }
}

// ── Risk Config Commands ────────────────────────────────────────────

#[tauri::command]
async fn get_risk_config(state: State<'_, SharedState>) -> Result<String, String> {
    let s = state.lock().await;
    Ok(serde_json::to_string(&s.risk_config).map_err(|e| format!("JSON error: {e}"))?)
}

#[tauri::command]
async fn set_order_mode(state: State<'_, SharedState>, mode: String) -> Result<(), String> {
    let mut s = state.lock().await;
    s.risk_config.order_mode = match mode.as_str() {
        "Standard" => OrderMode::Standard,
        "Fixed" => OrderMode::Fixed,
        "Dynamic" => OrderMode::Dynamic,
        "VaR" => OrderMode::VaR,
        _ => return Err(format!("Unknown order mode: {mode}")),
    };
    Ok(())
}

#[tauri::command]
async fn set_risk_config(state: State<'_, SharedState>, config_json: String) -> Result<(), String> {
    if config_json.len() > 4096 { return Err("Config too large".into()); }
    let config: RiskConfig = serde_json::from_str(&config_json)
        .map_err(|e| format!("Invalid config: {e}"))?;
    // Bounds validation on all financial parameters
    if config.risk_pct < 0.0 || config.risk_pct > 100.0 { return Err("risk_pct must be 0-100".into()); }
    if config.max_risk_pct < 0.0 || config.max_risk_pct > 100.0 { return Err("max_risk_pct must be 0-100".into()); }
    if config.var_confidence < 0.0 || config.var_confidence > 1.0 { return Err("var_confidence must be 0-1".into()); }
    if config.fixed_lots < 0.0 || config.fixed_lots > 1_000_000.0 { return Err("fixed_lots out of range".into()); }
    if config.fixed_orders > 100 { return Err("fixed_orders too large".into()); }
    if config.var_risk_pct < 0.0 || config.var_risk_pct > 100.0 { return Err("var_risk_pct must be 0-100".into()); }
    if config.var_notional < 0.0 || config.var_notional > 1e9 { return Err("var_notional out of range".into()); }
    if config.var_periods > 10_000 { return Err("var_periods too large".into()); }
    if config.margin_buffer_pct < 0.0 || config.margin_buffer_pct > 100.0 { return Err("margin_buffer_pct must be 0-100".into()); }
    if config.min_balance < 0.0 { return Err("min_balance must be non-negative".into()); }
    if config.additional_risk_ratio < 0.0 || config.additional_risk_ratio > 10.0 { return Err("additional_risk_ratio out of range".into()); }
    if !is_valid_timeframe(&config.var_timeframe) { return Err("Invalid var_timeframe".into()); }
    let mut s = state.lock().await;
    s.risk_config = config;
    Ok(())
}

// ── SL/TP Tracking Commands ─────────────────────────────────────────

#[tauri::command]
async fn set_sl_level(state: State<'_, SharedState>, symbol: String, price: f64) -> Result<(), String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    if !price.is_finite() || price <= 0.0 { return Err("Invalid price".into()); }
    let mut s = state.lock().await;
    if s.sl_levels.len() > 200 {
        s.sl_levels.clear();
    }
    s.sl_levels.insert(symbol, price);
    Ok(())
}

#[tauri::command]
async fn set_tp_level(state: State<'_, SharedState>, symbol: String, price: f64) -> Result<(), String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    if !price.is_finite() || price <= 0.0 { return Err("Invalid price".into()); }
    let mut s = state.lock().await;
    if s.tp_levels.len() > 200 {
        s.tp_levels.clear();
    }
    s.tp_levels.insert(symbol, price);
    Ok(())
}

#[tauri::command]
async fn get_sl_tp_pl(
    state: State<'_, SharedState>,
    symbol: String,
    qty: f64,
    side: String,
    entry_price: f64,
) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    if !qty.is_finite() || !entry_price.is_finite() { return Err("Invalid numeric value".into()); }
    if side != "long" && side != "short" { return Err("Invalid side".into()); }
    let s = state.lock().await;
    let sl = s.sl_levels.get(&symbol).copied();
    let tp = s.tp_levels.get(&symbol).copied();

    let sl_pl = sl.map(|sl_price| {
        if side == "long" { (sl_price - entry_price) * qty }
        else { (entry_price - sl_price) * qty }
    });
    let tp_pl = tp.map(|tp_price| {
        if side == "long" { (tp_price - entry_price) * qty }
        else { (entry_price - tp_price) * qty }
    });
    let rr = match (sl_pl, tp_pl) {
        (Some(s), Some(t)) if s.abs() > 1e-10 => Some(t / s.abs()),
        _ => None,
    };

    Ok(serde_json::to_string(&serde_json::json!({
        "sl_pl": sl_pl,
        "tp_pl": tp_pl,
        "rr": rr,
        "sl_price": sl,
        "tp_price": tp,
    })).unwrap())
}

// ── Martingale Commands ─────────────────────────────────────────────

#[tauri::command]
async fn get_martingale_state(state: State<'_, SharedState>) -> Result<String, String> {
    let s = state.lock().await;
    Ok(serde_json::to_string(&serde_json::json!({
        "mode": format!("{:?}", s.martingale.mode),
        "label": s.martingale.mode.label(),
        "enabled": s.martingale.config.enabled,
        "trim_pct": s.martingale.config.trim_pct,
        "protect_pct": s.martingale.config.protect_pct,
        "hedge_closes": s.martingale.hedge_closes,
        "bias_closes": s.martingale.bias_closes,
        "protect_fires": s.martingale.protect_fire_count,
    })).unwrap())
}

#[tauri::command]
async fn set_martingale_mode(state: State<'_, SharedState>, mode: String) -> Result<String, String> {
    let mut s = state.lock().await;
    s.martingale.mode = match mode.as_str() {
        "Off" => MartingaleMode::Off,
        "Long" => MartingaleMode::Long,
        "Short" => MartingaleMode::Short,
        "Unwind" => MartingaleMode::Unwind,
        _ => return Err(format!("Unknown MG mode: {mode}")),
    };
    s.martingale.config.enabled = s.martingale.mode != MartingaleMode::Off;
    Ok(s.martingale.mode.label().to_string())
}

#[tauri::command]
async fn toggle_martingale(state: State<'_, SharedState>) -> Result<String, String> {
    let mut s = state.lock().await;
    s.martingale.mode = s.martingale.mode.next();
    s.martingale.config.enabled = s.martingale.mode != MartingaleMode::Off;
    Ok(serde_json::to_string(&serde_json::json!({
        "mode": format!("{:?}", s.martingale.mode),
        "label": s.martingale.mode.label(),
    })).unwrap())
}

#[tauri::command]
async fn set_martingale_config(state: State<'_, SharedState>, config_json: String) -> Result<(), String> {
    if config_json.len() > 4096 { return Err("Config too large".into()); }
    let config: MartingaleConfig = serde_json::from_str(&config_json)
        .map_err(|e| format!("Invalid MG config: {e}"))?;
    // Bounds: percentages must be 0-10000 (allow high margin levels), spread tolerance non-negative
    if config.trim_pct < 0.0 || config.protect_pct < 0.0 || config.hard_floor_pct < 0.0 {
        return Err("Margin thresholds must be non-negative".into());
    }
    if config.spread_tolerance < 0.0 { return Err("Spread tolerance must be non-negative".into()); }
    let mut s = state.lock().await;
    s.martingale.config = config;
    Ok(())
}

#[tauri::command]
async fn calc_open_mg_size(state: State<'_, SharedState>) -> Result<String, String> {
    let (broker, mg_state) = {
        let s = state.lock().await;
        (s.broker.as_ref().ok_or("Not connected")?.clone(), s.martingale.clone())
    };
    let account = broker.get_account().await?;

    let (per_side, safe_gross) = mg_state.calc_open_mg_size(account.equity);

    Ok(serde_json::to_string(&serde_json::json!({
        "per_side": per_side,
        "safe_gross": safe_gross,
        "equity": account.equity,
        "spread_tolerance": mg_state.config.spread_tolerance,
    })).unwrap())
}

#[tauri::command]
async fn open_martingale_hedge(
    state: State<'_, SharedState>,
    symbol: String,
    direction: String,
) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    // Clone broker + MG config and drop lock before API calls (3 network calls)
    let (broker, mg_state) = {
        let s = state.lock().await;
        let broker = s.broker.as_ref().ok_or("Not connected")?.clone();
        (broker, s.martingale.clone())
    };
    let account = broker.get_account().await?;

    let (per_side, safe_gross) = mg_state.calc_open_mg_size(account.equity);
    if per_side <= 0.0 {
        return Err("Insufficient equity for MG position".to_string());
    }

    let bias_side = match direction.as_str() {
        "Long" | "long" => "buy",
        "Short" | "short" => "sell",
        _ => return Err(format!("Invalid direction: {direction}")),
    };
    let hedge_side = if bias_side == "buy" { "sell" } else { "buy" };

    // Place hedge first (safer — neutral until bias placed)
    let hedge_result = broker.market_order(&symbol, per_side, hedge_side).await?;
    let bias_result = broker.market_order(&symbol, per_side, bias_side).await?;

    Ok(serde_json::to_string(&serde_json::json!({
        "hedge_order": hedge_result,
        "bias_order": bias_result,
        "per_side": per_side,
        "safe_gross": safe_gross,
        "direction": direction,
    })).unwrap())
}

// ── Margin Calculation Command ──────────────────────────────────────

#[tauri::command]
async fn get_margin_info(state: State<'_, SharedState>) -> Result<String, String> {
    // Clone broker + config and drop lock before API calls
    let (broker, margin_buffer_pct, mg_config) = {
        let s = state.lock().await;
        let broker = s.broker.as_ref().ok_or("Not connected")?.clone();
        (broker, s.risk_config.margin_buffer_pct, s.martingale.config.clone())
    };
    let account = broker.get_account().await?;

    let ml = margin::margin_level_pct(account.equity, account.initial_margin);
    let usable = margin::usable_margin(
        account.balance,
        account.initial_margin,
        margin_buffer_pct,
    );
    let positions = broker.get_positions().await?;
    let gross: f64 = positions.iter().map(|p| p.qty.abs()).sum();
    let spread_tol = margin::spread_tolerance(account.equity, gross);

    // Determine MG zone — only show zone if positions exist and MG is active
    let zone = if gross <= 0.0 || !mg_config.enabled {
        ""
    } else if ml <= mg_config.hard_floor_pct {
        "HARD FLOOR"
    } else if ml < mg_config.protect_pct {
        "PROTECT"
    } else if ml <= mg_config.trim_pct {
        "DEAD ZONE"
    } else {
        "TRIM"
    };

    Ok(serde_json::to_string(&serde_json::json!({
        "margin_level_pct": ml,
        "usable_margin": usable,
        "spread_tolerance": spread_tol,
        "gross_lots": gross,
        "zone": zone,
        "equity": account.equity,
        "balance": account.balance,
        "margin_used": account.initial_margin,
    })).unwrap())
}

// ── Account Protection (Equity TP/SL — port of MQL5 EnableEquityTP/SL) ──

#[tauri::command]
async fn set_equity_protection(
    state: State<'_, SharedState>,
    equity_tp: Option<f64>,
    equity_sl: Option<f64>,
) -> Result<(), String> {
    if let Some(tp) = equity_tp {
        if !tp.is_finite() || tp <= 0.0 { return Err("Invalid equity TP".into()); }
    }
    if let Some(sl) = equity_sl {
        if !sl.is_finite() || sl <= 0.0 { return Err("Invalid equity SL".into()); }
    }
    let mut s = state.lock().await;
    s.equity_tp = equity_tp;
    s.equity_sl = equity_sl;
    tracing::info!("Equity protection set: TP={:?}, SL={:?}", equity_tp, equity_sl);
    Ok(())
}

#[tauri::command]
async fn check_equity_protection(state: State<'_, SharedState>) -> Result<String, String> {
    // Clone broker + protection config and drop lock before API call
    let (broker, equity_tp, equity_sl) = {
        let s = state.lock().await;
        let broker = s.broker.as_ref().ok_or("Not connected")?.clone();
        (broker, s.equity_tp, s.equity_sl)
    };
    let account = broker.get_account().await?;

    let mut triggered = String::new();

    if let Some(tp) = equity_tp {
        if account.equity >= tp {
            triggered = format!("EQUITY_TP: equity ${:.2} >= target ${:.2}", account.equity, tp);
        }
    }
    if let Some(sl) = equity_sl {
        if account.equity <= sl {
            triggered = format!("EQUITY_SL: equity ${:.2} <= floor ${:.2}", account.equity, sl);
        }
    }

    Ok(serde_json::to_string(&serde_json::json!({
        "equity": account.equity,
        "equity_tp": equity_tp,
        "equity_sl": equity_sl,
        "triggered": if triggered.is_empty() { None } else { Some(&triggered) },
    })).unwrap())
}

// ── News & Events ───────────────────────────────────────────────

#[tauri::command]
async fn get_news(state: State<'_, SharedState>, symbol: String, limit: u32) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    let limit = limit.min(50);
    let broker = {
        let s = state.lock().await;
        s.broker.as_ref().ok_or("Not connected")?.clone()
    };
    // Fetch from multiple sources in parallel
    let alpaca_fut = broker.get_news(&symbol, limit);

    // Finnhub (secondary source — needs API key from localStorage, passed via frontend)
    // For now, just use Alpaca. Finnhub integration available via get_finnhub_news command.
    let mut news = alpaca_fut.await?;

    // Sort by date descending and dedup by headline
    news.sort_by(|a, b| {
        let ta = a["created_at"].as_str().unwrap_or("");
        let tb = b["created_at"].as_str().unwrap_or("");
        tb.cmp(ta)
    });
    news.dedup_by(|a, b| a["headline"].as_str() == b["headline"].as_str());
    news.truncate(limit as usize);

    Ok(serde_json::to_string(&news).map_err(|e| format!("JSON error: {e}"))?)
}

#[tauri::command]
async fn get_av_earnings(state: State<'_, SharedState>, symbol: String, av_key: String) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    if av_key.is_empty() || av_key.len() > 100 { return Err("Invalid Alpha Vantage key".into()); }
    let broker = { let s = state.lock().await; s.broker.as_ref().ok_or("Not connected")?.clone() };
    let data = broker.get_alpha_vantage_earnings(&symbol, &av_key).await?;
    Ok(serde_json::to_string(&data).map_err(|e| format!("JSON error: {e}"))?)
}

#[tauri::command]
async fn get_fmp_ratings(state: State<'_, SharedState>, symbol: String, fmp_key: String) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    if fmp_key.is_empty() || fmp_key.len() > 100 { return Err("Invalid FMP key".into()); }
    let broker = { let s = state.lock().await; s.broker.as_ref().ok_or("Not connected")?.clone() };
    let data = broker.get_fmp_analyst_ratings(&symbol, &fmp_key).await?;
    Ok(serde_json::to_string(&data).map_err(|e| format!("JSON error: {e}"))?)
}

#[tauri::command]
async fn get_finnhub_news(state: State<'_, SharedState>, symbol: String, finnhub_key: String) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    if finnhub_key.is_empty() || finnhub_key.len() > 100 { return Err("Invalid Finnhub API key".into()); }
    let broker = {
        let s = state.lock().await;
        s.broker.as_ref().ok_or("Not connected")?.clone()
    };
    let news = broker.get_finnhub_news(&symbol, &finnhub_key).await?;
    Ok(serde_json::to_string(&news).map_err(|e| format!("JSON error: {e}"))?)
}

#[tauri::command]
async fn get_corporate_actions(state: State<'_, SharedState>, symbol: String) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    let broker = {
        let s = state.lock().await;
        s.broker.as_ref().ok_or("Not connected")?.clone()
    };
    let actions = broker.get_corporate_actions(&symbol).await?;
    Ok(serde_json::to_string(&actions).map_err(|e| format!("JSON error: {e}"))?)
}

#[tauri::command]
async fn get_portfolio_history(state: State<'_, SharedState>, period: String, timeframe: String) -> Result<String, String> {
    if period.is_empty() || period.len() > 20 { return Err("Invalid period".into()); }
    if timeframe.is_empty() || timeframe.len() > 20 { return Err("Invalid timeframe".into()); }
    let broker = {
        let s = state.lock().await;
        s.broker.as_ref().ok_or("Not connected")?.clone()
    };
    let history = broker.get_portfolio_history(&period, &timeframe).await?;
    Ok(serde_json::to_string(&history).map_err(|e| format!("JSON error: {e}"))?)
}

#[tauri::command]
async fn get_market_clock(state: State<'_, SharedState>) -> Result<String, String> {
    let broker = {
        let s = state.lock().await;
        s.broker.as_ref().ok_or("Not connected")?.clone()
    };
    let clock = broker.get_market_clock().await?;
    Ok(serde_json::to_string(&clock).map_err(|e| format!("JSON error: {e}"))?)
}

#[tauri::command]
async fn get_finnhub_recommendations(state: State<'_, SharedState>, symbol: String, finnhub_key: String) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    if finnhub_key.is_empty() || finnhub_key.len() > 100 { return Err("Invalid Finnhub API key".into()); }
    let broker = {
        let s = state.lock().await;
        s.broker.as_ref().ok_or("Not connected")?.clone()
    };
    let data = broker.get_finnhub_recommendations(&symbol, &finnhub_key).await?;
    Ok(serde_json::to_string(&data).map_err(|e| format!("JSON error: {e}"))?)
}

#[tauri::command]
async fn get_finnhub_price_target(state: State<'_, SharedState>, symbol: String, finnhub_key: String) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    if finnhub_key.is_empty() || finnhub_key.len() > 100 { return Err("Invalid Finnhub API key".into()); }
    let broker = {
        let s = state.lock().await;
        s.broker.as_ref().ok_or("Not connected")?.clone()
    };
    let data = broker.get_finnhub_price_target(&symbol, &finnhub_key).await?;
    Ok(serde_json::to_string(&data).map_err(|e| format!("JSON error: {e}"))?)
}

#[tauri::command]
async fn get_finnhub_insider_sentiment(state: State<'_, SharedState>, symbol: String, finnhub_key: String) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    if finnhub_key.is_empty() || finnhub_key.len() > 100 { return Err("Invalid Finnhub API key".into()); }
    let broker = {
        let s = state.lock().await;
        s.broker.as_ref().ok_or("Not connected")?.clone()
    };
    let data = broker.get_finnhub_insider_sentiment(&symbol, &finnhub_key).await?;
    Ok(serde_json::to_string(&data).map_err(|e| format!("JSON error: {e}"))?)
}

#[tauri::command]
async fn fetch_fear_greed() -> Result<String, String> {
    let resp = shared_client()
        .get("https://api.alternative.me/fng/?limit=30&format=json")
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("Fear & Greed request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Fear & Greed: HTTP {}", resp.status()));
    }
    resp.text().await.map_err(|e| format!("Fear & Greed read failed: {e}"))
}

#[tauri::command]
async fn fetch_treasury_yields() -> Result<String, String> {
    let resp = shared_client()
        .get("https://home.treasury.gov/resource-center/data-chart-center/interest-rates/daily-treasury-rates.csv/all/2026")
        .query(&[
            ("type", "daily_treasury_yield_curve"),
            ("field_tdr_date_value", "2026"),
            ("page", ""),
            ("_format", "csv"),
        ])
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| format!("Treasury yields request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Treasury yields: HTTP {}", resp.status()));
    }
    let csv_text = resp.text().await.map_err(|e| format!("Treasury yields read failed: {e}"))?;

    // Parse CSV into JSON array
    let mut lines = csv_text.lines();
    let header = lines.next().ok_or("Treasury CSV: no header row")?;
    let cols: Vec<&str> = header.split(',').collect();

    // Find column indices for the maturities we want
    let maturity_map: &[(&str, &str)] = &[
        ("1 Mo", "1mo"), ("3 Mo", "3mo"), ("6 Mo", "6mo"),
        ("1 Yr", "1yr"), ("2 Yr", "2yr"), ("5 Yr", "5yr"),
        ("10 Yr", "10yr"), ("20 Yr", "20yr"), ("30 Yr", "30yr"),
    ];

    let mut results: Vec<serde_json::Value> = Vec::new();
    for line in lines {
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() < 2 { continue; }
        let mut row = serde_json::json!({ "date": fields.first().unwrap_or(&"") });
        for (csv_col, json_key) in maturity_map {
            if let Some(idx) = cols.iter().position(|c| c.contains(csv_col)) {
                if let Some(val) = fields.get(idx) {
                    if let Ok(rate) = val.parse::<f64>() {
                        row[*json_key] = serde_json::json!(rate);
                    }
                }
            }
        }
        results.push(row);
    }

    serde_json::to_string(&results).map_err(|e| format!("Treasury JSON error: {e}"))
}

// ── Congress Trading ────────────────────────────────────────────────

#[tauri::command]
async fn fetch_congress_trades() -> Result<String, String> {
    let resp = shared_client()
        .get("https://house-stock-watcher-data.s3-us-west-2.amazonaws.com/data/all_transactions.json")
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("Congress trades request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Congress trades: HTTP {}", resp.status()));
    }
    resp.text().await.map_err(|e| format!("Congress trades read failed: {e}"))
}

// ── ECB Forex Rates ─────────────────────────────────────────────────

#[tauri::command]
async fn fetch_forex_rates() -> Result<String, String> {
    let resp = shared_client()
        .get("https://www.ecb.europa.eu/stats/eurofxref/eurofxref-daily.xml")
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("ECB forex request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("ECB forex: HTTP {}", resp.status()));
    }
    let xml = resp.text().await.map_err(|e| format!("ECB forex read failed: {e}"))?;

    // Parse currency="XXX" rate="Y.YYYY" pairs from the XML via string splitting
    let mut rates: Vec<serde_json::Value> = Vec::new();
    let mut usd_rate: Option<f64> = None;
    let mut gbp_rate: Option<f64> = None;
    let mut jpy_rate: Option<f64> = None;
    let mut chf_rate: Option<f64> = None;
    let mut cad_rate: Option<f64> = None;
    let mut aud_rate: Option<f64> = None;

    // Each <Cube currency="XXX" rate="Y.YYYY"/> appears on its own or inline
    for segment in xml.split("currency=\"") {
        // segment starts with e.g. `USD" rate="1.0856"/>`
        let Some(cur_end) = segment.find('"') else { continue };
        let currency = &segment[..cur_end];
        if currency.len() != 3 || !currency.chars().all(|c| c.is_ascii_uppercase()) {
            continue;
        }
        // Find rate="..." after the currency
        let Some(rate_start) = segment.find("rate=\"") else { continue };
        let after_rate = &segment[rate_start + 6..];
        let Some(rate_end) = after_rate.find('"') else { continue };
        let Ok(rate) = after_rate[..rate_end].parse::<f64>() else { continue };

        // Track specific rates for cross-rate computation
        match currency {
            "USD" => usd_rate = Some(rate),
            "GBP" => gbp_rate = Some(rate),
            "JPY" => jpy_rate = Some(rate),
            "CHF" => chf_rate = Some(rate),
            "CAD" => cad_rate = Some(rate),
            "AUD" => aud_rate = Some(rate),
            _ => {}
        }

        rates.push(serde_json::json!({
            "currency": currency,
            "rate": rate
        }));
    }

    // Add computed USD-based cross rates
    if let Some(usd) = usd_rate {
        // EUR/USD is just the USD rate itself (ECB rates are per 1 EUR)
        rates.push(serde_json::json!({ "currency": "EUR/USD", "rate": (1.0 / usd) }));
        if let Some(gbp) = gbp_rate {
            // USD/GBP = USD_per_EUR / GBP_per_EUR
            rates.push(serde_json::json!({ "currency": "GBP/USD", "rate": (usd / gbp) }));
        }
        if let Some(jpy) = jpy_rate {
            rates.push(serde_json::json!({ "currency": "USD/JPY", "rate": (jpy / usd) }));
        }
        if let Some(chf) = chf_rate {
            rates.push(serde_json::json!({ "currency": "USD/CHF", "rate": (chf / usd) }));
        }
        if let Some(cad) = cad_rate {
            rates.push(serde_json::json!({ "currency": "USD/CAD", "rate": (cad / usd) }));
        }
        if let Some(aud) = aud_rate {
            rates.push(serde_json::json!({ "currency": "AUD/USD", "rate": (usd / aud) }));
        }
    }

    serde_json::to_string(&rates).map_err(|e| format!("ECB forex JSON error: {e}"))
}

// ── CoinGecko Trending + Market ─────────────────────────────────────

#[tauri::command]
async fn fetch_crypto_trending() -> Result<String, String> {
    let resp = shared_client()
        .get("https://api.coingecko.com/api/v3/search/trending")
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("CoinGecko trending request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("CoinGecko trending: HTTP {}", resp.status()));
    }
    resp.text().await.map_err(|e| format!("CoinGecko trending read failed: {e}"))
}

#[tauri::command]
async fn fetch_crypto_market() -> Result<String, String> {
    let resp = shared_client()
        .get("https://api.coingecko.com/api/v3/coins/markets")
        .query(&[
            ("vs_currency", "usd"),
            ("order", "market_cap_desc"),
            ("per_page", "50"),
            ("page", "1"),
            ("sparkline", "true"),
        ])
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| format!("CoinGecko market request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("CoinGecko market: HTTP {}", resp.status()));
    }
    resp.text().await.map_err(|e| format!("CoinGecko market read failed: {e}"))
}

// ── Reddit WSB Mentions ─────────────────────────────────────────────

#[tauri::command]
async fn fetch_reddit_mentions(symbol: String) -> Result<String, String> {
    if !is_valid_symbol(&symbol) {
        return Err("Invalid symbol".into());
    }
    let url = format!(
        "https://www.reddit.com/r/wallstreetbets/search.json?q={}&sort=new&t=week&limit=25",
        symbol
    );
    let resp = shared_client()
        .get(&url)
        .header("User-Agent", "TyphooN-Terminal/1.0 (trading dashboard)")
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("Reddit mentions request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Reddit mentions: HTTP {}", resp.status()));
    }
    resp.text().await.map_err(|e| format!("Reddit mentions read failed: {e}"))
}

#[tauri::command]
async fn run_walk_forward(
    state: State<'_, SharedState>,
    symbol: String,
    timeframe: String,
    fast_min: usize,
    fast_max: usize,
    slow_min: usize,
    slow_max: usize,
    initial_equity: Option<f64>,
    in_sample_pct: Option<f64>,
    limit: Option<u32>,
) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    if !is_valid_timeframe(&timeframe) { return Err("Invalid timeframe".into()); }

    let equity = initial_equity.unwrap_or(100_000.0);
    if equity <= 0.0 || !equity.is_finite() { return Err("Invalid initial equity".into()); }

    let is_pct = in_sample_pct.unwrap_or(70.0).clamp(30.0, 90.0) / 100.0;
    let bar_limit = limit.unwrap_or(5000).min(50_000);

    if fast_min < 2 || fast_max > 200 || slow_min < 3 || slow_max > 500 {
        return Err("Period ranges out of bounds (fast: 2-200, slow: 3-500)".into());
    }

    let bars = {
        let broker = {
            let s = state.lock().await;
            s.broker.as_ref().ok_or("Not connected")?.clone()
        };
        broker.get_bars(&symbol, &timeframe, bar_limit).await?
    };

    if bars.len() < 50 {
        return Err("Insufficient bar data for walk-forward test".into());
    }

    let split = (bars.len() as f64 * is_pct) as usize;
    let in_sample = &bars[..split];
    let out_sample = &bars[split..];

    // Optimize on in-sample
    let opt_result = backtest_engine::optimize_sma_cross(
        in_sample,
        (fast_min, fast_max),
        (slow_min, slow_max),
        equity,
        1,
    );

    if opt_result.results.is_empty() {
        return Err("Optimization produced no results".into());
    }

    let best = &opt_result.results[0];
    let best_fast = best.fast_period;
    let best_slow = best.slow_period;

    // Run best params on in-sample (full result)
    let mut is_strat = SMACrossStrategy::new(best_fast, best_slow);
    let is_result = backtest_engine::run_backtest(in_sample, &mut is_strat, equity);

    // Run best params on out-of-sample
    let mut os_strat = SMACrossStrategy::new(best_fast, best_slow);
    let os_result = backtest_engine::run_backtest(out_sample, &mut os_strat, equity);

    let result = serde_json::json!({
        "best_fast": best_fast,
        "best_slow": best_slow,
        "in_sample_bars": in_sample.len(),
        "out_sample_bars": out_sample.len(),
        "in_sample": {
            "report": is_result.report,
            "trades": is_result.trades,
            "equity_curve": is_result.equity_curve,
        },
        "out_sample": {
            "report": os_result.report,
            "trades": os_result.trades,
            "equity_curve": os_result.equity_curve,
        },
    });

    Ok(serde_json::to_string(&result).map_err(|e| format!("JSON error: {e}"))?)
}

#[tauri::command]
async fn get_sec_filings(symbol: String, filing_type: String) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    let result = broker::alpaca::AlpacaBroker::get_sec_filings(&symbol, &filing_type, 20).await?;
    Ok(serde_json::to_string(&result).map_err(|e| format!("JSON error: {e}"))?)
}

#[tauri::command]
async fn get_company_fundamentals(symbol: String) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    let result = broker::alpaca::AlpacaBroker::get_sec_company_facts(&symbol).await?;
    Ok(serde_json::to_string(&result).map_err(|e| format!("JSON error: {e}"))?)
}

// ── Bid/Ask Quote Command ────────────────────────────────────────────

#[tauri::command]
async fn get_latest_quote(state: State<'_, SharedState>, symbol: String) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    let broker = {
        let s = state.lock().await;
        s.broker.as_ref().ok_or("Not connected")?.clone()
    };
    let quote = broker.get_latest_quote(&symbol).await?;
    Ok(serde_json::to_string(&quote).map_err(|e| format!("JSON error: {e}"))?)
}

// ── Account Activities Command ──────────────────────────────────────

#[tauri::command]
async fn get_account_activities(
    state: State<'_, SharedState>,
    activity_types: String,
    limit: u32,
) -> Result<String, String> {
    // Validate activity_types: comma-separated alphanumeric codes
    if activity_types.len() > 200 {
        return Err("Activity types string too long".into());
    }
    if !activity_types.is_empty() && !activity_types.chars().all(|c| c.is_ascii_alphanumeric() || c == ',') {
        return Err("Invalid activity types format".into());
    }
    let limit = limit.min(200);
    let broker = {
        let s = state.lock().await;
        s.broker.as_ref().ok_or("Not connected")?.clone()
    };
    let activities = broker.get_account_activities(&activity_types, limit).await?;
    Ok(serde_json::to_string(&activities).map_err(|e| format!("JSON error: {e}"))?)
}

// ── Insider Trading Command ─────────────────────────────────────────

#[tauri::command]
async fn get_insider_trades(symbol: String) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    let trades = broker::alpaca::AlpacaBroker::get_insider_trades(&symbol).await?;
    Ok(serde_json::to_string(&trades).map_err(|e| format!("JSON error: {e}"))?)
}

/// Fetch article content from URL, return as text. For in-app reading.
/// Hardened: HTTPS only, 10s timeout, 2MB max response.
#[tauri::command]
async fn fetch_article(url: String) -> Result<String, String> {
    if !url.starts_with("https://") {
        return Err("Only HTTPS URLs allowed".to_string());
    }
    let client = shared_client();
    let resp = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(10))
        .header("User-Agent", "Mozilla/5.0 (compatible)")
        .send()
        .await
        .map_err(|_| "Article fetch failed".to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    // Limit response body to 2MB
    let bytes = resp.bytes().await.map_err(|_| "Read failed".to_string())?;
    if bytes.len() > 2 * 1024 * 1024 {
        return Err("Response too large".to_string());
    }
    String::from_utf8(bytes.to_vec()).map_err(|_| "Invalid UTF-8".to_string())
}

/// Clear all cached data for a specific symbol from cold storage.
/// Hardened: validates symbol, ensures deletions stay within cache directory.
#[tauri::command]
async fn clear_symbol_cache(symbol: String) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    let dir = get_cache_dir();
    let canonical_dir = std::fs::canonicalize(&dir).map_err(|e| format!("Cache dir error: {e}"))?;
    let prefix = symbol.replace('/', "_");
    let mut removed = 0;
    if let Ok(mut entries) = tokio::fs::read_dir(&dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            // Ensure file is actually inside cache directory (prevent symlink traversal)
            let entry_path = entry.path();
            if let Ok(canonical_entry) = std::fs::canonicalize(&entry_path) {
                if !canonical_entry.starts_with(&canonical_dir) { continue; }
            } else {
                continue;
            }
            if let Some(name) = entry.file_name().to_str() {
                if !name.ends_with(".zst") { continue; }
                if name.starts_with(&prefix) || name.contains(&format!("_{prefix}")) {
                    tokio::fs::remove_file(entry_path).await.ok();
                    removed += 1;
                }
            }
        }
    }
    tracing::info!("Cleared {removed} cache files for {symbol}");
    Ok(format!("Cleared {removed} files"))
}

// ── Backtest Commands ────────────────────────────────────────────

#[tauri::command]
async fn run_backtest(
    state: State<'_, SharedState>,
    symbol: String,
    timeframe: String,
    strategy: String,
    fast_period: Option<usize>,
    slow_period: Option<usize>,
    initial_equity: Option<f64>,
    limit: Option<u32>,
) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    if !is_valid_timeframe(&timeframe) { return Err("Invalid timeframe".into()); }

    let equity = initial_equity.unwrap_or(100_000.0);
    if equity <= 0.0 || !equity.is_finite() { return Err("Invalid initial equity".into()); }

    let bar_limit = limit.unwrap_or(5000).min(50_000);

    // Fetch bars
    let bars = {
        let broker = {
            let s = state.lock().await;
            s.broker.as_ref().ok_or("Not connected")?.clone()
        };
        broker.get_bars(&symbol, &timeframe, bar_limit).await?
    };

    if bars.len() < 2 {
        return Err("Insufficient bar data for backtest".into());
    }

    // Create strategy
    let result: BacktestResult = match strategy.as_str() {
        "sma_cross" | "SMA Cross" => {
            let fast = fast_period.unwrap_or(10);
            let slow = slow_period.unwrap_or(20);
            if fast >= slow { return Err("fast_period must be < slow_period".into()); }
            if slow > bars.len() { return Err("Not enough bars for slow period".into()); }
            let mut strat = SMACrossStrategy::new(fast, slow);
            backtest_engine::run_backtest(&bars, &mut strat, equity)
        }
        "nnfx" | "NNFX" | "NNFX (KAMA+Fisher)" => {
            let kama = fast_period.unwrap_or(10);
            let fisher = slow_period.unwrap_or(32);
            let mut strat = NNFXStrategy::new(kama, fisher);
            backtest_engine::run_backtest(&bars, &mut strat, equity)
        }
        _ => return Err(format!("Unknown strategy: {strategy}. Available: sma_cross, nnfx")),
    };

    Ok(serde_json::to_string(&result).map_err(|e| format!("JSON error: {e}"))?)
}

// ── CSV Export Commands ─────────────────────────────────────────────

#[tauri::command]
async fn export_trade_history(
    state: State<'_, SharedState>,
    limit: Option<u32>,
) -> Result<String, String> {
    let limit = limit.unwrap_or(500).min(500);
    let broker = {
        let s = state.lock().await;
        s.broker.as_ref().ok_or("Not connected")?.clone()
    };
    let orders = broker.get_orders("closed", limit).await?;

    // CSV-safe escaping: quote fields that may contain commas/quotes
    fn csv_escape(s: &str) -> String {
        if s.contains(',') || s.contains('"') || s.contains('\n') {
            format!("\"{}\"", s.replace('"', "\"\""))
        } else {
            s.to_string()
        }
    }
    let mut csv = String::from("id,symbol,side,qty,filled_qty,order_type,status,limit_price,stop_price,created_at,filled_at,filled_avg_price\n");
    for o in &orders {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{}\n",
            csv_escape(&o.id),
            csv_escape(&o.symbol),
            csv_escape(&o.side),
            csv_escape(&o.qty),
            csv_escape(&o.filled_qty),
            csv_escape(&o.order_type),
            csv_escape(&o.status),
            csv_escape(o.limit_price.as_deref().unwrap_or("")),
            csv_escape(o.stop_price.as_deref().unwrap_or("")),
            csv_escape(&o.created_at),
            csv_escape(o.filled_at.as_deref().unwrap_or("")),
            csv_escape(o.filled_avg_price.as_deref().unwrap_or("")),
        ));
    }
    Ok(csv)
}

// ── Options Commands ────────────────────────────────────────────────

#[tauri::command]
async fn get_options(
    state: State<'_, SharedState>,
    symbol: String,
    expiry: String,
) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    // Validate expiry format: YYYY-MM-DD
    if expiry.len() != 10 || expiry.chars().nth(4) != Some('-') || expiry.chars().nth(7) != Some('-') {
        return Err("Invalid expiry format (expected YYYY-MM-DD)".into());
    }
    let broker = {
        let s = state.lock().await;
        s.broker.as_ref().ok_or("Not connected")?.clone()
    };
    let chain = broker.get_options_chain(&symbol, &expiry).await?;
    Ok(serde_json::to_string(&chain).map_err(|e| format!("JSON error: {e}"))?)
}

// ── Screener Commands ───────────────────────────────────────────────

#[tauri::command]
async fn run_screener(
    _state: State<'_, SharedState>,
    filters_json: String,
    symbols_json: String,
) -> Result<String, String> {
    if filters_json.len() > 4096 { return Err("Filters too large".into()); }
    if symbols_json.len() > 10 * 1024 * 1024 { return Err("Symbol data too large".into()); }

    let filters: ScreenerFilter = serde_json::from_str(&filters_json)
        .map_err(|e| format!("Invalid filters: {e}"))?;
    let symbols: Vec<ScreenerSymbol> = serde_json::from_str(&symbols_json)
        .map_err(|e| format!("Invalid symbols data: {e}"))?;

    let result = screener_engine::screen_symbols(&filters, &symbols);
    Ok(serde_json::to_string(&result).map_err(|e| format!("JSON error: {e}"))?)
}

// ── WebSocket Streaming Commands ────────────────────────────────────

#[tauri::command]
async fn start_streaming(
    state: State<'_, SharedState>,
    trade_symbols: Vec<String>,
    quote_symbols: Vec<String>,
) -> Result<String, String> {
    // Validate all symbols
    for sym in trade_symbols.iter().chain(quote_symbols.iter()) {
        if !is_valid_symbol(sym) { return Err(format!("Invalid symbol: {sym}")); }
    }
    if trade_symbols.is_empty() && quote_symbols.is_empty() {
        return Err("Must provide at least one trade or quote symbol".into());
    }
    if trade_symbols.len() + quote_symbols.len() > 100 {
        return Err("Too many symbols (max 100)".into());
    }

    // Clone broker and drop lock before WebSocket connect
    let broker = {
        let mut s = state.lock().await;
        s.stream_rx = None; // Drop any existing stream
        s.broker.as_ref().ok_or("Not connected")?.clone()
    };

    let rx = broker.start_stream(trade_symbols.clone(), quote_symbols.clone()).await?;
    // Re-acquire lock to store the stream receiver
    let mut s = state.lock().await;
    s.stream_rx = Some(rx);

    Ok(serde_json::to_string(&serde_json::json!({
        "status": "streaming",
        "trades": trade_symbols,
        "quotes": quote_symbols,
    })).unwrap())
}

#[tauri::command]
async fn poll_stream(state: State<'_, SharedState>) -> Result<String, String> {
    let mut s = state.lock().await;

    // Drain messages from stream into a local vec first (avoids double borrow)
    let mut messages = Vec::new();
    let mut disconnected = false;
    if let Some(rx) = s.stream_rx.as_mut() {
        for _ in 0..100 {
            match rx.try_recv() {
                Ok(msg) => messages.push(msg),
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                    disconnected = true;
                    break;
                }
            }
        }
    } else {
        return Err("No active stream".into());
    }
    if disconnected { s.stream_rx = None; }

    // Feed trades into bar builder for live candle construction
    for msg in &messages {
        if let StreamMessage::Trade(trade) = msg {
            s.bar_builder.ingest_trade(
                &trade.symbol, trade.price, trade.size, &trade.timestamp
            );
        }
    }

    Ok(serde_json::to_string(&messages).map_err(|e| format!("JSON error: {e}"))?)
}

/// Poll completed 1-minute bars built from WebSocket trade stream.
/// Returns completed bars (minute has ended) + current active (forming) bars.
/// Frontend can use these instead of the 10-second API polling loop.
#[tauri::command]
async fn poll_bars(state: State<'_, SharedState>) -> Result<String, String> {
    let mut s = state.lock().await;
    let completed = s.bar_builder.drain_completed();
    let active = s.bar_builder.get_all_active_bars();
    Ok(serde_json::json!({
        "completed": completed,
        "active": active,
    }).to_string())
}

#[tauri::command]
async fn stop_streaming(state: State<'_, SharedState>) -> Result<(), String> {
    let mut s = state.lock().await;
    s.stream_rx = None;
    Ok(())
}

// ── FRED API Commands ───────────────────────────────────────────────

#[tauri::command]
async fn fetch_fred_series(series_id: String, api_key: String, limit: Option<u32>) -> Result<String, String> {
    if series_id.is_empty() || series_id.len() > 50 || !series_id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err("Invalid FRED series ID".into());
    }
    if api_key.is_empty() || api_key.len() > 64 || !api_key.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err("Invalid FRED API key".into());
    }
    let limit = limit.unwrap_or(100).min(10000);
    let client = shared_client();

    let resp = client
        .get("https://api.stlouisfed.org/fred/series/observations")
        .timeout(std::time::Duration::from_secs(10))
        .query(&[
            ("series_id", series_id.as_str()),
            ("api_key", api_key.as_str()),
            ("file_type", "json"),
            ("sort_order", "desc"),
            ("limit", &limit.to_string()),
        ])
        .send()
        .await
        .map_err(|_| "FRED request failed".to_string())?;

    if !resp.status().is_success() {
        let status = resp.status();
        let _ = resp.text().await;
        return Err(format!("FRED request failed: HTTP {status}"));
    }

    let json: serde_json::Value = resp.json().await
        .map_err(|_| "FRED parse failed".to_string())?;
    Ok(serde_json::to_string(&json).map_err(|e| format!("JSON error: {e}"))?)
}

// ── AI Chat Command ─────────────────────────────────────────────────

#[tauri::command]
async fn ai_chat(
    api_key: String,
    provider: String,
    model: String,
    message: String,
    context: Option<String>,
) -> Result<String, String> {
    if api_key.is_empty() || api_key.len() > 200 {
        return Err("Invalid API key".into());
    }
    if message.is_empty() || message.len() > 10_000 {
        return Err("Message must be 1-10000 chars".into());
    }
    let client = shared_client();

    let system_prompt = "You are a trading assistant for TyphooN-Terminal. Help with market analysis, risk management, and trading decisions. Be concise.";
    let ctx = context.unwrap_or_default();
    let full_msg = if ctx.is_empty() { message.clone() } else { format!("{ctx}\n\nUser: {message}") };

    match provider.as_str() {
        "anthropic" => {
            let body = serde_json::json!({
                "model": if model.is_empty() { "claude-haiku-4-5-20251001" } else { model.as_str() },
                "max_tokens": 1024,
                "system": system_prompt,
                "messages": [{ "role": "user", "content": full_msg }],
            });
            let resp = client
                .post("https://api.anthropic.com/v1/messages")
                .timeout(std::time::Duration::from_secs(60))
                .header("x-api-key", &api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|_| "Anthropic request failed".to_string())?;

            if !resp.status().is_success() {
                let status = resp.status();
                let _ = resp.text().await;
                return Err(format!("Anthropic: HTTP {status}"));
            }
            let json: serde_json::Value = resp.json().await
                .map_err(|_| "Anthropic parse failed".to_string())?;
            let text = json["content"][0]["text"].as_str().unwrap_or("No response");
            Ok(text.to_string())
        }
        "openai" => {
            let body = serde_json::json!({
                "model": if model.is_empty() { "gpt-4o-mini" } else { model.as_str() },
                "max_tokens": 1024,
                "messages": [
                    { "role": "system", "content": system_prompt },
                    { "role": "user", "content": full_msg },
                ],
            });
            let resp = client
                .post("https://api.openai.com/v1/chat/completions")
                .timeout(std::time::Duration::from_secs(60))
                .header("Authorization", format!("Bearer {api_key}"))
                .header("content-type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|_| "OpenAI request failed".to_string())?;

            if !resp.status().is_success() {
                let status = resp.status();
                let _ = resp.text().await;
                return Err(format!("OpenAI: HTTP {status}"));
            }
            let json: serde_json::Value = resp.json().await
                .map_err(|_| "OpenAI parse failed".to_string())?;
            let text = json["choices"][0]["message"]["content"].as_str().unwrap_or("No response");
            Ok(text.to_string())
        }
        _ => Err("Provider must be 'anthropic' or 'openai'".into()),
    }
}

// ── Matrix Community Chat ────────────────────────────────────────────

const MATRIX_DEFAULT_ROOM: &str = "!placeholder:matrix.org"; // replaced by actual room ID at runtime

/// Log in to a Matrix homeserver and get an access token.
#[tauri::command]
async fn matrix_login(homeserver: String, username: String, password: String) -> Result<String, String> {
    if homeserver.is_empty() || !homeserver.starts_with("https://") {
        return Err("Homeserver must be an HTTPS URL".into());
    }
    if username.is_empty() || username.len() > 200 { return Err("Invalid username".into()); }
    if password.is_empty() || password.len() > 200 { return Err("Invalid password".into()); }

    let client = shared_client();
    let body = serde_json::json!({
        "type": "m.login.password",
        "identifier": { "type": "m.id.user", "user": username },
        "password": password,
    });

    let resp = client
        .post(format!("{}/_matrix/client/v3/login", homeserver))
        .timeout(std::time::Duration::from_secs(15))
        .json(&body)
        .send()
        .await
        .map_err(|_| "Matrix login request failed".to_string())?;

    if !resp.status().is_success() {
        let status = resp.status();
        let _ = resp.text().await;
        return Err(format!("Matrix login failed: HTTP {status}"));
    }

    let json: serde_json::Value = resp.json().await
        .map_err(|_| "Matrix login parse failed".to_string())?;

    Ok(serde_json::to_string(&serde_json::json!({
        "access_token": json["access_token"],
        "user_id": json["user_id"],
        "device_id": json["device_id"],
    })).unwrap())
}

/// Send a message to a Matrix room.
#[tauri::command]
async fn matrix_send(homeserver: String, access_token: String, room_id: String, message: String) -> Result<(), String> {
    if access_token.is_empty() || access_token.len() > 500 { return Err("Invalid access token".into()); }
    if room_id.is_empty() || room_id.len() > 200 { return Err("Invalid room ID".into()); }
    if message.is_empty() || message.len() > 4096 { return Err("Message must be 1-4096 chars".into()); }

    let client = shared_client();
    let txn_id = format!("tt_{}", chrono::Utc::now().timestamp_millis());
    let encoded_room = room_id.replace('!', "%21").replace(':', "%3A");

    let body = serde_json::json!({
        "msgtype": "m.text",
        "body": message,
    });

    let resp = client
        .put(format!("{}/_matrix/client/v3/rooms/{}/send/m.room.message/{}", homeserver, encoded_room, txn_id))
        .timeout(std::time::Duration::from_secs(10))
        .header("Authorization", format!("Bearer {access_token}"))
        .json(&body)
        .send()
        .await
        .map_err(|_| "Matrix send failed".to_string())?;

    if !resp.status().is_success() {
        let status = resp.status();
        let _ = resp.text().await;
        return Err(format!("Matrix send failed: HTTP {status}"));
    }
    Ok(())
}

/// Join a Matrix room by alias or ID.
#[tauri::command]
async fn matrix_join(homeserver: String, access_token: String, room: String) -> Result<String, String> {
    if access_token.is_empty() { return Err("Not logged in".into()); }
    if room.is_empty() || room.len() > 200 { return Err("Invalid room".into()); }

    let client = shared_client();
    let encoded_room = room.replace('#', "%23").replace('!', "%21").replace(':', "%3A");

    let resp = client
        .post(format!("{}/_matrix/client/v3/join/{}", homeserver, encoded_room))
        .timeout(std::time::Duration::from_secs(10))
        .header("Authorization", format!("Bearer {access_token}"))
        .json(&serde_json::json!({}))
        .send()
        .await
        .map_err(|_| "Matrix join failed".to_string())?;

    if !resp.status().is_success() {
        let status = resp.status();
        let _ = resp.text().await;
        return Err(format!("Matrix join failed: HTTP {status}"));
    }

    let json: serde_json::Value = resp.json().await
        .map_err(|_| "Matrix join parse failed".to_string())?;

    let room_id = json["room_id"].as_str().unwrap_or("").to_string();
    Ok(room_id)
}

/// Poll messages from a Matrix room using /sync.
#[tauri::command]
async fn matrix_poll(homeserver: String, access_token: String, since: Option<String>) -> Result<String, String> {
    if access_token.is_empty() { return Err("Not logged in".into()); }

    let client = shared_client();
    let filter = serde_json::json!({
        "room": {
            "timeline": { "limit": 50 },
            "state": { "lazy_load_members": true },
        },
    });

    let mut url = format!("{}/_matrix/client/v3/sync?timeout=5000&filter={}", homeserver, filter);
    if let Some(ref s) = since {
        url.push_str(&format!("&since={s}"));
    }

    let resp = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(15))
        .header("Authorization", format!("Bearer {access_token}"))
        .send()
        .await
        .map_err(|_| "Matrix sync failed".to_string())?;

    if !resp.status().is_success() {
        let status = resp.status();
        let _ = resp.text().await;
        return Err(format!("Matrix sync failed: HTTP {status}"));
    }

    let json: serde_json::Value = resp.json().await
        .map_err(|_| "Matrix sync parse failed".to_string())?;

    // Extract messages from all joined rooms
    let next_batch = json["next_batch"].as_str().unwrap_or("").to_string();
    let mut messages = Vec::new();

    if let Some(rooms) = json["rooms"]["join"].as_object() {
        for (room_id, room_data) in rooms {
            if let Some(events) = room_data["timeline"]["events"].as_array() {
                for event in events {
                    if event["type"].as_str() == Some("m.room.message") {
                        let sender = event["sender"].as_str().unwrap_or("");
                        let body = event["content"]["body"].as_str().unwrap_or("");
                        let ts = event["origin_server_ts"].as_u64().unwrap_or(0);
                        messages.push(serde_json::json!({
                            "room_id": room_id,
                            "sender": sender,
                            "body": body,
                            "timestamp": ts,
                        }));
                    }
                }
            }
        }
    }

    Ok(serde_json::to_string(&serde_json::json!({
        "next_batch": next_batch,
        "messages": messages,
    })).unwrap())
}

// ── Push Notification Commands ──────────────────────────────────────

#[tauri::command]
async fn send_pushover_notification(
    token: String,
    user: String,
    message: String,
) -> Result<(), String> {
    notifications::send_pushover(&token, &user, &message).await
}

#[tauri::command]
async fn send_ntfy_notification(
    topic: String,
    message: String,
) -> Result<(), String> {
    notifications::send_ntfy(&topic, &message).await
}

// ── FINRA Short Interest (Finnhub) ──────────────────────────────

#[tauri::command]
async fn fetch_short_interest(state: State<'_, SharedState>, symbol: String, finnhub_key: String) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    if finnhub_key.is_empty() || finnhub_key.len() > 100 { return Err("Invalid Finnhub API key".into()); }
    let broker = {
        let s = state.lock().await;
        s.broker.as_ref().ok_or("Not connected")?.clone()
    };
    let data = broker.get_finnhub_short_interest(&symbol, &finnhub_key).await?;
    Ok(serde_json::to_string(&data).map_err(|e| format!("JSON error: {e}"))?)
}

// ── World Equity Indices (Yahoo Finance) ────────────────────────

#[tauri::command]
async fn fetch_world_indices() -> Result<String, String> {
    let symbols = "^GSPC,^DJI,^IXIC,^RUT,^VIX,^FTSE,^GDAXI,^FCHI,^N225,^HSI,^STI,^AXJO,^BVSP,^GSPTSE";
    let resp = shared_client()
        .get("https://query1.finance.yahoo.com/v7/finance/quote")
        .query(&[("symbols", symbols)])
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| format!("Yahoo Finance request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Yahoo Finance: HTTP {}", resp.status()));
    }

    let body: serde_json::Value = resp.json().await
        .map_err(|e| format!("Yahoo Finance parse failed: {e}"))?;

    // Yahoo wraps results in quoteResponse.result[]
    let quotes = body
        .pointer("/quoteResponse/result")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "Yahoo Finance: unexpected response format".to_string())?;

    let indices: Vec<serde_json::Value> = quotes.iter().map(|q| {
        serde_json::json!({
            "symbol": q["symbol"].as_str().unwrap_or(""),
            "name": q["shortName"].as_str().unwrap_or(q["longName"].as_str().unwrap_or("")),
            "price": q["regularMarketPrice"].as_f64().unwrap_or(0.0),
            "change": q["regularMarketChange"].as_f64().unwrap_or(0.0),
            "changePct": q["regularMarketChangePercent"].as_f64().unwrap_or(0.0),
        })
    }).collect();

    serde_json::to_string(&indices).map_err(|e| format!("Indices JSON error: {e}"))
}

// ── Alpaca Watchlist Sync ───────────────────────────────────────

#[tauri::command]
async fn get_alpaca_watchlists(state: State<'_, SharedState>) -> Result<String, String> {
    let broker = {
        let s = state.lock().await;
        s.broker.as_ref().ok_or("Not connected")?.clone()
    };
    let watchlists = broker.get_watchlists().await?;
    Ok(serde_json::to_string(&watchlists).map_err(|e| format!("JSON error: {e}"))?)
}

#[tauri::command]
async fn sync_watchlist_to_alpaca(
    state: State<'_, SharedState>,
    name: String,
    symbols: Vec<String>,
) -> Result<String, String> {
    if name.is_empty() || name.len() > 64 {
        return Err("Watchlist name must be 1-64 characters".into());
    }
    for s in &symbols {
        if !is_valid_symbol(s) {
            return Err(format!("Invalid symbol: {s}"));
        }
    }
    let broker = {
        let s = state.lock().await;
        s.broker.as_ref().ok_or("Not connected")?.clone()
    };

    // Check if a watchlist with this name already exists
    let existing = broker.get_watchlists().await?;
    for wl in &existing {
        if wl["name"].as_str() == Some(&name) {
            if let Some(id) = wl["id"].as_str() {
                let updated = broker.update_watchlist(id, &symbols).await?;
                return Ok(serde_json::to_string(&updated).map_err(|e| format!("JSON error: {e}"))?);
            }
        }
    }

    // No existing watchlist — create a new one
    let created = broker.create_watchlist(&name, &symbols).await?;
    Ok(serde_json::to_string(&created).map_err(|e| format!("JSON error: {e}"))?)
}

// ── SQLite Cache Commands ───────────────────────────────────────

#[tauri::command]
async fn db_cache_put(state: State<'_, SharedState>, key: String, data: String, kind: Option<String>) -> Result<(), String> {
    if key.len() > 500 { return Err("Key too long".into()); }
    if data.len() > 50 * 1024 * 1024 { return Err("Data too large".into()); }
    let s = state.lock().await;
    let cache = s.db_cache.as_ref().ok_or("SQLite cache not available")?;
    let kind = kind.unwrap_or_else(|| "kv".to_string());
    if kind == "bars" {
        cache.put_bars(&key, &data)
    } else {
        cache.put_kv(&key, &data)
    }
}

#[tauri::command]
async fn db_cache_get(state: State<'_, SharedState>, key: String, kind: Option<String>) -> Result<String, String> {
    if key.len() > 500 { return Err("Key too long".into()); }
    let s = state.lock().await;
    let cache = s.db_cache.as_ref().ok_or("SQLite cache not available")?;
    let kind = kind.unwrap_or_else(|| "kv".to_string());
    if kind == "bars" {
        match cache.get_bars(&key)? {
            Some((json, _ts)) => Ok(json),
            None => Err("Not in cache".into()),
        }
    } else {
        match cache.get_kv(&key)? {
            Some(json) => Ok(json),
            None => Err("Not in cache".into()),
        }
    }
}

#[tauri::command]
async fn db_cache_stats(state: State<'_, SharedState>) -> Result<String, String> {
    let s = state.lock().await;
    let cache = s.db_cache.as_ref().ok_or("SQLite cache not available")?;
    let (bars, kvs, size) = cache.stats()?;
    Ok(serde_json::to_string(&serde_json::json!({
        "bar_entries": bars,
        "kv_entries": kvs,
        "total_compressed_bytes": size,
        "total_compressed_mb": (size as f64) / (1024.0 * 1024.0),
    })).unwrap())
}

#[tauri::command]
async fn db_cache_evict(state: State<'_, SharedState>, max_age_days: Option<i64>) -> Result<String, String> {
    let max_age = max_age_days.unwrap_or(30) * 86400; // default 30 days
    let s = state.lock().await;
    let cache = s.db_cache.as_ref().ok_or("SQLite cache not available")?;
    let deleted = cache.evict_old(max_age)?;
    Ok(format!("Evicted {deleted} entries older than {} days", max_age / 86400))
}

#[tauri::command]
async fn db_cache_detailed_stats(state: State<'_, SharedState>) -> Result<String, String> {
    let s = state.lock().await;
    let cache = s.db_cache.as_ref().ok_or("SQLite cache not available")?;
    let entries = cache.detailed_stats()?;
    let json_entries: Vec<serde_json::Value> = entries.iter().map(|(key, size, ts)| {
        let parts: Vec<&str> = key.splitn(2, ':').collect();
        let symbol = parts.first().unwrap_or(&"");
        let tf = parts.get(1).unwrap_or(&"");
        serde_json::json!({
            "key": key,
            "symbol": symbol,
            "timeframe": tf,
            "compressed_bytes": size,
            "timestamp": ts,
        })
    }).collect();
    Ok(serde_json::to_string(&json_entries).unwrap())
}

#[tauri::command]
async fn db_cache_delete_key(state: State<'_, SharedState>, key: String) -> Result<String, String> {
    let s = state.lock().await;
    let cache = s.db_cache.as_ref().ok_or("SQLite cache not available")?;
    let deleted = cache.delete_key(&key)?;
    Ok(if deleted { format!("Deleted {key}") } else { format!("Key not found: {key}") })
}

#[tauri::command]
async fn db_cache_delete_symbol(state: State<'_, SharedState>, symbol: String) -> Result<String, String> {
    let s = state.lock().await;
    let cache = s.db_cache.as_ref().ok_or("SQLite cache not available")?;
    let deleted = cache.delete_symbol(&symbol)?;
    Ok(format!("Deleted {deleted} entries for {symbol}"))
}

// ── Cold Cache (zstd-compressed files on disk) ──────────────────

fn get_cache_dir() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let dir = std::path::PathBuf::from(home).join(".config").join("typhoon-terminal").join("cache");
    std::fs::create_dir_all(&dir).ok();
    dir
}

fn cache_key_to_filename(key: &str) -> String {
    key.replace('/', "_").replace(':', "_") + ".zst"
}

#[tauri::command]
async fn save_cold_cache(key: String, data: String) -> Result<(), String> {
    // Validate cache key doesn't contain path traversal
    let filename = cache_key_to_filename(&key);
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return Err("Invalid cache key".to_string());
    }
    // Limit uncompressed data to 50MB (prevents disk exhaustion)
    if data.len() > 50 * 1024 * 1024 {
        return Err("Cache data too large".to_string());
    }
    let dir = get_cache_dir();
    let path = dir.join(&filename);
    let compressed = zstd::encode_all(data.as_bytes(), 3)
        .map_err(|e| format!("zstd compress failed: {e}"))?;
    let raw_size = data.len();
    let compressed_size = compressed.len();
    tokio::fs::write(&path, compressed).await
        .map_err(|e| format!("Cache write failed: {e}"))?;
    tracing::debug!(
        "Cold cache: {} → {} bytes ({:.1}x compression)",
        raw_size, compressed_size, raw_size as f64 / compressed_size.max(1) as f64
    );
    Ok(())
}

#[tauri::command]
async fn load_cold_cache(key: String) -> Result<String, String> {
    let filename = cache_key_to_filename(&key);
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return Err("Invalid cache key".to_string());
    }
    let dir = get_cache_dir();
    let path = dir.join(&filename);
    if !path.exists() {
        return Err("Not in cold cache".to_string());
    }
    let compressed = tokio::fs::read(&path).await
        .map_err(|e| format!("Cache read failed: {e}"))?;
    // Reject suspiciously large compressed files (>10MB compressed → could be a zstd bomb)
    if compressed.len() > 10 * 1024 * 1024 {
        return Err("Compressed cache file too large".to_string());
    }
    let decompressed = zstd::decode_all(compressed.as_slice())
        .map_err(|e| format!("zstd decompress failed: {e}"))?;
    // Cap decompressed size at 50MB
    if decompressed.len() > 50 * 1024 * 1024 {
        return Err("Decompressed data too large".to_string());
    }
    String::from_utf8(decompressed)
        .map_err(|e| format!("UTF-8 decode failed: {e}"))
}

#[tauri::command]
async fn list_cold_cache() -> Result<String, String> {
    let dir = get_cache_dir();
    let mut entries = Vec::new();
    if let Ok(mut read_dir) = tokio::fs::read_dir(&dir).await {
        while let Ok(Some(entry)) = read_dir.next_entry().await {
            if let Some(name) = entry.file_name().to_str() {
                if name.ends_with(".zst") {
                    let size = entry.metadata().await.map(|m| m.len()).unwrap_or(0);
                    entries.push(serde_json::json!({
                        "key": name.trim_end_matches(".zst").replace('_', ":"),
                        "size": size,
                    }));
                    if entries.len() >= 10_000 { break; } // cap listing
                }
            }
        }
    }
    Ok(serde_json::to_string(&entries).map_err(|e| format!("JSON error: {e}"))?)
}

// ── Financial Analysis Commands ──────────────────────────────────

#[tauri::command]
async fn get_financial_analysis(symbol: String) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    let result = AlpacaBroker::get_financial_analysis(&symbol).await?;
    Ok(serde_json::to_string(&result).map_err(|e| format!("JSON error: {e}"))?)
}

#[tauri::command]
async fn get_institutional_holders(symbol: String) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    let result = AlpacaBroker::get_institutional_holders(&symbol).await?;
    Ok(serde_json::to_string(&result).map_err(|e| format!("JSON error: {e}"))?)
}

// ── Most Active / Top Movers Commands ───────────────────────────

#[tauri::command]
async fn get_most_active(
    state: State<'_, SharedState>,
    top: Option<u32>,
) -> Result<String, String> {
    let top = top.unwrap_or(20).min(100);
    let broker = {
        let s = state.lock().await;
        s.broker.as_ref().ok_or("Not connected")?.clone()
    };
    let result = broker.get_most_active(top).await?;
    Ok(serde_json::to_string(&result).map_err(|e| format!("JSON error: {e}"))?)
}

#[tauri::command]
async fn get_top_movers(
    state: State<'_, SharedState>,
    market_type: Option<String>,
    top: Option<u32>,
) -> Result<String, String> {
    let market_type = market_type.unwrap_or_else(|| "stocks".to_string());
    let top = top.unwrap_or(20).min(100);
    let broker = {
        let s = state.lock().await;
        s.broker.as_ref().ok_or("Not connected")?.clone()
    };
    let result = broker.get_top_movers(&market_type, top).await?;
    Ok(serde_json::to_string(&result).map_err(|e| format!("JSON error: {e}"))?)
}

// ── Visual Backtester Commands ──────────────────────────────────

#[tauri::command]
async fn run_bar_by_bar_backtest(
    state: State<'_, SharedState>,
    symbol: String,
    timeframe: String,
    strategy: String,
    fast_period: Option<usize>,
    slow_period: Option<usize>,
    initial_equity: Option<f64>,
    limit: Option<u32>,
) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    if !is_valid_timeframe(&timeframe) { return Err("Invalid timeframe".into()); }

    let equity = initial_equity.unwrap_or(100_000.0);
    if equity <= 0.0 || !equity.is_finite() { return Err("Invalid initial equity".into()); }

    let bar_limit = limit.unwrap_or(5000).min(50_000);

    let bars = {
        let broker = {
            let s = state.lock().await;
            s.broker.as_ref().ok_or("Not connected")?.clone()
        };
        broker.get_bars(&symbol, &timeframe, bar_limit).await?
    };

    if bars.len() < 2 {
        return Err("Insufficient bar data for backtest".into());
    }

    let result: BarByBarResult = match strategy.as_str() {
        "sma_cross" | "SMA Cross" => {
            let fast = fast_period.unwrap_or(10);
            let slow = slow_period.unwrap_or(20);
            if fast >= slow { return Err("fast_period must be < slow_period".into()); }
            if slow > bars.len() { return Err("Not enough bars for slow period".into()); }
            let mut strat = SMACrossStrategy::new(fast, slow);
            backtest_engine::bar_by_bar_backtest(&bars, &mut strat, equity)
        }
        "nnfx" | "NNFX" | "NNFX (KAMA+Fisher)" => {
            let kama = fast_period.unwrap_or(10);
            let fisher = slow_period.unwrap_or(32);
            let mut strat = NNFXStrategy::new(kama, fisher);
            backtest_engine::bar_by_bar_backtest(&bars, &mut strat, equity)
        }
        _ => return Err(format!("Unknown strategy: {strategy}. Available: sma_cross, nnfx")),
    };

    Ok(serde_json::to_string(&result).map_err(|e| format!("JSON error: {e}"))?)
}

// ── Optimization Commands ───────────────────────────────────────

#[tauri::command]
async fn run_optimization(
    state: State<'_, SharedState>,
    symbol: String,
    timeframe: String,
    fast_min: usize,
    fast_max: usize,
    slow_min: usize,
    slow_max: usize,
    initial_equity: Option<f64>,
    top_n: Option<usize>,
    limit: Option<u32>,
) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    if !is_valid_timeframe(&timeframe) { return Err("Invalid timeframe".into()); }

    let equity = initial_equity.unwrap_or(100_000.0);
    if equity <= 0.0 || !equity.is_finite() { return Err("Invalid initial equity".into()); }

    // Sanity limits on ranges
    if fast_min < 2 || fast_max > 200 || slow_min < 3 || slow_max > 500 {
        return Err("Period ranges out of bounds (fast: 2-200, slow: 3-500)".into());
    }
    if fast_min > fast_max || slow_min > slow_max {
        return Err("Invalid range: min must be <= max".into());
    }
    // Cap total combinations to prevent abuse
    let total_combos = (fast_max - fast_min + 1) * (slow_max - slow_min + 1);
    if total_combos > 50_000 {
        return Err(format!("Too many combinations ({total_combos}). Max 50,000. Narrow ranges."));
    }

    let top = top_n.unwrap_or(20).min(500);
    let bar_limit = limit.unwrap_or(5000).min(50_000);

    let bars = {
        let broker = {
            let s = state.lock().await;
            s.broker.as_ref().ok_or("Not connected")?.clone()
        };
        broker.get_bars(&symbol, &timeframe, bar_limit).await?
    };

    if bars.len() < 2 {
        return Err("Insufficient bar data for optimization".into());
    }

    let result = backtest_engine::optimize_sma_cross(
        &bars,
        (fast_min, fast_max),
        (slow_min, slow_max),
        equity,
        top,
    );

    Ok(serde_json::to_string(&result).map_err(|e| format!("JSON error: {e}"))?)
}

// ── DOM / Level 2 Commands ──────────────────────────────────────

#[tauri::command]
async fn get_orderbook(
    state: State<'_, SharedState>,
    symbol: String,
) -> Result<String, String> {
    // Orderbook is crypto-only on Alpaca; symbol must contain "/"
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    if !symbol.contains('/') {
        return Err("Orderbook is only available for crypto pairs (e.g. BTC/USD)".into());
    }
    let broker = {
        let s = state.lock().await;
        s.broker.as_ref().ok_or("Not connected")?.clone()
    };
    let result = broker.get_orderbook(&symbol).await?;
    Ok(serde_json::to_string(&result).map_err(|e| format!("JSON error: {e}"))?)
}

// ── Custom Indicator Plugin System ──────────────────────────────

fn get_indicators_dir() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let dir = std::path::PathBuf::from(home)
        .join(".config")
        .join("typhoon-terminal")
        .join("indicators");
    std::fs::create_dir_all(&dir).ok();
    dir
}

/// Validate indicator name: alphanumeric, hyphens, underscores only. No path traversal.
fn is_valid_indicator_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        && !name.contains("..")
}

#[tauri::command]
async fn load_custom_indicator(source: String) -> Result<String, String> {
    // Validate JS source isn't absurdly large
    if source.len() > 1024 * 1024 {
        return Err("Indicator source too large (max 1MB)".to_string());
    }
    // Return the source for frontend sandboxed evaluation
    Ok(serde_json::to_string(&serde_json::json!({
        "source": source,
        "loaded": true,
    })).unwrap())
}

#[tauri::command]
async fn list_custom_indicators() -> Result<String, String> {
    let dir = get_indicators_dir();
    let mut indicators = Vec::new();

    if let Ok(mut entries) = tokio::fs::read_dir(&dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Some(name) = entry.file_name().to_str() {
                if name.ends_with(".js") {
                    let size = entry.metadata().await.map(|m| m.len()).unwrap_or(0);
                    indicators.push(serde_json::json!({
                        "name": name.trim_end_matches(".js"),
                        "filename": name,
                        "size": size,
                    }));
                    if indicators.len() >= 1000 { break; }
                }
            }
        }
    }

    Ok(serde_json::to_string(&indicators).map_err(|e| format!("JSON error: {e}"))?)
}

#[tauri::command]
async fn save_custom_indicator(name: String, source: String) -> Result<String, String> {
    if !is_valid_indicator_name(&name) {
        return Err("Invalid indicator name (alphanumeric, hyphens, underscores only)".to_string());
    }
    if source.len() > 1024 * 1024 {
        return Err("Indicator source too large (max 1MB)".to_string());
    }

    let dir = get_indicators_dir();
    let filename = format!("{}.js", name);
    // Verify path stays within indicators directory BEFORE writing
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return Err("Invalid indicator filename".to_string());
    }
    let path = dir.join(&filename);
    let canonical_dir = std::fs::canonicalize(&dir)
        .map_err(|e| format!("Indicators dir error: {e}"))?;
    // Verify the target path (without following symlinks on the final component)
    // Since filename is validated above, dir.join(filename) cannot escape dir
    let expected_parent = path.parent().and_then(|p| std::fs::canonicalize(p).ok());
    if expected_parent.as_ref() != Some(&canonical_dir) {
        return Err("Invalid path".to_string());
    }
    tokio::fs::write(&path, source.as_bytes()).await
        .map_err(|e| format!("Write failed: {e}"))?;

    Ok(serde_json::json!({
        "name": name,
        "filename": filename,
        "saved": true,
    }).to_string())
}

/// Headless CLI backtest mode — run strategies without GUI.
/// Usage: typhoon-terminal --backtest --symbol SMCI --timeframe 1Day --strategy nnfx
///        [--fast 10] [--slow 32] [--equity 100000] [--limit 5000]
fn run_headless_backtest(args: &[String]) {
    let get_arg = |name: &str| -> Option<String> {
        args.iter().position(|a| a == name).and_then(|i| args.get(i + 1).cloned())
    };

    let api_key = std::env::var("ALPACA_API_KEY").unwrap_or_default();
    let secret_key = std::env::var("ALPACA_SECRET_KEY").unwrap_or_default();
    let symbol = get_arg("--symbol").unwrap_or_else(|| "SPY".to_string());
    let timeframe = get_arg("--timeframe").unwrap_or_else(|| "1Day".to_string());
    let strategy = get_arg("--strategy").unwrap_or_else(|| "nnfx".to_string());
    let fast = get_arg("--fast").and_then(|s| s.parse().ok()).unwrap_or(10);
    let slow = get_arg("--slow").and_then(|s| s.parse().ok()).unwrap_or(32);
    let equity = get_arg("--equity").and_then(|s| s.parse().ok()).unwrap_or(100_000.0);
    let limit = get_arg("--limit").and_then(|s| s.parse().ok()).unwrap_or(5000u32);
    let paper = !args.iter().any(|a| a == "--live");

    println!("═══════════════════════════════════════════════════════");
    println!("  TyphooN Terminal — Headless Backtest");
    println!("═══════════════════════════════════════════════════════");
    println!("  Symbol:    {symbol}");
    println!("  Timeframe: {timeframe}");
    println!("  Strategy:  {strategy}");
    println!("  Params:    fast={fast}, slow={slow}");
    println!("  Equity:    ${equity:.2}");
    println!("  Bars:      {limit}");
    println!("═══════════════════════════════════════════════════════");

    if api_key.is_empty() || secret_key.is_empty() {
        eprintln!("ERROR: Set ALPACA_API_KEY and ALPACA_SECRET_KEY environment variables");
        std::process::exit(1);
    }

    let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
    rt.block_on(async {
        let broker = broker::alpaca::AlpacaBroker::new(api_key, secret_key, paper);

        // Verify connection
        match broker.get_account().await {
            Ok(acct) => println!("  Account:   ${:.2} equity, ${:.2} cash", acct.equity, acct.cash),
            Err(e) => { eprintln!("ERROR: {e}"); std::process::exit(1); }
        }

        println!("  Fetching bars...");
        let bars = match broker.get_bars(&symbol, &timeframe, limit).await {
            Ok(b) => b,
            Err(e) => { eprintln!("ERROR: {e}"); std::process::exit(1); }
        };
        println!("  Loaded {} bars", bars.len());

        if bars.len() < 50 {
            eprintln!("ERROR: Insufficient data ({} bars, need 50+)", bars.len());
            std::process::exit(1);
        }

        println!("  Running backtest...\n");

        let result = match strategy.as_str() {
            "sma_cross" | "sma" => {
                let mut strat = SMACrossStrategy::new(fast, slow);
                backtest_engine::run_backtest(&bars, &mut strat, equity)
            }
            "nnfx" | "NNFX" => {
                let mut strat = NNFXStrategy::new(fast, slow);
                backtest_engine::run_backtest(&bars, &mut strat, equity)
            }
            _ => {
                eprintln!("ERROR: Unknown strategy '{strategy}'. Available: sma_cross, nnfx");
                std::process::exit(1);
            }
        };

        let r = &result.report;
        println!("═══════════════════════════════════════════════════════");
        println!("  BACKTEST RESULTS: {symbol} @ {timeframe}");
        println!("═══════════════════════════════════════════════════════");
        println!("  Total Trades:       {}", r.total_trades);
        println!("  Win Rate:           {:.1}%", r.win_rate);
        println!("  Profit Factor:      {:.2}", r.profit_factor);
        println!("  Sharpe Ratio:       {:.2}", r.sharpe_ratio);
        println!("  Total P&L:          ${:.2}", r.total_pnl);
        println!("  Gross Profit:       ${:.2}", r.gross_profit);
        println!("  Gross Loss:         ${:.2}", r.gross_loss);
        println!("  Avg Win:            ${:.2}", r.avg_win);
        println!("  Avg Loss:           ${:.2}", r.avg_loss);
        println!("  Avg Trade:          ${:.2}", r.avg_trade);
        println!("  Max Drawdown:       ${:.2} ({:.1}%)", r.max_drawdown, r.max_drawdown_pct);
        println!("  Max Con. Wins:      {}", r.max_consecutive_wins);
        println!("  Max Con. Losses:    {}", r.max_consecutive_losses);
        println!("═══════════════════════════════════════════════════════");

        // Print trade log
        if !result.trades.is_empty() {
            println!("\n  TRADE LOG:");
            println!("  {:<6} {:<12} {:<12} {:<10} {:<12}", "Side", "Entry", "Exit", "P&L", "P&L%");
            for t in result.trades.iter().take(50) {
                println!("  {:<6} {:<12.4} {:<12.4} ${:<9.2} {:.2}%",
                    t.side, t.entry_price, t.exit_price, t.pnl, t.pnl_pct);
            }
            if result.trades.len() > 50 {
                println!("  ... and {} more trades", result.trades.len() - 50);
            }
        }
    });
}

// ── Earnings Surprise (Finnhub) ─────────────────────────────────

#[tauri::command]
async fn fetch_earnings_surprise(symbol: String, finnhub_key: String) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    if finnhub_key.is_empty() || finnhub_key.len() > 100 { return Err("Invalid Finnhub API key".into()); }
    let resp = shared_client()
        .get("https://finnhub.io/api/v1/stock/earnings")
        .query(&[("symbol", symbol.as_str()), ("token", finnhub_key.as_str())])
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("Earnings surprise request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Earnings surprise: HTTP {}", resp.status()));
    }
    resp.text().await.map_err(|e| format!("Earnings surprise read failed: {e}"))
}

// ── Earnings Calendar (Finnhub) ─────────────────────────────────

#[tauri::command]
async fn fetch_earnings_calendar(finnhub_key: String) -> Result<String, String> {
    if finnhub_key.is_empty() || finnhub_key.len() > 100 { return Err("Invalid Finnhub API key".into()); }
    let today = chrono::Utc::now().date_naive();
    let from = (today - chrono::Duration::days(7)).format("%Y-%m-%d").to_string();
    let to = (today + chrono::Duration::days(30)).format("%Y-%m-%d").to_string();
    let resp = shared_client()
        .get("https://finnhub.io/api/v1/calendar/earnings")
        .query(&[("from", from.as_str()), ("to", to.as_str()), ("token", finnhub_key.as_str())])
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| format!("Earnings calendar request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Earnings calendar: HTTP {}", resp.status()));
    }
    resp.text().await.map_err(|e| format!("Earnings calendar read failed: {e}"))
}

// ── Dark Pool Volume (FINRA) ────────────────────────────────────

#[tauri::command]
async fn fetch_dark_pool_volume(symbol: String) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    let sym_upper = symbol.to_uppercase();

    // Try today, then up to 5 days back (skipping weekends)
    let today = chrono::Utc::now().date_naive();
    let mut candidates = Vec::new();
    let mut d = today;
    while candidates.len() < 3 {
        let weekday = d.weekday();
        if weekday != chrono::Weekday::Sat && weekday != chrono::Weekday::Sun {
            candidates.push(d);
        }
        d -= chrono::Duration::days(1);
    }

    for date in &candidates {
        let date_str = date.format("%Y%m%d").to_string();
        let url = format!("https://cdn.finra.org/equity/regsho/daily/CNMSshvol{date_str}.txt");
        let resp = shared_client()
            .get(&url)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await;
        let resp = match resp {
            Ok(r) if r.status().is_success() => r,
            _ => continue,
        };
        let body = match resp.text().await {
            Ok(b) => b,
            Err(_) => continue,
        };

        // Pipe-delimited: Date|Symbol|ShortVolume|ShortExemptVolume|TotalVolume|Market
        for line in body.lines() {
            let fields: Vec<&str> = line.split('|').collect();
            if fields.len() < 5 { continue; }
            if fields[1].eq_ignore_ascii_case(&sym_upper) {
                let short_vol: f64 = fields[2].parse().unwrap_or(0.0);
                let short_exempt: f64 = fields[3].parse().unwrap_or(0.0);
                let total_vol: f64 = fields[4].parse().unwrap_or(0.0);
                let dark_pool_pct = if total_vol > 0.0 { (short_vol + short_exempt) / total_vol * 100.0 } else { 0.0 };
                let result = serde_json::json!({
                    "date": date.format("%Y-%m-%d").to_string(),
                    "symbol": sym_upper,
                    "shortVolume": short_vol as u64,
                    "shortExemptVolume": short_exempt as u64,
                    "totalVolume": total_vol as u64,
                    "darkPoolPct": (dark_pool_pct * 100.0).round() / 100.0,
                });
                return serde_json::to_string(&result).map_err(|e| format!("JSON error: {e}"));
            }
        }
    }

    Err(format!("No FINRA dark pool data found for {sym_upper} in recent trading days"))
}

// ── Whale Alert (Crypto) ────────────────────────────────────────

#[tauri::command]
async fn fetch_whale_alerts() -> Result<String, String> {
    // Try Whale Alert API with demo key first
    let start = chrono::Utc::now().timestamp() - 3600; // 1 hour ago
    let resp = shared_client()
        .get("https://api.whale-alert.io/v1/transactions")
        .query(&[
            ("api_key", "demo"),
            ("min_value", "1000000"),
            ("start", &start.to_string()),
        ])
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await;

    if let Ok(r) = resp {
        if r.status().is_success() {
            if let Ok(text) = r.text().await {
                return Ok(text);
            }
        }
    }

    // Fallback: Blockchair large BTC transactions (free, no key)
    let resp = shared_client()
        .get("https://api.blockchair.com/bitcoin/transactions")
        .query(&[("q", "output_total(100000000..)"), ("limit", "10")])
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("Whale alert request failed: {e}"))?;
    if !resp.status().is_success() {
        // Return empty array on failure rather than error
        return Ok("[]".to_string());
    }
    resp.text().await.map_err(|e| format!("Whale alert read failed: {e}"))
}

// ── IPO Calendar (Finnhub) ──────────────────────────────────────

#[tauri::command]
async fn fetch_ipo_calendar(finnhub_key: String) -> Result<String, String> {
    if finnhub_key.is_empty() || finnhub_key.len() > 100 { return Err("Invalid Finnhub API key".into()); }
    let today = chrono::Utc::now().date_naive();
    let from = today.format("%Y-%m-%d").to_string();
    let to = (today + chrono::Duration::days(90)).format("%Y-%m-%d").to_string();
    let resp = shared_client()
        .get("https://finnhub.io/api/v1/calendar/ipo")
        .query(&[("from", from.as_str()), ("to", to.as_str()), ("token", finnhub_key.as_str())])
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| format!("IPO calendar request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("IPO calendar: HTTP {}", resp.status()));
    }
    resp.text().await.map_err(|e| format!("IPO calendar read failed: {e}"))
}

// ── Economic Calendar (Finnhub) ─────────────────────────────────

#[tauri::command]
async fn fetch_economic_calendar(finnhub_key: String) -> Result<String, String> {
    if finnhub_key.is_empty() || finnhub_key.len() > 100 { return Err("Invalid Finnhub API key".into()); }
    let today = chrono::Utc::now().date_naive();
    let from = (today - chrono::Duration::days(7)).format("%Y-%m-%d").to_string();
    let to = (today + chrono::Duration::days(14)).format("%Y-%m-%d").to_string();
    let resp = shared_client()
        .get("https://finnhub.io/api/v1/calendar/economic")
        .query(&[("from", from.as_str()), ("to", to.as_str()), ("token", finnhub_key.as_str())])
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| format!("Economic calendar request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Economic calendar: HTTP {}", resp.status()));
    }
    resp.text().await.map_err(|e| format!("Economic calendar read failed: {e}"))
}

// ── Sector Peers (Finnhub) ──────────────────────────────────────

#[tauri::command]
async fn fetch_sector_peers(symbol: String) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    let resp = shared_client()
        .get("https://finnhub.io/api/v1/stock/peers")
        .query(&[("symbol", symbol.as_str()), ("token", "demo")])
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("Sector peers request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Sector peers: HTTP {}", resp.status()));
    }
    resp.text().await.map_err(|e| format!("Sector peers read failed: {e}"))
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "typhoon_terminal=info".into()),
        )
        .init();

    // ── Headless CLI Backtest Mode ──
    // Usage: typhoon-terminal --backtest --symbol SMCI --timeframe 1Day --strategy nnfx
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--backtest") {
        run_headless_backtest(&args);
        return;
    }

    let state: SharedState = Arc::new(Mutex::new(AppState {
        broker: None,
        risk_config: RiskConfig::default(),
        martingale: MartingaleState::new(MartingaleConfig::default()),
        sl_levels: std::collections::HashMap::new(),
        tp_levels: std::collections::HashMap::new(),
        symbols: Vec::new(),
        stream_rx: None,
        equity_tp: None,
        equity_sl: None,
        db_cache: {
            let cache_dir = get_cache_dir();
            let db_path = cache_dir.join("typhoon_cache.db");
            match SqliteCache::open(&db_path) {
                Ok(cache) => {
                    tracing::info!("SQLite cache opened: {:?}", db_path);
                    Some(cache)
                }
                Err(e) => {
                    tracing::warn!("SQLite cache failed: {e}. Falling back to zstd files.");
                    None
                }
            }
        },
        bar_builder: core::bar_builder::BarBuilder::new(),
        bar_inflight: std::collections::HashSet::new(),
    }));

    tauri::Builder::default()
        // tauri-plugin-shell removed — not used, reduces attack surface
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            // Keychain
            keychain_save,
            keychain_load,
            keychain_delete,
            // Broker
            connect,
            get_account,
            get_positions,
            get_bars,
            get_bars_incremental,
            place_order,
            place_limit_order,
            place_stop_order,
            place_stop_limit_order,
            place_trailing_stop,
            place_bracket_order,
            get_open_orders,
            get_order_history,
            modify_order,
            cancel_order,
            close_position,
            close_all,
            get_asset,
            get_multi_tf_bars,
            load_symbols,
            search_symbols,
            // Risk
            calculate_lots,
            calculate_position_var,
            calculate_portfolio_var_lots,
            get_risk_config,
            set_order_mode,
            set_risk_config,
            // SL/TP
            set_sl_level,
            set_tp_level,
            get_sl_tp_pl,
            // Martingale
            get_martingale_state,
            set_martingale_mode,
            toggle_martingale,
            set_martingale_config,
            calc_open_mg_size,
            open_martingale_hedge,
            // Margin
            get_margin_info,
            // Account Protection
            set_equity_protection,
            check_equity_protection,
            // FRED + AI
            fetch_fred_series,
            ai_chat,
            // Notifications
            send_discord_notification,
            // News, Events & Fundamentals
            get_news,
            get_finnhub_news,
            get_av_earnings,
            get_fmp_ratings,
            get_corporate_actions,
            get_portfolio_history,
            get_market_clock,
            get_finnhub_recommendations,
            get_finnhub_price_target,
            get_finnhub_insider_sentiment,
            fetch_fear_greed,
            fetch_treasury_yields,
            fetch_congress_trades,
            fetch_forex_rates,
            fetch_crypto_trending,
            fetch_crypto_market,
            fetch_reddit_mentions,
            get_sec_filings,
            get_company_fundamentals,
            // Bid/Ask, Activities, Insider
            get_latest_quote,
            get_account_activities,
            get_insider_trades,
            // Articles & cache management
            fetch_article,
            clear_symbol_cache,
            // SQLite cache
            db_cache_put,
            db_cache_get,
            db_cache_stats,
            db_cache_detailed_stats,
            db_cache_delete_key,
            db_cache_delete_symbol,
            db_cache_evict,
            // Cold cache (zstd files — legacy)
            save_cold_cache,
            load_cold_cache,
            list_cold_cache,
            // Backtest
            run_backtest,
            run_bar_by_bar_backtest,
            run_optimization,
            run_walk_forward,
            // CSV Export
            export_trade_history,
            // Options
            get_options,
            // Screener
            run_screener,
            // Financial Analysis & Institutional Holders
            get_financial_analysis,
            get_institutional_holders,
            // Most Active / Top Movers
            get_most_active,
            get_top_movers,
            // DOM / Level 2
            get_orderbook,
            // Custom Indicators
            load_custom_indicator,
            list_custom_indicators,
            save_custom_indicator,
            // WebSocket Streaming
            start_streaming,
            poll_stream,
            poll_bars,
            stop_streaming,
            // Matrix Community Chat
            matrix_login,
            matrix_send,
            matrix_join,
            matrix_poll,
            // Push Notifications
            send_pushover_notification,
            send_ntfy_notification,
            // Short Interest, World Indices, Watchlists
            fetch_short_interest,
            fetch_world_indices,
            get_alpaca_watchlists,
            sync_watchlist_to_alpaca,
            // Earnings, Calendars, Dark Pool, Whale, Peers
            fetch_earnings_surprise,
            fetch_earnings_calendar,
            fetch_dark_pool_volume,
            fetch_whale_alerts,
            fetch_ipo_calendar,
            fetch_economic_calendar,
            fetch_sector_peers,
        ])
        .run(tauri::generate_context!())
        .expect("error while running TyphooN Terminal");
}
