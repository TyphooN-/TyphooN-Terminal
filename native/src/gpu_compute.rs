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
        self.device.poll(wgpu::Maintain::Wait);

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
