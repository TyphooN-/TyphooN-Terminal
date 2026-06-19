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
mod projection_curve_tools;
use projection_curve_tools::draw_projection_curve_annotation;
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
        if draw_projection_curve_annotation(
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
