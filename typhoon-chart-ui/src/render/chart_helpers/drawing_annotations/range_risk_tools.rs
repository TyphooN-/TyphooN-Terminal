use super::*;

pub(super) fn draw_range_risk_annotation(
    painter: &egui::Painter,
    drawing: &Drawing,
    chart: &ChartState,
    chart_rect: egui::Rect,
    data_left: f32,
    bar_w: f32,
    price_to_y: &impl Fn(f64) -> f32,
    start_idx: usize,
    end_idx: usize,
    effective_width: f32,
    d_style: LineStyle,
    is_selected: bool,
    format_price: &impl Fn(f64) -> String,
) -> bool {
    let sel_tint = |c: egui::Color32| tint_for_selection(c, is_selected);
    match drawing {
        Drawing::DateRange { p1, p2 } => {
            let x1o = Some(data_left + ((p1.0 as i64 - start_idx as i64) as f32 + 0.5) * bar_w);
            let x2o = Some(data_left + ((p2.0 as i64 - start_idx as i64) as f32 + 0.5) * bar_w);
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
            let x1o = Some(data_left + ((p1.0 as i64 - start_idx as i64) as f32 + 0.5) * bar_w);
            let x2o = Some(data_left + ((p2.0 as i64 - start_idx as i64) as f32 + 0.5) * bar_w);
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
                {
                    let x = data_left + ((b as i64 - start_idx as i64) as f32 + 0.5) * bar_w;
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
        Drawing::SessionBreak { bar_idx, color } => {
            {
                let x = data_left + ((*bar_idx as i64 - start_idx as i64) as f32 + 0.5) * bar_w;
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
                    let last_close = chart.bars.get(end_idx - 1).map(|b| b.close).unwrap_or(0.0);
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
            let risk_rect =
                egui::Rect::from_two_pos(egui::pos2(entry_x, entry_y), egui::pos2(right_x, stop_y));
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
                {
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
                {
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
            let x1o = Some(data_left + ((p1.0 as i64 - start_idx as i64) as f32 + 0.5) * bar_w);
            let x2o = Some(data_left + ((p2.0 as i64 - start_idx as i64) as f32 + 0.5) * bar_w);
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
        _ => return false,
    }
    true
}
