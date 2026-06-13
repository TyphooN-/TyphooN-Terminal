use super::*;
use crate::app::chart_ops::{MTF_GRID_TIMEFRAMES, mtf_visible_chart_groups};

#[allow(deprecated)]
impl TyphooNApp {
    pub(super) fn render_right_panel_mtf_grid_section(&mut self, ui: &mut egui::Ui) {
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
                                let status = self.mtf_grid_status.iter().find(|s| s.0 == *tf);
                                let dot_color =
                                    if let Some(&(_, close, sma, kama, fisher, fsig)) = status {
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
                for group in mtf_groups {
                    ui.label(
                        egui::RichText::new(&group.symbol)
                            .color(ACCENT)
                            .small()
                            .strong(),
                    );
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
                                    let tf_match =
                                        |chart: &&ChartState| chart.timeframe.label() == *tf;
                                    let status = group
                                        .indices
                                        .iter()
                                        .filter_map(|idx| self.charts.get(*idx))
                                        .find(tf_match)
                                        .map(|chart| {
                                            (
                                                chart
                                                    .fresh_live_quote_mid()
                                                    .or_else(|| chart.bars.last().map(|b| b.close)),
                                                chart.sma200.last().and_then(|v| *v),
                                                chart.kama.last().and_then(|v| *v),
                                                chart.fisher.last().and_then(|v| *v),
                                                chart.fisher_signal.last().and_then(|v| *v),
                                            )
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
