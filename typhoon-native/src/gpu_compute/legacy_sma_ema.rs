//! Legacy SMA/EMA GPU dispatch and readback methods.

use super::{GpuCompute, Indicator, bytemuck_cast_slice, bytemuck_cast_slice_to_f32, wgpu};

impl GpuCompute {
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
}
