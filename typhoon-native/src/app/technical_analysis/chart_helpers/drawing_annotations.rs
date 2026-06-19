use super::*;

mod basic_shapes;
use basic_shapes::draw_basic_line_annotation;
mod measurement_tools;
use measurement_tools::draw_measurement_annotation;
mod geometric_labels;
use geometric_labels::draw_geometric_label_annotation;
mod regression_gann;
use regression_gann::draw_regression_gann_annotation;
mod pattern_tools;
use pattern_tools::draw_pattern_annotation;
mod range_risk_tools;
use range_risk_tools::draw_range_risk_annotation;
/// Draw all persisted drawing annotations.
/// Returns true when legacy draw_chart control flow should return early.
pub(crate) fn draw_drawing_annotations(
    painter: &egui::Painter,
    chart: &ChartState,
    chart_rect: egui::Rect,
    data_left: f32,
    bar_w: f32,
    price_to_y: impl Fn(f64) -> f32,
    start_idx: usize,
    end_idx: usize,
    bars: &[Bar],
    format_price: impl Fn(f64) -> String,
) -> bool {
    for (draw_idx, drawing) in chart.drawings.iter().enumerate() {
        let (effective_width, d_style) = effective_drawing_width_and_style(chart, draw_idx);
        let is_selected = is_drawing_selected(chart, draw_idx);
        if draw_basic_line_annotation(
            painter,
            drawing,
            chart_rect,
            data_left,
            bar_w,
            &price_to_y,
            start_idx,
            end_idx,
            effective_width,
            d_style,
            is_selected,
            &format_price,
        ) {
            continue;
        }
        if draw_measurement_annotation(
            painter,
            drawing,
            chart_rect,
            data_left,
            bar_w,
            &price_to_y,
            start_idx,
            end_idx,
            effective_width,
            d_style,
            is_selected,
        ) {
            continue;
        }
        if let Some(should_return) = draw_geometric_label_annotation(
            painter,
            drawing,
            chart_rect,
            data_left,
            bar_w,
            &price_to_y,
            start_idx,
            end_idx,
            effective_width,
            d_style,
            is_selected,
        ) {
            if should_return {
                return true;
            }
            continue;
        }
        if draw_regression_gann_annotation(
            painter,
            drawing,
            chart_rect,
            data_left,
            bar_w,
            &price_to_y,
            start_idx,
            end_idx,
            bars,
            effective_width,
            d_style,
            is_selected,
        ) {
            continue;
        }
        if draw_pattern_annotation(
            painter,
            drawing,
            data_left,
            bar_w,
            &price_to_y,
            start_idx,
            end_idx,
            effective_width,
            d_style,
            is_selected,
        ) {
            continue;
        }
        if draw_range_risk_annotation(
            painter,
            drawing,
            chart,
            chart_rect,
            data_left,
            bar_w,
            &price_to_y,
            start_idx,
            end_idx,
            effective_width,
            d_style,
            is_selected,
            &format_price,
        ) {
            continue;
        }
        let sel_tint = |c: egui::Color32| tint_for_selection(c, is_selected);
        match drawing {
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
            _ => {}
        }
    }

    false
}
