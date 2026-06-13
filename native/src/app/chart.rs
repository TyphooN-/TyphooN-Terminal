//! Extracted from app.rs: chart helpers.

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

/// All state for one chart viewport.
pub(crate) struct ChartState {
    /// The symbol string shown in the toolbar.
    pub(crate) symbol: String,
    /// Currently selected timeframe.
    pub(crate) timeframe: Timeframe,
    /// Cache source that most recently populated `bars` ("kraken", "alpaca", etc).
    pub(crate) primary_source: &'static str,
    /// User-forced chart source for debugging provider-specific cache rows.
    /// Empty means automatic source selection.
    pub(crate) source_override: &'static str,
    /// Active symbol-level regulatory warnings shown next to the chart symbol.
    pub(crate) regulatory_alerts: Vec<typhoon_engine::core::regulatory_alerts::RegulatoryAlert>,
    /// Chart rendering style.
    pub(crate) chart_type: ChartType,
    /// Logarithmic price scale (vs linear).
    pub(crate) log_scale: bool,
    // ── Configurable indicator periods ─────────────────────────────────
    pub(crate) sma_slow_period: u32,
    pub(crate) sma_fast_period: u32,
    pub(crate) ema_period: u32,
    pub(crate) rsi_period: u32,
    pub(crate) atr_period: u32,
    pub(crate) bb_period: u32,
    pub(crate) macd_fast: u32,
    pub(crate) macd_slow: u32,
    pub(crate) macd_signal_p: u32,
    pub(crate) stoch_period: u32,
    pub(crate) adx_period: u32,
    pub(crate) fisher_period: u32,
    pub(crate) momentum_period: u32,
    // ── Compare symbol overlay ─────────────────────────────────────────
    pub(crate) compare_symbol: Option<String>,
    pub(crate) compare_bars: Vec<Bar>,
    /// Live bid/ask from streaming quotes (for spread line rendering).
    pub(crate) live_bid: f64,
    pub(crate) live_ask: f64,
    /// When `live_bid`/`live_ask` were last refreshed from a streaming quote.
    /// The spread lines are hidden once this goes stale so a frozen quote isn't
    /// drawn next to a live (differently-priced) last/candle.
    pub(crate) live_quote_at: Option<std::time::Instant>,
    /// `true` when the most recent `live_bid`/`live_ask` came from a *delayed*
    /// source (the Kraken iapi equity ticker, ~15 min). Lets a real-time WS quote
    /// take precedence so the chart spread/last don't flip to the delayed snapshot
    /// while live ticks are flowing.
    pub(crate) live_quote_delayed: bool,
    // Extended hours candle (pre/post market)
    pub(crate) ext_open: f64,
    pub(crate) ext_high: f64,
    pub(crate) ext_low: f64,
    pub(crate) ext_close: f64,
    pub(crate) ext_active: bool, // true when ext hours data is available
    /// Raw bar data loaded from cache.
    pub(crate) bars: Vec<Bar>,
    /// Reusable buffers for full GPU upload path (avoids repeated allocations).
    #[allow(dead_code)]
    pub(crate) upload_opens: Vec<f32>,
    #[allow(dead_code)]
    pub(crate) upload_closes: Vec<f32>,
    #[allow(dead_code)]
    pub(crate) upload_highs: Vec<f32>,
    #[allow(dead_code)]
    pub(crate) upload_lows: Vec<f32>,
    #[allow(dead_code)]
    pub(crate) upload_volumes: Vec<f32>,
    /// Pre-computed SMA(200) — indexed parallel to `bars`.
    pub(crate) sma200: Vec<Option<f64>>,
    /// Pre-computed SMA(100) — indexed parallel to `bars`.
    pub(crate) sma100: Vec<Option<f64>>,
    /// Pre-computed KAMA(10,2,30) — indexed parallel to `bars`.
    pub(crate) kama: Vec<Option<f64>>,
    /// Pre-computed EMA(21).
    pub(crate) ema21: Vec<Option<f64>>,
    /// Bollinger Bands (middle, upper, lower).
    pub(crate) bb_mid: Vec<Option<f64>>,
    pub(crate) bb_upper: Vec<Option<f64>>,
    pub(crate) bb_lower: Vec<Option<f64>>,
    /// RSI(14) — 0..100 range.
    pub(crate) rsi: Vec<Option<f64>>,
    /// Fisher Transform.
    pub(crate) fisher: Vec<Option<f64>>,
    pub(crate) fisher_signal: Vec<Option<f64>>,
    /// ATR(14).
    pub(crate) atr: Vec<Option<f64>>,
    /// MACD(12,26,9).
    pub(crate) macd_line: Vec<Option<f64>>,
    pub(crate) macd_signal: Vec<Option<f64>>,
    pub(crate) macd_hist: Vec<Option<f64>>,
    /// Stochastic(14,3,3).
    pub(crate) stoch_k: Vec<Option<f64>>,
    pub(crate) stoch_d: Vec<Option<f64>>,
    /// ADX(14) + DI+/DI-.
    pub(crate) adx: Vec<Option<f64>>,
    pub(crate) di_plus: Vec<Option<f64>>,
    pub(crate) di_minus: Vec<Option<f64>>,
    /// Ichimoku(9,26,52).
    pub(crate) ichi_tenkan: Vec<Option<f64>>,
    pub(crate) ichi_kijun: Vec<Option<f64>>,
    pub(crate) ichi_span_a: Vec<Option<f64>>,
    pub(crate) ichi_span_b: Vec<Option<f64>>,
    /// Previous candle levels (daily high/low).
    pub(crate) prev_daily_high: Option<f64>,
    pub(crate) prev_daily_low: Option<f64>,
    pub(crate) prev_weekly_high: Option<f64>,
    pub(crate) prev_weekly_low: Option<f64>,
    pub(crate) prev_h4_high: Option<f64>,
    pub(crate) prev_h4_low: Option<f64>,
    pub(crate) prev_h1_high: Option<f64>,
    pub(crate) prev_h1_low: Option<f64>,
    pub(crate) prev_monthly_high: Option<f64>,
    pub(crate) prev_monthly_low: Option<f64>,
    /// WMA(20), HMA(20).
    pub(crate) wma: Vec<Option<f64>>,
    pub(crate) hma: Vec<Option<f64>>,
    /// CCI(20), Williams %R(14).
    pub(crate) cci: Vec<Option<f64>>,
    pub(crate) williams_r: Vec<Option<f64>>,
    /// OBV.
    pub(crate) obv: Vec<Option<f64>>,
    /// Momentum(10).
    pub(crate) momentum: Vec<Option<f64>>,
    /// CMO(9).
    pub(crate) cmo: Vec<Option<f64>>,
    /// Running sums for O(1) CMO forming-bar update
    pub(crate) cmo_sum_up: f64,
    pub(crate) cmo_sum_down: f64,
    /// Running sums for O(1) Linear Regression Slope
    pub(crate) linreg_sum_x: f64,
    pub(crate) linreg_sum_y: f64,
    pub(crate) linreg_sum_xy: f64,
    pub(crate) linreg_sum_x2: f64,
    /// QStick(14).
    pub(crate) qstick: Vec<Option<f64>>,
    /// Disparity Index(14).
    pub(crate) disparity: Vec<Option<f64>>,
    /// LINEARREG_SLOPE
    pub(crate) linreg_slope: Vec<Option<f64>>,
    /// LINEARREG_INTERCEPT
    pub(crate) linreg_intercept: Vec<Option<f64>>,
    /// LINEARREG_ANGLE
    #[allow(dead_code)]
    pub(crate) linreg_angle: Vec<Option<f64>>,
    /// LINEARREG (endpoint value)
    #[allow(dead_code)]
    pub(crate) linreg: Vec<Option<f64>>,
    /// BOP(14).
    pub(crate) bop: Vec<Option<f64>>,
    /// StdDev(20).
    pub(crate) stddev: Vec<Option<f64>>,
    /// MFI(14).
    pub(crate) mfi: Vec<Option<f64>>,
    /// TRIX(15,9).
    pub(crate) trix_line: Vec<Option<f64>>,
    pub(crate) trix_signal: Vec<Option<f64>>,
    pub(crate) trix_hist: Vec<Option<f64>>,
    /// PPO(12,26,9).
    pub(crate) ppo_line: Vec<Option<f64>>,
    pub(crate) ppo_signal: Vec<Option<f64>>,
    pub(crate) ppo_hist: Vec<Option<f64>>,
    /// Ultimate Oscillator(7,14,28).
    pub(crate) ultosc: Vec<Option<f64>>,
    /// StochRSI(14,14,3,3).
    pub(crate) stochrsi_k: Vec<Option<f64>>,
    pub(crate) stochrsi_d: Vec<Option<f64>>,
    /// VaR oscillator (20-bar rolling parametric VaR, 95%).
    pub(crate) var_oscillator: Vec<Option<f64>>,
    /// Parabolic SAR(0.02, 0.2).
    pub(crate) psar: Vec<Option<f64>>,
    /// MTF SMA lines — (label, color_idx, projected_points) matching MTF_MA.mqh.
    /// H1/200, H4/200, D1/200, W1/200, W1/100, MN1/100
    pub(crate) mtf_sma: Vec<(String, Vec<(usize, f64)>)>,
    /// ATR Projection (open ± ATR bands — legacy per-bar, kept for GPU path).
    pub(crate) atr_proj_upper: Vec<Option<f64>>,
    pub(crate) atr_proj_lower: Vec<Option<f64>>,
    /// ATR Projection MTF levels (label, open, atr_value, start_bar_idx) — matches ATR_Projection.mqh.
    pub(crate) atr_proj_levels: Vec<(&'static str, f64, f64, usize)>,
    /// Better Volume classification.
    pub(crate) better_vol_type: Vec<u8>, // 0=normal, 1=climax_up, 2=climax_dn, 3=high, 4=low, 5=churn
    /// Pivot points (computed from daily data).
    pub(crate) pivot_p: Option<f64>,
    pub(crate) pivot_r1: Option<f64>,
    pub(crate) pivot_r2: Option<f64>,
    pub(crate) pivot_s1: Option<f64>,
    pub(crate) pivot_s2: Option<f64>,
    /// Bill Williams Fractals (up/down arrows).
    pub(crate) fractal_up: Vec<bool>,
    pub(crate) fractal_down: Vec<bool>,
    // ── Ehlers indicators ──────────────────────────────────────────────
    /// Super Smoother (overlay).
    pub(crate) ehlers_ss: Vec<Option<f64>>,
    /// Decycler (overlay).
    pub(crate) ehlers_decycler: Vec<Option<f64>>,
    /// Instantaneous Trendline (overlay).
    pub(crate) ehlers_itl: Vec<Option<f64>>,
    /// MAMA (overlay).
    pub(crate) ehlers_mama: Vec<Option<f64>>,
    /// FAMA (overlay).
    pub(crate) ehlers_fama: Vec<Option<f64>>,
    /// Even Better Sinewave (sub-pane, -1 to 1).
    pub(crate) ehlers_ebsw: Vec<Option<f64>>,
    /// Cyber Cycle (sub-pane).
    pub(crate) ehlers_cyber: Vec<Option<f64>>,
    /// CG Oscillator (sub-pane).
    pub(crate) ehlers_cg: Vec<Option<f64>>,
    /// Roofing Filter (sub-pane).
    pub(crate) ehlers_roof: Vec<Option<f64>>,
    /// Supply/demand zones: (bar_idx, zone_high, zone_low, status).
    /// Status: 0=untested, 1=tested (price returned), 2=proven (price bounced)
    pub(crate) supply_zones: Vec<(usize, f64, f64, u8)>,
    /// Earliest timestamp from primary data source (Kraken/Alpaca). Bars before this are backfill.
    pub(crate) primary_first_ts: i64,
    /// Timestamps of bars sourced from gap-fill (Kraken). Colored magenta.
    pub(crate) gap_fill_timestamps: std::collections::HashSet<i64>,
    pub(crate) demand_zones: Vec<(usize, f64, f64, u8)>,
    /// Auto Fibonacci levels: (price, label, is_extension).
    pub(crate) auto_fib_levels: Vec<(f64, String, bool)>,
    /// Auto Fibonacci swing: (swing_high_price, swing_low_price, swing_high_idx, swing_low_idx).
    pub(crate) auto_fib_swing: Option<(f64, f64, usize, usize)>,
    /// VWAP (volume-weighted average price), anchored daily.
    pub(crate) vwap: Vec<Option<f64>>,
    /// VWAP upper deviation bands (1σ, 2σ, 3σ).
    pub(crate) vwap_upper1: Vec<Option<f64>>,
    pub(crate) vwap_upper2: Vec<Option<f64>>,
    pub(crate) vwap_upper3: Vec<Option<f64>>,
    /// VWAP lower deviation bands (1σ, 2σ, 3σ).
    pub(crate) vwap_lower1: Vec<Option<f64>>,
    pub(crate) vwap_lower2: Vec<Option<f64>>,
    pub(crate) vwap_lower3: Vec<Option<f64>>,
    /// Supertrend line (ATR-based trend bands, single line that flips).
    pub(crate) supertrend: Vec<Option<f64>>,
    /// Supertrend direction: true = bullish (below price), false = bearish (above).
    pub(crate) supertrend_bull: Vec<bool>,
    /// Donchian Channel upper (highest high over N bars).
    pub(crate) donchian_upper: Vec<Option<f64>>,
    /// Donchian Channel lower (lowest low over N bars).
    pub(crate) donchian_lower: Vec<Option<f64>>,
    /// Keltner Channel middle (EMA).
    pub(crate) keltner_mid: Vec<Option<f64>>,
    /// Keltner Channel upper (EMA + ATR×mult).
    pub(crate) keltner_upper: Vec<Option<f64>>,
    /// Keltner Channel lower (EMA - ATR×mult).
    pub(crate) keltner_lower: Vec<Option<f64>>,
    /// Regression Channel middle (regression line).
    pub(crate) regression_mid: Vec<Option<f64>>,
    /// Regression Channel upper (regression + 2σ).
    pub(crate) regression_upper: Vec<Option<f64>>,
    /// Regression Channel lower (regression - 2σ).
    pub(crate) regression_lower: Vec<Option<f64>>,
    /// Squeeze Momentum: momentum histogram value.
    pub(crate) squeeze_mom: Vec<Option<f64>>,
    /// Squeeze state: true = in squeeze (BB inside KC).
    pub(crate) squeeze_on: Vec<bool>,
    /// Pre-computed 20-bar rolling average volume (for volume heatmap candle coloring).
    pub(crate) vol_avg_20: Vec<f64>,
    /// MultiKAMA: KAMA values from higher timeframes projected onto this chart's x-axis.
    /// Each entry: (timeframe_label, Vec of (bar_index_in_this_chart, kama_value))
    pub(crate) multi_kama: Vec<(String, Vec<(usize, f64)>)>,
    /// Detected harmonic patterns.
    pub(super) harmonics: Vec<HarmonicPattern>,
    /// Drawing annotations.
    pub(crate) drawings: Vec<Drawing>,
    /// Per-drawing style: (line_width, line_style). Indexed parallel to `drawings`.
    pub(crate) drawing_styles: Vec<(f32, LineStyle)>,
    /// Undo stack for drawings (Ctrl+Z pops from drawings into here, Ctrl+Shift+Z restores)
    pub(crate) drawings_undo: Vec<Drawing>,
    pub(crate) selected_drawing: Option<usize>,
    /// Index of the control point being dragged (for resize). None = whole-drawing drag.
    pub(crate) dragging_cp: Option<usize>,
    /// True while dragging a selected drawing (suppresses chart pan).
    pub(crate) is_drawing_drag: bool,
    /// Cached trade overlay (rebuilt when bg data changes, not every frame).
    pub(crate) cached_trade_overlay: TradeOverlay,
    pub(crate) cached_trade_overlay_frame: u64,

    // ── view state ────────────────────────────────────────────────────────
    /// How many bars are visible horizontally (zoom level).
    pub(crate) visible_bars: usize,
    /// Index of the right-most visible bar (0 = oldest, len-1 = newest).
    pub(crate) view_offset: usize,
    /// True after the user manually pans/zooms the chart away from the auto-follow view.
    /// Cache reloads must preserve this viewport instead of snapping back to latest.
    pub(crate) manual_view_override: bool,
    /// Canonical TradingView-style camera. Legacy viewport fields above are kept
    /// in sync for draw/overlay code until the renderer consumes the camera directly.
    pub(crate) camera: ChartCamera,
    /// When replay mode is active, cap visible_range end at this bar index.
    pub(crate) replay_bar_cap: Option<usize>,
    /// Fractional price offset for vertical pan.
    pub(crate) price_pan: f64,
    /// Multiplier applied to the natural price range for vertical zoom.
    pub(crate) price_zoom: f64,

    // ── interaction helpers ───────────────────────────────────────────────
    pub(crate) is_dragging: bool,
    pub(crate) drag_start: Option<egui::Pos2>,
    /// True when dragging on the price axis (TradingView-style vertical scale).
    pub(crate) is_scaling_price: bool,
    /// Price zoom at start of price-axis drag.
    pub(crate) scale_start_zoom: f64,
    /// Y position at start of price-axis drag.
    pub(crate) scale_start_y: f32,

    // ── Live WS performance fields (added 2026-05) ──────────────────────
    /// Incremented whenever the visible closed bars change (not forming bar).
    /// Used by draw_chart for O(1) early-out when data is stable.
    pub(crate) visible_bars_gen: u64,
    /// Set by Kraken WS path when only the last bar was updated.
    /// Allows skipping full indicator recompute on forming-bar ticks.
    pub(crate) forming_bar_dirty: bool,
    /// Propagated from TyphooNApp during heavy backfill/sync so overlay panes can early-out.
    pub(crate) heavy_sync_in_progress: bool,
    /// Timestamp of the right-most bar (used for fast-path decisions).
    pub(crate) last_visible_bar_ts: i64,
    pub(crate) last_rendered_gen: u64,
    pub(crate) last_rendered_bar_ts: i64,
}

/// Extract the bare symbol from a cache key. Accepts canonical 3-part
/// `source:SYM:TF`, legacy 2-part `SYM:TF`, or bare `SYM`. Used by load
/// paths that need to put a canonical symbol into `ChartState::symbol`
/// so `try_load` and the chart header agree on its shape.
pub(crate) fn bare_symbol_from_key(key: &str) -> String {
    let parts: Vec<&str> = key.split(':').collect();
    match parts.as_slice() {
        [_src, sym, _tf] => (*sym).to_string(),
        [sym, _tf] => (*sym).to_string(),
        _ => key.to_string(),
    }
}

pub(crate) fn normalize_market_data_symbol(symbol: &str) -> String {
    let bare = bare_symbol_from_key(symbol).to_uppercase();
    match bare.rsplit_once('.') {
        Some((head, suffix))
            if (2..=4).contains(&suffix.len())
                && suffix.chars().all(|c| c.is_ascii_uppercase()) =>
        {
            head.to_string()
        }
        _ => bare,
    }
}

pub(crate) fn kraken_pair_source<'a>(pair_name: &'a str, display_name: &'a str) -> &'a str {
    if display_name.trim().is_empty() {
        pair_name
    } else {
        display_name
    }
}

pub(crate) fn kraken_pair_base_quote(
    pair_name: &str,
    display_name: &str,
) -> Option<(String, String)> {
    let source = kraken_pair_source(pair_name, display_name);
    if let Some((base, quote)) = source.split_once('/') {
        let base = typhoon_engine::core::kraken::normalize_pair_symbol(base);
        let quote = typhoon_engine::core::kraken::normalize_pair_symbol(quote);
        if !base.is_empty() && !quote.is_empty() {
            return Some((base, quote));
        }
    }
    let symbol = typhoon_engine::core::kraken::normalize_pair_symbol(source);
    pub(crate) const QUOTES: [&str; 15] = [
        "USDG", "USDT", "USDC", "USD", "EUR", "GBP", "CAD", "AUD", "JPY", "CHF", "XBT", "BTC",
        "ETH", "SOL", "DAI",
    ];
    let quote = QUOTES
        .iter()
        .find(|quote| symbol.ends_with(**quote) && symbol.len() > quote.len())?;
    let base = symbol.strip_suffix(*quote)?;
    Some((base.to_string(), quote.to_string()))
}

pub(crate) fn kraken_pair_is_fiat_fx(pair_name: &str, display_name: &str) -> bool {
    let Some((base, quote)) = kraken_pair_base_quote(pair_name, display_name) else {
        return false;
    };
    pub(crate) const FIAT: [&str; 7] = ["USD", "EUR", "GBP", "CAD", "AUD", "JPY", "CHF"];
    FIAT.contains(&base.as_str()) && FIAT.contains(&quote.as_str())
}

pub(crate) fn kraken_pair_asset_class(pair_name: &str, display_name: &str) -> &'static str {
    if kraken_pair_is_fiat_fx(pair_name, display_name) {
        "fx"
    } else if kraken_xstock_fundamental_symbol(pair_name, display_name).is_some() {
        "xstock"
    } else {
        "crypto"
    }
}

pub(crate) fn kraken_xstock_fundamental_symbol(
    pair_name: &str,
    display_name: &str,
) -> Option<String> {
    let source = kraken_pair_source(pair_name, display_name);
    let symbol = typhoon_engine::core::kraken::normalize_pair_symbol(source);
    let (base, _quote) = kraken_pair_base_quote(pair_name, display_name)?;
    // Public AssetPairs currently exposes crypto + spot FX. Tokenized equity
    // holdings from private balances use `.EQ`; avoid treating ordinary crypto
    // tickers that end in `X` (AVAX, FLUX, CVX, etc.) as xStocks.
    let equity = base
        .strip_suffix(".EQ")
        .or_else(|| symbol.strip_suffix(".EQ"))?;
    if equity.is_empty()
        || matches!(equity, "XBT" | "BTC" | "XDG" | "DOGE")
        || !equity
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.')
    {
        return None;
    }
    Some(equity.to_string())
}

pub(crate) fn cache_source_from_key(key: &str) -> &'static str {
    if key.starts_with("alpaca:")
        || key.starts_with("paper_TyphooN:")
        || key.starts_with("alpaca_paper_TyphooN:")
    {
        "alpaca"
    } else if key.starts_with("kraken-equities:") {
        "kraken-equities"
    } else if key.starts_with("kraken-futures:") {
        "kraken-futures"
    } else if key.starts_with("kraken:") {
        "kraken"
    } else if key.starts_with("yahoo-chart:") {
        "yahoo-chart"
    } else if key.starts_with("merged:") {
        "merged"
    } else if key.starts_with("default:") {
        "default"
    } else {
        ""
    }
}

pub(crate) fn chart_source_bars_match_timeframe(
    source: &str,
    timeframe: &str,
    bars: &[(i64, f64, f64, f64, f64, f64)],
) -> bool {
    if timeframe == "1Month" && matches!(source, "kraken" | "kraken-equities" | "kraken-futures") {
        return false;
    }
    if bars.len() < 20 {
        return true;
    }
    let Some((min_delta_ms, max_median_delta_ms)) = chart_timeframe_cadence_bounds(timeframe)
    else {
        return true;
    };
    let mut timestamps: Vec<i64> = bars
        .iter()
        .map(|(ts, _, _, _, _, _)| *ts)
        .filter(|ts| *ts > 0)
        .collect();
    timestamps.sort_unstable();
    timestamps.dedup();
    if timestamps.len() < 20 {
        return true;
    }
    let mut deltas: Vec<i64> = timestamps
        .windows(2)
        .filter_map(|w| w[1].checked_sub(w[0]))
        .filter(|delta| *delta > 0)
        .collect();
    if deltas.len() < 10 {
        return true;
    }
    deltas.sort_unstable();
    let median = deltas[deltas.len() / 2];
    median >= min_delta_ms && median <= max_median_delta_ms
}

pub(crate) fn chart_timeframe_cadence_bounds(timeframe: &str) -> Option<(i64, i64)> {
    let hour = 3_600_000i64;
    let day = 24 * hour;
    match timeframe {
        "1Min" => Some((30_000, 5 * 60_000)),
        "5Min" => Some((2 * 60_000, 20 * 60_000)),
        "15Min" => Some((5 * 60_000, 60 * 60_000)),
        "30Min" => Some((10 * 60_000, 2 * hour)),
        "1Hour" => Some((20 * 60_000, 4 * hour)),
        "4Hour" => Some((hour, 16 * hour)),
        "1Day" => Some((12 * hour, 5 * day)),
        "1Week" => Some((5 * day, 8 * day)),
        "1Month" => Some((26 * day, 35 * day)),
        _ => None,
    }
}

pub(crate) fn chart_gap_fill_bar_allowed(
    primary_source: &str,
    gap_source: &str,
    snapped: i64,
    primary_min_snapped: Option<i64>,
    primary_max_snapped: Option<i64>,
) -> bool {
    if !matches!(
        primary_source,
        "kraken-equities" | "tastytrade" | "alpaca" | "yahoo-chart"
    ) || !matches!(gap_source, "alpaca" | "yahoo-chart")
    {
        return true;
    }

    match (primary_min_snapped, primary_max_snapped) {
        (Some(min), Some(max)) => snapped < min || snapped > max,
        _ => true,
    }
}

#[allow(dead_code)]
pub(crate) fn chart_quote_overlay_allowed(quote_ts_ms: i64, last_bar_ts_ms: i64) -> bool {
    quote_ts_ms >= last_bar_ts_ms
}

pub(crate) fn chart_bar_last_valid_ts(raw: &[(i64, f64, f64, f64, f64, f64)]) -> i64 {
    raw.iter()
        .rev()
        .find_map(|(ts, _o, _h, _l, close, _v)| {
            (*ts > 0 && *close > 0.0 && close.is_finite()).then_some(*ts)
        })
        .unwrap_or(0)
}

pub(crate) fn chart_merge_bucket_ts(timeframe: &str, ts: i64) -> i64 {
    match timeframe {
        "1Month" => chrono::DateTime::from_timestamp_millis(ts)
            .and_then(|dt| {
                chrono::NaiveDate::from_ymd_opt(dt.year(), dt.month(), 1)
                    .and_then(|d| d.and_hms_opt(0, 0, 0))
            })
            .map(|ndt| ndt.and_utc().timestamp_millis())
            .unwrap_or(ts),
        "1Week" => chrono::DateTime::from_timestamp_millis(ts)
            .and_then(|dt| {
                let days_since_mon = dt.weekday().num_days_from_monday() as i64;
                (dt.date_naive() - chrono::Duration::days(days_since_mon)).and_hms_opt(0, 0, 0)
            })
            .map(|ndt| ndt.and_utc().timestamp_millis())
            .unwrap_or(ts),
        "1Day" => chrono::DateTime::from_timestamp_millis(ts)
            .and_then(|dt| {
                chrono::NaiveDate::from_ymd_opt(dt.year(), dt.month(), dt.day())
                    .and_then(|d| d.and_hms_opt(0, 0, 0))
            })
            .map(|ndt| ndt.and_utc().timestamp_millis())
            .unwrap_or(ts),
        "4Hour" => ts / (4 * 3_600_000) * (4 * 3_600_000),
        "1Hour" => ts / 3_600_000 * 3_600_000,
        "30Min" => ts / 1_800_000 * 1_800_000,
        "15Min" => ts / 900_000 * 900_000,
        "5Min" => ts / 300_000 * 300_000,
        _ => ts / 60_000 * 60_000,
    }
}

/// Merge equity/ETF bars from multiple providers into one continuous series.
///
/// Sources split into two tiers by [`chart_equity_source_rank`]:
/// * **Trusted** (rank ≤ 2 — `kraken-equities`, `tastytrade`, `alpaca`): live /
///   native feeds that define the authoritative, corporate-action-adjusted
///   price scale. Merged per-bucket by priority (best rank wins; latest tick
///   wins within a source).
/// * **Depth** (rank ≥ 3 — `yahoo-chart`, `default`): history-extenders. Yahoo
///   carries far deeper history but is frequently *unadjusted* across splits and
///   token re-denominations (WOK was ~10,000× too high before its 2025 action),
///   which a naïve splice would paste straight onto the trusted scale as a
///   discontinuity. So each depth source is back-adjusted to the trusted scale
///   by the price ratio where they overlap, and only fills buckets the trusted
///   tier lacks (older history + interior gaps). A depth source whose overlap
///   scale is inconsistent — the tell-tale of an unadjusted action mid-history —
///   is dropped entirely rather than splicing scale-jumped bars.
///
/// With no trusted source present (Yahoo-only symbol) we fall back to per-bucket
/// priority across the depth sources so the symbol still charts (best effort).
/// A known stock split / reverse split: bars strictly before `ex_ts_ms` are
/// multiplied by `pre_split_factor` (= old shares / new shares = denominator /
/// numerator) to lift raw, unadjusted pre-split history onto the post-split price
/// scale. For a 1-for-100 reverse split (WOK, 2025-12-29) the factor is 100.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ChartSplit {
    pub ex_ts_ms: i64,
    pub pre_split_factor: f64,
}

/// Back-adjust an unadjusted source's buckets for known splits: each bar before a
/// split's ex-date is scaled by the cumulative product of all later splits' factors.
/// Exact and source-independent — unlike the cross-source era inference, it works
/// even when no adjusted reference (Alpaca) is present and across a single split era.
fn chart_back_adjust_bars_for_splits(
    bucketed: &mut std::collections::BTreeMap<i64, Bar>,
    splits: &[ChartSplit],
) {
    if splits.is_empty() {
        return;
    }
    for (ts, bar) in bucketed.iter_mut() {
        let mut factor = 1.0;
        for s in splits {
            if *ts < s.ex_ts_ms {
                factor *= s.pre_split_factor;
            }
        }
        if (factor - 1.0).abs() > 1e-9 {
            bar.open *= factor;
            bar.high *= factor;
            bar.low *= factor;
            bar.close *= factor;
        }
    }
}

/// Convert a stored FMP `StockSplit` into a `ChartSplit` (parse the ex-date, derive
/// the pre-split multiplier). Skips malformed/zero entries.
fn chart_split_from_stock_split(
    s: &typhoon_engine::core::research::StockSplit,
) -> Option<ChartSplit> {
    if s.numerator <= 0.0 || s.denominator <= 0.0 {
        return None;
    }
    let date = chrono::NaiveDate::parse_from_str(&s.date, "%Y-%m-%d").ok()?;
    let ex_ts_ms = date.and_hms_opt(0, 0, 0)?.and_utc().timestamp_millis();
    Some(ChartSplit {
        ex_ts_ms,
        pre_split_factor: s.denominator / s.numerator,
    })
}

/// Curated corporate actions for symbols where the FMP split feed is missing or
/// unreliable. Free-tier FMP omits many microcap reverse splits, and a node that
/// has never scraped / LAN-synced `research_stock_splits` has the table empty (or
/// absent) entirely — which starves the exact back-adjustment
/// ([`chart_back_adjust_bars_for_splits`]) of the split it needs, so raw
/// pre-split history (Kraken xStock bars) gets painted on the wrong scale. That
/// is the WOK December reverse-split discontinuity vs TradingView: the merge code
/// is correct and tested, it was simply never handed the split. These entries
/// supplement [`chart_known_splits_from_cache`] so the back-adjust still fires
/// offline / without an FMP key. See ADR-122.
///
/// `pre_split_factor` = old shares / new shares (= 100 for a 1-for-100 reverse
/// split). Dates are the split ex-date at 00:00 UTC. Verify each against the
/// issuer's actual action before adding.
pub(crate) fn chart_curated_known_splits(symbol: &str) -> Vec<ChartSplit> {
    // (symbol, ex-date "YYYY-MM-DD", pre_split_factor = denominator/numerator)
    const CURATED: &[(&str, &str, f64)] = &[
        // WORK Medical Technology Group — 1-for-100 reverse split.
        ("WOK", "2025-12-29", 100.0),
    ];
    let su = symbol.trim().to_ascii_uppercase();
    CURATED
        .iter()
        .filter(|(sym, _, _)| su == *sym)
        .filter_map(|(_, date, factor)| {
            let d = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
            Some(ChartSplit {
                ex_ts_ms: d.and_hms_opt(0, 0, 0)?.and_utc().timestamp_millis(),
                pre_split_factor: *factor,
            })
        })
        .collect()
}

/// Load known splits for `symbol` for use in equity-merge back-adjustment:
/// FMP-sourced rows from the research cache (`research_stock_splits`, read-only),
/// supplemented by [`chart_curated_known_splits`] for actions FMP/LAN-sync missed.
/// Empty only when neither source knows a split (the era-inference fallback then
/// applies). Curated entries are deduped against cached rows by ex-date so real
/// FMP data takes precedence when both are present.
fn chart_known_splits_from_cache(cache: &SqliteCache, symbol: &str) -> Vec<ChartSplit> {
    let mut splits: Vec<ChartSplit> = Vec::new();
    if let Ok(conn) = cache.read_connection() {
        if let Ok(Some(rows)) = typhoon_engine::core::research::get_stock_splits(&conn, symbol) {
            splits = rows
                .iter()
                .filter_map(chart_split_from_stock_split)
                .collect();
        }
    }
    for c in chart_curated_known_splits(symbol) {
        let dup = splits
            .iter()
            .any(|s| (s.ex_ts_ms - c.ex_ts_ms).abs() < 86_400_000);
        if !dup {
            splits.push(c);
        }
    }
    splits
}

pub(crate) fn chart_merge_equity_raw_bars(
    timeframe: &str,
    sources: &[(&str, &[(i64, f64, f64, f64, f64, f64)])],
    splits: &[ChartSplit],
) -> Vec<Bar> {
    use std::collections::BTreeMap;
    const TRUSTED_MAX_RANK: u8 = 2; // alpaca and better define the price scale

    // Validate + bucket each usable source, tagged by its priority rank.
    let mut tagged: Vec<(u8, BTreeMap<i64, Bar>)> = Vec::new();
    for (source, raw) in sources {
        let Some(rank) = chart_equity_source_rank(source) else {
            continue;
        };
        if !chart_source_bars_match_timeframe(source, timeframe, raw) {
            continue;
        }
        let mut bucketed = chart_bucket_valid_source_bars(timeframe, raw);
        // kraken-equities iapi returns RAW (unadjusted) xStock bars. Back-adjust by
        // KNOWN splits at their known ex-dates so pre-split history lands on the
        // post-split scale (matching Alpaca `adjustment=all` + TradingView), instead
        // of relying solely on the cross-source era inference below.
        if *source == "kraken-equities" {
            chart_back_adjust_bars_for_splits(&mut bucketed, splits);
        }
        if !bucketed.is_empty() {
            tagged.push((rank, bucketed));
        }
    }
    // Best priority first; stable so equal ranks keep input order.
    tagged.sort_by_key(|(rank, _)| *rank);

    // Trusted tier defines the scale: per-bucket, the best rank present wins.
    let mut merged: BTreeMap<i64, Bar> = BTreeMap::new();
    for (rank, bucketed) in &tagged {
        if *rank > TRUSTED_MAX_RANK {
            continue;
        }
        for (bucket, bar) in bucketed {
            merged.entry(*bucket).or_insert_with(|| bar.clone());
        }
    }

    let trusted_merge_is_stale = chart_trusted_equity_merge_is_stale(timeframe, &merged, &tagged);
    if trusted_merge_is_stale {
        merged.clear();
    }

    if merged.is_empty() {
        // No trusted reference — best-effort per-bucket priority over depth.
        for (rank, bucketed) in &tagged {
            if trusted_merge_is_stale && *rank <= TRUSTED_MAX_RANK {
                continue;
            }
            for (bucket, bar) in bucketed {
                merged.entry(*bucket).or_insert_with(|| bar.clone());
            }
        }
        return merged.into_values().collect();
    }

    // Trusted-tier split-adjustment reconciliation. The best-rank trusted source
    // (kraken-equities iapi) returns RAW xStock bars, while Alpaca returns
    // split-adjusted bars (`adjustment=all`). Across a reverse split (WOK 1-for-100,
    // 2025-12) the raw source sits on a different scale and — out-ranking Alpaca
    // per bucket — paints unadjusted pre-split history (the December discontinuity
    // TradingView never shows). Where the raw source diverges from the adjusted
    // reference across a whole consistent ERA (not a single bad print), adopt the
    // adjusted bars so the series stays continuous.
    chart_reconcile_trusted_split_adjustment(&mut merged, &tagged);

    // Independent adjusted-depth reconciliation. Kraken xStock history can be
    // Alpaca-derived, so Kraken + Alpaca may share the same mis-adjusted split
    // history. When Yahoo/TradingView-style data agrees recently but exposes
    // older stable split-era ratios, let it replace those trusted OHLC eras.
    chart_reconcile_depth_split_adjustment(&mut merged, &tagged);

    // Trusted-tier outlier correction. A trusted feed can momentarily emit a
    // bad print — a thin microcap whose provider mis-applies a corporate action
    // (WOK doubled to ~2× on Alpaca for two days in 2026-06 while Yahoo,
    // TradingView and the live tape all stayed flat). The depth tier only fills
    // *gaps*, so a bad trusted bar would otherwise be charted unchallenged and
    // poison the autoscale + every MA/ATR. Where a depth corroborator overlaps on
    // a locally-consistent recent scale, replace any trusted bar that diverges
    // from the rescaled corroborator by more than OUTLIER_RATIO. Deliberately
    // recent-window only: deep history can legitimately sit on a different scale
    // per split era (an unadjusted depth source), so we never "correct" there.
    const OUTLIER_RATIO: f64 = 1.5;
    for (rank, bucketed) in &tagged {
        if *rank <= TRUSTED_MAX_RANK {
            continue;
        }
        let Some((scale, window_start)) = chart_recent_overlap_scale(&merged, bucketed) else {
            continue;
        };
        // Compare close, high, AND low against the rescaled corroborator. A bad
        // trusted print can be a full-candle doubling (close diverges) or a lone
        // wick spike (only the high diverges) — the WOK H4 artifact was the
        // latter, invisible to a close-only check.
        let diverges = |trusted_v: f64, depth_v: f64| -> bool {
            let expected = depth_v * scale;
            expected > 0.0
                && trusted_v > 0.0
                && (trusted_v / expected).max(expected / trusted_v) > OUTLIER_RATIO
        };
        for (bucket, dbar) in bucketed {
            if *bucket < window_start {
                continue; // only adjudicate the recent, locally-consistent window
            }
            let Some(tbar) = merged.get(bucket) else {
                continue;
            };
            if diverges(tbar.close, dbar.close)
                || diverges(tbar.high, dbar.high)
                || diverges(tbar.low, dbar.low)
            {
                merged.insert(
                    *bucket,
                    Bar {
                        ts_ms: tbar.ts_ms,
                        open: dbar.open * scale,
                        high: dbar.high * scale,
                        low: dbar.low * scale,
                        close: dbar.close * scale,
                        volume: tbar.volume,
                    },
                );
            }
        }
        break; // only the best valid corroborator adjudicates
    }

    // Splice depth sources in (best rank first), back-adjusted to the trusted
    // scale, filling only buckets not already covered (older history + gaps).
    for (rank, bucketed) in &tagged {
        if *rank <= TRUSTED_MAX_RANK {
            continue;
        }
        let Some(factor) = chart_depth_source_scale_factor(&merged, bucketed) else {
            continue; // unreconcilable scale (unadjusted action) → drop source
        };
        let rescale = (factor - 1.0).abs() > 1e-9;
        for (bucket, bar) in bucketed {
            if merged.contains_key(bucket) {
                continue;
            }
            let bar = if rescale {
                Bar {
                    ts_ms: bar.ts_ms,
                    open: bar.open * factor,
                    high: bar.high * factor,
                    low: bar.low * factor,
                    close: bar.close * factor,
                    volume: bar.volume,
                }
            } else {
                bar.clone()
            };
            merged.insert(*bucket, bar);
        }
    }

    merged.into_values().collect()
}

/// Reconcile a raw best-rank trusted source against a split-adjusted lower-rank
/// trusted source (Alpaca, `adjustment=all`) — see the call site. Only an
/// ERA-WIDE, internally-consistent divergence (a corporate-action scale step, not
/// a single bad print) is overridden, so a lone bad Alpaca bar can't hijack a good
/// raw bar. Buckets in the recent window are left to the Yahoo outlier guard.
fn chart_reconcile_trusted_split_adjustment(
    merged: &mut std::collections::BTreeMap<i64, Bar>,
    tagged: &[(u8, std::collections::BTreeMap<i64, Bar>)],
) {
    const TRUSTED_MAX_RANK: u8 = 2;
    const DIVERGE_RATIO: f64 = 1.5; // a scale step, not noise
    const MIN_ERA: usize = 5; // need a run of divergent buckets, not one bad bar
    const ERA_TOL: f64 = 1.25; // the divergent ratios must share one scale factor

    // The best-rank trusted source is the one that populated `merged`.
    let Some(best_rank) = tagged
        .iter()
        .map(|(rank, _)| *rank)
        .filter(|rank| *rank <= TRUSTED_MAX_RANK)
        .min()
    else {
        return;
    };

    for (rank, adj) in tagged {
        if *rank <= best_rank || *rank > TRUSTED_MAX_RANK {
            continue; // only a lower-rank trusted source is a candidate reference
        }
        // Recent consensus ratio between merged (raw best) and the adjusted
        // reference. They must agree recently (post-split) for the comparison to
        // mean anything; a window straddling the split is rejected by the
        // tightness check inside chart_recent_overlap_scale.
        let Some((consensus, window_start)) = chart_recent_overlap_scale(merged, adj) else {
            continue;
        };
        // Older buckets where merged diverges from the adjusted reference beyond
        // DIVERGE_RATIO of that recent consensus.
        let mut divergent: Vec<(i64, f64)> = Vec::new();
        for (bucket, abar) in adj {
            if *bucket >= window_start {
                continue; // recent window is handled by the outlier guard
            }
            let Some(mbar) = merged.get(bucket) else {
                continue;
            };
            if abar.close <= 0.0 || mbar.close <= 0.0 {
                continue;
            }
            let ratio = mbar.close / abar.close;
            if (ratio / consensus).max(consensus / ratio) > DIVERGE_RATIO {
                divergent.push((*bucket, ratio));
            }
        }
        if divergent.len() < MIN_ERA {
            continue; // not era-wide → could be a single bad print; leave it alone
        }
        // The divergent ratios must be one consistent scale (a split factor), not
        // scattered single-bar errors.
        let mut ratios: Vec<f64> = divergent.iter().map(|(_, r)| *r).collect();
        ratios.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let p25 = ratios[ratios.len() / 4];
        let p75 = ratios[ratios.len() * 3 / 4];
        if p25 <= 0.0 || p75 / p25 > ERA_TOL {
            continue;
        }
        // A consistent mis-adjusted era → the adjusted reference is authoritative.
        for (bucket, _) in &divergent {
            if let Some(abar) = adj.get(bucket) {
                merged.insert(*bucket, abar.clone());
            }
        }
    }
}

fn chart_reconcile_depth_split_adjustment(
    merged: &mut std::collections::BTreeMap<i64, Bar>,
    tagged: &[(u8, std::collections::BTreeMap<i64, Bar>)],
) {
    const TRUSTED_MAX_RANK: u8 = 2;
    const DIVERGE_RATIO: f64 = 1.5;
    const ERA_TOL: f64 = 1.25;
    const MIN_ERA: usize = 5;

    for (rank, depth) in tagged {
        if *rank <= TRUSTED_MAX_RANK {
            continue;
        }
        let Some((consensus, window_start)) = chart_recent_overlap_scale(merged, depth) else {
            continue;
        };
        let mut run: Vec<(i64, f64)> = Vec::new();
        let mut runs: Vec<Vec<(i64, f64)>> = Vec::new();
        for (bucket, dbar) in depth {
            if *bucket >= window_start {
                break;
            }
            let Some(tbar) = merged.get(bucket) else {
                chart_stage_depth_split_adjustment_run(&mut runs, &run, ERA_TOL, MIN_ERA);
                run.clear();
                continue;
            };
            if tbar.close <= 0.0 || dbar.close <= 0.0 {
                chart_stage_depth_split_adjustment_run(&mut runs, &run, ERA_TOL, MIN_ERA);
                run.clear();
                continue;
            }
            let ratio = tbar.close / dbar.close;
            if (ratio / consensus).max(consensus / ratio) <= DIVERGE_RATIO {
                chart_stage_depth_split_adjustment_run(&mut runs, &run, ERA_TOL, MIN_ERA);
                run.clear();
                continue;
            }
            let same_era = run
                .last()
                .map(|(_, prev_ratio)| {
                    let lo = ratio.min(*prev_ratio);
                    let hi = ratio.max(*prev_ratio);
                    lo > 0.0 && hi / lo <= ERA_TOL
                })
                .unwrap_or(true);
            if !same_era {
                chart_stage_depth_split_adjustment_run(&mut runs, &run, ERA_TOL, MIN_ERA);
                run.clear();
            }
            run.push((*bucket, ratio));
        }
        chart_stage_depth_split_adjustment_run(&mut runs, &run, ERA_TOL, MIN_ERA);
        // Require at least two older stable divergent eras before promoting a
        // depth source over trusted history. A single old depth-only scale jump
        // can be an unadjusted/bad provider region; two clean eras plus recent
        // agreement matches the WOK/TradingView multi-reverse-split shape.
        if runs.len() >= 2 {
            for run in &runs {
                chart_apply_depth_split_adjustment_run(merged, depth, run, consensus);
            }
        }
        break;
    }
}

fn chart_stage_depth_split_adjustment_run(
    runs: &mut Vec<Vec<(i64, f64)>>,
    run: &[(i64, f64)],
    era_tol: f64,
    min_era: usize,
) {
    if run.len() < min_era {
        return;
    }
    let mut ratios: Vec<f64> = run.iter().map(|(_, ratio)| *ratio).collect();
    ratios.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let p25 = ratios[ratios.len() / 4];
    let p75 = ratios[ratios.len() * 3 / 4];
    if p25 <= 0.0 || p75 / p25 > era_tol {
        return;
    }
    runs.push(run.to_vec());
}

fn chart_apply_depth_split_adjustment_run(
    merged: &mut std::collections::BTreeMap<i64, Bar>,
    depth: &std::collections::BTreeMap<i64, Bar>,
    run: &[(i64, f64)],
    consensus: f64,
) {
    if consensus <= 0.0 {
        return;
    }
    for (bucket, _) in run {
        let (Some(depth_bar), Some(trusted_bar)) = (depth.get(bucket), merged.get(bucket)) else {
            continue;
        };
        merged.insert(
            *bucket,
            Bar {
                ts_ms: trusted_bar.ts_ms,
                open: depth_bar.open * consensus,
                high: depth_bar.high * consensus,
                low: depth_bar.low * consensus,
                close: depth_bar.close * consensus,
                volume: trusted_bar.volume,
            },
        );
    }
}

fn chart_trusted_equity_merge_is_stale(
    timeframe: &str,
    trusted: &std::collections::BTreeMap<i64, Bar>,
    tagged: &[(u8, std::collections::BTreeMap<i64, Bar>)],
) -> bool {
    let Some(trusted_last) = trusted.keys().next_back().copied() else {
        return false;
    };
    let Some(depth_last) = tagged
        .iter()
        .filter(|(rank, _)| *rank > 2)
        .filter_map(|(_, bucketed)| bucketed.keys().next_back().copied())
        .max()
    else {
        return false;
    };
    depth_last.saturating_sub(trusted_last) > chart_stale_trusted_equity_gap_ms(timeframe)
}

fn chart_stale_trusted_equity_gap_ms(timeframe: &str) -> i64 {
    let hour = 3_600_000i64;
    let day = 24 * hour;
    match timeframe {
        "1Min" | "5Min" | "15Min" | "30Min" | "1Hour" | "4Hour" => 10 * day,
        "1Day" => 45 * day,
        "1Week" => 120 * day,
        "1Month" => 370 * day,
        _ => 45 * day,
    }
}

/// Validate, bucket, and de-duplicate one raw provider series into
/// `bucket → Bar` (the latest tick within a bucket wins). Bars that fail basic
/// sanity (non-positive / non-finite OHLC, high < low, non-positive ts) drop.
fn chart_bucket_valid_source_bars(
    timeframe: &str,
    raw: &[(i64, f64, f64, f64, f64, f64)],
) -> std::collections::BTreeMap<i64, Bar> {
    let mut out: std::collections::BTreeMap<i64, Bar> = std::collections::BTreeMap::new();
    for (ts, o, h, l, c, v) in raw.iter().copied() {
        if ts <= 0
            || o <= 0.0
            || h <= 0.0
            || l <= 0.0
            || c <= 0.0
            || !o.is_finite()
            || !h.is_finite()
            || !l.is_finite()
            || !c.is_finite()
            || h < l
        {
            continue;
        }
        let bucket = chart_merge_bucket_ts(timeframe, ts);
        let bar = Bar {
            ts_ms: ts,
            open: o,
            high: h,
            low: l,
            close: c,
            volume: v,
        };
        match out.get(&bucket) {
            Some(existing) if existing.ts_ms > ts => {}
            _ => {
                out.insert(bucket, bar);
            }
        }
    }
    out
}

/// Back-adjustment factor that brings a depth source onto the trusted scale:
/// `median(trusted_close / depth_close)` over the buckets they share. Returns
/// `None` — meaning "drop this source" — when there is no overlap, or when the
/// overlap is large enough to judge yet the per-bucket ratios span more than
/// `SCALE_TOL` (p90/p10). A continuously-offset source (a clean, unadjusted but
/// constant split) has a near-constant ratio and is kept & rescaled; an
/// internally-inconsistent source (an unadjusted action mid-history, like
/// Yahoo's WOK) trips the tolerance and is rejected.
fn chart_depth_source_scale_factor(
    trusted: &std::collections::BTreeMap<i64, Bar>,
    depth: &std::collections::BTreeMap<i64, Bar>,
) -> Option<f64> {
    const CONSISTENCY_MIN_OVERLAP: usize = 8;
    const SCALE_TOL: f64 = 3.0;

    let mut factors: Vec<f64> = depth
        .iter()
        .filter_map(|(bucket, dbar)| {
            trusted
                .get(bucket)
                .filter(|tbar| tbar.close > 0.0 && dbar.close > 0.0)
                .map(|tbar| tbar.close / dbar.close)
        })
        .collect();
    if factors.is_empty() {
        return None;
    }
    factors.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    if factors.len() >= CONSISTENCY_MIN_OVERLAP {
        let p10 = factors[factors.len() / 10];
        let p90 = factors[factors.len() * 9 / 10];
        if p10 <= 0.0 || p90 / p10 > SCALE_TOL {
            return None;
        }
    }
    let mid = factors.len() / 2;
    let median = if factors.len() % 2 == 0 {
        (factors[mid - 1] + factors[mid]) / 2.0
    } else {
        factors[mid]
    };
    (median.is_finite() && median > 0.0).then_some(median)
}

/// Robust `median(trusted_close / depth_close)` over only the most recent
/// overlapping buckets, used to sanity-check trusted bars against an independent
/// corroborator. Unlike [`chart_depth_source_scale_factor`] this ignores deep
/// history — where an unadjusted depth source legitimately sits on a different
/// scale per split era — and accepts the scale only when that recent window is
/// internally tight (p75/p25 within `LOCAL_TOL`). That lets it anchor an outlier
/// check on a clean recent scale without being thrown off by old unadjusted
/// bars, so a transient bad print in the trusted feed can be caught and the
/// genuine deep-history splice (handled separately) is left alone.
///
/// Note Kraken xStock bars are sourced from Alpaca on the backend, so the
/// trusted tier (kraken-equities + alpaca) is not self-corroborating — a backend
/// mis-adjustment hits both identically. Yahoo is the independent reference.
fn chart_recent_overlap_scale(
    trusted: &std::collections::BTreeMap<i64, Bar>,
    depth: &std::collections::BTreeMap<i64, Bar>,
) -> Option<(f64, i64)> {
    const MIN_COUNT: usize = 40;
    const MIN_OVERLAP: usize = 10;
    const LOCAL_TOL: f64 = 1.25;
    // The adjudication window is defined by TIME, not a fixed bucket count. A flat
    // 40 buckets is ~40 days on D1 but only ~10 hours on M15, so an intraday bad
    // print even a day old was never reached (the WOK M15 artifact). Take the most
    // recent buckets covering at least MIN_COUNT *and* at least RECENT_WINDOW_MS.
    // The p25/p75 tightness check below still rejects any window that straddles a
    // split-era scale change, so widening the reach stays safe.
    const RECENT_WINDOW_MS: i64 = 30 * 24 * 60 * 60 * 1000; // 30 days

    // Newest-first shared buckets, taken until BOTH the count and time floors pass.
    let mut recent: Vec<(i64, f64)> = Vec::new();
    let mut time_floor: Option<i64> = None;
    for (bucket, ratio) in trusted.iter().rev().filter_map(|(bucket, tbar)| {
        depth
            .get(bucket)
            .filter(|dbar| tbar.close > 0.0 && dbar.close > 0.0)
            .map(|dbar| (*bucket, tbar.close / dbar.close))
    }) {
        match time_floor {
            Some(floor) if recent.len() >= MIN_COUNT && bucket < floor => break,
            None => time_floor = Some(bucket - RECENT_WINDOW_MS),
            _ => {}
        }
        recent.push((bucket, ratio));
    }
    if recent.len() < MIN_OVERLAP {
        return None;
    }
    let window_start = recent.iter().map(|(bucket, _)| *bucket).min()?;
    let mut ratios: Vec<f64> = recent.iter().map(|(_, ratio)| *ratio).collect();
    ratios.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    // Reject a noisy recent window: the tight middle band must be consistent so
    // we never anchor the outlier check on a mixed-scale (mid-split) overlap.
    let p25 = ratios[ratios.len() / 4];
    let p75 = ratios[ratios.len() * 3 / 4];
    if p25 <= 0.0 || p75 / p25 > LOCAL_TOL {
        return None;
    }
    let mid = ratios.len() / 2;
    let median = if ratios.len() % 2 == 0 {
        (ratios[mid - 1] + ratios[mid]) / 2.0
    } else {
        ratios[mid]
    };
    (median.is_finite() && median > 0.0).then_some((median, window_start))
}

pub(crate) fn chart_equity_source_rank(source: &str) -> Option<u8> {
    match source {
        "kraken-equities" => Some(0),
        "tastytrade" => Some(1),
        "alpaca" => Some(2),
        "yahoo-chart" => Some(3),
        "default" => Some(4),
        _ => None,
    }
}

pub(crate) fn chart_prefers_fresh_equity_source(symbol: &str) -> bool {
    let compact = normalize_market_data_symbol(symbol)
        .replace('/', "")
        .trim_end_matches(".EQ")
        .to_ascii_uppercase();
    !compact.is_empty()
        && compact.len() <= 8
        && compact
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.')
        && !compact.ends_with("USD")
        && !compact.ends_with("USDT")
        && !compact.ends_with("USDC")
        && !compact.ends_with("ZUSD")
}

pub(crate) fn chart_forming_bar_allowed(last_bar_ts_ms: i64, now_ms: i64, tf_ms: i64) -> bool {
    if last_bar_ts_ms <= 0 || now_ms <= 0 || tf_ms <= 0 {
        return false;
    }
    let current_bucket = now_ms / tf_ms * tf_ms;
    current_bucket > last_bar_ts_ms && current_bucket.saturating_sub(last_bar_ts_ms) <= tf_ms
}

pub(crate) fn news_symbol_from_market_data_cache_key(key: &str, prefix: &str) -> Option<String> {
    let rest = key.strip_prefix(prefix)?.strip_prefix(':')?;
    let (raw_symbol, tf) = rest.rsplit_once(':')?;
    if raw_symbol.is_empty() || tf.is_empty() || raw_symbol.starts_with("__") {
        return None;
    }
    let mut symbol = normalize_market_data_symbol(raw_symbol)
        .replace('/', "")
        .to_uppercase();
    if let Some(stripped) = symbol.strip_suffix(".EQ") {
        symbol = stripped.to_string();
    }
    if symbol.is_empty() || symbol.starts_with("__") {
        None
    } else {
        Some(symbol)
    }
}

pub(crate) fn extract_news_symbols_from_market_data_cache(
    conn: &BgConnection,
    prefixes: &[&str],
) -> Result<Vec<String>, String> {
    let mut symbols = std::collections::BTreeSet::new();
    for prefix in prefixes {
        let like = format!("{}:%", prefix);
        let mut stmt = conn
            .prepare("SELECT DISTINCT key FROM bar_cache WHERE key LIKE ?1")
            .map_err(|e| format!("prepare {prefix} bar-cache news symbols: {e}"))?;
        let rows = stmt
            .query_map([like.as_str()], |row| row.get::<_, String>(0))
            .map_err(|e| format!("query {prefix} bar-cache news symbols: {e}"))?;
        for row in rows {
            if let Ok(key) = row {
                if let Some(symbol) = news_symbol_from_market_data_cache_key(&key, prefix) {
                    symbols.insert(symbol);
                }
            }
        }
    }
    Ok(symbols.into_iter().collect())
}

pub(crate) const CHART_SOURCE_ORDER: [(&str, &str); 7] = [
    ("kraken", "Kraken"),
    ("kraken-equities", "Kraken Equities"),
    ("kraken-futures", "Kraken Futures"),
    ("tastytrade", "tastytrade"),
    ("alpaca", "Alpaca"),
    ("yahoo-chart", "Yahoo Chart"),
    ("default", "Default"),
];

pub(crate) fn cache_source_label(source: &str) -> &'static str {
    CHART_SOURCE_ORDER
        .iter()
        .find_map(|(key, label)| (*key == source).then_some(*label))
        .unwrap_or("Source")
}

pub(crate) fn push_unique_symbol_variant(out: &mut Vec<String>, value: impl Into<String>) {
    let value = value.into();
    if value.trim().is_empty() {
        return;
    }
    let normalized = value.trim().to_uppercase();
    if !out.iter().any(|s| s.eq_ignore_ascii_case(&normalized)) {
        out.push(normalized);
    }
}

pub(crate) fn chart_source_symbol_variants(symbol: &str) -> Vec<String> {
    let mut variants = Vec::new();
    let raw = bare_symbol_from_key(symbol);
    let norm = normalize_market_data_symbol(&raw);
    let no_slash = norm.replace('/', "");

    push_unique_symbol_variant(&mut variants, raw);
    push_unique_symbol_variant(&mut variants, norm.clone());
    push_unique_symbol_variant(&mut variants, no_slash.clone());
    push_unique_symbol_variant(
        &mut variants,
        typhoon_engine::core::kraken::normalize_pair_symbol(&norm),
    );
    push_unique_symbol_variant(
        &mut variants,
        typhoon_engine::core::kraken_futures::normalize_futures_symbol(&norm),
    );

    if !no_slash.contains('/') && no_slash.len() >= 2 && !no_slash.ends_with("USD") {
        push_unique_symbol_variant(&mut variants, format!("{no_slash}USD"));
    }

    variants
}

pub(crate) fn chart_source_cache_keys(source: &str, symbol: &str, timeframe: &str) -> Vec<String> {
    let variants = chart_source_symbol_variants(symbol);
    let mut keys = Vec::new();
    for variant in variants {
        let source_variant = match source {
            "kraken" | "kraken-futures" => variant.replace('/', ""),
            "kraken-equities" => variant.replace('/', "").trim_end_matches(".EQ").to_string(),
            _ => variant,
        };
        let key = match source {
            "default" => format!("default:{source_variant}:{timeframe}"),
            "alpaca-legacy-paper" => format!("paper_TyphooN:{source_variant}:{timeframe}"),
            "alpaca-legacy-named" => format!("alpaca_paper_TyphooN:{source_variant}:{timeframe}"),
            _ => format!("{source}:{source_variant}:{timeframe}"),
        };
        if !keys.iter().any(|k: &String| k.eq_ignore_ascii_case(&key)) {
            keys.push(key);
        }
    }

    if source == "alpaca" {
        for legacy_source in ["alpaca-legacy-paper", "alpaca-legacy-named"] {
            for key in chart_source_cache_keys(legacy_source, symbol, timeframe) {
                if !keys.iter().any(|k: &String| k.eq_ignore_ascii_case(&key)) {
                    keys.push(key);
                }
            }
        }
    } else if source == "kraken" {
        // Kraken account scope can include xStock/equity balances whose market data is
        // not exposed through Kraken's public OHLC/AssetPairs API. Keep Kraken keys
        // first, then allow underlying-equity caches so active Kraken-scope charts
        // can still render HRTX/GDC/TNDM-style holdings.
        for fallback_source in ["kraken-equities", "alpaca", "default"] {
            for key in chart_source_cache_keys(fallback_source, symbol, timeframe) {
                if !keys.iter().any(|k: &String| k.eq_ignore_ascii_case(&key)) {
                    keys.push(key);
                }
            }
        }
    }

    keys
}

pub(crate) fn chart_merged_equity_cache_key(symbol: &str, timeframe: &str) -> String {
    let symbol = normalize_market_data_symbol(symbol)
        .replace('/', "")
        .trim_end_matches(".EQ")
        .to_ascii_uppercase();
    format!("merged:{symbol}:{timeframe}")
}

fn chart_equity_low_timeframe_requires_native_source(timeframe: &str) -> bool {
    matches!(timeframe, "1Min" | "5Min")
}

/// Serialize merged bars into the cache JSON payload, or `None` when there is
/// nothing worth persisting (no bars, or none with a valid timestamp).
fn chart_merged_bars_to_cache_json(bars: &[Bar]) -> Option<String> {
    if bars.is_empty() {
        return None;
    }
    let json: Vec<serde_json::Value> = bars
        .iter()
        .filter_map(|bar| {
            let timestamp = chrono::DateTime::from_timestamp_millis(bar.ts_ms)?.to_rfc3339();
            Some(serde_json::json!({
                "timestamp": timestamp,
                "open": bar.open,
                "high": bar.high,
                "low": bar.low,
                "close": bar.close,
                "volume": bar.volume,
            }))
        })
        .collect();
    if json.is_empty() {
        return None;
    }
    serde_json::to_string(&json).ok()
}

pub(crate) fn chart_persist_merged_equity_bars_to_cache(
    cache: &SqliteCache,
    symbol: &str,
    timeframe: &str,
    bars: &[Bar],
) -> Result<(), String> {
    let Some(json) = chart_merged_bars_to_cache_json(bars) else {
        return Ok(());
    };
    let key = chart_merged_equity_cache_key(symbol, timeframe);
    cache.put_bars(&key, &json)
}

/// Best-effort merged-cache warm for hot render-thread loads: skips the write
/// entirely when the writer connection is busy (bulk sync) so the render thread
/// never stalls behind it. The merged blob is re-materialised off-thread (the
/// background sync) when it ends up missing.
fn chart_persist_merged_equity_bars_best_effort(
    cache: &SqliteCache,
    symbol: &str,
    timeframe: &str,
    bars: &[Bar],
) {
    let Some(json) = chart_merged_bars_to_cache_json(bars) else {
        return;
    };
    let key = chart_merged_equity_cache_key(symbol, timeframe);
    let _ = cache.put_bars_if_uncontended(&key, &json);
}

pub(crate) fn chart_materialize_merged_equity_cache(
    cache: &SqliteCache,
    symbol: &str,
    timeframe: &str,
) -> Result<usize, String> {
    let merged = chart_build_merged_equity_bars_from_cache(cache, symbol, timeframe);
    chart_persist_merged_equity_bars_to_cache(cache, symbol, timeframe, &merged)?;
    Ok(merged.len())
}

pub(crate) fn chart_load_merged_equity_bars_from_cache(
    cache: &SqliteCache,
    symbol: &str,
    timeframe: &str,
) -> Vec<Bar> {
    let merged = chart_build_merged_equity_bars_from_cache(cache, symbol, timeframe);
    chart_persist_merged_equity_bars_best_effort(cache, symbol, timeframe, &merged);
    merged
}

#[cfg(target_os = "linux")]
fn chart_process_rss_mb() -> Option<f64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            let kb = rest.split_whitespace().next()?.parse::<f64>().ok()?;
            return Some(kb / 1024.0);
        }
    }
    None
}

#[cfg(not(target_os = "linux"))]
fn chart_process_rss_mb() -> Option<f64> {
    None
}

fn chart_rss_label(rss_mb: Option<f64>) -> String {
    rss_mb
        .map(|rss| format!("{rss:.1} MB"))
        .unwrap_or_else(|| "n/a".to_string())
}

fn chart_log_merged_cache_load_start(
    log: &mut std::collections::VecDeque<LogEntry>,
    context: &str,
    symbol: &str,
    timeframe: &str,
) -> (std::time::Instant, Option<f64>) {
    let rss = chart_process_rss_mb();
    let msg = format!(
        "Merged cache load start ({context}): {symbol} [{timeframe}] rss={}",
        chart_rss_label(rss)
    );
    tracing::info!("{msg}");
    log.push_back(LogEntry::info(msg));
    (std::time::Instant::now(), rss)
}

fn chart_log_merged_cache_load_done(
    log: &mut std::collections::VecDeque<LogEntry>,
    context: &str,
    symbol: &str,
    timeframe: &str,
    bars: usize,
    started_at: std::time::Instant,
    rss_before_mb: Option<f64>,
) {
    let rss_after_mb = chart_process_rss_mb();
    let msg = format!(
        "Merged cache load done ({context}): {bars} bars for {symbol} [{timeframe}] load_ms={:.2} rss={} → {}",
        started_at.elapsed().as_secs_f64() * 1000.0,
        chart_rss_label(rss_before_mb),
        chart_rss_label(rss_after_mb)
    );
    tracing::info!("{msg}");
    log.push_back(LogEntry::info(msg));
}

fn chart_build_merged_equity_bars_from_cache(
    cache: &SqliteCache,
    symbol: &str,
    timeframe: &str,
) -> Vec<Bar> {
    if timeframe == "4Hour" {
        let hourly = chart_build_merged_equity_bars_from_cache(cache, symbol, "1Hour");
        if hourly.len() >= 2 {
            let hourly_raw = chart_bars_to_raw(hourly);
            let four_hour = chart_aggregate_raw_to_4hour(&hourly_raw);
            if four_hour.len() >= 2 {
                return chart_raw_to_bars(four_hour);
            }
        }
    }

    if timeframe == "1Week" {
        let daily = chart_build_merged_equity_bars_from_cache(cache, symbol, "1Day");
        if daily.len() >= 2 {
            let daily_raw = chart_bars_to_raw(daily);
            let weekly = chart_aggregate_raw_to_weekly(&daily_raw);
            if weekly.len() >= 2 {
                return weekly;
            }
        }
    }

    if timeframe == "1Month" {
        let daily = chart_build_merged_equity_bars_from_cache(cache, symbol, "1Day");
        if daily.len() >= 2 {
            let daily_raw = chart_bars_to_raw(daily);
            let monthly = ChartState::aggregate_daily_raw_to_monthly(daily_raw);
            if monthly.len() >= 2 {
                return monthly;
            }
        }
    }

    type RawBars = Vec<(i64, f64, f64, f64, f64, f64)>;
    let mut loaded: Vec<(&'static str, RawBars)> = Vec::new();
    let sources: &[&'static str] = match timeframe {
        // For equity/xStock M1/M5, only native Kraken Equities rows are valid
        // merged inputs. Alpaca/Yahoo low-TF rows are stale provider-assist
        // artifacts unless explicitly selected by source override.
        tf if chart_equity_low_timeframe_requires_native_source(tf) => &["kraken-equities"],
        _ => &[
            "yahoo-chart",
            "alpaca",
            "tastytrade",
            "kraken-equities",
            "default",
        ],
    };
    for source in sources {
        for key in chart_source_cache_keys(source, symbol, timeframe) {
            let Ok(Some(raw)) = cache.get_bars_raw(&key) else {
                continue;
            };
            if raw.is_empty() {
                continue;
            }
            loaded.push((source, raw));
            break;
        }
    }

    // Yahoo exposes no native 4-hour interval (see `yahoo_chart_supports_timeframe`),
    // so a "4Hour" merge would otherwise have no independent corroborator and the
    // trusted-tier outlier correction is skipped entirely — exactly why a bad
    // Alpaca 4Hour print (WOK, 2026-06) reached the H4 chart while H1, corroborated
    // by Yahoo's 1h series, stayed clean. Synthesize a 4-hour Yahoo series by
    // aggregating cached 1-hour Yahoo bars to restore that corroborator.
    if timeframe == "4Hour" && !loaded.iter().any(|(src, _)| *src == "yahoo-chart") {
        if let Some(hourly) = chart_source_cache_keys("yahoo-chart", symbol, "1Hour")
            .iter()
            .find_map(|key| cache.get_bars_raw(key).ok().flatten())
            .filter(|raw| !raw.is_empty())
        {
            let agg = chart_aggregate_raw_to_4hour(&hourly);
            if agg.len() >= 2 {
                loaded.push(("yahoo-chart", agg));
            }
        }
    }

    let views: Vec<(&str, &[(i64, f64, f64, f64, f64, f64)])> = loaded
        .iter()
        .map(|(source, raw)| (*source, raw.as_slice()))
        .collect();
    let splits = chart_known_splits_from_cache(cache, symbol);
    chart_merge_equity_raw_bars(timeframe, &views, &splits)
}

fn chart_bars_to_raw(bars: Vec<Bar>) -> Vec<(i64, f64, f64, f64, f64, f64)> {
    bars.into_iter()
        .map(|bar| {
            (
                bar.ts_ms, bar.open, bar.high, bar.low, bar.close, bar.volume,
            )
        })
        .collect()
}

fn chart_raw_to_bars(raw: Vec<(i64, f64, f64, f64, f64, f64)>) -> Vec<Bar> {
    raw.into_iter()
        .map(|(ts_ms, open, high, low, close, volume)| Bar {
            ts_ms,
            open,
            high,
            low,
            close,
            volume,
        })
        .collect()
}

/// Aggregate a finer raw OHLCV series into 4-hour buckets aligned exactly to
/// [`chart_merge_bucket_ts`]'s "4Hour" boundaries, so the result overlaps native
/// 4-hour bars bucket-for-bucket inside the merge. Open = first bar in a bucket,
/// close = last, high/low = extremes, volume = sum. Used to synthesize a 4-hour
/// Yahoo corroborator from cached 1-hour Yahoo bars.
fn chart_aggregate_raw_to_4hour(
    raw: &[(i64, f64, f64, f64, f64, f64)],
) -> Vec<(i64, f64, f64, f64, f64, f64)> {
    let mut sorted: Vec<(i64, f64, f64, f64, f64, f64)> = raw
        .iter()
        .copied()
        .filter(|(ts, o, h, l, c, _v)| {
            *ts > 0
                && *o > 0.0
                && *h > 0.0
                && *l > 0.0
                && *c > 0.0
                && o.is_finite()
                && h.is_finite()
                && l.is_finite()
                && c.is_finite()
                && *h >= *l
        })
        .collect();
    sorted.sort_by_key(|(ts, ..)| *ts);

    let mut out: std::collections::BTreeMap<i64, Bar> = std::collections::BTreeMap::new();
    for (ts, o, h, l, c, v) in sorted {
        let bucket = chart_merge_bucket_ts("4Hour", ts);
        out.entry(bucket)
            .and_modify(|b| {
                if h > b.high {
                    b.high = h;
                }
                if l < b.low {
                    b.low = l;
                }
                b.close = c;
                b.volume += v;
            })
            .or_insert(Bar {
                ts_ms: bucket,
                open: o,
                high: h,
                low: l,
                close: c,
                volume: v,
            });
    }
    out.into_values()
        .map(|b| (b.ts_ms, b.open, b.high, b.low, b.close, b.volume))
        .collect()
}

fn chart_aggregate_raw_to_weekly(raw: &[(i64, f64, f64, f64, f64, f64)]) -> Vec<Bar> {
    let mut sorted: Vec<(i64, f64, f64, f64, f64, f64)> = raw
        .iter()
        .copied()
        .filter(|(ts, o, h, l, c, _v)| {
            *ts > 0
                && *o > 0.0
                && *h > 0.0
                && *l > 0.0
                && *c > 0.0
                && o.is_finite()
                && h.is_finite()
                && l.is_finite()
                && c.is_finite()
                && *h >= *l
        })
        .collect();
    sorted.sort_by_key(|(ts, ..)| *ts);

    let mut out: std::collections::BTreeMap<i64, Bar> = std::collections::BTreeMap::new();
    for (ts, o, h, l, c, v) in sorted {
        let bucket = chart_merge_bucket_ts("1Week", ts);
        out.entry(bucket)
            .and_modify(|b| {
                b.high = b.high.max(h).max(c);
                b.low = b.low.min(l).min(c);
                b.close = c;
                b.volume += v;
            })
            .or_insert(Bar {
                ts_ms: bucket,
                open: o,
                high: h,
                low: l,
                close: c,
                volume: v,
            });
    }
    out.into_values().collect()
}

impl ChartState {
    const MAX_LIVE_QUOTE_SPREAD_PCT_FOR_MID: f64 = 5.0;

    /// Point this chart at a new symbol and clear per-symbol live-quote state.
    /// `live_bid`/`live_ask`/`live_quote_at`/`live_quote_delayed` belong to the
    /// *previous* symbol: carrying them over folds a stale mid into the new
    /// symbol's forming bar (see `has_live_quotes`) and can make the real-time
    /// quote guards suppress the new symbol's newest delayed quotes for up to the
    /// freshness window. Use for in-place symbol switches (watchlist / screener /
    /// explorer / peers); `reload_symbol_auto` already builds a fresh ChartState.
    pub(crate) fn switch_symbol(&mut self, symbol: impl Into<String>) {
        self.symbol = symbol.into();
        self.live_bid = 0.0;
        self.live_ask = 0.0;
        self.live_quote_at = None;
        self.live_quote_delayed = false;
    }

    pub(crate) fn fresh_live_quote_mid(&self) -> Option<f64> {
        let fresh = self
            .live_quote_at
            .is_some_and(|t| t.elapsed() < std::time::Duration::from_secs(30));
        if fresh && self.live_bid > 0.0 && self.live_ask > 0.0 {
            let mid = (self.live_bid + self.live_ask) * 0.5;
            if mid > 0.0
                && mid.is_finite()
                && Self::live_quote_mid_is_usable(self.live_bid, self.live_ask)
            {
                return Some(mid);
            }
        }
        None
    }

    pub(crate) fn live_quote_spread_pct(bid: f64, ask: f64) -> Option<f64> {
        let mid = (bid + ask) * 0.5;
        if bid <= 0.0 || ask <= 0.0 || mid <= 0.0 || !bid.is_finite() || !ask.is_finite() {
            return None;
        }
        Some(((ask - bid).abs() / mid) * 100.0)
    }

    fn live_quote_mid_is_usable(bid: f64, ask: f64) -> bool {
        Self::live_quote_spread_pct(bid, ask)
            .is_some_and(|spread_pct| spread_pct <= Self::MAX_LIVE_QUOTE_SPREAD_PCT_FOR_MID)
    }

    pub(crate) fn apply_forming_price_update(&mut self, price: f64) -> bool {
        if price <= 0.0 || !price.is_finite() {
            return false;
        }
        let Some(bar) = self.bars.last_mut() else {
            return false;
        };
        bar.close = price;
        bar.high = bar.high.max(price);
        bar.low = if bar.low > 0.0 {
            bar.low.min(price)
        } else {
            price
        };
        self.forming_bar_dirty = true;
        self.last_visible_bar_ts = self.bars.last().map(|b| b.ts_ms).unwrap_or(0);
        true
    }

    pub(crate) fn apply_live_quote_update(&mut self, bid: f64, ask: f64, delayed: bool) -> bool {
        let mid = (bid + ask) * 0.5;
        if bid <= 0.0 || ask <= 0.0 || mid <= 0.0 || !bid.is_finite() || !ask.is_finite() {
            return false;
        }
        self.live_bid = bid;
        self.live_ask = ask;
        self.live_quote_at = Some(std::time::Instant::now());
        self.live_quote_delayed = delayed;
        if self.ext_active {
            return false;
        }
        if !Self::live_quote_mid_is_usable(bid, ask) {
            return false;
        }
        self.apply_forming_price_update(mid)
    }

    pub(crate) fn fold_fresh_live_quote_into_forming_bar(&mut self) -> bool {
        self.fresh_live_quote_mid()
            .is_some_and(|mid| self.apply_forming_price_update(mid))
    }

    pub(crate) fn new(symbol: impl Into<String>, tf: Timeframe) -> Self {
        Self {
            symbol: symbol.into(),
            timeframe: tf,
            primary_source: "",
            source_override: "",
            regulatory_alerts: Vec::new(),
            chart_type: ChartType::Candle,
            log_scale: false,
            sma_slow_period: 200,
            sma_fast_period: 100,
            ema_period: 21,
            rsi_period: 14,
            atr_period: 14,
            bb_period: 20,
            macd_fast: 12,
            macd_slow: 26,
            macd_signal_p: 9,
            stoch_period: 14,
            adx_period: 14,
            fisher_period: 32,
            momentum_period: 10,
            compare_symbol: None,
            compare_bars: Vec::new(),
            live_bid: 0.0,
            live_ask: 0.0,
            live_quote_at: None,
            live_quote_delayed: false,
            ext_open: 0.0,
            ext_high: 0.0,
            ext_low: 0.0,
            ext_close: 0.0,
            ext_active: false,
            bars: Vec::new(),
            upload_opens: Vec::new(),
            upload_closes: Vec::new(),
            upload_highs: Vec::new(),
            upload_lows: Vec::new(),
            upload_volumes: Vec::new(),
            sma200: Vec::new(),
            sma100: Vec::new(),
            kama: Vec::new(),
            ema21: Vec::new(),
            bb_mid: Vec::new(),
            bb_upper: Vec::new(),
            bb_lower: Vec::new(),
            rsi: Vec::new(),
            fisher: Vec::new(),
            fisher_signal: Vec::new(),
            atr: Vec::new(),
            macd_line: Vec::new(),
            macd_signal: Vec::new(),
            macd_hist: Vec::new(),
            stoch_k: Vec::new(),
            stoch_d: Vec::new(),
            adx: Vec::new(),
            di_plus: Vec::new(),
            di_minus: Vec::new(),
            ichi_tenkan: Vec::new(),
            ichi_kijun: Vec::new(),
            ichi_span_a: Vec::new(),
            ichi_span_b: Vec::new(),
            prev_daily_high: None,
            prev_daily_low: None,
            prev_weekly_high: None,
            prev_weekly_low: None,
            prev_h4_high: None,
            prev_h4_low: None,
            prev_h1_high: None,
            prev_h1_low: None,
            prev_monthly_high: None,
            prev_monthly_low: None,
            wma: Vec::new(),
            hma: Vec::new(),
            cci: Vec::new(),
            williams_r: Vec::new(),
            obv: Vec::new(),
            momentum: Vec::new(),
            cmo: Vec::new(),
            cmo_sum_up: 0.0,
            cmo_sum_down: 0.0,
            linreg_sum_x: 0.0,
            linreg_sum_y: 0.0,
            linreg_sum_xy: 0.0,
            linreg_sum_x2: 0.0,
            linreg: Vec::new(),
            linreg_angle: Vec::new(),
            linreg_intercept: Vec::new(),
            linreg_slope: Vec::new(),
            qstick: Vec::new(),
            disparity: Vec::new(),
            bop: Vec::new(),
            stddev: Vec::new(),
            mfi: Vec::new(),
            trix_line: Vec::new(),
            trix_signal: Vec::new(),
            trix_hist: Vec::new(),
            ppo_line: Vec::new(),
            ppo_signal: Vec::new(),
            ppo_hist: Vec::new(),
            ultosc: Vec::new(),
            stochrsi_k: Vec::new(),
            stochrsi_d: Vec::new(),
            var_oscillator: Vec::new(),
            psar: Vec::new(),
            mtf_sma: Vec::new(),
            atr_proj_upper: Vec::new(),
            atr_proj_lower: Vec::new(),
            atr_proj_levels: Vec::new(),
            better_vol_type: Vec::new(),
            pivot_p: None,
            pivot_r1: None,
            pivot_r2: None,
            pivot_s1: None,
            pivot_s2: None,
            fractal_up: Vec::new(),
            fractal_down: Vec::new(),
            ehlers_ss: Vec::new(),
            ehlers_decycler: Vec::new(),
            ehlers_itl: Vec::new(),
            ehlers_mama: Vec::new(),
            ehlers_fama: Vec::new(),
            ehlers_ebsw: Vec::new(),
            ehlers_cyber: Vec::new(),
            ehlers_cg: Vec::new(),
            ehlers_roof: Vec::new(),
            supply_zones: Vec::new(),
            primary_first_ts: 0,
            gap_fill_timestamps: std::collections::HashSet::new(),
            demand_zones: Vec::new(),
            auto_fib_levels: Vec::new(),
            auto_fib_swing: None,
            supertrend: Vec::new(),
            supertrend_bull: Vec::new(),
            donchian_upper: Vec::new(),
            donchian_lower: Vec::new(),
            keltner_mid: Vec::new(),
            keltner_upper: Vec::new(),
            keltner_lower: Vec::new(),
            regression_mid: Vec::new(),
            regression_upper: Vec::new(),
            regression_lower: Vec::new(),
            squeeze_mom: Vec::new(),
            squeeze_on: Vec::new(),
            vol_avg_20: Vec::new(),
            vwap: Vec::new(),
            vwap_upper1: Vec::new(),
            vwap_upper2: Vec::new(),
            vwap_upper3: Vec::new(),
            vwap_lower1: Vec::new(),
            vwap_lower2: Vec::new(),
            vwap_lower3: Vec::new(),
            multi_kama: Vec::new(),
            harmonics: Vec::new(),
            drawings: Vec::new(),
            drawing_styles: Vec::new(),
            drawings_undo: Vec::new(),
            selected_drawing: None,
            dragging_cp: None,
            is_drawing_drag: false,
            cached_trade_overlay: TradeOverlay::default(),
            cached_trade_overlay_frame: 0,
            visible_bars: 200,
            view_offset: 0,
            manual_view_override: false,
            camera: ChartCamera::from_legacy(0, 200, false),
            replay_bar_cap: None,
            price_pan: 0.0,
            price_zoom: 1.0,
            is_dragging: false,
            drag_start: None,
            is_scaling_price: false,
            scale_start_zoom: 1.0,
            scale_start_y: 0.0,
            visible_bars_gen: 0,
            forming_bar_dirty: false,
            heavy_sync_in_progress: false,
            last_visible_bar_ts: 0,
            last_rendered_gen: 0,
            last_rendered_bar_ts: 0,
        }
    }

    /// Fast-path update used by live Kraken WS: only mutate the last bar
    /// and set the dirty flag so draw_chart can early-out everything else.
    #[allow(dead_code)]
    pub fn apply_forming_bar_update(&mut self, bar: Bar) {
        if let Some(last) = self.bars.last_mut() {
            if last.ts_ms == bar.ts_ms {
                *last = bar;
            } else {
                self.bars.push(bar);
            }
        } else {
            self.bars.push(bar);
        }
        self.forming_bar_dirty = true;
        self.last_visible_bar_ts = self.bars.last().map(|b| b.ts_ms).unwrap_or(0);
    }

    /// Call when a closed bar is added or the visible range structurally changes.
    #[allow(dead_code)]
    pub fn mark_structural_change(&mut self) {
        self.visible_bars_gen = self.visible_bars_gen.wrapping_add(1);
        self.forming_bar_dirty = false;
        self.last_visible_bar_ts = self.bars.last().map(|b| b.ts_ms).unwrap_or(0);
    }

    pub(crate) fn symbol_matches(&self, symbol: &str) -> bool {
        normalize_market_data_symbol(&self.symbol)
            .replace('/', "")
            .eq_ignore_ascii_case(&normalize_market_data_symbol(symbol).replace('/', ""))
    }

    pub(crate) fn should_reload_for_bar_fetch(
        &self,
        symbol: &str,
        timeframe: &str,
        source: &str,
    ) -> bool {
        if !self.symbol_matches(symbol)
            || !self
                .timeframe
                .cache_suffix()
                .eq_ignore_ascii_case(timeframe)
        {
            return false;
        }
        if matches!(source, "alpaca" | "yahoo-chart")
            && self.primary_source.eq_ignore_ascii_case("kraken-equities")
        {
            return true;
        }
        self.bars.is_empty()
            || self.primary_source.is_empty()
            || self.primary_source.eq_ignore_ascii_case(source)
    }

    pub(crate) fn latest_quote_bar_from_cache(cache: &SqliteCache, symbol: &str) -> Option<Bar> {
        chart_source_cache_keys("kraken-equities", symbol, "quote")
            .into_iter()
            .filter_map(|key| cache.get_bars_raw(&key).ok().flatten())
            .flat_map(|raw| raw.into_iter())
            .filter(|(ts, o, h, l, c, _v)| {
                *ts > 0
                    && *o > 0.0
                    && *h > 0.0
                    && *l > 0.0
                    && *c > 0.0
                    && o.is_finite()
                    && h.is_finite()
                    && l.is_finite()
                    && c.is_finite()
                    && *h >= *l
            })
            .max_by_key(|(ts, _, _, _, _, _)| *ts)
            .map(|(ts_ms, open, high, low, close, volume)| Bar {
                ts_ms,
                open,
                high,
                low,
                close,
                volume,
            })
    }

    pub(crate) fn chart_timeframe_ms(&self) -> i64 {
        (self.timeframe.minutes().max(1) as i64) * 60_000
    }

    pub(crate) fn aggregate_daily_raw_to_monthly(
        raw: Vec<(i64, f64, f64, f64, f64, f64)>,
    ) -> Vec<Bar> {
        use chrono::{Datelike, TimeZone};
        let mut monthly: std::collections::BTreeMap<(i32, u32), Bar> =
            std::collections::BTreeMap::new();
        for (ts, o, h, l, c, v) in raw.into_iter().filter(|(ts, o, h, l, c, _v)| {
            *ts > 0
                && *o > 0.0
                && *h > 0.0
                && *l > 0.0
                && *c > 0.0
                && o.is_finite()
                && h.is_finite()
                && l.is_finite()
                && c.is_finite()
                && *h >= *l
        }) {
            let Some(dt) = chrono::Utc.timestamp_millis_opt(ts).single() else {
                continue;
            };
            let Some(bucket_dt) = chrono::Utc
                .with_ymd_and_hms(dt.year(), dt.month(), 1, 0, 0, 0)
                .single()
            else {
                continue;
            };
            let bucket_key = (dt.year(), dt.month());
            let bucket_ts = bucket_dt.timestamp_millis();
            monthly
                .entry(bucket_key)
                .and_modify(|bar| {
                    bar.high = bar.high.max(h).max(c);
                    bar.low = bar.low.min(l).min(c);
                    bar.close = c;
                    bar.volume += v;
                })
                .or_insert(Bar {
                    ts_ms: bucket_ts,
                    open: o,
                    high: h,
                    low: l,
                    close: c,
                    volume: v,
                });
        }
        monthly.into_values().collect()
    }

    pub(crate) fn aggregate_bars_to_timeframe(
        raw: Vec<(i64, f64, f64, f64, f64, f64)>,
        tf_ms: i64,
    ) -> Vec<Bar> {
        let mut aggregated: Vec<Bar> = Vec::new();
        let mut current_bucket: Option<i64> = None;
        for (ts, o, h, l, c, v) in raw.into_iter().filter(|(ts, o, h, l, c, _v)| {
            *ts > 0
                && *o > 0.0
                && *h > 0.0
                && *l > 0.0
                && *c > 0.0
                && o.is_finite()
                && h.is_finite()
                && l.is_finite()
                && c.is_finite()
                && *h >= *l
        }) {
            let bucket = ts / tf_ms * tf_ms;
            if current_bucket != Some(bucket) {
                aggregated.push(Bar {
                    ts_ms: bucket,
                    open: o,
                    high: h,
                    low: l,
                    close: c,
                    volume: v,
                });
                current_bucket = Some(bucket);
            } else if let Some(bar) = aggregated.last_mut() {
                bar.high = bar.high.max(h).max(c);
                bar.low = bar.low.min(l).min(c);
                bar.close = c;
                bar.volume += v;
            }
        }
        aggregated
    }

    pub(crate) fn rebuild_from_lower_timeframe_if_dislocated(
        &mut self,
        cache: &SqliteCache,
        symbol: &str,
    ) -> bool {
        let Some(quote) = Self::latest_quote_bar_from_cache(cache, symbol) else {
            return false;
        };
        if self.bars.is_empty() || quote.close <= 0.0 || !quote.close.is_finite() {
            return false;
        }
        let Some(last_close) = self
            .bars
            .last()
            .map(|bar| bar.close)
            .filter(|p| *p > 0.0 && p.is_finite())
        else {
            return false;
        };
        let ratio = if last_close >= quote.close {
            last_close / quote.close
        } else {
            quote.close / last_close
        };
        if ratio < 20.0 {
            return false;
        }

        let target_tf_ms = self.chart_timeframe_ms();
        let lower_tfs = [
            ("1Min", 60_000_i64),
            ("5Min", 5 * 60_000_i64),
            ("15Min", 15 * 60_000_i64),
            ("30Min", 30 * 60_000_i64),
            ("1Hour", 60 * 60_000_i64),
            ("4Hour", 4 * 60 * 60_000_i64),
        ];
        let source = if self.primary_source.is_empty() {
            "kraken-equities"
        } else {
            self.primary_source
        };
        for (lower_tf, lower_ms) in lower_tfs {
            if lower_ms >= target_tf_ms {
                continue;
            }
            for key in chart_source_cache_keys(source, symbol, lower_tf) {
                let Ok(Some(raw)) = cache.get_bars_raw(&key) else {
                    continue;
                };
                let rebuilt = Self::aggregate_bars_to_timeframe(raw, target_tf_ms);
                if rebuilt.len() < 2 {
                    continue;
                }
                let Some(rebuilt_close) = rebuilt
                    .last()
                    .map(|bar| bar.close)
                    .filter(|p| *p > 0.0 && p.is_finite())
                else {
                    continue;
                };
                let rebuilt_ratio = if rebuilt_close >= quote.close {
                    rebuilt_close / quote.close
                } else {
                    quote.close / rebuilt_close
                };
                if rebuilt_ratio < 20.0 {
                    self.bars = rebuilt;
                    self.primary_source = source;
                    self.primary_first_ts = self.bars.first().map(|bar| bar.ts_ms).unwrap_or(0);
                    self.gap_fill_timestamps.clear();
                    return true;
                }
            }
        }
        false
    }

    pub(crate) fn apply_quote_cache_overlay(&mut self, cache: &SqliteCache, symbol: &str) -> bool {
        let Some(quote) = Self::latest_quote_bar_from_cache(cache, symbol) else {
            return false;
        };
        if self.bars.is_empty() {
            self.bars.push(quote);
            self.primary_source = "kraken-equities";
            return true;
        }
        let tf_ms = self.chart_timeframe_ms();
        let Some(last) = self.bars.last_mut() else {
            return false;
        };
        // Always allow live quotes. The 30-second freshness guard in technical_analysis.rs
        // prevents stale bid/ask from being shown. This fixes decoupling during extended hours.
        if quote.ts_ms < last.ts_ms.saturating_add(tf_ms) {
            last.close = quote.close;
            last.high = last.high.max(quote.high).max(quote.close);
            last.low = if last.low > 0.0 {
                last.low.min(quote.low).min(quote.close)
            } else {
                quote.low.min(quote.close)
            };
            last.volume = last.volume.max(quote.volume);
        } else {
            self.bars.push(quote);
        }
        true
    }

    /// Cache key for this symbol + timeframe.
    /// Try multiple prefix variants to find data in cache.
    pub(crate) fn find_cache_key(
        &self,
        cache: &SqliteCache,
        dsm: &typhoon_engine::core::data_source::DataSourceManager,
    ) -> String {
        let tf = self.timeframe.cache_suffix();
        let sym = {
            let parts: Vec<&str> = self.symbol.split(':').collect();
            let is_tf = matches!(
                parts.last().copied(),
                Some(
                    "1Min"
                        | "5Min"
                        | "15Min"
                        | "30Min"
                        | "1Hour"
                        | "4Hour"
                        | "1Day"
                        | "1Week"
                        | "1Month"
                )
            );
            if is_tf && parts.len() > 1 {
                parts[..parts.len() - 1].join(":")
            } else {
                self.symbol.clone()
            }
        };
        let sym_norm = normalize_market_data_symbol(&sym);

        // Normalize crypto: try both with and without slash
        let sym_alt = if sym_norm.contains('/') {
            sym_norm.replace('/', "")
        } else {
            let crypto_bases = [
                "BTC", "ETH", "SOL", "DOGE", "XRP", "ADA", "LTC", "LINK", "AVAX", "DOT", "XMR",
                "ZEC", "DASH", "UNI", "AAVE", "MATIC", "SHIB", "ATOM", "ALGO", "FTM", "NEAR",
                "APE", "ARB", "OP", "MKR", "COMP", "SNX", "CRV", "SUSHI", "YFI", "BAT", "MANA",
                "SAND", "AXS", "BCH", "ETC", "XLM", "FIL", "HBAR", "ICP", "VET", "THETA",
            ];
            let su = sym_norm.to_uppercase();
            crypto_bases
                .iter()
                .find_map(|base| {
                    if su.starts_with(base) && su.ends_with("USD") && su.len() == base.len() + 3 {
                        Some(format!("{}/USD", base))
                    } else {
                        None
                    }
                })
                .unwrap_or_default()
        };

        // ADR-038 Phase 2: Use DataSourceManager for priority-ordered candidates
        let mut candidates = dsm.resolve_candidates(&sym, tf);
        if sym_norm != sym {
            candidates.extend(dsm.resolve_candidates(&sym_norm, tf));
        }
        // Also add legacy key variants for backward compatibility
        candidates.push(format!("paper_TyphooN:{}:{}", sym.to_uppercase(), tf));
        candidates.push(format!(
            "alpaca_paper_TyphooN:{}:{}",
            sym.to_uppercase(),
            tf
        ));
        candidates.push(format!("default:{}:{}", sym.to_uppercase(), tf));
        if sym_norm != sym {
            candidates.push(format!("paper_TyphooN:{}:{}", sym_norm.to_uppercase(), tf));
            candidates.push(format!(
                "alpaca_paper_TyphooN:{}:{}",
                sym_norm.to_uppercase(),
                tf
            ));
            candidates.push(format!("default:{}:{}", sym_norm.to_uppercase(), tf));
        }
        // Crypto slash/no-slash variants
        if !sym_alt.is_empty() {
            let alt_candidates = dsm.resolve_candidates(&sym_alt, tf);
            candidates.extend(alt_candidates);
        }

        let prefer_fresh_equity = chart_prefers_fresh_equity_source(&sym_norm);
        let mut best_equity: Option<(String, i64, u8)> = None;
        for key in &candidates {
            if let Ok(Some(raw)) = cache.get_bars_raw(key) {
                if !raw.is_empty()
                    && chart_source_bars_match_timeframe(cache_source_from_key(key), tf, &raw)
                {
                    let source = cache_source_from_key(key);
                    if prefer_fresh_equity {
                        if let Some(rank) = chart_equity_source_rank(source) {
                            let last_ts = chart_bar_last_valid_ts(&raw);
                            let replace = best_equity
                                .as_ref()
                                .map(|(_, best_ts, best_rank)| {
                                    last_ts > *best_ts || (last_ts == *best_ts && rank < *best_rank)
                                })
                                .unwrap_or(true);
                            if replace {
                                best_equity = Some((key.clone(), last_ts, rank));
                            }
                            continue;
                        }
                    }
                    return key.clone();
                }
            }
        }

        // Fallback: partial-match search via SQL LIKE
        if let Ok(keys) = cache.search_keys(&sym, 32) {
            let tf_lower = tf.to_lowercase();
            for key in &keys {
                if key.to_lowercase().ends_with(&tf_lower) {
                    if let Ok(Some(raw)) = cache.get_bars_raw(key) {
                        if !raw.is_empty()
                            && chart_source_bars_match_timeframe(
                                cache_source_from_key(key),
                                tf,
                                &raw,
                            )
                        {
                            let source = cache_source_from_key(key);
                            if prefer_fresh_equity {
                                if let Some(rank) = chart_equity_source_rank(source) {
                                    let last_ts = chart_bar_last_valid_ts(&raw);
                                    let replace = best_equity
                                        .as_ref()
                                        .map(|(_, best_ts, best_rank)| {
                                            last_ts > *best_ts
                                                || (last_ts == *best_ts && rank < *best_rank)
                                        })
                                        .unwrap_or(true);
                                    if replace {
                                        best_equity = Some((key.clone(), last_ts, rank));
                                    }
                                    continue;
                                }
                            }
                            return key.clone();
                        }
                    }
                }
            }
        }

        if let Some((key, _, _)) = best_equity {
            return key;
        }

        // Default fallback: first source in priority order
        format!("kraken:{}:{}", sym, tf)
    }

    /// Fast cache key without any DB probing. Used by try_load to avoid blocking.
    /// Try to load bars without blocking. Returns false if lock is contended.
    /// Use this from the UI thread render loop to avoid freezing.
    /// Load bars from cache. read_conn is exclusively owned by the UI thread,
    /// so lock() always succeeds immediately — no contention possible.
    /// Returns true if data was loaded (even if empty), false only on error.
    pub(crate) fn try_load(
        &mut self,
        cache: &SqliteCache,
        log: &mut VecDeque<LogEntry>,
        gpu: Option<&mut gpu_compute::GpuCompute>,
    ) -> bool {
        // Data priority mirrors DataSourceManager's default order:
        // Kraken spot/xStocks → Kraken Futures → Alpaca fallback.
        let sym = {
            let parts: Vec<&str> = self.symbol.split(':').collect();
            let is_tf = matches!(
                parts.last().copied(),
                Some(
                    "1Min"
                        | "5Min"
                        | "15Min"
                        | "30Min"
                        | "1Hour"
                        | "4Hour"
                        | "1Day"
                        | "1Week"
                        | "1Month"
                )
            );
            if is_tf && parts.len() > 1 {
                parts[..parts.len() - 1].join(":")
            } else {
                self.symbol.clone()
            }
        };
        let sym_norm = normalize_market_data_symbol(&sym);
        let tf = self.timeframe.cache_suffix();
        let old_bars_empty = self.bars.is_empty();
        let old_len = self.bars.len();
        let source_override = self.source_override;
        if source_override == "merged" {
            let (load_started_at, load_rss_before) = chart_log_merged_cache_load_start(
                log,
                "source_override",
                &sym,
                self.timeframe.label(),
            );
            let merged = chart_load_merged_equity_bars_from_cache(cache, &sym, tf);
            chart_log_merged_cache_load_done(
                log,
                "source_override",
                &sym,
                self.timeframe.label(),
                merged.len(),
                load_started_at,
                load_rss_before,
            );
            if !merged.is_empty() {
                self.gap_fill_timestamps.clear();
                self.bars = merged;
                self.primary_source = "merged";
                self.source_override = source_override;
                self.primary_first_ts = self.bars.first().map(|bar| bar.ts_ms).unwrap_or(0);
                if old_bars_empty {
                    self.view_offset = self.bars.len().saturating_sub(1) + CHART_RIGHT_MARGIN;
                    self.manual_view_override = false;
                    self.reset_camera_from_legacy();
                } else {
                    self.camera.on_data_len_changed(old_len, self.bars.len());
                    self.sync_camera_to_legacy();
                }
                self.compute_indicators_gpu(gpu);
            } else {
                self.bars.clear();
                self.primary_source = "";
                self.source_override = source_override;
                self.gap_fill_timestamps.clear();
                self.compute_indicators_gpu(gpu);
            }
            return true;
        }
        if !source_override.is_empty() {
            let mut result: Option<Vec<(i64, f64, f64, f64, f64, f64)>> = None;
            for key in chart_source_cache_keys(source_override, &sym, tf) {
                match cache.get_bars_raw(&key) {
                    Ok(Some(raw))
                        if !raw.is_empty()
                            && chart_source_bars_match_timeframe(source_override, tf, &raw) =>
                    {
                        result = Some(raw);
                        break;
                    }
                    _ => {}
                }
            }
            if let Some(raw) = result {
                self.gap_fill_timestamps.clear();
                self.bars = raw
                    .into_iter()
                    .filter(|(ts, o, h, l, c, _v)| {
                        *ts > 0
                            && *o > 0.0
                            && *h > 0.0
                            && *l > 0.0
                            && *c > 0.0
                            && o.is_finite()
                            && h.is_finite()
                            && l.is_finite()
                            && c.is_finite()
                            && *h >= *l
                    })
                    .map(|(ts, o, h, l, c, v)| Bar {
                        ts_ms: ts,
                        open: o,
                        high: h,
                        low: l,
                        close: c,
                        volume: v,
                    })
                    .collect();
                self.primary_source = if self.bars.is_empty() {
                    ""
                } else {
                    source_override
                };
                self.source_override = source_override;
                self.primary_first_ts = self.bars.first().map(|bar| bar.ts_ms).unwrap_or(0);
                if old_bars_empty {
                    self.view_offset = self.bars.len().saturating_sub(1) + CHART_RIGHT_MARGIN;
                    self.manual_view_override = false;
                    self.reset_camera_from_legacy();
                } else {
                    self.camera.on_data_len_changed(old_len, self.bars.len());
                    self.sync_camera_to_legacy();
                }
                self.compute_indicators_gpu(gpu);
            } else {
                self.bars.clear();
                self.primary_source = "";
                self.source_override = source_override;
                self.gap_fill_timestamps.clear();
                self.compute_indicators_gpu(gpu);
            }
            return true;
        }
        if chart_prefers_fresh_equity_source(&sym_norm) {
            let (load_started_at, load_rss_before) = chart_log_merged_cache_load_start(
                log,
                "fresh_equity_auto",
                &sym,
                self.timeframe.label(),
            );
            let merged = chart_load_merged_equity_bars_from_cache(cache, &sym, tf);
            chart_log_merged_cache_load_done(
                log,
                "fresh_equity_auto",
                &sym,
                self.timeframe.label(),
                merged.len(),
                load_started_at,
                load_rss_before,
            );
            if !merged.is_empty() {
                self.gap_fill_timestamps.clear();
                self.bars = merged;
                self.primary_source = "merged";
                self.primary_first_ts = self.bars.first().map(|bar| bar.ts_ms).unwrap_or(0);
                if old_bars_empty {
                    self.view_offset = self.bars.len().saturating_sub(1) + CHART_RIGHT_MARGIN;
                    self.manual_view_override = false;
                    self.reset_camera_from_legacy();
                } else {
                    self.camera.on_data_len_changed(old_len, self.bars.len());
                    self.sync_camera_to_legacy();
                }
                self.compute_indicators_gpu(gpu);
                return true;
            }
        }
        let mut keys_to_try = vec![
            format!("kraken:{}:{}", sym, tf),
            format!("kraken-equities:{}:{}", sym, tf),
            format!("kraken-futures:{}:{}", sym, tf),
            format!("tastytrade:{}:{}", sym, tf),
            format!("alpaca:{}:{}", sym, tf),
            format!("yahoo-chart:{}:{}", sym, tf),
        ];
        if sym_norm != sym {
            keys_to_try.extend([
                format!("kraken:{}:{}", sym_norm, tf),
                format!("kraken-equities:{}:{}", sym_norm, tf),
                format!("kraken-futures:{}:{}", sym_norm, tf),
                format!("tastytrade:{}:{}", sym_norm, tf),
                format!("alpaca:{}:{}", sym_norm, tf),
                format!("yahoo-chart:{}:{}", sym_norm, tf),
            ]);
        }
        let prefer_fresh_equity = chart_prefers_fresh_equity_source(&sym_norm);
        let native_equity_low_tf_only =
            prefer_fresh_equity && chart_equity_low_timeframe_requires_native_source(tf);
        let mut result: Option<(Vec<(i64, f64, f64, f64, f64, f64)>, bool, &'static str)> = None;
        let mut best_equity: Option<(
            Vec<(i64, f64, f64, f64, f64, f64)>,
            bool,
            &'static str,
            i64,
            u8,
        )> = None;
        for k in &keys_to_try {
            match cache.get_bars_raw(k) {
                Ok(Some(raw))
                    if !raw.is_empty()
                        && chart_source_bars_match_timeframe(
                            cache_source_from_key(k),
                            tf,
                            &raw,
                        ) =>
                {
                    let source = cache_source_from_key(k);
                    let is_gap_fill = k.starts_with("kraken:") || k.starts_with("kraken-futures:");
                    if prefer_fresh_equity {
                        if native_equity_low_tf_only && source != "kraken-equities" {
                            continue;
                        }
                        if let Some(rank) = chart_equity_source_rank(source) {
                            let last_ts = chart_bar_last_valid_ts(&raw);
                            let replace = best_equity
                                .as_ref()
                                .map(|(_, _, _, best_ts, best_rank)| {
                                    last_ts > *best_ts || (last_ts == *best_ts && rank < *best_rank)
                                })
                                .unwrap_or(true);
                            if replace {
                                best_equity = Some((raw, is_gap_fill, source, last_ts, rank));
                            }
                            continue;
                        }
                    }
                    result = Some((raw, is_gap_fill, source));
                    break;
                }
                _ => {}
            }
        }
        if result.is_none() {
            if let Some((raw, is_gap_fill, source, _, _)) = best_equity {
                result = Some((raw, is_gap_fill, source));
            }
        }
        if result.is_none() && tf == "1Month" {
            let monthly_sources = [
                "kraken",
                "kraken-equities",
                "kraken-futures",
                "tastytrade",
                "alpaca",
                "yahoo-chart",
                "default",
            ];
            for source in monthly_sources {
                for key in chart_source_cache_keys(source, &sym, "1Day") {
                    let Ok(Some(raw)) = cache.get_bars_raw(&key) else {
                        continue;
                    };
                    let monthly = Self::aggregate_daily_raw_to_monthly(raw);
                    if monthly.len() >= 2 {
                        self.bars = monthly;
                        self.primary_source = "merged";
                        self.primary_first_ts = self.bars.first().map(|bar| bar.ts_ms).unwrap_or(0);
                        self.gap_fill_timestamps.clear();
                        self.compute_indicators_gpu(gpu);
                        return true;
                    }
                }
            }
        }
        if let Some((raw, primary_is_gap_fill, primary_source)) = result {
            self.gap_fill_timestamps.clear();
            // Filter invalid bars (parity with load() at ~line 1842) — epoch-0 timestamps,
            // non-positive prices, NaN, or high<low would otherwise render as phantom
            // flat lines on the non-blocking UI hot path before load() runs.
            self.bars = raw
                .into_iter()
                .filter(|(ts, o, h, l, c, _v)| {
                    *ts > 0
                        && *o > 0.0
                        && *h > 0.0
                        && *l > 0.0
                        && *c > 0.0
                        && o.is_finite()
                        && h.is_finite()
                        && l.is_finite()
                        && c.is_finite()
                        && *h >= *l
                })
                .map(|(ts, o, h, l, c, v)| {
                    if primary_is_gap_fill {
                        self.gap_fill_timestamps.insert(ts);
                    }
                    Bar {
                        ts_ms: ts,
                        open: o,
                        high: h,
                        low: l,
                        close: c,
                        volume: v,
                    }
                })
                .collect();
            self.primary_source = if self.bars.is_empty() {
                ""
            } else {
                primary_source
            };

            // Track primary source range (bars before this are backfill)
            self.primary_first_ts = if primary_is_gap_fill {
                0
            } else {
                self.bars.first().map(|b| b.ts_ms).unwrap_or(0)
            };

            let mut gap_filled = 0usize;
            {
                // Merge provenance-tagged alternate-source bars without duplicating
                // the same D/W/M session. Providers do not agree on candle
                // timestamps: Kraken often uses 00:00 UTC, Alpaca/Yahoo US
                // equities use 04:00/05:00 UTC, and live daily candles can
                // arrive at close time. Use calendar buckets for higher
                // timeframes and offset aliases for intraday UTC+2/US
                // market-time variants.
                let tf_ms = match tf {
                    "4Hour" => 4 * 3_600_000,
                    "1Hour" => 3_600_000,
                    "30Min" => 1_800_000,
                    "15Min" => 900_000,
                    "5Min" => 300_000,
                    _ => 60_000,
                };
                let snap = |ts: i64| -> i64 {
                    match tf {
                        "1Month" => chrono::DateTime::from_timestamp_millis(ts)
                            .and_then(|dt| {
                                chrono::NaiveDate::from_ymd_opt(dt.year(), dt.month(), 1)
                                    .and_then(|d| d.and_hms_opt(0, 0, 0))
                            })
                            .map(|ndt| ndt.and_utc().timestamp_millis())
                            .unwrap_or(ts),
                        "1Week" => chrono::DateTime::from_timestamp_millis(ts)
                            .and_then(|dt| {
                                let days_since_mon = dt.weekday().num_days_from_monday() as i64;
                                (dt.date_naive() - chrono::Duration::days(days_since_mon))
                                    .and_hms_opt(0, 0, 0)
                            })
                            .map(|ndt| ndt.and_utc().timestamp_millis())
                            .unwrap_or(ts),
                        "1Day" => chrono::DateTime::from_timestamp_millis(ts)
                            .and_then(|dt| {
                                chrono::NaiveDate::from_ymd_opt(dt.year(), dt.month(), dt.day())
                                    .and_then(|d| d.and_hms_opt(0, 0, 0))
                            })
                            .map(|ndt| ndt.and_utc().timestamp_millis())
                            .unwrap_or(ts),
                        _ => ts / tf_ms * tf_ms,
                    }
                };
                let alias_offsets_ms: &[i64] = if matches!(tf, "1Day" | "1Week" | "1Month") {
                    &[0]
                } else {
                    &[
                        0,
                        2 * 3_600_000,
                        -2 * 3_600_000,
                        4 * 3_600_000,
                        -4 * 3_600_000,
                        5 * 3_600_000,
                        -5 * 3_600_000,
                    ]
                };
                let mut occupied: std::collections::HashSet<i64> = std::collections::HashSet::new();
                let mut primary_min_snapped: Option<i64> = None;
                let mut primary_max_snapped: Option<i64> = None;
                for b in self.bars.iter() {
                    let snapped = snap(b.ts_ms);
                    primary_min_snapped =
                        Some(primary_min_snapped.map_or(snapped, |min| min.min(snapped)));
                    primary_max_snapped =
                        Some(primary_max_snapped.map_or(snapped, |max| max.max(snapped)));
                    for offset in alias_offsets_ms {
                        occupied.insert(snap(b.ts_ms.saturating_add(*offset)));
                    }
                }
                self.gap_fill_timestamps.clear();
                // Try all alternate source prefixes for gap-fill (crypto slash variants too)
                let sym_slash = {
                    let s = sym.to_uppercase();
                    let crypto_bases = [
                        "BTC", "ETH", "SOL", "DOGE", "XRP", "ADA", "LTC", "LINK", "AVAX", "DOT",
                        "XMR", "ZEC", "DASH",
                    ];
                    crypto_bases
                        .iter()
                        .find_map(|base| {
                            if s.starts_with(base)
                                && s.ends_with("USD")
                                && s.len() == base.len() + 3
                            {
                                Some(format!("{}/USD", base))
                            } else {
                                None
                            }
                        })
                        .unwrap_or_default()
                };
                let gap_prefixes = [
                    "kraken",
                    "kraken-futures",
                    "alpaca",
                    "tastytrade",
                    "yahoo-chart",
                ];
                for prefix in &gap_prefixes {
                    // Try both SOLUSD and SOL/USD key forms
                    let keys_to_try: Vec<String> = if sym_slash.is_empty() {
                        vec![format!("{}:{}:{}", prefix, sym, tf)]
                    } else {
                        vec![
                            format!("{}:{}:{}", prefix, sym, tf),
                            format!("{}:{}:{}", prefix, sym_slash, tf),
                        ]
                    };
                    for gap_key in &keys_to_try {
                        if let Ok(Some(gap_raw)) = cache.get_bars_raw(gap_key) {
                            if !chart_source_bars_match_timeframe(
                                cache_source_from_key(gap_key),
                                tf,
                                &gap_raw,
                            ) {
                                continue;
                            }
                            for (ts, o, h, l, c, v) in gap_raw {
                                let snapped = snap(ts);
                                if !occupied.contains(&snapped)
                                    && chart_gap_fill_bar_allowed(
                                        primary_source,
                                        cache_source_from_key(gap_key),
                                        snapped,
                                        primary_min_snapped,
                                        primary_max_snapped,
                                    )
                                {
                                    for offset in alias_offsets_ms {
                                        occupied.insert(snap(ts.saturating_add(*offset)));
                                    }
                                    self.bars.push(Bar {
                                        ts_ms: ts,
                                        open: o,
                                        high: h,
                                        low: l,
                                        close: c,
                                        volume: v,
                                    });
                                    self.gap_fill_timestamps.insert(ts);
                                    gap_filled += 1;
                                }
                            }
                        }
                    }
                }
                if gap_filled > 0 {
                    self.bars.sort_by_key(|b| b.ts_ms);
                }
            }

            // Aggregate bars for custom timeframes (H2, D3, Y1, etc.)
            let agg_info = if let Some(factor) = self.timeframe.aggregation() {
                if factor > 1 && !self.bars.is_empty() {
                    let base_count = self.bars.len();
                    let mut aggregated = Vec::with_capacity(base_count / factor + 1);
                    let mut i = 0;
                    while i < self.bars.len() {
                        let end = (i + factor).min(self.bars.len());
                        let chunk = &self.bars[i..end];
                        let bar = Bar {
                            ts_ms: chunk[0].ts_ms,
                            open: chunk[0].open,
                            high: chunk.iter().map(|b| b.high).fold(f64::MIN, f64::max),
                            low: chunk.iter().map(|b| b.low).fold(f64::MAX, f64::min),
                            close: chunk[chunk.len() - 1].close,
                            volume: chunk.iter().map(|b| b.volume).sum(),
                        };
                        aggregated.push(bar);
                        i = end;
                    }
                    let agg_count = aggregated.len();
                    self.bars = aggregated;
                    format!(" ({}→{} aggregated ×{})", base_count, agg_count, factor)
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            let ltf_rebuilt = self.rebuild_from_lower_timeframe_if_dislocated(cache, &sym);
            let quote_overlaid = self.apply_quote_cache_overlay(cache, &sym);
            if old_bars_empty {
                self.view_offset = self.bars.len().saturating_sub(1) + CHART_RIGHT_MARGIN;
                self.manual_view_override = false;
                self.reset_camera_from_legacy();
            } else {
                self.camera.on_data_len_changed(old_len, self.bars.len());
                self.sync_camera_to_legacy();
            }
            self.compute_indicators_gpu(gpu);
            self.compute_mtf_sma(cache);
            self.compute_multi_kama(cache);
            let mtf_info = if !self.mtf_sma.is_empty() || !self.multi_kama.is_empty() {
                format!(
                    " | MTF_MA: {} lines, MultiKAMA: {} TFs",
                    self.mtf_sma.len(),
                    self.multi_kama.len()
                )
            } else {
                String::new()
            };
            let gap_info = if gap_filled > 0 {
                format!(" +{} gap-fill", gap_filled)
            } else if ltf_rebuilt {
                " +LTF rebuild +quote".to_string()
            } else if quote_overlaid {
                " +quote".to_string()
            } else {
                String::new()
            };
            log.push_back(LogEntry::info(format!(
                "Loaded {} bars for {} [{}]{}{}{}",
                self.bars.len(),
                self.symbol,
                self.timeframe.label(),
                agg_info,
                mtf_info,
                gap_info
            )));
        }
        true
    }

    /// Load bars from the shared cache, re-compute indicators.
    pub(crate) fn load(
        &mut self,
        cache: &SqliteCache,
        log: &mut VecDeque<LogEntry>,
        gpu: Option<&mut gpu_compute::GpuCompute>,
        dsm: &typhoon_engine::core::data_source::DataSourceManager,
    ) {
        let key = self.find_cache_key(cache, dsm);
        let key_source = cache_source_from_key(&key);
        let tf = self.timeframe.cache_suffix();

        // Extract bare symbol for multi-source lookup
        let bare_sym = {
            let parts: Vec<&str> = self.symbol.split(':').collect();
            let is_tf = matches!(
                parts.last().copied(),
                Some(
                    "1Min"
                        | "5Min"
                        | "15Min"
                        | "30Min"
                        | "1Hour"
                        | "4Hour"
                        | "1Day"
                        | "1Week"
                        | "1Month"
                )
            );
            let sym_parts = if is_tf && parts.len() > 1 {
                &parts[..parts.len() - 1]
            } else {
                &parts[..]
            };
            let s = sym_parts.last().copied().unwrap_or(&self.symbol);
            // Strip known prefixes
            let known = [
                "default:",
                "kraken-futures:",
                "kraken-equities:",
                "kraken:",
                "tastytrade:",
                "alpaca:",
                "yahoo-chart:",
                "paper_TyphooN:",
                "alpaca_paper_TyphooN:",
            ];
            let mut r = s;
            for pfx in &known {
                if r.starts_with(pfx) {
                    r = &r[pfx.len()..];
                    break;
                }
            }
            r.split(':').last().unwrap_or(r).replace('/', "")
        };

        if chart_prefers_fresh_equity_source(&bare_sym) {
            let (load_started_at, load_rss_before) = chart_log_merged_cache_load_start(
                log,
                "restored_cache",
                &bare_sym,
                self.timeframe.label(),
            );
            let merged = chart_load_merged_equity_bars_from_cache(cache, &bare_sym, tf);
            chart_log_merged_cache_load_done(
                log,
                "restored_cache",
                &bare_sym,
                self.timeframe.label(),
                merged.len(),
                load_started_at,
                load_rss_before,
            );
            if !merged.is_empty() {
                self.gap_fill_timestamps.clear();
                self.bars = merged;
                self.primary_source = "merged";
                self.primary_first_ts = self.bars.first().map(|bar| bar.ts_ms).unwrap_or(0);
                self.compute_indicators_gpu(gpu);
                self.compute_mtf_sma(cache);
                self.compute_multi_kama(cache);
                return;
            }
        }

        // Load primary source (filter invalid bars at read time)
        match cache.get_bars_raw(&key) {
            Ok(Some(raw)) if chart_source_bars_match_timeframe(key_source, tf, &raw) => {
                self.bars = raw
                    .into_iter()
                    .filter(|(ts, o, h, l, c, _v)| {
                        *ts > 0 && *o > 0.0 && *h > 0.0 && *l > 0.0 && *c > 0.0 && *h >= *l
                    })
                    .map(|(ts, o, h, l, c, v)| Bar {
                        ts_ms: ts,
                        open: o,
                        high: h,
                        low: l,
                        close: c,
                        volume: v,
                    })
                    .collect();
                self.primary_source = if self.bars.is_empty() { "" } else { key_source };
            }
            Ok(Some(_)) | Ok(None) => {
                self.bars.clear();
                self.primary_source = "";
                if tf == "1Month" {
                    for source in [
                        "kraken",
                        "kraken-equities",
                        "kraken-futures",
                        "tastytrade",
                        "alpaca",
                        "yahoo-chart",
                        "default",
                    ] {
                        for daily_key in chart_source_cache_keys(source, &bare_sym, "1Day") {
                            let Ok(Some(raw)) = cache.get_bars_raw(&daily_key) else {
                                continue;
                            };
                            let monthly = Self::aggregate_daily_raw_to_monthly(raw);
                            if monthly.len() >= 2 {
                                self.bars = monthly;
                                self.primary_source = "merged";
                                break;
                            }
                        }
                        if !self.bars.is_empty() {
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                self.bars.clear();
                self.primary_source = "";
                log.push_back(LogEntry::err(format!("Cache read error: {e}")));
            }
        }

        // Merge gap-fill sources: Kraken spot/xStocks and Futures
        // For crypto: merge ALL bars (fill gaps anywhere, not just append to end)
        let crypto_bases = [
            "BTC", "ETH", "SOL", "DOGE", "XRP", "ADA", "LTC", "LINK", "AVAX", "DOT", "XMR", "ZEC",
            "DASH", "UNI", "AAVE", "MATIC", "SHIB", "ATOM", "ALGO", "FTM", "NEAR", "APE", "ARB",
        ];
        let sym_upper = bare_sym.to_uppercase();
        let is_crypto = crypto_bases
            .iter()
            .any(|b| sym_upper.starts_with(b) && sym_upper.ends_with("USD"));

        if is_crypto {
            self.gap_fill_timestamps.clear();
            // Snap timestamps to TF boundary for dedup (handles per-source TZ offsets vs Kraken UTC)
            // Weekly: snap to Monday 00:00 UTC. Monthly: snap to 1st of month 00:00 UTC.
            let tf_ms: i64 = match tf {
                "1Day" => 86_400_000,
                "4Hour" => 4 * 3_600_000,
                "1Hour" => 3_600_000,
                "30Min" => 1_800_000,
                "15Min" => 900_000,
                "5Min" => 300_000,
                _ => 60_000,
            };
            let snap = |ts: i64| -> i64 {
                match tf {
                    "1Month" => {
                        // Snap to 1st of month 00:00 UTC
                        let dt = chrono::DateTime::from_timestamp(ts / 1000, 0).unwrap_or_default();
                        chrono::NaiveDate::from_ymd_opt(dt.year(), dt.month(), 1)
                            .and_then(|d| d.and_hms_opt(0, 0, 0))
                            .map(|ndt| ndt.and_utc().timestamp() * 1000)
                            .unwrap_or(ts / tf_ms * tf_ms)
                    }
                    "1Week" => {
                        // Snap to Monday 00:00 UTC
                        let dt = chrono::DateTime::from_timestamp(ts / 1000, 0).unwrap_or_default();
                        let days_since_mon = dt.weekday().num_days_from_monday() as i64;
                        let mon = dt.date_naive() - chrono::Duration::days(days_since_mon);
                        mon.and_hms_opt(0, 0, 0)
                            .map(|ndt| ndt.and_utc().timestamp() * 1000)
                            .unwrap_or(ts / (7 * 86_400_000) * (7 * 86_400_000))
                    }
                    _ => ts / tf_ms * tf_ms,
                }
            };
            let mut existing_snapped: std::collections::HashSet<i64> =
                self.bars.iter().map(|b| snap(b.ts_ms)).collect();

            let kr_key = format!("kraken:{}:{}", bare_sym, tf);
            if let Ok(Some(raw)) = cache.get_bars_raw(&kr_key) {
                let mut merged = 0;
                for (ts, o, h, l, c, v) in raw {
                    if o <= 0.0 || h <= 0.0 || l <= 0.0 || c <= 0.0 || h < l {
                        continue;
                    }
                    let snapped = snap(ts);
                    if !existing_snapped.contains(&snapped) {
                        self.bars.push(Bar {
                            ts_ms: ts,
                            open: o,
                            high: h,
                            low: l,
                            close: c,
                            volume: v,
                        });
                        self.gap_fill_timestamps.insert(ts);
                        existing_snapped.insert(snapped);
                        merged += 1;
                    }
                }
                if merged > 0 {
                    log.push_back(LogEntry::info(format!(
                        "  +{} bars from Kraken weekend fill",
                        merged
                    )));
                }
            }

            let kr_fut_key = format!("kraken-futures:{}:{}", bare_sym, tf);
            if let Ok(Some(raw)) = cache.get_bars_raw(&kr_fut_key) {
                let mut merged = 0;
                for (ts, o, h, l, c, v) in raw {
                    if o <= 0.0 || h <= 0.0 || l <= 0.0 || c <= 0.0 || h < l {
                        continue;
                    }
                    let snapped = snap(ts);
                    if !existing_snapped.contains(&snapped) {
                        self.bars.push(Bar {
                            ts_ms: ts,
                            open: o,
                            high: h,
                            low: l,
                            close: c,
                            volume: v,
                        });
                        self.gap_fill_timestamps.insert(ts);
                        existing_snapped.insert(snapped);
                        merged += 1;
                    }
                }
                if merged > 0 {
                    log.push_back(LogEntry::info(format!(
                        "  +{} bars from Kraken Futures fill",
                        merged
                    )));
                }
            }

            // Sort merged bars by timestamp (sources may interleave)
            if !self.bars.is_empty() {
                self.bars.sort_by_key(|b| b.ts_ms);
            }
        }

        // Remove any bars with invalid prices (negative, zero, NaN, or obviously wrong)
        // Runs unconditionally on ALL bars from ALL sources
        {
            let pre_filter = self.bars.len();
            self.bars.retain(|b| {
                b.open > 0.0
                    && b.high > 0.0
                    && b.low > 0.0
                    && b.close > 0.0
                    && b.open.is_finite()
                    && b.high.is_finite()
                    && b.low.is_finite()
                    && b.close.is_finite()
                    && b.high >= b.low
                    && b.ts_ms > 0
            });
            if self.bars.len() < pre_filter {
                log.push_back(LogEntry::warn(format!(
                    "  Filtered {} invalid bars (negative/zero/NaN/bad prices)",
                    pre_filter - self.bars.len()
                )));
            }
        }

        // Synthesize a current forming bar from lower-timeframe data only when
        // the existing HTF series is caught up through the previous bucket. If
        // the primary source is several sessions stale, aggregating every newer
        // LTF candle into one bar creates a fake multi-day monster candle.
        if !self.bars.is_empty() && self.timeframe.minutes() > 5 {
            let last_ts = self.bars.last().map(|b| b.ts_ms).unwrap_or(0);
            let tf_ms = self.timeframe.minutes() as i64 * 60 * 1000;
            let now_ms = chrono::Utc::now().timestamp_millis();
            if chart_forming_bar_allowed(last_ts, now_ms, tf_ms) {
                let current_bucket = now_ms / tf_ms * tf_ms;
                // Try M5 first, then M1
                // Cascade through all lower timeframes from all sources for best resolution
                let src = self.symbol.split(':').next().unwrap_or("kraken");
                let ltf_suffixes = [
                    "1Min", "5Min", "15Min", "30Min", "1Hour", "4Hour", "1Day", "1Week",
                ];
                let sources = ["kraken", "kraken-futures", src, "kraken-equities", "alpaca"];
                let mut ltf_keys = Vec::new();
                for ltf in &ltf_suffixes {
                    let ltf_min: u32 = match *ltf {
                        "1Min" => 1,
                        "5Min" => 5,
                        "15Min" => 15,
                        "30Min" => 30,
                        "1Hour" => 60,
                        "4Hour" => 240,
                        "1Day" => 1440,
                        _ => 10080,
                    };
                    if ltf_min < self.timeframe.minutes() {
                        for s in &sources {
                            ltf_keys.push(format!("{}:{}:{}", s, bare_sym, ltf));
                        }
                    }
                }
                for ltf_key in &ltf_keys {
                    if let Ok(Some(ltf_raw)) = cache.get_bars_raw(ltf_key) {
                        // Find LTF bars inside the current HTF bucket only.
                        let forming_start = current_bucket;
                        let forming_end = forming_start.saturating_add(tf_ms);
                        let newer: Vec<_> = ltf_raw
                            .iter()
                            .filter(|(ts, _, _, _, _, _)| *ts >= forming_start && *ts < forming_end)
                            .collect();
                        if !newer.is_empty() {
                            let open = newer.first().map(|(_, o, _, _, _, _)| *o).unwrap_or(0.0);
                            let high = newer
                                .iter()
                                .map(|(_, _, h, _, _, _)| *h)
                                .fold(f64::NEG_INFINITY, f64::max);
                            let low = newer
                                .iter()
                                .map(|(_, _, _, l, _, _)| *l)
                                .fold(f64::INFINITY, f64::min);
                            let close = newer.last().map(|(_, _, _, _, c, _)| *c).unwrap_or(0.0);
                            let volume: f64 = newer.iter().map(|(_, _, _, _, _, v)| *v).sum();
                            self.bars.push(Bar {
                                ts_ms: forming_start,
                                open,
                                high,
                                low,
                                close,
                                volume,
                            });
                            log.push_back(LogEntry::info(format!(
                                "  +1 forming bar from {} LTF bars",
                                newer.len()
                            )));
                            break;
                        }
                    }
                }
            }
        }

        let ltf_rebuilt = self.rebuild_from_lower_timeframe_if_dislocated(cache, &bare_sym);
        let quote_overlaid = self.apply_quote_cache_overlay(cache, &bare_sym);

        if self.bars.is_empty() {
            log.push_back(LogEntry::warn(format!(
                "No chart data found for key '{}'",
                key
            )));
        } else {
            self.view_offset = self.bars.len().saturating_sub(1) + CHART_RIGHT_MARGIN;
            self.compute_indicators_gpu(gpu);
            self.compute_mtf_sma(cache);
            self.compute_multi_kama(cache);
            let mtf_info = if !self.mtf_sma.is_empty() || !self.multi_kama.is_empty() {
                format!(
                    " | MTF_MA: {} lines, MultiKAMA: {} TFs",
                    self.mtf_sma.len(),
                    self.multi_kama.len()
                )
            } else {
                String::new()
            };
            let quote_info = if ltf_rebuilt {
                " +LTF rebuild +quote"
            } else if quote_overlaid {
                " +quote"
            } else {
                ""
            };
            log.push_back(LogEntry::info(format!(
                "Loaded {} bars for {} [{}]{}{}",
                self.bars.len(),
                self.symbol,
                self.timeframe.label(),
                mtf_info,
                quote_info
            )));
        }
        // Steady-state cap of 200 (was 500). Console log is diagnostic, not forensic —
        // keep it tight to avoid frame jank during bulk imports that push dozens of lines.
        while log.len() > 200 {
            log.pop_front();
        }
    }

    pub(crate) fn compute_indicators(&mut self) {
        self.compute_indicators_gpu(None);
    }

    pub(crate) fn compute_indicators_gpu(&mut self, gpu: Option<&mut gpu_compute::GpuCompute>) {
        let n = self.bars.len();
        let forming_bar_dirty_at_entry = self.forming_bar_dirty;
        // Cache reloads replace `bars` with the last persisted candle, which can lag the
        // live quote already shown in the watchlist/position panels. Fold the fresh live
        // mid back into the last bar before either the incremental or full GPU path so a
        // reload cannot make the active forming candle jump backward until the next tick.
        let live_quote_folded = self.fold_fresh_live_quote_into_forming_bar();
        if live_quote_folded && !forming_bar_dirty_at_entry {
            self.forming_bar_dirty = false;
        }

        // Forming-bar fast path: only update the last value of indicators
        // instead of full recompute + GPU upload. This is the key integration
        // point between our WS fast-path and the GPU compute path.
        // O(1) path for SMA/EMA (with hoisted close); stateful indicators
        // (KAMA, RSI, MACD, ATR, ...) intentionally fall through to the next
        // structural change (new closed bar) for full GPU dispatch.
        if forming_bar_dirty_at_entry && n > 1 {
            if let Some(last) = self.bars.last_mut() {
                let mut close = last.close;

                // When live quotes are present, fold the live mid into the forming bar so
                // the candle grows with real-time data (prevents the stale/grey candle).
                let has_live_quotes = self.live_bid > 0.0 && self.live_ask > 0.0;
                if has_live_quotes {
                    let mid = (self.live_bid + self.live_ask) * 0.5;
                    last.close = mid;
                    last.high = last.high.max(mid);
                    last.low = last.low.min(mid);
                    close = mid;
                }

                if let Some(gpu) = gpu {
                    let is_live = if self.live_bid > 0.0 && self.live_ask > 0.0 {
                        1.0
                    } else {
                        0.0
                    };
                    gpu.upload_forming_bar(
                        last.open as f32,
                        last.high as f32,
                        last.low as f32,
                        close as f32,
                        last.volume as f32,
                        is_live,
                    );
                }

                // Indicator rolling updates still happen (they only need the close value)
                // For SMA200 / SMA100 we can do a cheap rolling update
                let prev200 = self.sma200.get(n - 2).copied().flatten();
                if let (Some(last_sma200), Some(prev)) = (self.sma200.last_mut(), prev200) {
                    *last_sma200 = Some(
                        (prev * (self.sma_slow_period as f64 - 1.0) + close)
                            / self.sma_slow_period as f64,
                    );
                }
                let prev100 = self.sma100.get(n - 2).copied().flatten();
                if let (Some(last_sma100), Some(prev)) = (self.sma100.last_mut(), prev100) {
                    *last_sma100 = Some(
                        (prev * (self.sma_fast_period as f64 - 1.0) + close)
                            / self.sma_fast_period as f64,
                    );
                }
                // EMA21 fast-path last-value update (O(1) rolling)
                let ema_p = self.ema_period as f64;
                let k = 2.0 / (ema_p + 1.0);
                let prev_ema = self.ema21.get(n - 2).copied().flatten();
                if let (Some(last_ema), Some(prev)) = (self.ema21.last_mut(), prev_ema) {
                    *last_ema = Some(close * k + prev * (1.0 - k));
                }
            }
            self.forming_bar_dirty = false; // consumed
            return;
        }

        // ── GPU path: upload bars to VRAM, compute on GPU, read back ──
        if let Some(gpu) = gpu {
            if n > 0 {
                // Reuse upload buffers to avoid repeated allocations
                if self.upload_opens.len() < n {
                    self.upload_opens = Vec::with_capacity(n);
                    self.upload_closes = Vec::with_capacity(n);
                    self.upload_highs = Vec::with_capacity(n);
                    self.upload_lows = Vec::with_capacity(n);
                    self.upload_volumes = Vec::with_capacity(n);
                }
                self.upload_opens.clear();
                self.upload_closes.clear();
                self.upload_highs.clear();
                self.upload_lows.clear();
                self.upload_volumes.clear();
                for b in &self.bars {
                    self.upload_opens.push(b.open as f32);
                    self.upload_closes.push(b.close as f32);
                    self.upload_highs.push(b.high as f32);
                    self.upload_lows.push(b.low as f32);
                    self.upload_volumes.push(b.volume as f32);
                }
                gpu.upload_bars_full(
                    &self.upload_opens,
                    &self.upload_closes,
                    &self.upload_highs,
                    &self.upload_lows,
                    &self.upload_volumes,
                );

                // Update snapshot so the draw_chart early-out works correctly after GPU path
                self.last_rendered_gen = self.visible_bars_gen;
                self.last_rendered_bar_ts = self.last_visible_bar_ts;

                // SMA — parallel GPU
                let sma_slow = self.sma_slow_period;
                let sma_fast = self.sma_fast_period;
                // Prefer dedicated compute_sma_gpu when available
                if let Some(data) = gpu.compute_sma_gpu(sma_slow, 0, self.bars.len() as u32) {
                    self.sma200 = data
                        .iter()
                        .map(|&v| if v == 0.0 { None } else { Some(v as f64) })
                        .collect();
                } else if let Some(data) =
                    gpu.dispatch_indicator_pub(&gpu_compute::Indicator::Sma, sma_slow, true)
                {
                    self.sma200 = data
                        .iter()
                        .map(|&v| if v == 0.0 { None } else { Some(v as f64) })
                        .collect();
                } else {
                    self.sma200 = compute_sma(&self.bars, sma_slow as usize);
                }

                if let Some(data) =
                    gpu.dispatch_indicator_pub(&gpu_compute::Indicator::Sma, sma_fast, true)
                {
                    self.sma100 = data
                        .iter()
                        .map(|&v| if v == 0.0 { None } else { Some(v as f64) })
                        .collect();
                } else {
                    self.sma100 = compute_sma(&self.bars, sma_fast as usize);
                }

                // KAMA — sequential GPU
                if let Some(data) = gpu.compute_kama_gpu(10) {
                    self.kama = data
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| if i < 10 { None } else { Some(v as f64) })
                        .collect();
                } else if let Some(data) =
                    gpu.dispatch_indicator_pub(&gpu_compute::Indicator::Kama, 10, false)
                {
                    self.kama = data
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| if i < 10 { None } else { Some(v as f64) })
                        .collect();
                } else {
                    self.kama = compute_kama(&self.bars, 10, 2, 30);
                }

                // EMA — sequential GPU
                let ema_p = self.ema_period;
                if let Some(data) =
                    gpu.dispatch_indicator_pub(&gpu_compute::Indicator::Ema, ema_p, false)
                {
                    self.ema21 = data
                        .iter()
                        .map(|&v| if v == 0.0 { None } else { Some(v as f64) })
                        .collect();
                } else {
                    self.ema21 = compute_ema(&self.bars, ema_p as usize);
                }

                // Bollinger — parallel GPU
                let bb_p = self.bb_period;
                if let Some(data) = gpu.compute_bollinger_gpu(bb_p) {
                    let mut mid = Vec::with_capacity(n);
                    let mut upper = Vec::with_capacity(n);
                    let mut lower = Vec::with_capacity(n);
                    for i in 0..n {
                        let m = data.get(i * 3).copied().unwrap_or(0.0);
                        let u = data.get(i * 3 + 1).copied().unwrap_or(0.0);
                        let l = data.get(i * 3 + 2).copied().unwrap_or(0.0);
                        if m == 0.0 {
                            mid.push(None);
                            upper.push(None);
                            lower.push(None);
                        } else {
                            mid.push(Some(m as f64));
                            upper.push(Some(u as f64));
                            lower.push(Some(l as f64));
                        }
                    }
                    self.bb_mid = mid;
                    self.bb_upper = upper;
                    self.bb_lower = lower;
                } else {
                    let (m, u, l) = compute_bollinger(&self.bars, bb_p as usize, 2.0);
                    self.bb_mid = m;
                    self.bb_upper = u;
                    self.bb_lower = l;
                }

                // RSI — sequential GPU
                let rsi_p = self.rsi_period;
                // Prefer dedicated RSI GPU path, then generic dispatch, then CPU
                if let Some(data) = gpu.compute_rsi_gpu(rsi_p) {
                    self.rsi = data
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| {
                            if i < rsi_p as usize || v == 0.0 {
                                None
                            } else {
                                Some(v as f64)
                            }
                        })
                        .collect();
                } else if let Some(data) =
                    gpu.dispatch_indicator_pub(&gpu_compute::Indicator::Rsi, rsi_p, false)
                {
                    self.rsi = data
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| {
                            if i < rsi_p as usize || v == 0.0 {
                                None
                            } else {
                                Some(v as f64)
                            }
                        })
                        .collect();
                } else {
                    self.rsi = compute_rsi(&self.bars, rsi_p as usize);
                }

                // Fisher — sequential GPU (uses midpoints)
                let fisher_p = self.fisher_period;
                if let Some(data) = gpu.compute_fisher_gpu(fisher_p) {
                    let mut f = Vec::with_capacity(n);
                    let mut fs = Vec::with_capacity(n);
                    for i in 0..n {
                        let fv = data.get(i * 2).copied().unwrap_or(0.0);
                        let sv = data.get(i * 2 + 1).copied().unwrap_or(0.0);
                        if i < fisher_p as usize || (fv == 0.0 && sv == 0.0) {
                            f.push(None);
                            fs.push(None);
                        } else {
                            f.push(Some(fv as f64));
                            fs.push(Some(sv as f64));
                        }
                    }
                    self.fisher = f;
                    self.fisher_signal = fs;
                } else if let Some(data) =
                    gpu.dispatch_indicator_pub(&gpu_compute::Indicator::Fisher, fisher_p, true)
                {
                    let mut f = Vec::with_capacity(n);
                    let mut fs = Vec::with_capacity(n);
                    for i in 0..n {
                        let fv = data.get(i * 2).copied().unwrap_or(0.0);
                        let sv = data.get(i * 2 + 1).copied().unwrap_or(0.0);
                        if i < fisher_p as usize || (fv == 0.0 && sv == 0.0) {
                            f.push(None);
                            fs.push(None);
                        } else {
                            f.push(Some(fv as f64));
                            fs.push(Some(sv as f64));
                        }
                    }
                    self.fisher = f;
                    self.fisher_signal = fs;
                } else {
                    let (f, fs) = compute_fisher(&self.bars, fisher_p as usize);
                    self.fisher = f;
                    self.fisher_signal = fs;
                }

                // ATR — sequential GPU (uses OHLC)
                let atr_p = self.atr_period;
                if let Some(data) = gpu.compute_atr_gpu(atr_p) {
                    self.atr = data
                        .iter()
                        .map(|&v| if v == 0.0 { None } else { Some(v as f64) })
                        .collect();
                } else if let Some(data) =
                    gpu.dispatch_ohlc_indicator_pub(&gpu_compute::Indicator::Atr, atr_p, 1)
                {
                    self.atr = data
                        .iter()
                        .map(|&v| if v == 0.0 { None } else { Some(v as f64) })
                        .collect();
                } else {
                    self.atr = compute_atr(&self.bars, atr_p as usize);
                }

                // MACD — sequential GPU with dynamic periods
                if let Some(data) =
                    gpu.compute_macd_gpu_dynamic(self.macd_fast, self.macd_slow, self.macd_signal_p)
                {
                    // Reuse existing Vec allocations (clear + refill instead of new Vec)
                    self.macd_line.clear();
                    self.macd_signal.clear();
                    self.macd_hist.clear();
                    self.macd_line.reserve(n);
                    self.macd_signal.reserve(n);
                    self.macd_hist.reserve(n);
                    for i in 0..n {
                        let l = data.get(i * 3).copied().unwrap_or(0.0);
                        let s = data.get(i * 3 + 1).copied().unwrap_or(0.0);
                        let h = data.get(i * 3 + 2).copied().unwrap_or(0.0);
                        if i < self.macd_slow as usize || (l == 0.0 && s == 0.0 && h == 0.0) {
                            self.macd_line.push(None);
                            self.macd_signal.push(None);
                            self.macd_hist.push(None);
                        } else {
                            self.macd_line.push(Some(l as f64));
                            self.macd_signal.push(Some(s as f64));
                            self.macd_hist.push(Some(h as f64));
                        }
                    }
                } else {
                    let (ml, ms, mh) = compute_macd(
                        &self.bars,
                        self.macd_fast as usize,
                        self.macd_slow as usize,
                        self.macd_signal_p as usize,
                    );
                    self.macd_line = ml;
                    self.macd_signal = ms;
                    self.macd_hist = mh;
                }

                // Stochastic — sequential GPU (uses OHLC, warmup: period+3 bars)
                let stoch_p = self.stoch_period;
                if let Some(data) = gpu.compute_stochastic_gpu(stoch_p) {
                    let mut sk = Vec::with_capacity(n);
                    let mut sd = Vec::with_capacity(n);
                    for i in 0..n {
                        if i < stoch_p as usize {
                            sk.push(None);
                            sd.push(None);
                        } else {
                            sk.push(Some(data.get(i * 2).copied().unwrap_or(50.0) as f64));
                            sd.push(Some(data.get(i * 2 + 1).copied().unwrap_or(50.0) as f64));
                        }
                    }
                    self.stoch_k = sk;
                    self.stoch_d = sd;
                } else if let Some(data) =
                    gpu.dispatch_indicator_pub(&gpu_compute::Indicator::Stochastic, stoch_p, true)
                {
                    let mut sk = Vec::with_capacity(n);
                    let mut sd = Vec::with_capacity(n);
                    for i in 0..n {
                        if i < stoch_p as usize {
                            sk.push(None);
                            sd.push(None);
                        } else {
                            sk.push(Some(data.get(i * 2).copied().unwrap_or(50.0) as f64));
                            sd.push(Some(data.get(i * 2 + 1).copied().unwrap_or(50.0) as f64));
                        }
                    }
                    self.stoch_k = sk;
                    self.stoch_d = sd;
                } else {
                    let (sk, sd) = compute_stochastic(&self.bars, stoch_p as usize, 3, 3);
                    self.stoch_k = sk;
                    self.stoch_d = sd;
                }

                // ADX — sequential GPU (uses OHLC, warmup: 2×period bars)
                let adx_p = self.adx_period;
                if let Some(data) = gpu.compute_adx_gpu(adx_p) {
                    let mut adx = Vec::with_capacity(n);
                    let mut dip = Vec::with_capacity(n);
                    let mut dim = Vec::with_capacity(n);
                    for i in 0..n {
                        let a = data.get(i * 3).copied().unwrap_or(0.0);
                        let dp = data.get(i * 3 + 1).copied().unwrap_or(0.0);
                        let dm = data.get(i * 3 + 2).copied().unwrap_or(0.0);
                        let di_warmup = adx_p as usize;
                        let adx_warmup = (adx_p as usize * 2).saturating_sub(1);
                        dip.push(if i < di_warmup || (dp == 0.0 && dm == 0.0) {
                            None
                        } else {
                            Some(dp as f64)
                        });
                        dim.push(if i < di_warmup || (dp == 0.0 && dm == 0.0) {
                            None
                        } else {
                            Some(dm as f64)
                        });
                        adx.push(if i < adx_warmup || a == 0.0 {
                            None
                        } else {
                            Some(a as f64)
                        });
                    }
                    self.adx = adx;
                    self.di_plus = dip;
                    self.di_minus = dim;
                } else if let Some(data) =
                    gpu.dispatch_indicator_pub(&gpu_compute::Indicator::Adx, adx_p, true)
                {
                    let mut adx = Vec::with_capacity(n);
                    let mut dip = Vec::with_capacity(n);
                    let mut dim = Vec::with_capacity(n);
                    for i in 0..n {
                        let a = data.get(i * 3).copied().unwrap_or(0.0);
                        let dp = data.get(i * 3 + 1).copied().unwrap_or(0.0);
                        let dm = data.get(i * 3 + 2).copied().unwrap_or(0.0);
                        let di_warmup = adx_p as usize;
                        let adx_warmup = (adx_p as usize * 2).saturating_sub(1);
                        dip.push(if i < di_warmup || (dp == 0.0 && dm == 0.0) {
                            None
                        } else {
                            Some(dp as f64)
                        });
                        dim.push(if i < di_warmup || (dp == 0.0 && dm == 0.0) {
                            None
                        } else {
                            Some(dm as f64)
                        });
                        adx.push(if i < adx_warmup || a == 0.0 {
                            None
                        } else {
                            Some(a as f64)
                        });
                    }
                    self.adx = adx;
                    self.di_plus = dip;
                    self.di_minus = dim;
                } else {
                    let (adx, dip, dim) = compute_adx(&self.bars, adx_p as usize);
                    self.adx = adx;
                    self.di_plus = dip;
                    self.di_minus = dim;
                }

                // Remaining indicators — GPU where shader exists, CPU fallback

                // Ichimoku — GPU (sequential, 4 outputs per bar)
                // Warmup: Tenkan=8, Kijun=25, SpanA=51, SpanB=77 bars
                if let Some(data) = gpu.compute_ichimoku_gpu() {
                    let n = self.bars.len();
                    let mut tk = Vec::with_capacity(n);
                    let mut kj = Vec::with_capacity(n);
                    let mut sa = Vec::with_capacity(n);
                    let mut sb = Vec::with_capacity(n);
                    for i in 0..n {
                        let t = data.get(i * 4).copied().unwrap_or(0.0);
                        let k = data.get(i * 4 + 1).copied().unwrap_or(0.0);
                        let a = data.get(i * 4 + 2).copied().unwrap_or(0.0);
                        let b = data.get(i * 4 + 3).copied().unwrap_or(0.0);
                        tk.push(if i < 9 { None } else { Some(t as f64) });
                        kj.push(if i < 26 { None } else { Some(k as f64) });
                        sa.push(if i < 52 { None } else { Some(a as f64) });
                        sb.push(if i < 52 { None } else { Some(b as f64) });
                    }
                    self.ichi_tenkan = tk;
                    self.ichi_kijun = kj;
                    self.ichi_span_a = sa;
                    self.ichi_span_b = sb;
                } else {
                    let (tk, kj, sa, sb) = compute_ichimoku(&self.bars, 9, 26, 52);
                    self.ichi_tenkan = tk;
                    self.ichi_kijun = kj;
                    self.ichi_span_a = sa;
                    self.ichi_span_b = sb;
                }

                // WMA — GPU (parallel)
                if let Some(data) = gpu.compute_wma_gpu(20) {
                    self.wma = data
                        .iter()
                        .map(|&v| if v == 0.0 { None } else { Some(v as f64) })
                        .collect();
                } else if let Some(data) =
                    gpu.dispatch_indicator_pub(&gpu_compute::Indicator::Wma, 20, false)
                {
                    self.wma = data
                        .iter()
                        .map(|&v| if v == 0.0 { None } else { Some(v as f64) })
                        .collect();
                } else {
                    self.wma = compute_wma(&self.bars, 20);
                }

                // HMA — GPU (WMA composition shader)
                if let Some(data) = gpu.compute_hma_gpu(20) {
                    self.hma = data
                        .iter()
                        .map(|&v| if v == 0.0 { None } else { Some(v as f64) })
                        .collect();
                } else if let Some(data) =
                    gpu.dispatch_indicator_pub(&gpu_compute::Indicator::Hma, 20, false)
                {
                    self.hma = data
                        .iter()
                        .map(|&v| if v == 0.0 { None } else { Some(v as f64) })
                        .collect();
                } else {
                    self.hma = compute_hma(&self.bars, 20);
                }

                // CCI — GPU (parallel, from OHLC, warmup: period-1 bars)
                if let Some(data) = gpu.compute_cci_gpu(20) {
                    self.cci = data
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| if i < 19 { None } else { Some(v as f64) })
                        .collect();
                } else {
                    self.cci = compute_cci(&self.bars, 20);
                }

                // Williams %R — GPU (parallel, from OHLC, first valid at period-1)
                if let Some(data) = gpu.compute_williams_r_gpu(14) {
                    self.williams_r = data
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| if i < 13 { None } else { Some(v as f64) })
                        .collect();
                } else {
                    self.williams_r = compute_williams_r(&self.bars, 14);
                }

                // OBV — GPU (sequential, resident close + volume buffers)
                if let Some(data) = gpu.compute_obv_gpu() {
                    self.obv = data.iter().map(|&v| Some(v as f64)).collect();
                } else {
                    self.obv = compute_obv(&self.bars);
                }

                // Momentum — GPU (parallel, oscillator — 0.0 is valid)
                let mom_p = self.momentum_period;
                if let Some(data) = gpu.compute_momentum_gpu(mom_p) {
                    self.momentum = data
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| {
                            if i < mom_p as usize {
                                None
                            } else {
                                Some(v as f64)
                            }
                        })
                        .collect();
                } else {
                    self.momentum = compute_momentum(&self.bars, mom_p as usize);
                }

                // Simple O(1) forming-bar update for Momentum (approximate)
                if self.forming_bar_dirty && n > 1 && mom_p as usize > 0 {
                    if let Some(prev_mom) = self.momentum.get(n - 2).copied().flatten() {
                        if let Some(last_mom) = self.momentum.last_mut() {
                            if let Some(last) = self.bars.last() {
                                // Approximate: shift by the change in close
                                let change = last.close - self.bars[n - 2].close;
                                *last_mom = Some(prev_mom + change);
                            }
                        }
                    }
                }

                // Simple O(1) forming-bar update for Rate of Change (approximate)
                if self.forming_bar_dirty && n > 1 && mom_p as usize > 0 {
                    if let Some(_prev_roc) = self.momentum.get(n - 2).copied().flatten() {
                        if let Some(last_roc) = self.momentum.last_mut() {
                            if let Some(last) = self.bars.last() {
                                let prev_close = self.bars[n - 2].close;
                                if prev_close != 0.0 {
                                    let new_roc = ((last.close - prev_close) / prev_close) * 100.0;
                                    *last_roc = Some(new_roc);
                                }
                            }
                        }
                    }
                }

                // O(1) forming-bar update for Linear Regression Intercept
                if self.forming_bar_dirty && n > 1 {
                    if let Some(last_slope) = self.linreg_slope.get(n - 2).copied().flatten() {
                        if let Some(last_intercept) = self.linreg_intercept.last_mut() {
                            if let Some(last) = self.bars.last() {
                                // intercept = y - slope * x  (using current bar as reference)
                                let x = (n - 1) as f64;
                                *last_intercept = Some(last.close - last_slope * x);
                            }
                        }
                    }
                }

                // Simple O(1) forming-bar update for Chande Forecast Oscillator (CFO)
                if self.forming_bar_dirty && n > 1 {
                    if let Some(last_slope) = self.linreg_slope.get(n - 2).copied().flatten() {
                        if let Some(last_intercept) =
                            self.linreg_intercept.get(n - 2).copied().flatten()
                        {
                            if let Some(last_cfo) = self.cmo.last_mut() {
                                if let Some(last) = self.bars.last() {
                                    let x = (n - 1) as f64;
                                    let forecast = last_slope * x + last_intercept;
                                    if last.close != 0.0 {
                                        *last_cfo =
                                            Some(100.0 * (last.close - forecast) / last.close);
                                    }
                                }
                            }
                        }
                    }
                }

                // CMO / QStick / Disparity / BOP / StdDev — GPU with CPU fallback
                let cmo_p = 9u32;
                if let Some(data) = gpu.compute_cmo_gpu(cmo_p) {
                    self.cmo = data
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| {
                            if i < cmo_p as usize {
                                None
                            } else {
                                Some(v as f64)
                            }
                        })
                        .collect();
                } else {
                    self.cmo = compute_cmo(&self.bars, cmo_p as usize);
                }

                // O(1) forming-bar update for CMO
                if self.forming_bar_dirty && n > 1 && cmo_p as usize > 0 {
                    if let Some(last) = self.bars.last() {
                        let delta = last.close - self.bars[n - 2].close;
                        if delta > 0.0 {
                            self.cmo_sum_up += delta;
                        } else if delta < 0.0 {
                            self.cmo_sum_down += -delta;
                        }
                        let denom = self.cmo_sum_up + self.cmo_sum_down;
                        if let Some(last_cmo) = self.cmo.last_mut() {
                            *last_cmo = if denom > f64::EPSILON {
                                Some(100.0 * (self.cmo_sum_up - self.cmo_sum_down) / denom)
                            } else {
                                Some(0.0)
                            };
                        }
                    }
                }

                // O(1) forming-bar update for Linear Regression Slope (simple incremental)
                if self.forming_bar_dirty && n > 1 {
                    if let Some(last) = self.bars.last() {
                        let x = (n - 1) as f64; // current bar index
                        let y = last.close;
                        self.linreg_sum_x += x;
                        self.linreg_sum_y += y;
                        self.linreg_sum_xy += x * y;
                        self.linreg_sum_x2 += x * x;

                        let n_f = n as f64;
                        let denom =
                            n_f * self.linreg_sum_x2 - self.linreg_sum_x * self.linreg_sum_x;
                        if let Some(last_slope) = self.linreg_slope.last_mut() {
                            if denom > f64::EPSILON {
                                *last_slope = Some(
                                    (n_f * self.linreg_sum_xy
                                        - self.linreg_sum_x * self.linreg_sum_y)
                                        / denom,
                                );
                            } else {
                                *last_slope = Some(0.0);
                            }
                        }
                    }
                }

                let qstick_p = 14u32;
                if let Some(data) = gpu.compute_qstick_gpu(qstick_p) {
                    self.qstick = data
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| {
                            if i + 1 < qstick_p as usize {
                                None
                            } else {
                                Some(v as f64)
                            }
                        })
                        .collect();
                } else {
                    self.qstick = compute_qstick(&self.bars, qstick_p as usize);
                }

                let disparity_p = 14u32;
                if let Some(data) = gpu.compute_disparity_gpu(disparity_p) {
                    self.disparity = data
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| {
                            if i + 1 < disparity_p as usize {
                                None
                            } else {
                                Some(v as f64)
                            }
                        })
                        .collect();
                } else {
                    self.disparity = compute_disparity(&self.bars, disparity_p as usize);
                }

                // O(1) forming-bar update for Disparity (using existing SMA100)
                if self.forming_bar_dirty && n > 1 && disparity_p as usize == 100 {
                    if let Some(prev_sma) = self.sma100.get(n - 2).copied().flatten() {
                        if let Some(last_disp) = self.disparity.last_mut() {
                            if let Some(last) = self.bars.last() {
                                let new_ma = (prev_sma * 99.0 + last.close) / 100.0;
                                if new_ma != 0.0 {
                                    *last_disp = Some((last.close - new_ma) / new_ma);
                                }
                            }
                        }
                    }
                }

                let bop_p = 14u32;
                if let Some(data) = gpu.compute_bop_gpu(bop_p) {
                    self.bop = data
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| {
                            if i + 1 < bop_p as usize {
                                None
                            } else {
                                Some(v as f64)
                            }
                        })
                        .collect();
                } else {
                    self.bop = compute_bop(&self.bars, bop_p as usize);
                }

                let stddev_p = 20u32;
                if let Some(data) = gpu.compute_stddev_gpu(stddev_p) {
                    self.stddev = data
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| {
                            if i + 1 < stddev_p as usize {
                                None
                            } else {
                                Some(v as f64)
                            }
                        })
                        .collect();
                } else {
                    self.stddev = compute_stddev(&self.bars, stddev_p as usize);
                }

                let mfi_p = 14u32;
                if let Some(data) = gpu.compute_mfi_gpu(mfi_p) {
                    self.mfi = data
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| {
                            if i < mfi_p as usize {
                                None
                            } else {
                                Some((v as f64).clamp(0.0, 100.0))
                            }
                        })
                        .collect();
                } else {
                    self.mfi = compute_mfi(&self.bars, mfi_p as usize);
                }

                let trix_p = 15u32;
                let trix_sig_p = 9u32;
                if let Some(data) = gpu.compute_trix_gpu(&self.upload_closes, trix_p, trix_sig_p) {
                    let trix_line_warmup = (3 * trix_p as usize).saturating_sub(2);
                    let trix_signal_warmup = 3 * trix_p as usize + trix_sig_p as usize - 3;
                    let mut line = Vec::with_capacity(n);
                    let mut signal = Vec::with_capacity(n);
                    let mut hist = Vec::with_capacity(n);
                    for i in 0..n {
                        let l = data.get(i * 3).copied().unwrap_or(0.0);
                        let s = data.get(i * 3 + 1).copied().unwrap_or(0.0);
                        let h = data.get(i * 3 + 2).copied().unwrap_or(0.0);
                        line.push(if i < trix_line_warmup {
                            None
                        } else {
                            Some(l as f64)
                        });
                        signal.push(if i < trix_signal_warmup {
                            None
                        } else {
                            Some(s as f64)
                        });
                        hist.push(if i < trix_signal_warmup {
                            None
                        } else {
                            Some(h as f64)
                        });
                    }
                    self.trix_line = line;
                    self.trix_signal = signal;
                    self.trix_hist = hist;
                } else {
                    let (line, signal, hist) =
                        compute_trix(&self.bars, trix_p as usize, trix_sig_p as usize);
                    self.trix_line = line;
                    self.trix_signal = signal;
                    self.trix_hist = hist;
                }

                let ppo_fast = 12u32;
                let ppo_slow = 26u32;
                let ppo_sig = 9u32;
                if let Some(data) =
                    gpu.compute_ppo_gpu(&self.upload_closes, ppo_fast, ppo_slow, ppo_sig)
                {
                    let ppo_line_warmup = ppo_slow as usize - 1;
                    let ppo_signal_warmup = ppo_slow as usize + ppo_sig as usize - 2;
                    let mut line = Vec::with_capacity(n);
                    let mut signal = Vec::with_capacity(n);
                    let mut hist = Vec::with_capacity(n);
                    for i in 0..n {
                        let l = data.get(i * 3).copied().unwrap_or(0.0);
                        let s = data.get(i * 3 + 1).copied().unwrap_or(0.0);
                        let h = data.get(i * 3 + 2).copied().unwrap_or(0.0);
                        line.push(if i < ppo_line_warmup {
                            None
                        } else {
                            Some(l as f64)
                        });
                        signal.push(if i < ppo_signal_warmup {
                            None
                        } else {
                            Some(s as f64)
                        });
                        hist.push(if i < ppo_signal_warmup {
                            None
                        } else {
                            Some(h as f64)
                        });
                    }
                    self.ppo_line = line;
                    self.ppo_signal = signal;
                    self.ppo_hist = hist;
                } else {
                    let (line, signal, hist) = compute_ppo(
                        &self.bars,
                        ppo_fast as usize,
                        ppo_slow as usize,
                        ppo_sig as usize,
                    );
                    self.ppo_line = line;
                    self.ppo_signal = signal;
                    self.ppo_hist = hist;
                }

                if let Some(data) = gpu.compute_ultosc_gpu() {
                    self.ultosc = data
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| {
                            if i < 28 {
                                None
                            } else {
                                Some((v as f64).clamp(0.0, 100.0))
                            }
                        })
                        .collect();
                } else {
                    self.ultosc = compute_ultosc(&self.bars);
                }

                if let Some(data) = gpu.compute_stochrsi_gpu(&self.upload_closes, 14, 14, 3, 3) {
                    let stochrsi_k_warmup = 29usize;
                    let stochrsi_d_warmup = 31usize;
                    let mut k = Vec::with_capacity(n);
                    let mut d = Vec::with_capacity(n);
                    for i in 0..n {
                        let kv = data.get(i * 2).copied().unwrap_or(0.0);
                        let dv = data.get(i * 2 + 1).copied().unwrap_or(0.0);
                        k.push(if i < stochrsi_k_warmup {
                            None
                        } else {
                            Some((kv as f64).clamp(0.0, 100.0))
                        });
                        d.push(if i < stochrsi_d_warmup {
                            None
                        } else {
                            Some((dv as f64).clamp(0.0, 100.0))
                        });
                    }
                    self.stochrsi_k = k;
                    self.stochrsi_d = d;
                } else {
                    let (k, d) = compute_stochrsi(&self.bars, 14, 14, 3, 3);
                    self.stochrsi_k = k;
                    self.stochrsi_d = d;
                }

                // VaR Oscillator — GPU (sequential rolling 95% VaR, 0.0 is valid)
                let var_osc_p = 20u32;
                if let Some(data) = gpu.compute_var_oscillator_gpu(var_osc_p) {
                    self.var_oscillator = data
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| {
                            if i < var_osc_p as usize || !v.is_finite() {
                                None
                            } else {
                                Some(v as f64)
                            }
                        })
                        .collect();
                } else {
                    self.var_oscillator = compute_var_oscillator(&self.bars, var_osc_p as usize);
                }

                // Parabolic SAR — GPU (sequential, from OHLC)
                if let Some(data) = gpu.compute_psar_gpu() {
                    self.psar = data
                        .iter()
                        .map(|&v| if v == 0.0 { None } else { Some(v as f64) })
                        .collect();
                } else {
                    self.psar = compute_parabolic_sar(&self.bars, 0.02, 0.2);
                }
                let (au, al) = compute_atr_projection(&self.bars, &self.atr);
                self.atr_proj_upper = au;
                self.atr_proj_lower = al;
                // ATR Projection — GPU (parallel: open ± ATR)
                {
                    let atrs: Vec<f32> = self.atr.iter().map(|v| v.unwrap_or(0.0) as f32).collect();
                    if let Some(data) = gpu.compute_atr_projection_gpu(&atrs) {
                        let n = self.bars.len();
                        let mut au = Vec::with_capacity(n);
                        let mut al = Vec::with_capacity(n);
                        for i in 0..n {
                            let u = data.get(i * 2).copied().unwrap_or(0.0);
                            let l = data.get(i * 2 + 1).copied().unwrap_or(0.0);
                            if u == 0.0 {
                                au.push(None);
                                al.push(None);
                            } else {
                                au.push(Some(u as f64));
                                al.push(Some(l as f64));
                            }
                        }
                        self.atr_proj_upper = au;
                        self.atr_proj_lower = al;
                    } else {
                        let (au, al) = compute_atr_projection(&self.bars, &self.atr);
                        self.atr_proj_upper = au;
                        self.atr_proj_lower = al;
                    }
                }

                // ATR Projection MTF levels (matching ATR_Projection.mqh)
                self.atr_proj_levels =
                    compute_atr_projection_levels(&self.bars, self.timeframe.minutes());

                // BetterVolume — GPU (full Emini-Watch algorithm with OHLCV)
                if let Some(data) = gpu.compute_better_volume_gpu_full(20) {
                    self.better_vol_type = data.iter().map(|&v| v as u8).collect();
                } else {
                    self.better_vol_type = compute_better_volume(&self.bars);
                }

                let (h1, h4, d1, w1, mn1) = compute_prev_candle_levels(&self.bars);
                self.prev_h1_high = h1.0;
                self.prev_h1_low = h1.1;
                self.prev_h4_high = h4.0;
                self.prev_h4_low = h4.1;
                self.prev_daily_high = d1.0;
                self.prev_daily_low = d1.1;
                self.prev_weekly_high = w1.0;
                self.prev_weekly_low = w1.1;
                self.prev_monthly_high = mn1.0;
                self.prev_monthly_low = mn1.1;
                if let (Some(h), Some(l)) = (d1.0, d1.1) {
                    let prev_close = self
                        .bars
                        .iter()
                        .rev()
                        .find(|b| {
                            let day = b.ts_ms / 86_400_000;
                            let last_day = self
                                .bars
                                .last()
                                .map(|lb| lb.ts_ms / 86_400_000)
                                .unwrap_or(0);
                            day < last_day
                        })
                        .map(|b| b.close);
                    if let Some(c) = prev_close {
                        let p = (h + l + c) / 3.0;
                        self.pivot_p = Some(p);
                        self.pivot_r1 = Some(2.0 * p - l);
                        self.pivot_r2 = Some(p + (h - l));
                        self.pivot_s1 = Some(2.0 * p - h);
                        self.pivot_s2 = Some(p - (h - l));
                    }
                }

                // Fractals — GPU (parallel per-bar)
                if let Some(data) = gpu.compute_fractals_gpu() {
                    let n = self.bars.len();
                    self.fractal_up = vec![false; n];
                    self.fractal_down = vec![false; n];
                    for i in 0..n {
                        let up = data.get(i * 2).copied().unwrap_or(0.0);
                        let dn = data.get(i * 2 + 1).copied().unwrap_or(0.0);
                        if up != 0.0 {
                            self.fractal_up[i] = true;
                        }
                        if dn != 0.0 {
                            self.fractal_down[i] = true;
                        }
                    }
                } else {
                    self.fractal_up = compute_fractals_up(&self.bars);
                    self.fractal_down = compute_fractals_down(&self.bars);
                }

                self.harmonics =
                    detect_harmonic_patterns(&self.bars, &self.fractal_up, &self.fractal_down); // CPU (complex pattern matching)

                // Supply/Demand Zones — GPU fractal detection + CPU testing/merging
                // GPU Phase 1: detect fractals (parallel per-bar, 5-bar lookback)
                // CPU Phase 2: refine boundaries, test zones, merge, purge broken
                if let Some(data) = gpu.compute_sd_zones_gpu(5) {
                    let (sz, dz) = compute_supply_demand_zones_from_gpu(&data, &self.bars);
                    // GPU fallback: if GPU produces zero zones, try CPU
                    if sz.is_empty() && dz.is_empty() && self.bars.len() > 20 {
                        let (sz2, dz2) = compute_supply_demand_zones(&self.bars);
                        self.supply_zones = sz2;
                        self.demand_zones = dz2;
                        tracing::debug!(
                            "S/D: GPU produced 0 zones, CPU fallback: {} supply, {} demand",
                            self.supply_zones.len(),
                            self.demand_zones.len()
                        );
                    } else {
                        self.supply_zones = sz;
                        self.demand_zones = dz;
                    }
                } else {
                    let (sz, dz) = compute_supply_demand_zones(&self.bars);
                    self.supply_zones = sz;
                    self.demand_zones = dz;
                }
                self.compute_auto_fibonacci(); // CPU (fractal-based swing detection)
                // VWAP — GPU per-day segments with CPU deviation bands
                let gpu_vwap_ok = 'vwap_gpu: {
                    let n = self.bars.len();
                    if n == 0 {
                        break 'vwap_gpu false;
                    }

                    // Find day boundaries: indices where a new trading day starts
                    let mut day_starts: Vec<usize> = vec![0];
                    for i in 1..n {
                        let prev_day = self.bars[i - 1].ts_ms / 1000 / 86400;
                        let curr_day = self.bars[i].ts_ms / 1000 / 86400;
                        if curr_day != prev_day {
                            day_starts.push(i);
                        }
                    }

                    // Allocate output vectors
                    let mut vw = vec![None; n];
                    let mut vu1 = vec![None; n];
                    let mut vu2 = vec![None; n];
                    let mut vu3 = vec![None; n];
                    let mut vl1 = vec![None; n];
                    let mut vl2 = vec![None; n];
                    let mut vl3 = vec![None; n];

                    // Process each day segment on GPU
                    for seg_idx in 0..day_starts.len() {
                        let start = day_starts[seg_idx];
                        let end = if seg_idx + 1 < day_starts.len() {
                            day_starts[seg_idx + 1]
                        } else {
                            n
                        };
                        let seg_len = end - start;

                        // GPU: compute anchored VWAP for this day segment directly from resident
                        // OHLCV buffers without rebuilding per-segment scratch arrays.
                        let gpu_result = gpu.compute_anchored_vwap(start as u32, end as u32);
                        let gpu_vwap = match gpu_result {
                            Some(v) if v.len() >= seg_len => v,
                            _ => {
                                break 'vwap_gpu false;
                            }
                        };

                        // CPU: compute deviation bands from GPU VWAP values
                        // σ = sqrt( Σ(tp²·vol)/Σ(vol) - vwap² )
                        let mut cum_vol = 0.0_f64;
                        let mut cum_tp2_vol = 0.0_f64;
                        for j in 0..seg_len {
                            let b = &self.bars[start + j];
                            let tp = (b.high + b.low + b.close) / 3.0;
                            let vol = b.volume.max(1.0);
                            cum_vol += vol;
                            cum_tp2_vol += tp * tp * vol;

                            let vwap_val = gpu_vwap[j] as f64;
                            let variance = (cum_tp2_vol / cum_vol - vwap_val * vwap_val).max(0.0);
                            let sd = variance.sqrt();

                            let idx = start + j;
                            vw[idx] = Some(vwap_val);
                            vu1[idx] = Some(vwap_val + sd);
                            vu2[idx] = Some(vwap_val + 2.0 * sd);
                            vu3[idx] = Some(vwap_val + 3.0 * sd);
                            vl1[idx] = Some(vwap_val - sd);
                            vl2[idx] = Some(vwap_val - 2.0 * sd);
                            vl3[idx] = Some(vwap_val - 3.0 * sd);
                        }
                    }

                    self.vwap = vw;
                    self.vwap_upper1 = vu1;
                    self.vwap_upper2 = vu2;
                    self.vwap_upper3 = vu3;
                    self.vwap_lower1 = vl1;
                    self.vwap_lower2 = vl2;
                    self.vwap_lower3 = vl3;
                    true
                };
                if !gpu_vwap_ok {
                    // GPU VWAP failed — fall back to full CPU compute_vwap()
                    let (vw, vu1, vu2, vu3, vl1, vl2, vl3) = compute_vwap(&self.bars);
                    self.vwap = vw;
                    self.vwap_upper1 = vu1;
                    self.vwap_upper2 = vu2;
                    self.vwap_upper3 = vu3;
                    self.vwap_lower1 = vl1;
                    self.vwap_lower2 = vl2;
                    self.vwap_lower3 = vl3;
                }
                // Supertrend — GPU (sequential, ATR-based) with CPU fallback
                if let Some(data) = gpu.compute_supertrend_gpu(10) {
                    let n = self.bars.len();
                    let mut st = Vec::with_capacity(n);
                    let mut bull = Vec::with_capacity(n);
                    for i in 0..n {
                        let v = data.get(i * 2).copied().unwrap_or(0.0);
                        let d = data.get(i * 2 + 1).copied().unwrap_or(0.0);
                        if v == 0.0 {
                            st.push(None);
                        } else {
                            st.push(Some(v as f64));
                        }
                        bull.push(d > 0.0);
                    }
                    self.supertrend = st;
                    self.supertrend_bull = bull;
                } else {
                    let (st, st_bull) = compute_supertrend(&self.bars, &self.atr, 10, 3.0);
                    self.supertrend = st;
                    self.supertrend_bull = st_bull;
                }

                // Donchian Channel — GPU (parallel) with CPU fallback
                if let Some(data) = gpu.compute_donchian_gpu(20) {
                    let n = self.bars.len();
                    let mut du = Vec::with_capacity(n);
                    let mut dl = Vec::with_capacity(n);
                    for i in 0..n {
                        let u = data.get(i * 2).copied().unwrap_or(0.0);
                        let l = data.get(i * 2 + 1).copied().unwrap_or(0.0);
                        if u == 0.0 {
                            du.push(None);
                            dl.push(None);
                        } else {
                            du.push(Some(u as f64));
                            dl.push(Some(l as f64));
                        }
                    }
                    self.donchian_upper = du;
                    self.donchian_lower = dl;
                } else {
                    let (du, dl) = compute_donchian(&self.bars, 20);
                    self.donchian_upper = du;
                    self.donchian_lower = dl;
                }

                // Keltner Channel — GPU (sequential EMA+ATR) with CPU fallback
                if let Some(data) = gpu.compute_keltner_gpu(20) {
                    let n = self.bars.len();
                    let mut ku = Vec::with_capacity(n);
                    let mut km = Vec::with_capacity(n);
                    let mut kl = Vec::with_capacity(n);
                    for i in 0..n {
                        let u = data.get(i * 3).copied().unwrap_or(0.0);
                        let m = data.get(i * 3 + 1).copied().unwrap_or(0.0);
                        let l = data.get(i * 3 + 2).copied().unwrap_or(0.0);
                        if m == 0.0 {
                            ku.push(None);
                            km.push(None);
                            kl.push(None);
                        } else {
                            ku.push(Some(u as f64));
                            km.push(Some(m as f64));
                            kl.push(Some(l as f64));
                        }
                    }
                    self.keltner_upper = ku;
                    self.keltner_mid = km;
                    self.keltner_lower = kl;
                } else {
                    let (km, ku, kl) = compute_keltner(&self.bars, 20, 10, 1.5);
                    self.keltner_mid = km;
                    self.keltner_upper = ku;
                    self.keltner_lower = kl;
                }

                // Regression Channel — GPU (parallel least squares) with CPU fallback
                if let Some(data) = gpu.compute_regression_gpu(20) {
                    let n = self.bars.len();
                    let mut rm = Vec::with_capacity(n);
                    let mut ru = Vec::with_capacity(n);
                    let mut rl = Vec::with_capacity(n);
                    for i in 0..n {
                        let m = data.get(i * 3).copied().unwrap_or(0.0);
                        let u = data.get(i * 3 + 1).copied().unwrap_or(0.0);
                        let l = data.get(i * 3 + 2).copied().unwrap_or(0.0);
                        if m == 0.0 {
                            rm.push(None);
                            ru.push(None);
                            rl.push(None);
                        } else {
                            rm.push(Some(m as f64));
                            ru.push(Some(u as f64));
                            rl.push(Some(l as f64));
                        }
                    }
                    self.regression_mid = rm;
                    self.regression_upper = ru;
                    self.regression_lower = rl;
                } else {
                    let (rm, ru, rl) = compute_regression_channel(&self.bars, 20);
                    self.regression_mid = rm;
                    self.regression_upper = ru;
                    self.regression_lower = rl;
                }

                // Squeeze Momentum — GPU (sequential BB+KC) with CPU fallback
                if let Some(data) = gpu.compute_squeeze_gpu(20) {
                    let n = self.bars.len();
                    let mut sm = Vec::with_capacity(n);
                    let mut sq = Vec::with_capacity(n);
                    for i in 0..n {
                        let m = data.get(i * 2).copied().unwrap_or(0.0);
                        let s = data.get(i * 2 + 1).copied().unwrap_or(0.0);
                        sm.push(Some(m as f64));
                        sq.push(s > 0.5);
                    }
                    self.squeeze_mom = sm;
                    self.squeeze_on = sq;
                } else {
                    let (sm, sq) = compute_squeeze_momentum(
                        &self.bb_upper,
                        &self.bb_lower,
                        &self.keltner_upper,
                        &self.keltner_lower,
                        &self.bars,
                        20,
                    );
                    self.squeeze_mom = sm;
                    self.squeeze_on = sq;
                }
                // Pre-compute 20-bar rolling average volume for heatmap candle coloring
                {
                    let n = self.bars.len();
                    let mut avg = vec![0.0_f64; n];
                    let period = 20usize;
                    let mut sum = 0.0;
                    for i in 0..n {
                        sum += self.bars[i].volume;
                        if i >= period {
                            sum -= self.bars[i - period].volume;
                        }
                        avg[i] = if i >= period - 1 {
                            sum / period as f64
                        } else {
                            sum / (i + 1) as f64
                        };
                    }
                    self.vol_avg_20 = avg;
                }

                // Ehlers Super Smoother — GPU
                if let Some(data) = gpu.compute_ehlers_ss_gpu(10) {
                    self.ehlers_ss = data
                        .iter()
                        .map(|&v| if v == 0.0 { None } else { Some(v as f64) })
                        .collect();
                } else {
                    self.ehlers_ss = ehlers_super_smoother(&self.bars, 10);
                }

                // Ehlers Decycler — GPU
                if let Some(data) = gpu.compute_ehlers_dec_gpu(20) {
                    self.ehlers_decycler = data
                        .iter()
                        .map(|&v| if v == 0.0 { None } else { Some(v as f64) })
                        .collect();
                } else {
                    self.ehlers_decycler = ehlers_decycler(&self.bars, 20);
                }

                // Ehlers ITL — GPU
                if let Some(data) = gpu.compute_ehlers_itl_gpu() {
                    self.ehlers_itl = data
                        .iter()
                        .map(|&v| if v == 0.0 { None } else { Some(v as f64) })
                        .collect();
                } else {
                    self.ehlers_itl = ehlers_instantaneous_trendline(&self.bars);
                }

                // Ehlers MAMA/FAMA — GPU (2 outputs)
                if let Some(data) = gpu.compute_ehlers_mama_gpu() {
                    let n = self.bars.len();
                    let mut mama = Vec::with_capacity(n);
                    let mut fama = Vec::with_capacity(n);
                    for i in 0..n {
                        let m = data.get(i * 2).copied().unwrap_or(0.0);
                        let f = data.get(i * 2 + 1).copied().unwrap_or(0.0);
                        if i < 6 || (m == 0.0 && f == 0.0) {
                            mama.push(None);
                            fama.push(None);
                        } else {
                            mama.push(Some(m as f64));
                            fama.push(Some(f as f64));
                        }
                    }
                    self.ehlers_mama = mama;
                    self.ehlers_fama = fama;
                } else {
                    let (m, f) = ehlers_mama_fama(&self.bars, 0.5, 0.05);
                    self.ehlers_mama = m;
                    self.ehlers_fama = f;
                }

                // Ehlers EBSW — GPU (sub-pane oscillator, CPU starts at i=1)
                if let Some(data) = gpu.compute_ehlers_ebsw_gpu(40) {
                    self.ehlers_ebsw = data
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| if i < 2 { None } else { Some(v as f64) })
                        .collect();
                } else {
                    self.ehlers_ebsw = ehlers_even_better_sinewave(&self.bars, 40);
                }

                // Ehlers Cyber Cycle — GPU (sub-pane oscillator, CPU starts at i=4)
                if let Some(data) = gpu.compute_ehlers_cyber_gpu() {
                    self.ehlers_cyber = data
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| if i < 4 { None } else { Some(v as f64) })
                        .collect();
                } else {
                    self.ehlers_cyber = ehlers_cyber_cycle(&self.bars);
                }

                // Ehlers CG Oscillator — GPU (parallel, CPU starts at period-1=9, 0.0 is valid)
                if let Some(data) = gpu.compute_ehlers_cg_gpu(10) {
                    self.ehlers_cg = data
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| if i < 9 { None } else { Some(v as f64) })
                        .collect();
                } else {
                    self.ehlers_cg = ehlers_cg_oscillator(&self.bars, 10);
                }

                // Ehlers Roofing Filter — GPU (sub-pane oscillator, CPU starts at i=2)
                if let Some(data) = gpu.compute_ehlers_roof_gpu(10, 48) {
                    self.ehlers_roof = data
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| if i < 2 { None } else { Some(v as f64) })
                        .collect();
                } else {
                    self.ehlers_roof = ehlers_roofing_filter(&self.bars, 10, 48);
                }
                return;
            }
        }

        // ── CPU fallback path (no GPU available) ──
        let sma_slow = self.sma_slow_period as usize;
        let sma_fast = self.sma_fast_period as usize;
        let ema_p = self.ema_period as usize;
        let bb_p = self.bb_period as usize;
        let rsi_p = self.rsi_period as usize;
        let fisher_p = self.fisher_period as usize;
        let atr_p = self.atr_period as usize;
        let stoch_p = self.stoch_period as usize;
        let adx_p = self.adx_period as usize;
        let mom_p = self.momentum_period as usize;
        self.sma200 = compute_sma(&self.bars, sma_slow);
        self.sma100 = compute_sma(&self.bars, sma_fast);
        self.kama = compute_kama(&self.bars, 10, 2, 30);
        self.ema21 = compute_ema(&self.bars, ema_p);
        let (mid, upper, lower) = compute_bollinger(&self.bars, bb_p, 2.0);
        self.bb_mid = mid;
        self.bb_upper = upper;
        self.bb_lower = lower;
        self.rsi = compute_rsi(&self.bars, rsi_p);
        let (f, fs) = compute_fisher(&self.bars, fisher_p);
        self.fisher = f;
        self.fisher_signal = fs;
        self.atr = compute_atr(&self.bars, atr_p);
        let (ml, ms, mh) = compute_macd(
            &self.bars,
            self.macd_fast as usize,
            self.macd_slow as usize,
            self.macd_signal_p as usize,
        );
        self.macd_line = ml;
        self.macd_signal = ms;
        self.macd_hist = mh;
        let (sk, sd) = compute_stochastic(&self.bars, stoch_p, 3, 3);
        self.stoch_k = sk;
        self.stoch_d = sd;
        let (adx, dip, dim) = compute_adx(&self.bars, adx_p);
        self.adx = adx;
        self.di_plus = dip;
        self.di_minus = dim;
        let (tk, kj, sa, sb) = compute_ichimoku(&self.bars, 9, 26, 52);
        self.ichi_tenkan = tk;
        self.ichi_kijun = kj;
        self.ichi_span_a = sa;
        self.ichi_span_b = sb;
        self.wma = compute_wma(&self.bars, 20);
        self.hma = compute_hma(&self.bars, 20);
        self.cci = compute_cci(&self.bars, 20);
        self.williams_r = compute_williams_r(&self.bars, 14);
        self.obv = compute_obv(&self.bars);
        self.momentum = compute_momentum(&self.bars, mom_p);
        self.cmo = compute_cmo(&self.bars, 9);
        self.qstick = compute_qstick(&self.bars, 14);
        self.disparity = compute_disparity(&self.bars, 14);
        self.bop = compute_bop(&self.bars, 14);
        self.stddev = compute_stddev(&self.bars, 20);
        self.mfi = compute_mfi(&self.bars, 14);
        let (trix_line, trix_signal, trix_hist) = compute_trix(&self.bars, 15, 9);
        self.trix_line = trix_line;
        self.trix_signal = trix_signal;
        self.trix_hist = trix_hist;
        let (ppo_line, ppo_signal, ppo_hist) = compute_ppo(&self.bars, 12, 26, 9);
        self.ppo_line = ppo_line;
        self.ppo_signal = ppo_signal;
        self.ppo_hist = ppo_hist;
        self.ultosc = compute_ultosc(&self.bars);
        let (stochrsi_k, stochrsi_d) = compute_stochrsi(&self.bars, 14, 14, 3, 3);
        self.stochrsi_k = stochrsi_k;
        self.stochrsi_d = stochrsi_d;
        self.var_oscillator = compute_var_oscillator(&self.bars, 20);
        self.psar = compute_parabolic_sar(&self.bars, 0.02, 0.2);
        let (au, al) = compute_atr_projection(&self.bars, &self.atr);
        self.atr_proj_upper = au;
        self.atr_proj_lower = al;
        self.atr_proj_levels = compute_atr_projection_levels(&self.bars, self.timeframe.minutes());
        self.better_vol_type = compute_better_volume(&self.bars);
        // Previous candle levels — find the second-to-last daily/weekly bar boundaries
        let (h1, h4, d1, w1, mn1) = compute_prev_candle_levels(&self.bars);
        self.prev_h1_high = h1.0;
        self.prev_h1_low = h1.1;
        self.prev_h4_high = h4.0;
        self.prev_h4_low = h4.1;
        self.prev_daily_high = d1.0;
        self.prev_daily_low = d1.1;
        self.prev_weekly_high = w1.0;
        self.prev_weekly_low = w1.1;
        self.prev_monthly_high = mn1.0;
        self.prev_monthly_low = mn1.1;
        // Pivot points from previous day
        if let (Some(h), Some(l)) = (d1.0, d1.1) {
            // Hoist last_day out of the find closure — was recomputed on every
            // bar iteration during the reverse scan.
            let last_day = self
                .bars
                .last()
                .map(|lb| lb.ts_ms / 86_400_000)
                .unwrap_or(0);
            let prev_close = self
                .bars
                .iter()
                .rev()
                .find(|b| b.ts_ms / 86_400_000 < last_day)
                .map(|b| b.close);
            if let Some(c) = prev_close {
                let p = (h + l + c) / 3.0;
                self.pivot_p = Some(p);
                self.pivot_r1 = Some(2.0 * p - l);
                self.pivot_r2 = Some(p + (h - l));
                self.pivot_s1 = Some(2.0 * p - h);
                self.pivot_s2 = Some(p - (h - l));
            }
        }
        // Fractals
        self.fractal_up = compute_fractals_up(&self.bars);
        self.fractal_down = compute_fractals_down(&self.bars);
        self.harmonics = detect_harmonic_patterns(&self.bars, &self.fractal_up, &self.fractal_down);
        let (sz, dz) = compute_supply_demand_zones(&self.bars);
        self.supply_zones = sz;
        self.demand_zones = dz;
        // Auto Fibonacci (fractal-based swing detection, matching AutoFibonacci.mqh)
        self.compute_auto_fibonacci();
        // VWAP (daily anchored)
        let (vw, vu1, vu2, vu3, vl1, vl2, vl3) = compute_vwap(&self.bars);
        self.vwap = vw;
        self.vwap_upper1 = vu1;
        self.vwap_upper2 = vu2;
        self.vwap_upper3 = vu3;
        self.vwap_lower1 = vl1;
        self.vwap_lower2 = vl2;
        self.vwap_lower3 = vl3;
        // Supertrend, Donchian, Keltner
        let (st, st_bull) = compute_supertrend(&self.bars, &self.atr, 10, 3.0);
        self.supertrend = st;
        self.supertrend_bull = st_bull;
        let (du, dl) = compute_donchian(&self.bars, 20);
        self.donchian_upper = du;
        self.donchian_lower = dl;
        let (km, ku, kl) = compute_keltner(&self.bars, 20, 10, 1.5);
        self.keltner_mid = km;
        self.keltner_upper = ku;
        self.keltner_lower = kl;
        let (rm, ru, rl) = compute_regression_channel(&self.bars, 20);
        self.regression_mid = rm;
        self.regression_upper = ru;
        self.regression_lower = rl;
        let (sm, sq) = compute_squeeze_momentum(
            &self.bb_upper,
            &self.bb_lower,
            &self.keltner_upper,
            &self.keltner_lower,
            &self.bars,
            20,
        );
        self.squeeze_mom = sm;
        self.squeeze_on = sq;
        // Pre-compute 20-bar rolling average volume for heatmap candle coloring
        {
            let n = self.bars.len();
            let mut avg = vec![0.0_f64; n];
            let period = 20usize;
            let mut sum = 0.0;
            for i in 0..n {
                sum += self.bars[i].volume;
                if i >= period {
                    sum -= self.bars[i - period].volume;
                }
                avg[i] = if i >= period - 1 {
                    sum / period as f64
                } else {
                    sum / (i + 1) as f64
                };
            }
            self.vol_avg_20 = avg;
        }
        // Ehlers indicators
        self.ehlers_ss = ehlers_super_smoother(&self.bars, 10);
        self.ehlers_decycler = ehlers_decycler(&self.bars, 20);
        self.ehlers_itl = ehlers_instantaneous_trendline(&self.bars);
        let (mama, fama) = ehlers_mama_fama(&self.bars, 0.5, 0.05);
        self.ehlers_mama = mama;
        self.ehlers_fama = fama;
        self.ehlers_ebsw = ehlers_even_better_sinewave(&self.bars, 40);
        self.ehlers_cyber = ehlers_cyber_cycle(&self.bars);
        self.ehlers_cg = ehlers_cg_oscillator(&self.bars, 10);
        self.ehlers_roof = ehlers_roofing_filter(&self.bars, 10, 48);
    }

    /// Compute Auto Fibonacci levels from fractal swing points.
    /// Mirrors AutoFibonacci.mqh: finds most significant recent swing high/low
    /// and computes retracement (0-100%) + extension (127.2-423.6%) levels.
    /// Compute MultiKAMA: load bars from higher timeframes and compute KAMA(10,2,30) on each.
    /// Projects KAMA values onto this chart's x-axis by matching timestamps.
    /// Compute MTF SMA lines matching MTF_MA.mqh behavior.
    /// Loads HTF bars from cache, computes SMA on them, projects onto current chart.
    /// Lines: H1/200, H4/200, D1/200, W1/200, W1/100, MN1/100
    pub(crate) fn compute_mtf_sma(&mut self, cache: &SqliteCache) {
        self.mtf_sma.clear();
        if self.bars.is_empty() {
            return;
        }

        let base_sym = {
            let parts: Vec<&str> = self.symbol.split(':').collect();
            let is_tf = matches!(
                parts.last().copied(),
                Some(
                    "1Min"
                        | "5Min"
                        | "15Min"
                        | "30Min"
                        | "1Hour"
                        | "4Hour"
                        | "1Day"
                        | "1Week"
                        | "1Month"
                )
            );
            if is_tf && parts.len() > 1 {
                parts[..parts.len() - 1].join(":")
            } else {
                self.symbol.clone()
            }
        };

        // (label, tf_suffix, sma_period, tf_minutes) — matching MTF_MA.mqh plotted lines
        let mtf_lines: &[(&str, &str, usize, u32)] = &[
            ("H1 200", "1Hour", 200, 60),
            ("H4 200", "4Hour", 200, 240),
            ("D1 200", "1Day", 200, 1440),
            ("W1 200", "1Week", 200, 10080),
            ("W1 100", "1Week", 100, 10080),
            ("MN1 100", "1Month", 100, 43200),
        ];

        // Extract bare symbol (strip ALL prefixes and timeframe)
        let bare_sym = {
            let known_prefixes = [
                "default:",
                "kraken-futures:",
                "kraken-equities:",
                "kraken:",
                "tastytrade:",
                "alpaca:",
                "yahoo-chart:",
                "paper_TyphooN:",
                "alpaca_paper_TyphooN:",
            ];
            let mut s = base_sym.as_str();
            for pfx in &known_prefixes {
                if s.starts_with(pfx) {
                    s = &s[pfx.len()..];
                    break;
                }
            }
            let parts: Vec<&str> = s.split(':').collect();
            parts
                .last()
                .copied()
                .unwrap_or(s)
                .replace('/', "")
                .trim_end_matches(".EQ")
                .to_string()
        };

        let prefixes = [
            "merged:",
            "default:",
            "kraken-equities:",
            "kraken:",
            "kraken-futures:",
            "tastytrade:",
            "alpaca:",
            "yahoo-chart:",
            "paper_TyphooN:",
            "alpaca_paper_TyphooN:",
            "",
        ];

        for &(label, tf_suffix, period, _tf_min) in mtf_lines {
            // 1:1 MT5 parity: MTF_MA.mqh declares all 6 plotted buffers as INDICATOR_DATA
            // (see MTF_MA.mqh lines 72-77) with no chart-period guard, so every line is
            // drawn on every host timeframe. We match that exactly — lower-TF lines
            // projected onto higher-TF bars are informationally thin but MT5-accurate.
            let mut htf_bars: Option<Vec<Bar>> = None;
            // Try with bare symbol under each prefix, plus the original base_sym
            for prefix in &prefixes {
                // e.g. "kraken:EURUSD:1Hour" — canonical 3-part key format.
                let key = format!("{}{}:{}", prefix, bare_sym, tf_suffix);
                if let Ok(Some(raw)) = cache.get_bars_raw(&key) {
                    if !chart_source_bars_match_timeframe(
                        cache_source_from_key(&key),
                        tf_suffix,
                        &raw,
                    ) {
                        continue;
                    }
                    htf_bars = Some(
                        raw.into_iter()
                            .map(|(ts, o, h, l, c, v)| Bar {
                                ts_ms: ts,
                                open: o,
                                high: h,
                                low: l,
                                close: c,
                                volume: v,
                            })
                            .collect(),
                    );
                    break;
                }
            }
            // Also try with full base_sym (i.e. the raw chart symbol incl. its prefix).
            if htf_bars.is_none() {
                let key = format!("{}:{}", base_sym, tf_suffix);
                if let Ok(Some(raw)) = cache.get_bars_raw(&key) {
                    if !chart_source_bars_match_timeframe(
                        cache_source_from_key(&key),
                        tf_suffix,
                        &raw,
                    ) {
                        continue;
                    }
                    htf_bars = Some(
                        raw.into_iter()
                            .map(|(ts, o, h, l, c, v)| Bar {
                                ts_ms: ts,
                                open: o,
                                high: h,
                                low: l,
                                close: c,
                                volume: v,
                            })
                            .collect(),
                    );
                }
            }
            // Fallback: indexed partial-match search via SQL LIKE.
            if htf_bars.is_none() {
                if let Ok(keys) = cache.search_keys(&bare_sym, 32) {
                    let tf_lower = tf_suffix.to_lowercase();
                    for k in &keys {
                        if k.to_lowercase().ends_with(&tf_lower) {
                            if let Ok(Some(raw)) = cache.get_bars_raw(k) {
                                htf_bars = Some(
                                    raw.into_iter()
                                        .map(|(ts, o, h, l, c, v)| Bar {
                                            ts_ms: ts,
                                            open: o,
                                            high: h,
                                            low: l,
                                            close: c,
                                            volume: v,
                                        })
                                        .collect(),
                                );
                                break;
                            }
                        }
                    }
                }
            }

            if let Some(htf) = htf_bars {
                if htf.len() < period {
                    continue;
                }
                let sma_vals = compute_sma(&htf, period);

                // Project HTF SMA onto current chart bars via timestamp matching
                let mut projected: Vec<(usize, f64)> = Vec::new();
                let mut htf_idx = 0;
                for (i, bar) in self.bars.iter().enumerate() {
                    while htf_idx + 1 < htf.len() && htf[htf_idx + 1].ts_ms <= bar.ts_ms {
                        htf_idx += 1;
                    }
                    if htf_idx < sma_vals.len() {
                        if let Some(v) = sma_vals[htf_idx] {
                            projected.push((i, v));
                        }
                    }
                }

                if !projected.is_empty() {
                    self.mtf_sma.push((label.to_string(), projected));
                }
            }
        }
    }

    pub(crate) fn ensure_mql_mtf_overlays_for_render(
        &mut self,
        cache: &SqliteCache,
        show_mtf_ma: bool,
        show_multi_kama: bool,
    ) {
        if self.bars.is_empty() {
            return;
        }
        if show_mtf_ma && self.mtf_sma.is_empty() {
            self.compute_mtf_sma(cache);
        }
        if show_multi_kama && self.multi_kama.is_empty() {
            self.compute_multi_kama(cache);
        }
    }

    pub(crate) fn should_ensure_mql_mtf_overlays_for_render(
        heavy_sync_in_progress: bool,
        mtf_enabled: bool,
        is_focused: bool,
    ) -> bool {
        !heavy_sync_in_progress || !mtf_enabled || is_focused
    }

    pub(crate) fn compute_multi_kama(&mut self, cache: &SqliteCache) {
        self.multi_kama.clear();
        if self.bars.is_empty() {
            return;
        }

        // Extract base symbol (strip timeframe suffix from symbol)
        let base_sym = {
            let parts: Vec<&str> = self.symbol.split(':').collect();
            let is_tf = matches!(
                parts.last().copied(),
                Some(
                    "1Min"
                        | "5Min"
                        | "15Min"
                        | "30Min"
                        | "1Hour"
                        | "4Hour"
                        | "1Day"
                        | "1Week"
                        | "1Month"
                )
            );
            if is_tf && parts.len() > 1 {
                parts[..parts.len() - 1].join(":")
            } else {
                self.symbol.clone()
            }
        };

        let higher_tfs: &[(&str, &str, u32)] = &[
            ("H1", "1Hour", 60),
            ("H4", "4Hour", 240),
            ("D1", "1Day", 1440),
            ("W1", "1Week", 10080),
            ("MN1", "1Month", 43200),
        ];

        // Extract bare symbol (strip source prefixes like kraken:)
        let bare_sym = {
            let known_prefixes = [
                "default:",
                "kraken-futures:",
                "kraken-equities:",
                "kraken:",
                "tastytrade:",
                "alpaca:",
                "yahoo-chart:",
                "paper_TyphooN:",
                "alpaca_paper_TyphooN:",
            ];
            let mut s = base_sym.as_str();
            for pfx in &known_prefixes {
                if s.starts_with(pfx) {
                    s = &s[pfx.len()..];
                    break;
                }
            }
            let parts: Vec<&str> = s.split(':').collect();
            parts
                .last()
                .copied()
                .unwrap_or(s)
                .replace('/', "")
                .trim_end_matches(".EQ")
                .to_string()
        };

        let prefixes = [
            "merged:",
            "default:",
            "kraken-equities:",
            "kraken:",
            "kraken-futures:",
            "tastytrade:",
            "alpaca:",
            "yahoo-chart:",
            "paper_TyphooN:",
            "alpaca_paper_TyphooN:",
            "",
        ];

        for &(tf_label, tf_suffix, _tf_min) in higher_tfs {
            // 1:1 MT5 parity: MultiKAMA.mqh declares all 5 plotted buffers
            // (ExtAMABuffer_H1/H4/D1/W1/MN1) as INDICATOR_DATA with no chart-period
            // guard (see MultiKAMA.mqh lines 47-58), so every KAMA line is drawn on
            // every host timeframe. We match that exactly.
            let mut htf_bars: Option<Vec<Bar>> = None;
            // Try bare symbol with each prefix
            for prefix in &prefixes {
                let key = format!("{}{}:{}", prefix, bare_sym, tf_suffix);
                if let Ok(Some(raw)) = cache.get_bars_raw(&key) {
                    if !chart_source_bars_match_timeframe(
                        cache_source_from_key(&key),
                        tf_suffix,
                        &raw,
                    ) {
                        continue;
                    }
                    htf_bars = Some(
                        raw.into_iter()
                            .map(|(ts, o, h, l, c, v)| Bar {
                                ts_ms: ts,
                                open: o,
                                high: h,
                                low: l,
                                close: c,
                                volume: v,
                            })
                            .collect(),
                    );
                    break;
                }
            }
            // Fallback: try with full base_sym
            if htf_bars.is_none() {
                let key = format!("{}:{}", base_sym, tf_suffix);
                if let Ok(Some(raw)) = cache.get_bars_raw(&key) {
                    if !chart_source_bars_match_timeframe(
                        cache_source_from_key(&key),
                        tf_suffix,
                        &raw,
                    ) {
                        continue;
                    }
                    htf_bars = Some(
                        raw.into_iter()
                            .map(|(ts, o, h, l, c, v)| Bar {
                                ts_ms: ts,
                                open: o,
                                high: h,
                                low: l,
                                close: c,
                                volume: v,
                            })
                            .collect(),
                    );
                }
            }
            // Fallback: indexed partial-match search via SQL LIKE.
            if htf_bars.is_none() {
                if let Ok(keys) = cache.search_keys(&bare_sym, 32) {
                    let tf_lower = tf_suffix.to_lowercase();
                    for k in &keys {
                        if k.to_lowercase().ends_with(&tf_lower) {
                            if let Ok(Some(raw)) = cache.get_bars_raw(k) {
                                htf_bars = Some(
                                    raw.into_iter()
                                        .map(|(ts, o, h, l, c, v)| Bar {
                                            ts_ms: ts,
                                            open: o,
                                            high: h,
                                            low: l,
                                            close: c,
                                            volume: v,
                                        })
                                        .collect(),
                                );
                                break;
                            }
                        }
                    }
                }
            }

            if let Some(htf) = htf_bars {
                if htf.len() < 12 {
                    continue;
                }
                // Compute KAMA(10,2,30) on higher TF bars
                let kama_vals = compute_kama(&htf, 10, 2, 30);

                // Map higher TF KAMA values onto this chart's bar indices by timestamp
                // For each of our bars, find the most recent HTF bar that's <= our timestamp
                let mut projected: Vec<(usize, f64)> = Vec::new();
                let mut htf_idx = 0;
                for (i, bar) in self.bars.iter().enumerate() {
                    while htf_idx + 1 < htf.len() && htf[htf_idx + 1].ts_ms <= bar.ts_ms {
                        htf_idx += 1;
                    }
                    if htf_idx < kama_vals.len() {
                        if let Some(k) = kama_vals[htf_idx] {
                            projected.push((i, k));
                        }
                    }
                }

                if !projected.is_empty() {
                    self.multi_kama.push((tf_label.to_string(), projected));
                }
            }
        }
    }

    pub(crate) fn compute_auto_fibonacci(&mut self) {
        self.auto_fib_levels.clear();
        self.auto_fib_swing = None;
        if self.bars.len() < 20 {
            return;
        }

        let lookback = 10usize; // InpFractalLookback
        let recent_start = (self.bars.len() as f64 * 0.4) as usize; // search recent 60%
        let search = &self.bars[recent_start..];

        // Find swing high and swing low from fractals in search range
        let mut swing_high: Option<(f64, usize)> = None;
        let mut swing_low: Option<(f64, usize)> = None;

        for i in lookback..search.len().saturating_sub(lookback) {
            let abs_i = recent_start + i;
            if abs_i < self.fractal_up.len() && self.fractal_up[abs_i] {
                if swing_high.map_or(true, |(h, _)| search[i].high > h) {
                    swing_high = Some((search[i].high, abs_i));
                }
            }
            if abs_i < self.fractal_down.len() && self.fractal_down[abs_i] {
                if swing_low.map_or(true, |(l, _)| search[i].low < l) {
                    swing_low = Some((search[i].low, abs_i));
                }
            }
        }

        if let (Some((high, hi_idx)), Some((low, lo_idx))) = (swing_high, swing_low) {
            if (high - low).abs() < f64::EPSILON {
                return;
            }
            self.auto_fib_swing = Some((high, low, hi_idx, lo_idx));
            let range = high - low;
            let is_bull = lo_idx < hi_idx; // uptrend: low comes before high

            // Retracement levels (from high toward low for bull, from low toward high for bear)
            let retrace_levels = [0.0, 0.236, 0.382, 0.5, 0.618, 0.786, 1.0];
            let retrace_labels = ["0%", "23.6%", "38.2%", "50%", "61.8%", "78.6%", "100%"];
            for (lvl, label) in retrace_levels.iter().zip(retrace_labels.iter()) {
                let price = if is_bull {
                    high - lvl * range
                } else {
                    low + lvl * range
                };
                self.auto_fib_levels.push((price, label.to_string(), false));
            }

            // Extension levels (beyond the swing)
            let ext_levels = [1.272, 1.618, 2.0, 2.618, 3.618, 4.236];
            let ext_labels = ["127.2%", "161.8%", "200%", "261.8%", "361.8%", "423.6%"];
            for (lvl, label) in ext_levels.iter().zip(ext_labels.iter()) {
                let price = if is_bull {
                    low + lvl * range
                } else {
                    high - lvl * range
                };
                self.auto_fib_levels.push((price, label.to_string(), true));
            }
        }
    }

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
