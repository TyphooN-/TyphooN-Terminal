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
}

// ── Relative Strength Ranking ────────────────────────────────────────

/// A symbol ranked by relative performance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelativeStrengthEntry {
    pub symbol: String,
    pub return_pct: f64,  // % return over lookback period
    pub rank: usize,       // 1 = strongest
}

/// Compute relative strength ranking from bar cache.
/// `bars_map`: symbol → Vec<(timestamp, close)> for each symbol.
/// `lookback`: number of bars to measure return over.
pub fn compute_relative_strength(
    bars_map: &std::collections::HashMap<String, Vec<f64>>,
    lookback: usize,
) -> Vec<RelativeStrengthEntry> {
    let mut entries: Vec<RelativeStrengthEntry> = bars_map.iter().filter_map(|(sym, closes)| {
        if closes.len() < lookback + 1 { return None; }
        let old = closes[closes.len() - lookback - 1];
        let new = closes[closes.len() - 1];
        if old <= 0.0 { return None; }
        let ret = (new / old - 1.0) * 100.0;
        Some(RelativeStrengthEntry { symbol: sym.clone(), return_pct: ret, rank: 0 })
    }).collect();

    entries.sort_by(|a, b| b.return_pct.partial_cmp(&a.return_pct).unwrap_or(std::cmp::Ordering::Equal));
    for (i, e) in entries.iter_mut().enumerate() {
        e.rank = i + 1;
    }
    entries
}

#[cfg(test)]
mod rs_tests {
    use super::*;

    #[test]
    fn test_relative_strength_ranking() {
        let mut bars = std::collections::HashMap::new();
        bars.insert("AAPL".into(), vec![100.0, 105.0, 110.0, 115.0, 120.0]); // +20%
        bars.insert("MSFT".into(), vec![200.0, 198.0, 195.0, 190.0, 180.0]); // -10%
        bars.insert("GOOG".into(), vec![50.0, 52.0, 55.0, 58.0, 60.0]);      // +20%
        let rs = compute_relative_strength(&bars, 4);
        assert_eq!(rs.len(), 3);
        assert_eq!(rs[0].rank, 1);
        assert!(rs[0].return_pct > 19.0); // AAPL or GOOG at ~20%
        assert_eq!(rs[2].rank, 3);
        assert!(rs[2].return_pct < 0.0); // MSFT at -10%
    }

    #[test]
    fn test_rs_insufficient_data() {
        let mut bars = std::collections::HashMap::new();
        bars.insert("SHORT".into(), vec![100.0, 105.0]); // only 2 bars, lookback=4 needs 5
        let rs = compute_relative_strength(&bars, 4);
        assert!(rs.is_empty());
    }
}

// ── Symbol Correlation Matrix ────────────────────────────────────────

/// Pairwise correlation between two symbols.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolCorrelation {
    pub symbol_a: String,
    pub symbol_b: String,
    pub correlation: f64,
    pub sample_count: usize,
}

/// Full N×N correlation matrix for a set of symbols.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationMatrix {
    pub symbols: Vec<String>,
    pub matrix: Vec<Vec<f64>>,
    pub window_bars: usize,
}

/// Compute pairwise Pearson correlation matrix from close price series.
/// `close_map`: symbol → Vec<f64> close prices (aligned by index).
/// `window`: number of most recent bars to use (0 = all).
pub fn compute_symbol_correlation_matrix(
    close_map: &std::collections::HashMap<String, Vec<f64>>,
    window: usize,
) -> CorrelationMatrix {
    let symbols: Vec<String> = close_map.keys().cloned().collect();
    let n = symbols.len();
    let mut matrix = vec![vec![0.0; n]; n];

    for i in 0..n {
        matrix[i][i] = 1.0; // self-correlation
        for j in (i + 1)..n {
            let closes_a = close_map.get(&symbols[i]);
            let closes_b = close_map.get(&symbols[j]);
            let corr = match (closes_a, closes_b) {
                (Some(a), Some(b)) => {
                    // Align by taking tail of both series
                    let len = a.len().min(b.len());
                    if len < 3 { 0.0 } else {
                        let start = if window > 0 && len > window { len - window } else { 0 };
                        let sa = &a[a.len() - len + start..];
                        let sb = &b[b.len() - len + start..];
                        // Compute returns
                        let ret_a: Vec<f64> = sa.windows(2).map(|w| if w[0] > 0.0 { w[1] / w[0] - 1.0 } else { 0.0 }).collect();
                        let ret_b: Vec<f64> = sb.windows(2).map(|w| if w[0] > 0.0 { w[1] / w[0] - 1.0 } else { 0.0 }).collect();
                        let rn = ret_a.len().min(ret_b.len());
                        if rn < 3 { 0.0 } else {
                            let mean_a = ret_a[..rn].iter().sum::<f64>() / rn as f64;
                            let mean_b = ret_b[..rn].iter().sum::<f64>() / rn as f64;
                            let mut cov = 0.0;
                            let mut var_a = 0.0;
                            let mut var_b = 0.0;
                            for k in 0..rn {
                                let da = ret_a[k] - mean_a;
                                let db = ret_b[k] - mean_b;
                                cov += da * db;
                                var_a += da * da;
                                var_b += db * db;
                            }
                            if var_a > 0.0 && var_b > 0.0 {
                                (cov / (var_a.sqrt() * var_b.sqrt())).clamp(-1.0, 1.0)
                            } else { 0.0 }
                        }
                    }
                }
                _ => 0.0,
            };
            matrix[i][j] = corr;
            matrix[j][i] = corr;
        }
    }

    CorrelationMatrix { symbols, matrix, window_bars: window }
}

#[cfg(test)]
mod corr_tests {
    use super::*;

    #[test]
    fn test_correlation_matrix_basic() {
        let mut close_map = std::collections::HashMap::new();
        close_map.insert("A".into(), vec![100.0, 102.0, 104.0, 103.0, 105.0]);
        close_map.insert("B".into(), vec![50.0, 51.0, 52.0, 51.5, 52.5]);   // correlated with A
        close_map.insert("C".into(), vec![200.0, 198.0, 196.0, 197.0, 195.0]); // inversely correlated
        let cm = compute_symbol_correlation_matrix(&close_map, 0);
        assert_eq!(cm.symbols.len(), 3);
        assert_eq!(cm.matrix.len(), 3);
        // Self-correlation = 1.0
        for i in 0..3 { assert!((cm.matrix[i][i] - 1.0).abs() < 1e-10); }
        // All values in [-1, 1]
        for row in &cm.matrix {
            for &v in row { assert!(v >= -1.0 && v <= 1.0, "Correlation out of range: {v}"); }
        }
    }

    #[test]
    fn test_correlation_matrix_empty() {
        let close_map = std::collections::HashMap::new();
        let cm = compute_symbol_correlation_matrix(&close_map, 0);
        assert!(cm.symbols.is_empty());
        assert!(cm.matrix.is_empty());
    }

    #[test]
    fn test_correlation_with_window() {
        let mut close_map = std::collections::HashMap::new();
        close_map.insert("X".into(), vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0]);
        close_map.insert("Y".into(), vec![20.0, 22.0, 24.0, 26.0, 28.0, 30.0, 32.0, 34.0, 36.0, 38.0]);
        let cm = compute_symbol_correlation_matrix(&close_map, 5);
        assert_eq!(cm.window_bars, 5);
        // Perfect positive correlation
        let x_idx = cm.symbols.iter().position(|s| s == "X").unwrap_or(0);
        let y_idx = cm.symbols.iter().position(|s| s == "Y").unwrap_or(1);
        assert!((cm.matrix[x_idx][y_idx] - 1.0).abs() < 0.01, "Expected ~1.0, got {}", cm.matrix[x_idx][y_idx]);
    }
}

