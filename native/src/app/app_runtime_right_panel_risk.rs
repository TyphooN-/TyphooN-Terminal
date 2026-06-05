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
                    // DARWIN portfolio data — from bg cache
                    if let Some(ref portfolio) = self.bg.portfolio {
                        if !portfolio.accounts.is_empty() {
                            egui::Grid::new("risk_grid")
                                .striped(true)
                                .num_columns(2)
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new("Accounts").color(AXIS_TEXT).small(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{}",
                                            portfolio.accounts.len()
                                        ))
                                        .small(),
                                    );
                                    ui.end_row();
                                    ui.label(
                                        egui::RichText::new("Equity").color(AXIS_TEXT).small(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "${:.0}",
                                            portfolio.total_final_balance
                                        ))
                                        .small(),
                                    );
                                    ui.end_row();
                                    let pnl_c = if portfolio.total_net_pnl >= 0.0 {
                                        UP
                                    } else {
                                        DOWN
                                    };
                                    ui.label(
                                        egui::RichText::new("Net P&L").color(AXIS_TEXT).small(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "${:.0}",
                                            portfolio.total_net_pnl
                                        ))
                                        .color(pnl_c)
                                        .small(),
                                    );
                                    ui.end_row();
                                    ui.label(
                                        egui::RichText::new("Max DD").color(AXIS_TEXT).small(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{:.1}%",
                                            portfolio.combined_max_drawdown_pct
                                        ))
                                        .small(),
                                    );
                                    ui.end_row();
                                    ui.label(egui::RichText::new("Deals").color(AXIS_TEXT).small());
                                    ui.label(
                                        egui::RichText::new(format!("{}", portfolio.total_deals))
                                            .small(),
                                    );
                                    ui.end_row();
                                });
                            // VaR — from bg cache
                            if let Some(ref vs) = self.bg.var_stats {
                                ui.add_space(4.0);
                                egui::Grid::new("risk_var")
                                    .striped(true)
                                    .num_columns(2)
                                    .show(ui, |ui| {
                                        ui.label(
                                            egui::RichText::new("VaR 95%").color(AXIS_TEXT).small(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!("${:.0}", vs.var_95))
                                                .small(),
                                        );
                                        ui.end_row();
                                        ui.label(
                                            egui::RichText::new("Sharpe").color(AXIS_TEXT).small(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!("{:.3}", vs.sharpe))
                                                .small(),
                                        );
                                        ui.end_row();
                                    });
                            }
                        }
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
