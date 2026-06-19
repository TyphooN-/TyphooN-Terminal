//! Crypto symbol routing and crypto-native news source fetchers.

use super::NewsArticle;
use super::source_fetchers::{parse_iso_ts, parse_rss_items};

// ── Crypto symbol detection ───────────────────────────────────────────────
//
// Crypto-native news sources (CryptoPanic, CoinDesk, Finnhub `/news?category=crypto`)
// expect base tickers like "BTC" rather than the trading-pair form ("BTC/USD",
// "BTCUSD", "BTC-USD") that users may type. `crypto_base_for_symbol` peels off
// the quote currency and validates against a curated allowlist; anything not on
// the list is treated as a non-crypto symbol so the equity router runs instead.

/// Curated allowlist of crypto base tickers. Used to disambiguate concatenated
/// symbols like "BTCUSD" from equity tickers, and to filter general-feed crypto
/// news (CoinDesk RSS, Finnhub crypto) by base mention.
const CRYPTO_BASES: &[&str] = &[
    "BTC", "ETH", "SOL", "ADA", "DOT", "DOGE", "MATIC", "POL", "AVAX", "LINK", "UNI", "XRP", "LTC",
    "BCH", "ATOM", "ALGO", "NEAR", "FTM", "HBAR", "VET", "SAND", "MANA", "SHIB", "TRX", "ETC",
    "XLM", "USDT", "USDC", "DAI", "WBTC", "FIL", "ICP", "APT", "ARB", "OP", "INJ", "TIA", "SEI",
    "STX", "RNDR", "PYTH", "FET", "TAO", "PEPE", "BONK", "WIF", "FLOKI", "JUP", "STRK", "ENA",
    "ONDO", "SUI", "TON", "MKR", "GRT", "AAVE", "CRV", "SNX", "COMP", "LDO", "RUNE", "KAS", "QNT",
    "XMR", "ZEC", "DASH", "EOS", "NEO", "BAT", "ENJ", "CHZ", "CAKE", "GALA", "AXS", "FLOW", "ROSE",
    "1INCH", "YFI", "BAL", "ZRX", "KSM", "WAVES", "DCR", "OMG", "REN", "STORJ", "ANKR", "CELO",
    "NMR", "RLC", "BAND", "REP", "KAVA", "BNT", "OXT", "GNO", "POLY", "LRC", "NU", "PAXG", "KNC",
    "REQ", "WLD", "BABY",
];

/// Quote currencies recognised when parsing trading-pair symbols.
const CRYPTO_QUOTES: &[&str] = &[
    "USD", "USDT", "USDC", "DAI", "EUR", "GBP", "JPY", "CHF", "AUD", "CAD", "BTC", "ETH", "XBT",
];

/// Map a base ticker to the full asset name for keyword filtering of general
/// feeds (CoinDesk RSS, Finnhub crypto category). Only the top names with
/// distinctive titles are listed; bases not in this map are filtered by the
/// ticker alone, which is generally fine for shorter names appearing in headlines.
pub(super) fn crypto_full_name(base: &str) -> Option<&'static str> {
    match base {
        "BTC" | "WBTC" | "XBT" => Some("Bitcoin"),
        "ETH" => Some("Ethereum"),
        "SOL" => Some("Solana"),
        "ADA" => Some("Cardano"),
        "DOT" => Some("Polkadot"),
        "DOGE" => Some("Dogecoin"),
        "MATIC" | "POL" => Some("Polygon"),
        "AVAX" => Some("Avalanche"),
        "LINK" => Some("Chainlink"),
        "UNI" => Some("Uniswap"),
        "XRP" => Some("Ripple"),
        "LTC" => Some("Litecoin"),
        "BCH" => Some("Bitcoin Cash"),
        "ATOM" => Some("Cosmos"),
        "ALGO" => Some("Algorand"),
        "FTM" => Some("Fantom"),
        "HBAR" => Some("Hedera"),
        "SAND" => Some("Sandbox"),
        "MANA" => Some("Decentraland"),
        "SHIB" => Some("Shiba Inu"),
        "TRX" => Some("TRON"),
        "ETC" => Some("Ethereum Classic"),
        "XLM" => Some("Stellar"),
        "USDT" => Some("Tether"),
        "USDC" => Some("USD Coin"),
        "FIL" => Some("Filecoin"),
        "ICP" => Some("Internet Computer"),
        "APT" => Some("Aptos"),
        "ARB" => Some("Arbitrum"),
        "OP" => Some("Optimism"),
        "INJ" => Some("Injective"),
        "TIA" => Some("Celestia"),
        "PYTH" => Some("Pyth"),
        "TAO" => Some("Bittensor"),
        "WLD" => Some("Worldcoin"),
        "JUP" => Some("Jupiter"),
        "STRK" => Some("Starknet"),
        "ONDO" => Some("Ondo"),
        "SUI" => Some("Sui"),
        "TON" => Some("Toncoin"),
        "MKR" => Some("Maker"),
        "GRT" => Some("The Graph"),
        "AAVE" => Some("Aave"),
        "LDO" => Some("Lido"),
        "RUNE" => Some("THORChain"),
        "KAS" => Some("Kaspa"),
        "QNT" => Some("Quant"),
        "XMR" => Some("Monero"),
        _ => None,
    }
}

/// True if `symbol` looks like a crypto pair (BTC/USD, BTCUSD, BTC-USD, BTC).
pub fn is_crypto_symbol(symbol: &str) -> bool {
    crypto_base_for_symbol(symbol).is_some()
}

/// Peel a crypto base ticker out of `symbol`. Recognises:
/// - explicit pair separators: `BTC/USD`, `BTC-USD`
/// - concatenated pairs:       `BTCUSD`, `BTCUSDT`
/// - bare bases:               `BTC`
///
/// Returns `None` if the result isn't in [`CRYPTO_BASES`], so equity tickers
/// like `XOM` (oil) or `BTU` (Peabody) aren't misclassified.
pub fn crypto_base_for_symbol(symbol: &str) -> Option<String> {
    let s = symbol.trim().to_uppercase();
    if s.is_empty() {
        return None;
    }
    // Explicit separators first.
    for sep in ['/', '-', ':'] {
        if let Some((left, right)) = s.split_once(sep) {
            if CRYPTO_BASES.contains(&left) && CRYPTO_QUOTES.contains(&right) {
                return Some(left.to_string());
            }
        }
    }
    // Bare base, e.g. user typed "BTC".
    if CRYPTO_BASES.contains(&s.as_str()) {
        return Some(s);
    }
    // Concatenated form — peel off the longest matching quote suffix.
    for quote in CRYPTO_QUOTES {
        if let Some(base) = s.strip_suffix(quote) {
            if CRYPTO_BASES.contains(&base) {
                return Some(base.to_string());
            }
        }
    }
    None
}

/// True when `headline` or `summary` mentions either the base ticker or the
/// asset's full name. Used to filter general-feed crypto news to articles
/// actually about the requested coin.
pub(super) fn article_mentions_crypto(headline: &str, summary: &str, base: &str) -> bool {
    let hay = format!("{} {}", headline, summary);
    let hay_upper = hay.to_uppercase();
    if hay_upper.contains(base) {
        return true;
    }
    if let Some(name) = crypto_full_name(base) {
        if hay_upper.contains(&name.to_uppercase()) {
            return true;
        }
    }
    false
}

// ── Crypto-native fetchers ────────────────────────────────────────────────

/// CryptoPanic — public free tier, per-currency filtering.
/// See https://cryptopanic.com/developers/api/ — `auth_token` + `currencies=BTC,ETH`.
pub async fn fetch_cryptopanic_news(
    client: &reqwest::Client,
    symbol: &str,
    token: &str,
) -> Result<Vec<NewsArticle>, String> {
    if token.is_empty() {
        return Err("CryptoPanic auth token required".into());
    }
    let Some(base) = crypto_base_for_symbol(symbol) else {
        return Ok(vec![]);
    };
    let resp = client
        .get("https://cryptopanic.com/api/v1/posts/")
        .query(&[
            ("auth_token", token),
            ("currencies", base.as_str()),
            ("public", "true"),
            ("kind", "news"),
        ])
        .send()
        .await
        .map_err(|e| format!("CryptoPanic request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("CryptoPanic: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("CryptoPanic parse: {e}"))?;
    let mut out = Vec::new();
    if let Some(arr) = v["results"].as_array() {
        for e in arr {
            let url = e["url"].as_str().unwrap_or("").to_string();
            if url.is_empty() {
                continue;
            }
            let published_at = parse_iso_ts(e["published_at"].as_str().unwrap_or(""));
            let mut tickers = Vec::new();
            if let Some(cs) = e["currencies"].as_array() {
                for c in cs {
                    if let Some(code) = c["code"].as_str() {
                        tickers.push(code.to_uppercase());
                    }
                }
            }
            let votes_pos = e["votes"]["positive"].as_i64().unwrap_or(0);
            let votes_neg = e["votes"]["negative"].as_i64().unwrap_or(0);
            let sentiment_score = match (votes_pos, votes_neg) {
                (p, n) if p + n == 0 => 0.0,
                (p, n) => (p - n) as f64 / (p + n) as f64,
            };
            let sentiment = if sentiment_score > 0.15 {
                "bullish"
            } else if sentiment_score < -0.15 {
                "bearish"
            } else {
                "neutral"
            };
            let art = NewsArticle {
                symbol: base.clone(),
                source: "CryptoPanic".into(),
                provider: e["source"]["title"].as_str().unwrap_or("").to_string(),
                headline: e["title"].as_str().unwrap_or("").to_string(),
                summary: String::new(),
                url: url.clone(),
                published_at,
                image_url: String::new(),
                sentiment: sentiment.into(),
                sentiment_score,
                tickers,
                ..Default::default()
            }
            .with_hash();
            out.push(art);
        }
    }
    Ok(out)
}

/// CoinDesk RSS — general crypto news, no key. Filtered to articles mentioning
/// the requested base ticker or its full name.
pub async fn fetch_coindesk_rss(
    client: &reqwest::Client,
    symbol: &str,
) -> Result<Vec<NewsArticle>, String> {
    let Some(base) = crypto_base_for_symbol(symbol) else {
        return Ok(vec![]);
    };
    let resp = client
        .get("https://www.coindesk.com/arc/outboundfeeds/rss/")
        .header(
            "User-Agent",
            "Mozilla/5.0 (X11; Linux x86_64) TyphooN-Terminal/0.1",
        )
        .send()
        .await
        .map_err(|e| format!("CoinDesk RSS request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("CoinDesk RSS: HTTP {}", resp.status()));
    }
    let body = resp
        .text()
        .await
        .map_err(|e| format!("CoinDesk RSS read: {e}"))?;
    let all = parse_rss_items(&body, &base, "CoinDesk");
    let filtered: Vec<NewsArticle> = all
        .into_iter()
        .filter(|a| article_mentions_crypto(&a.headline, &a.summary, &base))
        .collect();
    Ok(filtered)
}

/// Finnhub general crypto feed — same key as `/company-news`, no symbol param.
/// Filtered to articles mentioning the requested base.
pub async fn fetch_finnhub_crypto_news(
    client: &reqwest::Client,
    symbol: &str,
    token: &str,
) -> Result<Vec<NewsArticle>, String> {
    if token.is_empty() {
        return Err("Finnhub key required".into());
    }
    let Some(base) = crypto_base_for_symbol(symbol) else {
        return Ok(vec![]);
    };
    let resp = client
        .get("https://finnhub.io/api/v1/news")
        .query(&[("category", "crypto"), ("token", token)])
        .send()
        .await
        .map_err(|e| format!("Finnhub crypto request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Finnhub crypto: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp
        .json()
        .await
        .map_err(|e| format!("Finnhub crypto parse: {e}"))?;
    let mut out = Vec::new();
    for e in arr {
        let url = e["url"].as_str().unwrap_or("").to_string();
        if url.is_empty() {
            continue;
        }
        let headline = e["headline"].as_str().unwrap_or("").to_string();
        let summary = e["summary"].as_str().unwrap_or("").to_string();
        if !article_mentions_crypto(&headline, &summary, &base) {
            continue;
        }
        let related = e["related"].as_str().unwrap_or("");
        let tickers: Vec<String> = related
            .split(',')
            .filter_map(|s| {
                let t = s.trim().to_uppercase();
                if t.is_empty() { None } else { Some(t) }
            })
            .collect();
        let art = NewsArticle {
            symbol: base.clone(),
            source: "Finnhub".into(),
            provider: e["source"].as_str().unwrap_or("").to_string(),
            headline,
            summary,
            url: url.clone(),
            published_at: e["datetime"].as_i64().unwrap_or(0),
            image_url: e["image"].as_str().unwrap_or("").to_string(),
            tickers,
            ..Default::default()
        }
        .with_hash();
        out.push(art);
    }
    Ok(out)
}
