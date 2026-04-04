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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_symbol(
        symbol: &str,
        price: f64,
        volume: f64,
        change_pct: f64,
        tradable: bool,
        shortable: bool,
        fractionable: bool,
        sector: Option<&str>,
    ) -> ScreenerSymbol {
        ScreenerSymbol {
            symbol: symbol.to_string(),
            name: format!("{symbol} Inc"),
            asset_class: "us_equity".to_string(),
            price,
            volume,
            change_pct,
            tradable,
            shortable,
            fractionable,
            sector: sector.map(|s| s.to_string()),
        }
    }

    #[test]
    fn empty_input_returns_zero() {
        let filter = ScreenerFilter::default();
        let result = screen_symbols(&filter, &[]);
        assert_eq!(result.total_scanned, 0);
        assert_eq!(result.total_matched, 0);
    }

    #[test]
    fn default_filter_passes_tradable() {
        let syms = vec![test_symbol("AAPL", 150.0, 1e6, 1.0, true, true, true, Some("Tech"))];
        let result = screen_symbols(&ScreenerFilter::default(), &syms);
        assert_eq!(result.total_matched, 1);
    }

    #[test]
    fn default_filter_rejects_non_tradable() {
        let syms = vec![test_symbol("OTC", 5.0, 1e3, 0.0, false, false, false, None)];
        let result = screen_symbols(&ScreenerFilter::default(), &syms);
        assert_eq!(result.total_matched, 0);
    }

    #[test]
    fn min_price_filter() {
        let syms = vec![
            test_symbol("CHEAP", 2.0, 1e6, 0.0, true, true, true, None),
            test_symbol("PRICEY", 200.0, 1e6, 0.0, true, true, true, None),
        ];
        let filter = ScreenerFilter { min_price: Some(10.0), ..Default::default() };
        let result = screen_symbols(&filter, &syms);
        assert_eq!(result.total_matched, 1);
        assert_eq!(result.symbols[0].symbol, "PRICEY");
    }

    #[test]
    fn max_price_filter() {
        let syms = vec![
            test_symbol("CHEAP", 2.0, 1e6, 0.0, true, true, true, None),
            test_symbol("PRICEY", 200.0, 1e6, 0.0, true, true, true, None),
        ];
        let filter = ScreenerFilter { max_price: Some(50.0), ..Default::default() };
        let result = screen_symbols(&filter, &syms);
        assert_eq!(result.total_matched, 1);
        assert_eq!(result.symbols[0].symbol, "CHEAP");
    }

    #[test]
    fn min_volume_filter() {
        let syms = vec![
            test_symbol("LOW", 50.0, 100.0, 0.0, true, true, true, None),
            test_symbol("HIGH", 50.0, 1e7, 0.0, true, true, true, None),
        ];
        let filter = ScreenerFilter { min_volume: Some(1e4), ..Default::default() };
        let result = screen_symbols(&filter, &syms);
        assert_eq!(result.total_matched, 1);
        assert_eq!(result.symbols[0].symbol, "HIGH");
    }

    #[test]
    fn sector_filter_case_insensitive() {
        let syms = vec![
            test_symbol("AAPL", 150.0, 1e6, 1.0, true, true, true, Some("Technology")),
            test_symbol("XOM", 100.0, 1e6, 0.5, true, true, true, Some("Energy")),
        ];
        let filter = ScreenerFilter {
            sector: Some("technology".to_string()),
            ..Default::default()
        };
        let result = screen_symbols(&filter, &syms);
        assert_eq!(result.total_matched, 1);
        assert_eq!(result.symbols[0].symbol, "AAPL");
    }

    #[test]
    fn shortable_only_filter() {
        let syms = vec![
            test_symbol("SHORT", 50.0, 1e6, 0.0, true, true, true, None),
            test_symbol("NOSHORT", 50.0, 1e6, 0.0, true, false, true, None),
        ];
        let filter = ScreenerFilter { shortable_only: true, ..Default::default() };
        let result = screen_symbols(&filter, &syms);
        assert_eq!(result.total_matched, 1);
        assert_eq!(result.symbols[0].symbol, "SHORT");
    }

    #[test]
    fn combined_price_and_volume_filters() {
        let syms = vec![
            test_symbol("A", 5.0, 1e6, 0.0, true, true, true, None),
            test_symbol("B", 50.0, 100.0, 0.0, true, true, true, None),
            test_symbol("C", 50.0, 1e6, 0.0, true, true, true, None),
        ];
        let filter = ScreenerFilter {
            min_price: Some(10.0),
            min_volume: Some(1e4),
            ..Default::default()
        };
        let result = screen_symbols(&filter, &syms);
        assert_eq!(result.total_matched, 1);
        assert_eq!(result.symbols[0].symbol, "C");
    }

    #[test]
    fn result_counts_accurate() {
        let syms = vec![
            test_symbol("A", 10.0, 1e6, 0.0, true, true, true, None),
            test_symbol("B", 20.0, 1e6, 0.0, false, true, true, None),
            test_symbol("C", 30.0, 1e6, 0.0, true, true, true, None),
            test_symbol("D", 40.0, 1e6, 0.0, false, true, true, None),
        ];
        let result = screen_symbols(&ScreenerFilter::default(), &syms);
        assert_eq!(result.total_scanned, 4);
        assert_eq!(result.total_matched, 2);
        assert_eq!(result.symbols.len(), 2);
    }
}
