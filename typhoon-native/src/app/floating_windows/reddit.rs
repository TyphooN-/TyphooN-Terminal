use super::*;

impl TyphooNApp {
    pub(super) fn render_reddit_window(&mut self, ctx: &egui::Context) {
        // Reddit WallStreetBets
        if self.show_reddit {
            egui::Window::new("Reddit — r/WallStreetBets")
                .open(&mut self.show_reddit)
                .resizable(true)
                .default_size([550.0, 400.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!("{} posts", self.reddit_posts.len()))
                                .strong(),
                        );
                        if ui.button("Refresh").clicked() {
                            let _ = self.broker_tx.send(BrokerCmd::FetchRedditWSB);
                        }
                    });
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .max_height(340.0)
                        .show(ui, |ui| {
                            for (title, _url, score, comments) in &self.reddit_posts {
                                ui.horizontal(|ui| {
                                    let score_col = if *score > 1000 {
                                        UP
                                    } else if *score > 100 {
                                        egui::Color32::from_rgb(255, 200, 50)
                                    } else {
                                        AXIS_TEXT
                                    };
                                    ui.label(
                                        egui::RichText::new(format!("{}▲", score))
                                            .color(score_col)
                                            .small()
                                            .monospace(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("{}💬", comments))
                                            .color(AXIS_TEXT)
                                            .small()
                                            .monospace(),
                                    );
                                    ui.label(egui::RichText::new(title).small());
                                });
                            }
                        });
                });
        }
    }
}
