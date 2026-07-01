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
    trade_overlay: &TradeOverlay,
    alerts: &[(f64, String)],
    regulatory_alerts: &[typhoon_engine::core::regulatory_alerts::RegulatoryAlert],
    draw_mode: &DrawMode,
    company_name: Option<&str>,
) {
    // Do not early-return for a stable chart. egui is immediate-mode: if this
    // function skips painting for a frame, the chart area can be left blank or
    // appear to flicker when the closed-market/auto-source chart is merely being
    // hovered, price-scaled, or panned. The old `last_rendered_*` fast path was
    // only safe with a retained render target, which we do not have here.

    // Heavy sync early-out: do near-O(1) work only during backfill
    if chart.heavy_sync_in_progress {
        painter.rect_filled(rect, 0.0, BG);
        return;
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
        return;
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
    let price_axis_w = 98.0_f32;
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
        return;
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

    // ── bar width ────────────────────────────────────────────────────────────
    // Horizontal camera: visible_slot_count is the full virtual viewport, while
    // `bars` is only the intersecting real-data slice. first_bar_slot offsets
    // the real bars inside the viewport so panning beyond either edge produces
    // real empty chart space instead of stretching/clamping candles.
    let n_bars = visible_slot_count.max(1) as f32;
    let bar_w = (chart_rect.width() / n_bars).max(1.0);
    let data_left = chart_rect.left() + first_bar_slot * bar_w;
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
                    egui::Stroke::new(0.5, *first_col),
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
    draw_volume_profile_overlay(painter, chart_rect, bars, price_min, price_max, flags);

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
                        egui::Stroke::new(1.5, atr_yellow),
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
        price_min,
        price_max,
        sl_price,
        tp_price,
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
                    [egui::pos2(chart_rect.left(), bid_y), egui::pos2(chart_rect.right(), bid_y)],
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 160, 60)), // faint green for bid
                );
            }
            if ask_y >= chart_rect.top() && ask_y <= chart_rect.bottom() {
                painter.line_segment(
                    [egui::pos2(chart_rect.left(), ask_y), egui::pos2(chart_rect.right(), ask_y)],
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
        return;
    }

    draw_drawing_preview(
        painter, chart_rect, data_left, bar_w, start_idx, end_idx, price_min, price_max, crosshair,
        draw_mode, price_to_y,
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn indicator_value_lookup_returns_none_when_series_lags_bars() {
        let series = vec![Some(1.0), None, Some(3.0)];

        assert_eq!(super::indicator_value_at(&series, 0), Some(1.0));
        assert_eq!(super::indicator_value_at(&series, 1), None);
        assert_eq!(super::indicator_value_at(&series, 3), None);
    }

    #[test]
    fn indicator_line_clipping_keeps_price_scale_crossing_segments() {
        let rect = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(100.0, 100.0));
        let clipped = super::clip_line_segment_to_rect(
            egui::pos2(10.0, -50.0),
            egui::pos2(90.0, 150.0),
            rect,
        )
        .expect("segment crosses chart pane even when both sampled y values are offscreen");

        assert!((clipped[0].x - 30.0).abs() < 0.001);
        assert!((clipped[0].y - 0.0).abs() < 0.001);
        assert!((clipped[1].x - 70.0).abs() < 0.001);
        assert!((clipped[1].y - 100.0).abs() < 0.001);
    }

    #[test]
    fn indicator_line_clipping_rejects_fully_offscreen_segments() {
        let rect = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(100.0, 100.0));
        assert!(
            super::clip_line_segment_to_rect(
                egui::pos2(10.0, -50.0),
                egui::pos2(90.0, -10.0),
                rect,
            )
            .is_none()
        );
    }

    #[test]
    fn clamp_f32_bounds_accepts_inverted_tiny_pane_bounds() {
        assert_eq!(super::clamp_f32_bounds(9.0, 8.0, 7.46875), 8.0);
        assert_eq!(super::clamp_f32_bounds(7.0, 8.0, 7.46875), 7.46875);
    }

    #[test]
    fn projection_candle_sits_in_next_slot_not_far_right_empty_space() {
        let rect = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1000.0, 400.0));
        let x = super::adjacent_projection_candle_x(0.0, 1, 10.0, 3.5, rect)
            .expect("one empty slot after the visible bar is enough for projection candle");

        assert!((x - 15.0).abs() < 0.001);
        assert!(
            x < rect.right() - 900.0,
            "projection candle must not be pinned to far-right chart edge; x={x}"
        );
    }

    #[test]
    fn projection_candle_is_hidden_when_next_slot_is_offscreen() {
        let rect = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(100.0, 400.0));

        assert!(super::adjacent_projection_candle_x(0.0, 10, 10.0, 3.5, rect).is_none());
    }

    #[test]
    fn candle_countdown_uses_current_bar_boundary() {
        let last_bar = 1_700_000_000_000_i64;
        let now = last_bar + 3 * 60_000 + 15_000;

        assert_eq!(
            super::next_candle_remaining_ms_at(last_bar, super::Timeframe::M5, now),
            Some(105_000)
        );
    }

    #[test]
    fn candle_countdown_hides_for_stale_or_closed_session_bar_data() {
        let last_bar = 1_700_000_000_000_i64;
        let now = last_bar + 17 * 60_000 + 10_000;

        assert_eq!(
            super::next_candle_remaining_ms_at(last_bar, super::Timeframe::M5, now),
            None
        );
    }

    #[test]
    fn candle_countdown_hides_for_closed_equity_weekend_but_not_crypto() {
        let saturday_et = chrono::DateTime::parse_from_rfc3339("2026-06-13T12:00:00Z")
            .unwrap()
            .timestamp_millis();

        assert!(!super::chart_candle_countdown_allowed_at(
            "kraken-equities",
            "WOK",
            saturday_et
        ));
        assert!(!super::chart_candle_countdown_allowed_at(
            "kraken",
            "WOK",
            saturday_et
        ));
        assert!(super::chart_candle_countdown_allowed_at(
            "kraken",
            "BTC/USD",
            saturday_et
        ));
    }

    #[test]
    fn candle_countdown_respects_friday_and_sunday_equity_weekend_boundary() {
        let friday_before_xstock_close =
            chrono::DateTime::parse_from_rfc3339("2026-06-12T23:59:00Z")
                .unwrap()
                .timestamp_millis();
        let friday_after_xstock_close =
            chrono::DateTime::parse_from_rfc3339("2026-06-13T00:01:00Z")
                .unwrap()
                .timestamp_millis();
        let sunday_before_xstock_open =
            chrono::DateTime::parse_from_rfc3339("2026-06-14T23:59:00Z")
                .unwrap()
                .timestamp_millis();
        let sunday_after_xstock_open = chrono::DateTime::parse_from_rfc3339("2026-06-15T00:01:00Z")
            .unwrap()
            .timestamp_millis();

        assert!(super::chart_candle_countdown_allowed_at(
            "kraken-equities",
            "WOK",
            friday_before_xstock_close
        ));
        assert!(!super::chart_candle_countdown_allowed_at(
            "kraken-equities",
            "WOK",
            friday_after_xstock_close
        ));
        assert!(!super::chart_candle_countdown_allowed_at(
            "kraken-equities",
            "WOK",
            sunday_before_xstock_open
        ));
        assert!(super::chart_candle_countdown_allowed_at(
            "kraken-equities",
            "WOK",
            sunday_after_xstock_open
        ));
    }

    #[test]
    fn candle_countdown_formats_like_chart_axis_timer() {
        assert_eq!(super::format_candle_countdown(4_000), "00:04");
        assert_eq!(super::format_candle_countdown(65_000), "01:05");
        assert_eq!(super::format_candle_countdown(3_661_000), "1:01:01");
        assert_eq!(super::format_candle_countdown(90_000_000), "1d 01:00");
    }

    #[test]
    fn time_axis_labels_include_dates_and_years_across_timeframes() {
        let ts = chrono::DateTime::parse_from_rfc3339("2026-06-11T14:35:00Z")
            .unwrap()
            .timestamp_millis();

        assert_eq!(
            super::format_ts(ts, super::Timeframe::M5),
            "11 Jun'26 14:35"
        );
        assert_eq!(
            super::format_ts(ts, super::Timeframe::H1),
            "11 Jun'26 14:35"
        );
        assert_eq!(super::format_ts(ts, super::Timeframe::D1), "11 Jun'26");
        assert_eq!(super::format_ts(ts, super::Timeframe::W1), "11 Jun'26");
        assert_eq!(super::format_ts(ts, super::Timeframe::MN1), "Jun 2026");
    }

    #[test]
    fn intraday_axis_stride_climbs_the_ladder_as_bars_shrink() {
        // Wide bars (zoomed in) → fine stride; thin bars (zoomed out) → coarse.
        // H1 (60m bars): 22px bars want ~3h between labels; 1.3px bars want days.
        assert_eq!(super::intraday_axis_stride_minutes(60, 22.0, 64.0), 180);
        assert_eq!(super::intraday_axis_stride_minutes(60, 1.3, 64.0), 4320);
        // Stride never drops below the timeframe itself (H4 = 240m floor).
        assert_eq!(super::intraday_axis_stride_minutes(240, 100.0, 64.0), 240);
        // H4 over a month (≈4.5px bars) → multi-day date ticks, no intraday smear.
        assert_eq!(super::intraday_axis_stride_minutes(240, 4.5, 64.0), 4320);
    }

    #[test]
    fn extended_hours_axis_labels_are_explicit() {
        assert_eq!(super::format_axis_price_label("EXT", 0.0924), "EXT 0.0924");
        assert_eq!(super::format_axis_price_label("C", 194.32), "C 194.3200");
    }

    #[test]
    fn close_reference_color_uses_previous_daily_close() {
        let day = 86_400_000_i64;
        let bars = vec![
            super::Bar {
                ts_ms: day,
                open: 0.11,
                high: 0.12,
                low: 0.10,
                close: 0.10065,
                volume: 1.0,
            },
            super::Bar {
                ts_ms: day * 2,
                open: 0.09,
                high: 0.11,
                low: 0.08,
                close: 0.09265,
                volume: 1.0,
            },
        ];

        assert_eq!(super::previous_daily_close_from_bars(&bars), Some(0.10065));
        assert_eq!(
            super::close_reference_color(0.09265, 0.09, &bars),
            super::DOWN
        );
        assert_eq!(super::close_reference_color(0.102, 0.09, &bars), super::UP);
    }

    #[test]
    fn close_reference_color_falls_back_to_bar_open_without_prior_day() {
        let bars = vec![super::Bar {
            ts_ms: 86_400_000,
            open: 10.0,
            high: 11.0,
            low: 9.0,
            close: 10.5,
            volume: 1.0,
        }];

        assert_eq!(super::previous_daily_close_from_bars(&bars), None);
        assert_eq!(super::close_reference_color(9.5, 10.0, &bars), super::DOWN);
        assert_eq!(super::close_reference_color(10.5, 10.0, &bars), super::UP);
    }

    #[test]
    fn extended_hours_symbol_badge_lists_close_ext_and_move() {
        assert_eq!(
            super::format_ext_hours_symbol_badge(100.0, 101.25, Some(98.0)),
            "Daily Close 100.0000 (+3.32%)  EXT last 101.2500  Δ/C +1.2500 (+1.25%)"
        );
        assert_eq!(
            super::format_ext_hours_symbol_badge(100.0, 99.5, Some(100.0)),
            "Daily Close 100.0000 (-0.50%)  EXT last 99.5000  Δ/C -0.5000 (-0.50%)"
        );
        assert_eq!(
            super::format_ext_hours_symbol_badge(0.0925, 0.0924, Some(0.0900)),
            "Daily Close 0.0925 (+2.67%)  EXT last 0.0924  Δ/C -0.0001 (-0.11%)"
        );
        assert_eq!(
            super::format_ext_hours_symbol_badge(0.0925, 0.0924, None),
            "Daily Close 0.0925  EXT last 0.0924  Δ/C -0.0001 (-0.11%)"
        );
    }

    #[test]
    fn prev_candle_levels_only_show_higher_timeframes() {
        use crate::types::Timeframe;
        // (label, max chart group_rank at which the level still draws) — must
        // mirror the draw site: a level shows iff chart.group_rank() <= max_rank.
        let levels = [
            ("Prev H1", 0u8),
            ("Prev H4", 0),
            ("Prev D", 1),
            ("Prev W", 2),
            ("Prev MN", 3),
            ("Cur D", 2),
            ("Cur W", 2),
            ("Cur MN", 3),
        ];
        let visible = |tf: Timeframe| -> Vec<&'static str> {
            let rank = tf.group_rank();
            levels
                .iter()
                .filter(|(_, m)| rank <= *m)
                .map(|(l, _)| *l)
                .collect()
        };
        // Sub-hour chart shows every previous + every current level.
        assert_eq!(
            visible(Timeframe::M15),
            vec![
                "Prev H1", "Prev H4", "Prev D", "Prev W", "Prev MN", "Cur D", "Cur W", "Cur MN"
            ]
        );
        // Hourly charts drop their own H1/H4 previous; keep daily+ and all current.
        assert_eq!(
            visible(Timeframe::H1),
            vec!["Prev D", "Prev W", "Prev MN", "Cur D", "Cur W", "Cur MN"]
        );
        assert_eq!(
            visible(Timeframe::H4),
            vec!["Prev D", "Prev W", "Prev MN", "Cur D", "Cur W", "Cur MN"]
        );
        assert_eq!(
            visible(Timeframe::D1),
            vec!["Prev W", "Prev MN", "Cur D", "Cur W", "Cur MN"]
        );
        // Weekly chart keeps only MN previous + MN current; monthly+ show nothing.
        assert_eq!(visible(Timeframe::W1), vec!["Prev MN", "Cur MN"]);
        assert!(visible(Timeframe::MN1).is_empty());
    }

    #[test]
    fn nnfx_view_uses_mql_mtf_names_when_projected_overlays_exist() {
        assert_eq!(
            super::nnfx_trend_legend_labels(true, true),
            ("MTF_MA", "MultiKAMA")
        );
        assert_eq!(
            super::nnfx_trend_legend_labels(false, false),
            ("SMA200", "KAMA(10,2,30)")
        );
    }

    #[test]
    fn nnfx_view_suppresses_generic_current_tf_lines_when_mql_mtf_overlays_exist() {
        assert!(!super::draw_current_sma200_overlay(true, true));
        assert!(super::draw_current_sma200_overlay(true, false));
        assert!(!super::draw_current_kama_overlay(true, true));
        assert!(super::draw_current_kama_overlay(true, false));
    }
}

#[cfg(test)]
mod company_name_overlay_tests {
    use super::chart_overlay_company_name;
    use std::collections::HashMap;
    use typhoon_engine::core::fundamentals::Fundamentals;

    fn fund(symbol: &str, name: &str) -> Fundamentals {
        Fundamentals {
            symbol: symbol.to_string(),
            company_name: name.to_string(),
            ..Default::default()
        }
    }

    fn no_names() -> HashMap<String, String> {
        HashMap::new()
    }

    #[test]
    fn resolves_company_name_case_insensitively() {
        let funds = vec![fund("MS", "Morgan Stanley"), fund("FI", "Fiserv, Inc.")];
        let names = no_names();
        assert_eq!(
            chart_overlay_company_name(&funds, &names, "MS").as_deref(),
            Some("Morgan Stanley")
        );
        assert_eq!(
            chart_overlay_company_name(&funds, &names, "ms").as_deref(),
            Some("Morgan Stanley")
        );
        assert_eq!(
            chart_overlay_company_name(&funds, &names, "FI").as_deref(),
            Some("Fiserv, Inc.")
        );
    }

    #[test]
    fn returns_none_for_unknown_or_blank() {
        let funds = vec![fund("MS", "Morgan Stanley"), fund("ZZZ", "   ")];
        let names = no_names();
        assert_eq!(chart_overlay_company_name(&funds, &names, "NVDA"), None);
        // Whitespace-only name is treated as missing.
        assert_eq!(chart_overlay_company_name(&funds, &names, "ZZZ"), None);
        // Empty fundamentals table.
        assert_eq!(chart_overlay_company_name(&[], &names, "MS"), None);
    }

    #[test]
    fn normalizes_slash_and_eq_suffix() {
        let funds = vec![fund("BTCUSD", "Bitcoin"), fund("AAPL", "Apple Inc")];
        let names = no_names();
        // Crypto pair "BTC/USD" collapses to "BTCUSD".
        assert_eq!(
            chart_overlay_company_name(&funds, &names, "BTC/USD").as_deref(),
            Some("Bitcoin")
        );
        // Kraken-equity ".EQ" suffix is trimmed before matching.
        assert_eq!(
            chart_overlay_company_name(&funds, &names, "AAPL.EQ").as_deref(),
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
        // No fundamentals row — the lightweight Kraken equity catalog resolves it.
        assert_eq!(
            chart_overlay_company_name(&[], &names, "WOK.EQ").as_deref(),
            Some("WORK Medical Technology Group")
        );
    }

    #[test]
    fn trims_surrounding_whitespace_in_name() {
        let funds = vec![fund("MS", "  Morgan Stanley  ")];
        let names = no_names();
        assert_eq!(
            chart_overlay_company_name(&funds, &names, "MS").as_deref(),
            Some("Morgan Stanley")
        );
    }
}

// ─── command palette ─────────────────────────────────────────────────────────
