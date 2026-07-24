use super::*;
use crate::app::app_runtime_tabs::tab_bar_chart_indices;

#[allow(deprecated)]
impl TyphooNApp {
    pub(super) fn render_menu_bar(&mut self, root_ui: &mut egui::Ui) {
        let ctx = &root_ui.ctx().clone();
        // ── top menu bar ─────────────────────────────────────────────────────
        egui::Panel::top("menu_bar").show(root_ui, |ui| {
                    egui::MenuBar::new().ui(ui, |ui| {
                        ui.menu_button("File", |ui| {
                            if ui.button("Settings").clicked() {
                                self.show_settings = true;
                                ui.close();
                            }
                            ui.separator();
                            if ui.button("Quit  Alt+F4").clicked() {
                                self.save_session();
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        });
                        ui.menu_button("View", |ui| {
                            let tab_indices = tab_bar_chart_indices(&self.charts);
                            let mtf_label = if self.mtf_enabled {
                                "Single Chart".to_string()
                            } else {
                                format!("MTF Grid ({} charts)", tab_indices.len())
                            };
                            if ui.button(&mtf_label).clicked() {
                                self.mtf_enabled = !self.mtf_enabled;
                                ui.close();
                            }
                            // MTF tab visibility checkboxes
                            if tab_indices.len() > 1 {
                                ui.menu_button("MTF Tabs", |ui| {
                                    // Ensure mtf_visible is the right size
                                    while self.mtf_visible.len() < self.charts.len() {
                                        self.mtf_visible.push(true);
                                    }
                                    ui.horizontal(|ui| {
                                        if ui.small_button("All").clicked() {
                                            for idx in &tab_indices {
                                                if let Some(visible) = self.mtf_visible.get_mut(*idx) {
                                                    *visible = true;
                                                }
                                            }
                                        }
                                        if ui.small_button("None").clicked() {
                                            for idx in &tab_indices {
                                                if let Some(visible) = self.mtf_visible.get_mut(*idx) {
                                                    *visible = false;
                                                }
                                            }
                                            if let Some(idx) = tab_indices.first().copied() {
                                                self.mtf_visible[idx] = true;
                                            }
                                        }
                                    });
                                    ui.separator();
                                    for i in &tab_indices {
                                        let Some(chart) = self.charts.get(*i) else {
                                            continue;
                                        };
                                        let label = format!(
                                            "{} [{}]",
                                            chart
                                                .symbol
                                                .split(':')
                                                .nth(1)
                                                .or(Some(&chart.symbol))
                                                .unwrap_or(&chart.symbol),
                                            chart.timeframe.label()
                                        );
                                        if *i < self.mtf_visible.len() {
                                            ui.checkbox(&mut self.mtf_visible[*i], label);
                                        }
                                    }
                                });
                            }
                            if ui.button("Indicators…").clicked() {
                                self.show_indicators_panel = true;
                                ui.close();
                            }
                            ui.separator();
                            ui.label(egui::RichText::new("Chart Type").color(AXIS_TEXT).small());
                            let ct = self
                                .charts
                                .get(self.active_tab)
                                .map(|c| c.chart_type)
                                .unwrap_or(ChartType::Candle);
                            for &chart_type in &[
                                ChartType::Candle,
                                ChartType::HeikinAshi,
                                ChartType::Line,
                                ChartType::OhlcBars,
                                ChartType::Renko,
                            ] {
                                let selected = ct == chart_type;
                                let label = if selected {
                                    format!("● {}", chart_type.label())
                                } else {
                                    format!("  {}", chart_type.label())
                                };
                                if ui.button(label).clicked() {
                                    if let Some(c) = self.charts.get_mut(self.active_tab) {
                                        c.chart_type = chart_type;
                                    }
                                    ui.close();
                                }
                            }
                            ui.separator();
                            ui.label(
                                egui::RichText::new("Overlay Indicators")
                                    .color(AXIS_TEXT)
                                    .small(),
                            );
                            ui.checkbox(&mut self.show_sma200, "SMA 200");
                            ui.checkbox(&mut self.show_sma100, "SMA 100");
                            ui.checkbox(&mut self.show_kama, "KAMA(10,2,30)");
                            ui.checkbox(&mut self.show_ema21, "EMA 21");
                            ui.checkbox(&mut self.show_bollinger, "Bollinger Bands");
                            ui.separator();
                            ui.checkbox(&mut self.show_ichimoku, "Ichimoku Cloud");
                            ui.checkbox(&mut self.show_wma, "WMA(20)");
                            ui.checkbox(&mut self.show_hma, "HMA(20)");
                            ui.checkbox(&mut self.show_psar, "Parabolic SAR");
                            ui.checkbox(&mut self.show_atr_proj, "ATR Projection");
                            ui.checkbox(&mut self.show_prev_levels, "Prev Candle Levels (D/W)");
                            ui.checkbox(&mut self.show_pivots, "Pivot Points (P/R1/R2/S1/S2)");
                            ui.checkbox(&mut self.show_supply_demand, "Supply/Demand Zones");
                            ui.checkbox(&mut self.show_fvg, "Fair Value Gaps (FVG)");
                            ui.checkbox(&mut self.show_order_blocks, "Order Blocks (ICT/SMC)");
                            ui.separator();
                            ui.label(
                                egui::RichText::new("Pattern Recognition")
                                    .color(AXIS_TEXT)
                                    .small(),
                            );
                            ui.checkbox(&mut self.show_fractals, "Fractals (Bill Williams)");
                            ui.checkbox(&mut self.show_harmonics, "Harmonic Patterns (Carney)");
                            ui.checkbox(&mut self.show_auto_fib, "Auto Fibonacci");
                            ui.separator();
                            ui.label(
                                egui::RichText::new("Ehlers (Overlay)")
                                    .color(AXIS_TEXT)
                                    .small(),
                            );
                            ui.checkbox(&mut self.show_ehlers_ss, "Super Smoother(10)");
                            ui.checkbox(&mut self.show_ehlers_decycler, "Decycler(20)");
                            ui.checkbox(&mut self.show_ehlers_itl, "Instant. Trendline");
                            ui.checkbox(&mut self.show_ehlers_mama, "MAMA / FAMA");
                            ui.separator();
                            ui.label(egui::RichText::new("Sub-Panes").color(AXIS_TEXT).small());
                            ui.checkbox(&mut self.show_rsi, "RSI(14)");
                            ui.checkbox(&mut self.show_fisher, "Fisher Transform");
                            ui.checkbox(&mut self.show_macd, "MACD(12,26,9)");
                            ui.checkbox(&mut self.show_stochastic, "Stochastic(14,3,3)");
                            ui.checkbox(&mut self.show_adx, "ADX(14)");
                            ui.checkbox(&mut self.show_cci, "CCI(20)");
                            ui.checkbox(&mut self.show_williams_r, "Williams %R(14)");
                            ui.checkbox(&mut self.show_obv, "OBV");
                            ui.checkbox(&mut self.show_momentum, "Momentum(10)");
                            ui.checkbox(&mut self.show_cmo, "CMO(9)");
                            ui.checkbox(&mut self.show_qstick, "QStick(14)");
                            ui.checkbox(&mut self.show_disparity, "Disparity(14)");
                            ui.checkbox(&mut self.show_bop, "BOP(14)");
                            ui.checkbox(&mut self.show_stddev, "StdDev(20)");
                            ui.checkbox(&mut self.show_mfi, "MFI(14)");
                            ui.checkbox(&mut self.show_trix, "TRIX(15,9)");
                            ui.checkbox(&mut self.show_ppo, "PPO(12,26,9)");
                            ui.checkbox(&mut self.show_ultosc, "ULTOSC(7,14,28)");
                            ui.checkbox(&mut self.show_stochrsi, "StochRSI(14,14,3,3)");
                            ui.checkbox(&mut self.show_var_oscillator, "VaR Oscillator(20,95%)");
                            ui.checkbox(&mut self.show_better_volume, "Better Volume");
                            ui.checkbox(&mut self.show_volume_pane, "Volume");
                        });
                        ui.menu_button("Trading", |ui| {
                            if ui.button("Open Trade").clicked() {
                                self.submit_quick_trade();
                                ui.close();
                            }
                            ui.separator();
                            if ui.button("Set SL").clicked() {
                                self.apply_current_sl_to_positions();
                                ui.close();
                            }
                            if ui.button("Set TP").clicked() {
                                self.apply_current_tp_to_positions();
                                ui.close();
                            }
                            if ui.button("Buy Lines").clicked() {
                                match self.set_visible_range_trade_lines(true) {
                                    Ok((sl, tp)) => {
                                        self.log.push_back(LogEntry::info(format!(
                                            "Buy Lines: SL {} TP {} (drag to adjust)",
                                            format_price(sl),
                                            format_price(tp)
                                        )));
                                    }
                                    Err(e) => self.log.push_back(LogEntry::warn(e)),
                                }
                                ui.close();
                            }
                            if ui.button("Sell Lines").clicked() {
                                match self.set_visible_range_trade_lines(false) {
                                    Ok((sl, tp)) => {
                                        self.log.push_back(LogEntry::info(format!(
                                            "Sell Lines: SL {} TP {} (drag to adjust)",
                                            format_price(sl),
                                            format_price(tp)
                                        )));
                                    }
                                    Err(e) => self.log.push_back(LogEntry::warn(e)),
                                }
                                ui.close();
                            }
                            ui.separator();
                            if ui.button("Set SL Line").clicked() {
                                self.apply_current_sl_to_positions();
                                ui.close();
                            }
                            if ui.button("Set TP Line").clicked() {
                                self.apply_current_tp_to_positions();
                                ui.close();
                            }
                            if self.sl_price.is_some() || self.tp_price.is_some() {
                                if ui.button("Clear SL/TP Lines").clicked() {
                                    self.clear_trade_lines();
                                    ui.close();
                                }
                            }
                            ui.separator();
                            let tradecopy_label = if self.tradecopy_mirror_orders {
                                "TradeCopy…  [mirroring ON]"
                            } else {
                                "TradeCopy…"
                            };
                            if ui
                                .button(tradecopy_label)
                                .on_hover_text("Copy trades between broker accounts: one-shot position copy plus live order mirroring across all trade-enabled accounts (ADR-130).")
                                .clicked()
                            {
                                self.show_tradecopy = true;
                                ui.close();
                            }
                        });
                        ui.menu_button("Tools", |ui| {
                            if ui.button("Console (~)").clicked() {
                                self.command_open = !self.command_open;
                                if self.command_open {
                                    self.command_input.clear();
                                }
                                ui.close();
                            }
                            ui.separator();
                            if ui.button("Backtest").clicked() {
                                self.show_backtest = true;
                                ui.close();
                            }
                            if ui.button("Screener").clicked() {
                                self.show_screener = true;
                                ui.close();
                            }
                            if ui.button("Optimizer").clicked() {
                                self.show_optimizer = true;
                                ui.close();
                            }
                            ui.separator();
                            if ui.button("Risk Calculator").clicked() {
                                self.show_risk_calc = true;
                                ui.close();
                            }
                            if ui.button("VaR Multiplier").clicked() {
                                self.show_var_mult = true;
                                ui.close();
                            }
                            if ui.button("Margin Monitor").clicked() {
                                self.show_margin_monitor = true;
                                ui.close();
                            }
                            ui.separator();
                            if ui.button("Cache Statistics").clicked() {
                                self.show_cache_stats = true;
                                ui.close();
                            }
                        });
                        ui.menu_button("Research", |ui| {
                            if ui.button("News & Events").clicked() {
                                self.show_news = true;
                                ui.close();
                            }
                            if ui.button("Economic Calendar").clicked() {
                                self.show_calendar = true;
                                ui.close();
                            }
                            if ui.button("SEC Filings").clicked() {
                                self.show_sec = true;
                                ui.close();
                            }
                            if ui.button("Insider Trades").clicked() {
                                self.show_insider = true;
                                ui.close();
                            }
                            ui.separator();
                            if ui.button("Fundamentals").clicked() {
                                self.show_fundamentals = true;
                                ui.close();
                            }
                            if ui.button("Analyst Ratings").clicked() {
                                self.show_analyst = true;
                                ui.close();
                            }
                            if ui.button("Institutional Holders").clicked() {
                                self.show_holders = true;
                                ui.close();
                            }
                        });
                        ui.menu_button("Analysis", |ui| {
                            if ui.button("Correlation Matrix").clicked() {
                                self.show_cor = true;
                                ui.close();
                            }
                            if ui.button("Seasonals").clicked() {
                                self.show_seasonals = true;
                                ui.close();
                            }
                            if ui.button("Monte Carlo VaR").clicked() {
                                self.show_montecarlo = true;
                                ui.close();
                            }
                            if ui.button("Stress Test").clicked() {
                                self.show_stress_test = true;
                                ui.close();
                            }
                            ui.separator();
                            if ui.button("Volume Profile").clicked() {
                                self.show_volume_profile = true;
                                ui.close();
                            }
                            if ui.button("Depth Profile (L2)").clicked() {
                                self.show_depth_profile = true;
                                ui.close();
                            }
                            if ui.button("Order Flow").clicked() {
                                self.show_order_flow = true;
                                ui.close();
                            }
                            if ui.button("Bookmap Heatmap").clicked() {
                                self.open_bookmap_window(None);
                                ui.close();
                            }
                        });
                        ui.menu_button("Help", |ui| {
                            if ui.button("Keyboard Shortcuts").clicked() {
                                self.show_help = true;
                                ui.close();
                            }
                            ui.separator();
                            ui.label(
                                egui::RichText::new("TyphooN Terminal v0.1.0")
                                    .color(AXIS_TEXT)
                                    .small(),
                            );
                            ui.label(egui::RichText::new("egui + wgpu").color(AXIS_TEXT).small());
                        });
                        ui.separator();
                        ui.label(
                            egui::RichText::new("TyphooN Terminal")
                                .color(ACCENT)
                                .strong(),
                        );
                        // The Scope and Primary switches only mean something with
                        // 2+ enabled brokers; with a single broker there is nothing
                        // to filter or re-prioritize, so both are hidden.
                        let top_brokers =
                            OrderBroker::enabled_cycle(self.alpaca_enabled, self.kraken_enabled);
                        // Broker scope indicator — click to cycle through scopes.
                        // Shows the current global filter so the trader always knows what
                        // data universe they're looking at (All / enabled brokers).
                        if top_brokers.len() >= 2 {
                        ui.separator();
                        let (scope_lbl, scope_col) = match self.broker_scope {
                            EventSource::All => ("All", egui::Color32::from_rgb(140, 140, 160)),
                            EventSource::Alpaca => ("Alpaca", egui::Color32::from_rgb(255, 160, 60)),
                            EventSource::Kraken => ("Kraken", egui::Color32::from_rgb(0, 170, 160)),
                            EventSource::Positions => ("Positions", egui::Color32::from_rgb(80, 220, 120)),
                        };
                        let scope_btn = egui::Button::new(
                            egui::RichText::new(format!("Scope: {}", scope_lbl))
                                .strong()
                                .color(egui::Color32::WHITE),
                        )
                        .fill(scope_col);
                        if ui
                            .add(scope_btn)
                            .on_hover_text("Left-click: cycle ALL and enabled brokers. Right-click: open scope settings.")
                            .clicked()
                        {
                            let mut scope_cycle = vec![EventSource::All];
                            if self.alpaca_enabled {
                                scope_cycle.push(EventSource::Alpaca);
                            }
                            if self.kraken_enabled {
                                scope_cycle.push(EventSource::Kraken);
                            }
                            let next_idx = scope_cycle
                                .iter()
                                .position(|scope| *scope == self.broker_scope)
                                .map(|idx| (idx + 1) % scope_cycle.len())
                                .unwrap_or(0);
                            self.broker_scope = scope_cycle[next_idx];
                            // Sync fund_source toggles
                            match self.broker_scope {
                                EventSource::All => {
                                    self.fund_source_alpaca = true;
                                    self.fund_source_kraken = true;
                                }
                                EventSource::Alpaca => {
                                    self.fund_source_alpaca = true;
                                    self.fund_source_kraken = false;
                                }
                                EventSource::Kraken => {
                                    self.fund_source_alpaca = false;
                                    self.fund_source_kraken = true;
                                }
                                EventSource::Positions => {
                                    self.fund_source_alpaca = true;
                                    self.fund_source_kraken = true;
                                }
                            }
                            // The scope set is resolved by the logic() pump, which
                            // already ran this frame — without this it reports the
                            // count for the scope we just left.
                            let _ = self.refresh_broker_scope_cache();
                            let n = self.scoped_fundamentals().len();
                            self.log.push_back(LogEntry::info(format!(
                                "Broker scope → {} ({} fundamentals in scope)",
                                self.broker_scope_label(),
                                n
                            )));
                        }
                        } // end Scope switch (only shown with 2+ enabled brokers)
                        // Primary switch — click to cycle which enabled broker
                        // ACCOUNT is PRIMARY (order-routing default + trusted
                        // equity-merge lane). The cycle now walks every connected
                        // account of every enabled broker (ADR-130), so e.g.
                        // Alpaca Live → Alpaca Paper 1 → … → Kraken. Every other
                        // enabled broker stays a sync ASSIST lane (ADR-126).
                        let account_cycle = self.primary_account_cycle();
                        if top_brokers.len() >= 2 || account_cycle.len() >= 2 {
                            ui.separator();
                            let primary_col = match self.primary_broker {
                                OrderBroker::Alpaca => egui::Color32::from_rgb(255, 160, 60),
                                OrderBroker::Kraken => egui::Color32::from_rgb(0, 170, 160),
                            };
                            let current_account_id =
                                self.primary_account_id_for(self.primary_broker);
                            let chip_label = account_cycle
                                .iter()
                                .find(|(broker, id, _)| {
                                    *broker == self.primary_broker && *id == current_account_id
                                })
                                .map(|(_, _, label)| label.clone())
                                .unwrap_or_else(|| self.primary_broker.label().to_string());
                            let primary_btn = egui::Button::new(
                                egui::RichText::new(format!("Primary: {}", chip_label))
                                    .strong()
                                    .color(egui::Color32::WHITE),
                            )
                            .fill(primary_col);
                            if ui
                                .add(primary_btn)
                                .on_hover_text("Primary account = order-routing default + trusted data-merge lane; other enabled brokers are sync assist lanes. Click to cycle through every connected broker account.")
                                .clicked()
                                && !account_cycle.is_empty()
                            {
                                let next_idx = account_cycle
                                    .iter()
                                    .position(|(broker, id, _)| {
                                        *broker == self.primary_broker
                                            && *id == current_account_id
                                    })
                                    .map(|idx| (idx + 1) % account_cycle.len())
                                    .unwrap_or(0);
                                let (next_broker, next_account, _) =
                                    account_cycle[next_idx].clone();
                                self.apply_primary_selection(next_broker, &next_account);
                            }
                        }
                        // Alert breach badge — visible red counter when alerts have fired.
                        // Clicking clears the counter and opens the alerts window.
                        if self.alert_breach_count > 0 {
                            ui.separator();
                            let breach_label = format!("🔔 {} ALERT", self.alert_breach_count);
                            let tooltip = if self.alert_last_breach_msg.is_empty() {
                                format!(
                                    "{} alert(s) fired — click to view and clear",
                                    self.alert_breach_count
                                )
                            } else {
                                format!(
                                    "{} alert(s) fired — latest:\n{}\n\nClick to view and clear.",
                                    self.alert_breach_count, self.alert_last_breach_msg
                                )
                            };
                            let btn = egui::Button::new(
                                egui::RichText::new(breach_label)
                                    .strong()
                                    .color(egui::Color32::WHITE),
                            )
                            .fill(egui::Color32::from_rgb(231, 76, 60));
                            if ui.add(btn).on_hover_text(tooltip).clicked() {
                                self.show_alert_builder = true;
                                self.alert_breach_count = 0;
                                self.alert_last_breach_msg.clear();
                            }
                        }
                    });
                });
    }
}
