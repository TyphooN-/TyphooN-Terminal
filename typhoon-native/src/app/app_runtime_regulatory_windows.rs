use super::*;

impl TyphooNApp {
    pub(crate) fn render_regulatory_alert_windows(&mut self, ctx: &egui::Context) {
        // === Regulatory floating windows (Reg SHO + Halts) ===
        // Populate price columns for every regulatory-alert symbol (not just the
        // few in the watchlist). On open, force a market-data refresh ordered
        // least-fresh / no-data first; then re-read cached daily bars off the
        // render thread on a throttle so fetched bars surface while open.
        if self.show_reg_sho_window || self.show_halts_window {
            if !self.regulatory_prices_loaded && !self.bg.regulatory_alerts_by_symbol.is_empty() {
                self.refresh_regulatory_prices();
                self.regulatory_prices_loaded = true;
            }
            let read_due = self
                .regulatory_price_read_at
                .map(|at| at.elapsed() >= std::time::Duration::from_secs(3))
                .unwrap_or(true);
            if read_due && self.regulatory_prices_rx.is_none() {
                self.spawn_regulatory_price_load();
                self.regulatory_price_read_at = Some(std::time::Instant::now());
            }
        }

        let regulatory_quote_by_symbol: std::collections::HashMap<String, (f64, f64, f64, f64)> =
            self.regulatory_prices
                .iter()
                .map(|(symbol, row)| {
                    (
                        symbol.clone(),
                        (row.last, row.regular_close, row.prev_close, row.change_pct),
                    )
                })
                .chain(self.watchlist_rows.iter().map(|row| {
                    (
                        row.symbol.clone(),
                        (row.last, row.regular_close, row.prev_close, row.change_pct),
                    )
                }))
                .collect();
        let regulatory_quote = |symbol: &str| regulatory_quote_by_symbol.get(symbol).copied();

        if self.show_reg_sho_window {
            let mut open = true;
            // Button clicks are collected here and applied after the window
            // closure (which holds an immutable borrow of self).
            let mut reg_sho_action: Option<SymbolAction> = None;
            let mut reg_sho_refresh = false;
            egui::Window::new("Reg SHO Threshold Securities")
                .open(&mut open)
                .default_width(960.0)
                .default_height(500.0)
                .show(ctx, |ui| {
                    ui.label("All symbols currently on the Nasdaq Reg SHO Threshold List (live from cache)");
                    ui.separator();

                    let alerts_map = &self.bg.regulatory_alerts_by_symbol;
                    if alerts_map.is_empty() {
                        ui.label("No Reg SHO symbols loaded yet.");
                        return;
                    }

                    // Build table data — this window is Reg SHO threshold only, so
                    // exclude symbols whose only alert is another kind (e.g. a
                    // trade halt), which shares the regulatory_alerts map.
                    let mut rows: Vec<_> = alerts_map
                        .iter()
                        .filter(|(_, alerts)| {
                            alerts.iter().any(|a| a.kind == "reg_sho_threshold")
                        })
                        .collect();
                    rows.sort_by_key(|(sym, _)| *sym);

                    ui.horizontal(|ui| {
                        if ui
                            .button("Refresh prices")
                            .on_hover_text(
                                "Re-fetch the daily bar for every row — least-fresh / no-data symbols first",
                            )
                            .clicked()
                        {
                            reg_sho_refresh = true;
                        }
                        ui.label(
                            egui::RichText::new(format!("{} symbols", rows.len())).weak(),
                        );
                    });

                    let table = egui_extras::TableBuilder::new(ui)
                        .striped(true)
                        .resizable(true)
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .column(egui_extras::Column::auto().at_least(80.0))   // Symbol
                        .column(egui_extras::Column::auto().at_least(70.0))   // Last
                        .column(egui_extras::Column::auto().at_least(80.0))   // Daily Close
                        .column(egui_extras::Column::auto().at_least(70.0))   // Chg%
                        .column(egui_extras::Column::auto().at_least(120.0))  // Actions
                        .column(egui_extras::Column::remainder().at_least(200.0)); // Details

                    // Cells show "—" when a value is absent (0.0) instead of a
                    // misleading 0.0000.
                    let fmt_px = |v: f64| -> String {
                        if v > 0.0 { format!("{:.4}", v) } else { "—".to_string() }
                    };

                    // Apply user-selected sort (if any)
                    if let Some((col, asc)) = self.reg_sho_sort {
                        rows.sort_by(|a, b| {
                            let (sym_a, _alerts_a) = a;
                            let (sym_b, _alerts_b) = b;
                            let wa = regulatory_quote(sym_a.as_str());
                            let wb = regulatory_quote(sym_b.as_str());
                            let cmp = match col {
                                0 => sym_a.cmp(sym_b),
                                1 => wa.map(|w| w.0).partial_cmp(&wb.map(|w| w.0)).unwrap_or(std::cmp::Ordering::Equal),
                                2 => wa.map(|w| w.1).partial_cmp(&wb.map(|w| w.1)).unwrap_or(std::cmp::Ordering::Equal),
                                3 => {
                                    let ca = wa.map(|w| if w.2 > 0.0 { (w.0 - w.2) / w.2 * 100.0 } else { 0.0 });
                                    let cb = wb.map(|w| if w.2 > 0.0 { (w.0 - w.2) / w.2 * 100.0 } else { 0.0 });
                                    ca.partial_cmp(&cb).unwrap_or(std::cmp::Ordering::Equal)
                                }
                                _ => sym_a.cmp(sym_b),
                            };
                            if asc { cmp } else { cmp.reverse() }
                        });
                    }

                    table.header(20.0, |mut header| {
                        let mut sort_click = |ui: &mut egui::Ui, label: &str, col_idx: usize| {
                            let resp = ui.strong(label);
                            if resp.clicked() {
                                self.reg_sho_sort = match self.reg_sho_sort {
                                    Some((c, asc)) if c == col_idx => Some((c, !asc)),
                                    _ => Some((col_idx, true)),
                                };
                            }
                            resp
                        };
                        header.col(|ui| { sort_click(ui, "Symbol", 0); });
                        header.col(|ui| { sort_click(ui, "Last", 1); });
                        header.col(|ui| { sort_click(ui, "Dly Close", 2); });
                        header.col(|ui| { sort_click(ui, "Chg%", 3); });
                        header.col(|ui| { ui.strong("Actions"); });
                        header.col(|ui| { ui.strong("Details"); });
                    })
                    .body(|mut body| {
                        for (sym, alerts) in rows {
                            let alert = alerts
                                .iter()
                                .find(|a| a.kind == "reg_sho_threshold")
                                .unwrap_or(&alerts[0]);
                            // Live watchlist row first (has bid/ask); otherwise the
                            // cache-loaded snapshot so every symbol's columns fill.
                            let quote = regulatory_quote(sym.as_str());

                            body.row(18.0, |mut row| {
                                row.col(|ui| {
                                    ui.label(egui::RichText::new(sym).monospace());
                                });
                                row.col(|ui| {
                                    ui.label(quote.map(|w| fmt_px(w.0)).unwrap_or_else(|| "—".into()));
                                });
                                row.col(|ui| {
                                    ui.label(quote.map(|w| fmt_px(w.1)).unwrap_or_else(|| "—".into()));
                                });
                                row.col(|ui| {
                                    match quote {
                                        Some((last, _regular_close, _prev_close, change_pct)) if last > 0.0 => {
                                            let c = if change_pct >= 0.0 { egui::Color32::from_rgb(0,200,0) } else { egui::Color32::from_rgb(200,0,0) };
                                            ui.colored_label(c, format!("{:.2}%", change_pct));
                                        }
                                        _ => { ui.label("—"); }
                                    }
                                });
                                row.col(|ui| {
                                    ui.spacing_mut().item_spacing.x = 3.0;
                                    let already_watched = self.user_watchlist_set.contains(sym.as_str());
                                    if already_watched {
                                        ui.add_enabled(false, egui::Button::new(egui::RichText::new("✓WL").small()))
                                            .on_hover_text("Already in watchlist");
                                    } else if ui.add(egui::Button::new(egui::RichText::new("+WL").small())).on_hover_text("Add to watchlist").clicked() {
                                        reg_sho_action = Some(SymbolAction::AddWatchlist(sym.clone()));
                                    }
                                    if ui.add(egui::Button::new(egui::RichText::new("D1").small())).on_hover_text("Open D1 chart").clicked() {
                                        reg_sho_action = Some(SymbolAction::OpenChartTf(sym.clone(), Timeframe::D1));
                                    }
                                    if ui.add(egui::Button::new(egui::RichText::new("W1").small())).on_hover_text("Open W1 chart").clicked() {
                                        reg_sho_action = Some(SymbolAction::OpenChartTf(sym.clone(), Timeframe::W1));
                                    }
                                });
                                row.col(|ui| {
                                    ui.label(&alert.details);
                                });
                            });
                        }
                    });
                });

            if let Some(action) = reg_sho_action {
                self.deferred_symbol_action = action;
            }
            if reg_sho_refresh {
                self.refresh_regulatory_prices();
            }
            if !open {
                self.show_reg_sho_window = false;
            }
        }

        // === Trading Halts / LULD floating window ===
        if self.show_halts_window {
            let mut open = true;
            let mut halts_action: Option<SymbolAction> = None;
            let mut halts_refresh = false;
            egui::Window::new("Trading Halts / LULD Pauses")
                .open(&mut open)
                .default_width(820.0)
                .default_height(460.0)
                .show(ctx, |ui| {
                    ui.label("Securities currently halted (live NasdaqTrader feed, cached)");
                    ui.separator();

                    let alerts_map = &self.bg.regulatory_alerts_by_symbol;
                    let mut rows: Vec<_> = alerts_map
                        .iter()
                        .filter(|(_, alerts)| alerts.iter().any(|a| a.kind == "trade_halt"))
                        .collect();
                    if rows.is_empty() {
                        ui.label("No active trading halts.");
                        return;
                    }
                    rows.sort_by_key(|(sym, _)| *sym);

                    ui.horizontal(|ui| {
                        if ui
                            .button("Refresh prices")
                            .on_hover_text(
                                "Re-fetch the daily bar for every row — least-fresh / no-data symbols first",
                            )
                            .clicked()
                        {
                            halts_refresh = true;
                        }
                        ui.label(
                            egui::RichText::new(format!("{} symbols", rows.len())).weak(),
                        );
                    });

                    let fmt_px = |v: f64| -> String {
                        if v > 0.0 { format!("{:.4}", v) } else { "—".to_string() }
                    };

                    let table = egui_extras::TableBuilder::new(ui)
                        .striped(true)
                        .resizable(true)
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .column(egui_extras::Column::auto().at_least(80.0))   // Symbol
                        .column(egui_extras::Column::auto().at_least(70.0))   // Last
                        .column(egui_extras::Column::auto().at_least(80.0))   // Prev Close
                        .column(egui_extras::Column::auto().at_least(70.0))   // Chg%
                        .column(egui_extras::Column::auto().at_least(120.0))  // Actions
                        .column(egui_extras::Column::remainder().at_least(240.0)); // Halt info

                    // Apply user-selected sort (if any)
                    if let Some((col, asc)) = self.halts_sort {
                        rows.sort_by(|a, b| {
                            let (sym_a, _alerts_a) = a;
                            let (sym_b, _alerts_b) = b;
                            let wa = regulatory_quote(sym_a.as_str());
                            let wb = regulatory_quote(sym_b.as_str());
                            let cmp = match col {
                                0 => sym_a.cmp(sym_b),
                                1 => wa.map(|w| w.0).partial_cmp(&wb.map(|w| w.0)).unwrap_or(std::cmp::Ordering::Equal),
                                2 => wa.map(|w| w.2).partial_cmp(&wb.map(|w| w.2)).unwrap_or(std::cmp::Ordering::Equal),
                                3 => {
                                    let ca = wa.map(|w| if w.2 > 0.0 { (w.0 - w.2) / w.2 * 100.0 } else { 0.0 });
                                    let cb = wb.map(|w| if w.2 > 0.0 { (w.0 - w.2) / w.2 * 100.0 } else { 0.0 });
                                    ca.partial_cmp(&cb).unwrap_or(std::cmp::Ordering::Equal)
                                }
                                _ => sym_a.cmp(sym_b),
                            };
                            if asc { cmp } else { cmp.reverse() }
                        });
                    }

                    table.header(20.0, |mut header| {
                        let mut sort_click = |ui: &mut egui::Ui, label: &str, col_idx: usize| {
                            let resp = ui.strong(label);
                            if resp.clicked() {
                                self.halts_sort = match self.halts_sort {
                                    Some((c, asc)) if c == col_idx => Some((c, !asc)),
                                    _ => Some((col_idx, true)),
                                };
                            }
                            resp
                        };
                        header.col(|ui| { sort_click(ui, "Symbol", 0); });
                        header.col(|ui| { sort_click(ui, "Last", 1); });
                        header.col(|ui| { sort_click(ui, "Prev Close", 2); });
                        header.col(|ui| { sort_click(ui, "Chg%", 3); });
                        header.col(|ui| { ui.strong("Actions"); });
                        header.col(|ui| { ui.strong("Halt info"); });
                    })
                    .body(|mut body| {
                        for (sym, alerts) in rows {
                            let alert = alerts
                                .iter()
                                .find(|a| a.kind == "trade_halt")
                                .unwrap_or(&alerts[0]);
                            let quote = regulatory_quote(sym.as_str());
                            body.row(18.0, |mut row| {
                                row.col(|ui| {
                                    ui.label(egui::RichText::new(sym).monospace().color(egui::Color32::from_rgb(255, 90, 90)));
                                });
                                row.col(|ui| {
                                    ui.label(quote.map(|w| fmt_px(w.0)).unwrap_or_else(|| "—".into()));
                                });
                                row.col(|ui| {
                                    ui.label(quote.map(|w| fmt_px(w.2)).unwrap_or_else(|| "—".into()));
                                });
                                row.col(|ui| {
                                    match quote {
                                        Some((last, _regular_close, _prev_close, change_pct)) if last > 0.0 => {
                                            let c = if change_pct >= 0.0 { egui::Color32::from_rgb(0,200,0) } else { egui::Color32::from_rgb(200,0,0) };
                                            ui.colored_label(c, format!("{:.2}%", change_pct));
                                        }
                                        _ => { ui.label("—"); }
                                    }
                                });
                                row.col(|ui| {
                                    ui.spacing_mut().item_spacing.x = 3.0;
                                    let already_watched = self.user_watchlist_set.contains(sym.as_str());
                                    if already_watched {
                                        ui.add_enabled(false, egui::Button::new(egui::RichText::new("✓WL").small()))
                                            .on_hover_text("Already in watchlist");
                                    } else if ui.add(egui::Button::new(egui::RichText::new("+WL").small())).on_hover_text("Add to watchlist").clicked() {
                                        halts_action = Some(SymbolAction::AddWatchlist(sym.clone()));
                                    }
                                    if ui.add(egui::Button::new(egui::RichText::new("D1").small())).on_hover_text("Open D1 chart").clicked() {
                                        halts_action = Some(SymbolAction::OpenChartTf(sym.clone(), Timeframe::D1));
                                    }
                                    if ui.add(egui::Button::new(egui::RichText::new("W1").small())).on_hover_text("Open W1 chart").clicked() {
                                        halts_action = Some(SymbolAction::OpenChartTf(sym.clone(), Timeframe::W1));
                                    }
                                });
                                row.col(|ui| {
                                    ui.label(&alert.details);
                                });
                            });
                        }
                    });
                });

            if let Some(action) = halts_action {
                self.deferred_symbol_action = action;
            }
            if halts_refresh {
                self.refresh_regulatory_prices();
            }
            if !open {
                self.show_halts_window = false;
            }
        }

        // Both regulatory windows closed → drop the one-shot fetch kick and the
        // read throttle so the next open re-fetches and re-reads fresh prices.
        if !self.show_reg_sho_window && !self.show_halts_window {
            self.regulatory_prices_loaded = false;
            self.regulatory_price_read_at = None;
        }
    }
}
