//! Return-distribution and risk-statistical research snapshot computations.

use super::*;

mod autocorr_regime;
mod distribution_shape;
mod downside_efficiency;
mod drawdown_liquidity_normality;
mod drawup_gap_range;
mod seasonality_spread;
mod volatility_estimators;
mod performance_runs_tests;
mod significance_stationarity;
mod tail_risk_diagnostics;
mod entropy_dependence;
mod upside_drawdown_risk;
mod entropy_stationarity;
mod robust_quantile_volatility;
mod normality_liquidity_tail;
mod fractal_rank_dynamics;
mod jump_trend_diagnostics;
mod spectral_nonlinear_diagnostics;
pub use autocorr_regime::*;
use autocorr_regime::acf_at_lag;
pub use distribution_shape::*;
pub use downside_efficiency::*;
pub use drawdown_liquidity_normality::*;
pub use drawup_gap_range::*;
pub use seasonality_spread::*;
pub use volatility_estimators::*;
pub use performance_runs_tests::*;
pub use significance_stationarity::*;
pub use tail_risk_diagnostics::*;
pub use entropy_dependence::*;
pub use upside_drawdown_risk::*;
pub use entropy_stationarity::*;
pub use robust_quantile_volatility::*;
pub use normality_liquidity_tail::*;
pub use fractal_rank_dynamics::*;
pub use jump_trend_diagnostics::*;
pub use spectral_nonlinear_diagnostics::*;

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
