use super::*;

#[derive(Clone)]
struct RiskAccountRow {
    broker: &'static str,
    account_id: String,
    label: String,
    is_primary: bool,
    equity: f64,
    previous_equity: f64,
    open_pl: f64,
    positions: usize,
}

#[allow(deprecated)]
impl TyphooNApp {
    pub(super) fn render_right_panel_risk_section(&mut self, ui: &mut egui::Ui) {
        // ── Risk & Account Section ───────────────────────────
        let risk_section =
            egui::CollapsingHeader::new(egui::RichText::new("☰ Risk & Account").strong().small())
                .default_open(self.right_risk_open)
                .show(ui, |ui| {
                    let alpaca_accounts = self.risk_alpaca_accounts();
                    let kraken_accounts = self.risk_kraken_accounts();
                    ui.add_space(4.0);
                    let source_count = [self.alpaca_enabled, self.kraken_enabled]
                        .into_iter()
                        .filter(|visible| *visible)
                        .count();
                    if source_count > 1 || alpaca_accounts.len() > 1 || kraken_accounts.len() > 1 {
                        self.render_risk_account_toggles(ui, &alpaca_accounts, &kraken_accounts);
                        ui.add_space(4.0);
                    }

                    let mut rendered = false;
                    for account in &alpaca_accounts {
                        if self
                            .hidden_alpaca_risk_account_ids
                            .contains(&account.account_id)
                        {
                            continue;
                        }
                        self.render_risk_account(ui, account);
                        rendered = true;
                    }
                    for account in &kraken_accounts {
                        if self
                            .hidden_kraken_risk_account_ids
                            .contains(&account.account_id)
                        {
                            continue;
                        }
                        self.render_risk_account(ui, account);
                        rendered = true;
                    }
                    if !rendered {
                        ui.label(
                            egui::RichText::new("No account data.")
                                .small()
                                .color(AXIS_TEXT),
                        );
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

    fn risk_alpaca_accounts(&self) -> Vec<RiskAccountRow> {
        if !self.alpaca_enabled {
            return Vec::new();
        }
        if !self.alpaca_account_positions.is_empty() {
            return self
                .alpaca_account_positions
                .iter()
                .map(|account| RiskAccountRow {
                    broker: "Alpaca",
                    account_id: account.account_id.clone(),
                    label: account.label.clone(),
                    is_primary: account.is_primary,
                    equity: account.account_equity,
                    previous_equity: account.account_last_equity,
                    open_pl: account.positions.iter().map(|pos| pos.unrealized_pl).sum(),
                    positions: account.positions.len(),
                })
                .collect();
        }
        if let Some(acct) = self.live_account.as_ref() {
            let primary = self
                .alpaca_account_roster
                .iter()
                .find(|account| account.is_primary);
            return vec![RiskAccountRow {
                broker: "Alpaca",
                account_id: primary
                    .map(|account| account.id.clone())
                    .unwrap_or_else(|| self.alpaca_primary_account_id.clone()),
                label: primary
                    .map(|account| account.label.clone())
                    .unwrap_or_else(|| if self.broker_paper { "Alpaca 1 (Paper)".to_string() } else { "Alpaca 1 (Live)".to_string() }),
                is_primary: true,
                equity: acct.equity,
                previous_equity: acct.last_equity,
                open_pl: self
                    .live_positions
                    .iter()
                    .map(|pos| pos.unrealized_pl)
                    .sum(),
                positions: self.live_positions.len(),
            }];
        }
        self.alpaca_account_roster
            .iter()
            .filter(|account| account.connected)
            .map(|account| RiskAccountRow {
                broker: "Alpaca",
                account_id: account.id.clone(),
                label: account.label.clone(),
                is_primary: account.is_primary,
                equity: account.equity,
                previous_equity: 0.0,
                open_pl: 0.0,
                positions: 0,
            })
            .collect()
    }

    fn risk_kraken_accounts(&self) -> Vec<RiskAccountRow> {
        if !self.kraken_enabled {
            return Vec::new();
        }
        if !self.kraken_account_positions.is_empty() {
            return self
                .kraken_account_positions
                .iter()
                .map(|account| RiskAccountRow {
                    broker: "Kraken",
                    account_id: account.account_id.clone(),
                    label: account.label.clone(),
                    is_primary: account.is_primary,
                    equity: if account.is_primary {
                        self.kraken_usd_equivalent_balance()
                    } else {
                        self.kraken_roster_by_id
                            .get(account.account_id.as_str())
                            .map(|roster| roster.equity)
                            .unwrap_or(0.0)
                    },
                    previous_equity: 0.0,
                    open_pl: account.positions.iter().map(|pos| pos.unrealized_pl).sum(),
                    positions: account.positions.len(),
                })
                .collect();
        }
        if self.kraken_trades.is_empty()
            && self.kraken_account_roster.is_empty()
            && self.kraken_balances.is_empty()
        {
            return Vec::new();
        }
        let primary = self.kraken_primary_roster_entry.as_ref();
        vec![RiskAccountRow {
            broker: "Kraken",
            account_id: primary
                .map(|account| account.id.clone())
                .unwrap_or_else(|| self.kraken_primary_account_id.clone()),
            label: primary
                .map(|account| account.label.clone())
                .unwrap_or_else(|| "Kraken (Live)".to_string()),
            is_primary: true,
            equity: self.kraken_usd_equivalent_balance(),
            previous_equity: 0.0,
            open_pl: self.kr_positions.iter().map(|pos| pos.unrealized_pl).sum(),
            positions: self.kr_positions.len(),
        }]
    }

    fn render_risk_account_toggles(
        &mut self,
        ui: &mut egui::Ui,
        alpaca_accounts: &[RiskAccountRow],
        kraken_accounts: &[RiskAccountRow],
    ) {
        ui.horizontal_wrapped(|ui| {
            if self.alpaca_enabled && alpaca_accounts.len() <= 1 {
                let mut shown = self
                    .alpaca_primary_risk_account_id()
                    .map(|id| !self.hidden_alpaca_risk_account_ids.contains(&id))
                    .unwrap_or(true);
                let alpaca_slabel = if self.broker_paper { "Alpaca (Paper)" } else { "Alpaca (Live)" };
                if ui.checkbox(&mut shown, egui::RichText::new(alpaca_slabel).small()).changed() {
                    if let Some(id) = self.alpaca_primary_risk_account_id() {
                        if shown {
                            self.hidden_alpaca_risk_account_ids.remove(&id);
                        } else {
                            self.hidden_alpaca_risk_account_ids.insert(id);
                        }
                    }
                }
            }
            for account in alpaca_accounts {
                let mut shown = !self.hidden_alpaca_risk_account_ids.contains(&account.account_id);
                if ui
                    .checkbox(
                        &mut shown,
                        egui::RichText::new(format!(
                            "{}{} ({})",
                            account.label,
                            if account.is_primary { " ★" } else { "" },
                            account.positions
                        ))
                        .small(),
                    )
                    .on_hover_text(format!("Alpaca account id: {}", account.account_id))
                    .changed()
                {
                    if shown {
                        self.hidden_alpaca_risk_account_ids.remove(&account.account_id);
                    } else {
                        self.hidden_alpaca_risk_account_ids.insert(account.account_id.clone());
                    }
                }
            }

            if self.kraken_enabled && kraken_accounts.len() <= 1 {
                let mut shown = self
                    .kraken_primary_risk_account_id()
                    .map(|id| !self.hidden_kraken_risk_account_ids.contains(&id))
                    .unwrap_or(true);
                if ui.checkbox(&mut shown, egui::RichText::new("Kraken (Live)").small()).changed() {
                    if let Some(id) = self.kraken_primary_risk_account_id() {
                        if shown {
                            self.hidden_kraken_risk_account_ids.remove(&id);
                        } else {
                            self.hidden_kraken_risk_account_ids.insert(id);
                        }
                    }
                }
            }
            let single_kraken_account = kraken_accounts.len() <= 1;
            if !single_kraken_account {
                for account in kraken_accounts {
                    let mut shown = !self.hidden_kraken_risk_account_ids.contains(&account.account_id);
                    if ui
                        .checkbox(
                            &mut shown,
                            egui::RichText::new(format!(
                                "{}{} ({})",
                                super::app_runtime_right_panel_positions::single_kraken_account_label(
                                    &account.label,
                                    single_kraken_account,
                                ),
                                super::app_runtime_right_panel_positions::primary_marker(
                                    account.is_primary,
                                    single_kraken_account,
                                ),
                                account.positions
                            ))
                            .small(),
                        )
                        .on_hover_text(format!("Kraken account id: {}", account.account_id))
                        .changed()
                    {
                        if shown {
                            self.hidden_kraken_risk_account_ids.remove(&account.account_id);
                        } else {
                            self.hidden_kraken_risk_account_ids.insert(account.account_id.clone());
                        }
                    }
                }
            }
        });
    }

    fn alpaca_primary_risk_account_id(&self) -> Option<String> {
        self.risk_alpaca_accounts()
            .into_iter()
            .find(|account| account.is_primary)
            .map(|account| account.account_id)
    }

    fn kraken_primary_risk_account_id(&self) -> Option<String> {
        self.risk_kraken_accounts()
            .into_iter()
            .find(|account| account.is_primary)
            .map(|account| account.account_id)
    }

    fn render_risk_account(&self, ui: &mut egui::Ui, account: &RiskAccountRow) {
        ui.label(
            egui::RichText::new(format!(
                "{}{}",
                account.label,
                if account.is_primary { " ★" } else { "" }
            ))
            .color(if account.is_primary {
                ACCENT
            } else {
                AXIS_TEXT
            })
            .small()
            .strong(),
        )
        .on_hover_text(format!(
            "{} account id: {}",
            account.broker, account.account_id
        ));

        if account.broker == "Alpaca" && account.is_primary {
            if let Some(acct) = self.live_account.as_ref() {
                self.render_primary_alpaca_risk_grid(ui, account, acct);
                ui.add_space(5.0);
                return;
            }
        }
        if account.broker == "Kraken" {
            self.render_kraken_risk_grid(ui, account);
        } else {
            self.render_compact_risk_grid(ui, account);
        }
        ui.add_space(5.0);
    }

    fn render_primary_alpaca_risk_grid(
        &self,
        ui: &mut egui::Ui,
        account: &RiskAccountRow,
        acct: &AccountInfo,
    ) {
        let day_change = (acct.last_equity > 0.0).then_some(acct.equity - acct.last_equity);
        egui::Grid::new(format!("live_risk_grid_{}", account.account_id))
            .striped(true)
            .num_columns(2)
            .show(ui, |ui| {
                self.risk_grid_row(ui, "Equity", format!("${:.2}", acct.equity), None);
                self.risk_grid_row(
                    ui,
                    "Portfolio Value",
                    format!("${:.2}", acct.portfolio_value),
                    None,
                );
                self.risk_grid_row(ui, "Cash", format!("${:.2}", acct.cash), None);
                if let Some(day_change) = day_change {
                    self.risk_grid_row(
                        ui,
                        "Day Δ vs Prev Equity",
                        format!("${day_change:+.2}"),
                        Some(if day_change >= 0.0 { UP } else { DOWN }),
                    );
                }
                if acct.last_equity > 0.0 {
                    self.risk_grid_row(
                        ui,
                        "Prev Equity",
                        format!("${:.2}", acct.last_equity),
                        None,
                    );
                }
                self.risk_grid_row(
                    ui,
                    "Open P/L",
                    format!("${:+.2}", account.open_pl),
                    Some(if account.open_pl >= 0.0 { UP } else { DOWN }),
                );
                self.risk_grid_row(
                    ui,
                    "Buying Power",
                    format!("${:.2}", acct.buying_power),
                    None,
                );
                self.risk_grid_row(
                    ui,
                    "Initial Margin",
                    format!("${:.2}", acct.initial_margin),
                    None,
                );
                self.risk_grid_row(
                    ui,
                    "Maintenance Margin",
                    format!("${:.2}", acct.maintenance_margin),
                    None,
                );
            });
    }

    fn render_kraken_risk_grid(&self, ui: &mut egui::Ui, account: &RiskAccountRow) {
        let cash = if account.is_primary {
            self.kraken_quote_balance()
        } else {
            0.0
        };
        let holdings = (account.equity - cash).max(0.0);
        egui::Grid::new(format!("live_risk_grid_{}", account.account_id))
            .striped(true)
            .num_columns(2)
            .show(ui, |ui| {
                self.risk_grid_row(ui, "Equity", format!("${:.2}", account.equity), None);
                self.risk_grid_row(ui, "Holdings", format!("${:.2}", holdings), None);
                self.risk_grid_row(ui, "Cash (USD/stable)", format!("${:.2}", cash), None);
                self.risk_grid_row(
                    ui,
                    "Open P/L",
                    format!("${:+.2}", account.open_pl),
                    Some(if account.open_pl >= 0.0 { UP } else { DOWN }),
                );
            });
    }

    fn render_compact_risk_grid(&self, ui: &mut egui::Ui, account: &RiskAccountRow) {
        egui::Grid::new(format!("live_risk_grid_{}", account.account_id))
            .striped(true)
            .num_columns(2)
            .show(ui, |ui| {
                self.risk_grid_row(ui, "Equity", format!("${:.2}", account.equity), None);
                if account.previous_equity > 0.0 {
                    self.risk_grid_row(
                        ui,
                        "Prev Equity",
                        format!("${:.2}", account.previous_equity),
                        None,
                    );
                }
                self.risk_grid_row(
                    ui,
                    "Open P/L",
                    format!("${:+.2}", account.open_pl),
                    Some(if account.open_pl >= 0.0 { UP } else { DOWN }),
                );
                self.risk_grid_row(ui, "Positions", account.positions.to_string(), None);
            });
    }

    fn risk_grid_row(
        &self,
        ui: &mut egui::Ui,
        label: &str,
        value: String,
        color: Option<egui::Color32>,
    ) {
        ui.label(egui::RichText::new(label).color(AXIS_TEXT).small());
        let text = egui::RichText::new(value).small();
        ui.label(if let Some(color) = color {
            text.color(color)
        } else {
            text
        });
        ui.end_row();
    }
}
