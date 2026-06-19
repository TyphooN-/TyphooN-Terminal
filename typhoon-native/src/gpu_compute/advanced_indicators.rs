//! Advanced GPU indicator compute methods.

use super::*;

impl GpuCompute {
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
}
