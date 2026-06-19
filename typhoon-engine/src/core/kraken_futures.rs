//! Kraken Futures public market data.
//!
//! Public instrument discovery and chart candles do not require an API key.
//! Private account/trading endpoints are intentionally separate from this
//! module.

const INSTRUMENTS_URL: &str = "https://futures.kraken.com/derivatives/api/v3/instruments";
const CHARTS_BASE: &str = "https://futures.kraken.com/api/charts/v1";

pub fn normalize_futures_symbol(symbol: &str) -> String {
    let mut raw = symbol.trim();
    if let Some(rest) = raw.strip_prefix("kraken-futures:") {
        raw = rest;
    }
    let parts: Vec<&str> = raw.split(':').collect();
    let without_tf = match parts.as_slice() {
        [sym, tf] if to_chart_resolution(tf).is_some() || tf.eq_ignore_ascii_case("1Month") => *sym,
        [_, sym, tf] if to_chart_resolution(tf).is_some() || tf.eq_ignore_ascii_case("1Month") => {
            *sym
        }
        _ => raw,
    };
    without_tf.trim().replace('/', "").to_ascii_uppercase()
}

pub fn is_futures_symbol(symbol: &str) -> bool {
    let symbol = normalize_futures_symbol(symbol);
    matches!(symbol.split('_').next(), Some("PI" | "PF" | "FI" | "FF"))
}

fn to_chart_resolution(tf: &str) -> Option<&'static str> {
    match tf {
        "1Min" => Some("1m"),
        "5Min" => Some("5m"),
        "15Min" => Some("15m"),
        "30Min" => Some("30m"),
        "1Hour" => Some("1h"),
        "4Hour" => Some("4h"),
        "1Day" => Some("1d"),
        "1Week" => Some("1w"),
        _ => None,
    }
}

fn timeframe_period_ms(tf: &str) -> Option<i64> {
    match tf {
        "1Min" => Some(60_000),
        "5Min" => Some(5 * 60_000),
        "15Min" => Some(15 * 60_000),
        "30Min" => Some(30 * 60_000),
        "1Hour" => Some(60 * 60_000),
        "4Hour" => Some(4 * 60 * 60_000),
        "1Day" => Some(24 * 60 * 60_000),
        "1Week" => Some(7 * 24 * 60 * 60_000),
        _ => None,
    }
}

pub async fn discover_instruments(client: &reqwest::Client) -> Result<Vec<String>, String> {
    let resp = client
        .get(INSTRUMENTS_URL)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("Kraken futures instruments request failed: {e}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Kraken futures instruments error {status}: {body}"));
    }
    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Kraken futures instruments JSON parse failed: {e}"))?;
    parse_instruments(&body)
}

fn parse_instruments(body: &serde_json::Value) -> Result<Vec<String>, String> {
    if let Some(result) = body.get("result").and_then(|v| v.as_str()) {
        if !result.eq_ignore_ascii_case("success") {
            return Err(format!("Kraken futures instruments result: {result}"));
        }
    }
    let Some(instruments) = body.get("instruments").and_then(|v| v.as_array()) else {
        return Err("Kraken futures instruments response missing instruments array".into());
    };

    let mut out = Vec::with_capacity(instruments.len());
    for instrument in instruments {
        if instrument
            .get("tradeable")
            .and_then(|v| v.as_bool())
            .is_some_and(|tradeable| !tradeable)
        {
            continue;
        }
        if instrument
            .get("status")
            .and_then(|v| v.as_str())
            .is_some_and(|status| {
                matches!(
                    status.to_ascii_lowercase().as_str(),
                    "closed" | "disabled" | "expired" | "settled" | "delisted"
                )
            })
        {
            continue;
        }
        let Some(symbol) = instrument.get("symbol").and_then(|v| v.as_str()) else {
            continue;
        };
        let symbol = normalize_futures_symbol(symbol);
        if !symbol.is_empty() && is_futures_symbol(&symbol) {
            out.push(symbol);
        }
    }
    out.sort();
    out.dedup();
    Ok(out)
}

pub async fn fetch_candles(
    client: &reqwest::Client,
    symbol: &str,
    timeframe: &str,
    start_ms: i64,
    end_ms: i64,
) -> Result<Vec<serde_json::Value>, String> {
    if timeframe == "1Month" {
        let daily = Box::pin(fetch_candles(client, symbol, "1Day", start_ms, end_ms)).await?;
        return Ok(aggregate_to_monthly(&daily));
    }

    let symbol = normalize_futures_symbol(symbol);
    if !is_futures_symbol(&symbol) {
        return Err(format!("Unsupported Kraken futures symbol: {symbol}"));
    }
    let resolution = to_chart_resolution(timeframe)
        .ok_or_else(|| format!("Unsupported timeframe for Kraken futures: {timeframe}"))?;
    let period_ms = timeframe_period_ms(timeframe)
        .ok_or_else(|| format!("Unsupported timeframe for Kraken futures: {timeframe}"))?;
    let end_ms = if end_ms > 0 {
        end_ms
    } else {
        chrono::Utc::now().timestamp_millis()
    };
    let mut cursor_ms = if start_ms > 0 {
        start_ms
    } else {
        chrono::NaiveDate::from_ymd_opt(2018, 1, 1)
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .map(|ndt| ndt.and_utc().timestamp_millis())
            .unwrap_or(0)
    }
    .max(0);

    let chunk_ms = period_ms.saturating_mul(1_500).max(period_ms);
    let url = format!("{CHARTS_BASE}/trade/{symbol}/{resolution}");
    let mut all_bars = Vec::new();
    let mut guard = 0usize;
    while cursor_ms <= end_ms && guard < 10_000 {
        guard += 1;
        let chunk_end_ms = cursor_ms.saturating_add(chunk_ms).min(end_ms);
        let from = cursor_ms / 1000;
        let to = chunk_end_ms / 1000;
        let resp = client
            .get(&url)
            .query(&[("from", from), ("to", to)])
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| format!("Kraken futures candles request failed: {e}"))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Kraken futures candles error {status}: {body}"));
        }
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Kraken futures candles JSON parse failed: {e}"))?;
        all_bars.extend(parse_candles(&body, start_ms.max(0), end_ms));
        if chunk_end_ms >= end_ms {
            break;
        }
        cursor_ms = chunk_end_ms.saturating_add(period_ms);
    }
    if guard >= 10_000 {
        return Err("Kraken futures candle pagination guard reached".into());
    }

    all_bars.sort_by(|a, b| {
        a["timestamp"]
            .as_str()
            .unwrap_or("")
            .cmp(b["timestamp"].as_str().unwrap_or(""))
    });
    all_bars.dedup_by(|a, b| a["timestamp"] == b["timestamp"]);
    Ok(all_bars)
}

fn parse_f64(value: &serde_json::Value) -> Option<f64> {
    let parsed = match value {
        serde_json::Value::Number(n) => n.as_f64(),
        serde_json::Value::String(s) => s.parse::<f64>().ok(),
        _ => None,
    }?;
    parsed.is_finite().then_some(parsed)
}

fn parse_ts_ms(value: &serde_json::Value) -> Option<i64> {
    match value {
        serde_json::Value::Number(n) => {
            let raw = n.as_i64().or_else(|| n.as_f64().map(|v| v as i64))?;
            Some(if raw > 10_000_000_000 {
                raw
            } else {
                raw.saturating_mul(1000)
            })
        }
        serde_json::Value::String(s) => {
            if let Ok(v) = s.parse::<f64>() {
                let raw = v as i64;
                return Some(if raw > 10_000_000_000 {
                    raw
                } else {
                    raw.saturating_mul(1000)
                });
            }
            chrono::DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|dt| dt.timestamp_millis())
        }
        _ => None,
    }
}

fn object_field<'a>(
    obj: &'a serde_json::Map<String, serde_json::Value>,
    names: &[&str],
) -> Option<&'a serde_json::Value> {
    names.iter().find_map(|name| obj.get(*name))
}

fn parse_candle_value(
    candle: &serde_json::Value,
    start_ms: i64,
    end_ms: i64,
) -> Option<serde_json::Value> {
    let (ts_ms, open, high, low, close, volume) = if let Some(obj) = candle.as_object() {
        let ts_ms = object_field(obj, &["time", "timestamp", "t", "date"]).and_then(parse_ts_ms)?;
        let open = object_field(obj, &["open", "o"]).and_then(parse_f64)?;
        let high = object_field(obj, &["high", "h"]).and_then(parse_f64)?;
        let low = object_field(obj, &["low", "l"]).and_then(parse_f64)?;
        let close = object_field(obj, &["close", "c"]).and_then(parse_f64)?;
        let volume = object_field(obj, &["volume", "v"])
            .and_then(parse_f64)
            .unwrap_or(0.0);
        (ts_ms, open, high, low, close, volume)
    } else if let Some(arr) = candle.as_array() {
        if arr.len() < 6 {
            return None;
        }
        (
            parse_ts_ms(&arr[0])?,
            parse_f64(&arr[1])?,
            parse_f64(&arr[2])?,
            parse_f64(&arr[3])?,
            parse_f64(&arr[4])?,
            parse_f64(&arr[5]).unwrap_or(0.0),
        )
    } else {
        return None;
    };

    if ts_ms < start_ms || ts_ms > end_ms {
        return None;
    }
    if !(open > 0.0 && high > 0.0 && low > 0.0 && close > 0.0 && high >= low) {
        return None;
    }
    let dt = chrono::DateTime::from_timestamp_millis(ts_ms)?;
    Some(serde_json::json!({
        "timestamp": dt.to_rfc3339(),
        "open": open,
        "high": high,
        "low": low,
        "close": close,
        "volume": volume.max(0.0),
    }))
}

fn parse_candles(body: &serde_json::Value, start_ms: i64, end_ms: i64) -> Vec<serde_json::Value> {
    let mut out = Vec::new();
    let candidates = [
        body.get("candles"),
        body.get("data"),
        body.get("result").and_then(|v| v.get("candles")),
        body.get("result").and_then(|v| v.get("data")),
        Some(body),
    ];
    for candidate in candidates.into_iter().flatten() {
        let Some(arr) = candidate.as_array() else {
            continue;
        };
        for candle in arr {
            if let Some(bar) = parse_candle_value(candle, start_ms, end_ms) {
                out.push(bar);
            }
        }
        if !out.is_empty() {
            break;
        }
    }
    out
}

fn aggregate_to_monthly(daily: &[serde_json::Value]) -> Vec<serde_json::Value> {
    let mut monthly: std::collections::BTreeMap<String, (f64, f64, f64, f64, f64, String)> =
        std::collections::BTreeMap::new();
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
        if o <= 0.0 || h <= 0.0 || l <= 0.0 || c <= 0.0 || h < l {
            continue;
        }
        let month_key = ts[..7].to_string();
        let entry = monthly
            .entry(month_key)
            .or_insert((o, h, l, c, 0.0, ts.to_string()));
        entry.1 = entry.1.max(h);
        entry.2 = entry.2.min(l);
        entry.3 = c;
        entry.4 += v;
    }
    monthly
        .into_values()
        .map(|(open, high, low, close, volume, timestamp)| {
            serde_json::json!({
                "timestamp": timestamp,
                "open": open,
                "high": high,
                "low": low,
                "close": close,
                "volume": volume,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn resolution_mapping_matches_kraken_futures_chart_api() {
        assert_eq!(to_chart_resolution("1Min"), Some("1m"));
        assert_eq!(to_chart_resolution("5Min"), Some("5m"));
        assert_eq!(to_chart_resolution("15Min"), Some("15m"));
        assert_eq!(to_chart_resolution("30Min"), Some("30m"));
        assert_eq!(to_chart_resolution("1Hour"), Some("1h"));
        assert_eq!(to_chart_resolution("4Hour"), Some("4h"));
        assert_eq!(to_chart_resolution("1Day"), Some("1d"));
        assert_eq!(to_chart_resolution("1Week"), Some("1w"));
        assert_eq!(to_chart_resolution("1Month"), None);
    }

    #[test]
    fn instrument_parser_keeps_tradeable_futures() {
        let body = json!({
            "result": "success",
            "instruments": [
                {"symbol": "PI_XBTUSD", "tradeable": true},
                {"symbol": "PF_ETHUSD", "tradeable": true},
                {"symbol": "FI_XBTUSD_260529", "tradeable": true},
                {"symbol": "PF_OLDUSD", "tradeable": false},
                {"symbol": "SPOT_XBTUSD", "tradeable": true}
            ]
        });
        let instruments = parse_instruments(&body).unwrap();
        assert_eq!(
            instruments,
            vec!["FI_XBTUSD_260529", "PF_ETHUSD", "PI_XBTUSD"]
        );
    }

    #[test]
    fn candle_parser_accepts_object_shape() {
        let body = json!({
            "candles": [
                {"time": 1777670040000_i64, "open": "77934", "high": "77934", "low": "77916", "close": "77929", "volume": "0.0057"},
                {"time": 1777670100000_i64, "open": "77929", "high": "77939", "low": "77914", "close": "77915", "volume": "0.0564"}
            ],
            "more_candles": false
        });
        let bars = parse_candles(&body, 1777670000000, 1777670200000);
        assert_eq!(bars.len(), 2);
        assert_eq!(bars[0]["open"].as_f64().unwrap(), 77934.0);
        assert_eq!(bars[1]["volume"].as_f64().unwrap(), 0.0564);
    }

    #[test]
    fn candle_parser_accepts_array_shape_and_filters_range() {
        let body = json!([
            [1777670040, "10", "12", "9", "11", "1.5"],
            [1777670100, "11", "13", "10", "12", "2.5"],
            [1777680000, "12", "14", "11", "13", "3.5"]
        ]);
        let bars = parse_candles(&body, 1777670040000, 1777670100000);
        assert_eq!(bars.len(), 2);
        assert_eq!(bars[0]["close"].as_f64().unwrap(), 11.0);
        assert_eq!(bars[1]["high"].as_f64().unwrap(), 13.0);
    }

    #[test]
    fn monthly_aggregation_rolls_daily_bars() {
        let daily = vec![
            json!({"timestamp":"2026-05-01T00:00:00+00:00","open":10.0,"high":12.0,"low":9.0,"close":11.0,"volume":1.0}),
            json!({"timestamp":"2026-05-02T00:00:00+00:00","open":11.0,"high":15.0,"low":10.0,"close":14.0,"volume":2.0}),
            json!({"timestamp":"2026-06-01T00:00:00+00:00","open":14.0,"high":16.0,"low":13.0,"close":15.0,"volume":3.0}),
        ];
        let monthly = aggregate_to_monthly(&daily);
        assert_eq!(monthly.len(), 2);
        assert_eq!(monthly[0]["open"].as_f64().unwrap(), 10.0);
        assert_eq!(monthly[0]["high"].as_f64().unwrap(), 15.0);
        assert_eq!(monthly[0]["close"].as_f64().unwrap(), 14.0);
        assert_eq!(monthly[0]["volume"].as_f64().unwrap(), 3.0);
    }
}
