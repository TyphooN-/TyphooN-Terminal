use super::*;

impl TyphooNApp {
    pub(super) fn render_sync_status_window(&mut self, ctx: &egui::Context) {
        if !self.show_sync_status {
            return;
        }
        let rows = compute_bar_sync_stats(&self.bg.detailed_stats, &self.bg.bar_ts_cache);
        let broker_totals = compute_bar_sync_broker_totals(&rows);
        let mut sync_save_after = false;
        let mut show_sync_status = self.show_sync_status;
        egui::Window::new("Sync Status")
            .open(&mut show_sync_status)
            .resizable(true).default_size([560.0, 480.0])
            .scroll([false, true])
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("Bar sync % per broker / timeframe").color(AXIS_TEXT).small());
                ui.label(egui::RichText::new("healthy = last bar within 24× TF period · stale beyond · empty = cached blob has no bars").color(AXIS_TEXT).small());
                ui.horizontal(|ui| {
                    if ui.checkbox(&mut self.crypto_backfill_enabled, egui::RichText::new("Crypto backfill").small())
                        .on_hover_text("Enable Alpaca crypto + CryptoCompare rotation. Kraken public scraping is controlled in Settings.")
                        .changed()
                    {
                        sync_save_after = true;
                    }
                });
                self.render_alpaca_sync_profile_controls(ui, &mut sync_save_after, "sync_status");
                self.render_sync_timeframe_controls(ui, &mut sync_save_after);
                ui.separator();

                // Per-broker summary chips
                ui.horizontal_wrapped(|ui| {
                    for (broker, total, healthy, pct) in &broker_totals {
                        let color = if *total == 0 {
                            egui::Color32::from_rgb(150, 150, 150)
                        } else if *pct >= 90.0 {
                            egui::Color32::from_rgb(26, 188, 156)
                        } else if *pct >= 50.0 {
                            egui::Color32::from_rgb(241, 196, 15)
                        } else {
                            egui::Color32::from_rgb(231, 76, 60)
                        };
                        ui.label(egui::RichText::new(format!(
                            "{}: {:.1}% ({}/{})",
                            broker, pct, healthy, total,
                        )).color(color).monospace().strong());
                        ui.label(egui::RichText::new("|").color(AXIS_TEXT));
                    }
                });
                ui.separator();

                egui::ScrollArea::vertical().id_salt("sync_scroll").auto_shrink(false).show(ui, |ui| {
                    egui::Grid::new("sync_grid").striped(true).num_columns(6).min_col_width(60.0).show(ui, |ui| {
                        ui.label(egui::RichText::new("Broker").color(AXIS_TEXT).small().strong());
                        ui.label(egui::RichText::new("TF").color(AXIS_TEXT).small().strong());
                        ui.label(egui::RichText::new("Symbols").color(AXIS_TEXT).small().strong());
                        ui.label(egui::RichText::new("Healthy").color(AXIS_TEXT).small().strong());
                        ui.label(egui::RichText::new("Stale").color(AXIS_TEXT).small().strong());
                        ui.label(egui::RichText::new("% Synced").color(AXIS_TEXT).small().strong());
                        ui.end_row();
                        for row in &rows {
                            let broker_color = match row.broker.as_str() {
                                "MT5"           => egui::Color32::from_rgb(26, 188, 156),
                                "Alpaca"        => egui::Color32::from_rgb(52, 152, 219),
                                "Tastytrade"    => egui::Color32::from_rgb(170, 100, 220),
                                "CryptoCompare" => egui::Color32::from_rgb(200, 170, 80),
                                "Kraken"        => egui::Color32::from_rgb(255, 130, 60),
                                _ => AXIS_TEXT,
                            };
                            ui.label(egui::RichText::new(&row.broker).color(broker_color).small().monospace().strong());
                            ui.label(egui::RichText::new(&row.tf).color(AXIS_TEXT).small().monospace());
                            ui.label(egui::RichText::new(format!("{}", row.total)).small());
                            ui.label(egui::RichText::new(format!("{}", row.healthy)).color(egui::Color32::from_rgb(26, 188, 156)).small());
                            ui.label(egui::RichText::new(format!("{}", row.stale + row.empty)).color(AXIS_TEXT).small());
                            let pct_color = if row.total == 0 {
                                egui::Color32::from_rgb(150, 150, 150)
                            } else if row.pct_healthy >= 90.0 {
                                egui::Color32::from_rgb(26, 188, 156)
                            } else if row.pct_healthy >= 50.0 {
                                egui::Color32::from_rgb(241, 196, 15)
                            } else {
                                egui::Color32::from_rgb(231, 76, 60)
                            };
                            ui.label(egui::RichText::new(format!("{:.1}%", row.pct_healthy))
                                .color(pct_color).small().strong());
                            ui.end_row();
                        }
                    });
                });
            });
        self.show_sync_status = show_sync_status;
        if sync_save_after {
            self.save_session();
        }
    }
}
