use super::*;

pub(super) fn draw_measurement_annotation(
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
) -> bool {
    let sel_tint = |c: egui::Color32| tint_for_selection(c, is_selected);
    match drawing {
        Drawing::Pitchfork {
            pivot,
            p2,
            p3,
            color,
        } => {
            // Andrews Pitchfork: median line from pivot to midpoint(p2,p3), parallel upper/lower
            let to_x = |idx: usize| -> Option<f32> {
                Some(data_left + ((idx as i64 - start_idx as i64) as f32 + 0.5) * bar_w)
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
                Some(data_left + ((idx as i64 - start_idx as i64) as f32 + 0.5) * bar_w)
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
                        let c =
                            egui::Color32::from_rgba_premultiplied(sc.r(), sc.g(), sc.b(), alpha);
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
            {
                let ox = data_left + ((origin.0 as i64 - start_idx as i64) as f32 + 0.5) * bar_w;
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
                    let c = egui::Color32::from_rgba_premultiplied(sc.r(), sc.g(), sc.b(), alpha);
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
            {
                let x = data_left + ((entry.0 as i64 - start_idx as i64) as f32 + 0.5) * bar_w;
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
            {
                let x = data_left + ((entry.0 as i64 - start_idx as i64) as f32 + 0.5) * bar_w;
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
            let x1 = Some(data_left + ((p1.0 as i64 - start_idx as i64) as f32 + 0.5) * bar_w);
            let x2 = Some(data_left + ((p2.0 as i64 - start_idx as i64) as f32 + 0.5) * bar_w);
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
        _ => return false,
    }
    true
}
