use super::*;

/// Draw Volume Profile overlay (volume-at-price with POC + Value Area).
pub(crate) fn draw_volume_profile_overlay(
    painter: &egui::Painter,
    chart_rect: egui::Rect,
    bars: &[Bar],
    price_min: f64,
    price_max: f64,
    flags: &IndicatorFlags,
) {
    // ── Volume Profile overlay (volume-at-price with POC + Value Area) ─────
    if flags.price_histogram {
        let num_buckets = (chart_rect.height() / 4.0).max(10.0) as usize;
        let bucket_h = chart_rect.height() / num_buckets as f32;
        let mut buckets = vec![0.0_f64; num_buckets];
        let mut buy_vol = vec![0.0_f64; num_buckets]; // close > open = buying pressure
        let mut max_vol = 0.0_f64;

        for bar in bars {
            let y_high_frac = ((price_max - bar.high) / (price_max - price_min)).clamp(0.0, 1.0);
            let y_low_frac = ((price_max - bar.low) / (price_max - price_min)).clamp(0.0, 1.0);
            let b_top = (y_high_frac * num_buckets as f64) as usize;
            let b_bot = ((y_low_frac * num_buckets as f64) as usize).min(num_buckets - 1);
            let span = (b_bot - b_top).max(1) as f64;
            let vol_per_level = bar.volume / span;
            let is_buy = bar.close >= bar.open;
            for b in b_top..=b_bot {
                if b < num_buckets {
                    buckets[b] += vol_per_level;
                    if is_buy {
                        buy_vol[b] += vol_per_level;
                    }
                    max_vol = max_vol.max(buckets[b]);
                }
            }
        }

        // POC = highest volume bucket
        let poc_idx = buckets
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(0);

        // Value Area: expand from POC until 70% of total volume
        let total_vol: f64 = buckets.iter().sum();
        let va_target = total_vol * 0.7;
        let mut va_vol = buckets[poc_idx];
        let mut va_lo = poc_idx;
        let mut va_hi = poc_idx;
        while va_vol < va_target && (va_lo > 0 || va_hi < num_buckets - 1) {
            let expand_lo = if va_lo > 0 { buckets[va_lo - 1] } else { 0.0 };
            let expand_hi = if va_hi < num_buckets - 1 {
                buckets[va_hi + 1]
            } else {
                0.0
            };
            if expand_lo >= expand_hi && va_lo > 0 {
                va_lo -= 1;
                va_vol += buckets[va_lo];
            } else if va_hi < num_buckets - 1 {
                va_hi += 1;
                va_vol += buckets[va_hi];
            } else {
                break;
            }
        }

        // Draw horizontal bars: buy (teal) left, sell (red) right, POC highlighted
        let max_bar_w = chart_rect.width() * 0.18;
        let poc_col = egui::Color32::from_rgba_premultiplied(255, 215, 0, 120); // gold
        let va_buy = egui::Color32::from_rgba_premultiplied(38, 166, 154, 60); // teal
        let va_sell = egui::Color32::from_rgba_premultiplied(239, 83, 80, 60); // red
        let out_buy = egui::Color32::from_rgba_premultiplied(38, 166, 154, 30);
        let out_sell = egui::Color32::from_rgba_premultiplied(239, 83, 80, 30);
        let edge_col = egui::Color32::from_rgba_premultiplied(100, 140, 255, 80);
        for (i, &vol) in buckets.iter().enumerate() {
            if vol <= 0.0 {
                continue;
            }
            let frac = (vol / max_vol) as f32;
            let total_w = frac * max_bar_w;
            let buy_frac = if vol > 0.0 {
                (buy_vol[i] / vol) as f32
            } else {
                0.5
            };
            let buy_w = total_w * buy_frac;
            let sell_w = total_w - buy_w;
            let y_top = chart_rect.top() + i as f32 * bucket_h;
            let y_bot = y_top + bucket_h;
            let is_va = i >= va_lo && i <= va_hi;

            if i == poc_idx {
                // POC: full-width gold highlight line
                painter.rect_filled(
                    egui::Rect::from_min_max(
                        egui::pos2(chart_rect.right() - total_w, y_top),
                        egui::pos2(chart_rect.right(), y_bot),
                    ),
                    0.0,
                    poc_col,
                );
            } else {
                // Buy volume (right-aligned, teal)
                let (bc, sc) = if is_va {
                    (va_buy, va_sell)
                } else {
                    (out_buy, out_sell)
                };
                painter.rect_filled(
                    egui::Rect::from_min_max(
                        egui::pos2(chart_rect.right() - total_w, y_top),
                        egui::pos2(chart_rect.right() - sell_w, y_bot),
                    ),
                    0.0,
                    bc,
                );
                // Sell volume
                painter.rect_filled(
                    egui::Rect::from_min_max(
                        egui::pos2(chart_rect.right() - sell_w, y_top),
                        egui::pos2(chart_rect.right(), y_bot),
                    ),
                    0.0,
                    sc,
                );
            }
            // Left edge
            painter.line_segment(
                [
                    egui::pos2(chart_rect.right() - total_w, y_top),
                    egui::pos2(chart_rect.right() - total_w, y_bot),
                ],
                egui::Stroke::new(0.5, edge_col),
            );
        }
        // POC dashed line across chart
        {
            let poc_y = chart_rect.top() + (poc_idx as f32 + 0.5) * bucket_h;
            let mut px = chart_rect.left();
            while px < chart_rect.right() {
                let end = (px + 4.0).min(chart_rect.right());
                painter.line_segment(
                    [egui::pos2(px, poc_y), egui::pos2(end, poc_y)],
                    egui::Stroke::new(1.0, egui::Color32::from_rgba_premultiplied(255, 215, 0, 80)),
                );
                px += 8.0;
            }
        }
    }
}
