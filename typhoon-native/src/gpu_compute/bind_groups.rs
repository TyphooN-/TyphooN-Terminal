//! Cached bind-group rebuild helpers for `gpu_compute`.

use super::*;

impl GpuCompute {
    pub(super) fn rebuild_cached_bind_groups(&mut self) {
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
}
