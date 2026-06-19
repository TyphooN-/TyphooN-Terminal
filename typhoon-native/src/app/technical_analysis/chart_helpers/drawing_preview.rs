use super::*;

/// Draw the ghost/preview shape while a drawing tool is in placement mode.
pub(crate) fn draw_drawing_preview(
    painter: &egui::Painter,
    chart_rect: egui::Rect,
    data_left: f32,
    bar_w: f32,
    start_idx: usize,
    end_idx: usize,
    price_min: f64,
    price_max: f64,
    crosshair: Option<egui::Pos2>,
    draw_mode: &DrawMode,
    price_to_y: impl Fn(f64) -> f32,
) {
    // ── Drawing Preview (ghost line during placement) ─────────────────────
    // When a drawing tool is active and the user has placed the first point,
    // render a semi-transparent preview line/shape from the first point to the
    // current mouse position. This gives immediate visual feedback — the user
    // sees exactly what the drawing will look like before committing.
    if let Some(cross) = crosshair {
        let preview_color = egui::Color32::from_rgba_premultiplied(200, 200, 255, 120);
        let preview_stroke = egui::Stroke::new(1.5, preview_color);
        // Convert crosshair to bar/price
        let mouse_rel = ((cross.x - chart_rect.left()) / bar_w).max(0.0) as usize;
        let _mouse_bar = start_idx + mouse_rel.min(end_idx.saturating_sub(start_idx + 1));
        let mouse_price = {
            let frac = (cross.y - chart_rect.top()) / chart_rect.height();
            price_max - frac as f64 * (price_max - price_min)
        };
        let _ = mouse_price;

        // Helper: convert (bar_idx, price) to screen pos
        let to_screen = |bar: usize, price: f64| -> Option<egui::Pos2> {
            if bar >= start_idx && bar < end_idx {
                let x = data_left + ((bar - start_idx) as f32 + 0.5) * bar_w;
                let y = price_to_y(price);
                Some(egui::pos2(x, y))
            } else {
                None
            }
        };

        // Generic preview: extract first point from any P2 state, draw line to cursor.
        // Extract second point from any P3 state, draw P1→P2→cursor.
        // This covers all 70+ drawing types without naming every variant.
        let p1_data: Option<(usize, f64)> = {
            // Use debug format to extract bar1/price1 from any P2 variant
            let dm_str = format!("{:?}", draw_mode);
            if dm_str.contains("bar1:") && dm_str.contains("price1:") {
                // Parse bar1 and price1 from debug string
                let bar1 = dm_str
                    .split("bar1: ")
                    .nth(1)
                    .and_then(|s| s.split([',', ' ', '}']).next())
                    .and_then(|s| s.parse::<usize>().ok());
                let price1 = dm_str
                    .split("price1: ")
                    .nth(1)
                    .and_then(|s| s.split([',', ' ', '}']).next())
                    .and_then(|s| s.parse::<f64>().ok());
                bar1.zip(price1)
            } else {
                None
            }
        };
        let p2_data: Option<(usize, f64)> = {
            let dm_str = format!("{:?}", draw_mode);
            if dm_str.contains("bar2:") && dm_str.contains("price2:") {
                let bar2 = dm_str
                    .split("bar2: ")
                    .nth(1)
                    .and_then(|s| s.split([',', ' ', '}']).next())
                    .and_then(|s| s.parse::<usize>().ok());
                let price2 = dm_str
                    .split("price2: ")
                    .nth(1)
                    .and_then(|s| s.split([',', ' ', '}']).next())
                    .and_then(|s| s.parse::<f64>().ok());
                bar2.zip(price2)
            } else {
                None
            }
        };

        match draw_mode {
            DrawMode::PlacingHLine => {
                painter.line_segment(
                    [
                        egui::pos2(chart_rect.left(), cross.y),
                        egui::pos2(chart_rect.right(), cross.y),
                    ],
                    preview_stroke,
                );
            }
            DrawMode::PlacingVLine => {
                painter.line_segment(
                    [
                        egui::pos2(cross.x, chart_rect.top()),
                        egui::pos2(cross.x, chart_rect.bottom()),
                    ],
                    preview_stroke,
                );
            }
            DrawMode::PlacingHRay => {
                painter.line_segment(
                    [
                        egui::pos2(cross.x, cross.y),
                        egui::pos2(chart_rect.right(), cross.y),
                    ],
                    preview_stroke,
                );
            }
            DrawMode::PlacingCrossLine => {
                painter.line_segment(
                    [
                        egui::pos2(chart_rect.left(), cross.y),
                        egui::pos2(chart_rect.right(), cross.y),
                    ],
                    preview_stroke,
                );
                painter.line_segment(
                    [
                        egui::pos2(cross.x, chart_rect.top()),
                        egui::pos2(cross.x, chart_rect.bottom()),
                    ],
                    preview_stroke,
                );
            }
            DrawMode::None => {}
            _ => {
                // Generic preview for all P2 states (point 1 placed, drawing line to cursor)
                if let Some((bar1, price1)) = p1_data {
                    if let Some(p1) = to_screen(bar1, price1) {
                        if let Some((bar2, price2)) = p2_data {
                            // P3 state: show P1→P2 solid, P2→cursor ghost
                            if let Some(p2) = to_screen(bar2, price2) {
                                painter
                                    .line_segment([p1, p2], egui::Stroke::new(1.5, preview_color));
                                painter.line_segment(
                                    [p2, cross],
                                    egui::Stroke::new(
                                        1.0,
                                        egui::Color32::from_rgba_premultiplied(200, 200, 255, 80),
                                    ),
                                );
                                painter.circle_filled(p1, 4.0, preview_color);
                                painter.circle_filled(p2, 4.0, preview_color);
                                painter.circle_stroke(cross, 4.0, preview_stroke);
                            }
                        } else {
                            // P2 state: show P1→cursor ghost line
                            painter.line_segment([p1, cross], preview_stroke);
                            painter.circle_filled(p1, 4.0, preview_color);
                            painter.circle_stroke(cross, 4.0, preview_stroke);
                        }
                    }
                }
            }
        }
    }
}
