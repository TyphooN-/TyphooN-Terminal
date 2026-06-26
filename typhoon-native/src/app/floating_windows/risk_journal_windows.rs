use super::*;

impl TyphooNApp {
    pub(super) fn render_risk_journal_windows(&mut self, ctx: &egui::Context) {
        // Risk-of-Ruin Calculator
        if self.show_risk_ruin {
            egui::Window::new("Risk-of-Ruin Calculator")
                .open(&mut self.show_risk_ruin)
                .resizable(true)
                .default_size([600.0, 450.0])
                .show(ctx, |ui| {
                    let rr_green = egui::Color32::from_rgb(46, 204, 113);
                    let rr_red = egui::Color32::from_rgb(231, 76, 60);
                    let rr_gold = egui::Color32::from_rgb(241, 196, 15);
                    let rr_dim = egui::Color32::from_rgb(100, 100, 120);

                    ui.label(egui::RichText::new("Monte Carlo Equity Path Simulation").strong());
                    ui.label(
                        egui::RichText::new(
                            "Simulate 10,000 trade sequences to estimate probability of ruin",
                        )
                        .color(rr_dim)
                        .small(),
                    );

                    // Input parameters
                    egui::Grid::new("ruin_params")
                        .num_columns(4)
                        .show(ui, |ui| {
                            ui.label("Win Rate %:");
                            ui.add(
                                egui::TextEdit::singleline(&mut self.ruin_win_rate)
                                    .desired_width(60.0),
                            );
                            ui.label("Avg Win $:");
                            ui.add(
                                egui::TextEdit::singleline(&mut self.ruin_avg_win)
                                    .desired_width(60.0),
                            );
                            ui.end_row();
                            ui.label("Avg Loss $:");
                            ui.add(
                                egui::TextEdit::singleline(&mut self.ruin_avg_loss)
                                    .desired_width(60.0),
                            );
                            ui.label("Risk %:");
                            ui.add(
                                egui::TextEdit::singleline(&mut self.ruin_risk_pct)
                                    .desired_width(60.0),
                            );
                            ui.end_row();
                        });

                    ui.horizontal(|ui| {
                        if ui
                            .button(
                                egui::RichText::new("Run 10,000 Simulations")
                                    .color(rr_green)
                                    .strong(),
                            )
                            .clicked()
                        {
                            let wr = self.ruin_win_rate.parse::<f64>().unwrap_or(55.0) / 100.0;
                            let avg_win = self.ruin_avg_win.parse::<f64>().unwrap_or(200.0);
                            let avg_loss = self.ruin_avg_loss.parse::<f64>().unwrap_or(150.0);
                            let _risk_pct = self.ruin_risk_pct.parse::<f64>().unwrap_or(2.0);

                            // CPU Monte Carlo (fast enough for 10K × 500 trades)
                            use std::collections::hash_map::DefaultHasher;
                            use std::hash::{Hash, Hasher};
                            let mut results = Vec::with_capacity(10000);
                            let starting_equity = 100000.0_f64;
                            for sim in 0..10000_u64 {
                                let mut equity = starting_equity;
                                let mut h = DefaultHasher::new();
                                sim.hash(&mut h);
                                let mut seed = h.finish();
                                for _ in 0..500 {
                                    // LCG random
                                    seed = seed
                                        .wrapping_mul(6364136223846793005)
                                        .wrapping_add(1442695040888963407);
                                    let r = (seed >> 33) as f64 / (u32::MAX as f64);
                                    if r < wr {
                                        equity += avg_win;
                                    } else {
                                        equity -= avg_loss;
                                    }
                                    if equity <= 0.0 {
                                        equity = 0.0;
                                        break;
                                    }
                                }
                                results.push(equity as f32);
                            }
                            results.sort_by(|a, b| {
                                a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
                            });
                            self.ruin_results = results;
                        }
                        // Auto-fill from journal
                        if !self.journal_entries.is_empty() {
                            if ui.small_button("Fill from Journal").clicked() {
                                let closed: Vec<_> = self
                                    .journal_entries
                                    .iter()
                                    .filter(|e| e.pnl.is_some())
                                    .collect();
                                if !closed.is_empty() {
                                    let wins: Vec<f64> = closed
                                        .iter()
                                        .filter_map(|e| e.pnl.filter(|&p| p > 0.0))
                                        .collect();
                                    let losses: Vec<f64> = closed
                                        .iter()
                                        .filter_map(|e| e.pnl.filter(|&p| p < 0.0).map(|p| p.abs()))
                                        .collect();
                                    let wr = wins.len() as f64 / closed.len() as f64 * 100.0;
                                    let avg_w = if wins.is_empty() {
                                        0.0
                                    } else {
                                        wins.iter().sum::<f64>() / wins.len() as f64
                                    };
                                    let avg_l = if losses.is_empty() {
                                        0.0
                                    } else {
                                        losses.iter().sum::<f64>() / losses.len() as f64
                                    };
                                    self.ruin_win_rate = format!("{:.1}", wr);
                                    self.ruin_avg_win = format!("{:.0}", avg_w);
                                    self.ruin_avg_loss = format!("{:.0}", avg_l);
                                }
                            }
                        }
                    });

                    if !self.ruin_results.is_empty() {
                        ui.separator();
                        let n = self.ruin_results.len();
                        let ruined = self.ruin_results.iter().filter(|&&e| e <= 0.0).count();
                        let ruin_pct = ruined as f64 / n as f64 * 100.0;
                        let median = self.ruin_results[n / 2];
                        let p5 = self.ruin_results[n * 5 / 100];
                        let p95 = self.ruin_results[n * 95 / 100];
                        let best = self.ruin_results.last().copied().unwrap_or(0.0);
                        let worst = self.ruin_results.first().copied().unwrap_or(0.0);

                        // Summary metrics
                        egui::Grid::new("ruin_metrics")
                            .num_columns(4)
                            .show(ui, |ui| {
                                let rc = if ruin_pct > 10.0 {
                                    rr_red
                                } else if ruin_pct > 1.0 {
                                    rr_gold
                                } else {
                                    rr_green
                                };
                                ui.label(egui::RichText::new("Prob of Ruin:").color(rr_dim));
                                ui.label(
                                    egui::RichText::new(format!("{:.2}%", ruin_pct))
                                        .color(rc)
                                        .strong(),
                                );
                                ui.label(egui::RichText::new("Median:").color(rr_dim));
                                ui.label(
                                    egui::RichText::new(format!("${:.0}", median)).color(rr_green),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("5th %ile:").color(rr_dim));
                                let p5c = if p5 < 100000.0 { rr_red } else { rr_dim };
                                ui.label(egui::RichText::new(format!("${:.0}", p5)).color(p5c));
                                ui.label(egui::RichText::new("95th %ile:").color(rr_dim));
                                ui.label(
                                    egui::RichText::new(format!("${:.0}", p95)).color(rr_green),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Worst:").color(rr_dim));
                                ui.label(
                                    egui::RichText::new(format!("${:.0}", worst)).color(rr_red),
                                );
                                ui.label(egui::RichText::new("Best:").color(rr_dim));
                                ui.label(
                                    egui::RichText::new(format!("${:.0}", best)).color(rr_green),
                                );
                                ui.end_row();
                            });

                        // Distribution chart (percentile buckets as line)
                        {
                            let pts: PlotPoints = PlotPoints::new(
                                (0..100)
                                    .map(|i| {
                                        let idx = i * n / 100;
                                        [i as f64, self.ruin_results[idx] as f64]
                                    })
                                    .collect(),
                            );
                            let c = if ruin_pct > 10.0 { rr_red } else { rr_green };
                            let line = Line::new("Equity Distribution", pts).color(c).width(1.5);
                            Plot::new("ruin_dist")
                                .height(200.0)
                                .allow_drag(false)
                                .allow_zoom(false)
                                .allow_scroll(false)
                                .show_axes([true, true])
                                .x_axis_label("Percentile")
                                .y_axis_label("Final Equity")
                                .show(ui, |plot_ui| {
                                    plot_ui.line(line);
                                    // Starting equity reference line
                                    let ref_pts =
                                        PlotPoints::new(vec![[0.0, 100000.0], [100.0, 100000.0]]);
                                    plot_ui.line(
                                        Line::new("Starting", ref_pts).color(rr_gold).width(1.0),
                                    );
                                });
                        }
                    }
                });
        }

        // Alert Builder + Alert Checker
        if self.show_alert_builder {
            egui::Window::new("Alert Builder")
                .open(&mut self.show_alert_builder)
                .resizable(true)
                .default_size([600.0, 400.0])
                .show(ctx, |ui| {
                    let al_green = egui::Color32::from_rgb(46, 204, 113);
                    let al_red = egui::Color32::from_rgb(231, 76, 60);
                    let al_gold = egui::Color32::from_rgb(241, 196, 15);
                    let al_cyan = egui::Color32::from_rgb(26, 188, 156);
                    let al_dim = egui::Color32::from_rgb(100, 100, 120);

                    // New alert form
                    ui.label(egui::RichText::new("Create Alert").strong());
                    ui.horizontal(|ui| {
                        ui.label("Symbol:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.alert_symbol).desired_width(80.0),
                        );
                        ui.label("Indicator:");
                        egui::ComboBox::from_id_salt("alert_ind")
                            .selected_text(ALERT_INDICATORS[self.alert_indicator])
                            .width(100.0)
                            .show_ui(ui, |ui| {
                                for (i, name) in ALERT_INDICATORS.iter().enumerate() {
                                    ui.selectable_value(&mut self.alert_indicator, i, *name);
                                }
                            });
                    });
                    ui.horizontal(|ui| {
                        ui.label("Condition:");
                        egui::ComboBox::from_id_salt("alert_cond")
                            .selected_text(ALERT_CONDITIONS[self.alert_condition])
                            .width(130.0)
                            .show_ui(ui, |ui| {
                                for (i, name) in ALERT_CONDITIONS.iter().enumerate() {
                                    ui.selectable_value(&mut self.alert_condition, i, *name);
                                }
                            });
                        ui.label("Threshold:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.alert_threshold)
                                .desired_width(80.0),
                        );
                        if ui
                            .button(egui::RichText::new("+ Add").color(al_green))
                            .clicked()
                        {
                            if let Ok(thresh) = self.alert_threshold.parse::<f64>() {
                                let tf = self
                                    .charts
                                    .get(self.active_tab)
                                    .map(|c| c.timeframe.label().to_string())
                                    .unwrap_or("H4".into());
                                let ind = ALERT_INDICATORS[self.alert_indicator].to_string();
                                let cond = ALERT_CONDITIONS[self.alert_condition].to_string();
                                let key = format!(
                                    "{}:{}:{}:{}:{:.4}",
                                    self.alert_symbol, tf, ind, cond, thresh
                                );
                                if self.indicator_alerts_set.insert(key) {
                                    self.indicator_alerts.push(IndicatorAlert {
                                        symbol: self.alert_symbol.clone(),
                                        timeframe: tf,
                                        indicator: ind,
                                        condition: cond,
                                        threshold: thresh,
                                        active: true,
                                        triggered: false,
                                        last_value: None,
                                    });
                                }
                                self.log.push_back(LogEntry::info(format!(
                                    "Alert: {} {} {} {}",
                                    self.alert_symbol,
                                    ALERT_INDICATORS[self.alert_indicator],
                                    ALERT_CONDITIONS[self.alert_condition],
                                    thresh
                                )));
                            }
                        }
                    });
                    ui.separator();

                    // Active alerts list
                    ui.label(
                        egui::RichText::new(format!(
                            "Active Alerts ({})",
                            self.indicator_alerts.len()
                        ))
                        .strong(),
                    );
                    let mut remove_idx: Option<usize> = None;
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .show(ui, |ui| {
                            for (idx, alert) in self.indicator_alerts.iter_mut().enumerate() {
                                ui.horizontal(|ui| {
                                    let status_c = if alert.triggered {
                                        al_red
                                    } else if alert.active {
                                        al_green
                                    } else {
                                        al_dim
                                    };
                                    let status_t = if alert.triggered {
                                        "TRIGGERED"
                                    } else if alert.active {
                                        "ACTIVE"
                                    } else {
                                        "OFF"
                                    };
                                    ui.label(
                                        egui::RichText::new(status_t)
                                            .color(status_c)
                                            .small()
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new(&alert.symbol)
                                            .color(al_cyan)
                                            .small()
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{} {} {:.2}",
                                            alert.indicator, alert.condition, alert.threshold
                                        ))
                                        .small(),
                                    );
                                    ui.label(
                                        egui::RichText::new(&alert.timeframe).color(al_dim).small(),
                                    );
                                    if let Some(v) = alert.last_value {
                                        ui.label(
                                            egui::RichText::new(format!("= {:.2}", v))
                                                .color(al_gold)
                                                .small(),
                                        );
                                    }
                                    ui.checkbox(&mut alert.active, "");
                                    if alert.triggered {
                                        if ui.small_button("Reset").clicked() {
                                            alert.triggered = false;
                                        }
                                    }
                                    if ui.small_button("x").clicked() {
                                        remove_idx = Some(idx);
                                    }
                                });
                            }
                        });
                    if let Some(idx) = remove_idx {
                        if idx < self.indicator_alerts.len() {
                            let a = &self.indicator_alerts[idx];
                            let key = format!(
                                "{}:{}:{}:{}:{:.4}",
                                a.symbol, a.timeframe, a.indicator, a.condition, a.threshold
                            );
                            self.indicator_alerts_set.remove(&key);
                        }
                        self.indicator_alerts.remove(idx);
                    }
                });
        }

        // Check indicator alerts against current chart data (every frame, cheap)
        {
            let chart_data: Option<(&str, &str, f64, f64, f64, f64, f64)> =
                self.charts.get(self.active_tab).and_then(|c| {
                    let n = c.bars.len();
                    if n < 2 {
                        return None;
                    }
                    let close = c.bars[n - 1].close;
                    let rsi = c.rsi.get(n - 1).and_then(|v| *v).unwrap_or(50.0);
                    let fisher = c.fisher.get(n - 1).and_then(|v| *v).unwrap_or(0.0);
                    let adx = c.adx.get(n - 1).and_then(|v| *v).unwrap_or(0.0);
                    let atr = c.atr.get(n - 1).and_then(|v| *v).unwrap_or(0.0);
                    Some((
                        &*c.symbol,
                        c.timeframe.label(),
                        close,
                        rsi,
                        fisher,
                        adx,
                        atr,
                    ))
                });

            if let Some((sym, _tf, close, rsi, fisher, adx, atr)) = chart_data {
                for alert in self.indicator_alerts.iter_mut() {
                    if !alert.active || alert.triggered {
                        continue;
                    }
                    // Only check if symbol matches current chart
                    if !sym.contains(&alert.symbol) {
                        continue;
                    }

                    let current_val = match alert.indicator.as_str() {
                        "Price" => close,
                        "RSI" => rsi,
                        "Fisher" => fisher,
                        "ADX" => adx,
                        "ATR" => atr,
                        _ => continue,
                    };

                    let prev_val = alert.last_value.unwrap_or(current_val);
                    let triggered = match alert.condition.as_str() {
                        "crosses above" => {
                            prev_val <= alert.threshold && current_val > alert.threshold
                        }
                        "crosses below" => {
                            prev_val >= alert.threshold && current_val < alert.threshold
                        }
                        "greater than" => current_val > alert.threshold,
                        "less than" => current_val < alert.threshold,
                        _ => false,
                    };

                    alert.last_value = Some(current_val);
                    if triggered {
                        alert.triggered = true;
                        let msg = format!(
                            "ALERT: {} {} {} {} (value: {:.2})",
                            alert.symbol,
                            alert.indicator,
                            alert.condition,
                            alert.threshold,
                            current_val
                        );
                        // Surface to the top-bar breach badge — trader cannot miss this.
                        self.alert_breach_count = self.alert_breach_count.saturating_add(1);
                        self.alert_last_breach_ts = chrono::Utc::now().timestamp();
                        self.alert_last_breach_msg = msg.clone();
                        // OS-level attention request: taskbar icon flashes, dock bounces on macOS,
                        // title bar flashes on Windows. No new crate dep — egui 0.34 supports this.
                        ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
                            egui::UserAttentionType::Critical,
                        ));
                        self.log.push_back(LogEntry::alert(&msg));
                        // ADR-094: Toast for triggered alerts
                        self.toasts.push(Toast {
                            message: msg.clone(),
                            color: egui::Color32::from_rgb(255, 165, 0),
                            created: std::time::Instant::now(),
                            duration: std::time::Duration::from_secs(10),
                            dismissable: true,
                            dismissed: false,
                        });
                        // Send notification if any provider configured
                        if !self.discord_webhook.is_empty()
                            || !self.ntfy_topic.is_empty()
                            || (!self.pushover_token.is_empty() && !self.pushover_user.is_empty())
                        {
                            let _ = self.broker_tx.send(BrokerCmd::SendNotification {
                                discord_webhook: self.discord_webhook.clone(),
                                pushover_token: self.pushover_token.clone(),
                                pushover_user: self.pushover_user.clone(),
                                ntfy_topic: self.ntfy_topic.clone(),
                                message: msg,
                            });
                        }
                    }
                }
            }
        }

        // Multi-Dimensional Outlier Scanner
        if self.show_outliers {
            let outlier_scope_label = self.broker_scope_label().to_string();
            // PERF: read from per-frame cache
            let outlier_scoped_fund = self.cached_scoped_fundamentals.clone();
            let mut pending_action = SymbolAction::None;
            // UX7: pre-fetch sparklines for top outlier symbols
            let mut outlier_syms: Vec<String> = self
                .outliers
                .iter()
                .take(200)
                .map(|o| o.symbol.clone())
                .collect();
            outlier_syms.extend(
                self.multi_outliers
                    .iter()
                    .take(200)
                    .map(|o| o.symbol.clone()),
            );
            let mut outlier_sparklines: std::collections::HashMap<
                String,
                std::sync::Arc<Vec<f64>>,
            > = std::collections::HashMap::new();
            for sym in &outlier_syms {
                let closes = self.get_sparkline(sym);
                if !closes.is_empty() {
                    outlier_sparklines.insert(sym.to_uppercase(), closes);
                }
            }
            egui::Window::new("Outlier Scanner")
                        .open(&mut self.show_outliers)
                        .resizable(true)
                        .default_size([900.0, 600.0])
                        .show(ctx, |ui| {
                            let ol_high = egui::Color32::from_rgb(231, 76, 60);
                            let ol_med = egui::Color32::from_rgb(241, 196, 15);
                            let ol_green = egui::Color32::from_rgb(46, 204, 113);
                            let ol_cyan = egui::Color32::from_rgb(26, 188, 156);
                            let ol_dim = egui::Color32::from_rgb(100, 100, 120);

                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(format!("Outlier Analysis [{}] — {} outliers, {} sectors", outlier_scope_label, self.outliers.len(), self.sector_stats.len())).strong());
                                if ui.small_button("Refresh").clicked() {
                                    // Re-run with scope filter (respects SCOPE command)
                                    if let Some(ref cache) = self.cache {
                                        if let Some(_conn) = cache.try_connection() {
                                            let mut data: Vec<(String, String, String, f64)> = Vec::new();
                                            for f in &outlier_scoped_fund {
                                                let sector = if f.sector.is_empty() { "Unknown".to_string() } else { f.sector.clone() };
                                                let industry = if f.industry.is_empty() { sector.clone() } else { f.industry.clone() };
                                                if let Some(mc) = f.market_cap { if mc > 0.0 { data.push((f.symbol.clone(), sector, industry, mc)); } }
                                            }
                                            if data.len() >= 10 {
                                                let (o, s) = typhoon_engine::core::var::detect_outliers(&data, 1.5);
                                                self.outliers = o; self.sector_stats = s;
                                            }
                                        }
                                    }
                                }
                            });
                            ui.separator();

                            egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                                // Multi-dimensional anomaly table (VaR + EV + ATR + SEC)
                                if !self.multi_outliers.is_empty() {
                                    let extreme_count = self.multi_outliers.iter().filter(|o| o.dimensions_flagged >= 3).count();
                                    let high_count = self.multi_outliers.iter().filter(|o| o.dimensions_flagged == 2).count();
                                    ui.label(egui::RichText::new(format!("Multi-Signal Anomaly Scanner — {} EXTREME, {} HIGH, {} total",
                                        extreme_count, high_count, self.multi_outliers.len())).strong());
                                    ui.label(egui::RichText::new("Score = sum of |z-scores| across flagged dimensions. Higher = more anomalous.").color(ol_dim).small());
                                    ui.label(egui::RichText::new("Dims: P/E (risk) + MCap/EV (valuation) + Short Ratio (volatility) + SEC filings+insider trades (activity)").color(ol_dim).small());
                                    ui.add_space(4.0);
                                    // Sort outliers. Column indices:
                                    //   0 Symbol, 1 Sector, 2 Industry, 3 Score, 4 Dims, 5 Tier,
                                    //   6 P/E z, 7 EV z, 8 Short z, 9 SEC z
                                    let mut sorted_outliers: Vec<&_> = self.multi_outliers.iter().collect();
                                    match self.outlier_sort.column {
                                        0 => sorted_outliers.sort_by(|a, b| a.symbol.cmp(&b.symbol)),
                                        1 => sorted_outliers.sort_by(|a, b| a.sector.cmp(&b.sector).then_with(|| a.industry.cmp(&b.industry))),
                                        2 => sorted_outliers.sort_by(|a, b| a.industry.cmp(&b.industry).then_with(|| a.symbol.cmp(&b.symbol))),
                                        3 => sorted_outliers.sort_by(|a, b| a.composite_score.partial_cmp(&b.composite_score).unwrap_or(std::cmp::Ordering::Equal)),
                                        4 => sorted_outliers.sort_by(|a, b| a.dimensions_flagged.cmp(&b.dimensions_flagged)),
                                        5 => sorted_outliers.sort_by(|a, b| a.tier.cmp(&b.tier)),
                                        6 => sorted_outliers.sort_by(|a, b| a.var_z.abs().partial_cmp(&b.var_z.abs()).unwrap_or(std::cmp::Ordering::Equal)),
                                        7 => sorted_outliers.sort_by(|a, b| a.ev_z.abs().partial_cmp(&b.ev_z.abs()).unwrap_or(std::cmp::Ordering::Equal)),
                                        8 => sorted_outliers.sort_by(|a, b| a.atr_z.abs().partial_cmp(&b.atr_z.abs()).unwrap_or(std::cmp::Ordering::Equal)),
                                        9 => sorted_outliers.sort_by(|a, b| a.sec_z.abs().partial_cmp(&b.sec_z.abs()).unwrap_or(std::cmp::Ordering::Equal)),
                                        _ => {}
                                    }
                                    if !self.outlier_sort.ascending { sorted_outliers.reverse(); }

                                    egui::Grid::new("multi_outlier_grid").striped(true).num_columns(11).min_col_width(50.0).show(ui, |ui| {
                                        if SortState::header(ui, "Symbol", 0, &self.outlier_sort) { self.outlier_sort.toggle(0); }
                                        ui.label(egui::RichText::new("30d").color(ol_dim).small());
                                        if SortState::header(ui, "Sector", 1, &self.outlier_sort) { self.outlier_sort.toggle(1); }
                                        if SortState::header(ui, "Industry", 2, &self.outlier_sort) { self.outlier_sort.toggle(2); }
                                        if SortState::header(ui, "Score", 3, &self.outlier_sort) { self.outlier_sort.toggle(3); }
                                        if SortState::header(ui, "Dims", 4, &self.outlier_sort) { self.outlier_sort.toggle(4); }
                                        if SortState::header(ui, "Tier", 5, &self.outlier_sort) { self.outlier_sort.toggle(5); }
                                        if SortState::header(ui, "P/E z", 6, &self.outlier_sort) { self.outlier_sort.toggle(6); }
                                        if SortState::header(ui, "EV z", 7, &self.outlier_sort) { self.outlier_sort.toggle(7); }
                                        if SortState::header(ui, "Short z", 8, &self.outlier_sort) { self.outlier_sort.toggle(8); }
                                        if SortState::header(ui, "SEC z", 9, &self.outlier_sort) { self.outlier_sort.toggle(9); }
                                        ui.end_row();

                                        for o in sorted_outliers.iter().take(200) {
                                            let tier_c = match o.tier.as_str() {
                                                "EXTREME" => ol_high, "HIGH" => ol_med, _ => ol_green
                                            };
                                            let z_color = |z: f64| -> egui::Color32 {
                                                if z.abs() > 2.0 { ol_high } else if z.abs() > 1.5 { ol_med } else { ol_dim }
                                            };
                                            ui.horizontal(|ui| {
                                                let (_, action) = symbol_label_with_menu(ui, &o.symbol,
                                                    egui::RichText::new(&o.symbol).small().strong().color(egui::Color32::WHITE));
                                                if !matches!(action, SymbolAction::None) { pending_action = action; }
                                                if ui.small_button(egui::RichText::new("+").small()).on_hover_text("Open new chart").clicked() {
                                                    pending_action = SymbolAction::OpenChart(o.symbol.clone());
                                                }
                                            });
                                            // UX7: Sparkline column (o.symbol is already uppercase).
                                            if let Some(closes) = outlier_sparklines.get(o.symbol.as_str()) {
                                                draw_inline_sparkline(ui, closes, 50.0, 12.0);
                                            } else {
                                                ui.label(egui::RichText::new("—").color(ol_dim).small());
                                            }
                                            ui.label(egui::RichText::new(&o.sector).small().color(ol_cyan));
                                            ui.label(egui::RichText::new(&o.industry).small());
                                            ui.label(egui::RichText::new(format!("{:.1}", o.composite_score)).small().color(tier_c).strong());
                                            ui.label(egui::RichText::new(format!("{}/4", o.dimensions_flagged)).small().color(tier_c));
                                            ui.label(egui::RichText::new(&o.tier).small().color(tier_c));
                                            ui.label(egui::RichText::new(format!("{:+.1}", o.var_z)).small().color(z_color(o.var_z)));
                                            ui.label(egui::RichText::new(format!("{:+.1}", o.ev_z)).small().color(z_color(o.ev_z)));
                                            ui.label(egui::RichText::new(format!("{:+.1}", o.atr_z)).small().color(z_color(o.atr_z)));
                                            ui.label(egui::RichText::new(format!("{:+.1}", o.sec_z)).small().color(z_color(o.sec_z)));
                                            ui.end_row();
                                        }
                                    });
                                    ui.add_space(8.0);
                                    ui.separator();
                                }

                                // Sector summary (single-dimension)
                                if !self.sector_stats.is_empty() {
                                    ui.label(egui::RichText::new("Sector Statistics").small().strong());
                                    egui::Grid::new("sector_stats_grid").striped(true).num_columns(6).min_col_width(60.0).show(ui, |ui| {
                                        ui.label(egui::RichText::new("Sector").color(ol_dim).small());
                                        ui.label(egui::RichText::new("Count").color(ol_dim).small());
                                        ui.label(egui::RichText::new("Median").color(ol_dim).small());
                                        ui.label(egui::RichText::new("IQR").color(ol_dim).small());
                                        ui.label(egui::RichText::new("Bounds").color(ol_dim).small());
                                        ui.label(egui::RichText::new("Outliers").color(ol_dim).small());
                                        ui.end_row();
                                        for s in &self.sector_stats {
                                            ui.label(egui::RichText::new(&s.sector).small().color(ol_cyan));
                                            ui.label(format!("{}", s.count));
                                            ui.label(typhoon_engine::core::fundamentals::format_large_number(s.median));
                                            ui.label(typhoon_engine::core::fundamentals::format_large_number(s.iqr));
                                            ui.label(egui::RichText::new(format!("{} – {}",
                                                typhoon_engine::core::fundamentals::format_large_number(s.lower_bound),
                                                typhoon_engine::core::fundamentals::format_large_number(s.upper_bound)
                                            )).color(ol_dim).small());
                                            let oc = if s.outlier_count > 3 { ol_high } else if s.outlier_count > 0 { ol_med } else { ol_green };
                                            ui.label(egui::RichText::new(format!("{}", s.outlier_count)).color(oc));
                                            ui.end_row();
                                        }
                                    });
                                    ui.add_space(8.0);
                                }

                                // Outlier table (single-metric) — click headers to sort
                                if !self.outliers.is_empty() {
                                    ui.label(egui::RichText::new("Outliers (click headers to sort)").small().strong());
                                    // Sort outliers per header state. Columns:
                                    //   0 Symbol, 1 Sector, 2 Industry, 3 Value, 4 Median,
                                    //   5 Tier, 6 Z-Score (|z|), 7 Direction
                                    // ("30d" sparkline is display-only between Symbol and Sector — no sort.)
                                    let mut sorted_single: Vec<&_> = self.outliers.iter().collect();
                                    match self.outlier_single_sort.column {
                                        0 => sorted_single.sort_by(|a, b| a.symbol.cmp(&b.symbol)),
                                        1 => sorted_single.sort_by(|a, b| a.sector.cmp(&b.sector).then_with(|| a.industry.cmp(&b.industry))),
                                        2 => sorted_single.sort_by(|a, b| a.industry.cmp(&b.industry).then_with(|| a.symbol.cmp(&b.symbol))),
                                        3 => sorted_single.sort_by(|a, b| a.metric.partial_cmp(&b.metric).unwrap_or(std::cmp::Ordering::Equal)),
                                        4 => sorted_single.sort_by(|a, b| a.sector_median.partial_cmp(&b.sector_median).unwrap_or(std::cmp::Ordering::Equal)),
                                        5 => sorted_single.sort_by(|a, b| a.tier.cmp(&b.tier)),
                                        6 => sorted_single.sort_by(|a, b| a.z_score.abs().partial_cmp(&b.z_score.abs()).unwrap_or(std::cmp::Ordering::Equal)),
                                        7 => sorted_single.sort_by(|a, b| a.direction.cmp(&b.direction)),
                                        _ => {}
                                    }
                                    if !self.outlier_single_sort.ascending { sorted_single.reverse(); }

                                    egui::Grid::new("outliers_grid").striped(true).num_columns(9).min_col_width(50.0).show(ui, |ui| {
                                        if SortState::header(ui, "Symbol", 0, &self.outlier_single_sort) { self.outlier_single_sort.toggle(0); }
                                        ui.label(egui::RichText::new("30d").color(ol_dim).small());
                                        if SortState::header(ui, "Sector", 1, &self.outlier_single_sort) { self.outlier_single_sort.toggle(1); }
                                        if SortState::header(ui, "Industry", 2, &self.outlier_single_sort) { self.outlier_single_sort.toggle(2); }
                                        if SortState::header(ui, "Value", 3, &self.outlier_single_sort) { self.outlier_single_sort.toggle(3); }
                                        if SortState::header(ui, "Median", 4, &self.outlier_single_sort) { self.outlier_single_sort.toggle(4); }
                                        if SortState::header(ui, "Tier", 5, &self.outlier_single_sort) { self.outlier_single_sort.toggle(5); }
                                        if SortState::header(ui, "Z-Score", 6, &self.outlier_single_sort) { self.outlier_single_sort.toggle(6); }
                                        if SortState::header(ui, "Dir", 7, &self.outlier_single_sort) { self.outlier_single_sort.toggle(7); }
                                        ui.end_row();
                                        let mut scrolled = false;
                                        for o in &sorted_single {
                                            let mut sym_resp_opt: Option<egui::Response> = None;
                                            ui.horizontal(|ui| {
                                                let (sym_resp, action) = symbol_label_with_menu(ui, &o.symbol,
                                                    egui::RichText::new(&o.symbol).strong().color(ol_cyan));
                                                if !matches!(action, SymbolAction::None) {
                                                    pending_action = action;
                                                }
                                                if ui.small_button(egui::RichText::new("+").small()).on_hover_text("Open new chart").clicked() {
                                                    pending_action = SymbolAction::OpenChart(o.symbol.clone());
                                                }
                                                sym_resp_opt = Some(sym_resp);
                                            });
                                            let sym_resp = sym_resp_opt.unwrap();
                                            // UX6: Auto-scroll to first EXTREME tier outlier on pending flag
                                            if self.outlier_scroll_pending && !scrolled && o.tier == "EXTREME" {
                                                sym_resp.scroll_to_me(Some(egui::Align::Center));
                                                scrolled = true;
                                            }
                                            // UX7: Sparkline column (o.symbol is already uppercase).
                                            if let Some(closes) = outlier_sparklines.get(o.symbol.as_str()) {
                                                draw_inline_sparkline(ui, closes, 50.0, 12.0);
                                            } else {
                                                ui.label(egui::RichText::new("—").color(ol_dim).small());
                                            }
                                            ui.label(egui::RichText::new(&o.sector).small().color(ol_cyan));
                                            ui.label(egui::RichText::new(&o.industry).small());
                                            ui.label(typhoon_engine::core::fundamentals::format_large_number(o.metric));
                                            ui.label(typhoon_engine::core::fundamentals::format_large_number(o.sector_median));
                                            let tc = match o.tier.as_str() { "EXTREME" => ol_high, "HIGH" => ol_med, _ => ol_dim };
                                            ui.label(egui::RichText::new(&o.tier).color(tc).small());
                                            let zc = if o.z_score.abs() > 3.0 { ol_high } else if o.z_score.abs() > 2.0 { ol_med } else { ol_dim };
                                            ui.label(egui::RichText::new(format!("{:.2}", o.z_score)).color(zc));
                                            let dc = if o.direction == "high" { ol_green } else { ol_high };
                                            ui.label(egui::RichText::new(&o.direction).color(dc).small());
                                            ui.end_row();
                                        }
                                        if scrolled || self.outlier_scroll_pending {
                                            self.outlier_scroll_pending = false;
                                        }
                                    });
                                } else {
                                    ui.label(egui::RichText::new("No outliers detected. Run EVSCRAPE first, then OUTLIERS.").color(ol_dim));
                                }
                            });
                        });
            // Apply deferred symbol context menu action (after window borrow released)
            self.apply_symbol_action(pending_action);
        }

        // Trade Journal
        if self.show_journal {
            egui::Window::new("Trade Journal")
                .open(&mut self.show_journal)
                .resizable(true)
                .default_size([700.0, 500.0])
                .show(ctx, |ui| {
                    let j_green = egui::Color32::from_rgb(46, 204, 113);
                    let j_red = egui::Color32::from_rgb(231, 76, 60);
                    let j_dim = egui::Color32::from_rgb(100, 100, 120);
                    let j_cyan = egui::Color32::from_rgb(26, 188, 156);

                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Trade Journal").strong());
                        ui.label(
                            egui::RichText::new(format!("{} entries", self.journal_entries.len()))
                                .color(j_dim)
                                .small(),
                        );
                        // Quick add from current chart
                        if ui
                            .small_button(egui::RichText::new("+ Add Trade").color(j_green))
                            .clicked()
                        {
                            let sym = self
                                .charts
                                .get(self.active_tab)
                                .map(|c| c.symbol.clone())
                                .unwrap_or_default();
                            let price = self
                                .charts
                                .get(self.active_tab)
                                .and_then(|c| c.bars.last().map(|b| b.close))
                                .unwrap_or(0.0);
                            self.journal_entries.push(JournalEntry {
                                timestamp: chrono::Utc::now().format("%Y-%m-%d %H:%M").to_string(),
                                symbol: sym,
                                side: "BUY".to_string(),
                                qty: 1.0,
                                entry_price: price,
                                exit_price: None,
                                pnl: None,
                                strategy: "NNFX".to_string(),
                                notes: String::new(),
                            });
                        }
                    });
                    ui.separator();

                    // Summary stats
                    if !self.journal_entries.is_empty() {
                        let total_pnl: f64 =
                            self.journal_entries.iter().filter_map(|e| e.pnl).sum();
                        let closed = self
                            .journal_entries
                            .iter()
                            .filter(|e| e.pnl.is_some())
                            .count();
                        let wins = self
                            .journal_entries
                            .iter()
                            .filter(|e| e.pnl.map(|p| p > 0.0).unwrap_or(false))
                            .count();
                        let wr = if closed > 0 {
                            wins as f64 / closed as f64 * 100.0
                        } else {
                            0.0
                        };
                        ui.horizontal(|ui| {
                            let pc = if total_pnl >= 0.0 { j_green } else { j_red };
                            ui.label(
                                egui::RichText::new(format!("P&L: ${:.0}", total_pnl))
                                    .color(pc)
                                    .strong(),
                            );
                            ui.label(
                                egui::RichText::new(format!("Closed: {}", closed))
                                    .color(j_dim)
                                    .small(),
                            );
                            let wc = if wr >= 50.0 { j_green } else { j_red };
                            ui.label(
                                egui::RichText::new(format!("Win: {:.0}%", wr))
                                    .color(wc)
                                    .small(),
                            );
                            ui.label(
                                egui::RichText::new(format!(
                                    "Open: {}",
                                    self.journal_entries.len() - closed
                                ))
                                .color(j_cyan)
                                .small(),
                            );
                        });
                        ui.separator();
                    }

                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .show(ui, |ui| {
                            let mut delete_idx: Option<usize> = None;
                            for (idx, entry) in self.journal_entries.iter_mut().enumerate() {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(&entry.timestamp).color(j_dim).small(),
                                    );
                                    let side_c = if entry.side == "BUY" { j_green } else { j_red };
                                    ui.label(
                                        egui::RichText::new(&entry.side)
                                            .color(side_c)
                                            .small()
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new(&entry.symbol)
                                            .color(j_cyan)
                                            .small()
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{:.2} @ {}",
                                            entry.qty,
                                            format_price(entry.entry_price)
                                        ))
                                        .small(),
                                    );
                                    if let Some(pnl) = entry.pnl {
                                        let pc = if pnl >= 0.0 { j_green } else { j_red };
                                        ui.label(
                                            egui::RichText::new(format!("${:.0}", pnl))
                                                .color(pc)
                                                .small()
                                                .strong(),
                                        );
                                    } else {
                                        ui.label(egui::RichText::new("open").color(j_cyan).small());
                                    }
                                    ui.label(
                                        egui::RichText::new(&entry.strategy).color(j_dim).small(),
                                    );
                                    if ui.small_button("x").clicked() {
                                        delete_idx = Some(idx);
                                    }
                                });
                                // Editable notes
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("  Notes:").color(j_dim).small());
                                    ui.add(
                                        egui::TextEdit::singleline(&mut entry.notes)
                                            .desired_width(400.0)
                                            .hint_text("Trade notes..."),
                                    );
                                });
                                // Close trade button
                                if entry.exit_price.is_none() {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new("  Exit:").color(j_dim).small(),
                                        );
                                        let mut exit_str = entry
                                            .exit_price
                                            .map(|p| format!("{:.4}", p))
                                            .unwrap_or_default();
                                        ui.add(
                                            egui::TextEdit::singleline(&mut exit_str)
                                                .desired_width(80.0)
                                                .hint_text("exit price"),
                                        );
                                        if let Ok(ep) = exit_str.parse::<f64>() {
                                            entry.exit_price = Some(ep);
                                            let pnl = if entry.side == "BUY" {
                                                (ep - entry.entry_price) * entry.qty
                                            } else {
                                                (entry.entry_price - ep) * entry.qty
                                            };
                                            entry.pnl = Some(pnl);
                                        }
                                    });
                                }
                                ui.add_space(2.0);
                            }
                            if let Some(idx) = delete_idx {
                                self.journal_entries.remove(idx);
                            }
                        });
                    // Cumulative P&L line chart
                    let closed_pnls: Vec<f64> =
                        self.journal_entries.iter().filter_map(|e| e.pnl).collect();
                    if closed_pnls.len() >= 2 {
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new("Cumulative P&L").strong());
                        let mut cum = 0.0_f64;
                        let pts: PlotPoints = PlotPoints::new(
                            closed_pnls
                                .iter()
                                .enumerate()
                                .map(|(i, &p)| {
                                    cum += p;
                                    [i as f64, cum]
                                })
                                .collect(),
                        );
                        let color = if cum >= 0.0 { UP } else { DOWN };
                        let line = Line::new("Cumulative P&L", pts).color(color).width(1.5);
                        Plot::new("journal_cum_pnl")
                            .height(100.0)
                            .allow_drag(false)
                            .allow_zoom(false)
                            .show(ui, |plot_ui| {
                                plot_ui.line(line);
                            });
                    }
                });
        }

        // Margin Monitor — wired to margin.rs functions
        if self.show_margin_monitor {
            egui::Window::new("Margin Monitor")
                        .open(&mut self.show_margin_monitor)
                        .resizable(true).default_size([450.0, 350.0])
                        .show(ctx, |ui| {
                            ui.heading("Margin Calculator");
                            ui.separator();
                            ui.horizontal(|ui| {
                                ui.label("Equity:"); ui.add(egui::TextEdit::singleline(&mut self.mm_equity).desired_width(100.0));
                                ui.label("Margin Used:"); ui.add(egui::TextEdit::singleline(&mut self.mm_margin).desired_width(100.0));
                            });
                            ui.horizontal(|ui| {
                                ui.label("Margin/Lot:"); ui.add(egui::TextEdit::singleline(&mut self.mm_margin_per_lot).desired_width(100.0));
                                ui.label("TRIM %:"); ui.add(egui::TextEdit::singleline(&mut self.mm_trim_pct).desired_width(60.0));
                            });
                            if ui.button("Calculate").clicked() {
                                let equity: f64 = self.mm_equity.replace(['$', ','], "").parse().unwrap_or(0.0);
                                let margin_used: f64 = self.mm_margin.replace(['$', ','], "").parse().unwrap_or(0.0);
                                let margin_per_lot: f64 = self.mm_margin_per_lot.replace(['$', ','], "").parse().unwrap_or(1000.0);
                                let trim_pct: f64 = self.mm_trim_pct.parse().unwrap_or(150.0);
                                if equity > 0.0 {
                                    let ml = margin::margin_level_pct(equity, margin_used);
                                    let free = margin::usable_margin(equity, margin_used, 10.0);
                                    let max_lots = margin::max_safe_lots(equity, margin_used, margin_per_lot, trim_pct);
                                    let urgency = margin::protect_urgency(ml, trim_pct);
                                    self.mm_result = format!(
                                        "Margin Level: {:.2}%\nFree Margin: ${:.2}\nMax Safe Lots: {}\nProtect Urgency: {:.2}",
                                        ml, free, max_lots, urgency
                                    );
                                }
                            }
                            if !self.mm_result.is_empty() {
                                ui.separator();
                                ui.label(egui::RichText::new(&self.mm_result).monospace().color(egui::Color32::from_rgb(200, 220, 255)));
                                // Visual margin level gauge
                                let equity: f64 = self.mm_equity.replace(['$', ','], "").parse().unwrap_or(0.0);
                                let margin_used: f64 = self.mm_margin.replace(['$', ','], "").parse().unwrap_or(0.0);
                                if margin_used > 0.0 && equity > 0.0 {
                                    let ml = equity / margin_used * 100.0;
                                    ui.add_space(6.0);
                                    ui.label(egui::RichText::new("Margin Level Gauge").strong());
                                    let bar_w = 360.0_f32;
                                    let bar_h = 22.0_f32;
                                    let (rect, _) = ui.allocate_exact_size(egui::vec2(bar_w, bar_h), egui::Sense::hover());
                                    let painter = ui.painter_at(rect);
                                    // Draw zones: red 0-100%, yellow 100-200%, green 200-400%
                                    let r_end = (bar_w * 0.25).min(bar_w);
                                    let y_end = (bar_w * 0.50).min(bar_w);
                                    painter.rect_filled(egui::Rect::from_min_max(rect.min, egui::pos2(rect.min.x + r_end, rect.max.y)), 0.0, egui::Color32::from_rgb(180, 40, 40));
                                    painter.rect_filled(egui::Rect::from_min_max(egui::pos2(rect.min.x + r_end, rect.min.y), egui::pos2(rect.min.x + y_end, rect.max.y)), 0.0, egui::Color32::from_rgb(200, 180, 40));
                                    painter.rect_filled(egui::Rect::from_min_max(egui::pos2(rect.min.x + y_end, rect.min.y), rect.max), 0.0, egui::Color32::from_rgb(40, 160, 60));
                                    // Marker for current level (clamped to 0-400%)
                                    let frac = (ml / 400.0).clamp(0.0, 1.0) as f32;
                                    let mx = rect.min.x + frac * bar_w;
                                    painter.rect_filled(egui::Rect::from_center_size(egui::pos2(mx, rect.center().y), egui::vec2(3.0, bar_h)), 0.0, egui::Color32::WHITE);
                                    painter.text(egui::pos2(mx, rect.min.y - 2.0), egui::Align2::CENTER_BOTTOM, format!("{:.0}%", ml), egui::FontId::proportional(10.0), egui::Color32::WHITE);
                                }
                            }
                        });
        }
    }
}
