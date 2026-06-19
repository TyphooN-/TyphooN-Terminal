use super::*;

pub(super) fn draw_basic_line_annotation(
    painter: &egui::Painter,
    drawing: &Drawing,
    chart_rect: egui::Rect,
    data_left: f32,
    bar_w: f32,
    price_to_y: impl Fn(f64) -> f32,
    start_idx: usize,
    end_idx: usize,
    effective_width: f32,
    d_style: LineStyle,
    is_selected: bool,
    format_price: impl Fn(f64) -> String,
) -> bool {
    let sel_tint = |c: egui::Color32| tint_for_selection(c, is_selected);
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
        _ => return false,
    }
    true
}
