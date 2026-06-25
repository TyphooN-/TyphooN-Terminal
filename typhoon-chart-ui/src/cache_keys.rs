//! Chart source variant and cache-key generation helpers.
//!
//! Shared by native chart/cache code and the future broker-runtime extraction.

use std::collections::HashSet;

use crate::types::bare_symbol_from_key;

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

fn push_unique_symbol_variant(
    out: &mut Vec<String>,
    seen: &mut HashSet<String>,
    value: impl Into<String>,
) {
    let value = value.into();
    if value.trim().is_empty() {
        return;
    }
    let normalized = value.trim().to_uppercase();
    if seen.insert(normalized.clone()) {
        out.push(normalized);
    }
}

fn chart_source_symbol_variants(symbol: &str) -> Vec<String> {
    let mut variants = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let raw = bare_symbol_from_key(symbol);
    let norm = normalize_market_data_symbol(&raw);
    let no_slash = norm.replace('/', "");

    push_unique_symbol_variant(&mut variants, &mut seen, raw);
    push_unique_symbol_variant(&mut variants, &mut seen, norm.clone());
    push_unique_symbol_variant(&mut variants, &mut seen, no_slash.clone());
    push_unique_symbol_variant(
        &mut variants,
        &mut seen,
        typhoon_engine::core::kraken::normalize_pair_symbol(&norm),
    );
    push_unique_symbol_variant(
        &mut variants,
        &mut seen,
        typhoon_engine::core::kraken_futures::normalize_futures_symbol(&norm),
    );

    if !no_slash.contains('/') && no_slash.len() >= 2 && !no_slash.ends_with("USD") {
        push_unique_symbol_variant(&mut variants, &mut seen, format!("{no_slash}USD"));
    }

    variants
}

pub fn chart_source_cache_keys(source: &str, symbol: &str, timeframe: &str) -> Vec<String> {
    let variants = chart_source_symbol_variants(symbol);
    let mut keys = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for variant in variants {
        let source_variant = match source {
            "kraken" | "kraken-futures" => variant.replace('/', ""),
            "kraken-equities" => variant.replace('/', "").trim_end_matches(".EQ").to_string(),
            _ => variant,
        };
        let key = match source {
            "default" => format!("default:{source_variant}:{timeframe}"),
            "alpaca-legacy-paper" => format!("paper_TyphooN:{source_variant}:{timeframe}"),
            "alpaca-legacy-named" => format!("alpaca_paper_TyphooN:{source_variant}:{timeframe}"),
            _ => format!("{source}:{source_variant}:{timeframe}"),
        };
        let key_u = key.to_ascii_uppercase();
        if seen.insert(key_u) {
            keys.push(key);
        }
    }

    if source == "alpaca" {
        for legacy_source in ["alpaca-legacy-paper", "alpaca-legacy-named"] {
            for key in chart_source_cache_keys(legacy_source, symbol, timeframe) {
                let key_u = key.to_ascii_uppercase();
                if seen.insert(key_u) {
                    keys.push(key);
                }
            }
        }
    } else if source == "kraken" {
        for fallback_source in ["kraken-equities", "alpaca", "default"] {
            for key in chart_source_cache_keys(fallback_source, symbol, timeframe) {
                let key_u = key.to_ascii_uppercase();
                if seen.insert(key_u) {
                    keys.push(key);
                }
            }
        }
    }

    keys
}

pub fn normalize_kraken_equity_symbol_list<'a, I>(symbols: I) -> Vec<String>
where
    I: IntoIterator<Item = &'a String>,
{
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for source in symbols {
        let symbol = normalize_market_data_symbol(source)
            .replace('/', "")
            .trim_end_matches(".EQ")
            .to_ascii_uppercase();
        if !symbol.is_empty() && seen.insert(symbol.clone()) {
            out.push(symbol);
        }
    }
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alpaca_keys_include_legacy_fallbacks_without_duplicates() {
        let keys = chart_source_cache_keys("alpaca", "AAPL.US", "1Day");
        assert!(keys.contains(&"alpaca:AAPL:1Day".to_string()));
        assert!(keys.contains(&"paper_TyphooN:AAPL:1Day".to_string()));
        assert!(keys.contains(&"alpaca_paper_TyphooN:AAPL:1Day".to_string()));
        let unique: HashSet<_> = keys.iter().map(|key| key.to_ascii_uppercase()).collect();
        assert_eq!(unique.len(), keys.len());
    }

    #[test]
    fn kraken_keys_include_equity_and_default_fallbacks() {
        let keys = chart_source_cache_keys("kraken", "BTC/USD", "1Hour");
        assert!(keys.contains(&"kraken:BTCUSD:1Hour".to_string()));
        assert!(keys.contains(&"kraken-equities:BTCUSD:1Hour".to_string()));
        assert!(keys.contains(&"default:BTCUSD:1Hour".to_string()));
    }

    #[test]
    fn kraken_equity_symbol_list_strips_wrappers_and_dedupes() {
        let raw = vec![
            "aapl.eq".to_string(),
            "AAPL".to_string(),
            "BRK.B".to_string(),
            "BTC/USD".to_string(),
        ];
        assert_eq!(
            normalize_kraken_equity_symbol_list(raw.iter()),
            vec![
                "AAPL".to_string(),
                "BRK.B".to_string(),
                "BTCUSD".to_string()
            ]
        );
    }
}
