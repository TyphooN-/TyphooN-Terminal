use super::*;

/// Draw SL/TP planning lines, selected drawing handles, and compare-symbol overlay.
pub(crate) fn draw_planning_and_compare_overlays(
    painter: &egui::Painter,
    chart: &ChartState,
    chart_rect: egui::Rect,
    data_left: f32,
    bar_w: f32,
    bars: &[Bar],
    _start_idx: usize,
    _end_idx: usize,
    price_geometry: &crate::render::PriceViewGeometry,
    sl_price: Option<f64>,
    tp_price: Option<f64>,
    active_position_avg_price: Option<f64>,
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

                // Distance from live/current price, plus active position entry
                // when available. The SL/TP UI owns the action; this label keeps
                // chart lines self-explanatory without requiring the command
                // palette or right-panel fields.
                if let Some(last) = bars.last() {
                    let dist_label = planning_line_distance_label(
                        *p,
                        last.close,
                        active_position_avg_price,
                        &format_price,
                    );
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
    // Handles come from the same `drawing_anchors` registry the grab/resize
    // input path uses, mapped through the exact painted geometry — every
    // variant gets handles, and they sit precisely where the grab test looks.
    if let Some(sel) = chart.selected_drawing {
        if let Some(drawing) = chart.drawings.get(sel) {
            let cp_size = 4.0_f32; // half-size of control point square
            let cp_fill = egui::Color32::from_rgb(0, 200, 220);
            let cp_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
            for a in crate::drawing_interaction::drawing_anchors(drawing) {
                let p = a.to_screen(price_geometry);
                if chart_rect.expand(cp_size).contains(p) {
                    let r =
                        egui::Rect::from_center_size(p, egui::vec2(cp_size * 2.0, cp_size * 2.0));
                    painter.rect_filled(r, 0.0, cp_fill);
                    painter.rect_stroke(r, 0.0, cp_stroke, egui::StrokeKind::Outside);
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

fn signed_pct(from: f64, reference: f64) -> Option<f64> {
    (from.is_finite() && reference.is_finite() && reference > 0.0)
        .then_some((from - reference) / reference * 100.0)
}

fn signed_price_delta(price: f64, reference: f64, format_price: &impl Fn(f64) -> String) -> String {
    let dist = price - reference;
    if dist > 0.0 {
        format!("+{}", format_price(dist.abs()))
    } else if dist < 0.0 {
        format!("-{}", format_price(dist.abs()))
    } else {
        format!("±{}", format_price(0.0))
    }
}

fn signed_pct_text(pct: f64) -> String {
    if pct > 0.0 {
        format!("+{pct:.1}%")
    } else if pct < 0.0 {
        format!("{pct:.1}%")
    } else {
        "±0.0%".to_string()
    }
}

pub(crate) fn planning_line_distance_label(
    line_price: f64,
    current_price: f64,
    active_position_avg_price: Option<f64>,
    format_price: &impl Fn(f64) -> String,
) -> String {
    let mut label = signed_price_delta(line_price, current_price, format_price);
    if let Some(cur_pct) = signed_pct(line_price, current_price) {
        label.push_str(&format!(" {} cur", signed_pct_text(cur_pct)));
    }
    if let Some(avg_pct) = active_position_avg_price.and_then(|avg| signed_pct(line_price, avg)) {
        label.push_str(&format!(" | {} avg", signed_pct_text(avg_pct)));
    }
    label
}

#[cfg(test)]
mod tests {
    use super::planning_line_distance_label;

    #[test]
    fn planning_line_distance_label_includes_current_and_avg_percentages() {
        let label = planning_line_distance_label(95.0, 100.0, Some(90.0), &|p| format!("{p:.2}"));
        assert_eq!(label, "-5.00 -5.0% cur | +5.6% avg");
    }

    #[test]
    fn planning_line_distance_label_skips_missing_avg() {
        let label = planning_line_distance_label(105.0, 100.0, None, &|p| format!("{p:.2}"));
        assert_eq!(label, "+5.00 +5.0% cur");
    }
}
