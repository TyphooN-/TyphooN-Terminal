use super::technical_indicators::*;
use super::*;

mod chart_helpers;
mod time_axis;

use chart_helpers::*;
pub use chart_helpers::chart_overlay_company_name;
pub use chart_helpers::{parse_range, parse_range_f32};
use time_axis::*;
pub(in crate::app) use time_axis::{format_price, format_price_buf, format_ts, format_ts_buf};

// ─── chart rendering ─────────────────────────────────────────────────────────

/// Draw a single chart viewport into `rect` using `painter`.
pub(super) fn draw_chart(
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

    // ATR Projection — multi-timeframe horizontal levels (matching ATR_Projection.mqh).
    // Draw one clipped line primitive per level; dotted per-pixel segments were pure
    // tessellation/GPU-upload pressure and did not add price accuracy.
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
            if chart_rank > *max_rank { continue; }
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
                    [egui::pos2(chart_rect.left(), y), egui::pos2(chart_rect.right(), y)],
                    egui::Stroke::new(0.5, *first_col),
                );
            }

            // Place the label (de-conflict with other price bands)
            let center = place_level_label(
                y - 8.0, 5.0, chart_rect.top(), chart_rect.bottom(), &mut label_bands,
            );

            // Render combined label with per-segment colour when >1 entry at same price
            let base_x = chart_rect.right() - 4.0;
            let mut x = base_x;
            let font = egui::FontId::monospace(8.0);

            for (idx, (lab, col)) in group.iter().enumerate() {
                if idx > 0 {
                    // comma separator (neutral colour)
                    let comma = painter.layout_no_wrap(", ".to_string(), font.clone(), egui::Color32::LIGHT_GRAY);
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
                painter.galley(
                    egui::pos2(x, center - g.rect.height() * 0.5),
                    g,
                    *col,
                );
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

    // ── harmonic patterns (Scott Carney XABCD) ────────────────────────────
    if flags.harmonics {
        let pattern_col = egui::Color32::from_rgb(0, 200, 255);
        let tp_col = egui::Color32::from_rgb(0, 200, 80);
        let sl_col = egui::Color32::from_rgb(220, 40, 40);
        for pat in &chart.harmonics {
            let pts = [pat.x, pat.a, pat.b, pat.c, pat.d];
            let screen_pts = pts.map(|(idx, price)| {
                if idx >= start_idx && idx < end_idx {
                    Some(egui::pos2(
                        data_left + ((idx - start_idx) as f32 + 0.5) * bar_w,
                        price_to_y(price),
                    ))
                } else {
                    None
                }
            });
            // XABCD lines
            for w in screen_pts.windows(2) {
                if let (Some(p1), Some(p2)) = (w[0], w[1]) {
                    painter.line_segment([p1, p2], egui::Stroke::new(1.5, pattern_col));
                }
            }
            // Labels
            let labels = ["X", "A", "B", "C", "D"];
            for (i, sp) in screen_pts.iter().enumerate() {
                if let Some(p) = sp {
                    painter.text(
                        egui::pos2(p.x, p.y + if i % 2 == 0 { -12.0 } else { 4.0 }),
                        egui::Align2::CENTER_TOP,
                        labels[i],
                        egui::FontId::monospace(10.0),
                        pattern_col,
                    );
                }
            }
            // Pattern name
            if let Some(d_pt) = screen_pts[4] {
                let dir = if pat.bullish { "BULL" } else { "BEAR" };
                let col = if pat.bullish { UP } else { DOWN };
                painter.text(
                    egui::pos2(d_pt.x + 5.0, d_pt.y - 20.0),
                    egui::Align2::LEFT_TOP,
                    &format!("{} {}", pat.name, dir),
                    egui::FontId::monospace(9.0),
                    col,
                );
                // TP/SL from D
                for (price, label, c) in [
                    (pat.tp1, "TP1", tp_col),
                    (pat.tp2, "TP2", tp_col),
                    (pat.sl, "SL", sl_col),
                ] {
                    let y = price_to_y(price);
                    if y >= chart_rect.top() && y <= chart_rect.bottom() {
                        painter.line_segment(
                            [egui::pos2(d_pt.x, y), egui::pos2(chart_rect.right(), y)],
                            egui::Stroke::new(0.7, c),
                        );
                        painter.text(
                            egui::pos2(d_pt.x + 2.0, y - 9.0),
                            egui::Align2::LEFT_TOP,
                            &format!("{} {}", label, format_price(price)),
                            egui::FontId::monospace(8.0),
                            c,
                        );
                    }
                }
            }
        }
    }

    // ── supply/demand zones ─────────────────────────────────────────────────
    if flags.supply_demand {
        let status_label = |s: u8| -> &str {
            match s {
                0 => "Untested",
                1 => "Tested",
                2 => "Proven",
                _ => "",
            }
        };
        // Zones extend from their creation bar to the chart right edge (matching MT5).
        // Show any zone whose creation bar is <= end_idx (it extends into or past the view).
        // Demand zones — MT5 colors: DarkSeaGreen/MediumSeaGreen/SeaGreen
        for &(idx, zh, zl, status) in &chart.demand_zones {
            if idx < end_idx {
                let x_start = if idx >= start_idx {
                    data_left + ((idx - start_idx) as f32) * bar_w
                } else {
                    chart_rect.left()
                };
                let y_top = price_to_y(zh);
                let y_bot = price_to_y(zl);
                if y_bot >= chart_rect.top() && y_top <= chart_rect.bottom() {
                    let (fill_col, label_col) = match status {
                        0 => (
                            egui::Color32::from_rgba_premultiplied(143, 188, 143, 50),
                            egui::Color32::from_rgb(200, 255, 200), // high contrast
                        ),
                        1 => (
                            egui::Color32::from_rgba_premultiplied(60, 179, 113, 60),
                            egui::Color32::from_rgb(220, 255, 220),
                        ),
                        _ => (
                            egui::Color32::from_rgba_premultiplied(46, 139, 87, 70),
                            egui::Color32::WHITE,
                        ),
                    };
                    painter.rect_filled(
                        egui::Rect::from_min_max(
                            egui::pos2(x_start, y_top.max(chart_rect.top())),
                            egui::pos2(chart_rect.right(), y_bot.min(chart_rect.bottom())),
                        ),
                        0.0,
                        fill_col,
                    );
                    painter.text(
                        egui::pos2(
                            chart_rect.right() - 4.0,
                            y_bot.min(chart_rect.bottom()) - 12.0,
                        ),
                        egui::Align2::RIGHT_TOP,
                        &format!("Demand [{}]", status_label(status)),
                        egui::FontId::monospace(9.0),
                        label_col,
                    );
                }
            }
        }
        // Supply zones — MT5 colors: SkyBlue/DeepSkyBlue/DodgerBlue
        for &(idx, zh, zl, status) in &chart.supply_zones {
            if idx < end_idx {
                let x_start = if idx >= start_idx {
                    data_left + ((idx - start_idx) as f32) * bar_w
                } else {
                    chart_rect.left()
                };
                let y_top = price_to_y(zh);
                let y_bot = price_to_y(zl);
                if y_bot >= chart_rect.top() && y_top <= chart_rect.bottom() {
                    let (fill_col, label_col) = match status {
                        0 => (
                            egui::Color32::from_rgba_premultiplied(135, 206, 235, 50),
                            egui::Color32::from_rgb(200, 230, 255), // high contrast on blue zones
                        ),
                        1 => (
                            egui::Color32::from_rgba_premultiplied(0, 191, 255, 60),
                            egui::Color32::from_rgb(220, 245, 255),
                        ),
                        _ => (
                            egui::Color32::from_rgba_premultiplied(30, 144, 255, 70),
                            egui::Color32::WHITE,
                        ),
                    };
                    painter.rect_filled(
                        egui::Rect::from_min_max(
                            egui::pos2(x_start, y_top.max(chart_rect.top())),
                            egui::pos2(chart_rect.right(), y_bot.min(chart_rect.bottom())),
                        ),
                        0.0,
                        fill_col,
                    );
                    painter.text(
                        egui::pos2(chart_rect.right() - 4.0, y_top.max(chart_rect.top()) + 2.0),
                        egui::Align2::RIGHT_TOP,
                        &format!("Supply [{}]", status_label(status)),
                        egui::FontId::monospace(9.0),
                        label_col,
                    );
                }
            }
        }
    }

    // ── Fair Value Gaps (3-bar imbalance zones) ────────────────────────────
    if flags.fvg && bars.len() >= 3 {
        let fvg_bull = egui::Color32::from_rgba_premultiplied(0, 180, 80, 30);
        let fvg_bear = egui::Color32::from_rgba_premultiplied(220, 50, 50, 30);
        let fvg_bull_edge = egui::Color32::from_rgba_premultiplied(0, 180, 80, 80);
        let fvg_bear_edge = egui::Color32::from_rgba_premultiplied(220, 50, 50, 80);
        // Suffix arrays make the "has this gap been filled?" lookup O(1).
        // future_min_low[k] = min(bars[k..].low); future_max_high[k] = max(bars[k..].high).
        // The previous code scanned bars[i+2..] for each FVG candidate (O(n²) per frame
        // — pricey on dense charts and unnecessary when only the suffix extremes matter).
        let n = bars.len();
        let mut future_min_low: Vec<f64> = vec![f64::INFINITY; n + 1];
        let mut future_max_high: Vec<f64> = vec![f64::NEG_INFINITY; n + 1];
        for k in (0..n).rev() {
            future_min_low[k] = future_min_low[k + 1].min(bars[k].low);
            future_max_high[k] = future_max_high[k + 1].max(bars[k].high);
        }
        for i in 1..n.saturating_sub(1) {
            let prev = &bars[i - 1];
            let next = &bars[i + 1];
            let x_start = data_left + ((i + 1) as f32 + 0.5) * bar_w;
            let x_end = chart_rect.right();
            let scan_start = (i + 2).min(n);
            // Bullish FVG: bar[i+1].low > bar[i-1].high (gap up)
            if next.low > prev.high {
                let gap_top = price_to_y(next.low);
                let gap_bot = price_to_y(prev.high);
                if gap_top <= chart_rect.bottom() && gap_bot >= chart_rect.top() {
                    let filled = future_min_low[scan_start] <= prev.high;
                    if !filled {
                        painter.rect_filled(
                            egui::Rect::from_min_max(
                                egui::pos2(x_start, gap_top.max(chart_rect.top())),
                                egui::pos2(x_end, gap_bot.min(chart_rect.bottom())),
                            ),
                            0.0,
                            fvg_bull,
                        );
                        painter.line_segment(
                            [egui::pos2(x_start, gap_top), egui::pos2(x_end, gap_top)],
                            egui::Stroke::new(0.5, fvg_bull_edge),
                        );
                        painter.line_segment(
                            [egui::pos2(x_start, gap_bot), egui::pos2(x_end, gap_bot)],
                            egui::Stroke::new(0.5, fvg_bull_edge),
                        );
                    }
                }
            }
            // Bearish FVG: bar[i+1].high < bar[i-1].low (gap down)
            if next.high < prev.low {
                let gap_top = price_to_y(prev.low);
                let gap_bot = price_to_y(next.high);
                if gap_top <= chart_rect.bottom() && gap_bot >= chart_rect.top() {
                    let filled = future_max_high[scan_start] >= prev.low;
                    if !filled {
                        painter.rect_filled(
                            egui::Rect::from_min_max(
                                egui::pos2(x_start, gap_top.max(chart_rect.top())),
                                egui::pos2(x_end, gap_bot.min(chart_rect.bottom())),
                            ),
                            0.0,
                            fvg_bear,
                        );
                        painter.line_segment(
                            [egui::pos2(x_start, gap_top), egui::pos2(x_end, gap_top)],
                            egui::Stroke::new(0.5, fvg_bear_edge),
                        );
                        painter.line_segment(
                            [egui::pos2(x_start, gap_bot), egui::pos2(x_end, gap_bot)],
                            egui::Stroke::new(0.5, fvg_bear_edge),
                        );
                    }
                }
            }
        }
    }

    // ── Order Blocks (ICT/Smart Money) ──────────────────────────────────────
    // Bullish OB: last bearish candle before a strong bullish move (next close > current high + 1 ATR)
    // Bearish OB: last bullish candle before a strong bearish move (next close < current low - 1 ATR)
    if flags.order_blocks && bars.len() >= 3 {
        let ob_bull_fill = egui::Color32::from_rgba_premultiplied(0, 180, 160, 25);
        let ob_bull_edge = egui::Color32::from_rgba_premultiplied(0, 180, 160, 100);
        let ob_bear_fill = egui::Color32::from_rgba_premultiplied(220, 50, 50, 25);
        let ob_bear_edge = egui::Color32::from_rgba_premultiplied(220, 50, 50, 100);
        let ob_label_col = egui::Color32::from_rgba_premultiplied(200, 200, 200, 180);

        // Compute rolling ATR(14) for impulsive move threshold. Keep the early-bar
        // behavior unchanged, but avoid recomputing the 14-bar true-range window
        // for every bar on provider-maximum histories.
        let atr_period = 14usize;
        let mut true_ranges: Vec<f64> = Vec::with_capacity(bars.len());
        let mut local_atr: Vec<f64> = Vec::with_capacity(bars.len());
        let mut rolling_sum = 0.0;
        for i in 0..bars.len() {
            let bar = &bars[i];
            let tr = if i == 0 {
                bar.high - bar.low
            } else {
                let prev_close = bars[i - 1].close;
                let hl = bar.high - bar.low;
                let hc = (bar.high - prev_close).abs();
                let lc = (bar.low - prev_close).abs();
                hl.max(hc).max(lc)
            };
            true_ranges.push(tr);
            rolling_sum += tr;
            if i >= atr_period {
                rolling_sum -= true_ranges[i - atr_period];
                local_atr.push(rolling_sum / atr_period as f64);
            } else {
                local_atr.push(bar.high - bar.low);
            }
        }

        // Collect order blocks (limit to most recent 20)
        struct OBZone {
            high: f64,
            low: f64,
            bar_idx: usize,
            is_bull: bool,
            end_idx: usize,
        }
        let mut zones: Vec<OBZone> = Vec::with_capacity(20);

        // Walk newest-to-oldest and stop once the render cap is full. The old path
        // scanned every bar, built every historical OB, then drained the front just
        // to keep the last 20. On provider-maximum histories that did wasted work
        // proportional to the full cache depth on every chart render.
        for i in (0..bars.len().saturating_sub(1)).rev() {
            let cur = &bars[i];
            let nxt = &bars[i + 1];
            let atr = local_atr[i];
            if atr <= 0.0 {
                continue;
            }

            // Bullish OB: bearish candle, then next close breaks above current high by >= 1 ATR
            if cur.close < cur.open && nxt.close > cur.high + atr {
                let mut end = bars.len();
                for j in (i + 2)..bars.len() {
                    if bars[j].low <= cur.high {
                        end = j;
                        break;
                    }
                }
                zones.push(OBZone {
                    high: cur.high,
                    low: cur.low,
                    bar_idx: i,
                    is_bull: true,
                    end_idx: end,
                });
            }

            // Bearish OB: bullish candle, then next close breaks below current low by >= 1 ATR
            if cur.close > cur.open && nxt.close < cur.low - atr {
                let mut end = bars.len();
                for j in (i + 2)..bars.len() {
                    if bars[j].high >= cur.low {
                        end = j;
                        break;
                    }
                }
                zones.push(OBZone {
                    high: cur.high,
                    low: cur.low,
                    bar_idx: i,
                    is_bull: false,
                    end_idx: end,
                });
            }

            if zones.len() >= 20 {
                break;
            }
        }
        zones.reverse();

        for ob in &zones {
            let x_start = data_left + (ob.bar_idx as f32 + 0.5) * bar_w;
            let x_end = if ob.end_idx >= bars.len() {
                chart_rect.right()
            } else {
                data_left + (ob.end_idx as f32 + 0.5) * bar_w
            };
            if x_end < chart_rect.left() || x_start > chart_rect.right() {
                continue;
            }

            let y_top = price_to_y(ob.high);
            let y_bot = price_to_y(ob.low);
            if y_top > chart_rect.bottom() || y_bot < chart_rect.top() {
                continue;
            }

            let (fill, edge) = if ob.is_bull {
                (ob_bull_fill, ob_bull_edge)
            } else {
                (ob_bear_fill, ob_bear_edge)
            };
            let ct = y_top.max(chart_rect.top());
            let cb = y_bot.min(chart_rect.bottom());
            let cl = x_start.max(chart_rect.left());
            let cr = x_end.min(chart_rect.right());

            painter.rect_filled(
                egui::Rect::from_min_max(egui::pos2(cl, ct), egui::pos2(cr, cb)),
                0.0,
                fill,
            );
            painter.line_segment(
                [egui::pos2(cl, ct), egui::pos2(cr, ct)],
                egui::Stroke::new(0.7, edge),
            );
            painter.line_segment(
                [egui::pos2(cl, cb), egui::pos2(cr, cb)],
                egui::Stroke::new(0.7, edge),
            );
            // "OB" label
            if cr - cl > 20.0 {
                painter.text(
                    egui::pos2(cl + 3.0, ct + 1.0),
                    egui::Align2::LEFT_TOP,
                    if ob.is_bull { "OB+" } else { "OB-" },
                    egui::FontId::monospace(9.0),
                    ob_label_col,
                );
            }
        }
    }

    // ── Auto Fibonacci levels (matching AutoFibonacci.mqh) ─────────────────
    if flags.auto_fib && !chart.auto_fib_levels.is_empty() {
        for (price, label, is_ext) in &chart.auto_fib_levels {
            let y = price_to_y(*price);
            if y >= chart_rect.top() && y <= chart_rect.bottom() {
                // One clipped line per level. Dotted Fib levels used to emit a
                // per-pixel segment loop, which is bad for dense adaptive-sync
                // repaint. Keep the exact level, drop the decorative primitive spam.
                let color = if *is_ext {
                    egui::Color32::from_rgb(30, 144, 255) // clrDodgerBlue
                } else {
                    egui::Color32::from_rgb(255, 215, 0) // clrGold
                };
                painter.line_segment(
                    [
                        egui::pos2(chart_rect.left(), y),
                        egui::pos2(chart_rect.right(), y),
                    ],
                    egui::Stroke::new(1.0, color),
                );
                // Label on right
                let mut fib_label = String::with_capacity(label.len() + 24);
                fib_label.push_str(label);
                fib_label.push(' ');
                fib_label.push_str(&format_price(*price));
                painter.text(
                    egui::pos2(chart_rect.right() - 4.0, y - 1.0),
                    egui::Align2::RIGHT_BOTTOM,
                    fib_label,
                    egui::FontId::monospace(8.0),
                    color,
                );
            }
        }
        // Draw swing line
        if let Some((_high, _low, hi_idx, lo_idx)) = chart.auto_fib_swing {
            if hi_idx >= start_idx && hi_idx < end_idx && lo_idx >= start_idx && lo_idx < end_idx {
                let x1 = data_left + ((hi_idx - start_idx) as f32 + 0.5) * bar_w;
                let y1 = price_to_y(_high);
                let x2 = data_left + ((lo_idx - start_idx) as f32 + 0.5) * bar_w;
                let y2 = price_to_y(_low);
                painter.line_segment(
                    [egui::pos2(x1, y1), egui::pos2(x2, y2)],
                    egui::Stroke::new(1.0, egui::Color32::WHITE),
                );
            }
        }
    }

    // ── price data (possibly Heikin-Ashi transformed) ──────────────────────
    let ha_bars;
    let renko_bars;
    let render_bars: &[Bar] = match chart.chart_type {
        ChartType::HeikinAshi => {
            ha_bars = heikin_ashi(bars);
            &ha_bars
        }
        ChartType::Renko => {
            renko_bars = renko_bricks(bars);
            &renko_bars
        }
        _ => bars,
    };

    // ── draw bars (candle/HA/line/OHLC) ──────────────────────────────────
    match chart.chart_type {
        ChartType::Line => {
            // Line chart: polyline through close prices. Downsample when the view
            // contains more bars than horizontal pixels can distinguish; drawing
            // tens of thousands of sub-pixel vertices only adds tessellation work.
            let mut points: Vec<egui::Pos2> = Vec::with_capacity(bars.len() / render_step + 1);
            for (rel_idx, bar) in bars.iter().enumerate().step_by(render_step) {
                let x = data_left + (rel_idx as f32 + 0.5) * bar_w;
                let y = price_to_y(bar.close);
                if y >= chart_rect.top() && y <= chart_rect.bottom() {
                    points.push(egui::pos2(x, y));
                }
            }
            if points.len() > 1 {
                painter.add(egui::Shape::line(points, egui::Stroke::new(1.5, ACCENT)));
            }
        }
        ChartType::OhlcBars => {
            // OHLC Bars: vertical wick + left tick (open) + right tick (close)
            for (rel_idx, bar) in bars.iter().enumerate().step_by(render_step) {
                let cx = data_left + (rel_idx as f32 + 0.5) * bar_w;
                let y_open = price_to_y(bar.open);
                let y_high = price_to_y(bar.high);
                let y_low = price_to_y(bar.low);
                let y_close = price_to_y(bar.close);
                let is_wknd = chart.gap_fill_timestamps.contains(&bar.ts_ms);
                let color = if is_wknd {
                    if bar.close >= bar.open {
                        egui::Color32::from_rgb(255, 0, 255)
                    } else {
                        egui::Color32::from_rgb(180, 0, 180)
                    }
                } else {
                    if bar.close >= bar.open { UP } else { DOWN }
                };
                let tick = half_body.max(2.0);

                // Vertical line
                painter.line_segment(
                    [egui::pos2(cx, y_high), egui::pos2(cx, y_low)],
                    egui::Stroke::new(1.0, color),
                );
                // Open tick (left)
                painter.line_segment(
                    [egui::pos2(cx - tick, y_open), egui::pos2(cx, y_open)],
                    egui::Stroke::new(1.0, color),
                );
                // Close tick (right)
                painter.line_segment(
                    [egui::pos2(cx, y_close), egui::pos2(cx + tick, y_close)],
                    egui::Stroke::new(1.0, color),
                );
            }
        }
        ChartType::Candle | ChartType::HeikinAshi | ChartType::Renko => {
            let weekend_up = egui::Color32::from_rgb(255, 0, 255); // magenta bull (gap-fill/weekend)
            let weekend_dn = egui::Color32::from_rgb(180, 0, 180); // dark magenta bear (weekend gap-fill)
            // Volume heatmap uses pre-computed vol_avg_20 from ChartState (no per-frame alloc)
            let vol_avg = &chart.vol_avg_20;
            for (rel_idx, bar) in render_bars.iter().enumerate().step_by(render_step) {
                let cx = data_left + (rel_idx as f32 + 0.5) * bar_w;
                let y_open = price_to_y(bar.open);
                let y_high = price_to_y(bar.high);
                let y_low = price_to_y(bar.low);
                let y_close = price_to_y(bar.close);
                // Gap-fill bars (Kraken) get magenta color.
                // Use explicit timestamp tracking rather than day-of-week:
                // per-source TZ offsets make day-of-week unreliable for weekend detection.
                let is_weekend = chart.gap_fill_timestamps.contains(&bar.ts_ms);
                let color = if flags.vol_heatmap && !vol_avg.is_empty() {
                    // Volume heatmap: blue (low) → green → yellow → red (high)
                    let abs_idx = start_idx + rel_idx;
                    let avg = if abs_idx < vol_avg.len() && vol_avg[abs_idx] > 0.0 {
                        vol_avg[abs_idx]
                    } else {
                        1.0
                    };
                    let ratio = (bar.volume / avg).min(3.0) / 3.0; // 0..1, capped at 3x avg
                    if ratio < 0.33 {
                        // Blue to green
                        let t = ratio / 0.33;
                        let r = (40.0 * (1.0 - t)) as u8;
                        let g = (80.0 + 140.0 * t) as u8;
                        let b = (200.0 * (1.0 - t)) as u8;
                        egui::Color32::from_rgb(r, g, b)
                    } else if ratio < 0.66 {
                        // Green to yellow
                        let t = (ratio - 0.33) / 0.33;
                        let r = (220.0 * t) as u8;
                        let g = (220.0 - 30.0 * t) as u8;
                        egui::Color32::from_rgb(r, g, 0)
                    } else {
                        // Yellow to red
                        let t = (ratio - 0.66) / 0.34;
                        let g = (190.0 * (1.0 - t)) as u8;
                        egui::Color32::from_rgb(230, g, 0)
                    }
                } else if is_weekend {
                    if bar.close >= bar.open {
                        weekend_up
                    } else {
                        weekend_dn
                    }
                } else if chart.primary_first_ts > 0 && bar.ts_ms < chart.primary_first_ts {
                    // Backfill data (older than primary source) — same magenta as weekend
                    if bar.close >= bar.open {
                        weekend_up
                    } else {
                        weekend_dn
                    }
                } else {
                    if bar.close >= bar.open { UP } else { DOWN }
                };

                // Wick
                painter.line_segment(
                    [egui::pos2(cx, y_high), egui::pos2(cx, y_low)],
                    egui::Stroke::new(1.0, color),
                );

                // Body
                let body_top = y_open.min(y_close);
                let body_bottom = y_open.max(y_close);
                let body_height = (body_bottom - body_top).max(1.0);
                let body_rect = egui::Rect::from_min_size(
                    egui::pos2(cx - half_body, body_top),
                    egui::vec2(candle_w, body_height),
                );

                if body_height > 2.0 {
                    // Solid filled candles (TradingView/lightweight-charts style)
                    painter.rect_filled(body_rect, 0.0, color);
                } else {
                    // Doji: single line
                    painter.line_segment(
                        [
                            egui::pos2(cx - half_body, body_top),
                            egui::pos2(cx + half_body, body_top),
                        ],
                        egui::Stroke::new(1.0, color),
                    );
                }
            }
        }
    }

    // ── Extended Hours Candle (magenta, TradingView-style) ─────────────
    // Only render when real pre/post-market data is present (`ext_active`).
    // During CORE/regular hours there is no extended-hours candle, so we draw
    // nothing here. The previous grey "ghost" placeholder was a fabricated
    // `last.close ± 0.3·range` candle: it carried no information, showed during
    // CORE while the live bar was already forming, and also cluttered 24/7
    // crypto charts — so it has been removed.
    if chart.ext_active && chart.ext_high > 0.0 {
        if let Some(next_x) =
            adjacent_projection_candle_x(data_left, bars.len(), bar_w, half_body, chart_rect)
        {
            // Real extended hours candle (magenta)
            let ext_col = egui::Color32::from_rgb(200, 50, 200); // Magenta
            let y_open = price_to_y(chart.ext_open);
            let y_high = price_to_y(chart.ext_high);
            let y_low = price_to_y(chart.ext_low);
            let y_close = price_to_y(chart.ext_close);
            // Wick
            painter.line_segment(
                [egui::pos2(next_x, y_high), egui::pos2(next_x, y_low)],
                egui::Stroke::new(1.0, ext_col),
            );
            // Body
            let body_top = y_open.min(y_close);
            let body_h = (y_open - y_close).abs().max(1.0);
            let body_rect = egui::Rect::from_min_size(
                egui::pos2(next_x - half_body, body_top),
                egui::vec2(candle_w, body_h),
            );
            if body_h > 2.0 {
                painter.rect_filled(body_rect, 0.0, ext_col);
            } else {
                painter.line_segment(
                    [
                        egui::pos2(next_x - half_body, body_top),
                        egui::pos2(next_x + half_body, body_top),
                    ],
                    egui::Stroke::new(1.0, ext_col),
                );
            }
        }
    }

    // ── right price-axis label de-confliction ─────────────────────────────
    // Every boxed price tag on the right axis (last/current, extended-hours,
    // bid, ask) is painted at the same x. When their prices cluster — common
    // for low-priced symbols where bid≈ask≈last — the boxes stack into an
    // unreadable smear. `place_axis_label` tracks the occupied vertical bands
    // and nudges each new tag to the nearest free slot; the underlying dashed
    // line still draws at the true price, only the label moves. Tags are placed
    // in draw order, so earlier (higher-priority) tags keep their preferred y.
    let axis_top = chart_rect.top();
    let axis_bot = chart_rect.bottom();
    let mut occupied_label_bands: Vec<(f32, f32)> = Vec::new();
    let mut place_axis_label = move |desired_center: f32, half_h: f32| -> f32 {
        place_level_label(
            desired_center,
            half_h,
            axis_top,
            axis_bot,
            &mut occupied_label_bands,
        )
    };

    // ── last/core close line ─────────────────────────────────────────────────
    if let Some(last) = bars.last() {
        let current_price = if chart.ext_active && chart.ext_close > 0.0 {
            // During extended hours the `C` tag is the regular-session daily-close
            // reference (the magenta EXT tag below owns the extended-hours last).
            // Use the SAME authoritative close as the "Daily Close" header
            // (chart.ext_open = the shared quote's regular_close), which is
            // timeframe-independent. last.close is the chart's own last-bar close
            // and can desync across timeframes / data sources (e.g. delayed-iapi
            // xStocks like WOK), which made the `C` tag disagree with the header.
            if chart.ext_open > 0.0 {
                chart.ext_open
            } else {
                last.close
            }
        } else {
            fresh_live_mid.unwrap_or(last.close)
        };
        let y = price_to_y(current_price);
        if y >= chart_rect.top() && y <= chart_rect.bottom() {
            let color = if chart.ext_active && chart.ext_close > 0.0 {
                // The `C` tag is the regular/daily close reference. Color it
                // against the previous daily close, not the current intraday
                // candle open; otherwise a down day can look green just because
                // the close finished above that bar's open while EXT is active.
                close_reference_color(current_price, last.open, &chart.bars)
            } else if current_price >= last.open {
                UP
            } else {
                DOWN
            };
            // Dashed line
            let dash_len = 6.0_f32;
            let mut x = chart_rect.left();
            while x < chart_rect.right() {
                let end = (x + dash_len).min(chart_rect.right());
                painter.line_segment(
                    [egui::pos2(x, y), egui::pos2(end, y)],
                    egui::Stroke::new(1.0, color),
                );
                x += dash_len * 2.0;
            }
            // Price label + TradingView-style countdown to the next candle close.
            let label = if chart.ext_active && chart.ext_close > 0.0 {
                format_axis_price_label("C", current_price)
            } else {
                format_price(current_price)
            };
            let countdown = if chart.ext_active && chart.ext_close > 0.0 {
                // Countdown belongs to a forming regular-session bar. During
                // ext-hours this tag is the static regular close reference, so
                // showing a rolling timer under it is misleading.
                None
            } else {
                chart.bars.last().and_then(|latest| {
                    next_candle_countdown_label_for_market(
                        latest.ts_ms,
                        chart.timeframe,
                        chart.primary_source,
                        &chart.symbol,
                    )
                })
            };
            if let Some(countdown) = countdown {
                // TradingView-style current-price tag: ticker / price / countdown
                // stacked, each in its OWN bordered box. The timer used to be a
                // borderless cell that blended into the chart and was hard to read
                // against the price; now all three rows are delineated and the
                // ticker is shown for context.
                let ticker = bare_symbol_from_key(&chart.symbol);
                let row_h = 14.0_f32;
                let badge_h = row_h * 3.0;
                let label_y = place_axis_label(y, badge_h * 0.5);
                let badge_left = chart_rect.right() + 2.0;
                let badge_w = price_axis_w - 4.0;
                let ticker_rect = egui::Rect::from_min_size(
                    egui::pos2(badge_left, label_y - badge_h * 0.5),
                    egui::vec2(badge_w, row_h),
                );
                let price_rect = egui::Rect::from_min_size(
                    egui::pos2(badge_left, ticker_rect.bottom()),
                    egui::vec2(badge_w, row_h),
                );
                let timer_rect = egui::Rect::from_min_size(
                    egui::pos2(badge_left, price_rect.bottom()),
                    egui::vec2(badge_w, row_h),
                );
                let bg = egui::Color32::from_rgb(12, 18, 28);
                let border = egui::Stroke::new(1.0, color);
                for r in [ticker_rect, price_rect, timer_rect] {
                    painter.rect_filled(r, 2.0, bg);
                    painter.rect_stroke(r, 2.0, border, egui::StrokeKind::Inside);
                }
                let text_x = badge_left + 3.0;
                painter.text(
                    egui::pos2(text_x, ticker_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    &ticker,
                    egui::FontId::monospace(9.0),
                    egui::Color32::from_rgb(190, 205, 225),
                );
                painter.text(
                    egui::pos2(text_x, price_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    &label,
                    egui::FontId::monospace(10.0),
                    color,
                );
                painter.text(
                    egui::pos2(text_x, timer_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    &countdown,
                    egui::FontId::monospace(9.0),
                    egui::Color32::from_rgb(215, 230, 245),
                );
            } else {
                let label_y = place_axis_label(y, 8.0);
                let lbl_rect = egui::Rect::from_min_size(
                    egui::pos2(chart_rect.right() + 2.0, label_y - 8.0),
                    egui::vec2(price_axis_w - 4.0, 16.0),
                );
                painter.rect_filled(lbl_rect, 2.0, egui::Color32::from_rgb(12, 18, 28));
                painter.rect_stroke(
                    lbl_rect,
                    2.0,
                    egui::Stroke::new(1.0, color),
                    egui::StrokeKind::Inside,
                );
                painter.text(
                    egui::pos2(chart_rect.right() + 4.0, label_y),
                    egui::Align2::LEFT_CENTER,
                    &label,
                    egui::FontId::monospace(10.0),
                    color,
                );
            }
        }
    }

    // ── Extended hours price line (magenta dashed) ─────────────────────────
    if chart.ext_active && chart.ext_close > 0.0 {
        let y = price_to_y(chart.ext_close);
        if y >= chart_rect.top() && y <= chart_rect.bottom() {
            let ext_col = egui::Color32::from_rgb(200, 50, 200);
            let dash_len = 4.0_f32;
            let mut x = chart_rect.left();
            while x < chart_rect.right() {
                let end = (x + dash_len).min(chart_rect.right());
                painter.line_segment(
                    [egui::pos2(x, y), egui::pos2(end, y)],
                    egui::Stroke::new(1.0, ext_col),
                );
                x += dash_len * 2.0;
            }
            // Price label. Prefix it so extended-hours last cannot be confused
            // with the regular daily close tag.
            let label = format_axis_price_label("EXT", chart.ext_close);
            let label_y = place_axis_label(y, 8.0);
            let lbl_rect = egui::Rect::from_min_size(
                egui::pos2(chart_rect.right() + 2.0, label_y - 8.0),
                egui::vec2(price_axis_w - 4.0, 16.0),
            );
            painter.rect_filled(lbl_rect, 2.0, ext_col);
            painter.text(
                egui::pos2(chart_rect.right() + 4.0, label_y),
                egui::Align2::LEFT_CENTER,
                &label,
                egui::FontId::monospace(10.0),
                egui::Color32::BLACK,
            );
        }
    }

    // ── Bid/Ask spread lines (live streaming quotes) ──────────────────────
    // Hide the spread lines once the streaming quote goes stale (>30s without a
    // tick) so a frozen bid/ask isn't drawn as if live next to a moving candle —
    // the source of the "ask/bid/last decoupled" confusion. Delayed quotes (iapi
    // equities, always delayed=true) are likewise not real-time top-of-book: for a
    // non-WS-tokenized xStock they sit far from the consolidated last and are the
    // direct cause of the chart-vs-watchlist bid/ask desync, so never draw them.
    let quote_fresh = !chart.live_quote_delayed
        && chart
            .live_quote_at
            .is_some_and(|t| t.elapsed() < std::time::Duration::from_secs(30));
    if quote_fresh && chart.live_bid > 0.0 && chart.live_ask > 0.0 {
        let bid_y = price_to_y(chart.live_bid);
        let ask_y = price_to_y(chart.live_ask);
        let bid_col = egui::Color32::from_rgba_premultiplied(0, 200, 80, 150);
        let ask_col = egui::Color32::from_rgba_premultiplied(220, 50, 50, 150);
        let bid_text_col = egui::Color32::from_rgb(0, 220, 80);
        let ask_text_col = egui::Color32::from_rgb(255, 90, 90);
        let label_bg = egui::Color32::from_rgb(12, 18, 28);
        if bid_y >= chart_rect.top() && bid_y <= chart_rect.bottom() {
            painter.line_segment(
                [
                    egui::pos2(chart_rect.left(), bid_y),
                    egui::pos2(chart_rect.right(), bid_y),
                ],
                egui::Stroke::new(0.75, bid_col),
            );
            let bid_label_y = place_axis_label(bid_y, 8.0);
            let lbl_rect = egui::Rect::from_min_size(
                egui::pos2(chart_rect.right() + 2.0, bid_label_y - 8.0),
                egui::vec2(price_axis_w - 4.0, 16.0),
            );
            painter.rect_filled(lbl_rect, 2.0, label_bg);
            painter.rect_stroke(
                lbl_rect,
                2.0,
                egui::Stroke::new(1.0, bid_text_col),
                egui::StrokeKind::Inside,
            );
            let label = format!("Bid {}", format_price(chart.live_bid));
            painter.text(
                egui::pos2(chart_rect.right() + 4.0, bid_label_y),
                egui::Align2::LEFT_CENTER,
                label,
                egui::FontId::monospace(9.0),
                bid_text_col,
            );
        }
        if ask_y >= chart_rect.top() && ask_y <= chart_rect.bottom() {
            painter.line_segment(
                [
                    egui::pos2(chart_rect.left(), ask_y),
                    egui::pos2(chart_rect.right(), ask_y),
                ],
                egui::Stroke::new(0.75, ask_col),
            );
            let ask_label_y = place_axis_label(ask_y, 8.0);
            let lbl_rect = egui::Rect::from_min_size(
                egui::pos2(chart_rect.right() + 2.0, ask_label_y - 8.0),
                egui::vec2(price_axis_w - 4.0, 16.0),
            );
            painter.rect_filled(lbl_rect, 2.0, label_bg);
            painter.rect_stroke(
                lbl_rect,
                2.0,
                egui::Stroke::new(1.0, ask_text_col),
                egui::StrokeKind::Inside,
            );
            let label = format!("Ask {}", format_price(chart.live_ask));
            painter.text(
                egui::pos2(chart_rect.right() + 4.0, ask_label_y),
                egui::Align2::LEFT_CENTER,
                label,
                egui::FontId::monospace(9.0),
                ask_text_col,
            );
        }
    }

    // ── Volume Profile overlay (volume-at-price with POC + Value Area) ─────
    if flags.price_histogram {
        let num_buckets = (chart_rect.height() / 4.0).max(10.0) as usize;
        let bucket_h = chart_rect.height() / num_buckets as f32;
        let mut buckets = vec![0.0_f64; num_buckets];
        let mut buy_vol = vec![0.0_f64; num_buckets]; // close > open = buying pressure
        let mut max_vol = 0.0_f64;

        for bar in bars {
            let y_high_frac = ((price_max - bar.high) / (price_max - price_min)).clamp(0.0, 1.0);
            let y_low_frac = ((price_max - bar.low) / (price_max - price_min)).clamp(0.0, 1.0);
            let b_top = (y_high_frac * num_buckets as f64) as usize;
            let b_bot = ((y_low_frac * num_buckets as f64) as usize).min(num_buckets - 1);
            let span = (b_bot - b_top).max(1) as f64;
            let vol_per_level = bar.volume / span;
            let is_buy = bar.close >= bar.open;
            for b in b_top..=b_bot {
                if b < num_buckets {
                    buckets[b] += vol_per_level;
                    if is_buy {
                        buy_vol[b] += vol_per_level;
                    }
                    max_vol = max_vol.max(buckets[b]);
                }
            }
        }

        // POC = highest volume bucket
        let poc_idx = buckets
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(0);

        // Value Area: expand from POC until 70% of total volume
        let total_vol: f64 = buckets.iter().sum();
        let va_target = total_vol * 0.7;
        let mut va_vol = buckets[poc_idx];
        let mut va_lo = poc_idx;
        let mut va_hi = poc_idx;
        while va_vol < va_target && (va_lo > 0 || va_hi < num_buckets - 1) {
            let expand_lo = if va_lo > 0 { buckets[va_lo - 1] } else { 0.0 };
            let expand_hi = if va_hi < num_buckets - 1 {
                buckets[va_hi + 1]
            } else {
                0.0
            };
            if expand_lo >= expand_hi && va_lo > 0 {
                va_lo -= 1;
                va_vol += buckets[va_lo];
            } else if va_hi < num_buckets - 1 {
                va_hi += 1;
                va_vol += buckets[va_hi];
            } else {
                break;
            }
        }

        // Draw horizontal bars: buy (teal) left, sell (red) right, POC highlighted
        let max_bar_w = chart_rect.width() * 0.18;
        let poc_col = egui::Color32::from_rgba_premultiplied(255, 215, 0, 120); // gold
        let va_buy = egui::Color32::from_rgba_premultiplied(38, 166, 154, 60); // teal
        let va_sell = egui::Color32::from_rgba_premultiplied(239, 83, 80, 60); // red
        let out_buy = egui::Color32::from_rgba_premultiplied(38, 166, 154, 30);
        let out_sell = egui::Color32::from_rgba_premultiplied(239, 83, 80, 30);
        let edge_col = egui::Color32::from_rgba_premultiplied(100, 140, 255, 80);
        for (i, &vol) in buckets.iter().enumerate() {
            if vol <= 0.0 {
                continue;
            }
            let frac = (vol / max_vol) as f32;
            let total_w = frac * max_bar_w;
            let buy_frac = if vol > 0.0 {
                (buy_vol[i] / vol) as f32
            } else {
                0.5
            };
            let buy_w = total_w * buy_frac;
            let sell_w = total_w - buy_w;
            let y_top = chart_rect.top() + i as f32 * bucket_h;
            let y_bot = y_top + bucket_h;
            let is_va = i >= va_lo && i <= va_hi;

            if i == poc_idx {
                // POC: full-width gold highlight line
                painter.rect_filled(
                    egui::Rect::from_min_max(
                        egui::pos2(chart_rect.right() - total_w, y_top),
                        egui::pos2(chart_rect.right(), y_bot),
                    ),
                    0.0,
                    poc_col,
                );
            } else {
                // Buy volume (right-aligned, teal)
                let (bc, sc) = if is_va {
                    (va_buy, va_sell)
                } else {
                    (out_buy, out_sell)
                };
                painter.rect_filled(
                    egui::Rect::from_min_max(
                        egui::pos2(chart_rect.right() - total_w, y_top),
                        egui::pos2(chart_rect.right() - sell_w, y_bot),
                    ),
                    0.0,
                    bc,
                );
                // Sell volume
                painter.rect_filled(
                    egui::Rect::from_min_max(
                        egui::pos2(chart_rect.right() - sell_w, y_top),
                        egui::pos2(chart_rect.right(), y_bot),
                    ),
                    0.0,
                    sc,
                );
            }
            // Left edge
            painter.line_segment(
                [
                    egui::pos2(chart_rect.right() - total_w, y_top),
                    egui::pos2(chart_rect.right() - total_w, y_bot),
                ],
                egui::Stroke::new(0.5, edge_col),
            );
        }
        // POC dashed line across chart
        {
            let poc_y = chart_rect.top() + (poc_idx as f32 + 0.5) * bucket_h;
            let mut px = chart_rect.left();
            while px < chart_rect.right() {
                let end = (px + 4.0).min(chart_rect.right());
                painter.line_segment(
                    [egui::pos2(px, poc_y), egui::pos2(end, poc_y)],
                    egui::Stroke::new(1.0, egui::Color32::from_rgba_premultiplied(255, 215, 0, 80)),
                );
                px += 8.0;
            }
        }
    }

    // Redraw primary NNFX trend overlays after translucent zones/volume profile so
    // Supply/Demand rectangles cannot bury the MultiKAMA and MTF_MA levels.
    if flags.sma200 && !chart.mtf_sma.is_empty() {
        for (label, projected) in &chart.mtf_sma {
            let color = match label.as_str() {
                "H1 200" => egui::Color32::from_rgb(255, 99, 71),
                _ => egui::Color32::from_rgb(255, 0, 255),
            };
            let mut prev: Option<egui::Pos2> = None;
            for &(bar_idx, sma_val) in projected {
                if bar_idx >= start_idx && bar_idx < end_idx {
                    let rel = bar_idx - start_idx;
                    let x = data_left + (rel as f32 + 0.5) * bar_w;
                    let pt = egui::pos2(x, price_to_y(sma_val));
                    if let Some(prev_pt) = prev {
                        if let Some([a, b]) = clip_line_segment_to_rect(prev_pt, pt, chart_rect) {
                            painter.line_segment([a, b], egui::Stroke::new(2.25, color));
                        }
                    }
                    prev = Some(pt);
                }
            }
        }
    }
    if flags.kama && !chart.multi_kama.is_empty() {
        for (_tf_label, projected) in &chart.multi_kama {
            let mut prev: Option<egui::Pos2> = None;
            for &(bar_idx, kama_val) in projected {
                if bar_idx >= start_idx && bar_idx < end_idx {
                    let rel = bar_idx - start_idx;
                    let x = data_left + (rel as f32 + 0.5) * bar_w;
                    let pt = egui::pos2(x, price_to_y(kama_val));
                    if let Some(prev_pt) = prev {
                        if let Some([a, b]) = clip_line_segment_to_rect(prev_pt, pt, chart_rect) {
                            painter.line_segment(
                                [a, b],
                                egui::Stroke::new(2.25, egui::Color32::from_rgb(255, 255, 255)),
                            );
                        }
                    }
                    prev = Some(pt);
                }
            }
        }
    }

    // ── symbol / tf header geometry ─────────────────────────────────────────
    // Compute this before the crosshair data window so the hover readout can
    // anchor underneath the same decorated header instead of being hidden by it.
    // Append the full company name when one is known and the viewport is wide
    // enough to carry it — keeps tiny MTF grid cells to the compact "SYM [TF]"
    // badge while the single chart and larger cells show "SYM [TF] · Company".
    // 240 px threshold chosen so Reg SHO / EXT badges still fit on the right
    // after the 25-char name cap.
    let sym_label = match company_name {
        Some(name) if chart_rect.width() >= 240.0 => {
            // Always show the full company name (no truncation).
            // The Reg SHO badge is protected by drawing order and the dynamic
            // 18-char cap only when a regulatory alert is present.
            format!("{} [{}] · {}", chart.symbol, chart.timeframe.label(), name)
        }
        _ => format!("{} [{}]", chart.symbol, chart.timeframe.label()),
    };
    let header_pos = egui::pos2(chart_rect.left() + 8.0, chart_rect.top() + 6.0);
    let header_pad_x = 6.0_f32;
    let header_pad_y = 3.0_f32;
    let sym_font = egui::FontId::monospace(11.0);
    let sym_galley = painter.layout_no_wrap(sym_label, sym_font, egui::Color32::WHITE);
    let sym_rect = egui::Rect::from_min_size(
        header_pos,
        egui::vec2(
            sym_galley.rect.width() + header_pad_x * 2.0,
            sym_galley.rect.height() + header_pad_y * 2.0,
        ),
    );

    // ── crosshair ────────────────────────────────────────────────────────────
    if let Some(pos) = crosshair {
        if chart_rect.contains(pos) {
            let ch_color = egui::Color32::from_rgba_premultiplied(180, 180, 200, 100);
            painter.line_segment(
                [
                    egui::pos2(pos.x, chart_rect.top()),
                    egui::pos2(pos.x, chart_rect.bottom()),
                ],
                egui::Stroke::new(0.5, ch_color),
            );
            painter.line_segment(
                [
                    egui::pos2(chart_rect.left(), pos.y),
                    egui::pos2(chart_rect.right(), pos.y),
                ],
                egui::Stroke::new(0.5, ch_color),
            );

            // Price label on right axis
            let frac = (pos.y - chart_rect.top()) / chart_rect.height();
            let price = price_max - frac as f64 * (price_max - price_min);
            let label = format_price(price);
            let lbl_rect = egui::Rect::from_min_size(
                egui::pos2(chart_rect.right() + 2.0, pos.y - 8.0),
                egui::vec2(price_axis_w - 4.0, 16.0),
            );
            painter.rect_filled(lbl_rect, 2.0, egui::Color32::from_rgb(50, 50, 80));
            painter.text(
                egui::pos2(chart_rect.right() + 4.0, pos.y),
                egui::Align2::LEFT_CENTER,
                &label,
                egui::FontId::monospace(10.0),
                egui::Color32::WHITE,
            );

            // OHLCV + indicator values data window (WebKit: .data-window — #000000ee bg)
            let rel_x = pos.x - chart_rect.left();
            let bar_idx = ((rel_x / bar_w) as usize).min(bars.len().saturating_sub(1));
            if bar_idx < bars.len() {
                let b = &bars[bar_idx];

                // Date/time tag on the bottom time axis (mirrors the right-axis
                // price tag) — the TradingView-style readout of the hovered bar's
                // timestamp, formatted per timeframe (intraday shows time, daily+
                // shows the date).
                {
                    let mut ts_buf = String::with_capacity(20);
                    format_ts_buf(b.ts_ms, chart.timeframe, &mut ts_buf);
                    let ts_galley = painter.layout_no_wrap(
                        ts_buf,
                        egui::FontId::monospace(10.0),
                        egui::Color32::WHITE,
                    );
                    let tw = ts_galley.rect.width();
                    let th = ts_galley.rect.height();
                    let box_w = tw + 10.0;
                    let half = box_w * 0.5;
                    // Centre on the crosshair x, clamped to keep the tag inside the
                    // chart's horizontal span. Guard the clamp: a very narrow MTF
                    // cell can be slimmer than the tag, where lo > hi would panic.
                    let lo = chart_rect.left() + half;
                    let hi = chart_rect.right() - half;
                    let cx = if lo <= hi {
                        pos.x.clamp(lo, hi)
                    } else {
                        chart_rect.center().x
                    };
                    let ts_rect = egui::Rect::from_center_size(
                        egui::pos2(cx, chart_rect.bottom() + 10.0),
                        egui::vec2(box_w, 16.0),
                    );
                    painter.rect_filled(ts_rect, 2.0, egui::Color32::from_rgb(50, 50, 80));
                    painter.galley(
                        egui::pos2(cx - tw * 0.5, ts_rect.center().y - th * 0.5),
                        ts_galley,
                        egui::Color32::WHITE,
                    );
                }

                let abs_idx = start_idx + bar_idx;
                let tooltip = format!(
                    "O:{} H:{} L:{} C:{} V:{:.0}",
                    format_price(b.open),
                    format_price(b.high),
                    format_price(b.low),
                    format_price(b.close),
                    b.volume
                );
                // Indicator values on second line
                let mut ind_parts: Vec<String> = Vec::new();
                if flags.sma200 {
                    if let Some(Some(v)) = chart.sma200.get(abs_idx) {
                        ind_parts.push(format!("SMA200:{}", format_price(*v)));
                    }
                }
                if flags.sma100 {
                    if let Some(Some(v)) = chart.sma100.get(abs_idx) {
                        ind_parts.push(format!("SMA100:{}", format_price(*v)));
                    }
                }
                if flags.kama {
                    if let Some(Some(v)) = chart.kama.get(abs_idx) {
                        ind_parts.push(format!("KAMA:{}", format_price(*v)));
                    }
                }
                if flags.ema21 {
                    if let Some(Some(v)) = chart.ema21.get(abs_idx) {
                        ind_parts.push(format!("EMA21:{}", format_price(*v)));
                    }
                }
                if show_rsi {
                    if let Some(Some(v)) = chart.rsi.get(abs_idx) {
                        ind_parts.push(format!("RSI:{:.1}", v));
                    }
                }
                if show_cmo {
                    if let Some(Some(v)) = chart.cmo.get(abs_idx) {
                        ind_parts.push(format!("CMO:{:+.1}", v));
                    }
                }
                if show_qstick {
                    if let Some(Some(v)) = chart.qstick.get(abs_idx) {
                        ind_parts.push(format!("QStick:{:+.3}", v));
                    }
                }
                if show_disparity {
                    if let Some(Some(v)) = chart.disparity.get(abs_idx) {
                        ind_parts.push(format!("Disp:{:+.2}%", v));
                    }
                }
                if show_bop {
                    if let Some(Some(v)) = chart.bop.get(abs_idx) {
                        ind_parts.push(format!("BOP:{:+.3}", v));
                    }
                }
                if show_stddev {
                    if let Some(Some(v)) = chart.stddev.get(abs_idx) {
                        ind_parts.push(format!("StdDev:{:.3}", v));
                    }
                }
                if show_mfi {
                    if let Some(Some(v)) = chart.mfi.get(abs_idx) {
                        ind_parts.push(format!("MFI:{:.1}", v));
                    }
                }
                if show_trix {
                    if let Some(Some(v)) = chart.trix_line.get(abs_idx) {
                        ind_parts.push(format!("TRIX:{:+.3}", v));
                    }
                }
                if show_ppo {
                    if let Some(Some(v)) = chart.ppo_line.get(abs_idx) {
                        ind_parts.push(format!("PPO:{:+.2}", v));
                    }
                }
                if show_ultosc {
                    if let Some(Some(v)) = chart.ultosc.get(abs_idx) {
                        ind_parts.push(format!("ULT:{:.1}", v));
                    }
                }
                if show_stochrsi {
                    if let (Some(Some(k)), Some(Some(d))) =
                        (chart.stochrsi_k.get(abs_idx), chart.stochrsi_d.get(abs_idx))
                    {
                        ind_parts.push(format!("StochRSI:{:.1}/{:.1}", k, d));
                    }
                }
                if show_var_oscillator {
                    if let Some(Some(v)) = chart.var_oscillator.get(abs_idx) {
                        ind_parts.push(format!("VaR:{:.1}", v));
                    }
                }
                if let Some(Some(v)) = chart.atr.get(abs_idx) {
                    ind_parts.push(format!("ATR:{}", format_price(*v)));
                }
                let ind_text = (!ind_parts.is_empty()).then(|| ind_parts.join("  "));
                let data_chars = ind_text
                    .as_ref()
                    .map(|s| tooltip.len().max(s.len()))
                    .unwrap_or(tooltip.len());
                let data_h = if ind_text.is_some() { 34.0 } else { 20.0 };
                // Anchor below both the symbol header row AND the indicator legend
                // row (which starts at ~top+34) so the hover readout never overlaps
                // either overlay and remains readable in all MTF/single views.
                let legend_row = chart_rect.top() + 38.0;
                let data_y = (sym_rect.bottom() + 22.0)
                    .max(legend_row)
                    .min((chart_rect.bottom() - data_h - 2.0).max(chart_rect.top() + 2.0));
                // Semi-transparent background behind data text. It intentionally
                // sits under the symbol/timeframe header with matching blue trim,
                // instead of competing for the same top-left pixels.
                let data_bg = egui::Rect::from_min_size(
                    egui::pos2(header_pos.x, data_y),
                    egui::vec2(data_chars as f32 * 6.5 + 12.0, data_h),
                );
                painter.rect_filled(
                    data_bg,
                    3.0,
                    egui::Color32::from_rgba_premultiplied(0, 0, 0, 238),
                );
                painter.rect_stroke(
                    data_bg,
                    3.0,
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(70, 120, 180)),
                    egui::StrokeKind::Inside,
                );
                painter.text(
                    egui::pos2(data_bg.left() + 6.0, data_bg.top() + 4.0),
                    egui::Align2::LEFT_TOP,
                    &tooltip,
                    egui::FontId::monospace(10.0),
                    egui::Color32::from_rgb(220, 220, 255),
                );
                if let Some(ind_text) = ind_text {
                    painter.text(
                        egui::pos2(data_bg.left() + 6.0, data_bg.top() + 18.0),
                        egui::Align2::LEFT_TOP,
                        &ind_text,
                        egui::FontId::monospace(10.0),
                        egui::Color32::from_rgb(180, 180, 200),
                    );
                }
            }
        }
    }

    // ── symbol / tf label (WebKit: .mtf-cell-label — #8cf, 11px bold, text-shadow)
    // Box the symbol first, then attach the extended-hours context to that same
    // header row. The old standalone EXT badge sat underneath this symbol text;
    // drawing one joined header makes ownership obvious and prevents overlap.
    // Every cell self-labels with the full "SYM [TF]" badge — same as the
    // single-chart view — so the MTF grid needs no separate symbol header.
    painter.rect_filled(
        sym_rect,
        3.0,
        egui::Color32::from_rgba_premultiplied(8, 12, 18, 232),
    );
    painter.rect_stroke(
        sym_rect,
        3.0,
        egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 150, 210)),
        egui::StrokeKind::Inside,
    );
    painter.galley(
        egui::pos2(
            sym_rect.left() + header_pad_x,
            sym_rect.center().y - sym_galley.rect.height() * 0.5,
        ),
        sym_galley,
        egui::Color32::WHITE,
    );

    // Regulatory alerts extracted to chart_helpers for modularity.
    let _header_right = draw_regulatory_alerts_header(
        painter,
        sym_rect,
        chart_rect,
        header_pad_x,
        regulatory_alerts,
    );

    draw_extended_hours_header_badge(painter, chart, bars, sym_rect, header_pad_x);

    // ── indicator legend ─────────────────────────────────────────────────────
    draw_indicator_legend(painter, chart, chart_rect, sym_rect, flags);

    // Chart overlay removed — info shown in crosshair tooltip + right panel instead

    // ── sub-panes (RSI, Fisher) ──────────────────────────────────────────────
    let mut sub_y = main_rect.bottom();

    if show_rsi {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_oscillator_pane(
            painter,
            pane_rect,
            bars,
            &chart.rsi,
            start_idx,
            bar_w,
            "RSI(14)",
            RSI_LINE,
            0.0,
            100.0,
            Some(70.0),
            Some(30.0),
        );
        sub_y += 80.0;
    }

    if show_fisher {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_fisher_pane(
            painter,
            pane_rect,
            bars,
            &chart.fisher,
            &chart.fisher_signal,
            start_idx,
            bar_w,
        );
        sub_y += 80.0;
    }

    if show_macd {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_macd_pane(
            painter,
            pane_rect,
            bars,
            &chart.macd_line,
            &chart.macd_signal,
            &chart.macd_hist,
            start_idx,
            bar_w,
            "MACD(12,26,9)",
            MACD_LINE_COL,
            MACD_SIG_COL,
        );
        sub_y += 80.0;
    }

    if show_volume_pane {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_volume_pane(painter, pane_rect, bars, start_idx, bar_w);
        sub_y += 80.0;
    }

    if show_stochastic {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_stoch_pane(
            painter,
            pane_rect,
            bars,
            &chart.stoch_k,
            &chart.stoch_d,
            start_idx,
            bar_w,
            "Stoch(14,3,3)",
        );
        sub_y += 80.0;
    }

    if show_adx {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_adx_pane(
            painter,
            pane_rect,
            bars,
            &chart.adx,
            &chart.di_plus,
            &chart.di_minus,
            start_idx,
            bar_w,
        );
        sub_y += 80.0;
    }

    if show_cci {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_oscillator_pane(
            painter,
            pane_rect,
            bars,
            &chart.cci,
            start_idx,
            bar_w,
            "CCI(20)",
            CCI_COL,
            -200.0,
            200.0,
            Some(100.0),
            Some(-100.0),
        );
        sub_y += 80.0;
    }

    if show_williams_r {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_oscillator_pane(
            painter,
            pane_rect,
            bars,
            &chart.williams_r,
            start_idx,
            bar_w,
            "Williams %R(14)",
            WILLR_COL,
            -100.0,
            0.0,
            Some(-20.0),
            Some(-80.0),
        );
        sub_y += 80.0;
    }

    if show_obv {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        // OBV auto-scales
        let mut ob_min = f64::MAX;
        let mut ob_max = f64::MIN;
        for (ri, _) in bars.iter().enumerate() {
            if let Some(Some(v)) = chart.obv.get(start_idx + ri) {
                ob_min = ob_min.min(*v);
                ob_max = ob_max.max(*v);
            }
        }
        let pad = (ob_max - ob_min) * 0.1;
        draw_oscillator_pane(
            painter,
            pane_rect,
            bars,
            &chart.obv,
            start_idx,
            bar_w,
            "OBV",
            OBV_COL,
            ob_min - pad,
            ob_max + pad,
            None,
            None,
        );
        sub_y += 80.0;
    }

    if show_momentum {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        let mut m_min = f64::MAX;
        let mut m_max = f64::MIN;
        for (ri, _) in bars.iter().enumerate() {
            if let Some(Some(v)) = chart.momentum.get(start_idx + ri) {
                m_min = m_min.min(*v);
                m_max = m_max.max(*v);
            }
        }
        let pad = (m_max - m_min).max(0.001) * 0.1;
        draw_oscillator_pane(
            painter,
            pane_rect,
            bars,
            &chart.momentum,
            start_idx,
            bar_w,
            "Momentum(10)",
            egui::Color32::from_rgb(200, 150, 100),
            m_min - pad,
            m_max + pad,
            None,
            None,
        );
        sub_y += 80.0;
    }

    if show_cmo {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_oscillator_pane(
            painter,
            pane_rect,
            bars,
            &chart.cmo,
            start_idx,
            bar_w,
            "CMO(9)",
            egui::Color32::from_rgb(120, 220, 200),
            -100.0,
            100.0,
            Some(50.0),
            Some(-50.0),
        );
        sub_y += 80.0;
    }

    if show_qstick {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        let mut bound = 0.0_f64;
        for (ri, _) in bars.iter().enumerate() {
            if let Some(Some(v)) = chart.qstick.get(start_idx + ri) {
                bound = bound.max(v.abs());
            }
        }
        let bound = bound.max(0.001);
        let pad = bound * 0.1;
        draw_oscillator_pane(
            painter,
            pane_rect,
            bars,
            &chart.qstick,
            start_idx,
            bar_w,
            "QStick(14)",
            egui::Color32::from_rgb(190, 140, 255),
            -(bound + pad),
            bound + pad,
            None,
            None,
        );
        sub_y += 80.0;
    }

    if show_disparity {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        let mut bound = 3.0_f64;
        for (ri, _) in bars.iter().enumerate() {
            if let Some(Some(v)) = chart.disparity.get(start_idx + ri) {
                bound = bound.max(v.abs());
            }
        }
        let pad = bound * 0.1;
        draw_oscillator_pane(
            painter,
            pane_rect,
            bars,
            &chart.disparity,
            start_idx,
            bar_w,
            "Disparity(14)",
            egui::Color32::from_rgb(255, 210, 90),
            -(bound + pad),
            bound + pad,
            Some(3.0),
            Some(-3.0),
        );
        sub_y += 80.0;
    }

    if show_bop {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_oscillator_pane(
            painter,
            pane_rect,
            bars,
            &chart.bop,
            start_idx,
            bar_w,
            "BOP(14)",
            egui::Color32::from_rgb(255, 120, 120),
            -1.0,
            1.0,
            Some(0.5),
            Some(-0.5),
        );
        sub_y += 80.0;
    }

    if show_stddev {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        let mut s_max = 0.0_f64;
        for (ri, _) in bars.iter().enumerate() {
            if let Some(Some(v)) = chart.stddev.get(start_idx + ri) {
                s_max = s_max.max(*v);
            }
        }
        let s_max = s_max.max(1.0);
        let pad = s_max * 0.1;
        draw_oscillator_pane(
            painter,
            pane_rect,
            bars,
            &chart.stddev,
            start_idx,
            bar_w,
            "StdDev(20)",
            egui::Color32::from_rgb(120, 180, 255),
            0.0,
            s_max + pad,
            None,
            None,
        );
        sub_y += 80.0;
    }

    if show_mfi {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_oscillator_pane(
            painter,
            pane_rect,
            bars,
            &chart.mfi,
            start_idx,
            bar_w,
            "MFI(14)",
            MFI_COL,
            0.0,
            100.0,
            Some(80.0),
            Some(20.0),
        );
        sub_y += 80.0;
    }

    if show_trix {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_macd_pane(
            painter,
            pane_rect,
            bars,
            &chart.trix_line,
            &chart.trix_signal,
            &chart.trix_hist,
            start_idx,
            bar_w,
            "TRIX(15,9)",
            TRIX_LINE_COL,
            TRIX_SIG_COL,
        );
        sub_y += 80.0;
    }

    if show_ppo {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_macd_pane(
            painter,
            pane_rect,
            bars,
            &chart.ppo_line,
            &chart.ppo_signal,
            &chart.ppo_hist,
            start_idx,
            bar_w,
            "PPO(12,26,9)",
            PPO_LINE_COL,
            PPO_SIG_COL,
        );
        sub_y += 80.0;
    }

    if show_ultosc {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_oscillator_pane(
            painter,
            pane_rect,
            bars,
            &chart.ultosc,
            start_idx,
            bar_w,
            "ULTOSC(7,14,28)",
            ULTOSC_COL,
            0.0,
            100.0,
            Some(70.0),
            Some(30.0),
        );
        sub_y += 80.0;
    }

    if show_stochrsi {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_stoch_pane(
            painter,
            pane_rect,
            bars,
            &chart.stochrsi_k,
            &chart.stochrsi_d,
            start_idx,
            bar_w,
            "StochRSI(14,14,3,3)",
        );
        sub_y += 80.0;
    }

    if show_var_oscillator {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        let mut bound = 100.0_f64;
        for (ri, _) in bars.iter().enumerate() {
            if let Some(Some(v)) = chart.var_oscillator.get(start_idx + ri) {
                bound = bound.max(v.abs());
            }
        }
        let bound = bound.max(100.0);
        let pad = bound * 0.1;
        draw_oscillator_pane(
            painter,
            pane_rect,
            bars,
            &chart.var_oscillator,
            start_idx,
            bar_w,
            "VaR Osc(20,95%)",
            egui::Color32::from_rgb(255, 170, 80),
            -(bound + pad),
            bound + pad,
            Some(100.0),
            Some(-100.0),
        );
        sub_y += 80.0;
    }

    if show_better_volume {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_better_volume_pane(
            painter,
            pane_rect,
            bars,
            &chart.better_vol_type,
            start_idx,
            bar_w,
        );
        sub_y += 80.0;
    }

    // Ehlers sub-panes
    if show_ehlers_ebsw {
        let pr = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_oscillator_pane(
            painter,
            pr,
            bars,
            &chart.ehlers_ebsw,
            start_idx,
            bar_w,
            "EBSW",
            EHLERS_EBSW_COL,
            -1.0,
            1.0,
            None,
            None,
        );
        sub_y += 80.0;
    }
    if show_ehlers_cyber {
        let pr = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        let mut cmin = f64::MAX;
        let mut cmax = f64::MIN;
        for (ri, _) in bars.iter().enumerate() {
            if let Some(Some(v)) = chart.ehlers_cyber.get(start_idx + ri) {
                cmin = cmin.min(*v);
                cmax = cmax.max(*v);
            }
        }
        let pad = (cmax - cmin).max(0.001) * 0.1;
        draw_oscillator_pane(
            painter,
            pr,
            bars,
            &chart.ehlers_cyber,
            start_idx,
            bar_w,
            "Cyber Cycle",
            EHLERS_CYBER_COL,
            cmin - pad,
            cmax + pad,
            None,
            None,
        );
        sub_y += 80.0;
    }
    if show_ehlers_cg {
        let pr = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        let mut cmin = f64::MAX;
        let mut cmax = f64::MIN;
        for (ri, _) in bars.iter().enumerate() {
            if let Some(Some(v)) = chart.ehlers_cg.get(start_idx + ri) {
                cmin = cmin.min(*v);
                cmax = cmax.max(*v);
            }
        }
        let pad = (cmax - cmin).max(0.001) * 0.1;
        draw_oscillator_pane(
            painter,
            pr,
            bars,
            &chart.ehlers_cg,
            start_idx,
            bar_w,
            "CG Oscillator",
            EHLERS_CG_COL,
            cmin - pad,
            cmax + pad,
            None,
            None,
        );
        sub_y += 80.0;
    }
    if show_ehlers_roof {
        let pr = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        let mut cmin = f64::MAX;
        let mut cmax = f64::MIN;
        for (ri, _) in bars.iter().enumerate() {
            if let Some(Some(v)) = chart.ehlers_roof.get(start_idx + ri) {
                cmin = cmin.min(*v);
                cmax = cmax.max(*v);
            }
        }
        let pad = (cmax - cmin).max(0.001) * 0.1;
        draw_oscillator_pane(
            painter,
            pr,
            bars,
            &chart.ehlers_roof,
            start_idx,
            bar_w,
            "Roofing Filter",
            EHLERS_ROOF_COL,
            cmin - pad,
            cmax + pad,
            None,
            None,
        );
    }

    // ── Squeeze Momentum sub-pane ──────────────────────────────────────────
    if show_squeeze {
        let sr = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        #[allow(unused_assignments)]
        {
            sub_y += 80.0;
        } // last sub-pane
        painter.rect_filled(sr, 0.0, egui::Color32::from_rgb(0, 0, 0));
        painter.line_segment(
            [
                egui::pos2(sr.left(), sr.top()),
                egui::pos2(sr.right(), sr.top()),
            ],
            egui::Stroke::new(1.0, egui::Color32::from_rgb(68, 68, 68)),
        );
        // Find momentum range
        let mut mom_min = f64::MAX;
        let mut mom_max = f64::MIN;
        for (ri, _) in bars.iter().enumerate() {
            if let Some(Some(v)) = chart.squeeze_mom.get(start_idx + ri) {
                mom_min = mom_min.min(*v);
                mom_max = mom_max.max(*v);
            }
        }
        if mom_min >= mom_max {
            mom_min = -1.0;
            mom_max = 1.0;
        }
        let pad = (mom_max - mom_min) * 0.1;
        mom_min -= pad;
        mom_max += pad;
        let val_to_y = |v: f64| -> f32 {
            sr.top() + ((mom_max - v) / (mom_max - mom_min)) as f32 * sr.height()
        };
        let zero_y = val_to_y(0.0);
        // Zero line
        painter.line_segment(
            [
                egui::pos2(sr.left(), zero_y),
                egui::pos2(sr.right(), zero_y),
            ],
            egui::Stroke::new(0.5, egui::Color32::from_rgb(60, 60, 60)),
        );
        // Histogram bars
        for (ri, _) in bars.iter().enumerate() {
            let abs_idx = start_idx + ri;
            if let Some(Some(v)) = chart.squeeze_mom.get(abs_idx) {
                let x = sr.left() + (ri as f32 + 0.5) * bar_w;
                let y = val_to_y(*v);
                let is_squeeze = chart.squeeze_on.get(abs_idx).copied().unwrap_or(false);
                // Color: squeeze=gray, released: positive=cyan, negative=red
                // Momentum direction: increasing=brighter, decreasing=dimmer
                let prev_v = if abs_idx > 0 {
                    chart
                        .squeeze_mom
                        .get(abs_idx - 1)
                        .and_then(|v| *v)
                        .unwrap_or(0.0)
                } else {
                    0.0
                };
                let color = if is_squeeze {
                    egui::Color32::from_rgb(100, 100, 100) // gray = squeeze active
                } else if *v > 0.0 {
                    if *v > prev_v {
                        egui::Color32::from_rgb(0, 220, 200)
                    } else {
                        egui::Color32::from_rgb(0, 120, 100)
                    }
                } else {
                    if *v < prev_v {
                        egui::Color32::from_rgb(220, 50, 50)
                    } else {
                        egui::Color32::from_rgb(120, 30, 30)
                    }
                };
                let half = (bar_w * 0.35).max(0.5);
                painter.rect_filled(
                    egui::Rect::from_min_max(
                        egui::pos2(x - half, y.min(zero_y)),
                        egui::pos2(x + half, y.max(zero_y)),
                    ),
                    0.0,
                    color,
                );
            }
        }
        // Label
        painter.text(
            egui::pos2(sr.left() + 4.0, sr.top() + 2.0),
            egui::Align2::LEFT_TOP,
            "Squeeze",
            egui::FontId::monospace(9.0),
            AXIS_TEXT,
        );
    }

    // ── SL/TP planning lines ───────────────────────────────────────────────
    for (price_opt, label, color) in [
        (&sl_price, "SL", egui::Color32::from_rgb(220, 40, 40)),
        (&tp_price, "TP", egui::Color32::from_rgb(0, 200, 80)),
    ] {
        if let Some(p) = price_opt {
            let y = price_to_y(*p);
            if y >= chart_rect.top() && y <= chart_rect.bottom() {
                let shadow = egui::Color32::from_rgba_premultiplied(0, 0, 0, 190);
                let band =
                    egui::Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 36);
                painter.rect_filled(
                    egui::Rect::from_min_max(
                        egui::pos2(chart_rect.left(), y - 5.0),
                        egui::pos2(chart_rect.right(), y + 5.0),
                    ),
                    0.0,
                    band,
                );
                painter.line_segment(
                    [
                        egui::pos2(chart_rect.left(), y),
                        egui::pos2(chart_rect.right(), y),
                    ],
                    egui::Stroke::new(6.0, shadow),
                );
                painter.line_segment(
                    [
                        egui::pos2(chart_rect.left(), y),
                        egui::pos2(chart_rect.right(), y),
                    ],
                    egui::Stroke::new(3.0, color),
                );

                let pad_x = 6.0_f32;
                let pad_y = 3.0_f32;
                let price_text = format!("{} {}", label, format_price(*p));
                let price_galley = painter.layout_no_wrap(
                    price_text,
                    egui::FontId::monospace(11.0),
                    egui::Color32::BLACK,
                );
                let price_rect = egui::Rect::from_min_size(
                    egui::pos2(
                        chart_rect.left() + 8.0,
                        y - price_galley.rect.height() * 0.5 - pad_y,
                    ),
                    egui::vec2(
                        price_galley.rect.width() + pad_x * 2.0,
                        price_galley.rect.height() + pad_y * 2.0,
                    ),
                );
                painter.rect_filled(price_rect, 3.0, color);
                painter.rect_stroke(
                    price_rect,
                    3.0,
                    egui::Stroke::new(1.0, shadow),
                    egui::StrokeKind::Outside,
                );
                painter.galley(
                    egui::pos2(
                        price_rect.left() + pad_x,
                        price_rect.center().y - price_galley.rect.height() * 0.5,
                    ),
                    price_galley,
                    egui::Color32::BLACK,
                );

                // P&L from last price
                if let Some(last) = bars.last() {
                    let dist = *p - last.close;
                    let dist_label = if dist > 0.0 {
                        format!("+{}", format_price(dist.abs()))
                    } else if dist < 0.0 {
                        format!("-{}", format_price(dist.abs()))
                    } else {
                        format!("±{}", format_price(0.0))
                    };
                    let dist_galley = painter.layout_no_wrap(
                        dist_label,
                        egui::FontId::monospace(10.0),
                        egui::Color32::BLACK,
                    );
                    let dist_rect = egui::Rect::from_min_size(
                        egui::pos2(
                            chart_rect.right() - dist_galley.rect.width() - 26.0,
                            y - dist_galley.rect.height() * 0.5 - pad_y,
                        ),
                        egui::vec2(
                            dist_galley.rect.width() + pad_x * 2.0,
                            dist_galley.rect.height() + pad_y * 2.0,
                        ),
                    );
                    painter.rect_filled(
                        dist_rect,
                        3.0,
                        egui::Color32::from_rgba_premultiplied(
                            color.r(),
                            color.g(),
                            color.b(),
                            220,
                        ),
                    );
                    painter.rect_stroke(
                        dist_rect,
                        3.0,
                        egui::Stroke::new(1.0, shadow),
                        egui::StrokeKind::Outside,
                    );
                    painter.galley(
                        egui::pos2(
                            dist_rect.left() + pad_x,
                            dist_rect.center().y - dist_galley.rect.height() * 0.5,
                        ),
                        dist_galley,
                        egui::Color32::BLACK,
                    );
                }
            }
        }
    }

    // ── Drawing control points (drag handles when selected) ────────────────
    if let Some(sel) = chart.selected_drawing {
        if let Some(drawing) = chart.drawings.get(sel) {
            let cp_size = 4.0_f32; // half-size of control point square
            let cp_fill = egui::Color32::from_rgb(0, 200, 220);
            let cp_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
            // Collect control points as (bar_idx, price)
            let mut cps: Vec<(usize, f64)> = Vec::new();
            match drawing {
                Drawing::HLine { price, .. } => {
                    cps.push((start_idx, *price));
                    cps.push((end_idx.saturating_sub(1), *price));
                }
                Drawing::VLine { bar_idx, .. } => {
                    cps.push((*bar_idx, price_max));
                    cps.push((*bar_idx, price_min));
                }
                Drawing::TrendLine { p1, p2, .. }
                | Drawing::ExtendedLine { p1, p2, .. }
                | Drawing::ArrowLine { p1, p2, .. }
                | Drawing::InfoLine { p1, p2, .. }
                | Drawing::TrendAngle { p1, p2, .. }
                | Drawing::Rectangle { p1, p2, .. }
                | Drawing::Highlighter { p1, p2, .. }
                | Drawing::Ruler { p1, p2, .. }
                | Drawing::MeasureTool { p1, p2, .. }
                | Drawing::Forecast { p1, p2, .. }
                | Drawing::Ellipse { p1, p2, .. }
                | Drawing::SineWave { p1, p2, .. } => {
                    cps.push(*p1);
                    cps.push(*p2);
                }
                Drawing::Pitchfork { pivot, p2, p3, .. }
                | Drawing::SchiffPitchfork { pivot, p2, p3, .. }
                | Drawing::ModSchiffPitchfork { pivot, p2, p3, .. }
                | Drawing::InsidePitchfork { pivot, p2, p3, .. } => {
                    cps.push(*pivot);
                    cps.push(*p2);
                    cps.push(*p3);
                }
                Drawing::FiboExtension { p1, p2, p3, .. }
                | Drawing::FibChannel { p1, p2, p3, .. }
                | Drawing::TrendChannel { p1, p2, p3, .. }
                | Drawing::ArcDraw { p1, p2, p3, .. }
                | Drawing::Triangle { p1, p2, p3, .. }
                | Drawing::RotatedRectangle { p1, p2, p3, .. } => {
                    cps.push(*p1);
                    cps.push(*p2);
                    cps.push(*p3);
                }
                Drawing::Polyline { points, .. }
                | Drawing::ElliottWave { points, .. }
                | Drawing::AbcCorrection { points, .. }
                | Drawing::HeadShoulders { points, .. }
                | Drawing::XabcdPattern { points, .. }
                | Drawing::PathDraw { points, .. } => {
                    for pt in points {
                        cps.push(*pt);
                    }
                }
                _ => {} // single-point tools: no resize handles needed
            }
            for (bi, pr) in &cps {
                if *bi >= start_idx && *bi < end_idx {
                    let x = data_left + ((*bi - start_idx) as f32 + 0.5) * bar_w;
                    let y = price_to_y(*pr);
                    if y >= chart_rect.top() && y <= chart_rect.bottom() {
                        let r = egui::Rect::from_center_size(
                            egui::pos2(x, y),
                            egui::vec2(cp_size * 2.0, cp_size * 2.0),
                        );
                        painter.rect_filled(r, 0.0, cp_fill);
                        painter.rect_stroke(r, 0.0, cp_stroke, egui::StrokeKind::Outside);
                    }
                }
            }
        }
    }

    // ── Compare symbol overlay (% change line) ──────────────────────────
    if let Some(ref _cmp_sym) = chart.compare_symbol {
        if !chart.compare_bars.is_empty() && bars.len() > 1 {
            let cmp = &chart.compare_bars;
            let (start_idx, _end_idx) = chart.visible_range();
            let base_close = chart.bars.get(start_idx).map(|b| b.close).unwrap_or(1.0);
            let cmp_base = cmp
                .get(start_idx.min(cmp.len().saturating_sub(1)))
                .map(|b| b.close)
                .unwrap_or(1.0);
            if base_close > 0.0 && cmp_base > 0.0 {
                let cmp_col = egui::Color32::from_rgb(200, 100, 255); // purple overlay
                let mut prev_pt: Option<egui::Pos2> = None;
                for rel_idx in 0..bars.len() {
                    let abs_idx = start_idx + rel_idx;
                    if abs_idx >= cmp.len() {
                        break;
                    }
                    let cmp_pct = (cmp[abs_idx].close - cmp_base) / cmp_base;
                    let mapped_price = base_close * (1.0 + cmp_pct);
                    let x = data_left + (rel_idx as f32 + 0.5) * bar_w;
                    let y = price_to_y(mapped_price);
                    let pt = egui::pos2(
                        x,
                        clamp_f32_bounds(y, chart_rect.top(), chart_rect.bottom()),
                    );
                    if let Some(pp) = prev_pt {
                        painter.line_segment([pp, pt], egui::Stroke::new(1.5, cmp_col));
                    }
                    prev_pt = Some(pt);
                }
            }
        }
    }

    // ── Broker trade markers (buy/sell arrows + position lines) ────────
    // Position entry/SL/TP lines
    for pl in &trade_overlay.position_lines {
        let y = price_to_y(pl.price);
        if y >= chart_rect.top() && y <= chart_rect.bottom() {
            let (color, label_prefix) = match pl.line_type {
                0 => (
                    if pl.is_buy {
                        egui::Color32::from_rgb(0, 150, 255)
                    } else {
                        egui::Color32::from_rgb(255, 100, 50)
                    },
                    if pl.is_buy { "BUY" } else { "SELL" },
                ),
                1 => (egui::Color32::from_rgb(255, 60, 60), "SL"),
                _ => (egui::Color32::from_rgb(60, 200, 60), "TP"),
            };
            // Dashed line across chart
            let dash_len = 6.0_f32;
            let gap_len = 4.0_f32;
            let mut fx = chart_rect.left();
            while fx < chart_rect.right() {
                let end = (fx + dash_len).min(chart_rect.right());
                painter.line_segment(
                    [egui::pos2(fx, y), egui::pos2(end, y)],
                    egui::Stroke::new(1.0, color),
                );
                fx += dash_len + gap_len;
            }
            // Entry lines (BUY/SELL) show size + average entry ("BUY 11155 @ 0.1058");
            // SL/TP lines just show the price level. Quantity is trimmed of trailing
            // zeros so whole-share lots read cleanly.
            let label = if pl.line_type == 0 {
                let qty_str = if pl.volume.fract().abs() < 1e-9 {
                    format!("{:.0}", pl.volume)
                } else {
                    format!("{:.8}", pl.volume)
                        .trim_end_matches('0')
                        .trim_end_matches('.')
                        .to_string()
                };
                format!("{} {} @ {:.4}", label_prefix, qty_str, pl.price)
            } else {
                format!("{} {:.4}", label_prefix, pl.price)
            };
            painter.text(
                egui::pos2(chart_rect.left() + 4.0, y - 10.0),
                egui::Align2::LEFT_TOP,
                &label,
                egui::FontId::monospace(9.0),
                color,
            );
        }
    }
    // Trade arrows (buy = green up-arrow, sell = red down-arrow).
    // PERF: markers are sorted by bar_idx (see build_trade_overlay). Binary-search
    // for the first in-range marker so we skip off-screen history in O(log N) instead
    // of scanning the full Vec every frame.
    // Arrows render per-fill (small triangles — not noisy). Labels are deferred
    // and collapsed by screen-pixel clustering so dense fill activity
    // (slightly different fill prices on the same bar) doesn't
    // bury the candles under overlapping text blocks. Previously each fill
    // rendered its own label and the chart became
    // unreadable at high trade density.
    struct PendingLabel {
        x: f32,
        y: f32,
        is_buy: bool,
        volume: f64,
        price: f64,
        ticker: String,
        count: u32,
    }
    let mut pending_labels: Vec<PendingLabel> = Vec::new();
    let marker_start = trade_overlay
        .markers
        .partition_point(|m| m.bar_idx < start_idx);
    for tm in trade_overlay.markers[marker_start..]
        .iter()
        .take_while(|m| m.bar_idx < end_idx)
    {
        let rel = tm.bar_idx - start_idx;
        let x = data_left + (rel as f32 + 0.5) * bar_w;
        let y = price_to_y(tm.price);
        if y < chart_rect.top() || y > chart_rect.bottom() {
            continue;
        }
        let (color, arrow_dir) = if tm.is_buy {
            (egui::Color32::from_rgb(76, 175, 80), 1.0_f32) // green, points up (below bar)
        } else {
            (egui::Color32::from_rgb(244, 67, 54), -1.0_f32) // red, points down (above bar)
        };
        let arrow_size = 6.0_f32;
        let y_offset = arrow_size * 2.0 * arrow_dir;
        let tip_y = y + y_offset;
        let base_y = tip_y + arrow_size * arrow_dir;
        let points = vec![
            egui::pos2(x, tip_y),
            egui::pos2(x - arrow_size * 0.6, base_y),
            egui::pos2(x + arrow_size * 0.6, base_y),
        ];
        painter.add(egui::Shape::convex_polygon(
            points,
            color,
            egui::Stroke::NONE,
        ));
        let label_y = if tm.is_buy {
            base_y + 2.0
        } else {
            base_y - 10.0
        };
        pending_labels.push(PendingLabel {
            x,
            y: label_y,
            is_buy: tm.is_buy,
            volume: tm.volume,
            price: tm.price,
            ticker: tm.ticker.clone(),
            count: tm.count,
        });
    }

    // Greedy pixel-proximity clustering per side. CLUSTER_X/Y roughly match the
    // bounding box of an 8pt monospace label so only markers that would
    // actually overlap get merged.
    pub(crate) const CLUSTER_X: f32 = 44.0;
    pub(crate) const CLUSTER_Y: f32 = 12.0;
    struct LabelCluster {
        x_sum: f32,
        y_sum: f32,
        n: u32,
        is_buy: bool,
        volume: f64,
        price_w_sum: f64,
        weight_sum: f64,
        tickers: Vec<String>,
        deals: u32,
    }
    pending_labels.sort_by(|a, b| {
        a.is_buy
            .cmp(&b.is_buy)
            .then(a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal))
            .then(a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal))
    });
    let mut clusters: Vec<LabelCluster> = Vec::new();
    'outer: for lbl in pending_labels {
        for c in clusters.iter_mut() {
            if c.is_buy != lbl.is_buy {
                continue;
            }
            let cx = c.x_sum / c.n as f32;
            let cy = c.y_sum / c.n as f32;
            if (cx - lbl.x).abs() < CLUSTER_X && (cy - lbl.y).abs() < CLUSTER_Y {
                let w = lbl.volume.max(1e-6);
                c.x_sum += lbl.x;
                c.y_sum += lbl.y;
                c.n += 1;
                c.volume += lbl.volume;
                c.price_w_sum += lbl.price * w;
                c.weight_sum += w;
                c.deals += lbl.count;
                for t in lbl.ticker.split(", ").filter(|t| !t.is_empty()) {
                    // O(1) dedup for tickers (was linear .any on small Vec)
                    let mut set: std::collections::HashSet<String> = c.tickers.iter().cloned().collect();
                    if set.insert(t.to_string()) {
                        c.tickers.push(t.to_string());
                    }
                }
                continue 'outer;
            }
        }
        let w = lbl.volume.max(1e-6);
        let mut tickers: Vec<String> = Vec::new();
        {
            // O(1) dedup for tickers (was linear .any on small Vec)
            let mut set: std::collections::HashSet<String> = std::collections::HashSet::new();
            for t in lbl.ticker.split(", ").filter(|t| !t.is_empty()) {
                if set.insert(t.to_string()) {
                    tickers.push(t.to_string());
                }
            }
        }
        clusters.push(LabelCluster {
            x_sum: lbl.x,
            y_sum: lbl.y,
            n: 1,
            is_buy: lbl.is_buy,
            volume: lbl.volume,
            price_w_sum: lbl.price * w,
            weight_sum: w,
            tickers,
            deals: lbl.count,
        });
    }
    for c in &clusters {
        let color = if c.is_buy {
            egui::Color32::from_rgb(76, 175, 80)
        } else {
            egui::Color32::from_rgb(244, 67, 54)
        };
        let x = c.x_sum / c.n as f32;
        let y = c.y_sum / c.n as f32;
        let avg_price = if c.weight_sum > 0.0 {
            c.price_w_sum / c.weight_sum
        } else {
            0.0
        };
        let label = if c.tickers.is_empty() {
            format!("{:.2}", c.volume)
        } else if c.n == 1 && c.tickers.len() == 1 {
            if c.deals > 1 || c.volume >= 0.1 {
                format!("{} {:.2}", c.tickers[0], c.volume)
            } else {
                c.tickers[0].clone()
            }
        } else {
            let head = if c.tickers.len() <= 3 {
                c.tickers.join(",")
            } else {
                format!("{}+{}", c.tickers[..2].join(","), c.tickers.len() - 2)
            };
            format!("[{}] @{:.2} {:.2}", head, avg_price, c.volume)
        };
        painter.text(
            egui::pos2(x, y),
            egui::Align2::CENTER_TOP,
            &label,
            egui::FontId::monospace(8.0),
            color,
        );
    }

    // ── alert price lines (extracted) ─────────────────────────────────────────
    draw_price_alert_lines(painter, chart_rect, price_to_y, alerts, format_price);

    // ── drawing annotations (draw_line extracted to chart_helpers) ───
    for (draw_idx, drawing) in chart.drawings.iter().enumerate() {
        // Per-drawing style: line width + style (with fallback defaults)
        let (d_width, d_style) = chart
            .drawing_styles
            .get(draw_idx)
            .copied()
            .unwrap_or((1.5, LineStyle::Solid));
        let is_selected = chart.selected_drawing == Some(draw_idx);
        // Selection: boost width and tint color slightly cyan
        let sel_boost = if is_selected { 1.5 } else { 0.0 };
        let effective_width = d_width + sel_boost;
        // Tint helper: if selected, blend color toward cyan for visibility
        let sel_tint = |c: egui::Color32| -> egui::Color32 {
            if !is_selected {
                return c;
            }
            egui::Color32::from_rgb(
                c.r().saturating_add(30),
                c.g().saturating_add(50),
                c.b().saturating_add(80),
            )
        };
        match drawing {
            Drawing::HLine { price, color } => {
                let y = price_to_y(*price);
                if y >= chart_rect.top() && y <= chart_rect.bottom() {
                    draw_styled_line(
                        &painter,
                        egui::pos2(chart_rect.left(), y),
                        egui::pos2(chart_rect.right(), y),
                        egui::Stroke::new(effective_width, sel_tint(*color)),
                        d_style,
                    );
                    painter.text(
                        egui::pos2(chart_rect.right() - 60.0, y - 10.0),
                        egui::Align2::LEFT_TOP,
                        &format_price(*price),
                        egui::FontId::monospace(9.0),
                        *color,
                    );
                }
            }
            Drawing::TrendLine { p1, p2, color } => {
                // Map bar indices to x positions
                let x1 = if p1.0 >= start_idx && p1.0 < end_idx {
                    Some(data_left + ((p1.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                let x2 = if p2.0 >= start_idx && p2.0 < end_idx {
                    Some(data_left + ((p2.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                if let (Some(x1), Some(x2)) = (x1, x2) {
                    let y1 = price_to_y(p1.1);
                    let y2 = price_to_y(p2.1);
                    draw_styled_line(
                        &painter,
                        egui::pos2(x1, y1),
                        egui::pos2(x2, y2),
                        egui::Stroke::new(effective_width, sel_tint(*color)),
                        d_style,
                    );
                }
            }
            Drawing::FiboRetrace {
                high,
                low,
                bar_start,
                bar_end,
            } => {
                let x_start = if *bar_start >= start_idx && *bar_start < end_idx {
                    data_left + ((*bar_start - start_idx) as f32 + 0.5) * bar_w
                } else {
                    chart_rect.left()
                };
                let x_end = if *bar_end >= start_idx && *bar_end < end_idx {
                    data_left + ((*bar_end - start_idx) as f32 + 0.5) * bar_w
                } else {
                    chart_rect.right()
                };
                let levels = [0.0, 0.236, 0.382, 0.5, 0.618, 0.786, 1.0];
                let range = high - low;
                for &level in &levels {
                    let price = high - range * level;
                    let y = price_to_y(price);
                    if y >= chart_rect.top() && y <= chart_rect.bottom() {
                        painter.line_segment(
                            [egui::pos2(x_start, y), egui::pos2(x_end, y)],
                            egui::Stroke::new(0.8, FIBO_COL),
                        );
                        painter.text(
                            egui::pos2(x_end + 2.0, y - 8.0),
                            egui::Align2::LEFT_TOP,
                            &format!("{:.1}% {}", level * 100.0, format_price(price)),
                            egui::FontId::monospace(8.0),
                            FIBO_COL,
                        );
                    }
                }
            }
            Drawing::VLine { bar_idx, color } => {
                if *bar_idx >= start_idx && *bar_idx < end_idx {
                    let x = data_left + ((*bar_idx - start_idx) as f32 + 0.5) * bar_w;
                    draw_styled_line(
                        &painter,
                        egui::pos2(x, chart_rect.top()),
                        egui::pos2(x, chart_rect.bottom()),
                        egui::Stroke::new(effective_width, sel_tint(*color)),
                        d_style,
                    );
                }
            }
            Drawing::Rectangle { p1, p2, color } => {
                let x1 = if p1.0 >= start_idx && p1.0 < end_idx {
                    Some(data_left + ((p1.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                let x2 = if p2.0 >= start_idx && p2.0 < end_idx {
                    Some(data_left + ((p2.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                if let (Some(x1), Some(x2)) = (x1, x2) {
                    let y1 = price_to_y(p1.1);
                    let y2 = price_to_y(p2.1);
                    let r = egui::Rect::from_two_pos(egui::pos2(x1, y1), egui::pos2(x2, y2));
                    painter.rect_filled(r, 0.0, *color);
                    painter.rect_stroke(
                        r,
                        0.0,
                        egui::Stroke::new(effective_width, sel_tint(*color)),
                        egui::StrokeKind::Outside,
                    );
                }
            }
            Drawing::Ray {
                origin,
                slope,
                color,
            } => {
                if origin.0 >= start_idx && origin.0 < end_idx {
                    let x1 = data_left + ((origin.0 - start_idx) as f32 + 0.5) * bar_w;
                    let y1 = price_to_y(origin.1);
                    let bars_to_edge = ((chart_rect.right() - x1) / bar_w) as f64;
                    let end_price = origin.1 + slope * bars_to_edge;
                    let y2 = price_to_y(end_price);
                    draw_styled_line(
                        &painter,
                        egui::pos2(x1, y1),
                        egui::pos2(chart_rect.right(), y2),
                        egui::Stroke::new(effective_width, sel_tint(*color)),
                        d_style,
                    );
                }
            }
            Drawing::Channel {
                p1,
                p2,
                width,
                color,
            } => {
                let x1 = if p1.0 >= start_idx && p1.0 < end_idx {
                    Some(data_left + ((p1.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                let x2 = if p2.0 >= start_idx && p2.0 < end_idx {
                    Some(data_left + ((p2.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                if let (Some(x1), Some(x2)) = (x1, x2) {
                    let y1 = price_to_y(p1.1);
                    let y2 = price_to_y(p2.1);
                    let y1b = price_to_y(p1.1 + width);
                    let y2b = price_to_y(p2.1 + width);
                    let sc = sel_tint(*color);
                    draw_styled_line(
                        &painter,
                        egui::pos2(x1, y1),
                        egui::pos2(x2, y2),
                        egui::Stroke::new(effective_width, sc),
                        d_style,
                    );
                    draw_styled_line(
                        &painter,
                        egui::pos2(x1, y1b),
                        egui::pos2(x2, y2b),
                        egui::Stroke::new(effective_width, sc),
                        d_style,
                    );
                    let fill =
                        egui::Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 20);
                    let poly = vec![
                        egui::pos2(x1, y1),
                        egui::pos2(x2, y2),
                        egui::pos2(x2, y2b),
                        egui::pos2(x1, y1b),
                    ];
                    painter.add(egui::Shape::convex_polygon(poly, fill, egui::Stroke::NONE));
                }
            }
            Drawing::ExtendedLine { p1, p2, color } => {
                // Extend line infinitely in both directions across visible chart
                if p1.0 != p2.0 {
                    let slope = (p2.1 - p1.1) / (p2.0 as f64 - p1.0 as f64);
                    let price_at_start = p1.1 + slope * (start_idx as f64 - p1.0 as f64);
                    let price_at_end = p1.1 + slope * (end_idx as f64 - p1.0 as f64);
                    let y1 = price_to_y(price_at_start);
                    let y2 = price_to_y(price_at_end);
                    draw_styled_line(
                        &painter,
                        egui::pos2(chart_rect.left(), y1),
                        egui::pos2(chart_rect.right(), y2),
                        egui::Stroke::new(effective_width, sel_tint(*color)),
                        d_style,
                    );
                }
            }
            Drawing::HRay {
                bar_idx,
                price,
                color,
            } => {
                let y = price_to_y(*price);
                let x_start = if *bar_idx >= start_idx && *bar_idx < end_idx {
                    data_left + ((*bar_idx - start_idx) as f32 + 0.5) * bar_w
                } else {
                    chart_rect.left()
                }; // bar left of view — draw full width
                draw_styled_line(
                    &painter,
                    egui::pos2(x_start, y),
                    egui::pos2(chart_rect.right(), y),
                    egui::Stroke::new(effective_width, sel_tint(*color)),
                    d_style,
                );
            }
            Drawing::CrossLine {
                bar_idx,
                price,
                color,
            } => {
                if *bar_idx >= start_idx && *bar_idx < end_idx {
                    let x = data_left + ((*bar_idx - start_idx) as f32 + 0.5) * bar_w;
                    let y = price_to_y(*price);
                    let sc = sel_tint(*color);
                    let sw = egui::Stroke::new(effective_width, sc);
                    draw_styled_line(
                        &painter,
                        egui::pos2(x, chart_rect.top()),
                        egui::pos2(x, chart_rect.bottom()),
                        sw,
                        d_style,
                    );
                    draw_styled_line(
                        &painter,
                        egui::pos2(chart_rect.left(), y),
                        egui::pos2(chart_rect.right(), y),
                        sw,
                        d_style,
                    );
                }
            }
            Drawing::ArrowLine { p1, p2, color } => {
                let x1 = if p1.0 >= start_idx && p1.0 < end_idx {
                    Some(data_left + ((p1.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                let x2 = if p2.0 >= start_idx && p2.0 < end_idx {
                    Some(data_left + ((p2.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                if let (Some(x1), Some(x2)) = (x1, x2) {
                    let y1 = price_to_y(p1.1);
                    let y2 = price_to_y(p2.1);
                    let sc = sel_tint(*color);
                    draw_styled_line(
                        &painter,
                        egui::pos2(x1, y1),
                        egui::pos2(x2, y2),
                        egui::Stroke::new(effective_width, sc),
                        d_style,
                    );
                    // Arrowhead at p2
                    let dx = x2 - x1;
                    let dy = y2 - y1;
                    let len = (dx * dx + dy * dy).sqrt().max(1.0);
                    let ux = dx / len;
                    let uy = dy / len;
                    let sz = 8.0_f32;
                    let ax = x2 - ux * sz + uy * sz * 0.4;
                    let ay = y2 - uy * sz - ux * sz * 0.4;
                    let bx = x2 - ux * sz - uy * sz * 0.4;
                    let by = y2 - uy * sz + ux * sz * 0.4;
                    painter.add(egui::Shape::convex_polygon(
                        vec![egui::pos2(x2, y2), egui::pos2(ax, ay), egui::pos2(bx, by)],
                        sc,
                        egui::Stroke::NONE,
                    ));
                }
            }
            Drawing::InfoLine { p1, p2, color } => {
                let x1 = if p1.0 >= start_idx && p1.0 < end_idx {
                    Some(data_left + ((p1.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                let x2 = if p2.0 >= start_idx && p2.0 < end_idx {
                    Some(data_left + ((p2.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                if let (Some(x1), Some(x2)) = (x1, x2) {
                    let y1 = price_to_y(p1.1);
                    let y2 = price_to_y(p2.1);
                    draw_styled_line(
                        &painter,
                        egui::pos2(x1, y1),
                        egui::pos2(x2, y2),
                        egui::Stroke::new(effective_width, sel_tint(*color)),
                        d_style,
                    );
                    // Info label: distance, percent, bars
                    let dist = p2.1 - p1.1;
                    let pct = if p1.1.abs() > f64::EPSILON {
                        dist / p1.1 * 100.0
                    } else {
                        0.0
                    };
                    let bar_count = if p2.0 > p1.0 {
                        p2.0 - p1.0
                    } else {
                        p1.0 - p2.0
                    };
                    let label = format!("{:.2} ({:+.2}%) {} bars", dist, pct, bar_count);
                    let mid_x = (x1 + x2) / 2.0;
                    let mid_y = (y1 + y2) / 2.0 - 12.0;
                    painter.text(
                        egui::pos2(mid_x, mid_y),
                        egui::Align2::CENTER_BOTTOM,
                        &label,
                        egui::FontId::monospace(10.0),
                        *color,
                    );
                }
            }
            Drawing::Pitchfork {
                pivot,
                p2,
                p3,
                color,
            } => {
                // Andrews Pitchfork: median line from pivot to midpoint(p2,p3), parallel upper/lower
                let to_x = |idx: usize| -> Option<f32> {
                    if idx >= start_idx && idx < end_idx {
                        Some(data_left + ((idx - start_idx) as f32 + 0.5) * bar_w)
                    } else {
                        None
                    }
                };
                if let (Some(xp), Some(x2), Some(x3)) = (to_x(pivot.0), to_x(p2.0), to_x(p3.0)) {
                    let yp = price_to_y(pivot.1);
                    let y2 = price_to_y(p2.1);
                    let y3 = price_to_y(p3.1);
                    let mid_x = (x2 + x3) / 2.0;
                    let mid_y = (y2 + y3) / 2.0;
                    // Median line (extended to chart edge)
                    let dx = mid_x - xp;
                    let dy = mid_y - yp;
                    let ext = if dx.abs() > 0.1 {
                        (chart_rect.right() - xp) / dx
                    } else {
                        1.0
                    };
                    let end_x = xp + dx * ext;
                    let end_y = yp + dy * ext;
                    let sc = sel_tint(*color);
                    let sw = egui::Stroke::new(effective_width, sc);
                    draw_styled_line(
                        &painter,
                        egui::pos2(xp, yp),
                        egui::pos2(end_x, end_y),
                        sw,
                        d_style,
                    );
                    // Upper line (through p2, parallel to median)
                    let ux = x2 + dx * ext;
                    let uy = y2 + dy * ext;
                    draw_styled_line(
                        &painter,
                        egui::pos2(x2, y2),
                        egui::pos2(ux.min(chart_rect.right()), uy),
                        sw,
                        d_style,
                    );
                    // Lower line (through p3, parallel to median)
                    let lx = x3 + dx * ext;
                    let ly = y3 + dy * ext;
                    draw_styled_line(
                        &painter,
                        egui::pos2(x3, y3),
                        egui::pos2(lx.min(chart_rect.right()), ly),
                        sw,
                        d_style,
                    );
                }
            }
            Drawing::FiboExtension { p1, p2, p3, color } => {
                let to_x = |idx: usize| -> Option<f32> {
                    if idx >= start_idx && idx < end_idx {
                        Some(data_left + ((idx - start_idx) as f32 + 0.5) * bar_w)
                    } else {
                        None
                    }
                };
                if let Some(x3) = to_x(p3.0) {
                    let range = (p2.1 - p1.1).abs();
                    let base = p3.1;
                    let dir = if p2.1 > p1.1 { 1.0 } else { -1.0 };
                    let levels = [0.0, 0.618, 1.0, 1.272, 1.618, 2.0, 2.618];
                    let names = ["0%", "61.8%", "100%", "127.2%", "161.8%", "200%", "261.8%"];
                    let sc = sel_tint(*color);
                    for (i, &lvl) in levels.iter().enumerate() {
                        let price = base + dir * range * lvl;
                        let y = price_to_y(price);
                        if y >= chart_rect.top() && y <= chart_rect.bottom() {
                            let alpha = if lvl == 1.0 || lvl == 1.618 { 180 } else { 100 };
                            let c = egui::Color32::from_rgba_premultiplied(
                                sc.r(),
                                sc.g(),
                                sc.b(),
                                alpha,
                            );
                            let lw = if lvl == 1.0 || lvl == 1.618 {
                                effective_width
                            } else {
                                effective_width * 0.65
                            };
                            draw_styled_line(
                                &painter,
                                egui::pos2(x3, y),
                                egui::pos2(chart_rect.right(), y),
                                egui::Stroke::new(lw, c),
                                d_style,
                            );
                            painter.text(
                                egui::pos2(chart_rect.right() - 60.0, y - 10.0),
                                egui::Align2::LEFT_BOTTOM,
                                names[i],
                                egui::FontId::monospace(9.0),
                                c,
                            );
                        }
                    }
                }
            }
            Drawing::GannFan {
                origin,
                scale,
                color,
            } => {
                if origin.0 >= start_idx && origin.0 < end_idx {
                    let ox = data_left + ((origin.0 - start_idx) as f32 + 0.5) * bar_w;
                    let oy = price_to_y(origin.1);
                    // Gann angles: 1×8, 1×4, 1×3, 1×2, 1×1, 2×1, 3×1, 4×1, 8×1
                    let ratios: &[(f64, &str)] = &[
                        (0.125, "1×8"),
                        (0.25, "1×4"),
                        (0.333, "1×3"),
                        (0.5, "1×2"),
                        (1.0, "1×1"),
                        (2.0, "2×1"),
                        (3.0, "3×1"),
                        (4.0, "4×1"),
                        (8.0, "8×1"),
                    ];
                    let sc = sel_tint(*color);
                    for &(ratio, label) in ratios {
                        let bars_to_edge = ((chart_rect.right() - ox) / bar_w) as f64;
                        let end_price = origin.1 + scale * ratio * bars_to_edge;
                        let end_y = price_to_y(end_price);
                        let alpha = if ratio == 1.0 { 200 } else { 100 };
                        let c =
                            egui::Color32::from_rgba_premultiplied(sc.r(), sc.g(), sc.b(), alpha);
                        let w = if ratio == 1.0 {
                            effective_width
                        } else {
                            effective_width * 0.55
                        };
                        draw_styled_line(
                            &painter,
                            egui::pos2(ox, oy),
                            egui::pos2(chart_rect.right(), end_y),
                            egui::Stroke::new(w, c),
                            d_style,
                        );
                        painter.text(
                            egui::pos2(chart_rect.right() - 2.0, end_y),
                            egui::Align2::RIGHT_CENTER,
                            label,
                            egui::FontId::monospace(8.0),
                            c,
                        );
                        // Downward mirror
                        let dn_price = origin.1 - scale * ratio * bars_to_edge;
                        let dn_y = price_to_y(dn_price);
                        draw_styled_line(
                            &painter,
                            egui::pos2(ox, oy),
                            egui::pos2(chart_rect.right(), dn_y),
                            egui::Stroke::new(w, c),
                            d_style,
                        );
                    }
                }
            }
            Drawing::LongPosition {
                entry,
                stop,
                target,
            } => {
                if entry.0 >= start_idx && entry.0 < end_idx {
                    let x = data_left + ((entry.0 - start_idx) as f32 + 0.5) * bar_w;
                    let ye = price_to_y(entry.1);
                    let ys = price_to_y(*stop);
                    let yt = price_to_y(*target);
                    let w = (chart_rect.right() - x).min(200.0);
                    // Stop zone (red)
                    painter.rect_filled(
                        egui::Rect::from_min_max(egui::pos2(x, ye), egui::pos2(x + w, ys)),
                        0.0,
                        egui::Color32::from_rgba_premultiplied(220, 40, 40, 30),
                    );
                    // Target zone (green)
                    painter.rect_filled(
                        egui::Rect::from_min_max(egui::pos2(x, yt), egui::pos2(x + w, ye)),
                        0.0,
                        egui::Color32::from_rgba_premultiplied(0, 200, 80, 30),
                    );
                    // Entry line
                    painter.line_segment(
                        [egui::pos2(x, ye), egui::pos2(x + w, ye)],
                        egui::Stroke::new(1.5, egui::Color32::WHITE),
                    );
                    // R:R label
                    let risk = (entry.1 - stop).abs();
                    let reward = (target - entry.1).abs();
                    let rr = if risk > f64::EPSILON {
                        reward / risk
                    } else {
                        0.0
                    };
                    painter.text(
                        egui::pos2(x + w + 4.0, ye),
                        egui::Align2::LEFT_CENTER,
                        &format!("R:R {:.1}", rr),
                        egui::FontId::monospace(10.0),
                        egui::Color32::WHITE,
                    );
                }
            }
            Drawing::ShortPosition {
                entry,
                stop,
                target,
            } => {
                if entry.0 >= start_idx && entry.0 < end_idx {
                    let x = data_left + ((entry.0 - start_idx) as f32 + 0.5) * bar_w;
                    let ye = price_to_y(entry.1);
                    let ys = price_to_y(*stop);
                    let yt = price_to_y(*target);
                    let w = (chart_rect.right() - x).min(200.0);
                    // Stop zone (red, above entry for short)
                    painter.rect_filled(
                        egui::Rect::from_min_max(egui::pos2(x, ys), egui::pos2(x + w, ye)),
                        0.0,
                        egui::Color32::from_rgba_premultiplied(220, 40, 40, 30),
                    );
                    // Target zone (green, below entry for short)
                    painter.rect_filled(
                        egui::Rect::from_min_max(egui::pos2(x, ye), egui::pos2(x + w, yt)),
                        0.0,
                        egui::Color32::from_rgba_premultiplied(0, 200, 80, 30),
                    );
                    painter.line_segment(
                        [egui::pos2(x, ye), egui::pos2(x + w, ye)],
                        egui::Stroke::new(1.5, egui::Color32::WHITE),
                    );
                    let risk = (stop - entry.1).abs();
                    let reward = (entry.1 - target).abs();
                    let rr = if risk > f64::EPSILON {
                        reward / risk
                    } else {
                        0.0
                    };
                    painter.text(
                        egui::pos2(x + w + 4.0, ye),
                        egui::Align2::LEFT_CENTER,
                        &format!("R:R {:.1}", rr),
                        egui::FontId::monospace(10.0),
                        egui::Color32::WHITE,
                    );
                }
            }
            Drawing::PriceRange { p1, p2 } => {
                let x1 = if p1.0 >= start_idx && p1.0 < end_idx {
                    Some(data_left + ((p1.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                let x2 = if p2.0 >= start_idx && p2.0 < end_idx {
                    Some(data_left + ((p2.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                if let (Some(x1), Some(x2)) = (x1, x2) {
                    let y1 = price_to_y(p1.1);
                    let y2 = price_to_y(p2.1);
                    let fill = egui::Color32::from_rgba_premultiplied(100, 150, 255, 20);
                    painter.rect_filled(
                        egui::Rect::from_two_pos(egui::pos2(x1, y1), egui::pos2(x2, y2)),
                        0.0,
                        fill,
                    );
                    let dist = p2.1 - p1.1;
                    let pct = if p1.1.abs() > f64::EPSILON {
                        dist / p1.1 * 100.0
                    } else {
                        0.0
                    };
                    let bars = if p2.0 > p1.0 {
                        p2.0 - p1.0
                    } else {
                        p1.0 - p2.0
                    };
                    let label = format!("{:.2} ({:+.2}%) {} bars", dist, pct, bars);
                    painter.text(
                        egui::pos2((x1 + x2) / 2.0, y1.min(y2) - 4.0),
                        egui::Align2::CENTER_BOTTOM,
                        &label,
                        egui::FontId::monospace(10.0),
                        egui::Color32::from_rgb(100, 150, 255),
                    );
                }
            }
            Drawing::TextLabel {
                bar_idx,
                price,
                text,
                color,
            } => {
                if *bar_idx >= start_idx && *bar_idx < end_idx {
                    let x = data_left + ((*bar_idx - start_idx) as f32 + 0.5) * bar_w;
                    let y = price_to_y(*price);
                    painter.text(
                        egui::pos2(x, y),
                        egui::Align2::CENTER_CENTER,
                        text,
                        egui::FontId::monospace(11.0),
                        *color,
                    );
                }
            }
            Drawing::ArrowMarker {
                bar_idx,
                price,
                is_up,
                color,
            } => {
                if *bar_idx >= start_idx && *bar_idx < end_idx {
                    let x = data_left + ((*bar_idx - start_idx) as f32 + 0.5) * bar_w;
                    let y = price_to_y(*price);
                    let sz = 8.0_f32;
                    if *is_up {
                        let pts = vec![
                            egui::pos2(x, y - sz),
                            egui::pos2(x - sz * 0.6, y + sz * 0.3),
                            egui::pos2(x + sz * 0.6, y + sz * 0.3),
                        ];
                        painter.add(egui::Shape::convex_polygon(pts, *color, egui::Stroke::NONE));
                    } else {
                        let pts = vec![
                            egui::pos2(x, y + sz),
                            egui::pos2(x - sz * 0.6, y - sz * 0.3),
                            egui::pos2(x + sz * 0.6, y - sz * 0.3),
                        ];
                        painter.add(egui::Shape::convex_polygon(pts, *color, egui::Stroke::NONE));
                    }
                }
            }
            Drawing::Ellipse { p1, p2, color } => {
                let x1 = if p1.0 >= start_idx && p1.0 < end_idx {
                    Some(data_left + ((p1.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                let x2 = if p2.0 >= start_idx && p2.0 < end_idx {
                    Some(data_left + ((p2.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                if let (Some(x1), Some(x2)) = (x1, x2) {
                    let y1 = price_to_y(p1.1);
                    let y2 = price_to_y(p2.1);
                    let cx = (x1 + x2) / 2.0;
                    let cy = (y1 + y2) / 2.0;
                    let rx = (x2 - x1).abs() / 2.0;
                    let ry = (y2 - y1).abs() / 2.0;
                    let n_pts = 48;
                    let pts: Vec<egui::Pos2> = (0..n_pts)
                        .map(|i| {
                            let a = 2.0 * std::f32::consts::PI * i as f32 / n_pts as f32;
                            egui::pos2(cx + rx * a.cos(), cy + ry * a.sin())
                        })
                        .collect();
                    let sc = sel_tint(*color);
                    let fill = egui::Color32::from_rgba_premultiplied(sc.r(), sc.g(), sc.b(), 20);
                    painter.add(egui::Shape::convex_polygon(
                        pts,
                        fill,
                        egui::Stroke::new(effective_width, sc),
                    ));
                }
            }
            Drawing::Triangle { p1, p2, p3, color } => {
                let to_pt = |idx: usize, price: f64| -> Option<egui::Pos2> {
                    if idx >= start_idx && idx < end_idx {
                        let x = data_left + ((idx - start_idx) as f32 + 0.5) * bar_w;
                        Some(egui::pos2(x, price_to_y(price)))
                    } else {
                        None
                    }
                };
                if let (Some(a), Some(b), Some(c)) =
                    (to_pt(p1.0, p1.1), to_pt(p2.0, p2.1), to_pt(p3.0, p3.1))
                {
                    let sc = sel_tint(*color);
                    let fill = egui::Color32::from_rgba_premultiplied(sc.r(), sc.g(), sc.b(), 20);
                    painter.add(egui::Shape::convex_polygon(
                        vec![a, b, c],
                        fill,
                        egui::Stroke::new(effective_width, sc),
                    ));
                }
            }
            Drawing::TrendAngle { p1, p2, color } => {
                let x1 = if p1.0 >= start_idx && p1.0 < end_idx {
                    Some(data_left + ((p1.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                let x2 = if p2.0 >= start_idx && p2.0 < end_idx {
                    Some(data_left + ((p2.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                if let (Some(x1), Some(x2)) = (x1, x2) {
                    let y1 = price_to_y(p1.1);
                    let y2 = price_to_y(p2.1);
                    draw_styled_line(
                        &painter,
                        egui::pos2(x1, y1),
                        egui::pos2(x2, y2),
                        egui::Stroke::new(effective_width, sel_tint(*color)),
                        d_style,
                    );
                    // Angle display
                    let dx = x2 - x1;
                    let dy = y2 - y1;
                    let angle_deg = (dy / dx).atan().to_degrees();
                    painter.text(
                        egui::pos2((x1 + x2) / 2.0, (y1 + y2) / 2.0 - 12.0),
                        egui::Align2::CENTER_BOTTOM,
                        &format!("{:.1}°", angle_deg),
                        egui::FontId::monospace(10.0),
                        sel_tint(*color),
                    );
                }
            }
            Drawing::ParallelChannel {
                p1,
                p2,
                offset,
                color,
            } => {
                let x1 = if p1.0 >= start_idx && p1.0 < end_idx {
                    Some(data_left + ((p1.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                let x2 = if p2.0 >= start_idx && p2.0 < end_idx {
                    Some(data_left + ((p2.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                if let (Some(x1), Some(x2)) = (x1, x2) {
                    let y1 = price_to_y(p1.1);
                    let y2 = price_to_y(p2.1);
                    let y1u = price_to_y(p1.1 + offset);
                    let y2u = price_to_y(p2.1 + offset);
                    let y1d = price_to_y(p1.1 - offset);
                    let y2d = price_to_y(p2.1 - offset);
                    let sc = sel_tint(*color);
                    // Center line (dashed-style: thinner)
                    draw_styled_line(
                        &painter,
                        egui::pos2(x1, y1),
                        egui::pos2(x2, y2),
                        egui::Stroke::new(effective_width * 0.5, sc),
                        d_style,
                    );
                    // Upper boundary
                    draw_styled_line(
                        &painter,
                        egui::pos2(x1, y1u),
                        egui::pos2(x2, y2u),
                        egui::Stroke::new(effective_width, sc),
                        d_style,
                    );
                    // Lower boundary
                    draw_styled_line(
                        &painter,
                        egui::pos2(x1, y1d),
                        egui::pos2(x2, y2d),
                        egui::Stroke::new(effective_width, sc),
                        d_style,
                    );
                    // Fill between upper and lower
                    let fill =
                        egui::Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 15);
                    let poly = vec![
                        egui::pos2(x1, y1u),
                        egui::pos2(x2, y2u),
                        egui::pos2(x2, y2d),
                        egui::pos2(x1, y1d),
                    ];
                    painter.add(egui::Shape::convex_polygon(poly, fill, egui::Stroke::NONE));
                }
            }
            Drawing::FibChannel { p1, p2, p3, color } => {
                let to_x = |idx: usize| -> Option<f32> {
                    if idx >= start_idx && idx < end_idx {
                        Some(data_left + ((idx - start_idx) as f32 + 0.5) * bar_w)
                    } else {
                        None
                    }
                };
                if let (Some(x1), Some(x2)) = (to_x(p1.0), to_x(p2.0)) {
                    // Channel width from p3 offset perpendicular to the trendline
                    let ch_offset = p3.1 - p1.1; // price offset defining full channel width
                    let levels = [0.0, 0.236, 0.382, 0.5, 0.618, 0.786, 1.0];
                    let names = ["0%", "23.6%", "38.2%", "50%", "61.8%", "78.6%", "100%"];
                    let sc = sel_tint(*color);
                    for (i, &lvl) in levels.iter().enumerate() {
                        let off = ch_offset * lvl;
                        let ly1 = price_to_y(p1.1 + off);
                        let ly2 = price_to_y(p2.1 + off);
                        let alpha = if lvl == 0.0 || lvl == 0.5 || lvl == 1.0 {
                            180
                        } else {
                            100
                        };
                        let c =
                            egui::Color32::from_rgba_premultiplied(sc.r(), sc.g(), sc.b(), alpha);
                        let w = if lvl == 0.0 || lvl == 1.0 {
                            effective_width
                        } else {
                            effective_width * 0.55
                        };
                        draw_styled_line(
                            &painter,
                            egui::pos2(x1, ly1),
                            egui::pos2(x2, ly2),
                            egui::Stroke::new(w, c),
                            d_style,
                        );
                        painter.text(
                            egui::pos2(x2 + 4.0, ly2),
                            egui::Align2::LEFT_CENTER,
                            names[i],
                            egui::FontId::monospace(8.0),
                            c,
                        );
                    }
                    // Fill 0-100%
                    let fill = egui::Color32::from_rgba_premultiplied(sc.r(), sc.g(), sc.b(), 10);
                    let poly = vec![
                        egui::pos2(x1, price_to_y(p1.1)),
                        egui::pos2(x2, price_to_y(p2.1)),
                        egui::pos2(x2, price_to_y(p2.1 + ch_offset)),
                        egui::pos2(x1, price_to_y(p1.1 + ch_offset)),
                    ];
                    painter.add(egui::Shape::convex_polygon(poly, fill, egui::Stroke::NONE));
                }
            }
            Drawing::FibTimeZones { bar_idx, color } => {
                // Draw vertical lines at Fibonacci intervals: 1, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144
                let fibs = [1usize, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144, 233];
                let mut cumulative = 0usize;
                for &f in &fibs {
                    cumulative += f;
                    let idx = bar_idx + cumulative;
                    if idx >= start_idx && idx < end_idx {
                        let x = data_left + ((idx - start_idx) as f32 + 0.5) * bar_w;
                        let alpha = if f <= 3 { 120 } else { 80 };
                        let sc = sel_tint(*color);
                        let c =
                            egui::Color32::from_rgba_premultiplied(sc.r(), sc.g(), sc.b(), alpha);
                        draw_styled_line(
                            &painter,
                            egui::pos2(x, chart_rect.top()),
                            egui::pos2(x, chart_rect.bottom()),
                            egui::Stroke::new(effective_width * 0.65, c),
                            d_style,
                        );
                        painter.text(
                            egui::pos2(x + 2.0, chart_rect.top() + 2.0),
                            egui::Align2::LEFT_TOP,
                            &format!("{}", cumulative),
                            egui::FontId::monospace(8.0),
                            c,
                        );
                    }
                }
            }
            Drawing::PriceLabel {
                bar_idx,
                price,
                color,
            } => {
                let y = price_to_y(*price);
                if y >= chart_rect.top() && y <= chart_rect.bottom() {
                    // Horizontal line from bar to right edge
                    let x_start = if *bar_idx >= start_idx && *bar_idx < end_idx {
                        data_left + ((*bar_idx - start_idx) as f32 + 0.5) * bar_w
                    } else if *bar_idx < start_idx {
                        chart_rect.left()
                    } else {
                        return; // bar beyond visible range
                    };
                    let sc = sel_tint(*color);
                    draw_styled_line(
                        &painter,
                        egui::pos2(x_start, y),
                        egui::pos2(chart_rect.right(), y),
                        egui::Stroke::new(effective_width, sc),
                        d_style,
                    );
                    // Price badge on the right
                    let label = format!("{:.5}", price);
                    let badge_w = 65.0_f32;
                    let badge_h = 14.0_f32;
                    let badge_rect = egui::Rect::from_min_size(
                        egui::pos2(chart_rect.right() - badge_w, y - badge_h / 2.0),
                        egui::vec2(badge_w, badge_h),
                    );
                    painter.rect_filled(badge_rect, 2.0, *color);
                    let text_col = if (color.r() as u16 + color.g() as u16 + color.b() as u16) > 384
                    {
                        egui::Color32::BLACK
                    } else {
                        egui::Color32::WHITE
                    };
                    painter.text(
                        badge_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        &label,
                        egui::FontId::monospace(9.0),
                        text_col,
                    );
                }
            }
            Drawing::Callout {
                anchor,
                label_pos,
                text,
                color,
            } => {
                let to_x = |idx: usize| -> Option<f32> {
                    if idx >= start_idx && idx < end_idx {
                        Some(data_left + ((idx - start_idx) as f32 + 0.5) * bar_w)
                    } else {
                        None
                    }
                };
                if let (Some(ax), Some(lx)) = (to_x(anchor.0), to_x(label_pos.0)) {
                    let ay = price_to_y(anchor.1);
                    let ly = price_to_y(label_pos.1);
                    // Arrow line from label to anchor
                    painter.line_segment(
                        [egui::pos2(lx, ly), egui::pos2(ax, ay)],
                        egui::Stroke::new(1.0, *color),
                    );
                    // Arrowhead at anchor
                    let dx = ax - lx;
                    let dy = ay - ly;
                    let len = (dx * dx + dy * dy).sqrt().max(1.0);
                    let ux = dx / len;
                    let uy = dy / len;
                    let sz = 6.0_f32;
                    let a1 = egui::pos2(ax - ux * sz + uy * sz * 0.4, ay - uy * sz - ux * sz * 0.4);
                    let a2 = egui::pos2(ax - ux * sz - uy * sz * 0.4, ay - uy * sz + ux * sz * 0.4);
                    painter.add(egui::Shape::convex_polygon(
                        vec![egui::pos2(ax, ay), a1, a2],
                        *color,
                        egui::Stroke::NONE,
                    ));
                    // Text box at label_pos
                    let pad = 4.0_f32;
                    let galley =
                        painter.layout_no_wrap(text.clone(), egui::FontId::monospace(10.0), *color);
                    let tw = galley.rect.width();
                    let th = galley.rect.height();
                    let box_rect = egui::Rect::from_min_size(
                        egui::pos2(lx - tw / 2.0 - pad, ly - th / 2.0 - pad),
                        egui::vec2(tw + pad * 2.0, th + pad * 2.0),
                    );
                    let bg = egui::Color32::from_rgba_premultiplied(20, 20, 30, 220);
                    painter.rect_filled(box_rect, 3.0, bg);
                    painter.rect_stroke(
                        box_rect,
                        3.0,
                        egui::Stroke::new(1.0, *color),
                        egui::StrokeKind::Outside,
                    );
                    painter.galley(egui::pos2(lx - tw / 2.0, ly - th / 2.0), galley, *color);
                }
            }
            Drawing::Highlighter { p1, p2, color } => {
                let x1 = if p1.0 >= start_idx && p1.0 < end_idx {
                    Some(data_left + ((p1.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                let x2 = if p2.0 >= start_idx && p2.0 < end_idx {
                    Some(data_left + ((p2.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                if let (Some(x1), Some(x2)) = (x1, x2) {
                    let y1 = price_to_y(p1.1);
                    let y2 = price_to_y(p2.1);
                    let sc = sel_tint(*color);
                    let fill = egui::Color32::from_rgba_premultiplied(sc.r(), sc.g(), sc.b(), 40);
                    painter.rect_filled(
                        egui::Rect::from_two_pos(egui::pos2(x1, y1), egui::pos2(x2, y2)),
                        0.0,
                        fill,
                    );
                    // Border
                    painter.rect_stroke(
                        egui::Rect::from_two_pos(egui::pos2(x1, y1), egui::pos2(x2, y2)),
                        0.0,
                        egui::Stroke::new(effective_width, sc),
                        egui::StrokeKind::Outside,
                    );
                }
            }
            Drawing::CrossMarker {
                bar_idx,
                price,
                color,
            } => {
                if *bar_idx >= start_idx && *bar_idx < end_idx {
                    let x = data_left + ((*bar_idx - start_idx) as f32 + 0.5) * bar_w;
                    let y = price_to_y(*price);
                    let sz = 6.0_f32;
                    let sc = sel_tint(*color);
                    let sw = egui::Stroke::new(effective_width, sc);
                    // + shape
                    draw_styled_line(
                        &painter,
                        egui::pos2(x - sz, y),
                        egui::pos2(x + sz, y),
                        sw,
                        d_style,
                    );
                    draw_styled_line(
                        &painter,
                        egui::pos2(x, y - sz),
                        egui::pos2(x, y + sz),
                        sw,
                        d_style,
                    );
                }
            }
            Drawing::Polyline { points, color } => {
                let mut screen_pts: Vec<egui::Pos2> = Vec::with_capacity(points.len());
                for &(idx, price) in points.iter() {
                    if idx >= start_idx && idx < end_idx {
                        let x = data_left + ((idx - start_idx) as f32 + 0.5) * bar_w;
                        screen_pts.push(egui::pos2(x, price_to_y(price)));
                    }
                }
                if screen_pts.len() > 1 {
                    painter.add(egui::Shape::line(
                        screen_pts,
                        egui::Stroke::new(effective_width, sel_tint(*color)),
                    ));
                }
            }
            Drawing::AnchorNote {
                bar_idx,
                price,
                text,
                color,
            } => {
                if *bar_idx >= start_idx && *bar_idx < end_idx {
                    let x = data_left + ((*bar_idx - start_idx) as f32 + 0.5) * bar_w;
                    let y = price_to_y(*price);
                    let pad = 4.0_f32;
                    let galley =
                        painter.layout_no_wrap(text.clone(), egui::FontId::monospace(10.0), *color);
                    let tw = galley.rect.width();
                    let th = galley.rect.height();
                    let box_rect = egui::Rect::from_min_size(
                        egui::pos2(x - pad, y - th - pad * 2.0),
                        egui::vec2(tw + pad * 2.0, th + pad * 2.0),
                    );
                    let bg = egui::Color32::from_rgba_premultiplied(15, 15, 25, 230);
                    painter.rect_filled(box_rect, 3.0, bg);
                    painter.rect_stroke(
                        box_rect,
                        3.0,
                        egui::Stroke::new(1.0, *color),
                        egui::StrokeKind::Outside,
                    );
                    painter.galley(egui::pos2(x, y - th - pad), galley, *color);
                    // Small triangle pointer down to the anchor point
                    let tri = vec![
                        egui::pos2(x + 4.0, y - pad),
                        egui::pos2(x + 10.0, y - pad),
                        egui::pos2(x + 7.0, y),
                    ];
                    painter.add(egui::Shape::convex_polygon(tri, *color, egui::Stroke::NONE));
                }
            }
            Drawing::RegressionChannel { p1, p2, color } => {
                // Linear regression of close prices between p1 and p2 bars
                let b1 = p1.0.min(p2.0);
                let b2 = p1.0.max(p2.0);
                if b2 > b1 && b1 < end_idx && b2 >= start_idx {
                    // Compute regression from bar data
                    let n = (b2 - b1 + 1) as f64;
                    let mut sum_x = 0.0_f64;
                    let mut sum_y = 0.0_f64;
                    let mut sum_xy = 0.0_f64;
                    let mut sum_xx = 0.0_f64;
                    let mut count = 0u32;
                    for idx in b1..=b2 {
                        if idx < bars.len() {
                            let xi = (idx - b1) as f64;
                            let yi = bars[idx].close;
                            sum_x += xi;
                            sum_y += yi;
                            sum_xy += xi * yi;
                            sum_xx += xi * xi;
                            count += 1;
                        }
                    }
                    if count > 1 {
                        let cn = count as f64;
                        let slope = (cn * sum_xy - sum_x * sum_y) / (cn * sum_xx - sum_x * sum_x);
                        let intercept = (sum_y - slope * sum_x) / cn;
                        // Standard deviation from regression line
                        let mut sum_sq = 0.0_f64;
                        for idx in b1..=b2 {
                            if idx < bars.len() {
                                let xi = (idx - b1) as f64;
                                let predicted = intercept + slope * xi;
                                let diff = bars[idx].close - predicted;
                                sum_sq += diff * diff;
                            }
                        }
                        let std_dev = (sum_sq / cn).sqrt();
                        // Draw regression line + 1 StdDev bands
                        let x_start = if b1 >= start_idx && b1 < end_idx {
                            data_left + ((b1 - start_idx) as f32 + 0.5) * bar_w
                        } else {
                            chart_rect.left()
                        };
                        let x_end = if b2 >= start_idx && b2 < end_idx {
                            data_left + ((b2 - start_idx) as f32 + 0.5) * bar_w
                        } else {
                            chart_rect.right()
                        };
                        let reg_y1 = price_to_y(intercept);
                        let reg_y2 = price_to_y(intercept + slope * n);
                        let sc = sel_tint(*color);
                        // Center line
                        draw_styled_line(
                            &painter,
                            egui::pos2(x_start, reg_y1),
                            egui::pos2(x_end, reg_y2),
                            egui::Stroke::new(effective_width, sc),
                            d_style,
                        );
                        // Upper band (+1 StdDev)
                        let uy1 = price_to_y(intercept + std_dev);
                        let uy2 = price_to_y(intercept + slope * n + std_dev);
                        draw_styled_line(
                            &painter,
                            egui::pos2(x_start, uy1),
                            egui::pos2(x_end, uy2),
                            egui::Stroke::new(effective_width * 0.55, sc),
                            d_style,
                        );
                        // Lower band (-1 StdDev)
                        let dy1 = price_to_y(intercept - std_dev);
                        let dy2 = price_to_y(intercept + slope * n - std_dev);
                        draw_styled_line(
                            &painter,
                            egui::pos2(x_start, dy1),
                            egui::pos2(x_end, dy2),
                            egui::Stroke::new(effective_width * 0.55, sc),
                            d_style,
                        );
                        // Fill between bands
                        let fill = egui::Color32::from_rgba_premultiplied(
                            color.r(),
                            color.g(),
                            color.b(),
                            15,
                        );
                        let poly = vec![
                            egui::pos2(x_start, uy1),
                            egui::pos2(x_end, uy2),
                            egui::pos2(x_end, dy2),
                            egui::pos2(x_start, dy1),
                        ];
                        painter.add(egui::Shape::convex_polygon(poly, fill, egui::Stroke::NONE));
                    }
                }
            }
            Drawing::GannBox { p1, p2, color } => {
                let x1o = if p1.0 >= start_idx && p1.0 < end_idx {
                    Some(data_left + ((p1.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                let x2o = if p2.0 >= start_idx && p2.0 < end_idx {
                    Some(data_left + ((p2.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                if let (Some(x1), Some(x2)) = (x1o, x2o) {
                    let y1 = price_to_y(p1.1);
                    let y2 = price_to_y(p2.1);
                    let rect_d = egui::Rect::from_two_pos(egui::pos2(x1, y1), egui::pos2(x2, y2));
                    let fill =
                        egui::Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 12);
                    painter.rect_filled(rect_d, 0.0, fill);
                    painter.rect_stroke(
                        rect_d,
                        0.0,
                        egui::Stroke::new(1.0, *color),
                        egui::StrokeKind::Outside,
                    );
                    // Gann grid: horizontal levels at Gann ratios
                    let gann_h: &[f64] = &[0.0, 0.125, 0.25, 0.375, 0.5, 0.625, 0.75, 0.875, 1.0];
                    for &ratio in gann_h {
                        let p = p1.1 + (p2.1 - p1.1) * ratio;
                        let yy = price_to_y(p);
                        let alpha = if ratio == 0.5 { 120 } else { 50 };
                        let c = egui::Color32::from_rgba_premultiplied(
                            color.r(),
                            color.g(),
                            color.b(),
                            alpha,
                        );
                        painter.line_segment(
                            [egui::pos2(x1, yy), egui::pos2(x2, yy)],
                            egui::Stroke::new(0.5, c),
                        );
                    }
                    // Vertical grid at same ratios
                    for &ratio in gann_h {
                        let xx = x1 + (x2 - x1) * ratio as f32;
                        let alpha = if ratio == 0.5 { 120 } else { 50 };
                        let c = egui::Color32::from_rgba_premultiplied(
                            color.r(),
                            color.g(),
                            color.b(),
                            alpha,
                        );
                        painter.line_segment(
                            [egui::pos2(xx, y1), egui::pos2(xx, y2)],
                            egui::Stroke::new(0.5, c),
                        );
                    }
                    // Diagonal 1×1 from corners
                    let c_diag =
                        egui::Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 80);
                    painter.line_segment(
                        [egui::pos2(x1, y1), egui::pos2(x2, y2)],
                        egui::Stroke::new(0.8, c_diag),
                    );
                    painter.line_segment(
                        [egui::pos2(x2, y1), egui::pos2(x1, y2)],
                        egui::Stroke::new(0.8, c_diag),
                    );
                }
            }
            Drawing::ElliottWave { points, color } => {
                let mut screen_pts: Vec<(f32, f32)> = Vec::new();
                for &(bi, pr) in points.iter() {
                    if bi >= start_idx && bi < end_idx {
                        let x = data_left + ((bi - start_idx) as f32 + 0.5) * bar_w;
                        let y = price_to_y(pr);
                        screen_pts.push((x, y));
                    }
                }
                let labels = ["1", "2", "3", "4", "5"];
                let sc = sel_tint(*color);
                for i in 0..screen_pts.len() {
                    if i + 1 < screen_pts.len() {
                        draw_styled_line(
                            &painter,
                            egui::pos2(screen_pts[i].0, screen_pts[i].1),
                            egui::pos2(screen_pts[i + 1].0, screen_pts[i + 1].1),
                            egui::Stroke::new(effective_width, sc),
                            d_style,
                        );
                    }
                    if i < labels.len() {
                        painter.text(
                            egui::pos2(screen_pts[i].0, screen_pts[i].1 - 10.0),
                            egui::Align2::CENTER_BOTTOM,
                            labels[i],
                            egui::FontId::monospace(11.0),
                            sc,
                        );
                    }
                }
            }
            Drawing::AbcCorrection { points, color } => {
                let mut screen_pts: Vec<(f32, f32)> = Vec::new();
                for &(bi, pr) in points.iter() {
                    if bi >= start_idx && bi < end_idx {
                        let x = data_left + ((bi - start_idx) as f32 + 0.5) * bar_w;
                        let y = price_to_y(pr);
                        screen_pts.push((x, y));
                    }
                }
                let labels = ["A", "B", "C"];
                let sc = sel_tint(*color);
                for i in 0..screen_pts.len() {
                    if i + 1 < screen_pts.len() {
                        draw_styled_line(
                            &painter,
                            egui::pos2(screen_pts[i].0, screen_pts[i].1),
                            egui::pos2(screen_pts[i + 1].0, screen_pts[i + 1].1),
                            egui::Stroke::new(effective_width, sc),
                            d_style,
                        );
                    }
                    if i < labels.len() {
                        painter.text(
                            egui::pos2(screen_pts[i].0, screen_pts[i].1 - 10.0),
                            egui::Align2::CENTER_BOTTOM,
                            labels[i],
                            egui::FontId::monospace(11.0),
                            sc,
                        );
                    }
                }
            }
            Drawing::DateRange { p1, p2 } => {
                let x1o = if p1.0 >= start_idx && p1.0 < end_idx {
                    Some(data_left + ((p1.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                let x2o = if p2.0 >= start_idx && p2.0 < end_idx {
                    Some(data_left + ((p2.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                if let (Some(x1), Some(x2)) = (x1o, x2o) {
                    let mid_y = (price_to_y(p1.1) + price_to_y(p2.1)) / 2.0;
                    let col = egui::Color32::from_rgb(100, 200, 255);
                    // Vertical markers
                    painter.line_segment(
                        [egui::pos2(x1, mid_y - 12.0), egui::pos2(x1, mid_y + 12.0)],
                        egui::Stroke::new(1.0, col),
                    );
                    painter.line_segment(
                        [egui::pos2(x2, mid_y - 12.0), egui::pos2(x2, mid_y + 12.0)],
                        egui::Stroke::new(1.0, col),
                    );
                    // Connecting line
                    painter.line_segment(
                        [egui::pos2(x1, mid_y), egui::pos2(x2, mid_y)],
                        egui::Stroke::new(1.0, col),
                    );
                    let bar_count = if p2.0 > p1.0 {
                        p2.0 - p1.0
                    } else {
                        p1.0 - p2.0
                    };
                    let label = format!("{} bars", bar_count);
                    painter.text(
                        egui::pos2((x1 + x2) / 2.0, mid_y - 6.0),
                        egui::Align2::CENTER_BOTTOM,
                        &label,
                        egui::FontId::monospace(10.0),
                        col,
                    );
                }
            }
            Drawing::DatePriceRange { p1, p2 } => {
                let x1o = if p1.0 >= start_idx && p1.0 < end_idx {
                    Some(data_left + ((p1.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                let x2o = if p2.0 >= start_idx && p2.0 < end_idx {
                    Some(data_left + ((p2.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                if let (Some(x1), Some(x2)) = (x1o, x2o) {
                    let y1 = price_to_y(p1.1);
                    let y2 = price_to_y(p2.1);
                    let fill = egui::Color32::from_rgba_premultiplied(100, 200, 150, 15);
                    painter.rect_filled(
                        egui::Rect::from_two_pos(egui::pos2(x1, y1), egui::pos2(x2, y2)),
                        0.0,
                        fill,
                    );
                    painter.rect_stroke(
                        egui::Rect::from_two_pos(egui::pos2(x1, y1), egui::pos2(x2, y2)),
                        0.0,
                        egui::Stroke::new(0.8, egui::Color32::from_rgb(100, 200, 150)),
                        egui::StrokeKind::Outside,
                    );
                    let bars = if p2.0 > p1.0 {
                        p2.0 - p1.0
                    } else {
                        p1.0 - p2.0
                    };
                    let dist = p2.1 - p1.1;
                    let pct = if p1.1.abs() > f64::EPSILON {
                        dist / p1.1 * 100.0
                    } else {
                        0.0
                    };
                    let label = format!("{} bars | {:.2} ({:+.2}%)", bars, dist, pct);
                    let col = egui::Color32::from_rgb(100, 200, 150);
                    painter.text(
                        egui::pos2((x1 + x2) / 2.0, y1.min(y2) - 4.0),
                        egui::Align2::CENTER_BOTTOM,
                        &label,
                        egui::FontId::monospace(10.0),
                        col,
                    );
                }
            }
            Drawing::HeadShoulders { points, color } => {
                // 5 points: 0=LS bottom, 1=LS top, 2=Head top, 3=RS top, 4=RS bottom
                // Connect all in order, draw neckline between 0 and 4
                let mut screen_pts: Vec<(f32, f32)> = Vec::new();
                for &(bi, pr) in points.iter() {
                    if bi >= start_idx && bi < end_idx {
                        let x = data_left + ((bi - start_idx) as f32 + 0.5) * bar_w;
                        let y = price_to_y(pr);
                        screen_pts.push((x, y));
                    }
                }
                let labels = ["LS", "L", "H", "R", "RS"];
                let sc = sel_tint(*color);
                for i in 0..screen_pts.len() {
                    if i + 1 < screen_pts.len() {
                        draw_styled_line(
                            &painter,
                            egui::pos2(screen_pts[i].0, screen_pts[i].1),
                            egui::pos2(screen_pts[i + 1].0, screen_pts[i + 1].1),
                            egui::Stroke::new(effective_width, sc),
                            d_style,
                        );
                    }
                    if i < labels.len() {
                        painter.text(
                            egui::pos2(screen_pts[i].0, screen_pts[i].1 - 10.0),
                            egui::Align2::CENTER_BOTTOM,
                            labels[i],
                            egui::FontId::monospace(9.0),
                            sc,
                        );
                    }
                }
                // Neckline: dashed line between point 0 and point 4
                if screen_pts.len() >= 5 {
                    let nk_col =
                        egui::Color32::from_rgba_premultiplied(sc.r(), sc.g(), sc.b(), 150);
                    draw_styled_line(
                        &painter,
                        egui::pos2(screen_pts[0].0, screen_pts[0].1),
                        egui::pos2(screen_pts[4].0, screen_pts[4].1),
                        egui::Stroke::new(effective_width, nk_col),
                        LineStyle::Dashed,
                    );
                    painter.text(
                        egui::pos2(
                            (screen_pts[0].0 + screen_pts[4].0) / 2.0,
                            (screen_pts[0].1 + screen_pts[4].1) / 2.0 + 12.0,
                        ),
                        egui::Align2::CENTER_TOP,
                        "Neckline",
                        egui::FontId::monospace(9.0),
                        nk_col,
                    );
                }
            }
            Drawing::XabcdPattern { points, color } => {
                let mut screen_pts: Vec<(f32, f32)> = Vec::new();
                for &(bi, pr) in points.iter() {
                    if bi >= start_idx && bi < end_idx {
                        let x = data_left + ((bi - start_idx) as f32 + 0.5) * bar_w;
                        let y = price_to_y(pr);
                        screen_pts.push((x, y));
                    }
                }
                let labels = ["X", "A", "B", "C", "D"];
                let sc = sel_tint(*color);
                for i in 0..screen_pts.len() {
                    if i + 1 < screen_pts.len() {
                        draw_styled_line(
                            &painter,
                            egui::pos2(screen_pts[i].0, screen_pts[i].1),
                            egui::pos2(screen_pts[i + 1].0, screen_pts[i + 1].1),
                            egui::Stroke::new(effective_width, sc),
                            d_style,
                        );
                    }
                    if i < labels.len() {
                        painter.text(
                            egui::pos2(screen_pts[i].0, screen_pts[i].1 - 10.0),
                            egui::Align2::CENTER_BOTTOM,
                            labels[i],
                            egui::FontId::monospace(11.0),
                            sc,
                        );
                    }
                }
                // XA→BD dashed line (harmonic diagonal)
                if screen_pts.len() >= 5 {
                    let diag = egui::Color32::from_rgba_premultiplied(sc.r(), sc.g(), sc.b(), 80);
                    draw_styled_line(
                        &painter,
                        egui::pos2(screen_pts[0].0, screen_pts[0].1),
                        egui::pos2(screen_pts[3].0, screen_pts[3].1),
                        egui::Stroke::new(0.6, diag),
                        LineStyle::Dashed,
                    );
                    draw_styled_line(
                        &painter,
                        egui::pos2(screen_pts[1].0, screen_pts[1].1),
                        egui::pos2(screen_pts[4].0, screen_pts[4].1),
                        egui::Stroke::new(0.6, diag),
                        LineStyle::Dashed,
                    );
                }
            }
            Drawing::Brush { points, color } => {
                for &(bi, pr) in points.iter() {
                    if bi >= start_idx && bi < end_idx {
                        let x = data_left + ((bi - start_idx) as f32 + 0.5) * bar_w;
                        let y = price_to_y(pr);
                        painter.circle_filled(egui::pos2(x, y), 2.0, *color);
                    }
                }
            }
            Drawing::SchiffPitchfork {
                pivot,
                p2,
                p3,
                color,
            }
            | Drawing::ModSchiffPitchfork {
                pivot,
                p2,
                p3,
                color,
            } => {
                // Schiff: shifted pivot = midpoint(pivot, p2) on bar-axis, midpoint(pivot, p2) on price
                // Modified Schiff: shifted pivot = (mid(pivot.bar, p2.bar), mid(pivot.price, p3.price))
                let is_mod = matches!(drawing, Drawing::ModSchiffPitchfork { .. });
                let shifted_bar = if is_mod {
                    ((pivot.0 as f64 + p2.0 as f64) / 2.0) as usize
                } else {
                    ((pivot.0 as f64 + p2.0 as f64) / 2.0) as usize
                };
                let shifted_price = if is_mod {
                    (pivot.1 + p2.1) / 2.0 * 0.5 + (pivot.1 + p3.1) / 2.0 * 0.5
                } else {
                    (pivot.1 + p2.1) / 2.0
                };
                let mid_bar = ((p2.0 as f64 + p3.0 as f64) / 2.0) as usize;
                let mid_price = (p2.1 + p3.1) / 2.0;
                let bar_to_x = |b: usize| -> Option<f32> {
                    if b >= start_idx && b < end_idx {
                        Some(data_left + ((b - start_idx) as f32 + 0.5) * bar_w)
                    } else {
                        None
                    }
                };
                let sc = sel_tint(*color);
                // Median line: shifted pivot → midpoint of p2,p3
                if let (Some(sx), Some(mx)) = (bar_to_x(shifted_bar), bar_to_x(mid_bar)) {
                    draw_styled_line(
                        &painter,
                        egui::pos2(sx, price_to_y(shifted_price)),
                        egui::pos2(mx, price_to_y(mid_price)),
                        egui::Stroke::new(effective_width, sc),
                        d_style,
                    );
                }
                // Parallel lines through p2 and p3
                if let (Some(sx), Some(mx), Some(x2), Some(x3)) = (
                    bar_to_x(shifted_bar),
                    bar_to_x(mid_bar),
                    bar_to_x(p2.0),
                    bar_to_x(p3.0),
                ) {
                    let dx = mx - sx;
                    let dy = price_to_y(mid_price) - price_to_y(shifted_price);
                    let y2 = price_to_y(p2.1);
                    let y3 = price_to_y(p3.1);
                    draw_styled_line(
                        &painter,
                        egui::pos2(x2, y2),
                        egui::pos2(x2 + dx, y2 + dy),
                        egui::Stroke::new(effective_width * 0.7, sc),
                        d_style,
                    );
                    draw_styled_line(
                        &painter,
                        egui::pos2(x3, y3),
                        egui::pos2(x3 + dx, y3 + dy),
                        egui::Stroke::new(effective_width * 0.7, sc),
                        d_style,
                    );
                }
            }
            Drawing::CyclicLines {
                bar_start,
                bar_end,
                color,
            } => {
                let interval = if *bar_end > *bar_start {
                    bar_end - bar_start
                } else {
                    1
                };
                let mut b = *bar_start;
                while b < start_idx + (end_idx - start_idx) + interval * 20 {
                    if b >= start_idx && b < end_idx {
                        let x = data_left + ((b - start_idx) as f32 + 0.5) * bar_w;
                        draw_styled_line(
                            &painter,
                            egui::pos2(x, chart_rect.top()),
                            egui::pos2(x, chart_rect.bottom()),
                            egui::Stroke::new(effective_width * 0.5, sel_tint(*color)),
                            d_style,
                        );
                    }
                    b += interval;
                }
            }
            Drawing::SineWave { p1, p2, color } => {
                let bar_to_x =
                    |b: usize| -> f32 { data_left + ((b as f32 - start_idx as f32) + 0.5) * bar_w };
                let period = ((p2.0 as f64 - p1.0 as f64).abs()).max(1.0);
                let amplitude = (p2.1 - p1.1).abs() / 2.0;
                let mid_price = (p1.1 + p2.1) / 2.0;
                let start_bar = p1.0;
                let mut prev: Option<egui::Pos2> = None;
                for b in start_idx..end_idx {
                    let phase = (b as f64 - start_bar as f64) / period * 2.0 * std::f64::consts::PI;
                    let price_val = mid_price + amplitude * phase.sin();
                    let x = bar_to_x(b);
                    let y = price_to_y(price_val);
                    let pt = egui::pos2(x, y);
                    if let Some(p) = prev {
                        painter.line_segment(
                            [p, pt],
                            egui::Stroke::new(effective_width, sel_tint(*color)),
                        );
                    }
                    prev = Some(pt);
                }
            }
            Drawing::Emoji {
                bar_idx,
                price,
                emoji,
            } => {
                if *bar_idx >= start_idx && *bar_idx < end_idx {
                    let x = data_left + ((*bar_idx - start_idx) as f32 + 0.5) * bar_w;
                    let y = price_to_y(*price);
                    painter.text(
                        egui::pos2(x, y),
                        egui::Align2::CENTER_CENTER,
                        emoji,
                        egui::FontId::proportional(16.0),
                        egui::Color32::WHITE,
                    );
                }
            }
            Drawing::Flag {
                bar_idx,
                price,
                color,
            } => {
                if *bar_idx >= start_idx && *bar_idx < end_idx {
                    let x = data_left + ((*bar_idx - start_idx) as f32 + 0.5) * bar_w;
                    let y = price_to_y(*price);
                    let sc = sel_tint(*color);
                    // Pole
                    draw_styled_line(
                        &painter,
                        egui::pos2(x, y),
                        egui::pos2(x, y - 20.0),
                        egui::Stroke::new(effective_width, sc),
                        d_style,
                    );
                    // Flag triangle
                    let tri = vec![
                        egui::pos2(x, y - 20.0),
                        egui::pos2(x + 12.0, y - 15.0),
                        egui::pos2(x, y - 10.0),
                    ];
                    painter.add(egui::Shape::convex_polygon(tri, sc, egui::Stroke::NONE));
                }
            }
            Drawing::Balloon {
                anchor,
                label_pos,
                text,
                color,
            } => {
                let bar_to_x = |b: usize| -> Option<f32> {
                    if b >= start_idx && b < end_idx {
                        Some(data_left + ((b - start_idx) as f32 + 0.5) * bar_w)
                    } else {
                        None
                    }
                };
                if let (Some(ax), Some(lx)) = (bar_to_x(anchor.0), bar_to_x(label_pos.0)) {
                    let ay = price_to_y(anchor.1);
                    let ly = price_to_y(label_pos.1);
                    // Line from anchor to label
                    draw_styled_line(
                        &painter,
                        egui::pos2(ax, ay),
                        egui::pos2(lx, ly),
                        egui::Stroke::new(effective_width, sel_tint(*color)),
                        d_style,
                    );
                    // Bubble background
                    let text_rect =
                        egui::Rect::from_center_size(egui::pos2(lx, ly), egui::vec2(80.0, 24.0));
                    painter.rect_filled(
                        text_rect,
                        6.0,
                        egui::Color32::from_rgba_premultiplied(40, 40, 60, 200),
                    );
                    let sc = sel_tint(*color);
                    painter.rect_stroke(
                        text_rect,
                        6.0,
                        egui::Stroke::new(effective_width, sc),
                        egui::StrokeKind::Outside,
                    );
                    painter.text(
                        egui::pos2(lx, ly),
                        egui::Align2::CENTER_CENTER,
                        text,
                        egui::FontId::monospace(10.0),
                        sc,
                    );
                }
            }
            Drawing::SessionBreak { bar_idx, color } => {
                if *bar_idx >= start_idx && *bar_idx < end_idx {
                    let x = data_left + ((*bar_idx - start_idx) as f32 + 0.5) * bar_w;
                    let sc = sel_tint(*color);
                    // Dashed vertical line — delegate to draw_line for style support
                    draw_styled_line(
                        &painter,
                        egui::pos2(x, chart_rect.top()),
                        egui::pos2(x, chart_rect.bottom()),
                        egui::Stroke::new(effective_width, sc),
                        LineStyle::Dashed,
                    );
                    painter.text(
                        egui::pos2(x + 4.0, chart_rect.top() + 2.0),
                        egui::Align2::LEFT_TOP,
                        "Session",
                        egui::FontId::monospace(8.0),
                        sc,
                    );
                }
            }
            Drawing::MagnetLevel { price, color } => {
                let y = price_to_y(*price);
                if y >= chart_rect.top() && y <= chart_rect.bottom() {
                    // Check if last bar's close is within 0.5% of this level
                    let glow = if end_idx > start_idx {
                        let last_close =
                            chart.bars.get(end_idx - 1).map(|b| b.close).unwrap_or(0.0);
                        (last_close - price).abs() / price.abs().max(0.0001) < 0.005
                    } else {
                        false
                    };
                    let base_col = if glow {
                        egui::Color32::from_rgb(255, 255, 100)
                    } else {
                        sel_tint(*color)
                    };
                    let stroke_w = if glow {
                        effective_width.max(2.5)
                    } else {
                        effective_width
                    };
                    let draw_color = base_col;
                    draw_styled_line(
                        &painter,
                        egui::pos2(chart_rect.left(), y),
                        egui::pos2(chart_rect.right(), y),
                        egui::Stroke::new(stroke_w, draw_color),
                        d_style,
                    );
                    if glow {
                        // Glow effect: semi-transparent wider line
                        let glow_col = egui::Color32::from_rgba_premultiplied(255, 255, 100, 40);
                        painter.line_segment(
                            [
                                egui::pos2(chart_rect.left(), y),
                                egui::pos2(chart_rect.right(), y),
                            ],
                            egui::Stroke::new(6.0, glow_col),
                        );
                    }
                    painter.text(
                        egui::pos2(chart_rect.right() - 80.0, y - 10.0),
                        egui::Align2::LEFT_TOP,
                        &format!("M {}", &format_price(*price)),
                        egui::FontId::monospace(9.0),
                        base_col,
                    );
                }
            }
            Drawing::RiskRewardBox {
                entry,
                stop,
                target,
            } => {
                let bar_to_x =
                    |b: usize| -> f32 { data_left + ((b as f32 - start_idx as f32) + 0.5) * bar_w };
                let entry_x = bar_to_x(entry.0);
                let entry_y = price_to_y(entry.1);
                let stop_y = price_to_y(*stop);
                let target_y = price_to_y(*target);
                let box_width = bar_w * 20.0;
                let right_x = entry_x + box_width;
                // Risk zone (entry to stop) — red
                let risk_rect = egui::Rect::from_two_pos(
                    egui::pos2(entry_x, entry_y),
                    egui::pos2(right_x, stop_y),
                );
                painter.rect_filled(
                    risk_rect,
                    0.0,
                    egui::Color32::from_rgba_premultiplied(220, 40, 40, 30),
                );
                painter.rect_stroke(
                    risk_rect,
                    0.0,
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(220, 40, 40)),
                    egui::StrokeKind::Outside,
                );
                // Reward zone (entry to target) — green
                let reward_rect = egui::Rect::from_two_pos(
                    egui::pos2(entry_x, entry_y),
                    egui::pos2(right_x, target_y),
                );
                painter.rect_filled(
                    reward_rect,
                    0.0,
                    egui::Color32::from_rgba_premultiplied(0, 200, 80, 30),
                );
                painter.rect_stroke(
                    reward_rect,
                    0.0,
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 200, 80)),
                    egui::StrokeKind::Outside,
                );
                // Entry line
                painter.line_segment(
                    [egui::pos2(entry_x, entry_y), egui::pos2(right_x, entry_y)],
                    egui::Stroke::new(1.5, egui::Color32::WHITE),
                );
                // R:R ratio
                let risk = (entry.1 - stop).abs();
                let reward = (target - entry.1).abs();
                let rr = if risk > 0.0 { reward / risk } else { 0.0 };
                painter.text(
                    egui::pos2(right_x + 4.0, entry_y),
                    egui::Align2::LEFT_CENTER,
                    &format!("R:R {:.1}", rr),
                    egui::FontId::monospace(10.0),
                    egui::Color32::WHITE,
                );
            }
            Drawing::FibCircle {
                center,
                radius_pt,
                color,
            } => {
                let cx = data_left + ((center.0 as f32 - start_idx as f32) + 0.5) * bar_w;
                let cy = price_to_y(center.1);
                let rx = data_left + ((radius_pt.0 as f32 - start_idx as f32) + 0.5) * bar_w;
                let ry = price_to_y(radius_pt.1);
                let base_r = ((rx - cx).powi(2) + (ry - cy).powi(2)).sqrt();
                let fib_ratios = [0.236, 0.382, 0.5, 0.618, 0.786, 1.0];
                for ratio in &fib_ratios {
                    let r = base_r * (*ratio as f32);
                    let segments = 64;
                    let mut pts = Vec::with_capacity(segments + 1);
                    for i in 0..=segments {
                        let angle = (i as f32 / segments as f32) * 2.0 * std::f32::consts::PI;
                        pts.push(egui::pos2(cx + r * angle.cos(), cy + r * angle.sin()));
                    }
                    let sc = sel_tint(*color);
                    for w in pts.windows(2) {
                        painter.line_segment([w[0], w[1]], egui::Stroke::new(effective_width, sc));
                    }
                    painter.text(
                        egui::pos2(cx + r + 2.0, cy),
                        egui::Align2::LEFT_CENTER,
                        &format!("{:.3}", ratio),
                        egui::FontId::monospace(8.0),
                        *color,
                    );
                }
            }
            Drawing::ArcDraw { p1, p2, p3, color } => {
                let bar_to_x =
                    |b: usize| -> f32 { data_left + ((b as f32 - start_idx as f32) + 0.5) * bar_w };
                let x1 = bar_to_x(p1.0);
                let y1 = price_to_y(p1.1);
                let x2 = bar_to_x(p2.0);
                let y2 = price_to_y(p2.1);
                let x3 = bar_to_x(p3.0);
                let y3 = price_to_y(p3.1);
                // Quadratic bezier through 3 points: control point derived from midpoint
                let ctrl_x = 2.0 * x2 - 0.5 * x1 - 0.5 * x3;
                let ctrl_y = 2.0 * y2 - 0.5 * y1 - 0.5 * y3;
                let segments = 48;
                let mut prev = egui::pos2(x1, y1);
                for i in 1..=segments {
                    let t = i as f32 / segments as f32;
                    let it = 1.0 - t;
                    let px = it * it * x1 + 2.0 * it * t * ctrl_x + t * t * x3;
                    let py = it * it * y1 + 2.0 * it * t * ctrl_y + t * t * y3;
                    let pt = egui::pos2(px, py);
                    painter.line_segment(
                        [prev, pt],
                        egui::Stroke::new(effective_width, sel_tint(*color)),
                    );
                    prev = pt;
                }
            }
            Drawing::CurveDraw {
                p1,
                ctrl1,
                ctrl2,
                p2,
                color,
            } => {
                let bar_to_x =
                    |b: usize| -> f32 { data_left + ((b as f32 - start_idx as f32) + 0.5) * bar_w };
                let x0 = bar_to_x(p1.0);
                let y0 = price_to_y(p1.1);
                let cx1 = bar_to_x(ctrl1.0);
                let cy1 = price_to_y(ctrl1.1);
                let cx2 = bar_to_x(ctrl2.0);
                let cy2 = price_to_y(ctrl2.1);
                let x3 = bar_to_x(p2.0);
                let y3 = price_to_y(p2.1);
                let segments = 64;
                let mut prev = egui::pos2(x0, y0);
                for i in 1..=segments {
                    let t = i as f32 / segments as f32;
                    let it = 1.0 - t;
                    let px = it.powi(3) * x0
                        + 3.0 * it.powi(2) * t * cx1
                        + 3.0 * it * t.powi(2) * cx2
                        + t.powi(3) * x3;
                    let py = it.powi(3) * y0
                        + 3.0 * it.powi(2) * t * cy1
                        + 3.0 * it * t.powi(2) * cy2
                        + t.powi(3) * y3;
                    let pt = egui::pos2(px, py);
                    painter.line_segment(
                        [prev, pt],
                        egui::Stroke::new(effective_width, sel_tint(*color)),
                    );
                    prev = pt;
                }
                // Draw control point markers
                painter.circle_stroke(egui::pos2(cx1, cy1), 3.0, egui::Stroke::new(1.0, *color));
                painter.circle_stroke(egui::pos2(cx2, cy2), 3.0, egui::Stroke::new(1.0, *color));
            }
            Drawing::PathDraw { points, color } => {
                if points.len() >= 2 {
                    let bar_to_x = |b: usize| -> f32 {
                        data_left + ((b as f32 - start_idx as f32) + 0.5) * bar_w
                    };
                    let screen_pts: Vec<egui::Pos2> = points
                        .iter()
                        .map(|(b, p)| egui::pos2(bar_to_x(*b), price_to_y(*p)))
                        .collect();
                    // Catmull-Rom interpolation between each segment
                    for seg in 0..screen_pts.len() - 1 {
                        let p0 = if seg > 0 {
                            screen_pts[seg - 1]
                        } else {
                            screen_pts[seg]
                        };
                        let pa = screen_pts[seg];
                        let pb = screen_pts[seg + 1];
                        let p3 = if seg + 2 < screen_pts.len() {
                            screen_pts[seg + 2]
                        } else {
                            screen_pts[seg + 1]
                        };
                        let steps = 24;
                        let mut prev = pa;
                        for i in 1..=steps {
                            let t = i as f32 / steps as f32;
                            let t2 = t * t;
                            let t3 = t2 * t;
                            let px = 0.5
                                * ((2.0 * pa.x)
                                    + (-p0.x + pb.x) * t
                                    + (2.0 * p0.x - 5.0 * pa.x + 4.0 * pb.x - p3.x) * t2
                                    + (-p0.x + 3.0 * pa.x - 3.0 * pb.x + p3.x) * t3);
                            let py = 0.5
                                * ((2.0 * pa.y)
                                    + (-p0.y + pb.y) * t
                                    + (2.0 * p0.y - 5.0 * pa.y + 4.0 * pb.y - p3.y) * t2
                                    + (-p0.y + 3.0 * pa.y - 3.0 * pb.y + p3.y) * t3);
                            let pt = egui::pos2(px, py);
                            painter.line_segment(
                                [prev, pt],
                                egui::Stroke::new(effective_width, sel_tint(*color)),
                            );
                            prev = pt;
                        }
                    }
                }
            }
            Drawing::Forecast { p1, p2, color } => {
                let bar_to_x =
                    |b: usize| -> f32 { data_left + ((b as f32 - start_idx as f32) + 0.5) * bar_w };
                let x1 = bar_to_x(p1.0);
                let y1 = price_to_y(p1.1);
                let x2 = bar_to_x(p2.0);
                let y2 = price_to_y(p2.1);
                let sc = sel_tint(*color);
                // Solid trend line
                draw_styled_line(
                    &painter,
                    egui::pos2(x1, y1),
                    egui::pos2(x2, y2),
                    egui::Stroke::new(effective_width, sc),
                    d_style,
                );
                // Dashed projection forward (same slope, same length)
                let dx = x2 - x1;
                let dy = y2 - y1;
                let proj_x = x2 + dx;
                let proj_y = y2 + dy;
                draw_styled_line(
                    &painter,
                    egui::pos2(x2, y2),
                    egui::pos2(proj_x, proj_y),
                    egui::Stroke::new(effective_width * 0.7, sc),
                    LineStyle::Dashed,
                );
                painter.text(
                    egui::pos2(proj_x + 4.0, proj_y),
                    egui::Align2::LEFT_CENTER,
                    "Forecast",
                    egui::FontId::monospace(9.0),
                    sc,
                );
            }
            Drawing::GhostFeed { p1, p2, color } => {
                let bar_to_x =
                    |b: usize| -> f32 { data_left + ((b as f32 - start_idx as f32) + 0.5) * bar_w };
                // Mirror the bars from p1..p2 forward starting at p2
                let src_start = p1.0.min(p2.0);
                let src_end = p1.0.max(p2.0);
                let mirror_len = src_end - src_start;
                if mirror_len > 0 {
                    for i in 0..mirror_len {
                        let src_idx = src_start + i;
                        let dst_idx = src_end + i;
                        if src_idx < chart.bars.len()
                            && dst_idx < chart.bars.len() + CHART_RIGHT_MARGIN
                        {
                            let src_bar = chart.bars.get(src_idx);
                            if let Some(sb) = src_bar {
                                let x = bar_to_x(dst_idx);
                                let oy = price_to_y(sb.open);
                                let cy = price_to_y(sb.close);
                                let hy = price_to_y(sb.high);
                                let ly = price_to_y(sb.low);
                                let ghost_col = egui::Color32::from_rgba_premultiplied(
                                    color.r(),
                                    color.g(),
                                    color.b(),
                                    80,
                                );
                                painter.line_segment(
                                    [egui::pos2(x, hy), egui::pos2(x, ly)],
                                    egui::Stroke::new(0.5, ghost_col),
                                );
                                let top = oy.min(cy);
                                let bot = oy.max(cy);
                                let w = (bar_w * 0.6).max(1.0);
                                painter.rect_filled(
                                    egui::Rect::from_min_max(
                                        egui::pos2(x - w / 2.0, top),
                                        egui::pos2(x + w / 2.0, bot),
                                    ),
                                    0.0,
                                    ghost_col,
                                );
                            }
                        }
                    }
                }
            }
            Drawing::Signpost {
                bar_idx,
                price,
                color,
            } => {
                if *bar_idx >= start_idx && *bar_idx < end_idx {
                    let x = data_left + ((*bar_idx - start_idx) as f32 + 0.5) * bar_w;
                    let y = price_to_y(*price);
                    let sc = sel_tint(*color);
                    // Pole
                    draw_styled_line(
                        &painter,
                        egui::pos2(x, y + 15.0),
                        egui::pos2(x, y - 15.0),
                        egui::Stroke::new(effective_width, sc),
                        d_style,
                    );
                    // Arrow head (pointing right)
                    let arrow = vec![
                        egui::pos2(x, y - 12.0),
                        egui::pos2(x + 14.0, y - 6.0),
                        egui::pos2(x, y),
                    ];
                    painter.add(egui::Shape::convex_polygon(arrow, sc, egui::Stroke::NONE));
                    // Base
                    draw_styled_line(
                        &painter,
                        egui::pos2(x - 5.0, y + 15.0),
                        egui::pos2(x + 5.0, y + 15.0),
                        egui::Stroke::new(effective_width, sc),
                        d_style,
                    );
                }
            }
            Drawing::Ruler { p1, p2, color } => {
                let bar_to_x =
                    |b: usize| -> f32 { data_left + ((b as f32 - start_idx as f32) + 0.5) * bar_w };
                let x1 = bar_to_x(p1.0);
                let y1 = price_to_y(p1.1);
                let x2 = bar_to_x(p2.0);
                let y2 = price_to_y(p2.1);
                let sc = sel_tint(*color);
                draw_styled_line(
                    &painter,
                    egui::pos2(x1, y1),
                    egui::pos2(x2, y2),
                    egui::Stroke::new(effective_width, sc),
                    d_style,
                );
                // Endpoints
                painter.circle_filled(egui::pos2(x1, y1), 3.0, sc);
                painter.circle_filled(egui::pos2(x2, y2), 3.0, sc);
                // Measurement label
                let price_diff = p2.1 - p1.1;
                let bars_diff = if p2.0 > p1.0 {
                    p2.0 - p1.0
                } else {
                    p1.0 - p2.0
                };
                let pct = if p1.1.abs() > 0.0001 {
                    (price_diff / p1.1) * 100.0
                } else {
                    0.0
                };
                let mid_x = (x1 + x2) / 2.0;
                let mid_y = (y1 + y2) / 2.0;
                let label = format!("{:.4} ({} bars, {:.2}%)", price_diff, bars_diff, pct);
                let bg_rect = egui::Rect::from_center_size(
                    egui::pos2(mid_x, mid_y - 12.0),
                    egui::vec2(label.len() as f32 * 6.5 + 8.0, 16.0),
                );
                painter.rect_filled(
                    bg_rect,
                    3.0,
                    egui::Color32::from_rgba_premultiplied(0, 0, 0, 200),
                );
                painter.text(
                    egui::pos2(mid_x, mid_y - 12.0),
                    egui::Align2::CENTER_CENTER,
                    &label,
                    egui::FontId::monospace(10.0),
                    sc,
                );
            }
            Drawing::TimeCycle {
                bar_start,
                bar_end,
                color,
            } => {
                let interval = if *bar_end > *bar_start {
                    bar_end - bar_start
                } else {
                    1
                };
                let mut b = *bar_start;
                while b < chart.bars.len() + CHART_RIGHT_MARGIN * 10 {
                    if b >= start_idx && b < end_idx {
                        let x = data_left + ((b as f32 - start_idx as f32) + 0.5) * bar_w;
                        let sc = sel_tint(*color);
                        draw_styled_line(
                            &painter,
                            egui::pos2(x, chart_rect.top()),
                            egui::pos2(x, chart_rect.bottom()),
                            egui::Stroke::new(effective_width, sc),
                            d_style,
                        );
                    }
                    // Draw semi-circle arc between this line and the next
                    let next_b = b + interval;
                    if b >= start_idx && next_b < end_idx {
                        let x1 = data_left + ((b as f32 - start_idx as f32) + 0.5) * bar_w;
                        let x2 = data_left + ((next_b as f32 - start_idx as f32) + 0.5) * bar_w;
                        let cx = (x1 + x2) / 2.0;
                        let r = (x2 - x1) / 2.0;
                        let arc_y = chart_rect.bottom() - 2.0;
                        let segs = 24;
                        let sc = sel_tint(*color);
                        let mut prev_pt = egui::pos2(x1, arc_y);
                        for i in 1..=segs {
                            let angle = std::f32::consts::PI * (i as f32 / segs as f32);
                            let px = cx - r * angle.cos();
                            let py = arc_y - r * angle.sin() * 0.3; // squashed arc
                            let pt = egui::pos2(px, py);
                            painter.line_segment(
                                [prev_pt, pt],
                                egui::Stroke::new(effective_width * 0.55, sc),
                            );
                            prev_pt = pt;
                        }
                    }
                    b += interval;
                    if b > end_idx + interval * 2 {
                        break;
                    }
                }
            }
            Drawing::SpeedResistanceFan { p1, p2, p3, color } => {
                let bar_to_x =
                    |b: usize| -> f32 { data_left + ((b as f32 - start_idx as f32) + 0.5) * bar_w };
                let x1 = bar_to_x(p1.0);
                let y1 = price_to_y(p1.1);
                let x2 = bar_to_x(p2.0);
                let y2 = price_to_y(p2.1);
                let x3 = bar_to_x(p3.0);
                let _ = x3;
                // Speed lines: 1/3 and 2/3 of the move
                let dy = y2 - y1;
                let dx = x2 - x1;
                let extend = chart_rect.right() - x1;
                let sc = sel_tint(*color);
                for frac in [1.0_f32 / 3.0, 2.0 / 3.0] {
                    let target_y = y1 + dy * frac;
                    let slope = if dx.abs() > 0.1 {
                        (target_y - y1) / dx
                    } else {
                        0.0
                    };
                    let end_x = x1 + extend;
                    let end_y = y1 + slope * extend;
                    draw_styled_line(
                        &painter,
                        egui::pos2(x1, y1),
                        egui::pos2(end_x, end_y),
                        egui::Stroke::new(effective_width * 0.7, sc),
                        d_style,
                    );
                    painter.text(
                        egui::pos2(end_x - 30.0, end_y),
                        egui::Align2::LEFT_CENTER,
                        &format!("{:.0}%", frac * 100.0),
                        egui::FontId::monospace(8.0),
                        sc,
                    );
                }
                // Base line
                draw_styled_line(
                    &painter,
                    egui::pos2(x1, y1),
                    egui::pos2(x2, y2),
                    egui::Stroke::new(effective_width, sc),
                    d_style,
                );
            }
            Drawing::SpeedResistanceArc { p1, p2, p3, color } => {
                let bar_to_x =
                    |b: usize| -> f32 { data_left + ((b as f32 - start_idx as f32) + 0.5) * bar_w };
                let x1 = bar_to_x(p1.0);
                let y1 = price_to_y(p1.1);
                let x2 = bar_to_x(p2.0);
                let y2 = price_to_y(p2.1);
                let _ = bar_to_x(p3.0);
                let base_r = ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt();
                let sc = sel_tint(*color);
                // Base line
                draw_styled_line(
                    &painter,
                    egui::pos2(x1, y1),
                    egui::pos2(x2, y2),
                    egui::Stroke::new(effective_width, sc),
                    d_style,
                );
                // Arcs at 1/3 and 2/3
                for frac in [1.0_f32 / 3.0, 2.0 / 3.0] {
                    let r = base_r * frac;
                    let segs = 32;
                    let mut prev: Option<egui::Pos2> = None;
                    for i in 0..=segs {
                        let angle = std::f32::consts::PI * (i as f32 / segs as f32);
                        let px = x1 + r * angle.cos();
                        let py = y1 - r * angle.sin();
                        let pt = egui::pos2(px, py);
                        if let Some(p) = prev {
                            painter.line_segment(
                                [p, pt],
                                egui::Stroke::new(effective_width * 0.7, sc),
                            );
                        }
                        prev = Some(pt);
                    }
                }
            }
            Drawing::FibSpiral {
                center,
                radius_pt,
                color,
            } => {
                let cx = data_left + ((center.0 as f32 - start_idx as f32) + 0.5) * bar_w;
                let cy = price_to_y(center.1);
                let rx = data_left + ((radius_pt.0 as f32 - start_idx as f32) + 0.5) * bar_w;
                let ry = price_to_y(radius_pt.1);
                let base_r = ((rx - cx).powi(2) + (ry - cy).powi(2)).sqrt().max(1.0);
                // Golden spiral: r = a * e^(b*theta) where b = ln(phi)/(PI/2)
                let phi: f32 = 1.618033988749895;
                let b_param = phi.ln() / (std::f32::consts::PI / 2.0);
                let a_param = base_r / (b_param * 6.0 * std::f32::consts::PI).exp();
                let total_angle = 6.0 * std::f32::consts::PI; // 3 full turns
                let steps = 200;
                let mut prev: Option<egui::Pos2> = None;
                for i in 0..=steps {
                    let theta = total_angle * (i as f32 / steps as f32);
                    let r = a_param * (b_param * theta).exp();
                    let px = cx + r * theta.cos();
                    let py = cy - r * theta.sin();
                    let pt = egui::pos2(px, py);
                    if let Some(p) = prev {
                        painter.line_segment(
                            [p, pt],
                            egui::Stroke::new(effective_width, sel_tint(*color)),
                        );
                    }
                    prev = Some(pt);
                }
            }
            Drawing::RotatedRectangle { p1, p2, p3, color } => {
                let bar_to_x =
                    |b: usize| -> f32 { data_left + ((b as f32 - start_idx as f32) + 0.5) * bar_w };
                let x1 = bar_to_x(p1.0);
                let y1 = price_to_y(p1.1);
                let x2 = bar_to_x(p2.0);
                let y2 = price_to_y(p2.1);
                let x3 = bar_to_x(p3.0);
                let y3 = price_to_y(p3.1);
                // Baseline direction
                let bx = x2 - x1;
                let by = y2 - y1;
                let blen = (bx * bx + by * by).sqrt().max(0.001);
                let nx = -by / blen;
                let ny = bx / blen;
                // Project p3 onto the normal to get height
                let h = (x3 - x1) * nx + (y3 - y1) * ny;
                // Four corners
                let c1 = egui::pos2(x1, y1);
                let c2 = egui::pos2(x2, y2);
                let c3 = egui::pos2(x2 + nx * h, y2 + ny * h);
                let c4 = egui::pos2(x1 + nx * h, y1 + ny * h);
                let fill =
                    egui::Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 25);
                painter.add(egui::Shape::convex_polygon(
                    vec![c1, c2, c3, c4],
                    fill,
                    egui::Stroke::new(effective_width, sel_tint(*color)),
                ));
            }
            Drawing::AnchoredVwapLine { bar_idx, color } => {
                if *bar_idx < chart.bars.len() {
                    let mut cum_vol_price = 0.0_f64;
                    let mut cum_vol = 0.0_f64;
                    let mut prev_pt: Option<egui::Pos2> = None;
                    for i in *bar_idx..chart.bars.len() {
                        let bar = &chart.bars[i];
                        let typical = (bar.high + bar.low + bar.close) / 3.0;
                        cum_vol_price += typical * bar.volume;
                        cum_vol += bar.volume;
                        let vwap = if cum_vol > 0.0 {
                            cum_vol_price / cum_vol
                        } else {
                            typical
                        };
                        if i >= start_idx && i < end_idx {
                            let x = data_left + ((i as f32 - start_idx as f32) + 0.5) * bar_w;
                            let y = price_to_y(vwap);
                            let pt = egui::pos2(x, y);
                            if let Some(p) = prev_pt {
                                painter.line_segment(
                                    [p, pt],
                                    egui::Stroke::new(effective_width, sel_tint(*color)),
                                );
                            }
                            prev_pt = Some(pt);
                        } else {
                            prev_pt = None;
                        }
                    }
                    // Label
                    if let Some(last) = prev_pt {
                        painter.text(
                            egui::pos2(last.x + 4.0, last.y),
                            egui::Align2::LEFT_CENTER,
                            "aVWAP",
                            egui::FontId::monospace(9.0),
                            *color,
                        );
                    }
                }
            }
            Drawing::TrendChannel { p1, p2, p3, color } => {
                let to_x = |idx: usize| -> Option<f32> {
                    if idx >= start_idx && idx < end_idx {
                        Some(data_left + ((idx - start_idx) as f32 + 0.5) * bar_w)
                    } else {
                        None
                    }
                };
                if let (Some(x1), Some(x2)) = (to_x(p1.0), to_x(p2.0)) {
                    let y1 = price_to_y(p1.1);
                    let y2 = price_to_y(p2.1);
                    let ch_offset = p3.1 - p1.1;
                    let sc = sel_tint(*color);
                    // Main trendline
                    draw_styled_line(
                        &painter,
                        egui::pos2(x1, y1),
                        egui::pos2(x2, y2),
                        egui::Stroke::new(effective_width, sc),
                        d_style,
                    );
                    // Parallel line
                    let y1p = price_to_y(p1.1 + ch_offset);
                    let y2p = price_to_y(p2.1 + ch_offset);
                    draw_styled_line(
                        &painter,
                        egui::pos2(x1, y1p),
                        egui::pos2(x2, y2p),
                        egui::Stroke::new(effective_width, sc),
                        d_style,
                    );
                    // Mid line (dashed)
                    let y1m = price_to_y(p1.1 + ch_offset * 0.5);
                    let y2m = price_to_y(p2.1 + ch_offset * 0.5);
                    draw_styled_line(
                        &painter,
                        egui::pos2(x1, y1m),
                        egui::pos2(x2, y2m),
                        egui::Stroke::new(effective_width * 0.35, sc),
                        LineStyle::Dashed,
                    );
                    // Fill
                    let fill =
                        egui::Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 18);
                    let poly = vec![
                        egui::pos2(x1, y1),
                        egui::pos2(x2, y2),
                        egui::pos2(x2, y2p),
                        egui::pos2(x1, y1p),
                    ];
                    painter.add(egui::Shape::convex_polygon(poly, fill, egui::Stroke::NONE));
                }
            }
            Drawing::InsidePitchfork {
                pivot,
                p2,
                p3,
                color,
            } => {
                let to_pt = |idx: usize, price: f64| -> Option<egui::Pos2> {
                    if idx >= start_idx && idx < end_idx {
                        Some(egui::pos2(
                            data_left + ((idx - start_idx) as f32 + 0.5) * bar_w,
                            price_to_y(price),
                        ))
                    } else {
                        None
                    }
                };
                if let (Some(pv), Some(a), Some(b)) = (
                    to_pt(pivot.0, pivot.1),
                    to_pt(p2.0, p2.1),
                    to_pt(p3.0, p3.1),
                ) {
                    let sc = sel_tint(*color);
                    // Inside pitchfork: median from midpoint of p2-p3 through pivot, extended
                    let mid = egui::pos2((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
                    // Median line from pivot through midpoint, extended 2x
                    let dx = mid.x - pv.x;
                    let dy = mid.y - pv.y;
                    let ext = egui::pos2(pv.x + dx * 2.5, pv.y + dy * 2.5);
                    draw_styled_line(
                        &painter,
                        pv,
                        ext,
                        egui::Stroke::new(effective_width, sc),
                        d_style,
                    );
                    // Prongs from p2 and p3, parallel to median
                    let ext_a = egui::pos2(a.x + dx * 2.0, a.y + dy * 2.0);
                    let ext_b = egui::pos2(b.x + dx * 2.0, b.y + dy * 2.0);
                    draw_styled_line(
                        &painter,
                        a,
                        ext_a,
                        egui::Stroke::new(effective_width * 0.7, sc),
                        d_style,
                    );
                    draw_styled_line(
                        &painter,
                        b,
                        ext_b,
                        egui::Stroke::new(effective_width * 0.7, sc),
                        d_style,
                    );
                    // Connect pivot to p2 and p3
                    draw_styled_line(
                        &painter,
                        pv,
                        a,
                        egui::Stroke::new(effective_width * 0.4, sc),
                        d_style,
                    );
                    draw_styled_line(
                        &painter,
                        pv,
                        b,
                        egui::Stroke::new(effective_width * 0.4, sc),
                        d_style,
                    );
                }
            }
            Drawing::FibWedge { p1, p2, p3, color } => {
                let to_pt = |idx: usize, price: f64| -> Option<egui::Pos2> {
                    if idx >= start_idx && idx < end_idx {
                        Some(egui::pos2(
                            data_left + ((idx - start_idx) as f32 + 0.5) * bar_w,
                            price_to_y(price),
                        ))
                    } else {
                        None
                    }
                };
                if let (Some(a), Some(b), Some(c)) =
                    (to_pt(p1.0, p1.1), to_pt(p2.0, p2.1), to_pt(p3.0, p3.1))
                {
                    let sc = sel_tint(*color);
                    // Two converging trendlines: p1->p2 and p1->p3
                    draw_styled_line(
                        &painter,
                        a,
                        b,
                        egui::Stroke::new(effective_width, sc),
                        d_style,
                    );
                    draw_styled_line(
                        &painter,
                        a,
                        c,
                        egui::Stroke::new(effective_width, sc),
                        d_style,
                    );
                    // Fib levels between the two lines
                    let levels = [0.236, 0.382, 0.5, 0.618, 0.786];
                    let names = ["23.6%", "38.2%", "50%", "61.8%", "78.6%"];
                    for (i, &lvl) in levels.iter().enumerate() {
                        let lb = egui::pos2(
                            a.x + (b.x - a.x) * lvl as f32,
                            a.y + (b.y - a.y) * lvl as f32,
                        );
                        let lc = egui::pos2(
                            a.x + (c.x - a.x) * lvl as f32,
                            a.y + (c.y - a.y) * lvl as f32,
                        );
                        let alpha = if lvl == 0.5 { 140 } else { 80 };
                        let lc2 = egui::Color32::from_rgba_premultiplied(
                            color.r(),
                            color.g(),
                            color.b(),
                            alpha,
                        );
                        painter.line_segment([lb, lc], egui::Stroke::new(0.7, lc2));
                        painter.text(
                            egui::pos2(lc.x + 3.0, lc.y),
                            egui::Align2::LEFT_CENTER,
                            names[i],
                            egui::FontId::monospace(8.0),
                            lc2,
                        );
                    }
                    // Fill between the two lines
                    let fill =
                        egui::Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 12);
                    painter.add(egui::Shape::convex_polygon(
                        vec![a, b, c],
                        fill,
                        egui::Stroke::NONE,
                    ));
                }
            }
            Drawing::PriceNote { price, text, color } => {
                let y = price_to_y(*price);
                if y >= chart_rect.top() && y <= chart_rect.bottom() {
                    // Dashed horizontal line
                    let alpha_line =
                        egui::Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 80);
                    painter.line_segment(
                        [
                            egui::pos2(chart_rect.left(), y),
                            egui::pos2(chart_rect.right(), y),
                        ],
                        egui::Stroke::new(0.5, alpha_line),
                    );
                    // Text box
                    let pad = 4.0_f32;
                    let galley =
                        painter.layout_no_wrap(text.clone(), egui::FontId::monospace(10.0), *color);
                    let tw = galley.rect.width();
                    let th = galley.rect.height();
                    let box_rect = egui::Rect::from_min_size(
                        egui::pos2(chart_rect.left() + 10.0, y - th - pad * 2.0),
                        egui::vec2(tw + pad * 2.0, th + pad * 2.0),
                    );
                    let bg = egui::Color32::from_rgba_premultiplied(25, 20, 35, 230);
                    painter.rect_filled(box_rect, 3.0, bg);
                    painter.rect_stroke(
                        box_rect,
                        3.0,
                        egui::Stroke::new(1.0, *color),
                        egui::StrokeKind::Outside,
                    );
                    painter.galley(
                        egui::pos2(chart_rect.left() + 10.0 + pad, y - th - pad),
                        galley,
                        *color,
                    );
                    // Price badge
                    let label = format!("{:.5}", price);
                    painter.text(
                        egui::pos2(chart_rect.right() - 4.0, y - 2.0),
                        egui::Align2::RIGHT_BOTTOM,
                        &label,
                        egui::FontId::monospace(8.0),
                        *color,
                    );
                }
            }
            Drawing::MeasureTool { p1, p2, color } => {
                let x1o = if p1.0 >= start_idx && p1.0 < end_idx {
                    Some(data_left + ((p1.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                let x2o = if p2.0 >= start_idx && p2.0 < end_idx {
                    Some(data_left + ((p2.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                if let (Some(x1), Some(x2)) = (x1o, x2o) {
                    let y1 = price_to_y(p1.1);
                    let y2 = price_to_y(p2.1);
                    // Connecting line
                    let sc = sel_tint(*color);
                    draw_styled_line(
                        &painter,
                        egui::pos2(x1, y1),
                        egui::pos2(x2, y2),
                        egui::Stroke::new(effective_width, sc),
                        d_style,
                    );
                    // Compute measurements
                    let bars_count = if p2.0 > p1.0 {
                        p2.0 - p1.0
                    } else {
                        p1.0 - p2.0
                    };
                    let price_diff = p2.1 - p1.1;
                    let pct = if p1.1.abs() > 1e-10 {
                        (price_diff / p1.1) * 100.0
                    } else {
                        0.0
                    };
                    let dx = x2 - x1;
                    let dy = y2 - y1;
                    let angle_deg = if dx.abs() > 0.01 {
                        (dy / dx).atan().to_degrees()
                    } else {
                        90.0
                    };
                    // R:R placeholder (1:1 without SL/TP context)
                    let info = format!(
                        "{} bars | {:.5} | {:.2}% | {:.1}° | R:R 1:1",
                        bars_count, price_diff, pct, angle_deg
                    );
                    // Background box
                    let mid_x = (x1 + x2) / 2.0;
                    let mid_y = (y1 + y2) / 2.0;
                    let pad = 4.0_f32;
                    let galley = painter.layout_no_wrap(info, egui::FontId::monospace(9.0), *color);
                    let tw = galley.rect.width();
                    let th = galley.rect.height();
                    let box_rect = egui::Rect::from_min_size(
                        egui::pos2(mid_x - tw / 2.0 - pad, mid_y - th - pad * 2.0),
                        egui::vec2(tw + pad * 2.0, th + pad * 2.0),
                    );
                    let bg = egui::Color32::from_rgba_premultiplied(15, 15, 25, 220);
                    painter.rect_filled(box_rect, 3.0, bg);
                    painter.rect_stroke(
                        box_rect,
                        3.0,
                        egui::Stroke::new(1.0, *color),
                        egui::StrokeKind::Outside,
                    );
                    painter.galley(
                        egui::pos2(mid_x - tw / 2.0, mid_y - th - pad),
                        galley,
                        *color,
                    );
                    // Endpoint markers
                    painter.circle_filled(egui::pos2(x1, y1), 3.0, *color);
                    painter.circle_filled(egui::pos2(x2, y2), 3.0, *color);
                }
            }
            Drawing::AnchoredText {
                bar_idx,
                price,
                text,
                color,
            } => {
                if *bar_idx >= start_idx && *bar_idx < end_idx {
                    let x = data_left + ((*bar_idx - start_idx) as f32 + 0.5) * bar_w;
                    let y = price_to_y(*price);
                    painter.text(
                        egui::pos2(x, y),
                        egui::Align2::LEFT_BOTTOM,
                        text,
                        egui::FontId::monospace(11.0),
                        sel_tint(*color),
                    );
                }
            }
            Drawing::Comment {
                bar_idx,
                price,
                text,
                color,
            } => {
                if *bar_idx >= start_idx && *bar_idx < end_idx {
                    let x = data_left + ((*bar_idx - start_idx) as f32 + 0.5) * bar_w;
                    let y = price_to_y(*price);
                    let sc = sel_tint(*color);
                    let galley =
                        painter.layout_no_wrap(text.clone(), egui::FontId::monospace(9.0), sc);
                    let tw = galley.rect.width();
                    let th = galley.rect.height();
                    let pad = 3.0_f32;
                    let br = egui::Rect::from_min_size(
                        egui::pos2(x - pad, y - th - pad * 2.0),
                        egui::vec2(tw + pad * 2.0, th + pad * 2.0),
                    );
                    painter.rect_filled(
                        br,
                        2.0,
                        egui::Color32::from_rgba_premultiplied(20, 20, 30, 200),
                    );
                    painter.rect_stroke(
                        br,
                        2.0,
                        egui::Stroke::new(1.0, sc),
                        egui::StrokeKind::Outside,
                    );
                    painter.galley(egui::pos2(x, y - th - pad), galley, sc);
                }
            }
            Drawing::ArrowMarkerLeft {
                bar_idx,
                price,
                color,
            } => {
                if *bar_idx >= start_idx && *bar_idx < end_idx {
                    let x = data_left + ((*bar_idx - start_idx) as f32 + 0.5) * bar_w;
                    let y = price_to_y(*price);
                    let sc = sel_tint(*color);
                    let sz = 8.0_f32;
                    painter.add(egui::Shape::convex_polygon(
                        vec![
                            egui::pos2(x - sz, y),
                            egui::pos2(x + sz * 0.5, y - sz * 0.7),
                            egui::pos2(x + sz * 0.5, y + sz * 0.7),
                        ],
                        sc,
                        egui::Stroke::NONE,
                    ));
                }
            }
            Drawing::ArrowMarkerRight {
                bar_idx,
                price,
                color,
            } => {
                if *bar_idx >= start_idx && *bar_idx < end_idx {
                    let x = data_left + ((*bar_idx - start_idx) as f32 + 0.5) * bar_w;
                    let y = price_to_y(*price);
                    let sc = sel_tint(*color);
                    let sz = 8.0_f32;
                    painter.add(egui::Shape::convex_polygon(
                        vec![
                            egui::pos2(x + sz, y),
                            egui::pos2(x - sz * 0.5, y - sz * 0.7),
                            egui::pos2(x - sz * 0.5, y + sz * 0.7),
                        ],
                        sc,
                        egui::Stroke::NONE,
                    ));
                }
            }
            Drawing::Circle { p1, p2, color } => {
                if p1.0 >= start_idx && p1.0 < end_idx && p2.0 >= start_idx && p2.0 < end_idx {
                    let cx = data_left + ((p1.0 - start_idx) as f32 + 0.5) * bar_w;
                    let cy = price_to_y(p1.1);
                    let rx = data_left + ((p2.0 - start_idx) as f32 + 0.5) * bar_w;
                    let ry = price_to_y(p2.1);
                    let radius = ((rx - cx).powi(2) + (ry - cy).powi(2)).sqrt();
                    painter.circle_stroke(
                        egui::pos2(cx, cy),
                        radius,
                        egui::Stroke::new(effective_width, sel_tint(*color)),
                    );
                }
            }
            Drawing::PitchFan { p1, p2, color }
            | Drawing::TrendFibTime { p1, p2, color }
            | Drawing::GannSquare { p1, p2, color }
            | Drawing::GannSquareFixed { p1, p2, color }
            | Drawing::BarsPattern { p1, p2, color }
            | Drawing::Projection { p1, p2, color }
            | Drawing::DoubleCurve { p1, p2, color } => {
                if p1.0 >= start_idx && p1.0 < end_idx && p2.0 >= start_idx && p2.0 < end_idx {
                    let x1 = data_left + ((p1.0 - start_idx) as f32 + 0.5) * bar_w;
                    let y1 = price_to_y(p1.1);
                    let x2 = data_left + ((p2.0 - start_idx) as f32 + 0.5) * bar_w;
                    let y2 = price_to_y(p2.1);
                    let sc = sel_tint(*color);
                    draw_styled_line(
                        &painter,
                        egui::pos2(x1, y1),
                        egui::pos2(x2, y2),
                        egui::Stroke::new(effective_width, sc),
                        d_style,
                    );
                    painter.circle_filled(egui::pos2(x1, y1), 3.0, sc);
                    painter.circle_filled(egui::pos2(x2, y2), 3.0, sc);
                }
            }
            Drawing::TrianglePattern { points, color }
            | Drawing::ThreeDrives { points, color }
            | Drawing::ElliottDouble { points, color }
            | Drawing::AbcdPattern { points, color }
            | Drawing::CypherPattern { points, color }
            | Drawing::ElliottTriangle { points, color }
            | Drawing::ElliottTripleCombo { points, color } => {
                let labels: &[&str] = match drawing {
                    Drawing::TrianglePattern { .. } => &["A", "B", "C"],
                    Drawing::ThreeDrives { .. } => &["1", "2", "3"],
                    Drawing::ElliottDouble { .. } => &["W", "X", "Y"],
                    Drawing::AbcdPattern { .. } => &["A", "B", "C", "D"],
                    Drawing::CypherPattern { .. } => &["X", "A", "B", "C", "D"],
                    Drawing::ElliottTriangle { .. } => &["A", "B", "C", "D", "E"],
                    Drawing::ElliottTripleCombo { .. } => &["W", "X", "Y", "X", "Z"],
                    _ => &[],
                };
                let screen_pts: Vec<(f32, f32)> = points
                    .iter()
                    .filter(|(bi, _)| *bi >= start_idx && *bi < end_idx)
                    .map(|(bi, pr)| {
                        (
                            data_left + ((*bi - start_idx) as f32 + 0.5) * bar_w,
                            price_to_y(*pr),
                        )
                    })
                    .collect();
                let sc = sel_tint(*color);
                for w in screen_pts.windows(2) {
                    draw_styled_line(
                        &painter,
                        egui::pos2(w[0].0, w[0].1),
                        egui::pos2(w[1].0, w[1].1),
                        egui::Stroke::new(effective_width, sc),
                        d_style,
                    );
                }
                for (i, &(x, y)) in screen_pts.iter().enumerate() {
                    painter.circle_filled(egui::pos2(x, y), 3.0, sc);
                    if i < labels.len() {
                        painter.text(
                            egui::pos2(x, y - 12.0),
                            egui::Align2::CENTER_BOTTOM,
                            labels[i],
                            egui::FontId::monospace(10.0),
                            sc,
                        );
                    }
                }
            }
        }
    }

    // ── Drawing Preview (ghost line during placement) ─────────────────────
    // When a drawing tool is active and the user has placed the first point,
    // render a semi-transparent preview line/shape from the first point to the
    // current mouse position. This gives immediate visual feedback — the user
    // sees exactly what the drawing will look like before committing.
    if let Some(cross) = crosshair {
        let preview_color = egui::Color32::from_rgba_premultiplied(200, 200, 255, 120);
        let preview_stroke = egui::Stroke::new(1.5, preview_color);
        // Convert crosshair to bar/price
        let mouse_rel = ((cross.x - chart_rect.left()) / bar_w).max(0.0) as usize;
        let _mouse_bar = start_idx + mouse_rel.min(end_idx.saturating_sub(start_idx + 1));
        let mouse_price = {
            let frac = (cross.y - chart_rect.top()) / chart_rect.height();
            price_max - frac as f64 * (price_max - price_min)
        };
        let _ = mouse_price;

        // Helper: convert (bar_idx, price) to screen pos
        let to_screen = |bar: usize, price: f64| -> Option<egui::Pos2> {
            if bar >= start_idx && bar < end_idx {
                let x = data_left + ((bar - start_idx) as f32 + 0.5) * bar_w;
                let y = price_to_y(price);
                Some(egui::pos2(x, y))
            } else {
                None
            }
        };

        // Generic preview: extract first point from any P2 state, draw line to cursor.
        // Extract second point from any P3 state, draw P1→P2→cursor.
        // This covers all 70+ drawing types without naming every variant.
        let p1_data: Option<(usize, f64)> = {
            // Use debug format to extract bar1/price1 from any P2 variant
            let dm_str = format!("{:?}", draw_mode);
            if dm_str.contains("bar1:") && dm_str.contains("price1:") {
                // Parse bar1 and price1 from debug string
                let bar1 = dm_str
                    .split("bar1: ")
                    .nth(1)
                    .and_then(|s| s.split([',', ' ', '}']).next())
                    .and_then(|s| s.parse::<usize>().ok());
                let price1 = dm_str
                    .split("price1: ")
                    .nth(1)
                    .and_then(|s| s.split([',', ' ', '}']).next())
                    .and_then(|s| s.parse::<f64>().ok());
                bar1.zip(price1)
            } else {
                None
            }
        };
        let p2_data: Option<(usize, f64)> = {
            let dm_str = format!("{:?}", draw_mode);
            if dm_str.contains("bar2:") && dm_str.contains("price2:") {
                let bar2 = dm_str
                    .split("bar2: ")
                    .nth(1)
                    .and_then(|s| s.split([',', ' ', '}']).next())
                    .and_then(|s| s.parse::<usize>().ok());
                let price2 = dm_str
                    .split("price2: ")
                    .nth(1)
                    .and_then(|s| s.split([',', ' ', '}']).next())
                    .and_then(|s| s.parse::<f64>().ok());
                bar2.zip(price2)
            } else {
                None
            }
        };

        match draw_mode {
            DrawMode::PlacingHLine => {
                painter.line_segment(
                    [
                        egui::pos2(chart_rect.left(), cross.y),
                        egui::pos2(chart_rect.right(), cross.y),
                    ],
                    preview_stroke,
                );
            }
            DrawMode::PlacingVLine => {
                painter.line_segment(
                    [
                        egui::pos2(cross.x, chart_rect.top()),
                        egui::pos2(cross.x, chart_rect.bottom()),
                    ],
                    preview_stroke,
                );
            }
            DrawMode::PlacingHRay => {
                painter.line_segment(
                    [
                        egui::pos2(cross.x, cross.y),
                        egui::pos2(chart_rect.right(), cross.y),
                    ],
                    preview_stroke,
                );
            }
            DrawMode::PlacingCrossLine => {
                painter.line_segment(
                    [
                        egui::pos2(chart_rect.left(), cross.y),
                        egui::pos2(chart_rect.right(), cross.y),
                    ],
                    preview_stroke,
                );
                painter.line_segment(
                    [
                        egui::pos2(cross.x, chart_rect.top()),
                        egui::pos2(cross.x, chart_rect.bottom()),
                    ],
                    preview_stroke,
                );
            }
            DrawMode::None => {}
            _ => {
                // Generic preview for all P2 states (point 1 placed, drawing line to cursor)
                if let Some((bar1, price1)) = p1_data {
                    if let Some(p1) = to_screen(bar1, price1) {
                        if let Some((bar2, price2)) = p2_data {
                            // P3 state: show P1→P2 solid, P2→cursor ghost
                            if let Some(p2) = to_screen(bar2, price2) {
                                painter
                                    .line_segment([p1, p2], egui::Stroke::new(1.5, preview_color));
                                painter.line_segment(
                                    [p2, cross],
                                    egui::Stroke::new(
                                        1.0,
                                        egui::Color32::from_rgba_premultiplied(200, 200, 255, 80),
                                    ),
                                );
                                painter.circle_filled(p1, 4.0, preview_color);
                                painter.circle_filled(p2, 4.0, preview_color);
                                painter.circle_stroke(cross, 4.0, preview_stroke);
                            }
                        } else {
                            // P2 state: show P1→cursor ghost line
                            painter.line_segment([p1, cross], preview_stroke);
                            painter.circle_filled(p1, 4.0, preview_color);
                            painter.circle_stroke(cross, 4.0, preview_stroke);
                        }
                    }
                }
            }
        }
    }
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
        assert_eq!(
            super::format_axis_price_label("EXT", 0.0924),
            "EXT 0.0924"
        );
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
        use crate::app::types::Timeframe;
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
            vec!["Prev H1", "Prev H4", "Prev D", "Prev W", "Prev MN", "Cur D", "Cur W", "Cur MN"]
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
        names.insert("WOK".to_string(), "WORK Medical Technology Group".to_string());
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
