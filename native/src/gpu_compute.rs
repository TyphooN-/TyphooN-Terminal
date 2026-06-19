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

mod bind_groups;
mod pipeline_bootstrap;
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

    /// Compute SMA on GPU for a bar range (reuses uploaded OHLC buffer).
    /// Delegates to the existing dispatch_indicator_pub path so it actually
    /// produces results instead of always falling back to CPU.
    pub fn compute_sma_gpu(
        &self,
        period: u32,
        _start_bar: u32,
        _end_bar_exclusive: u32,
    ) -> Option<Vec<f32>> {
        // Use the parallel SMA path that is already implemented
        self.dispatch_indicator_pub(&Indicator::Sma, period, true)
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

    /// Upload bar data to VRAM. Called once per symbol/timeframe load.
    /// `closes`: close prices (f32 per bar) — used by SMA, EMA, RSI, KAMA, Bollinger, MACD
    /// `highs`, `lows`: used by ATR, Stochastic, ADX, Fisher
    pub fn upload_bars(&mut self, closes: &[f32]) {
        self.upload_bars_full(&[], closes, &[], &[], &[]);
    }

    /// Upload full OHLCV data to VRAM.
    /// PERF4: Buffer pool — if bar_count matches the pooled size, reuse existing buffers
    /// and only update their contents (write_buffer). Saves O(N) buffer allocations per frame
    /// when re-uploading the same chart (e.g., forming bar updates).
    pub fn upload_bars_full(
        &mut self,
        opens: &[f32],
        closes: &[f32],
        highs: &[f32],
        lows: &[f32],
        volumes: &[f32],
    ) {
        let bar_count = closes.len() as u32;
        self.bar_count = bar_count;
        let same_size = bar_count == self.pooled_bar_count && self.bar_buffer.is_some();
        let mut buffers_changed = !same_size;

        // Close prices buffer (used by most indicators)
        if !same_size || self.bar_buffer.is_none() {
            self.bar_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("bar_data"),
                size: (bar_count as u64) * 4,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            buffers_changed = true;
        }
        if let Some(ref buf) = self.bar_buffer {
            self.queue.write_buffer(buf, 0, bytemuck_cast_slice(closes));
        }

        if opens.len() == closes.len() {
            if !same_size || self.open_buffer.is_none() {
                self.open_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("open_data"),
                    size: (bar_count as u64) * 4,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }));
                buffers_changed = true;
            }
            if let Some(ref buf) = self.open_buffer {
                self.queue.write_buffer(buf, 0, bytemuck_cast_slice(opens));
            }
        } else if self.open_buffer.take().is_some() {
            buffers_changed = true;
        }

        // OHLC interleaved buffer [h0,l0,c0, h1,l1,c1, ...] for ATR/Stoch/ADX
        if highs.len() == closes.len() && lows.len() == closes.len() {
            let mut ohlc = Vec::with_capacity(bar_count as usize * 3);
            for i in 0..bar_count as usize {
                ohlc.push(highs[i]);
                ohlc.push(lows[i]);
                ohlc.push(closes[i]);
            }
            if !same_size || self.ohlc_buffer.is_none() {
                self.ohlc_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("ohlc_data"),
                    size: (bar_count as u64) * 12,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }));
                buffers_changed = true;
            }
            if let Some(ref buf) = self.ohlc_buffer {
                self.queue.write_buffer(buf, 0, bytemuck_cast_slice(&ohlc));
            }

            // Midpoints (high+low)/2 for Fisher Transform
            let mids: Vec<f32> = (0..bar_count as usize)
                .map(|i| (highs[i] + lows[i]) / 2.0)
                .collect();
            if !same_size || self.mid_buffer.is_none() {
                self.mid_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("mid_data"),
                    size: (bar_count as u64) * 4,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }));
                buffers_changed = true;
            }
            if let Some(ref buf) = self.mid_buffer {
                self.queue.write_buffer(buf, 0, bytemuck_cast_slice(&mids));
            }
        } else {
            if self.ohlc_buffer.take().is_some() {
                buffers_changed = true;
            }
            if self.mid_buffer.take().is_some() {
                buffers_changed = true;
            }
        }

        // Volume buffer for OBV
        if volumes.len() == closes.len() {
            if !same_size || self.vol_buffer.is_none() {
                self.vol_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("vol_data"),
                    size: (bar_count as u64) * 4,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }));
                buffers_changed = true;
            }
            if let Some(ref buf) = self.vol_buffer {
                self.queue
                    .write_buffer(buf, 0, bytemuck_cast_slice(volumes));
            }
        } else if self.vol_buffer.take().is_some() {
            buffers_changed = true;
        }

        // Output buffers (reusable — allocate max size needed). PERF4: only realloc on size change.
        if !same_size || self.sma_buffer.is_none() || self.ema_buffer.is_none() {
            let out_size = (bar_count as u64) * 4;
            let out_size_4x = (bar_count as u64) * 16;
            self.sma_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("sma_output"),
                size: out_size,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }));
            self.ema_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("ema_output"),
                size: out_size,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }));
            self.readback_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("readback"),
                size: out_size_4x,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            self.ind_out_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("ind_out_shared"),
                size: out_size,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }));
            // Shared custom input/output buffers — sized for max width (4 floats per bar)
            // so every dispatch_custom_input_indicator + compute_anchored_vwap call
            // reuses one allocation instead of triggering a fresh device.create_buffer
            // pair on every chart load (qstick / bop / mfi / ultosc / stochrsi /
            // atr_proj / better_volume / sd_zones / ehlers_roof) and every per-day
            // AVWAP segment (~250 segments × 3 buffers per chart load before this).
            self.custom_in_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("custom_in_shared"),
                size: out_size_4x,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            self.custom_out_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("custom_out_shared"),
                size: out_size_4x,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }));
            buffers_changed = true;
            // params buffer is fixed 8-byte, only allocate once
            if self.ind_params_buffer.is_none() {
                self.ind_params_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("ind_params_shared"),
                    size: 8,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }));
                buffers_changed = true;
            }
        }
        if self.multi_params_buffer.is_none() {
            self.multi_params_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("multi_params_shared"),
                size: 16,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            buffers_changed = true;
        }
        self.pooled_bar_count = bar_count;
        if buffers_changed {
            self.rebuild_cached_bind_groups();
        }
    }

    /// Generic dispatch: run a compute pipeline with close prices as input, return f32 per bar.
    /// Uses shared output + params buffers (populated in `upload_bars`) to avoid per-call allocations.
    fn dispatch_indicator(
        &self,
        pipeline: &wgpu::ComputePipeline,
        period: u32,
        parallel: bool,
    ) -> Option<Vec<f32>> {
        if self.bar_count == 0 {
            return None;
        }
        let rb_buf = self.readback_buffer.as_ref()?;
        let out_buf = self.ind_out_buffer.as_ref()?;
        let params_buffer = self.ind_params_buffer.as_ref()?;
        let bind_group = self.indicator_bind_group.as_ref()?;

        let out_size = (self.bar_count as u64) * 4;
        let params = [period, self.bar_count];
        self.queue
            .write_buffer(params_buffer, 0, bytemuck_cast_slice(&params));

        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("ind_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, bind_group, &[]);
            if parallel {
                pass.dispatch_workgroups((self.bar_count + 255) / 256, 1, 1);
            } else {
                pass.dispatch_workgroups(1, 1, 1);
            }
        }
        encoder.copy_buffer_to_buffer(out_buf, 0, rb_buf, 0, out_size);
        self.queue.submit(std::iter::once(encoder.finish()));

        let slice = rb_buf.slice(0..out_size);
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
        let result = bytemuck_cast_slice_to_f32(&data);
        drop(data);
        rb_buf.unmap();
        Some(result)
    }

    /// Generic dispatch with OHLC input (for ATR, Stochastic, ADX)
    /// Public OHLC dispatch that accepts an Indicator variant.
    /// Routes to the correct pipeline internally.
    pub fn dispatch_ohlc_indicator_pub(
        &self,
        indicator: &Indicator,
        period: u32,
        out_per_bar: u32,
    ) -> Option<Vec<f32>> {
        let pipeline = match indicator {
            Indicator::Atr => &self.atr_pipeline,
            Indicator::Stochastic => &self.stochastic_pipeline,
            Indicator::Adx => &self.adx_pipeline,
            Indicator::Bollinger => &self.bollinger_pipeline,
            Indicator::Ichimoku => return None, // too complex for this path
            _ => return None,
        };
        self.dispatch_ohlc_indicator(pipeline, period, out_per_bar)
    }

    pub fn dispatch_ohlc_indicator(
        &self,
        pipeline: &wgpu::ComputePipeline,
        period: u32,
        out_per_bar: u32,
    ) -> Option<Vec<f32>> {
        if self.bar_count == 0 {
            return None;
        }
        let ohlc_buf = self.ohlc_buffer.as_ref()?;
        let rb_buf = self.readback_buffer.as_ref()?;

        let out_size = (self.bar_count as u64) * (out_per_bar as u64) * 4;
        let out_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ohlc_ind_out"),
            size: out_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let params = [period, self.bar_count];
        let params_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ohlc_ind_params"),
            size: 8,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue
            .write_buffer(&params_buffer, 0, bytemuck_cast_slice(&params));

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ohlc_ind_bg"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: ohlc_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: out_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("ohlc_ind_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(1, 1, 1); // sequential for these indicators
        }
        encoder.copy_buffer_to_buffer(&out_buf, 0, rb_buf, 0, out_size.min(rb_buf.size()));
        self.queue.submit(std::iter::once(encoder.finish()));

        let read_size = out_size.min(rb_buf.size());
        let slice = rb_buf.slice(0..read_size);
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
        let result = bytemuck_cast_slice_to_f32(&data);
        drop(data);
        rb_buf.unmap();
        Some(result)
    }

    fn dispatch_custom_input_indicator(
        &self,
        input: &[f32],
        pipeline: &wgpu::ComputePipeline,
        period: u32,
        out_per_bar: u32,
        parallel: bool,
    ) -> Option<Vec<f32>> {
        if self.bar_count == 0 || input.is_empty() {
            return None;
        }
        let rb_buf = self.readback_buffer.as_ref()?;
        let input_buf = self.custom_in_buffer.as_ref()?;
        let out_buf = self.custom_out_buffer.as_ref()?;
        let params_buffer = self.ind_params_buffer.as_ref()?;
        let bind_group = self.custom_bind_group.as_ref()?;
        // Pooled custom_in_buffer is sized for 4 × bar_count × 4 bytes; refuse if a
        // caller ever exceeds that (no current caller does — BOP at 4× is widest).
        let input_bytes = (input.len() as u64) * 4;
        if input_bytes > input_buf.size() {
            return None;
        }
        self.queue
            .write_buffer(input_buf, 0, bytemuck_cast_slice(input));

        let out_size = (self.bar_count as u64) * (out_per_bar as u64) * 4;
        let params = [period, self.bar_count];
        self.queue
            .write_buffer(params_buffer, 0, bytemuck_cast_slice(&params));

        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("custom_ind_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, bind_group, &[]);
            if parallel {
                pass.dispatch_workgroups((self.bar_count + 255) / 256, 1, 1);
            } else {
                pass.dispatch_workgroups(1, 1, 1);
            }
        }
        let read_size = out_size.min(rb_buf.size());
        encoder.copy_buffer_to_buffer(out_buf, 0, rb_buf, 0, read_size);
        self.queue.submit(std::iter::once(encoder.finish()));

        let slice = rb_buf.slice(0..read_size);
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
        let result = bytemuck_cast_slice_to_f32(&data);
        drop(data);
        rb_buf.unmap();
        Some(result)
    }

    fn dispatch_multi_indicator(
        &self,
        pipeline: &wgpu::ComputePipeline,
        params: [u32; 4],
        parallel: bool,
        read_offset: u64,
        read_size: u64,
    ) -> Option<Vec<f32>> {
        if self.bar_count == 0 || read_size == 0 {
            return None;
        }
        let rb_buf = self.readback_buffer.as_ref()?;
        let out_buf = self.custom_out_buffer.as_ref()?;
        let params_buffer = self.multi_params_buffer.as_ref()?;
        let bind_group = self.multi_bind_group.as_ref()?;
        if read_size > rb_buf.size() || read_offset + read_size > out_buf.size() {
            return None;
        }
        self.queue
            .write_buffer(params_buffer, 0, bytemuck_cast_slice(&params));

        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("multi_ind_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, bind_group, &[]);
            if parallel {
                pass.dispatch_workgroups((self.bar_count + 255) / 256, 1, 1);
            } else {
                pass.dispatch_workgroups(1, 1, 1);
            }
        }
        encoder.copy_buffer_to_buffer(out_buf, read_offset, rb_buf, 0, read_size);
        self.queue.submit(std::iter::once(encoder.finish()));

        let slice = rb_buf.slice(0..read_size);
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
        let result = bytemuck_cast_slice_to_f32(&data);
        drop(data);
        rb_buf.unmap();
        Some(result)
    }

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

    /// Compute CCI on GPU from OHLC (computes typical price internally). Parallel.
    pub fn compute_cci_gpu(&self, period: u32) -> Option<Vec<f32>> {
        if self.bar_count == 0 {
            return None;
        }
        let ohlc_buf = self.ohlc_buffer.as_ref()?;
        let rb_buf = self.readback_buffer.as_ref()?;
        let out_size = (self.bar_count as u64) * 4;
        let out_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cci_out"),
            size: out_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let params = [period, self.bar_count];
        let params_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cci_params"),
            size: 8,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue
            .write_buffer(&params_buffer, 0, bytemuck_cast_slice(&params));
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("cci_bg"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: ohlc_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: out_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("cci_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.cci_ohlc_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups((self.bar_count + 255) / 256, 1, 1);
        }
        let read_size = out_size.min(rb_buf.size());
        encoder.copy_buffer_to_buffer(&out_buf, 0, rb_buf, 0, read_size);
        self.queue.submit(std::iter::once(encoder.finish()));
        let slice = rb_buf.slice(0..read_size);
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
        let result = bytemuck_cast_slice_to_f32(&data);
        drop(data);
        rb_buf.unmap();
        Some(result)
    }

    /// Compute OBV on GPU using resident close + volume buffers.
    pub fn compute_obv_gpu(&self) -> Option<Vec<f32>> {
        if self.bar_count == 0 || self.vol_buffer.is_none() {
            return None;
        }
        self.dispatch_multi_indicator(
            &self.obv_pipeline,
            [0, self.bar_count, 0, 0],
            false,
            0,
            (self.bar_count as u64) * 4,
        )
    }

    /// Compute Ehlers Super Smoother on GPU. Returns f32 per bar.
    pub fn compute_ehlers_ss_gpu(&self, period: u32) -> Option<Vec<f32>> {
        self.dispatch_indicator(&self.ehlers_ss_pipeline, period, false)
    }

    /// Compute Ehlers Decycler on GPU. Returns f32 per bar.
    pub fn compute_ehlers_dec_gpu(&self, period: u32) -> Option<Vec<f32>> {
        self.dispatch_indicator(&self.ehlers_dec_pipeline, period, false)
    }

    /// Compute Fractals on GPU. Returns [up_price, down_price] × bar_count. Parallel. Requires OHLC.
    pub fn compute_fractals_gpu(&self) -> Option<Vec<f32>> {
        self.dispatch_ohlc_indicator(&self.fractals_pipeline, 0, 2)
    }

    /// Compute ATR Projection on GPU. Uses resident open prices plus an auxiliary ATR scratch buffer.
    /// Returns [upper, lower] × bar_count.
    pub fn compute_atr_projection_gpu(&self, atrs: &[f32]) -> Option<Vec<f32>> {
        if self.bar_count == 0
            || self.open_buffer.is_none()
            || atrs.len() != self.bar_count as usize
        {
            return None;
        }
        let aux_buf = self.custom_in_buffer.as_ref()?;
        self.queue
            .write_buffer(aux_buf, 0, bytemuck_cast_slice(atrs));
        self.dispatch_multi_indicator(
            &self.atr_proj_pipeline,
            [0, self.bar_count, 0, 0],
            true,
            0,
            (self.bar_count as u64) * 8,
        )
    }

    /// Full BetterVolume GPU dispatch with resident OHLCV buffers.
    pub fn compute_better_volume_gpu_full(&self, lookback: u32) -> Option<Vec<f32>> {
        let n = self.bar_count;
        if n == 0
            || self.open_buffer.is_none()
            || self.ohlc_buffer.is_none()
            || self.vol_buffer.is_none()
        {
            return None;
        }
        self.dispatch_multi_indicator(
            &self.better_vol_pipeline,
            [lookback, n, 0, 0],
            true,
            0,
            (n as u64) * 4,
        )
    }

    /// Compute Supply/Demand zones on GPU. Returns [type, high, low] × bar_count. Parallel.
    pub fn compute_sd_zones_gpu(&self, lookback: u32) -> Option<Vec<f32>> {
        self.dispatch_ohlc_indicator(&self.sd_zones_pipeline, lookback, 3)
    }

    /// Compute HMA on GPU (WMA composition: 2*WMA(n/2) - WMA(n), then WMA(sqrt(n))).
    pub fn compute_hma_gpu(&self, period: u32) -> Option<Vec<f32>> {
        self.dispatch_indicator(&self.hma_pipeline, period, false)
    }

    /// Compute Ehlers Instantaneous Trendline on GPU.
    pub fn compute_ehlers_itl_gpu(&self) -> Option<Vec<f32>> {
        self.dispatch_indicator(&self.ehlers_itl_pipeline, 0, false)
    }

    /// Compute Ehlers Cyber Cycle on GPU.
    pub fn compute_ehlers_cyber_gpu(&self) -> Option<Vec<f32>> {
        self.dispatch_indicator(&self.ehlers_cyber_pipeline, 0, false)
    }

    /// Compute Ehlers CG Oscillator on GPU. Parallel per-bar.
    pub fn compute_ehlers_cg_gpu(&self, period: u32) -> Option<Vec<f32>> {
        self.dispatch_indicator(&self.ehlers_cg_pipeline, period, true)
    }

    /// Compute Ehlers Roofing Filter on GPU. Packs lp_period and hp_period into single u32.
    pub fn compute_ehlers_roof_gpu(&self, lp_period: u32, hp_period: u32) -> Option<Vec<f32>> {
        let packed = lp_period | (hp_period << 16);
        self.dispatch_indicator(&self.ehlers_roof_pipeline, packed, false)
    }

    /// Compute Ehlers Even Better Sinewave on GPU.
    pub fn compute_ehlers_ebsw_gpu(&self, duration: u32) -> Option<Vec<f32>> {
        self.dispatch_indicator(&self.ehlers_ebsw_pipeline, duration, false)
    }

    /// Compute Ehlers MAMA/FAMA on GPU. Returns [mama, fama] × bar_count.
    pub fn compute_ehlers_mama_gpu(&self) -> Option<Vec<f32>> {
        // MAMA outputs 2 values per bar — need larger readback buffer
        if self.bar_count == 0 {
            return None;
        }
        let bar_buf = self.bar_buffer.as_ref()?;
        let rb_buf = self.readback_buffer.as_ref()?;
        let out_size = (self.bar_count as u64) * 2 * 4;
        let out_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mama_out"),
            size: out_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let params = [0u32, self.bar_count];
        let params_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mama_params"),
            size: 8,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue
            .write_buffer(&params_buffer, 0, bytemuck_cast_slice(&params));
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("mama_bg"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: bar_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: out_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("mama_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.ehlers_mama_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }
        let read_size = out_size.min(rb_buf.size());
        encoder.copy_buffer_to_buffer(&out_buf, 0, rb_buf, 0, read_size);
        self.queue.submit(std::iter::once(encoder.finish()));
        let slice = rb_buf.slice(0..read_size);
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
        let result = bytemuck_cast_slice_to_f32(&data);
        drop(data);
        rb_buf.unmap();
        Some(result)
    }

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

    /// Compute Fisher Transform on GPU. Returns [fisher, trigger] × bar_count.
    pub fn compute_fisher_gpu(&self, period: u32) -> Option<Vec<f32>> {
        if self.bar_count == 0 {
            return None;
        }
        let mid_buf = self.mid_buffer.as_ref()?;
        let rb_buf = self.readback_buffer.as_ref()?;

        let out_size = (self.bar_count as u64) * 2 * 4;
        let out_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fisher_out"),
            size: out_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let params = [period, self.bar_count];
        let params_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fisher_params"),
            size: 8,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue
            .write_buffer(&params_buffer, 0, bytemuck_cast_slice(&params));
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fisher_bg"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: mid_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: out_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("fisher_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.fisher_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }
        let read_size = out_size.min(rb_buf.size());
        encoder.copy_buffer_to_buffer(&out_buf, 0, rb_buf, 0, read_size);
        self.queue.submit(std::iter::once(encoder.finish()));
        let slice = rb_buf.slice(0..read_size);
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
        let result = bytemuck_cast_slice_to_f32(&data);
        drop(data);
        rb_buf.unmap();
        Some(result)
    }

    /// Compute Stochastic on GPU. Returns [%K, %D] × bar_count. Requires OHLC upload.
    pub fn compute_stochastic_gpu(&self, period: u32) -> Option<Vec<f32>> {
        self.dispatch_ohlc_indicator(&self.stochastic_pipeline, period, 2)
    }

    /// Compute ADX on GPU. Returns [adx, +DI, -DI] × bar_count. Requires OHLC upload.
    pub fn compute_adx_gpu(&self, period: u32) -> Option<Vec<f32>> {
        self.dispatch_ohlc_indicator(&self.adx_pipeline, period, 3)
    }

    /// Dispatch SMA compute shader. Results stay in VRAM.
    pub fn compute_sma(&self, period: u32) {
        if self.bar_count == 0 {
            return;
        }
        let (bar_buf, out_buf) = match (&self.bar_buffer, &self.sma_buffer) {
            (Some(b), Some(o)) => (b, o),
            _ => return,
        };

        // Params uniform: [period, bar_count]
        let params = [period, self.bar_count];
        let params_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sma_params"),
            size: 8,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue
            .write_buffer(&params_buffer, 0, bytemuck_cast_slice(&params));

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sma_bg"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: bar_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: out_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("sma_encoder"),
            });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("sma_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.sma_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups((self.bar_count + 255) / 256, 1, 1);
        }
        self.queue.submit(std::iter::once(encoder.finish()));
    }

    /// Dispatch EMA compute shader. Results stay in VRAM.
    pub fn compute_ema(&self, period: u32) {
        if self.bar_count == 0 {
            return;
        }
        let (bar_buf, out_buf) = match (&self.bar_buffer, &self.ema_buffer) {
            (Some(b), Some(o)) => (b, o),
            _ => return,
        };

        let params = [period, self.bar_count];
        let params_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ema_params"),
            size: 8,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue
            .write_buffer(&params_buffer, 0, bytemuck_cast_slice(&params));

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ema_bg"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: bar_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: out_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("ema_encoder"),
            });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("ema_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.ema_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            // EMA is sequential — dispatch 1 workgroup that processes all bars
            pass.dispatch_workgroups(1, 1, 1);
        }
        self.queue.submit(std::iter::once(encoder.finish()));
    }

    /// Read back indicator results from VRAM to CPU. Async — returns via callback.
    pub fn readback_sma(&self) -> Option<Vec<f32>> {
        let (out_buf, rb_buf) = match (&self.sma_buffer, &self.readback_buffer) {
            (Some(o), Some(r)) => (o, r),
            _ => return None,
        };
        let size = (self.bar_count as u64) * 4;

        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(out_buf, 0, rb_buf, 0, size);
        self.queue.submit(std::iter::once(encoder.finish()));

        // Synchronous readback (blocking — use sparingly)
        let slice = rb_buf.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .ok();

        if rx.recv().ok()?.is_ok() {
            let data = slice.get_mapped_range();
            let result: Vec<f32> = bytemuck_cast_slice_to_f32(&data);
            drop(data);
            rb_buf.unmap();
            Some(result)
        } else {
            None
        }
    }

    pub fn bar_count(&self) -> u32 {
        self.bar_count
    }
}

// Safe byte casting via bytemuck crate — eliminates all unsafe pointer casts
fn bytemuck_cast_slice<T: bytemuck::NoUninit>(data: &[T]) -> &[u8] {
    bytemuck::cast_slice(data)
}

fn bytemuck_cast_slice_to_f32(data: &[u8]) -> Vec<f32> {
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
