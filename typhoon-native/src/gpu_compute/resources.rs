//! GPU bar upload and pooled buffer resource management.

use super::*;

impl GpuCompute {
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
}
