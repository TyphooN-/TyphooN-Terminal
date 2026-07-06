//! SMA Intelligence window (ADR-131).
//!
//! Per-outfit SMA stack/trigger analysis for the focused chart's bars.
//! Concept credit: "SMA Outfits" by raultrades / Unfair Market
//! (github.com/raultrades/SMA-outfits) and the Apache-2.0
//! sma-intelligence-platform (niya-shroff). All math is local and
//! deterministic — see `typhoon_chart_ui::sma_outfits`.
use super::*;
use typhoon_chart_ui::sma_outfits::{
    OutfitStack, analyze_sma_outfit, outfit_label, parse_outfit_spec,
};

/// ± band (percent of the SMA value) inside which price counts as sitting on
/// an institutional trigger level.
const SMA_TRIGGER_BAND_PCT: f64 = 0.5;
/// How far back the most-recent-cross scan walks.
const SMA_CROSS_LOOKBACK: usize = 200;

impl TyphooNApp {
    pub(super) fn render_sma_intelligence_window(&mut self, ctx: &egui::Context) {
        if !self.show_sma_intelligence {
            return;
        }
        let chart_idx = self.mtf_focused.unwrap_or(self.active_tab);
        let (symbol, tf_label, bars_len, reports) = match self.charts.get(chart_idx) {
            Some(chart) => {
                let reports: Vec<_> = self
                    .sma_outfits
                    .iter()
                    .map(|periods| {
                        analyze_sma_outfit(
                            &chart.bars,
                            periods,
                            SMA_TRIGGER_BAND_PCT,
                            SMA_CROSS_LOOKBACK,
                        )
                    })
                    .collect();
                (
                    chart.symbol.clone(),
                    chart.timeframe.label().to_string(),
                    chart.bars.len(),
                    reports,
                )
            }
            None => (String::new(), String::new(), 0, Vec::new()),
        };

        let mut remove_outfit: Option<usize> = None;
        let mut add_outfit: Option<Vec<usize>> = None;
        let mut reset_defaults = false;

        egui::Window::new("SMA Intelligence")
            .open(&mut self.show_sma_intelligence)
            .resizable(true)
            .default_size([560.0, 420.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("{symbol} [{tf_label}] — {bars_len} bars"))
                            .strong()
                            .small(),
                    );
                    ui.label(
                        egui::RichText::new(format!(
                            "trigger band ±{SMA_TRIGGER_BAND_PCT}%"
                        ))
                        .color(AXIS_TEXT)
                        .small(),
                    );
                });
                ui.separator();

                egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                    for (i, report) in reports.iter().enumerate() {
                        let stack_col = match report.stack {
                            OutfitStack::Bullish => UP,
                            OutfitStack::Bearish => DOWN,
                            OutfitStack::Mixed => AXIS_TEXT,
                        };
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "Outfit {}",
                                    outfit_label(&report.periods)
                                ))
                                .strong()
                                .monospace(),
                            );
                            ui.label(
                                egui::RichText::new(report.stack.label())
                                    .color(stack_col)
                                    .strong()
                                    .small(),
                            );
                            ui.label(
                                egui::RichText::new(format!(
                                    "alignment {:.0}%",
                                    report.alignment_pct
                                ))
                                .color(stack_col)
                                .small(),
                            );
                            if report.insufficient_history {
                                ui.label(
                                    egui::RichText::new("insufficient history")
                                        .color(DOWN)
                                        .small(),
                                );
                            }
                            if ui.small_button("\u{2716}").clicked() {
                                remove_outfit = Some(i);
                            }
                        });
                        egui::Grid::new(format!("sma_outfit_grid_{i}"))
                            .striped(true)
                            .num_columns(5)
                            .show(ui, |ui| {
                                ui.strong("SMA");
                                ui.strong("Value");
                                ui.strong("Δ price");
                                ui.strong("Trigger");
                                ui.strong("Last cross");
                                ui.end_row();
                                for leg in &report.legs {
                                    ui.label(
                                        egui::RichText::new(format!("{}", leg.period))
                                            .monospace()
                                            .small(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format_price(leg.value))
                                            .monospace()
                                            .small(),
                                    );
                                    let d_col = if leg.price_delta_pct > 0.0 { UP } else { DOWN };
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{:+.2}%",
                                            leg.price_delta_pct
                                        ))
                                        .color(d_col)
                                        .small(),
                                    );
                                    if leg.at_trigger {
                                        ui.label(
                                            egui::RichText::new("AT TRIGGER")
                                                .color(egui::Color32::from_rgb(255, 200, 60))
                                                .strong()
                                                .small(),
                                        );
                                    } else {
                                        ui.label(egui::RichText::new("—").color(AXIS_TEXT).small());
                                    }
                                    match leg.bars_since_cross {
                                        Some(back) => {
                                            let (arrow, c_col) = if leg.last_cross_up {
                                                ("\u{2191}", UP)
                                            } else {
                                                ("\u{2193}", DOWN)
                                            };
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{arrow} {back} bars ago"
                                                ))
                                                .color(c_col)
                                                .small(),
                                            );
                                        }
                                        None => {
                                            ui.label(
                                                egui::RichText::new("none in lookback")
                                                    .color(AXIS_TEXT)
                                                    .small(),
                                            );
                                        }
                                    }
                                    ui.end_row();
                                }
                            });
                        ui.add_space(6.0);
                    }

                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Add outfit:").small());
                        ui.add(
                            egui::TextEdit::singleline(&mut self.sma_outfit_input)
                                .desired_width(120.0)
                                .hint_text("10/50/200"),
                        );
                        if ui.small_button("Add").clicked() {
                            match parse_outfit_spec(&self.sma_outfit_input) {
                                Some(periods) => add_outfit = Some(periods),
                                None => {
                                    self.log.push_back(LogEntry::warn(
                                        "SMA outfit spec invalid — need 2-6 periods in 1..=999, e.g. 10/50/200".to_string(),
                                    ));
                                }
                            }
                        }
                        if ui.small_button("Reset defaults").clicked() {
                            reset_defaults = true;
                        }
                    });
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(
                            "Concept: \"SMA Outfits\" — raultrades / Unfair Market \
                             (github.com/raultrades/SMA-outfits); sma-intelligence-platform \
                             (niya-shroff, Apache-2.0). Local bar math only.",
                        )
                        .color(AXIS_TEXT)
                        .small(),
                    );
                });
            });

        if let Some(i) = remove_outfit {
            if i < self.sma_outfits.len() {
                self.sma_outfits.remove(i);
            }
        }
        if let Some(periods) = add_outfit {
            if !self.sma_outfits.contains(&periods) {
                self.sma_outfits.push(periods);
            }
            self.sma_outfit_input.clear();
        }
        if reset_defaults {
            self.sma_outfits = typhoon_chart_ui::sma_outfits::default_sma_outfits();
        }
    }
}
