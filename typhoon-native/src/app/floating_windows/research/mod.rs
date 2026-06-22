//! Research-UI floating windows boundary (ADR-125 Phase 1).
//!
//! Groups the per-indicator / per-fundamental research floating-window
//! renderers (formerly loose `research_*` modules directly under
//! `floating_windows`) behind a single dispatch entry point so the research
//! UI has an identifiable surface ahead of the future `typhoon-research-ui`
//! crate extraction. The individual `render_research_*_windows` methods stay
//! `pub(super)` (now scoped to this module) and are invoked only through
//! [`TyphooNApp::render_research_ui_windows`]. Behavior and command names are
//! unchanged; this is a module-boundary move only.
use super::*;

/// Default research symbol derived from a chart's full symbol key (e.g.
/// `ALPACA:AAPL:1Day` → `AAPL`), falling back to `AAPL` when no chart is active.
/// ADR-125: the per-window renderers all derived this identically inline; this is
/// the first shared read-context helper for the research-UI boundary. Pure over the
/// symbol string (no `TyphooNApp`), so it is crate-movable.
pub(super) fn research_chart_symbol(chart_symbol: Option<&str>) -> String {
    chart_symbol
        .map(|sym| {
            sym.split(':')
                .rev()
                .nth(1)
                .or_else(|| sym.split(':').last())
                .unwrap_or("AAPL")
                .to_string()
        })
        .unwrap_or_else(|| "AAPL".to_string())
}

#[cfg(test)]
mod tests {
    use super::research_chart_symbol;

    #[test]
    fn extracts_symbol_from_source_symbol_timeframe_key() {
        assert_eq!(research_chart_symbol(Some("ALPACA:AAPL:1Day")), "AAPL");
        assert_eq!(research_chart_symbol(Some("KRAKEN:WOK:1Hour")), "WOK");
    }

    #[test]
    fn falls_back_to_aapl_when_absent_or_bare() {
        assert_eq!(research_chart_symbol(None), "AAPL");
        // A bare token with no ':' has no nth(1); `last()` yields the token itself.
        assert_eq!(research_chart_symbol(Some("TSLA")), "TSLA");
    }
}

mod render;
mod window_shell;
mod research_advanced_moving_averages;
mod research_aroon_macd_variable_ma;
mod research_autocorrelation_hurst_volume;
mod research_behavior_distribution_stats;
mod research_calmar_ulcer_liquidity_normality;
mod research_candlestick_core_patterns;
mod research_candlestick_marubozu_line_patterns;
mod research_candlestick_reversal_continuation;
mod research_company_events_market_feeds;
mod research_corporate_actions_analyst_esg;
mod research_directional_moneyflow_sar;
mod research_directional_movement_family;
mod research_dividends_earnings_upgrades;
mod research_downside_efficiency_wick_volatility;
mod research_ehlers_adaptive_ma_oscillators;
mod research_entropy_stationarity_recovery;
mod research_entropy_tail_autocorrelation;
mod research_factor_quality_credit_models;
mod research_factor_ranking_extensions;
mod research_financials_management_cot;
mod research_fractal_tail_nonlinear_rank;
mod research_fx_beta_valuation_identifiers;
mod research_gap_volatility_mean_reversion;
mod research_garch_bubble_dimension_information;
mod research_global_market_cost_capital;
mod research_ichimoku_supertrend_channels;
mod research_ingest;
mod research_insider_dividend_earnings_momentum;
mod research_jump_unitroot_multifractal_tsi;
mod research_laguerre_pivot_midpoint_models;
mod research_leverage_quality_volatility_shorts;
mod research_linearreg_hilbert_phase;
mod research_linearreg_hilbert_stochastic;
mod research_massindex_atr_squeeze_force;
mod research_momentum_gap_atr_drawdown;
mod research_moving_average_regression_pivots;
mod research_normality_lmoments_price_impact;
mod research_ohlc_price_transforms;
mod research_ohlc_volatility_cvar_calendar;
mod research_omega_fractal_burke_seasonality;
mod research_oscillator_price_momentum;
mod research_ownership_float_price_earnings;
mod research_portmanteau_ou_long_memory_spectrum;
mod research_quant_risk_nonlinearity;
mod research_rate_of_change_correlation;
mod research_residual_iid_heteroskedastic_cycles;
mod research_return_risk_dcf_options;
mod research_robust_entropy_quantile_volatility;
mod research_seasonality_correlation_technicals;
mod research_sector_factor_drift_ranks;
mod research_sharpe_stationarity_jump_drawdown;
mod research_solvency_scores_volatility_targets;
mod research_squeeze_breakout_channels;
mod research_sterling_kelly_stat_tests;
mod research_tail_arch_pain_structural_var;
mod research_upside_leverage_drawdown_var;
mod research_volume_flow_trend_oscillators;
mod research_volume_momentum_oscillators;
mod research_zero_lag_elder_forecast_balance;

impl TyphooNApp {
    /// Single dispatch entry point for all research floating windows.
    ///
    /// Each inner renderer early-returns on its own `show_*` flag, so this
    /// only does work for windows the user has actually opened. Call order is
    /// preserved from the previous inline dispatch in `draw_floating_windows`.
    pub(super) fn render_research_ui_windows(&mut self, ctx: &egui::Context) {
        self.render_research_company_events_market_feeds_windows(ctx);
        self.render_research_dividends_earnings_upgrades_windows(ctx);
        self.render_research_financials_management_cot_windows(ctx);
        self.render_research_corporate_actions_analyst_esg_windows(ctx);
        self.render_research_ownership_float_price_earnings_windows(ctx);
        self.render_research_global_market_cost_capital_windows(ctx);
        self.render_research_fx_beta_valuation_identifiers_windows(ctx);
        self.render_research_return_risk_dcf_options_windows(ctx);
        self.render_research_seasonality_correlation_technicals_windows(ctx);
        self.render_research_leverage_quality_volatility_shorts_windows(ctx);
        self.render_research_solvency_scores_volatility_targets_windows(ctx);
        self.render_research_insider_dividend_earnings_momentum_windows(ctx);
        self.render_research_factor_quality_credit_models_windows(ctx);
        self.render_research_sector_factor_drift_ranks_windows(ctx);
        self.render_research_factor_ranking_extensions_windows(ctx);
        self.render_research_momentum_gap_atr_drawdown_windows(ctx);
        self.render_research_behavior_distribution_stats_windows(ctx);
        self.render_research_autocorrelation_hurst_volume_windows(ctx);
        self.render_research_gap_volatility_mean_reversion_windows(ctx);
        self.render_research_downside_efficiency_wick_volatility_windows(ctx);
        self.render_research_calmar_ulcer_liquidity_normality_windows(ctx);
        self.render_research_omega_fractal_burke_seasonality_windows(ctx);
        self.render_research_ohlc_volatility_cvar_calendar_windows(ctx);
        self.render_research_sterling_kelly_stat_tests_windows(ctx);
        self.render_research_sharpe_stationarity_jump_drawdown_windows(ctx);
        self.render_research_tail_arch_pain_structural_var_windows(ctx);
        self.render_research_entropy_tail_autocorrelation_windows(ctx);
        self.render_research_upside_leverage_drawdown_var_windows(ctx);
        self.render_research_entropy_stationarity_recovery_windows(ctx);
        self.render_research_robust_entropy_quantile_volatility_windows(ctx);
        self.render_research_normality_lmoments_price_impact_windows(ctx);
        self.render_research_fractal_tail_nonlinear_rank_windows(ctx);
        self.render_research_jump_unitroot_multifractal_tsi_windows(ctx);
        self.render_research_garch_bubble_dimension_information_windows(ctx);
        self.render_research_residual_iid_heteroskedastic_cycles_windows(ctx);
        self.render_research_portmanteau_ou_long_memory_spectrum_windows(ctx);
        self.render_research_squeeze_breakout_channels_windows(ctx);
        self.render_research_ichimoku_supertrend_channels_windows(ctx);
        self.render_research_directional_moneyflow_sar_windows(ctx);
        self.render_research_oscillator_price_momentum_windows(ctx);
        self.render_research_volume_momentum_oscillators_windows(ctx);
        self.render_research_volume_flow_trend_oscillators_windows(ctx);
        self.render_research_moving_average_regression_pivots_windows(ctx);
        self.render_research_zero_lag_elder_forecast_balance_windows(ctx);
        self.render_research_advanced_moving_averages_windows(ctx);
        self.render_research_ehlers_adaptive_ma_oscillators_windows(ctx);
        self.render_research_laguerre_pivot_midpoint_models_windows(ctx);
        self.render_research_massindex_atr_squeeze_force_windows(ctx);
        self.render_research_linearreg_hilbert_stochastic_windows(ctx);
        self.render_research_linearreg_hilbert_phase_windows(ctx);
        self.render_research_ohlc_price_transforms_windows(ctx);
        self.render_research_directional_movement_family_windows(ctx);
        self.render_research_rate_of_change_correlation_windows(ctx);
        self.render_research_aroon_macd_variable_ma_windows(ctx);
        self.render_research_candlestick_core_patterns_windows(ctx);
        self.render_research_candlestick_marubozu_line_patterns_windows(ctx);
        self.render_research_candlestick_reversal_continuation_windows(ctx);
        self.render_research_quant_risk_nonlinearity_windows(ctx);
        self.render_research_ingest_windows(ctx);
    }
}
