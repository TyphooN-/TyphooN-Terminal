//! CryptoCompare deep crypto history — free, no API key, no geo-blocking.
//! Provides OHLCV data back to BTC genesis (2010) with 2000 bars per request.
//! Replaces Kraken OHLC as primary crypto backfill source.

use std::sync::atomic::{AtomicI64, Ordering};

/// Process-wide back-off clock. When CryptoCompare hits a rate limit
/// (HTTP 429 or in-body "rate limit"/"upgrade" message), this gets set
/// to `now + RATE_LIMIT_BACKOFF_SECS` and `fetch_ohlcv` short-circuits
/// until the clock passes. Callers should probe via
/// `rate_limited_for_secs()` BEFORE dispatching a call so they can
/// route to an alternate source (Kraken) instead of burning a
/// round-trip just to re-learn the back-off.
static RATE_LIMITED_UNTIL_SECS: AtomicI64 = AtomicI64::new(0);

/// How long to sit out after a rate-limit hit. CryptoCompare's free
/// tier resets on a ~minute cadence but the in-body rate-limit message
/// often means "you've consumed your hour budget", so a 10 min back-off
/// avoids ping-ponging while still recovering in the same session.
const RATE_LIMIT_BACKOFF_SECS: i64 = 10 * 60;

/// Remaining seconds in the CryptoCompare back-off window.
/// `None` = API is free to call; `Some(s)` = `s` seconds until retry.
/// Intended for callers that can fall back to another source — they
/// skip CryptoCompare entirely while in back-off.
pub fn rate_limited_for_secs() -> Option<i64> {
    let now = chrono::Utc::now().timestamp();
    let until = RATE_LIMITED_UNTIL_SECS.load(Ordering::Relaxed);
    if until > now { Some(until - now) } else { None }
}

fn mark_rate_limited() {
    let until = chrono::Utc::now().timestamp().saturating_add(RATE_LIMIT_BACKOFF_SECS);
    RATE_LIMITED_UNTIL_SECS.store(until, Ordering::Relaxed);
}

/// Map TyphooN timeframes to CryptoCompare endpoints.
fn endpoint_for_tf(tf: &str) -> Option<(&'static str, u32)> {
    match tf {
        "1Min" => Some(("histominute", 1)),
        "5Min" => None, // CryptoCompare doesn't have 5min; use 1min and aggregate
        "15Min" => None, // same
        "30Min" => None,
        "1Hour" => Some(("histohour", 1)),
        "4Hour" => None, // aggregate from hourly
        "1Day" => Some(("histoday", 1)),
        "1Week" => None, // aggregate from daily
        "1Month" => None, // aggregate from daily
        _ => None,
    }
}

/// Fetch OHLCV from CryptoCompare with backward pagination.
/// Returns ALL available history from `start_ms` to `end_ms`.
pub async fn fetch_ohlcv(
    client: &reqwest::Client,
    symbol: &str,
    timeframe: &str,
    start_ms: i64,
    end_ms: i64,
) -> Result<Vec<serde_json::Value>, String> {
    // Short-circuit during process-wide back-off. Returning immediately
    // (instead of calling the API and re-paying a 15 s sleep on each TF
    // × symbol combination) is the whole point of the back-off clock —
    // the caller is expected to route to Kraken while we're sitting out.
    if let Some(secs) = rate_limited_for_secs() {
        return Err(format!(
            "CryptoCompare in rate-limit back-off ({}s remaining) — caller should use Kraken",
            secs
        ));
    }

    // For timeframes without direct support, fetch base TF and aggregate
    let (endpoint, _multiplier) = match endpoint_for_tf(timeframe) {
        Some(e) => e,
        None => {
            // Aggregate: 4Hour from hourly, 1Week from daily, etc.
            match timeframe {
                "5Min" => {
                    let minutely = Box::pin(fetch_ohlcv(client, symbol, "1Min", start_ms, end_ms)).await?;
                    return Ok(aggregate_bars(&minutely, 5));
                }
                "15Min" => {
                    let minutely = Box::pin(fetch_ohlcv(client, symbol, "1Min", start_ms, end_ms)).await?;
                    return Ok(aggregate_bars(&minutely, 15));
                }
                "30Min" => {
                    let minutely = Box::pin(fetch_ohlcv(client, symbol, "1Min", start_ms, end_ms)).await?;
                    return Ok(aggregate_bars(&minutely, 30));
                }
                "4Hour" => {
                    let hourly = Box::pin(fetch_ohlcv(client, symbol, "1Hour", start_ms, end_ms)).await?;
                    return Ok(aggregate_bars(&hourly, 4));
                }
                "1Week" => {
                    let daily = Box::pin(fetch_ohlcv(client, symbol, "1Day", start_ms, end_ms)).await?;
                    return Ok(aggregate_bars(&daily, 7));
                }
                "1Month" => {
                    let daily = Box::pin(fetch_ohlcv(client, symbol, "1Day", start_ms, end_ms)).await?;
                    return Ok(aggregate_to_monthly(&daily));
                }
                _ => return Err(format!("Unsupported timeframe: {}", timeframe)),
            }
        }
    };

    // Normalize symbol: BTCUSD → BTC, SOL/USD → SOL
    let fsym = symbol.replace("/USD", "").replace("USD", "").to_uppercase();

    let mut all_bars = Vec::new();
    let mut to_ts = end_ms / 1000;
    let start_ts = start_ms / 1000;

    loop {
        let url = format!(
            "https://min-api.cryptocompare.com/data/v2/{}?fsym={}&tsym=USD&limit=2000&toTs={}",
            endpoint, fsym, to_ts
        );

        // HTTP 429 → arm the process-wide back-off clock and bail.
        // Previously this retried up to 3× with 10/20/30 s sleeps inside
        // the call, so N TFs × M symbols paid the penalty repeatedly and
        // the terminal appeared hung for minutes. Now the first 429
        // returns immediately; callers check `rate_limited_for_secs()`
        // up front on the next pass and route to Kraken instead.
        let resp = match client.get(&url)
            .timeout(std::time::Duration::from_secs(30))
            .send().await {
            Ok(r) => {
                if r.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    mark_rate_limited();
                    tracing::warn!(
                        "CryptoCompare HTTP 429 — entering {}s back-off; caller should fall back to Kraken",
                        RATE_LIMIT_BACKOFF_SECS
                    );
                    return Err("CryptoCompare rate-limited (HTTP 429)".into());
                }
                r
            }
            Err(e) => return Err(format!("CryptoCompare request failed: {e}")),
        };

        if !resp.status().is_success() {
            return Err(format!("CryptoCompare HTTP {}", resp.status()));
        }

        let body: serde_json::Value = resp.json().await
            .map_err(|e| format!("CryptoCompare JSON parse failed: {e}"))?;

        if body["Response"].as_str() != Some("Success") {
            let msg = body["Message"].as_str().unwrap_or("unknown");
            if msg.contains("rate limit") || msg.contains("upgrade") {
                // Arm the process-wide back-off clock, drop the 15 s
                // in-call sleep (callers will route to Kraken on the
                // next pass), and return what we have so far — if
                // we're mid-pagination we keep the bars already
                // collected, otherwise an empty Vec signals "fall
                // back to Kraken".
                mark_rate_limited();
                tracing::warn!(
                    "CryptoCompare rate limit in response body — entering {}s back-off; caller should fall back to Kraken (collected {} bars before the cap)",
                    RATE_LIMIT_BACKOFF_SECS, all_bars.len()
                );
                break;
            }
            return Err(format!("CryptoCompare error: {msg}"));
        }

        // (Success already verified above)

        let data = body["Data"]["Data"].as_array()
            .ok_or_else(|| "No data array in response".to_string())?;

        let mut page_bars = Vec::new();
        let mut earliest_ts = i64::MAX;
        for bar in data {
            let ts = bar["time"].as_i64().unwrap_or(0);
            if ts == 0 { continue; }
            let open = bar["open"].as_f64().unwrap_or(0.0);
            if open <= 0.0 { continue; } // skip zero-price bars (before trading started)

            let ts_ms = ts * 1000;
            if ts_ms < start_ms || ts_ms > end_ms { continue; }

            let dt = chrono::DateTime::from_timestamp(ts, 0).unwrap_or_default();
            page_bars.push(serde_json::json!({
                "timestamp": dt.to_rfc3339(),
                "open": open,
                "high": bar["high"].as_f64().unwrap_or(0.0),
                "low": bar["low"].as_f64().unwrap_or(0.0),
                "close": bar["close"].as_f64().unwrap_or(0.0),
                "volume": bar["volumefrom"].as_f64().unwrap_or(0.0),
            }));

            if ts < earliest_ts { earliest_ts = ts; }
        }

        let page_count = page_bars.len();
        all_bars.extend(page_bars);

        // Stop if: no bars, reached start, or didn't go further back
        if page_count == 0 || earliest_ts <= start_ts || earliest_ts >= to_ts {
            break;
        }
        to_ts = earliest_ts;

        // Rate limit: CryptoCompare free tier allows ~30 calls/min
        tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
    }

    // Sort by timestamp ascending and deduplicate
    all_bars.sort_by(|a, b| a["timestamp"].as_str().unwrap_or("").cmp(b["timestamp"].as_str().unwrap_or("")));
    all_bars.dedup_by(|a, b| a["timestamp"] == b["timestamp"]);

    Ok(all_bars)
}

/// Aggregate N bars into 1 (e.g., 4 hourly bars → 1 4-hour bar).
/// The last chunk may be incomplete (fewer than `period` bars); this is fine —
/// it produces a shorter-period bar for the most recent partial window.
fn aggregate_bars(bars: &[serde_json::Value], period: usize) -> Vec<serde_json::Value> {
    bars.chunks(period).filter_map(|chunk| {
        let first = &chunk[0];
        let mut high = f64::MIN;
        let mut low = f64::MAX;
        let mut vol = 0.0_f64;
        let mut valid = false;
        for b in chunk {
            let h = b["high"].as_f64()?;
            let l = b["low"].as_f64()?;
            let v = b["volume"].as_f64().unwrap_or(0.0);
            if h > high { high = h; }
            if l < low { low = l; }
            vol += v;
            valid = true;
        }
        if !valid { return None; }
        let last = chunk.last().unwrap_or(first);
        Some(serde_json::json!({
            "timestamp": first["timestamp"],
            "open": first["open"],
            "high": high,
            "low": low,
            "close": last["close"],
            "volume": vol,
        }))
    }).collect()
}

/// Aggregate daily bars into monthly OHLCV.
fn aggregate_to_monthly(daily: &[serde_json::Value]) -> Vec<serde_json::Value> {
    let mut monthly: std::collections::BTreeMap<String, (f64, f64, f64, f64, f64, String)> = std::collections::BTreeMap::new();
    for bar in daily {
        let ts = bar["timestamp"].as_str().unwrap_or("");
        if ts.len() < 7 { continue; }
        let o = bar["open"].as_f64().unwrap_or(0.0);
        let h = bar["high"].as_f64().unwrap_or(0.0);
        let l = bar["low"].as_f64().unwrap_or(0.0);
        let c = bar["close"].as_f64().unwrap_or(0.0);
        let v = bar["volume"].as_f64().unwrap_or(0.0);
        // Skip bars with invalid prices
        if o <= 0.0 || h <= 0.0 || l <= 0.0 || c <= 0.0 || h < l { continue; }
        let month_key = ts[..7].to_string();
        let entry = monthly.entry(month_key).or_insert((o, h, l, c, 0.0, ts.to_string()));
        if h > entry.1 { entry.1 = h; }
        if l < entry.2 { entry.2 = l; }
        entry.3 = c;
        entry.4 += v;
    }
    monthly.into_iter().map(|(_, (o, h, l, c, v, ts))| {
        serde_json::json!({"timestamp": ts, "open": o, "high": h, "low": l, "close": c, "volume": v})
    }).collect()
}

/// Check if a symbol is supported (all crypto pairs with USD are supported).
pub fn is_supported(symbol: &str) -> bool {
    let s = symbol.to_uppercase().replace('/', "");
    s.ends_with("USD") && s.len() >= 6
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── endpoint_for_tf ────────────────────────────────────

    #[test]
    fn endpoint_for_supported_timeframes() {
        assert_eq!(endpoint_for_tf("1Min"), Some(("histominute", 1)));
        assert_eq!(endpoint_for_tf("1Hour"), Some(("histohour", 1)));
        assert_eq!(endpoint_for_tf("1Day"), Some(("histoday", 1)));
    }

    #[test]
    fn endpoint_for_aggregate_timeframes() {
        // These require aggregation and return None
        assert_eq!(endpoint_for_tf("5Min"), None);
        assert_eq!(endpoint_for_tf("15Min"), None);
        assert_eq!(endpoint_for_tf("30Min"), None);
        assert_eq!(endpoint_for_tf("4Hour"), None);
        assert_eq!(endpoint_for_tf("1Week"), None);
        assert_eq!(endpoint_for_tf("1Month"), None);
    }

    #[test]
    fn endpoint_for_unknown_timeframe() {
        assert_eq!(endpoint_for_tf("2Hour"), None);
        assert_eq!(endpoint_for_tf(""), None);
        assert_eq!(endpoint_for_tf("garbage"), None);
    }

    // ── is_supported ───────────────────────────────────────

    #[test]
    fn supported_symbols() {
        assert!(is_supported("BTCUSD"));
        assert!(is_supported("ETHUSD"));
        assert!(is_supported("SOLUSD"));
        assert!(is_supported("BTC/USD"));
        assert!(is_supported("btcusd"));
    }

    #[test]
    fn unsupported_symbols() {
        // Too short after normalization
        assert!(!is_supported("USD"));
        assert!(!is_supported("XAUSD")); // only 5 chars
        // Wrong quote currency
        assert!(!is_supported("BTCEUR"));
        // Note: EURUSD (6 chars, ends with USD) passes — the function
        // only checks suffix + length, not a crypto whitelist
    }

    #[test]
    fn supported_edge_cases() {
        assert!(!is_supported(""));
        // "EURUSD" has 6 chars and ends with USD, so it is_supported returns true
        // (the function only checks suffix + length, not a whitelist)
        assert!(is_supported("EURUSD"));
    }

    // ── aggregate_bars ─────────────────────────────────────

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
    fn aggregate_bars_simple() {
        let bars = vec![
            make_bar("2024-01-01T00:00:00Z", 100.0, 110.0, 90.0, 105.0, 1000.0),
            make_bar("2024-01-01T01:00:00Z", 105.0, 120.0, 95.0, 115.0, 2000.0),
            make_bar("2024-01-01T02:00:00Z", 115.0, 130.0, 100.0, 125.0, 1500.0),
            make_bar("2024-01-01T03:00:00Z", 125.0, 140.0, 110.0, 135.0, 500.0),
        ];
        let result = aggregate_bars(&bars, 2);
        assert_eq!(result.len(), 2);

        // First aggregated bar: bars[0..2]
        let b0 = &result[0];
        assert_eq!(b0["open"].as_f64().unwrap(), 100.0);
        assert_eq!(b0["high"].as_f64().unwrap(), 120.0); // max of 110, 120
        assert_eq!(b0["low"].as_f64().unwrap(), 90.0);   // min of 90, 95
        assert_eq!(b0["close"].as_f64().unwrap(), 115.0); // last bar close
        assert!((b0["volume"].as_f64().unwrap() - 3000.0).abs() < 1e-10);

        // Second aggregated bar: bars[2..4]
        let b1 = &result[1];
        assert_eq!(b1["open"].as_f64().unwrap(), 115.0);
        assert_eq!(b1["high"].as_f64().unwrap(), 140.0);
        assert_eq!(b1["low"].as_f64().unwrap(), 100.0);
        assert_eq!(b1["close"].as_f64().unwrap(), 135.0);
        assert!((b1["volume"].as_f64().unwrap() - 2000.0).abs() < 1e-10);
    }

    #[test]
    fn aggregate_bars_incomplete_last_chunk() {
        // 3 bars aggregated by 2 → 2 output bars (last has 1 bar)
        let bars = vec![
            make_bar("2024-01-01T00:00:00Z", 100.0, 110.0, 90.0, 105.0, 1000.0),
            make_bar("2024-01-01T01:00:00Z", 105.0, 120.0, 95.0, 115.0, 2000.0),
            make_bar("2024-01-01T02:00:00Z", 115.0, 130.0, 100.0, 125.0, 1500.0),
        ];
        let result = aggregate_bars(&bars, 2);
        assert_eq!(result.len(), 2);
        // Last bar is a single-bar "aggregation"
        let last = &result[1];
        assert_eq!(last["open"].as_f64().unwrap(), 115.0);
        assert_eq!(last["close"].as_f64().unwrap(), 125.0);
    }

    #[test]
    fn aggregate_bars_empty_input() {
        let bars: Vec<serde_json::Value> = vec![];
        let result = aggregate_bars(&bars, 4);
        assert!(result.is_empty());
    }

    #[test]
    fn aggregate_bars_single_bar() {
        let bars = vec![
            make_bar("2024-01-01T00:00:00Z", 100.0, 110.0, 90.0, 105.0, 1000.0),
        ];
        let result = aggregate_bars(&bars, 4);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["open"].as_f64().unwrap(), 100.0);
    }

    #[test]
    fn aggregate_bars_period_one() {
        // period=1 should return same number of bars
        let bars = vec![
            make_bar("2024-01-01T00:00:00Z", 100.0, 110.0, 90.0, 105.0, 500.0),
            make_bar("2024-01-01T01:00:00Z", 105.0, 120.0, 95.0, 115.0, 600.0),
        ];
        let result = aggregate_bars(&bars, 1);
        assert_eq!(result.len(), 2);
    }

    // ── aggregate_to_monthly ───────────────────────────────

    #[test]
    fn aggregate_to_monthly_basic() {
        let daily = vec![
            make_bar("2024-01-02T00:00:00Z", 100.0, 110.0, 90.0, 105.0, 1000.0),
            make_bar("2024-01-15T00:00:00Z", 105.0, 150.0, 85.0, 140.0, 2000.0),
            make_bar("2024-01-31T00:00:00Z", 140.0, 145.0, 130.0, 135.0, 1500.0),
            make_bar("2024-02-01T00:00:00Z", 135.0, 160.0, 120.0, 155.0, 3000.0),
        ];
        let result = aggregate_to_monthly(&daily);
        assert_eq!(result.len(), 2);

        // January
        let jan = &result[0];
        assert_eq!(jan["open"].as_f64().unwrap(), 100.0);
        assert_eq!(jan["high"].as_f64().unwrap(), 150.0);
        assert_eq!(jan["low"].as_f64().unwrap(), 85.0);
        assert_eq!(jan["close"].as_f64().unwrap(), 135.0);
        assert!((jan["volume"].as_f64().unwrap() - 4500.0).abs() < 1e-10);

        // February
        let feb = &result[1];
        assert_eq!(feb["open"].as_f64().unwrap(), 135.0);
        assert_eq!(feb["close"].as_f64().unwrap(), 155.0);
    }

    #[test]
    fn aggregate_to_monthly_empty() {
        let result = aggregate_to_monthly(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn aggregate_to_monthly_single_day() {
        let daily = vec![
            make_bar("2024-06-15T00:00:00Z", 50.0, 55.0, 45.0, 52.0, 100.0),
        ];
        let result = aggregate_to_monthly(&daily);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["open"].as_f64().unwrap(), 50.0);
    }

    #[test]
    fn aggregate_to_monthly_short_timestamp_skipped() {
        // Timestamps shorter than 7 chars should be skipped
        let daily = vec![
            json!({"timestamp": "short", "open": 1.0, "high": 2.0, "low": 0.5, "close": 1.5, "volume": 100.0}),
        ];
        let result = aggregate_to_monthly(&daily);
        assert!(result.is_empty());
    }
}
