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
pub struct GpuCompute {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    /// Bar data in VRAM: [timestamp, open, high, low, close, volume] × bar_count
    bar_buffer: Option<wgpu::Buffer>,
    /// Number of bars currently in GPU buffer.
    bar_count: u32,
    /// SMA output buffer (one f32 per bar).
    sma_buffer: Option<wgpu::Buffer>,
    /// EMA output buffer.
    ema_buffer: Option<wgpu::Buffer>,
    /// Compute pipeline for SMA.
    sma_pipeline: wgpu::ComputePipeline,
    /// Compute pipeline for EMA.
    ema_pipeline: wgpu::ComputePipeline,
    /// Bind group layout for indicator shaders.
    bind_group_layout: wgpu::BindGroupLayout,
    /// Staging buffer for CPU readback.
    readback_buffer: Option<wgpu::Buffer>,
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
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
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

        Self {
            device,
            queue,
            bar_buffer: None,
            bar_count: 0,
            sma_buffer: None,
            ema_buffer: None,
            sma_pipeline,
            ema_pipeline,
            bind_group_layout,
            readback_buffer: None,
        }
    }

    /// Upload bar data to VRAM. Called once per symbol/timeframe load.
    /// Data format: [close_0, close_1, ..., close_n] as f32 array.
    pub fn upload_bars(&mut self, closes: &[f32]) {
        let bar_count = closes.len() as u32;
        self.bar_count = bar_count;

        // Create bar buffer in VRAM
        self.bar_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bar_data"),
            size: (bar_count as u64) * 4, // f32 per bar
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));

        // Upload close prices
        if let Some(ref buf) = self.bar_buffer {
            self.queue.write_buffer(buf, 0, bytemuck_cast_slice(closes));
        }

        // Create output buffers
        let out_size = (bar_count as u64) * 4;
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

        // Readback staging buffer
        self.readback_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: out_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
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

// Utility: cast &[f32] to &[u8] for wgpu buffer writes
fn bytemuck_cast_slice(data: &[impl bytemuck_compatible::Pod]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            data.as_ptr() as *const u8,
            data.len() * std::mem::size_of_val(&data[0]),
        )
    }
}

fn bytemuck_cast_slice_to_f32(data: &[u8]) -> Vec<f32> {
    let count = data.len() / 4;
    let mut result = vec![0.0_f32; count];
    unsafe {
        std::ptr::copy_nonoverlapping(data.as_ptr(), result.as_mut_ptr() as *mut u8, data.len());
    }
    result
}

// Trait for zero-copy casting (replaces bytemuck dependency)
mod bytemuck_compatible {
    pub unsafe trait Pod: Copy + 'static {}
    unsafe impl Pod for f32 {}
    unsafe impl Pod for u32 {}
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
            label: Some("darwin_stats_layout"), bind_group_layouts: &[&stats_bgl], push_constant_ranges: &[],
        });
        let corr_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("darwin_corr_layout"), bind_group_layouts: &[&corr_bgl], push_constant_ranges: &[],
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
            darwin_count: 0, max_days: 0,
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

        // Flatten and pad to max_days stride
        let total_floats = count as usize * max_days as usize;
        let mut flat = vec![0.0_f32; total_floats];
        let mut lengths = vec![0_u32; count as usize];

        for (i, series) in returns.iter().enumerate() {
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

        // Allocate stats output: 10 floats per DARWIN
        let stats_size = count as u64 * 10 * 4;
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

        tracing::info!("GPU: uploaded {} DARWINs × {} max_days ({:.1}MB VRAM)",
            count, max_days, total_floats as f64 * 4.0 / 1024.0 / 1024.0);
    }

    /// Dispatch batch statistics shader. Computes Sharpe/Sortino/DD/etc for ALL DARWINs in one pass.
    pub fn compute_stats(&self) {
        let (Some(ret_buf), Some(len_buf), Some(stats_buf)) =
            (&self.returns_buffer, &self.lengths_buffer, &self.stats_buffer) else { return; };

        // Params: [darwin_count, max_days]
        let params = [self.darwin_count, self.max_days];
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
            pass.dispatch_workgroups((self.darwin_count + 255) / 256, 1, 1);
        }
        self.queue.submit(std::iter::once(encoder.finish()));
    }

    /// Read back computed statistics from GPU.
    pub fn readback_stats(&self) -> Option<Vec<GpuDarwinStats>> {
        let (Some(stats_buf), Some(staging)) = (&self.stats_buffer, &self.staging_buffer) else { return None; };

        let size = self.darwin_count as u64 * 10 * 4;
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

        let data = buffer_slice.get_mapped_range();
        let floats = bytemuck_cast_slice_to_f32(&data);
        staging.unmap();

        let mut results = Vec::with_capacity(self.darwin_count as usize);
        for i in 0..self.darwin_count as usize {
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
        Some(results)
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
