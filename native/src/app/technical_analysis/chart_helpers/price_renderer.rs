use super::*;

/// Draw core chart price bars (line, OHLC, candle/HA/Renko).
pub(crate) fn draw_price_bars(
    painter: &egui::Painter,
    chart: &ChartState,
    chart_rect: egui::Rect,
    data_left: f32,
    bar_w: f32,
    candle_w: f32,
    half_body: f32,
    price_to_y: impl Fn(f64) -> f32,
    bars: &[Bar],
    flags: &IndicatorFlags,
    start_idx: usize,
    render_step: usize,
) {
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
}
