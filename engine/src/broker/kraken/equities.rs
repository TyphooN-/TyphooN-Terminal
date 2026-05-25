//! Kraken-internal equities (xStocks) types and JSON parsers.
//!
//! The equity endpoints under `iapi.kraken.com/api/internal` return values
//! mostly as JSON strings (e.g. `"123.45"` rather than `123.45`), so the
//! number/i64 parsers below accept both `Number` and `String` JSON nodes and
//! reject non-finite values. The `KrakenBroker` methods that consume these
//! types still live in `mod.rs` for the moment; the next split can pull them
//! over as a paired commit if the impl block gets carved up.
//!
//! Rate-limiting moved to `iapi_limiter`: a token-bucket plus escalating
//! backoff that all iapi callers share. The legacy `iapi_rate_limited_for_secs`
//! / `arm_iapi_backoff` entry points are thin compatibility shims over the
//! limiter so external callers (broker thread, sync scheduler) keep working
//! without churn.

use super::iapi_limiter::iapi_limiter;

/// Remaining back-off seconds for the iapi host, or `None` if it is free to
/// call. Callers should probe BEFORE dispatching so they skip the round-trip
/// (and the log spam) while the ban is in effect.
pub fn iapi_rate_limited_for_secs() -> Option<i64> {
    iapi_limiter().remaining_backoff_secs()
}

/// Arm the iapi back-off. Uses the longer Cloudflare window if the body looks
/// like a 1015, otherwise the default; the limiter also escalates the window
/// on repeated arms inside `escalation_reset_after`. Returns the chosen
/// back-off (post-cap, post-CAS) so the caller can log it once at the entry
/// edge. Sync wrapper around the async limiter — safe to call inside any
/// tokio runtime via `block_in_place`; for in-context async callers prefer
/// `iapi_limiter().record_rate_limited(body).await` directly.
pub(super) async fn arm_iapi_backoff(body: &str) -> i64 {
    iapi_limiter().record_rate_limited(body).await
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

    // Backoff/limiter coverage lives in `iapi_limiter::tests` — those tests
    // exercise the bucket and escalation state in isolated `IapiLimiter`
    // instances, which is safer than touching the process-wide singleton
    // from here in parallel runs.
}
