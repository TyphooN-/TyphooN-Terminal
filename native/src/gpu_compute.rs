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
mod bespoke_ohlc;
mod bind_groups;
mod core_indicator_methods;
mod dispatch;
mod indicator_methods;
mod legacy_sma_ema;
mod pipeline_bootstrap;
mod resources;
mod shaders;

use pipeline_bootstrap::*;
use shaders::*;

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
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        let device_ref = device.as_ref();
        let bind_group_layout = create_indicator_bind_group_layout(device_ref);
        let multi_bind_group_layout = create_multi_indicator_bind_group_layout(device_ref);
        let pipeline_layout = create_indicator_pipeline_layout(device_ref, &bind_group_layout);
        let multi_pipeline_layout =
            create_multi_indicator_pipeline_layout(device_ref, &multi_bind_group_layout);

        let sma_pipeline =
            make_indicator_pipeline(device_ref, &pipeline_layout, "sma_pipeline", SMA_SHADER);
        let ema_pipeline =
            make_indicator_pipeline(device_ref, &pipeline_layout, "ema_pipeline", EMA_SHADER);
        let rsi_pipeline =
            make_indicator_pipeline(device_ref, &pipeline_layout, "rsi_pipeline", RSI_SHADER);
        let kama_pipeline =
            make_indicator_pipeline(device_ref, &pipeline_layout, "kama_pipeline", KAMA_SHADER);
        let atr_pipeline =
            make_indicator_pipeline(device_ref, &pipeline_layout, "atr_pipeline", ATR_SHADER);
        let bollinger_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "bollinger_pipeline",
            BOLLINGER_SHADER,
        );
        let macd_pipeline =
            make_indicator_pipeline(device_ref, &pipeline_layout, "macd_pipeline", MACD_SHADER);
        let fisher_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "fisher_pipeline",
            FISHER_SHADER,
        );
        let stochastic_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "stochastic_pipeline",
            STOCHASTIC_SHADER,
        );
        let adx_pipeline =
            make_indicator_pipeline(device_ref, &pipeline_layout, "adx_pipeline", ADX_SHADER);
        let wma_pipeline =
            make_indicator_pipeline(device_ref, &pipeline_layout, "wma_pipeline", WMA_SHADER);
        let cci_pipeline =
            make_indicator_pipeline(device_ref, &pipeline_layout, "cci_pipeline", CCI_SHADER);
        let williams_r_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "williams_r_pipeline",
            WILLIAMS_R_SHADER,
        );
        let obv_pipeline = make_multi_indicator_pipeline(
            device_ref,
            &multi_pipeline_layout,
            "obv_pipeline",
            OBV_SHADER,
        );
        let momentum_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "momentum_pipeline",
            MOMENTUM_SHADER,
        );
        let cmo_pipeline =
            make_indicator_pipeline(device_ref, &pipeline_layout, "cmo_pipeline", CMO_SHADER);
        let qstick_pipeline = make_multi_indicator_pipeline(
            device_ref,
            &multi_pipeline_layout,
            "qstick_pipeline",
            QSTICK_SHADER,
        );
        let disparity_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "disparity_pipeline",
            DISPARITY_SHADER,
        );
        let bop_pipeline = make_multi_indicator_pipeline(
            device_ref,
            &multi_pipeline_layout,
            "bop_pipeline",
            BOP_SHADER,
        );
        let stddev_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "stddev_pipeline",
            STDDEV_SHADER,
        );
        let mfi_pipeline = make_multi_indicator_pipeline(
            device_ref,
            &multi_pipeline_layout,
            "mfi_pipeline",
            MFI_SHADER,
        );
        let trix_pipeline =
            make_indicator_pipeline(device_ref, &pipeline_layout, "trix_pipeline", TRIX_SHADER);
        let ppo_pipeline =
            make_indicator_pipeline(device_ref, &pipeline_layout, "ppo_pipeline", PPO_SHADER);
        let ultosc_pipeline = make_multi_indicator_pipeline(
            device_ref,
            &multi_pipeline_layout,
            "ultosc_pipeline",
            ULTOSC_SHADER,
        );
        let stochrsi_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "stochrsi_pipeline",
            STOCHRSI_SHADER,
        );
        let var_osc_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "var_osc_pipeline",
            VAR_OSCILLATOR_SHADER,
        );
        let psar_pipeline =
            make_indicator_pipeline(device_ref, &pipeline_layout, "psar_pipeline", PSAR_SHADER);
        let ichimoku_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "ichimoku_pipeline",
            ICHIMOKU_SHADER,
        );
        let cci_ohlc_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "cci_ohlc_pipeline",
            CCI_GPU_SHADER,
        );
        let obv_gpu_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "obv_gpu_pipeline",
            OBV_GPU_SHADER,
        );
        let ehlers_ss_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "ehlers_ss_pipeline",
            EHLERS_SUPERSMOOTHER_SHADER,
        );
        let ehlers_dec_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "ehlers_dec_pipeline",
            EHLERS_DECYCLER_SHADER,
        );
        let fractals_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "fractals_pipeline",
            FRACTALS_SHADER,
        );
        let ehlers_itl_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "ehlers_itl_pipeline",
            EHLERS_ITL_SHADER,
        );
        let ehlers_cyber_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "ehlers_cyber_pipeline",
            EHLERS_CYBER_SHADER,
        );
        let ehlers_cg_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "ehlers_cg_pipeline",
            EHLERS_CG_SHADER,
        );
        let ehlers_roof_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "ehlers_roof_pipeline",
            EHLERS_ROOF_SHADER,
        );
        let ehlers_ebsw_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "ehlers_ebsw_pipeline",
            EHLERS_EBSW_SHADER,
        );
        let ehlers_mama_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "ehlers_mama_pipeline",
            EHLERS_MAMA_SHADER,
        );
        let hma_pipeline =
            make_indicator_pipeline(device_ref, &pipeline_layout, "hma_pipeline", HMA_SHADER);
        let sd_zones_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "sd_zones_pipeline",
            SUPPLY_DEMAND_SHADER,
        );
        let atr_proj_pipeline = make_multi_indicator_pipeline(
            device_ref,
            &multi_pipeline_layout,
            "atr_proj_pipeline",
            ATR_PROJECTION_SHADER,
        );
        let better_vol_pipeline = make_multi_indicator_pipeline(
            device_ref,
            &multi_pipeline_layout,
            "better_vol_pipeline",
            BETTER_VOLUME_SHADER,
        );
        let anchored_vwap_pipeline = make_multi_indicator_pipeline(
            device_ref,
            &multi_pipeline_layout,
            "anchored_vwap_pipeline",
            ANCHORED_VWAP_SHADER,
        );
        // ADR-094: GPU parity shaders
        let supertrend_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "supertrend_pipeline",
            SUPERTREND_SHADER,
        );
        let donchian_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "donchian_pipeline",
            DONCHIAN_SHADER,
        );
        let keltner_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "keltner_pipeline",
            KELTNER_SHADER,
        );
        let regression_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "regression_pipeline",
            REGRESSION_SHADER,
        );
        let squeeze_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "squeeze_pipeline",
            SQUEEZE_SHADER,
        );
        let prev_levels_pipeline = make_indicator_pipeline(
            device_ref,
            &pipeline_layout,
            "prev_levels_pipeline",
            PREV_LEVELS_SHADER,
        );

        Self {
            device,
            queue,
            open_buffer: None,
            bar_buffer: None,
            ohlc_buffer: None,
            mid_buffer: None,
            vol_buffer: None,
            bar_count: 0,
            pooled_bar_count: 0,
            sma_buffer: None,
            ema_buffer: None,
            sma_pipeline,
            ema_pipeline,
            rsi_pipeline,
            kama_pipeline,
            atr_pipeline,
            bollinger_pipeline,
            macd_pipeline,
            fisher_pipeline,
            stochastic_pipeline,
            adx_pipeline,
            wma_pipeline,
            cci_pipeline,
            williams_r_pipeline,
            obv_pipeline,
            momentum_pipeline,
            cmo_pipeline,
            qstick_pipeline,
            disparity_pipeline,
            bop_pipeline,
            stddev_pipeline,
            mfi_pipeline,
            trix_pipeline,
            ppo_pipeline,
            ultosc_pipeline,
            stochrsi_pipeline,
            var_osc_pipeline,
            psar_pipeline,
            ichimoku_pipeline,
            cci_ohlc_pipeline,
            obv_gpu_pipeline,
            ehlers_ss_pipeline,
            ehlers_dec_pipeline,
            fractals_pipeline,
            ehlers_itl_pipeline,
            ehlers_cyber_pipeline,
            ehlers_cg_pipeline,
            ehlers_roof_pipeline,
            ehlers_ebsw_pipeline,
            ehlers_mama_pipeline,
            hma_pipeline,
            sd_zones_pipeline,
            atr_proj_pipeline,
            better_vol_pipeline,
            anchored_vwap_pipeline,
            supertrend_pipeline,
            donchian_pipeline,
            keltner_pipeline,
            regression_pipeline,
            squeeze_pipeline,
            prev_levels_pipeline,
            bind_group_layout,
            multi_bind_group_layout,
            readback_buffer: None,
            ind_out_buffer: None,
            ind_params_buffer: None,
            custom_in_buffer: None,
            custom_out_buffer: None,
            multi_params_buffer: None,
            indicator_bind_group: None,
            custom_bind_group: None,
            multi_bind_group: None,
        }
    }

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

// ─── ADR-094: GPU parity dispatch methods ─────────────────────────────────────

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

// ─── WGSL Compute Shaders ────────────────────────────────────────────────────

// ─── Additional Indicator Shaders ────────────────────────────────────────────

// ─── Additional Indicator Shaders (Phase 2 batch) ────────────────────────────

// ─── Phase 3 Indicators: Ichimoku, CCI, OBV, Ehlers, Fractals ──────────────

// ─── GPU Backtest / Strategy Optimizer Engine ─────────────────────────────────

/// Result of a single parameter combination backtest.
#[derive(Debug, Clone, Default)]
pub struct BacktestResult {
    pub net_pnl: f32,
    pub max_drawdown: f32,
    pub sharpe: f32,
    pub sortino: f32,
    pub win_rate: f32,
    pub profit_factor: f32,
    pub trade_count: u32,
    pub avg_hold_bars: f32,
    pub robustness_score: f32,
}

/// A parameter combination to test.
#[derive(Debug, Clone)]
pub struct ParamCombo {
    pub sma_fast: u32,
    pub sma_slow: u32,
    pub rsi_period: u32,
    pub rsi_overbought: f32,
    pub rsi_oversold: f32,
    pub atr_period: u32,
    pub atr_sl_mult: f32,
    pub atr_tp_mult: f32,
}

/// GPU-accelerated strategy backtester.
/// Tests thousands of parameter combinations in parallel.
pub struct GpuBacktester {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    /// Bar data: [close] per bar
    bar_buffer: Option<wgpu::Buffer>,
    /// OHLC: [high, low, close] × 3 per bar
    ohlc_buffer: Option<wgpu::Buffer>,
    /// Pre-computed indicators for each param combo
    /// Flat: [indicator_values_for_combo_0, indicator_values_for_combo_1, ...]
    indicator_buffer: Option<wgpu::Buffer>,
    /// Parameter combinations: packed struct per combo
    params_buffer: Option<wgpu::Buffer>,
    /// Output: BacktestResult per combo (9 floats × combo_count)
    results_buffer: Option<wgpu::Buffer>,
    /// Staging for readback
    staging_buffer: Option<wgpu::Buffer>,
    bar_count: u32,
    combo_count: u32,
    eval_pipeline: wgpu::ComputePipeline,
    nnfx_pipeline: wgpu::ComputePipeline,
    walk_forward_pipeline: wgpu::ComputePipeline,
    robustness_pipeline: wgpu::ComputePipeline,
    monte_carlo_pipeline: wgpu::ComputePipeline,
    eval_bgl: wgpu::BindGroupLayout,
}

/// NNFX-specific parameter combination.
#[derive(Debug, Clone)]
pub struct NnfxParamCombo {
    pub kama_period: u32,
    pub fisher_period: u32,
    pub atr_period: u32,
    pub adx_period: u32,
    pub adx_threshold: f32,
    pub atr_sl_mult: f32,
    pub atr_tp_mult: f32,
}

impl GpuBacktester {
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        // Bind group layout: bars (read), ohlc (read), params (read), results (read_write), uniforms
        let eval_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("backtest_eval_bgl"),
            entries: &[
                // 0: close prices
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // 1: OHLC data [h,l,c interleaved]
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // 2: parameter combos
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // 3: results output
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // 4: uniforms [bar_count, combo_count]
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("backtest_layout"),
            bind_group_layouts: &[Some(&eval_bgl)],
            immediate_size: 0,
        });

        let eval_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("backtest_eval"),
            source: wgpu::ShaderSource::Wgsl(BACKTEST_EVAL_SHADER.into()),
        });
        let robustness_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("robustness"),
            source: wgpu::ShaderSource::Wgsl(ROBUSTNESS_SHADER.into()),
        });
        let mc_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("monte_carlo"),
            source: wgpu::ShaderSource::Wgsl(MONTE_CARLO_SHADER.into()),
        });

        let eval_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("eval_pipeline"),
            layout: Some(&layout),
            module: &eval_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let robustness_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("robustness_pipeline"),
                layout: Some(&layout),
                module: &robustness_shader,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            });
        let monte_carlo_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("mc_pipeline"),
                layout: Some(&layout),
                module: &mc_shader,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            });
        let nnfx_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("nnfx_eval"),
            source: wgpu::ShaderSource::Wgsl(NNFX_EVAL_SHADER.into()),
        });
        let nnfx_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("nnfx_pipeline"),
            layout: Some(&layout),
            module: &nnfx_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let wf_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("walk_forward"),
            source: wgpu::ShaderSource::Wgsl(WALK_FORWARD_SHADER.into()),
        });
        let walk_forward_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("wf_pipeline"),
                layout: Some(&layout),
                module: &wf_shader,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            });

        Self {
            device,
            queue,
            bar_buffer: None,
            ohlc_buffer: None,
            indicator_buffer: None,
            params_buffer: None,
            results_buffer: None,
            staging_buffer: None,
            bar_count: 0,
            combo_count: 0,
            eval_pipeline,
            nnfx_pipeline,
            walk_forward_pipeline,
            robustness_pipeline,
            monte_carlo_pipeline,
            eval_bgl,
        }
    }

    /// Upload bar data and parameter grid to GPU.
    pub fn upload(&mut self, closes: &[f32], highs: &[f32], lows: &[f32], combos: &[ParamCombo]) {
        let n = closes.len() as u32;
        let nc = combos.len() as u32;
        self.bar_count = n;
        self.combo_count = nc;

        // Close prices
        let bar_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bt_closes"),
            size: (n as u64) * 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue
            .write_buffer(&bar_buf, 0, bytemuck_cast_slice(closes));
        self.bar_buffer = Some(bar_buf);

        // OHLC interleaved
        let mut ohlc = Vec::with_capacity(n as usize * 3);
        for i in 0..n as usize {
            ohlc.push(highs[i]);
            ohlc.push(lows[i]);
            ohlc.push(closes[i]);
        }
        let ohlc_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bt_ohlc"),
            size: (n as u64) * 12,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue
            .write_buffer(&ohlc_buf, 0, bytemuck_cast_slice(&ohlc));
        self.ohlc_buffer = Some(ohlc_buf);

        // Pack param combos: 8 u32/f32 per combo
        let mut packed = Vec::with_capacity(nc as usize * 8);
        for c in combos {
            packed.push(f32::from_bits(c.sma_fast));
            packed.push(f32::from_bits(c.sma_slow));
            packed.push(f32::from_bits(c.rsi_period));
            packed.push(c.rsi_overbought);
            packed.push(c.rsi_oversold);
            packed.push(f32::from_bits(c.atr_period));
            packed.push(c.atr_sl_mult);
            packed.push(c.atr_tp_mult);
        }
        let params_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bt_params"),
            size: (nc as u64) * 32,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue
            .write_buffer(&params_buf, 0, bytemuck_cast_slice(&packed));
        self.params_buffer = Some(params_buf);

        // Results: 9 floats per combo
        let results_size = (nc as u64) * 36;
        self.results_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bt_results"),
            size: results_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        }));
        self.staging_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bt_staging"),
            size: results_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
    }

    /// Run backtest evaluation: one GPU thread per parameter combination.
    pub fn evaluate(&self) -> Option<Vec<BacktestResult>> {
        let (Some(bar_buf), Some(ohlc_buf), Some(params_buf), Some(results_buf), Some(staging)) = (
            &self.bar_buffer,
            &self.ohlc_buffer,
            &self.params_buffer,
            &self.results_buffer,
            &self.staging_buffer,
        ) else {
            return None;
        };

        let uniforms = [self.bar_count, self.combo_count];
        let uniform_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bt_uniforms"),
            size: 8,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue
            .write_buffer(&uniform_buf, 0, bytemuck_cast_slice(&uniforms));

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bt_bg"),
            layout: &self.eval_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: bar_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: ohlc_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: results_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: uniform_buf.as_entire_binding(),
                },
            ],
        });

        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("bt_eval_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.eval_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups((self.combo_count + 255) / 256, 1, 1);
        }
        let result_size = (self.combo_count as u64) * 36;
        encoder.copy_buffer_to_buffer(results_buf, 0, staging, 0, result_size);
        self.queue.submit(std::iter::once(encoder.finish()));

        // Readback
        let slice = staging.slice(0..result_size);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .ok();
        if rx.recv().ok()?.is_err() {
            return None;
        }
        let data = slice.get_mapped_range();
        let floats = bytemuck_cast_slice_to_f32(&data);
        drop(data);
        staging.unmap();

        let mut results = Vec::with_capacity(self.combo_count as usize);
        for i in 0..self.combo_count as usize {
            let b = i * 9;
            if b + 8 < floats.len() {
                results.push(BacktestResult {
                    net_pnl: floats[b],
                    max_drawdown: floats[b + 1],
                    sharpe: floats[b + 2],
                    sortino: floats[b + 3],
                    win_rate: floats[b + 4],
                    profit_factor: floats[b + 5],
                    trade_count: floats[b + 6] as u32,
                    avg_hold_bars: floats[b + 7],
                    robustness_score: floats[b + 8],
                });
            }
        }
        Some(results)
    }

    /// Upload NNFX parameter combos and run evaluation.
    pub fn evaluate_nnfx(
        &mut self,
        closes: &[f32],
        highs: &[f32],
        lows: &[f32],
        combos: &[NnfxParamCombo],
    ) -> Option<Vec<BacktestResult>> {
        let n = closes.len() as u32;
        let nc = combos.len() as u32;
        self.bar_count = n;
        self.combo_count = nc;

        // Upload bar data
        let bar_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nnfx_closes"),
            size: (n as u64) * 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue
            .write_buffer(&bar_buf, 0, bytemuck_cast_slice(closes));
        self.bar_buffer = Some(bar_buf);

        let mut ohlc = Vec::with_capacity(n as usize * 3);
        for i in 0..n as usize {
            ohlc.push(highs[i]);
            ohlc.push(lows[i]);
            ohlc.push(closes[i]);
        }
        let ohlc_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nnfx_ohlc"),
            size: (n as u64) * 12,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue
            .write_buffer(&ohlc_buf, 0, bytemuck_cast_slice(&ohlc));
        self.ohlc_buffer = Some(ohlc_buf);

        // Pack NNFX params: 8 floats per combo [kama_p, fisher_p, atr_p, adx_p, adx_thresh, sl_mult, tp_mult, 0]
        let mut packed = Vec::with_capacity(nc as usize * 8);
        for c in combos {
            packed.push(f32::from_bits(c.kama_period));
            packed.push(f32::from_bits(c.fisher_period));
            packed.push(f32::from_bits(c.atr_period));
            packed.push(f32::from_bits(c.adx_period));
            packed.push(c.adx_threshold);
            packed.push(c.atr_sl_mult);
            packed.push(c.atr_tp_mult);
            packed.push(0.0);
        }
        let params_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nnfx_params"),
            size: (nc as u64) * 32,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue
            .write_buffer(&params_buf, 0, bytemuck_cast_slice(&packed));
        self.params_buffer = Some(params_buf);

        let results_size = (nc as u64) * 36;
        self.results_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nnfx_results"),
            size: results_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        }));
        self.staging_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nnfx_staging"),
            size: results_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));

        // Dispatch NNFX eval
        let (Some(bar_buf), Some(ohlc_buf), Some(params_buf), Some(results_buf), Some(staging)) = (
            &self.bar_buffer,
            &self.ohlc_buffer,
            &self.params_buffer,
            &self.results_buffer,
            &self.staging_buffer,
        ) else {
            return None;
        };

        let uniforms = [self.bar_count, self.combo_count];
        let uniform_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nnfx_uniforms"),
            size: 8,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue
            .write_buffer(&uniform_buf, 0, bytemuck_cast_slice(&uniforms));

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nnfx_bg"),
            layout: &self.eval_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: bar_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: ohlc_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: results_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: uniform_buf.as_entire_binding(),
                },
            ],
        });

        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("nnfx_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.nnfx_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups((nc + 255) / 256, 1, 1);
        }
        encoder.copy_buffer_to_buffer(results_buf, 0, staging, 0, results_size);
        self.queue.submit(std::iter::once(encoder.finish()));

        // Readback
        let slice = staging.slice(0..results_size);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .ok();
        if rx.recv().ok()?.is_err() {
            return None;
        }
        let data = slice.get_mapped_range();
        let floats = bytemuck_cast_slice_to_f32(&data);
        drop(data);
        staging.unmap();

        let mut results = Vec::with_capacity(nc as usize);
        for i in 0..nc as usize {
            let b = i * 9;
            if b + 8 < floats.len() {
                results.push(BacktestResult {
                    net_pnl: floats[b],
                    max_drawdown: floats[b + 1],
                    sharpe: floats[b + 2],
                    sortino: floats[b + 3],
                    win_rate: floats[b + 4],
                    profit_factor: floats[b + 5],
                    trade_count: floats[b + 6] as u32,
                    avg_hold_bars: floats[b + 7],
                    robustness_score: floats[b + 8],
                });
            }
        }
        Some(results)
    }

    /// Run GPU-accelerated Monte Carlo VaR simulation.
    /// `daily_returns`: historical daily return percentages (0-100 format → converted to fraction)
    /// `simulations`: number of parallel paths (e.g., 10000)
    /// `days_forward`: simulation horizon (e.g., 252 for 1 year)
    /// `starting_equity`: initial portfolio value
    /// Returns: Vec of final equity values (one per simulation), sorted ascending.
    pub fn run_monte_carlo_gpu(
        &self,
        daily_returns: &[f32],
        simulations: u32,
        days_forward: u32,
        starting_equity: f32,
    ) -> Option<Vec<f32>> {
        if daily_returns.is_empty() || simulations == 0 {
            return None;
        }

        // Upload daily returns as "closes" buffer (repurposed)
        let returns_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mc_returns"),
            size: (daily_returns.len() * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue
            .write_buffer(&returns_buf, 0, bytemuck_cast_slice(daily_returns));

        // Empty OHLC buffer (unused by MC shader)
        let empty_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mc_empty"),
            size: 4,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        // Params: [days_forward as f32 bits, starting_equity, 0, 0...]
        let mc_params: Vec<f32> = vec![f32::from_bits(days_forward), starting_equity, 0.0, 0.0];
        let params_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mc_params"),
            size: (mc_params.len() * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue
            .write_buffer(&params_buf, 0, bytemuck_cast_slice(&mc_params));

        // Results: 1 f32 per simulation (final equity)
        let result_size = (simulations as u64) * 4;
        let results_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mc_results"),
            size: result_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mc_staging"),
            size: result_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Uniforms: [bar_count (= n_returns), combo_count (= simulations)]
        let uniforms = [daily_returns.len() as u32, simulations];
        let uniform_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mc_uniforms"),
            size: 8,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue
            .write_buffer(&uniform_buf, 0, bytemuck_cast_slice(&uniforms));

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("mc_bg"),
            layout: &self.eval_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: returns_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: empty_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: results_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: uniform_buf.as_entire_binding(),
                },
            ],
        });

        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("mc_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.monte_carlo_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups((simulations + 255) / 256, 1, 1);
        }
        encoder.copy_buffer_to_buffer(&results_buf, 0, &staging, 0, result_size);
        self.queue.submit(std::iter::once(encoder.finish()));

        // Readback
        let slice = staging.slice(0..result_size);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .ok();
        if rx.recv().ok()?.is_err() {
            return None;
        }
        let data = slice.get_mapped_range();
        let mut equities: Vec<f32> = bytemuck_cast_slice_to_f32(&data).to_vec();
        drop(data);
        staging.unmap();

        equities.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        Some(equities)
    }
}

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
