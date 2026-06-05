use super::*;

impl TyphooNApp {
    pub(super) fn render_broker_darwin_windows(&mut self, ctx: &egui::Context) {
        self.render_connect_window(ctx, false);
        self.render_indicators_window(ctx);
        self.render_kraken_spot_sell_dialog(ctx);
        // ── Kraken Trade History Window ─────────────────────────────────────
        if self.show_kraken_trade_history {
            egui::Window::new("Kraken Trade History")
                .open(&mut self.show_kraken_trade_history)
                .default_size([900.0, 500.0])
                .max_size([900.0, 560.0])
                .resizable(true)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(format!("{} trades loaded", self.kraken_trades.len()));
                        if ui.button("Refresh").clicked() {
                            let _ = self.broker_tx.send(BrokerCmd::KrakenFetchTrades);
                        }
                        if ui.button("Open Orders").clicked() {
                            self.show_kraken_open_orders = true;
                            let _ = self.broker_tx.send(BrokerCmd::KrakenFetchOpenOrders);
                        }
                    });
                    ui.separator();

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        egui::Grid::new("kraken_trades_grid")
                            .striped(true)
                            .show(ui, |ui| {
                                // Header
                                ui.label(egui::RichText::new("Time").strong());
                                ui.label(egui::RichText::new("Pair").strong());
                                ui.label(egui::RichText::new("Side").strong());
                                ui.label(egui::RichText::new("Type").strong());
                                ui.label(egui::RichText::new("Price").strong());
                                ui.label(egui::RichText::new("Vol").strong());
                                ui.label(egui::RichText::new("Cost").strong());
                                ui.label(egui::RichText::new("Fee").strong());
                                ui.end_row();

                                for t in &self.kraken_trades {
                                    let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(
                                        t.time as i64,
                                        0,
                                    )
                                    .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                                    .unwrap_or_else(|| format!("{:.0}", t.time));

                                    ui.label(dt);
                                    ui.label(&t.pair);
                                    ui.colored_label(
                                        if t.side == "buy" {
                                            egui::Color32::from_rgb(46, 204, 113)
                                        } else {
                                            egui::Color32::from_rgb(231, 76, 60)
                                        },
                                        &t.side,
                                    );
                                    ui.label(&t.ordertype);
                                    ui.label(format!("{:.4}", t.price));
                                    ui.label(format!("{:.4}", t.vol));
                                    ui.label(format!("{:.2}", t.cost));
                                    ui.label(format!("{:.4}", t.fee));
                                    ui.end_row();
                                }
                            });
                    });
                });
        }

        // ── Kraken Open Orders Window ────────────────────────────────────────
        if self.show_kraken_open_orders {
            egui::Window::new("Kraken Open Orders")
                .open(&mut self.show_kraken_open_orders)
                .default_size([1000.0, 420.0])
                .max_size([1000.0, 560.0])
                .resizable(true)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(format!("{} open orders", self.kraken_open_orders.len()));
                        if ui.button("Refresh").clicked() {
                            let _ = self.broker_tx.send(BrokerCmd::KrakenFetchOpenOrders);
                        }
                        if ui.button("Trade History").clicked() {
                            self.show_kraken_trade_history = true;
                        }
                    });
                    ui.separator();

                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .show(ui, |ui| {
                            egui::Grid::new("kraken_open_orders_grid")
                                .striped(true)
                                .num_columns(11)
                                .show(ui, |ui| {
                                    ui.label(egui::RichText::new("Time").strong());
                                    ui.label(egui::RichText::new("Pair").strong());
                                    ui.label(egui::RichText::new("Side").strong());
                                    ui.label(egui::RichText::new("Type").strong());
                                    ui.label(egui::RichText::new("Price").strong());
                                    ui.label(egui::RichText::new("Vol").strong());
                                    ui.label(egui::RichText::new("Filled").strong());
                                    ui.label(egui::RichText::new("Remain").strong());
                                    ui.label(egui::RichText::new("Status").strong());
                                    ui.label(egui::RichText::new("TxID").strong());
                                    ui.label(egui::RichText::new("Action").strong());
                                    ui.end_row();

                                    for order in &self.kraken_open_orders {
                                        let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(
                                            order.opentm as i64,
                                            0,
                                        )
                                        .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                                        .unwrap_or_else(|| format!("{:.0}", order.opentm));
                                        let side_color = if order.r#type == "buy" {
                                            egui::Color32::from_rgb(46, 204, 113)
                                        } else {
                                            egui::Color32::from_rgb(231, 76, 60)
                                        };
                                        let remain = (order.vol - order.vol_exec).max(0.0);

                                        ui.label(dt);
                                        ui.label(&order.pair);
                                        ui.colored_label(side_color, &order.r#type);
                                        ui.label(&order.ordertype);
                                        ui.label(format!("{:.6}", order.price));
                                        ui.label(format!("{:.6}", order.vol));
                                        ui.label(format!("{:.6}", order.vol_exec));
                                        ui.label(format!("{:.6}", remain));
                                        ui.label(&order.status);
                                        ui.label(egui::RichText::new(&order.txid).small());
                                        if ui.small_button("Cancel").clicked() {
                                            let _ =
                                                self.broker_tx.send(BrokerCmd::KrakenCancelOrder {
                                                    txid: order.txid.clone(),
                                                });
                                        }
                                        ui.end_row();
                                    }
                                });
                        });
                });
        }

        // DARWIN Accounts — reads from self.bg (background-computed data)
        if self.show_darwin_accounts {
            egui::Window::new("DARWIN Accounts")
                        .open(&mut self.show_darwin_accounts)
                        .resizable(true).default_size([800.0, 600.0])
        .max_size([800.0, 640.0])
                        .show(ctx, |ui| {
                            // Soft palette for charts
                            let chart_green = egui::Color32::from_rgb(46, 204, 113);
                            let chart_red = egui::Color32::from_rgb(231, 76, 60);
                            let _chart_blue = egui::Color32::from_rgb(52, 152, 219);
                            let chart_gold = egui::Color32::from_rgb(241, 196, 15);
                            let chart_purple = egui::Color32::from_rgb(155, 89, 182);
                            let chart_cyan = egui::Color32::from_rgb(26, 188, 156);
                            let dim = egui::Color32::from_rgb(100, 100, 120);

                            if self.bg.accounts.is_empty() && self.cache.is_some() {
                                ui.label(egui::RichText::new("Loading DARWIN account data...").color(AXIS_TEXT));
                                return;
                            }
                            if self.bg.accounts.is_empty() { return; }

                            // Load AuM values from KV if not yet loaded
                            if self.darwin_aum.is_empty() {
                                if let Some(ref cache) = self.cache {
                                    for acct in &self.bg.accounts {
                                        if let Ok(Some(val)) = cache.get_kv(&format!("darwin:aum:{}", acct.darwin_ticker)) {
                                            self.darwin_aum.insert(acct.darwin_ticker.clone(), val);
                                        }
                                    }
                                }
                            }

                            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                            // ── Compact overview table ──────────────────────────────────
                            let mut darwin_to_delete: Option<String> = None;
                            let mut aum_changed: Option<(String, String)> = None;
                            egui::Grid::new("darwin_overview").striped(true).num_columns(12).min_col_width(55.0).show(ui, |ui| {
                                ui.label(egui::RichText::new("DARWIN").color(dim).small());
                                ui.label(egui::RichText::new("MT5").color(dim).small());
                                ui.label(egui::RichText::new("Deals").color(dim).small());
                                ui.label(egui::RichText::new("Pos").color(dim).small());
                                ui.label(egui::RichText::new("Balance").color(dim).small());
                                ui.label(egui::RichText::new("P&L").color(dim).small());
                                ui.label(egui::RichText::new("Win%").color(dim).small());
                                ui.label(egui::RichText::new("PF").color(dim).small());
                                ui.label(egui::RichText::new("Quote").color(dim).small());
                                ui.label(egui::RichText::new("Q.Ret%").color(dim).small());
                                ui.label(egui::RichText::new("AuM").color(dim).small());
                                ui.label(egui::RichText::new("").color(dim).small());
                                ui.end_row();
                                if !self.bg.account_details.is_empty() {
                                    let accounts_by_ticker: std::collections::HashMap<&str, &_> = self
                                        .bg
                                        .accounts
                                        .iter()
                                        .map(|a| (a.darwin_ticker.as_str(), a))
                                        .collect();
                                    for det in &self.bg.account_details {
                                        if let Some(ref s) = det.summary {
                                            let acct = accounts_by_ticker.get(det.ticker.as_str()).copied();
                                            ui.label(egui::RichText::new(&det.ticker).strong().color(chart_cyan));
                                            ui.label(egui::RichText::new(acct.map(|a| a.mt5_account.as_str()).unwrap_or("")).small());
                                            ui.label(format!("{}", s.win_count + s.loss_count));
                                            ui.label(format!("{}", acct.map(|a| a.position_count).unwrap_or(0)));
                                            ui.label(format!("${:.0}", s.final_balance));
                                            let pc = if s.total_profit >= 0.0 { chart_green } else { chart_red };
                                            ui.label(egui::RichText::new(format!("${:.0}", s.total_profit)).color(pc));
                                            let wc = if s.win_rate >= 50.0 { chart_green } else { chart_red };
                                            ui.label(egui::RichText::new(format!("{:.1}%", s.win_rate)).color(wc));
                                            ui.label(format!("{:.2}", s.profit_factor));
                                            // Quote columns from FTP data
                                            if let Some(ref fs) = det.ftp_summary {
                                                let qc = if fs.last_quote >= 100.0 { chart_green } else { chart_red };
                                                ui.label(egui::RichText::new(format!("{:.2}", fs.last_quote)).color(qc));
                                                let rc = if fs.total_return_pct >= 0.0 { chart_green } else { chart_red };
                                                ui.label(egui::RichText::new(format!("{:.1}%", fs.total_return_pct)).color(rc));
                                            } else {
                                                ui.label(egui::RichText::new("—").color(dim));
                                                ui.label(egui::RichText::new("—").color(dim));
                                            }
                                            // AuM input
                                            let aum_entry = self.darwin_aum.entry(det.ticker.clone()).or_default();
                                            let resp = ui.add(egui::TextEdit::singleline(aum_entry).desired_width(70.0).hint_text("AuM $").font(egui::TextStyle::Small));
                                            if resp.lost_focus() {
                                                aum_changed = Some((det.ticker.clone(), aum_entry.clone()));
                                            }
                                            if ui.small_button("X").on_hover_text(format!("Delete {}", det.ticker)).clicked() {
                                                darwin_to_delete = Some(det.ticker.clone());
                                            }
                                            ui.end_row();
                                        }
                                    }
                                } else {
                                    for acct in &self.bg.accounts {
                                        ui.label(egui::RichText::new(&acct.darwin_ticker).strong().color(chart_cyan));
                                        ui.label(egui::RichText::new(&acct.mt5_account).small());
                                        ui.label(format!("{}", acct.deal_count));
                                        ui.label(format!("{}", acct.position_count));
                                        ui.label(format!("${:.0}", acct.initial_balance));
                                        ui.label(egui::RichText::new("...").color(dim));
                                        ui.label(""); ui.label("");
                                        ui.label(""); ui.label("");
                                        // AuM input
                                        let aum_entry = self.darwin_aum.entry(acct.darwin_ticker.clone()).or_default();
                                        let resp = ui.add(egui::TextEdit::singleline(aum_entry).desired_width(70.0).hint_text("AuM $").font(egui::TextStyle::Small));
                                        if resp.lost_focus() {
                                            aum_changed = Some((acct.darwin_ticker.clone(), aum_entry.clone()));
                                        }
                                        if ui.small_button("X").on_hover_text(format!("Delete {}", acct.darwin_ticker)).clicked() {
                                            darwin_to_delete = Some(acct.darwin_ticker.clone());
                                        }
                                        ui.end_row();
                                    }
                                }
                            });

                            // Persist AuM changes to KV (synced to LAN clients automatically)
                            if let Some((ticker, val)) = aum_changed {
                                if let Some(ref cache) = self.cache {
                                    let _ = cache.put_kv(&format!("darwin:aum:{}", ticker), &val);
                                }
                            }

                            // Handle DARWIN deletion from grid button
                            if let Some(ticker) = darwin_to_delete {
                                // 1. Immediately remove from in-memory UI state (no freeze)
                                self.bg.accounts.retain(|a| a.darwin_ticker != ticker);
                                self.bg.account_details.retain(|d| d.ticker != ticker);

                                // 2. Write blacklist + update KV (fast, small writes)
                                if let Some(ref cache) = self.cache {
                                    let _ = cache.put_kv(&format!("darwin:deleted:{}", ticker), "1");
                                    let _ = cache.put_kv("darwin:account_details", &serde_json::to_string(&self.bg.account_details).unwrap_or_default());
                                }

                                // 3. Offload SQL DELETE through the app runtime's blocking pool.
                                if let Some(ref cache) = self.cache {
                                    let cache = cache.clone();
                                    let ticker_clone = ticker.clone();
                                    self.rt_handle.spawn_blocking(move || {
                                        if let Ok(conn) = cache.connection() {
                                            let _ = typhoon_engine::core::darwin::delete_darwin_account(
                                                &conn,
                                                &ticker_clone,
                                            );
                                        }
                                    });
                                }
                                self.log.push_back(LogEntry::info(format!("Deleting DARWIN {} (background)...", ticker)));
                            }

                            // ── Per-account detail cards with charts ─────────────────────
                            for det in &self.bg.account_details {
                                let summary = match det.summary.as_ref() { Some(s) => s, None => continue };
                                ui.add_space(6.0);
                                ui.separator();

                                // ── Account header with key metrics inline ──
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(&det.ticker).heading().color(chart_cyan));
                                    ui.add_space(10.0);
                                    let pc = if summary.total_profit >= 0.0 { chart_green } else { chart_red };
                                    ui.label(egui::RichText::new(format!("${:.0}", summary.total_profit)).color(pc).strong());
                                    ui.label(egui::RichText::new(format!("DD {:.1}%", summary.max_drawdown_pct)).color(chart_red).small());
                                    if let Some(ref var) = det.var_stats {
                                        ui.label(egui::RichText::new(format!("Sharpe {:.2}", var.sharpe)).color(chart_gold).small());
                                        ui.label(egui::RichText::new(format!("Sortino {:.2}", var.sortino)).color(chart_gold).small());
                                    }
                                    if let Some(ref ds) = det.dscore {
                                        ui.label(egui::RichText::new(format!("D-Score {:.1}", ds.total_dscore)).color(chart_purple).small());
                                    }
                                });

                                // ── Quote Performance (FTP) ──
                                if let Some(ref fs) = det.ftp_summary {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.spacing_mut().item_spacing.x = 12.0;
                                        let qc = if fs.last_quote >= 100.0 { chart_green } else { chart_red };
                                        ui.label(egui::RichText::new(format!("Quote {:.2}", fs.last_quote)).color(qc).small());
                                        let rc = if fs.total_return_pct >= 0.0 { chart_green } else { chart_red };
                                        ui.label(egui::RichText::new(format!("Ret {:.1}%", fs.total_return_pct)).color(rc).small());
                                        ui.label(egui::RichText::new(format!("MaxDD {:.1}%", fs.max_drawdown_pct)).color(chart_red).small());
                                        let sc = if fs.sharpe >= 0.0 { chart_green } else { chart_red };
                                        ui.label(egui::RichText::new(format!("Sharpe {:.2}", fs.sharpe)).color(sc).small());
                                        let soc = if fs.sortino >= 0.0 { chart_green } else { chart_red };
                                        ui.label(egui::RichText::new(format!("Sortino {:.2}", fs.sortino)).color(soc).small());
                                    });
                                }

                                // ── Replication Quality (Signal vs Quote) ──
                                if !det.daily_returns.is_empty() && !det.ftp_equity_curve.is_empty() {
                                    if let Some(mut rq) = darwin::compute_replication_quality(&det.daily_returns, &det.ftp_equity_curve) {
                                        rq.darwin_ticker = det.ticker.clone();
                                        let grade_c = match rq.quality_grade.as_str() {
                                            "A" => UP, "B" => egui::Color32::from_rgb(100, 200, 100),
                                            "C" => egui::Color32::from_rgb(241, 196, 15), _ => DOWN,
                                        };
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new(format!("Replication: {} (TE:{:.1}% IR:{:.2} R\u{00B2}:{:.2})",
                                                rq.quality_grade, rq.tracking_error, rq.information_ratio, rq.r_squared
                                            )).color(grade_c).small());
                                        });
                                    }
                                }

                                // ── Advanced metrics row (CAGR, RF, DD Duration) ──
                                ui.horizontal(|ui| {
                                    let cagr_c = if det.cagr >= 0.0 { chart_green } else { chart_red };
                                    ui.label(egui::RichText::new(format!("CAGR: {:.2}%", det.cagr)).color(cagr_c).small());
                                    ui.label(egui::RichText::new(format!("  RF: {:.2}", det.recovery_factor)).small());
                                    let (max_d, cur_d, _) = det.dd_duration;
                                    ui.label(egui::RichText::new(format!("  MaxDD: {}d", max_d)).small());
                                    if cur_d > 0 {
                                        ui.label(egui::RichText::new(format!("  InDD: {}d", cur_d)).color(chart_red).small());
                                    }
                                });

                                // ── Equity Curve ──
                                if det.equity_curve.len() > 2 {
                                    ui.label(egui::RichText::new("Equity").color(chart_cyan).small());
                                    let points: PlotPoints = PlotPoints::new(
                                        det.equity_curve.iter().enumerate().map(|(i, (_, bal))| [i as f64, *bal]).collect()
                                    );
                                    let line = Line::new("Equity", points).color(chart_cyan).width(1.5);
                                    Plot::new(format!("eq_{}", det.ticker))
                                        .height(100.0)
                                        .allow_drag(false).allow_zoom(false).allow_scroll(false)
                                        .show_axes([false, true])
                                        .show(ui, |plot_ui| { plot_ui.line(line); });
                                }

                                // ── Rolling 30d VaR ──
                                if det.rolling_var.len() > 5 {
                                    ui.label(egui::RichText::new("Rolling 30d VaR").color(chart_red).small());
                                    let var_pts: PlotPoints = PlotPoints::new(
                                        det.rolling_var.iter().enumerate().map(|(i, rv)| [i as f64, rv.var_95.abs()]).collect()
                                    );
                                    let var_line = Line::new("VaR 95%", var_pts).color(chart_red).width(1.2);
                                    Plot::new(format!("rvar_{}", det.ticker))
                                        .height(50.0)
                                        .allow_drag(false).allow_zoom(false).allow_scroll(false)
                                        .show_axes([false, true])
                                        .show(ui, |plot_ui| { plot_ui.line(var_line); });
                                }

                                // ── Cumulative Monthly P&L ──
                                if !det.monthly_returns.is_empty() {
                                    ui.label(egui::RichText::new("Cumulative Monthly P&L").color(dim).small());
                                    let monthly: Vec<&darwin::MonthlyReturn> = det.monthly_returns.iter().rev().take(24).collect::<Vec<_>>().into_iter().rev().collect();
                                    let mut cum = 0.0_f64;
                                    let pts: PlotPoints = PlotPoints::new(
                                        monthly.iter().enumerate().map(|(i, m)| { cum += m.pnl; [i as f64, cum] }).collect()
                                    );
                                    let c = if cum >= 0.0 { chart_green } else { chart_red };
                                    let line = Line::new("Cumulative P&L", pts).color(c).width(1.2);
                                    Plot::new(format!("cumpnl_{}", det.ticker))
                                        .height(50.0)
                                        .allow_drag(false).allow_zoom(false).allow_scroll(false)
                                        .show_axes([false, true])
                                        .show(ui, |plot_ui| { plot_ui.line(line); });
                                }

                                // ── Compact metrics row ──
                                ui.horizontal_wrapped(|ui| {
                                    ui.spacing_mut().item_spacing.x = 12.0;
                                    if let Some(ref var) = det.var_stats {
                                        ui.label(egui::RichText::new(format!("VaR95 ${:.0}", var.var_95)).color(chart_red).small());
                                        ui.label(egui::RichText::new(format!("Best ${:.0}", var.best_day)).color(chart_green).small());
                                        ui.label(egui::RichText::new(format!("Worst ${:.0}", var.worst_day)).color(chart_red).small());
                                        ui.label(egui::RichText::new(format!("Vol {:.3}", var.daily_vol)).color(dim).small());
                                    }
                                    if let Some(ref kelly) = det.kelly {
                                        ui.label(egui::RichText::new(format!("Kelly {:.1}%", kelly.half_kelly * 100.0)).color(chart_purple).small());
                                    }
                                    if let Some(ref ht) = det.hold_time {
                                        ui.label(egui::RichText::new(format!("Hold {:.0}h", ht.avg_hold_hours)).color(dim).small());
                                    }
                                    if let Some(ref costs) = det.cost_analysis {
                                        ui.label(egui::RichText::new(format!("Comm ${:.0}", costs.total_commission)).color(dim).small());
                                    }
                                    if let Some(ref streaks) = det.streaks {
                                        ui.label(egui::RichText::new(format!("W{}/L{}", streaks.max_win_streak, streaks.max_loss_streak)).color(dim).small());
                                        let sc = if streaks.current_streak >= 0 { chart_green } else { chart_red };
                                        ui.label(egui::RichText::new(format!("Now:{}", streaks.current_streak)).color(sc).small());
                                    }
                                });

                                // ── Collapsible advanced details ──
                                ui.collapsing(format!("{} Advanced", det.ticker), |ui| {
                                    // ── Day of Week + Hourly P&L side by side ──
                                    ui.horizontal(|ui| {
                                        if !det.day_of_week.is_empty() {
                                            let bars: Vec<PlotBar> = det.day_of_week.iter().enumerate().map(|(i, d)| {
                                                let c = if d.total_pnl >= 0.0 { chart_green } else { chart_red };
                                                PlotBar::new(i as f64, d.total_pnl).width(0.7).fill(c).name(&d.day)
                                            }).collect();
                                            let chart = BarChart::new("Day of Week", bars);
                                            Plot::new(format!("dow_{}", det.ticker))
                                                .height(80.0).width(250.0)
                                                .allow_drag(false).allow_zoom(false).allow_scroll(false)
                                                .show_axes([false, true])
                                                .show(ui, |plot_ui| { plot_ui.bar_chart(chart); });
                                        }
                                        if !det.hourly_pnl.is_empty() {
                                            let bars: Vec<PlotBar> = det.hourly_pnl.iter().map(|h| {
                                                let c = if h.total_pnl >= 0.0 { chart_green } else { chart_red };
                                                PlotBar::new(h.hour as f64, h.total_pnl).width(0.7).fill(c).name(format!("{:02}:00", h.hour))
                                            }).collect();
                                            let chart = BarChart::new("Hourly P&L", bars);
                                            Plot::new(format!("hr_{}", det.ticker))
                                                .height(80.0).width(400.0)
                                                .allow_drag(false).allow_zoom(false).allow_scroll(false)
                                                .show_axes([false, true])
                                                .show(ui, |plot_ui| { plot_ui.bar_chart(chart); });
                                        }
                                    });
                                    // D-Score radar (compact grid)
                                    if let Some(ref ds) = det.dscore {
                                        egui::Grid::new(format!("ds_{}", det.ticker)).num_columns(6).show(ui, |ui| {
                                            let scores = [("Exp", ds.experience), ("Risk", ds.risk_mgmt), ("Perf", ds.performance),
                                                           ("Timing", ds.market_timing), ("Cap", ds.capacity), ("Scale", ds.scalability)];
                                            for (label, _) in &scores { ui.label(egui::RichText::new(*label).color(dim).small()); }
                                            ui.end_row();
                                            for (_, val) in &scores {
                                                let c = if *val >= 7.0 { chart_green } else if *val >= 4.0 { chart_gold } else { chart_red };
                                                ui.label(egui::RichText::new(format!("{:.1}", val)).color(c).strong());
                                            }
                                            ui.end_row();
                                        });
                                    }
                                    // Benchmark
                                    if let Some(ref bench) = det.benchmark {
                                        ui.horizontal(|ui| {
                                            let ac = if bench.alpha >= 0.0 { chart_green } else { chart_red };
                                            ui.label(egui::RichText::new(format!("Alpha: {:.4}", bench.alpha)).color(ac).small());
                                            ui.label(egui::RichText::new(format!("Beta: {:.3}  IR: {:.3}", bench.beta, bench.information_ratio)).color(dim).small());
                                        });
                                    }
                                    // MAE/MFE + Slippage
                                    ui.horizontal(|ui| {
                                        if let Some(ref mae) = det.mae_mfe {
                                            ui.label(egui::RichText::new(format!("MAE {:.2}%  MFE {:.2}%  Ratio {:.2}", mae.avg_mae_pct, mae.avg_mfe_pct, mae.mae_mfe_ratio)).color(dim).small());
                                        }
                                        if let Some(ref slip) = det.slippage {
                                            ui.label(egui::RichText::new(format!("Slip {:.4}% (${:.0})", slip.avg_slippage_pct, slip.total_slippage_cost)).color(dim).small());
                                        }
                                    });
                                    // Autocorrelation
                                    if let Some(ref ac) = det.autocorrelation {
                                        let rc = if ac.is_random { chart_green } else { chart_gold };
                                        ui.label(egui::RichText::new(format!("Autocorr L1:{:.3} L2:{:.3} L3:{:.3} — {}", ac.lag1, ac.lag2, ac.lag3, ac.interpretation)).color(rc).small());
                                    }
                                    // Hold Time buckets as bar chart
                                    if let Some(ref ht) = det.hold_time {
                                        if !ht.buckets.is_empty() {
                                            let bars: Vec<PlotBar> = ht.buckets.iter().enumerate().map(|(i, (_, _, avg_pnl))| {
                                                let c = if *avg_pnl >= 0.0 { chart_green } else { chart_red };
                                                PlotBar::new(i as f64, *avg_pnl).width(0.7).fill(c)
                                            }).collect();
                                            let chart = BarChart::new("Hold Time Avg P&L", bars);
                                            Plot::new(format!("ht_{}", det.ticker))
                                                .height(60.0)
                                                .allow_drag(false).allow_zoom(false).allow_scroll(false)
                                                .show_axes([false, true])
                                                .show(ui, |plot_ui| { plot_ui.bar_chart(chart); });
                                        }
                                    }
                                    // Open Positions
                                    if !det.open_positions.is_empty() {
                                        ui.label(egui::RichText::new(format!("Open Positions ({})", det.open_positions.len())).small().strong());
                                        for p in &det.open_positions {
                                            let sc = if p.side == "buy" { chart_green } else { chart_red };
                                            ui.label(egui::RichText::new(format!("  {} {} {:.2} @ {}", p.symbol, p.side, p.total_volume, format_price(p.avg_price))).color(sc).small());
                                        }
                                    }
                                    // Performance Attribution (per-symbol P&L contribution)
                                    if !det.performance_attribution.is_empty() {
                                        ui.label(egui::RichText::new("P&L Attribution (Top 10)").small().strong());
                                        egui::Grid::new(format!("attr_{}", det.ticker)).striped(true).num_columns(4).show(ui, |ui| {
                                            ui.label(egui::RichText::new("Symbol").color(dim).small());
                                            ui.label(egui::RichText::new("P&L").color(dim).small());
                                            ui.label(egui::RichText::new("Win%").color(dim).small());
                                            ui.label(egui::RichText::new("Contrib%").color(dim).small());
                                            ui.end_row();
                                            for a in det.performance_attribution.iter().take(10) {
                                                ui.label(egui::RichText::new(&a.symbol).small());
                                                let pc = if a.total_pnl >= 0.0 { chart_green } else { chart_red };
                                                ui.label(egui::RichText::new(format!("${:.0}", a.total_pnl)).color(pc).small());
                                                {
                                                    let awr_c = if a.win_rate >= 50.0 { UP } else if a.win_rate >= 40.0 { egui::Color32::from_rgb(255, 200, 50) } else { DOWN };
                                                    ui.label(egui::RichText::new(format!("{:.1}%", a.win_rate)).color(awr_c).small());
                                                }
                                                let cont_c = if a.contribution_pct >= 0.0 { UP } else { DOWN };
                                                ui.label(egui::RichText::new(format!("{:.1}%", a.contribution_pct)).color(cont_c).small());
                                                ui.end_row();
                                            }
                                        });
                                    }
                                    // D-Score Components (FTP) — show grid if available
                                    if let Some(ref dsc) = det.dscore_components {
                                        ui.label(egui::RichText::new("D-Score Components (FTP)").small().strong());
                                        egui::Grid::new(format!("dsc_{}", det.ticker)).num_columns(4).show(ui, |ui| {
                                            let comps: Vec<(&str, f64)> = vec![
                                                ("Experience", dsc.experience.unwrap_or(0.0)),
                                                ("Risk Stability", dsc.risk_stability.unwrap_or(0.0)),
                                                ("Risk Adj", dsc.risk_adjustment.unwrap_or(0.0)),
                                                ("Mkt Corr", dsc.market_correlation.unwrap_or(0.0)),
                                                ("Win Consist", dsc.winning_consistency.unwrap_or(0.0)),
                                                ("Loss Consist", dsc.losing_consistency.unwrap_or(0.0)),
                                                ("Performance", dsc.performance.unwrap_or(0.0)),
                                                ("Scalability", dsc.scalability.unwrap_or(0.0)),
                                            ];
                                            for (label, val) in &comps {
                                                let c = if *val >= 7.0 { chart_green } else if *val >= 4.0 { chart_gold } else { chart_red };
                                                ui.label(egui::RichText::new(*label).color(dim).small());
                                                ui.label(egui::RichText::new(format!("{:.1}", val)).color(c).strong());
                                            }
                                            ui.end_row();
                                        });
                                    }
                                    // Investment Velocity (FTP)
                                    if !det.investment_velocity.is_empty() {
                                        ui.label(egui::RichText::new("Investor Growth Rate").small().strong());
                                        let pts: PlotPoints = PlotPoints::new(
                                            det.investment_velocity.iter().enumerate().map(|(i, (_, v))| [i as f64, *v]).collect()
                                        );
                                        let c = if det.investment_velocity.last().map(|(_, v)| *v >= 0.0).unwrap_or(true) { chart_green } else { chart_red };
                                        Plot::new(format!("velocity_{}", det.ticker)).height(50.0)
                                            .allow_drag(false).allow_zoom(false).allow_scroll(false)
                                            .show_axes([false, true])
                                            .show(ui, |plot_ui| { plot_ui.line(Line::new("Growth%", pts).color(c).width(1.0)); });
                                    }
                                    // Recent Deals (compact)
                                    if !det.recent_deals.is_empty() {
                                        ui.label(egui::RichText::new(format!("Recent Deals ({})", det.recent_deals.len())).small().strong());
                                        egui::Grid::new(format!("deals_{}", det.ticker)).striped(true).num_columns(5).show(ui, |ui| {
                                            for d in det.recent_deals.iter().take(10) {
                                                ui.label(egui::RichText::new(&d.time).color(dim).small());
                                                ui.label(egui::RichText::new(&d.symbol).small());
                                                let tc = if d.deal_type == "buy" { chart_green } else { chart_red };
                                                ui.label(egui::RichText::new(&d.deal_type).color(tc).small());
                                                ui.label(egui::RichText::new(format!("{:.2}", d.volume)).color(dim).small());
                                                let pc = if d.profit >= 0.0 { chart_green } else { chart_red };
                                                ui.label(egui::RichText::new(format!("${:.0}", d.profit)).color(pc).small());
                                                ui.end_row();
                                            }
                                        });
                                    }
                                });
                            } // end for det

                            // ── Floating Equity (compact) ─────────────────────────────────
                            {
                                ui.add_space(6.0);
                                ui.separator();
                                // Use account summaries as primary source (always populated from deals)
                                // Fall back to floating_equity dashboard for unrealized/MTM data
                                let details = &self.bg.account_details;
                                let total_balance: f64 = details.iter()
                                    .filter_map(|d| d.summary.as_ref())
                                    .map(|s| s.final_balance)
                                    .sum();
                                let total_unrealized: f64 = self.bg.floating_equity.as_ref()
                                    .map(|f| f.combined_unrealized_pnl)
                                    .unwrap_or(0.0);
                                let combined = total_balance + total_unrealized;
                                let fc = if combined >= total_balance { chart_green } else { chart_red };
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("Floating Equity").strong());
                                    ui.label(egui::RichText::new(format!("${:.0}", combined)).color(fc).strong());
                                });
                                egui::Grid::new("float_eq").striped(true).num_columns(4).min_col_width(70.0).show(ui, |ui| {
                                    ui.label(egui::RichText::new("DARWIN").color(dim).small());
                                    ui.label(egui::RichText::new("Balance").color(dim).small());
                                    ui.label(egui::RichText::new("P&L").color(dim).small());
                                    ui.label(egui::RichText::new("Quote").color(dim).small());
                                    ui.end_row();
                                    for det in details {
                                        if let Some(ref s) = det.summary {
                                            let pnl = s.total_profit;
                                            let pc = if pnl >= 0.0 { chart_green } else { chart_red };
                                            ui.label(egui::RichText::new(&det.ticker).small());
                                            ui.label(egui::RichText::new(format!("${:.0}", s.final_balance)).small());
                                            ui.label(egui::RichText::new(format!("${:.0}", pnl)).color(pc).small());
                                            if let Some(ref fs) = det.ftp_summary {
                                                let qc = if fs.last_quote >= 100.0 { chart_green } else { chart_red };
                                                ui.label(egui::RichText::new(format!("{:.2}", fs.last_quote)).color(qc).small());
                                            } else {
                                                ui.label(egui::RichText::new("—").color(dim).small());
                                            }
                                            ui.end_row();
                                        }
                                    }
                                });
                            }

                            // ── Import / Tools (compact) ──────────────────────────────────
                            ui.add_space(6.0);
                            ui.separator();
                            ui.horizontal(|ui| {
                                if !self.darwin_xlsx_dir.is_empty() {
                                    if ui.small_button("Import All XLSX Now").clicked() {
                                        let db_path = cache_db_path();
                                        let _ = self.broker_tx.send(BrokerCmd::DarwinImportAll {
                                            dir: PathBuf::from(&self.darwin_xlsx_dir),
                                            db_path,
                                        });
                                        self.log.push_back(LogEntry::info(format!("DARWIN XLSX import started from {}", self.darwin_xlsx_dir)));
                                    }
                                }
                                if ui.small_button("Daily Risk Report").clicked() {
                                    if let Some(ref cache) = self.cache {
                                        if let Some(conn) = cache.try_connection() {
                                            match darwin::generate_daily_report(&conn) {
                                                Ok(report) => {
                                                    self.log.push_back(LogEntry::info(format!(
                                                        "Daily Report {}: Equity ${:.0}, P&L ${:.2} ({:.2}%), VaR95 {:.2}%, DD {:.2}%",
                                                        report.date, report.portfolio_equity, report.daily_pnl,
                                                        report.daily_return_pct, report.current_var_95, report.current_drawdown_pct
                                                    )));
                                                }
                                                Err(e) => { self.log.push_back(LogEntry::err(format!("Report error: {}", e))); }
                                            }
                                        }
                                    }
                                }
                                if ui.small_button("Export Radar TXT").clicked() {
                                    if let Some(ref cache) = self.cache {
                                        if let Some(conn) = cache.try_connection() {
                                            let mut out = dirs_home();
                                            out.push("export");
                                            let _ = std::fs::create_dir_all(&out);
                                            match darwin::export_radar_txt(&conn, &conn, &out.display().to_string()) {
                                                Ok(path) => self.log.push_back(LogEntry::info(format!("Radar exported: {}", path))),
                                                Err(e) => self.log.push_back(LogEntry::err(format!("Export failed: {}", e))),
                                            }
                                        }
                                    }
                                }
                                if ui.small_button("SwapHarvest").clicked() {
                                    if let Some(ref cache) = self.cache {
                                        if let Some(conn) = cache.try_connection() {
                                            match darwin::swap_harvest(&conn, 0.0) {
                                                Ok(result) => {
                                                    self.log.push_back(LogEntry::info(format!(
                                                        "SWAPHARVEST: {} symbols with positive swap ({} long, {} short, {} both) out of {} scanned",
                                                        result.entries.len(), result.long_count, result.short_count, result.both_count, result.total_scanned
                                                    )));
                                                    self.swap_harvest_results = Some(result);
                                                    self.show_swap_harvest = true;
                                                }
                                                Err(e) => self.log.push_back(LogEntry::err(format!("SwapHarvest failed: {}", e))),
                                            }
                                        }
                                    }
                                }
                                let ftp_available = !self.darwin_ftp_dir.is_empty();
                                if ftp_available {
                                    let label = if self.gpu_darwin.is_some() { "DarwinIA Scan (GPU)" } else { "DarwinIA Scan (CPU)" };
                                    if ui.small_button(label).clicked() {
                                        if self.gpu_darwin.is_some() {
                                            let _ = self.broker_tx.send(BrokerCmd::DarwinGpuScan { ftp_dir: self.darwin_ftp_dir.clone(), min_days: 90 });
                                            self.log.push_back(LogEntry::info("DarwinIA scan started (GPU)..."));
                                        } else {
                                            let _ = self.broker_tx.send(BrokerCmd::DarwinFtpScan { ftp_dir: self.darwin_ftp_dir.clone(), min_days: 90 });
                                            self.log.push_back(LogEntry::info("DarwinIA scan started (CPU)..."));
                                        }
                                    }
                                }
                            });

                            // ── Delete Account (compact) ──
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Delete:").color(dim).small());
                                ui.add(egui::TextEdit::singleline(&mut self.darwin_import_ticker).desired_width(60.0));
                                if ui.small_button(egui::RichText::new("Delete").color(chart_red)).clicked() {
                                    let ticker = self.darwin_import_ticker.trim().to_string();
                                    if !ticker.is_empty() {
                                        if let Some(ref cache) = self.cache {
                                            if let Some(conn) = cache.try_connection() {
                                                match darwin::delete_darwin_account(&conn, &ticker) {
                                                    Ok(n) => { self.log.push_back(LogEntry::info(format!("Deleted DARWIN account: {} ({} rows)", ticker, n))); }
                                                    Err(e) => { self.log.push_back(LogEntry::err(format!("Delete failed: {}", e))); }
                                                }
                                            }
                                        }
                                    }
                                }
                            });
                            }); // ScrollArea
                        });
        }

        if self.show_darwin_portfolio {
            egui::Window::new("DARWIN Portfolio")
                        .open(&mut self.show_darwin_portfolio)
                        .resizable(true).default_size([700.0, 500.0])
        .max_size([700.0, 560.0])
                        .show(ctx, |ui| {
                            // View selector dropdown (matching old WebKit 20+ views)
                            let views = [
                                "Portfolio Summary", "Portfolio VaR", "Equity Curves", "Correlation Matrix",
                                "Symbol Exposure", "Combined Positions", "Trade Overlaps", "Drawdown",
                                "Monte Carlo", "Stress Test", "VaR Forecast", "Conditional VaR",
                                "Market Regime", "Tail Risk", "Seasonals", "Sector Exposure",
                                "Liquidity Risk", "Margin Call Sim", "Optimal Allocation", "What-If",
                                "VaR Simulator",
                            ];
                            let _prev_view = self.darwin_view;
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("View:").color(AXIS_TEXT));
                                egui::ComboBox::from_id_salt("darwin_view_combo")
                                    .selected_text(*views.get(self.darwin_view).unwrap_or(&"Portfolio Summary"))
                                    .width(200.0)
                                    .show_ui(ui, |ui| {
                                        for (i, v) in views.iter().enumerate() {
                                            ui.selectable_value(&mut self.darwin_view, i, *v);
                                        }
                                    });
                            });
                            ui.separator();
                            // Read directly from self.bg (background-computed data, no DB queries)
                            {
                                let dv = self.darwin_view;
                                egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                                match self.bg.portfolio.as_ref() {
                                    Some(portfolio) if !portfolio.accounts.is_empty() => {
                                        match dv {
                                                0 => { // Portfolio Summary
                                                    egui::Grid::new("port_summary").striped(true).num_columns(2).show(ui, |ui| {
                                                        ui.label("Accounts:"); ui.label(format!("{}", portfolio.accounts.len()));
                                                        ui.end_row();
                                                        ui.label("Initial Balance:"); ui.label(format!("${:.2}", portfolio.total_initial_balance));
                                                        ui.end_row();
                                                        ui.label("Final Balance:"); ui.label(format!("${:.2}", portfolio.total_final_balance));
                                                        ui.end_row();
                                                        let pnl_c = if portfolio.total_net_pnl >= 0.0 { UP } else { DOWN };
                                                        ui.label("Net P&L:"); ui.label(egui::RichText::new(format!("${:.2}", portfolio.total_net_pnl)).color(pnl_c));
                                                        ui.end_row();
                                                        ui.label("Total Commission:"); ui.label(format!("${:.2}", portfolio.total_commission));
                                                        ui.end_row();
                                                        ui.label("Max Drawdown:"); ui.label(egui::RichText::new(format!("{:.2}%", portfolio.combined_max_drawdown_pct)).color(DOWN));
                                                        ui.end_row();
                                                        ui.label("Total Deals:"); ui.label(format!("{}", portfolio.total_deals));
                                                        ui.end_row();
                                                    });
                                                    // Per-DARWIN table
                                                    ui.add_space(10.0);
                                                    ui.horizontal(|ui| {
                                                        ui.heading("Per-DARWIN");
                                                        // ADR-094: Context palette for DARWIN rows
                                                        if ui.small_button("Commands…").clicked() {
                                                            self.palette_context = PaletteContext::Darwin;
                                                            self.command_open = true;
                                                            self.command_input.clear();
                                                        }
                                                    });
                                                    ui.separator();
                                                    egui::Grid::new("per_darwin").striped(true).num_columns(15).show(ui, |ui| {
                                                        ui.strong("DARWIN"); ui.strong("Equity"); ui.strong("Signal Bal"); ui.strong("Signal P&L");
                                                        ui.strong("Win%"); ui.strong("PF"); ui.strong("Signal DD%");
                                                        ui.strong("Quote"); ui.strong("Quote Ret%"); ui.strong("Quote DD%");
                                                        ui.strong("Divergence"); ui.strong("Multiplier");
                                                        ui.strong("Exp"); ui.strong("Risk"); ui.strong("Perf");
                                                        ui.end_row();
                                                        for acct in &portfolio.accounts {
                                                            let ticker = &acct.account.darwin_ticker;
                                                            ui.label(ticker);
                                                            // ADR-094: Equity sparkline from daily returns
                                                            {
                                                                let daily_returns = self.bg.account_details.iter()
                                                                    .find(|d| d.ticker == *ticker)
                                                                    .map(|d| &d.daily_returns);
                                                                let (rect, _) = ui.allocate_exact_size(egui::vec2(40.0, 12.0), egui::Sense::hover());
                                                                if let Some(dr) = daily_returns {
                                                                    if dr.len() >= 2 {
                                                                        let vals: Vec<f64> = dr.iter().map(|r| r.balance).collect();
                                                                        let color = if vals.last() >= vals.first() { UP } else { DOWN };
                                                                        draw_sparkline(ui.painter(), rect, &vals, color);
                                                                    }
                                                                }
                                                            }
                                                            ui.label(format!("${:.0}", acct.final_balance));
                                                            let c = if acct.total_profit >= 0.0 { UP } else { DOWN };
                                                            ui.label(egui::RichText::new(format!("${:.0}", acct.total_profit)).color(c));
                                                            let wr_c = if acct.win_rate >= 50.0 { UP } else { DOWN };
                                                            ui.label(egui::RichText::new(format!("{:.1}%", acct.win_rate)).color(wr_c));
                                                            ui.label(format!("{:.2}", acct.profit_factor));
                                                            ui.label(egui::RichText::new(format!("{:.1}%", acct.max_drawdown_pct)).color(DOWN));
                                                            // DARWIN quote columns (from FTP data)
                                                            let ftp = self.bg.account_details.iter()
                                                                .find(|d| d.ticker == *ticker)
                                                                .and_then(|d| d.ftp_summary.as_ref());
                                                            if let Some(fs) = ftp {
                                                                let qc = if fs.last_quote >= 100.0 { UP } else { DOWN };
                                                                ui.label(egui::RichText::new(format!("{:.2}", fs.last_quote)).color(qc));
                                                                let rc = if fs.total_return_pct >= 0.0 { UP } else { DOWN };
                                                                ui.label(egui::RichText::new(format!("{:.1}%", fs.total_return_pct)).color(rc));
                                                                ui.label(egui::RichText::new(format!("{:.1}%", fs.max_drawdown_pct)).color(DOWN));
                                                                // Divergence: signal return % vs DARWIN quote return %
                                                                let signal_ret_pct = if acct.account.initial_balance > 0.0 {
                                                                    (acct.final_balance / acct.account.initial_balance - 1.0) * 100.0
                                                                } else { 0.0 };
                                                                let div = fs.total_return_pct - signal_ret_pct;
                                                                let dc = if div.abs() < 5.0 { AXIS_TEXT } else if div > 0.0 { UP } else { DOWN };
                                                                ui.label(egui::RichText::new(format!("{:+.1}%", div)).color(dc));
                                                                // VaR multiplier
                                                                let mult = self.bg.var_multipliers.iter()
                                                                    .find(|m| m.darwin_ticker == *ticker);
                                                                if let Some(m) = mult {
                                                                    let mc = if m.multiplier >= 1.0 { UP } else { DOWN };
                                                                    ui.label(egui::RichText::new(format!("{:.2}x", m.multiplier)).color(mc));
                                                                } else { ui.label("—"); }
                                                                // D-Score components
                                                                let ec = if fs.experience_score >= 5.0 { UP } else { AXIS_TEXT };
                                                                ui.label(egui::RichText::new(format!("{:.1}", fs.experience_score)).color(ec));
                                                                let rsc = if fs.risk_stability_score >= 5.0 { UP } else { AXIS_TEXT };
                                                                ui.label(egui::RichText::new(format!("{:.1}", fs.risk_stability_score)).color(rsc));
                                                                let pc = if fs.performance_score >= 5.0 { UP } else { AXIS_TEXT };
                                                                ui.label(egui::RichText::new(format!("{:.1}", fs.performance_score)).color(pc));
                                                            } else {
                                                                ui.label("—"); ui.label("—"); ui.label("—"); ui.label("—"); ui.label("—");
                                                                ui.label("—"); ui.label("—"); ui.label("—");
                                                            }
                                                            ui.end_row();
                                                        }
                                                    });
                                                        // ── DARWIN Ranking — composite score ──
                                                        if !self.bg.account_details.is_empty() {
                                                            ui.add_space(8.0);
                                                            ui.label(egui::RichText::new("DARWIN Ranking (Composite Score)").strong());

                                                            let mut rankings: Vec<(&str, f64, f64, f64, f64, f64)> = Vec::new(); // (ticker, sharpe, recovery, decay, composite, cagr)
                                                            for det in &self.bg.account_details {
                                                                let sharpe = det.var_stats.as_ref().map(|v| v.sharpe).unwrap_or(0.0);
                                                                let rf = det.recovery_factor;
                                                                let decay = det.dd_duration.0 as f64; // max DD duration (lower = better)
                                                                let repl = det.ftp_summary.as_ref().map(|_| 1.0).unwrap_or(0.0); // has FTP data

                                                                // Composite: higher = better
                                                                // Sharpe (weight 40%) + Recovery Factor (30%) - DD Duration penalty (20%) + Data completeness (10%)
                                                                let composite = sharpe * 0.4 + rf.min(5.0) / 5.0 * 0.3 - (decay / 200.0).min(1.0) * 0.2 + repl * 0.1;
                                                                rankings.push((&det.ticker, sharpe, det.recovery_factor, det.dd_duration.0 as f64, composite * 100.0, det.cagr));
                                                            }
                                                            rankings.sort_by(|a, b| b.4.partial_cmp(&a.4).unwrap_or(std::cmp::Ordering::Equal));

                                                            egui::Grid::new("darwin_ranking").striped(true).num_columns(7).show(ui, |ui| {
                                                                ui.label(egui::RichText::new("Rank").color(AXIS_TEXT).small().strong());
                                                                ui.label(egui::RichText::new("DARWIN").color(AXIS_TEXT).small().strong());
                                                                ui.label(egui::RichText::new("Score").color(AXIS_TEXT).small().strong());
                                                                ui.label(egui::RichText::new("Sharpe").color(AXIS_TEXT).small().strong());
                                                                ui.label(egui::RichText::new("CAGR").color(AXIS_TEXT).small().strong());
                                                                ui.label(egui::RichText::new("Recovery").color(AXIS_TEXT).small().strong());
                                                                ui.label(egui::RichText::new("Max DD Days").color(AXIS_TEXT).small().strong());
                                                                ui.end_row();
                                                                for (rank, (ticker, sharpe, rf, dd_days, score, cagr)) in rankings.iter().enumerate() {
                                                                    let rank_c = match rank { 0 => egui::Color32::from_rgb(255, 215, 0), 1 => egui::Color32::from_rgb(192, 192, 192), 2 => egui::Color32::from_rgb(205, 127, 50), _ => egui::Color32::WHITE };
                                                                    ui.label(egui::RichText::new(format!("#{}", rank + 1)).color(rank_c).small().strong());
                                                                    ui.label(egui::RichText::new(*ticker).small());
                                                                    let sc = if *score > 50.0 { UP } else if *score > 25.0 { egui::Color32::from_rgb(241, 196, 15) } else { DOWN };
                                                                    ui.label(egui::RichText::new(format!("{:.0}", score)).color(sc).small().strong());
                                                                    ui.label(egui::RichText::new(format!("{:.2}", sharpe)).small());
                                                                    let cc = if *cagr >= 0.0 { UP } else { DOWN };
                                                                    ui.label(egui::RichText::new(format!("{:.1}%", cagr)).color(cc).small());
                                                                    ui.label(egui::RichText::new(format!("{:.2}", rf)).small());
                                                                    ui.label(egui::RichText::new(format!("{:.0}", dd_days)).small());
                                                                    ui.end_row();
                                                                }
                                                            });
                                                        }
                                                }
                                                1 => { // Portfolio VaR (from bg cache)
                                                    { let daily = &self.bg.daily_returns; if !daily.is_empty() {
                                                        if let Some(ref vs) = self.bg.var_stats {
                                                            egui::Grid::new("port_var").striped(true).num_columns(4).show(ui, |ui| {
                                                                ui.label("VaR 95%:"); ui.label(format!("${:.2}", vs.var_95));
                                                                ui.label("Sharpe:"); ui.label(format!("{:.3}", vs.sharpe));
                                                                ui.end_row();
                                                                ui.label("VaR 99%:"); ui.label(format!("${:.2}", vs.var_99));
                                                                ui.label("Sortino:"); ui.label(format!("{:.3}", vs.sortino));
                                                                ui.end_row();
                                                                ui.label("CVaR 95%:"); ui.label(format!("${:.2}", vs.cvar_95));
                                                                ui.label("Calmar:"); ui.label(format!("{:.3}", vs.calmar));
                                                                ui.end_row();
                                                                ui.label("CVaR 99%:"); ui.label(format!("${:.2}", vs.cvar_99));
                                                                ui.label("Max DD:"); ui.label(egui::RichText::new(format!("{:.2}%", vs.max_drawdown_pct)).color(DOWN));
                                                                ui.end_row();
                                                                ui.label("Daily Vol:"); ui.label(format!("{:.4}", vs.daily_vol));
                                                                ui.label("Ann. Vol:"); ui.label(format!("{:.4}", vs.annualized_vol));
                                                                ui.end_row();
                                                                ui.label("Best Day:"); ui.label(egui::RichText::new(format!("${:.2}", vs.best_day)).color(UP));
                                                                ui.label("Worst Day:"); ui.label(egui::RichText::new(format!("${:.2}", vs.worst_day)).color(DOWN));
                                                                ui.end_row();
                                                                ui.label("Avg Daily:"); ui.label(format!("${:.2}", vs.avg_daily_pnl));
                                                                ui.label("Trading Days:"); ui.label(format!("{}", vs.trading_days));
                                                                ui.end_row();
                                                            });
                                                            // Rolling VaR (30-day window) — from bg cache
                                                            let rolling = &self.bg.rolling_var;
                                                            if rolling.len() > 5 {
                                                                ui.add_space(10.0);
                                                                ui.label(egui::RichText::new("Rolling 30d VaR").strong());
                                                                let points: PlotPoints = PlotPoints::new(
                                                                    rolling.iter().enumerate().map(|(i, rv)| [i as f64, rv.var_95]).collect()
                                                                );
                                                                let line = Line::new("VaR95", points).color(DOWN);
                                                                Plot::new("rolling_var_plot").height(120.0).allow_drag(false).allow_zoom(false)
                                                                    .show(ui, |plot_ui| { plot_ui.line(line); });
                                                            }
                                                            // Combined drawdown dashboard — from bg cache
                                                            if let Some(ref dd) = self.bg.drawdown_dashboard {
                                                                ui.add_space(10.0);
                                                                ui.label(egui::RichText::new("Drawdown Dashboard").strong());
                                                                egui::Grid::new("dd_dash").striped(true).num_columns(6).show(ui, |ui| {
                                                                    ui.strong("DARWIN"); ui.strong("Signal Max DD"); ui.strong("Date"); ui.strong("Signal Curr DD");
                                                                    ui.strong("Quote Max DD"); ui.strong("Quote Curr DD");
                                                                    ui.end_row();
                                                                    for d in &dd.darwins {
                                                                        ui.label(&d.darwin_ticker);
                                                                        ui.label(egui::RichText::new(format!("{:.2}%", d.max_drawdown_pct)).color(DOWN));
                                                                        ui.label(&d.max_dd_date);
                                                                        ui.label(egui::RichText::new(format!("{:.2}%", d.current_drawdown_pct)).color(DOWN));
                                                                        // Quote DD from FTP data
                                                                        let ftp = self.bg.account_details.iter()
                                                                            .find(|det| det.ticker == d.darwin_ticker)
                                                                            .and_then(|det| det.ftp_summary.as_ref());
                                                                        if let Some(fs) = ftp {
                                                                            ui.label(egui::RichText::new(format!("{:.2}%", fs.max_drawdown_pct)).color(DOWN));
                                                                            // Current DD from last drawdown curve point
                                                                            let cur_dd = self.bg.account_details.iter()
                                                                                .find(|det| det.ticker == d.darwin_ticker)
                                                                                .and_then(|det| det.ftp_drawdown_curve.last())
                                                                                .map(|&(_, dd)| -dd) // stored negative
                                                                                .unwrap_or(0.0);
                                                                            ui.label(egui::RichText::new(format!("{:.2}%", cur_dd)).color(DOWN));
                                                                        } else { ui.label(egui::RichText::new("—").color(AXIS_TEXT)); ui.label(egui::RichText::new("—").color(AXIS_TEXT)); }
                                                                        ui.end_row();
                                                                    }
                                                                    // Combined row
                                                                    ui.label(egui::RichText::new("COMBINED").strong());
                                                                    ui.label(egui::RichText::new(format!("{:.2}%", dd.combined.max_drawdown_pct)).color(DOWN).strong());
                                                                    ui.label(&dd.combined.max_dd_date);
                                                                    ui.label(egui::RichText::new(format!("{:.2}%", dd.combined.current_drawdown_pct)).color(DOWN));
                                                                    ui.label(egui::RichText::new("—").color(AXIS_TEXT));
                                                                    ui.label(egui::RichText::new("—").color(AXIS_TEXT));
                                                                    ui.end_row();
                                                                });
                                                            }
                                                            // ── Signal vs Quote Risk Metrics ──
                                                            {
                                                                let has_any_ftp = self.bg.account_details.iter().any(|d| d.ftp_summary.is_some());
                                                                if has_any_ftp {
                                                                    ui.add_space(10.0);
                                                                    ui.label(egui::RichText::new("Signal vs Quote Risk Metrics").strong());
                                                                    egui::Grid::new("sig_vs_quote").striped(true).num_columns(5).show(ui, |ui| {
                                                                        ui.strong("DARWIN"); ui.strong("Signal Sharpe"); ui.strong("Quote Sharpe");
                                                                        ui.strong("Signal DD%"); ui.strong("Quote DD%");
                                                                        ui.end_row();
                                                                        for det in &self.bg.account_details {
                                                                            ui.label(&det.ticker);
                                                                            // Signal metrics from per-account var_stats
                                                                            if let Some(ref vs) = det.var_stats {
                                                                                ui.label(format!("{:.3}", vs.sharpe));
                                                                            } else { ui.label(egui::RichText::new("—").color(AXIS_TEXT)); }
                                                                            // Quote metrics from FTP summary
                                                                            if let Some(ref fs) = det.ftp_summary {
                                                                                let qsc = if fs.sharpe >= 0.0 { UP } else { DOWN };
                                                                                ui.label(egui::RichText::new(format!("{:.3}", fs.sharpe)).color(qsc));
                                                                            } else { ui.label(egui::RichText::new("—").color(AXIS_TEXT)); }
                                                                            // Signal DD%
                                                                            if let Some(ref vs) = det.var_stats {
                                                                                ui.label(egui::RichText::new(format!("{:.2}%", vs.max_drawdown_pct)).color(DOWN));
                                                                            } else if let Some(ref s) = det.summary {
                                                                                ui.label(egui::RichText::new(format!("{:.2}%", s.max_drawdown_pct)).color(DOWN));
                                                                            } else { ui.label(egui::RichText::new("—").color(AXIS_TEXT)); }
                                                                            // Quote DD%
                                                                            if let Some(ref fs) = det.ftp_summary {
                                                                                ui.label(egui::RichText::new(format!("{:.2}%", fs.max_drawdown_pct)).color(DOWN));
                                                                            } else { ui.label(egui::RichText::new("—").color(AXIS_TEXT)); }
                                                                            ui.end_row();
                                                                        }
                                                                    });
                                                                }
                                                            }
                                                            // ── Advanced Metrics (CAGR, Recovery Factor, DD Duration) ──
                                                            ui.add_space(10.0);
                                                            ui.label(egui::RichText::new("Advanced Metrics").strong());
                                                            egui::Grid::new("adv_metrics").striped(true).num_columns(4).show(ui, |ui| {
                                                                let portfolio_daily = &self.bg.daily_returns;
                                                                if !portfolio_daily.is_empty() {
                                                                    let cagr = darwin::compute_cagr(portfolio_daily);
                                                                    let rf = darwin::compute_recovery_factor(portfolio_daily);
                                                                    let (max_dd_d, cur_dd_d, avg_dd_d) = darwin::compute_drawdown_duration(portfolio_daily);
                                                                    ui.label("CAGR:"); ui.label(egui::RichText::new(format!("{:.2}%", cagr)).color(if cagr >= 0.0 { UP } else { DOWN }));
                                                                    ui.label("Recovery Factor:"); ui.label(format!("{:.2}", rf));
                                                                    ui.end_row();
                                                                    ui.label("Max DD Duration:"); ui.label(format!("{} days", max_dd_d));
                                                                    ui.label("Current DD Duration:"); ui.label(format!("{} days", cur_dd_d));
                                                                    ui.end_row();
                                                                    ui.label("Avg DD Duration:"); ui.label(format!("{:.0} days", avg_dd_d));
                                                                    ui.label(""); ui.label("");
                                                                    ui.end_row();
                                                                }
                                                            });
                                                            // Per-DARWIN Advanced Metrics
                                                            egui::Grid::new("per_darwin_adv").striped(true).num_columns(6).show(ui, |ui| {
                                                                ui.strong("DARWIN"); ui.strong("CAGR"); ui.strong("RF");
                                                                ui.strong("Max DD Days"); ui.strong("Curr DD Days"); ui.strong("Avg DD Days");
                                                                ui.end_row();
                                                                for det in &self.bg.account_details {
                                                                    ui.label(&det.ticker);
                                                                    let cc = if det.cagr >= 0.0 { UP } else { DOWN };
                                                                    ui.label(egui::RichText::new(format!("{:.2}%", det.cagr)).color(cc));
                                                                    ui.label(format!("{:.2}", det.recovery_factor));
                                                                    let (md, cd, ad) = det.dd_duration;
                                                                    ui.label(format!("{}", md));
                                                                    if cd > 0 {
                                                                        ui.label(egui::RichText::new(format!("{}", cd)).color(DOWN));
                                                                    } else {
                                                                        ui.label("0");
                                                                    }
                                                                    ui.label(format!("{:.0}", ad));
                                                                    ui.end_row();
                                                                }
                                                            });
                                                            // ── Risk Budget (VaR Decomposition) ──
                                                            if !self.bg.risk_budget.is_empty() {
                                                                ui.add_space(8.0);
                                                                ui.label(egui::RichText::new("Risk Budget (VaR Decomposition)").strong());
                                                                egui::Grid::new("risk_budget_grid").striped(true).num_columns(6).show(ui, |ui| {
                                                                    ui.label(egui::RichText::new("DARWIN").color(AXIS_TEXT).small().strong());
                                                                    ui.label(egui::RichText::new("Standalone").color(AXIS_TEXT).small().strong());
                                                                    ui.label(egui::RichText::new("Marginal").color(AXIS_TEXT).small().strong());
                                                                    ui.label(egui::RichText::new("Risk %").color(AXIS_TEXT).small().strong());
                                                                    ui.label(egui::RichText::new("Diversif.").color(AXIS_TEXT).small().strong());
                                                                    ui.label(egui::RichText::new("Status").color(AXIS_TEXT).small().strong());
                                                                    ui.end_row();
                                                                    for rb in &self.bg.risk_budget {
                                                                        let risk_c = if rb.risk_contribution_pct > 40.0 { DOWN } else if rb.risk_contribution_pct > 25.0 { egui::Color32::from_rgb(241, 196, 15) } else { UP };
                                                                        let div_c = if rb.diversification_benefit > 0.0 { UP } else { DOWN };
                                                                        let status = if rb.diversification_benefit > 0.0 { "DIVERSIFIES" } else { "CONCENTRATES" };
                                                                        ui.label(egui::RichText::new(&rb.darwin_ticker).small());
                                                                        {
                                                                            let sv_c = if rb.standalone_var >= 3.25 && rb.standalone_var <= 6.5 { UP } else { DOWN };
                                                                            ui.label(egui::RichText::new(format!("{:.2}%", rb.standalone_var)).color(sv_c).small());
                                                                        }
                                                                        {
                                                                            let mv_c = if rb.marginal_var >= 3.25 && rb.marginal_var <= 6.5 { UP } else { DOWN };
                                                                            ui.label(egui::RichText::new(format!("{:.2}%", rb.marginal_var)).color(mv_c).small());
                                                                        }
                                                                        ui.label(egui::RichText::new(format!("{:.1}%", rb.risk_contribution_pct)).color(risk_c).small());
                                                                        ui.label(egui::RichText::new(format!("{:+.2}%", rb.diversification_benefit)).color(div_c).small());
                                                                        ui.label(egui::RichText::new(status).color(div_c).small());
                                                                        ui.end_row();
                                                                    }
                                                                });
                                                            }
                                                        } // if let Some(vs)
                                                    } } // if !daily.is_empty()
                                                }
                                                2 => { // Equity Curves — all DARWINs overlaid + combined
                                                    let darwin_colors = [
                                                        egui::Color32::from_rgb(26, 188, 156),   // teal
                                                        egui::Color32::from_rgb(52, 152, 219),   // blue
                                                        egui::Color32::from_rgb(241, 196, 15),   // gold
                                                        egui::Color32::from_rgb(155, 89, 182),   // purple
                                                        egui::Color32::from_rgb(230, 126, 34),   // orange
                                                        egui::Color32::from_rgb(231, 76, 60),    // coral
                                                        egui::Color32::from_rgb(46, 204, 113),   // emerald
                                                        egui::Color32::from_rgb(149, 165, 166),  // silver
                                                    ];
                                                    // Combined equity (thick white line)
                                                    ui.label(egui::RichText::new("Combined + Per-DARWIN Equity Curves").strong());
                                                    {
                                                        let eq_curve = &self.bg.equity_curve;
                                                        let details = &self.bg.account_details;
                                                        Plot::new("equity_overlay_plot")
                                                            .height(300.0)
                                                            .allow_drag(false).allow_zoom(false).allow_scroll(false)
                                                            .legend(egui_plot::Legend::default())
                                                            .show(ui, |plot_ui| {
                                                                // Combined equity — bold white
                                                                if eq_curve.len() > 2 {
                                                                    let pts: PlotPoints = PlotPoints::new(
                                                                        eq_curve.iter().enumerate().map(|(i, (_, b))| [i as f64, *b]).collect()
                                                                    );
                                                                    plot_ui.line(Line::new("Combined", pts).color(egui::Color32::WHITE).width(2.0));
                                                                }
                                                                // Per-DARWIN equity curves
                                                                for (idx, det) in details.iter().enumerate() {
                                                                    if det.equity_curve.len() > 2 {
                                                                        let c = darwin_colors[idx % darwin_colors.len()];
                                                                        let pts: PlotPoints = PlotPoints::new(
                                                                            det.equity_curve.iter().enumerate().map(|(i, (_, b))| [i as f64, *b]).collect()
                                                                        );
                                                                        plot_ui.line(Line::new(&det.ticker, pts).color(c).width(1.2));
                                                                    }
                                                                }
                                                            });
                                                    }
                                                    // DARWIN Quote Equity Curves (FTP — investor product price)
                                                    {
                                                        let has_ftp = self.bg.account_details.iter().any(|d| !d.ftp_equity_curve.is_empty());
                                                        if has_ftp {
                                                            ui.add_space(10.0);
                                                            ui.label(egui::RichText::new("DARWIN Quote Price (Investor View)").strong());
                                                            Plot::new("quote_equity_plot")
                                                                .height(200.0)
                                                                .allow_drag(false).allow_zoom(false).allow_scroll(false)
                                                                .legend(egui_plot::Legend::default())
                                                                .show(ui, |plot_ui| {
                                                                    for (idx, det) in self.bg.account_details.iter().enumerate() {
                                                                        if det.ftp_equity_curve.len() > 2 {
                                                                            let c = darwin_colors[idx % darwin_colors.len()];
                                                                            let pts: PlotPoints = PlotPoints::new(
                                                                                det.ftp_equity_curve.iter().map(|&(x, y)| [x, y]).collect()
                                                                            );
                                                                            plot_ui.line(Line::new(format!("{} Quote", det.ticker), pts).color(c).width(1.5));
                                                                        }
                                                                    }
                                                                    // Reference line at 100 (starting price)
                                                                    let max_days = self.bg.account_details.iter()
                                                                        .map(|d| d.ftp_equity_curve.len()).max().unwrap_or(100) as f64;
                                                                    plot_ui.hline(egui_plot::HLine::new("Start 100", 100.0).color(egui::Color32::from_rgb(80, 80, 100)).width(0.5));
                                                                    let _ = max_days; // used implicitly by hline domain
                                                                });

                                                            // DARWIN Quote Drawdown
                                                            ui.add_space(6.0);
                                                            ui.label(egui::RichText::new("DARWIN Quote Drawdown").strong());
                                                            Plot::new("quote_drawdown_plot")
                                                                .height(150.0)
                                                                .allow_drag(false).allow_zoom(false).allow_scroll(false)
                                                                .legend(egui_plot::Legend::default())
                                                                .show(ui, |plot_ui| {
                                                                    for (idx, det) in self.bg.account_details.iter().enumerate() {
                                                                        if det.ftp_drawdown_curve.len() > 2 {
                                                                            let c = darwin_colors[idx % darwin_colors.len()];
                                                                            let pts: PlotPoints = PlotPoints::new(
                                                                                det.ftp_drawdown_curve.iter().map(|&(x, y)| [x, y]).collect()
                                                                            );
                                                                            plot_ui.line(Line::new(format!("{} DD", det.ticker), pts).color(c).width(1.2));
                                                                        }
                                                                    }
                                                                    plot_ui.hline(egui_plot::HLine::new("Zero", 0.0).color(egui::Color32::from_rgb(80, 80, 100)).width(0.5));
                                                                });
                                                        }
                                                    }

                                                    // Divergence Index plot (Signal vs Quote return divergence over time)
                                                    {
                                                        let has_divergence = self.bg.account_details.iter().any(|d| !d.divergence.is_empty());
                                                        if has_divergence {
                                                            ui.add_space(10.0);
                                                            ui.label(egui::RichText::new("Signal vs Quote Divergence").strong());
                                                            Plot::new("divergence_plot")
                                                                .height(180.0)
                                                                .allow_drag(false).allow_zoom(false).allow_scroll(false)
                                                                .legend(egui_plot::Legend::default())
                                                                .show(ui, |plot_ui| {
                                                                    for (idx, det) in self.bg.account_details.iter().enumerate() {
                                                                        if det.divergence.len() > 2 {
                                                                            let c = darwin_colors[idx % darwin_colors.len()];
                                                                            let pts: PlotPoints = PlotPoints::new(
                                                                                det.divergence.iter().map(|d| [d.day_index as f64, d.divergence_pct]).collect()
                                                                            );
                                                                            plot_ui.line(Line::new(format!("{} Div", det.ticker), pts).color(c).width(1.5));
                                                                        }
                                                                    }
                                                                    plot_ui.hline(egui_plot::HLine::new("Zero", 0.0).color(egui::Color32::from_rgb(80, 80, 100)).width(0.5));
                                                                });
                                                        }
                                                    }

                                                    // Per-DARWIN individual equity curves: Signal vs Quote side-by-side
                                                    ui.add_space(6.0);
                                                    ui.label(egui::RichText::new("Signal vs Quote — Per-DARWIN").small().strong());
                                                    for (idx, det) in self.bg.account_details.iter().enumerate() {
                                                        if det.equity_curve.len() > 2 {
                                                            let c = darwin_colors[idx % darwin_colors.len()];
                                                            // Signal equity (solid)
                                                            let pts: PlotPoints = PlotPoints::new(
                                                                det.equity_curve.iter().enumerate().map(|(i, (_, b))| [i as f64, *b]).collect()
                                                            );
                                                            let signal_line = Line::new(format!("{} Signal", det.ticker), pts).color(c).width(1.5);
                                                            // Quote equity (dashed, same color but dimmer)
                                                            let has_quote = det.ftp_equity_curve.len() > 2;
                                                            Plot::new(format!("eq_ind_{}", det.ticker))
                                                                .height(100.0)
                                                                .allow_drag(false).allow_zoom(false).allow_scroll(false)
                                                                .legend(egui_plot::Legend::default())
                                                                .show_axes([false, true])
                                                                .show(ui, |plot_ui| {
                                                                    plot_ui.line(signal_line);
                                                                    if has_quote {
                                                                        let qpts: PlotPoints = PlotPoints::new(
                                                                            det.ftp_equity_curve.iter().map(|&(x, y)| [x, y]).collect()
                                                                        );
                                                                        let qc = egui::Color32::from_rgba_premultiplied(c.r() / 2 + 128, c.g() / 2 + 128, c.b() / 2 + 128, 180);
                                                                        plot_ui.line(Line::new(format!("{} Quote", det.ticker), qpts).color(qc).width(1.0));
                                                                    }
                                                                });
                                                            // Drawdown subplot for this DARWIN
                                                            if !det.ftp_drawdown_curve.is_empty() {
                                                                let dd_pts: PlotPoints = PlotPoints::new(
                                                                    det.ftp_drawdown_curve.iter().map(|&(x, y)| [x, y]).collect()
                                                                );
                                                                Plot::new(format!("dd_ind_{}", det.ticker))
                                                                    .height(60.0)
                                                                    .allow_drag(false).allow_zoom(false).allow_scroll(false)
                                                                    .show_axes([false, true])
                                                                    .show(ui, |plot_ui| {
                                                                        plot_ui.line(Line::new("Drawdown", dd_pts).color(DOWN).width(1.0).fill(-100.0));
                                                                        plot_ui.hline(egui_plot::HLine::new("Zero", 0.0).color(egui::Color32::from_rgb(60, 60, 80)).width(0.5));
                                                                    });
                                                            }
                                                        }
                                                    }

                                                    // Investor Flow (AUM & Count)
                                                    ui.add_space(10.0);
                                                    ui.label(egui::RichText::new("Investor Flow (AUM & Count)").strong());
                                                    ui.separator();
                                                    {
                                                        let has_any_ftp = self.bg.account_details.iter().any(|d| d.ftp_summary.is_some());
                                                        if has_any_ftp {
                                                            for det in &self.bg.account_details {
                                                                if det.ftp_summary.is_some() {
                                                                    ui.label(egui::RichText::new(format!("{} — Load via DarwinIA Browser for investor flow data", det.ticker)).color(AXIS_TEXT).small());
                                                                }
                                                            }
                                                        } else {
                                                            ui.label(egui::RichText::new("No FTP data loaded. Import DARWIN data first.").color(AXIS_TEXT));
                                                        }
                                                    }

                                                    // Per-DARWIN Monthly Returns
                                                    ui.add_space(10.0);
                                                    ui.label(egui::RichText::new("Per-DARWIN Monthly Returns").strong());
                                                    ui.separator();
                                                    for (idx, det) in self.bg.account_details.iter().enumerate() {
                                                        if !det.monthly_returns.is_empty() {
                                                            let c = darwin_colors[idx % darwin_colors.len()];
                                                            ui.label(egui::RichText::new(&det.ticker).color(c).strong());
                                                            // Show last 24 months as bar chart
                                                            let monthly: Vec<&darwin::MonthlyReturn> = det.monthly_returns.iter().rev().take(24).collect::<Vec<_>>().into_iter().rev().collect();
                                                            let bars: Vec<PlotBar> = monthly.iter().enumerate().map(|(i, m)| {
                                                                let bar_c = if m.pnl >= 0.0 { UP } else { DOWN };
                                                                PlotBar::new(i as f64, m.pnl).width(0.8).fill(bar_c).name(format!("{}/{:02}", m.year, m.month))
                                                            }).collect();
                                                            Plot::new(format!("monthly_pnl_{}", det.ticker))
                                                                .height(80.0)
                                                                .allow_drag(false).allow_zoom(false).allow_scroll(false)
                                                                .show_axes([false, true])
                                                                .show(ui, |plot_ui| {
                                                                    plot_ui.bar_chart(BarChart::new("Monthly P&L", bars));
                                                                    plot_ui.hline(egui_plot::HLine::new("Zero", 0.0).color(egui::Color32::from_rgb(60, 60, 80)).width(0.5));
                                                                });
                                                            // Summary grid: last 6 months
                                                            let recent: Vec<&darwin::MonthlyReturn> = det.monthly_returns.iter().rev().take(6).collect::<Vec<_>>().into_iter().rev().collect();
                                                            egui::Grid::new(format!("monthly_grid_{}", det.ticker)).striped(true).num_columns(6).show(ui, |ui| {
                                                                for m in &recent {
                                                                    ui.label(egui::RichText::new(format!("{}/{:02}", m.year, m.month)).small());
                                                                }
                                                                ui.end_row();
                                                                for m in &recent {
                                                                    let mc = if m.pnl >= 0.0 { UP } else { DOWN };
                                                                    ui.label(egui::RichText::new(format!("${:.0}", m.pnl)).color(mc).small());
                                                                }
                                                                ui.end_row();
                                                                for m in &recent {
                                                                    let rc = if m.return_pct >= 0.0 { UP } else { DOWN };
                                                                    ui.label(egui::RichText::new(format!("{:.1}%", m.return_pct)).color(rc).small());
                                                                }
                                                                ui.end_row();
                                                            });
                                                            ui.add_space(4.0);
                                                        }
                                                    }
                                                }
                                                3 => { // Correlation Matrix (from bg cache)
                                                    // ── Visual Correlation Heatmap ──
                                                    {
                                                        let tickers: Vec<&str> = self.bg.accounts.iter().map(|a| a.darwin_ticker.as_str()).collect();
                                                        let n = tickers.len();
                                                        if n >= 2 {
                                                            ui.label(egui::RichText::new("Correlation Heatmap").strong());
                                                            let cell_size = 40.0_f32;
                                                            let label_w = 50.0_f32;

                                                            // Header row
                                                            ui.horizontal(|ui| {
                                                                ui.add_space(label_w);
                                                                for t in &tickers {
                                                                    ui.add_sized([cell_size, 14.0], egui::Label::new(
                                                                        egui::RichText::new(*t).small().monospace().color(AXIS_TEXT)
                                                                    ));
                                                                }
                                                            });

                                                            // Matrix rows
                                                            for (i, row_ticker) in tickers.iter().enumerate() {
                                                                ui.horizontal(|ui| {
                                                                    ui.add_sized([label_w, cell_size], egui::Label::new(
                                                                        egui::RichText::new(*row_ticker).small().monospace().strong()
                                                                    ));
                                                                    for (j, col_ticker) in tickers.iter().enumerate() {
                                                                        let corr = if i == j { 1.0 } else {
                                                                            self.bg.correlations.iter()
                                                                                .find(|c| (c.darwin_a == *row_ticker && c.darwin_b == *col_ticker) ||
                                                                                          (c.darwin_b == *row_ticker && c.darwin_a == *col_ticker))
                                                                                .map(|c| c.correlation)
                                                                                .unwrap_or(0.0)
                                                                        };

                                                                        // Color: red for high (>0.7), yellow for moderate, green for low, blue for negative
                                                                        let color = if corr > 0.95 { egui::Color32::from_rgb(231, 76, 60) }     // danger red
                                                                            else if corr > 0.7 { egui::Color32::from_rgb(230, 126, 34) }         // orange
                                                                            else if corr > 0.3 { egui::Color32::from_rgb(241, 196, 15) }         // yellow
                                                                            else if corr > -0.3 { egui::Color32::from_rgb(46, 204, 113) }        // green
                                                                            else { egui::Color32::from_rgb(52, 152, 219) };                       // blue (negative)

                                                                        let (rect, _) = ui.allocate_exact_size(egui::vec2(cell_size, cell_size), egui::Sense::hover());
                                                                        ui.painter().rect_filled(rect, 2.0, color);
                                                                        // Text overlay
                                                                        ui.painter().text(
                                                                            rect.center(),
                                                                            egui::Align2::CENTER_CENTER,
                                                                            format!("{:.2}", corr),
                                                                            egui::FontId::monospace(9.0),
                                                                            egui::Color32::WHITE,
                                                                        );
                                                                    }
                                                                });
                                                            }
                                                            ui.add_space(8.0);
                                                            ui.separator();
                                                        }
                                                    }
                                                    // ── Text Correlation Table ──
                                                    { let corrs = &self.bg.correlations; if !corrs.is_empty() {
                                                        egui::Grid::new("corr_grid").striped(true).num_columns(3).show(ui, |ui| {
                                                            ui.strong("DARWIN A"); ui.strong("DARWIN B"); ui.strong("Correlation");
                                                            ui.end_row();
                                                            for c in corrs.iter() {
                                                                ui.label(&c.darwin_a); ui.label(&c.darwin_b);
                                                                let color = if c.correlation.abs() > 0.95 { egui::Color32::from_rgb(255, 80, 80) }
                                                                            else if c.correlation.abs() > 0.7 { egui::Color32::from_rgb(255, 200, 50) }
                                                                            else { UP };
                                                                ui.label(egui::RichText::new(format!("{:.4}", c.correlation)).color(color));
                                                                ui.end_row();
                                                            }
                                                        });
                                                    } }
                                                }
                                                4 => { // Symbol Exposure (from bg cache)
                                                    { let exposure = &self.bg.exposure; if !exposure.is_empty() {
                                                        egui::Grid::new("exp_grid").striped(true).num_columns(5).show(ui, |ui| {
                                                            ui.strong("Symbol"); ui.strong("Long $"); ui.strong("Short $"); ui.strong("Net $"); ui.strong("DARWINs");
                                                            ui.end_row();
                                                            for e in exposure.iter() {
                                                                ui.label(&e.symbol);
                                                                ui.label(format!("{:.0}", e.long_notional));
                                                                ui.label(format!("{:.0}", e.short_notional));
                                                                let net_c = if e.net_notional >= 0.0 { UP } else { DOWN };
                                                                ui.label(egui::RichText::new(format!("{:.0}", e.net_notional)).color(net_c));
                                                                ui.label(e.darwins.join(", "));
                                                                ui.end_row();
                                                            }
                                                        });
                                                    } }
                                                    // Exposure Treemap (flattened) — from bg cache
                                                    ui.add_space(10.0);
                                                    ui.heading("Exposure by Sector");
                                                    ui.separator();
                                                    if let Some(ref tree) = self.bg.exposure_treemap {
                                                        for child in &tree.children {
                                                            let sector_c = if child.color_value > 0.0 { UP } else if child.color_value < 0.0 { DOWN } else { AXIS_TEXT };
                                                            ui.label(egui::RichText::new(format!("{}: ${:.0}", child.name, child.value)).color(sector_c).strong());
                                                            for sym in &child.children {
                                                                let sc = if sym.color_value > 0.0 { UP } else { DOWN };
                                                                ui.label(egui::RichText::new(format!("  {} ${:.0}", sym.name, sym.value)).color(sc).small());
                                                            }
                                                        }
                                                    }
                                                }
                                                5 => { // Combined Positions (from bg cache)
                                                    { let positions = &self.bg.open_positions;
                                                        if positions.is_empty() {
                                                            ui.label(egui::RichText::new("No open positions.").color(AXIS_TEXT));
                                                        } else {
                                                            egui::Grid::new("cpos_grid").striped(true).num_columns(5).show(ui, |ui| {
                                                                ui.strong("Symbol"); ui.strong("Side"); ui.strong("Volume"); ui.strong("Avg Price"); ui.strong("DARWINs");
                                                                ui.end_row();
                                                                for pos in positions.iter() {
                                                                    ui.label(&pos.symbol);
                                                                    let side_c = if pos.side == "buy" { UP } else { DOWN };
                                                                    ui.label(egui::RichText::new(&pos.side).color(side_c));
                                                                    ui.label(format!("{:.2}", pos.total_volume));
                                                                    ui.label(format_price(pos.avg_price));
                                                                    let darwins: Vec<&str> = pos.darwin_breakdown.iter().map(|(d, _, _)| d.as_str()).collect();
                                                                    ui.label(darwins.join(", "));
                                                                    ui.end_row();
                                                                }
                                                            });
                                                        }
                                                    }
                                                }
                                                6 => { // Trade Overlaps (from bg cache)
                                                    { let overlaps = &self.bg.trade_overlaps;
                                                        if overlaps.is_empty() {
                                                            ui.label("No trade overlaps found.");
                                                        } else {
                                                            egui::Grid::new("overlap_grid").striped(true).num_columns(4).show(ui, |ui| {
                                                                ui.strong("Symbol"); ui.strong("DARWINs"); ui.strong("Volume"); ui.strong("Notional");
                                                                ui.end_row();
                                                                for o in overlaps.iter() {
                                                                    ui.label(&o.symbol);
                                                                    ui.label(o.darwins.join(", "));
                                                                    ui.label(format!("{:.2}", o.combined_volume));
                                                                    ui.label(format!("${:.0}", o.combined_notional));
                                                                    ui.end_row();
                                                                }
                                                            });
                                                        }
                                                    }
                                                }
                                                7 => { // Drawdown — comprehensive drawdown analytics + monthly gain/loss grid
                                                    let dd_colors = [
                                                        egui::Color32::from_rgb(26, 188, 156),
                                                        egui::Color32::from_rgb(52, 152, 219),
                                                        egui::Color32::from_rgb(241, 196, 15),
                                                        egui::Color32::from_rgb(155, 89, 182),
                                                        egui::Color32::from_rgb(230, 126, 34),
                                                        egui::Color32::from_rgb(231, 76, 60),
                                                        egui::Color32::from_rgb(46, 204, 113),
                                                        egui::Color32::from_rgb(149, 165, 166),
                                                    ];
                                                    if let Some(ref dd) = self.bg.drawdown_dashboard {
                                                        // ── Drawdown Curves Plot ──
                                                        ui.label(egui::RichText::new("Drawdown Curves (% from peak)").strong());
                                                        Plot::new("dd_overlay_plot")
                                                            .height(250.0)
                                                            .allow_drag(false).allow_zoom(false).allow_scroll(false)
                                                            .legend(egui_plot::Legend::default())
                                                            .show(ui, |plot_ui| {
                                                                if dd.combined.drawdown_curve.len() > 2 {
                                                                    let pts: PlotPoints = PlotPoints::new(
                                                                        dd.combined.drawdown_curve.iter().enumerate().map(|(i, d)| [i as f64, -d.drawdown_pct]).collect()
                                                                    );
                                                                    plot_ui.line(Line::new("Combined", pts).color(egui::Color32::from_rgb(255, 60, 60)).width(2.0));
                                                                }
                                                                for (idx, darwin_dd) in dd.darwins.iter().enumerate() {
                                                                    if darwin_dd.drawdown_curve.len() > 2 {
                                                                        let c = dd_colors[idx % dd_colors.len()];
                                                                        let pts: PlotPoints = PlotPoints::new(
                                                                            darwin_dd.drawdown_curve.iter().enumerate().map(|(i, d)| [i as f64, -d.drawdown_pct]).collect()
                                                                        );
                                                                        plot_ui.line(Line::new(&darwin_dd.darwin_ticker, pts).color(c).width(1.2));
                                                                    }
                                                                }
                                                            });

                                                        // ── Per-DARWIN Drawdown Stats Table (Signal + Quote) ──
                                                        ui.add_space(8.0);
                                                        ui.label(egui::RichText::new("Per-DARWIN Drawdown Analytics").strong());
                                                        ui.add_space(2.0);
                                                        let dim = egui::Color32::from_rgb(100, 100, 120);
                                                        let dd_details_by_ticker: std::collections::HashMap<&str, &_> = self
                                                            .bg
                                                            .account_details
                                                            .iter()
                                                            .map(|d| (d.ticker.as_str(), d))
                                                            .collect();
                                                        egui::Grid::new("dd_stats_full").striped(true).num_columns(9).min_col_width(70.0).show(ui, |ui| {
                                                            ui.label(egui::RichText::new("DARWIN").color(dim).small());
                                                            ui.label(egui::RichText::new("Signal DD%").color(dim).small());
                                                            ui.label(egui::RichText::new("Sig DD Days").color(dim).small());
                                                            ui.label(egui::RichText::new("Quote DD%").color(dim).small());
                                                            ui.label(egui::RichText::new("Quote DD Days").color(dim).small());
                                                            ui.label(egui::RichText::new("Recovery F").color(dim).small());
                                                            ui.label(egui::RichText::new("Current DD%").color(dim).small());
                                                            ui.label(egui::RichText::new("Best Day").color(dim).small());
                                                            ui.label(egui::RichText::new("Worst Day").color(dim).small());
                                                            ui.end_row();

                                                            for darwin_dd in &dd.darwins {
                                                                let ticker = &darwin_dd.darwin_ticker;
                                                                // Find matching AccountDetailCache for this DARWIN
                                                                let det_opt = dd_details_by_ticker.get(ticker.as_str()).copied();

                                                                ui.label(egui::RichText::new(ticker).strong());

                                                                // Signal Max DD%
                                                                let sig_dd = if let Some(det) = det_opt {
                                                                    det.summary.as_ref().map(|s| s.max_drawdown_pct).unwrap_or(darwin_dd.max_drawdown_pct)
                                                                } else { darwin_dd.max_drawdown_pct };
                                                                ui.label(egui::RichText::new(format!("{:.2}%", sig_dd)).color(DOWN));

                                                                // Signal DD Days (max)
                                                                if let Some(det) = det_opt {
                                                                    let (max_dd_days, _cur_dd_days, _avg) = det.dd_duration;
                                                                    ui.label(egui::RichText::new(format!("{}", max_dd_days)).color(AXIS_TEXT));
                                                                } else {
                                                                    ui.label(egui::RichText::new("--").color(dim));
                                                                }

                                                                // Quote DD% (from FTP)
                                                                if let Some(det) = det_opt {
                                                                    if let Some(ref ftp) = det.ftp_summary {
                                                                        ui.label(egui::RichText::new(format!("{:.2}%", ftp.max_drawdown_pct)).color(DOWN));
                                                                    } else {
                                                                        ui.label(egui::RichText::new("--").color(dim));
                                                                    }
                                                                } else {
                                                                    ui.label(egui::RichText::new("--").color(dim));
                                                                }

                                                                // Quote DD Days — derive from ftp_drawdown_curve
                                                                if let Some(det) = det_opt {
                                                                    if !det.ftp_drawdown_curve.is_empty() {
                                                                        // Count consecutive days in drawdown from end
                                                                        let mut cur_q_dd_days = 0usize;
                                                                        for &(_x, dd_val) in det.ftp_drawdown_curve.iter().rev() {
                                                                            if dd_val < -0.01 { cur_q_dd_days += 1; } else { break; }
                                                                        }
                                                                        ui.label(egui::RichText::new(format!("{}", cur_q_dd_days)).color(AXIS_TEXT));
                                                                    } else {
                                                                        ui.label(egui::RichText::new("--").color(dim));
                                                                    }
                                                                } else {
                                                                    ui.label(egui::RichText::new("--").color(dim));
                                                                }

                                                                // Recovery Factor
                                                                if let Some(det) = det_opt {
                                                                    let rf = det.recovery_factor;
                                                                    let rf_c = if rf > 2.0 { UP } else if rf > 1.0 { AXIS_TEXT } else { DOWN };
                                                                    ui.label(egui::RichText::new(format!("{:.2}", rf)).color(rf_c));
                                                                } else {
                                                                    ui.label(egui::RichText::new("--").color(dim));
                                                                }

                                                                // Current DD%
                                                                let cur_c = if darwin_dd.current_drawdown_pct > 5.0 { DOWN } else { AXIS_TEXT };
                                                                ui.label(egui::RichText::new(format!("{:.2}%", darwin_dd.current_drawdown_pct)).color(cur_c));

                                                                // Best Day% / Worst Day% (from FTP if available, else signal best_days)
                                                                if let Some(det) = det_opt {
                                                                    if let Some(ref ftp) = det.ftp_summary {
                                                                        ui.label(egui::RichText::new(format!("{:+.2}%", ftp.best_day_pct)).color(UP));
                                                                        ui.label(egui::RichText::new(format!("{:.2}%", ftp.worst_day_pct)).color(DOWN));
                                                                    } else {
                                                                        let best = darwin_dd.best_days.first().map(|d| d.return_pct).unwrap_or(0.0);
                                                                        let worst = darwin_dd.worst_days.first().map(|d| d.return_pct).unwrap_or(0.0);
                                                                        ui.label(egui::RichText::new(format!("{:+.2}%", best)).color(UP));
                                                                        ui.label(egui::RichText::new(format!("{:.2}%", worst)).color(DOWN));
                                                                    }
                                                                } else {
                                                                    let best = darwin_dd.best_days.first().map(|d| d.return_pct).unwrap_or(0.0);
                                                                    let worst = darwin_dd.worst_days.first().map(|d| d.return_pct).unwrap_or(0.0);
                                                                    ui.label(egui::RichText::new(format!("{:+.2}%", best)).color(UP));
                                                                    ui.label(egui::RichText::new(format!("{:.2}%", worst)).color(DOWN));
                                                                }
                                                                ui.end_row();
                                                            }
                                                            // Combined row
                                                            ui.label(egui::RichText::new("COMBINED").strong());
                                                            ui.label(egui::RichText::new(format!("{:.2}%", dd.combined.max_drawdown_pct)).color(DOWN).strong());
                                                            ui.label(egui::RichText::new("").color(dim)); // no combined dd days
                                                            ui.label(egui::RichText::new("--").color(dim)); // no combined quote dd
                                                            ui.label(egui::RichText::new("--").color(dim));
                                                            ui.label(egui::RichText::new("--").color(dim));
                                                            let comb_cur_c = if dd.combined.current_drawdown_pct > 5.0 { DOWN } else { AXIS_TEXT };
                                                            ui.label(egui::RichText::new(format!("{:.2}%", dd.combined.current_drawdown_pct)).color(comb_cur_c).strong());
                                                            let comb_best = dd.combined.best_days.first().map(|d| d.return_pct).unwrap_or(0.0);
                                                            let comb_worst = dd.combined.worst_days.first().map(|d| d.return_pct).unwrap_or(0.0);
                                                            ui.label(egui::RichText::new(format!("{:+.2}%", comb_best)).color(UP));
                                                            ui.label(egui::RichText::new(format!("{:.2}%", comb_worst)).color(DOWN));
                                                            ui.end_row();
                                                        });

                                                        // ── Monthly Gain/Loss Grid (Darwinex-style heatmap) ──
                                                        ui.add_space(12.0);
                                                        ui.label(egui::RichText::new("Monthly Returns Grid").strong());
                                                        ui.add_space(4.0);
                                                        let month_names = ["Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"];

                                                        for (det_idx, det) in self.bg.account_details.iter().enumerate() {
                                                            if det.monthly_returns.is_empty() { continue; }
                                                            ui.add_space(6.0);
                                                            ui.label(egui::RichText::new(format!("{} Monthly Returns (Signal)", det.ticker)).strong().color(dd_colors[det_idx % dd_colors.len()]));

                                                            // Build year -> month -> return_pct map
                                                            let mut year_map: std::collections::BTreeMap<i32, [Option<f64>; 12]> = std::collections::BTreeMap::new();
                                                            for mr in &det.monthly_returns {
                                                                let entry = year_map.entry(mr.year).or_insert([None; 12]);
                                                                if mr.month >= 1 && mr.month <= 12 {
                                                                    entry[(mr.month - 1) as usize] = Some(mr.return_pct);
                                                                }
                                                            }

                                                            egui::Grid::new(format!("monthly_grid_{}", det.ticker)).striped(true).num_columns(14).min_col_width(46.0).show(ui, |ui| {
                                                                // Header row
                                                                ui.label(egui::RichText::new("Year").color(dim).small());
                                                                for m in &month_names {
                                                                    ui.label(egui::RichText::new(*m).color(dim).small());
                                                                }
                                                                ui.label(egui::RichText::new("Year").color(dim).small());
                                                                ui.end_row();

                                                                for (&year, months) in &year_map {
                                                                    ui.label(egui::RichText::new(format!("{}", year)).strong().small());
                                                                    let mut year_total = 0.0f64;
                                                                    let mut year_count = 0;
                                                                    for m_val in months.iter() {
                                                                        if let Some(ret) = m_val {
                                                                            let intensity = (ret.abs() / 5.0).min(1.0);
                                                                            let color = if *ret >= 0.0 {
                                                                                egui::Color32::from_rgb(
                                                                                    (40.0 + 215.0 * intensity) as u8,
                                                                                    (180.0 + 75.0 * intensity) as u8,
                                                                                    (40.0 + 40.0 * intensity) as u8,
                                                                                )
                                                                            } else {
                                                                                egui::Color32::from_rgb(
                                                                                    (180.0 + 75.0 * intensity) as u8,
                                                                                    (40.0 + 40.0 * intensity) as u8,
                                                                                    (40.0 + 40.0 * intensity) as u8,
                                                                                )
                                                                            };
                                                                            ui.label(egui::RichText::new(format!("{:.1}%", ret)).color(color).small());
                                                                            year_total += ret;
                                                                            year_count += 1;
                                                                        } else {
                                                                            ui.label(egui::RichText::new("").small());
                                                                        }
                                                                    }
                                                                    // Year total
                                                                    if year_count > 0 {
                                                                        let yr_c = if year_total >= 0.0 { UP } else { DOWN };
                                                                        ui.label(egui::RichText::new(format!("{:.1}%", year_total)).color(yr_c).strong().small());
                                                                    } else {
                                                                        ui.label(egui::RichText::new("").small());
                                                                    }
                                                                    ui.end_row();
                                                                }
                                                            });

                                                            // FTP Quote monthly returns (derived from ftp_equity_curve)
                                                            if !det.ftp_equity_curve.is_empty() && det.ftp_equity_curve.len() > 30 {
                                                                ui.add_space(4.0);
                                                                ui.label(egui::RichText::new(format!("{} Monthly Returns (Quote)", det.ticker)).small().strong().color(AXIS_TEXT));

                                                                // Derive monthly returns from FTP equity curve
                                                                // ftp_equity_curve is (day_index, quote_price) — approximate months as 21 trading days
                                                                let eq = &det.ftp_equity_curve;
                                                                let chunk_size = 21usize; // ~1 month of trading days
                                                                let mut ftp_months: Vec<(usize, f64)> = Vec::new(); // (month_idx, return_pct)
                                                                let mut i = 0usize;
                                                                let mut month_idx = 0usize;
                                                                while i + chunk_size <= eq.len() {
                                                                    let start_price = eq[i].1;
                                                                    let end_idx = (i + chunk_size - 1).min(eq.len() - 1);
                                                                    let end_price = eq[end_idx].1;
                                                                    if start_price > 0.0 {
                                                                        let ret = (end_price / start_price - 1.0) * 100.0;
                                                                        ftp_months.push((month_idx, ret));
                                                                    }
                                                                    i += chunk_size;
                                                                    month_idx += 1;
                                                                }
                                                                // Handle remaining days
                                                                if i < eq.len() && i > 0 {
                                                                    let start_price = eq[i - chunk_size.min(i)].1;
                                                                    let end_price = eq[eq.len() - 1].1;
                                                                    if start_price > 0.0 {
                                                                        let ret = (end_price / start_price - 1.0) * 100.0;
                                                                        ftp_months.push((month_idx, ret));
                                                                    }
                                                                }

                                                                if !ftp_months.is_empty() {
                                                                    // Display as a simple row of monthly returns
                                                                    ui.horizontal_wrapped(|ui| {
                                                                        for (idx, ret) in &ftp_months {
                                                                            let intensity = (ret.abs() / 5.0).min(1.0);
                                                                            let color = if *ret >= 0.0 {
                                                                                egui::Color32::from_rgb(
                                                                                    (40.0 + 215.0 * intensity) as u8,
                                                                                    (180.0 + 75.0 * intensity) as u8,
                                                                                    (40.0 + 40.0 * intensity) as u8,
                                                                                )
                                                                            } else {
                                                                                egui::Color32::from_rgb(
                                                                                    (180.0 + 75.0 * intensity) as u8,
                                                                                    (40.0 + 40.0 * intensity) as u8,
                                                                                    (40.0 + 40.0 * intensity) as u8,
                                                                                )
                                                                            };
                                                                            // Map chunk index to approximate calendar month using signal monthly_returns as reference
                                                                            let month_label = det.monthly_returns.get(*idx)
                                                                                .map(|m| {
                                                                                    let mon = ["Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"];
                                                                                    format!("{} {}", mon.get((m.month - 1) as usize).unwrap_or(&"?"), m.year % 100)
                                                                                })
                                                                                .unwrap_or_else(|| format!("M{}", idx + 1));
                                                                            ui.label(egui::RichText::new(format!("{}: {:.1}%", month_label, ret)).color(color).small());
                                                                        }
                                                                    });
                                                                }
                                                            }
                                                        }

                                                        // ── Best/Worst Analysis Table ──
                                                        ui.add_space(12.0);
                                                        ui.label(egui::RichText::new("Best/Worst Analysis").strong());
                                                        ui.add_space(2.0);
                                                        egui::Grid::new("best_worst_table").striped(true).num_columns(7).min_col_width(90.0).show(ui, |ui| {
                                                            ui.label(egui::RichText::new("DARWIN").color(dim).small());
                                                            ui.label(egui::RichText::new("Best Day").color(dim).small());
                                                            ui.label(egui::RichText::new("Worst Day").color(dim).small());
                                                            ui.label(egui::RichText::new("Best Week").color(dim).small());
                                                            ui.label(egui::RichText::new("Worst Week").color(dim).small());
                                                            ui.label(egui::RichText::new("Best Month").color(dim).small());
                                                            ui.label(egui::RichText::new("Worst Month").color(dim).small());
                                                            ui.end_row();

                                                            for det in &self.bg.account_details {
                                                                ui.label(egui::RichText::new(&det.ticker).strong());

                                                                // Best/Worst Day (from daily_returns)
                                                                if !det.daily_returns.is_empty() {
                                                                    let best_day = det.daily_returns.iter().max_by(|a, b| a.return_pct.partial_cmp(&b.return_pct).unwrap_or(std::cmp::Ordering::Equal));
                                                                    let worst_day = det.daily_returns.iter().min_by(|a, b| a.return_pct.partial_cmp(&b.return_pct).unwrap_or(std::cmp::Ordering::Equal));
                                                                    if let Some(bd) = best_day {
                                                                        let date_short = if bd.date.len() >= 10 { &bd.date[5..10] } else { &bd.date };
                                                                        ui.label(egui::RichText::new(format!("{:+.2}% {}", bd.return_pct, date_short)).color(UP).small());
                                                                    } else { ui.label(egui::RichText::new("--").color(dim).small()); }
                                                                    if let Some(wd) = worst_day {
                                                                        let date_short = if wd.date.len() >= 10 { &wd.date[5..10] } else { &wd.date };
                                                                        ui.label(egui::RichText::new(format!("{:.2}% {}", wd.return_pct, date_short)).color(DOWN).small());
                                                                    } else { ui.label(egui::RichText::new("--").color(dim).small()); }
                                                                } else {
                                                                    ui.label(egui::RichText::new("--").color(dim).small());
                                                                    ui.label(egui::RichText::new("--").color(dim).small());
                                                                }

                                                                // Best/Worst Week (from daily_returns, grouped by ISO week)
                                                                if det.daily_returns.len() >= 5 {
                                                                    let mut week_map: std::collections::BTreeMap<String, f64> = std::collections::BTreeMap::new();
                                                                    for dr in &det.daily_returns {
                                                                        // Parse date to get ISO week key
                                                                        if dr.date.len() >= 10 {
                                                                            let parts: Vec<&str> = dr.date[..10].split('-').collect();
                                                                            if parts.len() == 3 {
                                                                                if let (Ok(y), Ok(m), Ok(d)) = (parts[0].parse::<i32>(), parts[1].parse::<u32>(), parts[2].parse::<u32>()) {
                                                                                    // Approximate ISO week: day_of_year / 7
                                                                                    let doy = (m - 1) * 30 + d; // rough approximation
                                                                                    let week_num = doy / 7;
                                                                                    let key = format!("{}-W{:02}", y, week_num);
                                                                                    *week_map.entry(key).or_insert(0.0) += dr.return_pct;
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                    if !week_map.is_empty() {
                                                                        let best_wk = week_map.iter().max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal));
                                                                        let worst_wk = week_map.iter().min_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal));
                                                                        if let Some((wk, ret)) = best_wk {
                                                                            ui.label(egui::RichText::new(format!("{:+.2}% {}", ret, wk)).color(UP).small());
                                                                        } else { ui.label(egui::RichText::new("--").color(dim).small()); }
                                                                        if let Some((wk, ret)) = worst_wk {
                                                                            ui.label(egui::RichText::new(format!("{:.2}% {}", ret, wk)).color(DOWN).small());
                                                                        } else { ui.label(egui::RichText::new("--").color(dim).small()); }
                                                                    } else {
                                                                        ui.label(egui::RichText::new("--").color(dim).small());
                                                                        ui.label(egui::RichText::new("--").color(dim).small());
                                                                    }
                                                                } else {
                                                                    ui.label(egui::RichText::new("--").color(dim).small());
                                                                    ui.label(egui::RichText::new("--").color(dim).small());
                                                                }

                                                                // Best/Worst Month (from monthly_returns)
                                                                if !det.monthly_returns.is_empty() {
                                                                    let best_mo = det.monthly_returns.iter().max_by(|a, b| a.return_pct.partial_cmp(&b.return_pct).unwrap_or(std::cmp::Ordering::Equal));
                                                                    let worst_mo = det.monthly_returns.iter().min_by(|a, b| a.return_pct.partial_cmp(&b.return_pct).unwrap_or(std::cmp::Ordering::Equal));
                                                                    if let Some(bm) = best_mo {
                                                                        let mo_name = month_names.get((bm.month - 1).max(0) as usize).unwrap_or(&"???");
                                                                        ui.label(egui::RichText::new(format!("{:+.2}% {}{}", bm.return_pct, mo_name, bm.year % 100)).color(UP).small());
                                                                    } else { ui.label(egui::RichText::new("--").color(dim).small()); }
                                                                    if let Some(wm) = worst_mo {
                                                                        let mo_name = month_names.get((wm.month - 1).max(0) as usize).unwrap_or(&"???");
                                                                        ui.label(egui::RichText::new(format!("{:.2}% {}{}", wm.return_pct, mo_name, wm.year % 100)).color(DOWN).small());
                                                                    } else { ui.label(egui::RichText::new("--").color(dim).small()); }
                                                                } else {
                                                                    ui.label(egui::RichText::new("--").color(dim).small());
                                                                    ui.label(egui::RichText::new("--").color(dim).small());
                                                                }

                                                                ui.end_row();
                                                            }

                                                            // Combined row (from portfolio daily_returns)
                                                            ui.label(egui::RichText::new("COMBINED").strong());
                                                            // Best/Worst day combined
                                                            let comb_best_d = dd.combined.best_days.first();
                                                            let comb_worst_d = dd.combined.worst_days.first();
                                                            if let Some(bd) = comb_best_d {
                                                                let ds = if bd.date.len() >= 10 { &bd.date[5..10] } else { &bd.date };
                                                                ui.label(egui::RichText::new(format!("{:+.2}% {}", bd.return_pct, ds)).color(UP).small());
                                                            } else { ui.label(egui::RichText::new("--").color(dim).small()); }
                                                            if let Some(wd) = comb_worst_d {
                                                                let ds = if wd.date.len() >= 10 { &wd.date[5..10] } else { &wd.date };
                                                                ui.label(egui::RichText::new(format!("{:.2}% {}", wd.return_pct, ds)).color(DOWN).small());
                                                            } else { ui.label(egui::RichText::new("--").color(dim).small()); }

                                                            // Best/Worst week combined (from portfolio daily returns)
                                                            if !self.bg.daily_returns.is_empty() {
                                                                let mut comb_week_map: std::collections::BTreeMap<String, f64> = std::collections::BTreeMap::new();
                                                                for dr in &self.bg.daily_returns {
                                                                    if dr.date.len() >= 10 {
                                                                        let parts: Vec<&str> = dr.date[..10].split('-').collect();
                                                                        if parts.len() == 3 {
                                                                            if let (Ok(y), Ok(m), Ok(d)) = (parts[0].parse::<i32>(), parts[1].parse::<u32>(), parts[2].parse::<u32>()) {
                                                                                let doy = (m - 1) * 30 + d;
                                                                                let week_num = doy / 7;
                                                                                let key = format!("{}-W{:02}", y, week_num);
                                                                                *comb_week_map.entry(key).or_insert(0.0) += dr.return_pct;
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                let cbw = comb_week_map.iter().max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal));
                                                                let cww = comb_week_map.iter().min_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal));
                                                                if let Some((wk, ret)) = cbw {
                                                                    ui.label(egui::RichText::new(format!("{:+.2}% {}", ret, wk)).color(UP).small());
                                                                } else { ui.label(egui::RichText::new("--").color(dim).small()); }
                                                                if let Some((wk, ret)) = cww {
                                                                    ui.label(egui::RichText::new(format!("{:.2}% {}", ret, wk)).color(DOWN).small());
                                                                } else { ui.label(egui::RichText::new("--").color(dim).small()); }
                                                            } else {
                                                                ui.label(egui::RichText::new("--").color(dim).small());
                                                                ui.label(egui::RichText::new("--").color(dim).small());
                                                            }

                                                            // Best/Worst month combined — not directly available, skip
                                                            ui.label(egui::RichText::new("--").color(dim).small());
                                                            ui.label(egui::RichText::new("--").color(dim).small());
                                                            ui.end_row();
                                                        });

                                                        // ── Combined Best/Worst Days (Top 5, kept from original) ──
                                                        if !dd.combined.best_days.is_empty() || !dd.combined.worst_days.is_empty() {
                                                            ui.add_space(8.0);
                                                            ui.horizontal(|ui| {
                                                                ui.vertical(|ui| {
                                                                    ui.label(egui::RichText::new("Best Days (Combined)").small().strong().color(UP));
                                                                    for d in dd.combined.best_days.iter().take(5) {
                                                                        ui.label(egui::RichText::new(format!("{} +${:.0} ({:+.2}%)", d.date, d.pnl, d.return_pct)).color(UP).small());
                                                                    }
                                                                });
                                                                ui.vertical(|ui| {
                                                                    ui.label(egui::RichText::new("Worst Days (Combined)").small().strong().color(DOWN));
                                                                    for d in dd.combined.worst_days.iter().take(5) {
                                                                        ui.label(egui::RichText::new(format!("{} ${:.0} ({:.2}%)", d.date, d.pnl, d.return_pct)).color(DOWN).small());
                                                                    }
                                                                });
                                                            });
                                                        }
                                                        // Drawdown Attribution — which DARWIN caused the most damage?
                                                        if !self.bg.drawdown_attribution.is_empty() {
                                                            ui.add_space(8.0);
                                                            ui.label(egui::RichText::new("Drawdown Attribution (Who Caused the Pain?)").strong());
                                                            egui::Grid::new("dd_attribution").striped(true).num_columns(4).show(ui, |ui| {
                                                                ui.label(egui::RichText::new("DARWIN").color(AXIS_TEXT).small().strong());
                                                                ui.label(egui::RichText::new("Contribution").color(AXIS_TEXT).small().strong());
                                                                ui.label(egui::RichText::new("Own DD%").color(AXIS_TEXT).small().strong());
                                                                ui.label(egui::RichText::new("Weight@Peak").color(AXIS_TEXT).small().strong());
                                                                ui.end_row();
                                                                for attr in &self.bg.drawdown_attribution {
                                                                    let c = if attr.contribution_pct > 30.0 { DOWN } else if attr.contribution_pct > 15.0 { egui::Color32::from_rgb(241, 196, 15) } else { AXIS_TEXT };
                                                                    ui.label(egui::RichText::new(&attr.darwin_ticker).small());
                                                                    ui.label(egui::RichText::new(format!("{:.1}%", attr.contribution_pct)).color(c).small());
                                                                    ui.label(egui::RichText::new(format!("{:.1}%", attr.standalone_dd_pct)).color(DOWN).small());
                                                                    ui.label(egui::RichText::new(format!("{:.1}%", attr.weight_at_peak)).small());
                                                                    ui.end_row();
                                                                }
                                                            });
                                                        }
                                                        // ── Calendar P&L Heatmap (GitHub-style) ──
                                                        if !self.bg.daily_returns.is_empty() {
                                                            ui.add_space(8.0);
                                                            ui.label(egui::RichText::new("Daily P&L Calendar Heatmap").strong());

                                                            // Group by year
                                                            let mut by_year: std::collections::BTreeMap<i32, Vec<&darwin::DailyReturn>> = std::collections::BTreeMap::new();
                                                            for d in &self.bg.daily_returns {
                                                                if d.date.len() >= 4 {
                                                                    if let Ok(year) = d.date[..4].parse::<i32>() {
                                                                        by_year.entry(year).or_default().push(d);
                                                                    }
                                                                }
                                                            }

                                                            let cell_size = 8.0_f32;
                                                            let gap = 1.0_f32;

                                                            for (year, days) in by_year.iter().rev().take(3) { // last 3 years
                                                                ui.label(egui::RichText::new(format!("{}", year)).small().strong());
                                                                let (rect, _) = ui.allocate_exact_size(
                                                                    egui::vec2(53.0 * (cell_size + gap), 7.0 * (cell_size + gap)),
                                                                    egui::Sense::hover(),
                                                                );

                                                                for day in days {
                                                                    // Parse date to get week_of_year and day_of_week
                                                                    if let Ok(date) = chrono::NaiveDate::parse_from_str(&day.date, "%Y-%m-%d") {
                                                                        use chrono::Datelike;
                                                                        let week = date.iso_week().week() as f32;
                                                                        let dow = date.weekday().num_days_from_monday() as f32;

                                                                        let x = rect.left() + week * (cell_size + gap);
                                                                        let y = rect.top() + dow * (cell_size + gap);

                                                                        let intensity = (day.return_pct.abs() as f32 / 2.0).min(1.0); // normalize, cap at 2%
                                                                        let color = if day.pnl >= 0.0 {
                                                                            egui::Color32::from_rgb(
                                                                                (20.0 + 30.0 * intensity) as u8,
                                                                                (60.0 + 195.0 * intensity) as u8,
                                                                                (20.0 + 30.0 * intensity) as u8,
                                                                            )
                                                                        } else {
                                                                            egui::Color32::from_rgb(
                                                                                (60.0 + 195.0 * intensity) as u8,
                                                                                (20.0 + 30.0 * intensity) as u8,
                                                                                (20.0 + 30.0 * intensity) as u8,
                                                                            )
                                                                        };

                                                                        let cell_rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(cell_size, cell_size));
                                                                        ui.painter().rect_filled(cell_rect, 1.0, color);
                                                                    }
                                                                }
                                                                ui.add_space(4.0);
                                                            }
                                                        }
                                                    } else {
                                                        ui.label(egui::RichText::new("Import DARWIN data first.").color(AXIS_TEXT));
                                                    }
                                                }
                                                8 => { // Monte Carlo (from bg cache + GPU)
                                                    let mc_green = egui::Color32::from_rgb(46, 204, 113);
                                                    let mc_red = egui::Color32::from_rgb(231, 76, 60);
                                                    let mc_gold = egui::Color32::from_rgb(241, 196, 15);
                                                    let mc_dim = egui::Color32::from_rgb(100, 100, 120);

                                                    if let Some(ref mc) = self.bg.monte_carlo {
                                                        ui.label(egui::RichText::new("Monte Carlo Simulation").strong());

                                                        // Compact metrics in two columns
                                                        egui::Grid::new("mc_grid").striped(true).num_columns(4).show(ui, |ui| {
                                                            ui.label(egui::RichText::new("Trading Days:").color(mc_dim).small());
                                                            ui.label(format!("{}", mc.days_forward));
                                                            ui.label(egui::RichText::new("Simulations:").color(mc_dim).small());
                                                            ui.label(format!("{}", mc.simulations));
                                                            ui.end_row();
                                                            ui.label(egui::RichText::new("VaR 95%:").color(mc_dim).small());
                                                            ui.label(egui::RichText::new(format!("{:.2}%", mc.var_95)).color(mc_red));
                                                            ui.label(egui::RichText::new("VaR 99%:").color(mc_dim).small());
                                                            ui.label(egui::RichText::new(format!("{:.2}%", mc.var_99)).color(mc_red));
                                                            ui.end_row();
                                                            ui.label(egui::RichText::new("Median:").color(mc_dim).small());
                                                            let med_c = if mc.median_outcome >= 0.0 { mc_green } else { mc_red };
                                                            ui.label(egui::RichText::new(format!("{:.2}%", mc.median_outcome)).color(med_c));
                                                            ui.label(egui::RichText::new("Prob Loss:").color(mc_dim).small());
                                                            let pl_c = if mc.probability_of_loss > 0.5 { mc_red } else { mc_green };
                                                            ui.label(egui::RichText::new(format!("{:.1}%", mc.probability_of_loss * 100.0)).color(pl_c));
                                                            ui.end_row();
                                                            ui.label(egui::RichText::new("Best Case:").color(mc_dim).small());
                                                            ui.label(egui::RichText::new(format!("{:.2}%", mc.best_case)).color(mc_green));
                                                            ui.label(egui::RichText::new("Worst Case:").color(mc_dim).small());
                                                            ui.label(egui::RichText::new(format!("{:.2}%", mc.worst_case)).color(mc_red));
                                                            ui.end_row();
                                                        });

                                                        // Outcome distribution as bar chart (if percentiles available)
                                                        if !mc.percentiles.is_empty() {
                                                            ui.add_space(6.0);
                                                            ui.label(egui::RichText::new("Outcome Distribution").small().strong());
                                                            let bars: Vec<PlotBar> = mc.percentiles.iter().enumerate().map(|(i, (pct, val))| {
                                                                let c = if *val >= 0.0 { mc_green } else { mc_red };
                                                                PlotBar::new(i as f64, *val).width(0.8).fill(c).name(format!("{}th", pct))
                                                            }).collect();
                                                            let chart = BarChart::new("MC Distribution", bars);
                                                            Plot::new("mc_dist").height(150.0)
                                                                .allow_drag(false).allow_zoom(false).allow_scroll(false)
                                                                .show_axes([false, true])
                                                                .show(ui, |plot_ui| { plot_ui.bar_chart(chart); });
                                                        }

                                                        // VaR confidence band visualization
                                                        ui.add_space(6.0);
                                                        let w = ui.available_width();
                                                        let (rect, _) = ui.allocate_exact_size(egui::vec2(w, 40.0), egui::Sense::hover());
                                                        let painter = ui.painter_at(rect);
                                                        painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(20, 20, 35));
                                                        // Draw confidence bands
                                                        let scale = |v: f64| -> f32 { ((v + 50.0) / 100.0 * w as f64) as f32 + rect.left() };
                                                        // 99% band
                                                        let x99_lo = scale(mc.worst_case);
                                                        let x99_hi = scale(mc.best_case);
                                                        painter.rect_filled(egui::Rect::from_min_max(
                                                            egui::pos2(x99_lo, rect.top()), egui::pos2(x99_hi, rect.bottom())),
                                                            0.0, egui::Color32::from_rgba_premultiplied(100, 100, 180, 30));
                                                        // 95% band
                                                        let x95_lo = scale(mc.var_95);
                                                        let x95_hi = scale(mc.median_outcome * 2.0 - mc.var_95); // approx symmetric
                                                        painter.rect_filled(egui::Rect::from_min_max(
                                                            egui::pos2(x95_lo, rect.top() + 5.0), egui::pos2(x95_hi, rect.bottom() - 5.0)),
                                                            0.0, egui::Color32::from_rgba_premultiplied(100, 180, 100, 40));
                                                        // Median line
                                                        let x_med = scale(mc.median_outcome);
                                                        painter.vline(x_med, egui::Rangef::new(rect.top(), rect.bottom()), egui::Stroke::new(2.0, mc_gold));
                                                        // Zero line
                                                        let x_zero = scale(0.0);
                                                        painter.vline(x_zero, egui::Rangef::new(rect.top(), rect.bottom()), egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 60, 60)));
                                                        // Labels
                                                        painter.text(egui::pos2(x_med + 3.0, rect.top() + 2.0), egui::Align2::LEFT_TOP,
                                                            format!("Median {:.1}%", mc.median_outcome), egui::FontId::monospace(9.0), mc_gold);
                                                        painter.text(egui::pos2(x95_lo, rect.bottom() - 2.0), egui::Align2::LEFT_BOTTOM,
                                                            format!("VaR95 {:.1}%", mc.var_95), egui::FontId::monospace(9.0), mc_red);
                                                    } else {
                                                        ui.label(egui::RichText::new("Need 30+ daily returns for Monte Carlo. Import DARWIN data.").color(AXIS_TEXT));
                                                    }
                                                }
                                                9 => { // Stress Test (from bg cache)
                                                    { let results = &self.bg.stress_tests; if !results.is_empty() {
                                                        egui::Grid::new("stress_grid").striped(true).num_columns(4).show(ui, |ui| {
                                                            ui.strong("Scenario"); ui.strong("Market Drop"); ui.strong("Portfolio Impact"); ui.strong("Impact %");
                                                            ui.end_row();
                                                            for r in results.iter() {
                                                                ui.label(&r.scenario);
                                                                ui.label(egui::RichText::new(format!("{:.1}%", r.market_drop_pct)).color(DOWN));
                                                                ui.label(egui::RichText::new(format!("${:.0}", r.estimated_portfolio_impact)).color(DOWN));
                                                                ui.label(egui::RichText::new(format!("{:.1}%", r.estimated_portfolio_impact_pct)).color(DOWN));
                                                                ui.end_row();
                                                            }
                                                        });
                                                    } }
                                                    // Timing divergences — from bg cache
                                                    ui.add_space(10.0);
                                                    ui.heading("Timing Divergences");
                                                    ui.separator();
                                                    { let divs = &self.bg.timing_divergences;
                                                        if divs.is_empty() {
                                                            ui.label("No timing divergences found.");
                                                        } else {
                                                            for d in divs {
                                                                ui.label(egui::RichText::new(format!("{}: spread {:.1}h, price {:.2}%", d.symbol, d.time_spread_hours, d.price_spread_pct)).small());
                                                            }
                                                        }
                                                    }
                                                }
                                                10 => { // VaR Forecast — from bg cache
                                                    if let Some(ref forecast) = self.bg.var_forecast {
                                                            egui::Grid::new("var_fc").striped(true).num_columns(2).show(ui, |ui| {
                                                                ui.label("Current VaR 95%:"); ui.label(format!("{:.2}%", forecast.current_var_95));
                                                                ui.end_row();
                                                                ui.label("Projected 30d:"); ui.label(format!("{:.2}%", forecast.projected_30d));
                                                                ui.end_row();
                                                                ui.label("Projected 60d:"); ui.label(format!("{:.2}%", forecast.projected_60d));
                                                                ui.end_row();
                                                                ui.label("Projected 90d:"); ui.label(format!("{:.2}%", forecast.projected_90d));
                                                                ui.end_row();
                                                                ui.label("VaR Trend:"); ui.label(&forecast.var_trend);
                                                                ui.end_row();
                                                                if let Some(days) = forecast.days_until_threshold {
                                                                    ui.label("Days to Threshold:"); ui.label(egui::RichText::new(format!("{}", days)).color(if days < 30 { DOWN } else { AXIS_TEXT }));
                                                                    ui.end_row();
                                                                }
                                                            });
                                                    }
                                                }
                                                11 => { // Conditional VaR — from bg cache
                                                    { let cvar = &self.bg.conditional_var; if !cvar.is_empty() {
                                                        egui::Grid::new("cvar_grid").striped(true).num_columns(4).show(ui, |ui| {
                                                            ui.strong("Regime"); ui.strong("VaR 95%"); ui.strong("VaR 99%"); ui.strong("Days"); ui.strong("Sharpe");
                                                            ui.end_row();
                                                            for cv in cvar {
                                                                ui.label(&cv.regime);
                                                                ui.label(format!("{:.2}%", cv.var_95));
                                                                ui.label(format!("{:.2}%", cv.var_99));
                                                                ui.label(format!("{}", cv.days_in_regime));
                                                                ui.label(format!("{:.3}", cv.sharpe));
                                                                ui.end_row();
                                                            }
                                                        });
                                                    } }
                                                }
                                                12 => { // Market Regime — from bg cache
                                                    if let Some(ref regime) = self.bg.market_regime {
                                                        egui::Grid::new("regime_grid").striped(true).num_columns(2).show(ui, |ui| {
                                                            ui.label("Current Regime:"); ui.label(egui::RichText::new(&regime.current_regime).strong());
                                                            ui.end_row();
                                                            ui.label("Since:"); ui.label(&regime.regime_start);
                                                            ui.end_row();
                                                            ui.label("Duration:"); ui.label(format!("{} days", regime.regime_duration_days));
                                                            ui.end_row();
                                                            ui.label("Rolling Vol:"); ui.label(format!("{:.4}", regime.rolling_vol));
                                                            ui.end_row();
                                                            ui.label("Vol Percentile:"); ui.label(format!("{:.1}%", regime.vol_percentile));
                                                            ui.end_row();
                                                        });
                                                        // Per-regime performance — from bg cache
                                                        { let rp = &self.bg.regime_performance; if !rp.is_empty() {
                                                            ui.add_space(10.0);
                                                            ui.heading("Performance by Regime");
                                                            ui.separator();
                                                            egui::Grid::new("rp_grid").striped(true).num_columns(5).show(ui, |ui| {
                                                                ui.strong("DARWIN"); ui.strong("Low Vol"); ui.strong("Med Vol"); ui.strong("High Vol"); ui.strong("Best");
                                                                ui.end_row();
                                                                for r in rp {
                                                                    ui.label(&r.darwin_ticker);
                                                                    ui.label(format!("{:.3}", r.low_vol_sharpe));
                                                                    ui.label(format!("{:.3}", r.medium_vol_sharpe));
                                                                    ui.label(format!("{:.3}", r.high_vol_sharpe));
                                                                    ui.label(&r.best_regime);
                                                                    ui.end_row();
                                                                }
                                                            });
                                                        } }
                                                    }
                                                }
                                                13 => { // Tail Risk — from bg cache
                                                    if let Some(ref tail) = self.bg.tail_risk {
                                                        egui::Grid::new("tail_grid").striped(true).num_columns(2).show(ui, |ui| {
                                                            ui.label("Skewness:"); ui.label(format!("{:.4}", tail.skewness));
                                                            ui.end_row();
                                                            ui.label("Kurtosis:"); ui.label(format!("{:.4}", tail.kurtosis));
                                                            ui.end_row();
                                                            ui.label("Tail Ratio:"); ui.label(format!("{:.4}", tail.tail_ratio));
                                                            ui.end_row();
                                                            ui.label("Gain/Pain:"); ui.label(format!("{:.4}", tail.gain_to_pain));
                                                            ui.end_row();
                                                            ui.label("Ulcer Index:"); ui.label(format!("{:.4}", tail.ulcer_index));
                                                            ui.end_row();
                                                            ui.label("Pain Index:"); ui.label(format!("{:.4}", tail.pain_index));
                                                            ui.end_row();
                                                            ui.label("Omega Ratio:"); ui.label(format!("{:.4}", tail.omega_ratio));
                                                            ui.end_row();
                                                            let ft_c = if tail.fat_tail_warning { DOWN } else { UP };
                                                            ui.label("Fat Tail Warning:"); ui.label(egui::RichText::new(if tail.fat_tail_warning { "YES" } else { "NO" }).color(ft_c));
                                                            ui.end_row();
                                                        });
                                                    }
                                                    // Signal Decay Analysis — is the strategy degrading?
                                                    if !self.bg.signal_decay.is_empty() {
                                                        ui.add_space(8.0);
                                                        ui.label(egui::RichText::new("Signal Decay Analysis (90-day rolling Sharpe)").strong());
                                                        egui::Grid::new("signal_decay_grid").striped(true).num_columns(5).show(ui, |ui| {
                                                            ui.label(egui::RichText::new("DARWIN").color(AXIS_TEXT).small().strong());
                                                            ui.label(egui::RichText::new("Current Sharpe").color(AXIS_TEXT).small().strong());
                                                            ui.label(egui::RichText::new("Peak Sharpe").color(AXIS_TEXT).small().strong());
                                                            ui.label(egui::RichText::new("Decay %").color(AXIS_TEXT).small().strong());
                                                            ui.label(egui::RichText::new("Status").color(AXIS_TEXT).small().strong());
                                                            ui.end_row();
                                                            for decay in &self.bg.signal_decay {
                                                                let decay_c = if decay.decay_pct > 50.0 { DOWN } else if decay.decay_pct > 25.0 { egui::Color32::from_rgb(241, 196, 15) } else { UP };
                                                                let status = if decay.decay_pct > 50.0 { "DEGRADED" } else if decay.decay_pct > 25.0 { "WEAKENING" } else { "HEALTHY" };
                                                                ui.label(egui::RichText::new(&decay.darwin_ticker).small());
                                                                ui.label(egui::RichText::new(format!("{:.2}", decay.current_sharpe)).small());
                                                                ui.label(egui::RichText::new(format!("{:.2}", decay.peak_sharpe)).small());
                                                                ui.label(egui::RichText::new(format!("{:.1}%", decay.decay_pct)).color(decay_c).small());
                                                                ui.label(egui::RichText::new(status).color(decay_c).small().strong());
                                                                ui.end_row();
                                                            }
                                                        });

                                                        // Signal decay plot
                                                        let darwin_colors = [
                                                            egui::Color32::from_rgb(26, 188, 156), egui::Color32::from_rgb(52, 152, 219),
                                                            egui::Color32::from_rgb(241, 196, 15), egui::Color32::from_rgb(155, 89, 182),
                                                            egui::Color32::from_rgb(230, 126, 34), egui::Color32::from_rgb(231, 76, 60),
                                                        ];
                                                        ui.add_space(4.0);
                                                        Plot::new("signal_decay_plot").height(150.0).allow_drag(false).allow_zoom(false).allow_scroll(false)
                                                            .legend(egui_plot::Legend::default())
                                                            .show(ui, |plot_ui| {
                                                                for (idx, decay) in self.bg.signal_decay.iter().enumerate() {
                                                                    if decay.points.len() > 5 {
                                                                        let c = darwin_colors[idx % darwin_colors.len()];
                                                                        let pts: PlotPoints = PlotPoints::new(
                                                                            decay.points.iter().enumerate().map(|(i, (_, s))| [i as f64, *s]).collect()
                                                                        );
                                                                        plot_ui.line(Line::new(&decay.darwin_ticker, pts).color(c).width(1.5));
                                                                    }
                                                                }
                                                                plot_ui.hline(egui_plot::HLine::new("Zero", 0.0).color(egui::Color32::from_rgb(80, 80, 100)).width(0.5));
                                                            });
                                                    }
                                                }
                                                14 => { // Seasonals — from bg cache (with bar chart)
                                                    { let seasonal = &self.bg.seasonal_analysis; if !seasonal.is_empty() {
                                                        // Monthly returns bar chart
                                                        let bars: Vec<PlotBar> = seasonal.iter().enumerate().map(|(i, s)| {
                                                            let c = if s.avg_return_pct >= 0.0 { egui::Color32::from_rgb(46, 204, 113) } else { egui::Color32::from_rgb(231, 76, 60) };
                                                            PlotBar::new(i as f64, s.avg_return_pct as f64).width(0.7).fill(c).name(&s.month_name)
                                                        }).collect();
                                                        let chart = BarChart::new("Seasonal Returns", bars);
                                                        Plot::new("seasonal_bars").height(150.0)
                                                            .allow_drag(false).allow_zoom(false).allow_scroll(false)
                                                            .show_axes([false, true])
                                                            .show(ui, |plot_ui| { plot_ui.bar_chart(chart); });

                                                        ui.add_space(4.0);
                                                        egui::Grid::new("seasonal_grid").striped(true).num_columns(4).show(ui, |ui| {
                                                            let dim = egui::Color32::from_rgb(100, 100, 120);
                                                            ui.label(egui::RichText::new("Month").color(dim).small());
                                                            ui.label(egui::RichText::new("Avg Return").color(dim).small());
                                                            ui.label(egui::RichText::new("Win%").color(dim).small());
                                                            ui.label(egui::RichText::new("Median").color(dim).small());
                                                            ui.end_row();
                                                            for s in seasonal {
                                                                ui.label(&s.month_name);
                                                                let c = if s.avg_return_pct >= 0.0 { UP } else { DOWN };
                                                                ui.label(egui::RichText::new(format!("{:.2}%", s.avg_return_pct)).color(c));
                                                                let wc = if s.win_rate >= 50.0 { UP } else { DOWN };
                                                                ui.label(egui::RichText::new(format!("{:.0}%", s.win_rate)).color(wc));
                                                                ui.label(format!("{:.2}%", s.median_return_pct));
                                                                ui.end_row();
                                                            }
                                                        });
                                                    } }
                                                }
                                                15 => { // Sector Exposure — from bg cache
                                                    { let sectors = &self.bg.sector_exposure; if !sectors.is_empty() {
                                                        egui::Grid::new("sector_grid").striped(true).num_columns(5).show(ui, |ui| {
                                                            ui.strong("Sector"); ui.strong("Long $"); ui.strong("Short $"); ui.strong("Net $"); ui.strong("Symbols");
                                                            ui.end_row();
                                                            for se in sectors {
                                                                ui.label(&se.sector);
                                                                ui.label(format!("{:.0}", se.long_notional));
                                                                ui.label(format!("{:.0}", se.short_notional));
                                                                let c = if se.net_notional >= 0.0 { UP } else { DOWN };
                                                                ui.label(egui::RichText::new(format!("{:.0}", se.net_notional)).color(c));
                                                                ui.label(se.symbols.join(", "));
                                                                ui.end_row();
                                                            }
                                                        });
                                                    } }
                                                }
                                                16 => { // Liquidity Risk — from bg cache
                                                    { let liq = &self.bg.liquidity_risk; if !liq.is_empty() {
                                                        egui::Grid::new("liq_grid").striped(true).num_columns(4).show(ui, |ui| {
                                                            ui.strong("Symbol"); ui.strong("Volume"); ui.strong("Notional"); ui.strong("Conc%"); ui.strong("Risk");
                                                            ui.end_row();
                                                            for l in liq {
                                                                ui.label(&l.symbol);
                                                                ui.label(format!("{:.0}", l.position_volume));
                                                                ui.label(format!("${:.0}", l.notional));
                                                                ui.label(format!("{:.1}%", l.concentration_pct));
                                                                let risk_c = match l.risk_tier.as_str() {
                                                                    "HIGH" => DOWN,
                                                                    "MEDIUM" => egui::Color32::from_rgb(255, 200, 50),
                                                                    _ => UP,
                                                                };
                                                                ui.label(egui::RichText::new(&l.risk_tier).color(risk_c));
                                                                ui.end_row();
                                                            }
                                                        });
                                                    } }
                                                }
                                                17 => { // Margin Call Sim (from bg cache)
                                                    if let Some(ref sim) = self.bg.margin_call_sim {
                                                        egui::Grid::new("mc_sim").striped(true).num_columns(2).show(ui, |ui| {
                                                            ui.label("Current Equity:"); ui.label(format!("${:.2}", sim.current_equity));
                                                            ui.end_row();
                                                            ui.label("Used Margin:"); ui.label(format!("${:.2}", sim.current_margin_used));
                                                            ui.end_row();
                                                            ui.label("Margin Level:"); ui.label(format!("{:.1}%", sim.margin_level_pct));
                                                            ui.end_row();
                                                            if let Some(d50) = sim.days_to_margin_call_50 {
                                                                ui.label("Days to MC@50%:"); ui.label(egui::RichText::new(format!("{}", d50)).color(egui::Color32::from_rgb(255, 200, 50)));
                                                                ui.end_row();
                                                            }
                                                            if let Some(d100) = sim.days_to_margin_call_100 {
                                                                ui.label("Days to MC@100%:"); ui.label(egui::RichText::new(format!("{}", d100)).color(DOWN));
                                                                ui.end_row();
                                                            }
                                                            ui.label("Prob MC 30d:"); ui.label(format!("{:.1}%", sim.probability_30d * 100.0));
                                                            ui.end_row();
                                                            ui.label("Prob MC 90d:"); ui.label(format!("{:.1}%", sim.probability_90d * 100.0));
                                                            ui.end_row();
                                                            ui.label("Worst Equity 30d:"); ui.label(egui::RichText::new(format!("${:.2}", sim.worst_case_equity_30d)).color(DOWN));
                                                            ui.end_row();
                                                        });
                                                    }
                                                }
                                                18 => { // Optimal Allocation (from bg cache)
                                                    { let alloc = &self.bg.optimal_allocation; if !alloc.is_empty() {
                                                        egui::Grid::new("alloc_grid").striped(true).num_columns(4).show(ui, |ui| {
                                                            ui.strong("DARWIN"); ui.strong("Current %"); ui.strong("Optimal %"); ui.strong("Sharpe Contr.");
                                                            ui.end_row();
                                                            for a in alloc.iter() {
                                                                ui.label(&a.darwin_ticker);
                                                                ui.label(format!("{:.1}%", a.current_weight * 100.0));
                                                                ui.label(format!("{:.1}%", a.optimal_weight * 100.0));
                                                                ui.label(format!("{:.3}", a.sharpe_contribution));
                                                                ui.end_row();
                                                            }
                                                        });
                                                    } } // close if !alloc.is_empty() + if let Some(alloc)
                                                    // Rebalance suggestions (VaR reduction via decorrelation)
                                                    ui.add_space(10.0);
                                                    ui.heading("Rebalance Suggestions");
                                                    ui.separator();
                                                    if let Some(ref rebal) = self.bg.rebalance {
                                                        egui::Grid::new("rebal_summary").striped(true).num_columns(2).show(ui, |ui| {
                                                            ui.label("Portfolio VaR 95%:"); ui.label(format!("{:.2}%", rebal.current_portfolio_var_95));
                                                            ui.end_row();
                                                            ui.label("Portfolio Sharpe:"); ui.label(format!("{:.3}", rebal.current_sharpe));
                                                            ui.end_row();
                                                        });
                                                        if !rebal.high_correlation_pairs.is_empty() {
                                                            ui.add_space(5.0);
                                                            ui.label(egui::RichText::new("High Correlation Pairs").color(DOWN));
                                                            for pair in &rebal.high_correlation_pairs {
                                                                ui.label(format!("{}:{} ↔ {}:{} = {:.4}", pair.darwin_a, pair.symbol_a, pair.darwin_b, pair.symbol_b, pair.correlation));
                                                            }
                                                        }
                                                        if !rebal.suggestions.is_empty() {
                                                            ui.add_space(5.0);
                                                            ui.label(egui::RichText::new("Rebalance Actions").strong());
                                                            egui::Grid::new("rebal_actions").striped(true).num_columns(5).show(ui, |ui| {
                                                                ui.strong("Action"); ui.strong("DARWIN"); ui.strong("Symbol"); ui.strong("Current→Target"); ui.strong("VaR Impact");
                                                                ui.end_row();
                                                                for s in &rebal.suggestions {
                                                                    let ac = match s.action.as_str() { "REDUCE" => DOWN, "INCREASE" => UP, _ => AXIS_TEXT };
                                                                    ui.label(egui::RichText::new(&s.action).color(ac));
                                                                    ui.label(&s.darwin_ticker);
                                                                    ui.label(&s.symbol);
                                                                    ui.label(format!("{:.2} → {:.2}", s.current_volume, s.suggested_volume));
                                                                    let vc = if s.impact_var_pct < 0.0 { UP } else { DOWN };
                                                                    ui.label(egui::RichText::new(format!("{:+.2}%", s.impact_var_pct)).color(vc));
                                                                    ui.end_row();
                                                                }
                                                            });
                                                        } else {
                                                            ui.add_space(5.0);
                                                            ui.label(egui::RichText::new("No rebalance actions needed — current allocation is within tolerance.").color(egui::Color32::from_rgb(46, 204, 113)));
                                                            ui.label(egui::RichText::new("Rebalance suggestions appear when position sizes deviate significantly from optimal weights, or when high-correlation overlaps can be reduced.").color(AXIS_TEXT).small());
                                                        }
                                                    } else {
                                                        ui.label(egui::RichText::new("Rebalance data not yet computed. Wait for background thread to finish.").color(AXIS_TEXT));
                                                    }
                                                }
                                                20 => { // VaR Simulator — suggest lot size changes to hit target VaR
                                                    let sim_green = egui::Color32::from_rgb(46, 204, 113);
                                                    let sim_red = egui::Color32::from_rgb(231, 76, 60);
                                                    let sim_gold = egui::Color32::from_rgb(241, 196, 15);
                                                    let sim_dim = egui::Color32::from_rgb(100, 100, 120);
                                                    ui.label(egui::RichText::new("VaR Simulator").strong());
                                                    ui.label(egui::RichText::new("Adjust lot sizes to see how VaR changes. Target corridor: 3.25% – 6.5%").color(sim_dim).small());
                                                    ui.add_space(6.0);

                                                    // Current per-DARWIN VaR status
                                                    if !self.bg.per_darwin_var.is_empty() && !self.bg.var_multipliers.is_empty() {
                                                        // Show current state + suggestions
                                                        let portfolio_var = self.bg.var_stats.as_ref().map(|v| v.var_95).unwrap_or(0.0);
                                                        let portfolio_bal = self.bg.portfolio.as_ref().map(|p| p.total_final_balance).unwrap_or(100000.0);
                                                        let var_pct: f64 = if portfolio_bal > 0.0 { (portfolio_var / portfolio_bal * 100.0).abs() } else { 0.0 };

                                                        let var_status_color = if var_pct >= 3.25 && var_pct <= 6.5 { sim_green }
                                                            else if var_pct < 3.25 { sim_gold }
                                                            else { sim_red };
                                                        ui.horizontal(|ui| {
                                                            ui.label(egui::RichText::new(format!("Current Portfolio VaR 95%: {:.2}%", var_pct)).color(var_status_color).strong());
                                                            if var_pct < 3.25 {
                                                                ui.label(egui::RichText::new("BELOW corridor — increase position sizes").color(sim_gold));
                                                            } else if var_pct > 6.5 {
                                                                ui.label(egui::RichText::new("ABOVE corridor — reduce position sizes").color(sim_red));
                                                            } else {
                                                                ui.label(egui::RichText::new("IN corridor").color(sim_green));
                                                            }
                                                        });
                                                        ui.add_space(6.0);

                                                        // Per-DARWIN VaR breakdown with suggestions
                                                        ui.label(egui::RichText::new("Per-DARWIN Lot Size Suggestions").strong());
                                                        ui.label(egui::RichText::new("To move VaR toward corridor midpoint (4.875%):").color(sim_dim).small());
                                                        ui.add_space(4.0);

                                                        let target_mid = 4.875; // midpoint of 3.25-6.5
                                                        egui::Grid::new("var_sim_grid").striped(true).num_columns(7).min_col_width(65.0).show(ui, |ui| {
                                                            ui.label(egui::RichText::new("DARWIN").color(sim_dim).small());
                                                            ui.label(egui::RichText::new("VaR 95%").color(sim_dim).small());
                                                            ui.label(egui::RichText::new("Monthly VaR%").color(sim_dim).small());
                                                            ui.label(egui::RichText::new("Multiplier").color(sim_dim).small());
                                                            ui.label(egui::RichText::new("Status").color(sim_dim).small());
                                                            ui.label(egui::RichText::new("Suggestion").color(sim_dim).small());
                                                            ui.label(egui::RichText::new("Action").color(sim_dim).small());
                                                            ui.end_row();

                                                            for vm in &self.bg.var_multipliers {
                                                                let darwin_var = self.bg.per_darwin_var.iter()
                                                                    .find(|(t, _)| *t == vm.darwin_ticker)
                                                                    .map(|(_, v)| v);

                                                                ui.label(egui::RichText::new(&vm.darwin_ticker).strong());

                                                                // VaR 95% absolute
                                                                if let Some(v) = darwin_var {
                                                                    ui.label(format!("${:.0}", v.var_95));
                                                                } else { ui.label("—"); }

                                                                // Monthly VaR %
                                                                ui.label(format!("{:.2}%", vm.monthly_var));

                                                                // Multiplier
                                                                let mc = if vm.multiplier > 0.0 && vm.multiplier < 0.5 { sim_red }
                                                                    else if vm.multiplier >= 0.5 { sim_green }
                                                                    else { sim_dim };
                                                                ui.label(egui::RichText::new(format!("{:.2}x", vm.multiplier)).color(mc));

                                                                // Corridor status
                                                                let in_corridor = vm.monthly_var >= 3.25 && vm.monthly_var <= 6.5;
                                                                let sc = if in_corridor { sim_green } else if vm.monthly_var < 3.25 { sim_gold } else { sim_red };
                                                                let status = if in_corridor { "IN" }
                                                                    else if vm.monthly_var < 3.25 { "LOW" }
                                                                    else { "HIGH" };
                                                                ui.label(egui::RichText::new(status).color(sc).strong());

                                                                // Suggestion: scale factor to move toward target
                                                                if vm.monthly_var > 0.01 {
                                                                    let scale = target_mid / vm.monthly_var;
                                                                    let suggestion_c = if scale > 1.0 { sim_green } else { sim_red };
                                                                    ui.label(egui::RichText::new(format!("{:.1}x lots", scale)).color(suggestion_c));

                                                                    // Action text
                                                                    if (scale - 1.0).abs() < 0.1 {
                                                                        ui.label(egui::RichText::new("Hold steady").color(sim_dim).small());
                                                                    } else if scale > 1.0 {
                                                                        ui.label(egui::RichText::new(format!("Increase {:.0}%", (scale - 1.0) * 100.0)).color(sim_green).small());
                                                                    } else {
                                                                        ui.label(egui::RichText::new(format!("Reduce {:.0}%", (1.0 - scale) * 100.0)).color(sim_red).small());
                                                                    }
                                                                } else {
                                                                    ui.label("—"); ui.label("—");
                                                                }
                                                                ui.end_row();
                                                            }
                                                        });

                                                        // VaR impact visualization — bar chart
                                                        ui.add_space(8.0);
                                                        ui.label(egui::RichText::new("Monthly VaR % by DARWIN (target corridor shaded)").small().strong());
                                                        {
                                                            let bars: Vec<PlotBar> = self.bg.var_multipliers.iter().enumerate().map(|(i, vm)| {
                                                                let c = if vm.monthly_var >= 3.25 && vm.monthly_var <= 6.5 { sim_green }
                                                                    else if vm.monthly_var < 3.25 { sim_gold }
                                                                    else { sim_red };
                                                                PlotBar::new(i as f64, vm.monthly_var).width(0.7).fill(c).name(&vm.darwin_ticker)
                                                            }).collect();
                                                            if !bars.is_empty() {
                                                                let chart = BarChart::new("VaR %", bars);
                                                                Plot::new("var_sim_bars")
                                                                    .height(120.0)
                                                                    .allow_drag(false).allow_zoom(false).allow_scroll(false)
                                                                    .show_axes([false, true])
                                                                    .show(ui, |plot_ui| {
                                                                        plot_ui.bar_chart(chart);
                                                                        // Corridor lines
                                                                        let lo = Line::new("Low", PlotPoints::new(vec![[-0.5, 3.25], [10.0, 3.25]])).color(egui::Color32::from_rgba_premultiplied(241, 196, 15, 100)).width(1.0);
                                                                        let hi = Line::new("High", PlotPoints::new(vec![[-0.5, 6.5], [10.0, 6.5]])).color(egui::Color32::from_rgba_premultiplied(231, 76, 60, 100)).width(1.0);
                                                                        plot_ui.line(lo);
                                                                        plot_ui.line(hi);
                                                                    });
                                                            }
                                                        }

                                                        // Portfolio-level suggestion
                                                        ui.add_space(6.0);
                                                        if var_pct < 3.25 {
                                                            let scale = target_mid / var_pct.max(0.01);
                                                            ui.label(egui::RichText::new(format!(
                                                                "Portfolio: Increase all lot sizes by ~{:.0}% to reach corridor midpoint ({:.2}% → {:.2}%)",
                                                                (scale - 1.0) * 100.0, var_pct, target_mid
                                                            )).color(sim_gold));
                                                        } else if var_pct > 6.5 {
                                                            let scale = target_mid / var_pct;
                                                            ui.label(egui::RichText::new(format!(
                                                                "Portfolio: Reduce all lot sizes by ~{:.0}% to reach corridor midpoint ({:.2}% → {:.2}%)",
                                                                (1.0 - scale) * 100.0, var_pct, target_mid
                                                            )).color(sim_red));
                                                        } else {
                                                            ui.label(egui::RichText::new(format!(
                                                                "Portfolio VaR {:.2}% is within corridor — maintain current sizing.", var_pct
                                                            )).color(sim_green));
                                                        }
                                                    } else {
                                                        ui.label(egui::RichText::new("Import DARWIN data first.").color(AXIS_TEXT));
                                                    }
                                                }
                                                19 => { // What-If (button-triggered, uses cache)
                                                    ui.label(egui::RichText::new("What-If: Close Symbol").strong());
                                                    ui.label("Click a symbol to see VaR impact of closing:");
                                                    ui.add_space(4.0);
                                                    // Read exposure from bg cache for display; button click does one-shot DB query
                                                    for e in self.bg.exposure.iter() {
                                                        if ui.button(format!("Close {} (${:.0} net)", e.symbol, e.net_notional)).clicked() {
                                                            if let Some(ref cache) = self.cache {
                                                                if let Some(conn) = cache.try_connection() {
                                                                    if let Ok(result) = darwin::what_if_close_symbol(&conn, &e.symbol) {
                                                                        self.log.push_back(LogEntry::info(format!(
                                                                            "What-If close {}: VaR {:.2}% → {:.2}% ({:+.2}%), notional ${:.0} → ${:.0}",
                                                                            e.symbol, result.current_portfolio_var, result.new_portfolio_var,
                                                                            result.var_change_pct, result.current_notional, result.new_notional
                                                                        )));
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                                _ => {
                                                    ui.label(egui::RichText::new("Select a view from the dropdown above.").color(AXIS_TEXT));
                                                }
                                            }
                                        }
                                        Some(_) => {
                                            ui.label(egui::RichText::new("No DARWIN accounts imported.").color(AXIS_TEXT));
                                        }
                                        None => {
                                            ui.label(egui::RichText::new("Loading DARWIN data...").color(AXIS_TEXT));
                                        }
                                    }
                                    });
                                    ui.add_space(10.0);
                            }
                        });
        }
    }
}
