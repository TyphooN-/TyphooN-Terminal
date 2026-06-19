use super::*;

// ─── Ichimoku data ───────────────────────────────────────────────────────────

pub(crate) const ICHI_TENKAN: egui::Color32 = egui::Color32::from_rgb(0, 180, 230);
pub(crate) const ICHI_KIJUN: egui::Color32 = egui::Color32::from_rgb(200, 50, 50);
pub(crate) const ICHI_SPAN_A: egui::Color32 = egui::Color32::from_rgb(80, 200, 80);
pub(crate) const ICHI_SPAN_B: egui::Color32 = egui::Color32::from_rgb(200, 80, 80);
pub(crate) const ICHI_CLOUD_BULL: egui::Color32 =
    egui::Color32::from_rgba_premultiplied(80, 200, 80, 20);
pub(crate) const ICHI_CLOUD_BEAR: egui::Color32 =
    egui::Color32::from_rgba_premultiplied(200, 80, 80, 20);

pub(crate) const STOCH_K_COL: egui::Color32 = egui::Color32::from_rgb(100, 180, 255);
pub(crate) const STOCH_D_COL: egui::Color32 = egui::Color32::from_rgb(255, 130, 60);
pub(crate) const ADX_COL: egui::Color32 = egui::Color32::from_rgb(200, 180, 60);
pub(crate) const DI_PLUS_COL: egui::Color32 = egui::Color32::from_rgb(0, 200, 100);
pub(crate) const DI_MINUS_COL: egui::Color32 = egui::Color32::from_rgb(200, 50, 50);
pub(crate) const WMA_COL: egui::Color32 = egui::Color32::from_rgb(180, 100, 200);
pub(crate) const HMA_COL: egui::Color32 = egui::Color32::from_rgb(0, 200, 200);
pub(crate) const CCI_COL: egui::Color32 = egui::Color32::from_rgb(200, 140, 80);
pub(crate) const WILLR_COL: egui::Color32 = egui::Color32::from_rgb(180, 80, 200);
pub(crate) const OBV_COL: egui::Color32 = egui::Color32::from_rgb(100, 200, 160);
pub(crate) const MFI_COL: egui::Color32 = egui::Color32::from_rgb(110, 210, 130);
pub(crate) const TRIX_LINE_COL: egui::Color32 = egui::Color32::from_rgb(255, 130, 180);
pub(crate) const TRIX_SIG_COL: egui::Color32 = egui::Color32::from_rgb(110, 210, 255);
pub(crate) const PPO_LINE_COL: egui::Color32 = egui::Color32::from_rgb(180, 140, 255);
pub(crate) const PPO_SIG_COL: egui::Color32 = egui::Color32::from_rgb(255, 185, 90);
pub(crate) const ULTOSC_COL: egui::Color32 = egui::Color32::from_rgb(255, 205, 110);
pub(crate) const SAR_COL: egui::Color32 = egui::Color32::from_rgb(255, 200, 0);
pub(crate) const EHLERS_SS_COL: egui::Color32 = egui::Color32::from_rgb(0, 220, 220);
pub(crate) const EHLERS_DEC_COL: egui::Color32 = egui::Color32::from_rgb(220, 160, 0);
pub(crate) const EHLERS_ITL_COL: egui::Color32 = egui::Color32::from_rgb(180, 220, 0);
pub(crate) const EHLERS_MAMA_COL: egui::Color32 = egui::Color32::from_rgb(255, 100, 200);
pub(crate) const EHLERS_FAMA_COL: egui::Color32 = egui::Color32::from_rgb(100, 200, 255);
pub(crate) const EHLERS_EBSW_COL: egui::Color32 = egui::Color32::from_rgb(0, 200, 180);
pub(crate) const EHLERS_CYBER_COL: egui::Color32 = egui::Color32::from_rgb(200, 100, 255);
pub(crate) const EHLERS_CG_COL: egui::Color32 = egui::Color32::from_rgb(255, 180, 0);
pub(crate) const EHLERS_ROOF_COL: egui::Color32 = egui::Color32::from_rgb(100, 255, 100);
// BetterVolume colors — exact MT5 BetterVolume.mqh values
pub(crate) const BVOL_CLIMAX_UP: egui::Color32 = egui::Color32::from_rgb(255, 0, 0); // clrRed — bullish climax
pub(crate) const BVOL_CLIMAX_DN: egui::Color32 = egui::Color32::from_rgb(255, 255, 255); // clrWhite — bearish climax
pub(crate) const BVOL_HIGH: egui::Color32 = egui::Color32::from_rgb(0, 255, 0); // clrGreen — churn (high vol, low move)
pub(crate) const BVOL_LOW: egui::Color32 = egui::Color32::from_rgb(255, 255, 0); // clrYellow — low volume
pub(crate) const BVOL_CHURN: egui::Color32 = egui::Color32::from_rgb(255, 0, 255); // clrMagenta — climax + churn
pub(crate) const BVOL_NORMAL: egui::Color32 = egui::Color32::from_rgb(70, 130, 180); // clrSteelBlue — normal volume

/// Right margin: empty bars of space after the last candle (MT5 "chart shift" feature).
pub(crate) const CHART_RIGHT_MARGIN: usize = 5;
pub(crate) const CHART_SUB_PANE_H: f32 = 80.0;
pub(crate) const CHART_MIN_MAIN_CHART_H: f32 = 140.0;
pub(crate) const CHART_TIME_AXIS_H: f32 = 22.0;

pub(crate) fn chart_price_pane_height(total_chart_height: f32, sub_pane_count: u8) -> f32 {
    let sub_pane_height = if sub_pane_count > 0 {
        (CHART_SUB_PANE_H * sub_pane_count as f32)
            .min((total_chart_height - CHART_MIN_MAIN_CHART_H).max(0.0))
    } else {
        0.0
    };
    (total_chart_height - sub_pane_height - CHART_TIME_AXIS_H).max(1.0)
}

#[derive(Clone, Debug)]
pub(crate) struct ChartCamera {
    pub(crate) center_bar: f64,
    pub(crate) bars_visible: f64,
    pub(crate) price_center: Option<f64>,
    pub(crate) price_span: Option<f64>,
    pub(crate) follow_latest: bool,
    pub(crate) pan_start_center_bar: f64,
    pub(crate) pan_start_price_center: Option<f64>,
    pub(crate) pan_start_price_span: Option<f64>,
}

impl ChartCamera {
    pub(crate) fn from_legacy(
        view_offset: usize,
        visible_bars: usize,
        manual_view_override: bool,
    ) -> Self {
        let bars_visible = visible_bars.max(1) as f64;
        let right_edge = view_offset as f64;
        let center_bar = right_edge - (bars_visible - 1.0) * 0.5;
        Self {
            center_bar,
            bars_visible,
            price_center: None,
            price_span: None,
            follow_latest: !manual_view_override,
            pan_start_center_bar: center_bar,
            pan_start_price_center: None,
            pan_start_price_span: None,
        }
    }

    pub(crate) fn right_edge_bar(&self) -> f64 {
        self.center_bar + (self.bars_visible - 1.0) * 0.5
    }

    pub(crate) fn manual_override(&self) -> bool {
        !self.follow_latest
    }

    pub(crate) fn follow_latest_right_edge(data_len: usize) -> f64 {
        if data_len == 0 {
            0.0
        } else {
            data_len.saturating_sub(1) as f64 + CHART_RIGHT_MARGIN as f64
        }
    }

    pub(crate) fn max_right_edge(&self, data_len: usize) -> f64 {
        if data_len == 0 {
            0.0
        } else {
            // TradingView-style horizontal free-look: allow one full viewport of
            // empty space to the right so the newest bar can be dragged all the
            // way to the left edge. The left bound stays at right_edge=0, which
            // puts the oldest bar at the right edge with empty space to its left.
            data_len.saturating_sub(1) as f64
                + (self.bars_visible - 1.0).max(CHART_RIGHT_MARGIN as f64)
        }
    }

    pub(crate) fn set_right_edge_bar(&mut self, right_edge: f64, data_len: usize) {
        let max_right = self.max_right_edge(data_len);
        let clamped = right_edge.clamp(0.0, max_right);
        self.center_bar = clamped - (self.bars_visible - 1.0) * 0.5;
    }

    pub(crate) fn set_price_view(&mut self, center: f64, span: f64) {
        self.price_center = Some(center);
        self.price_span = Some(span.max(f64::EPSILON));
    }

    pub(crate) fn begin_pan(
        &mut self,
        _rect_width: f32,
        _rect_height: f32,
        natural_price_center: f64,
        natural_price_span: f64,
    ) {
        if self.price_center.is_none() || self.price_span.is_none() {
            self.set_price_view(natural_price_center, natural_price_span);
        }
        self.pan_start_center_bar = self.center_bar;
        self.pan_start_price_center = self.price_center;
        self.pan_start_price_span = self.price_span;
        self.follow_latest = false;
    }

    pub(crate) fn pan_pixels(
        &mut self,
        delta_x: f32,
        delta_y: f32,
        rect_width: f32,
        rect_height: f32,
        data_len: usize,
        natural_price_center: f64,
        natural_price_span: f64,
    ) {
        if data_len == 0 || rect_width <= 1.0 || rect_height <= 1.0 {
            return;
        }
        let bar_px = rect_width as f64 / self.bars_visible.max(1.0);
        let delta_bars = delta_x as f64 / bar_px;
        self.center_bar = self.pan_start_center_bar - delta_bars;
        self.set_right_edge_bar(self.right_edge_bar(), data_len);

        let start_center = self.pan_start_price_center.unwrap_or(natural_price_center);
        let span = self
            .pan_start_price_span
            .unwrap_or(natural_price_span)
            .max(f64::EPSILON);
        self.price_center = Some(start_center + delta_y as f64 * span / rect_height as f64);
        self.price_span = Some(span);
        self.follow_latest = false;
    }

    pub(crate) fn zoom_price_by(
        &mut self,
        factor: f64,
        natural_price_center: f64,
        natural_price_span: f64,
    ) {
        let factor = factor.clamp(0.01, 100.0);
        if self.price_center.is_none() || self.price_span.is_none() {
            self.set_price_view(natural_price_center, natural_price_span);
        }
        let center = self.price_center.unwrap_or(natural_price_center);
        let span = self
            .price_span
            .unwrap_or(natural_price_span)
            .max(f64::EPSILON);
        // factor > 1.0 means zoom in / compress the visible price range.
        self.set_price_view(center, span / factor);
        self.follow_latest = false;
    }

    pub(crate) fn zoom_bars_by(&mut self, factor: f64, data_len: usize) {
        if data_len == 0 {
            return;
        }
        let old_right_edge = self.right_edge_bar();
        self.bars_visible = (self.bars_visible * factor).clamp(10.0, data_len.max(10) as f64);
        self.set_right_edge_bar(old_right_edge, data_len);
        self.follow_latest = false;
    }

    pub(crate) fn on_data_len_changed(&mut self, old_len: usize, new_len: usize) {
        if new_len == 0 {
            self.set_right_edge_bar(0.0, 0);
            return;
        }
        if self.follow_latest || old_len == 0 {
            self.set_right_edge_bar(Self::follow_latest_right_edge(new_len), new_len);
            return;
        }

        // Manual free-look is an absolute camera, not "distance from the live
        // right edge". Preserving distance from max_right_edge made every live
        // bar/cache reload nudge the viewport toward latest, which looked like
        // TradingView-style body drag was snapping back under active feeds.
        // Keep the user's recentered bar position fixed and only clamp if the
        // new dataset actually invalidates that position.
        let right_edge = self.right_edge_bar();
        self.set_right_edge_bar(right_edge, new_len);
    }

    pub(crate) fn sync_legacy_fields(
        &self,
        data_len: usize,
        visible_bars: &mut usize,
        view_offset: &mut usize,
        manual_view_override: &mut bool,
        price_pan: &mut f64,
        price_zoom: &mut f64,
        natural_price_center: f64,
        natural_price_span: f64,
    ) {
        *visible_bars = self.bars_visible.round().max(1.0) as usize;
        *view_offset = self
            .right_edge_bar()
            .round()
            .clamp(0.0, self.max_right_edge(data_len)) as usize;
        *manual_view_override = self.manual_override();
        if let (Some(center), Some(span)) = (self.price_center, self.price_span) {
            *price_pan = center - natural_price_center;
            // Compatibility only. Rendering consumes the camera's explicit span,
            // so do not let this legacy clamp define the actual free-look range.
            *price_zoom = natural_price_span / span.max(f64::EPSILON);
        }
    }

    pub(crate) fn explicit_price_range(&self) -> Option<(f64, f64)> {
        let center = self.price_center?;
        let span = self.price_span?.max(f64::EPSILON);
        Some((center - span * 0.5, center + span * 0.5))
    }
}

/// Indicator visibility flags passed to draw_chart.
pub(crate) struct IndicatorFlags {
    pub(crate) sma200: bool,
    pub(crate) sma100: bool,
    pub(crate) kama: bool,
    pub(crate) ema21: bool,
    pub(crate) bollinger: bool,
    pub(crate) ichimoku: bool,
    pub(crate) wma: bool,
    pub(crate) hma: bool,
    pub(crate) psar: bool,
    pub(crate) atr_proj: bool,
    pub(crate) prev_levels: bool,
    pub(crate) pivots: bool,
    pub(crate) fractals: bool,
    pub(crate) harmonics: bool,
    pub(crate) auto_fib: bool,
    pub(crate) supply_demand: bool,
    pub(crate) ehlers_ss: bool,
    pub(crate) ehlers_decycler: bool,
    pub(crate) ehlers_itl: bool,
    pub(crate) ehlers_mama: bool,
    pub(crate) sessions: bool,
    pub(crate) vol_heatmap: bool,
    pub(crate) vwap: bool,
    pub(crate) price_histogram: bool,
    pub(crate) supertrend: bool,
    pub(crate) donchian: bool,
    pub(crate) keltner: bool,
    pub(crate) regression: bool,
    pub(crate) fvg: bool,
    pub(crate) order_blocks: bool,
}
