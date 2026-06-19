use super::*;

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
            Drawing::Pitchfork {
                pivot,
                p2,
                p3,
                color,
            } => {
                // Andrews Pitchfork: median line from pivot to midpoint(p2,p3), parallel upper/lower
                let to_x = |idx: usize| -> Option<f32> {
                    if idx >= start_idx && idx < end_idx {
                        Some(data_left + ((idx - start_idx) as f32 + 0.5) * bar_w)
                    } else {
                        None
                    }
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
                    if idx >= start_idx && idx < end_idx {
                        Some(data_left + ((idx - start_idx) as f32 + 0.5) * bar_w)
                    } else {
                        None
                    }
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
                            let c = egui::Color32::from_rgba_premultiplied(
                                sc.r(),
                                sc.g(),
                                sc.b(),
                                alpha,
                            );
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
                if origin.0 >= start_idx && origin.0 < end_idx {
                    let ox = data_left + ((origin.0 - start_idx) as f32 + 0.5) * bar_w;
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
                        let c =
                            egui::Color32::from_rgba_premultiplied(sc.r(), sc.g(), sc.b(), alpha);
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
                if entry.0 >= start_idx && entry.0 < end_idx {
                    let x = data_left + ((entry.0 - start_idx) as f32 + 0.5) * bar_w;
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
                if entry.0 >= start_idx && entry.0 < end_idx {
                    let x = data_left + ((entry.0 - start_idx) as f32 + 0.5) * bar_w;
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
            Drawing::TextLabel {
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
                        egui::Align2::CENTER_CENTER,
                        text,
                        egui::FontId::monospace(11.0),
                        *color,
                    );
                }
            }
            Drawing::ArrowMarker {
                bar_idx,
                price,
                is_up,
                color,
            } => {
                if *bar_idx >= start_idx && *bar_idx < end_idx {
                    let x = data_left + ((*bar_idx - start_idx) as f32 + 0.5) * bar_w;
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
            }
            Drawing::Ellipse { p1, p2, color } => {
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
                    if idx >= start_idx && idx < end_idx {
                        let x = data_left + ((idx - start_idx) as f32 + 0.5) * bar_w;
                        Some(egui::pos2(x, price_to_y(price)))
                    } else {
                        None
                    }
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
                    if idx >= start_idx && idx < end_idx {
                        Some(data_left + ((idx - start_idx) as f32 + 0.5) * bar_w)
                    } else {
                        None
                    }
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
                        let c =
                            egui::Color32::from_rgba_premultiplied(sc.r(), sc.g(), sc.b(), alpha);
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
                    if idx >= start_idx && idx < end_idx {
                        let x = data_left + ((idx - start_idx) as f32 + 0.5) * bar_w;
                        let alpha = if f <= 3 { 120 } else { 80 };
                        let sc = sel_tint(*color);
                        let c =
                            egui::Color32::from_rgba_premultiplied(sc.r(), sc.g(), sc.b(), alpha);
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
                    // Horizontal line from bar to right edge
                    let x_start = if *bar_idx >= start_idx && *bar_idx < end_idx {
                        data_left + ((*bar_idx - start_idx) as f32 + 0.5) * bar_w
                    } else if *bar_idx < start_idx {
                        chart_rect.left()
                    } else {
                        return true; // bar beyond visible range
                    };
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
                    let text_col = if (color.r() as u16 + color.g() as u16 + color.b() as u16) > 384
                    {
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
                    if idx >= start_idx && idx < end_idx {
                        Some(data_left + ((idx - start_idx) as f32 + 0.5) * bar_w)
                    } else {
                        None
                    }
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
                if *bar_idx >= start_idx && *bar_idx < end_idx {
                    let x = data_left + ((*bar_idx - start_idx) as f32 + 0.5) * bar_w;
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
                    if idx >= start_idx && idx < end_idx {
                        let x = data_left + ((idx - start_idx) as f32 + 0.5) * bar_w;
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
                if *bar_idx >= start_idx && *bar_idx < end_idx {
                    let x = data_left + ((*bar_idx - start_idx) as f32 + 0.5) * bar_w;
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
                        let x_start = if b1 >= start_idx && b1 < end_idx {
                            data_left + ((b1 - start_idx) as f32 + 0.5) * bar_w
                        } else {
                            chart_rect.left()
                        };
                        let x_end = if b2 >= start_idx && b2 < end_idx {
                            data_left + ((b2 - start_idx) as f32 + 0.5) * bar_w
                        } else {
                            chart_rect.right()
                        };
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
                        let fill = egui::Color32::from_rgba_premultiplied(
                            color.r(),
                            color.g(),
                            color.b(),
                            15,
                        );
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
                let x1o = if p1.0 >= start_idx && p1.0 < end_idx {
                    Some(data_left + ((p1.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                let x2o = if p2.0 >= start_idx && p2.0 < end_idx {
                    Some(data_left + ((p2.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
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
            Drawing::ElliottWave { points, color } => {
                let mut screen_pts: Vec<(f32, f32)> = Vec::new();
                for &(bi, pr) in points.iter() {
                    if bi >= start_idx && bi < end_idx {
                        let x = data_left + ((bi - start_idx) as f32 + 0.5) * bar_w;
                        let y = price_to_y(pr);
                        screen_pts.push((x, y));
                    }
                }
                let labels = ["1", "2", "3", "4", "5"];
                let sc = sel_tint(*color);
                for i in 0..screen_pts.len() {
                    if i + 1 < screen_pts.len() {
                        draw_styled_line(
                            &painter,
                            egui::pos2(screen_pts[i].0, screen_pts[i].1),
                            egui::pos2(screen_pts[i + 1].0, screen_pts[i + 1].1),
                            egui::Stroke::new(effective_width, sc),
                            d_style,
                        );
                    }
                    if i < labels.len() {
                        painter.text(
                            egui::pos2(screen_pts[i].0, screen_pts[i].1 - 10.0),
                            egui::Align2::CENTER_BOTTOM,
                            labels[i],
                            egui::FontId::monospace(11.0),
                            sc,
                        );
                    }
                }
            }
            Drawing::AbcCorrection { points, color } => {
                let mut screen_pts: Vec<(f32, f32)> = Vec::new();
                for &(bi, pr) in points.iter() {
                    if bi >= start_idx && bi < end_idx {
                        let x = data_left + ((bi - start_idx) as f32 + 0.5) * bar_w;
                        let y = price_to_y(pr);
                        screen_pts.push((x, y));
                    }
                }
                let labels = ["A", "B", "C"];
                let sc = sel_tint(*color);
                for i in 0..screen_pts.len() {
                    if i + 1 < screen_pts.len() {
                        draw_styled_line(
                            &painter,
                            egui::pos2(screen_pts[i].0, screen_pts[i].1),
                            egui::pos2(screen_pts[i + 1].0, screen_pts[i + 1].1),
                            egui::Stroke::new(effective_width, sc),
                            d_style,
                        );
                    }
                    if i < labels.len() {
                        painter.text(
                            egui::pos2(screen_pts[i].0, screen_pts[i].1 - 10.0),
                            egui::Align2::CENTER_BOTTOM,
                            labels[i],
                            egui::FontId::monospace(11.0),
                            sc,
                        );
                    }
                }
            }
            Drawing::DateRange { p1, p2 } => {
                let x1o = if p1.0 >= start_idx && p1.0 < end_idx {
                    Some(data_left + ((p1.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                let x2o = if p2.0 >= start_idx && p2.0 < end_idx {
                    Some(data_left + ((p2.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
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
                let x1o = if p1.0 >= start_idx && p1.0 < end_idx {
                    Some(data_left + ((p1.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                let x2o = if p2.0 >= start_idx && p2.0 < end_idx {
                    Some(data_left + ((p2.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
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
            Drawing::HeadShoulders { points, color } => {
                // 5 points: 0=LS bottom, 1=LS top, 2=Head top, 3=RS top, 4=RS bottom
                // Connect all in order, draw neckline between 0 and 4
                let mut screen_pts: Vec<(f32, f32)> = Vec::new();
                for &(bi, pr) in points.iter() {
                    if bi >= start_idx && bi < end_idx {
                        let x = data_left + ((bi - start_idx) as f32 + 0.5) * bar_w;
                        let y = price_to_y(pr);
                        screen_pts.push((x, y));
                    }
                }
                let labels = ["LS", "L", "H", "R", "RS"];
                let sc = sel_tint(*color);
                for i in 0..screen_pts.len() {
                    if i + 1 < screen_pts.len() {
                        draw_styled_line(
                            &painter,
                            egui::pos2(screen_pts[i].0, screen_pts[i].1),
                            egui::pos2(screen_pts[i + 1].0, screen_pts[i + 1].1),
                            egui::Stroke::new(effective_width, sc),
                            d_style,
                        );
                    }
                    if i < labels.len() {
                        painter.text(
                            egui::pos2(screen_pts[i].0, screen_pts[i].1 - 10.0),
                            egui::Align2::CENTER_BOTTOM,
                            labels[i],
                            egui::FontId::monospace(9.0),
                            sc,
                        );
                    }
                }
                // Neckline: dashed line between point 0 and point 4
                if screen_pts.len() >= 5 {
                    let nk_col =
                        egui::Color32::from_rgba_premultiplied(sc.r(), sc.g(), sc.b(), 150);
                    draw_styled_line(
                        &painter,
                        egui::pos2(screen_pts[0].0, screen_pts[0].1),
                        egui::pos2(screen_pts[4].0, screen_pts[4].1),
                        egui::Stroke::new(effective_width, nk_col),
                        LineStyle::Dashed,
                    );
                    painter.text(
                        egui::pos2(
                            (screen_pts[0].0 + screen_pts[4].0) / 2.0,
                            (screen_pts[0].1 + screen_pts[4].1) / 2.0 + 12.0,
                        ),
                        egui::Align2::CENTER_TOP,
                        "Neckline",
                        egui::FontId::monospace(9.0),
                        nk_col,
                    );
                }
            }
            Drawing::XabcdPattern { points, color } => {
                let mut screen_pts: Vec<(f32, f32)> = Vec::new();
                for &(bi, pr) in points.iter() {
                    if bi >= start_idx && bi < end_idx {
                        let x = data_left + ((bi - start_idx) as f32 + 0.5) * bar_w;
                        let y = price_to_y(pr);
                        screen_pts.push((x, y));
                    }
                }
                let labels = ["X", "A", "B", "C", "D"];
                let sc = sel_tint(*color);
                for i in 0..screen_pts.len() {
                    if i + 1 < screen_pts.len() {
                        draw_styled_line(
                            &painter,
                            egui::pos2(screen_pts[i].0, screen_pts[i].1),
                            egui::pos2(screen_pts[i + 1].0, screen_pts[i + 1].1),
                            egui::Stroke::new(effective_width, sc),
                            d_style,
                        );
                    }
                    if i < labels.len() {
                        painter.text(
                            egui::pos2(screen_pts[i].0, screen_pts[i].1 - 10.0),
                            egui::Align2::CENTER_BOTTOM,
                            labels[i],
                            egui::FontId::monospace(11.0),
                            sc,
                        );
                    }
                }
                // XA→BD dashed line (harmonic diagonal)
                if screen_pts.len() >= 5 {
                    let diag = egui::Color32::from_rgba_premultiplied(sc.r(), sc.g(), sc.b(), 80);
                    draw_styled_line(
                        &painter,
                        egui::pos2(screen_pts[0].0, screen_pts[0].1),
                        egui::pos2(screen_pts[3].0, screen_pts[3].1),
                        egui::Stroke::new(0.6, diag),
                        LineStyle::Dashed,
                    );
                    draw_styled_line(
                        &painter,
                        egui::pos2(screen_pts[1].0, screen_pts[1].1),
                        egui::pos2(screen_pts[4].0, screen_pts[4].1),
                        egui::Stroke::new(0.6, diag),
                        LineStyle::Dashed,
                    );
                }
            }
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
                    if b >= start_idx && b < end_idx {
                        let x = data_left + ((b - start_idx) as f32 + 0.5) * bar_w;
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
            Drawing::SessionBreak { bar_idx, color } => {
                if *bar_idx >= start_idx && *bar_idx < end_idx {
                    let x = data_left + ((*bar_idx - start_idx) as f32 + 0.5) * bar_w;
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
                        let last_close =
                            chart.bars.get(end_idx - 1).map(|b| b.close).unwrap_or(0.0);
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
                let risk_rect = egui::Rect::from_two_pos(
                    egui::pos2(entry_x, entry_y),
                    egui::pos2(right_x, stop_y),
                );
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
                    if b >= start_idx && b < end_idx {
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
                    if b >= start_idx && next_b < end_idx {
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
                let x1o = if p1.0 >= start_idx && p1.0 < end_idx {
                    Some(data_left + ((p1.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                let x2o = if p2.0 >= start_idx && p2.0 < end_idx {
                    Some(data_left + ((p2.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
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
            Drawing::TrianglePattern { points, color }
            | Drawing::ThreeDrives { points, color }
            | Drawing::ElliottDouble { points, color }
            | Drawing::AbcdPattern { points, color }
            | Drawing::CypherPattern { points, color }
            | Drawing::ElliottTriangle { points, color }
            | Drawing::ElliottTripleCombo { points, color } => {
                let labels: &[&str] = match drawing {
                    Drawing::TrianglePattern { .. } => &["A", "B", "C"],
                    Drawing::ThreeDrives { .. } => &["1", "2", "3"],
                    Drawing::ElliottDouble { .. } => &["W", "X", "Y"],
                    Drawing::AbcdPattern { .. } => &["A", "B", "C", "D"],
                    Drawing::CypherPattern { .. } => &["X", "A", "B", "C", "D"],
                    Drawing::ElliottTriangle { .. } => &["A", "B", "C", "D", "E"],
                    Drawing::ElliottTripleCombo { .. } => &["W", "X", "Y", "X", "Z"],
                    _ => &[],
                };
                let screen_pts: Vec<(f32, f32)> = points
                    .iter()
                    .filter(|(bi, _)| *bi >= start_idx && *bi < end_idx)
                    .map(|(bi, pr)| {
                        (
                            data_left + ((*bi - start_idx) as f32 + 0.5) * bar_w,
                            price_to_y(*pr),
                        )
                    })
                    .collect();
                let sc = sel_tint(*color);
                for w in screen_pts.windows(2) {
                    draw_styled_line(
                        &painter,
                        egui::pos2(w[0].0, w[0].1),
                        egui::pos2(w[1].0, w[1].1),
                        egui::Stroke::new(effective_width, sc),
                        d_style,
                    );
                }
                for (i, &(x, y)) in screen_pts.iter().enumerate() {
                    painter.circle_filled(egui::pos2(x, y), 3.0, sc);
                    if i < labels.len() {
                        painter.text(
                            egui::pos2(x, y - 12.0),
                            egui::Align2::CENTER_BOTTOM,
                            labels[i],
                            egui::FontId::monospace(10.0),
                            sc,
                        );
                    }
                }
            }
        }
    }

    false
}
