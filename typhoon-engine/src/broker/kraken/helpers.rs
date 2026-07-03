//! Small helpers used across the Kraken REST integration.
//!
//! Two groups of functions:
//!  - HTTP / form encoding: shared `Client` (TCP-pool reuse) plus the
//!    application/x-www-form-urlencoded encoder Kraken's REST endpoints
//!    require (POST body, not query string).
//!  - Order-type recognition: the matrix of supported types, which require
//!    a primary price, which require a secondary price, and the form-name
//!    normalisation (`stop_loss_limit` → `stop-loss-limit`). The matchers
//!    are intentionally exhaustive `match` arms so adding a new Kraken
//!    order type forces the compiler to remind us about every check site.

use reqwest::Client;
use std::sync::OnceLock;
use std::time::Duration;

/// Shared HTTP client for Kraken API requests (reuses TCP connections).
pub(super) fn kraken_client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        Client::builder()
            .timeout(Duration::from_secs(15))
            .pool_max_idle_per_host(2)
            .build()
            .unwrap_or_else(|_| Client::new())
    })
}

pub(super) fn format_f64_param(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        value.to_string()
    }
}

pub(super) fn push_opt_param(params: &mut Vec<(String, String)>, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        if !value.is_empty() {
            params.push((key.to_string(), value.to_string()));
        }
    }
}

pub(super) fn normalize_kraken_order_type(order_type: &str) -> String {
    order_type.trim().replace('_', "-").to_ascii_lowercase()
}

pub(super) fn is_supported_kraken_order_type(order_type: &str) -> bool {
    matches!(
        order_type,
        "market"
            | "limit"
            | "iceberg"
            | "stop-loss"
            | "stop-loss-limit"
            | "take-profit"
            | "take-profit-limit"
            | "trailing-stop"
            | "trailing-stop-limit"
            | "settle-position"
    )
}

pub(super) fn is_supported_kraken_close_order_type(order_type: &str) -> bool {
    matches!(
        order_type,
        "limit"
            | "stop-loss"
            | "stop-loss-limit"
            | "take-profit"
            | "take-profit-limit"
            | "trailing-stop"
            | "trailing-stop-limit"
    )
}

pub(super) fn requires_primary_price(order_type: &str) -> bool {
    matches!(
        order_type,
        "limit"
            | "iceberg"
            | "stop-loss"
            | "stop-loss-limit"
            | "take-profit"
            | "take-profit-limit"
            | "trailing-stop"
            | "trailing-stop-limit"
    )
}

pub(super) fn requires_secondary_price(order_type: &str) -> bool {
    matches!(
        order_type,
        "stop-loss-limit" | "take-profit-limit" | "trailing-stop-limit"
    )
}

pub(super) fn encode_form_component(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

pub(super) fn encode_form_params(params: &[(String, String)]) -> String {
    params
        .iter()
        .map(|(k, v)| format!("{}={}", encode_form_component(k), encode_form_component(v)))
        .collect::<Vec<_>>()
        .join("&")
}

pub(super) fn sanitize_api_error_body(body: &str) -> String {
    let mut clean = body.split_whitespace().collect::<Vec<_>>().join(" ");
    if clean.len() > 512 {
        clean.truncate(512);
        clean.push('…');
    }
    clean
}

/// Kraken's documented per-endpoint counter cost. Order placement /
/// modification / cancellation cost zero (they have their own daily rate
/// limit). Ledger / trade-history pulls cost four. Everything else costs
/// one, matching Kraken's "Standard endpoint" tier.
pub(super) fn kraken_private_rest_counter_cost(path: &str) -> f64 {
    let endpoint = path.rsplit('/').next().unwrap_or(path);
    if matches!(
        endpoint,
        "AddOrder"
            | "AddOrderBatch"
            | "AmendOrder"
            | "EditOrder"
            | "CancelOrder"
            | "CancelOrderBatch"
            | "CancelAll"
            | "CancelAllOrdersAfter"
    ) {
        0.0
    } else if matches!(
        endpoint,
        "Ledgers" | "QueryLedgers" | "TradesHistory" | "QueryTrades" | "ClosedOrders"
    ) {
        4.0
    } else {
        1.0
    }
}

/// Append one decimal component in Kraken's WS-v2 book-checksum normal form:
/// strip a leading '+', drop the '.', strip leading zeros, and emit "0" when
/// no digits remain. Shared by the L2 book and L3 order-feed checksums so the
/// (live-debugged) normalisation exists exactly once. Runs 40× per book delta
/// (2 sides × 10 levels × price+qty), so it appends into `payload` without
/// allocating; only rare scientific-notation inputs take a re-format path.
pub(super) fn push_book_checksum_component(payload: &mut String, raw: &str) {
    if raw.contains(['e', 'E']) {
        let normalized = raw
            .parse::<f64>()
            .ok()
            .map(|v| {
                if v.fract() == 0.0 {
                    format!("{v:.1}")
                } else {
                    v.to_string()
                }
            })
            .unwrap_or_else(|| raw.to_string());
        push_book_checksum_digits(payload, &normalized);
    } else {
        push_book_checksum_digits(payload, raw);
    }
}

fn push_book_checksum_digits(payload: &mut String, raw: &str) {
    let trimmed = raw.trim().trim_start_matches('+');
    let mut seen_leading_digit = false;
    let mut pushed_any = false;
    for ch in trimmed.chars() {
        if ch == '.' {
            continue;
        }
        if !seen_leading_digit && ch == '0' {
            continue;
        }
        seen_leading_digit = true;
        payload.push(ch);
        pushed_any = true;
    }
    if !pushed_any {
        payload.push('0');
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_kraken_order_type_unifies_underscore_and_case() {
        assert_eq!(normalize_kraken_order_type("Stop_Loss"), "stop-loss");
        assert_eq!(normalize_kraken_order_type("LIMIT"), "limit");
        assert_eq!(
            normalize_kraken_order_type("  take_profit_limit  "),
            "take-profit-limit"
        );
    }

    #[test]
    fn is_supported_kraken_order_type_covers_advertised_types() {
        for ty in [
            "market",
            "limit",
            "iceberg",
            "stop-loss",
            "stop-loss-limit",
            "take-profit",
            "take-profit-limit",
            "trailing-stop",
            "trailing-stop-limit",
            "settle-position",
        ] {
            assert!(
                is_supported_kraken_order_type(ty),
                "{ty} should be supported"
            );
        }
        assert!(!is_supported_kraken_order_type("bracket"));
    }

    #[test]
    fn is_supported_kraken_close_order_type_excludes_market_and_iceberg() {
        // Kraken refuses market and iceberg as conditional closes.
        assert!(!is_supported_kraken_close_order_type("market"));
        assert!(!is_supported_kraken_close_order_type("iceberg"));
        assert!(is_supported_kraken_close_order_type("limit"));
        assert!(is_supported_kraken_close_order_type("stop-loss"));
    }

    #[test]
    fn requires_primary_price_is_true_for_priced_orders_only() {
        assert!(!requires_primary_price("market"));
        assert!(requires_primary_price("limit"));
        assert!(requires_primary_price("take-profit-limit"));
    }

    #[test]
    fn requires_secondary_price_is_true_for_limit_legs() {
        assert!(!requires_secondary_price("stop-loss"));
        assert!(requires_secondary_price("stop-loss-limit"));
        assert!(requires_secondary_price("take-profit-limit"));
        assert!(requires_secondary_price("trailing-stop-limit"));
    }

    #[test]
    fn format_f64_param_drops_zero_fraction() {
        assert_eq!(format_f64_param(1.0), "1");
        assert_eq!(format_f64_param(1.5), "1.5");
    }

    #[test]
    fn push_opt_param_skips_empty_and_none() {
        let mut params = Vec::new();
        push_opt_param(&mut params, "x", None);
        push_opt_param(&mut params, "y", Some(""));
        push_opt_param(&mut params, "z", Some("v"));
        assert_eq!(params, vec![("z".to_string(), "v".to_string())]);
    }

    #[test]
    fn encode_form_component_url_encodes_reserved_bytes() {
        assert_eq!(encode_form_component("hello world"), "hello+world");
        assert_eq!(encode_form_component("a=b"), "a%3Db");
        assert_eq!(encode_form_component("k.v-1_2~"), "k.v-1_2~");
    }

    #[test]
    fn encode_form_params_joins_with_ampersands() {
        let params = vec![
            ("pair".into(), "XBTUSD".into()),
            ("type".into(), "buy".into()),
        ];
        assert_eq!(encode_form_params(&params), "pair=XBTUSD&type=buy");
    }

    #[test]
    fn sanitize_api_error_body_trims_whitespace_and_caps_length() {
        let body = "  multi\n   line\nbody  ";
        assert_eq!(sanitize_api_error_body(body), "multi line body");

        let long: String = std::iter::repeat('a').take(1024).collect();
        let clean = sanitize_api_error_body(&long);
        assert!(clean.len() <= 520);
        assert!(clean.ends_with('…'));
    }

    #[test]
    fn kraken_private_rest_counter_cost_categorizes_endpoints() {
        assert_eq!(kraken_private_rest_counter_cost("private/AddOrder"), 0.0);
        assert_eq!(kraken_private_rest_counter_cost("private/CancelOrder"), 0.0);
        assert_eq!(
            kraken_private_rest_counter_cost("private/TradesHistory"),
            4.0
        );
        assert_eq!(kraken_private_rest_counter_cost("private/Balance"), 1.0);
        assert_eq!(kraken_private_rest_counter_cost("Anything"), 1.0);
    }
}
