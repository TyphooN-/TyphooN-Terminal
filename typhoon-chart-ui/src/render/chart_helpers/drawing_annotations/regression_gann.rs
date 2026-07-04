use super::*;

pub(super) fn draw_regression_gann_annotation(
    painter: &egui::Painter,
    drawing: &Drawing,
    _chart_rect: egui::Rect,
    data_left: f32,
    bar_w: f32,
    price_to_y: impl Fn(f64) -> f32,
    start_idx: usize,
    end_idx: usize,
    bars: &[Bar],
    effective_width: f32,
    d_style: LineStyle,
    is_selected: bool,
) -> bool {
    let sel_tint = |c: egui::Color32| tint_for_selection(c, is_selected);
    match drawing {
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
                    let x_start = data_left + ((b1 as i64 - start_idx as i64) as f32 + 0.5) * bar_w;
                    let x_end = data_left + ((b2 as i64 - start_idx as i64) as f32 + 0.5) * bar_w;
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
                    let fill =
                        egui::Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 15);
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
            let x1o = Some(data_left + ((p1.0 as i64 - start_idx as i64) as f32 + 0.5) * bar_w);
            let x2o = Some(data_left + ((p2.0 as i64 - start_idx as i64) as f32 + 0.5) * bar_w);
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
        _ => return false,
    }
    true
}
