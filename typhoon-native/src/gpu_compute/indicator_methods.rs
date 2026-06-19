//! Public GPU indicator compute method families.

use super::*;

impl GpuCompute {
    // ─── Public indicator compute methods ───

    /// Generic public dispatch for SMA/EMA/RSI/KAMA using close prices.
    pub fn dispatch_indicator_pub(
        &self,
        indicator: &Indicator,
        period: u32,
        parallel: bool,
    ) -> Option<Vec<f32>> {
        let pipeline = match indicator {
            Indicator::Sma => &self.sma_pipeline,
            Indicator::Ema => &self.ema_pipeline,
            Indicator::Rsi => &self.rsi_pipeline,
            Indicator::Kama => &self.kama_pipeline,
            Indicator::Wma => &self.wma_pipeline,
            Indicator::Momentum => &self.momentum_pipeline,
            Indicator::Cmo => &self.cmo_pipeline,
            Indicator::Disparity => &self.disparity_pipeline,
            Indicator::Stddev => &self.stddev_pipeline,
            Indicator::Fisher => &self.fisher_pipeline,
            Indicator::Stochastic => &self.stochastic_pipeline,
            Indicator::Atr => return None, // requires OHLC dispatch
            Indicator::Adx => return None, // requires OHLC dispatch
            Indicator::Bollinger => return None, // multi-output
            Indicator::Macd => return None, // multi-output
            Indicator::Ichimoku => return None, // complex multi-output
            _ => return None,
        };
        self.dispatch_indicator(pipeline, period, parallel)
    }

    /// Compute WMA on GPU. Returns f32 per bar.
    pub fn compute_wma_gpu(&self, period: u32) -> Option<Vec<f32>> {
        self.dispatch_indicator(&self.wma_pipeline, period, true)
    }

    /// Compute Momentum on GPU. Returns f32 per bar.
    pub fn compute_momentum_gpu(&self, period: u32) -> Option<Vec<f32>> {
        self.dispatch_indicator(&self.momentum_pipeline, period, true)
    }

    /// Compute CMO on GPU. Returns f32 per bar.
    pub fn compute_cmo_gpu(&self, period: u32) -> Option<Vec<f32>> {
        self.dispatch_indicator(&self.cmo_pipeline, period, true)
    }

    /// Compute QStick on GPU from resident open + close buffers.
    pub fn compute_qstick_gpu(&self, period: u32) -> Option<Vec<f32>> {
        if self.open_buffer.is_none() || self.bar_count == 0 {
            return None;
        }
        self.dispatch_multi_indicator(
            &self.qstick_pipeline,
            [period, self.bar_count, 0, 0],
            true,
            0,
            (self.bar_count as u64) * 4,
        )
    }

    /// Compute Disparity Index on GPU. Returns f32 per bar.
    pub fn compute_disparity_gpu(&self, period: u32) -> Option<Vec<f32>> {
        self.dispatch_indicator(&self.disparity_pipeline, period, true)
    }

    /// Compute BOP on GPU from resident open + OHLC buffers.
    pub fn compute_bop_gpu(&self, period: u32) -> Option<Vec<f32>> {
        if self.open_buffer.is_none() || self.ohlc_buffer.is_none() || self.bar_count == 0 {
            return None;
        }
        self.dispatch_multi_indicator(
            &self.bop_pipeline,
            [period, self.bar_count, 0, 0],
            true,
            0,
            (self.bar_count as u64) * 4,
        )
    }

    /// Compute rolling sample standard deviation on GPU. Returns f32 per bar.
    pub fn compute_stddev_gpu(&self, period: u32) -> Option<Vec<f32>> {
        self.dispatch_indicator(&self.stddev_pipeline, period, true)
    }

    /// Compute MFI on GPU from resident OHLC + volume buffers.
    pub fn compute_mfi_gpu(&self, period: u32) -> Option<Vec<f32>> {
        if self.ohlc_buffer.is_none() || self.vol_buffer.is_none() || self.bar_count == 0 {
            return None;
        }
        self.dispatch_multi_indicator(
            &self.mfi_pipeline,
            [period, self.bar_count, 0, 0],
            true,
            0,
            (self.bar_count as u64) * 4,
        )
    }

    /// Compute TRIX on GPU from close prices. Output: [line, signal, hist] × bar_count.
    pub fn compute_trix_gpu(
        &self,
        closes: &[f32],
        period: u32,
        signal_period: u32,
    ) -> Option<Vec<f32>> {
        if closes.len() != self.bar_count as usize {
            return None;
        }
        let packed = (period & 0xFF) | ((signal_period & 0xFF) << 8);
        self.dispatch_custom_input_indicator(closes, &self.trix_pipeline, packed, 3, false)
    }

    /// Compute PPO on GPU from close prices. Output: [line, signal, hist] × bar_count.
    pub fn compute_ppo_gpu(
        &self,
        closes: &[f32],
        fast_period: u32,
        slow_period: u32,
        signal_period: u32,
    ) -> Option<Vec<f32>> {
        if closes.len() != self.bar_count as usize {
            return None;
        }
        let packed =
            (fast_period & 0xFF) | ((slow_period & 0xFF) << 8) | ((signal_period & 0xFF) << 16);
        self.dispatch_custom_input_indicator(closes, &self.ppo_pipeline, packed, 3, false)
    }

    /// Compute Ultimate Oscillator on GPU from resident OHLC buffers.
    pub fn compute_ultosc_gpu(&self) -> Option<Vec<f32>> {
        if self.ohlc_buffer.is_none() || self.bar_count == 0 {
            return None;
        }
        let packed = 7u32 | (14u32 << 8) | (28u32 << 16);
        self.dispatch_multi_indicator(
            &self.ultosc_pipeline,
            [packed, self.bar_count, 0, 0],
            false,
            0,
            (self.bar_count as u64) * 4,
        )
    }

    /// Compute StochRSI on GPU from close prices. Output: [%K, %D] × bar_count.
    pub fn compute_stochrsi_gpu(
        &self,
        closes: &[f32],
        rsi_period: u32,
        stoch_period: u32,
        k_smooth: u32,
        d_smooth: u32,
    ) -> Option<Vec<f32>> {
        if closes.len() != self.bar_count as usize {
            return None;
        }
        let packed = (rsi_period & 0xFF)
            | ((stoch_period & 0xFF) << 8)
            | ((k_smooth & 0xFF) << 16)
            | ((d_smooth & 0xFF) << 24);
        self.dispatch_custom_input_indicator(closes, &self.stochrsi_pipeline, packed, 2, false)
    }

    /// Compute VaR oscillator on GPU using rolling parametric 95% VaR.
    pub fn compute_var_oscillator_gpu(&self, period: u32) -> Option<Vec<f32>> {
        self.dispatch_indicator(&self.var_osc_pipeline, period, false)
    }

    /// Compute Parabolic SAR on GPU. Returns f32 per bar. Requires OHLC upload.
    pub fn compute_psar_gpu(&self) -> Option<Vec<f32>> {
        self.dispatch_ohlc_indicator(&self.psar_pipeline, 0, 1)
    }

    /// Compute Williams %R on GPU. Returns f32 per bar. Requires OHLC upload.
    pub fn compute_williams_r_gpu(&self, period: u32) -> Option<Vec<f32>> {
        self.dispatch_ohlc_indicator(&self.williams_r_pipeline, period, 1)
    }

    /// Compute Ichimoku on GPU. Returns [tenkan, kijun, span_a, span_b] × bar_count. Requires OHLC.
    pub fn compute_ichimoku_gpu(&self) -> Option<Vec<f32>> {
        self.dispatch_ohlc_indicator(&self.ichimoku_pipeline, 0, 4)
    }
}
