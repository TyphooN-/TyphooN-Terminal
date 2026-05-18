//! Kraken public Spot/xStocks market data via the public REST OHLC endpoint.
//!
//! Kraken is used instead of Binance (geo-blocked in US/Canada).
//! No API key needed. No geo-restrictions.
//! History: BTC from 2013, ETH from 2016, most alts from 2017+.

use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use tokio::sync::Mutex;

const SPOT_PUBLIC_INTERVAL: Duration = Duration::from_millis(1_100);
const SPOT_PUBLIC_BASE_COOLDOWN: Duration = Duration::from_secs(5);
const SPOT_PUBLIC_MAX_COOLDOWN: Duration = Duration::from_secs(60);
const SPOT_PUBLIC_MAX_ATTEMPTS: usize = 3;

static SPOT_PUBLIC_LIMITER: LazyLock<KrakenSpotPublicLimiter> =
    LazyLock::new(KrakenSpotPublicLimiter::new);
static SPOT_PAIR_CATALOG: LazyLock<Mutex<Option<KrakenPairCatalog>>> =
    LazyLock::new(|| Mutex::new(None));

#[derive(Debug, Clone, Default)]
struct KrakenPairCatalog {
    by_symbol: HashMap<String, String>,
}

#[derive(Debug, Default)]
struct KrakenSpotPublicState {
    global_next_allowed: Option<Instant>,
    pair_next_allowed: HashMap<String, Instant>,
    cooldown_until: Option<Instant>,
    cooldown: Duration,
}

#[derive(Debug)]
struct KrakenSpotPublicLimiter {
    state: Mutex<KrakenSpotPublicState>,
}

impl KrakenSpotPublicLimiter {
    fn new() -> Self {
        Self {
            state: Mutex::new(KrakenSpotPublicState::default()),
        }
    }

    async fn wait_ohlc(&self, pair: &str) {
        let wait = {
            let now = Instant::now();
            let mut state = self.state.lock().await;
            if state
                .cooldown_until
                .is_some_and(|cooldown_until| cooldown_until <= now)
            {
                state.cooldown_until = None;
                state.cooldown = Duration::ZERO;
            }
            state
                .pair_next_allowed
                .retain(|_, next_allowed| *next_allowed > now);

            let mut ready_at = now;
            if let Some(cooldown_until) = state.cooldown_until {
                ready_at = ready_at.max(cooldown_until);
            }
            if let Some(next_allowed) = state.global_next_allowed {
                ready_at = ready_at.max(next_allowed);
            }
            if let Some(next_allowed) = state.pair_next_allowed.get(pair) {
                ready_at = ready_at.max(*next_allowed);
            }

            let next_allowed = ready_at + SPOT_PUBLIC_INTERVAL;
            state.global_next_allowed = Some(next_allowed);
            state
                .pair_next_allowed
                .insert(pair.to_string(), next_allowed);
            ready_at.saturating_duration_since(now)
        };

        if !wait.is_zero() {
            tokio::time::sleep(wait).await;
        }
    }

    async fn record_success(&self) {
        let now = Instant::now();
        let mut state = self.state.lock().await;
        if state
            .cooldown_until
            .is_some_and(|cooldown_until| cooldown_until <= now)
        {
            state.cooldown_until = None;
            state.cooldown = Duration::ZERO;
        }
    }

    async fn record_rate_limited(&self, message: &str) -> Duration {
        let explicit_wait = kraken_throttled_wait(message).map(|wait| {
            if wait.is_zero() {
                SPOT_PUBLIC_BASE_COOLDOWN
            } else {
                wait.min(SPOT_PUBLIC_MAX_COOLDOWN)
            }
        });
        let now = Instant::now();
        let mut state = self.state.lock().await;
        let wait = if let Some(wait) = explicit_wait {
            wait
        } else if state.cooldown_until.is_some_and(|until| until > now) {
            state
                .cooldown
                .max(SPOT_PUBLIC_BASE_COOLDOWN)
                .saturating_mul(2)
                .min(SPOT_PUBLIC_MAX_COOLDOWN)
        } else {
            SPOT_PUBLIC_BASE_COOLDOWN
        };
        state.cooldown = wait;
        let cooldown_until = now + wait;
        state.cooldown_until = Some(
            state
                .cooldown_until
                .map(|existing| existing.max(cooldown_until))
                .unwrap_or(cooldown_until),
        );
        state.global_next_allowed = Some(
            state
                .global_next_allowed
                .map(|existing| existing.max(cooldown_until))
                .unwrap_or(cooldown_until),
        );
        wait
    }
}

/// Map TyphooN timeframes to Kraken interval (minutes).
fn to_kraken_interval(tf: &str) -> Option<u32> {
    match tf {
        "1Min" => Some(1),
        "5Min" => Some(5),
        "15Min" => Some(15),
        "30Min" => Some(30),
        "1Hour" => Some(60),
        "4Hour" => Some(240),
        "1Day" => Some(1440),
        "1Week" => Some(10080),
        // Kraken doesn't support 1Month — we'll aggregate from daily
        "1Month" => None,
        _ => None,
    }
}

/// Map common TyphooN crypto symbols to Kraken trading pairs.
pub fn to_kraken_pair(sym: &str) -> Option<&'static str> {
    let clean = sym.replace("/", "").to_uppercase();
    match clean.as_str() {
        "BTCUSD" => Some("XBTUSD"),
        "ETHUSD" => Some("ETHUSD"),
        "SOLUSD" => Some("SOLUSD"),
        "DOGEUSD" => Some("XDGUSD"),
        "ADAUSD" => Some("ADAUSD"),
        "XRPUSD" => Some("XRPUSD"),
        "DOTUSD" => Some("DOTUSD"),
        "LINKUSD" => Some("LINKUSD"),
        "AVAXUSD" => Some("AVAXUSD"),
        "MATICUSD" => Some("POLUSD"),
        "POLUSD" => Some("POLUSD"),
        "UNIUSD" => Some("UNIUSD"),
        "LTCUSD" => Some("LTCUSD"),
        "BCHUSD" => Some("BCHUSD"),
        "XLMUSD" => Some("XLMUSD"),
        "ATOMUSD" => Some("ATOMUSD"),
        "NEARUSD" => Some("NEARUSD"),
        "FILUSD" => Some("FILUSD"),
        "AAVEUSD" => Some("AAVEUSD"),
        "ALGOUSD" => Some("ALGOUSD"),
        "MANAUSD" => Some("MANAUSD"),
        "SANDUSD" => Some("SANDUSD"),
        "GRTUSD" => Some("GRTUSD"),
        "ICPUSD" => Some("ICPUSD"),
        "TRXUSD" => Some("TRXUSD"),
        "ETCUSD" => Some("ETCUSD"),
        "EOSUSD" => Some("EOSUSD"),
        "XTZUSD" => Some("XTZUSD"),
        "KAVAUSD" => Some("KAVAUSD"),
        "COMPUSD" => Some("COMPUSD"),
        "MKRUSD" => Some("MKRUSD"),
        "SNXUSD" => Some("SNXUSD"),
        "CRVUSD" => Some("CRVUSD"),
        "SUSHIUSD" => Some("SUSHIUSD"),
        "YFIUSD" => Some("YFIUSD"),
        "BATUSD" => Some("BATUSD"),
        "XMRUSD" => Some("XXMRZUSD"),
        "ZECUSD" => Some("XZECZUSD"),
        "DASHUSD" => Some("DASHUSD"),
        "ENJUSD" => Some("ENJUSD"),
        "FTMUSD" => Some("FTMUSD"),
        "BNBUSD" => Some("BNBUSD"),
        "SHIBUSD" => Some("SHIBUSD"),
        "APEUSD" => Some("APEUSD"),
        "ARBUSD" => Some("ARBUSD"),
        "OPUSD" => Some("OPUSD"),
        "HBARUSD" => Some("HBARUSD"),
        "VETUSD" => Some("VETUSD"),
        "THETAUSD" => Some("THETAUSD"),
        "AXSUSD" => Some("AXSUSD"),
        _ => None,
    }
}

/// Normalize Kraken pair representations into TyphooN's canonical no-slash symbol
/// form. Handles display names (`BTC/USD`), Kraken altnames (`XBTUSD`), and
/// raw wrapped pairs (`XXBTZUSD`).
pub fn normalize_pair_symbol(sym: &str) -> String {
    let raw = sym.trim().replace('/', "").to_ascii_uppercase();
    if raw.is_empty() {
        return String::new();
    }
    let unwrapped = {
        let bytes = raw.as_bytes();
        if raw.len() == 8
            && matches!(bytes[0] as char, 'X' | 'Z')
            && matches!(bytes[4] as char, 'X' | 'Z')
        {
            format!("{}{}", &raw[1..4], &raw[5..8])
        } else {
            raw
        }
    };
    if let Some(rest) = unwrapped.strip_prefix("XBT") {
        format!("BTC{rest}")
    } else if let Some(rest) = unwrapped.strip_prefix("XDG") {
        format!("DOGE{rest}")
    } else {
        unwrapped
    }
}

/// Best-effort Kraken pair encoder for dynamic pair universes.
///
/// The curated `to_kraken_pair()` table is still used first for aliases that
/// need exact Kraken raw names. When no curated mapping exists, we fall back to
/// Kraken altname-style pairs such as `ETHBTC` or `ETHEUR`, which Kraken's
/// public endpoints accept for the broad pair universe.
pub fn to_kraken_pair_lossy(sym: &str) -> Option<String> {
    if let Some(mapped) = to_kraken_pair(sym) {
        return Some(mapped.to_string());
    }
    let normalized = normalize_pair_symbol(sym);
    if normalized.len() < 6 {
        return None;
    }
    if let Some(rest) = normalized.strip_prefix("BTC") {
        return Some(format!("XBT{rest}"));
    }
    if let Some(rest) = normalized.strip_prefix("DOGE") {
        return Some(format!("XDG{rest}"));
    }
    Some(normalized)
}

fn insert_kraken_pair_alias(
    map: &mut HashMap<String, String>,
    source: impl AsRef<str>,
    pair_name: &str,
) {
    let symbol = normalize_pair_symbol(source.as_ref())
        .replace('/', "")
        .to_ascii_uppercase();
    if !symbol.is_empty() {
        map.entry(symbol).or_insert_with(|| pair_name.to_string());
    }
}

fn parse_kraken_pair_catalog(body: &serde_json::Value) -> Result<KrakenPairCatalog, String> {
    if let Some(err_msg) = kraken_response_error(body) {
        return Err(format!("Kraken AssetPairs error: {err_msg}"));
    }

    let result = body
        .get("result")
        .and_then(|r| r.as_object())
        .ok_or("Expected object in Kraken AssetPairs response")?;
    let mut by_symbol = HashMap::with_capacity(result.len() * 3);

    for (pair_name, info) in result {
        let status = info
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("online");
        if status == "delisted" {
            continue;
        }

        insert_kraken_pair_alias(&mut by_symbol, pair_name, pair_name);
        if let Some(altname) = info.get("altname").and_then(|v| v.as_str()) {
            insert_kraken_pair_alias(&mut by_symbol, altname, pair_name);
        }
        if let Some(wsname) = info.get("wsname").and_then(|v| v.as_str()) {
            insert_kraken_pair_alias(&mut by_symbol, wsname, pair_name);
        }
    }

    Ok(KrakenPairCatalog { by_symbol })
}

async fn load_kraken_pair_catalog(client: &reqwest::Client) -> Result<KrakenPairCatalog, String> {
    let mut cached = SPOT_PAIR_CATALOG.lock().await;
    if let Some(catalog) = cached.clone() {
        return Ok(catalog);
    }

    SPOT_PUBLIC_LIMITER.wait_ohlc("AssetPairs").await;
    let body: serde_json::Value = client
        .get("https://api.kraken.com/0/public/AssetPairs")
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("Kraken AssetPairs request failed: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Kraken AssetPairs JSON parse failed: {e}"))?;
    let catalog = parse_kraken_pair_catalog(&body)?;
    *cached = Some(catalog.clone());
    Ok(catalog)
}

async fn resolve_kraken_pair(client: &reqwest::Client, symbol: &str) -> Option<String> {
    let normalized = normalize_pair_symbol(symbol)
        .replace('/', "")
        .to_ascii_uppercase();
    if normalized.is_empty() {
        return None;
    }

    match load_kraken_pair_catalog(client).await {
        Ok(catalog) => catalog.by_symbol.get(&normalized).cloned(),
        Err(_) => to_kraken_pair_lossy(&normalized),
    }
}

fn parse_kraken_number(value: &serde_json::Value) -> Option<f64> {
    match value {
        serde_json::Value::String(s) => s.parse::<f64>().ok(),
        serde_json::Value::Number(n) => n.as_f64(),
        _ => None,
    }
    .filter(|v| v.is_finite())
}

fn kraken_response_error(body: &serde_json::Value) -> Option<String> {
    let errors = body.get("error")?.as_array()?;
    let err_msg = errors
        .iter()
        .filter_map(|e| e.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    (!err_msg.is_empty()).then_some(err_msg)
}

pub(crate) fn is_kraken_rate_limit_error(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    lower.contains("too many requests")
        || lower.contains("rate limit")
        || lower.contains("ratelimit")
        || lower.contains("throttled")
        || lower.contains("429")
}

pub(crate) fn kraken_throttled_wait(message: &str) -> Option<Duration> {
    let lower = message.to_ascii_lowercase();
    let idx = lower.find("throttled:")?;
    let tail = &message[idx + "throttled:".len()..];
    let digits = tail
        .chars()
        .skip_while(|ch| !ch.is_ascii_digit())
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    let retry_at = digits.parse::<u64>().ok()?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    Some(Duration::from_secs(retry_at.saturating_sub(now)))
}

fn parse_kraken_ohlc_response(
    body: &serde_json::Value,
    start_ms: i64,
    end_ms: i64,
) -> Result<Vec<serde_json::Value>, String> {
    if let Some(err_msg) = kraken_response_error(body) {
        return Err(format!("Kraken error: {}", err_msg));
    }

    let result = &body["result"];
    let mut bars = Vec::new();
    for (key, val) in result.as_object().unwrap_or(&serde_json::Map::new()) {
        if key == "last" {
            continue;
        }
        if let Some(arr) = val.as_array() {
            bars.reserve(arr.len());
            for kline in arr {
                let Some(k) = kline.as_array() else {
                    continue;
                };
                // Kraken OHLCVT: [timestamp, open, high, low, close, vwap, volume, count]
                if k.len() < 7 {
                    continue;
                }
                let ts = k[0].as_i64().unwrap_or(0);
                if ts == 0 {
                    continue;
                }
                let ts_ms = ts.saturating_mul(1000);
                if ts_ms < start_ms || ts_ms > end_ms {
                    continue;
                }
                let Some(dt) = chrono::DateTime::from_timestamp(ts, 0) else {
                    continue;
                };
                let open = parse_kraken_number(&k[1]).unwrap_or(0.0);
                let high = parse_kraken_number(&k[2]).unwrap_or(0.0);
                let low = parse_kraken_number(&k[3]).unwrap_or(0.0);
                let close = parse_kraken_number(&k[4]).unwrap_or(0.0);
                let volume = parse_kraken_number(&k[6]).unwrap_or(0.0);

                if open > 0.0 && high > 0.0 && low > 0.0 && close > 0.0 && high >= low {
                    bars.push(serde_json::json!({
                        "timestamp": dt.to_rfc3339(),
                        "open": open,
                        "high": high,
                        "low": low,
                        "close": close,
                        "volume": volume.max(0.0),
                    }));
                }
            }
        }
    }
    Ok(bars)
}

/// Fetch OHLCV klines from Kraken public API.
/// Kraken's public OHLC endpoint returns a recent bounded window, so TyphooN
/// performs one request per `(symbol, timeframe)` and lets the async scheduler
/// provide concurrency.
pub async fn fetch_binance_klines(
    client: &reqwest::Client,
    symbol: &str,
    timeframe: &str,
    start_ms: i64,
    end_ms: i64,
) -> Result<Vec<serde_json::Value>, String> {
    // Handle 1Month by fetching daily and aggregating
    if timeframe == "1Month" {
        let daily = Box::pin(fetch_binance_klines(
            client, symbol, "1Day", start_ms, end_ms,
        ))
        .await?;
        return Ok(aggregate_to_monthly(&daily));
    }

    let interval = to_kraken_interval(timeframe)
        .ok_or_else(|| format!("Unsupported timeframe for Kraken: {}", timeframe))?;
    let kraken_pair = resolve_kraken_pair(client, symbol)
        .await
        .ok_or_else(|| format!("Unsupported symbol for Kraken: {}", symbol))?;

    let since = (start_ms / 1000).max(0);
    let url = format!(
        "https://api.kraken.com/0/public/OHLC?pair={}&interval={}&since={}",
        kraken_pair, interval, since
    );

    let mut last_rate_limit_error = None;
    for attempt in 0..SPOT_PUBLIC_MAX_ATTEMPTS {
        SPOT_PUBLIC_LIMITER.wait_ohlc(&kraken_pair).await;

        let resp = client
            .get(&url)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| format!("Kraken request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            let err = format!("Kraken API error {}: {}", status, body);
            if is_kraken_rate_limit_error(&err) {
                let wait = SPOT_PUBLIC_LIMITER.record_rate_limited(&err).await;
                last_rate_limit_error = Some(err.clone());
                if attempt + 1 < SPOT_PUBLIC_MAX_ATTEMPTS {
                    tracing::warn!(
                        "Kraken public OHLC rate-limited for {} {}; cooling down {}s before retry {}/{}",
                        kraken_pair,
                        timeframe,
                        wait.as_secs(),
                        attempt + 2,
                        SPOT_PUBLIC_MAX_ATTEMPTS
                    );
                    continue;
                }
            }
            return Err(err);
        }

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Kraken JSON parse failed: {e}"))?;

        if let Some(err_msg) = kraken_response_error(&body) {
            let err = format!("Kraken error: {}", err_msg);
            if is_kraken_rate_limit_error(&err) {
                let wait = SPOT_PUBLIC_LIMITER.record_rate_limited(&err).await;
                last_rate_limit_error = Some(err.clone());
                if attempt + 1 < SPOT_PUBLIC_MAX_ATTEMPTS {
                    tracing::warn!(
                        "Kraken public OHLC rate-limited for {} {}; cooling down {}s before retry {}/{}",
                        kraken_pair,
                        timeframe,
                        wait.as_secs(),
                        attempt + 2,
                        SPOT_PUBLIC_MAX_ATTEMPTS
                    );
                    continue;
                }
            }
            return Err(err);
        }

        let mut all_bars = parse_kraken_ohlc_response(&body, start_ms.max(0), end_ms)?;

        // Sort by timestamp and deduplicate
        all_bars.sort_by(|a, b| {
            let ta = a["timestamp"].as_str().unwrap_or("");
            let tb = b["timestamp"].as_str().unwrap_or("");
            ta.cmp(tb)
        });
        all_bars.dedup_by(|a, b| a["timestamp"] == b["timestamp"]);

        SPOT_PUBLIC_LIMITER.record_success().await;
        return Ok(all_bars);
    }

    Err(last_rate_limit_error.unwrap_or_else(|| "Kraken OHLC request failed".to_string()))
}

/// Aggregate daily bars into monthly OHLCV.
fn aggregate_to_monthly(daily: &[serde_json::Value]) -> Vec<serde_json::Value> {
    let mut monthly: std::collections::BTreeMap<String, (f64, f64, f64, f64, f64, String)> =
        std::collections::BTreeMap::new();
    // key = "YYYY-MM", value = (open, high, low, close, volume, first_timestamp)

    for bar in daily {
        let ts = bar["timestamp"].as_str().unwrap_or("");
        if ts.len() < 7 {
            continue;
        }
        let o = bar["open"].as_f64().unwrap_or(0.0);
        let h = bar["high"].as_f64().unwrap_or(0.0);
        let l = bar["low"].as_f64().unwrap_or(0.0);
        let c = bar["close"].as_f64().unwrap_or(0.0);
        let v = bar["volume"].as_f64().unwrap_or(0.0);
        // Skip bars with invalid prices
        if o <= 0.0 || h <= 0.0 || l <= 0.0 || c <= 0.0 || h < l {
            continue;
        }
        let month_key = ts[..7].to_string(); // "2024-06"

        let entry = monthly
            .entry(month_key)
            .or_insert((o, h, l, c, 0.0, ts.to_string()));
        if h > entry.1 {
            entry.1 = h;
        }
        if l < entry.2 {
            entry.2 = l;
        }
        entry.3 = c; // close = last day's close
        entry.4 += v;
    }

    monthly
        .into_iter()
        .map(|(_, (o, h, l, c, v, ts))| {
            serde_json::json!({
                "timestamp": ts,
                "open": o, "high": h, "low": l, "close": c, "volume": v,
            })
        })
        .collect()
}

/// Check if a symbol is supported by Kraken.
pub fn is_binance_supported(symbol: &str) -> bool {
    to_kraken_pair_lossy(symbol).is_some()
}

/// Get all supported crypto symbols from a list.
pub fn get_binance_crypto_symbols(symbols: &[String]) -> Vec<String> {
    symbols
        .iter()
        .filter(|s| is_binance_supported(s))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn kraken_pair_catalog_resolves_live_asset_pairs() {
        let body = json!({
            "error": [],
            "result": {
                "XXMRZUSD": {
                    "altname": "XMRUSD",
                    "wsname": "XMR/USD",
                    "status": "online"
                },
                "POLUSD": {
                    "altname": "POLUSD",
                    "wsname": "POL/USD",
                    "status": "online"
                },
                "XDGUSD": {
                    "altname": "XDGUSD",
                    "wsname": "XDG/USD",
                    "status": "online"
                },
                "MATICUSD": {
                    "altname": "MATICUSD",
                    "wsname": "MATIC/USD",
                    "status": "delisted"
                }
            }
        });
        let catalog = parse_kraken_pair_catalog(&body).unwrap();
        assert_eq!(
            catalog.by_symbol.get("XMRUSD"),
            Some(&"XXMRZUSD".to_string())
        );
        assert_eq!(catalog.by_symbol.get("POLUSD"), Some(&"POLUSD".to_string()));
        assert_eq!(
            catalog.by_symbol.get("DOGEUSD"),
            Some(&"XDGUSD".to_string())
        );
        assert!(!catalog.by_symbol.contains_key("MATICUSD"));
    }

    #[test]
    fn kraken_pair_static_pol_alias_uses_live_pol_market() {
        assert_eq!(to_kraken_pair("POLUSD"), Some("POLUSD"));
        assert_eq!(to_kraken_pair("MATICUSD"), Some("POLUSD"));
    }

    // ── to_kraken_interval ─────────────────────────────────

    #[test]
    fn kraken_interval_mapping() {
        assert_eq!(to_kraken_interval("1Min"), Some(1));
        assert_eq!(to_kraken_interval("5Min"), Some(5));
        assert_eq!(to_kraken_interval("15Min"), Some(15));
        assert_eq!(to_kraken_interval("30Min"), Some(30));
        assert_eq!(to_kraken_interval("1Hour"), Some(60));
        assert_eq!(to_kraken_interval("4Hour"), Some(240));
        assert_eq!(to_kraken_interval("1Day"), Some(1440));
        assert_eq!(to_kraken_interval("1Week"), Some(10080));
    }

    #[test]
    fn kraken_interval_unsupported() {
        assert_eq!(to_kraken_interval("1Month"), None);
        assert_eq!(to_kraken_interval(""), None);
        assert_eq!(to_kraken_interval("2Hour"), None);
        assert_eq!(to_kraken_interval("garbage"), None);
    }

    // ── to_kraken_pair ─────────────────────────────────────

    #[test]
    fn kraken_pair_major_cryptos() {
        assert_eq!(to_kraken_pair("BTCUSD"), Some("XBTUSD"));
        assert_eq!(to_kraken_pair("ETHUSD"), Some("ETHUSD"));
        assert_eq!(to_kraken_pair("SOLUSD"), Some("SOLUSD"));
        assert_eq!(to_kraken_pair("DOGEUSD"), Some("XDGUSD"));
        assert_eq!(to_kraken_pair("XRPUSD"), Some("XRPUSD"));
    }

    #[test]
    fn kraken_pair_with_slash() {
        assert_eq!(to_kraken_pair("BTC/USD"), Some("XBTUSD"));
        assert_eq!(to_kraken_pair("ETH/USD"), Some("ETHUSD"));
        assert_eq!(to_kraken_pair("DOGE/USD"), Some("XDGUSD"));
    }

    #[test]
    fn kraken_pair_case_insensitive() {
        // to_kraken_pair normalizes to uppercase internally
        assert_eq!(to_kraken_pair("btcusd"), Some("XBTUSD"));
        assert_eq!(to_kraken_pair("Ethusd"), Some("ETHUSD"));
    }

    #[test]
    fn kraken_pair_unsupported() {
        assert_eq!(to_kraken_pair("FAKEUSD"), None);
        assert_eq!(to_kraken_pair(""), None);
        assert_eq!(to_kraken_pair("BTCEUR"), None);
    }

    #[test]
    fn kraken_pair_matic_pol_alias() {
        assert_eq!(to_kraken_pair("MATICUSD"), Some("POLUSD"));
        assert_eq!(to_kraken_pair("POLUSD"), Some("POLUSD"));
    }

    #[test]
    fn normalize_pair_symbol_handles_display_and_wrapped_forms() {
        assert_eq!(normalize_pair_symbol("BTC/USD"), "BTCUSD");
        assert_eq!(normalize_pair_symbol("XBTUSD"), "BTCUSD");
        assert_eq!(normalize_pair_symbol("XXBTZUSD"), "BTCUSD");
        assert_eq!(normalize_pair_symbol("ETH/BTC"), "ETHBTC");
        assert_eq!(normalize_pair_symbol("XDG/USD"), "DOGEUSD");
    }

    #[test]
    fn kraken_pair_lossy_supports_dynamic_pairs() {
        assert_eq!(to_kraken_pair_lossy("BTC/USD"), Some("XBTUSD".to_string()));
        assert_eq!(to_kraken_pair_lossy("ETH/BTC"), Some("ETHBTC".to_string()));
        assert_eq!(to_kraken_pair_lossy("ETH/EUR"), Some("ETHEUR".to_string()));
        assert_eq!(to_kraken_pair_lossy("XXBTZUSD"), Some("XBTUSD".to_string()));
    }

    // ── is_binance_supported / get_binance_crypto_symbols ──

    #[test]
    fn is_supported_known_pairs() {
        assert!(is_binance_supported("BTCUSD"));
        assert!(is_binance_supported("ETHUSD"));
        assert!(is_binance_supported("SOLUSD"));
        assert!(is_binance_supported("BTC/USD"));
    }

    #[test]
    fn is_not_supported_unknown() {
        assert!(!is_binance_supported("AAPL"));
        assert!(!is_binance_supported(""));
    }

    #[test]
    fn get_supported_symbols_filters() {
        let syms = vec![
            "BTCUSD".to_string(),
            "ETHUSD".to_string(),
            "FAKEUSD".to_string(),
            "AAPL".to_string(),
        ];
        let result = get_binance_crypto_symbols(&syms);
        assert_eq!(result.len(), 3);
        assert!(result.contains(&"BTCUSD".to_string()));
        assert!(result.contains(&"ETHUSD".to_string()));
        assert!(result.contains(&"FAKEUSD".to_string()));
    }

    #[test]
    fn get_supported_symbols_empty() {
        let result = get_binance_crypto_symbols(&[]);
        assert!(result.is_empty());
    }

    // ── aggregate_to_monthly ───────────────────────────────

    fn make_bar(ts: &str, o: f64, h: f64, l: f64, c: f64, v: f64) -> serde_json::Value {
        json!({
            "timestamp": ts,
            "open": o,
            "high": h,
            "low": l,
            "close": c,
            "volume": v,
        })
    }

    #[test]
    fn monthly_aggregation_basic() {
        let daily = vec![
            make_bar(
                "2024-03-01T00:00:00Z",
                60000.0,
                62000.0,
                58000.0,
                61000.0,
                100.0,
            ),
            make_bar(
                "2024-03-15T00:00:00Z",
                61000.0,
                65000.0,
                59000.0,
                64000.0,
                200.0,
            ),
            make_bar(
                "2024-04-01T00:00:00Z",
                64000.0,
                70000.0,
                63000.0,
                68000.0,
                300.0,
            ),
        ];
        let result = aggregate_to_monthly(&daily);
        assert_eq!(result.len(), 2);

        // March
        let mar = &result[0];
        assert_eq!(mar["open"].as_f64().unwrap(), 60000.0);
        assert_eq!(mar["high"].as_f64().unwrap(), 65000.0);
        assert_eq!(mar["low"].as_f64().unwrap(), 58000.0);
        assert_eq!(mar["close"].as_f64().unwrap(), 64000.0);
        assert!((mar["volume"].as_f64().unwrap() - 300.0).abs() < 1e-10);

        // April
        let apr = &result[1];
        assert_eq!(apr["open"].as_f64().unwrap(), 64000.0);
    }

    #[test]
    fn monthly_aggregation_empty() {
        let result = aggregate_to_monthly(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn monthly_aggregation_short_timestamp_skipped() {
        let daily = vec![
            json!({"timestamp": "bad", "open": 1.0, "high": 2.0, "low": 0.5, "close": 1.5, "volume": 10.0}),
        ];
        let result = aggregate_to_monthly(&daily);
        assert!(result.is_empty());
    }

    #[test]
    fn monthly_aggregation_single_day() {
        let daily = vec![make_bar(
            "2024-06-15T00:00:00Z",
            50.0,
            55.0,
            45.0,
            52.0,
            100.0,
        )];
        let result = aggregate_to_monthly(&daily);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["open"].as_f64().unwrap(), 50.0);
        assert_eq!(result[0]["close"].as_f64().unwrap(), 52.0);
    }

    // ── OHLCV parsing from mock Kraken JSON ────────────────

    #[test]
    fn parse_mock_kraken_response() {
        // Simulate the structure Kraken returns
        let mock_response = json!({
            "error": [],
            "result": {
                "XBTUSD": [
                    [1704067200, "42000.0", "42500.0", "41500.0", "42200.0", "42100.0", "150.5", 1000],
                    [1704070800, "42200.0", "43000.0", "42000.0", "42800.0", "42500.0", "200.3", 1200],
                ],
                "last": 1704070800
            }
        });

        // Verify error array is empty
        let errors = mock_response["error"].as_array().unwrap();
        assert!(errors.is_empty());

        // Parse the OHLCV data the same way fetch_binance_klines does
        let result = &mock_response["result"];
        let mut parsed_bars = Vec::new();

        for (key, val) in result.as_object().unwrap() {
            if key == "last" {
                continue;
            }
            if let Some(arr) = val.as_array() {
                for kline in arr {
                    if let Some(k) = kline.as_array() {
                        if k.len() < 7 {
                            continue;
                        }
                        let ts = k[0].as_i64().unwrap_or(0);
                        let open = k[1]
                            .as_str()
                            .and_then(|s| s.parse::<f64>().ok())
                            .unwrap_or(0.0);
                        let high = k[2]
                            .as_str()
                            .and_then(|s| s.parse::<f64>().ok())
                            .unwrap_or(0.0);
                        let low = k[3]
                            .as_str()
                            .and_then(|s| s.parse::<f64>().ok())
                            .unwrap_or(0.0);
                        let close = k[4]
                            .as_str()
                            .and_then(|s| s.parse::<f64>().ok())
                            .unwrap_or(0.0);
                        let volume = k[6]
                            .as_str()
                            .and_then(|s| s.parse::<f64>().ok())
                            .unwrap_or(0.0);

                        if open > 0.0 {
                            parsed_bars.push(json!({
                                "timestamp": ts,
                                "open": open, "high": high, "low": low, "close": close, "volume": volume,
                            }));
                        }
                    }
                }
            }
        }

        assert_eq!(parsed_bars.len(), 2);
        assert_eq!(parsed_bars[0]["open"].as_f64().unwrap(), 42000.0);
        assert_eq!(parsed_bars[0]["high"].as_f64().unwrap(), 42500.0);
        assert_eq!(parsed_bars[0]["low"].as_f64().unwrap(), 41500.0);
        assert_eq!(parsed_bars[0]["close"].as_f64().unwrap(), 42200.0);
        assert_eq!(parsed_bars[0]["volume"].as_f64().unwrap(), 150.5);

        assert_eq!(parsed_bars[1]["open"].as_f64().unwrap(), 42200.0);
        assert_eq!(parsed_bars[1]["close"].as_f64().unwrap(), 42800.0);
    }

    #[test]
    fn parse_kraken_ohlc_response_helper_filters_and_sorts_input_shape() {
        let mock_response = json!({
            "error": [],
            "result": {
                "XBTUSD": [
                    [1704067200, "42000.0", "42500.0", "41500.0", "42200.0", "42100.0", "150.5", 1000],
                    [1704070800, "0.0", "43000.0", "42000.0", "42800.0", "42500.0", "200.3", 1200],
                    [1704074400, "42800.0", "43100.0", "42700.0", "43000.0", "42900.0", "50.0", 500],
                ],
                "last": 1704074400
            }
        });
        let bars = parse_kraken_ohlc_response(
            &mock_response,
            1704067200_i64 * 1000,
            1704074400_i64 * 1000,
        )
        .unwrap();
        assert_eq!(bars.len(), 2);
        assert_eq!(bars[0]["open"].as_f64().unwrap(), 42000.0);
        assert_eq!(bars[1]["close"].as_f64().unwrap(), 43000.0);
    }

    #[test]
    fn parse_kraken_ohlc_response_helper_returns_api_error() {
        let mock_response = json!({
            "error": ["EGeneral:Too many requests"],
            "result": {}
        });
        let err = parse_kraken_ohlc_response(&mock_response, 0, i64::MAX).unwrap_err();
        assert!(err.contains("Too many requests"));
    }

    #[test]
    fn parse_kraken_response_with_errors() {
        let mock_response = json!({
            "error": ["EGeneral:Too many requests"],
            "result": {}
        });
        let errors = mock_response["error"].as_array().unwrap();
        assert!(!errors.is_empty());
        let err_msg: Vec<&str> = errors.iter().filter_map(|e| e.as_str()).collect();
        assert_eq!(err_msg[0], "EGeneral:Too many requests");
    }

    #[test]
    fn kraken_rate_limit_detection_covers_public_and_rest_errors() {
        assert!(is_kraken_rate_limit_error("EGeneral:Too many requests"));
        assert!(is_kraken_rate_limit_error("EAPI:Rate limit exceeded"));
        assert!(is_kraken_rate_limit_error(
            "EService: Throttled: 1999999999"
        ));
        assert!(is_kraken_rate_limit_error(
            "Kraken API error 429: retry later"
        ));
        assert!(!is_kraken_rate_limit_error("EQuery:Unknown asset pair"));
    }

    #[test]
    fn kraken_throttled_wait_parses_retry_timestamp() {
        let retry_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 30;
        let wait = kraken_throttled_wait(&format!("EService: Throttled: {retry_at}")).unwrap();
        assert!(wait <= Duration::from_secs(30));
        assert!(wait >= Duration::from_secs(20));
    }

    #[test]
    fn parse_kraken_short_kline_skipped() {
        // Klines with fewer than 7 elements should be skipped
        let mock_response = json!({
            "error": [],
            "result": {
                "XBTUSD": [
                    [1704067200, "42000.0", "42500.0"], // too short
                    [1704070800, "42200.0", "43000.0", "42000.0", "42800.0", "42500.0", "200.3", 1200], // valid
                ],
                "last": 1704070800
            }
        });

        let result = &mock_response["result"];
        let mut count = 0;
        for (key, val) in result.as_object().unwrap() {
            if key == "last" {
                continue;
            }
            if let Some(arr) = val.as_array() {
                for kline in arr {
                    if let Some(k) = kline.as_array() {
                        if k.len() < 7 {
                            continue;
                        }
                        count += 1;
                    }
                }
            }
        }
        assert_eq!(count, 1, "should skip short klines");
    }

    #[test]
    fn parse_kraken_zero_open_skipped() {
        // Bars with open=0 should be skipped
        let mock_response = json!({
            "error": [],
            "result": {
                "XBTUSD": [
                    [1704067200, "0.0", "42500.0", "41500.0", "42200.0", "42100.0", "150.5", 1000],
                ],
                "last": 1704067200
            }
        });

        let result = &mock_response["result"];
        let mut parsed = Vec::new();
        for (key, val) in result.as_object().unwrap() {
            if key == "last" {
                continue;
            }
            if let Some(arr) = val.as_array() {
                for kline in arr {
                    if let Some(k) = kline.as_array() {
                        if k.len() < 7 {
                            continue;
                        }
                        let open = k[1]
                            .as_str()
                            .and_then(|s| s.parse::<f64>().ok())
                            .unwrap_or(0.0);
                        if open > 0.0 {
                            parsed.push(open);
                        }
                    }
                }
            }
        }
        assert!(parsed.is_empty(), "zero-open bars should be skipped");
    }

    #[test]
    fn parse_kraken_empty_result() {
        let mock_response = json!({
            "error": [],
            "result": {
                "XBTUSD": [],
                "last": 0
            }
        });
        let result = &mock_response["result"];
        let mut count = 0;
        for (key, val) in result.as_object().unwrap() {
            if key == "last" {
                continue;
            }
            if let Some(arr) = val.as_array() {
                count += arr.len();
            }
        }
        assert_eq!(count, 0);
    }
}
