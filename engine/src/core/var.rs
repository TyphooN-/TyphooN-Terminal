//! Value at Risk calculation.
//!
//! Port of DWEX Portfolio Risk Man v1.06 from MQL5.
//! Inline StdDev, configurable confidence level.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaRResult {
    pub var_dollars: f64,
    pub std_dev_returns: f64,
    pub nominal_value: f64,
    pub z_score: f64,
}

/// Compute VaR per lot for a symbol from daily close prices.
/// Matches DWEX Portfolio Risk Man formula:
///   VaR_1_Lot = z(confidence) × σ(daily_returns) × nominalValue
///   nominalValue = (tickValue / tickSize) × closePrice
/// If tick_value_per_tick_size is 0 or not available, uses 1.0 (pure price VaR).
/// Returns (var_1_lot, var_to_price_ratio_pct) or None if insufficient data.
pub fn compute_var_from_closes(closes: &[f64], confidence: f64) -> Option<(f64, f64)> {
    compute_var_from_closes_with_tick(closes, confidence, 1.0)
}

/// Full VaR calculation with tick value scaling (tickValue / tickSize).
pub fn compute_var_from_closes_with_tick(
    closes: &[f64],
    confidence: f64,
    tick_value_per_tick_size: f64,
) -> Option<(f64, f64)> {
    if closes.len() < 10 {
        return None;
    }
    let returns: Vec<f64> = closes
        .windows(2)
        .filter(|w| w[0] > 0.0 && w[1] > 0.0)
        .map(|w| w[1] / w[0] - 1.0)
        .collect();
    if returns.len() < 5 {
        return None;
    }
    let sd = std_dev(&returns);
    if sd <= 0.0 {
        return None;
    }
    let z = inverse_cumulative_normal(confidence);
    let last_price = closes.last().copied().unwrap_or(0.0);
    if last_price <= 0.0 {
        return None;
    }
    let nominal = tick_value_per_tick_size.max(1.0) * last_price;
    let var_1_lot = z * sd * nominal;
    let ratio = var_1_lot / last_price; // VaR/Ask ratio (as used by MarketWizardry.org)
    Some((var_1_lot, ratio * 100.0))
}

/// Population standard deviation (matches MQL5 MathStandardDeviation).
pub fn std_dev(values: &[f64]) -> f64 {
    let n = values.len();
    if n < 2 {
        return 0.0;
    }
    let n_f = n as f64;
    let mut sum = 0.0;
    let mut sum_sq = 0.0;
    for &v in values {
        sum += v;
        sum_sq += v * v;
    }
    let mean = sum / n_f;
    let variance = (sum_sq / n_f) - (mean * mean);
    if variance > 0.0 { variance.sqrt() } else { 0.0 }
}

/// Rational approximation of the inverse cumulative normal distribution.
/// Port of InverseCumulativeNormal from DWEX Portfolio Risk Man.
pub fn inverse_cumulative_normal(p: f64) -> f64 {
    const A: [f64; 6] = [
        -39.6968302866538,
        220.946098424521,
        -275.928510446969,
        138.357751867269,
        -30.6647980661472,
        2.50662827745924,
    ];
    const B: [f64; 5] = [
        -54.4760987982241,
        161.585836858041,
        -155.698979859887,
        66.8013118877197,
        -13.2806815528857,
    ];
    const C: [f64; 6] = [
        -0.00778489400243029,
        -0.322396458041136,
        -2.40075827716184,
        -2.54973253934373,
        4.37466414146497,
        2.93816398269878,
    ];
    const D: [f64; 4] = [
        0.00778469570904146,
        0.32246712907004,
        2.445134137143,
        3.75440866190742,
    ];
    const P_LOW: f64 = 0.02425;
    const P_HIGH: f64 = 1.0 - P_LOW;

    if p > 0.0 && p < P_LOW {
        let q = (-2.0 * p.ln()).sqrt();
        (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    } else if p >= P_LOW && p <= P_HIGH {
        let q = p - 0.5;
        let r = q * q;
        (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q
            / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0)
    } else if p > P_HIGH && p < 1.0 {
        let q = (-2.0 * (1.0 - p).ln()).sqrt();
        -(((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    } else {
        0.0
    }
}

/// Compute daily returns from close prices.
/// Shared by calculate_var and lot_size_from_var to avoid duplication.
pub fn compute_daily_returns(close_prices: &[f64]) -> Vec<f64> {
    let mut returns = Vec::with_capacity(close_prices.len().saturating_sub(1));
    for w in close_prices.windows(2) {
        if w[0] == 0.0 || w[1] == 0.0 {
            returns.push(0.0);
        } else {
            returns.push((w[1] / w[0]) - 1.0);
        }
    }
    returns
}

/// Calculate VaR for a single position.
///
/// Port of CPortfolioRiskMan::CalculateVaR.
pub fn calculate_var(
    close_prices: &[f64],
    position_size: f64,
    tick_value: f64,
    tick_size: f64,
    current_price: f64,
    confidence: f64,
) -> Option<VaRResult> {
    if close_prices.len() < 2 || tick_size <= 0.0 || current_price <= 0.0 {
        return None;
    }

    let daily_returns = compute_daily_returns(close_prices);
    let sd = std_dev(&daily_returns);
    if sd == 0.0 || !sd.is_finite() {
        return None;
    }

    let nominal_per_unit = tick_value / tick_size;
    let nominal_value = position_size.abs() * nominal_per_unit * current_price;
    let z_score = inverse_cumulative_normal(confidence);
    if z_score == 0.0 {
        return None;
    }

    Some(VaRResult {
        var_dollars: z_score * sd * nominal_value,
        std_dev_returns: sd,
        nominal_value,
        z_score,
    })
}

/// Calculate lot size to achieve target VaR percentage.
///
/// Port of CPortfolioRiskMan::CalculateLotSizeBasedOnVaR.
pub fn lot_size_from_var(
    close_prices: &[f64],
    tick_value: f64,
    tick_size: f64,
    current_price: f64,
    confidence: f64,
    equity: f64,
    var_pct: f64,
) -> Option<f64> {
    if close_prices.len() < 2 || tick_size <= 0.0 || current_price <= 0.0 || equity <= 0.0 {
        return None;
    }

    let daily_returns = compute_daily_returns(close_prices);
    let sd = std_dev(&daily_returns);
    if sd == 0.0 || !sd.is_finite() {
        return None;
    }

    let nominal_per_unit = tick_value / tick_size;
    if nominal_per_unit <= 0.0 {
        return None;
    }

    let z_score = inverse_cumulative_normal(confidence);
    if z_score == 0.0 {
        return None;
    }

    let unit_var = z_score * sd * nominal_per_unit * current_price;
    if unit_var < 1e-10 {
        return None;
    }

    let max_var = (var_pct / 100.0) * equity;
    Some(max_var / unit_var)
}

/// Calculate portfolio-aware lot size for a new position.
///
/// Instead of treating each position in isolation, this considers the
/// existing portfolio's VaR and computes how many lots of the new symbol
/// can be added before the TOTAL portfolio VaR exceeds the target budget.
///
/// Marginal VaR = change in portfolio VaR from adding one lot of the new symbol.
/// Max lots = remaining VaR budget / marginal VaR per lot.
///
/// The correlation effect is approximated: if the new position has low
/// correlation with existing positions, more lots are allowed (diversification
/// benefit). If highly correlated, fewer lots.
pub fn portfolio_aware_lot_size(
    new_symbol_closes: &[f64],
    existing_positions: &[(Vec<f64>, f64, f64)], // (close_prices, position_size, current_price) per existing position
    tick_value: f64,
    tick_size: f64,
    new_price: f64,
    confidence: f64,
    equity: f64,
    var_pct_target: f64, // target total portfolio VaR as % of equity
) -> Option<(f64, f64, f64)> {
    // (lots, marginal_var_per_lot, total_portfolio_var)
    if new_symbol_closes.len() < 2 || tick_size <= 0.0 || new_price <= 0.0 || equity <= 0.0 {
        return None;
    }

    // 1. Compute existing portfolio VaR (sum of individual VaRs with correlation adjustment)
    let mut existing_total_var = 0.0;
    let mut existing_return_series: Vec<Vec<f64>> = Vec::new();

    for (closes, size, price) in existing_positions {
        if let Some(var_result) =
            calculate_var(closes, *size, tick_value, tick_size, *price, confidence)
        {
            existing_total_var += var_result.var_dollars;
        }
        existing_return_series.push(compute_daily_returns(closes));
    }

    // 2. Compute VaR for 1 lot of the new symbol
    let new_returns = compute_daily_returns(new_symbol_closes);
    let sd_new = std_dev(&new_returns);
    if sd_new == 0.0 || !sd_new.is_finite() {
        return None;
    }

    let z = inverse_cumulative_normal(confidence);
    if z == 0.0 {
        return None;
    }

    let nominal_per_unit = tick_value / tick_size;
    let unit_var = z * sd_new * nominal_per_unit * new_price;
    if unit_var < 1e-10 {
        return None;
    }

    // 3. Compute average correlation of new symbol with existing positions
    let avg_corr = if existing_return_series.is_empty() {
        0.0 // no existing positions = no correlation
    } else {
        let mut corr_sum = 0.0;
        let mut corr_count = 0;
        for existing_returns in &existing_return_series {
            let c = pearson_correlation(&new_returns, existing_returns);
            if c.is_finite() {
                corr_sum += c.abs(); // use absolute correlation
                corr_count += 1;
            }
        }
        if corr_count > 0 {
            corr_sum / corr_count as f64
        } else {
            0.0
        }
    };

    // 4. Marginal VaR per lot, adjusted for correlation
    // Low correlation (0.0) = full diversification benefit → marginal VaR is reduced
    // High correlation (1.0) = no diversification → marginal VaR = full unit VaR
    // Formula: marginal_var = unit_var * (0.5 + 0.5 * avg_corr)
    // At corr=0: marginal = 50% of unit var (diversification)
    // At corr=1: marginal = 100% of unit var (no benefit)
    let diversification_factor = 0.5 + 0.5 * avg_corr.clamp(0.0, 1.0);
    let marginal_var_per_lot = unit_var * diversification_factor;

    // 5. Remaining VaR budget
    let max_portfolio_var = (var_pct_target / 100.0) * equity;
    let remaining_budget = (max_portfolio_var - existing_total_var).max(0.0);

    // 6. Max lots from remaining budget
    let lots = remaining_budget / marginal_var_per_lot;

    Some((lots, marginal_var_per_lot, existing_total_var))
}

/// Pearson correlation coefficient between two return series.
fn pearson_correlation(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len().min(y.len());
    if n < 3 {
        return 0.0;
    }

    let n_f = n as f64;
    let mut sum_x = 0.0;
    let mut sum_y = 0.0;
    let mut sum_xy = 0.0;
    let mut sum_x2 = 0.0;
    let mut sum_y2 = 0.0;

    for i in 0..n {
        sum_x += x[i];
        sum_y += y[i];
        sum_xy += x[i] * y[i];
        sum_x2 += x[i] * x[i];
        sum_y2 += y[i] * y[i];
    }

    let denom = ((n_f * sum_x2 - sum_x * sum_x) * (n_f * sum_y2 - sum_y * sum_y)).sqrt();
    if denom < 1e-20 {
        return 0.0;
    }
    (n_f * sum_xy - sum_x * sum_y) / denom
}

// ── ATR Calculation ──────────────────────────────────────────────

/// Calculate Average True Range from OHLC bars.
/// Returns ATR value (average of true ranges over `period` bars).
pub fn calculate_atr(bars: &[(f64, f64, f64, f64)], period: usize) -> f64 {
    // bars = [(open, high, low, close), ...]
    if bars.len() < period + 1 {
        return 0.0;
    }
    let mut true_ranges = Vec::with_capacity(bars.len() - 1);
    for i in 1..bars.len() {
        let (_, h, l, _) = bars[i];
        let prev_close = bars[i - 1].3;
        let tr = (h - l)
            .max((h - prev_close).abs())
            .max((l - prev_close).abs());
        true_ranges.push(tr);
    }
    if true_ranges.len() < period {
        return 0.0;
    }
    // Simple average of last `period` true ranges
    let start = true_ranges.len() - period;
    true_ranges[start..].iter().sum::<f64>() / period as f64
}

// ── IQR Outlier Detection ───────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlierResult {
    pub symbol: String,
    pub sector: String,
    pub industry: String,
    pub metric: f64,
    pub sector_median: f64,
    pub sector_q1: f64,
    pub sector_q3: f64,
    pub z_score: f64,
    pub tier: String,      // "EXTREME", "HIGH", "NORMAL", "LOW"
    pub direction: String, // "high" or "low"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectorStats {
    pub sector: String,
    pub count: usize,
    pub median: f64,
    pub q1: f64,
    pub q3: f64,
    pub iqr: f64,
    pub lower_bound: f64,
    pub upper_bound: f64,
    pub outlier_count: usize,
}

/// Detect outliers using IQR method, grouped by sector.
/// Returns (outliers, sector_stats).
///
/// Grouping stays by sector (IQR needs ~10+ peers to be statistically meaningful,
/// and most industries have 2-5 symbols which kills IQR validity). Industry is
/// carried through as a display/sort field only.
///
/// PERF5: Uses Arc<str> internally for sector deduplication — each unique sector
/// string is allocated once, then Arc::clone()'d (cheap refcount bump) per outlier.
pub fn detect_outliers(
    data: &[(String, String, String, f64)], // (symbol, sector, industry, metric_value)
    iqr_multiplier: f64,                    // typically 1.5 for standard, 3.0 for extreme
) -> (Vec<OutlierResult>, Vec<SectorStats>) {
    use std::sync::Arc;
    // Intern sector names — one allocation per unique sector
    let mut sector_intern: std::collections::HashMap<&str, Arc<str>> =
        std::collections::HashMap::new();
    let mut by_sector: std::collections::HashMap<Arc<str>, Vec<(&str, &str, f64)>> =
        std::collections::HashMap::new();
    for (sym, sector, industry, val) in data {
        let arc = sector_intern
            .entry(sector.as_str())
            .or_insert_with(|| Arc::from(sector.as_str()))
            .clone();
        by_sector
            .entry(arc)
            .or_default()
            .push((sym.as_str(), industry.as_str(), *val));
    }

    let mut outliers = Vec::new();
    let mut stats = Vec::new();

    for (sector, mut values) in by_sector {
        if values.len() < 4 {
            continue;
        } // need enough data for IQR
        values.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));

        let n = values.len();
        let q1 = values[n / 4].2;
        let median = values[n / 2].2;
        let q3 = values[3 * n / 4].2;
        let iqr = q3 - q1;
        let lower_bound = q1 - iqr_multiplier * iqr;
        let upper_bound = q3 + iqr_multiplier * iqr;

        // Compute sector mean + population std_dev in a single pass
        // (was two passes + an intermediate `Vec<f64>` collect feeding std_dev).
        let mut sum = 0.0f64;
        let mut sum_sq = 0.0f64;
        for (_, _, val) in &values {
            sum += *val;
            sum_sq += *val * *val;
        }
        let n_f = n as f64;
        let mean = sum / n_f;
        let sector_sd = {
            let var_pop = (sum_sq / n_f) - mean * mean;
            if var_pop > 0.0 { var_pop.sqrt() } else { 0.0 }
        };

        let mut sector_outliers = 0;
        // Pre-resolve sector String once per group instead of cloning per row
        let sector_str: String = (*sector).to_string();
        for (sym, industry, val) in &values {
            let is_high = *val > upper_bound;
            let is_low = *val < lower_bound;
            if !is_high && !is_low {
                continue;
            }

            let z = if sector_sd > 0.0 {
                (*val - mean) / sector_sd
            } else {
                0.0
            };
            // PERF5: tier as &'static str literal — no allocation
            let tier: &'static str = if z.abs() > 3.0 {
                "EXTREME"
            } else if z.abs() > 2.0 {
                "HIGH"
            } else if z.abs() > 1.0 {
                "ELEVATED"
            } else {
                "MODERATE"
            };

            outliers.push(OutlierResult {
                symbol: (*sym).to_string(),
                sector: sector_str.clone(),
                industry: (*industry).to_string(),
                metric: *val,
                sector_median: median,
                sector_q1: q1,
                sector_q3: q3,
                z_score: z,
                tier: tier.to_string(),
                direction: if is_high {
                    "high".to_string()
                } else {
                    "low".to_string()
                },
            });
            sector_outliers += 1;
        }

        stats.push(SectorStats {
            sector: sector_str,
            count: n,
            median,
            q1,
            q3,
            iqr,
            lower_bound,
            upper_bound,
            outlier_count: sector_outliers,
        });
    }

    // Sort outliers by |z_score| descending (most extreme first)
    outliers.sort_by(|a, b| {
        b.z_score
            .abs()
            .partial_cmp(&a.z_score.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    stats.sort_by(|a, b| b.outlier_count.cmp(&a.outlier_count));

    (outliers, stats)
}

/// Multi-dimensional anomaly detection result.
/// Combines VaR, EV, ATR, and SEC filing activity into a composite risk score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiOutlierResult {
    pub symbol: String,
    pub sector: String,
    pub industry: String,
    /// Individual z-scores per dimension (0 = not anomalous)
    pub var_z: f64, // price risk outlier
    pub ev_z: f64,  // valuation outlier
    pub atr_z: f64, // volatility outlier
    pub sec_z: f64, // SEC filing activity outlier
    /// Composite anomaly score (sum of |z-scores| for flagged dimensions)
    pub composite_score: f64,
    /// Number of dimensions flagging (1-4)
    pub dimensions_flagged: u8,
    /// Human-readable tier: EXTREME (3+dim), HIGH (2dim), ELEVATED (1dim)
    pub tier: String,
    /// Raw values
    pub var_value: f64,
    pub ev_value: f64,
    pub atr_value: f64,
    pub sec_filings: i32,
}

/// Multi-dimensional outlier detection combining VaR + EV + ATR + SEC activity.
/// Each dimension is z-scored within its sector. Symbols flagging on multiple
/// dimensions get higher composite scores.
pub fn detect_multi_outliers(
    symbols: &[(String, String, String)], // (symbol, sector, industry)
    var_map: &std::collections::HashMap<String, f64>, // symbol → VaR 95%
    ev_map: &std::collections::HashMap<String, f64>, // symbol → MCap/EV ratio
    atr_map: &std::collections::HashMap<String, f64>, // symbol → ATR as % of price
    sec_map: &std::collections::HashMap<String, i32>, // symbol → filing count (recent)
    threshold: f64,                       // z-score threshold (typically 1.5-2.0)
) -> Vec<MultiOutlierResult> {
    // Industry lookup (symbol → industry) so we can attach industry to results.
    let industry_by_sym: std::collections::HashMap<&str, &str> = symbols
        .iter()
        .map(|(s, _, i)| (s.as_str(), i.as_str()))
        .collect();
    // Group by sector (IQR stays meaningful at sector granularity)
    let mut by_sector: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for (sym, sector, _industry) in symbols {
        by_sector
            .entry(sector.clone())
            .or_default()
            .push(sym.clone());
    }

    let mut results = Vec::new();

    for (sector, syms) in &by_sector {
        if syms.len() < 4 {
            continue;
        }

        // Compute z-scores per dimension within this sector.
        // Single-pass mean + population std via sum + sum_sq. Was two passes
        // (sum then std_dev) over the same buffer.
        let z_scores = |map: &std::collections::HashMap<String, f64>| -> std::collections::HashMap<String, f64> {
            let vals: Vec<f64> = syms.iter().filter_map(|s| map.get(s).copied()).collect();
            if vals.len() < 4 { return std::collections::HashMap::new(); }
            let mut sum = 0.0f64;
            let mut sum_sq = 0.0f64;
            for &v in &vals {
                sum += v;
                sum_sq += v * v;
            }
            let n_f = vals.len() as f64;
            let mean = sum / n_f;
            let var_pop = (sum_sq / n_f) - mean * mean;
            let sd = if var_pop > 0.0 { var_pop.sqrt() } else { 0.0 };
            if sd <= 0.0 { return std::collections::HashMap::new(); }
            syms.iter().filter_map(|s| {
                map.get(s).map(|v| (s.clone(), (v - mean) / sd))
            }).collect()
        };

        let var_zs = z_scores(var_map);
        let ev_zs = z_scores(ev_map);
        let atr_zs = z_scores(atr_map);
        // SEC filings: convert i32 to f64 for z-scoring
        let sec_f64: std::collections::HashMap<String, f64> = sec_map
            .iter()
            .map(|(k, v)| (k.clone(), *v as f64))
            .collect();
        let sec_zs = z_scores(&sec_f64);

        for sym in syms {
            let vz = var_zs.get(sym).copied().unwrap_or(0.0);
            let ez = ev_zs.get(sym).copied().unwrap_or(0.0);
            let az = atr_zs.get(sym).copied().unwrap_or(0.0);
            let sz = sec_zs.get(sym).copied().unwrap_or(0.0);

            let mut dims = 0u8;
            if vz.abs() > threshold {
                dims += 1;
            }
            if ez.abs() > threshold {
                dims += 1;
            }
            if az.abs() > threshold {
                dims += 1;
            }
            if sz.abs() > threshold {
                dims += 1;
            }

            if dims == 0 {
                continue;
            } // not an outlier on any dimension

            let composite = vz.abs() + ez.abs() + az.abs() + sz.abs();
            let tier = if dims >= 3 {
                "EXTREME"
            } else if dims >= 2 {
                "HIGH"
            } else {
                "ELEVATED"
            };

            results.push(MultiOutlierResult {
                symbol: sym.clone(),
                sector: sector.clone(),
                industry: industry_by_sym
                    .get(sym.as_str())
                    .copied()
                    .unwrap_or("")
                    .to_string(),
                var_z: vz,
                ev_z: ez,
                atr_z: az,
                sec_z: sz,
                composite_score: composite,
                dimensions_flagged: dims,
                tier: tier.to_string(),
                var_value: var_map.get(sym).copied().unwrap_or(0.0),
                ev_value: ev_map.get(sym).copied().unwrap_or(0.0),
                atr_value: atr_map.get(sym).copied().unwrap_or(0.0),
                sec_filings: sec_map.get(sym).copied().unwrap_or(0),
            });
        }
    }

    // Sort by composite score descending (most anomalous first)
    results.sort_by(|a, b| {
        b.composite_score
            .partial_cmp(&a.composite_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_std_dev() {
        let values = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let sd = std_dev(&values);
        assert!((sd - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_inverse_normal_95() {
        let z = inverse_cumulative_normal(0.95);
        assert!((z - 1.6449).abs() < 0.001);
    }

    #[test]
    fn test_var_basic() {
        let prices: Vec<f64> = (0..22).map(|i| 100.0 + (i as f64) * 0.5).collect();
        let result = calculate_var(&prices, 1.0, 1.0, 0.01, 110.0, 0.95);
        assert!(result.is_some());
        assert!(result.unwrap().var_dollars > 0.0);
    }

    #[test]
    fn test_std_dev_single_value() {
        assert_eq!(std_dev(&[42.0]), 0.0);
    }

    #[test]
    fn test_std_dev_empty() {
        assert_eq!(std_dev(&[]), 0.0);
    }

    #[test]
    fn test_std_dev_identical_values() {
        assert_eq!(std_dev(&[5.0, 5.0, 5.0, 5.0]), 0.0);
    }

    #[test]
    fn test_inverse_normal_99() {
        let z = inverse_cumulative_normal(0.99);
        assert!((z - 2.3263).abs() < 0.001, "z99={z}");
    }

    #[test]
    fn test_inverse_normal_50() {
        let z = inverse_cumulative_normal(0.50);
        assert!(z.abs() < 0.01, "z50 should be ~0, got {z}");
    }

    #[test]
    fn test_var_insufficient_data() {
        let prices = vec![100.0, 101.0]; // only 2 prices, need >20
        let result = calculate_var(&prices, 1.0, 1.0, 0.01, 101.0, 0.95);
        assert!(result.is_none());
    }

    #[test]
    fn test_var_99_greater_than_95() {
        let prices: Vec<f64> = (0..100)
            .map(|i| 100.0 + (i as f64 * 0.3).sin() * 5.0)
            .collect();
        let var95 = calculate_var(&prices, 1.0, 1.0, 0.01, 100.0, 0.95);
        let var99 = calculate_var(&prices, 1.0, 1.0, 0.01, 100.0, 0.99);
        if let (Some(v95), Some(v99)) = (var95, var99) {
            assert!(v99.var_dollars >= v95.var_dollars, "VaR99 should >= VaR95");
        }
    }
}
