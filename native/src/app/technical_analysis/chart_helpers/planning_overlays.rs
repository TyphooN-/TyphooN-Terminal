use super::*;

/// Draw SL/TP planning lines, selected drawing handles, and compare-symbol overlay.
pub(crate) fn draw_planning_and_compare_overlays(
    painter: &egui::Painter,
    chart: &ChartState,
    chart_rect: egui::Rect,
    data_left: f32,
    bar_w: f32,
    bars: &[Bar],
    start_idx: usize,
    end_idx: usize,
    price_min: f64,
    price_max: f64,
    sl_price: Option<f64>,
    tp_price: Option<f64>,
    price_to_y: impl Fn(f64) -> f32,
    format_price: impl Fn(f64) -> String,
) {
    // ── SL/TP planning lines ───────────────────────────────────────────────
    for (price_opt, label, color) in [
        (&sl_price, "SL", egui::Color32::from_rgb(220, 40, 40)),
        (&tp_price, "TP", egui::Color32::from_rgb(0, 200, 80)),
    ] {
        if let Some(p) = price_opt {
            let y = price_to_y(*p);
            if y >= chart_rect.top() && y <= chart_rect.bottom() {
                let shadow = egui::Color32::from_rgba_premultiplied(0, 0, 0, 190);
                let band =
                    egui::Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 36);
                painter.rect_filled(
                    egui::Rect::from_min_max(
                        egui::pos2(chart_rect.left(), y - 5.0),
                        egui::pos2(chart_rect.right(), y + 5.0),
                    ),
                    0.0,
                    band,
                );
                painter.line_segment(
                    [
                        egui::pos2(chart_rect.left(), y),
                        egui::pos2(chart_rect.right(), y),
                    ],
                    egui::Stroke::new(6.0, shadow),
                );
                painter.line_segment(
                    [
                        egui::pos2(chart_rect.left(), y),
                        egui::pos2(chart_rect.right(), y),
                    ],
                    egui::Stroke::new(3.0, color),
                );

                let pad_x = 6.0_f32;
                let pad_y = 3.0_f32;
                let price_text = format!("{} {}", label, format_price(*p));
                let price_galley = painter.layout_no_wrap(
                    price_text,
                    egui::FontId::monospace(11.0),
                    egui::Color32::BLACK,
                );
                let price_rect = egui::Rect::from_min_size(
                    egui::pos2(
                        chart_rect.left() + 8.0,
                        y - price_galley.rect.height() * 0.5 - pad_y,
                    ),
                    egui::vec2(
                        price_galley.rect.width() + pad_x * 2.0,
                        price_galley.rect.height() + pad_y * 2.0,
                    ),
                );
                painter.rect_filled(price_rect, 3.0, color);
                painter.rect_stroke(
                    price_rect,
                    3.0,
                    egui::Stroke::new(1.0, shadow),
                    egui::StrokeKind::Outside,
                );
                painter.galley(
                    egui::pos2(
                        price_rect.left() + pad_x,
                        price_rect.center().y - price_galley.rect.height() * 0.5,
                    ),
                    price_galley,
                    egui::Color32::BLACK,
                );

                // P&L from last price
                if let Some(last) = bars.last() {
                    let dist = *p - last.close;
                    let dist_label = if dist > 0.0 {
                        format!("+{}", format_price(dist.abs()))
                    } else if dist < 0.0 {
                        format!("-{}", format_price(dist.abs()))
                    } else {
                        format!("±{}", format_price(0.0))
                    };
                    let dist_galley = painter.layout_no_wrap(
                        dist_label,
                        egui::FontId::monospace(10.0),
                        egui::Color32::BLACK,
                    );
                    let dist_rect = egui::Rect::from_min_size(
                        egui::pos2(
                            chart_rect.right() - dist_galley.rect.width() - 26.0,
                            y - dist_galley.rect.height() * 0.5 - pad_y,
                        ),
                        egui::vec2(
                            dist_galley.rect.width() + pad_x * 2.0,
                            dist_galley.rect.height() + pad_y * 2.0,
                        ),
                    );
                    painter.rect_filled(
                        dist_rect,
                        3.0,
                        egui::Color32::from_rgba_premultiplied(
                            color.r(),
                            color.g(),
                            color.b(),
                            220,
                        ),
                    );
                    painter.rect_stroke(
                        dist_rect,
                        3.0,
                        egui::Stroke::new(1.0, shadow),
                        egui::StrokeKind::Outside,
                    );
                    painter.galley(
                        egui::pos2(
                            dist_rect.left() + pad_x,
                            dist_rect.center().y - dist_galley.rect.height() * 0.5,
                        ),
                        dist_galley,
                        egui::Color32::BLACK,
                    );
                }
            }
        }
    }

    // ── Drawing control points (drag handles when selected) ────────────────
    if let Some(sel) = chart.selected_drawing {
        if let Some(drawing) = chart.drawings.get(sel) {
            let cp_size = 4.0_f32; // half-size of control point square
            let cp_fill = egui::Color32::from_rgb(0, 200, 220);
            let cp_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
            // Collect control points as (bar_idx, price)
            let mut cps: Vec<(usize, f64)> = Vec::new();
            match drawing {
                Drawing::HLine { price, .. } => {
                    cps.push((start_idx, *price));
                    cps.push((end_idx.saturating_sub(1), *price));
                }
                Drawing::VLine { bar_idx, .. } => {
                    cps.push((*bar_idx, price_max));
                    cps.push((*bar_idx, price_min));
                }
                Drawing::TrendLine { p1, p2, .. }
                | Drawing::ExtendedLine { p1, p2, .. }
                | Drawing::ArrowLine { p1, p2, .. }
                | Drawing::InfoLine { p1, p2, .. }
                | Drawing::TrendAngle { p1, p2, .. }
                | Drawing::Rectangle { p1, p2, .. }
                | Drawing::Highlighter { p1, p2, .. }
                | Drawing::Ruler { p1, p2, .. }
                | Drawing::MeasureTool { p1, p2, .. }
                | Drawing::Forecast { p1, p2, .. }
                | Drawing::Ellipse { p1, p2, .. }
                | Drawing::SineWave { p1, p2, .. } => {
                    cps.push(*p1);
                    cps.push(*p2);
                }
                Drawing::Pitchfork { pivot, p2, p3, .. }
                | Drawing::SchiffPitchfork { pivot, p2, p3, .. }
                | Drawing::ModSchiffPitchfork { pivot, p2, p3, .. }
                | Drawing::InsidePitchfork { pivot, p2, p3, .. } => {
                    cps.push(*pivot);
                    cps.push(*p2);
                    cps.push(*p3);
                }
                Drawing::FiboExtension { p1, p2, p3, .. }
                | Drawing::FibChannel { p1, p2, p3, .. }
                | Drawing::TrendChannel { p1, p2, p3, .. }
                | Drawing::ArcDraw { p1, p2, p3, .. }
                | Drawing::Triangle { p1, p2, p3, .. }
                | Drawing::RotatedRectangle { p1, p2, p3, .. } => {
                    cps.push(*p1);
                    cps.push(*p2);
                    cps.push(*p3);
                }
                Drawing::Polyline { points, .. }
                | Drawing::ElliottWave { points, .. }
                | Drawing::AbcCorrection { points, .. }
                | Drawing::HeadShoulders { points, .. }
                | Drawing::XabcdPattern { points, .. }
                | Drawing::PathDraw { points, .. } => {
                    for pt in points {
                        cps.push(*pt);
                    }
                }
                _ => {} // single-point tools: no resize handles needed
            }
            for (bi, pr) in &cps {
                if *bi >= start_idx && *bi < end_idx {
                    let x = data_left + ((*bi - start_idx) as f32 + 0.5) * bar_w;
                    let y = price_to_y(*pr);
                    if y >= chart_rect.top() && y <= chart_rect.bottom() {
                        let r = egui::Rect::from_center_size(
                            egui::pos2(x, y),
                            egui::vec2(cp_size * 2.0, cp_size * 2.0),
                        );
                        painter.rect_filled(r, 0.0, cp_fill);
                        painter.rect_stroke(r, 0.0, cp_stroke, egui::StrokeKind::Outside);
                    }
                }
            }
        }
    }

    // ── Compare symbol overlay (% change line) ──────────────────────────
    if let Some(ref _cmp_sym) = chart.compare_symbol {
        if !chart.compare_bars.is_empty() && bars.len() > 1 {
            let cmp = &chart.compare_bars;
            let (start_idx, _end_idx) = chart.visible_range();
            let base_close = chart.bars.get(start_idx).map(|b| b.close).unwrap_or(1.0);
            let cmp_base = cmp
                .get(start_idx.min(cmp.len().saturating_sub(1)))
                .map(|b| b.close)
                .unwrap_or(1.0);
            if base_close > 0.0 && cmp_base > 0.0 {
                let cmp_col = egui::Color32::from_rgb(200, 100, 255); // purple overlay
                let mut prev_pt: Option<egui::Pos2> = None;
                for rel_idx in 0..bars.len() {
                    let abs_idx = start_idx + rel_idx;
                    if abs_idx >= cmp.len() {
                        break;
                    }
                    let cmp_pct = (cmp[abs_idx].close - cmp_base) / cmp_base;
                    let mapped_price = base_close * (1.0 + cmp_pct);
                    let x = data_left + (rel_idx as f32 + 0.5) * bar_w;
                    let y = price_to_y(mapped_price);
                    let pt = egui::pos2(
                        x,
                        clamp_f32_bounds(y, chart_rect.top(), chart_rect.bottom()),
                    );
                    if let Some(pp) = prev_pt {
                        painter.line_segment([pp, pt], egui::Stroke::new(1.5, cmp_col));
                    }
                    prev_pt = Some(pt);
                }
            }
        }
    }
}
