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
        let Some(write) = forming_bar_gpu_write(self.bar_count, high, low, close, is_live_forming) else {
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
        // Create bind group layout: input bars (read), output indicator (read_write), params (uniform)
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("indicator_bgl"),
            entries: &[
                // Binding 0: bar data (read-only storage)
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
                // Binding 1: output indicator values (read-write storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Binding 2: params uniform (period, bar_count)
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
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
        let multi_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("multi_indicator_bgl"),
                entries: &[
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 6,
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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("indicator_pipeline_layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let multi_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("multi_indicator_pipeline_layout"),
                bind_group_layouts: &[Some(&multi_bind_group_layout)],
                immediate_size: 0,
            });

        // SMA compute shader
        let sma_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sma_shader"),
            source: wgpu::ShaderSource::Wgsl(SMA_SHADER.into()),
        });
        let sma_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("sma_pipeline"),
            layout: Some(&pipeline_layout),
            module: &sma_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        // EMA compute shader
        let ema_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ema_shader"),
            source: wgpu::ShaderSource::Wgsl(EMA_SHADER.into()),
        });
        let ema_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("ema_pipeline"),
            layout: Some(&pipeline_layout),
            module: &ema_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        // Create all indicator pipelines using same bind group layout
        let make_pipeline = |label: &str, source: &str| -> wgpu::ComputePipeline {
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        let make_multi_pipeline = |label: &str, source: &str| -> wgpu::ComputePipeline {
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(label),
                layout: Some(&multi_pipeline_layout),
                module: &shader,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        let rsi_pipeline = make_pipeline("rsi_pipeline", RSI_SHADER);
        let kama_pipeline = make_pipeline("kama_pipeline", KAMA_SHADER);
        let atr_pipeline = make_pipeline("atr_pipeline", ATR_SHADER);
        let bollinger_pipeline = make_pipeline("bollinger_pipeline", BOLLINGER_SHADER);
        let macd_pipeline = make_pipeline("macd_pipeline", MACD_SHADER);
        let fisher_pipeline = make_pipeline("fisher_pipeline", FISHER_SHADER);
        let stochastic_pipeline = make_pipeline("stochastic_pipeline", STOCHASTIC_SHADER);
        let adx_pipeline = make_pipeline("adx_pipeline", ADX_SHADER);
        let wma_pipeline = make_pipeline("wma_pipeline", WMA_SHADER);
        let cci_pipeline = make_pipeline("cci_pipeline", CCI_SHADER);
        let williams_r_pipeline = make_pipeline("williams_r_pipeline", WILLIAMS_R_SHADER);
        let obv_pipeline = make_multi_pipeline("obv_pipeline", OBV_SHADER);
        let momentum_pipeline = make_pipeline("momentum_pipeline", MOMENTUM_SHADER);
        let cmo_pipeline = make_pipeline("cmo_pipeline", CMO_SHADER);
        let qstick_pipeline = make_multi_pipeline("qstick_pipeline", QSTICK_SHADER);
        let disparity_pipeline = make_pipeline("disparity_pipeline", DISPARITY_SHADER);
        let bop_pipeline = make_multi_pipeline("bop_pipeline", BOP_SHADER);
        let stddev_pipeline = make_pipeline("stddev_pipeline", STDDEV_SHADER);
        let mfi_pipeline = make_multi_pipeline("mfi_pipeline", MFI_SHADER);
        let trix_pipeline = make_pipeline("trix_pipeline", TRIX_SHADER);
        let ppo_pipeline = make_pipeline("ppo_pipeline", PPO_SHADER);
        let ultosc_pipeline = make_multi_pipeline("ultosc_pipeline", ULTOSC_SHADER);
        let stochrsi_pipeline = make_pipeline("stochrsi_pipeline", STOCHRSI_SHADER);
        let var_osc_pipeline = make_pipeline("var_osc_pipeline", VAR_OSCILLATOR_SHADER);
        let psar_pipeline = make_pipeline("psar_pipeline", PSAR_SHADER);
        let ichimoku_pipeline = make_pipeline("ichimoku_pipeline", ICHIMOKU_SHADER);
        let cci_ohlc_pipeline = make_pipeline("cci_ohlc_pipeline", CCI_GPU_SHADER);
        let obv_gpu_pipeline = make_pipeline("obv_gpu_pipeline", OBV_GPU_SHADER);
        let ehlers_ss_pipeline = make_pipeline("ehlers_ss_pipeline", EHLERS_SUPERSMOOTHER_SHADER);
        let ehlers_dec_pipeline = make_pipeline("ehlers_dec_pipeline", EHLERS_DECYCLER_SHADER);
        let fractals_pipeline = make_pipeline("fractals_pipeline", FRACTALS_SHADER);
        let ehlers_itl_pipeline = make_pipeline("ehlers_itl_pipeline", EHLERS_ITL_SHADER);
        let ehlers_cyber_pipeline = make_pipeline("ehlers_cyber_pipeline", EHLERS_CYBER_SHADER);
        let ehlers_cg_pipeline = make_pipeline("ehlers_cg_pipeline", EHLERS_CG_SHADER);
        let ehlers_roof_pipeline = make_pipeline("ehlers_roof_pipeline", EHLERS_ROOF_SHADER);
        let ehlers_ebsw_pipeline = make_pipeline("ehlers_ebsw_pipeline", EHLERS_EBSW_SHADER);
        let ehlers_mama_pipeline = make_pipeline("ehlers_mama_pipeline", EHLERS_MAMA_SHADER);
        let hma_pipeline = make_pipeline("hma_pipeline", HMA_SHADER);
        let sd_zones_pipeline = make_pipeline("sd_zones_pipeline", SUPPLY_DEMAND_SHADER);
        let atr_proj_pipeline = make_multi_pipeline("atr_proj_pipeline", ATR_PROJECTION_SHADER);
        let better_vol_pipeline = make_multi_pipeline("better_vol_pipeline", BETTER_VOLUME_SHADER);
        let anchored_vwap_pipeline =
            make_multi_pipeline("anchored_vwap_pipeline", ANCHORED_VWAP_SHADER);
        // ADR-094: GPU parity shaders
        let supertrend_pipeline = make_pipeline("supertrend_pipeline", SUPERTREND_SHADER);
        let donchian_pipeline = make_pipeline("donchian_pipeline", DONCHIAN_SHADER);
        let keltner_pipeline = make_pipeline("keltner_pipeline", KELTNER_SHADER);
        let regression_pipeline = make_pipeline("regression_pipeline", REGRESSION_SHADER);
        let squeeze_pipeline = make_pipeline("squeeze_pipeline", SQUEEZE_SHADER);
        let prev_levels_pipeline = make_pipeline("prev_levels_pipeline", PREV_LEVELS_SHADER);

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

    fn rebuild_cached_bind_groups(&mut self) {
        self.indicator_bind_group = match (
            self.bar_buffer.as_ref(),
            self.ind_out_buffer.as_ref(),
            self.ind_params_buffer.as_ref(),
        ) {
            (Some(bar_buf), Some(out_buf), Some(params_buf)) => {
                Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("ind_bg_cached"),
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
                            resource: params_buf.as_entire_binding(),
                        },
                    ],
                }))
            }
            _ => None,
        };
        self.custom_bind_group = match (
            self.custom_in_buffer.as_ref(),
            self.custom_out_buffer.as_ref(),
            self.ind_params_buffer.as_ref(),
        ) {
            (Some(input_buf), Some(out_buf), Some(params_buf)) => {
                Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("custom_ind_bg_cached"),
                    layout: &self.bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: input_buf.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: out_buf.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: params_buf.as_entire_binding(),
                        },
                    ],
                }))
            }
            _ => None,
        };
        self.multi_bind_group =
            if let (Some(close_buf), Some(out_buf), Some(aux_buf), Some(params_buf)) = (
                self.bar_buffer.as_ref(),
                self.custom_out_buffer.as_ref(),
                self.custom_in_buffer.as_ref(),
                self.multi_params_buffer.as_ref(),
            ) {
                let open_buf = self.open_buffer.as_ref().unwrap_or(close_buf);
                let ohlc_buf = self.ohlc_buffer.as_ref().unwrap_or(close_buf);
                let vol_buf = self.vol_buffer.as_ref().unwrap_or(close_buf);
                Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("multi_ind_bg_cached"),
                    layout: &self.multi_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: open_buf.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: ohlc_buf.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: close_buf.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: vol_buf.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 4,
                            resource: aux_buf.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 5,
                            resource: out_buf.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 6,
                            resource: params_buf.as_entire_binding(),
                        },
                    ],
                }))
            } else {
                None
            };
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

const SMA_SHADER: &str = r#"
// SMA Compute Shader — parallel per-bar computation
// Each thread computes SMA for one bar by summing the lookback window

struct Params {
    period: u32,
    bar_count: u32,
}

@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.bar_count) { return; }

    if (i < params.period - 1u) {
        output[i] = 0.0;  // Not enough data for SMA
        return;
    }

    var sum: f32 = 0.0;
    for (var j: u32 = 0u; j < params.period; j = j + 1u) {
        sum = sum + bars[i - j];
    }
    output[i] = sum / f32(params.period);
}
"#;

const EMA_SHADER: &str = r#"
// EMA Compute Shader — sequential (each bar depends on previous)
// Single workgroup processes all bars in order

struct Params {
    period: u32,
    bar_count: u32,
}

@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    let k: f32 = 2.0 / (f32(params.period) + 1.0);

    // Seed with SMA of first `period` bars
    var sum: f32 = 0.0;
    for (var i: u32 = 0u; i < params.period; i = i + 1u) {
        sum = sum + bars[i];
        output[i] = 0.0;
    }
    var ema: f32 = sum / f32(params.period);
    output[params.period - 1u] = ema;

    // Recursive EMA
    for (var i: u32 = params.period; i < params.bar_count; i = i + 1u) {
        ema = bars[i] * k + ema * (1.0 - k);
        output[i] = ema;
    }
}
"#;

// ─── Additional Indicator Shaders ────────────────────────────────────────────

const RSI_SHADER: &str = r#"
// RSI Compute Shader — sequential (running average of gains/losses)
struct Params {
    period: u32,
    bar_count: u32,
}

@group(0) @binding(0) var<storage, read> bars: array<f32>;   // close prices
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    if (params.bar_count <= params.period) { return; }

    // Initial average gain/loss over first `period` changes
    var avg_gain: f32 = 0.0;
    var avg_loss: f32 = 0.0;
    for (var i: u32 = 1u; i <= params.period; i = i + 1u) {
        let change = bars[i] - bars[i - 1u];
        if (change > 0.0) { avg_gain = avg_gain + change; }
        else { avg_loss = avg_loss - change; }
        output[i - 1u] = 50.0;
    }
    avg_gain = avg_gain / f32(params.period);
    avg_loss = avg_loss / f32(params.period);

    let rs = select(avg_gain / avg_loss, 100.0, avg_loss < 0.000001);
    output[params.period] = 100.0 - 100.0 / (1.0 + rs);

    // Smoothed RSI
    for (var i: u32 = params.period + 1u; i < params.bar_count; i = i + 1u) {
        let change = bars[i] - bars[i - 1u];
        let gain = select(change, 0.0, change < 0.0);
        let loss = select(-change, 0.0, change > 0.0);
        avg_gain = (avg_gain * f32(params.period - 1u) + gain) / f32(params.period);
        avg_loss = (avg_loss * f32(params.period - 1u) + loss) / f32(params.period);
        let rs2 = select(avg_gain / avg_loss, 100.0, avg_loss < 0.000001);
        output[i] = 100.0 - 100.0 / (1.0 + rs2);
    }
}
"#;

const KAMA_SHADER: &str = r#"
// KAMA (Kaufman Adaptive Moving Average) Compute Shader — sequential
// KAMA adapts its smoothing constant based on market efficiency ratio
struct Params {
    period: u32,       // efficiency ratio lookback (e.g., 10)
    bar_count: u32,
}

@group(0) @binding(0) var<storage, read> bars: array<f32>;   // close prices
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    let fast_sc: f32 = 2.0 / 3.0;   // fast period = 2
    let slow_sc: f32 = 2.0 / 31.0;  // slow period = 30

    // Seed KAMA with first close
    var kama: f32 = bars[0];
    output[0] = kama;
    for (var i: u32 = 1u; i < params.period; i = i + 1u) {
        output[i] = bars[i];
        kama = bars[i];
    }

    // Compute KAMA
    for (var i: u32 = params.period; i < params.bar_count; i = i + 1u) {
        // Direction: absolute price change over period
        let direction = abs(bars[i] - bars[i - params.period]);
        // Volatility: sum of absolute bar-to-bar changes over period
        var volatility: f32 = 0.0;
        for (var j: u32 = i - params.period + 1u; j <= i; j = j + 1u) {
            volatility = volatility + abs(bars[j] - bars[j - 1u]);
        }
        // Efficiency Ratio
        let er = select(direction / volatility, 0.0, volatility < 0.000001);
        // Smoothing Constant = (ER × (fast_sc - slow_sc) + slow_sc)²
        let sc = er * (fast_sc - slow_sc) + slow_sc;
        let sc2 = sc * sc;
        // KAMA
        kama = kama + sc2 * (bars[i] - kama);
        output[i] = kama;
    }
}
"#;

const ATR_SHADER: &str = r#"
// ATR Compute Shader — sequential (smoothed True Range)
// Input: interleaved [high, low, close] per bar = 3 floats per bar
struct Params {
    period: u32,
    bar_count: u32,
}

@group(0) @binding(0) var<storage, read> bars: array<f32>;  // [h0,l0,c0, h1,l1,c1, ...]
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    if (params.bar_count < 2u) { return; }
    output[0] = bars[0] - bars[1];  // first TR = high - low

    // Compute True Range for all bars
    var atr_sum: f32 = bars[0] - bars[1]; // first bar TR
    for (var i: u32 = 1u; i < params.bar_count; i = i + 1u) {
        let h = bars[i * 3u];
        let l = bars[i * 3u + 1u];
        let prev_c = bars[(i - 1u) * 3u + 2u];
        let tr1 = h - l;
        let tr2 = abs(h - prev_c);
        let tr3 = abs(l - prev_c);
        let tr = max(tr1, max(tr2, tr3));

        if (i < params.period) {
            atr_sum = atr_sum + tr;
            output[i] = 0.0;
        } else if (i == params.period) {
            atr_sum = atr_sum + tr;
            output[i] = atr_sum / f32(params.period);
        } else {
            // Smoothed ATR
            output[i] = (output[i - 1u] * f32(params.period - 1u) + tr) / f32(params.period);
        }
    }
}
"#;

const BOLLINGER_SHADER: &str = r#"
// Bollinger Bands Compute Shader — parallel per-bar
// Each thread computes SMA + stddev for its lookback window
// Output: [middle, upper, lower] per bar = 3 floats per bar
struct Params {
    period: u32,
    bar_count: u32,
}

@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;  // [mid0, up0, lo0, mid1, ...]
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.bar_count) { return; }

    if (i < params.period - 1u) {
        output[i * 3u] = 0.0;
        output[i * 3u + 1u] = 0.0;
        output[i * 3u + 2u] = 0.0;
        return;
    }

    // SMA
    var sum: f32 = 0.0;
    for (var j: u32 = 0u; j < params.period; j = j + 1u) {
        sum = sum + bars[i - j];
    }
    let sma = sum / f32(params.period);

    // Standard deviation
    var var_sum: f32 = 0.0;
    for (var j: u32 = 0u; j < params.period; j = j + 1u) {
        let d = bars[i - j] - sma;
        var_sum = var_sum + d * d;
    }
    let stdev = sqrt(var_sum / f32(params.period));

    output[i * 3u] = sma;
    output[i * 3u + 1u] = sma + 2.0 * stdev;
    output[i * 3u + 2u] = sma - 2.0 * stdev;
}
"#;

const MACD_SHADER: &str = r#"
// MACD Compute Shader — sequential (two EMAs + signal EMA)
// Output: [macd_line, signal, histogram] per bar = 3 floats per bar
// params.period encodes 3 values: fast | (slow << 8) | (signal << 16)
// Default: 12 | (26 << 8) | (9 << 16) = 0x0009_1A0C
struct Params {
    period: u32,       // bit-packed: [7:0]=fast, [15:8]=slow, [23:16]=signal
    bar_count: u32,
}

@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    // Unpack periods from bit-packed param
    let fast_p = params.period & 0xFFu;
    let slow_p = (params.period >> 8u) & 0xFFu;
    let sig_p = (params.period >> 16u) & 0xFFu;
    // Fallback to standard if zero
    let fast = select(fast_p, 12u, fast_p == 0u);
    let slow = select(slow_p, 26u, slow_p == 0u);
    let sig = select(sig_p, 9u, sig_p == 0u);

    let k_fast: f32 = 2.0 / (f32(fast) + 1.0);
    let k_slow: f32 = 2.0 / (f32(slow) + 1.0);
    let k_sig: f32 = 2.0 / (f32(sig) + 1.0);

    var ema_fast: f32 = bars[0];
    var ema_slow: f32 = bars[0];
    var signal: f32 = 0.0;
    var macd_started: bool = false;

    for (var i: u32 = 0u; i < params.bar_count; i = i + 1u) {
        if (i == 0u) {
            ema_fast = bars[0]; ema_slow = bars[0];
        } else {
            ema_fast = bars[i] * k_fast + ema_fast * (1.0 - k_fast);
            ema_slow = bars[i] * k_slow + ema_slow * (1.0 - k_slow);
        }
        let macd_line = ema_fast - ema_slow;

        if (i >= slow && !macd_started) {
            signal = macd_line;
            macd_started = true;
        } else if (macd_started) {
            signal = macd_line * k_sig + signal * (1.0 - k_sig);
        }

        let hist = macd_line - signal;
        output[i * 3u] = macd_line;
        output[i * 3u + 1u] = signal;
        output[i * 3u + 2u] = hist;
    }
}
"#;

const FISHER_SHADER: &str = r#"
// Fisher Transform Compute Shader — sequential
// Ehlers Fisher Transform of normalized price
struct Params {
    period: u32,
    bar_count: u32,
}

@group(0) @binding(0) var<storage, read> bars: array<f32>;  // (high+low)/2 midpoints
@group(0) @binding(1) var<storage, read_write> output: array<f32>;  // [fisher, trigger] per bar
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    var fish: f32 = 0.0;
    var prev_fish: f32 = 0.0;
    var val: f32 = 0.0;

    for (var i: u32 = 0u; i < params.bar_count; i = i + 1u) {
        if (i < params.period) {
            output[i * 2u] = 0.0;
            output[i * 2u + 1u] = 0.0;
            continue;
        }

        // Find highest high and lowest low in period
        var highest: f32 = -1000000.0;
        var lowest: f32 = 1000000.0;
        for (var j: u32 = i - params.period + 1u; j <= i; j = j + 1u) {
            if (bars[j] > highest) { highest = bars[j]; }
            if (bars[j] < lowest) { lowest = bars[j]; }
        }

        // Normalize to -1..+1 range
        let range = highest - lowest;
        var raw: f32 = 0.0;
        if (range > 0.000001) {
            raw = 2.0 * (bars[i] - lowest) / range - 1.0;
        }
        // Clamp to (-0.999, 0.999)
        raw = max(-0.999, min(0.999, raw));
        // Smooth
        val = 0.33 * raw + 0.67 * val;
        val = max(-0.999, min(0.999, val));

        // Fisher transform
        prev_fish = fish;
        fish = 0.5 * log((1.0 + val) / (1.0 - val));

        output[i * 2u] = fish;
        output[i * 2u + 1u] = prev_fish;  // trigger = previous fisher
    }
}
"#;

const STOCHASTIC_SHADER: &str = r#"
// Stochastic Oscillator — parallel per-bar for %K, then sequential for %D
// Input: [high, low, close] interleaved (3 floats per bar)
// Output: [k, d] per bar (2 floats per bar)
struct Params {
    period: u32,       // %K period (e.g., 14)
    bar_count: u32,
}

@group(0) @binding(0) var<storage, read> bars: array<f32>;  // [h0,l0,c0, h1,l1,c1, ...]
@group(0) @binding(1) var<storage, read_write> output: array<f32>;  // [k0,d0, k1,d1, ...]
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    let d_period: u32 = 3u;  // %D smoothing period

    // Compute raw %K
    for (var i: u32 = 0u; i < params.bar_count; i = i + 1u) {
        if (i < params.period - 1u) {
            output[i * 2u] = 50.0;
            output[i * 2u + 1u] = 50.0;
            continue;
        }
        var highest: f32 = -1000000.0;
        var lowest: f32 = 1000000.0;
        for (var j: u32 = i - params.period + 1u; j <= i; j = j + 1u) {
            let h = bars[j * 3u];
            let l = bars[j * 3u + 1u];
            if (h > highest) { highest = h; }
            if (l < lowest) { lowest = l; }
        }
        let close = bars[i * 3u + 2u];
        let range = highest - lowest;
        let k = select((close - lowest) / range * 100.0, 50.0, range < 0.000001);
        output[i * 2u] = k;
    }

    // Compute %D (3-period SMA of %K)
    for (var i: u32 = 0u; i < params.bar_count; i = i + 1u) {
        if (i < params.period + d_period - 2u) {
            output[i * 2u + 1u] = output[i * 2u];
            continue;
        }
        var sum: f32 = 0.0;
        for (var j: u32 = 0u; j < d_period; j = j + 1u) {
            sum = sum + output[(i - j) * 2u];
        }
        output[i * 2u + 1u] = sum / f32(d_period);
    }
}
"#;

const ADX_SHADER: &str = r#"
// ADX (Average Directional Index) Compute Shader — sequential
// Input: [high, low, close] interleaved (3 floats per bar)
// Output: [adx, plus_di, minus_di] per bar (3 floats per bar)
struct Params {
    period: u32,       // ADX period (e.g., 14)
    bar_count: u32,
}

@group(0) @binding(0) var<storage, read> bars: array<f32>;  // [h0,l0,c0, ...]
@group(0) @binding(1) var<storage, read_write> output: array<f32>;  // [adx,+di,-di, ...]
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    if (params.bar_count < 2u) { return; }

    var smooth_plus_dm: f32 = 0.0;
    var smooth_minus_dm: f32 = 0.0;
    var smooth_tr: f32 = 0.0;
    var smooth_dx: f32 = 0.0;
    let p = f32(params.period);

    for (var i: u32 = 1u; i < params.bar_count; i = i + 1u) {
        let h = bars[i * 3u];
        let l = bars[i * 3u + 1u];
        let prev_h = bars[(i - 1u) * 3u];
        let prev_l = bars[(i - 1u) * 3u + 1u];
        let prev_c = bars[(i - 1u) * 3u + 2u];

        // True Range
        let tr = max(h - l, max(abs(h - prev_c), abs(l - prev_c)));

        // Directional Movement
        let up_move = h - prev_h;
        let down_move = prev_l - l;
        var plus_dm: f32 = 0.0;
        var minus_dm: f32 = 0.0;
        if (up_move > down_move && up_move > 0.0) { plus_dm = up_move; }
        if (down_move > up_move && down_move > 0.0) { minus_dm = down_move; }

        if (i <= params.period) {
            smooth_plus_dm = smooth_plus_dm + plus_dm;
            smooth_minus_dm = smooth_minus_dm + minus_dm;
            smooth_tr = smooth_tr + tr;
            output[i * 3u] = 0.0;
            output[i * 3u + 1u] = 0.0;
            output[i * 3u + 2u] = 0.0;
        } else {
            smooth_plus_dm = smooth_plus_dm - smooth_plus_dm / p + plus_dm;
            smooth_minus_dm = smooth_minus_dm - smooth_minus_dm / p + minus_dm;
            smooth_tr = smooth_tr - smooth_tr / p + tr;

            let plus_di = select(100.0 * smooth_plus_dm / smooth_tr, 0.0, smooth_tr < 0.000001);
            let minus_di = select(100.0 * smooth_minus_dm / smooth_tr, 0.0, smooth_tr < 0.000001);
            let di_sum = plus_di + minus_di;
            let dx = select(100.0 * abs(plus_di - minus_di) / di_sum, 0.0, di_sum < 0.000001);

            if (i == params.period + 1u) {
                smooth_dx = dx;
            } else {
                smooth_dx = (smooth_dx * (p - 1.0) + dx) / p;
            }

            output[i * 3u] = smooth_dx;
            output[i * 3u + 1u] = plus_di;
            output[i * 3u + 2u] = minus_di;
        }
    }
    output[0] = 0.0; output[1] = 0.0; output[2] = 0.0;
}
"#;

// ─── Additional Indicator Shaders (Phase 2 batch) ────────────────────────────

const WMA_SHADER: &str = r#"
// Weighted Moving Average — parallel per-bar
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.bar_count || i < params.period - 1u) { output[i] = 0.0; return; }
    var weighted_sum: f32 = 0.0;
    var weight_total: f32 = 0.0;
    for (var j: u32 = 0u; j < params.period; j = j + 1u) {
        let w = f32(params.period - j);
        weighted_sum = weighted_sum + bars[i - j] * w;
        weight_total = weight_total + w;
    }
    output[i] = weighted_sum / weight_total;
}
"#;

const CCI_SHADER: &str = r#"
// Commodity Channel Index — parallel per-bar (uses typical price from OHLC)
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;  // typical prices (H+L+C)/3
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.bar_count || i < params.period - 1u) { output[i] = 0.0; return; }
    // SMA of typical price
    var sum: f32 = 0.0;
    for (var j: u32 = 0u; j < params.period; j = j + 1u) { sum = sum + bars[i - j]; }
    let sma = sum / f32(params.period);
    // Mean deviation
    var md: f32 = 0.0;
    for (var j: u32 = 0u; j < params.period; j = j + 1u) { md = md + abs(bars[i - j] - sma); }
    md = md / f32(params.period);
    output[i] = select((bars[i] - sma) / (0.015 * md), 0.0, md < 0.000001);
}
"#;

const WILLIAMS_R_SHADER: &str = r#"
// Williams %R — parallel per-bar (uses OHLC)
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;  // [h,l,c] interleaved
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.bar_count || i < params.period - 1u) { output[i] = -50.0; return; }
    var hh: f32 = -1000000.0;
    var ll: f32 = 1000000.0;
    for (var j: u32 = 0u; j < params.period; j = j + 1u) {
        let idx = i - j;
        let h = bars[idx * 3u];
        let l = bars[idx * 3u + 1u];
        if (h > hh) { hh = h; }
        if (l < ll) { ll = l; }
    }
    let close = bars[i * 3u + 2u];
    let range = hh - ll;
    output[i] = select((hh - close) / range * -100.0, -50.0, range < 0.000001);
}
"#;

const OBV_SHADER: &str = r#"
// On-Balance Volume — sequential (cumulative) using resident close + volume buffers.
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(2) var<storage, read> close_bars: array<f32>;
@group(0) @binding(3) var<storage, read> volumes: array<f32>;
@group(0) @binding(5) var<storage, read_write> output: array<f32>;
@group(0) @binding(6) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    if (params.bar_count == 0u) { return; }
    output[0] = 0.0;
    var obv: f32 = 0.0;
    for (var i: u32 = 1u; i < params.bar_count; i = i + 1u) {
        let close = close_bars[i];
        let prev_close = close_bars[i - 1u];
        let vol = volumes[i];
        if (close > prev_close) { obv = obv + vol; }
        else if (close < prev_close) { obv = obv - vol; }
        output[i] = obv;
    }
}
"#;

const MOMENTUM_SHADER: &str = r#"
// Momentum — parallel per-bar (simple price difference)
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.bar_count) { output[i] = 0.0; return; }
    if (i < params.period) { output[i] = 0.0; return; }
    output[i] = bars[i] - bars[i - params.period];
}
"#;

const CMO_SHADER: &str = r#"
// CMO — parallel rolling gain/loss spread on closes.
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.bar_count) { return; }
    if (params.period == 0u || i < params.period) {
        output[i] = 0.0;
        return;
    }
    var sum_up: f32 = 0.0;
    var sum_dn: f32 = 0.0;
    let start = i + 1u - params.period;
    for (var j: u32 = start; j <= i; j = j + 1u) {
        let delta = bars[j] - bars[j - 1u];
        if (delta > 0.0) {
            sum_up = sum_up + delta;
        } else if (delta < 0.0) {
            sum_dn = sum_dn - delta;
        }
    }
    let denom = sum_up + sum_dn;
    var value: f32 = 0.0;
    if (denom > 1e-6) {
        value = 100.0 * (sum_up - sum_dn) / denom;
    }
    output[i] = value;
}
"#;

const QSTICK_SHADER: &str = r#"
// QStick — parallel SMA of candle body using resident open + close buffers.
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> open_bars: array<f32>;
@group(0) @binding(2) var<storage, read> close_bars: array<f32>;
@group(0) @binding(5) var<storage, read_write> output: array<f32>;
@group(0) @binding(6) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.bar_count) { return; }
    if (params.period == 0u || i + 1u < params.period) {
        output[i] = 0.0;
        return;
    }
    var sum: f32 = 0.0;
    let start = i + 1u - params.period;
    for (var j: u32 = start; j <= i; j = j + 1u) {
        let open = open_bars[j];
        let close = close_bars[j];
        sum = sum + (close - open);
    }
    output[i] = sum / f32(params.period);
}
"#;

const DISPARITY_SHADER: &str = r#"
// Disparity Index — parallel % deviation of close from SMA(period).
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.bar_count) { return; }
    if (params.period == 0u || i + 1u < params.period) {
        output[i] = 0.0;
        return;
    }
    var sum: f32 = 0.0;
    let start = i + 1u - params.period;
    for (var j: u32 = start; j <= i; j = j + 1u) {
        sum = sum + bars[j];
    }
    let sma = sum / f32(params.period);
    var value: f32 = 0.0;
    if (abs(sma) > 1e-6) {
        value = (bars[i] / sma - 1.0) * 100.0;
    }
    output[i] = value;
}
"#;

const BOP_SHADER: &str = r#"
// BOP — parallel SMA of (close-open)/(high-low) using resident open + OHLC buffers.
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> open_bars: array<f32>;
@group(0) @binding(1) var<storage, read> ohlc_bars: array<f32>;
@group(0) @binding(5) var<storage, read_write> output: array<f32>;
@group(0) @binding(6) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.bar_count) { return; }
    if (params.period == 0u || i + 1u < params.period) {
        output[i] = 0.0;
        return;
    }
    var sum: f32 = 0.0;
    let start = i + 1u - params.period;
    for (var j: u32 = start; j <= i; j = j + 1u) {
        let base = j * 3u;
        let open = open_bars[j];
        let high = ohlc_bars[base];
        let low = ohlc_bars[base + 1u];
        let close = ohlc_bars[base + 2u];
        let range = max(high - low, 1e-6);
        sum = sum + (close - open) / range;
    }
    output[i] = sum / f32(params.period);
}
"#;

const STDDEV_SHADER: &str = r#"
// StdDev — parallel rolling sample standard deviation of closes.
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.bar_count) { return; }
    if (params.period < 2u || i + 1u < params.period) {
        output[i] = 0.0;
        return;
    }
    var sum: f32 = 0.0;
    let start = i + 1u - params.period;
    for (var j: u32 = start; j <= i; j = j + 1u) {
        sum = sum + bars[j];
    }
    let mean = sum / f32(params.period);
    var ss: f32 = 0.0;
    for (var j: u32 = start; j <= i; j = j + 1u) {
        let d = bars[j] - mean;
        ss = ss + d * d;
    }
    output[i] = sqrt(max(ss / f32(params.period - 1u), 0.0));
}
"#;

const MFI_SHADER: &str = r#"
// MFI — parallel Money Flow Index using resident OHLC + volume buffers.
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(1) var<storage, read> ohlc_bars: array<f32>;
@group(0) @binding(3) var<storage, read> volumes: array<f32>;
@group(0) @binding(5) var<storage, read_write> output: array<f32>;
@group(0) @binding(6) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.bar_count) { return; }
    if (params.period == 0u || i < params.period) {
        output[i] = 0.0;
        return;
    }
    var pos_sum: f32 = 0.0;
    var neg_sum: f32 = 0.0;
    let start = i + 1u - params.period;
    for (var j: u32 = start; j <= i; j = j + 1u) {
        let base = j * 3u;
        let prev_base = (j - 1u) * 3u;
        let tp = (ohlc_bars[base] + ohlc_bars[base + 1u] + ohlc_bars[base + 2u]) / 3.0;
        let prev_tp =
            (ohlc_bars[prev_base] + ohlc_bars[prev_base + 1u] + ohlc_bars[prev_base + 2u]) / 3.0;
        let money_flow = tp * max(volumes[j], 0.0);
        if (tp > prev_tp) {
            pos_sum = pos_sum + money_flow;
        } else if (tp < prev_tp) {
            neg_sum = neg_sum + money_flow;
        }
    }
    if (neg_sum <= 1e-6) {
        output[i] = select(100.0, 50.0, pos_sum <= 1e-6);
        return;
    }
    let ratio = pos_sum / neg_sum;
    output[i] = clamp(100.0 - 100.0 / (1.0 + ratio), 0.0, 100.0);
}
"#;

const TRIX_SHADER: &str = r#"
// TRIX — sequential triple-EMA ROC with signal EMA.
// params.period encodes: [7:0]=period, [15:8]=signal period.
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    let period_raw = params.period & 0xFFu;
    let signal_raw = (params.period >> 8u) & 0xFFu;
    let period = select(period_raw, 15u, period_raw == 0u);
    let signal_period = select(signal_raw, 9u, signal_raw == 0u);
    let k = 2.0 / (f32(period) + 1.0);
    let sig_k = 2.0 / (f32(signal_period) + 1.0);

    var ema1 = bars[0];
    var ema2 = bars[0];
    var ema3 = bars[0];
    var prev_ema3 = bars[0];
    var signal = 0.0;
    var signal_started = false;

    for (var i: u32 = 0u; i < params.bar_count; i = i + 1u) {
        if (i > 0u) {
            ema1 = bars[i] * k + ema1 * (1.0 - k);
            ema2 = ema1 * k + ema2 * (1.0 - k);
            ema3 = ema2 * k + ema3 * (1.0 - k);
        }
        var trix = 0.0;
        if (i > 0u && abs(prev_ema3) > 1e-6) {
            trix = 100.0 * (ema3 / prev_ema3 - 1.0);
        }
        if (i + 1u >= signal_period) {
            if (!signal_started) {
                signal = trix;
                signal_started = true;
            } else {
                signal = trix * sig_k + signal * (1.0 - sig_k);
            }
        }
        output[i * 3u] = trix;
        output[i * 3u + 1u] = signal;
        output[i * 3u + 2u] = trix - signal;
        prev_ema3 = ema3;
    }
}
"#;

const PPO_SHADER: &str = r#"
// PPO — sequential percentage price oscillator with signal EMA.
// params.period encodes: [7:0]=fast, [15:8]=slow, [23:16]=signal.
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    let fast_raw = params.period & 0xFFu;
    let slow_raw = (params.period >> 8u) & 0xFFu;
    let signal_raw = (params.period >> 16u) & 0xFFu;
    let fast = select(fast_raw, 12u, fast_raw == 0u);
    let slow = select(slow_raw, 26u, slow_raw == 0u);
    let signal_period = select(signal_raw, 9u, signal_raw == 0u);
    let k_fast = 2.0 / (f32(fast) + 1.0);
    let k_slow = 2.0 / (f32(slow) + 1.0);
    let k_sig = 2.0 / (f32(signal_period) + 1.0);

    var ema_fast = bars[0];
    var ema_slow = bars[0];
    var signal = 0.0;
    var signal_started = false;

    for (var i: u32 = 0u; i < params.bar_count; i = i + 1u) {
        if (i > 0u) {
            ema_fast = bars[i] * k_fast + ema_fast * (1.0 - k_fast);
            ema_slow = bars[i] * k_slow + ema_slow * (1.0 - k_slow);
        }
        var ppo = 0.0;
        if (abs(ema_slow) > 1e-6) {
            ppo = 100.0 * (ema_fast - ema_slow) / ema_slow;
        }
        if (i + 1u >= signal_period) {
            if (!signal_started) {
                signal = ppo;
                signal_started = true;
            } else {
                signal = ppo * k_sig + signal * (1.0 - k_sig);
            }
        }
        output[i * 3u] = ppo;
        output[i * 3u + 1u] = signal;
        output[i * 3u + 2u] = ppo - signal;
    }
}
"#;

const ULTOSC_SHADER: &str = r#"
// Ultimate Oscillator — sequential weighted BP/TR average using resident OHLC buffers.
// params.period encodes: [7:0]=short, [15:8]=mid, [23:16]=long.
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(1) var<storage, read> ohlc_bars: array<f32>;
@group(0) @binding(5) var<storage, read_write> output: array<f32>;
@group(0) @binding(6) var<uniform> params: Params;

fn true_low(low: f32, prev_close: f32) -> f32 {
    return min(low, prev_close);
}

fn true_high(high: f32, prev_close: f32) -> f32 {
    return max(high, prev_close);
}

@compute @workgroup_size(1)
fn main() {
    let p1_raw = params.period & 0xFFu;
    let p2_raw = (params.period >> 8u) & 0xFFu;
    let p3_raw = (params.period >> 16u) & 0xFFu;
    let p1 = select(p1_raw, 7u, p1_raw == 0u);
    let p2 = select(p2_raw, 14u, p2_raw == 0u);
    let p3 = select(p3_raw, 28u, p3_raw == 0u);

    output[0] = 0.0;
    for (var i: u32 = 1u; i < params.bar_count; i = i + 1u) {
        if (i < p3) {
            output[i] = 0.0;
            continue;
        }
        var bp1: f32 = 0.0;
        var bp2: f32 = 0.0;
        var bp3: f32 = 0.0;
        var tr1: f32 = 0.0;
        var tr2: f32 = 0.0;
        var tr3: f32 = 0.0;
        for (var j: u32 = i + 1u - p3; j <= i; j = j + 1u) {
            let base = j * 3u;
            let prev_base = (j - 1u) * 3u;
            let prev_close = ohlc_bars[prev_base + 2u];
            let high = ohlc_bars[base];
            let low = ohlc_bars[base + 1u];
            let close = ohlc_bars[base + 2u];
            let bp = close - true_low(low, prev_close);
            let tr = max(true_high(high, prev_close) - true_low(low, prev_close), 1e-6);
            bp3 = bp3 + bp;
            tr3 = tr3 + tr;
            if (j + p2 >= i + 1u) {
                bp2 = bp2 + bp;
                tr2 = tr2 + tr;
            }
            if (j + p1 >= i + 1u) {
                bp1 = bp1 + bp;
                tr1 = tr1 + tr;
            }
        }
        let avg1 = bp1 / max(tr1, 1e-6);
        let avg2 = bp2 / max(tr2, 1e-6);
        let avg3 = bp3 / max(tr3, 1e-6);
        output[i] = clamp(100.0 * (4.0 * avg1 + 2.0 * avg2 + avg3) / 7.0, 0.0, 100.0);
    }
}
"#;

const STOCHRSI_SHADER: &str = r#"
// StochRSI — sequential RSI, raw StochRSI, %K, then %D.
// params.period encodes: [7:0]=rsi, [15:8]=stoch, [23:16]=k, [31:24]=d.
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

fn compute_rsi_value(avg_gain: f32, avg_loss: f32) -> f32 {
    if (avg_loss <= 1e-6) {
        return select(100.0, 50.0, avg_gain <= 1e-6);
    }
    let rs = avg_gain / avg_loss;
    return 100.0 - 100.0 / (1.0 + rs);
}

@compute @workgroup_size(1)
fn main() {
    let rsi_raw = params.period & 0xFFu;
    let stoch_raw = (params.period >> 8u) & 0xFFu;
    let k_raw = (params.period >> 16u) & 0xFFu;
    let d_raw = (params.period >> 24u) & 0xFFu;
    let rsi_period = select(rsi_raw, 14u, rsi_raw == 0u);
    let stoch_period = select(stoch_raw, 14u, stoch_raw == 0u);
    let k_period = select(k_raw, 3u, k_raw == 0u);
    let d_period = select(d_raw, 3u, d_raw == 0u);

    output[0] = 50.0;
    output[1] = 0.0;
    var avg_gain: f32 = 0.0;
    var avg_loss: f32 = 0.0;
    for (var i: u32 = 1u; i < params.bar_count; i = i + 1u) {
        let delta = bars[i] - bars[i - 1u];
        let gain = max(delta, 0.0);
        let loss = max(-delta, 0.0);
        if (i < rsi_period) {
            avg_gain = avg_gain + gain;
            avg_loss = avg_loss + loss;
            output[i * 2u] = 50.0;
        } else if (i == rsi_period) {
            avg_gain = (avg_gain + gain) / f32(rsi_period);
            avg_loss = (avg_loss + loss) / f32(rsi_period);
            output[i * 2u] = compute_rsi_value(avg_gain, avg_loss);
        } else {
            avg_gain = (avg_gain * f32(rsi_period - 1u) + gain) / f32(rsi_period);
            avg_loss = (avg_loss * f32(rsi_period - 1u) + loss) / f32(rsi_period);
            output[i * 2u] = compute_rsi_value(avg_gain, avg_loss);
        }
        output[i * 2u + 1u] = 0.0;
    }

    for (var i: u32 = 0u; i < params.bar_count; i = i + 1u) {
        if (i < stoch_period - 1u) {
            output[i * 2u + 1u] = 0.0;
            continue;
        }
        var min_rsi = 1e9;
        var max_rsi = -1e9;
        let start = i + 1u - stoch_period;
        for (var j: u32 = start; j <= i; j = j + 1u) {
            let rsi = output[j * 2u];
            min_rsi = min(min_rsi, rsi);
            max_rsi = max(max_rsi, rsi);
        }
        let range = max_rsi - min_rsi;
        output[i * 2u + 1u] = select(
            clamp((output[i * 2u] - min_rsi) / range * 100.0, 0.0, 100.0),
            50.0,
            range <= 1e-6,
        );
    }

    for (var i: u32 = 0u; i < params.bar_count; i = i + 1u) {
        if (i < k_period - 1u) {
            output[i * 2u] = 0.0;
            continue;
        }
        let start = i + 1u - k_period;
        var sum_k: f32 = 0.0;
        for (var j: u32 = start; j <= i; j = j + 1u) {
            sum_k = sum_k + output[j * 2u + 1u];
        }
        output[i * 2u] = clamp(sum_k / f32(k_period), 0.0, 100.0);
    }

    for (var i: u32 = 0u; i < params.bar_count; i = i + 1u) {
        if (i < d_period - 1u) {
            output[i * 2u + 1u] = 0.0;
            continue;
        }
        let start = i + 1u - d_period;
        var sum_d: f32 = 0.0;
        for (var j: u32 = start; j <= i; j = j + 1u) {
            sum_d = sum_d + output[j * 2u];
        }
        output[i * 2u + 1u] = clamp(sum_d / f32(d_period), 0.0, 100.0);
    }
}
"#;

const VAR_OSCILLATOR_SHADER: &str = r#"
// VaR oscillator — sequential rolling parametric VaR (95%) on close-to-close log returns.
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

const VAR_Z95: f32 = 1.6448536;
const VAR_EPS: f32 = 1e-6;

@compute @workgroup_size(1)
fn main() {
    for (var i: u32 = 0u; i < params.bar_count; i = i + 1u) {
        output[i] = 0.0;
    }
    if (params.period == 0u || params.bar_count <= params.period) { return; }

    for (var i: u32 = params.period; i < params.bar_count; i = i + 1u) {
        var sum: f32 = 0.0;
        var sum_sq: f32 = 0.0;
        let start = i + 1u - params.period;
        for (var j: u32 = start; j <= i; j = j + 1u) {
            let prev_close = max(bars[j - 1u], VAR_EPS);
            let close = max(bars[j], VAR_EPS);
            let ret = log(close / prev_close);
            sum = sum + ret;
            sum_sq = sum_sq + ret * ret;
        }

        let count = f32(params.period);
        let mean = sum / count;
        let variance = max(sum_sq / count - mean * mean, 0.0);
        let sigma = sqrt(variance);
        let var95 = max(VAR_EPS, VAR_Z95 * sigma - mean);

        let prev_close = max(bars[i - 1u], VAR_EPS);
        let close = max(bars[i], VAR_EPS);
        let current_ret = log(close / prev_close);
        output[i] = -100.0 * current_ret / var95;
    }
}
"#;

const PSAR_SHADER: &str = r#"
// Parabolic SAR — sequential (state machine)
struct Params { period: u32, bar_count: u32, }  // period unused, af_step=0.02, af_max=0.2 hardcoded
@group(0) @binding(0) var<storage, read> bars: array<f32>;  // [h,l,c] interleaved
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    if (params.bar_count < 3u) { return; }
    let af_step: f32 = 0.02;
    let af_max: f32 = 0.2;

    // Initialize: start long
    var is_long: bool = true;
    var af: f32 = af_step;
    var ep: f32 = bars[0];      // extreme point = first high
    var sar: f32 = bars[1];     // start at first low
    output[0] = sar;
    output[1] = sar;

    for (var i: u32 = 2u; i < params.bar_count; i = i + 1u) {
        let h = bars[i * 3u];
        let l = bars[i * 3u + 1u];

        var new_sar = sar + af * (ep - sar);

        if (is_long) {
            // Clamp SAR below prior two lows
            let prev_l = bars[(i - 1u) * 3u + 1u];
            let prev2_l = bars[(i - 2u) * 3u + 1u];
            new_sar = min(new_sar, min(prev_l, prev2_l));

            if (l < new_sar) {
                // Reverse to short
                is_long = false;
                new_sar = ep;
                ep = l;
                af = af_step;
            } else {
                if (h > ep) {
                    ep = h;
                    af = min(af + af_step, af_max);
                }
            }
        } else {
            // Clamp SAR above prior two highs
            let prev_h = bars[(i - 1u) * 3u];
            let prev2_h = bars[(i - 2u) * 3u];
            new_sar = max(new_sar, max(prev_h, prev2_h));

            if (h > new_sar) {
                // Reverse to long
                is_long = true;
                new_sar = ep;
                ep = h;
                af = af_step;
            } else {
                if (l < ep) {
                    ep = l;
                    af = min(af + af_step, af_max);
                }
            }
        }

        sar = new_sar;
        output[i] = sar;
    }
}
"#;

// ─── Phase 3 Indicators: Ichimoku, CCI, OBV, Ehlers, Fractals ──────────────

const ICHIMOKU_SHADER: &str = r#"
// Ichimoku Kinko Hyo — sequential (4 outputs: tenkan, kijun, span_a, span_b)
// Input: [high, low, close] interleaved
// Output: [tenkan, kijun, span_a, span_b] × bar_count = 4 floats per bar
struct Params { period: u32, bar_count: u32, }  // period unused, hardcoded 9/26/52
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

fn highest_high(start: u32, len: u32) -> f32 {
    var hh: f32 = -1000000.0;
    for (var j: u32 = 0u; j < len; j = j + 1u) {
        let h = bars[(start + j) * 3u];
        if (h > hh) { hh = h; }
    }
    return hh;
}
fn lowest_low(start: u32, len: u32) -> f32 {
    var ll: f32 = 1000000.0;
    for (var j: u32 = 0u; j < len; j = j + 1u) {
        let l = bars[(start + j) * 3u + 1u];
        if (l < ll) { ll = l; }
    }
    return ll;
}

@compute @workgroup_size(1)
fn main() {
    let tenkan_p: u32 = 9u;
    let kijun_p: u32 = 26u;
    let senkou_b_p: u32 = 52u;

    for (var i: u32 = 0u; i < params.bar_count; i = i + 1u) {
        let base = i * 4u;
        // Tenkan-sen (9-period midpoint)
        if (i >= tenkan_p - 1u) {
            let start = i - tenkan_p + 1u;
            output[base] = (highest_high(start, tenkan_p) + lowest_low(start, tenkan_p)) / 2.0;
        } else { output[base] = 0.0; }

        // Kijun-sen (26-period midpoint)
        if (i >= kijun_p - 1u) {
            let start = i - kijun_p + 1u;
            output[base + 1u] = (highest_high(start, kijun_p) + lowest_low(start, kijun_p)) / 2.0;
        } else { output[base + 1u] = 0.0; }

        // Senkou Span A (midpoint of tenkan + kijun, projected 26 forward)
        if (i >= kijun_p - 1u) {
            output[base + 2u] = (output[base] + output[base + 1u]) / 2.0;
        } else { output[base + 2u] = 0.0; }

        // Senkou Span B (52-period midpoint, projected 26 forward)
        if (i >= senkou_b_p - 1u) {
            let start = i - senkou_b_p + 1u;
            output[base + 3u] = (highest_high(start, senkou_b_p) + lowest_low(start, senkou_b_p)) / 2.0;
        } else { output[base + 3u] = 0.0; }
    }
}
"#;

const CCI_GPU_SHADER: &str = r#"
// CCI with built-in typical price computation from OHLC
// Input: [high, low, close] interleaved
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.bar_count || i < params.period - 1u) { output[i] = 0.0; return; }

    // Compute typical prices and SMA
    var sum: f32 = 0.0;
    for (var j: u32 = 0u; j < params.period; j = j + 1u) {
        let idx = i - j;
        let tp = (bars[idx * 3u] + bars[idx * 3u + 1u] + bars[idx * 3u + 2u]) / 3.0;
        sum = sum + tp;
    }
    let sma = sum / f32(params.period);

    // Mean deviation
    var md: f32 = 0.0;
    for (var j: u32 = 0u; j < params.period; j = j + 1u) {
        let idx = i - j;
        let tp = (bars[idx * 3u] + bars[idx * 3u + 1u] + bars[idx * 3u + 2u]) / 3.0;
        md = md + abs(tp - sma);
    }
    md = md / f32(params.period);

    let tp_now = (bars[i * 3u] + bars[i * 3u + 1u] + bars[i * 3u + 2u]) / 3.0;
    output[i] = select((tp_now - sma) / (0.015 * md), 0.0, md < 0.000001);
}
"#;

const OBV_GPU_SHADER: &str = r#"
// OBV with close+volume from OHLC buffer (close at offset 2, volume separate)
// Input binding 0: close prices, binding 1 is output
// We'll use a separate volume buffer approach — close prices in bars, volume uploaded separately
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;  // close prices
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    // OBV without volume data — use price change magnitude as proxy
    // (Real OBV requires volume buffer; this is a reasonable GPU approximation)
    if (params.bar_count == 0u) { return; }
    output[0] = 0.0;
    var obv: f32 = 0.0;
    for (var i: u32 = 1u; i < params.bar_count; i = i + 1u) {
        let change = bars[i] - bars[i - 1u];
        if (change > 0.0) { obv = obv + abs(change); }
        else if (change < 0.0) { obv = obv - abs(change); }
        output[i] = obv;
    }
}
"#;

const EHLERS_SUPERSMOOTHER_SHADER: &str = r#"
// Ehlers Super Smoother — 2-pole Butterworth low-pass filter (sequential)
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    let pi: f32 = 3.14159265;
    let a = exp(-1.414 * pi / f32(params.period));
    let b = 2.0 * a * cos(1.414 * pi / f32(params.period));
    let c2 = b;
    let c3 = -a * a;
    let c1 = 1.0 - c2 - c3;

    for (var i: u32 = 0u; i < params.bar_count; i = i + 1u) {
        if (i < 2u) {
            output[i] = bars[i];
        } else {
            output[i] = c1 * (bars[i] + bars[i - 1u]) / 2.0 + c2 * output[i - 1u] + c3 * output[i - 2u];
        }
    }
}
"#;

const EHLERS_DECYCLER_SHADER: &str = r#"
// Ehlers Decycler — price minus super-smoothed component (sequential)
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    let pi: f32 = 3.14159265;
    let a = exp(-1.414 * pi / f32(params.period));
    let b = 2.0 * a * cos(1.414 * pi / f32(params.period));
    let c2 = b;
    let c3 = -a * a;
    let c1 = 1.0 - c2 - c3;

    // First compute super smoother
    var ss: array<f32, 2>;
    ss[0] = bars[0]; ss[1] = bars[min(1u, params.bar_count - 1u)];
    output[0] = 0.0;
    if (params.bar_count > 1u) { output[1] = 0.0; }

    for (var i: u32 = 2u; i < params.bar_count; i = i + 1u) {
        let smoothed = c1 * (bars[i] + bars[i - 1u]) / 2.0 + c2 * ss[1] + c3 * ss[0];
        output[i] = bars[i] - smoothed;
        ss[0] = ss[1];
        ss[1] = smoothed;
    }
}
"#;

const FRACTALS_SHADER: &str = r#"
// Fractals (Williams) — parallel per-bar
// Fractal Up: high[i] > high[i-2..i+2] (5-bar pattern)
// Fractal Down: low[i] < low[i-2..i+2]
// Output: [fractal_up_flag, fractal_down_flag] per bar (2 floats, 0.0 or price)
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;  // [h,l,c] interleaved
@group(0) @binding(1) var<storage, read_write> output: array<f32>;  // [up, down] per bar
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.bar_count) { return; }
    let base = i * 2u;

    if (i < 2u || i + 2u >= params.bar_count) {
        output[base] = 0.0;
        output[base + 1u] = 0.0;
        return;
    }

    let h = bars[i * 3u];
    let l = bars[i * 3u + 1u];

    // Fractal up: current high > surrounding 4 highs
    let is_up = h > bars[(i - 2u) * 3u] && h > bars[(i - 1u) * 3u]
             && h > bars[(i + 1u) * 3u] && h > bars[(i + 2u) * 3u];
    output[base] = select(0.0, h, is_up);

    // Fractal down: current low < surrounding 4 lows
    let is_down = l < bars[(i - 2u) * 3u + 1u] && l < bars[(i - 1u) * 3u + 1u]
               && l < bars[(i + 1u) * 3u + 1u] && l < bars[(i + 2u) * 3u + 1u];
    output[base + 1u] = select(0.0, l, is_down);
}
"#;

const HMA_SHADER: &str = r#"
// Hull Moving Average — sequential (WMA composition: 2*WMA(n/2) - WMA(n), then WMA(sqrt(n)))
// All WMA computations inlined (WGSL can't pass storage pointers to functions)
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    let n = params.period;
    let half_n = max(n / 2u, 1u);
    let sqrt_n = max(u32(sqrt(f32(n))), 1u);

    // Step 1: Compute delta = 2*WMA(n/2) - WMA(n) into output
    for (var i: u32 = 0u; i < params.bar_count; i = i + 1u) {
        if (i < n - 1u) { output[i] = 0.0; continue; }

        // Inline WMA(half_n) on bars
        var ws_half: f32 = 0.0;
        var wt_half: f32 = 0.0;
        for (var j: u32 = 0u; j < half_n; j = j + 1u) {
            let w = f32(half_n - j);
            ws_half = ws_half + bars[i - j] * w;
            wt_half = wt_half + w;
        }
        let wma_half = ws_half / wt_half;

        // Inline WMA(n) on bars
        var ws_full: f32 = 0.0;
        var wt_full: f32 = 0.0;
        for (var j: u32 = 0u; j < n; j = j + 1u) {
            let w = f32(n - j);
            ws_full = ws_full + bars[i - j] * w;
            wt_full = wt_full + w;
        }
        let wma_full = ws_full / wt_full;

        output[i] = 2.0 * wma_half - wma_full;
    }

    // Step 2: WMA(sqrt_n) of the delta series (stored in output)
    // Copy delta to temp array first (can't read and write same buffer safely)
    var temp: array<f32, 512>;
    let copy_len = min(params.bar_count, 512u);
    for (var i: u32 = 0u; i < copy_len; i = i + 1u) {
        temp[i] = output[i];
    }

    for (var i: u32 = 0u; i < params.bar_count; i = i + 1u) {
        if (i < n - 1u + sqrt_n - 1u) { output[i] = 0.0; continue; }
        if (i >= 512u) { output[i] = 0.0; continue; }  // safety bound

        var ws: f32 = 0.0;
        var wt: f32 = 0.0;
        for (var j: u32 = 0u; j < sqrt_n; j = j + 1u) {
            let w = f32(sqrt_n - j);
            ws = ws + temp[i - j] * w;
            wt = wt + w;
        }
        output[i] = select(ws / wt, 0.0, wt < 0.000001);
    }
}
"#;

const EHLERS_ITL_SHADER: &str = r#"
// Ehlers Instantaneous Trendline — sequential IIR
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    for (var i: u32 = 0u; i < min(7u, params.bar_count); i = i + 1u) { output[i] = bars[i]; }
    for (var i: u32 = 7u; i < params.bar_count; i = i + 1u) {
        var itl = (bars[i] + 2.0 * bars[i - 1u] + bars[i - 2u]) / 4.0 * 0.5 + output[i - 1u] * 0.5;
        itl = (2.0 * itl + output[i - 1u] + output[i - 2u] + output[i - 3u]) / 5.0;
        output[i] = itl;
    }
}
"#;

const EHLERS_CYBER_SHADER: &str = r#"
// Ehlers Cyber Cycle — sequential 2nd-order bandpass
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    let alpha: f32 = 0.07;
    let c1: f32 = (1.0 - 0.5 * alpha) * (1.0 - 0.5 * alpha);
    let c2: f32 = 1.0 - alpha;

    // Smooth
    for (var i: u32 = 0u; i < min(3u, params.bar_count); i = i + 1u) { output[i] = 0.0; }
    for (var i: u32 = 3u; i < params.bar_count; i = i + 1u) {
        let sm_cur = (bars[i] + 2.0 * bars[i - 1u] + bars[i - 2u]) / 4.0;
        let sm_prev = (bars[i - 1u] + 2.0 * bars[i - 2u] + bars[i - 3u]) / 4.0;
        let sm_prev2 = (bars[i - 2u] + 2.0 * bars[i - 3u] + bars[max(i, 4u) - 4u]) / 4.0;
        output[i] = c1 * (sm_cur - 2.0 * sm_prev + sm_prev2) + 2.0 * c2 * output[i - 1u] - c2 * c2 * output[max(i, 2u) - 2u];
    }
}
"#;

const EHLERS_CG_SHADER: &str = r#"
// Ehlers Center of Gravity Oscillator — parallel per-bar
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.bar_count || i < params.period - 1u) { output[i] = 0.0; return; }
    var num: f32 = 0.0;
    var den: f32 = 0.0;
    for (var j: u32 = 0u; j < params.period; j = j + 1u) {
        let p = bars[i - j];
        num = num + f32(j + 1u) * p;
        den = den + p;
    }
    output[i] = select(-num / den + f32(params.period + 1u) / 2.0, 0.0, abs(den) < 0.000001);
}
"#;

const EHLERS_ROOF_SHADER: &str = r#"
// Ehlers Roofing Filter — sequential (highpass + super smoother)
// period field repurposed: low 16 bits = lp_period, high 16 bits = hp_period
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    let lp_period = params.period & 0xFFFFu;
    let hp_period = params.period >> 16u;
    let pi: f32 = 3.14159265;

    if (params.bar_count < 3u) { return; }

    // Highpass filter
    let alpha1 = cos(2.0 * pi / f32(max(hp_period, 2u)));
    let a1 = select(1.0 / max(alpha1 + sqrt(max(alpha1 * alpha1 - 1.0, 0.0)), 0.001), 0.5, abs(alpha1) < 0.000001);
    let hp_coeff = (1.0 - a1 / 2.0) * (1.0 - a1 / 2.0);
    let hp_c2 = 2.0 * (1.0 - a1);
    let hp_c3 = (1.0 - a1) * (1.0 - a1);

    // Super smoother coefficients
    let a = exp(-1.414 * pi / f32(max(lp_period, 1u)));
    let b = 2.0 * a * cos(1.414 * pi / f32(max(lp_period, 1u)));
    let ss_c1 = 1.0 - b + a * a;

    // Two-pass: highpass then super smooth
    output[0] = 0.0; output[1] = 0.0;
    var hp_prev1: f32 = 0.0;
    var hp_prev2: f32 = 0.0;
    var filt_prev1: f32 = 0.0;
    var filt_prev2: f32 = 0.0;

    for (var i: u32 = 2u; i < params.bar_count; i = i + 1u) {
        let hp = hp_coeff * (bars[i] - 2.0 * bars[i - 1u] + bars[i - 2u]) + hp_c2 * hp_prev1 - hp_c3 * hp_prev2;
        let filt = ss_c1 * (hp + hp_prev1) / 2.0 + b * filt_prev1 - a * a * filt_prev2;
        output[i] = filt;
        hp_prev2 = hp_prev1; hp_prev1 = hp;
        filt_prev2 = filt_prev1; filt_prev1 = filt;
    }
}
"#;

const EHLERS_EBSW_SHADER: &str = r#"
// Ehlers Even Better Sinewave — sequential (highpass + super smooth + atan)
struct Params { period: u32, bar_count: u32, }  // period = duration (e.g., 40)
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    let pi: f32 = 3.14159265;
    let duration = f32(max(params.period, 4u));
    if (params.bar_count < 5u) { return; }

    // Highpass coefficients
    let alpha1 = cos(2.0 * pi / (duration * 1.414));
    let a1 = select(1.0 / max(alpha1 + sqrt(max(alpha1 * alpha1 - 1.0, 0.0)), 0.001), 0.5, abs(alpha1) < 0.000001);
    let hp_coeff = (1.0 - a1 / 2.0) * (1.0 - a1 / 2.0);

    // Super smoother coefficients (period/4)
    let ss_period = max(duration / 4.0, 1.0);
    let a = exp(-1.414 * pi / ss_period);
    let b = 2.0 * a * cos(1.414 * pi / ss_period);
    let c1 = 1.0 - b + a * a;

    var hp_prev1: f32 = 0.0; var hp_prev2: f32 = 0.0;
    var filt_prev1: f32 = 0.0; var filt_prev2: f32 = 0.0;
    output[0] = 0.0; output[1] = 0.0;

    for (var i: u32 = 2u; i < params.bar_count; i = i + 1u) {
        // Highpass
        let hp = hp_coeff * (bars[i] - 2.0 * bars[i - 1u] + bars[i - 2u])
            + 2.0 * (1.0 - a1) * hp_prev1 - (1.0 - a1) * (1.0 - a1) * hp_prev2;
        // Super smooth
        let filt = c1 * (hp + hp_prev1) / 2.0 + b * filt_prev1 - a * a * filt_prev2;
        // Sinewave = atan(filt / filt_prev) normalized
        var wave: f32 = 0.0;
        if (abs(filt_prev1) > 0.000001) {
            wave = clamp(atan2(filt, filt_prev1) / (pi / 2.0), -1.0, 1.0);
        }
        output[i] = wave;
        hp_prev2 = hp_prev1; hp_prev1 = hp;
        filt_prev2 = filt_prev1; filt_prev1 = filt;
    }
}
"#;

const EHLERS_MAMA_SHADER: &str = r#"
// Ehlers MAMA/FAMA — sequential (adaptive moving average pair)
// Output: [mama, fama] per bar = 2 floats per bar
struct Params { period: u32, bar_count: u32, }  // period unused
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;  // [mama, fama] interleaved
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    let fast_limit: f32 = 0.5;
    let slow_limit: f32 = 0.05;
    let pi: f32 = 3.14159265;
    if (params.bar_count < 7u) { return; }

    // Smoothed price
    var sm_arr: array<f32, 7>;
    for (var i: u32 = 0u; i < 7u; i = i + 1u) { sm_arr[i] = bars[i]; }

    var mama_v: f32 = bars[0];
    var fama_v: f32 = bars[0];
    var prev_phase: f32 = 0.0;
    var prev_i1: f32 = 0.0;

    for (var i: u32 = 0u; i < params.bar_count; i = i + 1u) {
        if (i < 6u) {
            output[i * 2u] = bars[i];
            output[i * 2u + 1u] = bars[i];
            continue;
        }

        // 4-bar WMA smooth
        let s = (4.0 * bars[i] + 3.0 * bars[i - 1u] + 2.0 * bars[i - 2u] + bars[i - 3u]) / 10.0;
        let s2 = (4.0 * bars[i - 2u] + 3.0 * bars[i - 3u] + 2.0 * bars[i - 4u] + bars[i - 5u]) / 10.0;
        let s4 = (4.0 * bars[i - 4u] + 3.0 * bars[i - 5u] + 2.0 * bars[max(i, 6u) - 6u] + bars[max(i, 7u) - min(7u, i)]) / 10.0;

        // Hilbert discriminator
        let det = 0.0962 * s + 0.5769 * s2 - 0.5769 * s4 - 0.0962 * (4.0 * bars[max(i, 6u) - 6u] + 3.0 * bars[max(i, 7u) - min(7u, i)] + 2.0 * bars[max(i, 8u) - min(8u, i)] + bars[max(i, 9u) - min(9u, i)]) / 10.0;
        let i1 = bars[i - 3u];

        // Phase
        var phase: f32 = 0.0;
        if (abs(i1) > 0.000001) { phase = atan2(det, i1) * 180.0 / pi; }
        let delta_phase = max(prev_phase - phase, 1.0);
        let alpha = max(fast_limit / delta_phase, slow_limit);

        mama_v = alpha * s + (1.0 - alpha) * mama_v;
        fama_v = 0.5 * alpha * mama_v + (1.0 - 0.5 * alpha) * fama_v;

        output[i * 2u] = mama_v;
        output[i * 2u + 1u] = fama_v;
        prev_phase = phase;
        prev_i1 = i1;
    }
}
"#;

const SUPPLY_DEMAND_SHADER: &str = r#"
// Supply/Demand Zone Detection — Phase 1: GPU fractal detection (parallel per-bar)
// Port of SupplyDemand.mqh IsFractalHigh/IsFractalLow with 5-bar lookback.
// Output: [zone_type, zone_high, zone_low] per bar (3 floats)
//   zone_type: 0=none, 1=demand (fractal low), -1=supply (fractal high)
//   zone_high/zone_low: body-to-wick boundaries matching MQL5
// CPU then does zone testing, merging, and break detection.
struct Params { period: u32, bar_count: u32, }  // period = fractal lookback (5)
@group(0) @binding(0) var<storage, read> bars: array<f32>;  // [h,l,c] interleaved
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.bar_count) { return; }
    let base = i * 3u;
    let lookback = params.period;  // 5

    // Need lookback bars on each side
    if (i < lookback || i + lookback >= params.bar_count) {
        output[base] = 0.0; output[base + 1u] = 0.0; output[base + 2u] = 0.0;
        return;
    }

    let h = bars[i * 3u];
    let l = bars[i * 3u + 1u];
    let c = bars[i * 3u + 2u];

    // Fractal high: bar's high is strictly greater than lookback bars on each side
    var is_fractal_high = true;
    for (var k: u32 = 1u; k <= lookback; k = k + 1u) {
        if (bars[(i - k) * 3u] >= h || bars[(i + k) * 3u] >= h) {
            is_fractal_high = false;
            break;
        }
    }

    // Fractal low: bar's low is strictly less than lookback bars on each side
    var is_fractal_low = true;
    for (var k: u32 = 1u; k <= lookback; k = k + 1u) {
        if (bars[(i - k) * 3u + 1u] <= l || bars[(i + k) * 3u + 1u] <= l) {
            is_fractal_low = false;
            break;
        }
    }

    if (is_fractal_high) {
        // Supply zone: hi = high, lo = min(close, open) ≈ min(close, prev_close) approximation
        // Note: OHLC buffer lacks open; use close as approximation (CPU refines with actual open)
        output[base] = -1.0;
        output[base + 1u] = h;
        output[base + 2u] = c;  // placeholder — CPU replaces with min(close, open)
    } else if (is_fractal_low) {
        // Demand zone: hi = max(close, open) ≈ close, lo = low
        output[base] = 1.0;
        output[base + 1u] = c;  // placeholder — CPU replaces with max(close, open)
        output[base + 2u] = l;
    } else {
        output[base] = 0.0; output[base + 1u] = 0.0; output[base + 2u] = 0.0;
    }
}
"#;

const ATR_PROJECTION_SHADER: &str = r#"
// ATR Projection — parallel per-bar: open ± ATR using resident open + aux ATR buffers.
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> open_bars: array<f32>;
@group(0) @binding(4) var<storage, read> atr_values: array<f32>;
@group(0) @binding(5) var<storage, read_write> output: array<f32>;
@group(0) @binding(6) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.bar_count) { return; }
    let open_val = open_bars[i];
    let atr_val = atr_values[i];
    if (atr_val > 0.0) {
        output[i * 2u] = open_val + atr_val;
        output[i * 2u + 1u] = open_val - atr_val;
    } else {
        output[i * 2u] = 0.0;
        output[i * 2u + 1u] = 0.0;
    }
}
"#;

const BETTER_VOLUME_SHADER: &str = r#"
// BetterVolume — Full Emini-Watch algorithm (1:1 parity with CPU/MQL5)
// Output: classification f32: 0=low_vol, 1=climax_up, 2=climax_dn, 3=churn, 4=climax_churn, 5=normal
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> open_bars: array<f32>;
@group(0) @binding(1) var<storage, read> ohlc_bars: array<f32>;
@group(0) @binding(3) var<storage, read> volumes: array<f32>;
@group(0) @binding(5) var<storage, read_write> output: array<f32>;
@group(0) @binding(6) var<uniform> params: Params;

// Estimate buy/sell volume from candle structure (matching MQL5 EstimateBuySell)
fn estimate_buy(o: f32, h: f32, l: f32, c: f32, vol: f32) -> f32 {
    let range = h - l;
    if (range <= 0.0) { return vol * 0.5; }
    if (c > o) {
        let denom = 2.0 * range + o - c;
        let d = select(denom, range, denom <= 0.0);
        return (range / d) * vol;
    } else if (c < o) {
        let denom = 2.0 * range + c - o;
        let d = select(denom, range, denom <= 0.0);
        return ((range + c - o) / d) * vol;
    }
    return vol * 0.5;
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    let lb = params.period; // lookback (20)
    if (i >= params.bar_count) { output[i] = 5.0; return; }
    if (i < lb) { output[i] = 5.0; return; } // not enough history

    let min_range: f32 = 0.0000000001;

    // Current bar OHLCV
    let base = i * 3u;
    let o = open_bars[i];
    let h = ohlc_bars[base];
    let l = ohlc_bars[base + 1u];
    let c = ohlc_bars[base + 2u];
    let vol = volumes[i];
    let range = max(h - l, min_range);

    let buy_vol = estimate_buy(o, h, l, c, vol);
    let sell_vol = vol - buy_vol;

    let buy_range = buy_vol * range;
    let sell_range = sell_vol * range;
    let vol_div_r = vol / range;
    let sell_div_r = sell_vol / range;
    let buy_div_r = buy_vol / range;

    // Lookback extremes (previous lb bars)
    var high_buy_range: f32 = 0.0;
    var high_sell_range: f32 = 0.0;
    var high_vol_div_r: f32 = 0.0;
    var low_sell_div_r: f32 = 999999999.0;
    var low_buy_div_r: f32 = 999999999.0;
    var low_total_vol: f32 = 999999999.0;

    for (var j: u32 = 1u; j <= lb; j = j + 1u) {
        let bi = i - j;
        let bbase = bi * 3u;
        let bo = open_bars[bi];
        let bh = ohlc_bars[bbase];
        let bl = ohlc_bars[bbase + 1u];
        let bc = ohlc_bars[bbase + 2u];
        let bv = volumes[bi];
        let br = max(bh - bl, min_range);

        let bbuy = estimate_buy(bo, bh, bl, bc, bv);
        let bsell = bv - bbuy;

        let bbr = bbuy * br;
        let bsr = bsell * br;
        let bvr = bv / br;
        let bsdr = bsell / br;
        let bbdr = bbuy / br;

        high_buy_range = max(high_buy_range, bbr);
        high_sell_range = max(high_sell_range, bsr);
        high_vol_div_r = max(high_vol_div_r, bvr);
        low_sell_div_r = min(low_sell_div_r, bsdr);
        low_buy_div_r = min(low_buy_div_r, bbdr);
        low_total_vol = min(low_total_vol, bv);
    }

    // 1-bar classification
    var is_climax_up: bool = false;
    var is_climax_dn: bool = false;
    var is_churn: bool = false;
    var is_low_vol: bool = false;

    if (vol <= low_total_vol) { is_low_vol = true; }
    if (c > o && (buy_range >= high_buy_range || sell_div_r <= low_sell_div_r)) { is_climax_up = true; }
    if (c < o && (sell_range >= high_sell_range || buy_div_r <= low_buy_div_r)) { is_climax_dn = true; }
    if (vol_div_r >= high_vol_div_r) { is_churn = true; }

    // 2-bar analysis (matching MQL5 InpUse2Bars=true)
    if (i >= lb + 1u) {
        let pi = i - 1u;
        let pbase = pi * 3u;
        let po = open_bars[pi];
        let ph = ohlc_bars[pbase];
        let pl = ohlc_bars[pbase + 1u];
        let pc = ohlc_bars[pbase + 2u];
        let pv = volumes[pi];

        let pbuy = estimate_buy(po, ph, pl, pc, pv);
        let psell = pv - pbuy;
        let total_buy = buy_vol + pbuy;
        let total_sell = sell_vol + psell;
        let total_vol2 = vol + pv;
        let range2 = max(max(h, ph) - min(l, pl), min_range);

        let buy_range2 = total_buy * range2;
        let sell_range2 = total_sell * range2;
        let vol_div_r2 = total_vol2 / range2;
        let sell_div_r2 = total_sell / range2;
        let buy_div_r2 = total_buy / range2;

        // 2-bar lookback extremes
        var h_br2: f32 = 0.0;
        var h_sr2: f32 = 0.0;
        var h_vr2: f32 = 0.0;
        var l_sdr2: f32 = 999999999.0;
        var l_bdr2: f32 = 999999999.0;
        var l_vol2: f32 = 999999999.0;

        for (var j: u32 = 1u; j <= lb; j = j + 1u) {
            let b1i = i - j;
            if (b1i == 0u) { break; }
            let b2i = b1i - 1u;

            let base1 = b1i * 3u;
            let base2 = b2i * 3u;
            let o1 = open_bars[b1i]; let h1 = ohlc_bars[base1]; let l1 = ohlc_bars[base1 + 1u];
            let c1 = ohlc_bars[base1 + 2u]; let v1 = volumes[b1i];
            let o2 = open_bars[b2i]; let h2 = ohlc_bars[base2]; let l2 = ohlc_bars[base2 + 1u];
            let c2 = ohlc_bars[base2 + 2u]; let v2 = volumes[b2i];

            let tb = estimate_buy(o1, h1, l1, c1, v1) + estimate_buy(o2, h2, l2, c2, v2);
            let ts = (v1 - estimate_buy(o1, h1, l1, c1, v1)) + (v2 - estimate_buy(o2, h2, l2, c2, v2));
            let tv = v1 + v2;
            let r2 = max(max(h1, h2) - min(l1, l2), min_range);

            h_br2 = max(h_br2, tb * r2);
            h_sr2 = max(h_sr2, ts * r2);
            h_vr2 = max(h_vr2, tv / r2);
            l_sdr2 = min(l_sdr2, ts / r2);
            l_bdr2 = min(l_bdr2, tb / r2);
            l_vol2 = min(l_vol2, tv);
        }

        if (total_vol2 <= l_vol2) { is_low_vol = true; }
        if (c > o && (buy_range2 >= h_br2 || sell_div_r2 <= l_sdr2)) { is_climax_up = true; }
        if (c < o && (sell_range2 >= h_sr2 || buy_div_r2 <= l_bdr2)) { is_climax_dn = true; }
        if (vol_div_r2 >= h_vr2) { is_churn = true; }
    }

    // Priority: ClimaxChurn > LowVol > ClimaxUp > ClimaxDown > Churn > Normal
    if ((is_climax_up || is_climax_dn) && is_churn) { output[i] = 4.0; }  // climax+churn (magenta)
    else if (is_low_vol) { output[i] = 0.0; }     // low volume (yellow)
    else if (is_climax_up) { output[i] = 1.0; }   // climax up (red)
    else if (is_climax_dn) { output[i] = 2.0; }   // climax down (white)
    else if (is_churn) { output[i] = 3.0; }       // churn (green)
    else { output[i] = 5.0; }                     // normal (steelblue)
}
"#;

const ANCHORED_VWAP_SHADER: &str = r#"
// Anchored VWAP — sequential from anchor bar to end
// Cumulative (price × volume) / cumulative volume from anchor point
struct Params { period: u32, bar_count: u32, }  // period = anchor bar index
@group(0) @binding(2) var<storage, read> close_bars: array<f32>;
@group(0) @binding(3) var<storage, read> volumes: array<f32>;
@group(0) @binding(5) var<storage, read_write> output: array<f32>;
@group(0) @binding(6) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    let anchor = params.period;
    var cum_pv: f32 = 0.0;
    var cum_vol: f32 = 0.0;

    for (var i: u32 = 0u; i < params.bar_count; i = i + 1u) {
        if (i < anchor) { output[i] = 0.0; continue; }
        let close = close_bars[i];
        let vol = volumes[i];
        cum_pv = cum_pv + close * vol;
        cum_vol = cum_vol + vol;
        output[i] = select(cum_pv / cum_vol, close, cum_vol < 0.000001);
    }
}
"#;

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
const BACKTEST_EVAL_SHADER: &str = r#"
struct Params {
    bar_count: u32,
    combo_count: u32,
}

struct Combo {
    sma_fast: u32,
    sma_slow: u32,
    rsi_period: u32,
    rsi_overbought: f32,
    rsi_oversold: f32,
    atr_period: u32,
    atr_sl_mult: f32,
    atr_tp_mult: f32,
}

@group(0) @binding(0) var<storage, read> closes: array<f32>;
@group(0) @binding(1) var<storage, read> ohlc: array<f32>;  // [h,l,c] × bar_count
@group(0) @binding(2) var<storage, read> combos: array<f32>;  // 8 floats per combo
@group(0) @binding(3) var<storage, read_write> results: array<f32>;  // 9 floats per combo
@group(0) @binding(4) var<uniform> params: Params;

// Compute SMA at bar index for given period
fn sma_at(idx: u32, period: u32) -> f32 {
    if (idx < period - 1u) { return 0.0; }
    var sum: f32 = 0.0;
    for (var j: u32 = 0u; j < period; j = j + 1u) {
        sum = sum + closes[idx - j];
    }
    return sum / f32(period);
}

// Compute ATR at bar index
fn atr_at(idx: u32, period: u32) -> f32 {
    if (idx < period + 1u) { return 0.0; }
    var atr: f32 = 0.0;
    // Simple average of TR over last `period` bars
    for (var j: u32 = 0u; j < period; j = j + 1u) {
        let i = idx - j;
        let h = ohlc[i * 3u];
        let l = ohlc[i * 3u + 1u];
        let prev_c = ohlc[(i - 1u) * 3u + 2u];
        let tr = max(h - l, max(abs(h - prev_c), abs(l - prev_c)));
        atr = atr + tr;
    }
    return atr / f32(period);
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let combo_idx = id.x;
    if (combo_idx >= params.combo_count) { return; }

    // Unpack parameters
    let base = combo_idx * 8u;
    let sma_fast = bitcast<u32>(combos[base]);
    let sma_slow = bitcast<u32>(combos[base + 1u]);
    let rsi_period = bitcast<u32>(combos[base + 2u]);
    let rsi_ob = combos[base + 3u];
    let rsi_os = combos[base + 4u];
    let atr_period = bitcast<u32>(combos[base + 5u]);
    let atr_sl_mult = combos[base + 6u];
    let atr_tp_mult = combos[base + 7u];

    let lookback = max(sma_slow, max(rsi_period + 1u, atr_period + 1u));

    // State
    var equity: f32 = 100000.0;
    var peak: f32 = equity;
    var max_dd: f32 = 0.0;
    var in_trade: bool = false;
    var trade_dir: i32 = 0;  // 1=long, -1=short
    var entry_price: f32 = 0.0;
    var stop_loss: f32 = 0.0;
    var take_profit: f32 = 0.0;
    var wins: u32 = 0u;
    var losses: u32 = 0u;
    var total_profit: f32 = 0.0;
    var total_loss: f32 = 0.0;
    var total_hold: u32 = 0u;
    var trade_start: u32 = 0u;
    var daily_pnl_sum: f32 = 0.0;
    var daily_pnl_sq: f32 = 0.0;
    var daily_pnl_down: f32 = 0.0;
    var prev_equity: f32 = equity;

    // RSI state (running)
    var avg_gain: f32 = 0.0;
    var avg_loss: f32 = 0.0;
    var rsi_ready: bool = false;
    var rsi_val: f32 = 50.0;

    // Seed RSI
    if (rsi_period > 0u && lookback < params.bar_count) {
        for (var i: u32 = 1u; i <= rsi_period; i = i + 1u) {
            let chg = closes[i] - closes[i - 1u];
            if (chg > 0.0) { avg_gain = avg_gain + chg; }
            else { avg_loss = avg_loss - chg; }
        }
        avg_gain = avg_gain / f32(rsi_period);
        avg_loss = avg_loss / f32(rsi_period);
        rsi_ready = true;
    }

    // Walk bars
    for (var i: u32 = lookback; i < params.bar_count; i = i + 1u) {
        let close = closes[i];
        let prev_close = closes[i - 1u];
        let high = ohlc[i * 3u];
        let low = ohlc[i * 3u + 1u];

        // Update RSI
        if (rsi_ready && i > rsi_period) {
            let chg = close - prev_close;
            let gain = max(chg, 0.0);
            let loss = max(-chg, 0.0);
            avg_gain = (avg_gain * f32(rsi_period - 1u) + gain) / f32(rsi_period);
            avg_loss = (avg_loss * f32(rsi_period - 1u) + loss) / f32(rsi_period);
            let rs = select(avg_gain / avg_loss, 100.0, avg_loss < 0.000001);
            rsi_val = 100.0 - 100.0 / (1.0 + rs);
        }

        // SMA values
        let fast_sma = sma_at(i, sma_fast);
        let slow_sma = sma_at(i, sma_slow);
        let prev_fast = sma_at(i - 1u, sma_fast);
        let prev_slow = sma_at(i - 1u, sma_slow);
        let atr = atr_at(i, atr_period);

        // Check SL/TP if in trade
        if (in_trade) {
            var pnl: f32 = 0.0;
            var closed: bool = false;

            if (trade_dir == 1) {
                // Long: check stop loss (low touches SL) or take profit (high touches TP)
                if (low <= stop_loss) { pnl = stop_loss - entry_price; closed = true; }
                else if (take_profit > 0.0 && high >= take_profit) { pnl = take_profit - entry_price; closed = true; }
            } else {
                // Short
                if (high >= stop_loss) { pnl = entry_price - stop_loss; closed = true; }
                else if (take_profit > 0.0 && low <= take_profit) { pnl = entry_price - take_profit; closed = true; }
            }

            if (closed) {
                equity = equity + pnl;
                if (pnl > 0.0) { wins = wins + 1u; total_profit = total_profit + pnl; }
                else { losses = losses + 1u; total_loss = total_loss - pnl; }
                total_hold = total_hold + (i - trade_start);
                in_trade = false;
            }
        }

        // Entry signals (SMA crossover + RSI filter)
        if (!in_trade && fast_sma > 0.0 && slow_sma > 0.0 && atr > 0.0) {
            // Long: fast crosses above slow, RSI not overbought
            if (prev_fast <= prev_slow && fast_sma > slow_sma && rsi_val < rsi_ob) {
                in_trade = true;
                trade_dir = 1;
                entry_price = close;
                stop_loss = close - atr * atr_sl_mult;
                take_profit = close + atr * atr_tp_mult;
                trade_start = i;
            }
            // Short: fast crosses below slow, RSI not oversold
            else if (prev_fast >= prev_slow && fast_sma < slow_sma && rsi_val > rsi_os) {
                in_trade = true;
                trade_dir = -1;
                entry_price = close;
                stop_loss = close + atr * atr_sl_mult;
                take_profit = close - atr * atr_tp_mult;
                trade_start = i;
            }
        }

        // Track drawdown and daily PnL
        if (equity > peak) { peak = equity; }
        if (peak > 0.0) {
            let dd = (peak - equity) / peak;
            if (dd > max_dd) { max_dd = dd; }
        }
        let daily_ret = (equity - prev_equity) / max(prev_equity, 0.01);
        daily_pnl_sum = daily_pnl_sum + daily_ret;
        daily_pnl_sq = daily_pnl_sq + daily_ret * daily_ret;
        if (daily_ret < 0.0) { daily_pnl_down = daily_pnl_down + daily_ret * daily_ret; }
        prev_equity = equity;
    }

    // Close any open trade at last bar
    if (in_trade) {
        let last_close = closes[params.bar_count - 1u];
        var pnl: f32 = 0.0;
        if (trade_dir == 1) { pnl = last_close - entry_price; }
        else { pnl = entry_price - last_close; }
        equity = equity + pnl;
        if (pnl > 0.0) { wins = wins + 1u; } else { losses = losses + 1u; }
    }

    // Compute metrics
    let trades = wins + losses;
    let net_pnl = equity - 100000.0;
    let n_f = f32(params.bar_count - lookback);
    let mean_ret = daily_pnl_sum / max(n_f, 1.0);
    let variance = daily_pnl_sq / max(n_f, 1.0) - mean_ret * mean_ret;
    let std_dev = sqrt(max(variance, 0.0));
    let down_dev = sqrt(daily_pnl_down / max(n_f, 1.0));
    let ann_mean = mean_ret * 252.0;
    let ann_vol = std_dev * 15.8745;
    let ann_down = down_dev * 15.8745;
    let sharpe = select(ann_mean / ann_vol, 0.0, ann_vol < 0.000001);
    let sortino = select(ann_mean / ann_down, 0.0, ann_down < 0.000001);
    let win_rate = select(f32(wins) / f32(trades), 0.0, trades == 0u);
    let pf = select(total_profit / max(total_loss, 0.01), 0.0, trades == 0u);
    let avg_hold = select(f32(total_hold) / f32(trades), 0.0, trades == 0u);

    // Write results: [net_pnl, max_dd, sharpe, sortino, win_rate, pf, trades, avg_hold, 0(robustness)]
    let out = combo_idx * 9u;
    results[out] = net_pnl;
    results[out + 1u] = max_dd;
    results[out + 2u] = sharpe;
    results[out + 3u] = sortino;
    results[out + 4u] = win_rate;
    results[out + 5u] = pf;
    results[out + 6u] = f32(trades);
    results[out + 7u] = avg_hold;
    results[out + 8u] = 0.0;  // robustness filled by second pass
}
"#;

/// Robustness scoring shader — checks neighbor stability.
/// For each combo, compares its Sharpe to its neighbors (±1 on each param).
/// Score = 1.0 - normalized variance among neighbors.
const ROBUSTNESS_SHADER: &str = r#"
struct Params {
    bar_count: u32,
    combo_count: u32,
}

@group(0) @binding(0) var<storage, read> closes: array<f32>;       // unused but needed for layout
@group(0) @binding(1) var<storage, read> ohlc: array<f32>;         // unused
@group(0) @binding(2) var<storage, read> combos: array<f32>;       // param combos
@group(0) @binding(3) var<storage, read_write> results: array<f32>; // update robustness field
@group(0) @binding(4) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    if (idx >= params.combo_count) { return; }

    let out = idx * 9u;
    let my_sharpe = results[out + 2u];

    // Compare with neighbors (simple: ±1 index as proxy for ±1 on each param)
    var sum: f32 = my_sharpe;
    var sum_sq: f32 = my_sharpe * my_sharpe;
    var count: f32 = 1.0;

    if (idx > 0u) {
        let neighbor_sharpe = results[(idx - 1u) * 9u + 2u];
        sum = sum + neighbor_sharpe;
        sum_sq = sum_sq + neighbor_sharpe * neighbor_sharpe;
        count = count + 1.0;
    }
    if (idx + 1u < params.combo_count) {
        let neighbor_sharpe = results[(idx + 1u) * 9u + 2u];
        sum = sum + neighbor_sharpe;
        sum_sq = sum_sq + neighbor_sharpe * neighbor_sharpe;
        count = count + 1.0;
    }

    let mean = sum / count;
    let variance = sum_sq / count - mean * mean;
    // Robustness: low variance among neighbors = high score
    let robustness = select(1.0 / (1.0 + sqrt(max(variance, 0.0)) * 10.0), 0.0, my_sharpe < 0.0);
    results[out + 8u] = robustness;
}
"#;

/// GPU Monte Carlo VaR shader — parallel random walk simulations.
/// Each thread runs one simulation: samples from historical returns,
/// projects equity forward, records final value.
const MONTE_CARLO_SHADER: &str = r#"
struct Params {
    bar_count: u32,
    combo_count: u32,  // repurposed: simulation_count
}

@group(0) @binding(0) var<storage, read> closes: array<f32>;       // repurposed: daily returns
@group(0) @binding(1) var<storage, read> ohlc: array<f32>;         // unused
@group(0) @binding(2) var<storage, read> combos: array<f32>;       // repurposed: [days_forward, starting_equity, 0...]
@group(0) @binding(3) var<storage, read_write> results: array<f32>; // final equity per simulation
@group(0) @binding(4) var<uniform> params: Params;

// PCG hash for GPU-side pseudo-random number generation
fn pcg_hash(input: u32) -> u32 {
    var state = input * 747796405u + 2891336453u;
    var word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

fn rand_f32(seed: ptr<function, u32>) -> f32 {
    *seed = pcg_hash(*seed);
    return f32(*seed) / 4294967295.0;
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let sim_idx = id.x;
    if (sim_idx >= params.combo_count) { return; }

    let n_returns = params.bar_count;
    let days_forward = bitcast<u32>(combos[0]);
    let starting_equity = combos[1];

    var seed: u32 = sim_idx * 1234567u + 42u;
    var equity: f32 = starting_equity;
    var peak: f32 = equity;
    var max_dd: f32 = 0.0;

    // Random walk: sample from historical returns
    for (var d: u32 = 0u; d < days_forward; d = d + 1u) {
        // Pick a random historical return
        let r_idx = u32(rand_f32(&seed) * f32(n_returns - 1u));
        let daily_ret = closes[min(r_idx, n_returns - 1u)];
        equity = equity * (1.0 + daily_ret);

        if (equity > peak) { peak = equity; }
        let dd = (peak - equity) / max(peak, 0.01);
        if (dd > max_dd) { max_dd = dd; }
    }

    // Output: [final_equity, max_drawdown, ...] per simulation
    // Pack into 9-float result slots (reusing BacktestResult layout)
    let out = sim_idx * 9u;
    results[out] = equity - starting_equity;  // net PnL
    results[out + 1u] = max_dd;               // max drawdown
    results[out + 2u] = (equity - starting_equity) / starting_equity * 100.0;  // return %
    results[out + 3u] = equity;               // final equity
    // Remaining slots zeroed
    results[out + 4u] = 0.0;
    results[out + 5u] = 0.0;
    results[out + 6u] = 0.0;
    results[out + 7u] = 0.0;
    results[out + 8u] = 0.0;
}
"#;

/// NNFX Strategy Evaluation — Fisher crossover + KAMA trend + ATR stops + ADX filter.
/// One thread per parameter combination. Each thread computes Fisher, KAMA, ATR, ADX inline.
/// Params: [kama_period, fisher_period, atr_period, adx_period, adx_threshold, atr_sl_mult, atr_tp_mult, 0]
const NNFX_EVAL_SHADER: &str = r#"
struct Params {
    bar_count: u32,
    combo_count: u32,
}

@group(0) @binding(0) var<storage, read> closes: array<f32>;
@group(0) @binding(1) var<storage, read> ohlc: array<f32>;
@group(0) @binding(2) var<storage, read> combos: array<f32>;
@group(0) @binding(3) var<storage, read_write> results: array<f32>;
@group(0) @binding(4) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let combo_idx = id.x;
    if (combo_idx >= params.combo_count) { return; }

    let base = combo_idx * 8u;
    let kama_period = bitcast<u32>(combos[base]);
    let fisher_period = bitcast<u32>(combos[base + 1u]);
    let atr_period = bitcast<u32>(combos[base + 2u]);
    let adx_period = bitcast<u32>(combos[base + 3u]);
    let adx_threshold = combos[base + 4u];
    let atr_sl_mult = combos[base + 5u];
    let atr_tp_mult = combos[base + 6u];

    let lookback = max(max(kama_period, fisher_period), max(atr_period, adx_period)) + 2u;
    if (lookback >= params.bar_count) {
        let out = combo_idx * 9u;
        for (var k: u32 = 0u; k < 9u; k = k + 1u) { results[out + k] = 0.0; }
        return;
    }

    // State
    var equity: f32 = 100000.0;
    var peak: f32 = equity;
    var max_dd: f32 = 0.0;
    var in_trade: bool = false;
    var trade_dir: i32 = 0;
    var entry_price: f32 = 0.0;
    var stop_loss: f32 = 0.0;
    var take_profit: f32 = 0.0;
    var wins: u32 = 0u;
    var losses: u32 = 0u;
    var total_profit: f32 = 0.0;
    var total_loss: f32 = 0.0;
    var total_hold: u32 = 0u;
    var trade_start: u32 = 0u;
    var prev_equity: f32 = equity;
    var daily_pnl_sum: f32 = 0.0;
    var daily_pnl_sq: f32 = 0.0;
    var daily_pnl_down: f32 = 0.0;

    // KAMA state
    let fast_sc: f32 = 2.0 / 3.0;
    let slow_sc: f32 = 2.0 / 31.0;
    var kama: f32 = closes[0];
    var prev_kama: f32 = closes[0];

    // Fisher state
    var fish: f32 = 0.0;
    var prev_fish: f32 = 0.0;
    var fish_val: f32 = 0.0;

    // ATR state
    var atr_val: f32 = 0.0;
    var atr_sum: f32 = 0.0;
    var atr_ready: bool = false;

    // ADX state
    var smooth_plus_dm: f32 = 0.0;
    var smooth_minus_dm: f32 = 0.0;
    var smooth_tr: f32 = 0.0;
    var smooth_dx: f32 = 0.0;
    var adx_val: f32 = 0.0;

    for (var i: u32 = 1u; i < params.bar_count; i = i + 1u) {
        let close = closes[i];
        let high = ohlc[i * 3u];
        let low = ohlc[i * 3u + 1u];
        let prev_close = closes[i - 1u];
        let prev_high = ohlc[(i - 1u) * 3u];
        let prev_low = ohlc[(i - 1u) * 3u + 1u];
        let mid = (high + low) / 2.0;

        // Update KAMA
        if (i >= kama_period) {
            let direction = abs(close - closes[i - kama_period]);
            var volatility: f32 = 0.0;
            for (var j: u32 = i - kama_period + 1u; j <= i; j = j + 1u) {
                volatility = volatility + abs(closes[j] - closes[j - 1u]);
            }
            let er = select(direction / volatility, 0.0, volatility < 0.000001);
            let sc = er * (fast_sc - slow_sc) + slow_sc;
            prev_kama = kama;
            kama = kama + sc * sc * (close - kama);
        }

        // Update Fisher Transform
        if (i >= fisher_period) {
            var highest: f32 = -1000000.0;
            var lowest: f32 = 1000000.0;
            for (var j: u32 = i - fisher_period + 1u; j <= i; j = j + 1u) {
                let m = (ohlc[j * 3u] + ohlc[j * 3u + 1u]) / 2.0;
                if (m > highest) { highest = m; }
                if (m < lowest) { lowest = m; }
            }
            let range = highest - lowest;
            var raw: f32 = 0.0;
            if (range > 0.000001) { raw = 2.0 * (mid - lowest) / range - 1.0; }
            raw = clamp(raw, -0.999, 0.999);
            fish_val = 0.33 * raw + 0.67 * fish_val;
            fish_val = clamp(fish_val, -0.999, 0.999);
            prev_fish = fish;
            fish = 0.5 * log((1.0 + fish_val) / (1.0 - fish_val));
        }

        // Update ATR
        let tr = max(high - low, max(abs(high - prev_close), abs(low - prev_close)));
        if (i <= atr_period) {
            atr_sum = atr_sum + tr;
            if (i == atr_period) { atr_val = atr_sum / f32(atr_period); atr_ready = true; }
        } else if (atr_ready) {
            atr_val = (atr_val * f32(atr_period - 1u) + tr) / f32(atr_period);
        }

        // Update ADX
        let up_move = high - prev_high;
        let down_move = prev_low - low;
        var plus_dm: f32 = 0.0;
        var minus_dm: f32 = 0.0;
        if (up_move > down_move && up_move > 0.0) { plus_dm = up_move; }
        if (down_move > up_move && down_move > 0.0) { minus_dm = down_move; }
        if (i <= adx_period) {
            smooth_plus_dm = smooth_plus_dm + plus_dm;
            smooth_minus_dm = smooth_minus_dm + minus_dm;
            smooth_tr = smooth_tr + tr;
        } else {
            let p = f32(adx_period);
            smooth_plus_dm = smooth_plus_dm - smooth_plus_dm / p + plus_dm;
            smooth_minus_dm = smooth_minus_dm - smooth_minus_dm / p + minus_dm;
            smooth_tr = smooth_tr - smooth_tr / p + tr;
            let plus_di = select(100.0 * smooth_plus_dm / smooth_tr, 0.0, smooth_tr < 0.000001);
            let minus_di = select(100.0 * smooth_minus_dm / smooth_tr, 0.0, smooth_tr < 0.000001);
            let di_sum = plus_di + minus_di;
            let dx = select(100.0 * abs(plus_di - minus_di) / di_sum, 0.0, di_sum < 0.000001);
            smooth_dx = (smooth_dx * (f32(adx_period) - 1.0) + dx) / f32(adx_period);
            adx_val = smooth_dx;
        }

        if (i < lookback) { continue; }

        // Check SL/TP
        if (in_trade) {
            var pnl: f32 = 0.0;
            var closed: bool = false;
            if (trade_dir == 1) {
                if (low <= stop_loss) { pnl = stop_loss - entry_price; closed = true; }
                else if (take_profit > 0.0 && high >= take_profit) { pnl = take_profit - entry_price; closed = true; }
            } else {
                if (high >= stop_loss) { pnl = entry_price - stop_loss; closed = true; }
                else if (take_profit > 0.0 && low <= take_profit) { pnl = entry_price - take_profit; closed = true; }
            }
            if (closed) {
                equity = equity + pnl;
                if (pnl > 0.0) { wins = wins + 1u; total_profit = total_profit + pnl; }
                else { losses = losses + 1u; total_loss = total_loss - pnl; }
                total_hold = total_hold + (i - trade_start);
                in_trade = false;
            }
        }

        // NNFX Entry: Fisher crosses zero + KAMA confirms trend + ADX filter
        if (!in_trade && atr_ready && adx_val > adx_threshold) {
            // Long: Fisher crosses above 0, KAMA rising
            if (prev_fish <= 0.0 && fish > 0.0 && kama > prev_kama) {
                in_trade = true; trade_dir = 1;
                entry_price = close;
                stop_loss = close - atr_val * atr_sl_mult;
                take_profit = close + atr_val * atr_tp_mult;
                trade_start = i;
            }
            // Short: Fisher crosses below 0, KAMA falling
            else if (prev_fish >= 0.0 && fish < 0.0 && kama < prev_kama) {
                in_trade = true; trade_dir = -1;
                entry_price = close;
                stop_loss = close + atr_val * atr_sl_mult;
                take_profit = close - atr_val * atr_tp_mult;
                trade_start = i;
            }
        }

        // Track drawdown
        if (equity > peak) { peak = equity; }
        if (peak > 0.0) { let dd = (peak - equity) / peak; if (dd > max_dd) { max_dd = dd; } }
        let daily_ret = (equity - prev_equity) / max(prev_equity, 0.01);
        daily_pnl_sum = daily_pnl_sum + daily_ret;
        daily_pnl_sq = daily_pnl_sq + daily_ret * daily_ret;
        if (daily_ret < 0.0) { daily_pnl_down = daily_pnl_down + daily_ret * daily_ret; }
        prev_equity = equity;
    }

    // Close open trade
    if (in_trade) {
        let lc = closes[params.bar_count - 1u];
        if (trade_dir == 1) { equity = equity + lc - entry_price; }
        else { equity = equity + entry_price - lc; }
    }

    // Metrics
    let trades = wins + losses;
    let n_f = f32(params.bar_count - lookback);
    let mean_ret = daily_pnl_sum / max(n_f, 1.0);
    let variance = daily_pnl_sq / max(n_f, 1.0) - mean_ret * mean_ret;
    let std_dev = sqrt(max(variance, 0.0));
    let down_dev = sqrt(daily_pnl_down / max(n_f, 1.0));
    let ann_mean = mean_ret * 252.0;
    let ann_vol = std_dev * 15.8745;
    let ann_down = down_dev * 15.8745;

    let out = combo_idx * 9u;
    results[out] = equity - 100000.0;
    results[out + 1u] = max_dd;
    results[out + 2u] = select(ann_mean / ann_vol, 0.0, ann_vol < 0.000001);
    results[out + 3u] = select(ann_mean / ann_down, 0.0, ann_down < 0.000001);
    results[out + 4u] = select(f32(wins) / f32(trades), 0.0, trades == 0u);
    results[out + 5u] = select(total_profit / max(total_loss, 0.01), 0.0, trades == 0u);
    results[out + 6u] = f32(trades);
    results[out + 7u] = select(f32(total_hold) / f32(trades), 0.0, trades == 0u);
    results[out + 8u] = 0.0;
}
"#;

/// Walk-Forward Validation shader — evaluates strategy on out-of-sample window.
/// Same as NNFX eval but only processes bars[start..end] range.
/// Params uniform extended: [bar_count, combo_count, window_start, window_end]
const WALK_FORWARD_SHADER: &str = r#"
struct Params {
    bar_count: u32,
    combo_count: u32,
}

@group(0) @binding(0) var<storage, read> closes: array<f32>;
@group(0) @binding(1) var<storage, read> ohlc: array<f32>;
@group(0) @binding(2) var<storage, read> combos: array<f32>;
@group(0) @binding(3) var<storage, read_write> results: array<f32>;
@group(0) @binding(4) var<uniform> params: Params;

// Walk-forward uses same eval logic but the caller uploads a subset of bars
// for the out-of-sample window. This shader is identical to BACKTEST_EVAL_SHADER
// but exists as a separate pipeline for clarity. The host code handles windowing.
@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let combo_idx = id.x;
    if (combo_idx >= params.combo_count) { return; }

    // Identical to backtest eval — host controls which bars are uploaded
    let base = combo_idx * 8u;
    let sma_fast = bitcast<u32>(combos[base]);
    let sma_slow = bitcast<u32>(combos[base + 1u]);
    let lookback = max(sma_fast, sma_slow) + 2u;
    if (lookback >= params.bar_count) {
        let out = combo_idx * 9u;
        for (var k: u32 = 0u; k < 9u; k = k + 1u) { results[out + k] = 0.0; }
        return;
    }

    var equity: f32 = 100000.0;
    var peak: f32 = equity;
    var max_dd: f32 = 0.0;
    var wins: u32 = 0u;
    var losses: u32 = 0u;
    var in_trade: bool = false;
    var trade_dir: i32 = 0;
    var entry_price: f32 = 0.0;

    for (var i: u32 = lookback; i < params.bar_count; i = i + 1u) {
        // Simplified SMA cross for walk-forward (reuses same combo format)
        var fast_sum: f32 = 0.0;
        var slow_sum: f32 = 0.0;
        var prev_fast_sum: f32 = 0.0;
        var prev_slow_sum: f32 = 0.0;
        for (var j: u32 = 0u; j < sma_fast; j = j + 1u) { fast_sum += closes[i - j]; prev_fast_sum += closes[i - 1u - j]; }
        for (var j: u32 = 0u; j < sma_slow; j = j + 1u) { slow_sum += closes[i - j]; prev_slow_sum += closes[i - 1u - j]; }
        let fast_sma = fast_sum / f32(sma_fast);
        let slow_sma = slow_sum / f32(sma_slow);
        let prev_fast = prev_fast_sum / f32(sma_fast);
        let prev_slow = prev_slow_sum / f32(sma_slow);

        if (in_trade) {
            let pnl = select(closes[i] - entry_price, entry_price - closes[i], trade_dir == 1);
            // Simple exit: reverse signal
            if ((trade_dir == 1 && fast_sma < slow_sma) || (trade_dir == -1 && fast_sma > slow_sma)) {
                equity += pnl;
                if (pnl > 0.0) { wins += 1u; } else { losses += 1u; }
                in_trade = false;
            }
        }
        if (!in_trade) {
            if (prev_fast <= prev_slow && fast_sma > slow_sma) { in_trade = true; trade_dir = 1; entry_price = closes[i]; }
            else if (prev_fast >= prev_slow && fast_sma < slow_sma) { in_trade = true; trade_dir = -1; entry_price = closes[i]; }
        }
        if (equity > peak) { peak = equity; }
        let dd = (peak - equity) / max(peak, 0.01);
        if (dd > max_dd) { max_dd = dd; }
    }

    let trades = wins + losses;
    let out = combo_idx * 9u;
    results[out] = equity - 100000.0;
    results[out + 1u] = max_dd;
    results[out + 2u] = select(f32(wins) / f32(trades), 0.0, trades == 0u);
    results[out + 3u] = 0.0; results[out + 4u] = 0.0; results[out + 5u] = 0.0;
    results[out + 6u] = f32(trades);
    results[out + 7u] = 0.0; results[out + 8u] = 0.0;
}
"#;

// ── ADR-092: New GPU compute shaders ────────────────────────────────

/// Volume Profile — bins price×volume into N price levels.
/// Input: OHLCV interleaved [open, high, low, close, volume] × bar_count.
/// Output: histogram[num_levels] = cumulative volume at each price level.
const VOLUME_PROFILE_SHADER: &str = r#"
struct Params {
    bar_count: u32,
    num_levels: u32,
    price_min: f32,
    price_max: f32,
}
@group(0) @binding(0) var<storage, read> ohlcv: array<f32>;
@group(0) @binding(1) var<storage, read_write> histogram: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= params.bar_count) { return; }
    let base = i * 5u;
    let high = ohlcv[base + 1u];
    let low = ohlcv[base + 2u];
    let close = ohlcv[base + 3u];
    let volume = ohlcv[base + 4u];
    let price_range = params.price_max - params.price_min;
    if (price_range <= 0.0) { return; }
    // Distribute volume across price levels touched by this bar
    let level_size = price_range / f32(params.num_levels);
    let lo_level = u32(max((low - params.price_min) / level_size, 0.0));
    let hi_level = min(u32((high - params.price_min) / level_size), params.num_levels - 1u);
    let levels_touched = hi_level - lo_level + 1u;
    let vol_per_level = volume / f32(levels_touched);
    for (var l = lo_level; l <= hi_level; l++) {
        // Atomic-free: each thread writes to different regions (acceptable race for visualization)
        histogram[l] += vol_per_level;
    }
}
"#;

/// Batch Screener — computes RSI + SMA for 500+ symbols in one dispatch.
/// Each thread processes one symbol. Input: close prices for all symbols
/// packed sequentially with offsets. Output: [rsi, sma] per symbol.
const BATCH_SCREENER_SHADER: &str = r#"
struct Params {
    symbol_count: u32,
    bars_per_symbol: u32,
    rsi_period: u32,
    sma_period: u32,
}
@group(0) @binding(0) var<storage, read> closes: array<f32>;
@group(0) @binding(1) var<storage, read_write> results: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let sym = gid.x;
    if (sym >= params.symbol_count) { return; }
    let base = sym * params.bars_per_symbol;
    let n = params.bars_per_symbol;
    if (n < 2u) { results[sym * 2u] = 50.0; results[sym * 2u + 1u] = 0.0; return; }

    // RSI
    var avg_gain = 0.0;
    var avg_loss = 0.0;
    let rp = min(params.rsi_period, n - 1u);
    for (var i = 1u; i <= rp; i++) {
        let diff = closes[base + i] - closes[base + i - 1u];
        if (diff > 0.0) { avg_gain += diff; } else { avg_loss -= diff; }
    }
    avg_gain /= f32(rp);
    avg_loss /= f32(rp);
    for (var i = rp + 1u; i < n; i++) {
        let diff = closes[base + i] - closes[base + i - 1u];
        if (diff > 0.0) {
            avg_gain = (avg_gain * f32(rp - 1u) + diff) / f32(rp);
            avg_loss = (avg_loss * f32(rp - 1u)) / f32(rp);
        } else {
            avg_gain = (avg_gain * f32(rp - 1u)) / f32(rp);
            avg_loss = (avg_loss * f32(rp - 1u) - diff) / f32(rp);
        }
    }
    var rsi = 50.0;
    if (avg_loss > 0.0) {
        let rs = avg_gain / avg_loss;
        rsi = 100.0 - (100.0 / (1.0 + rs));
    } else if (avg_gain > 0.0) {
        rsi = 100.0;
    }
    results[sym * 2u] = rsi;

    // SMA (last sma_period bars)
    var sum = 0.0;
    let sp = min(params.sma_period, n);
    for (var i = n - sp; i < n; i++) {
        sum += closes[base + i];
    }
    results[sym * 2u + 1u] = sum / f32(sp);
}
"#;

/// Rolling Statistics — computes rolling Sharpe ratio for each window position.
/// Each thread computes one window. Input: returns array.
/// Output: rolling_sharpe[position].
const ROLLING_STATS_SHADER: &str = r#"
struct Params {
    total_days: u32,
    window_size: u32,
    _pad0: u32,
    _pad1: u32,
}
@group(0) @binding(0) var<storage, read> returns: array<f32>;
@group(0) @binding(1) var<storage, read_write> rolling_sharpe: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let pos = gid.x;
    let end = pos + params.window_size;
    if (end > params.total_days) { return; }

    // Mean
    var sum = 0.0;
    for (var i = pos; i < end; i++) {
        sum += returns[i];
    }
    let mean = sum / f32(params.window_size);

    // StdDev
    var var_sum = 0.0;
    for (var i = pos; i < end; i++) {
        let diff = returns[i] - mean;
        var_sum += diff * diff;
    }
    let std_dev = sqrt(var_sum / f32(params.window_size));

    // Sharpe (annualized, assumes daily returns)
    if (std_dev > 0.0001) {
        rolling_sharpe[pos] = (mean * 252.0) / (std_dev * sqrt(252.0));
    } else {
        rolling_sharpe[pos] = 0.0;
    }
}
"#;

/// Renko Builder — constructs Renko bricks from close price data.
/// Each brick has a fixed size. Output: [direction, open, close] per brick.
/// Sequential dispatch (brick dependencies).
const RENKO_BUILDER_SHADER: &str = r#"
struct Params {
    bar_count: u32,
    brick_size: f32,
    _pad0: u32,
    _pad1: u32,
}
@group(0) @binding(0) var<storage, read> closes: array<f32>;
@group(0) @binding(1) var<storage, read_write> bricks: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    if (params.bar_count < 1u) { return; }
    var brick_base = closes[0u];
    var brick_count = 0u;
    let max_bricks = params.bar_count * 2u; // upper bound

    for (var i = 1u; i < params.bar_count; i++) {
        let price = closes[i];
        // Up bricks
        while (price >= brick_base + params.brick_size && brick_count < max_bricks) {
            let out = brick_count * 3u;
            bricks[out] = 1.0;  // direction: up
            bricks[out + 1u] = brick_base;
            bricks[out + 2u] = brick_base + params.brick_size;
            brick_base += params.brick_size;
            brick_count++;
        }
        // Down bricks
        while (price <= brick_base - params.brick_size && brick_count < max_bricks) {
            let out = brick_count * 3u;
            bricks[out] = -1.0;  // direction: down
            bricks[out + 1u] = brick_base;
            bricks[out + 2u] = brick_base - params.brick_size;
            brick_base -= params.brick_size;
            brick_count++;
        }
    }
    // Store brick count in first output slot (slot 0 rewritten)
    // Consumers check bricks[i*3] for 1.0/-1.0 vs 0.0 to find end
}
"#;

/// Tick Aggregation — aggregates raw ticks into OHLCV bars at multiple timeframes.
/// Each thread processes one timeframe bucket. Input: tick prices + timestamps.
/// Output: OHLCV bars per timeframe.
const TICK_AGGREGATION_SHADER: &str = r#"
struct Params {
    tick_count: u32,
    tf_seconds: u32,
    _pad0: u32,
    _pad1: u32,
}
@group(0) @binding(0) var<storage, read> tick_prices: array<f32>;
@group(0) @binding(1) var<storage, read> tick_timestamps: array<u32>;
@group(0) @binding(2) var<storage, read_write> ohlcv_out: array<f32>;
@group(0) @binding(3) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let bar_idx = gid.x;
    if (params.tick_count == 0u) { return; }

    let bar_start_ts = tick_timestamps[0u] + bar_idx * params.tf_seconds;
    let bar_end_ts = bar_start_ts + params.tf_seconds;

    var o = 0.0;
    var h = -999999.0;
    var l = 999999.0;
    var c = 0.0;
    var v = 0.0;
    var found = false;

    for (var i = 0u; i < params.tick_count; i++) {
        let ts = tick_timestamps[i];
        if (ts >= bar_start_ts && ts < bar_end_ts) {
            let price = tick_prices[i];
            if (!found) { o = price; found = true; }
            if (price > h) { h = price; }
            if (price < l) { l = price; }
            c = price;
            v += 1.0;
        }
    }

    if (found) {
        let out = bar_idx * 5u;
        ohlcv_out[out] = o;
        ohlcv_out[out + 1u] = h;
        ohlcv_out[out + 2u] = l;
        ohlcv_out[out + 3u] = c;
        ohlcv_out[out + 4u] = v;
    }
}
"#;

/// Multi-Symbol Backtest — tests same strategy across N symbols × M param combos.
/// Extends BACKTEST_EVAL_SHADER to two-dimensional dispatch.
const MULTI_SYMBOL_BACKTEST_SHADER: &str = r#"
struct Params {
    bars_per_symbol: u32,
    symbol_count: u32,
    fast_start: u32,
    fast_step: u32,
    slow_start: u32,
    slow_step: u32,
    combos_per_symbol: u32,
    _pad: u32,
}
@group(0) @binding(0) var<storage, read> all_closes: array<f32>;
@group(0) @binding(1) var<storage, read_write> results: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let thread_idx = gid.x;
    let total_combos = params.symbol_count * params.combos_per_symbol;
    if (thread_idx >= total_combos) { return; }

    let sym_idx = thread_idx / params.combos_per_symbol;
    let combo_idx = thread_idx % params.combos_per_symbol;
    let base = sym_idx * params.bars_per_symbol;
    let n = params.bars_per_symbol;

    // Derive fast/slow periods from combo index (grid search)
    let fast_range = 10u; // 10 fast values per slow value
    let fast_idx = combo_idx % fast_range;
    let slow_idx = combo_idx / fast_range;
    let fast_period = params.fast_start + fast_idx * params.fast_step;
    let slow_period = params.slow_start + slow_idx * params.slow_step;

    if (fast_period >= slow_period || slow_period >= n) {
        let out = thread_idx * 4u;
        results[out] = 0.0; results[out + 1u] = 0.0;
        results[out + 2u] = 0.0; results[out + 3u] = 0.0;
        return;
    }

    // SMA crossover backtest
    var equity = 100000.0;
    var peak = equity;
    var max_dd = 0.0;
    var wins = 0u;
    var losses = 0u;
    var in_trade = false;
    var trade_dir = 0;
    var entry_price = 0.0;

    for (var i = slow_period; i < n; i++) {
        // Compute fast SMA
        var fast_sum = 0.0;
        for (var j = i - fast_period; j < i; j++) { fast_sum += all_closes[base + j]; }
        let fast_sma = fast_sum / f32(fast_period);
        // Compute slow SMA
        var slow_sum = 0.0;
        for (var j = i - slow_period; j < i; j++) { slow_sum += all_closes[base + j]; }
        let slow_sma = slow_sum / f32(slow_period);

        let price = all_closes[base + i];

        if (in_trade) {
            let pnl = f32(trade_dir) * (price - entry_price) * 100.0;
            if ((trade_dir == 1 && fast_sma < slow_sma) || (trade_dir == -1 && fast_sma > slow_sma)) {
                equity += pnl;
                if (pnl > 0.0) { wins++; } else { losses++; }
                in_trade = false;
            }
        }
        if (!in_trade) {
            // Compute previous bar SMAs for crossover detection
            if (i > slow_period) {
                var pf = 0.0;
                for (var j = i - 1u - fast_period; j < i - 1u; j++) { pf += all_closes[base + j]; }
                let prev_fast = pf / f32(fast_period);
                var ps = 0.0;
                for (var j = i - 1u - slow_period; j < i - 1u; j++) { ps += all_closes[base + j]; }
                let prev_slow = ps / f32(slow_period);
                if (prev_fast <= prev_slow && fast_sma > slow_sma) {
                    in_trade = true; trade_dir = 1; entry_price = price;
                } else if (prev_fast >= prev_slow && fast_sma < slow_sma) {
                    in_trade = true; trade_dir = -1; entry_price = price;
                }
            }
        }
        if (equity > peak) { peak = equity; }
        let dd = (peak - equity) / max(peak, 0.01);
        if (dd > max_dd) { max_dd = dd; }
    }

    let trades = wins + losses;
    let out = thread_idx * 4u;
    results[out] = equity - 100000.0;  // net P&L
    results[out + 1u] = max_dd;
    results[out + 2u] = select(f32(wins) / f32(trades), 0.0, trades == 0u);
    results[out + 3u] = f32(trades);
}
"#;

// ── ADR-092: GPU Render Shaders (Vertex + Fragment) ──────────────────

/// Instanced Candlestick Renderer — renders all visible candles in a single draw call.
/// Each instance is one candlestick. Instance data: [x, open_y, close_y, high_y, low_y, is_up].
/// Vertex shader expands each instance into body quad + wick line geometry.
#[allow(dead_code)]
const CANDLE_RENDER_SHADER: &str = r#"
// Per-instance data uploaded from CPU each frame
struct CandleInstance {
    @location(0) x_center: f32,
    @location(1) body_top: f32,
    @location(2) body_bot: f32,
    @location(3) wick_top: f32,
    @location(4) wick_bot: f32,
    @location(5) is_up: f32,           // 1.0 = green, 0.0 = red
    @location(6) half_width: f32,
    @location(7) is_live_forming: f32, // 1.0 = live quote mode (thin mid line, no body)
}

struct Uniforms {
    viewport_width: f32,
    viewport_height: f32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

// 10 vertices per instance: 4 for body quad (triangle strip), 2 for wick top, 2 for wick bot
// vertex_index 0-3: body quad, 4-5: top wick, 6-7: bottom wick, 8-9: unused
@vertex
fn vs_main(
    instance: CandleInstance,
    @builtin(vertex_index) vid: u32,
) -> VertexOutput {
    var out: VertexOutput;
    let green = vec4<f32>(0.0, 1.0, 0.0, 1.0);
    let red = vec4<f32>(1.0, 0.0, 0.0, 1.0);
    if (instance.is_live_forming > 0.5) {
        out.color = vec4<f32>(0.9, 0.9, 0.9, 1.0); // neutral/white for live forming bar
    } else {
        out.color = select(red, green, instance.is_up > 0.5);
    }

    // NDC coords: x in [-1, 1], y in [-1, 1]
    let ndc_x = (instance.x_center / uniforms.viewport_width) * 2.0 - 1.0;
    let hw = (instance.half_width / uniforms.viewport_width) * 2.0;
    let wick_w = 1.0 / uniforms.viewport_width; // 1px wick

    var pos = vec2<f32>(0.0, 0.0);

    if (instance.is_live_forming > 0.5) {
        // Live forming bar: draw NOTHING — let the Bid/Ask overlay be the only live indicator
        out.position = vec4<f32>(0.0, 0.0, 0.0, 0.0);
        return out;
    } else {
        switch (vid) {
            // Body quad (triangle strip: 0-1-2-3)
            case 0u: { pos = vec2<f32>(ndc_x - hw, 1.0 - instance.body_top / uniforms.viewport_height * 2.0); }
            case 1u: { pos = vec2<f32>(ndc_x + hw, 1.0 - instance.body_top / uniforms.viewport_height * 2.0); }
            case 2u: { pos = vec2<f32>(ndc_x - hw, 1.0 - instance.body_bot / uniforms.viewport_height * 2.0); }
            case 3u: { pos = vec2<f32>(ndc_x + hw, 1.0 - instance.body_bot / uniforms.viewport_height * 2.0); }
            // Top wick (line: 4-5)
            case 4u: { pos = vec2<f32>(ndc_x, 1.0 - instance.wick_top / uniforms.viewport_height * 2.0); }
            case 5u: { pos = vec2<f32>(ndc_x, 1.0 - instance.body_top / uniforms.viewport_height * 2.0); }
            // Bottom wick (line: 6-7)
            case 6u: { pos = vec2<f32>(ndc_x, 1.0 - instance.body_bot / uniforms.viewport_height * 2.0); }
            case 7u: { pos = vec2<f32>(ndc_x, 1.0 - instance.wick_bot / uniforms.viewport_height * 2.0); }
            default: { pos = vec2<f32>(0.0, 0.0); }
        }
    }

    out.position = vec4<f32>(pos, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

/// Indicator Polyline Renderer — renders indicator values as GPU line strip.
/// Input: vertex buffer of [x, y, r, g, b, a] per point.
#[allow(dead_code)]
const POLYLINE_RENDER_SHADER: &str = r#"
struct Uniforms {
    viewport_width: f32,
    viewport_height: f32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) pos: vec2<f32>,
    @location(1) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let ndc_x = (in.pos.x / uniforms.viewport_width) * 2.0 - 1.0;
    let ndc_y = 1.0 - (in.pos.y / uniforms.viewport_height) * 2.0;
    out.position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

/// Heatmap Texture Renderer — renders a compute-generated texture as fullscreen quad.
/// Used for correlation matrices, sector heatmaps, volume profiles.
#[allow(dead_code)]
const HEATMAP_RENDER_SHADER: &str = r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> VertexOutput {
    var out: VertexOutput;
    // Fullscreen triangle (3 vertices cover entire viewport)
    let x = f32(vid & 1u) * 4.0 - 1.0;
    let y = f32((vid >> 1u) & 1u) * 4.0 - 1.0;
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}

@group(0) @binding(0) var heatmap_texture: texture_2d<f32>;
@group(0) @binding(1) var heatmap_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(heatmap_texture, heatmap_sampler, in.uv);
}
"#;

/// Zone Compositor — renders session highlights, S/D zones, FVG as a texture overlay.
/// Alpha-blended composite of multiple zone layers.
#[allow(dead_code)]
const ZONE_COMPOSITE_SHADER: &str = r#"
struct ZoneInstance {
    @location(0) rect_min: vec2<f32>,
    @location(1) rect_max: vec2<f32>,
    @location(2) color: vec4<f32>,
}

struct Uniforms {
    viewport_width: f32,
    viewport_height: f32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(
    zone: ZoneInstance,
    @builtin(vertex_index) vid: u32,
) -> VertexOutput {
    var out: VertexOutput;
    out.color = zone.color;
    // Expand instance to quad (triangle strip: 4 vertices)
    var pos = vec2<f32>(0.0, 0.0);
    switch (vid) {
        case 0u: { pos = zone.rect_min; }
        case 1u: { pos = vec2<f32>(zone.rect_max.x, zone.rect_min.y); }
        case 2u: { pos = vec2<f32>(zone.rect_min.x, zone.rect_max.y); }
        case 3u: { pos = zone.rect_max; }
        default: {}
    }
    let ndc_x = (pos.x / uniforms.viewport_width) * 2.0 - 1.0;
    let ndc_y = 1.0 - (pos.y / uniforms.viewport_height) * 2.0;
    out.position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

// ─── ADR-094: GPU parity shaders ──────────────────────────────────────────────

const SUPERTREND_SHADER: &str = r#"
// Supertrend — sequential (ATR-based trailing stop with direction flip)
// Output: 2 per bar [supertrend_value, direction] where direction: 1.0=up, -1.0=down
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    let n = params.bar_count;
    let p = params.period;
    let mult: f32 = 3.0;
    if (n < p + 1u) { return; }

    // ATR seed
    var atr: f32 = 0.0;
    for (var i: u32 = 1u; i < p + 1u; i = i + 1u) {
        let h = bars[i * 3u]; let l = bars[i * 3u + 1u]; let pc = bars[(i - 1u) * 3u + 2u];
        atr += max(h - l, max(abs(h - pc), abs(l - pc)));
    }
    atr = atr / f32(p);

    var dir: f32 = 1.0;
    var upper_band: f32 = 0.0;
    var lower_band: f32 = 0.0;

    for (var i: u32 = 0u; i < n; i = i + 1u) {
        let h = bars[i * 3u]; let l = bars[i * 3u + 1u]; let c = bars[i * 3u + 2u];
        if (i >= p) {
            let pc = bars[(i - 1u) * 3u + 2u];
            let tr = max(h - l, max(abs(h - pc), abs(l - pc)));
            atr = (atr * f32(p - 1u) + tr) / f32(p);
        }
        let hl2 = (h + l) / 2.0;
        let raw_upper = hl2 + mult * atr;
        let raw_lower = hl2 - mult * atr;
        if (i == 0u) { upper_band = raw_upper; lower_band = raw_lower; }
        else {
            upper_band = select(raw_upper, min(raw_upper, upper_band), raw_upper < upper_band || bars[(i - 1u) * 3u + 2u] > upper_band);
            lower_band = select(raw_lower, max(raw_lower, lower_band), raw_lower > lower_band || bars[(i - 1u) * 3u + 2u] < lower_band);
        }
        if (dir == 1.0 && c < lower_band) { dir = -1.0; }
        else if (dir == -1.0 && c > upper_band) { dir = 1.0; }
        output[i * 2u] = select(upper_band, lower_band, dir == 1.0);
        output[i * 2u + 1u] = dir;
    }
}
"#;

const DONCHIAN_SHADER: &str = r#"
// Donchian Channel — parallel (rolling highest high, lowest low)
// Output: 2 per bar [upper, lower]
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.bar_count) { return; }
    if (i < params.period - 1u) { output[i * 2u] = 0.0; output[i * 2u + 1u] = 0.0; return; }
    var hh: f32 = -1000000.0;
    var ll: f32 = 1000000.0;
    for (var j: u32 = 0u; j < params.period; j = j + 1u) {
        let idx = i - j;
        if (bars[idx * 3u] > hh) { hh = bars[idx * 3u]; }
        if (bars[idx * 3u + 1u] < ll) { ll = bars[idx * 3u + 1u]; }
    }
    output[i * 2u] = hh;
    output[i * 2u + 1u] = ll;
}
"#;

const KELTNER_SHADER: &str = r#"
// Keltner Channel — sequential (EMA ± mult × ATR)
// Output: 3 per bar [upper, mid, lower]
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    let n = params.bar_count;
    let p = params.period;
    let mult: f32 = 1.5;
    if (n < p) { return; }
    let alpha = 2.0 / (f32(p) + 1.0);
    var ema: f32 = bars[2u];
    var atr: f32 = bars[0u] - bars[1u];
    output[0u] = 0.0; output[1u] = 0.0; output[2u] = 0.0;
    for (var i: u32 = 1u; i < n; i = i + 1u) {
        let h = bars[i * 3u]; let l = bars[i * 3u + 1u]; let c = bars[i * 3u + 2u];
        let pc = bars[(i - 1u) * 3u + 2u];
        ema = alpha * c + (1.0 - alpha) * ema;
        let tr = max(h - l, max(abs(h - pc), abs(l - pc)));
        atr = (atr * f32(p - 1u) + tr) / f32(p);
        if (i < p) { output[i * 3u] = 0.0; output[i * 3u + 1u] = 0.0; output[i * 3u + 2u] = 0.0; }
        else {
            output[i * 3u] = ema + mult * atr;
            output[i * 3u + 1u] = ema;
            output[i * 3u + 2u] = ema - mult * atr;
        }
    }
}
"#;

const REGRESSION_SHADER: &str = r#"
// Linear Regression Channel — parallel (least squares + standard error)
// Output: 3 per bar [mid, upper(+2σ), lower(−2σ)]
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.bar_count) { return; }
    let p = params.period;
    if (i < p - 1u) { output[i * 3u] = 0.0; output[i * 3u + 1u] = 0.0; output[i * 3u + 2u] = 0.0; return; }
    let n = f32(p);
    var sum_x: f32 = 0.0; var sum_y: f32 = 0.0; var sum_xy: f32 = 0.0; var sum_xx: f32 = 0.0;
    for (var j: u32 = 0u; j < p; j = j + 1u) {
        let x = f32(j);
        let y = bars[(i - p + 1u + j) * 3u + 2u];
        sum_x += x; sum_y += y; sum_xy += x * y; sum_xx += x * x;
    }
    let denom = n * sum_xx - sum_x * sum_x;
    if (abs(denom) < 0.000001) { let avg = sum_y / n; output[i * 3u] = avg; output[i * 3u + 1u] = avg; output[i * 3u + 2u] = avg; return; }
    let b = (n * sum_xy - sum_x * sum_y) / denom;
    let a = (sum_y - b * sum_x) / n;
    let reg_val = a + b * (n - 1.0);
    var sse: f32 = 0.0;
    for (var j: u32 = 0u; j < p; j = j + 1u) {
        let e = bars[(i - p + 1u + j) * 3u + 2u] - (a + b * f32(j));
        sse += e * e;
    }
    let se = sqrt(sse / n);
    output[i * 3u] = reg_val;
    output[i * 3u + 1u] = reg_val + 2.0 * se;
    output[i * 3u + 2u] = reg_val - 2.0 * se;
}
"#;

const SQUEEZE_SHADER: &str = r#"
// Squeeze Momentum — sequential (BB inside KC detection + momentum)
// Output: 2 per bar [momentum, squeeze_on]
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    let n = params.bar_count;
    let p = params.period;
    let bb_mult: f32 = 2.0;
    let kc_mult: f32 = 1.5;
    if (n < p) { return; }
    let alpha = 2.0 / (f32(p) + 1.0);
    var ema: f32 = bars[2u];
    var atr: f32 = bars[0u] - bars[1u];
    for (var i: u32 = 0u; i < n; i = i + 1u) {
        let h = bars[i * 3u]; let l = bars[i * 3u + 1u]; let c = bars[i * 3u + 2u];
        if (i > 0u) {
            let pc = bars[(i - 1u) * 3u + 2u];
            ema = alpha * c + (1.0 - alpha) * ema;
            atr = (atr * f32(p - 1u) + max(h - l, max(abs(h - pc), abs(l - pc)))) / f32(p);
        }
        if (i < p - 1u) { output[i * 2u] = 0.0; output[i * 2u + 1u] = 0.0; continue; }
        // SMA + StdDev + Donchian over window
        var sum: f32 = 0.0; var hh: f32 = -1e9; var ll: f32 = 1e9;
        for (var j: u32 = 0u; j < p; j = j + 1u) {
            let sc = bars[(i - j) * 3u + 2u]; sum += sc;
            let sh = bars[(i - j) * 3u]; let sl = bars[(i - j) * 3u + 1u];
            if (sh > hh) { hh = sh; } if (sl < ll) { ll = sl; }
        }
        let sma = sum / f32(p);
        var vs: f32 = 0.0;
        for (var j: u32 = 0u; j < p; j = j + 1u) { let d = bars[(i - j) * 3u + 2u] - sma; vs += d * d; }
        let sd = sqrt(vs / f32(p));
        let squeeze_on = select(0.0, 1.0, sma - bb_mult * sd > ema - kc_mult * atr && sma + bb_mult * sd < ema + kc_mult * atr);
        output[i * 2u] = c - ((hh + ll) / 2.0 + sma) / 2.0;
        output[i * 2u + 1u] = squeeze_on;
    }
}
"#;

const PREV_LEVELS_SHADER: &str = r#"
// Previous Candle Levels — parallel (approximate prev day high/low)
// Output: 2 per bar [prev_day_high, prev_day_low]
struct Params { period: u32, bar_count: u32, }  // period = minutes per bar
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.bar_count) { return; }
    let bpd = select(1u, 1440u / max(params.period, 1u), params.period > 0u);
    if (i < bpd) { output[i * 2u] = 0.0; output[i * 2u + 1u] = 0.0; return; }
    let start = i - bpd;
    var ph: f32 = -1e9; var pl: f32 = 1e9;
    for (var j: u32 = start; j < i; j = j + 1u) {
        if (bars[j * 3u] > ph) { ph = bars[j * 3u]; }
        if (bars[j * 3u + 1u] < pl) { pl = bars[j * 3u + 1u]; }
    }
    output[i * 2u] = ph;
    output[i * 2u + 1u] = pl;
}
"#;

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
            forming_bar_gpu_write(5, 111.0, 99.0, 108.5),
            Some(FormingBarGpuWrite {
                scalar_offset: 16,
                ohlc_offset: 48,
                ohlc: [111.0, 99.0, 108.5],
                mid: 105.0,
            })
        );
        assert_eq!(forming_bar_gpu_write(0, 111.0, 99.0, 108.5), None);
    }
}
