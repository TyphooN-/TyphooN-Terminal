use super::*;

pub(super) fn draw_geometric_label_annotation(
    painter: &egui::Painter,
    drawing: &Drawing,
    chart_rect: egui::Rect,
    data_left: f32,
    bar_w: f32,
    price_to_y: impl Fn(f64) -> f32,
    start_idx: usize,
    _end_idx: usize,
    effective_width: f32,
    d_style: LineStyle,
    is_selected: bool,
) -> Option<bool> {
    let sel_tint = |c: egui::Color32| tint_for_selection(c, is_selected);
    match drawing {
        Drawing::TextLabel {
            bar_idx,
            price,
            text,
            color,
        } => {
            let x = data_left + ((*bar_idx as i64 - start_idx as i64) as f32 + 0.5) * bar_w;
            let y = price_to_y(*price);
            painter.text(
                egui::pos2(x, y),
                egui::Align2::CENTER_CENTER,
                text,
                egui::FontId::monospace(11.0),
                *color,
            );
        }
        Drawing::ArrowMarker {
            bar_idx,
            price,
            is_up,
            color,
        } => {
            let x = data_left + ((*bar_idx as i64 - start_idx as i64) as f32 + 0.5) * bar_w;
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
        Drawing::Ellipse { p1, p2, color } => {
            let x1 = Some(data_left + ((p1.0 as i64 - start_idx as i64) as f32 + 0.5) * bar_w);
            let x2 = Some(data_left + ((p2.0 as i64 - start_idx as i64) as f32 + 0.5) * bar_w);
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
                let x = data_left + ((idx as i64 - start_idx as i64) as f32 + 0.5) * bar_w;
                Some(egui::pos2(x, price_to_y(price)))
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
            let x1 = Some(data_left + ((p1.0 as i64 - start_idx as i64) as f32 + 0.5) * bar_w);
            let x2 = Some(data_left + ((p2.0 as i64 - start_idx as i64) as f32 + 0.5) * bar_w);
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
            let x1 = Some(data_left + ((p1.0 as i64 - start_idx as i64) as f32 + 0.5) * bar_w);
            let x2 = Some(data_left + ((p2.0 as i64 - start_idx as i64) as f32 + 0.5) * bar_w);
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
                Some(data_left + ((idx as i64 - start_idx as i64) as f32 + 0.5) * bar_w)
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
                    let c = egui::Color32::from_rgba_premultiplied(sc.r(), sc.g(), sc.b(), alpha);
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
                {
                    let x = data_left + ((idx as i64 - start_idx as i64) as f32 + 0.5) * bar_w;
                    let alpha = if f <= 3 { 120 } else { 80 };
                    let sc = sel_tint(*color);
                    let c = egui::Color32::from_rgba_premultiplied(sc.r(), sc.g(), sc.b(), alpha);
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
                // Horizontal line from bar to right edge (signed mapping —
                // an off-screen anchor clips instead of aborting the whole
                // annotation pass, which the old `return Some(true)` did).
                let x_start =
                    data_left + ((*bar_idx as i64 - start_idx as i64) as f32 + 0.5) * bar_w;
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
                let text_col = if (color.r() as u16 + color.g() as u16 + color.b() as u16) > 384 {
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
                Some(data_left + ((idx as i64 - start_idx as i64) as f32 + 0.5) * bar_w)
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
            let x1 = Some(data_left + ((p1.0 as i64 - start_idx as i64) as f32 + 0.5) * bar_w);
            let x2 = Some(data_left + ((p2.0 as i64 - start_idx as i64) as f32 + 0.5) * bar_w);
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
            {
                let x = data_left + ((*bar_idx as i64 - start_idx as i64) as f32 + 0.5) * bar_w;
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
                {
                    let x = data_left + ((idx as i64 - start_idx as i64) as f32 + 0.5) * bar_w;
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
            {
                let x = data_left + ((*bar_idx as i64 - start_idx as i64) as f32 + 0.5) * bar_w;
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
        _ => return None,
    }
    Some(false)
}
