//! Core GPU indicator compute method wrappers.

use super::GpuCompute;

impl GpuCompute {
    /// Compute RSI on GPU. Returns f32 per bar.
    pub fn compute_rsi_gpu(&self, period: u32) -> Option<Vec<f32>> {
        self.dispatch_indicator(&self.rsi_pipeline, period, false)
    }
    /// Compute KAMA on GPU. Returns f32 per bar.
    pub fn compute_kama_gpu(&self, period: u32) -> Option<Vec<f32>> {
        self.dispatch_indicator(&self.kama_pipeline, period, false)
    }
    /// Compute Bollinger Bands on GPU. Returns [mid, upper, lower] × bar_count.
    pub fn compute_bollinger_gpu(&self, period: u32) -> Option<Vec<f32>> {
        self.dispatch_indicator(&self.bollinger_pipeline, period, true)
    }
    /// Compute MACD on GPU. Returns [macd, signal, histogram] × bar_count.
    /// Compute MACD on GPU with user-configurable periods.
    /// Periods are bit-packed: fast | (slow << 8) | (signal << 16).
    pub fn compute_macd_gpu_dynamic(&self, fast: u32, slow: u32, signal: u32) -> Option<Vec<f32>> {
        let packed = (fast & 0xFF) | ((slow & 0xFF) << 8) | ((signal & 0xFF) << 16);
        self.dispatch_indicator(&self.macd_pipeline, packed, false)
    }
    pub fn compute_macd_gpu(&self) -> Option<Vec<f32>> {
        self.compute_macd_gpu_dynamic(12, 26, 9)
    }
    /// Compute ATR on GPU. Returns f32 per bar. Requires OHLC upload.
    pub fn compute_atr_gpu(&self, period: u32) -> Option<Vec<f32>> {
        self.dispatch_ohlc_indicator(&self.atr_pipeline, period, 1)
    }
    /// Compute Stochastic on GPU. Returns [%K, %D] × bar_count. Requires OHLC upload.
    pub fn compute_stochastic_gpu(&self, period: u32) -> Option<Vec<f32>> {
        self.dispatch_ohlc_indicator(&self.stochastic_pipeline, period, 2)
    }
    /// Compute ADX on GPU. Returns [adx, +DI, -DI] × bar_count. Requires OHLC upload.
    pub fn compute_adx_gpu(&self, period: u32) -> Option<Vec<f32>> {
        self.dispatch_ohlc_indicator(&self.adx_pipeline, period, 3)
    }
}
