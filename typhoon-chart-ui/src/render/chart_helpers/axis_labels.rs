use super::*;

/// Draw right-axis price labels/lines for current, EXT, bid, and ask prices.
pub(crate) fn draw_right_axis_price_labels(
    painter: &egui::Painter,
    chart: &ChartState,
    chart_rect: egui::Rect,
    price_axis_w: f32,
    bars: &[Bar],
    fresh_live_mid: Option<f64>,
    price_to_y: impl Fn(f64) -> f32,
    format_price: impl Fn(f64) -> String,
) {
    // ── right price-axis label de-confliction ─────────────────────────────
    // Every boxed price tag on the right axis (last/current, extended-hours,
    // bid, ask) is painted at the same x. When their prices cluster — common
    // for low-priced symbols where bid≈ask≈last — the boxes stack into an
    // unreadable smear. `place_axis_label` tracks the occupied vertical bands
    // and nudges each new tag to the nearest free slot; the underlying dashed
    // line still draws at the true price, only the label moves. Tags are placed
    // in draw order, so earlier (higher-priority) tags keep their preferred y.
    let axis_top = chart_rect.top();
    let axis_bot = chart_rect.bottom();
    let mut occupied_label_bands: Vec<(f32, f32)> = Vec::new();
    let mut place_axis_label = move |desired_center: f32, half_h: f32| -> f32 {
        place_level_label(
            desired_center,
            half_h,
            axis_top,
            axis_bot,
            &mut occupied_label_bands,
        )
    };

    // ── last/core close line ─────────────────────────────────────────────────
    if let Some(last) = bars.last() {
        let current_price = if chart.ext_active && chart.ext_close > 0.0 {
            // During extended hours the `C` tag is the regular-session daily-close
            // reference (the magenta EXT tag below owns the extended-hours last).
            // Use the SAME authoritative close as the "Daily Close" header
            // (chart.ext_open = the shared quote's regular_close), which is
            // timeframe-independent. last.close is the chart's own last-bar close
            // and can desync across timeframes / data sources (e.g. delayed-iapi
            // xStocks like WOK), which made the `C` tag disagree with the header.
            if chart.ext_open > 0.0 {
                chart.ext_open
            } else {
                last.close
            }
        } else {
            fresh_live_mid.unwrap_or(last.close)
        };
        let y = price_to_y(current_price);
        if y >= chart_rect.top() && y <= chart_rect.bottom() {
            let color = if chart.ext_active && chart.ext_close > 0.0 {
                // The `C` tag is the regular/daily close reference. Color it
                // against the previous daily close, not the current intraday
                // candle open; otherwise a down day can look green just because
                // the close finished above that bar's open while EXT is active.
                close_reference_color(current_price, last.open, &chart.bars)
            } else if current_price >= last.open {
                UP
            } else {
                DOWN
            };
            // Dashed line
            let dash_len = 6.0_f32;
            let mut x = chart_rect.left();
            while x < chart_rect.right() {
                let end = (x + dash_len).min(chart_rect.right());
                painter.line_segment(
                    [egui::pos2(x, y), egui::pos2(end, y)],
                    egui::Stroke::new(1.0, color),
                );
                x += dash_len * 2.0;
            }
            // Price label + TradingView-style countdown to the next candle close.
            let label = if chart.ext_active && chart.ext_close > 0.0 {
                format_axis_price_label("C", current_price)
            } else {
                format_price(current_price)
            };
            let countdown = if chart.ext_active && chart.ext_close > 0.0 {
                // Countdown belongs to a forming regular-session bar. During
                // ext-hours this tag is the static regular close reference, so
                // showing a rolling timer under it is misleading.
                None
            } else {
                chart.bars.last().and_then(|latest| {
                    next_candle_countdown_label_for_market(
                        latest.ts_ms,
                        chart.timeframe,
                        chart.primary_source,
                        &chart.symbol,
                    )
                })
            };
            if let Some(countdown) = countdown {
                // TradingView-style current-price tag: ticker / price / countdown
                // stacked, each in its OWN bordered box. The timer used to be a
                // borderless cell that blended into the chart and was hard to read
                // against the price; now all three rows are delineated and the
                // ticker is shown for context.
                let ticker = bare_symbol_from_key(&chart.symbol);
                let row_h = 14.0_f32;
                let badge_h = row_h * 3.0;
                let label_y = place_axis_label(y, badge_h * 0.5);
                let badge_left = chart_rect.right() + 2.0;
                let badge_w = price_axis_w - 4.0;
                let ticker_rect = egui::Rect::from_min_size(
                    egui::pos2(badge_left, label_y - badge_h * 0.5),
                    egui::vec2(badge_w, row_h),
                );
                let price_rect = egui::Rect::from_min_size(
                    egui::pos2(badge_left, ticker_rect.bottom()),
                    egui::vec2(badge_w, row_h),
                );
                let timer_rect = egui::Rect::from_min_size(
                    egui::pos2(badge_left, price_rect.bottom()),
                    egui::vec2(badge_w, row_h),
                );
                let bg = egui::Color32::from_rgb(12, 18, 28);
                let border = egui::Stroke::new(1.0, color);
                for r in [ticker_rect, price_rect, timer_rect] {
                    painter.rect_filled(r, 2.0, bg);
                    painter.rect_stroke(r, 2.0, border, egui::StrokeKind::Inside);
                }
                let text_x = badge_left + 3.0;
                painter.text(
                    egui::pos2(text_x, ticker_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    &ticker,
                    egui::FontId::monospace(9.0),
                    egui::Color32::from_rgb(190, 205, 225),
                );
                painter.text(
                    egui::pos2(text_x, price_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    &label,
                    egui::FontId::monospace(10.0),
                    color,
                );
                painter.text(
                    egui::pos2(text_x, timer_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    &countdown,
                    egui::FontId::monospace(9.0),
                    egui::Color32::from_rgb(215, 230, 245),
                );
            } else {
                let label_y = place_axis_label(y, 8.0);
                let lbl_rect = egui::Rect::from_min_size(
                    egui::pos2(chart_rect.right() + 2.0, label_y - 8.0),
                    egui::vec2(price_axis_w - 4.0, 16.0),
                );
                painter.rect_filled(lbl_rect, 2.0, egui::Color32::from_rgb(12, 18, 28));
                painter.rect_stroke(
                    lbl_rect,
                    2.0,
                    egui::Stroke::new(1.0, color),
                    egui::StrokeKind::Inside,
                );
                painter.text(
                    egui::pos2(chart_rect.right() + 4.0, label_y),
                    egui::Align2::LEFT_CENTER,
                    &label,
                    egui::FontId::monospace(10.0),
                    color,
                );
            }
        }
    }

    // ── Extended hours price line (magenta dashed) ─────────────────────────
    if chart.ext_active && chart.ext_close > 0.0 {
        let y = price_to_y(chart.ext_close);
        if y >= chart_rect.top() && y <= chart_rect.bottom() {
            let ext_col = egui::Color32::from_rgb(200, 50, 200);
            let dash_len = 4.0_f32;
            let mut x = chart_rect.left();
            while x < chart_rect.right() {
                let end = (x + dash_len).min(chart_rect.right());
                painter.line_segment(
                    [egui::pos2(x, y), egui::pos2(end, y)],
                    egui::Stroke::new(1.0, ext_col),
                );
                x += dash_len * 2.0;
            }
            // Price label. Prefix it so extended-hours last cannot be confused
            // with the regular daily close tag.
            let label = format_axis_price_label("EXT", chart.ext_close);
            let label_y = place_axis_label(y, 8.0);
            let lbl_rect = egui::Rect::from_min_size(
                egui::pos2(chart_rect.right() + 2.0, label_y - 8.0),
                egui::vec2(price_axis_w - 4.0, 16.0),
            );
            painter.rect_filled(lbl_rect, 2.0, ext_col);
            painter.text(
                egui::pos2(chart_rect.right() + 4.0, label_y),
                egui::Align2::LEFT_CENTER,
                &label,
                egui::FontId::monospace(10.0),
                egui::Color32::BLACK,
            );
        }
    }

    // ── Bid/Ask spread lines (live streaming quotes) ──────────────────────
    // Hide the spread lines once the streaming quote goes stale (>30s without a
    // tick) so a frozen bid/ask isn't drawn as if live next to a moving candle —
    // the source of the "ask/bid/last decoupled" confusion. Delayed quotes (iapi
    // equities, always delayed=true) are likewise not real-time top-of-book: for a
    // non-WS-tokenized xStock they sit far from the consolidated last and are the
    // direct cause of the chart-vs-watchlist bid/ask desync, so never draw them.
    let quote_fresh = !chart.live_quote_delayed
        && chart
            .live_quote_at
            .is_some_and(|t| t.elapsed() < std::time::Duration::from_secs(30));
    if chart.live_trade_vol > 0.0
        && chart.live_trade_price > 0.0
        && chart.live_trade_price.is_finite()
    {
        let trade_y = price_to_y(chart.live_trade_price);
        if trade_y >= chart_rect.top() && trade_y <= chart_rect.bottom() {
            let trade_col = if chart.live_trade_is_buy {
                egui::Color32::from_rgb(0, 220, 180)
            } else {
                egui::Color32::from_rgb(255, 90, 90)
            };
            let line_col = if chart.live_trade_is_buy {
                egui::Color32::from_rgba_premultiplied(0, 220, 180, 175)
            } else {
                egui::Color32::from_rgba_premultiplied(255, 90, 90, 175)
            };
            let dash_len = 3.0_f32;
            let mut x = chart_rect.left();
            while x < chart_rect.right() {
                let end = (x + dash_len).min(chart_rect.right());
                painter.line_segment(
                    [egui::pos2(x, trade_y), egui::pos2(end, trade_y)],
                    egui::Stroke::new(1.0, line_col),
                );
                x += dash_len * 2.0;
            }
            let label_y = place_axis_label(trade_y, 8.0);
            let side = if chart.live_trade_is_buy {
                "Buy"
            } else {
                "Sell"
            };
            let label = format!(
                "{} {} x {}",
                side,
                format_price(chart.live_trade_price),
                crate::render::time_axis::format_size(chart.live_trade_vol)
            );
            draw_axis_flag(painter, chart_rect, price_axis_w, label_y, label, trade_col);
        }
    }

    if quote_fresh && chart.live_bid > 0.0 && chart.live_ask > 0.0 {
        let bid_y = price_to_y(chart.live_bid);
        let ask_y = price_to_y(chart.live_ask);
        let bid_col = egui::Color32::from_rgba_premultiplied(0, 200, 80, 150);
        let ask_col = egui::Color32::from_rgba_premultiplied(220, 50, 50, 150);
        let bid_text_col = egui::Color32::from_rgb(0, 220, 80);
        let ask_text_col = egui::Color32::from_rgb(255, 90, 90);
        if bid_y >= chart_rect.top() && bid_y <= chart_rect.bottom() {
            painter.line_segment(
                [
                    egui::pos2(chart_rect.left(), bid_y),
                    egui::pos2(chart_rect.right(), bid_y),
                ],
                egui::Stroke::new(0.75, bid_col),
            );
            let bid_label_y = place_axis_label(bid_y, 8.0);
            let size_part = if chart.live_bid_size > 0.0 {
                format!(
                    " x {}",
                    crate::render::time_axis::format_size(chart.live_bid_size)
                )
            } else {
                String::new()
            };
            let label = format!("B {}{}", format_price(chart.live_bid), size_part);
            draw_axis_flag(
                painter,
                chart_rect,
                price_axis_w,
                bid_label_y,
                label,
                bid_text_col,
            );
        }
        if ask_y >= chart_rect.top() && ask_y <= chart_rect.bottom() {
            painter.line_segment(
                [
                    egui::pos2(chart_rect.left(), ask_y),
                    egui::pos2(chart_rect.right(), ask_y),
                ],
                egui::Stroke::new(0.75, ask_col),
            );
            let ask_label_y = place_axis_label(ask_y, 8.0);
            let size_part = if chart.live_ask_size > 0.0 {
                format!(
                    " x {}",
                    crate::render::time_axis::format_size(chart.live_ask_size)
                )
            } else {
                String::new()
            };
            let label = format!("A {}{}", format_price(chart.live_ask), size_part);
            draw_axis_flag(
                painter,
                chart_rect,
                price_axis_w,
                ask_label_y,
                label,
                ask_text_col,
            );
        }
    }
}

/// Draw a right-axis price flag (current/EXT bid/ask/executed-trade tag).
///
/// The flag's right edge is pinned to the price-axis outer edge and it is sized
/// to its text, growing **leftward** when the label is wider than the axis
/// strip. This keeps long L1 labels ("A 0.1770 x 1200") fully on-screen instead
/// of spilling off the right edge of the window. Border and text share `col`.
fn draw_axis_flag(
    painter: &egui::Painter,
    chart_rect: egui::Rect,
    price_axis_w: f32,
    center_y: f32,
    text: String,
    col: egui::Color32,
) {
    let bg = egui::Color32::from_rgb(12, 18, 28);
    let pad_x = 4.0;
    let galley = painter.layout_no_wrap(text, egui::FontId::monospace(9.0), col);
    let box_w = (galley.rect.width() + pad_x * 2.0).max(price_axis_w - 4.0);
    let box_right = chart_rect.right() + price_axis_w - 2.0;
    let rect = egui::Rect::from_min_max(
        egui::pos2(box_right - box_w, center_y - 8.0),
        egui::pos2(box_right, center_y + 8.0),
    );
    painter.rect_filled(rect, 2.0, bg);
    painter.rect_stroke(
        rect,
        2.0,
        egui::Stroke::new(1.0, col),
        egui::StrokeKind::Inside,
    );
    painter.galley(
        egui::pos2(rect.left() + pad_x, center_y - galley.rect.height() * 0.5),
        galley,
        col,
    );
}
