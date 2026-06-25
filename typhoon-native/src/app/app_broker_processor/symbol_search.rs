use super::prelude::*;

const KRAKEN_CRYPTO_BASES: &[&str] = &[
    "BTC", "ETH", "SOL", "DOGE", "XRP", "ADA", "LTC", "LINK", "AVAX", "DOT", "XMR", "ZEC",
    "DASH", "UNI", "AAVE", "ATOM", "NEAR", "FIL", "ICP", "XLM", "ALGO", "VET", "HBAR",
    "FTM", "SAND", "MANA", "AXS", "GRT", "ENJ", "BAT", "COMP", "MKR", "SNX", "CRV",
    "SUSHI", "YFI", "TRX", "ETC", "EOS", "XTZ", "SHIB", "APE", "ARB", "OP", "THETA",
    "KAVA", "MATIC", "BCH",
];

pub(super) async fn handle_symbol_search_command(
    query: String,
    broker: Option<&AlpacaBroker>,
    broker_msg_tx: &tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
) {
    let q = query.to_uppercase();
    let q_without_usd = q.strip_suffix("USD").unwrap_or(&q);
    let mut all_suggestions: Vec<(String, String, String)> = Vec::with_capacity(25);
    let mut suggestion_symbols: HashSet<String> = HashSet::with_capacity(32);

    // Search Alpaca assets.
    if let Some(b) = broker {
        if let Ok(assets) = b.get_all_assets().await {
            let mut matches: Vec<(u8, &_)> = assets
                .iter()
                .filter_map(|a| {
                    let sym = a.symbol.to_uppercase();
                    let sym_no_slash = sym.replace('/', "");
                    if sym == q || sym_no_slash == q {
                        Some((0, a))
                    } else if sym.starts_with(&q) || sym_no_slash.starts_with(&q) {
                        Some((1, a))
                    } else if sym.contains(&q) || sym_no_slash.contains(&q) {
                        Some((2, a))
                    } else if a.name.to_uppercase().contains(&q) {
                        Some((3, a))
                    } else {
                        None
                    }
                })
                .collect();
            matches.sort_by_key(|(pri, _)| *pri);
            for (_, a) in matches.iter().take(15) {
                if suggestion_symbols.insert(a.symbol.to_uppercase()) {
                    all_suggestions.push((
                        a.symbol.clone(),
                        a.name.clone(),
                        format!("Alpaca {}", a.asset_class),
                    ));
                }
            }
        }
    }

    // Search common Kraken crypto symbols by pattern.
    for base in KRAKEN_CRYPTO_BASES {
        let sym = format!("{}USD", base);
        if (sym.contains(&q) || base.contains(&q_without_usd)) && suggestion_symbols.insert(sym.clone()) {
            all_suggestions.push((sym, format!("{} (crypto)", base), "Kraken".into()));
        }
    }

    if all_suggestions.is_empty() {
        return;
    }

    let text = all_suggestions
        .iter()
        .take(25)
        .map(|(s, n, src)| format!("{} — {} [{}]", s, n, src))
        .collect::<Vec<_>>()
        .join("\n");
    let _ = broker_msg_tx.send(BrokerMsg::JsonResult("Symbol Search".into(), text));
    let suggestions: Vec<(String, String, String)> = all_suggestions.into_iter().take(25).collect();
    let _ = broker_msg_tx.send(BrokerMsg::SymbolSuggestions(suggestions));
}
