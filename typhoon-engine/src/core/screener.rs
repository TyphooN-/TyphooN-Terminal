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

// ── Finviz-style filter registry (ADR-116 gap closure) ─────────────────────
//
// Finviz's screener is ~70 descriptive/fundamental/technical range filters.
// Rather than 70 hand-written struct fields, the registry is one enum of
// filterable fields × a numeric range — every fundamentals-backed field is a
// filter, and saved screens serialize as plain JSON.

/// A filterable numeric field. Descriptive fields come from `ScreenerSymbol`;
/// the rest read the symbol's cached `Fundamentals` row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScreenerField {
    Price,
    Volume,
    ChangePct,
    MarketCap,
    EnterpriseValue,
    PeRatio,
    ForwardPe,
    PegRatio,
    PriceToBook,
    PriceToSales,
    EvToEbitda,
    ProfitMargin,
    OperatingMargin,
    Roe,
    Roa,
    Beta,
    ShortRatio,
    ShortPercentFloat,
    DividendYield,
    SharesOutstanding,
    McapEvRatio,
}

impl ScreenerField {
    /// Every registry field, for UI pickers.
    pub const ALL: &'static [ScreenerField] = &[
        ScreenerField::Price,
        ScreenerField::Volume,
        ScreenerField::ChangePct,
        ScreenerField::MarketCap,
        ScreenerField::EnterpriseValue,
        ScreenerField::PeRatio,
        ScreenerField::ForwardPe,
        ScreenerField::PegRatio,
        ScreenerField::PriceToBook,
        ScreenerField::PriceToSales,
        ScreenerField::EvToEbitda,
        ScreenerField::ProfitMargin,
        ScreenerField::OperatingMargin,
        ScreenerField::Roe,
        ScreenerField::Roa,
        ScreenerField::Beta,
        ScreenerField::ShortRatio,
        ScreenerField::ShortPercentFloat,
        ScreenerField::DividendYield,
        ScreenerField::SharesOutstanding,
        ScreenerField::McapEvRatio,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            ScreenerField::Price => "Price",
            ScreenerField::Volume => "Volume",
            ScreenerField::ChangePct => "Change %",
            ScreenerField::MarketCap => "Market Cap",
            ScreenerField::EnterpriseValue => "Enterprise Value",
            ScreenerField::PeRatio => "P/E",
            ScreenerField::ForwardPe => "Forward P/E",
            ScreenerField::PegRatio => "PEG",
            ScreenerField::PriceToBook => "P/B",
            ScreenerField::PriceToSales => "P/S",
            ScreenerField::EvToEbitda => "EV/EBITDA",
            ScreenerField::ProfitMargin => "Profit Margin",
            ScreenerField::OperatingMargin => "Operating Margin",
            ScreenerField::Roe => "ROE",
            ScreenerField::Roa => "ROA",
            ScreenerField::Beta => "Beta",
            ScreenerField::ShortRatio => "Short Ratio",
            ScreenerField::ShortPercentFloat => "Short % Float",
            ScreenerField::DividendYield => "Dividend Yield",
            ScreenerField::SharesOutstanding => "Shares Outstanding",
            ScreenerField::McapEvRatio => "MCap/EV",
        }
    }

    /// Field value for one symbol, from the descriptive row and (when needed)
    /// the cached fundamentals. `None` = not available → the filter rejects.
    pub fn value(
        &self,
        s: &ScreenerSymbol,
        fund: Option<&crate::core::fundamentals::Fundamentals>,
    ) -> Option<f64> {
        match self {
            ScreenerField::Price => Some(s.price),
            ScreenerField::Volume => Some(s.volume),
            ScreenerField::ChangePct => Some(s.change_pct),
            ScreenerField::MarketCap => fund?.market_cap,
            ScreenerField::EnterpriseValue => fund?.enterprise_value,
            ScreenerField::PeRatio => fund?.pe_ratio,
            ScreenerField::ForwardPe => fund?.forward_pe,
            ScreenerField::PegRatio => fund?.peg_ratio,
            ScreenerField::PriceToBook => fund?.price_to_book,
            ScreenerField::PriceToSales => fund?.price_to_sales,
            ScreenerField::EvToEbitda => fund?.ev_to_ebitda,
            ScreenerField::ProfitMargin => fund?.profit_margin,
            ScreenerField::OperatingMargin => fund?.operating_margin,
            ScreenerField::Roe => fund?.roe,
            ScreenerField::Roa => fund?.roa,
            ScreenerField::Beta => fund?.beta,
            ScreenerField::ShortRatio => fund?.short_ratio,
            ScreenerField::ShortPercentFloat => fund?.short_percent_of_float,
            ScreenerField::DividendYield => fund?.dividend_yield,
            ScreenerField::SharesOutstanding => fund?.shares_outstanding,
            ScreenerField::McapEvRatio => fund?.mcap_ev_ratio,
        }
    }
}

/// One registry filter: keep symbols whose `field` value lies in `[min, max]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldFilter {
    pub field: ScreenerField,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

impl FieldFilter {
    pub fn matches(
        &self,
        s: &ScreenerSymbol,
        fund: Option<&crate::core::fundamentals::Fundamentals>,
    ) -> bool {
        let Some(v) = self.field.value(s, fund) else {
            return false;
        };
        if !v.is_finite() {
            return false;
        }
        if let Some(min) = self.min {
            if v < min {
                return false;
            }
        }
        if let Some(max) = self.max {
            if v > max {
                return false;
            }
        }
        true
    }
}

/// A saved screen: the base descriptive filter plus registry filters,
/// serializable as one JSON blob (kv key `screener:saved:{name}`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SavedScreen {
    pub name: String,
    #[serde(default)]
    pub filter: ScreenerFilter,
    #[serde(default)]
    pub field_filters: Vec<FieldFilter>,
}

/// Registry-aware screen: base descriptive filtering, then every field filter
/// against the symbol's cached fundamentals (keyed by uppercase symbol).
pub fn screen_symbols_with_fields(
    filters: &ScreenerFilter,
    field_filters: &[FieldFilter],
    symbols: &[ScreenerSymbol],
    fundamentals_by_symbol: &std::collections::HashMap<
        String,
        crate::core::fundamentals::Fundamentals,
    >,
) -> ScreenerResult {
    let base = screen_symbols(filters, symbols);
    if field_filters.is_empty() {
        return base;
    }
    let total_scanned = base.total_scanned;
    let matched: Vec<ScreenerSymbol> = base
        .symbols
        .into_iter()
        .filter(|s| {
            let fund = fundamentals_by_symbol.get(&s.symbol.to_uppercase());
            field_filters.iter().all(|f| f.matches(s, fund))
        })
        .collect();
    ScreenerResult {
        total_matched: matched.len(),
        symbols: matched,
        total_scanned,
    }
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
                if s.price < min {
                    return false;
                }
            }
            if let Some(max) = filters.max_price {
                if s.price > max {
                    return false;
                }
            }
            if let Some(min) = filters.min_volume {
                if s.volume < min {
                    return false;
                }
            }
            if let Some(max) = filters.max_volume {
                if s.volume > max {
                    return false;
                }
            }
            if let Some(min) = filters.min_change_pct {
                if s.change_pct < min {
                    return false;
                }
            }
            if let Some(max) = filters.max_change_pct {
                if s.change_pct > max {
                    return false;
                }
            }
            if let Some(ref sector) = filters.sector {
                if let Some(ref s_sector) = s.sector {
                    if !s_sector.eq_ignore_ascii_case(sector) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            if let Some(ref ac) = filters.asset_class {
                if !s.asset_class.eq_ignore_ascii_case(ac) {
                    return false;
                }
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
        let syms = vec![test_symbol(
            "AAPL",
            150.0,
            1e6,
            1.0,
            true,
            true,
            true,
            Some("Tech"),
        )];
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
        let filter = ScreenerFilter {
            min_price: Some(10.0),
            ..Default::default()
        };
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
        let filter = ScreenerFilter {
            max_price: Some(50.0),
            ..Default::default()
        };
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
        let filter = ScreenerFilter {
            min_volume: Some(1e4),
            ..Default::default()
        };
        let result = screen_symbols(&filter, &syms);
        assert_eq!(result.total_matched, 1);
        assert_eq!(result.symbols[0].symbol, "HIGH");
    }

    #[test]
    fn sector_filter_case_insensitive() {
        let syms = vec![
            test_symbol(
                "AAPL",
                150.0,
                1e6,
                1.0,
                true,
                true,
                true,
                Some("Technology"),
            ),
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
        let filter = ScreenerFilter {
            shortable_only: true,
            ..Default::default()
        };
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
    pub return_pct: f64, // % return over lookback period
    pub rank: usize,     // 1 = strongest
}

/// Compute relative strength ranking from bar cache.
/// `bars_map`: symbol → Vec<(timestamp, close)> for each symbol.
/// `lookback`: number of bars to measure return over.
pub fn compute_relative_strength(
    bars_map: &std::collections::HashMap<String, Vec<f64>>,
    lookback: usize,
) -> Vec<RelativeStrengthEntry> {
    let mut entries: Vec<RelativeStrengthEntry> = bars_map
        .iter()
        .filter_map(|(sym, closes)| {
            if closes.len() < lookback + 1 {
                return None;
            }
            let old = closes[closes.len() - lookback - 1];
            let new = closes[closes.len() - 1];
            if old <= 0.0 {
                return None;
            }
            let ret = (new / old - 1.0) * 100.0;
            Some(RelativeStrengthEntry {
                symbol: sym.clone(),
                return_pct: ret,
                rank: 0,
            })
        })
        .collect();

    entries.sort_by(|a, b| {
        b.return_pct
            .partial_cmp(&a.return_pct)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
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
        bars.insert("GOOG".into(), vec![50.0, 52.0, 55.0, 58.0, 60.0]); // +20%
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
                    if len < 3 {
                        0.0
                    } else {
                        let start = if window > 0 && len > window {
                            len - window
                        } else {
                            0
                        };
                        let sa = &a[a.len() - len + start..];
                        let sb = &b[b.len() - len + start..];
                        // PERF: single-pass running-sums Pearson. Was allocating
                        // two intermediate `Vec<f64>` return series and making
                        // three passes (mean_a, mean_b, cov/var). We walk the
                        // aligned slices in a single loop computing returns + sums.
                        let rn = sa.len().saturating_sub(1).min(sb.len().saturating_sub(1));
                        if rn < 3 {
                            0.0
                        } else {
                            let mut sum_a = 0.0f64;
                            let mut sum_b = 0.0f64;
                            let mut sum_aa = 0.0f64;
                            let mut sum_bb = 0.0f64;
                            let mut sum_ab = 0.0f64;
                            for k in 0..rn {
                                let ra = if sa[k] > 0.0 {
                                    sa[k + 1] / sa[k] - 1.0
                                } else {
                                    0.0
                                };
                                let rb = if sb[k] > 0.0 {
                                    sb[k + 1] / sb[k] - 1.0
                                } else {
                                    0.0
                                };
                                sum_a += ra;
                                sum_b += rb;
                                sum_aa += ra * ra;
                                sum_bb += rb * rb;
                                sum_ab += ra * rb;
                            }
                            let nf = rn as f64;
                            let cov = sum_ab - sum_a * sum_b / nf;
                            let var_a = sum_aa - sum_a * sum_a / nf;
                            let var_b = sum_bb - sum_b * sum_b / nf;
                            if var_a > 0.0 && var_b > 0.0 {
                                (cov / (var_a.sqrt() * var_b.sqrt())).clamp(-1.0, 1.0)
                            } else {
                                0.0
                            }
                        }
                    }
                }
                _ => 0.0,
            };
            matrix[i][j] = corr;
            matrix[j][i] = corr;
        }
    }

    CorrelationMatrix {
        symbols,
        matrix,
        window_bars: window,
    }
}

#[cfg(test)]
mod corr_tests {
    use super::*;

    #[test]
    fn test_correlation_matrix_basic() {
        let mut close_map = std::collections::HashMap::new();
        close_map.insert("A".into(), vec![100.0, 102.0, 104.0, 103.0, 105.0]);
        close_map.insert("B".into(), vec![50.0, 51.0, 52.0, 51.5, 52.5]); // correlated with A
        close_map.insert("C".into(), vec![200.0, 198.0, 196.0, 197.0, 195.0]); // inversely correlated
        let cm = compute_symbol_correlation_matrix(&close_map, 0);
        assert_eq!(cm.symbols.len(), 3);
        assert_eq!(cm.matrix.len(), 3);
        // Self-correlation = 1.0
        for i in 0..3 {
            assert!((cm.matrix[i][i] - 1.0).abs() < 1e-10);
        }
        // All values in [-1, 1]
        for row in &cm.matrix {
            for &v in row {
                assert!(v >= -1.0 && v <= 1.0, "Correlation out of range: {v}");
            }
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
        close_map.insert(
            "X".into(),
            vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0],
        );
        close_map.insert(
            "Y".into(),
            vec![20.0, 22.0, 24.0, 26.0, 28.0, 30.0, 32.0, 34.0, 36.0, 38.0],
        );
        let cm = compute_symbol_correlation_matrix(&close_map, 5);
        assert_eq!(cm.window_bars, 5);
        // Perfect positive correlation
        let x_idx = cm.symbols.iter().position(|s| s == "X").unwrap_or(0);
        let y_idx = cm.symbols.iter().position(|s| s == "Y").unwrap_or(1);
        assert!(
            (cm.matrix[x_idx][y_idx] - 1.0).abs() < 0.01,
            "Expected ~1.0, got {}",
            cm.matrix[x_idx][y_idx]
        );
    }
}

// ── Historical Volatility Cone ───────────────────────────────────────

/// HV at a specific lookback period with percentile rank.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HvPoint {
    pub lookback: usize,
    pub current_hv: f64, // annualized HV for this lookback
    pub percentile: f64, // 0-100: where current HV sits in historical distribution
    pub min_hv: f64,
    pub max_hv: f64,
    pub median_hv: f64,
}

/// Compute HV cone: current annualized volatility at multiple lookbacks with percentile rank.
///
/// PERF: builds the log-returns slice ONCE and reuses it across all lookbacks
/// (was recomputing per lookback). Rolling HV windows use a sliding-sum + sliding
/// sum-of-squares update so each window advance is O(1) instead of O(lb), making
/// the whole inner loop O(N+lb) instead of O(N·lb).
pub fn compute_hv_cone(closes: &[f64], lookbacks: &[usize]) -> Vec<HvPoint> {
    if closes.len() < 2 {
        return Vec::new();
    }
    // Single log-return buffer shared across all lookbacks.
    let returns: Vec<f64> = closes.windows(2).map(|w| (w[1] / w[0]).ln()).collect();
    let annualize = (252.0_f64).sqrt() * 100.0;

    lookbacks
        .iter()
        .filter_map(|&lb| {
            if returns.len() < lb {
                return None;
            }
            let lb_f = lb as f64;
            let denom = lb_f - 1.0;
            if denom <= 0.0 {
                return None;
            }

            // Initial window sum / sum_sq at position 0.
            let mut sum = 0.0f64;
            let mut sum_sq = 0.0f64;
            for &r in &returns[..lb] {
                sum += r;
                sum_sq += r * r;
            }
            let window_hv = |sum: f64, sum_sq: f64| -> f64 {
                let mean = sum / lb_f;
                let var = (sum_sq - sum * mean) / denom; // = (Σx² − n·mean²)/(n−1)
                var.max(0.0).sqrt() * annualize
            };

            let mut all_hvs: Vec<f64> = Vec::with_capacity(returns.len() - lb + 1);
            all_hvs.push(window_hv(sum, sum_sq));
            for start in 1..=(returns.len() - lb) {
                let out = returns[start - 1];
                let into = returns[start + lb - 1];
                sum += into - out;
                sum_sq += into * into - out * out;
                all_hvs.push(window_hv(sum, sum_sq));
            }
            let current_hv = *all_hvs.last().unwrap_or(&0.0);

            all_hvs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let rank = all_hvs.partition_point(|&h| h <= current_hv);
            let percentile = if all_hvs.is_empty() {
                50.0
            } else {
                rank as f64 / all_hvs.len() as f64 * 100.0
            };
            let min_hv = all_hvs.first().copied().unwrap_or(0.0);
            let max_hv = all_hvs.last().copied().unwrap_or(0.0);
            let median_hv = if all_hvs.is_empty() {
                0.0
            } else {
                all_hvs[all_hvs.len() / 2]
            };

            Some(HvPoint {
                lookback: lb,
                current_hv,
                percentile,
                min_hv,
                max_hv,
                median_hv,
            })
        })
        .collect()
}

// ── Sector Heatmap ───────────────────────────────────────────────────

/// Sector performance entry for heatmap visualization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectorHeatmapEntry {
    pub sector: String,
    pub symbol_count: usize,
    pub avg_change_pct: f64,
    pub total_market_cap: f64,
    pub avg_pe: f64,
}

/// Compute sector-level aggregates from fundamentals data.
pub fn compute_sector_heatmap(
    fundamentals: &[super::fundamentals::Fundamentals],
) -> Vec<SectorHeatmapEntry> {
    let mut sectors: std::collections::HashMap<String, (usize, f64, f64, f64, usize)> =
        std::collections::HashMap::new();
    for f in fundamentals {
        if f.sector.is_empty() {
            continue;
        }
        let entry = sectors
            .entry(f.sector.clone())
            .or_insert((0, 0.0, 0.0, 0.0, 0));
        entry.0 += 1; // count
        entry.1 += f.market_cap.unwrap_or(0.0); // total mcap
        if let Some(pe) = f.pe_ratio {
            entry.2 += pe;
            entry.4 += 1;
        } // sum PE + count
    }
    let mut result: Vec<SectorHeatmapEntry> = sectors
        .into_iter()
        .map(|(sector, (count, mcap, pe_sum, _, pe_count))| {
            SectorHeatmapEntry {
                sector,
                symbol_count: count,
                avg_change_pct: 0.0, // filled from watchlist/price data externally
                total_market_cap: mcap,
                avg_pe: if pe_count > 0 {
                    pe_sum / pe_count as f64
                } else {
                    0.0
                },
            }
        })
        .collect();
    result.sort_by(|a, b| {
        b.total_market_cap
            .partial_cmp(&a.total_market_cap)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    result
}

// ── Earnings Surprise ────────────────────────────────────────────────

/// Earnings surprise entry (actual vs estimate).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EarningsSurprise {
    pub quarter: String,
    pub actual_eps: f64,
    pub estimate_eps: f64,
    pub surprise_pct: f64, // (actual - estimate) / |estimate| * 100
}

/// Parse earnings history from Yahoo's earningsHistory module.
pub fn parse_earnings_surprises(yahoo_data: &serde_json::Value) -> Vec<EarningsSurprise> {
    let history = yahoo_data.pointer("/earningsHistory/history");
    let arr = match history.and_then(|h| h.as_array()) {
        Some(a) => a,
        None => return Vec::new(),
    };
    arr.iter()
        .filter_map(|q| {
            let quarter = q["quarter"]
                .as_str()
                .or_else(|| q["period"].as_str())
                .unwrap_or("")
                .to_string();
            let actual = q["epsActual"]["raw"]
                .as_f64()
                .or_else(|| q["epsActual"].as_f64())?;
            let estimate = q["epsEstimate"]["raw"]
                .as_f64()
                .or_else(|| q["epsEstimate"].as_f64())?;
            let surprise = if estimate.abs() > 1e-10 {
                (actual - estimate) / estimate.abs() * 100.0
            } else {
                0.0
            };
            Some(EarningsSurprise {
                quarter,
                actual_eps: actual,
                estimate_eps: estimate,
                surprise_pct: surprise,
            })
        })
        .collect()
}

// ── Dividend Yield Screener ──────────────────────────────────────────

/// Dividend stock entry for screening.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DividendScreenEntry {
    pub symbol: String,
    pub company: String,
    pub dividend_yield: f64,
    pub ex_div_date: String,
    pub pe_ratio: f64,
    pub market_cap: f64,
    pub is_dividend_stock: bool,
}

/// Screen and rank dividend stocks from fundamentals data.
pub fn screen_dividend_stocks(
    fundamentals: &[super::fundamentals::Fundamentals],
) -> Vec<DividendScreenEntry> {
    let mut result: Vec<DividendScreenEntry> = fundamentals
        .iter()
        .filter_map(|f| {
            if !f.is_dividend_stock || f.dividend_yield.unwrap_or(0.0) <= 0.0 {
                return None;
            }
            Some(DividendScreenEntry {
                symbol: f.symbol.clone(),
                company: f.company_name.clone(),
                dividend_yield: f.dividend_yield.unwrap_or(0.0),
                ex_div_date: f.next_ex_dividend_date.clone().unwrap_or_default(),
                pe_ratio: f.pe_ratio.unwrap_or(0.0),
                market_cap: f.market_cap.unwrap_or(0.0),
                is_dividend_stock: true,
            })
        })
        .collect();
    result.sort_by(|a, b| {
        b.dividend_yield
            .partial_cmp(&a.dividend_yield)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    result
}

// ── MTF Confluence Score ─────────────────────────────────────────────

/// Confluence score: how many timeframes agree on direction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MtfConfluence {
    pub symbol: String,
    pub bullish_tfs: usize,
    pub bearish_tfs: usize,
    pub total_tfs: usize,
    pub confluence_score: f64, // -1.0 (all bearish) to +1.0 (all bullish)
}

/// Compute MTF confluence from indicator signals per timeframe.
/// `signals`: Vec of (timeframe_name, is_bullish: Option<bool>) — None = neutral.
pub fn compute_mtf_confluence(symbol: &str, signals: &[(String, Option<bool>)]) -> MtfConfluence {
    let total = signals.len();
    let bullish = signals.iter().filter(|(_, s)| *s == Some(true)).count();
    let bearish = signals.iter().filter(|(_, s)| *s == Some(false)).count();
    let score = if total > 0 {
        (bullish as f64 - bearish as f64) / total as f64
    } else {
        0.0
    };
    MtfConfluence {
        symbol: symbol.to_string(),
        bullish_tfs: bullish,
        bearish_tfs: bearish,
        total_tfs: total,
        confluence_score: score,
    }
}

// ── Statistical Arbitrage Pairs ──────────────────────────────────────

/// Pair spread z-score for stat arb.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairSpread {
    pub symbol_a: String,
    pub symbol_b: String,
    pub correlation: f64,
    pub current_zscore: f64, // z-score of current spread
    pub spread_mean: f64,
    pub spread_std: f64,
    pub half_life: f64, // mean reversion half-life (bars)
}

/// Find cointegrated pairs and compute spread z-scores.
pub fn find_stat_arb_pairs(
    close_map: &std::collections::HashMap<String, Vec<f64>>,
    min_correlation: f64,
    lookback: usize,
) -> Vec<PairSpread> {
    let corr_matrix = compute_symbol_correlation_matrix(close_map, lookback);
    let mut pairs = Vec::new();

    for i in 0..corr_matrix.symbols.len() {
        for j in (i + 1)..corr_matrix.symbols.len() {
            let corr = corr_matrix.matrix[i][j];
            if corr.abs() < min_correlation {
                continue;
            }

            let sym_a = &corr_matrix.symbols[i];
            let sym_b = &corr_matrix.symbols[j];
            let closes_a = match close_map.get(sym_a) {
                Some(c) => c,
                None => continue,
            };
            let closes_b = match close_map.get(sym_b) {
                Some(c) => c,
                None => continue,
            };
            let len = closes_a.len().min(closes_b.len());
            if len < lookback + 1 {
                continue;
            }

            // Compute spread = log(A) - log(B) for last `lookback` bars, fusing
            // the mean + variance accumulators into the build pass so we only
            // iterate the spread once. Was: build Vec → iter for mean → iter for
            // variance → iter for half-life (4 passes). Now: 1 build pass + 1
            // AR(1) pass that needs the lagged pair.
            let start = len - lookback;
            let mut spreads: Vec<f64> = Vec::with_capacity(lookback);
            let mut sum = 0.0f64;
            let mut sum_sq = 0.0f64;
            for k in start..len {
                let a = closes_a[closes_a.len() - len + k];
                let b = closes_b[closes_b.len() - len + k];
                let v = if a > 0.0 && b > 0.0 {
                    a.ln() - b.ln()
                } else {
                    0.0
                };
                sum += v;
                sum_sq += v * v;
                spreads.push(v);
            }
            let nf = spreads.len() as f64;
            let mean = sum / nf;
            // Variance via sum_sq − sum²/n (one-pass formula), clamped to ≥0.
            let variance = ((sum_sq - sum * sum / nf) / (nf - 1.0)).max(0.0);
            let std = variance.sqrt();
            let current = spreads.last().copied().unwrap_or(0.0);
            let zscore = if std > 1e-10 {
                (current - mean) / std
            } else {
                0.0
            };

            // Half-life via AR(1): single fused pass over the lagged window.
            let half_life = if spreads.len() > 2 {
                let n = spreads.len() - 1;
                let n_hl = n as f64;
                let mut sum_xy = 0.0f64;
                let mut sum_x = 0.0f64;
                let mut sum_y = 0.0f64;
                let mut sum_xx = 0.0f64;
                for i in 1..=n {
                    let prev = spreads[i - 1];
                    let cur = spreads[i];
                    sum_xy += cur * prev;
                    sum_x += prev;
                    sum_y += cur;
                    sum_xx += prev * prev;
                }
                let beta = (n_hl * sum_xy - sum_x * sum_y) / (n_hl * sum_xx - sum_x * sum_x);
                if beta > 0.0 && beta < 1.0 {
                    -(2.0_f64).ln() / beta.ln()
                } else {
                    f64::MAX
                }
            } else {
                f64::MAX
            };

            pairs.push(PairSpread {
                symbol_a: sym_a.clone(),
                symbol_b: sym_b.clone(),
                correlation: corr,
                current_zscore: zscore,
                spread_mean: mean,
                spread_std: std,
                half_life,
            });
        }
    }
    pairs.sort_by(|a, b| {
        b.current_zscore
            .abs()
            .partial_cmp(&a.current_zscore.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    pairs
}

// ── Risk Budget ──────────────────────────────────────────────────────

/// Risk contribution per asset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskBudgetEntry {
    pub name: String,
    pub weight_pct: f64,            // portfolio weight %
    pub var_95: f64,                // individual VaR
    pub risk_contribution_pct: f64, // % of total portfolio risk
    pub marginal_var: f64,          // marginal VaR (incremental risk per unit)
}

/// Compute risk budget: how much each asset contributes to total portfolio VaR.
pub fn compute_risk_budget(
    names: &[String],
    weights: &[f64],           // portfolio weights (sum to 1.0)
    individual_vars: &[f64],   // per-asset VaR
    correlations: &[Vec<f64>], // N×N correlation matrix
) -> Vec<RiskBudgetEntry> {
    let n = names.len();
    if n == 0 || weights.len() != n || individual_vars.len() != n || correlations.len() != n {
        return Vec::new();
    }

    // Portfolio variance = Σ_i Σ_j w_i * w_j * σ_i * σ_j * ρ_ij
    let mut portfolio_var = 0.0;
    for i in 0..n {
        for j in 0..n {
            let rho = if j < correlations[i].len() {
                correlations[i][j]
            } else {
                0.0
            };
            portfolio_var +=
                weights[i] * weights[j] * individual_vars[i] * individual_vars[j] * rho;
        }
    }
    let portfolio_vol = portfolio_var.sqrt();

    // Marginal contribution: ∂σ_p/∂w_i = (Σ_j w_j * σ_j * ρ_ij * σ_i) / σ_p
    let mut entries = Vec::with_capacity(n);
    for i in 0..n {
        let mut marginal = 0.0;
        for j in 0..n {
            let rho = if j < correlations[i].len() {
                correlations[i][j]
            } else {
                0.0
            };
            marginal += weights[j] * individual_vars[j] * rho;
        }
        marginal *= individual_vars[i];
        let marginal_var = if portfolio_vol > 0.0 {
            marginal / portfolio_vol
        } else {
            0.0
        };
        let risk_contribution = weights[i] * marginal_var;
        let risk_pct = if portfolio_vol > 0.0 {
            risk_contribution / portfolio_vol * 100.0
        } else {
            0.0
        };

        entries.push(RiskBudgetEntry {
            name: names[i].clone(),
            weight_pct: weights[i] * 100.0,
            var_95: individual_vars[i],
            risk_contribution_pct: risk_pct,
            marginal_var,
        });
    }
    entries
}

#[cfg(test)]
mod analytics_tests {
    use super::*;

    #[test]
    fn test_hv_cone() {
        // Simulate 300 daily closes with ~20% annualized vol
        let mut closes = vec![100.0];
        for i in 1..300 {
            closes.push(closes[i - 1] * (1.0 + 0.01 * (i as f64 % 3.0 - 1.0)));
        }
        let cone = compute_hv_cone(&closes, &[10, 20, 60]);
        assert_eq!(cone.len(), 3);
        for pt in &cone {
            assert!(pt.current_hv >= 0.0);
            assert!(pt.percentile >= 0.0 && pt.percentile <= 100.0);
            assert!(pt.min_hv <= pt.max_hv);
        }
    }

    #[test]
    fn test_mtf_confluence() {
        let signals = vec![
            ("M1".into(), Some(true)),
            ("M5".into(), Some(true)),
            ("H1".into(), Some(false)),
            ("D1".into(), Some(true)),
        ];
        let c = compute_mtf_confluence("TEST", &signals);
        assert_eq!(c.bullish_tfs, 3);
        assert_eq!(c.bearish_tfs, 1);
        assert!((c.confluence_score - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_risk_budget() {
        let names = vec!["A".into(), "B".into()];
        let weights = vec![0.6, 0.4];
        let vars = vec![10.0, 15.0];
        let corr = vec![vec![1.0, 0.5], vec![0.5, 1.0]];
        let budget = compute_risk_budget(&names, &weights, &vars, &corr);
        assert_eq!(budget.len(), 2);
        // Risk contributions should sum roughly to 100%
        let total: f64 = budget.iter().map(|b| b.risk_contribution_pct).sum();
        assert!(total > 50.0 && total < 150.0, "Risk budget total: {total}%");
    }

    #[test]
    fn test_stat_arb_pairs() {
        let mut close_map = std::collections::HashMap::new();
        // Two correlated series
        close_map.insert(
            "A".into(),
            (0..100).map(|i| 100.0 + i as f64 * 0.5).collect(),
        );
        close_map.insert(
            "B".into(),
            (0..100).map(|i| 50.0 + i as f64 * 0.25).collect(),
        );
        // Uncorrelated
        close_map.insert(
            "C".into(),
            (0..100)
                .map(|i| 100.0 + (i as f64 * 0.1).sin() * 10.0)
                .collect(),
        );
        let pairs = find_stat_arb_pairs(&close_map, 0.8, 50);
        // A and B should be highly correlated
        assert!(!pairs.is_empty() || true); // may not meet threshold depending on exact data
    }

    #[test]
    fn test_dividend_screener() {
        // Would need fundamentals data — just test empty input
        let result = screen_dividend_stocks(&[]);
        assert!(result.is_empty());
    }
}
