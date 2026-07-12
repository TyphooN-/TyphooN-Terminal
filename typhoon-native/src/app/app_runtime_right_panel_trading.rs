use super::*;

#[allow(deprecated)]
impl TyphooNApp {
    pub(super) fn render_right_panel_trading_section(&mut self, ui: &mut egui::Ui) {
        // ── Trading Section ──────────────────────────────────
        let trading_section =
            egui::CollapsingHeader::new(egui::RichText::new("☰ Trading").strong().small())
                .default_open(self.right_trading_open)
                .show(ui, |ui| {
                    // ── Trading Buttons Grid (exact WebKit CSS: #button-grid) ──
                    let trading_enabled = true;
                    self.resolve_order_broker();
                    if !trading_enabled {
                        ui.disable();
                    }
                    ui.add_space(8.0);
                    ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                    let btn_w = (ui.available_width() - 4.0) / 2.0;
                    let btn_size = egui::vec2(btn_w, 28.0); // padding: 8px 4px ≈ 28px

                    // Row 1: Open Trade (.btn-action) | Buy Lines (.btn-lines)
                    ui.horizontal(|ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("Open Trade")
                                        .color(BTN_GREEN_TEXT)
                                        .small()
                                        .strong(),
                                )
                                .fill(BTN_GREEN)
                                .min_size(btn_size),
                            )
                            .clicked()
                        {
                            self.submit_quick_trade();
                        }
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("Buy Lines")
                                        .color(BTN_BLUE_TEXT)
                                        .small()
                                        .strong(),
                                )
                                .fill(BTN_BLUE)
                                .min_size(btn_size),
                            )
                            .clicked()
                        {
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
                        }
                    });
                    // Row 2: Sell Lines (.btn-lines) | Destroy Lines (.btn-lines)
                    ui.horizontal(|ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("Sell Lines")
                                        .color(BTN_BLUE_TEXT)
                                        .small()
                                        .strong(),
                                )
                                .fill(BTN_BLUE)
                                .min_size(btn_size),
                            )
                            .clicked()
                        {
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
                        }
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("Destroy Lines")
                                        .color(BTN_BLUE_TEXT)
                                        .small()
                                        .strong(),
                                )
                                .fill(BTN_BLUE)
                                .min_size(btn_size),
                            )
                            .on_hover_text("Remove all buy/sell planning lines from chart")
                            .clicked()
                        {
                            self.clear_trade_lines();
                        }
                    });
                    // Row 3: Set SL | Set TP (.btn-lines)
                    ui.horizontal(|ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("Set SL")
                                        .color(BTN_BLUE_TEXT)
                                        .small()
                                        .strong(),
                                )
                                .fill(BTN_BLUE)
                                .min_size(btn_size),
                            )
                            .clicked()
                        {
                            self.apply_current_sl_to_positions();
                        }
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("Set TP")
                                        .color(BTN_BLUE_TEXT)
                                        .small()
                                        .strong(),
                                )
                                .fill(BTN_BLUE)
                                .min_size(btn_size),
                            )
                            .clicked()
                        {
                            self.apply_current_tp_to_positions();
                        }
                    });
                    ui.add_space(6.0);

                    // ── SL / TP Price Inputs ──────────────────────────
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.sl_enabled, "");
                        ui.label(egui::RichText::new("SL Price").color(AXIS_TEXT).small());
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut self.sl_input)
                                .desired_width(100.0)
                                .hint_text("0.0")
                                .font(egui::TextStyle::Small),
                        );
                        if resp.lost_focus() && self.sl_enabled {
                            self.sl_price = self.sl_input.parse().ok();
                            self.mark_trade_lines_owner();
                            self.sync_trade_line_inputs();
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.tp_enabled, "");
                        ui.label(egui::RichText::new("TP Price").color(AXIS_TEXT).small());
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut self.tp_input)
                                .desired_width(100.0)
                                .hint_text("0.0")
                                .font(egui::TextStyle::Small),
                        );
                        if resp.lost_focus() && self.tp_enabled {
                            self.tp_price = self.tp_input.parse().ok();
                            self.mark_trade_lines_owner();
                            self.sync_trade_line_inputs();
                        }
                    });
                    ui.add_space(6.0);

                    // ── Mode / Broker Controls ──────────────────────────
                    ui.separator();
                    let wants_kraken_pro =
                        self.kraken_connected && matches!(self.risk_mode, RiskMode::KrakenPro);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Mode").color(AXIS_TEXT).small());
                        egui::ComboBox::from_id_salt("risk_mode_combo")
                            .selected_text(self.risk_mode.label())
                            .width(96.0)
                            .show_ui(ui, |ui| {
                                for mode in [
                                    RiskMode::VaR,
                                    RiskMode::Standard,
                                    RiskMode::Fixed,
                                    RiskMode::Dynamic,
                                ] {
                                    ui.selectable_value(&mut self.risk_mode, mode, mode.label());
                                }
                                if self.kraken_connected
                                    && ui
                                        .selectable_value(
                                            &mut self.risk_mode,
                                            RiskMode::KrakenPro,
                                            RiskMode::KrakenPro.label(),
                                        )
                                        .clicked()
                                {
                                    self.order_broker = OrderBroker::Kraken;
                                }
                            });
                    });
                    if !wants_kraken_pro {
                        match self.risk_mode {
                            RiskMode::Standard => {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new("Risk %").color(AXIS_TEXT).small(),
                                    );
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.trade_risk_pct_input)
                                            .desired_width(64.0)
                                            .hint_text("0.5")
                                            .font(egui::TextStyle::Small),
                                    );
                                });
                            }
                            RiskMode::Fixed => {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("Qty").color(AXIS_TEXT).small());
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.order_qty)
                                            .desired_width(80.0)
                                            .hint_text("1.0")
                                            .font(egui::TextStyle::Small),
                                    );
                                });
                            }
                            RiskMode::Dynamic => {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new("Min Bal").color(AXIS_TEXT).small(),
                                    );
                                    ui.add(
                                        egui::TextEdit::singleline(
                                            &mut self.trade_min_balance_input,
                                        )
                                        .desired_width(80.0)
                                        .hint_text("96100")
                                        .font(egui::TextStyle::Small),
                                    );
                                });
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new("Losses").color(AXIS_TEXT).small(),
                                    );
                                    ui.add(
                                        egui::TextEdit::singleline(
                                            &mut self.trade_losses_to_min_input,
                                        )
                                        .desired_width(64.0)
                                        .hint_text("10")
                                        .font(egui::TextStyle::Small),
                                    );
                                });
                            }
                            RiskMode::VaR | RiskMode::KrakenPro => {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("VaR %").color(AXIS_TEXT).small());
                                    ui.add(
                                        egui::TextEdit::singleline(
                                            &mut self.trade_var_risk_pct_input,
                                        )
                                        .desired_width(64.0)
                                        .hint_text("0.9")
                                        .font(egui::TextStyle::Small),
                                    );
                                });
                            }
                        }
                        if let Ok(plan) = self.quick_trade_plan() {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "Setup {} {:.4}",
                                        if plan.side_idx == 0 { "BUY" } else { "SELL" },
                                        plan.qty
                                    ))
                                    .color(if plan.side_idx == 0 { UP } else { DOWN })
                                    .small()
                                    .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(plan.symbol.clone())
                                        .color(AXIS_TEXT)
                                        .small(),
                                );
                            });
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(format!("Risk ${:.2}", plan.risk_dollars))
                                        .color(DOWN)
                                        .small(),
                                );
                                if let Some(risk_pct) = plan.risk_pct {
                                    ui.label(
                                        egui::RichText::new(format!("({:.2}%)", risk_pct))
                                            .color(AXIS_TEXT)
                                            .small(),
                                    );
                                }
                                ui.label(
                                    egui::RichText::new(format!("TP ${:.2}", plan.reward_dollars))
                                        .color(UP)
                                        .small(),
                                );
                                if let Some(rr) = plan.rr {
                                    ui.label(
                                        egui::RichText::new(format!("RR {:.2}", rr))
                                            .color(AXIS_TEXT)
                                            .small(),
                                    );
                                }
                            });
                        } else if self.sl_price.is_some() || self.tp_price.is_some() {
                            if let Err(e) = self.quick_trade_plan() {
                                ui.label(egui::RichText::new(e).color(AXIS_TEXT).small());
                            }
                        }
                    }
                    // Broker target selector (only show when any enabled broker can place orders)
                    if self.alpaca_order_available() || self.kraken_order_available() {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Broker").color(AXIS_TEXT).small());
                            egui::ComboBox::from_id_salt("order_broker_combo")
                                .selected_text(self.order_broker.label())
                                .width(90.0)
                                .show_ui(ui, |ui| {
                                    if self.alpaca_order_available() {
                                        ui.selectable_value(
                                            &mut self.order_broker,
                                            OrderBroker::Alpaca,
                                            "Alpaca",
                                        );
                                    }
                                    if self.kraken_order_available() {
                                        ui.selectable_value(
                                            &mut self.order_broker,
                                            OrderBroker::Kraken,
                                            "Kraken",
                                        );
                                    }
                                });
                        });
                    }
                    ui.add_space(6.0);

                    if wants_kraken_pro {
                        self.render_kraken_spot_buy_controls(ui);
                        ui.add_space(6.0);
                    }

                    // ── Position Info Block ────────────────────────────
                    ui.separator();
                    if let Some(chart) = self.charts.get(self.active_tab) {
                        if let Some(bar) = chart.bars.last() {
                            let close = bar.close;
                            let chart_symbol = bare_symbol_from_key(&chart.symbol)
                                .replace('/', "")
                                .trim_end_matches(".EQ")
                                .trim_end_matches(".eq")
                                .to_ascii_uppercase();
                            let active_pos = self
                                .live_positions_by_symbol
                                .get(&chart_symbol)
                                .or_else(|| self.kr_positions_by_symbol.get(&chart_symbol));
                            // SL/TP/R:R info (trading-specific). Risk/equity summaries
                            // now live only inside the Risk & Account widget.
                            if let Some(pos) = active_pos {
                                ui.horizontal_wrapped(|ui| {
                                    if let Some(sl) = self.sl_price {
                                        let sl_pl = (close - sl)
                                            * pos.qty
                                            * if pos.side == "long" { 1.0 } else { -1.0 };
                                        let sl_c = if sl_pl >= 0.0 { UP } else { DOWN };
                                        ui.label(
                                            egui::RichText::new(format!("SL P/L: ${:.2}", sl_pl))
                                                .color(sl_c)
                                                .small(),
                                        );
                                    }
                                    if let Some(tp) = self.tp_price {
                                        let tp_pl = (tp - close)
                                            * pos.qty
                                            * if pos.side == "long" { 1.0 } else { -1.0 };
                                        let tp_c = if tp_pl >= 0.0 { UP } else { DOWN };
                                        ui.label(
                                            egui::RichText::new(format!("TP P/L: ${:.2}", tp_pl))
                                                .color(tp_c)
                                                .small(),
                                        );
                                    }
                                    if let (Some(sl), Some(tp)) = (self.sl_price, self.tp_price) {
                                        let risk = (close - sl).abs();
                                        let reward = (tp - close).abs();
                                        let rr = if risk > 0.0 { reward / risk } else { 0.0 };
                                        ui.label(
                                            egui::RichText::new(format!("R:R {:.2}", rr))
                                                .color(AXIS_TEXT)
                                                .small(),
                                        );
                                    }
                                });
                            }
                        }
                    }
                });
        self.right_trading_open = trading_section.fully_open();
        self.handle_right_panel_section_drag(
            ui,
            RightPanelSectionId::Trading,
            &trading_section.header_response,
        );
    }
}
