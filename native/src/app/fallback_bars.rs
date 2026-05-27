//! Lightweight public fallback bar providers for Kraken-equity assist mode.
//!
//! These fetchers deliberately store provenance under their own cache prefixes
//! (`yahoo-chart:` and `stooq:`). They never overwrite Kraken/Alpaca bars and
//! are only queued by explicit Settings → Backfill providers toggles.

use super::*;

#[derive(Debug, Clone)]
pub(super) struct FallbackBar {
    pub ts_ms: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

fn valid_bar(bar: &FallbackBar) -> bool {
    bar.ts_ms > 0
        && bar.open.is_finite()
        && bar.high.is_finite()
        && bar.low.is_finite()
        && bar.close.is_finite()
        && bar.volume.is_finite()
        && bar.open > 0.0
        && bar.high > 0.0
        && bar.low > 0.0
        && bar.close > 0.0
        && bar.high >= bar.low
}

fn fallback_bars_to_cache_json(bars: &[FallbackBar]) -> Result<String, String> {
    let json_bars: Vec<_> = bars
        .iter()
        .filter(|bar| valid_bar(bar))
        .filter_map(|bar| {
            let ts = chrono::DateTime::from_timestamp_millis(bar.ts_ms)?.to_rfc3339();
            Some(serde_json::json!({
                "timestamp": ts,
                "open": bar.open,
                "high": bar.high,
                "low": bar.low,
                "close": bar.close,
                "volume": bar.volume,
            }))
        })
        .collect();
    serde_json::to_string(&json_bars).map_err(|e| format!("serialize fallback bars: {e}"))
}

pub(super) fn yahoo_chart_supports_timeframe(timeframe: &str) -> bool {
    matches!(
        normalize_sync_timeframe_key(timeframe),
        Some("1Min" | "5Min" | "15Min" | "30Min" | "1Hour" | "1Day" | "1Week" | "1Month")
    )
}

pub(super) fn stooq_supports_timeframe(timeframe: &str) -> bool {
    matches!(normalize_sync_timeframe_key(timeframe), Some("1Day"))
}

fn yahoo_interval_and_range(timeframe: &str) -> Option<(&'static str, &'static str)> {
    match normalize_sync_timeframe_key(timeframe)? {
        // Yahoo hard-limits 1m history. Keep this as freshness assist only.
        "1Min" => Some(("1m", "7d")),
        "5Min" => Some(("5m", "60d")),
        "15Min" => Some(("15m", "60d")),
        "30Min" => Some(("30m", "60d")),
        "1Hour" => Some(("1h", "2y")),
        "1Day" => Some(("1d", "max")),
        "1Week" => Some(("1wk", "max")),
        "1Month" => Some(("1mo", "max")),
        _ => None,
    }
}

pub(super) async fn fetch_yahoo_chart_bars(
    client: &reqwest::Client,
    symbol: &str,
    timeframe: &str,
) -> Result<Vec<FallbackBar>, String> {
    let (interval, range) = yahoo_interval_and_range(timeframe)
        .ok_or_else(|| format!("Yahoo Chart unsupported timeframe {timeframe}"))?;
    let symbol = normalize_market_data_symbol(symbol)
        .replace('/', "")
        .trim_end_matches(".EQ")
        .to_ascii_uppercase();
    if symbol.is_empty() {
        return Err("Yahoo Chart empty symbol".to_string());
    }
    let url = format!(
        "https://query1.finance.yahoo.com/v8/finance/chart/{symbol}?range={range}&interval={interval}&events=history&includePrePost=false"
    );
    let resp = client
        .get(&url)
        .header("User-Agent", "TyphooN-Terminal/1.0")
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("Yahoo Chart request failed for {symbol} {timeframe}: {e}"))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(format!(
            "Yahoo Chart HTTP {status} for {symbol} {timeframe}"
        ));
    }
    let root: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Yahoo Chart JSON parse failed for {symbol} {timeframe}: {e}"))?;
    if let Some(err) = root["chart"]["error"].as_object() {
        return Err(format!(
            "Yahoo Chart error for {symbol} {timeframe}: {err:?}"
        ));
    }
    let result = root["chart"]["result"]
        .as_array()
        .and_then(|arr| arr.first())
        .ok_or_else(|| format!("Yahoo Chart empty result for {symbol} {timeframe}"))?;
    let Some(ts) = result["timestamp"].as_array() else {
        return Ok(Vec::new());
    };
    let quote = result["indicators"]["quote"]
        .as_array()
        .and_then(|arr| arr.first())
        .ok_or_else(|| format!("Yahoo Chart missing quote arrays for {symbol} {timeframe}"))?;
    let opens = quote["open"].as_array().cloned().unwrap_or_default();
    let highs = quote["high"].as_array().cloned().unwrap_or_default();
    let lows = quote["low"].as_array().cloned().unwrap_or_default();
    let closes = quote["close"].as_array().cloned().unwrap_or_default();
    let volumes = quote["volume"].as_array().cloned().unwrap_or_default();

    let mut bars = Vec::with_capacity(ts.len());
    for i in 0..ts.len() {
        let Some(sec) = ts.get(i).and_then(|v| v.as_i64()) else {
            continue;
        };
        let Some(open) = opens.get(i).and_then(|v| v.as_f64()) else {
            continue;
        };
        let Some(high) = highs.get(i).and_then(|v| v.as_f64()) else {
            continue;
        };
        let Some(low) = lows.get(i).and_then(|v| v.as_f64()) else {
            continue;
        };
        let Some(close) = closes.get(i).and_then(|v| v.as_f64()) else {
            continue;
        };
        let volume = volumes.get(i).and_then(|v| v.as_f64()).unwrap_or(0.0);
        let bar = FallbackBar {
            ts_ms: sec.saturating_mul(1000),
            open,
            high,
            low,
            close,
            volume,
        };
        if valid_bar(&bar) {
            bars.push(bar);
        }
    }
    bars.sort_by_key(|bar| bar.ts_ms);
    bars.dedup_by_key(|bar| bar.ts_ms);
    Ok(bars)
}

fn stooq_symbol(symbol: &str) -> String {
    let bare = normalize_market_data_symbol(symbol)
        .replace('/', "")
        .trim_end_matches(".EQ")
        .to_ascii_lowercase();
    if bare.contains('.') {
        bare
    } else {
        format!("{bare}.us")
    }
}

pub(super) async fn fetch_stooq_daily_bars(
    client: &reqwest::Client,
    symbol: &str,
    timeframe: &str,
) -> Result<Vec<FallbackBar>, String> {
    if !stooq_supports_timeframe(timeframe) {
        return Err(format!("Stooq unsupported timeframe {timeframe}"));
    }
    let stooq_symbol = stooq_symbol(symbol);
    if stooq_symbol == ".us" || stooq_symbol.is_empty() {
        return Err("Stooq empty symbol".to_string());
    }
    let url = format!("https://stooq.com/q/d/l/?s={stooq_symbol}&i=d");
    let resp = client
        .get(&url)
        .header("User-Agent", "TyphooN-Terminal/1.0")
        .header("Accept", "text/csv,*/*")
        .send()
        .await
        .map_err(|e| format!("Stooq request failed for {symbol} {timeframe}: {e}"))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(format!("Stooq HTTP {status} for {symbol} {timeframe}"));
    }
    let text = resp
        .text()
        .await
        .map_err(|e| format!("Stooq CSV read failed for {symbol} {timeframe}: {e}"))?;
    let mut bars = Vec::new();
    for line in text.lines().skip(1) {
        let cols: Vec<&str> = line.split(',').collect();
        if cols.len() < 6 || cols.iter().any(|v| v.trim().eq_ignore_ascii_case("N/D")) {
            continue;
        }
        let Ok(date) = chrono::NaiveDate::parse_from_str(cols[0].trim(), "%Y-%m-%d") else {
            continue;
        };
        let Some(dt) = date.and_hms_opt(0, 0, 0) else {
            continue;
        };
        let parse = |idx: usize| cols[idx].trim().parse::<f64>().ok();
        let Some(open) = parse(1) else { continue };
        let Some(high) = parse(2) else { continue };
        let Some(low) = parse(3) else { continue };
        let Some(close) = parse(4) else { continue };
        let volume = parse(5).unwrap_or(0.0);
        let bar = FallbackBar {
            ts_ms: dt.and_utc().timestamp_millis(),
            open,
            high,
            low,
            close,
            volume,
        };
        if valid_bar(&bar) {
            bars.push(bar);
        }
    }
    bars.sort_by_key(|bar| bar.ts_ms);
    bars.dedup_by_key(|bar| bar.ts_ms);
    Ok(bars)
}

pub(super) fn store_fallback_bars(
    cache: &SqliteCache,
    source: &str,
    symbol: &str,
    timeframe: &str,
    bars: &[FallbackBar],
) -> Result<usize, String> {
    let symbol = normalize_market_data_symbol(symbol)
        .replace('/', "")
        .trim_end_matches(".EQ")
        .to_ascii_uppercase();
    let Some(tf) = normalize_sync_timeframe_key(timeframe) else {
        return Err(format!("unsupported timeframe {timeframe}"));
    };
    let valid_count = bars.iter().filter(|bar| valid_bar(bar)).count();
    let json = fallback_bars_to_cache_json(bars)?;
    let cache_key = format!("{source}:{symbol}:{tf}");
    cache
        .put_bars(&cache_key, &json)
        .map_err(|e| format!("{source} cache write failed for {symbol} {tf}: {e}"))?;
    Ok(valid_count)
}
