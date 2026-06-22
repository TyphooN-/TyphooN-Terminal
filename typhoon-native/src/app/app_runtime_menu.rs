use super::*;
use crate::app::chart_ops::MTF_GRID_TIMEFRAMES;

#[allow(deprecated)]
impl TyphooNApp {
    pub(super) fn render_menu_bar(&mut self, ctx: &egui::Context) {
        // ── top menu bar ─────────────────────────────────────────────────────
        egui::Panel::top("menu_bar").show(ctx, |ui| {
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
                            let mtf_label = if self.mtf_enabled {
                                "Single Chart".to_string()
                            } else {
                                format!("MTF Grid ({} charts)", self.charts.len())
                            };
                            if ui.button(&mtf_label).clicked() {
                                self.mtf_enabled = !self.mtf_enabled;
                                ui.close();
                            }
                            ui.menu_button("Grid Layout", |ui| {
                                if ui.button("2 columns per symbol").clicked() {
                                    self.setup_mtf_grid(2, MTF_GRID_TIMEFRAMES.len());
                                    ui.close();
                                }
                                if ui.button("3 columns per symbol").clicked() {
                                    self.setup_mtf_grid(3, MTF_GRID_TIMEFRAMES.len());
                                    ui.close();
                                }
                                if ui.button("4 columns per symbol").clicked() {
                                    self.setup_mtf_grid(4, MTF_GRID_TIMEFRAMES.len());
                                    ui.close();
                                }
                            });
                            // MTF tab visibility checkboxes
                            if self.charts.len() > 1 {
                                ui.menu_button("MTF Tabs", |ui| {
                                    // Ensure mtf_visible is the right size
                                    while self.mtf_visible.len() < self.charts.len() {
                                        self.mtf_visible.push(true);
                                    }
                                    ui.horizontal(|ui| {
                                        if ui.small_button("All").clicked() {
                                            self.mtf_visible.iter_mut().for_each(|v| *v = true);
                                        }
                                        if ui.small_button("None").clicked() {
                                            self.mtf_visible.iter_mut().for_each(|v| *v = false);
                                            self.mtf_visible[0] = true;
                                        }
                                    });
                                    ui.separator();
                                    for (i, chart) in self.charts.iter().enumerate() {
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
                                        if i < self.mtf_visible.len() {
                                            ui.checkbox(&mut self.mtf_visible[i], label);
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
                            EventSource::All => ("ALL", egui::Color32::from_rgb(140, 140, 160)),
                            EventSource::Alpaca => ("ALPACA", egui::Color32::from_rgb(255, 160, 60)),
                            EventSource::Kraken => ("KRAKEN", egui::Color32::from_rgb(0, 170, 160)),
                            EventSource::Positions => ("POSITIONS", egui::Color32::from_rgb(80, 220, 120)),
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
                            let n = self.scoped_fundamentals().len();
                            self.log.push_back(LogEntry::info(format!(
                                "Broker scope → {} ({} fundamentals in scope)",
                                self.broker_scope_label(),
                                n
                            )));
                        }
                        } // end Scope switch (only shown with 2+ enabled brokers)
                        // Primary broker switch — click to cycle which enabled
                        // broker is PRIMARY (order-routing default + trusted
                        // equity-merge lane). Every other enabled broker becomes a
                        // sync ASSIST lane. Mirrors the Scope button; scales to N
                        // brokers via OrderBroker::enabled_cycle.
                        if top_brokers.len() >= 2 {
                            let enabled_brokers = &top_brokers;
                            ui.separator();
                            let primary_col = match self.primary_broker {
                                OrderBroker::Alpaca => egui::Color32::from_rgb(255, 160, 60),
                                OrderBroker::Kraken => egui::Color32::from_rgb(0, 170, 160),
                            };
                            let primary_btn = egui::Button::new(
                                egui::RichText::new(format!(
                                    "Primary: {}",
                                    self.primary_broker.label()
                                ))
                                .strong()
                                .color(egui::Color32::WHITE),
                            )
                            .fill(primary_col);
                            if ui
                                .add(primary_btn)
                                .on_hover_text("Primary broker = order-routing default + trusted data-merge lane; other enabled brokers are sync assist lanes. Click to cycle.")
                                .clicked()
                            {
                                let next_idx = enabled_brokers
                                    .iter()
                                    .position(|broker| *broker == self.primary_broker)
                                    .map(|idx| (idx + 1) % enabled_brokers.len())
                                    .unwrap_or(0);
                                let next = enabled_brokers[next_idx];
                                if next != self.primary_broker {
                                    self.primary_broker = next;
                                    // Routing follows the primary immediately; the
                                    // per-trade Broker combo can still override.
                                    self.order_broker = next;
                                    // Flip the equity data-merge trusted lane too
                                    // (ADR-126): primary defines the price scale,
                                    // the other broker becomes gap-fill assist.
                                    set_chart_merge_primary_broker(next);
                                    let assists = self.assist_brokers();
                                    let assist_str = if assists.is_empty() {
                                        "none".to_string()
                                    } else {
                                        assists
                                            .iter()
                                            .map(|broker| broker.label())
                                            .collect::<Vec<_>>()
                                            .join(", ")
                                    };
                                    self.log.push_back(LogEntry::info(format!(
                                        "Primary broker → {} (assist: {})",
                                        next.label(),
                                        assist_str
                                    )));
                                    self.save_session();
                                }
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
