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
        "1Month" => Some(("1mo", YahooChartWindow::Range("max"))),
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

pub(super) async fn fetch_yahoo_chart_bars(
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn yahoo_chart_daily_and_weekly_use_period_bounds_to_avoid_monthly_downsample() {
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
            Some(("1mo", YahooChartWindow::Range("max")))
        );
    }
}
