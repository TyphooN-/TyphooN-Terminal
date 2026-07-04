use super::*;

pub(super) fn draw_pattern_annotation(
    painter: &egui::Painter,
    drawing: &Drawing,
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
        Drawing::ElliottWave { points, color } => {
            let mut screen_pts: Vec<(f32, f32)> = Vec::new();
            for &(bi, pr) in points.iter() {
                {
                    let x = data_left + ((bi as i64 - start_idx as i64) as f32 + 0.5) * bar_w;
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
                {
                    let x = data_left + ((bi as i64 - start_idx as i64) as f32 + 0.5) * bar_w;
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
        Drawing::HeadShoulders { points, color } => {
            // 5 points: 0=LS bottom, 1=LS top, 2=Head top, 3=RS top, 4=RS bottom
            // Connect all in order, draw neckline between 0 and 4
            let mut screen_pts: Vec<(f32, f32)> = Vec::new();
            for &(bi, pr) in points.iter() {
                {
                    let x = data_left + ((bi as i64 - start_idx as i64) as f32 + 0.5) * bar_w;
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
                let nk_col = egui::Color32::from_rgba_premultiplied(sc.r(), sc.g(), sc.b(), 150);
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
                {
                    let x = data_left + ((bi as i64 - start_idx as i64) as f32 + 0.5) * bar_w;
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
                .map(|(bi, pr)| {
                    (
                        data_left + ((*bi as i64 - start_idx as i64) as f32 + 0.5) * bar_w,
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
        _ => return false,
    }
    true
}
