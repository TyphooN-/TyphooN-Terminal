use super::*;

/// Draw all enabled lower indicator sub-panes below the main chart.
pub(crate) fn draw_enabled_sub_panes(
    painter: &egui::Painter,
    chart: &ChartState,
    rect: egui::Rect,
    main_rect: egui::Rect,
    price_axis_w: f32,
    bars: &[Bar],
    start_idx: usize,
    bar_w: f32,
    show_rsi: bool,
    show_fisher: bool,
    show_macd: bool,
    show_volume_pane: bool,
    show_stochastic: bool,
    show_adx: bool,
    show_cci: bool,
    show_williams_r: bool,
    show_obv: bool,
    show_momentum: bool,
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
    show_better_volume: bool,
    show_ehlers_ebsw: bool,
    show_ehlers_cyber: bool,
    show_ehlers_cg: bool,
    show_ehlers_roof: bool,
    show_squeeze: bool,
) {
    // ── sub-panes (RSI, Fisher) ──────────────────────────────────────────────
    let mut sub_y = main_rect.bottom();

    if show_rsi {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_oscillator_pane(
            painter,
            pane_rect,
            bars,
            &chart.rsi,
            start_idx,
            bar_w,
            "RSI(14)",
            RSI_LINE,
            0.0,
            100.0,
            Some(70.0),
            Some(30.0),
        );
        sub_y += 80.0;
    }

    if show_fisher {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_fisher_pane(
            painter,
            pane_rect,
            bars,
            &chart.fisher,
            &chart.fisher_signal,
            start_idx,
            bar_w,
        );
        sub_y += 80.0;
    }

    if show_macd {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_macd_pane(
            painter,
            pane_rect,
            bars,
            &chart.macd_line,
            &chart.macd_signal,
            &chart.macd_hist,
            start_idx,
            bar_w,
            "MACD(12,26,9)",
            MACD_LINE_COL,
            MACD_SIG_COL,
        );
        sub_y += 80.0;
    }

    if show_volume_pane {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_volume_pane(painter, pane_rect, bars, start_idx, bar_w);
        sub_y += 80.0;
    }

    if show_stochastic {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_stoch_pane(
            painter,
            pane_rect,
            bars,
            &chart.stoch_k,
            &chart.stoch_d,
            start_idx,
            bar_w,
            "Stoch(14,3,3)",
        );
        sub_y += 80.0;
    }

    if show_adx {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_adx_pane(
            painter,
            pane_rect,
            bars,
            &chart.adx,
            &chart.di_plus,
            &chart.di_minus,
            start_idx,
            bar_w,
        );
        sub_y += 80.0;
    }

    if show_cci {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_oscillator_pane(
            painter,
            pane_rect,
            bars,
            &chart.cci,
            start_idx,
            bar_w,
            "CCI(20)",
            CCI_COL,
            -200.0,
            200.0,
            Some(100.0),
            Some(-100.0),
        );
        sub_y += 80.0;
    }

    if show_williams_r {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_oscillator_pane(
            painter,
            pane_rect,
            bars,
            &chart.williams_r,
            start_idx,
            bar_w,
            "Williams %R(14)",
            WILLR_COL,
            -100.0,
            0.0,
            Some(-20.0),
            Some(-80.0),
        );
        sub_y += 80.0;
    }

    if show_obv {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        // OBV auto-scales
        let mut ob_min = f64::MAX;
        let mut ob_max = f64::MIN;
        for (ri, _) in bars.iter().enumerate() {
            if let Some(Some(v)) = chart.obv.get(start_idx + ri) {
                ob_min = ob_min.min(*v);
                ob_max = ob_max.max(*v);
            }
        }
        let pad = (ob_max - ob_min) * 0.1;
        draw_oscillator_pane(
            painter,
            pane_rect,
            bars,
            &chart.obv,
            start_idx,
            bar_w,
            "OBV",
            OBV_COL,
            ob_min - pad,
            ob_max + pad,
            None,
            None,
        );
        sub_y += 80.0;
    }

    if show_momentum {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        let mut m_min = f64::MAX;
        let mut m_max = f64::MIN;
        for (ri, _) in bars.iter().enumerate() {
            if let Some(Some(v)) = chart.momentum.get(start_idx + ri) {
                m_min = m_min.min(*v);
                m_max = m_max.max(*v);
            }
        }
        let pad = (m_max - m_min).max(0.001) * 0.1;
        draw_oscillator_pane(
            painter,
            pane_rect,
            bars,
            &chart.momentum,
            start_idx,
            bar_w,
            "Momentum(10)",
            egui::Color32::from_rgb(200, 150, 100),
            m_min - pad,
            m_max + pad,
            None,
            None,
        );
        sub_y += 80.0;
    }

    if show_cmo {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_oscillator_pane(
            painter,
            pane_rect,
            bars,
            &chart.cmo,
            start_idx,
            bar_w,
            "CMO(9)",
            egui::Color32::from_rgb(120, 220, 200),
            -100.0,
            100.0,
            Some(50.0),
            Some(-50.0),
        );
        sub_y += 80.0;
    }

    if show_qstick {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        let mut bound = 0.0_f64;
        for (ri, _) in bars.iter().enumerate() {
            if let Some(Some(v)) = chart.qstick.get(start_idx + ri) {
                bound = bound.max(v.abs());
            }
        }
        let bound = bound.max(0.001);
        let pad = bound * 0.1;
        draw_oscillator_pane(
            painter,
            pane_rect,
            bars,
            &chart.qstick,
            start_idx,
            bar_w,
            "QStick(14)",
            egui::Color32::from_rgb(190, 140, 255),
            -(bound + pad),
            bound + pad,
            None,
            None,
        );
        sub_y += 80.0;
    }

    if show_disparity {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        let mut bound = 3.0_f64;
        for (ri, _) in bars.iter().enumerate() {
            if let Some(Some(v)) = chart.disparity.get(start_idx + ri) {
                bound = bound.max(v.abs());
            }
        }
        let pad = bound * 0.1;
        draw_oscillator_pane(
            painter,
            pane_rect,
            bars,
            &chart.disparity,
            start_idx,
            bar_w,
            "Disparity(14)",
            egui::Color32::from_rgb(255, 210, 90),
            -(bound + pad),
            bound + pad,
            Some(3.0),
            Some(-3.0),
        );
        sub_y += 80.0;
    }

    if show_bop {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_oscillator_pane(
            painter,
            pane_rect,
            bars,
            &chart.bop,
            start_idx,
            bar_w,
            "BOP(14)",
            egui::Color32::from_rgb(255, 120, 120),
            -1.0,
            1.0,
            Some(0.5),
            Some(-0.5),
        );
        sub_y += 80.0;
    }

    if show_stddev {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        let mut s_max = 0.0_f64;
        for (ri, _) in bars.iter().enumerate() {
            if let Some(Some(v)) = chart.stddev.get(start_idx + ri) {
                s_max = s_max.max(*v);
            }
        }
        let s_max = s_max.max(1.0);
        let pad = s_max * 0.1;
        draw_oscillator_pane(
            painter,
            pane_rect,
            bars,
            &chart.stddev,
            start_idx,
            bar_w,
            "StdDev(20)",
            egui::Color32::from_rgb(120, 180, 255),
            0.0,
            s_max + pad,
            None,
            None,
        );
        sub_y += 80.0;
    }

    if show_mfi {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_oscillator_pane(
            painter,
            pane_rect,
            bars,
            &chart.mfi,
            start_idx,
            bar_w,
            "MFI(14)",
            MFI_COL,
            0.0,
            100.0,
            Some(80.0),
            Some(20.0),
        );
        sub_y += 80.0;
    }

    if show_trix {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_macd_pane(
            painter,
            pane_rect,
            bars,
            &chart.trix_line,
            &chart.trix_signal,
            &chart.trix_hist,
            start_idx,
            bar_w,
            "TRIX(15,9)",
            TRIX_LINE_COL,
            TRIX_SIG_COL,
        );
        sub_y += 80.0;
    }

    if show_ppo {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_macd_pane(
            painter,
            pane_rect,
            bars,
            &chart.ppo_line,
            &chart.ppo_signal,
            &chart.ppo_hist,
            start_idx,
            bar_w,
            "PPO(12,26,9)",
            PPO_LINE_COL,
            PPO_SIG_COL,
        );
        sub_y += 80.0;
    }

    if show_ultosc {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_oscillator_pane(
            painter,
            pane_rect,
            bars,
            &chart.ultosc,
            start_idx,
            bar_w,
            "ULTOSC(7,14,28)",
            ULTOSC_COL,
            0.0,
            100.0,
            Some(70.0),
            Some(30.0),
        );
        sub_y += 80.0;
    }

    if show_stochrsi {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_stoch_pane(
            painter,
            pane_rect,
            bars,
            &chart.stochrsi_k,
            &chart.stochrsi_d,
            start_idx,
            bar_w,
            "StochRSI(14,14,3,3)",
        );
        sub_y += 80.0;
    }

    if show_var_oscillator {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        let mut bound = 100.0_f64;
        for (ri, _) in bars.iter().enumerate() {
            if let Some(Some(v)) = chart.var_oscillator.get(start_idx + ri) {
                bound = bound.max(v.abs());
            }
        }
        let bound = bound.max(100.0);
        let pad = bound * 0.1;
        draw_oscillator_pane(
            painter,
            pane_rect,
            bars,
            &chart.var_oscillator,
            start_idx,
            bar_w,
            "VaR Osc(20,95%)",
            egui::Color32::from_rgb(255, 170, 80),
            -(bound + pad),
            bound + pad,
            Some(100.0),
            Some(-100.0),
        );
        sub_y += 80.0;
    }

    if show_better_volume {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_better_volume_pane(
            painter,
            pane_rect,
            bars,
            &chart.better_vol_type,
            start_idx,
            bar_w,
        );
        sub_y += 80.0;
    }

    // Ehlers sub-panes
    if show_ehlers_ebsw {
        let pr = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_oscillator_pane(
            painter,
            pr,
            bars,
            &chart.ehlers_ebsw,
            start_idx,
            bar_w,
            "EBSW",
            EHLERS_EBSW_COL,
            -1.0,
            1.0,
            None,
            None,
        );
        sub_y += 80.0;
    }
    if show_ehlers_cyber {
        let pr = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        let mut cmin = f64::MAX;
        let mut cmax = f64::MIN;
        for (ri, _) in bars.iter().enumerate() {
            if let Some(Some(v)) = chart.ehlers_cyber.get(start_idx + ri) {
                cmin = cmin.min(*v);
                cmax = cmax.max(*v);
            }
        }
        let pad = (cmax - cmin).max(0.001) * 0.1;
        draw_oscillator_pane(
            painter,
            pr,
            bars,
            &chart.ehlers_cyber,
            start_idx,
            bar_w,
            "Cyber Cycle",
            EHLERS_CYBER_COL,
            cmin - pad,
            cmax + pad,
            None,
            None,
        );
        sub_y += 80.0;
    }
    if show_ehlers_cg {
        let pr = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        let mut cmin = f64::MAX;
        let mut cmax = f64::MIN;
        for (ri, _) in bars.iter().enumerate() {
            if let Some(Some(v)) = chart.ehlers_cg.get(start_idx + ri) {
                cmin = cmin.min(*v);
                cmax = cmax.max(*v);
            }
        }
        let pad = (cmax - cmin).max(0.001) * 0.1;
        draw_oscillator_pane(
            painter,
            pr,
            bars,
            &chart.ehlers_cg,
            start_idx,
            bar_w,
            "CG Oscillator",
            EHLERS_CG_COL,
            cmin - pad,
            cmax + pad,
            None,
            None,
        );
        sub_y += 80.0;
    }
    if show_ehlers_roof {
        let pr = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        let mut cmin = f64::MAX;
        let mut cmax = f64::MIN;
        for (ri, _) in bars.iter().enumerate() {
            if let Some(Some(v)) = chart.ehlers_roof.get(start_idx + ri) {
                cmin = cmin.min(*v);
                cmax = cmax.max(*v);
            }
        }
        let pad = (cmax - cmin).max(0.001) * 0.1;
        draw_oscillator_pane(
            painter,
            pr,
            bars,
            &chart.ehlers_roof,
            start_idx,
            bar_w,
            "Roofing Filter",
            EHLERS_ROOF_COL,
            cmin - pad,
            cmax + pad,
            None,
            None,
        );
    }

    // ── Squeeze Momentum sub-pane ──────────────────────────────────────────
    if show_squeeze {
        let sr = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        #[allow(unused_assignments)]
        {
            sub_y += 80.0;
        } // last sub-pane
        painter.rect_filled(sr, 0.0, egui::Color32::from_rgb(0, 0, 0));
        painter.line_segment(
            [
                egui::pos2(sr.left(), sr.top()),
                egui::pos2(sr.right(), sr.top()),
            ],
            egui::Stroke::new(1.0, egui::Color32::from_rgb(68, 68, 68)),
        );
        // Find momentum range
        let mut mom_min = f64::MAX;
        let mut mom_max = f64::MIN;
        for (ri, _) in bars.iter().enumerate() {
            if let Some(Some(v)) = chart.squeeze_mom.get(start_idx + ri) {
                mom_min = mom_min.min(*v);
                mom_max = mom_max.max(*v);
            }
        }
        if mom_min >= mom_max {
            mom_min = -1.0;
            mom_max = 1.0;
        }
        let pad = (mom_max - mom_min) * 0.1;
        mom_min -= pad;
        mom_max += pad;
        let val_to_y = |v: f64| -> f32 {
            sr.top() + ((mom_max - v) / (mom_max - mom_min)) as f32 * sr.height()
        };
        let zero_y = val_to_y(0.0);
        // Zero line
        painter.line_segment(
            [
                egui::pos2(sr.left(), zero_y),
                egui::pos2(sr.right(), zero_y),
            ],
            egui::Stroke::new(0.5, egui::Color32::from_rgb(60, 60, 60)),
        );
        // Histogram bars
        for (ri, _) in bars.iter().enumerate() {
            let abs_idx = start_idx + ri;
            if let Some(Some(v)) = chart.squeeze_mom.get(abs_idx) {
                let x = sr.left() + (ri as f32 + 0.5) * bar_w;
                let y = val_to_y(*v);
                let is_squeeze = chart.squeeze_on.get(abs_idx).copied().unwrap_or(false);
                // Color: squeeze=gray, released: positive=cyan, negative=red
                // Momentum direction: increasing=brighter, decreasing=dimmer
                let prev_v = if abs_idx > 0 {
                    chart
                        .squeeze_mom
                        .get(abs_idx - 1)
                        .and_then(|v| *v)
                        .unwrap_or(0.0)
                } else {
                    0.0
                };
                let color = if is_squeeze {
                    egui::Color32::from_rgb(100, 100, 100) // gray = squeeze active
                } else if *v > 0.0 {
                    if *v > prev_v {
                        egui::Color32::from_rgb(0, 220, 200)
                    } else {
                        egui::Color32::from_rgb(0, 120, 100)
                    }
                } else {
                    if *v < prev_v {
                        egui::Color32::from_rgb(220, 50, 50)
                    } else {
                        egui::Color32::from_rgb(120, 30, 30)
                    }
                };
                let half = (bar_w * 0.35).max(0.5);
                painter.rect_filled(
                    egui::Rect::from_min_max(
                        egui::pos2(x - half, y.min(zero_y)),
                        egui::pos2(x + half, y.max(zero_y)),
                    ),
                    0.0,
                    color,
                );
            }
        }
        // Label
        painter.text(
            egui::pos2(sr.left() + 4.0, sr.top() + 2.0),
            egui::Align2::LEFT_TOP,
            "Squeeze",
            egui::FontId::monospace(9.0),
            AXIS_TEXT,
        );
    }
}
