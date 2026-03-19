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
    if variance > 0.0 {
        variance.sqrt()
    } else {
        0.0
    }
}

/// Rational approximation of the inverse cumulative normal distribution.
/// Port of InverseCumulativeNormal from DWEX Portfolio Risk Man.
pub fn inverse_cumulative_normal(p: f64) -> f64 {
    const A: [f64; 6] = [
        -39.6968302866538, 220.946098424521, -275.928510446969,
        138.357751867269, -30.6647980661472, 2.50662827745924,
    ];
    const B: [f64; 5] = [
        -54.4760987982241, 161.585836858041, -155.698979859887,
        66.8013118877197, -13.2806815528857,
    ];
    const C: [f64; 6] = [
        -0.00778489400243029, -0.322396458041136, -2.40075827716184,
        -2.54973253934373, 4.37466414146497, 2.93816398269878,
    ];
    const D: [f64; 4] = [
        0.00778469570904146, 0.32246712907004, 2.445134137143, 3.75440866190742,
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
fn compute_daily_returns(close_prices: &[f64]) -> Vec<f64> {
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
) -> Option<(f64, f64, f64)> { // (lots, marginal_var_per_lot, total_portfolio_var)
    if new_symbol_closes.len() < 2 || tick_size <= 0.0 || new_price <= 0.0 || equity <= 0.0 {
        return None;
    }

    // 1. Compute existing portfolio VaR (sum of individual VaRs with correlation adjustment)
    let mut existing_total_var = 0.0;
    let mut existing_return_series: Vec<Vec<f64>> = Vec::new();

    for (closes, size, price) in existing_positions {
        if let Some(var_result) = calculate_var(closes, *size, tick_value, tick_size, *price, confidence) {
            existing_total_var += var_result.var_dollars;
        }
        existing_return_series.push(compute_daily_returns(closes));
    }

    // 2. Compute VaR for 1 lot of the new symbol
    let new_returns = compute_daily_returns(new_symbol_closes);
    let sd_new = std_dev(&new_returns);
    if sd_new == 0.0 || !sd_new.is_finite() { return None; }

    let z = inverse_cumulative_normal(confidence);
    if z == 0.0 { return None; }

    let nominal_per_unit = tick_value / tick_size;
    let unit_var = z * sd_new * nominal_per_unit * new_price;
    if unit_var < 1e-10 { return None; }

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
        if corr_count > 0 { corr_sum / corr_count as f64 } else { 0.0 }
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
    if n < 3 { return 0.0; }

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
    if denom < 1e-20 { return 0.0; }
    (n_f * sum_xy - sum_x * sum_y) / denom
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
}
