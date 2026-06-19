use super::*;

pub(super) fn draw_indicator_legend(
    painter: &egui::Painter,
    chart: &ChartState,
    chart_rect: egui::Rect,
    sym_rect: egui::Rect,
    flags: &IndicatorFlags,
) {
    // Push legend down when EXT badge is present so it does not overlap.
    let ly = if chart.ext_active && chart.ext_close > 0.0 {
        sym_rect.bottom() + 24.0
    } else {
        chart_rect.top() + 34.0
    };
    let mut lx = chart_rect.left() + 8.0;
    let (ma_legend_label, kama_legend_label) =
        nnfx_trend_legend_labels(!chart.mtf_sma.is_empty(), !chart.multi_kama.is_empty());
    // MTF_MA / MultiKAMA legend labels are intentionally suppressed — the
    // colored overlay lines speak for themselves and the text was clutter.
    // Only the current-TF SMA200 / KAMA labels remain (drawn when no MTF data).
    if flags.sma200 && chart.mtf_sma.is_empty() {
        painter.text(
            egui::pos2(lx, ly),
            egui::Align2::LEFT_TOP,
            ma_legend_label,
            egui::FontId::monospace(10.0),
            SMA200_COL,
        );
        lx += 57.0;
    }
    if flags.sma100 {
        painter.text(
            egui::pos2(lx, ly),
            egui::Align2::LEFT_TOP,
            "SMA100",
            egui::FontId::monospace(10.0),
            SMA100_COL,
        );
        lx += 57.0;
    }
    if flags.kama && chart.multi_kama.is_empty() {
        painter.text(
            egui::pos2(lx, ly),
            egui::Align2::LEFT_TOP,
            kama_legend_label,
            egui::FontId::monospace(10.0),
            KAMA_COL,
        );
        lx += 110.0;
    }
    if flags.ema21 {
        painter.text(
            egui::pos2(lx, ly),
            egui::Align2::LEFT_TOP,
            "EMA21",
            egui::FontId::monospace(10.0),
            EMA_COL,
        );
        lx += 50.0;
    }
    if flags.bollinger {
        painter.text(
            egui::pos2(lx, ly),
            egui::Align2::LEFT_TOP,
            "BB(20,2)",
            egui::FontId::monospace(10.0),
            BB_COL,
        );
    }
}

pub(super) fn draw_symbol_header_badge(
    painter: &egui::Painter,
    sym_rect: egui::Rect,
    header_pad_x: f32,
    sym_galley: std::sync::Arc<egui::Galley>,
) {
    // WebKit: .mtf-cell-label — #8cf, 11px bold, text-shadow.
    // Every cell self-labels with the full "SYM [TF]" badge.
    painter.rect_filled(
        sym_rect,
        3.0,
        egui::Color32::from_rgba_premultiplied(8, 12, 18, 232),
    );
    painter.rect_stroke(
        sym_rect,
        3.0,
        egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 150, 210)),
        egui::StrokeKind::Inside,
    );
    painter.galley(
        egui::pos2(
            sym_rect.left() + header_pad_x,
            sym_rect.center().y - sym_galley.rect.height() * 0.5,
        ),
        sym_galley,
        egui::Color32::WHITE,
    );
}

pub(super) fn draw_extended_hours_header_badge(
    painter: &egui::Painter,
    chart: &ChartState,
    bars: &[Bar],
    sym_rect: egui::Rect,
    header_pad_x: f32,
) {
    // Draw EXT hours badge on a second line below the symbol box to save
    // horizontal space. Reg SHO stays on the first line.
    if !(chart.ext_active && chart.ext_close > 0.0) {
        return;
    }
    let Some(last) = bars.last() else {
        return;
    };
    let daily_close = if chart.ext_open > 0.0 {
        chart.ext_open
    } else {
        last.close
    };
    let prev_close = (chart.prev_daily_close > 0.0)
        .then_some(chart.prev_daily_close)
        .or_else(|| previous_daily_close_from_bars(&chart.bars));
    let ext_text =
        super::time_axis::format_ext_hours_symbol_badge(daily_close, chart.ext_close, prev_close);
    let ext_col = egui::Color32::from_rgb(200, 50, 200);
    let ext_galley = painter.layout_no_wrap(
        ext_text,
        egui::FontId::monospace(10.0),
        egui::Color32::from_rgb(245, 220, 250),
    );
    let ext_rect = egui::Rect::from_min_size(
        egui::pos2(sym_rect.left(), sym_rect.bottom() + 2.0),
        egui::vec2(
            ext_galley.rect.width() + header_pad_x * 2.0,
            sym_rect.height(),
        ),
    );
    painter.rect_filled(
        ext_rect,
        3.0,
        egui::Color32::from_rgba_premultiplied(30, 8, 34, 235),
    );
    painter.rect_stroke(
        ext_rect,
        3.0,
        egui::Stroke::new(1.0, ext_col),
        egui::StrokeKind::Inside,
    );
    painter.galley(
        egui::pos2(
            ext_rect.left() + header_pad_x,
            ext_rect.center().y - ext_galley.rect.height() * 0.5,
        ),
        ext_galley,
        egui::Color32::from_rgb(245, 220, 250),
    );
}

pub(super) fn draw_oscillator_pane(
    painter: &egui::Painter,
    rect: egui::Rect,
    bars: &[Bar],
    series: &[Option<f64>],
    start_idx: usize,
    bar_w: f32,
    label: &str,
    color: egui::Color32,
    val_min: f64,
    val_max: f64,
    ob_level: Option<f64>,
    os_level: Option<f64>,
) {
    painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(0, 0, 0));
    // Sub-pane border-top separator (#444 matching old WebKit)
    painter.line_segment(
        [
            egui::pos2(rect.left(), rect.top()),
            egui::pos2(rect.right(), rect.top()),
        ],
        egui::Stroke::new(1.0, egui::Color32::from_rgb(68, 68, 68)),
    );

    let val_to_y = |v: f64| -> f32 {
        let frac = (val_max - v) / (val_max - val_min);
        rect.top() + frac as f32 * rect.height()
    };

    // OB/OS levels
    let level_color = egui::Color32::from_rgba_premultiplied(140, 140, 160, 60);
    if let Some(ob) = ob_level {
        let y = val_to_y(ob);
        painter.line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            egui::Stroke::new(0.5, level_color),
        );
    }
    if let Some(os) = os_level {
        let y = val_to_y(os);
        painter.line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            egui::Stroke::new(0.5, level_color),
        );
    }
    // Mid line
    let mid_y = val_to_y((val_max + val_min) / 2.0);
    painter.line_segment(
        [
            egui::pos2(rect.left(), mid_y),
            egui::pos2(rect.right(), mid_y),
        ],
        egui::Stroke::new(0.3, GRID),
    );

    // Data line. Sub-panes share the main chart's pixel-aware decimation so
    // dense views don't upload invisible sub-pixel oscillator vertices.
    let sample_step = chart_render_sample_step(bars.len(), rect.width());
    let mut points: Vec<egui::Pos2> = Vec::with_capacity(bars.len() / sample_step + 1);
    for (rel_idx, _) in bars.iter().enumerate().step_by(sample_step) {
        let abs_idx = start_idx + rel_idx;
        if abs_idx >= series.len() {
            continue;
        }
        if let Some(v) = series[abs_idx] {
            let x = rect.left() + (rel_idx as f32 + 0.5) * bar_w;
            let y = val_to_y(v).clamp(rect.top(), rect.bottom());
            points.push(egui::pos2(x, y));
        }
    }
    if points.len() > 1 {
        painter.add(egui::Shape::line(points, egui::Stroke::new(1.5, color)));
    }

    // Label
    painter.text(
        egui::pos2(rect.left() + 4.0, rect.top() + 2.0),
        egui::Align2::LEFT_TOP,
        label,
        egui::FontId::monospace(9.0),
        egui::Color32::WHITE,
    );
}

/// Draw Fisher Transform sub-pane with color-coded histogram bars.
pub(super) fn draw_fisher_pane(
    painter: &egui::Painter,
    rect: egui::Rect,
    bars: &[Bar],
    fisher: &[Option<f64>],
    signal: &[Option<f64>],
    start_idx: usize,
    bar_w: f32,
) {
    painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(0, 0, 0));
    // Sub-pane border-top separator (#444 matching old WebKit)
    painter.line_segment(
        [
            egui::pos2(rect.left(), rect.top()),
            egui::pos2(rect.right(), rect.top()),
        ],
        egui::Stroke::new(1.0, egui::Color32::from_rgb(68, 68, 68)),
    );

    // Fisher typically ranges -3..3, auto-scale
    let mut f_min = -2.0_f64;
    let mut f_max = 2.0_f64;
    for (rel_idx, _) in bars.iter().enumerate() {
        let abs_idx = start_idx + rel_idx;
        if abs_idx >= fisher.len() {
            continue;
        }
        if let Some(v) = fisher[abs_idx] {
            f_min = f_min.min(v);
            f_max = f_max.max(v);
        }
    }
    let padding = (f_max - f_min) * 0.1;
    f_min -= padding;
    f_max += padding;

    let val_to_y = |v: f64| -> f32 {
        let frac = (f_max - v) / (f_max - f_min);
        rect.top() + frac as f32 * rect.height()
    };

    let sample_step = chart_render_sample_step(bars.len(), rect.width());

    // Zero line. Use one primitive instead of dotted per-pixel segment spam.
    let zero_y = val_to_y(0.0);
    painter.line_segment(
        [
            egui::pos2(rect.left(), zero_y),
            egui::pos2(rect.right(), zero_y),
        ],
        egui::Stroke::new(0.5, egui::Color32::from_rgb(60, 60, 60)),
    );

    // Signal line FIRST (behind Fisher — MT5: clrDarkGray/orange, width 1)
    let mut sig_points: Vec<egui::Pos2> = Vec::with_capacity(bars.len() / sample_step + 1);
    for (rel_idx, _) in bars.iter().enumerate().step_by(sample_step) {
        let abs_idx = start_idx + rel_idx;
        if abs_idx >= signal.len() {
            continue;
        }
        if let Some(v) = signal[abs_idx] {
            let x = rect.left() + (rel_idx as f32 + 0.5) * bar_w;
            let y = val_to_y(v).clamp(rect.top(), rect.bottom());
            sig_points.push(egui::pos2(x, y));
        }
    }
    if sig_points.len() > 1 {
        painter.add(egui::Shape::line(
            sig_points,
            egui::Stroke::new(1.0, FISHER_SIG),
        )); // clrDarkGray signal (MT5 buffer 3)
    }

    // Fisher line — colored segments per sampled bar (MT5 exact: green when Fisher > Signal, red when < Signal)
    // NO histogram bars — just the line (matching MT5 screenshot exactly)
    for (rel_idx, _) in bars.iter().enumerate().step_by(sample_step) {
        let abs_idx = start_idx + rel_idx;
        let next_rel_idx = (rel_idx + sample_step).min(bars.len().saturating_sub(1));
        let next_abs_idx = start_idx + next_rel_idx;
        if next_abs_idx >= fisher.len() || next_rel_idx == rel_idx {
            continue;
        }
        if let (Some(f0), Some(f1)) = (fisher[abs_idx], fisher[next_abs_idx]) {
            let sig = if abs_idx < signal.len() {
                signal[abs_idx]
            } else {
                None
            };
            // MT5: clrMediumSeaGreen when Fisher > Signal, clrOrangeRed when Fisher < Signal
            let color = match sig {
                Some(s) if f0 > s => FISHER_POS, // green
                Some(_) => FISHER_NEG,           // red
                None => {
                    if f0 >= 0.0 {
                        FISHER_POS
                    } else {
                        FISHER_NEG
                    }
                }
            };
            let x0 = rect.left() + (rel_idx as f32 + 0.5) * bar_w;
            let x1 = rect.left() + (next_rel_idx as f32 + 0.5) * bar_w;
            let y0 = val_to_y(f0).clamp(rect.top(), rect.bottom());
            let y1 = val_to_y(f1).clamp(rect.top(), rect.bottom());
            painter.line_segment(
                [egui::pos2(x0, y0), egui::pos2(x1, y1)],
                egui::Stroke::new(2.0, color),
            );
        }
    }

    // Label with current values (MT5 style: "Ehlers Fisher transform (32) -2.037 -2.068")
    let last_fisher = fisher.iter().rev().find_map(|v| *v);
    let last_signal = signal.iter().rev().find_map(|v| *v);
    let label = match (last_fisher, last_signal) {
        (Some(f), Some(s)) => format!("Ehlers Fisher transform (32) {:.3} {:.3}", f, s),
        (Some(f), None) => format!("Ehlers Fisher transform (32) {:.3}", f),
        _ => "Ehlers Fisher transform (32)".to_string(),
    };
    painter.text(
        egui::pos2(rect.left() + 4.0, rect.top() + 2.0),
        egui::Align2::LEFT_TOP,
        &label,
        egui::FontId::monospace(9.0),
        egui::Color32::WHITE,
    );
}

/// Draw MACD sub-pane with two lines + histogram.
pub(super) fn draw_macd_pane(
    painter: &egui::Painter,
    rect: egui::Rect,
    bars: &[Bar],
    macd_line: &[Option<f64>],
    macd_signal: &[Option<f64>],
    macd_hist: &[Option<f64>],
    start_idx: usize,
    bar_w: f32,
    label: &str,
    line_color: egui::Color32,
    signal_color: egui::Color32,
) {
    painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(0, 0, 0));
    // Sub-pane border-top separator (#444 matching old WebKit)
    painter.line_segment(
        [
            egui::pos2(rect.left(), rect.top()),
            egui::pos2(rect.right(), rect.top()),
        ],
        egui::Stroke::new(1.0, egui::Color32::from_rgb(68, 68, 68)),
    );

    // Auto-scale
    let mut v_min = 0.0_f64;
    let mut v_max = 0.0_f64;
    for (rel_idx, _) in bars.iter().enumerate() {
        let abs_idx = start_idx + rel_idx;
        if abs_idx >= macd_line.len() {
            continue;
        }
        for series in [macd_line, macd_signal, macd_hist] {
            if let Some(Some(v)) = series.get(abs_idx) {
                v_min = v_min.min(*v);
                v_max = v_max.max(*v);
            }
        }
    }
    let padding = (v_max - v_min).max(0.001) * 0.1;
    v_min -= padding;
    v_max += padding;

    let val_to_y = |v: f64| -> f32 {
        let frac = (v_max - v) / (v_max - v_min);
        rect.top() + frac as f32 * rect.height()
    };

    let sample_step = chart_render_sample_step(bars.len(), rect.width());

    // Zero line
    let zero_y = val_to_y(0.0);
    painter.line_segment(
        [
            egui::pos2(rect.left(), zero_y),
            egui::pos2(rect.right(), zero_y),
        ],
        egui::Stroke::new(0.3, GRID),
    );

    // Histogram. Preserve the strongest absolute bar in each sampled bucket so
    // dense rendering does not hide spikes while still emitting ~pixel-count rects.
    let hist_w = (bar_w * sample_step as f32 * 0.6).max(1.0);
    for rel_idx in (0..bars.len()).step_by(sample_step) {
        let bucket_end = (rel_idx + sample_step).min(bars.len());
        let mut selected: Option<(usize, f64)> = None;
        for bucket_rel in rel_idx..bucket_end {
            let abs_idx = start_idx + bucket_rel;
            if let Some(Some(v)) = macd_hist.get(abs_idx) {
                if selected.map_or(true, |(_, cur)| v.abs() > cur.abs()) {
                    selected = Some((bucket_rel, *v));
                }
            }
        }
        if let Some((bucket_rel, v)) = selected {
            let x = rect.left() + (bucket_rel as f32 + 0.5) * bar_w;
            let y = val_to_y(v);
            // MACD histogram: teal green positive, coral red negative (TradingView/MT5 style)
            let color = if v >= 0.0 {
                egui::Color32::from_rgb(38, 166, 154) // #26a69a (teal green)
            } else {
                egui::Color32::from_rgb(239, 83, 80) // #ef5350 (coral red)
            };
            let (top, bottom) = if v >= 0.0 { (y, zero_y) } else { (zero_y, y) };
            painter.rect_filled(
                egui::Rect::from_min_max(
                    egui::pos2(x - hist_w / 2.0, top),
                    egui::pos2(x + hist_w / 2.0, bottom),
                ),
                0.0,
                color,
            );
        }
    }

    // MACD line
    let mut points: Vec<egui::Pos2> = Vec::with_capacity(bars.len() / sample_step + 1);
    for (rel_idx, _) in bars.iter().enumerate().step_by(sample_step) {
        let abs_idx = start_idx + rel_idx;
        if let Some(Some(v)) = macd_line.get(abs_idx) {
            points.push(egui::pos2(
                rect.left() + (rel_idx as f32 + 0.5) * bar_w,
                val_to_y(*v).clamp(rect.top(), rect.bottom()),
            ));
        }
    }
    if points.len() > 1 {
        painter.add(egui::Shape::line(
            points,
            egui::Stroke::new(1.5, line_color),
        ));
    }

    // Signal line
    let mut points: Vec<egui::Pos2> = Vec::with_capacity(bars.len() / sample_step + 1);
    for (rel_idx, _) in bars.iter().enumerate().step_by(sample_step) {
        let abs_idx = start_idx + rel_idx;
        if let Some(Some(v)) = macd_signal.get(abs_idx) {
            points.push(egui::pos2(
                rect.left() + (rel_idx as f32 + 0.5) * bar_w,
                val_to_y(*v).clamp(rect.top(), rect.bottom()),
            ));
        }
    }
    if points.len() > 1 {
        painter.add(egui::Shape::line(
            points,
            egui::Stroke::new(1.0, signal_color),
        ));
    }

    painter.text(
        egui::pos2(rect.left() + 4.0, rect.top() + 2.0),
        egui::Align2::LEFT_TOP,
        label,
        egui::FontId::monospace(9.0),
        egui::Color32::WHITE,
    );
}

/// Draw volume bars sub-pane.
pub(super) fn draw_volume_pane(
    painter: &egui::Painter,
    rect: egui::Rect,
    bars: &[Bar],
    _start_idx: usize,
    bar_w: f32,
) {
    painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(0, 0, 0));
    // Sub-pane border-top separator (#444 matching old WebKit)
    painter.line_segment(
        [
            egui::pos2(rect.left(), rect.top()),
            egui::pos2(rect.right(), rect.top()),
        ],
        egui::Stroke::new(1.0, egui::Color32::from_rgb(68, 68, 68)),
    );

    if bars.is_empty() {
        return;
    }
    let max_vol = bars.iter().map(|b| b.volume).fold(0.0_f64, f64::max);
    if max_vol <= 0.0 {
        return;
    }

    let sample_step = chart_render_sample_step(bars.len(), rect.width());
    let hist_w = (bar_w * sample_step as f32 * 0.7).max(1.0);
    for rel_idx in (0..bars.len()).step_by(sample_step) {
        let bucket_end = (rel_idx + sample_step).min(bars.len());
        let Some((bucket_rel, b)) = bars[rel_idx..bucket_end]
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.volume.total_cmp(&b.volume))
            .map(|(offset, b)| (rel_idx + offset, b))
        else {
            continue;
        };
        let x = rect.left() + (bucket_rel as f32 + 0.5) * bar_w;
        let h = (b.volume / max_vol) as f32 * rect.height();
        let color = if b.close >= b.open {
            egui::Color32::from_rgba_premultiplied(UP.r(), UP.g(), UP.b(), 150)
        } else {
            egui::Color32::from_rgba_premultiplied(DOWN.r(), DOWN.g(), DOWN.b(), 150)
        };
        painter.rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(x - hist_w / 2.0, rect.bottom() - h),
                egui::pos2(x + hist_w / 2.0, rect.bottom()),
            ),
            0.0,
            color,
        );
    }

    painter.text(
        egui::pos2(rect.left() + 4.0, rect.top() + 2.0),
        egui::Align2::LEFT_TOP,
        "Volume",
        egui::FontId::monospace(9.0),
        egui::Color32::WHITE,
    );
}

/// Draw Better Volume sub-pane (NNFX-style color-coded volume).
pub(super) fn draw_better_volume_pane(
    painter: &egui::Painter,
    rect: egui::Rect,
    bars: &[Bar],
    vol_type: &[u8],
    start_idx: usize,
    bar_w: f32,
) {
    painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(0, 0, 0));
    // Sub-pane border-top separator (#444 matching old WebKit)
    painter.line_segment(
        [
            egui::pos2(rect.left(), rect.top()),
            egui::pos2(rect.right(), rect.top()),
        ],
        egui::Stroke::new(1.0, egui::Color32::from_rgb(68, 68, 68)),
    );

    if bars.is_empty() {
        return;
    }
    let max_vol = bars.iter().map(|b| b.volume).fold(0.0_f64, f64::max);
    if max_vol <= 0.0 {
        return;
    }

    let sample_step = chart_render_sample_step(bars.len(), rect.width());
    let hist_w = (bar_w * sample_step as f32 * 0.7).max(1.0);
    for rel_idx in (0..bars.len()).step_by(sample_step) {
        let bucket_end = (rel_idx + sample_step).min(bars.len());
        let Some((bucket_rel, b)) = bars[rel_idx..bucket_end]
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.volume.total_cmp(&b.volume))
            .map(|(offset, b)| (rel_idx + offset, b))
        else {
            continue;
        };
        let abs_idx = start_idx + bucket_rel;
        let x = rect.left() + (bucket_rel as f32 + 0.5) * bar_w;
        let h = (b.volume / max_vol) as f32 * rect.height();
        let vt = vol_type.get(abs_idx).copied().unwrap_or(5);
        // MQL5 enum: 0=low(yellow), 1=climax_up(red), 2=climax_dn(white),
        //            3=churn(green), 4=climax_churn(magenta), 5=normal(steelblue)
        let color = match vt {
            0 => BVOL_LOW,       // Yellow — low volume
            1 => BVOL_CLIMAX_UP, // Red — climax up
            2 => BVOL_CLIMAX_DN, // White — climax down
            3 => BVOL_HIGH,      // Green — churn
            4 => BVOL_CHURN,     // Magenta — climax + churn
            _ => BVOL_NORMAL,    // SteelBlue — normal
        };
        painter.rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(x - hist_w / 2.0, rect.bottom() - h),
                egui::pos2(x + hist_w / 2.0, rect.bottom()),
            ),
            0.0,
            color,
        );
    }
    // Label with current volume value (MT5 style: "BetterVol(20) 10748 0")
    let last_vol = bars.last().map(|b| b.volume as i64).unwrap_or(0);
    let label = format!("BetterVol(20) {} 0", last_vol);
    painter.text(
        egui::pos2(rect.left() + 4.0, rect.top() + 2.0),
        egui::Align2::LEFT_TOP,
        &label,
        egui::FontId::monospace(9.0),
        egui::Color32::WHITE,
    );
}

/// Draw Stochastic sub-pane.
pub(super) fn draw_stoch_pane(
    painter: &egui::Painter,
    rect: egui::Rect,
    bars: &[Bar],
    stoch_k: &[Option<f64>],
    stoch_d: &[Option<f64>],
    start_idx: usize,
    bar_w: f32,
    label: &str,
) {
    painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(0, 0, 0));
    // Sub-pane border-top separator (#444 matching old WebKit)
    painter.line_segment(
        [
            egui::pos2(rect.left(), rect.top()),
            egui::pos2(rect.right(), rect.top()),
        ],
        egui::Stroke::new(1.0, egui::Color32::from_rgb(68, 68, 68)),
    );

    let val_to_y = |v: f64| -> f32 {
        let frac = (100.0 - v) / 100.0;
        rect.top() + frac as f32 * rect.height()
    };

    // OB/OS levels
    let level_col = egui::Color32::from_rgba_premultiplied(140, 140, 160, 60);
    for &lvl in &[80.0, 20.0] {
        let y = val_to_y(lvl);
        painter.line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            egui::Stroke::new(0.5, level_col),
        );
    }

    let sample_step = chart_render_sample_step(bars.len(), rect.width());

    // %K line
    let mut points: Vec<egui::Pos2> = Vec::with_capacity(bars.len() / sample_step + 1);
    for (rel_idx, _) in bars.iter().enumerate().step_by(sample_step) {
        if let Some(Some(v)) = stoch_k.get(start_idx + rel_idx) {
            points.push(egui::pos2(
                rect.left() + (rel_idx as f32 + 0.5) * bar_w,
                val_to_y(*v).clamp(rect.top(), rect.bottom()),
            ));
        }
    }
    if points.len() > 1 {
        painter.add(egui::Shape::line(
            points,
            egui::Stroke::new(1.5, STOCH_K_COL),
        ));
    }

    // %D line
    let mut points: Vec<egui::Pos2> = Vec::with_capacity(bars.len() / sample_step + 1);
    for (rel_idx, _) in bars.iter().enumerate().step_by(sample_step) {
        if let Some(Some(v)) = stoch_d.get(start_idx + rel_idx) {
            points.push(egui::pos2(
                rect.left() + (rel_idx as f32 + 0.5) * bar_w,
                val_to_y(*v).clamp(rect.top(), rect.bottom()),
            ));
        }
    }
    if points.len() > 1 {
        painter.add(egui::Shape::line(
            points,
            egui::Stroke::new(1.0, STOCH_D_COL),
        ));
    }

    painter.text(
        egui::pos2(rect.left() + 4.0, rect.top() + 2.0),
        egui::Align2::LEFT_TOP,
        label,
        egui::FontId::monospace(9.0),
        egui::Color32::WHITE,
    );
}

/// Draw ADX + DI+/DI- sub-pane.
pub(super) fn draw_adx_pane(
    painter: &egui::Painter,
    rect: egui::Rect,
    bars: &[Bar],
    adx: &[Option<f64>],
    di_plus: &[Option<f64>],
    di_minus: &[Option<f64>],
    start_idx: usize,
    bar_w: f32,
) {
    painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(0, 0, 0));
    // Sub-pane border-top separator (#444 matching old WebKit)
    painter.line_segment(
        [
            egui::pos2(rect.left(), rect.top()),
            egui::pos2(rect.right(), rect.top()),
        ],
        egui::Stroke::new(1.0, egui::Color32::from_rgb(68, 68, 68)),
    );

    // Auto-scale 0-60
    let val_to_y = |v: f64| -> f32 {
        let frac = (60.0 - v) / 60.0;
        rect.top() + frac as f32 * rect.height()
    };

    // Reference line at 25
    let y25 = val_to_y(25.0);
    painter.line_segment(
        [egui::pos2(rect.left(), y25), egui::pos2(rect.right(), y25)],
        egui::Stroke::new(
            0.5,
            egui::Color32::from_rgba_premultiplied(140, 140, 160, 60),
        ),
    );

    let sample_step = chart_render_sample_step(bars.len(), rect.width());
    for (series, color, width) in [
        (adx, ADX_COL, 1.5_f32),
        (di_plus, DI_PLUS_COL, 1.0),
        (di_minus, DI_MINUS_COL, 1.0),
    ] {
        let mut points: Vec<egui::Pos2> = Vec::with_capacity(bars.len() / sample_step + 1);
        for (rel_idx, _) in bars.iter().enumerate().step_by(sample_step) {
            if let Some(Some(v)) = series.get(start_idx + rel_idx) {
                points.push(egui::pos2(
                    rect.left() + (rel_idx as f32 + 0.5) * bar_w,
                    val_to_y(*v).clamp(rect.top(), rect.bottom()),
                ));
            }
        }
        if points.len() > 1 {
            painter.add(egui::Shape::line(points, egui::Stroke::new(width, color)));
        }
    }

    painter.text(
        egui::pos2(rect.left() + 4.0, rect.top() + 2.0),
        egui::Align2::LEFT_TOP,
        "ADX(14)",
        egui::FontId::monospace(9.0),
        egui::Color32::WHITE,
    );
}

/// Render decimation for dense chart views.
///
/// Keep at most ~2 samples per horizontal pixel. More than that is visually
/// indistinguishable but expensive for egui tessellation and GPU upload.
pub(super) fn chart_render_sample_step(len: usize, width_px: f32) -> usize {
    if len <= 1 {
        return 1;
    }
    let max_samples = ((width_px.max(1.0).ceil() as usize).saturating_mul(2)).max(1);
    if len <= max_samples {
        1
    } else {
        len.saturating_add(max_samples - 1) / max_samples
    }
}

pub(super) fn adjacent_projection_candle_x(
    data_left: f32,
    visible_real_bars: usize,
    bar_w: f32,
    half_body: f32,
    chart_rect: egui::Rect,
) -> Option<f32> {
    if visible_real_bars == 0 || !bar_w.is_finite() || bar_w <= 0.0 {
        return None;
    }
    let x = data_left + (visible_real_bars as f32 + 0.5) * bar_w;
    if x - half_body >= chart_rect.left() && x + half_body <= chart_rect.right() {
        Some(x)
    } else {
        None
    }
}

/// Render a single indicator series as clipped line segments.
///
/// Do not cull individual points by `y` before drawing. Price-axis zoom/pan can
/// put both sampled endpoints outside the pane while the segment between them
/// still crosses the visible chart. The old point-culling path dropped those
/// crossing segments, which made overlays pop in/out while scaling the price
/// axis or free-looking vertically.
pub(super) fn draw_indicator_line(
    painter: &egui::Painter,
    chart_rect: egui::Rect,
    data_left: f32,
    bars: &[Bar],
    series: &[Option<f64>],
    start_idx: usize,
    bar_w: f32,
    price_to_y: &dyn Fn(f64) -> f32,
    color: egui::Color32,
    width: f32,
) {
    let sample_step = chart_render_sample_step(bars.len(), chart_rect.width());
    let stroke = egui::Stroke::new(width, color);
    let mut prev: Option<egui::Pos2> = None;
    for (rel_idx, _bar) in bars.iter().enumerate().step_by(sample_step) {
        let abs_idx = start_idx + rel_idx;
        let Some(v) = indicator_value_at(series, abs_idx) else {
            prev = None;
            continue;
        };
        let x = data_left + (rel_idx as f32 + 0.5) * bar_w;
        let pt = egui::pos2(x, price_to_y(v));
        if let Some(prev_pt) = prev {
            if let Some([a, b]) = clip_line_segment_to_rect(prev_pt, pt, chart_rect) {
                painter.line_segment([a, b], stroke);
            }
        }
        prev = Some(pt);
    }
}

// ─── helpers ─────────────────────────────────────────────────────────────────

pub(super) fn indicator_value_at(series: &[Option<f64>], idx: usize) -> Option<f64> {
    series.get(idx).copied().flatten()
}

pub(super) fn clip_line_segment_to_rect(
    a: egui::Pos2,
    b: egui::Pos2,
    rect: egui::Rect,
) -> Option<[egui::Pos2; 2]> {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let mut t0 = 0.0_f32;
    let mut t1 = 1.0_f32;

    for (p, q) in [
        (-dx, a.x - rect.left()),
        (dx, rect.right() - a.x),
        (-dy, a.y - rect.top()),
        (dy, rect.bottom() - a.y),
    ] {
        if p.abs() < f32::EPSILON {
            if q < 0.0 {
                return None;
            }
            continue;
        }
        let r = q / p;
        if p < 0.0 {
            t0 = t0.max(r);
        } else {
            t1 = t1.min(r);
        }
        if t0 > t1 {
            return None;
        }
    }

    Some([
        egui::pos2(a.x + t0 * dx, a.y + t0 * dy),
        egui::pos2(a.x + t1 * dx, a.y + t1 * dy),
    ])
}

// ─── overlay / label helpers (moved from technical_analysis.rs for modularity) ─

pub(super) fn draw_current_sma200_overlay(flags_sma200: bool, has_mtf_ma: bool) -> bool {
    flags_sma200 && !has_mtf_ma
}

pub(super) fn draw_current_kama_overlay(flags_kama: bool, has_multi_kama: bool) -> bool {
    flags_kama && !has_multi_kama
}

pub(super) fn nnfx_trend_legend_labels(
    has_mtf_ma: bool,
    has_multi_kama: bool,
) -> (&'static str, &'static str) {
    let ma_label = if has_mtf_ma { "MTF_MA" } else { "SMA200" };
    let kama_label = if has_multi_kama {
        "MultiKAMA"
    } else {
        "KAMA(10,2,30)"
    };
    (ma_label, kama_label)
}

pub(super) fn previous_daily_close_from_bars(bars: &[Bar]) -> Option<f64> {
    let latest_day = bars.last()?.ts_ms / 86_400_000;
    bars.iter()
        .rev()
        .find(|bar| bar.ts_ms / 86_400_000 < latest_day)
        .map(|bar| bar.close)
}

pub(super) fn close_reference_color(
    current_close: f64,
    fallback_open: f64,
    bars: &[Bar],
) -> egui::Color32 {
    let reference = previous_daily_close_from_bars(bars).unwrap_or(fallback_open);
    if current_close >= reference { UP } else { DOWN }
}

pub(super) fn clamp_f32_bounds(value: f32, a: f32, b: f32) -> f32 {
    if !value.is_finite() || !a.is_finite() || !b.is_finite() {
        return value;
    }
    let lo = a.min(b);
    let hi = a.max(b);
    value.clamp(lo, hi)
}

/// Nudge a horizontal price-level text label to the nearest free vertical band
/// so clustered levels don't overprint each other into an unreadable smear.
/// The level line itself stays at its true price (`desired_center`); only the
/// label moves. Labels are placed in draw order, so earlier (higher-priority)
/// labels keep their preferred y and later ones flow around them. The chosen
/// band is recorded in `occupied` for subsequent calls. This is the same
/// policy the right price-axis tags use (`place_axis_label`); keep them in sync.
pub(super) fn place_level_label(
    desired_center: f32,
    half_h: f32,
    top: f32,
    bot: f32,
    occupied: &mut Vec<(f32, f32)>,
) -> f32 {
    let lo_bound = top + half_h;
    let hi_bound = (bot - half_h).max(lo_bound);
    let clamp_center = |c: f32| clamp_f32_bounds(c, lo_bound, hi_bound);
    let collides = |c: f32, bands: &[(f32, f32)]| {
        bands
            .iter()
            .any(|&(lo, hi)| c - half_h < hi + 1.0 && c + half_h + 1.0 > lo)
    };
    let mut center = clamp_center(desired_center);
    if collides(center, occupied.as_slice()) {
        let span = bot - top;
        let mut offset = 1.0_f32;
        loop {
            let up = clamp_center(desired_center - offset);
            if !collides(up, occupied.as_slice()) {
                center = up;
                break;
            }
            let down = clamp_center(desired_center + offset);
            if !collides(down, occupied.as_slice()) {
                center = down;
                break;
            }
            offset += 1.0;
            if offset > span {
                break;
            }
        }
    }
    occupied.push((center - half_h, center + half_h));
    center
}

/// Resolve the display company name for a chart's symbol from the in-memory
/// fundamentals table (`self.bg.all_fundamentals`). The chart symbol is
/// normalized to the bare ticker the table keys on — drop a forward slash
/// (crypto pairs like "BTC/USD") and trim a trailing ".EQ" Kraken-equity
/// suffix — then matched case-insensitively, the same way the research packet's
/// company header resolves it. Returns None when there is no row or the name is
/// blank, so the caller falls back to the plain "SYM [TF]" header.
pub fn chart_overlay_company_name(
    fundamentals: &[typhoon_engine::core::fundamentals::Fundamentals],
    equity_names: &std::collections::HashMap<String, String>,
    chart_symbol: &str,
) -> Option<String> {
    let stripped = chart_symbol.replace('/', "");
    let bare = stripped
        .trim_end_matches(".EQ")
        .trim_end_matches(".eq")
        .to_ascii_uppercase();

    // 1. Full fundamentals (highest quality)
    if let Some(name) = fundamentals
        .iter()
        .find(|f| f.symbol.eq_ignore_ascii_case(&bare))
        .map(|f| f.company_name.trim().to_string())
        .filter(|name| !name.is_empty())
    {
        return Some(name);
    }

    // 2. Lightweight Kraken equity catalog name (fast path for xStocks)
    if let Some(name) = equity_names.get(&bare) {
        let trimmed = name.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    None
}
/// Parse helpers for oscillator range strings (e.g. "30-70" for overbought/oversold).
pub fn parse_range(s: &str, default_lo: usize, default_hi: usize) -> (usize, usize) {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() == 2 {
        let lo = parts[0].trim().parse().unwrap_or(default_lo);
        let hi = parts[1].trim().parse().unwrap_or(default_hi);
        (lo, hi)
    } else {
        (default_lo, default_hi)
    }
}

pub fn parse_range_f32(s: &str, default_lo: f64, default_hi: f64) -> (f64, f64) {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() == 2 {
        let lo = parts[0].trim().parse().unwrap_or(default_lo);
        let hi = parts[1].trim().parse().unwrap_or(default_hi);
        (lo, hi)
    } else {
        (default_lo, default_hi)
    }
}

/// Draw regulatory alerts (e.g. Reg SHO badges) in the chart header.
/// Extracted for modularity (technical_analysis.rs draw_chart is large).
pub(super) fn draw_regulatory_alerts_header(
    painter: &egui::Painter,
    sym_rect: egui::Rect,
    chart_rect: egui::Rect,
    header_pad_x: f32,
    regulatory_alerts: &[typhoon_engine::core::regulatory_alerts::RegulatoryAlert],
) {
    let mut header_right = sym_rect.right();
    for alert in regulatory_alerts {
        let alert_galley = painter.layout_no_wrap(
            alert.label.clone(),
            egui::FontId::monospace(10.0),
            egui::Color32::from_rgb(255, 245, 220),
        );
        let alert_rect = egui::Rect::from_min_size(
            egui::pos2(header_right + 2.0, sym_rect.top()),
            egui::vec2(
                alert_galley.rect.width() + header_pad_x * 2.0,
                sym_rect.height(),
            ),
        );
        if alert_rect.right() > chart_rect.right() - 4.0 {
            break;
        }
        painter.rect_filled(
            alert_rect,
            3.0,
            egui::Color32::from_rgba_premultiplied(80, 12, 12, 238),
        );
        painter.rect_stroke(
            alert_rect,
            3.0,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(255, 70, 70)),
            egui::StrokeKind::Inside,
        );
        painter.galley(
            egui::pos2(
                alert_rect.left() + header_pad_x,
                alert_rect.center().y - alert_galley.rect.height() * 0.5,
            ),
            alert_galley,
            egui::Color32::from_rgb(255, 245, 220),
        );
        header_right = alert_rect.right();
    }
}

/// Draw price alert lines (orange dotted + bell labels) on the chart.
/// price_to_y and format_price passed from draw_chart.
/// Extracted for modularity (part of technical_analysis.rs cleanup).
pub(super) fn draw_price_alert_lines(
    painter: &egui::Painter,
    chart_rect: egui::Rect,
    price_to_y: impl Fn(f64) -> f32,
    alerts: &[(f64, String)],
    format_price: impl Fn(f64) -> String,
) {
    if alerts.is_empty() {
        return;
    }
    let alert_col = egui::Color32::from_rgb(255, 165, 0);
    let alert_bg = egui::Color32::from_rgba_premultiplied(255, 165, 0, 30);
    for (price, label) in alerts {
        let y = price_to_y(*price);
        if y >= chart_rect.top() && y <= chart_rect.bottom() {
            let mut ax = chart_rect.left();
            while ax < chart_rect.right() {
                let end = (ax + 4.0).min(chart_rect.right());
                painter.line_segment(
                    [egui::pos2(ax, y), egui::pos2(end, y)],
                    egui::Stroke::new(1.0, alert_col),
                );
                ax += 8.0;
            }
            let bell = "\u{1F514}";
            let lbl = if label.is_empty() {
                format!("{} {}", bell, format_price(*price))
            } else {
                format!("{} {} {}", bell, label, format_price(*price))
            };
            let text_rect = egui::Rect::from_min_size(
                egui::pos2(chart_rect.left() + 2.0, y - 9.0),
                egui::vec2(lbl.len() as f32 * 6.5 + 6.0, 16.0),
            );
            painter.rect_filled(text_rect, 2.0, alert_bg);
            painter.text(
                egui::pos2(chart_rect.left() + 5.0, y),
                egui::Align2::LEFT_CENTER,
                &lbl,
                egui::FontId::monospace(9.0),
                alert_col,
            );
        }
    }
}

/// Draw a line segment respecting the per-drawing LineStyle (solid/dashed/dotted).
/// Extracted from technical_analysis draw_chart for modularity.
pub(super) fn draw_styled_line(
    painter: &egui::Painter,
    p1: egui::Pos2,
    p2: egui::Pos2,
    stroke: egui::Stroke,
    style: LineStyle,
) {
    match style {
        LineStyle::Solid => {
            painter.line_segment([p1, p2], stroke);
        }
        LineStyle::Dashed => {
            let dx = p2.x - p1.x;
            let dy = p2.y - p1.y;
            let len = (dx * dx + dy * dy).sqrt();
            if len < 0.1 {
                return;
            }
            let (nx, ny) = (dx / len, dy / len);
            let dash = 8.0f32;
            let gap = 5.0f32;
            let mut t = 0.0f32;
            while t < len {
                let t1 = (t + dash).min(len);
                painter.line_segment(
                    [
                        egui::pos2(p1.x + nx * t, p1.y + ny * t),
                        egui::pos2(p1.x + nx * t1, p1.y + ny * t1),
                    ],
                    stroke,
                );
                t += dash + gap;
            }
        }
        LineStyle::Dotted => {
            let dx = p2.x - p1.x;
            let dy = p2.y - p1.y;
            let len = (dx * dx + dy * dy).sqrt();
            if len < 0.1 {
                return;
            }
            let (nx, ny) = (dx / len, dy / len);
            let dot = stroke.width.max(2.0);
            let gap = 4.0f32;
            let mut t = 0.0f32;
            while t < len {
                let t1 = (t + dot).min(len);
                painter.line_segment(
                    [
                        egui::pos2(p1.x + nx * t, p1.y + ny * t),
                        egui::pos2(p1.x + nx * t1, p1.y + ny * t1),
                    ],
                    stroke,
                );
                t += dot + gap;
            }
        }
    }
}

/// Draw Auto Fibonacci levels (retrace + extensions) and the swing line.
/// Extracted to chart_helpers.rs to shrink the main draw_chart.
pub(super) fn draw_auto_fib_levels(
    painter: &egui::Painter,
    chart: &ChartState,
    chart_rect: egui::Rect,
    data_left: f32,
    bar_w: f32,
    price_to_y: impl Fn(f64) -> f32,
    start_idx: usize,
    end_idx: usize,
    format_price: impl Fn(f64) -> String,
) {
    for (price, label, is_ext) in &chart.auto_fib_levels {
        let y = price_to_y(*price);
        if y >= chart_rect.top() && y <= chart_rect.bottom() {
            let color = if *is_ext {
                egui::Color32::from_rgb(30, 144, 255)
            } else {
                egui::Color32::from_rgb(255, 215, 0)
            };
            painter.line_segment(
                [
                    egui::pos2(chart_rect.left(), y),
                    egui::pos2(chart_rect.right(), y),
                ],
                egui::Stroke::new(1.0, color),
            );
            let mut fib_label = String::with_capacity(label.len() + 24);
            fib_label.push_str(label);
            fib_label.push(' ');
            fib_label.push_str(&format_price(*price));
            painter.text(
                egui::pos2(chart_rect.right() - 4.0, y - 1.0),
                egui::Align2::RIGHT_BOTTOM,
                fib_label,
                egui::FontId::monospace(8.0),
                color,
            );
        }
    }
    // Draw swing line
    if let Some((_high, _low, hi_idx, lo_idx)) = chart.auto_fib_swing {
        if hi_idx >= start_idx && hi_idx < end_idx && lo_idx >= start_idx && lo_idx < end_idx {
            let x1 = data_left + ((hi_idx - start_idx) as f32 + 0.5) * bar_w;
            let y1 = price_to_y(_high);
            let x2 = data_left + ((lo_idx - start_idx) as f32 + 0.5) * bar_w;
            let y2 = price_to_y(_low);
            painter.line_segment(
                [egui::pos2(x1, y1), egui::pos2(x2, y2)],
                egui::Stroke::new(1.0, egui::Color32::WHITE),
            );
        }
    }
}
/// Draw harmonic (XABCD) patterns: lines, point labels, TP/SL.
/// Extracted from draw_chart (technical_analysis.rs) for modularity.
pub(super) fn draw_harmonics(
    painter: &egui::Painter,
    chart: &ChartState,
    chart_rect: egui::Rect,
    data_left: f32,
    bar_w: f32,
    price_to_y: impl Fn(f64) -> f32,
    start_idx: usize,
    end_idx: usize,
    format_price: impl Fn(f64) -> String,
) {
    let pattern_col = egui::Color32::from_rgb(0, 200, 255);
    let tp_col = egui::Color32::from_rgb(0, 200, 80);
    let sl_col = egui::Color32::from_rgb(220, 40, 40);
    for pat in &chart.harmonics {
        let pts = [pat.x, pat.a, pat.b, pat.c, pat.d];
        let screen_pts = pts.map(|(idx, price)| {
            if idx >= start_idx && idx < end_idx {
                Some(egui::pos2(
                    data_left + ((idx - start_idx) as f32 + 0.5) * bar_w,
                    price_to_y(price),
                ))
            } else {
                None
            }
        });
        // XABCD lines
        for w in screen_pts.windows(2) {
            if let (Some(p1), Some(p2)) = (w[0], w[1]) {
                painter.line_segment([p1, p2], egui::Stroke::new(1.5, pattern_col));
            }
        }
        // Labels
        let labels = ["X", "A", "B", "C", "D"];
        for (i, sp) in screen_pts.iter().enumerate() {
            if let Some(p) = sp {
                painter.text(
                    egui::pos2(p.x, p.y + if i % 2 == 0 { -12.0 } else { 4.0 }),
                    egui::Align2::CENTER_TOP,
                    labels[i],
                    egui::FontId::monospace(10.0),
                    pattern_col,
                );
            }
        }
        // Pattern name
        if let Some(d_pt) = screen_pts[4] {
            let dir = if pat.bullish { "BULL" } else { "BEAR" };
            let col = if pat.bullish { UP } else { DOWN };
            painter.text(
                egui::pos2(d_pt.x + 5.0, d_pt.y - 20.0),
                egui::Align2::LEFT_TOP,
                &format!("{} {}", pat.name, dir),
                egui::FontId::monospace(9.0),
                col,
            );
            // TP/SL from D
            for (price, label, c) in [
                (pat.tp1, "TP1", tp_col),
                (pat.tp2, "TP2", tp_col),
                (pat.sl, "SL", sl_col),
            ] {
                let y = price_to_y(price);
                if y >= chart_rect.top() && y <= chart_rect.bottom() {
                    painter.line_segment(
                        [egui::pos2(d_pt.x, y), egui::pos2(chart_rect.right(), y)],
                        egui::Stroke::new(0.7, c),
                    );
                    painter.text(
                        egui::pos2(d_pt.x + 2.0, y - 9.0),
                        egui::Align2::LEFT_TOP,
                        &format!("{} {}", label, format_price(price)),
                        egui::FontId::monospace(8.0),
                        c,
                    );
                }
            }
        }
    }
}
/// Draw supply/demand zones (rects + labels with status).
/// Extracted from draw_chart for modularity.
pub(super) fn draw_supply_demand_zones(
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
