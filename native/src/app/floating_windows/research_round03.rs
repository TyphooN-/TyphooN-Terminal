use super::*;

impl TyphooNApp {
    pub(super) fn render_research_round03_windows(&mut self, ctx: &egui::Context) {
        let chart_sym_research: String = self
            .charts
            .get(self.active_tab)
            .map(|c| {
                c.symbol
                    .split(':')
                    .rev()
                    .nth(1)
                    .or_else(|| c.symbol.split(':').last())
                    .unwrap_or("AAPL")
                    .to_string()
            })
            .unwrap_or_else(|| "AAPL".to_string());

        // ── Research Godel Parity Round 3 windows ─────────────────────

        // FA — Financial Statements (Income / Balance / Cash Flow × Annual / Quarterly)
        if self.show_financials {
            if self.financials_symbol.is_empty() {
                self.financials_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_financials;
            egui::Window::new("FA — Financial Statements")
                .open(&mut open)
                .resizable(true)
                .default_size([960.0, 580.0])
                .max_size([960.0, 560.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.financials_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.financials_symbol = chart_sym_research.clone(); }
                        let have_cache = self.cache.is_some();
                        if ui.add_enabled(have_cache, egui::Button::new("Load Cached")).clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.financials_symbol.to_uppercase();
                                    if let Ok(Some(bundle)) = typhoon_engine::core::research::get_financials(&conn, &sym_u) {
                                        self.financials = bundle;
                                        self.financials_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        let have_key = !self.fmp_key.is_empty();
                        if ui.add_enabled(have_key, egui::Button::new("Fetch").fill(BTN_MG)).clicked() {
                            let sym = self.financials_symbol.to_uppercase();
                            self.financials_loading = true;
                            self.financials_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::FetchFinancialStatements { symbol: sym, fmp_key: self.fmp_key.clone() });
                        }
                        if self.fmp_key.is_empty() {
                            ui.label(egui::RichText::new("(add FMP key in Settings)").color(AXIS_TEXT).small());
                        }
                        if self.financials_loading {
                            ui.label(egui::RichText::new("Loading… (6 FMP calls)").color(AXIS_TEXT).small());
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.financials_view, FinancialsView::Income,   "Income");
                        ui.selectable_value(&mut self.financials_view, FinancialsView::Balance,  "Balance");
                        ui.selectable_value(&mut self.financials_view, FinancialsView::CashFlow, "Cash Flow");
                        ui.separator();
                        ui.selectable_value(&mut self.financials_period, FinancialsPeriod::Annual,    "Annual");
                        ui.selectable_value(&mut self.financials_period, FinancialsPeriod::Quarterly, "Quarterly");
                    });
                    ui.separator();

                    let fmt_m = |v: f64| -> String {
                        if v == 0.0 { "—".into() }
                        else if v.abs() >= 1e9 { format!("{:.2}B", v / 1e9) }
                        else if v.abs() >= 1e6 { format!("{:.1}M", v / 1e6) }
                        else if v.abs() >= 1e3 { format!("{:.1}K", v / 1e3) }
                        else { format!("{:.2}", v) }
                    };
                    let is_empty = match (self.financials_view, self.financials_period) {
                        (FinancialsView::Income,   FinancialsPeriod::Annual)    => self.financials.income_annual.is_empty(),
                        (FinancialsView::Income,   FinancialsPeriod::Quarterly) => self.financials.income_quarterly.is_empty(),
                        (FinancialsView::Balance,  FinancialsPeriod::Annual)    => self.financials.balance_annual.is_empty(),
                        (FinancialsView::Balance,  FinancialsPeriod::Quarterly) => self.financials.balance_quarterly.is_empty(),
                        (FinancialsView::CashFlow, FinancialsPeriod::Annual)    => self.financials.cashflow_annual.is_empty(),
                        (FinancialsView::CashFlow, FinancialsPeriod::Quarterly) => self.financials.cashflow_quarterly.is_empty(),
                    };
                    if is_empty {
                        ui.label(egui::RichText::new("No data — click Load Cached or Fetch.").color(AXIS_TEXT).small());
                    } else {
                        egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                            match self.financials_view {
                                FinancialsView::Income => {
                                    let rows = if self.financials_period == FinancialsPeriod::Annual {
                                        &self.financials.income_annual
                                    } else {
                                        &self.financials.income_quarterly
                                    };
                                    let take = rows.iter().take(8).collect::<Vec<_>>();
                                    let cols = take.len() + 1;
                                    egui::Grid::new("fa_income_grid").striped(true).num_columns(cols).spacing([14.0, 2.0]).show(ui, |ui| {
                                        ui.label(egui::RichText::new("Line Item").strong());
                                        for r in &take { ui.label(egui::RichText::new(&r.date).strong().monospace()); }
                                        ui.end_row();
                                        let render = |label: &str, vals: Vec<f64>, strong: bool, ui: &mut egui::Ui| {
                                            let rt = egui::RichText::new(label);
                                            ui.label(if strong { rt.strong() } else { rt });
                                            for v in vals {
                                                let c = if v < 0.0 { DOWN } else { AXIS_TEXT };
                                                ui.label(egui::RichText::new(fmt_m(v)).color(c).monospace().small());
                                            }
                                            ui.end_row();
                                        };
                                        render("Revenue",            take.iter().map(|r| r.revenue).collect(), true, ui);
                                        render("Cost of Revenue",    take.iter().map(|r| r.cost_of_revenue).collect(), false, ui);
                                        render("Gross Profit",       take.iter().map(|r| r.gross_profit).collect(), true, ui);
                                        render("R&D",                take.iter().map(|r| r.research_and_development).collect(), false, ui);
                                        render("SG&A",               take.iter().map(|r| r.selling_general_admin).collect(), false, ui);
                                        render("Op Expenses",        take.iter().map(|r| r.operating_expenses).collect(), false, ui);
                                        render("Op Income",          take.iter().map(|r| r.operating_income).collect(), true, ui);
                                        render("Interest Expense",   take.iter().map(|r| r.interest_expense).collect(), false, ui);
                                        render("EBITDA",             take.iter().map(|r| r.ebitda).collect(), true, ui);
                                        render("Pre-Tax Income",     take.iter().map(|r| r.income_before_tax).collect(), false, ui);
                                        render("Tax Expense",        take.iter().map(|r| r.income_tax_expense).collect(), false, ui);
                                        render("Net Income",         take.iter().map(|r| r.net_income).collect(), true, ui);
                                        // EPS rendered as dollars (not scaled with fmt_m).
                                        ui.label(egui::RichText::new("EPS (Basic)").strong());
                                        for r in &take { ui.label(egui::RichText::new(format!("${:.2}", r.eps)).color(UP).monospace().small()); }
                                        ui.end_row();
                                        ui.label(egui::RichText::new("EPS (Diluted)").strong());
                                        for r in &take { ui.label(egui::RichText::new(format!("${:.2}", r.eps_diluted)).color(UP).monospace().small()); }
                                        ui.end_row();
                                        render("Weighted Shares",    take.iter().map(|r| r.weighted_shares_out).collect(), false, ui);
                                    });
                                }
                                FinancialsView::Balance => {
                                    let rows = if self.financials_period == FinancialsPeriod::Annual {
                                        &self.financials.balance_annual
                                    } else {
                                        &self.financials.balance_quarterly
                                    };
                                    let take = rows.iter().take(8).collect::<Vec<_>>();
                                    let cols = take.len() + 1;
                                    egui::Grid::new("fa_balance_grid").striped(true).num_columns(cols).spacing([14.0, 2.0]).show(ui, |ui| {
                                        ui.label(egui::RichText::new("Line Item").strong());
                                        for r in &take { ui.label(egui::RichText::new(&r.date).strong().monospace()); }
                                        ui.end_row();
                                        let render = |label: &str, vals: Vec<f64>, strong: bool, ui: &mut egui::Ui| {
                                            let rt = egui::RichText::new(label);
                                            ui.label(if strong { rt.strong() } else { rt });
                                            for v in vals {
                                                let c = if v < 0.0 { DOWN } else { AXIS_TEXT };
                                                ui.label(egui::RichText::new(fmt_m(v)).color(c).monospace().small());
                                            }
                                            ui.end_row();
                                        };
                                        render("Cash & Equiv",          take.iter().map(|r| r.cash_and_equiv).collect(), false, ui);
                                        render("ST Investments",        take.iter().map(|r| r.short_term_investments).collect(), false, ui);
                                        render("Net Receivables",       take.iter().map(|r| r.net_receivables).collect(), false, ui);
                                        render("Inventory",             take.iter().map(|r| r.inventory).collect(), false, ui);
                                        render("Total Current Assets",  take.iter().map(|r| r.total_current_assets).collect(), true, ui);
                                        render("PP&E",                  take.iter().map(|r| r.property_plant_equipment).collect(), false, ui);
                                        render("Goodwill",              take.iter().map(|r| r.goodwill).collect(), false, ui);
                                        render("Intangibles",           take.iter().map(|r| r.intangible_assets).collect(), false, ui);
                                        render("Total Assets",          take.iter().map(|r| r.total_assets).collect(), true, ui);
                                        render("Accounts Payable",      take.iter().map(|r| r.accounts_payable).collect(), false, ui);
                                        render("ST Debt",               take.iter().map(|r| r.short_term_debt).collect(), false, ui);
                                        render("Current Liab",          take.iter().map(|r| r.total_current_liabilities).collect(), false, ui);
                                        render("LT Debt",               take.iter().map(|r| r.long_term_debt).collect(), false, ui);
                                        render("Total Liab",            take.iter().map(|r| r.total_liabilities).collect(), true, ui);
                                        render("Common Stock",          take.iter().map(|r| r.common_stock).collect(), false, ui);
                                        render("Retained Earnings",     take.iter().map(|r| r.retained_earnings).collect(), false, ui);
                                        render("Total Equity",          take.iter().map(|r| r.total_equity).collect(), true, ui);
                                        render("Total Debt",            take.iter().map(|r| r.total_debt).collect(), true, ui);
                                        render("Net Debt",              take.iter().map(|r| r.net_debt).collect(), true, ui);
                                    });
                                }
                                FinancialsView::CashFlow => {
                                    let rows = if self.financials_period == FinancialsPeriod::Annual {
                                        &self.financials.cashflow_annual
                                    } else {
                                        &self.financials.cashflow_quarterly
                                    };
                                    let take = rows.iter().take(8).collect::<Vec<_>>();
                                    let cols = take.len() + 1;
                                    egui::Grid::new("fa_cashflow_grid").striped(true).num_columns(cols).spacing([14.0, 2.0]).show(ui, |ui| {
                                        ui.label(egui::RichText::new("Line Item").strong());
                                        for r in &take { ui.label(egui::RichText::new(&r.date).strong().monospace()); }
                                        ui.end_row();
                                        let render = |label: &str, vals: Vec<f64>, strong: bool, ui: &mut egui::Ui| {
                                            let rt = egui::RichText::new(label);
                                            ui.label(if strong { rt.strong() } else { rt });
                                            for v in vals {
                                                let c = if v < 0.0 { DOWN } else { AXIS_TEXT };
                                                ui.label(egui::RichText::new(fmt_m(v)).color(c).monospace().small());
                                            }
                                            ui.end_row();
                                        };
                                        render("Net Income",         take.iter().map(|r| r.net_income).collect(), true, ui);
                                        render("D&A",                take.iter().map(|r| r.depreciation_amortization).collect(), false, ui);
                                        render("Stock-Based Comp",   take.iter().map(|r| r.stock_based_comp).collect(), false, ui);
                                        render("Δ Working Capital",  take.iter().map(|r| r.change_working_capital).collect(), false, ui);
                                        render("Cash from Ops",      take.iter().map(|r| r.cash_from_operations).collect(), true, ui);
                                        render("CapEx",              take.iter().map(|r| r.capex).collect(), false, ui);
                                        render("Acquisitions",       take.iter().map(|r| r.acquisitions).collect(), false, ui);
                                        render("Invest Purchases",   take.iter().map(|r| r.investments_purchases).collect(), false, ui);
                                        render("Cash from Investing",take.iter().map(|r| r.cash_from_investing).collect(), true, ui);
                                        render("Debt Repayment",     take.iter().map(|r| r.debt_repayment).collect(), false, ui);
                                        render("Dividends Paid",     take.iter().map(|r| r.dividends_paid).collect(), false, ui);
                                        render("Stock Repurchases",  take.iter().map(|r| r.stock_repurchases).collect(), false, ui);
                                        render("Cash from Financing",take.iter().map(|r| r.cash_from_financing).collect(), true, ui);
                                        render("Net Change Cash",    take.iter().map(|r| r.net_change_cash).collect(), true, ui);
                                        render("Free Cash Flow",     take.iter().map(|r| r.free_cash_flow).collect(), true, ui);
                                    });
                                }
                            }
                        });
                    }
                });
            self.show_financials = open;
        }

        // MGMT — Management / Officers
        if self.show_executives {
            if self.executives_symbol.is_empty() {
                self.executives_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_executives;
            egui::Window::new("MGMT — Management")
                .open(&mut open)
                .resizable(true)
                .default_size([720.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.executives_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.executives_symbol = chart_sym_research.clone();
                        }
                        let have_cache = self.cache.is_some();
                        if ui
                            .add_enabled(have_cache, egui::Button::new("Load Cached"))
                            .clicked()
                        {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.executives_symbol.to_uppercase();
                                    if let Ok(Some(rows)) =
                                        typhoon_engine::core::research::get_executives(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.executives = rows;
                                        self.executives_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        let have_key = !self.finnhub_key.is_empty();
                        if ui
                            .add_enabled(have_key, egui::Button::new("Fetch").fill(BTN_MG))
                            .clicked()
                        {
                            let sym = self.executives_symbol.to_uppercase();
                            self.executives_loading = true;
                            self.executives_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::FetchExecutives {
                                symbol: sym,
                                finnhub_key: self.finnhub_key.clone(),
                            });
                        }
                        if self.finnhub_key.is_empty() {
                            ui.label(
                                egui::RichText::new("(add Finnhub key in Settings)")
                                    .color(AXIS_TEXT)
                                    .small(),
                            );
                        }
                        if self.executives_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    if self.executives.is_empty() {
                        ui.label(
                            egui::RichText::new(
                                "No officers on file — click Load Cached or Fetch.",
                            )
                            .color(AXIS_TEXT)
                            .small(),
                        );
                    } else {
                        // Aggregate total comp
                        let total_comp: f64 = self.executives.iter().map(|e| e.compensation).sum();
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "Total comp: ${:.1}M",
                                    total_comp / 1e6
                                ))
                                .strong()
                                .color(UP),
                            );
                            ui.label(
                                egui::RichText::new(format!(
                                    "({} officers)",
                                    self.executives.len()
                                ))
                                .color(AXIS_TEXT)
                                .small(),
                            );
                        });
                        ui.separator();
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                egui::Grid::new("mgmt_grid")
                                    .striped(true)
                                    .num_columns(6)
                                    .spacing([14.0, 3.0])
                                    .show(ui, |ui| {
                                        ui.label(egui::RichText::new("Name").strong());
                                        ui.label(egui::RichText::new("Position").strong());
                                        ui.label(egui::RichText::new("Age").strong());
                                        ui.label(egui::RichText::new("Sex").strong());
                                        ui.label(egui::RichText::new("Since").strong());
                                        ui.label(egui::RichText::new("Compensation").strong());
                                        ui.end_row();
                                        for e in self.executives.iter() {
                                            ui.label(egui::RichText::new(&e.name).small().strong());
                                            ui.label(
                                                egui::RichText::new(&e.position)
                                                    .color(AXIS_TEXT)
                                                    .small(),
                                            );
                                            if e.age > 0 {
                                                ui.label(
                                                    egui::RichText::new(format!("{}", e.age))
                                                        .monospace()
                                                        .small(),
                                                );
                                            } else {
                                                ui.label(
                                                    egui::RichText::new("—")
                                                        .color(AXIS_TEXT)
                                                        .small(),
                                                );
                                            }
                                            ui.label(
                                                egui::RichText::new(&e.sex).monospace().small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(&e.since).monospace().small(),
                                            );
                                            if e.compensation > 0.0 {
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "${:.2}M",
                                                        e.compensation / 1e6
                                                    ))
                                                    .color(UP)
                                                    .monospace(),
                                                );
                                            } else {
                                                ui.label(
                                                    egui::RichText::new("—")
                                                        .color(AXIS_TEXT)
                                                        .small(),
                                                );
                                            }
                                            ui.end_row();
                                        }
                                    });
                            });
                    }
                });
            self.show_executives = open;
        }

        // COT — CFTC Commitments of Traders (weekly)
        if self.show_cot {
            let mut open = self.show_cot;
            egui::Window::new("COT — Commitments of Traders")
                .open(&mut open)
                .resizable(true)
                .default_size([920.0, 560.0])
                .max_size([920.0, 560.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if ui.add(egui::Button::new("Fetch").fill(BTN_MG)).clicked() {
                            self.cot_loading = true;
                            let _ = self.broker_tx.send(BrokerCmd::FetchCotReports);
                        }
                        ui.label(egui::RichText::new("Filter:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cot_filter).desired_width(200.0).hint_text("e.g. GOLD, CRUDE, S&P"));
                        if self.cot_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                        if let Some(ts) = self.cot_last_fetch {
                            let mins = ts.elapsed().as_secs() / 60;
                            ui.label(egui::RichText::new(format!("Updated {}m ago", mins)).color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    if self.cot_reports.is_empty() {
                        ui.label(egui::RichText::new("No CoT data — click Fetch to pull the latest weekly report from CFTC.").color(AXIS_TEXT).small());
                    } else {
                        let filter = self.cot_filter.to_uppercase();
                        let filtered: Vec<&typhoon_engine::core::research::CotReport> = self.cot_reports.iter()
                            .filter(|r| filter.is_empty() || r.market_name.to_uppercase().contains(&filter))
                            .collect();
                        let latest_date = self.cot_reports.first().map(|r| r.report_date.clone()).unwrap_or_default();
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(format!("Week of {}", latest_date)).strong());
                            ui.label(egui::RichText::new(format!("({} of {} markets shown)", filtered.len(), self.cot_reports.len())).color(AXIS_TEXT).small());
                        });
                        ui.separator();
                        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                            egui::Grid::new("cot_grid").striped(true).num_columns(7).spacing([12.0, 2.0]).show(ui, |ui| {
                                ui.label(egui::RichText::new("Market").strong());
                                ui.label(egui::RichText::new("Open Int").strong());
                                ui.label(egui::RichText::new("NC Long").strong());
                                ui.label(egui::RichText::new("NC Short").strong());
                                ui.label(egui::RichText::new("NC Net").strong());
                                ui.label(egui::RichText::new("Δ Net").strong());
                                ui.label(egui::RichText::new("Comm Net").strong());
                                ui.end_row();
                                let fmt_k = |v: f64| -> String {
                                    if v.abs() >= 1e6 { format!("{:.2}M", v / 1e6) }
                                    else if v.abs() >= 1e3 { format!("{:.1}K", v / 1e3) }
                                    else { format!("{:.0}", v) }
                                };
                                for r in filtered.iter().take(200) {
                                    let short_name: String = r.market_name.split(" - ").next().unwrap_or(&r.market_name).chars().take(36).collect();
                                    ui.label(egui::RichText::new(short_name).small());
                                    ui.label(egui::RichText::new(fmt_k(r.open_interest)).monospace().small());
                                    ui.label(egui::RichText::new(fmt_k(r.noncomm_long)).color(UP).monospace().small());
                                    ui.label(egui::RichText::new(fmt_k(r.noncomm_short)).color(DOWN).monospace().small());
                                    let net_col = if r.noncomm_net < 0.0 { DOWN } else { UP };
                                    ui.label(egui::RichText::new(fmt_k(r.noncomm_net)).color(net_col).monospace().strong());
                                    let chg_col = if r.noncomm_net_change < 0.0 { DOWN } else if r.noncomm_net_change > 0.0 { UP } else { AXIS_TEXT };
                                    ui.label(egui::RichText::new(format!("{:+}", fmt_k(r.noncomm_net_change))).color(chg_col).monospace().small());
                                    let comm_net = r.comm_long - r.comm_short;
                                    let comm_col = if comm_net < 0.0 { DOWN } else { UP };
                                    ui.label(egui::RichText::new(fmt_k(comm_net)).color(comm_col).monospace().small());
                                    ui.end_row();
                                }
                            });
                        });
                    }
                });
            self.show_cot = open;
        }
    }
}
