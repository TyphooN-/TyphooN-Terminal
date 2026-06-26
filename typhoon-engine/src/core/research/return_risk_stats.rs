//! Return-distribution and risk-statistical research snapshot computations.

use super::*;

mod autocorr_regime;
mod distribution_shape;
mod downside_efficiency;
mod drawdown_liquidity_normality;
mod drawup_gap_range;
mod entropy_dependence;
mod entropy_stationarity;
mod fractal_rank_dynamics;
mod jump_trend_diagnostics;
mod normality_liquidity_tail;
mod performance_runs_tests;
mod robust_quantile_volatility;
mod seasonality_spread;
mod significance_stationarity;
mod spectral_nonlinear_diagnostics;
mod tail_risk_diagnostics;
mod upside_drawdown_risk;
mod volatility_estimators;
use autocorr_regime::acf_at_lag;
pub use autocorr_regime::*;
pub use distribution_shape::*;
pub use downside_efficiency::*;
pub use drawdown_liquidity_normality::*;
pub use drawup_gap_range::*;
pub use entropy_dependence::*;
pub use entropy_stationarity::*;
pub use fractal_rank_dynamics::*;
pub use jump_trend_diagnostics::*;
pub use normality_liquidity_tail::*;
pub use performance_runs_tests::*;
pub use robust_quantile_volatility::*;
pub use seasonality_spread::*;
pub use significance_stationarity::*;
pub use spectral_nonlinear_diagnostics::*;
pub use tail_risk_diagnostics::*;
pub use upside_drawdown_risk::*;
pub use volatility_estimators::*;

// Shared helpers for return-distribution and risk-statistical compute modules.

/// Shared helper: collect trailing 253 bars sorted oldest-first and
/// compute log returns. Returns (sorted_bars, log_returns).
pub(crate) fn trailing_log_returns(
    bars: &[HistoricalPriceRow],
) -> (Vec<&HistoricalPriceRow>, Vec<f64>) {
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let window: Vec<&HistoricalPriceRow> = sorted.iter().rev().take(253).rev().copied().collect();
    let mut log_rets: Vec<f64> = Vec::with_capacity(window.len());
    for w in window.windows(2) {
        let prev = w[0].close;
        let curr = w[1].close;
        if prev > 0.0 && curr > 0.0 {
            log_rets.push((curr / prev).ln());
        }
    }
    (window, log_rets)
}
