use super::*;

impl ChartState {
    pub(crate) fn natural_visible_price_view(&self) -> Option<(f64, f64)> {
        let (si, ei) = self.visible_range();
        if ei <= si {
            return None;
        }
        let slice = &self.bars[si..ei];
        let hi = slice.iter().map(|b| b.high).fold(f64::MIN, f64::max);
        let lo = slice.iter().map(|b| b.low).fold(f64::MAX, f64::min);
        let padding = (hi - lo).abs() * 0.05;
        let min = lo - padding;
        let max = hi + padding;
        Some(((min + max) * 0.5, (max - min).max(f64::EPSILON)))
    }

    pub(crate) fn visible_price_range(&self) -> Option<(f64, f64)> {
        if !self.manual_view_override {
            return None;
        }
        self.camera.explicit_price_range()
    }

    pub(crate) fn sync_camera_to_legacy(&mut self) {
        let (natural_center, natural_span) =
            self.natural_visible_price_view().unwrap_or((0.0, 1.0));
        self.camera.sync_legacy_fields(
            self.bars.len(),
            &mut self.visible_bars,
            &mut self.view_offset,
            &mut self.manual_view_override,
            &mut self.price_pan,
            &mut self.price_zoom,
            natural_center,
            natural_span,
        );
    }

    pub(crate) fn reset_camera_from_legacy(&mut self) {
        self.camera = ChartCamera::from_legacy(
            self.view_offset,
            self.visible_bars,
            self.manual_view_override,
        );
        if let Some((natural_center, natural_span)) = self.natural_visible_price_view() {
            let visible_span = natural_span / self.price_zoom.max(0.1);
            self.camera
                .set_price_view(natural_center + self.price_pan, visible_span);
        }
    }

    pub(crate) fn begin_chart_camera_pan(&mut self, rect_width: f32, rect_height: f32) {
        // Do not rebuild the camera from rounded legacy fields once manual
        // free-look is active. `view_offset` is integer compatibility state;
        // `ChartCamera` is the authoritative fractional bar/price camera.
        // Reconstructing from legacy at every drag start caused the visible
        // snap-back between recenter gestures.
        if !self.manual_view_override {
            self.reset_camera_from_legacy();
        }
        let (natural_center, natural_span) =
            self.natural_visible_price_view().unwrap_or((0.0, 1.0));
        self.camera
            .begin_pan(rect_width, rect_height, natural_center, natural_span);
        self.sync_camera_to_legacy();
        self.mark_view_changed();
    }

    pub(crate) fn pan_chart_camera_pixels(
        &mut self,
        delta: egui::Vec2,
        rect_width: f32,
        rect_height: f32,
    ) {
        let (natural_center, natural_span) =
            self.natural_visible_price_view().unwrap_or((0.0, 1.0));
        self.camera.pan_pixels(
            delta.x,
            delta.y,
            rect_width,
            rect_height,
            self.bars.len(),
            natural_center,
            natural_span,
        );
        self.sync_camera_to_legacy();
        self.mark_view_changed();
    }

    pub(crate) fn zoom_chart_price_by(&mut self, factor: f64) {
        let (natural_center, natural_span) =
            self.natural_visible_price_view().unwrap_or((0.0, 1.0));
        self.camera
            .zoom_price_by(factor, natural_center, natural_span);
        self.sync_camera_to_legacy();
        self.mark_view_changed();
    }

    pub(crate) fn zoom_chart_bars_by(&mut self, factor: f64) {
        self.camera.zoom_bars_by(factor, self.bars.len());
        self.sync_camera_to_legacy();
        self.mark_view_changed();
    }

    pub(crate) fn mark_view_changed(&mut self) {
        // Camera movement changes pixels even when no new bars arrive. The
        // renderer's live-WS early-out keys off `visible_bars_gen`; without
        // invalidating it, drag frames can reuse the old picture and look like
        // rubber-banding/snap-back.
        self.visible_bars_gen = self.visible_bars_gen.wrapping_add(1);
    }

    pub(crate) fn visible_range(&self) -> (usize, usize) {
        let (start, end, _, _) = self.visible_slot_window();
        (start, end)
    }

    pub(crate) fn visible_slot_window(&self) -> (usize, usize, f32, usize) {
        if self.bars.is_empty() {
            return (0, 0, 0.0, self.visible_bars.max(1));
        }
        let slot_count = self.visible_bars.max(1);
        let right_edge = if self.manual_view_override {
            self.camera.right_edge_bar()
        } else {
            self.view_offset as f64
        };
        let virtual_start = right_edge - slot_count as f64 + 1.0;
        let virtual_end_exclusive = right_edge + 1.0;
        let data_len = self.bars.len() as f64;
        let start = virtual_start.ceil().clamp(0.0, data_len) as usize;
        let mut end = virtual_end_exclusive.ceil().clamp(0.0, data_len) as usize;
        if let Some(cap) = self.replay_bar_cap {
            end = end.min(cap);
        }
        let start = start.min(end);
        let first_slot = ((start as f64 - virtual_start).max(0.0)) as f32;
        (start, end, first_slot, slot_count)
    }
}
