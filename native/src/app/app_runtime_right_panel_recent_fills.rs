use super::*;

#[allow(deprecated)]
impl TyphooNApp {
    pub(super) fn render_right_panel_recent_fills_section(&mut self, ui: &mut egui::Ui) {
        // ── Recent Fills Section ──────────────────────────────
        let mut visible_recent_fills: Vec<(String, String, f64, f64, String)> = Vec::new();
        if self.alpaca_enabled && self.show_alpaca_positions {
            visible_recent_fills.extend(self.recent_fills.iter().cloned());
        }
        if self.kraken_enabled && self.show_kr_positions {
            visible_recent_fills.extend(self.kraken_trades.iter().take(100).map(|t| {
                let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(t.time as i64, 0)
                    .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| format!("{:.0}", t.time));
                (
                    format!("[Kraken] {}", Self::kraken_base_asset_for_pair(&t.pair)),
                    t.side.clone(),
                    t.vol,
                    t.price,
                    dt,
                )
            }));
        }
        let fills_count2 = visible_recent_fills.len();
        let recent_fills_section = egui::CollapsingHeader::new(
            egui::RichText::new(format!("☰ Recent Fills ({})", fills_count2))
                .strong()
                .small(),
        )
        .id_salt("recent_fills_top")
        .default_open(self.right_recent_fills_open)
        .show(ui, |ui| {
            if visible_recent_fills.is_empty() {
                ui.label(
                    egui::RichText::new("No recent fills.")
                        .color(AXIS_TEXT)
                        .small(),
                );
            } else {
                for (sym, side, qty, price, time) in &visible_recent_fills {
                    let c = if side == "buy" { UP } else { DOWN };
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(sym).small().strong());
                        ui.label(egui::RichText::new(side).color(c).small());
                        ui.label(
                            egui::RichText::new(format!("{:.2} @ {}", qty, format_price(*price)))
                                .small(),
                        );
                        ui.label(egui::RichText::new(time).color(AXIS_TEXT).small());
                    });
                }
            }
        });
        self.right_recent_fills_open = recent_fills_section.fully_open();
        self.handle_right_panel_section_drag(
            ui,
            RightPanelSectionId::RecentFills,
            &recent_fills_section.header_response,
        );
    }
}
