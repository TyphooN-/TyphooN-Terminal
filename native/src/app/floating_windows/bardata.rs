use super::*;

impl TyphooNApp {
    pub(super) fn render_bardata_progress_window(&mut self, ctx: &egui::Context) {
        // BARDATA Progress Window
        if self.show_bardata {
            egui::Window::new("BARDATA Sync")
                .open(&mut self.show_bardata)
                .resizable(true)
                .default_size([450.0, 350.0])
                .show(ctx, |ui| {
                    let total = self.bardata_total;
                    let queued = self.bardata_queued;
                    let completed = self.bardata_completed;
                    let skipped = self.bardata_skipped;
                    let pct = if queued > 0 {
                        ((completed * 100) / queued).min(100)
                    } else {
                        0
                    };

                    // Progress bar
                    ui.label(
                        egui::RichText::new(format!(
                            "Progress: {}/{} fetches ({}%)",
                            completed, queued, pct
                        ))
                        .strong(),
                    );
                    let bar_frac = if queued > 0 {
                        (completed as f32 / queued as f32).min(1.0)
                    } else {
                        0.0
                    };
                    let (bar_rect, _) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 20.0),
                        egui::Sense::hover(),
                    );
                    let painter = ui.painter_at(bar_rect);
                    painter.rect_filled(bar_rect, 2.0, egui::Color32::from_rgb(30, 30, 50));
                    let filled = egui::Rect::from_min_size(
                        bar_rect.min,
                        egui::vec2(bar_rect.width() * bar_frac, 20.0),
                    );
                    painter.rect_filled(filled, 2.0, egui::Color32::from_rgb(0, 200, 100));
                    painter.text(
                        bar_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        format!("{}%", pct),
                        egui::FontId::monospace(11.0),
                        egui::Color32::WHITE,
                    );

                    ui.add_space(4.0);
                    egui::Grid::new("bardata_stats")
                        .num_columns(2)
                        .show(ui, |ui| {
                            ui.label("Total symbols:");
                            ui.label(total.to_string());
                            ui.end_row();
                            ui.label("Queued:");
                            ui.label(egui::RichText::new(queued.to_string()).color(UP));
                            ui.end_row();
                            ui.label("Completed:");
                            ui.label(egui::RichText::new(completed.to_string()).color(UP));
                            ui.end_row();
                            ui.label("Skipped (cached):");
                            ui.label(egui::RichText::new(skipped.to_string()).color(AXIS_TEXT));
                            ui.end_row();
                        });
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Activity Log").small().strong());
                        if self.bardata_active {
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("Stop").color(DOWN).small().strong(),
                                    )
                                    .fill(egui::Color32::from_rgb(60, 20, 20)),
                                )
                                .clicked()
                            {
                                self.bardata_active = false;
                                let line = "BARDATA: stopped by user".to_string();
                                self.bardata_log.push_back(line.clone());
                                self.log.push_back(LogEntry::warn(line));
                            }
                        } else if completed >= queued && queued > 0 {
                            ui.label(egui::RichText::new("Complete").color(UP).small().strong());
                        }
                    });
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .max_height(180.0)
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            for msg in &self.bardata_log {
                                ui.label(
                                    egui::RichText::new(msg)
                                        .monospace()
                                        .small()
                                        .color(AXIS_TEXT),
                                );
                            }
                        });
                });
        }
    }
}
