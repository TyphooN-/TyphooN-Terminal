use super::*;

pub(super) fn draw_projection_curve_annotation(
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
) -> bool {
    let sel_tint = |c: egui::Color32| tint_for_selection(c, is_selected);
    match drawing {
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
                let bar_to_x =
                    |b: usize| -> f32 { data_left + ((b as f32 - start_idx as f32) + 0.5) * bar_w };
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
                    if src_idx < chart.bars.len() && dst_idx < chart.bars.len() + CHART_RIGHT_MARGIN
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
                        painter.line_segment([p, pt], egui::Stroke::new(effective_width * 0.7, sc));
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
        _ => return false,
    }
    true
}
