use super::*;

#[derive(Clone)]
struct FillAccountRows {
    account_id: String,
    label: String,
    is_primary: bool,
    rows: Vec<(String, String, f64, f64, String)>,
}

#[allow(deprecated)]
impl TyphooNApp {
    pub(super) fn render_right_panel_recent_fills_section(&mut self, ui: &mut egui::Ui) {
        // ── Recent Fills Section ──────────────────────────────
        let alpaca_accounts = self.recent_fill_alpaca_accounts();
        let kraken_accounts = self.recent_fill_kraken_accounts();
        let fills_count = alpaca_accounts
            .iter()
            .filter(|account| {
                !self
                    .hidden_alpaca_recent_fill_account_ids
                    .contains(&account.account_id)
            })
            .map(|account| account.rows.len())
            .sum::<usize>()
            + kraken_accounts
                .iter()
                .filter(|account| {
                    !self
                        .hidden_kraken_recent_fill_account_ids
                        .contains(&account.account_id)
                })
                .map(|account| account.rows.len())
                .sum::<usize>();

        let recent_fills_section = egui::CollapsingHeader::new(
            egui::RichText::new(format!("☰ Recent Fills ({fills_count})"))
                .strong()
                .small(),
        )
        .id_salt("recent_fills_top")
        .default_open(self.right_recent_fills_open)
        .show(ui, |ui| {
            let source_count = [self.alpaca_enabled, self.kraken_enabled]
                .into_iter()
                .filter(|visible| *visible)
                .count();
            if source_count > 1 || alpaca_accounts.len() > 1 || kraken_accounts.len() > 1 {
                self.render_recent_fill_account_toggles(ui, &alpaca_accounts, &kraken_accounts);
                ui.add_space(4.0);
            }

            let mut rendered = false;
            for account in &alpaca_accounts {
                if self
                    .hidden_alpaca_recent_fill_account_ids
                    .contains(&account.account_id)
                {
                    continue;
                }
                rendered |= self.render_recent_fill_account_rows(ui, "Alpaca", account);
            }
            for account in &kraken_accounts {
                if self
                    .hidden_kraken_recent_fill_account_ids
                    .contains(&account.account_id)
                {
                    continue;
                }
                rendered |= self.render_recent_fill_account_rows(ui, "Kraken", account);
            }

            if !rendered {
                ui.label(
                    egui::RichText::new("No recent fills.")
                        .color(AXIS_TEXT)
                        .small(),
                );
            }
        });
        self.right_recent_fills_open = recent_fills_section.fully_open();
        self.handle_right_panel_section_drag(
            ui,
            RightPanelSectionId::RecentFills,
            &recent_fills_section.header_response,
        );
    }

    fn recent_fill_alpaca_accounts(&self) -> Vec<FillAccountRows> {
        if !self.alpaca_enabled {
            return Vec::new();
        }
        if !self.alpaca_account_fills.is_empty() {
            return self
                .alpaca_account_fills
                .iter()
                .map(|account| FillAccountRows {
                    account_id: account.account_id.clone(),
                    label: account.label.clone(),
                    is_primary: account.is_primary,
                    rows: account.fills.clone(),
                })
                .collect();
        }
        if self.recent_fills.is_empty() && self.alpaca_account_roster.is_empty() {
            return Vec::new();
        }
        let primary = self
            .alpaca_roster_by_id
            .get(&self.alpaca_primary_account_id)
            .cloned()
            .or_else(|| self.alpaca_primary_roster_entry.clone());
        vec![FillAccountRows {
            account_id: primary
                .as_ref()
                .map(|account| account.id.clone())
                .unwrap_or_else(|| self.alpaca_primary_account_id.clone()),
            label: primary
                .as_ref()
                .map(|account| account.label.clone())
                .unwrap_or_else(|| "Alpaca 1".to_string()),
            is_primary: true,
            rows: self.recent_fills.clone(),
        }]
    }

    fn recent_fill_kraken_accounts(&self) -> Vec<FillAccountRows> {
        if !self.kraken_enabled {
            return Vec::new();
        }
        if !self.kraken_account_trades.is_empty() {
            return self
                .kraken_account_trades
                .iter()
                .map(|account| FillAccountRows {
                    account_id: account.account_id.clone(),
                    label: account.label.clone(),
                    is_primary: account.is_primary,
                    rows: account
                        .trades
                        .iter()
                        .take(100)
                        .map(|t| Self::kraken_fill_row(t))
                        .collect(),
                })
                .collect();
        }
        if self.kraken_trades.is_empty() && self.kraken_account_roster.is_empty() {
            return Vec::new();
        }
        let primary = self
            .kraken_roster_by_id
            .get(&self.kraken_primary_account_id)
            .cloned()
            .or_else(|| self.kraken_primary_roster_entry.clone());
        vec![FillAccountRows {
            account_id: primary
                .as_ref()
                .map(|account| account.id.clone())
                .unwrap_or_else(|| self.kraken_primary_account_id.clone()),
            label: primary
                .as_ref()
                .map(|account| account.label.clone())
                .unwrap_or_else(|| "Kraken (Live)".to_string()),
            is_primary: true,
            rows: self
                .kraken_trades
                .iter()
                .take(100)
                .map(Self::kraken_fill_row)
                .collect(),
        }]
    }

    fn kraken_fill_row(
        t: &typhoon_engine::broker::kraken::KrakenTrade,
    ) -> (String, String, f64, f64, String) {
        let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(t.time as i64, 0)
            .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| format!("{:.0}", t.time));
        (
            Self::kraken_base_asset_for_pair(&t.pair),
            t.side.clone(),
            t.vol,
            t.price,
            dt,
        )
    }

    fn render_recent_fill_account_toggles(
        &mut self,
        ui: &mut egui::Ui,
        alpaca_accounts: &[FillAccountRows],
        kraken_accounts: &[FillAccountRows],
    ) {
        ui.horizontal_wrapped(|ui| {
            if self.alpaca_enabled && alpaca_accounts.len() <= 1 {
                ui.checkbox(
                    &mut self.show_alpaca_positions,
                    egui::RichText::new("Alpaca").small(),
                );
            }
            for account in alpaca_accounts {
                let mut shown = self.show_alpaca_positions
                    && !self
                        .hidden_alpaca_recent_fill_account_ids
                        .contains(&account.account_id);
                if ui
                    .checkbox(
                        &mut shown,
                        egui::RichText::new(format!(
                            "{}{} ({})",
                            account.label,
                            if account.is_primary { " ★" } else { "" },
                            account.rows.len()
                        ))
                        .small(),
                    )
                    .on_hover_text(format!("Alpaca account id: {}", account.account_id))
                    .changed()
                {
                    if shown {
                        self.hidden_alpaca_recent_fill_account_ids
                            .remove(&account.account_id);
                        self.show_alpaca_positions = true;
                    } else {
                        self.hidden_alpaca_recent_fill_account_ids
                            .insert(account.account_id.clone());
                    }
                }
            }

            if self.kraken_enabled && kraken_accounts.len() <= 1 {
                ui.checkbox(
                    &mut self.show_kr_positions,
                    egui::RichText::new("Kraken").small(),
                );
            }
            let single_kraken_account = kraken_accounts.len() <= 1;
            if !single_kraken_account {
                for account in kraken_accounts {
                    let mut shown = self.show_kr_positions
                        && !self
                            .hidden_kraken_recent_fill_account_ids
                            .contains(&account.account_id);
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
                                account.rows.len()
                            ))
                            .small(),
                        )
                        .on_hover_text(format!("Kraken account id: {}", account.account_id))
                        .changed()
                    {
                        if shown {
                            self.hidden_kraken_recent_fill_account_ids
                                .remove(&account.account_id);
                            self.show_kr_positions = true;
                        } else {
                            self.hidden_kraken_recent_fill_account_ids
                                .insert(account.account_id.clone());
                        }
                    }
                }
            }
        });
    }

    fn render_recent_fill_account_rows(
        &self,
        ui: &mut egui::Ui,
        broker: &str,
        account: &FillAccountRows,
    ) -> bool {
        if account.rows.is_empty() {
            return false;
        }
        ui.label(
            egui::RichText::new(format!(
                "{}{} ({})",
                account.label,
                if account.is_primary { " ★" } else { "" },
                account.rows.len()
            ))
            .small()
            .strong()
            .color(if account.is_primary {
                ACCENT
            } else {
                AXIS_TEXT
            }),
        )
        .on_hover_text(format!("{broker} account id: {}", account.account_id));
        for (sym, side, qty, price, time) in &account.rows {
            let c = if side == "buy" { UP } else { DOWN };
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new(sym).small().strong());
                ui.label(egui::RichText::new(side).color(c).small());
                ui.label(
                    egui::RichText::new(format!("{qty:.2} @ {}", format_price(*price))).small(),
                );
                ui.label(egui::RichText::new(time).color(AXIS_TEXT).small());
            });
        }
        true
    }
}
