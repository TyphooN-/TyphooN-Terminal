//! ADR-094 GPU parity compute wrappers.

use super::GpuCompute;

impl GpuCompute {
    /// Supertrend — sequential GPU (state machine like PSAR). 2 outputs: value, direction (1.0=up, -1.0=down).
    pub fn compute_supertrend_gpu(&self, period: u32) -> Option<Vec<f32>> {
        self.dispatch_ohlc_indicator(&self.supertrend_pipeline, period, 2)
    }

    /// Donchian Channel — parallel GPU. 2 outputs: upper (highest high), lower (lowest low).
    pub fn compute_donchian_gpu(&self, period: u32) -> Option<Vec<f32>> {
        self.dispatch_ohlc_indicator(&self.donchian_pipeline, period, 2)
    }

    /// Keltner Channel — parallel GPU. 3 outputs: upper, mid (EMA), lower.
    pub fn compute_keltner_gpu(&self, period: u32) -> Option<Vec<f32>> {
        self.dispatch_ohlc_indicator(&self.keltner_pipeline, period, 3)
    }

    /// Regression Channel — parallel GPU. 3 outputs: mid (linear reg), upper (+2σ), lower (−2σ).
    pub fn compute_regression_gpu(&self, period: u32) -> Option<Vec<f32>> {
        self.dispatch_ohlc_indicator(&self.regression_pipeline, period, 3)
    }

    /// Squeeze Momentum — parallel GPU. 2 outputs: momentum value, squeeze_on (1.0/0.0).
    pub fn compute_squeeze_gpu(&self, period: u32) -> Option<Vec<f32>> {
        self.dispatch_ohlc_indicator(&self.squeeze_pipeline, period, 2)
    }

    /// Previous Candle Levels — parallel GPU. 2 outputs per bar: prev_day_high, prev_day_low.
    pub fn compute_prev_levels_gpu(&self, period: u32) -> Option<Vec<f32>> {
        self.dispatch_ohlc_indicator(&self.prev_levels_pipeline, period, 2)
    }
}
