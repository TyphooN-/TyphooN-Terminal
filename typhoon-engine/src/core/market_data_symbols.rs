//! Market-data cache symbol extraction helpers shared by native chart/news code and
//! broker-runtime orchestration.

use std::collections::BTreeSet;

use crate::core::cache::BgConnection;

fn bare_symbol_from_key(key: &str) -> String {
    let parts: Vec<&str> = key.split(':').collect();
    match parts.as_slice() {
        [_src, sym, _tf] => (*sym).to_string(),
        [sym, _tf] => (*sym).to_string(),
        _ => key.to_string(),
    }
}

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

pub fn news_symbol_from_market_data_cache_key(key: &str, prefix: &str) -> Option<String> {
    let rest = key.strip_prefix(prefix)?.strip_prefix(':')?;
    let (raw_symbol, tf) = rest.rsplit_once(':')?;
    if raw_symbol.is_empty() || tf.is_empty() || raw_symbol.starts_with("__") {
        return None;
    }
    let mut symbol = normalize_market_data_symbol(raw_symbol)
        .replace('/', "")
        .to_uppercase();
    if let Some(stripped) = symbol.strip_suffix(".EQ") {
        symbol = stripped.to_string();
    }
    if symbol.is_empty() || symbol.starts_with("__") {
        None
    } else {
        Some(symbol)
    }
}

pub fn extract_news_symbols_from_market_data_cache(
    conn: &BgConnection,
    prefixes: &[&str],
) -> Result<Vec<String>, String> {
    let mut symbols = BTreeSet::new();
    for prefix in prefixes {
        let like = format!("{}:%", prefix);
        let mut stmt = conn
            .prepare("SELECT DISTINCT key FROM bar_cache WHERE key LIKE ?1")
            .map_err(|e| format!("prepare {prefix} bar-cache news symbols: {e}"))?;
        let rows = stmt
            .query_map([like.as_str()], |row| row.get::<_, String>(0))
            .map_err(|e| format!("query {prefix} bar-cache news symbols: {e}"))?;
        for row in rows {
            if let Ok(key) = row {
                if let Some(symbol) = news_symbol_from_market_data_cache_key(&key, prefix) {
                    symbols.insert(symbol);
                }
            }
        }
    }
    Ok(symbols.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn news_symbol_parser_skips_meta_and_bad_keys() {
        assert_eq!(
            news_symbol_from_market_data_cache_key("alpaca:AAPL:1Day", "alpaca"),
            Some("AAPL".to_string())
        );
        assert_eq!(
            news_symbol_from_market_data_cache_key(
                "kraken-equities:WOK.EQ:1Day",
                "kraken-equities"
            ),
            Some("WOK".to_string())
        );
        assert_eq!(
            news_symbol_from_market_data_cache_key("alpaca:__META__:1Day", "alpaca"),
            None
        );
        assert_eq!(
            news_symbol_from_market_data_cache_key("alpaca:BADKEY", "alpaca"),
            None
        );
    }
}
