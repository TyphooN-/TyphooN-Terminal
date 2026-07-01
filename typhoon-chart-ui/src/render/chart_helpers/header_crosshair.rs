use super::*;

/// Draw the symbol/timeframe header, crosshair, data window, alert badges, and indicator legend.
pub(crate) fn draw_header_crosshair_and_legend(
    painter: &egui::Painter,
    chart: &ChartState,
    chart_rect: egui::Rect,
    price_axis_w: f32,
    crosshair: Option<egui::Pos2>,
    bars: &[Bar],
    bar_w: f32,
    start_idx: usize,
    price_min: f64,
    price_max: f64,
    flags: &IndicatorFlags,
    company_name: Option<&str>,
    regulatory_alerts: &[typhoon_engine::core::regulatory_alerts::RegulatoryAlert],
    show_rsi: bool,
    show_cmo: bool,
    show_qstick: bool,
    show_disparity: bool,
    show_bop: bool,
    show_stddev: bool,
    show_mfi: bool,
    show_trix: bool,
    show_ppo: bool,
    show_ultosc: bool,
    show_stochrsi: bool,
    show_var_oscillator: bool,
) {
    // ── symbol / tf header geometry ─────────────────────────────────────────
    // Compute this before the crosshair data window so the hover readout can
    // anchor underneath the same decorated header instead of being hidden by it.
    // Append the full company name when one is known and the viewport is wide
    // enough to carry it — keeps tiny MTF grid cells to the compact "SYM [TF]"
    // badge while the single chart and larger cells show "SYM [TF] · Company".
    // 240 px threshold chosen so Reg SHO / EXT badges still fit on the right
    // after the 25-char name cap.
    let sym_label = match company_name {
        Some(name) if chart_rect.width() >= 240.0 => {
            // Always show the full company name (no truncation).
            // The Reg SHO badge is protected by drawing order and the dynamic
            // 18-char cap only when a regulatory alert is present.
            format!("{} [{}] · {}", chart.symbol, chart.timeframe.label(), name)
        }
        _ => format!("{} [{}]", chart.symbol, chart.timeframe.label()),
    };
    let header_pos = egui::pos2(chart_rect.left() + 8.0, chart_rect.top() + 6.0);
    let header_pad_x = 6.0_f32;
    let header_pad_y = 3.0_f32;
    let sym_font = egui::FontId::monospace(11.0);
    let sym_galley = painter.layout_no_wrap(sym_label, sym_font, egui::Color32::WHITE);
    let sym_rect = egui::Rect::from_min_size(
        header_pos,
        egui::vec2(
            sym_galley.rect.width() + header_pad_x * 2.0,
            sym_galley.rect.height() + header_pad_y * 2.0,
        ),
    );

    // ── crosshair ────────────────────────────────────────────────────────────
    if let Some(pos) = crosshair {
        if chart_rect.contains(pos) {
            let ch_color = egui::Color32::from_rgba_premultiplied(180, 180, 200, 100);
            painter.line_segment(
                [
                    egui::pos2(pos.x, chart_rect.top()),
                    egui::pos2(pos.x, chart_rect.bottom()),
                ],
                egui::Stroke::new(0.5, ch_color),
            );
            painter.line_segment(
                [
                    egui::pos2(chart_rect.left(), pos.y),
                    egui::pos2(chart_rect.right(), pos.y),
                ],
                egui::Stroke::new(0.5, ch_color),
            );

            // Price label on right axis
            let frac = (pos.y - chart_rect.top()) / chart_rect.height();
            let price = price_max - frac as f64 * (price_max - price_min);
            let label = format_price(price);
            let lbl_rect = egui::Rect::from_min_size(
                egui::pos2(chart_rect.right() + 2.0, pos.y - 8.0),
                egui::vec2(price_axis_w - 4.0, 16.0),
            );
            painter.rect_filled(lbl_rect, 2.0, egui::Color32::from_rgb(50, 50, 80));
            painter.text(
                egui::pos2(chart_rect.right() + 4.0, pos.y),
                egui::Align2::LEFT_CENTER,
                &label,
                egui::FontId::monospace(10.0),
                egui::Color32::WHITE,
            );

            // OHLCV + indicator values data window (WebKit: .data-window — #000000ee bg)
            let rel_x = pos.x - chart_rect.left();
            let bar_idx = ((rel_x / bar_w) as usize).min(bars.len().saturating_sub(1));
            if bar_idx < bars.len() {
                let b = &bars[bar_idx];

                // Date/time tag on the bottom time axis (mirrors the right-axis
                // price tag) — the TradingView-style readout of the hovered bar's
                // timestamp, formatted per timeframe (intraday shows time, daily+
                // shows the date).
                {
                    let mut ts_buf = String::with_capacity(20);
                    format_ts_buf(b.ts_ms, chart.timeframe, &mut ts_buf);
                    let ts_galley = painter.layout_no_wrap(
                        ts_buf,
                        egui::FontId::monospace(10.0),
                        egui::Color32::WHITE,
                    );
                    let tw = ts_galley.rect.width();
                    let th = ts_galley.rect.height();
                    let box_w = tw + 10.0;
                    let half = box_w * 0.5;
                    // Centre on the crosshair x, clamped to keep the tag inside the
                    // chart's horizontal span. Guard the clamp: a very narrow MTF
                    // cell can be slimmer than the tag, where lo > hi would panic.
                    let lo = chart_rect.left() + half;
                    let hi = chart_rect.right() - half;
                    let cx = if lo <= hi {
                        pos.x.clamp(lo, hi)
                    } else {
                        chart_rect.center().x
                    };
                    let ts_rect = egui::Rect::from_center_size(
                        egui::pos2(cx, chart_rect.bottom() + 10.0),
                        egui::vec2(box_w, 16.0),
                    );
                    painter.rect_filled(ts_rect, 2.0, egui::Color32::from_rgb(50, 50, 80));
                    painter.galley(
                        egui::pos2(cx - tw * 0.5, ts_rect.center().y - th * 0.5),
                        ts_galley,
                        egui::Color32::WHITE,
                    );
                }

                let abs_idx = start_idx + bar_idx;
                let mut tooltip = format!(
                    "O:{} H:{} L:{} C:{} V:{:.0}",
                    format_price(b.open),
                    format_price(b.high),
                    format_price(b.low),
                    format_price(b.close),
                    b.volume,
                );
                // Follow-up: richer L1 in tooltip when live sizes available
                if chart.live_bid > 0.0 && chart.live_ask > 0.0 {
                    let bsz = if chart.live_bid_size > 0.0 { format!(" x {:.2}", chart.live_bid_size) } else { String::new() };
                    let asz = if chart.live_ask_size > 0.0 { format!(" x {:.2}", chart.live_ask_size) } else { String::new() };
                    tooltip.push_str(&format!("\nBid:{}{}  Ask:{}{}", format_price(chart.live_bid), bsz, format_price(chart.live_ask), asz));
                }
                // Indicator values on second line
                let mut ind_parts: Vec<String> = Vec::new();
                if flags.sma200 {
                    if let Some(Some(v)) = chart.sma200.get(abs_idx) {
                        ind_parts.push(format!("SMA200:{}", format_price(*v)));
                    }
                }
                if flags.sma100 {
                    if let Some(Some(v)) = chart.sma100.get(abs_idx) {
                        ind_parts.push(format!("SMA100:{}", format_price(*v)));
                    }
                }
                if flags.kama {
                    if let Some(Some(v)) = chart.kama.get(abs_idx) {
                        ind_parts.push(format!("KAMA:{}", format_price(*v)));
                    }
                }
                if flags.ema21 {
                    if let Some(Some(v)) = chart.ema21.get(abs_idx) {
                        ind_parts.push(format!("EMA21:{}", format_price(*v)));
                    }
                }
                if show_rsi {
                    if let Some(Some(v)) = chart.rsi.get(abs_idx) {
                        ind_parts.push(format!("RSI:{:.1}", v));
                    }
                }
                if show_cmo {
                    if let Some(Some(v)) = chart.cmo.get(abs_idx) {
                        ind_parts.push(format!("CMO:{:+.1}", v));
                    }
                }
                if show_qstick {
                    if let Some(Some(v)) = chart.qstick.get(abs_idx) {
                        ind_parts.push(format!("QStick:{:+.3}", v));
                    }
                }
                if show_disparity {
                    if let Some(Some(v)) = chart.disparity.get(abs_idx) {
                        ind_parts.push(format!("Disp:{:+.2}%", v));
                    }
                }
                if show_bop {
                    if let Some(Some(v)) = chart.bop.get(abs_idx) {
                        ind_parts.push(format!("BOP:{:+.3}", v));
                    }
                }
                if show_stddev {
                    if let Some(Some(v)) = chart.stddev.get(abs_idx) {
                        ind_parts.push(format!("StdDev:{:.3}", v));
                    }
                }
                if show_mfi {
                    if let Some(Some(v)) = chart.mfi.get(abs_idx) {
                        ind_parts.push(format!("MFI:{:.1}", v));
                    }
                }
                if show_trix {
                    if let Some(Some(v)) = chart.trix_line.get(abs_idx) {
                        ind_parts.push(format!("TRIX:{:+.3}", v));
                    }
                }
                if show_ppo {
                    if let Some(Some(v)) = chart.ppo_line.get(abs_idx) {
                        ind_parts.push(format!("PPO:{:+.2}", v));
                    }
                }
                if show_ultosc {
                    if let Some(Some(v)) = chart.ultosc.get(abs_idx) {
                        ind_parts.push(format!("ULT:{:.1}", v));
                    }
                }
                if show_stochrsi {
                    if let (Some(Some(k)), Some(Some(d))) =
                        (chart.stochrsi_k.get(abs_idx), chart.stochrsi_d.get(abs_idx))
                    {
                        ind_parts.push(format!("StochRSI:{:.1}/{:.1}", k, d));
                    }
                }
                if show_var_oscillator {
                    if let Some(Some(v)) = chart.var_oscillator.get(abs_idx) {
                        ind_parts.push(format!("VaR:{:.1}", v));
                    }
                }
                if let Some(Some(v)) = chart.atr.get(abs_idx) {
                    ind_parts.push(format!("ATR:{}", format_price(*v)));
                }
                let ind_text = (!ind_parts.is_empty()).then(|| ind_parts.join("  "));
                let data_chars = ind_text
                    .as_ref()
                    .map(|s| tooltip.len().max(s.len()))
                    .unwrap_or(tooltip.len());
                let data_h = if ind_text.is_some() { 34.0 } else { 20.0 };
                // Anchor below both the symbol header row AND the indicator legend
                // row (which starts at ~top+34) so the hover readout never overlaps
                // either overlay and remains readable in all MTF/single views.
                let legend_row = chart_rect.top() + 38.0;
                let data_y = (sym_rect.bottom() + 22.0)
                    .max(legend_row)
                    .min((chart_rect.bottom() - data_h - 2.0).max(chart_rect.top() + 2.0));
                // Semi-transparent background behind data text. It intentionally
                // sits under the symbol/timeframe header with matching blue trim,
                // instead of competing for the same top-left pixels.
                let data_bg = egui::Rect::from_min_size(
                    egui::pos2(header_pos.x, data_y),
                    egui::vec2(data_chars as f32 * 6.5 + 12.0, data_h),
                );
                painter.rect_filled(
                    data_bg,
                    3.0,
                    egui::Color32::from_rgba_premultiplied(0, 0, 0, 238),
                );
                painter.rect_stroke(
                    data_bg,
                    3.0,
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(70, 120, 180)),
                    egui::StrokeKind::Inside,
                );
                painter.text(
                    egui::pos2(data_bg.left() + 6.0, data_bg.top() + 4.0),
                    egui::Align2::LEFT_TOP,
                    &tooltip,
                    egui::FontId::monospace(10.0),
                    egui::Color32::from_rgb(220, 220, 255),
                );
                if let Some(ind_text) = ind_text {
                    painter.text(
                        egui::pos2(data_bg.left() + 6.0, data_bg.top() + 18.0),
                        egui::Align2::LEFT_TOP,
                        &ind_text,
                        egui::FontId::monospace(10.0),
                        egui::Color32::from_rgb(180, 180, 200),
                    );
                }
            }
        }
    }

    // ── symbol / tf label ───────────────────────────────────────────────────
    draw_symbol_header_badge(painter, sym_rect, header_pad_x, sym_galley);

    // Regulatory alerts extracted to chart_helpers for modularity.
    draw_regulatory_alerts_header(
        painter,
        sym_rect,
        chart_rect,
        header_pad_x,
        regulatory_alerts,
    );

    draw_extended_hours_header_badge(painter, chart, bars, sym_rect, header_pad_x);

    // ── indicator legend ─────────────────────────────────────────────────────
    draw_indicator_legend(painter, chart, chart_rect, sym_rect, flags);
}
