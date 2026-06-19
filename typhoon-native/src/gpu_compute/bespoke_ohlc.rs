//! Bespoke OHLC/midpoint GPU indicator methods.

use super::{GpuCompute, bytemuck_cast_slice, bytemuck_cast_slice_to_f32};

impl GpuCompute {
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
}
