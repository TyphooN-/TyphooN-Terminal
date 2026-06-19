use super::*;

/// Redraw primary NNFX trend overlays after translucent zones/volume profile.
pub(crate) fn draw_post_zone_trend_overlays(
    painter: &egui::Painter,
    chart: &ChartState,
    chart_rect: egui::Rect,
    data_left: f32,
    bar_w: f32,
    price_to_y: impl Fn(f64) -> f32,
    flags: &IndicatorFlags,
    start_idx: usize,
    end_idx: usize,
) {
    // Redraw primary NNFX trend overlays after translucent zones/volume profile so
    // Supply/Demand rectangles cannot bury the MultiKAMA and MTF_MA levels.
    if flags.sma200 && !chart.mtf_sma.is_empty() {
        for (label, projected) in &chart.mtf_sma {
            let color = match label.as_str() {
                "H1 200" => egui::Color32::from_rgb(255, 99, 71),
                _ => egui::Color32::from_rgb(255, 0, 255),
            };
            let mut prev: Option<egui::Pos2> = None;
            for &(bar_idx, sma_val) in projected {
                if bar_idx >= start_idx && bar_idx < end_idx {
                    let rel = bar_idx - start_idx;
                    let x = data_left + (rel as f32 + 0.5) * bar_w;
                    let pt = egui::pos2(x, price_to_y(sma_val));
                    if let Some(prev_pt) = prev {
                        if let Some([a, b]) = clip_line_segment_to_rect(prev_pt, pt, chart_rect) {
                            painter.line_segment([a, b], egui::Stroke::new(2.25, color));
                        }
                    }
                    prev = Some(pt);
                }
            }
        }
    }
    if flags.kama && !chart.multi_kama.is_empty() {
        for (_tf_label, projected) in &chart.multi_kama {
            let mut prev: Option<egui::Pos2> = None;
            for &(bar_idx, kama_val) in projected {
                if bar_idx >= start_idx && bar_idx < end_idx {
                    let rel = bar_idx - start_idx;
                    let x = data_left + (rel as f32 + 0.5) * bar_w;
                    let pt = egui::pos2(x, price_to_y(kama_val));
                    if let Some(prev_pt) = prev {
                        if let Some([a, b]) = clip_line_segment_to_rect(prev_pt, pt, chart_rect) {
                            painter.line_segment(
                                [a, b],
                                egui::Stroke::new(2.25, egui::Color32::from_rgb(255, 255, 255)),
                            );
                        }
                    }
                    prev = Some(pt);
                }
            }
        }
    }
}
