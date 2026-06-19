use super::*;

/// Draw supply/demand zones (rects + labels with status).
/// Extracted from draw_chart for modularity.
pub(crate) fn draw_supply_demand_zones(
    painter: &egui::Painter,
    chart: &ChartState,
    chart_rect: egui::Rect,
    data_left: f32,
    bar_w: f32,
    price_to_y: impl Fn(f64) -> f32,
    start_idx: usize,
    end_idx: usize,
) {
    let status_label = |s: u8| -> &str {
        match s {
            0 => "Untested",
            1 => "Tested",
            2 => "Proven",
            _ => "",
        }
    };
    // Zones extend from their creation bar to the chart right edge (matching MT5).
    // Show any zone whose creation bar is <= end_idx (it extends into or past the view).
    // Demand zones — MT5 colors: DarkSeaGreen/MediumSeaGreen/SeaGreen
    for &(idx, zh, zl, status) in &chart.demand_zones {
        if idx < end_idx {
            let x_start = if idx >= start_idx {
                data_left + ((idx - start_idx) as f32) * bar_w
            } else {
                chart_rect.left()
            };
            let y_top = price_to_y(zh);
            let y_bot = price_to_y(zl);
            if y_bot >= chart_rect.top() && y_top <= chart_rect.bottom() {
                let (fill_col, label_col) = match status {
                    0 => (
                        egui::Color32::from_rgba_premultiplied(143, 188, 143, 50),
                        egui::Color32::from_rgb(200, 255, 200), // high contrast
                    ),
                    1 => (
                        egui::Color32::from_rgba_premultiplied(60, 179, 113, 60),
                        egui::Color32::from_rgb(220, 255, 220),
                    ),
                    _ => (
                        egui::Color32::from_rgba_premultiplied(46, 139, 87, 70),
                        egui::Color32::WHITE,
                    ),
                };
                painter.rect_filled(
                    egui::Rect::from_min_max(
                        egui::pos2(x_start, y_top.max(chart_rect.top())),
                        egui::pos2(chart_rect.right(), y_bot.min(chart_rect.bottom())),
                    ),
                    0.0,
                    fill_col,
                );
                painter.text(
                    egui::pos2(
                        chart_rect.right() - 4.0,
                        y_bot.min(chart_rect.bottom()) - 12.0,
                    ),
                    egui::Align2::RIGHT_TOP,
                    &format!("Demand [{}]", status_label(status)),
                    egui::FontId::monospace(9.0),
                    label_col,
                );
            }
        }
    }
    // Supply zones — MT5 colors: SkyBlue/DeepSkyBlue/DodgerBlue
    for &(idx, zh, zl, status) in &chart.supply_zones {
        if idx < end_idx {
            let x_start = if idx >= start_idx {
                data_left + ((idx - start_idx) as f32) * bar_w
            } else {
                chart_rect.left()
            };
            let y_top = price_to_y(zh);
            let y_bot = price_to_y(zl);
            if y_bot >= chart_rect.top() && y_top <= chart_rect.bottom() {
                let (fill_col, label_col) = match status {
                    0 => (
                        egui::Color32::from_rgba_premultiplied(135, 206, 235, 50),
                        egui::Color32::from_rgb(200, 230, 255), // high contrast on blue zones
                    ),
                    1 => (
                        egui::Color32::from_rgba_premultiplied(0, 191, 255, 60),
                        egui::Color32::from_rgb(220, 245, 255),
                    ),
                    _ => (
                        egui::Color32::from_rgba_premultiplied(30, 144, 255, 70),
                        egui::Color32::WHITE,
                    ),
                };
                painter.rect_filled(
                    egui::Rect::from_min_max(
                        egui::pos2(x_start, y_top.max(chart_rect.top())),
                        egui::pos2(chart_rect.right(), y_bot.min(chart_rect.bottom())),
                    ),
                    0.0,
                    fill_col,
                );
                painter.text(
                    egui::pos2(chart_rect.right() - 4.0, y_top.max(chart_rect.top()) + 2.0),
                    egui::Align2::RIGHT_TOP,
                    &format!("Supply [{}]", status_label(status)),
                    egui::FontId::monospace(9.0),
                    label_col,
                );
            }
        }
    }
}
/// Draw Fair Value Gaps (3-bar imbalance zones).
/// Keeps the suffix-array O(1) filled-gap lookup local to the feature.
pub(crate) fn draw_fair_value_gaps(
    painter: &egui::Painter,
    chart_rect: egui::Rect,
    data_left: f32,
    bar_w: f32,
    price_to_y: impl Fn(f64) -> f32,
    bars: &[Bar],
) {
    let fvg_bull = egui::Color32::from_rgba_premultiplied(0, 180, 80, 30);
    let fvg_bear = egui::Color32::from_rgba_premultiplied(220, 50, 50, 30);
    let fvg_bull_edge = egui::Color32::from_rgba_premultiplied(0, 180, 80, 80);
    let fvg_bear_edge = egui::Color32::from_rgba_premultiplied(220, 50, 50, 80);
    // Suffix arrays make the "has this gap been filled?" lookup O(1).
    // future_min_low[k] = min(bars[k..].low); future_max_high[k] = max(bars[k..].high).
    // The previous code scanned bars[i+2..] for each FVG candidate (O(n²) per frame
    // — pricey on dense charts and unnecessary when only the suffix extremes matter).
    let n = bars.len();
    let mut future_min_low: Vec<f64> = vec![f64::INFINITY; n + 1];
    let mut future_max_high: Vec<f64> = vec![f64::NEG_INFINITY; n + 1];
    for k in (0..n).rev() {
        future_min_low[k] = future_min_low[k + 1].min(bars[k].low);
        future_max_high[k] = future_max_high[k + 1].max(bars[k].high);
    }
    for i in 1..n.saturating_sub(1) {
        let prev = &bars[i - 1];
        let next = &bars[i + 1];
        let x_start = data_left + ((i + 1) as f32 + 0.5) * bar_w;
        let x_end = chart_rect.right();
        let scan_start = (i + 2).min(n);
        // Bullish FVG: bar[i+1].low > bar[i-1].high (gap up)
        if next.low > prev.high {
            let gap_top = price_to_y(next.low);
            let gap_bot = price_to_y(prev.high);
            if gap_top <= chart_rect.bottom() && gap_bot >= chart_rect.top() {
                let filled = future_min_low[scan_start] <= prev.high;
                if !filled {
                    painter.rect_filled(
                        egui::Rect::from_min_max(
                            egui::pos2(x_start, gap_top.max(chart_rect.top())),
                            egui::pos2(x_end, gap_bot.min(chart_rect.bottom())),
                        ),
                        0.0,
                        fvg_bull,
                    );
                    painter.line_segment(
                        [egui::pos2(x_start, gap_top), egui::pos2(x_end, gap_top)],
                        egui::Stroke::new(0.5, fvg_bull_edge),
                    );
                    painter.line_segment(
                        [egui::pos2(x_start, gap_bot), egui::pos2(x_end, gap_bot)],
                        egui::Stroke::new(0.5, fvg_bull_edge),
                    );
                }
            }
        }
        // Bearish FVG: bar[i+1].high < bar[i-1].low (gap down)
        if next.high < prev.low {
            let gap_top = price_to_y(prev.low);
            let gap_bot = price_to_y(next.high);
            if gap_top <= chart_rect.bottom() && gap_bot >= chart_rect.top() {
                let filled = future_max_high[scan_start] >= prev.low;
                if !filled {
                    painter.rect_filled(
                        egui::Rect::from_min_max(
                            egui::pos2(x_start, gap_top.max(chart_rect.top())),
                            egui::pos2(x_end, gap_bot.min(chart_rect.bottom())),
                        ),
                        0.0,
                        fvg_bear,
                    );
                    painter.line_segment(
                        [egui::pos2(x_start, gap_top), egui::pos2(x_end, gap_top)],
                        egui::Stroke::new(0.5, fvg_bear_edge),
                    );
                    painter.line_segment(
                        [egui::pos2(x_start, gap_bot), egui::pos2(x_end, gap_bot)],
                        egui::Stroke::new(0.5, fvg_bear_edge),
                    );
                }
            }
        }
    }
}
/// Draw ICT/Smart Money Order Blocks.
/// Keeps rolling ATR thresholding and the newest-first 20-zone cap local to the feature.
pub(crate) fn draw_order_blocks(
    painter: &egui::Painter,
    chart_rect: egui::Rect,
    data_left: f32,
    bar_w: f32,
    price_to_y: impl Fn(f64) -> f32,
    bars: &[Bar],
) {
    let ob_bull_fill = egui::Color32::from_rgba_premultiplied(0, 180, 160, 25);
    let ob_bull_edge = egui::Color32::from_rgba_premultiplied(0, 180, 160, 100);
    let ob_bear_fill = egui::Color32::from_rgba_premultiplied(220, 50, 50, 25);
    let ob_bear_edge = egui::Color32::from_rgba_premultiplied(220, 50, 50, 100);
    let ob_label_col = egui::Color32::from_rgba_premultiplied(200, 200, 200, 180);

    // Compute rolling ATR(14) for impulsive move threshold. Keep the early-bar
    // behavior unchanged, but avoid recomputing the 14-bar true-range window
    // for every bar on provider-maximum histories.
    let atr_period = 14usize;
    let mut true_ranges: Vec<f64> = Vec::with_capacity(bars.len());
    let mut local_atr: Vec<f64> = Vec::with_capacity(bars.len());
    let mut rolling_sum = 0.0;
    for i in 0..bars.len() {
        let bar = &bars[i];
        let tr = if i == 0 {
            bar.high - bar.low
        } else {
            let prev_close = bars[i - 1].close;
            let hl = bar.high - bar.low;
            let hc = (bar.high - prev_close).abs();
            let lc = (bar.low - prev_close).abs();
            hl.max(hc).max(lc)
        };
        true_ranges.push(tr);
        rolling_sum += tr;
        if i >= atr_period {
            rolling_sum -= true_ranges[i - atr_period];
            local_atr.push(rolling_sum / atr_period as f64);
        } else {
            local_atr.push(bar.high - bar.low);
        }
    }

    // Collect order blocks (limit to most recent 20)
    struct OBZone {
        high: f64,
        low: f64,
        bar_idx: usize,
        is_bull: bool,
        end_idx: usize,
    }
    let mut zones: Vec<OBZone> = Vec::with_capacity(20);

    // Walk newest-to-oldest and stop once the render cap is full. The old path
    // scanned every bar, built every historical OB, then drained the front just
    // to keep the last 20. On provider-maximum histories that did wasted work
    // proportional to the full cache depth on every chart render.
    for i in (0..bars.len().saturating_sub(1)).rev() {
        let cur = &bars[i];
        let nxt = &bars[i + 1];
        let atr = local_atr[i];
        if atr <= 0.0 {
            continue;
        }

        // Bullish OB: bearish candle, then next close breaks above current high by >= 1 ATR
        if cur.close < cur.open && nxt.close > cur.high + atr {
            let mut end = bars.len();
            for j in (i + 2)..bars.len() {
                if bars[j].low <= cur.high {
                    end = j;
                    break;
                }
            }
            zones.push(OBZone {
                high: cur.high,
                low: cur.low,
                bar_idx: i,
                is_bull: true,
                end_idx: end,
            });
        }

        // Bearish OB: bullish candle, then next close breaks below current low by >= 1 ATR
        if cur.close > cur.open && nxt.close < cur.low - atr {
            let mut end = bars.len();
            for j in (i + 2)..bars.len() {
                if bars[j].high >= cur.low {
                    end = j;
                    break;
                }
            }
            zones.push(OBZone {
                high: cur.high,
                low: cur.low,
                bar_idx: i,
                is_bull: false,
                end_idx: end,
            });
        }

        if zones.len() >= 20 {
            break;
        }
    }
    zones.reverse();

    for ob in &zones {
        let x_start = data_left + (ob.bar_idx as f32 + 0.5) * bar_w;
        let x_end = if ob.end_idx >= bars.len() {
            chart_rect.right()
        } else {
            data_left + (ob.end_idx as f32 + 0.5) * bar_w
        };
        if x_end < chart_rect.left() || x_start > chart_rect.right() {
            continue;
        }

        let y_top = price_to_y(ob.high);
        let y_bot = price_to_y(ob.low);
        if y_top > chart_rect.bottom() || y_bot < chart_rect.top() {
            continue;
        }

        let (fill, edge) = if ob.is_bull {
            (ob_bull_fill, ob_bull_edge)
        } else {
            (ob_bear_fill, ob_bear_edge)
        };
        let ct = y_top.max(chart_rect.top());
        let cb = y_bot.min(chart_rect.bottom());
        let cl = x_start.max(chart_rect.left());
        let cr = x_end.min(chart_rect.right());

        painter.rect_filled(
            egui::Rect::from_min_max(egui::pos2(cl, ct), egui::pos2(cr, cb)),
            0.0,
            fill,
        );
        painter.line_segment(
            [egui::pos2(cl, ct), egui::pos2(cr, ct)],
            egui::Stroke::new(0.7, edge),
        );
        painter.line_segment(
            [egui::pos2(cl, cb), egui::pos2(cr, cb)],
            egui::Stroke::new(0.7, edge),
        );
        // "OB" label
        if cr - cl > 20.0 {
            painter.text(
                egui::pos2(cl + 3.0, ct + 1.0),
                egui::Align2::LEFT_TOP,
                if ob.is_bull { "OB+" } else { "OB-" },
                egui::FontId::monospace(9.0),
                ob_label_col,
            );
        }
    }
}
