//! Chart rendering layer (ADR-125 Target 2, slice 7b): draws price bars/candles, overlays,
//! axes, crosshair, sub-panes, drawing annotations, volume profile, smart-money zones, MTF
//! overlays — free functions over egui + the crate's chart types. No `TyphooNApp`.
use crate::drawing::*;
use crate::indicators::*;
use crate::models::*;
use crate::state::ChartState;
use crate::types::*;

mod chart_helpers;
mod time_axis;

pub use chart_helpers::chart_overlay_company_name;
use chart_helpers::*;
pub use chart_helpers::{parse_range, parse_range_f32};
use time_axis::*;
pub use time_axis::{format_price, format_price_buf, format_ts, format_ts_buf};

// ─── chart rendering ─────────────────────────────────────────────────────────

/// The exact price↔y mapping `draw_chart` painted with on its last frame:
/// the price-pane rect (sub-panes and time axis already excluded) plus the
/// final price range after live-quote/indicator extension, padding, and the
/// manual-camera override. Stored per chart so input hit-testing (SL/TP line
/// drags) uses the rendered pixels, not a re-derived approximation — the old
/// legacy re-derivation ignored sub-panes, the time axis, log scale, and the
/// free-look camera, which is why line grabs missed.
/// Painted price-axis width. Interaction code (widget splits, hit regions)
/// must use this same constant — a mismatched split leaves a dead strip of
/// painted axis that pans the chart instead of scaling it.
pub const PRICE_AXIS_W: f32 = 98.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PriceViewGeometry {
    pub chart_rect: egui::Rect,
    pub price_min: f64,
    pub price_max: f64,
    pub log_scale: bool,
    /// x of the first *real* bar's slot left edge (camera-aware: can sit left
    /// of `chart_rect` when panned past the data edge).
    pub data_left: f32,
    /// Painted slot width in pixels (camera viewport, not the data slice).
    pub bar_w: f32,
    /// Absolute index of the first bar in the rendered data slice.
    pub start_idx: usize,
}

impl PriceViewGeometry {
    /// Screen x of an absolute bar index's candle center — signed and
    /// unconditional, so off-viewport bars map to off-viewport pixels instead
    /// of being unmappable (the painter clips; interactions clamp).
    pub fn bar_to_x(&self, bar_idx: usize) -> f32 {
        self.data_left + ((bar_idx as i64 - self.start_idx as i64) as f32 + 0.5) * self.bar_w
    }

    /// Fractional absolute bar position for a screen x (candle centers land on
    /// `.0`; may be negative when x is left of bar 0's slot).
    pub fn x_to_bar_f(&self, x: f32) -> f64 {
        self.start_idx as f64 + ((x - self.data_left) / self.bar_w.max(f32::EPSILON)) as f64 - 0.5
    }

    /// Nearest absolute bar index for a screen x, clamped to `[0, max_bar]`.
    pub fn x_to_bar(&self, x: f32, max_bar: usize) -> usize {
        self.x_to_bar_f(x).round().clamp(0.0, max_bar as f64) as usize
    }
    pub fn price_to_y(&self, p: f64) -> f32 {
        let frac = if self.log_scale {
            let log_max = self.price_max.ln();
            let log_min = self.price_min.ln();
            let log_range = log_max - log_min;
            if log_range.abs() < f64::EPSILON {
                0.5
            } else {
                (log_max - p.max(0.001).ln()) / log_range
            }
        } else {
            (self.price_max - p) / (self.price_max - self.price_min)
        };
        self.chart_rect.top() + frac as f32 * self.chart_rect.height()
    }

    pub fn price_from_y(&self, y: f32) -> f64 {
        let h = self.chart_rect.height().max(f32::EPSILON);
        let frac = ((y - self.chart_rect.top()) / h) as f64;
        if self.log_scale {
            let log_max = self.price_max.ln();
            let log_min = self.price_min.ln();
            (log_max - frac * (log_max - log_min)).exp()
        } else {
            self.price_max - frac * (self.price_max - self.price_min)
        }
    }

    /// New price after dragging a horizontal line at `price` by `dy` pixels —
    /// exact under both linear and log scales.
    pub fn drag_price(&self, price: f64, dy: f32) -> f64 {
        self.price_from_y(self.price_to_y(price) + dy)
    }
}

/// Draw a single chart viewport into `rect` using `painter`.
pub fn draw_chart(
    painter: &egui::Painter,
    chart: &ChartState,
    rect: egui::Rect,
    crosshair: Option<egui::Pos2>,
    flags: &IndicatorFlags,
    show_rsi: bool,
    show_fisher: bool,
    show_macd: bool,
    show_volume_pane: bool,
    show_stochastic: bool,
    show_adx: bool,
    show_cci: bool,
    show_williams_r: bool,
    show_obv: bool,
    show_momentum: bool,
    show_cmo: bool,
    show_qstick: bool,
    show_disparity: bool,
    show_bop: bool,
    show_stddev: bool,
    show_mfi: bool,
    show_trix: bool,
    show_ppo: bool,
    show_ultosc: bool,
    show_stochrsi: bool,
    show_var_oscillator: bool,
    show_better_volume: bool,
    show_ehlers_ebsw: bool,
    show_ehlers_cyber: bool,
    show_ehlers_cg: bool,
    show_ehlers_roof: bool,
    show_squeeze: bool,
    sl_price: Option<f64>,
    tp_price: Option<f64>,
    active_position_avg_price: Option<f64>,
    trade_overlay: &TradeOverlay,
    alerts: &[(f64, String)],
    regulatory_alerts: &[typhoon_engine::core::regulatory_alerts::RegulatoryAlert],
    draw_mode: &DrawMode,
    company_name: Option<&str>,
) -> Option<PriceViewGeometry> {
    // Do not early-return for a stable chart. egui is immediate-mode: if this
    // function skips painting for a frame, the chart area can be left blank or
    // appear to flicker when the closed-market/auto-source chart is merely being
    // hovered, price-scaled, or panned. The old `last_rendered_*` fast path was
    // only safe with a retained render target, which we do not have here.

    // Heavy sync early-out: do near-O(1) work only during backfill
    if chart.heavy_sync_in_progress {
        painter.rect_filled(rect, 0.0, BG);
        return None;
    }
    // Update the "last rendered" snapshot for next frame
    // (we mutate through &mut via interior mutability or by accepting &mut ChartState
    // in a real caller; for now we just document the intent).
    // In practice the render loop should call chart.last_rendered_gen = chart.visible_bars_gen etc after draw.

    // ── background ──────────────────────────────────────────────────────────
    painter.rect_filled(rect, 0.0, BG);

    let (start_idx, end_idx, first_bar_slot, visible_slot_count) = chart.visible_slot_window();
    let bars = &chart.bars[start_idx..end_idx];

    if bars.is_empty() {
        // Show the live bar-fetch path so users know data is on the way.
        let sym = chart.symbol.as_str();
        let line1 = format!("No data for {}", sym);
        let line2 = "Fetching bars from Kraken when available; chart refreshes after cache update"
            .to_string();
        painter.text(
            rect.center() - egui::vec2(0.0, 12.0),
            egui::Align2::CENTER_CENTER,
            line1,
            egui::FontId::proportional(16.0),
            egui::Color32::from_rgb(180, 180, 200),
        );
        painter.text(
            rect.center() + egui::vec2(0.0, 10.0),
            egui::Align2::CENTER_CENTER,
            line2,
            egui::FontId::proportional(12.0),
            egui::Color32::from_rgb(110, 110, 130),
        );
        return None;
    }

    // Allocate sub-pane space at bottom
    let sub_pane_count = show_rsi as u8
        + show_fisher as u8
        + show_macd as u8
        + show_volume_pane as u8
        + show_stochastic as u8
        + show_adx as u8
        + show_cci as u8
        + show_williams_r as u8
        + show_obv as u8
        + show_momentum as u8
        + show_cmo as u8
        + show_qstick as u8
        + show_disparity as u8
        + show_bop as u8
        + show_stddev as u8
        + show_mfi as u8
        + show_trix as u8
        + show_ppo as u8
        + show_ultosc as u8
        + show_stochrsi as u8
        + show_var_oscillator as u8
        + show_better_volume as u8
        + show_ehlers_ebsw as u8
        + show_ehlers_cyber as u8
        + show_ehlers_cg as u8
        + show_ehlers_roof as u8
        + show_squeeze as u8;
    pub(crate) const SUB_PANE_H: f32 = CHART_SUB_PANE_H; // Height per indicator sub-pane (RSI, Fisher, MACD, Volume)
    pub(crate) const MIN_MAIN_CHART_H: f32 = CHART_MIN_MAIN_CHART_H;
    // When user is interacting, some expensive sub-pane rendering can be skipped in future passes
    let sub_pane_height = if sub_pane_count > 0 {
        // Keep the main price chart valid even when many sub-panes are enabled
        // or the window is temporarily tiny during startup/layout restore. A
        // negative-height chart rect makes later f32::clamp calls panic
        // (`min > max`). The sub-panes may overflow/clipped below, but the app
        // must never crash because indicator height exceeded available space.
        (SUB_PANE_H * sub_pane_count as f32).min((rect.height() - MIN_MAIN_CHART_H).max(0.0))
    } else {
        0.0
    };
    let main_rect = egui::Rect::from_min_max(
        rect.min,
        egui::pos2(rect.right(), rect.bottom() - sub_pane_height),
    );

    // Price axis margins
    let price_axis_w = PRICE_AXIS_W;
    let time_axis_h = 24.0_f32;
    let chart_rect = egui::Rect::from_min_max(
        main_rect.min,
        egui::pos2(
            main_rect.right() - price_axis_w,
            main_rect.bottom() - time_axis_h,
        ),
    );

    // Price axis background (subtle — indicates it's interactive like TradingView)
    let price_axis_bg = egui::Rect::from_min_max(
        egui::pos2(chart_rect.right(), chart_rect.top()),
        egui::pos2(rect.right(), chart_rect.bottom()),
    );
    painter.rect_filled(price_axis_bg, 0.0, egui::Color32::from_rgb(6, 6, 10));
    // Thin separator line between chart and price axis
    painter.line_segment(
        [
            egui::pos2(chart_rect.right(), chart_rect.top()),
            egui::pos2(chart_rect.right(), chart_rect.bottom()),
        ],
        egui::Stroke::new(1.0, egui::Color32::from_rgb(25, 30, 45)),
    );
    // Subtle drag handle indicator (3 horizontal lines at center of price axis)
    if let Some(cross) = crosshair {
        if cross.x > chart_rect.right() && cross.x < rect.right() {
            let cx = chart_rect.right() + price_axis_w * 0.5;
            let cy = price_axis_bg.center().y;
            for dy in [-4.0_f32, 0.0, 4.0] {
                painter.line_segment(
                    [egui::pos2(cx - 6.0, cy + dy), egui::pos2(cx + 6.0, cy + dy)],
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 70, 90)),
                );
            }
        }
    }

    // ── price range ─────────────────────────────────────────────────────────
    let mut price_min = bars.iter().map(|b| b.low).fold(f64::MAX, f64::min);
    let mut price_max = bars.iter().map(|b| b.high).fold(f64::MIN, f64::max);

    // The right-axis current line should follow the freshest live quote, not
    // whichever timeframe's cached/forming bar last happened to repaint. In
    // MTF Grid that prevents H1/H4/D1/W1 panes from showing a stale current
    // tag while the bid/ask tags already reflect Kraken WS v2 L2 top-of-book.
    let fresh_live_mid = chart.fresh_live_quote_mid();
    if let Some(mid) = fresh_live_mid {
        price_min = price_min.min(mid).min(chart.live_bid).min(chart.live_ask);
        price_max = price_max.max(mid).max(chart.live_bid).max(chart.live_ask);
    }
    if chart.ext_active && chart.ext_close > 0.0 {
        price_min = price_min
            .min(chart.ext_open)
            .min(chart.ext_high)
            .min(chart.ext_low)
            .min(chart.ext_close);
        price_max = price_max
            .max(chart.ext_open)
            .max(chart.ext_high)
            .max(chart.ext_low)
            .max(chart.ext_close);
    }

    // Also account for indicator values in visible range
    for i in start_idx..end_idx {
        if flags.sma200 {
            if let Some(v) = indicator_value_at(&chart.sma200, i) {
                price_min = price_min.min(v);
                price_max = price_max.max(v);
            }
        }
        if flags.sma100 {
            if let Some(v) = indicator_value_at(&chart.sma100, i) {
                price_min = price_min.min(v);
                price_max = price_max.max(v);
            }
        }
        if flags.kama {
            if let Some(v) = indicator_value_at(&chart.kama, i) {
                price_min = price_min.min(v);
                price_max = price_max.max(v);
            }
        }
        if flags.ema21 {
            if let Some(v) = indicator_value_at(&chart.ema21, i) {
                price_min = price_min.min(v);
                price_max = price_max.max(v);
            }
        }
        if flags.bollinger {
            if let Some(v) = indicator_value_at(&chart.bb_upper, i) {
                price_max = price_max.max(v);
            }
            if let Some(v) = indicator_value_at(&chart.bb_lower, i) {
                price_min = price_min.min(v);
            }
        }
        if flags.ichimoku {
            if let Some(v) = indicator_value_at(&chart.ichi_span_a, i) {
                price_min = price_min.min(v);
                price_max = price_max.max(v);
            }
            if let Some(v) = indicator_value_at(&chart.ichi_span_b, i) {
                price_min = price_min.min(v);
                price_max = price_max.max(v);
            }
        }
    }

    let padding = if chart.manual_view_override {
        (price_max - price_min) * 0.02
    } else {
        (price_max - price_min) * 0.05
    };
    price_min -= padding;
    price_max += padding;

    // Vertical camera. In manual/free-look mode the camera's explicit price
    // center/span is authoritative; do not reconstruct it through legacy
    // price_pan/price_zoom clamps. This is what lets body-drag behave like
    // TradingView: the visible price window can be panned 1:1 and can cross
    // through zero into negative values for penny stocks/indicators.
    if chart.manual_view_override {
        if let Some((camera_min, camera_max)) = chart.visible_price_range() {
            price_min = camera_min;
            price_max = camera_max;
        } else if let Some((min, max)) = chart.camera.explicit_price_range() {
            price_min = min;
            price_max = max;
        } else {
            // Still in manual mode but camera not yet populated — use legacy as last resort
            let range = price_max - price_min;
            let centre = (price_max + price_min) * 0.5 + chart.price_pan;
            let half = range * 0.5 / chart.price_zoom.max(f64::EPSILON);
            price_min = centre - half;
            price_max = centre + half;
        }
    } else {
        let range = price_max - price_min;
        let centre = (price_max + price_min) * 0.5 + chart.price_pan;
        let half = range * 0.5 / chart.price_zoom.max(f64::EPSILON);
        price_min = centre - half;
        price_max = centre + half;
    }

    if (price_max - price_min).abs() < f64::EPSILON {
        return None;
    }

    let use_log = chart.log_scale && price_min > 0.0; // log scale requires positive prices
    // Precompute the log-axis constants once. price_to_y is called once per visible
    // bar per indicator (~hundreds per frame), so hoisting the two `.ln()` calls out
    // of the closure turns a per-call cost into a per-frame cost.
    let log_max = if use_log { price_max.ln() } else { 0.0 };
    let log_min = if use_log { price_min.ln() } else { 0.0 };
    let log_range = log_max - log_min;
    let log_range_degenerate = use_log && log_range.abs() < f64::EPSILON;
    let linear_range = price_max - price_min;
    let chart_top = chart_rect.top();
    let chart_h = chart_rect.height();
    let price_to_y = |p: f64| -> f32 {
        let frac = if use_log {
            if log_range_degenerate {
                0.5
            } else {
                (log_max - p.max(0.001).ln()) / log_range
            }
        } else {
            (price_max - p) / linear_range
        };
        chart_top + frac as f32 * chart_h
    };
    // The exact mapping this frame paints with — returned so input hit-testing
    // (SL/TP line drags) can agree with the rendered pixels instead of
    // re-deriving an approximation.
    // ── bar width ────────────────────────────────────────────────────────────
    // Horizontal camera: visible_slot_count is the full virtual viewport, while
    // `bars` is only the intersecting real-data slice. first_bar_slot offsets
    // the real bars inside the viewport so panning beyond either edge produces
    // real empty chart space instead of stretching/clamping candles.
    let n_bars = visible_slot_count.max(1) as f32;
    let bar_w = (chart_rect.width() / n_bars).max(1.0);
    let data_left = chart_rect.left() + first_bar_slot * bar_w;
    let price_geometry = PriceViewGeometry {
        chart_rect,
        price_min,
        price_max,
        log_scale: use_log,
        data_left,
        bar_w,
        start_idx,
    };
    let candle_w = (bar_w * 0.7).max(1.0);
    let half_body = candle_w * 0.5;
    let render_step = chart_render_sample_step(bars.len(), chart_rect.width());
    let fill_half_w = (bar_w * render_step as f32 * 0.5).max(bar_w * 0.5);

    // ── session highlighting (Asian / London / New York) ────────────────────
    // Batched: find contiguous session blocks and draw one rect per block (not per bar).
    if flags.sessions {
        let session_asian = egui::Color32::from_rgba_premultiplied(40, 60, 120, 18);
        let session_london = egui::Color32::from_rgba_premultiplied(60, 120, 60, 18);
        let session_ny = egui::Color32::from_rgba_premultiplied(120, 60, 40, 18);
        let tf_minutes = chart.timeframe.minutes();
        if tf_minutes < 240 {
            // For each session, find contiguous blocks and draw one rect per block
            let sessions: &[(u32, u32, egui::Color32)] = &[
                (0, 540, session_asian),
                (420, 960, session_london),
                (810, 1200, session_ny),
            ];
            for &(start_hm, end_hm, color) in sessions {
                let mut block_start: Option<usize> = None;
                for i in 0..=bars.len() {
                    let in_session = if i < bars.len() {
                        let secs = bars[i].ts_ms / 1000;
                        let day_secs = ((secs % 86400) + 86400) % 86400;
                        let hm = (day_secs / 60) as u32;
                        hm >= start_hm && hm < end_hm
                    } else {
                        false
                    };
                    if in_session && block_start.is_none() {
                        block_start = Some(i);
                    } else if !in_session {
                        let bs = match block_start {
                            Some(v) => v,
                            None => continue,
                        };
                        let x1 = data_left + bs as f32 * bar_w;
                        let x2 = (data_left + i as f32 * bar_w).min(chart_rect.right());
                        painter.rect_filled(
                            egui::Rect::from_min_max(
                                egui::pos2(x1, chart_rect.top()),
                                egui::pos2(x2, chart_rect.bottom()),
                            ),
                            0.0,
                            color,
                        );
                        block_start = None;
                    }
                }
            }
        }
    }

    // ── grid lines (price) ──────────────────────────────────────────────────
    // Use one faint line per grid level. The old dotted grid emitted hundreds to
    // thousands of tiny line-segment shapes every frame on large charts, which is
    // pure UI overhead during drag/zoom. Solid low-alpha grid lines keep the same
    // spatial reference with a tiny, fixed primitive count.
    let grid_steps = 8;
    let grid_col = egui::Color32::from_rgb(33, 33, 33);
    let grid_stroke = egui::Stroke::new(0.5, grid_col);
    let mut label_buf = String::with_capacity(16); // reuse buffer across grid labels (avoids heap alloc per label per frame)
    for i in 0..=grid_steps {
        let p = price_min + (price_max - price_min) * (i as f64 / grid_steps as f64);
        let y = price_to_y(p);
        painter.line_segment(
            [
                egui::pos2(chart_rect.left(), y),
                egui::pos2(chart_rect.right(), y),
            ],
            grid_stroke,
        );
        format_price_buf(p, &mut label_buf);
        painter.text(
            egui::pos2(chart_rect.right() + 4.0, y),
            egui::Align2::LEFT_CENTER,
            &label_buf,
            egui::FontId::monospace(10.0),
            AXIS_TEXT,
        );
    }

    // ── grid lines (time) ────────────────────────────────────────────────────
    // Intraday axes get hierarchical, boundary-aligned labels (a date on each
    // day rollover, HH:MM otherwise), so lower timeframes stop smearing the full
    // "%d %b'%y %H:%M" string onto every tick — the unreadable case in the H1/H4
    // screenshots. Daily-and-up keep the terse date-only labels that already read
    // cleanly per the higher-timeframe screenshot.
    if chart.timeframe.minutes() < 1440 {
        draw_intraday_time_axis(
            painter,
            bars,
            data_left,
            bar_w,
            chart_rect,
            chart.timeframe.minutes(),
            grid_stroke,
            &mut label_buf,
        );
    } else {
        let time_step = ((80.0 / bar_w) as usize).max(1);
        for (rel_idx, bar) in bars.iter().enumerate().step_by(time_step) {
            let x = data_left + (rel_idx as f32 + 0.5) * bar_w;
            painter.line_segment(
                [
                    egui::pos2(x, chart_rect.top()),
                    egui::pos2(x, chart_rect.bottom()),
                ],
                grid_stroke,
            );
            format_ts_buf(bar.ts_ms, chart.timeframe, &mut label_buf);
            painter.text(
                egui::pos2(x, chart_rect.bottom() + 2.0),
                egui::Align2::CENTER_TOP,
                &label_buf,
                egui::FontId::monospace(9.0),
                AXIS_TEXT,
            );
        }
    }

    // ── MA ribbon fill (KAMA vs SMA200) — only when single-TF lines are visible ──
    if flags.sma200 && flags.kama && chart.mtf_sma.is_empty() && chart.multi_kama.is_empty() {
        for (rel_idx, _) in bars.iter().enumerate().step_by(render_step) {
            let abs_idx = start_idx + rel_idx;
            if abs_idx >= chart.sma200.len() || abs_idx >= chart.kama.len() {
                continue;
            }
            if let (Some(sma_v), Some(kama_v)) = (
                indicator_value_at(&chart.sma200, abs_idx),
                indicator_value_at(&chart.kama, abs_idx),
            ) {
                let x = data_left + (rel_idx as f32 + 0.5) * bar_w;
                let y_sma = price_to_y(sma_v);
                let y_kama = price_to_y(kama_v);
                let (top, bot) = if y_sma < y_kama {
                    (y_sma, y_kama)
                } else {
                    (y_kama, y_sma)
                };
                if top <= chart_rect.bottom() && bot >= chart_rect.top() {
                    let fill = if kama_v > sma_v {
                        egui::Color32::from_rgba_premultiplied(0, 180, 60, 18) // bullish green
                    } else {
                        egui::Color32::from_rgba_premultiplied(180, 40, 0, 18) // bearish red
                    };
                    painter.rect_filled(
                        egui::Rect::from_min_max(
                            egui::pos2(x - fill_half_w, top.max(chart_rect.top())),
                            egui::pos2(x + fill_half_w, bot.min(chart_rect.bottom())),
                        ),
                        0.0,
                        fill,
                    );
                }
            }
        }
    }

    // ── Bollinger Band fill ──────────────────────────────────────────────────
    if flags.bollinger {
        // Build polygon directly: upper points forward, lower points reversed — no clone needed.
        // Dense views use the same pixel-aware decimation as line/candle rendering.
        let mut fill_points_upper: Vec<egui::Pos2> =
            Vec::with_capacity(bars.len() / render_step + 1);
        let mut fill_points_lower: Vec<egui::Pos2> =
            Vec::with_capacity(bars.len() / render_step + 1);
        for (rel_idx, _) in bars.iter().enumerate().step_by(render_step) {
            let abs_idx = start_idx + rel_idx;
            if abs_idx >= chart.bb_upper.len() {
                continue;
            }
            if let (Some(u), Some(l)) = (
                indicator_value_at(&chart.bb_upper, abs_idx),
                indicator_value_at(&chart.bb_lower, abs_idx),
            ) {
                let x = data_left + (rel_idx as f32 + 0.5) * bar_w;
                let yu = price_to_y(u);
                let yl = price_to_y(l);
                if yu >= chart_rect.top() && yl <= chart_rect.bottom() {
                    fill_points_upper.push(egui::pos2(x, yu));
                    fill_points_lower.push(egui::pos2(x, yl));
                }
            }
        }
        if fill_points_upper.len() > 1 {
            let mut poly = Vec::with_capacity(fill_points_upper.len() + fill_points_lower.len());
            poly.extend_from_slice(&fill_points_upper);
            poly.extend(fill_points_lower.iter().rev());
            painter.add(egui::Shape::convex_polygon(
                poly,
                BB_FILL,
                egui::Stroke::NONE,
            ));
        }
        draw_indicator_line(
            painter,
            chart_rect,
            data_left,
            bars,
            &chart.bb_upper,
            start_idx,
            bar_w,
            &price_to_y,
            BB_COL,
            1.0,
        );
        draw_indicator_line(
            painter,
            chart_rect,
            data_left,
            bars,
            &chart.bb_lower,
            start_idx,
            bar_w,
            &price_to_y,
            BB_COL,
            1.0,
        );
        draw_indicator_line(
            painter,
            chart_rect,
            data_left,
            bars,
            &chart.bb_mid,
            start_idx,
            bar_w,
            &price_to_y,
            BB_COL,
            0.5,
        );
    }

    // ── VWAP with deviation bands ───────────────────────────────────────────
    if flags.vwap {
        let vwap_col = egui::Color32::from_rgb(255, 215, 0); // gold
        let band_col1 = egui::Color32::from_rgba_premultiplied(100, 149, 237, 50); // cornflower blue
        let band_col2 = egui::Color32::from_rgba_premultiplied(100, 149, 237, 30);
        let band_col3 = egui::Color32::from_rgba_premultiplied(100, 149, 237, 15);
        // Fill bands (3σ first, then 2σ, then 1σ so inner is on top)
        for (upper, lower, fill_col) in [
            (&chart.vwap_upper3, &chart.vwap_lower3, band_col3),
            (&chart.vwap_upper2, &chart.vwap_lower2, band_col2),
            (&chart.vwap_upper1, &chart.vwap_lower1, band_col1),
        ] {
            for (rel_idx, _) in bars.iter().enumerate().step_by(render_step) {
                let abs_idx = start_idx + rel_idx;
                if abs_idx >= upper.len() {
                    continue;
                }
                if let (Some(u), Some(l)) = (
                    indicator_value_at(&upper, abs_idx),
                    indicator_value_at(&lower, abs_idx),
                ) {
                    let x = data_left + (rel_idx as f32 + 0.5) * bar_w;
                    let yu = price_to_y(u);
                    let yl = price_to_y(l);
                    let (top, bot) = if yu < yl { (yu, yl) } else { (yl, yu) };
                    if top <= chart_rect.bottom() && bot >= chart_rect.top() {
                        painter.rect_filled(
                            egui::Rect::from_min_max(
                                egui::pos2(x - fill_half_w, top.max(chart_rect.top())),
                                egui::pos2(x + fill_half_w, bot.min(chart_rect.bottom())),
                            ),
                            0.0,
                            fill_col,
                        );
                    }
                }
            }
        }
        // VWAP line
        draw_indicator_line(
            painter,
            chart_rect,
            data_left,
            bars,
            &chart.vwap,
            start_idx,
            bar_w,
            &price_to_y,
            vwap_col,
            2.0,
        );
        // Band edge lines
        let band_line = egui::Color32::from_rgba_premultiplied(100, 149, 237, 80);
        draw_indicator_line(
            painter,
            chart_rect,
            data_left,
            bars,
            &chart.vwap_upper1,
            start_idx,
            bar_w,
            &price_to_y,
            band_line,
            0.5,
        );
        draw_indicator_line(
            painter,
            chart_rect,
            data_left,
            bars,
            &chart.vwap_lower1,
            start_idx,
            bar_w,
            &price_to_y,
            band_line,
            0.5,
        );
        draw_indicator_line(
            painter,
            chart_rect,
            data_left,
            bars,
            &chart.vwap_upper2,
            start_idx,
            bar_w,
            &price_to_y,
            band_line,
            0.5,
        );
        draw_indicator_line(
            painter,
            chart_rect,
            data_left,
            bars,
            &chart.vwap_lower2,
            start_idx,
            bar_w,
            &price_to_y,
            band_line,
            0.5,
        );
    }

    // ── Supertrend ─────────────────────────────────────────────────────────
    if flags.supertrend {
        let st_bull_col = egui::Color32::from_rgb(0, 200, 100);
        let st_bear_col = egui::Color32::from_rgb(220, 50, 50);
        // Draw as colored clipped segments — bull=green, bear=red.
        let mut prev: Option<(egui::Pos2, bool)> = None;
        for (rel_idx, _) in bars.iter().enumerate().step_by(render_step) {
            let abs_idx = start_idx + rel_idx;
            let Some(v) = indicator_value_at(&chart.supertrend, abs_idx) else {
                prev = None;
                continue;
            };
            let x = data_left + (rel_idx as f32 + 0.5) * bar_w;
            let pt = egui::pos2(x, price_to_y(v));
            let is_bull = chart.supertrend_bull.get(abs_idx).copied().unwrap_or(true);
            if let Some((prev_pt, prev_bull)) = prev {
                if let Some([a, b]) = clip_line_segment_to_rect(prev_pt, pt, chart_rect) {
                    let col = if prev_bull { st_bull_col } else { st_bear_col };
                    painter.line_segment([a, b], egui::Stroke::new(2.0, col));
                }
            }
            prev = Some((pt, is_bull));
        }
    }

    // ── Donchian Channels ────────────────────────────────────────────────
    if flags.donchian {
        let dc_col = egui::Color32::from_rgb(0, 180, 255);
        let dc_fill = egui::Color32::from_rgba_premultiplied(0, 180, 255, 15);
        // Fill between upper and lower
        for (rel_idx, _) in bars.iter().enumerate().step_by(render_step) {
            let abs_idx = start_idx + rel_idx;
            if abs_idx >= chart.donchian_upper.len() {
                continue;
            }
            if let (Some(u), Some(l)) = (
                indicator_value_at(&chart.donchian_upper, abs_idx),
                indicator_value_at(&chart.donchian_lower, abs_idx),
            ) {
                let x = data_left + (rel_idx as f32 + 0.5) * bar_w;
                let yu = price_to_y(u);
                let yl = price_to_y(l);
                if yu <= chart_rect.bottom() && yl >= chart_rect.top() {
                    painter.rect_filled(
                        egui::Rect::from_min_max(
                            egui::pos2(x - fill_half_w, yu.max(chart_rect.top())),
                            egui::pos2(x + fill_half_w, yl.min(chart_rect.bottom())),
                        ),
                        0.0,
                        dc_fill,
                    );
                }
            }
        }
        draw_indicator_line(
            painter,
            chart_rect,
            data_left,
            bars,
            &chart.donchian_upper,
            start_idx,
            bar_w,
            &price_to_y,
            dc_col,
            1.0,
        );
        draw_indicator_line(
            painter,
            chart_rect,
            data_left,
            bars,
            &chart.donchian_lower,
            start_idx,
            bar_w,
            &price_to_y,
            dc_col,
            1.0,
        );
    }

    // ── Keltner Channels ─────────────────────────────────────────────────
    if flags.keltner {
        let kc_col = egui::Color32::from_rgb(255, 165, 0); // orange
        let kc_fill = egui::Color32::from_rgba_premultiplied(255, 165, 0, 15);
        for (rel_idx, _) in bars.iter().enumerate().step_by(render_step) {
            let abs_idx = start_idx + rel_idx;
            if abs_idx >= chart.keltner_upper.len() {
                continue;
            }
            if let (Some(u), Some(l)) = (
                indicator_value_at(&chart.keltner_upper, abs_idx),
                indicator_value_at(&chart.keltner_lower, abs_idx),
            ) {
                let x = data_left + (rel_idx as f32 + 0.5) * bar_w;
                let yu = price_to_y(u);
                let yl = price_to_y(l);
                if yu <= chart_rect.bottom() && yl >= chart_rect.top() {
                    painter.rect_filled(
                        egui::Rect::from_min_max(
                            egui::pos2(x - fill_half_w, yu.max(chart_rect.top())),
                            egui::pos2(x + fill_half_w, yl.min(chart_rect.bottom())),
                        ),
                        0.0,
                        kc_fill,
                    );
                }
            }
        }
        draw_indicator_line(
            painter,
            chart_rect,
            data_left,
            bars,
            &chart.keltner_upper,
            start_idx,
            bar_w,
            &price_to_y,
            kc_col,
            1.0,
        );
        draw_indicator_line(
            painter,
            chart_rect,
            data_left,
            bars,
            &chart.keltner_lower,
            start_idx,
            bar_w,
            &price_to_y,
            kc_col,
            1.0,
        );
        draw_indicator_line(
            painter,
            chart_rect,
            data_left,
            bars,
            &chart.keltner_mid,
            start_idx,
            bar_w,
            &price_to_y,
            kc_col,
            0.5,
        );
    }

    // ── Regression Channel ─────────────────────────────────────────────────
    if flags.regression {
        let rc_col = egui::Color32::from_rgb(180, 130, 255); // light purple
        let rc_fill = egui::Color32::from_rgba_premultiplied(180, 130, 255, 15);
        for (rel_idx, _) in bars.iter().enumerate().step_by(render_step) {
            let abs_idx = start_idx + rel_idx;
            if abs_idx >= chart.regression_upper.len() {
                continue;
            }
            if let (Some(u), Some(l)) = (
                indicator_value_at(&chart.regression_upper, abs_idx),
                indicator_value_at(&chart.regression_lower, abs_idx),
            ) {
                let x = data_left + (rel_idx as f32 + 0.5) * bar_w;
                let yu = price_to_y(u);
                let yl = price_to_y(l);
                if yu <= chart_rect.bottom() && yl >= chart_rect.top() {
                    painter.rect_filled(
                        egui::Rect::from_min_max(
                            egui::pos2(x - fill_half_w, yu.max(chart_rect.top())),
                            egui::pos2(x + fill_half_w, yl.min(chart_rect.bottom())),
                        ),
                        0.0,
                        rc_fill,
                    );
                }
            }
        }
        draw_indicator_line(
            painter,
            chart_rect,
            data_left,
            bars,
            &chart.regression_upper,
            start_idx,
            bar_w,
            &price_to_y,
            rc_col,
            0.8,
        );
        draw_indicator_line(
            painter,
            chart_rect,
            data_left,
            bars,
            &chart.regression_lower,
            start_idx,
            bar_w,
            &price_to_y,
            rc_col,
            0.8,
        );
        draw_indicator_line(
            painter,
            chart_rect,
            data_left,
            bars,
            &chart.regression_mid,
            start_idx,
            bar_w,
            &price_to_y,
            rc_col,
            1.5,
        );
    }

    // ── Ichimoku cloud ─────────────────────────────────────────────────────
    if flags.ichimoku {
        // Cloud fill between Span A and Span B
        for (rel_idx, _) in bars.iter().enumerate().step_by(render_step) {
            let abs_idx = start_idx + rel_idx;
            if abs_idx >= chart.ichi_span_a.len() {
                continue;
            }
            if let (Some(a), Some(b)) = (
                indicator_value_at(&chart.ichi_span_a, abs_idx),
                indicator_value_at(&chart.ichi_span_b, abs_idx),
            ) {
                let x = data_left + (rel_idx as f32 + 0.5) * bar_w;
                let ya = price_to_y(a);
                let yb = price_to_y(b);
                let color = if a >= b {
                    ICHI_CLOUD_BULL
                } else {
                    ICHI_CLOUD_BEAR
                };
                let (top, bot) = if ya < yb { (ya, yb) } else { (yb, ya) };
                if top <= chart_rect.bottom() && bot >= chart_rect.top() {
                    painter.rect_filled(
                        egui::Rect::from_min_max(
                            egui::pos2(x - fill_half_w, top.max(chart_rect.top())),
                            egui::pos2(x + fill_half_w, bot.min(chart_rect.bottom())),
                        ),
                        0.0,
                        color,
                    );
                }
            }
        }
        draw_indicator_line(
            painter,
            chart_rect,
            data_left,
            bars,
            &chart.ichi_tenkan,
            start_idx,
            bar_w,
            &price_to_y,
            ICHI_TENKAN,
            1.0,
        );
        draw_indicator_line(
            painter,
            chart_rect,
            data_left,
            bars,
            &chart.ichi_kijun,
            start_idx,
            bar_w,
            &price_to_y,
            ICHI_KIJUN,
            1.0,
        );
        draw_indicator_line(
            painter,
            chart_rect,
            data_left,
            bars,
            &chart.ichi_span_a,
            start_idx,
            bar_w,
            &price_to_y,
            ICHI_SPAN_A,
            0.8,
        );
        draw_indicator_line(
            painter,
            chart_rect,
            data_left,
            bars,
            &chart.ichi_span_b,
            start_idx,
            bar_w,
            &price_to_y,
            ICHI_SPAN_B,
            0.8,
        );
    }

    // ── indicator lines ──────────────────────────────────────────────────────
    // Current-TF SMA200: only show if NO MTF_MA data exists (MTF_MA replaces it in NNFX mode)
    if draw_current_sma200_overlay(flags.sma200, !chart.mtf_sma.is_empty()) {
        draw_indicator_line(
            painter,
            chart_rect,
            data_left,
            bars,
            &chart.sma200,
            start_idx,
            bar_w,
            &price_to_y,
            SMA200_COL,
            1.5,
        );
    }
    if flags.sma100 {
        draw_indicator_line(
            painter,
            chart_rect,
            data_left,
            bars,
            &chart.sma100,
            start_idx,
            bar_w,
            &price_to_y,
            SMA100_COL,
            1.5,
        );
    }
    // Current-TF KAMA: only show if NO MultiKAMA HTF data exists
    if draw_current_kama_overlay(flags.kama, !chart.multi_kama.is_empty()) {
        draw_indicator_line(
            painter,
            chart_rect,
            data_left,
            bars,
            &chart.kama,
            start_idx,
            bar_w,
            &price_to_y,
            KAMA_COL,
            1.5,
        );
    }
    // MultiKAMA: higher TF KAMAs (MT5: clrWhite for KAMA, but visually distinguished)
    // MTF SMA lines (matching MTF_MA.mqh: H1/200, H4/200, D1/200, W1/200, W1/100, MN1/100)
    if flags.sma200 && !chart.mtf_sma.is_empty() {
        // Colors matching MTF_MA.mqh SetIndexStyle (lines 226-231)
        for (label, projected) in &chart.mtf_sma {
            let color = match label.as_str() {
                "H1 200" => egui::Color32::from_rgb(255, 99, 71), // clrTomato
                _ => egui::Color32::from_rgb(255, 0, 255),        // clrMagenta (all others)
            };
            let mut prev: Option<egui::Pos2> = None;
            for &(bar_idx, sma_val) in projected {
                if bar_idx >= start_idx && bar_idx < end_idx {
                    let rel = bar_idx - start_idx;
                    let x = data_left + (rel as f32 + 0.5) * bar_w;
                    let pt = egui::pos2(x, price_to_y(sma_val));
                    if let Some(prev_pt) = prev {
                        if let Some([a, b]) = clip_line_segment_to_rect(prev_pt, pt, chart_rect) {
                            painter.line_segment([a, b], egui::Stroke::new(2.0, color));
                        }
                    }
                    prev = Some(pt);
                }
            }
        }
    }

    // MQL4 mode uses white for all; MTF_MA overlay uses magenta for higher TFs
    if flags.kama && !chart.multi_kama.is_empty() {
        // MultiKAMA: ALL WHITE (matching MT5 MultiKAMA.mqh SetIndexStyle lines 59-63)
        let htf_colors = [
            egui::Color32::from_rgb(255, 255, 255), // H1 — white (clrWhite)
            egui::Color32::from_rgb(255, 255, 255), // H4 — white (clrWhite)
            egui::Color32::from_rgb(255, 255, 255), // D1 — white (clrWhite)
            egui::Color32::from_rgb(255, 255, 255), // W1 — white (clrWhite)
            egui::Color32::from_rgb(255, 255, 255), // MN1 — white (clrWhite)
        ];
        for (tf_idx, (_tf_label, projected)) in chart.multi_kama.iter().enumerate() {
            let color = htf_colors
                .get(tf_idx)
                .copied()
                .unwrap_or(egui::Color32::from_rgb(255, 0, 255));
            let mut prev: Option<egui::Pos2> = None;
            for &(bar_idx, kama_val) in projected {
                if bar_idx >= start_idx && bar_idx < end_idx {
                    let rel = bar_idx - start_idx;
                    let x = data_left + (rel as f32 + 0.5) * bar_w;
                    let pt = egui::pos2(x, price_to_y(kama_val));
                    if let Some(prev_pt) = prev {
                        if let Some([a, b]) = clip_line_segment_to_rect(prev_pt, pt, chart_rect) {
                            painter.line_segment([a, b], egui::Stroke::new(2.0, color));
                        }
                    }
                    prev = Some(pt);
                }
            }
        }
    }
    if flags.ema21 {
        draw_indicator_line(
            painter,
            chart_rect,
            data_left,
            bars,
            &chart.ema21,
            start_idx,
            bar_w,
            &price_to_y,
            EMA_COL,
            1.5,
        );
    }
    if flags.wma {
        draw_indicator_line(
            painter,
            chart_rect,
            data_left,
            bars,
            &chart.wma,
            start_idx,
            bar_w,
            &price_to_y,
            WMA_COL,
            1.0,
        );
    }
    if flags.hma {
        draw_indicator_line(
            painter,
            chart_rect,
            data_left,
            bars,
            &chart.hma,
            start_idx,
            bar_w,
            &price_to_y,
            HMA_COL,
            1.5,
        );
    }

    // ATR Projection lines + labels are drawn AFTER the supply/demand & FVG zones
    // (see below, just past draw_post_zone_trend_overlays) so the translucent zone
    // fills can't tint the yellow band or its text — same layering as the MAs.

    // Parabolic SAR dots. Dense zoomed-out views cannot distinguish one dot per
    // historical bar, so sample at viewport density like candles/indicator lines.
    if flags.psar {
        for (rel_idx, _) in bars.iter().enumerate().step_by(render_step) {
            let abs_idx = start_idx + rel_idx;
            if abs_idx >= chart.psar.len() {
                continue;
            }
            if let Some(sar) = indicator_value_at(&chart.psar, abs_idx) {
                let x = data_left + (rel_idx as f32 + 0.5) * bar_w;
                let y = price_to_y(sar);
                if y >= chart_rect.top() && y <= chart_rect.bottom() {
                    painter.circle_filled(egui::pos2(x, y), 2.0, SAR_COL);
                }
            }
        }
    }

    // ── Ehlers overlay indicators ───────────────────────────────────────────
    if flags.ehlers_ss {
        draw_indicator_line(
            painter,
            chart_rect,
            data_left,
            bars,
            &chart.ehlers_ss,
            start_idx,
            bar_w,
            &price_to_y,
            EHLERS_SS_COL,
            1.5,
        );
    }
    if flags.ehlers_decycler {
        draw_indicator_line(
            painter,
            chart_rect,
            data_left,
            bars,
            &chart.ehlers_decycler,
            start_idx,
            bar_w,
            &price_to_y,
            EHLERS_DEC_COL,
            1.5,
        );
    }
    if flags.ehlers_itl {
        draw_indicator_line(
            painter,
            chart_rect,
            data_left,
            bars,
            &chart.ehlers_itl,
            start_idx,
            bar_w,
            &price_to_y,
            EHLERS_ITL_COL,
            1.5,
        );
    }
    if flags.ehlers_mama {
        draw_indicator_line(
            painter,
            chart_rect,
            data_left,
            bars,
            &chart.ehlers_mama,
            start_idx,
            bar_w,
            &price_to_y,
            EHLERS_MAMA_COL,
            1.5,
        );
        draw_indicator_line(
            painter,
            chart_rect,
            data_left,
            bars,
            &chart.ehlers_fama,
            start_idx,
            bar_w,
            &price_to_y,
            EHLERS_FAMA_COL,
            1.0,
        );
    }

    // ── previous candle levels ─────────────────────────────────────────────
    if flags.prev_levels {
        // PreviousCandleLevels.mqh logic, but colour- and text-coded so the two
        // candle states are unambiguous (the .mqh had no labels and drew both in
        // magenta): MAGENTA "Prev …" = the last closed candle, CYAN "Cur …" = the
        // current/forming "Judas" candle. The 4th field is the highest chart
        // group_rank at which the level still draws (0 sub-hour, 1 hour, 2 day,
        // 3 week): the level shows only while the chart sits at or below it — so an
        // H1/H4 chart drops its own H1/H4 previous levels but keeps D/W/MN, a
        // weekly chart keeps only MN, current D1/W1 stay visible through the daily
        // chart and current MN1 through the weekly chart.
        let white = egui::Color32::WHITE;
        let magenta = egui::Color32::from_rgb(255, 0, 255);
        // Non-magenta levels are white; magenta (D/W/MN prev) drawn last so they
        // appear in foreground when multiple levels coincide at same price.
        let level_pairs = [
            // White (non-magenta) first — H1/H4 prev + all Cur levels.
            (chart.prev_h1_high, "Prev H1 Hi", white, 0u8),
            (chart.prev_h1_low, "Prev H1 Lo", white, 0),
            (chart.prev_h4_high, "Prev H4 Hi", white, 0),
            (chart.prev_h4_low, "Prev H4 Lo", white, 0),
            (chart.current_daily_high, "Cur D Hi", white, 2),
            (chart.current_daily_low, "Cur D Lo", white, 2),
            (chart.current_weekly_high, "Cur W Hi", white, 2),
            (chart.current_weekly_low, "Cur W Lo", white, 2),
            (chart.current_monthly_high, "Cur MN Hi", white, 3),
            (chart.current_monthly_low, "Cur MN Lo", white, 3),
            // Magenta (D/W/MN prev) last for foreground priority on overlaps.
            (chart.prev_daily_high, "Prev D Hi", magenta, 1),
            (chart.prev_daily_low, "Prev D Lo", magenta, 1),
            (chart.prev_weekly_high, "Prev W Hi", magenta, 2),
            (chart.prev_weekly_low, "Prev W Lo", magenta, 2),
            (chart.prev_monthly_high, "Prev MN Hi", magenta, 3),
            (chart.prev_monthly_low, "Prev MN Lo", magenta, 3),
        ];
        let chart_rank = chart.timeframe.group_rank();
        // Match ATR Projection line weight so Previous Candle Levels carry the
        // same visual priority when both MT5-style level overlays are enabled.
        let prev_level_stroke_width = 3.0;

        // Draw each level line at its true price immediately, but defer the text
        // so labels for levels that sit within a few ticks of each other (e.g.
        // H4 Hi vs D Hi) get spread into separate vertical bands instead of
        // overprinting into an unreadable smear.
        // Collect visible levels, then group by exact price so we can emit a
        // single comma-separated label (with per-segment colour) when multiple
        // levels land on the identical price.
        let mut visible: Vec<(f64, &'static str, egui::Color32)> = Vec::new();
        for (price_opt, label, color, max_rank) in &level_pairs {
            if chart_rank > *max_rank {
                continue;
            }
            if let Some(p) = price_opt {
                let y = price_to_y(*p);
                if y >= chart_rect.top() && y <= chart_rect.bottom() {
                    visible.push((*p, *label, *color));
                }
            }
        }

        // Sort by price so identical prices are adjacent
        visible.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        let mut label_bands: Vec<(f32, f32)> = Vec::new();
        let mut i = 0;
        while i < visible.len() {
            let price = visible[i].0;
            let y = price_to_y(price);

            // Collect all entries with this exact price
            let mut group: Vec<(&'static str, egui::Color32)> = Vec::new();
            while i < visible.len() && (visible[i].0 - price).abs() < f64::EPSILON {
                group.push((visible[i].1, visible[i].2));
                i += 1;
            }

            // Draw the horizontal line once (use first colour)
            if let Some((_, first_col)) = group.first() {
                painter.line_segment(
                    [
                        egui::pos2(chart_rect.left(), y),
                        egui::pos2(chart_rect.right(), y),
                    ],
                    egui::Stroke::new(prev_level_stroke_width, *first_col),
                );
            }

            // Place the label (de-conflict with other price bands)
            let center = place_level_label(
                y - 8.0,
                5.0,
                chart_rect.top(),
                chart_rect.bottom(),
                &mut label_bands,
            );

            // Render combined label with per-segment colour when >1 entry at same price
            let base_x = chart_rect.right() - 4.0;
            let mut x = base_x;
            let font = egui::FontId::monospace(8.0);

            for (idx, (lab, col)) in group.iter().enumerate() {
                if idx > 0 {
                    // comma separator (neutral colour)
                    let comma = painter.layout_no_wrap(
                        ", ".to_string(),
                        font.clone(),
                        egui::Color32::LIGHT_GRAY,
                    );
                    x -= comma.rect.width();
                    painter.galley(
                        egui::pos2(x, center - comma.rect.height() * 0.5),
                        comma,
                        egui::Color32::LIGHT_GRAY,
                    );
                    x -= 1.5;
                }

                let g = painter.layout_no_wrap(lab.to_string(), font.clone(), *col);
                x -= g.rect.width();
                painter.galley(egui::pos2(x, center - g.rect.height() * 0.5), g, *col);
            }
        }
    }

    // ── pivot points ──────────────────────────────────────────────────────
    if flags.pivots {
        let pivot_levels = [
            (chart.pivot_p, "P", egui::Color32::from_rgb(200, 200, 200)),
            (chart.pivot_r1, "R1", egui::Color32::from_rgb(200, 80, 80)),
            (chart.pivot_r2, "R2", egui::Color32::from_rgb(255, 40, 40)),
            (chart.pivot_s1, "S1", egui::Color32::from_rgb(80, 200, 80)),
            (chart.pivot_s2, "S2", egui::Color32::from_rgb(40, 255, 40)),
        ];
        for (price_opt, label, color) in &pivot_levels {
            if let Some(p) = price_opt {
                let y = price_to_y(*p);
                if y >= chart_rect.top() && y <= chart_rect.bottom() {
                    painter.line_segment(
                        [
                            egui::pos2(chart_rect.left(), y),
                            egui::pos2(chart_rect.right(), y),
                        ],
                        egui::Stroke::new(0.7, *color),
                    );
                    painter.text(
                        egui::pos2(chart_rect.left() + 2.0, y - 10.0),
                        egui::Align2::LEFT_TOP,
                        label,
                        egui::FontId::monospace(8.0),
                        *color,
                    );
                }
            }
        }
    }

    // ── fractals ─────────────────────────────────────────────────────────
    if flags.fractals {
        // Market structure: track prev swing high/low to label HH/HL/LH/LL
        let mut prev_swing_high: Option<f64> = None;
        let mut prev_swing_low: Option<f64> = None;
        // Scan all bars up to visible end to get accurate structure context
        let scan_start = start_idx.saturating_sub(50); // look back for prior swings
        for si in scan_start..start_idx {
            if si < chart.fractal_up.len() && chart.fractal_up[si] {
                prev_swing_high = Some(chart.bars[si].high);
            }
            if si < chart.fractal_down.len() && chart.fractal_down[si] {
                prev_swing_low = Some(chart.bars[si].low);
            }
        }
        let ms_font = egui::FontId::monospace(8.0);
        let fractal_font = egui::FontId::proportional(10.0);
        let min_structure_label_gap = if render_step > 1 { 12.0 } else { 0.0 };
        let mut last_high_label_x = f32::NEG_INFINITY;
        let mut last_low_label_x = f32::NEG_INFINITY;
        for (rel_idx, bar) in bars.iter().enumerate() {
            let abs_idx = start_idx + rel_idx;
            let x = data_left + (rel_idx as f32 + 0.5) * bar_w;
            if abs_idx < chart.fractal_up.len() && chart.fractal_up[abs_idx] {
                let y = price_to_y(bar.high) - 8.0;
                if y >= chart_rect.top() && x - last_high_label_x >= min_structure_label_gap {
                    painter.text(
                        egui::pos2(x, y),
                        egui::Align2::CENTER_BOTTOM,
                        "▲",
                        fractal_font.clone(),
                        UP,
                    );
                    // Market structure label
                    if let Some(prev_h) = prev_swing_high {
                        let (label, col) = if bar.high > prev_h {
                            ("HH", UP)
                        } else {
                            ("LH", DOWN)
                        };
                        painter.text(
                            egui::pos2(x, y - 10.0),
                            egui::Align2::CENTER_BOTTOM,
                            label,
                            ms_font.clone(),
                            col,
                        );
                    }
                    last_high_label_x = x;
                }
                prev_swing_high = Some(bar.high);
            }
            if abs_idx < chart.fractal_down.len() && chart.fractal_down[abs_idx] {
                let y = price_to_y(bar.low) + 2.0;
                if y <= chart_rect.bottom() && x - last_low_label_x >= min_structure_label_gap {
                    painter.text(
                        egui::pos2(x, y),
                        egui::Align2::CENTER_TOP,
                        "▼",
                        fractal_font.clone(),
                        DOWN,
                    );
                    if let Some(prev_l) = prev_swing_low {
                        let (label, col) = if bar.low > prev_l {
                            ("HL", UP)
                        } else {
                            ("LL", DOWN)
                        };
                        painter.text(
                            egui::pos2(x, y + 10.0),
                            egui::Align2::CENTER_TOP,
                            label,
                            ms_font.clone(),
                            col,
                        );
                    }
                    last_low_label_x = x;
                }
                prev_swing_low = Some(bar.low);
            }
        }
    }

    if flags.harmonics {
        draw_harmonics(
            painter,
            chart,
            chart_rect,
            data_left,
            bar_w,
            price_to_y,
            start_idx,
            end_idx,
            format_price,
        );
    }

    if flags.supply_demand {
        draw_supply_demand_zones(
            painter, chart, chart_rect, data_left, bar_w, price_to_y, start_idx, end_idx,
        );
    }

    // ── Fair Value Gaps (3-bar imbalance zones) ────────────────────────────
    if flags.fvg && bars.len() >= 3 {
        draw_fair_value_gaps(painter, chart_rect, data_left, bar_w, price_to_y, bars);
    }

    // ── Order Blocks (ICT/Smart Money) ──────────────────────────────────────
    // Bullish OB: last bearish candle before a strong bullish move (next close > current high + 1 ATR)
    // Bearish OB: last bullish candle before a strong bearish move (next close < current low - 1 ATR)
    if flags.order_blocks && bars.len() >= 3 {
        draw_order_blocks(painter, chart_rect, data_left, bar_w, price_to_y, bars);
    }

    if flags.auto_fib && !chart.auto_fib_levels.is_empty() {
        draw_auto_fib_levels(
            painter,
            chart,
            chart_rect,
            data_left,
            bar_w,
            price_to_y,
            start_idx,
            end_idx,
            format_price,
        );
    }

    // ── price data / bars (line, OHLC, candle/HA/Renko) ───────────────────
    draw_price_bars(
        painter,
        chart,
        chart_rect,
        data_left,
        bar_w,
        candle_w,
        half_body,
        price_to_y,
        bars,
        flags,
        start_idx,
        render_step,
    );

    // Previous Candle Levels should sit above candle bodies, matching the ATR
    // Projection foreground layering. The earlier pass still owns label
    // de-confliction; this foreground pass guarantees the horizontal levels are
    // not buried by candle fills/wicks.
    if flags.prev_levels {
        draw_previous_candle_level_lines_foreground(painter, chart, chart_rect, price_to_y);
    }

    // ── Extended Hours Candle (magenta, TradingView-style) ─────────────
    draw_extended_hours_candle(
        painter,
        chart,
        chart_rect,
        data_left,
        bar_w,
        candle_w,
        half_body,
        price_to_y,
        bars.len(),
    );

    draw_right_axis_price_labels(
        painter,
        chart,
        chart_rect,
        price_axis_w,
        bars,
        fresh_live_mid,
        price_to_y,
        format_price,
    );

    // ── Volume Profile overlay (volume-at-price with POC + Value Area) ─────
    let live_trade = if chart.live_trade_vol > 0.0 && chart.live_trade_price > 0.0 {
        Some((
            chart.live_trade_price,
            chart.live_trade_vol,
            chart.live_trade_is_buy,
        ))
    } else {
        None
    };
    draw_volume_profile_overlay(
        painter, chart_rect, bars, price_min, price_max, flags, live_trade,
    );

    // ── Live Depth Profile (binned from L2/L3 book levels) — full book depth
    // Bins live_depth_bids/asks (price, size) into horizontal volume-at-price bars.
    // L3 per-order data (when wired) produces richer bins + explicit "L3" label.
    // Complements historical volume profile.
    if let Some(depth_summary) = live_depth_summary(&chart.live_depth_bids, &chart.live_depth_asks)
    {
        let max_size = depth_summary.max_size;
        let max_w = (chart_rect.width() * 0.15).max(40.0);
        // Heuristic: treat as L3 if many levels (>4) or from recent L3 feed (status wired via orderbook)
        let looks_l3 = depth_summary.level_count > 4
            || (chart.live_bid_size > 0.0 && depth_summary.level_count > 2);
        let label = if looks_l3 { "L3 depth" } else { "" };
        let col = if looks_l3 {
            egui::Color32::from_rgb(80, 200, 120)
        } else {
            egui::Color32::from_rgb(160, 160, 80)
        };
        if !label.is_empty() {
            painter.text(
                egui::pos2(chart_rect.right() - 2.0, chart_rect.top() + 10.0),
                egui::Align2::RIGHT_TOP,
                label,
                egui::FontId::monospace(8.0),
                col,
            );
        }
        // Simple binning: treat each level as its own 'bucket' for now (full binning by price can expand later)
        for (price, size) in chart
            .live_depth_bids
            .iter()
            .chain(chart.live_depth_asks.iter())
        {
            if *size <= 0.0 || *price <= 0.0 {
                continue;
            }
            let y = price_to_y(*price);
            if y < chart_rect.top() || y > chart_rect.bottom() {
                continue;
            }
            let frac = (*size / max_size) as f32;
            let w = (frac * max_w as f32).max(2.0);
            let col = if chart.live_bid > 0.0 && (*price - chart.live_bid).abs() < 1e-9
                || chart.live_bid_size > 0.0 && *size == chart.live_bid_size
            {
                egui::Color32::from_rgba_premultiplied(0, 180, 60, 160) // bid green
            } else {
                egui::Color32::from_rgba_premultiplied(200, 40, 40, 160) // ask red
            };
            painter.rect_filled(
                egui::Rect::from_min_max(
                    egui::pos2(chart_rect.right() - w, y - 1.5),
                    egui::pos2(chart_rect.right(), y + 1.5),
                ),
                0.0,
                col,
            );
        }

        // Live executed trade marker on depth profile (from public trades feed, O(1))
        if chart.live_trade_vol > 0.0 && chart.live_trade_price > 0.0 {
            let y = price_to_y(chart.live_trade_price);
            if y >= chart_rect.top() && y <= chart_rect.bottom() {
                let tw = (chart.live_trade_vol / max_size.max(1.0) * max_w as f64 * 0.6)
                    .clamp(3.0, 20.0) as f32;
                let tcol = if chart.live_trade_is_buy {
                    egui::Color32::from_rgba_premultiplied(0, 220, 120, 220) // buy teal
                } else {
                    egui::Color32::from_rgba_premultiplied(255, 80, 80, 220) // sell red
                };
                painter.rect_filled(
                    egui::Rect::from_min_max(
                        egui::pos2(chart_rect.right() - tw, y - 2.0),
                        egui::pos2(chart_rect.right(), y + 2.0),
                    ),
                    0.0,
                    tcol,
                );
            }
        }
    }

    draw_post_zone_trend_overlays(
        painter, chart, chart_rect, data_left, bar_w, price_to_y, flags, start_idx, end_idx,
    );

    // ATR Projection — multi-timeframe horizontal levels (matching ATR_Projection.mqh).
    // Drawn here, after the zones and post-zone trend overlays, so the translucent
    // supply/demand & FVG fills don't tint the yellow band/labels (same layering as
    // the moving averages). One clipped line primitive per level.
    if flags.atr_proj {
        let atr_yellow = egui::Color32::from_rgb(255, 255, 0); // clrYellow
        // A timeframe whose ATR band is narrow puts its Hi and Lo labels on top
        // of each other ("ATR W1 Hi" / "ATR W1 Lo" smearing into "ATR WL HL").
        // Spread the labels into separate bands; the lines stay at true price.
        let mut label_bands: Vec<(f32, f32)> = Vec::new();
        for &(label, htf_open, atr_val, line_start_idx) in &chart.atr_proj_levels {
            let upper_price = htf_open + atr_val;
            let lower_price = htf_open - atr_val;
            let x_start_raw = if line_start_idx >= start_idx {
                data_left + ((line_start_idx - start_idx) as f32) * bar_w
            } else {
                chart_rect.left()
            };
            let x_start = clamp_f32_bounds(x_start_raw, chart_rect.left(), chart_rect.right());
            let x_end = chart_rect.right();
            for (price, suffix) in [(upper_price, "Hi"), (lower_price, "Lo")] {
                let y = price_to_y(price);
                if y >= chart_rect.top() && y <= chart_rect.bottom() {
                    painter.line_segment(
                        [egui::pos2(x_start, y), egui::pos2(x_end, y)],
                        egui::Stroke::new(3.0, atr_yellow),
                    );
                    // Label: "ATR D1 Hi 1.2345" — anchored above its line, then
                    // de-conflicted so a tight Hi/Lo pair stays legible.
                    let center = place_level_label(
                        y - 8.0,
                        5.0,
                        chart_rect.top(),
                        chart_rect.bottom(),
                        &mut label_bands,
                    );
                    painter.text(
                        egui::pos2(x_start + 4.0, center),
                        egui::Align2::LEFT_CENTER,
                        &format!("ATR {} {} {}", label, suffix, format_price(price)),
                        egui::FontId::monospace(8.0),
                        atr_yellow,
                    );
                }
            }
        }
    }

    draw_header_crosshair_and_legend(
        painter,
        chart,
        chart_rect,
        price_axis_w,
        crosshair,
        bars,
        bar_w,
        start_idx,
        price_min,
        price_max,
        flags,
        company_name,
        regulatory_alerts,
        show_rsi,
        show_cmo,
        show_qstick,
        show_disparity,
        show_bop,
        show_stddev,
        show_mfi,
        show_trix,
        show_ppo,
        show_ultosc,
        show_stochrsi,
        show_var_oscillator,
    );

    // Chart overlay removed — info shown in crosshair tooltip + right panel instead

    draw_enabled_sub_panes(
        painter,
        chart,
        rect,
        main_rect,
        price_axis_w,
        bars,
        start_idx,
        bar_w,
        show_rsi,
        show_fisher,
        show_macd,
        show_volume_pane,
        show_stochastic,
        show_adx,
        show_cci,
        show_williams_r,
        show_obv,
        show_momentum,
        show_cmo,
        show_qstick,
        show_disparity,
        show_bop,
        show_stddev,
        show_mfi,
        show_trix,
        show_ppo,
        show_ultosc,
        show_stochrsi,
        show_var_oscillator,
        show_better_volume,
        show_ehlers_ebsw,
        show_ehlers_cyber,
        show_ehlers_cg,
        show_ehlers_roof,
        show_squeeze,
    );

    draw_planning_and_compare_overlays(
        painter,
        chart,
        chart_rect,
        data_left,
        bar_w,
        bars,
        start_idx,
        end_idx,
        &price_geometry,
        sl_price,
        tp_price,
        active_position_avg_price,
        price_to_y,
        format_price,
    );

    draw_broker_trade_overlays(
        painter,
        trade_overlay,
        chart_rect,
        data_left,
        bar_w,
        start_idx,
        end_idx,
        price_to_y,
        format_price,
    );

    // ── alert price lines (extracted) ─────────────────────────────────────────
    draw_price_alert_lines(painter, chart_rect, price_to_y, alerts, format_price);

    // Follow-up polish: faint live bid/ask horizontal lines (rich L1 from WS ticker/book)
    if let Some(_mid) = chart.fresh_live_quote_mid() {
        if chart.live_bid > 0.0 && chart.live_ask > 0.0 {
            let bid_y = price_to_y(chart.live_bid);
            let ask_y = price_to_y(chart.live_ask);
            if bid_y >= chart_rect.top() && bid_y <= chart_rect.bottom() {
                painter.line_segment(
                    [
                        egui::pos2(chart_rect.left(), bid_y),
                        egui::pos2(chart_rect.right(), bid_y),
                    ],
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 160, 60)), // faint green for bid
                );
            }
            if ask_y >= chart_rect.top() && ask_y <= chart_rect.bottom() {
                painter.line_segment(
                    [
                        egui::pos2(chart_rect.left(), ask_y),
                        egui::pos2(chart_rect.right(), ask_y),
                    ],
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(180, 40, 40)), // faint red for ask
                );
            }
        }
    }

    // ── drawing annotations (extracted) ─────────────────────────────────────
    if draw_drawing_annotations(
        painter,
        chart,
        chart_rect,
        data_left,
        bar_w,
        price_to_y,
        start_idx,
        end_idx,
        bars,
        format_price,
    ) {
        return Some(price_geometry);
    }

    draw_drawing_preview(
        painter,
        chart,
        chart_rect,
        data_left,
        bar_w,
        start_idx,
        end_idx,
        bars,
        crosshair,
        draw_mode,
        &price_geometry,
        price_to_y,
        format_price,
    );
    Some(price_geometry)
}

fn draw_previous_candle_level_lines_foreground(
    painter: &egui::Painter,
    chart: &ChartState,
    chart_rect: egui::Rect,
    price_to_y: impl Fn(f64) -> f32,
) {
    let white = egui::Color32::WHITE;
    let magenta = egui::Color32::from_rgb(255, 0, 255);
    let level_pairs = [
        (chart.prev_h1_high, white, 0u8),
        (chart.prev_h1_low, white, 0),
        (chart.prev_h4_high, white, 0),
        (chart.prev_h4_low, white, 0),
        (chart.current_daily_high, white, 2),
        (chart.current_daily_low, white, 2),
        (chart.current_weekly_high, white, 2),
        (chart.current_weekly_low, white, 2),
        (chart.current_monthly_high, white, 3),
        (chart.current_monthly_low, white, 3),
        (chart.prev_daily_high, magenta, 1),
        (chart.prev_daily_low, magenta, 1),
        (chart.prev_weekly_high, magenta, 2),
        (chart.prev_weekly_low, magenta, 2),
        (chart.prev_monthly_high, magenta, 3),
        (chart.prev_monthly_low, magenta, 3),
    ];
    let chart_rank = chart.timeframe.group_rank();
    for (price_opt, color, max_rank) in level_pairs {
        if chart_rank > max_rank {
            continue;
        }
        let Some(price) = price_opt else {
            continue;
        };
        let y = price_to_y(price);
        if y >= chart_rect.top() && y <= chart_rect.bottom() {
            painter.line_segment(
                [
                    egui::pos2(chart_rect.left(), y),
                    egui::pos2(chart_rect.right(), y),
                ],
                egui::Stroke::new(3.0, color),
            );
        }
    }
}

#[cfg(test)]
mod tests;

struct LiveDepthSummary {
    level_count: usize,
    max_size: f64,
}

fn live_depth_summary(bids: &[(f64, f64)], asks: &[(f64, f64)]) -> Option<LiveDepthSummary> {
    let level_count = bids.len() + asks.len();
    if level_count == 0 {
        return None;
    }
    let max_size = bids
        .iter()
        .chain(asks)
        .map(|(_, size)| *size)
        .fold(0.0_f64, f64::max)
        .max(1.0);
    Some(LiveDepthSummary {
        level_count,
        max_size,
    })
}

#[cfg(test)]
mod company_name_overlay_tests {
    use super::chart_overlay_company_name;
    use std::collections::HashMap;

    fn names(entries: &[(&str, &str)]) -> HashMap<String, String> {
        entries
            .iter()
            .map(|(symbol, name)| ((*symbol).to_string(), (*name).to_string()))
            .collect()
    }

    #[test]
    fn resolves_company_name_case_insensitively() {
        let names = names(&[("MS", "Morgan Stanley"), ("FI", "Fiserv, Inc.")]);
        assert_eq!(
            chart_overlay_company_name(&names, &HashMap::new(), "MS").as_deref(),
            Some("Morgan Stanley")
        );
        assert_eq!(
            chart_overlay_company_name(&names, &HashMap::new(), "ms").as_deref(),
            Some("Morgan Stanley")
        );
        assert_eq!(
            chart_overlay_company_name(&names, &HashMap::new(), "FI").as_deref(),
            Some("Fiserv, Inc.")
        );
    }

    #[test]
    fn returns_none_for_unknown_or_blank() {
        let names = names(&[("MS", "Morgan Stanley"), ("ZZZ", "   ")]);
        assert_eq!(
            chart_overlay_company_name(&names, &HashMap::new(), "NVDA"),
            None
        );
        // Whitespace-only name is treated as missing.
        assert_eq!(
            chart_overlay_company_name(&names, &HashMap::new(), "ZZZ"),
            None
        );
        assert_eq!(
            chart_overlay_company_name(&HashMap::new(), &HashMap::new(), "MS"),
            None
        );
    }

    #[test]
    fn normalizes_slash_and_eq_suffix() {
        let names = names(&[("BTCUSD", "Bitcoin"), ("AAPL", "Apple Inc")]);
        // Crypto pair "BTC/USD" collapses to "BTCUSD".
        assert_eq!(
            chart_overlay_company_name(&names, &HashMap::new(), "BTC/USD").as_deref(),
            Some("Bitcoin")
        );
        // Kraken-equity ".EQ" suffix is trimmed before matching.
        assert_eq!(
            chart_overlay_company_name(&names, &HashMap::new(), "AAPL.EQ").as_deref(),
            Some("Apple Inc")
        );
    }

    #[test]
    fn resolves_from_equity_names_catalog_when_no_fundamentals() {
        let mut names = HashMap::new();
        names.insert(
            "WOK".to_string(),
            "WORK Medical Technology Group".to_string(),
        );
        assert_eq!(
            chart_overlay_company_name(&names, &HashMap::new(), "WOK.EQ").as_deref(),
            Some("WORK Medical Technology Group")
        );
    }

    #[test]
    fn fundamentals_override_provider_placeholders() {
        let provider = names(&[("CC", "Provider Placeholder")]);
        let fundamentals = names(&[("CC", "The Chemours Company")]);
        assert_eq!(
            chart_overlay_company_name(&provider, &fundamentals, "CC"),
            Some("The Chemours Company")
        );
    }

    #[test]
    fn trims_surrounding_whitespace_in_name() {
        let names = names(&[("MS", "  Morgan Stanley  ")]);
        assert_eq!(
            chart_overlay_company_name(&names, &HashMap::new(), "MS").as_deref(),
            Some("Morgan Stanley")
        );
    }
}

// ─── command palette ─────────────────────────────────────────────────────────
