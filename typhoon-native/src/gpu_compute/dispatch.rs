//! Shared GPU indicator dispatch helpers.

use super::*;

impl GpuCompute {
    /// Generic dispatch: run a compute pipeline with close prices as input, return f32 per bar.
    /// Uses shared output + params buffers (populated in `upload_bars`) to avoid per-call allocations.
    pub(super) fn dispatch_indicator(
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

    pub(super) fn dispatch_custom_input_indicator(
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

    pub(super) fn dispatch_multi_indicator(
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
}
