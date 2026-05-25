//! Kraken-internal equities (xStocks) types and JSON parsers.
//!
//! The equity endpoints under `iapi.kraken.com/api/internal` return values
//! mostly as JSON strings (e.g. `"123.45"` rather than `123.45`), so the
//! number/i64 parsers below accept both `Number` and `String` JSON nodes and
//! reject non-finite values. The `KrakenBroker` methods that consume these
//! types still live in `mod.rs` for the moment; the next split can pull them
//! over as a paired commit if the impl block gets carved up.

use std::sync::atomic::{AtomicI64, Ordering};

/// Process-wide back-off clock for `iapi.kraken.com`. Cloudflare in front of
/// the internal API rate-limits by client IP (error 1015) and by endpoint
/// budget (HTTP 429). Once any iapi call gets rejected, hammering the OTHER
/// endpoints on the same host just extends the ban — so the back-off here is
/// shared across `get_equity_ticker`, `get_equity_history`, and
/// `get_equity_markets`.
static IAPI_RATE_LIMITED_UNTIL_SECS: AtomicI64 = AtomicI64::new(0);

/// Back-off applied to a plain HTTP 429 with no Cloudflare body marker.
const IAPI_DEFAULT_BACKOFF_SECS: i64 = 90;

/// Back-off applied when the response body carries Cloudflare's "error code
/// 1015" — that signals an IP-level rate-limit which typically takes 5-15
/// minutes to lift, far longer than the per-endpoint 45 s used previously.
const IAPI_CLOUDFLARE_BACKOFF_SECS: i64 = 600;

/// Remaining back-off seconds for the iapi host, or `None` if it is free to
/// call. Callers should probe BEFORE dispatching so they skip the round-trip
/// (and the log spam) while the ban is in effect.
pub fn iapi_rate_limited_for_secs() -> Option<i64> {
    let now = chrono::Utc::now().timestamp();
    let until = IAPI_RATE_LIMITED_UNTIL_SECS.load(Ordering::Relaxed);
    if until > now { Some(until - now) } else { None }
}

/// Arm the iapi back-off. Uses the longer Cloudflare window if the body
/// looks like a 1015, otherwise the default. Multiple in-flight callers
/// racing to arm CAS toward the later expiry so a stale shorter window can't
/// shrink an active longer one. Returns the chosen back-off so the caller
/// can log it once at the entry edge.
pub(super) fn arm_iapi_backoff(body: &str) -> i64 {
    let secs = if body.contains("1015") {
        IAPI_CLOUDFLARE_BACKOFF_SECS
    } else {
        IAPI_DEFAULT_BACKOFF_SECS
    };
    let until = chrono::Utc::now().timestamp().saturating_add(secs);
    let mut current = IAPI_RATE_LIMITED_UNTIL_SECS.load(Ordering::Relaxed);
    while until > current {
        match IAPI_RATE_LIMITED_UNTIL_SECS.compare_exchange_weak(
            current,
            until,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => break,
            Err(actual) => current = actual,
        }
    }
    secs
}

/// Common error prefix the broker thread / app can match on to recognise an
/// iapi rate-limit so the message is treated as routine status instead of an
/// error popped into the user log on every retry.
pub const IAPI_RATE_LIMITED_ERR_PREFIX: &str = "Kraken iapi rate-limited";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KrakenEquityTicker {
    pub symbol: String,
    pub bid: f64,
    pub ask: f64,
    pub price: f64,
    pub volume: f64,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub previous_close: Option<f64>,
    pub time_ms: i64,
    pub delayed: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KrakenEquityBar {
    pub time_ms: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KrakenEquityMarket {
    pub symbol: String,
    pub name: Option<String>,
    pub tradable: bool,
    pub status: Option<String>,
    pub instrument_status: Option<String>,
}

pub(super) fn parse_json_number(value: &serde_json::Value) -> Option<f64> {
    match value {
        serde_json::Value::String(s) => s.parse::<f64>().ok(),
        serde_json::Value::Number(n) => n.as_f64(),
        _ => None,
    }
    .filter(|v| v.is_finite())
}

pub(super) fn parse_json_i64(value: &serde_json::Value) -> Option<i64> {
    match value {
        serde_json::Value::String(s) => s.parse::<i64>().ok(),
        serde_json::Value::Number(n) => n.as_i64().or_else(|| n.as_u64().map(|v| v as i64)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_json_number_accepts_string_and_number() {
        assert_eq!(parse_json_number(&json!("1.5")), Some(1.5));
        assert_eq!(parse_json_number(&json!(2.25)), Some(2.25));
        assert_eq!(parse_json_number(&json!(42)), Some(42.0));
    }

    #[test]
    fn parse_json_number_rejects_non_finite_and_garbage() {
        assert!(parse_json_number(&json!("not-a-number")).is_none());
        assert!(parse_json_number(&json!(null)).is_none());
        assert!(parse_json_number(&json!(true)).is_none());
        // NaN / infinity arrive only via String form (serde_json refuses
        // them as Number literals), so the finite filter still bites:
        assert!(parse_json_number(&json!("NaN")).is_none());
    }

    #[test]
    fn parse_json_i64_accepts_string_and_number() {
        assert_eq!(parse_json_i64(&json!("1700000000")), Some(1_700_000_000));
        assert_eq!(parse_json_i64(&json!(1700000000)), Some(1_700_000_000));
    }

    #[test]
    fn parse_json_i64_handles_u64_in_range_via_signed_cast() {
        // Kraken sometimes emits epoch-millis as a u64; the parser falls
        // back to as_u64() and casts.
        assert_eq!(parse_json_i64(&json!(1u64 << 60)), Some(1i64 << 60));
    }

    #[test]
    fn parse_json_i64_rejects_garbage() {
        assert!(parse_json_i64(&json!(null)).is_none());
        assert!(parse_json_i64(&json!("not-a-number")).is_none());
        assert!(parse_json_i64(&json!(true)).is_none());
    }

    /// All iapi back-off cases share the process-wide static, so they run
    /// in one test to avoid the parallel-runner race that splitting them
    /// would introduce.
    #[test]
    fn iapi_backoff_state_machine() {
        // Case 1: plain HTTP 429 body → default window.
        IAPI_RATE_LIMITED_UNTIL_SECS.store(0, Ordering::Relaxed);
        let default_secs = arm_iapi_backoff("Too Many Requests");
        assert_eq!(default_secs, IAPI_DEFAULT_BACKOFF_SECS);
        let default_remaining =
            iapi_rate_limited_for_secs().expect("default back-off armed");
        assert!(default_remaining >= IAPI_DEFAULT_BACKOFF_SECS - 1);

        // Case 2: Cloudflare 1015 body extends the window to the longer one.
        let cf_secs = arm_iapi_backoff("<html>error code: 1015</html>");
        assert_eq!(cf_secs, IAPI_CLOUDFLARE_BACKOFF_SECS);
        let cf_remaining =
            iapi_rate_limited_for_secs().expect("Cloudflare back-off armed");
        assert!(cf_remaining >= IAPI_CLOUDFLARE_BACKOFF_SECS - 1);

        // Case 3: a later plain 429 must not shrink the active Cloudflare
        // window — arm_iapi_backoff returns the *chosen* duration for the
        // body it saw, but the global expiry must stay at the longer one.
        let _ = arm_iapi_backoff("Too Many Requests");
        let still_long =
            iapi_rate_limited_for_secs().expect("window still armed");
        assert!(
            still_long >= cf_remaining - 1,
            "shorter plain-429 must not shrink the live Cloudflare window"
        );

        // Reset so other tests (and re-runs of this one) start clean.
        IAPI_RATE_LIMITED_UNTIL_SECS.store(0, Ordering::Relaxed);
        assert!(iapi_rate_limited_for_secs().is_none());
    }
}
