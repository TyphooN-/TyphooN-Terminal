//! GPU strategy backtester and Monte Carlo compute helpers.

use std::sync::Arc;

use super::{
    BACKTEST_EVAL_SHADER, MONTE_CARLO_SHADER, NNFX_EVAL_SHADER, ROBUSTNESS_SHADER,
    WALK_FORWARD_SHADER, bytemuck_cast_slice, bytemuck_cast_slice_to_f32, wgpu,
};

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
