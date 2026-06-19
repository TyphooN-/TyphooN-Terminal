/// Check if a symbol looks like a US equity (not forex, commodities, etc.)
pub(super) fn is_equity_symbol(sym: &str) -> bool {
    !sym.is_empty()
        && !sym.contains('/')
        && !sym.starts_with("XAU")
        && !sym.starts_with("XAG")
        && !sym.starts_with("XNG")
        && !sym.starts_with("XBR")
        && !sym.starts_with("XTI")
        && sym.len() <= 5
        && sym.chars().all(|c| c.is_ascii_alphabetic())
}

pub(super) fn normalize_sec_equity_symbol(sym: &str) -> Option<String> {
    let mut sym = sym.trim().to_uppercase();
    if sym.is_empty() || sym.starts_with("__") || sym.contains('/') {
        return None;
    }
    // Kraken xStocks can be stored/transmitted as venue-qualified symbols
    // (WOK.EQ, etc.). SEC EDGAR lookup needs the underlying equity
    // ticker. Normalize before applying the equity filter so scoped SEC scrapes
    // don't silently drop xStock holdings.
    if let Some(stripped) = sym.strip_suffix(".EQ") {
        sym = stripped.to_string();
    } else if let Some(stripped) = sym.strip_suffix(".X") {
        sym = stripped.to_string();
    }
    if crate::core::news::is_crypto_symbol(&sym) {
        return None;
    }
    if is_equity_symbol(&sym) {
        Some(sym)
    } else {
        None
    }
}

pub(super) fn normalize_sec_equity_symbols_preserving_order<I>(symbols: I) -> Vec<String>
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for sym in symbols {
        let Some(normalized) = normalize_sec_equity_symbol(sym.as_ref()) else {
            continue;
        };
        if seen.insert(normalized.clone()) {
            out.push(normalized);
        }
    }
    out
}

pub(super) fn collect_equity_symbols_from_kv_blob(
    compressed: &[u8],
    out: &mut std::collections::HashSet<String>,
) {
    let Ok(decompressed) = zstd::decode_all(compressed) else {
        return;
    };
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&decompressed) else {
        return;
    };
    collect_equity_symbols_from_json(&value, false, out);
}

fn collect_equity_symbols_from_json(
    value: &serde_json::Value,
    symbol_context: bool,
    out: &mut std::collections::HashSet<String>,
) {
    match value {
        serde_json::Value::String(raw) => {
            if !symbol_context {
                return;
            }
            if let Some(sym) = normalize_sec_equity_symbol(raw) {
                out.insert(sym);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                collect_equity_symbols_from_json(item, true, out);
            }
        }
        serde_json::Value::Object(map) => {
            for (key, child) in map {
                let is_symbol_field = matches!(
                    key.as_str(),
                    "symbol" | "ticker" | "sym" | "asset" | "underlying_symbol"
                );
                collect_equity_symbols_from_json(child, is_symbol_field, out);
            }
        }
        _ => {}
    }
}
