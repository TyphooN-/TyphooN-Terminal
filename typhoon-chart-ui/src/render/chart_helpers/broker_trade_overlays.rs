use super::*;

/// Draw broker position lines and in-range buy/sell fill markers.
pub(crate) fn draw_broker_trade_overlays(
    painter: &egui::Painter,
    trade_overlay: &TradeOverlay,
    chart_rect: egui::Rect,
    data_left: f32,
    bar_w: f32,
    start_idx: usize,
    end_idx: usize,
    price_to_y: impl Fn(f64) -> f32,
) {
    // ── Broker trade markers (buy/sell arrows + position lines) ────────
    // Position entry/SL/TP lines
    for pl in &trade_overlay.position_lines {
        let y = price_to_y(pl.price);
        if y >= chart_rect.top() && y <= chart_rect.bottom() {
            let (color, label_prefix) = match pl.line_type {
                0 => (
                    if pl.is_buy {
                        egui::Color32::from_rgb(0, 150, 255)
                    } else {
                        egui::Color32::from_rgb(255, 100, 50)
                    },
                    if pl.is_buy { "BUY" } else { "SELL" },
                ),
                1 => (egui::Color32::from_rgb(255, 60, 60), "SL"),
                _ => (egui::Color32::from_rgb(60, 200, 60), "TP"),
            };
            // Dashed line across chart
            let dash_len = 6.0_f32;
            let gap_len = 4.0_f32;
            let mut fx = chart_rect.left();
            while fx < chart_rect.right() {
                let end = (fx + dash_len).min(chart_rect.right());
                painter.line_segment(
                    [egui::pos2(fx, y), egui::pos2(end, y)],
                    egui::Stroke::new(1.0, color),
                );
                fx += dash_len + gap_len;
            }
            // Entry lines (BUY/SELL) show size + average entry ("BUY 11155 @ 0.1058");
            // SL/TP lines just show the price level. Quantity is trimmed of trailing
            // zeros so whole-share lots read cleanly.
            let label = if pl.line_type == 0 {
                let qty_str = if pl.volume.fract().abs() < 1e-9 {
                    format!("{:.0}", pl.volume)
                } else {
                    format!("{:.8}", pl.volume)
                        .trim_end_matches('0')
                        .trim_end_matches('.')
                        .to_string()
                };
                format!("{} {} @ {:.4}", label_prefix, qty_str, pl.price)
            } else {
                format!("{} {:.4}", label_prefix, pl.price)
            };
            painter.text(
                egui::pos2(chart_rect.left() + 4.0, y - 10.0),
                egui::Align2::LEFT_TOP,
                &label,
                egui::FontId::monospace(9.0),
                color,
            );
        }
    }
    // Trade arrows (buy = green up-arrow, sell = red down-arrow).
    // PERF: markers are sorted by bar_idx (see build_trade_overlay). Binary-search
    // for the first in-range marker so we skip off-screen history in O(log N) instead
    // of scanning the full Vec every frame.
    // Arrows render per-fill (small triangles — not noisy). Labels are deferred
    // and collapsed by screen-pixel clustering so dense fill activity
    // (slightly different fill prices on the same bar) doesn't
    // bury the candles under overlapping text blocks. Previously each fill
    // rendered its own label and the chart became
    // unreadable at high trade density.
    struct PendingLabel {
        x: f32,
        y: f32,
        is_buy: bool,
        volume: f64,
        price: f64,
        ticker: String,
        count: u32,
    }
    let mut pending_labels: Vec<PendingLabel> = Vec::new();
    let marker_start = trade_overlay
        .markers
        .partition_point(|m| m.bar_idx < start_idx);
    for tm in trade_overlay.markers[marker_start..]
        .iter()
        .take_while(|m| m.bar_idx < end_idx)
    {
        let rel = tm.bar_idx - start_idx;
        let x = data_left + (rel as f32 + 0.5) * bar_w;
        let y = price_to_y(tm.price);
        if y < chart_rect.top() || y > chart_rect.bottom() {
            continue;
        }
        let (color, arrow_dir) = if tm.is_buy {
            (egui::Color32::from_rgb(76, 175, 80), 1.0_f32) // green, points up (below bar)
        } else {
            (egui::Color32::from_rgb(244, 67, 54), -1.0_f32) // red, points down (above bar)
        };
        let arrow_size = 6.0_f32;
        let y_offset = arrow_size * 2.0 * arrow_dir;
        let tip_y = y + y_offset;
        let base_y = tip_y + arrow_size * arrow_dir;
        let points = vec![
            egui::pos2(x, tip_y),
            egui::pos2(x - arrow_size * 0.6, base_y),
            egui::pos2(x + arrow_size * 0.6, base_y),
        ];
        painter.add(egui::Shape::convex_polygon(
            points,
            color,
            egui::Stroke::NONE,
        ));
        let label_y = if tm.is_buy {
            base_y + 2.0
        } else {
            base_y - 10.0
        };
        pending_labels.push(PendingLabel {
            x,
            y: label_y,
            is_buy: tm.is_buy,
            volume: tm.volume,
            price: tm.price,
            ticker: tm.ticker.clone(),
            count: tm.count,
        });
    }

    // Greedy pixel-proximity clustering per side. CLUSTER_X/Y roughly match the
    // bounding box of an 8pt monospace label so only markers that would
    // actually overlap get merged.
    pub(crate) const CLUSTER_X: f32 = 44.0;
    pub(crate) const CLUSTER_Y: f32 = 12.0;
    struct LabelCluster {
        x_sum: f32,
        y_sum: f32,
        n: u32,
        is_buy: bool,
        volume: f64,
        price_w_sum: f64,
        weight_sum: f64,
        tickers: Vec<String>,
        deals: u32,
    }
    pending_labels.sort_by(|a, b| {
        a.is_buy
            .cmp(&b.is_buy)
            .then(a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal))
            .then(a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal))
    });
    let mut clusters: Vec<LabelCluster> = Vec::new();
    'outer: for lbl in pending_labels {
        for c in clusters.iter_mut() {
            if c.is_buy != lbl.is_buy {
                continue;
            }
            let cx = c.x_sum / c.n as f32;
            let cy = c.y_sum / c.n as f32;
            if (cx - lbl.x).abs() < CLUSTER_X && (cy - lbl.y).abs() < CLUSTER_Y {
                let w = lbl.volume.max(1e-6);
                c.x_sum += lbl.x;
                c.y_sum += lbl.y;
                c.n += 1;
                c.volume += lbl.volume;
                c.price_w_sum += lbl.price * w;
                c.weight_sum += w;
                c.deals += lbl.count;
                for t in lbl.ticker.split(", ").filter(|t| !t.is_empty()) {
                    // O(1) dedup for tickers (was linear .any on small Vec)
                    let mut set: std::collections::HashSet<String> =
                        c.tickers.iter().cloned().collect();
                    if set.insert(t.to_string()) {
                        c.tickers.push(t.to_string());
                    }
                }
                continue 'outer;
            }
        }
        let w = lbl.volume.max(1e-6);
        let mut tickers: Vec<String> = Vec::new();
        {
            // O(1) dedup for tickers (was linear .any on small Vec)
            let mut set: std::collections::HashSet<String> = std::collections::HashSet::new();
            for t in lbl.ticker.split(", ").filter(|t| !t.is_empty()) {
                if set.insert(t.to_string()) {
                    tickers.push(t.to_string());
                }
            }
        }
        clusters.push(LabelCluster {
            x_sum: lbl.x,
            y_sum: lbl.y,
            n: 1,
            is_buy: lbl.is_buy,
            volume: lbl.volume,
            price_w_sum: lbl.price * w,
            weight_sum: w,
            tickers,
            deals: lbl.count,
        });
    }
    for c in &clusters {
        let color = if c.is_buy {
            egui::Color32::from_rgb(76, 175, 80)
        } else {
            egui::Color32::from_rgb(244, 67, 54)
        };
        let x = c.x_sum / c.n as f32;
        let y = c.y_sum / c.n as f32;
        let avg_price = if c.weight_sum > 0.0 {
            c.price_w_sum / c.weight_sum
        } else {
            0.0
        };
        let label = if c.tickers.is_empty() {
            format!("{:.2}", c.volume)
        } else if c.n == 1 && c.tickers.len() == 1 {
            if c.deals > 1 || c.volume >= 0.1 {
                format!("{} {:.2}", c.tickers[0], c.volume)
            } else {
                c.tickers[0].clone()
            }
        } else {
            let head = if c.tickers.len() <= 3 {
                c.tickers.join(",")
            } else {
                format!("{}+{}", c.tickers[..2].join(","), c.tickers.len() - 2)
            };
            format!("[{}] @{:.2} {:.2}", head, avg_price, c.volume)
        };
        painter.text(
            egui::pos2(x, y),
            egui::Align2::CENTER_TOP,
            &label,
            egui::FontId::monospace(8.0),
            color,
        );
    }
}
