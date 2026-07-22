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
    wsname_by_symbol: HashMap<String, String>,
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
        } else if raw.len() > 4 {
            ["ZUSD", "ZEUR", "ZGBP", "ZJPY", "ZCAD", "ZAUD", "ZCHF"]
                .iter()
                .find_map(|quote| {
                    raw.strip_suffix(quote)
                        .map(|base| format!("{}{}", base, &quote[1..]))
                })
                .unwrap_or(raw)
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

fn insert_kraken_ws_alias(
    map: &mut HashMap<String, String>,
    source: impl AsRef<str>,
    wsname: &str,
) {
    let symbol = normalize_pair_symbol(source.as_ref())
        .replace('/', "")
        .to_ascii_uppercase();
    if !symbol.is_empty() && !wsname.is_empty() {
        map.entry(symbol).or_insert_with(|| wsname.to_string());
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
    let mut wsname_by_symbol = HashMap::with_capacity(result.len() * 3);

    for (pair_name, info) in result {
        let status = info
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("online");
        if status == "delisted" {
            continue;
        }

        let wsname = info
            .get("wsname")
            .and_then(|v| v.as_str())
            .unwrap_or(pair_name);

        insert_kraken_pair_alias(&mut by_symbol, pair_name, pair_name);
        insert_kraken_ws_alias(&mut wsname_by_symbol, pair_name, wsname);
        if let Some(altname) = info.get("altname").and_then(|v| v.as_str()) {
            insert_kraken_pair_alias(&mut by_symbol, altname, pair_name);
            insert_kraken_ws_alias(&mut wsname_by_symbol, altname, wsname);
        }
        if let Some(wsname) = info.get("wsname").and_then(|v| v.as_str()) {
            insert_kraken_pair_alias(&mut by_symbol, wsname, pair_name);
            insert_kraken_ws_alias(&mut wsname_by_symbol, wsname, wsname);
        }
    }

    Ok(KrakenPairCatalog {
        by_symbol,
        wsname_by_symbol,
    })
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

/// Resolve a TyphooN/Kraken symbol to Kraken's public WebSocket `wsname`
/// format, e.g. `BTC/USD` -> `XBT/USD`.
pub async fn resolve_kraken_ws_pair(client: &reqwest::Client, symbol: &str) -> Option<String> {
    let normalized = normalize_pair_symbol(symbol)
        .replace('/', "")
        .to_ascii_uppercase();
    if normalized.is_empty() {
        return None;
    }

    if let Ok(catalog) = load_kraken_pair_catalog(client).await
        && let Some(wsname) = catalog.wsname_by_symbol.get(&normalized)
    {
        return Some(wsname.clone());
    }

    kraken_ws_pair_lossy(&normalized)
}

fn kraken_ws_pair_lossy(normalized: &str) -> Option<String> {
    const QUOTES: &[&str] = &[
        "USDT", "USDC", "PYUSD", "EUR", "USD", "GBP", "CAD", "AUD", "JPY", "CHF", "BTC", "XBT",
        "ETH",
    ];
    let normalized = normalize_pair_symbol(normalized)
        .replace('/', "")
        .to_ascii_uppercase();
    for quote in QUOTES {
        if let Some(base) = normalized.strip_suffix(quote)
            && !base.is_empty()
        {
            let base = match base {
                "BTC" => "XBT",
                "DOGE" => "XDG",
                other => other,
            };
            let quote = if *quote == "BTC" { "XBT" } else { quote };
            return Some(format!("{base}/{quote}"));
        }
    }
    None
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
mod tests;
