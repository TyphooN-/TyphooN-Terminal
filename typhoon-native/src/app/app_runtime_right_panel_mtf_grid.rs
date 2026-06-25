use super::*;
use crate::app::chart_ops::{
    MTF_GRID_TIMEFRAMES, mtf_grid_symbol_key, mtf_grid_symbols_with_missing_timeframes,
    mtf_visible_chart_groups,
};

#[allow(deprecated)]
impl TyphooNApp {
    pub(super) fn render_right_panel_mtf_grid_section(&mut self, ui: &mut egui::Ui) {
        // Keep the cache-loaded all-timeframe status fresh so the grid shows
        // values for EVERY timeframe, not just the ones with an open chart tab.
        // compute_mtf_grid_status loads missing timeframes from cache in the
        // background. We (re)trigger it — only when no load is already in flight,
        // so the render path never re-spawns the loader every frame — when:
        //   • the active symbol changed (whole snapshot is now wrong), or
        //   • a chart was opened/closed/retimeframed (a just-closed timeframe
        //     would otherwise fall back to a stale/empty cell — the reported bug), or
        //   • some cell is still empty and a short throttle elapsed, so timeframes
        //     the async all-TF sync fills into the cache appear on their own. This
        //     last path is self-terminating: once every cell is filled it stops.
        if self.right_mtf_grid_open
            && self.cache.is_some()
            && self.mtf_grid_rx.is_none()
            && !self.symbol_input.trim().is_empty()
        {
            let missing_symbols = if self.mtf_enabled {
                mtf_grid_symbols_with_missing_timeframes(&self.charts, &self.mtf_visible)
            } else {
                Vec::new()
            };
            for symbol in missing_symbols {
                self.ensure_mtf_grid_for_symbol(&symbol);
            }
            let symbol_changed = self.mtf_grid_status_symbol != self.symbol_input.trim();
            let open_changed = self.mtf_grid_status_open_sig != self.mtf_open_chart_signature();
            let has_missing = self.mtf_grid_status.len() < MTF_GRID_TIMEFRAMES.len()
                || self.mtf_grid_status.iter().any(|s| {
                    s.1.is_none()
                        && s.2.is_none()
                        && s.3.is_none()
                        && s.4.is_none()
                        && s.5.is_none()
                });
            let throttle_ok = self
                .mtf_grid_status_at
                .map(|t| t.elapsed().as_secs() >= 6)
                .unwrap_or(true);
            if symbol_changed || open_changed || (has_missing && throttle_ok) {
                self.compute_mtf_grid_status();
            }
        }
        // ── MTF Grid ────────────────────────────────────────
        let mtf_grid_section = egui::CollapsingHeader::new(
            egui::RichText::new("☰ MTF Grid")
                .color(AXIS_TEXT)
                .small()
                .strong(),
        )
        .id_salt("mtf_grid_section")
        .default_open(self.right_mtf_grid_open)
        .show(ui, |ui| {
            let tf_labels: Vec<&'static str> = MTF_GRID_TIMEFRAMES
                .iter()
                .map(|(label, _)| *label)
                .collect();
            let ma_labels = ["SMA200", "KAMA", "Fisher"];
            // Symbol count only — the Fetch News button moved into the News
            // section header and now picks the multi-symbol path automatically
            // when MTF mode has >1 symbol, so we no longer render a second one
            // here.
            let mtf_news_symbols = self.mtf_grid_news_symbols();
            let mtf_groups = mtf_visible_chart_groups(&self.charts, &self.mtf_visible);
            ui.label(
                egui::RichText::new(format!(
                    "{} symbol{} · {} TFs",
                    mtf_news_symbols.len(),
                    if mtf_news_symbols.len() == 1 { "" } else { "s" },
                    tf_labels.len()
                ))
                .color(AXIS_TEXT)
                .small(),
            );
            let cached_status_by_tf: std::collections::HashMap<
                &'static str,
                (
                    Option<f64>,
                    Option<f64>,
                    Option<f64>,
                    Option<f64>,
                    Option<f64>,
                ),
            > = self
                .mtf_grid_status
                .iter()
                .map(|&(tf, close, sma, kama, fisher, fsig)| (tf, (close, sma, kama, fisher, fsig)))
                .collect();
            if mtf_groups.is_empty() {
                egui::Grid::new("mtf_ma_grid")
                    .spacing(egui::vec2(4.0, 2.0))
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("").small());
                        for tf in &tf_labels {
                            ui.label(egui::RichText::new(*tf).color(AXIS_TEXT).small());
                        }
                        ui.end_row();
                        for ma in &ma_labels {
                            ui.label(egui::RichText::new(*ma).color(AXIS_TEXT).small());
                            for tf in &tf_labels {
                                let dot_color = if let Some(&(close, sma, kama, fisher, fsig)) =
                                    cached_status_by_tf.get(tf)
                                {
                                    let bullish = match *ma {
                                        "SMA200" => match (close, sma) {
                                            (Some(c), Some(s)) => Some(c > s),
                                            _ => None,
                                        },
                                        "KAMA" => match (close, kama) {
                                            (Some(c), Some(k)) => Some(c > k),
                                            _ => None,
                                        },
                                        "Fisher" => match (fisher, fsig) {
                                            (Some(f), Some(s)) => Some(f > s),
                                            _ => None,
                                        },
                                        _ => None,
                                    };
                                    match bullish {
                                        Some(true) => UP,
                                        Some(false) => DOWN,
                                        None => AXIS_TEXT,
                                    }
                                } else {
                                    egui::Color32::from_rgb(50, 50, 60)
                                };
                                ui.label(egui::RichText::new("\u{25CF}").color(dot_color).small());
                            }
                            ui.end_row();
                        }
                    });
            } else {
                // The cache-loaded all-timeframe status is computed for the
                // active symbol; use it to fill timeframes that have no open
                // chart so the grid is not limited to open tabs.
                let active_sym_key = mtf_grid_symbol_key(&self.symbol_input);
                for group in mtf_groups {
                    ui.label(
                        egui::RichText::new(&group.symbol)
                            .color(ACCENT)
                            .small()
                            .strong(),
                    );
                    let open_status_by_tf: std::collections::HashMap<
                        &'static str,
                        (
                            Option<f64>,
                            Option<f64>,
                            Option<f64>,
                            Option<f64>,
                            Option<f64>,
                        ),
                    > = group
                        .indices
                        .iter()
                        .filter_map(|idx| self.charts.get(*idx))
                        .map(|chart| {
                            (
                                chart.timeframe.label(),
                                (
                                    chart
                                        .fresh_live_quote_mid()
                                        .or_else(|| chart.bars.last().map(|b| b.close)),
                                    chart.sma200.last().and_then(|v| *v),
                                    chart.kama.last().and_then(|v| *v),
                                    chart.fisher.last().and_then(|v| *v),
                                    chart.fisher_signal.last().and_then(|v| *v),
                                ),
                            )
                        })
                        .collect();
                    egui::Grid::new(format!("mtf_ma_grid_{}", group.symbol))
                        .spacing(egui::vec2(4.0, 2.0))
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("").small());
                            for tf in &tf_labels {
                                ui.label(egui::RichText::new(*tf).color(AXIS_TEXT).small());
                            }
                            ui.end_row();
                            for ma in &ma_labels {
                                ui.label(egui::RichText::new(*ma).color(AXIS_TEXT).small());
                                for tf in &tf_labels {
                                    let status = open_status_by_tf.get(tf).copied().or_else(|| {
                                        // No open chart for this timeframe — fall
                                        // back to the cache-loaded status (active
                                        // symbol only; it is the one we precompute).
                                        if mtf_grid_symbol_key(&group.symbol)
                                            .eq_ignore_ascii_case(&active_sym_key)
                                        {
                                            cached_status_by_tf.get(tf).copied()
                                        } else {
                                            None
                                        }
                                    });
                                    let dot_color =
                                        if let Some((close, sma, kama, fisher, fsig)) = status {
                                            let bullish = match *ma {
                                                "SMA200" => match (close, sma) {
                                                    (Some(c), Some(s)) => Some(c > s),
                                                    _ => None,
                                                },
                                                "KAMA" => match (close, kama) {
                                                    (Some(c), Some(k)) => Some(c > k),
                                                    _ => None,
                                                },
                                                "Fisher" => match (fisher, fsig) {
                                                    (Some(f), Some(s)) => Some(f > s),
                                                    _ => None,
                                                },
                                                _ => None,
                                            };
                                            match bullish {
                                                Some(true) => UP,
                                                Some(false) => DOWN,
                                                None => AXIS_TEXT,
                                            }
                                        } else {
                                            egui::Color32::from_rgb(50, 50, 60)
                                        };
                                    ui.label(
                                        egui::RichText::new("\u{25CF}").color(dot_color).small(),
                                    );
                                }
                                ui.end_row();
                            }
                        });
                }
            }
        });
        self.right_mtf_grid_open = mtf_grid_section.fully_open();
        self.handle_right_panel_section_drag(
            ui,
            RightPanelSectionId::MtfGrid,
            &mtf_grid_section.header_response,
        );
    }
}
