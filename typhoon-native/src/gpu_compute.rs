//! GPU Compute Module — wgpu compute shaders for indicator computation.
//!
//! All bar data stored in VRAM. Indicators computed in parallel on GPU.
//! CPU only reads back small results for UI text display.
//!
//! Infrastructure ready — will replace CPU indicator computation paths
//! when wired to TyphooNApp's chart rendering pipeline.

#![allow(dead_code)]

use std::sync::Arc;
use wgpu;

mod advanced_indicators;
mod backtester;
mod bespoke_ohlc;
mod bind_groups;
mod constructor;
mod core_indicator_methods;
mod dispatch;
mod indicator_methods;
mod legacy_sma_ema;
mod parity_indicators;
mod pipeline_bootstrap;
mod resources;
mod shaders;

use pipeline_bootstrap::*;
use shaders::*;

pub use backtester::{BacktestResult, GpuBacktester, NnfxParamCombo, ParamCombo};

/// Manages GPU compute pipelines and buffers for indicator computation.
/// Indicator type selector for generic dispatch.
pub enum Indicator {
    Sma,
    Ema,
    Rsi,
    Kama,
    Wma,
    Hma,
    Cci,
    WilliamsR,
    Obv,
    Momentum,
    Cmo,
    Disparity,
    Stddev,
    // Extended for full 3-path GPU prioritization
    Fisher,
    Stochastic,
    Atr,
    Adx,
    Bollinger,
    Macd,
    Ichimoku,
}

pub struct GpuCompute {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    /// Open prices in VRAM: open prices (f32 per bar) for multi-input indicators.
    open_buffer: Option<wgpu::Buffer>,
    /// Bar data in VRAM: close prices (f32 per bar) for simple indicators
    bar_buffer: Option<wgpu::Buffer>,
    /// OHLC data in VRAM: [high, low, close] interleaved (3 × f32 per bar) for ATR/Stoch/ADX
    ohlc_buffer: Option<wgpu::Buffer>,
    /// Midpoint data in VRAM: (high+low)/2 per bar for Fisher Transform
    mid_buffer: Option<wgpu::Buffer>,
    /// Volume data in VRAM: f32 per bar for OBV
    vol_buffer: Option<wgpu::Buffer>,
    /// Number of bars currently in GPU buffer.
    bar_count: u32,
    /// PERF4: Bar count of current pooled buffers — skips reallocation if upload_bars is called
    /// with the same size (common: re-uploading after a forming bar update).
    pooled_bar_count: u32,
    /// SMA output buffer (one f32 per bar).
    sma_buffer: Option<wgpu::Buffer>,
    /// EMA output buffer.
    ema_buffer: Option<wgpu::Buffer>,
    /// Bind group layout for indicator shaders.
    bind_group_layout: wgpu::BindGroupLayout,
    /// Multi-input bind group layout for indicators that consume open / OHLC / volume / aux buffers.
    multi_bind_group_layout: wgpu::BindGroupLayout,
    /// Staging buffer for CPU readback.
    readback_buffer: Option<wgpu::Buffer>,
    /// Shared output buffer for `dispatch_indicator` (sized 1x bars).
    /// Reused across every single-output indicator call — avoids per-call alloc.
    ind_out_buffer: Option<wgpu::Buffer>,
    /// Shared 8-byte uniform buffer for `dispatch_indicator` params (period, bar_count).
    /// Reused across every call — avoids per-call alloc.
    ind_params_buffer: Option<wgpu::Buffer>,
    /// Shared scratch input buffer for `dispatch_custom_input_indicator` and auxiliary per-bar
    /// series in multi-input dispatches (for example ATR in ATR projection).
    /// Sized 4 × bar_count × 4 bytes — enough for legacy packed custom inputs and any
    /// single-series auxiliary input.
    custom_in_buffer: Option<wgpu::Buffer>,
    /// Shared output buffer for `dispatch_custom_input_indicator`. Sized 4 × bar_count
    /// to cover ATR-projection's 4 outputs per bar.
    custom_out_buffer: Option<wgpu::Buffer>,
    /// Shared 16-byte uniform buffer for multi-input dispatch params.
    multi_params_buffer: Option<wgpu::Buffer>,
    /// Cached bind group for close-only indicator dispatches.
    indicator_bind_group: Option<wgpu::BindGroup>,
    /// Cached bind group for legacy custom-input dispatches using `custom_in_buffer`.
    custom_bind_group: Option<wgpu::BindGroup>,
    /// Cached bind group for multi-input indicator dispatches.
    multi_bind_group: Option<wgpu::BindGroup>,
    // ─── Compute pipelines ───
    sma_pipeline: wgpu::ComputePipeline,
    ema_pipeline: wgpu::ComputePipeline,
    rsi_pipeline: wgpu::ComputePipeline,
    kama_pipeline: wgpu::ComputePipeline,
    atr_pipeline: wgpu::ComputePipeline,
    bollinger_pipeline: wgpu::ComputePipeline,
    macd_pipeline: wgpu::ComputePipeline,
    fisher_pipeline: wgpu::ComputePipeline,
    stochastic_pipeline: wgpu::ComputePipeline,
    adx_pipeline: wgpu::ComputePipeline,
    wma_pipeline: wgpu::ComputePipeline,
    cci_pipeline: wgpu::ComputePipeline,
    williams_r_pipeline: wgpu::ComputePipeline,
    obv_pipeline: wgpu::ComputePipeline,
    momentum_pipeline: wgpu::ComputePipeline,
    cmo_pipeline: wgpu::ComputePipeline,
    qstick_pipeline: wgpu::ComputePipeline,
    disparity_pipeline: wgpu::ComputePipeline,
    bop_pipeline: wgpu::ComputePipeline,
    stddev_pipeline: wgpu::ComputePipeline,
    mfi_pipeline: wgpu::ComputePipeline,
    trix_pipeline: wgpu::ComputePipeline,
    ppo_pipeline: wgpu::ComputePipeline,
    ultosc_pipeline: wgpu::ComputePipeline,
    stochrsi_pipeline: wgpu::ComputePipeline,
    var_osc_pipeline: wgpu::ComputePipeline,
    psar_pipeline: wgpu::ComputePipeline,
    ichimoku_pipeline: wgpu::ComputePipeline,
    cci_ohlc_pipeline: wgpu::ComputePipeline,
    obv_gpu_pipeline: wgpu::ComputePipeline,
    ehlers_ss_pipeline: wgpu::ComputePipeline,
    ehlers_dec_pipeline: wgpu::ComputePipeline,
    fractals_pipeline: wgpu::ComputePipeline,
    ehlers_itl_pipeline: wgpu::ComputePipeline,
    ehlers_cyber_pipeline: wgpu::ComputePipeline,
    ehlers_cg_pipeline: wgpu::ComputePipeline,
    ehlers_roof_pipeline: wgpu::ComputePipeline,
    ehlers_ebsw_pipeline: wgpu::ComputePipeline,
    ehlers_mama_pipeline: wgpu::ComputePipeline,
    hma_pipeline: wgpu::ComputePipeline,
    sd_zones_pipeline: wgpu::ComputePipeline,
    atr_proj_pipeline: wgpu::ComputePipeline,
    better_vol_pipeline: wgpu::ComputePipeline,
    anchored_vwap_pipeline: wgpu::ComputePipeline,
    // ─── ADR-094: GPU parity shaders ───
    supertrend_pipeline: wgpu::ComputePipeline,
    donchian_pipeline: wgpu::ComputePipeline,
    keltner_pipeline: wgpu::ComputePipeline,
    regression_pipeline: wgpu::ComputePipeline,
    squeeze_pipeline: wgpu::ComputePipeline,
    prev_levels_pipeline: wgpu::ComputePipeline,
}

fn anchored_vwap_read_window(
    bar_count: u32,
    anchor_bar: u32,
    end_bar_exclusive: u32,
) -> Option<(u64, u64)> {
    if bar_count == 0 || anchor_bar >= end_bar_exclusive || end_bar_exclusive > bar_count {
        return None;
    }
    Some((
        (anchor_bar as u64) * 4,
        ((end_bar_exclusive - anchor_bar) as u64) * 4,
    ))
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct FormingBarGpuWrite {
    scalar_offset: u64,
    ohlc_offset: u64,
    ohlc: [f32; 3],
    mid: f32,
    is_live_forming: f32,
}

fn forming_bar_gpu_write(
    bar_count: u32,
    high: f32,
    low: f32,
    close: f32,
    is_live_forming: f32,
) -> Option<FormingBarGpuWrite> {
    if bar_count == 0 {
        return None;
    }
    let last_idx = bar_count - 1;
    Some(FormingBarGpuWrite {
        scalar_offset: (last_idx as u64) * 4,
        ohlc_offset: (last_idx as u64) * 12,
        ohlc: [high, low, close],
        mid: (high + low) * 0.5,
        is_live_forming,
    })
}

impl GpuCompute {
    /// Upload only the last (forming) bar incrementally.
    /// Much cheaper than full re-upload during live Kraken WS ticks.
    pub fn upload_forming_bar(
        &mut self,
        open: f32,
        high: f32,
        low: f32,
        close: f32,
        volume: f32,
        is_live_forming: f32,
    ) -> bool {
        let Some(write) = forming_bar_gpu_write(self.bar_count, high, low, close, is_live_forming)
        else {
            return false;
        };
        let Some(ref close_buf) = self.bar_buffer else {
            return false;
        };

        let close = [close];
        self.queue
            .write_buffer(close_buf, write.scalar_offset, bytemuck_cast_slice(&close));

        if let Some(ref open_buf) = self.open_buffer {
            let open = [open];
            self.queue
                .write_buffer(open_buf, write.scalar_offset, bytemuck_cast_slice(&open));
        }
        if let Some(ref ohlc_buf) = self.ohlc_buffer {
            self.queue.write_buffer(
                ohlc_buf,
                write.ohlc_offset,
                bytemuck_cast_slice(&write.ohlc),
            );
        }
        if let Some(ref mid_buf) = self.mid_buffer {
            let mid = [write.mid];
            self.queue
                .write_buffer(mid_buf, write.scalar_offset, bytemuck_cast_slice(&mid));
        }
        if let Some(ref vol_buf) = self.vol_buffer {
            let volume = [volume];
            self.queue
                .write_buffer(vol_buf, write.scalar_offset, bytemuck_cast_slice(&volume));
        }
        true
    }

    /// Compute Anchored VWAP for a bar range on the already-uploaded chart buffers.
    /// Returns one f32 per bar in `[anchor_bar, end_bar_exclusive)`.
    pub fn compute_anchored_vwap(
        &self,
        anchor_bar: u32,
        end_bar_exclusive: u32,
    ) -> Option<Vec<f32>> {
        let (read_offset, read_size) =
            anchored_vwap_read_window(self.bar_count, anchor_bar, end_bar_exclusive)?;
        if self.ohlc_buffer.is_none() || self.vol_buffer.is_none() {
            return None;
        }
        self.dispatch_multi_indicator(
            &self.anchored_vwap_pipeline,
            [anchor_bar, self.bar_count, end_bar_exclusive, 0],
            false,
            read_offset,
            read_size,
        )
    }
}

impl GpuCompute {
    pub fn bar_count(&self) -> u32 {
        self.bar_count
    }
}

// Safe byte casting via bytemuck crate — eliminates all unsafe pointer casts
pub(super) fn bytemuck_cast_slice<T: bytemuck::NoUninit>(data: &[T]) -> &[u8] {
    bytemuck::cast_slice(data)
}

pub(super) fn bytemuck_cast_slice_to_f32(data: &[u8]) -> Vec<f32> {
    bytemuck::cast_slice::<u8, f32>(data).to_vec()
}

// ─── WGSL Compute Shaders ────────────────────────────────────────────────────

// ─── Additional Indicator Shaders ────────────────────────────────────────────

// ─── Additional Indicator Shaders (Phase 2 batch) ────────────────────────────

// ─── Phase 3 Indicators: Ichimoku, CCI, OBV, Ehlers, Fractals ──────────────

// ─── GPU Backtest / Strategy Optimizer Engine ─────────────────────────────────

// ─── Backtest WGSL Shaders ───────────────────────────────────────────────────

/// Strategy evaluation shader — one thread per parameter combination.
/// Each thread walks all bars, computes SMA crossover + RSI filter + ATR stop.

/// Robustness scoring shader — checks neighbor stability.
/// For each combo, compares its Sharpe to its neighbors (±1 on each param).
/// Score = 1.0 - normalized variance among neighbors.

/// GPU Monte Carlo VaR shader — parallel random walk simulations.
/// Each thread runs one simulation: samples from historical returns,
/// projects equity forward, records final value.

/// NNFX Strategy Evaluation — Fisher crossover + KAMA trend + ATR stops + ADX filter.
/// One thread per parameter combination. Each thread computes Fisher, KAMA, ATR, ADX inline.
/// Params: [kama_period, fisher_period, atr_period, adx_period, adx_threshold, atr_sl_mult, atr_tp_mult, 0]

/// Walk-Forward Validation shader — evaluates strategy on out-of-sample window.
/// Same as NNFX eval but only processes bars[start..end] range.
/// Params uniform extended: [bar_count, combo_count, window_start, window_end]

// ── ADR-092: New GPU compute shaders ────────────────────────────────

/// Volume Profile — bins price×volume into N price levels.
/// Input: OHLCV interleaved [open, high, low, close, volume] × bar_count.
/// Output: histogram[num_levels] = cumulative volume at each price level.

/// Batch Screener — computes RSI + SMA for 500+ symbols in one dispatch.
/// Each thread processes one symbol. Input: close prices for all symbols
/// packed sequentially with offsets. Output: [rsi, sma] per symbol.

/// Rolling Statistics — computes rolling Sharpe ratio for each window position.
/// Each thread computes one window. Input: returns array.
/// Output: rolling_sharpe[position].

/// Renko Builder — constructs Renko bricks from close price data.
/// Each brick has a fixed size. Output: [direction, open, close] per brick.
/// Sequential dispatch (brick dependencies).

/// Tick Aggregation — aggregates raw ticks into OHLCV bars at multiple timeframes.
/// Each thread processes one timeframe bucket. Input: tick prices + timestamps.
/// Output: OHLCV bars per timeframe.

/// Multi-Symbol Backtest — tests same strategy across N symbols × M param combos.
/// Extends BACKTEST_EVAL_SHADER to two-dimensional dispatch.

// ── ADR-092: GPU Render Shaders (Vertex + Fragment) ──────────────────

/// Instanced Candlestick Renderer — renders all visible candles in a single draw call.
/// Each instance is one candlestick. Instance data: [x, open_y, close_y, high_y, low_y, is_up].
/// Vertex shader expands each instance into body quad + wick line geometry.
#[allow(dead_code)]

/// Indicator Polyline Renderer — renders indicator values as GPU line strip.
/// Input: vertex buffer of [x, y, r, g, b, a] per point.
#[allow(dead_code)]

/// Heatmap Texture Renderer — renders a compute-generated texture as fullscreen quad.
/// Used for correlation matrices, sector heatmaps, volume profiles.
#[allow(dead_code)]

/// Zone Compositor — renders session highlights, S/D zones, FVG as a texture overlay.
/// Alpha-blended composite of multiple zone layers.
#[allow(dead_code)]
// ─── ADR-094: GPU parity shaders ──────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::{FormingBarGpuWrite, anchored_vwap_read_window, forming_bar_gpu_write};

    #[test]
    fn anchored_vwap_read_window_computes_expected_offset_and_size() {
        assert_eq!(anchored_vwap_read_window(10, 3, 7), Some((12, 16)));
    }

    #[test]
    fn anchored_vwap_read_window_rejects_invalid_ranges() {
        assert_eq!(anchored_vwap_read_window(0, 0, 1), None);
        assert_eq!(anchored_vwap_read_window(10, 4, 4), None);
        assert_eq!(anchored_vwap_read_window(10, 8, 7), None);
        assert_eq!(anchored_vwap_read_window(10, 8, 11), None);
    }

    #[test]
    fn forming_bar_gpu_write_targets_last_bar_offsets() {
        assert_eq!(
            forming_bar_gpu_write(5, 111.0, 99.0, 108.5, 1.0),
            Some(FormingBarGpuWrite {
                scalar_offset: 16,
                ohlc_offset: 48,
                ohlc: [111.0, 99.0, 108.5],
                mid: 105.0,
                is_live_forming: 1.0,
            })
        );
        assert_eq!(forming_bar_gpu_write(0, 111.0, 99.0, 108.5, 1.0), None);
    }
}
