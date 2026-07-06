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
mod pitchfork_wedge_tools;
use pitchfork_wedge_tools::draw_pitchfork_wedge_annotation;
mod fallback_tools;
use fallback_tools::draw_fallback_annotation;
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
        if draw_one_drawing_annotation(
            painter,
            drawing,
            chart,
            chart_rect,
            data_left,
            bar_w,
            &price_to_y,
            start_idx,
            end_idx,
            bars,
            &format_price,
            effective_width,
            d_style,
            is_selected,
        ) {
            return true;
        }
    }

    false
}

/// Render a single drawing through the full annotation chain — shared by the
/// persisted-drawings loop above and the live placement preview (which renders
/// the would-be drawing as a ghost). Returns true when legacy draw_chart
/// control flow should return early.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_one_drawing_annotation(
    painter: &egui::Painter,
    drawing: &Drawing,
    chart: &ChartState,
    chart_rect: egui::Rect,
    data_left: f32,
    bar_w: f32,
    price_to_y: impl Fn(f64) -> f32,
    start_idx: usize,
    end_idx: usize,
    bars: &[Bar],
    format_price: impl Fn(f64) -> String,
    effective_width: f32,
    d_style: LineStyle,
    is_selected: bool,
) -> bool {
    {
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
            return false;
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
            return false;
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
            return false;
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
            return false;
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
            return false;
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
            return false;
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
            return false;
        }
        if draw_pitchfork_wedge_annotation(
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
            return false;
        }
        if draw_fallback_annotation(
            painter,
            drawing,
            chart,
            data_left,
            bar_w,
            &price_to_y,
            start_idx,
            end_idx,
            effective_width,
            d_style,
            is_selected,
        ) {
            return false;
        }
    }

    false
}
