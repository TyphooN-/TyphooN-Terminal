use super::*;

impl TyphooNApp {
    pub(super) fn render_scope_window(&mut self, ctx: &egui::Context) {
        // ── SCOPE popup window with source checkboxes ──
        if self.show_scope_window {
            egui::Window::new("Scope — Symbol Sources")
                .open(&mut self.show_scope_window)
                .resizable(false)
                .default_size([320.0, 240.0])
                .show(ctx, |ui| {
                    ui.label("Symbol sources for fundamentals scraping + analytics:");
                    ui.add_space(4.0);
                    ui.checkbox(&mut self.fund_source_alpaca, "Alpaca");
                    ui.checkbox(&mut self.fund_source_kraken, "Kraken");

                    // Always sync scope enum from current checkbox state
                    self.broker_scope =
                        match (self.fund_source_alpaca, self.fund_source_kraken) {
                            (true, false) => EventSource::Alpaca,
                            (false, true) => EventSource::Kraken,
                            _ => EventSource::All,
                        };

                    ui.separator();
                    let total = if self.broker_scope == EventSource::All {
                        self.bg.all_fundamentals.len()
                    } else {
                        self.cached_scoped_fundamentals.len()
                    };
                    let lbl = match self.broker_scope {
                        EventSource::All => "ALL",
                        EventSource::Alpaca => "ALPACA",
                        EventSource::Kraken => "KRAKEN",
                        EventSource::Positions => "POSITIONS",
                    };
                    let src_note = format!(
                        "Alpaca:{} Kraken:{}",
                        if self.fund_source_alpaca { "ON" } else { "off" },
                        if self.fund_source_kraken { "ON" } else { "off" },
                    );
                    ui.label(
                        egui::RichText::new(format!(
                            "Scope: {} | {} total fundamentals | {}",
                            lbl, total, src_note
                        ))
                        .strong(),
                    );

                    ui.separator();
                    ui.label("Quick presets:");
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("ALL").clicked() {
                            self.fund_source_alpaca = true;
                            self.fund_source_kraken = true;
                            self.broker_scope = EventSource::All;
                        }
                        if ui.button("Alpaca Only").clicked() {
                            self.fund_source_alpaca = true;
                            self.fund_source_kraken = false;
                            self.broker_scope = EventSource::Alpaca;
                        }
                        if ui.button("Kraken Only").clicked() {
                            self.fund_source_alpaca = false;
                            self.fund_source_kraken = true;
                            self.broker_scope = EventSource::Kraken;
                        }
                        if ui.button("Positions").clicked() {
                            self.fund_source_alpaca = true;
                            self.fund_source_kraken = true;
                            self.broker_scope = EventSource::Positions;
                        }
                    });
                });
        }
    }
}
