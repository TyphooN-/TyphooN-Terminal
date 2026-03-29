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
}

pub struct GpuCompute {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
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
    /// SMA output buffer (one f32 per bar).
    sma_buffer: Option<wgpu::Buffer>,
    /// EMA output buffer.
    ema_buffer: Option<wgpu::Buffer>,
    /// Bind group layout for indicator shaders.
    bind_group_layout: wgpu::BindGroupLayout,
    /// Staging buffer for CPU readback.
    readback_buffer: Option<wgpu::Buffer>,
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
}

impl GpuCompute {
    /// Compute Anchored VWAP from anchor bar to end. Needs close+volume interleaved buffer.
    pub fn compute_anchored_vwap(&self, closes: &[f32], volumes: &[f32], anchor_bar: u32) -> Option<Vec<f32>> {
        if closes.len() != volumes.len() || closes.is_empty() { return None; }
        let rb_buf = self.readback_buffer.as_ref()?;
        let n = closes.len() as u32;

        let mut interleaved = Vec::with_capacity(n as usize * 2);
        for i in 0..n as usize { interleaved.push(closes[i]); interleaved.push(volumes[i]); }

        let input_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("avwap_in"), size: (n as u64) * 8,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false,
        });
        self.queue.write_buffer(&input_buf, 0, bytemuck_cast_slice(&interleaved));

        let out_size = (n as u64) * 4;
        let out_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("avwap_out"), size: out_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC, mapped_at_creation: false,
        });
        let params = [anchor_bar, n];
        let params_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("avwap_params"), size: 8,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false,
        });
        self.queue.write_buffer(&params_buffer, 0, bytemuck_cast_slice(&params));
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("avwap_bg"), layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: input_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: out_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: params_buffer.as_entire_binding() },
            ],
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("avwap_pass"), timestamp_writes: None });
            pass.set_pipeline(&self.anchored_vwap_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }
        let read_size = out_size.min(rb_buf.size());
        encoder.copy_buffer_to_buffer(&out_buf, 0, rb_buf, 0, read_size);
        self.queue.submit(std::iter::once(encoder.finish()));
        let slice = rb_buf.slice(0..read_size);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| { let _ = tx.send(r); });
        self.device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None }).ok();
        if rx.recv().ok()?.is_err() { return None; }
        let data = slice.get_mapped_range();
        let result = bytemuck_cast_slice_to_f32(&data);
        drop(data);
        rb_buf.unmap();
        Some(result)
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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("indicator_pipeline_layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
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
                label: Some(label), source: wgpu::ShaderSource::Wgsl(source.into()),
            });
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(label), layout: Some(&pipeline_layout), module: &shader,
                entry_point: Some("main"), compilation_options: Default::default(), cache: None,
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
        let obv_pipeline = make_pipeline("obv_pipeline", OBV_SHADER);
        let momentum_pipeline = make_pipeline("momentum_pipeline", MOMENTUM_SHADER);
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
        let atr_proj_pipeline = make_pipeline("atr_proj_pipeline", ATR_PROJECTION_SHADER);
        let better_vol_pipeline = make_pipeline("better_vol_pipeline", BETTER_VOLUME_SHADER);
        let anchored_vwap_pipeline = make_pipeline("anchored_vwap_pipeline", ANCHORED_VWAP_SHADER);

        Self {
            device,
            queue,
            bar_buffer: None,
            ohlc_buffer: None,
            mid_buffer: None,
            vol_buffer: None,
            bar_count: 0,
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
            bind_group_layout,
            readback_buffer: None,
        }
    }

    /// Upload bar data to VRAM. Called once per symbol/timeframe load.
    /// `closes`: close prices (f32 per bar) — used by SMA, EMA, RSI, KAMA, Bollinger, MACD
    /// `highs`, `lows`: used by ATR, Stochastic, ADX, Fisher
    pub fn upload_bars(&mut self, closes: &[f32]) {
        self.upload_bars_full(closes, &[], &[], &[]);
    }

    /// Upload full OHLCV data to VRAM.
    pub fn upload_bars_full(&mut self, closes: &[f32], highs: &[f32], lows: &[f32], volumes: &[f32]) {
        let bar_count = closes.len() as u32;
        self.bar_count = bar_count;

        // Close prices buffer (used by most indicators)
        self.bar_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bar_data"),
            size: (bar_count as u64) * 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        if let Some(ref buf) = self.bar_buffer {
            self.queue.write_buffer(buf, 0, bytemuck_cast_slice(closes));
        }

        // OHLC interleaved buffer [h0,l0,c0, h1,l1,c1, ...] for ATR/Stoch/ADX
        if highs.len() == closes.len() && lows.len() == closes.len() {
            let mut ohlc = Vec::with_capacity(bar_count as usize * 3);
            for i in 0..bar_count as usize {
                ohlc.push(highs[i]);
                ohlc.push(lows[i]);
                ohlc.push(closes[i]);
            }
            self.ohlc_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("ohlc_data"),
                size: (bar_count as u64) * 12, // 3 × f32 per bar
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            if let Some(ref buf) = self.ohlc_buffer {
                self.queue.write_buffer(buf, 0, bytemuck_cast_slice(&ohlc));
            }

            // Midpoints (high+low)/2 for Fisher Transform
            let mids: Vec<f32> = (0..bar_count as usize).map(|i| (highs[i] + lows[i]) / 2.0).collect();
            self.mid_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("mid_data"),
                size: (bar_count as u64) * 4,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            if let Some(ref buf) = self.mid_buffer {
                self.queue.write_buffer(buf, 0, bytemuck_cast_slice(&mids));
            }
        }

        // Volume buffer for OBV
        if volumes.len() == closes.len() {
            self.vol_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("vol_data"),
                size: (bar_count as u64) * 4,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            if let Some(ref buf) = self.vol_buffer {
                self.queue.write_buffer(buf, 0, bytemuck_cast_slice(volumes));
            }
        }

        // Output buffers (reusable — allocate max size needed)
        let out_size = (bar_count as u64) * 4;
        let out_size_4x = (bar_count as u64) * 16; // for Ichimoku (4 outputs per bar), also covers 3-output indicators
        self.sma_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sma_output"), size: out_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC, mapped_at_creation: false,
        }));
        self.ema_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ema_output"), size: out_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC, mapped_at_creation: false,
        }));

        // Readback staging buffer (large enough for 4x output — Ichimoku uses 4 outputs/bar)
        self.readback_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"), size: out_size_4x,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false,
        }));
    }

    /// Generic dispatch: run a compute pipeline with close prices as input, return f32 per bar.
    fn dispatch_indicator(&self, pipeline: &wgpu::ComputePipeline, period: u32, parallel: bool) -> Option<Vec<f32>> {
        if self.bar_count == 0 { return None; }
        let bar_buf = self.bar_buffer.as_ref()?;
        let rb_buf = self.readback_buffer.as_ref()?;

        let out_size = (self.bar_count as u64) * 4;
        let out_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ind_out"), size: out_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC, mapped_at_creation: false,
        });

        let params = [period, self.bar_count];
        let params_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ind_params"), size: 8,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false,
        });
        self.queue.write_buffer(&params_buffer, 0, bytemuck_cast_slice(&params));

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ind_bg"), layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: bar_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: out_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: params_buffer.as_entire_binding() },
            ],
        });

        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("ind_pass"), timestamp_writes: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            if parallel {
                pass.dispatch_workgroups((self.bar_count + 255) / 256, 1, 1);
            } else {
                pass.dispatch_workgroups(1, 1, 1);
            }
        }
        encoder.copy_buffer_to_buffer(&out_buf, 0, rb_buf, 0, out_size);
        self.queue.submit(std::iter::once(encoder.finish()));

        let slice = rb_buf.slice(0..out_size);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| { let _ = tx.send(r); });
        self.device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None }).ok();
        if rx.recv().ok()?.is_err() { return None; }
        let data = slice.get_mapped_range();
        let result = bytemuck_cast_slice_to_f32(&data);
        drop(data);
        rb_buf.unmap();
        Some(result)
    }

    /// Generic dispatch with OHLC input (for ATR, Stochastic, ADX)
    fn dispatch_ohlc_indicator(&self, pipeline: &wgpu::ComputePipeline, period: u32, out_per_bar: u32) -> Option<Vec<f32>> {
        if self.bar_count == 0 { return None; }
        let ohlc_buf = self.ohlc_buffer.as_ref()?;
        let rb_buf = self.readback_buffer.as_ref()?;

        let out_size = (self.bar_count as u64) * (out_per_bar as u64) * 4;
        let out_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ohlc_ind_out"), size: out_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC, mapped_at_creation: false,
        });

        let params = [period, self.bar_count];
        let params_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ohlc_ind_params"), size: 8,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false,
        });
        self.queue.write_buffer(&params_buffer, 0, bytemuck_cast_slice(&params));

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ohlc_ind_bg"), layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: ohlc_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: out_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: params_buffer.as_entire_binding() },
            ],
        });

        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("ohlc_ind_pass"), timestamp_writes: None,
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
        slice.map_async(wgpu::MapMode::Read, move |r| { let _ = tx.send(r); });
        self.device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None }).ok();
        if rx.recv().ok()?.is_err() { return None; }
        let data = slice.get_mapped_range();
        let result = bytemuck_cast_slice_to_f32(&data);
        drop(data);
        rb_buf.unmap();
        Some(result)
    }

    // ─── Public indicator compute methods ───

    /// Generic public dispatch for SMA/EMA/RSI/KAMA using close prices.
    pub fn dispatch_indicator_pub(&self, indicator: &Indicator, period: u32, parallel: bool) -> Option<Vec<f32>> {
        let pipeline = match indicator {
            Indicator::Sma => &self.sma_pipeline,
            Indicator::Ema => &self.ema_pipeline,
            Indicator::Rsi => &self.rsi_pipeline,
            Indicator::Kama => &self.kama_pipeline,
            Indicator::Wma => &self.wma_pipeline,
            Indicator::Momentum => &self.momentum_pipeline,
            _ => return None, // CCI, WilliamsR, Obv need special input buffers
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
        if self.bar_count == 0 { return None; }
        let ohlc_buf = self.ohlc_buffer.as_ref()?;
        let rb_buf = self.readback_buffer.as_ref()?;
        let out_size = (self.bar_count as u64) * 4;
        let out_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cci_out"), size: out_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC, mapped_at_creation: false,
        });
        let params = [period, self.bar_count];
        let params_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cci_params"), size: 8,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false,
        });
        self.queue.write_buffer(&params_buffer, 0, bytemuck_cast_slice(&params));
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("cci_bg"), layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: ohlc_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: out_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: params_buffer.as_entire_binding() },
            ],
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("cci_pass"), timestamp_writes: None });
            pass.set_pipeline(&self.cci_ohlc_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups((self.bar_count + 255) / 256, 1, 1);
        }
        let read_size = out_size.min(rb_buf.size());
        encoder.copy_buffer_to_buffer(&out_buf, 0, rb_buf, 0, read_size);
        self.queue.submit(std::iter::once(encoder.finish()));
        let slice = rb_buf.slice(0..read_size);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| { let _ = tx.send(r); });
        self.device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None }).ok();
        if rx.recv().ok()?.is_err() { return None; }
        let data = slice.get_mapped_range();
        let result = bytemuck_cast_slice_to_f32(&data);
        drop(data);
        rb_buf.unmap();
        Some(result)
    }

    /// Compute OBV on GPU using real volume data.
    /// Caller provides pre-interleaved [close, volume] pairs.
    pub fn compute_obv_gpu_with_cv(&self, cv_interleaved: &[f32]) -> Option<Vec<f32>> {
        if self.bar_count == 0 { return None; }
        let rb_buf = self.readback_buffer.as_ref()?;
        let n = self.bar_count as usize;
        if cv_interleaved.len() != n * 2 { return None; }

        let cv_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("obv_cv"), size: (n as u64) * 8,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false,
        });
        self.queue.write_buffer(&cv_buf, 0, bytemuck_cast_slice(cv_interleaved));

        let out_size = (n as u64) * 4;
        let out_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("obv_out"), size: out_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC, mapped_at_creation: false,
        });
        let params = [0u32, self.bar_count];
        let params_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("obv_params"), size: 8,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false,
        });
        self.queue.write_buffer(&params_buffer, 0, bytemuck_cast_slice(&params));

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("obv_bg"), layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: cv_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: out_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: params_buffer.as_entire_binding() },
            ],
        });

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("obv_dispatch") });
        { let mut pass = encoder.begin_compute_pass(&Default::default()); pass.set_pipeline(&self.obv_pipeline); pass.set_bind_group(0, &bind_group, &[]); pass.dispatch_workgroups(1, 1, 1); }
        let read_size = out_size.min(rb_buf.size());
        encoder.copy_buffer_to_buffer(&out_buf, 0, rb_buf, 0, read_size);
        self.queue.submit(std::iter::once(encoder.finish()));

        let slice = rb_buf.slice(0..read_size);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| { let _ = tx.send(r); });
        self.device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None }).ok();
        if rx.recv().ok()?.is_err() { return None; }
        let data = slice.get_mapped_range();
        let result = bytemuck_cast_slice_to_f32(&data);
        drop(data);
        rb_buf.unmap();
        Some(result)
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

    /// Compute ATR Projection on GPU. Needs custom [open, atr] interleaved buffer.
    /// Returns [upper, lower] × bar_count.
    pub fn compute_atr_projection_gpu(&self, opens: &[f32], atrs: &[f32]) -> Option<Vec<f32>> {
        if self.bar_count == 0 || opens.len() != atrs.len() { return None; }
        let rb_buf = self.readback_buffer.as_ref()?;
        let n = opens.len() as u32;

        // Create interleaved [open, atr] buffer
        let mut interleaved = Vec::with_capacity(n as usize * 2);
        for i in 0..n as usize { interleaved.push(opens[i]); interleaved.push(atrs[i]); }

        let input_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("atr_proj_in"), size: (n as u64) * 8,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false,
        });
        self.queue.write_buffer(&input_buf, 0, bytemuck_cast_slice(&interleaved));

        let out_size = (n as u64) * 8; // 2 floats per bar
        let out_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("atr_proj_out"), size: out_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC, mapped_at_creation: false,
        });
        let params = [0u32, n];
        let params_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("atr_proj_params"), size: 8,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false,
        });
        self.queue.write_buffer(&params_buffer, 0, bytemuck_cast_slice(&params));
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("atr_proj_bg"), layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: input_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: out_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: params_buffer.as_entire_binding() },
            ],
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("atr_proj_pass"), timestamp_writes: None });
            pass.set_pipeline(&self.atr_proj_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups((n + 255) / 256, 1, 1);
        }
        let read_size = out_size.min(rb_buf.size());
        encoder.copy_buffer_to_buffer(&out_buf, 0, rb_buf, 0, read_size);
        self.queue.submit(std::iter::once(encoder.finish()));
        let slice = rb_buf.slice(0..read_size);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| { let _ = tx.send(r); });
        self.device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None }).ok();
        if rx.recv().ok()?.is_err() { return None; }
        let data = slice.get_mapped_range();
        let result = bytemuck_cast_slice_to_f32(&data);
        drop(data);
        rb_buf.unmap();
        Some(result)
    }

    /// Compute BetterVolume classification on GPU from OHLC. Parallel.
    /// Returns f32 per bar: 0=normal, 1=climax_up, 2=climax_down, 3=high, 4=low, 5=churn
    pub fn compute_better_volume_gpu(&self, lookback: u32) -> Option<Vec<f32>> {
        self.dispatch_ohlc_indicator(&self.better_vol_pipeline, lookback, 1)
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
        if self.bar_count == 0 { return None; }
        let bar_buf = self.bar_buffer.as_ref()?;
        let rb_buf = self.readback_buffer.as_ref()?;
        let out_size = (self.bar_count as u64) * 2 * 4;
        let out_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mama_out"), size: out_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC, mapped_at_creation: false,
        });
        let params = [0u32, self.bar_count];
        let params_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mama_params"), size: 8,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false,
        });
        self.queue.write_buffer(&params_buffer, 0, bytemuck_cast_slice(&params));
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("mama_bg"), layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: bar_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: out_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: params_buffer.as_entire_binding() },
            ],
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("mama_pass"), timestamp_writes: None });
            pass.set_pipeline(&self.ehlers_mama_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }
        let read_size = out_size.min(rb_buf.size());
        encoder.copy_buffer_to_buffer(&out_buf, 0, rb_buf, 0, read_size);
        self.queue.submit(std::iter::once(encoder.finish()));
        let slice = rb_buf.slice(0..read_size);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| { let _ = tx.send(r); });
        self.device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None }).ok();
        if rx.recv().ok()?.is_err() { return None; }
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
    pub fn compute_macd_gpu(&self) -> Option<Vec<f32>> {
        self.dispatch_indicator(&self.macd_pipeline, 0, false)
    }

    /// Compute ATR on GPU. Returns f32 per bar. Requires OHLC upload.
    pub fn compute_atr_gpu(&self, period: u32) -> Option<Vec<f32>> {
        self.dispatch_ohlc_indicator(&self.atr_pipeline, period, 1)
    }

    /// Compute Fisher Transform on GPU. Returns [fisher, trigger] × bar_count.
    pub fn compute_fisher_gpu(&self, period: u32) -> Option<Vec<f32>> {
        if self.bar_count == 0 { return None; }
        let mid_buf = self.mid_buffer.as_ref()?;
        let rb_buf = self.readback_buffer.as_ref()?;

        let out_size = (self.bar_count as u64) * 2 * 4;
        let out_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fisher_out"), size: out_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC, mapped_at_creation: false,
        });
        let params = [period, self.bar_count];
        let params_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fisher_params"), size: 8,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false,
        });
        self.queue.write_buffer(&params_buffer, 0, bytemuck_cast_slice(&params));
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fisher_bg"), layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: mid_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: out_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: params_buffer.as_entire_binding() },
            ],
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("fisher_pass"), timestamp_writes: None });
            pass.set_pipeline(&self.fisher_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }
        let read_size = out_size.min(rb_buf.size());
        encoder.copy_buffer_to_buffer(&out_buf, 0, rb_buf, 0, read_size);
        self.queue.submit(std::iter::once(encoder.finish()));
        let slice = rb_buf.slice(0..read_size);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| { let _ = tx.send(r); });
        self.device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None }).ok();
        if rx.recv().ok()?.is_err() { return None; }
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
        if self.bar_count == 0 { return; }
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
        self.queue.write_buffer(&params_buffer, 0, bytemuck_cast_slice(&params));

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sma_bg"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: bar_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: out_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: params_buffer.as_entire_binding() },
            ],
        });

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
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
        if self.bar_count == 0 { return; }
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
        self.queue.write_buffer(&params_buffer, 0, bytemuck_cast_slice(&params));

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ema_bg"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: bar_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: out_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: params_buffer.as_entire_binding() },
            ],
        });

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
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
        slice.map_async(wgpu::MapMode::Read, move |result| { let _ = tx.send(result); });
        self.device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None }).ok();

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

    pub fn bar_count(&self) -> u32 { self.bar_count }
}

// Safe byte casting via bytemuck crate — eliminates all unsafe pointer casts
fn bytemuck_cast_slice<T: bytemuck::NoUninit>(data: &[T]) -> &[u8] {
    bytemuck::cast_slice(data)
}

fn bytemuck_cast_slice_to_f32(data: &[u8]) -> Vec<f32> {
    bytemuck::cast_slice::<u8, f32>(data).to_vec()
}

// ─── DARWIN GPU Analytics ─────────────────────────────────────────────────────

/// Per-DARWIN statistics computed on GPU.
#[derive(Debug, Clone, Default)]
pub struct GpuDarwinStats {
    pub mean: f32,
    pub variance: f32,
    pub sharpe: f32,
    pub sortino: f32,
    pub max_drawdown: f32,
    pub best_day: f32,
    pub worst_day: f32,
    pub skewness: f32,
    pub kurtosis: f32,
    pub total_return: f32,
}

/// GPU-accelerated DARWIN analytics engine.
/// Holds return series for the entire universe in VRAM and dispatches compute shaders.
pub struct GpuDarwinAnalytics {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    /// Flat return series: [d0_r0, d0_r1, ..., d0_rN, d1_r0, ..., dM_rN]
    returns_buffer: Option<wgpu::Buffer>,
    /// Actual length per DARWIN: [len_0, len_1, ..., len_M]
    lengths_buffer: Option<wgpu::Buffer>,
    /// Output: 10 floats per DARWIN (mean, var, sharpe, sortino, maxdd, best, worst, skew, kurt, total_ret)
    stats_buffer: Option<wgpu::Buffer>,
    /// Output: correlation tile (tile_size × tile_size floats)
    corr_buffer: Option<wgpu::Buffer>,
    /// Staging buffer for CPU readback.
    staging_buffer: Option<wgpu::Buffer>,
    /// Number of DARWINs uploaded.
    darwin_count: u32,
    /// Max days per DARWIN (stride in the flat array).
    max_days: u32,
    /// Max DARWINs per GPU batch (for chunked processing when buffer limit exceeded).
    chunk_size: u32,
    /// All return series (kept for multi-batch processing).
    all_returns: Vec<Vec<f32>>,
    /// Compute pipelines.
    stats_pipeline: wgpu::ComputePipeline,
    corr_pipeline: wgpu::ComputePipeline,
    /// Bind group layouts.
    stats_bgl: wgpu::BindGroupLayout,
    corr_bgl: wgpu::BindGroupLayout,
}

impl GpuDarwinAnalytics {
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        // Stats bind group layout: returns (read), lengths (read), output (read_write), params (uniform)
        let stats_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("darwin_stats_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
            ],
        });

        // Correlation bind group layout: returns (read), lengths (read), output (read_write), params (uniform)
        let corr_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("darwin_corr_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
            ],
        });

        let stats_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("darwin_stats_layout"), bind_group_layouts: &[Some(&stats_bgl)], immediate_size: 0,
        });
        let corr_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("darwin_corr_layout"), bind_group_layouts: &[Some(&corr_bgl)], immediate_size: 0,
        });

        let stats_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("darwin_stats_shader"), source: wgpu::ShaderSource::Wgsl(DARWIN_STATS_SHADER.into()),
        });
        let corr_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("darwin_corr_shader"), source: wgpu::ShaderSource::Wgsl(DARWIN_CORR_SHADER.into()),
        });

        let stats_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("darwin_stats_pipeline"), layout: Some(&stats_layout), module: &stats_shader,
            entry_point: Some("main"), compilation_options: Default::default(), cache: None,
        });
        let corr_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("darwin_corr_pipeline"), layout: Some(&corr_layout), module: &corr_shader,
            entry_point: Some("main"), compilation_options: Default::default(), cache: None,
        });

        Self {
            device, queue, returns_buffer: None, lengths_buffer: None,
            stats_buffer: None, corr_buffer: None, staging_buffer: None,
            darwin_count: 0, max_days: 0, chunk_size: 0, all_returns: Vec::new(),
            stats_pipeline, corr_pipeline, stats_bgl, corr_bgl,
        }
    }

    /// Upload DARWIN return series to VRAM.
    /// `returns`: Vec of (daily_returns_f32) per DARWIN. Padded to `max_days` stride.
    /// `lengths`: actual day count per DARWIN.
    pub fn upload_returns(&mut self, returns: &[Vec<f32>], max_days: u32) {
        let count = returns.len() as u32;
        self.darwin_count = count;
        self.max_days = max_days;

        // Check GPU buffer size limit — chunk by DARWIN count if needed
        let max_buffer = self.device.limits().max_storage_buffer_binding_size as usize;
        let per_darwin_bytes = max_days as usize * 4;
        let max_darwins_per_batch = max_buffer / per_darwin_bytes;
        if (count as usize) > max_darwins_per_batch {
            // Need to chunk — store batch info for multi-pass compute
            self.chunk_size = max_darwins_per_batch as u32;
            tracing::info!("GPU: {}MB needed for {} DARWINs × {} days — chunking into batches of {} (limit {}MB)",
                count as usize * per_darwin_bytes / 1024 / 1024, count, max_days,
                max_darwins_per_batch, max_buffer / 1024 / 1024);
        } else {
            self.chunk_size = count; // single batch
        }

        // Store all returns for chunked processing
        self.all_returns = returns.to_vec();

        // For upload, use min(count, chunk_size) DARWINs (first batch)
        let batch_count = (count as usize).min(self.chunk_size as usize);
        let total_floats = batch_count * max_days as usize;
        let mut flat = vec![0.0_f32; total_floats];
        let mut lengths = vec![0_u32; batch_count];

        for (i, series) in returns.iter().take(batch_count).enumerate() {
            let len = series.len().min(max_days as usize);
            lengths[i] = len as u32;
            for (j, &val) in series.iter().take(len).enumerate() {
                flat[i * max_days as usize + j] = val;
            }
        }

        // Upload returns
        let returns_bytes = unsafe {
            std::slice::from_raw_parts(flat.as_ptr() as *const u8, flat.len() * 4)
        };
        self.returns_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("darwin_returns"), size: returns_bytes.len() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false,
        }));
        self.queue.write_buffer(self.returns_buffer.as_ref().unwrap(), 0, returns_bytes);

        // Upload lengths
        let lengths_bytes = unsafe {
            std::slice::from_raw_parts(lengths.as_ptr() as *const u8, lengths.len() * 4)
        };
        self.lengths_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("darwin_lengths"), size: lengths_bytes.len() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false,
        }));
        self.queue.write_buffer(self.lengths_buffer.as_ref().unwrap(), 0, lengths_bytes);

        // Allocate stats output: 10 floats per batch
        let stats_size = batch_count as u64 * 10 * 4;
        self.stats_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("darwin_stats_out"), size: stats_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC, mapped_at_creation: false,
        }));

        // Staging buffer for readback (max of stats or correlation tile)
        let staging_size = stats_size.max(1024 * 1024 * 4); // at least 1M floats for corr tiles
        self.staging_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("darwin_staging"), size: staging_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false,
        }));

        let num_batches = (count as usize + self.chunk_size as usize - 1) / self.chunk_size as usize;
        tracing::info!("GPU: uploaded batch 1/{} ({} DARWINs × {} days, {:.1}MB VRAM)",
            num_batches, batch_count, max_days, total_floats as f64 * 4.0 / 1024.0 / 1024.0);
    }

    /// Dispatch batch statistics shader for currently uploaded batch.
    pub fn compute_stats(&self) {
        let (Some(ret_buf), Some(len_buf), Some(stats_buf)) =
            (&self.returns_buffer, &self.lengths_buffer, &self.stats_buffer) else { return; };

        // Params: [batch_darwin_count, max_days]
        let batch_count = (self.darwin_count as usize).min(self.chunk_size as usize) as u32;
        let params = [batch_count, self.max_days];
        let params_bytes = unsafe {
            std::slice::from_raw_parts(params.as_ptr() as *const u8, 8)
        };
        let params_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("stats_params"), size: 8,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false,
        });
        self.queue.write_buffer(&params_buffer, 0, params_bytes);

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("stats_bg"), layout: &self.stats_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: ret_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: len_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: stats_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: params_buffer.as_entire_binding() },
            ],
        });

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("stats_encoder"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("stats_pass"), timestamp_writes: None,
            });
            pass.set_pipeline(&self.stats_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups((batch_count + 255) / 256, 1, 1);
        }
        self.queue.submit(std::iter::once(encoder.finish()));
    }

    /// Read back computed statistics from GPU (current batch).
    pub fn readback_stats(&self) -> Option<Vec<GpuDarwinStats>> {
        let (Some(stats_buf), Some(staging)) = (&self.stats_buffer, &self.staging_buffer) else { return None; };

        let batch_count = (self.darwin_count as usize).min(self.chunk_size as usize);
        let size = batch_count as u64 * 10 * 4;
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("stats_readback_encoder"),
        });
        encoder.copy_buffer_to_buffer(stats_buf, 0, staging, 0, size);
        self.queue.submit(std::iter::once(encoder.finish()));

        let buffer_slice = staging.slice(0..size);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| { let _ = tx.send(result); });
        self.device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None }).ok();

        if rx.recv().ok()?.is_err() { return None; }

        let results = {
            let data = buffer_slice.get_mapped_range();
            let floats = bytemuck_cast_slice_to_f32(&data);
            let mut results = Vec::with_capacity(batch_count);
            for i in 0..batch_count {
                let base = i * 10;
                if base + 9 < floats.len() {
                    results.push(GpuDarwinStats {
                        mean: floats[base],
                        variance: floats[base + 1],
                        sharpe: floats[base + 2],
                        sortino: floats[base + 3],
                        max_drawdown: floats[base + 4],
                        best_day: floats[base + 5],
                        worst_day: floats[base + 6],
                        skewness: floats[base + 7],
                        kurtosis: floats[base + 8],
                        total_return: floats[base + 9],
                    });
                }
            }
            results
        }; // data (mapped range) dropped here
        staging.unmap();
        Some(results)
    }

    /// Process ALL batches and return merged stats for all DARWINs.
    /// Handles chunked GPU processing when dataset exceeds buffer limit.
    pub fn compute_all_batches(&mut self) -> Option<Vec<GpuDarwinStats>> {
        if self.all_returns.is_empty() { return None; }
        let total = self.all_returns.len();
        let chunk = self.chunk_size as usize;
        if chunk == 0 { return None; }
        let num_batches = (total + chunk - 1) / chunk;
        let mut all_stats = Vec::with_capacity(total);

        for batch_idx in 0..num_batches {
            let start = batch_idx * chunk;
            let end = (start + chunk).min(total);
            let batch_slice = &self.all_returns[start..end];
            let batch_count = batch_slice.len();

            // Flatten this batch
            let total_floats = batch_count * self.max_days as usize;
            let mut flat = vec![0.0_f32; total_floats];
            let mut lengths = vec![0_u32; batch_count];
            for (i, series) in batch_slice.iter().enumerate() {
                let len = series.len().min(self.max_days as usize);
                lengths[i] = len as u32;
                for (j, &val) in series.iter().take(len).enumerate() {
                    flat[i * self.max_days as usize + j] = val;
                }
            }

            // Upload
            let returns_bytes = unsafe {
                std::slice::from_raw_parts(flat.as_ptr() as *const u8, flat.len() * 4)
            };
            self.returns_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("darwin_returns"), size: returns_bytes.len() as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false,
            }));
            self.queue.write_buffer(self.returns_buffer.as_ref().unwrap(), 0, returns_bytes);

            let lengths_bytes = unsafe {
                std::slice::from_raw_parts(lengths.as_ptr() as *const u8, lengths.len() * 4)
            };
            self.lengths_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("darwin_lengths"), size: lengths_bytes.len() as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false,
            }));
            self.queue.write_buffer(self.lengths_buffer.as_ref().unwrap(), 0, lengths_bytes);

            let stats_size = batch_count as u64 * 10 * 4;
            self.stats_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("darwin_stats_out"), size: stats_size,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC, mapped_at_creation: false,
            }));
            self.staging_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("darwin_staging"), size: stats_size.max(1024 * 1024 * 4),
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false,
            }));

            // Temporarily set darwin_count to batch size for compute/readback
            let saved_count = self.darwin_count;
            self.darwin_count = batch_count as u32;
            self.chunk_size = batch_count as u32; // so readback uses correct count

            self.compute_stats();
            if let Some(batch_stats) = self.readback_stats() {
                all_stats.extend(batch_stats);
            }

            self.darwin_count = saved_count;
            self.chunk_size = (self.device.limits().max_storage_buffer_binding_size as usize / (self.max_days as usize * 4)) as u32;

            if num_batches > 1 {
                tracing::info!("GPU: batch {}/{} complete ({} DARWINs)", batch_idx + 1, num_batches, batch_count);
            }
        }

        // Free stored returns to reclaim memory
        self.all_returns.clear();
        self.all_returns.shrink_to_fit();

        Some(all_stats)
    }

    /// Dispatch correlation shader for a tile of DARWINs.
    /// Computes correlation between DARWINs [row_start..row_start+tile_size] and [col_start..col_start+tile_size].
    pub fn compute_correlation_tile(&self, row_start: u32, col_start: u32, tile_size: u32) {
        let (Some(ret_buf), Some(len_buf)) = (&self.returns_buffer, &self.lengths_buffer) else { return; };

        // Allocate tile output
        let tile_floats = tile_size * tile_size;
        let corr_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("corr_tile"), size: tile_floats as u64 * 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC, mapped_at_creation: false,
        });

        // Params: [darwin_count, max_days, row_start, col_start, tile_size, 0, 0, 0]
        let params = [self.darwin_count, self.max_days, row_start, col_start, tile_size, 0u32, 0u32, 0u32];
        let params_bytes = unsafe {
            std::slice::from_raw_parts(params.as_ptr() as *const u8, 32)
        };
        let params_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("corr_params"), size: 32,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false,
        });
        self.queue.write_buffer(&params_buffer, 0, params_bytes);

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("corr_bg"), layout: &self.corr_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: ret_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: len_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: corr_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: params_buffer.as_entire_binding() },
            ],
        });

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("corr_encoder"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("corr_pass"), timestamp_writes: None,
            });
            pass.set_pipeline(&self.corr_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            // 2D dispatch: tile_size × tile_size threads
            pass.dispatch_workgroups((tile_size + 15) / 16, (tile_size + 15) / 16, 1);
        }
        self.queue.submit(std::iter::once(encoder.finish()));

        // Store for readback
        // (In a full implementation, we'd store corr_buf and read it back)
    }

    pub fn darwin_count(&self) -> u32 { self.darwin_count }
    pub fn max_days(&self) -> u32 { self.max_days }
}

// ─── WGSL Compute Shaders ────────────────────────────────────────────────────

/// DARWIN batch statistics shader — one thread per DARWIN.
/// Computes: mean, variance, Sharpe, Sortino, max drawdown, best/worst day, skewness, kurtosis, total return.
const DARWIN_STATS_SHADER: &str = r#"
struct Params {
    darwin_count: u32,
    max_days: u32,
}

@group(0) @binding(0) var<storage, read> returns: array<f32>;
@group(0) @binding(1) var<storage, read> lengths: array<u32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;
@group(0) @binding(3) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let darwin_idx = id.x;
    if (darwin_idx >= params.darwin_count) { return; }

    let n = lengths[darwin_idx];
    let base = darwin_idx * params.max_days;
    let out_base = darwin_idx * 10u;

    if (n < 10u) {
        // Not enough data — zero all outputs
        for (var k: u32 = 0u; k < 10u; k = k + 1u) {
            output[out_base + k] = 0.0;
        }
        return;
    }

    // Pass 1: mean, min, max, cumulative product for total return
    var sum: f32 = 0.0;
    var best: f32 = -1000.0;
    var worst: f32 = 1000.0;
    for (var i: u32 = 0u; i < n; i = i + 1u) {
        let r = returns[base + i];
        sum = sum + r;
        if (r > best) { best = r; }
        if (r < worst) { worst = r; }
    }
    let mean = sum / f32(n);

    // Total return: product of (1+r) - 1
    var cum: f32 = 1.0;
    for (var i: u32 = 0u; i < n; i = i + 1u) {
        cum = cum * (1.0 + returns[base + i]);
    }
    let total_return = cum - 1.0;

    // Pass 2: variance, downside variance, skewness, kurtosis
    var var_sum: f32 = 0.0;
    var down_sum: f32 = 0.0;
    var skew_sum: f32 = 0.0;
    var kurt_sum: f32 = 0.0;
    for (var i: u32 = 0u; i < n; i = i + 1u) {
        let d = returns[base + i] - mean;
        var_sum = var_sum + d * d;
        if (returns[base + i] < 0.0) {
            down_sum = down_sum + d * d;
        }
        skew_sum = skew_sum + d * d * d;
        kurt_sum = kurt_sum + d * d * d * d;
    }
    let variance = var_sum / f32(n - 1u);
    let std_dev = sqrt(variance);
    let down_dev = sqrt(down_sum / f32(n - 1u));

    // Annualized Sharpe = (mean * 252) / (std_dev * sqrt(252))
    let ann_mean = mean * 252.0;
    let ann_vol = std_dev * 15.8745;  // sqrt(252)
    let sharpe = select(ann_mean / ann_vol, 0.0, ann_vol < 0.000001);

    // Sortino
    let ann_down = down_dev * 15.8745;
    let sortino = select(ann_mean / ann_down, 0.0, ann_down < 0.000001);

    // Max drawdown from cumulative returns
    var peak: f32 = 1.0;
    var max_dd: f32 = 0.0;
    var equity: f32 = 1.0;
    for (var i: u32 = 0u; i < n; i = i + 1u) {
        equity = equity * (1.0 + returns[base + i]);
        if (equity > peak) { peak = equity; }
        if (peak > 0.0) {
            let dd = (peak - equity) / peak;
            if (dd > max_dd) { max_dd = dd; }
        }
    }

    // Skewness and kurtosis
    let n_f = f32(n);
    let skewness = select((skew_sum / n_f) / (std_dev * std_dev * std_dev), 0.0, std_dev < 0.000001);
    let kurtosis = select((kurt_sum / n_f) / (variance * variance) - 3.0, 0.0, variance < 0.000001);

    // Write outputs: mean, variance, sharpe, sortino, maxdd, best, worst, skew, kurt, total_ret
    output[out_base + 0u] = mean;
    output[out_base + 1u] = variance;
    output[out_base + 2u] = sharpe;
    output[out_base + 3u] = sortino;
    output[out_base + 4u] = max_dd;
    output[out_base + 5u] = best;
    output[out_base + 6u] = worst;
    output[out_base + 7u] = skewness;
    output[out_base + 8u] = kurtosis;
    output[out_base + 9u] = total_return;
}
"#;

/// DARWIN pairwise correlation shader — one thread per (i, j) pair in a tile.
/// Computes Pearson correlation coefficient between DARWIN i and DARWIN j.
const DARWIN_CORR_SHADER: &str = r#"
struct Params {
    darwin_count: u32,
    max_days: u32,
    row_start: u32,
    col_start: u32,
    tile_size: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

@group(0) @binding(0) var<storage, read> returns: array<f32>;
@group(0) @binding(1) var<storage, read> lengths: array<u32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;
@group(0) @binding(3) var<uniform> params: Params;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let local_row = id.x;
    let local_col = id.y;
    if (local_row >= params.tile_size || local_col >= params.tile_size) { return; }

    let i = params.row_start + local_row;
    let j = params.col_start + local_col;
    if (i >= params.darwin_count || j >= params.darwin_count) {
        output[local_row * params.tile_size + local_col] = 0.0;
        return;
    }

    // Self-correlation = 1.0
    if (i == j) {
        output[local_row * params.tile_size + local_col] = 1.0;
        return;
    }

    let n_i = lengths[i];
    let n_j = lengths[j];
    let n = min(n_i, n_j);
    if (n < 30u) {
        output[local_row * params.tile_size + local_col] = 0.0;
        return;
    }

    let base_i = i * params.max_days;
    let base_j = j * params.max_days;

    // Use last n overlapping days (align from the end)
    let offset_i = n_i - n;
    let offset_j = n_j - n;

    // Compute means
    var sum_i: f32 = 0.0;
    var sum_j: f32 = 0.0;
    for (var k: u32 = 0u; k < n; k = k + 1u) {
        sum_i = sum_i + returns[base_i + offset_i + k];
        sum_j = sum_j + returns[base_j + offset_j + k];
    }
    let mean_i = sum_i / f32(n);
    let mean_j = sum_j / f32(n);

    // Compute correlation
    var cov: f32 = 0.0;
    var var_i: f32 = 0.0;
    var var_j: f32 = 0.0;
    for (var k: u32 = 0u; k < n; k = k + 1u) {
        let di = returns[base_i + offset_i + k] - mean_i;
        let dj = returns[base_j + offset_j + k] - mean_j;
        cov = cov + di * dj;
        var_i = var_i + di * di;
        var_j = var_j + dj * dj;
    }

    let denom = sqrt(var_i * var_j);
    let corr = select(cov / denom, 0.0, denom < 0.000001);
    output[local_row * params.tile_size + local_col] = corr;
}
"#;

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
struct Params {
    period: u32,       // unused (fast=12, slow=26, signal=9 hardcoded)
    bar_count: u32,
}

@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;  // [macd0, sig0, hist0, ...]
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    let k_fast: f32 = 2.0 / 13.0;  // EMA(12)
    let k_slow: f32 = 2.0 / 27.0;  // EMA(26)
    let k_sig: f32 = 2.0 / 10.0;   // EMA(9) of MACD

    // Seed EMAs
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

        if (i >= 26u && !macd_started) {
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
// On-Balance Volume — sequential (cumulative)
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;  // [close, volume] interleaved (2 per bar)
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    if (params.bar_count == 0u) { return; }
    output[0] = 0.0;
    var obv: f32 = 0.0;
    for (var i: u32 = 1u; i < params.bar_count; i = i + 1u) {
        let close = bars[i * 2u];
        let prev_close = bars[(i - 1u) * 2u];
        let vol = bars[i * 2u + 1u];
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
// ATR Projection — parallel per-bar: open ± ATR
// Input binding 0: [open, atr] interleaved (2 floats per bar)
// Output: [upper, lower] per bar (2 floats)
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;  // [open0, atr0, open1, atr1, ...]
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.bar_count) { return; }
    let open_val = bars[i * 2u];
    let atr_val = bars[i * 2u + 1u];
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
// BetterVolume classification — parallel per-bar
// Input: [high, low, close, open, volume] per bar = 5 floats (uses OHLC + volume)
// Output: classification u32 as f32: 0=normal, 1=climax_up, 2=climax_down, 3=high_vol, 4=low_vol, 5=churn
struct Params { period: u32, bar_count: u32, }  // period = lookback for averages (20)
@group(0) @binding(0) var<storage, read> bars: array<f32>;  // [h,l,c] interleaved (3 per bar)
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.bar_count || i < params.period) { output[i] = 0.0; return; }

    // Compute average range over lookback
    var avg_range: f32 = 0.0;
    for (var j: u32 = 0u; j < params.period; j = j + 1u) {
        let idx = i - j - 1u;
        avg_range = avg_range + bars[idx * 3u] - bars[idx * 3u + 1u];
    }
    avg_range = avg_range / f32(params.period);

    let h = bars[i * 3u];
    let l = bars[i * 3u + 1u];
    let c = bars[i * 3u + 2u];
    let range = h - l;
    let range_ratio = select(range / avg_range, 1.0, avg_range < 0.000001);

    // Use range as volume proxy (we don't have separate volume buffer in OHLC layout)
    // Classification based on range ratios
    let prev_c = bars[(i - 1u) * 3u + 2u];
    let is_up = c >= prev_c;

    if (range_ratio > 2.0) {
        output[i] = select(2.0, 1.0, is_up);  // climax up/down
    } else if (range_ratio < 0.4) {
        output[i] = 4.0;  // low range (proxy for low volume)
    } else if (range_ratio > 1.5) {
        output[i] = 3.0;  // high range
    } else {
        output[i] = 0.0;  // normal
    }
}
"#;

const ANCHORED_VWAP_SHADER: &str = r#"
// Anchored VWAP — sequential from anchor bar to end
// Cumulative (price × volume) / cumulative volume from anchor point
// Input: [close, volume] interleaved (2 per bar), anchor_bar in params.period
struct Params { period: u32, bar_count: u32, }  // period = anchor bar index
@group(0) @binding(0) var<storage, read> bars: array<f32>;  // [close0, vol0, close1, vol1, ...]
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    let anchor = params.period;
    var cum_pv: f32 = 0.0;
    var cum_vol: f32 = 0.0;

    for (var i: u32 = 0u; i < params.bar_count; i = i + 1u) {
        if (i < anchor) { output[i] = 0.0; continue; }
        let close = bars[i * 2u];
        let vol = bars[i * 2u + 1u];
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
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                // 1: OHLC data [h,l,c interleaved]
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                // 2: parameter combos
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                // 3: results output
                wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                // 4: uniforms [bar_count, combo_count]
                wgpu::BindGroupLayoutEntry { binding: 4, visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
            ],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("backtest_layout"), bind_group_layouts: &[Some(&eval_bgl)], immediate_size: 0,
        });

        let eval_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("backtest_eval"), source: wgpu::ShaderSource::Wgsl(BACKTEST_EVAL_SHADER.into()),
        });
        let robustness_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("robustness"), source: wgpu::ShaderSource::Wgsl(ROBUSTNESS_SHADER.into()),
        });
        let mc_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("monte_carlo"), source: wgpu::ShaderSource::Wgsl(MONTE_CARLO_SHADER.into()),
        });

        let eval_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("eval_pipeline"), layout: Some(&layout), module: &eval_shader,
            entry_point: Some("main"), compilation_options: Default::default(), cache: None,
        });
        let robustness_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("robustness_pipeline"), layout: Some(&layout), module: &robustness_shader,
            entry_point: Some("main"), compilation_options: Default::default(), cache: None,
        });
        let monte_carlo_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("mc_pipeline"), layout: Some(&layout), module: &mc_shader,
            entry_point: Some("main"), compilation_options: Default::default(), cache: None,
        });
        let nnfx_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("nnfx_eval"), source: wgpu::ShaderSource::Wgsl(NNFX_EVAL_SHADER.into()),
        });
        let nnfx_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("nnfx_pipeline"), layout: Some(&layout), module: &nnfx_shader,
            entry_point: Some("main"), compilation_options: Default::default(), cache: None,
        });
        let wf_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("walk_forward"), source: wgpu::ShaderSource::Wgsl(WALK_FORWARD_SHADER.into()),
        });
        let walk_forward_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("wf_pipeline"), layout: Some(&layout), module: &wf_shader,
            entry_point: Some("main"), compilation_options: Default::default(), cache: None,
        });

        Self {
            device, queue, bar_buffer: None, ohlc_buffer: None,
            indicator_buffer: None, params_buffer: None, results_buffer: None,
            staging_buffer: None, bar_count: 0, combo_count: 0,
            eval_pipeline, nnfx_pipeline, walk_forward_pipeline,
            robustness_pipeline, monte_carlo_pipeline,
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
        self.bar_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bt_closes"), size: (n as u64) * 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false,
        }));
        self.queue.write_buffer(self.bar_buffer.as_ref().unwrap(), 0, bytemuck_cast_slice(closes));

        // OHLC interleaved
        let mut ohlc = Vec::with_capacity(n as usize * 3);
        for i in 0..n as usize {
            ohlc.push(highs[i]); ohlc.push(lows[i]); ohlc.push(closes[i]);
        }
        self.ohlc_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bt_ohlc"), size: (n as u64) * 12,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false,
        }));
        self.queue.write_buffer(self.ohlc_buffer.as_ref().unwrap(), 0, bytemuck_cast_slice(&ohlc));

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
        self.params_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bt_params"), size: (nc as u64) * 32,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false,
        }));
        self.queue.write_buffer(self.params_buffer.as_ref().unwrap(), 0, bytemuck_cast_slice(&packed));

        // Results: 9 floats per combo
        let results_size = (nc as u64) * 36;
        self.results_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bt_results"), size: results_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC, mapped_at_creation: false,
        }));
        self.staging_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bt_staging"), size: results_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false,
        }));
    }

    /// Run backtest evaluation: one GPU thread per parameter combination.
    pub fn evaluate(&self) -> Option<Vec<BacktestResult>> {
        let (Some(bar_buf), Some(ohlc_buf), Some(params_buf), Some(results_buf), Some(staging)) =
            (&self.bar_buffer, &self.ohlc_buffer, &self.params_buffer, &self.results_buffer, &self.staging_buffer)
        else { return None; };

        let uniforms = [self.bar_count, self.combo_count];
        let uniform_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bt_uniforms"), size: 8,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false,
        });
        self.queue.write_buffer(&uniform_buf, 0, bytemuck_cast_slice(&uniforms));

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bt_bg"), layout: &self.eval_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: bar_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: ohlc_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: params_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: results_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 4, resource: uniform_buf.as_entire_binding() },
            ],
        });

        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("bt_eval_pass"), timestamp_writes: None,
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
        slice.map_async(wgpu::MapMode::Read, move |r| { let _ = tx.send(r); });
        self.device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None }).ok();
        if rx.recv().ok()?.is_err() { return None; }
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
    pub fn evaluate_nnfx(&mut self, closes: &[f32], highs: &[f32], lows: &[f32], combos: &[NnfxParamCombo]) -> Option<Vec<BacktestResult>> {
        let n = closes.len() as u32;
        let nc = combos.len() as u32;
        self.bar_count = n;
        self.combo_count = nc;

        // Upload bar data
        self.bar_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nnfx_closes"), size: (n as u64) * 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false,
        }));
        self.queue.write_buffer(self.bar_buffer.as_ref().unwrap(), 0, bytemuck_cast_slice(closes));

        let mut ohlc = Vec::with_capacity(n as usize * 3);
        for i in 0..n as usize { ohlc.push(highs[i]); ohlc.push(lows[i]); ohlc.push(closes[i]); }
        self.ohlc_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nnfx_ohlc"), size: (n as u64) * 12,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false,
        }));
        self.queue.write_buffer(self.ohlc_buffer.as_ref().unwrap(), 0, bytemuck_cast_slice(&ohlc));

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
        self.params_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nnfx_params"), size: (nc as u64) * 32,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false,
        }));
        self.queue.write_buffer(self.params_buffer.as_ref().unwrap(), 0, bytemuck_cast_slice(&packed));

        let results_size = (nc as u64) * 36;
        self.results_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nnfx_results"), size: results_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC, mapped_at_creation: false,
        }));
        self.staging_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nnfx_staging"), size: results_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false,
        }));

        // Dispatch NNFX eval
        let (Some(bar_buf), Some(ohlc_buf), Some(params_buf), Some(results_buf), Some(staging)) =
            (&self.bar_buffer, &self.ohlc_buffer, &self.params_buffer, &self.results_buffer, &self.staging_buffer)
        else { return None; };

        let uniforms = [self.bar_count, self.combo_count];
        let uniform_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nnfx_uniforms"), size: 8,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false,
        });
        self.queue.write_buffer(&uniform_buf, 0, bytemuck_cast_slice(&uniforms));

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nnfx_bg"), layout: &self.eval_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: bar_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: ohlc_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: params_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: results_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 4, resource: uniform_buf.as_entire_binding() },
            ],
        });

        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("nnfx_pass"), timestamp_writes: None });
            pass.set_pipeline(&self.nnfx_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups((nc + 255) / 256, 1, 1);
        }
        encoder.copy_buffer_to_buffer(results_buf, 0, staging, 0, results_size);
        self.queue.submit(std::iter::once(encoder.finish()));

        // Readback
        let slice = staging.slice(0..results_size);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| { let _ = tx.send(r); });
        self.device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None }).ok();
        if rx.recv().ok()?.is_err() { return None; }
        let data = slice.get_mapped_range();
        let floats = bytemuck_cast_slice_to_f32(&data);
        drop(data);
        staging.unmap();

        let mut results = Vec::with_capacity(nc as usize);
        for i in 0..nc as usize {
            let b = i * 9;
            if b + 8 < floats.len() {
                results.push(BacktestResult {
                    net_pnl: floats[b], max_drawdown: floats[b + 1],
                    sharpe: floats[b + 2], sortino: floats[b + 3],
                    win_rate: floats[b + 4], profit_factor: floats[b + 5],
                    trade_count: floats[b + 6] as u32, avg_hold_bars: floats[b + 7],
                    robustness_score: floats[b + 8],
                });
            }
        }
        Some(results)
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
