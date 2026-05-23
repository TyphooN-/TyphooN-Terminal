//! Tastytrade-specific small helpers.
//!
//! Symbol-universe merging tolerates partial source failures: if either
//! the public-watchlists call or the active-futures call fails, the merge
//! still returns whatever the other source provided. Only when both fail
//! does the caller see an error — and the error string mentions both
//! sources so the caller can decide which one to retry.

pub(super) fn merge_market_data_universe_sources(
    watchlists: Result<Vec<String>, String>,
    futures: Result<Vec<String>, String>,
) -> Result<Vec<String>, String> {
    let mut symbols = std::collections::BTreeSet::new();
    let mut errors = Vec::new();

    match watchlists {
        Ok(items) => {
            for symbol in items {
                let symbol = symbol.trim().to_ascii_uppercase();
                if !symbol.is_empty() {
                    symbols.insert(symbol);
                }
            }
        }
        Err(e) => errors.push(format!("public watchlists: {e}")),
    }

    match futures {
        Ok(items) => {
            for symbol in items {
                let symbol = symbol.trim().to_ascii_uppercase();
                if !symbol.is_empty() {
                    symbols.insert(symbol);
                }
            }
        }
        Err(e) => errors.push(format!("active futures: {e}")),
    }

    if !symbols.is_empty() {
        Ok(symbols.into_iter().collect())
    } else if errors.is_empty() {
        Ok(Vec::new())
    } else {
        Err(errors.join(" | "))
    }
}

/// Tastytrade rejects equity limit orders with prices that have more digits
/// of precision than the underlying tick. Bucket by magnitude: >= $1 → 2dp
/// (standard penny tick), >= $0.01 → 4dp (sub-penny tick for low-price
/// names), and < $0.01 → 6dp (warrant / cheap-option tail).
pub(super) fn format_equity_order_price(price: f64) -> String {
    if price >= 1.0 {
        format!("{:.2}", price)
    } else if price >= 0.01 {
        format!("{:.4}", price)
    } else {
        format!("{:.6}", price)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_market_data_universe_sources_keeps_partial_success() {
        let merged = merge_market_data_universe_sources(
            Err("HTTP 502".into()),
            Ok(vec!["/ESM6".into(), "/NQM6".into(), "/esm6".into()]),
        )
        .unwrap();
        assert_eq!(merged, vec!["/ESM6".to_string(), "/NQM6".to_string()]);
    }

    #[test]
    fn merge_market_data_universe_sources_reports_total_failure() {
        let err =
            merge_market_data_universe_sources(Err("HTTP 502".into()), Err("HTTP 503".into()))
                .unwrap_err();
        assert!(err.contains("public watchlists"));
        assert!(err.contains("active futures"));
    }

    #[test]
    fn merge_market_data_universe_sources_dedupes_case_insensitively() {
        let merged = merge_market_data_universe_sources(
            Ok(vec!["aapl".into(), "  AAPL ".into()]),
            Ok(vec!["AAPL".into()]),
        )
        .unwrap();
        assert_eq!(merged, vec!["AAPL".to_string()]);
    }

    #[test]
    fn merge_market_data_universe_sources_empty_both_sources_is_empty_ok() {
        let merged =
            merge_market_data_universe_sources(Ok(Vec::new()), Ok(Vec::new())).unwrap();
        assert!(merged.is_empty());
    }

    #[test]
    fn format_equity_order_price_scales_for_penny_names() {
        assert_eq!(format_equity_order_price(12.3456), "12.35");
        assert_eq!(format_equity_order_price(0.123456), "0.1235");
        assert_eq!(format_equity_order_price(0.000321), "0.000321");
    }

    #[test]
    fn format_equity_order_price_at_thresholds() {
        // Exactly $1 uses 2dp; exactly $0.01 uses 4dp.
        assert_eq!(format_equity_order_price(1.0), "1.00");
        assert_eq!(format_equity_order_price(0.01), "0.0100");
        assert_eq!(format_equity_order_price(0.009999), "0.009999");
    }
}
