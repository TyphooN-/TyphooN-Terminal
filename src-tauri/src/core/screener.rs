//! Stock screener — filter symbols by price, volume, sector, and other criteria.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenerFilter {
    pub min_price: Option<f64>,
    pub max_price: Option<f64>,
    pub min_volume: Option<f64>,
    pub max_volume: Option<f64>,
    pub sector: Option<String>,
    pub asset_class: Option<String>,
    pub min_change_pct: Option<f64>,
    pub max_change_pct: Option<f64>,
    pub tradable_only: bool,
    pub shortable_only: bool,
    pub fractionable_only: bool,
}

impl Default for ScreenerFilter {
    fn default() -> Self {
        Self {
            min_price: None,
            max_price: None,
            min_volume: None,
            max_volume: None,
            sector: None,
            asset_class: None,
            min_change_pct: None,
            max_change_pct: None,
            tradable_only: true,
            shortable_only: false,
            fractionable_only: false,
        }
    }
}

/// A symbol with cached data for screening.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenerSymbol {
    pub symbol: String,
    pub name: String,
    pub asset_class: String,
    pub price: f64,
    pub volume: f64,
    pub change_pct: f64,
    pub tradable: bool,
    pub shortable: bool,
    pub fractionable: bool,
    pub sector: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenerResult {
    pub symbols: Vec<ScreenerSymbol>,
    pub total_matched: usize,
    pub total_scanned: usize,
}

/// Filter cached symbol data by the given filters.
pub fn screen_symbols(filters: &ScreenerFilter, symbols: &[ScreenerSymbol]) -> ScreenerResult {
    let total_scanned = symbols.len();

    let matched: Vec<ScreenerSymbol> = symbols
        .iter()
        .filter(|s| {
            if filters.tradable_only && !s.tradable {
                return false;
            }
            if filters.shortable_only && !s.shortable {
                return false;
            }
            if filters.fractionable_only && !s.fractionable {
                return false;
            }
            if let Some(min) = filters.min_price {
                if s.price < min { return false; }
            }
            if let Some(max) = filters.max_price {
                if s.price > max { return false; }
            }
            if let Some(min) = filters.min_volume {
                if s.volume < min { return false; }
            }
            if let Some(max) = filters.max_volume {
                if s.volume > max { return false; }
            }
            if let Some(min) = filters.min_change_pct {
                if s.change_pct < min { return false; }
            }
            if let Some(max) = filters.max_change_pct {
                if s.change_pct > max { return false; }
            }
            if let Some(ref sector) = filters.sector {
                if let Some(ref s_sector) = s.sector {
                    if !s_sector.eq_ignore_ascii_case(sector) { return false; }
                } else {
                    return false;
                }
            }
            if let Some(ref ac) = filters.asset_class {
                if !s.asset_class.eq_ignore_ascii_case(ac) { return false; }
            }
            true
        })
        .cloned()
        .collect();

    let total_matched = matched.len();

    ScreenerResult {
        symbols: matched,
        total_matched,
        total_scanned,
    }
}
