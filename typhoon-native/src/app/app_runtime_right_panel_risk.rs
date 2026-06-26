use super::*;

#[allow(deprecated)]
impl TyphooNApp {
    pub(super) fn render_right_panel_risk_section(&mut self, ui: &mut egui::Ui) {
        // ── Risk & Account Section ───────────────────────────
        let risk_section =
            egui::CollapsingHeader::new(egui::RichText::new("☰ Risk & Account").strong().small())
                .default_open(self.right_risk_open)
                .show(ui, |ui| {
                    ui.add_space(4.0);
                    // Live broker account data for selected target(s)
                    let account_snaps = self.selected_trade_account_snapshots();
                    for (idx, snap) in account_snaps.iter().enumerate() {
                        ui.label(
                            egui::RichText::new(snap.broker)
                                .color(AXIS_TEXT)
                                .small()
                                .strong(),
                        );
                        // Kraken has no margin (spot-only), so Buying Power and Margin
                        // Used are meaningless duplicates of Equity / 0. Show a slim
                        // Equity + Holdings + Cash layout instead so the user can see
                        // total NAV, what's in tokens, and what's actually deployable.
                        if snap.broker == "Kraken" {
                            let cash = self.kraken_quote_balance();
                            let equity = snap.equity;
                            let holdings = (equity - cash).max(0.0);
                            egui::Grid::new(format!("live_risk_grid_{idx}"))
                                .striped(true)
                                .num_columns(2)
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new("Equity").color(AXIS_TEXT).small(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("${:.2}", equity)).small(),
                                    );
                                    ui.end_row();
                                    ui.label(
                                        egui::RichText::new("Holdings").color(AXIS_TEXT).small(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("${:.2}", holdings)).small(),
                                    );
                                    ui.end_row();
                                    ui.label(
                                        egui::RichText::new("Cash (USD/stable)")
                                            .color(AXIS_TEXT)
                                            .small(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("${:.2}", cash))
                                            .small()
                                            .strong(),
                                    );
                                    ui.end_row();
                                });
                        } else if snap.broker == "Alpaca" {
                            let acct = self.live_account.as_ref();
                            let cash = acct.map(|acct| acct.cash).unwrap_or(0.0);
                            let initial_margin = acct
                                .map(|acct| acct.initial_margin)
                                .unwrap_or(snap.margin_used);
                            let maintenance_margin =
                                acct.map(|acct| acct.maintenance_margin).unwrap_or(0.0);
                            let portfolio_value =
                                acct.map(|acct| acct.portfolio_value).unwrap_or(snap.equity);
                            let previous_equity = acct.map(|acct| acct.last_equity).unwrap_or(0.0);
                            let day_change =
                                (previous_equity > 0.0).then_some(snap.equity - previous_equity);
                            egui::Grid::new(format!("live_risk_grid_{idx}"))
                                .striped(true)
                                .num_columns(2)
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new("Equity").color(AXIS_TEXT).small(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("${:.2}", snap.equity)).small(),
                                    );
                                    ui.end_row();
                                    ui.label(
                                        egui::RichText::new("Portfolio Value")
                                            .color(AXIS_TEXT)
                                            .small(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("${:.2}", portfolio_value))
                                            .small(),
                                    );
                                    ui.end_row();
                                    ui.label(egui::RichText::new("Cash").color(AXIS_TEXT).small());
                                    ui.label(egui::RichText::new(format!("${:.2}", cash)).small());
                                    ui.end_row();
                                    if let Some(day_change) = day_change {
                                        ui.label(
                                            egui::RichText::new("Day Δ vs Prev Equity")
                                                .color(AXIS_TEXT)
                                                .small(),
                                        );
                                        let day_color = if day_change >= 0.0 { UP } else { DOWN };
                                        ui.label(
                                            egui::RichText::new(format!("${:+.2}", day_change))
                                                .color(day_color)
                                                .small(),
                                        );
                                        ui.end_row();
                                    }
                                    if previous_equity > 0.0 {
                                        ui.label(
                                            egui::RichText::new("Prev Equity")
                                                .color(AXIS_TEXT)
                                                .small(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!("${:.2}", previous_equity))
                                                .small(),
                                        );
                                        ui.end_row();
                                    }
                                    ui.label(
                                        egui::RichText::new("Buying Power")
                                            .color(AXIS_TEXT)
                                            .small(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("${:.2}", snap.buying_power))
                                            .small(),
                                    );
                                    ui.end_row();
                                    ui.label(
                                        egui::RichText::new("Initial Margin")
                                            .color(AXIS_TEXT)
                                            .small(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("${:.2}", initial_margin))
                                            .small(),
                                    );
                                    ui.end_row();
                                    ui.label(
                                        egui::RichText::new("Maintenance Margin")
                                            .color(AXIS_TEXT)
                                            .small(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("${:.2}", maintenance_margin))
                                            .small(),
                                    );
                                    ui.end_row();
                                });
                        } else {
                            egui::Grid::new(format!("live_risk_grid_{idx}"))
                                .striped(true)
                                .num_columns(2)
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new("Equity").color(AXIS_TEXT).small(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("${:.2}", snap.equity)).small(),
                                    );
                                    ui.end_row();
                                    ui.label(
                                        egui::RichText::new("Balance").color(AXIS_TEXT).small(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("${:.2}", snap.balance))
                                            .small(),
                                    );
                                    ui.end_row();
                                    ui.label(
                                        egui::RichText::new("Buying Power")
                                            .color(AXIS_TEXT)
                                            .small(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("${:.2}", snap.buying_power))
                                            .small(),
                                    );
                                    ui.end_row();
                                    ui.label(
                                        egui::RichText::new("Margin Used").color(AXIS_TEXT).small(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("${:.2}", snap.margin_used))
                                            .small(),
                                    );
                                    ui.end_row();
                                });
                        }
                        ui.add_space(5.0);
                    }
                    ui.add_space(6.0);
                    ui.separator();
                });
        self.right_risk_open = risk_section.fully_open();
        self.handle_right_panel_section_drag(
            ui,
            RightPanelSectionId::Risk,
            &risk_section.header_response,
        );
    }
}
