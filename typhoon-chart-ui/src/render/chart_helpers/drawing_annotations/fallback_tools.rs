use super::*;

pub(super) fn draw_fallback_annotation(
    painter: &egui::Painter,
    drawing: &Drawing,
    chart: &ChartState,
    data_left: f32,
    bar_w: f32,
    price_to_y: &impl Fn(f64) -> f32,
    start_idx: usize,
    _end_idx: usize,
    effective_width: f32,
    d_style: LineStyle,
    is_selected: bool,
) -> bool {
    let sel_tint = |c: egui::Color32| tint_for_selection(c, is_selected);
    match drawing {
        Drawing::Brush { points, color } => {
            for &(bi, pr) in points.iter() {
                {
                    let x = data_left + ((bi as i64 - start_idx as i64) as f32 + 0.5) * bar_w;
                    let y = price_to_y(pr);
                    painter.circle_filled(egui::pos2(x, y), 2.0, *color);
                }
            }
        }
        Drawing::Emoji {
            bar_idx,
            price,
            emoji,
        } => {
            let x = data_left + ((*bar_idx as i64 - start_idx as i64) as f32 + 0.5) * bar_w;
            let y = price_to_y(*price);
            painter.text(
                egui::pos2(x, y),
                egui::Align2::CENTER_CENTER,
                emoji,
                egui::FontId::proportional(16.0),
                egui::Color32::WHITE,
            );
        }
        Drawing::Flag {
            bar_idx,
            price,
            color,
        } => {
            {
                let x = data_left + ((*bar_idx as i64 - start_idx as i64) as f32 + 0.5) * bar_w;
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
                Some(data_left + ((b as i64 - start_idx as i64) as f32 + 0.5) * bar_w)
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
            {
                let x = data_left + ((*bar_idx as i64 - start_idx as i64) as f32 + 0.5) * bar_w;
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
            let fill = egui::Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 25);
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
                    {
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
        Drawing::AnchoredText {
            bar_idx,
            price,
            text,
            color,
        } => {
            let x = data_left + ((*bar_idx as i64 - start_idx as i64) as f32 + 0.5) * bar_w;
            let y = price_to_y(*price);
            painter.text(
                egui::pos2(x, y),
                egui::Align2::LEFT_BOTTOM,
                text,
                egui::FontId::monospace(11.0),
                sel_tint(*color),
            );
        }
        Drawing::Comment {
            bar_idx,
            price,
            text,
            color,
        } => {
            let x = data_left + ((*bar_idx as i64 - start_idx as i64) as f32 + 0.5) * bar_w;
            let y = price_to_y(*price);
            let sc = sel_tint(*color);
            let galley = painter.layout_no_wrap(text.clone(), egui::FontId::monospace(9.0), sc);
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
        Drawing::ArrowMarkerLeft {
            bar_idx,
            price,
            color,
        } => {
            let x = data_left + ((*bar_idx as i64 - start_idx as i64) as f32 + 0.5) * bar_w;
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
        Drawing::ArrowMarkerRight {
            bar_idx,
            price,
            color,
        } => {
            let x = data_left + ((*bar_idx as i64 - start_idx as i64) as f32 + 0.5) * bar_w;
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
        Drawing::Circle { p1, p2, color } => {
            let cx = data_left + ((p1.0 as i64 - start_idx as i64) as f32 + 0.5) * bar_w;
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
        Drawing::PitchFan { p1, p2, color }
        | Drawing::TrendFibTime { p1, p2, color }
        | Drawing::GannSquare { p1, p2, color }
        | Drawing::GannSquareFixed { p1, p2, color }
        | Drawing::BarsPattern { p1, p2, color }
        | Drawing::Projection { p1, p2, color }
        | Drawing::DoubleCurve { p1, p2, color } => {
            let x1 = data_left + ((p1.0 as i64 - start_idx as i64) as f32 + 0.5) * bar_w;
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
        _ => return false,
    }
    true
}
