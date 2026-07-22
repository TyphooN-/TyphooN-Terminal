use super::*;

#[test]
fn empty_normalized_quote_symbol_never_matches_a_chart_alias() {
    let row_symbol = normalize_quote_symbol("/");
    assert!(row_symbol.is_empty());
    assert!(!quote_symbol_alias_matches("AAPL", &row_symbol));
    assert!(!quote_symbol_alias_matches("BTC/USD", &row_symbol));
}

#[test]
fn nonempty_quote_aliases_keep_supported_partial_matching() {
    assert!(quote_symbol_alias_matches("BTC/USD", "BTCUSD"));
    assert!(quote_symbol_alias_matches("BTCUSD", "BTC"));
    assert!(!quote_symbol_alias_matches("AAPL", "MSFT"));
}
