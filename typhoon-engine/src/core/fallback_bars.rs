//! Lightweight public fallback bar providers for Kraken-equity assist mode.
//!
//! These fetchers deliberately store provenance under their own cache prefixes
//! (`yahoo-chart:`). They never overwrite Kraken/Alpaca bars and
//! are only queued by explicit Settings → Backfill providers toggles.

use crate::core::cache::SqliteCache;

const STANDARD_SYNC_TIMEFRAMES: [(&str, &str); 9] = [
    ("M1", "1Min"),
    ("M5", "5Min"),
    ("M15", "15Min"),
    ("M30", "30Min"),
    ("H1", "1Hour"),
    ("H4", "4Hour"),
    ("D1", "1Day"),
    ("W1", "1Week"),
    ("MN1", "1Month"),
];

fn bare_symbol_from_key(key: &str) -> String {
    let parts: Vec<&str> = key.split(':').collect();
    match parts.as_slice() {
        [_src, sym, _tf] => (*sym).to_string(),
        [sym, _tf] => (*sym).to_string(),
        _ => key.to_string(),
    }
}

fn normalize_market_data_symbol(symbol: &str) -> String {
    let bare = bare_symbol_from_key(symbol).to_uppercase();
    match bare.rsplit_once('.') {
        Some((head, suffix))
            if (2..=4).contains(&suffix.len())
                && suffix.chars().all(|c| c.is_ascii_uppercase()) =>
        {
            head.to_string()
        }
        _ => bare,
    }
}

fn normalize_sync_timeframe_key(tf: &str) -> Option<&'static str> {
    STANDARD_SYNC_TIMEFRAMES.iter().find_map(|(short, cache)| {
        if tf.eq_ignore_ascii_case(short) || tf.eq_ignore_ascii_case(cache) {
            Some(*cache)
        } else {
            None
        }
    })
}

#[derive(Debug, Clone)]
pub struct FallbackBar {
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

/// Timeframes the Yahoo chart assist lane will fetch. Intentionally daily and up:
/// Yahoo's unauthenticated endpoint is rate-fragile (429 → lane backoff) and its
/// intraday history is a shallow recent window, so in practice the 15Min/30Min/
/// 1Hour rows never converged (~0.2% synced) while they consumed the lane's tiny
/// budget. Yahoo's real merge value is breadth on the higher timeframes (1Month
/// neared 100%, 1Week ~10%), so we focus the limited lane there — intraday equity
/// depth comes from Alpaca / Kraken instead.
pub fn yahoo_chart_supports_timeframe(timeframe: &str) -> bool {
    matches!(
        normalize_sync_timeframe_key(timeframe),
        Some("1Day" | "1Week" | "1Month")
    )
}

fn yahoo_chart_request_symbol(symbol: &str) -> String {
    let symbol = normalize_market_data_symbol(symbol)
        .replace('/', "")
        .trim_end_matches(".EQ")
        .to_ascii_uppercase();
    // Yahoo uses hyphenated class-share symbols (`BRK-B`, `BH-A`) while Kraken
    // Securities catalogs and several other feeds commonly use dotted class
    // notation (`BRK.B`, `BH.A`). Do not blindly rewrite single-letter non-class
    // suffixes such as SPAC units (`DGAC.U`) into invented Yahoo symbols.
    if let Some((base, suffix)) = symbol.split_once('.') {
        if matches!(suffix, "A" | "B" | "C") && base.chars().all(|c| c.is_ascii_alphabetic()) {
            return format!("{base}-{suffix}");
        }
    }
    symbol
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum YahooChartWindow {
    Range(&'static str),
    PeriodFromUnixEpoch,
}

fn yahoo_interval_and_window(timeframe: &str) -> Option<(&'static str, YahooChartWindow)> {
    match normalize_sync_timeframe_key(timeframe)? {
        // Yahoo hard-limits 1m history. Keep this as freshness assist only.
        "1Min" => Some(("1m", YahooChartWindow::Range("7d"))),
        "5Min" => Some(("5m", YahooChartWindow::Range("60d"))),
        "15Min" => Some(("15m", YahooChartWindow::Range("60d"))),
        "30Min" => Some(("30m", YahooChartWindow::Range("60d"))),
        "1Hour" => Some(("1h", YahooChartWindow::Range("2y"))),
        // Yahoo silently down-samples range=max daily/weekly requests for some
        // equities (TNDM reproduced this) to monthly bars while returning 200.
        // Use explicit period bounds so the requested granularity is honored.
        "1Day" => Some(("1d", YahooChartWindow::PeriodFromUnixEpoch)),
        "1Week" => Some(("1wk", YahooChartWindow::PeriodFromUnixEpoch)),
        // Yahoo also silently down-samples `range=max&interval=1mo` for long-lived
        // and short-lived symbols: old equities can come back as 3mo bars, while
        // young ETFs/SPACs can come back as 1wk/1d/1h bars. Period bounds preserve
        // actual monthly granularity across both shapes.
        "1Month" => Some(("1mo", YahooChartWindow::PeriodFromUnixEpoch)),
        _ => None,
    }
}

fn yahoo_expected_granularity(timeframe: &str) -> Option<&'static str> {
    match normalize_sync_timeframe_key(timeframe)? {
        "1Min" => Some("1m"),
        "5Min" => Some("5m"),
        "15Min" => Some("15m"),
        "30Min" => Some("30m"),
        "1Hour" => Some("1h"),
        "1Day" => Some("1d"),
        "1Week" => Some("1wk"),
        "1Month" => Some("1mo"),
        _ => None,
    }
}

pub fn yahoo_chart_provider_no_data_error(error: &str) -> bool {
    let e = error.to_lowercase();
    if e.contains("rate limited") || e.contains("429") || e.contains("too many") || e.contains("throttl") {
        return false;  // transient; let backoff / Error path handle it
    }
    e.contains("http 400")
        || e.contains("http 404")
        || e.contains("empty result")
        || e.contains("missing quote arrays")
        || e.contains("no valid bars")
        || e.contains("yahoo chart returned")
}

async fn _fetch_yahoo_chart_bars_internal(
    client: &reqwest::Client,
    symbol: &str,
    timeframe: &str,
) -> Result<Vec<FallbackBar>, String> {
    let (interval, window) = yahoo_interval_and_window(timeframe)
        .ok_or_else(|| format!("Yahoo Chart unsupported timeframe {timeframe}"))?;
    let expected_granularity = yahoo_expected_granularity(timeframe)
        .ok_or_else(|| format!("Yahoo Chart unsupported timeframe {timeframe}"))?;
    let symbol = yahoo_chart_request_symbol(symbol);
    if symbol.is_empty() {
        return Err("Yahoo Chart empty symbol".to_string());
    }
    let url = match window {
        YahooChartWindow::Range(range) => format!(
            "https://query1.finance.yahoo.com/v8/finance/chart/{symbol}?range={range}&interval={interval}&events=history&includePrePost=false"
        ),
        YahooChartWindow::PeriodFromUnixEpoch => {
            let period2 = chrono::Utc::now().timestamp().saturating_add(86_400);
            format!(
                "https://query1.finance.yahoo.com/v8/finance/chart/{symbol}?period1=0&period2={period2}&interval={interval}&events=history&includePrePost=false"
            )
        }
    };
    let resp = client
        .get(&url)
        .header("User-Agent", "TyphooN-Terminal/1.0")
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("Yahoo Chart request failed for {symbol} {timeframe}: {e}"))?;
    let status = resp.status();
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err(format!("Yahoo Chart rate limited (429) for {symbol} {timeframe}"));
    }
    if !status.is_success() {
        return Err(format!(
            "Yahoo Chart HTTP {status} for {symbol} {timeframe}"
        ));
    }
    let body_text = resp
        .text()
        .await
        .map_err(|e| format!("Yahoo Chart read body failed for {symbol} {timeframe}: {e}"))?;
    let lower = body_text.to_lowercase();
    if lower.contains("too many requests") || lower.contains("rate limit") || lower.contains("throttl") {
        return Err(format!("Yahoo Chart rate limited for {symbol} {timeframe}"));
    }
    let root: serde_json::Value = serde_json::from_str(&body_text)
        .map_err(|e| {
            let preview: String = body_text.chars().take(400).collect();
            format!(
                "Yahoo Chart JSON parse failed for {symbol} {timeframe}: {e}; body preview: {preview}"
            )
        })?;
    if let Some(err) = root["chart"]["error"].as_object() {
        return Err(format!(
            "Yahoo Chart error for {symbol} {timeframe}: {err:?}"
        ));
    }
    let result = root["chart"]["result"]
        .as_array()
        .and_then(|arr| arr.first())
        .ok_or_else(|| format!("Yahoo Chart empty result for {symbol} {timeframe}"))?;
    let actual_granularity = result["meta"]["dataGranularity"].as_str().unwrap_or("");
    if actual_granularity != expected_granularity {
        return Err(format!(
            "Yahoo Chart returned {actual_granularity} bars for {symbol} {timeframe}; expected {expected_granularity}"
        ));
    }
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
    // Split+dividend-adjusted close (daily+ only; Yahoo omits adjclose for
    // intraday granularities). Where present we rebase the whole bar onto the
    // adjusted scale by the per-bar adjclose/close ratio — Yahoo only adjusts
    // close, not O/H/L — so yahoo-chart becomes a clean, scale-consistent
    // corroborator that matches TradingView's adjusted view and no longer needs
    // the deep-history back-adjust hack for split/redenomination actions (e.g.
    // WOK's ~10,000× unadjusted era). See ADR-113.
    let adj_closes = root["chart"]["result"][0]["indicators"]["adjclose"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|obj| obj["adjclose"].as_array())
        .cloned()
        .unwrap_or_default();

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
        // Rebase onto the split/dividend-adjusted scale where Yahoo provides it.
        let (open, high, low, close) = match adj_closes.get(i).and_then(|v| v.as_f64()) {
            Some(adj) if adj > 0.0 && close > 0.0 => {
                let factor = adj / close;
                (open * factor, high * factor, low * factor, adj)
            }
            _ => (open, high, low, close),
        };
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

pub async fn fetch_yahoo_chart_bars(
    client: &reqwest::Client,
    symbol: &str,
    timeframe: &str,
) -> Result<Vec<FallbackBar>, String> {
    let result = _fetch_yahoo_chart_bars_internal(client, symbol, timeframe).await;

    // Yahoo Finance often down-samples 1h requests to 1d for illiquid/young symbols
    // without returning an error code. If we hit a granularity mismatch on 1Hour,
    // or a 422 Unprocessable Entity (interval unsupported), retry once at 1Day
    // to get at least some historical context.
    if let Err(ref e) = result {
        if (timeframe == "1Hour" || timeframe == "1h")
            && (e.contains("expected 1h") || e.contains("HTTP 422"))
        {
            return _fetch_yahoo_chart_bars_internal(client, symbol, "1Day").await;
        }
    }
    result
}

pub fn store_fallback_bars(
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
    if valid_count == 0 {
        return Err(format!("{source} returned no valid bars for {symbol} {tf}"));
    }
    let json = fallback_bars_to_cache_json(bars)?;
    let cache_key = format!("{source}:{symbol}:{tf}");
    cache
        .put_bars(&cache_key, &json)
        .map_err(|e| format!("{source} cache write failed for {symbol} {tf}: {e}"))?;
    Ok(valid_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yahoo_chart_lane_is_daily_and_up_only() {
        for tf in ["1Day", "1Week", "1Month", "D1", "W1", "MN1"] {
            assert!(
                yahoo_chart_supports_timeframe(tf),
                "{tf} should be supported"
            );
        }
        for tf in [
            "15Min", "30Min", "1Hour", "M15", "M30", "H1", "5Min", "1Min",
        ] {
            assert!(
                !yahoo_chart_supports_timeframe(tf),
                "{tf} should NOT be on the Yahoo lane"
            );
        }
    }

    #[test]
    fn yahoo_chart_request_symbol_uses_hyphen_for_class_shares() {
        assert_eq!(yahoo_chart_request_symbol("BH.A"), "BH-A");
        assert_eq!(yahoo_chart_request_symbol("brk.b"), "BRK-B");
        assert_eq!(yahoo_chart_request_symbol("BH.A.EQ"), "BH-A");
    }

    #[test]
    fn yahoo_chart_request_symbol_keeps_non_class_dot_symbols() {
        assert_eq!(yahoo_chart_request_symbol("DGAC.U"), "DGAC.U");
        assert_eq!(yahoo_chart_request_symbol("BIII.U"), "BIII.U");
    }

    #[test]
    fn yahoo_chart_high_timeframes_use_period_bounds_to_avoid_downsample() {
        assert_eq!(
            yahoo_interval_and_window("1Day"),
            Some(("1d", YahooChartWindow::PeriodFromUnixEpoch))
        );
        assert_eq!(
            yahoo_interval_and_window("D1"),
            Some(("1d", YahooChartWindow::PeriodFromUnixEpoch))
        );
        assert_eq!(
            yahoo_interval_and_window("1Week"),
            Some(("1wk", YahooChartWindow::PeriodFromUnixEpoch))
        );
        assert_eq!(yahoo_expected_granularity("1Day"), Some("1d"));
        assert_eq!(yahoo_expected_granularity("1Week"), Some("1wk"));
        assert_eq!(
            yahoo_interval_and_window("1Month"),
            Some(("1mo", YahooChartWindow::PeriodFromUnixEpoch))
        );
        assert_eq!(yahoo_expected_granularity("1Month"), Some("1mo"));
    }

    #[test]
    fn yahoo_chart_granularity_mismatch_is_provider_no_data() {
        assert!(yahoo_chart_provider_no_data_error(
            "Yahoo Chart returned 1wk bars for USSH 1Month; expected 1mo"
        ));
        assert!(yahoo_chart_provider_no_data_error(
            "Yahoo Chart missing quote arrays for ABC 1Day"
        ));
        assert!(!yahoo_chart_provider_no_data_error(
            "Yahoo Chart JSON parse failed for ABC 1Day: trailing characters"
        ));
    }
}
