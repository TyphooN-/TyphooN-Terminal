//! Kraken-internal equities (xStocks) types and JSON parsers.
//!
//! The equity endpoints under `iapi.kraken.com/api/internal` return values
//! mostly as JSON strings (e.g. `"123.45"` rather than `123.45`), so the
//! number/i64 parsers below accept both `Number` and `String` JSON nodes and
//! reject non-finite values. The `KrakenBroker` methods that consume these
//! types still live in `mod.rs` for the moment; the next split can pull them
//! over as a paired commit if the impl block gets carved up.

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
}
