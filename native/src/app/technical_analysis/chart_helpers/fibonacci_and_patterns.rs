use super::*;

/// Draw Auto Fibonacci levels (retrace + extensions) and the swing line.
/// Extracted to chart_helpers.rs to shrink the main draw_chart.
pub(crate) fn draw_auto_fib_levels(
    painter: &egui::Painter,
    chart: &ChartState,
    chart_rect: egui::Rect,
    data_left: f32,
    bar_w: f32,
    price_to_y: impl Fn(f64) -> f32,
    start_idx: usize,
    end_idx: usize,
    format_price: impl Fn(f64) -> String,
) {
    for (price, label, is_ext) in &chart.auto_fib_levels {
        let y = price_to_y(*price);
        if y >= chart_rect.top() && y <= chart_rect.bottom() {
            let color = if *is_ext {
                egui::Color32::from_rgb(30, 144, 255)
            } else {
                egui::Color32::from_rgb(255, 215, 0)
            };
            painter.line_segment(
                [
                    egui::pos2(chart_rect.left(), y),
                    egui::pos2(chart_rect.right(), y),
                ],
                egui::Stroke::new(1.0, color),
            );
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
/// Draw harmonic (XABCD) patterns: lines, point labels, TP/SL.
/// Extracted from draw_chart (technical_analysis.rs) for modularity.
pub(crate) fn draw_harmonics(
    painter: &egui::Painter,
    chart: &ChartState,
    chart_rect: egui::Rect,
    data_left: f32,
    bar_w: f32,
    price_to_y: impl Fn(f64) -> f32,
    start_idx: usize,
    end_idx: usize,
    format_price: impl Fn(f64) -> String,
) {
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
