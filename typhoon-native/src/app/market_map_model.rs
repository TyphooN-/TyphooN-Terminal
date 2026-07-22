use std::collections::BTreeMap;

use typhoon_engine::broker::cache_keys::bare_symbol_from_key;
use typhoon_engine::core::fundamentals::Fundamentals;

#[derive(Debug)]
pub(crate) struct MarketMapSymbol {
    pub(crate) symbol: String,
    pub(crate) watchlist_key: String,
    pub(crate) market_cap: f64,
}

#[derive(Debug)]
pub(crate) struct MarketMapSector {
    pub(crate) sector: String,
    pub(crate) total_cap: f64,
    pub(crate) symbols: Box<[MarketMapSymbol]>,
}

#[derive(Debug, Default)]
pub(crate) struct MarketMapModel {
    pub(crate) grand_total: f64,
    pub(crate) sectors: Box<[MarketMapSector]>,
}

pub(crate) fn cap_weighted_change(
    sector: &MarketMapSector,
    mut live_change: impl FnMut(&str) -> f64,
) -> f64 {
    let change = sector
        .symbols
        .iter()
        .filter_map(|symbol| {
            let change = live_change(&symbol.watchlist_key);
            change
                .is_finite()
                .then_some((symbol.market_cap / sector.total_cap) * change)
        })
        .sum::<f64>();
    if change.is_finite() { change } else { 0.0 }
}

pub(crate) fn build_market_map_model(fundamentals: &[Fundamentals]) -> MarketMapModel {
    let mut by_sector: BTreeMap<String, Vec<MarketMapSymbol>> = BTreeMap::new();
    for fundamental in fundamentals {
        let Some(market_cap) = fundamental
            .market_cap
            .filter(|market_cap| market_cap.is_finite() && *market_cap > 0.0)
        else {
            continue;
        };
        let sector = match fundamental.sector.trim() {
            "" => "Other".to_string(),
            sector => sector.to_string(),
        };
        let watchlist_key = bare_symbol_from_key(fundamental.symbol.trim())
            .replace('/', "")
            .trim_end_matches(".EQ")
            .trim_end_matches(".eq")
            .to_ascii_uppercase();
        by_sector.entry(sector).or_default().push(MarketMapSymbol {
            symbol: fundamental.symbol.clone(),
            watchlist_key,
            market_cap,
        });
    }

    let mut sectors: Vec<MarketMapSector> = by_sector
        .into_iter()
        .filter_map(|(sector, mut symbols)| {
            symbols.sort_by(|left, right| {
                right
                    .market_cap
                    .total_cmp(&left.market_cap)
                    .then_with(|| left.symbol.cmp(&right.symbol))
                    .then_with(|| left.watchlist_key.cmp(&right.watchlist_key))
            });
            let total_cap = symbols.iter().try_fold(0.0, |total, symbol| {
                let next = total + symbol.market_cap;
                next.is_finite().then_some(next)
            })?;
            Some(MarketMapSector {
                sector,
                total_cap,
                symbols: symbols.into_boxed_slice(),
            })
        })
        .collect();
    sectors.sort_by(|left, right| {
        right
            .total_cap
            .total_cmp(&left.total_cap)
            .then_with(|| left.sector.cmp(&right.sector))
    });
    let mut grand_total = 0.0;
    sectors.retain(|sector| {
        let next = grand_total + sector.total_cap;
        if next.is_finite() {
            grand_total = next;
            true
        } else {
            false
        }
    });

    MarketMapModel {
        grand_total,
        sectors: sectors.into_boxed_slice(),
    }
}

#[cfg(test)]
mod tests;
