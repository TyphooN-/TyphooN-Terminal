use super::*;

use crate::drawing_interaction::preview_drawing;
use crate::render::PriceViewGeometry;

/// Live placement preview (TradingView-style): once a tool is armed, the
/// would-be drawing tracks the cursor as a dashed ghost — for **every** tool,
/// not just the four line types the old hand-rolled preview covered (which
/// parsed `Debug` strings per frame to guess pending points). The ghost is
/// produced by `preview_drawing` (cursor completes the pending points) and
/// rendered through the exact annotation chain committed drawings use, so
/// what you see is what you get.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_drawing_preview(
    painter: &egui::Painter,
    chart: &ChartState,
    chart_rect: egui::Rect,
    data_left: f32,
    bar_w: f32,
    start_idx: usize,
    end_idx: usize,
    bars: &[Bar],
    crosshair: Option<egui::Pos2>,
    draw_mode: &DrawMode,
    price_geometry: &PriceViewGeometry,
    price_to_y: impl Fn(f64) -> f32,
    format_price: impl Fn(f64) -> String,
) {
    if matches!(draw_mode, DrawMode::None | DrawMode::Eraser) {
        return;
    }
    let Some(cross) = crosshair else {
        return;
    };
    if !chart_rect.contains(cross) || bars.is_empty() {
        return;
    }
    let max_bar = bars.len().saturating_sub(1);
    let bar = price_geometry.x_to_bar(cross.x, max_bar);
    let price = price_geometry.price_from_y(cross.y);
    if let Some(ghost) = preview_drawing(draw_mode, &chart.preview_pending_points, bar, price) {
        draw_one_drawing_annotation(
            painter,
            &ghost,
            chart,
            chart_rect,
            data_left,
            bar_w,
            &price_to_y,
            start_idx,
            end_idx,
            bars,
            &format_price,
            1.5,
            LineStyle::Dashed,
            false,
        );
    }
}
