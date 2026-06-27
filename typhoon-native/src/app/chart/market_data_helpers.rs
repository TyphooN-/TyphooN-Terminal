use super::*;

/// Extract the bare symbol from a cache key. Accepts canonical 3-part
/// `source:SYM:TF`, legacy 2-part `SYM:TF`, or bare `SYM`. Used by load
/// paths that need to put a canonical symbol into `ChartState::symbol`
/// so `try_load` and the chart header agree on its shape.
// `bare_symbol_from_key` moved to typhoon-chart-ui (ADR-125 Target 2, slice 7b); re-exported.
pub(crate) use typhoon_chart_ui::types::bare_symbol_from_key;

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

pub(crate) fn kraken_pair_source<'a>(pair_name: &'a str, display_name: &'a str) -> &'a str {
    if display_name.trim().is_empty() {
        pair_name
    } else {
        display_name
    }
}

pub(crate) fn kraken_pair_base_quote(
    pair_name: &str,
    display_name: &str,
) -> Option<(String, String)> {
    let source = kraken_pair_source(pair_name, display_name);
    if let Some((base, quote)) = source.split_once('/') {
        let base = typhoon_engine::core::kraken::normalize_pair_symbol(base);
        let quote = typhoon_engine::core::kraken::normalize_pair_symbol(quote);
        if !base.is_empty() && !quote.is_empty() {
            return Some((base, quote));
        }
    }
    let symbol = typhoon_engine::core::kraken::normalize_pair_symbol(source);
    pub(crate) const QUOTES: [&str; 15] = [
        "USDG", "USDT", "USDC", "USD", "EUR", "GBP", "CAD", "AUD", "JPY", "CHF", "XBT", "BTC",
        "ETH", "SOL", "DAI",
    ];
    let quote = QUOTES
        .iter()
        .find(|quote| symbol.ends_with(**quote) && symbol.len() > quote.len())?;
    let base = symbol.strip_suffix(*quote)?;
    Some((base.to_string(), quote.to_string()))
}

pub(crate) fn kraken_pair_is_fiat_fx(pair_name: &str, display_name: &str) -> bool {
    let Some((base, quote)) = kraken_pair_base_quote(pair_name, display_name) else {
        return false;
    };
    pub(crate) const FIAT: [&str; 7] = ["USD", "EUR", "GBP", "CAD", "AUD", "JPY", "CHF"];
    FIAT.contains(&base.as_str()) && FIAT.contains(&quote.as_str())
}

pub(crate) fn kraken_pair_asset_class(pair_name: &str, display_name: &str) -> &'static str {
    if kraken_pair_is_fiat_fx(pair_name, display_name) {
        "fx"
    } else if kraken_xstock_fundamental_symbol(pair_name, display_name).is_some() {
        "xstock"
    } else {
        "crypto"
    }
}

pub(crate) fn kraken_xstock_fundamental_symbol(
    pair_name: &str,
    display_name: &str,
) -> Option<String> {
    let source = kraken_pair_source(pair_name, display_name);
    let symbol = typhoon_engine::core::kraken::normalize_pair_symbol(source);
    let (base, _quote) = kraken_pair_base_quote(pair_name, display_name)?;
    // Public AssetPairs currently exposes crypto + spot FX. Tokenized equity
    // holdings from private balances use `.EQ`; avoid treating ordinary crypto
    // tickers that end in `X` (AVAX, FLUX, CVX, etc.) as xStocks.
    let equity = base
        .strip_suffix(".EQ")
        .or_else(|| symbol.strip_suffix(".EQ"))?;
    if equity.is_empty()
        || matches!(equity, "XBT" | "BTC" | "XDG" | "DOGE")
        || !equity
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.')
    {
        return None;
    }
    Some(equity.to_string())
}

pub(crate) fn cache_source_from_key(key: &str) -> &'static str {
    if key.starts_with("alpaca:") {
        "alpaca"
    } else if key.starts_with("kraken-equities:") {
        "kraken-equities"
    } else if key.starts_with("kraken-futures:") {
        "kraken-futures"
    } else if key.starts_with("kraken:") {
        "kraken"
    } else if key.starts_with("yahoo-chart:") {
        "yahoo-chart"
    } else if key.starts_with("merged:") {
        "merged"
    } else if key.starts_with("default:") {
        "default"
    } else {
        ""
    }
}

pub(crate) fn chart_source_bars_match_timeframe(
    source: &str,
    timeframe: &str,
    bars: &[(i64, f64, f64, f64, f64, f64)],
) -> bool {
    if timeframe == "1Month" && matches!(source, "kraken" | "kraken-equities" | "kraken-futures") {
        return false;
    }
    if bars.len() < 20 {
        return true;
    }
    let Some((min_delta_ms, max_median_delta_ms)) = chart_timeframe_cadence_bounds(timeframe)
    else {
        return true;
    };
    let mut timestamps: Vec<i64> = bars
        .iter()
        .map(|(ts, _, _, _, _, _)| *ts)
        .filter(|ts| *ts > 0)
        .collect();
    timestamps.sort_unstable();
    timestamps.dedup();
    if timestamps.len() < 20 {
        return true;
    }
    let mut deltas: Vec<i64> = timestamps
        .windows(2)
        .filter_map(|w| w[1].checked_sub(w[0]))
        .filter(|delta| *delta > 0)
        .collect();
    if deltas.len() < 10 {
        return true;
    }
    deltas.sort_unstable();
    let median = deltas[deltas.len() / 2];
    median >= min_delta_ms && median <= max_median_delta_ms
}

pub(crate) fn chart_timeframe_cadence_bounds(timeframe: &str) -> Option<(i64, i64)> {
    let hour = 3_600_000i64;
    let day = 24 * hour;
    match timeframe {
        "1Min" => Some((30_000, 5 * 60_000)),
        "5Min" => Some((2 * 60_000, 20 * 60_000)),
        "15Min" => Some((5 * 60_000, 60 * 60_000)),
        "30Min" => Some((10 * 60_000, 2 * hour)),
        "1Hour" => Some((20 * 60_000, 4 * hour)),
        "4Hour" => Some((hour, 16 * hour)),
        "1Day" => Some((12 * hour, 5 * day)),
        "1Week" => Some((5 * day, 8 * day)),
        "1Month" => Some((26 * day, 35 * day)),
        _ => None,
    }
}

pub(crate) fn chart_gap_fill_bar_allowed(
    primary_source: &str,
    gap_source: &str,
    snapped: i64,
    primary_min_snapped: Option<i64>,
    primary_max_snapped: Option<i64>,
) -> bool {
    if !matches!(primary_source, "kraken-equities" | "alpaca" | "yahoo-chart")
        || !matches!(gap_source, "alpaca" | "yahoo-chart")
    {
        return true;
    }

    match (primary_min_snapped, primary_max_snapped) {
        (Some(min), Some(max)) => snapped < min || snapped > max,
        _ => true,
    }
}

#[allow(dead_code)]
pub(crate) fn chart_quote_overlay_allowed(quote_ts_ms: i64, last_bar_ts_ms: i64) -> bool {
    quote_ts_ms >= last_bar_ts_ms
}

pub(crate) fn chart_bar_last_valid_ts(raw: &[(i64, f64, f64, f64, f64, f64)]) -> i64 {
    raw.iter()
        .rev()
        .find_map(|(ts, _o, _h, _l, close, _v)| {
            (*ts > 0 && *close > 0.0 && close.is_finite()).then_some(*ts)
        })
        .unwrap_or(0)
}

pub(crate) fn chart_merge_bucket_ts(timeframe: &str, ts: i64) -> i64 {
    match timeframe {
        "1Month" => chrono::DateTime::from_timestamp_millis(ts)
            .and_then(|dt| {
                chrono::NaiveDate::from_ymd_opt(dt.year(), dt.month(), 1)
                    .and_then(|d| d.and_hms_opt(0, 0, 0))
            })
            .map(|ndt| ndt.and_utc().timestamp_millis())
            .unwrap_or(ts),
        "1Week" => chrono::DateTime::from_timestamp_millis(ts)
            .and_then(|dt| {
                let days_since_mon = dt.weekday().num_days_from_monday() as i64;
                (dt.date_naive() - chrono::Duration::days(days_since_mon)).and_hms_opt(0, 0, 0)
            })
            .map(|ndt| ndt.and_utc().timestamp_millis())
            .unwrap_or(ts),
        "1Day" => chrono::DateTime::from_timestamp_millis(ts)
            .and_then(|dt| {
                chrono::NaiveDate::from_ymd_opt(dt.year(), dt.month(), dt.day())
                    .and_then(|d| d.and_hms_opt(0, 0, 0))
            })
            .map(|ndt| ndt.and_utc().timestamp_millis())
            .unwrap_or(ts),
        "4Hour" => ts / (4 * 3_600_000) * (4 * 3_600_000),
        "1Hour" => ts / 3_600_000 * 3_600_000,
        "30Min" => ts / 1_800_000 * 1_800_000,
        "15Min" => ts / 900_000 * 900_000,
        "5Min" => ts / 300_000 * 300_000,
        _ => ts / 60_000 * 60_000,
    }
}
