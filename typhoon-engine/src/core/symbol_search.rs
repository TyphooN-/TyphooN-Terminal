//! Symbol-search helpers shared by broker runtime surfaces.

use std::collections::HashSet;

const KRAKEN_CRYPTO_BASES: &[&str] = &[
    "BTC", "ETH", "SOL", "DOGE", "XRP", "ADA", "LTC", "LINK", "AVAX", "DOT", "XMR", "ZEC", "DASH",
    "UNI", "AAVE", "ATOM", "NEAR", "FIL", "ICP", "XLM", "ALGO", "VET", "HBAR", "FTM", "SAND",
    "MANA", "AXS", "GRT", "ENJ", "BAT", "COMP", "MKR", "SNX", "CRV", "SUSHI", "YFI", "TRX", "ETC",
    "EOS", "XTZ", "SHIB", "APE", "ARB", "OP", "THETA", "KAVA", "MATIC", "BCH",
];

pub fn append_kraken_crypto_symbol_suggestions(
    query_upper: &str,
    query_without_usd: &str,
    suggestion_symbols: &mut HashSet<String>,
    all_suggestions: &mut Vec<(String, String, String)>,
) {
    for base in KRAKEN_CRYPTO_BASES {
        let sym = format!("{}USD", base);
        if (sym.contains(query_upper) || base.contains(query_without_usd))
            && suggestion_symbols.insert(sym.clone())
        {
            all_suggestions.push((sym, format!("{} (crypto)", base), "Kraken".into()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kraken_crypto_suggestions_match_base_or_usd_suffix_query() {
        let mut seen = HashSet::new();
        let mut suggestions = Vec::new();

        append_kraken_crypto_symbol_suggestions("BTCUSD", "BTC", &mut seen, &mut suggestions);

        assert_eq!(suggestions[0].0, "BTCUSD");
        assert_eq!(suggestions[0].1, "BTC (crypto)");
        assert_eq!(suggestions[0].2, "Kraken");
    }

    #[test]
    fn kraken_crypto_suggestions_respect_existing_seen_symbols() {
        let mut seen = HashSet::from(["ETHUSD".to_string()]);
        let mut suggestions = Vec::new();

        append_kraken_crypto_symbol_suggestions("ETH", "ETH", &mut seen, &mut suggestions);

        assert!(suggestions.is_empty());
    }
}
