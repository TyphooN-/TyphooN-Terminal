//! Source symbol variant and cache key generation helpers (extracted from chart.rs).
//!
//! Cohesive unit for "which cache keys should we try for this symbol + source + TF".
//! Extracted to reduce chart.rs size for faster incremental compiles.
//! Maintains O(1) dedup via HashSet.
//!
//! Re-exported from chart for backward compat with callers.

use std::collections::HashSet;

pub(crate) fn bare_symbol_from_key(key: &str) -> String {
    let parts: Vec<&str> = key.split(':').collect();
    match parts.as_slice() {
        [_src, sym, _tf] => (*sym).to_string(),
        [sym, _tf] => (*sym).to_string(),
        _ => key.to_string(),
    }
}

pub(crate) fn normalize_market_data_symbol(symbol: &str) -> String {
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

pub(crate) fn push_unique_symbol_variant(out: &mut Vec<String>, seen: &mut HashSet<String>, value: impl Into<String>) {
    let value = value.into();
    if value.trim().is_empty() {
        return;
    }
    let normalized = value.trim().to_uppercase();
    if seen.insert(normalized.clone()) {
        out.push(normalized);
    }
}

pub(crate) fn chart_source_symbol_variants(symbol: &str) -> Vec<String> {
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

pub(crate) fn chart_source_cache_keys(source: &str, symbol: &str, timeframe: &str) -> Vec<String> {
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
