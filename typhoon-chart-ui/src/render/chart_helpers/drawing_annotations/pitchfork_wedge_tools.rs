use super::*;

pub(super) fn draw_pitchfork_wedge_annotation(
    painter: &egui::Painter,
    drawing: &Drawing,
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
        _ => return false,
    }
    true
}
