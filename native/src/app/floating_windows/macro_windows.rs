use super::*;

impl TyphooNApp {
    pub(super) fn render_macro_windows(&mut self, ctx: &egui::Context) {
        if self.show_treasury_curve {
            let mut open = self.show_treasury_curve;
            egui::Window::new("GY — US Treasury Yield Curve")
                .open(&mut open)
                .resizable(true)
                .default_size([460.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if ui.add(egui::Button::new("Fetch").fill(BTN_MG)).clicked() {
                            self.treasury_yields_loading = true;
                            let _ = self.broker_tx.send(BrokerCmd::FetchTreasuryYields);
                        }
                        if self.treasury_yields_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                        if let Some(ts) = self.treasury_yields_last_fetch {
                            let secs = ts.elapsed().as_secs();
                            ui.label(
                                egui::RichText::new(format!("Updated {}s ago", secs))
                                    .color(AXIS_TEXT)
                                    .small(),
                            );
                        }
                    });
                    ui.separator();
                    if self.treasury_yields.is_empty() {
                        ui.label(
                            egui::RichText::new(
                                "No data — click Fetch to pull ^IRX/^FVX/^TNX/^TYX from Yahoo.",
                            )
                            .color(AXIS_TEXT)
                            .small(),
                        );
                    } else {
                        // Slope indicators: 10s-3mo, 30s-10s
                        let by_tenor: std::collections::HashMap<&str, f64> = self
                            .treasury_yields
                            .iter()
                            .map(|t| (t.tenor.as_str(), t.yield_pct))
                            .collect();
                        let slope_10_3mo = match (by_tenor.get("10Y"), by_tenor.get("13W")) {
                            (Some(y10), Some(y3m)) => Some(y10 - y3m),
                            _ => None,
                        };
                        let slope_30_10 = match (by_tenor.get("30Y"), by_tenor.get("10Y")) {
                            (Some(y30), Some(y10)) => Some(y30 - y10),
                            _ => None,
                        };
                        ui.horizontal(|ui| {
                            if let Some(s) = slope_10_3mo {
                                let col = if s < 0.0 { DOWN } else { UP };
                                let label = if s < 0.0 { "INVERTED" } else { "NORMAL" };
                                ui.label(
                                    egui::RichText::new(format!("10Y-3M: {:+.2}%  ({})", s, label))
                                        .color(col)
                                        .strong(),
                                );
                            }
                            if let Some(s) = slope_30_10 {
                                ui.label(
                                    egui::RichText::new(format!("30Y-10Y: {:+.2}%", s))
                                        .color(AXIS_TEXT),
                                );
                            }
                        });
                        ui.separator();
                        egui::Grid::new("gy_grid")
                            .striped(true)
                            .num_columns(5)
                            .spacing([18.0, 4.0])
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Tenor").strong());
                                ui.label(egui::RichText::new("Yahoo").strong());
                                ui.label(egui::RichText::new("Yield").strong());
                                ui.label(egui::RichText::new("Δ").strong());
                                ui.label(egui::RichText::new("Δ%").strong());
                                ui.end_row();
                                for t in &self.treasury_yields {
                                    ui.label(egui::RichText::new(&t.tenor).monospace().strong());
                                    ui.label(
                                        egui::RichText::new(&t.ticker)
                                            .color(AXIS_TEXT)
                                            .monospace()
                                            .small(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("{:.3}%", t.yield_pct))
                                            .color(UP)
                                            .monospace(),
                                    );
                                    let dc = if t.change < 0.0 {
                                        DOWN
                                    } else if t.change > 0.0 {
                                        UP
                                    } else {
                                        AXIS_TEXT
                                    };
                                    ui.label(
                                        egui::RichText::new(format!("{:+.3}", t.change))
                                            .color(dc)
                                            .monospace()
                                            .small(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("{:+.2}%", t.change_pct))
                                            .color(dc)
                                            .monospace()
                                            .small(),
                                    );
                                    ui.end_row();
                                }
                            });
                    }
                });
            self.show_treasury_curve = open;
        }

        // Economic Calendar
        if self.show_calendar {
            egui::Window::new("Economic Calendar")
                .open(&mut self.show_calendar)
                .resizable(true)
                .default_size([500.0, 400.0])
                .show(ctx, |ui| {
                    let sym = self
                        .charts
                        .get(self.active_tab)
                        .map(|c| {
                            c.symbol
                                .split(':')
                                .rev()
                                .nth(1)
                                .or_else(|| c.symbol.split(':').last())
                                .unwrap_or("")
                                .to_string()
                        })
                        .unwrap_or_default();
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Economic Calendar").strong());
                        if ui.button("Fetch Earnings").clicked() && !sym.is_empty() {
                            // Use Finnhub or AlphaVantage for earnings data
                            self.log.push_back(LogEntry::info(format!(
                                "Earnings calendar for {}: set AV_KEY or FINNHUB_KEY in Settings",
                                sym
                            )));
                        }
                    });
                    ui.separator();
                    // Key economic events (static reference — updated via data feeds when connected)
                    ui.label(egui::RichText::new("Key Events").strong());
                    let events = [
                        ("FOMC Rate Decision", "8 meetings/year", "Fed funds rate"),
                        ("Non-Farm Payrolls", "Monthly (1st Friday)", "US employment"),
                        ("CPI / Core CPI", "Monthly", "Inflation gauge"),
                        ("GDP (Advance/Final)", "Quarterly", "Economic growth"),
                        (
                            "ISM Manufacturing",
                            "Monthly (1st business day)",
                            "Factory activity",
                        ),
                        ("Retail Sales", "Monthly", "Consumer spending"),
                        ("Jobless Claims", "Weekly (Thursday)", "Employment health"),
                    ];
                    egui::Grid::new("econ_cal")
                        .striped(true)
                        .num_columns(3)
                        .show(ui, |ui| {
                            ui.strong("Event");
                            ui.strong("Frequency");
                            ui.strong("Measures");
                            ui.end_row();
                            for (event, freq, desc) in &events {
                                ui.label(*event);
                                ui.label(egui::RichText::new(*freq).color(AXIS_TEXT).small());
                                ui.label(egui::RichText::new(*desc).color(AXIS_TEXT).small());
                                ui.end_row();
                            }
                        });
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(
                            "Live data: connect Finnhub or AlphaVantage API key in Settings.",
                        )
                        .color(AXIS_TEXT)
                        .small(),
                    );
                });
        }
    }
}
